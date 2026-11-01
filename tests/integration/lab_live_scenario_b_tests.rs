//! Scenario B (T2, MEASURED_LOOPBACK) harness tests: schedule determinism, samplers, percentiles, the
//! mechanical verdict rule, origin degradation, and an end-to-end smoke with the REAL gateway process
//! (debug build: mechanics only, never performance evidence).

use phantom_cache_demo::lab::rng::{AliasSampler, StreamRng};
use phantom_cache_demo::lab_live::analysis::{evaluate_arm, paired, ArmEvidence, Outcome};
use phantom_cache_demo::lab_live::gateway::pid_alive;
use phantom_cache_demo::lab_live::http::simple_get;
use phantom_cache_demo::lab_live::loadgen::{percentile, schedule, ArrivalProcess, ClientRecord, Workload};
use phantom_cache_demo::lab_live::origin::{Origin, OriginRecord, OriginScript};
use phantom_cache_demo::lab_live::scenario_b::{
    compute_run_metrics, recovery_from_bins, run_experiment, smoke_config, Arm, Phases, Progress, ProgressFn, RunInputs,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static E2E_LOCK: Mutex<()> = Mutex::new(());

fn gateway_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_phantom-cache-demo"))
}

fn tmp_dir(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("sb_test_{}_{}_{}", name, std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn wl(seed: u64) -> Workload {
    Workload {
        seed,
        n_keys: 500,
        zipf_alpha: 0.9,
        rate_rps: 200.0,
        duration_ms: 5_000,
        arrival: ArrivalProcess::Poisson,
        path_prefix: "/k/".into(),
        client_deadline_ms: 5_000,
    }
}

// ------------------------------------------------------------------ determinism, samplers, percentiles

#[test]
fn schedule_is_deterministic_and_seed_sensitive() {
    let a = schedule(&wl(7));
    let b = schedule(&wl(7));
    let c = schedule(&wl(8));
    assert_eq!(a, b, "same workload must give the identical schedule (arm independent)");
    assert_ne!(a, c, "a different seed must change the schedule");
    assert!(a.windows(2).all(|w| w[0].t_ns <= w[1].t_ns), "arrivals are ordered");
    assert!(a.iter().all(|x| x.t_ns < 5_000 * 1_000_000));
    // Poisson with rate 200/s over 5 s: ~1000 arrivals (6 sigma ~ 190)
    assert!((a.len() as i64 - 1000).abs() < 200, "n={}", a.len());
    assert!(a.iter().all(|x| (x.key as usize) < 500));
}

#[test]
fn regular_arrivals_are_evenly_spaced() {
    let mut w = wl(1);
    w.arrival = ArrivalProcess::Regular;
    w.rate_rps = 100.0;
    w.duration_ms = 1_000;
    let s = schedule(&w);
    assert!(s.len() == 99 || s.len() == 100, "n={}", s.len());
    let gap = s[1].t_ns - s[0].t_ns;
    assert!((gap as i64 - 10_000_000).abs() < 10);
}

#[test]
fn zipf_sampler_matches_theory() {
    let n = 100;
    let sampler = AliasSampler::new_zipf(n, 1.0);
    let mut rng = StreamRng::new(42);
    let mut counts = vec![0usize; n];
    let draws = 200_000;
    for _ in 0..draws {
        counts[sampler.sample(&mut rng)] += 1;
    }
    let h: f64 = (1..=n).map(|k| 1.0 / k as f64).sum();
    let p0 = counts[0] as f64 / draws as f64;
    assert!((p0 - 1.0 / h).abs() < 0.01, "p0={p0} expected {}", 1.0 / h);
    let p9 = counts[9] as f64 / draws as f64;
    assert!((p9 - 0.1 / h).abs() < 0.005, "p9={p9}");
    assert!(counts[0] > counts[9] && counts[9] > counts[99]);
}

#[test]
fn percentile_nearest_rank() {
    let v: Vec<f64> = (1..=100).map(|x| x as f64).collect();
    assert_eq!(percentile(&v, 50.0), Some(50.0));
    assert_eq!(percentile(&v, 99.0), Some(99.0));
    assert_eq!(percentile(&v, 100.0), Some(100.0));
    assert_eq!(percentile(&v, 0.0), Some(1.0));
    assert_eq!(percentile(&[], 99.0), None, "no samples is None, never 0");
    assert_eq!(percentile(&[f64::NAN, 3.0, 1.0], 100.0), Some(3.0), "non-finite values are dropped");
    // order independent
    let mut r = v.clone();
    r.reverse();
    assert_eq!(percentile(&r, 95.0), Some(95.0));
}

// ------------------------------------------------------------------ mechanical rule

fn ev() -> ArmEvidence {
    ArmEvidence {
        n_valid_pairs: 10,
        contention_flagged: false,
        mechanism_pass: true,
        rel_reduction_mean: Some(0.40),
        rel_ci_lo: Some(0.30),
        rel_ci_hi: Some(0.50),
        origin_p99_diff_mean: Some(-5.0),
        success_diff_pp_mean: Some(-0.2),
    }
}

#[test]
fn rule_covers_every_outcome() {
    assert_eq!(evaluate_arm(&ev()).outcome, Outcome::BenefitSupported);

    let mut e = ev();
    e.mechanism_pass = false;
    e.n_valid_pairs = 3; // a mechanism failure is never masked by "inconclusive"
    assert_eq!(evaluate_arm(&e).outcome, Outcome::MechanismFailed);

    let mut e = ev();
    e.n_valid_pairs = 9;
    assert_eq!(evaluate_arm(&e).outcome, Outcome::Inconclusive);

    let mut e = ev();
    e.contention_flagged = true;
    assert_eq!(evaluate_arm(&e).outcome, Outcome::Inconclusive);

    // CI includes zero and cannot exclude 5%: inconclusive
    let mut e = ev();
    e.rel_reduction_mean = Some(0.04);
    e.rel_ci_lo = Some(-0.05);
    e.rel_ci_hi = Some(0.13);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::Inconclusive);

    // CI includes zero, upper bound < 5%: a 5% effect is excluded -> not supported
    let mut e = ev();
    e.rel_reduction_mean = Some(0.01);
    e.rel_ci_lo = Some(-0.02);
    e.rel_ci_hi = Some(0.03);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::BenefitNotSupported);

    // reduction < 5% with CI excluding zero -> not supported
    let mut e = ev();
    e.rel_reduction_mean = Some(0.03);
    e.rel_ci_lo = Some(0.02);
    e.rel_ci_hi = Some(0.04);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::BenefitNotSupported);

    // benefit but success rate 1.5 pp below BASE -> trade-off failure
    let mut e = ev();
    e.success_diff_pp_mean = Some(-1.5);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::TradeOffFailure);

    // exactly at the 1 pp limit is still within tolerance
    let mut e = ev();
    e.success_diff_pp_mean = Some(-1.0);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::BenefitSupported);

    // origin p99 worse -> not supported
    let mut e = ev();
    e.origin_p99_diff_mean = Some(3.0);
    assert_eq!(evaluate_arm(&e).outcome, Outcome::BenefitNotSupported);

    // missing endpoint -> inconclusive (never treated as zero)
    let mut e = ev();
    e.rel_reduction_mean = None;
    assert_eq!(evaluate_arm(&e).outcome, Outcome::Inconclusive);
}

#[test]
fn paired_detects_planted_effect_and_no_effect() {
    let base: Vec<f64> = (0..10).map(|i| 40.0 + i as f64).collect();
    let treat: Vec<f64> = base.iter().map(|b| b * 0.5).collect();
    let p = paired(&base, &treat, 11).unwrap();
    assert_eq!(p.n_pairs, 10);
    assert!((p.rel_reduction_mean.unwrap() - 0.5).abs() < 1e-9);
    assert!(p.perm_p.unwrap() < 0.01);
    let same = paired(&base, &base, 11).unwrap();
    assert_eq!(same.rel_reduction_mean, Some(0.0));
    assert_eq!(same.perm_p, Some(1.0));
    // NaN pairs are dropped, not zeroed
    let p = paired(&[1.0, f64::NAN, 3.0], &[1.0, 5.0, 3.0], 1).unwrap();
    assert_eq!(p.n_pairs, 2);
    assert!(paired(&[], &[], 1).is_none());
}

#[test]
fn recovery_bins() {
    assert_eq!(recovery_from_bins(&[Some(0.0), Some(0.0)], 0.02), Some(0.0));
    assert_eq!(recovery_from_bins(&[Some(0.5), Some(0.3), Some(0.0), Some(0.0)], 0.02), Some(2.0));
    assert_eq!(recovery_from_bins(&[Some(0.5), Some(0.0), Some(0.4)], 0.02), None, "last bin still bad = not recovered");
    assert_eq!(recovery_from_bins(&[], 0.02), None);
}

// ------------------------------------------------------------------ run metrics from raw records

fn crec(idx: u32, sched_ms: u64, lat_ms: u64, status: u16, cache_error: &str) -> ClientRecord {
    ClientRecord {
        idx,
        key: idx,
        scheduled_ns: sched_ms * 1_000_000,
        send_ns: sched_ms * 1_000_000 + 1_000_000,
        done_ns: (sched_ms + lat_ms) * 1_000_000,
        status,
        cache_status: if status == 200 { "HIT".into() } else { String::new() },
        cache_error: cache_error.into(),
        bytes: 10,
        error: if status == 0 { "deadline".into() } else { String::new() },
    }
}

fn orec(arrival_ms: u64, conc: u32, svc_ms: u64, degraded: bool) -> OriginRecord {
    OriginRecord { arrival_ns: arrival_ms * 1_000_000, conc_at_arrival: conc, service_ns: svc_ms * 1_000_000, status: 200, degraded, path_hash: 0 }
}

#[test]
fn run_metrics_window_shed_mechanism_and_no_zero_fill() {
    let ph = Phases { warmup_ms: 1000, healthy_ms: 1000, degraded_ms: 1000, recovery_ms: 1000 };
    let mut client = vec![crec(0, 100, 5, 200, "")]; // warm-up: excluded
    for i in 0..90u32 {
        client.push(crec(i + 1, 1000 + i as u64 * 10, 20, 200, ""));
    }
    for i in 0..10u32 {
        client.push(crec(100 + i, 2000 + i as u64 * 10, 30, 503, "CapacityExceeded"));
    }
    client.push(crec(200, 2500, 5000, 0, ""));
    let origin = vec![orec(50, 99, 20, false), orec(1500, 3, 20, false), orec(2500, 9, 120, true)];
    let snaps: Vec<serde_json::Value> = (0..6)
        .map(|i| serde_json::json!({"t_ns": i * 500_000_000u64, "origin_cap": {"enforcing": true, "cap": 8, "in_flight": 2, "rejections": i, "dispatched": 10, "controller_steps": 0}, "stale_served_total": 0}))
        .collect();
    let inp = RunInputs {
        arm: Arm::StaticCap,
        rep: 0,
        phases: ph,
        client: &client,
        scheduled_total: client.len(),
        origin: &origin,
        origin_sched_start_ns: 0,
        origin_peak_whole_run: 99,
        snaps: &snaps,
        snapshot_errors: 0,
        cap_limit: Some(8),
        gateway_alive: true,
        gateway_pid: 1,
        enforce_ok: true,
    };
    let (m, lat) = compute_run_metrics(&inp);
    assert_eq!(m.n_scheduled_window, 101, "warm-up request excluded");
    assert_eq!(lat.len(), 101);
    assert_eq!(m.n_no_response_window, 1);
    assert!((m.shed_fraction.unwrap() - 10.0 / 101.0).abs() < 1e-12);
    assert!((m.success_rate.unwrap() - 90.0 / 101.0).abs() < 1e-12);
    assert_eq!(m.origin_peak_window, Some(9), "the 99 at 50 ms is warm-up and excluded from the window peak");
    assert_eq!(m.origin_peak_whole_run, 99);
    assert!(m.mechanism_violated, "whole-run peak 99 > cap 8 must be flagged");
    assert_eq!(m.client_max_ms, Some(5000.0), "a request with no response counts at its deadline");
    assert!(m.valid, "{:?}", m.invalid_reasons);

    // no origin records in window: None, not 0
    let inp2 = RunInputs { origin: &[], origin_peak_whole_run: 0, ..inp };
    let (m2, _) = compute_run_metrics(&inp2);
    assert_eq!(m2.origin_peak_window, None);
    assert_eq!(m2.origin_p99_service_ms, None);
    assert!(!m2.mechanism_violated);
}

// ------------------------------------------------------------------ scripted origin

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn origin_applies_degradation_schedule_and_records_every_request() {
    let script = OriginScript {
        base_service_ms: 10,
        degraded_mult: 8.0,
        degrade_start_ms: 200,
        degrade_len_ms: 400,
        error_rate: 0.0,
        max_age_s: 3,
        stale_while_revalidate_s: Some(7),
        stale_if_error_s: None,
        body_bytes: 256,
        seed: 1,
    };
    let origin = Origin::start(script).await.unwrap();
    let t0 = origin.begin_schedule();
    let healthy = simple_get(origin.addr, "/a", Duration::from_secs(2)).await.unwrap();
    assert_eq!(healthy.status, 200);
    assert_eq!(healthy.body_len, 256);
    assert_eq!(healthy.header("cache-control"), Some("max-age=3, stale-while-revalidate=7"));
    // deterministic per-path body
    let b1 = simple_get(origin.addr, "/same", Duration::from_secs(2)).await.unwrap();
    let b2 = simple_get(origin.addr, "/same", Duration::from_secs(2)).await.unwrap();
    let b3 = simple_get(origin.addr, "/other", Duration::from_secs(2)).await.unwrap();
    assert_eq!(b1.body, b2.body);
    assert_ne!(b1.body, b3.body);
    tokio::time::sleep(Duration::from_millis(250)).await;
    let slow = simple_get(origin.addr, "/b", Duration::from_secs(3)).await.unwrap();
    assert_eq!(slow.status, 200);
    let res = origin.stop().await;
    assert_eq!(res.records.len(), 5, "every request is recorded");
    assert_eq!(res.schedule_start_ns, Some(t0));
    let degraded: Vec<_> = res.records.iter().filter(|r| r.degraded).collect();
    assert_eq!(degraded.len(), 1);
    assert!(degraded[0].service_ns >= 80_000_000, "x8 of 10 ms, got {} ns", degraded[0].service_ns);
    let healthy_svc = res.records.iter().filter(|r| !r.degraded).map(|r| r.service_ns).max().unwrap();
    assert!(healthy_svc < 80_000_000, "healthy requests stay near base: {healthy_svc}");
    assert!(res.records.iter().all(|r| r.conc_at_arrival >= 1));
}

// ------------------------------------------------------------------ end to end with the real gateway

fn smoke_run(dir: &std::path::Path, phase_ms: u64, cancel: Arc<AtomicBool>, progress: ProgressFn) -> phantom_cache_demo::lab_live::scenario_b::ExperimentOutcome {
    let cfg = smoke_config(gateway_exe(), dir.to_path_buf(), phase_ms);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
    rt.block_on(run_experiment(&cfg, cancel, progress)).expect("experiment")
}

fn wait_dead(pid: u32) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(10) {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// 1 rep x 3 arms, 2 s phases, debug gateway. Asserts artifacts, cap enforcement, the mutation-style
/// detectability check (BASE exceeds the cap the STATIC-CAP arm respects), and that no gateway is left running.
#[test]
fn e2e_smoke_three_arms_artifacts_cap_and_no_orphans() {
    let _g = E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tmp_dir("smoke");
    let pids: Arc<Mutex<HashSet<u32>>> = Arc::new(Mutex::new(HashSet::new()));
    let p2 = pids.clone();
    let progress: ProgressFn = Arc::new(move |p: Progress| {
        if p.gateway_pid != 0 {
            p2.lock().unwrap().insert(p.gateway_pid);
        }
    });
    let o = smoke_run(&dir, 2000, Arc::new(AtomicBool::new(false)), progress);
    assert!(!o.cancelled);
    assert_eq!(o.label, "EXPLORATORY", "debug / 1 rep must never be labelled VALID");
    assert_eq!(o.summary["tier"], "MEASURED_LOOPBACK");
    assert_eq!(o.runs.len(), 3);
    for f in ["manifest.json", "summary.json"] {
        assert!(dir.join(f).exists(), "{f}");
    }
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["status"], "COMPLETE");
    assert_eq!(manifest["exit_code"], 0);
    assert!(manifest["started_utc"].as_str().unwrap().ends_with('Z'));
    assert!(manifest["gateway"]["exe_sha256"].as_str().unwrap().len() == 64);
    assert!(manifest["source_sha256"]["src/lab_live/scenario_b.rs"].as_str().is_some());
    for arm in ["BASE", "STATIC-CAP", "AIMD-CAP"] {
        let rd = dir.join("runs").join(format!("rep01_{arm}"));
        for f in ["client_requests.csv", "origin_requests.csv", "gateway_snapshots.jsonl", "gateway.log", "run_metrics.json"] {
            let p = rd.join(f);
            assert!(p.exists() && std::fs::metadata(&p).unwrap().len() > 0, "{} missing/empty", p.display());
        }
    }
    let by = |a: &str| o.runs.iter().find(|r| r.arm == a).unwrap();
    for r in &o.runs {
        assert!(r.valid, "{} invalid: {:?}", r.arm, r.invalid_reasons);
        assert!(r.n_scheduled_window > 100, "{} samples {}", r.arm, r.n_scheduled_window);
    }
    // caps enforce end to end
    let st = by("STATIC-CAP");
    assert_eq!(st.cap_limit, Some(8));
    assert!(st.origin_peak_whole_run <= 8, "static cap violated: origin peak {}", st.origin_peak_whole_run);
    assert_eq!(st.enforcing_all_snapshots, Some(true));
    let am = by("AIMD-CAP");
    assert!(am.origin_peak_whole_run <= 32, "aimd max violated: {}", am.origin_peak_whole_run);
    assert_eq!(o.summary["mechanism_h_b1_all_pass"], true);
    // mutation-style: with the cap disabled the same workload drives the origin above the static cap
    let base = by("BASE");
    assert_eq!(base.enforcing_all_snapshots, Some(false));
    assert!(
        base.origin_peak_whole_run > 8,
        "experiment cannot detect the mechanism: BASE peak {} <= cap 8",
        base.origin_peak_whole_run
    );
    assert!(st.gateway_rejections_final.unwrap_or(0) > 0, "cap should have shed some requests");
    // no orphan gateways
    let pids = pids.lock().unwrap().clone();
    assert_eq!(pids.len(), 3, "three gateway processes were spawned");
    for pid in pids {
        assert!(wait_dead(pid), "gateway pid {pid} is still running");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Cancellation: raise the flag during the first run; the experiment returns promptly with CANCELLED status
/// and the gateway child is dead.
#[test]
fn e2e_cancellation_terminates_children() {
    let _g = E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tmp_dir("cancel");
    let cancel = Arc::new(AtomicBool::new(false));
    let seen_pid = Arc::new(AtomicU32::new(0));
    let (c2, s2) = (cancel.clone(), seen_pid.clone());
    let progress: ProgressFn = Arc::new(move |p: Progress| {
        if p.phase == "healthy" {
            s2.store(p.gateway_pid, Ordering::SeqCst);
            c2.store(true, Ordering::SeqCst);
        }
    });
    let t = Instant::now();
    let o = smoke_run(&dir, 1500, cancel, progress);
    assert!(o.cancelled);
    assert!(t.elapsed() < Duration::from_secs(20), "cancellation must be prompt, took {:?}", t.elapsed());
    let pid = seen_pid.load(Ordering::SeqCst);
    assert!(pid != 0, "a gateway pid was reported");
    assert!(wait_dead(pid), "gateway {pid} survived cancellation");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["status"], "CANCELLED");
    assert_eq!(o.summary["status"], "CANCELLED");
    assert!(o.runs.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Heavy: 10 repetitions x 3 arms with short phases in the current (debug) profile. Mechanics only.
#[test]
#[ignore]
fn heavy_ten_reps_short_phases_is_exploratory_in_debug() {
    let _g = E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tmp_dir("heavy");
    let mut cfg = smoke_config(gateway_exe(), dir.clone(), 2000);
    cfg.reps = 10;
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().unwrap();
    let progress: ProgressFn = Arc::new(|_| {});
    let o = rt.block_on(run_experiment(&cfg, Arc::new(AtomicBool::new(false)), progress)).unwrap();
    assert_eq!(o.runs.len(), 30);
    assert_eq!(o.label, "EXPLORATORY");
    assert!(o.summary["verdicts"]["STATIC-CAP"]["outcome"].is_string());
}

// ------------------------------------------------------------------ U-01: incomplete runs get no verdicts

mod u01_incomplete {
    use super::*;
    use phantom_cache_demo::lab_live::scenario_b::{effective_label, incomplete_reason, summarize, RunMetrics, ScenarioBConfig, SummaryMeta};
    use std::collections::BTreeMap;

    fn cfg_with(reps: usize, arms: Vec<Arm>) -> ScenarioBConfig {
        let mut c = ScenarioBConfig::default_experiment(PathBuf::from("C:/x/target/release/phantom-cache-demo.exe"), PathBuf::from("out"));
        c.reps = reps;
        c.arms = arms;
        c
    }

    /// One real RunMetrics via compute_run_metrics (same inputs shape as the existing metrics test).
    fn run_of(arm: Arm, rep: usize, peak_whole: usize, cap: Option<usize>) -> RunMetrics {
        let ph = Phases { warmup_ms: 1000, healthy_ms: 1000, degraded_ms: 1000, recovery_ms: 1000 };
        let mut client = vec![crec(0, 100, 5, 200, "")];
        for i in 0..90u32 {
            client.push(crec(i + 1, 1000 + i as u64 * 10, 20, 200, ""));
        }
        let origin = vec![orec(1500, 3, 20, false), orec(2500, 5, 60, true)];
        let snaps: Vec<serde_json::Value> = (0..6)
            .map(|i| serde_json::json!({"t_ns": i * 500_000_000u64, "origin_cap": {"enforcing": cap.is_some(), "cap": cap, "in_flight": 2, "rejections": i, "dispatched": 10, "controller_steps": 0}, "stale_served_total": 0}))
            .collect();
        let inp = RunInputs {
            arm,
            rep,
            phases: ph,
            client: &client,
            scheduled_total: client.len(),
            origin: &origin,
            origin_sched_start_ns: 0,
            origin_peak_whole_run: peak_whole,
            snaps: &snaps,
            snapshot_errors: 0,
            cap_limit: cap,
            gateway_alive: true,
            gateway_pid: 1,
            enforce_ok: true,
        };
        let (m, _) = compute_run_metrics(&inp);
        assert!(m.valid, "{:?}", m.invalid_reasons);
        m
    }

    fn meta(cancelled: bool) -> SummaryMeta {
        // the planned label is VALID (release binaries, documented workload); it must not survive on an incomplete summary
        SummaryMeta { label: "VALID".into(), label_reasons: vec![], cancelled }
    }

    fn no_outcome_anywhere(v: &serde_json::Value) {
        let text = v.to_string();
        for banned in ["MECHANISM_FAILED", "H-B1 falsified", "\"outcome\"", "INCONCLUSIVE", "BENEFIT_SUPPORTED", "BENEFIT_NOT_SUPPORTED"] {
            assert!(!text.contains(banned), "incomplete summary must not carry {banned}");
        }
    }

    fn assert_incomplete(s: &serde_json::Value, reason: &str) {
        assert!(s["verdicts"].is_null(), "verdicts must be null, got {}", s["verdicts"]);
        assert!(s["mechanism_h_b1_all_pass"].is_null(), "all_pass must be null, got {}", s["mechanism_h_b1_all_pass"]);
        assert_eq!(s["verdicts_status"], "not_computed");
        assert!(s["verdicts_reason"].is_string());
        assert_eq!(s["run_completeness"]["complete"], false);
        assert_eq!(s["run_completeness"]["incomplete_reason"], reason);
        assert_ne!(s["label"], "VALID", "VALID must not label an incomplete summary");
        assert_eq!(s["label"], "EXPLORATORY");
        assert_eq!(s["label_planned"], "VALID");
        assert!(s["label_reasons"].as_array().unwrap().iter().any(|r| r.as_str().unwrap().contains("incomplete")));
        for (_, m) in s["mechanism_h_b1"].as_object().unwrap() {
            assert!(m["pass"].is_null(), "mechanism pass must be null on an incomplete run: {m}");
        }
        no_outcome_anywhere(&s["verdicts"]);
        no_outcome_anywhere(&s["mechanism_h_b1"]);
    }

    #[test]
    fn cancelled_with_zero_runs_gets_no_verdict_and_no_h_b1_falsified() {
        let cfg = cfg_with(10, vec![Arm::Base, Arm::StaticCap, Arm::AimdCap]);
        let s = summarize(&cfg, &[], &BTreeMap::new(), &meta(true));
        assert_eq!(s["status"], "CANCELLED");
        assert_incomplete(&s, "cancelled");
        assert_eq!(s["summary_version"], 2);
        assert_eq!(s["mechanism_h_b1"]["STATIC-CAP"]["runs"], 0);
        assert!(s["mechanism_h_b1"]["STATIC-CAP"]["max_origin_concurrent_observed_whole_run"].is_null());
    }

    #[test]
    fn cancelled_partial_keeps_raw_diagnostics_but_withholds_verdict() {
        let cfg = cfg_with(10, vec![Arm::Base, Arm::StaticCap]);
        let runs = vec![run_of(Arm::Base, 0, 40, None), run_of(Arm::StaticCap, 0, 20, Some(24))];
        let s = summarize(&cfg, &runs, &BTreeMap::new(), &meta(true));
        assert_incomplete(&s, "cancelled");
        assert_eq!(s["summary_version"], 2);
        assert_eq!(s["arms"]["BASE"]["runs"], 1);
        assert_eq!(s["arms"]["STATIC-CAP"]["runs"], 1);
        assert!(s["arms"]["STATIC-CAP"]["per_run_mean"]["client_p50_ms"].is_number());
        assert_eq!(s["mechanism_h_b1"]["STATIC-CAP"]["max_origin_concurrent_observed_whole_run"], 20);
        assert_eq!(s["run_completeness"]["completed_runs"], 2);
        assert_eq!(s["run_completeness"]["planned_runs"], 20);
    }

    #[test]
    fn observed_violation_in_a_cancelled_run_is_kept_as_observation_only() {
        let cfg = cfg_with(10, vec![Arm::Base, Arm::StaticCap]);
        let runs = vec![run_of(Arm::Base, 0, 40, None), run_of(Arm::StaticCap, 0, 99, Some(24))];
        let s = summarize(&cfg, &runs, &BTreeMap::new(), &meta(true));
        assert_incomplete(&s, "cancelled");
        assert_eq!(s["summary_version"], 2);
        assert_eq!(s["mechanism_h_b1"]["STATIC-CAP"]["violating_reps"], serde_json::json!([0]));
    }

    #[test]
    fn not_cancelled_but_missing_runs_is_also_incomplete() {
        let cfg = cfg_with(2, vec![Arm::Base, Arm::StaticCap]);
        let runs = vec![run_of(Arm::Base, 0, 40, None), run_of(Arm::Base, 1, 40, None), run_of(Arm::StaticCap, 0, 20, Some(24))];
        let s = summarize(&cfg, &runs, &BTreeMap::new(), &meta(false));
        assert_eq!(s["status"], "COMPLETE");
        assert_incomplete(&s, "runs_missing");
        assert_eq!(s["summary_version"], 2);
    }

    #[test]
    fn completed_run_keeps_verdict_semantics() {
        let cfg = cfg_with(1, vec![Arm::Base, Arm::StaticCap]);
        let runs = vec![run_of(Arm::Base, 0, 40, None), run_of(Arm::StaticCap, 0, 20, Some(24))];
        let s = summarize(&cfg, &runs, &BTreeMap::new(), &meta(false));
        assert_eq!(s["status"], "COMPLETE");
        assert_eq!(s["label"], "VALID", "completed-run label is unchanged");
        assert_eq!(s["summary_version"], 2);
        assert_eq!(s["verdicts_status"], "computed");
        assert_eq!(s["run_completeness"]["complete"], true);
        assert!(s["run_completeness"]["incomplete_reason"].is_null());
        assert_eq!(s["mechanism_h_b1"]["STATIC-CAP"]["pass"], true);
        assert_eq!(s["mechanism_h_b1_all_pass"], true);
        // 1 valid pair < 10 required: the pre-existing rule outcome, unchanged
        assert_eq!(s["verdicts"]["STATIC-CAP"]["outcome"], "INCONCLUSIVE");
    }

    #[test]
    fn completed_run_with_real_violation_is_still_mechanism_failed() {
        let cfg = cfg_with(1, vec![Arm::Base, Arm::StaticCap]);
        let runs = vec![run_of(Arm::Base, 0, 40, None), run_of(Arm::StaticCap, 0, 99, Some(24))];
        let s = summarize(&cfg, &runs, &BTreeMap::new(), &meta(false));
        assert_eq!(s["summary_version"], 2);
        assert_eq!(s["mechanism_h_b1"]["STATIC-CAP"]["pass"], false);
        assert_eq!(s["mechanism_h_b1_all_pass"], false);
        assert_eq!(s["verdicts"]["STATIC-CAP"]["outcome"], "MECHANISM_FAILED");
    }

    #[test]
    fn isolated_label_suppresses_planned_valid_to_exploratory_when_incomplete() {
        let cfg = cfg_with(2, vec![Arm::Base, Arm::StaticCap]);
        assert_eq!(incomplete_reason(&cfg, &[], true), Some("cancelled"));
        assert_eq!(incomplete_reason(&cfg, &[], false), Some("runs_missing"));

        // F6: Pure isolated test proving that a planned VALID label is suppressed to EXPLORATORY
        let (label_complete, reasons_complete) = effective_label("VALID", &["release profile".into()], None);
        assert_eq!(label_complete, "VALID");
        assert_eq!(reasons_complete, vec!["release profile".to_string()]);

        let (label_cancelled, reasons_cancelled) = effective_label("VALID", &["release profile".into()], Some("cancelled"));
        assert_eq!(label_cancelled, "EXPLORATORY");
        assert!(reasons_cancelled.iter().any(|r| r.contains("run incomplete (cancelled)")));

        let (label_missing, reasons_missing) = effective_label("VALID", &["release profile".into()], Some("runs_missing"));
        assert_eq!(label_missing, "EXPLORATORY");
        assert!(reasons_missing.iter().any(|r| r.contains("run incomplete (runs_missing)")));
    }
}

// G2: Lab HTTP API tests (real router, real JWTs, real runners) live in a submodule of this registered target.
#[path = "lab_api/mod.rs"]
mod lab_api_tests;
