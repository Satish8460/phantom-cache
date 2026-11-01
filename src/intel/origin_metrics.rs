use crate::core::coalesce::OriginError;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Fixed bucket boundaries in milliseconds for origin fetch latency.
/// 15 thresholds => 16 buckets.
pub const LATENCY_BUCKETS_MS: [u64; 15] = [
    1, 2, 5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000, 30_000, 60_000,
];

pub struct OriginLatencyHistogram {
    buckets: [AtomicU64; 16],
    total_samples: AtomicU64,
}

impl Default for OriginLatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginLatencyHistogram {
    pub fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            total_samples: AtomicU64::new(0),
        }
    }

    pub fn record(&self, latency_ms: u64) {
        let mut idx = 15;
        for (i, &bound) in LATENCY_BUCKETS_MS.iter().enumerate() {
            if latency_ms <= bound {
                idx = i;
                break;
            }
        }
        self.buckets[idx].fetch_add(1, Ordering::Relaxed);
        self.total_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn calculate_percentiles(&self) -> Option<(u64, u64, u64)> {
        let total = self.total_samples.load(Ordering::Relaxed);
        if total == 0 {
            return None;
        }

        let mut counts = [0u64; 16];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = self.buckets[i].load(Ordering::Relaxed);
        }

        let p50_target = ((total as f64) * 0.50).ceil() as u64;
        let p95_target = ((total as f64) * 0.95).ceil() as u64;
        let p99_target = ((total as f64) * 0.99).ceil() as u64;

        let find_quantile = |target: u64| -> u64 {
            let mut accum = 0;
            for i in 0..16 {
                accum += counts[i];
                if accum >= target {
                    return if i < 15 {
                        LATENCY_BUCKETS_MS[i]
                    } else {
                        120_000 // Upper bound for overflow bucket
                    };
                }
            }
            60_000
        };

        Some((
            find_quantile(p50_target),
            find_quantile(p95_target),
            find_quantile(p99_target),
        ))
    }
}

#[derive(Default)]
pub struct OriginMetrics {
    pub requests_started: AtomicU64,
    pub requests_completed_ok: AtomicU64,
    pub in_flight: AtomicU64,

    pub errors_timeout: AtomicU64,
    pub errors_network: AtomicU64,
    pub errors_status_5xx: AtomicU64,
    pub errors_other: AtomicU64,

    pub histogram: OriginLatencyHistogram,
}

impl OriginMetrics {
    pub fn new() -> Self {
        Self {
            requests_started: AtomicU64::new(0),
            requests_completed_ok: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            errors_timeout: AtomicU64::new(0),
            errors_network: AtomicU64::new(0),
            errors_status_5xx: AtomicU64::new(0),
            errors_other: AtomicU64::new(0),
            histogram: OriginLatencyHistogram::new(),
        }
    }

    /// Starts an origin fetch, incrementing `in_flight` and `requests_started`,
    /// and returning an RAII guard that decrements `in_flight` on drop.
    pub fn start_request(self: &Arc<Self>) -> OriginInFlightGuard {
        self.requests_started.fetch_add(1, Ordering::Relaxed);
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        OriginInFlightGuard {
            metrics: self.clone(),
            start: Instant::now(),
            completed: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> OriginMetricsSnapshot {
        let percentiles = self.histogram.calculate_percentiles();
        OriginMetricsSnapshot {
            requests_started: self.requests_started.load(Ordering::Relaxed),
            requests_completed_ok: self.requests_completed_ok.load(Ordering::Relaxed),
            in_flight: self.in_flight.load(Ordering::Relaxed),
            errors_timeout: self.errors_timeout.load(Ordering::Relaxed),
            errors_network: self.errors_network.load(Ordering::Relaxed),
            errors_status_5xx: self.errors_status_5xx.load(Ordering::Relaxed),
            errors_other: self.errors_other.load(Ordering::Relaxed),
            latency_p50: percentiles.map(|p| p.0),
            latency_p95: percentiles.map(|p| p.1),
            latency_p99: percentiles.map(|p| p.2),
        }
    }
}

pub struct OriginInFlightGuard {
    metrics: Arc<OriginMetrics>,
    start: Instant,
    completed: AtomicBool,
}

impl OriginInFlightGuard {
    pub fn complete_ok(self) {
        if !self.completed.swap(true, Ordering::SeqCst) {
            let elapsed_ms = self.start.elapsed().as_millis() as u64;
            self.metrics.requests_completed_ok.fetch_add(1, Ordering::Relaxed);
            self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.metrics.histogram.record(elapsed_ms);
        }
    }

    pub fn complete_err(self, err: &OriginError) {
        if !self.completed.swap(true, Ordering::SeqCst) {
            let elapsed_ms = self.start.elapsed().as_millis() as u64;
            self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.metrics.histogram.record(elapsed_ms);

            match err {
                OriginError::Timeout => {
                    self.metrics.errors_timeout.fetch_add(1, Ordering::Relaxed);
                }
                OriginError::NetworkError(_) => {
                    self.metrics.errors_network.fetch_add(1, Ordering::Relaxed);
                }
                OriginError::UpstreamStatus(code) if *code >= 500 && *code < 600 => {
                    self.metrics.errors_status_5xx.fetch_add(1, Ordering::Relaxed);
                }
                OriginError::SecurityError(sec) => {
                    if matches!(sec, crate::security::upstream::OriginSecurityError::Timeout) {
                        self.metrics.errors_timeout.fetch_add(1, Ordering::Relaxed);
                    } else {
                        self.metrics.errors_other.fetch_add(1, Ordering::Relaxed);
                    }
                }
                _ => {
                    self.metrics.errors_other.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

impl Drop for OriginInFlightGuard {
    fn drop(&mut self) {
        if !self.completed.swap(true, Ordering::SeqCst) {
            // Unhandled drop: task canceled, aborted, or panicked
            let elapsed_ms = self.start.elapsed().as_millis() as u64;
            self.metrics.in_flight.fetch_sub(1, Ordering::Relaxed);
            self.metrics.errors_other.fetch_add(1, Ordering::Relaxed);
            self.metrics.histogram.record(elapsed_ms);
        }
    }
}

#[derive(Debug, Clone)]
pub struct OriginMetricsSnapshot {
    pub requests_started: u64,
    pub requests_completed_ok: u64,
    pub in_flight: u64,
    pub errors_timeout: u64,
    pub errors_network: u64,
    pub errors_status_5xx: u64,
    pub errors_other: u64,
    pub latency_p50: Option<u64>,
    pub latency_p95: Option<u64>,
    pub latency_p99: Option<u64>,
}

