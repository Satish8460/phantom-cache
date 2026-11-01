use phantom_cache_demo::dist::frame::{
    read_frame, write_frame, FrameHeader, MessageType, PhantomMeshFrame, HEADER_LEN, MAGIC,
    MAX_FRAME_PAYLOAD_BYTES, MAX_KEY_LENGTH_BYTES, VERSION_1,
};
use phantom_cache_demo::dist::node::VirtualNode;
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, ClusterMembershipRegistry, ClusterTlsConfig, MemberNode,
};
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

/// Helper to establish a mutual TLS connection to a node.
async fn connect_mtls_client(
    addr: std::net::SocketAddr,
    client_tls: Arc<ClusterTlsConfig>,
    target_node_id: u8,
    cluster_id: &str,
) -> tokio_rustls::client::TlsStream<tokio::net::TcpStream> {
    let tcp_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let connector = client_tls.client_connector();
    let server_name = rustls::pki_types::ServerName::try_from(format!(
        "node-{}.{}.phantom.internal",
        target_node_id, cluster_id
    ))
    .unwrap();
    connector.connect(server_name, tcp_stream).await.unwrap()
}

/// AT-P3-11: Valid binary GetRequest and GetResponse over mTLS
#[tokio::test]
async fn test_p3b_at11_valid_binary_get_request_over_mtls() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal");
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal");
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

    let router = RendezvousRouter::new(vec![node1.clone()]).with_cluster_tls(n2_tls);

    // First request: Cold miss
    let res1 = router.forward_key_over_tcp("user_profile_123", 2000, 1).await;
    assert!(res1.is_ok(), "Forwarding over binary mTLS must succeed: {:?}", res1);
    let (node_id, body1) = res1.unwrap();
    assert_eq!(node_id, 1);
    assert_eq!(body1, "MISS val_node_1");

    // Second request: Cache hit
    let res2 = router.forward_key_over_tcp("user_profile_123", 2000, 1).await;
    assert!(res2.is_ok(), "Second request must hit cache: {:?}", res2);
    let (_, body2) = res2.unwrap();
    assert_eq!(body2, "HIT val_node_1");
}

/// AT-P3-11: Valid binary GetRequest and GetResponse over loopback TCP (dev mode)
#[tokio::test]
async fn test_p3b_at11_valid_binary_get_request_over_loopback() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(n1_addr);

    let router = RendezvousRouter::new(vec![node1.clone()]);

    let res = router.forward_key_over_tcp("dev_key_456", 2000, 1).await;
    assert!(res.is_ok(), "Loopback binary forwarding must succeed: {:?}", res);
    let (served_by, body) = res.unwrap();
    assert_eq!(served_by, 1);
    assert_eq!(body, "MISS val_node_1");
}

/// AT-P3-11: Legacy ASCII protocol rejected on secure binary protocol path (No Downgrade)
#[tokio::test]
async fn test_p3b_at11_legacy_ascii_rejection_no_downgrade() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Send legacy ASCII command "GET legacy_key\r\n"
    stream.write_all(b"GET legacy_key\r\n").await.unwrap();
    stream.flush().await.unwrap();

    // Node must abort immediately on BadMagic ([0x47, 0x45, 0x54, 0x20] != "PHAN")
    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();

    assert_eq!(n, 0, "Server must close connection without sending any response to legacy ASCII input");
}

/// AT-P3-11: Frame with bad magic is rejected immediately
#[tokio::test]
async fn test_p3b_at11_bad_magic_rejected() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Construct 28-byte header with invalid magic "NOPE"
    let mut bad_header = [0u8; HEADER_LEN];
    bad_header[0..4].copy_from_slice(b"NOPE");
    bad_header[4] = VERSION_1;
    bad_header[5] = MessageType::GetRequest.to_u8();

    stream.write_all(&bad_header).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Bad magic frame must immediately close socket");
}

/// AT-P3-11: Frame with unsupported version is rejected
#[tokio::test]
async fn test_p3b_at11_unsupported_version_rejected() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Construct 28-byte header with magic "PHAN" but version = 99
    let mut bad_header = [0u8; HEADER_LEN];
    bad_header[0..4].copy_from_slice(&MAGIC);
    bad_header[4] = 99; // Unsupported version
    bad_header[5] = MessageType::GetRequest.to_u8();

    stream.write_all(&bad_header).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Unsupported version must immediately close socket");
}

/// AT-P3-11: Frame with unknown message type is rejected
#[tokio::test]
async fn test_p3b_at11_unknown_message_type_rejected() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Construct 28-byte header with unknown message type 0xFE
    let mut bad_header = [0u8; HEADER_LEN];
    bad_header[0..4].copy_from_slice(&MAGIC);
    bad_header[4] = VERSION_1;
    bad_header[5] = 0xFE; // Unknown message type

    stream.write_all(&bad_header).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Unknown message type must immediately close socket");
}

/// AT-P3-11: Oversized payload length (> 1 MB) rejected before reading or allocating payload
#[tokio::test]
async fn test_p3b_at11_oversized_payload_rejected_before_alloc() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Header claims payload length = 2 MB (2,097,152 bytes > 1,048,576 byte ceiling)
    let mut bad_header = [0u8; HEADER_LEN];
    bad_header[0..4].copy_from_slice(&MAGIC);
    bad_header[4] = VERSION_1;
    bad_header[5] = MessageType::GetRequest.to_u8();
    let oversized_len = (MAX_FRAME_PAYLOAD_BYTES * 2).to_be_bytes();
    bad_header[24..28].copy_from_slice(&oversized_len);

    // Send only the 28-byte header (zero payload sent)
    stream.write_all(&bad_header).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Oversized payload header must immediately close socket without hanging");
}

/// AT-P3-11: Oversized key (> 4096 bytes) rejected safely
#[tokio::test]
async fn test_p3b_at11_oversized_key_rejected() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // 5000 bytes key exceeds MAX_KEY_LENGTH_BYTES (4096)
    let huge_key = vec![b'k'; MAX_KEY_LENGTH_BYTES + 500];
    let header = FrameHeader {
        magic: MAGIC,
        version: VERSION_1,
        msg_type: MessageType::GetRequest,
        flags: 0,
        request_id: 1,
        sequence_number: 1,
        payload_length: huge_key.len() as u32,
    };

    stream.write_all(&header.encode()).await.unwrap();
    stream.write_all(&huge_key).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Oversized key must be rejected by VirtualNode and socket closed");
}

/// AT-P3-11: Truncated header handled gracefully without panic or hang
#[tokio::test]
async fn test_p3b_at11_truncated_header_handled_gracefully() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Send only 15 bytes of the 28-byte header and close socket
    let partial_header = [0x50, 0x48, 0x41, 0x4E, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    stream.write_all(&partial_header).await.unwrap();
    stream.shutdown().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Truncated header must cause clean connection teardown");
}

/// AT-P3-11: Truncated payload handled gracefully without panic or hang
#[tokio::test]
async fn test_p3b_at11_truncated_payload_handled_gracefully() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    // Header claims 100 bytes payload
    let header = FrameHeader {
        magic: MAGIC,
        version: VERSION_1,
        msg_type: MessageType::GetRequest,
        flags: 0,
        request_id: 1,
        sequence_number: 1,
        payload_length: 100,
    };
    stream.write_all(&header.encode()).await.unwrap();

    // Send only 25 bytes of payload and shutdown
    let partial_payload = [b'x'; 25];
    stream.write_all(&partial_payload).await.unwrap();
    stream.shutdown().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Truncated payload must cause clean connection teardown");
}

/// AT-P3-11: Partial I/O reassembly (slow byte-by-byte sender)
#[tokio::test]
async fn test_p3b_at11_partial_chunked_io_reassembly() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    let req_frame = PhantomMeshFrame::new_get_request(777, 10, "slow_chunked_key").unwrap();
    let mut wire_bytes = Vec::new();
    wire_bytes.extend_from_slice(&req_frame.header.encode());
    wire_bytes.extend_from_slice(&req_frame.payload);

    // Send the frame in small 3-byte chunks with small sleeps to simulate packet fragmentation
    for chunk in wire_bytes.chunks(3) {
        stream.write_all(chunk).await.unwrap();
        stream.flush().await.unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }

    // Node must reassemble and return a valid response frame
    let resp = read_frame(&mut stream).await.expect("Should reassemble valid frame");
    assert_eq!(resp.header.msg_type, MessageType::GetResponse);
    assert_eq!(resp.header.request_id, 777);
    assert_eq!(resp.header.sequence_number, 11);
    assert_eq!(resp.extract_response().unwrap(), "MISS val_node_1");
}

/// AT-P3-11: Invalid UTF-8 key rejected safely
#[tokio::test]
async fn test_p3b_at11_invalid_utf8_key_rejected() {
    let node1 = Arc::new(VirtualNode::new(1));
    let n1_addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(n1_addr).await.unwrap();

    let invalid_utf8_bytes = vec![0xFF, 0xFE, 0xFD];
    let header = FrameHeader {
        magic: MAGIC,
        version: VERSION_1,
        msg_type: MessageType::GetRequest,
        flags: 0,
        request_id: 1,
        sequence_number: 1,
        payload_length: invalid_utf8_bytes.len() as u32,
    };

    stream.write_all(&header.encode()).await.unwrap();
    stream.write_all(&invalid_utf8_bytes).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = [0u8; 128];
    let n = stream.read(&mut buf).await.unwrap();
    assert_eq!(n, 0, "Invalid UTF-8 key must cause socket closure");
}

/// AT-P3-11: Request ID and Sequence Number correlation
#[tokio::test]
async fn test_p3b_at11_request_correlation_and_sequence_number() {
    let ca = TestCa::new();
    let cluster_id = "cluster-alpha";

    let (n1_cert_pem, n1_key_pem, n1_der) =
        ca.issue_cert("node-1.cluster-alpha.phantom.internal");
    let n1_fp = compute_cert_fingerprint(&n1_der);

    let (n2_cert_pem, n2_key_pem, n2_der) =
        ca.issue_cert("node-2.cluster-alpha.phantom.internal");
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

    let mut tls_stream = connect_mtls_client(n1_addr, n2_tls, 1, cluster_id).await;

    let test_req_id = 0xCAFE_BABE_1234_5678_u64;
    let test_seq = 42_u64;

    let req_frame = PhantomMeshFrame::new_get_request(test_req_id, test_seq, "correlation_key").unwrap();
    write_frame(&mut tls_stream, &req_frame).await.unwrap();

    let resp_frame = read_frame(&mut tls_stream).await.unwrap();
    assert_eq!(resp_frame.header.msg_type, MessageType::GetResponse);
    assert_eq!(resp_frame.header.request_id, test_req_id, "Response must correlate request_id");
    assert_eq!(
        resp_frame.header.sequence_number,
        test_seq + 1,
        "Response must increment sequence_number"
    );
    assert_eq!(resp_frame.extract_response().unwrap(), "MISS val_node_1");
}

