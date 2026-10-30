# Phantom Intelligence & PIE v2: Architecture & Interface Specification (WP-30)

**Authoritative Specification:** `docs/research/phase_5_execution_package.md`  
**Milestone:** Phase 5 (M5 — Phantom Intelligence & PIE)  
**Status:** **AUTHORITATIVE SPECIFICATION — STEP M5-01**  
**Date:** `2026-10-29T17:12:00+05:30` (IST)  
**Author:** Antigravity (AG)  

---

## 1. Architectural Overview

Phantom Intelligence is an empirical, safe, adaptive caching intelligence system organized into five distinct pillars:

```
+----------------------------------------------------------------------------------------------------+
|                                  PHANTOM INTELLIGENCE ARCHITECTURE                                 |
+----------------------------------------------------------------------------------------------------+
|                                                                                                    |
|  +------------------------+      +------------------------+      +-------------------------------+ |
|  |  PILLAR 1: WORKLOAD    |      |  PILLAR 2: PREDICTIVE  |      |  PILLAR 3: POLICY             | |
|  |  INTELLIGENCE          |      |  INTELLIGENCE          |      |  INTELLIGENCE                 | |
|  +------------------------+      +------------------------+      +-------------------------------+ |
|  | F1 Rate (EWMA)         |      | Rate Forecasting       |      | Eviction: Sieve/S3/LRU/TinyLFU| |
|  | F2 Hit Ratio (exact)   |      | Hot-Key Emergence      |      | Admission: TinyLFU / Size     | |
|  | F3 Heavy Hitters (CM)  | ---> | Next-Key Markov        | ---> | Stale Window Extension        | |
|  | F4 Skew alpha (MLE)    |      | Origin Health Warning  |      | Adaptive Coalescing           | |
|  | F5 Working Set (SHARDS)|      | (Downstream verified)  |      | Concurrency Shedding          | |
|  | F10 Origin Health      |      +------------------------+      +-------------------------------+ |
|  +------------------------+                                                      |                 |
|              |                                                                   v                 |
|              |                  +----------------------------------------------------------------+ |
|              |                  |  PILLAR 4: SAFE ADAPTATION (PIE v2 Decision Engine)            | |
|              +----------------> +----------------------------------------------------------------+ |
|                                 | * Stratified Hash Buckets (B=1024)                             | |
|                                 | * Candidate Arm (f <= 5%), Control Arm, Holdback Arm (1%)      | |
|                                 | * Block Bootstrap CIs, Minimum Effect delta_min = 0.05         | |
|                                 | * Hard Guardrails (Staleness Cap, Byte Bound, P99 Bound)       | |
|                                 | * Cooldown Timer (>=60s), Automated Immediate Rollback         | |
|                                 | * Watchdog Liveness Monitor (1000ms heartbeat)                 | |
|                                 +----------------------------------------------------------------+ |
|                                                                  |                                 |
|                                                                  v                                 |
|                                 +----------------------------------------------------------------+ |
|                                 |  PILLAR 5: EVALUATION & EXPLAINABILITY                         | |
|                                 +----------------------------------------------------------------+ |
|                                 | * Bounded In-Memory Decision Store (100 records max)           | |
|                                 | * Immutable DecisionRecord (inputs, guardrails, verdict)       | |
|                                 | * Deterministic Replay Verification in Tier-1 Simulator        | |
|                                 +----------------------------------------------------------------+ |
+----------------------------------------------------------------------------------------------------+
```

---

## 2. Core Type Definitions and Compilable Trait Skeletons

### 2.1 Operating Mode
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntelligenceMode {
    Off,
    Observe,
    Advisory,
    Canary,
    Active,
}
```

### 2.2 Workload Feature Signals & Provenance
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricProvenance {
    Measured,
    Derived,
    Simulated,
    Unavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureValue<T> {
    pub value: T,
    pub provenance: MetricProvenance,
    pub sample_count: usize,
    pub window_duration_ms: u64,
    pub calculated_at_ms: u64,
}

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
```

### 2.3 Detector & Predictor Interfaces
```rust
pub trait Detector: Send + Sync {
    type Output;
    fn name(&self) -> &'static str;
    fn observe(&mut self, key_hash: u64, is_hit: bool, latency_us: u64, timestamp_ms: u64);
    fn evaluate(&self, window_duration_ms: u64) -> FeatureValue<Self::Output>;
    fn reset(&mut self);
}

pub trait Predictor: Send + Sync {
    type Prediction;
    fn name(&self) -> &'static str;
    fn train_or_update(&mut self, signal: &SignalWindow);
    fn predict(&self) -> Option<Self::Prediction>;
    fn evaluate_downstream_utility(&self, actual: &SignalWindow) -> f64;
}
```

### 2.4 Policy Candidate & Evaluator
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyKind {
    Eviction,
    Admission,
    RefreshEarly,
    StaleServing,
    ConcurrencyShedding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCandidate {
    pub candidate_id: String,
    pub kind: PolicyKind,
    pub parameters: BTreeMap<String, f64>,
    pub bounds: BTreeMap<String, (f64, f64)>,
    pub expected_effect: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Commit,
    Rollback,
    NoAction,
    Fault,
}

pub trait Evaluator: Send + Sync {
    fn evaluate_candidate(
        &mut self,
        candidate: &PolicyCandidate,
        signals: &SignalWindow,
        now_ms: u64,
    ) -> DecisionRecord;
}
```

### 2.5 Decision Record & Policy Executor
```rust
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

pub trait PolicyExecutor: Send + Sync {
    fn current_mode(&self) -> IntelligenceMode;
    fn set_mode(&self, mode: IntelligenceMode) -> Result<(), &'static str>;
    fn apply_policy(&self, candidate: &PolicyCandidate) -> Result<(), &'static str>;
    fn revert_to_safe(&self);
    fn heartbeat(&self);
}
```

---

## 3. Threading, Buffering & Hot-Path Isolation

To strictly guarantee that intelligence processing never impairs request path throughput or latency:
1. **Off-Path Asynchronous Execution:** All feature computation, sketch aging, and counterfactual evaluation execute on dedicated background worker tasks (`phantom-intel-worker`).
2. **Lock-Free Congestion Dropping:** Serving threads push compact telemetry event records (`key_hash`, `is_hit`, `latency_us`) into a bounded lock-free ring buffer (capacity: 65,536 events).
3. **Zero Backpressure:** If the ring buffer fills during high-traffic bursts, incoming sample records are dropped immediately (`drop-newest`), logging an `overrun_drops` counter. Serving threads **NEVER BLOCK**.
4. **Isolated Memory Arena:** All intelligence data structures (Count-Min Sketch, Space-Saving, Decision Store) are pre-allocated with static memory bounds. Total memory footprint is bounded at $\le 16$ MB.

---

## 4. Failure Semantics & Guardrails

### 4.1 Failure Semantics per Interface:
- **Detector Failure / Insufficient Data:** If sample count $N < N_{\text{min}}$, the feature returns `MetricProvenance::Unavailable("Insufficient samples")`. Decisions requiring that feature immediately output `Verdict::NoAction`.
- **Canary Guardrail Breach:** If candidate arm latency P99 inflates by $>10\%$, origin error rate increases, or a single stale-cap breach is observed, the canary evaluator terminates exposure within $\le 50$ ms and executes `revert_to_safe()`.
- **Decision Engine Panic or Exception:** Any panic within the intelligence thread is caught by a panic handler, records `Verdict::Fault`, executes `revert_to_safe()`, and downgrades the operating mode to `Advisory` or `Off`.
- **Watchdog Timeout:** The executor maintains an atomic timestamp of the latest intelligence loop heartbeat. If the heartbeat is not updated within 1,000 ms, the watchdog thread automatically calls `revert_to_safe()` and forces mode to `Off`.

---

## 5. Hot Path Invariant Guarantee

When `PHANTOM_INTELLIGENCE=off` (the production default):
- Ring buffer dispatch is a single atomic load check (`mode == Off => return`).
- Execution is strictly identical to the verified `BL-1` single-flight request path.
- Preservation hash of `src/core/coalesce.rs` remains untouched and identical.

