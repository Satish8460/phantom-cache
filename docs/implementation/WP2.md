# WP2 — Intelligence & Adaptive Control Plan (Revised)

## 1. Objective
Implement out-of-band telemetry pipelines, Count-Min sketch key popularity estimators, hot-key detectors, and the closed-loop control system (PIE canary rollouts, policy execution evaluation, and automatic rollbacks) under strict multi-objective optimization parameters.

---

## 2. Multi-Objective PIE System Design

### A. Optimization Objectives
PIE evaluates caching strategy modifications by computing a multi-objective utility score $U$:

$$U = w_1 \cdot \text{HitRate} - w_2 \cdot \text{Latency}_{\text{P99}} - w_3 \cdot \text{BackendLoad} - w_4 \cdot \text{ResourceCost}$$

Where resource costs combine memory footprint, CPU consumption, and inter-node network replication bandwidth.

### B. Decision Logic & States
* **COMMIT:** Candidate policy utility score exceeds baseline by more than the activation threshold, confidence criteria are met, and canary tests pass.
* **ROLLBACK:** Candidate policy utility drops below baseline or violates configured latency safety thresholds during a canary window.
* **NO ACTION:** Low confidence, insufficient sample evidence size, or minor predicted benefits relative to risk parameters.

---

## 3. Explicit Canary Guardrails

The controller operates within defined safety limits:
* **Baseline Window:** $60\text{ seconds}$ of steady-state profiling.
* **Candidate Window:** $30\text{ seconds}$ canary evaluation time.
* **Sample Size Requirement:** Minimum of $10,000$ logical requests within the canary segment.
* **Regression Threshold:** Maximum P99 latency increase of $10\%$ over baseline.
* **Confidence Interval:** $95\%$ statistical confidence bounds computed on sliding telemetry windows.
* **Cooldown Interval:** Minimum $60\text{ seconds}$ recovery phase after a rollback or commit action.
* **Exposure Limit:** Canary traffic is limited to a maximum $5\%$ of total request routing.

---

## 4. Telemetry Schema Separation

* **Raw Observations:**
  * Request timestamp.
  * Target key hash.
  * Observed latency (ms).
  * Cache hit/miss outcome.
  * Coalescing waiter count.
* **Derived Metrics:**
  * Rolling P50/P95/P99 latency calculations.
  * Hit-rate trends.
  * Backend load profiles.
* **Decisions:**
  * Candidate comparison logs.
  * Canary start/stop states.
  * Commit/Rollback command structures.

## 5. Completion Criteria
* Telemetry collector implemented.
* Count-Min sketch implemented.
* PIE decision loop, canary rules, and cooldown controllers implemented.
* All unit tests passed.

## Status
* **COMPLETED**

---

## 6. Traceability Log Structure
Every policy change decision must emit a fully traceable JSON entry containing:
1. **Observation:** Trigger key/workload shift description.
2. **Evidence:** Count-Min frequency estimators, active P99 latency scores.
3. **Candidate Policies:** Evaluated configurations (e.g. SIEVE vs. S3-FIFO).
4. **Constraints:** Memory capacity limits, network replication bandwidth thresholds.
5. **Expected Outcome:** Projected utility increase.
6. **Canary:** Target window, sample size, and exposure ratios.
7. **Actual Outcome:** Measured metrics from the canary window.
8. **Decision:** Commit, Rollback, or No Action verdict.

---

## 7. Implementation Files
* `src/intel/telemetry.rs`
* `src/intel/sketch.rs`
* `src/intel/workload.rs`
* `src/control/pie.rs`
* `src/control/policies.rs`
* `src/control/canary.rs`
* `src/control/rollback.rs`
