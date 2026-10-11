pub mod sim {
    pub mod backend;
    pub mod clock;
    pub mod network;
    pub mod rng;
    pub mod scheduler;
    pub mod traffic;
}

pub mod core {
    pub mod cache;
    pub mod coalesce;
}

pub mod intel {
    pub mod sketch;
    pub mod telemetry;
    pub mod workload;
}

pub mod control {
    pub mod pie;
}

pub mod dist {
    pub mod node;
    pub mod router;
}

pub mod api {
    pub mod server;
}

use api::server::{ApiState, NodeSnapshot, SimulationSnapshot};
use core::cache::PartitionedCache;
use core::coalesce::Coalescer;
use dist::node::{NodeState, VirtualNode};
use sim::backend::BackendDatabase;
use sim::clock::{Clock, VirtualClock};
use sim::network::NetworkLink;
use sim::rng::SeededRngSource;
use sim::traffic::TrafficGenerator;

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    println!("PHANTOM CORE INITIALIZING...");

    let clock = Arc::new(VirtualClock::new(0));
    let rng = Arc::new(SeededRngSource::new(42));
    let _traffic = TrafficGenerator::new(rng.clone());

    let _network = Arc::new(NetworkLink::new(10, 2, 0.001));
    let _db = Arc::new(BackendDatabase::new(15, 100));

    let _cache = Arc::new(PartitionedCache::new(8));
    let _coalescer = Arc::new(Coalescer::new(1000));

    let nodes = vec![
        Arc::new(VirtualNode::new(1)),
        Arc::new(VirtualNode::new(2)),
        Arc::new(VirtualNode::new(3)),
    ];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }

    let initial_snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: clock.now_ms(),
        throughput_req_sec: 12000.0,
        p50_latency_ms: 10,
        p95_latency_ms: 12,
        p99_latency_ms: 15,
        db_load_ratio: 0.2,
        avoided_compute_dollars: 0.0,
        nodes: vec![
            NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
            NodeSnapshot {
                node_id: 2,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
            NodeSnapshot {
                node_id: 3,
                state: "Active".to_string(),
                cpu_utilization: 28.4,
                memory_used_bytes: 536870912,
            },
        ],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(initial_snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
    });

    println!("PHANTOM CORE V1.0 INITIALIZED (API Server Mode)");
    api::server::start_server(api_state).await;
}
