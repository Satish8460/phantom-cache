use phantom_cache_demo::intelligence::engine::*;
use phantom_cache_demo::intelligence::executor::BucketArm;
use phantom_cache_demo::intelligence::*;

fn make_candidate(id: &str) -> PolicyCandidate {
    PolicyCandidate {
        candidate_id: id.to_string(),
        kind: PolicyKind::Eviction,
        parameters: [].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "Optimization".to_string(),
    }
}

#[test]
fn test_decision_engine_commit_rule() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("S3-FIFO");

    // Feed 1,000 control requests: 60% hit ratio
    for _ in 0..600 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
    }
    for _ in 0..400 {
        engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
    }

    // Feed 1,000 candidate requests: 75% hit ratio (+15% improvement, well above delta_min = 5%)
    for _ in 0..750 {
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }
    for _ in 0..250 {
        engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
    }

    let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_OLD");
    assert_eq!(record.verdict, Verdict::Commit);
    assert!(record.guardrails_passed);
    assert!(record.confidence_interval.0 > 0.05);
    assert_ne!(record.config_hash_before, record.config_hash_after);
}

#[test]
fn test_decision_engine_rollback_on_stale_violation() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("TINYLFU");

    // Feed normal traffic
    for _ in 0..600 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }

    // Inject single stale violation on candidate arm
    engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, true);

    let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_OLD");
    assert_eq!(record.verdict, Verdict::Rollback);
    assert!(!record.guardrails_passed);
    assert!(record.explanation.contains("stale cap violations"));
}

#[test]
fn test_decision_engine_rollback_on_latency_inflation() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("HEAVY_POLICY");

    // Control: p99 latency 500 us
    for _ in 0..600 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
    }

    // Candidate: p99 latency inflated to 2500 us (+400% inflation, bound is +10%)
    for _ in 0..600 {
        engine.record_arm_traffic(BucketArm::Candidate, true, 2500, false, false);
    }

    let record = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_OLD");
    assert_eq!(record.verdict, Verdict::Rollback);
    assert!(!record.guardrails_passed);
    assert!(record.explanation.contains("inflated"));
}

#[test]
fn test_decision_engine_no_action_in_cooldown() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("S3-FIFO");

    // 1. Commit first candidate at t = 1,000 (control 50%, candidate 80%)
    for _ in 0..500 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
    }
    for _ in 0..500 {
        engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
    }
    for _ in 0..800 {
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }
    for _ in 0..200 {
        engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
    }

    let rec1 = engine.evaluate(&candidate, IntelligenceMode::Canary, 1_000, "CONF_A");
    assert_eq!(rec1.verdict, Verdict::Commit);

    // 2. Immediate next evaluation at t = 10,000 (within 60s cooldown)
    let rec2 = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_B");
    assert_eq!(rec2.verdict, Verdict::NoAction);
    assert!(rec2.explanation.contains("cooldown"));
}

#[test]
fn test_decision_engine_insufficient_samples() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("TEST");

    // Only 50 samples (min_samples = 500)
    for _ in 0..50 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }

    let rec = engine.evaluate(&candidate, IntelligenceMode::Canary, 10_000, "CONF_A");
    assert_eq!(rec.verdict, Verdict::NoAction);
    assert!(rec.explanation.contains("Insufficient samples"));
}

#[test]
fn test_hourly_switch_rate_cap() {
    let mut engine = PieDecisionEngine::new();
    engine = engine.with_cooldown_ms(100); // 100ms cooldown for fast test
    let candidate = make_candidate("RAPID_SWITCH");

    // Perform 4 commits at 1-second intervals within 1 minute
    for switch_num in 0..4 {
        engine.reset_arm_metrics();
        for _ in 0..500 {
            engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
        }
        for _ in 0..500 {
            engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
        }
        for _ in 0..800 {
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..200 {
            engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
        }

        let t = 10_000 + switch_num * 1000;
        let rec = engine.evaluate(&candidate, IntelligenceMode::Canary, t, "CONF");
        assert_eq!(rec.verdict, Verdict::Commit);
    }

    // 5th commit attempted at t = 15,000 within the same hour
    engine.reset_arm_metrics();
    for _ in 0..500 {
        engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
    }
    for _ in 0..500 {
        engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
    }
    for _ in 0..800 {
        engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
    }
    for _ in 0..200 {
        engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
    }

    let rec5 = engine.evaluate(&candidate, IntelligenceMode::Canary, 15_000, "CONF");
    assert_eq!(rec5.verdict, Verdict::NoAction);
    assert!(rec5.explanation.contains("hourly switch cap"));
}

#[test]
fn test_decision_engine_aa_calibration() {
    let mut engine = PieDecisionEngine::new();
    let candidate = make_candidate("AA_TEST");

    let mut false_commits = 0;

    // Run 50 independent A/A evaluations with identical distributions (70% hit ratio)
    for trial in 0..50 {
        engine.reset_arm_metrics();
        for _ in 0..700 {
            engine.record_arm_traffic(BucketArm::Control, true, 500, false, false);
            engine.record_arm_traffic(BucketArm::Candidate, true, 500, false, false);
        }
        for _ in 0..300 {
            engine.record_arm_traffic(BucketArm::Control, false, 500, false, false);
            engine.record_arm_traffic(BucketArm::Candidate, false, 500, false, false);
        }

        let t = 100_000 + trial * 70_000;
        let rec = engine.evaluate(&candidate, IntelligenceMode::Canary, t, "CONF");
        if rec.verdict == Verdict::Commit {
            false_commits += 1;
        }
    }

    // False commit rate should be 0 because delta = 0 and delta_min = 0.05
    assert_eq!(
        false_commits, 0,
        "A/A calibration failed: false commits = {}",
        false_commits
    );
}
