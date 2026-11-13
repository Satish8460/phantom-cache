//! Benchmark B8e: End-to-End Gateway Overhead (Core C5)
//!
//! Specifications:
//! - C5: Loopback GET of a cached key, concurrency 16, telemetry {on, off}.
//! - Non-core: Concurrency 1.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use hdrhistogram::Histogram;
use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

#[derive(Clone)]
struct MockOriginState {
    count: Arc<AtomicU64>,
}

async fn origin_handler(State(state): State<MockOriginState>, path: Option<Path<String>>) -> Response {
    state.count.fetch_add(1, Ordering::SeqCst);
    let p = path.map(|s| s.0).unwrap_or_default();
    (StatusCode::OK, Json(serde_json::json!({ "key": p, "val": "cached_data" }))).into_response()
}

async fn spawn_mock_origin() -> String {
    let app = Router::new()
        .route("/*path", get(origin_handler))
        .with_state(MockOriginState {
            count: Arc::new(AtomicU64::new(0)),
        });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://127.0.0.1:{}", port)
}

async fn run_gateway_bench(concurrency: usize, duration_secs: u64, enable_telemetry: bool) -> serde_json::Value {
    let origin_url = spawn_mock_origin().await;
    let cache = Arc::new(PartitionedCache::new(8));
    let telemetry = if enable_telemetry {
        Some(Arc::new(TelemetryCollector::new(10_000)))
    } else {
        None
    };

    let state = Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 1000.0,
            p50_latency_ms: 1,
            p95_latency_ms: 2,
            p99_latency_ms: 5,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 10.0,
                memory_used_bytes: 1024,
            }],
        }),
        nodes: vec![],
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache),
        origin_url: Some(origin_url),
        telemetry,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gw_port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, create_app(state)).await.unwrap();
    });

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(concurrency * 2)
        .build()
        .unwrap();

    let target_url = format!("http://127.0.0.1:{}/api/v1/cache/cached_item", gw_port);

    // Warm-up and pre-cache
    let r = client.get(&target_url).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    let running = Arc::new(AtomicBool::new(true));
    let measuring = Arc::new(AtomicBool::new(false));
    let total_reqs = Arc::new(AtomicU64::new(0));
    let mut tasks = Vec::new();

    for _ in 0..concurrency {
        let cl = client.clone();
        let u = target_url.clone();
        let r = running.clone();
        let m = measuring.clone();
        let counter = total_reqs.clone();

        tasks.push(tokio::spawn(async move {
            let mut hist = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
            let mut local_reqs: u64 = 0;
            while r.load(Ordering::Relaxed) {
                let is_m = m.load(Ordering::Relaxed);
                let start = if is_m { Some(Instant::now()) } else { None };
                let resp = cl.get(&u).send().await;
                if let Ok(res) = resp {
                    if res.status() == StatusCode::OK {
                        if let Some(s) = start {
                            let elapsed_ns = s.elapsed().as_nanos() as u64;
                            let _ = hist.record(elapsed_ns.clamp(1, 60_000_000_000));
                            local_reqs += 1;
                        }
                    }
                }
            }
            counter.fetch_add(local_reqs, Ordering::SeqCst);
            hist
        }));
    }

    // Warm-up 2 seconds
    tokio::time::sleep(Duration::from_secs(2)).await;

    let start = Instant::now();
    measuring.store(true, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_secs(duration_secs)).await;
    measuring.store(false, Ordering::SeqCst);
    running.store(false, Ordering::SeqCst);

    let mut merged = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
    for t in tasks {
        let h = t.await.unwrap();
        merged.add(h).unwrap();
    }

    let actual_duration = start.elapsed().as_secs_f64();
    let reqs = total_reqs.load(Ordering::SeqCst);
    let throughput = (reqs as f64) / actual_duration;

    serde_json::json!({
        "concurrency": concurrency,
        "telemetry_enabled": enable_telemetry,
        "duration_sec": actual_duration,
        "total_requests": reqs,
        "throughput_req_sec": throughput,
        "latency_us": {
            "p50": merged.value_at_quantile(0.50) / 1000,
            "p90": merged.value_at_quantile(0.90) / 1000,
            "p95": merged.value_at_quantile(0.95) / 1000,
            "p99": merged.value_at_quantile(0.99) / 1000,
            "max": merged.max() / 1000
        }
    })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let duration = if args.len() > 1 && args[1] == "--quick" {
        2
    } else {
        4
    };

    // Core C5: concurrency 16, telemetry on vs off
    let c5_tel_off = run_gateway_bench(16, duration, false).await;
    let c5_tel_on = run_gateway_bench(16, duration, true).await;

    // Non-core: concurrency 1
    let noncore_c1 = run_gateway_bench(1, duration, false).await;

    let output = serde_json::json!({
        "benchmark": "B8e",
        "c5_concurrency_16_telemetry_off": c5_tel_off,
        "c5_concurrency_16_telemetry_on": c5_tel_on,
        "noncore_concurrency_1": noncore_c1
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
