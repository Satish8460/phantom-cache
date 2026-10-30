# P2: Workload Intelligence (three-pillar product direction)

**Status: RESEARCH / PROPOSAL. Not a plan of record. Nothing here is approved or built.** Date: 2026-10-30. Workstream P2 of 4. Tree at start: HEAD `456bb25`, branch `feat/publish-phantomcache-core`, 93 dirty entries (read only; no git mutation, no cargo, no edits except this file and `docs/research/rt0_prior_art_sweep_2026-10-30.md`).
**Labels:** FACT = read in code or a record and cited. INFERENCE = my reasoning from facts. RECOMMENDATION = what I suggest; needs owner approval. UNRESOLVED = open. No confidence numbers are given because none were measured.
**Frozen records:** preregistrations, sealed results and `novelty_snapshot_m6.md` were read only. No seed, salt or commitment is generated or proposed here.
**Peer files:** when I started the folder was empty. Late in the work `01_`, `03_` and `04_` appeared; I read only their "requests to other workstreams" sections (and P3 section 4.6, P4 section 2.9) and answer them in section 10. I did not review the rest of those files.

## 0. The one-paragraph position

FACT: the only intelligence that touches the serving path today is an origin-concurrency gate (static cap or AIMD), and the AIMD controller did not beat a static cap in sealed M6 (H1 NOT_SUPPORTED, Tier 1 SIMULATED) or in Scenario B (`TRADE_OFF_FAILURE`, MEASURED_LOOPBACK). That result is preserved here as it stands. FACT: it is not evidence that intelligence is not viable. The accountability report says so itself (section 11.3, last paragraph). INFERENCE: a defensible intelligence advantage must come from decisions the deterministic engine cannot make alone: knowing demand before it arrives (declared or detected), choosing where to spend a bounded origin budget, and telling the operator things they cannot see. RECOMMENDATION: build intelligence as a bounded advisor first (observe, explain, recommend), add one narrow actuator (budgeted pre-warming from a declared calendar) behind a safety layer, and let every further capability earn its place against a strong static baseline with a kill criterion. Do not call any of it validated until a pre-registered result says so.

## 1. Verified baseline: what intelligence code exists

Tags: IMPLEMENTED = code exists. TESTED = automated tests exist (counts are `#[test]`/`#[tokio::test]` attributes found by grep, not a run). INDEPENDENTLY VERIFIED = someone other than the author checked retained evidence. Other classes as in the brief.

| Component | Where | Tests (grep count) | Wired into serving path? | Status class |
|---|---|---|---|---|
| Mode enum `Off/Observe/Advisory/Canary/Active`, default Off | `src/intelligence/mod.rs:22-36` | 3 unit (`mod.rs:206-258`) | Parsed in `src/main.rs:459-466`, stored in executor `main.rs:468-470` | IMPLEMENTED, TESTED |
| Config key `PHANTOM_INTELLIGENCE` (`off/observe/advisory/canary/active`, default `off`) | `src/config.rs:161-162`; warns canary/active are experimental `config.rs:921-923` | `m7_config_tests.rs` (not counted) | Yes (sets mode) | IMPLEMENTED, TESTED |
| Origin gate: static cap or AIMD, RAII permit | `src/intelligence/executor.rs:78-165, 217-250`; cap env keys `config.rs:163-169`; configured `main.rs:471-478` | `origin_cap_tests.rs`, `intel_executor_tests.rs` (6) | **Yes.** Permits taken at `src/api/server.rs:1276` (bypass), `:1365` (background revalidation), `:1647` (miss leader); helpers `:2377-2425` | IMPLEMENTED, TESTED; static-cap enforcement INDEPENDENTLY VERIFIED by reading and recomputation (`g1_verdict.md` row "Origin-cap enforcement"); AIMD limited by 265 ms snapshot sampling |
| Cap enforced only in Canary or Active | `executor.rs:217-220` (`origin_cap_enforcing`); `config.rs:878` | covered by cap tests | Yes | FACT |
| Status and audit HTTP routes | `server.rs:213-214` (routes), `:389-416` (status), `:418-458` (audit) | - | Yes | IMPLEMENTED |
| Audit store | `src/intelligence/audit.rs:17-33`; created `main.rs:480` | `intel_audit_tests.rs` (4) | **No producer.** grep this pass: `AuditStore` appears outside `src/intelligence/` only at `main.rs:480` and `server.rs:78-93, 430`; no `.append(` call in production code | IMPLEMENTED, TESTED; endpoint serves an empty store (FACT) |
| Workload pipeline: EWMA rate, Space-Saving heavy hitters, Zipf alpha, sampled working set, CUSUM burst, hot-key emergence | `src/intelligence/signals.rs:27-455` | `intel_signals_tests.rs` (7), `intel_safety_and_replay_tests.rs` (10, partly) | **No.** No reference outside `src/intelligence/` and tests (grep) | IMPLEMENTED, TESTED (unit level), UNVERIFIED accuracy (E4 never run) |
| Detector constants | CUSUM threshold 500.0, slack 50.0: `signals.rs:404`; emergence = count >= 3x previous and >= 30: `signals.rs:363` | - | - | FACT: absolute, not scale-free |
| Shadow evaluator: LRU/SIEVE/S3-FIFO/TinyLFU on sampled keys | `src/intelligence/shadow.rs` (CI at `:462`: `delta - 1.96 * se`) | `intel_shadow_tests.rs` (5) | **No** | IMPLEMENTED, TESTED; hit-ratio only; CI treats two policies on the same stream as independent samples (reported by V1 and the accountability report, partly seen at `shadow.rs:462`) |
| Decision engine `PieDecisionEngine` | `src/intelligence/engine.rs:36-393` | `intel_engine_tests.rs` (7), `intel_lab_integration_tests.rs` (5), safety/replay (10) | **No.** Only in tests and `intelligence/` (grep) | IMPLEMENTED, TESTED; UNVERIFIED independently. Known defects reported by the lead: P99 tracker is not a quantile, CI treats requests as independent (`accountability report 11.2`, not re-read by me) |
| Candidates: frequency admission, origin-aware stale controller, AIMD cap, EWMA rate forecaster, hot-key emergence predictor, decision stump | `src/intelligence/candidates.rs:65, 97, 135, 175, 251, 278` | `intel_candidates_tests.rs` (11) | Lab only: `src/lab/t1_sim.rs:30-32, 1067` imports and applies them. AIMD also runs inside the origin gate | PROTOTYPE ONLY (lab); AIMD RETRACTED as a promoted feature (negative result) |
| Forecaster memory | `candidates.rs:179-181`: `predictions`, `actuals`, `naive_predictions` are unbounded `Vec<f64>` | - | - | FACT: violates the explicit-bounds rule (CLAUDE.md section 5) |
| Executor `apply_policy` (eviction, stale window, background-refresh limits), watchdog, revert, canary bucket router `route_bucket` | `executor.rs:270, 285-310, 346-407` (revert at `:390`) | `intel_executor_tests.rs` (6) | **No production caller** of `apply_policy` or `route_bucket` (grep: only `lab/t1_sim.rs:1067` and tests). Per-bucket canary cannot exist as written because `apply_policy` sets limits globally (reported, accountability 11.2) | IMPLEMENTED, TESTED; not wired |
| `hot_keys` in dashboard payload | `server.rs:766` returns `{"src":"unavailable","reason":"not_implemented"}` | - | Yes | FACT: no hot-key output exists |
| Real key identity available at request handler | `server.rs:1261` `key_hash = fnv1a_64(canonical key)` passed to telemetry (`:1299, 1510, 1927`) | - | Yes | FACT: this supersedes the "key_hash always 0" note in `pie_audit.md` |
| Telemetry primitives | `src/intel/telemetry.rs` (`AtomicLogLinearHistogram` at `:280`), `origin_metrics.rs`, `diagnostics.rs`, `sketch.rs` (legacy CountMinSketch) | in unit/integration suites | Partly | IMPLEMENTED; histogram has no serving callers per reported audit (C5) |
| Legacy threshold PIE | `src/control/pie.rs`; `pie: None` at `main.rs:433` | `p4_pie_tests`, `wp2_tests` (assert stub behaviour) | No | RETRACTED/quarantined (CLAUDE.md 6b) |
| Pre-warm of the **cache** | none. grep for prewarm/prefetch: only connection-pool pre-warm in `src/dist/router.rs` and a comment at `candidates.rs:262` | `cluster_prewarm_repair_tests.rs` tests connection pools, not cache contents | No | DESIGNED BUT NOT IMPLEMENTED |
| Calendar, request log, event tap, per-tenant quotas | none found | - | - | DESIGNED BUT NOT IMPLEMENTED (V2 designs a calendar; quotas reported absent by V1/V3) |
| Lab T1 simulator (what it can evaluate) | `src/lab/t1_sim.rs`: arms `:162-164`; scenarios `M6_SCENARIOS` `:382`; families W-Z, W-FC, W-SC, W-DI, W-SH, W-MX, W-HK, W-SF `:383-433` | `lab_*_tests.rs` | Offline | IMPLEMENTED, TESTED. Constant-service-time origin (no load feedback), no warm actuator, no tenant dimension, no queue-wait column, no cap sweep arms (E2 draft section 4) |
| Lab T2 live (Scenario B) | `src/lab_live/*`, scenario_b.rs | `lab_live_scenario_b_tests.rs` | Offline, real gateway process | IMPLEMENTED, TESTED; G1 PASS with dispositions; pure-delay scripted origin |

Test totals: FACT (reported, `docs/STATUS.md` section 3): 851 passed, 0 failed, 8 ignored, 75 targets, debug, 2026-10-30. Passing tests do not show decision quality. Several intelligence tests use deterministic repeats or literal records (claims ledger C9-C12 UNVERIFIED).

**What is wired, in one line (FACT):** mode parsing, the origin gate (which caps only in Canary/Active), and two HTTP routes. Everything else in `src/intelligence/` is a library exercised by tests and the lab. The config name `canary` therefore does not mean key-bucket canary exposure; today it only turns on cap enforcement (`executor.rs:217-220`; `route_bucket` has no serving caller).

**Independence (FACT):** no intelligence component other than the static origin cap has been independently verified. M5 is "Partial (machinery only)... independently verified: No" in `STATUS.md` section 1.

## 2. Architecture: layers, action interface, mode ladder

### 2.1 Layers (RECOMMENDATION; the deterministic engine stays authoritative)

```
 Operator (declares: calendar, caps, envelopes, tenant budgets)
        |
 L0 DETERMINISTIC ENGINE  cache, single-flight, freshness, fenced invalidation, tenant epochs,
        |                 stale cap, origin cap, byte budgets I1-I8, never evict in-flight
        |  bounded drop-newest event tap (key_hash, tenant, hit/miss, latency, origin health)
 L1 OBSERVE      counters, sketches, per-field provenance (Measured/Derived/Simulated/Unavailable)
 L2 FEATURES     windowed features F1-F12 (exists as a library: SignalWindow, mod.rs:98-113)
 L3 FORECAST     persistence/EWMA/seasonal-naive/Bayesian shrinkage of declared vs observed; intervals
 L4 SHADOW EVAL  miniature simulations, paired comparison, no exposure
 L5 DECISION     chooses a candidate action or NoAction; carries predicted effect and interval
 L6 SAFETY/POLICY VALIDATOR   deterministic code; validates every request; may refuse, clamp, expire
 L7 ACTUATION    executes the validated action through the normal paths (fill, gate, executor)
 L8 OUTCOME      measures realized effect, writes the audit record, updates calibration, trips fallback
```

Rules (FACT where cited, otherwise RECOMMENDATION):
1. Intelligence proposes; L6 and L0 dispose. FACT: this is the repo rule (`phantom_intelligence_phase.md` section 1; CLAUDE.md 6b).
2. L6 is plain deterministic code with its own tests. It does not import forecaster or model code. It reads budgets from operator config and measured state from L0.
3. A prediction that never changes a decision is not an adaptive capability. Each capability in section 3 therefore names the action it changes, or is labelled "observation only".
4. Off must be identical to the static path (spec: single atomic mode check, `intelligence_architecture.md` section 3, reported).
5. Any fault, stall, stale input or `Unavailable` provenance gives NoAction and, for actuators, an automatic return to the baseline (`revert_to_safe`, `executor.rs:390`).
6. The event tap, the audit store and the Lab event stream should share one bounded tap with tiered consumers (V1 11.1 item 6, reported).

### 2.2 The action interface (RECOMMENDATION)

This is the complete, closed list of things intelligence may ask the deterministic engine to do. Anything not on the list is impossible by construction. Every action is a data record with the common envelope below; L6 validates it before L7 runs it.

**Common envelope (all actions):** `action_id`, `kind`, `tenant` (exactly one, verified identity; never a client-supplied header in dev mode, see V1 11.1 item 2), `mode_at_issue`, `inputs` with provenance, `predicted_effect` with interval and method, `budget_claim`, `issued_at`, `expires_at` (hard, short), `revert_plan`, `reason_codes_if_refused`. Every action, accepted or refused, writes one audit record (`DecisionRecord` extended; today it has no field for realized effect or refusal reason, `mod.rs:149-163`).

| Action | What it asks | Preconditions L6 checks | Budgets (explicit counts/bytes) | Scope and expiry | Fallback |
|---|---|---|---|---|---|
| **A1 Warm request** | Fetch key K for tenant T through the normal fill path before expected demand | Mode >= Canary (or Advisory = record only); secure mode and verified tenant; key cacheable by last known response and not an auth-bound key; remaining freshness lifetime < lead time (else no-op); origin healthy (error rate and p99 below operator thresholds); warm queue not full; no cooldown; calendar entry or detector basis is active | warm fetches per second, in-flight warm fetches, warm bytes in flight plus resident warmed bytes, per-entry cap, per-tenant cap, daily cap | per key and tenant; expiry = entry window end; fetch aborted if it cannot finish before onset | stop warming; organic path unchanged |
| **A2 Admission hint** | Mark a key class as admit-first or probation | Never denies a key with active ops or an explicit warm; hint count and bytes bounded; mode >= Canary | hint table entries (count) | tenant; seconds-scale expiry | hints dropped; default admission |
| **A3 Retention hint** | Protect a bounded set of keys from eviction for a while | Cannot override byte accounting I1-I8; can only add protection, never remove the in-flight protection; fraction of shard bytes capped | protected bytes <= operator percent of cache bytes; entry count cap | tenant; expiry required | protection lapses |
| **A4 TTL recommendation** | Suggest `max-age`/SWR for a path class | Advisory by default. If ever applied: never above the origin directive, never above `PHANTOM_MAX_STALE_MS` (default 86,400,000 ms); lengthening is blocked while invalidation durability is unproven (accountability section 12) | recommendation count | class; report only | none needed (not applied) |
| **A5 Origin-share suggestion** | Suggest how the operator-set origin budget is split between tenants and between warm and organic traffic | Clamped to operator min/max; needs per-tenant quotas, which do not exist | share table size | tenant; recommendation until quotas exist | static split |
| **A6 Strategy switch** | Choose one strategy from a pre-approved menu: eviction policy among faithful implementations, cap value within `[PHANTOM_ORIGIN_CAP_MIN, PHANTOM_ORIGIN_CAP_MAX]`, stale window within the stale cap | Hysteresis and cooldown (rate limiter exists in the engine, reported); evidence gate for that strategy passed; watchdog alive; refuse if a switch happened within cooldown | switches per hour | global (current `apply_policy` is global) | `revert_to_safe` |
| **A7 Alert / no-op** | Emit a typed alert or choose NoAction | Always allowed | alert rate limit | - | - |

**Refusal reasons (named, logged, each needs a negative test).** Reuse the V2 list R1-R16 (`v2_forecasting_and_evaluation.md` section 4.3): entry not active or unauthorised, selector too large or cross-tenant, input provenance Simulated/Unavailable, forecast uncalibrated, origin degraded, any invariant or stale-cap violation seen, cache byte pressure, budget exhausted, source reliability too low, regret governor tripped, insufficient lead time, mode Off or kill switch, object size unknown, conflicting entries, clock anomaly, cooldown. Add: R17 ring-buffer overrun above bound (observations incomplete); R18 audit store full and not drainable.

**Priority rule (INFERENCE, essential):** a warm fetch must lose to an organic miss at the origin gate. Today the gate has no priority classes (`acquire_origin_permit` is first-come, `executor.rs:233-250`). A1 therefore needs either a second, smaller warm-only gate that is checked before the shared gate, or a gate that refuses warm permits when `in_flight` exceeds a warm ceiling. Either is new code in a safety-critical path.

**What intelligence may never do (FACT from CLAUDE.md 5 and 6b; RECOMMENDATION as design rule):** change generation fencing or tenant epochs; extend freshness beyond origin directives or `PHANTOM_MAX_STALE_MS`; raise hard caps; change key canonicalization; bypass the origin gate; evict a key with active ops or an in-flight fetch; read another tenant's state; write cache entries directly (a warm is a request through the fill path, so `finalize_fetch` generation and epoch checks apply).

### 2.3 Mode ladder

FACT: `PHANTOM_INTELLIGENCE` accepts exactly `off`, `observe`, `advisory`, `canary`, `active` (`src/config.rs:161`). There is no `shadow` value. RECOMMENDATION: add Shadow as a distinct value (code and config change, needs approval) or define it as a sub-state of Advisory. I use the six-step ladder below and mark Shadow as proposed.

| Mode | Behaviour | Entry criterion | Exit (promotion) criterion | Demotion trigger |
|---|---|---|---|---|
| **Off** | No tap, no collection. Identical to static path | default | E0 overhead measured | - |
| **Observe** | Tap and features on; nothing shown as advice; provenance on every field; hot keys and regime visible to operators | E0 passed: tap overhead within a budget fixed before the run; ring drop rate bounded | detector accuracy vs exact counts (E4) within pre-registered bound; no invariant violation in the run window | overhead breach, drop rate above bound, any invariant violation |
| **Advisory** | Recommendations and "would warm X keys / Y bytes because..." shown with labels and intervals; recorded in audit; never applied | forecasts calibrated on matured predictions (coverage inside tolerance) | operator-explainability check; precision/recall of predicted-hot vs realized-hot above a floor | calibration failure, audit writer failure |
| **Shadow** (proposed) | Counterfactual computed on live traffic: "had we warmed, hits saved = ..., origin fetches spent = ...". No exposure | Advisory exit plus a counterfactual estimator validated in the lab (predicted vs simulated agreement within stated error) | shadow-estimated benefit agrees with lab prediction; no projected harm | disagreement with later canary |
| **Canary** | Real actions on a bounded slice: one low-risk tenant or a key-hash bucket subset (<= 5%), with permanent holdback | pre-registered evidence for that action (CI lower bound > 0, no adverse scenario, T2 confirmation) | zero hard-cap violations and guardrails green over a pre-set window; interference check (warmed bytes compete with the control arm) | any guardrail breach: immediate revert to Advisory |
| **Active** | Action under full budgets and refusal list, per-tenant opt-in, regret governor on | canary exit plus owner approval | continuous | regret governor trip, calibration failure, or repeated reverts: back to Advisory |

FACT/INFERENCE: with the code as written, `canary` and `active` only enforce the origin cap. Calling a mode "Canary" before key-bucket exposure exists would be misleading; RECOMMENDATION: rename or document it before any new action reuses the word.

## 3. Capability catalogue

Ranked by value, feasibility and evidence need (my judgment, not measured). Fields: problem; observed; decision and how it changes the system; expected benefit (a hypothesis, `[H]`); overhead (estimates `[A]`, none measured); evaluation; when wrong; fallback; strongest baseline; falsification test; kill criteria; status. Capability ids are used by `rt0_prior_art_sweep_2026-10-30.md`.

**I1. Truthful workload observation: hot keys, regime, burst (observation only; first thing to build)**
- Problem: `hot_keys` is `unavailable/not_implemented` (`server.rs:766`); operators and every later capability need real key-level signals.
- Observed: `key_hash`, tenant, hit/miss, latency, remaining lifetime (handler already has `key_hash`, `server.rs:1261`).
- Decision/effect: none on the cache. Changes what the operator sees. It also gates I3-I5: no signal, no action.
- Benefit `[H]`: certain operator value; not a performance claim.
- Overhead `[A]`: sketch update per sampled request, O(depth); fixed memory (spec budget 16 MB, reported). Must be measured in E0.
- Evaluation: recall of true top-10 vs exact counts across skew and shift (E4); overhead off/signals/full in a release build with CIs.
- Wrong: false hot keys shown with an error bound; hash collisions inflate counts (bounded by sketch error).
- Fallback: field returns `unavailable` again.
- Strongest baseline: exact counts over a sampled window; per-key EWMA.
- Falsification/kill: recall < 0.9 at the stated memory (E4, reported kill) or hot-path adder beyond budget (E0).
- Status: IMPLEMENTED (library), TESTED (unit), NOT WIRED, UNVERIFIED accuracy. Constants must become scale-free first (`signals.rs:363, 404`).

**I2. Miss-cause split and origin-pressure signals (observation only)**
- Problem: a miss can be cold, expired, invalidated, evicted or shed; the right response differs. Origin pressure is also the gate for all warming.
- Observed: miss reason at the handler, origin latency/error/in-flight (`OriginMetrics`, gate snapshot `executor.rs:65-73`), rejection count.
- Decision/effect: none directly; supplies the gates R5 (origin degraded) and the E9 miss-storm metric.
- Benefit `[H]`: explains why a flash crowd hurt; enables honest Lab output.
- Evaluation: agreement between the tap's counts and the existing counters on a seeded workload.
- Fallback: `unavailable`. Baseline: existing counters. Kill: counts disagree with existing counters beyond tolerance.
- Status: counters exist; miss-cause split DESIGNED BUT NOT IMPLEMENTED (not verified whether the handler already records it).

**I3. Declared-demand pre-warming (Demand Calendar) with a deterministic scheduler (best candidate for a real advantage)**
- Problem: at the first requests of a known event (on-sale, launch, batch import) N distinct cold keys miss at once. Single-flight collapses per key only (reported), so N keys cost N concurrent origin fetches.
- Observed: operator-declared entries (tenant-scoped, bounded, expiring); measured origin health; each key's freshness lifetime and last origin latency. Declared values are labelled `DECLARED`, never `MEASURED` (V2 section 2.1).
- Decision/effect: A1 warm requests at `start - lead`, paced under a warm budget; refresh-ahead so each key's lifetime spans the event window. The calendar can only permit bounded actions; it cannot raise caps.
- Benefit `[H]`: removes or reduces the t0 miss stampede for declared keys; makes origin load at onset predictable. Largest when the origin cannot absorb the burst and lifetime >= lead. Smallest for unscheduled events.
- Overhead `[A]`: negligible CPU; memory = entries x selector size (hard-bounded); extra origin load = the warm itself, accounted as waste when keys are not requested. Holding K keys fresh costs about K x D / TTL fetches (V2 4.2: 1,000 keys, 60 s TTL, 1 h = 60,000 fetches), so lead time is bounded by lifetime unless refresh-ahead is budgeted.
- Evaluation: E9 (oracle-warm headroom), then E11 (calendar value and robustness against mis-specified calendars), then E12 (T2). Metrics: miss-storm size, origin peak and queue wait, goodput, success-only p99, waste fraction.
- Wrong prediction: wrong time or keys waste origin fetches and displace real hot entries. Bounded by budgets, per-entry caps, abort on origin degradation, `rho_s` source-reliability discount (V2 2.3), regret governor.
- Fallback: stop warming; entry moves to `quarantined`; organic path unchanged.
- Strongest baseline: calendar-literal cron warmer (V2 B3), static TTL plus SWR, single-flight alone. Practitioners already run cron warmers (RT-0 T2).
- Falsification: forecast-assisted warm not non-inferior-better than the cron warmer by a pre-registered margin (K-C1). Harm: origin peak/p99 worse than B0 by > 5% or another tenant's hit ratio down > 1 pp (K-C3) (both PROPOSED thresholds, owner decisions).
- Kill: E9 kill K-H (oracle gain below margin) abandons predictive warming; at most calendar-literal remains.
- Status: DESIGNED BUT NOT IMPLEMENTED. Deterministic core needs no ML. Prior art: pre-warming is common; no source found for a tenant-scoped declarative calendar with a safety envelope in a gateway (coverage gap, not novelty).
- Scope limit (FACT, reported V4 5.2): gateway forwards no client identity by default, so pre-warm can only fetch what an anonymous request could fetch. This narrows I3 to public or shared content. Say so in every pitch.

**I4. Hot-key refresh-ahead (predictive expiry-miss avoidance)**
- Problem: expiry of a very hot key is the costly miss (delayed hits scale with rate x origin latency).
- Observed: I1 top-k, remaining lifetime, last origin latency.
- Decision/effect: A1 for top-k keys shortly before expiry, using existing background refresh slots (`max_background_refresh` default 128, ceiling 256; per-key backoff 1 s to 60 s, reported `cache.rs:983, 1084`).
- Benefit `[H]`: fewer synchronous expiry misses on the top keys. Alternative with no detector: XFetch-style randomized early refresh (RT-0 T3, OPENED-DIRECT).
- Evaluation: T1 hot-key worlds (W-HK, W-FC) vs reactive and vs probabilistic early refresh; expiry-miss count and origin fetch count.
- Wrong: wasted refreshes of keys that cool down (bounded by slot cap).
- Fallback: refresh-ahead off. Strongest baseline: XFetch randomized early refresh with zero detection; single-flight with SWR.
- Kill: not better than XFetch with CI excluding zero, or more origin fetches per saved miss than the baseline.
- Status: DESIGNED BUT NOT IMPLEMENTED (library predictor `candidates.rs:251` exists, not wired).

**I5. Early flash-crowd detection and classification (gates other actions)**
- Problem: a crowd on cached keys is absorbed by the cache; a crowd on uncached or just-invalidated keys, or a scan, hits the origin. The response differs.
- Observed: request rate, cold-miss rate, distinct-key rate vs request rate, miss amplification, origin in-flight. Client diversity is not observed today; Jung et al. used client-side features (RT-0 T3).
- Decision/effect: typed alert and a regime label that gates A1/A6. No direct cache change in the first version.
- Benefit `[H]`: earlier than a static threshold on origin pressure; mostly value as a gate.
- Evaluation: detection delay at matched false-alarm rate on non-event traces; T1 families W-FC, W-HK, W-SC.
- Wrong: false alarm triggers a gated action; all gated actions are reversible and capped. Baseline drift can hide a slow ramp.
- Fallback: regime `unknown`, NoAction. Strongest baseline: static threshold on rate or on origin in-flight/miss rate tuned per scenario.
- Kill: detection delay not shorter than the threshold baseline with CI excluding zero at matched false alarms.
- Status: detectors IMPLEMENTED (library) with non-scale-free constants; PROTOTYPE ONLY.

**I6. Origin-protection advisor: cap sizing (Little's law) and bounded wait queue (E1/E2-gated)**
- Problem: Scenario B showed any binding cap sheds. Operators need a sizing rule and a way to avoid needless shedding.
- Observed: arrival rate, origin latency, in-flight, rejections.
- Decision/effect: first an advisory number ("cap X would shed about Y at peak Z", labelled Simulated until calibrated). Later A6 within the operator's `[min, max]`. A bounded wait queue is a deterministic feature, not intelligence.
- Benefit `[H]`: helps the operator choose a static cap; the static cap with queue is the first candidate to test (accountability 11.4).
- Evaluation: E1 (load-dependent origin plant), E2 (does a per-scenario oracle cap beat one good static cap by >= 3 pp goodput), then E3 only if E2 shows headroom.
- Wrong: wrong advice under a plant that does not match reality. Advice is labelled by tier and carries its calibration error.
- Fallback: static cap set by the operator. Strongest baseline: best enumerated static cap; static cap plus bounded queue.
- Kill: E2 NO_HEADROOM on an E1-validated plant abandons adaptive caps; the advisor continues (E2 draft section 12).
- Status: gate IMPLEMENTED; AIMD NOT SUPPORTED (preserved); advisor and queue DESIGNED BUT NOT IMPLEMENTED; E1/E2 BLOCKED on owner approval of lab changes and the v3 salt.

**I7. Eviction and admission strategy selection (shadow-selected)**
- Problem: which policy suits this workload; one-hit wonders and scans pollute.
- Observed: sampled key stream, sizes, TTLs, origin cost per class.
- Decision/effect: A6 eviction policy among faithful implementations; A2 admission hints.
- Benefit `[H]`: only where the default is not near the best fixed policy. RT-0 T4: crowded field, strong simple baselines (S3-FIFO, SIEVE, W-TinyLFU) and online adaptation already ship in mainstream caches.
- Evaluation: E5 with a TTL-, size- and invalidation-aware shadow; paired comparison by key-hash block.
- Wrong: flapping; shadow ignoring TTL/size misleads (known). Switch must keep I1-I8 and never evict in-flight.
- Fallback: fixed default. Strongest baseline: best fixed faithful policy per family (earlier CAND-1 used the product default, a weak baseline).
- Kill: selector not better than best fixed within CI.
- Status: PROTOTYPE ONLY. Open prerequisite: CLAUDE.md sections 5 and 6 disagree on whether the serving cache uses the faithful policies; my grep found `core::eviction` used by the lab only (V1 inventory; not re-confirmed here) (UNRESOLVED).

**I8. TTL / Cache-Control recommender (advisory only)**
- Problem: header-less responses get a literal 60 s because `PHANTOM_DEFAULT_TTL_MS` is not wired (CLAUDE.md 5). Origin owners often set `max-age` badly.
- Observed: revalidation outcomes (304 vs 200), ETag change intervals, invalidation counts.
- Decision/effect: A4, a report for origin owners. No gateway change.
- Benefit `[H]`: modest; accountability rates adaptive TTL "low value now".
- Wrong: lengthening raises staleness exposure after a lost invalidation. Shorten before lengthen; never past origin directives.
- Strongest baseline: static TTL plus SWR; RFC 9111 heuristic freshness (not implemented).
- Kill: recommendations do not reduce revalidations at equal measured staleness exposure on lab traces with change processes.
- Status: DESIGNED BUT NOT IMPLEMENTED.

**I9. Per-tenant policy advice (blocked)**
- Problem: tenants differ; one setting is wrong for some.
- Decision/effect: A5 recommendations; active reallocation only with operator min/max.
- Blockers (FACT, reported V1 11.1, V3): no per-tenant quotas, no tenant-attributed telemetry, dev mode trusts `x-tenant-id`. RT-0 T9: reserved minimums plus dynamic sharing is a published design.
- Strongest baseline: static equal or weighted quotas. Kill: no worst-tenant gain with CI excluding zero in a heterogeneous multi-tenant lab family (does not exist yet).
- Status: BLOCKED.

**I10. Restart / flush warm-up planner**
- Problem: the cache is in-memory; restart and flush-on-gap give a cold cache (accountability section 12).
- Decision/effect: A1 in priority order from an opt-in tenant-scoped hot-set manifest at paced rate. Local node only. Peer-assisted variant is blocked (section 5).
- Risks: manifest stores key names (privacy; opt-in, encrypted, ACL). Never warm from a stale value snapshot; always fetch from origin.
- Strongest baseline: do nothing (lazy fill with single-flight); static ramp limit.
- Kill: origin peak not lower than lazy fill, or goodput during warm-up falls.
- Status: DESIGNED BUT NOT IMPLEMENTED. Shares the I3 scheduler.

**I11. Outcome ledger, calibration, regret governor (cross-cutting; required for honesty)**
- Problem: without recording prediction vs result, nothing can be demoted and nothing can be explained.
- Observed: every A-record plus the realized effect measured later.
- Decision/effect: automatic demotion of an action class when calibration error exceeds the claimed effect; cooldown after revert; waste-fraction governor (R10).
- Evaluation: planted-signal worlds (section 6) must show the ledger recovers known effects and flags known harm.
- Strongest baseline: periodic manual review of audit records.
- Kill: ledger-driven demotion not better than no demotion in a drifting-workload scenario: keep as audit only.
- Status: PARTIAL. Store exists but has no producer; record has no realized-effect field. Replace the independence-assuming CI first (`shadow.rs:462`, reported in engine).

**I12. Distributed signal aggregation and warm coordination**: see section 5. Status: BLOCKED.

**I13. Seasonality forecasting**
- Problem: daily/weekly rhythm could allow preparation without operator input.
- Evaluation needs a real multi-week trace; none is in the repo (all workloads synthetic). Strongest baseline: seasonal-naive and the operator calendar. Kill: MASE >= 1 vs seasonal-naive on a held-out real trace.
- Status: BLOCKED on data.

**I14. Key-fragmentation detector (advisory)**
- Problem: the same content under many keys (tracking parameters) collapses hit ratio. Often the largest real-world loss.
- Decision/effect: report candidate ignore-lists; never auto-normalize (wrong merges serve wrong content).
- Baseline: operator knowledge, vendor query-string rules. Kill: precision of "irrelevant parameter" calls below a pre-registered bar. Prior art not searched.
- Status: DESIGNED BUT NOT IMPLEMENTED. Also useful as a cache-busting abuse signal.

**Ranking (INFERENCE).** First, I1 and I2 (observation, prerequisite for everything). Second, I3 (best chance of a measurable advantage, deterministic core). Third, I11 (needed to be honest about all others) and I6 advisor. Then I5, I4, I10. I7 and I8 are low-differentiation. I9, I12, I13 are blocked. I14 is cheap but unsearched.

### 3.1 The integrated flash-crowd scenario, step by step

Owner scenario: many clients; nodes observe, detect, estimate; nodes coordinate pre-warming; the deterministic engine enforces limits; observability records decisions and outcomes; the Lab shows prediction vs execution vs result and compares reproducibly with a static baseline.

| Step | What happens | Component | Status today |
|---|---|---|---|
| 1 | Many clients send requests, including an event on N keys | Lab load generator (`lab_live/loadgen.rs`), T1 flash-crowd family (`t1_sim.rs:424`) | T1 W-FC exists; T2 generator has no key groups or rate schedule (V2 6.7) |
| 2 | Nodes observe key-level traffic | L1 tap and I1 pipeline | NOT WIRED; no tap |
| 3 | Detect the crowd and classify it | I5 on `BurstDetector`, `HotKeyEmergenceDetector` | library only; constants not scale-free |
| 4 | Estimate demand and the keys involved: declared overlay plus observed counts | L3 forecaster, V2 Gamma-Poisson shrinkage (F8) | DESIGNED (V2); nothing implemented |
| 5 | Decide: warm, hold, or NoAction | L5 decision, A1 envelope | none |
| 6 | Validate against budgets and refusals | L6 safety/policy validator | none (the origin gate is the only live limiter) |
| 7 | Coordinate across nodes: each node warms keys it will serve, within a cluster origin budget | P1 ownership and budget split | BLOCKED on P1 and durable invalidation (section 5) |
| 8 | Execute: warm fetches through the normal fill path under the origin gate | L7, fill path, `acquire_origin_permit` (`server.rs:2377`) | gate exists; no priority; no warm fill path or internal request context (not checked) |
| 9 | Record the decision, the prediction and the realized outcome | L8 audit, `AuditStore` | store exists, no producer, no outcome field |
| 10 | Show prediction vs execution vs result, labelled; compare to static baseline reproducibly | Lab (T1 now, T2 later), P4 views | Lab runner and sealed-protocol machinery exist; no warm arm, no Demand-World generator, no comparison view |

INFERENCE: only steps 1 (T1 half) and 10 (generic runner) exist. The scenario is a build-out of steps 2-9, preceded by E9 to learn whether it is worth building.

## 4. Experiment sequence

All v3/v4 sealed runs need new protocols, new salts and seed sets generated by the owner offline. I do not generate or propose any. Exploratory runs on tuning/dev data are labelled EXPLORATORY and cannot trigger a kill. Confirmatory runs follow a frozen pre-registration with a fail-closed freeze and a run-once marker (`eval.rs:531, 644, 703, 902`, reported).

| Exp | Question | Kind | Tier | Baselines | Metrics | Kill / stop | Needs from the lab | Needs from P1 / P3 / P4 |
|---|---|---|---|---|---|---|---|---|
| **E0** Observe-mode instrumentation and event-tap overhead | What does the tap and pipeline cost? Do counts match existing counters? | Confirmatory for overhead only after a pre-registered budget; first runs exploratory | MEASURED_LOOPBACK, release build, hardware and OS stated | tap off vs signals-only vs full (WP-37 style) | hot-path latency adder with CI, memory, ring drop rate, count agreement | hot-path adder beyond the documented budget (reported kill) | none (needs gateway code, owner approval) | P3: arena size cap (16 MB) and tenant-scoped views; P4: event-tap consumers |
| **E1** Load-dependent origin plant | Does a plant where load slows service exist and behave right? | Validity work (not a hypothesis test) | SIMULATED (T1), then T2 | kappa = 0 regression to existing cells; Little's-law check; collapse signature | V-E1-0..V-E1-5 (E2 draft section 4.3) | T1 vs T2 rank disagreement > 25% of paired scenarios (reported) | new module (not frozen files): `LoadDependentOrigin`, queue-wait column | P4: show plant parameters and tier |
| **E2** Headroom protocol (v3, DRAFT) | Can any per-scenario cap choice beat one good static cap by >= 3 pp goodput? | Dry run on today's origin is EXPLORATORY only. Confirmatory needs E1 validity, a new salt generated by the owner, and a freeze | T1 SIMULATED | enumerated static caps 1..32, NOCAP, best single, oracle per scenario, static+wait-queue, 2-phase oracle | goodput first, then origin queue delay; success-only p99 guard; seed-level paired BCa CI, Holm | NO_HEADROOM on an E1-validated plant abandons adaptive caps; advisor continues | `STATIC-CAP-n` arms, queue arm, rate-ramp, SimParams overrides through `RunSpec`; new `src/lab/e2/` preferred to leave frozen M6 files untouched | P3: none; P4: results view with tier labels |
| **E9** Oracle-warm headroom (V2 6.8) | Is any warming strategy worth building? | Exploratory first on dev data; confirmatory later under a new protocol | T1 SIMULATED | B0 reactive, B1 strongest eviction/admission, B2 prefetch-on-miss, **B3 cron warm (most important)**, B5 yesterday's top-K, B6 random warm of equal budget, B7 oracle-warm | miss-storm size, origin peak and queue wait, goodput, success p99, waste fraction, regret | K-H: oracle gain below a pre-registered margin (V2 proposes 10%) in true-declaration worlds abandons predictive warming | Demand-World generators (new module), warm actuator through `leader_dispatch` with priority and budgets, tenant dimension or unit tests, new columns | P1: cluster budget semantics if multi-node; P4: planted-signal scenario view |
| **E10** Forecast skill and calibration | Do seasonal/quantile/Bayesian forecasters beat the right naive baseline? | Exploratory on tuning regimes; confirmatory on held-out regimes | SYNTHETIC_FORECAST_HARNESS (labelled SIMULATED) | persistence, seasonal-naive, EWMA, literal declaration | MASE (seasonal scale), pinball, coverage, sharpness | MASE < 1 - margin in >= 5 of 8 held-out regimes else no forecaster ships; coverage outside [85%, 95%] means `UNCALIBRATED` (both PROPOSED) | harness module | P4: calibration display |
| **E11** Calendar value and robustness | Does calendar plus forecast beat cron warm; safe under bad declarations? | Confirmatory under protocol | T1 + harness | B3 cron warm, B4 history-only, B6 random | as E9 plus wasted warm fraction, other-tenant hit ratio | K-C1..K-C4 (V2) | G3, G4, G5, G8, G9 worlds, null world G1 | P3: quotas for the multi-tenant world |
| **E12** T2 actuation confirm | Does the T1 sign and rank survive a real process? | Confirmatory, >= 10 repetitions, release | MEASURED_LOOPBACK (never mixed with T1) | same arms | same, plus overhead | sign/rank disagreement > 25% of paired cells | out-of-process warmer first (no gateway change), load generator with rate schedule and key groups, scripted origin event phase | P4: loopback labels |
| **E4** Hot-key detector accuracy | Recall vs exact counts | Exploratory then confirmatory | T1 + microbench | exact counts, per-key EWMA | recall of true top-10, error bound, memory | recall < 0.9 at stated memory | detector harness | - |
| **E5** Eviction selection | Beats best fixed policy? | Confirmatory | T1 | best fixed faithful policy, W-TinyLFU | miss ratio, origin cost | selector <= best fixed within CI | TTL-aware shadow | - |
| **E6** Origin-pressure forecast | Beats persistence? | Confirmatory, needs E1 | T1 | persistence, EWMA | MASE, lead time | MASE >= 1 on held-out scenarios | E1 plant | - |
| **E7** Advisor what-if calibration | Predicted vs measured effect of a config change | Confirmatory | T1 vs T2 | operator rule of thumb | calibration error | error larger than recommended effect | trace sampler, T2 | P4: calibration shown |
| **E8** ML selector | Passes the five-part ML gate? | Only after E5 passes with a non-ML policy | - | non-ML selector | - | fails any gate part | - | - |

Order (RECOMMENDATION, from accountability 11.6 and V2 6.8): E0 and E4 (observation) in parallel with E2/E1 (lab only); E9 before building any warm actuator; E10-E11 only if E9 passes; E12 last. The E2 dry run on D seeds is the cheapest first step that needs no salt.

**Contamination (FACT, E2 draft section 3):** designers have seen aggregate M6 results for the same scenario shapes, so "caps differ by scenario" is a prior belief, not a surprise. The sealed M6 set is spent. New protocols need new domain strings and seed sets.

## 5. Distributed intelligence boundaries

FACT (reported by CLAUDE.md section 6 and V1/V3; not all re-read): GET is served from the local cache only; the mTLS cluster path shares the serving cache for the local node (`main.rs:327`, as recorded in CLAUDE.md with date 2026-10-29); static membership; one origin per gateway; the origin gate is process-wide (`executor.rs:75-78` per V1), so N gateways can present up to N x cap to the origin and there is no cross-node single-flight. An anti-entropy worker runs in cluster configurations (`main.rs:370, 374`, per CLAUDE.md). The M3 invalidation-durability defect is REPRODUCED on unmodified code: a 204 can be returned, the issuer killed, and a peer never converges for the origin's `max-age` (accountability section 12 update; `docs/research/m3_durability/reproduction_report.md`, not re-read). A fix exists only as a scratch prototype and is not integrated.

**What stays node-local (RECOMMENDATION):**
- Observation, features, shadow evaluation, the safety/policy validator and all actuation. A node never acts on another node's raw data.
- The origin gate and warm budgets: each node enforces its own share.
- Per-tenant state. A node keeps one state per tenant (bounded count, "other" bucket for the tail).

**What may be aggregated, and under which constraints:**
- Allowed: per-tenant counters and mergeable sketches (count-min sketches add; this is a standard property, not verified in this repo), regime labels, origin-health summaries, warm-spend totals. Cluster-wide demand pictures are tenant-scoped.
- Operator-only: a global top-k across tenants. FACT: `/api/v1/intelligence/audit` and `/status` are guarded by `require_observer_if_secure` (`server.rs:393-395, 422-424`) but return unscoped data; once records flow they would reveal cross-tenant load (V1 11.2, reported). Required: observer views are per tenant or aggregate-only.
- Never shared: raw key names across tenants; per-key counts from one tenant to another; any per-key data in a shared model (use aggregate hyper-parameters only).
- A shared sketch across tenants is a side channel (forecast and warm-state oracle, V4 5.2 reported); keep sketches per tenant.

**Avoiding duplicate warming (INFERENCE).** Because GET is served from the local cache only, a node benefits only from keys it warms itself. So "avoid duplicate warming" does not mean one node warms for all. It means: (a) the cluster origin budget is split across nodes so total warm fetches stay under one cap (per-node warm share = cluster warm budget / N, a static or P1-provided split); (b) nodes warm only the keys that their own traffic is expected to request (declared selector intersected with locally observed key interest); (c) no node warms a key already warm and fresh locally. Cross-node dedup by "rendezvous owner warms, others fetch from owner" is a peer-fill design, which does not exist and is an architecture decision (V1 11.2).

**Why cluster-wide warming is blocked on durable invalidation (INFERENCE from cited facts).** Warming adds fresh copies in advance and, in a peer-assisted design, copies content between nodes. If an invalidation is lost when its issuer crashes (reproduced), any copy created or refreshed on the peer before that invalidation arrives stays stale for the object's remaining freshness lifetime, which has no gateway ceiling (accountability section 12). More resident copies and earlier fills widen that exposure window, and a warm decision made by a coordinator would multiply it across nodes. A purely local, origin-sourced warm is no worse than an organic fill, because it goes through generation and epoch fencing like any fill; that is why local-node warming (I3, I4, I10) is not blocked but cross-node warming is. This is the same blocker the V-docs record (V1 11.1 item 3 and 11.6).

**Node health, capacity and tenant protection awareness (RECOMMENDATION):** P1 should expose to P2 per-node health, remaining origin budget share, and membership state; any warm plan is refused (R5) when the local node is degraded or suspect. Tenant protection: per-tenant warm budgets enforced by P3 mechanisms; a tenant's warm spend cannot reduce another tenant's budget (noisy-neighbour test G9).

## 6. Evaluation principles

1. **Strong static baselines first.** Every capability names its strongest non-intelligent baseline (section 3). The earlier CAND-1 and CAND-4 dispositions used weak baselines (product default, a cap of 128) and must be recomputed before citation (accountability section 11.8).
2. **Planted-signal designs (V2 6.5).** Generators are separate from the system under test and the forecaster. Include a null world (no signal; system must do about nothing), a positive control (planted benefit that is computable), and a harm control (planted harm that must be detected). Mis-specified and fake declarations (inflated multiplier, never materializes, huge selector) are first-class worlds.
3. **Anti-circularity.** No metric uses the forecaster's own output as ground truth. Forecast, generator and decision code share no modules. Tune on tuning regimes; test on held-out regimes, not just seeds.
4. **Unit of analysis is the seed.** Cells within a seed are not independent. Paired differences, BCa CIs, sign-flip tests, Holm across scenarios (E2 draft section 8). Report per-scenario and worst-scenario results; cross-scenario means hid per-scenario harm in M6.
5. **Objective before data.** Goodput first, origin queue delay second; peak concurrency is reported but never the headline because it rewards the smallest cap by construction (M6 finding). Success-only latency; all-request latency is survivorship-biased (Scenario B notice item 3).
6. **Negative results are kept.** H1 NOT_SUPPORTED, Scenario B `TRADE_OFF_FAILURE`, the stale window being inert at the M6 horizon, and any NO_HEADROOM or K-H outcome go into the claims ledger as findings, not as failures to be tuned away. An exploratory dry run never overrides a confirmatory result and never triggers a kill.
7. **Tier labels on everything.** `SIMULATED` (T1), `SYNTHETIC_FORECAST_HARNESS`, `MEASURED_LOOPBACK` (T2, release, single host), WSL2 and physical-host kept separate. Never mix tiers in one number. Never call a T1 result a measurement. `DECLARED` is a new provenance value for operator input and may inform but never gate a decision (V2 R3).
8. **Performance claims** need a release build, hardware and OS, workload, seed, baseline, repetitions, CIs and limits (CLAUDE.md 4). None exists for intelligence.
9. **Nothing is "validated" before evidence.** Until a pre-registered result meeting the promotion thresholds exists (delta_min proposed 5%, BCa lower bound > 0 in >= 5 of 8 scenarios, none adverse, T2 confirmation >= 10 repetitions, overhead in budget, zero hard-cap violations, deterministic replay; accountability 11.6), every benefit is a hypothesis.

## 7. Proposal dossiers

For each significant proposal: why necessary, builds on, must change, depends on, how tested, completion evidence, what could make it fail. All code changes need owner approval (CLAUDE.md section 2) and, where fencing, epochs or invalidation are touched, an independent review plus a test that fails without the change.

| ID | Proposal | Why necessary | Builds on | Must change | Depends on | Tested by | Completion evidence | Could fail because |
|---|---|---|---|---|---|---|---|---|
| P-1 | Bounded event tap (drop-newest ring) and E0 measurement | Nothing downstream is trustworthy without truthful observations; hot path must not block | `intelligence_architecture.md` section 3 spec; handler `key_hash` at `server.rs:1261` | `server.rs` request path (one atomic mode check, one push), new tap module | owner approval; E0 budget fixed first | overhead benchmark off/signals/full in release with CIs; fault injection (stall consumer, fill ring) | measured overhead within budget; drop counter; serving unaffected under consumer stall | hot-path adder too large; ring overruns under bursts |
| P-2 | Observe-mode consumer: truthful `hot_keys`, regime, burst | Replaces `unavailable/not_implemented` with measured values | `WorkloadPipeline` (`signals.rs:382-455`) | scale-free detector constants; bounded forecaster vectors; `server.rs:766` field; per-tenant keyed state | P-1 | E4 recall vs exact counts; unit tests with provenance; tenant-scoping tests | E4 passes pre-registered bound; every field carries a label | sketch error; adversarial hot-key flooding; cross-tenant leakage |
| P-3 | Audit producer and extended `DecisionRecord` (refusal reason, realized effect, interval method) | `/audit` serves an empty store; "prediction vs result" needs a record | `AuditStore` (`audit.rs`), `DecisionRecord` (`mod.rs:149`) | add appends at decision points; extend record schema (serde-compatible) | P-2 or lab producer | round-trip tests; deterministic replay gives identical records (PI-6, reported) | records present for every decision including refusals | store capacity 100 loses history (`main.rs:480`); schema drift |
| P-4 | Safety/policy validator (L6) with action interface A1-A7 | Makes "intelligence cannot bypass correctness" structural | `PolicyExecutor` trait (`mod.rs:194`), clamps in `executor.rs:346-377`, refusal list R1-R18 | new deterministic module; one entry point for all actions | owner approval; P3 verified-tenant identity and quotas | per-refusal negative tests; property tests that no accepted action exceeds a budget; fault injection | every action path goes through L6 (grep-checkable); zero budget overruns in tests | validator bugs become the single point of failure; budgets mis-set |
| P-5 | Warm actuator (A1) and warm-only origin-gate class | I3, I4, I10 need a way to fill; warm must lose to organic misses | normal fill path and `finalize_fetch` fencing (CLAUDE.md 5); origin gate | `server.rs` internal-request context for tenant identity (not checked whether the handler allows it); gate priority; warm counters | P-4, P-1; fencing tests with warm fetches in the loop | fencing race tests (invalidate during warm); noisy-neighbour; gate priority tests | tests that fail without the change; independent review | fill path assumes a client request; priority starvation of warm; amplification abuse |
| P-6 | Demand Calendar store and deterministic scheduler | Declared demand removes the need to predict; the best baseline-beating candidate | V2 section 2 schema; bounded in-memory store | new module and API under `/api/v1/intelligence/`; secure-mode, role-gated writes | P-4, P-5, P3 authn/authz | schema abuse tests (V2 2.2: huge selector, renewals, cross-tenant, DST, fat finger) | E9 passes, then E11, E12 | restart loses entries; calendar is an amplification endpoint if unauthenticated |
| P-7 | Lab: load-dependent origin plant (E1) and E2 arms | Without a plant where load matters, no cap analysis is meaningful | `origin_sim.rs`, `t1_sim.rs` | new modules (preferred over editing frozen files); new columns | owner approval for `src/lab` changes; possibly freeze addendum | V-E1-0..5 | validity checks pass; T1/T2 rank agreement | the contention law is an assumption; T1/T2 disagree |
| P-8 | Lab: Demand-World generators and a T1 warm arm (E9) | Cheapest way to learn whether warming has headroom before building P-5 | `WorkloadGenerator` trait, `leader_dispatch` | new module `src/lab/demand/`; tenant dimension or unit tests | P-7 columns | null/positive/harm controls; determinism re-run | E9 result with CIs; positive control recovered | generator shape contamination; synthetic declarations written by researchers |
| P-9 | Shadow evaluator fixes: paired block-bootstrap CI, TTL/size/invalidation aware | Current CI overstates significance; shadow ignores TTL | `shadow.rs` | CI method; shadow policies take size/TTL | none | synthetic data with known effect: false-commit and false-rollback rates | meets pre-set rates | still less power; TTL modelling errors |
| P-10 | Ledger, calibration monitor, regret governor | Honest demotion; needed before any actuator | audit store, V2 4.4 | calibration ring per action class; governor in L6 | P-3, P-4 | planted-signal worlds | demotes planted-harm worlds; no false demotion in null world | too-sensitive governor disables good actions |
| P-11 | Advisor: Little's-law sizing and what-if | Value even if no controller beats a static cap | lab (T1), E1 plant, X3 in V1 | trace sampler (opt-in), advisor report | P-7; E7 calibration | E7 calibration error vs recommended effect | labelled `Simulated` until calibrated | simulator fidelity unvalidated |
| P-12 | Add Shadow to the mode enum and config | The ladder needs it; today `canary` is mislabelled | `mod.rs:22-36`, `config.rs:161` | enum and config value; docs | owner approval | config tests, mode tests | enum and docs consistent | breaking existing deployments' config |
| P-13 | Distributed aggregation and warm coordination | Cluster-wide demand pictures | `src/dist/*`, sessions | merge protocol for per-tenant sketches; budget split | P1, durable invalidation, P3 privacy rules | multi-node loopback tests | agreement between aggregate and exact | blocked; side channels |
| P-14 | Hot-key refresh-ahead (I4) | Expiry misses on top keys | background refresh slots (`cache.rs:983, 1084`) | schedule from top-k | P-2, P-4 | T1 hot-key worlds vs XFetch | beats XFetch with CI | wasted refreshes |

## 8. Risks, unresolved questions, owner decisions

**Risks**
- FACT/INFERENCE: intelligence ahead of correctness work polishes a prototype. The blockers to a useful product are durable invalidation, per-tenant quotas, truthful config (`PHANTOM_DEFAULT_TTL_MS` unwired), persistence and restart behaviour (V1 11.3).
- Pre-warming is an amplification path: an unauthenticated or over-permissive calendar becomes a fill-amplification endpoint. Writes must be secure-mode only, role-gated, tenant-scoped.
- Warmed bytes can evict hotter entries; bucket canaries are biased optimistic unless a byte-quota partition is used (V2 section 3).
- Time compression in T1 (TTL 3 s vs effective default 60 s) is a simulation convenience with an unverified mapping to production.
- Synthetic evaluation tests only what was put into it. No real trace exists in the repo.
- Competition: incumbents (Varnish grace plus health, nginx lock plus `updating`, Fastly collapsing, Cloudflare tiering) already give operator-controlled protection (RT-0 T5). "Uses origin health" cannot be the differentiator.

**UNRESOLVED**
- Does the serving cache use the faithful eviction policies? (CLAUDE.md sections 5 and 6 disagree.)
- Does the handler have a way to issue an internal fill with a tenant identity (A1 prerequisite)? Not checked.
- Does the handler already record the miss cause (I2)? Not checked.
- Is a bucket canary meaningful with 8 shards (12.5% granularity)?
- RT-0 remains open; no novelty claim is possible.

**Owner decisions (none presumed approved)**
- O-1: approve E0 (application-code change for the tap) and the order E0/E4 vs E1/E2.
- O-2: approve `src/lab` changes (new modules for E1/E2/E9) and whether a freeze addendum-2 is acceptable; preferred isolation is new modules.
- O-3: who generates and stores the salt for any v3/v4 sealed run (recommendation in the E2 draft: the owner, offline, outside any agent session). I propose no salt, seed, or commitment.
- O-4: the E2 margin definition (3 percentage points of goodput vs 3% relative reduction in goodput loss) and the other E2 parameters.
- O-5: whether the E2 exploratory dry run on dev seeds may run now.
- O-6: whether the Demand Calendar is "intelligence" for the product narrative or a deterministic feature that intelligence calibrates (affects the ML gate and positioning).
- O-7: whether storing key names (warm-up manifest, trace sampling) is acceptable, or everything stays hashed.
- O-8: whether T2 loopback confirmation is enough for promotion, or a physical-host run is required before any serving-path actuation.
- O-9: whether to fund a dedicated RT-0 plan (fixed venues and years, recorded queries, full reads of the closest candidates) before any positioning claim.
- O-10: add Shadow to the mode enum, and rename or document the meaning of `canary` today.
- O-11: whether any real multi-week trace or design partner is obtainable (I13 cannot be evaluated without it).
- O-12: the thresholds in V2 (10% miss-storm margin, MASE margin, coverage band, waste bound) are proposals only.

## 9. Requests to other workstreams

**P1 (distributed systems)**
- Expose per-node health, membership state and a per-node share of a cluster origin budget (warm budget = cluster budget / N or a P1-provided split).
- State the status of durable invalidation (M3 owed-log) and the failure model; cross-node warming is blocked until it holds.
- Say whether a peer-fill / owner-fetch path is planned. It changes what duplicate warming means.
- Provide key ownership (rendezvous) lookups if a single node is to own a key's refresh.
- Tell me if dynamic membership will land; static membership simplifies budget splits.

**P3 (security, tenancy, resources)**
- Per-tenant quotas for warm fetches per second, warm bytes, entries and daily spend. Without them A1 and A5 cannot be enforced per tenant.
- Verified tenant identity for all intelligence write paths; role gating (operator/admin only); disable calendar and warm in dev mode that trusts `x-tenant-id`.
- Tenant-scoped intelligence endpoints: `/status` and `/audit` must not leak cross-tenant aggregates; observer views per tenant or aggregate-only.
- A hard memory cap for the intelligence arena (spec: 16 MB) and a per-tenant state limit with an "other" bucket.
- Abuse detection using the shared detector family (cache-busting, bypass-header rate, DELETE rate) with enforcement kept deterministic.
- Privacy rules for any stored key names (hot-set manifests, trace samples).

**P4 (Lab, UX, design)**
- A prediction vs execution vs result view per decision: declared, predicted (with interval), executed (budgets spent), realized (hits saved, origin fetches spent), with tier label and a reason code for every refusal.
- Provenance and tier labels on every number: measured, derived, simulated, declared, unavailable; `DECLARED` as a new label.
- A reproducible comparison runner view: static baseline vs warm arm, same seed, paired, with CIs and the baseline named.
- Show negative results (H1 NOT_SUPPORTED, K-H kills) with the same weight as positive ones.
- One shared event tap with tiered consumers (observer aggregate-only, tenant-scoped, operator/lab raw), so the Lab and the intelligence layer do not build two taps.
- A planted-signal gallery (null, positive, harm worlds) so users can see what the system does when there is nothing to find.

## 10. Replies to requests addressed to P2 (from `01_`, `03_`, `04_`)

All replies are RECOMMENDATIONS and DESIGNED BUT NOT IMPLEMENTED unless stated.

**To P1 (`01_distributed_systems.md` section 12, "To P2")**
1. *Minimum cross-node signal schema for Level D.* Per `(tenant, window_id, node_id, schema_version)`: request count, hit count, miss count, cold-miss count, origin in-flight peak, origin error rate, regime label with a confidence field, warm spend (fetches, bytes), and an optional per-key section (fingerprints and counts) that exists only at the operator privacy tier. Every field carries a provenance label (Measured, Derived, Simulated, Unavailable). Advisory only; versioned; bounded size; no raw keys, no cross-tenant joins. Sketches may be sent only as per-tenant sketches.
2. *Single-node evidence gate before Level D coordination.* Level D stays gated until: E4 (detector accuracy) has passed; E9 has not killed predictive warming; the local calendar-literal or forecast-assisted warm (I3) is retained after E11 and confirmed by E12 on one node in a release build with at least 10 repetitions. Without a retained single-node benefit there is nothing to coordinate.
3. *Pre-warm ordering.* Confirmed: cross-node warming (any warming that moves content between nodes, or a coordinator that triggers fills on peers) stays off until invalidation durability and the sync lease P1 names exist. Local, origin-sourced warming is separate and not blocked (section 5).
4. *Where the hot-key detector runs.* Owner-local first, as you assume. Without peer-fill every node serves from its own cache, so each node needs its own view of its own traffic; an aggregator adds a side-channel risk for little gain. Revisit if peer-fill is built.

**To P3 (`03_security_tenancy_resources.md` section 12, "To P2")**: agreed on all seven.
- State is keyed by registered tenant with count and byte bounds; no shared per-key sketches; global top-k is operator-only (matches section 5).
- Warm and calendar actions are billed to the target tenant's quotas and origin ceiling, dropped (not queued) when saturated, use the same credential policy as a client, and never touch `BYPASS-AUTH` keys. This is consistent with the A1 preconditions and the "anonymous-fetchable only" scope limit.
- Write paths exist only in secure mode, scoped to the verified tenant, behind a role distinct from Observer.
- Intelligence never changes quota values or hard caps: A5 is a recommendation only. A6 (a global eviction switch) must preserve per-tenant structures; until per-tenant quotas exist, A6 stays Advisory.
- Intelligence endpoints become tenant-scoped or aggregate-only; the field-set test pattern is welcome.
- Abuse detectors from the shared detector family are observe-only; enforcement is S9.
- No detection-rate claim will be made from synthetic attack traces; they are tagged Simulated.

**To P4 (`04_lab_ux_design.md` section 9, "To P2")**
1. *Event payloads.* I accept the list `workload_observation`, `prediction`, `prediction_outcome`, `capacity_estimate`, `action`, `safety_decision`, `decision_trace`. Mapping: `action` = the A-record (section 2.2, with mode and guardrails); `safety_decision` = every L6 refusal or clamp with its reason code (R1-R18); `prediction` is frozen at issue time with intervals and `input_kind` (`declared`, `forecast`); `prediction_outcome` comes from the L8 outcome step. Today none of these is emitted (the audit store has no producer).
2. *Minimum viable forecaster and gate.* The minimum is persistence, seasonal-naive and EWMA as baselines, plus Gamma-Poisson shrinkage of a declared uplift against observed counts (V2 F8). The gate is E10: no forecaster ships unless it beats the best naive baseline by the pre-registered MASE margin in at least 5 of 8 held-out regimes and its nominal-90% interval covers inside the pre-registered band (both thresholds PROPOSED in V2). Until then P4 should show forecasts as OFFLINE REPLAY (Simulated), or "detected, not predicted" (Derived), never as live prediction. Nothing runs in SHADOW today: the shadow evaluator is unwired and `Shadow` is not in the mode enum (section 2.3).
3. *Declared-demand input.* The Demand Calendar schema is V2 section 2.2 (`DemandEntry`: tenant, selector, window, expected demand with the operator's own interval, permitted actions with per-entry budgets, source, authorization, expiry, provenance label `DECLARED`). P4 can render those fields; nothing exists in the product yet.
4. *Statistics to display for comparison with static policies.* Seed is the unit of analysis; paired differences; 95% BCa bootstrap CI (10,000 resamples, fixed public RNG constant chosen before the tuning run); one-sided paired sign-flip test; Holm correction across scenarios; practical-significance rule (effect >= pre-registered margin and BCa lower bound > 0); the named baselines (for warming: B0, B3 cron warm, B6 random warm, B7 oracle); success-only latency alongside all-request latency. The exact values are frozen in each protocol; the Compare view should print them verbatim from the manifest and refuse to render a "controlled comparison" when seed, workload, build or tier differ.
