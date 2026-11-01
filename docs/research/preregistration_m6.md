# PhantomCache — M6 Pre-Registration Specification (RT-1)

**Document:** `docs/research/preregistration_m6.md`  
**Date of Freeze:** `2026-10-29T18:30:00+05:30` (IST)  
**Author:** Antigravity (AG) — Research Evaluation Track  
**Prerequisites:** G-M5 (PASS), M6-00 (PASS), M6-01 (PASS)  
**Status:** FROZEN — Prior to any M6 Evaluation Run  

---

## 1. Hypotheses & Primary / Secondary Endpoints

### 1.1 Hypotheses

- **H1 (Primary Benefit):** Under partial origin degradation (latency inflation, brownout, error bursts, retry storms), the integrated origin-aware adaptive controller (`PIE-Retained`: Dynamic Stale Extension + AIMD Origin Concurrency Cap + TinyLFU Admission Filter) reduces peak origin concurrency ($C_{\text{peak}}$) and client p99 latency ($L_{p99}$) relative to `BL-12b` (family-tuned static deployable baseline) by at least $\delta_{\text{min}} = 0.05$ (5% relative reduction), with hit-ratio loss $\le 1.0$ percentage point, and exactly zero violations of configured stale and origin concurrency caps.
- **H2 (Mechanism Attribution):** Most of the gain in H1 ($> 70\%$) is attributable to miss-first shedding (AIMD concurrency cap) and dynamic stale window extension, rather than adaptive eviction.
- **H3 (Resource & Overhead Budget):** The integrated controller adds $< 1\%$ CPU utilization, $< 2\,\mu\text{s}$ p50 latency overhead on the hit path, and maintains $< 16\text{ MB}$ memory footprint.
- **H4 (Dynamic Stability):** The controller exhibits no oscillation or flapping ($\le 4$ policy mode switches per degradation episode; 60s minimum hold cooldown).
- **H5 (Shadow Evaluator Fidelity):** The spatially sampled shadow evaluator estimates miss ratios within $\pm 0.03$ (MAE) of full-scale simulation across all evaluation workloads.
- **H6 (Decision Protocol Bounded Errors):** The online canary decision protocol achieves 0% false commits on regressions and $\ge 90\%$ statistical power on true improvements ($\ge 8\%$).

### 1.2 Endpoints

| Endpoint Category | Metric | Definition | Threshold / Tolerance |
|---|---|---|---|
| **Primary 1** | $\Delta C_{\text{peak}}$ | Relative reduction in peak origin concurrency vs `BL-12b` | $D_{\text{rel}} \ge 0.05$ ($CI_{\text{lower}} > 0$) |
| **Primary 2** | $\Delta L_{p99}$ | Relative reduction in client P99 response latency vs `BL-12b` | $D_{\text{rel}} \ge 0.05$ ($CI_{\text{lower}} > 0$) |
| **Constraint 1** | Hit Ratio Difference ($\Delta \text{HR}$) | $\text{HR}_{\text{adaptive}} - \text{HR}_{\text{static}}$ | $\ge -0.01$ (loss $\le 1.0$ pp) |
| **Constraint 2** | Stale-Cap Violations | Requests served with age $> \text{cap}$ | **Strictly 0** |
| **Constraint 3** | Origin-Cap Violations | Concurrency $> C_{\text{cap}}$ | **Strictly 0** |
| **Secondary 1** | Total Upstream Requests | Cumulative requests dispatched to origin | Reduction $> 10\%$ |
| **Secondary 2** | P50 / P95 Latency | Client response time distribution | No regression ($> 2\%$) |
| **Secondary 3** | Mean Stale Age | Average age of stale responses served during degradation | $\le 0.5 \times \text{StaleCap}$ |

---

## 2. Formal Falsification Criteria

Any one of the following criteria conclusively rejects or narrows H1:
- **F-a:** The tuned static baseline (`BL-12b` or `BL-6`) achieves $C_{\text{peak}}$ and $L_{p99}$ within **10%** of the adaptive controller on $\ge 80\%$ of evaluated scenarios.
- **F-b:** The controller's advantage vanishes when origin latency is heterogeneous per key or under gradual linear degradation.
- **F-c:** The controller increases client p99 latency by $> 5\%$ in any scenario versus the unconstrained baseline (`BL-1`).
- **F-d:** Any violation of the configured stale cap ($\Delta_{\text{max}}$) or origin concurrency cap ($C_{\text{cap}}$).
- **F-e:** The 95% bootstrap confidence interval of the paired difference includes zero across $\ge 30$ sealed seeds.
- **F-f:** Substantial overlap with published prior art invalidating technical differentiation.

---

## 3. Evaluation Design & Scenario Matrix

### 3.1 Workload Families

1. **Tuning Families (Sanity / Sanity Check Only):**
   - `W-Z`: Standard Zipfian distribution ($\alpha = 0.95$).
   - `W-FC`: Flash crowd / sudden popular item emergence.
   - `W-SC`: Repeated scan-heavy working set churn.
2. **Held-Out Families (Primary Evaluation):**
   - `W-DI`: Diurnal sinusoidal load fluctuation.
   - `W-SH`: Distribution shift / working set drift.
   - `W-MX`: Mixed read-heavy / write-invalidation traffic.
   - `W-HK`: Dynamic hot-key emergence.
   - `W-TR`: Production-like empirical trace replay.
3. **Sealed Family (Introduced Exclusively for M6):**
   - `W-SF`: Adversarial non-stationary bimodal workload with sudden phase shifts and variable item cost.

### 3.2 Origin Degradation Regimes

1. `O-DEG`: Step degradation ($3\times$ origin latency spike).
2. `O-BRN`: Brownout (30% upstream 503 error rate).
3. `O-SLW`: Slow origin recovery following transient outage.
4. `O-CO`: Cold restart stampede under high concurrent arrival.
5. `O-RT`: Client retry storm ($3\times$ multiplier on client timeouts).
6. `O-FL`: Flapping origin health (periodic degradation pulses).
7. `O-HET`: Heterogeneous per-key origin computation latency.
8. `O-GRD`: Gradual linear latency inflation.

### 3.3 Comparative Baseline Arms

1. `BL-0`: Naive FIFO (no coalescing, no stale serving, no rate limiter).
2. `BL-1`: Request coalescing (single-flight) only.
3. `BL-4`: AIMD origin concurrency limiter (Breakwater/Netflix style).
4. `BL-5`: Circuit breaker (fail-fast on error threshold).
5. `BL-6`: Oracle-tuned static baseline (best static configuration discovered by grid search).
6. `BL-12b`: Family-tuned static deployable baseline (single-flight + tuned SWR + AIMD limiter).
7. `BL-CW`: DynamoDB constant-work asynchronous refresh.
8. `BL-FT`: Farahbakhsh theoretical tagged fill / young-priority baseline.
9. `BL-PIE-legacy`: Legacy threshold PIE controller (quarantined).
10. `PIE-Retained`: Integrated origin-aware controller (TinyLFU + Dynamic Stale Window + AIMD Concurrency Cap).

---

## 4. Sealed Seeds & Execution Protocol

- **Sealed Seed Set $S$:** $\mathbf{S} = [30001..=30030]$ (30 independent pseudo-random seeds generated via SplitMix64).
- **Execution Order:** Interleaved across arms to prevent temporal or machine-state bias:
  $$\text{For each seed } s \in S: \text{For each scenario } o: \text{Evaluate all arms in randomized order}.$$
- **Exclusion & Invalid Run Rules:**
  - Host OS context switch freeze $> 100\text{ ms} \implies \text{INVALID}$.
  - System memory allocation failure $\implies \text{INVALID}$.
  - Every invalid run is discarded, logged with reason, and rerun with the identical seed.

---

## 5. Statistical Analysis Plan

1. **Paired Differences:** Primary comparisons are paired strictly by seed:
   $$D_i = Y_{\text{adaptive}, i} - Y_{\text{static}, i} \quad \text{for } i \in S$$
2. **Confidence Intervals:** 95% BCa (bias-corrected and accelerated) bootstrap intervals with $B = 10,000$ resamples.
3. **Multiple Testing Correction:** Holm-Bonferroni step-down correction applied across the family of primary endpoints:
   $$p_{(k)} \le \frac{\alpha}{m - k + 1}$$
4. **Effect Size:** Relative difference of medians alongside Cliff's $\delta$ non-parametric effect size.
5. **No Data-Dependent Stopping:** All 30 seeds per cell are evaluated completely regardless of early intermediate trends.

---

## 6. Pre-Registration Cryptographic Freeze

- **Document Version:** v1.0 (M6 Pre-Registration Freeze)
- **Frozen Timestamp:** `2026-10-29T18:30:00+05:30` (IST)
- **Amendments Protocol:** Any post-freeze modifications require a dated addendum with documented justification. Any analyses conducted outside this specification are strictly classified and labeled as *exploratory*.

