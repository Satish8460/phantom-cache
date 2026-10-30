# Refined Hypothesis and Falsifiable Experimental Plan

**Status: PROVISIONAL**, pending literature verification. Pre-registration means: thresholds below are fixed *before* any controller exists. Changing them later requires a dated amendment.

## 1. Hypotheses

**H1 (main).** Under partial origin degradation (latency inflation, brownout, error bursts), a cache controller that uses measured origin health to bound origin concurrency, extend stale-serving, and shed misses reduces **peak origin concurrency** and **p99 client latency** relative to the best *tuned static* configuration of strong baselines, at no more than 1 percentage point lower hit ratio, while never exceeding the configured staleness cap.

**H2 (mechanism).** Most of the gain comes from *miss-first shedding* and *stale extension*, not from adaptive eviction. (Ablation.)

**H3 (cost).** Controller overhead < 1% CPU and < 2 µs added p50 on the hit path. (Measured on the real Rust path.)

**H4 (stability).** The controller does not oscillate: no more than N mode flips per degradation episode (N set after the first baseline run; fixed before the controller is evaluated).

## 2. Falsification criteria (any one rejects or narrows H1)
- F-a: A tuned static SWR + AIMD concurrency limiter achieves peak origin concurrency and p99 within **10%** of the controller on ≥ 80% of scenarios.
- F-b: The controller's advantage disappears when origin latency is heterogeneous per key or when degradation is gradual rather than step-like.
- F-c: The controller increases p99 for any scenario by > 5% versus the no-controller baseline (harm).
- F-d: Stale-cap violation in any trial (hard guarantee broken).
- F-e: The effect size's 95% CI includes zero over ≥ 30 seeded trials.

## 3. Baselines (all implemented in the same harness)
1. No origin awareness, SIEVE (correct implementation), single-flight.
2. Same with tuned `stale-while-revalidate` / `stale-if-error` windows.
3. + XFetch probabilistic early refresh.
4. + AIMD / gradient concurrency limiter on origin calls.
5. + circuit breaker.
6. Best static combination of 2–5 found by grid search per scenario (the "oracle-tuned static" baseline — the hardest one).
7. Eviction ablations: LRU, S3-FIFO, TinyLFU (correct implementations) to show eviction is not the lever.
8. **Varnish-style binary health rule**: grace extended when a health probe marks the origin unhealthy (closest shipping feature; keeps us from claiming "first to react to origin health").
9. **Nginx-style** `lock` + `use_stale` + background update, and **Memcache-style** lease with 10 s token interval + stale return.
10. Delayed-hit-aware eviction (MAD-like / RFB-like) as an ablation of the coalescing dimension.
11. Contrast, not baseline: RobinHood (allocation across multiple backends) applies only if the lab models several origins; otherwise discuss in related work.

12. **Farahbakhsh et al. remedies** (arXiv:2606.00942 App. B): randomly tagged never-timeout origin fills; young-request priority at the origin queue; sufficiently large timeout for warm-up.
13. **Envoy-style** adaptive concurrency (gradient) and success-rate admission control in front of the origin; **Breakwater/DAGOR-style** queue-delay admission; **retry budget** (RetryGuard/ARB idea) for the retry-storm scenario.

14. **DynamoDB-style constant-work refresh** (every hit triggers an asynchronous refresh so origin load is independent of hit ratio) — the strongest feedback-free baseline; if it matches the controller within F-a's tolerance, C1 is falsified.
15. **Varnish-style two-level grace** (short when healthy, long when sick) with a health probe, and **XFetch + single-flight** as the industry-default stack.

Metastable scenarios added (motivated by HotOS'21/OSDI'22): cold restart at high load, retry storm (client retries amplifying), sustained slow origin after a transient trigger.

## 4. Scenarios
Origin models (parameterized, seeded): M/M/c queue with capacity step-down; latency inflation ×k; brownout (partial error rate); slow-start recovery; heterogeneous per-key cost.
Workloads: Zipf (α ∈ {0.7, 0.95, 1.2}); flash crowd (hot-key emergence); workload shift; one or more public production traces if licensing permits (to be selected in the literature review); invalidation storms.
Each scenario × 30 seeds minimum.

## 5. Metrics
Primary: peak origin concurrency, origin requests/s integral, client p50/p95/p99/p99.9, error rate, staleness distribution (age of served stale data), hit ratio.
Overhead: hit-path latency (ns), CPU %, memory bytes. Controller: decisions/s, flips/episode.
Never report a metric the harness cannot measure; label each as **measured-real**, **measured-sim**, or **modelled**.

## 6. Method
- Tier 1 (simulation lab): virtual clock, seeded RNG, deterministic replay. Results labeled **simulated**.
- Tier 2 (real Rust implementation, single host): real cache path against a controllable origin process; release build; pinned CPUs; warm-up; ≥ 30 repetitions; report medians with bootstrap 95% CIs; raw data and manifest archived.
- Tier 3 (multi-host, optional for paper): physical or VM testbed for network effects; never mixed with Tier 1 numbers.
- Agreement check: Tier 1 predictions are compared with Tier 2 outcomes; disagreement is reported, not hidden.
- Prerequisites before any Tier 2 claim: telemetry cleanup (F11, F13), correct baseline implementations, hot-path microbenchmarks.

## 7. Analysis
Effect sizes with bootstrap CIs; paired comparison across seeds; sensitivity to controller parameters; ablation table; negative results reported.

## 8. Threats to validity (to be extended)
Simulated origin may not reflect real services; single-machine runs hide network effects; trace availability; baselines under-tuned (mitigated by the oracle-tuned static baseline); implementer bias (mitigated by pre-registered thresholds and an independent review of baseline implementations).
