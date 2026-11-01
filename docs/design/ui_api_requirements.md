# UI API Requirements

**Date:** 2026-10-29. Backend contracts the UI needs before the remaining screens or features can show real data. Nothing here is implemented. IDs are `R-UI-nn` to avoid colliding with the engineering `R-01` series. The UI never fabricates data for any item below: until the contract exists it shows an explicit "unavailable / not enabled / no contract yet" state.

**Observed while writing this (not assumed):** `src/api/server.rs` was being edited concurrently. At the time of reading it emitted `metrics.origin_cap` (R-01) with members `enforcing` (a boolean `v`), `cap`, `in_flight`, `dispatched`, `origin_cap_rejections`, `controller_steps`, or `{src:"unavailable", reason:"intelligence_not_installed"}`. The UI has **no dedicated tile** for these; they appear automatically under "Other origin fields" on the Origin screen and "Other reported fields" on the Overview, with their stated provenance. The UI now accepts boolean `v` as a flag value. Field names and semantics may still change; give the UI team the final catalog (path, unit, window, provenance) so tiles can be added to `ui/src/lib/catalog.js`.

## A. Cross-cutting

| ID | Need | Why / current gap |
|---|---|---|
| R-UI-01 | A way for a browser to authenticate the SSE stream: accept the token as a query parameter or a cookie, or provide a short-lived stream ticket endpoint. | `/api/events` requires a bearer header when `require_auth_for_control` is on (`server.rs` SSE handler); `EventSource` cannot set headers. The UI cannot connect to a protected gateway. |
| R-UI-02 | Fix zero-for-unavailable: `stale_served_total`, `stale_cap_violations`, `revalidations.*`, and the counter tuple, when `state.cache` is `None` should be `{src:"unavailable", reason:"cache_unavailable"}`. Same for `uptime_ms` without telemetry. | BD-01, BD-04 in `ui_audit.md`. The UI cannot distinguish a real 0 from an absent cache. |
| R-UI-03 | `hit_ratio_cumulative` should be unavailable (`insufficient_window`) when there are no hits or misses. | BD-02: currently 1.0, labelled derived. |
| R-UI-04 | In live mode no field may carry simulated provenance anywhere, including `nodes[].state_src`; and extend the contract walker to check every `*_src` key. | BD-03. |
| R-UI-05 | A machine-readable capability list with versions, for example `capabilities.lab`, `capabilities.config_report`, `capabilities.hot_keys`, each with the same vocabulary (`available`/`disabled`/`unavailable`/`experimental`/`active`/`none`), plus a documented unknown-value rule. | Placeholder screens are driven only by `cluster`, `controller`, `origin_metrics` today. |
| R-UI-06 | Documented JSON schema (or TypeScript types) for schema 2 shipped with the repo, including value types (number or boolean) and whether new fields are additive. | The UI parses defensively and lists unknown fields, but cannot validate shape. |
| R-UI-07 | Readiness and liveness semantics: `/health/live` and `/health/ready` appeared via `crate::health::router` at the time of reading; document their JSON, and say whether `ready` reflects the origin and cluster state. The UI does not call them yet. | A "gateway readiness" tile needs a stated contract and provenance. |
| R-UI-08 | Optional: server-side time series (for example `GET /api/v1/metrics/history?window=300s&step=1s`) with provenance per series. | The UI keeps only the last 120 received points since page load, and labels that. |

## B. Per screen

### Traffic & latency
| ID | Need |
|---|---|
| R-UI-09 | Latency histogram buckets (counts per bucket, bucket bounds, unit) so the UI can draw a distribution rather than four percentiles. |
| R-UI-10 | Per-outcome request counts (hit, miss, coalesced, stale-served, error) with a stated window, and error-rate counters by status class. |

### Cache & memory
| ID | Need |
|---|---|
| R-UI-11 | `entries` (resident entry count) and per-shard or per-policy occupancy. |
| R-UI-12 | Eviction policy name and parameters (read-only), with provenance `measured` for configuration. |

### Freshness
| ID | Need |
|---|---|
| R-UI-13 | Age of the oldest and median stale response served, and stale-served fraction over a stated window. |
| R-UI-14 | The configured freshness limits in effect (`default_ttl_ms`, `max_stale_ms`) as measured configuration. |

### Origin
| ID | Need |
|---|---|
| R-UI-15 | Final catalog for origin cap / concurrency gate telemetry (see the observed fields above): path, unit, window, provenance, meaning of `enforcing` and `cap` when uncapped. |
| R-UI-16 | Origin error breakdown by status code class and timeout versus connect failure over a window. |

### Cluster (needs M3 contracts)
| ID | Need |
|---|---|
| R-UI-17 | Real membership: node id, address, state with provenance (`measured` for a node's own report, `inferred` for peer suspicion), last-seen timestamp, version. |
| R-UI-18 | Invalidation status: pending/queued/acknowledged counts, per-peer lag, last reconcile time and result. |
| R-UI-19 | Replication or peer-traffic counters with units. |

### Lab (needs M4 contracts)
| ID | Need |
|---|---|
| R-UI-20 | An endpoint or static-file index listing lab runs: id, scenario manifest, seed, build and hardware identity, status, start/end time. |
| R-UI-21 | Run results as files with repetitions, confidence intervals, baselines and a statement of limitations, each carrying provenance (`measured` in a stated environment, `simulated` for model runs). The UI will refuse results without an environment label. |
| R-UI-22 | A capability key (`capabilities.lab`) so the screen can tell "no lab" from "no runs". |

### Intelligence (needs M5 contracts)
| ID | Need |
|---|---|
| R-UI-23 | Authenticated, paginated `GET /api/v1/intelligence/status` and `/audit` with a published schema. Today both handlers skip authentication and `/audit` returns every retained record. |
| R-UI-24 | For each decision: id, time, mode, signals used (with provenance), candidate policies, evaluation result with uncertainty, guardrails and caps in force, outcome, rollback state, and a human-readable reason. |
| R-UI-25 | Explicit shadow/advisory/active mode with the rule that only `active` changes serving behaviour. |

### Not yet designed (no placeholder screen)
| ID | Need |
|---|---|
| R-UI-26 | Hot keys / workload skew: `hot_keys` is `unavailable: not_implemented`. Needs key identifiers (or hashes), rate, window, and a privacy decision on exposing keys. |
| R-UI-27 | Effective configuration report with secrets redacted, validation findings and build/version identity. |
| R-UI-28 | Guided recovery actions (for example disable intelligence) need authenticated admin endpoints, an audit trail and an owner-approved runbook link; the UI will not offer controls whose backend does not exist. |
