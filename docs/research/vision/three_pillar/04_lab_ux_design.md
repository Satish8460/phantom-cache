# P4 - Phantom Lab, Demo Center, observability, UX and design system

**Date:** 2026-10-30. **Workstream:** P4 of 4 (three-pillar product direction). **Status of this document:** PROPOSAL. It changes no code, UI, dependency or configuration. Nothing here is approved by the owner. Nothing here was run: no browser, no build, no test, no experiment.
**Repo state read:** commit `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 93 changed/untracked paths in the working tree (so working-tree line numbers can drift; the citations below were checked on 2026-10-30).
**Sibling files:** at the time of writing `01_distributed_systems.md`, `02_intelligence.md` and `03_security_tenancy_resources.md` did not yet exist in `docs/research/vision/three_pillar/` (directory was empty). The event requirements in section 2.9 and 'Requests to other workstreams' (section 9) are therefore my asks, not agreed contracts. The integrator must reconcile them when P1-P3 land.

**Labels used.** FACT (verified by me in a file, with path), INFERENCE (my reasoning from facts), RECOMMENDATION, UNRESOLVED. Status classes: IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, BLOCKED, RETRACTED. Where I only read a prior agent's document I say "reported by" and do not upgrade it to FACT.

---

## 0. Position in ten lines

1. The repository has one honest pipeline today: seeded run -> real gateway child process on loopback with a scripted degrading origin and open-loop load -> persisted `events.jsonl` -> evidence with tiers and a binding notice (Scenario B through the Lab API and `#/lab`). That is the spine.
2. There is no per-request event, no cluster stream, no decision trace, no prediction event and no multi-node topology feed. The only live feed is a 100 ms aggregate snapshot and a lossy ring of 512 diagnostic events.
3. The owner mandate (3+ real gateway nodes, tenants, authenticated peers, invalidation propagation, failure and recovery, intelligence decisions, prediction vs actual, measured comparison) needs a **new observability layer**. I recommend a **new `demo-1` session stream**, not an extension of `lab-1` (section 2.8).
4. The six labels the owner asked for (REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED, UNAVAILABLE) are a superset-by-renaming of the labels in the code today (MEASURED, DERIVED, SIMULATED, INFERRED, UNAVAILABLE, RETAINED_*). I propose a mapping and one extra rule: every REAL value also carries a **topology class**, so a three-process single-host run can never be read as three hosts (section 2.2).
5. `POST /api/control {"event":"NODE_FAILURE"}` is a logical flag in the demo node list, not a failure (`src/api/server.rs:1086-1091`). In the new UI a fault has a `layer` (`process_kill`, `app_proxy`, `os`, `logical_flag`) and a `logical_flag` can only ever render as SIMULATED (section 2.2, rule R3).
6. The flagship is rewritten as ten stages (section 4) that run on three real gateway processes. For every stage I give a truthful interim version that runs on current code, and say what it does not prove.
7. Design resource facts, re-fetched today (section 5.1): `emilkowalski/skills` is MIT, HEAD still `a81dbca07f2217b7103e510c0f13f8d5b017afc9`; the two other repositories carry no licence file. My recommendation for O8-1 is "adopt as a pinned, filtered, read-only reference with LICENSE and SOURCE.md; do not install; do not copy the other two" (section 5.3).
8. O8-2 (design approval) cannot be closed by the agents that built the UI. The existing browser rounds and ~212 UI tests do not establish visual quality or usability. I specify an independent human design review and a usability study (section 5.7).
9. The Simulation demo (`ui/src/SimDemo.jsx`) is a different product inside the same shell, with fabricated-looking cost panels. I propose a two-step consolidation (section 5.6).
10. Everything is dependency-ordered in section 7, with owner decisions in section 8. Nothing is presumed approved and no dependency is installed.

---

## 1. Verified baseline: what Lab / Demo / observability is today

### 1.1 Lab API (`lab-1`)

| # | Statement | Tag | Evidence |
|---|---|---|---|
| 1.1.1 | Schema string `lab-1`; at most 16 concurrent SSE streams (429 `too_many_streams` beyond); heartbeat comment every 15 s | FACT | `src/api/lab_api.rs:50`, `:59`, `:60`, `:2033-2037`, `:2082` |
| 1.1.2 | A run stream is **not** an in-memory bus: each client builds a `Tail` over the run's `events.jsonl` file, resumes from `Last-Event-ID`, and is woken by a `watch` receiver with a 2 s timeout; the stream closes when the token that opened it expires (`t.expired()` checked in the loop) | FACT | `src/api/lab_api.rs:2020-2080` |
| 1.1.3 | Event kinds in the contract: `state`, `progress`, `result`, `done`, `error`; `progress` carries `done/total`; Scenario B progress carries `arm`, `rep`, `phase`, `gateway_pid` | FACT (contract) / reported (field list) | `docs/design/lab_api_contract.md` section 3; `src/api/lab_api.rs:694`; field list from `docs/design/phantom_lab_ui.md` section 5, not re-verified in the runner |
| 1.1.4 | One active run (409 otherwise); runs persisted under `lab-results/runs/<id>/` (`run.json`, `events.jsonl`, artifacts); on restart, running runs become `interrupted` | FACT (contract) | `docs/design/lab_api_contract.md` sections 4-5 |
| 1.1.5 | Real-gateway runs are spawned per arm/repetition as child processes on a free loopback port with a scripted origin; `GatewaySpec` takes one `exe`, one origin, byte budget, `intelligence` mode and an optional origin-cap config. **There is no multi-gateway spec.** | FACT | `src/lab_live/gateway.rs:15-50` |
| 1.1.6 | Load generator: `Workload { seed, n_keys, zipf_alpha, rate_rps, duration_ms, arrival, path_prefix, client_deadline_ms }` and a per-request `ClientRecord` with `send_lag_ms`. No rate-step, hot-set, tenant or client-identity field. | FACT | `src/lab_live/loadgen.rs:20`, `:66`, `:94` |
| 1.1.7 | Authentication: JWT roles (Observer reads, Operator starts/cancels) or an anonymous loopback-only dev mode with Host/Origin guard | FACT (doc) | `docs/design/phantom_lab_ui.md` section 2; `docs/user/lab_api.md` |

### 1.2 Gateway telemetry

| # | Statement | Tag | Evidence |
|---|---|---|---|
| 1.2.1 | `/api/events` is a 100 ms interval snapshot (not an event log), schema v1 (deprecated) or `?schema=2`; no resume, no event ids | FACT | `src/api/server.rs:200`, `:959` |
| 1.2.2 | `/api/v1/diagnostics/events` returns the **last 100** diagnostic events on every call (no cursor) | FACT | `src/api/server.rs:2171` |
| 1.2.3 | Diagnostic ring capacity 512; `push` uses `try_lock` and, if the lock is busy, **drops the event and increments `diagnostic_events_dropped`**; when full, pops the oldest | FACT | `src/core/cache.rs:302`; `src/intel/diagnostics.rs:181-215` |
| 1.2.4 | `DiagnosticEvent` fields: `timestamp_epoch_ms`, `severity`, `event_name`, `key_fp`, `op_id`, `op_gen`, `outcome`, `duration_ms`, `error_msg`. Event names found by my grep of `event_name: "..."`: `CacheHit`, `CacheInvalidated`, `CacheStaleServed`, `OriginFetchFailed`, `OriginFetchSuccess`. V5 additionally reports `OriginFetchNon200`; I did not re-verify it | FACT / reported | `src/intel/diagnostics.rs:15-27` |
| 1.2.5 | No `miss`, `coalescing`, `admission`, `eviction`, `fencing`, `membership` or `peer_ack` event exists; these exist only as aggregate counters (for example `coalesced_waiters`, `stale_writes_prevented`, `evictions_total`, `waiter_limit_rejections`, `capacity_rejections`, `anti_entropy_*`) | FACT | `src/intel/diagnostics.rs:30-60` |
| 1.2.6 | Response headers expose `X-Cache-Status` (HIT/STALE/MISS...), `X-Cache-Error`, `X-Cache-Error-Reason`, `Age`, `Retry-After` | FACT | `src/api/server.rs:167`, `:1282`, `:1498`, `:1556` |
| 1.2.7 | No request id / trace id anywhere in the request path (I grepped `server.rs` and `diagnostics.rs` for `trace_id`, `request_id`; no hits) | FACT (grep coverage: those two files) | |
| 1.2.8 | `/api/control` `FLASH_CROWD` and `NODE_FAILURE` change a workload string and a demo node state held in `ApiState`; they do not generate traffic or kill anything | FACT | `src/api/server.rs:1086-1091` |
| 1.2.9 | `pie: None` in the live `ApiState`; an origin-concurrency-cap controller is selected by `PHANTOM_INTELLIGENCE` (`off/observe/advisory/canary/active`) | FACT | `src/main.rs:433`, `:460`; `src/config.rs:161` |
| 1.2.10 | `/api/v1/intelligence/status` and `/audit` routes exist; unauthenticated per R-UI-23 (reported) | FACT (routes) | `src/api/server.rs:213-214` |

### 1.3 UI

| # | Statement | Tag | Evidence |
|---|---|---|---|
| 1.3.1 | Hash routes via a registry; Lab sub-routes `story, evidence, run, runs`; legacy `?view=sim-demo` honoured | FACT | `ui/src/lib/routes.js:26-35`, `:59-62` |
| 1.3.2 | Lab screens read the Lab API only and "compute no result"; fetch-streaming with an `Authorization` header (not `EventSource`) | FACT (doc + guard test named) | `docs/design/phantom_lab_ui.md` sections 1-2, 8 |
| 1.3.3 | Tokens: type scale 12/13/15/18/22/24 px, 4 px spacing, `--shadow-1/2` defined, `--dur-fast 120ms`, `--dur-base 200ms`, `--ease-out`; **shadow tokens are defined at `tokens.css:39-40` and not applied** (A3 grep count 0; reported) | FACT / reported | `ui/src/styles/tokens.css:6-45` |
| 1.3.4 | The Simulation screen wraps the legacy `SimDemo.jsx` (517 lines): its own `EventSource` and a POST to `/api/control` | FACT | `ui/src/SimDemo.jsx:92`, `:177` |
| 1.3.5 | The built-in Cluster and Intelligence screens are explicit placeholders ("Cluster mode is not enabled ... What this screen needs") | FACT | screenshot `desktop_light__cluster.png` (opened) |
| 1.3.6 | `#/lab` screens: Overview, Evidence (M6 sealed and Scenario B), Start a run, Runs, Run detail with SSE status banner | FACT | `docs/design/phantom_lab_ui.md` section 1 |

### 1.4 Screenshots I opened (8, all under `docs/demo/browser_verification/20261030T045605Z/shots/matrix/`)

`desktop_light__.png` (Overview), `desktop_dark__simulation.png`, `desktop_dark__lab_run.png` (Lab, Start a run), `desktop_light__lab_evidence_scenario_b_20261030T003932Z.png`, `desktop_light__cluster.png`, `laptop_light__traffic.png`, `mobile_dark__lab.png` (390 px Lab Overview), `desktop_dark__lab_evidence_m6_sealed_2026_10_29T234845Z.png`.

What I observed (FACT, from the images; not a design verdict by an independent reviewer):
- The live screens and the Lab share one calm, flat visual language with a provenance badge on each value. Unavailable values say "Unavailable" with a reason (Traffic: "Not enough samples in the window yet"), not zero. This is good and should be kept.
- Overview is a grid of tiles with large empty areas to the right of the Capabilities panel and under the tiles; Cluster and Intelligence are a small dashed box plus a nearly empty page.
- The Lab page stacks Connection, a debug-build banner and an "intelligence is not part of the live serving path" banner above the tabs, so the tab content starts about 650 px down at 1920x1080 (`desktop_dark__lab_run.png`).
- The Scenario B evidence page puts a long monospace notice inside a scroll box with literal `**` markdown markers visible (`...scenario_b_...png`).
- The Simulation screen is a different product: orange monospace on black, 8-10 px labels, a double-border frame, a "NET SAVED/1M $-0.15" tile, a "PIE DECISION INSPECTOR [SIMULATED]" and a "SIMULATED CLUSTER TOPOLOGY MAP", with RUN/PAUSE/0.25x/1x/2x controls that browser round 3 measured at 21 px (report `20261030T055008Z/report.md` N-3).
- M6 effect-size chart has a zero line, diamond markers for adverse rows, a "View as table" disclosure; the axis caption wraps into a narrow column at 1920 px (caption text squeezed beside the bars in the opened image).

I did **not** open tablet-width, forced-colors or reduced-motion shots, and I could not observe hover, press or animation. Round-3 report content is reported by that report, not re-run by me.

### 1.5 What does not exist (consolidated)

Per-request events; per-decision traces; coalescing/admission/eviction/fencing events; peer-send/receive/ack events; a cluster-wide stream; a membership stream; a multi-gateway Lab run kind; tenant-labelled load or tenant quota events; a prediction event or any producer of one; a presenter/audience view; a pairing flow; a compare endpoint; a report builder beyond artifact download (`src/lab/report_gen.rs` exists, contents not verified by me); a session object. Status class for all of these: DESIGNED BUT NOT IMPLEMENTED at best, mostly not even designed. FACT for 1.2.5 and 1.2.7 within stated grep coverage; the rest per V3/V5 (reported).

---

## 2. Observability architecture

### 2.1 Why this is architecture and not a widget

The owner mandate requires that every displayed value be traceable to actual backend state. Today the UI can only show what the gateway aggregates (100 ms snapshot) or what a batch run reports at the end. To show "why did this request miss, who coalesced, which peer acknowledged, what did the controller decide" the backend must **emit** those facts, with identity and causality, under a bounded cost. That is a cross-cutting contract that P1 (distributed), P2 (intelligence) and P3 (security/tenancy) all write into. This document specifies the envelope; they specify payloads.

### 2.2 Tier labels (REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED, UNAVAILABLE)

Mapping to the vocabulary that exists in code today (`ui/src/lib/provenance.js`, `ui/src/lib/labProvenance.js`):

| New label | Meaning (one sentence) | Maps from today | Must carry |
|---|---|---|---|
| **REAL** | Produced by the actual backend code on actual processes during this session | `MEASURED`, `MEASURED_LOOPBACK` | `topology_class`, `build_profile`, `source` (node id or component) |
| **SIMULATED** | Produced by the T1 virtual-time simulator or by a logical flag | `SIMULATED`, `RETAINED_SIMULATED` (only if replayed, see below) | `sim_kind` (`t1_virtual_time` or `logical_flag`) |
| **REPLAYED** | Played back from a recorded event log; no live process is producing it | `RETAINED_*` | `recorded_run_id`, original tier, original time |
| **PREDICTED** | A forecast or model output about the future, frozen at issue time | none (new) | `prediction_id`, `issued_at`, `horizon`, `model_id`, `interval` |
| **DERIVED** | Computed server-side from other labelled values by a stated rule | `DERIVED` | `formula_id`, `inputs[]`, window |
| **UNAVAILABLE** | Not produced; reason stated | `UNAVAILABLE` | `reason_code` |

Two existing labels need a decision rather than silent loss: `INFERRED` (observed by the page, e.g. client-minus-server clock offset, peer-suspected state) and "DECLARED" (an input the presenter or a calendar supplied, not predicted). RECOMMENDATION: keep `INFERRED` as a **basis qualifier** on REAL or DERIVED values (`basis: observed_by_peer | observed_by_page`) and show it as a small suffix on the badge ("REAL - inferred by peers"); keep `DECLARED` as an **input-kind attribute** on `action` and `prediction` events (`input_kind: declared | forecast`), never as a tier, so a declared warm is never visually adjacent to "PREDICTED" without saying which it is. UNRESOLVED: whether the owner wants these two as top-level labels instead (owner decision D-P4-04).

**Rules (R1-R6).** These are the anti-decoration device and each has a test in section 6.
- **R1 Topology class on every REAL value.** `SINGLE_PROCESS` (gateway and loadgen in one process, e.g. T1), `SINGLE_HOST_MULTI_PROCESS` (several gateway processes on one machine), `MULTI_HOST_LAN`. The class is **computed** from node-reported `host_id` fingerprints (hash of machine identity), never typed by a presenter. Three processes with one `host_id` render "3 processes on 1 host" everywhere, including the audience strip. Multi-host is a tier not exercised in the repository today (V3 §2.2, reported: "a real multi-device LAN run has never been performed"); its label may not appear until a run exists.
- **R2 Fault layer on every injected fault.** `process_kill`, `app_proxy` (TCP relay fault, today only in `tests/support/fault_proxy.rs`, reported by V3/00 §5.5), `os` (firewall/driver; needs approval), `logical_flag`.
- **R3 A `logical_flag` fault is SIMULATED.** It may not draw a node as failed in the topology, may not contribute to "nodes failed" counts, and the control is labelled "Set logical flag (does not stop any process)". The old `NODE_FAILURE` button is either removed from the Demo Center or shown under that label.
- **R4 No client-side computation of a metric.** (Existing rule; keep.) A chart moves only because an event or an artifact field arrived.
- **R5 No counterfactual line** ("without Phantom") unless a real baseline arm with the same seed ran and is named in the comparison.
- **R6 Mode strip is non-removable:** `MODE (LIVE | REPLAY | RE-RUN | REHEARSAL) | TOPOLOGY CLASS | BUILD | SEED | STREAM HEALTH | TRACING ON/OFF`.

### 2.3 Event taxonomy

All events are JSON objects with the envelope below. Kinds are grouped by the pillar that owns the **payload**; P4 owns the envelope, the transport and the rendering.

**Envelope (every event).**

| Field | Type | Notes |
|---|---|---|
| `schema` | string | `demo-1` |
| `session_id` | string | the session object (section 3.1) |
| `id` | u64 | assigned by the single **session writer**; gap-free, monotonic per session (same discipline as Lab `RunHandle`) |
| `src` | string | emitting component: `node:<node_id>`, `loadgen:<client_id>`, `origin:<id>`, `runner`, `controller:<node_id>`, `ui-presenter` |
| `src_seq` | u64 | per-source monotonic counter, assigned at the source; lets the writer detect loss per source |
| `t_src_wall_ns` | u64 | source wall clock (UTC ns) |
| `t_src_mono_ns` | u64 | source monotonic clock (not comparable across processes) |
| `t_session_us` | u64 | writer-assigned offset from session start, using the writer's monotonic clock |
| `clock` | object | `{offset_est_ns, offset_err_ns, method}` for that source at that time (section 2.5); absent means "not estimated" |
| `kind` | enum | section 2.3 table |
| `tier` | enum | REAL / SIMULATED / REPLAYED / PREDICTED / DERIVED / UNAVAILABLE |
| `topology_class`, `build_profile` | string | required when `tier = REAL` |
| `node_id` | string? | gateway node, if applicable |
| `tenant_fp`, `key_fp` | string? | privacy tier P1 fingerprints (section 2.7); never raw by default |
| `client_id` | string? | load-generator or paired client identity (a label, not a credential) |
| `trace_id` | string? | one client request and everything it caused |
| `op_id`, `op_gen` | u64? | existing cache operation identity (already in `DiagnosticEvent`) |
| `inv_id` | string? | one invalidation and all its per-peer legs |
| `cause` | string[]? | ids of events this one was caused by (explicit causality; id adjacency is not causality) |
| `scenario_ref` | object | `{scenario_id, stage, arm, rep, phase}` |
| `sampled` | `{rate, rule}`? | present iff the event class is sampled; absent means complete |
| `reason_code` | string? | machine-readable reason; the UI renders the sentence from a listed template, never freehand |

**Kinds.** (Payload owner in brackets. "Exists today" means an equivalent fact exists in the gateway in some form; see section 1.2.)

| Kind | Required payload | Owner | Exists today |
|---|---|---|---|
| `request` | `method`, `status`, `x_cache_status`, `latency_us`, `bytes?`, `sent_at`, `send_lag_ms` (client-side) | P4 (loadgen) / gateway | client side: `ClientRecord` (`loadgen.rs:66`); server side: no |
| `cache_decision` | `outcome` (`hit_fresh`, `hit_stale_served`, `miss`, `bypass`, `rejected`), `entry_age_ms?`, `ttl_remaining_ms?`, `stale_window_ms?`, `stale_cap_ms`, `shard` | core | partial: `CacheHit`, `CacheStaleServed`; `miss` no |
| `origin_fetch` | `outcome` (`success`, `non200`, `failed`, `timeout`), `duration_ms`, `status`, `origin_in_flight_at_start`, `role` (`leader`) | core | yes (`OriginFetchSuccess/Failed`) |
| `coalescing` | `role` (`leader`, `waiter`), `waiters_now`, `waiter_limit`, `result` | core | counter only |
| `admission` | `gate` (`origin_cap`, `waiter_limit`, `capacity`, `byte_budget`, `tenant_quota`), `decision`, `cap`, `in_flight`, `reason` | core / P3 | rejections as headers and counters |
| `eviction` | `reason` (`capacity`, `byte_budget`, `ttl`, `invalidate`), `policy`, `bytes`, `victim_key_fp`, `skipped_active_ops?` | core | counter only (`evictions_total`) |
| `fencing` | `result` (`stored`, `discarded_stale_generation`, `discarded_epoch`), `op_gen`, `current_gen` | core | counters (`stale_writes_prevented`) |
| `invalidation_issue` | `scope` (`key`, `tenant`, `all`), `issuer_node`, `gen_before/after`, `peers_owed[]` | P1 | `CacheInvalidated` locally only |
| `invalidation_send` / `invalidation_recv` / `invalidation_ack` | `inv_id`, `from`, `to`, `leg` (`primary_150ms`, `async_repair`, `anti_entropy`), `lag_ms`, `result` | P1 | none |
| `invalidation_durability` | `inv_id`, `state` (`pending`, `acked`, `repaired`, `lost_with_issuer`), `log` (`none`, `memory`, `durable`) | P1 | none (M3 defect, reported) |
| `node_health` | `state`, `state_src` (`self`, `peer_inferred`), `last_seen_ms`, `rss_bytes`, `cpu_pct`, `host_id`, `process_start`, `build` | P1 + gateway | `nodes[]` in schema 2, demo node CPU/memory reported as literals (reported by `ui_metric_provenance.md`; not re-verified) |
| `membership` | `change` (`join`, `leave`, `suspect`, `active`), `view_id`, `members[]` | P1 | none (membership read from a file at startup; reported V3) |
| `workload_observation` | window stats per tenant and key class: rate, hot-key set (fingerprints), entropy | P2 | none |
| `prediction` | `prediction_id`, `target`, `horizon_ms`, `median`, `intervals[{level,lo,hi}]`, `model_id`, `mode` (`offline_replay`, `shadow`, `active`), `input_kind` (`declared`, `forecast`) | P2 | none |
| `prediction_outcome` | `prediction_id`, `actual`, `within[{level,bool}]`, `abs_err` | P2 / P4 (derived) | none |
| `capacity_estimate` | `resource`, `headroom`, `basis`, `interval`, `method` | P2 / P3 | none |
| `action` | `action_id`, `actor`, `mode` (`advisory`, `canary`, `active`), `change{from,to}`, `guardrails_in_force`, `rollback_state`, `evidence_ref`, `input_kind` | P2 | origin-cap controller steps exist as counters (`controller_steps`, reported by screenshot of Overview) |
| `safety_decision` | `check` (`stale_cap`, `origin_cap`, `byte_budget`, `fence`, `tenant_quota`, `intelligence_veto`), `result` (`allow`, `clamp`, `deny`), `bound`, `requested`, `granted` | P3 / core | counters only |
| `tenant_quota` | `tenant_fp`, `resource`, `used`, `limit`, `result` | P3 | none (no per-tenant quota found; V5 grep, reported) |
| `security_decision` | `check` (`jwt`, `role`, `tenant_resolve`, `forged_tenant_header`, `peer_cert`, `host_origin`, `stream_scope`), `result`, `code` (generic, no secret text) | P3 | partial (auth failures answered generically) |
| `fault_injected` / `fault_cleared` | `layer` (R2), `target`, `spec`, `injector` | P1 / P4 | none (process kill only manual) |
| `scenario_state` | `state` (`armed`, `running`, `paused`, `completed`, `failed`, `cancelled`, `interrupted`, `reset`), `stage`, `phase`, `progress` | P4 | partial: Lab `state` / `progress` |
| `marker` | `text`, `author_role` | P4 | none |
| `provenance` | `config_hash`, `build{commit,dirty,profile}`, `host_ids`, `seed`, `topology`, `tool_versions` | P4 | `manifest.json` per run (reported) |
| `gap` | `from_id`, `to_id`, `src`, `reason` (`dropped_backpressure`, `sampled`, `source_restart`, `writer_cap`) | P4 | none |
| `aggregate` | counters, gauges, histogram buckets, per-tenant slices, window | gateway | `/api/events?schema=2` |

### 2.4 Data classes, sampling, retention

Three classes, same as V5 section 4.4 with three changes: (a) tenant slices are explicit; (b) histograms are included (needed for latency **distributions**, R-UI-09 open); (c) retention is stated per class in counts and bytes.

| Class | Contents | Delivery | Retention (proposal; numbers are placeholders to be set by measurement) |
|---|---|---|---|
| A "story" | `scenario_state`, `marker`, `fault_*`, `membership`, `node_health` changes, `invalidation_*`, `fencing` discards, `action`, `prediction`, `prediction_outcome`, `safety_decision` (deny/clamp), `security_decision` (deny), `admission` rejects, `provenance` | lossless; if its buffer fills, the **session is marked DEGRADED** and says so | whole session, persisted, hashed into evidence |
| B "aggregates" | `aggregate` per 100 ms (as `/api/events` today) and per-second per-tenant | fixed rate | downsampled series kept for whole session |
| C "trace samples" | `request`, `cache_decision`, `origin_fetch`, `coalescing`, `eviction` | sampled by stated rule (all non-hit; all events of a pinned key or trace; N% of hits) | bounded in-memory ring per node; sampled set persisted; full capture only with `record_raw` on |

Sampling must be **deterministic by hash** of `trace_id` or `key_fp` with the rate stated in `sampled`, so that all events for a sampled trace are present together (a half-traced request is worse than none). UNRESOLVED: a sampler that makes this decision with no lock on the hit path. CLAUDE.md section 5 requires read-path zero contention and the existing diagnostics ring takes a `Mutex` (`try_lock`) on push (`diagnostics.rs:205`). Any new per-request emission touches an owner-reserved invariant and application code; it needs explicit approval and a benchmark under the project evidence rules (CLAUDE.md sections 2, 4). I make no claim that the cost is small.

### 2.5 Ordering and clocks across nodes

FACT: each gateway uses its own wall clock and nothing synchronises them (V3 §5.1, reported; 00 §5.3 reports token `exp` uses wall clock with 10 s leeway). The UI already tracks client-minus-server offset per gateway and labels it INFERRED (`App.jsx` / ConnectionPanel; V3 cites `App.jsx:56-61`, not re-verified by me).

RECOMMENDATION:
1. **Order shown = order observed by the session writer.** The UI says so. Causal order is shown only through `cause[]`, `trace_id`, `op_id`, `inv_id`.
2. Per-source `src_seq` is gap-checked by the writer; a missing number yields a `gap` event naming the source.
3. The writer estimates each source's clock offset with periodic round-trip probes (client-side NTP-style: send T0, source returns its wall time T1, receive T2; offset estimate `T1 - (T0+T2)/2`, error bound `(T2-T0)/2`), stored in the envelope `clock` field. On a single machine the error bound is small but not zero (process scheduling), and the UI still prints it.
4. Cross-node durations (invalidation propagation time) are computed from **one source's** timestamps wherever possible (issuer-side: `issue -> ack received`), and when two sources are needed the DERIVED value carries `error_bound = e_a + e_b`.
5. A source whose `t_src_mono_ns` goes backwards raises `clock_regress`; the event is shown, not reordered.
6. A hybrid logical clock is an option if P1 already has one; I do not propose adding it for the UI.

### 2.6 Bounded overhead and measuring it

- **Budget:** owner decision D-P4-07 (maximum acceptable measured overhead on client p99 and throughput). I do not invent a number.
- **Protocol (RECOMMENDATION):** release build; paired A/B of tracing off/on on the same seed; at least 10 interleaved repetitions (the Scenario B VALID rule uses 10); report the paired difference of client p50/p99 and throughput with a 95% interval, the host, OS, workload, seed and limitations; a UI label `TRACING ON: latency includes instrumentation` whenever class C is enabled.
- **Default:** class C **off** in evidence-grade runs; allowed in rehearsal and in trace demos with the label. Matches V5 R4.
- **Report the observer's own loss:** drop counters per source and per class are themselves events, so "0 dropped" is a measured claim.

### 2.7 Privacy tiers and tenant-aware metrics

| Tier | Content | Who may read |
|---|---|---|
| P0 aggregate | counts, gauges, histograms; per-tenant **only** if cardinality-bounded (<= N tenants, N set explicitly) | any Observer/Audience |
| P1 fingerprint | `tenant_fp`, `key_fp` = truncated keyed hash with a **per-session random salt** (so fingerprints are not comparable across sessions and cannot be dictionary-checked without the salt) | Observer |
| P2 raw | raw keys/paths | Operator with explicit per-session opt-in; never in reports by default |

FACT: `DiagnosticEvent.key_fp` already exists. UNRESOLVED: how it is computed (salted or not) - not verified; P3 to state. Tenant slicing of an Observer token (a token that sees tenant `alpha` only) needs a `tenants` scope claim; that is P3's design.

### 2.8 Stream transport, auth, scoped read tokens

**Recommendation: a new `demo-1` session stream; keep `lab-1` unchanged.**

Reasons: (1) `lab-1` pins five event names and fixtures in `ui/test/` (`docs/design/lab_api_contract.md` section 3; `lab.stream.test.mjs`, `lab.view.test.mjs` exist); (2) Lab's run model is "one run, closed batch, one active"; a session spans several runs and several nodes; (3) the session needs its own writer, caps and tickets. Reuse everything mechanical: single-writer gap-free ids, `events.jsonl` persistence, `Last-Event-ID` resume, fetch-streaming with a header, 15 s heartbeat, a stream cap, close on token expiry.

| Need | Reuse | New |
|---|---|---|
| Persistence and resume | `RunHandle` pattern, `Tail` over `events.jsonl` (`lab_api.rs:2020-2080`) | `session` writer merging events from N node taps |
| Auth | Observer/Operator JWT roles; Host/Origin guard in dev mode | `scope` claims: `session_id`, `classes` (A/B/C), `privacy` (P0-P2), `tenants[]`; short TTL; stream closes on expiry |
| Browser streaming | fetch-streaming (option D, `docs/design/sse_auth_options.md`) | view ticket exchanged once for an in-memory header token for audience devices |
| Node taps | `GET /api/v1/diagnostics/events` is polled-last-100 (lossy) | per-node tap with a cursor (`?after=src_seq`) or push to the writer; bounded ring; loss reported |
| Endpoints | `GET /capabilities` | `POST /sessions`, `GET /sessions/{id}`, `GET /sessions/{id}/events`, `POST /sessions/{id}/{pause,resume,reset,markers}`, `POST /sessions/{id}/tickets`, `GET /sessions/{id}/trace/{trace_id}`, `POST /reports`, `GET /compare` |

The existing `/api/events?schema=2` 100 ms snapshot stays as the single-node operator console feed and as the source of class B for each node.

FACT (consolidated 00 §5.4, V3 §5.1; not re-verified): `/api/events` does not check token expiry inside its loop and has no stream cap. The Lab stream does check expiry (`lab_api.rs` `t.expired()`). The demo stream should copy the Lab behaviour.

### 2.9 What P1, P2, P3 must emit (summary; full asks in section 9)

- **P1 distributed:** `membership`, `node_health` (with `host_id`, real RSS/CPU), `invalidation_issue/send/recv/ack/durability` with `inv_id` and per-leg lag, `fault_injected/cleared` for process kill and any proxy fault, `warm_plan` and `warm_result` (which node warmed which key, origin fetches caused) if coordinated warming exists, anti-entropy rounds with keys repaired.
- **P2 intelligence:** `workload_observation`, `prediction` with frozen issue time and calibrated intervals, `prediction_outcome`, `capacity_estimate`, `action` with mode and guardrails, `safety_decision` for every veto/clamp, and a `decision_trace` payload (inputs seen, rule/model id, alternatives considered, why chosen) whose every field is a named event field so the UI can render the sentence from a template.
- **P3 security/tenancy/resources:** `security_decision` for each check, `tenant_quota`/`admission` with `gate=tenant_quota`, resource-use events (`byte_budget`, `origin_cap`, `waiter_limit`, memory/CPU), stream-scope enforcement events, and the token scope model for demo-1.

---

## 3. Phantom Lab and Demo Center product direction

### 3.1 One product, one model

FACT: the V5 object model (Scenario, Session, Evidence) and V3's Demo Session JSON (`phantom.demo-session/1`) already exist as proposals. I adopt them and add: `topology_class` computed from nodes, `fault_layers` used, `tracing` on/off, `report_ids`. A Scenario is data: topology, workload, fault schedule, policy arms, seed, stage plan, capability requirements. A Session is a run of a Scenario with attached views. Evidence is immutable.

**Navigation (extends `ui/src/lib/routes.js`; `#/lab/*` stays addressable):**

```
Demo Center  #/demo
  Scenarios   library: flagship, short demos, each with a "what is real here" strip
  Stage       a running/recorded session: timeline + lenses
     Lens: Topology & Traffic     (nodes, clients, tenants, origin, edges; request rates)
     Lens: Cache & Invalidation   (per-key lifecycle, per-node hit/miss, propagation)
     Lens: Decisions              (workload -> prediction -> action -> safety -> result; trace)
     Lens: Resources & Tenants    (bytes, origin pressure, quotas, per-tenant outcome)
  Compare     two to four sessions/arms, backend-computed
  Evidence    existing #/lab/evidence + report builder + retention policy
  Runs        existing #/lab/runs
Operator console (existing five live screens) stays as-is
```

The seven "playgrounds" (the list was not provided to me; V5 also did not have it) become **scenarios** in the library and **lenses** on the stage, not separate apps. RECOMMENDATION. UNRESOLVED: the owner's actual seven names; I cannot map them.

### 3.2 Views and roles

| | Presenter | Observer | Audience |
|---|---|---|---|
| Purpose | drive and explain | inspect at own pace | read from distance |
| Controls | start/pause/step stage, inject fault (with layer label), mark, reset, re-run, share, end | filters, pin key/trace, export | none |
| Data | identical triples (metric, value, tier) | identical | identical, fewer elements, larger |
| Extra | stage script, "say / do not say", failure checklist, stream health per source | glossary, "why this value" popover | none |
| Always visible | mode strip (R6) | same | same, enlarged |

Invariant (test in section 6): switching view changes chrome and density only; the set of (metric, value, tier) triples for a given session cursor is identical across the three views. A presenter cannot show the audience a number an observer cannot see.

### 3.3 Topology view

- Layers: clients (grouped by tenant) -> gateway nodes -> origin. Peer links from real membership events. Node box: id, host class (R1), state with `state_src`, build, last-seen, cache bytes/entries, hit ratio window, origin in-flight.
- Edges animate **only** when an event occurred, at recorded rate, with a sampling note when sampled; reduced motion replaces animation with counters and a last-event list.
- A node killed by `process_kill` shows as failed **only after** `node_health` or `fault_injected` events say so; a `logical_flag` shows a SIMULATED chip on a node that is still drawn running (R3).
- Single-node/no-cluster: one box and "cluster: disabled"; no fake peers (the old three demo nodes must never be drawn as a cluster).
- Scale limit: <= 8 nodes on stage; text equivalent table (node, state, tier, last seen) beside every diagram.
- Status: DESIGNED BUT NOT IMPLEMENTED. Depends on P1 `membership` + `node_health`.

### 3.4 Request and decision trace panel

Two traces share one panel:
1. **Request trace** (`trace_id`): received -> auth/tenant resolved -> key fingerprint -> lookup outcome -> coalescing role -> admission -> origin fetch -> fenced store -> response (status, `X-Cache-Status`, `Age`). Each step is an event field; a missing step renders "step not observed", never an inferred step.
2. **Decision trace** (`action_id`): inputs seen (`workload_observation`), prediction (if any) with issue time, capacity estimate, candidate actions, guardrails/safety result, action taken, rollback state, outcome event. Rendered from listed sentence templates.

Honest minimum on current code (IMPLEMENTED pieces, PARTIAL): the load generator records each client request with `X-Cache-Status` and `send_lag_ms` (`loadgen.rs:66`), so an **outcome-only** trace card is truthful today; internal steps are labelled UNAVAILABLE.

### 3.5 Prediction-versus-actual view with uncertainty

Contract (extends V5 §3.6): actual drawn from measured aggregates; prediction drawn as median plus stated intervals over the future horizon; prediction **frozen at issue time** (an issue marker, never redrawn); horizon, model id, training-data identity, interval calibration method shown; **observed coverage so far** as DERIVED with N ("80% band contained the actual in 31 of 40 windows"); a residual strip; "prediction withheld" gaps, not interpolation; mode label OFFLINE REPLAY / SHADOW / ACTIVE; `input_kind` (declared vs forecast) always visible. Table alternative and pattern-differentiated bands. Gate per CLAUDE.md section 6b: no panel may display PREDICTED as acting on live traffic until the executor integration exists. Status: DESIGNED BUT NOT IMPLEMENTED; BLOCKED on P2.

### 3.6 Experiment comparison

Backend computes comparisons (UI computes none). A pre-flight compatibility check refuses or labels "NOT A CONTROLLED COMPARISON" when seed, workload, fault schedule, build hash, topology class, host class or tracing state differ. Always shown: N, paired difference, interval, zero line, verdict with equal visual weight for positive, negative, inconclusive. Existing building block: Scenario B `paired_vs_base` (BCa intervals, sign-flip p; reported in `phantom_lab_ui.md` section 5). Comparison arms for the flagship: **STATIC** (fixed TTL, no intelligence), **DECLARED-WARM** (calendar-driven warm, no forecast), **ADAPTIVE** (P2 + P1 + P3 active). Status: PARTIAL (Scenario B only).

### 3.7 Evidence browser, report generation, retention

- Evidence browser: existing `#/lab/evidence` plus session evidence: per session, the hash-chained event log, per-stage bundles, artifacts with SHA-256, provenance event, binding notices verbatim.
- Report: server-assembled from retained evidence ids; per claim: statement, test, N, effect with interval, verdict, tier + topology class, limitations, falsifier, "what this does not show"; negative rows styled like positive; "reproduce" command; regeneration from the same evidence ids is byte-identical.
- Citation guard: copy button appends tier and notice id.
- Retention: explicit caps in counts and bytes (CLAUDE.md section 5); a `gap` or `writer_cap` event at the cap; retained sessions are never deleted by "reset".
- PDF only through browser print (no new dependency).
- Integrity statement: hash chain proves **integrity, not authenticity** until signing exists (consistent with V4 §7, reported).

### 3.8 Reset, replay, re-run (three different words)

| Action | What it does | Label | Honest for which tier |
|---|---|---|---|
| **Replay** | plays back the recorded `events.jsonl` (scrubbable with an index); no process runs | REPLAYED | all |
| **Re-run (same seed)** | starts new processes with the same seed and config hash; same offered-load schedule, **new measurements** | LIVE, with "re-run of <session>" | T2/real: schedule deterministic, outcomes not bit-deterministic (V5 §3.9, reported) |
| **Deterministic replay of a simulation** | T1 virtual time; reproduces `cells.csv` hash | SIMULATED | T1 only |
| **Reset** | writes `scenario_reset`, kills the session's processes, starts a cold state; retained evidence untouched | | |

Never label a re-run as a replay of the same result. Single-machine mode is the tool for deterministic replay and development; physical multi-device mode is for stronger distributed claims (section 4.12).

### 3.9 Pairing UX

Presenter "Share this session" -> short-lived read-only view ticket (code plus link; QR deferred, no encoder in repo per 00 §5.4) bound to the session, class/privacy scope and TTL, revocable, never grants Operator, redeemed over TLS for an in-memory header token. Anonymous dev mode is loopback-only by design (`docs/design/phantom_lab_ui.md` section 2), so cross-device pairing requires JWT mode or the new ticket. Security-sensitive; P3 owns the design, P4 owns the UX. Status: DESIGNED BUT NOT IMPLEMENTED; BLOCKED on owner decision D-P4-06 and P3.

### 3.10 Projection mode

Audience view at 1920x1080 and 4K: body text >= 24 px-equivalent, metric values >= 48 px, <= 4 primary elements per stage, >= 3 px strokes, no hover-only content, no auto-scroll, high-contrast token variant, no flashing > 3 per second, large-type text equivalent of each chart's headline generated from the same fields as the table; fixed mode strip. These numbers are proposals (from V5 §5), not tested.

### 3.11 States grammar (summary; full spec in 5.5)

Loading, empty, partial, error+retry, unauthorised, disabled, stale (with age), disconnected/reconnecting (with gap count), sampled, replayed, rehearsal, simulated, predicted, unavailable. Each has its own wording; "no data" is never drawn as zero.

---

## 4. Flagship demonstration specification: "Launch Day on three nodes"

### 4.0 Design principles for the script

1. One continuous causal story with one seed, ten stages; the audience can always say why the screen changed.
2. Every stage lists: backend mechanism, events, tier labels, status of each ingredient (exists / needs building), a **truthful interim** runnable on current code, live-failure handling, what it proves, what it does not.
3. The honest negative (Scenario B recorded `TRADE_OFF_FAILURE` for both capped arms, notice in `docs/demo/scenario_b_interpretation_notice.md`, reported by V5/00; opened as screenshot text) stays in the show and is read out cost-first.
4. Stages marked BLOCKED or DESIGNED BUT NOT IMPLEMENTED are **not shown as working**; the Stage page shows an UNAVAILABLE panel naming the missing capability and the pillar that owns it.

**Cast.** Nodes: `node-1`, `node-2`, `node-3` (three real gateway processes, authenticated peers). Clients: `client-A` (tenant `alpha`), `client-B` (tenant `beta`), `client-C` (flash-crowd generator, tenant `alpha`), each with its own token. Origin: scripted origin process (real separate process; faults are scripted, labelled "scripted"). Controller: runs inside each node or in the P2 component. Presenter: Control Center.

**Today's constraints (FACT / reported).**
- A Lab run spawns one gateway per arm/rep; there is no multi-gateway run kind (`gateway.rs:15-50`).
- Cluster mode exists only with mTLS; remote peers are address-only descriptors; membership read from a file at startup; no shipped PKI tool (00 §5.3, V3 §1.3, reported; CLAUDE.md section 6).
- GET is served from the local cache only (CLAUDE.md section 6), so "cache behaviour across nodes" means "each node's own cache, with clients routed by URL".
- Invalidation durability is bounded to the issuer process staying alive (M3 defect reproduced 2026-10-30 08:26 per `docs/STATUS.md`).
- The PIE decision engine is not in the serving path (`main.rs:433`); an origin-cap controller selectable by `PHANTOM_INTELLIGENCE` is (`main.rs:460`).

### Stage 0 - Preflight (presenter only)
- **Mechanism:** capability checks (Lab API enabled, gateway binary path and build profile, three free ports, PKI present, origin script present, clock probe to each node, host ids).
- **Events:** `provenance`, `node_health` (initial), clock probes.
- **Tier:** REAL (configuration facts); topology class computed.
- **Exists / needs:** `GET /capabilities` exists (`lab_api.rs`, route contract). Multi-node preflight needs the session service (new).
- **Interim:** existing Connection panel + `capabilities`, plus a manual checklist in the presenter notes.
- **Failure handling:** a debug build brands the whole session EXPLORATORY (rule already in the Lab API, reported). Missing PKI: stage 1 is skipped and the cluster stages show UNAVAILABLE.
- **Proves:** the environment is what the strip says. **Does not prove:** anything about performance.

### Stage 1 - Bring up three authenticated nodes and clients
- **Mechanism:** session service spawns `node-1..3` (child processes, ports, cluster members file, mTLS identities), origin, and issues scoped tokens to `client-A/B/C`.
- **Events:** `membership(join)` x3, `node_health`, `security_decision(peer_cert allow)` per peer handshake, `scenario_state(armed)`.
- **Tier:** REAL; `topology_class = SINGLE_HOST_MULTI_PROCESS` (computed from `host_id`); in multi-device mode `MULTI_HOST_LAN`.
- **Exists / needs:** mTLS cluster path exists (experimental); multi-gateway run kind, PKI generation, per-node taps **need building** (P1 + P3 + P4). Status: DESIGNED BUT NOT IMPLEMENTED.
- **Interim (runs today):** start one gateway with the Lab API (or three by hand with `scripts/`), show the one-node topology with "cluster: disabled" if clustering is not configured. Do not draw three nodes unless three processes report themselves.
- **Failure handling:** a node that fails readiness is shown FAILED with its log tail (Lab already tails gateway logs, `gateway.rs`); the session refuses to start the workload with fewer than 3 nodes unless the presenter selects "degraded demo", which the strip shows.
- **Proves:** peers authenticated to each other (if the handshake event is real); three processes exist. **Not:** that the cluster is production-grade, nor three hosts.

### Stage 2 - Normal load, baseline, cache behaviour across nodes
- **Mechanism:** `client-A` and `client-B` send Zipf traffic to different nodes (client chooses node by URL, FACT per 00 §5.2); each node serves its own cache.
- **Events:** class B aggregates per node and per tenant; class C trace samples (tracing label); `cache_decision`, `origin_fetch`, `coalescing`.
- **Tier:** REAL counters; hit ratio per window DERIVED (server-side; windowed ratios and histograms are **UNAVAILABLE today**, R-UI-03/09/10 open).
- **Exists / needs:** real-gateway Zipf open-loop load exists (`loadgen.rs`); tenant-labelled load, multi-node routing, per-tenant counters **need building**. 
- **Interim:** Scenario B `warmup`/`healthy` phases on one real gateway, counters MEASURED, windowed ratios shown UNAVAILABLE with reason.
- **Failure handling:** loadgen cannot hold the open-loop schedule -> show "offered load not achieved" using `send_lag_ms`; host-contention flag.
- **Proves:** baseline numbers for the later comparison. **Not:** anything about the same key being consistent across nodes (each node has its own copy).

### Stage 3 - Known future demand (declared), coordinated pre-warming
- **Mechanism:** the presenter or a calendar **declares** an upcoming event ("10x on keys K at T+60 s"). P2 turns the declaration into a warm plan; P1 coordinates which node warms which keys; P3 limits warm-induced origin load.
- **Events:** `action(input_kind=declared, mode=...)`, `warm_plan`, `origin_fetch` caused by warming, `safety_decision(origin_cap)`.
- **Tier:** the declaration is REAL (an input); effects REAL; **no PREDICTED** label here because nothing was forecast.
- **Exists / needs:** nothing in serving path today. Status: DESIGNED BUT NOT IMPLEMENTED; BLOCKED on P1 (coordination) and P2 (calendar input). The consolidated report already says to show it as DECLARED (00 §14.1).
- **Interim:** out-of-process script warms keys by ordinary GETs before the surge; labelled DECLARED / EXTERNAL SCRIPT; origin load measured with and without it (two real runs, same seed).
- **Failure handling:** if the warm does not complete before the surge, the report shows "warm incomplete" and the comparison still runs.
- **Proves:** a pre-declared warm reduces origin fetches during the surge (if the paired measurement shows it). **Not:** that the system predicts anything.

### Stage 4 - Unexpected flash crowd; hot-key emergence; detection; capacity; coordinated response; limits
This is the centre of the show and the narrative the owner asked for:
1. *Clients load:* `client-C` steps rate and concentrates on a handful of previously cold keys.
2. *Nodes observe:* `workload_observation` events show the rate and hot-set change.
3. *Intelligence detects:* P2 emits a detection with its inputs and confidence; **PREDICTED** only if a forecast exists; otherwise the label is DERIVED ("detected, not predicted").
4. *Capacity estimated:* `capacity_estimate` for origin headroom and cache bytes.
5. *Distributed layer coordinates warming:* P1 emits which nodes warm which hot keys.
6. *Deterministic engine enforces limits:* `safety_decision` (origin cap, byte budget, stale cap, tenant quota) is always visible, and an intelligence suggestion above a limit appears as `clamp`/`deny`.
7. *Observability records decisions:* `action` + `decision_trace`.
8. *Lab shows prediction, execution, result:* stage 5 view.
- **Tier:** REAL execution events; detection DERIVED or PREDICTED per P2; capacity DERIVED with interval.
- **Exists / needs:** coalescing and origin cap are real (CLAUDE.md section 5); rate-step/hot-set arrival process for the real gateway **does not exist** (`loadgen.rs:20-30`; `W-FC` exists only in the T1 simulator per V5); detection, capacity, coordination, decision traces **need building** (P1/P2). Status: PARTIAL (mechanism) / DESIGNED BUT NOT IMPLEMENTED (scenario).
- **Interim:** scripted burst of concurrent requests for a few cold keys from `client-C`: show N concurrent misses become one origin fetch using the origin's own request log vs client count; "origin requests avoided" DERIVED with the formula displayed; origin cap and shed shown from the real counters. No detection or capacity panel (UNAVAILABLE). The `FLASH_CROWD` button is **not** used.
- **Failure handling:** any panel with no producing event shows UNAVAILABLE with the owning pillar; if the origin script does not slow under surge, the screen says "scripted degradation" (the slowdown is a schedule, not load-dependent; Scenario B notice qualification 4).
- **Proves (interim):** coalescing and cap mechanics on a real gateway. **Does not prove:** adaptivity; the Scenario B notice says adaptive-control merit is not established.

### Stage 5 - Prediction vs actual and comparison vs static policies
- **Mechanism:** the same seed is run three times (STATIC, DECLARED-WARM, ADAPTIVE) in sequence on release binaries; the Compare view shows paired differences in origin requests, client success, p99, hit ratio, bytes, with N, intervals, zero line.
- **Events:** `prediction`, `prediction_outcome` (frozen forecasts, coverage), per-arm `provenance` and aggregates.
- **Tier:** arms REAL; paired differences DERIVED; forecasts PREDICTED.
- **Exists / needs:** paired-arm machinery exists for Scenario B (`paired_vs_base`); generic compare endpoint, compatibility check, forecast events **need building**. Status: PARTIAL.
- **Interim:** Scenario B 10-repetition retained result presented as the comparison vs a static cap, `TRADE_OFF_FAILURE` read out; prediction panel shows "No predictor in the serving path" plus the retained M6 `NOT_SUPPORTED` snapshot labelled REPLAYED.
- **Failure handling:** a result that does not favour the adaptive arm is shown identically weighted and the script contains the sentence for it. A "documented failure" is a valid outcome of this stage.
- **Proves:** measured benefit or failure **for this workload, seed, build and host class**. **Not:** generalisation, production performance, cross-tier "x faster".

### Stage 6 - Invalidation propagation under load, fencing
- **Mechanism:** `client-A` issues DELETE for a hot key while fetches are in flight; generation fencing discards the late response; the issuer fans out to peers (150 ms primary leg, async repair, anti-entropy; CLAUDE.md section 6).
- **Events:** `invalidation_issue/send/recv/ack`, `fencing(discarded_stale_generation)`, `cache_decision`, DERIVED propagation time = issuer `issue -> ack` per peer with error bound.
- **Tier:** REAL; propagation time DERIVED.
- **Exists / needs:** fencing and counters exist; local `CacheInvalidated` event exists; peer legs, `inv_id`, per-key causal chain, per-key slow-fetch injection **need building**. Status: PARTIAL.
- **Interim:** scripted race against one gateway: slow origin for one key (needs verification that the scripted origin supports per-key delay; V5 could not verify), DELETE mid-fetch, check by GET that nothing stale was stored; counters `stale_writes_prevented` before/after. The screen carries the sentence "a 204 on DELETE is not proof the object is gone everywhere" (`docs/user/limitations.md`, reported).
- **Failure handling:** in single-node mode the propagation panel reads "no peers" (UNAVAILABLE).
- **Proves:** fencing prevents overwrite in the scripted race (needs a mutation test that fails when fencing is disabled, project rule). **Not:** cluster-wide delivery bounds unless the three-node run measured them.

### Stage 7 - Node failure and recovery
- **Mechanism:** `fault_injected(layer=process_kill, target=node-2)`; peers mark it Suspect; owed invalidations queue; the node restarts; repair and anti-entropy return it to Active.
- **Events:** `fault_injected`, `node_health`, `membership`, `invalidation_durability`, anti-entropy rounds.
- **Tier:** REAL; the failure is a real process kill.
- **Exists / needs:** killing a process is possible; cluster behaviour exists only in the experimental mTLS path; launcher, node-health stream, membership events **need building**.
- **Interim:** kill one of two gateways started by hand; show that clients of the other are unaffected (reads are local) and that the killed node's clients lose service. Do not claim failover (GET is local-only).
- **Failure handling:** on a platform where process supervision is not supported the stage is skipped with "not supported", never simulated. If the old `NODE_FAILURE` flag is used it is shown SIMULATED (R3).
- **Proves:** what a node loss does and does not affect. **Not:** automatic failover.

### Stage 8 - Issuer failure and durable recovery
- **Mechanism:** kill the node that issued an invalidation before its peers received it; with a durable invalidation log (P1), the restarted issuer or a peer completes delivery; without it, the stale copy persists.
- **Events:** `invalidation_durability(state=lost_with_issuer | repaired)`, `log` field (`none`/`durable`).
- **Exists / needs:** the defect is reproduced (STATUS); a durable log does **not** exist. Status: BLOCKED on P1.
- **Interim:** run the reproduction and **show the defect as a known limit**, labelled "KNOWN LIMITATION - durable log not implemented" (00 §14.1 says the same).
- **Failure handling:** if the demo accidentally shows no loss, report it as not reproduced this time; do not claim a fix.
- **Proves (interim):** the limit exists. After P1: durable recovery under the stated failure model. **Not:** correctness under arbitrary partitions.

### Stage 9 - Security demos
- **Mechanism:** (a) forged `x-tenant-id` ignored in secure mode (`server.rs`; V5 cites `:1212, 1967`, not re-verified); (b) tenant-scoped DELETE leaves other tenants; (c) a stream token scoped to tenant `beta` cannot read `alpha` events (new); (d) a node without a peer certificate is rejected (P3); (e) expired token closes the stream.
- **Events:** `security_decision` allow/deny with codes; no secret text.
- **Tier:** REAL.
- **Exists / needs:** (a)(b) real; (c)-(e) depend on P3. Frame as a demonstration of specific checks, **not** security certification (V4 §9).
- **Failure handling:** a check that cannot run shows UNAVAILABLE with the reason; no canned "denied" panels.

### Stage 10 - Consistency evidence and report
- **Mechanism:** session report assembled from per-stage bundles; a consistency check reads the final state of each node for the invalidated keys and reports agreement/disagreement as DERIVED with timing.
- **Exists / needs:** evidence viewer and artifact hashes exist; report builder, hash-chained session log, verifier need building.
- **Interim:** per-stage artifact bundle with `SHA256SUMS`, stating "integrity only, not authenticity".
- **Failure handling:** an incomplete or skipped stage renders "not run" and the report header says "incomplete scenario".
- **Proves:** what was recorded. **Not:** anything outside the recorded failure model.

### 4.11 Quick status table (flagship)

| Stage | Runnable on current code? | Class of the full version |
|---|---|---|
| 0 Preflight | Partly (capabilities) | DESIGNED BUT NOT IMPLEMENTED (session service) |
| 1 Three authenticated nodes | One node only | DESIGNED BUT NOT IMPLEMENTED |
| 2 Baseline | Yes via Scenario B phases (one node) | PARTIAL |
| 3 Declared demand + warm | Script only | BLOCKED (P1, P2) |
| 4 Flash crowd + intelligence + limits | Mechanism subset via burst script | DESIGNED BUT NOT IMPLEMENTED |
| 5 Prediction vs actual + comparison | Retained Scenario B only | PARTIAL / BLOCKED (P2) |
| 6 Invalidation + fencing | One node, scripted race | PARTIAL |
| 7 Node failure | Manual process kill | DESIGNED BUT NOT IMPLEMENTED |
| 8 Issuer failure, durable recovery | Defect reproduction only | BLOCKED (P1) |
| 9 Security demos | (a)(b) only | PARTIAL (P3) |
| 10 Report | Artifact bundle | PARTIAL |

### 4.12 Single-machine versus multi-device

| | Single machine (dev + deterministic replay) | Physical multi-device (stronger distributed claims) |
|---|---|---|
| Topology class | `SINGLE_HOST_MULTI_PROCESS` | `MULTI_HOST_LAN` (never run in repo to date; reported) |
| Use | development, rehearsal, deterministic replay of T1, repeatable CI-like runs | claims about real network latency, partitions, clock skew, separate failure domains |
| What the UI says | "3 processes on 1 host: shared CPU, memory and kernel; loopback network" in the strip and on each node box | "N hosts" computed from distinct `host_id`s, plus per-host OS/CPU as **reported by the device** |
| Must never happen | showing it as three machines | showing a single-host result as a multi-host one |
| Extra requirements | none | JWT mode, TLS, CORS exact origins, `PHANTOM_ALLOW_LAN_ORIGIN`, clock-skew display, pairing tickets (00 §5.3-5.4, reported) |

A tier label for multi-host does not exist in the code today; adding it is owner decision D-P4-08.

### 4.13 Live-demo failure handling (generic)

| Failure | What the audience sees | Presenter action |
|---|---|---|
| Stream disconnect | `DISCONNECTED - last event #N`, charts freeze with "last known" | one click "Replay recorded stage" (labelled REPLAYED) |
| Node fails to start | node box FAILED with reason | continue in degraded mode (strip shows) or switch to replay |
| Loadgen lag | "offered load not achieved" | continue; recorded in the report |
| Tracing perturbs numbers | "TRACING ON" | switch off for the measured arm |
| Projector device drops | nothing (observer view independent) | re-pair |
| Result unfavourable | shown identically weighted | read the pre-written sentence |

A recorded fallback is always labelled REPLAYED; it is a safety net, never a silent substitute.

---

## 5. Design-system decision package

### 5.1 Facts about the three candidate resources (re-fetched 2026-10-30)

**Method and limit.** I fetched the GitHub API and raw files with `WebFetch`. `WebFetch` passes the page through a small summarising model, so quotations below are **summaries by that tool, not byte-exact text**. I did not clone, install, run `npx skills`, or execute anything. File sizes and the file list are from the GitHub git-tree API response. "UNVERIFIED" below means I did not read the full text myself.

| | `emilkowalski/skills` | `dickwu/apple-design-skill` | `ykswang/apple-hig-skill` |
|---|---|---|---|
| Pinned commit (HEAD of `main`, fetched today) | `a81dbca07f2217b7103e510c0f13f8d5b017afc9`, 2026-10-28T18:25:28Z, message "add remove-animations skill" | `ff4ebddf0488598723a82bfdce1afb04de0b1c44`, 2026-10-28T14:28:09Z, merge of PR #7 "chore: re-pull Apple HIG on October 28 with no guideline changes" | `9dbbb980f522f0e8b36ed456e632a2741cefce1d`, 2026-09-25T12:43:35Z, "chore: ignore local AGENTS.md" |
| Licence file | `LICENSE`, 1,070 bytes, "MIT License", "Copyright (c) 2026 Emil Kowalski" (via raw fetch; text summarised by the tool) | **none** listed in the repository; README "Origin and license" says Apple guideline text belongs to Apple Inc., reproduced from the public HIG for AI-assisted review, project "provided as they are"; no software licence named | **none** listed; README says "HIG content (c) Apple Inc." and that the repo holds condensed notes for use as AI context; no software licence named |
| Size / files | 31 files, 312,269 bytes total, 17 directories (git tree API) | not enumerated by me beyond README layout (SKILL.md, AGENTS.md, `.cursorrules`, `GEMINI.md`, `scripts/pull-hig.mjs`, `references/` with 123 generated HIG pages + `liquid-glass.md`) | not enumerated by me beyond README layout (`skills/apple-hig/SKILL.md`, `references/` per HIG page, `tools/*.py`) |
| Agent-skill count / scope | 16 skills (animation: `emil-design-eng`, `animate`, `animate-expo`, `review-animations`, `improve-animations`, `find-animation-opportunities`, `remove-animations`, `animation-performance`, `animation-vocabulary`; design/platform: `apple-design`, `mobile-native`; engineering: `write-swift`, `pick-ui-library`; workflow: `prototype`, `break-ui`; toasts: `ask-sonner`), plus `performance-cheatsheet.md`, `ui-cheatsheet.md` | design review against Apple HIG, cross-platform term translation (Flutter, RN, Tauri, Electron), code audit patterns | HIG as an agent skill, 173 pages (README claim), condensed notes |
| Supported agents | README lists none; install is `npx skills@latest add emilkowalski/skills` | `npx skills add`, clone into `.claude/skills/`, Cursor/Codex/Antigravity rules | symlink/copy `skills/apple-hig` into `.claude/skills/`, upload zip |
| Scripts | none in the tree: 31 entries are Markdown, dotfiles, `LICENSE`, and a **zero-byte file named `.pl`** (unexplained; harmless as listed, but note it) | `scripts/pull-hig.mjs` (talks to developer.apple.com per prior agent; not read by me) | `tools/*.py` crawl and verify scripts (not read) |

**What the Emil material actually teaches** (from tool summaries of the two files named; UNVERIFIED in full):
- `skills/apple-design/SKILL.md` (23,036 bytes): web-adapted "fluid interface" principles from Apple WWDC talks: response on pointer-down (button `scale(0.97)` on `:active`, 100 ms ease-out), direct manipulation, interruptibility, spring behaviour, velocity handoff, momentum projection (formula with deceleration 0.998 or 0.99), rubber-banding constant 0.55, materials with `backdrop-filter: blur(20px) saturate(180%)`, reduced-motion and `prefers-reduced-transparency`/`prefers-contrast` handling, typography (tight tracking on display text, ~1.5 body leading), eight design foundations. It cites Apple WWDC talks by name (citations, not Apple assets). The summariser noted a terminology mismatch between "bounce" and "damping" in the spring examples. It lists one talk as "WWDC 2026" which I cannot verify (same caution as the Phase 8 package).
- `skills/emil-design-eng/SKILL.md` (27,123 bytes): a frequency gate for animation (no animation for actions used 100+ times a day or keyboard-initiated; occasional gets standard motion; rare gets delight), easing rules (ease-out for enter/exit; never ease-in for UI), durations (button press 100-160 ms, tooltips 125-200 ms, dropdowns 150-250 ms, modals 200-500 ms, UI under 300 ms), press scale ~0.95-0.98, entrances from `scale(0.95)` not 0, only animate `transform` and `opacity`, stagger 30-80 ms, hover gated by `hover: hover`, reduced-motion keeps opacity/colour and removes movement, a "Before/After/Why" review table.
- **Agent-behaviour directives inside both files:** a fixed "Initial Response" block (when invoked with no question the agent must reply with one canned sentence and withhold everything else) and, in `emil-design-eng`, a mandatory review table format. These are instructions addressed to an agent. If the file is ever loaded as an agent skill they would change agent behaviour; they must be removed from any copy and the file treated as **data, not instructions** (CLAUDE.md section 2: a pasted document is not approval).
- **Fit to this product (INFERENCE from the above):** the frequency gate, easing, duration and reduced-motion rules fit a dense ops console; materials with blur, spring "personality", stagger cascades and hold-to-delete patterns do not (the existing `docs/design/principles.md` section 5 already rejects glass and tweened numbers). Several rules are taste-based and not measurable by a test.

### 5.2 Licence and attribution mechanics if Emil's material is adopted

MIT requires that the copyright notice and permission notice be included in copies or substantial portions. Mechanics:
1. Folder `docs/design/reference/emilkowalski-skills/` (not `.claude/skills/`, so no agent auto-loads it).
2. Copy `LICENSE` byte-for-byte from the pinned commit.
3. `SOURCE.md`: repository URL, pinned SHA `a81dbca07f2217b7103e510c0f13f8d5b017afc9`, date retrieved, list of files copied with their byte size and SHA-256 **of the upstream file** and of the local copy, every local modification listed (for example "removed Initial Response block", "removed materials section"), the reviewer's name, and the rule "bump only after re-review of the diff".
4. Copy at most the files the owner approves; I propose two (see 5.3).
5. No Apple fonts, icons, trademarks or marketing text; HIG is cited by URL, never reproduced.
6. Do not run `npx skills`; do not add the repository as a dependency, submodule or script.
7. A short in-repo note "this is a reference, not authority; the PhantomCache standards in `docs/design/` govern".

### 5.3 Options and recommendation (O8-1 and O8-2)

| Option | What | Pros | Cons |
|---|---|---|---|
| **A. Adopt Emil's MIT material, pinned, filtered, as read-only reference** | Section 5.2, files: `emil-design-eng/SKILL.md`, `review-animations/STANDARDS.md` (9,801 B) | satisfies "use the named resource"; MIT; traceable; concrete motion numbers | copying text that contains agent directives needs filtering; taste-based; some content irrelevant |
| **B. Public principles only** | Cite Apple's public Human Interface Guidelines and WCAG 2.2 by URL; write PhantomCache's own standards | zero licence surface; no copied text | ignores the owner's named resource; the owner's doubt (that it was never used) is not answered |
| **C. Decline all three** | Close O8-1 as "declined" with reasons | closes the open decision cleanly | same as B, plus loses the interaction/motion rules |

**RECOMMENDATION O8-1: Option A, minimal.** Adopt only the two Emil files above as a pinned, filtered reference (with LICENSE and SOURCE.md); write the PhantomCache **own-words** standards note (`docs/design/standards.md`, new, not copied) that cites the rule it came from. **Do not copy** `dickwu/apple-design-skill` or `ykswang/apple-hig-skill` (no licence file; embed Apple text; the Apple copyright notice in both READMEs). `dickwu` may be read locally by a reviewer as a prompt for review questions, with nothing copied. Use Apple's public HIG and WCAG 2.2 as principle sources. I did not verify that reading these files would change any design outcome; the value is that the owner's instruction is then actually fulfilled or formally declined. UNRESOLVED: whether the owner's phrase "apple-design" meant the `apple-design` skill file or the repository as a whole; the owner decides.

**RECOMMENDATION O8-2:** do not treat O8-2 as closed by agents. Define it as "owner approves the design system AND an independent human design review has reported (section 5.7) with no open severity-1 finding". Until then, `docs/design/principles.md` line 3 ("pending owner approval O8-2") stays true.

### 5.4 What the PhantomCache design system must contain

Starting point FACT: tokens exist for type, space, radius, shadow (unused), motion, targets, colour (light and dark), focus; ~11 components exist. Target (RECOMMENDATION):

1. **Type tiers.** `display` (audience stage, >= 48 px metric), `metric` (24-32), `title` (22), `section` (18, currently unused), `body` (15), `label` (13), `caption` (12 minimum), `mono` (ids, hashes). Tabular numerals on all figures. System font stack (no bundled or Apple fonts). A real step between page title and panel title (today 22 vs 15, A3 finding).
2. **Elevation scale** applied: `flat` (page), `raised` (panels, uses `--shadow-1`), `overlay` (menus, dialogs, tooltips, uses `--shadow-2`), plus a border-only variant for high-contrast/forced-colors. Nested panel-in-panel stacking is banned (use a divider, not a second bordered box).
3. **Motion standards.** From the frequency gate (Emil `emil-design-eng`, reviewed) and the existing principle "live values never animate": no animation on keyboard-initiated actions or live data ticks; 100-160 ms press feedback on discrete controls only; 150-250 ms for menus/popovers; ease-out for enter/exit; reduced-motion = instant or opacity only; event-driven topology motion plays recorded events at recorded rate or is replaced by counters; nothing flashes more than 3 per second.
4. **State grammar** (one visual language, a gallery of each state on each tier): loading (static skeleton that cannot be mistaken for data, with the word "Loading"), empty ("no events yet - not a failure"), partial, error+retry, unauthorised, disabled (reason), success (quiet), stale (age), disconnected (gap count), **simulated**, **replayed**, **predicted**, **unavailable** (reason), sampled, rehearsal.
5. **Form patterns.** Grouped fieldsets with select-all and a selected-summary; errors appear after interaction or submit, not on a pristine form (A3 observed an error shown immediately on the Start form); sticky submit with a live "exact request" preview (the page already has an Exact request panel); progressive disclosure for advanced fields; segmented controls for exclusive choices.
6. **Provenance badges.** Six tiers (section 2.2) with icon shape + word + a shared left "tier rail" on panels, reducing the current reliance on four border styles (solid, dashed, double, dotted) that A3 found noisy; topology class and basis shown as secondary text on the badge. Badge appears on every value or the block that holds it.
7. **Chart primitives**, each with a table alternative and a text summary, colours from a validated categorical palette of at most six hues (the `dataviz` skill's validator exists in this environment but has never been used on this project; any palette must be re-validated in both themes): time series with gap rendering (never interpolate across a `gap`), latency distribution (histogram/percentile ladder), count timeline/swimlane for events, prediction band with frozen issue marker, effect size with zero line (exists), key heat strip, topology graph, per-tenant small multiples, sparkline.
8. **Theme control.** Segmented System / Light / Dark with persisted choice when storage is available; high-contrast variant for projection; dark block de-duplicated (tokens.css repeats it twice, A3).
9. **Layout.** Lab promoted out of the "Not yet available" nav grouping logic; connection and banners collapsed to a status strip with expandable detail so the primary task starts above the fold; no large dead areas (cards fill or the page uses a two-column rule).
10. **Content.** A notice renderer for Markdown (no literal `**` marks); sentences generated from templates for decisions.

Status of each: IMPLEMENTED partially (tokens, badges, tables), UNVERIFIED (visual quality), DESIGNED BUT NOT IMPLEMENTED (items 2, 3 interaction layer, 5, 7 mostly, 8 segmented).

### 5.5 Dependencies for the design system

Charts: the project rule is hand-rolled SVG/CSS (`docs/design/principles.md` section 5; O8-3 owns any chart library, visual-regression tool, accessibility tester or font). I propose none. If the owner later approves a dev-dependency for visual regression or accessibility testing, CLAUDE.md section 2 requires a justification per dependency (purpose, alternatives, maintenance, security, dev-only); I do not provide one here because I propose none.

### 5.6 Simulation demo consolidation

Facts: `ui/src/SimDemo.jsx` (517 lines) has its own `EventSource` and a POST to `/api/control` (`:92`, `:177`); screenshot shows 8-10 px text, a double-border frame, tiles for "NET SAVED/1M $", "COALESCING 40%", a "PIE DECISION INSPECTOR [SIMULATED]" and a "SIMULATED CLUSTER TOPOLOGY MAP"; round 3 measured five buttons at 21 px (below the project's 24 px floor); the prior (reported) provenance record says its "Phantom ON/OFF" and cost panels were fabricated models.

Plan:
- **Step 1 (cheap, interim):** re-home it under the shared tokens and type scale (>= 12 px, 32/44 px targets, shared `.btn`), keep the SIMULATED banner, relabel `KILL NODE` as "Set logical flag (no process stopped)", remove the dollar/"Phantom OFF" counterfactual tiles unless parameters are visible and the tile is labelled MODEL, and make the topology map say "illustrative; not the cluster". No new data.
- **Step 2 (target):** delete the legacy screen. The same content becomes the **Simulation lens** of the Demo Center: scenarios run by the T1 simulator (`src/lab`, deterministic virtual time, `cells.csv` hash) appear on the same stage with tier SIMULATED and the same views, events and mode strip. T1 does not currently emit per-event `demo-1` events (it writes `requests.csv`/`fetches.csv` when `record_raw`, reported by V5); a small exporter is needed.
- **Acceptance:** one type scale and one component set across live, Lab and Simulation; a source guard test that fails on font sizes under 12 px and on hex colours in any UI file; no tile without a tier badge; no client-side counterfactual.
- **Risk:** the old controls drive a legitimate teaching view for some audiences; owner may prefer keeping it as an explicitly "toy" page. That is acceptable if it is visibly separate (owner decision D-P4-10).

### 5.7 Independent human design review and usability study

**Why:** FACT: ~212 UI tests are static CSS/source scans and server-render DOM checks (`lab.render.test.mjs` says visual quality is not tested here, A3 quote; test count reported, not re-run). Three headless-browser rounds measured overflow, target size, focus, contrast ratios and keyboard paths; their own reports say PASS means "no failure observed" (`20261030T055008Z/report.md` header). Neither establishes hierarchy, aesthetics, comprehension or usability. No human review or usability test exists (A3, reported).

**Reviewers (not the authoring agents, and not given the agents' self-assessment first):**
- R1: one product/visual designer with data-dense or operations UI experience.
- R2: one accessibility specialist (WCAG 2.2 AA) using at least a keyboard and one screen reader.
- R3: one engineer who did not build the UI, as a task walkthrough observer.
- The owner decides who (decision D-P4-09). I name no individuals.

**Materials:** a build of the UI at a stated commit; fixtures-driven prototypes always bannered "PROTOTYPE - FIXTURE DATA" (per Phase 8 M8-06) for screens whose backend does not exist; the section 5.4 state gallery.

**Screens (minimum 16):** Overview, Traffic, Origin, Phantom Lab landing, Start a run, Run detail (live, stale, disconnected), Evidence Scenario B, Evidence M6, Demo Center Scenarios, Stage presenter, Stage audience, Topology (3 nodes, 1 failed), Decision trace, Prediction-vs-actual, Compare, Report; plus the State gallery and the consolidated Simulation lens.

**Matrix:** light and dark; 390, 820 and 1280 px (plus 1920 for audience view); keyboard-only pass; 200% and 400% zoom; forced-colors and reduced-motion.

**Review protocol:** (1) each reviewer completes an independent written rubric before discussion (hierarchy, spacing, density, colour, typography, state clarity, provenance legibility, consistency, motion, accessibility); (2) severity scale S1 blocker / S2 major / S3 minor; (3) findings logged with screen, width, theme, screenshot, and the recommended change; (4) the authoring team responds in writing; (5) a re-review of every S1/S2 fix; (6) report stored under `docs/design/review/<date>/` with reviewer-written text unedited.

**Pass criteria (proposals, owner to confirm):** no open S1; no more than an owner-set number of S2; every screen reviewed in both themes at all three widths; R2 reports no WCAG 2.2 AA failure that blocks a task; every tier badge judged legible by all reviewers.

**Usability study outline (formative, not statistical):**
- 5-8 participants: at least 3 engineers who have not seen the project, at least 2 non-technical audience members; recruitment and consent per owner (O8-4 participants is owner-reserved).
- Tasks: T1 "is this number live, replayed or simulated?" on four screens; T2 find why a given request missed; T3 find how long invalidation took to reach node-3 and say how sure the screen is; T4 identify the cost of the cap in Scenario B; T5 start a run and explain what will be sent; T6 recognise a failed node vs a logical flag; T7 in the audience view read the headline reading; T8 recover from a disconnected stream.
- Measures: task success, errors, time, think-aloud notes, a standard questionnaire (SUS or equivalent) as an indicator only. With N this small, report counts, not percentages with confidence intervals.
- Proposed pass targets (placeholders): T1 and T6 correct for all but at most one participant; no participant misreads SIMULATED as live on the Simulation lens.
- **What it cannot show:** statistical usability, long-session fatigue, real incident conditions.

---

## 6. Per-proposal specification

Format: why necessary; builds on; what must change; depends on; how tested; completion evidence; what could make it fail. All status classes below are for the *proposal*; nothing is implemented unless a row says so.

### Proposal A. `demo-1` session stream and writer
- **Why:** nothing today streams per-event data across nodes with resume; the owner mandate requires it.
- **Builds on:** Lab `RunHandle`, `events.jsonl`, `Tail`, `Last-Event-ID`, fetch-streaming, stream cap (`lab_api.rs:2020-2080`).
- **Must change:** new session module (not `lab-1`); node taps; scoped read tokens.
- **Depends on:** owner decision D-P4-02 (new stream vs extend); P3 token scopes; P1/P2/gateway taps.
- **Tested by:** property test (ids strictly increasing and gap-free); forced overflow yields exactly one `gap` event for the dropped range; reconnect with `Last-Event-ID` produces zero duplicates; fixtures recorded from a real session; source-guard tests copied from `lab.guard`.
- **Completion evidence:** recorded three-node session whose `events.jsonl` replays byte-identically through the UI; per-source `src_seq` continuity report.
- **Could fail:** writer becomes a bottleneck or single point of failure for the demo; clock probes mislead; stream cap (16) is hit by projector plus observers.

### Proposal B. Tier registry and rules R1-R6
- **Why:** the owner requires six labels and forbids logical flags presented as failures.
- **Builds on:** `ui/src/lib/provenance.js`, `labProvenance.js`, `ProvenanceBadge.jsx`.
- **Must change:** vocabulary mapping (2.2), topology class computation from `host_id`, fault layer field, strip component.
- **Depends on:** node `host_id` in `node_health` (P1); owner decisions D-P4-04, D-P4-08.
- **Tested by:** fixture test that a `logical_flag` event never changes a node to failed; test that three nodes with one `host_id` render "1 host"; DOM test that every value has a badge or is inside a badged block (existing style); test that no UI file computes a metric (`forbidden` scan extension).
- **Completion evidence:** DOM test output plus screenshots reviewed in the human review.
- **Could fail:** `host_id` is spoofable (reported value, not verified identity); the label then says "reported".

### Proposal C. Mode strip and the REPLAY / RE-RUN / REHEARSAL vocabulary
- **Why:** prevents a recorded result being read as live and a re-run being read as the same measurement.
- **Builds on:** Lab run states, `Last-Event-ID` replay.
- **Must change:** session `mode`; strip component; UI wording.
- **Depends on:** Proposal A.
- **Tested by:** snapshot test of the strip in each mode; test that a REPLAYED session cannot display a `LIVE` chip; usability task T1.
- **Completion evidence:** usability study T1 results.
- **Could fail:** presenters ignore the strip (a human problem the study can surface but not remove).

### Proposal D. Topology lens
- **Why:** the owner mandate names topology, node health, propagation.
- **Builds on:** `Placeholder.jsx` node list; `nodes[]` in schema 2.
- **Must change:** real membership/health events; SVG graph with table alternative.
- **Depends on:** P1 `membership`, `node_health`, `invalidation_*`.
- **Tested by:** fixture renders for 1, 3, 8 nodes; failed-node vs logical-flag test (B); reduced-motion test; a11y lint.
- **Completion evidence:** recorded three-process session, node kill visible, reviewed by R1/R2.
- **Could fail:** more than 8 nodes; event rate overloads the browser; sampled edges misread as full.

### Proposal E. Request/decision trace panel
- **Why:** "show why" is the core credibility feature.
- **Builds on:** `ClientRecord` and `X-Cache-Status` (outcome-only trace today); `DiagnosticEvent` fields.
- **Must change:** `trace_id` through the request path; template list; sampling.
- **Depends on:** core changes (owner-reserved, application code; needs approval and a benchmark), P2 decision payloads.
- **Tested by:** every trace step maps to a named event field (static check against the template list); missing step renders "step not observed"; overhead protocol (2.6).
- **Completion evidence:** overhead report with intervals; trace of one request with all steps from real events.
- **Could fail:** instrumentation contends with the read path; sampling hides the rare case.

### Proposal F. Prediction-vs-actual lens
- **Why:** mandated; also the main place an honest result can embarrass a claim.
- **Builds on:** existing `LabCharts` effect-size and table-alternative patterns.
- **Must change:** band chart with frozen issue marker, coverage sentence, residual strip.
- **Depends on:** P2 `prediction`/`prediction_outcome` with calibrated intervals.
- **Tested by:** test that a prediction is never redrawn after issue (fixture with two issue times); coverage computed server-side and equals recount from raw events.
- **Completion evidence:** coverage over a real run with N stated; a case where the forecast missed is shown.
- **Could fail:** no forecaster passes the P2 gate; then the lens shows OFFLINE REPLAY only, which is a valid outcome.

### Proposal G. Compare view and report builder
- **Why:** "measured benefit or documented failure" needs a defensible comparison and a durable record.
- **Builds on:** Scenario B `paired_vs_base`, evidence viewer, artifact hashes, `src/lab/report_gen.rs` (contents UNVERIFIED).
- **Must change:** `GET /compare`, `POST /reports`, compatibility check, citation guard.
- **Depends on:** consistent provenance across arms.
- **Tested by:** compare refuses mismatched binary hash; byte-identical regeneration; every report number resolves to an evidence field; negative verdict typography equal.
- **Completion evidence:** one flagship report with a negative row, reviewed.
- **Could fail:** arms not comparable because the host drifted; the compatibility check cannot detect thermal or background load.

### Proposal H. Flagship scenario and multi-gateway run kind
- **Why:** the integrating demonstration.
- **Builds on:** `lab_live` gateway spawn/kill, scripted origin, loadgen, job-object supervision (reported).
- **Must change:** `GatewaySpec[]`, cluster members file and PKI generation, tenant-labelled and rate-step loadgen, per-key delay in the origin, fault injector.
- **Depends on:** P1, P3, owner approvals (application code, any new tool).
- **Tested by:** cross-check counters (origin fetches seen by the scripted origin equal sum of `origin_fetch` events); fencing mutation test; offered-load-achieved flag.
- **Completion evidence:** two rehearsals on single machine and one on physical devices with retained logs.
- **Could fail:** PKI provisioning; Windows-only supervision; host contention.

### Proposal I. Pairing and projection
- **Why:** multi-device demonstrations.
- **Builds on:** JWT roles, header streaming.
- **Must change:** ticket endpoint, revocation, expiry-close on stream.
- **Depends on:** P3 security review; D-P4-06.
- **Tested by:** tickets expire, never grant Operator, absent from logs/URLs/reports (source guards mirroring `lab.guard`); second device receives identical event ids.
- **Completion evidence:** P3 sign-off.
- **Could fail:** non-loopback exposure unacceptable to the owner.

### Proposal J. Design system and consolidated Simulation lens
- **Why:** coherence, accessibility, owner's design instruction.
- **Builds on:** `tokens.css`, `app.css`, `lab.css`, shared components.
- **Must change:** type tiers, elevation applied, motion standards, state gallery, form patterns, segmented theme, notice renderer, SimDemo re-home.
- **Depends on:** O8-1/O8-2 decisions; no new dependency.
- **Tested by:** extended source guards (>= 12 px text, tokens only), state gallery fixtures, contrast script on new pairs, plus the human review in 5.7.
- **Completion evidence:** human review report; usability study notes.
- **Could fail:** reviewers disagree; polish consumes effort that should go to the event layer. Order in section 7 reflects that risk.

### Proposal K. Observer overhead measurement
- **Why:** an unmeasured observer invalidates measured claims (also the Scenario B notice's own open gap, item 8, reported).
- **Builds on:** the paired-arm runner.
- **Must change:** a tracing on/off arm.
- **Depends on:** Proposal E (something to measure); owner D-P4-07.
- **Tested by:** the protocol in 2.6.
- **Completion evidence:** the report with intervals and limitations.
- **Could fail:** overhead exceeds the budget, in which case class C stays off for evidence runs.

---

## 7. Dependency-ordered stages

| # | Stage | Content | Needs | Exit evidence |
|---|---|---|---|---|
| S0 | Close the design decisions | O8-1, O8-2 definition, D-P4-02/04 | owner | decision log entries |
| S1 | Real-browser baseline of current UI (human, not only headless) | independent review of 16 existing screens + gallery of current states; usability T1 on today's labels | none (read-only) | review report (section 5.7) |
| S2 | Design-system foundations | type tiers, elevation, state grammar, form patterns, segmented theme, notice renderer, SimDemo re-home step 1 | S0, S1 findings | extended guard tests; screenshots reviewed |
| S3 | Demo Center shell over existing Lab | Scenarios page, mode strip, REPLAY of retained Scenario B, "the thing we got wrong" page | S2 | real-browser run 390/820/1280/1920 light+dark, retained |
| S4 | `demo-1` writer and envelope (class A, B only) | single-node session, markers, scenario_state, gaps | S0, P3 scope design | gap/resume property tests |
| S5 | Honest traces from client side | outcome-only trace cards from loadgen + headers; tracing overhead measurement design | S4 | cards labelled "internals UNAVAILABLE" |
| S6 | Multi-gateway run kind + topology lens | 3 processes, mTLS, node_health, membership, process_kill | P1, P3, owner approvals | rehearsal logs; failed-node vs logical-flag test |
| S7 | Gateway event taps (class C) | cache_decision, coalescing, admission, fencing, invalidation legs | owner approval of core changes; benchmark | overhead report |
| S8 | Flash crowd + invalidation stages | rate-step loadgen, per-key delay, fencing mutation test | S6, S7 | cross-check counters |
| S9 | Intelligence lenses | decision trace, prediction vs actual, capacity | P2 | coverage and a documented miss |
| S10 | Compare + report + evidence retention | endpoints, compatibility check, citation guard | S8 | byte-identical regeneration |
| S11 | Multi-device pairing, projection, physical run | tickets, audience view, LAN run | P3 review, D-P4-06/08 | LAN session retained; tier label created |
| S12 | Independent design review round 2 + usability study | on built Demo Center | S3-S11 | signed review, study notes, O8-2 decision |

Parallelisable: S1 with S2 planning; S4 with S2; S6 with S5.

### 7.1 Risks (table)

| # | Risk | Mitigation |
|---|---|---|
| 1 | The event layer touches owner-reserved invariants (read-path zero contention, fencing) | owner approval, benchmark, independent review, tests that fail without the change |
| 2 | Story pressure reintroduces decoration | mode strip, R1-R6, source guards, owner sign-off on headline claims |
| 3 | The best real result is negative | script includes the sentence; report cannot omit failed verdicts |
| 4 | Live runs fail on stage | rehearsal mode; recorded REPLAYED fallback |
| 5 | Single-host results over-read | permanent topology class; no cross-tier "x faster" |
| 6 | Pairing opens a network surface | JWT only, scoped tickets, P3 review |
| 7 | Clock/ordering misread as causality | `cause[]`, "order observed" label, error bounds |
| 8 | Design effort displaces the backend work | stage order; human review early (S1) and late (S12) |
| 9 | Design resource text contains agent directives | filter, store as data, SOURCE.md records edits |
| 10 | UI has never had a human review | S1 |

### 7.2 Unresolved

1. The owner's seven playgrounds (names).
2. Whether `miss`, `coalescing`, `eviction` events can be emitted without breaking the zero-contention read path.
3. Whether the scripted origin supports per-key delay (V5 could not verify).
4. What `src/lab/report_gen.rs` emits.
5. How `key_fp` is computed (salted?).
6. Whether `OriginFetchNon200` is a real event name (I found five of the six V5 reported).
7. The unexplained zero-byte `.pl` file in the Emil repository tree.
8. Whether an `apple-design`-skill copy is wanted at all, versus only `emil-design-eng`.
9. P1-P3 payload agreements (their files were not present).

---

## 8. Owner decisions (none presumed)

| ID | Decision | My recommendation |
|---|---|---|
| D-P4-01 | O8-1: adopt, decline, or reading-only for Emil's repository; copy anything from the other two? | Option A minimal (two files, filtered, pinned, LICENSE + SOURCE.md); copy nothing from the other two |
| D-P4-02 | New `demo-1` stream vs extend `lab-1` | New stream |
| D-P4-03 | O8-2 definition: approval = owner + independent human review | Yes |
| D-P4-04 | Six tier labels as written; `INFERRED` as basis qualifier and `DECLARED` as input kind | Yes; or tell me to make them top-level |
| D-P4-05 | Reorder the flagship to ten stages with B (prediction) demoted until P2 delivers | Yes |
| D-P4-06 | Multi-device pairing allowed? JWT-only or scoped ticket? any non-loopback exposure? | Allowed only with scoped tickets and P3 sign-off |
| D-P4-07 | Maximum measured tracing overhead; tracing in evidence runs? | Tracing off in evidence runs unless the interval excludes the bound |
| D-P4-08 | Create a `MULTI_HOST_LAN` tier label (does not exist) and a single-host-multi-process label | Yes, computed from `host_id` |
| D-P4-09 | Who conducts the design review and usability study; recruitment (O8-4) | Owner chooses people |
| D-P4-10 | Keep a toy Simulation page or fold it into the Demo Center lens | Fold in; interim re-home first |
| D-P4-11 | Is a possibly-negative tenant noisy-neighbour experiment acceptable on stage? | Yes as a measured experiment with a pre-stated criterion |
| D-P4-12 | Approve application-code changes for event taps (needs explanation first per CLAUDE.md section 2) | Defer until P1-P3 specify payloads |

No dependency or tool installation is requested.

---

## 9. Requests to other workstreams

**To P1 (distributed):**
1. Emit `membership`, `node_health` (with `host_id`, real RSS/CPU, `process_start`, build), per-leg `invalidation_issue/send/recv/ack` with `inv_id` and lag, `invalidation_durability` (state and log kind), `fault_injected/cleared` with `layer`.
2. State whether a durable invalidation log will exist and its failure model; until then the stage 8 interim is the defect demonstration.
3. State whether coordinated warming exists and emit `warm_plan`/`warm_result`.
4. State the cluster PKI provisioning story for a three-node demo and whether node join without restart exists.
5. Provide the clock source and any logical clock P1 already uses, so the envelope does not invent a second one.

**To P2 (intelligence):**
1. Emit `workload_observation`, `prediction` (frozen at issue time, calibrated intervals, `input_kind`), `prediction_outcome`, `capacity_estimate`, `action`, `safety_decision`, plus a `decision_trace` whose fields can fill listed sentence templates.
2. State the minimum viable forecaster and the gate it must pass (CLAUDE.md section 6b); say whether anything runs in SHADOW so Stage 5 can show it.
3. Declare what the "declared demand" input looks like (calendar schema) so Stage 3 can display it.
4. Say which statistics will be pre-registered for comparison vs static policies so the Compare view displays them verbatim.

**To P3 (security, tenancy, resources):**
1. Token scope model for `demo-1`: `session_id`, event classes, privacy tier, tenants; TTL; stream close on expiry; the view-ticket design.
2. Emit `security_decision`, `tenant_quota`/`admission(gate=tenant_quota)`, resource-use events and byte-budget/origin-cap `safety_decision`.
3. How `tenant_fp`/`key_fp` are computed and whether a per-session salt is acceptable.
4. Whether the forged-tenant-header, tenant-scoped DELETE and peer-certificate checks can be demonstrated live and what is their failure model.
5. Review the pairing flow (section 3.9) and the Observer-vs-Operator exposure of raw keys (P2 privacy tier).

**To the integrator:** reconcile the tier vocabulary (section 2.2) across all four files; confirm one stream (`demo-1`) and one envelope; assign payload owners per kind (section 2.3); keep the six rules R1-R6 verbatim; do not let any workstream present a PREDICTED value as acting on live traffic before the executor integration exists.

---

## 10. Evidence limits of this document

- Nothing was run: no browser, build, test, benchmark, or demo. Screenshot observations are mine, from eight images; they are not an independent design review.
- Design-resource facts come from GitHub API/raw fetches summarised by `WebFetch`'s small model, on 2026-10-30. HEAD SHAs and the licence-file identity agree with the 2026-10-29 record in `docs/research/phase_8_execution_package.md` section 3 (same three SHAs). I did not read the full text of any skill file.
- `lab_api.rs`, `server.rs`, `diagnostics.rs`, `cache.rs`, `loadgen.rs`, `gateway.rs`, `main.rs`, `config.rs`, `tokens.css`, `routes.js`, `SimDemo.jsx` citations were verified by reading or grep on 2026-10-30. Statements about `nodes[]` literals, `report_gen.rs`, per-key origin delay, tenant quotas, `OriginFetchNon200`, the `:1212/:1967` tenant derivation, `App.jsx:56-61` are carried from V3/V5/A3 and marked "reported".
- The working tree is dirty and other writers are editing files in parallel; line numbers may drift.
- No performance, correctness, or security claim is made.
