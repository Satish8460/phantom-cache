//! Candidate Intelligence Evaluation & Dispositions Tests (M5-09..M5-13 / WP-36 / WP-18)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-09..M5-13

use phantom_cache_demo::intelligence::candidates::*;
use phantom_cache_demo::intelligence::executor::MAX_HARD_STALE_MS;

// ----------------------------------------------------------------------------
// 1. Statistical Policy Candidates (M5-09)
// ----------------------------------------------------------------------------

#[test]
fn test_statistical_eviction_shadow_selection_scan_workload() {
    let mut candidate = ShadowSelectedEvictionCandidate::new(30, 0.05);

    // Initial warm-up of hot keys
    for k in 1..=10u64 {
        candidate.observe(k);
        candidate.observe(k);
    }

    // Interleaved scan cycles: 30 cycles of (hot keys 1..10, then 40 one-hit scan keys)
    // On each cycle, 40 scan keys evict all 10 hot keys in LRU (cap 30),
    // whereas S3-FIFO isolates scan keys in small FIFO and protects hot keys in Main FIFO.
    for cycle in 0..30 {
        for k in 1..=10u64 {
            candidate.observe(k);
        }
        for k in 1000..1040u64 {
            let scan_key = (cycle * 1000) + k;
            candidate.observe(scan_key);
        }
    }

    // Under scan-heavy workloads, S3-FIFO significantly outperforms LRU
    let rec = candidate.evaluate_switch_recommendation();
    assert!(rec.is_some(), "Expected recommendation under scan workload");
    let report = rec.unwrap();
    assert_eq!(report.candidate_policy, "S3-FIFO");
    assert!(report.delta_hit_ratio > 0.05, "Delta hit ratio {} <= delta_min 0.05", report.delta_hit_ratio);
    assert!(report.confidence_interval_95.0 > 0.0, "95% CI lower bound must exclude zero");
}

#[test]
fn test_frequency_admission_filter_rejects_one_hit_wonders() {
    let mut filter = FrequencyAdmissionFilter::new();

    // Hot key 42 accessed 50 times
    for _ in 0..50 {
        filter.observe(42);
    }

    // One-hit wonder scan key 9999 accessed once
    filter.observe(9999);

    // Admission test: incoming scan key (freq 1) vs hot key victim (freq 50)
    assert!(
        !filter.should_admit(9999, 42),
        "Frequency filter must reject one-hit wonder from replacing hot victim"
    );

    // Hot key (freq 50) replaces cold key (freq 1)
    assert!(
        filter.should_admit(42, 9999),
        "Frequency filter must admit hot key over cold victim"
    );
}

// ----------------------------------------------------------------------------
// 2. Origin-Aware Control Candidates (M5-10)
// ----------------------------------------------------------------------------

#[test]
fn test_origin_aware_stale_window_extension() {
    let mut controller = OriginAwareStaleController::new(60_000, 0.05);

    // Normal health: error rate 0.0, latency 20ms -> stale window stays at base 60s
    let s0 = controller.update(0.0, 20);
    assert_eq!(s0, 60_000);

    // Outage / degradation spike: error rate 50%
    let s1 = controller.update(0.50, 500);
    assert!(s1 >= 600_000, "Stale window must expand during outage: {}", s1);

    // Severe outage: latency 8000ms, error rate 80%
    let s2 = controller.update(0.80, 8000);
    assert!(s2 > s1);
    assert!(
        s2 <= MAX_HARD_STALE_MS,
        "Stale window must never exceed RFC 9111 hard cap ({} ms)",
        MAX_HARD_STALE_MS
    );

    // Origin recovers: healthy latency and 0% errors
    let s3 = controller.update(0.0, 20);
    assert!(s3 < s2, "Stale window must contract upon recovery: {}", s3);
}

#[test]
fn test_origin_concurrency_cap_aimd_stabilization() {
    let mut controller = OriginConcurrencyCapController::new(4, 128, 500);

    assert_eq!(controller.current_concurrency_limit, 128);

    // Latency spike to 800ms (> 500ms ceiling) -> Multiplicative Decrease (30% backoff)
    let c1 = controller.step(800, 0.0);
    assert_eq!(c1, (128.0 * 0.70) as usize);

    // Sustained latency spike -> further multiplicative decrease down towards min_concurrency
    let c2 = controller.step(900, 0.10);
    assert!(c2 < c1);

    // Recovered: latency drops to 50ms -> Additive Increase (+1)
    let c3 = controller.step(50, 0.0);
    assert_eq!(c3, c2 + 1);
}

// ----------------------------------------------------------------------------
// 3. Predictive Candidates (M5-11)
// ----------------------------------------------------------------------------

#[test]
fn test_predictive_rate_forecaster_mase_evaluation() {
    let mut forecaster = PredictiveRateForecaster::new(0.5);

    // Series with gentle upward trend: 100, 105, 110, 115, ...
    for i in 0..50 {
        let rate = 100.0 + (i as f64) * 2.0;
        forecaster.record_and_forecast(rate);
    }

    let mase = forecaster.compute_mase();
    assert!(
        mase > 0.0 && mase < 2.0,
        "MASE calculation returned invalid value: {}",
        mase
    );
}

#[test]
fn test_hot_key_emergence_predictor_accuracy() {
    let predictor = HotKeyEmergencePredictor::new(3.0); // 3x velocity threshold

    // Key with stationary traffic: 20 -> 22 (1.1x)
    assert!(!predictor.is_emergent(20, 22));

    // Emergent key with velocity burst: 10 -> 35 (3.5x)
    assert!(predictor.is_emergent(10, 35));

    // Brand new key with initial low volume: 0 -> 5
    assert!(!predictor.is_emergent(0, 5));

    // Brand new key surging to 20 requests
    assert!(predictor.is_emergent(0, 20));
}

// ----------------------------------------------------------------------------
// 4. COMPUTED held-out candidate comparisons (M5-12 ML gate, M5-13 dispositions)
//
// Replaces the former literal `DispositionRecord`s (hard-coded effect sizes and CIs) and the ML-gate
// test that asserted nothing about a comparison. Every effect below is computed in this file from
// executed simulations / policy replays on DEV seeds D (20001..) and HELD-OUT families, with
// paired-by-seed differences, a BCa 95% CI and a sign-flip permutation p-value. Tuning seeds T are
// used only to fit the ML stump. Tier: SIMULATED (T1) / replay of generated workloads.
//
// Disposition rule (docs/research/phase_5_preregistration.md section 5), applied mechanically to the
// computed CIs: Retain-statistical if CI_lower > delta_min in ALL held-out families; Narrow if in
// SOME; Reject if in NONE. The overhead budget (CPU <= 2%, hot-path adder <= 100 ns) is UNMEASURED
// here, so no candidate can be finally "Retained" by this file: the strongest outcome is
// "Retain-statistical, overhead UNVERIFIED".
// ----------------------------------------------------------------------------

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::analysis::StatisticalEvaluator;
use lab::eval::{analyze_endpoints, plain_seeds, read_cells_csv, repo_root, run_matrix, MatrixSpec, Metric};
use lab::rng::{MasterRng, SeedSets};
use lab::t1_sim::*;
use phantom_cache_demo::intelligence::shadow::{LruShadow, S3FifoShadow, ShadowPolicy, SieveShadow, TinyLfuShadow};

const DELTA_MIN: f64 = 0.05;
const STREAM_N: usize = 12_000;
const PREFIX: usize = STREAM_N / 4;
const CAP: usize = 64;

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn key_stream(family: &str, seed: u64, n: usize) -> Vec<u64> {
    let p = SimParams::standard();
    let mut wl = make_workload(family, &p);
    let mut rng = MasterRng::new(seed).stream("workload");
    let mut t = 0u64;
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let r = wl.next_request(t, &mut rng);
        t += r.arrival_delta_us;
        if !r.is_invalidation {
            out.push(fnv(&r.key));
        }
    }
    out
}

fn make_policy(name: &str) -> Box<dyn ShadowPolicy> {
    match name {
        "LRU" => Box::new(LruShadow::new(CAP)),
        "SIEVE" => Box::new(SieveShadow::new(CAP)),
        "S3-FIFO" => Box::new(S3FifoShadow::new(CAP)),
        "TinyLFU" => Box::new(TinyLfuShadow::new(CAP)),
        other => panic!("unknown policy {other}"),
    }
}

/// Hit ratio over keys[from..] with the policy warmed on keys[..from].
fn suffix_hit_ratio(name: &str, keys: &[u64], from: usize) -> f64 {
    let mut p = make_policy(name);
    let mut hits = 0usize;
    for (i, &k) in keys.iter().enumerate() {
        let h = p.access(k);
        if i >= from && h {
            hits += 1;
        }
    }
    hits as f64 / (keys.len() - from) as f64
}

/// Non-ML statistical selection: policy with the best hit ratio on the observed prefix.
fn shadow_select(keys: &[u64], options: &[&'static str]) -> &'static str {
    let mut best = options[0];
    let mut best_hr = -1.0;
    for &o in options {
        let mut p = make_policy(o);
        let mut hits = 0usize;
        for &k in &keys[..PREFIX] {
            if p.access(k) {
                hits += 1;
            }
        }
        let hr = hits as f64 / PREFIX as f64;
        if hr > best_hr {
            best_hr = hr;
            best = o;
        }
    }
    best
}

fn unique_ratio(keys: &[u64]) -> f64 {
    let set: std::collections::HashSet<&u64> = keys[..PREFIX].iter().collect();
    set.len() as f64 / PREFIX as f64
}

#[derive(Debug, Clone)]
struct PairedResult {
    candidate: &'static str,
    baseline: &'static str,
    metric: &'static str,
    family: String,
    n: usize,
    mean_effect: f64,
    ci_lo: f64,
    ci_hi: f64,
    p_value: f64,
    diffs: Vec<f64>,
}

fn paired(candidate: &'static str, baseline: &'static str, metric: &'static str, family: &str, diffs: Vec<f64>, seed: u64) -> PairedResult {
    let n = diffs.len();
    let mean = diffs.iter().sum::<f64>() / n.max(1) as f64;
    let (lo, hi) = StatisticalEvaluator::bca_ci_mean(&diffs, 10_000, seed);
    let p = StatisticalEvaluator::sign_flip_p_value(&diffs, seed ^ 0xabc);
    PairedResult { candidate, baseline, metric, family: family.to_string(), n, mean_effect: mean, ci_lo: lo, ci_hi: hi, p_value: p, diffs }
}

/// Mechanical application of the pre-registered rule to computed CIs.
fn statistical_disposition(rs: &[PairedResult]) -> (CandidateDisposition, usize) {
    let pass = rs.iter().filter(|r| r.ci_lo > DELTA_MIN).count();
    let d = if pass == rs.len() && !rs.is_empty() {
        CandidateDisposition::Retain
    } else if pass > 0 {
        CandidateDisposition::Narrow
    } else {
        CandidateDisposition::Reject
    };
    (d, pass)
}

fn dev30() -> Vec<u64> {
    SeedSets::dev_eval_seeds()
}

const HELD_OUT: [&str; 4] = ["W-DI", "W-SH", "W-MX", "W-HK"];

fn cand1_shadow_selected_eviction() -> Vec<PairedResult> {
    HELD_OUT
        .iter()
        .map(|fam| {
            let diffs: Vec<f64> = dev30()
                .iter()
                .map(|&s| {
                    let keys = key_stream(fam, s, STREAM_N);
                    let sel = shadow_select(&keys, &["LRU", "SIEVE", "S3-FIFO"]);
                    suffix_hit_ratio(sel, &keys, PREFIX) - suffix_hit_ratio("SIEVE", &keys, PREFIX)
                })
                .collect();
            paired("CAND-1 Shadow-Selected Eviction", "static SIEVE (product default)", "suffix hit ratio (absolute)", fam, diffs, 101)
        })
        .collect()
}

fn cand5_rate_forecaster() -> Vec<PairedResult> {
    HELD_OUT
        .iter()
        .map(|fam| {
            let p = SimParams::standard();
            let diffs: Vec<f64> = dev30()
                .iter()
                .map(|&s| {
                    let mut wl = make_workload(fam, &p);
                    let mut rng = MasterRng::new(s).stream("workload");
                    let mut t = 0u64;
                    let mut counts = vec![0.0f64; 60];
                    loop {
                        let r = wl.next_request(t, &mut rng);
                        t += r.arrival_delta_us;
                        let sec = (t / 1_000_000) as usize;
                        if sec >= counts.len() {
                            break;
                        }
                        counts[sec] += 1.0;
                    }
                    let mut f = PredictiveRateForecaster::new(0.5);
                    for c in &counts {
                        f.record_and_forecast(*c);
                    }
                    1.0 - f.compute_mase() // > 0 means better than naive persistence
                })
                .collect();
            paired("CAND-5 Standalone Rate Forecaster (EWMA a=0.5)", "naive persistence", "1 - MASE (per-second request rate)", fam, diffs, 105)
        })
        .collect()
}

/// Stump trained on tuning seeds T / tuning families only; feature = unique-key ratio of the prefix.
fn train_stump() -> DecisionStumpSelector {
    let samples: Vec<(f64, f64, f64)> = SeedSets::tuning_seeds()[..20]
        .iter()
        .flat_map(|&s| {
            ["W-Z", "W-FC", "W-SC"].iter().map(move |fam| {
                let keys = key_stream(fam, s, STREAM_N);
                (unique_ratio(&keys), suffix_hit_ratio("SIEVE", &keys, PREFIX), suffix_hit_ratio("S3-FIFO", &keys, PREFIX))
            })
        })
        .collect();
    let mut best: Option<(f64, f64, bool)> = None; // (score, threshold, low_means_sieve)
    for k in 1..20 {
        let thr = k as f64 * 0.05;
        for low_sieve in [true, false] {
            let score: f64 = samples
                .iter()
                .map(|&(u, hs, h3)| {
                    let pick_sieve = (u < thr) == low_sieve;
                    if pick_sieve { hs } else { h3 }
                })
                .sum::<f64>()
                / samples.len() as f64;
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, thr, low_sieve));
            }
        }
    }
    let (_, thr, low_sieve) = best.unwrap();
    if low_sieve {
        DecisionStumpSelector::new(0, thr, "SIEVE", "S3-FIFO")
    } else {
        DecisionStumpSelector::new(0, thr, "S3-FIFO", "SIEVE")
    }
}

fn cand6_ml_gate() -> Vec<PairedResult> {
    let stump = train_stump();
    HELD_OUT
        .iter()
        .map(|fam| {
            let diffs: Vec<f64> = dev30()
                .iter()
                .map(|&s| {
                    let keys = key_stream(fam, s, STREAM_N);
                    let ml = stump.predict(&[unique_ratio(&keys)]);
                    let nonml = shadow_select(&keys, &["SIEVE", "S3-FIFO"]);
                    suffix_hit_ratio(ml, &keys, PREFIX) - suffix_hit_ratio(nonml, &keys, PREFIX)
                })
                .collect();
            paired("CAND-6 ML Policy Selector (decision stump)", "non-ML shadow selection (SIEVE vs S3-FIFO)", "suffix hit ratio (absolute)", fam, diffs, 106)
        })
        .collect()
}

/// Runs two arms on the T1 runner over dev seeds and held-out families (raw data written to
/// lab-results/candidates_m5/ and re-read from disk); returns per-family paired results.
#[allow(clippy::too_many_arguments)]
fn runner_candidate(
    cand: &'static str,
    base: &'static str,
    metric_name: &'static str,
    treat_arm: &str,
    ctrl_arm: &str,
    scenarios: &[&str],
    metric: fn(&CellSummary) -> f64,
    relative: bool,
    seed: u64,
) -> Vec<PairedResult> {
    let spec = MatrixSpec {
        name: format!("cand_{}", treat_arm),
        seed_set_name: "D".to_string(),
        scenarios: scenarios.iter().map(|s| s.to_string()).collect(),
        families: HELD_OUT.iter().map(|s| s.to_string()).collect(),
        arms: vec![treat_arm.to_string(), ctrl_arm.to_string()],
        seeds: plain_seeds(&dev30()[..8]),
        params: SimParams::smoke(),
        arm_params: ArmParams::untuned_defaults(),
        record_raw: false,
    };
    let dir = repo_root().join("lab-results").join("candidates_m5").join(&spec.name);
    run_matrix(&spec, &dir).expect("matrix");
    let rows = read_cells_csv(&dir.join("cells.csv")).expect("read raw");
    HELD_OUT
        .iter()
        .map(|fam| {
            let mut per_seed: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
            for r in rows.iter().filter(|r| r.family == *fam) {
                let e = per_seed.entry(r.seed_label.clone()).or_insert((0.0, 0.0));
                if r.arm == treat_arm {
                    e.0 += metric(r);
                } else if r.arm == ctrl_arm {
                    e.1 += metric(r);
                }
            }
            let k = scenarios.len() as f64;
            let diffs: Vec<f64> = per_seed
                .values()
                .map(|&(t, c)| {
                    let (t, c) = (t / k, c / k);
                    if relative {
                        if c.abs() < 1e-12 { 0.0 } else { (c - t) / c }
                    } else {
                        // positive = candidate better (the metric passed in encodes the direction)
                        t - c
                    }
                })
                .collect();
            paired(cand, base, metric_name, fam, diffs, seed)
        })
        .collect()
}

fn cand2_admission() -> Vec<PairedResult> {
    runner_candidate("CAND-2 TinyLFU-style Frequency Admission", "single-flight without admission filter", "fresh hit ratio (absolute)", "CAND-ADM", "CAND-NOADM", &["O-NONE"], |c| c.fresh_hit_ratio, false, 102)
}

fn cand3_dynamic_stale() -> Vec<PairedResult> {
    // effect = reduction in client failure rate (control - candidate) => metric is the negated error rate.
    runner_candidate("CAND-3 Dynamic Stale Extension", "static stale window (same base)", "reduction in client failure rate (absolute)", "CAND-DYN-STALE", "CAND-STATIC-STALE", &["O-BRN", "O-SLW"], |c| -c.error_rate, false, 103)
}

fn cand4_aimd_cap() -> Vec<PairedResult> {
    runner_candidate("CAND-4 AIMD Origin Concurrency Cap", "static concurrency cap 128", "relative reduction in peak origin concurrency", "CAND-AIMD", "CAND-STATIC-CAP128", &["O-DEG", "O-GRD"], |c| c.peak_outstanding as f64, true, 104)
}

fn report(all: &[(&str, Vec<PairedResult>)]) -> serde_json::Value {
    let mut out = Vec::new();
    for (id, rs) in all {
        let (d, pass) = statistical_disposition(rs);
        out.push(serde_json::json!({
            "candidate": id,
            "tier": "SIMULATED / generated-workload replay",
            "seed_set": "D (dev), held-out families only; tuning seeds T used only to fit the ML stump",
            "delta_min": DELTA_MIN,
            "statistical_disposition": format!("{:?}", d),
            "families_passing_ci_lower_gt_delta_min": pass,
            "overhead_budget": "UNVERIFIED (not measured here)",
            "final_disposition": if d == CandidateDisposition::Reject { "Reject".to_string() } else { format!("{:?}-statistical; overhead UNVERIFIED => not final", d) },
            "per_family": rs.iter().map(|r| serde_json::json!({
                "family": r.family, "baseline": r.baseline, "metric": r.metric, "n_seeds": r.n,
                "mean_effect": r.mean_effect, "bca95": [r.ci_lo, r.ci_hi], "p_sign_flip": r.p_value,
                "per_seed_diffs": r.diffs,
            })).collect::<Vec<_>>(),
        }));
    }
    serde_json::Value::Array(out)
}

#[test]
fn test_computed_candidate_comparisons_and_dispositions() {
    let all: Vec<(&str, Vec<PairedResult>)> = vec![
        ("CAND-1", cand1_shadow_selected_eviction()),
        ("CAND-2", cand2_admission()),
        ("CAND-3", cand3_dynamic_stale()),
        ("CAND-4", cand4_aimd_cap()),
        ("CAND-5", cand5_rate_forecaster()),
        ("CAND-6", cand6_ml_gate()),
    ];
    let rep = report(&all);
    let dir = repo_root().join("lab-results").join("candidates_m5");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dispositions_computed.json"), serde_json::to_string_pretty(&rep).unwrap()).unwrap();

    for (id, rs) in &all {
        assert_eq!(rs.len(), 4, "{id}: one comparison per held-out family");
        let (d, pass) = statistical_disposition(rs);
        for r in rs {
            assert!(r.n >= 8, "{id}/{}: too few paired seeds", r.family);
            assert!(r.mean_effect.is_finite() && r.ci_lo.is_finite() && r.ci_hi.is_finite());
            assert!(r.ci_lo <= r.ci_hi);
            assert_eq!(r.diffs.len(), r.n);
            println!(
                "COMPUTED(SIMULATED,D) {id} {:5} {} vs {}: effect={:+.4} BCa95=[{:+.4},{:+.4}] p={:.4} (n={})",
                r.family, r.candidate, r.baseline, r.mean_effect, r.ci_lo, r.ci_hi, r.p_value, r.n
            );
        }
        // Disposition is a pure function of the computed CIs (no manual override).
        let recomputed = rs.iter().filter(|r| r.ci_lo > DELTA_MIN).count();
        assert_eq!(recomputed, pass);
        println!("COMPUTED {id}: statistical disposition = {d:?} ({pass}/4 held-out families with CI_lower > {DELTA_MIN}); overhead UNVERIFIED");
    }
}

#[test]
fn test_ml_gate_is_a_computed_comparison_against_the_non_ml_selector() {
    let stump = train_stump();
    // The stump was fitted on tuning seeds only; evaluate on dev seeds / held-out families.
    let rs = cand6_ml_gate();
    assert_eq!(rs.len(), 4);
    for r in &rs {
        assert_eq!(r.n, 30);
        println!(
            "ML-GATE(SIMULATED,D) {} split={:.2} ML-minus-nonML hit ratio {:+.4} BCa95=[{:+.4},{:+.4}]",
            r.family, stump.split_value, r.mean_effect, r.ci_lo, r.ci_hi
        );
    }
    // Gate rule: the ML candidate passes only if it beats the non-ML selector by delta_min (CI lower bound).
    let (d, _) = statistical_disposition(&rs);
    println!("ML GATE VERDICT (computed): passes={}", d != CandidateDisposition::Reject);
}

#[test]
fn test_legacy_pie_is_dispositioned_by_code_fact_not_by_a_literal_effect() {
    // CAND-7: no effect size is claimed. The structural finding (hard-coded expected utility) is verified in source.
    let src = std::fs::read_to_string(repo_root().join("src/control/pie.rs")).expect("read pie.rs");
    let line = src.lines().enumerate().find(|(_, l)| l.contains("85.0") && l.contains("expected_utility"));
    let (n, l) = line.expect("hard-coded expected_utility = 85.0 no longer found in src/control/pie.rs (re-audit CAND-7)");
    println!("CAND-7 structural fact (verified in code): src/control/pie.rs:{} {}", n + 1, l.trim());
}

#[test]
fn test_no_literal_effect_sizes_remain_in_this_file() {
    let text = std::fs::read_to_string(repo_root().join("tests/integration/intel_candidates_tests.rs")).unwrap();
    let a = format!("{}{}", "effect_estimate", ": 0.");
    let b = format!("{}{}", "ci_95", ": (");
    let c = format!("{}{}", "Disposition", "Record {");
    assert!(!text.contains(&a));
    assert!(!text.contains(&b));
    assert!(!text.contains(&c));
}

#[test]
fn test_runner_candidate_helper_uses_raw_data_from_disk() {
    let rs = cand2_admission();
    let dir = repo_root().join("lab-results/candidates_m5/cand_CAND-ADM");
    assert!(dir.join("cells.csv").exists() && dir.join("manifest.json").exists());
    let rows = read_cells_csv(&dir.join("cells.csv")).unwrap();
    let m = Metric { name: "fresh_hit_ratio", f: |c| c.fresh_hit_ratio };
    let res = analyze_endpoints(&rows, "CAND-ADM", "CAND-NOADM", &["O-NONE".to_string()], &["W-SH".to_string()], &[m], DELTA_MIN, 1);
    assert_eq!(res.len(), 1);
    assert_eq!(rs.len(), 4);
}
