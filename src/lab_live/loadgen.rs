//! Open-loop load generator (no coordinated omission): arrivals are fixed by the seed before the run
//! and are sent at their scheduled times regardless of earlier responses. Latency is reported from
//! the SCHEDULED send time (and, separately, from the actual send time).

use super::http::Pool;
use crate::lab::rng::{AliasSampler, MasterRng};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrivalProcess {
    Poisson,
    Regular,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    pub seed: u64,
    pub n_keys: usize,
    pub zipf_alpha: f64,
    pub rate_rps: f64,
    pub duration_ms: u64,
    pub arrival: ArrivalProcess,
    /// Request path = `path_prefix` + key index.
    pub path_prefix: String,
    pub client_deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub idx: u32,
    pub t_ns: u64,
    pub key: u32,
}

/// Deterministic schedule: depends only on the workload (seed, rates, key space), never on the arm.
pub fn schedule(w: &Workload) -> Vec<Arrival> {
    let m = MasterRng::new(w.seed);
    let mut arr_rng = m.stream("arrivals");
    let mut key_rng = m.stream("keys");
    let sampler = AliasSampler::new_zipf(w.n_keys.max(1), w.zipf_alpha);
    let horizon_ns = w.duration_ms * 1_000_000;
    let mut out = Vec::new();
    let mut t_s = 0.0f64;
    let mut i = 0u32;
    loop {
        let gap = match w.arrival {
            ArrivalProcess::Poisson => arr_rng.next_exponential(w.rate_rps),
            ArrivalProcess::Regular => 1.0 / w.rate_rps,
        };
        t_s += gap;
        let t_ns = (t_s * 1e9) as u64;
        if t_ns >= horizon_ns {
            break;
        }
        out.push(Arrival { idx: i, t_ns, key: sampler.sample(&mut key_rng) as u32 });
        i += 1;
    }
    out
}

#[derive(Debug, Clone)]
pub struct ClientRecord {
    pub idx: u32,
    pub key: u32,
    pub scheduled_ns: u64,
    /// 0 when the request was never written.
    pub send_ns: u64,
    pub done_ns: u64,
    /// 0 = no HTTP response.
    pub status: u16,
    pub cache_status: String,
    pub cache_error: String,
    pub bytes: usize,
    pub error: String,
}

impl ClientRecord {
    /// Latency from the scheduled time in ms (coordinated-omission-safe).
    pub fn latency_ms(&self) -> f64 {
        self.done_ns.saturating_sub(self.scheduled_ns) as f64 / 1e6
    }
    /// Latency from the actual send in ms (what a closed-loop tool would report).
    pub fn service_latency_ms(&self) -> f64 {
        if self.send_ns == 0 {
            return f64::NAN;
        }
        self.done_ns.saturating_sub(self.send_ns) as f64 / 1e6
    }
    /// Generator lag: actual minus scheduled send, ms.
    pub fn send_lag_ms(&self) -> f64 {
        if self.send_ns == 0 {
            return f64::NAN;
        }
        self.send_ns.saturating_sub(self.scheduled_ns) as f64 / 1e6
    }
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Runs the schedule against `target` starting at `epoch`. Returns all records (including those
/// still unfinished at the deadline, as errors). `cancel` stops sending; outstanding requests drain.
pub async fn run_open_loop(
    target: SocketAddr,
    arrivals: &[Arrival],
    w: &Workload,
    epoch: Instant,
    cancel: Arc<AtomicBool>,
) -> Vec<ClientRecord> {
    let pool = Arc::new(Pool::new(target));
    let deadline = Duration::from_millis(w.client_deadline_ms);
    let mut handles = Vec::with_capacity(arrivals.len());
    for a in arrivals {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let due = epoch + Duration::from_nanos(a.t_ns);
        if due > Instant::now() {
            tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
        }
        let pool = pool.clone();
        let path = format!("{}{}", w.path_prefix, a.key);
        let a = a.clone();
        handles.push(tokio::spawn(async move {
            let now = move || epoch.elapsed().as_nanos() as u64;
            let o = pool.get(&path, deadline, &now).await;
            let done_ns = epoch.elapsed().as_nanos() as u64;
            ClientRecord {
                idx: a.idx,
                key: a.key,
                scheduled_ns: a.t_ns,
                send_ns: o.send_ns,
                done_ns,
                status: o.status,
                cache_status: o.cache_status,
                cache_error: o.cache_error,
                bytes: o.bytes,
                error: o.error,
            }
        }));
    }
    if cancel.load(Ordering::Relaxed) {
        for h in &handles {
            h.abort();
        }
        return Vec::new();
    }
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(r) = h.await {
            out.push(r);
        }
    }
    out.sort_by_key(|r| r.idx);
    out
}

/// Nearest-rank percentile (p in 0..=100) of an UNSORTED slice; `None` for an empty slice (never 0).
pub fn percentile(values: &[f64], p: f64) -> Option<f64> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank = ((p / 100.0) * v.len() as f64).ceil().max(1.0) as usize;
    Some(v[rank.min(v.len()) - 1])
}
