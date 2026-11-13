use crate::core::cache::PartitionedCache;
use crate::core::coalesce::Coalescer;
use crate::intel::telemetry::TelemetryCollector;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Joining,
    Active,
    Degraded,
    Suspect,
    Failed,
    Offline,
    Recovering,
}

pub struct VirtualNode {
    pub node_id: u8,
    pub state: Mutex<NodeState>,
    pub cache: Arc<PartitionedCache>,
    pub coalescer: Arc<Coalescer>,
    pub telemetry: Arc<TelemetryCollector>,
    pub cpu_utilization: Mutex<f32>,
    pub memory_used_bytes: Mutex<u64>,
    pub addr: Mutex<Option<SocketAddr>>,
    pub shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    pub artificial_delay_ms: Mutex<u64>,
    pub cluster_tls: Mutex<Option<Arc<crate::security::ClusterTlsConfig>>>,
    pub session_tracker: Arc<crate::dist::session::SessionTracker>,
    pub ring_buffer: Arc<Mutex<crate::dist::anti_entropy::InvalidationRingBuffer>>,
    pub anti_entropy_tracker: Arc<Mutex<crate::dist::anti_entropy::AntiEntropyTracker>>,
    pub is_shared_cache: bool,
    pub consecutive_failures: std::sync::atomic::AtomicU32,
    pub consecutive_successes: std::sync::atomic::AtomicU32,
    /// Consecutive failed attempts to reach this node (connect, handshake, frame exchange) since the
    /// last successful exchange. Observation only: it does not change `state` or any routing (PC-AUTO-044).
    pub unreachable_streak: std::sync::atomic::AtomicU32,
}

impl VirtualNode {
    pub fn new(node_id: u8) -> Self {
        Self {
            node_id,
            state: Mutex::new(NodeState::Joining),
            cache: Arc::new(PartitionedCache::new(4)),
            coalescer: Arc::new(Coalescer::new(1000)),
            telemetry: Arc::new(TelemetryCollector::new(1000)),
            cpu_utilization: Mutex::new(0.0),
            memory_used_bytes: Mutex::new(0),
            addr: Mutex::new(None),
            shutdown_tx: Mutex::new(None),
            artificial_delay_ms: Mutex::new(0),
            cluster_tls: Mutex::new(None),
            session_tracker: Arc::new(crate::dist::session::SessionTracker::new()),
            ring_buffer: Arc::new(Mutex::new(
                crate::dist::anti_entropy::InvalidationRingBuffer::default(),
            )),
            anti_entropy_tracker: Arc::new(Mutex::new(
                crate::dist::anti_entropy::AntiEntropyTracker::default(),
            )),
            is_shared_cache: false,
            consecutive_failures: std::sync::atomic::AtomicU32::new(0),
            consecutive_successes: std::sync::atomic::AtomicU32::new(0),
            unreachable_streak: std::sync::atomic::AtomicU32::new(0),
        }
    }

    /// Creates a VirtualNode unified with an existing shared serving cache (C-01 / ADR-003).
    pub fn new_with_cache(node_id: u8, cache: Arc<PartitionedCache>) -> Self {
        let mut node = Self::new(node_id);
        node.cache = cache;
        node.is_shared_cache = true;
        node
    }

    /// Attaches an existing shared serving cache to this node (C-01 / ADR-003).
    pub fn set_cache(&mut self, cache: Arc<PartitionedCache>) {
        self.cache = cache;
        self.is_shared_cache = true;
    }

    /// Replays a batch of invalidation records received during anti-entropy catchup,
    /// purging local cache entries and tracking metrics.
    pub fn sync_anti_entropy_batch(
        &self,
        records: &[crate::dist::frame::InvalidateRecord],
    ) -> usize {
        let mut count = 0;
        for rec in records {
            match rec.scope {
                crate::dist::frame::InvalidationScope::ExactKey => {
                    let _ = self.cache.try_delete_canonical(&rec.key);
                }
                crate::dist::frame::InvalidationScope::Tenant => {
                    let _ = self.cache.try_invalidate_tenant(&rec.key);
                }
                crate::dist::frame::InvalidationScope::FullCache => {
                    let _ = self.cache.try_invalidate_all();
                }
            }
            count += 1;
        }
        self.cache
            .counters
            .anti_entropy_keys_evicted
            .fetch_add(count as u64, std::sync::atomic::Ordering::Relaxed);
        self.cache
            .counters
            .anti_entropy_sync_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        count
    }

    /// Executes a full cache flush when unrecoverable ring buffer sequence overwrites occur,
    /// advancing global epochs and preventing stale entry resurrection.
    pub fn flush_for_anti_entropy_overwrite(&self) {
        let _ = self.cache.try_invalidate_all();
        self.cache
            .counters
            .anti_entropy_full_flushes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.cache
            .counters
            .anti_entropy_sync_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Actively evicts all open sessions for a revoked peer node ID (Layer 5 revocation).
    pub fn evict_peer(&self, peer_node_id: u8) -> usize {
        self.session_tracker.evict_peer(peer_node_id)
    }

    pub fn set_cluster_tls(&self, config: Arc<crate::security::ClusterTlsConfig>) {
        *self.cluster_tls.lock().unwrap() = Some(config);
    }

    pub fn get_cluster_tls(&self) -> Option<Arc<crate::security::ClusterTlsConfig>> {
        self.cluster_tls.lock().unwrap().clone()
    }

    pub fn set_state(&self, new_state: NodeState) {
        let mut guard = self.state.lock().unwrap();
        *guard = new_state;
    }

    pub fn get_state(&self) -> NodeState {
        *self.state.lock().unwrap()
    }

    /// Atomically moves `expected` -> `new`; returns false (state untouched) if the current state
    /// is anything else, e.g. because a concurrent failure marked the node Suspect again.
    pub fn compare_and_set_state(&self, expected: NodeState, new: NodeState) -> bool {
        let mut guard = self.state.lock().unwrap();
        if *guard == expected {
            *guard = new;
            true
        } else {
            false
        }
    }

    /// Records a failed health probe. Transitions Active -> Suspect on 1 failure,
    /// and Suspect -> Failed when consecutive failures reach or exceed `failure_threshold`.
    pub fn record_probe_failure(&self, failure_threshold: u32) -> NodeState {
        use std::sync::atomic::Ordering;
        self.consecutive_successes.store(0, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        let mut guard = self.state.lock().unwrap();
        if failures >= failure_threshold {
            *guard = NodeState::Failed;
        } else if *guard == NodeState::Active || *guard == NodeState::Joining {
            *guard = NodeState::Suspect;
        }
        *guard
    }

    /// Records a successful health probe. Transitions Failed/Offline -> Recovering,
    /// and Recovering/Suspect -> Active once consecutive successes reach `recovery_threshold`.
    pub fn record_probe_success(&self, recovery_threshold: u32) -> NodeState {
        use std::sync::atomic::Ordering;
        self.consecutive_failures.store(0, Ordering::Relaxed);
        let successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
        let mut guard = self.state.lock().unwrap();
        match *guard {
            NodeState::Failed | NodeState::Offline => {
                *guard = NodeState::Recovering;
            }
            NodeState::Recovering | NodeState::Suspect | NodeState::Joining | NodeState::Degraded => {
                if successes >= recovery_threshold {
                    *guard = NodeState::Active;
                }
            }
            NodeState::Active => {}
        }
        *guard
    }

    /// A connection or frame exchange with this node failed.
    pub fn note_unreachable(&self) {
        let _ = self.unreachable_streak.fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |v| Some(v.saturating_add(1)),
        );
    }

    /// A frame exchange with this node succeeded.
    pub fn note_reachable(&self) {
        self.unreachable_streak.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// `unreachable_streak` at or above which a peer is reported as unreachable.
    pub const UNREACHABLE_STREAK_THRESHOLD: u32 = 3;

    /// Observed reachability label: `unreachable` after repeated failures, `reachable` after a success
    /// with no failure since, otherwise `degraded` (1 or 2 recent failures).
    pub fn reachability(&self) -> &'static str {
        let s = self.unreachable_streak.load(std::sync::atomic::Ordering::Relaxed);
        if s >= Self::UNREACHABLE_STREAK_THRESHOLD {
            "unreachable"
        } else if s > 0 {
            "degraded"
        } else {
            "reachable"
        }
    }

    pub fn set_addr(&self, addr: SocketAddr) {
        *self.addr.lock().unwrap() = Some(addr);
    }

    pub fn get_addr(&self) -> Option<SocketAddr> {
        *self.addr.lock().unwrap()
    }

    pub fn set_artificial_delay(&self, delay_ms: u64) {
        *self.artificial_delay_ms.lock().unwrap() = delay_ms;
    }

    /// Stops the TCP listener, cancels all active sessions, and drops sockets.
    pub fn stop_tcp_service(&self) {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        let _ = self.session_tracker.evict_all();
    }

    /// Starts a TCP listener on this virtual node to process distributed network requests.
    /// In Stage 3A, if `cluster_tls` is configured, inbound connections require mutual TLS
    /// with strict SAN identity extraction, RFC 1035 syntax validation, cluster ID check,
    /// and membership registry validation.
    /// Handshakes are bounded by a 64-permit semaphore and a 1000ms timeout.
    /// If non-loopback and cluster_tls is missing, startup fails closed.
    pub async fn start_tcp_service(
        self: &Arc<Self>,
        bind_addr: &str,
    ) -> std::io::Result<SocketAddr> {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        let addr = listener.local_addr()?;
        self.set_addr(addr);

        let cluster_tls = self.get_cluster_tls();
        if !addr.ip().is_loopback() && cluster_tls.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "Fail-Closed: Node {} configured to bind to non-loopback address {} without cluster mTLS certificates.",
                    self.node_id, addr
                ),
            ));
        }

        self.set_state(NodeState::Active);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

        let (stop_notify_tx, stop_notify_rx) = tokio::sync::watch::channel(false);

        // SEC-3C-03: Wire membership registry revocation directly to SessionTracker eviction
        if let Some(ref tls_cfg) = cluster_tls {
            let mut rev_rx = tls_cfg.subscribe_revocations();
            let mut stop_rx = stop_notify_rx.clone();
            let node_evict = self.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = stop_rx.changed() => break,
                        res = rev_rx.recv() => {
                            match res {
                                Ok(revoked_id) => {
                                    node_evict.evict_peer(revoked_id);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            }
                        }
                    }
                }
            });
        }

        let handshake_semaphore = Arc::new(tokio::sync::Semaphore::new(64));
        let node = self.clone();

        tokio::spawn(async move {
            loop {
                let conn = tokio::select! {
                    _ = &mut shutdown_rx => {
                        let _ = stop_notify_tx.send(true);
                        // Drop listener and exit immediately
                        break;
                    }
                    res = listener.accept() => {
                        match res {
                            Ok(c) => c,
                            Err(_) => break,
                        }
                    }
                };

                let (socket, _) = conn;

                let current_state = node.get_state();
                if current_state == NodeState::Failed || current_state == NodeState::Offline {
                    // Abruptly drop socket to simulate node offline or network partition
                    drop(socket);
                    continue;
                }

                let node_clone = node.clone();
                let tls_opt = cluster_tls.clone();
                let sem_clone = handshake_semaphore.clone();

                tokio::spawn(async move {
                    if let Some(tls_cfg) = tls_opt {
                        // Bounded concurrent handshakes via semaphore
                        let permit = match sem_clone.try_acquire_owned() {
                            Ok(p) => p,
                            Err(_) => {
                                // Handshake capacity saturated; fail closed and drop socket immediately
                                drop(socket);
                                return;
                            }
                        };

                        let acceptor = tls_cfg.server_acceptor();
                        // 1000ms TLS handshake timeout (AT-P3-10)
                        let handshake_res = tokio::time::timeout(
                            Duration::from_millis(1000),
                            acceptor.accept(socket),
                        )
                        .await;

                        let tls_stream = match handshake_res {
                            Ok(Ok(stream)) => stream,
                            _ => {
                                // Handshake timed out, plaintext received, untrusted CA, or expired cert
                                drop(permit);
                                return;
                            }
                        };

                        // Release handshake permit immediately once TLS handshake succeeds
                        drop(permit);

                        // Extract and validate peer certificate (Layers 2-4)
                        let leaf_der = {
                            let (_, server_conn) = tls_stream.get_ref();
                            let peer_certs = match server_conn.peer_certificates() {
                                Some(certs) if !certs.is_empty() => certs,
                                _ => return, // Layer 1: Client certificate missing
                            };
                            peer_certs[0].as_ref().to_vec()
                        };

                        // Layer 2: Extract DNS SANs
                        let sans = match crate::security::cluster::extract_sans_from_der(&leaf_der) {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        // SEC-3A-03: Require exactly one DNS SAN for node identity.
                        // Reject zero SANs, multiple DNS SANs, and ambiguous identity certificates.
                        if sans.len() != 1 {
                            return;
                        }

                        let identity = match crate::security::cluster::parse_san_node_identity(&sans[0]) {
                            Ok(id) => id,
                            Err(_) => return, // Malformed identity / illegal SAN syntax
                        };

                        // Layers 3 & 4: Validate identity and fingerprint against registry
                        {
                            let reg_guard = match tls_cfg.registry.read() {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            if reg_guard.validate_peer(&identity, &leaf_der).is_err() {
                                return; // Admission rejected: cluster mismatch, unknown node, inactive node, or fingerprint mismatch
                            }
                        }

                        // Layer 5: Register active session in SessionTracker
                        let (session_id, cancel_rx) = node_clone.session_tracker.register(identity.node_id);

                        // Process requests over reusable encrypted TLS stream
                        process_binary_cache_stream(
                            node_clone,
                            tls_stream,
                            session_id,
                            cancel_rx,
                            Some(identity),
                            Some(leaf_der),
                        )
                        .await;
                    } else {
                        // Loopback development mode plaintext fallback
                        let (session_id, cancel_rx) = node_clone.session_tracker.register(0);
                        process_binary_cache_stream(node_clone, socket, session_id, cancel_rx, None, None).await;
                    }
                });
            }
        });

        Ok(addr)
    }
}

async fn process_binary_cache_stream<S>(
    node: Arc<VirtualNode>,
    mut stream: S,
    session_id: u64,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
    peer_identity: Option<crate::security::cluster::NodeIdentity>,
    peer_cert_der: Option<Vec<u8>>,
)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use crate::dist::frame::{read_frame, write_frame, MessageType, PhantomMeshFrame};

    let delay = *node.artificial_delay_ms.lock().unwrap();

    // Reusable connection loop: handles multiple sequential requests and 5-second keepalive heartbeats
    // over the same pooled connection until client disconnects, socket errors, or SessionTracker evicts peer.
    loop {
        let frame_res = tokio::select! {
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    // Layer 5 Active Revocation: Session cancelled by SessionTracker
                    break;
                }
                continue;
            }
            res = read_frame(&mut stream) => res,
        };

        let frame = match frame_res {
            Ok(f) => f,
            Err(_) => break, // Clean EOF or malformed frame: exit loop and drop socket
        };

        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        // Layer 4 & Layer 5: Re-validate peer identity against registry on every request over pooled connection
        if let (Some(ref identity), Some(ref cert_der)) = (&peer_identity, &peer_cert_der) {
            if let Some(tls_cfg) = node.get_cluster_tls() {
                let is_valid = if let Ok(reg) = tls_cfg.registry.read() {
                    reg.validate_peer(identity, cert_der).is_ok()
                } else {
                    false
                };
                if !is_valid {
                    // Revoked or quarantined peer! Immediately abort connection.
                    break;
                }
            }
        }

        match frame.header.msg_type {
            MessageType::GetRequest => {
                let key = match frame.extract_key() {
                    Ok(k) => k,
                    Err(_) => break, // Invalid UTF-8 or key > 4096 bytes
                };

                let cache_key = if let Some(ck) = crate::core::cache::CacheKey::from_canonical_string(key) {
                    ck
                } else {
                    crate::core::cache::CacheKey::new(None, key, None)
                };

                // M-2 (round-3 review): on private-cache harness nodes, a peer-supplied key must not
                // start with '/': HTTP keys are `/path` under a tenant, so a peer key '/x' would address
                // the `__default__` tenant's HTTP entry '/x'. On shared-cache nodes, canonical keys
                // (e.g. `__default__:/x`) or non-slash keys are validated via `cache_key.validate()`.
                if (!node.is_shared_cache && key.starts_with('/')) || cache_key.validate().is_err() {
                    let resp = match PhantomMeshFrame::new_get_response(
                        frame.header.request_id,
                        frame.header.sequence_number.wrapping_add(1),
                        "ERR 400",
                    ) {
                        Ok(f) => f,
                        Err(_) => break,
                    };
                    if write_frame(&mut stream, &resp).await.is_err() {
                        break;
                    }
                    continue;
                }
                let now_ms = crate::intel::diagnostics::DiagnosticBuffer::now_ms();

                let current_state = node.get_state();
                let body = if current_state == NodeState::Recovering || current_state == NodeState::Suspect {
                    // SEC-POA-04: Nodes undergoing recovery or in suspect state must NEVER serve potentially
                    // stale cache hits. They bypass the local cache and fetch directly from origin as MISS.
                    if node.is_shared_cache {
                        "MISS".to_string()
                    } else {
                        let val = format!("val_node_{}", node.node_id);
                        format!("MISS {}", val)
                    }
                } else if node.is_shared_cache {
                    // SETBACK-11 / DR-029: Genuine peer GET semantics on production shared-cache nodes.
                    // Perform a non-mutating read-only cache lookup. Never fabricate synthetic entries
                    // into the serving cache, pollute counters, or trigger unwanted evictions on misses!
                    if let Some(res) = node.cache.get_resource(&cache_key, now_ms) {
                        format!("HIT {}", String::from_utf8_lossy(&res.body))
                    } else {
                        "MISS".to_string()
                    }
                } else {
                    match node.cache.begin_fetch_atomic(&cache_key, now_ms) {
                        crate::core::cache::FetchRegistrationResult::Hit(res) => {
                            format!("HIT {}", String::from_utf8_lossy(&res.body))
                        }
                        crate::core::cache::FetchRegistrationResult::Initiate(reg) => {
                            let val = format!("val_node_{}", node.node_id);
                            let res = Arc::new(crate::core::cache::CachedResource {
                                status: 200,
                                content_type: "text/plain".to_string(),
                                body: Arc::new(val.as_bytes().to_vec()),
                                etag: None,
                                created_at_ms: now_ms,
                                ttl_ms: 60_000,
                            });
                            let guard = reg.into_guard();
                            guard.complete(crate::core::coalesce::FinalizeOutcome::Success(res));
                            format!("MISS {}", val)
                        }
                        crate::core::cache::FetchRegistrationResult::Wait(rx) => match rx.await {
                            Ok(Ok(res)) => {
                                format!("COALESCED {}", String::from_utf8_lossy(&res.body))
                            }
                            _ => "ERR 500".to_string(),
                        },
                        crate::core::cache::FetchRegistrationResult::Rejected(_) => "ERR 503".to_string(),
                    }
                };

                let resp_frame = match PhantomMeshFrame::new_get_response(
                    frame.header.request_id,
                    frame.header.sequence_number.wrapping_add(1),
                    &body,
                ) {
                    Ok(f) => f,
                    Err(_) => break,
                };

                if write_frame(&mut stream, &resp_frame).await.is_err() {
                    break;
                }
            }
            MessageType::HeartbeatRequest => {
                let resp_frame = match PhantomMeshFrame::new(
                    MessageType::HeartbeatResponse,
                    0,
                    frame.header.request_id,
                    frame.header.sequence_number.wrapping_add(1),
                    Vec::new(),
                ) {
                    Ok(f) => f,
                    Err(_) => break,
                };

                if write_frame(&mut stream, &resp_frame).await.is_err() {
                    break;
                }
            }
            MessageType::InvalidateRequest => {
                let payload = match frame.extract_invalidate_request() {
                    Ok(p) => p,
                    Err(_) => break, // Malformed payload, invalid scope, or bad UTF-8: terminate connection
                };

                // L3: the originator claimed in the frame must be the authenticated TLS peer. Without
                // this a member could poison another member's sequence space in our tracker/ring.
                if let Some(ref identity) = peer_identity {
                    if payload.originator_id != identity.node_id {
                        break;
                    }
                }

                // Track peer stream sequence and record gaps for anti-entropy reconciliation
                node.anti_entropy_tracker
                    .lock()
                    .unwrap()
                    .record_inbound_seq(
                        payload.originator_id,
                        payload.stream_seq,
                        Some(&node.cache.counters),
                    );

                // Record the received invalidation for accounting only: replay serves just the records
                // THIS node originated for the requester, never inbound ones.
                node.ring_buffer.lock().unwrap().push_inbound(
                    crate::dist::frame::InvalidateRecord {
                        stream_seq: payload.stream_seq,
                        scope: payload.scope,
                        key: payload.key.clone(),
                    },
                );

                let (status, reclaimed_bytes) = match payload.scope {
                    crate::dist::frame::InvalidationScope::ExactKey => {
                        match node.cache.try_delete_canonical(&payload.key) {
                            Ok((was_present, reclaimed)) => {
                                let status = if was_present {
                                    crate::dist::frame::InvalidationStatus::SuccessEvicted
                                } else {
                                    crate::dist::frame::InvalidationStatus::SuccessNotPresent
                                };
                                (status, reclaimed)
                            }
                            Err(_) => (crate::dist::frame::InvalidationStatus::InternalError, 0),
                        }
                    }
                    crate::dist::frame::InvalidationScope::Tenant => {
                        match node.cache.try_invalidate_tenant(&payload.key) {
                            Ok(_) => (crate::dist::frame::InvalidationStatus::SuccessEvicted, 0),
                            Err(_) => (crate::dist::frame::InvalidationStatus::InternalError, 0),
                        }
                    }
                    crate::dist::frame::InvalidationScope::FullCache => {
                        match node.cache.try_invalidate_all() {
                            Ok(_) => (crate::dist::frame::InvalidationStatus::SuccessEvicted, 0),
                            Err(_) => (crate::dist::frame::InvalidationStatus::InternalError, 0),
                        }
                    }
                };

                let resp_frame = match PhantomMeshFrame::new_invalidate_ack(
                    frame.header.request_id,
                    frame.header.sequence_number.wrapping_add(1),
                    payload.membership_epoch,
                    payload.stream_seq,
                    node.node_id,
                    status,
                    reclaimed_bytes,
                ) {
                    Ok(f) => f,
                    Err(_) => break,
                };

                if write_frame(&mut stream, &resp_frame).await.is_err() {
                    break;
                }
            }
            MessageType::AntiEntropySyncRequest => {
                let payload = match frame.extract_anti_entropy_sync_request() {
                    Ok(p) => p,
                    Err(_) => break, // Malformed payload: terminate connection
                };

                // SEC-4C-02: Cryptographic peer identity & membership epoch verification
                if let Some(ref identity) = peer_identity {
                    if payload.sender_node_id != identity.node_id {
                        break; // Spoofed sender node ID
                    }
                }
                if let Some(tls_cfg) = node.get_cluster_tls() {
                    if let Ok(reg) = tls_cfg.registry.read() {
                        if payload.membership_epoch != reg.version {
                            break; // Obsolete or mismatched cluster membership epoch
                        }
                    }
                }

                let (status, range_start, range_end, records) = {
                    let ring = node.ring_buffer.lock().unwrap();
                    let local_digest = ring.compute_digest();

                    // SEC-4C-03: InSync requires matching newest_seq and matching digests,
                    // or both nodes in clean initial state (highest_acked_seq == 0 and ring is empty).
                    let both_clean = ring.is_empty() && payload.highest_acked_seq == 0 && payload.ring_digest == local_digest;
                    let matches_current = !ring.is_empty() && payload.highest_acked_seq == ring.newest_seq() && payload.ring_digest == local_digest;

                    if both_clean || matches_current {
                        (
                            crate::dist::frame::AntiEntropyStatus::InSync,
                            ring.oldest_seq(),
                            ring.newest_seq(),
                            Vec::new(),
                        )
                    } else {
                        // Replay only the stream this node sent to the requester (its own sequence space).
                        let replay = if ring.is_empty() {
                            ring.get_replay_batch(payload.highest_acked_seq)
                        } else {
                            ring.get_replay_batch_for(payload.sender_node_id, payload.highest_acked_seq)
                        };
                        match replay {
                            Ok(batch) => {
                                let start = payload.highest_acked_seq.saturating_add(1);
                                // SEC-4C-04: range_end must reflect the actual last record in this batch,
                                // preventing receiver from prematurely advancing past unreceived records.
                                let end = batch.iter().map(|r| r.stream_seq).max().unwrap_or(payload.highest_acked_seq);
                                (
                                    crate::dist::frame::AntiEntropyStatus::ReplayBatch,
                                    start,
                                    end,
                                    batch,
                                )
                            }
                            Err(status) => (
                                status,
                                ring.oldest_seq(),
                                // Range end in the requester's own sequence space (never another stream's).
                                if ring.is_empty() { ring.newest_seq() } else { ring.newest_seq_for(payload.sender_node_id) },
                                Vec::new(),
                            ),
                        }
                    }
                };

                let resp_frame = match PhantomMeshFrame::new_anti_entropy_sync_response(
                    frame.header.request_id,
                    frame.header.sequence_number.wrapping_add(1),
                    status,
                    range_start,
                    range_end,
                    &records,
                ) {
                    Ok(f) => f,
                    Err(_) => break,
                };

                if write_frame(&mut stream, &resp_frame).await.is_err() {
                    break;
                }
            }
            MessageType::PeerReadRequest => {
                // PC-AUTO-041/042: peer read with full resource metadata (see dist/frame.rs).
                let (format, key) = match frame.extract_peer_read_request() {
                    Ok(v) => v,
                    Err(_) => break, // Malformed payload / key too long: terminate connection
                };
                let resp = if format != crate::dist::frame::PEER_ENTRY_FORMAT {
                    PhantomMeshFrame::new_peer_read_unsupported(
                        frame.header.request_id,
                        frame.header.sequence_number.wrapping_add(1),
                    )
                } else {
                    let entry = peer_read_lookup(&node, key);
                    let hit = entry.as_ref().and_then(|e| {
                        PhantomMeshFrame::new_peer_read_hit(
                            frame.header.request_id,
                            frame.header.sequence_number.wrapping_add(1),
                            e,
                        )
                        .ok()
                    });
                    match hit {
                        Some(f) => Ok(f),
                        // No fresh entry, a node that must not serve, or an entry too large for one frame.
                        None => PhantomMeshFrame::new_peer_read_miss(
                            frame.header.request_id,
                            frame.header.sequence_number.wrapping_add(1),
                        ),
                    }
                };
                let resp = match resp {
                    Ok(f) => f,
                    Err(_) => break,
                };
                if write_frame(&mut stream, &resp).await.is_err() {
                    break;
                }
            }
            _ => {
                // Unsupported message type: terminate connection
                break;
            }
        }
    }

    // Cleanly unregister session from SessionTracker upon socket closure
    node.session_tracker.unregister(session_id);
}

/// Looks up a FRESH entry for a peer read. Only shared-cache nodes (the gateway's own serving cache)
/// answer; a Recovering or Suspect node never serves (SEC-POA-04). Returns relative age and remaining
/// lifetime, never absolute timestamps.
fn peer_read_lookup(node: &Arc<VirtualNode>, key: &str) -> Option<crate::dist::frame::PeerEntry> {
    if !node.is_shared_cache {
        return None;
    }
    let state = node.get_state();
    if state == NodeState::Recovering || state == NodeState::Suspect {
        return None;
    }
    let cache_key = crate::core::cache::CacheKey::from_canonical_string(key)
        .unwrap_or_else(|| crate::core::cache::CacheKey::new(None, key, None));
    if cache_key.validate().is_err() {
        return None;
    }
    let now_ms = crate::intel::diagnostics::DiagnosticBuffer::now_ms();
    let (res, meta) = node.cache.get_resource_with_metadata(&cache_key, now_ms)?;
    let expiry = res.created_at_ms.saturating_add(res.ttl_ms);
    let remaining_ms = expiry.saturating_sub(now_ms);
    if remaining_ms == 0 {
        return None;
    }
    Some(crate::dist::frame::PeerEntry {
        status: res.status,
        content_type: res.content_type.clone(),
        etag: res.etag.clone(),
        last_modified: meta.as_ref().and_then(|m| m.last_modified.clone()),
        must_revalidate: meta.as_ref().map(|m| m.must_revalidate).unwrap_or(false),
        stale_while_revalidate_ms: meta.as_ref().and_then(|m| m.stale_while_revalidate_ms),
        stale_if_error_ms: meta.as_ref().and_then(|m| m.stale_if_error_ms),
        date_ms: meta.as_ref().and_then(|m| m.date_ms),
        initial_age_ms: meta.as_ref().map(|m| m.initial_age_ms).unwrap_or(0),
        age_ms: now_ms.saturating_sub(res.created_at_ms),
        remaining_ms,
        body: (*res.body).clone(),
    })
}
