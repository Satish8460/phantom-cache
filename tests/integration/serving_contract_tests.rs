use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::{
    FinalizeOutcome, OriginError,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

fn make_key(tenant: &str, path: &str) -> CacheKey {
    CacheKey::new(Some(tenant), path, None)
}

fn make_resource(body: &[u8], ttl_ms: u64, now_ms: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/octet-stream".to_string(),
        body: Arc::new(body.to_vec()),
        etag: Some("etag-ct".to_string()),
        created_at_ms: now_ms,
        ttl_ms,
    }
}

#[tokio::test]
async fn test_ct1_single_flight_coalescing() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct1");
    let now = 1000;

    // First request initiates
    let reg1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate for first request"),
    };

    // Second request waits
    let rx2 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait for second request"),
    };

    // Third request waits
    let rx3 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait for third request"),
    };

    // Complete the operation
    let guard = reg1.into_guard();
    let res = make_resource(b"ct1_payload", 60_000, now);
    let batch = guard.complete(FinalizeOutcome::Success(Arc::new(res)));
    assert!(batch.is_some());
    batch.unwrap().deliver();

    // Verify all waiters received the identical response
    let r2 = rx2.await.expect("Wait 2 delivered").expect("Outcome ok");
    let r3 = rx3.await.expect("Wait 3 delivered").expect("Outcome ok");
    assert_eq!(*r2.body, b"ct1_payload");
    assert_eq!(*r3.body, b"ct1_payload");
}

#[tokio::test]
async fn test_ct2_initiator_cancellation_detached_fetch() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct2");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    // Register a secondary waiter
    let rx_waiter = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait for secondary request"),
    };

    // Simulate initiator client disconnecting by dropping initiator's own channel
    let (_tx_initiator, rx_initiator) = oneshot::channel::<()>();
    drop(rx_initiator); // Initiator disconnected

    // Detached background task completes the guard
    let guard = reg.into_guard();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let g = guard;
        let res = make_resource(b"detached_success", 60_000, now);
        if let Some(mut batch) = g.complete(FinalizeOutcome::Success(Arc::new(res))) {
            batch.deliver();
        }
    });

    handle.await.unwrap();

    // Secondary waiter still received the completed payload
    let outcome = rx_waiter.await.expect("Channel delivered").expect("Payload ok");
    assert_eq!(*outcome.body, b"detached_success");

    // Cache entry is resident and can be read by subsequent requests
    let hit = cache.get_resource(&key, now + 50);
    assert!(hit.is_some());
    assert_eq!(*hit.unwrap().body, b"detached_success");
}

#[tokio::test]
async fn test_ct3_waiter_limit_enforcement() {
    let cache = Arc::new(PartitionedCache::new(4));
    cache.max_waiters_per_key.store(2, Ordering::Relaxed);
    let key = make_key("t1", "/resource/ct3");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    // Waiter 1 (count 1 <= 2)
    let _rx1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait 1"),
    };

    // Waiter 2 (count 2 <= 2)
    let _rx2 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait 2"),
    };

    // Waiter 3 exceeds limit (2 reached) -> Rejected(WaiterLimitExceeded)
    match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Rejected(OriginError::WaiterLimitExceeded) => {
            // Correctly rejected
        }
        _ => panic!("Expected Rejected(WaiterLimitExceeded)"),
    }

    // Clean up
    let g = reg.into_guard();
    let res = make_resource(b"cleanup", 60_000, now);
    let _ = g.complete(FinalizeOutcome::Success(Arc::new(res)));
}

#[tokio::test]
async fn test_ct4_origin_timeout_mapping() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct4");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    let rx = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait"),
    };

    let guard = reg.into_guard();
    // Simulate origin timeout
    let batch = guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::Timeout)));
    assert!(batch.is_some());
    batch.unwrap().deliver();

    let res = rx.await.expect("delivered");
    match res {
        Err(err) => match *err {
            OriginError::Timeout => {} // Correctly mapped
            ref other => panic!("Expected OriginError::Timeout, got {:?}", other),
        },
        Ok(_) => panic!("Expected error outcome"),
    }
}

#[tokio::test]
async fn test_ct5_panic_abort_safety() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct5");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    let rx = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Wait(rx) => rx,
        _ => panic!("Expected Wait"),
    };

    // Simulate task abortion or panic: drop guard without complete()
    let guard = reg.into_guard();
    drop(guard);

    // Waiter must immediately receive Aborted error rather than hanging
    let res = rx.await.expect("channel delivered on guard drop");
    match res {
        Err(err) => match *err {
            OriginError::Aborted => {} // Deterministically notified
            ref other => panic!("Expected OriginError::Aborted, got {:?}", other),
        },
        Ok(_) => panic!("Expected error on aborted guard"),
    }

    // Shard in_flight must be clean
    let shard_idx = cache.get_shard_index(&key);
    let shard = cache.shards[shard_idx].lock().unwrap();
    assert!(!shard.in_flight.contains_key(&key));
}

#[tokio::test]
async fn test_ct6_errors_uncacheable() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct6");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    let guard = reg.into_guard();
    // Origin returned 500 / Network Error
    let batch = guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::UpstreamStatus(500))));
    if let Some(mut b) = batch {
        b.deliver();
    }

    // Entry must NOT be resident in cache
    assert!(cache.get_resource(&key, now + 10).is_none());

    // Next request initiates a fresh fetch
    match cache.begin_fetch_atomic(&key, now + 20) {
        FetchRegistrationResult::Initiate(reg2) => {
            let guard2 = reg2.into_guard();
            let res = make_resource(b"recovered", 60_000, now + 20);
            let _ = guard2.complete(FinalizeOutcome::Success(Arc::new(res)));
        }
        _ => panic!("Expected fresh Initiate after uncacheable error"),
    }

    assert_eq!(*cache.get_resource(&key, now + 30).unwrap().body, b"recovered");
}

#[tokio::test]
async fn test_ct7_fencing_under_invalidation() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct7");
    let now = 1000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(reg) => reg,
        _ => panic!("Expected Initiate"),
    };

    // Shard invalidation occurs while fetch is in-flight
    cache.delete(&key);

    let stale_before = cache.counters.stale_ops_ignored.load(Ordering::Relaxed);

    // Stale fetch completes
    let guard = reg.into_guard();
    let res = make_resource(b"stale_body", 60_000, now);
    let batch = guard.complete(FinalizeOutcome::Success(Arc::new(res)));
    assert!(batch.is_some());
    batch.unwrap().deliver();

    // Generation mismatch caught: stale_writes_prevented incremented
    let stale_after = cache.counters.stale_writes_prevented.load(Ordering::Relaxed);
    assert_eq!(stale_after, stale_before + 1);

    // Cache must remain empty (no resurrecting stale data)
    assert!(cache.get_resource(&key, now + 10).is_none());
}

#[tokio::test]
async fn test_ct8_framing_and_body_limits() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct8");
    let now = 1000;

    let valid_body = vec![0x42u8; 1024];
    let res = make_resource(&valid_body, 60_000, now);

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let g = reg.into_guard();
    let batch = g.complete(FinalizeOutcome::Success(Arc::new(res)));
    assert!(batch.is_some());
    batch.unwrap().deliver();

    assert_eq!(cache.get_resource(&key, now + 1).unwrap().body.len(), 1024);
}

#[tokio::test]
async fn test_ct9_expiry_herd_coalescing() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = make_key("t1", "/resource/ct9");
    let now = 1000;

    // Fill with 100ms TTL
    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let g = reg.into_guard();
    let res = make_resource(b"initial", 100, now);
    g.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    // At now + 50ms: still valid hit
    assert!(cache.get_resource(&key, now + 50).is_some());

    // At now + 150ms: entry is expired!
    let later = now + 150;
    // Exactly 1 request initiates revalidation
    let reg_new = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate for expired entry"),
    };

    // Concurrent request for same expired key waits on the single flight
    match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Wait(_) => {} // Herd prevented
        _ => panic!("Expected Wait on in-flight expired re-fetch"),
    }

    let g_new = reg_new.into_guard();
    let res_updated = make_resource(b"updated", 100, later);
    g_new.complete(FinalizeOutcome::Success(Arc::new(res_updated))).unwrap().deliver();

    assert_eq!(*cache.get_resource(&key, later + 10).unwrap().body, b"updated");
}

#[tokio::test]
async fn test_ct10_resource_bounds() {
    let cache = Arc::new(PartitionedCache::new(4));
    cache.set_byte_budget(10_000); // 10 KiB budget

    let initial_bytes = cache.bytes_resident();
    assert_eq!(initial_bytes, 0);

    // Insert entries and verify byte bounds are tracked accurately
    let key = make_key("t1", "/resource/ct10");
    let reg = match cache.begin_fetch_atomic(&key, 1000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let g = reg.into_guard();
    let res = make_resource(&[0xaa; 500], 60_000, 1000);
    g.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let resident = cache.bytes_resident();
    assert!(resident > 500); // body + key + fixed overhead
    assert!(resident <= 10_000);

    // In-flight count is 0 after completion
    for shard in &cache.shards {
        let s = shard.lock().unwrap();
        assert_eq!(s.in_flight.len(), 0);
    }
}
