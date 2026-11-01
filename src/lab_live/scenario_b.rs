//! Scenario B (origin slowdown) end to end on loopback with the REAL gateway process.
//!
//! One experiment = `reps` repetitions x arms (BASE, STATIC-CAP, AIMD-CAP), arms interleaved rep-major with a
//! rotating order, the same seed / arrival schedule for every arm of a repetition. Per run: scripted origin,
//! gateway process, open-loop load, snapshot polling; per experiment: paired analysis and the mechanical
//! rule. Tier label: MEASURED_LOOPBACK. See `docs/demo/scenario_b_artifacts.md`.

use super::analysis::{self, ArmEvidence, Paired, MIN_VALID_RUNS};
use super::gateway::{self, CapConfig, Gateway, GatewaySpec};
use super::loadgen::{self, percentile, ArrivalProcess, ClientRecord, Workload};
use super::origin::{Origin, OriginRecord, OriginScript};
use crate::lab::eval::{git_dirty_entries, git_head, repo_root, sha256_file, utc_now_iso};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const TIER: &str = "MEASURED_LOOPBACK";
pub const CONTENTION_LAG_P99_MS: f64 = 100.0;
const LOG_TAIL_LINES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arm {
    Base,
    StaticCap,
    AimdCap,
}

impl Arm {
    pub fn name(&self) -> &'static str {
        match self {
            Arm::Base => "BASE",
            Arm::StaticCap => "STATIC-CAP",
            Arm::AimdCap => "AIMD-CAP",
        }
    }
    pub fn parse(s: &str) -> Option<Arm> {
        match s.to_ascii_uppercase().as_str() {
            "BASE" => Some(Arm::Base),
            "STATIC-CAP" | "STATIC" => Some(Arm::StaticCap),
            "AIMD-CAP" | "AIMD" => Some(Arm::AimdCap),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Phases {
    pub warmup_ms: u64,
    pub healthy_ms: u64,
    pub degraded_ms: u64,
    pub recovery_ms: u64,
}

impl Phases {
    pub fn total_ms(&self) -> u64 {
        self.warmup_ms + self.healthy_ms + self.degraded_ms + self.recovery_ms
    }
    pub fn degr_start_ms(&self) -> u64 {
        self.warmup_ms + self.healthy_ms
    }
    pub fn degr_end_ms(&self) -> u64 {
        self.degr_start_ms() + self.degraded_ms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioBConfig {
    pub gateway_exe: PathBuf,
    pub out_dir: PathBuf,
    pub reps: usize,
    pub seed: u64,
    pub arms: Vec<Arm>,
    pub phases: Phases,
    pub workload: Workload,
    pub origin: OriginScript,
    pub max_cache_bytes: u64,
    pub static_cap: usize,
    pub aimd_min: usize,
    pub aimd_max: usize,
    pub aimd_p99_ceiling_ms: u64,
    pub aimd_window: usize,
    pub snapshot_interval_ms: u64,
    pub command_line: String,
}

impl ScenarioBConfig {
    /// The documented default experiment (see `docs/demo/scenario_b_runbook.md`).
    pub fn default_experiment(gateway_exe: PathBuf, out_dir: PathBuf) -> Self {
        let phases = Phases { warmup_ms: 8_000, healthy_ms: 5_000, degraded_ms: 10_000, recovery_ms: 8_000 };
        Self::with_phases(gateway_exe, out_dir, phases)
    }

    pub fn with_phases(gateway_exe: PathBuf, out_dir: PathBuf, phases: Phases) -> Self {
        let seed = 20_261_030;
        ScenarioBConfig {
            gateway_exe,
            out_dir,
            reps: 10,
            seed,
            arms: vec![Arm::Base, Arm::StaticCap, Arm::AimdCap],
            phases,
            workload: Workload {
                seed,
                n_keys: 5_000,
                zipf_alpha: 0.9,
                rate_rps: 300.0,
                duration_ms: phases.total_ms(),
                arrival: ArrivalProcess::Poisson,
                path_prefix: "/api/v1/cache/k/".to_string(),
                client_deadline_ms: 10_000,
            },
            origin: OriginScript {
                base_service_ms: 20,
                degraded_mult: 6.0,
                degrade_start_ms: phases.degr_start_ms(),
                degrade_len_ms: phases.degraded_ms,
                error_rate: 0.0,
                max_age_s: 3,
                stale_while_revalidate_s: None,
                stale_if_error_s: None,
                body_bytes: 1024,
                seed,
            },
            max_cache_bytes: 262_144,
            static_cap: 24,
            aimd_min: 12,
            aimd_max: 32,
            aimd_p99_ceiling_ms: 60,
            aimd_window: 30,
            snapshot_interval_ms: 250,
            command_line: String::new(),
        }
    }

    fn sync_derived(&mut self) {
        self.workload.duration_ms = self.phases.total_ms();
        self.origin.degrade_start_ms = self.phases.degr_start_ms();
        self.origin.degrade_len_ms = self.phases.degraded_ms;
    }

    /// Cap limit that must hold for an arm (static: C; AIMD: configured max).
    pub fn cap_limit(&self, arm: Arm) -> Option<usize> {
        match arm {
            Arm::Base => None,
            Arm::StaticCap => Some(self.static_cap),
            Arm::AimdCap => Some(self.aimd_max),
        }
    }

    fn gateway_spec(&self, arm: Arm, origin: std::net::SocketAddr, log_path: PathBuf) -> GatewaySpec {
        let (intelligence, cap) = match arm {
            Arm::Base => ("off".to_string(), None),
            Arm::StaticCap => (
                "canary".to_string(),
                Some(CapConfig { min: self.static_cap, max: self.static_cap, p99_ceiling_ms: 1000, window: 100 }),
            ),
            Arm::AimdCap => (
                "canary".to_string(),
                Some(CapConfig {
                    min: self.aimd_min,
                    max: self.aimd_max,
                    p99_ceiling_ms: self.aimd_p99_ceiling_ms,
                    window: self.aimd_window,
                }),
            ),
        };
        GatewaySpec {
            exe: self.gateway_exe.clone(),
            origin,
            max_cache_bytes: self.max_cache_bytes,
            intelligence,
            cap,
            log_path,
        }
    }
}

/// Smoke configuration: 1 rep x 3 arms, short phases (EXPLORATORY by construction: reps < 10).
pub fn smoke_config(gateway_exe: PathBuf, out_dir: PathBuf, phase_ms: u64) -> ScenarioBConfig {
    let phases = Phases { warmup_ms: phase_ms, healthy_ms: phase_ms, degraded_ms: phase_ms, recovery_ms: phase_ms };
    let mut c = ScenarioBConfig::with_phases(gateway_exe, out_dir, phases);
    c.reps = 1;
    // Mechanics smoke: a tight cap so a short debug run still sheds and the mechanism is observable.
    c.static_cap = 8;
    c.aimd_min = 4;
    c.sync_derived();
    c
}

#[derive(Debug)]
pub enum ScenarioError {
    Cancelled,
    /// The cap did not enforce as configured (or BASE unexpectedly enforced): the run is refused.
    CapNotEnforcing(String),
    Gateway(String),
    Io(String),
    Config(String),
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioError::Cancelled => write!(f, "cancelled"),
            ScenarioError::CapNotEnforcing(s) => write!(f, "cap not enforcing as configured: {s}"),
            ScenarioError::Gateway(s) => write!(f, "gateway: {s}"),
            ScenarioError::Io(s) => write!(f, "io: {s}"),
            ScenarioError::Config(s) => write!(f, "config: {s}"),
        }
    }
}

impl std::error::Error for ScenarioError {}

#[derive(Debug, Clone)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub arm: String,
    pub rep: usize,
    pub phase: String,
    /// Gateway process id of the current run (0 before it is spawned).
    pub gateway_pid: u32,
}

pub type ProgressFn = Arc<dyn Fn(Progress) + Send + Sync>;

// ---------------------------------------------------------------------------------------------
// Per-run metrics (pure functions over raw records)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunMetrics {
    pub arm: String,
    pub rep: usize,
    pub valid: bool,
    pub invalid_reasons: Vec<String>,
    pub contention_flag: bool,
    pub gateway_pid: u32,
    // client side, analysis window (scheduled time >= warm-up)
    pub n_scheduled_window: usize,
    pub n_with_response_window: usize,
    pub n_no_response_window: usize,
    pub success_rate: Option<f64>,
    pub shed_fraction: Option<f64>,
    pub other_5xx_fraction: Option<f64>,
    pub status_counts: BTreeMap<String, usize>,
    pub cache_status_counts: BTreeMap<String, usize>,
    pub client_p50_ms: Option<f64>,
    pub client_p90_ms: Option<f64>,
    pub client_p95_ms: Option<f64>,
    pub client_p99_ms: Option<f64>,
    pub client_max_ms: Option<f64>,
    pub client_success_p99_ms: Option<f64>,
    pub send_lag_p99_ms: Option<f64>,
    // origin side
    pub origin_requests_window: usize,
    pub origin_requests_whole_run: usize,
    pub origin_peak_window: Option<u32>,
    pub origin_peak_degraded: Option<u32>,
    pub origin_peak_whole_run: usize,
    pub origin_p99_service_ms: Option<f64>,
    pub origin_p50_service_healthy_ms: Option<f64>,
    pub origin_503_window: usize,
    // mechanism
    pub cap_limit: Option<usize>,
    pub mechanism_violated: bool,
    // gateway snapshots
    pub snapshots: usize,
    pub snapshot_errors: usize,
    pub enforcing_all_snapshots: Option<bool>,
    pub gateway_cap_min: Option<u64>,
    pub gateway_cap_max: Option<u64>,
    pub gateway_in_flight_max: Option<u64>,
    pub gateway_rejections_final: Option<u64>,
    pub gateway_dispatched_final: Option<u64>,
    pub gateway_controller_steps_final: Option<u64>,
    pub stale_served_total_final: Option<u64>,
    pub stale_cap_violations_final: Option<u64>,
    // recovery (H-B4)
    pub error_rate_healthy: Option<f64>,
    pub recovery_error_s: Option<f64>,
    pub recovery_in_flight_s: Option<f64>,
    pub cap_at_degr_end: Option<u64>,
    pub cap_final: Option<u64>,
}

pub struct RunInputs<'a> {
    pub arm: Arm,
    pub rep: usize,
    pub phases: Phases,
    pub client: &'a [ClientRecord],
    pub scheduled_total: usize,
    pub origin: &'a [OriginRecord],
    pub origin_sched_start_ns: u64,
    pub origin_peak_whole_run: usize,
    pub snaps: &'a [Value],
    pub snapshot_errors: usize,
    pub cap_limit: Option<usize>,
    pub gateway_alive: bool,
    pub gateway_pid: u32,
    pub enforce_ok: bool,
}

fn snap_u64(s: &Value, path: &[&str]) -> Option<u64> {
    let mut v = s;
    for p in path {
        v = v.get(*p)?;
    }
    v.as_u64()
}

fn frac(n: usize, d: usize) -> Option<f64> {
    if d == 0 {
        None
    } else {
        Some(n as f64 / d as f64)
    }
}

/// Last bin index violating `rate > threshold` -> recovery seconds after `degr_end` (None if the final bin still violates).
pub fn recovery_from_bins(bin_rates: &[Option<f64>], threshold: f64) -> Option<f64> {
    if bin_rates.is_empty() {
        return None;
    }
    let last_bad = bin_rates.iter().rposition(|r| matches!(r, Some(x) if *x > threshold));
    match last_bad {
        None => Some(0.0),
        Some(i) if i + 1 >= bin_rates.len() => None,
        Some(i) => Some((i + 1) as f64),
    }
}

pub fn compute_run_metrics(inp: &RunInputs) -> (RunMetrics, Vec<f64>) {
    let ph = inp.phases;
    let warm_ns = ph.warmup_ms * 1_000_000;
    let mut m = RunMetrics {
        arm: inp.arm.name().to_string(),
        rep: inp.rep,
        gateway_pid: inp.gateway_pid,
        cap_limit: inp.cap_limit,
        origin_peak_whole_run: inp.origin_peak_whole_run,
        origin_requests_whole_run: inp.origin.len(),
        snapshots: inp.snaps.len(),
        snapshot_errors: inp.snapshot_errors,
        ..Default::default()
    };

    // ---- client window
    let win: Vec<&ClientRecord> = inp.client.iter().filter(|r| r.scheduled_ns >= warm_ns).collect();
    m.n_scheduled_window = win.len();
    let with_resp: Vec<&&ClientRecord> = win.iter().filter(|r| r.status != 0).collect();
    m.n_with_response_window = with_resp.len();
    m.n_no_response_window = win.len() - with_resp.len();
    let ok = win.iter().filter(|r| r.is_success()).count();
    let shed = win.iter().filter(|r| r.status == 503 && r.cache_error == "CapacityExceeded").count();
    let e5 = win.iter().filter(|r| r.status >= 500 && !(r.status == 503 && r.cache_error == "CapacityExceeded")).count();
    m.success_rate = frac(ok, win.len());
    m.shed_fraction = frac(shed, win.len());
    m.other_5xx_fraction = frac(e5, win.len());
    for r in &win {
        let k = if r.status == 0 { format!("none:{}", r.error.split(':').next().unwrap_or("")) } else { r.status.to_string() };
        *m.status_counts.entry(k).or_default() += 1;
        let cs = if r.cache_status.is_empty() { "NONE".to_string() } else { r.cache_status.clone() };
        *m.cache_status_counts.entry(cs).or_default() += 1;
    }
    // Latency from the scheduled time over ALL window requests (a request with no response counts at its completion
    // time, i.e. the deadline), so failures cannot hide in the tail.
    let lat: Vec<f64> = win.iter().map(|r| r.latency_ms()).collect();
    m.client_p50_ms = percentile(&lat, 50.0);
    m.client_p90_ms = percentile(&lat, 90.0);
    m.client_p95_ms = percentile(&lat, 95.0);
    m.client_p99_ms = percentile(&lat, 99.0);
    m.client_max_ms = percentile(&lat, 100.0);
    let lat_ok: Vec<f64> = win.iter().filter(|r| r.is_success()).map(|r| r.latency_ms()).collect();
    m.client_success_p99_ms = percentile(&lat_ok, 99.0);
    let lags: Vec<f64> = win.iter().map(|r| r.send_lag_ms()).collect();
    m.send_lag_p99_ms = percentile(&lags, 99.0);
    m.contention_flag = m.send_lag_p99_ms.map(|x| x > CONTENTION_LAG_P99_MS).unwrap_or(false);

    // ---- origin window (offsets relative to the schedule start)
    let rel = |r: &OriginRecord| r.arrival_ns as i64 - inp.origin_sched_start_ns as i64;
    let in_win: Vec<&OriginRecord> = inp.origin.iter().filter(|r| rel(r) >= warm_ns as i64).collect();
    m.origin_requests_window = in_win.len();
    m.origin_peak_window = in_win.iter().map(|r| r.conc_at_arrival).max();
    m.origin_peak_degraded = in_win
        .iter()
        .filter(|r| {
            let ms = rel(r) / 1_000_000;
            ms >= ph.degr_start_ms() as i64 && ms < ph.degr_end_ms() as i64
        })
        .map(|r| r.conc_at_arrival)
        .max();
    let svc: Vec<f64> = in_win.iter().map(|r| r.service_ns as f64 / 1e6).collect();
    m.origin_p99_service_ms = percentile(&svc, 99.0);
    let healthy_svc: Vec<f64> = inp
        .origin
        .iter()
        .filter(|r| !r.degraded)
        .map(|r| r.service_ns as f64 / 1e6)
        .collect();
    m.origin_p50_service_healthy_ms = percentile(&healthy_svc, 50.0);
    m.origin_503_window = in_win.iter().filter(|r| r.status == 503).count();
    m.mechanism_violated = match inp.cap_limit {
        Some(c) => inp.origin_peak_whole_run > c,
        None => false,
    };

    // ---- gateway snapshots
    let caps: Vec<u64> = inp.snaps.iter().filter_map(|s| snap_u64(s, &["origin_cap", "cap"])).collect();
    m.gateway_cap_min = caps.iter().copied().min();
    m.gateway_cap_max = caps.iter().copied().max();
    m.gateway_in_flight_max = inp.snaps.iter().filter_map(|s| snap_u64(s, &["origin_cap", "in_flight"])).max();
    let last = inp.snaps.last();
    m.gateway_rejections_final = last.and_then(|s| snap_u64(s, &["origin_cap", "rejections"]));
    m.gateway_dispatched_final = last.and_then(|s| snap_u64(s, &["origin_cap", "dispatched"]));
    m.gateway_controller_steps_final = last.and_then(|s| snap_u64(s, &["origin_cap", "controller_steps"]));
    m.stale_served_total_final = last.and_then(|s| snap_u64(s, &["stale_served_total"]));
    m.stale_cap_violations_final = last.and_then(|s| snap_u64(s, &["stale_cap_violations"]));
    m.cap_final = last.and_then(|s| snap_u64(s, &["origin_cap", "cap"]));
    let enf: Vec<bool> = inp.snaps.iter().filter_map(|s| s["origin_cap"]["enforcing"].as_bool()).collect();
    m.enforcing_all_snapshots = if enf.is_empty() { None } else { Some(enf.iter().all(|b| *b)) };
    let t_ms = |s: &Value| s["t_ns"].as_u64().unwrap_or(0) / 1_000_000;
    m.cap_at_degr_end = inp
        .snaps
        .iter()
        .rfind(|s| t_ms(s) <= ph.degr_end_ms())
        .and_then(|s| snap_u64(s, &["origin_cap", "cap"]));

    // ---- recovery (H-B4)
    let healthy_win: Vec<&&ClientRecord> = win
        .iter()
        .filter(|r| r.scheduled_ns / 1_000_000 < ph.degr_start_ms())
        .collect();
    let hn = healthy_win.len();
    let hbad = healthy_win.iter().filter(|r| !r.is_success()).count();
    m.error_rate_healthy = frac(hbad, hn);
    if let Some(base_err) = m.error_rate_healthy {
        let thr = base_err + 0.02;
        let mut bins: Vec<(usize, usize)> = Vec::new();
        let start = ph.degr_end_ms();
        let nbins = ((ph.total_ms().saturating_sub(start)) / 1000) as usize;
        bins.resize(nbins, (0, 0));
        for r in inp.client.iter() {
            let ms = r.scheduled_ns / 1_000_000;
            if ms >= start {
                let b = ((ms - start) / 1000) as usize;
                if b < nbins {
                    bins[b].0 += 1;
                    if !r.is_success() {
                        bins[b].1 += 1;
                    }
                }
            }
        }
        let rates: Vec<Option<f64>> = bins.iter().map(|(n, b)| frac(*b, *n)).collect();
        m.recovery_error_s = recovery_from_bins(&rates, thr);
    }
    let healthy_if: Vec<u64> = inp
        .snaps
        .iter()
        .filter(|s| t_ms(s) >= ph.warmup_ms && t_ms(s) < ph.degr_start_ms())
        .filter_map(|s| snap_u64(s, &["origin_cap", "in_flight"]))
        .collect();
    if let Some(hmax) = healthy_if.iter().copied().max() {
        let after: Vec<(u64, u64)> = inp
            .snaps
            .iter()
            .filter(|s| t_ms(s) >= ph.degr_end_ms())
            .filter_map(|s| snap_u64(s, &["origin_cap", "in_flight"]).map(|v| (t_ms(s), v)))
            .collect();
        if !after.is_empty() {
            let last_bad = after.iter().rposition(|(_, v)| *v > hmax + 1);
            m.recovery_in_flight_s = match last_bad {
                None => Some(0.0),
                Some(i) if i + 1 >= after.len() => None,
                Some(i) => Some((after[i + 1].0 - ph.degr_end_ms()) as f64 / 1000.0),
            };
        }
    }

    // ---- validity (infrastructure soundness only; outcomes never decide validity)
    if !inp.gateway_alive {
        m.invalid_reasons.push("gateway process died during the run".into());
    }
    if !inp.enforce_ok {
        m.invalid_reasons.push("cap enforcement not verified before measuring".into());
    }
    if inp.client.len() != inp.scheduled_total {
        m.invalid_reasons.push(format!("only {} of {} scheduled requests were sent", inp.client.len(), inp.scheduled_total));
    }
    if inp.snaps.len() < 4 {
        m.invalid_reasons.push(format!("only {} gateway snapshots", inp.snaps.len()));
    }
    if inp.cap_limit.is_some() && m.enforcing_all_snapshots != Some(true) {
        m.invalid_reasons.push("a snapshot reported enforcing != true in a capped arm".into());
    }
    m.valid = m.invalid_reasons.is_empty();
    (m, lat)
}

// ---------------------------------------------------------------------------------------------
// Summary (pure)
// ---------------------------------------------------------------------------------------------

fn opt(v: Option<f64>) -> Value {
    match v {
        Some(x) if x.is_finite() => json!(x),
        _ => Value::Null,
    }
}

fn mean(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        None
    } else {
        Some(v.iter().sum::<f64>() / v.len() as f64)
    }
}

type Extract = fn(&RunMetrics) -> Option<f64>;

fn metric_table() -> Vec<(&'static str, Extract)> {
    vec![
        ("origin_peak_concurrency_window", |m| m.origin_peak_window.map(|x| x as f64)),
        ("origin_peak_concurrency_degraded", |m| m.origin_peak_degraded.map(|x| x as f64)),
        ("origin_p99_service_ms", |m| m.origin_p99_service_ms),
        ("origin_requests_window", |m| Some(m.origin_requests_window as f64)),
        ("client_success_rate_pp", |m| m.success_rate.map(|x| x * 100.0)),
        ("shed_fraction_pp", |m| m.shed_fraction.map(|x| x * 100.0)),
        ("client_p50_ms", |m| m.client_p50_ms),
        ("client_p95_ms", |m| m.client_p95_ms),
        ("client_p99_ms", |m| m.client_p99_ms),
    ]
}

fn paired_json(p: &Paired) -> Value {
    json!({
        "n_pairs": p.n_pairs,
        "mean_base": p.mean_base, "mean_treat": p.mean_treat, "mean_diff_treat_minus_base": p.mean_diff,
        "diff_ci95_bca": [opt(p.diff_ci95_lo), opt(p.diff_ci95_hi)],
        "relative_reduction_mean": opt(p.rel_reduction_mean),
        "relative_reduction_ci95_bca": [opt(p.rel_ci95_lo), opt(p.rel_ci95_hi)],
        "permutation_p_sign_flip": opt(p.perm_p),
    })
}

pub const CAN_NOT_ESTABLISH: &str = "Not established by Scenario B: adaptive-control merit; production, WAN or multi-host behaviour; origin-p99 benefits; stale-serving effectiveness; pre-registered cap selection; a served-client latency improvement.";

// ---- G1 disposition D1: wording that must accompany every NEW run (retained artifacts are never edited;
// the dated interpretation notice docs/demo/scenario_b_interpretation_notice.md stays binding for them).
/// What the label `VALID` means (and does not mean).
pub const LABEL_VALID_MEANING: &str = "Label VALID means only: release-profile gateway and driver binaries (gateway profile inferred from the directory directly above the executable), >= 10 repetitions per arm, BASE and at least one capped arm present, workload parameters within tolerance of the documented experiment (phases >= half of 8/5/10/8 s, arrival rate and key catalogue within 0.5x..2x of 300 rps and 5000 keys), and the registered caps and origin parameters unchanged (static cap 24; AIMD 12-32, ceiling 60 ms, window 30), one host, scripted origin. It is not a statement of statistical validity beyond the mechanical rule, of generality, or of adaptive-control merit.";
/// The nominal AIMD arm can behave as a static cap near its minimum.
pub const AIMD_EFFECTIVE_CAP_NOTE: &str = "AIMD-CAP effective cap: the nominal range [aimd_min, aimd_max] is not necessarily the cap in force, and with other aimd_min/aimd_max or workloads it may behave as a static cap near aimd_min. In the retained experiment (12-32, ceiling 60 ms, window 30) the controller fell from 32 to about 12 within ~2 s, stayed there in every phase and shed about 2% of requests even in the healthy phase, so AIMD-CAP vs BASE was effectively a static cap near 12 versus no cap and does not show adaptive-controller effectiveness or ineffectiveness; check gateway_cap_min_seen / cap_final per run. The controller's 30-sample 'p99' is the window maximum.";
/// Canary vs off is not a clean A/B of the cap alone.
pub const CANARY_VS_OFF_NOTE: &str = "Arms differ by more than the cap: capped arms run PHANTOM_INTELLIGENCE=canary and BASE runs off (the cap is enforced only in canary/active). Fairness of canary versus off is excluded by code reading only; no cap-free canary arm and no overhead measurement exists.";
/// All-request latency includes fast-failing sheds.
pub const ALL_REQUEST_LATENCY_NOTE: &str = "Client latency percentiles (client_p50/p95/p99) are computed over ALL window requests, including fast-failing 503 sheds; they are NOT evidence of improved latency for served clients. Success-only latency is recorded per run (client_success_p99_ms in run_metrics.json) and must be reported separately.";
/// Origin p99 is untestable with the scripted origin.
pub const ORIGIN_P99_NOTE: &str = "The scripted origin has no load-dependent latency (a pure delay server), so the contract's origin-p99 clause is untestable here; observed origin_p99 differences are timer noise.";
/// Stale-serving was not exercised.
pub const STALE_SERVING_NOTE: &str = "Stale-serving code exists but the workload did not exercise it (the origin sends only max-age=3; stale-while-revalidate and stale-if-error are not run; stale_served_total = 0), so nothing about stale-serving is established.";
/// Pilot / tuning disclosure.
pub const PILOT_DISCLOSURE: &str = "Cap values and thresholds are not pre-registered. The retained experiment chose its caps (static 24; AIMD 12-32) after a single-repetition pilot with the same seed as repetition 1; any run whose parameters were chosen after looking at pilot data is likewise unregistered.";

/// Notes attached to every summary and manifest produced by a new run.
pub fn interpretation_notes() -> Vec<&'static str> {
    vec![AIMD_EFFECTIVE_CAP_NOTE, CANARY_VS_OFF_NOTE, ALL_REQUEST_LATENCY_NOTE, ORIGIN_P99_NOTE, STALE_SERVING_NOTE, PILOT_DISCLOSURE, CAN_NOT_ESTABLISH]
}

pub struct SummaryMeta {
    pub label: String,
    pub label_reasons: Vec<String>,
    pub cancelled: bool,
}

/// Why a summary is incomplete, or `None` when every planned repetition of every arm finished.
/// `"cancelled"`: the cancel flag was raised. `"runs_missing"`: not cancelled, yet some arm has fewer runs than
/// `cfg.reps` (defensive: a mechanism/verdict computed from absent runs would be meaningless).
pub fn incomplete_reason(cfg: &ScenarioBConfig, runs: &[RunMetrics], cancelled: bool) -> Option<&'static str> {
    if cancelled {
        return Some("cancelled");
    }
    let short = cfg.arms.iter().any(|a| runs.iter().filter(|r| r.arm == a.name()).count() < cfg.reps);
    if short { Some("runs_missing") } else { None }
}

pub const VERDICTS_NOT_COMPUTED_REASON: &str = "the run did not finish all planned repetitions of every arm; no mechanism verdict or arm verdict is computed from a partial design";

/// Label for a summary: a planned `VALID` (config-only) never survives on an incomplete run (G4C F02).
pub fn effective_label(planned: &str, reasons: &[String], incomplete: Option<&str>) -> (String, Vec<String>) {
    match incomplete {
        None => (planned.to_string(), reasons.to_vec()),
        Some(why) => {
            let mut r = reasons.to_vec();
            r.push(format!("run incomplete ({why}): the planned label {planned} does not apply to a partial run"));
            ("EXPLORATORY".to_string(), r)
        }
    }
}

pub fn summarize(cfg: &ScenarioBConfig, runs: &[RunMetrics], pooled: &BTreeMap<String, Vec<f64>>, meta: &SummaryMeta) -> Value {
    let incomplete = incomplete_reason(cfg, runs, meta.cancelled);
    let mut arms_json = serde_json::Map::new();
    for arm in &cfg.arms {
        let name = arm.name();
        let rs: Vec<&RunMetrics> = runs.iter().filter(|r| r.arm == name).collect();
        let valid: Vec<&&RunMetrics> = rs.iter().filter(|r| r.valid).collect();
        let pool = pooled.get(name).cloned().unwrap_or_default();
        let per_run_mean = |f: Extract| -> Value { opt(mean(&valid.iter().filter_map(|r| f(r)).collect::<Vec<_>>())) };
        let status_total: BTreeMap<String, usize> = valid.iter().fold(BTreeMap::new(), |mut a, r| {
            for (k, v) in &r.status_counts {
                *a.entry(k.clone()).or_default() += v;
            }
            a
        });
        let cache_total: BTreeMap<String, usize> = valid.iter().fold(BTreeMap::new(), |mut a, r| {
            for (k, v) in &r.cache_status_counts {
                *a.entry(k.clone()).or_default() += v;
            }
            a
        });
        let n_window: usize = valid.iter().map(|r| r.n_scheduled_window).sum();
        let recovered_err = valid.iter().filter(|r| r.recovery_error_s.is_some()).count();
        let rec_vals: Vec<f64> = valid.iter().filter_map(|r| r.recovery_error_s).collect();
        let rec_if: Vec<f64> = valid.iter().filter_map(|r| r.recovery_in_flight_s).collect();
        arms_json.insert(
            name.to_string(),
            json!({
                "runs": rs.len(), "valid_runs": valid.len(),
                "invalid_runs": rs.iter().filter(|r| !r.valid).map(|r| json!({"rep": r.rep, "reasons": r.invalid_reasons})).collect::<Vec<_>>(),
                "client_samples_window_total": n_window,
                "client_latency_ms_pooled_scheduled_time": {
                    "n": pool.len(),
                    "p50": opt(percentile(&pool, 50.0)), "p90": opt(percentile(&pool, 90.0)),
                    "p95": opt(percentile(&pool, 95.0)), "p99": opt(percentile(&pool, 99.0)),
                    "max": opt(percentile(&pool, 100.0)),
                },
                "per_run_mean": {
                    "client_success_rate": per_run_mean(|m| m.success_rate),
                    "shed_fraction": per_run_mean(|m| m.shed_fraction),
                    "other_5xx_fraction": per_run_mean(|m| m.other_5xx_fraction),
                    "origin_peak_concurrency_window": per_run_mean(|m| m.origin_peak_window.map(|x| x as f64)),
                    "origin_peak_concurrency_degraded": per_run_mean(|m| m.origin_peak_degraded.map(|x| x as f64)),
                    "origin_peak_concurrency_whole_run": per_run_mean(|m| Some(m.origin_peak_whole_run as f64)),
                    "origin_p99_service_ms": per_run_mean(|m| m.origin_p99_service_ms),
                    "origin_requests_window": per_run_mean(|m| Some(m.origin_requests_window as f64)),
                    "client_p50_ms": per_run_mean(|m| m.client_p50_ms),
                    "client_p99_ms": per_run_mean(|m| m.client_p99_ms),
                    "gateway_rejections_final": per_run_mean(|m| m.gateway_rejections_final.map(|x| x as f64)),
                    "gateway_in_flight_max": per_run_mean(|m| m.gateway_in_flight_max.map(|x| x as f64)),
                    "gateway_cap_min_seen": per_run_mean(|m| m.gateway_cap_min.map(|x| x as f64)),
                    "stale_served_total_final": per_run_mean(|m| m.stale_served_total_final.map(|x| x as f64)),
                },
                "status_counts_total": status_total,
                "cache_status_counts_total": cache_total,
                "recovery_h_b4": {
                    "runs_with_error_recovery": recovered_err, "valid_runs": valid.len(),
                    "error_recovery_s_median": opt(percentile(&rec_vals, 50.0)),
                    "error_recovery_s_max": opt(percentile(&rec_vals, 100.0)),
                    "in_flight_recovery_s_median": opt(percentile(&rec_if, 50.0)),
                    "note": "seconds after the degraded interval ends until every later 1 s bin (errors) / snapshot (in_flight) is back at the healthy-phase level; null = not recovered within the run",
                },
            }),
        );
    }

    // mechanism + paired + verdict
    let mut mech = serde_json::Map::new();
    let mut paired_out = serde_json::Map::new();
    let mut verdicts = serde_json::Map::new();
    let base_name = Arm::Base.name();
    let base_runs: BTreeMap<usize, &RunMetrics> = runs.iter().filter(|r| r.arm == base_name).map(|r| (r.rep, r)).collect();
    for arm in cfg.arms.iter().filter(|a| **a != Arm::Base) {
        let name = arm.name();
        let rs: Vec<&RunMetrics> = runs.iter().filter(|r| r.arm == name).collect();
        let violating: Vec<usize> = rs.iter().filter(|r| r.mechanism_violated).map(|r| r.rep).collect();
        let max_obs = rs.iter().map(|r| r.origin_peak_whole_run).max();
        let mech_pass = !rs.is_empty() && violating.is_empty();
        // U-01: on an incomplete summary the observations stay, the pass/fail judgement is withheld (null).
        let mech_pass_json = if incomplete.is_some() { Value::Null } else { json!(mech_pass) };
        mech.insert(
            name.to_string(),
            json!({
                "cap_limit_in_force": cfg.cap_limit(*arm), "max_origin_concurrent_observed_whole_run": max_obs,
                "runs": rs.len(), "violating_reps": violating, "pass": mech_pass_json,
                "note": "checked over the whole run including warm-up; AIMD limit = configured max (cap can only be lower)",
            }),
        );
        // pairs: same rep valid in both arms
        let pairs: Vec<(&RunMetrics, &RunMetrics)> = rs
            .iter()
            .filter(|t| t.valid)
            .filter_map(|t| base_runs.get(&t.rep).filter(|b| b.valid).map(|b| (*b, *t)))
            .collect();
        let mut pj = serde_json::Map::new();
        let mut table: BTreeMap<&str, Paired> = BTreeMap::new();
        for (mname, f) in metric_table() {
            let b: Vec<f64> = pairs.iter().map(|p| f(p.0).unwrap_or(f64::NAN)).collect();
            let t: Vec<f64> = pairs.iter().map(|p| f(p.1).unwrap_or(f64::NAN)).collect();
            if let Some(p) = analysis::paired(&b, &t, cfg.seed ^ (mname.len() as u64 * 7919)) {
                pj.insert(mname.to_string(), paired_json(&p));
                table.insert(mname, p);
            } else {
                pj.insert(mname.to_string(), Value::Null);
            }
        }
        paired_out.insert(name.to_string(), Value::Object(pj));
        let contention = pairs.iter().any(|p| p.0.contention_flag || p.1.contention_flag)
            || rs.iter().any(|r| r.contention_flag);
        let peak = table.get("origin_peak_concurrency_window");
        let ev = ArmEvidence {
            n_valid_pairs: pairs.len(),
            contention_flagged: contention,
            mechanism_pass: mech_pass,
            rel_reduction_mean: peak.and_then(|p| p.rel_reduction_mean),
            rel_ci_lo: peak.and_then(|p| p.rel_ci95_lo),
            rel_ci_hi: peak.and_then(|p| p.rel_ci95_hi),
            origin_p99_diff_mean: table.get("origin_p99_service_ms").map(|p| p.mean_diff),
            success_diff_pp_mean: table.get("client_success_rate_pp").map(|p| p.mean_diff),
        };
        if incomplete.is_none() {
            let v = analysis::evaluate_arm(&ev);
            verdicts.insert(
                name.to_string(),
                json!({ "outcome": v.outcome.as_str(), "reasons": v.reasons, "evidence": ev }),
            );
        }
    }

    let (verdicts_json, h_b1_json, verdicts_status, verdicts_reason) = if incomplete.is_none() {
        (Value::Object(verdicts), json!(mech.values().all(|m| m["pass"] == json!(true))), "computed", Value::Null)
    } else {
        (Value::Null, Value::Null, "not_computed", json!(VERDICTS_NOT_COMPUTED_REASON))
    };
    let (label, label_reasons) = effective_label(&meta.label, &meta.label_reasons, incomplete);
    json!({
        "summary_version": 2,
        "tier": TIER,
        "label": label,
        "label_planned": meta.label,
        "label_reasons": label_reasons,
        "run_completeness": {
            "complete": incomplete.is_none(),
            "incomplete_reason": incomplete,
            "planned_runs": cfg.reps * cfg.arms.len(),
            "completed_runs": runs.len(),
        },
        "verdicts_status": verdicts_status,
        "verdicts_reason": verdicts_reason,
        "label_meaning": LABEL_VALID_MEANING,
        "interpretation_notes": interpretation_notes(),
        "status": if meta.cancelled { "CANCELLED" } else { "COMPLETE" },
        "min_valid_runs_required": MIN_VALID_RUNS,
        "provenance": {
            "tier": TIER,
            "what_ran": "real phantom-cache-demo gateway process per arm per repetition on 127.0.0.1; scripted raw-TCP origin; open-loop load generator; all in real time on one host (shared CPU)",
            "measured": ["client latency/status/X-Cache-Status per request", "origin concurrency/service time per request", "gateway origin_cap snapshots from /api/v1/metrics (same schema-2 payload as /api/events)"],
            "derived": ["percentiles (nearest rank, raw samples)", "paired differences, BCa 95% CI (10000 resamples), sign-flip permutation p", "recovery times", "verdicts (mechanical rule)"],
            "simulated": [],
            "not_covered": "no WAN, no multi-host, no real service; adaptive stale-window/admission mechanisms are not part of the live gateway",
        },
        "config": cfg,
        "arms": Value::Object(arms_json),
        "mechanism_h_b1": Value::Object(mech),
        "mechanism_h_b1_all_pass": h_b1_json,
        "paired_vs_base": Value::Object(paired_out),
        "verdicts": verdicts_json,
        "limitations": limitations(),
    })
}

pub fn limitations() -> Vec<&'static str> {
    vec![
        "Loopback on one host: no network RTT; load generator, gateway and origin share CPU and the scheduler.",
        "Scripted origin is a pure delay server (no internal queue or capacity): service time does not grow with load, so the positive-feedback loop of the contract is NOT reproduced; only the concurrency/latency effect of the scripted inflation is.",
        "Service times and arrival timing use tokio timers; real resolution is host dependent (see manifest timer_probe). Observed service times exceed the nominal value by the timer overshoot.",
        "Limited key catalogue and a single Zipf shape / arrival rate / degradation shape; results apply to this workload family only.",
        "Per-run latencies are measured from the SCHEDULED send time (open loop); runs are real time and not bit-deterministic.",
        CANARY_VS_OFF_NOTE,
        AIMD_EFFECTIVE_CAP_NOTE,
        ALL_REQUEST_LATENCY_NOTE,
        ORIGIN_P99_NOTE,
        STALE_SERVING_NOTE,
        PILOT_DISCLOSURE,
        "Peak origin concurrency counts requests in service on the origin; connections the gateway abandoned (client disconnect) keep their origin slot until the scripted service time ends.",
        "No claim about production, WAN, multi-host behaviour; debug-build runs are mechanics only.",
    ]
}

// ---------------------------------------------------------------------------------------------
// Environment / provenance helpers
// ---------------------------------------------------------------------------------------------

pub fn host_info() -> Value {
    let ver = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    json!({
        "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "os_version": if ver.is_empty() { Value::Null } else { json!(ver) },
        "cpu": std::env::var("PROCESSOR_IDENTIFIER").ok(),
        "logical_cpus_env": std::env::var("NUMBER_OF_PROCESSORS").ok(),
        "available_parallelism": std::thread::available_parallelism().map(|n| n.get()).ok(),
        "ram": "not collected",
    })
}

/// Median / p99 overshoot of a 1 ms tokio sleep (documents the timer resolution of this host).
pub async fn timer_probe() -> Value {
    let mut over = Vec::new();
    for _ in 0..100 {
        let t = Instant::now();
        tokio::time::sleep(Duration::from_millis(1)).await;
        over.push(t.elapsed().as_secs_f64() * 1000.0 - 1.0);
    }
    json!({"sleep_1ms_overshoot_ms_p50": opt(percentile(&over, 50.0)), "sleep_1ms_overshoot_ms_p99": opt(percentile(&over, 99.0)), "samples": over.len()})
}

/// Build profile of the gateway binary, inferred from the directory immediately above it (`target/<profile>/`,
/// also `target/<triple>/<profile>/`). Any other location (including `deps/`, `examples/`, an installed copy)
/// is "unknown", which is never treated as release. A `release` component further up the path does not count.
pub fn gateway_profile(exe: &Path) -> String {
    let parent = exe
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase());
    match parent.as_deref() {
        Some("release") => "release".into(),
        Some("debug") => "debug".into(),
        _ => "unknown".into(),
    }
}

fn lab_live_source_hashes() -> BTreeMap<String, String> {
    let root = repo_root();
    let mut m = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(root.join("src/lab_live")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(h) = sha256_file(&p) {
                    m.insert(format!("src/lab_live/{}", p.file_name().unwrap().to_string_lossy()), h);
                }
            }
        }
    }
    for f in ["src/bin/phantom-lab.rs"] {
        if let Ok(h) = sha256_file(&root.join(f)) {
            m.insert(f.to_string(), h);
        }
    }
    m
}

/// Prefix of the reason added when the workload deviates from the documented experiment.
pub const DEVIATES_REASON: &str = "deviates_from_retained_experiment_parameters";

/// Workload parameters outside the tolerance around the documented defaults (phases >= half of the
/// default length, arrival rate and key catalogue within 0.5x..2x). Empty = within tolerance.
pub fn parameter_deviations(cfg: &ScenarioBConfig) -> Vec<String> {
    let d = ScenarioBConfig::default_experiment(PathBuf::new(), PathBuf::new());
    let mut v = Vec::new();
    let mut phase = |name: &str, got: u64, def: u64| {
        if got < def / 2 {
            v.push(format!("{name} {got} ms < {} ms (half of the default {def} ms)", def / 2));
        }
    };
    phase("warmup", cfg.phases.warmup_ms, d.phases.warmup_ms);
    phase("healthy", cfg.phases.healthy_ms, d.phases.healthy_ms);
    phase("degraded", cfg.phases.degraded_ms, d.phases.degraded_ms);
    phase("recovery", cfg.phases.recovery_ms, d.phases.recovery_ms);
    let (r, dr) = (cfg.workload.rate_rps, d.workload.rate_rps);
    if !(r >= dr * 0.5 && r <= dr * 2.0) {
        v.push(format!("arrival rate {r} rps outside 0.5x..2x of the default {dr} rps"));
    }
    let (k, dk) = (cfg.workload.n_keys as f64, d.workload.n_keys as f64);
    if !(k >= dk * 0.5 && k <= dk * 2.0) {
        v.push(format!("key catalogue {} outside 0.5x..2x of the default {}", cfg.workload.n_keys, d.workload.n_keys));
    }
    // registered cap and origin parameters: ANY change is a deviation (the caps were chosen once, see PILOT_DISCLOSURE)
    let mut same = |name: &str, ok: bool, got: String, def: String| {
        if !ok {
            v.push(format!("{name} {got} differs from the registered default {def}"));
        }
    };
    same("static_cap", cfg.static_cap == d.static_cap, cfg.static_cap.to_string(), d.static_cap.to_string());
    same("aimd_min", cfg.aimd_min == d.aimd_min, cfg.aimd_min.to_string(), d.aimd_min.to_string());
    same("aimd_max", cfg.aimd_max == d.aimd_max, cfg.aimd_max.to_string(), d.aimd_max.to_string());
    same("aimd p99 ceiling ms", cfg.aimd_p99_ceiling_ms == d.aimd_p99_ceiling_ms, cfg.aimd_p99_ceiling_ms.to_string(), d.aimd_p99_ceiling_ms.to_string());
    same("aimd window", cfg.aimd_window == d.aimd_window, cfg.aimd_window.to_string(), d.aimd_window.to_string());
    same("max cache bytes", cfg.max_cache_bytes == d.max_cache_bytes, cfg.max_cache_bytes.to_string(), d.max_cache_bytes.to_string());
    same("origin base service ms", cfg.origin.base_service_ms == d.origin.base_service_ms, cfg.origin.base_service_ms.to_string(), d.origin.base_service_ms.to_string());
    same("origin degraded multiplier", cfg.origin.degraded_mult == d.origin.degraded_mult, cfg.origin.degraded_mult.to_string(), d.origin.degraded_mult.to_string());
    same("origin error rate", cfg.origin.error_rate == d.origin.error_rate, cfg.origin.error_rate.to_string(), d.origin.error_rate.to_string());
    same("origin max-age", cfg.origin.max_age_s == d.origin.max_age_s, cfg.origin.max_age_s.to_string(), d.origin.max_age_s.to_string());
    same("origin stale-while-revalidate", cfg.origin.stale_while_revalidate_s == d.origin.stale_while_revalidate_s, format!("{:?}", cfg.origin.stale_while_revalidate_s), "None".into());
    same("origin stale-if-error", cfg.origin.stale_if_error_s == d.origin.stale_if_error_s, format!("{:?}", cfg.origin.stale_if_error_s), "None".into());
    same("zipf alpha", cfg.workload.zipf_alpha == d.workload.zipf_alpha, cfg.workload.zipf_alpha.to_string(), d.workload.zipf_alpha.to_string());
    v
}

pub fn label_for(cfg: &ScenarioBConfig) -> (String, Vec<String>) {
    label_for_inputs(cfg, cfg!(debug_assertions))
}

/// Pure form of [`label_for`] (the driver build profile is a parameter so every branch is testable).
pub fn label_for_inputs(cfg: &ScenarioBConfig, driver_debug: bool) -> (String, Vec<String>) {
    let mut reasons = Vec::new();
    if driver_debug {
        reasons.push("phantom-lab / lab_live library built in the DEBUG profile".to_string());
    }
    let gp = gateway_profile(&cfg.gateway_exe);
    if gp != "release" {
        reasons.push(format!("gateway binary profile is '{gp}' (path {}), not release", cfg.gateway_exe.display()));
    }
    if cfg.reps < MIN_VALID_RUNS {
        reasons.push(format!("{} repetitions per arm < {} required", cfg.reps, MIN_VALID_RUNS));
    }
    if !cfg.arms.contains(&Arm::Base) {
        reasons.push("BASE arm missing: no paired comparison possible".to_string());
    } else if !cfg.arms.iter().any(|a| *a != Arm::Base) {
        reasons.push("no capped arm (STATIC-CAP or AIMD-CAP): at least one capped arm and BASE are required".to_string());
    }
    let dev = parameter_deviations(cfg);
    if !dev.is_empty() {
        reasons.push(format!("{DEVIATES_REASON}: {}", dev.join("; ")));
    }
    if reasons.is_empty() {
        ("VALID".into(), reasons)
    } else {
        ("EXPLORATORY".into(), reasons)
    }
}

// ---------------------------------------------------------------------------------------------
// File output
// ---------------------------------------------------------------------------------------------

fn io<T>(r: std::io::Result<T>, what: &str) -> Result<T, ScenarioError> {
    r.map_err(|e| ScenarioError::Io(format!("{what}: {e}")))
}

fn write_client_csv(path: &Path, rows: &[ClientRecord], warm_ns: u64) -> Result<(), ScenarioError> {
    let mut s = String::with_capacity(rows.len() * 80 + 128);
    s.push_str("idx,key,scheduled_ms,send_ms,done_ms,send_lag_ms,latency_ms,service_latency_ms,status,x_cache_status,x_cache_error,bytes,error,in_analysis_window\n");
    for r in rows {
        let _ = writeln!(
            s,
            "{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{}",
            r.idx,
            r.key,
            r.scheduled_ns as f64 / 1e6,
            r.send_ns as f64 / 1e6,
            r.done_ns as f64 / 1e6,
            r.send_lag_ms(),
            r.latency_ms(),
            r.service_latency_ms(),
            r.status,
            r.cache_status,
            r.cache_error,
            r.bytes,
            r.error.replace(',', ";"),
            (r.scheduled_ns >= warm_ns) as u8
        );
    }
    io(std::fs::write(path, s), "client_requests.csv")
}

fn write_origin_csv(path: &Path, rows: &[OriginRecord], sched_start_ns: u64) -> Result<(), ScenarioError> {
    let mut s = String::with_capacity(rows.len() * 50 + 64);
    s.push_str("arrival_ms_since_load_start,conc_at_arrival,service_ms,status,degraded,path_hash\n");
    let mut sorted: Vec<&OriginRecord> = rows.iter().collect();
    sorted.sort_by_key(|r| r.arrival_ns);
    for r in sorted {
        let _ = writeln!(
            s,
            "{:.3},{},{:.3},{},{},{:016x}",
            (r.arrival_ns as f64 - sched_start_ns as f64) / 1e6,
            r.conc_at_arrival,
            r.service_ns as f64 / 1e6,
            r.status,
            r.degraded as u8,
            r.path_hash
        );
    }
    io(std::fs::write(path, s), "origin_requests.csv")
}

// ---------------------------------------------------------------------------------------------
// One run
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn run_one(
    cfg: &ScenarioBConfig,
    arm: Arm,
    rep: usize,
    run_dir: &Path,
    cancel: &Arc<AtomicBool>,
    progress: &ProgressFn,
    done: usize,
    total: usize,
) -> Result<(RunMetrics, Vec<f64>), ScenarioError> {
    let pid_cell = std::cell::Cell::new(0u32);
    let emit = |phase: &str| {
        progress(Progress { done, total, arm: arm.name().to_string(), rep, phase: phase.to_string(), gateway_pid: pid_cell.get() });
    };
    if cancel.load(Ordering::Relaxed) {
        return Err(ScenarioError::Cancelled);
    }
    io(std::fs::create_dir_all(run_dir), "run dir")?;
    emit("starting");
    let run_seed = cfg.seed + rep as u64;
    let mut script = cfg.origin.clone();
    script.seed = run_seed;
    let origin = io(Origin::start(script).await, "origin start")?;
    let spec = cfg.gateway_spec(arm, origin.addr, run_dir.join("gateway.log"));
    let mut gw = match Gateway::spawn(&spec).await {
        Ok(g) => g,
        Err(e) => {
            let _ = origin.stop().await;
            return Err(ScenarioError::Gateway(e));
        }
    };
    let pid = gw.pid;
    pid_cell.set(pid);
    if cancel.load(Ordering::Relaxed) {
        gw.kill().await;
        let _ = origin.stop().await;
        return Err(ScenarioError::Cancelled);
    }

    // ---- verify the cap is (not) enforcing before measuring
    let verdict = match gw.metrics().await {
        Ok(v) => {
            let oc = &v["metrics"]["origin_cap"];
            let enforcing = oc["enforcing"]["v"].as_bool();
            let cap = oc["cap"]["v"].as_u64();
            match arm {
                Arm::Base => {
                    if enforcing == Some(false) && cap.is_none() {
                        Ok(())
                    } else {
                        Err(format!("BASE expected enforcing=false and no cap, got enforcing={enforcing:?} cap={cap:?}"))
                    }
                }
                _ => {
                    let want = cfg.cap_limit(arm).unwrap() as u64;
                    if enforcing == Some(true) && cap == Some(want) {
                        Ok(())
                    } else {
                        Err(format!("{} expected enforcing=true cap={want}, got enforcing={enforcing:?} cap={cap:?}", arm.name()))
                    }
                }
            }
        }
        Err(e) => Err(format!("metrics unavailable before measuring: {e}")),
    };
    if let Err(e) = verdict {
        let tail = gateway::log_tail(&gw.log_path, 20);
        gw.kill().await;
        let _ = origin.stop().await;
        return Err(ScenarioError::CapNotEnforcing(format!("{e}\n--- gateway log tail ---\n{tail}")));
    }

    // ---- run
    let arrivals = loadgen::schedule(&Workload { seed: run_seed, ..cfg.workload.clone() });
    let scheduled_total = arrivals.len();
    let w = Workload { seed: run_seed, ..cfg.workload.clone() };
    let epoch = Instant::now();
    let sched_start_ns = origin.begin_schedule();
    let stop_poll = Arc::new(AtomicBool::new(false));
    let snaps: Arc<std::sync::Mutex<(Vec<Value>, usize)>> = Arc::new(std::sync::Mutex::new((Vec::new(), 0)));
    let poll = {
        let stop = stop_poll.clone();
        let snaps = snaps.clone();
        let addr = gw.addr;
        let every = Duration::from_millis(cfg.snapshot_interval_ms.max(50));
        tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let t_ns = epoch.elapsed().as_nanos() as u64;
                match super::http::simple_get(addr, "/api/v1/metrics", Duration::from_secs(2)).await {
                    Ok(r) if r.status == 200 => match serde_json::from_slice::<Value>(&r.body) {
                        Ok(v) => snaps.lock().unwrap().0.push(gateway::flatten_snapshot(&v, t_ns)),
                        Err(_) => snaps.lock().unwrap().1 += 1,
                    },
                    _ => snaps.lock().unwrap().1 += 1,
                }
                tokio::time::sleep(every).await;
            }
        })
    };
    let ticker = {
        let progress = progress.clone();
        let ph = cfg.phases;
        let arm_name = arm.name().to_string();
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let marks = [
                (0, "warmup"),
                (ph.warmup_ms, "healthy"),
                (ph.degr_start_ms(), "degraded"),
                (ph.degr_end_ms(), "recovery"),
                (ph.total_ms(), "drain"),
            ];
            for (ms, name) in marks {
                let due = epoch + Duration::from_millis(ms);
                tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                progress(Progress { done, total, arm: arm_name.clone(), rep, phase: name.to_string(), gateway_pid: pid });
            }
        })
    };
    let client = loadgen::run_open_loop(gw.addr, &arrivals, &w, epoch, cancel.clone()).await;
    ticker.abort();
    // one last snapshot, then stop polling
    if let Ok(v) = gw.metrics().await {
        snaps.lock().unwrap().0.push(gateway::flatten_snapshot(&v, epoch.elapsed().as_nanos() as u64));
    }
    stop_poll.store(true, Ordering::Relaxed);
    let _ = poll.await;
    let gateway_alive = gw.alive();
    let cancelled = cancel.load(Ordering::Relaxed);
    emit("collect");
    let exit = gw.kill().await;
    let res = origin.stop().await;
    gateway::truncate_log_to_tail(&run_dir.join("gateway.log"), LOG_TAIL_LINES);
    if cancelled {
        return Err(ScenarioError::Cancelled);
    }
    let (snap_vec, snap_err) = {
        let g = snaps.lock().unwrap();
        (g.0.clone(), g.1)
    };
    let inputs = RunInputs {
        arm,
        rep,
        phases: cfg.phases,
        client: &client,
        scheduled_total,
        origin: &res.records,
        origin_sched_start_ns: sched_start_ns,
        origin_peak_whole_run: res.peak_concurrency,
        snaps: &snap_vec,
        snapshot_errors: snap_err,
        cap_limit: cfg.cap_limit(arm),
        gateway_alive,
        gateway_pid: pid,
        enforce_ok: true,
    };
    let (metrics, lat) = compute_run_metrics(&inputs);

    // ---- artifacts
    write_client_csv(&run_dir.join("client_requests.csv"), &client, cfg.phases.warmup_ms * 1_000_000)?;
    write_origin_csv(&run_dir.join("origin_requests.csv"), &res.records, sched_start_ns)?;
    let mut jl = String::new();
    for s in &snap_vec {
        jl.push_str(&s.to_string());
        jl.push('\n');
    }
    io(std::fs::write(run_dir.join("gateway_snapshots.jsonl"), jl), "snapshots")?;
    let mut mj = serde_json::to_value(&metrics).unwrap_or(Value::Null);
    mj["gateway_exit_status"] = json!(exit);
    mj["tier"] = json!(TIER);
    io(std::fs::write(run_dir.join("run_metrics.json"), serde_json::to_string_pretty(&mj).unwrap()), "run_metrics.json")?;
    Ok((metrics, lat))
}

// ---------------------------------------------------------------------------------------------
// Experiment
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ExperimentOutcome {
    pub out_dir: PathBuf,
    pub summary: Value,
    pub runs: Vec<RunMetrics>,
    pub cancelled: bool,
    pub label: String,
}

fn write_manifest(
    cfg: &ScenarioBConfig,
    extra: &Value,
    started_utc: &str,
    ended_utc: Option<&str>,
    status: &str,
    exit_code: Option<i32>,
) -> Result<(), ScenarioError> {
    let root = repo_root();
    let gw_sha = sha256_file(&cfg.gateway_exe).ok();
    let (label, label_reasons) = label_for(cfg);
    let m = json!({
        "manifest_version": 1,
        "tier": TIER,
        "scenario": "B (origin slowdown)",
        "status": status,
        "exit_code": exit_code,
        "started_utc": started_utc,
        "ended_utc": ended_utc,
        "command_line": cfg.command_line,
        "seed": cfg.seed,
        "seed_note": "repetition r uses seed + r for arrivals/keys/origin; identical across arms of the repetition; never a sealed M6 seed",
        "label_planned": label, "label_reasons": label_reasons,
        "label_meaning": LABEL_VALID_MEANING,
        "interpretation_notes": interpretation_notes(),
        "profile": {
            "driver_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "gateway_profile": gateway_profile(&cfg.gateway_exe),
        },
        "gateway": {
            "exe": cfg.gateway_exe.display().to_string(), "exe_sha256": gw_sha,
            "env_keys_set": ["PHANTOM_MODE=development", "PHANTOM_BIND_ADDR=127.0.0.1:<free port>", "ORIGIN_URL=http://127.0.0.1:<origin port>",
                "PHANTOM_MAX_CACHE_BYTES", "PHANTOM_INTELLIGENCE (off | canary)", "PHANTOM_ORIGIN_CAP_{MAX,MIN,P99_CEILING_MS,WINDOW} (capped arms only)"],
            "inherited_phantom_env_removed": true,
        },
        "git": { "head": git_head(&root), "dirty_entries": git_dirty_entries(&root) },
        "source_sha256": lab_live_source_hashes(),
        "host": host_info(),
        "workload": cfg.workload, "phases": cfg.phases, "origin_script": cfg.origin,
        "arms": cfg.arms.iter().map(|a| a.name()).collect::<Vec<_>>(),
        "arm_configs": {
            "BASE": {"PHANTOM_INTELLIGENCE": "off", "cap": null},
            "STATIC-CAP": {"PHANTOM_INTELLIGENCE": "canary", "min": cfg.static_cap, "max": cfg.static_cap},
            "AIMD-CAP": {"PHANTOM_INTELLIGENCE": "canary", "min": cfg.aimd_min, "max": cfg.aimd_max,
                "p99_ceiling_ms": cfg.aimd_p99_ceiling_ms, "window": cfg.aimd_window},
        },
        "max_cache_bytes": cfg.max_cache_bytes, "reps": cfg.reps,
        "snapshot_interval_ms": cfg.snapshot_interval_ms,
        "extra": extra,
    });
    io(std::fs::write(cfg.out_dir.join("manifest.json"), serde_json::to_string_pretty(&m).unwrap()), "manifest.json")
}

/// Runs the whole experiment. Returns `Ok` with `cancelled = true` if the cancel flag was raised (children are
/// killed first); `Err` for any run that cannot be trusted (cap not enforcing, gateway failure, I/O).
pub async fn run_experiment(
    cfg: &ScenarioBConfig,
    cancel: Arc<AtomicBool>,
    progress: ProgressFn,
) -> Result<ExperimentOutcome, ScenarioError> {
    if cfg.arms.is_empty() || cfg.reps == 0 {
        return Err(ScenarioError::Config("need at least one arm and one repetition".into()));
    }
    if cfg.static_cap == 0 || cfg.aimd_min == 0 || cfg.aimd_min > cfg.aimd_max {
        return Err(ScenarioError::Config("invalid cap configuration".into()));
    }
    if cfg.out_dir.join("manifest.json").exists() {
        return Err(ScenarioError::Config(format!("{} already holds an experiment", cfg.out_dir.display())));
    }
    io(std::fs::create_dir_all(cfg.out_dir.join("runs")), "out dir")?;
    let started = utc_now_iso();
    let probe = timer_probe().await;
    let extra = json!({ "timer_probe": probe });
    write_manifest(cfg, &extra, &started, None, "RUNNING", None)?;

    let total = cfg.reps * cfg.arms.len();
    let mut done = 0usize;
    let mut runs: Vec<RunMetrics> = Vec::new();
    let mut pooled: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut cancelled = false;
    let n = cfg.arms.len();
    'outer: for rep in 0..cfg.reps {
        for i in 0..n {
            let arm = cfg.arms[(i + rep) % n];
            let dir = cfg.out_dir.join("runs").join(format!("rep{:02}_{}", rep + 1, arm.name()));
            match run_one(cfg, arm, rep, &dir, &cancel, &progress, done, total).await {
                Ok((m, lat)) => {
                    if m.valid {
                        pooled.entry(m.arm.clone()).or_default().extend(lat);
                    }
                    runs.push(m);
                    done += 1;
                    progress(Progress { done, total, arm: arm.name().to_string(), rep, phase: "run-complete".into(), gateway_pid: 0 });
                }
                Err(ScenarioError::Cancelled) => {
                    cancelled = true;
                    break 'outer;
                }
                Err(e) => {
                    let _ = write_manifest(
                        cfg,
                        &json!({"timer_probe": extra["timer_probe"], "error": e.to_string()}),
                        &started,
                        Some(&utc_now_iso()),
                        "FAILED",
                        Some(1),
                    );
                    return Err(e);
                }
            }
        }
    }
    let (planned_label, planned_reasons) = label_for(cfg);
    // ExperimentOutcome.label (used by the Lab API result event) must agree with summary.json (G4C F02).
    let (label, _) = effective_label(&planned_label, &planned_reasons, incomplete_reason(cfg, &runs, cancelled));
    let meta = SummaryMeta { label: planned_label, label_reasons: planned_reasons, cancelled };
    let summary = summarize(cfg, &runs, &pooled, &meta);
    io(
        std::fs::write(cfg.out_dir.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap()),
        "summary.json",
    )?;
    let (status, code) = if cancelled { ("CANCELLED", 130) } else { ("COMPLETE", 0) };
    write_manifest(cfg, &extra, &started, Some(&utc_now_iso()), status, Some(code))?;
    Ok(ExperimentOutcome { out_dir: cfg.out_dir.clone(), summary, runs, cancelled, label })
}

/// Default gateway path: `phantom-cache-demo[.exe]` next to the running executable.
pub fn default_gateway_exe() -> PathBuf {
    let name = if cfg!(windows) { "phantom-cache-demo.exe" } else { "phantom-cache-demo" };
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// Applies `sync_derived` (phase-dependent fields) after the caller changed phases.
pub fn finalize_config(cfg: &mut ScenarioBConfig) {
    cfg.sync_derived();
}
