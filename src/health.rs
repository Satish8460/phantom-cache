//! Liveness / readiness handlers (M7-02), designed as a self-contained router so that they
//! can be integrated into `api::server::create_app` with one `.merge(...)` line once
//! `src/api/server.rs` is available for editing (deferred integration item, see
//! `docs/release/deferred_integration.md`).
//!
//! Semantics (all values are measured process state, not simulated):
//! * `GET /health/live`  : 200 while the process can serve a request. No dependencies checked.
//! * `GET /health/ready` : 200 only when configuration validated, the ingress listener accepts
//!   connections, and the process is not draining; otherwise 503 with the failing checks.
//!
//! The pre-existing `GET /health` (plain `OK`) is unchanged.

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Default)]
pub struct Readiness {
    config_valid: AtomicBool,
    listening: AtomicBool,
    draining: AtomicBool,
}

impl Readiness {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn mark_config_valid(&self) {
        self.config_valid.store(true, Ordering::SeqCst);
    }
    pub fn mark_listening(&self) {
        self.listening.store(true, Ordering::SeqCst);
    }
    /// Called on shutdown signal: readiness drops immediately, liveness stays 200.
    pub fn begin_draining(&self) {
        self.draining.store(true, Ordering::SeqCst);
    }
    pub fn is_ready(&self) -> bool {
        self.config_valid.load(Ordering::SeqCst)
            && self.listening.load(Ordering::SeqCst)
            && !self.draining.load(Ordering::SeqCst)
    }
    pub fn checks(&self) -> Value {
        json!({
            "config_valid": self.config_valid.load(Ordering::SeqCst),
            "listening": self.listening.load(Ordering::SeqCst),
            "draining": self.draining.load(Ordering::SeqCst),
        })
    }
}

static GLOBAL: OnceLock<Arc<Readiness>> = OnceLock::new();

/// Process-wide readiness state shared by `main` and the router.
pub fn global() -> Arc<Readiness> {
    GLOBAL.get_or_init(|| Arc::new(Readiness::new())).clone()
}

async fn live() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "alive" })))
}

async fn ready(State(r): State<Arc<Readiness>>) -> (StatusCode, Json<Value>) {
    if r.is_ready() {
        (StatusCode::OK, Json(json!({ "status": "ready", "checks": r.checks() })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "checks": r.checks() })),
        )
    }
}

/// Router exposing `/health/live` and `/health/ready`. Merge into the application router:
/// `create_app(state).merge(crate::health::router(crate::health::global()))`.
pub fn router(readiness: Arc<Readiness>) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .with_state(readiness)
}
