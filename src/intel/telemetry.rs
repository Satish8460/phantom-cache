use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RawObservation {
    pub timestamp_ms: u64,
    pub key_hash: u64,
    pub latency_ms: u32,
    pub hit: bool,
    pub waiters_coalesced: u32,
}

pub struct TelemetryCollector {
    raw_buffer: Mutex<VecDeque<RawObservation>>,
    capacity: usize,
}

impl TelemetryCollector {
    pub fn new(capacity: usize) -> Self {
        Self {
            raw_buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn record(&self, obs: RawObservation) {
        let mut guard = self.raw_buffer.lock().unwrap();
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(obs);
    }

    pub fn calculate_p99(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        if guard.is_empty() {
            return 0;
        }

        let mut latencies: Vec<u32> = guard.iter().map(|o| o.latency_ms).collect();
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.99) as usize;
        let idx = idx.min(latencies.len() - 1);
        latencies[idx]
    }

    pub fn calculate_hit_rate(&self) -> f32 {
        let guard = self.raw_buffer.lock().unwrap();
        if guard.is_empty() {
            return 1.0;
        }
        let hits = guard.iter().filter(|o| o.hit).count();
        hits as f32 / guard.len() as f32
    }

    pub fn calculate_coalescing_savings(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        guard.iter().map(|o| o.waiters_coalesced).sum()
    }

    pub fn len(&self) -> usize {
        self.raw_buffer.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.raw_buffer.lock().unwrap().is_empty()
    }

    pub fn clear(&self) {
        self.raw_buffer.lock().unwrap().clear();
    }
}
