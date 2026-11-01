# Phase P4: Adaptive Policy Selection, Runtime Switching and Guardrails

## 1. Objective
Demonstrate that PhantomCache evaluates and switches eviction and origin concurrency policies using empirical evidence while preserving safety invariants, guardrails, automatic rollback, and academic research integrity (preserving the M6 H1 NOT_SUPPORTED finding).

---

## 2. Architecture and Execution Path
- **Policy Execution Engine:** Dynamic policy executor supporting LRU, S3FIFO, and SIEVE algorithms (`src/intelligence/`).
- **Origin Concurrency Controller (AIMD):** Additive-Increase Multiplicative-Decrease adaptive gate protecting origin databases from overload (`src/intelligence/executor.rs`).
- **Guardrails & Rollback:** Performance monitoring comparing candidate policies against static baseline; regression automatically triggers rollback (`src/intelligence/guardrails.rs`).
- **Phantom Lab Scenario B:** Real-process live experiment comparing `BASE` (uncapped), `STATIC-CAP` (fixed ceiling), and `AIMD-CAP` (adaptive controller) under degraded origin latency (`src/lab_live/scenario_b.rs`).
- **Research Freeze Integrity:** Pinned freeze record (`docs/research/preregistration_m6_v2_2_freeze.json`) preserving H1 NOT_SUPPORTED (verified 56/56 hashes).

---

## 3. Exact Setup Instructions
1. Run with Phantom Lab and Intelligence enabled:
   ```powershell
   $env:PHANTOM_LAB_ENABLED = "true"
   $env:PHANTOM_INTELLIGENCE = "canary"
   $env:PHANTOM_ORIGIN_CAP_MAX = "10"
   $env:PHANTOM_ORIGIN_CAP_MIN = "2"
   target\release\phantom-cache-demo.exe
   ```
2. In Phantom Control Center (`http://localhost:3000`), navigate to **Phantom Lab**.

---

## 4. Policy Evaluation & Experiment Execution

### A. Scenario B Live Run
Start Scenario B through the Lab API:
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"kind":"scenario_b","spec":{"repetitions":1,"arms":["BASE","STATIC-CAP"],"warmup_s":1,"healthy_s":1,"degraded_s":1,"recovery_s":1,"rate_rps":50.0,"n_keys":50}}' \
  http://127.0.0.1:8085/api/v1/lab/runs
```
*Observable Execution:*
- Spawns isolated gateway process bound via Windows Job Object (`KillOnJobClose`).
- Exercises Phase 1: Warmup $\to$ Phase 2: Healthy $\to$ Phase 3: Degraded $\to$ Phase 4: Recovery.
- Evaluates `BASE` vs `STATIC-CAP`.
- Writes atomic run evidence to `lab-results/runs/<run_id>/run.json` and `events.jsonl`.

### B. Eviction Policy Comparison
Benchmark LRU vs S3FIFO vs SIEVE under identical trace inputs:
```powershell
cargo test --test p4_pie_tests test_p4_reproducible_policy_benchmark_comparison -j 1
```
- S3FIFO filters one-hit wonders, reducing origin fetch rate on long-tail workloads.
- Active operations strictly preserve memory safety invariants across all policies.

### C. Automatic Rollback Verification
```powershell
cargo test --test p4_pie_tests test_p4_pie_adaptive_switching_and_rollback -j 1
```
- A regression threshold violation in a candidate policy triggers automatic, zero-downtime rollback to the safe static baseline.

---

## 5. Research Integrity & Evidence Honesty

PhantomCache adheres to strict empirical honesty:
- **Preserved Finding:** Under Milestone 6 Hypothesis 1 (M6 H1), adaptive control was evaluated against a tuned static baseline. The result was **NOT_SUPPORTED** (adaptive policy did not demonstrate statistically significant superiority over a tuned static cap).
- **Labeling Contract:** Incomplete, interrupted, or exploratory experiment runs receive `EXPLORATORY` or `TRADE_OFF_FAILURE` labels. Under DEC-001, premature or incomplete runs are strictly prevented from claiming false `VALID` verdicts.
- **Serving Path Boundary:** While the Origin Concurrency Gate (AIMD) is integrated into the serving path, speculative predictive prewarming remains quarantined in accordance with ADR-002.

---

## 6. Actual Test Results & Automated Verification

### Automated Test Suites Executed:
1. `tests/integration/p4_pie_tests.rs`: **6 passed, 0 failed** (1.36s).
   - `test_p4_lru_eviction_order`: PASSED.
   - `test_p4_s3fifo_one_hit_wonder_filtering`: PASSED.
   - `test_p4_all_policies_preserve_active_op_safety_invariant`: PASSED.
   - `test_p4_pie_adaptive_switching_and_rollback`: PASSED.
   - `test_p4_reproducible_policy_benchmark_comparison`: PASSED.
   - `test_p4_live_pie_control_path_under_concurrent_traffic`: PASSED.
2. `tests/integration/lab_live_scenario_b_tests.rs`: **67 passed, 0 failed, 2 ignored** (53.55s).
   - Validates Scenario B runner, process lifecycle, cancellation, and honest evidence labeling.
3. `tests/integration/m6_research_evaluation_tests.rs`: **1 passed, 0 failed** (0.05s).
   - 56/56 research freeze file hashes verified intact.
4. `scripts/run_e2e_acceptance.ps1` (Steps 16–20): **5 passed, 0 failed**.
   - Validates Origin Gate enforcing state, Scenario B launch, event streaming, evidence artifact generation, and cooperative cancellation.

---

## 7. Evidence Artifact Locations
- Acceptance Evidence JSON: `lab-results/e2e_acceptance_evidence.json` (Steps 16–20).
- Scenario B Run Artifacts: `lab-results/runs/` (`run.json`, `events.jsonl`, `summary.json`).
- Research Freeze Verification: `docs/research/preregistration_m6_v2_2_freeze.json`.

---

## 8. Known Limitations & Research Boundaries
- **Binding M6 H1 Finding:** Under Milestone 6 Hypothesis 1 (M6 H1), adaptive policy control did not demonstrate statistically significant superiority over a tuned static baseline. Demonstrating P4 confirms mechanism execution and origin protection; it does not claim adaptive superiority.
- **Quarantined Prototypes:** Predictive prewarming and online workload forecasting remain quarantined research prototypes per ADR-002 and are not enabled in the live serving path. The AIMD origin concurrency controller is the sole active control mechanism in the serving path.

---

## 9. Acceptance Verdict
**Verdict: PASS (Mechanism Verified; Superiority Not Claimed)**  
Policy evaluation, comparative benchmarking, AIMD origin concurrency control, cooperative run cancellation, and research integrity contracts are fully verified and reproducible while preserving M6 H1 NOT_SUPPORTED.

