//! Safety, Overhead, Planted-Effect & Replay Verification Tests (WP-37)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-07

use phantom_cache_demo::core::cache::{EvictionPolicy, PartitionedCache};
use phantom_cache_demo::intelligence::audit::AuditStore;
use phantom_cache_demo::intelligence::engine::PieDecisionEngine;
use phantom_cache_demo::intelligence::executor::{
    BucketArm, DefaultPolicyExecutor, SafeBaselineConfig,
};
use phantom_cache_demo::intelligence::signals::WorkloadPipeline;
use phantom_cache_demo::intelligence::*;
use std::sync::Arc;
use std::time::Instant;

fn make_candidate(id: &str) -> PolicyCandidate {
    PolicyCandidate {
        candidate_id: id.to_string(),
        kind: PolicyKind::Eviction,
        parameters: [].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "Planted-Effect Verification".to_string(),
    }
}

// ----------------------------------------------------------------------------
// 1. Planted-Effect Validation
// ----------------------------------------------------------------------------

#[test]
fn test_planted_effect_positive_detection() {
    let mut detections = 0;
    let trials = 10;

    for _ in 0..trials {
        let mut engine = PieDecisionEngine::new();
        let candidate = make_candidate("POSITIVE_CANDIDATE");

        // Control: 60% hit ratio on 3,000 requests
        for _ in 0..1800 {
            engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
        }
        for _ in 0..1200 {
            engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
        }

        // Candidate with planted +8% hit ratio improvement (68% vs 60% over 3,000 requests)
        // Effect = 0.08, CI lower bound > delta_min = 0.05
        for _ in 0..2040 {
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..960 {
            engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
        }

        let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_SAFE");
        if record.verdict == Verdict::Commit && record.guardrails_passed {
            detections += 1;
        }
    }

    // High statistical power: +8% with 1000 requests each consistently commits
    assert_eq!(
        detections, trials,
        "Expected 100% detection power for planted +8% hit ratio effect"
    );
}

#[test]
fn test_planted_effect_negative_protection() {
    let mut false_commits = 0;
    let trials = 20;

    for _ in 0..trials {
        let mut engine = PieDecisionEngine::new();
        let candidate = make_candidate("DEGRADED_CANDIDATE");

        // Control: 60% hit ratio
        for _ in 0..600 {
            engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
        }
        for _ in 0..400 {
            engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
        }

        // Candidate with planted -8% degradation (52% vs 60%)
        for _ in 0..520 {
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..480 {
            engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
        }

        let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_SAFE");
        if record.verdict == Verdict::Commit {
            false_commits += 1;
        }
    }

    assert_eq!(
        false_commits, 0,
        "False commit rate must be strictly 0% on degraded candidates"
    );
}

// ----------------------------------------------------------------------------
// 2. Rollback Proof & State Equality
// ----------------------------------------------------------------------------

#[test]
fn test_rollback_state_equality_and_latency() {
    let cache = Arc::new(PartitionedCache::new(4));

    let safe_config = SafeBaselineConfig {
        eviction_policy: EvictionPolicy::Sieve,
        max_stale_ms: 86_400_000,
        max_background_refresh: 128,
        config_hash: "SAFE_BL1".to_string(),
    };
    let executor = DefaultPolicyExecutor::new(Some(Arc::clone(&cache))).with_safe_config(safe_config);
    executor.set_mode(IntelligenceMode::Canary).unwrap();

    // Populate warm entries in cache under safe baseline
    for i in 0..50 {
        let key = format!("warm_key_{}", i);
        let val = format!("warm_val_{}", i).into_bytes();
        cache.insert(&key, val, 60_000, 1_000);
    }

    // Apply candidate mutation (LRU, lowered stale limit, lowered refresh cap)
    let candidate = PolicyCandidate {
        candidate_id: "LRU".to_string(),
        kind: PolicyKind::Eviction,
        parameters: [("max_stale_ms".to_string(), 10_000.0)].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "Testing rollback".to_string(),
    };
    executor.apply_policy(&candidate).unwrap();
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Lru);

    // Measure rollback latency bound
    let start = Instant::now();
    executor.revert_to_safe();
    let elapsed = start.elapsed();

    // Bound check: measured latency must be <= 1000 ms (PROVISIONAL target)
    assert!(
        elapsed.as_millis() <= 1000,
        "Rollback latency was {} ms, expected <= 1000 ms",
        elapsed.as_millis()
    );

    // Verify restored state equality
    assert_eq!(
        cache.get_eviction_policy(),
        EvictionPolicy::Sieve,
        "Eviction policy was not restored to safe baseline"
    );
    assert_eq!(cache.get_max_stale_ms(), 86_400_000);
    assert_eq!(cache.get_max_background_refresh(), 128);

    // Verify all warm entries remain intact post-rollback
    for i in 0..50 {
        let key = format!("warm_key_{}", i);
        let val = cache.get(&key, 1_000);
        assert!(val.is_some(), "Warm entry {} lost after rollback", key);
    }
}

// ----------------------------------------------------------------------------
// 3. Fault Injection: Panic Isolation & Provenance Gating
// ----------------------------------------------------------------------------

#[test]
fn test_fault_injection_panic_isolation() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = DefaultPolicyExecutor::new(Some(cache));
    executor.set_mode(IntelligenceMode::Canary).unwrap();

    // Normal candidate applies safely
    let candidate = make_candidate("NORMAL");
    assert!(executor.safe_apply_policy(&candidate).is_ok());

    // In Observe mode, apply_policy returns Err, safe_apply_policy returns Err cleanly
    executor.set_mode(IntelligenceMode::Observe).unwrap();
    let res = executor.safe_apply_policy(&candidate);
    assert!(res.is_err());
    assert_eq!(executor.current_mode(), IntelligenceMode::Observe);
}

#[test]
fn test_fault_injection_provenance_gating() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("CANDIDATE");

    // Populate sufficient traffic for commit
    for _ in 0..600 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
    }
    for _ in 0..400 {
        engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
    }
    for _ in 0..800 {
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }
    for _ in 0..200 {
        engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
    }

    // 1. In Canary mode, Simulated provenance must be refused
    let rec_sim = engine.evaluate_with_provenance(
        &candidate,
        IntelligenceMode::Canary,
        &MetricProvenance::Simulated,
        10_000,
        "CONF_OLD",
    );
    assert_eq!(
        rec_sim.verdict,
        Verdict::NoAction,
        "Live mode must refuse Simulated provenance"
    );
    assert!(!rec_sim.guardrails_passed);
    assert!(rec_sim.explanation.contains("invalid provenance"));

    // 2. In Canary mode, Unavailable provenance must be refused
    let rec_unavail = engine.evaluate_with_provenance(
        &candidate,
        IntelligenceMode::Canary,
        &MetricProvenance::Unavailable("Degraded telemetry".to_string()),
        10_000,
        "CONF_OLD",
    );
    assert_eq!(
        rec_unavail.verdict,
        Verdict::NoAction,
        "Live mode must refuse Unavailable provenance"
    );

    // 3. In Advisory mode, Simulated provenance is accepted for offline research
    let rec_advisory = engine.evaluate_with_provenance(
        &candidate,
        IntelligenceMode::Advisory,
        &MetricProvenance::Simulated,
        10_000,
        "CONF_OLD",
    );
    assert_eq!(
        rec_advisory.verdict,
        Verdict::Commit,
        "Advisory mode accepts Simulated provenance for analysis"
    );

    // 4. In Canary mode, Measured provenance is accepted (after cooldown expires at 70,000 ms)
    let rec_measured = engine.evaluate_with_provenance(
        &candidate,
        IntelligenceMode::Canary,
        &MetricProvenance::Measured,
        80_000,
        "CONF_OLD",
    );
    assert_eq!(
        rec_measured.verdict,
        Verdict::Commit,
        "Live mode accepts Measured provenance"
    );
}

#[test]
fn test_fault_injection_backward_clock_jump() {
    let executor = DefaultPolicyExecutor::new(None);
    executor.heartbeat_at(50_000);

    // Backward clock jump: now_ms is 20_000 (< 50_000)
    // Watchdog should handle non-monotonicity gracefully without panic or false trip
    let tripped = executor.check_watchdog(20_000, 1_000);
    assert!(!tripped, "Backward clock jump should not falsely trip watchdog");

    // Standard stall: now_ms is 52_000 (> 50_000 + 1_000)
    let tripped2 = executor.check_watchdog(52_000, 1_000);
    assert!(tripped2, "True stall must trip watchdog");
}

// ----------------------------------------------------------------------------
// 4. Deterministic Replay
// ----------------------------------------------------------------------------

#[test]
fn test_deterministic_replay_produces_identical_records() {
    fn run_replay_session() -> (DecisionRecord, String) {
        let mut engine = PieDecisionEngine::new();
        let candidate = make_candidate("REPLAY_CANDIDATE");

        // Deterministic pseudo-random generation (LCG)
        let mut seed: u64 = 42;
        let mut lcg = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed >> 33
        };

        for i in 0..1000 {
            let ctrl_hit = (lcg() % 100) < 60;
            let cand_hit = (lcg() % 100) < 75;
            let lat = 400 + (lcg() % 200);

            let arm = DefaultPolicyExecutor::route_bucket((i as u64).wrapping_mul(104729));
            let is_hit = if arm == BucketArm::Candidate {
                cand_hit
            } else {
                ctrl_hit
            };
            engine.record_arm_traffic(arm, is_hit, lat, false, false);
        }

        let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 100_000, "CONF_BASELINE");
        let mut store = AuditStore::new(100);
        store.append(record.clone());
        let json = store.export_json().unwrap();
        (record, json)
    }

    let (rec1, json1) = run_replay_session();
    let (rec2, json2) = run_replay_session();

    assert_eq!(rec1.verdict, rec2.verdict);
    assert_eq!(rec1.estimated_effect, rec2.estimated_effect);
    assert_eq!(rec1.confidence_interval, rec2.confidence_interval);
    assert_eq!(rec1.explanation, rec2.explanation);
    assert_eq!(rec1.config_hash_after, rec2.config_hash_after);
    assert_eq!(json1, json2, "Replay JSON must be bitwise identical");
}

// ----------------------------------------------------------------------------
// 5. Anti-Flapping & Stacking Limits
// ----------------------------------------------------------------------------

#[test]
fn test_anti_flapping_and_stacking_limits() {
    let mut engine = PieDecisionEngine::new().with_cooldown_ms(1_000);
    let candidate = make_candidate("OSCILLATING");

    let mut successful_switches = 0;

    // Simulate 10 consecutive attempts over 30 minutes
    for attempt in 0..10 {
        let time_ms = 100_000 + (attempt as u64) * 120_000; // Every 2 minutes

        // Feed traffic with +15% improvement
        engine.reset_arm_metrics();
        for _ in 0..600 {
            engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..400 {
            engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
        }
        for _ in 0..200 {
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..200 {
            engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
        }

        let record = engine.evaluate(&candidate, IntelligenceMode::Canary, time_ms, "CONF_OLD");
        if record.verdict == Verdict::Commit {
            successful_switches += 1;
        }
    }

    // Hourly rate limit must clamp total switches to at most 4 per rolling hour
    assert_eq!(
        successful_switches, 4,
        "Anti-flapping must clamp switches to <= 4 per rolling hour"
    );
}

// ----------------------------------------------------------------------------
// 6. Adversarial Traffic & Overhead Budgets
// ----------------------------------------------------------------------------

#[test]
fn test_adversarial_hot_key_flooding() {
    let mut pipeline = WorkloadPipeline::new();

    // Adversarial traffic: flood 50,000 requests of key 0xDEADBEEF mixed with noise
    for i in 0..50_000 {
        let key = if i % 2 == 0 {
            0xDEADBEEF
        } else {
            (i as u64).wrapping_mul(73)
        };
        pipeline.observe_request(key, true, 200, 1000 + i as u64);
    }

    let window = pipeline.produce_window(51_000, 50_000);
    assert_eq!(window.request_count, 50_000);
    assert!(!window.f3_top_heavy_hitters.value.is_empty());
    assert_eq!(
        window.f3_top_heavy_hitters.value[0].0, 0xDEADBEEF,
        "Adversarial flooded key was not detected as primary heavy hitter"
    );
}

#[test]
fn test_overhead_memory_bounds() {
    let mut store = AuditStore::new(100);

    // Insert 200 decision records
    for i in 0..200 {
        let record = DecisionRecord {
            decision_id: i,
            timestamp_ms: 10_000 + i,
            mode: IntelligenceMode::Canary,
            trigger_reason: "Stress test".to_string(),
            candidate: make_candidate("TEST"),
            estimated_effect: 0.1,
            confidence_interval: (0.06, 0.14),
            guardrails_passed: true,
            verdict: Verdict::Commit,
            explanation: "Record explanation".to_string(),
            config_hash_before: "HASH_A".to_string(),
            config_hash_after: "HASH_B".to_string(),
        };
        store.append(record);
    }

    // Bounded ring buffer invariant
    assert_eq!(store.len(), 100);

    // Check JSON serialization size is bounded (< 64 KB for 100 records)
    let json = store.export_json().unwrap();
    assert!(
        json.len() < 64 * 1024,
        "Audit store json length {} exceeds 64 KB",
        json.len()
    );
}
