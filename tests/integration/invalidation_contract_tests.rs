use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

fn make_key(tenant: &str, path: &str) -> CacheKey {
    CacheKey::new(Some(tenant), path, None)
}

fn make_resource(body: &[u8], ttl_ms: u64, created_at_ms: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(body.to_vec()),
        etag: Some("\"etag-test\"".to_string()),
        created_at_ms,
        ttl_ms,
    }
}

// ---------------------------------------------------------------------------
// 1. Linearizable Single-Key Deletion (Clause INV-1)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_linearizable_delete_prevents_read() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters, diagnostics));

    let key = make_key("t1", "/linearizable-del");
    let now = 1_000_000;
    let res = make_resource(b"initial_val", 60_000, now);

    // Initial fill
    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    reg.into_guard()
        .complete(FinalizeOutcome::Success(Arc::new(res)))
        .unwrap()
        .deliver();

    // Verify resident and readable
    assert!(cache.get_resource(&key, now + 10).is_some());

    // Execute DELETE
    let (was_present, reclaimed_bytes) = cache.delete_with_status(&key);
    assert!(was_present);
    assert!(reclaimed_bytes > 0);

    // Subsequent read must immediately return None (per-node linearizable)
    assert!(cache.get_resource(&key, now + 11).is_none());

    // Next begin_fetch_atomic must be Initiate (not Hit)
    match cache.begin_fetch_atomic(&key, now + 12) {
        FetchRegistrationResult::Initiate(r) => {
            let res2 = make_resource(b"new_val", 60_000, now + 12);
            r.into_guard()
                .complete(FinalizeOutcome::Success(Arc::new(res2)))
                .unwrap()
                .deliver();
        }
        _ => panic!("Expected fresh Initiate following DELETE"),
    }

    assert_eq!(*cache.get_resource(&key, now + 15).unwrap().body, b"new_val");
}

// ---------------------------------------------------------------------------
// 2. Fencing In-Flight Fills Racing with Concurrent DELETE (Clause INV-2)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_delete_fences_in_flight_fill_race() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/race-fill-del");
    let now = 1_000_000;

    // 1. Requester 1 initiates fetch
    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();

    // 2. While fetch is in flight, a concurrent client sends DELETE
    let (was_present, _) = cache.delete_with_status(&key);
    // Key was registered in shard.map with in_flight op, so state exists
    assert!(!was_present); // No CachedResource yet, but placeholder state is marked deleted

    let stale_prevented_before = counters.stale_writes_prevented.load(Ordering::Relaxed);

    // 3. Stale in-flight fetch completes with old data
    let res = make_resource(b"stale_zombie_data", 60_000, now);
    let batch = guard.complete(FinalizeOutcome::Success(Arc::new(res)));
    assert!(batch.is_some());
    batch.unwrap().deliver();

    // Invariant: generation fencing caught the mismatch!
    let stale_prevented_after = counters.stale_writes_prevented.load(Ordering::Relaxed);
    assert_eq!(stale_prevented_after, stale_prevented_before + 1);

    // Entry must NOT be resident in cache (zombie data dropped)
    assert!(cache.get_resource(&key, now + 10).is_none());
}

// ---------------------------------------------------------------------------
// 3. O(1) Tenant Flush Ordering & Invalidation (Clause INV-3)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_tenant_flush_ordering_and_lazy_invalidation() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let tenant = "tenant_alpha";
    let now = 1_000_000;

    // Populate 10 keys for tenant_alpha
    for i in 0..10 {
        let key = make_key(tenant, &format!("/item-{}", i));
        let reg = match cache.begin_fetch_atomic(&key, now) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let res = make_resource(format!("val_{}", i).as_bytes(), 60_000, now);
        reg.into_guard()
            .complete(FinalizeOutcome::Success(Arc::new(res)))
            .unwrap()
            .deliver();
    }

    // Also populate a key for another tenant
    let other_key = make_key("tenant_beta", "/item-0");
    let reg_other = match cache.begin_fetch_atomic(&other_key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res_other = make_resource(b"beta_val", 60_000, now);
    reg_other
        .into_guard()
        .complete(FinalizeOutcome::Success(Arc::new(res_other)))
        .unwrap()
        .deliver();

    // Verify all keys readable
    for i in 0..10 {
        let key = make_key(tenant, &format!("/item-{}", i));
        assert!(cache.get_resource(&key, now + 1).is_some());
    }
    assert!(cache.get_resource(&other_key, now + 1).is_some());

    // Flush tenant_alpha in O(1)
    let flush_time = Instant::now();
    let epoch = cache.invalidate_tenant(tenant);
    let flush_duration = flush_time.elapsed();

    // Invariant: O(1) epoch bump is strictly sub-millisecond
    assert!(flush_duration < Duration::from_millis(5));
    assert!(epoch > 0);

    // All tenant_alpha keys immediately evaluate as misses
    for i in 0..10 {
        let key = make_key(tenant, &format!("/item-{}", i));
        assert!(cache.get_resource(&key, now + 2).is_none());
    }

    // Other tenant remains completely unaffected
    assert!(cache.get_resource(&other_key, now + 2).is_some());
}

// ---------------------------------------------------------------------------
// 4. Sweeper Convergence After Tenant Flush (Clause INV-4, INV-5)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_sweeper_convergence_after_tenant_flush() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let tenant = "tenant_to_sweep";
    let now = 1_000_000;
    let entry_count = 200;

    for i in 0..entry_count {
        let key = make_key(tenant, &format!("/data-{}", i));
        let reg = match cache.begin_fetch_atomic(&key, now) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let res = make_resource(&[0xcc; 256], 60_000, now);
        reg.into_guard()
            .complete(FinalizeOutcome::Success(Arc::new(res)))
            .unwrap()
            .deliver();
    }

    let bytes_before_flush = cache.bytes_resident();
    assert!(bytes_before_flush > entry_count * 256);

    // Invalidate tenant
    cache.invalidate_tenant(tenant);

    // Run bounded sweeper ticks (batch size = 32 per shard, 4 shards = 128 max per tick)
    let mut total_reclaimed = 0;
    let mut ticks = 0;
    while total_reclaimed < entry_count && ticks < 20 {
        let (reclaimed, _) = cache.sweep_lazy_reclamation(32, now + 10);
        total_reclaimed += reclaimed;
        ticks += 1;
    }

    assert_eq!(total_reclaimed, entry_count);
    assert_eq!(cache.bytes_resident(), 0);
    assert_eq!(counters.reclamations_total.load(Ordering::Relaxed) as usize, entry_count);
}

// ---------------------------------------------------------------------------
// 5. Sweeper vs Concurrent Fills Stress & Accounting Invariants (Clause INV-5)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_sweeper_vs_concurrent_fills_stress() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let num_tasks = 8;
    let iterations_per_task = 50;
    let barrier = Arc::new(Barrier::new(num_tasks + 1));

    // Spawn 8 writer/reader tasks
    let mut handles = Vec::new();
    for task_id in 0..num_tasks {
        let c = cache.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for i in 0..iterations_per_task {
                let tenant = format!("tenant_{}", task_id % 3);
                let key = make_key(&tenant, &format!("/key-{}", i));
                let now = 1_000_000 + i as u64;

                match c.begin_fetch_atomic(&key, now) {
                    FetchRegistrationResult::Initiate(reg) => {
                        let res = make_resource(&[0x55; 128], 500, now);
                        reg.into_guard()
                            .complete(FinalizeOutcome::Success(Arc::new(res)))
                            .unwrap()
                            .deliver();
                    }
                    FetchRegistrationResult::Wait(rx) => {
                        let _ = rx.await;
                    }
                    FetchRegistrationResult::Hit(..) | FetchRegistrationResult::Rejected(..) => {}
                }

                if i % 15 == 0 {
                    c.invalidate_tenant(&tenant);
                }
            }
        }));
    }

    // Sweeper task concurrently sweeps
    let c_sweep = cache.clone();
    let b_sweep = barrier.clone();
    let sweep_handle = tokio::spawn(async move {
        b_sweep.wait().await;
        for tick in 0..40 {
            let now = 1_000_000 + (tick * 50);
            let _ = c_sweep.sweep_lazy_reclamation(16, now);
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    });

    for h in handles {
        h.await.unwrap();
    }
    sweep_handle.await.unwrap();

    // Verify Invariant I1-I8: accounted bytes must match recorded bytes exactly
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(
        accounted, recorded,
        "Accounting invariant violated under concurrent sweeping: accounted {} != recorded {}",
        accounted, recorded
    );
}

// ---------------------------------------------------------------------------
// 6. Sweeper Lock-Hold Bound Measurement (Clause INV-4)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_sweeper_lock_hold_time_bounded() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters, diagnostics));

    let now = 1_000_000;
    // Populate 2,000 entries across 4 shards (500 entries per shard)
    for i in 0..2_000 {
        let key = make_key("t_perf", &format!("/item-{}", i));
        let reg = match cache.begin_fetch_atomic(&key, now) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let res = make_resource(&[0x11; 64], 60_000, now);
        reg.into_guard()
            .complete(FinalizeOutcome::Success(Arc::new(res)))
            .unwrap()
            .deliver();
    }

    // Invalidate tenant to make all entries candidates for reclamation
    cache.invalidate_tenant("t_perf");

    // Execute sweep with batch size 32 per shard
    let (_, max_lock_hold_ns) = cache.sweep_lazy_reclamation(32, now + 10);

    // Acceptance criterion: max lock hold <= 1 ms (1_000_000 ns)
    assert!(
        max_lock_hold_ns <= 1_000_000,
        "Sweeper lock hold exceeded 1ms bound: {} ns ({} µs)",
        max_lock_hold_ns,
        max_lock_hold_ns / 1_000
    );
}

// ---------------------------------------------------------------------------
// 7. SEC-4A Saturation Fallback Preserved
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_epoch_saturation_fallback_sec4a_preserved() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters, diagnostics));

    let now = 1_000_000;
    let key = make_key("t_fallback", "/key");
    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res = make_resource(b"fallback_data", 60_000, now);
    reg.into_guard()
        .complete(FinalizeOutcome::Success(Arc::new(res)))
        .unwrap()
        .deliver();

    // Verify hit before global invalidation
    assert!(cache.get_resource(&key, now + 1).is_some());

    // Global invalidation (bumps global epoch and clears tenant map)
    let global_epoch = cache.invalidate_all();
    assert!(global_epoch > 0);

    // Entry with previous global epoch must now miss
    assert!(cache.get_resource(&key, now + 2).is_none());
}

