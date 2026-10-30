use crate::dist::node::{NodeState, VirtualNode};
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

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
}

pub fn create_app(shared_state: Arc<ApiState>) -> Router {
    let cors = CorsLayer::permissive();
    Router::new()
        .route("/health", get(health_check))
        .route("/api/events", get(sse_handler))
        .route("/api/control", post(control_handler))
        .with_state(shared_state)
        .layer(cors)
}

pub async fn start_server(shared_state: Arc<ApiState>) {
    // Spawn simulation ticker in background
    spawn_simulation_loop(shared_state.clone());

    let app = create_app(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8085));
    println!("[Phantom API] Starting Axum server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn sse_handler(
    State(state): State<Arc<ApiState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let interval = tokio::time::interval(Duration::from_millis(100));
    let stream = IntervalStream::new(interval).map(move |_| {
        let snap = state.snapshot.lock().unwrap().clone();
        let payload = json!({
            "version": snap.version,
            "timestamp_ms": snap.timestamp_ms,
            "nodes": snap.nodes,
            "metrics": {
                "throughput_req_sec": snap.throughput_req_sec,
                "p50_latency_ms": snap.p50_latency_ms,
                "p95_latency_ms": snap.p95_latency_ms,
                "p99_latency_ms": snap.p99_latency_ms,
                "backend_load_ratio": snap.db_load_ratio,
                "avoided_compute_dollars": snap.avoided_compute_dollars
            },
            "hot_keys": [
                { "key": "/availability/hotel/123", "req_sec": 9200.0, "hit_rate": 0.99 }
            ]
        });
        Ok(Event::default().json_data(payload).unwrap())
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn control_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
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

    Json(json!({ "status": "success", "triggered": event }))
}

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

            // Generate observed latency sample distribution
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

            let mut snap_guard = state.snapshot.lock().unwrap();
            snap_guard.timestamp_ms = clock;
            snap_guard.throughput_req_sec = throughput;
            snap_guard.p50_latency_ms = latency_p50;
            snap_guard.p95_latency_ms = latency_p95;
            snap_guard.p99_latency_ms = latency_p99;
            snap_guard.db_load_ratio = db_load;
            snap_guard.avoided_compute_dollars = avoided_dollars;

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
