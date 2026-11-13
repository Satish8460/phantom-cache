use crate::dist::frame::{
    read_frame, write_frame, FrameError, MessageType, PhantomMeshFrame,
};
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Abstraction over a cluster network transport stream (encrypted TLS or loopback plaintext).
pub enum ClusterStream {
    Tls(Box<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>),
    Plain(tokio::net::TcpStream),
}

impl AsyncRead for ClusterStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClusterStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// A pooled connection to an authenticated cluster node.
pub struct PooledConnection {
    pub node_id: u8,
    pub stream: ClusterStream,
    pub created_at: Instant,
    pub last_used_at: Instant,
    pub sequence_number: u64,
    pub epoch: u64,
    /// How many times this connection has been returned to the pool (0 = freshly opened). A reused
    /// connection may be stale (peer restarted, heartbeat has not noticed yet).
    pub use_count: u32,
}

impl PooledConnection {
    pub fn new(node_id: u8, stream: ClusterStream, epoch: u64) -> Self {
        let now = Instant::now();
        Self {
            node_id,
            stream,
            created_at: now,
            last_used_at: now,
            sequence_number: 1,
            epoch,
            use_count: 0,
        }
    }
}

/// Configuration options for the inter-node connection pool.
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub max_idle_per_node: usize,
    pub max_total_per_node: usize,
    pub idle_timeout: Duration,
    pub heartbeat_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_node: 4,
            max_total_per_node: 8,
            idle_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
        }
    }
}

/// Result of an atomic checkout or capacity reservation attempt.
pub enum AcquisitionResult {
    /// An existing healthy idle connection was checked out.
    Idle(PooledConnection),
    /// No idle connection was available, but capacity was atomically reserved.
    /// The caller MUST convert this reservation into a connection via `into_connection`
    /// or drop it, which safely decrements the active count exactly once.
    Reserved(ConnectionReservation),
    /// The pool is full for this node (`active_count >= max_total_per_node`).
    Full,
}

/// RAII reservation guard for an active connection slot.
/// Guarantees that if a network connect, TLS handshake, or task is cancelled or fails,
/// the active connection counter is decremented exactly once upon drop.
pub struct ConnectionReservation {
    node_id: u8,
    pool: Arc<TlsConnectionPool>,
    epoch: u64,
    committed: bool,
}

impl ConnectionReservation {
    pub fn into_connection(mut self, stream: ClusterStream) -> PooledConnection {
        self.committed = true;
        PooledConnection::new(self.node_id, stream, self.epoch)
    }

    pub fn node_id(&self) -> u8 {
        self.node_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl Drop for ConnectionReservation {
    fn drop(&mut self) {
        if !self.committed {
            self.pool.decrement_active(self.node_id);
        }
    }
}

/// Bounded, thread-safe connection pool for inter-node TLS cluster communication.
/// Enforces exclusive acquire-release semantics: each connection checked out from the pool
/// is owned exclusively by one asynchronous task, completely preventing frame interleaving
/// or response cross-talk.
pub struct TlsConnectionPool {
    config: PoolConfig,
    idle_conns: Mutex<HashMap<u8, Vec<PooledConnection>>>,
    active_counts: Mutex<HashMap<u8, usize>>,
    node_epochs: Mutex<HashMap<u8, u64>>,
    heartbeat_shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl TlsConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            idle_conns: Mutex::new(HashMap::new()),
            active_counts: Mutex::new(HashMap::new()),
            node_epochs: Mutex::new(HashMap::new()),
            heartbeat_shutdown_tx: Mutex::new(None),
        }
    }

    /// Returns the current eviction epoch for a node ID.
    pub fn current_epoch(&self, node_id: u8) -> u64 {
        let mut epochs = self.node_epochs.lock().unwrap();
        *epochs.entry(node_id).or_insert(1)
    }

    /// Returns (idle_count, active_count) for a specific node ID (WP8 observability).
    pub fn get_pool_counts(&self, node_id: u8) -> (usize, usize) {
        let idle = self.idle_conns.lock().unwrap().get(&node_id).map(|v| v.len()).unwrap_or(0);
        let active = self.active_counts.lock().unwrap().get(&node_id).copied().unwrap_or(0);
        (idle, active)
    }

    /// Atomically checks out an idle, healthy connection or reserves capacity under `max_total_per_node`.
    /// Guarantees that `max_total_per_node` cannot be exceeded by concurrent callers.
    pub fn checkout_or_reserve(self: &Arc<Self>, node_id: u8) -> AcquisitionResult {
        let cur_epoch = self.current_epoch(node_id);
        let now = Instant::now();

        let mut idle_guard = self.idle_conns.lock().unwrap();
        let idle_list = idle_guard.entry(node_id).or_default();

        while let Some(conn) = idle_list.pop() {
            if conn.epoch == cur_epoch && now.duration_since(conn.last_used_at) <= self.config.idle_timeout {
                return AcquisitionResult::Idle(conn);
            }
            // Expired or stale epoch connection dropped; decrement active count
            self.decrement_active(node_id);
        }

        let mut count_guard = self.active_counts.lock().unwrap();
        let count = count_guard.entry(node_id).or_insert(0);
        if *count < self.config.max_total_per_node {
            *count += 1;
            AcquisitionResult::Reserved(ConnectionReservation {
                node_id,
                pool: self.clone(),
                epoch: cur_epoch,
                committed: false,
            })
        } else {
            AcquisitionResult::Full
        }
    }

    /// Attempts to check out an idle, healthy connection for the target node.
    /// Connections exceeding `idle_timeout` or belonging to an older eviction epoch are pruned.
    pub fn checkout(&self, node_id: u8) -> Option<PooledConnection> {
        let cur_epoch = self.current_epoch(node_id);
        let mut idle_guard = self.idle_conns.lock().unwrap();
        let idle_list = idle_guard.entry(node_id).or_default();

        let now = Instant::now();
        while let Some(conn) = idle_list.pop() {
            if conn.epoch == cur_epoch && now.duration_since(conn.last_used_at) <= self.config.idle_timeout {
                return Some(conn);
            }
            self.decrement_active(node_id);
        }

        None
    }

    /// Checks if a new connection is permitted under `max_total_per_node`.
    /// If permitted, increments the active count and returns true.
    pub fn try_increment_active(&self, node_id: u8) -> bool {
        let mut count_guard = self.active_counts.lock().unwrap();
        let count = count_guard.entry(node_id).or_insert(0);
        if *count < self.config.max_total_per_node {
            *count += 1;
            true
        } else {
            false
        }
    }

    /// Decrements the active connection count for a node when a connection is dropped or discarded.
    pub fn decrement_active(&self, node_id: u8) {
        let mut count_guard = self.active_counts.lock().unwrap();
        if let Some(count) = count_guard.get_mut(&node_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }

    /// Returns a healthy connection to the idle pool for reuse.
    /// If the connection was checked out before a node eviction occurred (`conn.epoch != current_epoch`),
    /// it is dropped immediately and NOT returned to the idle pool (SEC-3C-04).
    /// If the idle pool is full, the connection is dropped and closed cleanly.
    pub fn checkin(&self, mut conn: PooledConnection) {
        let node_id = conn.node_id;
        let cur_epoch = self.current_epoch(node_id);

        // SEC-3C-04: Drop stale connections from prior eviction epochs
        if conn.epoch != cur_epoch {
            drop(conn);
            self.decrement_active(node_id);
            return;
        }

        conn.last_used_at = Instant::now();
        conn.use_count = conn.use_count.saturating_add(1);

        let mut idle_guard = self.idle_conns.lock().unwrap();
        let idle_list = idle_guard.entry(node_id).or_default();

        if idle_list.len() < self.config.max_idle_per_node {
            idle_list.push(conn);
        } else {
            // Pool full for this node; drop connection and decrement active count
            drop(conn);
            self.decrement_active(node_id);
        }
    }

    /// Discards an unhealthy or failed connection, closing it and updating active counts.
    pub fn discard(&self, conn: PooledConnection) {
        let node_id = conn.node_id;
        drop(conn);
        self.decrement_active(node_id);
    }

    /// Evicts all idle connections associated with a node ID (e.g. on peer failure or revocation)
    /// and increments the node's eviction epoch so that in-flight checked-out connections are
    /// dropped upon checkin (SEC-3C-04).
    pub fn evict_node(&self, node_id: u8) {
        // SEC-3C-04: Invalidate in-flight checked-out connections via epoch advance
        {
            let mut epochs = self.node_epochs.lock().unwrap();
            let e = epochs.entry(node_id).or_insert(1);
            *e = e.wrapping_add(1);
        }

        let mut idle_guard = self.idle_conns.lock().unwrap();
        if let Some(idle_list) = idle_guard.remove(&node_id) {
            let count = idle_list.len();
            drop(idle_list);
            let mut count_guard = self.active_counts.lock().unwrap();
            if let Some(c) = count_guard.get_mut(&node_id) {
                *c = c.saturating_sub(count);
            }
        }
    }

    /// Starts a lifecycle-managed background heartbeat worker (SEC-3C-02).
    /// Periodically probes idle pooled connections, verifying that the peer responds
    /// with a valid HeartbeatResponse, refreshing `last_used_at`, and pruning failed connections.
    pub fn start_heartbeat_worker(self: &Arc<Self>) {
        if tokio::runtime::Handle::try_current().is_err() {
            // No active Tokio runtime (e.g. running in synchronous unit tests like wp3_tests); skip spawning worker
            return;
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        {
            let mut guard = self.heartbeat_shutdown_tx.lock().unwrap();
            if guard.is_some() {
                return; // Already running
            }
            *guard = Some(shutdown_tx);
        }

        let pool = Arc::downgrade(self);
        let interval_dur = self.config.heartbeat_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_dur);
            ticker.tick().await; // First tick resolves immediately

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    _ = ticker.tick() => {
                        let strong_pool = match pool.upgrade() {
                            Some(p) => p,
                            None => break,
                        };
                        strong_pool.probe_idle_connections().await;
                    }
                }
            }
        });
    }

    /// Stops the background heartbeat worker cleanly, ensuring no tasks are leaked.
    pub fn stop_heartbeat_worker(&self) {
        if let Some(tx) = self.heartbeat_shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    /// Helper to probe idle connections across all nodes during heartbeat ticks.
    async fn probe_idle_connections(&self) {
        let node_ids: Vec<u8> = {
            let idle_guard = self.idle_conns.lock().unwrap();
            idle_guard.keys().copied().collect()
        };

        for node_id in node_ids {
            let maybe_conn = {
                let mut idle_guard = self.idle_conns.lock().unwrap();
                idle_guard.get_mut(&node_id).and_then(|list| list.pop())
            };

            if let Some(mut conn) = maybe_conn {
                let res = self.send_heartbeat(&mut conn, 1000).await;
                if res.is_ok() {
                    self.checkin(conn);
                } else {
                    self.discard(conn);
                }
            }
        }
    }

    /// Returns the number of currently idle connections for a node.
    pub fn idle_count_for_node(&self, node_id: u8) -> usize {
        let idle_guard = self.idle_conns.lock().unwrap();
        idle_guard.get(&node_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns total active (idle + checked out) connections for a node.
    pub fn total_active_for_node(&self, node_id: u8) -> usize {
        let count_guard = self.active_counts.lock().unwrap();
        count_guard.get(&node_id).copied().unwrap_or(0)
    }

    /// Sends a keepalive heartbeat request over a pooled connection,
    /// verifying that the peer responds with a matching `request_id` and `HeartbeatResponse`.
    pub async fn send_heartbeat(
        &self,
        conn: &mut PooledConnection,
        timeout_ms: u64,
    ) -> Result<(), FrameError> {
        let req_id = rand::random::<u64>();
        let seq = conn.sequence_number;
        conn.sequence_number = conn.sequence_number.wrapping_add(1);

        let req_frame = PhantomMeshFrame::new(
            MessageType::HeartbeatRequest,
            0,
            req_id,
            seq,
            Vec::new(),
        )?;

        write_frame(&mut conn.stream, &req_frame).await?;

        let read_res = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            read_frame(&mut conn.stream),
        )
        .await;

        let resp_frame = match read_res {
            Ok(Ok(f)) => f,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(FrameError::Io(io::ErrorKind::TimedOut)),
        };

        if resp_frame.header.msg_type != MessageType::HeartbeatResponse {
            return Err(FrameError::UnknownMessageType(resp_frame.header.msg_type.to_u8()));
        }

        // Strict request correlation
        if resp_frame.header.request_id != req_id {
            return Err(FrameError::BadMagic([0xFF, 0xFF, 0xFF, 0xFF])); // Correlation failure
        }

        conn.last_used_at = Instant::now();
        Ok(())
    }
}

impl Drop for TlsConnectionPool {
    fn drop(&mut self) {
        self.stop_heartbeat_worker();
    }
}
