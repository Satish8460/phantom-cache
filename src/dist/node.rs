use crate::core::cache::PartitionedCache;
use crate::core::coalesce::Coalescer;
use crate::intel::telemetry::TelemetryCollector;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Joining,
    Active,
    Degraded,
    Suspect,
    Failed,
    Offline,
    Recovering,
}

pub struct VirtualNode {
    pub node_id: u8,
    pub state: Mutex<NodeState>,
    pub cache: Arc<PartitionedCache>,
    pub coalescer: Arc<Coalescer>,
    pub telemetry: Arc<TelemetryCollector>,
    pub cpu_utilization: Mutex<f32>,
    pub memory_used_bytes: Mutex<u64>,
}

impl VirtualNode {
    pub fn new(node_id: u8) -> Self {
        Self {
            node_id,
            state: Mutex::new(NodeState::Joining),
            cache: Arc::new(PartitionedCache::new(4)),
            coalescer: Arc::new(Coalescer::new(1000)),
            telemetry: Arc::new(TelemetryCollector::new(1000)),
            cpu_utilization: Mutex::new(0.0),
            memory_used_bytes: Mutex::new(0),
        }
    }

    pub fn set_state(&self, new_state: NodeState) {
        let mut guard = self.state.lock().unwrap();
        *guard = new_state;
    }

    pub fn get_state(&self) -> NodeState {
        *self.state.lock().unwrap()
    }
}
