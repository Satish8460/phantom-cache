use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, PartitionedCache, MAX_TENANT_EPOCHS,
};
use phantom_cache_demo::dist::frame::{
    read_frame, write_frame, FrameError, InvalidationScope, InvalidationStatus, MessageType,
    PhantomMeshFrame, INVALIDATE_ACK_PAYLOAD_LEN,
    MAX_KEY_LENGTH_BYTES, MIN_INVALIDATE_REQUEST_PAYLOAD,
};
use phantom_cache_demo::dist::node::VirtualNode;
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

// =========================================================================
// Test 1: Codec Round-Trips and Boundary Rejections
// =========================================================================
#[test]
fn test_p4a_codec_round_trips_and_boundaries() {
    // 1. Exact round-trip with maximum allowable key (4,096 bytes)
    let max_key = "a".repeat(MAX_KEY_LENGTH_BYTES);
    let frame = PhantomMeshFrame::new_invalidate_request(
        1001,
        1,
        5,
        1200,
        1,
        InvalidationScope::ExactKey,
        &max_key,
    )
    .expect("Should construct frame with max key length");

    assert_eq!(frame.header.msg_type, MessageType::InvalidateRequest);
    assert_eq!(frame.payload.len(), 20 + MAX_KEY_LENGTH_BYTES);

    let decoded = frame.extract_invalidate_request().expect("Decode should succeed");
    assert_eq!(decoded.membership_epoch, 5);
    assert_eq!(decoded.stream_seq, 1200);
    assert_eq!(decoded.originator_id, 1);
    assert_eq!(decoded.scope, InvalidationScope::ExactKey);
    assert_eq!(decoded.key, max_key);

    // 2. Exact round-trip for InvalidateAck
    let ack_frame = PhantomMeshFrame::new_invalidate_ack(
        1001,
        2,
        5,
        1200,
        2,
        InvalidationStatus::SuccessEvicted,
        4096,
    )
    .expect("Should construct InvalidateAck");

    assert_eq!(ack_frame.header.msg_type, MessageType::InvalidateAck);
    assert_eq!(ack_frame.payload.len(), INVALIDATE_ACK_PAYLOAD_LEN);

    let decoded_ack = ack_frame.extract_invalidate_ack().expect("Decode ack should succeed");
    assert_eq!(decoded_ack.membership_epoch, 5);
    assert_eq!(decoded_ack.stream_seq, 1200);
    assert_eq!(decoded_ack.responder_id, 2);
    assert_eq!(decoded_ack.status, InvalidationStatus::SuccessEvicted);
    assert_eq!(decoded_ack.reclaimed_bytes, 4096);

    // 3. Oversized key (> 4,096 bytes) rejected at construction
    let huge_key = "b".repeat(MAX_KEY_LENGTH_BYTES + 1);
    let err = PhantomMeshFrame::new_invalidate_request(
        1002,
        1,
        5,
        1201,
        1,
        InvalidationScope::ExactKey,
        &huge_key,
    )
    .unwrap_err();

    assert_eq!(
        err,
        FrameError::KeyTooLarge {
            length: MAX_KEY_LENGTH_BYTES + 1,
            max: MAX_KEY_LENGTH_BYTES,
        }
    );

    // 4. Truncated payload (< 20 bytes) rejected
    let truncated_frame = PhantomMeshFrame::new(
        MessageType::InvalidateRequest,
        0,
        1,
        1,
        vec![0u8; 18],
    )
    .unwrap();
    let trunc_err = truncated_frame.extract_invalidate_request().unwrap_err();
    assert_eq!(
        trunc_err,
        FrameError::TruncatedPayload {
            expected: MIN_INVALIDATE_REQUEST_PAYLOAD as u32,
            received: 18,
        }
    );

    // 5. Invalid scope (0x7F) rejected
    let mut bad_scope_payload = vec![0u8; 20];
    bad_scope_payload[17] = 0x7F;
    let bad_scope_frame = PhantomMeshFrame::new(
        MessageType::InvalidateRequest,
        0,
        1,
        1,
        bad_scope_payload,
    )
    .unwrap();
    assert_eq!(
        bad_scope_frame.extract_invalidate_request().unwrap_err(),
        FrameError::InvalidScope(0x7F)
    );

    // 6. Invalid UTF-8 key rejected
    let mut bad_utf8_payload = vec![0u8; 24];
    bad_utf8_payload[18] = 0;
    bad_utf8_payload[19] = 4; // key_len = 4
    bad_utf8_payload[20..24].copy_from_slice(&[0xFF, 0xFE, 0xFD, 0xFC]);
    let bad_utf8_frame = PhantomMeshFrame::new(
        MessageType::InvalidateRequest,
        0,
        1,
        1,
        bad_utf8_payload,
    )
    .unwrap();
    assert_eq!(
        bad_utf8_frame.extract_invalidate_request().unwrap_err(),
        FrameError::InvalidKeyUtf8
    );

    // 7. Invalid InvalidateAck length rejected
    let bad_len_ack = PhantomMeshFrame::new(
        MessageType::InvalidateAck,
        0,
        1,
        1,
        vec![0u8; 25],
    )
    .unwrap();
    assert_eq!(
        bad_len_ack.extract_invalidate_ack().unwrap_err(),
        FrameError::InvalidPayloadLength {
            expected: INVALIDATE_ACK_PAYLOAD_LEN,
            received: 25,
        }
    );
}

// =========================================================================
// Test 2: Invalidation of Existing and Missing Key over mTLS
// =========================================================================
#[tokio::test]
async fn test_p4a_invalidation_of_existing_and_missing_key_over_mtls() {
    let ca = TestCa::new();
    let cluster_id = "cluster-p4a";

    let (n1_cert, n1_key, n1_der) = ca.issue_cert("node-1.cluster-p4a.phantom.internal");
    let (n2_cert, n2_key, n2_der) = ca.issue_cert("node-2.cluster-p4a.phantom.internal");

    let mut reg = ClusterMembershipRegistry::new(cluster_id, 1);
    reg.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![compute_cert_fingerprint(&n1_der)],
        active: true,
    });
    reg.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![compute_cert_fingerprint(&n2_der)],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert, &n1_key, reg.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert, &n2_key, reg);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    // Populate node 1 cache directly
    let key = CacheKey::new(Some("tenant_alpha"), "/catalog/items", Some("id=42"));
    let canonical = key.to_canonical_string();
    let now_ms = 1_000_000;
    node1.cache.insert(&canonical, b"payload_bytes_42".to_vec(), 60_000, now_ms);

    // Verify key exists in cache initially
    assert_eq!(node1.cache.get(&canonical, now_ms), Some(b"payload_bytes_42".to_vec()));

    let mut client = connect_mtls_client(addr, n2_tls, 1, cluster_id).await;

    // Send InvalidateRequest for existing key
    let req_id = 998811;
    let req_frame = PhantomMeshFrame::new_invalidate_request(
        req_id,
        1,
        1,
        101,
        2,
        InvalidationScope::ExactKey,
        &canonical,
    )
    .unwrap();

    write_frame(&mut client, &req_frame).await.unwrap();

    // Read InvalidateAck response
    let ack_frame = read_frame(&mut client).await.unwrap();
    assert_eq!(ack_frame.header.msg_type, MessageType::InvalidateAck);
    assert_eq!(ack_frame.header.request_id, req_id);

    let ack_payload = ack_frame.extract_invalidate_ack().unwrap();
    assert_eq!(ack_payload.membership_epoch, 1);
    assert_eq!(ack_payload.stream_seq, 101);
    assert_eq!(ack_payload.responder_id, 1);
    assert_eq!(ack_payload.status, InvalidationStatus::SuccessEvicted);
    assert_eq!(ack_payload.reclaimed_bytes, 16); // "payload_bytes_42".len() == 16

    // Verify key is now gone from cache
    assert_eq!(node1.cache.get(&canonical, now_ms), None);

    // Send InvalidateRequest again for now-missing key
    let req_id_2 = 998812;
    let req_frame_2 = PhantomMeshFrame::new_invalidate_request(
        req_id_2,
        2,
        1,
        102,
        2,
        InvalidationScope::ExactKey,
        &canonical,
    )
    .unwrap();

    write_frame(&mut client, &req_frame_2).await.unwrap();

    let ack_frame_2 = read_frame(&mut client).await.unwrap();
    assert_eq!(ack_frame_2.header.request_id, req_id_2);

    let ack_payload_2 = ack_frame_2.extract_invalidate_ack().unwrap();
    assert_eq!(ack_payload_2.status, InvalidationStatus::SuccessNotPresent);
    assert_eq!(ack_payload_2.reclaimed_bytes, 0);

    node1.stop_tcp_service();
}

// =========================================================================
// Test 3: In-Flight Fetch Generational Fencing Prevents Stale Writes
// =========================================================================
#[tokio::test]
async fn test_p4a_inflight_fetch_fencing_prevents_stale_writes() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = CacheKey::new(Some("tenant_sec"), "/products/99", None);
    let now_ms = 1_000_000;

    // 1. Begin fetch atomic (simulating in-flight origin fetch)
    let reg_result = cache.begin_fetch_atomic(&key, now_ms);
    let reg = match reg_result {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate fetch result"),
    };

    assert_eq!(reg.op_gen, 0);
    let guard = reg.into_guard();

    // 2. Invalidation arrives while fetch is in-flight!
    let (was_present, _) = cache.delete_canonical(&key.to_canonical_string());
    assert!(!was_present); // Key wasn't populated yet

    // 3. In-flight fetch finishes and attempts to commit stale response
    let fetched_resource = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(b"{\"id\": 99, \"stale\": true}".to_vec()),
        etag: Some("\"etag-old\"".to_string()),
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    let before_stale_drops = cache.counters.stale_writes_prevented.load(std::sync::atomic::Ordering::Relaxed);
    guard.complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(fetched_resource));
    let after_stale_drops = cache.counters.stale_writes_prevented.load(std::sync::atomic::Ordering::Relaxed);

    // 4. Assert: Stale write was prevented!
    assert_eq!(after_stale_drops, before_stale_drops + 1);

    // 5. Assert: Cache does NOT contain the stale value
    let lookup = cache.get_resource(&key, now_ms + 10);
    assert!(lookup.is_none(), "Stale value must NOT be present in cache");
}

// =========================================================================
// Test 4: Duplicate Delivery is Idempotent
// =========================================================================
#[tokio::test]
async fn test_p4a_duplicate_delivery_is_idempotent() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key = CacheKey::new(Some("t1"), "/profile", None);
    let canonical = key.to_canonical_string();
    let now_ms = 500_000;

    // Insert entry
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"initial_data".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
        reg.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }

    assert!(cache.get_resource(&key, now_ms).is_some());

    // First deletion
    let (p1, r1) = cache.delete_canonical(&canonical);
    assert!(p1);
    assert_eq!(r1, 12); // "initial_data".len()

    // Duplicate deletion of the same key
    let (p2, r2) = cache.delete_canonical(&canonical);
    assert!(!p2);
    assert_eq!(r2, 0);

    // Third duplicate deletion
    let (p3, r3) = cache.delete_canonical(&canonical);
    assert!(!p3);
    assert_eq!(r3, 0);

    // Cache remains empty without panic or corruption
    assert!(cache.get_resource(&key, now_ms).is_none());
}

// =========================================================================
// Test 5: Distinct Keys Cannot Suppress Each Other
// =========================================================================
#[tokio::test]
async fn test_p4a_distinct_keys_cannot_suppress_each_other() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key_alpha = CacheKey::new(Some("t1"), "/alpha", None);
    let key_beta = CacheKey::new(Some("t1"), "/beta", None);
    let now_ms = 500_000;

    let res_alpha = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"data_alpha".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    let res_beta = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"data_beta".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_alpha, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_alpha));
    }
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_beta, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_beta));
    }

    assert!(cache.get_resource(&key_alpha, now_ms).is_some());
    assert!(cache.get_resource(&key_beta, now_ms).is_some());

    // Invalidate key_alpha
    let (p_a, _) = cache.delete_canonical(&key_alpha.to_canonical_string());
    assert!(p_a);

    // Assert: key_alpha is purged, but key_beta remains cached!
    assert!(cache.get_resource(&key_alpha, now_ms).is_none());
    assert!(cache.get_resource(&key_beta, now_ms).is_some());

    // Invalidate key_beta
    let (p_b, _) = cache.delete_canonical(&key_beta.to_canonical_string());
    assert!(p_b);

    // Now both are purged
    assert!(cache.get_resource(&key_alpha, now_ms).is_none());
    assert!(cache.get_resource(&key_beta, now_ms).is_none());
}

// =========================================================================
// Test 6: Tenant Invalidation Cannot Affect Another Tenant
// =========================================================================
#[tokio::test]
async fn test_p4a_tenant_invalidation_isolation() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key_t1 = CacheKey::new(Some("tenant_one"), "/shared_endpoint", None);
    let key_t2 = CacheKey::new(Some("tenant_two"), "/shared_endpoint", None);
    let now_ms = 500_000;

    let res_t1 = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"tenant_one_data".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    let res_t2 = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"tenant_two_data".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_t1, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_t1));
    }
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_t2, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_t2));
    }

    assert!(cache.get_resource(&key_t1, now_ms).is_some());
    assert!(cache.get_resource(&key_t2, now_ms).is_some());

    // Invalidate tenant_one exclusively
    let new_epoch = cache.invalidate_tenant("tenant_one");
    assert_eq!(new_epoch, 1);

    // Assert: tenant_one data is now treated as stale/miss!
    assert!(cache.get_resource(&key_t1, now_ms).is_none());

    // Assert: tenant_two data is STILL a HIT and completely unaffected!
    let t2_lookup = cache.get_resource(&key_t2, now_ms);
    assert!(t2_lookup.is_some());
    assert_eq!(*t2_lookup.unwrap().body, b"tenant_two_data".to_vec());
}

// =========================================================================
// Test 7: FullCache Invalidation Scope
// =========================================================================
#[tokio::test]
async fn test_p4a_full_cache_invalidation_scope() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key1 = CacheKey::new(Some("t1"), "/item1", None);
    let key2 = CacheKey::new(Some("t2"), "/item2", None);
    let now_ms = 500_000;

    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"data".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key1, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res.clone()));
    }
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key2, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }

    assert!(cache.get_resource(&key1, now_ms).is_some());
    assert!(cache.get_resource(&key2, now_ms).is_some());

    // Trigger full cache invalidation
    cache.invalidate_all();

    // Assert both are now misses
    assert!(cache.get_resource(&key1, now_ms).is_none());
    assert!(cache.get_resource(&key2, now_ms).is_none());
}

// =========================================================================
// Test 8: Malformed Frame over mTLS Closes Socket Safely
// =========================================================================
#[tokio::test]
async fn test_p4a_malformed_invalidation_frame_terminates_socket() {
    let ca = TestCa::new();
    let cluster_id = "cluster-malformed";

    let (n1_cert, n1_key, n1_der) = ca.issue_cert("node-1.cluster-malformed.phantom.internal");
    let (n2_cert, n2_key, n2_der) = ca.issue_cert("node-2.cluster-malformed.phantom.internal");

    let mut reg = ClusterMembershipRegistry::new(cluster_id, 1);
    reg.add_member(MemberNode {
        node_id: 1,
        fingerprints: vec![compute_cert_fingerprint(&n1_der)],
        active: true,
    });
    reg.add_member(MemberNode {
        node_id: 2,
        fingerprints: vec![compute_cert_fingerprint(&n2_der)],
        active: true,
    });

    let n1_tls = create_test_node_tls(1, cluster_id, &ca.cert_pem, &n1_cert, &n1_key, reg.clone());
    let n2_tls = create_test_node_tls(2, cluster_id, &ca.cert_pem, &n2_cert, &n2_key, reg);

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let mut client = connect_mtls_client(addr, n2_tls, 1, cluster_id).await;

    // Send malformed InvalidateRequest with invalid scope (0xFE)
    let mut bad_payload = vec![0u8; 20];
    bad_scope_bytes(&mut bad_payload, 0xFE);
    let bad_frame = PhantomMeshFrame::new(
        MessageType::InvalidateRequest,
        0,
        777,
        1,
        bad_payload,
    )
    .unwrap();

    write_frame(&mut client, &bad_frame).await.unwrap();

    // Node should detect malformed frame and terminate socket cleanly
    let read_result = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut client)).await;
    match read_result {
        Ok(Err(FrameError::ConnectionClosed)) => {} // Expected clean close
        Ok(Err(e)) => panic!("Expected ConnectionClosed, got {:?}", e),
        Ok(Ok(f)) => panic!("Expected socket close, but got frame: {:?}", f),
        Err(_) => panic!("Timed out waiting for socket close"),
    }

    node1.stop_tcp_service();
}

fn bad_scope_bytes(buf: &mut [u8], scope: u8) {
    buf[17] = scope;
}

// =========================================================================
// Test 9 (SEC-4A-01 Regression): Stale Entry Resurrection Prevented After Cap Exhaustion
// =========================================================================
#[tokio::test]
async fn test_p4a_sec01_regression_resurrection_prevented_after_cap_exhaustion() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key_alpha = CacheKey::new(Some("tenant_victim"), "/api/v1/sensitive", None);
    let now_ms = 1_000_000;

    // 1. Populate tenant_victim at initial epoch 0
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(b"{\"secret\":\"stale_data\"}".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key_alpha, now_ms) {
        reg.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
    assert!(cache.get_resource(&key_alpha, now_ms).is_some());

    // 2. Invalidate tenant_victim -> epoch becomes 1
    let ep = cache.invalidate_tenant("tenant_victim");
    assert_eq!(ep, 1);

    // Note: Do NOT access key_alpha here! The unaccessed entry in shard map still holds entry_tenant_epoch = 0.

    // 3. Exhaust tenant-epoch capacity by invalidating 10,000 distinct other tenants
    for i in 0..MAX_TENANT_EPOCHS {
        cache.invalidate_tenant(&format!("noise_tenant_{}", i));
    }

    // 4. In the original flawed implementation:
    //    tenant_victim was arbitrarily evicted from tenant_epochs map.
    //    get_tenant_epoch("tenant_victim") defaulted back to 0.
    //    get_resource(&key_alpha) matched entry_tenant_epoch (0) == current_tenant_epoch (0) -> RESURRECTED!
    //
    // In remediated implementation:
    //    Global fallback advanced global_epoch from 0 to 1 upon cap exhaustion.
    //    entry_global_epoch (0) != current_global_epoch (1) -> MISS!
    assert!(
        cache.get_resource(&key_alpha, now_ms).is_none(),
        "CRITICAL REGRESSION: Pre-invalidation cached entry must NEVER be resurrected after cap exhaustion!"
    );
}

// =========================================================================
// Test 10 (SEC-4A-01): Invalidating > 10,000 Distinct Tenants Strictly Bounded
// =========================================================================
#[tokio::test]
async fn test_p4a_sec01_over_10k_tenants_strictly_bounded_and_valid() {
    let cache = Arc::new(PartitionedCache::new(4));

    // Invalidate 10,500 distinct tenants
    for i in 0..10_500 {
        cache.invalidate_tenant(&format!("tenant_{}", i));
        let map_len = cache.tenant_epochs.lock().unwrap().len();
        assert!(
            map_len <= MAX_TENANT_EPOCHS,
            "tenant_epochs map size {} exceeded MAX_TENANT_EPOCHS {}",
            map_len,
            MAX_TENANT_EPOCHS
        );
    }

    // After 10,500 distinct tenants, capacity fallback occurred at 10,001:
    // map cleared and added tenant_10000 (1 entry), plus 499 subsequent tenants = 500 entries.
    let final_len = cache.tenant_epochs.lock().unwrap().len();
    assert_eq!(final_len, 500);

    // Invalidate a target tenant after exhaustion and verify eviction
    let key_post = CacheKey::new(Some("tenant_post"), "/data", None);
    let now_ms = 1_000_000;
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"fresh".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key_post, now_ms) {
        reg.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
    assert!(cache.get_resource(&key_post, now_ms).is_some());

    cache.invalidate_tenant("tenant_post");
    assert!(cache.get_resource(&key_post, now_ms).is_none());
}

// =========================================================================
// Test 11 (SEC-4A-01): In-Flight Fetch Fencing During Capacity Fallback
// =========================================================================
#[tokio::test]
async fn test_p4a_sec01_inflight_fetch_fencing_during_capacity_exhaustion() {
    let cache = Arc::new(PartitionedCache::new(4));
    let key_inflight = CacheKey::new(Some("tenant_inflight"), "/heavy_query", None);
    let now_ms = 1_000_000;

    // 1. Begin fetch atomic under global_epoch = 0
    let reg_result = cache.begin_fetch_atomic(&key_inflight, now_ms);
    let reg = match reg_result {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate fetch result"),
    };
    let guard = reg.into_guard();

    // 2. While fetch is in flight, trigger capacity exhaustion fallback (10,001 distinct tenants)
    for i in 0..=MAX_TENANT_EPOCHS {
        cache.invalidate_tenant(&format!("exhaust_{}", i));
    }

    // 3. Fetch completes and attempts to commit stale response
    let stale_resource = Arc::new(CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(b"stale_inflight".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    let before_drops = cache.counters.stale_writes_prevented.load(std::sync::atomic::Ordering::Relaxed);
    guard.complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(stale_resource));
    let after_drops = cache.counters.stale_writes_prevented.load(std::sync::atomic::Ordering::Relaxed);

    // 4. Assert: Write was fenced because global_epoch advanced
    assert_eq!(after_drops, before_drops + 1);
    assert!(cache.get_resource(&key_inflight, now_ms + 10).is_none());
}

// =========================================================================
// Test 12 (SEC-4A-01): Tenant Isolation Under Capacity Pressure
// =========================================================================
#[tokio::test]
async fn test_p4a_sec01_tenant_isolation_under_capacity_pressure() {
    let cache = Arc::new(PartitionedCache::new(4));
    let now_ms = 500_000;

    let key_a = CacheKey::new(Some("tenant_iso_a"), "/res", None);
    let key_b = CacheKey::new(Some("tenant_iso_b"), "/res", None);

    let res_a = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"data_a".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    let res_b = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"data_b".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });

    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_a, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_a));
    }
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key_b, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res_b));
    }

    // Invalidate tenant_iso_a: tenant_iso_b must remain a HIT
    cache.invalidate_tenant("tenant_iso_a");
    assert!(cache.get_resource(&key_a, now_ms).is_none());
    assert!(cache.get_resource(&key_b, now_ms).is_some());

    // Invalidate 5,000 other tenants (within capacity)
    for i in 0..5_000 {
        cache.invalidate_tenant(&format!("fill_{}", i));
    }

    // tenant_iso_b STILL remains a HIT!
    let b_lookup = cache.get_resource(&key_b, now_ms);
    assert!(b_lookup.is_some());
    assert_eq!(*b_lookup.unwrap().body, b"data_b".to_vec());
}

// =========================================================================
// Test 13 (SEC-4A-01): Repeated Exhaustion Cycles and Invalidate All Interactions
// =========================================================================
#[tokio::test]
async fn test_p4a_sec01_repeated_exhaustion_cycles_and_global_interaction() {
    let cache = Arc::new(PartitionedCache::new(4));
    let now_ms = 500_000;

    // Cycle 1: Fill past 10,000 (10,001 distinct tenants triggers fallback to global_epoch = 1)
    for i in 0..=MAX_TENANT_EPOCHS {
        cache.invalidate_tenant(&format!("c1_tenant_{}", i));
    }
    assert_eq!(cache.global_epoch.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(cache.tenant_epochs.lock().unwrap().len(), 1);

    // Call invalidate_all() directly
    let g = cache.invalidate_all();
    assert_eq!(g, 2);
    assert_eq!(cache.tenant_epochs.lock().unwrap().len(), 0);

    // Cycle 2: Fill past 10,000 again (10,001 distinct tenants triggers fallback to global_epoch = 3)
    for i in 0..=MAX_TENANT_EPOCHS {
        cache.invalidate_tenant(&format!("c2_tenant_{}", i));
    }
    assert_eq!(cache.global_epoch.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(cache.tenant_epochs.lock().unwrap().len(), 1);

    // Re-insert and verify entry under new global epoch
    let key = CacheKey::new(Some("tenant_c2"), "/path", None);
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"c2_val".to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(&key, now_ms) {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
    assert!(cache.get_resource(&key, now_ms).is_some());
}

// =========================================================================
// Test 14 (SEC-4A-01): Checked Overflow Behavior
// =========================================================================
#[test]
fn test_p4a_sec01_epoch_overflow_checked() {
    let cache = PartitionedCache::new(4);
    // Artificially saturate tenant epoch map entry to u64::MAX
    {
        let mut guard = cache.tenant_epochs.lock().unwrap();
        guard.insert("tenant_max".to_string(), u64::MAX);
    }

    // Must panic on checked_add overflow rather than silently wrapping to 0
    let result = std::panic::catch_unwind(|| {
        cache.invalidate_tenant("tenant_max");
    });
    assert!(result.is_err(), "Expected panic on tenant epoch overflow");
}
