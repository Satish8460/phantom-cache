//! Known-gap and characterization tests for PhantomCache Phase 1 (WP-00).
//!
//! Specifications:
//! - docs/research/phase_1_execution_package.md v3.1 §6.2
//! - docs/research/adr/ADR-001-present-architecture-and-direction.md
//!
//! Evidence Notes:
//! - KG-2: `reconcile_peer_anti_entropy` has no production caller in `src/`.
//!   Verified: definition at `src/dist/router.rs:787`; called only in unit/integration tests.
//! - TG-2: `key_hash` is hardcoded to 0 (`server.rs:482,770,946`); p95 field holds p90
//!   percentile (`server.rs:187,1665`); throughput is cumulative counter over uptime (`server.rs:181`).
//! - PIE-1: `PieController` is not constructed in `main()` (`main.rs:306`); `pie: None` is
//!   passed to `ApiState`. `src/control/pie.rs` remains untouched legacy stub in Phase 1.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::anti_entropy::AntiEntropyTracker;
use phantom_cache_demo::dist::frame::{read_frame, write_frame, InvalidationScope, PhantomMeshFrame};
use phantom_cache_demo::dist::node::VirtualNode;
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::intel::telemetry::{ObservationKind, TelemetryCollector};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;

// ---------------------------------------------------------------------------
// Helpers: Mock Origin
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockOriginState {
    request_count: Arc<AtomicU64>,
}

async fn mock_handler(
    State(state): State<MockOriginState>,
    path: Option<Path<String>>,
    _headers: HeaderMap,
) -> Response {
    let count = state.request_count.fetch_add(1, Ordering::SeqCst) + 1;
    let path_str = path.map(|p| p.0).unwrap_or_default();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "key": path_str,
            "req_num": count
        })),
    )
        .into_response()
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

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://127.0.0.1:{}", addr.port()), request_count)
}

fn make_dummy_snapshot(nodes: &[Arc<VirtualNode>]) -> SimulationSnapshot {
    SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 1000.0,
        p50_latency_ms: 1,
        p95_latency_ms: 2,
        p99_latency_ms: 5,
        db_load_ratio: 0.1,
        avoided_compute_dollars: 0.0,
        nodes: nodes
            .iter()
            .map(|n| NodeSnapshot {
                node_id: n.node_id,
                state: "Active".to_string(),
                cpu_utilization: 10.0,
                memory_used_bytes: 1024,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// KG-1 & PC-1: Peer invalidation does not purge other gateway HTTP cache
// ---------------------------------------------------------------------------

#[tokio::test]
async fn kg1_peer_invalidation_must_purge_other_gateways_http_cache() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, origin_count) = spawn_mock_origin().await;

        let cache_a = Arc::new(PartitionedCache::new(4));
        let cache_b = Arc::new(PartitionedCache::new(4));

        // Node 1 (Gateway A local node), Node 2 (Gateway B local node)
        // Unified with serving caches via new_with_cache (C-01 / ADR-003)
        let node1 = Arc::new(VirtualNode::new_with_cache(1, cache_a.clone()));
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache_b.clone()));
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);

        let cluster_nodes = vec![node1.clone(), node2.clone()];
        let router_a = Arc::new(RendezvousRouter::new(cluster_nodes.clone()));
        let router_b = Arc::new(RendezvousRouter::new(cluster_nodes.clone()));

        let state_a = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&cluster_nodes)),
            nodes: cluster_nodes.clone(),
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache_a),
            origin_url: Some(origin_url.clone()),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router_a.clone()),
        });

        let state_b = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&cluster_nodes)),
            nodes: cluster_nodes.clone(),
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache_b),
            origin_url: Some(origin_url.clone()),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router_b.clone()),
        });

        let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_gw_a = listener_a.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener_a, create_app(state_a)).await.unwrap();
        });

        let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_gw_b = listener_b.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener_b, create_app(state_b)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let url_b = format!("http://127.0.0.1:{}/api/v1/cache/kg1_test_item", addr_gw_b.port());
        let url_a = format!("http://127.0.0.1:{}/api/v1/cache/kg1_test_item", addr_gw_a.port());

        // PRECONDITION 1: Initial GET on B fetches from origin (origin count == 1)
        let resp1 = client.get(&url_b).send().await.expect("KG-1 PRECONDITION: GET on B failed");
        assert_eq!(resp1.status(), StatusCode::OK, "KG-1 PRECONDITION: GET on B returned non-200");
        assert_eq!(origin_count.load(Ordering::SeqCst), 1, "KG-1 PRECONDITION: initial origin count must be 1");

        // PRECONDITION 2: Second GET on B is served from cache (origin count remains 1, HIT)
        let resp2 = client.get(&url_b).send().await.expect("KG-1 PRECONDITION: second GET on B failed");
        assert_eq!(resp2.status(), StatusCode::OK, "KG-1 PRECONDITION: second GET on B returned non-200");
        assert_eq!(
            resp2.headers().get("x-cache-status").and_then(|h| h.to_str().ok()),
            Some("HIT"),
            "KG-1 PRECONDITION: second GET on B must be a cache HIT"
        );
        assert_eq!(origin_count.load(Ordering::SeqCst), 1, "KG-1 PRECONDITION: origin count must remain 1 on hit");

        // PRECONDITION 3: DELETE on Gateway A returns 204 No Content
        let del_resp = client.delete(&url_a).send().await.expect("KG-1 PRECONDITION: DELETE on A failed");
        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT, "KG-1 PRECONDITION: DELETE on A must return 204");

        // DEFECT ASSERTION: After DELETE on A, a GET on B must trigger a fresh origin fetch (origin count == 2).
        // Today, peer invalidation does not purge Gateway B's HTTP cache, so B serves the stale cached item.
        let resp3 = client.get(&url_b).send().await.expect("KG-1 PRECONDITION: post-delete GET on B failed");
        assert_eq!(resp3.status(), StatusCode::OK);
        let final_origin_calls = origin_count.load(Ordering::SeqCst);
        assert_eq!(
            final_origin_calls,
            2,
            "KG-1 DEFECT: peer invalidation on Gateway A did not purge Gateway B's HTTP cache (expected origin count 2, got {})",
            final_origin_calls
        );

        // Teardown
        node1.stop_tcp_service();
        node2.stop_tcp_service();
        router_a.stop();
        router_b.stop();
    })
    .await
    .expect("KG-1 test timed out after 10s");
}

#[tokio::test]
async fn pc1_standalone_delete_purges_local_gateway_entry() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, origin_count) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&[])),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None, // Standalone gateway
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/cache/pc1_item", gw_addr.port());

        // 1. GET -> origin count = 1
        let r1 = client.get(&url).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 1);

        // 2. GET -> cached, origin count = 1
        let r2 = client.get(&url).send().await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 1);

        // 3. DELETE on same gateway -> 204
        let del = client.delete(&url).send().await.unwrap();
        assert_eq!(del.status(), StatusCode::NO_CONTENT);

        // 4. GET -> refetched from origin, origin count = 2
        let r3 = client.get(&url).send().await.unwrap();
        assert_eq!(r3.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 2);
    })
    .await
    .expect("PC-1 test timed out after 10s");
}

// ---------------------------------------------------------------------------
// KG-3 & PC-3: Gap handling advances highest_acked_seq past missing sequence
// ---------------------------------------------------------------------------

#[test]
fn kg3_gap_handling_advances_past_missing_sequence() {
    let mut tracker = AntiEntropyTracker::new();
    tracker.record_inbound_seq(7, 1, None);
    tracker.record_inbound_seq(7, 2, None);
    tracker.record_inbound_seq(7, 5, None); // Missed sequences 3 and 4

    // PRECONDITION: pop_gap() must return the detected gap (7, 3, 4)
    let gap = tracker.pop_gap();
    assert_eq!(
        gap,
        Some((7, 3, 4)),
        "KG-3 PRECONDITION: tracker did not report gap (7, 3, 4)"
    );

    // DEFECT: get_highest_acked_seq must be the last contiguous sequence (2).
    // Today, it erroneously advances to 5, discarding gap recovery.
    let highest = tracker.get_highest_acked_seq(7);
    assert_eq!(
        highest, 2,
        "KG-3 DEFECT: highest acked sequence advanced past missing sequence (expected 2, got {})",
        highest
    );
}

#[test]
fn pc3_sequential_inbound_updates_highest_acked_seq() {
    let mut tracker = AntiEntropyTracker::new();
    tracker.record_inbound_seq(7, 1, None);
    tracker.record_inbound_seq(7, 2, None);
    tracker.record_inbound_seq(7, 3, None);

    assert_eq!(tracker.get_highest_acked_seq(7), 3);
    assert_eq!(tracker.pop_gap(), None);
    assert!(!tracker.has_gaps());
}

// ---------------------------------------------------------------------------
// KG-4 & PC-4: Shared stream sequence causes false interleaved gaps
// ---------------------------------------------------------------------------

#[tokio::test]
async fn kg4_shared_stream_sequence_causes_false_interleaved_gaps() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let node1 = Arc::new(VirtualNode::new(1));
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        let node2 = Arc::new(VirtualNode::new(2));
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);

        let cluster_nodes = vec![node1.clone(), node2.clone()];
        let router = Arc::new(RendezvousRouter::new(cluster_nodes));

        // Alternately send invalidation frames to node1 and node2.
        // stream_sequence is shared on the router (fetches: 1 to node1, 2 to node2, 3 to node1, 4 to node2).
        let (st1, _) = router
            .send_invalidation_frame(&node1, "key1", InvalidationScope::ExactKey, 2000)
            .await
            .expect("KG-4 PRECONDITION: frame 1 to node1 failed");
        assert!(st1.is_success(), "KG-4 PRECONDITION: frame 1 status not success");

        let (st2, _) = router
            .send_invalidation_frame(&node2, "key2", InvalidationScope::ExactKey, 2000)
            .await
            .expect("KG-4 PRECONDITION: frame 2 to node2 failed");
        assert!(st2.is_success(), "KG-4 PRECONDITION: frame 2 status not success");

        let (st3, _) = router
            .send_invalidation_frame(&node1, "key3", InvalidationScope::ExactKey, 2000)
            .await
            .expect("KG-4 PRECONDITION: frame 3 to node1 failed");
        assert!(st3.is_success(), "KG-4 PRECONDITION: frame 3 status not success");

        let (st4, _) = router
            .send_invalidation_frame(&node2, "key4", InvalidationScope::ExactKey, 2000)
            .await
            .expect("KG-4 PRECONDITION: frame 4 to node2 failed");
        assert!(st4.is_success(), "KG-4 PRECONDITION: frame 4 status not success");

        // Small yield to allow message processing on node tasks
        tokio::time::sleep(Duration::from_millis(50)).await;

        // DEFECT ASSERTION: With zero packet loss, neither node should detect a sequence gap.
        // Today, because stream_sequence is global per router rather than per peer connection,
        // node1 receives seqs 1 and 3 (detects missing 2), and node2 receives 2 and 4 (detects missing 3).
        let gaps_node1 = node1
            .cache
            .counters
            .anti_entropy_gaps_detected
            .load(Ordering::SeqCst);
        let gaps_node2 = node2
            .cache
            .counters
            .anti_entropy_gaps_detected
            .load(Ordering::SeqCst);

        assert_eq!(
            gaps_node1 + gaps_node2,
            0,
            "KG-4 DEFECT: false gaps detected due to shared stream_seq (node1 gaps: {}, node2 gaps: {})",
            gaps_node1,
            gaps_node2
        );

        node1.stop_tcp_service();
        node2.stop_tcp_service();
        router.stop();
    })
    .await
    .expect("KG-4 test timed out after 10s");
}

#[tokio::test]
async fn pc4_single_target_stream_sequence_has_zero_gaps() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let node1 = Arc::new(VirtualNode::new(1));
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        let router = Arc::new(RendezvousRouter::new(vec![node1.clone()]));

        // Sending consecutively to node1 only gives contiguous stream sequences 1, 2, 3
        let (s1, _) = router
            .send_invalidation_frame(&node1, "k1", InvalidationScope::ExactKey, 2000)
            .await
            .unwrap();
        let (s2, _) = router
            .send_invalidation_frame(&node1, "k2", InvalidationScope::ExactKey, 2000)
            .await
            .unwrap();
        let (s3, _) = router
            .send_invalidation_frame(&node1, "k3", InvalidationScope::ExactKey, 2000)
            .await
            .unwrap();
        assert!(s1.is_success() && s2.is_success() && s3.is_success());

        tokio::time::sleep(Duration::from_millis(50)).await;

        let gaps = node1
            .cache
            .counters
            .anti_entropy_gaps_detected
            .load(Ordering::SeqCst);
        assert_eq!(gaps, 0, "Single-target contiguous sequence must have 0 gaps");

        node1.stop_tcp_service();
        router.stop();
    })
    .await
    .expect("PC-4 test timed out after 10s");
}

// ---------------------------------------------------------------------------
// KG-5 & PC-5: DELETE purges local entry only after cluster round trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn kg5_get_after_delete_start_must_not_serve_old_entry() {
    std::env::set_var("PHANTOM_CLUSTER_TIMEOUT_MS", "1000");
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, origin_count) = spawn_mock_origin().await;

        let node1 = Arc::new(VirtualNode::new(1));
        node1.set_artificial_delay(400); // 400ms delay in cluster processing
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        let cluster_nodes = vec![node1.clone()];
        let router = Arc::new(RendezvousRouter::new(cluster_nodes.clone()));
        let cache = Arc::new(PartitionedCache::new(4));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&cluster_nodes)),
            nodes: cluster_nodes.clone(),
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router.clone()),
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let item_url = format!("http://127.0.0.1:{}/api/v1/cache/kg5_item", gw_addr.port());

        // Prefill cache via GET -> 200, origin calls == 1
        let r1 = client.get(&item_url).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 1);

        // Spawn DELETE in background
        let c_del = client.clone();
        let u_del = item_url.clone();
        let del_handle = tokio::spawn(async move { c_del.delete(&u_del).send().await });

        // Sleep ~100 ms: DELETE should still be in-flight due to the 400 ms artificial delay
        tokio::time::sleep(Duration::from_millis(100)).await;

        // PRECONDITION: DELETE must still be in flight
        assert!(
            !del_handle.is_finished(),
            "KG-5 PRECONDITION: DELETE completed prematurely before GET was issued"
        );

        // Issue GET while DELETE is in flight
        let r_mid = client.get(&item_url).send().await.unwrap();
        let mid_status = r_mid.headers().get("x-cache-status").and_then(|h| h.to_str().ok());

        // DEFECT ASSERTION: Desired architecture requires immediate local eviction or invalidation marker
        // so that in-flight DELETE never serves stale pre-DELETE body.
        // Today, local delete is deferred until after cluster round-trip succeeds (server.rs:832),
        // so r_mid returns 200 HIT.
        assert_ne!(
            mid_status,
            Some("HIT"),
            "KG-5 DEFECT: GET served stale cached entry while cluster DELETE was in flight (got cache HIT)"
        );

        let del_res = del_handle.await.unwrap().unwrap();
        assert_eq!(del_res.status(), StatusCode::NO_CONTENT);

        node1.stop_tcp_service();
        router.stop();
    })
    .await
    .expect("KG-5 test timed out after 10s");
}

#[tokio::test]
async fn pc5_get_after_delete_completes_refetches() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, origin_count) = spawn_mock_origin().await;

        let node1 = Arc::new(VirtualNode::new(1));
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        let cluster_nodes = vec![node1.clone()];
        let router = Arc::new(RendezvousRouter::new(cluster_nodes.clone()));
        let cache = Arc::new(PartitionedCache::new(4));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&cluster_nodes)),
            nodes: cluster_nodes.clone(),
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router.clone()),
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let item_url = format!("http://127.0.0.1:{}/api/v1/cache/pc5_item", gw_addr.port());

        // 1. Prefill
        let r1 = client.get(&item_url).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 1);

        // 2. DELETE with delay 0 completes
        let del = client.delete(&item_url).send().await.unwrap();
        assert_eq!(del.status(), StatusCode::NO_CONTENT);

        // 3. GET after DELETE completes triggers refetch
        let r2 = client.get(&item_url).send().await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);
        assert_eq!(origin_count.load(Ordering::SeqCst), 2);

        node1.stop_tcp_service();
        router.stop();
    })
    .await
    .expect("PC-5 test timed out after 10s");
}

// ---------------------------------------------------------------------------
// KG-6: Node GetRequest fabricates val_node_<id> on missing key (Pin Test)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pin_kg6_node_get_request_fabricates_missing_key_body() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let node = Arc::new(VirtualNode::new(42));
        let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

        // Send a GetRequest frame over TCP for a missing key
        let req_frame = PhantomMeshFrame::new_get_request(1, 0, "non_existent_key_12345").unwrap();
        write_frame(&mut stream, &req_frame).await.unwrap();

        // Read response frame
        let resp_frame = read_frame(&mut stream).await.unwrap();
        let body_str = String::from_utf8_lossy(&resp_frame.payload);

        // Document current behavior: node fabricates MISS val_node_42
        assert!(
            body_str.starts_with("MISS val_node_42"),
            "Current behavior pinned: expected 'MISS val_node_42...', got '{}'",
            body_str
        );

        node.stop_tcp_service();
    })
    .await
    .expect("KG-6 pin test timed out after 10s");
}

// ---------------------------------------------------------------------------
// TG-1: DELETE recorded as zero-latency miss in telemetry (Pin Test)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pin_tg1_delete_records_as_zero_latency_miss_in_telemetry() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot(&[])),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let item_url = format!("http://127.0.0.1:{}/api/v1/cache/tg1_item", gw_addr.port());

        assert_eq!(telemetry.len(), 0);

        // GET -> records hit or miss
        let _ = client.get(&item_url).send().await.unwrap();
        assert_eq!(telemetry.len(), 1);

        // DELETE -> records ObservationKind::Delete (corrected S7 behavior)
        let del = client.delete(&item_url).send().await.unwrap();
        assert_eq!(del.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            telemetry.len(),
            2,
            "DELETE increments telemetry count by 1"
        );
        let obs = telemetry.observations();
        assert_eq!(obs[1].kind, ObservationKind::Delete);
    })
    .await
    .expect("TG-1 pin test timed out after 10s");
}
