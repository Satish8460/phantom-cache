use crate::control::pie::PieController;
use crate::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
use crate::core::coalesce::{FinalizeOutcome, OriginError};
use crate::dist::node::{NodeState, VirtualNode};
use crate::intel::diagnostics::{CacheCounters, DiagnosticBuffer, DiagnosticEvent, DiagnosticLevel};
use crate::intel::telemetry::TelemetryCollector;
use crate::intelligence::PolicyExecutor;
use axum::{
    extract::{Path, RawQuery, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// G2: the Lab HTTP API (routes exist only when a `LabService` is installed; see docs/user/lab_api.md).
#[path = "lab_api.rs"]
pub mod lab_api;

static LAB_SERVICE: std::sync::OnceLock<Arc<lab_api::LabService>> = std::sync::OnceLock::new();

/// Installs the process-wide Lab service used by `start_server_with_cors` (called by `main` only when
/// `PHANTOM_LAB_ENABLED=true`). Tests build routers explicitly with `create_app_with_lab`.
pub fn install_lab_service(svc: Arc<lab_api::LabService>) {
    let _ = LAB_SERVICE.set(svc);
}

#[derive(Clone, Serialize)]
pub struct NodeSnapshot {
    pub node_id: u8,
    pub state: String,
    pub cpu_utilization: f32,
    pub memory_used_bytes: u64,
}

#[derive(Clone, Serialize)]
pub struct SimulationSnapshot {
    pub version: String,
    pub timestamp_ms: u64,
    pub throughput_req_sec: f64,
    pub p50_latency_ms: u32,
    pub p95_latency_ms: u32,
    pub p99_latency_ms: u32,
    pub db_load_ratio: f32,
    pub avoided_compute_dollars: f64,
    pub nodes: Vec<NodeSnapshot>,
}

pub struct ApiState {
    pub snapshot: Mutex<SimulationSnapshot>,
    pub nodes: Vec<Arc<VirtualNode>>,
    pub workload_type: Mutex<String>,
    pub cache: Option<Arc<PartitionedCache>>,
    pub origin_url: Option<String>,
    pub telemetry: Option<Arc<TelemetryCollector>>,
    pub pie: Option<Arc<PieController>>,
    pub security: Option<Arc<crate::security::SecurityConfig>>,
    pub upstream_policy: Option<Arc<crate::security::upstream::UpstreamPolicy>>,
    pub router: Option<Arc<crate::dist::router::RendezvousRouter>>,
}

/// Where the leader response of a cache miss came from (internal; see `mark_source`).
/// A peer-supplied remaining lifetime is clamped to this (PC-AUTO-051, D-036).
const PEER_MAX_LIFETIME_MS: u64 = 86_400_000;
/// A peer-supplied age or remaining lifetime above this is ignored (the entry is not stored).
const PEER_ABSURD_MS: u64 = 365 * 86_400_000;
const SOURCE_ORIGIN: u8 = 0;
const SOURCE_PEER: u8 = 1;

/// Per-process (one gateway = one node) counters of how each cache GET response was produced, the
/// counterpart of the additive `X-Cache-Source` response header (D-016). Exactly one of
/// `local`, `peer`, `origin`, `coalesced`, `stale` is incremented per response that carries the
/// header; error responses carry none. `peer_read_*` describe peer reads that did not produce a
/// response from the peer (the request then went to the origin and counts as `origin`).
/// Process-global because the gateway state struct is constructed field by field in many tests.
pub struct OutcomeCounters {
    pub local: std::sync::atomic::AtomicU64,
    pub peer: std::sync::atomic::AtomicU64,
    pub origin: std::sync::atomic::AtomicU64,
    pub coalesced: std::sync::atomic::AtomicU64,
    pub stale: std::sync::atomic::AtomicU64,
    pub peer_read_miss: std::sync::atomic::AtomicU64,
    pub peer_read_failed: std::sync::atomic::AtomicU64,
    pub peer_read_refused: std::sync::atomic::AtomicU64,
    pub peer_read_rejected: std::sync::atomic::AtomicU64,
}

static OUTCOMES: OutcomeCounters = OutcomeCounters {
    local: std::sync::atomic::AtomicU64::new(0),
    peer: std::sync::atomic::AtomicU64::new(0),
    origin: std::sync::atomic::AtomicU64::new(0),
    coalesced: std::sync::atomic::AtomicU64::new(0),
    stale: std::sync::atomic::AtomicU64::new(0),
    peer_read_miss: std::sync::atomic::AtomicU64::new(0),
    peer_read_failed: std::sync::atomic::AtomicU64::new(0),
    peer_read_refused: std::sync::atomic::AtomicU64::new(0),
    peer_read_rejected: std::sync::atomic::AtomicU64::new(0),
};

pub fn outcome_counters() -> &'static OutcomeCounters {
    &OUTCOMES
}

impl OutcomeCounters {
    pub fn to_json(&self) -> serde_json::Value {
        let c = |a: &std::sync::atomic::AtomicU64| json!({"src": "measured", "v": a.load(Ordering::Relaxed), "unit": "count"});
        json!({
            "responses": {
                "local": c(&self.local),
                "peer": c(&self.peer),
                "origin": c(&self.origin),
                "coalesced": c(&self.coalesced),
                "stale": c(&self.stale)
            },
            "peer_reads_without_hit": {
                "miss": c(&self.peer_read_miss),
                "failed": c(&self.peer_read_failed),
                "refused_format": c(&self.peer_read_refused),
                "rejected_lifetime": c(&self.peer_read_rejected)
            }
        })
    }
}

/// Sets the additive `X-Cache-Source` header (`local | peer | origin | coalesced | stale`) and counts
/// the outcome. `X-Cache-Status` is untouched.
fn mark_source(resp: &mut Response, source: &'static str) {
    let counter = match source {
        "local" => &OUTCOMES.local,
        "peer" => &OUTCOMES.peer,
        "coalesced" => &OUTCOMES.coalesced,
        "stale" => &OUTCOMES.stale,
        _ => &OUTCOMES.origin,
    };
    counter.fetch_add(1, Ordering::Relaxed);
    resp.headers_mut().insert("X-Cache-Source", HeaderValue::from_static(source));
}

static GLOBAL_INTEL_EXECUTOR: std::sync::OnceLock<Arc<crate::intelligence::executor::DefaultPolicyExecutor>> = std::sync::OnceLock::new();
static GLOBAL_INTEL_AUDIT: std::sync::OnceLock<Arc<Mutex<crate::intelligence::audit::AuditStore>>> = std::sync::OnceLock::new();

impl ApiState {
    pub fn set_intelligence(
        executor: Arc<crate::intelligence::executor::DefaultPolicyExecutor>,
        audit: Arc<Mutex<crate::intelligence::audit::AuditStore>>,
    ) {
        let _ = GLOBAL_INTEL_EXECUTOR.set(executor);
        let _ = GLOBAL_INTEL_AUDIT.set(audit);
    }

    pub fn intel_executor(&self) -> Option<Arc<crate::intelligence::executor::DefaultPolicyExecutor>> {
        GLOBAL_INTEL_EXECUTOR.get().cloned()
    }

    pub fn intel_audit(&self) -> Option<Arc<Mutex<crate::intelligence::audit::AuditStore>>> {
        GLOBAL_INTEL_AUDIT.get().cloned()
    }

    pub fn with_router(mut self, router: Arc<crate::dist::router::RendezvousRouter>) -> Self {
        self.router = Some(router);
        self
    }

    pub fn get_upstream_policy(&self) -> Option<Arc<crate::security::upstream::UpstreamPolicy>> {
        self.upstream_policy
            .clone()
            .or_else(|| self.security.as_ref().and_then(|s| s.upstream.clone()))
    }

    pub fn origin_metrics(&self) -> Option<&Arc<crate::intel::origin_metrics::OriginMetrics>> {
        self.telemetry.as_ref().map(|t| &t.origin_metrics)
    }
}

/// SEC-M7-03: browser cross-origin policy. CORS is a browser-origin control only; it is NOT
/// authentication or authorization (curl and other non-browser clients ignore it).
///
/// The default (and `same_origin()`) allows NO cross-origin access: no `Access-Control-Allow-Origin`
/// header is ever emitted. Origins are exact `scheme://host[:port]` values validated by
/// `crate::config::parse_cors_origins` (no wildcard). Credentials (cookies) are never allowed;
/// browser clients authenticate with an `Authorization: Bearer` header.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorsConfig {
    allowed_origins: Vec<String>,
}

impl CorsConfig {
    /// No cross-origin access (default).
    pub fn same_origin() -> Self {
        Self::default()
    }

    /// Build from raw origin strings; every entry is validated and normalized.
    pub fn with_origins<I, S>(origins: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let joined = origins.into_iter().map(|s| s.as_ref().to_string()).collect::<Vec<_>>().join(",");
        if joined.is_empty() {
            return Ok(Self::default());
        }
        Ok(Self { allowed_origins: crate::config::parse_cors_origins(&joined)? })
    }

    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }

    fn layer(&self) -> CorsLayer {
        use axum::http::{header, Method};
        let origins: Vec<HeaderValue> = self
            .allowed_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                HeaderName::from_static("x-tenant-id"),
                HeaderName::from_static("x-cache-bypass"),
                // Lab API SSE resume via fetch streaming
                HeaderName::from_static("last-event-id"),
            ])
            .expose_headers([
                HeaderName::from_static("x-cache-status"),
                HeaderName::from_static("x-cache-source"),
                header::ETAG,
                header::LAST_MODIFIED,
                header::AGE,
                header::RETRY_AFTER,
                HeaderName::from_static("x-cache-error"),
                HeaderName::from_static("x-cache-warning"),
                header::WWW_AUTHENTICATE,
                HeaderName::from_static("x-auth-error"),
                header::CONTENT_TYPE,
            ])
            .max_age(Duration::from_secs(600))
    }
}

/// Router with the default CORS policy (same-origin only).
pub fn create_app(shared_state: Arc<ApiState>) -> Router {
    create_app_with_cors(shared_state, &CorsConfig::same_origin())
}

pub fn create_app_with_cors(shared_state: Arc<ApiState>, cors_cfg: &CorsConfig) -> Router {
    create_app_with_lab(shared_state, cors_cfg, None)
}

/// Same as `create_app_with_cors`, plus the Lab API under `/api/v1/lab` when `lab` is `Some`.
/// With `None` (the default everywhere) no lab route exists and every `/api/v1/lab/*` path is a 404.
pub fn create_app_with_lab(
    shared_state: Arc<ApiState>,
    cors_cfg: &CorsConfig,
    lab: Option<Arc<lab_api::LabService>>,
) -> Router {
    let cors = cors_cfg.layer();
    let lab_routes = lab.map(|l| lab_api::router(shared_state.clone(), l, cors_cfg.allowed_origins().to_vec()));
    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/api/events", get(sse_handler))
        .route("/api/control", post(control_handler))
        .route(
            "/api/v1/cache/*key",
            get(get_cache_handler).delete(delete_cache_handler),
        )
        .route(
            "/api/v1/cache",
            get(get_cache_handler_root).delete(delete_cache_handler_root),
        )
        .route("/api/v1/metrics", get(metrics_handler))
        .route("/api/v1/diagnostics/summary", get(diagnostics_summary_handler))
        .route("/api/v1/diagnostics/events", get(diagnostics_events_handler))
        .route("/api/v1/intelligence/status", get(intelligence_status_handler))
        .route("/api/v1/intelligence/audit", get(intelligence_audit_handler))
        // PIE v2 (shadow mode), contract docs/lanes/CONTRACT-pie-api-v1.md
        .route("/api/v1/intelligence/registry", get(pie_registry_handler))
        .route("/api/v1/intelligence/signals", get(pie_signals_handler))
        .route("/api/v1/intelligence/decisions", get(pie_decisions_handler))
        .route("/api/v1/intelligence/decisions/:id", get(pie_decision_handler))
        .route("/api/v1/intelligence/policy/:id", get(pie_policy_handler))
        .with_state(shared_state)
        .merge(crate::health::router(crate::health::global()));
    if let Some(r) = lab_routes {
        app = app.merge(r);
    }
    app.layer(axum::extract::DefaultBodyLimit::max(512 * 1024)).layer(cors)
}

pub async fn start_server(shared_state: Arc<ApiState>) {
    start_server_with_cors(shared_state, CorsConfig::same_origin()).await
}

/// Same as `start_server` with an explicit CORS policy (wired from `PHANTOM_CORS_ORIGINS` by `main`).
pub async fn start_server_with_cors(shared_state: Arc<ApiState>, cors_cfg: CorsConfig) {
    let sim_enabled = std::env::var("PHANTOM_SIMULATION")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);
    if sim_enabled {
        spawn_simulation_loop(shared_state.clone());
    }
    spawn_metrics_sampler(shared_state.clone());
    // D-2: keep the sender alive for the whole server lifetime (it was dropped immediately, which
    // made the sweeper's shutdown receiver permanently ready). Signalled when the server returns.
    let (sweeper_shutdown_tx, sweeper_handle) = if let Some(ref cache) = shared_state.cache {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let h = PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
        (Some(tx), Some(h))
    } else {
        (None, None)
    };

    let lab = LAB_SERVICE.get().cloned();
    let app = create_app_with_lab(shared_state.clone(), &cors_cfg, lab.clone());

    let bind_addr_str = std::env::var("PHANTOM_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8085".to_string());
    let addr: SocketAddr = bind_addr_str
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 8085)));

    let tls_cert = std::env::var("PHANTOM_TLS_CERT").ok();
    let tls_key = std::env::var("PHANTOM_TLS_KEY").ok();
    let require_auth = std::env::var("PHANTOM_REQUIRE_AUTH")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);
    let allow_insecure = std::env::var("PHANTOM_ALLOW_INSECURE_HTTP")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);

    crate::security::validate_startup_security(
        &addr,
        shared_state.security.is_some(),
        tls_cert.is_some() && tls_key.is_some(),
        require_auth,
        allow_insecure,
    )
    .expect("Security Fail-Closed Startup Validation Failed");

    // G2: the Lab API may run anonymously only on a loopback bind (fail closed, like the checks above).
    let secure = shared_state.security.as_ref().map(|s| s.require_auth_for_control).unwrap_or(false);
    if let Err(m) = lab_api::check_lab_bind(secure, &addr, lab.is_some()) {
        panic!("{}", m);
    }
    if let Some(l) = lab.as_ref() {
        l.set_bind_addr(addr);
        l.set_tls(tls_cert.is_some() && tls_key.is_some());
    }

    if let Some(upstream_policy) = shared_state.get_upstream_policy() {
        crate::security::validate_upstream_startup_security(
            &addr,
            upstream_policy.allow_insecure_upstream,
            !upstream_policy.allowed_origins.is_empty(),
        )
        .expect("Upstream Security Fail-Closed Startup Validation Failed");
    }

    if let (Some(cert_path), Some(key_path)) = (tls_cert, tls_key) {
        println!("[Phantom API] Starting Axum server with TLS 1.3 on {}", addr);
        let tls_cfg = crate::security::TlsServerConfig::from_pem_file(&cert_path, &key_path)
            .await
            .expect("Failed to load TLS certificate or private key");
        axum_server::bind_rustls(addr, tls_cfg.rustls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
        if let Some(tx) = sweeper_shutdown_tx {
            let _ = tx.send(true);
        }
        drop(sweeper_handle);
    } else {
        if !addr.ip().is_loopback() && allow_insecure {
            eprintln!("================================================================================");
            eprintln!("[SECURITY WARNING] PHANTOM_ALLOW_INSECURE_HTTP=true IS ACTIVE ON {}", addr);
            eprintln!("[SECURITY WARNING] INGRESS TRAFFIC IS UNENCRYPTED. STRICTLY FOR DEMO LAB USE!");
            eprintln!("[SECURITY WARNING] MANDATORY JWT AUTHENTICATION REMAINS FULLY ENFORCED.");
            eprintln!("================================================================================");
        }
        println!("[Phantom API] Starting Axum server on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
        if let Some(tx) = sweeper_shutdown_tx {
            let _ = tx.send(true);
        }
        drop(sweeper_handle);
    }
}


/// True only when PHANTOM_SIMULATION is enabled. Single source for "mode" decisions.
fn simulation_enabled() -> bool {
    std::env::var("PHANTOM_SIMULATION")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

async fn health_check() -> &'static str {
    "OK"
}

/// SEC-M7-02: when security is enabled, intelligence endpoints require the Observer role, with
/// the same policy as metrics/diagnostics (401 missing/invalid, 403 insufficient role).
#[allow(clippy::result_large_err)]
fn require_observer_if_secure(state: &ApiState, headers: &HeaderMap) -> Result<(), Response> {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            authenticate_request(headers, sec, crate::security::Role::Observer)?;
        }
    }
    Ok(())
}

/// Audit page size bounds (SEC-M7-02).
pub const AUDIT_DEFAULT_LIMIT: usize = 50;
pub const AUDIT_MAX_LIMIT: usize = 100;

fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({"status": "error", "message": msg}))).into_response()
}

/// Parse `limit` / `offset` from a raw query string. Strict: unknown keys are ignored, duplicates
/// use the first occurrence, non-decimal or out-of-range values are an error (-> 400).
fn parse_audit_page(raw: Option<&str>) -> Result<(usize, usize), &'static str> {
    let mut limit: Option<&str> = None;
    let mut offset: Option<&str> = None;
    for pair in raw.unwrap_or("").split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "limit" if limit.is_none() => limit = Some(v),
                "offset" if offset.is_none() => offset = Some(v),
                _ => {}
            }
        } else if pair == "limit" || pair == "offset" {
            return Err("limit/offset need a value");
        }
    }
    let digits = |s: &str| !s.is_empty() && s.len() <= 18 && s.bytes().all(|b| b.is_ascii_digit());
    let limit = match limit {
        None => AUDIT_DEFAULT_LIMIT,
        Some(v) if digits(v) => v.parse::<usize>().map_err(|_| "limit must be an integer")?,
        Some(_) => return Err("limit must be a non-negative decimal integer"),
    };
    if limit == 0 || limit > AUDIT_MAX_LIMIT {
        return Err("limit must be between 1 and 100");
    }
    let offset = match offset {
        None => 0,
        Some(v) if digits(v) => v.parse::<usize>().map_err(|_| "offset must be an integer")?,
        Some(_) => return Err("offset must be a non-negative decimal integer"),
    };
    Ok((limit, offset))
}

/// Exposes (Observer role when security is enabled): mode name, active candidate id, revert
/// count, and a computed `truthful` flag. No secrets, key material, request keys or tenant data.
async fn intelligence_status_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    let (mode, active_cand, revert_count) = if let Some(ref exec) = state.intel_executor() {
        (
            format!("{:?}", exec.current_mode()),
            exec.get_active_candidate().map(|c| c.candidate_id),
            exec.get_revert_count(),
        )
    } else {
        ("Off".to_string(), None, 0)
    };
    // When PIE v2 is installed its mode is the process intelligence mode (Off, Observe or Advisory only).
    let (mode, v2) = match pie_v2() {
        Some(p) => (format!("{:?}", p.mode()), crate::intelligence_v2::api::status_json(p)),
        None => (mode, crate::intelligence_v2::api::not_installed_json()),
    };
    Json(json!({
        "status": "ok",
        "v2": v2,
        "intelligence_mode": mode,
        "active_candidate": active_cand,
        "revert_count": revert_count,
        "truthful": !simulation_enabled(),
        "truthful_basis": "computed: true only when PHANTOM_SIMULATION is not enabled. It says the status fields come from the executor, not that any decision is correct."
    })).into_response()
}

// ---------------------------------------------------------------------------------------------
// PIE v2 (shadow-mode Phantom Intelligence), library `crate::intelligence_v2`. Only Off, Observe and Advisory exist.
// The gateway installs at most one instance per process; the request path only calls `try_record` (non-blocking).
// ---------------------------------------------------------------------------------------------
static PIE_V2: std::sync::OnceLock<Arc<crate::intelligence_v2::Pie>> = std::sync::OnceLock::new();
static PIE_V2_SINK: std::sync::OnceLock<crate::intelligence_v2::signals::EventSink> = std::sync::OnceLock::new();
static PROCESS_START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Installs the PIE v2 instance (once per process) and its request-path sink.
pub fn install_pie_v2(p: Arc<crate::intelligence_v2::Pie>) {
    PROCESS_START.get_or_init(std::time::Instant::now);
    let _ = PIE_V2_SINK.set(p.sink());
    let _ = PIE_V2.set(p);
}

pub fn pie_v2() -> Option<&'static Arc<crate::intelligence_v2::Pie>> {
    PIE_V2.get()
}

/// Monotonic process clock in ms (the PIE takes its time from the caller; never the wall clock).
pub fn process_clock_ms() -> u64 {
    PROCESS_START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

/// One non-blocking event on the request path: a relaxed atomic load when PIE is off or not installed, a try-send
/// otherwise (a full queue drops the event and counts it). It never waits and never changes a response.
#[inline]
fn pie_record(key_hash: u64, latency_us: u64, outcome: crate::intelligence_v2::signals::Outcome, origin: crate::intelligence_v2::signals::OriginResult) {
    if let Some(sink) = PIE_V2_SINK.get() {
        let _ = sink.try_record(crate::intelligence_v2::signals::Event {
            key_hash,
            latency_us: latency_us.min(u32::MAX as u64) as u32,
            origin_latency_us: 0,
            // 0 = unknown: the size-aware features (F27 to F29) report `unavailable` instead of guessing.
            size_bytes: 0,
            outcome,
            origin,
        });
    }
}

fn pie_outcome_of(resp: &Response) -> Option<crate::intelligence_v2::signals::Outcome> {
    use crate::intelligence_v2::signals::Outcome;
    match resp.headers().get("x-cache-source").and_then(|v| v.to_str().ok()) {
        Some("local") => Some(Outcome::Hit),
        Some("origin") => Some(Outcome::Miss),
        Some("coalesced") => Some(Outcome::Coalesced),
        Some("stale") => Some(Outcome::Stale),
        Some("peer") => Some(Outcome::Peer),
        _ => None,
    }
}

async fn pie_registry_handler(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    match pie_v2() {
        Some(p) => Json(crate::intelligence_v2::api::registry_json(p)).into_response(),
        None => Json(crate::intelligence_v2::api::not_installed_json()).into_response(),
    }
}

async fn pie_signals_handler(State(state): State<Arc<ApiState>>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    match pie_v2() {
        Some(p) => Json(crate::intelligence_v2::api::signals_json(p)).into_response(),
        None => Json(crate::intelligence_v2::api::not_installed_json()).into_response(),
    }
}

async fn pie_decisions_handler(State(state): State<Arc<ApiState>>, raw_query: RawQuery, headers: HeaderMap) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    let p = match pie_v2() {
        Some(p) => p,
        None => return Json(crate::intelligence_v2::api::not_installed_json()).into_response(),
    };
    match crate::intelligence_v2::api::parse_decisions_query(raw_query.0.as_deref()) {
        Ok((limit, offset, pid)) => Json(crate::intelligence_v2::api::decisions_json(p, limit, offset, pid.as_deref())).into_response(),
        Err(detail) => (StatusCode::BAD_REQUEST, Json(crate::intelligence_v2::api::bad_request_json(&detail))).into_response(),
    }
}

async fn pie_decision_handler(State(state): State<Arc<ApiState>>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    let p = match pie_v2() {
        Some(p) => p,
        None => return Json(crate::intelligence_v2::api::not_installed_json()).into_response(),
    };
    let parsed: Option<u64> = id.parse().ok();
    match parsed.and_then(|n| crate::intelligence_v2::api::decision_json(p, n)) {
        Some(v) => Json(v).into_response(),
        None => (StatusCode::NOT_FOUND, Json(crate::intelligence_v2::api::unknown_decision_json())).into_response(),
    }
}

async fn pie_policy_handler(State(state): State<Arc<ApiState>>, Path(id): Path<String>, headers: HeaderMap) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    let p = match pie_v2() {
        Some(p) => p,
        None => return Json(crate::intelligence_v2::api::not_installed_json()).into_response(),
    };
    match crate::intelligence_v2::api::policy_json(p, &id) {
        Some(v) => Json(v).into_response(),
        None => (StatusCode::NOT_FOUND, Json(crate::intelligence_v2::api::unknown_policy_json(&id))).into_response(),
    }
}

/// Paginated audit trail. Ordering: retained records oldest-first (ascending `decision_id`, the
/// store's insertion order); `offset` counts from the oldest retained record. Cost is bounded by
/// the retained record count (store capacity), never by the lifetime counter `total_records`.
async fn intelligence_audit_handler(
    State(state): State<Arc<ApiState>>,
    raw_query: RawQuery,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_observer_if_secure(&state, &headers) {
        return resp;
    }
    let (limit, offset) = match parse_audit_page(raw_query.0.as_deref()) {
        Ok(v) => v,
        Err(msg) => return bad_request(msg),
    };
    if let Some(ref audit) = state.intel_audit() {
        let store = audit.lock().unwrap();
        let retained = store.len();
        let records: Vec<_> = store
            .all_decisions()
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        Json(json!({
            "status": "ok",
            "total_records": store.total_recorded(),
            "retained_records": retained,
            "limit": limit,
            "offset": offset,
            "returned": records.len(),
            "ordering": "oldest_first",
            "records": records
        })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "disabled",
                "message": "Phantom Intelligence is not enabled"
            })),
        ).into_response()
    }
}

pub fn spawn_metrics_sampler(state: Arc<ApiState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if let (Some(ref cache), Some(ref tel)) = (&state.cache, &state.telemetry) {
                let requests = cache.counters.requests_total.load(Ordering::Relaxed);
                let hits = cache.counters.cache_hits.load(Ordering::Relaxed);
                let misses = cache.counters.cache_misses.load(Ordering::Relaxed);
                tel.rate_sampler.sample(requests, hits, misses);
            }
        }
    });
}

#[cfg(target_os = "linux")]
fn get_process_rss_bytes() -> serde_json::Value {
    if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(pages) = parts[1].parse::<u64>() {
                let page_size = 4096u64;
                return json!({
                    "src": "measured",
                    "v": pages * page_size,
                    "unit": "bytes"
                });
            }
        }
    }
    json!({"src": "unavailable", "reason": "proc_statm_read_error", "unit": "bytes"})
}

#[cfg(target_os = "windows")]
fn get_process_rss_bytes() -> serde_json::Value {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            pmc: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }

    unsafe {
        let mut pmc: ProcessMemoryCounters = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
        let proc = GetCurrentProcess();
        if K32GetProcessMemoryInfo(proc, &mut pmc, pmc.cb) != 0 {
            return json!({
                "src": "measured",
                "v": pmc.working_set_size as u64,
                "unit": "bytes"
            });
        }
    }
    json!({"src": "unavailable", "reason": "win32_mem_info_failed", "unit": "bytes"})
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_process_rss_bytes() -> serde_json::Value {
    json!({"src": "unavailable", "reason": "unsupported_platform", "unit": "bytes"})
}

pub fn build_schema_v2_payload(state: &ApiState) -> serde_json::Value {
    let mode = if simulation_enabled() {
        "simulation"
    } else {
        "live"
    };

    let cluster_cap = if state.router.is_some() {
        "experimental"
    } else {
        "disabled"
    };
    let controller_cap = if state.pie.is_some() {
        "active"
    } else {
        "none"
    };
    let origin_cap = if state.origin_url.is_some() {
        "available"
    } else {
        "unavailable"
    };

    if let (Some(ref cache), Some(ref tel)) = (&state.cache, &state.telemetry) {
        let req = cache.counters.requests_total.load(Ordering::Relaxed);
        let h = cache.counters.cache_hits.load(Ordering::Relaxed);
        let m = cache.counters.cache_misses.load(Ordering::Relaxed);
        tel.rate_sampler.sample_if_due(req, h, m);
    }

    // BD-01/BD-02: with no cache, counters are unavailable (never a measured 0); the cumulative
    // hit ratio is unavailable until there is at least one hit or miss (never a measured 1.0).
    let cnt = |f: fn(&CacheCounters) -> u64| -> serde_json::Value {
        match state.cache.as_ref() {
            Some(c) => json!({"src": "measured", "v": f(&c.counters), "unit": "count"}),
            None => json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "count"}),
        }
    };
    let requests_total_val = cnt(|c| c.requests_total.load(Ordering::Relaxed));
    let cache_hits_val = cnt(|c| c.cache_hits.load(Ordering::Relaxed));
    let cache_misses_val = cnt(|c| c.cache_misses.load(Ordering::Relaxed));
    let coalesced_val = cnt(|c| c.coalesced_waiters.load(Ordering::Relaxed));
    let evictions_val = cnt(|c| c.evictions_total.load(Ordering::Relaxed));
    let cap_rejections_val = cnt(|c| c.capacity_rejections.load(Ordering::Relaxed));
    let hit_ratio_cum_val = match state.cache.as_ref() {
        None => json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "ratio"}),
        Some(cache) => {
            let r = cache.counters.requests_total.load(Ordering::Relaxed);
            let h = cache.counters.cache_hits.load(Ordering::Relaxed);
            let m = cache.counters.cache_misses.load(Ordering::Relaxed);
            let total = h + m;
            if total > 0 {
                json!({"src": "derived", "v": h as f64 / total as f64, "unit": "ratio"})
            } else if r == 0 {
                json!({"src": "unavailable", "reason": "no_requests", "unit": "ratio"})
            } else {
                json!({"src": "unavailable", "reason": "no_cache_lookups", "unit": "ratio"})
            }
        }
    };

    let (hit_ratio_10s_val, hit_ratio_1s_val, tp_10s_val, tp_1s_val) =
        if let Some(ref tel) = state.telemetry {
            let hr_10s = match tel.rate_sampler.hit_ratio_10s() {
                Some(v) => json!({"src": "derived", "v": v, "unit": "ratio"}),
                None => json!({"src": "unavailable", "reason": "insufficient_window", "unit": "ratio"}),
            };
            let hr_1s = match tel.rate_sampler.hit_ratio_1s() {
                Some(v) => json!({"src": "derived", "v": v, "unit": "ratio"}),
                None => json!({"src": "unavailable", "reason": "insufficient_window", "unit": "ratio"}),
            };
            let tp_10s = json!({"src": "derived", "v": tel.rate_sampler.throughput_10s(), "unit": "req/s"});
            let tp_1s = json!({"src": "derived", "v": tel.rate_sampler.throughput_1s(), "unit": "req/s"});
            (hr_10s, hr_1s, tp_10s, tp_1s)
        } else {
            (
                json!({"src": "unavailable", "reason": "telemetry_disabled", "unit": "ratio"}),
                json!({"src": "unavailable", "reason": "telemetry_disabled", "unit": "ratio"}),
                json!({"src": "unavailable", "reason": "telemetry_disabled", "unit": "req/s"}),
                json!({"src": "unavailable", "reason": "telemetry_disabled", "unit": "req/s"}),
            )
        };

    let latency_val = if let Some(ref tel) = state.telemetry {
        if !tel.is_empty() {
            let (p50_us, p90_us, p95_us, p99_us) = tel.calculate_percentiles_us();
            let (p50_ms, p90_ms, p95_ms, p99_ms) = tel.calculate_percentiles_f64_ms();
            json!({
                "src": "measured",
                "unit": "ms",
                "resolution": "microsecond",
                "p50": p50_ms,
                "p90": p90_ms,
                "p95": p95_ms,
                "p99": p99_ms,
                "p50_us": p50_us,
                "p90_us": p90_us,
                "p95_us": p95_us,
                "p99_us": p99_us,
                "samples": tel.len()
            })
        } else {
            json!({"src": "unavailable", "reason": "no_samples", "unit": "ms"})
        }
    } else {
        json!({"src": "unavailable", "reason": "no_samples", "unit": "ms"})
    };

    let origin_val = if state.origin_url.is_some() {
        if let Some(ref tel) = state.telemetry {
            let snap = tel.origin_metrics.snapshot();
            let lat_obj = match (snap.latency_p50, snap.latency_p95, snap.latency_p99) {
                (Some(p50), Some(p95), Some(p99)) => json!({
                    "src": "derived",
                    "unit": "ms",
                    "p50": p50,
                    "p95": p95,
                    "p99": p99
                }),
                _ => json!({"src": "unavailable", "reason": "no_samples", "unit": "ms"}),
            };
            json!({
                "requests_started": {"src": "measured", "v": snap.requests_started, "unit": "count"},
                "requests_completed_ok": {"src": "measured", "v": snap.requests_completed_ok, "unit": "count"},
                "in_flight": {"src": "measured", "v": snap.in_flight, "unit": "count"},
                "errors": {
                    "timeout": {"src": "measured", "v": snap.errors_timeout, "unit": "count"},
                    "network": {"src": "measured", "v": snap.errors_network, "unit": "count"},
                    "status_5xx": {"src": "measured", "v": snap.errors_status_5xx, "unit": "count"},
                    "other": {"src": "measured", "v": snap.errors_other, "unit": "count"}
                },
                "latency_ms": lat_obj
            })
        } else {
            json!({"src": "unavailable", "reason": "telemetry_disabled"})
        }
    } else {
        json!({"src": "unavailable", "reason": "origin_not_configured"})
    };

    // R-01: origin concurrency gate state. Each value is a direct atomic read at request time.
    let origin_cap_val = if let Some(exec) = state.intel_executor() {
        let g = exec.origin_gate_snapshot();
        let cap_field = match g.cap {
            Some(c) => json!({"src": "measured", "v": c, "unit": "count"}),
            None => json!({"src": "unavailable", "reason": "uncapped", "unit": "count"}),
        };
        json!({
            "enforcing": {"src": "measured", "v": g.enforcing},
            "cap": cap_field,
            "in_flight": {"src": "measured", "v": g.in_flight, "unit": "count"},
            "dispatched": {"src": "measured", "v": g.dispatched, "unit": "count"},
            "origin_cap_rejections": {"src": "measured", "v": g.rejections, "unit": "count"},
            "controller_steps": {"src": "measured", "v": g.controller_steps, "unit": "count"}
        })
    } else {
        json!({"src": "unavailable", "reason": "intelligence_not_installed"})
    };

    let rss_val = get_process_rss_bytes();

    let mut nodes_val = Vec::new();
    // BD-03: a standalone gateway (no router) has no cluster; state.nodes then holds placeholder
    // VirtualNodes created by main.rs. Never report those as live nodes. They are listed, labelled
    // simulated, only in simulation mode. With a router the state is configured, not probed: inferred.
    let state_src = if state.router.is_some() { "inferred" } else { "simulated" };
    let list_nodes = state.router.is_some() || mode == "simulation";
    for n in state.nodes.iter().filter(|_| list_nodes) {
        let (cpu, mem) = if mode == "simulation" {
            (json!({"src": "simulated", "v": 28.4}), json!({"src": "simulated", "v": 536870912u64}))
        } else {
            (
                json!({"src": "unavailable", "reason": "not_measured"}),
                json!({"src": "unavailable", "reason": "not_measured"}),
            )
        };
        nodes_val.push(json!({
            "node_id": n.node_id,
            "state": format!("{:?}", n.get_state()),
            "state_src": state_src,
            "cpu": cpu,
            "memory": mem,
        }));
    }

    let (bytes_resident_val, byte_budget_val, byte_budget_rej_val, oversize_rej_val) =
        if let Some(ref cache) = state.cache {
            (
                json!({
                    "src": "measured",
                    "v": cache.bytes_resident(),
                    "basis": "logical_accounting",
                    "unit": "bytes"
                }),
                json!({
                    "src": "measured",
                    "v": cache.byte_budget(),
                    "unit": "bytes"
                }),
                json!({
                    "src": "measured",
                    "v": cache.counters.byte_budget_rejections.load(Ordering::Relaxed),
                    "unit": "count"
                }),
                json!({
                    "src": "measured",
                    "v": cache.counters.oversize_rejections.load(Ordering::Relaxed),
                    "unit": "count"
                }),
            )
        } else {
            (
                json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "bytes"}),
                json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "bytes"}),
                json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "count"}),
                json!({"src": "unavailable", "reason": "cache_unavailable", "unit": "count"}),
            )
        };

    // BD-02: no telemetry collector means no uptime clock; do not report a measured 0.
    let uptime_val = match state.telemetry.as_ref() {
        Some(t) => json!(t.rate_sampler.uptime_ms()),
        None => json!({"src": "unavailable", "reason": "telemetry_disabled", "unit": "ms"}),
    };

    // Cluster observability: broadcaster atomics and per-peer connection states (WP8).
    // `peers` lists only REMOTE members; this gateway's own node is reported under `local`
    // (PC-AUTO-039). Every value carries `src`.
    let cluster_metrics_val = if let Some(ref router) = state.router {
        let c = router.get_broadcaster_counters();
        let local_id = router.get_node_id();
        let peer_obs = router.get_peer_observability();
        let reach: std::collections::HashMap<u8, (&'static str, u32)> = router
            .get_peer_reachability()
            .into_iter()
            .map(|(id, label, streak)| (id, (label, streak)))
            .collect();
        let count = |v: usize| json!({"src": "measured", "v": v, "unit": "count"});
        let state_val = |s: crate::dist::node::NodeState| json!({"src": "measured", "v": format!("{:?}", s)});
        let mut local_val = json!({"src": "unavailable", "reason": "local_node_not_configured"});
        let mut peers_val: Vec<serde_json::Value> = Vec::new();
        for (id, node_state, idle, active) in peer_obs {
            if id == local_id {
                local_val = json!({"node_id": id, "state": state_val(node_state)});
            } else {
                let (reach_label, reach_streak) = reach.get(&id).copied().unwrap_or(("reachable", 0));
                peers_val.push(json!({
                    "node_id": id,
                    "state": state_val(node_state),
                    // `state` is the routing state machine, not a liveness verdict (a dead peer can sit in
                    // Recovering). `reachability` is inferred from the outcome of recent connection and frame
                    // attempts (PC-AUTO-044).
                    "reachability": {"src": "inferred", "v": reach_label},
                    "consecutive_failed_attempts": count(reach_streak as usize),
                    "idle_connections": count(idle),
                    "active_connections": count(active)
                }));
            }
        }

        json!({
            "broadcaster": {
                "enqueued": count(c.enqueued),
                "sent": count(c.sent),
                "retried": count(c.retried),
                "acked": count(c.acked),
                "dropped": count(c.dropped),
                "exhausted": count(c.exhausted)
            },
            "local": local_val,
            "peers": peers_val
        })
    } else {
        json!({"src": "unavailable", "reason": "cluster_not_configured"})
    };

    let mut root = json!({
        "schema": 2,
        "mode": mode,
        "as_of_ms": DiagnosticBuffer::now_ms(),
        "uptime_ms": uptime_val,
        "capabilities": {
            "cluster": cluster_cap,
            "controller": controller_cap,
            "origin_metrics": origin_cap,
        },
        "metrics": {
            "requests_total": requests_total_val,
            "cache_hits": cache_hits_val,
            "cache_misses": cache_misses_val,
            "coalesced_waiters": coalesced_val,
            "evictions_total": evictions_val,
            "capacity_rejections": cap_rejections_val,
            "hit_ratio_cumulative": hit_ratio_cum_val,
            "hit_ratio_10s": hit_ratio_10s_val,
            "hit_ratio_1s": hit_ratio_1s_val,
            "throughput_req_s_10s": tp_10s_val,
            "throughput_req_s_1s": tp_1s_val,
            "latency_ms": latency_val,
            "bytes_resident": bytes_resident_val,
            "byte_budget": byte_budget_val,
            "byte_budget_rejections": byte_budget_rej_val,
            "oversize_rejections": oversize_rej_val,
            "stale_served_total": cnt(|c| c.stale_served_total.load(Ordering::Relaxed)),
            "stale_cap_violations": cnt(|c| c.stale_cap_violations.load(Ordering::Relaxed)),
            "revalidations": {
                "started": cnt(|c| c.revalidation_started.load(Ordering::Relaxed)),
                "ok": cnt(|c| c.revalidation_ok.load(Ordering::Relaxed)),
                "not_modified": cnt(|c| c.revalidation_not_modified.load(Ordering::Relaxed)),
                "error": cnt(|c| c.revalidation_error.load(Ordering::Relaxed)),
                "in_flight": cnt(|c| c.background_refresh_in_flight.load(Ordering::Relaxed))
            },
            
"origin": origin_val,
            "origin_cap": origin_cap_val,
            "origin_static_cap": static_origin_cap_json(),
            "process": {
                "rss_bytes": rss_val
            },
            "cluster": cluster_metrics_val,
            "outcomes": outcome_counters().to_json()
        },
        "hot_keys": {"src": "unavailable", "reason": "not_implemented"},
        "nodes": nodes_val,
    });

    if mode == "simulation" {
        let snap = state.snapshot.lock().unwrap();
        if let Some(obj) = root.as_object_mut() {
            obj.insert("backend_load_ratio".to_string(), json!({"src": "simulated", "v": snap.db_load_ratio}));
            obj.insert("avoided_compute_dollars".to_string(), json!({"src": "simulated", "v": snap.avoided_compute_dollars}));
            obj.insert("cost".to_string(), json!({"src": "simulated", "v": 0.05}));
        }
    }

    root
}

async fn metrics_handler(
    State(state): State<Arc<ApiState>>,
    raw_query: RawQuery,
    headers: HeaderMap,
) -> Response {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            if let Err(resp) = authenticate_request(&headers, sec, crate::security::Role::Observer) {
                return resp;
            }
        }
    }
    let is_schema_v1 = raw_query
        .0
        .as_deref()
        .map(|q| q.split('&').any(|p| p == "schema=1" || p.starts_with("schema=1")))
        .unwrap_or(false);

    if is_schema_v1 {
        let mut resp = Json(build_schema_v2_payload(&state)).into_response();
        resp.headers_mut().insert("Deprecation", HeaderValue::from_static("true"));
        resp.headers_mut().insert("Sunset", HeaderValue::from_static("Wed, 31 Dec 2026 23:59:59 GMT"));
        resp.headers_mut().insert(
            "Link",
            HeaderValue::from_static("</api/v1/metrics?schema=2>; rel=\"successor-version\""),
        );
        resp
    } else {
        Json(build_schema_v2_payload(&state)).into_response()
    }
}

/// Schema 1 (unversioned, deprecated) `/api/events` payload. The v1 field names and types are frozen
/// (approval A-3) except for documented corrections. Correction (S-7): when PHANTOM_SIMULATION is not
/// enabled, values that only exist as synthetic placeholders (startup snapshot, hard-coded hot key,
/// placeholder nodes) are no longer emitted as live. They become `null` (scalars) or `[]` (lists),
/// and the additive top-level `v1_provenance` object states the provenance of every such field.
/// `v1_provenance.fields` keys are dotted paths into this payload; values are one of
/// measured | derived | simulated | unavailable. Unknown clients may ignore `v1_provenance`.
pub fn build_schema_v1_payload(state: &ApiState) -> serde_json::Value {
    let sim = simulation_enabled();
    let snap = state.snapshot.lock().unwrap().clone();
    let (h, m, c, e, cap) = match state.cache.as_ref() {
        Some(cache) => (
            cache.counters.cache_hits.load(Ordering::Relaxed),
            cache.counters.cache_misses.load(Ordering::Relaxed),
            cache.counters.coalesced_waiters.load(Ordering::Relaxed),
            cache.counters.evictions_total.load(Ordering::Relaxed),
            cache.counters.capacity_rejections.load(Ordering::Relaxed),
        ),
        None => (0, 0, 0, 0, 0),
    };
    let total = h + m;
    let have_cache = state.cache.is_some();

    // hit_ratio: legacy value was 1.0 with no lookups and 0.0 without a cache. Live mode: null instead.
    let hit_ratio = if total > 0 {
        json!(h as f64 / total as f64)
    } else if sim {
        json!(if have_cache { 1.0 } else { 0.0 })
    } else {
        serde_json::Value::Null
    };

    // Latency percentiles: measured by the telemetry ring when it has samples.
    let live_latency = state.telemetry.as_ref().filter(|t| !t.is_empty()).map(|t| t.calculate_percentiles_v2());
    let (p50, p90, p95, p99, lat_prov) = match (live_latency, sim) {
        (Some((a, b, c2, d)), _) if have_cache => (json!(a), json!(b), json!(c2), json!(d), "measured"),
        (_, true) => (
            json!(snap.p50_latency_ms),
            json!(snap.p95_latency_ms),
            json!(snap.p95_latency_ms),
            json!(snap.p99_latency_ms),
            "simulated",
        ),
        _ => (
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::Value::Null,
            serde_json::Value::Null,
            "unavailable",
        ),
    };

    let (throughput, throughput_prov) = if sim {
        (json!(snap.throughput_req_sec), "simulated")
    } else if let Some(ref tel) = state.telemetry {
        (json!(tel.rate_sampler.throughput_1s()), "derived")
    } else {
        (serde_json::Value::Null, "unavailable")
    };

    let (backend_load, avoided, nodes, hot_keys, synth_prov, timestamp_ms) = if sim {
        (
            json!(snap.db_load_ratio),
            json!(snap.avoided_compute_dollars),
            json!(snap.nodes),
            json!([{ "key": "/availability/hotel/123", "req_sec": 9200.0, "hit_rate": 0.99 }]),
            "simulated",
            snap.timestamp_ms,
        )
    } else {
        (
            serde_json::Value::Null,
            serde_json::Value::Null,
            json!([]),
            json!([]),
            "unavailable",
            DiagnosticBuffer::now_ms(),
        )
    };

    let counter_prov = if have_cache { "measured" } else { "unavailable" };
    json!({
        "version": snap.version,
        "timestamp_ms": timestamp_ms,
        "nodes": nodes,
        "metrics": {
            "throughput_req_sec": throughput,
            "p50_latency_ms": p50,
            "p90_latency_ms": p90,
            "p95_latency_ms": p95,
            "p99_latency_ms": p99,
            "backend_load_ratio": backend_load,
            "avoided_compute_dollars": avoided,
            "cache_hits": h,
            "cache_misses": m,
            "hit_ratio": hit_ratio,
            "coalesced_waiters": c,
            "evictions_total": e,
            "capacity_rejections": cap
        },
        "hot_keys": hot_keys,
        "v1_provenance": {
            "mode": if sim { "simulation" } else { "live" },
            "note": "Additive. Fields marked unavailable are null or empty in live mode; counters are 0 when the cache is absent (provenance unavailable).",
            "fields": {
                "timestamp_ms": if sim { "simulated" } else { "measured" },
                "nodes": synth_prov,
                "hot_keys": synth_prov,
                "metrics.throughput_req_sec": throughput_prov,
                "metrics.p50_latency_ms": lat_prov,
                "metrics.p90_latency_ms": lat_prov,
                "metrics.p95_latency_ms": lat_prov,
                "metrics.p99_latency_ms": lat_prov,
                "metrics.backend_load_ratio": synth_prov,
                "metrics.avoided_compute_dollars": synth_prov,
                "metrics.cache_hits": counter_prov,
                "metrics.cache_misses": counter_prov,
                "metrics.coalesced_waiters": counter_prov,
                "metrics.evictions_total": counter_prov,
                "metrics.capacity_rejections": counter_prov,
                "metrics.hit_ratio": if total > 0 { "derived" } else if sim { "simulated" } else { "unavailable" }
            }
        }
    })
}

async fn sse_handler(
    State(state): State<Arc<ApiState>>,
    raw_query: RawQuery,
    headers: HeaderMap,
) -> Response {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            if let Err(resp) = authenticate_request(&headers, sec, crate::security::Role::Observer) {
                return resp;
            }
        }
    }

    let is_schema_v2 = raw_query
        .0
        .as_deref()
        .map(|q| q.split('&').any(|p| p == "schema=2" || p.starts_with("schema=2")))
        .unwrap_or(false);

    let interval = tokio::time::interval(Duration::from_millis(100));
    let stream = IntervalStream::new(interval).map(move |_| {
        if is_schema_v2 {
            let payload = build_schema_v2_payload(&state);
            return Ok::<Event, Infallible>(Event::default().json_data(payload).unwrap());
        }

        let payload = build_schema_v1_payload(&state);
        Ok::<Event, Infallible>(Event::default().json_data(payload).unwrap())
    });

    let mut resp = Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response();
    if !is_schema_v2 {
        resp.headers_mut().insert("Deprecation", HeaderValue::from_static("true"));
        resp.headers_mut().insert("Sunset", HeaderValue::from_static("Wed, 31 Dec 2026 23:59:59 GMT"));
        resp.headers_mut().insert(
            "Link",
            HeaderValue::from_static("</api/events?schema=2>; rel=\"successor-version\""),
        );
    }
    resp
}

#[allow(clippy::result_large_err)]
fn authenticate_request(
    headers: &HeaderMap,
    sec: &crate::security::SecurityConfig,
    required_role: crate::security::Role,
) -> Result<crate::security::JwtClaims, Response> {
    let auth_header = headers.get("authorization").and_then(|v| v.to_str().ok());
    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => h[7..].trim(),
        Some(h) => h.trim(),
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                [("X-Auth-Error", "MissingToken"), ("WWW-Authenticate", "Bearer")],
                "Missing Bearer authorization token",
            )
                .into_response());
        }
    };

    let validator = match sec.jwt_validator.as_ref() {
        Some(v) => v,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                [("X-Auth-Error", "MisconfiguredValidator")],
                "Security validator uninitialized",
            )
                .into_response());
        }
    };

    let claims = match validator.validate(token) {
        Ok(c) => c,
        Err(e) => {
            let err_code = match e {
                crate::security::AuthError::ExpiredToken => "ExpiredToken",
                crate::security::AuthError::InvalidIssuer => "InvalidIssuer",
                crate::security::AuthError::InvalidAudience => "InvalidAudience",
                _ => "InvalidToken",
            };
            return Err((
                StatusCode::UNAUTHORIZED,
                [("X-Auth-Error", err_code), ("WWW-Authenticate", "Bearer")],
                "Authentication failed",
            )
                .into_response());
        }
    };

    if !claims.has_role(required_role) {
        return Err((
            StatusCode::FORBIDDEN,
            [("X-Auth-Error", "InsufficientPermissions")],
            format!("Forbidden: {} role required", required_role.as_str()),
        )
            .into_response());
    }

    Ok(claims)
}

/// M1 (round-2 review): in secure mode cache reads and invalidations need a tenant. A token with
/// no (or empty) `tenant_id` would otherwise share the `__default__` namespace with every other
/// tenant-less principal, and a claim equal to `__default__` would address that shared namespace
/// directly. The principal is authenticated but has no usable tenant, hence 403 (not 401).
/// Alphabet/length of the claim itself are enforced later by `CacheKey::validate` (400).
#[allow(clippy::result_large_err)]
fn require_tenant_claim(claims: crate::security::JwtClaims) -> Result<String, Response> {
    match claims.tenant_id {
        Some(t) if t == "__default__" => Err((
            StatusCode::FORBIDDEN,
            [("X-Auth-Error", "ReservedTenant")],
            "Forbidden: the tenant_id claim uses a reserved value",
        )
            .into_response()),
        Some(t) if !t.is_empty() => Ok(t),
        _ => Err((
            StatusCode::FORBIDDEN,
            [("X-Auth-Error", "MissingTenant")],
            "Forbidden: this operation requires a token with a non-empty tenant_id claim",
        )
            .into_response()),
    }
}

async fn control_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            if let Err(resp) = authenticate_request(&headers, sec, crate::security::Role::Operator) {
                return resp;
            }
        }
    }

    let event = payload["event"].as_str().unwrap_or("NORMAL");
    println!("[Phantom Control] Received event trigger: {}", event);

    if event == "FLASH_CROWD" {
        let mut wl = state.workload_type.lock().unwrap();
        *wl = "FlashCrowd".to_string();
    } else if event == "NODE_FAILURE" {
        if let Some(n) = state.nodes.iter().find(|x| x.node_id == 2) {
            n.set_state(NodeState::Failed);
        }
    } else {
        let mut wl = state.workload_type.lock().unwrap();
        *wl = "Normal".to_string();
        for n in &state.nodes {
            n.set_state(NodeState::Active);
        }
    }

    Json(json!({ "status": "success", "triggered": event })).into_response()
}

// ==========================================
// Milestone 1 API Handlers
// ==========================================

async fn get_cache_handler_root(
    state: State<Arc<ApiState>>,
    raw_query: RawQuery,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    get_cache_handler(state, Path("".to_string()), raw_query, headers, body).await
}

async fn delete_cache_handler_root(
    state: State<Arc<ApiState>>,
    raw_query: RawQuery,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    delete_cache_handler(state, Path("".to_string()), raw_query, headers, body).await
}

/// Non-secure mode tenant from `x-tenant-id`. A header that is present but not valid visible
/// ASCII is an error (400): it must NOT silently fall back to the default tenant (M2). The
/// alphabet/length of the value itself is enforced by `CacheKey::validate`.
fn header_tenant(headers: &HeaderMap) -> Result<Option<&str>, OriginError> {
    match headers.get("x-tenant-id") {
        None => Ok(None),
        Some(v) => v.to_str().map(Some).map_err(|_| OriginError::InvalidTenantId),
    }
}

async fn get_cache_handler(
    State(state): State<Arc<ApiState>>,
    Path(key_path): Path<String>,
    raw_query: RawQuery,
    headers: HeaderMap,
    _body: axum::body::Bytes,
) -> Response {
    // SEC-M7-01: authenticate/authorize FIRST, before the cache lookup, tenant resolution,
    // accounting, coalescing, origin fetch or permit acquisition. This applies to every branch
    // below (including `x-cache-bypass`). When JWT security is enabled, a valid token with
    // `cache:read` is mandatory: 401 (missing/invalid) or 403 (lacking the role). The tenant is
    // taken from the verified token only.
    let mut authenticated_tenant: Option<String> = None;
    let has_auth_header = headers.get("authorization").and_then(|v| v.to_str().ok()).is_some();

    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            match authenticate_request(&headers, sec, crate::security::Role::CacheRead)
                .and_then(require_tenant_claim)
            {
                Ok(t) => authenticated_tenant = Some(t),
                Err(resp) => return resp,
            }
        } else if has_auth_header {
            // Auth enforcement was explicitly disabled by the embedding code; a token that is
            // presented must still be valid and carry cache:read (previous behaviour, unchanged).
            if let Some(ref validator) = sec.jwt_validator {
                let auth_val = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
                let token = match auth_val {
                    h if h.starts_with("Bearer ") => h[7..].trim(),
                    h => h.trim(),
                };
                match validator.validate(token) {
                    Ok(claims) => {
                        if !claims.has_role(crate::security::Role::CacheRead) {
                            return (
                                StatusCode::FORBIDDEN,
                                [("X-Auth-Error", "InsufficientPermissions")],
                                "Forbidden: cache:read role required",
                            )
                                .into_response();
                        }
                        match require_tenant_claim(claims) {
                            Ok(t) => authenticated_tenant = Some(t),
                            Err(resp) => return resp,
                        }
                    }
                    Err(e) => {
                        let err_code = match e {
                            crate::security::AuthError::ExpiredToken => "ExpiredToken",
                            crate::security::AuthError::InvalidIssuer => "InvalidIssuer",
                            crate::security::AuthError::InvalidAudience => "InvalidAudience",
                            _ => "InvalidToken",
                        };
                        return (
                            StatusCode::UNAUTHORIZED,
                            [("X-Auth-Error", err_code), ("WWW-Authenticate", "Bearer")],
                            "Authentication failed",
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    let cache = match state.cache.as_ref() {
        Some(c) => c,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cache uninitialized").into_response();
        }
    };

    let tenant_id = if let Some(ref t) = authenticated_tenant {
        Some(t.as_str())
    } else if state.security.is_some() {
        // Secure mode: caller-supplied x-tenant-id is NEVER trusted
        None
    } else {
        match header_tenant(&headers) {
            Ok(t) => t,
            Err(e) => return map_origin_error_to_response(&e),
        }
    };

    let mut clean_headers = headers.clone();
    // The cached representation is shared across clients: the CLIENT's own conditional/range
    // headers are never forwarded upstream (fetch_origin_raw also refuses them). `headers`
    // keeps them for answering the client's conditional request from the cache.
    for name in ["if-none-match", "if-modified-since", "if-match", "if-unmodified-since", "if-range", "range"] {
        clean_headers.remove(name);
    }
    if let Some(t) = tenant_id {
        if let Ok(val) = HeaderValue::from_str(t) {
            clean_headers.insert("x-tenant-id", val);
        }
    } else if state.security.is_some() {
        clean_headers.remove("x-tenant-id");
    }

    let force_auth_bypass = has_auth_header
        && state
            .security
            .as_ref()
            .map(|s| s.enforce_rfc9111_auth_bypass)
            .unwrap_or(true);

    let is_bypass = headers
        .get("x-cache-bypass")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let norm_path = if key_path.starts_with('/') {
        key_path
    } else {
        format!("/{}", key_path)
    };

    let cache_key = CacheKey::new(tenant_id, &norm_path, raw_query.0.as_deref());
    if let Err(e) = cache_key.validate() {
        return map_origin_error_to_response(&e);
    }
    let start_instant = std::time::Instant::now();
    let now_ms = DiagnosticBuffer::now_ms();
    let key_hash = crate::intel::telemetry::fnv1a_64(&cache_key.to_canonical_string());

    cache.counters.requests_total.fetch_add(1, Ordering::Relaxed);

    if is_bypass || force_auth_bypass {
        cache
            .counters
            .cache_bypasses
            .fetch_add(1, Ordering::Relaxed);
        let origin_base = state
            .origin_url
            .as_deref()
            .unwrap_or("http://127.0.0.1:0");
        let upstream_pol = state.get_upstream_policy();
        let exec_for_bypass = state.intel_executor();
        return match fetch_origin_gated(exec_for_bypass.as_ref(), origin_base, &cache_key, &clean_headers, upstream_pol.as_deref()).await {
            Ok((status, headers_out, body)) => {
                let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
                let mut resp = (status_code, body).into_response();
                let status_header = if force_auth_bypass { "BYPASS-AUTH" } else { "BYPASS" };
                resp.headers_mut()
                    .insert("X-Cache-Status", HeaderValue::from_static(status_header));
                mark_source(&mut resp, "origin");
                for (k, v) in headers_out {
                    if let Some(name) = k {
                        // Round 3: the body was already de-framed (chunked decoded, length
                        // checked) and axum re-frames the response itself. Copying the origin's
                        // framing / hop-by-hop headers would send a stale Content-Length or a
                        // `Transfer-Encoding: chunked` for a body that is no longer chunked.
                        if is_hop_by_hop_or_framing_header(name.as_str()) {
                            continue;
                        }
                        resp.headers_mut().insert(name, v);
                    }
                }
                if let Some(ref tel) = state.telemetry {
                    let elapsed_us = start_instant.elapsed().as_micros() as u64;
                    tel.record_us(crate::intel::telemetry::RawObservationUs::new_get(
                        now_ms,
                        key_hash,
                        elapsed_us.max(1),
                        false,
                        0,
                    ));
                }
                resp
            }
            Err(e) => {
                if let Some(ref tel) = state.telemetry {
                    tel.record_error();
                }
                map_origin_error_to_response(&e)
            }
        };
    }



    let hard_cap_ms = cache.get_max_stale_ms();
    let stale_check = cache.check_stale_serving(&cache_key, now_ms, hard_cap_ms, false);
    if let crate::core::cache::StaleServingDecision::Eligible(stale_res, _stale_meta) = stale_check {
        let age_ms = now_ms.saturating_sub(stale_res.created_at_ms);
        let age_secs = crate::core::freshness::calculate_current_age_seconds(stale_res.created_at_ms, 0, now_ms);
        cache.record_stale_served(age_ms, stale_res.ttl_ms, hard_cap_ms);

        let fp = cache.diagnostics.hasher.fingerprint(&cache_key.to_canonical_string());
        cache.diagnostics.push(DiagnosticEvent {
            timestamp_epoch_ms: now_ms,
            severity: DiagnosticLevel::INFO,
            event_name: "CacheStaleServed",
            key_fp: fp,
            op_id: None,
            op_gen: None,
            outcome: "Stale",
            duration_ms: Some(0),
            error_msg: None,
        });

        // Trigger bounded single-flight background revalidation if due
        if cache.is_revalidation_due(&cache_key, now_ms) {
            let max_bg = cache.get_max_background_refresh() as u64;
            // M-b: reserve the slot atomically BEFORE spawning (check-then-increment inside the
            // task let a burst exceed the bound). The task releases it when it ends.
            let reserved_slot = cache.try_reserve_background_slot(max_bg);
            if let Some(bg_slot) = reserved_slot {
                let cache_clone = cache.clone();
                let key_for_bg = cache_key.clone();
                let origin_base_bg = state.origin_url.clone().unwrap_or_else(|| "http://127.0.0.1:0".to_string());
                // Client conditional/range headers were stripped from `clean_headers` above; the
                // validators sent upstream are read from the resident entry AFTER registration.
                let headers_for_bg = clean_headers.clone();
                let policy_for_bg = state.get_upstream_policy();
                let exec_for_bg = state.intel_executor();
                tokio::spawn(async move {
                    // L1: RAII slot, released on every exit including panic/cancellation.
                    let _bg_slot = bg_slot;
                    // Revalidate via single-flight begin_fetch_atomic
                    let reg_res = cache_clone.begin_fetch_atomic(&key_for_bg, DiagnosticBuffer::now_ms());
                    if let FetchRegistrationResult::Initiate(reg) = reg_res {
                        // M-b: only a real single-flight leader counts as a started revalidation.
                        cache_clone.counters.revalidation_started.fetch_add(1, Ordering::Relaxed);
                        let guard = reg.into_guard();
                        let fetch_start = DiagnosticBuffer::now_ms();
                        // Validators of the entry resident now (H1), not of the copy served above.
                        let validators = stored_validators(&cache_clone, &key_for_bg);
                        let res = match acquire_origin_permit(exec_for_bg.as_ref()) {
                            Ok(permit) => {
                                if validators.is_some() {
                                    cache_clone.counters.revalidation_conditional_sent.fetch_add(1, Ordering::Relaxed);
                                }
                                fetch_origin_permitted(permit, &origin_base_bg, &key_for_bg, &headers_for_bg, validators.as_ref(), policy_for_bg.as_deref()).await
                            }
                            Err(e) => Err(e),
                        };
                        let _duration = DiagnosticBuffer::now_ms().saturating_sub(fetch_start);

                        match res {
                            Ok((304, headers_out, _)) => {
                                if validators.is_some() {
                                    cache_clone.counters.revalidation_304_received.fetch_add(1, Ordering::Relaxed);
                                }
                                // S-1/M2/H1/M-a: all 304 decisions use the state resident AFTER
                                // registration (never a copy captured before it) and live in
                                // PartitionedCache::complete_not_modified.
                                let now_reval = DiagnosticBuffer::now_ms();
                                let has_auth = headers_for_bg.contains_key("authorization");
                                let _ = cache_clone.complete_not_modified(guard, &headers_out, now_reval, has_auth);
                            }
                            Ok((status, headers_out, body)) if status == 200 => {
                                let now_reval = DiagnosticBuffer::now_ms();
                                let has_explicit_framing = headers_out.iter().any(|(k, _)| {
                                    if let Some(name) = k {
                                        let n = name.as_str();
                                        return n.eq_ignore_ascii_case("content-length")
                                            || n.eq_ignore_ascii_case("transfer-encoding");
                                    }
                                    false
                                });
                                let has_auth = headers_for_bg.contains_key("authorization");
                                let decision = crate::core::freshness::evaluate_response_freshness(
                                    status,
                                    &headers_out,
                                    now_reval,
                                    has_explicit_framing,
                                    body.len(),
                                    60_000,
                                    has_auth,
                                );
                                let content_type = headers_out
                                    .iter()
                                    .find_map(|(k, v)| {
                                        if let Some(name) = k {
                                            if name.as_str().eq_ignore_ascii_case("content-type") {
                                                return v.to_str().ok().map(|s| s.to_string());
                                            }
                                        }
                                        None
                                    })
                                    .unwrap_or_else(|| "application/json; charset=utf-8".to_string());

                                let resource = Arc::new(CachedResource {
                                    status,
                                    content_type,
                                    body: Arc::new(body),
                                    etag: decision.etag,
                                    created_at_ms: now_reval,
                                    ttl_ms: decision.ttl_ms,
                                });

                                if decision.is_cacheable {
                                    cache_clone.update_freshness_for_guard(
                                        &guard,
                                        crate::core::freshness::FreshnessMetadata {
                                            must_revalidate: decision.must_revalidate,
                                            stale_while_revalidate_ms: decision.stale_while_revalidate_ms,
                                            stale_if_error_ms: decision.stale_if_error_ms,
                                            last_modified: decision.last_modified,
                                            date_ms: decision.date_ms,
                                            initial_age_ms: decision.initial_age_ms,
                                        },
                                    );
                                }

                                cache_clone.counters.revalidation_ok.fetch_add(1, Ordering::Relaxed);
                                let outcome = if decision.is_cacheable {
                                    FinalizeOutcome::Success(resource)
                                } else {
                                    FinalizeOutcome::Uncacheable(resource)
                                };
                                if let Some(mut batch) = guard.complete(outcome) {
                                    batch.deliver();
                                }
                            }
                            Ok((status, _, _)) => {
                                cache_clone.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                                cache_clone.record_revalidation_failure(&key_for_bg, DiagnosticBuffer::now_ms());
                                if let Some(mut batch) = guard.complete(FinalizeOutcome::Error(Arc::new(OriginError::UpstreamStatus(status)))) {
                                    batch.deliver();
                                }
                            }
                            Err(e) => {
                                cache_clone.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                                cache_clone.record_revalidation_failure(&key_for_bg, DiagnosticBuffer::now_ms());
                                if let Some(mut batch) = guard.complete(FinalizeOutcome::Error(Arc::new(e))) {
                                    batch.deliver();
                                }
                            }
                        }
                    } else {
                        // Joined an in-flight fetch, hit a fresh entry, or was rejected.
                        cache_clone.counters.revalidation_not_initiated.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        }

        // Return stale response immediately to client
        let is_not_modified = if let Some(inm) = headers.get("if-none-match").and_then(|v| v.to_str().ok()) {
            crate::core::freshness::matches_if_none_match(inm, stale_res.etag.as_deref())
        } else if let Some(ims) = headers.get("if-modified-since").and_then(|v| v.to_str().ok()) {
            crate::core::freshness::matches_if_modified_since(ims, None, stale_res.created_at_ms)
        } else {
            false
        };

        let mut resp = if is_not_modified {
            StatusCode::NOT_MODIFIED.into_response()
        } else {
            let status_code = StatusCode::from_u16(stale_res.status).unwrap_or(StatusCode::OK);
            let mut r = (status_code, (*stale_res.body).clone()).into_response();
            r.headers_mut().insert(
                "Content-Type",
                HeaderValue::from_str(&stale_res.content_type)
                    .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
            );
            r
        };

        resp.headers_mut().insert("X-Cache-Status", HeaderValue::from_static("STALE"));
        mark_source(&mut resp, "stale");
        resp.headers_mut().insert("Age", HeaderValue::from(age_secs));
        if let Some(ref etag) = stale_res.etag {
            if let Ok(val) = HeaderValue::from_str(etag) {
                resp.headers_mut().insert("ETag", val);
            }
        }

        if let Some(ref tel) = state.telemetry {
            let elapsed_us = start_instant.elapsed().as_micros() as u64;
            tel.record_us(crate::intel::telemetry::RawObservationUs::new_get(
                now_ms,
                key_hash,
                elapsed_us.max(1),
                true,
                0,
            ));
        }

        return resp;
    }

    let (resp, hit, coalesced, is_err) = match cache.begin_fetch_atomic(&cache_key, now_ms) {
        FetchRegistrationResult::Hit(resource) => {
            let fp = cache
                .diagnostics
                .hasher
                .fingerprint(&cache_key.to_canonical_string());
            cache.diagnostics.push(DiagnosticEvent {
                timestamp_epoch_ms: now_ms,
                severity: DiagnosticLevel::INFO,
                event_name: "CacheHit",
                key_fp: fp,
                op_id: None,
                op_gen: None,
                outcome: "Hit",
                duration_ms: Some(0),
                error_msg: None,
            });

            let age_secs = crate::core::freshness::calculate_current_age_seconds(
                resource.created_at_ms,
                0,
                now_ms,
            );

            // Conditional request checks (RFC 9110 §13.1 / RFC 9111 §3.2)
            let is_not_modified = if let Some(inm) = headers.get("if-none-match").and_then(|v| v.to_str().ok()) {
                crate::core::freshness::matches_if_none_match(inm, resource.etag.as_deref())
            } else if let Some(ims) = headers.get("if-modified-since").and_then(|v| v.to_str().ok()) {
                crate::core::freshness::matches_if_modified_since(ims, None, resource.created_at_ms)
            } else {
                false
            };

            if is_not_modified {
                let mut resp = StatusCode::NOT_MODIFIED.into_response();
                resp.headers_mut()
                    .insert("X-Cache-Status", HeaderValue::from_static("HIT"));
                mark_source(&mut resp, "local");
                resp.headers_mut().insert("Age", HeaderValue::from(age_secs));
                if let Some(ref etag) = resource.etag {
                    if let Ok(val) = HeaderValue::from_str(etag) {
                        resp.headers_mut().insert("ETag", val);
                    }
                }
                (resp, true, 0, false)
            } else {
                let status_code = StatusCode::from_u16(resource.status).unwrap_or(StatusCode::OK);
                let mut resp = (status_code, (*resource.body).clone()).into_response();
                resp.headers_mut().insert(
                    "Content-Type",
                    HeaderValue::from_str(&resource.content_type)
                        .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
                );
                resp.headers_mut()
                    .insert("X-Cache-Status", HeaderValue::from_static("HIT"));
                mark_source(&mut resp, "local");
                resp.headers_mut().insert("Age", HeaderValue::from(age_secs));
                if let Some(ref etag) = resource.etag {
                    if let Ok(val) = HeaderValue::from_str(etag) {
                        resp.headers_mut().insert("ETag", val);
                    }
                }
                (resp, true, 0, false)
            }
        }
        FetchRegistrationResult::Wait(rx) => match rx.await {
            Ok(Ok(resource)) => {
                let age_secs = crate::core::freshness::calculate_current_age_seconds(
                    resource.created_at_ms,
                    0,
                    DiagnosticBuffer::now_ms(),
                );
                let status_code = StatusCode::from_u16(resource.status).unwrap_or(StatusCode::OK);
                let mut resp = (status_code, (*resource.body).clone()).into_response();
                resp.headers_mut().insert(
                    "Content-Type",
                    HeaderValue::from_str(&resource.content_type)
                        .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
                );
                let is_stale = resource.created_at_ms < now_ms && resource.created_at_ms.saturating_add(resource.ttl_ms) <= now_ms;
                let status_header = if is_stale { "STALE" } else { "COALESCED" };
                resp.headers_mut()
                    .insert("X-Cache-Status", HeaderValue::from_static(status_header));
                mark_source(&mut resp, if is_stale { "stale" } else { "coalesced" });
                resp.headers_mut().insert("Age", HeaderValue::from(age_secs));
                if let Some(ref etag) = resource.etag {
                    if let Ok(val) = HeaderValue::from_str(etag) {
                        resp.headers_mut().insert("ETag", val);
                    }
                }
                (resp, true, 1, false)
            }
            Ok(Err(err)) => (map_origin_error_to_response(&err), false, 1, true),
            Err(_) => (
                (
                    StatusCode::BAD_GATEWAY,
                    [("X-Cache-Error", "InitiatorCancelled")],
                    "Origin fetch aborted",
                )
                    .into_response(),
                false,
                1,
                true,
            ),
        },
        FetchRegistrationResult::Initiate(registration) => {
            let origin_base = state
                .origin_url
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:0".to_string());
            let (my_tx, my_rx) = oneshot::channel();

            // Register myself as a waiter on registration.shared_op
            {
                let mut guard = registration.shared_op.waiters.lock().unwrap();
                guard.push(my_tx);
            }

            let op_id = registration.op_id;
            let op_gen = registration.op_gen;
            let guard = registration.into_guard();
            let key_for_task = cache_key.clone();
            let headers_for_task = clean_headers;
            let policy_for_task = state.get_upstream_policy();
            let origin_metrics_for_task = state.telemetry.as_ref().map(|t| t.origin_metrics.clone());
            let exec_for_task = state.intel_executor();
            let router_for_task = state.router.clone();
            // Records where the leader response came from (peer fill or origin) for X-Cache-Source.
            let source_cell = Arc::new(std::sync::atomic::AtomicU8::new(SOURCE_ORIGIN));
            let source_for_task = source_cell.clone();

            tokio::spawn(async move {
                let canonical_key = key_for_task.to_canonical_string();
                let mut peer_entry: Option<(crate::dist::frame::PeerEntry, u64)> = None;

                if let Some(ref router) = router_for_task {
                    let local_node_id = router.get_node_id();
                    let primary = router.route_key(&canonical_key);
                    if let Some(ref p) = primary {
                        if p.node_id != local_node_id {
                            // SETBACK-14 / Phase 2.2: Remote primary exists. Attempt a genuine peer read
                            // over cluster mTLS/TCP before falling back to origin. The peer returns the
                            // full entry (status, Content-Type, ETag, validators, age, remaining lifetime).
                            // Primary only (no replica fallback, PC-AUTO-050): a replica can lag behind an invalidation
                            // that the primary and this gateway already applied, and would hand back a deleted entry.
                            match router.read_entry_from_peer(&canonical_key, 2000, 0).await {
                                Ok(crate::dist::router::PeerReadResult::Hit { entry, elapsed_ms, .. }) => {
                                    peer_entry = Some((entry, elapsed_ms));
                                }
                                Ok(crate::dist::router::PeerReadResult::Miss { .. }) => {
                                    outcome_counters().peer_read_miss.fetch_add(1, Ordering::Relaxed);
                                }
                                // Format/version refusal: the peer is alive but speaks another layout.
                                Err(crate::dist::router::DistError::ProtocolError) => {
                                    outcome_counters().peer_read_refused.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(_) => {
                                    outcome_counters().peer_read_failed.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }

                if let Some((entry, elapsed_ms)) = peer_entry {
                    // PC-AUTO-051 / D-036: a peer-supplied lifetime is bounded. An absurd age or remaining
                    // lifetime (more than a year) means a broken or hostile peer: ignore the entry and ask
                    // the origin. A merely long lifetime is clamped to 24 h.
                    let absurd = entry.remaining_ms > PEER_ABSURD_MS || entry.age_ms > PEER_ABSURD_MS;
                    if absurd {
                        outcome_counters().peer_read_rejected.fetch_add(1, Ordering::Relaxed);
                    }
                    let entry = if absurd { None } else { Some(entry) };
                    if let Some(mut entry) = entry {
                    entry.remaining_ms = entry.remaining_ms.min(PEER_MAX_LIFETIME_MS);
                    // Genuine peer hit: populate local cache (peer-fill) and complete single-flight
                    // without dispatching any origin request or consuming origin concurrency.
                    // Fidelity (PC-AUTO-041/042): status, Content-Type, ETag and validators are stored as
                    // the source had them, and the copy keeps the source's age and remaining lifetime
                    // (minus the transfer time), so it never outlives the entry it was copied from and
                    // `Age` stays truthful.
                    let remaining_ms = entry.remaining_ms.saturating_sub(elapsed_ms);
                    if remaining_ms > 0 {
                        let now_peer_fill = DiagnosticBuffer::now_ms();
                        let age_ms = entry.age_ms.saturating_add(elapsed_ms);
                        let resource = Arc::new(CachedResource {
                            status: entry.status,
                            content_type: entry.content_type.clone(),
                            body: Arc::new(entry.body.clone()),
                            etag: entry.etag.clone(),
                            created_at_ms: now_peer_fill.saturating_sub(age_ms),
                            ttl_ms: age_ms.saturating_add(remaining_ms),
                        });

                        guard.cache.update_freshness_for_guard(
                            &guard,
                            crate::core::freshness::FreshnessMetadata {
                                must_revalidate: entry.must_revalidate,
                                stale_while_revalidate_ms: entry.stale_while_revalidate_ms,
                                stale_if_error_ms: entry.stale_if_error_ms,
                                last_modified: entry.last_modified.clone(),
                                date_ms: entry.date_ms,
                                initial_age_ms: entry.initial_age_ms,
                            },
                        );

                        let fp = guard.cache.diagnostics.hasher.fingerprint(&canonical_key);
                        guard.cache.diagnostics.push(DiagnosticEvent {
                            timestamp_epoch_ms: now_peer_fill,
                            severity: DiagnosticLevel::INFO,
                            event_name: "PeerFillHit",
                            key_fp: fp,
                            op_id: Some(op_id),
                            op_gen: Some(op_gen),
                            outcome: "Success",
                            duration_ms: Some(0),
                            error_msg: None,
                        });

                        source_for_task.store(SOURCE_PEER, Ordering::Relaxed);
                        if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(resource)) {
                            batch.deliver();
                        }
                        return;
                    }
                    // The source entry was about to expire: do not store a dead copy, ask the origin.
                    }
                }

                // R-01: only this single-flight leader takes a permit; coalesced waiters
                // await `my_rx`/shared_op and never reach this point.
                let permit_res = acquire_origin_permit(exec_for_task.as_ref());
                // A cap rejection is not an origin dispatch: do not count it in OriginMetrics.
                let in_flight_guard = if permit_res.is_ok() {
                    origin_metrics_for_task.as_ref().map(|m| m.start_request())
                } else {
                    None
                };
                let start_time = DiagnosticBuffer::now_ms();
                // Revalidation of a stale resident entry: send ITS validators (read now, after
                // single-flight registration). A miss (nothing resident) is unconditional.
                let validators = stored_validators(&guard.cache, &key_for_task);
                let fetch_res = match permit_res {
                    Ok(permit) => {
                        if validators.is_some() {
                            guard.cache.counters.revalidation_conditional_sent.fetch_add(1, Ordering::Relaxed);
                        }
                        fetch_origin_permitted(permit, &origin_base, &key_for_task, &headers_for_task, validators.as_ref(), policy_for_task.as_deref()).await
                    }
                    Err(e) => Err(e),
                };
                let duration_ms = DiagnosticBuffer::now_ms().saturating_sub(start_time);

                match fetch_res {
                    Ok((status, headers_out, body)) => {
                        let now_fetch = DiagnosticBuffer::now_ms();
                        let has_explicit_framing = headers_out.iter().any(|(k, _)| {
                            if let Some(name) = k {
                                let n = name.as_str();
                                return n.eq_ignore_ascii_case("content-length")
                                    || n.eq_ignore_ascii_case("transfer-encoding");
                            }
                            false
                        });

                        let has_auth = headers_for_task.contains_key("authorization");
                        let decision = crate::core::freshness::evaluate_response_freshness(
                            status,
                            &headers_out,
                            now_fetch,
                            has_explicit_framing,
                            body.len(),
                            60_000,
                            has_auth,
                        );

                        if status == 200 || (decision.is_cacheable && (status == 301 || status == 404 || status == 410)) {
                            if let Some(g) = in_flight_guard {
                                g.complete_ok();
                            }

                            let content_type = headers_out
                                .iter()
                                .find_map(|(k, v)| {
                                    if let Some(name) = k {
                                        if name.as_str().eq_ignore_ascii_case("content-type") {
                                            return v.to_str().ok().map(|s| s.to_string());
                                        }
                                    }
                                    None
                                })
                                .unwrap_or_else(|| "application/json; charset=utf-8".to_string());

                            let resource = Arc::new(CachedResource {
                                status,
                                content_type,
                                body: Arc::new(body),
                                etag: decision.etag,
                                created_at_ms: now_fetch,
                                ttl_ms: decision.ttl_ms,
                            });

                            if decision.is_cacheable {
                                guard.cache.update_freshness_for_guard(
                                    &guard,
                                    crate::core::freshness::FreshnessMetadata {
                                        must_revalidate: decision.must_revalidate,
                                        stale_while_revalidate_ms: decision.stale_while_revalidate_ms,
                                        stale_if_error_ms: decision.stale_if_error_ms,
                                        last_modified: decision.last_modified,
                                        date_ms: decision.date_ms,
                                        initial_age_ms: decision.initial_age_ms,
                                    },
                                );
                            }

                            let outcome = if decision.is_cacheable {
                                FinalizeOutcome::Success(resource)
                            } else {
                                FinalizeOutcome::Uncacheable(resource)
                            };

                            let fp = guard
                                .cache
                                .diagnostics
                                .hasher
                                .fingerprint(&key_for_task.to_canonical_string());
                            guard.cache.diagnostics.push(DiagnosticEvent {
                                timestamp_epoch_ms: DiagnosticBuffer::now_ms(),
                                severity: DiagnosticLevel::INFO,
                                event_name: "OriginFetchSuccess",
                                key_fp: fp,
                                op_id: Some(op_id),
                                op_gen: Some(op_gen),
                                outcome: "Success",
                                duration_ms: Some(duration_ms),
                                error_msg: None,
                            });

                            if let Some(mut batch) = guard.complete(outcome) {
                                batch.deliver();
                            }
                        } else if status == 304 {
                            if validators.is_some() {
                                guard.cache.counters.revalidation_304_received.fetch_add(1, Ordering::Relaxed);
                            }
                            // S-1/M2/H1/M-a/L-4: see PartitionedCache::complete_not_modified.
                            let now_reval = DiagnosticBuffer::now_ms();
                            let has_auth = headers_for_task.contains_key("authorization");
                            let cache_for_304 = guard.cache.clone();
                            let outcome_304 = cache_for_304.complete_not_modified(guard, &headers_out, now_reval, has_auth);
                            if let Some(g) = in_flight_guard {
                                if outcome_304 == crate::core::cache::NotModifiedOutcome::Refreshed {
                                    g.complete_ok();
                                } else {
                                    g.complete_err(&OriginError::UpstreamStatus(status));
                                }
                            }
                        } else {
                            // Check Stale-If-Error before reporting failure
                            let hard_cap_ms = guard.cache.get_max_stale_ms();
                            let sie_check = guard.cache.check_stale_serving(&key_for_task, DiagnosticBuffer::now_ms(), hard_cap_ms, true);
                            if let crate::core::cache::StaleServingDecision::Eligible(stale_res, _) = sie_check {
                                let now_sie = DiagnosticBuffer::now_ms();
                                let age_ms = now_sie.saturating_sub(stale_res.created_at_ms);
                                guard.cache.record_stale_served(age_ms, stale_res.ttl_ms, hard_cap_ms);
                                guard.cache.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                                guard.cache.record_revalidation_failure(&key_for_task, now_sie);
                                if let Some(g) = in_flight_guard {
                                    g.complete_err(&OriginError::UpstreamStatus(status));
                                }
                                if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(stale_res))) {
                                    batch.deliver();
                                }
                            } else {
                                if let Some(g) = in_flight_guard {
                                    g.complete_err(&OriginError::UpstreamStatus(status));
                                }
                                guard
                                    .cache
                                    .counters
                                    .origin_fetches_failed
                                    .fetch_add(1, Ordering::Relaxed);
                                let fp = guard
                                    .cache
                                    .diagnostics
                                    .hasher
                                    .fingerprint(&key_for_task.to_canonical_string());
                                guard.cache.diagnostics.push(DiagnosticEvent {
                                    timestamp_epoch_ms: DiagnosticBuffer::now_ms(),
                                    severity: DiagnosticLevel::WARN,
                                    event_name: "OriginFetchNon200",
                                    key_fp: fp,
                                    op_id: Some(op_id),
                                    op_gen: Some(op_gen),
                                    outcome: "Error",
                                    duration_ms: Some(duration_ms),
                                    error_msg: Some(format!("HTTP {}", status)),
                                });

                                if let Some(mut batch) = guard.complete(FinalizeOutcome::Error(
                                    Arc::new(OriginError::UpstreamStatus(status)),
                                )) {
                                    batch.deliver();
                                }
                            }
                        }
                    }
                    Err(err) => {
                        // Check Stale-If-Error on network / connection error
                        let hard_cap_ms = guard.cache.get_max_stale_ms();
                        let sie_check = guard.cache.check_stale_serving(&key_for_task, DiagnosticBuffer::now_ms(), hard_cap_ms, true);
                        if let crate::core::cache::StaleServingDecision::Eligible(stale_res, _) = sie_check {
                            let now_sie = DiagnosticBuffer::now_ms();
                            let age_ms = now_sie.saturating_sub(stale_res.created_at_ms);
                            guard.cache.record_stale_served(age_ms, stale_res.ttl_ms, hard_cap_ms);
                            guard.cache.counters.revalidation_error.fetch_add(1, Ordering::Relaxed);
                            guard.cache.record_revalidation_failure(&key_for_task, now_sie);
                            if let Some(g) = in_flight_guard {
                                g.complete_err(&err);
                            }
                            if let Some(mut batch) = guard.complete(FinalizeOutcome::Success(Arc::new(stale_res))) {
                                batch.deliver();
                            }
                        } else {
                            if let Some(g) = in_flight_guard {
                                g.complete_err(&err);
                            }
                            guard
                                .cache
                                .counters
                                .origin_fetches_failed
                                .fetch_add(1, Ordering::Relaxed);
                            let fp = guard
                                .cache
                                .diagnostics
                                .hasher
                                .fingerprint(&key_for_task.to_canonical_string());
                            guard.cache.diagnostics.push(DiagnosticEvent {
                                timestamp_epoch_ms: DiagnosticBuffer::now_ms(),
                                severity: DiagnosticLevel::ERROR,
                                event_name: "OriginFetchFailed",
                                key_fp: fp,
                                op_id: Some(op_id),
                                op_gen: Some(op_gen),
                                outcome: "Error",
                                duration_ms: Some(duration_ms),
                                error_msg: Some(format!("{:?}", err)),
                            });

                            if let Some(mut batch) =
                                guard.complete(FinalizeOutcome::Error(Arc::new(err)))
                            {
                                batch.deliver();
                            }
                        }
                    }
                }
            });

            match my_rx.await {
                Ok(Ok(resource)) => {
                    let status_code =
                        StatusCode::from_u16(resource.status).unwrap_or(StatusCode::OK);
                    let mut resp = (status_code, (*resource.body).clone()).into_response();
                    resp.headers_mut().insert(
                        "Content-Type",
                        HeaderValue::from_str(&resource.content_type)
                            .unwrap_or(HeaderValue::from_static("application/json; charset=utf-8")),
                    );
                    let is_stale = resource.created_at_ms < now_ms && resource.created_at_ms.saturating_add(resource.ttl_ms) <= now_ms;
                    let age_secs = crate::core::freshness::calculate_current_age_seconds(
                        resource.created_at_ms,
                        0,
                        DiagnosticBuffer::now_ms(),
                    );
                    let status_header = if is_stale { "STALE" } else { "MISS" };
                    resp.headers_mut()
                        .insert("X-Cache-Status", HeaderValue::from_static(status_header));
                    let source = if source_cell.load(Ordering::Relaxed) == SOURCE_PEER {
                        "peer"
                    } else if is_stale {
                        "stale"
                    } else {
                        "origin"
                    };
                    mark_source(&mut resp, source);
                    resp.headers_mut().insert("Age", HeaderValue::from(age_secs));
                    if let Some(ref etag) = resource.etag {
                        if let Ok(val) = HeaderValue::from_str(etag) {
                            resp.headers_mut().insert("ETag", val);
                        }
                    }
                    (resp, false, 0, false)
                }
                Ok(Err(err)) => (map_origin_error_to_response(&err), false, 0, true),
                Err(_) => (
                    (
                        StatusCode::BAD_GATEWAY,
                        [("X-Cache-Error", "InitiatorCancelled")],
                        "Origin fetch aborted",
                    )
                        .into_response(),
                    false,
                    0,
                    true,
                ),
            }
        }
        FetchRegistrationResult::Rejected(err) => (map_origin_error_to_response(&err), false, 0, true),
    };

    // PIE v2: one non-blocking event (hash only, no key material); never affects the response.
    if !is_err {
        if let Some(outcome) = pie_outcome_of(&resp) {
            let origin = if outcome == crate::intelligence_v2::signals::Outcome::Miss {
                crate::intelligence_v2::signals::OriginResult::Ok
            } else {
                crate::intelligence_v2::signals::OriginResult::None
            };
            pie_record(key_hash, start_instant.elapsed().as_micros() as u64, outcome, origin);
        }
    } else {
        pie_record(key_hash, start_instant.elapsed().as_micros() as u64, crate::intelligence_v2::signals::Outcome::Miss, crate::intelligence_v2::signals::OriginResult::Err);
    }

    if let Some(ref tel) = state.telemetry {
        let elapsed_us = start_instant.elapsed().as_micros() as u64;
        if is_err {
            tel.record_error();
        }
        tel.record_us(crate::intel::telemetry::RawObservationUs::new_get(
            now_ms,
            key_hash,
            elapsed_us.max(1),
            hit,
            coalesced,
        ));
    }

    resp
}

async fn delete_cache_handler(
    State(state): State<Arc<ApiState>>,
    Path(key_path): Path<String>,
    raw_query: RawQuery,
    headers: HeaderMap,
    _body: axum::body::Bytes,
) -> Response {
    let start_instant = std::time::Instant::now();
    let cache = match state.cache.as_ref() {
        Some(c) => c,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cache uninitialized").into_response();
        }
    };

    let mut authenticated_tenant: Option<String> = None;

    if let Some(ref sec) = state.security {
        let tenant = match authenticate_request(&headers, sec, crate::security::Role::CacheInvalidate)
            .and_then(require_tenant_claim)
        {
            Ok(t) => t,
            Err(resp) => return resp,
        };
        authenticated_tenant = Some(tenant);
    }

    let tenant_id = if let Some(ref t) = authenticated_tenant {
        Some(t.as_str())
    } else if state.security.is_some() {
        // Secure mode: caller-supplied x-tenant-id is NEVER trusted
        None
    } else {
        match header_tenant(&headers) {
            Ok(t) => t,
            Err(e) => return map_origin_error_to_response(&e),
        }
    };

    let norm_path = if key_path.starts_with('/') {
        key_path
    } else {
        format!("/{}", key_path)
    };

    let cache_key = CacheKey::new(tenant_id, &norm_path, raw_query.0.as_deref());

    if let Err(e) = cache_key.validate() {
        return map_origin_error_to_response(&e);
    }

    // Invalidate local cache FIRST before cluster round-trip (C-08 / Guarantee G1 / ADR-003)
    // Guarantees linearizability: concurrent GETs on this gateway immediately miss / observe deletion.
    cache.delete(&cache_key);

    let (status_code, headers_resp) = if let Some(ref router) = state.router {
        let canonical_key = cache_key.to_canonical_string();
        match router
            .invalidate_key_across_cluster(
                &canonical_key,
                crate::dist::frame::InvalidationScope::ExactKey,
            )
            .await
        {
            Ok(outcome) => {
                if outcome.status.is_success() {
                    let mut resp_headers = HeaderMap::new();
                    if outcome.is_acting_primary {
                        let warning_val = match outcome.suspect_node_id {
                            Some(id) => format!("PrimaryFailover({})", id),
                            None => "PrimaryFailover".to_string(),
                        };
                        if let Ok(hv) = HeaderValue::from_str(&warning_val) {
                            resp_headers.insert(HeaderName::from_static("x-cache-warning"), hv);
                        }
                    }
                    (StatusCode::NO_CONTENT, resp_headers)
                } else {
                    let mut resp_headers = HeaderMap::new();
                    let (code, err_header) = match outcome.status {
                        crate::dist::frame::InvalidationStatus::RejectedRateLimit => (
                            StatusCode::SERVICE_UNAVAILABLE,
                            "PrimaryRateLimited",
                        ),
                        crate::dist::frame::InvalidationStatus::InternalError => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "PrimaryInternalError",
                        ),
                        crate::dist::frame::InvalidationStatus::IgnoredObsolete => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "ObsoleteInvalidation",
                        ),
                        _ => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "PrimaryInvalidationFailed",
                        ),
                    };
                    resp_headers.insert(
                        HeaderName::from_static("x-cache-error"),
                        HeaderValue::from_static(err_header),
                    );
                    (code, resp_headers)
                }
            }
            Err(err) => {
                // Local hygiene deletion to avoid retaining known stale entries if timeout occurred
                cache.delete(&cache_key);
                let mut resp_headers = HeaderMap::new();
                let (code, err_header) = match err {
                    crate::dist::router::DistError::Rejected(crate::dist::frame::InvalidationStatus::RejectedRateLimit) => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "PrimaryRateLimited",
                    ),
                    crate::dist::router::DistError::Rejected(crate::dist::frame::InvalidationStatus::InternalError) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PrimaryInternalError",
                    ),
                    crate::dist::router::DistError::Rejected(crate::dist::frame::InvalidationStatus::IgnoredObsolete) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "ObsoleteInvalidation",
                    ),
                    crate::dist::router::DistError::Rejected(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PrimaryInvalidationRejected",
                    ),
                    crate::dist::router::DistError::Timeout
                    | crate::dist::router::DistError::NodeUnreachable(_)
                    | crate::dist::router::DistError::NoActiveNodes => (
                        StatusCode::GATEWAY_TIMEOUT,
                        "PrimaryInvalidationTimeout",
                    ),
                    crate::dist::router::DistError::ProtocolError => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PrimaryProtocolError",
                    ),
                    // Local connection pool saturated: retryable back-pressure, not a peer failure.
                    // The invalidation is still delivered in the background by the broadcaster.
                    crate::dist::router::DistError::Busy(_) => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "PrimaryInvalidationBusy",
                    ),
                };
                resp_headers.insert(
                    HeaderName::from_static("x-cache-error"),
                    HeaderValue::from_static(err_header),
                );
                (code, resp_headers)
            }
        }
    } else {
        cache.delete(&cache_key);
        (StatusCode::NO_CONTENT, HeaderMap::new())
    };

    // PC-AUTO-050: purge AGAIN after the cluster leg. A GET that started after the first purge but before
    // the primary applied the invalidation may have peer-filled the entry the cluster is deleting. This
    // second purge removes such a copy (and bumps the generation, so a peer read still in flight cannot
    // store its result), which restores read-your-delete on this gateway once DELETE has returned.
    cache.delete(&cache_key);

    let fp = cache
        .diagnostics
        .hasher
        .fingerprint(&cache_key.to_canonical_string());
    cache.diagnostics.push(DiagnosticEvent {
        timestamp_epoch_ms: DiagnosticBuffer::now_ms(),
        severity: if status_code.is_server_error() {
            DiagnosticLevel::ERROR
        } else {
            DiagnosticLevel::INFO
        },
        event_name: "CacheInvalidated",
        key_fp: fp,
        op_id: None,
        op_gen: None,
        outcome: if status_code == StatusCode::NO_CONTENT {
            "Invalidated"
        } else if status_code == StatusCode::GATEWAY_TIMEOUT {
            "PrimaryTimeout"
        } else {
            "InvalidationRejected"
        },
        duration_ms: Some(0),
        error_msg: if status_code != StatusCode::NO_CONTENT {
            Some(format!("Invalidation failed with status {}", status_code))
        } else {
            None
        },
    });

    pie_record(
        crate::intel::telemetry::fnv1a_64(&cache_key.to_canonical_string()),
        start_instant.elapsed().as_micros() as u64,
        crate::intelligence_v2::signals::Outcome::Invalidation,
        crate::intelligence_v2::signals::OriginResult::None,
    );

    if let Some(ref tel) = state.telemetry {
        let key_hash = crate::intel::telemetry::fnv1a_64(&cache_key.to_canonical_string());
        let elapsed_us = start_instant.elapsed().as_micros() as u64;
        tel.record_us(crate::intel::telemetry::RawObservationUs::new_delete(
            DiagnosticBuffer::now_ms(),
            key_hash,
            elapsed_us.max(1),
        ));
    }

    let mut resp = if status_code == StatusCode::GATEWAY_TIMEOUT {
        (status_code, "Primary invalidation timeout").into_response()
    } else {
        status_code.into_response()
    };
    *resp.headers_mut() = headers_resp;
    resp
}

async fn diagnostics_summary_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            if let Err(resp) = authenticate_request(&headers, sec, crate::security::Role::Observer) {
                return resp;
            }
        }
    }
    let cache = match state.cache.as_ref() {
        Some(c) => c,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cache uninitialized").into_response();
        }
    };
    Json(cache.counters.snapshot()).into_response()
}

async fn diagnostics_events_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    if let Some(ref sec) = state.security {
        if sec.require_auth_for_control {
            if let Err(resp) = authenticate_request(&headers, sec, crate::security::Role::Observer) {
                return resp;
            }
        }
    }
    let cache = match state.cache.as_ref() {
        Some(c) => c,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cache uninitialized").into_response();
        }
    };
    Json(cache.diagnostics.get_recent_events(100)).into_response()
}

fn map_origin_error_to_response(err: &OriginError) -> Response {
    match err {
        OriginError::UpstreamStatus(404) => (
            StatusCode::NOT_FOUND,
            [("X-Cache-Error", "NotFound")],
            "Origin Not Found",
        )
            .into_response(),
        OriginError::UpstreamStatus(code) => (
            StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY),
            [("X-Cache-Error", "UpstreamStatus")],
            "Upstream Origin Error",
        )
            .into_response(),
        OriginError::Timeout => (
            StatusCode::GATEWAY_TIMEOUT,
            [("X-Cache-Error", "Timeout")],
            "Origin Timeout",
        )
            .into_response(),
        OriginError::Aborted => (
            StatusCode::BAD_GATEWAY,
            [("X-Cache-Error", "Aborted")],
            "Origin Fetch Aborted",
        )
            .into_response(),
        OriginError::ResponseTooLarge => (
            StatusCode::BAD_GATEWAY,
            [("X-Cache-Error", "ResponseTooLarge")],
            "Upstream Response Too Large",
        )
            .into_response(),
        OriginError::WaiterLimitExceeded => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                ("X-Cache-Error", "WaiterLimitExceeded"),
                ("Retry-After", "1"),
            ],
            "Waiter queue capacity saturated for this key",
        )
            .into_response(),
        // D2 (G1 verdict): an origin-concurrency-cap rejection is carried as this marker inside
        // `NetworkError` (OriginError is in the frozen core). It must never claim shard saturation.
        OriginError::NetworkError(m) if m == ORIGIN_CAP_SATURATED_MARKER => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                // compatibility: existing clients and the Scenario B shed classifier key on this value
                ("X-Cache-Error", "CapacityExceeded"),
                ("X-Cache-Error-Reason", "OriginCapSaturated"),
                ("Retry-After", "1"),
            ],
            "Origin concurrency cap saturated; the request was shed before reaching the origin",
        )
            .into_response(),
        OriginError::CapacityExceeded => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                ("X-Cache-Error", "CapacityExceeded"),
                ("Retry-After", "1"),
            ],
            "Cache shard capacity saturated; all entries are active or in-flight",
        )
            .into_response(),
        OriginError::TenantIdTooLong => (
            StatusCode::BAD_REQUEST,
            [("X-Cache-Error", "TenantIdTooLong")],
            "Tenant ID exceeds maximum allowed length of 128 bytes",
        )
            .into_response(),
        OriginError::InvalidTenantId => (
            StatusCode::BAD_REQUEST,
            [("X-Cache-Error", "InvalidTenantId")],
            "Tenant ID must be 1-128 characters of [A-Za-z0-9._-]",
        )
            .into_response(),
        OriginError::InvalidPath => (
            StatusCode::BAD_REQUEST,
            [("X-Cache-Error", "InvalidPath")],
            "Request path or query contains a forbidden character (control character, space) or the sequence \":?\" in the path",
        )
            .into_response(),
        OriginError::UriTooLong => (
            StatusCode::URI_TOO_LONG,
            [("X-Cache-Error", "UriTooLong")],
            "URI path or query string exceeds maximum allowed length",
        )
            .into_response(),
        OriginError::NetworkError(_) => (
            StatusCode::BAD_GATEWAY,
            [("X-Cache-Error", "NetworkError")],
            "Upstream Network Error",
        )
            .into_response(),
        OriginError::SecurityError(sec_err) => {
            let (status, reason, safe_msg) = match sec_err {
                crate::security::upstream::OriginSecurityError::InvalidOriginUrl(_)
                | crate::security::upstream::OriginSecurityError::DisallowedScheme(_)
                | crate::security::upstream::OriginSecurityError::DisallowedPort(_)
                | crate::security::upstream::OriginSecurityError::SensitivePortBlocked(_) => {
                    (StatusCode::BAD_REQUEST, "InvalidOrigin", "Invalid or prohibited origin destination")
                }
                crate::security::upstream::OriginSecurityError::OriginNotAllowlisted(_)
                | crate::security::upstream::OriginSecurityError::InsecureUpstreamDisallowed
                | crate::security::upstream::OriginSecurityError::SsrfBlocked(_) => {
                    (StatusCode::BAD_GATEWAY, "SsrfBlocked", "Destination blocked by security policy")
                }
                crate::security::upstream::OriginSecurityError::TlsHandshakeFailed(_)
                | crate::security::upstream::OriginSecurityError::TlsCertificateExpired
                | crate::security::upstream::OriginSecurityError::TlsHostnameMismatch(_)
                | crate::security::upstream::OriginSecurityError::TlsUntrustedRoot => {
                    (StatusCode::BAD_GATEWAY, "UpstreamTlsFailed", "Upstream TLS verification failed")
                }
                crate::security::upstream::OriginSecurityError::Timeout => {
                    (StatusCode::GATEWAY_TIMEOUT, "Timeout", "Upstream Origin Timeout")
                }
                crate::security::upstream::OriginSecurityError::MalformedFraming(_) => {
                    (StatusCode::BAD_GATEWAY, "MalformedFraming", "Malformed HTTP response framing from upstream origin")
                }
                crate::security::upstream::OriginSecurityError::TruncatedResponse(_) => {
                    (StatusCode::BAD_GATEWAY, "TruncatedResponse", "Truncated response body from upstream origin")
                }
                crate::security::upstream::OriginSecurityError::OversizedResponse(_) => {
                    (StatusCode::BAD_GATEWAY, "ResponseTooLarge", "Upstream response exceeds size limit")
                }
                crate::security::upstream::OriginSecurityError::NetworkError(_) => {
                    (StatusCode::BAD_GATEWAY, "NetworkError", "Upstream Network Error")
                }
            };
            (
                status,
                [("X-Cache-Error", reason)],
                safe_msg,
            )
                .into_response()
        }
    }
}

// ==========================================
// Hardened Upstream Transport & HTTP/1.1 Origin Fetcher
// ==========================================

const MAX_HEADER_BYTES: usize = 64 * 1024; // 64 KB header limit
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MB body limit
const MAX_TOTAL_RESPONSE_BYTES: usize = MAX_HEADER_BYTES + MAX_BODY_BYTES + 4096;

pub type RawHttpResponse = (u16, Vec<(Option<HeaderName>, HeaderValue)>, Vec<u8>);

enum UpstreamTransport {
    Plain(tokio::net::TcpStream),
    Tls(Box<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>),
}

impl tokio::io::AsyncRead for UpstreamTransport {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            Self::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for UpstreamTransport {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            Self::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            Self::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            Self::Tls(s) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

/// R-01: take an origin-concurrency permit if the intelligence executor is installed.
/// `Ok(None)` = no executor (uncapped, behavior unchanged). `Err(marker)` = saturated.
/// Must be called only by the party that really dispatches to the origin (single-flight leader,
/// bypass request, background revalidation leader) - never by coalesced waiters.
fn acquire_origin_permit(
    exec: Option<&Arc<crate::intelligence::executor::DefaultPolicyExecutor>>,
) -> Result<OriginAdmission, OriginError> {
    let saturated = || OriginError::NetworkError(ORIGIN_CAP_SATURATED_MARKER.to_string());
    // Static cap first (every mode). If the executor then refuses, the static permit is released on drop.
    let static_permit = match STATIC_ORIGIN_CAP.try_acquire() {
        Ok(p) => p,
        Err(()) => return Err(saturated()),
    };
    let exec_permit = match exec {
        None => None,
        Some(e) => Some(e.acquire_origin_permit().map_err(|_| saturated())?),
    };
    Ok(OriginAdmission { exec: exec_permit, _static: static_permit })
}

/// What a dispatching party holds while it talks to the origin: the optional adaptive-executor permit
/// (enforced only in canary/active, frozen code) and the optional STATIC cap permit (every mode).
pub struct OriginAdmission {
    exec: Option<crate::intelligence::executor::OriginPermit>,
    _static: Option<StaticCapPermit>,
}

/// Plain static cap on concurrent origin fetches (`PHANTOM_ORIGIN_CAP_MAX`), enforced in EVERY intelligence
/// mode. It is not adaptive, not a policy and not part of the PIE: a counter compared with a fixed limit.
/// `limit == 0` means no static cap. Process-global (one gateway = one process) because the gateway state
/// struct is built field by field in many tests.
pub struct StaticOriginCap {
    limit: std::sync::atomic::AtomicUsize,
    in_flight: std::sync::atomic::AtomicUsize,
    rejected: std::sync::atomic::AtomicU64,
}

pub struct StaticCapPermit {
    cap: &'static StaticOriginCap,
}

impl Drop for StaticCapPermit {
    fn drop(&mut self) {
        self.cap.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

impl StaticOriginCap {
    const fn new() -> Self {
        Self {
            limit: std::sync::atomic::AtomicUsize::new(0),
            in_flight: std::sync::atomic::AtomicUsize::new(0),
            rejected: std::sync::atomic::AtomicU64::new(0),
        }
    }
    /// `Ok(None)` when no cap is configured, `Ok(Some(permit))` when admitted, `Err(())` when saturated.
    fn try_acquire(&'static self) -> Result<Option<StaticCapPermit>, ()> {
        let limit = self.limit.load(Ordering::Acquire);
        if limit == 0 {
            return Ok(None);
        }
        let mut cur = self.in_flight.load(Ordering::Acquire);
        loop {
            if cur >= limit {
                self.rejected.fetch_add(1, Ordering::Relaxed);
                return Err(());
            }
            match self.in_flight.compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(Some(StaticCapPermit { cap: self })),
                Err(actual) => cur = actual,
            }
        }
    }
    pub fn limit(&self) -> usize { self.limit.load(Ordering::Acquire) }
    pub fn in_flight(&self) -> usize { self.in_flight.load(Ordering::Acquire) }
    pub fn rejected(&self) -> u64 { self.rejected.load(Ordering::Relaxed) }
}

static STATIC_ORIGIN_CAP: StaticOriginCap = StaticOriginCap::new();

/// Sets the static origin cap (`0` removes it). Called once by `main` from `PHANTOM_ORIGIN_CAP_MAX`.
pub fn set_static_origin_cap(limit: usize) {
    STATIC_ORIGIN_CAP.limit.store(limit, Ordering::Release);
}

pub fn static_origin_cap() -> &'static StaticOriginCap {
    &STATIC_ORIGIN_CAP
}

/// `metrics.origin_static_cap`: configured limit (0 = no static cap), fetches in flight now, rejections since start.
fn static_origin_cap_json() -> serde_json::Value {
    let c = &STATIC_ORIGIN_CAP;
    json!({
        "limit": {"src": "measured", "v": c.limit(), "unit": "concurrent origin fetches"},
        "in_flight": {"src": "measured", "v": c.in_flight(), "unit": "count"},
        "rejections": {"src": "measured", "v": c.rejected(), "unit": "count"}
    })
}

/// Sentinel text of an origin-cap rejection (see `map_origin_error_to_response`). No real network
/// error produces exactly this string.
const ORIGIN_CAP_SATURATED_MARKER: &str = "origin-cap-saturated";

/// Dispatch to the origin while holding `permit` (released on every exit, including
/// cancellation of this future). Records real latency/failure into the AIMD controller.
async fn fetch_origin_permitted(
    permit: OriginAdmission,
    origin_base: &str,
    cache_key: &CacheKey,
    headers: &HeaderMap,
    validators: Option<&StoredValidators>,
    policy: Option<&crate::security::upstream::UpstreamPolicy>,
) -> Result<RawHttpResponse, OriginError> {
    let started = std::time::Instant::now();
    let res = fetch_origin_raw(origin_base, cache_key, headers, validators, policy).await;
    if let Some(ref p) = permit.exec {
        let failed = match &res {
            Ok((status, _, _)) => *status >= 500,
            Err(_) => true,
        };
        p.observe(started.elapsed().as_millis() as u64, failed);
    }
    drop(permit);
    res
}

async fn fetch_origin_gated(
    exec: Option<&Arc<crate::intelligence::executor::DefaultPolicyExecutor>>,
    origin_base: &str,
    cache_key: &CacheKey,
    headers: &HeaderMap,
    policy: Option<&crate::security::upstream::UpstreamPolicy>,
) -> Result<RawHttpResponse, OriginError> {
    let permit = acquire_origin_permit(exec)?;
    // Bypass fetches are plain unconditional requests: nothing is being revalidated.
    fetch_origin_permitted(permit, origin_base, cache_key, headers, None, policy).await
}

/// Validators of the STORED (cached) representation, used to revalidate it upstream
/// (RFC 9111 4.3.1). They come from the resident entry only, never from the client's request.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredValidators {
    etag: Option<String>,
    last_modified: Option<String>,
}

fn is_header_safe(v: &str) -> bool {
    !v.is_empty() && !v.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0)
}

/// Stored validators of the entry resident NOW (read after single-flight registration, never from
/// a copy captured earlier: H1). `None` = nothing resident or no usable validator, in which case
/// the fetch is unconditional. An ETag is forwarded verbatim, weak prefix included (If-None-Match
/// uses weak comparison, RFC 9110 13.1.2).
fn stored_validators(cache: &PartitionedCache, key: &CacheKey) -> Option<StoredValidators> {
    let (entry, meta) = cache.peek_resident(key)?;
    let etag = entry.etag.filter(|e| is_header_safe(e));
    let last_modified = meta.and_then(|m| m.last_modified).filter(|l| is_header_safe(l));
    if etag.is_none() && last_modified.is_none() {
        None
    } else {
        Some(StoredValidators { etag, last_modified })
    }
}

/// Request headers that make a request conditional or ranged (RFC 9110 13.1, 14.2). The cached
/// representation is shared by every client, so a CLIENT's value for any of these must never
/// reach the origin: a 304 (or a 206) would describe the client's copy, not the cache's, and be
/// applied to the wrong body.
fn is_client_conditional_or_range_header(name: &str) -> bool {
    ["if-none-match", "if-modified-since", "if-match", "if-unmodified-since", "if-range", "range"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
}

// Request headers forwarded upstream (see the M-1 contract in fetch_origin_raw): authorization only if the upstream policy
// allows, a validated x-request-id, the key tenant as x-tenant-id. Client conditional/range headers are NEVER forwarded (explicitly skipped
// below, in addition to not being on the allowlist); the only conditional headers sent are the
// STORED entry's validators passed in `validators`. Guarded by the strip/forward tests in
// tests/integration/revalidation_304_tests.rs.
/// H1: build the origin-form request target from the (percent-decoded) cache key. Everything
/// outside the RFC 3986 `pchar` set (and `/`, plus `?` inside the query) is percent-encoded
/// byte-wise, including control characters, space and non-ASCII bytes, so the result is pure
/// ASCII with no whitespace and can never terminate the request line or inject a header, even if
/// `CacheKey::validate` were bypassed. A `%` that already starts a valid `%XX` escape is kept as is
/// (in the QUERY only). In the decoded PATH a literal `%` is always written as `%25`, so a client `%252F`
/// reaches the origin as `%252F` (faithful), never as `%2F`. A bare `%` in the query becomes
/// `%25`. A `?` inside the path is encoded (`%3F`) so it cannot start a query.
pub fn encode_origin_target(path: &str, query: Option<&str>) -> String {
    fn push(out: &mut String, s: &str, in_query: bool) {
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            let keep = c.is_ascii_alphanumeric()
                || matches!(c, b'-' | b'.' | b'_' | b'~' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':' | b'@' | b'/')
                || (in_query && c == b'?');
            if keep {
                out.push(c as char);
            } else if in_query && c == b'%' && i + 2 < b.len() && b[i + 1].is_ascii_hexdigit() && b[i + 2].is_ascii_hexdigit() {
                out.push('%');
            } else {
                out.push_str(&format!("%{:02X}", c));
            }
            i += 1;
        }
    }
    let mut out = String::with_capacity(path.len() + 1);
    if !path.starts_with('/') {
        out.push('/');
    }
    push(&mut out, path, false);
    if let Some(q) = query {
        out.push('?');
        push(&mut out, q, true);
    }
    out
}

/// Connection-level and message-framing headers that must not be copied from an origin response
/// onto a response the gateway frames itself.
fn is_hop_by_hop_or_framing_header(name: &str) -> bool {
    [
        "content-length",
        "transfer-encoding",
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "upgrade",
    ]
    .iter()
    .any(|n| name.eq_ignore_ascii_case(n))
}

/// `x-request-id` values the gateway is willing to forward upstream: [A-Za-z0-9._-]{1,64}.
fn is_valid_request_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 64
        && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

async fn fetch_origin_raw(
    origin_base: &str,
    cache_key: &CacheKey,
    headers: &HeaderMap,
    validators: Option<&StoredValidators>,
    policy: Option<&crate::security::upstream::UpstreamPolicy>,
) -> Result<RawHttpResponse, OriginError> {
    let parsed_url = if origin_base.starts_with("http://") || origin_base.starts_with("https://") {
        url::Url::parse(origin_base).map_err(|e| {
            OriginError::SecurityError(crate::security::upstream::OriginSecurityError::InvalidOriginUrl(
                format!("Failed to parse origin URL '{}': {}", origin_base, e),
            ))
        })?
    } else {
        url::Url::parse(&format!("http://{}", origin_base)).map_err(|e| {
            OriginError::SecurityError(crate::security::upstream::OriginSecurityError::InvalidOriginUrl(
                format!("Failed to parse origin URL '{}': {}", origin_base, e),
            ))
        })?
    };

    if let Some(pol) = policy {
        pol.validate_target(&parsed_url)?;
    } else {
        let port = parsed_url
            .port_or_known_default()
            .unwrap_or(if parsed_url.scheme() == "https" { 443 } else { 80 });
        crate::security::upstream::validate_port(port, parsed_url.as_str(), &[])?;
    }

    let is_https = parsed_url.scheme() == "https";
    let host = parsed_url.host_str().ok_or_else(|| {
        OriginError::SecurityError(crate::security::upstream::OriginSecurityError::InvalidOriginUrl(
            "Origin URL missing host".to_string(),
        ))
    })?;
    let port = parsed_url
        .port_or_known_default()
        .unwrap_or(if is_https { 443 } else { 80 });

    let filter = match policy {
        Some(pol) => crate::security::upstream::PreConnectIpFilter::new(
            pol.allow_loopback_origin,
            pol.allow_lan_origin,
        ),
        None => crate::security::upstream::PreConnectIpFilter::new(true, false),
    };

    let pinned_addr = crate::security::upstream::resolve_and_pin_origin(host, port, &filter).await?;

    let tcp_stream = tokio::time::timeout(
        Duration::from_millis(5000),
        tokio::net::TcpStream::connect(pinned_addr),
    )
    .await
    .map_err(|_| OriginError::Timeout)?
    .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    let mut transport = if is_https {
        let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|e| {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::TlsHostnameMismatch(
                    format!("Invalid DNS name {}: {}", host, e),
                ))
            })?;

        let connector = match policy.and_then(|p| p.tls_config.as_ref()) {
            Some(cfg) => cfg.connector.clone(),
            None => crate::security::upstream::UpstreamTlsConfig::new_default().connector,
        };

        let tls_stream = tokio::time::timeout(
            Duration::from_millis(5000),
            connector.connect(server_name, tcp_stream),
        )
        .await
        .map_err(|_| OriginError::SecurityError(crate::security::upstream::OriginSecurityError::Timeout))?
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("expired") || err_str.contains("Expired") {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::TlsCertificateExpired)
            } else if err_str.contains("UnknownIssuer")
                || err_str.contains("untrusted")
                || err_str.contains("CertNotValidForName")
                || err_str.contains("CertExpired")
            {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::TlsUntrustedRoot)
            } else {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::TlsHandshakeFailed(
                    err_str,
                ))
            }
        })?;

        UpstreamTransport::Tls(Box::new(tls_stream))
    } else {
        UpstreamTransport::Plain(tcp_stream)
    };

    let host_header = if (is_https && port == 443) || (!is_https && port == 80) {
        host.to_string()
    } else {
        format!("{}:{}", host, port)
    };

    let target = encode_origin_target(&cache_key.path, cache_key.query.as_deref());

    let mut req_str = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\n",
        target, host_header
    );

    let forward_auth = policy.map(|p| p.forward_client_authorization).unwrap_or(false);

    // M-1 (round-3 review): forward ONLY gateway-controlled request headers. The cache key is
    // tenant + path + query, so any other client-controlled header (`x-user-id`, `x-forwarded-*`,
    // `x-http-method-override`, `Accept`, ...) that influenced the origin response would let one
    // user choose a variant stored under the tenant key for everyone in the tenant (and for
    // coalesced waiters). Contract: origins must not vary responses on client-controlled request
    // headers. Forwarded: a client `x-request-id` only if it matches [A-Za-z0-9._-]{1,64};
    // `x-tenant-id` taken from the cache KEY (the token tenant in secure mode), never from the
    // client; `Authorization` only when PHANTOM_FORWARD_UPSTREAM_AUTH is set; a fixed
    // `Accept: application/json` (already in the request head above); the stored validators.
    // `x-cache-bypass` and every other `x-*` header are never forwarded.
    for (k, v) in headers {
        let name = k.as_str();
        if is_client_conditional_or_range_header(name) {
            continue;
        }
        if name.eq_ignore_ascii_case("authorization") {
            if forward_auth {
                if let Ok(v_str) = v.to_str() {
                    req_str.push_str(&format!("{}: {}\r\n", name, v_str));
                }
            }
        } else if name.eq_ignore_ascii_case("x-request-id") {
            if let Ok(v_str) = v.to_str() {
                if is_valid_request_id(v_str) {
                    req_str.push_str(&format!("x-request-id: {}\r\n", v_str));
                }
            }
        }
    }
    if cache_key.tenant_id != "__default__" {
        // tenant ids are validated to [A-Za-z0-9._-]{1,128} by CacheKey::validate
        req_str.push_str(&format!("x-tenant-id: {}\r\n", cache_key.tenant_id));
    }
    if let Some(v) = validators {
        if let Some(ref etag) = v.etag {
            req_str.push_str(&format!("If-None-Match: {}\r\n", etag));
        }
        if let Some(ref lm) = v.last_modified {
            req_str.push_str(&format!("If-Modified-Since: {}\r\n", lm));
        }
    }
    req_str.push_str("\r\n");

    transport
        .write_all(req_str.as_bytes())
        .await
        .map_err(|e| OriginError::NetworkError(e.to_string()))?;
    transport
        .flush()
        .await
        .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    let mut resp_bytes = Vec::new();
    let mut limited_reader = transport.take((MAX_TOTAL_RESPONSE_BYTES + 1) as u64);
    tokio::time::timeout(
        Duration::from_millis(5000),
        limited_reader.read_to_end(&mut resp_bytes),
    )
    .await
    .map_err(|_| OriginError::Timeout)?
    .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    parse_http_response(&resp_bytes)
}

pub fn parse_http_response(
    bytes: &[u8],
) -> Result<RawHttpResponse, OriginError> {
    let sep = b"\r\n\r\n";
    let sep_pos = bytes
        .windows(sep.len())
        .position(|window| window == sep)
        .ok_or_else(|| {
            if bytes.len() > MAX_HEADER_BYTES {
                OriginError::ResponseTooLarge
            } else {
                OriginError::NetworkError("Incomplete HTTP response: missing header delimiter".to_string())
            }
        })?;

    if sep_pos > MAX_HEADER_BYTES {
        return Err(OriginError::ResponseTooLarge);
    }

    let header_bytes = &bytes[..sep_pos];
    let raw_body = &bytes[sep_pos + sep.len()..];

    let header_str = std::str::from_utf8(header_bytes)
        .map_err(|e| OriginError::NetworkError(e.to_string()))?;

    let mut lines = header_str.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| OriginError::NetworkError("Missing status line".to_string()))?;

    let mut status_parts = status_line.split_whitespace();
    let _version = status_parts.next();
    let status_code_str = status_parts
        .next()
        .ok_or_else(|| OriginError::NetworkError("Missing status code".to_string()))?;
    let status_code: u16 = status_code_str
        .parse()
        .map_err(|_| OriginError::NetworkError("Invalid status code".to_string()))?;

    let mut headers = Vec::new();
    let mut content_lengths: Vec<usize> = Vec::new();
    let mut has_invalid_cl = false;
    let mut is_chunked = false;

    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let k_trim = k.trim();
            let v_trim = v.trim();
            if k_trim.eq_ignore_ascii_case("content-length") {
                let subparts: Vec<&str> = v_trim.split(',').map(|s| s.trim()).collect();
                for sub in subparts {
                    match sub.parse::<usize>() {
                        Ok(val) => content_lengths.push(val),
                        Err(_) => has_invalid_cl = true,
                    }
                }
            }
            if k_trim.eq_ignore_ascii_case("transfer-encoding") {
                if v_trim.eq_ignore_ascii_case("chunked") {
                    is_chunked = true;
                } else {
                    return Err(OriginError::SecurityError(
                        crate::security::upstream::OriginSecurityError::MalformedFraming(
                            format!("Unsupported Transfer-Encoding: '{}'", v_trim),
                        ),
                    ));
                }
            }
            if let Ok(name) = HeaderName::from_bytes(k_trim.as_bytes()) {
                if let Ok(val) = HeaderValue::from_str(v_trim) {
                    headers.push((Some(name), val));
                }
            }
        }
    }

    if has_invalid_cl {
        return Err(OriginError::SecurityError(
            crate::security::upstream::OriginSecurityError::MalformedFraming(
                "Invalid Content-Length header value".to_string(),
            ),
        ));
    }

    // Check multiple / conflicting Content-Length values
    let content_length = if !content_lengths.is_empty() {
        let first = content_lengths[0];
        if content_lengths.iter().any(|&val| val != first) {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Conflicting Content-Length headers detected".to_string(),
                ),
            ));
        }
        Some(first)
    } else {
        None
    };

    // RFC 9112 §6.1: Conflicting Content-Length and Transfer-Encoding chunked
    if content_length.is_some() && is_chunked {
        return Err(OriginError::SecurityError(
            crate::security::upstream::OriginSecurityError::MalformedFraming(
                "Conflicting Content-Length and Transfer-Encoding headers detected".to_string(),
            ),
        ));
    }

    // Check oversized Content-Length header early
    if let Some(cl) = content_length {
        if cl > MAX_BODY_BYTES {
            return Err(OriginError::ResponseTooLarge);
        }
    }

    let body = if status_code == 304 || status_code == 204 || (100..200).contains(&status_code) {
        // RFC 9112 6.3: these responses never have a body, whatever Content-Length says (a 304
        // may repeat the Content-Length of the 200 it stands for). The framing checks above
        // (conflicting/invalid lengths, unsupported Transfer-Encoding) still apply.
        Vec::new()
    } else if is_chunked {
        decode_chunked_body(raw_body, MAX_BODY_BYTES)?
    } else if let Some(cl) = content_length {
        if raw_body.len() < cl {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::TruncatedResponse(format!(
                    "Truncated response body: received {} bytes, expected {}",
                    raw_body.len(),
                    cl
                )),
            ));
        }
        let slice = &raw_body[..cl];
        if slice.len() > MAX_BODY_BYTES {
            return Err(OriginError::ResponseTooLarge);
        }
        slice.to_vec()
    } else {
        if raw_body.len() > MAX_BODY_BYTES {
            return Err(OriginError::ResponseTooLarge);
        }
        raw_body.to_vec()
    };

    if body.len() > MAX_BODY_BYTES {
        return Err(OriginError::ResponseTooLarge);
    }

    Ok((status_code, headers, body))
}

pub fn decode_chunked_body(mut bytes: &[u8], max_bytes: usize) -> Result<Vec<u8>, OriginError> {
    let mut body = Vec::new();
    loop {
        let crlf = bytes
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Malformed chunked response: missing CRLF after chunk size".to_string(),
                ))
            })?;
        let size_str = std::str::from_utf8(&bytes[..crlf])
            .map_err(|_| {
                OriginError::SecurityError(crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Malformed chunk size encoding (non-UTF8)".to_string(),
                ))
            })?;
        let trimmed_size = size_str.trim();
        if trimmed_size.is_empty() {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Empty chunk size line".to_string(),
                ),
            ));
        }
        let hex_part = trimmed_size.split(';').next().unwrap_or("").trim();
        if hex_part.is_empty() {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Empty chunk size line".to_string(),
                ),
            ));
        }
        let chunk_size = usize::from_str_radix(hex_part, 16).map_err(|_| {
            OriginError::SecurityError(crate::security::upstream::OriginSecurityError::MalformedFraming(
                format!("Invalid hex chunk size: '{}'", hex_part),
            ))
        })?;

        bytes = &bytes[crlf + 2..];

        if chunk_size == 0 {
            if !bytes.starts_with(b"\r\n") && !bytes.ends_with(b"\r\n\r\n") && bytes.is_empty() {
                return Err(OriginError::SecurityError(
                    crate::security::upstream::OriginSecurityError::TruncatedResponse(
                        "Truncated chunked response: missing terminating CRLF after chunk 0".to_string(),
                    ),
                ));
            }
            break; // Terminal chunk: 0\r\n\r\n
        }

        if chunk_size > max_bytes || body.len().saturating_add(chunk_size) > max_bytes {
            return Err(OriginError::ResponseTooLarge);
        }

        if bytes.len() < chunk_size {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::TruncatedResponse(
                    "Truncated chunked response: missing chunk data".to_string(),
                ),
            ));
        }

        body.extend_from_slice(&bytes[..chunk_size]);
        bytes = &bytes[chunk_size..];

        if bytes.starts_with(b"\r\n") {
            bytes = &bytes[2..];
        } else {
            return Err(OriginError::SecurityError(
                crate::security::upstream::OriginSecurityError::MalformedFraming(
                    "Malformed chunk: missing trailing CRLF after chunk data".to_string(),
                ),
            ));
        }
    }
    Ok(body)
}

// ==========================================
// Simulation loop preserved
// ==========================================

fn spawn_simulation_loop(state: Arc<ApiState>) {
    tokio::spawn(async move {
        let mut clock = 0u64;
        let mut loop_interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            loop_interval.tick().await;
            clock += 100;

            let workload = {
                let guard = state.workload_type.lock().unwrap();
                guard.clone()
            };

            let active_nodes_count = state
                .nodes
                .iter()
                .filter(|n| n.get_state() == NodeState::Active)
                .count();

            let target_throughput = if workload == "FlashCrowd" {
                62000.0 + (rand::random::<f64>() * 3000.0 - 1500.0)
            } else {
                12000.0 + (rand::random::<f64>() * 800.0 - 400.0)
            };

            let throughput = if active_nodes_count == 0 {
                0.0
            } else {
                target_throughput
            };

            let hit_rate = if active_nodes_count == 0 {
                0.0
            } else {
                let base = if workload == "FlashCrowd" { 0.94 } else { 0.88 };
                let penalty = if active_nodes_count < 3 {
                    (3 - active_nodes_count) as f32 * 0.05
                } else {
                    0.0
                };
                (base - penalty).max(0.1)
            };

            let coalesced_ratio = if workload == "FlashCrowd" { 0.85 } else { 0.40 };
            let backend_requests = throughput * (1.0 - hit_rate as f64) * (1.0 - coalesced_ratio);

            let db_capacity = 15000.0;
            let db_load = (backend_requests / db_capacity).min(1.0) as f32;

            let base_service_ms = 8.0;

            let mut samples = Vec::with_capacity(100);
            for _ in 0..100 {
                let is_hit = rand::random::<f64>() < hit_rate as f64;
                let lat = if is_hit {
                    1.0 + rand::random::<f64>() * 2.0
                } else {
                    let queue_delay = if db_load >= 1.0 {
                        500.0
                    } else {
                        let mean_delay = base_service_ms / (1.0 - db_load as f64);
                        -mean_delay * (1.0 - rand::random::<f64>()).ln()
                    };
                    10.0 + queue_delay
                };
                samples.push(lat);
            }

            samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let latency_p50 = samples[50] as u32;
            let latency_p95 = samples[95] as u32;
            let latency_p99 = samples[99] as u32;

            let backend_cost_per_mill = 0.15;
            let node_cost_per_hr = 0.05;

            let baseline_backend_requests = throughput * (1.0 - 0.70);
            let baseline_cost_hr =
                (baseline_backend_requests * 3600.0 / 1_000_000.0) * backend_cost_per_mill;

            let phantom_backend_cost_hr =
                (backend_requests * 3600.0 / 1_000_000.0) * backend_cost_per_mill;
            let phantom_overhead_cost_hr = active_nodes_count as f64 * node_cost_per_hr;
            let phantom_total_cost_hr = phantom_backend_cost_hr + phantom_overhead_cost_hr;

            let savings_hr = (baseline_cost_hr - phantom_total_cost_hr).max(0.0);
            let avoided_dollars = (clock as f64 / 3600000.0) * savings_hr;

            let (live_throughput, live_p50, live_p95, live_p99, live_db_load) =
                if let Some(ref cache) = state.cache {
                    let hits = cache.counters.cache_hits.load(Ordering::Relaxed);
                    let misses = cache.counters.cache_misses.load(Ordering::Relaxed);
                    let requests = cache.counters.requests_total.load(Ordering::Relaxed);
                    let total = hits + misses;
                    let hr = if total > 0 {
                        hits as f32 / total as f32
                    } else {
                        1.0
                    };
                    let (p50, p90, p99) = if let Some(ref tel) = state.telemetry {
                        tel.calculate_percentiles()
                    } else {
                        (latency_p50, latency_p95, latency_p99)
                    };
                    (requests as f64, p50, p90, p99, (1.0 - hr).max(0.0))
                } else {
                    (throughput, latency_p50, latency_p95, latency_p99, db_load)
                };

            let mut snap_guard = state.snapshot.lock().unwrap();
            snap_guard.timestamp_ms = clock;
            snap_guard.throughput_req_sec = live_throughput;
            snap_guard.p50_latency_ms = live_p50;
            snap_guard.p95_latency_ms = live_p95;
            snap_guard.p99_latency_ms = live_p99;
            snap_guard.db_load_ratio = live_db_load;
            snap_guard.avoided_compute_dollars = avoided_dollars;

            if let (Some(pie), Some(tel)) = (&state.pie, &state.telemetry) {
                if tel.len() >= 1000 {
                    let cand = if pie.get_active_policy() == "SIEVE" {
                        "S3-FIFO"
                    } else {
                        "LRU"
                    };
                    pie.evaluate_candidate(cand, clock, true);
                }
            }

            snap_guard.nodes.clear();
            for n in &state.nodes {
                let state_str = format!("{:?}", n.get_state());
                let (cpu, mem) = if n.get_state() == NodeState::Active {
                    if workload == "FlashCrowd" {
                        (85.2, 1073741824u64)
                    } else {
                        (28.4, 536870912u64)
                    }
                } else {
                    (0.0, 0u64)
                };
                snap_guard.nodes.push(NodeSnapshot {
                    node_id: n.node_id,
                    state: state_str,
                    cpu_utilization: cpu,
                    memory_used_bytes: mem,
                });
            }
        }
    });
}
