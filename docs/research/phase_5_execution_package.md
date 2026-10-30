> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work against this package was executed and reported in [phase_5_completion_report.md](phase_5_completion_report.md) (agent-reported PASS; supported status machinery implemented, not integrated). No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 5 Execution Package — M5: Phantom Intelligence & PIE

**Status: PROPOSED — not authorized.** Two separately authorized parts: **M5a (machinery)** after G-M1 + G-M2 + the M4 lab core (T1 deterministic replay); **M5b (candidate intelligence)** after G-M4 **and G-RESEARCH** (prior-art closure, pre-registration frozen). Common rules: `phase_execution_packages_index.md` §4. This package refines `phantom_intelligence_phase.md` and builds on `pie_audit.md`; where they differ this package governs M5 execution.

## 1. Purpose and the five pillars (Phantom Intelligence is not a single controller)
**Phantom Intelligence** = observe → detect → predict (optional) → evaluate → adapt → explain, executed with hard safety limits. **PIE** is its evaluate/adapt/audit core. The phase delivers all five pillars and the execution/audit machinery around them; each pillar has its own evidence requirement:
| Pillar | Content | WPs | Evidence it must produce |
|---|---|---|---|
| 1 Workload intelligence | distributions, hot-key emergence, bursts, locality, miss amplification, origin health | M5-02 | detector accuracy vs ground truth (lab), error bounds, overhead |
| 2 Predictive intelligence | forecasts and predictions *only where measurably valuable* | M5-11, M5-12 | prediction quality vs naive baselines **and** downstream benefit, held-out |
| 3 Policy intelligence | admission, eviction, refresh, coalescing, stale-serving, origin-load control | M5-03, M5-09, M5-10 | candidate dispositions vs strong static/non-ML baselines |
| 4 Safe adaptation | shadow/canary, guardrails, rollback, cooldown, operating modes, kill switch | M5-04, M5-05, M5-07 | rollback proof, planted-effect validation, fault injection |
| 5 Evaluation & explanation | audit trail, explainability, overhead, replay, generalization | M5-06, M5-07, M5-13 | auditable records, deterministic replay, overhead budgets, held-out results |
**Non-goals:** declaring a research contribution (M6), claiming novelty, shipping any mode above `off` as default (owner decides at M6/M7), UI polish (M8), rebuilding baselines (M4).

## 2. Starting state (evidence; PROVISIONAL items re-verified at M5-00)
**Verified before Phase 1 (`pie_audit.md`):** the only PIE code is `src/control/pie.rs` (124 lines): threshold trigger on the *active* policy's aggregate `hit_rate*100 − p99*0.1` (commit if >80; rollback only for candidate `S3-FIFO` if <50), `expected_utility` literal 85.0, no canary/windows/confidence/exposure, `active_policy` not updated on rollback, unbounded trace `Vec`, `pie: None` in `main.rs`, `CountMinSketch` unused, telemetry `key_hash` zero. Historical design values ([HIST], `docs/implementation/WP2.md`, never verified or implemented): baseline window 60 s, canary window 30 s, ≥10,000 canary requests, +10% P99 regression bound, 95% confidence, cooldown 60 s, exposure ≤ 5%; utility weights w1…w4 unspecified. **Expected from earlier phases (PROVISIONAL):** M1 truthful telemetry with provenance, real `key_hash`, origin metrics, byte accounting, reference policies; M2 freshness/stale serving with hard cap, eviction trait with `rebuild_from`, µs latency, concurrency-verified cache; M4 lab (T1 replay, T2), strong baselines, workload families with a frozen tuning/held-out split, reproduction procedure; M3 only if cluster scenarios matter.
**Treat historical thresholds, 85.0, and "simulated regression" as prototype constants, not validated design.**

## 3. Operating modes and defaults
`PHANTOM_INTELLIGENCE=off|observe|advisory|canary|active`, **default `off`**. `observe`: signals and detectors only. `advisory`: candidates evaluated (shadow; canary disabled), decisions *recorded not applied*. `canary`: limited live exposure with automatic revert. `active`: committed policies applied (still bounded by hard caps and holdback). Mode changes occur only via authenticated admin configuration/API; automatic transitions are **downward only** (active→advisory on guardrail trip; any→off on kill switch or watchdog). Live modes refuse inputs whose provenance is `simulated` or `unavailable`. The executor in the cache enforces hard caps (staleness cap, byte budget, max background refresh, origin-concurrency cap) irrespective of any decision.

## 4. Work packages
| ID | Title (legacy) | Part | Depends on |
|---|---|---|---|
| M5-00 | Entry verification, PIE re-audit, pre-registration of detector/candidate criteria | a | gates |
| M5-01 | Architecture and interface specification (WP-30) — **owner approval before code** | a | M5-00 |
| M5-02 | Signal and workload-intelligence pipeline (WP-31) | a | M5-01 |
| M5-03 | Shadow (counterfactual) evaluator (WP-32) | a | M5-02, reference policies |
| M5-04 | Policy execution layer and safe fallback (WP-34) | a | M5-01, M2 eviction trait |
| M5-05 | PIE decision engine v2 (WP-33) | a | M5-02/03/04 |
| M5-06 | Decision audit and explainability (WP-35) | a | M5-05 |
| M5-07 | Safety, overhead, planted-effect and replay verification (WP-37) | a | M5-05/06 |
| M5-08 | PIE-in-the-lab integration (deterministic decision traces) | a | M5-05, M4-01 |
| — | **Internal gate G-M5a** (machinery PASS) | | M5-02…08 |
| M5-09 | Statistical policy candidates (WP-36a) | b | G-M5a, G-M4, G-RESEARCH |
| M5-10 | Origin-aware control candidate family (WP-18) | b | G-M5a, G-M4, G-RESEARCH |
| M5-11 | Predictive candidates (WP-36b) | b | M5-09 |
| M5-12 | ML selector (WP-36c) — ML gate | b | M5-09/11 evaluated |
| M5-13 | Candidate evaluation and dispositions (WP-19a) | b | M5-09…12 |
| M5-14 | Integration of retained capabilities behind flags (owner-reserved) | b | M5-13 |
| M5-15 | Documentation and handoff to M6/M7/M8 | b | all |

### M5-00 Entry verification and pre-registration
CER-2 plus: re-run the PIE audit on the then-current tree (what changed since `pie_audit.md`); record that legacy `pie.rs` remains non-authoritative (a labeled **floor baseline** `BL-PIE-legacy` in the lab only); freeze before any detector/candidate result is seen (file hashed in the log): the detector accuracy targets (as *reporting metrics*, thresholds set from lab ground truth only after the first pilot and frozen before the evaluation seeds), the candidate evaluation protocol and disposition rules (below), the minimum effect δ_min, the guardrail list, the canary design parameters (initial values = [HIST]; final values come from a power analysis on M4 variance — owner O5-3), and the held-out families.

### M5-01 Architecture and interfaces (spec first)
Deliver `docs/research/intelligence_architecture.md` + compilable type/trait skeletons (no behavior): `SignalWindow` (features with provenance and validity: sample count, window), `Detector`, `Predictor`, `PolicyCandidate{id, kind, params, bounds, constraints, expected_effect}`, `Evaluator` (shadow/canary), `Verdict`, `DecisionRecord`, `PolicyExecutor{apply, current, revert_to_safe}`. Specify: threading (intelligence on separate tasks off the serving path, bounded channels, drop-newest signal sampling), failure semantics per interface, kill switch, watchdog, config surface, API additions (proposed endpoints `/api/v1/intelligence/{status,decisions}` and admin mode control — **new endpoints = owner-reserved O5-1**), memory bounds (every structure fixed-size, byte-accounted), per-feature overhead model. **Acceptance:** research-lead review + owner approval (O5-1) before M5-02…; every interface has a failure paragraph; hot path unchanged.

### M5-02 Signal and workload-intelligence pipeline (Pillar 1)
Feature catalog (each with definition, estimator, error bound, window, provenance=`derived`, validity rule, overhead):
| ID | Feature | Estimator (candidate) | Evaluation vs ground truth (lab T1/T2) |
|---|---|---|---|
| F1 | request rate | EWMA over fixed windows | bias/variance, lag |
| F2 | windowed hit ratio, miss cost | counter snapshots | exactness |
| F3 | per-key frequency / heavy hitters | aged Count-Min (depth d, width w; error εN w.p. 1−δ) + Space-Saving (k counters) | precision/recall at k, frequency error vs exact |
| F4 | skew (Zipf α) | MLE/regression on heavy-hitter counts with CI | |α̂−α|, CI coverage |
| F5 | working-set / miss-ratio curve | spatially sampled shadow (SHARDS-style), rate R | MRC error vs full simulation |
| F6 | burst / level shift | CUSUM or Page–Hinkley on rate and on key-distribution distance | detection delay, false-alarm rate (ARL₀) |
| F7 | hot-key emergence | growth-rate test on heavy-hitter counts | lead time, precision/recall |
| F8 | locality | sampled reuse-distance histogram | histogram distance |
| F9 | miss amplification | origin requests ÷ client misses; coalesced waiters | exactness |
| F10 | origin health | latency quantiles (histogram snapshot differences), error rate, in-flight; capacity/utilization only as `inferred` | agreement with lab origin log |
| F11 | invalidation/churn rate | counters | exactness |
| F12 | freshness/stale metrics | M2 counters | exactness |
**Requirements:** features computed off the hot path from sampled/aggregated data; fixed memory (documented formula); each feature declares validity (insufficient data ⇒ `unavailable` with reason); no feature uses `simulated`/`unavailable` inputs in live modes. **Tests/evidence:** accuracy tables on seeded lab workloads with ground truth (T1) and T2 spot checks; error-bound conformance (empirical error ≤ bound at the stated confidence); detector ROC/ARL; naive baselines (exact counting over short windows; fixed thresholds) reported alongside; overhead benchmarks (signals-only vs off) with Phase 1 harness; memory bound test. **Acceptance:** detectors meet the frozen reporting metrics or are marked `NARROWED`/`NOT USABLE` for their purpose (a detector that does not beat the naive baseline is not used by candidates).

### M5-03 Shadow evaluator (Pillar 3 support)
Miniature simulations of candidate eviction/admission policies on a spatially sampled key stream (key-hash threshold R, cache scaled by R); outputs miss ratio and origin-cost-weighted miss cost with intervals; deterministic given seed/stream. **Tests:** agreement with full simulation on lab workloads (error distribution per workload family; failures reported), determinism, bounded memory, overhead. Eviction/admission candidates are evaluated *only* in shadow (no exposure risk); refresh/stale/origin-control candidates need canary (M5-05).

### M5-04 Policy execution layer and safe fallback (Pillar 4)
In `cache.rs` (independent review required): a policy handle swapped atomically by the executor; eviction via the M2 trait with `rebuild_from` for switching (bounded work, performed off the hot path or in bounded slices); canary partitioning by **key-hash buckets** (B buckets, e.g., 1,024) stratified so heavy-hitter keys are split evenly across arms; **hard caps** enforced here (staleness cap, byte budget, max background refresh, origin-concurrency cap); `revert_to_safe` restores the recorded safe configuration; watchdog: if the intelligence loop stalls/panics, the executor reverts and the mode drops to `advisory`/`off`. **Invariants:** pinned states never evicted; accounting invariants I1–I8 hold across switches; fencing/epoch semantics unchanged; arm assignment is a pure function of key hash (stable); revert restores a configuration equal (hash) to the recorded one. **Tests:** swap under concurrent load (stress + model tests from M2), rebuild correctness, revert equality, watchdog test (injected stall/panic), arm-assignment uniformity/stratification tests, hard caps hold under adversarial candidates (a candidate that tries to exceed caps is clamped and logged).

### M5-05 PIE decision engine v2 (Pillar 3/4)
**Objective form (constraint-first, [PROVISIONAL]):** maximize a declared primary objective (e.g., origin-cost-weighted miss cost reduction, or origin peak concurrency) subject to hard constraints (client p99 regression ≤ bound, staleness cap, memory budget, error rate, zero cap violations); optional weighted scalarization with *explicit, versioned weights* (no hidden constants). **Evaluation paths:** shadow (eviction/admission) and canary (refresh/stale/origin control). **Canary protocol:** one candidate at a time; exposure fraction f of buckets to the candidate arm, matched control buckets, a permanent **holdback arm** (e.g., 1%) always on the baseline policy to detect long-term regressions; **arm-labeled telemetry** (every counter/histogram has an `arm` dimension; baseline arm metrics are computed only from baseline-arm traffic); statistics: fixed windows (no peeking) or alpha-spending, block bootstrap with block length from the autocorrelation function, minimum sample size from the power analysis (initial [HIST]: 10,000 requests/arm), periodic **A/A tests** to estimate the noise floor; decision rule pre-registered: *commit* iff the lower CI bound of the improvement > δ_min **and** all guardrails held for the whole window; *rollback* immediately on any hard-guardrail breach, or if the upper CI bound of regression exceeds the threshold; otherwise *no-action*. **Interference control:** origin load is shared — origin-side metrics are evaluated as system-level time series with before/during/after comparison and exposure limits; the candidate-arm effect on control-arm latency is monitored. **Anti-drift:** cooldown after any commit/rollback ([HIST] 60 s, to be re-derived), hysteresis, maximum switches per hour, exploration budget, periodic revalidation against the holdback, bounded decision state (ring buffers, fixed-size sketches). **Failure handling:** any exception ⇒ `Fault` verdict ⇒ revert + mode downgrade + audit record. **Tests:** decision-table tests (every verdict reachable for the stated reason), statistical-correctness tests on synthetic arms with known effects (false-commit/false-rollback rates measured), guardrail breach tests, flapping/oscillation tests under oscillating workloads, mode-transition tests (only downward automatic), A/A calibration test (false-commit rate at δ=0 within the preregistered alpha).

### M5-06 Decision audit and explainability (Pillar 5)
Bounded, byte-accounted decision store; `DecisionRecord` fields: id, time, mode, trigger, inputs with provenance and validity, candidate set with rejection reasons, evaluation method, estimated effect with interval, observed effect, guardrail states, verdict, exposure and arm statistics, config hashes before/after, overhead. Proposed endpoints (O5-1) and a minimal truthful panel that reads only real records (no fabricated "COMMITTED"); export format for the lab/M6. **Acceptance:** a reviewer can reconstruct why a decision was made from the record alone (checked on ≥ 10 recorded decisions by a second reader).

### M5-07 Safety, overhead, planted-effect and replay verification
(1) **Planted-effect validation:** synthetic candidates with *known* effects (e.g., a policy that adds a controlled miss-ratio penalty or origin-load reduction of x%) run through shadow/canary: measure detection power, false-commit and false-rollback rates, time-to-decision. (2) **Rollback proof:** trigger → restored state equality, latency bound measured in T2 (PROVISIONAL target ≤ 1 s; set after measurement, O5-6), rollback under load and under panic/stall injection. (3) **Fault injection** into the intelligence layer (panic, stall, poisoned/missing signals, clock jumps, unavailable provenance) ⇒ serving path unaffected, safe policy restored. (4) **Overhead:** three configurations — off, observe, full — on the Phase 1 Core items and the end-to-end item: added hot-path latency (paired ABAB, VALID class), decision-loop CPU, resident bytes of all intelligence structures; budgets proposed from measurements for owner approval (O5-6). (5) **Deterministic replay:** same seed + trace ⇒ byte-identical decision records (T1). (6) **Stacking/oscillation test:** multiple levers active together (e.g., admission + stale window + concurrency cap) under oscillating origin conditions: no sustained oscillation beyond the preregistered bound. (7) **Adversarial inputs:** hot-key flooding, workload oscillation designed to induce flapping, signal poisoning.

### M5-08 PIE in the lab
Lab gateway runs the same engine/executor logic against T1 (virtual time, deterministic traces) and T2; decision traces stored in `lab-results/` with manifests; equivalence tests between lab-hosted and product-hosted engine on the conformance scenarios (same decisions given the same recorded signals).

### G-M5a (machinery gate; internal)
PI-1…PI-10 machinery subset PASS (functional integration in a running gateway built from the repo with the engine constructed in `main()` in `off`/`observe`/`advisory`; provenance gating; decision correctness; safe failure; reproducibility; overhead measured; explanations). Legacy stub out of product paths. **No candidate may be evaluated for disposition before G-M5a.**

### M5-09 Statistical candidates (WP-36a) — hypotheses with kill criteria
Each candidate: hypothesis, mechanism, parameters with bounds, expected signature, **kill criterion** (disposition rule), baseline set. Candidates: (a) **shadow-selected eviction** among reference-quality policies (SIEVE/S3-FIFO/LRU/TinyLFU) using M5-03; (b) **frequency-based admission** (TinyLFU-style) and size-aware admission (probabilistic e^{−size/c}); (c) **hot-key-aware pinning / refresh-ahead** using F3/F7; (d) **adaptive coalescing limits**. Baselines: best static, the corresponding simple heuristic, BL-PIE-legacy as floor, and relevant M4 baselines.

### M5-10 Origin-aware control (WP-18) — central research candidate family
Hypothesis H1 (`hypothesis_and_experiments.md`): measured origin health (F10) drives, within hard caps, (i) stale window extension, (ii) refresh scheduling, (iii) miss admission/shedding, (iv) origin-concurrency cap, (v) coalescing limits, reducing origin load and tail latency versus tuned static baselines in degradation scenarios. **Build order:** clamped single mechanisms first (miss shedding, stale extension, concurrency cap), each evaluated alone; a feedback controller only if ablations show static tuning insufficient; **stacking tests** (oscillation) mandatory; lives in the lab gateway until dispositioned. Controller design choices (AIMD/gradient/PI) are recorded as alternatives with reasons; stability argument (e.g., bounded gain, hysteresis) documented; hard staleness and origin-load caps are enforced by the executor, not by the controller.

### M5-11 Predictive candidates (WP-36b)
Rate forecasting (naive persistence, EWMA, Holt–Winters, small AR), hot-key emergence prediction (F7-based), next-key prediction (order-1/2 Markov) for prefetch, origin-degradation early warning (trend of F10 quantiles). **Evaluation (both required):** *prediction quality* (MASE vs persistence for forecasts; precision/recall/lead time and calibration for classifiers; held-out workload family) **and** *downstream benefit* (does acting on the prediction reduce origin load/latency versus acting without it, including prefetch waste). A predictor that does not beat the naive baseline on held-out data is rejected regardless of training accuracy.

### M5-12 ML selector (WP-36c) — gated
Allowed only if all hold: (1) a non-ML statistical candidate has been evaluated and the ML candidate beats it by the preregistered margin (CI) on held-out workload families **different from training families**; (2) inference cost within the overhead budget; no training on the serving path; offline reproducible training (fixed seeds, versioned and hashed artifact, data manifests); (3) drift/out-of-distribution detection with automatic fallback; (4) identical shadow/canary/rollback protection as any candidate; (5) explainability sufficient for the record (selected configuration + evidence + feature attribution). Leakage checks (family-level splits, time-ordered splits). New dependencies (ML libraries) are runtime/dev dependencies ⇒ owner-reserved (O5-5); the default implementation is dependency-free (small trees/linear models coded in-house). Literature context (SCION, LAH/S4-FIFO, Darwin, CACHEUS, LRB) says this space is crowded: ML is optional and lowest priority.

### M5-13 Candidate evaluation and dispositions (WP-19a)
For each candidate: T1 full matrix and T2 prioritized subset from M4 (tuning families and **tuning seeds T** used for design; **held-out families with development-evaluation seeds D for the disposition — never the sealed seeds S reserved for M6**), ≥ 30 seeds (T1) / ≥ 10 repetitions (T2), against the preregistered baselines; metrics: primary objective, constraints, overhead, prediction quality (where applicable); statistics per M6 conventions (paired seeds, bootstrap CIs, effect sizes, Holm correction across candidates). **Disposition rules (preregistered):** *Retain* iff benefit CI excludes 0 and exceeds δ_min over the best applicable baseline, all constraints hold in every trial, overhead within budget, replay deterministic, fault-injection PASS. *Narrow* iff the benefit holds only in an identified regime (documented boundary, with the regime detector available as a feature). *Reject* otherwise (recorded as a negative result; stays in the lab). Dispositions feed `claims_ledger.md`.

### M5-14 Integration of retained capabilities (owner-reserved O5-4)
Only retained/narrowed capabilities are compiled into the product build, each behind its own flag, default off. **Acceptance:** equivalence with the lab implementation on conformance scenarios; overhead re-measured in the product build; no change to default behavior.

## 5. Acceptance criteria and exit gate G-M5 (extends `phantom_intelligence_phase.md` PI-1…PI-10)
| ID | Criterion | Blocking |
|---|---|---|
| AC5-1 | E0 complete; PIE re-audit; preregistration frozen (hash) before results | yes |
| AC5-2 | Architecture spec approved (O5-1); no hot-path change in spec | yes |
| AC5-3 | Detectors: accuracy tables vs ground truth; each beats its naive baseline or is marked unusable | yes |
| AC5-4 | Shadow evaluator error distribution reported; determinism PASS | yes |
| AC5-5 | Executor invariants + hard caps + revert equality + watchdog PASS | yes |
| AC5-6 | Decision engine: decision-table PASS; planted-effect detection power and false-decision rates reported against preregistered bounds; A/A calibration PASS | yes |
| AC5-7 | Rollback proof: equality, latency (measured), under load and under injected faults | yes |
| AC5-8 | Audit: reconstructability check by a second reader; bounded store; no fabricated records | yes |
| AC5-9 | Overhead measured (off/observe/full), budgets approved (O5-6), within budget or owner-accepted | yes |
| AC5-10 | Deterministic replay: identical decision records | yes |
| AC5-11 | Provenance gate: live modes refuse simulated/unavailable inputs (negative tests) | yes |
| AC5-12 | Every candidate has a disposition with held-out evidence (or NOT RUN with owner-visible reason) | yes |
| AC5-13 | `PHANTOM_INTELLIGENCE` defaults to `off`; legacy stub not in any product path; `pie: None` replaced by the v2 engine constructed in `main()` in the off/observe/advisory path | yes |
| AC5-14 | No unapproved dependency; ledger clean | yes |
**G-M5 passes** when all blocking criteria PASS (or have written waivers) and the owner accepts after review. The product may then ship `observe`/`advisory`; **`canary`/`active` defaults are not decided until G-MVP.**

## 6. Measurement and telemetry integrity
Arm-labeled telemetry; baseline-arm metrics never include candidate traffic; holdback arm continuous; candidate evaluation inputs record provenance; all decision inputs are `measured`/`derived`; any `inferred` input (e.g., capacity estimate) is labeled and never used as a hard guardrail. Results classes per CER-5; overhead comparisons paired and interleaved; T1 decision traces are *simulated* evidence and cannot support wall-clock claims.

## 7. Risk register
| Risk | Containment |
|---|---|
| "AI branding" without benefit | kill criteria, naive baselines, ML gate, held-out dispositions |
| Policy drift / flapping | cooldown, hysteresis, switch limits, holdback, oscillation tests |
| Canary contamination (shared origin, popularity imbalance) | stratified buckets, arm-labeled telemetry, A/A tests, system-level origin metrics, exposure limits |
| Statistical misuse (peeking, autocorrelation, multiplicity) | fixed windows/alpha-spending, block bootstrap, Holm, planted-effect validation |
| Intelligence layer harms serving path | off-path execution, bounded queues, watchdog, hard caps in executor |
| Over-fitting to lab families | frozen split, held-out dispositions |
| Overhead creep | budgets, off/observe/full measurements, fixed-size structures |
| Prior-art gap → mis-claimed novelty | G-RESEARCH gate; no claim in M5 |
| Large `cache.rs` change | independent review, snapshot, M2 model tests reused |

## 8. Authorized files / prohibitions
May modify with ledger: new modules (e.g., `src/intelligence/**` or the location fixed in M5-01), `src/core/cache.rs` (executor hooks only, reviewed), `src/main.rs` (construction/config), `src/api/server.rs` (proposed endpoints after O5-1), lab code, `benches/**`, `tests/**` (new), docs, `ui/**` (minimal Intelligence panel, truthful). **Must not modify:** `src/security/*`, `src/dist/*`, M2 freshness semantics, the hard-cap definitions except via owner decision; `pie.rs` only for banner comments. CER-3 prohibitions apply.

## 9. Owner-reserved decisions
O5-1 architecture approval and any new API endpoints/admin control · O5-2 default mode and who may change it · O5-3 canary/statistics parameters after power analysis (exposure, windows, min samples, alpha, δ_min, cooldown) · O5-4 integration of retained capabilities into the product · O5-5 any ML library or other dependency · O5-6 overhead budgets and rollback-latency bound · O5-7 held-out family confirmation · O5-8 enabling `canary`/`active` by default (not in this phase). **Agents may decide:** internal data structures (within the memory bounds), feature estimator implementation details, test structure, report formatting.

## 10. Stop conditions
M5-01 not approved; a detector or shadow evaluator cannot meet determinism; executor invariants fail; planted-effect validation shows the decision rule's false-commit rate above the preregistered bound; rollback cannot be proven; intelligence overhead exceeds the budget with no mitigation; any evidence that candidate telemetry contaminated baseline telemetry (INVALID experiments); G-RESEARCH not passed when M5b is about to start; need for a dependency or endpoint not approved.

## 11. Completion report
CER-8 plus: pillar-by-pillar evidence table, detector accuracy appendix, planted-effect validation results, rollback proof, overhead tables, decision-record samples (reconstructed by a second reader), dispositions table with held-out evidence, list of retained/narrowed/rejected capabilities, legacy-PIE retirement record, open questions for M6, next-phase proposal only.
