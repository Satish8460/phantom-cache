use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum DiagnosticLevel {
    INFO,
    WARN,
    ERROR,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticEvent {
    pub timestamp_epoch_ms: u64,
    pub severity: DiagnosticLevel,
    pub event_name: &'static str,
    pub key_fp: String,
    pub op_id: Option<u64>,
    pub op_gen: Option<u64>,
    pub outcome: &'static str,
    pub duration_ms: Option<u64>,
    pub error_msg: Option<String>,
}

#[derive(Default)]
pub struct CacheCounters {
    pub requests_total: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub cache_bypasses: AtomicU64,
    pub coalesced_waiters: AtomicU64,
    pub stale_writes_prevented: AtomicU64,
    pub stale_ops_ignored: AtomicU64,
    pub finalization_panics: AtomicU64,
    pub lock_poison_recovered: AtomicU64,
    pub reclamations_total: AtomicU64,
    pub evictions_total: AtomicU64,
    pub waiter_limit_rejections: AtomicU64,
    pub capacity_rejections: AtomicU64,
    pub origin_fetches_failed: AtomicU64,
    pub diagnostic_events_dropped: AtomicU64,
    pub byte_budget_rejections: AtomicU64,
    pub oversize_rejections: AtomicU64,
    pub anti_entropy_sync_total: AtomicU64,
    pub anti_entropy_gaps_detected: AtomicU64,
    pub anti_entropy_keys_evicted: AtomicU64,
    pub anti_entropy_full_flushes: AtomicU64,
    pub stale_served_total: AtomicU64,
    pub stale_cap_violations: AtomicU64,
    pub revalidation_started: AtomicU64,
    pub revalidation_ok: AtomicU64,
    pub revalidation_not_modified: AtomicU64,
    pub revalidation_error: AtomicU64,
    pub background_refresh_in_flight: AtomicU64,
    // The counters below are deliberately NOT in `CacheCountersSnapshot` / the telemetry schema.
    /// Freshness-metadata / 304-refresh setter calls refused by the generation/instance/live-op
    /// FENCE (the caller was overtaken by a DELETE, a newer fill, or already finished).
    pub metadata_setter_rejections: AtomicU64,
    /// 304 refreshes whose fence passed but which could not be applied because the resident entry
    /// was missing or epoch-stale (tenant/global invalidation).
    pub metadata_refresh_not_applied: AtomicU64,
    /// Parked (pending) freshness metadata discarded by `finalize_fetch` because the store was
    /// rejected (oversize/budget/generation/epoch) or the op ended Uncacheable/Error.
    pub pending_metadata_dropped: AtomicU64,
    /// Background revalidations that were triggered but did not become the single-flight leader
    /// (joined an in-flight fetch, hit a fresh entry, or were rejected). Complements
    /// `revalidation_started`, which counts only real leaders.
    pub revalidation_not_initiated: AtomicU64,
    /// 304 responses that could not refresh anything (entry purged/invalidated/replaced meanwhile).
    pub revalidation_not_applied: AtomicU64,
    /// Origin requests that actually carried a stored validator (If-None-Match and/or
    /// If-Modified-Since from the STORED entry). Measured at dispatch. Not in the telemetry schema.
    pub revalidation_conditional_sent: AtomicU64,
    /// Raw 304 responses received to those conditional requests (before the fenced merge decides
    /// whether they were applied; compare `revalidation_not_modified` / `revalidation_not_applied`).
    pub revalidation_304_received: AtomicU64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CacheCountersSnapshot {
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_bypasses: u64,
    pub coalesced_waiters: u64,
    pub stale_writes_prevented: u64,
    pub stale_ops_ignored: u64,
    pub finalization_panics: u64,
    pub lock_poison_recovered: u64,
    pub reclamations_total: u64,
    pub evictions_total: u64,
    pub waiter_limit_rejections: u64,
    pub capacity_rejections: u64,
    pub origin_fetches_failed: u64,
    pub diagnostic_events_dropped: u64,
    pub byte_budget_rejections: u64,
    pub oversize_rejections: u64,
    pub anti_entropy_sync_total: u64,
    pub anti_entropy_gaps_detected: u64,
    pub anti_entropy_keys_evicted: u64,
    pub anti_entropy_full_flushes: u64,
    pub stale_served_total: u64,
    pub stale_cap_violations: u64,
    pub revalidation_started: u64,
    pub revalidation_ok: u64,
    pub revalidation_not_modified: u64,
    pub revalidation_error: u64,
    pub background_refresh_in_flight: u64,
}

impl CacheCounters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> CacheCountersSnapshot {
        CacheCountersSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            cache_bypasses: self.cache_bypasses.load(Ordering::Relaxed),
            coalesced_waiters: self.coalesced_waiters.load(Ordering::Relaxed),
            stale_writes_prevented: self.stale_writes_prevented.load(Ordering::Relaxed),
            stale_ops_ignored: self.stale_ops_ignored.load(Ordering::Relaxed),
            finalization_panics: self.finalization_panics.load(Ordering::Relaxed),
            lock_poison_recovered: self.lock_poison_recovered.load(Ordering::Relaxed),
            reclamations_total: self.reclamations_total.load(Ordering::Relaxed),
            evictions_total: self.evictions_total.load(Ordering::Relaxed),
            waiter_limit_rejections: self.waiter_limit_rejections.load(Ordering::Relaxed),
            capacity_rejections: self.capacity_rejections.load(Ordering::Relaxed),
            origin_fetches_failed: self.origin_fetches_failed.load(Ordering::Relaxed),
            diagnostic_events_dropped: self.diagnostic_events_dropped.load(Ordering::Relaxed),
            byte_budget_rejections: self.byte_budget_rejections.load(Ordering::Relaxed),
            oversize_rejections: self.oversize_rejections.load(Ordering::Relaxed),
            anti_entropy_sync_total: self.anti_entropy_sync_total.load(Ordering::Relaxed),
            anti_entropy_gaps_detected: self.anti_entropy_gaps_detected.load(Ordering::Relaxed),
            anti_entropy_keys_evicted: self.anti_entropy_keys_evicted.load(Ordering::Relaxed),
            anti_entropy_full_flushes: self.anti_entropy_full_flushes.load(Ordering::Relaxed),
            stale_served_total: self.stale_served_total.load(Ordering::Relaxed),
            stale_cap_violations: self.stale_cap_violations.load(Ordering::Relaxed),
            revalidation_started: self.revalidation_started.load(Ordering::Relaxed),
            revalidation_ok: self.revalidation_ok.load(Ordering::Relaxed),
            revalidation_not_modified: self.revalidation_not_modified.load(Ordering::Relaxed),
            revalidation_error: self.revalidation_error.load(Ordering::Relaxed),
            background_refresh_in_flight: self.background_refresh_in_flight.load(Ordering::Relaxed),
        }
    }
}

pub struct DiagnosticHasher {
    seed: u64,
}

impl Default for DiagnosticHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticHasher {
    pub fn new() -> Self {
        Self {
            seed: rand::random::<u64>(),
        }
    }

    pub fn fingerprint(&self, key_str: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        key_str.hash(&mut hasher);
        let h = hasher.finish();
        format!("k_{:08x}", (h ^ (h >> 32)) as u32)
    }
}

pub struct DiagnosticBuffer {
    pub capacity: usize,
    pub events: Mutex<VecDeque<DiagnosticEvent>>,
    pub counters: Arc<CacheCounters>,
    pub hasher: DiagnosticHasher,
}

impl DiagnosticBuffer {
    pub fn new(capacity: usize, counters: Arc<CacheCounters>) -> Self {
        Self {
            capacity,
            events: Mutex::new(VecDeque::with_capacity(capacity)),
            counters,
            hasher: DiagnosticHasher::new(),
        }
    }

    pub fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn push(&self, event: DiagnosticEvent) {
        match self.events.try_lock() {
            Ok(mut guard) => {
                if guard.len() >= self.capacity {
                    guard.pop_front();
                }
                guard.push_back(event);
            }
            Err(_) => {
                self.counters
                    .diagnostic_events_dropped
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn get_recent_events(&self, limit: usize) -> Vec<DiagnosticEvent> {
        let guard = match self.events.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.iter().rev().take(limit).cloned().collect()
    }
}
