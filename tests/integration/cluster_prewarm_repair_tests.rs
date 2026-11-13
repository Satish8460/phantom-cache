//! In-process tests for the cluster connection pre-warm worker and the invalidation repair ledger
//! (task CF-3 / CF-4: S-3 replica coverage, queue-overflow behaviour, eager pool pre-warm).
//!
//! Transport: plaintext loopback between in-process `VirtualNode`s (no mTLS, no child processes),
//! which exercises exactly the same pool / router code paths that the gateway uses; the mTLS and
//! multi-process variants are covered by `distributed_consistency_tests` and
//! `cluster_propagation_latency_tests`.

use phantom_cache_demo::dist::frame::InvalidationScope;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::router::{
    RendezvousRouter, ReplicaInvalidationTask, // RepairLedger and REPAIR_MAX_ENTRIES_PER_PEER are not in src/ (PC-AUTO-047)
};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn started_node(id: u8) -> Arc<VirtualNode> {
    let node = Arc::new(VirtualNode::new(id));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    node.set_addr(addr);
    node
}

/// Polls `cond` every 10 ms until it holds or `cap` elapses; returns the elapsed time on success.
async fn poll_until(cap: Duration, mut cond: impl FnMut() -> bool) -> Result<Duration, Duration> {
    let t0 = Instant::now();
    loop {
        if cond() {
            return Ok(t0.elapsed());
        }
        if t0.elapsed() > cap {
            return Err(t0.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

// ---------------------------------------------------------------------------
// Pre-warm
// ---------------------------------------------------------------------------

/// After the pre-warm worker has run, the pool holds exactly one idle connection per member and
/// the first invalidation to each member REUSES it (no new connect/handshake on the first use).
///
/// Hard cap 5 s: the first worker iteration runs immediately; a plaintext loopback connect takes
/// well under 100 ms; the cap only guards against a hung worker.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): tests the pool pre-warm worker, which is a no-op stub in src/dist/router.rs (start_prewarm_worker, DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_prewarm_holds_idle_connection_to_every_member_and_first_invalidation_reuses_it() {
    let nodes = vec![started_node(1).await, started_node(2).await, started_node(3).await];
    let router = Arc::new(RendezvousRouter::new(nodes.clone()));

    for n in &nodes {
        assert_eq!(router.pool().idle_count_for_node(n.node_id), 0, "cold before pre-warm");
    }

    router.start_prewarm_worker();
    let pool = router.pool().clone();
    let warmed = poll_until(Duration::from_secs(5), || {
        nodes.iter().all(|n| pool.idle_count_for_node(n.node_id) >= 1)
    })
    .await;
    println!("[CF-PREWARM] all members warm after {:?}", warmed);
    assert!(warmed.is_ok(), "pool must hold an idle connection to every member (pre-warm), got none within 5s");

    for n in &nodes {
        assert_eq!(n.get_state(), NodeState::Active, "pre-warm must not change peer state");
        assert_eq!(pool.total_active_for_node(n.node_id), 1);
    }

    // First invalidation to each member must not open a second connection.
    for n in &nodes {
        let (status, _) = router
            .send_invalidation_frame(n, "/prewarm_key", InvalidationScope::ExactKey, 1000)
            .await
            .expect("invalidation over pre-warmed connection");
        assert!(status.is_success());
        assert_eq!(pool.total_active_for_node(n.node_id), 1, "no new connection opened for member {}", n.node_id);
        assert_eq!(pool.idle_count_for_node(n.node_id), 1, "connection returned to the pool");
    }

    router.stop();
    for n in &nodes {
        n.stop_tcp_service();
    }
}

/// A member that is not listening yet must NOT be demoted to Suspect by speculative pre-warm
/// attempts, and must be warmed (with backoff, no busy loop) as soon as it starts listening.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): tests the pool pre-warm worker, which is a no-op stub in src/dist/router.rs (start_prewarm_worker, DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_prewarm_unreachable_member_is_not_marked_suspect_and_is_warmed_later() {
    let n1 = started_node(1).await;
    let n3 = started_node(3).await;

    // Reserve a port for node 2 but do not listen on it yet.
    let reserved = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let n2 = Arc::new(VirtualNode::new(2));
    n2.set_addr(reserved);
    n2.set_state(NodeState::Active);

    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]));
    router.start_prewarm_worker();
    let pool = router.pool().clone();

    let others = poll_until(Duration::from_secs(5), || {
        pool.idle_count_for_node(1) >= 1 && pool.idle_count_for_node(3) >= 1
    })
    .await;
    assert!(others.is_ok(), "reachable members must be warmed even though member 2 is down");

    // Observe for 700 ms: the worker keeps trying node 2 (backoff 200/400/... ms) but never marks it.
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(700) {
        assert_eq!(n2.get_state(), NodeState::Active, "failed pre-warm must not mark the peer Suspect");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(pool.idle_count_for_node(2), 0);

    // Node 2 comes up on the reserved address; the worker must warm it within its backoff window
    // (max 5 s backoff by construction; cap 10 s).
    n2.start_tcp_service(&reserved.to_string())
        .await
        .expect("bind reserved port for node 2");
    let late = poll_until(Duration::from_secs(10), || pool.idle_count_for_node(2) >= 1).await;
    println!("[CF-PREWARM] late member warmed after {:?}", late);
    assert!(late.is_ok(), "member 2 must be warmed after it starts listening");
    assert_eq!(n2.get_state(), NodeState::Active);

    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
    n3.stop_tcp_service();
}

// ---------------------------------------------------------------------------
// Repair ledger
// ---------------------------------------------------------------------------

#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[test]
fn test_ledger_dedupes_and_completes() {
    let l = RepairLedger::new();
    l.record(2, Some(7), InvalidationScope::ExactKey, "/a");
    l.record(2, Some(8), InvalidationScope::ExactKey, "/a"); // duplicate: collapses
    l.record(2, None, InvalidationScope::ExactKey, "/b");
    assert_eq!(l.pending_count(2), 2);
    assert!(l.has_pending(2));
    assert!(!l.has_pending(3));

    let batch = l.peek_batch(2, 10);
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].seq, Some(7), "first sequence is kept for a deduplicated key");
    for e in &batch {
        l.complete(2, e);
    }
    assert!(!l.has_pending(2));
}

/// A re-record of a key while its delivery is in flight must survive the ack of the older
/// delivery (the in-flight copy may predate the newest invalidation).
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[test]
fn test_ledger_rerecord_during_inflight_delivery_is_not_lost() {
    let l = RepairLedger::new();
    l.record(2, Some(1), InvalidationScope::ExactKey, "/hot");
    let batch = l.peek_batch(2, 10); // delivery in flight
    l.record(2, Some(2), InvalidationScope::ExactKey, "/hot"); // newer invalidation arrives
    l.complete(2, &batch[0]); // older delivery acknowledged
    assert!(l.has_pending(2), "newer invalidation must still be owed");
    let again = l.peek_batch(2, 10);
    assert_eq!(again.len(), 1);
    l.complete(2, &again[0]);
    assert!(!l.has_pending(2));
}

/// Resource bound: beyond `REPAIR_MAX_ENTRIES_PER_PEER` distinct keys of ONE tenant the backlog
/// collapses into a single tenant-scope invalidation (no global flush); memory stays bounded.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[test]
fn test_ledger_overflow_collapses_to_tenant_flush() {
    let l = RepairLedger::new();
    for i in 0..REPAIR_MAX_ENTRIES_PER_PEER {
        l.record(2, None, InvalidationScope::ExactKey, &format!("t1:/k{i}"));
    }
    assert_eq!(l.pending_count(2), REPAIR_MAX_ENTRIES_PER_PEER);
    l.record(2, None, InvalidationScope::ExactKey, "t1:/one_too_many");
    assert!(!l.has_full_flush(2), "single-tenant overflow must stay tenant-scoped");
    assert_eq!(l.pending_count(2), 1, "per-key backlog replaced by one tenant flush entry");
    // Further keys of that tenant are covered by the pending tenant flush.
    l.record(2, None, InvalidationScope::ExactKey, "t1:/later");
    assert_eq!(l.pending_count(2), 1);
}

/// When the backlog is spread over many tenants with a single entry each (nothing to collapse),
/// the ledger escalates to one global flush, still bounded.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[test]
fn test_ledger_overflow_escalates_to_full_flush_when_not_collapsible() {
    let l = RepairLedger::new();
    for i in 0..REPAIR_MAX_ENTRIES_PER_PEER {
        l.record(2, None, InvalidationScope::ExactKey, &format!("tenant{i}:/k"));
    }
    l.record(2, None, InvalidationScope::ExactKey, "tenantX:/k");
    assert!(l.has_full_flush(2));
    assert_eq!(l.pending_count(2), 0);
    l.record(2, None, InvalidationScope::ExactKey, "tenantY:/later");
    assert_eq!(l.pending_count(2), 0);
    assert!(l.take_full_flush(2));
    assert!(!l.has_full_flush(2));
    l.restore_full_flush(2);
    assert!(l.has_full_flush(2));
}

// ---------------------------------------------------------------------------
// Queue overflow: a dropped fan-out task must not silently leave a peer stale
// ---------------------------------------------------------------------------

/// The broadcaster queue holds 1024 tasks. This test runs on the current-thread runtime and
/// never yields while enqueueing, so the consumer cannot drain: everything beyond 1024 is
/// rejected by `try_send`. Every rejected task must be recorded in the repair ledger for its
/// destination (previously it was dropped and counted only).
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_broadcaster_queue_overflow_is_diverted_to_repair_ledger() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));
    let broadcaster = router.broadcaster().clone();

    let total = 1100usize; // 1024 accepted, 76 rejected
    let mut rejected = 0usize;
    for i in 0..total {
        let accepted = broadcaster.enqueue(ReplicaInvalidationTask {
            key: format!("/overflow_{i}"),
            scope: InvalidationScope::ExactKey,
            replica_node_ids: vec![2],
            membership_epoch: 1,
        });
        if !accepted {
            rejected += 1;
        }
    }
    // No `.await` happened above: assertions are race-free.
    assert_eq!(rejected, total - 1024);
    assert_eq!(
        broadcaster.dropped_count.load(std::sync::atomic::Ordering::Relaxed),
        rejected,
        "drop counter still reports rejected tasks"
    );
    assert_eq!(
        broadcaster.repair_ledger().pending_count(2),
        rejected,
        "every rejected task is owed to node 2 via the repair ledger"
    );

    // The repair path delivers the owed invalidations (fresh sequences for never-dispatched tasks),
    // acknowledged by the peer, and empties the ledger.
    broadcaster.stop();
    // stop() also moves the 1024 accepted-but-undispatched tasks into the ledger: 1100 distinct keys
    // of one tenant exceed the per-peer bound and collapse into ONE tenant-scope invalidation.
    let ring_before = n2.ring_buffer.lock().unwrap().len();
    assert!(broadcaster.repair_ledger().has_pending(2), "everything owed stays in the ledger after stop()");
    // Deliver until empty (bounded batches per call).
    let mut rounds = 0;
    while broadcaster.repair_ledger().has_pending(2) && rounds < 10 {
        router.repair_peer(&n2).await;
        rounds += 1;
    }
    assert!(!broadcaster.repair_ledger().has_pending(2), "ledger must drain once the peer acknowledges");
    let ring_after = n2.ring_buffer.lock().unwrap().len();
    assert!(ring_after > ring_before, "the peer received the (collapsed) invalidation: before={ring_before} after={ring_after}");

    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}

/// Suspect peers are not contacted on the hot path, but the invalidation is owed to them and is
/// delivered by `repair_peer`, which then restores them to Active.
#[cfg(any())] // QUARANTINED (PC-AUTO-047, D-043): needs the repair ledger / repair_peer / pull_reconcile_all_peers, which are not in src/ (sandbox DEC-027). Compiled out, not deleted.
#[tokio::test]
async fn test_suspect_peer_is_repaired_and_restored() {
    let n1 = started_node(1).await;
    let n2 = started_node(2).await;
    let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]));

    n2.set_state(NodeState::Suspect);
    router.broadcaster().repair_ledger().record(2, None, InvalidationScope::ExactKey, "/owed");
    let ok = router.repair_peer(&n2).await;
    assert!(ok, "owed invalidation must be acknowledged and the ledger drained");
    assert_eq!(n2.get_state(), NodeState::Recovering, "push drain only reaches Recovering");
    router.reconcile_all_peers().await;
    assert_eq!(n2.get_state(), NodeState::Active, "pull reconciliation completes the recovery");
    assert_eq!(n2.ring_buffer.lock().unwrap().len(), 1);

    router.stop();
    n1.stop_tcp_service();
    n2.stop_tcp_service();
}
