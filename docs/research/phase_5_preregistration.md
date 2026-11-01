# Phase 5 Pre-Registration Specification: Phantom Intelligence & PIE v2

**Authoritative Specification:** `docs/research/phase_5_execution_package.md`  
**Status:** **FROZEN & IMMUTABLE PRIOR TO CANDIDATE EVALUATION**  
**Created:** `2026-10-29T17:08:00+05:30` (IST)  
**Execution Agent:** Antigravity (AG)  

---

## 1. Operating Modes & Default Behavior

Phantom Intelligence defines five strictly hierarchical operating modes:
- `off` (**DEFAULT**): All intelligence components disabled; zero background threads, zero telemetry overhead, standard `BL-1` single-flight serving path.
- `observe`: Workload feature pipeline active; detectors calculate features F1..F12 off the hot path; no policy evaluation or candidate selection.
- `advisory`: Candidates evaluated via shadow simulation (or counterfactual modeling); recommendations recorded in decision audit store but **never applied** to live cache.
- `canary`: Candidates evaluated with limited live traffic exposure (stratified key-hash buckets, $f \le 5\%$) with automated rollback upon any guardrail trip.
- `active`: Retained policy candidates applied to live traffic, subject to permanent 1% holdback arm and continuous hard-cap enforcement.

### Mode Transition Rules:
- Upward transitions (`off` -> `observe` -> `advisory` -> `canary` -> `active`) require explicit authenticated administrative command.
- Downward transitions (`active` -> `advisory` on guardrail breach; any mode -> `off` on watchdog timeout or emergency kill switch) are **automatic and immediate**.
- Inputs with provenance `simulated` or `unavailable` are strictly rejected in live modes (`canary`, `active`).

---

## 2. Hard Guardrails & Constraints

Any candidate policy evaluation or active deployment must strictly satisfy:
1. **Zero Staleness-Cap Violations (EV-12):** `stale_cap_violations == 0`. RFC 9111 hard staleness cap ($T_{\text{fresh}} + T_{\text{stale}}$) is absolute and inviolable.
2. **Byte Budget Bound:** Cache byte occupancy must remain $\le C_{\text{max}}$.
3. **Client P99 Latency Bound:** Relative p99 latency inflation on candidate arm vs control arm $\le +10\%$.
4. **Origin Concurrency Bound:** Peak origin in-flight fetches must not exceed configured capacity.
5. **Origin Error Rate Bound:** Error rate on candidate arm $\le$ control arm.
6. **Watchdog Liveness:** Controller decision cycle heartbeat $\le 1000$ ms. Stalls trigger immediate fallback to `BL-1` safe baseline.

---

## 3. Canary Protocol & Statistical Decision Rules

### 3.1 Stratified Bucket Partitioning
- Address space partitioned into $B = 1024$ hash buckets based on deterministic key hash.
- Heavy-hitter keys are stratified evenly across arms to eliminate popularity imbalance.
- Three concurrent arms:
  1. **Candidate Arm ($f \le 5\%$):** Routes traffic for assigned buckets to candidate policy.
  2. **Control Arm ($1 - f - f_{\text{holdback}}$):** Routes to current active policy.
  3. **Permanent Holdback Arm ($f_{\text{holdback}} = 1\%$):** Always routed to safe baseline `BL-1` to prevent long-term multi-generational regression.

### 3.2 Statistical Verdict Rules
- **Minimum Sample Size:** $N \ge 10,000$ requests per arm before evaluating decision.
- **Minimum Detectable Effect ($\delta_{\text{min}}$):** $\delta_{\text{min}} = 0.05$ (5% relative reduction in primary metric).
- **Commit Rule:** Commit candidate iff $\text{CI}_{95\%}^{\text{lower}}(\text{improvement}) > \delta_{\text{min}}$ **AND** all guardrails satisfied across entire window.
- **Rollback Rule:** Trigger immediate rollback if any hard guardrail is breached **OR** $\text{CI}_{95\%}^{\text{upper}}(\text{regression}) > 0.10$.
- **NoAction Rule:** If neither commit nor rollback conditions are met, maintain current configuration.
- **Cooldown Period:** $T_{\text{cooldown}} \ge 60$ seconds post-switch to prevent oscillation.
- **Switch Rate Limit:** At most 4 policy adaptations per hour.

---

## 4. Workload Feature Catalog (F1..F12) & Accuracy Targets

| Feature | Description | Estimator | Validity Condition | Accuracy Target vs Ground Truth |
|---|---|---|---|---|
| **F1** | Request Rate | EWMA over fixed window | $N \ge 100$ | Relative error $\le 5\%$ |
| **F2** | Windowed Hit Ratio | Sliding counter snapshots | $N \ge 500$ | Exact ($0\%$ error) |
| **F3** | Heavy Hitters | Aged Count-Min Sketch + Space-Saving | $N \ge 1000$ | Precision $\ge 90\%$, Recall $\ge 85\%$ at top-100 |
| **F4** | Zipf Skew ($\alpha$) | MLE on heavy-hitter frequencies | $N \ge 1000$ | $|\hat{\alpha} - \alpha| \le 0.15$ |
| **F5** | Working Set / MRC | Spatially sampled shadow (SHARDS $R=1/16$) | $N \ge 2000$ | Relative miss-ratio error $\le 8\%$ |
| **F6** | Burst / Level Shift | CUSUM change detector | Window $\ge 5$ s | Detection delay $\le 2$ s, False alarm rate $\le 1\%$ |
| **F7** | Hot-Key Emergence | Geometric growth rate test on F3 | Rate $\ge 3\times$ | Precision $\ge 80\%$, Lead time $\ge 1$ s |
| **F8** | Locality | Sampled reuse distance histogram | $N \ge 1000$ | Wasserstein distance bound |
| **F9** | Miss Amplification | Origin requests / Client misses | Misses $\ge 50$ | Exact ($0\%$ error) |
| **F10** | Origin Health | Monotonic latency quantiles & error rate | $N_{\text{orig}} \ge 50$ | Agreement with origin log within $\pm 2\%$ |
| **F11** | Churn / Invalidation Rate| Invalidation event counters | Window $\ge 5$ s | Exact ($0\%$ error) |
| **F12** | Freshness / Stale Fraction| RFC 9111 stale-served counters | Window $\ge 5$ s | Exact ($0\%$ error) |

---

## 5. Candidate Evaluation Protocol & Disposition Rules

Every candidate policy is dispositioned under formal scientific criteria:

1. **Retain:**
   - Evaluated on **held-out workload families** ($\{W\text{-DI}, W\text{-SH}, W\text{-MX}, W\text{-HK}\}$) and **Dev-Eval Seeds $D$** (`[20001..=20060]`).
   - Statistically significant improvement over best static baseline: $\text{CI}_{\text{lower}} > \delta_{\text{min}}$.
   - Zero guardrail violations across all evaluation runs.
   - Overhead within budget: CPU overhead $\le 2\%$, hot-path latency adder $\le 100$ ns.
   - Deterministic replay verified bitwise.
2. **Narrow:**
   - Demonstrates statistically significant benefit only within an identifiable operating regime (e.g., high skew or specific origin degradation).
   - Documented boundary conditions and regime detector verified.
3. **Reject:**
   - Fails to outperform best static baseline by $\delta_{\text{min}}$, breaches guardrails, or fails overhead budget.
   - Documented as negative research finding; retained in lab testbed, excluded from production build.

---

## 6. Disjoint Seeds & Family Partitions

Strict partition enforcement:
- **Tuning Seeds ($T$):** `[10001..=10060]` — Exclusively for candidate algorithm development and tuning.
- **Dev-Eval Seeds ($D$):** `[20001..=20060]` — Exclusively for candidate disposition evaluation in M5.
- **Sealed Seeds ($S$):** `[30001..=30060]` — Strictly reserved for Phase 6 (Research Evaluation). Never accessed in M5.
- **Tuning Workload Families:** $\{W\text{-Z}, W\text{-FC}, W\text{-SC}\}$.
- **Held-out Workload Families:** $\{W\text{-DI}, W\text{-SH}, W\text{-MX}, W\text{-HK}\}$.

