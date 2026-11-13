# R4: PhantomCache system objects, lifecycles, operations and motion opportunities

**Status:** Research (R4), draft complete for lead review. **Date:** 2026-10-30. **Scope:** research only; no source, frontend, test, Cargo or skill changes were made. Everything that proposes something is marked PROPOSAL and is not a decision.
**Companion file:** `r4_lifecycle_and_event_matrices.md` (large tables: motion matrix, candidate experiences, missing telemetry, capability register, capability-to-visual table).
**Coordination:** `coord/r4.md`.

## 0. Method, legend, evidence limits

Tags: **FACT** (R4 opened the file and read the cited line, 2026-10-30), **FACT-SECOND** (taken from a cited repository document and spot-checked by R4 where stated), **INFERENCE**, **PROPOSAL**, **UNVERIFIED**. Line numbers are of the working tree at HEAD `456bb25` plus its uncommitted changes. Nothing was compiled or run (no cargo, no npm, no browser). Passing tests are not used as evidence of correctness.

Implementation status of an object (shared vocabulary of the coordination board): **REAL** (implemented and reached by the shipped gateway binary), **DERIVED** (computed from real values), **SIMULATED** (produced by a model, never live telemetry), **PROPOSED** (design only), **UNAVAILABLE** (cannot be provided; the product says so). Where an object is implemented but nothing in the serving path calls it, the status is **REAL (library only, not in serving path)**.

Frontend column: "stream" = `GET /api/events?schema=2` (a 100 ms interval snapshot, `src/api/server.rs:940-983`; the only live feed the UI consumes, `ui/src/lib/config.js:25`, `ui/src/hooks/useTelemetry.js:6`). "Lab API" = `/api/v1/lab/*` (lab-1; consumed by `ui/src/screens/lab/*`). The UI does **not** consume `/api/v1/diagnostics/events`, `/api/v1/diagnostics/summary`, `/api/v1/intelligence/status` or `/api/v1/intelligence/audit` (grep of `ui/src`, FACT; these routes are declared at `src/api/server.rs:211-214`).

## 1. Object and domain inventory

### 1.1 Compact register (all objects)

| ID | Object | Status | Primary source (FACT) | In stream (schema 2)? | Real-data render today? |
|---|---|---|---|---|---|
| O1 | Client request / response (GET, DELETE) | REAL | `server.rs:1136` (GET), `:1937` (DELETE), routes `:202-209` | aggregate only (`requests_total`) | aggregate yes; per-request no |
| O2 | Gateway process | REAL | `main.rs:200-207` (8 shards), `server.rs:228` | `mode`, `uptime_ms`, `capabilities` | yes |
| O3 | Tenant (namespace + epoch) | REAL | `cache.rs:273,560-636`; `server.rs:1052` | none | no |
| O4 | Cache key and canonical form | REAL | `cache.rs:23-117` | none (diagnostic events carry a salted `key_fp`) | no |
| O5 | Cache entry (`CachedResource` + `KeyState` + `FreshnessMetadata`) | REAL | `cache.rs:120,136`; `freshness.rs:98` | `bytes_resident`, `evictions_total` only; no entry count | aggregate bytes only |
| O6 | Lookup / HIT | REAL | `cache.rs:1238-1290`; counter `:1285` | `cache_hits` | yes (counter) |
| O7 | MISS (leader of a fetch) | REAL | `cache.rs:1347` | `cache_misses` | yes (counter) |
| O8 | STALE served (stale-while-revalidate / stale-if-error) | REAL | `cache.rs:749-822`; `server.rs:1317-1340,1498` | `stale_served_total`, `stale_cap_violations` | yes (counters) |
| O9 | Revalidation (conditional request, 304) | REAL | `cache.rs:1025-1080`; `server.rs:1358-1400,1758-1770` | `revalidations.{started,ok,not_modified,error,in_flight}` | yes (counters) |
| O10 | Bypass (`x-cache-bypass`, authorization) | REAL | `server.rs:1236-1300` | none (`cache_bypasses` only in diagnostics summary) | no |
| O11 | Request coalescer (leader / waiters, `SharedOp`) | REAL | `coalesce.rs:97-176`; `cache.rs:1292-1316` | `coalesced_waiters` (followers only) | counter only |
| O12 | Origin request / response | REAL | `server.rs:2534` (`fetch_origin_raw`) | `origin.{requests_started,requests_completed_ok,in_flight,errors.*,latency_ms}` | yes (aggregates) |
| O13 | Origin concurrency gate (cap, permit) | REAL when `PHANTOM_INTELLIGENCE` is canary/active and a cap is configured | `executor.rs:78-165,217,233` | `origin_cap.{enforcing,cap,in_flight,dispatched,origin_cap_rejections,controller_steps}` | in stream, no dedicated tile (not in `catalog.js`) |
| O14 | Shed (HTTP 503) | REAL | `server.rs:2217-2230`, marker `:2385` | `origin_cap_rejections`, `capacity_rejections`; waiter-limit rejections not in stream | partial |
| O15 | AIMD cap controller | REAL (the only controller in the serving path) | `candidates.rs:135-172`; `executor.rs:134-160` | `origin_cap.cap`, `controller_steps` | numbers only |
| O16 | Cache fill / finalize / generation fence | REAL | `cache.rs:1401-1572` | none (`stale_writes_prevented` only in diagnostics summary) | no |
| O17 | TTL / freshness / expiry | REAL; default TTL literal 60 s | `freshness.rs:139`; `server.rs:1405,1688` (literal `60_000`) | none per entry | no |
| O18 | Expiration reclamation (sweeper) | REAL | `cache.rs:386-458,459` | none (`reclamations_total` only in diagnostics summary) | no |
| O19 | Eviction and policy (SIEVE / LRU / S3-FIFO) | REAL | `eviction.rs:14-35`; `cache.rs:1201-1236` | `evictions_total` | counter |
| O20 | Byte accounting and budget | REAL | `cache.rs:1800-1880` | `bytes_resident`, `byte_budget`, rejections | yes |
| O21 | Invalidation (DELETE exact key) | REAL | `server.rs:1937-2130` | none; diagnostics event `CacheInvalidated` | no |
| O22 | Invalidation scopes tenant / full | REAL in library and cluster wire; **no HTTP route** | `cache.rs:576,613`; `frame.rs:56-60` | none | no |
| O23 | Epochs (tenant, global) | REAL | `cache.rs:273,576-636` | none | no |
| O24 | Cache node (`VirtualNode`) | REAL in cluster mode (local node shares serving cache); **SIMULATED** placeholders otherwise | `node.rs:20-72`; `main.rs:207-217,332` | `nodes[]` (id, state, state_src) | list only |
| O25 | Cluster membership | REAL (static file) | `security/cluster.rs:309-317,557`; `main.rs:263-275` | none (not exposed) | no |
| O26 | Rendezvous routing | PARTIAL: plain HRW (weight constant 1.0), used for DELETE primary selection only | `router.rs:1351-1418`, `capacity_weight = 1.0` at `:1363,1381` | none | no |
| O27 | Node connection, session, pool | REAL | `pool.rs:59-440`; `session.rs` | none | no |
| O28 | Node state (7 variants) | REAL for 5; Offline and Degraded have no writer | `node.rs:10-18`; writers `router.rs:1212-1670,1840,1995,2149-2268` | `nodes[].state`, `state_src` | state text only |
| O29 | Invalidation message (frame, scope, status) | REAL | `frame.rs:22-31,56-100` | none | no |
| O30 | Repair ledger (owed invalidations) | REAL, in memory, non-durable | `router.rs:54-480` | none | no |
| O31 | Anti-entropy (ring, gap tracker, pull reconcile) | REAL, effectiveness weak | `anti_entropy.rs:9-480`; `router.rs:1000-1044,2128` | none (`anti_entropy_*` counters in diagnostics summary only) | no |
| O32 | Failure detection and recovery | PARTIAL (side effect of failed sends and repair) | `router.rs:1840,1995,2149-2268` | node `state` only | state text only |
| O33 | `NODE_FAILURE` control event | SIMULATED logical flag on node id 2 | `server.rs:1089-1092` | node `state` (`state_src` inferred / simulated) | would render as real failure unless labelled |
| O34 | Workload observation (raw ring, `key_hash`) | REAL internal (10 000-entry ring), not exposed | `telemetry.rs:7-139,413-498`; `main.rs:254` | latency percentiles only | percentiles yes |
| O35 | Hot keys | **UNAVAILABLE** (stream states it) | `server.rs:766` `"hot_keys":{"src":"unavailable","reason":"not_implemented"}` | explicit unavailable | no (correctly) |
| O36 | Workload signals (`SignalWindow`, `WorkloadPipeline`, detectors) | REAL (library only, not in serving path) | `intelligence/mod.rs:98`, `signals.rs:382` | none | no |
| O37 | Prediction (forecaster, emergence predictor) | REAL (library only); PREDICTED output does not exist at runtime | `candidates.rs:175-275`, `mod.rs:174-180` | none | no |
| O38 | Policy candidates | mixed (see 1.3 O38) | `candidates.rs:29-135` | none | no |
| O39 | Shadow evaluator (LRU / SIEVE / S3-FIFO / TinyLFU shadows) | REAL (library only) | `shadow.rs:418` | none | no |
| O40 | Intelligence mode (Off, Observe, Advisory, Canary, Active) | REAL flag; only effect: origin cap enforcement | `intelligence/mod.rs:24`; `executor.rs:217`; `main.rs:459-480` | `/api/v1/intelligence/status` (not consumed by UI) | no |
| O41 | Canary bucket routing (Holdback / Candidate / Control) | REAL function, **no caller** | `executor.rs:270` | none | no |
| O42 | Guardrail, rollback, watchdog | REAL (library only): `revert_to_safe`, `check_watchdog` have no production caller | `executor.rs:285,390` | revert count via status endpoint | no |
| O43 | Decision record and audit store | REAL types; store empty at runtime (nothing appends) | `mod.rs:150`, `audit.rs:17`, `main.rs:480` | `/api/v1/intelligence/audit` (not consumed) | no |
| O44 | Legacy PIE stub | REAL code, `pie: None` | `main.rs:433`, `control/pie.rs` | `capabilities.controller = "none"` | placeholder |
| O45 | Experiment / run kinds (`t1_matrix`, `scenario_b`) | REAL when `PHANTOM_LAB_ENABLED=true` | `lab_api.rs:1025-1033,1579-1870` | Lab API | yes |
| O46 | Experimental arm | REAL (Scenario B: BASE, STATIC-CAP, AIMD-CAP); SIMULATED (T1: BL-0, BL-1, BL-4, BL-5, BL-6, BL-12b, PIE-*) | `scenario_b.rs:26-50`; `t1_sim.rs:169`; `lab-results/m6_sealed_*/cells.csv` | Lab API / retained | yes (aggregates) |
| O47 | Run (state machine and lab-1 events) | REAL | `lab_api.rs:641-760`; contract `docs/design/lab_api_contract.md` s3 | `state`, `progress`, `result`, `done`, `error` | yes |
| O48 | Verdict / label | DERIVED (mechanical rules) | `scenario_b.rs:552-572,867`; `lab/eval.rs`; M6 `verdict.json` | Lab API | yes |
| O49 | Retained evidence | REAL (files) | `lab-results/` ; `lab_api.rs:1320-1470` | `/evidence`, `/evidence/{id}` | yes |
| O50 | Legacy simulation snapshot | SIMULATED | `server.rs:2948-3098`; only when `PHANTOM_SIMULATION=true` | schema 1 stream (deprecated), `backend_load_ratio`, `avoided_compute_dollars`, `cost` labelled simulated in v2 | must stay labelled |
| O51 | Diagnostic event ring | REAL, lossy | `diagnostics.rs:180-227`, capacity 512 at `cache.rs:302` | `/api/v1/diagnostics/events` (last 100, no cursor; UI does not consume) | no |
| O52 | Provenance label | REAL (contract) | `server.rs:498-780`; `ui/src/lib/provenance.js` | every schema 2 field | yes |

Counts (52 objects): REAL (reachable) 31; REAL-library-only (no serving-path caller) 8 (O36, O37, O39, O41, O42, part of O38, O43 types, O44 code); PARTIAL 3 (O26, O28 partly, O32); SIMULATED 3 (O33, O46-T1, O50 and placeholder nodes in O24); UNAVAILABLE 1 (O35, plus UI-unavailable items); DERIVED 1 (O48); PROPOSED objects are not in the register (see matrices file section M3). INFERENCE: counts are approximate because some rows mix statuses.

### 1.2 Domain model: how the objects relate (FACT unless marked)

```
client request ──► gateway ──► key (tenant, path, query) ──► shard (8, power of two) ──► KeyState
                                                                   │
              KeyState { generation, active_op_ids, entry?, freshness?, epochs }
                                                                   │
  lookup ──► HIT (fresh, epochs match) ───────────────────────────► response  X-Cache-Status: HIT
         ├─► STALE eligible (expired, inside SWR/SIE window, under hard cap) ► response STALE + background revalidation
         ├─► in-flight fetch exists ──► WAIT (follower)  ───────────► response COALESCED (or STALE)
         └─► nothing usable ──► INITIATE (leader) ──► [origin gate permit] ──► origin ──► finalize_fetch (fence) ──► waiters
                                                                   │                             │
                                         reject: waiter limit / capacity / cap  ──► 503         fill entry ► bytes ► eviction
DELETE ──► local delete (generation++) ──► [cluster] primary ack (150 ms) ──► async fan-out ──► repair ledger ──► anti-entropy
```

### 1.3 Per-object detail

Each block gives: meaning, source, status, states, transitions, relations, exposure, frontend renderability.

**O1 Client request / response.** Meaning: a GET for `/api/v1/cache/*key` (plus optional query) or DELETE. FACT: handler `get_cache_handler` `server.rs:1136`; authentication first (`:1148-1210`); tenant from verified token, or from `x-tenant-id` only when security is off (`:1213-1226`); `requests_total` is incremented once the key is valid (`:1263`). Response outcomes (FACT, `X-Cache-Status` values at `:1282,1498,1556,1600,1894`): `HIT`, `MISS`, `STALE`, `COALESCED`, `BYPASS`, `BYPASS-AUTH`; errors carry `X-Cache-Error` (+ `X-Cache-Error-Reason`, `Retry-After: 1`) (`:2174-2260`). Exposure: per-request status is visible only to the client in headers; the gateway emits no per-request event (no request or trace id; FACT-SECOND `04_lab_ux_design.md` 1.2.7, consistent with R4 reading). The Scenario B load generator records each request (`client_requests.csv`, columns `idx,key,scheduled_ms,send_ms,done_ms,send_lag_ms,latency_ms,service_latency_ms,status,x_cache_status,x_cache_error,bytes,error,in_analysis_window`; FACT, `lab-results/scenario_b/20261030T003932Z/runs/rep01_AIMD-CAP/client_requests.csv`). Frontend: aggregate counters only; per-request views are possible only as REPLAYED views of retained Scenario B CSVs, which the Lab API does not serve (only artifact names/sizes/downloads, see O49).

**O4 Cache key.** `CacheKey{tenant_id, path, query}` (`cache.rs:23-27`); tenant defaults to `__default__`; tenant id 1..128 bytes of `[A-Za-z0-9._-]` (`:38-60`); canonical string `to_canonical_string` (`:96`) is what travels on the wire (4096-byte cap `:19`). The query string is part of the key and is not normalised (INFERENCE from `:30-36`: raw query is stored as given), so `?a=1&b=2` and `?b=2&a=1` are different entries (FACT-SECOND `limitations.md`: DELETE of one query variant leaves others). Diagnostic events expose `key_fp` = salted-hash fingerprint (`diagnostics.rs:119-131`; salt random per process), never the key. The telemetry observation carries `key_hash` = unsalted FNV-1a of the canonical key (`server.rs:1262`, `telemetry.rs:14`) but that ring is not exposed. Privacy: any future key-level event must use the salted fingerprint; the unsalted `key_hash` is guessable for short keys (INFERENCE).

**O5 Cache entry.** `KeyState` holds `entry: Option<CachedResource{status, content_type, body, etag, created_at_ms, ttl_ms}>`, `generation`, `active_op_ids`, epochs, `freshness: Option<FreshnessMetadata{must_revalidate, stale_while_revalidate_ms, stale_if_error_ms, last_modified, date_ms, initial_age_ms}>`, `visited`, `access_count`, revalidation backoff (`cache.rs:120-149`). States: see lifecycle L1. Exposure: none per entry; no entry count and no per-entry age distribution exist in schema 2 (FACT: `build_schema_v2_payload` has `bytes_resident` but no entry count). Frontend: can render resident bytes versus budget (`CacheMemory.jsx`), not individual entries.

**O6/O7 HIT and MISS.** HIT: fresh entry and tenant/global epochs match, `begin_fetch_atomic` increments `cache_hits` (`cache.rs:1285`) and calls `eviction.on_hit`. MISS: `cache_misses` counts only fetch **leaders** (`:1347`); followers are counted in `coalesced_waiters` instead (`:1311`). A stale-served request increments neither `cache_hits` nor `cache_misses` (it returns earlier at `server.rs:1317-1400`), and a bypass increments neither. Consequence (FACT, derived from those lines): the schema-2 `hit_ratio_cumulative = hits/(hits+misses)` (`server.rs:542-556`) excludes coalesced, stale and bypass requests; `requests_total` is therefore not equal to `hits + misses`. INFERENCE: any "request outcome mix" chart must compute shares against `requests_total` and show the unattributed remainder, not against `hits+misses`.

**O8 STALE.** `check_stale_serving` (`cache.rs:749`): an entry that is already expired, epochs matching, not `must_revalidate`, within `min(window, hard cap)` after expiry, is `Eligible`; beyond `lifetime + hard_cap` is `CapExceeded`. Served with `X-Cache-Status: STALE` and `Age`, and triggers a bounded background revalidation (`server.rs:1344-1470`, slot reserved atomically via `try_reserve_background_slot`, max `max_background_refresh` = 128 default `cache.rs:337`). Also served on origin error (stale-if-error) through the leader path (`server.rs:1778-1790,1826-1840`). `stale_cap_violations` must stay 0 (`record_stale_served` `cache.rs:824`). SIMULATED vs REAL: Scenario B did not exercise stale serving (`scenario_b.rs:567` STALE_SERVING_NOTE), so retained evidence contains no real stale-serving series.

**O9 Revalidation.** A leader whose key has a resident (expired) entry sends the entry's own validators (`stored_validators`, `server.rs:2444`); 304 goes through `complete_not_modified` (`cache.rs:1025`) with outcomes Refreshed / NotApplied / ValidatorMismatch / Uncacheable / NoResident (`:181-192`). Counters in stream: started, ok, not_modified, error, in_flight (`server.rs:731-737` region). Not in stream: `revalidation_conditional_sent`, `revalidation_304_received`, `revalidation_not_applied`, `revalidation_not_initiated` (`diagnostics.rs:60-80`: "deliberately NOT in the snapshot / schema").

**O11 Request coalescer.** See section 5 (deep dive). Legacy `Coalescer` (`coalesce.rs:19-52`) is preserved only for old tests and `VirtualNode`; it is not in the HTTP serving path (grep: constructed at `main.rs:209` as `_coalescer`, unused).

**O12 Origin request.** `fetch_origin_raw` (`server.rs:2534`), validators from the stored entry only, no client conditional or range headers forwarded (`:1186-1196`, `:2459`). `OriginMetrics` (`intel/origin_metrics.rs`) counts started/ok/in_flight/errors and a 15-bucket latency histogram (bucket edges `LATENCY_BUCKETS_MS`) giving p50/p95/p99 as bucket values (`server.rs:588-612`). Cap rejections are not counted as origin dispatches (`server.rs:1648`).

**O13 Origin gate.** `OriginGate{cap, in_flight, rejections, dispatched, controller_steps}` (`executor.rs:78-90`); `acquire_origin_permit` returns `OriginCapSaturated` when `in_flight >= cap` and enforcing (`:233-250`); enforcement only in Canary/Active with a configured cap (`:217-220`); in Off/Observe/Advisory it counts but never rejects. A permit is taken by the single-flight leader (and background revalidation, bypass), never by coalesced followers (`server.rs:1645-1648`). Snapshot read per stream frame (`server.rs:639-662`). `metrics.origin_cap` is in schema 2 but absent from `ui/src/lib/catalog.js`; the Overview shows it only through the generic "other reported fields" list (INFERENCE from `knownMetricPaths`, `catalog.js:62-69`). A purpose-built origin-protection visual is therefore a new UI element on an existing field.

**O15 AIMD controller.** `step(p99, error_rate)`: p99 > ceiling or error rate > 5% multiplies the limit by 0.70 (floor at min), otherwise +1 up to max (clamped to 256) (`candidates.rs:155-172`). It steps once per observation window of `window_size` origin responses (default 32) (`executor.rs:134-160`). The "p99" is the nearest-rank percentile of that window, which for window 30 equals the window maximum (FACT-SECOND `scenario_b.rs:559` AIMD_EFFECTIVE_CAP_NOTE). Retained evidence shows the cap falling from 32 to 12 within about 1 s and staying (`gateway_snapshots.jsonl` rep01_AIMD-CAP, FACT: cap 32 at t=0.02 s, 15 at 0.27 s, 13 at 0.53 s, 12 at 0.80 s). Honest label for any control visual: "origin concurrency controller", not "adaptive intelligence".

**O16 Fill / finalize / fence.** `finalize_fetch` (`cache.rs:1401`): (1) detaches the in-flight entry and extracts waiters; (2) stores the resource only when `state.generation == op_gen` and the tenant/global epochs still match, otherwise increments `stale_writes_prevented` (`:1448-1475`); (3) enforces byte budget/oversize with up to 32 evictions of other keys (`:1491-1530`); (4) installs pending freshness metadata atomically with the entry; (5) reclaims a placeholder with no entry and no active ops; (6) delivers the outcome to every waiter via `WaiterBatch` (also on Drop; leader abort delivers `Aborted`, `coalesce.rs:119-168`). Known documented gap L3: a request arriving after an invalidation can still join a pre-invalidation fetch and receive its body (`cache.rs:1292-1298` comment); the body is kept out of the cache. Exposure: only the aggregate counters `stale_writes_prevented`, `stale_ops_ignored` in diagnostics summary.

**O17 TTL / freshness.** `evaluate_response_freshness` (`freshness.rs:139`) applies RFC 9111 subset; default TTL when the origin gives no lifetime = the literal `60_000` passed at `server.rs:1405` and `:1688`; `PHANTOM_DEFAULT_TTL_MS` is validated but not read at runtime (CLAUDE.md s5, FACT-SECOND, consistent with the two literals R4 saw). There is no gateway ceiling on origin `max-age` (FACT-SECOND a2 matrix row 6a).

**O19 Eviction.** `ShardEvictionPolicy` trait (`eviction.rs:14-35`) with `FaithfulSieve` (indexed linked nodes), LRU, S3-FIFO implementations. Victim selection excludes keys with active ops or in-flight fetches (`cache.rs:1214-1220`). Policy default Sieve (`cache.rs:327`); changed at runtime only through the executor (`executor.rs:360,392`), not by any HTTP route. Discrepancy: CLAUDE.md s6 still says "evictors named SIEVE/S3-FIFO/LRU are scans" while CLAUDE.md s5 and `eviction.rs` describe O(1) structures; R4 read the trait and the SIEVE node structure (`eviction.rs:44-60`) and did not audit complexity of all three, so the s6 sentence is UNVERIFIED-as-outdated.

**O21/O22/O23 Invalidation and epochs.** HTTP exposes exactly one invalidation: `DELETE /api/v1/cache/*key` (exact key) (`server.rs:202-209`). It deletes locally first (generation++ , entry dropped, in-flight detached: `cache.rs:1586-1640`, `server.rs:1991`), then, with a router, `invalidate_key_across_cluster` (ExactKey). Tenant invalidation (`try_invalidate_tenant`, `cache.rs:576`, tenant epoch table capped at `MAX_TENANT_EPOCHS=10_000` with global-epoch fallback `:581-593`) and full invalidation (`:613`) exist in the library and on the cluster wire but have no HTTP route (FACT-SECOND a2 row 3b, consistent with R4's reading of routes). They are used inside the repair ledger's escalation. Stale entries from an epoch bump are purged lazily at lookup (`cache.rs:1259-1275`) or by the sweeper.

**O24/O28 Node and node state.** `NodeState {Joining, Active, Degraded, Suspect, Failed, Offline, Recovering}` (`node.rs:10-18`). Production writers (FACT, grep of `set_state`/`compare_and_set_state`): `Joining` initial (`node.rs:41`) → `Active` when the TCP service starts (`node.rs:198`, `main.rs:217,366`); `Suspect` on failed sends or timeouts (`router.rs:1212-1670,1840,1874`); `Suspect → Recovering` after a repair drain (`:1995`, `:2156-2157`); `Recovering → Active` after reconcile when no older debt remains (`:2149,2233-2268`); `Failed` only through the operator control event (`server.rs:1091`); a control event other than FLASH_CROWD/NODE_FAILURE sets all nodes Active (`:1097`). `Degraded` and `Offline` have **no writer**; they appear only in guards. Exposure: `nodes[]` = `{node_id, state, state_src, cpu, memory}` in the stream, only when a router exists or simulation is on (`server.rs:660-690`); `state_src` is `inferred` with a router and `simulated` otherwise; cpu/memory `unavailable` in live mode. Cluster mode: remote peers are address-only descriptors; the local node shares the serving cache (`main.rs:332`, FACT; and the startup banner claim at `main.rs:368` is out of date per CLAUDE.md s6). Frontend: `Placeholder.jsx` renders `nodeViews(payload)` as a table when present (`Placeholder.jsx:29-62`).

**O26 Routing.** `route_key` selects the node with the highest `hash(key, node_id)` among nodes not in {Failed, Offline, Suspect, Recovering}, falling back to Suspect/Recovering when none (`router.rs:1351-1392`). `capacity_weight` is the constant `1.0` at `:1363` and `:1381`, so it is plain rendezvous hashing, not weighted. The hash is `std::DefaultHasher` (`:1343-1349`), not guaranteed stable across toolchains (INFERENCE, a2 row 5b). `route_key` and `get_replicas` have a production caller only in the DELETE path (`router.rs:1814`); `forward_key_over_tcp` (`router.rs:1420`) has no production caller (FACT-SECOND a2 5a; R4 grep found no call in `server.rs`). GET is served from the local cache only.

**O29/O30/O31 Invalidation message, ledger, anti-entropy.** Frame kinds: Get, Invalidate, Heartbeat, AntiEntropy (`frame.rs:22-31`, FACT-SECOND). Invalidation scope ExactKey / Tenant / FullCache; status SuccessEvicted / SuccessNotPresent / IgnoredObsolete / RejectedRateLimit / InternalError (`frame.rs:56-100`). Delivery: primary synchronous leg with a 150 ms timeout, then asynchronous fan-out, per-peer queue 256 and in-flight 1 (`router.rs:506-509`); unacknowledged work is owed in the `RepairLedger` (max 1024 entries / 256 KiB per peer; tenant-collapse then full flush: `router.rs:54-60`); a pull-based anti-entropy worker runs every 250 ms in cluster configurations (`main.rs:375,383`, `router.rs:1000`) with a replay ring of 4 096 entries / 4 MiB (`anti_entropy.rs:9-12`). Limits (FACT-SECOND `limitations.md` "Cluster invalidation durability", a2 rows 3c/4d): ledger, ring and sequence trackers are in process memory; a crash or restart of the **issuer** loses owed invalidations and the peers cannot detect it; there is no shutdown drain (`main.rs:509-514`); the reproduced defect (`m3_durability/reproduction_report.md`) is unpatched in tree (patch not applied). No consistency statement may be drawn as an animation guarantee ("every node purges within X").

**O33 NODE_FAILURE.** `POST /api/control {"event":"NODE_FAILURE"}` sets node id 2's state to Failed (`server.rs:1089-1092`); this changes routing decisions in the same process because the same `Arc<VirtualNode>` is used by the router, without any network fault. It is a logical flag, SIMULATED. The Lab UX design already requires a `logical_flag` fault layer that can only show a SIMULATED chip (`04_lab_ux_design.md` line 17, FACT-SECOND).

**O35/O36/O37 Workload intelligence.** FACT: the stream states `hot_keys` unavailable. `WorkloadPipeline` (rate EWMA, Space-Saving heavy hitters, Zipf alpha, working-set sampler, burst CUSUM, hot-key emergence) produces a `SignalWindow` with `FeatureValue{value, provenance, sample_count, window_duration_ms}` (`mod.rs:60-110`, `signals.rs:382-567`); grep shows no non-test caller outside `src/intelligence/`, so there is no live workload analysis (consistent with FACT-SECOND claims ledger C8). `CountMinSketch` (`intel/sketch.rs`) and `WorkloadProfile` (`intel/workload.rs`) are unused data structures. Forecasters (`PredictiveRateForecaster` with MASE `candidates.rs:175-250`, `HotKeyEmergencePredictor`) exist as library candidates with no consumer. Therefore **no hot-key, demand, Zipf or prediction visual can be drawn from the live gateway today**.

**O38 Policy candidates** (`candidates.rs`): (a) shadow-selected eviction (library only); (b) frequency admission filter (used by the T1 simulator only, `t1_sim.rs` header); (c) origin-aware stale controller (T1 only); (d) AIMD origin concurrency controller (live, O15); predictive/decision stump (library only). Only (d) reaches the serving path.

**O40–O44 Adaptive control machinery.** Modes exist as a flag (`mod.rs:24`, wired from `PHANTOM_INTELLIGENCE`, `main.rs:459-480`). `route_bucket(key_hash)` (hash → Holdback / Candidate / Control) has no caller outside its definition (R4 grep). `check_watchdog`, `heartbeat_at`, `safe_apply_policy` have no production caller; `apply_policy` is called by the T1 simulator only (`t1_sim.rs:1067`). `PieDecisionEngine` (`engine.rs:36`) is referenced only inside `src/intelligence/` (matches STATUS.md M5 row). `AuditStore::new(100)` is created in `main` and installed (`main.rs:480`) but nothing calls `append`, so `/api/v1/intelligence/audit` returns an empty list at runtime (INFERENCE from grep; 100-record capacity FACT). `capabilities.controller` is `"none"` because `pie` is `None` (`server.rs:511-515`, `main.rs:433`) even though the origin cap controller is running, so the Intelligence placeholder page reads "controller: none" while `metrics.origin_cap` is live. Discrepancy to be resolved by the lead (see section 6).

**O45–O49 Lab objects.** T1: `t1_matrix` run = scenarios (`M6_SCENARIOS` 8: O-DEG, O-BRN, O-SLW, O-CO, O-RT, O-FL, O-HET, O-GRD) × workload families × arms × seeds → `cells.csv` (36 columns, `t1_sim.rs:487`); drives the real cache and executor code on a virtual clock, SIMULATED. Sealed M6: 12 000 cells, 10 arms, verdict H1 NOT_SUPPORTED, H2/H4 NOT_TESTABLE, H3/H5/H6 UNVERIFIED (`lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`, FACT). T2: Scenario B = reps × arms (BASE, STATIC-CAP, AIMD-CAP), interleaved, same seed per rep, phases warmup/healthy/degraded/recovery, scripted origin on loopback, real gateway child process, open-loop load, snapshot polling; tier `MEASURED_LOOPBACK`; label VALID/other by mechanical rules (`scenario_b.rs:557-572,867`). Retained: `lab-results/scenario_b/20261030T003932Z/` with 30 runs, each with `client_requests.csv`, `origin_requests.csv`, `gateway_snapshots.jsonl`, `run_metrics.json`, `gateway.log`. Run state machine: queued → running → cancelling → completed | failed | cancelled | interrupted (`labProvenance.js:46-54`, `lab_api.rs:680-760`); SSE events `state`, `progress {done,total,[arm,rep,phase,gateway_pid],tier}`, `result`, `done`, `error` (`lab_api.rs:647-752,1807,1845`); events persisted in `events.jsonl` and resumable with `Last-Event-ID`. Interpretation is bound by `docs/demo/scenario_b_interpretation_notice.md` (Not established: adaptive-control merit, origin-p99 benefits, stale-serving effectiveness, served-client latency improvement).

## 2. State-transition and lifecycle maps

### L1 Cache key state / entry lifecycle (FACT from `cache.rs`)

```
                       begin_fetch_atomic (miss, capacity ok)
 ABSENT ─────────────────────────────────────────────────────► PLACEHOLDER
 (no KeyState)                                                  (KeyState, entry=None, active_op_ids={op})
    ▲                                                                │ finalize Success + fence ok + budget ok
    │ reclaim (no entry, no ops)                                     ▼
    │ delete (no active ops) / evict / sweeper                     FRESH ◄────────────────────────────┐
    │                                                                │ now >= created+ttl               │ 304 Refreshed (refresh_entry_lifetime)
    │                                                                ▼                                 │ or new 200 fill
    │                                                             EXPIRED (still resident)──────────────┤
    │                                         ┌──────────────────────┴──────────────────────┐          │
    │                                         │ check_stale_serving                         │          │
    │                                         ▼                                             ▼          │
    │                              STALE-ELIGIBLE (SWR/SIE window ≤ hard cap,      NOT SERVEABLE          │
    │                               not must-revalidate): served STALE +           (window 0, must-revalidate,
    │                               background revalidation (single-flight)        CapExceeded, or errored) ──► next GET = leader, conditional fetch
    │                                                                                                      
    │  epoch bump (tenant/global) or DELETE: entry dropped, generation++ ; in-flight fetch's later Success is discarded (stale_writes_prevented)
    └─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
```
Transition triggers: time (ttl, window, cap), lookup, fill/304, DELETE, epoch bump, eviction pressure (budget or entry cap), sweeper tick (`sweep_lazy_reclamation`, default every 100 ms, 32 candidates per shard: `cache.rs:337-340`). Retention subtlety (FACT `cache.rs:411-429`): an expired entry is retained until `ttl + min(max(swr, sie), hard cap)`; so "EXPIRED" and "reclaimable" are different moments. No event is emitted at any of these transitions; only aggregate counters.

### L2 Single-flight fetch (coalescing) lifecycle (FACT `coalesce.rs`, `cache.rs`)

```
 (no op) ──first miss──► INITIATED (SharedOp{op_id, op_gen}, leader registered as first waiter)
   │                         │ other callers: WAIT (followers push oneshot senders; count coalesced_waiters;
   │                         │   > max_waiters_per_key(1000) ► 503 WaiterLimitExceeded, waiter_limit_rejections)
   │                         ▼
   │                  DISPATCHED (leader task: origin permit → origin request)  ── no permit ► Error → 503 for leader AND all followers
   │                         ▼
   │                  FINALIZING (PHASE_PENDING→FINALIZING→COMPLETED, CAS)
   │                         ▼
   └──────────── COMPLETED: Success(store if fenced) | Uncacheable | Error | Aborted (guard dropped) → WaiterBatch delivers to ALL waiters
```
Panic/drop safety: a dropped guard in PENDING delivers `Aborted`; in FINALIZING counts `finalization_panics` and cleans up (`coalesce.rs:179-310`).

### L3 Request outcome (per request) (FACT)

```
 GET ─► auth(401/403) ─► key validation(400) ─► requests_total++
   ├─ bypass header / Authorization (RFC 9111) ─► BYPASS / BYPASS-AUTH (permit → origin, not stored)
   ├─ check_stale_serving Eligible ─► STALE (+ background revalidation if due & slot)
   └─ begin_fetch_atomic:
        Hit ─► HIT (or 304 to client conditional, still X-Cache-Status HIT)
        Wait ─► COALESCED (or STALE if the delivered resource is already expired) | error
        Initiate ─► MISS (or STALE if stale-if-error served) | error (UpstreamStatus, Timeout, Aborted, 503 cap)
        Rejected ─► 503 CapacityExceeded / WaiterLimitExceeded
```

### L4 Node state (FACT, writers listed in O28)

```
 Joining ─start_tcp_service─► Active ─failed send/timeout─► Suspect ─repair drain─► Recovering ─reconcile + no older debt─► Active
                               │                              ▲  │                         └─new failure─► Suspect
                               └─control NODE_FAILURE (node 2)► Failed ─control "other event"─► Active (all nodes)
 Degraded, Offline: declared, never assigned.   Routing: Suspect/Recovering/Failed/Offline excluded in pass 1 (fallback only if no other node).
```
Recovery rule: promotion to Active requires no debt recorded at or before the node's recovery ticket in the repair ledger (`router.rs:2133-2150`); a Recovering node refuses cluster Get hits (FACT-SECOND `node.rs:436`).

### L5 Invalidation propagation lifecycle (FACT-SECOND where marked; local part FACT)

```
 DELETE ─► [1] local purge (generation++)  (server.rs:1991)           ◄─ linearizable for THIS gateway only
        ─► [2] route_key → primary ─ frame ─ 150 ms ─► ack?  yes ► OK / no ► primary Suspect ► acting primary (replica 1) ► ...
        ─► [3] async fan-out to remaining peers (queue 256, 1 in flight)  ► ack | no ack ► RepairLedger.record(peer, scope, key)
        ─► [4] repair worker + pull reconcile every 250 ms: re-deliver; overflow ► tenant flush ► full flush
        ✗ issuer restart/crash ► ledger, ring, sequence state lost, peers cannot tell  (M3 durability limit, unpatched)
```
HTTP result mapping: 204 on success (+ `X-Cache-Warning: PrimaryFailover(id)` if acting primary), 503 `PrimaryRateLimited`, 500 on internal/obsolete/reject, 504 on timeout/unreachable (`server.rs:2009-2085`).

### L6 Lab run lifecycle (FACT, `labProvenance.js:46-54`, `lab_api.rs`)

```
 queued ─► running ─► completed | failed | cancelled | interrupted(gateway restart)
              └─► cancelling ─► cancelled
 events: state(queued) … progress*(done,total[,arm,rep,phase,gateway_pid],tier) … result … state(terminal) … done(exit_code)
```

### L7 Adaptive-control lifecycle (what exists vs designed)

Designed (library / spec): observe → detect → predict → evaluate (shadow) → canary (Holdback/Candidate/Control buckets) → guardrail check → commit or rollback (`Verdict {Commit, Rollback, NoAction, Fault}`, `mod.rs:~140`) → audit record. **Implemented in the serving path:** a single loop: origin responses → window of 32 → p99/error rate → AIMD step → `cap` update (O15). Everything else is library-only (see O36–O44). Any "observe/evaluate/canary/rollback" timeline for a live gateway would be PROPOSED, not drawn from data.

## 3. Operations and flows with implementation status

| # | Flow | Steps | Status per step |
|---|---|---|---|
| F1 | Request → gateway → lookup | HTTP accept; auth; tenant resolution; key validation; shard select; epoch read; entry lookup | REAL all steps (`server.rs:1148-1263`, `cache.rs:544-560`). Data exposed: counter only |
| F2 | Hit → response | `eviction.on_hit`, `cache_hits++`, `Age`, optional 304 to client | REAL. Event: diagnostics `CacheHit` (lossy ring) |
| F3 | Miss → origin fetch | `Initiate`, leader registered, spawn task, permit, request with stored validators, freshness evaluation, `OriginFetchSuccess/Non200/Failed` event | REAL. Origin metrics and diagnostics events exist |
| F4 | Concurrent requests → coalescing | followers `Wait`; one origin request; shared result to all | REAL. Exposed: `coalesced_waiters` only. No per-fetch fan-in, no join event, no "origin requests avoided" counter |
| F5 | Origin response → fill → waiters | fence check, byte budget, evictions, entry install, waiter delivery | REAL. Fence outcome counted outside the stream |
| F6 | Entry → freshness → expiration | fresh → expired → stale-eligible → not serveable → reclaim | REAL. No transition events; sweeper tick every 100 ms |
| F7 | Stale serve → background revalidation → 304/200 | slot reserved, single-flight leader, conditional fetch | REAL. Counters in stream |
| F8 | Origin protection | permit acquire; rejection → 503 with `Retry-After: 1`; AIMD step per 32 responses | REAL only with `PHANTOM_INTELLIGENCE=canary|active` and a configured cap |
| F9 | Invalidation (single node) | DELETE → local delete, generation bump, in-flight detached | REAL (exact-key only over HTTP) |
| F10 | Invalidation (cluster) | primary leg, failover, async fan-out, ledger, anti-entropy | PARTIAL: REAL for DELETE; loopback/experimental (FACT-SECOND `main.rs` banner `[EXPERIMENTAL]`); not durable across issuer restart; GET not routed |
| F11 | Node failure → detection → recovery | failed send → Suspect → repair → Recovering → Active; NODE_FAILURE flag | Detection REAL but is a side effect of invalidation traffic (idle cluster learns nothing, FACT-SECOND a2 4a, consistent with `pool.rs` not calling `set_state`); NODE_FAILURE SIMULATED |
| F12 | Workload observation → analysis → policy evaluation | observation ring (REAL) → WorkloadPipeline → SignalWindow → shadow/engine → decision → audit | Ring REAL; every later step library-only; T1 simulator and unit tests drive them |
| F13 | Experiment config → execution → results → retained evidence | `POST /runs` (validated spec) → child gateway/simulator → events.jsonl → artifacts → `/evidence` | REAL (Lab API). Tier and label rules in code |
| F14 | GET in cluster | any node serves from its own cache; cache fills from origin independently | REAL (by design: ADR-003 rejects coordinated reads). Peers learn only "key is gone", never a value |

## 4. Object-operation-purpose matrix and screen-to-object mapping

### 4.1 Object × operation × purpose (user understanding)

| Object | Operation | Purpose for the user | Data source today |
|---|---|---|---|
| Request | arrives, resolved to outcome | "what happened to my request?" | response headers (client side); aggregates |
| Coalescer | followers join a fetch | "many callers, one origin call" | `coalesced_waiters` aggregate |
| Entry | ages, expires, refreshes | "is what I serve still valid?" | none per entry; counters for stale/revalidation |
| Origin gate | admits/sheds | "is the origin protected, at what cost?" | `origin_cap.*` series (100 ms), Scenario B snapshots |
| Invalidation | spreads to peers | "did the purge reach everyone?" | no cluster telemetry; HTTP status of DELETE only |
| Node | changes state | "which nodes are healthy?" | `nodes[]` state text, labelled inferred/simulated |
| Experiment arm | runs, compared | "did the controller help?" | Lab API summary, retained CSV/JSON |
| Verdict | issued | "what is established, what is not?" | `verdict.json`, `summary.json` |

### 4.2 Screen → objects (what each existing UI screen presents, from which data, with which provenance)

| Screen (`routes.js`) | Objects presented | Data / provenance | Source of truth |
|---|---|---|---|
| Overview | O2 gateway, capabilities; grouped counters (O6–O9, O12, O19, O20); "other reported fields" incl. `origin_cap`, `nodes` | stream schema 2; each value carries `src` measured → shown "Real", derived, inferred ("Derived (inferred)"), simulated, unavailable (`provenance.js:7-13`) | `Overview.jsx`, `catalog.js` |
| Traffic & latency | O1 aggregate, O6/O7/O11 counters, GET latency percentiles, 3 sparklines kept in the page for the session | measured counters; derived rates/ratios; page-local history explicitly "not gateway history" | `Traffic.jsx` |
| Cache & memory | O20 bytes vs budget, O19 evictions, rejections, RSS | measured, logical accounting basis; RSS Linux only | `CacheMemory.jsx` |
| Freshness | O8 stale counters, O9 revalidation counters, stale-cap violation (critical when ≠ 0) | measured counters | `Freshness.jsx` |
| Origin | O12 started/ok/in_flight, errors by class, latency p50/p95/p99 | measured / derived | `Origin.jsx` |
| Phantom Lab → About / Start a run / Runs / Evidence | O45–O49: capabilities, run forms, run list and detail with SSE progress, artifacts table, M6 detail (effect charts, verdict chips), Scenario B result (paired table, shed/latency/tradeoff charts) | Lab API; tiers MEASURED_LOOPBACK (shown "Real (loopback, one host)"), SIMULATED, RETAINED_*, DERIVED, UNAVAILABLE (`labProvenance.js:18-30`); debug-build qualifier added | `ui/src/screens/lab/*`, `labView.js` |
| Simulation | static explainer; a **non-live** coalescing illustration (3 dots → 1 line → origin square, "no numbers") | none; always bannered "Simulated" | `Simulation.jsx` |
| Cluster (placeholder) | O24/O28 node table when `nodes[]` present; "what this page would show" list | `nodes[]`, `capabilities.cluster`; no health colours | `Placeholder.jsx` |
| Intelligence (placeholder) | capability text only (O40/O44) | `capabilities.controller` = `none` | `Placeholder.jsx` |

INFERENCE: the Cluster and Intelligence placeholders correctly refuse to show invented values; the design-direction doc (`phantomcache_design_direction_v2.md` s5) already specifies that request flow and node relationships are illustrations with a visible mark; R4 agrees and adds that the Simulation page's existing coalescing graphic is the only existing flow illustration.

## 5. Request coalescing deep dive (special attention)

**What the code records (FACT).**
1. `coalesced_waiters` (`diagnostics.rs:33`) is incremented exactly once per follower that joins an existing in-flight op (`cache.rs:1309-1312`). The leader is **not** counted; the leader is a waiter of its own `SharedOp` (`server.rs:1629-1632`) but never increments the counter.
2. `cache_misses` is incremented only when a new op is created (`cache.rs:1347`), i.e. once per origin-fetch leader (including background revalidation leaders, which also call `begin_fetch_atomic`: `server.rs:1358`; note: a background revalidation that finds an entry non-hit and no op becomes a leader and increments `cache_misses`, so "misses" is not strictly "client-visible misses", INFERENCE).
3. `waiter_limit_rejections` counts followers rejected because the per-key waiter list reached `max_waiters_per_key` (1000 default `cache.rs:335`); not in schema 2.
4. Per-request raw observations carry `waiters_coalesced` (0/1) and mark coalesced followers as `hit = true` (`server.rs:1583-1608` returns `(resp, true, 1, false)`); `calculate_coalescing_savings` sums them (`telemetry.rs:660-667`), but that ring is not exposed over HTTP. So, in the telemetry ring, a coalesced follower is counted as a hit for hit-rate purposes while the cumulative `hit_ratio_cumulative` in the stream ignores them (see O6/O7).
5. There is no per-fetch fan-in count (no "N followers joined op 42"), no join timestamp, no record of how long followers waited, and no counter of origin requests avoided. Origin requests avoided is **derivable as an estimate**: `coalesced_waiters` (INFERENCE: each follower would otherwise have been a leader-or-follower of another fetch; it is exact only if every follower would have missed independently, which depends on timing).
6. Diagnostic events: none for joins (`DiagnosticEvent` names emitted: CacheHit, CacheStaleServed, OriginFetchSuccess, OriginFetchNon200, OriginFetchFailed, CacheInvalidated).
7. Client side: `X-Cache-Status: COALESCED` exists on the response (`server.rs:1594-1600`); followers also may see `STALE` if the shared resource is already expired.

**What real data say (FACT, retained Scenario B, 30 runs, 69 107 pooled window requests per arm (10 repetitions), loopback, scripted origin, 300 rps, 5 000-key catalogue).** `summary.json` `cache_status_counts_total`: BASE COALESCED 982 / HIT 26 366 / MISS 41 759; STATIC-CAP COALESCED 940 / HIT 26 189 / MISS 38 557 / NONE 3 421; AIMD-CAP COALESCED 756 / HIT 25 467 / MISS 31 050 / NONE 11 834. Coalescing was ~1.4% of window requests (BASE); that workload is not a stampede. NONE = responses without an `X-Cache-Status` (sheds). Any visual that claims coalescing as a dramatic effect would misrepresent this evidence.

**Verdict.**
- Aggregate real-data visual (IMPLEMENTABLE FROM EXISTING DATA): "followers vs leaders over time" from `coalesced_waiters` and `cache_misses` (both in the stream, 100 ms resolution), e.g. a rolling ratio; and per-arm status mix from retained Scenario B summaries.
- Per-fetch convergence animation (callers converge on one fetch): **ILLUSTRATIVE ONLY** from the live stream; **REPLAYED** at per-request granularity is possible only by downloading Scenario B `client_requests.csv` (key + timing + status), which still shows requests, not the leader/follower linkage (INFERENCE: followers can be inferred only by timing overlap on the same key; unreliable).
- Needs new telemetry (PROPOSAL, matrices file M3): `coalescing_group` summary events (op id, key fingerprint, followers at close, wait durations) sampled; or counters `origin_fetches_started` and `followers_per_fetch` histogram.
- Overhead: followers are on the request hot path under a shard mutex; adding per-follower events inside `begin_fetch_atomic` would extend the critical section. An aggregate-at-completion event emitted from `finalize_fetch` after the shard lock is released (`WaiterBatch` delivery already happens outside the lock) would not (INFERENCE; PROPOSAL only).

## 6. Discrepancies found (documentation versus code, or between documents)

| # | Claim | Where | Code reality | Tag |
|---|---|---|---|---|
| D1 | "weighted rendezvous hashing" | task brief / architecture wording | `capacity_weight = 1.0` constant, plain HRW (`router.rs:1363,1381`) | FACT |
| D2 | `capabilities.controller` "none" / Intelligence page "not available" | `server.rs:511-515` | A real controller (AIMD origin cap) runs when configured and is reported under `metrics.origin_cap`; the capability flag keys only on the legacy `pie` field | FACT; presentation decision for lead |
| D3 | CLAUDE.md s6 "evictors ... are scans" | CLAUDE.md | s5 and `eviction.rs:14-35,44-60` show trait-based O(1) structures; R4 did not audit all three for complexity | FACT/UNVERIFIED |
| D4 | Startup banner (`main.rs:368`) says peer invalidations do not purge the gateway HTTP cache | `main.rs` | local node shares the serving cache (`main.rs:332`) | FACT-SECOND CLAUDE.md s6, spot-checked line 332 |
| D5 | Schema-2 `hit_ratio_cumulative` described as "hit ratio" | catalog note | excludes coalesced, stale, bypass requests (counted neither as hit nor miss); `requests_total ≠ hits + misses` | FACT (`cache.rs:1285,1347`; `server.rs:542-556`) |
| D6 | Latency percentiles come from the "AtomicLogLinearHistogram, 2 688 buckets" (CLAUDE.md s5) | CLAUDE.md | the stream's `latency_ms` percentiles are computed by sorting the raw 10 000-entry ring on each frame (`telemetry.rs:500-517`, capacity `main.rs:254`); the histogram is fed but is not the source of the stream percentiles | FACT; also an overhead note: a sort of up to 10 000 values per SSE client every 100 ms |
| D7 | `PHANTOM_DEFAULT_TTL_MS` configures default TTL | config docs | not wired; literal 60 000 at `server.rs:1405,1688` | FACT-SECOND/FACT |
| D8 | Anti-entropy has no production caller | older CLAUDE.md text | called from `main.rs:375,383` (CLAUDE.md already marks it outdated) | FACT |
| D9 | `AuditStore`/intelligence endpoints show decisions | spec | store never appended to; endpoint returns empty | INFERENCE (grep) |
| D10 | Viewmodel text "Lab runs are offline files today and are not served by the gateway" | `ui/src/lib/viewmodel.js:139` | the Lab API now serves runs when `PHANTOM_LAB_ENABLED=true` (`lab_api.rs:1025`) | FACT; stale UI string (not a recommendation to edit) |
| D11 | Diagnostic buffer lossy semantic | `04_lab_ux_design.md` 1.2.3 | confirmed: try_lock drop + `diagnostic_events_dropped` (`diagnostics.rs:200-215`) | FACT |
| D12 | `OriginFetchNon200` | `04_lab_ux_design.md` 1.2.4 "not re-verified" | exists (`server.rs:1807`) | FACT |

## 7. The three most meaningful real-data visual stories (PROPOSAL)

1. **Origin protection under a degrading origin (Scenario B).** Real series exist at ~250 ms resolution (cap, in-flight, rejections, hits, misses, coalesced, bytes) inside retained `gateway_snapshots.jsonl`, plus per-request `x_cache_status` and origin arrival concurrency (`conc_at_arrival`). Live, the same fields exist in the stream (`origin_cap`). Honest framing is binding: label MEASURED_LOOPBACK, with the notice (no claim of benefit; AIMD behaved as a static cap near 12 in the retained experiment).
2. **Request outcome mix and its honest remainder.** HIT / MISS / COALESCED / STALE / shed shares per arm and over time from counters (live) and from Scenario B status counts (retained); shows what fraction of work the cache absorbed and what was shed, including the share that is unattributed (stale/bypass) rather than fabricating a hit ratio.
3. **Evidence journey: configuration → run → events → verdict (Lab).** Run state, progress (done/total, arm, rep, phase), result and verdict with negative verdicts shown identically (sealed M6: H1 NOT_SUPPORTED). All of it is real API data and already partly rendered.

Runner-ups: resident bytes vs budget with eviction/rejection counters (REAL, simple); freshness counters (REAL, but the Scenario B workload never exercised stale serving).

## 8. Cross-agent section

See `coord/r4.md` for the live exchange log. State at time of writing: R1 asked (1) which objects carry provenance labels and the vocabulary, (2) which values are time series / snapshot / event and update rates, (3) which flows have genuine failure or degraded states. R2 asked for per-object event rate and cardinality, which timestamps exist, whether percentile histograms are raw or pre-aggregated, and whether an origin-concurrency-vs-cap series with timestamps exists. R3 asked which events stream, at what rate, for rendering and test budgets. Answers are in `coord/r4.md` under "Replies" and tabulated in the matrices file (M6). The design direction (`docs/design/phantomcache_design_direction_v2.md`) already treats request flow and node relationships as labelled illustrations (s5) and restricts operational motion to real events (s0 item 7, s6); R4 agrees and refines which flows can be event-driven (matrices file M1). `docs/design/remediation/03_validation_accessibility.md` does not exist at time of writing (checked).

## 9. Limits of this research

- No code was executed. Counters were read, not observed changing. Line numbers will drift as the dirty tree changes.
- Node-level cluster details were read mostly through the A2 matrix (FACT-SECOND) and spot-checked at `router.rs:1351-1418,1790-1900,2128`, `node.rs:10-72`, `server.rs:1089-1092`, `main.rs:332`; frame and pool internals were not re-read.
- The T1 simulator was read at header/CSV level; its internal event types were not enumerated beyond the per-request `outcome` field.
- Volume estimates in the matrices file are derived from the 100 ms stream interval, the 250 ms Scenario B snapshot cadence and the load rates stated in Scenario B; production rates are unknown (UNVERIFIED).
