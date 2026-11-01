# R4: lifecycle, event and motion matrices

**Status:** Research (R4), 2026-10-30. Companion to `r4_system_objects_and_motion.md` (object IDs O1..O52, lifecycles L1..L7, flows F1..F14, discrepancies D1..D12 are defined there). Everything marked PROPOSAL is a research recommendation, not a decision and not an implementation plan. Tags: FACT (file:line read by R4), FACT-SECOND (from a cited repository document), INFERENCE, PROPOSAL, UNVERIFIED.

Verdict vocabulary (as requested): **IMPLEMENTABLE FROM EXISTING DATA**, **NEEDS NEW TELEMETRY**, **ILLUSTRATIVE ONLY (must be labelled)**, **NOT SUPPORTED**. Provenance of the data feeding a visual: REAL (live stream), REPLAYED (retained artifact), SIMULATED (T1), DERIVED, UNAVAILABLE.

## M0. Data characterisation (answers to R1, R2, R3, R5 requests)

### M0.1 What the live and lab feeds are

| Feed | Kind | Cadence / size | Timestamp | Identity | Replayable | Source |
|---|---|---|---|---|---|---|
| `GET /api/events?schema=2` | full snapshot (not an event log; no ids, no resume) | one frame per 100 ms **per client**; each frame rebuilds the payload (FACT) | `as_of_ms` (server wall clock) per frame; `uptime_ms` | none per frame | no | `server.rs:940-983` |
| Rate sampler | 1 s history, 15 points; `sample_if_due` refuses within 800 ms | 1 Hz internal | monotonic | n/a | no | `telemetry.rs:141-190`, `server.rs:460-475` |
| `GET /api/v1/diagnostics/events` | last 100 of a 512-entry ring, no cursor; `try_lock` drop counted in `diagnostic_events_dropped` | pull | `timestamp_epoch_ms` per event | `op_id`, `op_gen`, `key_fp` (salted) | no | `diagnostics.rs:180-227`, `server.rs:2154` |
| `GET /api/v1/diagnostics/summary` | counters snapshot (28 counters) | pull | none | none | no | `server.rs:2134` |
| `GET /api/v1/intelligence/status` and `/audit` | status / paginated audit (empty at runtime) | pull | record `timestamp_ms` | `decision_id` | n/a | `server.rs:389-455` |
| Lab `GET /runs/{id}/events` | SSE from persisted `events.jsonl`; resumable via `Last-Event-ID`; heartbeat comment 15 s | event-driven; T1 progress throttled to about 5/s; Scenario B emits phase changes | `ts` ISO per event | event `id` monotonic per run, `run_id` | **yes** | `lab_api.rs:60,647-752,2033-2082`, `lab_api_contract.md` s3 |
| Retained Scenario B artifacts | CSV/JSONL files | `gateway_snapshots.jsonl` about 250 ms (`t_ns` relative to load start); `client_requests.csv` per request (ms relative); `origin_requests.csv` per origin arrival (`arrival_ms_since_load_start`, `conc_at_arrival`, `service_ms`, `degraded`, `path_hash`) | relative only; wall clock only in manifest | `idx`, `key` (client csv), `path_hash` (origin csv), `rep`, `arm` (folder name) | **yes** (files; the API exposes names/sizes/SHA-256 and a download route, not parsed series) | `lab-results/scenario_b/20261030T003932Z/runs/*` (FACT), `lab_api.rs:2147` |
| Retained T1 `cells.csv` | one row per cell (36 columns, aggregated); per-request and per-fetch CSV writers exist (`REQ_CSV_HEADER`, `FETCH_CSV_HEADER`, `t1_sim.rs:561,572`) but the sealed run retains only `cells.csv` | n/a | virtual time | `cell_id` = scenario|family|arm|seed | yes (aggregate) | FACT |

**Values by type (R1 request 2).** Snapshot gauges: `bytes_resident`, `byte_budget`, `origin.in_flight`, `origin_cap.{cap,in_flight,enforcing}`, `revalidations.in_flight`, `nodes[].state`. Cumulative counters: `requests_total`, `cache_hits`, `cache_misses`, `coalesced_waiters`, `evictions_total`, `capacity_rejections`, `stale_*`, `revalidations.*`, `origin.requests_*`, `origin.errors.*`, `origin_cap.{dispatched,origin_cap_rejections,controller_steps}`, rejections. Derived windows: `hit_ratio_{1s,10s}`, `throughput_req_s_{1s,10s}`. Distribution summaries: `latency_ms` (p50/p90/p95/p99 + `samples`), `origin.latency_ms` (p50/p95/p99). Events: none in the stream. Time series: only the UI-side history (120 points, `ui/src/lib/history.js:5`, per R3) built from frames; the gateway keeps no history except the 15-point rate sampler.

**Histogram exposure (R2 request 2).** Not raw. The stream carries pre-aggregated percentiles. The 2 688-bucket `AtomicLogLinearHistogram` is fed on every GET (`telemetry.rs:472-478`) but its buckets are not in any payload, and the stream's percentiles are computed by sorting the raw 10 000-entry observation ring (`telemetry.rs:500-517`; discrepancy D6). The origin histogram has 15 fixed bucket edges internally and also yields only p50/p95/p99. Distribution charts are therefore NOT SUPPORTED live; retained `client_requests.csv` supports them (REPLAYED).

**Timestamps (R2 request 3).** Live stream: one wall-clock `as_of_ms` per frame (client can build an actual-time axis, with 100 ms resolution at best). Scenario B snapshots: `t_ns` ns since load start (relative axis, aligned across `client_requests.csv` ms and `origin_requests.csv` ms of the same run, INFERENCE: all three are relative to the same load start; not verified that offsets are identical). Diagnostic events: wall-clock ms. Cluster / invalidation propagation: no timestamps exposed.

**Scenario B per-sample vs summary and interval method (R2 request 4).** The API serves a summary (`summary.json`, parsed by `ScenarioBResult`) and artifact downloads. Summary intervals: per paired metric `diff_ci95_bca` (BCa bootstrap 95% interval of the paired mean difference), `relative_reduction_ci95_bca`, `n_pairs` = 10, `permutation_p_sign_flip` (e.g. 0.001953125 = 2/1024 with 10 pairs), `mean_base`, `mean_treat` (FACT, `lab-results/scenario_b/.../summary.json`, `src/lab_live/analysis.rs:50-55`). Verdicts: `outcome` (e.g. `TRADE_OFF_FAILURE` for AIMD-CAP: peak reduction 64.0%, CI [62.5%, 65.3%], but client success rate 17.1 percentage points below BASE), `reasons`, `evidence` (FACT, same file). Per-sample data exist only in the CSVs. Pooled per-arm client latency percentiles are in `summary.json` (`client_latency_ms_pooled_scheduled_time`, includes sheds: ALL_REQUEST_LATENCY_NOTE).

**Per-entry state (R2 request 5, R3 R4-2).** None observable: no entry list, no per-key age, generation or in-flight state in any endpoint. Identity: keys have a salted `key_fp` only in diagnostic events; nodes have `node_id` (u8, stable per cluster configuration); runs have `run_id` (UTC timestamp + random suffix); arms are named (`BASE`, `STATIC-CAP`, `AIMD-CAP`, or T1 arm ids); requests have no id; origin fetches have `op_id` only inside diagnostic events; invalidations have no id.

**Controller decision log (R2 request 6, R5).** None. The only controller output is the `cap` gauge and the `controller_steps` counter. The step inputs (window p99, error rate) are not recorded. `DecisionRecord`/`AuditStore` exist but are never appended to at runtime.

### M0.2 Run states per run kind, and the events that carry them (R5 request 1)

| State | `t1_matrix` | `scenario_b` | Event carrying it |
|---|---|---|---|
| queued | yes | yes | `state {state:"queued", tier}` (T1: `SIMULATED`; SB: `MEASURED_LOOPBACK`) |
| running | yes | yes | `state {state:"running"}`, then `progress` |
| cancelling | yes: cooperative, a simulation stops at the next cell boundary | yes: gateway children terminated | `state {state:"cancelling"}` |
| cancelled | yes (partial artifacts) | yes (partial artifacts; see R5 U-01 re verdict content in a cancelled summary, UNVERIFIED by R4) | `state`, `done {state, exit_code}` |
| completed | yes: `result {tier, cells, cells_csv_sha256, ...}` | yes: `result {tier, label, label_reasons, verdicts, ...}` | `result`, `state`, `done` |
| failed | yes | yes (runner crash is never reported as completed) | `error {code, message}`, `state`, `done` |
| interrupted | yes: gateway restarted while active | same | `state {state:"interrupted", source:"server_recovery_on_startup"}` + `done` appended at startup recovery |

FACT sources: `lab_api.rs:641-760` (emit and state machine), `:911-950` (recovery), `:1807,1845` (progress payloads), `ui/src/lib/labProvenance.js:46-54`, fixtures `ui/test/fixtures/lab/events_{t1,sb,cancelled}.sse`. Progress payloads: T1 `{done,total,tier}`; Scenario B `{done,total,arm,rep,phase,gateway_pid,tier}` where `phase` is one of `starting, warmup, healthy, degraded, recovery, collect, run-complete` (FACT: `scenario_b.rs:970-971,1077,1241` and fixture). Only one run may be active (409); a stream is limited to 16 concurrent SSE streams (FACT-SECOND `04_lab_ux_design.md` 1.1.1).

### M0.3 Client observations versus server facts (R5 request 2)

| State / indication | Where derived | Class |
|---|---|---|
| Telemetry connection kind (`loading`, `live`, `stale`, `disconnected`, `error`, `degraded`) | browser: reducer over EventSource phases and age of the last frame (`STALE_AFTER_MS = 3000`, `ui/src/lib/connection.js:4,58-71`) | CLIENT observation (the UI badge calls live "Derived (inferred)", per R5) |
| Run-stream status (live / reconnecting / stale / disconnected / failed ...), backoff 1 s to 30 s, stale after 25 s | browser: `labStream.js:11-15,87-98` | CLIENT observation (the server only sends a heartbeat comment every 15 s) |
| Gateway `mode` (`live` / `simulation`), `uptime_ms`, `capabilities.{cluster,controller,origin_metrics}`, `nodes[].state` | server (`server.rs:498-566,660-690`) | SERVER fact; `nodes[].state_src` states `inferred` (router configured, not probed) or `simulated` |
| Run state, progress, tier, verdict, label | server (lab) | SERVER fact (verdicts are DERIVED by experiment code) |
| Hit ratio windows, throughput | server (rate sampler) | DERIVED server-side |
| "Reconnecting" for the telemetry stream | not modelled (R5 finding 5) | n/a |

### M0.4 Telemetry stream authentication (R5 request 3)

FACT: `sse_handler` authenticates from request headers when `security.require_auth_for_control` is set (Observer role; `server.rs:940-952`, `authenticate_request` reads the Authorization header). FACT: `useTelemetry.js:32` constructs `new EventSource(url)`; the EventSource API cannot add headers and the token is never put in the URL (`docs/design/sse_auth_options.md`, FACT-SECOND). INFERENCE (confirms R5): under JWT enforcement the live telemetry stream returns 401 to the browser; the Lab, which uses fetch streaming with an Authorization header (`lab_api.md`, FACT-SECOND), can still work. Not observed in a browser by R4. Consequence for design: any live motion depends on the stream being connected; on a JWT gateway the live screens would show a connection failure while Lab data remains available. The decision belongs to the owner (BD-05 per R5).

## M1. Motion and continuity opportunity matrix (verified operations)

Columns: Operation | objects involved | what changes | trigger | what the user should understand | motion metaphor | driver (event / state / user / illustrative) | identity persists across screens? | data required | a11y and performance constraints | **verdict**.
Constraint shorthand used in the last-but-one column: RM = a reduced-motion equivalent is a non-animated state change plus text (DD v2 s6.3); NO-SR = must be reachable as text/table; N<=20 = fewer than about 20 simultaneously animated elements (R2's standing answer); 100ms = the stream cannot deliver changes faster than every 100 ms and counters are cumulative, so motion can follow counter deltas but not individual requests.

| # | Operation (flow) | Objects | What changes | Trigger | User should understand | Metaphor | Driver | Identity across screens | Data required | Constraints | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| V1 | Request outcome mix (F1-F4) | O1, O6, O7, O8, O11, O14 | shares of HIT / MISS / COALESCED / STALE / shed; remainder unattributed | counter deltas between frames | where requests ended up; what fraction the cache absorbed | aggregation (stacked bar / segmented rail) | state-driven (smooth, not per request) | outcome categories (stable names) | `requests_total`, `cache_hits`, `cache_misses`, `coalesced_waiters`, `stale_served_total`, `origin_cap.origin_cap_rejections`, `capacity_rejections`; remainder computed against `requests_total` (D5) | 100ms; RM; NO-SR table | **IMPLEMENTABLE FROM EXISTING DATA** (bypass and waiter-limit rejections not in the stream: shown inside the remainder) |
| V2 | One request's journey (hit / miss / coalesce / origin) | O1, O4, O6, O7, O11, O12 | a token moves gateway → lookup → (cache / origin) → response | a request | how a request is resolved | direction / transformation | event-driven if real; else illustrative | request id (does not exist) | per-request or sampled trace event (none today) | N<=20; never one token per request at load | Live: **NEEDS NEW TELEMETRY**; static diagram: **ILLUSTRATIVE ONLY (must be labelled)** (the existing Simulation page graphic); retained: partially REPLAYED from Scenario B `client_requests.csv` (status, latency, no leader link) |
| V3 | Coalescing convergence (F4) | O11, O12, O16 | many callers → one origin call → result to all | follower joins (not evented) | N callers shared one fetch | aggregation / convergence | illustrative for the per-fetch picture; state-driven for the aggregate | none across screens (fetch ops have no id outside diagnostics) | aggregate: `coalesced_waiters`, `cache_misses`. per-fetch fan-in: absent | RM; N<=20 | Aggregate follower-versus-leader ratio: **IMPLEMENTABLE FROM EXISTING DATA**. Per-fetch convergence animation: **ILLUSTRATIVE ONLY** now; real version **NEEDS NEW TELEMETRY** (M3 T2/T3). Retained evidence: coalescing was about 1.4% of window requests (BASE) so a real visual would be modest |
| V4 | Cache fill (F5) | O5, O16, O20 | entry stored; resident bytes rises | finalize Success | the cache grew by one entry; bytes within budget | persistence / accumulation | state-driven (bytes gauge) | none | `bytes_resident`, `byte_budget`, fill event absent | RM | Aggregate bytes: **IMPLEMENTABLE FROM EXISTING DATA**. Per-fill: **NEEDS NEW TELEMETRY** |
| V5 | Entry freshness transition (L1, F6) | O5, O17 | fresh → expired → stale-eligible → not serveable → reclaimed | time | what "stale but servable" means and where the hard cap sits | transformation | illustrative | key | per-entry age/state: absent. Config values: `PHANTOM_MAX_STALE_MS` and per-response directives vary; default TTL is a literal 60 s | must not draw configured values as effective (D7); label example values | Live: **NOT SUPPORTED** (no per-entry state). Explainer: **ILLUSTRATIVE ONLY (must be labelled)** as a model with named example numbers. Aggregate entry-state counts: **NEEDS NEW TELEMETRY** (M3 T5) |
| V6 | Stale served and background revalidation (F7) | O8, O9 | counters tick; in-flight refresh count rises/falls | counter deltas | the cache served an expired entry within bounds and refreshed behind the scenes | feedback | event-driven by counter delta | none | `stale_served_total`, `stale_cap_violations` (must be 0), `revalidations.*` | 100ms; RM; violation > 0 must be shown as critical (as `catalog.js` does) | **IMPLEMENTABLE FROM EXISTING DATA** (note: Scenario B never exercised stale serving; real data appear only under a workload with SWR/SIE directives) |
| V7 | Revalidation outcome (304 vs 200 vs error) | O9 | split of `ok`, `not_modified`, `error` | counter deltas | whether refresh saved bandwidth (304) | aggregation | state-driven | none | `revalidations.{ok,not_modified,error}` | RM | **IMPLEMENTABLE FROM EXISTING DATA** |
| V8 | Eviction under byte pressure (F5) | O19, O20 | resident bytes approaches budget; evictions tick; rejections | fill beyond budget | the cache is full and discarding entries | feedback | state-driven | none | `bytes_resident`, `byte_budget`, `evictions_total`, `byte_budget_rejections`, `oversize_rejections`, `capacity_rejections` | RM | Aggregate: **IMPLEMENTABLE FROM EXISTING DATA**. Which entry was evicted / policy decisions: **NEEDS NEW TELEMETRY** (M3 T7) |
| V9 | Origin admission and shedding (F8) | O13, O14, O12 | in-flight against cap; rejections increment (503) | requests reach the origin path | the origin is protected by refusing excess work; refused requests are failures, not savings | selection / feedback (gate) | state-driven (gauge vs cap, rejection ticks) | cap/in-flight (stable names) | `origin_cap.{enforcing,cap,in_flight,dispatched,origin_cap_rejections}`; only meaningful when `enforcing` is true (Canary/Active with configured cap; otherwise the cap is not enforced, FACT `executor.rs:217`) | 100ms; RM; label MEASURED_LOOPBACK for Scenario B replays; show `enforcing=false` explicitly | **IMPLEMENTABLE FROM EXISTING DATA** (live and REPLAYED from `gateway_snapshots.jsonl`) |
| V10 | AIMD cap trajectory (O15) | O15, O13 | cap steps down by about 30% on a bad window, +1 otherwise | one step per 32 origin responses | the controller changed the limit | direction (step-line) | state-driven | arm / run | `origin_cap.cap`, `controller_steps`; reason (window p99, error rate) absent | RM; do not label "adaptive intelligence" or "learning" (notice: cap behaved as a static cap near 12 in the retained experiment) | Trajectory: **IMPLEMENTABLE FROM EXISTING DATA**. Decision reasons: **NEEDS NEW TELEMETRY** (M3 T9) |
| V11 | Experiment phase timeline (F13) | O45, O47 | run progresses through starting, warmup, healthy, degraded, recovery, collect per arm and repetition | `progress` events | where in the experiment the run is; when the origin degraded | direction / progress | event-driven (REAL) | `run_id`, arm, rep | Lab `progress {done,total,arm,rep,phase}` | RM; announce state changes politely; one active run | **IMPLEMENTABLE FROM EXISTING DATA** |
| V12 | Arm comparison with intervals (F13) | O46, O48 | paired differences with BCa CI per metric; per-phase shed and latency | run completion / evidence load | whether the arms differ, by how much, with what uncertainty and trade-off | selection / comparison (not animated per datum) | user-triggered (reveal/toggle) | arm names | `summary.json`: `paired_vs_base`, `arms`, `verdicts` | scientific charting; table equivalent; negative verdicts styled identically (lab contract s6) | **IMPLEMENTABLE FROM EXISTING DATA** (T1: aggregated `cells.csv`; SIMULATED) |
| V13 | Verdict reveal | O48 | outcome chip (e.g. TRADE_OFF_FAILURE, NOT_SUPPORTED) with reasons | result event | what is established and what is not (binding notice) | completion | event-driven (once) | verdict id / evidence id | `verdicts`, `label`, `label_reasons`, notice strings | no celebratory motion for any verdict; cancelled runs: verify summary content before showing a verdict (R5 U-01) | **IMPLEMENTABLE FROM EXISTING DATA** |
| V14 | Evidence retention and integrity | O49 | artifact list with sizes and SHA-256; manifest | evidence load | the result is a retained, hashed record | persistence | state-driven | evidence id (`scenario_b.<utc>`, folder names) | `/evidence`, `/runs/{id}/artifacts` | NO-SR | **IMPLEMENTABLE FROM EXISTING DATA** |
| V15 | Single-node invalidation (F9) | O21, O23, O5 | entry removed; generation advances; pending fetch detached | DELETE | a purge takes effect on this gateway immediately | completion | user-triggered (client action) | key | HTTP status of the DELETE the client itself issued; the UI has no DELETE control today | n/a | No gateway-side event: **NEEDS NEW TELEMETRY** for a live indication; a UI-issued DELETE feedback would need a product decision (PROPOSAL, not recommended without owner approval) |
| V16 | Invalidation propagation between nodes (F10, L5) | O29, O30, O31, O24 | messages from issuer to peers; acks; repair | DELETE in cluster mode | the purge reached which peers and how | distribution | illustrative only | invalidation id (none) | send/recv/ack events with ids and lag: none | must not imply bounded delivery or durability (M3 limit; claim C6 weak); no animated message flow | **ILLUSTRATIVE ONLY (must be labelled)**; real version **NEEDS NEW TELEMETRY** (M3 T10); durability claims **NOT SUPPORTED** |
| V17 | Repair backlog | O30 | owed invalidations per peer rise and drain | peer failure / heal | some peers still owe a purge | persistence / recovery | state-driven if exposed | peer id | per-peer pending count (getter exists in process: `RepairLedger::pending_count`, `router.rs:305`); not exposed | RM | **NEEDS NEW TELEMETRY** (gauge only; cheap) |
| V18 | Node state change (L4) | O24, O28, O32 | node chip changes state | state write | which nodes the router would avoid | recovery / state | state-driven (diff of `nodes[].state` between frames) | `node_id` | `nodes[].{node_id,state,state_src}` (only with a router or simulation) | no health colour for `simulated` / `inferred` without the label; `NODE_FAILURE` must read SIMULATED | State text: **IMPLEMENTABLE FROM EXISTING DATA** (labelled inferred/simulated). Cause / recovery steps: **NEEDS NEW TELEMETRY** (M3 T11). Rendering NODE_FAILURE as a failure: **NOT SUPPORTED** |
| V19 | Routing / key ownership (O26) | O26, O25 | which node owns a key | key | where a DELETE would go | selection | illustrative | key, node | none | GET is local-only, weights are constant: ownership does not describe data placement | **ILLUSTRATIVE ONLY (must be labelled)** for DELETE routing; "data placement" **NOT SUPPORTED** |
| V20 | Hot keys and demand (F12) | O35, O36 | top-K keys, skew | window | which keys dominate | data visualisation | state-driven | key fingerprint | stream says `hot_keys` unavailable (`not_implemented`) | privacy: fingerprints only | **NOT SUPPORTED** now; real version **NEEDS NEW TELEMETRY** (M3 T12); workload families in T1 are SIMULATED generator parameters only |
| V21 | Workload signal windows and prediction (F12) | O36, O37 | rate, hit ratio, Zipf alpha, burst, forecast with interval | window | what workload the cache sees; what is predicted | data visualisation | state-driven | window id | `SignalWindow` never produced at runtime | prediction must show issue time and error (V2 forecasting doc) | **NOT SUPPORTED** (library only); **NEEDS NEW TELEMETRY** |
| V22 | Policy candidate evaluation / shadow caches (F12) | O38, O39 | shadow hit ratios per policy | window | which eviction policy would have done better | comparison | state-driven | candidate id | none at runtime; T1 cells compare arms (SIMULATED) | SIMULATED labelling | **NOT SUPPORTED** live; T1 arms: **IMPLEMENTABLE FROM EXISTING DATA** (SIMULATED) |
| V23 | Canary, guardrail, rollback (L7) | O40-O43 | traffic split; guardrail trip; revert | decision | adaptive control was bounded and reversible | state timeline | illustrative | action id (none) | none: `route_bucket`, watchdog, `revert_to_safe` not in serving path; audit empty | no animation of non-existing events; the legacy PIE is a threshold stub (`pie: None`) | **NOT SUPPORTED** (live); lifecycle diagram **ILLUSTRATIVE ONLY (must be labelled)** |
| V24 | Connection state change (client) | telemetry / run stream | chip changes | EventSource events, frame age | the page's view is current or not | feedback / recovery | event-driven (client) | connection | client reducers (M0.3) | RM; no animation for states the code cannot distinguish (R5) | **IMPLEMENTABLE FROM EXISTING DATA** (client-derived; must be labelled as such) |
| V25 | Throughput and hit ratio over time | O1, O6 | sparkline | frame | trend since the page loaded | direction | state-driven | metric id | stream windows (1 s, 10 s) and page-held history (120 points) | the page-held history is not gateway history (already stated in UI) | **IMPLEMENTABLE FROM EXISTING DATA** |

Rules derived (PROPOSAL): (1) per-request motion is not supported anywhere live; no animation may imply a request-level or message-level event that the backend does not emit. (2) Counter-delta-driven motion (V1, V6-V9) is supportable at no more than the 100 ms frame rate; typical changes can be shown as a short, non-positional emphasis (no travelling tokens). (3) REPLAYED motion must be visibly framed as a replay of a retained run with tier and run id. (4) Object identity across screens is only available for nodes (`node_id`), runs (`run_id`), arms (names) and evidence ids; keys, requests, fetches and invalidations have no stable ids in any feed.

## M2. Candidate visual experiences (grounded)

| Experience | Real data available now | Missing | Honest feasibility today | First real-data slice (PROPOSAL) |
|---|---|---|---|---|
| **Request Flow** | outcome counters (V1), origin gate (V9), per-request status only in retained Scenario B CSV | per-request or sampled trace, request id, coalescing join events | Aggregate outcome flow: IMPLEMENTABLE. Individual request path: ILLUSTRATIVE ONLY; real version needs T4 | segmented outcome rail with explicit "unattributed" part; text table twin |
| **Cache Lifecycle** | bytes vs budget, evictions, stale and revalidation counters | entry counts and states, age distribution, per-entry events | Counters only. Entry-level lifecycle: ILLUSTRATIVE ONLY (model with example numbers) | state-machine diagram tied to the counters that really exist |
| **Cache and Cluster Space** | `nodes[]` state (inferred/simulated), `capabilities.cluster` | peer links, acks, repair backlog, ownership that means anything for GET | Node table only. Spatial invalidation propagation: ILLUSTRATIVE ONLY; cluster mode experimental and loopback-tested | keep the placeholder; add repair backlog gauge only after T10/T11 |
| **Workload Intelligence** | none live (`hot_keys` unavailable); T1 workload families (SIMULATED) | pipeline wiring, hot keys, signals | NOT SUPPORTED live | an explicit unavailable panel (already designed); T1 generator parameters as a labelled explainer |
| **Experiment Journey** | everything in V11-V14: state, progress phases, results, CIs, verdicts, hashes | none for Scenario B summaries; per-sample series only via artifact download | IMPLEMENTABLE (strongest area) | timeline: configure → run phases (live progress) → paired results with intervals → verdict and notice → retained evidence with hashes |
| **Adaptive Control** | `origin_cap` gauge series (live; REPLAYED in Scenario B), controller step counter, intelligence mode flag | decision log, canary, rollback events, prediction | Origin concurrency controller trajectory: IMPLEMENTABLE. Observe/evaluate/canary/rollback timeline: NOT SUPPORTED; diagram ILLUSTRATIVE ONLY | in-flight vs cap step-line with rejections; title "Origin concurrency controller" |

## M3. Missing telemetry and event requirements (PROPOSALS only; none exists)

Event names are proposals and align with `docs/research/vision/three_pillar/04_lab_ux_design.md` s2.3 where marked [04]. Rates are derived from known loads: stream 10 Hz per client (FACT); Scenario B 300 rps (FACT-SECOND `scenario_b.rs:557`); production rates are UNVERIFIED. "Overhead" is the concern named in the brief: any per-request emission perturbs the thing measured (the hot path holds a shard mutex; `DiagnosticBuffer::push` is already lossy by design).

| ID | Proposed signal | Fields | Frequency / volume estimate | Privacy / tenant | Overhead concern | Enables | Relation to [04]/V5 |
|---|---|---|---|---|---|---|---|
| T1 | Add existing counters to schema 2: `cache_bypasses`, `waiter_limit_rejections`, `stale_writes_prevented`, `stale_ops_ignored`, `reclamations_total`, `origin_fetches_failed`, `anti_entropy_*` | counter values | 10 Hz frame, ~0 extra | none | none (atomics already exist) | complete outcome accounting (V1), fence activity (L2), anti-entropy gauges | [04] 1.2.5 lists these as counters only; agree |
| T2 | `origin_fetches_started` is `origin.requests_started` already; add a `followers_per_fetch` histogram (log buckets) updated at fetch close | bucket counts | one atomic add per fetch | none | negligible (done outside the shard lock after waiters are extracted) | real distribution of fan-in; "origin requests avoided" without estimation | differs: [04] proposes per-event `coalescing` with `waiters_now`; R4 prefers an aggregate at close |
| T3 | `coalescing_group_closed` (sampled) | `op_id`, `key_fp`, `followers`, `first_join_ms`, `last_join_ms`, `outcome` | sampled by hash(`key_fp`), e.g. at most 5/s | `key_fp` salted only | emit after lock release | per-fetch convergence view (REAL) | refines [04] `coalescing`: group summary, not per-follower |
| T4 | Sampled request trace (`request`, `cache_decision`, `origin_fetch`, `fencing`) with `trace_id` | per [04] s2.3 | Class C: all non-hit, N% of hits, deterministic by hash. At Scenario B load (300 rps, about 62% of window requests non-hit, FACT summary.json) "all non-hit" alone is about 190 events/s, so a stated per-second cap or lower sampling is needed (INFERENCE) | tenant slicing needs token scope; no raw keys | each event costs a clock read and a ring push (try_lock, may drop) | Request Flow (real) | agree with [04] class C |
| T5 | Entry-state gauges: `entries_total`, `entries_by_state{fresh, stale_window, expired_resident, placeholder}`, age histogram | gauges | 1 Hz (the sweeper already iterates entries) or incremental | none | an extra pass per sweeper tick; incremental counters preferred | Cache Lifecycle (real, aggregate) | not in [04]; PROPOSAL |
| T6 | `eviction` (sampled) | reason, policy, bytes, `victim_key_fp` | at most a few/s sampled | salted fp | emitted from inside the shard lock in `evict_with_policy` today; defer to after lock | eviction explainers | agree [04] |
| T7 | `fence_result` already counted (`stale_writes_prevented`); event only if sampled | result, `op_gen`, `current_gen` | rare | none | low | fencing explainer | agree [04] |
| T8 | Raw latency histogram buckets `latency_hist` (2 688 buckets is large; export a coarse 64-bucket projection) | bucket counts | 1 Hz | none | removes the per-frame sort of up to 10 000 values (D6) | latency distribution charts | R3/R2 request; not in [04] (it marks R-UI-09 open) |
| T9 | `origin_cap_decision` | step index, window size, window p99, error rate, old cap, new cap, reason (`md`/`ai`) | one per 32 origin responses (at 300 rps about 9/s worst case, far less when mostly hits) | none | very low | controller trace (real) | refines [04] `action`/`safety_decision` |
| T10 | `invalidation_issue/send/recv/ack/repair` with `inv_id`, `from`, `to`, `leg`, `lag_ms`, `result`; plus per-peer `ledger_pending`, `full_flush`, `ae_gaps` gauges | per [04] | invalidations are rare (operator-driven); bursts bounded by queue 256 per peer | key = salted fp | acks already traverse the code path | Distributed invalidation (real) | agree [04]; note `lost_with_issuer` cannot be emitted by the failed issuer: it can only be inferred by an observer (differs from [04] `invalidation_durability`) |
| T11 | `node_health` with `cause` (`send_failed`, `timeout`, `repair_done`, `reconcile_done`), `last_seen_ms`, `membership_view`; stop reporting cpu/memory as placeholders | per [04] | on change | none | none | Node recovery story (real) | agree [04] |
| T12 | `hot_keys` top-K | fingerprints, counts, window, `sample_rate` | 1 Hz | salted fp; tenant slicing optional | wiring `HeavyHitterDetector::observe` into the request path costs a mutex or sharded update per request: sample | Workload Intelligence | [04] `workload_observation` |
| T13 | `workload_window` (SignalWindow summary with provenance per feature) | per `mod.rs:98` | 1-5 s | none | pipeline runs off the hot path if fed from a bounded channel | Workload Intelligence | [04] |
| T14 | `decision_record` append for each controller action into the existing `AuditStore` | `DecisionRecord` | per controller action | none | negligible | Adaptive Control timeline; the audit endpoint stops being empty | agree [04] `action` |
| T15 | Per-tenant counters (hit, miss, bytes) | tenant fp | 1 Hz | requires tenant scoping in the stream token; tenant ids are namespaces, treat as sensitive | per-tenant maps | tenancy views | [04] `tenant_quota` |
| T16 | Request / trace id in a response header | opaque id | per request | opaque | header write | links client records to server events | [04] 1.2.7 |
| T17 | A monotonic sequence number and `gap` signalling on every event stream | `seq`, `gap` | per event | none | none | trust in "no loss" claims for motion | agree [04] s2.4 |

Overhead rule (PROPOSAL): prefer aggregate gauges and close-time summaries to per-request events; sample deterministically by hash; never emit while holding a shard or epoch lock; always report drops.

## M4. Internal capability versus proposed capability register

| Capability | Status | Evidence |
|---|---|---|
| Single-flight coalescing with bounded waiters | Implemented | `cache.rs:1292-1316`, `coalesce.rs` |
| Generation and epoch fencing of fills | Implemented | `cache.rs:1448-1475` |
| Hard stale cap | Implemented | `cache.rs:749-822` |
| RFC 9111 revalidation (ETag / Last-Modified, 304) | Implemented | `cache.rs:1025`, `server.rs:1358,1758` |
| Origin concurrency cap with AIMD controller | Implemented; enforced only in Canary/Active with config | `executor.rs`, `candidates.rs:135` |
| Byte-accounted eviction, three policies | Implemented | `eviction.rs`, `cache.rs:1201` |
| Exact-key distributed invalidation | Partial (experimental, loopback, not durable across issuer restart) | `router.rs:1790`, M3 reproduction report |
| Tenant / full invalidation over HTTP | Not implemented (library and wire only) | `server.rs:199-214` |
| Weighted rendezvous routing | Partial (plain HRW, constant weights, DELETE only) | `router.rs:1363` |
| Coordinated GET across nodes | Not implemented by decision (ADR-003) | `server.rs`; a2 row 5a |
| Dynamic membership / join / gossip | Not implemented | a2 row 2b |
| Heartbeat-driven failure detection | Partial: pool heartbeats do not set `NodeState` | a2 row 4a |
| Workload pipeline, hot keys | Library only; hot keys explicitly unavailable | `signals.rs`, `server.rs:766` |
| Prediction / forecasting | Library only | `candidates.rs:175` |
| Shadow evaluation, admission filter, stale controller | Library only (T1 simulator drives admission and stale controller) | `shadow.rs`, `t1_sim.rs` header |
| Canary routing, watchdog, rollback | Library only, no caller | `executor.rs:270,285,390` |
| Decision audit | Types and endpoint exist; store empty at runtime | `audit.rs`, `main.rs:480` |
| Telemetry schema 2 with provenance | Implemented | `server.rs:498-780` |
| T1 simulation with real cache code on a virtual clock | Implemented, SIMULATED | `t1_sim.rs` |
| Scenario B real-gateway experiment, retained evidence | Implemented, MEASURED_LOOPBACK, debug label per UI | `scenario_b.rs`, `lab-results/scenario_b/` |
| Lab API with SSE, resume, persistence | Implemented | `lab_api.rs` |
| Per-request events, tracing, sampled trace | Designed only ([04] s2.3) | `04_lab_ux_design.md` |
| Per-entry visibility | Not designed in code; proposed here (T5) | n/a |
| Multi-host experiments, fault injection layers | Designed only; cluster fault proxy is test-only | a2 row 9b |
| Stale-serving effectiveness evidence | Unsupported (workload did not exercise it) | `scenario_b.rs:567` |
| Adaptive-control benefit | Unsupported (sealed M6 H1 NOT_SUPPORTED, Scenario B TRADE_OFF_FAILURE for AIMD-CAP) | `m6_sealed_results.md`, `summary.json` |

## M5. PhantomCache capability | visual opportunity | external capability needed | feasibility today

"External capability needed" names a capability class; the toolkit decision is R1/R2/R3's and the lead's. R2 currently finds that CSS/WAAPI and SVG suffice and recommends no animation, chart, 3D or diagram runtime dependency (`r2_motion_materials_diagrams_dataviz.md` s12, as read by R4); R4 does not contradict that on the facts above.

| PhantomCache capability | Visual opportunity | External capability needed | Feasibility today |
|---|---|---|---|
| Request coalescing | callers converge on one fetch (convergence / aggregation) | SVG diagram + CSS/WAAPI transform-opacity transitions (≤ 20 elements); for real data, new telemetry T2/T3 | Aggregate ratio: yes. Per-fetch convergence: illustration only |
| Cache hit / miss / stale / shed mix | segmented outcome rail with honest remainder | static SVG/CSS bars, table twin | Yes (existing counters) |
| Cache lifecycle (fresh, stale, expired, evicted) | state animation / state machine | state-machine diagram; CSS state transitions | Illustration (labelled) now; aggregate real with T5 |
| Byte budget and eviction | meter + counters | CSS/SVG meter | Yes |
| Origin protection (gate, shed, AIMD) | in-flight vs cap step-line with rejection markers | time-series chart (SVG/canvas), step interpolation, small multiples by phase | Yes live (enforcing only) and replayed (Scenario B); decision reasons need T9 |
| Freshness / revalidation | counters with 304-vs-200 split | simple bars | Yes |
| Distributed invalidation | messages propagating between nodes (distribution) | spatial/sequence diagram with step-through | Illustration only until T10; no durability or bound claims |
| Node failure and recovery | state chips and a recovery timeline | state chip transitions | State text yes (labelled inferred/simulated); causes need T11; NODE_FAILURE is SIMULATED |
| Rendezvous routing | key → owner selection | diagram | Illustration only; meaning limited to DELETE routing |
| Workload analysis (hot keys, Zipf, bursts) | hot-key bars, demand curves | bar chart, sparkline | Not supported live; needs wiring + T12/T13 |
| Prediction | forecast with intervals and later error | interval chart | Not supported; library only |
| Experiment comparison (arms, outcomes) | paired difference plot with intervals; per-phase small multiples | scientific charting (dumbbell/interval plot), table toggle | Yes (summary.json); per-sample via artifacts (REPLAYED) |
| Experiment journey (config → run → results → evidence) | phased timeline with progress and verdict | progress/timeline component, SSE client (exists) | Yes (lab-1 events) |
| Adaptive policy (observe, evaluate, canary, rollback) | decision timeline | timeline/state visualisation | Not supported (machinery unwired); diagram illustration only |
| Retained evidence integrity | hash/manifest list | none | Yes |

## M6. Opportunities that cannot be implemented now because the backend lacks events or data

1. Per-request or per-fetch animation (request flow, coalescing convergence) on live data: no request id, no join event, no fan-in count.
2. Cache entry lifecycle on live data: no per-entry state, no entry count, no age distribution.
3. Hot-key, demand, Zipf and burst visuals: `hot_keys` explicitly unavailable; the workload pipeline is not in the serving path.
4. Prediction with intervals and calibration: no predictor runs.
5. Canary / guardrail / rollback timeline: no wiring; audit store empty; `route_bucket`, watchdog and rollback have no production caller.
6. Distributed invalidation propagation, repair backlog and durability: no cluster events or gauges; durability is a documented limit (issuer restart loses owed invalidations; unpatched).
7. Node failure realism: `NODE_FAILURE` is a logical flag; `Degraded` and `Offline` states are never assigned; cpu/memory per node unavailable in live mode.
8. Latency distribution charts live: only percentiles are exposed.
9. Stale-serving effectiveness stories: no retained experiment exercised SWR / SIE.
10. Multi-tenant views: no per-tenant telemetry or quotas.

## M7. Cross-agent log (details in `coord/r4.md`)

- R1 requests 1-3 answered in M0.1 and M0.3 (provenance vocabularies: stream `measured/derived/inferred/simulated/unavailable` shown as Real / Derived / Derived (inferred) / Simulated / Unavailable, `ui/src/lib/provenance.js:7-13`; Lab tiers `MEASURED_LOOPBACK`, `SIMULATED`, `RETAINED_SIMULATED`, `RETAINED_MEASURED_LOOPBACK`, `DERIVED`, `UNAVAILABLE`, `labProvenance.js:18-30`; REPLAYED and PREDICTED exist in the Lab UX design vocabulary but not in UI code; failure and degraded states in L3/L4/L6, M0.2).
- R2 requests 1-6 answered in M0.1 (rates and cardinalities in M0.1 and M3; histogram, timestamps, Scenario B intervals, per-entry state, controller log).
- R3 requests R4-1 to R4-4 answered in M0.1, M1 rule (4), M0.1 replayability column and M6.
- R5 requests 1-3 answered in M0.2, M0.3, M0.4.
- Conflicts: none with R1-R3 on facts. Open for the lead: D2 (capability flag says no controller while the origin cap controller is live) and the X-04 vocabulary question (ILLUSTRATION as caption versus label), where R4's position is: illustrations need a visible text mark, and the existing six-word vocabulary should remain reserved for data.
