use crate::sim::rng::RngSource;
use std::sync::{Arc, Mutex};

pub struct NetworkLink {
    base_latency_ms: u32,
    jitter_ms: u32,
    packet_loss_ratio: f32,
    congestion_factor: Mutex<f64>,
}

impl NetworkLink {
    pub fn new(base_latency_ms: u32, jitter_ms: u32, packet_loss_ratio: f32) -> Self {
        Self {
            base_latency_ms,
            jitter_ms,
            packet_loss_ratio,
            congestion_factor: Mutex::new(1.0),
        }
    }

    pub fn set_congestion(&self, factor: f64) {
        let mut guard = self.congestion_factor.lock().unwrap();
        *guard = factor;
    }

    pub fn calculate_delay_ms(
        &self,
        rng: &Arc<dyn RngSource>,
        active_connections: u32,
    ) -> Option<u32> {
        if rng.next_f64() < self.packet_loss_ratio as f64 {
            return None; // Packet drop
        }

        let c_factor = *self.congestion_factor.lock().unwrap();
        let queue_delay = (active_connections as f64 * c_factor) as u32;

        let jitter_offset = if self.jitter_ms > 0 {
            (rng.next_f64() * self.jitter_ms as f64) as u32
        } else {
            0
        };

        Some(self.base_latency_ms + queue_delay + jitter_offset)
    }
}
