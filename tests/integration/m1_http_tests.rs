use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::{FinalizeOutcome, OriginError};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::intel::diagnostics::{DiagnosticBuffer, DiagnosticEvent, DiagnosticLevel};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Barrier;

// ==========================================
// In-Process Mock HTTP Origin Server
// ==========================================

#[derive(Clone)]
struct MockOriginState {
    request_count: Arc<AtomicU64>,
}

// Round 3 (M-1): the gateway no longer forwards client `x-*` request headers to the origin (the
// cache key ignores them, so forwarding them let one user's header choose a variant stored for
// everyone). Tests that used `x-mock-*` CLIENT headers to steer this mock now register a one-shot
// per-path plan instead; the assertions are unchanged.
#[derive(Clone, Default)]
struct Plan {
    delay_ms: u64,
    private: bool,
    oversized: bool,
}
impl Plan {
    fn delay(ms: u64) -> Self {
        Plan { delay_ms: ms, ..Default::default() }
    }
    fn private() -> Self {
        Plan { private: true, ..Default::default() }
    }
    fn oversized() -> Self {
        Plan { oversized: true, ..Default::default() }
    }
}

static MOCK_PLANS: std::sync::OnceLock<Mutex<std::collections::HashMap<String, Plan>>> = std::sync::OnceLock::new();

/// One-shot: the next origin request for `/<path>` follows `plan`, later ones are plain.
fn mock_plan(path: &str, plan: Plan) {
    MOCK_PLANS
        .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap()
        .insert(path.trim_start_matches('/').to_string(), plan);
}

fn take_plan(path: &str) -> Plan {
    MOCK_PLANS
        .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap()
        .remove(path.trim_start_matches('/'))
        .unwrap_or_default()
}

async fn mock_handler(
    State(state): State<MockOriginState>,
    path: Option<Path<String>>,
    _headers: HeaderMap,
) -> Response {
    let count = state.request_count.fetch_add(1, Ordering::SeqCst) + 1;
    let path_str = path.map(|p| p.0).unwrap_or_default();
    let plan = take_plan(&path_str);

    if plan.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(plan.delay_ms)).await;
    }

    let status_code = StatusCode::OK;

    if plan.oversized {
        let large_body = "{\"data\":\"".to_string() + &"A".repeat(3 * 1024 * 1024) + "\"}";
        return (
            StatusCode::OK,
            [("Content-Type", "application/json")],
            large_body,
        )
            .into_response();
    }

    let mut resp = (
        status_code,
        Json(serde_json::json!({
            "key": path_str,
            "req_num": count
        })),
    )
        .into_response();

    if plan.private {
        resp.headers_mut().insert(
            "Cache-Control",
            HeaderValue::from_static("private, no-store"),
        );
    }
    resp.headers_mut()
        .insert("ETag", HeaderValue::from_static("\"etag-mock-1\""));

    resp
}

async fn spawn_mock_origin() -> (String, Arc<AtomicU64>) {
    let request_count = Arc::new(AtomicU64::new(0));
    let state = MockOriginState {
        request_count: request_count.clone(),
    };

    let app = Router::new()
        .route("/*path", get(mock_handler))
        .route("/", get(mock_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://127.0.0.1:{}", addr.port()), request_count)
}

async fn spawn_raw_tcp_origin(response_bytes: Vec<u8>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let _ = socket.write_all(&response_bytes).await;
            let _ = socket.flush().await;
        }
    });
    format!("http://127.0.0.1:{}", port)
}

struct TestContext {
    base_url: String,
    cache: Arc<PartitionedCache>,
    origin_count: Arc<AtomicU64>,
    client: reqwest::Client,
}

impl TestContext {
    async fn new() -> Self {
        let (origin_url, origin_count) = spawn_mock_origin().await;
        Self::with_origin(origin_url, origin_count).await
    }

    async fn with_origin(origin_url: String, origin_count: Arc<AtomicU64>) -> Self {
        let cache = Arc::new(PartitionedCache::new(8));

        let initial_snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 12000.0,
            p50_latency_ms: 10,
            p95_latency_ms: 12,
            p99_latency_ms: 15,
            db_load_ratio: 0.2,
            avoided_compute_dollars: 0.0,
            nodes: vec![NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            }],
        };

        let nodes = vec![Arc::new(VirtualNode::new(1))];
        nodes[0].set_state(NodeState::Active);

        let api_state = Arc::new(ApiState {
            snapshot: Mutex::new(initial_snap),
            nodes,
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache.clone()),
            origin_url: Some(origin_url),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let app = create_app(api_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            base_url: format!("http://127.0.0.1:{}", port),
            cache,
            origin_count,
            client: reqwest::Client::new(),
        }
    }
}

// ==========================================
// 23 Integration Tests (M1-T01 through M1-T23)
// ==========================================

#[tokio::test]
async fn test_m1_t01_cold_cache_miss() {
    let ctx = TestContext::new().await;
    let resp = ctx
        .client
        .get(format!("{}/api/v1/cache/item1", ctx.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("X-Cache-Status").unwrap(),
        HeaderValue::from_static("MISS")
    );
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_m1_t02_repeat_cache_hit() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item2", ctx.base_url);

    let resp1 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);

    let resp2 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "HIT");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_m1_t03_cache_invalidation() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item3", ctx.base_url);

    let resp1 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);

    let del_resp = ctx.client.delete(&url).send().await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let resp2 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "MISS");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_m1_t04_invalidation_race_key_exists() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item4", ctx.base_url);
    mock_plan("item4", Plan::delay(200));

    // 1. Populate key
    ctx.client.get(&url).send().await.unwrap();
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);

    // 2. Start slow fetch (origin delay 200ms)
    let client_slow = ctx.client.clone();
    let url_slow = url.clone();
    let slow_fetch = tokio::spawn(async move {
        client_slow
            .get(&url_slow)
            .header("X-Cache-Bypass", "false")
            .send()
            .await
            .unwrap()
    });

    // 3. At 50ms, call DELETE
    tokio::time::sleep(Duration::from_millis(50)).await;
    let del_resp = ctx.client.delete(&url).send().await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let slow_resp = slow_fetch.await.unwrap();
    // In-flight fetch was served from existing cache or finished
    assert_eq!(slow_resp.status(), StatusCode::OK);

    // 4. Next GET fetches from origin because DELETE invalidated it
    let resp3 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp3.headers().get("X-Cache-Status").unwrap(), "MISS");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_m1_t05_invalidation_race_cold_key() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item5", ctx.base_url);
    mock_plan("item5", Plan::delay(200));

    // 1. Key is cold; start slow fetch (200ms delay)
    let client_slow = ctx.client.clone();
    let url_slow = url.clone();
    let slow_fetch = tokio::spawn(async move {
        client_slow
            .get(&url_slow)
            .send()
            .await
            .unwrap()
    });

    // 2. At 50ms, send DELETE
    tokio::time::sleep(Duration::from_millis(50)).await;
    let del_resp = ctx.client.delete(&url).send().await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // 3. Wait for slow fetch to finish
    let slow_resp = slow_fetch.await.unwrap();
    assert_eq!(slow_resp.status(), StatusCode::OK);

    // 4. Stale write must be rejected; next GET must miss and contact origin
    let next_resp = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(next_resp.headers().get("X-Cache-Status").unwrap(), "MISS");
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 2);
    assert_eq!(
        ctx.cache
            .counters
            .stale_writes_prevented
            .load(Ordering::Relaxed),
        1
    );
}

#[tokio::test]
async fn test_m1_t06_delete_between_registration_and_task_startup() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item6", None);

    // Register fetch manually
    let reg_result = ctx.cache.begin_fetch_atomic(&key, 100);
    let reg = match reg_result {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    // Call DELETE while registration is held but before task spawns
    ctx.cache.delete(&key);

    // Convert to guard and complete
    let guard = reg.into_guard();
    let resource = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });
    guard.complete(FinalizeOutcome::Success(resource));

    // Assert write was rejected and entry was not populated
    assert_eq!(
        ctx.cache
            .counters
            .stale_writes_prevented
            .load(Ordering::Relaxed),
        1
    );
    assert!(ctx.cache.get_resource(&key, 200).is_none());
}

#[tokio::test]
async fn test_m1_t07_task_spawn_failure_cleanup() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item7", None);

    // Simulate spawn failure: FetchRegistration dropped without into_guard
    {
        let reg_result = ctx.cache.begin_fetch_atomic(&key, 100);
        let _reg = match reg_result {
            phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        // _reg drops here!
    }

    // Verify in-flight entry was removed and memory reclaimed
    let idx = ctx.cache.get_shard_index(&key);
    let shard = ctx.cache.shards[idx].lock().unwrap();
    assert!(!shard.in_flight.contains_key(&key));
    assert!(!shard.map.contains_key(&key));
}

#[tokio::test]
async fn test_m1_t08_spawn_failure_with_waiters() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item8", None);

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();

    {
        let reg_result = ctx.cache.begin_fetch_atomic(&key, 100);
        let reg = match reg_result {
            phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };

        // Attach 2 waiters
        reg.shared_op.waiters.lock().unwrap().push(tx1);
        reg.shared_op.waiters.lock().unwrap().push(tx2);
        // reg drops without spawning task!
    }

    // Both waiters must receive Aborted error
    let res1 = rx1.await.unwrap();
    let res2 = rx2.await.unwrap();
    assert!(matches!(res1, Err(ref e) if matches!(**e, OriginError::Aborted)));
    assert!(matches!(res2, Err(ref e) if matches!(**e, OriginError::Aborted)));
}

#[tokio::test]
async fn test_m1_t09_get_after_delete_does_not_join_old_fetch() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item9", ctx.base_url);
    mock_plan("item9", Plan::delay(250));

    // Op 1 starts at Gen 0 with 250ms delay
    let client1 = ctx.client.clone();
    let url1 = url.clone();
    let fetch1 = tokio::spawn(async move {
        client1
            .get(&url1)
            .send()
            .await
            .unwrap()
    });

    // At 50ms, DELETE arrives
    tokio::time::sleep(Duration::from_millis(50)).await;
    let del_resp = ctx.client.delete(&url).send().await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Client 2 calls GET at 70ms
    tokio::time::sleep(Duration::from_millis(20)).await;
    let resp2 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "MISS");

    let resp1 = fetch1.await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    // Origin counter must be 2: Op 1 and Op 2 both contacted origin!
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_m1_t10_old_op_cleanup_racing_with_newer_registration() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item10", None);

    // Op 1 registers at Gen 0
    let reg1 = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard1 = reg1.into_guard();

    // DELETE detaches Op 1 and increments generation
    ctx.cache.delete(&key);

    // Op 2 registers at Gen 1
    let reg2 = match ctx.cache.begin_fetch_atomic(&key, 150) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard2 = reg2.into_guard();

    // Op 1 completes: must NOT remove Op 2 from in_flight!
    let res1 = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{\"data\":\"old\"}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });
    guard1.complete(FinalizeOutcome::Success(res1));

    // Op 2 should still be in_flight
    let idx = ctx.cache.get_shard_index(&key);
    {
        let shard = ctx.cache.shards[idx].lock().unwrap();
        assert_eq!(shard.in_flight.get(&key).unwrap().op_id, guard2.op_id);
    }

    // Op 2 completes and populates cache
    let res2 = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{\"data\":\"new\"}".to_vec()),
        etag: None,
        created_at_ms: 150,
        ttl_ms: 60_000,
    });
    guard2.complete(FinalizeOutcome::Success(res2));

    // Fresh GET should hit Op 2's data
    let cached = ctx.cache.get_resource(&key, 200).unwrap();
    assert_eq!(*cached.body, b"{\"data\":\"new\"}");
}

#[tokio::test]
async fn test_m1_t11_stale_op_cannot_decrement_newer_counter() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item11", None);

    let reg1 = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard1 = reg1.into_guard();

    // DELETE key and simulate reclamation
    ctx.cache.delete(&key);
    let idx = ctx.cache.get_shard_index(&key);
    {
        let mut shard = ctx.cache.shards[idx].lock().unwrap();
        shard.map.remove(&key);
    }

    // Recreate key under new state_instance_id
    let reg2 = match ctx.cache.begin_fetch_atomic(&key, 200) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };

    // Stale guard1 finishes
    let res1 = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });
    guard1.complete(FinalizeOutcome::Success(res1));

    // Op 2's active_op_ids must still contain reg2.op_id!
    {
        let shard = ctx.cache.shards[idx].lock().unwrap();
        let state = shard.map.get(&key).unwrap();
        assert!(state.active_op_ids.contains(&reg2.op_id));
    }
    assert_eq!(
        ctx.cache.counters.stale_ops_ignored.load(Ordering::Relaxed),
        1
    );
}

#[tokio::test]
async fn test_m1_t12_missing_state_resolves_own_waiters() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item12", None);

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let reg = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    reg.shared_op.waiters.lock().unwrap().push(tx1);
    let guard = reg.into_guard();

    // Reclaim state manually
    let idx = ctx.cache.get_shard_index(&key);
    {
        let mut shard = ctx.cache.shards[idx].lock().unwrap();
        shard.map.remove(&key);
    }

    // Complete guard
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{\"hello\":\"world\"}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });
    guard.complete(FinalizeOutcome::Success(res));

    // Waiter must receive payload cleanly
    let outcome = rx1.await.unwrap().unwrap();
    assert_eq!(*outcome.body, b"{\"hello\":\"world\"}");
}

#[tokio::test]
async fn test_m1_t13_barrier_stampede_100_clients() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/stampede", ctx.base_url);
    mock_plan("stampede", Plan::delay(150));

    let barrier = Arc::new(Barrier::new(101));
    let mut handles = Vec::with_capacity(100);

    for _ in 0..100 {
        let b = barrier.clone();
        let client = ctx.client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            client
                .get(&u)
                .send()
                .await
                .unwrap()
        }));
    }

    // Fire all 100 requests simultaneously
    barrier.wait().await;

    let mut misses = 0;
    let mut coalesced = 0;
    for h in handles {
        let resp = h.await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let status_hdr = resp.headers().get("X-Cache-Status").unwrap();
        if status_hdr == "MISS" {
            misses += 1;
        } else if status_hdr == "COALESCED" {
            coalesced += 1;
        }
    }

    assert_eq!(misses, 1);
    assert_eq!(coalesced, 99);
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_m1_t14_initiator_disconnect_resilience() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item14", ctx.base_url);
    mock_plan("item14", Plan::delay(300));

    // Client 1 initiates fetch with 300ms delay and disconnects early
    let client1 = reqwest::Client::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let url1 = url.clone();
    let _ = client1
        .get(&url1)
        .send()
        .await;

    // Clients 2 & 3 join as waiters
    tokio::time::sleep(Duration::from_millis(20)).await;
    let resp2 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    // Cache should be populated
    let resp3 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(resp3.headers().get("X-Cache-Status").unwrap(), "HIT");
}

#[tokio::test]
async fn test_m1_t15_background_task_panic_cleanup() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item15", None);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let reg = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    reg.shared_op.waiters.lock().unwrap().push(tx);
    let guard = reg.into_guard();

    // Spawn task that panics
    let task = tokio::spawn(async move {
        let _g = guard;
        panic!("Simulated background task panic");
    });
    let _ = task.await;

    // Waiter must receive 502 / Aborted
    let outcome = rx.await.unwrap();
    assert!(matches!(outcome, Err(ref e) if matches!(**e, OriginError::Aborted)));

    // In-flight map must be clean
    let idx = ctx.cache.get_shard_index(&key);
    let shard = ctx.cache.shards[idx].lock().unwrap();
    assert!(!shard.in_flight.contains_key(&key));
}

#[tokio::test]
async fn test_m1_t16_panic_at_finalization_boundary_and_lock_poison_recovery() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item16", None);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let reg = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    reg.shared_op.waiters.lock().unwrap().push(tx);
    let guard = reg.into_guard();

    // Enable injected panic in finalize_fetch
    ctx.cache
        .test_panic_after_waiter_extraction
        .store(true, Ordering::Relaxed);

    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });

    let task = tokio::spawn(async move {
        guard.complete(FinalizeOutcome::Success(res));
    });
    let _ = task.await;

    // Disable injected panic
    ctx.cache
        .test_panic_after_waiter_extraction
        .store(false, Ordering::Relaxed);

    // Waiter must receive result via WaiterBatch drop
    let outcome = rx.await.unwrap();
    assert!(outcome.is_ok());

    // Subsequent operation recovers poisoned lock cleanly
    let reg2 = ctx.cache.begin_fetch_atomic(&key, 200);
    assert!(matches!(
        reg2,
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(_)
    ));
    assert!(
        ctx.cache
            .counters
            .lock_poison_recovered
            .load(Ordering::Relaxed)
            >= 1
    );
}

#[tokio::test]
async fn test_m1_t17_safe_generation_reclamation() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item17", None);

    let reg = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();

    // Call DELETE
    ctx.cache.delete(&key);

    // Complete fetch
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });
    guard.complete(FinalizeOutcome::Success(res));

    // Map must be reclaimed
    let idx = ctx.cache.get_shard_index(&key);
    {
        let shard = ctx.cache.shards[idx].lock().unwrap();
        assert!(!shard.map.contains_key(&key));
    }

    // Fresh request allocates clean KeyState
    let reg2 = match ctx.cache.begin_fetch_atomic(&key, 200) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    assert_eq!(reg2.op_gen, 0); // Fresh allocation starts at 0
}

#[tokio::test]
async fn test_m1_t18_cross_tenant_isolation() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item18", ctx.base_url);

    let resp1 = ctx
        .client
        .get(&url)
        .header("X-Tenant-ID", "tenant_a")
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");

    let resp2 = ctx
        .client
        .get(&url)
        .header("X-Tenant-ID", "tenant_b")
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "MISS");

    // 2 separate origin fetches for tenant_a and tenant_b
    assert_eq!(ctx.origin_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_m1_t19_uncacheable_headers_and_bypass() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item19", ctx.base_url);

    // 1. Bypass request
    let bypass_resp = ctx
        .client
        .get(&url)
        .header("X-Cache-Bypass", "true")
        .send()
        .await
        .unwrap();
    assert_eq!(
        bypass_resp.headers().get("X-Cache-Status").unwrap(),
        "BYPASS"
    );

    // 2. Uncacheable request (Cache-Control: private)
    let url_private = format!("{}/api/v1/cache/item19_private", ctx.base_url);
    mock_plan("item19_private", Plan::private());
    let resp1 = ctx
        .client
        .get(&url_private)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");

    // Next request must NOT hit cache
    let resp2 = ctx.client.get(&url_private).send().await.unwrap();
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "MISS");
}

#[tokio::test]
async fn test_m1_t20_diagnostics_counters_accuracy() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item20", ctx.base_url);

    // Miss 1
    ctx.client.get(&url).send().await.unwrap();
    // Hit 1, 2, 3
    ctx.client.get(&url).send().await.unwrap();
    ctx.client.get(&url).send().await.unwrap();
    ctx.client.get(&url).send().await.unwrap();

    // Invalidation 1
    ctx.client.delete(&url).send().await.unwrap();

    let diag_resp = ctx
        .client
        .get(format!("{}/api/v1/diagnostics/summary", ctx.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(diag_resp.status(), StatusCode::OK);
    let body: Value = diag_resp.json().await.unwrap();

    assert_eq!(body["cache_hits"], 3);
    assert_eq!(body["cache_misses"], 1);
}

#[tokio::test]
async fn test_m1_t21_diagnostics_ring_buffer_capacity_overflow() {
    let counters = Arc::new(phantom_cache_demo::intel::diagnostics::CacheCounters::new());
    let buf = DiagnosticBuffer::new(512, counters);

    for i in 0..600 {
        buf.push(DiagnosticEvent {
            timestamp_epoch_ms: i,
            severity: DiagnosticLevel::INFO,
            event_name: "TestEvent",
            key_fp: format!("k_{:08x}", i),
            op_id: Some(i),
            op_gen: Some(0),
            outcome: "Test",
            duration_ms: None,
            error_msg: None,
        });
    }

    let events = buf.get_recent_events(1000);
    assert_eq!(events.len(), 512);
    // Oldest 88 events were dropped: oldest remaining event is 88
    assert_eq!(events.last().unwrap().op_id, Some(88));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_m1_t22_diagnostics_non_blocking_try_lock_contention() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item22", ctx.base_url);

    // Populate key first
    ctx.client.get(&url).send().await.unwrap();

    // Artificially hold the diagnostic events mutex in a background thread
    let events_lock = ctx.cache.diagnostics.events.lock().unwrap();

    // Send 10 concurrent requests: workers must NOT block on logging!
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client = ctx.client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(async move {
            let start = std::time::Instant::now();
            let resp = client.get(&u).send().await.unwrap();
            (resp.status(), start.elapsed())
        }));
    }

    for h in handles {
        let (status, elapsed) = h.await.unwrap();
        assert_eq!(status, StatusCode::OK);
        // Guaranteed fast completion without blocking on lock
        assert!(elapsed < Duration::from_millis(100));
    }

    drop(events_lock);

    // Dropped events must have incremented
    assert!(
        ctx.cache
            .counters
            .diagnostic_events_dropped
            .load(Ordering::Relaxed)
            >= 10
    );
}

#[tokio::test]
async fn test_m1_t23_partial_finalization_failure_unwinds_safely() {
    let ctx = TestContext::new().await;
    let key = CacheKey::new(None, "/item23", None);

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();

    let reg = match ctx.cache.begin_fetch_atomic(&key, 100) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    reg.shared_op.waiters.lock().unwrap().push(tx1);
    reg.shared_op.waiters.lock().unwrap().push(tx2);
    let guard = reg.into_guard();

    // Trigger injected panic after waiter extraction
    ctx.cache
        .test_panic_after_waiter_extraction
        .store(true, Ordering::Relaxed);

    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body: Arc::new(b"{\"hello\":\"world\"}".to_vec()),
        etag: None,
        created_at_ms: 100,
        ttl_ms: 60_000,
    });

    let task = tokio::spawn(async move {
        guard.complete(FinalizeOutcome::Success(res));
    });
    let _ = task.await;

    ctx.cache
        .test_panic_after_waiter_extraction
        .store(false, Ordering::Relaxed);

    // Both waiters must have been resolved via stack unwinding
    let out1 = rx1.await.unwrap().unwrap();
    let out2 = rx2.await.unwrap().unwrap();
    assert_eq!(*out1.body, b"{\"hello\":\"world\"}");
    assert_eq!(*out2.body, b"{\"hello\":\"world\"}");

    // 1. Verify in-flight map is clean (recovering poisoned shard mutex)
    let idx = ctx.cache.get_shard_index(&key);
    {
        let shard = ctx.cache.shards[idx]
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        assert!(
            !shard.in_flight.contains_key(&key),
            "in_flight map must not contain leaked entry"
        );
        // 2. Verify state was reclaimed (active_op_ids removed without leaks)
        assert!(
            !shard.map.contains_key(&key),
            "state map must be reclaimed after panic with zero active_op_ids"
        );
    }

    // 3. Verify finalization_panics and lock_poison_recovered counters incremented
    assert!(
        ctx.cache
            .counters
            .finalization_panics
            .load(Ordering::Relaxed)
            >= 1,
        "finalization_panics counter must increment"
    );

    // 4. Verify successful fresh request on the same key after panic recovery
    let fresh_url = format!("{}/api/v1/cache/item23", ctx.base_url);
    let fresh_resp = ctx.client.get(&fresh_url).send().await.unwrap();
    assert_eq!(fresh_resp.status(), StatusCode::OK);
    assert_eq!(
        fresh_resp.headers().get("X-Cache-Status").unwrap(),
        "MISS",
        "Fresh request after panic recovery must initiate fresh fetch"
    );

    // 5. Subsequent request must be a cache HIT (fresh fetch populated cache cleanly)
    let hit_resp = ctx.client.get(&fresh_url).send().await.unwrap();
    assert_eq!(hit_resp.status(), StatusCode::OK);
    assert_eq!(
        hit_resp.headers().get("X-Cache-Status").unwrap(),
        "HIT",
        "Subsequent request must hit the cleanly populated cache"
    );
}

#[tokio::test]
async fn test_m1_t24_unique_key_flood_and_sieve_capacity_eviction() {
    let ctx = TestContext::new().await;
    // Set per-shard capacity to 3 entries
    ctx.cache.set_capacity_limits(3, 100);

    // Target shard 0 specifically by finding keys that hash to shard 0
    let mut shard_0_keys = Vec::new();
    for i in 0..100 {
        let key = CacheKey::new(None, &format!("/flood_{}", i), None);
        if ctx.cache.get_shard_index(&key) == 0 {
            shard_0_keys.push(format!("flood_{}", i));
            if shard_0_keys.len() == 6 {
                break;
            }
        }
    }
    assert_eq!(shard_0_keys.len(), 6, "Must find 6 keys targeting shard 0");

    // 1. Send slow request on key 0 to hold an active in-flight operation on shard 0
    let slow_key = &shard_0_keys[0];
    let slow_url = format!("{}/api/v1/cache/{}", ctx.base_url, slow_key);
    mock_plan(slow_key, Plan::delay(300));
    let client_slow = ctx.client.clone();
    let slow_handle = tokio::spawn(async move {
        client_slow
            .get(&slow_url)
            .send()
            .await
            .unwrap()
    });

    // Allow slow request to register and become in-flight
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify key 0 is currently in-flight
    let slow_cache_key = CacheKey::new(None, &format!("/{}", slow_key), None);
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert!(
            shard.in_flight.contains_key(&slow_cache_key),
            "Slow key must be in-flight"
        );
    }

    // 2. Flood remaining 5 keys into shard 0 (capacity is 3)
    for key in &shard_0_keys[1..] {
        let url = format!("{}/api/v1/cache/{}", ctx.base_url, key);
        let resp = ctx.client.get(&url).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // 3. Assert eviction counter incremented
    let evictions = ctx.cache.counters.evictions_total.load(Ordering::Relaxed);
    assert!(
        evictions >= 2,
        "Evictions total must be at least 2 due to capacity overflow, got {}",
        evictions
    );

    // 4. Assert shard 0 active state was NEVER evicted
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        // INVARIANT: The actively in-flight slow key must NEVER have been evicted
        assert!(
            shard.in_flight.contains_key(&slow_cache_key),
            "Active in-flight operation must NEVER be evicted"
        );
        assert!(
            shard.map.contains_key(&slow_cache_key),
            "Active KeyState must NEVER be evicted"
        );
    }

    // 5. Await slow request completion; must succeed
    let slow_resp = slow_handle.await.unwrap();
    assert_eq!(slow_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_m1_t25_oversized_upstream_response_rejected() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/oversized_item", ctx.base_url);
    mock_plan("oversized_item", Plan::oversized());

    // Request with x-mock-oversized triggers 3 MB upstream payload (limit is 2 MB)
    let resp = ctx
        .client
        .get(&url)
        .send()
        .await
        .unwrap();

    // Assert rejection with 502 Bad Gateway and typed header
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        resp.headers().get("X-Cache-Error").unwrap(),
        "ResponseTooLarge",
        "Must return ResponseTooLarge error header"
    );

    // Verify oversized response was NEVER cached
    let key = CacheKey::new(None, "/oversized_item", None);
    let cached = ctx.cache.get_resource(&key, DiagnosticBuffer::now_ms());
    assert!(
        cached.is_none(),
        "Oversized response must not be stored in cache"
    );

    // Verify subsequent normal request to same key succeeds cleanly
    let fresh_resp = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(fresh_resp.status(), StatusCode::OK);
    assert_eq!(
        fresh_resp.headers().get("X-Cache-Status").unwrap(),
        "MISS"
    );
}

#[tokio::test]
async fn test_m1_t26_ingress_request_body_limit() {
    let ctx = TestContext::new().await;
    let url = format!("{}/api/v1/cache/item26", ctx.base_url);

    // Send request with body larger than 512 KB (600 KB)
    let oversized_body = vec![b'x'; 600 * 1024];
    let resp = ctx
        .client
        .get(&url)
        .body(oversized_body)
        .send()
        .await
        .unwrap();

    // Axum DefaultBodyLimit must reject with 413 Payload Too Large
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    // Normal request without body must succeed
    let ok_resp = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(ok_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_m1_t27_waiter_saturation_and_cleanup_after_rejection() {
    let ctx = TestContext::new().await;
    // Set max waiters per key to 3 (1 initiator + 2 coalesced waiters)
    ctx.cache.set_capacity_limits(1000, 3);

    let url = format!("{}/api/v1/cache/crowded_key", ctx.base_url);
    mock_plan("crowded_key", Plan::delay(250));

    // Initiator starts slow fetch (250ms)
    let client1 = ctx.client.clone();
    let url1 = url.clone();
    let initiator = tokio::spawn(async move {
        client1
            .get(&url1)
            .send()
            .await
            .unwrap()
    });

    // Give initiator time to register
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Dispatch Waiter 1 (accepted - waiter 1/2)
    let client2 = ctx.client.clone();
    let url2 = url.clone();
    let waiter1 = tokio::spawn(async move { client2.get(&url2).send().await.unwrap() });

    // Dispatch Waiter 2 (accepted - waiter 2/2)
    let client3 = ctx.client.clone();
    let url3 = url.clone();
    let waiter2 = tokio::spawn(async move { client3.get(&url3).send().await.unwrap() });

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Dispatch Waiter 3 (rejected - capacity saturated)
    let rejected1 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(
        rejected1.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "Saturated waiter must receive 503 Service Unavailable"
    );
    assert_eq!(
        rejected1.headers().get("X-Cache-Error").unwrap(),
        "WaiterLimitExceeded"
    );
    assert_eq!(rejected1.headers().get("Retry-After").unwrap(), "1");

    // Dispatch Waiter 4 (rejected - capacity saturated)
    let rejected2 = ctx.client.get(&url).send().await.unwrap();
    assert_eq!(rejected2.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Verify counter
    let rejections = ctx
        .cache
        .counters
        .waiter_limit_rejections
        .load(Ordering::Relaxed);
    assert!(
        rejections >= 2,
        "Expected at least 2 waiter limit rejections, got {}",
        rejections
    );

    // Initiator and accepted waiters must succeed
    let init_resp = initiator.await.unwrap();
    assert_eq!(init_resp.status(), StatusCode::OK);
    assert_eq!(init_resp.headers().get("X-Cache-Status").unwrap(), "MISS");

    let w1_resp = waiter1.await.unwrap();
    assert_eq!(w1_resp.status(), StatusCode::OK);
    assert_eq!(w1_resp.headers().get("X-Cache-Status").unwrap(), "COALESCED");

    let w2_resp = waiter2.await.unwrap();
    assert_eq!(w2_resp.status(), StatusCode::OK);
    assert_eq!(w2_resp.headers().get("X-Cache-Status").unwrap(), "COALESCED");

    // Post-completion check: zero in-flight entries leaked
    let key = CacheKey::new(None, "/crowded_key", None);
    let idx = ctx.cache.get_shard_index(&key);
    {
        let shard = ctx.cache.shards[idx].lock().unwrap();
        assert!(
            !shard.in_flight.contains_key(&key),
            "In-flight entry must be cleaned up"
        );
    }
}

#[tokio::test]
async fn test_m1_t28_strict_capacity_saturation_rejects_when_no_eviction_candidate() {
    let ctx = TestContext::new().await;
    // Set per-shard capacity to exactly 2 entries
    ctx.cache.set_capacity_limits(2, 100);

    // Find 3 keys that hash to shard 0
    let mut shard_0_keys = Vec::new();
    for i in 0..100 {
        let key = CacheKey::new(None, &format!("/cap_{}", i), None);
        if ctx.cache.get_shard_index(&key) == 0 {
            shard_0_keys.push(format!("cap_{}", i));
            if shard_0_keys.len() == 3 {
                break;
            }
        }
    }
    assert_eq!(shard_0_keys.len(), 3);

    // 1. Start two slow concurrent requests to saturate shard 0 with active in-flight operations
    let key_a = shard_0_keys[0].clone();
    let url_a = format!("{}/api/v1/cache/{}", ctx.base_url, key_a);
    mock_plan(&key_a, Plan::delay(300));
    let client_a = ctx.client.clone();
    let handle_a = tokio::spawn(async move {
        client_a
            .get(&url_a)
            .send()
            .await
            .unwrap()
    });

    let key_b = shard_0_keys[1].clone();
    let url_b = format!("{}/api/v1/cache/{}", ctx.base_url, key_b);
    mock_plan(&key_b, Plan::delay(300));
    let client_b = ctx.client.clone();
    let handle_b = tokio::spawn(async move {
        client_b
            .get(&url_b)
            .send()
            .await
            .unwrap()
    });

    // Give both tasks time to register in shard 0
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify shard 0 is at full capacity (len == 2) and both entries are in-flight
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert_eq!(shard.map.len(), 2, "Shard 0 must be exactly at capacity 2");
        assert_eq!(shard.in_flight.len(), 2, "Both operations must be in-flight");
    }

    // 2. Dispatch 3rd request (Key C) targeting shard 0.
    // Since both existing entries have live in-flight operations, neither is eligible for eviction.
    // STRICT CAPACITY INVARIANT must reject Key C without exceeding max_entries_per_shard!
    let key_c = &shard_0_keys[2];
    let url_c = format!("{}/api/v1/cache/{}", ctx.base_url, key_c);
    let resp_c = ctx.client.get(&url_c).send().await.unwrap();

    assert_eq!(
        resp_c.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "Request must be rejected with 503 when shard capacity is saturated by active operations"
    );
    assert_eq!(
        resp_c.headers().get("X-Cache-Error").unwrap(),
        "CapacityExceeded"
    );
    assert_eq!(resp_c.headers().get("Retry-After").unwrap(), "1");

    // 3. Assert shard 0 map length is STILL strictly <= 2 (NEVER exceeded)
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert!(
            shard.map.len() <= 2,
            "Shard 0 capacity must NOT be exceeded even under saturation"
        );
    }

    // 4. Await completion of original operations (Keys A and B) - both must succeed
    let res_a = handle_a.await.unwrap();
    assert_eq!(res_a.status(), StatusCode::OK);
    let res_b = handle_b.await.unwrap();
    assert_eq!(res_b.status(), StatusCode::OK);

    // 5. Now that Keys A and B completed, they are eligible for eviction.
    // Retrying Key C must now succeed (evicting one of the idle cached keys).
    let retry_c = ctx.client.get(&url_c).send().await.unwrap();
    assert_eq!(retry_c.status(), StatusCode::OK);
    assert_eq!(retry_c.headers().get("X-Cache-Status").unwrap(), "MISS");

    // 6. Assert capacity_rejections counter recorded the rejection
    assert!(
        ctx.cache
            .counters
            .capacity_rejections
            .load(Ordering::Relaxed)
            >= 1
    );
}

#[tokio::test]
async fn test_m1_t29_response_size_edge_cases() {
    // Case 1: Oversized Content-Length header (100 MB)
    let raw_resp1 = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 104857600\r\nConnection: close\r\n\r\n{\"hello\":\"world\"}".to_vec();
    let origin1 = spawn_raw_tcp_origin(raw_resp1).await;
    let ctx1 = TestContext::with_origin(origin1, Arc::new(AtomicU64::new(0))).await;
    let resp1 = ctx1
        .client
        .get(format!("{}/api/v1/cache/item_cl", ctx1.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        resp1.headers().get("X-Cache-Error").unwrap(),
        "ResponseTooLarge"
    );

    // Case 2: Truncated response body (CL claims 5000 bytes, but socket only sends 5 bytes)
    let raw_resp2 = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 5000\r\nConnection: close\r\n\r\nshort".to_vec();
    let origin2 = spawn_raw_tcp_origin(raw_resp2).await;
    let ctx2 = TestContext::with_origin(origin2, Arc::new(AtomicU64::new(0))).await;
    let resp2 = ctx2
        .client
        .get(format!("{}/api/v1/cache/item_trunc", ctx2.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::BAD_GATEWAY);
    let err_hdr = resp2.headers().get("X-Cache-Error").unwrap().to_str().unwrap();
    assert!(
        err_hdr == "TruncatedResponse" || err_hdr == "NetworkError",
        "Unexpected error header: {}",
        err_hdr
    );

    // Case 3: Chunked Transfer-Encoding exceeding 2 MB (two 1 MB+ chunks)
    let chunk1_size = 1024 * 1024;
    let chunk2_size = 1024 * 1024 + 1024;
    let chunked_raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        chunk1_size,
        "A".repeat(chunk1_size),
        chunk2_size,
        "B".repeat(chunk2_size)
    ).into_bytes();
    let origin3 = spawn_raw_tcp_origin(chunked_raw).await;
    let ctx3 = TestContext::with_origin(origin3, Arc::new(AtomicU64::new(0))).await;
    let resp3 = ctx3
        .client
        .get(format!("{}/api/v1/cache/item_chunk", ctx3.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp3.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        resp3.headers().get("X-Cache-Error").unwrap(),
        "ResponseTooLarge"
    );

    // Case 4: Boundary test - Exact 2 MB body accepted (2097152 bytes)
    let exact_2mb = "A".repeat(2 * 1024 * 1024);
    let exact_raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2097152\r\nConnection: close\r\n\r\n{}",
        exact_2mb
    ).into_bytes();
    let origin4 = spawn_raw_tcp_origin(exact_raw).await;
    let ctx4 = TestContext::with_origin(origin4, Arc::new(AtomicU64::new(0))).await;
    let resp4 = ctx4
        .client
        .get(format!("{}/api/v1/cache/item_exact", ctx4.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp4.status(), StatusCode::OK);
    assert_eq!(
        resp4.headers().get("X-Cache-Status").unwrap(),
        "MISS"
    );

    // Verify exact 2 MB body was cached cleanly
    let hit4 = ctx4
        .client
        .get(format!("{}/api/v1/cache/item_exact", ctx4.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(hit4.status(), StatusCode::OK);
    assert_eq!(
        hit4.headers().get("X-Cache-Status").unwrap(),
        "HIT"
    );

    // Case 5: Boundary test - 2 MB + 1 byte rejected as ResponseTooLarge (2097153 bytes)
    let exceeded_2mb = "A".repeat(2 * 1024 * 1024 + 1);
    let exceeded_raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2097153\r\nConnection: close\r\n\r\n{}",
        exceeded_2mb
    ).into_bytes();
    let origin5 = spawn_raw_tcp_origin(exceeded_raw).await;
    let ctx5 = TestContext::with_origin(origin5, Arc::new(AtomicU64::new(0))).await;
    let resp5 = ctx5
        .client
        .get(format!("{}/api/v1/cache/item_exceeded", ctx5.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp5.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        resp5.headers().get("X-Cache-Error").unwrap(),
        "ResponseTooLarge"
    );
}

#[tokio::test]
async fn test_m1_t30_capacity_reconfiguration_immediate_contraction() {
    let ctx = TestContext::new().await;
    // Set initial high capacity
    ctx.cache.set_capacity_limits(20, 100);

    // Populate 10 items in shard 0
    let mut shard_0_keys = Vec::new();
    for i in 0..200 {
        let key = CacheKey::new(None, &format!("/reconfig_{}", i), None);
        if ctx.cache.get_shard_index(&key) == 0 {
            shard_0_keys.push(format!("reconfig_{}", i));
            if shard_0_keys.len() == 10 {
                break;
            }
        }
    }
    assert_eq!(shard_0_keys.len(), 10);

    for k in &shard_0_keys {
        let resp = ctx
            .client
            .get(format!("{}/api/v1/cache/{}", ctx.base_url, k))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Verify shard 0 currently holds 10 items
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert_eq!(shard.map.len(), 10);
    }

    // Reconfigure capacity down to 3 per shard
    ctx.cache.set_capacity_limits(3, 100);

    // Assert that shard 0 immediately contracted down to 3 without waiting for misses
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert!(
            shard.map.len() <= 3,
            "Shard 0 must immediately contract to <= 3 upon reconfiguration, got {}",
            shard.map.len()
        );
    }

    let evictions = ctx.cache.counters.evictions_total.load(Ordering::Relaxed);
    assert!(
        evictions >= 7,
        "Expected at least 7 evictions during immediate contraction, got {}",
        evictions
    );
}

#[tokio::test]
async fn test_m1_t31_key_and_tenant_size_limits() {
    let ctx = TestContext::new().await;

    // Case 1: Tenant ID exceeding 128 bytes
    let huge_tenant = "T".repeat(200);
    let resp1 = ctx
        .client
        .get(format!("{}/api/v1/cache/valid_item", ctx.base_url))
        .header("x-tenant-id", huge_tenant)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp1.headers().get("X-Cache-Error").unwrap(),
        "TenantIdTooLong"
    );

    // Case 2: URI path exceeding 2048 bytes
    let huge_path = "p".repeat(2500);
    let resp2 = ctx
        .client
        .get(format!("{}/api/v1/cache/{}", ctx.base_url, huge_path))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::URI_TOO_LONG);
    assert_eq!(
        resp2.headers().get("X-Cache-Error").unwrap(),
        "UriTooLong"
    );

    // Case 3: Query string exceeding 4096 bytes
    let huge_query = format!("search={}", "q".repeat(5000));
    let resp3 = ctx
        .client
        .get(format!("{}/api/v1/cache/valid_item?{}", ctx.base_url, huge_query))
        .send()
        .await
        .unwrap();
    assert_eq!(resp3.status(), StatusCode::URI_TOO_LONG);
    assert_eq!(
        resp3.headers().get("X-Cache-Error").unwrap(),
        "UriTooLong"
    );

    // Case 4: Valid normal tenant and path succeeds
    let resp4 = ctx
        .client
        .get(format!("{}/api/v1/cache/valid_item?search=ok", ctx.base_url))
        .header("x-tenant-id", "valid_tenant_1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp4.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_m1_t32_capacity_reduction_while_operations_active() {
    let ctx = TestContext::new().await;
    ctx.cache.set_capacity_limits(10, 100);

    // Find keys that map to shard 0
    let mut shard_0_keys = Vec::new();
    for i in 0..200 {
        let key = CacheKey::new(None, &format!("/active_reconfig_{}", i), None);
        if ctx.cache.get_shard_index(&key) == 0 {
            shard_0_keys.push(key);
            if shard_0_keys.len() == 6 {
                break;
            }
        }
    }
    assert_eq!(shard_0_keys.len(), 6);

    // 1. Insert 3 completed idle entries in shard 0
    for key in &shard_0_keys[0..3] {
        if let FetchRegistrationResult::Initiate(reg) = ctx.cache.begin_fetch_atomic(key, 1000) {
            let res = Arc::new(CachedResource {
                status: 200,
                content_type: "application/json".to_string(),
                body: Arc::new(vec![1, 2, 3]),
                etag: None,
                created_at_ms: 1000,
                ttl_ms: 60_000,
            });
            reg.into_guard().complete(FinalizeOutcome::Success(res));
        }
    }

    // 2. Start 2 active in-flight fetches on shard 0 and keep guards alive
    let active_reg_1 = match ctx.cache.begin_fetch_atomic(&shard_0_keys[3], 1000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate for key 3"),
    };
    let active_guard_1 = active_reg_1.into_guard();

    let active_reg_2 = match ctx.cache.begin_fetch_atomic(&shard_0_keys[4], 1000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate for key 4"),
    };
    let active_guard_2 = active_reg_2.into_guard();

    // Verify shard 0 state before capacity reduction:
    // 3 completed idle entries + 2 active in-flight entries = 5 entries
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert_eq!(shard.map.len(), 5);
        assert_eq!(shard.in_flight.len(), 2);
    }

    // 3. REDUCE CAPACITY DOWN TO 1 while operations are actively in flight!
    ctx.cache.set_capacity_limits(1, 100);

    // Verify immediate contraction effect:
    // - All 3 idle entries were immediately evicted
    // - The 2 active in-flight operations were STRICTLY PRESERVED (never evicted)
    {
        let shard = ctx.cache.shards[0].lock().unwrap();
        assert_eq!(
            shard.map.len(),
            2,
            "Contraction must preserve exactly the 2 active operations"
        );
        assert!(shard.map.contains_key(&shard_0_keys[3]));
        assert!(shard.map.contains_key(&shard_0_keys[4]));
        assert_eq!(shard.in_flight.len(), 2);
    }

    // 4. Complete the active operations safely
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(vec![1, 2, 3]),
        etag: None,
        created_at_ms: 1000,
        ttl_ms: 60_000,
    });
    active_guard_1.complete(FinalizeOutcome::Success(res.clone()));
    active_guard_2.complete(FinalizeOutcome::Success(res.clone()));

    // 5. Shard 0 now holds 2 entries (exceeding new limit 1).
    // A fresh miss on key 5 must evict one of the idle entries down to 1!
    match ctx.cache.begin_fetch_atomic(&shard_0_keys[5], 2000) {
        FetchRegistrationResult::Initiate(reg) => {
            reg.into_guard().complete(FinalizeOutcome::Success(res));
        }
        _ => panic!("Expected Initiate after eviction down to limit"),
    }
}
