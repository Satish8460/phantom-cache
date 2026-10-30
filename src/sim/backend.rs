use std::sync::atomic::{AtomicU32, Ordering};

pub struct BackendDatabase {
    base_latency_ms: u32,
    saturation_threshold: u32,
    active_queries: AtomicU32,
}

impl BackendDatabase {
    pub fn new(base_latency_ms: u32, saturation_threshold: u32) -> Self {
        Self {
            base_latency_ms,
            saturation_threshold,
            active_queries: AtomicU32::new(0),
        }
    }

    pub fn enter_query(&self) -> u32 {
        self.active_queries.fetch_add(1, Ordering::SeqCst)
    }

    pub fn exit_query(&self) {
        self.active_queries.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn process_query_ms(&self) -> u32 {
        let active = self.active_queries.load(Ordering::SeqCst);
        if active <= self.saturation_threshold {
            self.base_latency_ms
        } else {
            // Exponential latency degradation under saturation
            let overflow = active - self.saturation_threshold;
            self.base_latency_ms + (overflow as f64).powf(1.8) as u32
        }
    }

    pub fn get_cpu_utilization(&self) -> f32 {
        let active = self.active_queries.load(Ordering::SeqCst) as f32;
        let limit = self.saturation_threshold as f32;
        if limit == 0.0 {
            return 100.0;
        }
        let util = (active / limit) * 100.0;
        if util > 100.0 {
            100.0
        } else {
            util
        }
    }
}
