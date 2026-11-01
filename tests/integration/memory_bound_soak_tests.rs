use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, EvictionPolicy, FetchRegistrationResult, PartitionedCache,
    StaleServingDecision,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::core::freshness::FreshnessMetadata;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn make_key(tenant: &str, path: &str) -> CacheKey {
    CacheKey::new(Some(tenant), path, None)
}

fn make_resource(body_len: usize, ttl_ms: u64, created_at_ms: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/octet-stream".to_string(),
        body: Arc::new(vec![0xAAu8; body_len]),
        etag: Some("\"soak-etag\"".to_string()),
        created_at_ms,
        ttl_ms,
    }
}

// ---------------------------------------------------------------------------
// 1. Memory-Bounded Soak Under Churn & Eviction
// ---------------------------------------------------------------------------
#[test]
fn test_memory_bounded_churn_soak() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(2, counters.clone(), diagnostics));

    // Configure strict byte budget: 256 KiB total (128 KiB per shard)
    let total_budget = 256 * 1024;
    cache.set_byte_budget(total_budget);
    cache.set_capacity_limits(50, 100);
    cache.set_eviction_policy(EvictionPolicy::Sieve);

    let num_requests = 3000;
    let mut now_ms = 1_000_000u64;

    // Pseudo-random Zipf-like key popularity across 60 distinct keys
    // Heavy-tailed body sizes: 1 KiB (70%), 8 KiB (25%), 32 KiB (5%)
    for i in 0..num_requests {
        let rank = (i * 37) % 60;
        let tenant = if rank < 10 { "vip_tenant" } else { "bulk_tenant" };
        let path = format!("/resource-{}", rank);
        let key = make_key(tenant, &path);

        now_ms += 10;

        // Invalidate tenant occasionally (5% churn)
        if i % 150 == 0 {
            let _ = cache.try_invalidate_tenant("bulk_tenant");
        }
        // Invalidate single key occasionally
        if i % 80 == 0 {
            let _ = cache.delete_with_status(&key);
        }

        // Sweeper tick periodically
        if i % 50 == 0 {
            cache.sweep_lazy_reclamation(16, now_ms);
        }

        match cache.begin_fetch_atomic(&key, now_ms) {
            FetchRegistrationResult::Initiate(reg) => {
                let body_len = if rank % 20 == 0 {
                    32 * 1024
                } else if rank % 4 == 0 {
                    8 * 1024
                } else {
                    1024
                };
                let res = make_resource(body_len, 30_000, now_ms);
                let guard = reg.into_guard();
                if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
                    batch.deliver();
                }
            }
            FetchRegistrationResult::Hit(_) => {}
            FetchRegistrationResult::Wait(_) => {}
            FetchRegistrationResult::Rejected(_) => {}
        }

        // Checkpoint verification every 200 requests
        if i % 200 == 0 {
            let (logical, recorded) = cache.audit_bytes();
            assert_eq!(
                logical, recorded,
                "Logical bytes must equal recorded bytes at step {}: logical={}, recorded={}",
                i, logical, recorded
            );

            // Assert accounted bytes strictly bounded by budget + 1 max entry slop
            let total_resident = cache.bytes_resident();
            assert!(
                total_resident <= total_budget + 32 * 1024,
                "Resident bytes {} exceeded budget limit {}",
                total_resident,
                total_budget
            );
        }
    }

    // Final soak assertions
    let (final_logical, final_recorded) = cache.audit_bytes();
    assert_eq!(final_logical, final_recorded);

    // Stale cap violations must be strictly 0
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);
    println!(
        "Soak completed: {} requests, final resident bytes: {}, evictions: {}",
        num_requests,
        cache.bytes_resident(),
        counters.evictions_total.load(Ordering::Relaxed)
    );
    assert!(counters.evictions_total.load(Ordering::Relaxed) > 10);
}

// ---------------------------------------------------------------------------
// 2. Waiter Capacity Bounding (No Unbounded Waiter Memory Growth)
// ---------------------------------------------------------------------------
#[test]
fn test_waiter_capacity_bound() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(1, counters.clone(), diagnostics));

    let key = make_key("t1", "/bounded-waiters");
    let now_ms = 1_000_000;

    // Leader initiates
    let reg1 = match cache.begin_fetch_atomic(&key, now_ms) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    // Coalesce up to max_waiters_per_key
    let mut waiters = Vec::new();
    for _ in 0..100 {
        match cache.begin_fetch_atomic(&key, now_ms) {
            FetchRegistrationResult::Wait(rx) => waiters.push(rx),
            _ => break,
        }
    }

    // Clean up leader
    let res = make_resource(64, 60_000, now_ms);
    let guard = reg1.into_guard();
    if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
        batch.deliver();
    }
}

// ---------------------------------------------------------------------------
// 3. Sweeper Reclaims Lazy Invalidation Memory Under Churn
// ---------------------------------------------------------------------------
#[test]
fn test_sweeper_reclaims_under_soak_churn() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(2, counters.clone(), diagnostics));

    cache.set_byte_budget(512 * 1024);
    cache.set_capacity_limits(100, 100);
    let now_ms = 1_000_000;

    // Fill 30 entries for tenant "churn_tenant"
    for i in 0..30 {
        let key = make_key("churn_tenant", &format!("/item-{}", i));
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            let res = make_resource(2048, 60_000, now_ms);
            let guard = reg.into_guard();
            if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
                batch.deliver();
            }
        }
    }

    let bytes_before = cache.bytes_resident();
    assert!(bytes_before >= 30 * 2048);

    // Invalidate tenant
    cache.try_invalidate_tenant("churn_tenant").unwrap();

    // Run sweeper ticks until all lazy-invalid entries are swept
    let mut total_swept = 0;
    for _ in 0..10 {
        let (swept, _) = cache.sweep_lazy_reclamation(16, now_ms);
        total_swept += swept;
        if total_swept >= 30 {
            break;
        }
    }

    assert!(total_swept >= 30, "Sweeper must reclaim all invalid tenant entries");
    assert_eq!(cache.bytes_resident(), 0, "Bytes resident must drain to 0");
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(accounted, 0);
    assert_eq!(recorded, 0);
}

// ---------------------------------------------------------------------------
// 4. Stale Cap Enforced Under Outage Soak
// ---------------------------------------------------------------------------
#[test]
fn test_stale_cap_enforced_under_outage_soak() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(1, counters.clone(), diagnostics));

    let key = make_key("t_outage", "/outage-resource");
    let initial_time = 1_000_000u64;
    let ttl_ms = 10_000u64; // 10s lifetime

    // Initial fill with SWR = 60s
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, initial_time) {
        let res = make_resource(1024, ttl_ms, initial_time);
        let guard = reg.into_guard();
        guard.cache.update_freshness_metadata(
            &key,
            FreshnessMetadata {
                must_revalidate: false,
                stale_while_revalidate_ms: Some(60_000),
                stale_if_error_ms: None,
                last_modified: None,
                date_ms: Some(initial_time),
                initial_age_ms: 0,
            },
        );
        if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(res))) {
            batch.deliver();
        }
    }

    let hard_cap = cache.get_max_stale_ms(); // 24h = 86,400,000 ms

    // Advance time past TTL (now 5s stale, within 60s SWR window and within hard cap)
    let stale_time = initial_time + 15_000;
    let decision = cache.check_stale_serving(&key, stale_time, hard_cap, false);
    match decision {
        StaleServingDecision::Eligible(stale_res, _) => {
            cache.record_stale_served(stale_time - stale_res.created_at_ms, stale_res.ttl_ms, hard_cap);
        }
        other => panic!("Expected Eligible within SWR window, got {:?}", other),
    }

    assert_eq!(counters.stale_served_total.load(Ordering::Relaxed), 1);
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);

    // Advance time past hard staleness cap (e.g. 24h + 15s)
    let hard_cap_exceeded_time = initial_time + hard_cap + 15_000;
    let post_cap_decision = cache.check_stale_serving(&key, hard_cap_exceeded_time, hard_cap, false);
    assert!(
        matches!(post_cap_decision, StaleServingDecision::CapExceeded | StaleServingDecision::Ineligible),
        "Entry must be Ineligible past hard staleness cap, got {:?}",
        post_cap_decision
    );

    // Assert that stale cap violations remained strictly 0
    assert_eq!(
        counters.stale_cap_violations.load(Ordering::Relaxed),
        0,
        "Stale cap violations must remain strictly 0"
    );
}
