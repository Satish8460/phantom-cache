# PhantomCache — Initial Research Assessment (A–F)

**Status:** Preliminary. Written 2026-10-28 after a partial read-only inspection. Nothing here has been benchmarked.
**Superseded in part by:** the research package in this directory (`literature_review.md`, `code_findings.md`, `candidate_comparison.md`, `hypothesis_and_experiments.md`, `plan_of_action.md`) as they are completed. Where they disagree, they win.

## 0. Evidence legend

Every statement below carries one of these tags.

| Tag | Meaning |
|---|---|
| **[CODE]** | I read the source and the statement follows directly from it. Still unexecuted, so behavior under load is not established. |
| **[REPRO]** | I ran it in this environment and observed the result. |
| **[HANDOFF]** | Claimed by `docs/handoff/*`. Not independently checked. |
| **[HYP]** | Hypothesis. Needs a benchmark or experiment. |
| **[LIT?]** | Recollection from memory. **Not verified against primary sources.** |
| **[REC]** | My provisional judgment. |

## 1. What was and was not inspected

**Inspected (read in full or in the cited range):**
- `docs/handoff/00_START_HERE.md`, `01_PROJECT_MASTER_CONTEXT.md`, `04_TESTS_BENCHMARKS_AND_EVIDENCE.md`, `05_OPEN_RISKS_AND_NEXT_STEPS.md`, `07_CONVERSATION_AND_DECISION_INDEX.md` (first 120 lines).
- `src/core/cache.rs` (all of it), `src/control/pie.rs`, `src/intel/telemetry.rs`, `src/intel/sketch.rs`, `src/intel/workload.rs`, `src/sim/traffic.rs`.
- `src/api/server.rs`: lines ~740–780 (end of GET handler), ~1566–1716 (`spawn_simulation_loop`). Greps for telemetry and PIE use.
- Reproduced: `cargo test --all-targets -j 1` → 242 passed, 0 failed, 19 targets, debug profile **[REPRO]**.

**Not yet inspected (so no claims are made about them):**
- `src/dist/router.rs`, `node.rs`, `frame.rs`, `pool.rs`, `session.rs`, `anti_entropy.rs`
- `src/security/*`, `src/core/coalesce.rs`, `src/main.rs`
- Handoff docs `02`, `03`, `06`, `08`, `MANIFEST`
- The preserved transcripts, the `ui/` source, and the individual test bodies
- Clippy and whitespace checks were not re-run.

The original foundational ChatGPT conversation is documented as missing **[HANDOFF]**.

## 2. A. Technical assessment

### 2.1 What the system is (per handoff, partly confirmed)
A Rust (Tokio/Axum) HTTP caching reverse proxy with single-flight coalescing, 64 lock-striped shards, eviction policies, JWT RBAC, anti-SSRF egress, mTLS cluster transport with binary framing, rendezvous-hash routing, primary-directed invalidation, and bounded anti-entropy. **[HANDOFF]** Sharding, coalescing, generation fencing and tenant epochs are confirmed in `cache.rs` **[CODE]**. The distributed layer is unverified by me.

### 2.2 Observations about the adaptive layer (all **[CODE]**)
1. `PieController::evaluate_candidate` (`src/control/pie.rs:57-119`) computes `utility = hit_rate*100 - p99*0.1` from telemetry of the *currently active* policy. It never estimates the candidate's performance. Commit is `utility > 80`; the rollback branch is reachable only when `candidate_name == "S3-FIFO" && utility < 50`, and its source comment says "Simulated regression". `expected_utility` is the literal `85.0`.
2. `CountMinSketch` (`src/intel/sketch.rs`) has no references outside its own file (grep over `src/`).
3. Every `RawObservation` constructed in the `server.rs` call sites I saw passes `key_hash = 0`.
4. The only caller of `evaluate_candidate` is `spawn_simulation_loop` (`server.rs:~1691`), which passes the simulated `clock`, not wall time, and only when 1000+ telemetry samples exist.

Interpretation **[REC]**: the PIE is a threshold trigger with a cooldown, not a counterfactual canary. The handoff's description of PIE as monitoring access skew and evaluating policies **[HANDOFF]** is not supported by this code.

### 2.3 Observations about the cache core (all **[CODE]** unless tagged)
- `evict_sieve_from_shard` iterates a `HashMap`, clearing `visited` bits until it finds an unvisited entry. There is no FIFO queue and no hand. `evict_s3fifo_from_shard` has no small/main/ghost queues and picks by `last_accessed_ms`. `evict_lru_from_shard` scans all entries. Each eviction is O(entries in shard) under the shard mutex (default cap 10,000 entries/shard).
- `get_resource` and `begin_fetch_atomic` call `get_effective_epochs`, which locks the single global `tenant_epochs` `Mutex` while the shard mutex is held.
- Capacity is `max_entries_per_shard` (a count). Bodies are `Arc<Vec<u8>>`; I found no byte accounting.
- `CachedResource` contains `String` fields, cloned on every hit.
- Hash: `DefaultHasher` (SipHash) per lookup.
- `TelemetryCollector::record` takes one global `Mutex<VecDeque>` on every request; percentile functions copy and sort the whole buffer under that lock.
- `spawn_simulation_loop` synthesizes throughput, hit rate, latency, db load, node CPU (85.2/28.4) and memory (1 GiB/512 MiB) from constants and `rand`. In the visible range, live counters override *some* fields when a `cache` is configured.

**[HYP]** (not measured): that these produce material tail-latency stalls or throughput ceilings. The ceiling and the eviction stall must be benchmarked before being called problems.

### 2.4 Strengths (**[CODE]** for fencing/epochs; **[HANDOFF]** for the rest)
Generation fencing against stale-fetch resurrection, an RAII guard to prevent waiter deadlock, tenant epochs with an overflow fallback, per-key waiter caps, and a broad test suite (242 passing **[REPRO]**) with security-oriented regression tests.

### 2.5 Limitations
- All network evidence is loopback **[HANDOFF]**. No physical multi-host data.
- "T_stale = 0" is described for the canonical primary only **[HANDOFF]**. The guarantee at other nodes, during partitions, and under replica-broadcast loss was not examined by me.
- The "P50 10 ms / P99 15 ms" figure is from the discrete-event simulation **[HANDOFF]**, not from the real cache path.

## 3. B. Literature (UNVERIFIED RECOLLECTION)

Superseded by `literature_review.md`. The initial recollection, kept for audit, is **[LIT?]**:
SIEVE (NSDI'24), S3-FIFO (SOSP'23), TinyLFU (TOS'17), ARC, LHD, Hyperbolic caching, GDSF, LeCaR, CACHEUS, LRB, GL-Cache, Darwin (SIGCOMM'23), AdaptSize, Cliffhanger, miniature simulations (ATC'17), learning-augmented caching, XFetch (VLDB'15), Facebook Memcache leases (NSDI'13), Meta "Cache Made Consistent", RFC 5861, metastable failures. Any author, year or venue stated here is a recollection until the verified review replaces it.

## 4. C. Candidate contributions (initial four, **[REC]**)

1. Origin-health-aware control (serve-stale, TTL stretching, admission, coalescing limits driven by origin state).
2. Safe policy selection by shadow (ghost-cache) evaluation with a do-no-harm guard.
3. Cost-aware eviction using measured per-key fetch cost, with coalescing.
4. Bounded-staleness invalidation, quantified and model-checked.

The full comparison, including additional candidates, is in `candidate_comparison.md`.

## 5. D. Low-level engineering opportunities (all **[HYP]** until profiled)

Global epoch mutex → atomics or per-shard epochs; scan-based eviction → real queue-based SIEVE/S3-FIFO; telemetry → sharded counters/histograms; hasher choice; byte-based capacity; clone cost of `CachedResource`; a hot-path criterion benchmark with flamegraphs. None of these has been measured.

## 6. E. Provisional recommendation (**[REC]**, to be challenged)

Origin-health-aware control, with shadow evaluation as the safety mechanism and cost-aware eviction as an ablation. This is explicitly provisional: the literature review may show the idea is already covered, in which case the direction changes.

## 7. F. Plan

Superseded by `plan_of_action.md`.

## 8. Remaining inspection work

Distributed layer (`router.rs`, `node.rs`, `frame.rs`, `pool.rs`, `anti_entropy.rs`), `coalesce.rs`, `main.rs`, handoff docs 02/03/06/08, the transcripts, `ui/`, and the historical POA documents under `docs/`. Hot-path benchmarks do not exist yet.
