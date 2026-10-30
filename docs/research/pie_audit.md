# PIE Audit — Intended Design vs Documented Claims vs Actual Code

**Date:** 2026-10-28. **Mode:** read-only; no code changed. **Purpose:** recover the existing Policy Intelligence Engine design, reconcile it with the code, and separate intended, documented, simulated, verified and missing behavior. Evidence tags: **[V]** I read the cited source/lines; **[T]** from the preserved transcripts (`docs/handoff/transcripts/*.jsonl`, searched with ripgrep, matching windows only); **[I]** my inference from [V], not executed.
**Naming note.** Documents use "Policy Inference Engine" (handoff), "Policy Intelligence and Evaluation" (PHASE_2 report, README) and "Policy Intelligence Engine" (this project's naming going forward). They denote the same component; this audit uses **PIE** and records the divergence.

## 1. Sources inspected
`docs/implementation/WP2.md` (design plan "Revised", marked COMPLETED) [V full]; `docs/PHASE_2_WORK_REPORT.md` §3.4 and test tables [V ranges]; `README.md` PIE lines [V grep]; `MILESTONE_1_DESIGN_PLAN.md` line 19, 746 [V]; `docs/handoff/{00,01,05}` PIE passages [V grep]; `src/control/pie.rs` [V full]; `src/intel/{telemetry,sketch,workload}.rs` [V full]; `src/api/server.rs:1684-1693` [V]; `src/main.rs:306` [V]; `tests/unit/wp2_tests.rs` [V full]; `tests/integration/p4_pie_tests.rs` lines 150–375 [V]; transcripts (session 1 and 2) for `canary`, `expected_utility`, `Simulated regression` [T].

## 2. Intended design (WP2.md) — exact recovered values
| Element | Intended value / behavior |
|---|---|
| Utility | `U = w1·HitRate − w2·Latency_P99 − w3·BackendLoad − w4·ResourceCost`; resource cost = memory + CPU + inter-node replication bandwidth. **Weights w1…w4 are never given numerical values in any document I found.** |
| Decisions | COMMIT (utility exceeds baseline by more than an activation threshold, confidence met, canary passes); ROLLBACK (utility below baseline or latency safety threshold violated in canary); NO ACTION (low confidence, insufficient samples, minor predicted benefit). |
| Baseline window | 60 s steady-state profiling |
| Candidate (canary) window | 30 s |
| Minimum sample size | 10,000 logical requests in the canary segment |
| Regression threshold | max +10% P99 vs baseline |
| Confidence | 95% bounds on sliding telemetry windows |
| Cooldown | ≥ 60 s after commit or rollback |
| Exposure limit | canary ≤ 5% of request routing |
| Telemetry | raw observations (timestamp, **key hash**, latency, hit/miss, waiters) → derived metrics (rolling P50/P95/P99, hit-rate trends, backend load profiles) → decisions (candidate comparison logs, canary start/stop, commit/rollback commands) |
| Traceability | 8-field JSON per decision: observation, evidence (Count-Min estimates, P99), candidate policies, constraints (memory, replication bandwidth), expected outcome, canary (window, sample size, exposure), actual outcome, decision |
| Planned files | `src/intel/{telemetry,sketch,workload}.rs`, `src/control/{pie,policies,canary,rollback}.rs` |
| Also in scope | Count-Min popularity estimators and **hot-key detectors** |
Status field in WP2.md: **COMPLETED**; completion criteria include "PIE decision loop, canary rules, and cooldown controllers implemented".

## 3. Documented as implemented (PHASE_2 report §3.4 — the later "evidence-first" revision [T])
`Utility = HitRate×100 − P99×0.1`; guardrails: sample sufficiency `telemetry.len() < 1000 ⇒ NoAction`; `constraints_pass == false ⇒ NoAction`; cooldown `now + 60000`; Commit if utility > 80.0; Rollback if utility < 50.0 **on candidate `S3-FIFO`**; "fixed constants rather than configurable parameters". The report is explicit that these differ from the design. The transcript shows the report was revised ("Update PHASE_2_WORK_REPORT.md with verified evidence-first data and remove unsupported claims") but `WP2.md` was left saying COMPLETED with the aspirational values, and `README.md:34` still calls it "Multi-objective utility optimization with canary guardrails and rollback cooldowns", and handoff `01:109` says it "tracks key access skew via Count-Min sketch… and rolls back if hit ratio degrades".

## 4. Actual code and data flow [V]
`src/control/pie.rs` (124 lines) — the only file under `src/control/`.
- `PieController{telemetry, traces, active_policy: Mutex<String>, cooldown_until_ms, cache: Option<Arc<PartitionedCache>>}`.
- `evaluate_candidate(candidate_name, now_ms, constraints_pass)`: (1) `now_ms < cooldown ⇒ NoAction`; (2) `telemetry.len() < 1000 ⇒ NoAction`; (3) `!constraints_pass ⇒ NoAction`; (4) `p99 = telemetry.calculate_p99()`, `hit_rate = telemetry.calculate_hit_rate()` over the **entire ring buffer under the currently active policy**; (5) `utility = hit_rate*100 − p99*0.1`; (6) **if `candidate_name == "S3-FIFO" && utility < 50` ⇒ set cooldown, `cache.set_eviction_policy(Sieve)`, return Rollback; else if `utility > 80` ⇒ `active_policy = candidate_name`, cooldown, `cache.set_eviction_policy(map(candidate))`, return Commit; else NoAction**; (7) push a `DecisionTrace` with `expected_utility: 85.0` (literal).
- `DecisionTrace` has 7 fields (timestamp, observation string `"Evaluated {candidate}"`, p99, hit_rate, candidate, expected_utility, actual_utility, verdict) — not the 8-field schema; no constraints, no canary fields, no evidence beyond two scalars. Traces are an **unbounded `Vec`** (`traces.lock().push`), never trimmed.
- The source comment at the utility line reads "(simulated counterfactual utility vs raw stats)" and at the rollback branch "Simulated regression triggers rollback" [V]. The transcripts show this exact text in the **first version** of `pie.rs` written in session 1 [T]: the file was a stub from its creation and was never upgraded.
- **Wiring:** `main.rs:306` sets `pie: None`; `PieController` is not constructed anywhere in `src/` except tests [V grep: `PieController::new` only in tests]. **In the production binary the PIE never runs.** The only runtime caller is `spawn_simulation_loop` (`server.rs:1684-1693`): when `state.pie` is `Some` and `telemetry.len() ≥ 1000` it calls `pie.evaluate_candidate(cand, clock, true)` every 100 ms with `cand = if active=="SIEVE" {"S3-FIFO"} else {"LRU"}` and the **simulated clock** (`clock += 100`), so the 60 s cooldown means 600 ticks of *simulated* time.
- **Not present anywhere in `src/`** [V grep `canary|exposure|baseline_window|candidate_window|confidence`: one comment in `pie.rs`]: canary controller, traffic split/exposure, baseline vs candidate windows, confidence intervals, regression-threshold test, BackendLoad/ResourceCost terms, weights, hot-key detector, policy interface (`policies.rs`), rollback module, bounded trace store. `CountMinSketch` is never used (F4). Telemetry `key_hash` is always 0 (F5).

## 5. Behavior classification
| Class | Items |
|---|---|
| **Intended (design)** | everything in §2 |
| **Existing code, real** | the switchable eviction policy in `PartitionedCache` (`set_eviction_policy`; per-key metadata `visited/access_count/last_accessed_ms` is maintained for all three policies, so a switch needs no migration — [V `cache.rs:520-527,574-584`]); a cooldown timestamp; sample-count gate; a bounded-by-nothing trace list |
| **Simulated / placeholder** | `expected_utility = 85.0`; "simulated counterfactual"; "simulated regression"; the S3-FIFO-only rollback branch; candidate chosen by name alternation; simulated clock in the caller; the UI "PIE DECISION INSPECTOR" (hard-coded text, F22) |
| **Verified behavior (tests)** | `wp2_tests`: NoAction below 1000 samples, NoAction when `constraints_pass=false`, cooldown prevents an immediate second decision. `p4_pie_tests`: with prepared telemetry (high hit rate) `evaluate_candidate("S3-FIFO")` ⇒ Commit and the cache policy becomes S3-FIFO; with prepared "bad" telemetry the next evaluation ⇒ Rollback and policy returns to SIEVE; cooldown blocks a further decision; all three eviction policies never evict an in-flight op; a live in-process traffic test crosses the 1000-sample gate and then Commits. **These verify the state machine and policy-switch plumbing, not decision quality** — the "bad telemetry" is injected by the test, and no test compares candidate vs baseline performance. |
| **Known gaps** | no counterfactual evaluation; no canary or exposure; no statistics (no CI, no test, no effect size); decisions use the **active** policy's aggregate stats, so the candidate name is irrelevant except in the rollback branch; no resource/backend-load/staleness objectives or constraints; no per-key or workload signals; no explainability beyond 2 numbers; unbounded trace memory; fixed constants; not wired in production |

## 6. Defects and hazards found in the code (new, [V] reading; behavior [I])
1. **Rollback does not update `active_policy`.** The Rollback branch resets the cache to SIEVE and sets the cooldown but leaves `*active_policy` unchanged (`pie.rs:83-89`), so `get_active_policy()` can return `"S3-FIFO"` while the cache runs SIEVE. Tests assert the cache policy, not the string.
2. **Good performance drives policy drift.** `Commit` fires when *current* utility > 80, i.e., whenever the system is doing well; with the simulation loop's candidate rule (`active=="SIEVE" ⇒ S3-FIFO else LRU`) the system moves SIEVE → S3-FIFO → LRU and stays on LRU (candidate "LRU" while active is not SIEVE) [I from `pie.rs:57-119` + `server.rs:1686-1691`], with no evidence that any switch helped.
3. **Threshold semantics are inverted for safety:** a *low* utility on the current policy triggers a "Rollback" only when the candidate string is S3-FIFO; low utility under any other candidate yields NoAction, so the engine cannot react to poor performance generally.
4. **Utility mixes units** (`hit_rate*100` vs `p99_ms*0.1`): a 10 ms change in P99 equals 1 percentage point of hit rate; no basis recorded. Telemetry latency is handler wall time at 1 ms resolution, and since DELETEs are recorded as zero-latency misses (F13) the inputs are biased.
5. **Trace store unbounded** (memory growth under repeated evaluations).
6. **Locks:** three separate mutexes taken sequentially; no atomic decision/execution step (a concurrent evaluator could interleave cooldown and policy writes).

## 7. Historical plan drift (recorded, not rewritten)
- `MILESTONE_1_DESIGN_PLAN.md:746` deferred "PIE adaptive switching" to "Phase 5".
- The handoff roadmap (`docs/handoff/05`) re-labeled Phase 5 as soak testing and Phase 6 as "Adaptive / Predictive Caching Research"; the demo-lab roadmap used Phase 6 for the HTTP mesh. PIE was therefore deferred repeatedly and never scheduled as a deliverable.
- `WP2.md` (design, COMPLETED) vs PHASE_2 report (implementation, honest constants) vs README/handoff (overstated) — three inconsistent statements of the same component.

## 8. Reconciliation with the conceptual architecture (owner's relationship model)
| Concept | Exists in code today? | Gap |
|---|---|---|
| Phantom Intelligence (observe → signals → patterns → evaluate → safe adapt) | Fragments: `telemetry.rs` (partial, polluted), `sketch.rs` (unused), `workload.rs` (profiles, consumers unverified), `pie.rs` | No signal pipeline, no detectors, no integration, not wired |
| PIE | `pie.rs` threshold trigger | no candidate evaluation, canary, statistics, audit |
| Policy implementations | three eviction policies (named after, but not implementing, SIEVE/S3-FIFO/LRU); no admission/refresh/coalescing/stale policies | policies are an enum matched inside `cache.rs`; no policy interface |
| Cache execution layer | `PartitionedCache` (invariants, fencing, capacity) | no safe "apply policy" boundary or fallback; no canary partitioning |
| Phantom Lab | `sim/*` partial; M4 planned | not available for evaluation yet |
Do not claim any of these connections work today.

## 9. Consequences for the plan (see `phantom_intelligence_phase.md`)
PIE becomes a required, named milestone; the legacy `pie.rs` is quarantined (WP-22a) and replaced by a new decision engine whose inputs are Phase-1 truthful signals; the design values in §2 are retained as *documented design defaults awaiting power analysis*, not as verified parameters; the 8-field traceability schema is retained as the starting audit format.
