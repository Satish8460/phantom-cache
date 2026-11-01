use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tokio::net::TcpListener;

use phantom_cache_demo::api::server::{build_schema_v2_payload, create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::intel::telemetry::{RawObservationUs, TelemetryCollector};

#[derive(Clone)]
struct MockOriginState {
    pub request_count: Arc<AtomicU64>,
}

async fn mock_origin_handler(
    State(state): State<MockOriginState>,
    path: Option<Path<String>>,
) -> impl IntoResponse {
    let count = state
        .request_count
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path_str = path.map(|p| p.0).unwrap_or_else(|| "root".to_string());
    (
        StatusCode::OK,
        [("Cache-Control", "public, max-age=60")],
        Json(serde_json::json!({
            "key": path_str,
            "req_num": count
        })),
    )
        .into_response()
}

async fn spawn_mock_origin() -> (String, Arc<AtomicU64>) {
    let request_count = Arc::new(AtomicU64::new(0));
    let state = MockOriginState {
        request_count: request_count.clone(),
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
    (format!("http://127.0.0.1:{}", port), request_count)
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

fn assert_no_simulated_in_value(v: &serde_json::Value, path: &str) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(src) = map.get("src") {
                if src == "simulated" {
                    panic!("Contract violation: found 'src: simulated' at path '{}'", path);
                }
            }
            for (k, child) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                assert_no_simulated_in_value(child, &child_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, child) in arr.iter().enumerate() {
                let child_path = format!("{}[{}]", path, idx);
                assert_no_simulated_in_value(child, &child_path);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// 1. Percentile accuracy vs exact on seeded distributions
// ---------------------------------------------------------------------------

#[test]
fn test_percentile_accuracy_seeded_distribution() {
    let collector = TelemetryCollector::new(2000);

    // Seed 1,000 observations: 1 µs, 2 µs, ..., 1,000 µs
    for i in 1u64..=1000u64 {
        collector.record_us(RawObservationUs::new_get(
            1_000_000 + i,
            i,
            i,
            true,
            0,
        ));
    }

    let (p50, p90, p95, p99) = collector.calculate_percentiles_us();
    // 50% of 1000 = index 500 -> 501 µs (0-indexed)
    // 90% of 1000 = index 900 -> 901 µs
    // 95% of 1000 = index 950 -> 951 µs
    // 99% of 1000 = index 990 -> 991 µs
    assert_eq!(p50, 501, "Expected p50_us = 501, got {}", p50);
    assert_eq!(p90, 901, "Expected p90_us = 901, got {}", p90);
    assert_eq!(p95, 951, "Expected p95_us = 951, got {}", p95);
    assert_eq!(p99, 991, "Expected p99_us = 991, got {}", p99);

    let (p50_ms, p90_ms, p95_ms, p99_ms) = collector.calculate_percentiles_f64_ms();
    assert!((p50_ms - 0.501).abs() < 1e-6);
    assert!((p90_ms - 0.901).abs() < 1e-6);
    assert!((p95_ms - 0.951).abs() < 1e-6);
    assert!((p99_ms - 0.991).abs() < 1e-6);
}

#[test]
fn test_empty_and_single_observation_behavior() {
    let collector = TelemetryCollector::new(100);

    // Empty buffer
    assert_eq!(collector.calculate_percentiles_us(), (0, 0, 0, 0));
    assert_eq!(collector.calculate_percentiles_f64_ms(), (0.0, 0.0, 0.0, 0.0));

    // Single observation
    collector.record_us(RawObservationUs::new_get(100, 1, 420, true, 0));
    assert_eq!(collector.calculate_percentiles_us(), (420, 420, 420, 420));
    assert_eq!(collector.calculate_percentiles_f64_ms(), (0.42, 0.42, 0.42, 0.42));

    // After clear()
    collector.clear();
    assert_eq!(collector.calculate_percentiles_us(), (0, 0, 0, 0));
    assert_eq!(collector.calculate_percentiles_f64_ms(), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn test_ring_buffer_eviction_percentiles() {
    let collector = TelemetryCollector::new(10);

    // Insert 10 observations with 100 µs
    for i in 0..10 {
        collector.record_us(RawObservationUs::new_get(100 + i, i, 100, true, 0));
    }
    assert_eq!(collector.calculate_percentiles_us(), (100, 100, 100, 100));

    // Insert 10 new observations with 500 µs -> pushes out the 100 µs ones
    for i in 0..10 {
        collector.record_us(RawObservationUs::new_get(200 + i, 10 + i, 500, true, 0));
    }
    assert_eq!(collector.calculate_percentiles_us(), (500, 500, 500, 500));
}

#[test]
fn test_delete_excluded_from_microsecond_percentiles() {
    let collector = TelemetryCollector::new(100);

    // Record GET at 50 µs
    collector.record_us(RawObservationUs::new_get(100, 1, 50, true, 0));

    // Record DELETE at 10,000 µs
    collector.record_us(RawObservationUs::new_delete(101, 1, 10_000));

    // Percentiles must ONLY reflect GET (50 µs), not DELETE
    assert_eq!(collector.calculate_percentiles_us(), (50, 50, 50, 50));
    assert_eq!(collector.observations_us().len(), 2);
}

// ---------------------------------------------------------------------------
// 2. Loopback hit path test asserting non-zero sub-ms percentiles (p50_us > 0)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_loopback_hit_sub_millisecond_resolution() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (origin_url, _) = spawn_mock_origin().await;
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(200));

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
        let key_url = format!("http://127.0.0.1:{}/api/v1/cache/item_loopback", port);

        // Warm cache with initial GET (MISS)
        let r1 = client.get(&key_url).send().await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        assert_eq!(r1.headers().get("X-Cache-Status").unwrap(), "MISS");

        // Fire 10 subsequent GET requests (all HITS)
        for _ in 0..10 {
            let r = client.get(&key_url).send().await.unwrap();
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers().get("X-Cache-Status").unwrap(), "HIT");
        }

        // Verify telemetry recorded all observations in microsecond resolution
        let (p50_us, p90_us, p95_us, p99_us) = telemetry.calculate_percentiles_us();
        assert!(
            p50_us > 0,
            "p50_us must be strictly > 0 for loopback cache hits (measured {} µs)",
            p50_us
        );
        assert!(p90_us >= p50_us, "p90_us ({}) >= p50_us ({})", p90_us, p50_us);
        assert!(p95_us >= p90_us, "p95_us ({}) >= p90_us ({})", p95_us, p90_us);
        assert!(p99_us >= p95_us, "p99_us ({}) >= p95_us ({})", p99_us, p95_us);

        // Check microsecond observations collection
        let obs_us = telemetry.observations_us();
        assert_eq!(obs_us.len(), 11); // 1 miss + 10 hits
        for obs in &obs_us {
            assert!(obs.latency_us > 0, "Latency must be strictly positive");
        }
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// 3. Schema v2 contract tests (microsecond resolution metadata, provenance)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_schema_v2_latency_resolution_contract() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let cache = Arc::new(PartitionedCache::new(4));
        let telemetry = Arc::new(TelemetryCollector::new(200));

        // Seed some microsecond observations
        telemetry.record_us(RawObservationUs::new_get(100, 1, 250, true, 0));
        telemetry.record_us(RawObservationUs::new_get(101, 2, 500, true, 0));
        telemetry.record_us(RawObservationUs::new_get(102, 3, 1000, true, 0));

        let state = ApiState {
            snapshot: Mutex::new(make_dummy_snapshot()),
            nodes: vec![],
            workload_type: Mutex::new("Normal".to_string()),
            cache: Some(cache),
            origin_url: None,
            telemetry: Some(telemetry.clone()),
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        };

        let json_body = build_schema_v2_payload(&state);

        // Contract: latency_ms has resolution: "microsecond"
        let lat = &json_body["metrics"]["latency_ms"];
        assert_eq!(lat["src"], "measured");
        assert_eq!(lat["unit"], "ms");
        assert_eq!(lat["resolution"], "microsecond");

        // Verify microsecond and float millisecond fields exist
        assert!(lat["p50_us"].is_number());
        assert!(lat["p90_us"].is_number());
        assert!(lat["p95_us"].is_number());
        assert!(lat["p99_us"].is_number());
        assert!(lat["p50"].is_number());
        assert!(lat["p90"].is_number());
        assert!(lat["p95"].is_number());
        assert!(lat["p99"].is_number());

        assert_eq!(lat["p50_us"], 500);
        assert_eq!(lat["p50"], 0.5);

        // Contract: ZERO fields with src: "simulated" in live mode
        assert_no_simulated_in_value(&json_body, "root");
    })
    .await
    .expect("test timed out");
}

#[test]
fn test_schema_v2_unavailable_latency_contract() {
    let cache = Arc::new(PartitionedCache::new(4));
    let telemetry = Arc::new(TelemetryCollector::new(200)); // empty

    let state = ApiState {
        snapshot: Mutex::new(make_dummy_snapshot()),
        nodes: vec![],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache),
        origin_url: None,
        telemetry: Some(telemetry),
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    };

    let json_body = build_schema_v2_payload(&state);

    let lat = &json_body["metrics"]["latency_ms"];
    assert_eq!(lat["src"], "unavailable");
    assert_eq!(lat["reason"], "no_samples");
    assert_eq!(lat["unit"], "ms");
    // Crucial schema contract: unavailable metrics MUST NOT contain a 'v' field
    assert!(lat.get("v").is_none());
}
