pub enum SimEvent {
    FlashCrowd { target_rate: f64 },
    NetworkDegradation { packet_loss_ratio: f32 },
    NodeFailure { node_id: u8 },
    BackendSlowdown { capacity_reduction_ratio: f32 },
    WorkloadShift { skew: f64 },
}

pub struct ScenarioOrchestrator {
    pub events: Vec<(u64, SimEvent)>,
}

impl Default for ScenarioOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioOrchestrator {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn add_event(&mut self, time_ms: u64, event: SimEvent) {
        self.events.push((time_ms, event));
        // Sort by trigger time
        self.events.sort_by_key(|e| e.0);
    }
}
