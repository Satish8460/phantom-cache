# UI Metric Provenance Inventory and Specification

**Date:** 2026-10-28. **Sources inspected personally:** `ui/src/App.jsx` lines 1–260 and 403–455 (rest of the 524 lines only via grep), `src/api/server.rs:163-237` (SSE), `:1566-1716` (simulation loop), earlier reads. **Not inspected:** `App.jsx` 260–402 and 455–524 in full (node panels, charts, footer); `ui/dist`.
Classes: **M** measured from the running system; **D** derived from measured values; **S** simulated; **I** inferred; **U** unavailable (should be shown as unavailable).

> **Authoritative field catalog (2026-10-29):** the schema-2 provenance catalog (source, collection point, unit, window/reset, PIE-safety, representation of unavailable) is `phase_1_execution_package.md` (v3) §6.4.2; UI rules are §6.4.4. Where the tables below differ, v3 wins. The inventory below remains the record of *today's* (pre-Phase-1) behavior.

## 1. SSE payload fields (`/api/events`, 100 ms)
| Field | Source today | Class today | Issue | Target source | Target class |
|---|---|---|---|---|---|
| `metrics.cache_hits`, `cache_misses`, `hit_ratio`, `coalesced_waiters`, `evictions_total`, `capacity_rejections` | `cache.counters` atomics (`server.rs:180-186`) when `state.cache` exists, else 0 | **M** | Cumulative counters; hit ratio is cumulative since start, not windowed; misses include fills and DELETE-unrelated; the UI ignores these fields | keep counters; add windowed rates computed server-side | M / D |
| `metrics.p50/p90/p99_latency_ms` | `TelemetryCollector.calculate_percentiles()` (`:187-188`) over a ring of the last N observations | **M** (but polluted) | Observations include zero-latency "miss" entries from every DELETE (`server.rs:945-953`); recorded latency is handler wall time, 1 ms resolution | per-request latency histogram (WP-02/09), exclude DELETE | M |
| `metrics.p95_latency_ms` | `snap.p95_latency_ms`, which the simulation loop fills with the **p90** when a cache exists (`server.rs:1654-1673`) | **mislabeled M**; **S** when no cache | wrong percentile under the p95 name | compute real p95 | M |
| `metrics.throughput_req_sec` | with cache: `requests_total` counter cast to f64 (`server.rs:1658,1670`) = a **cumulative count**, labeled req/s; without cache: `rand` around 12,000 / 62,000 | mislabeled **M**/**S** | not a rate | server-side rate over a sliding window | D |
| `metrics.backend_load_ratio` | with cache: `1 − cumulative hit ratio` (`:1670`); else `db_load` from constants | **D** (misleading) / **S** | not origin load; no origin measurements exist | origin in-flight/req rate/latency from the origin client (WP-02) | M (after origin metrics exist); otherwise U |
| `metrics.avoided_compute_dollars` | `(clock/3.6e6) × savings_hr` with hard-coded costs (`:1639-1652`) and a **70% baseline hit rate assumption** | **S** | not measured; assumes a counterfactual | remove, or label "model with parameters" with the parameters shown | S (labeled model) |
| `hot_keys[]` | literal `/availability/hotel/123, 9200 req/s, 0.99` (`:227-229`) | **S** (fake constant) | presents a fixed fake as live | heavy-hitter sketch from real keys (needs real `key_hash`/keys, WP-02) | M or U |
| `nodes[].state` | `NodeState` of `VirtualNode`s | **M** of in-process objects; in standalone mode these are demo nodes | not cluster-measured | real peer states once cluster path is trusted | M / U |
| `nodes[].cpu_utilization`, `memory_used_bytes` | literals 28.4/85.2 %, 512 MiB/1 GiB (`server.rs:1698-1706`, `main.rs:77-106` [A]) | **S** | fabricated | process RSS/CPU from OS counters; per-node only if reported by the node | M or U |
| `snap.version`, `timestamp_ms` | simulation clock `clock += 100` | **S** clock | not wall time | wall clock + monotonic uptime | M |

## 2. UI-computed values (`App.jsx`)
| Element | Computation | Class | Problem | Target |
|---|---|---|---|---|
| Hits / misses / coalesced / backend QPS (`:53-60`) | `throughput × hitRate`, with `hitRate = 0.94/0.88/0.70` and `coalescedRatio = 0.85/0.40/0` chosen by **preset and toggle** | **S** | ignores real counters; shows invented numbers | read server counters |
| "Phantom OFF" comparison (`:93-98`) | multiplies p50×2, p95×2.5, p99×3, db-load×2.5 client-side | **S** (fabricated counterfactual) | implies a measured baseline | a real "no-cache" baseline run in the lab, labeled as an experiment |
| Hit-rate chart (`:167`) | pushes the constant `hitRate` | **S** | not telemetry | real windowed hit ratio |
| Replication traffic chart (`:168`) | constants 12 / 45 "MB/s" | **S** | no replication traffic is measured | U unless cluster metrics exist |
| Cost panel (`:62-66, 433-455`) | constants ($0.15/M, $0.05/node, 70% baseline, break-even 8,000 req/s) | **S** | invented economics | remove or show as a parameterized model with editable inputs |
| PIE DECISION INSPECTOR (`:412-429`) | text selected by `trafficConfig`: "HOT KEY DENSITY DETECTED", "Memory: OK // Canary Duration: 30s", "✓ COMMITTED" | **S** (hard-coded, no PIE data) | shows decisions that never happened; the real PIE has `get_traces()` unused by the UI | render real `DecisionTrace`s or "no adaptive controller active" |
| Narrative panel (`:68-81`) | preset text + synthetic throughput | **S** | | derived from real events |
| Initial state (`:22-47`) | 12,050 req/s, p99 15 ms, 3 nodes, etc. | **S** | shown before data arrives | show "no data" |
| Backend URL (`:87,179`) | hard-coded `http://localhost:8085` | config | cannot connect to another host | configurable |
| Controls (`handlePreset`, `triggerEvent`) → `POST /api/control` | `FLASH_CROWD`, `NODE_FAILURE`, `NORMAL` mutate **simulation state** only (`server.rs:321-328`) | **S** control | the buttons do not generate load or fail real nodes | relabel "simulation controls", or hide in production mode |

## 3. Specification: provenance contract
1. Every SSE field is an object `{ "v": <value>, "src": "measured"|"derived"|"simulated"|"inferred"|"unavailable", "window_ms"?: n, "as_of_ms": n }` (or a parallel `provenance` map; schema chosen in WP-02, versioned `"schema": 2`).
2. A top-level `mode`: `"live"` (real gateway counters only) or `"simulation"`. In `live`, no field may have `src = "simulated"`; synthetic fields are removed or `unavailable`.
3. The UI renders each value with a provenance badge and renders `unavailable` as an explicit empty state, never as zero.
4. No client-side arithmetic that produces a *new metric* (rates, hit ratios, costs); only formatting and charting of server-supplied values. Derived values the UI needs are computed server-side with their window stated.
5. Simulation features (flash-crowd preset, node-failure preset, "Phantom OFF") move to a separate **Lab view** that consumes lab-run outputs and says "SIMULATED"; they are not shown in the live view.
6. UI test: a fixture SSE stream containing one `simulated` field in `live` mode must fail the contract test (WP-03).

## 4. Planned UI capability by stage (see work packages)
- **Now (WP-03):** truthful live view: real counters/rates/latencies with provenance, windowed hit ratio, unavailable states, configurable gateway URL, honest controller panel ("no adaptive controller active" until one exists).
- **After byte accounting / key telemetry (WP-06, WP-02):** occupancy in bytes and entries, real hot keys.
- **After origin metrics (WP-02):** origin request rate/latency/error/in-flight, coalescing savings measured.
- **Request explainer (later):** per-request outcome (hit/miss/coalesced/rejected) from the diagnostics buffer (`intel/diagnostics.rs`) — requires verifying that buffer's content first.
- **Lab view (WP-20):** configure/replay scenarios, baseline comparison, repeated trials with CIs, manifest export.
- **Cluster view (after G-ARCH):** real membership, peer states, invalidation fan-out status and queue depth, reconciliation state. Not before the cluster path is trustworthy.
- **Controller view (if integrated):** current policy, evidence, guard/rollback events, overhead.
Do not add controls that imply backend capabilities that do not exist.
