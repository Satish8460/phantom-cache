use phantom_cache_demo::core::cache::{EvictionPolicy, PartitionedCache};
use phantom_cache_demo::intelligence::executor::*;
use phantom_cache_demo::intelligence::*;
use std::sync::Arc;

#[test]
fn test_canary_bucket_stratification() {
    let mut holdback_count = 0;
    let mut candidate_count = 0;
    let mut control_count = 0;

    for key_hash in 0..10_000u64 {
        match DefaultPolicyExecutor::route_bucket(key_hash) {
            BucketArm::Holdback => holdback_count += 1,
            BucketArm::Candidate => candidate_count += 1,
            BucketArm::Control => control_count += 1,
        }
    }

    let total = 10_000.0;
    let holdback_pct = holdback_count as f64 / total;
    let candidate_pct = candidate_count as f64 / total;
    let control_pct = control_count as f64 / total;

    // Expected: ~1% holdback, ~5% candidate, ~94% control
    assert!(
        (holdback_pct - 0.01).abs() <= 0.005,
        "Holdback pct {} deviated from 0.01",
        holdback_pct
    );
    assert!(
        (candidate_pct - 0.05).abs() <= 0.015,
        "Candidate pct {} deviated from 0.05",
        candidate_pct
    );
    assert!(
        (control_pct - 0.94).abs() <= 0.02,
        "Control pct {} deviated from 0.94",
        control_pct
    );
}

#[test]
fn test_hard_caps_enforcement() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = DefaultPolicyExecutor::new(Some(cache.clone()));
    executor
        .set_mode(IntelligenceMode::Active)
        .expect("Set mode failed");

    // Candidate attempting to exceed RFC 9111 hard staleness cap (e.g. 500,000,000 ms)
    let candidate = PolicyCandidate {
        candidate_id: "ADVERSARIAL_STALE".to_string(),
        kind: PolicyKind::StaleServing,
        parameters: [("max_stale_ms".to_string(), 500_000_000.0)]
            .into_iter()
            .collect(),
        bounds: [("max_stale_ms".to_string(), (0.0, 500_000_000.0))]
            .into_iter()
            .collect(),
        expected_effect: "Infinite stale serving".to_string(),
    };

    executor
        .apply_policy(&candidate)
        .expect("Apply policy failed");
    // Verify cache was clamped to MAX_HARD_STALE_MS
    assert_eq!(cache.get_max_stale_ms(), MAX_HARD_STALE_MS);

    // Candidate attempting to exceed background refresh cap (e.g. 10,000)
    let refresh_candidate = PolicyCandidate {
        candidate_id: "ADVERSARIAL_REFRESH".to_string(),
        kind: PolicyKind::RefreshEarly,
        parameters: [("max_background_refresh".to_string(), 10_000.0)]
            .into_iter()
            .collect(),
        bounds: [("max_background_refresh".to_string(), (0.0, 10_000.0))]
            .into_iter()
            .collect(),
        expected_effect: "Excessive refresh".to_string(),
    };

    executor
        .apply_policy(&refresh_candidate)
        .expect("Apply policy failed");
    assert_eq!(
        cache.get_max_background_refresh(),
        MAX_BACKGROUND_REFRESH_CAP
    );
}

#[test]
fn test_revert_to_safe_equality() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = DefaultPolicyExecutor::new(Some(cache.clone()));
    executor.set_mode(IntelligenceMode::Active).unwrap();

    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);

    // Apply S3-FIFO candidate
    let candidate = PolicyCandidate {
        candidate_id: "S3-FIFO".to_string(),
        kind: PolicyKind::Eviction,
        parameters: [].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "Scan filtering".to_string(),
    };
    executor.apply_policy(&candidate).unwrap();
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::S3Fifo);

    // Revert to safe
    executor.revert_to_safe();
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);
    assert_eq!(executor.get_revert_count(), 1);
    assert!(executor.get_active_candidate().is_none());
}

#[test]
fn test_watchdog_stall_triggers_fallback() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = DefaultPolicyExecutor::new(Some(cache.clone()));
    executor.set_mode(IntelligenceMode::Canary).unwrap();

    // Candidate active
    let candidate = PolicyCandidate {
        candidate_id: "LRU".to_string(),
        kind: PolicyKind::Eviction,
        parameters: [].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "LRU testing".to_string(),
    };
    executor.apply_policy(&candidate).unwrap();
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Lru);

    // Record heartbeat at t = 1000 ms
    executor.heartbeat_at(1000);

    // Check at t = 1500 ms with 1000 ms timeout -> false (healthy)
    assert!(!executor.check_watchdog(1500, 1000));

    // Stalled: simulate now_ms at t = 3000 ms -> true (tripped!)
    let tripped = executor.check_watchdog(3000, 1000);
    assert!(tripped);

    // Verify mode dropped to Off and policy reverted to safe Sieve
    assert_eq!(executor.current_mode(), IntelligenceMode::Off);
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);
}

#[test]
fn test_mode_gating_prevents_unsafe_application() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = DefaultPolicyExecutor::new(Some(cache.clone()));

    // Mode is Off by default
    assert_eq!(executor.current_mode(), IntelligenceMode::Off);

    let candidate = PolicyCandidate {
        candidate_id: "LRU".to_string(),
        kind: PolicyKind::Eviction,
        parameters: [].into_iter().collect(),
        bounds: [].into_iter().collect(),
        expected_effect: "LRU".to_string(),
    };

    let res = executor.apply_policy(&candidate);
    assert!(res.is_err());
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);
}

#[test]
fn test_atomic_policy_swap_under_concurrent_load() {
    let cache = Arc::new(PartitionedCache::new(4));
    let executor = Arc::new(DefaultPolicyExecutor::new(Some(cache.clone())));
    executor.set_mode(IntelligenceMode::Active).unwrap();

    // Pre-populate cache with keys
    for i in 0..100 {
        cache.insert(&format!("tenant-a:/item_{}", i), b"DATA".to_vec(), 5000, 1000);
    }

    // Spawn 4 concurrent readers
    let mut handles = Vec::new();
    for _ in 0..4 {
        let c = cache.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..200 {
                let _ = c.get(&format!("tenant-a:/item_{}", i % 100), 1000);
            }
        }));
    }

    // Interleave policy swaps between Sieve and S3-FIFO
    for round in 0..10 {
        let policy_name = if round % 2 == 0 { "S3-FIFO" } else { "LRU" };
        let candidate = PolicyCandidate {
            candidate_id: policy_name.to_string(),
            kind: PolicyKind::Eviction,
            parameters: [].into_iter().collect(),
            bounds: [].into_iter().collect(),
            expected_effect: "Policy swap".to_string(),
        };
        executor.apply_policy(&candidate).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Revert to safe
    executor.revert_to_safe();
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);
}
