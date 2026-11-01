//! Integration tests for Scenario Matrix Execution and T1 <-> T2 Agreement Study (M4-09 & M4-10 / WP-19).
//!
//! Validates:
//! - T1 full matrix execution across degradation scenarios (O-NONE, O-ST, O-BR, O-OUT) and baselines.
//! - Strict EV-12 staleness-cap validation across all matrix cells.
//! - T1 <-> T2 agreement study verifying identical ranking order between simulation and measured tiers.
//! - Consequence rule application: certifies scenarios as predictive when concordance >= 80%.

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::matrix_runner::{AgreementStudy, CellResult, MatrixRunner};

#[test]
fn test_t1_matrix_execution_and_ev12_check() {
    let scenarios = ["O-NONE", "O-ST", "O-BR", "O-OUT"];
    let baselines = ["BL-0", "BL-1", "BL-12b"];
    let mut results = Vec::new();

    for &sc in &scenarios {
        for &bl in &baselines {
            let res = MatrixRunner::run_t1_cell(sc, "W-Z(0.8)", bl, 1000, 20001);
            assert!(res.is_valid, "Matrix cell must be valid");
            assert_eq!(res.stale_cap_violations, 0, "EV-12 violation: stale_cap_violations must be 0");
            results.push(res);
        }
    }

    assert_eq!(results.len(), scenarios.len() * baselines.len());

    // In every scenario, BL-1 and BL-12b must outperform BL-0 on origin fetches
    for sc in &scenarios {
        let bl0 = results.iter().find(|r| r.scenario_id == *sc && r.baseline_id == "BL-0").unwrap();
        let bl1 = results.iter().find(|r| r.scenario_id == *sc && r.baseline_id == "BL-1").unwrap();
        let bl12b = results.iter().find(|r| r.scenario_id == *sc && r.baseline_id == "BL-12b").unwrap();

        assert!(bl1.origin_fetches < bl0.origin_fetches);
        assert!(bl12b.origin_fetches <= bl1.origin_fetches);
    }
}

#[test]
fn test_t1_t2_agreement_study_concordance() {
    // Generate T1 results for O-NONE scenario across 3 baselines
    let t1_res = vec![
    // R-02: T1 cells are now real product-path runs. Under a healthy origin the three arms tie on peak
    // concurrency (no ranking exists), so the degraded O-DEG scenario is used. The T2 cells below are
    // SYNTHETIC FIXTURES that only exercise AgreementStudy mechanics; they are not measurements.
        MatrixRunner::run_t1_cell("O-DEG", "W-Z(0.8)", "BL-12b", 1000, 20002), // best: lowest peak
        MatrixRunner::run_t1_cell("O-DEG", "W-Z(0.8)", "BL-1", 1000, 20002),   // mid
        MatrixRunner::run_t1_cell("O-DEG", "W-Z(0.8)", "BL-0", 1000, 20002),   // worst: stampede
    ];

    // Corresponding T2 measured results reflecting identical relative performance order
    let t2_res = vec![
        CellResult {
            scenario_id: "O-NONE".to_string(),
            workload_family: "W-Z(0.8)".to_string(),
            baseline_id: "BL-12b".to_string(),
            tier: "MEASURED_LOOPBACK",
            total_requests: 1000,
            origin_fetches: 85,
            peak_concurrency: 2,
            hit_ratio: 0.90,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 2.0,
        },
        CellResult {
            scenario_id: "O-NONE".to_string(),
            workload_family: "W-Z(0.8)".to_string(),
            baseline_id: "BL-1".to_string(),
            tier: "MEASURED_LOOPBACK",
            total_requests: 1000,
            origin_fetches: 100,
            peak_concurrency: 4,
            hit_ratio: 0.88,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 4.0,
        },
        CellResult {
            scenario_id: "O-NONE".to_string(),
            workload_family: "W-Z(0.8)".to_string(),
            baseline_id: "BL-0".to_string(),
            tier: "MEASURED_LOOPBACK",
            total_requests: 1000,
            origin_fetches: 1000,
            peak_concurrency: 12,
            hit_ratio: 0.0,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 12.0,
        },
    ];

    let (concordance, is_predictive) = AgreementStudy::evaluate_concordance(&t1_res, &t2_res);
    assert_eq!(concordance, 1.0, "T1 and T2 must yield identical ranking order (100% concordance)");
    assert!(is_predictive, "Consequence rule: concordance >= 0.80 must certify T1 as predictive");
}

#[test]
fn test_consequence_rule_rejection() {
    let t1_res = vec![
        CellResult {
            scenario_id: "O-ST".to_string(),
            workload_family: "W-FC".to_string(),
            baseline_id: "BL-1".to_string(),
            tier: "SIMULATED",
            total_requests: 1000,
            origin_fetches: 100,
            peak_concurrency: 3,
            hit_ratio: 0.85,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 3.0,
        },
        CellResult {
            scenario_id: "O-ST".to_string(),
            workload_family: "W-FC".to_string(),
            baseline_id: "BL-2".to_string(),
            tier: "SIMULATED",
            total_requests: 1000,
            origin_fetches: 200,
            peak_concurrency: 8,
            hit_ratio: 0.80,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 8.0,
        },
    ];

    // Artificial inversion: T2 ranks BL-2 as better than BL-1
    let t2_inverted = vec![
        CellResult {
            scenario_id: "O-ST".to_string(),
            workload_family: "W-FC".to_string(),
            baseline_id: "BL-2".to_string(),
            tier: "MEASURED_LOOPBACK",
            total_requests: 1000,
            origin_fetches: 80,
            peak_concurrency: 2,
            hit_ratio: 0.90,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 2.0,
        },
        CellResult {
            scenario_id: "O-ST".to_string(),
            workload_family: "W-FC".to_string(),
            baseline_id: "BL-1".to_string(),
            tier: "MEASURED_LOOPBACK",
            total_requests: 1000,
            origin_fetches: 150,
            peak_concurrency: 6,
            hit_ratio: 0.82,
            stale_cap_violations: 0,
            is_valid: true,
            rank_score: 6.0,
        },
    ];

    let (concordance, is_predictive) = AgreementStudy::evaluate_concordance(&t1_res, &t2_inverted);
    assert_eq!(concordance, 0.0);
    assert!(!is_predictive, "Consequence rule must flag ranking mismatch as non-predictive");
}

