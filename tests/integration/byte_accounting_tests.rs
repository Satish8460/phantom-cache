use phantom_cache_demo::core::cache::{
    calculate_key_bytes, calculate_resource_bytes, fixed_entry_overhead,
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::{
    FetchRegistration, FinalizeOutcome, OriginError, OriginFetchGuard, WaiterBatch,
};
use proptest::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn make_resource(body_len: usize, ttl_ms: u64, now_ms: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/octet-stream".to_string(),
        body: Arc::new(vec![0xAA; body_len]),
        etag: Some("etag-test-1".to_string()),
        created_at_ms: now_ms,
        ttl_ms,
    }
}

fn complete_ok(reg: FetchRegistration, res: CachedResource) -> Option<WaiterBatch> {
    reg.into_guard().complete(FinalizeOutcome::Success(Arc::new(res)))
}

fn complete_err(reg: FetchRegistration, err: OriginError) -> Option<WaiterBatch> {
    reg.into_guard().complete(FinalizeOutcome::Error(Arc::new(err)))
}

// ===========================================================================
// 1. Lifecycle Table Unit Tests
// ===========================================================================

#[test]
fn test_lifecycle_placeholder_insert_miss() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/1", None);

    let fixed = fixed_entry_overhead();
    let key_bytes = calculate_key_bytes(&key);
    let expected_placeholder_bytes = key_bytes + fixed;

    // Initially 0
    assert_eq!(cache.bytes_resident(), 0);

    // Initiate placeholder insert
    let reg_res = cache.begin_fetch_atomic(&key, now_ms);
    let reg = match reg_res {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    // Accounted bytes must equal placeholder bytes
    assert_eq!(cache.bytes_resident(), expected_placeholder_bytes);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, expected_placeholder_bytes);
    assert_eq!(recorded, expected_placeholder_bytes);

    // If budget is set below placeholder bytes, next miss is rejected
    cache.set_byte_budget(expected_placeholder_bytes); // exactly fits 1 placeholder
    let key2 = CacheKey::new(Some("t1"), "/item/2", None);
    let reg_res2 = cache.begin_fetch_atomic(&key2, now_ms);
    match reg_res2 {
        FetchRegistrationResult::Rejected(OriginError::CapacityExceeded) => {}
        _ => panic!("Expected CapacityExceeded when exceeding byte budget"),
    }
    assert_eq!(cache.counters.capacity_rejections.load(Ordering::Relaxed), 1);

    // Cleanup first reg
    drop(reg);
}

#[test]
fn test_lifecycle_fill_success() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/fill", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    let res = make_resource(256, 60_000, now_ms);
    let new_res_bytes = calculate_resource_bytes(Some(&res));
    let key_bytes = calculate_key_bytes(&key);
    let fixed = fixed_entry_overhead();
    let total_expected = key_bytes + fixed + new_res_bytes;

    let batch = complete_ok(reg, res);
    assert!(batch.is_some());

    // Accounted bytes equals full state bytes
    assert_eq!(cache.bytes_resident(), total_expected);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, total_expected);
    assert_eq!(recorded, total_expected);

    // Verify oversize rejection
    // Set budget = 500 bytes. Attempt to fill 1000-byte body.
    cache.set_byte_budget(500);
    let key2 = CacheKey::new(Some("t1"), "/item/oversize", None);
    let reg2 = match cache.begin_fetch_atomic(&key2, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    let oversize_res = make_resource(1000, 60_000, now_ms);
    let batch2 = complete_ok(reg2, oversize_res);
    assert!(batch2.is_some(), "Response must still be delivered to waiters");

    // Not stored! Rejection counters incremented.
    assert_eq!(cache.counters.byte_budget_rejections.load(Ordering::Relaxed), 1);
    assert_eq!(cache.counters.oversize_rejections.load(Ordering::Relaxed), 1);

    // Placeholder removed, bytes back to previous entry (or whatever fits)
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, recorded);
}

#[test]
fn test_lifecycle_replacement() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/replace", None);

    // Initial fill: 100 bytes
    let reg1 = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res1 = make_resource(100, 60_000, now_ms);
    complete_ok(reg1, res1);

    let bytes_after_first = cache.bytes_resident();

    // Replacement: 300 bytes (after TTL passes or forced refresh)
    let reg2 = match cache.begin_fetch_atomic(&key, now_ms + 70_000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res2 = make_resource(300, 60_000, now_ms + 70_000);
    complete_ok(reg2, res2);

    // Old resource subtracted, new resource added: delta = 200
    assert_eq!(cache.bytes_resident(), bytes_after_first + 200);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, recorded);
}

#[test]
fn test_lifecycle_failed_insertion() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/fail", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    assert!(cache.bytes_resident() > 0);

    // Complete with Error
    let batch = complete_err(reg, OriginError::Timeout);
    assert!(batch.is_some());

    // Placeholder removed, bytes returns to 0
    assert_eq!(cache.bytes_resident(), 0);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, 0);
    assert_eq!(recorded, 0);
}

#[test]
fn test_lifecycle_expiry() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/expiry", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res = make_resource(100, 500, now_ms); // TTL 500ms
    complete_ok(reg, res);

    let initial_bytes = cache.bytes_resident();

    // At now = 2000ms (TTL expired), entry is still counted until evicted or replaced
    assert_eq!(cache.bytes_resident(), initial_bytes);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, initial_bytes);
    assert_eq!(recorded, initial_bytes);

    // Eviction removes expired entry
    let policy = cache.get_eviction_policy();
    {
        let mut shard = cache.shards[0].lock().unwrap();
        let victim = PartitionedCache::evict_from_shard(&mut shard, policy, 2000, &cache.counters);
        assert_eq!(victim, Some(key));
    }
    assert_eq!(cache.bytes_resident(), 0);
}

#[test]
fn test_lifecycle_invalidation_delete() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/del", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res = make_resource(150, 60_000, now_ms);
    complete_ok(reg, res);

    assert!(cache.bytes_resident() > 0);

    // Delete
    cache.delete(&key);

    assert_eq!(cache.bytes_resident(), 0);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, 0);
    assert_eq!(recorded, 0);
}

#[test]
fn test_lifecycle_epoch_stale_lazy() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("tenant_epoch"), "/item/epoch", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res = make_resource(200, 60_000, now_ms);
    let res_bytes = calculate_resource_bytes(Some(&res));
    let initial_total = calculate_key_bytes(&key) + fixed_entry_overhead() + res_bytes;
    complete_ok(reg, res);

    assert_eq!(cache.bytes_resident(), initial_total);

    // Invalidate tenant epoch
    cache.invalidate_tenant("tenant_epoch");

    // Before lazy touch: still resident and counted
    assert_eq!(cache.bytes_resident(), initial_total);

    // Touch via get_resource: detects epoch stale, sets entry = None, subtracts res_bytes
    let get_res = cache.get_resource(&key, now_ms);
    assert!(get_res.is_none());

    let expected_after = calculate_key_bytes(&key) + fixed_entry_overhead();
    assert_eq!(cache.bytes_resident(), expected_after);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, expected_after);
    assert_eq!(recorded, expected_after);
}

#[test]
fn test_single_arc_per_entry() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;
    let key = CacheKey::new(Some("t1"), "/item/single_arc", None);

    let reg = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    let body_arc = Arc::new(vec![1, 2, 3, 4]);
    let initial_strong = Arc::strong_count(&body_arc);

    let res = CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: body_arc.clone(),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    };

    let mut batch = complete_ok(reg, res);
    assert!(batch.is_some());
    // Waiter batch delivers and is dropped
    batch.as_mut().unwrap().deliver();
    drop(batch);

    // In cache: entry owns exactly one clone of the body Arc
    // (body_arc local variable + cache entry)
    assert_eq!(Arc::strong_count(&body_arc), initial_strong + 1);
}

// ===========================================================================
// 2. Contraction / Excess Suite (T-C1 through T-C5)
// ===========================================================================

#[test]
fn test_t_c1_contraction_no_in_flight() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;

    // Fill 10 entries of 100 bytes each
    for i in 0..10 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c1_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            complete_ok(reg, make_resource(100, 60_000, now_ms));
        }
    }

    let initial_bytes = cache.bytes_resident();
    assert!(initial_bytes > 0);

    // Contract budget to half of initial bytes
    let new_budget = initial_bytes / 2;
    cache.set_byte_budget(new_budget);

    // I2' first branch: bytes <= B_shard immediately
    assert!(cache.bytes_resident() <= new_budget);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, recorded);
    assert!(cache.counters.evictions_total.load(Ordering::Relaxed) > 0);
}

#[test]
fn test_t_c2_contraction_with_in_flight_ops() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;

    // Fill 3 completed entries
    for i in 0..3 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c2_done_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            complete_ok(reg, make_resource(100, 60_000, now_ms));
        }
    }

    // Initiate 3 in-flight fetches and keep their guards
    let mut guards: Vec<OriginFetchGuard> = Vec::new();
    let mut pinned_bytes = 0;
    for i in 0..3 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c2_inflight_{}", i), None);
        let key_bytes = calculate_key_bytes(&key);
        pinned_bytes += key_bytes + fixed_entry_overhead();
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            guards.push(reg.into_guard());
        }
    }

    assert_eq!(guards.len(), 3);

    // Contract budget below pinned_bytes
    let tiny_budget = pinned_bytes / 2;
    cache.set_byte_budget(tiny_budget);

    // I2' second branch: bytes == pinned_bytes > B_shard (excess regime)
    assert_eq!(cache.bytes_resident(), pinned_bytes);
    assert!(cache.bytes_resident() > tiny_budget);

    // Verify I5: None of the pinned states were evicted
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, pinned_bytes);
    assert_eq!(recorded, pinned_bytes);

    // Cleanup: complete with error
    for guard in guards {
        guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::Aborted)));
    }
}

#[test]
fn test_t_c3_excess_regime_operations() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;

    // Initiate 3 in-flight fetches to hold pinned states
    let mut guards: Vec<OriginFetchGuard> = Vec::new();
    let mut pinned_bytes = 0;
    for i in 0..3 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c3_{}", i), None);
        pinned_bytes += calculate_key_bytes(&key) + fixed_entry_overhead();
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            guards.push(reg.into_guard());
        }
    }

    // Contract into excess regime
    cache.set_byte_budget(pinned_bytes / 2);
    let excess_bytes_before = cache.bytes_resident();
    assert_eq!(excess_bytes_before, pinned_bytes);

    // 1. Miss on another key => CapacityExceeded, bytes unchanged (I7)
    let key_miss = CacheKey::new(Some("t1"), "/item/c3_other", None);
    let miss_res = cache.begin_fetch_atomic(&key_miss, now_ms);
    match miss_res {
        FetchRegistrationResult::Rejected(OriginError::CapacityExceeded) => {}
        _ => panic!("Expected CapacityExceeded during excess regime"),
    }
    assert_eq!(cache.bytes_resident(), excess_bytes_before);

    // 2. Fill for an in-flight key that would add bytes: response delivered, not stored (I7)
    let guard0 = guards.remove(0);
    let batch = guard0.complete(FinalizeOutcome::Success(Arc::new(make_resource(500, 60_000, now_ms))));
    assert!(batch.is_some(), "Response must be delivered to waiters");
    assert_eq!(cache.counters.byte_budget_rejections.load(Ordering::Relaxed), 1);

    // Placeholder was removed because no active ops remained and not stored,
    // so bytes decreased by placeholder's bytes!
    assert!(cache.bytes_resident() < excess_bytes_before);

    // Cleanup remaining
    for guard in guards {
        guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::Aborted)));
    }
}

#[test]
fn test_t_c4_recovery_convergence() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;

    let mut guards: Vec<OriginFetchGuard> = Vec::new();
    let mut pinned_bytes = 0;
    for i in 0..4 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c4_{}", i), None);
        pinned_bytes += calculate_key_bytes(&key) + fixed_entry_overhead();
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            guards.push(reg.into_guard());
        }
    }

    // Set budget to allow 1 item but less than 4 items (excess regime)
    let budget = pinned_bytes / 2;
    cache.set_byte_budget(budget);
    assert!(cache.bytes_resident() > budget); // In excess

    let mut prev_bytes = cache.bytes_resident();

    // Complete ops one by one (different variants: Error, Success-not-stored, guard drop)
    // Variant 1: Error
    let guard0 = guards.remove(0);
    guard0.complete(FinalizeOutcome::Error(Arc::new(OriginError::Timeout)));
    assert!(cache.bytes_resident() <= prev_bytes);
    prev_bytes = cache.bytes_resident();

    // Variant 2: Success (would add bytes, rejected from storage)
    let guard1 = guards.remove(0);
    guard1.complete(FinalizeOutcome::Success(Arc::new(make_resource(500, 60_000, now_ms))));
    assert!(cache.bytes_resident() <= prev_bytes);
    prev_bytes = cache.bytes_resident();

    // Variant 3: Guard drop (aborted)
    let guard2 = guards.remove(0);
    drop(guard2);
    assert!(cache.bytes_resident() <= prev_bytes);
    prev_bytes = cache.bytes_resident();

    // Variant 4: Last op drops
    let guard3 = guards.remove(0);
    drop(guard3);
    assert!(cache.bytes_resident() <= prev_bytes);

    // Now all pins released: shrink step ensures bytes <= budget (I8 convergence)
    assert!(cache.bytes_resident() <= budget);
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, recorded);

    // Normal admissions resume
    let key_new = CacheKey::new(Some("t1"), "/item/c4_after", None);
    let res = cache.begin_fetch_atomic(&key_new, now_ms);
    match res {
        FetchRegistrationResult::Initiate(r) => drop(r),
        _ => panic!("Expected Initiate after recovery"),
    }
}

#[test]
fn test_t_c5_contraction_followed_by_expansion() {
    let cache = Arc::new(PartitionedCache::new(1));
    let now_ms = 1000;

    let mut guards: Vec<OriginFetchGuard> = Vec::new();
    for i in 0..3 {
        let key = CacheKey::new(Some("t1"), &format!("/item/c5_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            guards.push(reg.into_guard());
        }
    }

    let pinned = cache.bytes_resident();
    cache.set_byte_budget(pinned / 2); // Contraction into excess
    assert_eq!(cache.bytes_resident(), pinned);

    let evictions_before = cache.counters.evictions_total.load(Ordering::Relaxed);

    // Re-expansion
    cache.set_byte_budget(pinned * 10);

    // No evictions occurred on expansion
    assert_eq!(cache.counters.evictions_total.load(Ordering::Relaxed), evictions_before);
    assert!(cache.bytes_resident() <= cache.byte_budget());

    // Admissions resume immediately
    let key_new = CacheKey::new(Some("t1"), "/item/c5_new", None);
    match cache.begin_fetch_atomic(&key_new, now_ms) {
        FetchRegistrationResult::Initiate(r) => drop(r),
        _ => panic!("Expected Initiate"),
    }

    for guard in guards {
        guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::Aborted)));
    }
}

// ===========================================================================
// 3. Concurrent Stress (T-C6)
// ===========================================================================

#[test]
fn test_t_c6_concurrent_stress() {
    let cache = Arc::new(PartitionedCache::new(1));
    let start_time = std::time::Instant::now();
    let duration = Duration::from_millis(500);

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let cache_c = cache.clone();
            std::thread::spawn(move || {
                let mut step = 0;
                while start_time.elapsed() < duration {
                    step += 1;
                    let now_ms = 1000 + step;
                    let key = CacheKey::new(Some("t1"), &format!("/stress/{}_{}", thread_id, step % 20), None);

                    match cache_c.begin_fetch_atomic(&key, now_ms) {
                        FetchRegistrationResult::Initiate(reg) => {
                            if step % 2 == 0 {
                                complete_ok(reg, make_resource(64, 5000, now_ms));
                            } else {
                                complete_err(reg, OriginError::Timeout);
                            }
                        }
                        FetchRegistrationResult::Hit(_) => {}
                        FetchRegistrationResult::Wait(_) => {}
                        FetchRegistrationResult::Rejected(_) => {}
                    }

                    if step % 5 == 0 {
                        cache_c.delete(&key);
                    }

                    if step % 25 == 0 {
                        // Periodic contraction / expansion
                        let budget = if step % 50 == 0 { 2000 } else { 10_000 };
                        cache_c.set_byte_budget(budget);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // Quiescent verification: I1 holds
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, recorded, "Invariant I1 failed after concurrent stress");
}

// ===========================================================================
// 4. Property Tests (T-C7)
// ===========================================================================

#[derive(Clone, Debug)]
enum Op {
    Initiate(u8),
    CompleteSuccess(u8, usize),
    CompleteError(u8),
    Delete(u8),
    Contract(usize),
    Expand(usize),
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn test_t_c7_property_invariants(ops in prop::collection::vec(
        prop_oneof![
            (0u8..10).prop_map(Op::Initiate),
            (0u8..10, 16usize..256).prop_map(|(k, s)| Op::CompleteSuccess(k, s)),
            (0u8..10).prop_map(Op::CompleteError),
            (0u8..10).prop_map(Op::Delete),
            (200usize..1000).prop_map(Op::Contract),
            (2000usize..10000).prop_map(Op::Expand),
        ],
        10..60
    )) {
        let cache = Arc::new(PartitionedCache::new(1));
        let mut active_guards = std::collections::HashMap::new();
        let now_ms = 1000;

        for op in ops {
            match op {
                Op::Initiate(k) => {
                    let key = CacheKey::new(Some("prop"), &format!("/p_{}", k), None);
                    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
                        active_guards.insert(k, reg.into_guard());
                    }
                }
                Op::CompleteSuccess(k, sz) => {
                    if let Some(guard) = active_guards.remove(&k) {
                        guard.complete(FinalizeOutcome::Success(Arc::new(make_resource(sz, 60_000, now_ms))));
                    }
                }
                Op::CompleteError(k) => {
                    if let Some(guard) = active_guards.remove(&k) {
                        guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::Timeout)));
                    }
                }
                Op::Delete(k) => {
                    let key = CacheKey::new(Some("prop"), &format!("/p_{}", k), None);
                    cache.delete(&key);
                }
                Op::Contract(budget) => {
                    cache.set_byte_budget(budget);
                }
                Op::Expand(budget) => {
                    cache.set_byte_budget(budget);
                }
            }

            // Assert Invariant I1 at every step
            let (accounted, recorded) = cache.audit_bytes();
            prop_assert_eq!(accounted, recorded, "Invariant I1 violated during step");
        }

        // Quiescence: complete all remaining guards
        for (_, guard) in active_guards.drain() {
            drop(guard);
        }

        // Final quiescence check
        let (accounted, recorded) = cache.audit_bytes();
        prop_assert_eq!(accounted, recorded, "Invariant I1 violated at quiescence");
        prop_assert!(recorded <= cache.byte_budget(), "Invariant I8 violated at quiescence: {} > {}", recorded, cache.byte_budget());
    }
}

// ===========================================================================
// 5. Adversarial Large-Body Workload Integration Test
// ===========================================================================

#[test]
fn test_large_body_adversarial_bounded_accounting() {
    let cache = Arc::new(PartitionedCache::new(1));
    let shard_budget = 32 * 1024 * 1024; // 32 MiB
    cache.set_byte_budget(shard_budget);

    let one_mib = 1024 * 1024;
    let now_ms = 1000;

    // Fill 50 entries with 1 MiB bodies
    for i in 0..50 {
        let key = CacheKey::new(Some("t1"), &format!("/large/body_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            let res = make_resource(one_mib, 60_000, now_ms);
            let _ = complete_ok(reg, res);
        }

        // Must never exceed the configured byte budget
        assert!(
            cache.bytes_resident() <= shard_budget,
            "Accounted bytes {} exceeded shard budget {} at item {}",
            cache.bytes_resident(),
            shard_budget,
            i
        );

        let (accounted, recorded) = cache.audit_bytes();
        assert_eq!(accounted, recorded);
    }

    // Evictions must have occurred
    assert!(cache.counters.evictions_total.load(Ordering::Relaxed) > 0);
}
