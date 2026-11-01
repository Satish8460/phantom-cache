# PhantomCache M6 Pre-Registration v2 (supersedes `preregistration_m6.md`)

**Document:** `docs/research/preregistration_m6_v2.md`
**Supersedes:** `docs/research/preregistration_m6.md` (v1, preserved unmodified; its "frozen" status is void, see section 1)
**Freeze record:** `docs/research/preregistration_m6_v2_freeze.json` (SHA-256 of this file, hashes of all code under test, real UTC timestamp). This document carries no freeze timestamp itself; the record is written after this file is final and **before any evaluation run on the sealed set**.
**Evidence tier of everything produced under this protocol:** `SIMULATED` (T1: deterministic virtual-time simulation driving the real library components; not a measurement of any network, host or process).
**Status of the sealed evaluation (R-03):** NOT RUN. Nothing in this repository is a result of the sealed evaluation.

---

## 1. Why v1 is void

Independent review (master_audit_and_execution_plan.md, Independent Review Addendum, findings F-05, F-06, F-07, F-10) established that v1 was not a valid protocol instance:

| Finding | Defect | Repair in v2 |
|---|---|---|
| F-05 | The "adaptive arm" in the test was computed as `static * 0.72` and `static_hit + 0.02`; the H2 ablation and the T1<->T2 check used hard-coded numbers. | Removed. Cells run the real product components (section 5). A test fails if the old fragments reappear (`test_sensitivity_c_old_shortcuts_are_gone`). |
| F-06 | `run_t1_cell` used an unbounded `HashSet` as cache, fixed per-baseline multipliers, ignored `PIE-Retained`, and none of the eight M6 scenario ids matched a match arm. | Rewritten (`tests/support/lab/t1_sim.rs`). Every M6 scenario id is a real origin degradation (section 6). |
| F-07 | Candidate dispositions were literals. | Replaced by computed held-out comparisons (`tests/integration/intel_candidates_tests.rs`). Remaining gaps are listed in section 10. |
| F-10 | The sealed seed set was the plaintext range `30001..=30030` in the repository; pre-registration, code and results shared one file time, so freeze-before-run was not demonstrable. | Sealed set is hash-committed and derived at run time (section 7). The range `30001..=30030` is **burned**: it must never be used as an evaluation seed and is rejected by the leak checks. Freeze record with SHA-256 and real timestamp precedes any sealed run (section 11). |

The v1 text is retained for the record. Where v1 and v2 disagree, v2 governs.

## 2. Hypotheses (operational definitions)

All comparisons are PIE-Retained (the adaptive arm) versus a named control, paired by seed.

- **H1 (primary benefit).** Under origin degradation, PIE-Retained reduces (a) peak origin concurrency and (b) client P99 latency relative to BL-12b by a mean per-seed relative reduction >= delta_min = 0.05, with fresh-hit-ratio loss <= 1.0 percentage point and zero stale-cap and zero origin-cap violations.
- **H2 (mechanism attribution).** More than 70% of the H1 gain in peak origin concurrency is attributable to the origin-aware mechanisms (AIMD cap and dynamic stale window), i.e. `1 - gain(PIE-evict-only) / gain(PIE-Retained) > 0.70` (pooled over scenarios and families; percentile bootstrap over seeds, B = 10,000). H2 is evaluated only if the full adaptive arm shows a pooled gain; otherwise it is reported as *not applicable*.
- **H3 (overhead).** NOT TESTABLE under T1. CPU, hit-path latency and RSS need a release-mode benchmark on a stated host. Status: **UNVERIFIED**. No figure for H3 is claimed anywhere.
- **H4 (stability).** Proxy: the 95th percentile (nearest rank) over `PIE-Retained` cells of the number of changes of the dynamic stale-window value per run is <= 4. Changes of the AIMD cap are reported but not bounded (AIMD adjusts by design). A calibration run on one tuning seed (not evidence) showed 20 stale-window changes per 20 s run under a 30% brownout, so H4 may well be falsified; that is an acceptable outcome.
- **H5 (shadow evaluator fidelity), H6 (canary protocol error rates).** NOT evaluated by this protocol. Status: **UNVERIFIED**.

## 3. Endpoints

Unit of analysis: the seed. For each scenario and seed, the metric of an arm is the mean over the evaluation families {W-DI, W-SH, W-MX, W-HK, W-SF} of the per-cell value. W-TR (trace replay) is **UNAVAILABLE** (no trace data in the repository) and excluded.

| Role | Metric (from executed events) | Definition |
|---|---|---|
| Primary 1 | `peak_origin_concurrency` | Maximum number of simultaneously outstanding origin fetches (dispatched and not completed, including queued at the origin) during the measurement window. |
| Primary 2 | `client_p99_latency_us` | Nearest-rank P99 over all logical client requests of the measurement window, including failed requests at the time they failed (client timeout 1 s, so P99 saturates at 1 s when >= 1% of requests time out). |
| Constraint 1 | fresh hit ratio | `fresh_hits / requests`; PIE-Retained minus BL-12b mean per-seed difference >= -0.01. |
| Constraint 2 | stale-cap violations | `cache.counters.stale_cap_violations` plus the runner's independent check (age > ttl + window in force); must be 0 in every cell. |
| Constraint 3 | origin-cap violations | Dispatches accepted while outstanding >= the cap in force; must be 0 in every cell. |
| Secondary | client error rate, P50/P95, upstream dispatches, mean stale age | Reported, not used for the verdict. |

## 4. Fixed analysis (no data-dependent choices)

1. For each of the 8 scenarios and each of the 2 primary endpoints: per-seed relative reduction `r_i = (ctrl_i - treat_i) / ctrl_i` with ctrl = BL-12b, treat = PIE-Retained, i over the 30 sealed seeds.
2. Effect: mean of `r_i`. Interval: 95% BCa bootstrap of the mean, B = 10,000, fixed RNG seed `20261029`.
3. Test: two-sided paired sign-flip permutation test on the mean of `r_i` (Monte-Carlo, 20,000 flips, fixed RNG seed).
4. Multiplicity: Holm-Bonferroni step-down across the 16 hypotheses (8 scenarios x 2 endpoints), alpha = 0.05.
5. **Practical-significance rule.** A hypothesis is **supported** iff Holm-adjusted p < 0.05 AND mean `r_i` >= 0.05 AND BCa lower bound > 0. It is reported as **practically null** if the BCa upper bound < 0.05, and **adverse** if the BCa upper bound < 0. Whatever the outcome, all 16 results are reported with effect sizes and intervals; no result is dropped.
6. Falsification criteria, operationalised: **F-a** fires if, for at least 6 of 8 scenarios, the mean `r_i` of PIE-Retained versus BL-12b (or versus BL-6) is below 0.10 on both primary endpoints; **F-b** is reported from O-HET and O-GRD results; **F-c** fires if PIE-Retained's mean client P99 exceeds BL-1's by more than 5% in any scenario; **F-d** fires on any violation of Constraints 2 or 3; **F-e** is read per hypothesis from the BCa interval; **F-f** (prior-art overlap) is outside this protocol.
7. The full result set (`cells.csv`, `manifest.json`, `analysis_*.csv`) is retained. Raw per-request records are optional because of size (`PHANTOM_M6_RAW_REQUESTS=1`).

## 5. Arms (different real configurations of the same event loop; no multipliers)

Component glossary: *single-flight* = `PartitionedCache::begin_fetch_atomic` with generation fencing; *SWR/SIE* = `check_stale_serving` windows (stale-while-revalidate / stale-if-error) bounded by `max_stale_ms`; *cap* = `DefaultPolicyExecutor` origin gate (`try_acquire_origin_permit`, `OriginPermit::observe`) with the product `OriginConcurrencyCapController`; *dyn-stale* = `OriginAwareStaleController` applied through `apply_policy(StaleServing)`; *admission* = `FrequencyAdmissionFilter` (TinyLFU-style, only while the executor is Active).

| Arm | Configuration |
|---|---|
| BL-0 | no coalescing, no stale, no cap |
| BL-1 | single-flight only |
| BL-4 | single-flight + standalone AIMD cap (product controller), tuned |
| BL-5 | single-flight + lab circuit breaker (error-rate trip, open period) + SIE; fixed defaults (not tuned) |
| BL-6 | per-scenario oracle static (cap, stale window) chosen from the BL-12b tuning grid per scenario on tuning seeds |
| BL-12b | single-flight + SWR/SIE with static window + static cap (min = max), tuned on tuning seeds/families |
| PIE-Retained | single-flight + SWR/SIE + AIMD cap + dyn-stale + admission, all via the product executor in Active mode, tuned |
| PIE-no-stale, PIE-no-cap, PIE-evict-only | ablations: remove dyn-stale / remove cap / keep admission only |

Not implemented, therefore **not in the matrix**: BL-CW (constant-work refresh), BL-FT (Farahbakhsh remedies), BL-PIE-legacy. They are dropped, not "passed".

Sensitivity/control arms used only by tests: PIE-off (PIE configuration, executor in `IntelligenceMode::Off`), CTRL-nonadaptive (PIE's substrate with no adaptive component), PLANT-CAP4 (BL-1 plus a static cap of 4), CAND-* (one-component pairs for candidate dispositions).

## 6. Simulation and scenarios (frozen; see `SimParams::standard()` in `t1_sim.rs`)

Cache: 4 shards x 40 entries, 208,000-byte total budget, SIEVE eviction, TTL 3 s, 1,024-byte bodies, catalog 400, Zipf alpha 0.95, open-loop Poisson arrivals at 250 req/s. Origin: 8 servers, constant 60 ms service, FIFO queueing (`SimulatedOrigin`). Timeline: 8 s healthy warm-up, 20 s measurement window; scenario onset at t = 8 s; client timeout 1 s; 0.3 ms client processing time on served responses.

| Id | Real origin / runner behaviour |
|---|---|
| O-DEG | latency x3 from onset |
| O-BRN | 30% of origin requests fail (HTTP 500) for 10 s |
| O-SLW | full outage for 4 s, then capacity ramps 25% -> 100% over 8 s |
| O-CO | cache invalidated at onset (`try_invalidate_all`) plus 2x arrival burst for 5 s |
| O-RT | latency x2.5 from onset and clients retry failures/timeouts (<= 3 retries, 100 ms backoff) |
| O-FL | flapping: 4 s cycle, 1.5 s down (HTTP 503) |
| O-HET | heterogeneous per-key cost (0.5x-3x, size dependent), no step |
| O-GRD | latency inflation rising linearly 1x -> 4x over the window |

Workload families: tuning {W-Z, W-FC, W-SC}; held-out {W-DI, W-SH, W-MX, W-HK}; sealed family W-SF (adversarial bimodal, introduced for M6; the *family* is public, only the *seeds* are secret).

## 7. Seeds and the sealed-set commitment

Sets: **T** tuning `10001..=10060` (this protocol tunes on the first 8), **D** development `20001..=20030` (smoke runs, candidate comparisons), **S** sealed (30 seeds). T, D and S are pairwise disjoint (tested).

**Sealed set S is hash-only in the repository.** Construction:

- A 32-byte random salt was generated from the operating-system RNG on the evaluation workstation and stored **outside the repository** at `D:\Personal\.phantom-sealed-m6\m6_salt.hex` (hex). It was never printed. It was produced inside an AI-agent session; the agent did not echo it, but the owner may regenerate it (which requires a new commitment and a new protocol version) before R-03.
- Committed value (in code and here): `SEALED_SALT_COMMITMENT_SHA256 = bc7c9683fb46b1e4d8f3b2c01612668c9591427402e49c576220bba3724f0e75` = SHA-256 of the 32 raw salt bytes.
- At run time `seed_i = first 8 bytes (big-endian) of SHA-256("phantomcache-m6-sealed-v2" || 0x00 || salt || i_be32)` with the top bit cleared, `i = 0..29`. Derivation refuses to run unless `SHA-256(salt)` equals the commitment, and rejects any seed that collides with T, D or the burned range.
- The salt is supplied only through the environment variable `PHANTOM_SEALED_SALT_HEX` at R-03 time.
- Outputs of a sealed run label seeds `S00..S29`; the numeric values are never written to `cells.csv` or the manifest (manifest records only the SHA-256 of the seed list).

**Leak check (before every sealed run, and as a test when the salt is available).** `scan_for_seed_leak` scans every text file under `lab-results`, `docs`, `tests`, `src`, `scripts` for the decimal form of each derived sealed seed and aborts the run on any hit. The unit test proves the scanner flags a planted seed and passes a clean directory. `tuning.rs` and `verify_no_sealed_leak` reject the burned range and, when the salt is available, the real sealed seeds. What this scheme does **not** protect against: someone holding the salt can compute the seeds; the commitment prevents *changing* the seeds after the fact and prevents anyone without the salt (including this repository's readers) from tuning on them.

## 8. Tuning protocol

Tuning seeds: first 8 of T (`10001..=10008`). Families: W-Z, W-FC, W-SC only. Scenarios: all eight M6 ids (the regimes are not held out; the held-out *workload families* and *seeds* are). Equal budget of 18 configurations for each tuned arm (BL-12b: cap x stale window; PIE-Retained: p99 ceiling x base stale x max cap; BL-4: p99 ceiling x max x min). BL-0, BL-1 and BL-5 are untuned (BL-5 uses fixed defaults; this is a known weakness of the BL-5 comparison). Objective: mean of relative peak concurrency and relative client P99 versus BL-1 over tuning cells, subject to zero cap/stale-cap violations, fresh-hit-ratio loss <= 1 pp and error-rate increase <= 0.5 pp versus BL-1. If no configuration is feasible, the untuned default is kept and recorded. The tuning artifacts (`lab-results/tuning/tuning_trials.csv`, `frozen_arm_params.json`) are hashed in the freeze record. **No re-tuning after the freeze.**

## 9. Stopping, exclusion and invalid-run rules

- All 30 sealed seeds x 8 scenarios x 5 families x 10 arms are executed; no data-dependent stopping.
- Execution order is interleaved per protocol (seed -> scenario -> family -> arms rotated); because the simulator is deterministic in virtual time, order cannot affect results (determinism is tested).
- A cell with `bytes_audit_equal = false`, a panic, or a non-zero cap/stale-cap violation is **not** discarded: it is reported as a defect finding (and, for Constraints 2/3, fires F-d). There is no wall-clock or host-noise exclusion rule because T1 has no wall-clock dependence in its outcomes.
- If the simulator code, the code under test or this document changes after the freeze, the sealed evaluation is refused (`verify_freeze`); a changed protocol requires v3 with a new freeze record.

## 10. Limitations and open items (stated up front)

1. T1 is a simulation. It exercises the real cache, fencing, stale logic and executor/controller code but not HTTP, TLS, sockets, or the server wiring of R-01. T1 results must not be quoted as measurements. A T2 (loopback) agreement study is not part of this protocol.
2. H3, H5, H6 are unverified; W-TR is unavailable; BL-CW, BL-FT, BL-PIE-legacy are absent.
3. P99 saturates at the 1 s client timeout in overload scenarios, which can hide differences between arms; the metric is fixed anyway so that the choice cannot be made after seeing data. Success-only P99 is recorded as a secondary field.
4. The admission component is a TinyLFU-style frequency threshold on the product sketch (admit when the cache is >= 90% full only if the key's sketch estimate >= 2); it does not compare against an eviction victim.
5. Candidate dispositions (`intel_candidates_tests.rs`) are computed on dev seeds D, 8 seeds for runner-based candidates and 30 for replay-based ones; overhead budgets are unmeasured, so no candidate is finally "Retained"; CAND-7 (legacy PIE) carries a code-fact disposition only. Prior-art novelty is out of scope.
6. The salt was created by an agent process on the owner's workstation (see section 7).

## 11. Freeze procedure

1. Finalise this document, the lab code and the tuning artifacts.
2. Record in `preregistration_m6_v2_freeze.json`: SHA-256 of this file, SHA-256 of every file listed by `tracked_source_files` plus the tuning artifacts, the sealed commitment, the git HEAD, the real UTC and local timestamps from the system clock, and the list of evaluation-like runs that occurred before the freeze (tuning on T, smoke and candidate runs on D; **none on S**).
3. `m6_full_sealed_evaluation` calls `verify_freeze` first and refuses to run if anything differs.
4. Amendments: a dated addendum file referencing this document's hash; analyses outside this specification are labelled exploratory.
