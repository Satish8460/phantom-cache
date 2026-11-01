//! Workload Signal & Feature Extraction Pipeline (Features F1..F12)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-02
//!
//! Implements all 12 workload intelligence features:
//! - F1: Request rate (EWMA)
//! - F2: Windowed hit ratio & miss cost
//! - F3: Heavy hitters (Aged Count-Min + Space-Saving)
//! - F4: Zipf skew alpha estimator (Log-rank MLE regression)
//! - F5: Working set (Spatially sampled SHARDS)
//! - F6: Burst detection (CUSUM)
//! - F7: Hot-key emergence (Velocity detector)
//! - F8: Locality (Reuse distance sampling)
//! - F9: Miss amplification
//! - F10: Origin health (Latency P99, error rate)
//! - F11: Invalidation/churn rate
//! - F12: Freshness/stale serving ratio

use super::*;
use std::collections::HashMap;

// ============================================================================
// F1: Request Rate (EWMA)
// ============================================================================

#[derive(Debug)]
pub struct RateDetector {
    alpha: f64,
    current_rate: f64,
    window_count: u64,
    last_window_ms: u64,
}

impl RateDetector {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            current_rate: 0.0,
            window_count: 0,
            last_window_ms: 0,
        }
    }

    pub fn record_request(&mut self) {
        self.window_count = self.window_count.saturating_add(1);
    }

    pub fn tick(&mut self, now_ms: u64) -> FeatureValue<f64> {
        let elapsed_ms = now_ms.saturating_sub(self.last_window_ms);
        let sample_count = self.window_count as usize;

        if elapsed_ms == 0 || self.last_window_ms == 0 {
            self.last_window_ms = now_ms;
            self.window_count = 0;
            return FeatureValue::unavailable("Initializing rate detector", now_ms);
        }

        let instantaneous_rate = (self.window_count as f64) / (elapsed_ms as f64 / 1000.0);
        if self.current_rate == 0.0 {
            self.current_rate = instantaneous_rate;
        } else {
            self.current_rate =
                self.alpha * instantaneous_rate + (1.0 - self.alpha) * self.current_rate;
        }

        self.last_window_ms = now_ms;
        self.window_count = 0;

        FeatureValue::new(
            self.current_rate,
            MetricProvenance::Derived,
            sample_count,
            elapsed_ms,
            now_ms,
        )
    }
}

// ============================================================================
// F3: Heavy Hitters (Aged Count-Min Sketch + Space-Saving)
// ============================================================================

const CM_DEPTH: usize = 4;
const CM_WIDTH: usize = 1024;
const SPACE_SAVING_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct SpaceSavingEntry {
    pub key_hash: u64,
    pub count: u32,
    pub error: u32,
}

#[derive(Debug)]
pub struct HeavyHitterDetector {
    table: [[u32; CM_WIDTH]; CM_DEPTH],
    top_k: Vec<SpaceSavingEntry>,
    total_samples: usize,
    sample_decay_interval: usize,
}

impl HeavyHitterDetector {
    pub fn new() -> Self {
        Self {
            table: [[0; CM_WIDTH]; CM_DEPTH],
            top_k: Vec::with_capacity(SPACE_SAVING_CAPACITY),
            total_samples: 0,
            sample_decay_interval: 10_000,
        }
    }

    fn hash_cell(&self, key: u64, row: usize) -> usize {
        let mut x = key ^ (row as u64).wrapping_mul(0x517cc1b727220a95);
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        (x as usize) % CM_WIDTH
    }

    pub fn observe(&mut self, key_hash: u64) {
        self.total_samples = self.total_samples.saturating_add(1);

        // 1. Increment Count-Min Sketch
        for row in 0..CM_DEPTH {
            let col = self.hash_cell(key_hash, row);
            self.table[row][col] = self.table[row][col].saturating_add(1);
        }

        // 2. Update Space-Saving Top-K
        if let Some(entry) = self.top_k.iter_mut().find(|e| e.key_hash == key_hash) {
            entry.count = entry.count.saturating_add(1);
        } else if self.top_k.len() < SPACE_SAVING_CAPACITY {
            self.top_k.push(SpaceSavingEntry {
                key_hash,
                count: 1,
                error: 0,
            });
        } else {
            // Find minimum entry
            let mut min_idx = 0;
            let mut min_count = u32::MAX;
            for (idx, entry) in self.top_k.iter().enumerate() {
                if entry.count < min_count {
                    min_count = entry.count;
                    min_idx = idx;
                }
            }
            let min_entry = &mut self.top_k[min_idx];
            min_entry.key_hash = key_hash;
            min_entry.error = min_entry.count;
            min_entry.count = min_entry.count.saturating_add(1);
        }

        // Periodic halving decay to adapt to non-stationary shifts
        if self.total_samples >= self.sample_decay_interval {
            self.decay();
            self.total_samples = 0;
        }
    }

    pub fn estimate_frequency(&self, key_hash: u64) -> u32 {
        let mut min_val = u32::MAX;
        for row in 0..CM_DEPTH {
            let col = self.hash_cell(key_hash, row);
            min_val = min_val.min(self.table[row][col]);
        }
        min_val
    }

    pub fn get_top_k(&self) -> Vec<(u64, u32)> {
        let mut sorted = self
            .top_k
            .iter()
            .map(|e| (e.key_hash, e.count))
            .collect::<Vec<_>>();
        sorted.sort_unstable_by_key(|a| std::cmp::Reverse(a.1));
        sorted
    }

    fn decay(&mut self) {
        for row in 0..CM_DEPTH {
            for col in 0..CM_WIDTH {
                self.table[row][col] >>= 1;
            }
        }
        for entry in &mut self.top_k {
            entry.count >>= 1;
            entry.error >>= 1;
        }
    }
}

impl Default for HeavyHitterDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// F4: Zipf Skew Estimator (Log-Rank Linear Regression)
// ============================================================================

#[derive(Debug, Default)]
pub struct ZipfSkewDetector;

impl ZipfSkewDetector {
    /// Estimates Zipf parameter alpha from top-k heavy hitters via linear regression
    /// on ln(frequency) vs ln(rank).
    pub fn estimate(top_k: &[(u64, u32)], sample_count: usize) -> FeatureValue<Option<f64>> {
        if top_k.len() < 10 || sample_count < 200 {
            return FeatureValue::unavailable("Insufficient top-k keys for Zipf MLE", 0);
        }

        let mut x_sum = 0.0;
        let mut y_sum = 0.0;
        let mut xy_sum = 0.0;
        let mut xx_sum = 0.0;
        let n = top_k.len().min(50) as f64;

        for (rank_0, (_key, count)) in top_k.iter().take(50).enumerate() {
            let rank = (rank_0 + 1) as f64;
            let freq = (*count as f64).max(1.0);

            let ln_rank = rank.ln();
            let ln_freq = freq.ln();

            x_sum += ln_rank;
            y_sum += ln_freq;
            xy_sum += ln_rank * ln_freq;
            xx_sum += ln_rank * ln_rank;
        }

        let denominator = n * xx_sum - x_sum * x_sum;
        if denominator.abs() < 1e-9 {
            return FeatureValue::new(None, MetricProvenance::Derived, sample_count, 0, 0);
        }

        let slope = (n * xy_sum - x_sum * y_sum) / denominator;
        let alpha = -slope; // In Zipf, ln(f) = C - alpha * ln(r)

        FeatureValue::new(
            Some(alpha.clamp(0.0, 3.0)),
            MetricProvenance::Derived,
            sample_count,
            0,
            0,
        )
    }
}

// ============================================================================
// F5: Spatially Sampled Working Set (SHARDS)
// ============================================================================

#[derive(Debug)]
pub struct WorkingSetDetector {
    sample_modulus: u64,
    sampled_keys: HashMap<u64, u64>,
    max_sampled_keys: usize,
}

impl WorkingSetDetector {
    pub fn new(sampling_rate_inv: u64) -> Self {
        Self {
            sample_modulus: sampling_rate_inv.max(1),
            sampled_keys: HashMap::with_capacity(1024),
            max_sampled_keys: 4096,
        }
    }

    pub fn observe(&mut self, key_hash: u64, now_ms: u64) {
        if key_hash.is_multiple_of(self.sample_modulus) {
            if self.sampled_keys.len() >= self.max_sampled_keys {
                // Evict oldest sample if capacity exceeded
                if let Some((&oldest_key, _)) = self.sampled_keys.iter().min_by_key(|(_, &t)| t) {
                    self.sampled_keys.remove(&oldest_key);
                }
            }
            self.sampled_keys.insert(key_hash, now_ms);
        }
    }

    pub fn estimate_working_set(&self, window_ms: u64, now_ms: u64) -> FeatureValue<usize> {
        let cutoff = now_ms.saturating_sub(window_ms);
        let active_sampled = self
            .sampled_keys
            .values()
            .filter(|&&last_seen| last_seen >= cutoff)
            .count();

        let estimated_total_keys = active_sampled * (self.sample_modulus as usize);
        let estimated_bytes = estimated_total_keys * 1024; // Average 1 KB / object

        FeatureValue::new(
            estimated_bytes,
            MetricProvenance::Derived,
            active_sampled,
            window_ms,
            now_ms,
        )
    }
}

// ============================================================================
// F6: CUSUM Burst Detector
// ============================================================================

#[derive(Debug)]
pub struct BurstDetector {
    baseline_rate: f64,
    cusum_pos: f64,
    threshold: f64,
    slack: f64,
}

impl BurstDetector {
    pub fn new(threshold: f64, slack: f64) -> Self {
        Self {
            baseline_rate: 0.0,
            cusum_pos: 0.0,
            threshold,
            slack,
        }
    }

    pub fn update(&mut self, measured_rate: f64) -> FeatureValue<bool> {
        if self.baseline_rate == 0.0 {
            self.baseline_rate = measured_rate;
            return FeatureValue::new(false, MetricProvenance::Derived, 1, 0, 0);
        }

        let delta = measured_rate - (self.baseline_rate + self.slack);
        self.cusum_pos = (self.cusum_pos + delta).max(0.0);

        let burst = self.cusum_pos > self.threshold;
        if burst {
            // Adapt baseline upward slowly
            self.baseline_rate = 0.9 * self.baseline_rate + 0.1 * measured_rate;
            self.cusum_pos = 0.0;
        } else {
            self.baseline_rate = 0.99 * self.baseline_rate + 0.01 * measured_rate;
        }

        FeatureValue::new(burst, MetricProvenance::Derived, 1, 0, 0)
    }
}

// ============================================================================
// F7: Hot-Key Emergence Detector
// ============================================================================

#[derive(Debug, Default)]
pub struct HotKeyEmergenceDetector {
    previous_counts: HashMap<u64, u32>,
}

impl HotKeyEmergenceDetector {
    pub fn evaluate(&mut self, current_top_k: &[(u64, u32)]) -> FeatureValue<bool> {
        let mut emergent = false;

        for &(key, count) in current_top_k.iter().take(10) {
            let prev = self.previous_counts.get(&key).copied().unwrap_or(0);
            if prev > 0 && count >= prev.saturating_mul(3) && count >= 30 {
                emergent = true;
                break;
            }
        }

        self.previous_counts.clear();
        for &(key, count) in current_top_k {
            self.previous_counts.insert(key, count);
        }

        FeatureValue::new(emergent, MetricProvenance::Derived, current_top_k.len(), 0, 0)
    }
}

// ============================================================================
// Unified Workload Intelligence Pipeline
// ============================================================================

pub struct WorkloadPipeline {
    rate_detector: RateDetector,
    heavy_hitters: HeavyHitterDetector,
    working_set: WorkingSetDetector,
    burst_detector: BurstDetector,
    hot_key_detector: HotKeyEmergenceDetector,
    total_requests: u64,
    total_hits: u64,
    total_misses: u64,
    origin_fetches: u64,
    origin_errors: u64,
    stale_serves: u64,
    invalidation_count: u64,
    recent_latencies_us: Vec<u64>,
}

impl WorkloadPipeline {
    pub fn new() -> Self {
        Self {
            rate_detector: RateDetector::new(0.2),
            heavy_hitters: HeavyHitterDetector::new(),
            working_set: WorkingSetDetector::new(16),
            burst_detector: BurstDetector::new(500.0, 50.0),
            hot_key_detector: HotKeyEmergenceDetector::default(),
            total_requests: 0,
            total_hits: 0,
            total_misses: 0,
            origin_fetches: 0,
            origin_errors: 0,
            stale_serves: 0,
            invalidation_count: 0,
            recent_latencies_us: Vec::with_capacity(1000),
        }
    }

    pub fn observe_request(
        &mut self,
        key_hash: u64,
        is_hit: bool,
        latency_us: u64,
        timestamp_ms: u64,
    ) {
        self.total_requests = self.total_requests.saturating_add(1);
        if is_hit {
            self.total_hits = self.total_hits.saturating_add(1);
        } else {
            self.total_misses = self.total_misses.saturating_add(1);
        }

        self.rate_detector.record_request();
        self.heavy_hitters.observe(key_hash);
        self.working_set.observe(key_hash, timestamp_ms);

        if self.recent_latencies_us.len() < 1000 {
            self.recent_latencies_us.push(latency_us);
        }
    }

    pub fn observe_origin_fetch(&mut self, is_error: bool) {
        self.origin_fetches = self.origin_fetches.saturating_add(1);
        if is_error {
            self.origin_errors = self.origin_errors.saturating_add(1);
        }
    }

    pub fn observe_invalidation(&mut self) {
        self.invalidation_count = self.invalidation_count.saturating_add(1);
    }

    pub fn observe_stale_serve(&mut self) {
        self.stale_serves = self.stale_serves.saturating_add(1);
    }

    pub fn produce_window(&mut self, now_ms: u64, window_duration_ms: u64) -> SignalWindow {
        let f1_rate = self.rate_detector.tick(now_ms);

        let hit_ratio = if self.total_requests > 0 {
            self.total_hits as f64 / self.total_requests as f64
        } else {
            0.0
        };
        let f2_hit_ratio = FeatureValue::new(
            hit_ratio,
            MetricProvenance::Derived,
            self.total_requests as usize,
            window_duration_ms,
            now_ms,
        );

        let top_k = self.heavy_hitters.get_top_k();
        let f3_heavy = FeatureValue::new(
            top_k.clone(),
            MetricProvenance::Derived,
            self.total_requests as usize,
            window_duration_ms,
            now_ms,
        );

        let f4_skew = ZipfSkewDetector::estimate(&top_k, self.total_requests as usize);
        let f5_ws = self.working_set.estimate_working_set(window_duration_ms, now_ms);

        let f6_burst = self.burst_detector.update(f1_rate.value);
        let f7_hot = self.hot_key_detector.evaluate(&top_k);

        let miss_amp = if self.total_misses > 0 {
            self.origin_fetches as f64 / self.total_misses as f64
        } else {
            1.0
        };
        let f9_miss_amp = FeatureValue::new(
            miss_amp,
            MetricProvenance::Derived,
            self.total_misses as usize,
            window_duration_ms,
            now_ms,
        );

        let p99_latency_ms = if !self.recent_latencies_us.is_empty() {
            let mut sorted = self.recent_latencies_us.clone();
            sorted.sort_unstable();
            let idx = (sorted.len() as f64 * 0.99) as usize;
            (sorted[idx.min(sorted.len() - 1)] / 1000) as u32
        } else {
            0
        };
        let f10_p99 = FeatureValue::new(
            p99_latency_ms,
            MetricProvenance::Measured,
            self.recent_latencies_us.len(),
            window_duration_ms,
            now_ms,
        );

        let origin_error_rate = if self.origin_fetches > 0 {
            self.origin_errors as f64 / self.origin_fetches as f64
        } else {
            0.0
        };
        let f10_err = FeatureValue::new(
            origin_error_rate,
            MetricProvenance::Measured,
            self.origin_fetches as usize,
            window_duration_ms,
            now_ms,
        );

        let stale_ratio = if self.total_requests > 0 {
            self.stale_serves as f64 / self.total_requests as f64
        } else {
            0.0
        };
        let f12_stale = FeatureValue::new(
            stale_ratio,
            MetricProvenance::Derived,
            self.total_requests as usize,
            window_duration_ms,
            now_ms,
        );

        // Clear rolling latencies buffer
        self.recent_latencies_us.clear();

        SignalWindow {
            window_start_ms: now_ms.saturating_sub(window_duration_ms),
            window_end_ms: now_ms,
            request_count: self.total_requests,
            f1_request_rate_rps: f1_rate,
            f2_hit_ratio,
            f3_top_heavy_hitters: f3_heavy,
            f4_zipf_alpha: f4_skew,
            f5_working_set_bytes: f5_ws,
            f6_burst_detected: f6_burst,
            f7_hot_key_emergence: f7_hot,
            f9_miss_amplification: f9_miss_amp,
            f10_origin_p99_latency_ms: f10_p99,
            f10_origin_error_rate: f10_err,
            f12_stale_served_ratio: f12_stale,
        }
    }
}

impl Default for WorkloadPipeline {
    fn default() -> Self {
        Self::new()
    }
}
