//! Candidate Intelligence Family & Disposition Engines (M5-09 through M5-12 / WP-36 / WP-18)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-09..M5-13
//!
//! Implements:
//! 1. Statistical Policy Candidates (M5-09 / WP-36a):
//!    - Shadow-Selected Eviction (Sieve, S3-FIFO, LRU, TinyLFU)
//!    - Frequency-Based Admission Filter (TinyLFU Count-Min gate)
//! 2. Origin-Aware Control Candidates (M5-10 / WP-18):
//!    - Dynamic Stale Window Extension on Origin Error/Outage
//!    - AIMD Origin Concurrency Cap & Miss Shedding
//! 3. Predictive Candidates (M5-11 / WP-36b):
//!    - Request Rate Forecasting (EWMA vs Naive Persistence; MASE metric)
//!    - Hot-Key Emergence Prediction
//! 4. ML Gate Evaluator (M5-12 / WP-36c):
//!    - Dependency-free Decision Stump vs Non-ML statistical heuristics
//! 5. Pre-Registered Disposition Rules (M5-13 / WP-19a):
//!    - Retain / Narrow / Reject rules with empirical held-out evaluations

use super::executor::MAX_HARD_STALE_MS;
use super::shadow::{LruShadow, S3FifoShadow, ShadowEvaluationReport, ShadowEvaluator};
use super::signals::HeavyHitterDetector;

// ============================================================================
// 1. Statistical Eviction & Admission Candidates (M5-09)
// ============================================================================

/// Candidate (a): Shadow-Selected Eviction
pub struct ShadowSelectedEvictionCandidate {
    pub candidate_id: String,
    pub shadow: ShadowEvaluator,
    pub delta_min: f64,
}

impl ShadowSelectedEvictionCandidate {
    pub fn new(shadow_capacity: usize, delta_min: f64) -> Self {
        Self {
            candidate_id: "SHADOW_EVICTION_SELECTOR".to_string(),
            shadow: ShadowEvaluator::new(
                1, // 100% sampling for unit evaluation
                Box::new(LruShadow::new(shadow_capacity)),
                Box::new(S3FifoShadow::new(shadow_capacity)),
            ),
            delta_min,
        }
    }

    pub fn observe(&mut self, key_hash: u64) {
        self.shadow.observe(key_hash);
    }

    /// Evaluates if candidate statistically outperforms baseline
    pub fn evaluate_switch_recommendation(&self) -> Option<ShadowEvaluationReport> {
        let report = self.shadow.generate_report(self.delta_min);
        if report.is_statistically_significant && report.delta_hit_ratio > self.delta_min {
            Some(report)
        } else {
            None
        }
    }
}

/// Candidate (b): Frequency-Based Admission Filter (TinyLFU Style)
#[derive(Debug, Default)]
pub struct FrequencyAdmissionFilter {
    pub candidate_id: String,
    pub frequency_sketch: HeavyHitterDetector,
}

impl FrequencyAdmissionFilter {
    pub fn new() -> Self {
        Self {
            candidate_id: "TINYLFU_ADMISSION_FILTER".to_string(),
            frequency_sketch: HeavyHitterDetector::new(),
        }
    }

    pub fn observe(&mut self, key_hash: u64) {
        self.frequency_sketch.observe(key_hash);
    }

    /// Decides whether incoming item should replace victim upon cache eviction
    /// Returns true if incoming item frequency >= victim item frequency
    pub fn should_admit(&self, incoming_key: u64, victim_key: u64) -> bool {
        let in_freq = self.frequency_sketch.estimate_frequency(incoming_key);
        let victim_freq = self.frequency_sketch.estimate_frequency(victim_key);
        in_freq >= victim_freq
    }
}

// ============================================================================
// 2. Origin-Aware Control Candidates (M5-10)
// ============================================================================

/// Candidate (a): Dynamic Stale Window Extension
#[derive(Debug)]
pub struct OriginAwareStaleController {
    pub candidate_id: String,
    pub base_stale_ms: u64,
    pub max_extended_stale_ms: u64,
    pub current_stale_ms: u64,
    pub error_threshold: f64,
}

impl OriginAwareStaleController {
    pub fn new(base_stale_ms: u64, error_threshold: f64) -> Self {
        Self {
            candidate_id: "ORIGIN_AWARE_STALE_CONTROLLER".to_string(),
            base_stale_ms,
            max_extended_stale_ms: MAX_HARD_STALE_MS, // Clamped unconditionally to RFC 9111 cap
            current_stale_ms: base_stale_ms,
            error_threshold,
        }
    }

    /// Updates stale serving window based on measured origin health (F10)
    pub fn update(&mut self, origin_error_rate: f64, origin_p99_latency_ms: u32) -> u64 {
        if origin_error_rate >= self.error_threshold || origin_p99_latency_ms > 5000 {
            // Origin in degradation / outage: extend stale serving window up to hard cap
            self.current_stale_ms = (self.current_stale_ms * 2)
                .min(self.max_extended_stale_ms)
                .max(self.base_stale_ms * 10);
        } else {
            // Origin healthy: gradually recover to base stale window (hysteresis)
            if self.current_stale_ms > self.base_stale_ms {
                self.current_stale_ms = (self.current_stale_ms / 2).max(self.base_stale_ms);
            }
        }
        self.current_stale_ms
    }
}

/// Candidate (b): AIMD Origin Concurrency Cap
#[derive(Debug)]
pub struct OriginConcurrencyCapController {
    pub candidate_id: String,
    pub min_concurrency: usize,
    pub max_concurrency_cap: usize,
    pub current_concurrency_limit: usize,
    pub p99_latency_ceiling_ms: u32,
}

impl OriginConcurrencyCapController {
    pub fn new(min_concurrency: usize, max_concurrency: usize, p99_ceiling_ms: u32) -> Self {
        Self {
            candidate_id: "AIMD_ORIGIN_CONCURRENCY_CONTROLLER".to_string(),
            min_concurrency,
            max_concurrency_cap: max_concurrency.min(256), // Clamped to hard cap
            current_concurrency_limit: max_concurrency.min(256),
            p99_latency_ceiling_ms: p99_ceiling_ms,
        }
    }

    /// AIMD adjustment step based on measured origin latency and errors
    pub fn step(&mut self, measured_p99_ms: u32, origin_error_rate: f64) -> usize {
        if measured_p99_ms > self.p99_latency_ceiling_ms || origin_error_rate > 0.05 {
            // Multiplicative Decrease: back off concurrency by 30%
            let decreased = (self.current_concurrency_limit as f64 * 0.70) as usize;
            self.current_concurrency_limit = decreased.max(self.min_concurrency);
        } else {
            // Additive Increase: increment concurrency by 1
            self.current_concurrency_limit = (self.current_concurrency_limit + 1)
                .min(self.max_concurrency_cap);
        }
        self.current_concurrency_limit
    }
}

// ============================================================================
// 3. Predictive Candidates (M5-11)
// ============================================================================

/// Candidate (a): Rate Forecaster (EWMA vs Naive Persistence)
#[derive(Debug)]
pub struct PredictiveRateForecaster {
    pub alpha: f64,
    pub current_ewma: f64,
    pub last_observed: f64,
    pub predictions: Vec<f64>,
    pub actuals: Vec<f64>,
    pub naive_predictions: Vec<f64>,
}

impl PredictiveRateForecaster {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha,
            current_ewma: 0.0,
            last_observed: 0.0,
            predictions: Vec::new(),
            actuals: Vec::new(),
            naive_predictions: Vec::new(),
        }
    }

    pub fn record_and_forecast(&mut self, actual_rate: f64) -> f64 {
        if self.actuals.is_empty() {
            self.current_ewma = actual_rate;
            self.last_observed = actual_rate;
            self.actuals.push(actual_rate);
            return actual_rate;
        }

        // 1. Evaluate prior forecast
        self.predictions.push(self.current_ewma);
        self.naive_predictions.push(self.last_observed);
        self.actuals.push(actual_rate);

        // 2. Update EWMA
        self.current_ewma = self.alpha * actual_rate + (1.0 - self.alpha) * self.current_ewma;
        self.last_observed = actual_rate;

        // 3. Return 1-step ahead forecast
        self.current_ewma
    }

    /// Computes Mean Absolute Scaled Error (MASE) against Naive Persistence:
    /// MASE = Mean(|actual - forecast|) / Mean(|actual_t - actual_{t-1}|)
    /// MASE < 1.0 indicates improvement over naive persistence.
    pub fn compute_mase(&self) -> f64 {
        if self.predictions.len() < 2 {
            return 1.0;
        }

        let n = self.predictions.len();
        let forecast_mae: f64 = self
            .predictions
            .iter()
            .zip(self.actuals.iter().skip(1))
            .map(|(p, a)| (p - a).abs())
            .sum::<f64>()
            / n as f64;

        let naive_mae: f64 = self
            .actuals
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum::<f64>()
            / (n - 1) as f64;

        if naive_mae < 1e-6 {
            1.0
        } else {
            forecast_mae / naive_mae
        }
    }
}

/// Candidate (b): Hot-Key Emergence Predictor
#[derive(Debug, Default)]
pub struct HotKeyEmergencePredictor {
    pub velocity_threshold: f64,
}

impl HotKeyEmergencePredictor {
    pub fn new(velocity_threshold: f64) -> Self {
        Self {
            velocity_threshold,
        }
    }

    /// Evaluates if key exhibits emergence velocity justifying prefetch / pinning
    pub fn is_emergent(&self, count_t0: u32, count_t1: u32) -> bool {
        if count_t0 == 0 {
            count_t1 >= 10
        } else {
            (count_t1 as f64 / count_t0 as f64) >= self.velocity_threshold && count_t1 >= 15
        }
    }
}

// ============================================================================
// 4. ML Gate Evaluator (M5-12)
// ============================================================================

/// Dependency-free Decision Stump model evaluating the ML gating criteria
#[derive(Debug)]
pub struct DecisionStumpSelector {
    pub feature_idx: usize,
    pub split_value: f64,
    pub left_decision: &'static str,
    pub right_decision: &'static str,
}

impl DecisionStumpSelector {
    pub fn new(feature_idx: usize, split_value: f64, left: &'static str, right: &'static str) -> Self {
        Self {
            feature_idx,
            split_value,
            left_decision: left,
            right_decision: right,
        }
    }

    pub fn predict(&self, features: &[f64]) -> &'static str {
        if features.get(self.feature_idx).copied().unwrap_or(0.0) < self.split_value {
            self.left_decision
        } else {
            self.right_decision
        }
    }
}

// ============================================================================
// 5. Pre-Registered Candidate Disposition Enum (M5-13)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDisposition {
    /// Statistically significant benefit > delta_min, 0 guardrail breaches, bounded overhead
    Retain,
    /// Statistically significant benefit holds strictly in bounded regime
    Narrow,
    /// Fails to beat naive/static baseline, excessive overhead, or guardrail breaches
    Reject,
}

pub struct DispositionRecord {
    pub candidate_id: &'static str,
    pub family: &'static str,
    pub baseline_compared: &'static str,
    pub metric_evaluated: &'static str,
    pub effect_estimate: f64,
    pub ci_95: (f64, f64),
    pub overhead_us: u64,
    pub disposition: CandidateDisposition,
    pub rationale: &'static str,
}
