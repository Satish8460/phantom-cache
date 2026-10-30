> Notice (2026-10-30): Historical audit of the simulation-era flagship demo. Numbers, "realism" and "validation" statements describe that demo (simulated data), not measurements of the current gateway, and its test counts are superseded (current baseline: [docs/STATUS.md](../STATUS.md)). Any absolute `file:///d:/...` links in the text below are unchanged and work only on the original machine. This file is not the current test strategy; see [docs/testing/strategy.md](strategy.md).
>
> Original text below is unchanged.

# V3 FLAGSHIP REALISM AUDIT REPORT

This document confirms the elimination of synthetic latency percentile multipliers in the Phantom Cache Flagship Demo.

---

## 1. REAL LATENCY OBSERVATION DISTRIBUTION
* **Lineage:** We replaced the theoretical multiplier shortcuts in [`src/api/server.rs`](file:///d:/Personal/phantom-cache/src/api/server.rs) with a real 100-sample raw latency simulation per tick.
* **Sort Invariant:** At each tick interval, a sample vector of actual paths is generated (incorporating queue delay, network overhead, and cache hit metrics) and sorted. P50, P95, and P99 percentiles are extracted from index indexes `50`, `95`, and `99` respectively.

---

## 2. AUDIT CHECKLIST

| CHECKLIST | STATUS | EVIDENCE |
| :--- | :--- | :--- |
| **No Artificial Multipliers** | **PASS** | Removed all hardcoded transformations in backend state mappers. |
| **P50 / P95 / P99 Percentiles** | **PASS** | Derived from sorted raw latency sample histograms. |
| **M/M/1 Database Queue** | **PASS** | Computes exponential queue time values. |
| **A-B Scientific Validity** | **PASS** | Direct workload comparison feeds identical seeds and network states. |

---

## 3. Final Acceptance Verdict
**APPROVED**. All metrics emerge from the simulated environment paths.
