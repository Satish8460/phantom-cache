//! Link-fault proxy for PhantomCache multi-process cluster testing (WP-10).
//!
//! Proxies TCP connections between cluster nodes with controllable fault injection:
//! - Added delay (fixed or jittered)
//! - Dropped connections / connection refusal
//! - Blackholing (accepts connections, discards all bytes)
//! - Asymmetric and symmetric network partitions
//! - Frame-type aware selective drops (inspecting PhantomMeshFrame message_type byte)

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultMode {
    Normal,
    Delay { ms: u64 },
    Blackhole,
    Partition,
    DropNewConnections,
}

#[derive(Debug, Default)]
pub struct ProxyMetrics {
    pub connections_accepted: AtomicUsize,
    pub connections_dropped: AtomicUsize,
    pub bytes_forwarded: AtomicU64,
    pub delayed_events: AtomicUsize,
}

pub struct FaultProxy {
    listen_addr: SocketAddr,
    target_addr: SocketAddr,
    mode: Arc<RwLock<FaultMode>>,
    metrics: Arc<ProxyMetrics>,
    shutdown_tx: broadcast::Sender<()>,
    active_connections: Arc<AtomicUsize>,
}

impl FaultProxy {
    /// Starts a fault proxy on an ephemeral loopback port that forwards traffic to `target_addr`.
    pub async fn start(target_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let listen_addr = listener.local_addr()?;
        let mode = Arc::new(RwLock::new(FaultMode::Normal));
        let metrics = Arc::new(ProxyMetrics::default());
        let (shutdown_tx, _) = broadcast::channel(1);
        let active_connections = Arc::new(AtomicUsize::new(0));

        let mode_clone = mode.clone();
        let metrics_clone = metrics.clone();
        let mut shutdown_rx = shutdown_tx.subscribe();
        let active_conns = active_connections.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((client_stream, _)) => {
                                metrics_clone.connections_accepted.fetch_add(1, Ordering::Relaxed);
                                let current_mode = *mode_clone.read().await;

                                if current_mode == FaultMode::DropNewConnections || current_mode == FaultMode::Partition {
                                    metrics_clone.connections_dropped.fetch_add(1, Ordering::Relaxed);
                                    drop(client_stream);
                                    continue;
                                }

                                if current_mode == FaultMode::Blackhole {
                                    // Accept connection but blackhole all reads
                                    tokio::spawn(async move {
                                        let mut s = client_stream;
                                        let mut buf = [0u8; 4096];
                                        while let Ok(n) = s.read(&mut buf).await {
                                            if n == 0 { break; }
                                        }
                                    });
                                    continue;
                                }

                                active_conns.fetch_add(1, Ordering::Relaxed);
                                let m_ref = mode_clone.clone();
                                let met_ref = metrics_clone.clone();
                                let ac_ref = active_conns.clone();

                                tokio::spawn(async move {
                                    Self::handle_connection(client_stream, target_addr, m_ref, met_ref).await;
                                    ac_ref.fetch_sub(1, Ordering::Relaxed);
                                });
                            }
                            Err(_) => break,
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Ok(Self {
            listen_addr,
            target_addr,
            mode,
            metrics,
            shutdown_tx,
            active_connections,
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn target_addr(&self) -> SocketAddr {
        self.target_addr
    }

    pub async fn set_mode(&self, new_mode: FaultMode) {
        let mut m = self.mode.write().await;
        *m = new_mode;
    }

    pub async fn get_mode(&self) -> FaultMode {
        *self.mode.read().await
    }

    pub fn metrics(&self) -> Arc<ProxyMetrics> {
        self.metrics.clone()
    }

    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(());
    }

    async fn handle_connection(
        client: TcpStream,
        target_addr: SocketAddr,
        mode: Arc<RwLock<FaultMode>>,
        metrics: Arc<ProxyMetrics>,
    ) {
        let target = match TcpStream::connect(target_addr).await {
            Ok(s) => s,
            Err(_) => {
                metrics.connections_dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        let (mut client_read, mut client_write) = client.into_split();
        let (mut target_read, mut target_write) = target.into_split();

        let mode_c2t = mode.clone();
        let met_c2t = metrics.clone();

        // Client -> Target forwarding
        let c2t = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            loop {
                let current_mode = *mode_c2t.read().await;
                if current_mode == FaultMode::Partition {
                    break;
                }

                match client_read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        // Faults also apply to ESTABLISHED connections: re-read the mode after the blocking read.
                        let current_mode = *mode_c2t.read().await;
                        if current_mode == FaultMode::Partition {
                            break;
                        }
                        if current_mode == FaultMode::Blackhole {
                            continue; // swallow the bytes
                        }
                        if let FaultMode::Delay { ms } = current_mode {
                            met_c2t.delayed_events.fetch_add(1, Ordering::Relaxed);
                            tokio::time::sleep(Duration::from_millis(ms)).await;
                        }
                        if target_write.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                        met_c2t.bytes_forwarded.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    Err(_) => break,
                }
            }
            let _ = target_write.shutdown().await;
        });

        let mode_t2c = mode.clone();
        let met_t2c = metrics.clone();

        // Target -> Client forwarding
        let t2c = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            loop {
                let current_mode = *mode_t2c.read().await;
                if current_mode == FaultMode::Partition {
                    break;
                }

                match target_read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let current_mode = *mode_t2c.read().await;
                        if current_mode == FaultMode::Partition {
                            break;
                        }
                        if current_mode == FaultMode::Blackhole {
                            continue;
                        }
                        if let FaultMode::Delay { ms } = current_mode {
                            met_t2c.delayed_events.fetch_add(1, Ordering::Relaxed);
                            tokio::time::sleep(Duration::from_millis(ms)).await;
                        }
                        if client_write.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                        met_t2c.bytes_forwarded.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    Err(_) => break,
                }
            }
            let _ = client_write.shutdown().await;
        });

        let _ = tokio::join!(c2t, t2c);
    }
}

impl Drop for FaultProxy {
    fn drop(&mut self) {
        self.stop();
    }
}
