//! M7-02 liveness/readiness endpoints and shutdown semantics.

use phantom_cache_demo::health::{router, Readiness};
use std::sync::Arc;

async fn serve(r: Arc<Readiness>) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    let app = router(r);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{}", a)
}

#[tokio::test]
async fn m7_live_is_always_200_and_ready_tracks_state() {
    let r = Arc::new(Readiness::new());
    let base = serve(r.clone()).await;
    let get = |p: &str| reqwest::get(format!("{}{}", base, p));

    assert_eq!(get("/health/live").await.unwrap().status(), 200);
    let not_ready = get("/health/ready").await.unwrap();
    assert_eq!(not_ready.status(), 503);
    let body: serde_json::Value = not_ready.json().await.unwrap();
    assert_eq!(body["status"], "not_ready");
    assert_eq!(body["checks"]["listening"], false);

    r.mark_config_valid();
    assert_eq!(get("/health/ready").await.unwrap().status(), 503, "config alone is not ready");
    r.mark_listening();
    let ready = get("/health/ready").await.unwrap();
    assert_eq!(ready.status(), 200);
    assert_eq!(ready.json::<serde_json::Value>().await.unwrap()["status"], "ready");

    r.begin_draining();
    assert_eq!(get("/health/ready").await.unwrap().status(), 503, "draining must fail readiness");
    assert_eq!(get("/health/live").await.unwrap().status(), 200, "liveness stays 200 while draining");
}

#[test]
fn m7_readiness_default_is_not_ready() {
    assert!(!Readiness::new().is_ready());
}

#[tokio::test]
async fn m7_health_routes_are_wired_into_create_app_without_changing_health() {
    use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
    use std::sync::Mutex;
    let st = Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".into(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![],
        }),
        nodes: vec![],
        workload_type: Mutex::new("Normal".into()),
        cache: None,
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    let app = create_app(st);
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    let base = format!("http://{}", a);

    let h = reqwest::get(format!("{}/health", base)).await.unwrap();
    assert_eq!(h.status(), 200);
    assert_eq!(h.text().await.unwrap(), "OK", "/health body is unchanged");
    assert_eq!(reqwest::get(format!("{}/health/live", base)).await.unwrap().status(), 200);
    // The process-global readiness has not been marked ready in a test process.
    assert_eq!(reqwest::get(format!("{}/health/ready", base)).await.unwrap().status(), 503);
}
