use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::intel::telemetry::{fnv1a_64, ObservationKind, TelemetryCollector};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;

#[derive(Clone)]
struct MockOriginState {
    pub request_count: Arc<AtomicU64>,
    pub return_500: Arc<std::sync::atomic::AtomicBool>,
}

async fn mock_origin_handler(
    State(state): State<MockOriginState>,
    path: Option<Path<String>>,
) -> impl IntoResponse {
    let count = state.request_count.fetch_add(1, Ordering::SeqCst) + 1;
    let path_str = path.map(|p| p.0).unwrap_or_else(|| "/".to_string());

    if state.return_500.load(Ordering::SeqCst) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Simulated origin error",
                "req_num": count
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "key": path_str,
            "req_num": count
        })),
    )
        .into_response()
}

async fn spawn_mock_origin() -> (String, Arc<AtomicU64>, Arc<std::sync::atomic::AtomicBool>) {
    let request_count = Arc::new(AtomicU64::new(0));
    let return_500 = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let state = MockOriginState {
        request_count: request_count.clone(),
        return_500: return_500.clone(),
    };
    let app = Router::new()
        .route("/*path", get(mock_origin_handler))
        .route("/", get(mock_origin_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://127.0.0.1:{}", port), request_count, return_500)
}

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

// ---------------------------------------------------------------------------
// 1. DELETE does not decrease hit rate or enter latency percentiles
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_does_not_affect_hit_rate_or_latency_percentiles() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let key_url = format!("http://127.0.0.1:{}/api/v1/cache/item_alpha", port);

        // First GET: miss -> filled from origin
        let r1 = client.get(&key_url).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(r1.headers().get("X-Cache-Status").unwrap(), "MISS");

        // Second GET: hit
        let r2 = client.get(&key_url).send().await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);
        assert_eq!(r2.headers().get("X-Cache-Status").unwrap(), "HIT");

        // Observations should have 2 GETs
        let obs = telemetry.observations();
        assert_eq!(obs.len(), 2);
        assert_eq!(obs[0].kind, ObservationKind::Get);
        assert_eq!(obs[1].kind, ObservationKind::Get);

        let p95_before = telemetry.calculate_p95();
        let (p50_before, p90_before, p99_before) = telemetry.calculate_percentiles();

        // DELETE: purges key
        let del = client.delete(&key_url).send().await.unwrap();
        assert_eq!(del.status(), StatusCode::NO_CONTENT);

        // Observation buffer length should increment by 1
        let obs_after = telemetry.observations();
        assert_eq!(obs_after.len(), 3);
        assert_eq!(obs_after[2].kind, ObservationKind::Delete);

        // Latency percentiles MUST NOT be diluted or changed by the 0ms DELETE observation
        let p95_after = telemetry.calculate_p95();
        let (p50_after, p90_after, p99_after) = telemetry.calculate_percentiles();
        assert_eq!(p95_before, p95_after);
        assert_eq!(p50_before, p50_after);
        assert_eq!(p90_before, p90_after);
        assert_eq!(p99_before, p99_after);
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 2. Real FNV-1a key hash is recorded in observations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_real_fnv1a_key_hash_recorded() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let path = "/test/fnv1a/sample_item";
        let key_url = format!("http://127.0.0.1:{}/api/v1/cache{}", port, path);
        let expected_key = phantom_cache_demo::core::cache::CacheKey::new(None, path, None);
        let expected_hash = fnv1a_64(&expected_key.to_canonical_string());
        assert_ne!(expected_hash, 0, "FNV-1a hash must not be zero");

        // GET
        let _ = client.get(&key_url).send().await.unwrap();
        let obs = telemetry.observations();
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].key_hash, expected_hash);

        // DELETE
        let _ = client.delete(&key_url).send().await.unwrap();
        let obs2 = telemetry.observations();
        assert_eq!(obs2.len(), 2);
        assert_eq!(obs2[1].key_hash, expected_hash);
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 3. Schema v2 field catalog, provenance contract, and absence of simulated fields
// ---------------------------------------------------------------------------

fn assert_no_simulated_in_value(v: &serde_json::Value, path: &str) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(src) = map.get("src") {
                if src == "simulated" {
                    panic!("Contract violation: found 'src: simulated' at path '{}'", path);
                }
            }
            for (k, val) in map {
                let sub_path = format!("{}.{}", path, k);
                assert_no_simulated_in_value(val, &sub_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, val) in arr.iter().enumerate() {
                let sub_path = format!("{}[{}]", path, idx);
                assert_no_simulated_in_value(val, &sub_path);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_schema_v2_field_catalog_and_provenance_live_mode() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();
        let metrics_url = format!("http://127.0.0.1:{}/api/v1/metrics", port);

        let resp = client.get(&metrics_url).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json_body: serde_json::Value = resp.json().await.unwrap();

        // 1. Schema version and mode
        assert_eq!(json_body["schema"], 2);
        assert_eq!(json_body["mode"], "live");

        // 2. Capabilities
        assert_eq!(json_body["capabilities"]["cluster"], "disabled");
        assert_eq!(json_body["capabilities"]["controller"], "none");
        assert_eq!(json_body["capabilities"]["origin_metrics"], "available");

        // 3. Measured / derived metrics
        assert_eq!(json_body["metrics"]["requests_total"]["src"], "measured");
        assert_eq!(json_body["metrics"]["cache_hits"]["src"], "measured");
        assert_eq!(json_body["metrics"]["cache_misses"]["src"], "measured");
        // BD-02: this request is the first metrics read, no traffic yet; 1.0 was a fabricated default.
        assert_eq!(json_body["metrics"]["hit_ratio_cumulative"]["src"], "unavailable");
        assert_eq!(json_body["metrics"]["hit_ratio_cumulative"]["reason"], "no_requests");
        assert!(json_body["metrics"]["hit_ratio_cumulative"].get("v").is_none());

        // 4. Byte accounting metrics (measured per S9)
        assert_eq!(json_body["metrics"]["bytes_resident"]["src"], "measured");
        assert_eq!(json_body["metrics"]["bytes_resident"]["basis"], "logical_accounting");
        assert!(json_body["metrics"]["bytes_resident"]["v"].is_number());
        assert_eq!(json_body["metrics"]["byte_budget"]["src"], "measured");
        assert!(json_body["metrics"]["byte_budget"]["v"].is_number());
        assert_eq!(json_body["metrics"]["byte_budget_rejections"]["src"], "measured");
        assert_eq!(json_body["metrics"]["oversize_rejections"]["src"], "measured");

        // 5. Unavailable fields must carry reason and have no 'v' field
        assert_eq!(json_body["hot_keys"]["src"], "unavailable");
        assert!(json_body["hot_keys"].get("v").is_none());

        // 6. Origin metrics
        assert_eq!(json_body["metrics"]["origin"]["requests_started"]["src"], "measured");
        assert_eq!(json_body["metrics"]["origin"]["in_flight"]["src"], "measured");

        // 7. Mandatory contract test: in live mode, ZERO fields have src: "simulated"
        assert_no_simulated_in_value(&json_body, "root");

        // 7. Absence of simulated fields in live mode
        assert!(json_body.get("avoided_compute_dollars").is_none());
        assert!(json_body.get("backend_load_ratio").is_none());
        assert!(json_body.get("cost").is_none());
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 4. Origin metrics tracking (in-flight, success, error classification)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_origin_metrics_tracking() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _, return_500) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        let client = reqwest::Client::new();

        // 1. Successful fetch
        let key_ok = format!("http://127.0.0.1:{}/api/v1/cache/item_origin_ok", port);
        let r_ok = client.get(&key_ok).send().await.unwrap();
        assert_eq!(r_ok.status(), StatusCode::OK);

        let snap1 = telemetry.origin_metrics.snapshot();
        assert_eq!(snap1.requests_started, 1);
        assert_eq!(snap1.requests_completed_ok, 1);
        assert_eq!(snap1.in_flight, 0);
        assert_eq!(snap1.errors_status_5xx, 0);

        // 2. Upstream 500 fetch
        return_500.store(true, Ordering::SeqCst);
        let key_err = format!("http://127.0.0.1:{}/api/v1/cache/item_origin_err", port);
        let r_err = client.get(&key_err).send().await.unwrap();
        assert_eq!(r_err.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let snap2 = telemetry.origin_metrics.snapshot();
        assert_eq!(snap2.requests_started, 2);
        assert_eq!(snap2.requests_completed_ok, 1);
        assert_eq!(snap2.in_flight, 0);
        assert_eq!(snap2.errors_status_5xx, 1);
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 5. SSE stream supports ?schema=2 for v2 payload, defaults to v1
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sse_schema_v2_and_v1_fallback() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(100));

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: Some(origin_url),
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, create_app(state)).await.unwrap();
        });

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        // 1. SSE with ?schema=2
        let mut stream_v2 = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        stream_v2
            .write_all(b"GET /api/events?schema=2 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut reader_v2 = BufReader::new(stream_v2);
        let mut line_buf = String::new();
        let mut found_v2_event = false;
        for _ in 0..20 {
            line_buf.clear();
            if reader_v2.read_line(&mut line_buf).await.unwrap() == 0 {
                break;
            }
            if line_buf.starts_with("data:") {
                let data_json = line_buf.trim_start_matches("data:").trim();
                let parsed: serde_json::Value = serde_json::from_str(data_json).unwrap();
                assert_eq!(parsed["schema"], 2);
                assert_eq!(parsed["mode"], "live");
                found_v2_event = true;
                break;
            }
        }
        assert!(found_v2_event, "Did not receive valid SSE event with schema 2");

        // 2. SSE default (v1)
        let mut stream_v1 = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        stream_v1
            .write_all(b"GET /api/events HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut reader_v1 = BufReader::new(stream_v1);
        let mut found_v1_event = false;
        for _ in 0..20 {
            line_buf.clear();
            if reader_v1.read_line(&mut line_buf).await.unwrap() == 0 {
                break;
            }
            if line_buf.starts_with("data:") {
                let data_json = line_buf.trim_start_matches("data:").trim();
                let parsed: serde_json::Value = serde_json::from_str(data_json).unwrap();
                assert_eq!(parsed["version"], "v1.0-test");
                assert!(parsed["metrics"].get("p95_latency_ms").is_some());
                assert!(parsed.get("hot_keys").is_some());
                found_v1_event = true;
                break;
            }
        }
        assert!(found_v1_event, "Did not receive valid v1 SSE event");
    })
    .await
    .expect("test timed out");
}
