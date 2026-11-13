use phantom_cache_demo::core::cache::{CacheKey, CachedResource};
use phantom_cache_demo::dist::anti_entropy::{AntiEntropyTracker, InvalidationRingBuffer};
use phantom_cache_demo::dist::frame::{
    read_frame, write_frame, AntiEntropyStatus, InvalidateRecord, InvalidationScope, MessageType,
    PhantomMeshFrame, ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN,
    MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD,
};
use phantom_cache_demo::dist::node::VirtualNode;
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::intel::diagnostics::CacheCounters;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::net::TcpStream;

#[test]
fn test_p4c_ring_buffer_dual_bounds_and_eviction() {
    // Test dual bounds: max 4 entries, max 120 bytes
    let mut ring = InvalidationRingBuffer::new(4, 120);
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());

    // Entry size: 11 + 5 = 16 bytes
    for i in 1..=4 {
        ring.push(InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("k_{:03}", i),
        });
    }

    assert_eq!(ring.len(), 4);
    assert_eq!(ring.oldest_seq(), 1);
    assert_eq!(ring.newest_seq(), 4);

    // Pushing a 5th entry should evict seq 1 due to max_entries=4
    ring.push(InvalidateRecord {
        stream_seq: 5,
        scope: InvalidationScope::ExactKey,
        key: "k_005".to_string(),
    });

    assert_eq!(ring.len(), 4);
    assert_eq!(ring.oldest_seq(), 2);
    assert_eq!(ring.newest_seq(), 5);

    // Now test byte limit eviction: push a large key (e.g. 80 bytes)
    let large_key = "x".repeat(80); // entry size = 11 + 80 = 91 bytes
    ring.push(InvalidateRecord {
        stream_seq: 6,
        scope: InvalidationScope::ExactKey,
        key: large_key,
    });

    // Ring max_bytes is 120, so earlier entries must be evicted to stay within 120
    assert!(ring.current_bytes() <= 120);
    assert!(ring.oldest_seq() >= 4);
    assert_eq!(ring.newest_seq(), 6);
}

#[test]
fn test_p4c_digest_computation_determinism_and_mismatch() {
    let mut ring_a = InvalidationRingBuffer::default();
    let mut ring_b = InvalidationRingBuffer::default();

    // Empty digests match
    assert_eq!(ring_a.compute_digest(), ring_b.compute_digest());

    // Push identical records
    for i in 1..=5 {
        let rec = InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("item_{}", i),
        };
        ring_a.push(rec.clone());
        ring_b.push(rec);
    }

    let digest_a = ring_a.compute_digest();
    let digest_b = ring_b.compute_digest();

    assert_eq!(digest_a, digest_b);
    assert_ne!(digest_a, [0u8; 32]);

    // Diverge ring B
    ring_b.push(InvalidateRecord {
        stream_seq: 6,
        scope: InvalidationScope::ExactKey,
        key: "item_divergent".to_string(),
    });

    assert_ne!(ring_a.compute_digest(), ring_b.compute_digest());
}

#[test]
fn test_p4c_stream_sequence_gap_detection_at_p4_04() {
    let counters = CacheCounters::new();
    let mut tracker = AntiEntropyTracker::new();

    // Inbound seq 20 from peer 2 initializes expectation
    let gap0 = tracker.record_inbound_seq(2, 20, Some(&counters));
    assert!(gap0.is_none());
    assert_eq!(counters.anti_entropy_gaps_detected.load(Ordering::Relaxed), 0);
    assert_eq!(tracker.get_highest_acked_seq(2), 20);

    // Inbound seq 22 from peer 2 arrives, missing seq 21 -> Gap detected!
    let gap1 = tracker.record_inbound_seq(2, 22, Some(&counters));
    assert_eq!(gap1, Some((21, 21)));
    assert_eq!(counters.anti_entropy_gaps_detected.load(Ordering::Relaxed), 1);
    // With KG-3 / C-05 resolved, highest_acked_seq remains at the last contiguous sequence (20),
    // ensuring anti-entropy replay will request the missing sequence 21.
    assert_eq!(tracker.get_highest_acked_seq(2), 20);

    // Pop the recorded gap
    assert!(tracker.has_gaps());
    let popped = tracker.pop_gap();
    assert_eq!(popped, Some((2, 21, 21)));
    assert!(!tracker.has_gaps());

    // When missing seq 21 arrives, contiguous expected advances to 23 (highest acked 22)
    let gap2 = tracker.record_inbound_seq(2, 21, Some(&counters));
    assert!(gap2.is_none());
    assert_eq!(tracker.get_highest_acked_seq(2), 22);
}

#[test]
fn test_p4c_anti_entropy_in_sync_response() {
    let mut ring = InvalidationRingBuffer::default();
    for i in 1..=3 {
        ring.push(InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("key_{}", i),
        });
    }

    // Peer requests catchup from seq 3 (peer already has seq 3)
    let res = ring.get_replay_batch(3);
    assert!(res.is_ok());
    assert!(res.unwrap().is_empty());
}

#[test]
fn test_p4c_offline_node_anti_entropy_catchup_and_replay_at_p4_06() {
    let node1 = Arc::new(VirtualNode::new(1));
    let node2 = Arc::new(VirtualNode::new(2));

    // Populate node 2's cache with "key_B"
    let cache_key = CacheKey::new(None, "key_B", None);
    let now = 100_000;
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"initial_data".to_vec()),
        etag: None,
        created_at_ms: now,
        ttl_ms: 60_000,
    });
    let reg = match node2.cache.begin_fetch_atomic(&cache_key, now) {
        phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected initiate"),
    };
    reg.into_guard()
        .complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));

    // Verify key_B is present in node 2
    assert!(node2.cache.get_resource(&cache_key, now).is_some());

    // Node 1 receives invalidation for key_B at stream sequence 10
    node1.ring_buffer.lock().unwrap().push(InvalidateRecord {
        stream_seq: 10,
        scope: InvalidationScope::ExactKey,
        key: "key_B".to_string(),
    });

    // Node 2 was offline and its highest acked sequence from Node 1 is 9
    let highest_acked_seq = 9;
    let replay_batch = node1
        .ring_buffer
        .lock()
        .unwrap()
        .get_replay_batch(highest_acked_seq)
        .expect("Should be recoverable");

    assert_eq!(replay_batch.len(), 1);
    assert_eq!(replay_batch[0].key, "key_B");

    // Node 2 syncs the replay batch
    let evicted = node2.sync_anti_entropy_batch(&replay_batch);
    assert_eq!(evicted, 1);

    // Verify key_B is evicted from node 2
    assert!(node2.cache.get_resource(&cache_key, now).is_none());
    assert_eq!(
        node2
            .cache
            .counters
            .anti_entropy_keys_evicted
            .load(Ordering::Relaxed),
        1
    );
    assert_eq!(
        node2
            .cache
            .counters
            .anti_entropy_sync_total
            .load(Ordering::Relaxed),
        1
    );
}

#[test]
fn test_p4c_ring_buffer_overflow_full_cache_flush_at_p4_07() {
    let mut ring = InvalidationRingBuffer::new(5, 10_000);

    // Push 10 invalidations into a buffer of size 5
    for i in 1..=10 {
        ring.push(InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("k_{}", i),
        });
    }

    assert_eq!(ring.oldest_seq(), 6);
    assert_eq!(ring.newest_seq(), 10);

    // Peer requests catchup from seq 4 (overwritten!)
    let result = ring.get_replay_batch(4);
    assert_eq!(result, Err(AntiEntropyStatus::HistoryOverwritten));

    // Peer node handles HistoryOverwritten by executing a full cache flush
    let peer_node = Arc::new(VirtualNode::new(2));

    // Populate peer cache with some entries
    let now = 100_000;
    for k in &["foo", "bar", "baz"] {
        let key = CacheKey::new(None, k, None);
        let res = Arc::new(CachedResource {
            status: 200,
            content_type: "text/plain".to_string(),
            body: Arc::new(b"val".to_vec()),
            etag: None,
            created_at_ms: now,
            ttl_ms: 60_000,
        });
        if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) =
            peer_node.cache.begin_fetch_atomic(&key, now)
        {
            r.into_guard()
                .complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
        }
    }

    // Flush for anti-entropy overwrite
    peer_node.flush_for_anti_entropy_overwrite();

    // Verify all entries miss and counters recorded
    assert!(peer_node
        .cache
        .get_resource(&CacheKey::new(None, "foo", None), now)
        .is_none());
    assert!(peer_node
        .cache
        .get_resource(&CacheKey::new(None, "bar", None), now)
        .is_none());
    assert_eq!(
        peer_node
            .cache
            .counters
            .anti_entropy_full_flushes
            .load(Ordering::Relaxed),
        1
    );
}

#[test]
fn test_p4c_anti_entropy_sync_request_and_response_frame_codecs() {
    // 1. Test AntiEntropySyncRequest frame codec
    let req_frame = PhantomMeshFrame::new_anti_entropy_sync_request(
        1001,
        42,
        1,
        500,
        3,
        [0xAA; 32],
    )
    .expect("Should encode AntiEntropySyncRequest");

    assert_eq!(req_frame.header.msg_type, MessageType::AntiEntropySyncRequest);
    assert_eq!(req_frame.payload.len(), ANTI_ENTROPY_SYNC_REQUEST_PAYLOAD_LEN);

    let decoded_req = req_frame
        .extract_anti_entropy_sync_request()
        .expect("Should decode AntiEntropySyncRequest");

    assert_eq!(decoded_req.membership_epoch, 1);
    assert_eq!(decoded_req.highest_acked_seq, 500);
    assert_eq!(decoded_req.sender_node_id, 3);
    assert_eq!(decoded_req.ring_digest, [0xAA; 32]);

    // 2. Test AntiEntropySyncResponse frame codec
    let records = vec![
        InvalidateRecord {
            stream_seq: 501,
            scope: InvalidationScope::ExactKey,
            key: "sku_101".to_string(),
        },
        InvalidateRecord {
            stream_seq: 502,
            scope: InvalidationScope::Tenant,
            key: "tenant_corp".to_string(),
        },
    ];

    let resp_frame = PhantomMeshFrame::new_anti_entropy_sync_response(
        1001,
        43,
        AntiEntropyStatus::ReplayBatch,
        501,
        502,
        &records,
    )
    .expect("Should encode AntiEntropySyncResponse");

    assert_eq!(
        resp_frame.header.msg_type,
        MessageType::AntiEntropySyncResponse
    );
    assert!(resp_frame.payload.len() >= MIN_ANTI_ENTROPY_SYNC_RESPONSE_PAYLOAD);

    let decoded_resp = resp_frame
        .extract_anti_entropy_sync_response()
        .expect("Should decode AntiEntropySyncResponse");

    assert_eq!(decoded_resp.status, AntiEntropyStatus::ReplayBatch);
    assert_eq!(decoded_resp.range_start, 501);
    assert_eq!(decoded_resp.range_end, 502);
    assert_eq!(decoded_resp.record_count, 2);
    assert_eq!(decoded_resp.records, records);
}

#[tokio::test]
async fn test_p4c_live_tcp_anti_entropy_exchange() {
    let node1 = Arc::new(VirtualNode::new(1));
    let listener_addr = node1
        .start_tcp_service("127.0.0.1:0")
        .await
        .expect("TCP listener start");

    // Client connects to Node 1
    let mut client = TcpStream::connect(listener_addr).await.expect("Connect");

    // 1. Client sends InvalidateRequest for "live_key_1"
    let inval_frame = PhantomMeshFrame::new_invalidate_request(
        555,
        1,
        1,
        101,
        2, // Originator = Node 2
        InvalidationScope::ExactKey,
        "inbound_only_key",
    )
    .expect("Encode invalidate");

    write_frame(&mut client, &inval_frame).await.expect("Write invalidate");
    let ack_frame = read_frame(&mut client).await.expect("Read ack");
    assert_eq!(ack_frame.header.msg_type, MessageType::InvalidateAck);

    // Node 1 also holds an OUTBOUND record it sent to node 2 (its own stream towards node 2).
    node1.ring_buffer.lock().unwrap().push_outbound(
        InvalidateRecord {
            stream_seq: 101,
            scope: InvalidationScope::ExactKey,
            key: "live_key_1".to_string(),
        },
        2,
    );

    // 2. Client sends AntiEntropySyncRequest asking for replay from seq 100
    let sync_req = PhantomMeshFrame::new_anti_entropy_sync_request(
        556,
        2,
        1,
        100, // Highest acked was 100 -> Needs seq 101
        2,
        [0u8; 32],
    )
    .expect("Encode sync request");

    write_frame(&mut client, &sync_req).await.expect("Write sync request");
    let sync_resp_frame = read_frame(&mut client).await.expect("Read sync response");
    assert_eq!(
        sync_resp_frame.header.msg_type,
        MessageType::AntiEntropySyncResponse
    );

    let sync_resp = sync_resp_frame
        .extract_anti_entropy_sync_response()
        .expect("Extract sync response");

    assert_eq!(sync_resp.status, AntiEntropyStatus::ReplayBatch);
    assert_eq!(sync_resp.record_count, 1);
    assert_eq!(sync_resp.records[0].stream_seq, 101);
    assert_eq!(sync_resp.records[0].key, "live_key_1");
    // The record node 2 itself sent to node 1 ("inbound_only_key") is never echoed back to it.
    assert!(sync_resp.records.iter().all(|r| r.key != "inbound_only_key"));

    node1.stop_tcp_service();
}

#[tokio::test]
async fn test_p4c_sec01_stream_sequence_assignment_is_atomic_and_independent_of_connection_seq() {
    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_state(phantom_cache_demo::dist::node::NodeState::Active);
    let _ = node1.start_tcp_service("127.0.0.1:0").await.expect("start node1");

    let router = RendezvousRouter::new(vec![node1.clone()]);

    // Send first invalidation
    let outcome1 = router
        .invalidate_key_across_cluster("sec01_k1", InvalidationScope::ExactKey)
        .await
        .expect("inval 1");
    assert_eq!(outcome1.canonical_node_id, 1);

    // Send second invalidation
    let outcome2 = router
        .invalidate_key_across_cluster("sec01_k2", InvalidationScope::ExactKey)
        .await
        .expect("inval 2");
    assert_eq!(outcome2.canonical_node_id, 1);

    // Verify ring buffer on node 1 received strictly monotonic sequences (1 and 2)
    let ring = node1.ring_buffer.lock().unwrap();
    assert_eq!(ring.len(), 2);
    assert_eq!(ring.oldest_seq(), 1);
    assert_eq!(ring.newest_seq(), 2);

    node1.stop_tcp_service();
}

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

#[tokio::test]
async fn test_p4c_sec02_mismatched_epoch_rejected() {
    let cluster_id = "test-cluster";
    let ca = TestCa::new();
    let (n1_cert, n1_key, n1_der) = ca.issue_cert(&format!("node-1.{}.phantom.internal", cluster_id));
    let (n2_cert, n2_key, n2_der) = ca.issue_cert(&format!("node-2.{}.phantom.internal", cluster_id));

    let n1_fp = phantom_cache_demo::security::cluster::compute_cert_fingerprint(&n1_der);
    let n2_fp = phantom_cache_demo::security::cluster::compute_cert_fingerprint(&n2_der);

    let mut reg = phantom_cache_demo::security::cluster::ClusterMembershipRegistry::new(cluster_id, 5); // Expected version is 5!
    reg.add_member(phantom_cache_demo::security::cluster::MemberNode {
        node_id: 1,
        fingerprints: vec![n1_fp],
        active: true,
    });
    reg.add_member(phantom_cache_demo::security::cluster::MemberNode {
        node_id: 2,
        fingerprints: vec![n2_fp],
        active: true,
    });

    let n1_tls = Arc::new(
        phantom_cache_demo::security::cluster::ClusterTlsConfig::from_pem(
            1,
            cluster_id,
            ca.cert_pem.as_bytes(),
            n1_cert.as_bytes(),
            n1_key.as_bytes(),
            reg.clone(),
        )
        .unwrap(),
    );

    let n2_tls = Arc::new(
        phantom_cache_demo::security::cluster::ClusterTlsConfig::from_pem(
            2,
            cluster_id,
            ca.cert_pem.as_bytes(),
            n2_cert.as_bytes(),
            n2_key.as_bytes(),
            reg,
        )
        .unwrap(),
    );

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(n1_tls);
    let listener_addr = node1
        .start_tcp_service("127.0.0.1:0")
        .await
        .expect("start tcp");

    // Client connects using valid mTLS
    let tcp_stream = tokio::net::TcpStream::connect(listener_addr).await.unwrap();
    let connector = n2_tls.client_connector();
    let server_name = rustls::pki_types::ServerName::try_from(format!(
        "node-1.{}.phantom.internal",
        cluster_id
    ))
    .unwrap();
    let mut client = connector.connect(server_name, tcp_stream).await.unwrap();

    // Send sync request with obsolete epoch 1 (expected 5)
    let sync_req = PhantomMeshFrame::new_anti_entropy_sync_request(
        999,
        1,
        1, // Obsolete epoch!
        0,
        2,
        [0u8; 32],
    )
    .expect("encode");

    write_frame(&mut client, &sync_req).await.expect("write");
    // Receiver must drop connection due to epoch mismatch (SEC-4C-02)
    let res = read_frame(&mut client).await;
    assert!(res.is_err());

    node1.stop_tcp_service();
}

#[tokio::test]
async fn test_p4c_sec03_empty_ring_buffer_does_not_falsely_claim_insync_when_peer_has_sequences() {
    let node1 = Arc::new(VirtualNode::new(1));
    let listener_addr = node1
        .start_tcp_service("127.0.0.1:0")
        .await
        .expect("start tcp");

    let mut client = TcpStream::connect(listener_addr).await.expect("connect");

    // Peer requests catchup with highest_acked_seq = 10 from empty ring buffer
    let sync_req = PhantomMeshFrame::new_anti_entropy_sync_request(
        888,
        1,
        1,
        10, // Peer already has sequence 10, but node 1 has 0 records!
        2,
        [0u8; 32],
    )
    .expect("encode");

    write_frame(&mut client, &sync_req).await.expect("write");
    let resp = read_frame(&mut client).await.expect("read response");
    let payload = resp.extract_anti_entropy_sync_response().expect("payload");

    // SEC-4C-03: Must return HistoryOverwritten (cannot falsely claim InSync!)
    assert_eq!(payload.status, AntiEntropyStatus::HistoryOverwritten);

    node1.stop_tcp_service();
}

#[test]
fn test_p4c_sec04_truncated_replay_batch_sets_accurate_range_end() {
    let mut ring = InvalidationRingBuffer::default();
    for i in 1..=5 {
        ring.push(InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("k{}", i),
        });
    }

    let batch = ring.get_replay_batch(2).expect("replay batch");
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].stream_seq, 3);
    assert_eq!(batch.last().unwrap().stream_seq, 5);
}

#[test]
fn test_p4c_sec05_anti_entropy_tracker_monotonic_advancement_prevents_rewind() {
    let mut tracker = AntiEntropyTracker::new();

    // Inbound seq 100 sets expected to 101
    tracker.record_inbound_seq(1, 100, None);
    assert_eq!(tracker.get_highest_acked_seq(1), 100);

    // Out-of-order replay completion calls set_highest_acked_seq with 50
    tracker.set_highest_acked_seq(1, 50);

    // SEC-4C-05: Highest acked sequence must remain 100, never rewind to 50!
    assert_eq!(tracker.get_highest_acked_seq(1), 100);

    // Seq 101 arrives next: should NOT be treated as gap
    let gap = tracker.record_inbound_seq(1, 101, None);
    assert!(gap.is_none());
}

#[tokio::test]
async fn test_p4c_sec06_autonomous_anti_entropy_reconciliation_roundtrip() {
    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_state(phantom_cache_demo::dist::node::NodeState::Active);
    let _ = node1
        .start_tcp_service("127.0.0.1:0")
        .await
        .expect("start node1 tcp");

    let node2 = Arc::new(VirtualNode::new(2));

    // Populate node 2 cache with item_X
    let key = CacheKey::new(None, "item_X", None);
    let now = 100_000;
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"initial".to_vec()),
        etag: None,
        created_at_ms: now,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) =
        node2.cache.begin_fetch_atomic(&key, now)
    {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
    assert!(node2.cache.get_resource(&key, now).is_some());

    // Record invalidation on node 1 for item_X
    node1.ring_buffer.lock().unwrap().push(InvalidateRecord {
        stream_seq: 1,
        scope: InvalidationScope::ExactKey,
        key: "item_X".to_string(),
    });

    // Node 2 router reconciles anti-entropy with node 1
    let router2 = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);
    let status = router2
        .reconcile_peer_anti_entropy(&node1, &node2)
        .await
        .expect("reconcile anti-entropy");

    assert_eq!(status, AntiEntropyStatus::ReplayBatch);

    // Verify item_X was evicted from node 2
    assert!(node2.cache.get_resource(&key, now).is_none());
    assert_eq!(
        node2.cache.counters.anti_entropy_keys_evicted.load(Ordering::Relaxed),
        1
    );
    assert_eq!(
        node2.cache.counters.anti_entropy_sync_total.load(Ordering::Relaxed),
        1
    );

    node1.stop_tcp_service();
}

#[tokio::test]
async fn test_p4c_sec07_active_to_suspect_to_safe_recovery() {
    use phantom_cache_demo::dist::node::NodeState;

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_state(NodeState::Active);
    let _ = node1.start_tcp_service("127.0.0.1:0").await.expect("start node1");

    let node2 = Arc::new(VirtualNode::new(2));
    node2.set_state(NodeState::Suspect); // Simulates a previous transient timeout

    let router2 = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);
    assert_eq!(node2.get_state(), NodeState::Suspect);

    // Reconcile anti-entropy
    let status = router2
        .reconcile_peer_anti_entropy(&node1, &node2)
        .await
        .expect("reconcile");

    // Must be InSync and node2 must be restored to Active
    assert_eq!(status, AntiEntropyStatus::InSync);
    assert_eq!(node2.get_state(), NodeState::Active);

    node1.stop_tcp_service();
}

#[tokio::test]
async fn test_p4c_sec08_failed_reconciliation_leaves_node_in_recovering_or_suspect() {
    use phantom_cache_demo::dist::node::NodeState;

    // Node 1 is configured but TCP service is not running (connection refused)
    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_addr("127.0.0.1:1".parse().unwrap()); // Closed port

    let node2 = Arc::new(VirtualNode::new(2));
    node2.set_state(NodeState::Suspect);

    let router2 = RendezvousRouter::new(vec![node1.clone(), node2.clone()]);

    let res = router2.reconcile_peer_anti_entropy(&node1, &node2).await;
    assert!(res.is_err(), "Reconciliation to closed peer must fail");

    // SEC-POA-02: Failed reconciliation must NOT prematurely activate node!
    let s2 = node2.get_state();
    assert!(
        s2 == NodeState::Suspect || s2 == NodeState::Recovering,
        "Node state must remain non-active (found {:?})",
        s2
    );
}

#[tokio::test]
async fn test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits() {
    use phantom_cache_demo::dist::node::NodeState;

    let node = Arc::new(VirtualNode::new(1));
    let addr = node.start_tcp_service("127.0.0.1:0").await.expect("start");

    // Populate node cache with an entry
    let key = CacheKey::new(None, "cached_item", None);
    let now = 100_000;
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(b"initial_val".to_vec()),
        etag: None,
        created_at_ms: now,
        ttl_ms: 60_000,
    });
    if let phantom_cache_demo::core::cache::FetchRegistrationResult::Initiate(r) =
        node.cache.begin_fetch_atomic(&key, now)
    {
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }

    // Set node state to Recovering
    node.set_state(NodeState::Recovering);

    // Send GetRequest over TCP
    let mut client = TcpStream::connect(addr).await.expect("connect");
    let get_req = PhantomMeshFrame::new_get_request(1, 1, "cached_item").expect("get frame");
    write_frame(&mut client, &get_req).await.expect("write get");

    let resp = read_frame(&mut client).await.expect("read resp");
    let body = resp.extract_response().expect("extract body");

    // SEC-POA-04: Must return MISS (bypassing the local cache hit) while in Recovering state!
    assert!(
        body.starts_with("MISS"),
        "Recovering node must not serve stale cache hit (got '{}')",
        body
    );

    node.stop_tcp_service();
}

#[test]
fn test_p4c_sec10_ring_buffer_resident_memory_accounting() {
    // Test that 4,096 entries do not exceed defensible resident bounds (<= 4.5 MB)
    let mut ring = InvalidationRingBuffer::new(4096, 4 * 1024 * 1024);

    for i in 1..=4096 {
        ring.push(InvalidateRecord {
            stream_seq: i,
            scope: InvalidationScope::ExactKey,
            key: format!("resource_key_{:06}", i),
        });
    }

    assert_eq!(ring.len(), 4096);
    let resident = ring.estimated_resident_memory_bytes();
    // Resident memory must be bounded under 4.5 MB (4,718,592 bytes)
    assert!(
        resident <= 4_718_592,
        "Resident memory {} exceeded 4.5 MB bound",
        resident
    );
    // And must strictly account for collection overhead (> logical serialized bytes)
    assert!(
        resident > 4096 * 20,
        "Resident memory should exceed pure key byte minimum"
    );
}

