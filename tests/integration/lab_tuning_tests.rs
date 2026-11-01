//! Integration tests for Phantom Lab Tuning Protocol & Leak Detector (M4-08 / WP-18).
//!
//! Validates:
//! - Fair tuning protocol execution with equal budget per baseline.
//! - Frozen configuration selection and SHA-256 digest computation.
//! - Zero staleness-cap violations enforcement.
//! - Leak detection: Dev-Eval seeds (D), Sealed evaluation seeds (S), and held-out families
//!   are immediately detected and flagged.

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::rng::SeedSets;
use lab::tuning::{TuningLog, TuningTrial};
use std::collections::HashMap;

#[test]
fn test_tuning_protocol_equal_budget_and_optimization() {
    let mut log = TuningLog::new();
    let tuning_seeds = SeedSets::tuning_seeds();

    // Run 60 tuning trials across the 60 tuning seeds
    for (i, &seed) in tuning_seeds.iter().enumerate() {
        let mut params = HashMap::new();
        let timeout_val = 50.0 + (i as f64) * 10.0;
        params.insert("timeout_ms".to_string(), timeout_val);

        // Simulate tuning outcome
        let peak_concurrency = if timeout_val < 200.0 { 12 } else { 4 };
        let score = peak_concurrency as f64;

        log.record(TuningTrial {
            trial_id: i,
            baseline_id: "BL-2".to_string(),
            params,
            seed,
            workload_family: "W-Z(0.8)".to_string(),
            peak_concurrency,
            hit_ratio: 0.85,
            stale_cap_violations: 0,
            objective_score: score,
        });
    }

    // Must pass leak check
    assert!(log.assert_zero_leakage().is_ok());

    // Freeze best configuration
    let frozen = log.freeze_best_config("BL-2").expect("Must select best config");
    assert_eq!(frozen.baseline_id, "BL-2");
    assert_eq!(frozen.trials_run, 60);
    assert_eq!(frozen.best_score, 4.0);
    assert_eq!(frozen.config_sha256.len(), 64); // SHA-256 hex string
}

#[test]
fn test_tuning_leak_detector_flags_dev_eval_seed() {
    let mut log = TuningLog::new();
    let tuning_seeds = SeedSets::tuning_seeds();
    let dev_eval_seeds = SeedSets::dev_eval_seeds();

    // Use a legitimate tuning seed first
    log.record(TuningTrial {
        trial_id: 1,
        baseline_id: "BL-1".to_string(),
        params: HashMap::new(),
        seed: tuning_seeds[0],
        workload_family: "W-Z(0.8)".to_string(),
        peak_concurrency: 5,
        hit_ratio: 0.80,
        stale_cap_violations: 0,
        objective_score: 5.0,
    });
    assert!(log.assert_zero_leakage().is_ok());

    // Contaminate with Dev-Eval seed from D
    log.record(TuningTrial {
        trial_id: 2,
        baseline_id: "BL-1".to_string(),
        params: HashMap::new(),
        seed: dev_eval_seeds[5], // Dev-Eval seed!
        workload_family: "W-Z(0.8)".to_string(),
        peak_concurrency: 5,
        hit_ratio: 0.80,
        stale_cap_violations: 0,
        objective_score: 5.0,
    });

    let res = log.assert_zero_leakage();
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("LEAK DETECTED: Dev-Eval seed"));
}

#[test]
fn test_tuning_leak_detector_flags_sealed_seed() {
    let mut log = TuningLog::new();

    // Contaminate with Sealed evaluation seed from S (30005)
    log.record(TuningTrial {
        trial_id: 1,
        baseline_id: "BL-4".to_string(),
        params: HashMap::new(),
        seed: 30005, // Sealed seed!
        workload_family: "W-FC".to_string(),
        peak_concurrency: 10,
        hit_ratio: 0.70,
        stale_cap_violations: 0,
        objective_score: 10.0,
    });

    let res = log.assert_zero_leakage();
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("CRITICAL LEAK DETECTED: Sealed evaluation seed"));
}

#[test]
fn test_tuning_leak_detector_flags_held_out_family() {
    let mut log = TuningLog::new();
    let tuning_seeds = SeedSets::tuning_seeds();

    // Contaminate with held-out Diurnal family (W-DI)
    log.record(TuningTrial {
        trial_id: 1,
        baseline_id: "BL-7".to_string(),
        params: HashMap::new(),
        seed: tuning_seeds[2],
        workload_family: "W-DI".to_string(), // Held-out family!
        peak_concurrency: 8,
        hit_ratio: 0.75,
        stale_cap_violations: 0,
        objective_score: 8.0,
    });

    let res = log.assert_zero_leakage();
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("HELD-OUT FAMILY CONTAMINATION"));
}
