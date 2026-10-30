> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work against this package was executed and reported in [phase_4_completion_report.md](phase_4_completion_report.md) (agent-reported PASS; supported status partially complete). No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 4 Execution Package — M4: Phantom Lab & Static Baselines

**Status: PROPOSED — not authorized.** Authorization is separate for the two parts: **M4-A lab core** after G-M1; **M4-B baselines and scenario evaluation** after G-M2 (freshness/stale-serving exist). Cluster-dependent scenarios additionally require G-M3. Common rules: `phase_execution_packages_index.md` §4.

## 1. Purpose, scope, non-goals, outcomes
**Purpose.** Build the experimental platform on which every later claim rests: deterministic workload replay, a controllable origin with degradation scenarios, a reviewed set of **strong static baselines**, strict result provenance, and independent reproduction. **No adaptive logic is built here.** **Scope:** lab runner, workload generators, origin models (simulated and real-process), scenario library, baseline implementations (including eviction baselines via the M2 trait), run manifests and result schemas, report generation, a minimal truthful Lab viewer. **Non-goals:** the controller/PIE (M5), the final statistical analysis and claims (M6), polished UI (M8), WAN or physical-network experiments (optional T3).
**Outcomes.** (1) Tier-1 deterministic simulator and Tier-2 real-process lab, both producing schema-validated, hashed results; (2) a scenario library with IDs and validity checks; (3) baselines BL-0…BL-12 plus eviction baselines, each fidelity-reviewed and fairly tuned; (4) a T1↔T2 agreement report; (5) independent reproduction PASS.

## 2. Starting state
**Verified earlier (PROVISIONAL line numbers):** the existing `sim/*` modules (virtual clock, seeded RNG source, traffic generator with Zipf/flash-crowd, scheduler, an M/M/1 backend) are used by tests and the demo loop; the Zipf sampler recomputes its normalizing constant per sample in O(catalog) [V `traffic.rs:168-191`]; cache APIs take an injected `now_ms` (so real cache logic can run under virtual time) [V]; the demo dashboard numbers are synthetic [V]. **Expected from prior phases (PROVISIONAL — verify at M4-00):** Phase 1 manifests, Core baselines and noise bounds, origin metrics, provenance schema v2, reference policies; M2: freshness engine with `stale-while-revalidate`/`stale-if-error`, hard staleness cap, eviction trait, µs latency; M3 (only for cluster scenarios): harness and fault proxy. **Not available:** a Linux host, real traces (licensing not yet checked), second machine.

## 3. Architecture of the lab (decisions marked O4-x are owner-reserved)
**Tiers.** **T1 — deterministic software-in-the-loop simulation:** single thread, integer-microsecond virtual clock, event queue with total ordering `(time, sequence_id)`, real `PartitionedCache`/freshness code driven by injected time, simulated origin (queue model) and simulated clients; outputs bit-identical on the same platform/toolchain; labeled **SIMULATED** and never used for wall-clock performance claims. **T2 — real single-host system:** the lab gateway process, a real controllable origin process, an open-loop load generator over loopback HTTP, wall-clock measurement, platform-labeled; labeled **MEASURED (loopback)**. **T3 — multi-host** (optional, M6/M7 if network claims are planned).
**Where baselines run.** Product handlers embed the origin-fetch path, so baselines are implemented in a **lab gateway** (a binary or feature-gated target that reuses the library's `PartitionedCache`, freshness engine and origin client but owns its request path and policy layer) — O4-1 decides the packaging (new workspace member vs `[[bin]]` with `required-features = ["lab"]`, default off so the product build is unaffected) and O4-2 confirms that the lab gateway, not a product refactor, hosts baselines in M4. **Fidelity guard:** a conformance suite proves that the lab gateway configured as BL-1 (single-flight only) behaves like the product gateway on the shared HTTP conformance tests (status codes, headers, coalescing counts, freshness decisions); divergences block baseline use. M5 will later define the product-side policy interface; equivalence tests between lab and product implementations are an M5/M6 requirement.
**Determinism rules (T1).** One RNG master seed → named independent streams (arrivals, keys, sizes, origin service, faults) derived by seed-splitting; no wall-clock reads; no iteration over hash-map order (use ordered containers or sort); integer time; a deterministic Zipf/discrete sampler (precomputed cumulative table or alias method built once — fixes the per-sample O(n) cost); floating-point use confined to documented places, and **bitwise reproducibility is claimed only for the same OS/toolchain** (cross-platform runs are compared statistically). Test: 10 reruns hash-identical; changing one stream's parameter does not perturb other streams.
**T2 timing rules.** Open-loop arrivals at scheduled intended-send times; latency = completion − *intended* send time (coordinated-omission-safe); generator health recorded (achieved/target rate, max scheduling lag); origin records per-request start/finish with monotonic timestamps; resolution limits of the platform stated.

## 4. Work packages
| ID | Title (legacy) | Part | Depends on |
|---|---|---|---|
| M4-00 | Entry verification; reconcile Phase 1–3 outputs | A | G-M1 |
| M4-01 | Lab core: runner, virtual clock, event engine, RNG streams, manifests, result schemas (WP-15) | A | M4-00 |
| M4-02 | Workload library | A | M4-01 |
| M4-03 | Origin models (simulated) and degradation scenarios | A | M4-01 |
| M4-04 | Real-process controllable origin and open-loop generator (WP-16) | A | M4-02/03 |
| M4-05 | Lab gateway and fidelity conformance suite | B | G-M2, M4-04 |
| M4-06 | Static baselines BL-0…BL-12 (WP-17) | B | M4-05 |
| M4-07 | Eviction baselines on the M2 trait | B | G-M2 |
| M4-08 | Tuning protocol and frozen configurations | B | M4-06/07 |
| M4-09 | Scenario matrix execution (T1 full, T2 prioritized subset) | B | M4-08 |
| M4-10 | T1↔T2 agreement study | B | M4-09 |
| M4-11 | Independent reproduction and provenance verification | B | M4-09 |
| M4-12 | Minimal Lab viewer and report generator (WP-20a) | B | M4-09 |
| M4-13 | Cluster-aware scenarios (conditional) | C | G-M3 |
| M4-14 | Documentation and handoff to M5/M6 | B | all |

### M4-01 Lab core
Components: scenario DSL (declarative, versioned file: workload family + params, origin model + schedule of changes, gateway config, duration, warm-up policy, seeds, measured outputs); runner (T1/T2); manifest writer; schema validator. **Run manifest v2** (JSON, hashed): scenario id and hash; gateway/lab binary hash and git tree hash (dirty flag); config hash; baseline id and parameters; seed(s); tier and platform label; hardware/OS/toolchain; start/end UTC; limitations text; raw artifact paths with sha256; metric provenance map (`measured|derived|simulated|unavailable`); validity-check results (EV list below). **Acceptance:** schema validation tests; determinism test; manifest completeness test (all required fields present or explicitly `unavailable`).

### M4-02 Workload library (IDs are stable; parameters in the scenario file)
`W-U` uniform; `W-Z(α)` Zipf with α ∈ {0.6, 0.8, 1.0, 1.2}; `W-SC` scans (one-hit wonders at fraction f); `W-SH` hot-set shift at t_s (fraction of hot keys replaced); `W-FC` flash crowd (rate ×k on m keys for duration d); `W-HK` hot-key emergence (new key's rate grows geometrically); `W-DI` diurnal (sinusoidal rate + noise); `W-MX` mixtures; `W-SZ` object-size distributions (fixed, lognormal, bimodal; size–popularity correlation settings); `W-TTL` heterogeneous freshness lifetimes; `W-INV` invalidation/write streams (Poisson with correlated bursts); `W-TR` trace replay (formats: simple CSV of `time, key, size, ttl`; **public-trace use requires licence/provenance review and owner approval, O4-3**). Arrival processes: Poisson, MMPP (bursty), closed-loop sessions. **Family split (PROVISIONAL, frozen at M4-00 by the owner, O4-5):** *tuning families* {W-Z, W-FC, W-SC}; *held-out families* {W-DI, W-SH, W-MX, W-TR (if licensed), W-HK}. Held-out data are never used for tuning in M4 or M5. **Three disjoint seed sets are generated and recorded (hashes) at M4-01:** *T* tuning seeds (baseline and candidate tuning), *D* development-evaluation seeds (M5-13 dispositions and M5 overhead/replay checks), and *S* **sealed** evaluation seeds reserved for M6 — the list is stored hash-only in the manifest, never used by any M4/M5 run, and a leak check (M4-08, M5-13, M6-00) asserts it never appears in any log. **Tests:** statistical conformance of each generator (Zipf rank–frequency fit, burst timing exactness, rate accuracy within 1% over windows), determinism, parameter-edge cases.

### M4-03 Origin models and degradation scenarios
Origin = c parallel servers with service-time distribution (lognormal, Pareto-tail, or empirical), optional size- or key-dependent cost (uncorrelated / popularity-correlated), queueing emerges from the model. **Scenario scripts (IDs):** `O-ST` capacity step-down ×r at t; `O-LAT` latency inflation ×k; `O-BR` brownout (error rate e, correlated bursts); `O-OUT` outage of duration d; `O-REC` slow recovery (capacity ramp); `O-RT` retry storm (client retry policy with timeout and retry budget as part of the load model); `O-CO` cold start (gateway restarts empty under load); `O-FL` flapping (alternating up/down); `O-HC` heterogeneous per-key cost; `O-NONE` healthy control. Each scenario defines expected observable signatures (e.g., origin utilization ρ(t), error rate) that the validity checks verify. **Tests:** model conformance (M/M/c sanity vs analytic results for exponential service: mean wait within tolerance), scheduled-change timing exactness.

### M4-04 Real-process origin and generator (T2)
Origin process with an admin API (set capacity, latency distribution, error rate, outage, ramp) executed on a script clock; service time realized with async sleep + semaphore capacity (queueing emerges); per-request start/finish log; Prometheus-style counters not required. Generator: open-loop scheduler from the same workload definitions as T1 (shared code for key/size/arrival sampling so T1 and T2 see the same request sequence for the same seed). **Validity probes:** generator achieved-rate ≥ 99% of target, max lag bound; origin utilization from its own log vs configured capacity; cross-check against the gateway's M1 origin metrics (agreement within a preregistered tolerance — EV-13). **Tests:** timing accuracy tests (scheduled event error distribution), saturation tests (generator never the bottleneck at target rates), clean shutdown.

### M4-05 Lab gateway and fidelity conformance
Lab gateway = HTTP front door + policy layer + the real cache/freshness engine. Conformance suite (shared with the product gateway): coalescing counts, status/header mapping, freshness decisions, stale-serving cap, byte budget behavior, fencing under delete. **Acceptance:** BL-1 passes the full suite; any divergence from the product gateway is documented and either fixed or declared out-of-scope with the baseline disclosure.

### M4-06 Static baselines (each: mechanism, knobs, prior art, fidelity checklist; tuning in M4-08)
| ID | Baseline | Mechanism and knobs |
|---|---|---|
| BL-0 | naive | no coalescing, TTL only (control; shows stampede) |
| BL-1 | single-flight | one origin fetch per key; waiter cap; TTL only (industry default) |
| BL-2 | nginx-style | single-flight with lock timeout fall-through (`proxy_cache_lock_timeout`-like), `use_stale updating error timeout`-like serving, background update; knobs: lock age/timeout, stale windows |
| BL-3 | Varnish-style health grace | binary backend health from a probe (e.g., 3-of-5 window, probe interval/timeout), short grace when healthy / long grace when sick; knobs: graces, probe params |
| BL-4 | XFetch | probabilistic early refresh (β) + single-flight; recompute-cost estimate from observed fetch latency |
| BL-5 | Memcache-style lease | one refill token per key per interval, others get stale or wait; knobs: token interval, stale window |
| BL-6 | constant-work refresh | each hit triggers an asynchronous refresh with probability q or rate cap (origin load independent of hit ratio); knobs: q, cap |
| BL-7 | adaptive concurrency | gradient/AIMD limit on in-flight origin fetches driven by origin latency (Envoy/Netflix-style); excess → queue, shed, or serve stale; knobs: min RTT window, headroom, queue bound |
| BL-8 | success-rate admission | probabilistic rejection of misses as origin success rate falls (Envoy admission-filter-style); knobs: threshold, aggression, max reject probability |
| BL-9 | queue-delay admission | Breakwater/DAGOR-style signal (origin queueing delay) gating miss admission; knobs: delay target, step sizes |
| BL-10 | retry budget | cap on retry ratio for client/gateway retries; knobs: budget fraction, window |
| BL-11 | Farahbakhsh remedies | never-timeout randomly tagged fills; young-request priority at the origin queue; sufficient timeout |
| BL-12a | per-scenario oracle static | best static combination of BL-2…BL-11 knobs chosen per scenario with the full evaluation seed set excluded from selection (upper bound on static tuning; non-causal, labeled) |
| BL-12b | family-tuned static | best static combination tuned on tuning families and applied unchanged to held-out families (deployable) |
Each baseline's **fidelity checklist** is derived from the primary documentation/papers (cited in the literature review), lists implemented vs deliberately omitted behaviors, and is reviewed by a second agent/the research lead (M4-11 includes the review record). Deviations are disclosed in the result tables.

### M4-07 Eviction baselines
Reference-quality SIEVE, S3-FIFO, LRU, TinyLFU/W-TinyLFU through the M2 eviction trait (if M2 renamed instead of implemented them, M4 uses the Phase 1 reference implementations in an isolated cache and discloses that limitation). Metrics: miss ratio, origin-cost-weighted miss cost, eviction latency.

### M4-08 Tuning protocol (fairness)
Objective (PROVISIONAL, frozen by the owner O4-6): minimize peak origin concurrency subject to client p99 ≤ SLO, **zero staleness-cap violations**, and hit-ratio loss ≤ x pp versus BL-1; lexicographic or penalty form recorded. **Equal budget** per baseline (e.g., 60 configurations: Latin hypercube/random + local refinement), evaluated on **tuning seeds disjoint from evaluation seeds** and on tuning families only (BL-12b) or per scenario (BL-12a, labeled oracle). All trials are logged (config, seed, result); the selected configuration is frozen (hash) before evaluation seeds are touched. A tuning-leak check asserts no evaluation seed or held-out family appears in tuning logs.

### M4-09 Scenario matrix execution
**Matrix:** scenarios {O-NONE, O-ST, O-LAT, O-BR, O-OUT, O-REC, O-RT, O-CO, O-FL, O-HC} × workload families (tuning and held-out) × baselines {BL-0…BL-12b} × eviction baselines (separate sub-matrix). **T1:** full matrix, ≥ 30 seeds per cell (increase after the M6 power analysis if needed — M6 may request more). **T2:** a *prioritized subset* (PROVISIONAL: O-ST, O-BR, O-OUT, O-CO, O-RT × {W-Z(0.8), W-FC, W-DI} × key baselines {BL-1, BL-2, BL-3, BL-7, BL-12b}) with ≥ 10 repetitions per cell, class rules per CER-5; time plan: estimate per cell = scenario duration (PROVISIONAL 120–300 s incl. warm-up) × repetitions; total T2 ≈ 5×3×5×10 cells-runs × ≈ 4 min ≈ 50 h — **the subset or durations are reduced in a documented order (never repetitions below 10)** under the owner-set budget (O4-6/A-style time budget). Execution order is randomized per block (ABAB-like interleaving of baselines) to spread drift.
**Experiment validity checklist (all must hold or the run is INVALID):** EV-1 scenario/config hash preregistered before the run; EV-2 seeds recorded and disjoint from tuning seeds; EV-3 warm-up/steady-state rule satisfied (hit-ratio slope within ε over the window, or an explicit transient-measurement window); EV-4 generator health (achieved ≥ 99% of target, no coordinated omission); EV-5 origin utilization/saturation trajectory matches the scenario's expected signature; EV-6 fresh process per run, explicit cold/warm cache protocol; EV-7 exactly one policy configuration per process, arm order randomized; EV-8 unique ports/temp dirs, no cross-run state; EV-9 clock/time-source stated; EV-10 platform label and fitness (CI width within the preregistered bound for timing metrics); EV-11 raw outputs hashed and stored; EV-12 `stale_cap_violations == 0` (any violation is a *finding*, recorded, run kept but flagged); EV-13 gateway origin metrics agree with origin log within tolerance (PROVISIONAL ±2%); EV-14 no unexpected process errors/panics. INVALID runs are rerun and logged, never silently dropped.

### M4-10 T1↔T2 agreement study
For ≥ 3 scenarios, compare T1 and T2 on: ranking of baselines by the primary metrics, direction and approximate magnitude of differences, and signature metrics (origin concurrency peak, hit ratio, stale-served fraction). Preregistered agreement bands (PROVISIONAL: same ranking of top-3 baselines; relative differences within a band set from T2 noise; O4-7). Discrepancies are reported with hypotheses (e.g., scheduler latency, loopback effects). **Consequence rule:** if T1 does not predict T2 ordering for a scenario family, T1 results for that family are marked non-predictive and may not be used as evidence in M6.

### M4-11 Independent reproduction and fidelity review
A reproduction run on a *fresh checkout at the recorded tree hash* (second directory and clean toolchain, and—if available—second machine, O4-8): T1 outputs must hash-match (same platform) or match statistically (different platform, bands set in advance); T2 summaries must fall within the preregistered equivalence band. Baseline fidelity review record (per BL-id): reviewer, checklist result, deviations. **Acceptance:** reproduction PASS or BLOCKED (never silently waived).

### M4-12 Minimal Lab viewer and reports (WP-20a)
CLI/markdown/CSV report generator (tables with class labels, intervals, provenance) and a read-only viewer that renders manifests and results with the M1 provenance components: **T1 shown as SIMULATED, T2 as MEASURED (loopback)**, unavailable as unavailable, validity-check results visible, link to raw artifacts. Backend: the lab runner writes JSON files; the viewer loads them via file import or a static route in the **lab binary only** (not the product) — proposed UX requiring that lab-side support (listed in M8). No product endpoint is added.

### M4-13 Cluster-aware scenarios (conditional, G-M3)
Reuse the M3 harness for scenarios that include invalidation propagation and node failure under the chosen contract; same validity rules plus M3's staleness definitions.

## 5. Acceptance criteria and exit gate G-M4
| ID | Criterion | Blocking |
|---|---|---|
| AC4-1 | E0 complete; prior-phase outputs recorded; family split and objective frozen by the owner | yes |
| AC4-2 | T1 determinism: 10 reruns hash-identical on the same platform; stream independence tests PASS | yes |
| AC4-3 | Workload and origin generators pass statistical/timing conformance | yes |
| AC4-4 | Lab gateway conformance: BL-1 equals product behavior on the shared suite | yes |
| AC4-5 | Every baseline BL-0…BL-12b implemented with a fidelity checklist and review record PASS | yes |
| AC4-6 | Tuning protocol followed: equal budgets, disjoint seeds, frozen configs, leak check PASS | yes |
| AC4-7 | T1 matrix complete (≥ 30 seeds/cell); T2 subset complete with VALID classes (or BLOCKED/NOT RUN with owner-visible reasons) | yes |
| AC4-8 | All runs have manifests; validity checks recorded; INVALID runs listed with reruns | yes |
| AC4-9 | T1↔T2 agreement study delivered; consequence rule applied | yes |
| AC4-10 | Independent reproduction PASS (fresh checkout; second machine if provided) | yes |
| AC4-11 | Viewer/reports label SIMULATED vs MEASURED vs derived vs unavailable correctly (labeling test) | yes |
| AC4-12 | No change to product behavior (product hashes/tests unchanged except authorized integration points); no unapproved dependencies | yes |
**G-M4 passes** when all blocking criteria PASS (or have written owner waivers) and the owner accepts. **Disabled until G-M4:** any candidate-policy (M5b) evaluation, any claim comparing baselines, UI Lab view as a product feature.

## 6. Measurement integrity specifics
T1 outputs are *simulation*, T2 *measured on loopback*; neither supports physical-network claims. Metrics catalog for results (each with source/unit/window/provenance): origin concurrency peak and time series (measured from the origin log in T2; simulated in T1), origin requests/s integral, client latency distribution (HDR, intended-time based), error rate, hit ratio (windowed), stale-served fraction and age distribution, staleness-cap violations, bytes resident, gateway CPU/RSS (Linux only), generator health. Statistical treatment (intervals, effect sizes) belongs to M6; M4 supplies per-cell raw distributions and simple summaries only and states n.

## 7. Risks
| Risk | Containment |
|---|---|
| Simulator/real mismatch | T1↔T2 study + consequence rule |
| Baseline infidelity ("strawman" baselines) | primary-source checklists, independent review, oracle upper bound, disclosure table |
| Tuning leakage/over-fitting | disjoint seeds, family split, leak check, frozen hashes |
| Generator saturation/coordinated omission | health probes, intended-time latency, EV-4 |
| Platform noise (hybrid-core laptop) | VALID-only rules, interleaving, labels; BLOCKED if unfit |
| Matrix too large | prioritized T2 subset, documented reduction order, never n<10 |
| Scope creep into the controller | prohibition; candidate code lives only in M5 |
Known limitations: single host; loopback; closed set of scenario families; baselines are re-implementations from public documentation, not vendor binaries.

## 8. Authorized files / prohibitions
May create: lab runner/gateway/origin/generator code under the lab packaging chosen by O4-1 (e.g., `lab/**`), `lab-results/**` (untracked), scenario files, tests/bench support, docs. May modify with ledger: minimal integration points in the library only if M2/M3 interfaces require (listed, owner-visible); `Cargo.toml` only for the lab packaging/features approved by O4-1. **Must not modify:** product request-path semantics, `src/control/pie.rs`, `src/security/*`. CER-3 prohibitions apply.

## 9. Owner-reserved decisions
O4-1 lab packaging (workspace member vs feature-gated bins) and any `Cargo.toml` change · O4-2 lab gateway vs product refactor for hosting baselines · O4-3 public-trace use/licensing/download · O4-4 any dependency (runtime or dev) · O4-5 tuning/held-out family split · O4-6 tuning objective, budget and the T2 time budget · O4-7 agreement and equivalence tolerances · O4-8 second machine for reproduction · O4-9 exposing the Lab view in the product UI. **Agents may decide:** module layout in the lab, scenario file naming, generator internals, report formatting.

## 10. Stop conditions
Determinism failure not explained; conformance divergence of BL-1 from the product; generator health below bound in T2 on all feasible rates; baseline fidelity review FAIL; tuning-leak check FAIL; T1↔T2 disagreement across all families (stop, report, owner decides on scope); any need to alter product semantics.

## 11. Completion report
CER-8 plus: lab design record, scenario catalog with validity checks, baseline fidelity table, tuning logs summary, matrix coverage table (cells run/NOT RUN with reasons), T1↔T2 study, reproduction record, data inventory with hashes, handoff notes for M5 (interfaces and policy-hook requirements discovered) and M6 (power-analysis inputs: observed variances), next phase proposal only.
