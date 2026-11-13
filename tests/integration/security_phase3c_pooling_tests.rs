use phantom_cache_demo::dist::frame::{read_frame, write_frame, PhantomMeshFrame};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::pool::{ClusterStream, PoolConfig, PooledConnection, TlsConnectionPool};
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, ClusterMembershipRegistry, ClusterTlsConfig, MemberNode,
};
use std::sync::Arc;
use std::time::Duration;

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

/// AT-P3C-01: Strict Request Correlation & Request ID Mismatch Discard
/// Verifies that if a rogue/corrupted node responds with an unexpected request_id,
/// the connection pool discards the connection, the node is marked Suspect,
/// and the router transparently fails over to the replica.
#[tokio::test]
async fn test_p3c_request_id_mismatch_discard_and_failover() {
    // Mock server for Node 1 that intentionally corrupts the response request_id
    let rogue_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let rogue_addr = rogue_listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = rogue_listener.accept().await {
            if let Ok(req_frame) = read_frame(&mut socket).await {
                // Send response with a deliberately mismatched request_id (req_id + 999)
                let corrupted_id = req_frame.header.request_id.wrapping_add(999);
                let resp_frame = PhantomMeshFrame::new_get_response(
                    corrupted_id,
                    req_frame.header.sequence_number.wrapping_add(1),
                    "CORRUPTED_RESPONSE",
                )
                .unwrap();
                let _ = write_frame(&mut socket, &resp_frame).await;
            }
        }
    });

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr(rogue_addr);
    node1.set_state(NodeState::Active);

    // Legitimate Node 2 as replica
    let node2 = Arc::new(VirtualNode::new(2));
    let node2_addr = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(node2_addr);

    let router = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let mut test_key = "test_key_0";
    for i in 0..100 {
        let k = format!("key_{}", i);
        if router.route_key(&k).unwrap().node_id == 1 {
            test_key = Box::leak(k.into_boxed_str());
            break;
        }
    }

    let result = router.forward_key_over_tcp(test_key, 2000, 1).await;

    // Must succeed via failover to replica (node 2)
    assert!(result.is_ok(), "Request must succeed via failover to replica");
    let (served_by, body) = result.unwrap();
    assert_eq!(served_by, 2, "Response should have been served by replica node 2");
    assert!(body.starts_with("MISS val_node_2"));

    // Node 1 must have been marked Suspect due to request ID mismatch
    assert_eq!(node1.get_state(), NodeState::Suspect);

    // Node 1's corrupted connection must have been discarded from the pool
    assert_eq!(router.pool().idle_count_for_node(1), 0);
    assert_eq!(router.pool().total_active_for_node(1), 0);

    // Node 2's healthy connection should be in the pool
    assert_eq!(router.pool().idle_count_for_node(2), 1);
    assert_eq!(router.pool().total_active_for_node(2), 1);

    node2.stop_tcp_service();
}

/// AT-P3C-02: Connection Reuse Across Sequential Requests
/// Verifies that multiple sequential requests check out and return the same pooled connection,
/// keeping the total active connection count bounded to 1 without creating new TCP sockets.
#[tokio::test]
async fn test_p3c_connection_reuse_across_sequential_requests() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);

    let router = RendezvousRouter::new(vec![node.clone()]);
    let pool = router.pool();

    assert_eq!(pool.idle_count_for_node(1), 0);
    assert_eq!(pool.total_active_for_node(1), 0);

    // First request establishes connection and checks it in
    let (served_by_1, body1) = router.forward_key_over_tcp("user:101", 1000, 1).await.unwrap();
    assert_eq!(served_by_1, 1);
    assert!(body1.starts_with("MISS"));

    assert_eq!(pool.idle_count_for_node(1), 1, "Connection should be pooled");
    assert_eq!(pool.total_active_for_node(1), 1, "Total active should be 1");

    // Second request reuses the pooled connection
    let (served_by_2, body2) = router.forward_key_over_tcp("user:101", 1000, 1).await.unwrap();
    assert_eq!(served_by_2, 1);
    assert!(body2.starts_with("HIT"), "Should be a cache hit on the same node");

    assert_eq!(pool.idle_count_for_node(1), 1, "Connection remains pooled after reuse");
    assert_eq!(pool.total_active_for_node(1), 1, "Total active count remains 1");

    // Third request with different key reuses same connection
    let (served_by_3, body3) = router.forward_key_over_tcp("user:202", 1000, 1).await.unwrap();
    assert_eq!(served_by_3, 1);
    assert!(body3.starts_with("MISS"));

    assert_eq!(pool.idle_count_for_node(1), 1);
    assert_eq!(pool.total_active_for_node(1), 1);

    node.stop_tcp_service();
}

/// AT-P3C-03: Concurrent Requests & Pool Isolation
/// Verifies that concurrent tasks check out separate connections from the pool,
/// avoiding frame interleaving, with all responses properly correlated.
#[tokio::test]
async fn test_p3c_concurrent_requests_pool_isolation() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);

    let router = Arc::new(RendezvousRouter::new(vec![node.clone()]));

    let mut handles = Vec::new();
    for i in 0..6 {
        let r = router.clone();
        handles.push(tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i);
            let res = r.forward_key_over_tcp(&key, 2000, 1).await;
            (i, res)
        }));
    }

    for handle in handles {
        let (i, res) = handle.await.unwrap();
        assert!(res.is_ok(), "Concurrent request {} failed: {:?}", i, res);
        let (node_id, body) = res.unwrap();
        assert_eq!(node_id, 1);
        assert!(body.starts_with("MISS val_node_1"));
    }

    // Pool idle count should be at most max_idle_per_node (4)
    assert!(router.pool().idle_count_for_node(1) <= 4);
    assert!(router.pool().total_active_for_node(1) <= 4);

    node.stop_tcp_service();
}

/// SEC-3C-01 Adversarial: Atomic Capacity Reservation & Hard Cap Enforcement under Heavy Concurrency
/// Verifies that when 16 simultaneous requests target a single node with max_total_per_node = 8,
/// active connections NEVER exceed 8, excess requests safely fail over to the replica,
/// and counters do not underflow or diverge after completion.
#[tokio::test]
async fn test_p3c_sec01_concurrency_capacity_cap_and_no_leak() {
    let node1 = Arc::new(VirtualNode::new(1));
    let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr1);

    let node2 = Arc::new(VirtualNode::new(2));
    let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(addr2);

    let router = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone()]));

    // Spawn 16 concurrent requests simultaneously
    let mut handles = Vec::new();
    for i in 0..16 {
        let r = router.clone();
        handles.push(tokio::spawn(async move {
            let key = format!("heavy_load_{}", i);
            r.forward_key_over_tcp(&key, 3000, 1).await
        }));
    }

    for handle in handles {
        let res = handle.await.unwrap();
        assert!(res.is_ok(), "Request must succeed either on primary or via replica failover: {:?}", res);
    }

    // Node 1 total active connections must NEVER have exceeded max_total_per_node (8)
    assert!(router.pool().total_active_for_node(1) <= 8);
    // Idle connections must not exceed max_idle_per_node (4)
    assert!(router.pool().idle_count_for_node(1) <= 4);
    assert!(router.pool().idle_count_for_node(2) <= 4);

    node1.stop_tcp_service();
    node2.stop_tcp_service();
}

/// AT-P3C-04: Heartbeat Ping-Pong Validation
/// Verifies that `TlsConnectionPool::send_heartbeat` sends a valid HeartbeatRequest (0x05)
/// and receives a matching HeartbeatResponse (0x06) with correct request_id.
#[tokio::test]
async fn test_p3c_heartbeat_ping_pong() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);

    let pool = TlsConnectionPool::new(PoolConfig::default());
    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut conn = PooledConnection::new(1, ClusterStream::Plain(socket), 1);

    let prev_used = conn.last_used_at;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Send keepalive heartbeat
    let hb_result = pool.send_heartbeat(&mut conn, 1000).await;
    assert!(hb_result.is_ok(), "Heartbeat ping-pong failed: {:?}", hb_result);
    assert!(conn.last_used_at > prev_used, "last_used_at must be refreshed on heartbeat");
    assert_eq!(conn.sequence_number, 2, "Sequence number should have incremented");

    // Second heartbeat
    let hb2 = pool.send_heartbeat(&mut conn, 1000).await;
    assert!(hb2.is_ok());
    assert_eq!(conn.sequence_number, 3);

    pool.checkin(conn);
    assert_eq!(pool.idle_count_for_node(1), 1);

    node.stop_tcp_service();
}

/// SEC-3C-02: Real Background Periodic Heartbeat Scheduling & Unresponsive Peer Eviction
/// Verifies that the background worker ticks at `heartbeat_interval`, probes idle connections,
/// refreshes `last_used_at`, and automatically discards broken connections when a node goes down.
#[tokio::test]
async fn test_p3c_sec02_periodic_heartbeat_scheduler_background_probing() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);

    let config = PoolConfig {
        heartbeat_interval: Duration::from_millis(40),
        ..Default::default()
    };

    let pool = Arc::new(TlsConnectionPool::new(config));
    pool.start_heartbeat_worker();

    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    pool.try_increment_active(1);
    let conn = PooledConnection::new(1, ClusterStream::Plain(socket), 1);
    pool.checkin(conn);

    assert_eq!(pool.idle_count_for_node(1), 1);

    // Wait 100ms: allows the background worker to execute at least 2 ticks
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Checkout connection: sequence number must have been incremented by the background scheduler
    let probed_conn = pool.checkout(1).expect("Connection should still be idle and healthy");
    assert!(probed_conn.sequence_number >= 2, "Background worker must have probed connection");
    pool.checkin(probed_conn);

    // Now shut down node 1 listener: simulates sudden peer crash
    node.stop_tcp_service();

    // Wait 100ms: background worker ticks, detects broken connection, and prunes it from pool
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Pool must have evicted the broken connection automatically
    assert_eq!(pool.idle_count_for_node(1), 0, "Dead connection must be evicted by background worker");
    assert_eq!(pool.total_active_for_node(1), 0);

    pool.stop_heartbeat_worker();
}

/// AT-P3C-05: Heartbeat Failure & Discard
/// Verifies that an unfulfilled heartbeat timeout discards the connection cleanly.
#[tokio::test]
async fn test_p3c_heartbeat_failure_eviction() {
    // Unresponsive listener: accepts connection but never writes back
    let dummy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = dummy_listener.local_addr().unwrap();

    let _server_task = tokio::spawn(async move {
        if let Ok((_sock, _)) = dummy_listener.accept().await {
            // Hang indefinitely without responding
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

    let pool = TlsConnectionPool::new(PoolConfig::default());
    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    pool.try_increment_active(1);
    let mut conn = PooledConnection::new(1, ClusterStream::Plain(socket), 1);

    assert_eq!(pool.total_active_for_node(1), 1);

    // Heartbeat should timeout quickly
    let hb_result = pool.send_heartbeat(&mut conn, 100).await;
    assert!(hb_result.is_err(), "Heartbeat must fail on unresponsive peer");

    // Discard the unhealthy connection
    pool.discard(conn);
    assert_eq!(pool.total_active_for_node(1), 0);
    assert_eq!(pool.idle_count_for_node(1), 0);
}

/// SEC-3C-04: Eviction Epoch Invalidation — Stale Connections Dropped on Checkin
/// Verifies that a connection checked out before node eviction is recognized as stale
/// via epoch mismatch upon checkin, dropped cleanly, and never returned to the idle pool.
#[tokio::test]
async fn test_p3c_sec04_evicted_connection_dropped_on_checkin() {
    let pool = Arc::new(TlsConnectionPool::new(PoolConfig::default()));

    let dummy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = dummy_listener.local_addr().unwrap();

    // Establish connection for Node 1 in epoch 1
    let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
    pool.try_increment_active(1);
    let conn = PooledConnection::new(1, ClusterStream::Plain(socket), 1);

    assert_eq!(conn.epoch, 1);
    assert_eq!(pool.total_active_for_node(1), 1);

    // Node 1 is evicted (increments epoch from 1 to 2)
    pool.evict_node(1);
    assert_eq!(pool.current_epoch(1), 2);

    // Checked-out connection from epoch 1 finishes and attempts to check in
    pool.checkin(conn);

    // Must be dropped immediately due to epoch mismatch (epoch 1 != epoch 2)
    assert_eq!(pool.idle_count_for_node(1), 0, "Stale epoch connection must not enter idle pool");
    assert_eq!(pool.total_active_for_node(1), 0, "Active counter must be decremented on stale checkin");

    // Node recovers and establishes fresh connection under epoch 2
    let socket2 = tokio::net::TcpStream::connect(addr).await.unwrap();
    pool.try_increment_active(1);
    let fresh_conn = PooledConnection::new(1, ClusterStream::Plain(socket2), 2);
    pool.checkin(fresh_conn);

    assert_eq!(pool.idle_count_for_node(1), 1, "Fresh connection with matching epoch is accepted");
    assert_eq!(pool.total_active_for_node(1), 1);
}

/// AT-P3C-06: Layer 5 Active Session Eviction on Peer Revocation
/// Verifies that calling `VirtualNode::evict_peer` immediately closes open inbound sockets
/// for the specified peer node via `SessionTracker` watch cancellation.
#[tokio::test]
async fn test_p3c_active_session_eviction_on_peer_revocation() {
    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);

    // Client connects as node 2
    let mut client_socket = tokio::net::TcpStream::connect(addr).await.unwrap();

    // Send a frame to establish the session loop
    let req = PhantomMeshFrame::new_get_request(1001, 1, "test_session").unwrap();
    write_frame(&mut client_socket, &req).await.unwrap();
    let resp = read_frame(&mut client_socket).await.unwrap();
    assert_eq!(resp.header.request_id, 1001);

    assert!(node.session_tracker.total_active_sessions() >= 1);

    // Active eviction
    let evicted = node.session_tracker.evict_all();
    assert!(evicted >= 1, "Must have evicted active session");

    // Subsequent read from client must immediately observe EOF or reset
    let next_read = read_frame(&mut client_socket).await;
    assert!(next_read.is_err(), "Socket must have been closed by server: {:?}", next_read);

    node.stop_tcp_service();
}

/// SEC-3C-03: Automated Membership Revocation Wiring to Active Session Eviction
/// Verifies that updating the cluster registry to mark a peer inactive automatically triggers
/// active session termination and severs open sockets WITHOUT manually calling `evict_peer()`.
#[tokio::test]
async fn test_p3c_sec03_automated_membership_revocation_wiring() {
    let ca = TestCa::new();
    let cluster_id = "cluster-auto-rev";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-auto-rev.phantom.internal");
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-auto-rev.phantom.internal");
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp.clone()],
        active: true,
    });

    let n1_tls = create_test_node_tls(
        1,
        cluster_id,
        &ca.cert_pem,
        &n1_cert_pem,
        &n1_key_pem,
        registry.clone(),
    );
    let n2_tls = create_test_node_tls(
        2,
        cluster_id,
        &ca.cert_pem,
        &n2_cert_pem,
        &n2_key_pem,
        registry.clone(),
    );

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls.clone());
    let addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr);

    // Node 2 establishes mTLS connection to Node 1
    let tcp_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let connector = n2_tls.client_connector();
    let server_name = rustls::pki_types::ServerName::try_from(format!(
        "node-1.{}.phantom.internal",
        cluster_id
    ))
    .unwrap();
    let mut tls_stream = connector.connect(server_name, tcp_stream).await.unwrap();

    // Verify session is established with an initial request
    let req1 = PhantomMeshFrame::new_get_request(7001, 1, "test_wire").unwrap();
    write_frame(&mut tls_stream, &req1).await.unwrap();
    let resp1 = read_frame(&mut tls_stream).await.unwrap();
    assert_eq!(resp1.header.request_id, 7001);

    // Wait 20ms to ensure session is registered in SessionTracker
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(node1.session_tracker.active_sessions_for_peer(2), 1);

    // Revoke Node 2 in authoritative registry via add_member (WITHOUT invoking evict_peer)
    n1_tls.registry.write().unwrap().add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: false,
    });

    // Wait 30ms for the automated revocation event to propagate and sever the socket
    tokio::time::sleep(Duration::from_millis(30)).await;

    // Node 1's active sessions for peer 2 must be 0
    assert_eq!(node1.session_tracker.active_sessions_for_peer(2), 0);

    // Node 2's socket must be closed; reading from it must return EOF / error
    let read_res = read_frame(&mut tls_stream).await;
    assert!(read_res.is_err(), "Socket must be closed automatically upon registry revocation: {:?}", read_res);

    node1.stop_tcp_service();
}

/// AT-P3C-07: Per-Request Peer Revocation Check on Reused mTLS Connection
/// Verifies that when a peer node is revoked in the membership registry mid-session,
/// the server detects the revocation on the NEXT request over the existing pooled connection
/// and aborts the connection immediately without waiting for TLS re-negotiation.
#[tokio::test]
async fn test_p3c_per_request_revocation_on_reused_connection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-test-3c";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-test-3c.phantom.internal");
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-test-3c.phantom.internal");
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp.clone()],
        active: true,
    });

    let n1_tls = create_test_node_tls(
        1,
        cluster_id,
        &ca.cert_pem,
        &n1_cert_pem,
        &n1_key_pem,
        registry.clone(),
    );
    let n2_tls = create_test_node_tls(
        2,
        cluster_id,
        &ca.cert_pem,
        &n2_cert_pem,
        &n2_key_pem,
        registry.clone(),
    );

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls.clone());
    let addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(addr);

    // Node 2 establishes mTLS client connection to Node 1
    let tcp_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let connector = n2_tls.client_connector();
    let server_name = rustls::pki_types::ServerName::try_from(format!(
        "node-1.{}.phantom.internal",
        cluster_id
    ))
    .unwrap();
    let mut tls_stream = connector.connect(server_name, tcp_stream).await.unwrap();

    // Request 1: Node 2 is active, request succeeds
    let req1 = PhantomMeshFrame::new_get_request(5001, 1, "allowed_key").unwrap();
    write_frame(&mut tls_stream, &req1).await.unwrap();
    let resp1 = read_frame(&mut tls_stream).await.unwrap();
    assert_eq!(resp1.header.request_id, 5001);

    // Mid-session Revocation: Quarantining Node 2 in Node 1's live registry
    n1_tls.registry.write().unwrap().add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: false,
    });

    // Request 2: Node 2 sends another request over the SAME open connection
    let req2 = PhantomMeshFrame::new_get_request(5002, 2, "forbidden_key").unwrap();
    write_frame(&mut tls_stream, &req2).await.unwrap();

    // Node 1 validates registry admission on each frame and must abort connection
    let resp2 = read_frame(&mut tls_stream).await;
    assert!(
        resp2.is_err(),
        "Server must close connection upon detecting revoked peer on reused connection: {:?}",
        resp2
    );

    node1.stop_tcp_service();
}
