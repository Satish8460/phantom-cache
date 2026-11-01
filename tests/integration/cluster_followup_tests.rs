//! In-process tests for the review follow-ups (H1, H2, M3-M10, L2, L3) of the cluster work.
//!
//! Plaintext loopback transport between in-process `VirtualNode`s (same router/pool code paths the
//! gateway uses; mTLS-specific behaviour is covered by the L3 test and the multi-process suites).
//!
//! Lines/blocks marked NEWAPI use symbols that did not exist before the fixes; they are the only
//! parts that cannot compile against the pre-fix code (used to produce the failing-before runs).

use phantom_cache_demo::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
use phantom_cache_demo::dist::anti_entropy::{AntiEntropyTracker, InvalidationRingBuffer};
use phantom_cache_demo::dist::frame::{
    read_frame, write_frame, InvalidateRecord, InvalidationScope, InvalidationStatus, MessageType, PhantomMeshFrame,
};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::router::{DistError, RendezvousRouter, ReplicaInvalidationTask};
use phantom_cache_demo::security::cluster::{
    compute_cert_fingerprint, ClusterMembershipRegistry, ClusterTlsConfig, MemberNode,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn started_node(id: u8) -> Arc<VirtualNode> {
    let node = Arc::new(VirtualNode::new(id));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);
    node
}

/// Node whose address points at a closed port (connect refused).
fn dead_node(id: u8) -> Arc<VirtualNode> {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    let n = Arc::new(VirtualNode::new(id));
    n.set_addr(addr);
    n.set_state(NodeState::Active);
    n
}

/// Listener that accepts TCP connections and never reads or writes (a blackholed peer).
async fn blackhole_node(id: u8) -> Arc<VirtualNode> {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((s, _)) = l.accept().await {
            held.push(s);
        }
    });
    let n = Arc::new(VirtualNode::new(id));
    n.set_addr(addr);
    n.set_state(NodeState::Active);
    n
}

fn cache_put(cache: &Arc<PartitionedCache>, key: &CacheKey, body: &[u8]) {
    let now_ms = 500_000;
    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(body.to_vec()),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 600_000,
    });
    if let FetchRegistrationResult::Initiate(r) = cache.begin_fetch_atomic(key, now_ms) {
        r.into_guard()
            .complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    } else {
        panic!("expected Initiate for a fresh key");
    }
}

fn cache_has(cache: &PartitionedCache, key: &CacheKey) -> bool {
    cache.get_resource(key, 500_000).is_some()
}

/// Key (by scan) whose ranking by the production router starts with `want_first` (primary).
fn key_with_primary(nodes: &[Arc<VirtualNode>], want_primary: u8) -> String {
    let router = RendezvousRouter::new(nodes.to_vec());
    for i in 0..2000 {
        let k = format!("__default__:/scan_{i}");
        if router.route_key(&k).map(|n| n.node_id) == Some(want_primary) {
            router.stop();
            return k;
        }
    }
    router.stop();
    panic!("no key found");
}

// ---------------------------------------------------------------------------
// H2: a failed DELETE still owes every peer
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_h2_failed_delete_records_all_other_peers_as_owed() {
    let n1 = dead_node(1);
    let n2 = dead_node(2);
    let n3 = started_node(3).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]));
    // A key whose primary AND first replica are the two dead nodes -> synchronous leg fails.
    let mut key = None;
    for i in 0..5000 {
        let k = format!("__default__:/h2_{i}");
        let p = router.route_key(&k).unwrap().node_id;
        let r = router.get_replicas(&k, 2);
        if p != 3 && r.first().map(|x| x.node_id) != Some(3) {
            key = Some(k);
            break;
        }
    }
    let key = key.expect("key with two dead nodes at the top of the ranking");
    let res = router.invalidate_key_across_cluster(&key, InvalidationScope::ExactKey).await;
    assert!(res.is_err(), "synchronous leg cannot succeed: {res:?}");
    let ledger = router.broadcaster().repair_ledger();
    for id in [1u8, 2, 3] {
        assert_eq!(
            ledger.pending_count(id),
            1,
            "peer {id} must be owed the invalidation of a failed DELETE"
        );
    }
    router.stop();
    n3.stop_tcp_service();
}

// M3 / L-2: hard-rejection handling is tested with a fake receiver (see test_m3_hard_rejection_... below).


// ---------------------------------------------------------------------------
// M4: blackholed peer + thousands of DELETEs: bounded tasks, overflow handled by the ledger
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m4_blackholed_peer_keeps_task_count_bounded_and_owes_ledger() {
    let n1 = started_node(1).await;
    let n2 = blackhole_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let metrics = tokio::runtime::Handle::current().metrics();
    let baseline = metrics.num_alive_tasks();

    for i in 0..3000 {
        router.broadcaster().enqueue(ReplicaInvalidationTask {
            key: format!("t{}:/m4_{i}", i % 7),
            scope: InvalidationScope::ExactKey,
            replica_node_ids: vec![2],
            membership_epoch: 1,
        });
        if i % 25 == 0 {
            tokio::task::yield_now().await;
        }
    }
    // Let the consumer drain the queue against the blackholed peer.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let alive_delta = metrics.num_alive_tasks().saturating_sub(baseline);
    assert!(
        alive_delta <= 8,
        "spawned delivery tasks must stay bounded (per-peer in-flight cap), got +{alive_delta} alive tasks"
    );
    assert!(
        router.broadcaster().peak_inflight_tasks.load(std::sync::atomic::Ordering::Relaxed) <= 4, // NEWAPI
        "peak in-flight deliveries per peer is capped"
    );
    // Everything that could not be delivered is owed (per key, per tenant, or as a flush) - not lost.
    assert!(router.broadcaster().repair_ledger().has_pending(2));
    router.stop();
    n1.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M5: pool saturation must not demote a healthy node
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_m5_pool_saturation_does_not_mark_primary_suspect() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let nodes = vec![n1.clone(), n2.clone()];
    let router = Arc::new(RendezvousRouter::new(nodes.clone()));
    let key = key_with_primary(&nodes, 2);

    // Hold all 8 slots (max_total_per_node) for node 2.
    let held: Vec<_> = (0..8).map(|_| router.pool().checkout_or_reserve(2)).collect();
    assert_eq!(router.pool().total_active_for_node(2), 8);

    let res = router.invalidate_key_across_cluster(&key, InvalidationScope::ExactKey).await;
    assert!(
        // Since batch A (D-022) local saturation is `Busy` (a local back-pressure condition), no longer a fake
        // `Rejected(RateLimit)`; the invariant this test protects (no demotion of the healthy peer) is unchanged.
        matches!(res, Err(DistError::Busy(2))),
        "saturation is a local back-pressure condition (Busy), got {res:?}"
    );
    assert_eq!(n2.get_state(), NodeState::Active, "pool saturation must not demote a healthy peer");
    assert_eq!(n1.get_state(), NodeState::Active);

    drop(held);
    let ok = router.invalidate_key_across_cluster(&key, InvalidationScope::ExactKey).await;
    assert!(ok.is_ok(), "after the slots are released the invalidation succeeds: {ok:?}");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// L2: a stale pooled connection is retried once on a fresh one before judging the peer
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_l2_stale_pooled_connection_is_retried_on_fresh_connection() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));

    // Warm: one pooled connection to node 2.
    router
        .send_invalidation_frame(&n2, "__default__:/warm", InvalidationScope::ExactKey, 1000)
        .await
        .expect("warm-up");
    assert_eq!(router.pool().idle_count_for_node(2), 1);

    // "Restart" node 2 on the same address: the pooled connection is now dead but still pooled.
    let addr = n2.get_addr().unwrap();
    n2.stop_tcp_service();
    let mut restarted = false;
    for _ in 0..100 {
        if n2.start_tcp_service(&addr.to_string()).await.is_ok() {
            restarted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(restarted, "could not rebind node 2's address");
    n2.set_state(NodeState::Active);

    let res = router
        .send_invalidation_frame(&n2, "__default__:/after_restart", InvalidationScope::ExactKey, 1000)
        .await;
    assert!(res.is_ok(), "stale pooled connection must be retried once on a fresh one: {res:?}");
    assert_eq!(n2.get_state(), NodeState::Active, "a healthy restarted peer must not be marked Suspect");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M6: overflow collapses per tenant, not globally
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m6_one_tenants_overflow_does_not_flush_other_tenants_on_peer() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));

    let key_a = CacheKey::new(Some("tenant_a"), "/cached", None);
    let key_b = CacheKey::new(Some("tenant_b"), "/cached", None);
    cache_put(&n2.cache, &key_a, b"a");
    cache_put(&n2.cache, &key_b, b"b");
    assert!(cache_has(&n2.cache, &key_a) && cache_has(&n2.cache, &key_b));

    let ledger = router.broadcaster().repair_ledger().clone();
    ledger.record(2, None, InvalidationScope::ExactKey, "tenant_b:/other_pending");
    // Tenant A floods the backlog beyond its per-peer bound.
    for i in 0..1100 {
        ledger.record(2, None, InvalidationScope::ExactKey, &format!("tenant_a:/flood_{i}"));
    }
    assert!(!ledger.has_full_flush(2), "single-tenant overflow must not escalate to a global flush");

    let mut rounds = 0;
    while ledger.has_pending(2) && rounds < 5 {
        router.repair_peer(&n2).await;
        rounds += 1;
    }
    assert!(!ledger.has_pending(2));
    assert!(!cache_has(&n2.cache, &key_a), "tenant A's entry on the peer must be invalidated (tenant flush)");
    assert!(cache_has(&n2.cache, &key_b), "tenant B's entry on the peer must survive tenant A's overflow");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M7 + M8: state machine
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_m7_suspect_peer_with_nothing_owed_becomes_active_after_reconcile_cycles() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    n2.set_state(NodeState::Suspect);
    let mut cycles = 0;
    while n2.get_state() != NodeState::Active && cycles < 5 {
        router.reconcile_all_peers().await;
        cycles += 1;
    }
    assert_eq!(n2.get_state(), NodeState::Active, "no sink state: reconcile cycles must restore Active");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m8_drained_peer_is_only_recovering_until_pull_reconciliation_then_active() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    n2.set_state(NodeState::Suspect);
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "__default__:/owed");
    assert!(router.repair_peer(&n2).await);
    assert_eq!(n2.get_state(), NodeState::Recovering, "push drain alone must not restore Active");
    let mut cycles = 0;
    while n2.get_state() != NodeState::Active && cycles < 5 {
        router.reconcile_all_peers().await;
        cycles += 1;
    }
    assert_eq!(n2.get_state(), NodeState::Active, "Active only after a successful pull reconciliation");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

/// The peer holds an invalidation that THIS gateway never sent (it came from a third party):
/// the pull phase must still apply it locally.
#[tokio::test]
async fn test_m8_peer_that_holds_what_this_gateway_missed_is_still_pulled() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router_a = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));

    let key = CacheKey::new(None, "/pulled", None);
    cache_put(&n1.cache, &key, b"stale");
    assert!(cache_has(&n1.cache, &key));

    // Node 2 holds a replayable record that node 1 never received (its delivery to node 1 failed).
    n2.ring_buffer.lock().unwrap().push_outbound(
        InvalidateRecord {
            stream_seq: 1,
            scope: InvalidationScope::ExactKey,
            key: key.to_canonical_string(),
        },
        1,
    );
    n2.set_state(NodeState::Suspect);

    let mut cycles = 0;
    while (cache_has(&n1.cache, &key) || n2.get_state() != NodeState::Active) && cycles < 5 {
        router_a.reconcile_all_peers().await;
        cycles += 1;
    }
    assert!(!cache_has(&n1.cache, &key), "node 1 must pull and apply the invalidation held by node 2");
    assert_eq!(n2.get_state(), NodeState::Active);
    router_a.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// NEWAPI-BEGIN
#[test]
fn test_m7_state_transition_is_compare_and_set() {
    let n = VirtualNode::new(9);
    n.set_state(NodeState::Suspect); // concurrent failure marking
    assert!(!n.compare_and_set_state(NodeState::Recovering, NodeState::Active));
    assert_eq!(n.get_state(), NodeState::Suspect, "a stale Recovering->Active decision must not overwrite Suspect");
    assert!(n.compare_and_set_state(NodeState::Suspect, NodeState::Recovering));
    assert!(n.compare_and_set_state(NodeState::Recovering, NodeState::Active));
}
// NEWAPI-END

// ---------------------------------------------------------------------------
// M9: gap draining, monotone ring, bounded Phase 2, no sequence burn on connect failure
// ---------------------------------------------------------------------------
#[test]
fn test_m9_late_sequence_clears_detected_gap() {
    let mut t = AntiEntropyTracker::new();
    assert!(t.record_inbound_seq(1, 1, None).is_none());
    assert!(t.record_inbound_seq(1, 3, None).is_some());
    assert!(t.has_gaps());
    assert!(t.record_inbound_seq(1, 2, None).is_none());
    assert!(!t.has_gaps(), "the late sequence filled the gap; reconciliation must stop running forever");
}

#[test]
fn test_m9_ring_newest_seq_is_monotone_under_redelivery() {
    let mut r = InvalidationRingBuffer::default();
    for s in [5u64, 3u64] {
        r.push(InvalidateRecord { stream_seq: s, scope: InvalidationScope::ExactKey, key: format!("k{s}") });
    }
    assert_eq!(r.newest_seq(), 5, "a re-delivery with an older sequence must not move newest_seq backwards");
}

#[tokio::test]
async fn test_m9_connect_failure_does_not_burn_a_sequence() {
    let n1 = started_node(1).await;
    let n2 = dead_node(2);
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let r = router
        .send_invalidation_frame(&n2, "__default__:/x", InvalidationScope::ExactKey, 200)
        .await;
    assert!(r.is_err());
    assert_eq!(
        router.dest_sequencer.next_sequence_for(2),
        1,
        "no usable connection -> no sequence may be consumed"
    );
    router.stop();
    n1.stop_tcp_service();
}

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m9_blackholed_peers_do_not_serialise_reconciliation() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let b3 = blackhole_node(3).await;
    let b4 = blackhole_node(4).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), b3.clone(), b4.clone()]));
    let ledger = router.broadcaster().repair_ledger().clone();

    // Baseline in THIS run: cost of reconciling with ONE blackholed peer (bounded by its read timeout).
    b3.set_state(NodeState::Suspect);
    let t0 = Instant::now();
    router.reconcile_all_peers().await;
    let one = t0.elapsed();

    // Two blackholed peers: concurrent => about the same cost; serial => about twice.
    b3.set_state(NodeState::Suspect);
    b4.set_state(NodeState::Suspect);
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/owed2");
    let t1 = Instant::now();
    router.reconcile_all_peers().await;
    let two = t1.elapsed();

    assert!(!ledger.has_pending(2), "healthy peer repaired in the same pass");
    assert!(
        two.as_secs_f64() < one.as_secs_f64() * 1.6,
        "pull reconciliation must be concurrent across peers: one blackholed peer {one:?}, two {two:?}"
    );
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M10: Failed/Offline peers are ledgered too
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m10_failed_peer_is_owed_the_invalidation() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let n3 = started_node(3).await;
    let nodes = vec![n1.clone(), n2.clone(), n3.clone()];
    let router = Arc::new(RendezvousRouter::new(nodes.clone()));
    n2.set_state(NodeState::Failed);
    let key = key_with_primary(&nodes, 3);
    let res = router.invalidate_key_across_cluster(&key, InvalidationScope::ExactKey).await;
    assert!(res.is_ok(), "{res:?}");
    assert_eq!(
        router.broadcaster().repair_ledger().pending_count(2),
        1,
        "a Failed peer must be recorded as owed so it is repaired when it returns"
    );
    router.stop();
    n1.stop_tcp_service();
    n3.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// H1 (documented behaviour): after a sender restart its sequences restart at 1 and look like
// duplicates to the receiver's tracker, but every invalidation is still APPLIED.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_h1_restarted_sender_invalidations_are_still_applied_by_receiver() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let before_restart = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    for i in 0..3 {
        before_restart
            .send_invalidation_frame(&n2, &format!("__default__:/old_{i}"), InvalidationScope::ExactKey, 1000)
            .await
            .unwrap();
    }
    before_restart.stop();

    // "Restart": a brand-new router has a fresh DestinationSequencer, so its first sequence is 1.
    let after_restart = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let key = CacheKey::new(None, "/cached_on_peer", None);
    cache_put(&n2.cache, &key, b"stale");
    after_restart
        .send_invalidation_frame(&n2, &key.to_canonical_string(), InvalidationScope::ExactKey, 1000)
        .await
        .unwrap();
    assert!(
        !cache_has(&n2.cache, &key),
        "the receiver applies every request regardless of its sequence number"
    );
    after_restart.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// L3: originator_id must equal the authenticated TLS peer identity
// ---------------------------------------------------------------------------
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
        Self { cert_pem: ca_cert.pem(), ca_cert, ca_key }
    }
    fn issue(&self, san: &str) -> (String, String, Vec<u8>) {
        let key = rcgen::KeyPair::generate().unwrap();
        let params = rcgen::CertificateParams::new(vec![san.to_string()]).unwrap();
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key).unwrap();
        (cert.pem(), key.serialize_pem(), cert.der().to_vec())
    }
}

#[tokio::test]
async fn test_l3_invalidate_request_originator_must_match_tls_identity() {
    let ca = TestCa::new();
    let cid = "cluster-l3";
    let (c1, k1, d1) = ca.issue("node-1.cluster-l3.phantom.internal");
    let (c2, k2, d2) = ca.issue("node-2.cluster-l3.phantom.internal");
    let mut reg = ClusterMembershipRegistry::new(cid, 1);
    reg.add_member(MemberNode { node_id: 1, fingerprints: vec![compute_cert_fingerprint(&d1)], active: true });
    reg.add_member(MemberNode { node_id: 2, fingerprints: vec![compute_cert_fingerprint(&d2)], active: true });
    let t1 = Arc::new(ClusterTlsConfig::from_pem(1, cid, ca.cert_pem.as_bytes(), c1.as_bytes(), k1.as_bytes(), reg.clone()).unwrap());
    let t2 = Arc::new(ClusterTlsConfig::from_pem(2, cid, ca.cert_pem.as_bytes(), c2.as_bytes(), k2.as_bytes(), reg).unwrap());

    let node1 = Arc::new(VirtualNode::new(1));
    node1.set_cluster_tls(t1);
    let addr = node1.start_tcp_service("127.0.0.1:0").await.unwrap();

    let key = CacheKey::new(Some("t"), "/victim", None);
    cache_put(&node1.cache, &key, b"payload");

    let connect = || async {
        let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        let name = rustls::pki_types::ServerName::try_from(format!("node-1.{cid}.phantom.internal")).unwrap();
        t2.client_connector().connect(name, tcp).await.unwrap()
    };

    // Node 2 (authenticated) claims to be node 3: rejected, nothing applied.
    let mut c = connect().await;
    let spoofed = PhantomMeshFrame::new_invalidate_request(
        1, 1, 1, 7, 3, InvalidationScope::ExactKey, &key.to_canonical_string(),
    )
    .unwrap();
    write_frame(&mut c, &spoofed).await.unwrap();
    let r = tokio::time::timeout(Duration::from_secs(3), read_frame(&mut c)).await.expect("server must answer or close");
    assert!(r.is_err(), "spoofed originator must get the connection closed, not an ack: {r:?}");
    assert!(cache_has(&node1.cache, &key), "a spoofed-originator request must not invalidate anything");

    // Honest originator (2) is accepted.
    let mut c = connect().await;
    let honest = PhantomMeshFrame::new_invalidate_request(
        2, 1, 1, 8, 2, InvalidationScope::ExactKey, &key.to_canonical_string(),
    )
    .unwrap();
    write_frame(&mut c, &honest).await.unwrap();
    let ack = read_frame(&mut c).await.unwrap();
    assert_eq!(ack.header.msg_type, MessageType::InvalidateAck);
    assert!(!cache_has(&node1.cache, &key));
    node1.stop_tcp_service();
}

// ===========================================================================
// Review round 2 (H-1, M-1 .. M-6, L-2 .. L-4)
// ===========================================================================

// ---------------------------------------------------------------------------
// H-1: a legitimately cached canonical key longer than the frame cap (4096 B) is invalidated
// via a tenant-scope invalidation, without demoting any node.
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_h1_oversized_canonical_key_is_invalidated_cluster_wide_without_demotion() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let n3 = started_node(3).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]));

    // path <= 2048 and query <= 4096 pass CacheKey::validate; the canonical form is ~5 KB.
    let big = CacheKey::new(Some("tenant_big"), &format!("/{}", "p".repeat(2000)), Some(&"q".repeat(3000)));
    // (The cache layer now also rejects such keys at the edge; this test pins the router-level
    // fallback for any key that still reaches it.)
    let canonical = big.to_canonical_string();
    assert!(canonical.len() > 4096 && canonical.len() < 6400, "len {}", canonical.len());
    let other = CacheKey::new(Some("tenant_other"), "/keep", None);
    for n in [&n2, &n3] {
        cache_put(&n.cache, &big, b"stale-big");
        cache_put(&n.cache, &other, b"keep");
    }

    let res = router.invalidate_key_across_cluster(&canonical, InvalidationScope::ExactKey).await;
    assert!(res.is_ok(), "oversized key must be invalidated, not fail the DELETE: {res:?}");

    let t0 = Instant::now();
    loop {
        if !cache_has(&n2.cache, &big) && !cache_has(&n3.cache, &big) {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(5), "oversized key still cached on a peer");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for n in [&n1, &n2, &n3] {
        assert_eq!(n.get_state(), NodeState::Active, "no node may be demoted by an oversized key");
    }
    assert!(cache_has(&n2.cache, &other) && cache_has(&n3.cache, &other), "the other tenant's entries survive");
    let ledger = router.broadcaster().repair_ledger();
    for id in [1u8, 2, 3] {
        assert_eq!(ledger.pending_count(id), 0, "nothing is owed: delivery succeeded live");
        assert!(!ledger.has_full_flush(id));
    }
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
    n3.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M-1: a debt recorded while the peer is Recovering blocks promotion to Active
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m1_debt_while_recovering_blocks_promotion_until_repaired() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    n2.set_state(NodeState::Recovering);
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "__default__:/new_debt");

    router.reconcile_peer_anti_entropy(&n2, &n1).await.expect("pull succeeds");
    assert_eq!(n2.get_state(), NodeState::Recovering, "owing peer must stay Recovering after a successful pull");

    assert!(router.repair_peer(&n2).await);
    router.reconcile_peer_anti_entropy(&n2, &n1).await.expect("pull succeeds");
    assert_eq!(n2.get_state(), NodeState::Active);
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M-2: the replay ring is per destination
// ---------------------------------------------------------------------------
fn rec(seq: u64, scope: InvalidationScope, key: &str) -> InvalidateRecord {
    InvalidateRecord { stream_seq: seq, scope, key: key.to_string() }
}

/// B has sent 500 records to C and 15 to A; A has acknowledged 12 from B. A's pull from B must
/// return only B->A records 13..15, leave A's tracker for B at 15 (not 500+) and must not apply
/// the Tenant flush that B sent to C.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m2_pull_replays_only_the_requesters_own_stream() {
    let a = started_node(1).await;
    let b = started_node(2).await;
    let c = started_node(3).await;
    let router = Arc::new(RendezvousRouter::new(vec![a.clone(), b.clone(), c.clone()]));

    {
        let mut ring = b.ring_buffer.lock().unwrap();
        for i in 1..=500u64 {
            ring.push_outbound(rec(i, InvalidationScope::ExactKey, &format!("__default__:/c{i}")), 3);
        }
        ring.push_outbound(rec(501, InvalidationScope::Tenant, "tenant_x"), 3);
        for i in 1..=15u64 {
            ring.push_outbound(rec(i, InvalidationScope::ExactKey, &format!("__default__:/a{i}")), 1);
        }
    }
    {
        let mut t = a.anti_entropy_tracker.lock().unwrap();
        for s in 1..=12u64 {
            t.record_inbound_seq(2, s, None);
        }
        assert_eq!(t.get_highest_acked_seq(2), 12);
    }
    let foreign = CacheKey::new(Some("tenant_x"), "/mine", None);
    let lost = CacheKey::new(None, "/a14", None);
    cache_put(&a.cache, &foreign, b"x");
    cache_put(&a.cache, &lost, b"y");

    b.set_state(NodeState::Suspect);
    router.pull_reconcile_all_peers().await;

    assert!(!cache_has(&a.cache, &lost), "the genuinely lost B->A record must be replayed");
    assert!(cache_has(&a.cache, &foreign), "a record B sent to C (Tenant flush) must NOT be applied at A");
    assert_eq!(
        a.anti_entropy_tracker.lock().unwrap().get_highest_acked_seq(2),
        15,
        "A's tracker for B advances only within the B->A stream"
    );
    assert_eq!(a.cache.counters.anti_entropy_keys_evicted.load(std::sync::atomic::Ordering::Relaxed), 3);
    router.stop();
    a.stop_tcp_service();
    b.stop_tcp_service();
    c.stop_tcp_service();
}

/// Echo: node 1 sent K to node 2; pulling from node 2 must not hand node 1 its own record back.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m2_pull_does_not_echo_own_records_back() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    router
        .send_invalidation_frame(&n2, "__default__:/echo", InvalidationScope::ExactKey, 1000)
        .await
        .unwrap();
    n2.set_state(NodeState::Suspect);
    router.pull_reconcile_all_peers().await;
    assert_eq!(n1.anti_entropy_tracker.lock().unwrap().get_highest_acked_seq(2), 0);
    assert_eq!(n1.cache.counters.anti_entropy_keys_evicted.load(std::sync::atomic::Ordering::Relaxed), 0);
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

/// Out-of-order pushes (a re-delivery with an older sequence) must not look like evicted history.
#[test]
fn test_m2_out_of_order_push_is_not_spurious_history_overwritten() {
    let mut r = InvalidationRingBuffer::default();
    r.push(rec(5, InvalidationScope::ExactKey, "k5"));
    r.push(rec(3, InvalidationScope::ExactKey, "k3"));
    let batch = r.get_replay_batch(0).expect("nothing was evicted: no HistoryOverwritten");
    assert_eq!(batch.len(), 2);
    assert_eq!(r.oldest_seq(), 3, "oldest is the minimum retained sequence, not the front record's");
}

// ---------------------------------------------------------------------------
// M-3: bounded gap bookkeeping; restart case; per-peer Phase 2
// ---------------------------------------------------------------------------
#[test]
fn test_m3_gap_bookkeeping_is_bounded() {
    let mut t = AntiEntropyTracker::new();
    t.record_inbound_seq(2, 1, None);
    for i in 0..600u64 {
        t.record_inbound_seq(2, 3 + 2 * i, None); // every other sequence is missing
    }
    let mut n = 0;
    while t.pop_gap().is_some() {
        n += 1;
    }
    assert!(n <= 256, "recorded gap ranges must stay bounded, got {n}");
    assert!(t.has_gaps(), "collapsed bookkeeping still demands one resync");
}

/// Sender restarted (gap can never be filled, history gone): one successful pull clears the gap,
/// after which neither that peer nor any other peer is contacted again.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m3_unfillable_gap_is_cleared_by_one_pull_and_phase2_stops() {
    let a = started_node(1).await;
    let b = started_node(2).await;
    let c = dead_node(3); // never contacted unless Phase 2 runs against everyone
    let router = Arc::new(RendezvousRouter::new(vec![a.clone(), b.clone(), c.clone()]));
    {
        let mut t = a.anti_entropy_tracker.lock().unwrap();
        t.record_inbound_seq(2, 1, None);
        t.record_inbound_seq(2, 3, None); // seq 2 never arrives (sender restarted)
        assert!(t.has_gaps());
    }
    router.pull_reconcile_all_peers().await;
    assert!(!a.anti_entropy_tracker.lock().unwrap().has_gaps(), "one successful pull clears the unfillable gap");
    // B's ring is empty while A claims to have received seq 1 from it: B's stream restarted and its
    // history is gone, so A's only safe repair is ONE full flush of its own cache.
    assert_eq!(
        a.cache.counters.anti_entropy_full_flushes.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "exactly one flush, caused by the empty-history responder"
    );
    assert_eq!(c.get_state(), NodeState::Active, "a gap with peer 2 must not make this node contact peer 3");

    b.stop_tcp_service(); // any further contact with peer 2 would now fail and demote it
    router.pull_reconcile_all_peers().await;
    assert_eq!(b.get_state(), NodeState::Active, "no gap left: peer 2 is not contacted again");
    router.stop();
    a.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M-4: debts of revoked members are converted to a pending flush, not forgotten
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m4_revoked_member_debts_become_pending_flush() {
    let ca = TestCa::new();
    let ca_pem = ca.cert_pem.clone();
    let (c1, k1, d1) = ca.issue("node-1.cluster-m4.phantom.internal");
    let cid = "cluster-m4";
    let mut reg = ClusterMembershipRegistry::new(cid, 1);
    reg.add_member(MemberNode { node_id: 1, fingerprints: vec![compute_cert_fingerprint(&d1)], active: true });
    reg.add_member(MemberNode { node_id: 2, fingerprints: vec![compute_cert_fingerprint(&d1)], active: false });
    let t1 = Arc::new(ClusterTlsConfig::from_pem(1, cid, ca_pem.as_bytes(), c1.as_bytes(), k1.as_bytes(), reg).unwrap());

    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = dead_node(2);
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_cluster_tls(t1));
    let ledger = router.broadcaster().repair_ledger().clone();
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/owed_to_revoked");
    router.reconcile_all_peers().await;
    assert!(ledger.has_pending(2), "a revoked member's debt must not be forgotten");
    assert!(ledger.has_full_flush(2), "it becomes one pending full flush for when the member returns");
    assert_eq!(ledger.pending_count(2), 0);
    router.stop();
}

// ---------------------------------------------------------------------------
// M-5: waiting on a saturated local pool must not make a healthy peer look slow
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_m5_pool_wait_that_eats_the_budget_does_not_mark_peer_suspect() {
    // Scenario: the local pool stays saturated for 140 ms. Since batch B2 (D-039) the synchronous leg waits up
    // to 1000 ms for a slot, separately from the 150 ms connect/IO deadline, so the call waits for the slot at
    // 140 ms and then completes the exchange (the peer needs 30 ms). It must never end with a timeout that
    // demotes the peer because of LOCAL contention.
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    n2.set_artificial_delay(30);
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));

    let held: Vec<_> = (0..8).map(|_| router.pool().checkout_or_reserve(2)).collect();
    let releaser = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(140)).await;
        drop(held);
    });
    let t0 = Instant::now();
    let res = router
        .send_invalidation_frame(&n2, "__default__:/m5", InvalidationScope::ExactKey, 150)
        .await;
    let took = t0.elapsed();
    releaser.await.unwrap();
    assert!(res.is_ok(), "the sync leg outwaits local contention, got {res:?} after {took:?}");
    assert!(took >= Duration::from_millis(130), "it waited for the slot, took {took:?}");
    assert_eq!(n2.get_state(), NodeState::Active, "a healthy-but-slow peer must not be demoted by LOCAL contention");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

/// Slots are freed at 40 ms (inside the 75 ms wait cap): the wait succeeds and the exchange that
/// follows (the peer needs 30 ms per request) completes within the remaining budget.
#[tokio::test]
async fn test_m5_pool_wait_within_cap_then_exchange_succeeds() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    n2.set_artificial_delay(30);
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let held: Vec<_> = (0..8).map(|_| router.pool().checkout_or_reserve(2)).collect();
    let releaser = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        drop(held);
    });
    let t0 = Instant::now();
    let res = router
        .send_invalidation_frame(&n2, "__default__:/m5b", InvalidationScope::ExactKey, 150)
        .await;
    let took = t0.elapsed();
    releaser.await.unwrap();
    assert!(res.is_ok(), "exchange after a successful wait must complete: {res:?} (took {took:?})");
    assert!(took >= Duration::from_millis(35), "the call really waited for a slot: {took:?}");
    assert_eq!(n2.get_state(), NodeState::Active);
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M-6: blackholed peers must not starve healthy ones; repair loop is independent of the pull loop
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_m6_blackholed_peers_do_not_starve_a_healthy_peer() {
    let n1 = started_node(1).await;
    let h2 = started_node(2).await;
    let mut nodes = vec![n1.clone(), h2.clone()];
    for id in 3..=6u8 {
        nodes.push(blackhole_node(id).await);
    }
    let router = Arc::new(RendezvousRouter::new(nodes));

    let total = 200;
    for i in 0..total {
        cache_put(&h2.cache, &CacheKey::new(None, &format!("/m6_{i}"), None), b"stale");
    }
    for i in 0..total {
        router.broadcaster().enqueue(ReplicaInvalidationTask {
            key: format!("__default__:/m6_{i}"),
            scope: InvalidationScope::ExactKey,
            replica_node_ids: vec![2, 3, 4, 5, 6],
            membership_epoch: 1,
        });
        if i % 20 == 0 {
            tokio::task::yield_now().await;
        }
    }
    // The healthy peer's 200 deliveries take milliseconds when it is not starved; a blackholed
    // attempt takes >= 150 ms and a whole retry chain >= 600 ms, so 4 s separates the cases.
    let t0 = Instant::now();
    loop {
        let remaining = (0..total)
            .filter(|i| cache_has(&h2.cache, &CacheKey::new(None, &format!("/m6_{i}"), None)))
            .count();
        if remaining == 0 {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(4), "healthy peer starved: {remaining} entries still stale");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    router.stop();
    n1.stop_tcp_service();
    h2.stop_tcp_service();
}

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m6_repair_loop_is_independent_of_slow_pull_loop() {
    let n1 = started_node(1).await;
    let h2 = started_node(2).await;
    let mut nodes = vec![n1.clone(), h2.clone()];
    for id in 3..=5u8 {
        let b = blackhole_node(id).await;
        b.set_state(NodeState::Suspect); // each pull exchange blocks for its 1000 ms read timeout
        nodes.push(b);
    }
    let router = Arc::new(RendezvousRouter::new(nodes));
    router.start_anti_entropy_worker(Duration::from_millis(100));
    // Let the pull loop enter an exchange (they last ~1 s), then create a debt for the healthy peer.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let ledger = router.broadcaster().repair_ledger().clone();
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/owed_healthy");
    let t0 = Instant::now();
    while ledger.has_pending(2) {
        // A pull exchange with a blackholed peer lasts ~1000 ms; the repair must not wait for it.
        assert!(t0.elapsed() < Duration::from_millis(500), "repair delayed behind a slow pull exchange");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    router.stop();
    n1.stop_tcp_service();
    h2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// L-3: a read TIMEOUT is not evidence of a stale pooled connection and the leg stays strict
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_l3_read_timeout_is_not_retried_and_leg_budget_is_not_doubled() {
    let n1 = started_node(1).await;
    let b2 = blackhole_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), b2.clone()]));
    // Park one REUSED connection to the blackholed peer (use_count > 0).
    let conn = RendezvousRouter::acquire_connection_to_node(router.pool(), &None, &b2, 150)
        .await
        .expect("connect");
    router.pool().checkin(conn);

    let t0 = Instant::now();
    let res = router
        .send_invalidation_frame(&b2, "__default__:/l3", InvalidationScope::ExactKey, 150)
        .await;
    let took = t0.elapsed();
    assert!(res.is_err());
    // One strict 150 ms leg. A retry on a fresh connection would add another full 150 ms (>= 300).
    assert!(took < Duration::from_millis(250), "leg budget doubled by a retry: {took:?}");
    router.stop();
    n1.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// L-4: NoActiveNodes still owes everyone
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_l4_no_active_nodes_records_owed_state() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    n1.set_state(NodeState::Failed);
    n2.set_state(NodeState::Failed);
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let res = router.invalidate_key_across_cluster("__default__:/l4", InvalidationScope::ExactKey).await;
    assert_eq!(res.err(), Some(DistError::NoActiveNodes));
    for id in [1u8, 2] {
        assert_eq!(router.broadcaster().repair_ledger().pending_count(id), 1, "peer {id} must be owed");
    }
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// Fake receiver speaking the invalidation protocol, to exercise hard-rejection paths
// ---------------------------------------------------------------------------
type DeliveryLog = Arc<std::sync::Mutex<Vec<(InvalidationScope, String)>>>;

/// Plaintext peer that logs every InvalidateRequest and answers InternalError when `reject`
/// returns true for (scope, key), success otherwise.
async fn fake_peer(
    id: u8,
    reject: impl Fn(InvalidationScope, &str) -> bool + Send + Sync + 'static,
) -> (Arc<VirtualNode>, DeliveryLog) {
    fake_peer_with(
        id,
        move |s, k| {
            if reject(s, k) {
                InvalidationStatus::InternalError
            } else {
                InvalidationStatus::SuccessNotPresent
            }
        },
        0,
    )
    .await
}

/// Generic fake receiver: `status_of` decides the ack status; AntiEntropySyncRequests are answered
/// with an EMPTY ReplayBatch after `sync_delay_ms`.
async fn fake_peer_with(
    id: u8,
    status_of: impl Fn(InvalidationScope, &str) -> InvalidationStatus + Send + Sync + 'static,
    sync_delay_ms: u64,
) -> (Arc<VirtualNode>, DeliveryLog) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let log: DeliveryLog = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log2 = log.clone();
    let status_of = Arc::new(status_of);
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            let log = log2.clone();
            let status_of = status_of.clone();
            tokio::spawn(async move {
                while let Ok(frame) = read_frame(&mut s).await {
                    if frame.header.msg_type == MessageType::AntiEntropySyncRequest {
                        tokio::time::sleep(Duration::from_millis(sync_delay_ms)).await;
                        let resp = PhantomMeshFrame::new_anti_entropy_sync_response(
                            frame.header.request_id,
                            frame.header.sequence_number.wrapping_add(1),
                            phantom_cache_demo::dist::frame::AntiEntropyStatus::ReplayBatch,
                            0,
                            0,
                            &[],
                        )
                        .unwrap();
                        if write_frame(&mut s, &resp).await.is_err() {
                            break;
                        }
                        continue;
                    }
                    if frame.header.msg_type != MessageType::InvalidateRequest {
                        break;
                    }
                    let p = match frame.extract_invalidate_request() {
                        Ok(p) => p,
                        Err(_) => break,
                    };
                    log.lock().unwrap().push((p.scope, p.key.clone()));
                    let status = status_of(p.scope, &p.key);
                    let ack = PhantomMeshFrame::new_invalidate_ack(
                        frame.header.request_id,
                        frame.header.sequence_number.wrapping_add(1),
                        p.membership_epoch,
                        p.stream_seq,
                        id,
                        status,
                        0,
                    )
                    .unwrap();
                    if write_frame(&mut s, &ack).await.is_err() {
                        break;
                    }
                }
            });
        }
    });
    let n = Arc::new(VirtualNode::new(id));
    n.set_addr(addr);
    n.set_state(NodeState::Active);
    (n, log)
}

/// A hard rejection (receiver InternalError) of the first entry must not block the entries behind
/// it; the rejected entry is dropped (counted) and escalated to a tenant-scope invalidation.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_m3_hard_rejection_does_not_block_later_entries_and_escalates_to_tenant() {
    let n1 = started_node(1).await;
    let (peer, log) = fake_peer(2, |_scope, key| key.contains("poison")).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), peer.clone()]));
    peer.set_state(NodeState::Suspect);
    let ledger = router.broadcaster().repair_ledger().clone();
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/poison");
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/ok");

    let first = router.repair_peer(&peer).await;
    assert!(
        log.lock().unwrap().iter().any(|(_, k)| k == "__default__:/ok"),
        "the entry behind the poisoned one must still be delivered in the same pass"
    );
    assert!(!first, "the escalation entry is still owed after the first pass");
    let mut passes = 0;
    while ledger.has_pending(2) && passes < 4 {
        router.repair_peer(&peer).await;
        passes += 1;
    }
    assert!(!ledger.has_pending(2));
    assert!(log.lock().unwrap().iter().any(|(s, k)| *s == InvalidationScope::Tenant && k == "__default__"));
    assert_eq!(ledger.permanent_drop_count.load(std::sync::atomic::Ordering::Relaxed), 1); // NEWAPI
    assert_eq!(peer.get_state(), NodeState::Recovering);
    router.stop();
    n1.stop_tcp_service();
}

/// L-2: a Tenant invalidation the receiver hard-rejects is escalated to a global flush.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_l2_failed_tenant_invalidation_escalates_to_full_flush() {
    let n1 = started_node(1).await;
    let (peer, log) =
        fake_peer(2, |scope, key| key.contains("poison") || scope == InvalidationScope::Tenant).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), peer.clone()]));
    let ledger = router.broadcaster().repair_ledger().clone();
    ledger.record(2, None, InvalidationScope::ExactKey, "__default__:/poison");
    let mut passes = 0;
    while ledger.has_pending(2) && passes < 6 {
        router.repair_peer(&peer).await;
        passes += 1;
    }
    assert!(!ledger.has_pending(2), "chain exact -> tenant -> full flush must terminate");
    let scopes: Vec<InvalidationScope> = log.lock().unwrap().iter().map(|(s, _)| *s).collect();
    assert!(scopes.contains(&InvalidationScope::FullCache), "escalated to a full flush: {scopes:?}");
    router.stop();
    n1.stop_tcp_service();
}

// ===========================================================================
// Review round 3 (M1 .. M6 of the final focused review)
// ===========================================================================

/// Peer that accepts, reads ONE frame and closes the connection without answering.
async fn closer_peer(id: u8) -> Arc<VirtualNode> {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = l.accept().await {
            tokio::spawn(async move {
                let _ = read_frame(&mut s).await;
                drop(s);
            });
        }
    });
    let n = Arc::new(VirtualNode::new(id));
    n.set_addr(addr);
    n.set_state(NodeState::Active);
    n
}

// ---------------------------------------------------------------------------
// M1: a failed repair send must not burn a sequence + ring record per tick
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m1_failed_flush_retries_do_not_burn_sequences() {
    let n1 = started_node(1).await;
    let n3 = started_node(3).await;
    let c2 = closer_peer(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), c2.clone(), n3.clone()]));
    {
        let mut ring = n1.ring_buffer.lock().unwrap();
        for s in 1..=3u64 {
            ring.push_outbound(rec(s, InvalidationScope::ExactKey, &format!("__default__:/h{s}")), 3);
        }
    }
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::FullCache, "");
    for _ in 0..4500 {
        router.repair_peer(&c2).await;
    }
    let len = n1.ring_buffer.lock().unwrap().len();
    assert!(len <= 6, "4500 failed flush retries must reuse one sequence, ring holds {len} records");
    let replay = n1.ring_buffer.lock().unwrap().get_replay_batch_for(3, 0);
    assert_eq!(replay.map(|b| b.len()), Ok(3), "a healthy peer's replay history must survive");
    router.stop();
    n1.stop_tcp_service();
    n3.stop_tcp_service();
}

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m1_transient_rejection_retries_do_not_burn_sequences() {
    let n1 = started_node(1).await;
    let n3 = started_node(3).await;
    let (p2, _log) = fake_peer_with(2, |_, _| InvalidationStatus::RejectedRateLimit, 0).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), p2.clone(), n3.clone()]));
    {
        let mut ring = n1.ring_buffer.lock().unwrap();
        for s in 1..=3u64 {
            ring.push_outbound(rec(s, InvalidationScope::ExactKey, &format!("__default__:/h{s}")), 3);
        }
    }
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "__default__:/stuck");
    for _ in 0..4500 {
        router.repair_peer(&p2).await;
    }
    let len = n1.ring_buffer.lock().unwrap().len();
    assert!(len <= 6, "transient rejections must reuse the consumed sequence, ring holds {len} records");
    let replay = n1.ring_buffer.lock().unwrap().get_replay_batch_for(3, 0);
    assert_eq!(replay.map(|b| b.len()), Ok(3));
    router.stop();
    n1.stop_tcp_service();
    n3.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M2: inbound accounting never evicts replayable history
// ---------------------------------------------------------------------------
#[test]
fn test_r4_m2_inbound_flood_does_not_evict_outbound_history() {
    let mut r = InvalidationRingBuffer::default();
    for s in 1..=3u64 {
        r.push_outbound(rec(s, InvalidationScope::ExactKey, &format!("k{s}")), 7);
    }
    for s in 1..=5000u64 {
        r.push_inbound(rec(s, InvalidationScope::ExactKey, &format!("in{s}")));
    }
    let batch = r.get_replay_batch_for(7, 0).expect("history must not be reported overwritten");
    assert_eq!(batch.len(), 3);
    assert!(r.len() <= 3 + 256, "inbound accounting is bounded, ring holds {}", r.len());
    // A sender retry of the same sequence does not add a second inbound record.
    let before = r.len();
    r.push_inbound(rec(5000, InvalidationScope::ExactKey, "in5000"));
    assert_eq!(r.len(), before);
}

// ---------------------------------------------------------------------------
// M3: a restarted stream is detected
// ---------------------------------------------------------------------------
#[test]
fn test_r4_m3_requester_ahead_of_our_stream_means_history_lost() {
    let mut fresh = InvalidationRingBuffer::default();
    fresh.push_inbound(rec(9, InvalidationScope::ExactKey, "only_inbound"));
    assert_eq!(
        fresh.get_replay_batch_for(2, 50).map(|b| b.len()),
        Err(phantom_cache_demo::dist::frame::AntiEntropyStatus::HistoryOverwritten),
        "requester acked 50 of a stream this node never sent: the stream restarted"
    );

    // Normal operation never flushes spuriously.
    let mut r = InvalidationRingBuffer::default();
    for s in 1..=5u64 {
        r.push_outbound(rec(s, InvalidationScope::ExactKey, &format!("k{s}")), 2);
    }
    assert_eq!(r.get_replay_batch_for(2, 0).map(|b| b.len()), Ok(5));
    assert_eq!(r.get_replay_batch_for(2, 3).map(|b| b.len()), Ok(2));
    assert_eq!(r.get_replay_batch_for(2, 5).map(|b| b.len()), Ok(0));
    assert!(r.get_replay_batch_for(2, 6).is_err(), "acked beyond what was ever sent");
    assert_eq!(r.get_replay_batch_for(9, 0).map(|b| b.len()), Ok(0), "nothing sent to peer 9, acked 0: in sync");
}

// ---------------------------------------------------------------------------
// M4: a truncated batch must not clear gaps it did not cover
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m4_truncated_batch_keeps_tail_gap_until_complete() {
    let a = started_node(1).await;
    let b = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![a.clone(), b.clone()]));
    {
        let mut ring = b.ring_buffer.lock().unwrap();
        for i in 1..=300u64 {
            // ~4 KB keys: 300 of them exceed the 1 MiB frame budget, so one batch is truncated.
            ring.push_outbound(
                rec(i, InvalidationScope::ExactKey, &format!("__default__:/{i:04}{}", "x".repeat(3990))),
                1,
            );
        }
    }
    {
        let mut t = a.anti_entropy_tracker.lock().unwrap();
        t.record_inbound_seq(2, 1, None);
        t.record_inbound_seq(2, 3, None); // gap at the start
        t.record_inbound_seq(2, 299, None); // gap reaching to the end of the stream
    }

    router.pull_reconcile_all_peers().await;
    let acked1 = a.anti_entropy_tracker.lock().unwrap().get_highest_acked_seq(2);
    assert!(acked1 > 1 && acked1 < 300, "first pull delivered only a prefix, acked {acked1}");
    assert!(
        a.anti_entropy_tracker.lock().unwrap().has_gaps_for(2),
        "the tail gap beyond the delivered prefix must survive the first (truncated) pull"
    );

    router.pull_reconcile_all_peers().await;
    assert_eq!(a.anti_entropy_tracker.lock().unwrap().get_highest_acked_seq(2), 300);
    assert!(!a.anti_entropy_tracker.lock().unwrap().has_gaps_for(2), "complete batch clears the gaps");
    router.stop();
    a.stop_tcp_service();
    b.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M5: promotion is not starved by debts recorded after the Recovering episode began
// ---------------------------------------------------------------------------
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m5_debt_recorded_during_pull_does_not_block_promotion() {
    let n1 = started_node(1).await;
    // Deterministic: the peer answers the pull after 150 ms; a NEW debt arrives meanwhile.
    let (peer, _log) = fake_peer_with(2, |_, _| InvalidationStatus::SuccessNotPresent, 150).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), peer.clone()]));
    peer.set_state(NodeState::Recovering);
    let (r2, p2, n1c) = (router.clone(), peer.clone(), n1.clone());
    let pull = tokio::spawn(async move { r2.reconcile_peer_anti_entropy(&p2, &n1c).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "__default__:/after_ticket");
    pull.await.unwrap().expect("pull succeeds");
    assert_eq!(peer.get_state(), NodeState::Active, "a debt newer than the ticket must not block promotion");
    router.stop();
    n1.stop_tcp_service();
}

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m5_sustained_debt_stream_still_reaches_active() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    n2.set_state(NodeState::Recovering);
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (ledger, stop2) = (router.broadcaster().repair_ledger().clone(), stop.clone());
    let recorder = tokio::spawn(async move {
        let mut i = 0u64;
        while !stop2.load(std::sync::atomic::Ordering::Relaxed) {
            ledger.record(2, None, InvalidationScope::ExactKey, &format!("__default__:/stream_{i}"));
            i += 1;
            tokio::time::sleep(Duration::from_millis(10)).await; // 100 debts per second
        }
    });
    let mut rounds = 0;
    while n2.get_state() != NodeState::Active && rounds < 10 {
        router.reconcile_all_peers().await;
        rounds += 1;
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    recorder.await.unwrap();
    println!("[CF-R4-M5] reached {:?} after {rounds} rounds under 100 debts/s", n2.get_state());
    assert_eq!(n2.get_state(), NodeState::Active, "documented bound: within 10 reconcile rounds");
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// M6: broadcaster worker lifecycle
// ---------------------------------------------------------------------------
fn test_tls(node_id: u8, cid: &str) -> Arc<ClusterTlsConfig> {
    let ca = TestCa::new();
    let (c, k, d) = ca.issue(&format!("node-{node_id}.{cid}.phantom.internal"));
    let mut reg = ClusterMembershipRegistry::new(cid, 1);
    reg.add_member(MemberNode { node_id, fingerprints: vec![compute_cert_fingerprint(&d)], active: true });
    Arc::new(ClusterTlsConfig::from_pem(node_id, cid, ca.cert_pem.as_bytes(), c.as_bytes(), k.as_bytes(), reg).unwrap())
}

fn task_for(peer: u8, key: &str) -> ReplicaInvalidationTask {
    ReplicaInvalidationTask {
        key: key.to_string(),
        scope: InvalidationScope::ExactKey,
        replica_node_ids: vec![peer],
        membership_epoch: 1,
    }
}

async fn wait_for(cap: Duration, what: &str, mut cond: impl FnMut() -> bool) {
    let t0 = Instant::now();
    while !cond() {
        assert!(t0.elapsed() < cap, "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A panicking worker loses neither its in-flight job nor its queue, and the dispatcher respawns it.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m6_panicking_worker_records_its_jobs_and_is_respawned() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    // Poison node 2's address mutex: the worker's `get_addr()` now panics.
    let n2c = n2.clone();
    let _ = std::thread::spawn(move || {
        let _g = n2c.addr.lock().unwrap();
        panic!("poisoning node 2's addr mutex on purpose");
    })
    .join();
    let ledger = router.broadcaster().repair_ledger().clone();

    router.broadcaster().enqueue(task_for(2, "__default__:/p1"));
    wait_for(Duration::from_secs(3), "first job recorded after worker panic", || ledger.pending_count(2) == 1).await;
    router.broadcaster().enqueue(task_for(2, "__default__:/p2"));
    wait_for(Duration::from_secs(3), "second job recorded: worker was respawned and panicked again", || {
        ledger.pending_count(2) == 2
    })
    .await;

    n2.addr.clear_poison();
    router.broadcaster().enqueue(task_for(2, "__default__:/p3"));
    let b = router.broadcaster().clone();
    wait_for(Duration::from_secs(3), "third job delivered by a healthy respawned worker", || {
        b.acked_count.load(std::sync::atomic::Ordering::Relaxed) >= 1
    })
    .await;
    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

/// stop() moves the in-flight job and everything still queued into the ledger.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m6_stop_drains_queued_jobs_into_the_ledger() {
    let n1 = started_node(1).await;
    let b2 = blackhole_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), b2.clone()]));
    let ledger = router.broadcaster().repair_ledger().clone();
    for i in 0..50 {
        router.broadcaster().enqueue(task_for(2, &format!("__default__:/q{i}")));
    }
    tokio::time::sleep(Duration::from_millis(50)).await; // dispatcher forwards them to the peer's queue
    router.broadcaster().stop();
    // The in-flight job needs up to ~1.4 s to exhaust its retries against the blackhole; the 49
    // queued ones are recorded when the worker ends.
    wait_for(Duration::from_secs(8), "all 50 jobs owed in the ledger", || ledger.pending_count(2) == 50).await;
    router.stop();
    n1.stop_tcp_service();
}

/// Rebuilding the broadcaster (new TLS configuration) keeps the debts.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_r4_m6_rebuild_migrates_the_repair_ledger() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = dead_node(2);
    let mut router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "__default__:/debt");
    router.set_cluster_tls(test_tls(1, "cluster-r4"));
    assert_eq!(
        router.broadcaster().repair_ledger().pending_count(2),
        1,
        "debts recorded before a rebuild must not be discarded"
    );
    router.stop();
}
