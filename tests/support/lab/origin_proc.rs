//! Real-process controllable HTTP origin server for Phantom Lab Tier-2 (M4-04 / WP-16).
//!
//! Provides a real HTTP origin on loopback with:
//! - Axum HTTP engine on an ephemeral port (`127.0.0.1:0`).
//! - Concurrency limiting via `tokio::sync::Semaphore` (queueing emerges naturally).
//! - Admin API for dynamic degradation: capacity changes, latency inflation, error rates, outages.
//! - Per-request start/finish logging and telemetry counters.
//! - Graceful shutdown.

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Semaphore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginConfig {
    pub capacity: usize,
    pub latency_ms: u64,
    pub error_rate: f64, // 0.0 to 1.0
    pub is_outage: bool,
    pub default_ttl_sec: u64,
}

impl Default for OriginConfig {
    fn default() -> Self {
        Self {
            capacity: 16,
            latency_ms: 10,
            error_rate: 0.0,
            is_outage: false,
            default_ttl_sec: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginMetricsSnapshot {
    pub total_received: u64,
    pub total_completed: u64,
    pub total_errors: u64,
    pub current_in_flight: usize,
    pub max_in_flight: usize,
    pub total_queue_wait_us: u64,
    pub total_service_time_us: u64,
}

struct OriginInner {
    config: tokio::sync::RwLock<OriginConfig>,
    semaphore: Arc<tokio::sync::RwLock<Arc<Semaphore>>>,
    // Metrics
    total_received: AtomicU64,
    total_completed: AtomicU64,
    total_errors: AtomicU64,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    total_queue_wait_us: AtomicU64,
    total_service_time_us: AtomicU64,
}

pub struct ControllableOrigin {
    addr: SocketAddr,
    inner: Arc<OriginInner>,
    shutdown_tx: broadcast::Sender<()>,
}

impl ControllableOrigin {
    pub async fn start(config: OriginConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let capacity = config.capacity.max(1);
        let sem = Arc::new(Semaphore::new(capacity));
        let inner = Arc::new(OriginInner {
            config: tokio::sync::RwLock::new(config),
            semaphore: Arc::new(tokio::sync::RwLock::new(sem)),
            total_received: AtomicU64::new(0),
            total_completed: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            total_queue_wait_us: AtomicU64::new(0),
            total_service_time_us: AtomicU64::new(0),
        });

        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let inner_state = inner.clone();
        let app = Router::new()
            .route("/admin/config", get(get_config).post(set_config))
            .route("/admin/metrics", get(get_metrics))
            .route("/admin/reset", post(reset_metrics))
            .route("/data/*path", get(handle_data))
            .with_state(inner_state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.recv().await;
                })
                .await
                .ok();
        });

        Ok(Self {
            addr,
            inner,
            shutdown_tx,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub async fn update_config(&self, new_config: OriginConfig) {
        let mut cfg = self.inner.config.write().await;
        if cfg.capacity != new_config.capacity {
            let mut sem_lock = self.inner.semaphore.write().await;
            *sem_lock = Arc::new(Semaphore::new(new_config.capacity.max(1)));
        }
        *cfg = new_config;
    }

    pub fn snapshot_metrics(&self) -> OriginMetricsSnapshot {
        OriginMetricsSnapshot {
            total_received: self.inner.total_received.load(Ordering::Relaxed),
            total_completed: self.inner.total_completed.load(Ordering::Relaxed),
            total_errors: self.inner.total_errors.load(Ordering::Relaxed),
            current_in_flight: self.inner.current_in_flight.load(Ordering::Relaxed),
            max_in_flight: self.inner.max_in_flight.load(Ordering::Relaxed),
            total_queue_wait_us: self.inner.total_queue_wait_us.load(Ordering::Relaxed),
            total_service_time_us: self.inner.total_service_time_us.load(Ordering::Relaxed),
        }
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for ControllableOrigin {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// Handler implementations

async fn get_config(State(inner): State<Arc<OriginInner>>) -> Json<OriginConfig> {
    let cfg = inner.config.read().await;
    Json(cfg.clone())
}

async fn set_config(
    State(inner): State<Arc<OriginInner>>,
    Json(new_config): Json<OriginConfig>,
) -> StatusCode {
    let mut cfg = inner.config.write().await;
    if cfg.capacity != new_config.capacity {
        let mut sem_lock = inner.semaphore.write().await;
        *sem_lock = Arc::new(Semaphore::new(new_config.capacity.max(1)));
    }
    *cfg = new_config;
    StatusCode::OK
}

async fn get_metrics(State(inner): State<Arc<OriginInner>>) -> Json<OriginMetricsSnapshot> {
    let snap = OriginMetricsSnapshot {
        total_received: inner.total_received.load(Ordering::Relaxed),
        total_completed: inner.total_completed.load(Ordering::Relaxed),
        total_errors: inner.total_errors.load(Ordering::Relaxed),
        current_in_flight: inner.current_in_flight.load(Ordering::Relaxed),
        max_in_flight: inner.max_in_flight.load(Ordering::Relaxed),
        total_queue_wait_us: inner.total_queue_wait_us.load(Ordering::Relaxed),
        total_service_time_us: inner.total_service_time_us.load(Ordering::Relaxed),
    };
    Json(snap)
}

async fn reset_metrics(State(inner): State<Arc<OriginInner>>) -> StatusCode {
    inner.total_received.store(0, Ordering::Relaxed);
    inner.total_completed.store(0, Ordering::Relaxed);
    inner.total_errors.store(0, Ordering::Relaxed);
    inner.max_in_flight.store(0, Ordering::Relaxed);
    inner.total_queue_wait_us.store(0, Ordering::Relaxed);
    inner.total_service_time_us.store(0, Ordering::Relaxed);
    StatusCode::OK
}

async fn handle_data(
    State(inner): State<Arc<OriginInner>>,
    Path(path): Path<String>,
) -> Response {
    inner.total_received.fetch_add(1, Ordering::Relaxed);

    let (is_outage, error_rate, latency_ms, ttl_sec) = {
        let cfg = inner.config.read().await;
        (cfg.is_outage, cfg.error_rate, cfg.latency_ms, cfg.default_ttl_sec)
    };

    if is_outage {
        inner.total_errors.fetch_add(1, Ordering::Relaxed);
        return (StatusCode::SERVICE_UNAVAILABLE, "Origin in Outage (O-OUT)").into_response();
    }

    if error_rate > 0.0 && rand::random::<f64>() < error_rate {
        inner.total_errors.fetch_add(1, Ordering::Relaxed);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Origin Brownout Error (O-BR)").into_response();
    }

    // Acquire semaphore permit (measures queue wait time!)
    let sem = {
        let sem_lock = inner.semaphore.read().await;
        sem_lock.clone()
    };

    let start_queue = Instant::now();
    let permit = match sem.acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            inner.total_errors.fetch_add(1, Ordering::Relaxed);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Semaphore closed").into_response();
        }
    };
    let queue_wait_us = start_queue.elapsed().as_micros() as u64;
    inner.total_queue_wait_us.fetch_add(queue_wait_us, Ordering::Relaxed);

    // Track active in-flight
    let active = inner.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
    let mut current_max = inner.max_in_flight.load(Ordering::Relaxed);
    while active > current_max {
        match inner.max_in_flight.compare_exchange_weak(
            current_max,
            active,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(actual) => current_max = actual,
        }
    }

    // Simulate server execution latency
    let start_service = Instant::now();
    if latency_ms > 0 {
        tokio::time::sleep(Duration::from_millis(latency_ms)).await;
    }
    let service_time_us = start_service.elapsed().as_micros() as u64;
    inner.total_service_time_us.fetch_add(service_time_us, Ordering::Relaxed);

    inner.current_in_flight.fetch_sub(1, Ordering::SeqCst);
    inner.total_completed.fetch_add(1, Ordering::Relaxed);
    drop(permit);

    // Build HTTP response
    let body = format!("DATA:{}", path);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&format!("public, max-age={}", ttl_sec)).unwrap(),
    );

    (StatusCode::OK, headers, body).into_response()
}
