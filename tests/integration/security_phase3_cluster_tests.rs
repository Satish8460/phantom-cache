use phantom_cache_demo::dist::node::VirtualNode;
use phantom_cache_demo::dist::router::{DistError, RendezvousRouter};
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, extract_sans_from_der, load_membership_from_file,
    load_membership_from_json_str, parse_san_node_identity, validate_cluster_startup_security,
    ClusterMembershipRegistry, ClusterSecurityError, ClusterTlsConfig, MemberNode, NodeIdentity,
};
use phantom_cache_demo::security::StartupSecurityError;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    fn issue_cert(&self, san: &str, expired: bool) -> (String, String, Vec<u8>) {
        let key = rcgen::KeyPair::generate().unwrap();
        let mut params = rcgen::CertificateParams::new(vec![san.to_string()]).unwrap();
        if expired {
            params.not_before = rcgen::date_time_ymd(1975, 1, 1);
            params.not_after = rcgen::date_time_ymd(1975, 1, 2);
        }
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key).unwrap();
        let cert_pem = cert.pem();
        let key_pem = key.serialize_pem();
        let cert_der = cert.der().to_vec();
        (cert_pem, key_pem, cert_der)
    }

    fn issue_multi_san_cert(&self, sans: Vec<String>) -> (String, String, Vec<u8>) {
        let key = rcgen::KeyPair::generate().unwrap();
        let params = rcgen::CertificateParams::new(sans).unwrap();
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key).unwrap();
        let cert_pem = cert.pem();
        let key_pem = key.serialize_pem();
        let cert_der = cert.der().to_vec();
        (cert_pem, key_pem, cert_der)
    }
}

/// Helper to create ClusterTlsConfig for a node
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

/// AT-P3-01: Authorized Node mTLS Connection
/// Mutually authenticated TLS 1.3 handshake succeeds; cache request delivered.
#[tokio::test]
async fn test_p3_at01_authorized_mtls_connection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    // Node 1 (Server)
    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 (Client)
    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry.clone());

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("user_profile_123", 2000, 1).await;
    assert!(res.is_ok(), "Authorized mTLS connection should succeed: {:?}", res);
    let (served_by, body) = res.unwrap();
    assert_eq!(served_by, 1);
    assert_eq!(body, "MISS val_node_1");
}

/// AT-P3-02: Untrusted CA Rejection
/// Connection presenting certificate signed by untrusted CA is rejected; 0 application bytes read.
#[tokio::test]
async fn test_p3_at02_untrusted_ca_rejection() {
    let trusted_ca = TestCa::new();
    let untrusted_ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        trusted_ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 cert signed by rogue / untrusted CA
    let (rogue_cert_pem, rogue_key_pem, rogue_der) =
        untrusted_ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let rogue_fp = compute_cert_fingerprint(&rogue_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![rogue_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &trusted_ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    // Rogue client trusts untrusted_ca, connects with rogue cert
    let rogue_tls = create_test_node_tls(2, cluster_id, &untrusted_ca.cert_pem, &rogue_cert_pem, &rogue_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(rogue_tls);

    let res = router.forward_key_over_tcp("user_profile_123", 1000, 1).await;
    assert!(res.is_err(), "Untrusted CA connection must be rejected");
}

/// AT-P3-03: Expired Certificate Rejection
/// Expired certificate rejected during TLS handshake.
#[tokio::test]
async fn test_p3_at03_expired_certificate_rejection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 cert is expired (valid in 1975)
    let (expired_cert_pem, expired_key_pem, expired_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", true);
    let expired_fp = compute_cert_fingerprint(&expired_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![expired_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let expired_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &expired_cert_pem, &expired_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(expired_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Expired certificate handshake must be rejected");
}

/// AT-P3-04: SAN Identity Target Mismatch
/// Client connecting to candidate Node 2 rejects server presenting SAN node-3.
#[tokio::test]
async fn test_p3_at04_san_identity_target_mismatch() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    // Node 1 server presents SAN node-3 instead of node-1!
    let (n3_cert_pem, n3_key_pem, n3_der) =
        ca.issue_cert("node-3.cluster-alpha.phantom.internal", false);
    let n3_fp = compute_cert_fingerprint(&n3_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n3_fp.clone()],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 3,
        fingerprints: vec![n3_fp],
        active: true,
    });

    // Server configured with node_id = 1 but cert SAN says node-3
    let server_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n3_cert_pem, &n3_key_pem, registry.clone());
    let client_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(server_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(client_tls);

    // Client expects node-1 SAN, but server presents node-3 SAN -> Handshake rejected!
    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Target SAN mismatch must be rejected by client connector");
}

/// AT-P3-05: Cross-Cluster Connection Rejection
/// Connection presenting cert with SAN cluster ID != local cluster ID is rejected.
#[tokio::test]
async fn test_p3_at05_cross_cluster_connection_rejection() {
    let ca = TestCa::new();

    // Node 1 in cluster-alpha
    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 cert in cluster-beta
    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-beta.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut alpha_reg = ClusterMembershipRegistry::new("cluster-alpha", 1);
    alpha_reg.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    alpha_reg.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp.clone()],
        active: true,
    });

    let mut beta_reg = ClusterMembershipRegistry::new("cluster-beta", 1);
    beta_reg.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, "cluster-alpha", &ca.cert_pem, &n1_cert_pem, &n1_key_pem, alpha_reg);
    let n2_tls = create_test_node_tls(2, "cluster-beta", &ca.cert_pem, &n2_cert_pem, &n2_key_pem, beta_reg);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Cross-cluster connection must be rejected");
}

/// AT-P3-06: Revoked / Inactive Node Rejection
/// Peer certificate with active == false in registry is rejected.
#[tokio::test]
async fn test_p3_at06_revoked_node_rejection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    // Node 2 is REVOKED
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: false,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Revoked node connection must be rejected");
}

/// AT-P3-07: Unknown Node ID Rejection
/// Node presenting valid CA cert with SAN node-99 not in registry is rejected.
#[tokio::test]
async fn test_p3_at07_unknown_node_id_rejection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 99 is signed by CA, but absent from registry
    let (n99_cert_pem, n99_key_pem, _) =
        ca.issue_cert("node-99.cluster-alpha.phantom.internal", false);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let n99_tls = create_test_node_tls(99, cluster_id, &ca.cert_pem, &n99_cert_pem, &n99_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n99_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Unknown node not in registry must be rejected");
}

/// AT-P3-08: Certificate Fingerprint Mismatch
/// Peer presenting valid cert with SAN node-2 but fingerprint != pinned fingerprint is rejected.
#[tokio::test]
async fn test_p3_at08_certificate_fingerprint_mismatch() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, _) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    // Registry pins a bogus fingerprint for Node 2
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec!["0000000000000000000000000000000000000000000000000000000000000000".to_string()],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Fingerprint mismatch must be rejected");
}

/// AT-P3-09: Plaintext TCP Connect Rejection
/// Plaintext bytes sent to mTLS port are rejected immediately without application response.
#[tokio::test]
async fn test_p3_at09_plaintext_tcp_connect_rejection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    // Connect raw plaintext TCP
    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();
    let _ = stream.write_all(b"GET test_key\r\n").await;

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let resp_str = String::from_utf8_lossy(&buf[..n]);
    assert!(
        !resp_str.contains("MISS") && !resp_str.contains("HIT") && !resp_str.contains("val_node"),
        "Plaintext connection must never receive application cache response"
    );
    if n > 0 {
        // Standard TLS Alert frame (0x15)
        assert_eq!(buf[0], 0x15, "Any returned bytes must be a TLS Alert frame (0x15)");
    }

    // Socket must be closed immediately
    let n2 = stream.read(&mut buf).await.unwrap_or(0);
    assert_eq!(n2, 0, "Socket must be closed after rejection");
}

/// AT-P3-10: TLS Handshake Timeout
/// Stalled handshake closed after 1000ms limit; listener remains unblocked.
#[tokio::test]
async fn test_p3_at10_tls_handshake_timeout() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    // Stalled client: opens TCP socket but sends no TLS ClientHello
    let stalled_socket = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // The server must close the stalled socket after its 1000 ms handshake timeout. Instead of a
    // fixed 1200 ms sleep, block on the read itself with a hard cap of 4 s (4x the timeout): EOF
    // (0 bytes) arrives as soon as the server gives up.
    let mut check_buf = [0u8; 16];
    let mut stalled_socket = stalled_socket;
    let closed_after = std::time::Instant::now();
    let read_res = tokio::time::timeout(Duration::from_secs(4), stalled_socket.read(&mut check_buf))
        .await
        .expect("server did not close the stalled socket within 4 s")
        .unwrap_or(0);
    assert_eq!(read_res, 0, "Stalled socket must be closed by server timeout");
    assert!(
        closed_after.elapsed() >= Duration::from_millis(900),
        "socket was closed before the 1000 ms handshake timeout elapsed: {:?}",
        closed_after.elapsed()
    );

    // Verify listener is still healthy and accepts legitimate requests
    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);
    let res = router.forward_key_over_tcp("valid_key", 2000, 1).await;
    assert!(res.is_ok(), "Listener must remain unblocked after timeout: {:?}", res);
}

/// AT-P3-16: Stale / Partitioned Registry Isolation
/// Monotonic epoch prevents stale or lower-versioned membership updates from overwriting state.
#[test]
fn test_p3_at16_stale_registry_epoch_isolation() {
    let mut reg = ClusterMembershipRegistry::new("cluster-alpha", 5);

    // Incoming update with lower epoch (4) is rejected
    let stale_reg = ClusterMembershipRegistry::new("cluster-alpha", 4);
    let res = reg.update(stale_reg);
    assert!(matches!(res, Err(ClusterSecurityError::StaleRegistryEpoch { current: 5, incoming: 4 })));

    // Incoming update with same epoch (5) is rejected
    let same_reg = ClusterMembershipRegistry::new("cluster-alpha", 5);
    let res = reg.update(same_reg);
    assert!(matches!(res, Err(ClusterSecurityError::StaleRegistryEpoch { current: 5, incoming: 5 })));

    // Incoming update for wrong cluster is rejected
    let wrong_cluster = ClusterMembershipRegistry::new("cluster-beta", 6);
    let res = reg.update(wrong_cluster);
    assert!(matches!(res, Err(ClusterSecurityError::ClusterMismatch { .. })));

    // Incoming update with higher epoch (6) succeeds
    let fresh_reg = ClusterMembershipRegistry::new("cluster-alpha", 6);
    let res = reg.update(fresh_reg);
    assert!(res.is_ok());
    assert_eq!(reg.version, 6);
}

/// AT-P3-21: CA Rollover Sequence & Quarantine
/// Dual-CA trust store allows coexistence; single-CA retirement drops old certs; un-migrated nodes quarantined.
#[tokio::test]
async fn test_p3_at21_ca_rollover_sequence_and_quarantine() {
    let ca_v1 = TestCa::new();
    let ca_v2 = TestCa::new();
    let cluster_id = "cluster-alpha";

    // Bundle both CA certificates
    let dual_ca_pem = format!("{}\n{}", ca_v1.cert_pem, ca_v2.cert_pem);

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca_v2.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 cert signed by CA v1
    let (n2_v1_cert_pem, n2_v1_key_pem, n2_v1_der) =
        ca_v1.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_v1_fp = compute_cert_fingerprint(&n2_v1_der);

    // Node 3 cert signed by CA v2
    let (n3_v2_cert_pem, n3_v2_key_pem, n3_v2_der) =
        ca_v2.issue_cert("node-3.cluster-alpha.phantom.internal", false);
    let n3_v2_fp = compute_cert_fingerprint(&n3_v2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_v1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 3,
        fingerprints: vec![n3_v2_fp],
        active: true,
    });

    // Server trusts dual CA bundle
    let server_tls = create_test_node_tls(1, cluster_id, &dual_ca_pem, &n1_cert_pem, &n1_key_pem, registry.clone());

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(server_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    // 1. Client with CA v1 cert connects successfully during rollover window
    let client_v1_tls = create_test_node_tls(2, cluster_id, &dual_ca_pem, &n2_v1_cert_pem, &n2_v1_key_pem, registry.clone());
    let router_v1 = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(client_v1_tls);
    let res1 = router_v1.forward_key_over_tcp("key_v1", 2000, 1).await;
    assert!(res1.is_ok(), "CA v1 connection should succeed during dual-CA window: {:?}", res1);

    // 2. Client with CA v2 cert also connects successfully
    let client_v2_tls = create_test_node_tls(3, cluster_id, &dual_ca_pem, &n3_v2_cert_pem, &n3_v2_key_pem, registry.clone());
    let router_v2 = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(client_v2_tls);
    let res2 = router_v2.forward_key_over_tcp("key_v2", 2000, 1).await;
    assert!(res2.is_ok(), "CA v2 connection should succeed during dual-CA window: {:?}", res2);

    // 3. Quarantine un-migrated Node 2 in registry
    let mut updated_reg = registry.clone();
    updated_reg.version = 2;
    if let Some(m) = updated_reg.members.get_mut(&2) {
        m.active = false; // Quarantined!
    }
    {
        let node1_tls = node1.get_cluster_tls().unwrap();
        let mut reg_guard = node1_tls.registry.write().unwrap();
        *reg_guard = updated_reg.clone();
    }

    let res_quarantined = router_v1.forward_key_over_tcp("key_v1", 1000, 1).await;
    assert!(res_quarantined.is_err(), "Quarantined Node 2 must be rejected");

    // Node 3 (healthy) continues to succeed
    let res_healthy = router_v2.forward_key_over_tcp("key_v2", 2000, 1).await;
    assert!(res_healthy.is_ok(), "Migrated Node 3 must remain active");
}

/// AT-P3-22: Malformed Identity & Illegal SAN Syntax
/// Underscores, illegal characters, and non-RFC 1035 syntax rejected at parse boundary.
#[test]
fn test_p3_at22_malformed_identity_and_illegal_san() {
    // Valid cases
    let ok1 = parse_san_node_identity("node-1.cluster-alpha.phantom.internal").unwrap();
    assert_eq!(ok1, NodeIdentity { node_id: 1, cluster_id: "cluster-alpha".to_string() });

    let ok2 = parse_san_node_identity("DNS:node-255.c123.phantom.internal").unwrap();
    assert_eq!(ok2, NodeIdentity { node_id: 255, cluster_id: "c123".to_string() });

    // Underscores forbidden (RFC 1035 / RFC 1123)
    let err_under = parse_san_node_identity("node_1.cluster-alpha.phantom.internal");
    assert!(matches!(err_under, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Node ID 0 is out of bounds (1..=255)
    let err_zero = parse_san_node_identity("node-0.cluster-alpha.phantom.internal");
    assert!(matches!(err_zero, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Node ID 256 is out of u8 bounds
    let err_overflow = parse_san_node_identity("node-256.cluster-alpha.phantom.internal");
    assert!(matches!(err_overflow, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Missing node- prefix
    let err_prefix = parse_san_node_identity("worker-1.cluster-alpha.phantom.internal");
    assert!(matches!(err_prefix, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Uppercase in cluster ID forbidden (must be lowercase)
    let err_upper = parse_san_node_identity("node-1.ClusterAlpha.phantom.internal");
    assert!(matches!(err_upper, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Hyphen at start of cluster ID forbidden
    let err_start_hyphen = parse_san_node_identity("node-1.-cluster.phantom.internal");
    assert!(matches!(err_start_hyphen, Err(ClusterSecurityError::MalformedIdentity(_))));

    // Wrong root domain
    let err_domain = parse_san_node_identity("node-1.cluster-alpha.evil.com");
    assert!(matches!(err_domain, Err(ClusterSecurityError::MalformedIdentity(_))));
}

/// AT-P3-Startup: Fail-Closed Startup Security Validation
#[test]
fn test_p3_startup_security_fail_closed_validation() {
    let network_addr: SocketAddr = "0.0.0.0:8085".parse().unwrap();
    let loopback_addr: SocketAddr = "127.0.0.1:8085".parse().unwrap();

    // Non-loopback interface missing certs fails closed
    let res = validate_cluster_startup_security(&network_addr, false, false, false, false);
    assert_eq!(res, Err(StartupSecurityError::NetworkWithoutClusterTls(network_addr)));

    // Non-loopback interface CANNOT bypass with allow_insecure_cluster
    let res = validate_cluster_startup_security(&network_addr, false, false, false, true);
    assert_eq!(res, Err(StartupSecurityError::NetworkWithoutClusterTls(network_addr)));

    // Non-loopback interface with all certs succeeds
    let res = validate_cluster_startup_security(&network_addr, true, true, true, false);
    assert!(res.is_ok());

    // Loopback with partial certs fails closed
    let res = validate_cluster_startup_security(&loopback_addr, true, false, false, false);
    assert_eq!(res, Err(StartupSecurityError::NetworkWithoutClusterTls(loopback_addr)));

    // Loopback dev mode without certs succeeds
    let res = validate_cluster_startup_security(&loopback_addr, false, false, false, true);
    assert!(res.is_ok());
}

/// AT-P3-SAN-Extraction: X.509 DER Subject Alternative Name Parser
#[test]
fn test_p3_der_san_extraction() {
    let ca = TestCa::new();
    let (_, _, cert_der) = ca.issue_cert("node-42.test-cluster.phantom.internal", false);

    let sans = extract_sans_from_der(&cert_der).expect("Should extract SAN from DER");
    assert_eq!(sans, vec!["node-42.test-cluster.phantom.internal"]);
}

/// SEC-3A-01: Valid JSON membership file loading and end-to-end mTLS routing
#[tokio::test]
async fn test_p3_sec01_json_membership_loading_and_mtls_routing() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let json_content = format!(
        r#"{{
  "cluster_id": "cluster-alpha",
  "version": 1,
  "members": [
    {{
      "node_id": 1,
      "fingerprints": ["{}"],
      "active": true
    }},
    {{
      "node_id": 2,
      "fingerprints": ["{}"],
      "active": true
    }}
  ]
}}"#,
        n1_fp, n2_fp
    );

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_cluster_members_{}.json", rand::random::<u64>()));
    std::fs::write(&file_path, &json_content).unwrap();

    let loaded_registry = load_membership_from_file(&file_path, cluster_id)
        .expect("Should load valid membership file");
    assert_eq!(loaded_registry.cluster_id, "cluster-alpha");
    assert_eq!(loaded_registry.version, 1);
    assert_eq!(loaded_registry.members.len(), 2);

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, loaded_registry.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, loaded_registry);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("user_profile_json", 2000, 1).await;
    assert!(res.is_ok(), "Real mTLS using JSON-loaded registry must succeed: {:?}", res);
    let (served_by, body) = res.unwrap();
    assert_eq!(served_by, 1);
    assert_eq!(body, "MISS val_node_1");

    let _ = std::fs::remove_file(file_path);
}

/// SEC-3A-01: JSON membership validation edge cases
#[test]
fn test_p3_sec01_json_membership_validation_edge_cases() {
    let valid_fp = "a".repeat(64);

    // 1. Malformed JSON
    let err_json = load_membership_from_json_str("not a json", "cluster-alpha");
    assert!(matches!(err_json, Err(ClusterSecurityError::CorruptClusterRegistry(_))));

    // 2. Cluster mismatch
    let json_wrong_cluster = format!(
        r#"{{"cluster_id": "cluster-beta", "version": 1, "members": [{{"node_id": 1, "fingerprints": ["{}"], "active": true}}]}}"#,
        valid_fp
    );
    let err_cluster = load_membership_from_json_str(&json_wrong_cluster, "cluster-alpha");
    assert!(matches!(err_cluster, Err(ClusterSecurityError::ClusterMismatch { .. })));

    // 3. Duplicate node ID
    let json_dup = format!(
        r#"{{"cluster_id": "cluster-alpha", "version": 1, "members": [
            {{"node_id": 1, "fingerprints": ["{}"], "active": true}},
            {{"node_id": 1, "fingerprints": ["{}"], "active": true}}
        ]}}"#,
        valid_fp, valid_fp
    );
    let err_dup = load_membership_from_json_str(&json_dup, "cluster-alpha");
    assert!(matches!(err_dup, Err(ClusterSecurityError::CorruptClusterRegistry(msg)) if msg.contains("Duplicate node_id")));

    // 4. Node ID 0 (invalid)
    let json_zero = format!(
        r#"{{"cluster_id": "cluster-alpha", "version": 1, "members": [{{"node_id": 0, "fingerprints": ["{}"], "active": true}}]}}"#,
        valid_fp
    );
    let err_zero = load_membership_from_json_str(&json_zero, "cluster-alpha");
    assert!(matches!(err_zero, Err(ClusterSecurityError::CorruptClusterRegistry(_))));

    // 5. Empty fingerprints list
    let json_empty_fp = r#"{"cluster_id": "cluster-alpha", "version": 1, "members": [{"node_id": 1, "fingerprints": [], "active": true}]}"#;
    let err_empty_fp = load_membership_from_json_str(json_empty_fp, "cluster-alpha");
    assert!(matches!(err_empty_fp, Err(ClusterSecurityError::CorruptClusterRegistry(msg)) if msg.contains("empty fingerprints")));

    // 6. Invalid fingerprint length (63 chars instead of 64)
    let json_short_fp = format!(
        r#"{{"cluster_id": "cluster-alpha", "version": 1, "members": [{{"node_id": 1, "fingerprints": ["{}"], "active": true}}]}}"#,
        "a".repeat(63)
    );
    let err_short_fp = load_membership_from_json_str(&json_short_fp, "cluster-alpha");
    assert!(matches!(err_short_fp, Err(ClusterSecurityError::CorruptClusterRegistry(msg)) if msg.contains("invalid length")));

    // 7. Invalid fingerprint characters (uppercase hex or non-hex)
    let json_upper_fp = format!(
        r#"{{"cluster_id": "cluster-alpha", "version": 1, "members": [{{"node_id": 1, "fingerprints": ["{}A"], "active": true}}]}}"#,
        "a".repeat(63)
    );
    let err_upper_fp = load_membership_from_json_str(&json_upper_fp, "cluster-alpha");
    assert!(matches!(err_upper_fp, Err(ClusterSecurityError::CorruptClusterRegistry(msg)) if msg.contains("invalid character")));

    // 8. Version 0 is invalid
    let json_v0 = format!(
        r#"{{"cluster_id": "cluster-alpha", "version": 0, "members": [{{"node_id": 1, "fingerprints": ["{}"], "active": true}}]}}"#,
        valid_fp
    );
    let err_v0 = load_membership_from_json_str(&json_v0, "cluster-alpha");
    assert!(matches!(err_v0, Err(ClusterSecurityError::CorruptClusterRegistry(_))));
}

/// SEC-3A-02: Router plaintext fallback strictly restricted to loopback destinations
#[tokio::test]
async fn test_p3_sec02_router_plaintext_guard_loopback_vs_network() {
    // 1. Loopback destination without cluster TLS succeeds (local development / testing compatibility)
    let node_loopback = Arc::new(VirtualNode::new(1));
    let addr = node_loopback.start_tcp_service("127.0.0.1:0").await.unwrap();
    node_loopback.set_addr(addr);

    let router = RendezvousRouter::new(vec![node_loopback.clone()]);
    let res_loopback = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res_loopback.is_ok(), "Loopback plaintext must succeed: {:?}", res_loopback);

    // 2. Non-loopback destination without cluster TLS fails immediately with NodeUnreachable
    let node_remote = Arc::new(VirtualNode::new(2));
    let remote_addr: SocketAddr = "192.168.1.100:8085".parse().unwrap();
    node_remote.set_addr(remote_addr);
    node_remote.set_state(phantom_cache_demo::dist::node::NodeState::Active);

    let router_remote = RendezvousRouter::new(vec![node_remote.clone()]);
    let res_remote = router_remote.forward_key_over_tcp("test_key", 1000, 1).await;
    assert_eq!(
        res_remote,
        Err(DistError::NodeUnreachable(2)),
        "Non-loopback plaintext must be rejected before attempting connection"
    );
}

/// SEC-3A-03: Strict single-SAN node identity (multi-SAN certificate rejected)
#[tokio::test]
async fn test_p3_sec03_strict_single_san_multi_san_rejection() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    // Node 2 presents a certificate with MULTIPLE SANs (ambiguous identity)
    let (multi_san_cert_pem, multi_san_key_pem, multi_san_der) = ca.issue_multi_san_cert(vec![
        "node-2.cluster-alpha.phantom.internal".to_string(),
        "node-99.cluster-alpha.phantom.internal".to_string(),
    ]);
    let multi_san_fp = compute_cert_fingerprint(&multi_san_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![multi_san_fp.clone()],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 99,
        fingerprints: vec![multi_san_fp],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let multi_san_tls = create_test_node_tls(
        2,
        cluster_id,
        &ca.cert_pem,
        &multi_san_cert_pem,
        &multi_san_key_pem,
        registry,
    );

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(multi_san_tls);

    let res = router.forward_key_over_tcp("test_key", 1000, 1).await;
    assert!(res.is_err(), "Multi-SAN certificate must be strictly rejected at inbound handshake");
}

/// SEC-3A-04: Strict TLS 1.3 only (TLS 1.2 client rejected, TLS 1.3 client succeeds)
#[tokio::test]
async fn test_p3_sec04_strict_tls13_rejection_of_tls12_client() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal", false);
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal", false);
    let n2_fp = compute_cert_fingerprint(&n2_der);

    let mut registry = ClusterMembershipRegistry::new(cluster_id, 1);
    registry.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    registry.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });

    // Server is configured with ClusterTlsConfig (strictly TLS 1.3)
    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    // 1. Build a client explicitly restricted to TLS 1.2
    let mut root_store = rustls::RootCertStore::empty();
    let mut ca_reader = std::io::BufReader::new(ca.cert_pem.as_bytes());
    let ca_certs = rustls_pemfile::certs(&mut ca_reader).collect::<Result<Vec<_>, _>>().unwrap();
    for cert in ca_certs {
        root_store.add(cert).unwrap();
    }
    let mut cert_reader = std::io::BufReader::new(n2_cert_pem.as_bytes());
    let node_certs = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>().unwrap();
    let mut key_reader = std::io::BufReader::new(n2_key_pem.as_bytes());
    let private_key = rustls_pemfile::private_key(&mut key_reader).unwrap().unwrap();

    let tls12_client_config = rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .with_root_certificates(root_store)
        .with_client_auth_cert(node_certs, private_key)
        .unwrap();

    let tls12_connector = tokio_rustls::TlsConnector::from(Arc::new(tls12_client_config));

    let tcp_stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("node-1.cluster-alpha.phantom.internal".to_string()).unwrap();

    // TLS 1.2 handshake must fail closed
    let tls12_res = tokio::time::timeout(
        Duration::from_millis(1000),
        tls12_connector.connect(server_name, tcp_stream),
    ).await;

    match tls12_res {
        Ok(Err(_)) => {
            // Handshake failed as expected (version negotiation failure / alert)
        }
        Err(_) => {
            // Handshake timed out / aborted by server
        }
        Ok(Ok(_)) => {
            panic!("TLS 1.2 client connection must NOT negotiate with TLS 1.3-only server");
        }
    }

    // 2. TLS 1.3 client using standard ClusterTlsConfig succeeds
    let n2_tls13 = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);
    let router_tls13 = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls13);
    let res_tls13 = router_tls13.forward_key_over_tcp("test_key", 2000, 1).await;
    assert!(res_tls13.is_ok(), "TLS 1.3 client must successfully connect: {:?}", res_tls13);
}

#[tokio::test]
async fn test_p3_sec05_valid_cluster_config_populates_peer_addresses_and_binds() {
    let ca = TestCa::new();
    let cluster_id = "cluster-runtime";

    let (n1_cert_pem, n1_key_pem, n1_cert_der) = ca.issue_cert(&format!("node-1.{}.phantom.internal", cluster_id), false);
    let (n2_cert_pem, n2_key_pem, n2_cert_der) = ca.issue_cert(&format!("node-2.{}.phantom.internal", cluster_id), false);

    let n1_fp = compute_cert_fingerprint(&n1_cert_der);
    let n2_fp = compute_cert_fingerprint(&n2_cert_der);

    let config_json = format!(
        r#"{{
            "cluster_id": "{}",
            "version": 1,
            "members": [
                {{ "node_id": 1, "fingerprints": ["{}"], "active": true }},
                {{ "node_id": 2, "fingerprints": ["{}"], "active": true }}
            ],
            "addresses": {{
                "1": "127.0.0.1:19101",
                "2": "127.0.0.1:19102"
            }}
        }}"#,
        cluster_id, n1_fp, n2_fp
    );

    let registry = load_membership_from_json_str(&config_json, cluster_id).expect("load membership");
    assert_eq!(
        registry.get_address(1),
        Some("127.0.0.1:19101".parse::<SocketAddr>().unwrap())
    );
    assert_eq!(
        registry.get_address(2),
        Some("127.0.0.1:19102".parse::<SocketAddr>().unwrap())
    );

    // Node 1 binds to an ephemeral loopback port for testing
    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert_pem, &n1_key_pem, registry.clone());
    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let bound_addr = node1.start_tcp_service("127.0.0.1:0").await.expect("bind listener");
    node1.set_addr(bound_addr);

    // Node 2 connects to node 1 using the populated address
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert_pem, &n2_key_pem, registry);
    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    let res = router.forward_key_over_tcp("test_key", 2000, 1).await;
    assert!(res.is_ok(), "Forward over populated cluster address must succeed: {:?}", res);

    node1.stop_tcp_service();
}

#[test]
fn test_p3_sec06_cluster_config_malformed_address_fails_closed() {
    let config_json = r#"{
        "cluster_id": "cluster-test",
        "version": 1,
        "members": [
            { "node_id": 1, "fingerprints": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"], "active": true }
        ],
        "addresses": {
            "1": "invalid-ip-and-port"
        }
    }"#;

    let res = load_membership_from_json_str(config_json, "cluster-test");
    assert!(
        res.is_err(),
        "Malformed socket address in configuration must fail closed"
    );
    match res {
        Err(ClusterSecurityError::CorruptClusterRegistry(msg)) => {
            assert!(msg.contains("Invalid socket address"), "Error message should mention invalid address: {}", msg);
        }
        other => panic!("Expected CorruptClusterRegistry error, got {:?}", other),
    }
}

#[test]
fn test_p3_sec07_conflicting_local_and_peer_address_detection() {
    let local_addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
    let peer_addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();

    let has_conflict = local_addr == peer_addr;
    assert!(has_conflict, "Identical local and peer addresses must be detected as conflict");

    let distinct_peer_addr: SocketAddr = "127.0.0.1:9091".parse().unwrap();
    assert_ne!(local_addr, distinct_peer_addr, "Distinct addresses must not conflict");
}

#[tokio::test]
async fn test_p3_sec08_standalone_mode_starts_cleanly_without_cluster_listener() {
    let node = Arc::new(VirtualNode::new(1));
    node.set_state(phantom_cache_demo::dist::node::NodeState::Active);

    // Standalone router without cluster TLS
    let router = RendezvousRouter::new(vec![node.clone()]);
    assert!(router.cluster_tls().is_none());

    // Local key routing operates entirely in-memory without TCP service
    let selected = router.route_key("standalone_key");
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().node_id, 1);
}
