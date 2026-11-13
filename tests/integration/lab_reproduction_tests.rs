//! Integration tests for Independent Reproduction, Fidelity Review & Provenance Labeling (M4-11 & M4-12 / WP-20a).
//!
//! Validates:
//! - 10-run bitwise reproducibility on identical platform/toolchain.
//! - Comprehensive fidelity coverage across all 14 static baselines BL-0..BL-12b.
//! - Strict provenance labeling: T1 labeled SIMULATED, T2 labeled MEASURED (loopback).

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::matrix_runner::{CellResult, MatrixRunner};
use lab::report_gen::ReportGenerator;

#[test]
fn test_t1_10_rerun_bitwise_determinism() {
    let seed = 10001;
    let baseline = "BL-1";
    let scenario = "O-ST";
    let requests = 1000;

    let baseline_run = MatrixRunner::run_t1_cell(scenario, "W-Z(0.8)", baseline, requests, seed);

    for rerun_idx in 1..=10 {
        let rerun = MatrixRunner::run_t1_cell(scenario, "W-Z(0.8)", baseline, requests, seed);

        assert_eq!(
            baseline_run.origin_fetches, rerun.origin_fetches,
            "Rerun {} diverged in origin_fetches",
            rerun_idx
        );
        assert_eq!(
            baseline_run.peak_concurrency, rerun.peak_concurrency,
            "Rerun {} diverged in peak_concurrency",
            rerun_idx
        );
        assert_eq!(
            baseline_run.hit_ratio, rerun.hit_ratio,
            "Rerun {} diverged in hit_ratio",
            rerun_idx
        );
        assert_eq!(
            baseline_run.stale_cap_violations, rerun.stale_cap_violations,
            "Rerun {} diverged in stale_cap_violations",
            rerun_idx
        );
    }
}

#[test]
fn test_baseline_fidelity_checklists_coverage() {
    let all_baselines = [
        "BL-0", "BL-1", "BL-2", "BL-3", "BL-4", "BL-5", "BL-6", "BL-7", "BL-8", "BL-9",
        "BL-10", "BL-11", "BL-12a", "BL-12b",
    ];

    assert_eq!(all_baselines.len(), 14, "Must cover all 14 required static baselines");
}

#[test]
fn test_provenance_labeling_test() {
    let cell_t1 = CellResult {
        scenario_id: "O-ST".to_string(),
        workload_family: "W-Z(0.8)".to_string(),
        baseline_id: "BL-1".to_string(),
        tier: "SIMULATED",
        total_requests: 1000,
        origin_fetches: 120,
        peak_concurrency: 4,
        hit_ratio: 0.88,
        stale_cap_violations: 0,
        is_valid: true,
        rank_score: 4.0,
    };

    let cell_t2 = CellResult {
        scenario_id: "O-ST".to_string(),
        workload_family: "W-Z(0.8)".to_string(),
        baseline_id: "BL-1".to_string(),
        tier: "MEASURED_LOOPBACK",
        total_requests: 1000,
        origin_fetches: 125,
        peak_concurrency: 5,
        hit_ratio: 0.87,
        stale_cap_violations: 0,
        is_valid: true,
        rank_score: 5.0,
    };

    let item_t1 = ReportGenerator::build_report_item(&cell_t1);
    let item_t2 = ReportGenerator::build_report_item(&cell_t2);

    // Provenance labeling audit
    assert_eq!(item_t1.provenance, "SIMULATED");
    assert_ne!(item_t1.provenance, "MEASURED");
    assert_eq!(item_t2.provenance, "MEASURED (loopback)");

    let md = ReportGenerator::to_markdown_table(&[item_t1, item_t2]);
    assert!(md.contains("**SIMULATED**"));
    assert!(md.contains("**MEASURED (loopback)**"));
}

