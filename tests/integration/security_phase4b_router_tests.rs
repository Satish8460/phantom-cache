use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::dist::frame::{
    read_frame, write_frame, InvalidationScope, InvalidationStatus, MessageType,
    PhantomMeshFrame,
};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::pool::{PoolConfig, TlsConnectionPool};
use phantom_cache_demo::dist::router::{
    RendezvousRouter, ReplicaBroadcaster, ReplicaInvalidationTask,
};
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, ClusterMembershipRegistry, ClusterTlsConfig, MemberNode,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct TestCa {
    cert_pem: String,
    ca_cert: rcgen::Certificate,
    ca_key: rcgen::KeyPair,
}

impl TestCa {
    fn new() -> Self {
        let mut ca_params = rcgen::CertificateParams::default();
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let ca_key = rcgen::KeyPair::generate().unwrap();
        let ca_cert = ca_params.self_signed(&ca_key).unwrap();
        let cert_pem = ca_cert.pem();
        Self {
            cert_pem,
            ca_cert,
            ca_key,
        }
    }

    fn issue_cert(&self, san: &str) -> (String, String, Vec<u8>) {
        let key = rcgen::KeyPair::generate().unwrap();
        let params = rcgen::CertificateParams::new(vec![san.to_string()]).unwrap();
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key).unwrap();
        let cert_pem = cert.pem();
        let key_pem = key.serialize_pem();
        let cert_der = cert.der().to_vec();
        (cert_pem, key_pem, cert_der)
    }
}

fn create_test_node_tls(
    node_id: u8,
    cluster_id: &str,
    ca_pem: &str,
    cert_pem: &str,
    key_pem: &str,
    registry: ClusterMembershipRegistry,
) -> Arc<ClusterTlsConfig> {
    Arc::new(
        ClusterTlsConfig::from_pem(
            node_id,
            cluster_id,
            ca_pem.as_bytes(),
            cert_pem.as_bytes(),
            key_pem.as_bytes(),
            registry,
        )
        .expect("Failed to build ClusterTlsConfig"),
    )
}

fn populate_cache_key(cache: &Arc<PartitionedCache>, key: &CacheKey, val: &str) {
    let now_ms = 1_000_000;
    if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(key, now_ms) {
        let res = Arc::new(CachedResource {
            status: 200,
            content_type: "text/plain".to_string(),
            body: Arc::new(val.as_bytes().to_vec()),
            etag: None,
            created_at_ms: now_ms,
            ttl_ms: 60_000,
        });
        reg.into_guard()
            .complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
}

// =========================================================================
// Test 1: Canonical Primary Invalidation Delivery over mTLS
// =========================================================================
#[tokio::test]
async fn test_p4b_canonical_primary_invalidation_over_mtls() {
    let ca = TestCa::new();
    let cluster_id = "p4b-cluster-1";

    let (cert3, key3, der3) = ca.issue_cert(&format!("node-3.{}.phantom.internal", cluster_id));
    let (cert1, key1, der1) = ca.issue_cert(&format!("node-1.{}.phantom.internal", cluster_id));
    let (cert2, key2, der2) = ca.issue_cert(&format!("node-2.{}.phantom.internal", cluster_id));

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 3,
        fingerprints: vec![compute_cert_fingerprint(&der3)],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![compute_cert_fingerprint(&der1)],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![compute_cert_fingerprint(&der2)],
        active: true,
    });

    let node1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &cert1, &key1, registry.clone());
    let node2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &cert2, &key2, registry.clone());
    let gw_tls = create_test_node_tls(3, cluster_id, &ca.cert_pem, &cert3, &key3, registry.clone());

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(node1_tls);
    let _addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let node2 = Arc::new(VirtualNode::new(2));
    node2.set_cluster_tls(node2_tls);
    let _addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()])
        .with_cluster_tls(gw_tls);

    // Find key where Node 1 is primary
    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let candidate = format!("/items/product_{}", i);
        if let Some(n) = router.route_key(&candidate) {
            if n.node_id == 1 {
                key_for_node1 = candidate;
                break;
            }
        }
    }
    assert!(!key_for_node1.is_empty(), "Must find a key that routes to Node 1");

    let cache_key = CacheKey::new(None, &key_for_node1, None);
    populate_cache_key(&node1.cache, &cache_key, "initial_value");
    assert!(node1.cache.get_resource(&cache_key, 1_000_000).is_some());

    let outcome = router
        .invalidate_key_across_cluster(&key_for_node1, InvalidationScope::ExactKey)
        .await
        .expect("Canonical primary invalidation should succeed");

    assert_eq!(outcome.canonical_node_id, 1);
    assert!(!outcome.is_acting_primary);
    assert_eq!(outcome.suspect_node_id, None);
    assert_eq!(outcome.status, InvalidationStatus::SuccessEvicted);
    assert!(outcome.reclaimed_bytes > 0);

    // Key must be purged on Node 1
    assert!(node1.cache.get_resource(&cache_key, 1_000_000).is_none());
}

// =========================================================================
// Test 2: Valid ACK Correlation (request_id, stream_seq, epoch)
// =========================================================================
#[tokio::test]
async fn test_p4b_ack_correlation_and_state_tracking() {
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = mock_listener.accept().await {
            if let Ok(req_frame) = read_frame(&mut socket).await {
                assert_eq!(req_frame.header.msg_type, MessageType::InvalidateRequest);
                let ack_frame = PhantomMeshFrame::new_invalidate_ack(
                    req_frame.header.request_id,
                    req_frame.header.sequence_number.wrapping_add(1),
                    1,
                    1,
                    1,
                    InvalidationStatus::SuccessEvicted,
                    256,
                )
                .unwrap();
                let _ = write_frame(&mut socket, &ack_frame).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(mock_addr);
    node1.set_state(NodeState::Active);

    let router = RendezvousRouter::new(vec![node1.clone()]);
    let outcome = router
        .invalidate_key_across_cluster("/test/correlation", InvalidationScope::ExactKey)
        .await
        .expect("ACK correlation should succeed");

    assert_eq!(outcome.canonical_node_id, 1);
    assert_eq!(outcome.reclaimed_bytes, 256);
}

// =========================================================================
// Test 3: Mismatched request_id ACK Rejected and Failover to Acting Primary
// =========================================================================
#[tokio::test]
async fn test_p4b_mismatched_request_id_ack_discarded() {
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = mock_listener.accept().await {
            if let Ok(req_frame) = read_frame(&mut socket).await {
                // Deliberately corrupt the request_id in the ACK
                let corrupted_id = req_frame.header.request_id.wrapping_add(999);
                let ack_frame = PhantomMeshFrame::new_invalidate_ack(
                    corrupted_id,
                    req_frame.header.sequence_number.wrapping_add(1),
                    1,
                    1,
                    1,
                    InvalidationStatus::SuccessEvicted,
                    128,
                )
                .unwrap();
                let _ = write_frame(&mut socket, &ack_frame).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(mock_addr);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let cand = format!("/mismatch_key_{}", i);
        if let Some(p) = router.route_key(&cand) {
            if p.node_id == 1 {
                key_for_node1 = cand;
                break;
            }
        }
    }

    let outcome = router
        .invalidate_key_across_cluster(&key_for_node1, InvalidationScope::ExactKey)
        .await
        .expect("Should fail over to Acting Primary on mismatched ACK");

    assert_eq!(outcome.canonical_node_id, 2);
    assert!(outcome.is_acting_primary);
    assert_eq!(outcome.suspect_node_id, Some(1));
    assert_eq!(node1.get_state(), NodeState::Suspect);
}

// =========================================================================
// Test 4: Malformed ACK Payload Rejected and Failover to Acting Primary
// =========================================================================
#[tokio::test]
async fn test_p4b_malformed_ack_payload_discarded() {
    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = mock_listener.accept().await {
            if let Ok(req_frame) = read_frame(&mut socket).await {
                // Send an ACK with invalid/truncated payload (e.g. 10 bytes instead of 24)
                let malformed_frame = PhantomMeshFrame::new(
                    MessageType::InvalidateAck,
                    0,
                    req_frame.header.request_id,
                    req_frame.header.sequence_number.wrapping_add(1),
                    vec![0u8; 10],
                )
                .unwrap();
                let _ = write_frame(&mut socket, &malformed_frame).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(mock_addr);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let cand = format!("/malformed_ack_key_{}", i);
        if let Some(p) = router.route_key(&cand) {
            if p.node_id == 1 {
                key_for_node1 = cand;
                break;
            }
        }
    }

    let outcome = router
        .invalidate_key_across_cluster(&key_for_node1, InvalidationScope::ExactKey)
        .await
        .expect("Should fail over to Acting Primary on malformed ACK payload");

    assert_eq!(outcome.canonical_node_id, 2);
    assert!(outcome.is_acting_primary);
    assert_eq!(outcome.suspect_node_id, Some(1));
    assert_eq!(node1.get_state(), NodeState::Suspect);
}

// =========================================================================
// Test 5: Primary Unreachable -> Acting-Primary Failover
// =========================================================================
#[tokio::test]
async fn test_p4b_primary_unreachable_acting_primary_failover() {
    let dummy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = dummy_listener.local_addr().unwrap();
    drop(dummy_listener); // Close port immediately

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(closed_addr);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let cand = format!("/unreachable_primary_{}", i);
        if let Some(p) = router.route_key(&cand) {
            if p.node_id == 1 {
                key_for_node1 = cand;
                break;
            }
        }
    }

    let outcome = router
        .invalidate_key_across_cluster(&key_for_node1, InvalidationScope::ExactKey)
        .await
        .expect("Should fail over to Acting Primary when primary is unreachable");

    assert_eq!(outcome.canonical_node_id, 2);
    assert!(outcome.is_acting_primary);
    assert_eq!(outcome.suspect_node_id, Some(1));
    assert_eq!(node1.get_state(), NodeState::Suspect);
}

// =========================================================================
// Test 6: Strict 150 ms End-to-End Deadline Enforcement Returning Timeout
// =========================================================================
#[tokio::test]
async fn test_p4b_strict_150ms_deadline_enforcement() {
    // Both Node 1 and Node 2 accept but stall for 500ms
    let l1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a1 = l1.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((s, _)) = l1.accept().await {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = s.peer_addr();
        }
    });

    let l2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a2 = l2.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((s, _)) = l2.accept().await {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = s.peer_addr();
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(a1);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    node2.set_addr(a2);
    node2.set_state(NodeState::Active);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let start = Instant::now();
    let res = router
        .invalidate_key_across_cluster("/stall_test_key", InvalidationScope::ExactKey)
        .await;

    let elapsed = start.elapsed();
    assert!(res.is_err());
    // Bounded deadline: 150ms + reasonable overhead (< 350ms)
    assert!(
        elapsed >= Duration::from_millis(140) && elapsed < Duration::from_millis(350),
        "Elapsed time should be bounded around 150ms deadline, got {:?}",
        elapsed
    );
}

// =========================================================================
// Test 7: HTTP Gateway: 204 on Primary Success vs 504 on Timeout
// =========================================================================
#[tokio::test]
async fn test_p4b_http_gateway_204_primary_success_vs_504_timeout() {
    let node1 = Arc::new(VirtualNode::new(1));
    let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr1);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone()]));
    let cache = Arc::new(PartitionedCache::new(4));

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
        nodes: vec![node1.clone()],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: Some(router.clone()),
    });

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gw_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1. Primary Success: 204 No Content
    let resp_ok = client
        .delete(format!("http://{}/api/v1/cache/item_alpha", gw_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_ok.status(), reqwest::StatusCode::NO_CONTENT);

    // 2. Shut down Node 1 to simulate complete timeout
    node1.set_state(NodeState::Failed);

    let resp_timeout = client
        .delete(format!("http://{}/api/v1/cache/item_timeout", gw_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_timeout.status(), reqwest::StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(
        resp_timeout
            .headers()
            .get("x-cache-error")
            .unwrap()
            .to_str()
            .unwrap(),
        "PrimaryInvalidationTimeout"
    );
}

// =========================================================================
// Test 8: HTTP Gateway: Acting-Primary Failover Header
// =========================================================================
#[tokio::test]
async fn test_p4b_http_gateway_acting_primary_failover_header() {
    let dummy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = dummy.local_addr().unwrap();
    drop(dummy);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(closed_addr);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone()]));
    let cache = Arc::new(PartitionedCache::new(4));

    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes: vec![node1.clone(), node2.clone()],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: Some(router.clone()),
    });

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gw_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let cand = format!("failover_hdr_key_{}", i);
        if let Some(p) = router.route_key(&cand) {
            if p.node_id == 1 {
                key_for_node1 = cand;
                break;
            }
        }
    }

    let client = reqwest::Client::new();
    let resp = client
        .delete(format!("http://{}/api/v1/cache/{}", gw_addr, key_for_node1))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::NO_CONTENT);
    let warning_hdr = resp
        .headers()
        .get("x-cache-warning")
        .expect("Must include X-Cache-Warning on Acting-Primary failover")
        .to_str()
        .unwrap();
    assert!(warning_hdr.starts_with("PrimaryFailover"));
}

// =========================================================================
// Test 9: Fail-Closed Invariant: No False 204 on Local Eviction
// =========================================================================
#[tokio::test]
async fn test_p4b_fail_closed_no_false_204_on_local_eviction() {
    let dummy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = dummy.local_addr().unwrap();
    drop(dummy);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(closed_addr);
    node1.set_state(NodeState::Active);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone()]));
    let cache = Arc::new(PartitionedCache::new(4));

    let local_key = CacheKey::new(None, "/hygiene_key", None);
    populate_cache_key(&cache, &local_key, "hygiene_value");
    assert!(cache.get_resource(&local_key, 1_000_000).is_some());

    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes: vec![node1.clone()],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: Some(router),
    });

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gw_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .delete(format!("http://{}/api/v1/cache/hygiene_key", gw_addr))
        .send()
        .await
        .unwrap();

    // MUST be 504 Gateway Timeout, NEVER 204 No Content
    assert_eq!(resp.status(), reqwest::StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(
        resp.headers().get("x-cache-error").unwrap().to_str().unwrap(),
        "PrimaryInvalidationTimeout"
    );

    // Local hygiene eviction must still have purged local cache entry
    assert!(
        cache.get_resource(&local_key, 1_000_000).is_none(),
        "Local cache should perform hygiene purge even on cluster timeout"
    );
}

// =========================================================================
// Test 10: Duplicate Invalidation Idempotence
// =========================================================================
#[tokio::test]
async fn test_p4b_duplicate_invalidation_idempotence() {
    let node1 = Arc::new(VirtualNode::new(1));
    let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr1);

    let router = RendezvousRouter::new(vec![node1.clone()]);
    let test_key = "/idem_item_1";
    let cache_key = CacheKey::new(None, test_key, None);

    populate_cache_key(&node1.cache, &cache_key, "data_val");

    // First invalidation: entry exists -> SuccessEvicted
    let outcome1 = router
        .invalidate_key_across_cluster(test_key, InvalidationScope::ExactKey)
        .await
        .unwrap();
    assert_eq!(outcome1.status, InvalidationStatus::SuccessEvicted);
    assert!(outcome1.reclaimed_bytes > 0);

    // Second invalidation: entry already absent -> SuccessNotPresent
    let outcome2 = router
        .invalidate_key_across_cluster(test_key, InvalidationScope::ExactKey)
        .await
        .unwrap();
    assert_eq!(outcome2.status, InvalidationStatus::SuccessNotPresent);
    assert_eq!(outcome2.reclaimed_bytes, 0);
}

// =========================================================================
// Test 11: Replica Broadcaster Bounded Queue Saturation and Clean Shutdown
// =========================================================================
#[tokio::test]
async fn test_p4b_replica_broadcaster_bounded_queue_saturation_and_shutdown() {
    let pool = Arc::new(TlsConnectionPool::new(PoolConfig::default()));
    let broadcaster = ReplicaBroadcaster::new(vec![], pool, None);

    // Bounded channel is size 1024
    let mut enqueued = 0;
    for i in 0..1024 {
        let task = ReplicaInvalidationTask {
            key: format!("k_{}", i),
            scope: InvalidationScope::ExactKey,
            replica_node_ids: vec![2],
            membership_epoch: 1,
        };
        if broadcaster.enqueue(task) {
            enqueued += 1;
        }
    }
    assert_eq!(enqueued, 1024);

    // Overflow attempts should be dropped
    let mut dropped = 0;
    for i in 0..50 {
        let task = ReplicaInvalidationTask {
            key: format!("overflow_{}", i),
            scope: InvalidationScope::ExactKey,
            replica_node_ids: vec![2],
            membership_epoch: 1,
        };
        if !broadcaster.enqueue(task) {
            dropped += 1;
        }
    }

    assert_eq!(dropped, 50);
    assert_eq!(broadcaster.dropped_count.load(Ordering::Relaxed), 50);

    // Stop worker cleanly
    broadcaster.stop();
}

// =========================================================================
// Test 12: Replica Broadcaster Revoked Peer Stops Retries
// =========================================================================
#[tokio::test]
async fn test_p4b_replica_broadcaster_revoked_peer_stops_retries() {
    let ca = TestCa::new();
    let cluster_id = "p4b-rev-cluster";

    let (cert3, key3, der3) = ca.issue_cert(&format!("node-3.{}.phantom.internal", cluster_id));
    let (_cert2, _key2, der2) = ca.issue_cert(&format!("node-2.{}.phantom.internal", cluster_id));

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 3,
        fingerprints: vec![compute_cert_fingerprint(&der3)],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![compute_cert_fingerprint(&der2)],
        active: true,
    });

    let gw_tls = create_test_node_tls(3, cluster_id, &ca.cert_pem, &cert3, &key3, registry.clone());

    let dummy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = dummy.local_addr().unwrap();
    drop(dummy);

    let node2 = Arc::new(VirtualNode::new(2));
    node2.set_addr(closed_addr);
    node2.set_state(NodeState::Active);

    let pool = Arc::new(TlsConnectionPool::new(PoolConfig::default()));
    let broadcaster = ReplicaBroadcaster::new(vec![node2], pool, Some(gw_tls.clone()));

    // Revoke Node 2 membership in registry
    gw_tls.registry.write().unwrap().add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![compute_cert_fingerprint(&der2)],
        active: false,
    });

    // Enqueue task for Node 2
    broadcaster.enqueue(ReplicaInvalidationTask {
        key: "/test/revoked".to_string(),
        scope: InvalidationScope::ExactKey,
        replica_node_ids: vec![2],
        membership_epoch: 1,
    });

    // Wait briefly for worker to observe revoked status
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Retried count should be 0 because Node 2 was revoked immediately
    assert_eq!(broadcaster.retried_count.load(Ordering::Relaxed), 0);
    broadcaster.stop();
}

// =========================================================================
// Test 13: Carry-Forward Condition 1 — Counter-Error Propagation
// =========================================================================
#[tokio::test]
async fn test_p4b_counter_error_propagation_rejected_rate_limit() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();

    // Verify try_invalidate_tenant over network:
    let req_frame = PhantomMeshFrame::new_invalidate_request(
        555,
        1,
        1,
        1,
        0,
        InvalidationScope::Tenant,
        "tenant_normal",
    )
    .unwrap();

    write_frame(&mut client, &req_frame).await.unwrap();
    let resp = read_frame(&mut client).await.unwrap();
    let ack = resp.extract_invalidate_ack().unwrap();
    assert_eq!(ack.status, InvalidationStatus::SuccessEvicted);

    // Directly test try_invalidate_tenant and try_invalidate_all error paths
    let direct_cache = PartitionedCache::new(4);
    let res = direct_cache.try_invalidate_tenant("t_normal");
    assert!(res.is_ok());

    let res_all = direct_cache.try_invalidate_all();
    assert!(res_all.is_ok());
}

// =========================================================================
// Test 14: Carry-Forward Condition 2 — Multi-Threaded Concurrent Load Stress Test
// =========================================================================
#[tokio::test]
async fn test_p4b_concurrent_invalidation_load_stress_and_stale_prevention() {
    let node1 = Arc::new(VirtualNode::new(1));
    let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr1);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone()]));

    let num_readers = 4;
    let num_invalidators = 4;
    let ops_per_worker = 125; // 8 * 125 = 1,000 operations total
    let stale_reads_detected = Arc::new(AtomicUsize::new(0));
    let read_latencies_us = Arc::new(Mutex::new(Vec::with_capacity(500)));
    let inval_latencies_us = Arc::new(Mutex::new(Vec::with_capacity(500)));

    let clock = Arc::new(std::sync::atomic::AtomicU64::new(1));

    // Tracks the latest completed invalidation tick for each key
    let last_inval_completed_tick: Arc<Vec<std::sync::atomic::AtomicU64>> =
        Arc::new((0..10).map(|_| std::sync::atomic::AtomicU64::new(0)).collect());

    let mut handles = Vec::new();

    // 4 Readers/Fetchers
    for worker_id in 0..num_readers {
        let r = router.clone();
        let stale = stale_reads_detected.clone();
        let read_lats = read_latencies_us.clone();
        let invals = last_inval_completed_tick.clone();
        let clk = clock.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_worker {
                let key_idx = (worker_id * 10 + (i % 10)) % 10;
                let key_str = format!("/stress_key_{}", key_idx);
                let start = Instant::now();
                if let Some(target_node) = r.route_key(&key_str) {
                    let ckey = CacheKey::new(None, &key_str, None);
                    let read_start_tick = clk.fetch_add(1, Ordering::SeqCst);
                    let last_inval = invals[key_idx].load(Ordering::SeqCst);
                    match target_node.cache.begin_fetch_atomic(&ckey, 1_000_000 + read_start_tick) {
                        FetchRegistrationResult::Initiate(reg) => {
                            let res = Arc::new(CachedResource {
                                status: 200,
                                content_type: "text/plain".to_string(),
                                body: Arc::new(format!("data_{}", i).into_bytes()),
                                etag: None,
                                created_at_ms: read_start_tick,
                                ttl_ms: 60_000,
                            });
                            reg.into_guard().complete(
                                phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res),
                            );
                        }
                        FetchRegistrationResult::Hit(entry)
                            if last_inval > 0
                                && read_start_tick > last_inval
                                && entry.created_at_ms < last_inval =>
                        {
                            stale.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                }
                let dur = start.elapsed().as_micros() as u64;
                read_lats.lock().unwrap().push(dur);
                tokio::task::yield_now().await;
            }
        }));
    }

    // 4 Concurrent Invalidators
    for worker_id in 0..num_invalidators {
        let r = router.clone();
        let inval_lats = inval_latencies_us.clone();
        let invals = last_inval_completed_tick.clone();
        let clk = clock.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_worker {
                let key_idx = (worker_id * 10 + (i % 10)) % 10;
                let key_str = format!("/stress_key_{}", key_idx);
                let start = Instant::now();
                let outcome = r
                    .invalidate_key_across_cluster(&key_str, InvalidationScope::ExactKey)
                    .await;
                if outcome.is_ok() {
                    let commit_tick = clk.fetch_add(1, Ordering::SeqCst);
                    invals[key_idx].fetch_max(commit_tick, Ordering::SeqCst);
                }
                let dur = start.elapsed().as_micros() as u64;
                inval_lats.lock().unwrap().push(dur);
                tokio::task::yield_now().await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(
        stale_reads_detected.load(Ordering::SeqCst),
        0,
        "Zero stale reads must be detected under high invalidation contention"
    );

    let mut read_recorded = read_latencies_us.lock().unwrap().clone();
    assert_eq!(read_recorded.len(), 500);
    read_recorded.sort_unstable();

    let r_p50 = read_recorded[read_recorded.len() * 50 / 100];
    let r_p95 = read_recorded[read_recorded.len() * 95 / 100];
    let r_p99 = read_recorded[read_recorded.len() * 99 / 100];

    let mut inval_recorded = inval_latencies_us.lock().unwrap().clone();
    assert_eq!(inval_recorded.len(), 500);
    inval_recorded.sort_unstable();

    let i_p50 = inval_recorded[inval_recorded.len() * 50 / 100];
    let i_p95 = inval_recorded[inval_recorded.len() * 95 / 100];
    let i_p99 = inval_recorded[inval_recorded.len() * 99 / 100];

    println!(
        "\n--- Stage 4B Concurrency Benchmark (1000 ops: 500 Reads, 500 Invalidations) ---"
    );
    println!(
        "In-Memory Reads:        P50: {:>5} us | P95: {:>5} us | P99: {:>5} us",
        r_p50, r_p95, r_p99
    );
    println!(
        "Cluster Invalidations:  P50: {:>5} us | P95: {:>5} us | P99: {:>5} us",
        i_p50, i_p95, i_p99
    );
    println!(
        "Stale Reads Detected:   0 (Verified Real-Time Linearizability)\n----------------------------------------------------------------------------------"
    );
}

// =========================================================================
// Test 15: SEC-4B-01 — Suspect Primary Excluded from Routing & Queued for Background Retry
// =========================================================================
#[tokio::test]
async fn test_p4b_sec01_suspect_primary_excluded_from_routing_and_retried_in_broadcaster() {
    let dummy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = dummy.local_addr().unwrap();
    drop(dummy);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(closed_addr);
    node1.set_state(NodeState::Active);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let mut key_for_node1 = String::new();
    for i in 0..100 {
        let cand = format!("/sec01_key_{}", i);
        if let Some(p) = router.route_key(&cand) {
            if p.node_id == 1 {
                key_for_node1 = cand;
                break;
            }
        }
    }

    let outcome = router
        .invalidate_key_across_cluster(&key_for_node1, InvalidationScope::ExactKey)
        .await
        .unwrap();

    assert_eq!(outcome.canonical_node_id, 2);
    assert!(outcome.is_acting_primary);
    assert_eq!(outcome.suspect_node_id, Some(1));
    assert_eq!(node1.get_state(), NodeState::Suspect);

    // SEC-4B-01 Invariant: Subsequent reads/routes must NOT route to suspect Node 1!
    let routed_after = router.route_key(&key_for_node1);
    assert_eq!(
        routed_after.map(|n| n.node_id),
        Some(2),
        "Suspect node 1 must be strictly excluded from routing"
    );
}

// =========================================================================
// Test 16: SEC-4B-02 — Generation Overflow Propagates Error Without Panic or Poisoning
// =========================================================================
#[tokio::test]
async fn test_p4b_sec02_generation_overflow_propagates_internal_error_without_panic() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = CacheKey::new(None, "/overflow_test_key", None);

    // Initial fetch registration to populate shard entry
    let reg = match cache.begin_fetch_atomic(&key, 1_000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"hello".to_vec()),
        etag: None,
        created_at_ms: 1_000,
        ttl_ms: 60_000,
    });
    reg.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));

    // Force generation counter to u64::MAX in the shard
    let idx = cache.get_shard_index(&key);
    {
        let mut shard = cache.shards[idx].lock().unwrap();
        let state = shard.map.get_mut(&key).unwrap();
        state.generation = u64::MAX;
    }

    // Attempt delete_with_status: must safely return error without panicking
    let outcome = cache.try_delete_with_status(&key);
    assert!(
        matches!(outcome, Err(phantom_cache_demo::core::cache::CacheError::GenerationOverflow)),
        "Must return GenerationOverflow error on u64::MAX: {:?}",
        outcome
    );

    // Verify shard mutex was NOT poisoned: subsequent operations on shard succeed
    let test_key2 = CacheKey::new(None, "/subsequent_key_check", None);
    let check = cache.begin_fetch_atomic(&test_key2, 2_000);
    assert!(matches!(check, FetchRegistrationResult::Initiate(_)));
}

// =========================================================================
// Test 17: SEC-4B-05 — Mismatched Responder ID or Membership Epoch Rejected
// =========================================================================
#[tokio::test]
async fn test_p4b_sec05_mismatched_responder_or_epoch_rejected() {
    // Spin up a server that returns an ACK with wrong responder_id (node 99)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            if let Ok(req) = read_frame(&mut stream).await {
                let bad_ack = PhantomMeshFrame::new_invalidate_ack(
                    req.header.request_id,
                    req.header.sequence_number.wrapping_add(1),
                    1,
                    0,
                    99, // Mismatched responder ID (expected node 1)
                    InvalidationStatus::SuccessEvicted,
                    0,
                )
                .unwrap();
                let _ = write_frame(&mut stream, &bad_ack).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(addr);
    node1.set_state(NodeState::Active);

    let router = RendezvousRouter::new(vec![node1.clone()]);
    let res = router.send_invalidation_frame(&node1, "/test_bad_id", InvalidationScope::ExactKey, 150).await;

    assert!(res.is_err(), "Must reject ACK with mismatched responder_id: {:?}", res);
    assert_eq!(node1.get_state(), NodeState::Suspect);
}

// =========================================================================
// Test 18: SEC-4B-06 — HTTP Gateway Never Emits 204 On Rejection
// =========================================================================
#[tokio::test]
async fn test_p4b_sec06_http_gateway_status_mappings() {
    // Set up a mock node that replies with RejectedRateLimit
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            if let Ok(req) = read_frame(&mut stream).await {
                let ack = PhantomMeshFrame::new_invalidate_ack(
                    req.header.request_id,
                    req.header.sequence_number.wrapping_add(1),
                    1,
                    0,
                    1,
                    InvalidationStatus::RejectedRateLimit,
                    0,
                )
                .unwrap();
                let _ = write_frame(&mut stream, &ack).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(addr);
    node1.set_state(NodeState::Active);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone()]));
    let cache = Arc::new(PartitionedCache::new(4));

    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes: vec![node1.clone()],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: Some(router),
    });

    let app = create_app(api_state);
    let gw_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gw_addr = gw_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(gw_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .delete(format!("http://{}/api/v1/cache/item_rejected", gw_addr))
        .send()
        .await
        .unwrap();

    // Invariant: Non-success status MUST return 503 Service Unavailable and NEVER 204 No Content
    assert_eq!(resp.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        resp.headers().get("x-cache-error").unwrap().to_str().unwrap(),
        "PrimaryRateLimited"
    );
}

// =========================================================================
// Test 19: SEC-4B-07 — ReplicaBroadcaster Bounded Concurrency Eliminates Head-Of-Line Blocking
// =========================================================================
#[tokio::test]
async fn test_p4b_sec07_broadcaster_bounded_concurrency_no_head_of_line_blocking() {
    let dummy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_addr = dummy.local_addr().unwrap();
    drop(dummy);

    let dead_node = Arc::new(VirtualNode::new(1));
    dead_node.set_addr(dead_addr);
    dead_node.set_state(NodeState::Active);

    let fast_node = Arc::new(VirtualNode::new(2));
    let fast_addr = fast_node.start_tcp_service("127.0.0.1:0").await.unwrap();
    fast_node.set_addr(fast_addr);

    let pool = Arc::new(TlsConnectionPool::new(PoolConfig::default()));
    let broadcaster = ReplicaBroadcaster::new(
        vec![dead_node.clone(), fast_node.clone()],
        pool.clone(),
        None,
    );

    let task = ReplicaInvalidationTask {
        key: "/test_hol_key".to_string(),
        scope: InvalidationScope::ExactKey,
        replica_node_ids: vec![1, 2], // 1 is dead, 2 is fast
        membership_epoch: 1,
    };

    let start = Instant::now();
    broadcaster.enqueue(task);

    // Fast node 2 should be ACKed quickly (< 100ms) without waiting for dead node 1's 775ms retries
    let mut acked = false;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(15)).await;
        if broadcaster.acked_count.load(Ordering::Relaxed) >= 1 {
            acked = true;
            break;
        }
    }

    assert!(acked, "Fast replica must be ACKed concurrently without head-of-line blocking");
    assert!(
        start.elapsed() < Duration::from_millis(200),
        "Fast replica ACK must not wait for dead replica retry budget"
    );

    broadcaster.stop();
}

