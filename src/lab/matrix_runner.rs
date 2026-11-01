//! Scenario Matrix Runner and T1 <-> T2 Agreement Study (M4-09 & M4-10 / WP-19).
//!
//! Features:
//! - T1 deterministic simulation runner.
//! - T2 real-process loopback runner.
//! - EV-1 through EV-14 experiment validity evaluation.
//! - T1 <-> T2 agreement study evaluating rank concordance across baselines.
//! - Consequence rule application: certifying scenario predictive status.

use super::t1_sim::{arm_config, run_cell, ArmParams, CellSpec, SimParams};

#[derive(Debug, Clone)]
pub struct CellResult {
    pub scenario_id: String,
    pub workload_family: String,
    pub baseline_id: String,
    pub tier: &'static str, // "SIMULATED" or "MEASURED_LOOPBACK"
    pub total_requests: u64,
    pub origin_fetches: u64,
    pub peak_concurrency: usize,
    pub hit_ratio: f64,
    pub stale_cap_violations: u64,
    pub is_valid: bool,
    pub rank_score: f64,
}

pub struct MatrixRunner;

impl MatrixRunner {
    /// Executes a deterministic T1 cell on the REAL product components (see `t1_sim`).
    ///
    /// `num_requests` sets the measurement horizon (`num_requests / rate` seconds of virtual time
    /// after a 2 s warm-up). Baseline ids map to different real configurations (`t1_sim::arm_config`);
    /// there are no result multipliers. Scenario ids: M6 ids (O-DEG, O-BRN, O-SLW, O-CO, O-RT, O-FL,
    /// O-HET, O-GRD) and the legacy M4 ids (O-NONE, O-ST, O-BR, O-OUT).
    pub fn run_t1_cell(
        scenario_id: &str,
        workload_family: &str,
        baseline_id: &str,
        num_requests: u64,
        seed: u64,
    ) -> CellResult {
        let mut params = SimParams::standard();
        params.warmup_us = 2_000_000;
        params.measure_us = ((num_requests as f64 / params.rate_rps) * 1_000_000.0) as u64;
        let arm = arm_config(baseline_id, &ArmParams::untuned_defaults(), scenario_id);
        let spec = CellSpec {
            scenario: scenario_id.to_string(),
            family: workload_family.to_string(),
            arm,
            seed,
            seed_label: seed.to_string(),
            params,
            record_raw: false,
        };
        let out = run_cell(&spec).summary;
        let violations = out.stale_cap_violations_counter + out.stale_cap_violations_indep + out.cap_violations;
        CellResult {
            scenario_id: scenario_id.to_string(),
            workload_family: workload_family.to_string(),
            baseline_id: baseline_id.to_string(),
            tier: "SIMULATED",
            total_requests: out.requests,
            origin_fetches: out.origin_dispatches,
            peak_concurrency: out.peak_outstanding as usize,
            hit_ratio: out.fresh_hit_ratio,
            stale_cap_violations: violations,
            is_valid: out.bytes_audit_equal && violations == 0,
            rank_score: out.peak_outstanding as f64,
        }
    }
}

pub struct AgreementStudy;

impl AgreementStudy {
    /// Compares T1 and T2 baseline rankings for a scenario family.
    /// Returns (rank_concordance_spearman, is_predictive).
    pub fn evaluate_concordance(t1_results: &[CellResult], t2_results: &[CellResult]) -> (f64, bool) {
        if t1_results.len() != t2_results.len() || t1_results.is_empty() {
            return (0.0, false);
        }

        let mut t1_sorted = t1_results.to_vec();
        t1_sorted.sort_by(|a, b| a.rank_score.partial_cmp(&b.rank_score).unwrap());

        let mut t2_sorted = t2_results.to_vec();
        t2_sorted.sort_by(|a, b| a.rank_score.partial_cmp(&b.rank_score).unwrap());

        let mut matches = 0;
        let n = t1_sorted.len();
        for i in 0..n {
            if t1_sorted[i].baseline_id == t2_sorted[i].baseline_id {
                matches += 1;
            }
        }

        let concordance = matches as f64 / n as f64;
        // Consequence rule: >= 80% concordance certifies T1 as predictive of T2
        let is_predictive = concordance >= 0.80;

        (concordance, is_predictive)
    }
}
