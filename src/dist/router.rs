use crate::dist::node::{NodeState, VirtualNode};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum DistError {
    NoActiveNodes,
    NodeUnreachable(u8),
    /// The local connection pool to this node stayed full for the whole acquisition budget.
    /// This is local back-pressure, NOT evidence that the peer is down: callers must not mark
    /// the peer Suspect because of it (PC-AUTO-037).
    Busy(u8),
    Timeout,
    Rejected(crate::dist::frame::InvalidationStatus),
    ProtocolError,
}

/// Result of a peer read (PC-AUTO-041/042).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerReadResult {
    Hit { node_id: u8, entry: crate::dist::frame::PeerEntry, elapsed_ms: u64 },
    Miss { node_id: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationOutcome {
    pub canonical_node_id: u8,
    pub is_acting_primary: bool,
    pub suspect_node_id: Option<u8>,
    pub status: crate::dist::frame::InvalidationStatus,
    pub reclaimed_bytes: u32,
}

#[derive(Debug, Clone)]
pub struct ReplicaInvalidationTask {
    pub key: String,
    pub scope: crate::dist::frame::InvalidationScope,
    pub replica_node_ids: Vec<u8>,
    pub membership_epoch: u64,
}

#[derive(Clone, Default)]
pub struct DestinationSequencer {
    counters: Arc<std::sync::Mutex<std::collections::HashMap<u8, u64>>>,
}

impl DestinationSequencer {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn next_sequence_for(&self, dest_node_id: u8) -> u64 {
        let mut map = self.counters.lock().unwrap();
        let entry = map.entry(dest_node_id).or_insert(1);
        let val = *entry;
        *entry = entry.saturating_add(1);
        val
    }
}

pub struct ReplicaBroadcaster {
    tx: tokio::sync::mpsc::Sender<ReplicaInvalidationTask>,
    pub dropped_count: Arc<std::sync::atomic::AtomicUsize>,
    pub sent_count: Arc<std::sync::atomic::AtomicUsize>,
    pub retried_count: Arc<std::sync::atomic::AtomicUsize>,
    pub acked_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Tasks accepted into the queue by `enqueue` (one task = one key for a list of destinations).
    pub enqueued_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Fan-out deliveries (one per key and destination) that gave up without an ack: every
    /// attempt failed, or the destination left the membership. NOT queue overflow (see `dropped_count`).
    pub exhausted_count: Arc<std::sync::atomic::AtomicUsize>,
    shutdown_tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub dest_sequencer: DestinationSequencer,
}

/// Snapshot of the broadcaster counters. All counts are cumulative since process start.
/// `sent` counts every invalidation frame attempt (including the immediate retry on a fresh
/// connection after a suspected stale pooled connection); `retried` counts every attempt that
/// was not the first for its delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroadcasterCounters {
    pub enqueued: usize,
    pub sent: usize,
    pub retried: usize,
    pub acked: usize,
    pub dropped: usize,
    pub exhausted: usize,
}

/// Outcome of one invalidation frame attempt against one destination.
enum FanoutAttempt {
    Acked,
    /// No ack. `was_reused` is true when the failure happened on a pooled connection that had
    /// been used before (it may be a stale connection to a peer that restarted).
    Failed { was_reused: bool },
}

/// Maximum concurrent in-flight fan-out deliveries PER DESTINATION. Kept below the pool cap
/// (`max_total_per_node` = 8) so the synchronous primary leg of a DELETE can always get a slot.
const FANOUT_DEST_CONCURRENCY: usize = 4;
/// Pending deliveries queued per destination. Bounds memory (a delivery holds one key) and the
/// number of spawned tasks (at most `FANOUT_DEST_CONCURRENCY` per destination).
const FANOUT_DEST_QUEUE: usize = 8192;
/// A destination that is not Active when its delivery starts (Suspect, Recovering, Failed, ...) gets a
/// single attempt: it is already known to be unhealthy, retrying with backoff only burns a worker
/// slot. The delivery is still counted (`exhausted`) and repaired later by anti-entropy.
const FANOUT_UNHEALTHY_MAX_ATTEMPTS: usize = 1;
const FANOUT_MAX_ATTEMPTS: usize = 4;
/// How long the synchronous (primary or failover) leg of a DELETE may wait for a free pool slot before it
/// reports `Busy` (503). Separate from the 150 ms connect/IO deadline (PC-AUTO-049).
const SYNC_LEG_POOL_WAIT_MS: u64 = 1_000;
/// While a destination is `unreachable` (repeated connection failures, see `VirtualNode::reachability`),
/// at most one delivery per interval is actually attempted (a probe); the others are given up at once
/// and counted `exhausted`, so a dead peer cannot keep the workers busy. Repair is left to anti-entropy.
const FANOUT_UNREACHABLE_PROBE_INTERVAL: Duration = Duration::from_millis(500);

/// Shared, immutable context of the fan-out workers.
struct FanoutCtx {
    pool: Arc<crate::dist::pool::TlsConnectionPool>,
    tls: Option<Arc<crate::security::ClusterTlsConfig>>,
    seq_gen: Arc<std::sync::atomic::AtomicU64>,
    sent: Arc<std::sync::atomic::AtomicUsize>,
    retried: Arc<std::sync::atomic::AtomicUsize>,
    acked: Arc<std::sync::atomic::AtomicUsize>,
    exhausted: Arc<std::sync::atomic::AtomicUsize>,
    dropped: Arc<std::sync::atomic::AtomicUsize>,
    /// When an unreachable destination was last actually attempted.
    last_probe: std::sync::Mutex<std::collections::HashMap<u8, std::time::Instant>>,
}

/// One key to deliver to one destination.
struct FanoutDelivery {
    target_node: Arc<VirtualNode>,
    key: String,
    scope: crate::dist::frame::InvalidationScope,
    epoch: u64,
    dest_seq: u64,
}

/// Drains one destination queue with at most `FANOUT_DEST_CONCURRENCY` deliveries in flight. The
/// backlog stays in the (bounded) channel, not in spawned tasks.
async fn fanout_dest_worker(ctx: Arc<FanoutCtx>, mut rx: tokio::sync::mpsc::Receiver<FanoutDelivery>) {
    let sem = Arc::new(tokio::sync::Semaphore::new(FANOUT_DEST_CONCURRENCY));
    while let Some(delivery) = rx.recv().await {
        let permit = match sem.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let _permit = permit;
            fanout_deliver(&ctx, delivery).await;
        });
    }
}

/// Delivers one invalidation with bounded retries; counts `sent`, `retried`, `acked`, `exhausted`.
async fn fanout_deliver(ctx: &FanoutCtx, d: FanoutDelivery) {
    use std::sync::atomic::Ordering::Relaxed;
    let node_id = d.target_node.node_id;
    let mut max_attempts = if d.target_node.get_state() == NodeState::Active {
        FANOUT_MAX_ATTEMPTS
    } else {
        FANOUT_UNHEALTHY_MAX_ATTEMPTS
    };
    if d.target_node.reachability() == "unreachable" {
        let now = std::time::Instant::now();
        let mut probes = ctx.last_probe.lock().unwrap();
        let due = probes.get(&node_id).is_none_or(|t| now.duration_since(*t) >= FANOUT_UNREACHABLE_PROBE_INTERVAL);
        if due {
            probes.insert(node_id, now);
        } else {
            max_attempts = 0;
        }
    }
    let mut backoff_ms = 25u64;
    let mut delivered = false;
    for retry in 0..max_attempts {
        if let Some(ref c_tls) = ctx.tls {
            if let Ok(reg) = c_tls.registry.read() {
                match reg.get_member(node_id) {
                    Some(m) if m.active => {}
                    // Destination left the membership: nothing can be delivered.
                    _ => break,
                }
            }
        }

        if retry > 0 {
            ctx.retried.fetch_add(1, Relaxed);
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(200);
        }

        ctx.sent.fetch_add(1, Relaxed);
        // Retries reuse the exact stream_seq assigned for this destination.
        match ReplicaBroadcaster::attempt_fanout(
            &ctx.pool, &ctx.tls, &d.target_node, &d.key, d.scope, d.epoch, d.dest_seq, &ctx.seq_gen,
        )
        .await
        {
            FanoutAttempt::Acked => {
                ctx.acked.fetch_add(1, Relaxed);
                delivered = true;
                break;
            }
            FanoutAttempt::Failed { was_reused: true } => {
                // Stale-connection mitigation (PC-AUTO-038). A failure on a REUSED pooled connection may
                // mean the peer restarted and every idle connection to it is dead. Evict them all (this
                // also advances the pool epoch) so the immediate retry opens a genuinely new connection
                // instead of picking another stale one. The extra attempt is counted in `sent`/`retried`.
                ctx.pool.evict_node(node_id);
                ctx.retried.fetch_add(1, Relaxed);
                ctx.sent.fetch_add(1, Relaxed);
                if let FanoutAttempt::Acked = ReplicaBroadcaster::attempt_fanout(
                    &ctx.pool, &ctx.tls, &d.target_node, &d.key, d.scope, d.epoch, d.dest_seq, &ctx.seq_gen,
                )
                .await
                {
                    ctx.acked.fetch_add(1, Relaxed);
                    delivered = true;
                    break;
                }
            }
            FanoutAttempt::Failed { was_reused: false } => {}
        }
    }
    if !delivered {
        let n = ctx.exhausted.fetch_add(1, Relaxed) + 1;
        // No cache key at INFO level (PC-AUTO-039). Rate-limited: a dead peer would otherwise log once per DELETE.
        if n <= 5 || n.is_multiple_of(1000) {
            eprintln!("[INFO] [phantom_mesh] Replica fan-out gave up without an ack for node_id={} (retry budget exhausted, destination unhealthy, or member inactive)", node_id);
        }
    }
}

impl ReplicaBroadcaster {
    /// One attempt: acquire a connection, send the invalidation frame, wait for a matching ack.
    #[allow(clippy::too_many_arguments)]
    async fn attempt_fanout(
        pool: &Arc<crate::dist::pool::TlsConnectionPool>,
        tls: &Option<Arc<crate::security::ClusterTlsConfig>>,
        target_node: &VirtualNode,
        key: &str,
        scope: crate::dist::frame::InvalidationScope,
        epoch: u64,
        stream_seq: u64,
        seq_gen: &Arc<std::sync::atomic::AtomicU64>,
    ) -> FanoutAttempt {
        let node_id = target_node.node_id;
        let mut conn = match RendezvousRouter::acquire_connection_to_node(pool, tls, target_node, 150).await {
            Ok(c) => c,
            Err(_) => return FanoutAttempt::Failed { was_reused: false },
        };
        let was_reused = conn.use_count > 0;
        let req_id = rand::random::<u64>();
        let seq = conn.sequence_number;
        conn.sequence_number = conn.sequence_number.wrapping_add(1);
        let _ = seq_gen.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let orig_id = tls.as_ref().map(|c| c.node_id).unwrap_or(0);

        if let Ok(frame) = crate::dist::frame::PhantomMeshFrame::new_invalidate_request(
            req_id, seq, epoch, stream_seq, orig_id, scope, key,
        ) {
            if crate::dist::frame::write_frame(&mut conn.stream, &frame).await.is_ok() {
                if let Ok(Ok(resp)) = tokio::time::timeout(
                    Duration::from_millis(150),
                    crate::dist::frame::read_frame(&mut conn.stream),
                )
                .await
                {
                    if resp.header.request_id == req_id
                        && resp.header.msg_type == crate::dist::frame::MessageType::InvalidateAck
                    {
                        if let Ok(ack) = resp.extract_invalidate_ack() {
                            if ack.responder_id == node_id && ack.status.is_success() {
                                pool.checkin(conn);
                                target_node.note_reachable();
                                return FanoutAttempt::Acked;
                            }
                        }
                    }
                }
            }
        }
        pool.discard(conn);
        target_node.note_unreachable();
        FanoutAttempt::Failed { was_reused }
    }

    pub fn new(
        nodes: Vec<Arc<VirtualNode>>,
        pool: Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: Option<Arc<crate::security::ClusterTlsConfig>>,
    ) -> Self {
        Self::with_dest_sequencer(
            nodes,
            pool,
            cluster_tls,
            Arc::new(std::sync::atomic::AtomicU64::new(1)),
            DestinationSequencer::new(),
        )
    }

    pub fn with_stream_sequence(
        nodes: Vec<Arc<VirtualNode>>,
        pool: Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: Option<Arc<crate::security::ClusterTlsConfig>>,
        stream_sequence: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        Self::with_dest_sequencer(
            nodes,
            pool,
            cluster_tls,
            stream_sequence,
            DestinationSequencer::new(),
        )
    }

    pub fn with_dest_sequencer(
        nodes: Vec<Arc<VirtualNode>>,
        pool: Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: Option<Arc<crate::security::ClusterTlsConfig>>,
        stream_sequence: Arc<std::sync::atomic::AtomicU64>,
        dest_sequencer: DestinationSequencer,
    ) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ReplicaInvalidationTask>(1024);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let dropped_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sent_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let retried_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let acked_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let enqueued_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let exhausted_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let ctx = Arc::new(FanoutCtx {
            pool,
            tls: cluster_tls,
            seq_gen: stream_sequence,
            sent: sent_count.clone(),
            retried: retried_count.clone(),
            acked: acked_count.clone(),
            exhausted: exhausted_count.clone(),
            dropped: dropped_count.clone(),
            last_probe: std::sync::Mutex::new(std::collections::HashMap::new()),
        });

        if tokio::runtime::Handle::try_current().is_ok() {
            let d_seq = dest_sequencer.clone();
            tokio::spawn(async move {
                // One bounded queue and one worker per destination (PC-AUTO-043): a destination that is
                // slow or dead can only delay its OWN deliveries, never those to healthy replicas.
                let mut dest_queues: std::collections::HashMap<u8, tokio::sync::mpsc::Sender<FanoutDelivery>> =
                    std::collections::HashMap::new();
                loop {
                    let task = tokio::select! {
                        _ = &mut shutdown_rx => break,
                        item = rx.recv() => match item {
                            Some(t) => t,
                            None => break,
                        },
                    };

                    for node_id in task.replica_node_ids {
                        let target_node = match nodes.iter().find(|n| n.node_id == node_id).cloned() {
                            Some(n) => n,
                            None => {
                                ctx.exhausted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                continue;
                            }
                        };
                        let dest_seq = d_seq.next_sequence_for(node_id);

                        // Outbound invalidation logging (C-03)
                        if let Some(ref c_tls) = ctx.tls {
                            if let Some(local_node) = nodes.iter().find(|n| n.node_id == c_tls.node_id) {
                                local_node.ring_buffer.lock().unwrap().push(crate::dist::frame::InvalidateRecord {
                                    stream_seq: dest_seq,
                                    scope: task.scope,
                                    key: task.key.clone(),
                                });
                            }
                        }

                        let queue = dest_queues.entry(node_id).or_insert_with(|| {
                            let (dtx, drx) = tokio::sync::mpsc::channel::<FanoutDelivery>(FANOUT_DEST_QUEUE);
                            tokio::spawn(fanout_dest_worker(ctx.clone(), drx));
                            dtx
                        });
                        let delivery = FanoutDelivery {
                            target_node,
                            key: task.key.clone(),
                            scope: task.scope,
                            epoch: task.membership_epoch,
                            dest_seq,
                        };
                        if queue.try_send(delivery).is_err() {
                            // Destination queue full: this delivery is not attempted. It is counted as
                            // queue overflow (`dropped`); the ring-buffer record above lets anti-entropy
                            // repair the peer later.
                            ctx.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            });
        }

        Self {
            tx,
            dropped_count,
            sent_count,
            retried_count,
            acked_count,
            enqueued_count,
            exhausted_count,
            shutdown_tx: std::sync::Mutex::new(Some(shutdown_tx)),
            dest_sequencer,
        }
    }

    pub fn enqueue(&self, task: ReplicaInvalidationTask) -> bool {
        match self.tx.try_send(task) {
            Ok(_) => {
                self.enqueued_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                true
            }
            Err(_) => {
                self.dropped_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                false
            }
        }
    }

    /// Cumulative counters (see `BroadcasterCounters`).
    pub fn counters(&self) -> BroadcasterCounters {
        use std::sync::atomic::Ordering::Relaxed;
        BroadcasterCounters {
            enqueued: self.enqueued_count.load(Relaxed),
            sent: self.sent_count.load(Relaxed),
            retried: self.retried_count.load(Relaxed),
            acked: self.acked_count.load(Relaxed),
            dropped: self.dropped_count.load(Relaxed),
            exhausted: self.exhausted_count.load(Relaxed),
        }
    }

    pub fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}

pub struct RendezvousRouter {
    nodes: Vec<Arc<VirtualNode>>,
    cluster_tls: Option<Arc<crate::security::ClusterTlsConfig>>,
    local_node_id: Option<u8>,
    pool: Arc<crate::dist::pool::TlsConnectionPool>,
    broadcaster: Arc<ReplicaBroadcaster>,
    pub stream_sequence: Arc<std::sync::atomic::AtomicU64>,
    pub dest_sequencer: DestinationSequencer,
    reconcile_shutdown_tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    health_shutdown_tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl RendezvousRouter {
    pub fn new(nodes: Vec<Arc<VirtualNode>>) -> Self {
        let pool = Arc::new(crate::dist::pool::TlsConnectionPool::new(Default::default()));
        pool.start_heartbeat_worker();
        let stream_sequence = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let dest_sequencer = DestinationSequencer::new();
        let broadcaster = Arc::new(ReplicaBroadcaster::with_dest_sequencer(
            nodes.clone(),
            pool.clone(),
            None,
            stream_sequence.clone(),
            dest_sequencer.clone(),
        ));
        Self {
            nodes,
            cluster_tls: None,
            local_node_id: None,
            pool,
            broadcaster,
            stream_sequence,
            dest_sequencer,
            reconcile_shutdown_tx: std::sync::Mutex::new(None),
            health_shutdown_tx: std::sync::Mutex::new(None),
        }
    }

    pub fn with_local_node_id(mut self, node_id: u8) -> Self {
        self.local_node_id = Some(node_id);
        self
    }

    pub fn with_pool(mut self, pool: Arc<crate::dist::pool::TlsConnectionPool>) -> Self {
        self.pool.stop_heartbeat_worker();
        pool.start_heartbeat_worker();
        let broadcaster = Arc::new(ReplicaBroadcaster::with_dest_sequencer(
            self.nodes.clone(),
            pool.clone(),
            self.cluster_tls.clone(),
            self.stream_sequence.clone(),
            self.dest_sequencer.clone(),
        ));
        self.broadcaster.stop();
        self.broadcaster = broadcaster;
        self.pool = pool;
        self
    }

    pub fn pool(&self) -> &Arc<crate::dist::pool::TlsConnectionPool> {
        &self.pool
    }

    pub fn broadcaster(&self) -> &Arc<ReplicaBroadcaster> {
        &self.broadcaster
    }

    pub fn cluster_tls(&self) -> Option<&Arc<crate::security::ClusterTlsConfig>> {
        self.cluster_tls.as_ref()
    }

    pub fn stop(&self) {
        self.pool.stop_heartbeat_worker();
        self.broadcaster.stop();
        self.stop_anti_entropy_worker();
        self.stop_health_probe_worker();
    }

    pub fn get_local_node(&self) -> Option<Arc<VirtualNode>> {
        let orig_id = self.get_node_id();
        self.nodes
            .iter()
            .find(|n| n.node_id == orig_id)
            .cloned()
            .or_else(|| self.nodes.first().cloned())
    }

    pub fn start_anti_entropy_worker(self: &Arc<Self>, interval: Duration) {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut guard = self.reconcile_shutdown_tx.lock().unwrap();
            if guard.is_some() {
                return;
            }
            *guard = Some(shutdown_tx);
        }

        let router = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    _ = ticker.tick() => {
                        let r = match router.upgrade() {
                            Some(r) => r,
                            None => break,
                        };
                        r.reconcile_all_peers().await;
                    }
                }
            }
        });
    }

    pub fn stop_anti_entropy_worker(&self) {
        if let Some(tx) = self.reconcile_shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    /// Stub for inter-node connection pool pre-warming (M3/M7 cluster mode).
    /// Full pre-warming and durability log replay is isolated to DEC-027 remediation in sandbox.
    pub fn start_prewarm_worker(self: &Arc<Self>) {
        // No-op in core until DEC-027 remediation is formally reviewed and merged.
    }

    pub async fn reconcile_all_peers(&self) {
        let local = match self.get_local_node() {
            Some(l) => l,
            None => return,
        };

        for peer in &self.nodes {
            if peer.node_id == local.node_id {
                continue;
            }
            // A peer (or this node) that is Suspect OR Recovering must be reconciled again until it is
            // promoted: `Recovering` is a transitional state, not a resting one (PC-AUTO-037). Without
            // retrying it, a peer left Recovering after a failed or partial reconciliation would stay
            // out of replica routing for good.
            let needs_recovery = |s: NodeState| s == NodeState::Suspect || s == NodeState::Recovering;
            let is_unhealthy = needs_recovery(peer.get_state()) || needs_recovery(local.get_state());
            let has_gaps = local.anti_entropy_tracker.lock().unwrap().has_gaps();
            if is_unhealthy || has_gaps {
                let _ = self.reconcile_peer_anti_entropy(peer, &local).await;
            }
        }
    }

    /// Actively probes a specific peer node via TCP / mTLS heartbeat.
    /// Updates the node's health state machine: transitions Active -> Suspect -> Failed on consecutive errors,
    /// and Failed/Suspect -> Recovering -> Active upon verified probe responses.
    pub async fn probe_node(
        &self,
        node: &VirtualNode,
        timeout_ms: u64,
        failure_threshold: u32,
    ) -> Result<(), DistError> {
        let local_id = self.get_node_id();
        if node.node_id == local_id {
            node.record_probe_success(1);
            return Ok(());
        }

        let mut conn = match Self::acquire_connection_to_node(&self.pool, &self.cluster_tls, node, timeout_ms).await {
            Ok(c) => c,
            Err(DistError::Busy(id)) => return Err(DistError::Busy(id)), // local pool pressure is not a peer failure
            Err(e) => {
                node.record_probe_failure(failure_threshold);
                return Err(e);
            }
        };

        match self.pool.send_heartbeat(&mut conn, timeout_ms).await {
            Ok(()) => {
                self.pool.checkin(conn);
                node.record_probe_success(1);
                Ok(())
            }
            Err(_) => {
                self.pool.discard(conn);
                node.record_probe_failure(failure_threshold);
                Err(DistError::Timeout)
            }
        }
    }

    /// Actively probes all configured peer nodes.
    pub async fn probe_all_peers(&self, timeout_ms: u64, failure_threshold: u32) {
        let local_id = self.get_node_id();
        for node in &self.nodes {
            if node.node_id != local_id {
                let _ = self.probe_node(node, timeout_ms, failure_threshold).await;
            }
        }
    }

    /// Starts a background health probing worker that periodically pings all cluster peers.
    pub fn start_health_probe_worker(
        self: &Arc<Self>,
        interval: Duration,
        timeout_ms: u64,
        failure_threshold: u32,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut guard = self.health_shutdown_tx.lock().unwrap();
            if guard.is_some() {
                return;
            }
            *guard = Some(shutdown_tx);
        }

        let router = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    _ = ticker.tick() => {
                        let r = match router.upgrade() {
                            Some(r) => r,
                            None => break,
                        };
                        r.probe_all_peers(timeout_ms, failure_threshold).await;
                    }
                }
            }
        });
    }

    pub fn stop_health_probe_worker(&self) {
        if let Some(tx) = self.health_shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    pub fn with_cluster_tls(mut self, cluster_tls: Arc<crate::security::ClusterTlsConfig>) -> Self {
        let broadcaster = Arc::new(ReplicaBroadcaster::with_dest_sequencer(
            self.nodes.clone(),
            self.pool.clone(),
            Some(cluster_tls.clone()),
            self.stream_sequence.clone(),
            self.dest_sequencer.clone(),
        ));
        self.broadcaster.stop();
        self.broadcaster = broadcaster;
        self.cluster_tls = Some(cluster_tls);
        self
    }

    pub fn set_cluster_tls(&mut self, cluster_tls: Arc<crate::security::ClusterTlsConfig>) {
        let broadcaster = Arc::new(ReplicaBroadcaster::with_dest_sequencer(
            self.nodes.clone(),
            self.pool.clone(),
            Some(cluster_tls.clone()),
            self.stream_sequence.clone(),
            self.dest_sequencer.clone(),
        ));
        self.broadcaster.stop();
        self.broadcaster = broadcaster;
        self.cluster_tls = Some(cluster_tls);
    }

    pub fn get_membership_epoch(&self) -> u64 {
        if let Some(ref tls) = self.cluster_tls {
            if let Ok(reg) = tls.registry.read() {
                return reg.version;
            }
        }
        1
    }

    pub fn get_node_id(&self) -> u8 {
        self.cluster_tls
            .as_ref()
            .map(|c| c.node_id)
            .or(self.local_node_id)
            .unwrap_or(0)
    }

    pub async fn acquire_connection_to_node(
        pool: &Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: &Option<Arc<crate::security::ClusterTlsConfig>>,
        node: &VirtualNode,
        timeout_ms: u64,
    ) -> Result<crate::dist::pool::PooledConnection, DistError> {
        Self::acquire_connection_waiting(pool, cluster_tls, node, timeout_ms, timeout_ms).await
    }

    /// Like `acquire_connection_to_node`, but the wait for a free pool slot has its own budget
    /// (`wait_ms`, at least `timeout_ms`) separate from the connect, handshake and I/O deadline
    /// (`timeout_ms`). Used by the synchronous leg of a DELETE (PC-AUTO-049): under a concurrent
    /// storm a caller that waits a little longer for a slot succeeds instead of answering 503.
    pub async fn acquire_connection_waiting(
        pool: &Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: &Option<Arc<crate::security::ClusterTlsConfig>>,
        node: &VirtualNode,
        timeout_ms: u64,
        wait_ms: u64,
    ) -> Result<crate::dist::pool::PooledConnection, DistError> {
        let mut count_failure = true;
        let res = Self::acquire_connection_inner(pool, cluster_tls, node, timeout_ms, wait_ms.max(timeout_ms), &mut count_failure).await;
        // Failing to establish a verified connection counts toward the peer's unreachable streak;
        // local pool pressure (`Busy`) says nothing about the peer.
        if let Err(ref e) = res {
            // `count_failure` is false when a waiter gave up without attempting anything (PC-AUTO-052): only real
            // connect, handshake or message failures count toward `consecutive_failed_attempts`.
            if count_failure && !matches!(e, DistError::Busy(_)) {
                node.note_unreachable();
            }
        }
        res
    }

    async fn acquire_connection_inner(
        pool: &Arc<crate::dist::pool::TlsConnectionPool>,
        cluster_tls: &Option<Arc<crate::security::ClusterTlsConfig>>,
        node: &VirtualNode,
        timeout_ms: u64,
        wait_ms: u64,
        count_failure: &mut bool,
    ) -> Result<crate::dist::pool::PooledConnection, DistError> {
        if node.get_state() == NodeState::Failed || node.get_state() == NodeState::Offline {
            return Err(DistError::NodeUnreachable(node.node_id));
        }
        let addr = match node.get_addr() {
            Some(a) => a,
            None => return Err(DistError::NodeUnreachable(node.node_id)),
        };

        if cluster_tls.is_none() && !addr.ip().is_loopback() {
            node.set_state(NodeState::Suspect);
            return Err(DistError::NodeUnreachable(node.node_id));
        }

        let wait_start = std::time::Instant::now();
        let wait_dur = Duration::from_millis(wait_ms);
        let timeout_dur = Duration::from_millis(timeout_ms);

        // A full pool is local back-pressure (every connection slot to this peer is in use by
        // concurrent callers), not a sign that the peer is down. Wait for a slot within the caller's
        // budget; if none frees up report `Busy`, which callers must not treat as peer failure.
        let acquisition = loop {
            match pool.checkout_or_reserve(node.node_id) {
                crate::dist::pool::AcquisitionResult::Full => {
                    if wait_start.elapsed() >= wait_dur {
                        return Err(DistError::Busy(node.node_id));
                    }
                    // The slots are full because connects to a DEAD peer are hanging (each holds a slot for the
                    // connect timeout). Waiting longer would only turn a dead peer into a 503: once the peer has
                    // proven unreachable, stop waiting and let the caller fail over (PC-AUTO-049).
                    if node.reachability() == "unreachable" {
                        *count_failure = false;
                        return Err(DistError::NodeUnreachable(node.node_id));
                    }
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                other => break other,
            }
        };

        // The connect/handshake budget starts once a slot is held.
        let start = std::time::Instant::now();
        match acquisition {
            crate::dist::pool::AcquisitionResult::Idle(c) => Ok(c),
            crate::dist::pool::AcquisitionResult::Full => Err(DistError::Busy(node.node_id)),
            crate::dist::pool::AcquisitionResult::Reserved(reservation) => {
                let remaining_connect = timeout_dur.saturating_sub(start.elapsed());
                if remaining_connect.is_zero() {
                    return Err(DistError::Timeout);
                }

                let connect_res = tokio::time::timeout(
                    remaining_connect,
                    tokio::net::TcpStream::connect(addr),
                )
                .await;

                let stream = match connect_res {
                    Ok(Ok(s)) => s,
                    Ok(Err(_)) => {
                        node.set_state(NodeState::Suspect);
                        return Err(DistError::NodeUnreachable(node.node_id));
                    }
                    Err(_) => {
                        node.set_state(NodeState::Suspect);
                        return Err(DistError::Timeout);
                    }
                };

                let cluster_stream = if let Some(tls_cfg) = cluster_tls {
                    let server_name = match rustls::pki_types::ServerName::try_from(format!(
                        "node-{}.{}.phantom.internal",
                        node.node_id, tls_cfg.cluster_id
                    )) {
                        Ok(name) => name,
                        Err(_) => {
                            node.set_state(NodeState::Suspect);
                            return Err(DistError::NodeUnreachable(node.node_id));
                        }
                    };

                    let connector = tls_cfg.client_connector();
                    let remaining_handshake = timeout_dur.saturating_sub(start.elapsed());
                    if remaining_handshake.is_zero() {
                        node.set_state(NodeState::Suspect);
                        return Err(DistError::Timeout);
                    }

                    let handshake_res = tokio::time::timeout(
                        remaining_handshake,
                        connector.connect(server_name, stream),
                    )
                    .await;

                    let tls_stream = match handshake_res {
                        Ok(Ok(s)) => s,
                        Ok(Err(_)) => {
                            node.set_state(NodeState::Suspect);
                            return Err(DistError::NodeUnreachable(node.node_id));
                        }
                        Err(_) => {
                            node.set_state(NodeState::Suspect);
                            return Err(DistError::Timeout);
                        }
                    };

                    let (_, client_conn) = tls_stream.get_ref();
                    let peer_certs = match client_conn.peer_certificates() {
                        Some(certs) if !certs.is_empty() => certs,
                        _ => {
                            node.set_state(NodeState::Suspect);
                            return Err(DistError::NodeUnreachable(node.node_id));
                        }
                    };
                    let leaf_cert = &peer_certs[0];

                    {
                        let reg_guard = match tls_cfg.registry.read() {
                            Ok(g) => g,
                            Err(_) => {
                                node.set_state(NodeState::Suspect);
                                return Err(DistError::NodeUnreachable(node.node_id));
                            }
                        };
                        let member = match reg_guard.get_member(node.node_id) {
                            Some(m) if m.active => m,
                            _ => {
                                node.set_state(NodeState::Suspect);
                                return Err(DistError::NodeUnreachable(node.node_id));
                            }
                        };
                        let fingerprint = crate::security::cluster::compute_cert_fingerprint(leaf_cert.as_ref());
                        if !member.fingerprints.iter().any(|fp| fp.eq_ignore_ascii_case(&fingerprint)) {
                            node.set_state(NodeState::Suspect);
                            return Err(DistError::NodeUnreachable(node.node_id));
                        }
                    }

                    crate::dist::pool::ClusterStream::Tls(Box::new(tls_stream))
                } else {
                    crate::dist::pool::ClusterStream::Plain(stream)
                };

                Ok(reservation.into_connection(cluster_stream))
            }
        }
    }

    /// Deterministic 64-bit FNV-1a hash algorithm for consistent cross-node rendezvous routing.
    ///
    /// # Rationale
    /// `std::collections::hash_map::DefaultHasher` has an unspecified algorithm and does not
    /// guarantee stable hashing across Rust releases or compiler versions. FNV-1a provides
    /// an explicitly defined, specification-locked, cross-version deterministic 64-bit hash
    /// without introducing external crate dependencies.
    ///
    /// # Input Encoding
    /// The input stream hashed is strictly the UTF-8 bytes of `key` (`key.as_bytes()`)
    /// followed by the single `node_id` byte (`u8`).
    ///
    /// # Scope
    /// FNV-1a establishes deterministic scoring for Highest Random Weight (HRW) consensus,
    /// not cryptographic security or complete distributed ownership correctness.
    pub fn calculate_hash(key: &str, node_id: u8) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;
        for byte in key.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash ^= node_id as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash
    }

    pub fn route_key(&self, key: &str) -> Option<Arc<VirtualNode>> {
        let mut highest_weight = 0.0;
        let mut selected_node: Option<Arc<VirtualNode>> = None;

        // Pass 1: Prefer active, healthy nodes (strictly exclude Suspect, Failed, Offline, Recovering)
        for node in &self.nodes {
            let state = node.get_state();
            if state == NodeState::Failed || state == NodeState::Offline || state == NodeState::Suspect || state == NodeState::Recovering {
                continue;
            }

            let hash_val = Self::calculate_hash(key, node.node_id);
            let capacity_weight = 1.0;
            let weight = (hash_val as f64) * capacity_weight;

            if weight > highest_weight {
                highest_weight = weight;
                selected_node = Some(node.clone());
            }
        }

        // Pass 2: Fallback to suspect/recovering nodes ONLY if no active non-suspect nodes exist
        if selected_node.is_none() {
            for node in &self.nodes {
                let state = node.get_state();
                if state == NodeState::Failed || state == NodeState::Offline {
                    continue;
                }

                let hash_val = Self::calculate_hash(key, node.node_id);
                let capacity_weight = 1.0;
                let weight = (hash_val as f64) * capacity_weight;

                if weight > highest_weight {
                    highest_weight = weight;
                    selected_node = Some(node.clone());
                }
            }
        }

        selected_node
    }

    pub fn get_replicas(&self, key: &str, count: usize) -> Vec<Arc<VirtualNode>> {
        let mut weighted_nodes: Vec<(f64, Arc<VirtualNode>)> = self
            .nodes
            .iter()
            .filter(|node| {
                let s = node.get_state();
                s != NodeState::Failed && s != NodeState::Offline && s != NodeState::Suspect && s != NodeState::Recovering
            })
            .map(|node| {
                let hash_val = Self::calculate_hash(key, node.node_id);
                (hash_val as f64, node.clone())
            })
            .collect();

        // Sort descending by weight value
        weighted_nodes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        weighted_nodes
            .into_iter()
            .skip(1) // Skip primary
            .take(count)
            .map(|(_, node)| node)
            .collect()
    }

    /// Fan-out targets for an invalidation: every CONFIGURED member except `exclude` (the node that
    /// is invalidated synchronously), ranked by rendezvous weight, whatever its health state.
    ///
    /// Unlike `get_replicas` (read routing and failover candidates, which skips unhealthy members),
    /// this list must not shrink when a peer is Suspect, Recovering or Failed: such a peer may still
    /// hold the entry, and dropping it silently is how a stale entry survives a DELETE (PC-AUTO-037).
    pub fn invalidation_targets(&self, key: &str, exclude: u8) -> Vec<Arc<VirtualNode>> {
        let mut ranked: Vec<(u64, Arc<VirtualNode>)> = self
            .nodes
            .iter()
            .filter(|n| n.node_id != exclude)
            .map(|n| (Self::calculate_hash(key, n.node_id), n.clone()))
            .collect();
        ranked.sort_by_key(|a| std::cmp::Reverse(a.0));
        ranked.into_iter().map(|(_, n)| n).collect()
    }

    /// Returns the broadcaster atomic counters (sent, retried, acked, dropped).
    pub fn get_broadcaster_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.broadcaster.sent_count.load(std::sync::atomic::Ordering::Relaxed),
            self.broadcaster.retried_count.load(std::sync::atomic::Ordering::Relaxed),
            self.broadcaster.acked_count.load(std::sync::atomic::Ordering::Relaxed),
            self.broadcaster.dropped_count.load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Returns all broadcaster counters, including `enqueued` and `exhausted` (PC-AUTO-034).
    pub fn get_broadcaster_counters(&self) -> BroadcasterCounters {
        self.broadcaster.counters()
    }

    /// Returns per-peer observability status: (node_id, state, idle_connections, active_connections).
    pub fn get_peer_observability(&self) -> Vec<(u8, NodeState, usize, usize)> {
        self.nodes
            .iter()
            .map(|node| {
                let (idle, active) = self.pool.get_pool_counts(node.node_id);
                (node.node_id, node.get_state(), idle, active)
            })
            .collect()
    }

    /// Observed reachability per configured node: (node_id, label, consecutive failed attempts).
    /// Labels: `reachable`, `degraded`, `unreachable` (see `VirtualNode::reachability`).
    pub fn get_peer_reachability(&self) -> Vec<(u8, &'static str, u32)> {
        self.nodes
            .iter()
            .map(|n| (n.node_id, n.reachability(), n.unreachable_streak.load(std::sync::atomic::Ordering::Relaxed)))
            .collect()
    }

    /// Forwards a request over real TCP to the primary owner or fails over to secondary replicas.
    /// Distinguishes actual TCP connection failures from in-memory state flags.
    pub async fn forward_key_over_tcp(
        &self,
        key: &str,
        timeout_ms: u64,
        max_retries: usize,
    ) -> Result<(u8, String), DistError> {
        let primary = self.route_key(key).ok_or(DistError::NoActiveNodes)?;
        let replicas = self.get_replicas(key, max_retries);
        let mut candidates = vec![primary];
        candidates.extend(replicas);

        let mut last_err = DistError::Timeout;
        let local_id = self.get_node_id();
        for node in candidates {
            if node.node_id == local_id {
                continue;
            }
            let mut conn = match Self::acquire_connection_to_node(&self.pool, &self.cluster_tls, &node, timeout_ms).await {
                Ok(c) => c,
                Err(e) => {
                    last_err = e;
                    continue;
                }
            };

            let req_id = rand::random::<u64>();
            let seq = conn.sequence_number;
            conn.sequence_number = conn.sequence_number.wrapping_add(1);

            let req_frame = match crate::dist::frame::PhantomMeshFrame::new_get_request(
                req_id,
                seq,
                key,
            ) {
                Ok(f) => f,
                Err(_) => {
                    self.pool.discard(conn);
                    return Err(DistError::Timeout);
                }
            };

            if crate::dist::frame::write_frame(&mut conn.stream, &req_frame).await.is_err() {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                continue;
            }

            let read_res = tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                crate::dist::frame::read_frame(&mut conn.stream),
            )
            .await;

            let resp_frame = match read_res {
                Ok(Ok(f)) => f,
                _ => {
                    self.pool.discard(conn);
                    node.set_state(NodeState::Suspect);
                    continue;
                }
            };

            if resp_frame.header.request_id != req_id {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                continue;
            }

            if resp_frame.header.msg_type != crate::dist::frame::MessageType::GetResponse {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                continue;
            }

            let body = match resp_frame.extract_response() {
                Ok(b) => b.to_string(),
                Err(_) => {
                    self.pool.discard(conn);
                    node.set_state(NodeState::Suspect);
                    continue;
                }
            };

            self.pool.checkin(conn);
            return Ok((node.node_id, body));
        }

        Err(last_err)
    }

    /// Peer read with full resource metadata (PC-AUTO-041/042). Asks the primary owner (then up to
    /// `max_retries` ranked replicas) for a FRESH entry over `PeerReadRequest`; the first node that
    /// answers decides. A node that does not speak the payload format yields `ProtocolError` (it is
    /// refused cleanly, not marked Suspect, and the next candidate is tried); the caller falls back
    /// to the origin. `elapsed_ms` is the time this call took, so the asking gateway can subtract the
    /// transfer time from the remaining lifetime.
    pub async fn read_entry_from_peer(
        &self,
        key: &str,
        timeout_ms: u64,
        max_retries: usize,
    ) -> Result<PeerReadResult, DistError> {
        let started = std::time::Instant::now();
        let primary = self.route_key(key).ok_or(DistError::NoActiveNodes)?;
        let mut candidates = vec![primary];
        candidates.extend(self.get_replicas(key, max_retries));

        let mut last_err = DistError::Timeout;
        let local_id = self.get_node_id();
        for node in candidates {
            if node.node_id == local_id {
                continue;
            }
            let mut conn = match Self::acquire_connection_to_node(&self.pool, &self.cluster_tls, &node, timeout_ms).await {
                Ok(c) => c,
                Err(e) => {
                    last_err = e;
                    continue;
                }
            };
            let req_id = rand::random::<u64>();
            let seq = conn.sequence_number;
            conn.sequence_number = conn.sequence_number.wrapping_add(1);
            let req = match crate::dist::frame::PhantomMeshFrame::new_peer_read_request(req_id, seq, key) {
                Ok(f) => f,
                Err(_) => {
                    self.pool.discard(conn);
                    return Err(DistError::ProtocolError);
                }
            };
            if crate::dist::frame::write_frame(&mut conn.stream, &req).await.is_err() {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                last_err = DistError::NodeUnreachable(node.node_id);
                continue;
            }
            let resp = match tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                crate::dist::frame::read_frame(&mut conn.stream),
            )
            .await
            {
                Ok(Ok(f)) if f.header.request_id == req_id => f,
                // Timeout, closed connection or mismatched correlation id. A build that does not know
                // PeerReadRequest closes the connection, which lands here too: for this read only the
                // peer is skipped (no health state change), and the caller falls back to the origin.
                _ => {
                    self.pool.discard(conn);
                    last_err = DistError::Timeout;
                    continue;
                }
            };
            let outcome = resp.extract_peer_read_response();
            match outcome {
                Ok(Some(entry)) => {
                    self.pool.checkin(conn);
                    node.note_reachable();
                    return Ok(PeerReadResult::Hit {
                        node_id: node.node_id,
                        entry,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    });
                }
                Ok(None) => {
                    self.pool.checkin(conn);
                    node.note_reachable();
                    return Ok(PeerReadResult::Miss { node_id: node.node_id });
                }
                Err(_) => {
                    // Unsupported format or malformed payload: the peer is alive, we just cannot use it.
                    self.pool.discard(conn);
                    last_err = DistError::ProtocolError;
                    continue;
                }
            }
        }
        Err(last_err)
    }

    pub async fn send_invalidation_frame(
        &self,
        node: &VirtualNode,
        key: &str,
        scope: crate::dist::frame::InvalidationScope,
        timeout_ms: u64,
    ) -> Result<(crate::dist::frame::InvalidationStatus, u32), DistError> {
        let timeout_dur = Duration::from_millis(timeout_ms);

        // First connection BEFORE a sequence number is allocated: a connect failure must not burn a sequence (M-9).
        let mut first_conn = Some(Self::acquire_connection_waiting(&self.pool, &self.cluster_tls, node, timeout_ms, SYNC_LEG_POOL_WAIT_MS).await?);
        let stream_seq = self.dest_sequencer.next_sequence_for(node.node_id);
        self.stream_sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let membership_epoch = self.get_membership_epoch();
        let originator_id = self.get_node_id();

        // Outbound invalidation logging (C-03)
        if let Some(ref local) = self.get_local_node() {
            if local.node_id != node.node_id {
                local.ring_buffer.lock().unwrap().push(crate::dist::frame::InvalidateRecord {
                    stream_seq,
                    scope,
                    key: key.to_string(),
                });
            }
        }

        // A pooled connection that was idle across a peer restart is dead. A failure on a REUSED connection is
        // retried once on a fresh one (all idle connections evicted) before the peer is judged (L2, PC-AUTO-047).
        let mut retried_on_fresh = false;
        let attempt_result = 'attempt: loop {
        let mut conn = match first_conn.take() {
            Some(c) => c,
            None => Self::acquire_connection_waiting(&self.pool, &self.cluster_tls, node, timeout_ms, SYNC_LEG_POOL_WAIT_MS).await?,
        };
        // The I/O deadline starts once a connection is held: waiting for a pool slot must not eat it (M-5).
        let start = std::time::Instant::now();
        let was_reused = conn.use_count > 0;
        let req_id = rand::random::<u64>();
        let seq = conn.sequence_number;
        conn.sequence_number = conn.sequence_number.wrapping_add(1);

        let req_frame = match crate::dist::frame::PhantomMeshFrame::new_invalidate_request(
            req_id,
            seq,
            membership_epoch,
            stream_seq,
            originator_id,
            scope,
            key,
        ) {
            Ok(f) => f,
            Err(_) => {
                self.pool.discard(conn);
                return Err(DistError::Timeout);
            }
        };

        if crate::dist::frame::write_frame(&mut conn.stream, &req_frame).await.is_err() {
            self.pool.discard(conn);
            if was_reused && !retried_on_fresh {
                retried_on_fresh = true;
                self.pool.evict_node(node.node_id);
                continue 'attempt;
            }
            node.set_state(NodeState::Suspect);
            return Err(DistError::NodeUnreachable(node.node_id));
        }

        let remaining = timeout_dur.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            self.pool.discard(conn);
            node.set_state(NodeState::Suspect);
            return Err(DistError::Timeout);
        }

        let read_res = tokio::time::timeout(
            remaining,
            crate::dist::frame::read_frame(&mut conn.stream),
        )
        .await;

        match read_res {
            Ok(Ok(f)) => break 'attempt (conn, req_id, f),
            // A read TIMEOUT is not evidence of a stale connection (L-3): never retried, the leg stays strict.
            Err(_) => {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                return Err(DistError::Timeout);
            }
            // The connection itself failed (closed by a restarted peer): retry once on a fresh one.
            Ok(Err(_)) => {
                self.pool.discard(conn);
                if was_reused && !retried_on_fresh {
                    retried_on_fresh = true;
                    self.pool.evict_node(node.node_id);
                    continue 'attempt;
                }
                node.set_state(NodeState::Suspect);
                return Err(DistError::Timeout);
            }
        }
        };
        let (conn, req_id, resp_frame) = attempt_result;

        if resp_frame.header.request_id != req_id {
            self.pool.discard(conn);
            node.set_state(NodeState::Suspect);
            return Err(DistError::NodeUnreachable(node.node_id));
        }

        if resp_frame.header.msg_type != crate::dist::frame::MessageType::InvalidateAck {
            self.pool.discard(conn);
            node.set_state(NodeState::Suspect);
            return Err(DistError::NodeUnreachable(node.node_id));
        }

        let ack = match resp_frame.extract_invalidate_ack() {
            Ok(a) => a,
            Err(_) => {
                self.pool.discard(conn);
                node.set_state(NodeState::Suspect);
                return Err(DistError::NodeUnreachable(node.node_id));
            }
        };

        // SEC-4B-05: Validate responder identity and membership epoch
        if ack.responder_id != node.node_id {
            self.pool.discard(conn);
            node.set_state(NodeState::Suspect);
            return Err(DistError::NodeUnreachable(node.node_id));
        }

        if ack.membership_epoch != self.get_membership_epoch() {
            self.pool.discard(conn);
            return Err(DistError::Rejected(crate::dist::frame::InvalidationStatus::IgnoredObsolete));
        }

        // SEC-4B-06: Explicitly check status success
        if !ack.status.is_success() {
            self.pool.checkin(conn);
            return Err(DistError::Rejected(ack.status));
        }

        self.pool.checkin(conn);
        node.note_reachable();
        Ok((ack.status, ack.reclaimed_bytes))
    }

    /// Synchronously invalidates a key on the canonical primary node with acting-primary failover,
    /// bounded by a strict 150ms deadline. On canonical success, enqueues secondary replicas into
    /// the background broadcast worker.
    pub async fn invalidate_key_across_cluster(
        &self,
        key: &str,
        scope: crate::dist::frame::InvalidationScope,
    ) -> Result<InvalidationOutcome, DistError> {
        let timeout_ms = std::env::var("PHANTOM_CLUSTER_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(150);

        let primary = self.route_key(key).ok_or(DistError::NoActiveNodes)?;
        // Read routing keeps its health filter; invalidation delivery does not (PC-AUTO-037). A
        // member that is Suspect/Recovering/Failed is exactly the one that may still hold the
        // entry, so it must stay a fan-out target. The broadcaster retries it and, if it is still
        // unreachable, counts the delivery as `exhausted`.
        let targets = self.invalidation_targets(key, primary.node_id);
        let target_ids: Vec<u8> = targets.iter().map(|n| n.node_id).collect();
        // Healthy replicas, ranked: the only candidates for acting-primary failover.
        let replicas = self.get_replicas(key, 2);

        // 1. Attempt Primary Node (150ms per-attempt timeout)
        match self.send_invalidation_frame(&primary, key, scope, timeout_ms).await {
            Ok((status, reclaimed_bytes)) => {
                if !target_ids.is_empty() {
                    self.broadcaster.enqueue(ReplicaInvalidationTask {
                        key: key.to_string(),
                        scope,
                        replica_node_ids: target_ids,
                        membership_epoch: self.get_membership_epoch(),
                    });
                }

                Ok(InvalidationOutcome {
                    canonical_node_id: primary.node_id,
                    is_acting_primary: false,
                    suspect_node_id: None,
                    status,
                    reclaimed_bytes,
                })
            }
            Err(DistError::Rejected(status)) => {
                // Primary is authoritative and reachable, but rejected invalidation: do not fail over.
                Err(DistError::Rejected(status))
            }
            Err(DistError::Busy(id)) => {
                // Local pool pressure, not a peer failure: do not mark the primary Suspect and do not
                // fail over. The caller is told to retry, but the invalidation is not left undone: the
                // broadcaster delivers it to every other member including the primary.
                let mut all_ids = vec![primary.node_id];
                all_ids.extend(target_ids);
                self.broadcaster.enqueue(ReplicaInvalidationTask {
                    key: key.to_string(),
                    scope,
                    replica_node_ids: all_ids,
                    membership_epoch: self.get_membership_epoch(),
                });
                Err(DistError::Busy(id))
            }
            Err(err) => {
                primary.set_state(NodeState::Suspect);

                // 2. Attempt Acting Primary Failover (150ms per-attempt timeout)
                let acting_primary = match replicas.first() {
                    Some(r) if r.get_state() != NodeState::Failed && r.get_state() != NodeState::Offline && r.get_state() != NodeState::Suspect => r.clone(),
                    _ => return Err(err),
                };

                match self.send_invalidation_frame(&acting_primary, key, scope, timeout_ms).await {
                    Ok((status, reclaimed_bytes)) => {
                        // Suspect primary plus every other member go into the background broadcast
                        // retry queue (SEC-4B-01); only the acting primary is already done.
                        let remaining_replicas: Vec<u8> = std::iter::once(primary.node_id)
                            .chain(
                                self.invalidation_targets(key, primary.node_id)
                                    .iter()
                                    .map(|n| n.node_id)
                                    .filter(|id| *id != acting_primary.node_id),
                            )
                            .collect();

                        self.broadcaster.enqueue(ReplicaInvalidationTask {
                            key: key.to_string(),
                            scope,
                            replica_node_ids: remaining_replicas,
                            membership_epoch: self.get_membership_epoch(),
                        });

                        Ok(InvalidationOutcome {
                            canonical_node_id: acting_primary.node_id,
                            is_acting_primary: true,
                            suspect_node_id: Some(primary.node_id),
                            status,
                            reclaimed_bytes,
                        })
                    }
                    Err(DistError::Busy(id)) => {
                        // The acting primary is only locally saturated. Do not lose the invalidation: hand it
                        // to the broadcaster for every other member, the suspected primary included.
                        let mut all_ids = vec![primary.node_id];
                        all_ids.extend(self.invalidation_targets(key, primary.node_id).iter().map(|n| n.node_id));
                        self.broadcaster.enqueue(ReplicaInvalidationTask {
                            key: key.to_string(),
                            scope,
                            replica_node_ids: all_ids,
                            membership_epoch: self.get_membership_epoch(),
                        });
                        Err(DistError::Busy(id))
                    }
                    Err(failover_err) => {
                        acting_primary.set_state(NodeState::Suspect);
                        Err(failover_err)
                    }
                }
            }
        }
    }

    /// Autonomous anti-entropy reconciliation probe (SEC-4C-06, SEC-POA-02):
    /// Connects to a cluster peer, exchanges anti-entropy state digests, and replays
    /// missing invalidations or triggers partition flushes when history was overwritten.
    /// Only restores `NodeState::Active` upon complete verified reconciliation or safe flush.
    pub async fn reconcile_peer_anti_entropy(
        &self,
        peer_node: &VirtualNode,
        local_node: &VirtualNode,
    ) -> Result<crate::dist::frame::AntiEntropyStatus, DistError> {
        if local_node.get_state() == NodeState::Suspect {
            local_node.set_state(NodeState::Recovering);
        }
        if peer_node.get_state() == NodeState::Suspect {
            peer_node.set_state(NodeState::Recovering);
        }

        let membership_epoch = self.get_membership_epoch();

        let mut conn = Self::acquire_connection_to_node(&self.pool, &self.cluster_tls, peer_node, 1000).await?;
        let req_id = rand::random::<u64>();
        let seq = conn.sequence_number;
        conn.sequence_number = conn.sequence_number.wrapping_add(1);

        let highest_acked_seq = local_node
            .anti_entropy_tracker
            .lock()
            .unwrap()
            .get_highest_acked_seq(peer_node.node_id);
        let ring_digest = local_node.ring_buffer.lock().unwrap().compute_digest();

        let req_frame = match crate::dist::frame::PhantomMeshFrame::new_anti_entropy_sync_request(
            req_id,
            seq,
            membership_epoch,
            highest_acked_seq,
            local_node.node_id,
            ring_digest,
        ) {
            Ok(f) => f,
            Err(_) => {
                self.pool.discard(conn);
                return Err(DistError::ProtocolError);
            }
        };

        if crate::dist::frame::write_frame(&mut conn.stream, &req_frame).await.is_err() {
            self.pool.discard(conn);
            return Err(DistError::NodeUnreachable(peer_node.node_id));
        }

        let resp_res = tokio::time::timeout(
            Duration::from_millis(1000),
            crate::dist::frame::read_frame(&mut conn.stream),
        )
        .await;

        let resp_frame = match resp_res {
            Ok(Ok(f)) => f,
            _ => {
                self.pool.discard(conn);
                return Err(DistError::Timeout);
            }
        };

        if resp_frame.header.request_id != req_id
            || resp_frame.header.msg_type != crate::dist::frame::MessageType::AntiEntropySyncResponse
        {
            self.pool.discard(conn);
            return Err(DistError::ProtocolError);
        }

        let payload = match resp_frame.extract_anti_entropy_sync_response() {
            Ok(p) => p,
            Err(_) => {
                self.pool.discard(conn);
                return Err(DistError::ProtocolError);
            }
        };

        self.pool.checkin(conn);
        peer_node.note_reachable();

        match payload.status {
            crate::dist::frame::AntiEntropyStatus::InSync => {
                local_node
                    .cache
                    .counters
                    .anti_entropy_sync_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                local_node.set_state(NodeState::Active);
                peer_node.set_state(NodeState::Active);
                Ok(crate::dist::frame::AntiEntropyStatus::InSync)
            }
            crate::dist::frame::AntiEntropyStatus::ReplayBatch => {
                local_node.sync_anti_entropy_batch(&payload.records);
                local_node
                    .anti_entropy_tracker
                    .lock()
                    .unwrap()
                    .set_highest_acked_seq(peer_node.node_id, payload.range_end);

                // Convergence (PC-AUTO-037): a replay that was applied completely IS a successful
                // reconciliation. Waiting for a later `InSync` is not enough, because `InSync` also
                // requires equal ring digests, which two nodes that both send and receive
                // invalidations essentially never have. A truncated batch (or an open gap for this
                // peer) is not complete: the peer stays Recovering and the next tick continues from
                // `range_end`. compare_and_set never overwrites a concurrent Suspect.
                let truncated = crate::dist::anti_entropy::InvalidationRingBuffer::batch_may_be_truncated(&payload.records);
                let gaps_open = local_node.anti_entropy_tracker.lock().unwrap().has_gaps_for(peer_node.node_id);
                if !truncated && !gaps_open {
                    local_node.compare_and_set_state(NodeState::Recovering, NodeState::Active);
                    peer_node.compare_and_set_state(NodeState::Recovering, NodeState::Active);
                }
                Ok(crate::dist::frame::AntiEntropyStatus::ReplayBatch)
            }
            crate::dist::frame::AntiEntropyStatus::HistoryOverwritten => {
                local_node.flush_for_anti_entropy_overwrite();
                local_node
                    .anti_entropy_tracker
                    .lock()
                    .unwrap()
                    .set_highest_acked_seq(peer_node.node_id, payload.range_end);
                local_node.set_state(NodeState::Active);
                peer_node.set_state(NodeState::Active);
                Ok(crate::dist::frame::AntiEntropyStatus::HistoryOverwritten)
            }
        }
    }
}
