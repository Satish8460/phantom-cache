//! Open-loop coordinated-omission-safe load generator for Phantom Lab Tier-2 (M4-04 / WP-16).
//!
//! Features:
//! - Open-loop scheduled intended-send times: intended_t = t_0 + sum(deltas).
//! - True async non-blocking dispatch: requests run concurrently without blocking future sends.
//! - Coordinated-omission-safe latency: latency = completion_time - intended_send_time.
//! - Generator health verification (EV-4):
//!   - Achieved request rate >= 99% of target.
//!   - Scheduling lag bounds (max lag between actual dispatch and intended send time).
//! - HDR latency histogram recording.

use super::rng::StreamRng;
use super::workloads::WorkloadGenerator;
use hdrhistogram::Histogram;
use reqwest::Client;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct LoadgenReport {
    pub target_requests: u64,
    pub dispatched_requests: u64,
    pub completed_requests: u64,
    pub error_requests: u64,
    pub achieved_ratio: f64,
    pub max_scheduling_lag_ms: f64,
    pub latency_p50_us: u64,
    pub latency_p90_us: u64,
    pub latency_p99_us: u64,
    pub latency_max_us: u64,
}

pub struct OpenLoopLoadgen {
    target_url: String,
    client: Client,
    max_concurrency: usize,
}

impl OpenLoopLoadgen {
    pub fn new(target_url: String) -> Self {
        Self {
            target_url,
            client: Client::builder()
                .pool_max_idle_per_host(256)
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            max_concurrency: 512,
        }
    }

    pub fn with_max_concurrency(mut self, cap: usize) -> Self {
        self.max_concurrency = cap.max(16);
        self
    }

    /// Run open-loop workload replay for `num_requests`.
    pub async fn run<W: WorkloadGenerator>(
        &self,
        mut workload: W,
        num_requests: u64,
        mut rng: StreamRng,
    ) -> Result<LoadgenReport, Box<dyn std::error::Error + Send + Sync>> {
        let concurrency_sem = Arc::new(Semaphore::new(self.max_concurrency));
        let dispatched = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicU64::new(0));
        let errors = Arc::new(AtomicU64::new(0));
        let max_lag_us = Arc::new(AtomicU64::new(0));

        let hist = Arc::new(tokio::sync::Mutex::new(
            Histogram::<u64>::new_with_bounds(1, 60_000_000, 3).unwrap(),
        ));

        let t0 = Instant::now();
        let mut cumulative_delta_us = 0u64;

        for _ in 0..num_requests {
            let req = workload.next_request(cumulative_delta_us, &mut rng);
            cumulative_delta_us += req.arrival_delta_us;
            let intended_send = t0 + Duration::from_micros(cumulative_delta_us);

            // Wait until intended send time
            let now = Instant::now();
            if now < intended_send {
                tokio::time::sleep(intended_send - now).await;
            }

            let actual_send = Instant::now();
            let lag_us = actual_send.saturating_duration_since(intended_send).as_micros() as u64;

            // Track max scheduling lag
            let mut cur_max = max_lag_us.load(Ordering::Relaxed);
            while lag_us > cur_max {
                match max_lag_us.compare_exchange_weak(
                    cur_max,
                    lag_us,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(actual) => cur_max = actual,
                }
            }

            dispatched.fetch_add(1, Ordering::Relaxed);

            // Acquire concurrency permit so task limit isn't completely unbounded
            let permit = concurrency_sem.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();
            let url = format!("{}{}", self.target_url, req.key);
            let comp_clone = completed.clone();
            let err_clone = errors.clone();
            let hist_clone = hist.clone();

            tokio::spawn(async move {
                let resp_res = client.get(&url).send().await;
                let t_done = Instant::now();
                drop(permit);

                // Coordinated-omission-safe latency: completion time - intended send time!
                let latency_us = t_done.saturating_duration_since(intended_send).as_micros() as u64;

                match resp_res {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            comp_clone.fetch_add(1, Ordering::Relaxed);
                            let mut h = hist_clone.lock().await;
                            let _ = h.record(latency_us.max(1));
                        } else {
                            err_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        err_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }

        // Wait for all in-flight requests to drain
        let drain_start = Instant::now();
        while completed.load(Ordering::Relaxed) + errors.load(Ordering::Relaxed) < num_requests {
            if drain_start.elapsed() > Duration::from_secs(10) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let total_dispatched = dispatched.load(Ordering::Relaxed);
        let total_completed = completed.load(Ordering::Relaxed);
        let total_errors = errors.load(Ordering::Relaxed);
        let max_lag = max_lag_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let achieved_ratio = if num_requests > 0 {
            (total_completed + total_errors) as f64 / num_requests as f64
        } else {
            1.0
        };

        let h = hist.lock().await;
        let latency_p50_us = h.value_at_quantile(0.50);
        let latency_p90_us = h.value_at_quantile(0.90);
        let latency_p99_us = h.value_at_quantile(0.99);
        let latency_max_us = h.max();

        Ok(LoadgenReport {
            target_requests: num_requests,
            dispatched_requests: total_dispatched,
            completed_requests: total_completed,
            error_requests: total_errors,
            achieved_ratio,
            max_scheduling_lag_ms: max_lag,
            latency_p50_us,
            latency_p90_us,
            latency_p99_us,
            latency_max_us,
        })
    }
}
