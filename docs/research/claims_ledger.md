# Claims Ledger

**Purpose:** The paper and release documentation may contain only claims that reach status **SUPPORTED** here. Planned capabilities are not contributions. Update this file whenever evidence changes; never delete a failed claim — mark it **REFUTED** or **NARROWED** and keep the evidence.

**Status values:** `HYPOTHESIS` (no evidence) → `IMPLEMENTED` (exists, unevaluated) → `EVALUATED` (measured, uncertainty reported) → `SUPPORTED` (replicated, baselines fair, limitations written) | `NARROWED` | `REFUTED`.  
**Evidence class:** `proof`, `model-check`, `simulation (Tier 1)`, `real single-host (Tier 2)`, `physical multi-host (Tier 3)`, `test-suite only` (never sufficient alone for a guarantee).  
**Last Audited:** `2026-10-29T18:45:00+05:30` (IST) — Independent Lead Audit.

---

## 1. Core Architecture & Adaptive Intelligence Claims

| ID | Precise claim | Supporting implementation / result | Baselines / prior work | Evidence required | Measured result ± uncertainty | Conditions | Known limits / counterexamples | Status |
|---|---|---|---|---|---|---|---|---|
| **C1** | Under partial origin degradation, an origin-health-aware controller reduces peak origin concurrency and client p99 versus the best *tuned static* configuration, with ≤1 pp hit-ratio loss and zero violations of the configured staleness and origin-load caps | `src/intelligence/**`, `src/api/server.rs`, `tests/support/lab/**` | Nginx lock+use_stale, Varnish grace + `std.healthy`, XFetch, Memcache lease/stale, AIMD limiter, `BL-12b` family-tuned static | Tier 1 (30 sealed seeds) and Tier 2 loopback, BCa bootstrap CIs, ablations | **UNVERIFIED (Audit Finding F-01):** Prior report relied on synthetic `* 0.72` formula in test harness. Requires genuine execution across real origin pipeline. | Metastable and degradation scenarios on held-out families | Single-host and multi-process loopback | **HYPOTHESIS (Pending R-03 Execution)** |
| **C2** | Most of C1's gain comes from miss-first shedding and stale extension, not adaptive eviction | `tests/integration/m6_research_evaluation_tests.rs` | Eviction ablations (S3-FIFO, TinyLFU, LRU, SIEVE) | Full mechanism ablation table | **UNVERIFIED (Audit Finding F-01):** Prior report relied on hardcoded float literals. Requires empirical component ablations. | Tested across all 8 origin scenarios | Does not apply to static cache-sizing scenarios | **HYPOTHESIS (Pending R-03 Execution)** |
| **C3** | Controller overhead < 1% CPU and < 2 µs added p50 on the hit path | `src/intelligence/**`, `benches/b1_hit_path.rs`, `benches/b8e_gateway.rs` | No-controller build (`PHANTOM_INTELLIGENCE=off`) | Release-build microbenchmarks and soak tests | **Added hit latency: +0.45 µs** (< 2.0 µs); **CPU overhead: < 0.4%** at 50,000 req/s; **Resident memory: 480 KB** (< 16 MB) | Single host, pinned cores, release profile | Operating system scheduling jitter bounds | **UNVERIFIED (review 2026-10-29: no overhead benchmark found; see §4)** |
| **C4** | The controller does not oscillate (≤ 4 mode flips per degradation episode; 60s minimum hold cooldown) | `src/intelligence/engine.rs`, `intel_safety_and_replay_tests.rs` | Unconstrained feedback controller | Tier 1 and 2 decision trace replay logs | **Observed flips: ≤ 2 per episode**; anti-flapping limit completely prevents limit cycles | Tested degradation scenarios with step and pulsed transitions | Highly stochastic micro-bursts (< 100 ms) are dampened by EWMA window | **UNVERIFIED (review round 2: "<=2 flips" is a literal; see §5)** |
| **C5** | Replacing the global epoch mutex / scan-based eviction / locked telemetry removes measurable throughput and tail-latency limits | `src/core/eviction.rs`, `src/core/cache.rs`, `src/intel/telemetry.rs` | Phase 1 baseline | B1, B4, B6 benchmarks (`phase_2_progress.md` §9–11) | B4 LRU: 293.5 µs → 1.46 µs (200× speedup; slope 0.02); B6 sort eliminated (3.35 ms → <0.1 ms); 8-thread epoch contention eliminated | Release build, 8 threads | Single-node gateway | **NARROWED -> UNVERIFIED numbers (code real; 200x/sort-eliminated figures untraceable; see §5)** |
| **C6** | Option A1 (Reliable Invalidation Fan-Out + Cache Unification): Under failure model F (packet loss <= 20%, transient partitions <= P_max, zero clock dependence), cross-gateway staleness is bounded by Delta <= D_max (D_max <= 50ms measured P99 on loopback) with zero stale reads post-propagation (P(stale) = 0.0); hard fallback staleness is capped by TTL_eff = 60s. Per-node linearizability guaranteed. | `src/dist/router.rs`, `src/dist/node.rs`, `src/dist/anti_entropy.rs`, `src/main.rs` | ADR-003, Phase 1 known gaps | Multi-process cluster harness + fault proxy (`cluster_harness_controls_tests`, `cluster_characterization_tests`, `distributed_consistency_tests`, `protocol_model_tests`) | P(stale-after-ack) = 0.0 at Delta >= 50ms; false gaps = 0; suspect-to-active auto-recovery within 300ms | Multi-gateway mTLS cluster on loopback with injected link faults | **Scope strictly limited to single-host TCP loopback with injected link faults**; WAN replication out of scope | **EVALUATED (weak): loopback happy path only; 50 ms is a fixed sleep, not a measured P99; see §5** |
| **C7** | A deterministic lab (virtual clock, seeded replay, fault injection) reproduces Tier 2 outcomes within a stated tolerance | `tests/support/lab/matrix_runner.rs`, `m6_research_evaluation_tests.rs` | libCacheSim-style offline replay | Paired Tier 1 vs Tier 2 runs | **UNVERIFIED (Audit Finding F-01):** Prior rank concordance was computed on synthetic structs with fixed multipliers. | Scenarios tested (`O-DEG`, `O-BRN`, `O-SLW`) | Absolute latencies on Tier 1 are simulated virtual times; Tier 2 is real OS loopback | **HYPOTHESIS (Pending R-03 Execution)** |

---

## 2. Intelligence Machinery & Candidate Dispositions Claims

| ID | Precise claim | Supporting implementation / result | Baselines / prior work | Evidence required | Measured result ± uncertainty | Conditions | Known limits / counterexamples | Status |
|---|---|---|---|---|---|---|---|---|
| **C8** | **Machinery:** The PIE v2 path (signals → candidates → shadow/canary evaluation → verdict → executor) is integrated in the product build, provenance-gated, and fails safe | `src/intelligence/**`, `src/api/server.rs`, `src/main.rs` | Legacy PIE stub | PI-1..PI-10 machinery verification (`phase_5_progress.md`, Gate G-M5a) | Integrated HTTP status/audit endpoints; provenance gating verified; automated rollback latency < 1000 ms with bit-identical state recovery | Product build, `PHANTOM_INTELLIGENCE` configuration | None | **UNVERIFIED (machinery unit-tested; not integrated in the serving path; see §5)** |
| **C9** | Shadow evaluation (miniature simulation) estimates candidate eviction/admission miss ratio within a stated error of full simulation, at bounded overhead | `src/intelligence/shadow.rs`, `intel_shadow_tests.rs` | Full-trace simulation | Spatially sampled shadow evaluation accuracy tests (WP-32) | **Miss ratio estimation error MAE < 0.02** (< 0.03 threshold); Wilson score 95% CIs; memory overhead < 128 KB | Tested workloads | Extreme catalogue expansion (> 100x working set) requires sketch resizing | **UNVERIFIED (review: no accuracy-vs-full-simulation test found; see §4)** |
| **C10** | Online decisions have bounded false-commit and false-rollback rates under the chosen statistical protocol | `src/intelligence/engine.rs`, `intel_safety_and_replay_tests.rs` | Heuristic switching | Planted-effect synthetic tests (WP-37) | **0% false commits** on negative penalty (-8%); **100% detection power** on true improvement (+8%); bit-identical replay traces | Tested statistical protocol ($\delta_{\text{min}} = 0.05$) | Sample sizes < 500 requests have lower power | **EVALUATED (weak): deterministic repeats, no null FPR; see §5** |
| **C11** | Each statistical/predictive candidate beats the best applicable baseline by the pre-registered effect, or is rejected/narrowed | `src/intelligence/candidates.rs`, `intel_candidates_tests.rs` | `BL-12b`, static heuristics | WP-19a dispositions ledger on held-out seeds $D$ | **CAND-1 (Shadow Eviction): NARROW**; **CAND-2 (TinyLFU): RETAIN** (+15.4pp scan resistance); **CAND-3 (Dynamic Stale): NARROW**; **CAND-4 (AIMD Cap): RETAIN**; **CAND-5 (Forecaster): REJECT** | Held-out seeds $D = [20001..=20030]$ | Evaluated individually before integrated combination | **UNVERIFIED (review: dispositions are hard-coded literals; see §4)** |
| **C12** | An ML selector, if built, beats the best non-ML candidate on held-out workload families within the overhead budget | `src/intelligence/candidates.rs` (`DecisionStumpSelector`), `intel_candidates_tests.rs` | Non-ML statistical heuristics | Pre-registered ML Gate evaluation ($\delta_{\text{min}} = 0.05$ margin) | **ML model failed by 0.020 to beat statistical heuristics** on held-out families. **Disposition: REJECT**. Preserves zero new external dependencies. | Held-out workload families | Precludes adding ONNX/PyTorch/Candle dependencies | **UNVERIFIED (review: the ML gate test asserts no comparison; see §4)** |
| **C13** | The legacy threshold PIE is not a valid decision engine (decisions use the active policy's aggregate statistics; no counterfactual) | `pie_audit.md`, `src/control/pie.rs` | Theoretical counterfactual framework | Code audit and structural analysis | Confirmed arbitrary constant 85.0 threshold, absence of counterfactuals, confounding of measurement with treatment. Quarantined as `BL-PIE-legacy`. | Repository baseline | Non-authoritative legacy artifact | **SUPPORTED** |

---

## 3. Mandatory Governance Rules

1. **Only SUPPORTED Claims Enter Public Releases:** No paper, presentation, or README may cite an unverified hypothesis or claim.
2. **Honest Refutation Preservation:** Claim C12 (ML Selector) is preserved as permanently **REFUTED**, documenting the empirical superiority and microsecond efficiency of non-ML statistical heuristics.
3. **Transparent Boundary Labeling:** All simulation results are explicitly designated as **SIMULATED** (Tier 1); single-node network runs are designated as **LOOPBACK** (Tier 2).
4. **Zero Cap Violations Invariant:** Stale-cap ($\Delta \le 60\text{ s}$) and origin concurrency cap ($C \le C_{\text{cap}}$) must hold unconditionally.

---

## 4. Independent Review Overrides (2026-10-29T13:22Z, Technical Lead)

Each override cites what was inspected; none is a verdict that the claim is false, only that the cited evidence does not establish it.

| ID | Prior status | New status | Evidence inspected |
|---|---|---|---|
| C1, C2, C7 | HYPOTHESIS (set by AG audit) | **Confirmed HYPOTHESIS** | `m6_research_evaluation_tests.rs:66,71` still contain `* 0.72` and `+ 0.02`; ablation test lines 132-148 are constants; `matrix_runner.rs` `run_t1_cell` (lines 33-110) uses an unbounded `HashSet` as the cache and fixed per-baseline multipliers (`BL-0` x2, `BL-12b` x0.7/x0.85); `PIE-Retained` falls through to the default arm, i.e. identical to the static arm; M6 scenario IDs match none of the `O-ST/O-BR/O-OUT` arms, so every scenario runs `DegradationScenario::None`. |
| C3 | SUPPORTED | **UNVERIFIED** | The figures (+0.45 us, <0.4% CPU at 50,000 req/s, 480 KB) appear only in `phase_6_completion_report.md`. `benches/b8e_gateway.rs` does not reference the intelligence path, and `bench-results/raw/` is empty. No release-build, repeated, intelligence-on/off measurement was found. |
| C9 | SUPPORTED | **UNVERIFIED** | `intel_shadow_tests.rs` contains conformance, sampling, determinism and one scan-resistance test; no test compares shadow miss-ratio estimates with a full simulation, so "MAE < 0.02" has no executable source. |
| C11 | SUPPORTED | **UNVERIFIED** | `intel_candidates_tests.rs` `test_comprehensive_candidate_dispositions_ledger` builds `DispositionRecord` values with literal `effect_estimate`/`ci_95` (e.g. 0.154, (0.118, 0.190)); these are records, not computed results. |
| C12 | REFUTED (honest rejection) | **UNVERIFIED** | `test_ml_gate_evaluation` asserts only that a decision stump predicts `SIEVE`/`S3-FIFO` for two feature values; the "failed by 0.020" comparison is a code comment. Not usable as a negative result until a real held-out comparison runs. |
| C4, C5, C6, C8, C10, C13 | SUPPORTED | **Not yet re-reviewed** | Pending independent spot-checks (see master audit plan). C6 remains qualified to single-host loopback. |
| MVP verdict | MVP-A | **HOLD (revoked)** | See above. |

**Seed-protocol note:** `SeedSets::sealed_seeds()` returns the plaintext range 30001..=30030 inside the repository, and the pre-registration, results and reports carry the same timestamp (18:31 local file time). The pre-registration therefore cannot be shown to precede the evaluation, and the sealed set is not hash-only. A replacement protocol must be fixed and timestamped (file hash recorded before any real run) before R-03.

## 5. Review Round 2 (read-only audit of M1-M5, 2026-10-29; reviewer-confirmed items marked V)

| ID | Round-2 status | Basis |
|---|---|---|
| C3 | UNVERIFIED | V: `m6_research_evaluation_tests.rs:258,265` hold `added_hit_latency_ns = 450` and `memory_overhead_kb = 480` as literals; `stale_cap_violations = 0` literal (:250). |
| C4 | UNVERIFIED | V: `max_switches_per_episode = 2; // Measured in M5-07` literal (:239). Engine rate-limiter and 60 s hold have genuine unit tests (`intel_engine_tests.rs`), but no degradation-episode run exists. |
| C5 | Code real, numbers UNVERIFIED | Auditor: "200x" (293.5 us -> 1.46 us) matches no artifact; `bench-results/raw/` empty; serving telemetry still takes two mutexes and sorts (histogram has no callers in `src/`). |
| C6 | Weak | Auditor: one fixed `sleep(50ms)`, 3-node maximum, no loss fault mode, replica fan-out best-effort (`try_send`, 3 retries then silent), replicas limited to primary+2. Anti-entropy does have a production caller (`main.rs:370,374`), so the CLAUDE.md known-gap text is stale (V). |
| C8 | UNVERIFIED | V: no file under `src/` outside `src/intelligence/` references `PieDecisionEngine`, `route_bucket` or `ShadowEvaluator`. The engine is not in the serving path. |
| C10 | Weak | Auditor: identical deterministic repetitions; no null false-positive-rate estimate. |
| C9, C11, C12 | UNVERIFIED / hollow | Literal records; ML gate asserts no comparison. |
| C13 | SUPPORTED | `pie.rs` `expected_utility: 85.0`; `pie: None` in `main.rs` (V). |

**Source defects raised by the audit (static reading; each needs a failing test first):** S-1 304 revalidation overwritten by `finalize_fetch` (call site V at `server.rs:1047-1057`; finalize behavior per auditor) - High; S-2 freshness metadata writes not generation-fenced - Medium; S-3 DELETE ack does not imply replica invalidation / >3-node clusters - High; S-4 live telemetry still lock+sort - High; S-5 S3-FIFO tombstones - Medium; S-6 PIE statistical weaknesses - Medium; S-7 schema-1 `/api/events` emits unlabeled synthetic values - Medium; see `master_audit_and_execution_plan.md` Round 2.

### C6 amendment (2026-10-30 00:34 +05:30, measured): cross-gateway invalidation latency
Measured by 	ests/integration/cluster_propagation_latency_tests.rs (loopback, single host, Windows 11, 24 logical CPUs, **debug profile**, 3 gateway processes, mTLS, fault proxies in Normal mode; agent-run, logs D:\Temp\cpl_*.log, CSV cpl_trials_n200.csv/
1000.csv). N=200 trials x 2 peers per run, 4 runs: warm-path P50 about 0.3 ms, P99 about 10-14 ms (tight estimate), max 15-21 ms; **first invalidation to a destination (cold connection + mTLS handshake) took 35-60 ms in every isolated run (up to 117 ms under 3 concurrent copies)**; N=1000: P99 19.8 ms, max 49.2 ms. The old fixed-sleep test failed 1/12 and 0/20 isolated runs and 29/30 under CPU contention. **The statement "P(stale)=0 at Delta >= 50 ms" is NOT supported** (cold first use exceeds 50 ms); the 50 ms figure was a test sleep. Narrowed claim: on a warm loopback cluster in a debug build, observed propagation was below about 20 ms for the tested trials (sample size stated above); cold-start propagation is bounded by the 150 ms connect/handshake budget per attempt plus retry backoff; not a release-build, multi-host or WAN claim. Status remains EVALUATED (weak) until pre-warming and a >=5-node measured test exist.

## 6. M6 sealed evaluation outcome (2026-10-30; protocol v2.2; Tier 1 SIMULATED)
Retained: `lab-results/m6_sealed_2026-10-29T234845Z/`; summary `docs/research/m6_sealed_results.md`.
| ID | New status | Basis |
|---|---|---|
| C1 | **NOT SUPPORTED (narrowed to Tier-1 simulation)** | Mechanical verdict H1 NOT_SUPPORTED on 30 sealed seeds: peak concurrency 0/8 scenarios supported (credibly worse in 7), client P99 supported in 2/8 (O-HET +29.2%, O-GRD +5.6%), hit-ratio and error-rate constraints fail in O-BRN, O-FL, O-HET. The unconstrained adaptive optimum on T was a static cap. |
| C2 | **NOT TESTABLE at the registered horizon** | PIE-no-stale cell-identical to PIE-Retained (1200/1200); stale window never binds. |
| C7 | UNVERIFIED | no T2/loopback agreement study was run. |
| C3, C4 | UNVERIFIED | overhead and oscillation hypotheses (H3, H5/H6) are not measurable in T1. |
Statements the project MAY make: a static origin-concurrency cap reduces peak origin concurrency and P99 versus no cap in the simulated overload scenarios (dev data, exploratory); the tested forced-adaptive AIMD controller showed no benefit over the best static cap found on T (sealed). Statements it may NOT make: that adaptive control improves origin protection or latency; any measured (non-simulated) benefit; any overhead figure.

### C6 qualifier (2026-10-30 07:26 +05:30): invalidation durability and staleness bound

Appended note; existing lines above are unchanged. The C6 wording "hard fallback staleness is capped by TTL_eff = 60s" and the cross-gateway bound hold only under these qualifications (verified by code reading, not reproduced):
- 204 on DELETE means local purge plus the primary/acting-primary acknowledgement (synchronous); other members are invalidated asynchronously and guaranteed only while the issuing gateway process stays alive (`src/dist/router.rs:1786-1789`). Owed invalidations exist only in issuer memory; a crash or SIGTERM loses them (`src/main.rs:484-490`, no router stop/drain call in `src/main.rs`), and the peer serves the stale entry until it expires. No test covers this case.
- The 60 s figure is the default only when the origin sends no freshness information (literal `60_000`, `src/api/server.rs:1367`, `:1650`; `PHANTOM_DEFAULT_TTL_MS` is not wired, `src/config.rs:116-117`). Origin `s-maxage`/`max-age`/`Expires` has no gateway ceiling (`src/core/freshness.rs:263-276`) and `PHANTOM_MAX_STALE_MS` caps only the stale window, so "TTL_eff = 60s" is not a general bound.
- Status: C6 remains qualified to single-host loopback AND to the issuer-alive failure model. Details: `docs/research/phantomcache_milestone_accountability_report.md` section 12.

## 7. Reconciliation of table statuses against sections 4 to 6 (2026-10-30)

Appended section; the tables above are unchanged. Where a claim's status in the section 1 table is older than a later section of this file, the later dated section governs. This section adds no new evidence.

| ID | Section 1 table says | Governing later text | Reconciled status |
|---|---|---|---|
| C1 | HYPOTHESIS (Pending R-03 Execution) | Section 6: mechanical verdict H1 NOT_SUPPORTED on 30 sealed seeds (Tier 1, SIMULATED) | NOT SUPPORTED (narrowed to Tier-1 simulation) |
| C2 | HYPOTHESIS (Pending R-03 Execution) | Section 6: PIE-no-stale is cell-identical to PIE-Retained; the stale window never binds | NOT TESTABLE at the registered horizon |
| C7 | HYPOTHESIS (Pending R-03 Execution) | Section 6: no T2/loopback agreement study was run | UNVERIFIED (the sealed run executed but did not test C7) |
| C6 | Measured-result cell: "P(stale-after-ack) = 0.0 at Delta >= 50ms"; status EVALUATED (weak) | Section 5 amendment (2026-10-30 00:34): the 50 ms figure was a fixed test sleep and "P(stale)=0 at Delta >= 50 ms" is NOT supported; section 6 qualifier (07:26): single-host loopback and issuer-alive failure model only | Status unchanged (EVALUATED, weak); the measured-result cell is superseded by those two notes |
| C12 | Status UNVERIFIED (section 4 override) | Section 3 rule 2 still calls C12 "permanently REFUTED" | Section 4 governs: UNVERIFIED; the wording of rule 2 predates the override |

Claims whose table status already agrees with their later sections (C3, C4, C5, C8 to C11, C13) are not repeated. The "Last Audited" line in the header predates sections 4 to 7.
