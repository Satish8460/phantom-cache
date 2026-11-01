//! Lab Gateway and Policy Layer for Phantom Lab (M4-05 / WP-17).
//!
//! Hosts static baselines BL-0..BL-12b without mutating product request-path semantics.
//! Reuses PartitionedCache, RFC 9111 freshness engine, and origin client.
//! Exposes Axum HTTP front door on an ephemeral loopback port (`127.0.0.1:0`).

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::future::BoxFuture;
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache, StaleServingDecision,
};
use phantom_cache_demo::core::coalesce::{FinalizeOutcome, OriginError};
use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer;
use phantom_cache_demo::intelligence::audit::AuditStore;
use phantom_cache_demo::intelligence::engine::PieDecisionEngine;
use phantom_cache_demo::intelligence::executor::DefaultPolicyExecutor;
use phantom_cache_demo::intelligence::PolicyExecutor;
use reqwest::Client;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};

/// Baseline policy decision hook.
pub trait LabPolicy: Send + Sync {
    fn policy_id(&self) -> &'static str;

    /// Fetches upstream resource for cache miss.
    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>>;
}

/// Baseline 1: Standard Single-Flight Coalescing (BL-1).
pub struct SingleFlightPolicy;

impl LabPolicy for SingleFlightPolicy {
    fn policy_id(&self) -> &'static str {
        "BL-1"
    }

    fn fetch_miss<'a>(
        &'a self,
        key: &'a CacheKey,
        origin_url: &'a str,
        client: &'a Client,
    ) -> BoxFuture<'a, Result<CachedResource, OriginError>> {
        Box::pin(async move {
            let url = format!("{}/data{}", origin_url, key.path);
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| OriginError::NetworkError(e.to_string()))?;

            let status = resp.status().as_u16();
            if !resp.status().is_success() {
                return Err(OriginError::UpstreamStatus(status));
            }

            let body_bytes = resp
                .bytes()
                .await
                .map_err(|e| OriginError::NetworkError(e.to_string()))?;

            let now_ms = DiagnosticBuffer::now_ms();
            Ok(CachedResource {
                status,
                content_type: "text/plain".to_string(),
                body: Arc::new(body_bytes.to_vec()),
                etag: None,
                created_at_ms: now_ms,
                ttl_ms: 60_000,
            })
        })
    }
}

pub struct GatewayTelemetry {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub stale_served: AtomicU64,
    pub origin_fetches: AtomicU64,
    pub errors: AtomicU64,
}

struct GatewayInner {
    cache: Arc<PartitionedCache>,
    origin_url: String,
    origin_client: Client,
    policy: Arc<dyn LabPolicy>,
    telemetry: Arc<GatewayTelemetry>,
    executor: Option<Arc<DefaultPolicyExecutor>>,
    engine: Option<Arc<Mutex<PieDecisionEngine>>>,
    audit: Option<Arc<Mutex<AuditStore>>>,
}

pub struct LabGateway {
    addr: SocketAddr,
    inner: Arc<GatewayInner>,
    shutdown_tx: broadcast::Sender<()>,
}

impl LabGateway {
    pub async fn start(
        cache: Arc<PartitionedCache>,
        origin_url: String,
        policy: Arc<dyn LabPolicy>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let telemetry = Arc::new(GatewayTelemetry {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            stale_served: AtomicU64::new(0),
            origin_fetches: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        });

        let inner = Arc::new(GatewayInner {
            cache,
            origin_url,
            origin_client: Client::builder()
                .pool_max_idle_per_host(128)
                .timeout(Duration::from_secs(5))
                .build()?,
            policy,
            telemetry,
            executor: None,
            engine: None,
            audit: None,
        });

        let app = Router::new()
            .route("/*path", get(handle_get).delete(handle_delete))
            .with_state(inner.clone());

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

    pub async fn start_with_intelligence(
        cache: Arc<PartitionedCache>,
        origin_url: String,
        policy: Arc<dyn LabPolicy>,
        executor: Arc<DefaultPolicyExecutor>,
        engine: Arc<Mutex<PieDecisionEngine>>,
        audit: Arc<Mutex<AuditStore>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let telemetry = Arc::new(GatewayTelemetry {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            stale_served: AtomicU64::new(0),
            origin_fetches: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        });

        let inner = Arc::new(GatewayInner {
            cache,
            origin_url,
            origin_client: Client::builder()
                .pool_max_idle_per_host(128)
                .timeout(Duration::from_secs(5))
                .build()?,
            policy,
            telemetry,
            executor: Some(executor),
            engine: Some(engine),
            audit: Some(audit),
        });

        let app = Router::new()
            .route("/*path", get(handle_get).delete(handle_delete))
            .with_state(inner.clone());

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

    pub fn telemetry(&self) -> &GatewayTelemetry {
        &self.inner.telemetry
    }

    pub fn executor(&self) -> Option<&Arc<DefaultPolicyExecutor>> {
        self.inner.executor.as_ref()
    }

    pub fn engine(&self) -> Option<&Arc<Mutex<PieDecisionEngine>>> {
        self.inner.engine.as_ref()
    }

    pub fn audit(&self) -> Option<&Arc<Mutex<AuditStore>>> {
        self.inner.audit.as_ref()
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for LabGateway {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// Request handlers

async fn handle_get(
    State(inner): State<Arc<GatewayInner>>,
    Path(path): Path<String>,
) -> Response {
    let norm_path = if path.starts_with('/') {
        path
    } else {
        format!("/{}", path)
    };

    let cache_key = CacheKey::new(None, &norm_path, None);
    let key_hash = {
        let mut hasher = DefaultHasher::new();
        cache_key.hash(&mut hasher);
        hasher.finish()
    };
    let now_ms = DiagnosticBuffer::now_ms();

    // 1. Fresh cache lookup / single-flight atomic registration
    let outcome: Result<Arc<CachedResource>, OriginError> = match inner.cache.begin_fetch_atomic(&cache_key, now_ms) {
        FetchRegistrationResult::Hit(resource) => {
            inner.telemetry.hits.fetch_add(1, Ordering::Relaxed);
            if let (Some(ref engine), Some(ref exec)) = (&inner.engine, &inner.executor) {
                let arm = DefaultPolicyExecutor::route_bucket(key_hash);
                let lat_us = (DiagnosticBuffer::now_ms().saturating_sub(now_ms) * 1000).max(100);
                engine.lock().unwrap().record_arm_traffic(arm, true, lat_us, false, false);
                exec.heartbeat();
            }
            let mut headers = HeaderMap::new();
            headers.insert(HeaderName::from_static("x-cache"), HeaderValue::from_static("HIT"));
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
            return (StatusCode::OK, headers, (*resource.body).clone()).into_response();
        }
        FetchRegistrationResult::Wait(rx) => {
            inner.telemetry.hits.fetch_add(1, Ordering::Relaxed); // coalesced waiter
            match rx.await {
                Ok(Ok(resource)) => {
                    if let (Some(ref engine), Some(ref exec)) = (&inner.engine, &inner.executor) {
                        let arm = DefaultPolicyExecutor::route_bucket(key_hash);
                        let lat_us = (DiagnosticBuffer::now_ms().saturating_sub(now_ms) * 1000).max(100);
                        engine.lock().unwrap().record_arm_traffic(arm, true, lat_us, false, false);
                        exec.heartbeat();
                    }
                    let mut headers = HeaderMap::new();
                    headers.insert(HeaderName::from_static("x-cache"), HeaderValue::from_static("HIT"));
                    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                    return (StatusCode::OK, headers, (*resource.body).clone()).into_response();
                }
                Ok(Err(err)) => Err((*err).clone()),
                Err(_) => Err(OriginError::NetworkError("Leader aborted fetch".to_string())),
            }
        }
        FetchRegistrationResult::Initiate(reg) => {
            inner.telemetry.misses.fetch_add(1, Ordering::Relaxed);
            inner.telemetry.origin_fetches.fetch_add(1, Ordering::Relaxed);

            let (my_tx, my_rx) = oneshot::channel();
            {
                let mut guard = reg.shared_op.waiters.lock().unwrap();
                guard.push(my_tx);
            }

            let guard = reg.into_guard();
            let policy = inner.policy.clone();
            let origin_url = inner.origin_url.clone();
            let client = inner.origin_client.clone();
            let key_clone = cache_key.clone();

            tokio::spawn(async move {
                let fetch_res = policy.fetch_miss(&key_clone, &origin_url, &client).await;
                match fetch_res {
                    Ok(res) => {
                        let arc_res = Arc::new(res);
                        if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(arc_res)) {
                            batch.deliver();
                        }
                    }
                    Err(err) => {
                        let arc_err = Arc::new(err);
                        if let Some(mut batch) = guard.complete(FinalizeOutcome::Error(arc_err)) {
                            batch.deliver();
                        }
                    }
                }
            });

            match my_rx.await {
                Ok(Ok(resource)) => {
                    if let (Some(ref engine), Some(ref exec)) = (&inner.engine, &inner.executor) {
                        let arm = DefaultPolicyExecutor::route_bucket(key_hash);
                        let lat_us = (DiagnosticBuffer::now_ms().saturating_sub(now_ms) * 1000).max(100);
                        engine.lock().unwrap().record_arm_traffic(arm, false, lat_us, false, false);
                        exec.heartbeat();
                    }
                    let mut headers = HeaderMap::new();
                    headers.insert(HeaderName::from_static("x-cache"), HeaderValue::from_static("MISS"));
                    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                    return (StatusCode::OK, headers, (*resource.body).clone()).into_response();
                }
                Ok(Err(err)) => Err((*err).clone()),
                Err(_) => Err(OriginError::NetworkError("Leader aborted fetch".to_string())),
            }
        }
        FetchRegistrationResult::Rejected(err) => Err(err),
    };

    // Stale fallback check (RFC 9111) if miss failed with origin error
    let hard_cap_ms = inner.cache.get_max_stale_ms();
    let stale_decision = inner.cache.check_stale_serving(&cache_key, now_ms, hard_cap_ms, true);
    if let StaleServingDecision::Eligible(stale_res, _) = stale_decision {
        let age_ms = now_ms.saturating_sub(stale_res.created_at_ms);
        inner.cache.record_stale_served(age_ms, stale_res.ttl_ms, hard_cap_ms);
        inner.telemetry.stale_served.fetch_add(1, Ordering::Relaxed);
        if let (Some(ref engine), Some(ref exec)) = (&inner.engine, &inner.executor) {
            let arm = DefaultPolicyExecutor::route_bucket(key_hash);
            let lat_us = (DiagnosticBuffer::now_ms().saturating_sub(now_ms) * 1000).max(100);
            engine.lock().unwrap().record_arm_traffic(arm, true, lat_us, false, false);
            exec.heartbeat();
        }

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("x-cache"), HeaderValue::from_static("STALE"));
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        return (StatusCode::OK, headers, (*stale_res.body).clone()).into_response();
    }

    match outcome {
        Ok(_) => unreachable!(),
        Err(err) => {
            inner.telemetry.errors.fetch_add(1, Ordering::Relaxed);
            if let (Some(ref engine), Some(ref exec)) = (&inner.engine, &inner.executor) {
                let arm = DefaultPolicyExecutor::route_bucket(key_hash);
                let lat_us = (DiagnosticBuffer::now_ms().saturating_sub(now_ms) * 1000).max(100);
                engine.lock().unwrap().record_arm_traffic(arm, false, lat_us, true, false);
                exec.heartbeat();
            }
            match err {
                OriginError::UpstreamStatus(status) => {
                    let sc = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
                    (sc, "Origin Fetch Error").into_response()
                }
                OriginError::NetworkError(_) => (StatusCode::BAD_GATEWAY, "Origin Network Error").into_response(),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Gateway Error").into_response(),
            }
        }
    }
}

async fn handle_delete(
    State(inner): State<Arc<GatewayInner>>,
    Path(path): Path<String>,
) -> StatusCode {
    let norm_path = if path.starts_with('/') {
        path
    } else {
        format!("/{}", path)
    };

    let cache_key = CacheKey::new(None, &norm_path, None);
    // Linearizable cache deletion
    inner.cache.delete(&cache_key);
    StatusCode::NO_CONTENT
}
