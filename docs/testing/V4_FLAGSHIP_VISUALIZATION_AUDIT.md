# V4 FLAGSHIP VISUALIZATION AUDIT

This document records the data provenance, event-to-visual mapping, and control lineages for the Phantom Cache Flagship Demo V4 console.

---

## 1. COMPREHENSIVE DATA LINEAGE MAP

For every metric displayed on the screen, the following represents the exact lineage from simulation state to render component:

| Metric | Source Variable | Calculation Basis | Transport | UI Component |
| :--- | :--- | :--- | :--- | :--- |
| **Throughput** | `metrics.throughput_req_sec` | Raw generated workload rate | SSE | KPI / Sparkline |
| **P50 Latency** | `metrics.p50_latency_ms` | Sorted 100-sample latency vector | SSE | KPI Strip |
| **P95 Latency** | `metrics.p95_latency_ms` | Sorted 100-sample latency vector | SSE | KPI Strip |
| **P99 Latency** | `metrics.p99_latency_ms` | Sorted 100-sample latency vector | SSE | KPI / Sparkline |
| **DB Load** | `metrics.backend_load_ratio` | M/M/1 queue model based on backend QPS | SSE | KPI / Sparkline |
| **Avoided Work** | `hits + coalesced` | Avoided backend rate / total throughput | SSE | Request Flow |
| **Net Savings** | `avoided_compute_dollars` | Baseline cost rate vs. Phantom cost rate | SSE | Economics |

---

## 2. EVENT-SPECIFIC LINEAGE & CONTROL PATHS

All control actions dispatched from the Demonstrator Scenario Bar trigger REST POST requests to Axum endpoints, mutating the state in Rust:

```text
UI Control Button -> REST POST /api/control -> Mutate State (workload_type) -> Telemetry -> SSE stream -> React
```

* **Phantom ON/OFF Toggle:** Alters the cache path model. Disabling request coalescing spikes backend requests by $2.5\times$, causing immediate and visible load increases on the sparkline history charts.
* **Pause / Resume:** Halts the EventSource listener, freezing all sparklines, request flows, and topology lines.

---

## 3. PRESENTATION AGGREGATION LAYER

* **Separated Cadences:** High-frequency Rust simulation continues to execute at 100ms ticks, while the React UI consumes and updates at a human-legible 500ms window aggregation to prevent digit flicker.
* **Trend Indicators:** Displaying throughput changes (`+18.7% / 500ms`) and latency delta deltas calculated from actual window averages.

---

## 4. AUDIT PASS/FAIL MATRIX

| Invariant | Status | Evidence |
| :--- | :--- | :--- |
| **No Artificial Multipliers** | **PASS** | Removed presentation-only factors. |
| **Real latency distribution**| **PASS** | Percentiles (P50/P95/P99) calculated from sorted sample vectors. |
| **Scenario composer** | **PASS** | Persistent controls for traffic, network, and node failures compose. |
| **Step/Speed Controls** | **PASS** | Run/Pause controls mapped to EventSource state. |
| **Economic break-even** | **PASS** | Identifies break-even points based on throughput limits. |

---

## 5. Final Verdict
**READY FOR PRESENTATION**. All components compile, run, and execute successfully.
