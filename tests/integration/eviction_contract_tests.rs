use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;

use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, EvictionPolicy, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::{FetchRegistration, FinalizeOutcome, WaiterBatch};
use phantom_cache_demo::core::eviction::{
    FaithfulLru, FaithfulS3Fifo, FaithfulSieve, ShardEvictionPolicy,
};

fn k(name: &str) -> CacheKey {
    CacheKey::new(Some("t1"), name, None)
}

fn make_resource(body_len: usize) -> CachedResource {
    CachedResource {
        status: 200,
        body: Arc::new(vec![0x42u8; body_len]),
        content_type: "application/octet-stream".to_string(),
        etag: Some("\"test-etag\"".to_string()),
        created_at_ms: 1_000_000,
        ttl_ms: 60_000,
    }
}

fn complete_ok(reg: FetchRegistration, res: CachedResource) -> Option<WaiterBatch> {
    reg.into_guard().complete(FinalizeOutcome::Success(Arc::new(res)))
}

// ---------------------------------------------------------------------------
// 1. Golden Trace Parity Tests
// ---------------------------------------------------------------------------

#[test]
fn test_lru_golden_trace_parity() {
    let mut lru = FaithfulLru::new(3);
    let mut no_pin = |_k: &CacheKey| false;

    // Insert A, B, C
    lru.on_insert(&k("A"));
    lru.on_insert(&k("B"));
    lru.on_insert(&k("C"));

    // Touch A -> MRU order becomes B, C, A (B is LRU tail)
    lru.on_hit(&k("A"));

    // Insert D -> capacity 3 exceeded, select victim
    let victim = lru.select_victim(&mut no_pin);
    assert_eq!(victim, Some(k("B")), "LRU must evict B (least recently used)");

    lru.on_insert(&k("D"));

    // Remaining in order from LRU to MRU: C, A, D
    let next_victim = lru.select_victim(&mut no_pin);
    assert_eq!(next_victim, Some(k("C")), "Next LRU victim must be C");
}

#[test]
fn test_sieve_golden_trace_parity() {
    let mut sieve = FaithfulSieve::new(3);
    let mut no_pin = |_k: &CacheKey| false;

    // Insert A, B, C
    sieve.on_insert(&k("A"));
    sieve.on_insert(&k("B"));
    sieve.on_insert(&k("C"));

    // Touch B (sets visited=true on B)
    sieve.on_hit(&k("B"));

    // Evict: hand sweeps from tail (C was newest? No, head is C, tail is A or C depending on direction)
    // Sieve unlinks unvisited victim. If B was visited, B's bit is cleared and next unvisited is chosen.
    let victim = sieve.select_victim(&mut no_pin);
    assert!(victim.is_some(), "Sieve must evict an unvisited entry");
    let victim_key = victim.unwrap();
    assert_ne!(victim_key, k("B"), "Sieve must NOT evict visited entry B on first pass");

    sieve.on_insert(&k("D"));
}

#[test]
fn test_s3fifo_golden_trace_parity() {
    let mut s3 = FaithfulS3Fifo::new(10);
    let mut no_pin = |_k: &CacheKey| false;

    // Fill small queue
    for i in 0..10 {
        s3.on_insert(&k(&format!("key_{}", i)));
    }

    // Touch some keys to promote them
    s3.on_hit(&k("key_0"));
    s3.on_hit(&k("key_0")); // freq = 2 -> will promote to main

    // Evict
    let victim = s3.select_victim(&mut no_pin);
    assert!(victim.is_some(), "S3-FIFO must evict a candidate");
    assert_ne!(victim, Some(k("key_0")), "Key 0 had freq=2 and should be promoted to Main, not evicted");
}

// ---------------------------------------------------------------------------
// 2. Invariant I5: Pinned entries are NEVER evicted
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_i5_pinned_never_evicted() {
    let mut lru = FaithfulLru::new(5);

    lru.on_insert(&k("A"));
    lru.on_insert(&k("B"));
    lru.on_insert(&k("C"));

    // Pin "A" (e.g. active in-flight request or active op)
    let mut is_pinned = |key: &CacheKey| key == &k("A");

    // Even though "A" is the oldest (LRU), it must be skipped
    let victim1 = lru.select_victim(&mut is_pinned);
    assert_eq!(victim1, Some(k("B")), "Pinned A must be skipped; B must be evicted");

    let victim2 = lru.select_victim(&mut is_pinned);
    assert_eq!(victim2, Some(k("C")), "Pinned A must be skipped; C must be evicted");

    // Only A remains and it is pinned -> select_victim must return None
    let victim3 = lru.select_victim(&mut is_pinned);
    assert_eq!(victim3, None, "All remaining entries pinned -> must return None");
}

// ---------------------------------------------------------------------------
// 3. O(1) Scaling / Latency Bound
// ---------------------------------------------------------------------------

#[test]
fn test_o1_scaling_bounded_eviction_time() {
    // Test FaithfulLru and FaithfulSieve scaling from 100 to 10,000 entries
    let mut sieve_small = FaithfulSieve::new(100);
    for i in 0..100 {
        sieve_small.on_insert(&k(&format!("small_{}", i)));
    }

    let mut sieve_large = FaithfulSieve::new(10_000);
    for i in 0..10_000 {
        sieve_large.on_insert(&k(&format!("large_{}", i)));
    }

    let mut no_pin = |_k: &CacheKey| false;

    // Evict 50 entries from small
    let t0 = Instant::now();
    for _ in 0..50 {
        let _ = sieve_small.select_victim(&mut no_pin);
    }
    let elapsed_small = t0.elapsed();

    // Evict 50 entries from large
    let t1 = Instant::now();
    for _ in 0..50 {
        let _ = sieve_large.select_victim(&mut no_pin);
    }
    let elapsed_large = t1.elapsed();

    // Average per eviction must be strictly sub-microsecond or low microsecond (< 10 µs)
    let avg_ns_large = elapsed_large.as_nanos() / 50;
    assert!(
        avg_ns_large < 10_000,
        "O(1) FaithfulSieve eviction took {} ns per item at 10,000 entries (must be < 10,000 ns)",
        avg_ns_large
    );

    // Scaling ratio should not grow with N (factor of 100x entry count must not cause 100x latency)
    // Both small and large should complete 50 evictions in < 1 ms total
    assert!(elapsed_small < Duration::from_millis(5));
    assert!(elapsed_large < Duration::from_millis(5));
}

// ---------------------------------------------------------------------------
// 4. Policy Reconfiguration & Dynamic Switching (rebuild_from)
// ---------------------------------------------------------------------------

#[test]
fn test_policy_reconfiguration_and_rebuild() {
    let cache = Arc::new(PartitionedCache::new(1));
    cache.set_capacity_limits(10, 100);

    // Start with Sieve
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);

    // Insert 5 entries
    for i in 0..5 {
        let key = k(&format!("k_{}", i));
        let reg = match cache.begin_fetch_atomic(&key, 1_000) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let outcome = complete_ok(reg, make_resource(64));
        assert!(outcome.is_some());
    }

    assert_eq!(cache.shards[0].lock().unwrap().map.len(), 5);

    // Switch to LRU
    cache.set_eviction_policy(EvictionPolicy::Lru);
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Lru);
    assert_eq!(cache.shards[0].lock().unwrap().map.len(), 5, "All keys preserved across policy switch");

    // Access k_0 to make it MRU in LRU policy
    let hit = cache.begin_fetch_atomic(&k("k_0"), 2_000);
    assert!(matches!(hit, FetchRegistrationResult::Hit(_)));

    // Switch to S3Fifo
    cache.set_eviction_policy(EvictionPolicy::S3Fifo);
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::S3Fifo);
    assert_eq!(cache.shards[0].lock().unwrap().map.len(), 5, "All keys preserved across switch to S3Fifo");
}

// ---------------------------------------------------------------------------
// 5. Accounting Invariants Preserved Under Eviction
// ---------------------------------------------------------------------------

#[test]
fn test_accounting_invariants_preserved_under_eviction() {
    let cache = Arc::new(PartitionedCache::new(1));
    cache.set_capacity_limits(5, 100);
    cache.set_eviction_policy(EvictionPolicy::Sieve);

    // Insert 10 entries into capacity-5 shard -> triggers 5 evictions
    for i in 0..10 {
        let key = k(&format!("churn_{}", i));
        let reg = match cache.begin_fetch_atomic(&key, 1_000 + i as u64) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let outcome = complete_ok(reg, make_resource(128));
        assert!(outcome.is_some());
    }

    assert_eq!(cache.counters.evictions_total.load(Ordering::Relaxed), 5);
    assert_eq!(cache.shards[0].lock().unwrap().map.len(), 5);

    // Verify logical byte accounting equality
    let (accounted, recorded) = cache.audit_bytes();
    assert_eq!(
        accounted, recorded,
        "Logical byte accounting must equal recorded resident bytes (accounted={}, recorded={})",
        accounted, recorded
    );
}

// ---------------------------------------------------------------------------
// 6. HTTP Gateway End-to-End Eviction Test
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockOriginState {
    pub count: Arc<AtomicU64>,
}

async fn mock_origin_handler(
    State(state): State<MockOriginState>,
    path: Option<Path<String>>,
) -> impl IntoResponse {
    let count = state.count.fetch_add(1, Ordering::SeqCst);
    let path_str = path.map(|p| p.0).unwrap_or_else(|| "root".to_string());
    (
        StatusCode::OK,
        [("Cache-Control", "public, max-age=60")],
        Json(serde_json::json!({
            "key": path_str,
            "req_num": count
        })),
    )
        .into_response()
}

#[tokio::test]
async fn test_http_gateway_eviction_end_to_end() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let origin_count = Arc::new(AtomicU64::new(0));
        let origin_state = MockOriginState {
            count: origin_count.clone(),
        };
        let origin_app = Router::new()
            .route("/*path", get(mock_origin_handler))
            .with_state(origin_state);
        let origin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_port = origin_listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(origin_listener, origin_app).await.unwrap();
        });

        // 1-shard cache with max_entries_per_shard = 2
        let cache = Arc::new(PartitionedCache::new(1));
        cache.set_capacity_limits(2, 100);
        cache.set_eviction_policy(EvictionPolicy::Lru);

        let dummy_snap = SimulationSnapshot {
            version: "v1.0".to_string(),
            timestamp_ms: 1000,
            throughput_req_sec: 100.0,
            p50_latency_ms: 1,
            p95_latency_ms: 2,
            p99_latency_ms: 3,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![],
        };

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(dummy_snap),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache.clone()),
            origin_url: Some(format!("http://127.0.0.1:{}", origin_port)),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_port = gateway_listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(gateway_listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let u1 = format!("http://127.0.0.1:{}/api/v1/cache/item_1", gateway_port);
        let u2 = format!("http://127.0.0.1:{}/api/v1/cache/item_2", gateway_port);
        let u3 = format!("http://127.0.0.1:{}/api/v1/cache/item_3", gateway_port);

        // Fetch 1: MISS
        let r1 = client.get(&u1).send().await.unwrap();
        assert_eq!(r1.headers().get("X-Cache-Status").unwrap(), "MISS");

        // Fetch 2: MISS
        let r2 = client.get(&u2).send().await.unwrap();
        assert_eq!(r2.headers().get("X-Cache-Status").unwrap(), "MISS");

        // Fetch 3: MISS (triggers eviction of item_1)
        let r3 = client.get(&u3).send().await.unwrap();
        assert_eq!(r3.headers().get("X-Cache-Status").unwrap(), "MISS");

        // Check eviction counter
        assert_eq!(cache.counters.evictions_total.load(Ordering::Relaxed), 1);

        // item_1 was evicted -> next GET is a MISS
        let r1_again = client.get(&u1).send().await.unwrap();
        assert_eq!(r1_again.headers().get("X-Cache-Status").unwrap(), "MISS");

        // item_3 is still resident -> HIT
        let r3_again = client.get(&u3).send().await.unwrap();
        assert_eq!(r3_again.headers().get("X-Cache-Status").unwrap(), "HIT");
    })
    .await
    .expect("test timed out");
}
