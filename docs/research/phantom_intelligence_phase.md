# Milestone 5 — Phantom Intelligence & Policy Intelligence Engine (PIE)

**Status: PROPOSED — requires owner approval of the revised roadmap before it is a plan of record.** Not authorized for implementation. Depends on `pie_audit.md` (what exists), `mvp_and_milestones.md` (placement, MVP), `hypothesis_and_experiments.md` and `literature_review.md` (research), `phase_1_execution_package.md` (prerequisite telemetry).
Evidence tags as elsewhere: numbers marked **[HIST]** are the historical design values recovered from `docs/implementation/WP2.md` (design intent, never verified or implemented); **[DECISION]** = requires owner approval or measurement before being fixed; **[PROPOSAL]** = my recommendation.

## 1. Conceptual architecture (to be reconciled with code, not claimed as existing)
```
 Cache execution layer ──(requests, outcomes, origin fetches)──► SIGNAL LAYER  (Phantom Intelligence: Observe)
   ▲  enforces hard caps & invariants,                            telemetry with provenance → windowed features
   │  serves requests, can always revert                          (rates, per-key frequency, skew, working set,
   │                                                               origin health, invalidation/churn, shifts)
   │                                                                    │
   │                                                              DETECTORS  (hot keys, workload shift,
   │                                                               origin degradation / recovery)
   │                                                                    │
   │                                                              PREDICTORS (optional, only if they beat
   │                                                               simple baselines)
   │                                                                    │
 POLICY EXECUTOR ◄── approved PolicyConfig ◄── PIE  (candidate generation → evaluation (shadow / canary) →
 (atomic apply, safe fallback)                      constrained utility → verdict → cooldown / hysteresis)
                                                         │
                                                  AUDIT & EXPLAINABILITY (decision records, API, UI, export)
 Phantom Lab ──► deterministic workloads, replay, origin models, baselines, evidence (offline and CI)
```
- **PhantomCache** = whole system. **Phantom Intelligence** = Observe → Detect → (Predict) → Evaluate → Adapt → Explain. **PIE** = the Evaluate/Adapt/Audit core (policy evaluation, canary, verdict). **Policy implementations** = eviction, admission, refresh, coalescing limits, stale-serving windows, origin-concurrency caps — behind one interface. **Execution layer** = `PartitionedCache` + a *policy executor* that applies only approved configurations within hard limits.
- **Hard rule:** the execution layer never trusts the intelligence layer for correctness. Staleness caps, origin-load caps, memory budgets and fencing invariants are enforced in the execution layer irrespective of any PIE decision.
- **Input rule (provenance gate):** PIE acts on a live gateway only on signals whose provenance is `measured` or `derived` from measured. A `simulated` or `unavailable` input forces `NoAction` (live) or labels the decision `SIMULATED` (lab). PIE is never activated against the synthetic dashboard metrics.

## 2. Objective
Deliver a real, auditable intelligence path from trustworthy signals to policy evaluation to executed (or advisory) decisions — integrated in the running gateway, verifiable in the lab, safe on failure, explainable to an operator — and determine by evidence which intelligent capabilities to retain, narrow, or reject. Not every capability needs ML; ML is admitted only through the gate in §6.

## 3. Entry dependencies and staging
- **Entry (M5a):** G-M1 (truthful telemetry, origin metrics, byte accounting, baselines, reference policies) **and** G-M2 (freshness semantics WP-14 with a hard staleness cap; eviction decision from WP-08; PIE stub quarantined WP-22a) **and** WP-15 lab core (deterministic replay) — M5a may therefore overlap M4 on disjoint files.
- **Entry (M5b):** G-M4 (lab, controllable origin, reviewed static baselines) **and** G-RESEARCH (RT-0 closed incl. the remaining sweep gaps; hypotheses pre-registered).
- **Exit gate G-M5** (§9).
- **Sequence consequence:** Phantom Intelligence is inserted as a new **Milestone 5**; former M5 ("Research Contribution, Evaluation & MVP Gate") becomes **M6** and former M6 ("Productization & Research Release") becomes **M7**. M1–M4 and all WP ids are unchanged; WP-18/19/20/22 are re-mapped as listed in `mvp_and_milestones.md` §6.

## 4. Work packages (new ids WP-30…WP-37; WP-18/19a/22 re-mapped)
Each package is specified in full (all 17 fields) at the M5a/M5b gate; the table below fixes scope, design decisions, tests and acceptance now.

### M5a — Machinery
**WP-30 Architecture & interface specification** (docs + type skeletons, behind a Cargo feature or isolated module; no behavior change). Deliver `docs/research/intelligence_architecture.md` and compilable traits/types: `SignalWindow` (feature vector with per-field provenance), `Detector`, `PolicyCandidate{id, kind (Eviction|Admission|Refresh|Coalescing|StaleWindow|OriginConcurrency), params, constraints, expected_effect}`, `Evaluator` (shadow or canary), `DecisionRecord`, `PieVerdict{NoAction{reason}, StartCanary, Commit, Rollback{reason}, Fault}`, `PolicyExecutor{apply, current, revert_to_safe}`. Decide: where the intelligence loop runs (separate task(s) off the serving path), bounded queues, timeouts, kill-switch, config surface. *Acceptance:* reviewed by me; every interface has a failure semantics paragraph; no code in the hot path.
**WP-31 Signal & feature pipeline** (consumes Phase-1 schema v2 data): windowed features from real counters — request rate, hit/miss, latency quantiles, origin latency/error/in-flight (WP-02b), **per-key frequency via an aged Count-Min sketch fed with real `key_hash`**, heavy hitters (Space-Saving or CMS+heap; choose by benchmark), skew estimate, working-set / miss-ratio-curve estimate by spatially sampled keys (SHARDS-style), invalidation/churn rate, shift detection (distribution distance over windows; CUSUM/Page-Hinkley candidates). *Tests:* accuracy vs exact computation on seeded workloads (documented error bounds), provenance propagation, bounded memory, determinism under replay. *Overhead:* measured with intelligence off / signals only / full (WP-37).
**WP-32 Shadow (counterfactual) evaluator:** miniature simulations of candidate eviction/admission policies on a sampled key stream (Waldspurger et al., ATC'17 technique), driven by the reference policies (WP-05) with sampling rate and metric definitions fixed in advance; estimates miss ratio and *origin-cost-weighted* miss cost (using measured origin latency per class). Shadow evaluation has **no exposure risk** and is the preferred mode for eviction/admission. *Tests:* agreement with full simulation on seeded traces within a stated error; deterministic.
**WP-33 PIE decision engine v2** (replaces the legacy `pie.rs` logic; legacy retained only as a labeled baseline): candidate generation; **constraint-first utility** — maximize a declared objective (e.g., origin-weighted miss cost, or origin load) subject to hard constraints (staleness cap, origin-load cap, P99 SLO, memory); the historical weighted-sum `U = w1·Hit − w2·P99 − w3·BackendLoad − w4·ResourceCost` is retained as an optional scalarization with documented weights [DECISION]; two evaluation modes — *shadow* (WP-32) and *live canary* (key-hash-partitioned exposure) for policies that act on origin load/staleness and cannot be shadowed; statistics: paired/blocked comparisons with autocorrelation-aware intervals, minimum effect size, fixed windows or alpha-spending to avoid peeking, correction across candidates; verdicts and cooldown/hysteresis; kill switch; *advisory* vs *active* mode (advisory records what it would do). *Historical defaults [HIST][DECISION]:* baseline window 60 s, canary window 30 s, ≥10,000 canary requests, +10% P99 regression bound, 95% confidence, cooldown 60 s, exposure ≤ 5%. Note the shard-granularity issue: the HTTP cache has 8 shards (`main.rs:65`), so shard-level canaries cannot go below 12.5% exposure — canaries must partition by key-hash buckets inside shards, or the default shard count must change (design item for WP-34). *Tests:* decision-table tests (each verdict reachable for stated reasons), statistical-correctness tests on synthetic distributions with known effect (false commit/rollback rates measured), flapping tests, replay determinism.
**WP-34 Policy execution layer & safe fallback** (touches `src/core/cache.rs`; independent review required): replace the `EvictionPolicy` enum matching with a policy handle that the executor swaps atomically; canary partitioning by key-hash buckets; hard caps enforced here; a **safe policy** to which the executor reverts on rollback, fault, kill switch, or watchdog timeout; bounded work per eviction preserved; bookkeeping consistent (fixes the legacy `active_policy` desync class of bug). *Tests:* invariants (never evict active ops; fencing unaffected), policy swap under concurrent load (stress/`loom` where feasible), revert-on-fault, accounting from WP-06 preserved.
**WP-35 Decision audit & explainability:** bounded decision-record store (ring, byte-bounded), API (`/api/v1/intelligence/decisions`, `/status`), 8-field historical record extended with: inputs+provenance, candidates considered and rejected (with reason), estimated vs observed effect with intervals, constraints and their status, mode (shadow/canary/active/advisory), exposure, outcome, guardrail events; UI "Intelligence" panel showing only real records (replacing the hard-coded inspector), experiment-export format. *Acceptance:* a reviewer can reconstruct why each decision was made from the record alone.
**WP-37 Intelligence safety, overhead & replay verification:** fault injection into the intelligence layer (panic, stall, malformed/poisoned signals, clock jumps, missing provenance) ⇒ serving path unaffected and safe policy restored; overhead benchmarks in three configurations (off / signals-only / full) on the serving path and for decision CPU/memory; adversarial-workload tests (hot-key flooding, workload oscillation designed to induce flapping); deterministic replay: same seed + trace ⇒ byte-identical decision records.

### M5b — Candidate intelligence (each a *hypothesis with a kill criterion*)
**WP-36a Statistical candidates (no ML):** shadow-selected eviction policy among the reference-quality policies (WP-05/08); frequency-based admission (TinyLFU-style) and hot-key-aware pinning/admission; aged-sketch-driven adaptive refresh-ahead for hot keys; coalescing limits. 
**WP-18 Origin-aware control (central research candidate; id preserved):** measured-origin-health-driven stale window, refresh scheduling, miss admission and origin-concurrency cap under explicit staleness and origin-load caps (hypothesis H1 in `hypothesis_and_experiments.md`); starts with clamped mechanisms; a feedback controller only if ablations show static tuning insufficient; must be tested for oscillation when stacked with other levers (Charapko et al. report oscillation from stacked controllers).
**WP-36b Predictive candidates:** short-horizon rate forecasting (EWMA/Holt) for pre-warming, next-key prediction (Markov/n-gram) for prefetch where access patterns are sequential — each must beat a no-prediction baseline on origin load or latency with prefetch waste accounted.
**WP-36c ML selector (gated, see §6).**
**WP-19a Candidate evaluation:** for each candidate, Tier 1 + Tier 2 runs on the lab with the static/heuristic baselines of WP-17, ≥30 seeds, pre-registered criteria, producing a **disposition**: *retain* / *narrow (state regime)* / *reject*. Results feed `claims_ledger.md`.

## 5. Evidence and disposition rules (pre-registered before M5b runs)
- **Retain** a capability only if: benefit over the best applicable baseline has a 95% CI excluding zero **and** exceeds a pre-registered minimum effect [DECISION: set after M4 pilots]; all hard constraints hold in every trial; intelligence-layer overhead is within budget (§7); decisions are reproducible under replay; failure-injection passes.
- **Narrow** if the benefit holds only in an identified regime (e.g., skew above X, origin brownouts only): ship enabled only in that regime or default-off with documentation.
- **Reject** otherwise; record the negative result with the evidence and conditions. Rejected capabilities remain in the lab, not in the product default path.
- Baselines for every candidate: best static configuration (oracle-tuned), the simple heuristic equivalent, and the legacy threshold PIE (as a floor), plus the strong baselines listed in `hypothesis_and_experiments.md` §3.

## 6. Where ML is allowed (no AI branding)
ML (e.g., gradient-boosted or online-linear policy/parameter selectors, learned eviction) is admitted only if **all** hold: (1) a non-ML candidate (statistical or heuristic) has been evaluated first and the ML candidate beats it by a pre-registered margin with CI on held-out traces from a *different* workload family; (2) inference cost within the overhead budget, no training on the serving path (offline reproducible training, fixed seeds, versioned model artifact); (3) drift/out-of-distribution detection with automatic fallback; (4) the guard that bounds worst-case harm (shadow evaluation + canary + rollback applies identically to ML candidates); (5) explainability sufficient for the decision record (feature attributions or the selected configuration and the evidence). Literature context: SCION, LAH/S4-FIFO, Darwin, CACHEUS, LRB (see `literature_review.md`) show the field is crowded; the ML candidate is therefore a lower-priority, optional capability, not the thesis.

## 7. Acceptance criteria for the milestone (objective; numeric thresholds only where justified)
| ID | Criterion | Evidence | Numeric threshold |
|---|---|---|---|
| PI-1 Functional integration | In a running gateway built from the repo, signals → candidates → evaluation → audited verdict → executor action (or advisory record) works end to end, with PIE constructed in `main()` (not only in tests) | integration test using a real process + lab origin; recorded demo run with decision records | — |
| PI-2 Data provenance | Every PIE input carries provenance; live PIE refuses `simulated`/`unavailable` inputs; the decision record lists inputs and provenance | contract tests incl. negative cases | — |
| PI-3 Decision correctness | Each verdict (NoAction/Canary/Commit/Rollback) is produced for the right reason in deterministic scenarios; statistical false-commit and false-rollback rates measured on synthetic data with known effects | decision-table + simulation tests | false-decision rate bounds **[DECISION]** after measurement |
| PI-4 Safe failure | Kill switch; intelligence panic/stall/poisoned input ⇒ safe policy restored, serving path unaffected; cooldown/hysteresis prevent flapping under oscillating workloads | WP-37 fault-injection suite | restore time bound **[DECISION]** after measurement |
| PI-5 Baseline comparison | Every retained capability compared with applicable baselines (§5) | WP-19a reports with CIs | min effect **[DECISION]** |
| PI-6 Reproducibility | Same seed + trace ⇒ identical decision records; manifest for each run | replay tests | exact equality |
| PI-7 Overhead | Hot-path cost (intelligence off vs signals-only vs full) and decision CPU/memory measured with CIs; memory bounded in bytes | B-suite variants on a fit platform | budgets **[DECISION]** — proposal: hot-path p50 added latency whose CI upper bound stays below the platform noise bound established in M1; decision loop CPU and resident bytes set after measurement |
| PI-8 Explainability | UI/API show inputs, candidates, estimated vs observed effect, constraints, guardrail events, outcome; operator task "why did the policy change?" answerable without logs | scripted usability check | — |
| PI-9 Hard caps | Staleness cap and origin-load cap enforced by the execution layer regardless of PIE; property-tested | tests | zero violations |
| PI-10 Honesty | Dispositions (retain/narrow/reject) recorded for every candidate in `claims_ledger.md`; no placeholder or simulated decision path in the product build | ledger + grep for legacy stubs | — |

## 8. Risks and mitigations
- **Over-engineering / AI branding:** statistical-first ordering, kill criteria, ML gate.
- **Canary exposure granularity** (8 shards ⇒ 12.5%): key-bucket partitioning or shard-count decision in WP-34.
- **Interaction of stacked controllers** (oscillation): explicit stacking tests (WP-37), single-writer executor, hysteresis.
- **Statistical validity of online decisions** (peeking, autocorrelation, multiple candidates): fixed windows or alpha-spending, block-bootstrap, correction; validated on synthetic data with known effects before any live use.
- **Overhead:** budgets and off-switch; signals sampled, not exhaustive.
- **Dependency on M1/M2/M4 quality:** the provenance gate refuses to act on unverified inputs.
- **Schedule:** M5 is large; M5a/M5b split and overlap with M4 are the mitigation.
- **Novelty:** claims limited to demonstrated contributions; G-RESEARCH required before M5b.

## 9. Exit gate G-M5
All PI-1…PI-10 evidenced in a consolidated report; dispositions recorded for each candidate; legacy PIE stub no longer in any product path; documentation (`CLAUDE.md`, handoff note, ADR for the intelligence layer) updated; my review + owner acceptance. Authorizes M6.

## 10. Decisions awaiting the owner
1. Approve inserting Phantom Intelligence & PIE as **Milestone 5** (renumbering M5→M6, M6→M7) vs alternatives (A: keep six milestones and fold M5a into M4 and M5b into M5; B: place the machinery earlier, in parallel with M2–M4). My recommendation: the seven-milestone form, with M5a allowed to overlap M4.
2. Adopt the historical canary defaults [HIST] as provisional defaults pending power analysis.
3. Constraint-first utility instead of the weighted-sum utility (weighted sum kept as an option).
4. Advisory-mode shipping default for the MVP if no capability is validated.
5. Intelligence overhead budgets (after M1/M5 measurements).
