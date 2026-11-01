# PIE architecture design (batch B3) — DESIGN ONLY, NO CODE

Status: **PROPOSED, for Claude's review before any PIE code** (board D-043, item 3). Written 2026-10-31 by `AG-S`. Nothing in this document is implemented by this batch. It refines, and does not replace, `docs/research/phantom_intelligence_phase.md` (M5), `docs/research/intelligence_architecture.md` and `docs/research/pie_audit.md`; where they disagree with the code, the code and the "what exists" section below win.

Evidence tags: **verified in code** (cited), **reported** (prior document, not re-verified), **proposal**, **decision needed**.

## 0. Constraints this design obeys (not negotiable)

1. `CLAUDE.md` section 6b: the legacy `src/control/pie.rs` is a threshold stub; it is not extended and not presented as a controller. No PIE is activated against synthetic or unverified metrics.
2. `CLAUDE.md` section 5: anything touching fencing, epochs or invalidation needs an independent review and a test that fails without the change. This design touches none of them; any later batch that needs the execution layer (`src/core/cache.rs`) must say so and take that path.
3. **Only `Off`, `Observe` and `Advisory` run.** No canary mode, no active mode, no live application of a decision. (D-017.) Section 8 lists what must change in the existing configuration so that this is enforced and not just promised.
4. **Hard caps stay in the execution layer** (staleness cap `PHANTOM_MAX_STALE_MS`, origin-load cap, memory budget, generation fencing). No policy, shadow result or recommendation can raise, lower or bypass them. The intelligence layer has no write path to them in these modes.
5. **H1 stays `NOT_SUPPORTED`** (`docs/research/m6_sealed_results.md`). Nothing here re-opens it. A new claim needs a new pre-registration and new seeds (D-018); sealed M6 records, the freeze hashes and `tests/m6_*` are never touched.
6. Provenance gate: an input whose provenance is not `measured` (or `derived` from measured) forces `NoAction`; `simulated` inputs are only allowed in the Lab and are labelled so.
7. No ML in this design. ML is admitted only through the gate in `phantom_intelligence_phase.md` section 6, after a non-ML candidate has been evaluated.

## 1. What is built, what is not, what is evidence-gated

| Area | State | Evidence |
|---|---|---|
| Mode enum `Off, Observe, Advisory, Canary, Active`; config key `PHANTOM_INTELLIGENCE` accepts all five | **Built**, but the enum and config still accept `canary` and `active` (experimental warning only) | verified in code: `src/intelligence/mod.rs:24`, `src/config.rs:161`, `:921` |
| Executor with origin-concurrency gate and safe baseline, watchdog, revert counter | **Built**; the gate enforces only in `canary`/`active` and is inert otherwise | verified: `src/intelligence/executor.rs`; `config.rs:878` warning |
| `SignalWindow` (features F1 to F12) with per-field provenance, `Detector` trait, `RateDetector` | **Types built**; no detector is fed by the gateway | verified: `src/intelligence/mod.rs`, `signals.rs`; a grep finds no use of `SignalWindow`, `PieDecisionEngine`, `ShadowEvaluator` or `RateDetector` outside `src/intelligence` and `src/lab` |
| Shadow policies (LRU, SIEVE, S3-FIFO, TinyLFU) and `ShadowEvaluator` with a delta threshold | **Built as library code**, exercised by tests and the Lab only | verified: `src/intelligence/shadow.rs` |
| Candidate controllers and predictors (origin-aware stale, AIMD concurrency cap, rate forecaster, hot-key emergence, decision stump) and `CandidateDisposition` | **Built as library code**; dispositions recorded for the sealed M6 study only | verified: `src/intelligence/candidates.rs` |
| `PieDecisionEngine` (arms, cooldown, provenance-aware evaluation) | **Built as library code**; not constructed in the gateway process | verified: `src/intelligence/engine.rs` plus the grep above |
| `AuditStore` (bounded ring of `DecisionRecord`), `/api/v1/intelligence/status` and `/audit` | **Built**; the store stays empty because nothing produces decisions in the gateway | verified: `src/intelligence/audit.rs`, `src/api/server.rs:469-498`, `src/main.rs` (store created with capacity 100, nothing appends) |
| Policy registry (id, objective, inputs, parameters, safety constraints, lifecycle) | **Not built** | this document, section 4 |
| Signal loop in the gateway (feeds detectors off the hot path) | **Not built** | section 3 |
| Shadow-mode portfolio running on live traffic | **Not built** | section 5 |
| Evaluation protocol for the portfolio (new pre-registration, new seeds, tuned static baselines, ablations) | **Not built**; the old M6 protocol is sealed | section 6 |
| Decision explanations with inputs, rejected candidates, estimated and observed effect | **Partly built** (`explain_decision` text, 12-field record); the richer record is **not built** | verified: `audit.rs:79`; section 7 |
| Control Center Intelligence screen | **Not built** (placeholder, board WP12) | `ui/src/screens/Placeholder.jsx` |
| Any adaptive benefit (latency, hit ratio, origin load) | **Evidence-gated: NOT established.** M6 H1 is `NOT_SUPPORTED`; nothing else has been measured | `docs/research/m6_sealed_results.md`, `docs/user/limitations.md` |
| Predictive candidates, ML selector | **Evidence-gated** and out of scope for the first batches | `phantom_intelligence_phase.md` section 6 |
| Canary / active application of any decision | **Forbidden** in this programme (D-017) | section 0 |

## 2. Architecture

```
 request path (unchanged, no PIE code on it except one bounded, lock-free event write)
   |   events: key_hash, hit/miss/coalesced/stale/peer, latency_us, origin outcome   [measured]
   v
 SIGNAL LAYER  (own task, own queue)      <- Observe mode starts here
   bounded MPSC of compact events (drop-on-full, counted)  ->  windowed features F1..F12 + new F13..F16
   |   every feature carries provenance, sample count, window; below a minimum sample count it is `unavailable`
   v
 DETECTORS (hot keys, burst, skew, working set, origin health, invalidation churn, node imbalance)
   v
 POLICY REGISTRY  (section 4)  ->  for each registered policy: should it be evaluated now? (inputs available? mode allows?)
   v
 SHADOW EVALUATION  (section 5)     <- Advisory mode: counterfactual only, on a sampled key stream, no effect on serving
   v
 ARBITER  (conflict detection, ranking, cooldown/hysteresis, constraint check)
   v
 DECISION RECORD (section 7)  ->  AuditStore (bounded)  ->  /api/v1/intelligence/*  ->  Control Center
```

Placement and bounds (proposal):
- The only code on the request path is one non-blocking event write into a bounded queue (try-send; a full queue increments a `signal_events_dropped` counter and the window is then marked degraded, never extrapolated). Everything else runs in a separate task on a low-priority interval (default 1 s window, 10 s decision tick).
- All memory is bounded in bytes and counted: event queue, sketches, shadow caches (fixed capacity, sampled), audit ring. Each bound is a configuration value with a documented default, and `metrics.intelligence.memory_bytes` reports it (measured).
- A fault in the intelligence task (panic, stall, poisoned input) is caught at the task boundary: the mode drops to `Off`, a `Fault` record is written, serving is unaffected. There is nothing to revert because nothing was applied (this is the safety argument for Advisory-only).
- Single writer: the arbiter is the only producer of decision records; detectors and shadows are pure functions of their window.
- The kill switch is the mode itself: `Off` at startup, and a runtime downgrade to `Off` through the same code path as a fault. There is no runtime upgrade path in these batches.

## 3. Signals and features

Existing F1 to F12 (`SignalWindow`) are kept. Additions needed for the portfolio (proposal; all `measured` or `derived` from measured):

| Feature | Source | Used by |
|---|---|---|
| F13 stampede pressure: coalesced waiters per origin fetch, concurrent fetches per key | request outcomes, `X-Cache-Source` counters (`metrics.outcomes`) | stampede-risk warning |
| F14 invalidation churn: DELETEs per minute, fan-out `exhausted`/`dropped` deltas | `metrics.cluster.broadcaster` | freshness advice, distributed warning |
| F15 primary-load imbalance: requests answered per node over the window (needs other nodes' counters; one gateway's own view is NOT enough) | `metrics.outcomes` of each node, collected by the Lab or the Cluster screen, not by the gateway | distributed imbalance advice |
| F16 peer-read pressure: share of misses served by `peer`, `peer_reads_without_hit` rates | `metrics.outcomes` | distributed advice |
| Origin health: p50/p99 latency and error rate of origin fetches | `metrics.origin` | origin-saturation warning, TTL advice |

Rules: a feature that needs another node's data is `unavailable` on a single gateway and is never estimated from the local node alone (CLAUDE.md section 4: no synthetic value presented as live).

## 4. Policy registry (design)

A registry is a static, versioned table compiled into the binary (no runtime loading, no plugin system). Each entry:

| Field | Meaning |
|---|---|
| `policy_id` | stable identifier, for example `POL-ADM-TINYLFU-1` |
| `name`, `kind` | human name; `Eviction`, `Admission`, `RefreshAdvice`, `StaleWindowAdvice`, `OriginSaturationWarning`, `StampedeWarning`, `DistributedImbalanceAdvice`, `WorkloadClassification` |
| `objective` | one sentence and the metric it is judged on (for example "miss ratio weighted by origin latency") |
| `trigger` | the detector condition that makes it evaluable |
| `required_inputs` | feature ids with minimum sample counts and allowed provenance |
| `parameters` and `bounds` | tunables with hard min and max |
| `safety_constraints` | the hard caps it can never relax (staleness cap, origin cap, memory), checked by the arbiter and, for any future live application, by the execution layer |
| `expected_effect` | a testable statement, never a number promised in advance |
| `evaluation_method` | which section 6 protocol applies, which baseline |
| `limitations` | what it cannot see or do |
| `version` | semver of the policy logic; a change makes new evidence necessary |
| `lifecycle` | `Proposed`, `Implemented`, `UnitTested`, `Integrated`, `ShadowEvaluated`, `Retained`, `Narrowed`, `Rejected`. Moves only forward by evidence recorded in the ledger, `Rejected` is terminal for that version |
| `runtime_state` | `Off`, `Observe`, `Advisory`: the highest mode in which it may run, bounded by the process mode. The badge in the UI shows this value, read from the running gateway, never a label |

Rules: a policy cannot be `Advisory` before `UnitTested` and `Integrated`; no policy may have `runtime_state` above `Advisory`; the registry is exposed read-only at `/api/v1/intelligence/policies` (each field labelled; `lifecycle` and `limitations` are documentation values and say so).

## 5. Shadow-mode policy portfolio

All entries run only in `Advisory`, produce a recommendation record, and change nothing. Each has a kill criterion (pre-registered, section 6): if it cannot beat its baseline or if its overhead exceeds the budget, it is rejected and stays out of the product path.

| # | Policy | What it does | Method (reuses) | Kill criterion |
|---|---|---|---|---|
| P1 | Workload classification | Labels the window: hot-key dominated, cold/scan, Zipf-like (alpha estimate), bursty/flash crowd, phase change. Descriptive only, it triggers other advice but applies nothing | F3, F4, F6, F7, change-point test on the key distribution | classification disagrees with the generating label of seeded Lab workloads more than a pre-registered rate |
| P2 | Eviction shadow selection | Runs the reference policies on a sampled key stream and recommends the one with the lowest origin-weighted miss cost when the margin exceeds `delta_min` | `ShadowEvaluator`, `LruShadow`, `SieveShadow`, `S3FifoShadow`, `TinyLfuShadow` | recommendation is not better than the incumbent in held-out Lab traces with a CI excluding zero |
| P3 | Frequency-aware admission | Recommends enabling a TinyLFU-style admission filter when one-hit wonders dominate | `FrequencyAdmissionFilter`, `TinyLfuShadow` | no gain over the incumbent policy, or sketch memory above budget |
| P4 | TTL and freshness advice | Recommends a shorter lifetime for keys with high invalidation churn, and flags expiry storms (many keys sharing an expiry second). It can never advise exceeding the staleness cap | F14, per-key age histogram | advice does not lower stale-serving or origin load against a tuned static TTL |
| P5 | Stampede-risk warning | Warns when F13 shows repeated concurrent fetches for hot cold-start keys (coalescing is already built; this warns when it is saturated) | F13 | false-warning rate above the pre-registered bound |
| P6 | Origin-saturation warning | Warns when origin latency and error rate cross declared thresholds and concurrency is near the origin cap; advisory text only, the cap is not changed | origin health, `OriginGate` snapshot | warning does not precede observed degradation in replayed brownout scenarios |
| P7 | Distributed advice | Reports primary-load imbalance, peer-read pressure and a failing member's impact on miss rate; needs several nodes' data, therefore Lab or Cluster-screen driven | F15, F16, `metrics.cluster` | advice indistinguishable from random on seeded multi-node Lab runs |
| P8 | Conflict detection and ranking | If two recommendations touch the same lever (for example TTL advice and a stale-window warning), the arbiter marks the conflict, ranks by estimated effect with its interval, and emits one primary recommendation with the others as alternatives | arbiter | none (it is infrastructure); tested on constructed conflicts |

Not in the portfolio: any controller that adjusts a cap or a window live (that is the area of the sealed H1 result), predictive prefetching, ML selection.

## 6. Evaluation protocol

1. **Pre-registration first (D-018).** A new document under `docs/research/` fixes, before any run: hypotheses per policy (P1 to P7), metrics, seeds (new, not the sealed M6 seeds, at least 30), the minimum effect, the dispositions rule (Retain, Narrow, Reject from `phantom_intelligence_phase.md` section 5) and the stopping rule. It is committed to the repository (by the owner's action, not by an agent) before data collection.
2. **Baselines** for every claim: the best statically tuned configuration found on separate tuning seeds, the simple heuristic equivalent, and "intelligence off". The legacy threshold PIE is reported only as a floor, never as a competitor.
3. **Where it runs.** Offline in the Lab (deterministic workloads, controllable origin, replay) for all effect claims; on a live three-gateway cluster only for overhead, provenance and decision-record checks (no effect claim from a live run without a controlled origin).
4. **Statistics.** Paired comparison per seed, block bootstrap or the documented method, 95 percent intervals, correction for the number of candidates; peeking forbidden (fixed windows).
5. **Overhead.** Three configurations: `Off`, `Observe`, `Advisory`; measured on a release build with repetitions and intervals, reported with hardware, OS and limitations (CLAUDE.md section 4). The hot-path added cost must stay below the noise bound set in the pre-registration.
6. **Reproducibility.** Same seed and trace give byte-identical decision records; the manifest (binary hash, seeds, trace id, registry version) is stored with each run.
7. **Negative controls.** Shuffled-trace and constant-workload runs must produce `NoAction`; a policy that fires on a constant workload is rejected.
8. **Fault injection.** Panic, stall, poisoned (NaN, huge) inputs, missing provenance, clock jumps: serving unaffected, mode drops to `Off`, a `Fault` record exists.
9. **Claims.** Results go to `claims_ledger.md` with the disposition. H1 stays `NOT_SUPPORTED` unless a new pre-registered experiment says otherwise; this design makes no benefit claim.

## 7. Explanations

`DecisionRecord` v2 (additive to the built record; old fields stay):

| Field | Content |
|---|---|
| ids and time | `decision_id`, `timestamp_ms`, `registry_version`, `policy_id`, `policy_version`, `mode` |
| inputs | each feature used with value, provenance, sample count, window (copied, not referenced) |
| candidates | all candidates considered, with estimated effect and interval; rejected ones with the rejection reason code (below minimum margin, constraint, conflict, insufficient samples) |
| constraints | each hard cap checked, its value and whether it holds |
| recommendation | text and structured form; always labelled "advisory, not applied" |
| observed effect | after the fact, the measured change over the following windows, with the same provenance rules (so a recommendation can be shown to have been wrong) |
| guardrail events | provenance refusals, faults, cooldown suppressions |
| `explanation` | deterministic text generated from the fields by a template (no free text from a model); a reviewer can reconstruct the decision from the record alone |

API: `/api/v1/intelligence/status` (mode, registry version, memory bytes, events dropped), `/policies`, `/decisions` (paginated, bounded, oldest first as today), `/decisions/{id}`. Every value carries `src`. The Control Center Intelligence screen shows only what these return, with the same measured/inferred/unavailable labelling as the Cluster screen, and shows the runtime state read from the gateway.

## 8. Safety and what must change in existing code (decision needed)

- The enum and `PHANTOM_INTELLIGENCE` accept `canary` and `active` today, with a warning. To make D-017 enforceable, a later batch should either remove those values from the accepted set or reject them unless an explicit `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true` is set, and the Control Center must never offer them. This touches `src/config.rs` and `src/intelligence/mod.rs` (the latter is frozen; the coordinator decides whether it is unfrozen for this).
- The existing origin gate is inert outside canary/active. The design does not change that. P6 only reads its snapshot.
- Nothing in this design writes to `src/core`; no batch below needs fencing, epoch or invalidation changes.
- `src/control/pie.rs` stays untouched and unused.

## 9. Proposed batches (each with acceptance; none started)

| Batch | Scope | Acceptance |
|---|---|---|
| C1 | Registry (read-only API and screen), decision record v2, status API with memory and drop counters; no detectors yet | registry entries validated by tests (no policy above Advisory, lifecycle order); API values labelled; UI shows runtime state from the gateway |
| C2 | Signal loop (bounded queue, windows, F1 to F16 that are available), fault containment to `Off`, overhead measurement for `Off` and `Observe` | fault-injection tests pass; memory bounded and reported; overhead measured with intervals |
| D1 | P1, P2, P3 in shadow, Lab-driven first, then live Advisory records | each has unit tests, negative controls, a live 3-gateway Advisory run with real decision records and provenance |
| D2 | P4 to P8, arbiter | conflict tests, replay determinism |
| E | Evaluation (section 6) | pre-registration first, then runs, then dispositions in the ledger |

## 10. Risks and open decisions

Risks: scope (this is large; the batches are ordered so each ends in something reviewable), measurement validity (shadow estimates can mislead; negative controls and held-out traces are mandatory), and the temptation to show a benefit; the honest outcome of Batch E may be "no capability retained", and that is an acceptable result.

Decisions needed from the coordinator:
1. Unfreeze `src/intelligence/mod.rs` (mode enum and record types) for Batch C1, or keep new types in a new module and leave the frozen file untouched (my recommendation: new module, `src/intelligence_v2/` or similar, and no change to the frozen files).
2. Accept the section 8 change (remove or guard `canary` and `active` in configuration).
3. Whether F15 and P7 are in the first portfolio or deferred until the Cluster screen can read several gateways.
4. Overhead budget for the signal loop (after the first measurement).
