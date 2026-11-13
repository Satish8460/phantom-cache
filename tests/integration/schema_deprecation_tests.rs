use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

fn make_dummy_snapshot() -> SimulationSnapshot {
    SimulationSnapshot {
        version: "v1.0-test".to_string(),
        timestamp_ms: 1000,
        throughput_req_sec: 100.0,
        p50_latency_ms: 10,
        p95_latency_ms: 25,
        p99_latency_ms: 50,
        db_load_ratio: 0.1,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    }
}

fn make_test_state() -> Arc<ApiState> {
    Arc::new(ApiState {
        snapshot: Mutex::new(make_dummy_snapshot()),
        nodes: vec![],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(Arc::new(PartitionedCache::new(2))),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    })
}

#[tokio::test]
async fn test_schema_v1_metrics_returns_deprecation_headers() {
    let state = make_test_state();
    let app = create_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1. Query with ?schema=1 -> MUST contain RFC 8594 Deprecation headers
    let resp_v1 = client
        .get(format!("http://{}/api/v1/metrics?schema=1", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_v1.status(), reqwest::StatusCode::OK);
    assert_eq!(
        resp_v1.headers().get("Deprecation").and_then(|v| v.to_str().ok()),
        Some("true")
    );
    assert!(resp_v1.headers().get("Sunset").is_some());
    assert!(resp_v1.headers().get("Link").is_some());

    // 2. Query with ?schema=2 -> MUST NOT contain Deprecation header
    let resp_v2 = client
        .get(format!("http://{}/api/v1/metrics?schema=2", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_v2.status(), reqwest::StatusCode::OK);
    assert!(
        resp_v2.headers().get("Deprecation").is_none(),
        "Schema v2 must not contain Deprecation header"
    );

    // 3. Query without query params (defaults to v2) -> MUST NOT contain Deprecation header
    let resp_default = client
        .get(format!("http://{}/api/v1/metrics", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_default.status(), reqwest::StatusCode::OK);
    assert!(resp_default.headers().get("Deprecation").is_none());
}

#[tokio::test]
async fn test_schema_v1_sse_returns_deprecation_headers() {
    let state = make_test_state();
    let app = create_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1. SSE with ?schema=2 -> NO Deprecation header
    let resp_v2 = client
        .get(format!("http://{}/api/events?schema=2", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_v2.status(), reqwest::StatusCode::OK);
    assert!(
        resp_v2.headers().get("Deprecation").is_none(),
        "SSE Schema v2 must not contain Deprecation header"
    );

    // 2. SSE without ?schema=2 (defaults to legacy v1) -> MUST contain Deprecation header
    let resp_v1 = client
        .get(format!("http://{}/api/events", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_v1.status(), reqwest::StatusCode::OK);
    assert_eq!(
        resp_v1.headers().get("Deprecation").and_then(|v| v.to_str().ok()),
        Some("true"),
        "SSE legacy stream must contain Deprecation header"
    );
    assert!(resp_v1.headers().get("Sunset").is_some());
    assert!(resp_v1.headers().get("Link").is_some());
}
