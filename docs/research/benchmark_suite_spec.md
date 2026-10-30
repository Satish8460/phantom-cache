# Benchmark Suite Specification (low-level performance investigation)

> **Precedence (2026-10-29):** for Phase 1, `phase_1_execution_package.md` (v3) §6.3 is authoritative — it fixes the mandatory vs deferred subset, repetition levels (R1 within-process, R2 process-level ≥10 for mandatory benchmarks; ≥30 optional), validity/fitness rules, the time plan, and the public-API mapping. This document remains the broader suite catalog (including benchmarks deferred to the lab).

**Status:** specification only; no benchmark has been run and no target values are set. Extends `baseline_and_profiling_plan.md`. Targets are *derived from the baseline after it exists*; none are invented here.

## 1. Hypotheses and their current status
| ID | Hypothesis | Code status | Performance status |
|---|---|---|---|
| H1 | Global `tenant_epochs` mutex on the request path limits multi-thread hit throughput | **Confirmed in code** (`cache.rs:251-259`, called under the shard lock from `:313-346`, `:494-612`) | **Unverified** |
| H2 | Scan-based eviction (labeled SIEVE/S3-FIFO/LRU) causes latency growing with shard size | **Confirmed** (`cache.rs:348-479`: iterate whole shard map) | **Unverified**; evictions occur only on a miss in a full shard (`cache.rs:558-568`) |
| H3 | Telemetry global lock + full-buffer sort stalls requests | **Confirmed** (`telemetry.rs:47-53, 85-109`; `record` per request `server.rs:765-771`) | **Unverified**; percentile calls come from the 100 ms SSE/sim loops, buffer cap = `TelemetryCollector::new` capacity (value not re-read) |
| H4 | Entry-count capacity allows unbounded bytes | **Confirmed** (no byte accounting in `cache.rs`) | Arithmetic bound only: shards × 10,000 × body cap (2 MiB per [A]) |
| H5 | `key_hash` telemetry is uninformative | **Confirmed** (`0` at all three call sites) | n/a (functional gap) |
| H6 | Dashboard metrics partly synthetic | **Confirmed** (`server.rs:1585-1713`, `main.rs:77-106` [A], `App.jsx:53-66,94-98,167-168,418-426`) | n/a |
| H7 | `CachedResource` clone allocates on each hit (`String` + `Option<String>`) | **Confirmed in struct** (`cache.rs:74-81`; `begin_fetch_atomic` returns `entry.clone()` at `:523`) | **Unverified** |
| H8 | SipHash (`DefaultHasher`) shard selection is a measurable hit-path cost | Code confirmed (`cache.rs:235-241`) | **Unverified** |
| H9 | Scheduler/allocation overhead dominates the hit path | — | **Unverified; profile first** |

## 2. Design rules
1. Release profile; record `rustc -Vv`, `Cargo.lock` hash, git SHA (and a dirty-tree flag), OS/kernel, CPU model, core pinning, `RUSTFLAGS`, seed, full command line in a **run manifest** (`bench-results/<date>/<run-id>/manifest.json`).
2. Semantic-correctness tests live in `tests/`; benchmarks live in `benches/` (and a separate end-to-end harness). Benchmarks assert nothing about behavior except sanity checks that the measured operation did what it claims (e.g., hit rate observed).
3. Platforms are labeled `windows-native`, `wsl2`, `linux-host`; results are never pooled across platforms. Absolute tail latency is only claimed from `linux-host`.
4. Open-loop load generation for end-to-end runs (no coordinated omission); closed-loop only for microbenchmarks, and labeled.
5. ≥30 independent process-level repetitions for any headline number; medians with bootstrap 95% CIs; raw samples archived; warm-up excluded and recorded; outliers reported, never silently dropped.
6. A pilot run fixes the run-to-run variation bound *before* any optimization is evaluated; optimization claims need a CI that excludes the bound.
7. Every benchmark has a stated purpose, the hypothesis it tests, and the decision it informs.

## 3. Suites
### B1 — Single-thread operation latency (hypotheses H7, H8, H9)
- Operations: `begin_fetch_atomic` Hit; Initiate+`finalize_fetch` (fill with a prebuilt response); `delete`; `get_resource` Hit.
- Parameters: key length {16, 64, 256 B}; body {0.1, 4, 64, 1024 KiB}; tenant count {1, 1000}.
- Metrics: ns/op median and p99 (microbench harness), allocations/op (DHAT or counting allocator), instructions only where `linux-host` PMU exists.
- Component-cost isolation (timing one part alone; not an ablation): measure `get_effective_epochs` alone, hasher alone, clone alone.
### B2 — Concurrent throughput and tail (H1, H3)
- Threads {1, 2, 4, 8, 16, 24}; read-only Zipf α ∈ {0.0, 0.8, 1.2}; 90/10 and 99/1 hit/miss mixes via pre-warmed vs cold keys.
- Tenants {1, 100, 10,000}: tests epoch-map contention (with 1 tenant the map is hit but small).
- Metrics: ops/s, per-op latency histogram (HDR) p50/p99/p99.9; scaling efficiency vs 1 thread.
- Telemetry on/off: run with `telemetry: None` vs `Some` using the same handler path via in-process requests (no TCP) to isolate cost.
### B3 — Cold misses and coalescing (correctness-adjacent performance)
- N concurrent requesters {1, 10, 100, 1000} for one key against a mock origin with latency L ∈ {1, 10, 100 ms}; metrics: origin calls (must be 1), waiter latency distribution, memory per waiter, waiter-limit rejections at `max_waiters_per_key` (default 1,000).
### B4 — Eviction latency vs size (H2)
- Shard capacity {100, 1k, 10k, 100k}; policy {Sieve, Lru, S3Fifo as implemented}; workloads: insert-only churn (all-miss), Zipf, scan; metric: latency of the call that triggers eviction (p50/p99/max), eviction count, hit ratio vs the **reference** policies (WP-05).
### B5 — Hot-key / skew / shift / flash crowd
- Zipf α sweep; hot-set shift at t=T (swap 50% of hot keys); flash crowd (10× rate onto 1–10 keys for 10 s). Metrics: hit ratio over time, origin requests/s, p99. Used both as microbenchmark input and as the lab's scenario library.
### B6 — Telemetry designs (H3)
- Compare: current mutex+VecDeque; atomic counters only; sharded per-thread histograms (e.g., fixed-bucket log-linear); metric: record() cost, percentile read cost under concurrent writers, accuracy of percentiles vs exact on the same samples.
### B7 — Memory and allocation (H4, H7)
- Resident bytes vs entries for body sizes; allocations per hit/miss; peak RSS in an end-to-end run with 100k entries; byte-accounting error (after WP-06) = accounted vs measured RSS delta.
### B8 — End-to-end origin degradation and recovery
- Real gateway process + controllable mock origin (latency step, brownout error rate, outage, recovery); open-loop client at fixed rate; metrics: origin concurrency, origin requests/s, client p50/p99, error rate, stale-age distribution (after staleness features exist). This is the experimental substrate for the research evaluation (`hypothesis_and_experiments.md`) and is built once in the lab (WP-15/16).
### B9 — Cold start
- Process start with empty cache under load at rate R: time to 90% of steady-state hit ratio; origin concurrency peak. (Baseline for the metastable scenarios.)

## 4. Baseline comparisons
Current implementation is the baseline for engineering changes. For eviction: reference SIEVE/S3-FIFO/LRU/TinyLFU implementations from WP-05; for telemetry: B6 variants; for research: the baseline set in `hypothesis_and_experiments.md` §3.

## 5. Recording and reporting
`bench-results/` is *not* committed by AG unless approved; a results README per run lists manifest, command, raw-data path, summary table (median, CI, n). Analysis scripts must be deterministic and version-controlled; language (Rust/Node/Python-in-WSL) decided by owners.

## 6. Environment checklist (per run)
Power plan / governor noted; other load stopped; core pinning recorded; thermal throttling check (sample frequency at start and end if available); CPU model note: the i7-14650HX is a hybrid P/E-core part — pin to P-cores and record which.

## 7. Out of scope here
Any target number, any decision to change code before the pilot, physical multi-host network benchmarks (later; RISK-P2-01).
