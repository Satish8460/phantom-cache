# Information Architecture (M8-04)

**Date:** 2026-10-29. Screens are contracts: a screen exists as a data screen only if the data exists today (verified against `src/api/server.rs` `build_schema_v2_payload`). Everything else is a labelled placeholder driven by capability discovery.

## 1. Navigation and URLs

Hash routing, no router dependency: `#/overview` (default), `#/traffic`, `#/cache`, `#/freshness`, `#/origin`, `#/cluster`, `#/lab`, `#/intelligence`, `#/simulation`. Unknown hashes fall back to the overview with a visible notice. The legacy `?view=sim-demo` still opens the simulation. The gateway is chosen by `?gateway=<http(s) origin>` (query), then `VITE_PHANTOM_GATEWAY`, then `http://localhost:8085`; other schemes are refused with a note.

Navigation is a vertical list at 960 px and wider and a horizontal strip below. Groups: **Live gateway**, **Not yet available**, **Simulated**. Placeholder and simulated entries carry a visually hidden suffix for screen readers.

## 2. Screen matrix

| Screen | Route | Journey | Data source today | Status |
|---|---|---|---|---|
| Overview | `#/overview` | Is the gateway healthy now? | `capabilities`, `metrics.*` (6 key tiles), connection state | Real |
| Traffic & latency | `#/traffic` | Rates, hit ratios, percentiles, trends | `throughput_*`, `hit_ratio_*`, `latency_ms`, counters | Real; latency in ms (microsecond fields `p*_us` exist in the payload but are not shown) |
| Cache & memory | `#/cache` | Bytes against budget, evictions and rejections | `bytes_resident`, `byte_budget`, rejection and eviction counters, `process.rss_bytes` | Real; no entry count exists |
| Freshness | `#/freshness` | Stale serving and revalidation | `stale_served_total`, `stale_cap_violations`, `revalidations.*` | Real; no age or fraction fields exist |
| Origin | `#/origin` | Fetch volume, errors, latency | `metrics.origin.*`, plus any other `origin*` provenance fields listed generically | Real when an origin URL is configured, otherwise explicitly unavailable |
| Cluster | `#/cluster` | Membership, invalidation, recovery | `capabilities.cluster`, basic `nodes[]` | Placeholder; node table only when `cluster = experimental` |
| Lab | `#/lab` | Scenarios, replay, comparisons | none | Placeholder |
| Intelligence | `#/intelligence` | Mode, signals, decisions, guardrails | `capabilities.controller` only | Placeholder |
| Simulation demo | `#/simulation` | Explore the legacy model | `SimDemo.jsx` (model output, `/api/control`) | Simulated, always bannered |

Not built (no data and no placeholder value): key/workload investigation (`hot_keys` is unavailable), configuration and diagnostics, incident and recovery. See `ui_api_requirements.md`.

## 3. Shell

`header` (product name, connection pill, mode, gateway URL, theme toggle), `nav`, `main` (screen), `footer` (one-line schema statement). A visually hidden `role=status` region announces connection-state changes. A banner appears above live screens if the gateway itself reports simulation mode.

## 4. Task flows

**Is it healthy?** Overview: connection pill (live / stale / disconnected) then key signals; any tile that is Unavailable shows its reason; a non-zero stale-cap violation turns critical with text.

**Why is latency up?** Traffic: percentile bars and the p99 trend; open "View as table" for exact received values; Origin: origin latency and errors; Cache: evictions and rejections.

**Is the origin protected?** Origin: in-flight and error counters; extra origin fields (for example concurrency-gate telemetry once the backend emits it) appear in "Other origin fields" automatically.

**What can I not see?** Cluster, Lab, Intelligence: each states whether the capability is off or has no UI contract yet.

## 5. Error and degraded flows

| Situation | UI response |
|---|---|
| No frame yet | Loading block, no values anywhere. |
| Gateway unreachable, never connected | Disconnected block with "Reconnect now"; automatic retry with 1 s to 30 s capped backoff when the browser gives up. |
| Connected, then stream drops | Disconnected banner with age and reconnect; last values flagged stale. |
| Stream open but silent for over 3 s | Stale banner; values flagged stale. |
| Frames not schema 2, or invalid JSON, none ever good | Error block naming the problem. |
| Bad frame after good frames | Degraded banner; last good payload kept. |
| Live frame has a simulated or `unavailable`-with-value field | That value is withheld; Degraded banner with the count. |
| Gateway requires authentication | Cannot connect (EventSource cannot send headers); shows as disconnected. Requirement R-UI-01. |

## 6. Responsive behaviour

Single column below 960 px with a horizontal nav strip; metric grids auto-fill (`minmax(11rem, 1fr)`) and collapse to two columns at 480 px; tables scroll horizontally inside their wrapper. 16 px side gutter. **Not verified in a browser.**
