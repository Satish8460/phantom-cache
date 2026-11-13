//! The M6 evaluation pipeline of protocol v2.2: ONE function shared by the sealed evaluation and the
//! development-seed dry run, so that the dry run executes the identical matrix, analysis and verdict code.
//!
//! The pipeline (a) runs the full matrix (8 scenarios x 5 families x 10 arms x N seeds) with `SimParams::standard()`,
//! (b) re-reads the RAW `cells.csv` from disk, (c) runs the completeness checks, (d) writes every analysis file
//! (all labelled tier=SIMULATED with a banner) and (e) applies the pre-registered verdict rule MECHANICALLY and writes
//! `verdict.json`. Nothing here depends on the seed values; sealed seeds are labelled S00.. and never written.
//!
//! Every number is tier SIMULATED: a deterministic virtual-time simulation driving the real cache / executor code. It is
//! not a measurement of any network, host or process.

use super::eval::*;
use super::t1_sim::*;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub const BANNER: &str = "SIMULATION RESULT (T1 deterministic virtual-time simulation driving the real cache/executor code). NOT A MEASUREMENT of any network, host or process.";
/// Fixed bootstrap / permutation base seed of the registered analysis.
pub const ANALYSIS_BOOT_SEED: u64 = 20_261_029;
/// Registered minimum practical effect (mean relative reduction).
pub const DELTA_MIN: f64 = 0.05;
/// Registered number of seeds for H1 (sealed set size).
pub const REGISTERED_SEEDS: usize = 30;

pub fn pipeline_arms() -> Vec<String> {
    ["BL-0", "BL-1", "BL-4", "BL-5", "BL-6", "BL-12b", "PIE-Retained", "PIE-no-stale", "PIE-no-cap", "PIE-evict-only"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn pipeline_scenarios() -> Vec<String> {
    M6_SCENARIOS.iter().map(|s| s.to_string()).collect()
}

pub fn pipeline_families() -> Vec<String> {
    let mut f: Vec<String> = HELD_OUT_FAMILIES.iter().map(|s| s.to_string()).collect();
    f.push(SEALED_FAMILY.to_string());
    f
}

pub fn expected_cell_count(n_seeds: usize) -> usize {
    n_seeds * pipeline_scenarios().len() * pipeline_families().len() * pipeline_arms().len()
}

pub struct PipelineConfig {
    pub name: String,
    /// "D" (dry run) or "S" (sealed: numeric seeds are never written).
    pub seed_set_name: String,
    pub seeds: Vec<SeedSpec>,
    pub out_dir: PathBuf,
    pub arm_params: ArmParams,
    pub record_raw: bool,
    /// Print progress to stderr every this many cells (0 = never).
    pub progress_every: usize,
}

pub struct PipelineOutcome {
    pub out_dir: PathBuf,
    pub cells: usize,
    pub expected_cells: usize,
    pub runtime_secs: f64,
    /// Completeness / validity problems. Empty means the run is complete and clean.
    pub problems: Vec<String>,
    pub verdict: serde_json::Value,
}

fn csv_banner(role: &str) -> String {
    format!("# tier=SIMULATED\n# banner={BANNER}\n# role={role}\n")
}

const ENDPOINT_HEADER: &str = "scenario,endpoint,treat,ctrl,n_seeds,mean_ctrl,mean_treat,mean_rel_reduction,ci_lo,ci_hi,p,p_holm,supported,practically_null,adverse,EXPLORATORY_log_ratio_rel_reduction,EXPLORATORY_median_rel_reduction,n_ratio_abs_gt_10";

fn endpoint_csv(role: &str, res: &[EndpointResult]) -> String {
    let mut s = csv_banner(role);
    s.push_str(ENDPOINT_HEADER);
    s.push('\n');
    for r in res {
        s.push_str(&format!(
            "{},{},{},{},{},{:.4},{:.4},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{:.6},{:.6},{}\n",
            r.scenario, r.endpoint, r.treat, r.ctrl, r.n_seeds, r.mean_ctrl, r.mean_treat, r.mean_rel_reduction, r.ci_lo, r.ci_hi, r.p_value,
            r.p_holm, r.supported, r.practically_null, r.adverse, r.expl_log_ratio_rel_reduction, r.expl_median_rel_reduction, r.n_extreme_ratio
        ));
    }
    s
}

fn endpoint_json(r: &EndpointResult) -> serde_json::Value {
    json!({
        "scenario": r.scenario, "endpoint": r.endpoint, "n_seeds": r.n_seeds,
        "mean_ctrl": r.mean_ctrl, "mean_treat": r.mean_treat, "mean_rel_reduction": r.mean_rel_reduction,
        "ci_lo": r.ci_lo, "ci_hi": r.ci_hi, "p": r.p_value, "p_holm": r.p_holm,
        "supported": r.supported, "practically_null": r.practically_null, "adverse": r.adverse,
    })
}

/// Applies the pre-registered H1 decision rule (v2.2 amendment, section H3) MECHANICALLY to the raw cells.
/// No judgement is involved; the returned JSON records every input to the rule.
pub fn compute_verdict(rows: &[CellSummary], scenarios: &[String], families: &[String], n_seeds_expected: usize) -> serde_json::Value {
    let primary = analyze_endpoints(rows, "PIE-Retained", "BL-12b", scenarios, families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let vs_bl6 = analyze_endpoints(rows, "PIE-Retained", "BL-6", scenarios, families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let vs_bl1 = analyze_endpoints(rows, "PIE-Retained", "BL-1", scenarios, families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let cons = check_constraints(rows, "PIE-Retained", "BL-12b", scenarios, families);

    let count_supported = |endpoint: &str| primary.iter().filter(|r| r.endpoint == endpoint && r.supported).count();
    let any_adverse = |endpoint: &str| primary.iter().any(|r| r.endpoint == endpoint && r.adverse);
    let n_peak = count_supported(M_PEAK.name);
    let n_p99 = count_supported(M_P99.name);
    let adverse_peak = any_adverse(M_PEAK.name);
    let adverse_p99 = any_adverse(M_P99.name);
    let fresh_ok_all = cons.iter().all(|c| c.hit_ratio_ok);
    let error_ok_all = cons.iter().all(|c| c.error_rate_ok);
    let n_seeds_ok = primary.iter().all(|r| r.n_seeds == n_seeds_expected);

    // Falsifiers of preregistration_m6_v2.md section 4.6, operationalised.
    // F-a: for >= 6 of 8 scenarios the mean r_i of PIE-Retained is below 0.10 on BOTH primary endpoints, versus BL-12b or versus BL-6.
    let f_a_count = |res: &[EndpointResult]| {
        scenarios
            .iter()
            .filter(|sc| res.iter().filter(|r| &r.scenario == *sc).all(|r| r.mean_rel_reduction < 0.10) && res.iter().any(|r| &r.scenario == *sc))
            .count()
    };
    let (fa12, fa6) = (f_a_count(&primary), f_a_count(&vs_bl6));
    let f_a = fa12 >= 6 || fa6 >= 6;
    // F-c: PIE-Retained's mean client P99 exceeds BL-1's by more than 5% in any scenario.
    let f_c_scen: Vec<serde_json::Value> = vs_bl1
        .iter()
        .filter(|r| r.endpoint == M_P99.name)
        .filter(|r| r.mean_ctrl > 0.0 && (r.mean_treat - r.mean_ctrl) / r.mean_ctrl > 0.05)
        .map(|r| json!({"scenario": r.scenario, "mean_pie_p99_us": r.mean_treat, "mean_bl1_p99_us": r.mean_ctrl}))
        .collect();
    let f_c = !f_c_scen.is_empty();
    // F-d: any cap or stale-cap violation in any PIE-Retained cell.
    let (mut cap_v, mut stale_v) = (0u64, 0u64);
    for r in rows.iter().filter(|r| r.arm == "PIE-Retained") {
        cap_v += r.cap_violations;
        stale_v += r.stale_cap_violations_counter + r.stale_cap_violations_indep;
    }
    let f_d = cap_v + stale_v > 0;
    // F-e: every one of the 16 primary hypotheses is practically null or adverse (BCa upper bound < delta_min).
    let f_e = !primary.is_empty() && primary.iter().all(|r| r.practically_null);

    let fewer_than_3 = n_peak.min(n_p99) < 3;
    let supported_rule = n_peak >= 5 && n_p99 >= 5 && !adverse_peak && !adverse_p99 && fresh_ok_all && error_ok_all;
    let not_supported = fewer_than_3 || f_a || f_c || f_d || f_e;
    let h1 = if !n_seeds_ok {
        "INVALID_RUN_incomplete_seed_pairing"
    } else if not_supported {
        "NOT_SUPPORTED"
    } else if supported_rule {
        "SUPPORTED"
    } else {
        "MIXED_INCONCLUSIVE"
    };

    let (same, total) = arms_cell_for_cell_identical(rows, "PIE-no-stale", "PIE-Retained");
    let inert = total > 0 && same == total;
    let h2_json = if inert {
        json!({"status": "NOT_TESTABLE_AT_REGISTERED_HORIZON", "reason": "dynamic stale window inert: PIE-no-stale executes cell-for-cell identically to PIE-Retained", "identical_cells": same, "compared_cells": total})
    } else {
        json!({"status": "COMPUTED_SEE_analysis_h2_h4.csv", "identical_cells": same, "compared_cells": total})
    };
    let h4_json = if inert {
        json!({"status": "NOT_TESTABLE_AT_REGISTERED_HORIZON", "reason": "the stale window never binds at the registered horizon"})
    } else {
        json!({"status": "COMPUTED_SEE_analysis_h2_h4.csv"})
    };
    let per_scenario: Vec<serde_json::Value> = scenarios
        .iter()
        .map(|sc| {
            let c = cons.iter().find(|c| &c.scenario == sc);
            json!({
                "scenario": sc,
                "peak": primary.iter().find(|r| &r.scenario == sc && r.endpoint == M_PEAK.name).map(endpoint_json),
                "p99": primary.iter().find(|r| &r.scenario == sc && r.endpoint == M_P99.name).map(endpoint_json),
                "d_fresh_hit_ratio": c.map(|c| c.mean_fresh_hit_ratio_diff),
                "d_error_rate": c.map(|c| c.mean_error_rate_diff),
                "fresh_hit_ok": c.map(|c| c.hit_ratio_ok),
                "error_rate_ok": c.map(|c| c.error_rate_ok),
            })
        })
        .collect();
    json!({
        "tier": "SIMULATED",
        "banner": BANNER,
        "protocol_version": super::rng::PROTOCOL_VERSION,
        "rule_source": "docs/research/preregistration_m6_v2_2_amendment.md section H3 (applied mechanically; no judgement after seeing data)",
        "H1": {
            "verdict": h1,
            "supported_scenarios_peak_concurrency": n_peak,
            "supported_scenarios_client_p99": n_p99,
            "any_adverse_ci_peak": adverse_peak,
            "any_adverse_ci_p99": adverse_p99,
            "fresh_hit_constraint_holds_in_all_scenarios": fresh_ok_all,
            "error_rate_constraint_holds_in_all_scenarios": error_ok_all,
            "fewer_than_3_scenarios_supported_on_an_endpoint": fewer_than_3,
            "supported_rule_met": supported_rule,
            "not_supported_triggered": not_supported,
            "n_seeds_paired_in_every_hypothesis": n_seeds_ok,
        },
        "falsifiers": {
            "F-a": {"fires": f_a, "scenarios_below_0.10_vs_BL-12b": fa12, "scenarios_below_0.10_vs_BL-6": fa6, "threshold_scenarios": 6},
            "F-b": {"fires": null, "note": "reported from O-HET and O-GRD rows of analysis_primary.csv; not mechanical"},
            "F-c": {"fires": f_c, "scenarios_where_PIE_P99_exceeds_BL-1_by_more_than_5pct": f_c_scen},
            "F-d": {"fires": f_d, "pie_retained_cap_violations": cap_v, "pie_retained_stale_cap_violations": stale_v},
            "F-e": {"fires": f_e, "rule": "all 16 primary hypotheses practically null or adverse (BCa upper bound < 0.05)"},
            "F-f": {"fires": null, "note": "prior-art overlap is outside this protocol"},
        },
        "H2": h2_json,
        "H4": h4_json,
        "H3": {"status": "UNVERIFIED", "note": "overhead is not measurable under T1"},
        "H5_H6": {"status": "UNVERIFIED", "note": "not evaluated by this protocol"},
        "per_scenario": per_scenario,
    })
}

fn has_nan(x: f64) -> bool {
    !x.is_finite()
}

/// Completeness / validity checks on the raw rows and the analysis results. Returns human-readable problems.
pub fn completeness_problems(rows: &[CellSummary], expected_cells: usize, n_seeds: usize, arm_params: &ArmParams, results: &[&Vec<EndpointResult>]) -> Vec<String> {
    let mut p = Vec::new();
    if rows.len() != expected_cells {
        p.push(format!("row count {} != expected {}", rows.len(), expected_cells));
    }
    let mut ids: Vec<&str> = rows.iter().map(|r| r.cell_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != rows.len() {
        p.push(format!("duplicate cell ids: {} unique of {}", ids.len(), rows.len()));
    }
    let bad_audit = rows.iter().filter(|r| !r.bytes_audit_equal).count();
    if bad_audit > 0 {
        p.push(format!("{bad_audit} cells with bytes_audit_equal=false"));
    }
    let inv: u64 = rows.iter().map(|r| r.invariant_violations).sum();
    if inv > 0 {
        p.push(format!("{inv} simulator invariant violations / panics recorded in cells"));
    }
    if rows.iter().any(|r| r.tier != "SIMULATED") {
        p.push("a cell is not labelled tier=SIMULATED".to_string());
    }
    let nan_cells = rows.iter().filter(|r| has_nan(r.fresh_hit_ratio) || has_nan(r.served_from_cache_ratio) || has_nan(r.error_rate) || has_nan(r.mean_stale_age_ms)).count();
    if nan_cells > 0 {
        p.push(format!("{nan_cells} cells contain NaN/inf"));
    }
    let missing = bl6_missing_scenarios(arm_params, &pipeline_scenarios());
    if !missing.is_empty() {
        p.push(format!("BL-6 has no tuned entry for scenarios {missing:?} (it would silently equal BL-12b)"));
    }
    for res in results {
        for r in res.iter() {
            if r.n_seeds != n_seeds {
                p.push(format!("hypothesis {} {} vs {} has n_seeds={} (expected {})", r.scenario, r.endpoint, r.ctrl, r.n_seeds, n_seeds));
            }
            if has_nan(r.mean_rel_reduction) || has_nan(r.ci_lo) || has_nan(r.ci_hi) || has_nan(r.p_value) || has_nan(r.p_holm) || has_nan(r.mean_ctrl) || has_nan(r.mean_treat) {
                p.push(format!("NaN/inf in hypothesis {} {} {} vs {}", r.scenario, r.endpoint, r.treat, r.ctrl));
            }
        }
    }
    p
}

/// Runs the whole pipeline. Always writes `cells.csv`, `manifest.json` and all analysis files BEFORE reporting problems;
/// problems are returned (not panicked on) so the caller can fail loudly after the data are recorded.
pub fn run_pipeline(cfg: &PipelineConfig) -> Result<PipelineOutcome, String> {
    let started = std::time::Instant::now();
    let scenarios = pipeline_scenarios();
    let families = pipeline_families();
    let n_seeds = cfg.seeds.len();
    let expected = expected_cell_count(n_seeds);
    let spec = MatrixSpec {
        name: cfg.name.clone(),
        seed_set_name: cfg.seed_set_name.clone(),
        scenarios: scenarios.clone(),
        families: families.clone(),
        arms: pipeline_arms(),
        seeds: cfg.seeds.clone(),
        params: SimParams::standard(),
        arm_params: cfg.arm_params.clone(),
        record_raw: cfg.record_raw,
    };
    let every = cfg.progress_every;
    let mut progress = |done: usize, total: usize| {
        if every > 0 && (done.is_multiple_of(every) || done == total) {
            eprintln!("[m6 {}] {done}/{total} cells, {:.0} s", cfg.name, started.elapsed().as_secs_f64());
        }
    };
    run_matrix_with_progress(&spec, &cfg.out_dir, &mut progress)?;
    let dir: &Path = &cfg.out_dir;

    // Analysis consumes the RAW file from disk.
    let rows = read_cells_csv(&dir.join("cells.csv"))?;
    let primary = analyze_endpoints(&rows, "PIE-Retained", "BL-12b", &scenarios, &families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let secondary = analyze_endpoints(&rows, "PIE-Retained", "BL-12b", &scenarios, &families, &[M_P99_SUCCESS], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let vs_bl6 = analyze_endpoints(&rows, "PIE-Retained", "BL-6", &scenarios, &families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let vs_bl1 = analyze_endpoints(&rows, "PIE-Retained", "BL-1", &scenarios, &families, &[M_PEAK, M_P99], DELTA_MIN, ANALYSIS_BOOT_SEED);
    let constraints = check_constraints(&rows, "PIE-Retained", "BL-12b", &scenarios, &families);

    let w = |name: &str, body: String| fs::write(dir.join(name), body).map_err(|e| format!("{name}: {e}"));
    w("analysis_primary.csv", endpoint_csv("PRIMARY pre-registered: PIE-Retained vs BL-12b; primary P99 includes failed requests at their failure latency (shedding can look like a P99 win; see error-rate constraint)", &primary))?;
    w("analysis_secondary_p99_success_only.csv", endpoint_csv("SECONDARY (not in the verdict): success-only P99, PIE-Retained vs BL-12b", &secondary))?;
    w("analysis_vs_bl6.csv", endpoint_csv("falsifier F-a input: PIE-Retained vs BL-6 (per-scenario oracle static)", &vs_bl6))?;
    w("analysis_vs_bl1.csv", endpoint_csv("falsifier F-c input: PIE-Retained vs BL-1 (single-flight only)", &vs_bl1))?;
    let mut cons = csv_banner("constraints, PIE-Retained vs BL-12b (fresh-hit loss <= 1 pp, error-rate increase <= 0.5 pp, zero cap/stale-cap violations)");
    cons.push_str("scenario,d_fresh_hit_ratio,d_error_rate,cap_violations,stale_cap_violations,hit_ratio_ok,error_rate_ok\n");
    for c in &constraints {
        cons.push_str(&format!("{},{:.6},{:.6},{},{},{},{}\n", c.scenario, c.mean_fresh_hit_ratio_diff, c.mean_error_rate_diff, c.treat_cap_violations, c.treat_stale_cap_violations, c.hit_ratio_ok, c.error_rate_ok));
    }
    w("analysis_constraints.csv", cons)?;

    // H2 / H4: not testable when the dynamic stale window is inert.
    let (same, total) = arms_cell_for_cell_identical(&rows, "PIE-no-stale", "PIE-Retained");
    let inert = total > 0 && same == total;
    let mut h = csv_banner("H2 attribution and H4 stale-window stability");
    h.push_str("item,status,detail\n");
    if inert {
        let msg = format!("NOT_TESTABLE_AT_REGISTERED_HORIZON,dynamic stale window inert: PIE-no-stale is cell-for-cell identical to PIE-Retained in {same}/{total} cells (the stale window in force never binds within the 28 s run); no attribution number is computed");
        h.push_str(&format!("H2,{msg}\n"));
        h.push_str("H4,NOT_TESTABLE_AT_REGISTERED_HORIZON,the stale window never binds at the registered horizon; the window-change count below is controller bookkeeping only\n");
        h.push_str(&format!("H4_bookkeeping_only,stale_window_changes_p95,{}\n", h4_stale_window_changes_p95(&rows, "PIE-Retained")));
    } else {
        h.push_str(&format!("H2_precondition,PIE-no-stale differs from PIE-Retained in {} of {} cells; the numbers below are EXPLORATORY (v2.2 does not register H2/H4 as tests at this horizon),\n", total - same, total));
        for m in [M_PEAK, M_P99] {
            match h2_origin_mechanism_share(&rows, "BL-12b", "PIE-Retained", "PIE-evict-only", &scenarios, &families, &m, 99) {
                Some((s, lo, hi)) => h.push_str(&format!("H2,EXPLORATORY_COMPUTED,{} share_origin_mechanisms={s:.6} [{lo:.6};{hi:.6}]\n", m.name)),
                None => h.push_str(&format!("H2,NOT_APPLICABLE_no_gain_to_attribute,{}\n", m.name)),
            }
        }
        h.push_str(&format!("H4,EXPLORATORY_COMPUTED,stale_window_changes_p95={}\n", h4_stale_window_changes_p95(&rows, "PIE-Retained")));
    }
    w("analysis_h2_h4.csv", h)?;

    let verdict = compute_verdict(&rows, &scenarios, &families, n_seeds);
    let problems = completeness_problems(&rows, expected, n_seeds, &cfg.arm_params, &[&primary, &secondary, &vs_bl6, &vs_bl1]);
    let mut vfile = verdict.clone();
    if let Some(o) = vfile.as_object_mut() {
        o.insert("run_name".into(), json!(cfg.name));
        o.insert("seed_set".into(), json!(cfg.seed_set_name));
        o.insert("cells".into(), json!(rows.len()));
        o.insert("expected_cells".into(), json!(expected));
        o.insert("completeness_problems".into(), json!(problems));
        o.insert("generated_utc".into(), json!(utc_now_iso()));
    }
    w("verdict.json", serde_json::to_string_pretty(&vfile).map_err(|e| e.to_string())?)?;
    Ok(PipelineOutcome {
        out_dir: cfg.out_dir.clone(),
        cells: rows.len(),
        expected_cells: expected,
        runtime_secs: started.elapsed().as_secs_f64(),
        problems,
        verdict,
    })
}
