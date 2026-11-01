//! Tier-1 (T1, SIMULATED) product-path cell runner (R-02).
//!
//! Every cell drives the REAL library components from `phantom_cache_demo` on a deterministic
//! virtual clock against the lab `SimulatedOrigin`:
//!   - `PartitionedCache` (entry + byte limits, SIEVE eviction, epoch invalidation),
//!   - `begin_fetch_atomic` / `OriginFetchGuard::complete` / `finalize_fetch` (single-flight, fencing),
//!   - `check_stale_serving` / `record_stale_served` (RFC 9111 stale-while-revalidate / stale-if-error,
//!     hard staleness cap),
//!   - `DefaultPolicyExecutor` origin gate (`try_acquire_origin_permit`, `OriginPermit::observe`,
//!     AIMD `OriginConcurrencyCapController`), `OriginAwareStaleController` + `apply_policy`
//!     (dynamic stale window), `FrequencyAdmissionFilter` (TinyLFU-style admission).
//!
//! Arms (baselines and the adaptive arm) are DIFFERENT CONFIGURATIONS of the same event loop
//! (`ArmConfig`); there are no multipliers applied to results. All reported metrics (peak origin
//! concurrency, client latency, hit ratio, errors, staleness) are computed from executed events.
//!
//! Provenance: tier = SIMULATED. Nothing here is a measurement of a real network or host.
//! The only wall clock used is inside the product cache (`shrink_shard`), which does not affect
//! outcomes in the configurations used here; determinism is asserted by tests.

use super::events::EventQueue;
use super::origin_sim::{DegradationScenario, ServiceDistribution, SimulatedOrigin};
use super::rng::{MasterRng, StreamRng};
use super::workloads::*;
use crate::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache, StaleServingDecision,
};
use crate::core::coalesce::{FetchRegistration, FinalizeOutcome, OriginError, OriginFetchGuard};
use crate::core::freshness::FreshnessMetadata;
use crate::intelligence::candidates::{FrequencyAdmissionFilter, OriginAwareStaleController};
use crate::intelligence::executor::{DefaultPolicyExecutor, OriginPermit, MAX_HARD_STALE_MS};
use crate::intelligence::{IntelligenceMode, PolicyCandidate, PolicyExecutor, PolicyKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Origin concurrency cap configuration (realised by the product `DefaultPolicyExecutor` gate).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapConfig {
    None,
    /// Constant cap (AIMD controller with min == max).
    Static { n: usize },
    /// Product AIMD controller (`OriginConcurrencyCapController`), stepped every `window` origin observations.
    Aimd { min: usize, max: usize, p99_ceiling_ms: u32, window: usize },
}

/// Dynamic stale window (product `OriginAwareStaleController`, applied via executor `apply_policy`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynStale {
    pub base_ms: u64,
    pub error_threshold: f64,
}

/// Lab circuit breaker (BL-5). Not a product component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BreakerCfg {
    pub err_threshold: f64,
    pub min_samples: usize,
    pub open_ms: u64,
}

/// One arm = one real configuration of the event loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArmConfig {
    pub id: String,
    /// Single-flight coalescing of concurrent misses (product `begin_fetch_atomic` `Wait`).
    pub coalesce: bool,
    /// Serve stale immediately within the window while refreshing (stale-while-revalidate).
    pub swr: bool,
    /// Serve stale on origin error / shed (stale-if-error).
    pub stale_on_error: bool,
    /// Stale window in force (= product `cache.max_stale_ms`) when no dynamic controller applies.
    pub static_stale_ms: u64,
    pub dyn_stale: Option<DynStale>,
    pub cap: CapConfig,
    pub breaker: Option<BreakerCfg>,
    /// TinyLFU-style admission (product `FrequencyAdmissionFilter`), only effective when the executor is Active.
    pub tinylfu: bool,
    /// Install the executor but leave it in `IntelligenceMode::Off` (sensitivity check: adaptive mechanism disabled).
    pub executor_mode_off: bool,
}

impl ArmConfig {
    fn uses_executor(&self) -> bool {
        self.cap != CapConfig::None || self.dyn_stale.is_some() || self.tinylfu
    }
}

/// Frozen arm parameters (tuned on tuning seeds T and tuning families only; see tuning protocol).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArmParams {
    pub bl12b_cap: usize,
    pub bl12b_stale_ms: u64,
    pub bl4_min: usize,
    pub bl4_max: usize,
    pub bl4_p99_ceiling_ms: u32,
    pub bl5_err_threshold: f64,
    pub bl5_open_ms: u64,
    pub aimd_window: usize,
    pub pie_min: usize,
    pub pie_max: usize,
    pub pie_p99_ceiling_ms: u32,
    pub pie_base_stale_ms: u64,
    pub pie_error_threshold: f64,
    /// AIMD observation window of PIE-Retained (v2.2 tuned parameter; defaults to 16 for older artifacts).
    #[serde(default = "default_pie_window")]
    pub pie_window: usize,
    /// BL-6: per-scenario oracle static (cap, stale_ms); empty unless produced by tuning.
    pub bl6_by_scenario: BTreeMap<String, (usize, u64)>,
}

fn default_pie_window() -> usize {
    16
}

impl ArmParams {
    /// UNTUNED development defaults, used by smoke runs only. Real runs must load the frozen file.
    pub fn untuned_defaults() -> Self {
        Self {
            bl12b_cap: 16,
            bl12b_stale_ms: 10_000,
            bl4_min: 2,
            bl4_max: 32,
            bl4_p99_ceiling_ms: 300,
            bl5_err_threshold: 0.5,
            bl5_open_ms: 1_000,
            aimd_window: 16,
            pie_min: 2,
            pie_max: 32,
            pie_p99_ceiling_ms: 300,
            pie_base_stale_ms: 10_000,
            pie_error_threshold: 0.2,
            pie_window: 16,
            bl6_by_scenario: BTreeMap::new(),
        }
    }
}

fn base_arm(id: &str) -> ArmConfig {
    ArmConfig {
        id: id.to_string(),
        coalesce: true,
        swr: false,
        stale_on_error: false,
        static_stale_ms: 0,
        dyn_stale: None,
        cap: CapConfig::None,
        breaker: None,
        tinylfu: false,
        executor_mode_off: false,
    }
}

/// Arms known to the runner.
pub fn arm_ids() -> Vec<&'static str> {
    vec![
        "BL-0", "BL-1", "BL-4", "BL-5", "BL-6", "BL-12b", "PIE-Retained", "PIE-no-stale", "PIE-no-cap",
        "PIE-evict-only", "PIE-off", "CTRL-nonadaptive", "PLANT-CAP4", "CAND-STATIC-STALE", "CAND-DYN-STALE",
        "CAND-STATIC-CAP128", "CAND-AIMD", "CAND-NOADM", "CAND-ADM",
    ]
}

/// Builds the configuration of an arm. `scenario_id` is only used by BL-6 (per-scenario oracle).
pub fn arm_config(id: &str, p: &ArmParams, scenario_id: &str) -> ArmConfig {
    let pie_cap = CapConfig::Aimd { min: p.pie_min, max: p.pie_max, p99_ceiling_ms: p.pie_p99_ceiling_ms, window: p.pie_window };
    let pie_dyn = Some(DynStale { base_ms: p.pie_base_stale_ms, error_threshold: p.pie_error_threshold });
    let mut a = base_arm(id);
    match id {
        // Naive: miss -> origin every time, no coalescing, no stale, no limiter.
        "BL-0" => a.coalesce = false,
        // Single-flight only.
        "BL-1" => {}
        // Standalone AIMD limiter (product controller), nothing else.
        "BL-4" => {
            a.cap = CapConfig::Aimd { min: p.bl4_min, max: p.bl4_max, p99_ceiling_ms: p.bl4_p99_ceiling_ms, window: p.aimd_window };
        }
        // Circuit breaker (lab component) + stale-if-error.
        "BL-5" => {
            a.breaker = Some(BreakerCfg { err_threshold: p.bl5_err_threshold, min_samples: 8, open_ms: p.bl5_open_ms });
            a.stale_on_error = true;
            a.static_stale_ms = p.bl12b_stale_ms;
        }
        // Per-scenario oracle static (cap, stale window) from the tuning grid.
        "BL-6" => {
            let (cap, stale) = p.bl6_by_scenario.get(scenario_id).copied().unwrap_or((p.bl12b_cap, p.bl12b_stale_ms));
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = stale;
            a.cap = CapConfig::Static { n: cap };
        }
        // Static baseline: the best configuration found in the documented grid on tuning seeds T (not claimed to be the best deployable static policy): single-flight + static SWR window + static cap.
        "BL-12b" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.bl12b_stale_ms;
            a.cap = CapConfig::Static { n: p.bl12b_cap };
        }
        // Adaptive arm: AIMD cap + dynamic stale window + TinyLFU admission (all via the product executor).
        "PIE-Retained" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.cap = pie_cap;
            a.dyn_stale = pie_dyn;
            a.tinylfu = true;
        }
        // Ablations (H2).
        "PIE-no-stale" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.cap = pie_cap;
            a.tinylfu = true;
        }
        "PIE-no-cap" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.dyn_stale = pie_dyn;
            a.tinylfu = true;
        }
        "PIE-evict-only" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.tinylfu = true;
        }
        // Sensitivity (a): identical configuration to PIE-Retained, executor installed but Mode::Off.
        "PIE-off" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.cap = pie_cap;
            a.dyn_stale = pie_dyn;
            a.tinylfu = true;
            a.executor_mode_off = true;
        }
        // Control for sensitivity (a): PIE's static substrate with no adaptive component at all.
        "CTRL-nonadaptive" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
        }
        // Sensitivity (b): deliberately better configuration for peak concurrency (very low static cap).
        "PLANT-CAP4" => {
            a.cap = CapConfig::Static { n: 4 };
        }
        // Candidate-level comparisons (intel_candidates_tests.rs). Each pair differs in exactly one component.
        "CAND-STATIC-STALE" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
        }
        "CAND-DYN-STALE" => {
            a.swr = true;
            a.stale_on_error = true;
            a.static_stale_ms = p.pie_base_stale_ms;
            a.dyn_stale = pie_dyn;
        }
        "CAND-STATIC-CAP128" => a.cap = CapConfig::Static { n: 128 },
        "CAND-AIMD" => {
            a.cap = CapConfig::Aimd { min: p.pie_min, max: 128, p99_ceiling_ms: p.pie_p99_ceiling_ms, window: p.pie_window };
        }
        "CAND-NOADM" => {}
        "CAND-ADM" => a.tinylfu = true,
        other => panic!("unknown arm id {other}"),
    }
    a
}

/// Fixed simulation parameters (frozen in preregistration_m6_v2.md section 7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimParams {
    pub rate_rps: f64,
    pub catalog: usize,
    pub zipf_alpha: f64,
    pub ttl_ms: u64,
    pub body_bytes: usize,
    pub shards: usize,
    pub entries_per_shard: usize,
    pub total_byte_budget: usize,
    pub origin_servers: usize,
    pub origin_service_us: u64,
    pub warmup_us: u64,
    pub measure_us: u64,
    pub client_timeout_us: u64,
    pub client_proc_us: u64,
    pub ctl_interval_us: u64,
    pub retry_max: u32,
    pub retry_backoff_us: u64,
}

impl SimParams {
    pub fn standard() -> Self {
        Self {
            rate_rps: 250.0,
            catalog: 400,
            zipf_alpha: 0.95,
            ttl_ms: 3_000,
            body_bytes: 1024,
            shards: 4,
            entries_per_shard: 40,
            total_byte_budget: 208_000,
            origin_servers: 8,
            origin_service_us: 60_000,
            warmup_us: 8_000_000,
            measure_us: 20_000_000,
            client_timeout_us: 1_000_000,
            client_proc_us: 300,
            ctl_interval_us: 1_000_000,
            retry_max: 3,
            retry_backoff_us: 100_000,
        }
    }

    /// Short horizon used by smoke tests (same mechanics, fewer events).
    pub fn smoke() -> Self {
        let mut p = Self::standard();
        p.warmup_us = 5_000_000;
        p.measure_us = 10_000_000;
        p
    }
}

/// Runtime extras of a scenario that are not origin-side.
struct ScenarioExtras {
    origin: DegradationScenario,
    restart_at_us: Option<u64>,
    burst: Option<(u64, u64, f64)>,
    retries: bool,
}

/// Maps a scenario id to a real origin degradation (plus runner-side extras). Every M6 id is implemented.
fn scenario_for(id: &str, p: &SimParams) -> ScenarioExtras {
    let t0 = p.warmup_us;
    let end = p.warmup_us + p.measure_us;
    let sec = 1_000_000u64;
    let mut x = ScenarioExtras { origin: DegradationScenario::None, restart_at_us: None, burst: None, retries: false };
    match id {
        "O-NONE" => {}
        // Step latency degradation: x3 service time from t0.
        "O-DEG" => x.origin = DegradationScenario::LatencyInflation { at_us: t0, factor: 3.0 },
        // Brownout: 30% of origin requests fail (HTTP 500) for 10 s.
        "O-BRN" => x.origin = DegradationScenario::Brownout { start_us: t0, end_us: (t0 + 10 * sec).min(end), error_rate: 0.30 },
        // Slow recovery: full outage for 4 s, then capacity ramps from 25% to 100% over 8 s.
        "O-SLW" => {
            x.origin = DegradationScenario::Composite(vec![
                DegradationScenario::Outage { start_us: t0, end_us: t0 + 4 * sec },
                DegradationScenario::CapacityRecovery { start_us: t0 + 4 * sec, end_us: t0 + 12 * sec, initial_capacity_factor: 0.25 },
            ])
        }
        // Cold restart stampede: cache invalidated at t0 (product invalidate_all) and 2x arrival burst for 5 s.
        "O-CO" => {
            x.restart_at_us = Some(t0);
            x.burst = Some((t0, t0 + 5 * sec, 2.0));
        }
        // Client retry storm: origin 2.5x slower from t0 and clients retry failures/timeouts (<=3, 100 ms backoff).
        "O-RT" => {
            x.origin = DegradationScenario::LatencyInflation { at_us: t0, factor: 2.5 };
            x.retries = true;
        }
        // Flapping origin health: 4 s cycle, 1.5 s down (HTTP 503).
        "O-FL" => x.origin = DegradationScenario::Flapping { start_us: t0, end_us: end, cycle_us: 4 * sec, down_duration_us: 1_500_000 },
        // Heterogeneous per-key origin cost for the whole run (no step).
        "O-HET" => x.origin = DegradationScenario::HeterogeneousCost,
        // Gradual linear latency inflation from 1x to 4x over the measurement window.
        "O-GRD" => x.origin = DegradationScenario::GradualInflation { start_us: t0, end_us: end, final_factor: 4.0 },
        // Legacy ids used by the M4 matrix tests.
        "O-ST" => x.origin = DegradationScenario::CapacityStepDown { at_us: t0, factor: 0.25 },
        "O-BR" => x.origin = DegradationScenario::Brownout { start_us: t0, end_us: t0 + 6 * sec, error_rate: 0.20 },
        "O-OUT" => x.origin = DegradationScenario::Outage { start_us: t0, end_us: t0 + 2 * sec },
        other => panic!("unknown scenario id {other}"),
    }
    x
}

pub const M6_SCENARIOS: [&str; 8] = ["O-DEG", "O-BRN", "O-SLW", "O-CO", "O-RT", "O-FL", "O-HET", "O-GRD"];
pub const TUNING_FAMILIES: [&str; 3] = ["W-Z", "W-FC", "W-SC"];
pub const HELD_OUT_FAMILIES: [&str; 4] = ["W-DI", "W-SH", "W-MX", "W-HK"];
pub const SEALED_FAMILY: &str = "W-SF";

/// W-SF: adversarial non-stationary bimodal workload with sudden phase shifts (introduced for M6).
/// Phases of 2.5 s alternate between a narrow hot set (Zipf 1.3 over 40 keys) and a wide uniform sweep
/// over a disjoint key range 4x the size of the hot catalog.
struct AdversarialBimodal {
    hot: ZipfWorkload,
    wide: UniformWorkload,
    phase_us: u64,
}

impl WorkloadGenerator for AdversarialBimodal {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        if (now_us / self.phase_us).is_multiple_of(2) {
            self.hot.next_request(now_us, rng)
        } else {
            let mut r = self.wide.next_request(now_us, rng);
            r.key = r.key.replace("/key_", "/sf_wide_");
            r
        }
    }
}

pub fn make_workload(family: &str, p: &SimParams) -> Box<dyn WorkloadGenerator + Send> {
    let rate = p.rate_rps;
    let cat = p.catalog;
    let a = p.zipf_alpha;
    let t0 = p.warmup_us;
    let mid = p.warmup_us + p.measure_us / 2;
    let fam = family.split('(').next().unwrap_or(family);
    match fam {
        "W-Z" => {
            let alpha = family
                .split('(')
                .nth(1)
                .and_then(|s| s.trim_end_matches(')').parse::<f64>().ok())
                .unwrap_or(a);
            Box::new(ZipfWorkload::new(cat, alpha, rate))
        }
        "W-FC" => Box::new(FlashCrowdWorkload::new(cat, a, rate, t0 + 2_000_000, t0 + 8_000_000, 2.0, 8)),
        "W-SC" => Box::new(ScanWorkload::new(cat, a, 0.2, rate)),
        "W-DI" => Box::new(DiurnalWorkload::new(cat, a, rate, 0.5, p.measure_us / 2)),
        "W-SH" => Box::new(HotSetShiftWorkload::new(cat, a, mid, cat / 2, rate)),
        "W-MX" => Box::new(MixtureWorkload::new(vec![
            (Box::new(ZipfWorkload::new(cat, a, rate)) as Box<dyn WorkloadGenerator + Send>, 0.9),
            (Box::new(InvalidationWorkload::new(cat, a, rate, 0.5)) as Box<dyn WorkloadGenerator + Send>, 0.1),
        ])),
        "W-HK" => Box::new(HotKeyWorkload::new(cat, a, rate, "/hot_emerging".to_string(), t0, 0.5, 0.4)),
        "W-SF" => Box::new(AdversarialBimodal {
            hot: ZipfWorkload::new(40, 1.3, rate),
            wide: UniformWorkload::new(cat * 4, rate),
            phase_us: 2_500_000,
        }),
        other => panic!("unknown workload family {other} (W-TR is UNAVAILABLE: no trace data in the repository)"),
    }
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellSummary {
    pub cell_id: String,
    pub tier: String,
    pub scenario: String,
    pub family: String,
    pub arm: String,
    pub seed_label: String,
    pub requests: u64,
    pub fresh_hits: u64,
    pub miss_served: u64,
    pub stale_served: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub shed: u64,
    pub origin_dispatches: u64,
    pub origin_errors: u64,
    pub peak_outstanding: u64,
    pub peak_servers_busy: u64,
    pub cap_violations: u64,
    pub stale_cap_violations_counter: u64,
    pub stale_cap_violations_indep: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p99_success_us: u64,
    pub mean_stale_age_ms: f64,
    pub fresh_hit_ratio: f64,
    pub served_from_cache_ratio: f64,
    pub error_rate: f64,
    pub final_cap: i64,
    pub max_stale_ms_final: u64,
    pub bytes_audit_equal: bool,
    pub events_processed: u64,
    pub attempts_total: u64,
    pub stale_window_changes: u64,
    pub cap_changes: u64,
    /// Count of internal simulator invariant violations recorded in this cell (missing op, outstanding underflow, panic). Must be 0.
    pub invariant_violations: u64,
}

pub const CELL_CSV_HEADER: &str = "cell_id,tier,scenario,family,arm,seed_label,requests,fresh_hits,miss_served,stale_served,failures,timeouts,shed,origin_dispatches,origin_errors,peak_outstanding,peak_servers_busy,cap_violations,stale_cap_violations_counter,stale_cap_violations_indep,p50_us,p95_us,p99_us,p99_success_us,mean_stale_age_ms,fresh_hit_ratio,served_from_cache_ratio,error_rate,final_cap,max_stale_ms_final,bytes_audit_equal,events_processed,attempts_total,stale_window_changes,cap_changes,invariant_violations";

impl CellSummary {
    pub fn csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.9},{:.9},{:.9},{},{},{},{},{},{},{},{}",
            self.cell_id, self.tier, self.scenario, self.family, self.arm, self.seed_label, self.requests,
            self.fresh_hits, self.miss_served, self.stale_served, self.failures, self.timeouts, self.shed,
            self.origin_dispatches, self.origin_errors, self.peak_outstanding, self.peak_servers_busy,
            self.cap_violations, self.stale_cap_violations_counter, self.stale_cap_violations_indep,
            self.p50_us, self.p95_us, self.p99_us, self.p99_success_us, self.mean_stale_age_ms,
            self.fresh_hit_ratio, self.served_from_cache_ratio, self.error_rate, self.final_cap,
            self.max_stale_ms_final, self.bytes_audit_equal, self.events_processed, self.attempts_total,
            self.stale_window_changes, self.cap_changes, self.invariant_violations
        )
    }

    pub fn from_csv_row(row: &str) -> Option<Self> {
        let f: Vec<&str> = row.trim().split(',').collect();
        if f.len() != 36 {
            return None;
        }
        Some(Self {
            cell_id: f[0].to_string(),
            tier: f[1].to_string(),
            scenario: f[2].to_string(),
            family: f[3].to_string(),
            arm: f[4].to_string(),
            seed_label: f[5].to_string(),
            requests: f[6].parse().ok()?,
            fresh_hits: f[7].parse().ok()?,
            miss_served: f[8].parse().ok()?,
            stale_served: f[9].parse().ok()?,
            failures: f[10].parse().ok()?,
            timeouts: f[11].parse().ok()?,
            shed: f[12].parse().ok()?,
            origin_dispatches: f[13].parse().ok()?,
            origin_errors: f[14].parse().ok()?,
            peak_outstanding: f[15].parse().ok()?,
            peak_servers_busy: f[16].parse().ok()?,
            cap_violations: f[17].parse().ok()?,
            stale_cap_violations_counter: f[18].parse().ok()?,
            stale_cap_violations_indep: f[19].parse().ok()?,
            p50_us: f[20].parse().ok()?,
            p95_us: f[21].parse().ok()?,
            p99_us: f[22].parse().ok()?,
            p99_success_us: f[23].parse().ok()?,
            mean_stale_age_ms: f[24].parse().ok()?,
            fresh_hit_ratio: f[25].parse().ok()?,
            served_from_cache_ratio: f[26].parse().ok()?,
            error_rate: f[27].parse().ok()?,
            final_cap: f[28].parse().ok()?,
            max_stale_ms_final: f[29].parse().ok()?,
            bytes_audit_equal: f[30].parse().ok()?,
            events_processed: f[31].parse().ok()?,
            attempts_total: f[32].parse().ok()?,
            stale_window_changes: f[33].parse().ok()?,
            cap_changes: f[34].parse().ok()?,
            invariant_violations: f[35].parse().ok()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub req_id: u64,
    pub arrival_us: u64,
    pub key: String,
    pub outcome: &'static str,
    pub latency_us: u64,
    pub attempts: u32,
    pub stale_age_ms: u64,
}

pub const REQ_CSV_HEADER: &str = "cell_id,req_id,arrival_us,key,outcome,latency_us,attempts,stale_age_ms";

#[derive(Debug, Clone)]
pub struct FetchRecord {
    pub dispatch_us: u64,
    pub finish_us: u64,
    pub status: u16,
    pub outstanding_after_dispatch: u64,
    pub background: bool,
}

pub const FETCH_CSV_HEADER: &str = "cell_id,dispatch_us,finish_us,status,outstanding_after_dispatch,background";

#[derive(Debug, Clone)]
pub struct CellOutput {
    pub summary: CellSummary,
    pub requests: Vec<RequestRecord>,
    pub fetches: Vec<FetchRecord>,
    /// (virtual time us, dynamic stale window ms, gate cap) per control tick.
    pub control_trace: Vec<(u64, u64, i64)>,
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Hit,
    Miss,
    MissWait,
    Stale,
    Timeout,
    Shed,
    OriginErr,
    Reject,
}

impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Outcome::Hit => "HIT",
            Outcome::Miss => "MISS",
            Outcome::MissWait => "MISS_WAIT",
            Outcome::Stale => "STALE",
            Outcome::Timeout => "TIMEOUT",
            Outcome::Shed => "SHED",
            Outcome::OriginErr => "ORIGIN_ERR",
            Outcome::Reject => "REJECT",
        }
    }
    fn is_failure(self) -> bool {
        matches!(self, Outcome::Timeout | Outcome::Shed | Outcome::OriginErr | Outcome::Reject)
    }
}

enum Ev {
    Arrive { key: String, body: usize, invalidation: bool },
    Attempt { req: usize },
    Timeout { req: usize },
    FetchDone { op: usize },
    Ctl,
    Restart,
}

struct KeyInfo {
    path: String,
    ck: CacheKey,
    hash: u64,
}

struct Req {
    root_arrival_us: u64,
    attempt: u32,
    key: usize,
    body: usize,
    done: bool,
    in_window: bool,
    root_id: u64,
}

struct Op {
    guard: Option<OriginFetchGuard>,
    permit: Option<OriginPermit>,
    key: usize,
    dispatched_us: u64,
    finish_us: u64,
    status: u16,
    is_error: bool,
    leader: Option<usize>,
    body: usize,
}

#[derive(Default)]
struct Breaker {
    open_until_us: u64,
    window: Vec<bool>,
}

struct Sim<'a> {
    p: &'a SimParams,
    arm: ArmConfig,
    cache: Arc<PartitionedCache>,
    exec: Option<Arc<DefaultPolicyExecutor>>,
    exec_active: bool,
    origin: SimulatedOrigin,
    rng_origin: StreamRng,
    queue: EventQueue<Ev>,
    keys: Vec<KeyInfo>,
    key_ids: HashMap<String, usize>,
    reqs: Vec<Req>,
    ops: Vec<Option<Op>>,
    pending: HashMap<usize, Vec<(usize, oneshot::Receiver<crate::core::coalesce::CoalesceResult>)>>,
    inflight: HashSet<usize>,
    outstanding: u64,
    filter: FrequencyAdmissionFilter,
    stale_ctl: Option<OriginAwareStaleController>,
    win_stats: Vec<(u32, bool)>,
    breaker: Breaker,
    retries_enabled: bool,
    t0: u64,
    record_raw: bool,
    // metrics
    events: u64,
    attempts_total: u64,
    next_root: u64,
    outcomes: Vec<(Outcome, u64, bool)>, // (outcome, latency_us, success)
    stale_ages_ms: Vec<u64>,
    peak_outstanding: u64,
    cap_violations: u64,
    stale_viol_indep: u64,
    origin_dispatches: u64,
    origin_errors: u64,
    shed: u64,
    timeouts: u64,
    records: Vec<RequestRecord>,
    fetch_records: Vec<FetchRecord>,
    control_trace: Vec<(u64, u64, i64)>,
    invariant_violations: u64,
}

impl<'a> Sim<'a> {
    fn intern(&mut self, path: &str) -> usize {
        if let Some(&i) = self.key_ids.get(path) {
            return i;
        }
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let i = self.keys.len();
        self.keys.push(KeyInfo { path: path.to_string(), ck: CacheKey::new(None, path, None), hash: h });
        self.key_ids.insert(path.to_string(), i);
        i
    }

    fn stale_cap_ms(&self) -> u64 {
        self.cache.get_max_stale_ms()
    }

    fn gate_cap(&self) -> Option<usize> {
        match &self.exec {
            Some(e) if e.origin_cap_enforcing() => e.origin_gate().current_cap(),
            _ => None,
        }
    }

    fn start_attempt(&mut self, req: usize, t: u64) {
        let ki = self.reqs[req].key;
        let now_ms = t / 1000;
        self.attempts_total += 1;
        if self.arm.tinylfu && self.exec_active {
            let h = self.keys[ki].hash;
            self.filter.observe(h);
        }
        let ck = self.keys[ki].ck.clone();

        // Stale-while-revalidate: serve stale immediately (mirrors the product GET handler order:
        // fresh lookup, then stale check, then single-flight fetch).
        if self.arm.swr && self.cache.get_resource(&ck, now_ms).is_none() {
            let cap = self.stale_cap_ms();
            if let StaleServingDecision::Eligible(res, _) = self.cache.check_stale_serving(&ck, now_ms, cap, false) {
                self.serve_stale(req, &res, t, cap);
                if !self.inflight.contains(&ki) && self.cache.is_revalidation_due(&ck, now_ms) {
                    if let FetchRegistrationResult::Initiate(reg) = self.cache.begin_fetch_atomic(&ck, now_ms) {
                        self.leader_dispatch(None, ki, reg, t, self.reqs[req].body);
                    }
                }
                return;
            }
        }

        match self.cache.begin_fetch_atomic(&ck, now_ms) {
            FetchRegistrationResult::Hit(_) => self.resolve(req, Outcome::Hit, t, 0),
            FetchRegistrationResult::Wait(rx) => {
                if self.arm.coalesce {
                    self.pending.entry(ki).or_default().push((req, rx));
                } else {
                    drop(rx);
                    self.direct_dispatch(req, ki, t);
                }
            }
            FetchRegistrationResult::Initiate(reg) => {
                let body = self.reqs[req].body;
                self.leader_dispatch(Some(req), ki, reg, t, body)
            }
            FetchRegistrationResult::Rejected(_) => {
                self.error_path(req, t, Outcome::Reject);
            }
        }
    }

    fn serve_stale(&mut self, req: usize, res: &CachedResource, t: u64, cap: u64) {
        let now_ms = t / 1000;
        let age_ms = now_ms.saturating_sub(res.created_at_ms);
        self.cache.record_stale_served(age_ms, res.ttl_ms, cap);
        if age_ms > res.ttl_ms.saturating_add(cap) {
            self.stale_viol_indep += 1;
        }
        self.resolve(req, Outcome::Stale, t, age_ms);
    }

    /// Origin unavailable for this request (error, shed, rejection): stale-if-error else fail.
    fn error_path(&mut self, req: usize, t: u64, fail: Outcome) {
        if self.reqs[req].done {
            return;
        }
        if self.arm.stale_on_error {
            let ki = self.reqs[req].key;
            let ck = self.keys[ki].ck.clone();
            let cap = self.stale_cap_ms();
            if let StaleServingDecision::Eligible(res, _) = self.cache.check_stale_serving(&ck, t / 1000, cap, true) {
                self.serve_stale(req, &res, t, cap);
                return;
            }
        }
        self.resolve(req, fail, t, 0);
    }

    fn breaker_open(&self, t: u64) -> bool {
        self.arm.breaker.is_some() && t < self.breaker.open_until_us
    }

    fn breaker_record(&mut self, ok: bool, t: u64) {
        if let Some(cfg) = self.arm.breaker.clone() {
            self.breaker.window.push(ok);
            if self.breaker.window.len() > 20 {
                self.breaker.window.remove(0);
            }
            if self.breaker.window.len() >= cfg.min_samples {
                let errs = self.breaker.window.iter().filter(|&&o| !o).count() as f64;
                if errs / self.breaker.window.len() as f64 >= cfg.err_threshold {
                    self.breaker.open_until_us = t + cfg.open_ms * 1000;
                    self.breaker.window.clear();
                }
            }
        }
    }

    /// Single-flight leader (or background refresh when `leader` is None) wants to go to the origin.
    fn leader_dispatch(
        &mut self,
        leader: Option<usize>,
        ki: usize,
        reg: FetchRegistration,
        t: u64,
        body: usize,
    ) {
        if self.breaker_open(t) {
            drop(reg);
            self.shed += 1;
            if let Some(r) = leader {
                self.error_path(r, t, Outcome::Shed);
            }
            return;
        }
        let permit = match &self.exec {
            Some(e) => match e.try_acquire_origin_permit() {
                Ok(p) => Some(p),
                Err(()) => {
                    drop(reg);
                    self.shed += 1;
                    if let Some(r) = leader {
                        self.error_path(r, t, Outcome::Shed);
                    }
                    return;
                }
            },
            None => None,
        };
        // Independent cap check from executed events (dispatch accepted while at/over the cap in force).
        if let Some(cap) = self.gate_cap() {
            if self.outstanding as usize >= cap {
                self.cap_violations += 1;
            }
        }
        let guard = reg.into_guard();
        self.dispatch(Some(guard), permit, leader, ki, t, body);
    }

    /// BL-0: independent fetch that is not coalesced and not cached.
    fn direct_dispatch(&mut self, req: usize, ki: usize, t: u64) {
        let body = self.reqs[req].body;
        self.dispatch(None, None, Some(req), ki, t, body);
    }

    fn dispatch(
        &mut self,
        guard: Option<OriginFetchGuard>,
        permit: Option<OriginPermit>,
        leader: Option<usize>,
        ki: usize,
        t: u64,
        body: usize,
    ) {
        let path = self.keys[ki].path.clone();
        let resp = self.origin.fetch(t, &path, body, &mut self.rng_origin);
        self.outstanding += 1;
        if t >= self.t0 {
            self.peak_outstanding = self.peak_outstanding.max(self.outstanding);
            self.origin_dispatches += 1;
        }
        if guard.is_some() {
            self.inflight.insert(ki);
        }
        if self.record_raw && t >= self.t0 {
            self.fetch_records.push(FetchRecord {
                dispatch_us: t,
                finish_us: resp.finish_service_us,
                status: resp.status_code,
                outstanding_after_dispatch: self.outstanding,
                background: leader.is_none(),
            });
        }
        let op = Op {
            guard,
            permit,
            key: ki,
            dispatched_us: t,
            finish_us: resp.finish_service_us,
            status: resp.status_code,
            is_error: resp.is_error,
            leader,
            body,
        };
        let id = self.ops.len();
        self.ops.push(Some(op));
        self.queue.schedule(resp.finish_service_us.max(t), Ev::FetchDone { op: id });
    }

    fn on_fetch_done(&mut self, id: usize, t: u64) {
        // Protocol v2.2 (finding C1b): an internal inconsistency must never abort a one-shot run. It is
        // counted in the cell (`invariant_violations`), the event is dropped, and the run is failed loudly
        // by the caller AFTER the data have been recorded.
        let Some(mut op) = self.ops.get_mut(id).and_then(|o| o.take()) else {
            self.invariant_violations += 1;
            return;
        };
        if self.outstanding == 0 {
            self.invariant_violations += 1;
        } else {
            self.outstanding -= 1;
        }
        let ok = !op.is_error;
        let lat_ms = ((op.finish_us.saturating_sub(op.dispatched_us)) / 1000) as u32;
        if op.dispatched_us >= self.t0 && !ok {
            self.origin_errors += 1;
        }
        self.win_stats.push((lat_ms, !ok));
        if let Some(p) = op.permit.take() {
            p.observe(lat_ms as u64, !ok);
            drop(p);
        }
        self.breaker_record(ok, t);

        let ki = op.key;
        let ck = self.keys[ki].ck.clone();
        let waiters = self.pending.remove(&ki).unwrap_or_default();

        if let Some(guard) = op.guard.take() {
            let batch = if ok {
                let res = Arc::new(CachedResource {
                    status: 200,
                    content_type: "text/plain".to_string(),
                    body: Arc::new(vec![0u8; op.body]),
                    etag: None,
                    created_at_ms: t / 1000,
                    ttl_ms: self.p.ttl_ms,
                });
                let admit = self.admit(ki);
                let outcome = if admit { FinalizeOutcome::Success(res) } else { FinalizeOutcome::Uncacheable(res) };
                let b = guard.complete(outcome);
                if admit && (self.arm.swr || self.arm.stale_on_error) {
                    self.cache.update_freshness_metadata(
                        &ck,
                        FreshnessMetadata {
                            must_revalidate: false,
                            stale_while_revalidate_ms: if self.arm.swr { Some(MAX_HARD_STALE_MS) } else { None },
                            stale_if_error_ms: if self.arm.stale_on_error { Some(MAX_HARD_STALE_MS) } else { None },
                            last_modified: None,
                            date_ms: None,
                            initial_age_ms: 0,
                        },
                    );
                }
                b
            } else {
                self.cache.record_revalidation_failure(&ck, t / 1000);
                guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::UpstreamStatus(op.status))))
            };
            if let Some(mut b) = batch {
                b.deliver();
            }
            self.inflight.remove(&ki);
        }

        if let Some(r) = op.leader {
            if !self.reqs[r].done {
                if ok {
                    self.resolve(r, Outcome::Miss, t, 0);
                } else {
                    self.error_path(r, t, Outcome::OriginErr);
                }
            }
        }
        for (r, mut rx) in waiters {
            if self.reqs[r].done {
                continue;
            }
            match rx.try_recv() {
                Ok(Ok(_)) => self.resolve(r, Outcome::MissWait, t, 0),
                _ => self.error_path(r, t, Outcome::OriginErr),
            }
        }
    }

    /// TinyLFU-style admission: when the cache is at its entry capacity, admit only keys the
    /// product frequency sketch has seen at least twice. Effective only while the executor is Active.
    fn admit(&self, ki: usize) -> bool {
        if !(self.arm.tinylfu && self.exec_active) {
            return true;
        }
        let resident: usize = self.cache.shards.iter().map(|s| s.lock().map(|g| g.map.len()).unwrap_or(0)).sum();
        let full = resident >= self.p.shards * self.p.entries_per_shard * 9 / 10;
        !full || self.filter.frequency_sketch.estimate_frequency(self.keys[ki].hash) >= 2
    }

    fn resolve(&mut self, req: usize, outcome: Outcome, t: u64, stale_age_ms: u64) {
        if self.reqs[req].done {
            return;
        }
        self.reqs[req].done = true;
        if outcome == Outcome::Timeout {
            self.timeouts += 1;
        }
        let (root_arrival, attempt, in_window, key, body, root_id) = {
            let r = &self.reqs[req];
            (r.root_arrival_us, r.attempt, r.in_window, r.key, r.body, r.root_id)
        };
        if outcome.is_failure() && self.retries_enabled && attempt < self.p.retry_max {
            let nr = self.reqs.len();
            self.reqs.push(Req { root_arrival_us: root_arrival, attempt: attempt + 1, key, body, done: false, in_window, root_id });
            self.queue.schedule(t + self.p.retry_backoff_us, Ev::Attempt { req: nr });
            return;
        }
        let proc = if outcome.is_failure() { 0 } else { self.p.client_proc_us };
        let latency = t.saturating_sub(root_arrival) + proc;
        if in_window {
            if outcome == Outcome::Shed {
                // counted in self.shed at gate time
            }
            self.outcomes.push((outcome, latency, !outcome.is_failure()));
            if outcome == Outcome::Stale {
                self.stale_ages_ms.push(stale_age_ms);
            }
            if self.record_raw {
                self.records.push(RequestRecord {
                    req_id: root_id,
                    arrival_us: root_arrival,
                    key: self.keys[key].path.clone(),
                    outcome: outcome.label(),
                    latency_us: latency,
                    attempts: attempt + 1,
                    stale_age_ms,
                });
            }
        }
    }

    fn on_ctl(&mut self, t: u64) {
        let stats = std::mem::take(&mut self.win_stats);
        if let (Some(ctl), Some(exec)) = (self.stale_ctl.as_mut(), self.exec.as_ref()) {
            let n = stats.len();
            let err_rate = if n == 0 { 0.0 } else { stats.iter().filter(|s| s.1).count() as f64 / n as f64 };
            let mut lats: Vec<u32> = stats.iter().map(|s| s.0).collect();
            lats.sort_unstable();
            let p99 = if lats.is_empty() { 0 } else { lats[((lats.len() as f64 * 0.99).ceil() as usize).clamp(1, lats.len()) - 1] };
            let v = ctl.update(err_rate, p99);
            let cand = PolicyCandidate {
                candidate_id: "dynamic-stale-window".to_string(),
                kind: PolicyKind::StaleServing,
                parameters: [("max_stale_ms".to_string(), v as f64)].into_iter().collect(),
                bounds: BTreeMap::new(),
                expected_effect: "extend stale window while origin is degraded".to_string(),
            };
            let _ = exec.apply_policy(&cand); // Err when executor is Off: window stays static
        }
        let cap = self.exec.as_ref().and_then(|e| e.origin_gate().current_cap()).map(|c| c as i64).unwrap_or(-1);
        self.control_trace.push((t, self.cache.get_max_stale_ms(), cap));
    }
}

fn percentile(sorted: &[u64], q: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((sorted.len() as f64) * q).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Specification of one cell.
#[derive(Debug, Clone)]
pub struct CellSpec {
    pub scenario: String,
    pub family: String,
    pub arm: ArmConfig,
    pub seed: u64,
    /// Label written to outputs. For the sealed set this is an index label, never the seed value.
    pub seed_label: String,
    pub params: SimParams,
    pub record_raw: bool,
}

impl CellSpec {
    pub fn cell_id(&self) -> String {
        format!("{}|{}|{}|{}", self.scenario, self.family, self.arm.id, self.seed_label)
    }
}

/// Runs one deterministic T1 cell against the real product components.
pub fn run_cell(spec: &CellSpec) -> CellOutput {
    let p = &spec.params;
    let arm = spec.arm.clone();
    let extras = scenario_for(&spec.scenario, p);
    let master = MasterRng::new(spec.seed);
    let mut rng_wl = master.stream("workload");
    let rng_origin = master.stream("origin");

    // Real cache with entry and byte limits.
    let cache = Arc::new(PartitionedCache::new(p.shards));
    cache.set_capacity_limits(p.entries_per_shard, 100_000);
    cache.set_byte_budget(p.total_byte_budget);
    cache.set_max_stale_ms(if arm.dyn_stale.is_some() || arm.swr || arm.stale_on_error { arm.static_stale_ms } else { 0 });

    // Real executor + origin gate.
    let mut exec: Option<Arc<DefaultPolicyExecutor>> = None;
    let mut exec_active = false;
    if arm.uses_executor() {
        let e = Arc::new(DefaultPolicyExecutor::new(Some(cache.clone())));
        match &arm.cap {
            CapConfig::None => {}
            CapConfig::Static { n } => e.configure_origin_cap(*n, *n, u32::MAX, 16),
            CapConfig::Aimd { min, max, p99_ceiling_ms, window } => {
                e.configure_origin_cap(*min, *max, *p99_ceiling_ms, *window)
            }
        }
        if arm.executor_mode_off {
            e.set_mode(IntelligenceMode::Off).unwrap();
        } else {
            e.set_mode(IntelligenceMode::Active).unwrap();
            exec_active = true;
        }
        exec = Some(e);
    }

    let origin = SimulatedOrigin::new(p.origin_servers, ServiceDistribution::Constant(p.origin_service_us))
        .with_scenario(extras.origin.clone())
        // Common random numbers for brownout faults (v2.2 M2): same fate for the same (seed, key, 10 ms bucket) in every arm.
        .with_fault_crn(spec.seed, 10_000);

    let stale_ctl = arm.dyn_stale.as_ref().map(|d| OriginAwareStaleController::new(d.base_ms, d.error_threshold));

    let mut sim = Sim {
        p,
        arm: arm.clone(),
        cache: cache.clone(),
        exec,
        exec_active,
        origin,
        rng_origin,
        queue: EventQueue::new(),
        keys: Vec::new(),
        key_ids: HashMap::new(),
        reqs: Vec::new(),
        ops: Vec::new(),
        pending: HashMap::new(),
        inflight: HashSet::new(),
        outstanding: 0,
        filter: FrequencyAdmissionFilter::new(),
        stale_ctl,
        win_stats: Vec::new(),
        breaker: Breaker::default(),
        retries_enabled: extras.retries,
        t0: p.warmup_us,
        record_raw: spec.record_raw,
        events: 0,
        attempts_total: 0,
        next_root: 0,
        outcomes: Vec::new(),
        stale_ages_ms: Vec::new(),
        peak_outstanding: 0,
        cap_violations: 0,
        stale_viol_indep: 0,
        origin_dispatches: 0,
        origin_errors: 0,
        shed: 0,
        timeouts: 0,
        records: Vec::new(),
        fetch_records: Vec::new(),
        control_trace: Vec::new(),
        invariant_violations: 0,
    };

    let end_us = p.warmup_us + p.measure_us;
    let mut workload = make_workload(&spec.family, p);

    // Seed the first arrival.
    let first = workload.next_request(0, &mut rng_wl);
    let mut next_arrival_us = first.arrival_delta_us;
    if next_arrival_us < end_us {
        sim.queue.schedule(
            next_arrival_us,
            Ev::Arrive { key: first.key, body: first.body_size_bytes, invalidation: first.is_invalidation },
        );
    }
    if let Some(r) = extras.restart_at_us {
        sim.queue.schedule(r, Ev::Restart);
    }
    if sim.stale_ctl.is_some() || sim.exec.is_some() {
        sim.queue.schedule(p.ctl_interval_us, Ev::Ctl);
    }

    let hard_stop_us = end_us + 20 * p.client_timeout_us; // drain bound
    while let Some(ev) = sim.queue.pop() {
        let t = ev.id.time_us;
        if t > hard_stop_us {
            break;
        }
        sim.events += 1;
        match ev.payload {
            Ev::Arrive { key, body, invalidation } => {
                // Draw the next arrival (open loop: independent of system state).
                let mut next = workload.next_request(t, &mut rng_wl);
                if let Some((bs, be, mult)) = extras.burst {
                    if t >= bs && t < be {
                        next.arrival_delta_us = ((next.arrival_delta_us as f64) / mult).max(1.0) as u64;
                    }
                }
                next_arrival_us = t + next.arrival_delta_us;
                if next_arrival_us < end_us {
                    sim.queue.schedule(
                        next_arrival_us,
                        Ev::Arrive { key: next.key, body: next.body_size_bytes, invalidation: next.is_invalidation },
                    );
                }
                let ki = sim.intern(&key);
                if invalidation {
                    sim.cache.delete(&sim.keys[ki].ck);
                    continue;
                }
                let rid = sim.reqs.len();
                let root_id = sim.next_root;
                sim.next_root += 1;
                sim.reqs.push(Req {
                    root_arrival_us: t,
                    attempt: 0,
                    key: ki,
                    body,
                    done: false,
                    in_window: t >= p.warmup_us,
                    root_id,
                });
                sim.queue.schedule(t + p.client_timeout_us, Ev::Timeout { req: rid });
                sim.start_attempt(rid, t);
            }
            Ev::Attempt { req } => {
                sim.queue.schedule(t + p.client_timeout_us, Ev::Timeout { req });
                sim.start_attempt(req, t);
            }
            Ev::Timeout { req } => sim.resolve(req, Outcome::Timeout, t, 0),
            Ev::FetchDone { op } => sim.on_fetch_done(op, t),
            Ev::Ctl => {
                sim.on_ctl(t);
                if t + p.ctl_interval_us <= end_us {
                    sim.queue.schedule(t + p.ctl_interval_us, Ev::Ctl);
                }
            }
            Ev::Restart => {
                let _ = sim.cache.try_invalidate_all();
            }
        }
    }
    let _ = next_arrival_us;

    // Aggregate from executed events.
    let mut lats: Vec<u64> = sim.outcomes.iter().map(|o| o.1).collect();
    lats.sort_unstable();
    let mut succ: Vec<u64> = sim.outcomes.iter().filter(|o| o.2).map(|o| o.1).collect();
    succ.sort_unstable();
    let n = sim.outcomes.len() as u64;
    let count = |o: Outcome| sim.outcomes.iter().filter(|x| x.0 == o).count() as u64;
    let fresh_hits = count(Outcome::Hit);
    let miss_served = count(Outcome::Miss) + count(Outcome::MissWait);
    let stale_served = count(Outcome::Stale);
    let failures = n - fresh_hits - miss_served - stale_served;
    let (acc, rec) = cache.audit_bytes();
    let mean_stale = if sim.stale_ages_ms.is_empty() {
        0.0
    } else {
        sim.stale_ages_ms.iter().sum::<u64>() as f64 / sim.stale_ages_ms.len() as f64
    };
    let denom = n.max(1) as f64;
    let summary = CellSummary {
        cell_id: spec.cell_id(),
        tier: "SIMULATED".to_string(),
        scenario: spec.scenario.clone(),
        family: spec.family.clone(),
        arm: arm.id.clone(),
        seed_label: spec.seed_label.clone(),
        requests: n,
        fresh_hits,
        miss_served,
        stale_served,
        failures,
        timeouts: sim.outcomes.iter().filter(|o| o.0 == Outcome::Timeout).count() as u64,
        shed: sim.outcomes.iter().filter(|o| o.0 == Outcome::Shed).count() as u64,
        origin_dispatches: sim.origin_dispatches,
        origin_errors: sim.origin_errors,
        peak_outstanding: sim.peak_outstanding.max(1),
        peak_servers_busy: sim.origin.peak_in_flight() as u64,
        cap_violations: sim.cap_violations,
        stale_cap_violations_counter: cache.counters.stale_cap_violations.load(std::sync::atomic::Ordering::Relaxed),
        stale_cap_violations_indep: sim.stale_viol_indep,
        p50_us: percentile(&lats, 0.50),
        p95_us: percentile(&lats, 0.95),
        p99_us: percentile(&lats, 0.99),
        p99_success_us: percentile(&succ, 0.99),
        mean_stale_age_ms: mean_stale,
        fresh_hit_ratio: fresh_hits as f64 / denom,
        served_from_cache_ratio: (fresh_hits + stale_served) as f64 / denom,
        error_rate: failures as f64 / denom,
        final_cap: sim.exec.as_ref().and_then(|e| e.origin_gate().current_cap()).map(|c| c as i64).unwrap_or(-1),
        max_stale_ms_final: cache.get_max_stale_ms(),
        bytes_audit_equal: acc == rec,
        events_processed: sim.events,
        attempts_total: sim.attempts_total,
        stale_window_changes: sim.control_trace.windows(2).filter(|w| w[0].1 != w[1].1).count() as u64,
        cap_changes: sim.control_trace.windows(2).filter(|w| w[0].2 != w[1].2).count() as u64,
        invariant_violations: sim.invariant_violations,
    };
    CellOutput { summary, requests: sim.records, fetches: sim.fetch_records, control_trace: sim.control_trace }
}

/// Placeholder output for a cell whose simulation panicked: the defect is RECORDED (invariant_violations = 1,
/// bytes_audit_equal = false, all counters zero) instead of aborting a one-shot run; callers fail loudly afterwards.
pub fn defect_cell_output(spec: &CellSpec) -> CellOutput {
    let summary = CellSummary {
        cell_id: spec.cell_id(),
        tier: "SIMULATED".to_string(),
        scenario: spec.scenario.clone(),
        family: spec.family.clone(),
        arm: spec.arm.id.clone(),
        seed_label: spec.seed_label.clone(),
        requests: 0,
        fresh_hits: 0,
        miss_served: 0,
        stale_served: 0,
        failures: 0,
        timeouts: 0,
        shed: 0,
        origin_dispatches: 0,
        origin_errors: 0,
        peak_outstanding: 1,
        peak_servers_busy: 0,
        cap_violations: 0,
        stale_cap_violations_counter: 0,
        stale_cap_violations_indep: 0,
        p50_us: 0,
        p95_us: 0,
        p99_us: 0,
        p99_success_us: 0,
        mean_stale_age_ms: 0.0,
        fresh_hit_ratio: 0.0,
        served_from_cache_ratio: 0.0,
        error_rate: 0.0,
        final_cap: -1,
        max_stale_ms_final: 0,
        bytes_audit_equal: false,
        events_processed: 0,
        attempts_total: 0,
        stale_window_changes: 0,
        cap_changes: 0,
        invariant_violations: 1,
    };
    CellOutput { summary, requests: Vec::new(), fetches: Vec::new(), control_trace: Vec::new() }
}
