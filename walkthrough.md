> Notice (2026-10-30): Undated note from the simulation-era "V3 flagship demo". It is historical and not a description of the current gateway. In particular, "gathered and sorted at each 100ms interval" for percentiles describes the old demo path; the serving hit path no longer sorts ([CORR-M2-03 in CORRECTIONS.md](CORRECTIONS.md), `src/intel/telemetry.rs`). Current status: [docs/STATUS.md](docs/STATUS.md).
>
> Original text below is unchanged.

# Walkthrough of Hardening Changes

This walkthrough describes the realism enhancements implemented for the Phantom Cache V3 Flagship Demo.

## 1. Summary of Actions
* **True Percentiles:** Latency observations are gathered and sorted at each 100ms interval to yield genuine P50/P95/P99 latency calculations.
* **Layout Structure Overhaul:** Added the KPI Strip, Topology and Request Flow panels, Observability Sparkline history charts, and the Economics Waterfall cost model breakdown.
* **Combined Event Composition:** Composed simultaneous environments like Flash Crowd and Node failure.
* **Vite Bundle Compilation:** Built the React web UI.
