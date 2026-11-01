//! Phantom Intelligence & PIE v2 Core Architecture & Interface Definitions
//!
//! Authoritative Specification: `docs/research/intelligence_architecture.md`
//!
//! Organized across five pillars:
//! 1. Workload Intelligence (SignalWindow, Detectors)
//! 2. Predictive Intelligence (Predictors)
//! 3. Policy Intelligence (PolicyCandidate, Evaluators)
//! 4. Safe Adaptation (PolicyExecutor, Canary, Watchdog)
//! 5. Evaluation & Explainability (DecisionRecord, AuditStore)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod audit;
pub mod candidates;
pub mod engine;
pub mod executor;
pub mod shadow;
pub mod signals;

/// Operating mode for Phantom Intelligence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IntelligenceMode {
    /// Disabled. Request path operates strictly identical to static BL-1 single-flight.
    #[default]
    Off,
    /// Observation only. Telemetry and workload feature pipeline active off hot path.
    Observe,
    /// Advisory. Shadow evaluation active; recommendations logged to audit store, never applied.
    Advisory,
    /// Canary. Bounded live traffic exposure (f <= 5%) with immediate automated rollback.
    Canary,
    /// Active. Retained policies applied to live traffic, subject to permanent 1% holdback.
    Active,
}

/// Provenance metadata for every metric and signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricProvenance {
    /// Directly measured from real system hardware or OS clock.
    Measured,
    /// Mathematically derived from measured inputs via declared formula.
    Derived,
    /// Produced by deterministic virtual-time simulation.
    Simulated,
    /// Data missing, sample count below minimum, or signal degraded.
    Unavailable(String),
}

/// Wrapped feature value carrying provenance and validity bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureValue<T> {
    pub value: T,
    pub provenance: MetricProvenance,
    pub sample_count: usize,
    pub window_duration_ms: u64,
    pub calculated_at_ms: u64,
}

impl<T> FeatureValue<T> {
    pub fn new(
        value: T,
        provenance: MetricProvenance,
        sample_count: usize,
        window_duration_ms: u64,
        calculated_at_ms: u64,
    ) -> Self {
        Self {
            value,
            provenance,
            sample_count,
            window_duration_ms,
            calculated_at_ms,
        }
    }

    pub fn unavailable(reason: impl Into<String>, calculated_at_ms: u64) -> Self
    where
        T: Default,
    {
        Self {
            value: T::default(),
            provenance: MetricProvenance::Unavailable(reason.into()),
            sample_count: 0,
            window_duration_ms: 0,
            calculated_at_ms,
        }
    }

    pub fn is_valid(&self) -> bool {
        !matches!(self.provenance, MetricProvenance::Unavailable(_))
    }
}

/// Aggregated workload signal window capturing features F1..F12.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWindow {
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub request_count: u64,
    pub f1_request_rate_rps: FeatureValue<f64>,
    pub f2_hit_ratio: FeatureValue<f64>,
    pub f3_top_heavy_hitters: FeatureValue<Vec<(u64, u32)>>,
    pub f4_zipf_alpha: FeatureValue<Option<f64>>,
    pub f5_working_set_bytes: FeatureValue<usize>,
    pub f6_burst_detected: FeatureValue<bool>,
    pub f7_hot_key_emergence: FeatureValue<bool>,
    pub f9_miss_amplification: FeatureValue<f64>,
    pub f10_origin_p99_latency_ms: FeatureValue<u32>,
    pub f10_origin_error_rate: FeatureValue<f64>,
    pub f12_stale_served_ratio: FeatureValue<f64>,
}

/// Category of caching or origin control policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyKind {
    Eviction,
    Admission,
    RefreshEarly,
    StaleServing,
    ConcurrencyShedding,
}

/// Specification of a policy candidate evaluated by PIE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCandidate {
    pub candidate_id: String,
    pub kind: PolicyKind,
    pub parameters: BTreeMap<String, f64>,
    pub bounds: BTreeMap<String, (f64, f64)>,
    pub expected_effect: String,
}

/// Decision engine verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Improvement CI lower bound > delta_min, all guardrails passed.
    Commit,
    /// Guardrail breach or regression upper bound > threshold. Revert immediately.
    Rollback,
    /// Neutral or inconclusive evidence. Maintain current baseline.
    NoAction,
    /// Uncaught panic, stall, or exception. Revert immediately and downgrade mode.
    Fault,
}

/// Immutable audit record of an evaluation decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision_id: u64,
    pub timestamp_ms: u64,
    pub mode: IntelligenceMode,
    pub trigger_reason: String,
    pub candidate: PolicyCandidate,
    pub estimated_effect: f64,
    pub confidence_interval: (f64, f64),
    pub guardrails_passed: bool,
    pub verdict: Verdict,
    pub explanation: String,
    pub config_hash_before: String,
    pub config_hash_after: String,
}

/// Detector trait for workload intelligence features.
pub trait Detector: Send + Sync {
    type Output;
    fn name(&self) -> &'static str;
    fn observe(&mut self, key_hash: u64, is_hit: bool, latency_us: u64, timestamp_ms: u64);
    fn evaluate(&self, window_duration_ms: u64) -> FeatureValue<Self::Output>;
    fn reset(&mut self);
}

/// Predictor trait for predictive intelligence features.
pub trait Predictor: Send + Sync {
    type Prediction;
    fn name(&self) -> &'static str;
    fn train_or_update(&mut self, signal: &SignalWindow);
    fn predict(&self) -> Option<Self::Prediction>;
    fn evaluate_downstream_utility(&self, actual: &SignalWindow) -> f64;
}

/// Evaluator trait for shadow and canary evaluation.
pub trait Evaluator: Send + Sync {
    fn evaluate_candidate(
        &mut self,
        candidate: &PolicyCandidate,
        signals: &SignalWindow,
        now_ms: u64,
    ) -> DecisionRecord;
}

/// PolicyExecutor trait governing atomic policy activation and safe fallback.
pub trait PolicyExecutor: Send + Sync {
    fn current_mode(&self) -> IntelligenceMode;
    fn set_mode(&self, mode: IntelligenceMode) -> Result<(), &'static str>;
    fn apply_policy(&self, candidate: &PolicyCandidate) -> Result<(), &'static str>;
    fn revert_to_safe(&self);
    fn heartbeat(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intelligence_mode_default_is_off() {
        assert_eq!(IntelligenceMode::default(), IntelligenceMode::Off);
    }

    #[test]
    fn test_feature_value_provenance_and_validity() {
        let valid = FeatureValue::new(42.5, MetricProvenance::Derived, 100, 5000, 1000);
        assert!(valid.is_valid());
        assert_eq!(valid.value, 42.5);

        let unavail: FeatureValue<f64> = FeatureValue::unavailable("Cold cache", 1000);
        assert!(!unavail.is_valid());
        assert_eq!(unavail.value, 0.0);
        assert_eq!(
            unavail.provenance,
            MetricProvenance::Unavailable("Cold cache".to_string())
        );
    }

    #[test]
    fn test_decision_record_serialization_roundtrip() {
        let candidate = PolicyCandidate {
            candidate_id: "TINYLFU_ADMISSION".to_string(),
            kind: PolicyKind::Admission,
            parameters: [("threshold".to_string(), 0.8)].into_iter().collect(),
            bounds: [("threshold".to_string(), (0.1, 1.0))].into_iter().collect(),
            expected_effect: "Filter one-hit wonders".to_string(),
        };

        let record = DecisionRecord {
            decision_id: 1,
            timestamp_ms: 123456789,
            mode: IntelligenceMode::Canary,
            trigger_reason: "High scan traffic detected".to_string(),
            candidate,
            estimated_effect: 0.12,
            confidence_interval: (0.06, 0.18),
            guardrails_passed: true,
            verdict: Verdict::Commit,
            explanation: "Scan resistance improved with 0 stale violations".to_string(),
            config_hash_before: "abc".to_string(),
            config_hash_after: "def".to_string(),
        };

        let json = serde_json::to_string(&record).expect("Serialization failed");
        let deserialized: DecisionRecord =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.decision_id, 1);
        assert_eq!(deserialized.verdict, Verdict::Commit);
        assert_eq!(deserialized.mode, IntelligenceMode::Canary);
    }
}
