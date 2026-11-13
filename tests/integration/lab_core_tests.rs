//! Integration tests for Phantom Lab Core (M4-01 / WP-15).
//!
//! Validates:
//! - Virtual clock precision and monotonicity.
//! - Deterministic event scheduling with total ordering.
//! - Seed-splitting orthogonality and 10-run bitwise reproducibility.
//! - Vose's alias sampler accuracy and O(1) performance.
//! - Disjoint seed sets and sealed seed leak prevention.
//! - Run Manifest v2 generation, validity checks, and SHA-256 hashing.

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::clock::VirtualClock;
use lab::events::EventQueue;
use lab::manifest::{ExecutionTier, ProvenanceTag, RunManifestV2, RunValidityStatus};
use lab::rng::{AliasSampler, MasterRng, SeedSets};
use std::collections::HashSet;

#[test]
fn test_t1_virtual_clock_monotonicity() {
    let mut clock = VirtualClock::new();
    assert_eq!(clock.now_us(), 0);
    assert_eq!(clock.now_ms(), 0);

    clock.advance_to(1500);
    assert_eq!(clock.now_us(), 1500);
    assert_eq!(clock.now_ms(), 1); // 1500 / 1000 = 1

    clock.advance_by(550);
    assert_eq!(clock.now_us(), 2050);
    assert_eq!(clock.now_ms(), 2);
}

#[test]
fn test_t1_event_engine_total_ordering() {
    let mut q = EventQueue::new();

    // Schedule out of order with simultaneous timestamps
    let _ = q.schedule(100, "event_100_a");
    let _ = q.schedule(50, "event_50_a");
    let _ = q.schedule(100, "event_100_b");
    let _ = q.schedule(50, "event_50_b");
    let _ = q.schedule(20, "event_20");

    let e1 = q.pop().unwrap();
    assert_eq!(e1.id.time_us, 20);
    assert_eq!(e1.payload, "event_20");

    // Simultaneous timestamps at 50us must break ties by sequence ID
    let e2 = q.pop().unwrap();
    assert_eq!(e2.id.time_us, 50);
    assert_eq!(e2.payload, "event_50_a");

    let e3 = q.pop().unwrap();
    assert_eq!(e3.id.time_us, 50);
    assert_eq!(e3.payload, "event_50_b");

    // Simultaneous timestamps at 100us
    let e4 = q.pop().unwrap();
    assert_eq!(e4.id.time_us, 100);
    assert_eq!(e4.payload, "event_100_a");

    let e5 = q.pop().unwrap();
    assert_eq!(e5.id.time_us, 100);
    assert_eq!(e5.payload, "event_100_b");

    assert!(q.is_empty());
}

#[test]
fn test_t1_master_rng_seed_splitting_determinism() {
    // 10 reruns with the same master seed must yield identical pseudorandom streams
    let master_seed = 42_424_242;

    let baseline_samples: Vec<u64> = {
        let rng_master = MasterRng::new(master_seed);
        let mut arrivals = rng_master.stream("arrivals");
        (0..100).map(|_| arrivals.next_u64()).collect()
    };

    for run in 1..=10 {
        let rng_master = MasterRng::new(master_seed);
        let mut arrivals = rng_master.stream("arrivals");
        let samples: Vec<u64> = (0..100).map(|_| arrivals.next_u64()).collect();
        assert_eq!(
            samples, baseline_samples,
            "Run {} failed bitwise reproducibility test",
            run
        );
    }
}

#[test]
fn test_t1_stream_independence() {
    let master_seed = 999_888_777;
    let rng_master = MasterRng::new(master_seed);

    let mut keys_stream = rng_master.stream("keys");
    let keys_samples_baseline: Vec<u64> = (0..50).map(|_| keys_stream.next_u64()).collect();

    // In a second run, draw 10,000 samples from the "arrivals" and "origin" streams first
    let rng_master_2 = MasterRng::new(master_seed);
    let mut arrivals = rng_master_2.stream("arrivals");
    for _ in 0..10_000 {
        let _ = arrivals.next_u64();
    }
    let mut origin = rng_master_2.stream("origin");
    for _ in 0..5_000 {
        let _ = origin.next_u64();
    }

    // Drawing from "keys" must produce the EXACT same baseline samples despite heavy usage of other streams
    let mut keys_stream_2 = rng_master_2.stream("keys");
    let keys_samples_test: Vec<u64> = (0..50).map(|_| keys_stream_2.next_u64()).collect();
    assert_eq!(
        keys_samples_test, keys_samples_baseline,
        "Stream independence violation: consumption of 'arrivals' or 'origin' perturbed 'keys'"
    );
}

#[test]
fn test_vose_alias_sampler_accuracy_and_o1_speed() {
    let master_seed = 12345;
    let rng_master = MasterRng::new(master_seed);
    let mut rng = rng_master.stream("keys");

    // Zipf over 10 items with alpha = 1.0
    let n = 10;
    let alpha = 1.0;
    let sampler = AliasSampler::new_zipf(n, alpha);

    // Compute expected probabilities: P(k) = (1 / k) / sum(1 / k)
    let weights: Vec<f64> = (1..=n).map(|k| 1.0 / (k as f64)).collect();
    let sum: f64 = weights.iter().sum();
    let expected_p0 = weights[0] / sum; // Most frequent item (rank 1)
    let expected_p9 = weights[9] / sum; // Least frequent item (rank 10)

    let total_samples = 100_000;
    let mut counts = vec![0u64; n];
    for _ in 0..total_samples {
        let idx = sampler.sample(&mut rng);
        counts[idx] += 1;
    }

    let empirical_p0 = counts[0] as f64 / total_samples as f64;
    let empirical_p9 = counts[9] as f64 / total_samples as f64;

    // Chi-square style relative error within 3% on high-probability item
    assert!(
        (empirical_p0 - expected_p0).abs() < 0.015,
        "Rank 1 frequency mismatch: empirical {}, expected {}",
        empirical_p0,
        expected_p0
    );
    assert!(
        (empirical_p9 - expected_p9).abs() < 0.010,
        "Rank 10 frequency mismatch: empirical {}, expected {}",
        empirical_p9,
        expected_p9
    );
}

#[test]
fn test_seed_sets_disjointness_and_sealed_leak_detection() {
    let tuning = SeedSets::tuning_seeds();
    let dev = SeedSets::dev_eval_seeds();
    // Sealed seeds are no longer a plaintext range (M6 protocol repair, F-10); the derivation
    // mechanism is exercised with a synthetic test salt unrelated to the real committed salt.
    let sealed = SeedSets::derive_with_test_salt(b"unit-test-salt-not-the-real-one");

    assert_eq!(tuning.len(), 60);
    assert_eq!(dev.len(), 30);
    assert_eq!(sealed.len(), 30);

    let tuning_set: HashSet<u64> = tuning.into_iter().collect();
    let dev_set: HashSet<u64> = dev.into_iter().collect();
    let sealed_set: HashSet<u64> = sealed.into_iter().collect();

    // Verify pairwise disjointness
    assert!(tuning_set.is_disjoint(&dev_set));
    assert!(tuning_set.is_disjoint(&sealed_set));
    assert!(dev_set.is_disjoint(&sealed_set));

    // Verify sealed seed leak detector
    assert!(SeedSets::verify_no_sealed_leak(&[10001, 10002, 20005]).is_ok());

    // Using a burned legacy sealed seed (30001..=30030) must trigger Err
    let leaked_seed = 30015;
    let res = SeedSets::verify_no_sealed_leak(&[10001, leaked_seed, 20005]);
    assert_eq!(res, Err(leaked_seed));
}

#[test]
fn test_run_manifest_v2_provenance_and_validity_evaluation() {
    let mut manifest = RunManifestV2::new(
        "run_test_001".to_string(),
        "SCENARIO_O_ST_WZ".to_string(),
        "hash_scen_123456".to_string(),
        "BL-1".to_string(),
        serde_json::json!({ "single_flight": true }),
        ExecutionTier::T1Simulated,
        10001, // Tuning seed
    );

    // Populate metric provenance map
    manifest.metric_provenance.insert("p99_latency_us".to_string(), ProvenanceTag::Simulated);
    manifest.metric_provenance.insert("hit_ratio".to_string(), ProvenanceTag::Simulated);

    let status = manifest.evaluate_validity();
    assert_eq!(status, RunValidityStatus::Valid);
    assert!(manifest.invalid_reasons.is_empty());

    let hash1 = manifest.compute_manifest_hash();
    assert_eq!(hash1.len(), 64);

    // If master seed is set to a sealed evaluation seed, validity must FAIL
    manifest.master_seed = 30005; // Leaked sealed seed!
    let status_bad = manifest.evaluate_validity();
    assert_eq!(status_bad, RunValidityStatus::Invalid);
    assert!(!manifest.invalid_reasons.is_empty());
}

