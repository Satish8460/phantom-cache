# Chapter 7: Phantom Lab, Experimentation and Innovation

This chapter has two halves. The first half explains Phantom Lab: the machinery the project built to test its own ideas, and how much each result can be trusted. The second half uses that evidence to build an honest innovation argument: what is different about PhantomCache, what is demonstrated, and what is not.

Read this chapter with one rule in mind. A result is only as strong as its label. Every number below carries one of two tiers: **SIMULATED** (T1, a computer model) or **REAL with qualifiers** (T2, real processes, loopback, one host). No result in this repository is a multi-host or production measurement.

## What you will learn

- What Phantom Lab is, why it was built, and how its parts fit together (T1 simulator, T2 real-gateway runner, Lab API, UI, evidence store).
- How a deterministic simulation works: virtual clock, seeded random streams, event queue, workload scheduling, simulated origin, and why the same seed gives the same answer.
- How results are turned into a verdict: paired differences, bootstrap confidence intervals (BCa), a permutation test, Holm correction, and the preregistration and sealed-seed mechanism.
- The exact relationship between the T1 simulation and the T2 loopback experiment, and what each can and cannot show.
- The evidence-based innovation argument: the actual technical contribution, the negative M6 result, how PhantomCache compares with Redis, Memcached, Varnish, groupcache, Caffeine and others, and where those systems are more mature.

---

## 7.1 What Phantom Lab is, and why it exists

**Simple explanation.** Imagine you invent a new traffic-light controller. You cannot test it first on a real city. So you build a model of the city, run the same morning rush hour many times, and compare your controller with the old fixed timer. Later you try it on one real road. Phantom Lab is that workshop for PhantomCache.

**Technical terms.**

- *Experiment*: a defined comparison with a stated question, inputs, outputs and a rule for reading the result.
- *Arm*: one configuration being compared (for example "single-flight only" or "AIMD cap plus stale window").
- *Cell*: one run of one arm in one scenario, workload family and seed. Cells are the rows of the result table.
- *Scenario*: what happens to the origin (for example "3 times slower from t0", or "30% errors for 10 s").
- *Workload family*: the shape of client traffic (for example Zipf-skewed keys, a flash crowd, a scan).
- *Seed*: the starting number of a pseudo-random generator. Same seed, same random numbers.
- *Tier*: how the number was produced. T1 is simulation. T2 is real processes on loopback. T3 would be physical multi-host (none exists).

**Why it was built.** The project made a claim: an origin-aware adaptive controller should protect the origin better than a tuned static configuration. An early test of that claim turned out to be invalid. The first preregistration used a formula (`static * 0.72`) instead of running anything. The record of that failure is in `docs/research/preregistration_m6_v2.md` section 1 (findings F-05, F-06, F-07, F-10), and a test now fails if the old shortcuts reappear (`test_sensitivity_c_old_shortcuts_are_gone` in `tests/integration/m6_research_evaluation_tests.rs`). The Lab was rebuilt so that every number comes from executed events. [IMPLEMENTED; the rebuild is TESTED by the tests named in this chapter.]

**What the Lab is not.** It is not a benchmark suite, not a production load test, and not evidence about real networks. `src/lab/mod.rs` says so in its first lines: "Every number produced here is tier SIMULATED."

![Figure 7.1: Phantom Lab architecture](../assets/ch07_lab_architecture.svg)

Source: `src/lab/mod.rs`, `src/lab_live/mod.rs`, `src/api/lab_api.rs::router`, `src/bin/phantom-lab.rs`, `ui/src/lib/routes.js`, `lab-results/README.md`.

### 7.1.1 The parts

| Part | Where | What it does | Status |
|---|---|---|---|
| T1 simulator | `src/lab/t1_sim.rs::run_cell` | Runs one cell on a virtual clock, driving the real cache and executor | IMPLEMENTED, TESTED (determinism, hard-cap invariants in smoke cells) |
| Event queue | `src/lab/events.rs::EventQueue` | Min-heap ordered by `(time_us, seq_id)` | IMPLEMENTED, TESTED (`test_t1_event_engine_total_ordering`) |
| Seeded RNG | `src/lab/rng.rs::MasterRng`, `StreamRng` | SplitMix64 streams named per subsystem | IMPLEMENTED, TESTED (`test_t1_master_rng_seed_splitting_determinism`, `test_t1_stream_independence`) |
| Workloads | `src/lab/workloads.rs` | Zipf, scan, flash crowd, diurnal, hot-set shift, mixture, hot key, invalidation | IMPLEMENTED, TESTED (`tests/integration/lab_workload_tests.rs`) |
| Simulated origin | `src/lab/origin_sim.rs::SimulatedOrigin` | Parallel servers, queue, degradation scenarios | IMPLEMENTED, TESTED (`lab_origin_tests.rs`) |
| Runner and CLI | `src/lab/runner.rs::execute`, `src/bin/phantom-lab.rs` | Validated `RunSpec`, writes `cells.csv` and `manifest.json` | IMPLEMENTED, TESTED (`lab_runner_lib_tests.rs`) |
| M6 pipeline | `src/lab/m6_pipeline.rs::run_pipeline` | Full registered matrix, analysis files, mechanical verdict | IMPLEMENTED, TESTED on synthetic matrices; run once on sealed seeds |
| Analysis | `src/lab/analysis.rs`, `src/lab/eval.rs::analyze_endpoints` | BCa interval, sign-flip permutation test, Holm adjustment | IMPLEMENTED, TESTED on planted data; no independent reference check (G4C item (ii)) |
| T2 runner | `src/lab_live/scenario_b.rs` and siblings | Real gateway process on loopback, scripted origin, open-loop load | IMPLEMENTED, TESTED (mechanics; debug-profile smoke runs) |
| Lab API | `src/api/lab_api.rs` | Starts and observes runs, evidence views, SSE events (`lab-1`) | IMPLEMENTED, TESTED (`tests/integration/lab_api/`); off by default |
| UI | `ui/src/screens/lab/*` | About, Start a run, Runs, Evidence tabs | IMPLEMENTED; read for this chapter, not run |

Neither the T1 simulator nor the T2 runner has had a human or external review. The G4C report (`docs/release/g4/cycle1_lab_intelligence_deps_review.md`) is an AI second look and it did not read the event loop in `t1_sim.rs`, `origin_sim.rs` or `workloads.rs` (`docs/release/known_limitations_frozen_code.md` section 1). This chapter's author also read those files, but that is another AI read, not independent verification. Status for the sealed M6 result: **not INDEPENDENTLY VERIFIED**.

---

## 7.2 Deterministic simulation, step by step

### 7.2.1 Virtual clock

**Simple explanation.** A game pauses and fast-forwards. A simulation does the same with time. Nothing waits for a real second to pass. The simulator jumps from one event to the next and sets "now" to that event's timestamp.

**Technical terms.** *Virtual time* is an integer count of microseconds (`u64`) that the simulation controls. *Monotonic* means it never goes backwards. *Discrete-event simulation* means state changes only at event timestamps.

**How it works.** `src/lab/clock.rs::VirtualClock` holds `time_us`, converts to milliseconds with `now_ms = time_us / 1000`, and `advance_to` panics if asked to go backwards. The product cache API takes `now_ms` as an argument, so the simulator can pass virtual milliseconds to the real cache. `t1_sim.rs::start_attempt` does exactly this (`let now_ms = t / 1000;`).

::: caution
**Discrepancy found.** The Lab struct `lab::clock::VirtualClock` (`src/lab/clock.rs`) is exercised only by `tests/integration/lab_core_tests.rs::test_t1_virtual_clock_monotonicity`. `run_cell` does not instantiate it. The loop takes `t = ev.id.time_us` from the event it popped, and the heap guarantees non-decreasing order. The effect is the same (virtual, monotonic time), but a reader who expects the `VirtualClock` struct in the event loop will not find it. Grep of `src/` and `tests/` shows no other user of `lab::clock::VirtualClock`. A different struct with the same name, `sim::clock::VirtualClock` in `src/sim/clock.rs`, belongs to the legacy simulation module (it is used by `src/main.rs` and `tests/unit/wp1_tests.rs`) and is not the Lab clock. [verified in code]
:::

**Where wall-clock time still enters.** The header of `t1_sim.rs` states that "the only wall clock used is inside the product cache (`shrink_shard`), which does not affect outcomes in the configurations used here; determinism is asserted by tests." That is the author's statement plus tests; I did not independently prove it. [UNVERIFIED beyond the tests]

### 7.2.2 The event queue and tie-breaking

**Simple explanation.** A to-do list sorted by time. If two items have the same time, the one added first goes first. Without this rule, two events at the same microsecond could swap order from run to run.

**How it works.** `events.rs::EventId` compares `(time_us, seq_id)` in reverse so Rust's max-heap `BinaryHeap` acts as a min-heap. `seq_id` is a counter that increases by one per `schedule` call. Complexity is O(log n) per push or pop.

::: example
**Worked example: tie-breaking.** Suppose a client `Timeout` (sequence 7) and the next `Arrive` (sequence 8) are both scheduled for t = 1,000,000 us. The heap returns sequence 7 first. Run it again and it returns sequence 7 first again, because sequence numbers are assigned in program order, and program order is fixed by the seed. Without the sequence number, the order of equal-time events would depend on heap internals. (The numbers are illustrative; the ordering rule is the verified part.)
:::

### 7.2.3 Seeded random streams

**Simple explanation.** A seed is like the shuffle order of a deck you can reproduce. Instead of one deck for everything, the Lab gives each concern its own deck, derived from the master seed. Then changing how many random numbers the origin consumes cannot disturb the arrival times.

**Technical terms.** A *pseudo-random number generator* (PRNG) produces a repeatable sequence from a seed. *SplitMix64* is a small PRNG with a 64-bit state. *Stream splitting* derives independent-looking streams from one master seed by name.

**How it works.** `rng.rs::StreamRng::next_u64` adds a constant to its state and mixes the bits (the SplitMix64 steps). `MasterRng::stream(name)` hashes the master seed with the stream name bytes, then discards two outputs as warm-up. `run_cell` takes the `"workload"` and `"origin"` streams from `MasterRng::new(spec.seed)`. `next_exponential(lambda)` gives Poisson inter-arrival gaps: `-ln(1-u)/lambda`.

**Zipf keys in O(1).** `AliasSampler` is Vose's alias method. It spends O(N) once to build two tables, then each sample costs one random integer and one random float. `new_zipf(n, alpha)` sets weights `1/k^alpha`. A reader who knows the Zipf law ("a few keys get most requests") can see why this is the right sampler for cache workloads.

::: example
**Worked example: why alias sampling is exact.** Take three keys with weights 1, 1, 2 (sum 4, n = 3). Scaled probabilities are `w*n/sum` = 0.75, 0.75, 1.5. Keys 0 and 1 are "small" (below 1), key 2 is "large". Pair small key 0 with large key 2: `prob[0] = 0.75`, `alias[0] = 2`, and key 2's remaining mass becomes `1.5 + 0.75 - 1 = 1.25`. Pair key 1 with key 2 the same way: `prob[1] = 0.75`, `alias[1] = 2`, key 2's mass becomes `1.25 + 0.75 - 1 = 1.0`. Sampling picks a slot uniformly (1/3 each), then keeps it with probability `prob`. P(key 0) = 1/3 x 0.75 = 0.25. P(key 1) = 0.25. P(key 2) = 1/3 x 1.0 + 1/3 x 0.25 + 1/3 x 0.25 = 0.5. These match the weights 1/4, 1/4, 2/4. (My arithmetic from the algorithm in `rng.rs::AliasSampler::new`; the code test is `test_vose_alias_sampler_accuracy_and_o1_speed`.)
:::

**Three disjoint seed sets.** `rng.rs::SeedSets` defines: tuning seeds T = 10001..=10060 (60 seeds), development seeds D = 20001..=20030 (30 seeds), and a sealed set S of 30 seeds derived at run time from a secret salt whose SHA-256 commitment is in the source. The old plaintext range 30001..=30030 is "burned" and rejected by leak checks. The salt is not in the repository, and this chapter does not reproduce the commitment value. [IMPLEMENTED, TESTED: `test_seed_sets_disjointness_and_sealed_leak_detection`, `test_seed_sets_disjoint_and_sealed_set_is_hash_only`]

### 7.2.4 The workload schedule

**Simple explanation.** The workload generator answers two questions again and again: "how long until the next request?" and "which key?" Because both answers come from seeded streams, the whole schedule is fixed by the seed before any cache logic runs.

**How it works.** Each generator implements `WorkloadGenerator::next_request(now_us, rng) -> RequestSpec { arrival_delta_us, key, body_size_bytes, ttl_ms, is_invalidation }`. The loop is *open loop*: `Ev::Arrive` draws the next arrival immediately, independent of system state. This matters. A closed-loop generator waits for each reply before sending the next request, so a slow system quietly receives less load (the "coordinated omission" problem). Open loop keeps offering the same load while the system struggles. [verified in code: `t1_sim.rs` `Ev::Arrive` handling comment "open loop"]

`t1_sim.rs::make_workload` maps family ids to generators:

| Family id | Meaning | Used in |
|---|---|---|
| `W-Z(alpha)` | Zipf skew | tuning |
| `W-FC` | flash crowd (rate x2 on 8 keys for 6 s after warm-up) | tuning |
| `W-SC` | scan (20% one-hit keys) | tuning |
| `W-DI` | diurnal sinusoid | held-out |
| `W-SH` | hot-set shift at mid-run | held-out |
| `W-MX` | 90% Zipf plus 10% invalidation stream | held-out |
| `W-HK` | an emerging hot key | held-out |
| `W-SF` | adversarial bimodal, phases of 2.5 s | sealed family, refused by the runner |
| `W-TR` | real trace | UNAVAILABLE: panics with "no trace data in the repository" |

So every T1 workload is synthetic. There is no production trace anywhere in the repository. [verified in code]

### 7.2.5 The simulated origin and scenarios

**Simple explanation.** The origin is a shop with `c` tills. A request takes a till for some service time; if all tills are busy, it queues. A scenario script then makes the shop worse: slower service, fewer tills, random failures, or a full outage.

**How it works.** `origin_sim.rs::SimulatedOrigin` keeps a min-heap of "server busy until" times. `fetch(now_us, key, body, rng)` first decides whether this fetch fails (outage, brownout, flapping), then finds the start time (now, if a till is free; otherwise when the earliest till frees), multiplies a sampled service time by the latency and per-key cost factors, and returns `finish_service_us`. Failures return in 100 us. The M6 standard parameters (`SimParams::standard()`): 8 servers, constant 60,000 us service time, 250 requests per second, catalogue 400 keys, Zipf alpha 0.95, TTL 3,000 ms, 1 KiB bodies, warm-up 8 s, measurement 20 s.

The eight M6 scenarios (`t1_sim.rs::scenario_for`):

| Id | What happens to the origin |
|---|---|
| O-DEG | service time x3 from the start of measurement |
| O-BRN | 30% of origin fetches fail (HTTP 500) for 10 s |
| O-SLW | full outage for 4 s, then capacity ramps 25% to 100% over 8 s |
| O-CO | cache invalidated at t0 and 2x arrival burst for 5 s (cold restart) |
| O-RT | service x2.5 and clients retry failures (up to 3, 100 ms backoff) |
| O-FL | flapping: 4 s cycle, 1.5 s down (HTTP 503) |
| O-HET | per-key origin cost differs for the whole run |
| O-GRD | latency rises linearly from 1x to 4x over the window |

::: caution
**Model limits, by reading the code.** With constant 60 ms service and Poisson arrivals, the origin is closest to an M/D/c queue; the module header says "M/M/c". [our reasoning] The model has no connection pool, no TCP, no memory pressure, no garbage collection and no network RTT. Offered load is 250 rps; if every request missed, 8 servers at 60 ms each could serve at most 8 / 0.06 = about 133 per second, so the cache must absorb most requests for the origin to cope. Whether the chosen load stresses the origin as intended in each scenario is a modelling decision documented in the preregistration, not something independently validated.
:::

### 7.2.6 What a "real component" means in T1

The T1 header lists what is real and what is simulated. This matters for the defence, so here it is as a table.

| Piece | In T1 | Why it matters |
|---|---|---|
| `PartitionedCache` (storage, SIEVE eviction, byte accounting, epochs) | REAL product code, in process | Bugs in the cache code can show up in the simulation |
| `begin_fetch_atomic`, `finalize_fetch` (single-flight, generation fencing) | REAL | The coalescing and fencing logic is exercised, with `oneshot` channels for waiters |
| `check_stale_serving`, `record_stale_served` | REAL | Stale rules and the hard cap counters are exercised |
| `DefaultPolicyExecutor` origin gate, AIMD controller, `OriginAwareStaleController`, `FrequencyAdmissionFilter` | REAL library code | The same code the gateway runs when `PHANTOM_INTELLIGENCE` is canary/active, for the cap only |
| Client arrivals, timeouts, retries | SIMULATED | A modelling choice |
| Origin service times, failures | SIMULATED | A modelling choice |
| Network, TLS, HTTP parsing, Axum handlers, Tokio scheduling | ABSENT | The serving path is not run in T1 |
| Cluster / peers | ABSENT | T1 is single cache |

So T1 tests the *policy logic and cache semantics* under a modelled origin. It does not test the HTTP gateway. That is why T2 exists.

![Figure 7.2: One turn of the T1 event loop](../assets/ch07_sim_event_loop.svg)

Source: `src/lab/t1_sim.rs::run_cell`, `src/lab/events.rs::EventQueue`, `src/lab/origin_sim.rs::SimulatedOrigin::fetch`, `src/lab/workloads.rs`.

### 7.2.7 Arms: different configurations, no multipliers

An *arm* is a configuration of the same event loop (`t1_sim.rs::ArmConfig`): does it coalesce, serve stale-while-revalidate, serve stale on error, which stale window, which cap (`CapConfig::None`, `Static`, `Aimd`), a lab-only circuit breaker, TinyLFU admission. `arm_config` builds them. Examples:

- `BL-0`: no coalescing, naive: every miss goes to the origin.
- `BL-1`: single-flight only.
- `BL-4`: standalone AIMD limiter.
- `BL-12b`: static baseline: single-flight plus stale-while-revalidate plus stale-if-error plus a static cap, with values tuned on tuning seeds T. The code comment says it is "not claimed to be the best deployable static policy".
- `BL-6`: per-scenario oracle static (cap and stale window tuned for each scenario).
- `PIE-Retained`: the adaptive arm: AIMD cap plus dynamic stale window plus TinyLFU admission.
- `PIE-no-stale`, `PIE-no-cap`, `PIE-evict-only`: ablations. `PIE-off`, `CTRL-nonadaptive`, `PLANT-CAP4`: sensitivity checks.

"PIE" in these arm names is a historical label for the adaptive bundle. It is **not** the legacy `src/control/pie.rs` stub, which is `pie: None` in the running gateway (CLAUDE.md section 6b). See Chapter 5 for the intelligence machinery.

---

## 7.3 Reading the numbers: metrics, paired differences and BCa intervals

### 7.3.1 Metrics in a cell

Each cell produces a `CellSummary` (`t1_sim.rs`): request counts by outcome (fresh hit, miss served, stale served, failure), `origin_dispatches`, `peak_outstanding` (the peak number of origin fetches in flight), `peak_servers_busy`, `cap_violations`, stale-cap violation counters, `p50_us`/`p95_us`/`p99_us`, `p99_success_us`, `fresh_hit_ratio`, `error_rate`, `final_cap`, and `bytes_audit_equal`. Latencies are virtual microseconds, not wall-clock measurements.

**Percentile definition.** `t1_sim.rs::percentile` is nearest-rank: `rank = ceil(n*q)`, index `rank-1`. For n = 1,000 requests and q = 0.99, rank 990, the 990th smallest value. Other places in the repository use other definitions (G4A-24 and F24; see Chapter 6). Do not compare percentiles across places without checking the definition.

::: caution
**The primary P99 includes failures.** `M_P99` takes the P99 over *all* logical requests, and a request that is shed or errors enters at the time the failure was known, which can be near zero. A policy that sheds quickly can therefore look better on this endpoint. The v2.2 amendment records this in section 2, and success-only P99 (`p99_success_us`) is reported as a secondary endpoint. [verified in code and amendment]
:::

### 7.3.2 The comparison: paired by seed

The registered analysis compares a *treatment* arm with a *control* arm on the same seeds (`eval.rs::analyze_endpoints`). For each seed it averages the metric over the chosen families, then computes a *relative reduction*:

```
r_i = (control_i - treatment_i) / control_i
```

Positive `r_i` means the treatment is lower (better for peak concurrency and latency). Pairing by seed removes the large seed-to-seed variation (a bad seed is bad for both arms). If the control value is zero, the code sets `r_i = 0` (a documented weakness: G4C F16).

### 7.3.3 Bootstrap and BCa, in plain English

**Simple explanation.** You have 30 numbers (one `r_i` per seed). You want a range for the true average effect, but you only have this one sample. The bootstrap pretends the sample is the whole world: draw 30 values *with replacement*, take the mean, repeat 10,000 times. The spread of those 10,000 means shows how much the average could wobble. The middle 95% is a confidence interval.

**Why "BCa".** The simple percentile interval can be biased if the bootstrap distribution is lopsided. **BCa** (bias-corrected and accelerated) fixes two things:

- *Bias correction* `z0`: how far the bootstrap means sit above or below the original mean (the fraction of bootstrap means below the observed mean, turned into a normal score).
- *Acceleration* `a`: how much the spread changes as the true value changes, estimated from a leave-one-out (jackknife) calculation. It captures skew.

BCa then picks different percentile positions than 2.5% and 97.5%. `analysis.rs::StatisticalEvaluator::bca_ci_mean(data, resamples, seed)` implements it with B = 10,000, a SplitMix64 stream for resampling, and Acklam's approximation for the inverse normal. Constant data returns `(mean, mean)`, a zero-width interval (G4C F06 notes this for degenerate cells).

**Sign-flip permutation test.** Under "no effect", each paired difference is equally likely to be positive or negative. So flip the sign of each `r_i` at random and see how often the flipped mean is as extreme as the real mean. For n at most 16 the code enumerates all `2^n` sign patterns exactly; above that it samples 20,000. The p-value is the fraction at least as extreme.

**Holm correction.** With 16 primary hypotheses (8 scenarios x 2 endpoints) some will look significant by chance. `HolmBonferroni::adjust` sorts p-values, multiplies the smallest by m, the next by m-1, and so on, keeping the sequence monotone. A hypothesis is Holm-significant if the adjusted p is below 0.05.

**The registered "supported" rule per hypothesis:** Holm-significant AND mean relative reduction at least `delta_min = 0.05` AND BCa lower bound above 0. "Adverse" means the BCa *upper* bound is below 0 (credibly worse).

::: example
**Worked example (invented numbers, for teaching only; not a result of this project).** Five seeds give relative reductions `r = [0.10, 0.02, -0.05, 0.08, 0.05]`.

1. Mean = (0.10 + 0.02 - 0.05 + 0.08 + 0.05) / 5 = **0.04** (4% lower peak concurrency on average).
2. Exact sign-flip test: there are 2^5 = 32 sign patterns. I enumerated them: 8 of 32 give a sum whose absolute value is at least the observed 0.20, so p = 8/32 = **0.25**. Not significant. (With so few seeds the smallest possible p is 2/32 = 0.0625, so five seeds can never reach p < 0.05; this is why the registered design uses 30 seeds.)
3. Percentile bootstrap, B = 10,000 resamples with an arbitrary generator I ran for this illustration: the 95% interval came out near [-0.01, +0.08]. It contains zero.
4. BCa ingredients: the fraction of bootstrap means below 0.04 was about 0.46, so `z0` is slightly negative (about -0.09); the jackknife leave-one-out means are 0.025, 0.045, 0.0625, 0.030, 0.0375 giving acceleration `a` of about -0.047. BCa shifts the endpoints a little relative to the percentile interval but does not change the story.

Reading: a 4% average gain, p = 0.25, an interval that includes zero and extends below zero. Under the rule above this is **not supported** (Holm cannot rescue p = 0.25, and the lower bound is not above 0). Also, the interval's upper end (about 8%) stays above `delta_min = 5%`, so the data cannot exclude a 5% effect either. That is the honest reading: "inconclusive with five seeds", not "no effect".
:::

(The rule for inconclusive versus not supported in Scenario B, T2, is in section 7.5.)

### 7.3.4 Check that the statistics work

`SyntheticAnalysisValidator::verify_analysis_pipeline` plants a +8% effect, a -8% effect and a null effect in synthetic data and checks that the interval detects the first two and spans zero for the third. `test_bca_permutation_and_holm_on_synthetic_data` and the sensitivity tests (`test_sensitivity_a_disabling_the_adaptive_mechanism_yields_zero_effect`, `test_sensitivity_b_planted_real_effect_is_detected`) test the end-to-end path. These show the pipeline *can* detect a planted effect. They do not show that BCa was independently reproduced against a reference implementation (G4C section 6 item (ii) lists that as undone), and the validator's interval is a percentile bootstrap, not BCa (`analysis.rs::bootstrap_ci` is used by `evaluate_paired`; BCa is a separate function). [verified in code]

---

## 7.4 Reproducibility, preregistration and evidence retention

![Figure 7.3: From question to retained evidence](../assets/ch07_experiment_lifecycle.svg)

Source: `src/lab/runner.rs::execute`, `src/lab/eval.rs::run_matrix_with_progress`, `src/lab/m6_pipeline.rs::run_pipeline`, `docs/research/preregistration_m6_v2_2_amendment.md` section 10.

### 7.4.1 Deterministic replay

**Claim.** The same `RunSpec` and the same source give byte-identical `cells.csv`. [IMPLEMENTED; TESTED]

**Tests.** `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes` runs a spec twice and compares bytes. `lab_reproduction_tests.rs::test_t1_10_rerun_bitwise_determinism` reruns one cell ten times and compares four summary fields (origin fetches, peak concurrency, hit ratio, stale-cap violations). `m6_research_evaluation_tests.rs::test_runner_is_deterministic` is a third check.

**What these tests do not prove.** They run in one process on one machine. They do not show equality across operating systems, Rust versions or CPU architectures (floating point and `HashMap` iteration could differ). Source hashes in the manifest let you detect that the code changed; they do not make it portable. [limit]

### 7.4.2 Arm interleaving

`eval.rs::run_matrix_with_progress` loops seed, then scenario, then family, then arms rotated by `(seed_index + scenario_index) mod n_arms`. The comment names this as interleaving. In a simulation there is no warm-machine effect to balance, so the rotation is a defence-in-depth habit from real experiments (T2 uses the same idea with real warm-up effects). [our reasoning]

### 7.4.3 Common random numbers

The O-BRN scenario draws a random failure for each origin fetch. If the number of earlier fetches changed the random draw, two arms would see *different* fault luck for the same request, and some of the "effect" would be luck. Protocol v2.2 (finding M2) fixes this: `SimulatedOrigin::with_fault_crn(seed, 10_000)` makes the fate of a fetch a function of `(seed, key, 10 ms bucket)`. All arms then see the same faults. Residual caveat from the amendment: two requests for the same key in the same 10 ms bucket share a fate. Test: `test_brownout_faults_use_common_random_numbers_across_dispatch_orders`. [IMPLEMENTED, TESTED]

### 7.4.4 Preregistration and the freeze

**Simple explanation.** Before you open the sealed envelope, you write down exactly how you will read it. Then no one can accuse you of choosing a favourable reading afterwards.

**What exists.**

1. A preregistration text (`docs/research/preregistration_m6_v2.md`) with hypotheses, endpoints and the analysis. H1 (primary): the adaptive arm lowers peak origin concurrency and client P99 versus the tuned static baseline by at least 5%, with at most 1 percentage point of hit-ratio loss and zero cap violations.
2. Two amendments before the sealed run (`preregistration_m6_v2_1_amendment.md`, `preregistration_m6_v2_2_amendment.md`). v2.1 replaced a sealed-salt commitment whose salt had been generated inside an agent session; v2.2 corrected defects found by an independent pre-run review and fixed a mechanical verdict rule (`m6_pipeline.rs::compute_verdict`).
3. A freeze record (`preregistration_m6_v2_2_freeze.json`) that pins SHA-256 values of the preregistration files and 56 source files (counted from G4C section 2) and the salt commitment. `eval.rs::verify_freeze` and `verify_freeze_with_addendum` check them and fail closed.
4. A run-once marker (`lab-results/m6_sealed_run_started.marker`, created by `create_run_once_marker`) written before cell 1.
5. A leak scan (`scan_for_seed_leak`) that looks for the sealed seed values in the repository text.

**Honest limits of this mechanism.**

- Amendment 7b (adaptivity requirement `max >= 4*min`) was added *after* the researchers saw tuning and development results, but before any sealed data. The amendment discloses this as a "researcher degree of freedom". The unconstrained tuned optimum on T was a static cap (min = max = 2), so, in the amendment's own words, the sealed test is "a forced-adaptive AIMD controller versus the best static configuration found on T".
- The freeze check covers 4 of 56 files that now differ because of a pinned addendum (`Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, test file). G4C reproduced that the differences match the addendum; whether editing `eval.rs` after the sealed run is acceptable is a human decision (known_limitations section 2).
- The salt was generated by an agent process and is readable by same-user processes (the freeze record's own limitations, quoted in G4C). The leak scan is decimal-substring only (F19). `verify_no_sealed_leak` is vacuous in normal runs because the salt is not in the environment (F32).
- The sealed manifest shows 64 `git_status_entries`: the sealed run executed on a dirty working tree. Source hashes were recorded per file (`source_sha256`), which is the defence. [verified: read from `lab-results/m6_sealed_2026-10-29T234845Z/manifest.json`]

Nobody outside the project has audited this procedure. [not INDEPENDENTLY VERIFIED]

### 7.4.5 Manifests and evidence retention

Every run folder has a `manifest.json`. For T1 matrices (`eval.rs::run_matrix_with_progress`): manifest version, tier `SIMULATED`, scenario/family/arm lists, per-arm configuration JSON, sim parameters, `git_head`, `git_status_entries`, per-file `source_sha256`, build profile, host OS and architecture, UTC start and end times, SHA-256 of each output file, and the interleaving rule. For sealed runs the seed values are `null` and labels are `S00`...; the commitment is recorded. For T2 (Scenario B): manifest plus `summary.json` plus one folder per (repetition, arm) with client, origin and gateway-snapshot files (`docs/demo/scenario_b_artifacts.md`).

`lab-results/README.md` states the retention rules: do not edit, rename or delete; the sealed and frozen material has hash records in `docs/research/preregistration_m6_v2_*freeze.json`; about 48 MB total. The `runs/` folder is empty at the time of writing, which means no API-started run is retained.

::: caution
**Two manifest systems exist.** `src/lab/manifest.rs::RunManifestV2` (provenance tags, EV-1 to EV-14 checklist) is **not** the manifest that real runs write. It hard-codes a git hash and `is_dirty: true`, implements 3 of 14 validity checks (EV-1, EV-2, EV-11), serialises through a `HashMap` so its digest is not deterministic, and is used only by tests (`lab_core_tests.rs`). Real runs write the JSON manifest built in `eval.rs`. G4C finding F04. [verified in code]
:::

### 7.4.6 How to reproduce an experiment

For a retained T1 result:

1. Read `manifest.json` in the result folder: note `git_head`, `source_sha256`, `build_profile`, `arm_params`, scenarios, families, arms, seed labels.
2. Check out that source state (the working tree was dirty, so use the per-file hashes to confirm your files match).
3. Build in release. The sealed test requires it (`require_release_build`).
4. Re-run: for a development-seed run use `phantom-lab run --scenarios ... --families ... --arms ... --seed-set D --n-seeds N --profile standard --arm-params frozen --out DIR` (flags from `src/bin/phantom-lab.rs`). The M6 dry run is the ignored test `m6_full_matrix_dry_run_on_dev_seeds`.
5. Compare `cells.csv` bytes or SHA-256 with the retained file.

For the sealed result, step 4 is impossible for anyone without the secret salt. Anyone can still re-run the *analysis* on the retained `cells.csv` and check the verdict, which is the check the pipeline itself performs (it re-reads the raw file from disk). I did not re-run this analysis for this chapter; I read the retained `verdict.json`. [read, not reproduced]

For T2: re-running produces a new sample. Compare against the retained summary using the stated intervals, not byte equality.

---

## 7.5 T2: Scenario B on a real gateway over loopback

**Simple explanation.** T1 asks "does the policy logic behave on a model?" T2 asks "does the real program, with real sockets and a real clock, behave the same way?" It starts the actual `phantom-cache-demo` binary as a child process, points it at a small scripted origin, fires seeded traffic at it, and records what happens.

### 7.5.1 The experiment

Source: `src/lab_live/scenario_b.rs`. Contract: `docs/demo/scenario_B_origin_slowdown.md`.

- **Arms.** `BASE` (intelligence `off`, no cap), `STATIC-CAP` (cap 24; `PHANTOM_INTELLIGENCE=canary`, min = max = 24), `AIMD-CAP` (min 12, max 32, p99 ceiling 60 ms, window 30; canary).
- **Phases.** Warm-up 8 s, healthy 5 s, degraded 10 s, recovery 8 s (default). During the degraded phase the scripted origin multiplies its 20 ms service time by 6.
- **Workload.** Poisson arrivals at 300 rps, 5,000 keys, Zipf alpha 0.9, seed `20261030 + repetition`, identical across the arms of a repetition. The schedule comes from `loadgen.rs::schedule`, which reuses `MasterRng` streams `"arrivals"` and `"keys"` and `AliasSampler` from the T1 code. Open loop: each request is sent at its scheduled time regardless of earlier replies; latency is measured from the scheduled time, so a late send still counts.
- **Gateway control.** `gateway.rs::gateway_env` removes inherited `PHANTOM_*` and `ORIGIN_URL` variables, sets bind address, cache size, `PHANTOM_INTELLIGENCE` and the cap keys; the child is killed on drop. The runner polls `GET /api/v1/metrics` every 250 ms.
- **Repetitions.** 10 per arm, arms interleaved rep-major in a rotating order.
- **Origin.** A raw-TCP scripted delay server (`origin.rs`). Service time does not grow with load, so a real origin's positive-feedback overload is **not** reproduced.
- **Analysis.** `lab_live/analysis.rs::paired` calls the same `bca_ci_mean` and `sign_flip_p_value` as T1. `evaluate_arm` applies a fixed rule in this order: (1) mechanism failure if origin concurrency exceeded the cap in any run; (2) inconclusive if fewer than 10 valid pairs, host contention flagged (send-lag P99 above 100 ms), or the interval includes zero but its upper bound is at least 5%; (3) benefit not supported if the mean reduction is under 5% or the interval does not exclude zero; (4) trade-off failure if mean client success is more than 1 percentage point below BASE (shed 503s count as failures); (5) benefit not supported if origin P99 is worse; (6) otherwise benefit supported.

### 7.5.2 The retained result and its qualifiers

Retained: `lab-results/scenario_b/20261030T003932Z/`. Release-profile gateway and driver (manifest), host Windows 11 (build 10.0.26300), x86_64, 24 logical CPUs (`Intel64 Family 6 Model 183`), RAM not collected, loopback, 10 repetitions, seed 20261030 + r. Label `VALID` in the sense of `LABEL_VALID_MEANING` only.

| Quantity (mean over 10 paired repetitions) | BASE | STATIC-CAP (24) | AIMD-CAP (12-32) |
|---|---|---|---|
| Origin peak concurrency, degraded phase | 39.0 | 24.0 | about 13.1 |
| Reduction vs BASE, 95% BCa interval (window metric `origin_peak_concurrency_window`; the degraded-phase means above give 38.5% and 66.4%) | | 38.3% [36.7, 40.2] | 64.0% [62.5, 65.3] |
| Client success rate | 100% | 95.05% (-4.95 pp) | about 82.9% (-17.1 pp) |
| Mechanical verdict | | TRADE_OFF_FAILURE | TRADE_OFF_FAILURE |
| All-request client P99 (mean of per-run P99s) | 167.1 ms | difference -0.24 ms, interval [-1.10, +0.47], p = 0.59 | not tabulated here |

(Source: `summary.json` `verdicts`, `arms`, `paired_vs_base`; read for this chapter. Host, build and seed as above. These figures were produced by the project; I did not re-run them. Independent recomputation by the project is in `docs/demo/scenario_b_verification/`, reported by that document, not repeated by me.)

What this shows, in plain English: a hard cap on origin concurrency really limits origin concurrency (the static arm peaked at exactly 24 in all 10 runs), and the price is that some requests are refused with a 503. This is the expected shape of any cap.

What the dated interpretation notice (`docs/demo/scenario_b_interpretation_notice.md`) says **must** accompany every citation:

1. The adaptive arm fell from 32 to about 12 within about 2 s and stayed near 12 in every phase, shedding about 2% even when healthy. So AIMD-CAP versus BASE was effectively a static cap near 12 versus no cap. The experiment shows nothing about adaptive-controller merit.
2. The controller's 30-sample "p99" is the window maximum, compared with an absolute 60 ms ceiling that timer overshoot and healthy variation can cross.
3. All-request latency includes fast-failing 503s and is not evidence of improved latency for served clients.
4. The scripted origin has no load-dependent latency, so the origin-P99 clause is untestable.
5. Stale serving never happened (`stale_served_total` is 0; the origin sent only `max-age=3`).
6. The caps were chosen after a single-repetition pilot and were not preregistered.
7. Arms differ by more than the cap: the capped arms run `canary` and BASE runs `off`. No cap-free canary arm exists.

The code carries these as constants (`AIMD_EFFECTIVE_CAP_NOTE`, `CANARY_VS_OFF_NOTE`, `ALL_REQUEST_LATENCY_NOTE`, `ORIGIN_P99_NOTE`, `STALE_SERVING_NOTE`, `PILOT_DISCLOSURE`, `CAN_NOT_ESTABLISH`) and the Lab API ships the notice with every Scenario B evidence payload (`lab_api.rs::SCENARIO_B_NOTICE`). That is a good design for honesty: the caveat travels with the number.

::: caution
**Known open defects in the integrated Lab code.** G4C F01 (High): a capped arm with no run data can get the verdict `MECHANISM_FAILED`. F02: label `VALID` can appear on cancelled or partial summaries. Their remediation (U-01) is sandbox only: the owner has approved integration with conditions (DEC-001) and it is in progress, but at the time of writing it is **not integrated**, not committed and not human-verified (`cycle1_findings_disposition_register.md`: IN-SANDBOX-REMEDIATION). I did not re-run the code path; the status is from the register. The retained experiment is not affected by them in the sense that all 10 pairs per arm had data (the summary shows `valid_runs` 10 and `mechanism_pass` true).
:::

### 7.5.3 The exact relationship between T1 and T2

| Question | T1 (`src/lab`) | T2 (`src/lab_live`) |
|---|---|---|
| Label | SIMULATED | MEASURED_LOOPBACK; shown in the UI as "Real (loopback, one host)" |
| Clock | virtual, integer microseconds | real time (Tokio timers; resolution host-dependent) |
| Product code that runs | cache, coalescing, executor, candidates, as a library | the whole gateway binary: HTTP, Axum, Tokio, cache, executor |
| Origin | modelled queue with scenario scripts | scripted raw-TCP delay server, no internal queue |
| Load | modelled arrivals in the same event loop | real sockets, open-loop sender |
| Repeatability | byte-identical for the same source and spec | statistically similar, not bit-exact |
| Scenarios | 8 degradations (M6) x 4 held-out families | one: origin slowdown (Scenario B) |
| Sample size | 30 sealed seeds x 12,000 cells | 10 repetitions x 3 arms |
| Shared code | `MasterRng`, `AliasSampler`, `bca_ci_mean`, `sign_flip_p_value` | the same functions, imported |

What T2 adds over T1: it shows the cap is enforced by the real binary (mechanism pass in every run) and that the shape of the trade-off (lower origin peak, some 503s) appears outside the model. What it does not add: any agreement study. The class `matrix_runner.rs::AgreementStudy` computes "rank concordance" between T1 and T2 cells, but (a) it compares *positional* rank matches, not a Spearman coefficient despite the return-value comment, and (b) its only test (`test_t1_t2_agreement_study_concordance`) feeds it hand-written T2 numbers that the test itself calls "SYNTHETIC FIXTURES that only exercise AgreementStudy mechanics; they are not measurements." The claims ledger therefore keeps C7 ("lab reproduces Tier 2 outcomes within a stated tolerance") as **UNVERIFIED**: "no T2/loopback agreement study was run." [verified in code, test and ledger]

So the correct statement is: T1 and T2 are two separate pieces of evidence about different things. T2 does not validate the T1 model, and T1 does not predict T2. A project that wanted to claim "the simulator is predictive" would need paired T1 and T2 runs of the same scenario and a pre-stated tolerance. That has not been done.

---

## 7.6 The M6 result: a negative outcome, reported as such

**What was tested.** H1: a forced-adaptive AIMD controller (with `max >= 4*min`, tuned on seeds T) beats the best static configuration found on T (`BL-12b`) on peak origin concurrency and client P99, without breaking hit-ratio and error-rate constraints.

**Setup (verified from the retained `manifest.json` and `verdict.json`).** 8 scenarios x 5 families (the 4 held-out families plus the sealed `W-SF`) x 10 arms x 30 sealed seeds = 12,000 cells (`cells.csv` has 12,001 lines including the header). Release build, Windows, started 2026-10-29 23:48:45 UTC, finished about 97 seconds later (23:50:22 UTC per the manifest; the results document says 97 s of test time). Tier SIMULATED.

**Mechanical verdict:** `H1: NOT_SUPPORTED`.

| Item | Result (from `verdict.json` and `m6_sealed_results.md`) |
|---|---|
| Peak origin concurrency, scenarios supported | 0 of 8; the adaptive arm is credibly worse in 7 (it adapts up to 8 concurrent fetches while the static baseline holds 2) |
| Client P99, scenarios supported | 2 of 8 (O-HET about +29%, O-GRD about +5.6%); five scenarios identical (P99 saturated at the failure latency) |
| Hit-ratio constraint (loss at most 1 pp) | fails in O-BRN, O-FL, O-HET |
| Error-rate constraint (increase at most 0.5 pp) | fails in the same three |
| Cap and stale-cap violations | 0 in the adaptive arm (F-d does not fire) |
| Falsifier F-a (fewer than 10% gain in at least 6 of 8 scenarios) | fires: 7 of 8 versus BL-12b |
| H2 (mechanism attribution), H4 (stability) | NOT TESTABLE at the registered horizon: the dynamic stale window never binds; `PIE-no-stale` equals `PIE-Retained` in 1,200 of 1,200 cells |
| H3 (overhead), H5, H6 | UNVERIFIED: not measurable in T1 |

**How to read it.**

- It is a real negative result, produced by a rule written before the sealed data and applied by code. That is a strength of the process.
- The unconstrained tuned optimum on T was already a static cap. On tuning data, no adaptive configuration beat the best static one.
- It does **not** show that adaptive caching is useless. It shows no evidence of benefit for *this* controller (absolute-ceiling AIMD), under *this* protocol, in *this* simulation, with synthetic workloads.
- The statement the project may make (claims ledger section 6): a static origin-concurrency cap reduces peak origin concurrency and P99 versus no cap in the simulated overload scenarios (development data, exploratory). The statement it may not make: that adaptive control improves origin protection or latency.
- The result has not been independently reviewed (known_limitations section 1).

**The product consequence** (research lead, `m6_sealed_results.md`): MVP-B, "the static gateway": single-flight, correct freshness and stale serving, generation-fenced invalidation and a configurable static origin cap. The adaptive controller is not promoted. The serving path today calls only `DefaultPolicyExecutor::new/set_mode/configure_origin_cap`, `acquire_origin_permit`/`OriginPermit::observe`; the rest of `src/intelligence` (shadow evaluation, decision engine, canary routing) has no caller in the serving path (G4C section 5, a grep result I did not repeat). `PHANTOM_INTELLIGENCE=canary` applies the cap to 100% of fetches (G4C F14). [see Chapter 5]

---

## 7.7 The Lab API and UI

### 7.7.1 API

`src/api/lab_api.rs`, schema `lab-1`. Routes (verified in `router`): `GET /api/v1/lab/capabilities`, `GET /api/v1/lab/evidence`, `GET /api/v1/lab/evidence/:id`, `POST` and `GET /api/v1/lab/runs`, `GET /api/v1/lab/runs/:id`, `POST .../cancel`, `GET .../events` (SSE), `GET .../artifacts`, `GET .../artifacts/:name`.

Properties from the module header and constants (reading, with tests under `tests/integration/lab_api/`):

- Off unless `PHANTOM_LAB_ENABLED=true` (`src/config.rs`); the routes do not exist otherwise.
- With JWT enabled, every route authenticates before any side effect: Observer role for reads, Operator for start and cancel. Without JWT (development), only loopback is allowed (`check_lab_bind`) with a Host and Origin guard against DNS rebinding.
- Body cap 16 KiB, read timeout 5 s, at most 16 SSE streams, at most one concurrent run, at most 30 seeds, 20,000 cells, 30 repetitions per request.
- No client-supplied path is joined to a filesystem path. Run ids follow a fixed grammar; artifacts are matched by equality against an allow-list.
- The API refuses seed set S and family W-SF (library validator) and refuses to start if `PHANTOM_SEALED_SALT_HEX` is in the process environment.
- Events are written to `events.jsonl` under one lock with monotone ids (Chapter 6 covers SSE authentication limits: the UI uses fetch streaming with an `Authorization` header, not `EventSource` URLs).

The API starts the same code the CLI and tests use (`runner::execute` and `scenario_b::run_experiment`). It contains "no second implementation and no canned result" (module header; consistent with what I read).

### 7.7.2 UI

`ui/src/screens/lab/` holds the Lab screens (About, Start a run, Runs, Evidence tabs; `ScenarioBResult.jsx`; `ConnectionPanel.jsx` keeps the token in memory). `ui/src/lib/labProvenance.js` maps tiers to display words: `MEASURED_LOOPBACK` appears as **Real** with the qualifier "loopback, one host" (and ", debug build" when known); `SIMULATED` as **Simulated**; `DERIVED`; `UNAVAILABLE`. `REPLAYED` and `PREDICTED` are reserved words with no backend data. The retained-evidence tiers carry a separate "retained" tag. The provenance vocabulary matches CLAUDE.md section 4: every displayed value is labelled measured, simulated, derived, or unavailable. I read the UI source for this chapter; I did not run it in a browser. [IMPLEMENTED; display behaviour UNVERIFIED by me]

The main navigation also has a "Simulation" explainer labelled Simulated, and "Cluster" and "Intelligence" entries marked "Not available" (`ui/src/lib/routes.js`). That is consistent with the backend facts above.

---

## 7.8 Innovation: building the argument from evidence

**The rule for this section.** The project's own prior-art record says RT-0 (the prior-art closure) is **OPEN**: `rt0_prior_art_sweep_2026-10-30.md` begins "Status: PARTIAL. RT-0 is still OPEN after this sweep. This file makes NO novelty claim, now or conditionally." The older `novelty_snapshot_m6.md` says "RT-0 PASS" but carries a correction notice that this is unsupported and that several features it lists (canary bucketing, provenance-gated rollback, AIMD-paced background refresh) are not in the serving path. This chapter therefore does not use the snapshot's matrix. It makes no "first" and no "novel" claim. What it can do is separate three things: what exists elsewhere, what the combination is, and what the evidence demonstrates.

### 7.8.1 What problem the combination addresses

Chapter 1 introduced the problems: stampedes on popular keys, stale data after change, an origin that gets slow and then collapses under retries, and operators who must tune several knobs. A cache gateway can address them with separate mechanisms. PhantomCache's design puts these mechanisms in one HTTP gateway with explicit invariants:

1. single-flight coalescing of concurrent misses (per node);
2. generation-fenced fill, so a fetch that started before a DELETE cannot store a stale body;
3. tenant and global epoch invalidation;
4. RFC 9111-style freshness with stale-while-revalidate and stale-if-error, bounded by a hard stale cap (`PHANTOM_MAX_STALE_MS`);
5. a bounded origin-concurrency gate, with an optional AIMD controller;
6. byte accounting with an audit (`audit_bytes`);
7. a cluster layer: rendezvous placement for DELETE, mTLS peers, invalidation fan-out, an anti-entropy worker;
8. a *lab* that can test these under degradation with a deterministic, preregistered, auditable method.

Details of 1 to 7 are in Chapters 2, 4 and 5. Their statuses are in those chapters. Item 8 is this chapter.

### 7.8.2 Comparison with established systems

Figure 7.4 summarises; the text below gives the evidence class for each statement. The classes are those of the repository's own sweep: **OPENED-DIRECT** (primary text read), **OPENED-VIA-TOOL** (primary page fetched through a summarising tool), **SNIPPET** or **SECONDARY** (not primary). Statements marked *this session* were fetched while writing this chapter, via the WebFetch tool, which returns a summary. They are OPENED-VIA-TOOL, not verbatim reads.

![Figure 7.4: Overlap, difference and maturity](../assets/ch07_comparison_table.svg)

Source: `docs/research/rt0_prior_art_sweep_2026-10-30.md`, `intelligence_prior_art_coverage.md`, `src/dist/router.rs`, `src/lab/t1_sim.rs`, and pages fetched this session.

**Memcached and Redis.**
- *Verified from a primary source (OPENED-DIRECT, sweep T3):* Facebook's "Scaling Memcache" (NSDI '13) section 3.2.1 describes leases: a token bound to the key, invalidated by a delete, which prevents stale sets, and rate-limiting tokens to mitigate thundering herds. The authors report a peak database query rate falling from 17K/s to 1.3K/s over one week for herd-prone keys (authors' data; the intelligence coverage record flags that number as secondary-sourced, class B, in that pass).
- *OPENED-VIA-TOOL (sweep T10):* Redis client-side caching server tracks keys per connection and pushes invalidations; on disconnect the client flushes its whole local cache.
- *Consequence for PhantomCache:* generation fencing against stale sets is the same family of idea as leases. That is not new. What is different is the shape of the product: Redis and Memcached are general key-value servers; PhantomCache is an HTTP-aware caching gateway in front of an origin with freshness rules. The two are not interchangeable and a judge's "why not Redis?" has a clear answer (judge box below).

**Varnish, nginx, Fastly (HTTP caches).**
- *OPENED-VIA-TOOL (sweep T5, T6):* Varnish serves one backend request while others wait on a waiting list, supports grace (stale serving) and `std.healthy` to vary grace by backend health; nginx has `proxy_cache_lock` (default off), `proxy_cache_use_stale updating` and background update; Fastly documents request collapsing and soft purge. RFC 9111 describes collapsing of multiple requests into one forward request.
- *Consequence:* coalescing and stale serving are standard. PhantomCache's coalescer is not novel and the repository's own prior-art record says so ("May state as a standard mechanism").

**groupcache and Go `singleflight` (this session, OPENED-VIA-TOOL).**
- The groupcache README says it coordinates fills so "only one load in one process of an entire replicated set of processes populates the cache", "shards by key to select which peer is responsible", and "supports automatic mirroring of super-hot items to multiple processes." It also says it has "neither cache expiration times, nor explicit cache evictions."
- The `golang.org/x/sync/singleflight` package doc: "provides a duplicate function call suppression mechanism"; concurrent callers with the same key wait and receive the same result.
- *Consequence:* groupcache solves **cross-node** duplicate suppression, which PhantomCache does **not**: in the current tree GET is served from the local cache only (Chapter 5). On that axis groupcache is ahead. PhantomCache adds HTTP freshness, explicit invalidation, stale caps and an origin gate, which groupcache's README says it lacks (no expiry, no explicit eviction).

**Caffeine, TinyLFU, SIEVE, S3-FIFO (eviction and admission).**
- *OPENED-VIA-TOOL (sweep T4):* TinyLFU is frequency-based admission (arXiv:1512.00727); SIEVE (NSDI '24) is a simple FIFO-with-hand policy with author-reported gains over ARC on 1,559 traces; S3-FIFO (SOSP '23, OPENED-DIRECT abstract and introduction) uses three FIFO queues, author-reported lowest mean miss ratio on 10 of 14 datasets. This session: the Caffeine README says it provides "size-based eviction ... based on frequency and recency" and lists refresh-after-write and asynchronous loading; it does not mention stampede coalescing and the page does not name Window-TinyLFU (it cites TinyLFU papers).
- *Consequence:* PhantomCache uses SIEVE as default and a TinyLFU-style admission filter in the lab arms; eviction adaptation is explicitly not the thesis. The repository's sweep says the field is crowded and strong simple baselines are published. No eviction claim is made here. How faithful PhantomCache's policy implementations are is Chapter 4's subject.

**Consistent and rendezvous hashing.**
- *This session (search result, SECONDARY to primary):* the original rendezvous (highest random weight) hashing paper is Thaler and Ravishankar, "Using name-based mappings to increase hit rates", IEEE/ACM Transactions on Networking 6(1):1-14, 1998. The repository's sweep did not search rendezvous literature ("Not searched").
- *Verified in code:* `src/dist/router.rs` computes a hash-times-weight score with `capacity_weight = 1.0` (plain, unweighted rendezvous in practice). Routing is used for DELETE; GET is local (Chapter 5). Bounded-load hashing (Mirrokni et al., arXiv:1608.01350, OPENED-VIA-TOOL abstract in sweep T11) addresses hot nodes under skew; PhantomCache does not implement it.

**Anti-entropy and Cassandra repair.**
- *This session (OPENED-VIA-TOOL):* the Cassandra documentation page calls the function "repair", says it reconciles diverged replicas by comparing data with Merkle trees and streaming only mismatched sections.
- *Consequence:* the idea of periodic reconciliation is established. PhantomCache's `RendezvousRouter::start_anti_entropy_worker` (250 ms) calls `reconcile_peer_anti_entropy`; its effectiveness is not established (claims ledger C6 and its qualifiers: owed invalidations are in issuer memory only and are lost if the issuer dies).

**Adaptive concurrency limits (Envoy, Netflix).**
- *OPENED-VIA-TOOL (sweep T7):* Envoy's adaptive concurrency filter uses a gradient controller that compares sampled latency with a periodically measured minimum RTT; Netflix's concurrency-limits library offers Vegas and Gradient2 variants. Both use *relative* signals.
- *Consequence:* PhantomCache's AIMD (`candidates.rs::OriginConcurrencyCapController::step`: multiply by 0.7 if measured P99 exceeds the ceiling or the error rate exceeds 5%, otherwise add 1) compares against an *absolute* ceiling. The sweep notes this is consistent with the repository's own finding that its AIMD behaved like a threshold switch. It is not evidence about whether a relative controller would help.

**CDNs and request collapsing.** Fastly's guide (OPENED-VIA-TOOL) says collapsing happens per cache node and cluster-wide collapsing needs a shield or tier. PhantomCache's coalescing is also per node. Nothing here is a differentiator.

### 7.8.3 What is genuinely different, stated carefully

Using only what the repository's evidence supports:

1. **The packaging of mechanisms with explicit, testable invariants in one HTTP gateway.** Generation fencing + epochs + stale cap + origin gate + byte audit, with invariants written down (CLAUDE.md section 5; Chapter 4). Each mechanism has precedent; the repository does not claim otherwise. Whether the *combination* offers value over assembling Varnish or nginx with a limiter is **not demonstrated**.
2. **An evaluation method with preregistration and a mechanical verdict.** A deterministic simulator driving the real cache code, a preregistered verdict rule applied by code, a hash-committed sealed seed set, an honest negative result, and provenance labels carried through the API and UI. This is a methodological contribution of this project's practice. It is not a claim that the method is rarer or stricter than anyone else's: no survey of other projects was done.
3. **A real negative result.** H1 is `NOT_SUPPORTED`, the rule was fixed before the data, and the headline finding (the best tuned "adaptive" setting was a static cap) is reported rather than hidden.
4. **A hypothesis the evidence supports in simulation:** a static origin-concurrency cap reduces peak origin concurrency and P99 versus no cap in simulated overload (development data, exploratory), at a cost of refused requests. T2 shows the same shape on a real binary on loopback. This is useful but is not novel science: caps trade availability for protection.

### 7.8.4 The actual technical contribution (what we can defend)

> A working, tested HTTP caching gateway core that combines single-flight coalescing, generation-fenced fills, epoch invalidation, stale-serving with a hard staleness cap and a bounded origin gate, plus a deterministic, preregistered experiment harness that drove the real cache code through eight origin-degradation scenarios and reported, mechanically, that a forced-adaptive controller did **not** beat a tuned static cap.

That sentence is deliberately narrow. It avoids "novel", "first" and "better than".

### 7.8.5 What is demonstrated, and what needs stronger evidence

| Claim | Evidence | Status |
|---|---|---|
| Deterministic T1 replay of cells | `same_runspec_twice_gives_identical_cells_and_bytes`, `test_t1_10_rerun_bitwise_determinism`; one machine | TESTED (limits in 7.4.1) |
| Static cap reduces peak origin concurrency vs no cap (simulation) | dev data per claims ledger section 6 | EVALUATED (exploratory, Tier 1) |
| Cap is enforced by the real binary | Scenario B: static arm peak exactly 24 in all 10 runs | TESTED/MEASURED (loopback, one host, release) |
| Forced-adaptive AIMD beats tuned static | M6 sealed: `NOT_SUPPORTED` | REFUTED in this simulation, for this controller |
| Adaptive stale window helps | sealed horizon: window never binds | NOT TESTABLE |
| Controller overhead | no release benchmark | UNVERIFIED (C3) |
| T1 predicts T2 | no study | UNVERIFIED (C7) |
| Hit rate or latency superiority over Redis, Varnish, etc. | no comparison run | NOT CLAIMED, NOT EVIDENCED |
| Multi-host behaviour | no physical multi-host run | NOT EVIDENCED |
| Novelty of any mechanism | RT-0 OPEN | NOT CLAIMED |
| Independent review of the Lab | none | NOT INDEPENDENTLY VERIFIED |

Evidence that would strengthen the case, in order of cost:

1. Run the M6 pipeline again with a *relative-signal* controller (a gradient or latency-ratio cap) as a new, newly registered arm, with a new protocol and new seeds. Do not reinterpret the completed result.
2. Make the scripted origin load-dependent (a queue with finite capacity) so Scenario B reproduces overload feedback, and add a cap-free canary arm so the cap is the only difference.
3. Pair T1 and T2 runs of one scenario with a pre-stated tolerance (a real agreement study).
4. A release-profile overhead benchmark on a named host for hit-path cost with intelligence off and on.
5. An independent human review of the Lab core and a BCa cross-check against a reference implementation.
6. A multi-host (T3) run and baseline runs against actual Varnish or nginx with the same workload, with fairness rules written in advance.

### 7.8.6 Where established products are more mature (honest list)

- **Production history and operability:** Redis, Memcached, Varnish and nginx run at scale with years of bug-fixing, documentation, monitoring and community. Redis adds data structures, clustering, persistence and clients in every language; Varnish and nginx add VCL programmability, disk storage, TLS offload and broad HTTP coverage; Caffeine has years of library use and published evaluations on thousands of traces (general knowledge, not a repository finding).
- **Cross-node duplicate suppression and hot-item mirroring:** groupcache documents them; PhantomCache's GET is local-only.
- **Invalidation durability:** Fastly and Cloudflare document purge networks; Cloudflare reports global P50 purge latency of 149 ms (sweep T10, OPENED-VIA-TOOL, vendor-reported). PhantomCache's cross-node invalidation is best-effort after the first acknowledgement, and owed invalidations are lost if the issuer dies (claims ledger C6 qualifier; M3 durability is a sandbox proposal, not integrated).
- **Adaptive concurrency control:** Envoy and Netflix ship relative-signal controllers; PhantomCache's absolute-ceiling AIMD is not better than a static cap in the evidence.
- **Eviction:** published evaluations over thousands of traces versus a lab with synthetic workloads only.
- **Security and tenancy:** Chapter 6; no per-tenant resource quotas exist (vision audit).
- **Performance:** there is no head-to-head performance measurement in the repository. Any claim of speed would be unsupported.

### 7.8.7 The "intelligence" question

"Intelligence" in PhantomCache means, at present, small, understandable control rules: an AIMD loop (multiply by 0.7 on a high P99 or more than 5% errors, otherwise add 1), a dynamic stale-window controller, a frequency-based admission filter, and a shadow-evaluation and canary framework in the library. It is not machine learning (the ML gate disposition is unverified: claims ledger C12). On the serving path only the origin gate and the AIMD controller are wired; `src/control/pie.rs` is a threshold stub with `pie: None`; the shadow, canary and decision engine have no serving-path caller. Say "adaptive control", not "AI". The evidence is that it did not beat a static cap in the one test run (algorithms: Chapter 5).

---

## 7.9 Judge boxes

::: judge
**JUDGE MAY ASK**

Question: What is novel about PhantomCache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

I will not claim any single mechanism is new. Request coalescing, stale-while-revalidate, leases against stale writes, adaptive concurrency limits and anti-entropy all exist in Varnish, nginx, Memcached, Envoy and Cassandra. Our own prior-art record is still open and says we may not claim novelty. What I can defend is the combination and the method. The combination is an HTTP caching gateway where fencing, epochs, stale caps, an origin gate and byte accounting are explicit invariants with tests. The method is a deterministic simulator that drives the real cache code, a preregistered mechanical verdict, and an honest negative result: our adaptive controller did not beat a tuned static cap.

**If the judge asks further:**

"Is the combination better?" I do not know, because no head-to-head measurement exists. Showing it would need the same workload against Varnish or nginx with a limiter, with fairness rules fixed in advance.

**Evidence to know:** `docs/research/rt0_prior_art_sweep_2026-10-30.md` (status PARTIAL, RT-0 OPEN), `docs/research/novelty_snapshot_m6.md` (correction notice), `docs/research/claims_ledger.md` sections 6-7.
:::

::: judge
**JUDGE MAY ASK**

Question: Why not just use Redis or Memcached?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

For a general key-value cache, you should. They are mature, widely supported and have more features. PhantomCache is a different shape: an HTTP gateway that sits in front of an origin, applies HTTP freshness rules, serves stale content under a hard cap, coalesces concurrent misses and caps origin concurrency. With Redis you would write that logic in the application. Memcached's leases and Redis client tracking solve parts of the stale-write and invalidation problem; our generation fencing is the same family of idea. The honest trade-off is that we offer less maturity, no persistence and no clustering depth, and no benchmark shows we are faster.

**If the judge asks further:**

"Could you put Redis behind your gateway?" That is a plausible design, but it is not what exists. The cache is in-process memory per node.

**Evidence to know:** `docs/research/rt0_prior_art_sweep_2026-10-30.md` T3 (Facebook leases, OPENED-DIRECT) and T10 (Redis tracking); `src/core/cache.rs` (in-process cache); Chapters 2 and 4.
:::

::: judge
**JUDGE MAY ASK**

Question: What is the actual intelligence in the system?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is adaptive control, not machine learning. The part that touches live traffic is an origin concurrency cap with an optional AIMD controller: if the measured origin P99 exceeds a ceiling or errors exceed 5%, cut the allowed concurrency by 30%; otherwise add one. Around it the library has a dynamic stale-window rule, a frequency-based admission filter, and a shadow and canary framework, but the serving path does not call the shadow and canary parts today. The legacy PIE controller is a threshold stub that is not even instantiated. And the evidence says the adaptive version did not beat a static cap.

**If the judge asks further:**

"Why keep it?" It is available behind explicit configuration, labelled experimental, and the result is published as a negative finding. The product decision, recorded by the research lead, is to ship the static cap path as the MVP.

**Evidence to know:** `src/intelligence/candidates.rs::OriginConcurrencyCapController::step`, `docs/release/g4/cycle1_lab_intelligence_deps_review.md` section 5, `docs/research/m6_sealed_results.md`.
:::

::: judge
**JUDGE MAY ASK**

Question: What is your strongest technical contribution?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two things, and I would rank the method above the mechanisms. First, the evaluation discipline: a deterministic simulator that runs the real cache and executor, a preregistered rule applied by code, a hash-committed sealed set, and a result we reported even though it was negative. Second, the cache core's explicit invariants: generation fencing, epochs, a hard staleness cap and byte accounting, each with tests. I would not call either a breakthrough. They are careful engineering and honest reporting.

**If the judge asks further:**

"Show me the strongest evidence." The M6 sealed run: 12,000 cells, 30 sealed seeds, verdict H1 NOT_SUPPORTED, produced by `m6_pipeline::compute_verdict`. It has not been independently reviewed, and I say so.

**Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`, `src/lab/m6_pipeline.rs::compute_verdict`, `docs/release/known_limitations_frozen_code.md` section 1.
:::

::: judge
**JUDGE MAY ASK**

Question: What evidence do you have for your claims? How do you know your results are not an artefact of the simulator?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

I am careful to say what each tier shows. T1 is a simulation that runs the real cache code against a modelled origin, so it tests policy logic and cache semantics, not networking. T2 runs the real gateway binary on loopback with a scripted origin. They are separate pieces of evidence, and I have not run an agreement study between them, so I cannot say the simulator predicts reality. What I can say: the simulation is deterministic and tested for determinism, and the analysis is preregistered and mechanical. The main result is negative, so it is not the kind of result a flattering simulator would produce, but that does not show the simulator is right either.

**If the judge asks further:**

"Could the simulator hide a real benefit?" Yes. For example, the dynamic stale window never bound at the registered horizon, so that mechanism was not tested at all. And the scripted origin in T2 has no overload feedback.

**Evidence to know:** `src/lab/matrix_runner.rs::AgreementStudy` and its synthetic-fixture test; claims ledger C7 UNVERIFIED; `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json` H2 section.
:::

::: judge
**JUDGE MAY ASK**

Question: Where are established products stronger than PhantomCache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Many places. Production maturity and tooling in Redis, Memcached, Varnish and nginx. Cross-node duplicate suppression and hot-item mirroring in groupcache, where our GET is local-only. Invalidation durability: our cross-node invalidations are best-effort after the first ack, and owed invalidations are lost if the issuing node dies. Adaptive concurrency in Envoy and Netflix use relative signals, ours uses an absolute ceiling and did not beat a static cap. Eviction research with thousands of traces. Per-tenant resource quotas are absent. And we have no head-to-head performance numbers.

**If the judge asks further:**

"What would you fix first?" The invalidation durability gap, then a load-dependent test origin, then a relative-signal controller under a new preregistration.

**Evidence to know:** claims ledger C6 qualifier; `src/dist/router.rs` (constant weight); `docs/research/rt0_prior_art_sweep_2026-10-30.md` T7, T10; Chapter 8 section on limitations.
:::

::: judge
**JUDGE MAY ASK**

Question: How would I reproduce one of your experiments?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Every run folder has a manifest with the source file hashes, git head, build profile, seeds, scenarios and arm configurations. Check out or confirm that source, build in release, and run the same `RunSpec` with the `phantom-lab` command; the cells file should be byte-identical on the same machine. For the sealed M6 run, nobody can regenerate the seeds without the secret salt, but anyone can re-run the analysis on the retained cells and check the verdict. The T2 Scenario B experiment is real time, so a re-run gives a fresh sample that should agree within the intervals, not byte for byte.

**If the judge asks further:**

"Would it be identical on my laptop?" I have not tested cross-platform determinism. The tests ran in one environment.

**Evidence to know:** `src/lab/eval.rs::run_matrix_with_progress` (manifest fields), `docs/release/reproduction_guide.md` (exists; not read for this chapter), `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes`.
:::

::: judge
**JUDGE MAY ASK**

Question: What is the difference between your simulation and a real test?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In T1 the clock is virtual, the origin and the clients are models, and the real cache and policy code run in-process as a library. Nothing touches a network or the HTTP stack. Results are exactly repeatable. In T2 the real gateway binary runs as a child process, the load generator uses real sockets and real time, and the origin is a small scripted server. It is real but only on loopback on one machine with shared CPU, so no network latency, and the origin has no queue. Neither is a production or multi-host measurement. The interface labels them differently: Simulated versus Real with the qualifier "loopback, one host".

**If the judge asks further:**

"Why is T2 not enough on its own?" Real-time runs are not repeatable, cost minutes each, and cover one scenario. T1 covered 12,000 cells in about 97 seconds of test time.

**Evidence to know:** `src/lab/t1_sim.rs` header, `src/lab_live/mod.rs`, `ui/src/lib/labProvenance.js`, `docs/demo/scenario_b_interpretation_notice.md`.
:::

::: judge
**JUDGE MAY ASK**

Question: You report that your adaptive controller failed. Why should I trust anything else?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Reporting the failure is the reason to trust the process. The rule that produced it was written before the sealed seeds were used, and the code applied it mechanically. I also list the weaknesses: the amendment that forced the controller to be adaptive was chosen after seeing development results, the stale-window mechanism was never exercised at that horizon, the primary P99 includes quickly refused requests, and nobody independent has reviewed the pipeline. What the failure does not mean is that adaptive control is useless. It means this controller, with an absolute ceiling, shows no benefit in this simulation.

**If the judge asks further:**

"What would make adaptive control win?" A relative-signal controller, a load-dependent origin and workloads where a static setting cannot fit all phases. That is a new hypothesis needing a new protocol.

**Evidence to know:** `preregistration_m6_v2_2_amendment.md` sections 3, 5, 7b; `m6_sealed_results.md`; `docs/research/rt0_prior_art_sweep_2026-10-30.md` T7.
:::

::: judge
**JUDGE MAY ASK**

Question: What would you improve with more time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In order: make invalidations durable so a crashed issuer does not leave stale entries; make GET able to use the owner, or at least cluster-wide single flight, since that is where groupcache is ahead; make the Scenario B origin load-dependent and add a cap-free canary arm; run a real T1-to-T2 agreement study; write a new preregistration for a relative-signal controller; run an overhead benchmark in release on a named host; and get an independent human review of the Lab core and a BCa cross-check against a reference implementation.

**If the judge asks further:**

"What would you not do?" I would not claim the product is faster or more consistent than anything until a fair head-to-head run exists.

**Evidence to know:** `docs/release/known_limitations_frozen_code.md` section 5; claims ledger C3, C6, C7; Chapter 8 roadmap.
:::

::: judge
**JUDGE MAY ASK**

Question: What does "H1 NOT_SUPPORTED" actually say, and what does it not say?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It says that on 30 sealed seeds, 8 origin-degradation scenarios and 4 held-out workload families plus one sealed family, an AIMD controller allowed to range at least 4 times its minimum did not reduce peak origin concurrency in any scenario versus the tuned static baseline, and it was credibly worse in seven. It reduced client P99 in two scenarios, and it broke the hit-ratio and error-rate limits in three. It does not say adaptive caching cannot work, it does not say anything about real networks, and it does not test overhead or the stale-window rule.

**If the judge asks further:**

"What was the tuned static baseline?" BL-12b: a static cap and stale window chosen on tuning seeds. The code comment says it is not claimed to be the best deployable static policy.

**Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`, `docs/research/m6_sealed_results.md` "What this does and does not establish".
:::

::: judge
**JUDGE MAY ASK**

Question: Scenario B shows a 64% drop in origin load for the adaptive arm. Isn't that a win?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No, and we say so. The adaptive arm fell from 32 to about 12 concurrent fetches within two seconds and stayed there, so it behaved as a static cap of about 12. Client success dropped by about 17 percentage points, because refused requests count as failures. The mechanical verdict for both capped arms is trade-off failure. What the experiment does show is that a cap is enforced by the real binary, and that the price is refused requests.

**If the judge asks further:**

"Why did it stay near 12?" The controller compares a window maximum of 30 samples against an absolute 60 ms ceiling; timer overshoot and healthy variation can cross that ceiling, and the multiplicative decrease then pins it at the minimum. That is our reading of the notice and code, not a recorded root-cause analysis.

**Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`, `lab-results/scenario_b/20261030T003932Z/summary.json`, `src/lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`.
:::

---

## 7.10 Discrepancies found while writing this chapter

| # | Document says | Code or retained evidence shows |
|---|---|---|
| 1 | `src/lab/mod.rs` header: T2 helpers "remain in `tests/support/lab`" | `src/lab_live/` now exists and is the T2 implementation |
| 2 | Virtual clock described as the simulation clock | The Lab `VirtualClock` (`src/lab/clock.rs`) is used only in `lab_core_tests.rs`; `run_cell` uses event timestamps (a different `VirtualClock` exists in `src/sim/clock.rs`) |
| 3 | `matrix_runner.rs` return comment "rank_concordance_spearman" | computes the fraction of positional matches, not Spearman |
| 4 | `RunManifestV2` with EV-1 to EV-14 | 3 of 14 checks; hard-coded git hash; test-only |
| 5 | `origin_sim.rs` header "M/M/c" | M6 uses constant service time (M/D/c-like) |
| 6 | `analysis.rs` header "BCa" for the module | the synthetic validator and `evaluate_paired` use a percentile bootstrap |
| 7 | `novelty_snapshot_m6.md` "RT-0 PASS" | its own correction notice, `rt0_prior_art_sweep`, and STATUS: RT-0 OPEN |
| 8 | `claims_ledger.md` section 1 table (C1, C2, C7 HYPOTHESIS) | section 7 reconciliation: C1 NOT SUPPORTED, C2 NOT TESTABLE, C7 UNVERIFIED |
| 9 | `docs/demo/README.md` describes a demo launcher and Showcase/Explorer modes | not the Lab; it appears to be an older demo guide (not investigated further) [UNVERIFIED relevance] |

---

## Key ideas to remember

- T1 is a simulation: virtual time, seeded random streams, real cache code, modelled origin and clients. Same seed and source, same bytes.
- T2 is the real gateway on loopback, one host, real time. T1 and T2 are separate evidence; no agreement study exists.
- Paired-by-seed relative reductions, a BCa interval, a sign-flip test and Holm correction turn cells into a verdict; the M6 verdict rule was fixed before the sealed run and applied by code.
- M6 H1 is NOT_SUPPORTED: the forced-adaptive AIMD controller did not beat the tuned static cap. Scenario B's adaptive arm behaved as a static cap near 12.
- The innovation argument rests on the combination and the method, not on any new mechanism. RT-0 is open; no novelty or superiority claim is made.

## What you should now understand

- Why a simulation can be deterministic and why that is not the same as being right.
- The difference between SIMULATED and REAL (loopback, one host), and why the UI always shows the qualifier.
- How a bootstrap interval and a permutation test differ, and why 5 seeds cannot reach p below 0.05 in an exact sign-flip test.
- What the sealed-seed and freeze mechanism protects against and what it does not.
- What PhantomCache shares with Varnish, nginx, Memcached, groupcache, Caffeine and Envoy, and where each is ahead.

## Knowledge check

1. What does the heap in `events.rs` order by, and what happens when two events have the same time?
2. Why does the T1 loop draw the next arrival immediately instead of waiting for the system?
3. Name three things in T1 that are real product code and three that are modelled.
4. For `r = [0.06, 0.04, 0.05]`, what is the mean relative reduction, and can an exact sign-flip test on 3 seeds reach p below 0.05? Why?
5. What does "adverse" mean in `EndpointResult`, and how does it differ from "not supported"?
6. Why is it wrong to say Scenario B shows the adaptive controller works?
7. What would a real T1-to-T2 agreement study need that the current `AgreementStudy` test lacks?
8. Give two reasons the M6 result does not prove that adaptive caching is useless.
9. Name two capabilities where groupcache, Varnish or Envoy is ahead of PhantomCache, with the evidence class for each statement.
10. Which tier would a result from `phantom-lab run` carry, and which tier would a `phantom-lab scenario-b` result carry?

::: answers
1. By `(time_us, seq_id)`, earliest first; equal times are served in the order scheduled (lower `seq_id` first).
2. It is an open-loop model. A slow system must not receive less load, otherwise latency problems are hidden (coordinated omission).
3. Real: `PartitionedCache`, `begin_fetch_atomic`/`finalize_fetch`, the executor's origin gate and AIMD controller (also stale checks). Modelled: client arrivals, origin service times and failures, retries and timeouts.
4. Mean = 0.05. With n = 3 there are 8 sign patterns; the smallest two-sided p is 2/8 = 0.25, so it can never reach 0.05. (Exact test minimum p is 2/2^n.)
5. Adverse: the BCa upper bound is below 0 (the treatment is credibly worse). Not supported: the rule for a benefit was not met, which also includes "no clear difference".
6. The adaptive arm fell to about 12 and stayed there, so it behaved as a static cap near 12; both capped arms are TRADE_OFF_FAILURE; the experiment cannot separate adaptivity from a lower static cap.
7. Real T2 measurements for the same scenarios, paired with T1 cells, a rank statistic that is actually computed (for example Spearman), and a pre-stated tolerance. The current test uses hand-written synthetic T2 values.
8. Any two of: absolute-ceiling controller only; stale window never bound; synthetic workloads only; one model; no overhead data; no real-process or multi-host data; the constraint `max >= 4*min` was chosen after seeing T and D data.
9. groupcache: cross-node duplicate suppression and hot-item mirroring (README fetched this session, OPENED-VIA-TOOL). Envoy: adaptive concurrency with a relative signal (docs opened via tool in the sweep). Varnish: grace and waiting-list collapsing with production maturity (OPENED-VIA-TOOL in the sweep; maturity is general knowledge).
10. `phantom-lab run`: SIMULATED. `phantom-lab scenario-b`: MEASURED_LOOPBACK (displayed as Real, loopback, one host).
:::

## Codebase exploration tasks

1. **Follow one event.** Start at `src/lab/t1_sim.rs::run_cell`. Find the `Ev::Arrive` branch, then `start_attempt`, then `leader_dispatch` and `on_fetch_done`. Find where `now_ms = t / 1000` is passed to the cache and where `FetchDone` is scheduled. You should learn how a virtual-time event drives real cache calls without sleeping.
2. **Trace a verdict.** Start at `src/lab/m6_pipeline.rs::compute_verdict`. Find the `analyze_endpoints` calls, the F-a to F-e falsifiers and the `not_supported` expression. Open `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json` and match each field to a line of code. You should learn how a rule written in advance becomes a mechanical output.
3. **Compare T1 and T2 schedules.** Start at `src/lab_live/loadgen.rs::schedule`. Find the `MasterRng` streams and `AliasSampler` it reuses from `src/lab/rng.rs`, then find `schedule_is_deterministic_and_seed_sensitive` in `tests/integration/lab_live_scenario_b_tests.rs`. You should learn what is shared between the tiers and why the T2 arrival schedule is repeatable even though T2 results are not.

## Challenge questions

1. **Design a fair test of adaptive control.** Suppose you want to test whether a relative-signal controller (latency divided by a measured minimum) beats a static cap. List the changes needed to the T1 origin model, the arms, the preregistration and the claims you would allow in advance.

   *Model answer.* The T1 origin must have feedback (a queue whose service time grows with concurrency, or a finite connection pool), otherwise a controller has nothing to control. Add the controller as a new arm with its own tuned parameters and an equal tuning budget with `BL-12b` and `BL-6`. Include a no-adaptation control (same arm, mechanism disabled) and a planted-effect sensitivity check. Fix the primary endpoints, `delta_min`, the verdict rule and the analysis before seeing any sealed data; use fresh sealed seeds, since the old ones are spent. Allow in advance only: "in this simulation, with these workloads, the controller does/does not beat the static cap". Do not claim real-world benefit until a T2 run with a load-dependent origin and a T1-to-T2 agreement check have been done.

2. **A judge says: "Your T2 result shows the AIMD arm lowers origin load 64% with the same P99. Why isn't that your headline?"** How do you answer, using the three most important qualifiers?

   *Model answer.* (1) The arm is effectively a static cap near 12: it fell to about 12 within 2 s and stayed there, so the comparison is a low static cap versus no cap. (2) The price is about 17 points of client success, because the refused requests are failures; the mechanical verdict is TRADE_OFF_FAILURE. (3) The P99 is computed over all requests including fast 503s, so it does not show a better experience for served clients. Add that the origin has no load-dependent latency, stale serving was not exercised and the caps were chosen after a pilot. The headline is therefore the opposite: the experiment shows a cap works as a cap and costs availability.
