//! Policy Execution Layer, Stratified Canary Partitioning & Safe Fallback (WP-34)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-04
//!
//! Enforces:
//! - Atomic policy reconfiguration with `rebuild_from` via `PartitionedCache`
//! - Stratified canary bucket partitioning (B = 1024 hash buckets)
//! - Unconditional hard caps (RFC 9111 staleness cap, byte budget, refresh limit)
//! - Automated, immutable `revert_to_safe` restoring baseline state
//! - Watchdog heartbeat monitor triggering automatic fallback on loop stall

use super::*;
use crate::core::cache::{EvictionPolicy, PartitionedCache};
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const TOTAL_BUCKETS: usize = 1024;
pub const HOLDBACK_BUCKET_COUNT: usize = 10; // 1% permanent holdback
pub const CANARY_BUCKET_COUNT: usize = 50; // 5% canary exposure
pub const MAX_HARD_STALE_MS: u64 = 86_400_000; // 24-hour RFC 9111 hard cap
pub const MAX_BACKGROUND_REFRESH_CAP: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketArm {
    Holdback,
    Candidate,
    Control,
}

#[derive(Debug, Clone)]
pub struct SafeBaselineConfig {
    pub eviction_policy: EvictionPolicy,
    pub max_stale_ms: u64,
    pub max_background_refresh: usize,
    pub config_hash: String,
}

impl Default for SafeBaselineConfig {
    fn default() -> Self {
        Self {
            eviction_policy: EvictionPolicy::Sieve,
            max_stale_ms: 86_400_000,
            max_background_refresh: 128,
            config_hash: "SAFE_DEFAULT_BL1".to_string(),
        }
    }
}

// ============================================================================
// Origin concurrency gate (R-01)
// ============================================================================

/// Sentinel meaning "no cap configured" (uncapped).
const CAP_UNCONFIGURED: usize = usize::MAX;

/// Observation window used to drive the AIMD controller from real origin dispatches.
#[derive(Default)]
struct ObservationWindow {
    latencies_ms: Vec<u32>,
    errors: u32,
}

/// Point-in-time read of gate state (all values are direct atomic reads).
#[derive(Debug, Clone, Copy)]
pub struct OriginGateSnapshot {
    pub enforcing: bool,
    /// `None` when no cap is configured.
    pub cap: Option<usize>,
    pub in_flight: usize,
    pub rejections: u64,
    pub dispatched: u64,
    pub controller_steps: u64,
}

/// Process-wide origin dispatch gate. In-flight is a single atomic; the cap is an atomic
/// updated by the AIMD controller. No lock is held across an `.await`: the only mutexes
/// (observation window, controller) are taken for short synchronous sections.
pub struct OriginGate {
    cap: AtomicUsize,
    in_flight: AtomicUsize,
    rejections: AtomicU64,
    dispatched: AtomicU64,
    controller_steps: AtomicU64,
    window_size: AtomicUsize,
    window: Mutex<ObservationWindow>,
    controller: Mutex<Option<super::candidates::OriginConcurrencyCapController>>,
}

/// RAII permit: the in-flight slot is released on drop, covering success, error, timeout
/// and future cancellation.
pub struct OriginPermit {
    gate: Arc<OriginGate>,
}

impl Drop for OriginPermit {
    fn drop(&mut self) {
        self.gate.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

impl OriginPermit {
    /// Feed one real observation (origin latency and failure) into the controller window.
    /// A permit dropped without `observe` (cancellation) contributes no sample.
    pub fn observe(&self, latency_ms: u64, failed: bool) {
        self.gate.record_observation(latency_ms.min(u32::MAX as u64) as u32, failed);
    }
}

impl OriginGate {
    fn new() -> Self {
        Self {
            cap: AtomicUsize::new(CAP_UNCONFIGURED),
            in_flight: AtomicUsize::new(0),
            rejections: AtomicU64::new(0),
            dispatched: AtomicU64::new(0),
            controller_steps: AtomicU64::new(0),
            window_size: AtomicUsize::new(32),
            window: Mutex::new(ObservationWindow::default()),
            controller: Mutex::new(None),
        }
    }

    pub fn current_cap(&self) -> Option<usize> {
        match self.cap.load(Ordering::SeqCst) {
            CAP_UNCONFIGURED => None,
            c => Some(c),
        }
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    fn record_observation(&self, latency_ms: u32, failed: bool) {
        let size = self.window_size.load(Ordering::Relaxed).max(1);
        let mut full: Option<(Vec<u32>, u32)> = None;
        {
            let mut w = self.window.lock().unwrap();
            w.latencies_ms.push(latency_ms);
            if failed {
                w.errors += 1;
            }
            if w.latencies_ms.len() >= size {
                full = Some((std::mem::take(&mut w.latencies_ms), std::mem::take(&mut w.errors)));
            }
        }
        if let Some((mut lat, errs)) = full {
            lat.sort_unstable();
            // nearest-rank p99
            let rank = ((lat.len() as f64) * 0.99).ceil() as usize;
            let p99 = lat[rank.clamp(1, lat.len()) - 1];
            let err_rate = errs as f64 / lat.len() as f64;
            let mut guard = self.controller.lock().unwrap();
            if let Some(ctrl) = guard.as_mut() {
                let new_cap = ctrl.step(p99, err_rate);
                self.cap.store(new_cap, Ordering::SeqCst);
                self.controller_steps.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// The origin-concurrency cap is saturated; no permit was issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginCapSaturated;

impl std::fmt::Display for OriginCapSaturated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "origin concurrency cap saturated")
    }
}

impl std::error::Error for OriginCapSaturated {}

pub struct DefaultPolicyExecutor {
    origin_gate: Arc<OriginGate>,
    mode: AtomicU8,
    cache: Option<Arc<PartitionedCache>>,
    safe_config: SafeBaselineConfig,
    active_candidate: Mutex<Option<PolicyCandidate>>,
    last_heartbeat_ms: AtomicU64,
    revert_count: AtomicU64,
}

impl DefaultPolicyExecutor {
    pub fn new(cache: Option<Arc<PartitionedCache>>) -> Self {
        Self {
            origin_gate: Arc::new(OriginGate::new()),
            mode: AtomicU8::new(IntelligenceMode::Off as u8),
            cache,
            safe_config: SafeBaselineConfig::default(),
            active_candidate: Mutex::new(None),
            last_heartbeat_ms: AtomicU64::new(0),
            revert_count: AtomicU64::new(0),
        }
    }

    /// Configure the AIMD origin concurrency cap. Until called the gate is uncapped.
    /// `window` is the number of real origin observations per controller step.
    pub fn configure_origin_cap(&self, min_concurrency: usize, max_concurrency: usize, p99_ceiling_ms: u32, window: usize) {
        let min_c = min_concurrency.max(1);
        let ctrl = super::candidates::OriginConcurrencyCapController::new(min_c, max_concurrency.max(min_c), p99_ceiling_ms);
        let initial = ctrl.current_concurrency_limit;
        let g = &self.origin_gate;
        g.window_size.store(window.max(1), Ordering::Relaxed);
        *g.window.lock().unwrap() = ObservationWindow::default();
        *g.controller.lock().unwrap() = Some(ctrl);
        g.cap.store(initial, Ordering::SeqCst);
    }

    pub fn origin_gate(&self) -> &Arc<OriginGate> {
        &self.origin_gate
    }

    /// The cap is enforced only in Canary or Active mode with a configured cap. In Off,
    /// Observe and Advisory the origin path is uncapped (in-flight is still counted).
    pub fn origin_cap_enforcing(&self) -> bool {
        matches!(self.current_mode(), IntelligenceMode::Canary | IntelligenceMode::Active)
            && self.origin_gate.cap.load(Ordering::SeqCst) != CAP_UNCONFIGURED
    }

    /// Legacy unit-error form of [`Self::acquire_origin_permit`]. Kept because `src/lab/t1_sim.rs`,
    /// `tests/integration/origin_cap_tests.rs` and the source-text check in the frozen M6 test
    /// (`m6_research_evaluation_tests.rs`, which requires this name in the lab source) still use it;
    /// migrating them is a protocol-amendment-time change. The lint allow is justified by those
    /// existing callers; new code must use the typed form.
    #[allow(clippy::result_unit_err)]
    pub fn try_acquire_origin_permit(&self) -> Result<OriginPermit, ()> {
        self.acquire_origin_permit().map_err(|_| ())
    }

    /// Try to take an origin permit. `Err(OriginCapSaturated)` means saturated (counted as a rejection).
    pub fn acquire_origin_permit(&self) -> Result<OriginPermit, OriginCapSaturated> {
        let g = &self.origin_gate;
        if self.origin_cap_enforcing() {
            let cap = g.cap.load(Ordering::SeqCst);
            let ok = g
                .in_flight
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| if c < cap { Some(c + 1) } else { None })
                .is_ok();
            if !ok {
                g.rejections.fetch_add(1, Ordering::Relaxed);
                return Err(OriginCapSaturated);
            }
        } else {
            g.in_flight.fetch_add(1, Ordering::SeqCst);
        }
        g.dispatched.fetch_add(1, Ordering::Relaxed);
        Ok(OriginPermit { gate: g.clone() })
    }

    pub fn origin_gate_snapshot(&self) -> OriginGateSnapshot {
        let g = &self.origin_gate;
        OriginGateSnapshot {
            enforcing: self.origin_cap_enforcing(),
            cap: g.current_cap(),
            in_flight: g.in_flight(),
            rejections: g.rejections.load(Ordering::Relaxed),
            dispatched: g.dispatched.load(Ordering::Relaxed),
            controller_steps: g.controller_steps.load(Ordering::Relaxed),
        }
    }

    pub fn with_safe_config(mut self, config: SafeBaselineConfig) -> Self {
        self.safe_config = config;
        self
    }

    /// Pure function routing key hash to canary bucket arm
    pub fn route_bucket(key_hash: u64) -> BucketArm {
        let mut x = key_hash ^ (key_hash >> 32);
        x ^= x >> 16;
        let bucket = (x as usize) % TOTAL_BUCKETS;

        if bucket < HOLDBACK_BUCKET_COUNT {
            BucketArm::Holdback
        } else if bucket < HOLDBACK_BUCKET_COUNT + CANARY_BUCKET_COUNT {
            BucketArm::Candidate
        } else {
            BucketArm::Control
        }
    }

    /// Evaluates watchdog health. If stall > timeout_ms, immediately reverts and drops mode to Off.
    pub fn check_watchdog(&self, now_ms: u64, timeout_ms: u64) -> bool {
        let last = self.last_heartbeat_ms.load(Ordering::Relaxed);
        if last > 0 && now_ms.saturating_sub(last) > timeout_ms {
            self.revert_to_safe();
            self.set_mode(IntelligenceMode::Off).ok();
            true // Watchdog tripped
        } else {
            false
        }
    }

    pub fn get_revert_count(&self) -> u64 {
        self.revert_count.load(Ordering::Relaxed)
    }

    pub fn get_active_candidate(&self) -> Option<PolicyCandidate> {
        self.active_candidate.lock().unwrap().clone()
    }
    pub fn heartbeat_at(&self, now_ms: u64) {
        self.last_heartbeat_ms.store(now_ms, Ordering::Relaxed);
    }

    /// Fault-tolerant policy application that catches panics, invokes revert_to_safe, and drops mode to Off
    pub fn safe_apply_policy(&self, candidate: &PolicyCandidate) -> Result<(), &'static str> {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.apply_policy(candidate)
        }));
        match res {
            Ok(apply_res) => apply_res,
            Err(_) => {
                self.revert_to_safe();
                self.set_mode(IntelligenceMode::Off).ok();
                Err("Panic during policy application; safely reverted to baseline")
            }
        }
    }
}

impl PolicyExecutor for DefaultPolicyExecutor {
    fn current_mode(&self) -> IntelligenceMode {
        match self.mode.load(Ordering::Relaxed) {
            1 => IntelligenceMode::Observe,
            2 => IntelligenceMode::Advisory,
            3 => IntelligenceMode::Canary,
            4 => IntelligenceMode::Active,
            _ => IntelligenceMode::Off,
        }
    }

    fn set_mode(&self, mode: IntelligenceMode) -> Result<(), &'static str> {
        let val = match mode {
            IntelligenceMode::Off => 0,
            IntelligenceMode::Observe => 1,
            IntelligenceMode::Advisory => 2,
            IntelligenceMode::Canary => 3,
            IntelligenceMode::Active => 4,
        };
        self.mode.store(val, Ordering::Relaxed);
        Ok(())
    }

    fn apply_policy(&self, candidate: &PolicyCandidate) -> Result<(), &'static str> {
        let mode = self.current_mode();
        if mode == IntelligenceMode::Off || mode == IntelligenceMode::Observe {
            return Err("Cannot apply policy while in Off or Observe mode");
        }

        if let Some(ref c) = self.cache {
            match candidate.kind {
                PolicyKind::Eviction => {
                    let policy = match candidate.candidate_id.as_str() {
                        "LRU" => EvictionPolicy::Lru,
                        "S3-FIFO" | "S3FIFO" => EvictionPolicy::S3Fifo,
                        _ => EvictionPolicy::Sieve,
                    };
                    c.set_eviction_policy(policy);
                }
                PolicyKind::StaleServing => {
                    let stale_ms = candidate
                        .parameters
                        .get("max_stale_ms")
                        .copied()
                        .unwrap_or(86_400_000.0) as u64;
                    // Enforce hard cap
                    let clamped = stale_ms.min(MAX_HARD_STALE_MS);
                    c.set_max_stale_ms(clamped);
                }
                PolicyKind::RefreshEarly => {
                    let max_refresh = candidate
                        .parameters
                        .get("max_background_refresh")
                        .copied()
                        .unwrap_or(128.0) as usize;
                    // Enforce hard cap
                    let clamped = max_refresh.min(MAX_BACKGROUND_REFRESH_CAP);
                    c.set_max_background_refresh(clamped);
                }
                _ => {}
            }
        }

        *self.active_candidate.lock().unwrap() = Some(candidate.clone());
        Ok(())
    }

    fn revert_to_safe(&self) {
        if let Some(ref c) = self.cache {
            c.set_eviction_policy(self.safe_config.eviction_policy);
            c.set_max_stale_ms(self.safe_config.max_stale_ms);
            c.set_max_background_refresh(self.safe_config.max_background_refresh);
        }
        *self.active_candidate.lock().unwrap() = None;
        self.revert_count.fetch_add(1, Ordering::Relaxed);
    }

    fn heartbeat(&self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_heartbeat_ms.store(now_ms, Ordering::Relaxed);
    }
}
