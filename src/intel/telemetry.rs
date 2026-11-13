use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObservationKind {
    #[default]
    Get,
    Delete,
}

/// FNV-1a 64-bit hash over a string slice (A-5), computed outside telemetry locks.
pub fn fnv1a_64(s: &str) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for byte in s.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Debug, Clone)]
pub struct RawObservation {
    pub timestamp_ms: u64,
    pub key_hash: u64,
    pub latency_ms: u32,
    pub hit: bool,
    pub waiters_coalesced: u32,
    pub kind: ObservationKind,
}

#[derive(Debug, Clone)]
pub struct RawObservationUs {
    pub timestamp_ms: u64,
    pub key_hash: u64,
    pub latency_us: u64,
    pub hit: bool,
    pub waiters_coalesced: u32,
    pub kind: ObservationKind,
}

impl RawObservation {
    pub fn new(
        timestamp_ms: u64,
        key_hash: u64,
        latency_ms: u32,
        hit: bool,
        waiters_coalesced: u32,
        kind: ObservationKind,
    ) -> Self {
        Self {
            timestamp_ms,
            key_hash,
            latency_ms,
            hit,
            waiters_coalesced,
            kind,
        }
    }

    pub fn new_get(
        timestamp_ms: u64,
        key_hash: u64,
        latency_ms: u32,
        hit: bool,
        waiters_coalesced: u32,
    ) -> Self {
        Self::new(
            timestamp_ms,
            key_hash,
            latency_ms,
            hit,
            waiters_coalesced,
            ObservationKind::Get,
        )
    }

    pub fn new_delete(timestamp_ms: u64, key_hash: u64, latency_ms: u32) -> Self {
        Self::new(
            timestamp_ms,
            key_hash,
            latency_ms,
            false,
            0,
            ObservationKind::Delete,
        )
    }
}

impl RawObservationUs {
    pub fn new(
        timestamp_ms: u64,
        key_hash: u64,
        latency_us: u64,
        hit: bool,
        waiters_coalesced: u32,
        kind: ObservationKind,
    ) -> Self {
        Self {
            timestamp_ms,
            key_hash,
            latency_us,
            hit,
            waiters_coalesced,
            kind,
        }
    }

    pub fn new_get(
        timestamp_ms: u64,
        key_hash: u64,
        latency_us: u64,
        hit: bool,
        waiters_coalesced: u32,
    ) -> Self {
        Self::new(
            timestamp_ms,
            key_hash,
            latency_us,
            hit,
            waiters_coalesced,
            ObservationKind::Get,
        )
    }

    pub fn new_delete(timestamp_ms: u64, key_hash: u64, latency_us: u64) -> Self {
        Self::new(
            timestamp_ms,
            key_hash,
            latency_us,
            false,
            0,
            ObservationKind::Delete,
        )
    }
}

pub struct WindowedRateSampler {
    snapshots: Mutex<VecDeque<(Instant, u64, u64, u64)>>,
    start_time: Instant,
}

impl Default for WindowedRateSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowedRateSampler {
    pub fn new() -> Self {
        let now = Instant::now();
        let mut q = VecDeque::with_capacity(20);
        q.push_back((now, 0, 0, 0));
        Self {
            snapshots: Mutex::new(q),
            start_time: now,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn sample(&self, requests: u64, hits: u64, misses: u64) {
        let now = Instant::now();
        let mut guard = self.snapshots.lock().unwrap();
        guard.push_back((now, requests, hits, misses));
        while guard.len() > 15 {
            guard.pop_front();
        }
    }

    pub fn sample_if_due(&self, requests: u64, hits: u64, misses: u64) {
        let now = Instant::now();
        let mut guard = self.snapshots.lock().unwrap();
        if let Some(last) = guard.back() {
            if now.duration_since(last.0) < Duration::from_millis(800) {
                return;
            }
        }
        guard.push_back((now, requests, hits, misses));
        while guard.len() > 15 {
            guard.pop_front();
        }
    }

    pub fn throughput_1s(&self) -> f64 {
        let guard = self.snapshots.lock().unwrap();
        if guard.len() < 2 {
            return 0.0;
        }
        let newest = guard.back().unwrap();
        let mut prev = &guard[0];
        for snap in guard.iter().rev().skip(1) {
            let dt = newest.0.duration_since(snap.0).as_secs_f64();
            if dt >= 0.8 {
                prev = snap;
                break;
            }
        }
        let dt = newest.0.duration_since(prev.0).as_secs_f64();
        if dt > 0.0 {
            (newest.1.saturating_sub(prev.1) as f64) / dt
        } else {
            0.0
        }
    }

    pub fn throughput_10s(&self) -> f64 {
        let guard = self.snapshots.lock().unwrap();
        if guard.len() < 2 {
            return 0.0;
        }
        let newest = guard.back().unwrap();
        let oldest = guard.front().unwrap();
        let dt = newest.0.duration_since(oldest.0).as_secs_f64();
        if dt > 0.0 {
            (newest.1.saturating_sub(oldest.1) as f64) / dt
        } else {
            0.0
        }
    }

    pub fn hit_ratio_1s(&self) -> Option<f64> {
        let guard = self.snapshots.lock().unwrap();
        if guard.len() < 2 {
            return None;
        }
        let newest = guard.back().unwrap();
        let mut prev = &guard[0];
        for snap in guard.iter().rev().skip(1) {
            let dt = newest.0.duration_since(snap.0).as_secs_f64();
            if dt >= 0.8 {
                prev = snap;
                break;
            }
        }
        let dh = newest.2.saturating_sub(prev.2);
        let dm = newest.3.saturating_sub(prev.3);
        let total = dh + dm;
        if total > 0 {
            Some(dh as f64 / total as f64)
        } else {
            None
        }
    }

    pub fn hit_ratio_10s(&self) -> Option<f64> {
        let guard = self.snapshots.lock().unwrap();
        if guard.len() < 3 {
            return None;
        }
        let newest = guard.back().unwrap();
        let oldest = guard.front().unwrap();
        let dh = newest.2.saturating_sub(oldest.2);
        let dm = newest.3.saturating_sub(oldest.3);
        let total = dh + dm;
        if total > 0 {
            Some(dh as f64 / total as f64)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Atomic Log-Linear Histogram (M2-08 / WP-09)
// Documented relative error <= 0.5% (strict requirement <= 1%)
// ---------------------------------------------------------------------------

pub const HISTOGRAM_EXACT_BUCKETS: usize = 128;
pub const HISTOGRAM_SUB_BUCKETS_PER_OCTAVE: usize = 128;
pub const HISTOGRAM_OCTAVES: usize = 20; // 2^7 to 2^27 (128 us to ~67.1 seconds)
pub const HISTOGRAM_TOTAL_BUCKETS: usize =
    HISTOGRAM_EXACT_BUCKETS + HISTOGRAM_OCTAVES * HISTOGRAM_SUB_BUCKETS_PER_OCTAVE; // 2,688 buckets

pub struct AtomicLogLinearHistogram {
    buckets: Vec<AtomicU64>,
    total_count: AtomicU64,
}

impl Default for AtomicLogLinearHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicLogLinearHistogram {
    pub fn new() -> Self {
        let mut buckets = Vec::with_capacity(HISTOGRAM_TOTAL_BUCKETS);
        for _ in 0..HISTOGRAM_TOTAL_BUCKETS {
            buckets.push(AtomicU64::new(0));
        }
        Self {
            buckets,
            total_count: AtomicU64::new(0),
        }
    }

    /// Maps a latency in microseconds to its log-linear bucket index in O(1) time (single LZCNT instruction).
    pub fn value_to_bucket(val_us: u64) -> usize {
        if val_us < HISTOGRAM_EXACT_BUCKETS as u64 {
            return val_us as usize;
        }
        let leading_zeros = val_us.leading_zeros();
        let msb = 63 - leading_zeros; // msb >= 7
        let octave = ((msb - 7) as usize).min(HISTOGRAM_OCTAVES - 1);
        let k = octave as u32 + 7;
        let shift = octave as u32;
        let base = 1u64 << k;
        let offset = val_us.saturating_sub(base);
        let sub = ((offset >> shift) as usize).min(HISTOGRAM_SUB_BUCKETS_PER_OCTAVE - 1);
        HISTOGRAM_EXACT_BUCKETS + octave * HISTOGRAM_SUB_BUCKETS_PER_OCTAVE + sub
    }

    /// Returns the representative midpoint value in microseconds for a given bucket index.
    /// Max relative error vs any value mapped to this bucket is strictly <= 1 / 256 = 0.39%.
    pub fn bucket_to_value(bucket: usize) -> u64 {
        if bucket < HISTOGRAM_EXACT_BUCKETS {
            return bucket as u64;
        }
        let rel = (bucket - HISTOGRAM_EXACT_BUCKETS)
            .min(HISTOGRAM_OCTAVES * HISTOGRAM_SUB_BUCKETS_PER_OCTAVE - 1);
        let octave = rel / HISTOGRAM_SUB_BUCKETS_PER_OCTAVE;
        let sub = rel % HISTOGRAM_SUB_BUCKETS_PER_OCTAVE;
        let k = octave as u32 + 7;
        let base = 1u64 << k;
        let step = 1u64 << (k - 7);
        base + (sub as u64) * step + (step >> 1)
    }

    pub fn record(&self, latency_us: u64) {
        let b = Self::value_to_bucket(latency_us);
        if b < self.buckets.len() {
            self.buckets[b].fetch_add(1, Ordering::Relaxed);
            self.total_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }

    pub fn clear(&self) {
        for b in &self.buckets {
            b.store(0, Ordering::Relaxed);
        }
        self.total_count.store(0, Ordering::Relaxed);
    }

    pub fn calculate_percentiles_us(&self) -> (u64, u64, u64, u64) {
        let total = self.total_count.load(Ordering::Relaxed);
        if total == 0 {
            return (0, 0, 0, 0);
        }

        let target_p50 = ((total as f64) * 0.50).ceil() as u64;
        let target_p90 = ((total as f64) * 0.90).ceil() as u64;
        let target_p95 = ((total as f64) * 0.95).ceil() as u64;
        let target_p99 = ((total as f64) * 0.99).ceil() as u64;

        let mut cum = 0u64;
        let mut p50: Option<u64> = None;
        let mut p90: Option<u64> = None;
        let mut p95: Option<u64> = None;
        let mut p99: Option<u64> = None;

        for (idx, b) in self.buckets.iter().enumerate() {
            let c = b.load(Ordering::Relaxed);
            if c == 0 {
                continue;
            }
            cum += c;
            let val = Self::bucket_to_value(idx);

            if p50.is_none() && cum >= target_p50 {
                p50 = Some(val);
            }
            if p90.is_none() && cum >= target_p90 {
                p90 = Some(val);
            }
            if p95.is_none() && cum >= target_p95 {
                p95 = Some(val);
            }
            if p99.is_none() && cum >= target_p99 {
                p99 = Some(val);
                break;
            }
        }

        let p50_val = p50.unwrap_or(0);
        let p90_val = p90.unwrap_or(p50_val);
        let p95_val = p95.unwrap_or(p90_val);
        let p99_val = p99.unwrap_or(p95_val);

        (p50_val, p90_val, p95_val, p99_val)
    }

    pub fn calculate_percentiles_f64_ms(&self) -> (f64, f64, f64, f64) {
        let (p50, p90, p95, p99) = self.calculate_percentiles_us();
        (
            (p50 as f64) / 1000.0,
            (p90 as f64) / 1000.0,
            (p95 as f64) / 1000.0,
            (p99 as f64) / 1000.0,
        )
    }
}

pub struct TelemetryCollector {
    raw_buffer: Mutex<VecDeque<RawObservation>>,
    raw_buffer_us: Mutex<VecDeque<RawObservationUs>>,
    pub histogram: Arc<AtomicLogLinearHistogram>,
    capacity: usize,
    error_count: AtomicU64,
    pub origin_metrics: Arc<crate::intel::origin_metrics::OriginMetrics>,
    pub rate_sampler: Arc<WindowedRateSampler>,
}

impl TelemetryCollector {
    pub fn new(capacity: usize) -> Self {
        Self {
            raw_buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            raw_buffer_us: Mutex::new(VecDeque::with_capacity(capacity)),
            histogram: Arc::new(AtomicLogLinearHistogram::new()),
            capacity,
            error_count: AtomicU64::new(0),
            origin_metrics: Arc::new(crate::intel::origin_metrics::OriginMetrics::new()),
            rate_sampler: Arc::new(WindowedRateSampler::new()),
        }
    }

    pub fn origin_metrics(&self) -> &Arc<crate::intel::origin_metrics::OriginMetrics> {
        &self.origin_metrics
    }

    pub fn histogram(&self) -> &Arc<AtomicLogLinearHistogram> {
        &self.histogram
    }

    pub fn record(&self, obs: RawObservation) {
        let us_obs = RawObservationUs {
            timestamp_ms: obs.timestamp_ms,
            key_hash: obs.key_hash,
            latency_us: (obs.latency_ms as u64) * 1000,
            hit: obs.hit,
            waiters_coalesced: obs.waiters_coalesced,
            kind: obs.kind,
        };
        if obs.kind == ObservationKind::Get {
            self.histogram.record(us_obs.latency_us);
        }
        {
            let mut guard = self.raw_buffer.lock().unwrap();
            if guard.len() >= self.capacity {
                guard.pop_front();
            }
            guard.push_back(obs);
        }
        {
            let mut guard_us = self.raw_buffer_us.lock().unwrap();
            if guard_us.len() >= self.capacity {
                guard_us.pop_front();
            }
            guard_us.push_back(us_obs);
        }
    }

    pub fn record_us(&self, obs: RawObservationUs) {
        if obs.kind == ObservationKind::Get {
            self.histogram.record(obs.latency_us);
        }
        let ms_obs = RawObservation {
            timestamp_ms: obs.timestamp_ms,
            key_hash: obs.key_hash,
            latency_ms: (obs.latency_us / 1000) as u32,
            hit: obs.hit,
            waiters_coalesced: obs.waiters_coalesced,
            kind: obs.kind,
        };
        {
            let mut guard = self.raw_buffer.lock().unwrap();
            if guard.len() >= self.capacity {
                guard.pop_front();
            }
            guard.push_back(ms_obs);
        }
        {
            let mut guard_us = self.raw_buffer_us.lock().unwrap();
            if guard_us.len() >= self.capacity {
                guard_us.pop_front();
            }
            guard_us.push_back(obs);
        }
    }

    pub fn calculate_percentiles_us(&self) -> (u64, u64, u64, u64) {
        let guard = self.raw_buffer_us.lock().unwrap();
        let mut latencies: Vec<u64> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_us)
            .collect();
        if latencies.is_empty() {
            return (0, 0, 0, 0);
        }
        latencies.sort_unstable();
        let n = latencies.len();
        let p50 = latencies[(n as f64 * 0.50) as usize % n];
        let p90 = latencies[((n as f64 * 0.90) as usize).min(n - 1)];
        let p95 = latencies[((n as f64 * 0.95) as usize).min(n - 1)];
        let p99 = latencies[((n as f64 * 0.99) as usize).min(n - 1)];
        (p50, p90, p95, p99)
    }

    pub fn calculate_percentiles_f64_ms(&self) -> (f64, f64, f64, f64) {
        let (p50, p90, p95, p99) = self.calculate_percentiles_us();
        (
            (p50 as f64) / 1000.0,
            (p90 as f64) / 1000.0,
            (p95 as f64) / 1000.0,
            (p99 as f64) / 1000.0,
        )
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Read-only snapshot of recorded observations for verification and testing (A-11).
    pub fn observations(&self) -> Vec<RawObservation> {
        self.raw_buffer.lock().unwrap().iter().cloned().collect()
    }

    /// Read-only snapshot of recorded microsecond observations for verification and testing.
    pub fn observations_us(&self) -> Vec<RawObservationUs> {
        self.raw_buffer_us.lock().unwrap().iter().cloned().collect()
    }

    pub fn calculate_p50(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.50) as usize;
        latencies[idx.min(latencies.len() - 1)]
    }

    pub fn calculate_p90(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.90) as usize;
        latencies[idx.min(latencies.len() - 1)]
    }

    pub fn calculate_p95(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.95) as usize;
        latencies[idx.min(latencies.len() - 1)]
    }

    pub fn calculate_p99(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return 0;
        }
        latencies.sort_unstable();
        let idx = (latencies.len() as f64 * 0.99) as usize;
        latencies[idx.min(latencies.len() - 1)]
    }

    /// Returns (p50, p95, p99) over GET observations with real p95 (A-3).
    pub fn calculate_percentiles(&self) -> (u32, u32, u32) {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return (0, 0, 0);
        }
        latencies.sort_unstable();
        let n = latencies.len();
        let p50 = latencies[(n as f64 * 0.50) as usize % n];
        let p95 = latencies[((n as f64 * 0.95) as usize).min(n - 1)];
        let p99 = latencies[((n as f64 * 0.99) as usize).min(n - 1)];
        (p50, p95, p99)
    }

    /// Returns (p50, p90, p95, p99) over GET observations.
    pub fn calculate_percentiles_v2(&self) -> (u32, u32, u32, u32) {
        let guard = self.raw_buffer.lock().unwrap();
        let mut latencies: Vec<u32> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.latency_ms)
            .collect();
        if latencies.is_empty() {
            return (0, 0, 0, 0);
        }
        latencies.sort_unstable();
        let n = latencies.len();
        let p50 = latencies[(n as f64 * 0.50) as usize % n];
        let p90 = latencies[((n as f64 * 0.90) as usize).min(n - 1)];
        let p95 = latencies[((n as f64 * 0.95) as usize).min(n - 1)];
        let p99 = latencies[((n as f64 * 0.99) as usize).min(n - 1)];
        (p50, p90, p95, p99)
    }

    /// Excludes DELETE observations from hit-rate calculation (A-3).
    pub fn calculate_hit_rate(&self) -> f32 {
        let guard = self.raw_buffer.lock().unwrap();
        let get_obs: Vec<_> = guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .collect();
        if get_obs.is_empty() {
            return 1.0;
        }
        let hits = get_obs.iter().filter(|o| o.hit).count();
        hits as f32 / get_obs.len() as f32
    }

    pub fn calculate_coalescing_savings(&self) -> u32 {
        let guard = self.raw_buffer.lock().unwrap();
        guard
            .iter()
            .filter(|o| o.kind == ObservationKind::Get)
            .map(|o| o.waiters_coalesced)
            .sum()
    }

    pub fn len(&self) -> usize {
        self.raw_buffer.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.raw_buffer.lock().unwrap().is_empty()
    }

    pub fn calculate_percentiles_histogram_us(&self) -> (u64, u64, u64, u64) {
        self.histogram.calculate_percentiles_us()
    }

    pub fn calculate_percentiles_histogram_f64_ms(&self) -> (f64, f64, f64, f64) {
        self.histogram.calculate_percentiles_f64_ms()
    }

    pub fn clear(&self) {
        self.raw_buffer.lock().unwrap().clear();
        self.raw_buffer_us.lock().unwrap().clear();
        self.histogram.clear();
        self.error_count.store(0, Ordering::Relaxed);
    }
}
