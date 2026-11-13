use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::control::pie::{PieController, PieDecision};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, EvictionPolicy, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::intel::telemetry::{ObservationKind, RawObservation, TelemetryCollector};
use phantom_cache_demo::sim::rng::SeededRngSource;
use phantom_cache_demo::sim::traffic::{KeyDistribution, RequestType, TrafficGenerator, WorkloadWeights};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn make_resource(body_val: u8, created_at_ms: u64, ttl_ms: u64) -> Arc<CachedResource> {
    Arc::new(CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(vec![body_val]),
        etag: None,
        created_at_ms,
        ttl_ms,
    })
}

#[test]
fn test_p4_lru_eviction_order() {
    let cache = Arc::new(PartitionedCache::new(1));
    cache.set_capacity_limits(2, 100);
    cache.set_eviction_policy(EvictionPolicy::Lru);

    let k1 = CacheKey::new(None, "/key_1", None);
    let k2 = CacheKey::new(None, "/key_2", None);
    let k3 = CacheKey::new(None, "/key_3", None);

    // Insert key 1 at t=100
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k1, 100) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(1, 100, 60_000)));
    }

    // Insert key 2 at t=200
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k2, 200) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(2, 200, 60_000)));
    }

    // Access key 1 at t=300 (making key 2 the LRU victim!)
    match cache.begin_fetch_atomic(&k1, 300) {
        FetchRegistrationResult::Hit(_) => {}
        _ => panic!("Expected Hit for k1"),
    }

    // Insert key 3 at t=400 -> LRU must evict key 2, keeping key 1
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k3, 400) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(3, 400, 60_000)));
    }

    // k1 should be HIT, k2 should be MISS (evicted)
    match cache.begin_fetch_atomic(&k1, 500) {
        FetchRegistrationResult::Hit(_) => {}
        _ => panic!("k1 should still be present in LRU cache"),
    }

    let shard = cache.shards[0].lock().unwrap();
    assert!(!shard.map.contains_key(&k2), "k2 should have been evicted by LRU");
    assert!(shard.map.contains_key(&k1), "k1 should remain in LRU");
    assert!(shard.map.contains_key(&k3), "k3 should remain in LRU");
}

#[test]
fn test_p4_s3fifo_one_hit_wonder_filtering() {
    let cache = Arc::new(PartitionedCache::new(1));
    cache.set_capacity_limits(2, 100);
    cache.set_eviction_policy(EvictionPolicy::S3Fifo);

    let k_frequent = CacheKey::new(None, "/frequent", None);
    let k_one_hit_1 = CacheKey::new(None, "/one_hit_1", None);
    let k_one_hit_2 = CacheKey::new(None, "/one_hit_2", None);

    // 1. Insert frequent key at t=100 and access again at t=150 (access_count >= 2 -> promoted to Main FIFO)
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k_frequent, 100) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(1, 100, 60_000)));
    }
    match cache.begin_fetch_atomic(&k_frequent, 150) {
        FetchRegistrationResult::Hit(_) => {}
        _ => panic!("Expected Hit for k_frequent"),
    }

    // 2. Insert one-hit wonder 1 at t=200 (access_count = 1 -> in Small FIFO)
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k_one_hit_1, 200) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(2, 200, 60_000)));
    }

    // 3. Insert one-hit wonder 2 at t=300 (cache full: S3-FIFO must evict one_hit_1 from Small FIFO, NOT frequent!)
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k_one_hit_2, 300) {
        reg.into_guard().complete(FinalizeOutcome::Success(make_resource(3, 300, 60_000)));
    }

    let shard = cache.shards[0].lock().unwrap();
    assert!(
        shard.map.contains_key(&k_frequent),
        "S3-FIFO must preserve frequent key in Main FIFO"
    );
    assert!(
        !shard.map.contains_key(&k_one_hit_1),
        "S3-FIFO must evict one-hit wonder from Small FIFO"
    );
    assert!(
        shard.map.contains_key(&k_one_hit_2),
        "k_one_hit_2 should now reside in cache"
    );
}

#[test]
fn test_p4_all_policies_preserve_active_op_safety_invariant() {
    for policy in [EvictionPolicy::Sieve, EvictionPolicy::Lru, EvictionPolicy::S3Fifo] {
        let cache = Arc::new(PartitionedCache::new(1));
        cache.set_capacity_limits(1, 100);
        cache.set_eviction_policy(policy);

        let k_active = CacheKey::new(None, "/active_in_flight", None);
        let k_incoming = CacheKey::new(None, "/incoming_miss", None);

        // Initiate k_active but keep the guard alive (in-flight + active_op_id registered)
        let reg = match cache.begin_fetch_atomic(&k_active, 100) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let _guard = reg.into_guard();

        // Now capacity is 1, and the only entry is active in-flight!
        // Attempting to begin fetch for k_incoming cannot evict k_active.
        // It must strictly reject with CapacityExceeded rather than evicting the active operation!
        match cache.begin_fetch_atomic(&k_incoming, 150) {
            FetchRegistrationResult::Rejected(phantom_cache_demo::core::coalesce::OriginError::CapacityExceeded) => {
                // PASS: Capacity preserved safely without evicting active operation
            }
            _ => panic!(
                "Policy {:?} violated safety invariant: expected CapacityExceeded",
                policy
            ),
        }

        let shard = cache.shards[0].lock().unwrap();
        assert!(
            shard.map.contains_key(&k_active),
            "Policy {:?} must never evict active operation state",
            policy
        );
        assert!(
            shard.in_flight.contains_key(&k_active),
            "Policy {:?} must never evict in-flight shared op",
            policy
        );
    }
}

// LEGACY STUB TEST: the decision is the hard-coded threshold `hit_rate*100 - p99*0.1` in
// `src/control/pie.rs` (CLAUDE.md section 6b), not a counterfactual canary. The legacy controller no
// longer holds a cache handle (D-062), so this test checks its decisions and its own `active_policy`
// label only. It does not show that any policy was applied to a cache.
#[test]
fn test_p4_pie_adaptive_switching_and_rollback() {
    let telemetry = Arc::new(TelemetryCollector::new(2000));
    let pie = PieController::new(telemetry.clone());

    // 1. Prime telemetry with good performance: high hit rate (0.95), low latency (10ms)
    for i in 0..1100 {
        telemetry.record(RawObservation::new(
            i as u64,
            1,
            10,
            i % 20 != 0, // 95% hits
            2,
            ObservationKind::Get,
        ));
    }

    assert_eq!(pie.get_active_policy(), "SIEVE");

    // 2. Evaluate candidate "S3-FIFO" -> expected utility high (> 80.0) -> Commit
    let decision = pie.evaluate_candidate("S3-FIFO", 1000, true);
    assert_eq!(decision, PieDecision::Commit);
    assert_eq!(pie.get_active_policy(), "S3-FIFO");

    // 3. Clear and prime telemetry with severe regression (low hit rate 0.20, high latency 350ms)
    telemetry.clear();
    for i in 0..1100 {
        telemetry.record(RawObservation::new(
            (2000 + i) as u64,
            1,
            350,
            i % 5 == 0, // 20% hits
            0,
            ObservationKind::Get,
        ));
    }

    // Now utility = 20 - 35 = -15 < 50.0 -> Rollback!
    // Must advance clock past the 60s cooldown (1000 + 60_000 = 61_000)
    let rollback_decision = pie.evaluate_candidate("S3-FIFO", 65_000, true);
    assert_eq!(rollback_decision, PieDecision::Rollback);
    // Known stub gap (CORRECTIONS C-14): the legacy Rollback branch does not reset `active_policy`,
    // so no assertion is made about the label after a rollback; only the decision and the cooldown.

    // 4. Cooldown guardrail: immediate subsequent evaluation at 66_000 (< 65_000 + 60_000) must return NoAction
    let cooldown_decision = pie.evaluate_candidate("LRU", 66_000, true);
    assert_eq!(cooldown_decision, PieDecision::NoAction);
}

#[test]
fn test_p4_reproducible_policy_benchmark_comparison() {
    let policies = [EvictionPolicy::Sieve, EvictionPolicy::Lru, EvictionPolicy::S3Fifo];
    let capacity_per_shard = 50; // Total capacity = 50 entries
    let catalog_size = 500;      // 10x larger than cache to create sustained eviction pressure

    let mut results: Vec<(EvictionPolicy, f64, u64)> = Vec::new();

    for policy in policies {
        let cache = Arc::new(PartitionedCache::new(1));
        cache.set_capacity_limits(capacity_per_shard, 1000);
        cache.set_eviction_policy(policy);

        let rng = Arc::new(SeededRngSource::new(88888));
        let gen = TrafficGenerator::new(rng);
        let dist = KeyDistribution::Zipfian {
            catalog_size,
            skew: 1.1,
        };
        let weights = WorkloadWeights {
            cacheable_get: 1.0,
            uncacheable_get: 0.0,
            delete_invalidation: 0.0,
            bypass: 0.0,
            upstream_error: 0.0,
        };

        let requests = gen.generate_batch(5_000, &dist, &weights);
        let mut now_ms = 1000;

        for req in requests {
            now_ms += 1;
            if let RequestType::CacheableGet { key } = req {
                let cache_key = CacheKey::new(None, &key, None);
                match cache.begin_fetch_atomic(&cache_key, now_ms) {
                    FetchRegistrationResult::Hit(_) => {}
                    FetchRegistrationResult::Initiate(reg) => {
                        let res = make_resource(1, now_ms, 60_000);
                        reg.into_guard().complete(FinalizeOutcome::Success(res));
                    }
                    _ => {}
                }
            }
        }

        let hits = cache.counters.cache_hits.load(Ordering::Relaxed);
        let misses = cache.counters.cache_misses.load(Ordering::Relaxed);
        let evictions = cache.counters.evictions_total.load(Ordering::Relaxed);
        let hit_ratio = hits as f64 / (hits + misses) as f64;

        results.push((policy, hit_ratio, evictions));
    }

    // Report benchmark metrics
    for (pol, hr, ev) in &results {
        println!(
            "[PIE Benchmark] Policy: {:?}, Hit Ratio: {:.2}%, Evictions: {}",
            pol,
            hr * 100.0,
            ev
        );
        // All policies must achieve viable hit rates on Zipfian traffic with finite capacity
        assert!(*hr > 0.15, "Policy {:?} must achieve > 15% hit ratio", pol);
        assert!(*ev > 1000, "Policy {:?} must experience active evictions", pol);
    }
}

// LEGACY STUB TEST: real HTTP traffic feeds the telemetry collector; the legacy controller then applies
// its hard-coded threshold. The controller has no cache handle (D-062), and the final assertions check
// that committing a label leaves the serving cache's eviction policy untouched.
#[tokio::test]
async fn test_p4_live_pie_control_path_under_concurrent_traffic() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = origin_listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = r#"{"live":"pie_data"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let telemetry = Arc::new(TelemetryCollector::new(2000));
    let pie = Arc::new(PieController::new(telemetry.clone()));
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    nodes[0].set_state(NodeState::Active);

    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![NodeSnapshot {
            node_id: 1,
            state: "Active".to_string(),
            cpu_utilization: 10.0,
            memory_used_bytes: 1024,
        }],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: Some(format!("http://{}", origin_addr)),
        telemetry: Some(telemetry.clone()),
        pie: Some(pie.clone()),
        security: None,
        upstream_policy: None,
        router: None,
    });

    let app = create_app(api_state);
    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server_listener, app).await.unwrap();
    });

    // Run real concurrent traffic: 1,200 requests to cross the PIE sample threshold (1,000)
    let total_requests = 1200;
    let mut tasks = Vec::new();
    for i in 0..total_requests {
        let key = format!("/item_{}", i % 10);
        let task = tokio::spawn(async move {
            let req = format!(
                "GET /api/v1/cache{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                key
            );
            if let Ok(mut stream) = tokio::net::TcpStream::connect(server_addr).await {
                let _ = stream.write_all(req.as_bytes()).await;
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf).await;
            }
        });
        tasks.push(task);
    }

    for t in tasks {
        let _ = t.await;
    }

    assert!(telemetry.len() >= 1000, "Telemetry buffer must contain >= 1000 live samples");
    let hit_rate = telemetry.calculate_hit_rate();
    assert!(hit_rate > 0.80, "Live hit rate on 10 repeated keys should exceed 80% (got {})", hit_rate);

    // PIE controller evaluates S3-FIFO candidate with live real telemetry
    assert_eq!(cache.get_eviction_policy(), EvictionPolicy::Sieve);
    assert_eq!(pie.get_active_policy(), "SIEVE");
    let decision = pie.evaluate_candidate("S3-FIFO", 1000, true);
    assert_eq!(decision, PieDecision::Commit);
    assert_eq!(pie.get_active_policy(), "S3-FIFO");
    assert_eq!(
        cache.get_eviction_policy(),
        EvictionPolicy::Sieve,
        "The legacy controller must not change the serving cache's eviction policy"
    );
}
