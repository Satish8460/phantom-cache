# PhantomCache — Code Findings Register

**Date:** 2026-10-28. **Mode:** read-only. Nothing was executed except `cargo test --all-targets -j 1` (242 passed, 0 failed, 19 targets, debug profile).
**Method:** Source reading plus `Grep` for call sites. "Confirmed" means the statement follows from the code as read; it does **not** mean the performance impact is measured. Impact claims are tagged **[HYP]**.

Status vocabulary: **CONFIRMED** (code supports it), **CONFIRMED-WITH-CORRECTION** (my earlier wording was inaccurate), **UNRESOLVED**, **REJECTED**.

## 1. Findings from the first pass, re-checked

| # | Claim | Status | Evidence | Notes / interpretation |
|---|---|---|---|---|
| F1 | PIE does not estimate a candidate policy's performance | **CONFIRMED** | `src/control/pie.rs:57-119`: `utility = hit_rate*100 - p99*0.1` from `telemetry` of the live (current) policy; `expected_utility: 85.0` literal; commit if `utility > 80`. | It is a threshold trigger with a cooldown, not a counterfactual canary. |
| F2 | Rollback is a stub | **CONFIRMED** | `pie.rs:83`: `candidate_name == "S3-FIFO" && utility < 50.0`, comment "Simulated regression triggers rollback". | Rollback fires on the *low* utility of the current policy, only if the candidate is named S3-FIFO. |
| F3 | PIE tests assert the stub behavior | **CONFIRMED** | `tests/integration/p4_pie_tests.rs:177-206`: `evaluate_candidate("S3-FIFO", ...)` → `Commit`; later → `Rollback`; `LRU` inside cooldown → `NoAction`. | Passing tests validate the state machine's wiring, not decision quality. |
| F4 | `CountMinSketch` is unused | **CONFIRMED** | Grep for `CountMinSketch`: only its own definition in `src/intel/sketch.rs`. (I did not grep `tests/`.) | The earlier capability audit planned to feed `hot_keys` from it (`docs/demo-lab/implementation-roadmap.md`, Phase 2). |
| F5 | Telemetry `key_hash` is always 0 | **CONFIRMED** | All three `RawObservation::new` call sites (`server.rs:482`, `:770`, `:946`) pass `0`. | No per-key signal reaches the controller. |
| F6 | SIEVE is not SIEVE | **CONFIRMED** | `cache.rs:348-394`: iterates `shard.map.iter_mut()` (HashMap order), clears `visited` bits, evicts the first unvisited. No queue, no hand. | Behavior depends on hash iteration order; not the algorithm in the SIEVE paper (to be confirmed by the literature review). |
| F7 | S3-FIFO is not S3-FIFO | **CONFIRMED** | `cache.rs:431-479`: no small/main/ghost queues; `access_count <= 1` split, oldest `last_accessed_ms`. | Same caveat. |
| F8 | LRU eviction is O(n) | **CONFIRMED** | `cache.rs:396-429`: full scan. | All three evictors scan the shard under its mutex. Max 10,000 entries/shard by default (`cache.rs:191`). Stall magnitude is **[HYP]**. |
| F9 | Global epoch mutex on every request | **CONFIRMED** | `get_resource` (`cache.rs:313-346`) and `begin_fetch_atomic` (`:494-612`) call `get_effective_epochs` (`:251-259`), which locks `tenant_epochs: Mutex<HashMap>` while the shard lock is held. | Lock order is shard → tenant_epochs. I found no reverse order in `cache.rs`, so no deadlock was seen. Throughput impact **[HYP]**. |
| F10 | Telemetry is on the hot path with a global lock | **CONFIRMED** | `telemetry.rs:47-53` `record` locks `Mutex<VecDeque>`; `calculate_*` clone and sort the full buffer under that lock. Handlers call `record` at the end of every request (`server.rs:765-771`). | Contention and sort cost **[HYP]**. |
| F11 | Dashboard metrics are synthetic | **CONFIRMED-WITH-CORRECTION** | `server.rs:1585-1593` (throughput from constants + `rand`), `:1597-1607` (hit rate from constants), `:1609-1613` (coalesced ratio, db_load), `:1698-1706` (CPU 85.2/28.4, memory 1 GiB/512 MiB). When `state.cache` exists, throughput, hit rate, latency percentiles and db load are overridden by live counters/telemetry (`:1654-1673`). | Correction: not *all* values are synthetic; it is a **mixture**. CPU, memory, node count effects, avoided-dollar cost, hot keys remain synthetic; the SSE payload does not label which is which. The `live_throughput` is cumulative `requests_total` (a counter), not a rate, by my reading of `:1658,1670` **[to verify]**. |
| F12 | Capacity is entry-count based | **CONFIRMED** | `cache.rs:151,191`: `max_entries_per_shard`; no byte accounting anywhere in `cache.rs`. | Upstream body cap is 2 MB (`MAX_BODY_BYTES`, per `docs/demo-lab/capability-audit.md`, not re-read). Worst case 64 shards × 10,000 × 2 MB ≈ 1.3 TB, so the "no unbounded allocation" non-goal is not met for the cache body store. This is an arithmetic bound, not a measurement. |

## 2. New findings from the second pass

| # | Finding | Status | Evidence |
|---|---|---|---|
| F13 | **DELETE requests are recorded as zero-latency cache misses** in the telemetry that the PIE reads. | **CONFIRMED** | `server.rs:945-953`: `RawObservation::new(now, 0, 0, false, 0)` in the invalidation handler. This deflates `p99` and `hit_rate` as invalidations increase. |
| F14 | **`reconcile_peer_anti_entropy` has no call site in `src/`.** | **CONFIRMED** (for `src/`; tests call it) | Grep `reconcile` over `src/`: only the definition at `router.rs:787`. Whether `node.rs`'s recovery path uses a different mechanism was not traced. |
| F15 | **The HTTP GET path never uses the router.** `route_key`, `get_replicas`, `forward_key_over_tcp` are called only inside `router.rs`. `server.rs` uses the router only at `:821` for `invalidate_key_across_cluster`. | **CONFIRMED** | Grep: `router` appears in `server.rs` at lines 61, 65-66, 821, 824 only. Implication: each gateway serves GETs from its own local cache; the "distributed cache" is a **set of independent local caches plus a distributed invalidation protocol**. This matches the earlier `capability-audit.md` §5 finding, whose "Phase 6: wire HRW into Axum" was not done (**not verified further**). |
| F16 | Replica invalidation broadcast is lossy by construction, with retries limited to 3 and a 1,024-slot queue. | **CONFIRMED** | `router.rs:61` (`channel(1024)`), `:194-202` (`try_send`, drop counter on failure), `:110` (`for retry in 0..=3`). Dropped or exhausted tasks are not recorded anywhere I saw other than counters. |
| F17 | Receiver-side invalidation is recorded in the ring buffer before application. | **CONFIRMED** | `node.rs:475-481` pushes then deletes. Whether the *sender* also records into its own ring (needed for replay to a peer that missed it) was not traced. |
| F18 | Because of F14–F16 the staleness bound for non-primary copies is not established. | **UNRESOLVED** | Needs: (a) trace of who records invalidations on the primary, (b) trace of recovery triggers in `node.rs`, (c) an experiment with injected drops. Do **not** state a bounded-staleness guarantee. |
| F19 | The `Coalescer` struct in `coalesce.rs:18-58` appears to be legacy; the production path is `PartitionedCache::begin_fetch_atomic` + `FetchRegistration`. | **CONFIRMED** (reading) | The legacy `register/resolve` uses `Mutex<HashMap>`; handlers use `FetchRegistrationResult`. Legacy shards also remain in `PartitionedCache` (`legacy_shards`). Candidate for removal; not verified for external users. |

## 2b. Resolution of F14–F18 (see `architecture_investigation.md`)
- **F14** — CONFIRMED and extended: no indirect trigger exists either.
- **F15** — CONFIRMED and extended: GET is local-only *and* peer invalidations purge a separate per-node cache, not the serving cache (A1; verified in `main.rs:65`, `node.rs:42,485`).
- **F16** — CONFIRMED; worse: drops/exhaustion leave no record and the "bounded" queue feeds unbounded spawned tasks (partly agent-reported).
- **F17** — RESOLVED: the sender never records outbound invalidations (`ring_buffer.push` only at `node.rs:475`) — verified.
- **F18** — RESOLVED to the extent possible: the only staleness bound is the fixed 60 s TTL; no cross-gateway repair exists.
- **New:** shared global `stream_seq` across targets and retries (`router.rs:142,615`) — verified; DELETE ordering reversed relative to docs (`server.rs:830-832`) — verified.

## 2c. Additional verified findings (second implementation pass, see ADR-001)
| # | Finding | Status | Evidence |
|---|---|---|---|
| F20 | Gap handling discards the gap: `record_inbound_seq` sets `expected = stream_seq+1` on a gap and `get_highest_acked_seq` returns `expected-1`, so a reconciliation request would ask only for records after the latest seen number; `pop_gap` (which could drive targeted replay) has no caller | CONFIRMED (reading); consequence inferred, not run | `anti_entropy.rs:200-231`; `router.rs:806-810` |
| F21 | `throughput_req_sec` in the live payload is the cumulative `requests_total` counter cast to f64 (not a rate); the "p95" field carries the p90 | CONFIRMED | `server.rs:1654-1673` |
| F22 | PIE DECISION INSPECTOR in the UI is hard-coded text chosen by the traffic preset ("✓ COMMITTED", "Canary Duration: 30s"); it shows no PIE data | CONFIRMED | `ui/src/App.jsx:412-429` |
| F23 | UI ignores the real `cache_hits/misses/hit_ratio` fields and recomputes hits/misses/coalesced/cost from constants; "Phantom OFF" multiplies latencies client-side; hit-rate and replication charts push constants; backend URL hard-coded | CONFIRMED | `App.jsx:53-66,87,93-98,167-168,179` |
| F24 | No test calls `main()`; the DELETE-over-HTTP tests assert only status/headers, with the gateway cache and the node cache as separate objects | CONFIRMED | `security_phase4b_router_tests.rs:417-492`; `main.rs` has no test module |
| F25 | In clustered mode the router's node list includes the local node; whether HRW selecting the local node causes a TCP self-connection and what it purges was not traced | UNRESOLVED | `main.rs:234-264` |

## 3. Still unresolved / not inspected

- `frame.rs`, `pool.rs`, `session.rs`, `anti_entropy.rs` internals; `security/*`; `main.rs` beyond grep hits; `fetch_origin_raw` (current version) and whether HTTPS origin is now handled (the audit said it was plaintext; Phase 2 security work claims to address it **[HANDOFF]**).
- Whether `requests_total` is incremented per request and how the SSE derives throughput.
- Whether WorkloadProfile (`intel/workload.rs`) has any consumers.
- Handoff docs `03`, `06`, `08`; the transcripts; UI behavior at runtime (`ui/src/App.jsx` read only through greps and the earlier capability audit).
- Clippy and whitespace claims (not re-run).

## 4. What these findings imply for sequencing

Findings F1–F5, F11, F13 mean that any research on adaptive control would currently be built on telemetry that is partly synthetic and partly polluted. F15 means any distributed-GET claim has no runtime path to exercise. Therefore **measurement truth precedes controller design**. See `plan_of_action.md`.

---

## 5. Known-Gap and Characterization Test Results (WP-00)

Executed via `tests/integration/known_gaps.rs` per `phase_1_execution_package.md` v3.1 §6.2.

| ID | Defect / Architectural Gap | Code Reference | Test Target & Form | Outcome | Verified Evidence |
|---|---|---|---|---|---|
| **KG-1** | Peer invalidation does not purge another gateway's HTTP cache (Q1/Q2) | `src/main.rs:65`, `src/dist/node.rs:42,485` | `kg1_peer_invalidation_must_purge_other_gateways_http_cache` (ignored) | **REPRODUCED** | Fails 3/3 with `KG-1 DEFECT: peer invalidation on Gateway A did not purge Gateway B's HTTP cache (expected origin count 2, got 1)`. Preconditions passed. Paired positive control `pc1_standalone_delete_purges_local_gateway_entry` passes. |
| **KG-2** | `reconcile_peer_anti_entropy` has no production caller (Q4) | `src/dist/router.rs:787` | Evidence note (not a test) | **REPRODUCED** | Confirmed: `rg -n "reconcile_peer_anti_entropy"` finds only definition at line 787; zero production call sites in `src/`. |
| **KG-3** | Gap handling discards gaps: acking advances past missing sequence (Q5b) | `src/dist/anti_entropy.rs:200-231` | `kg3_gap_handling_advances_past_missing_sequence` (ignored) | **REPRODUCED** | Fails 3/3 with `KG-3 DEFECT: highest acked sequence advanced past missing sequence (expected 2, got 5)`. Precondition passed (`pop_gap` yielded `Some((7, 3, 4))`). Paired positive control `pc3_sequential_inbound_updates_highest_acked_seq` passes. |
| **KG-4** | Shared stream sequence across nodes causes false interleaved gaps (Q5) | `src/dist/router.rs:142,615` | `kg4_shared_stream_sequence_causes_false_interleaved_gaps` (ignored) | **REPRODUCED** | Fails 3/3 with `KG-4 DEFECT: false gaps detected due to shared stream_seq (node1 gaps: 1, node2 gaps: 1)`. Preconditions passed. Paired positive control `pc4_single_target_stream_sequence_has_zero_gaps` passes. |
| **KG-5** | DELETE purges local entry only after cluster round trip completes (Q2) | `src/api/server.rs:830-832` | `kg5_get_after_delete_start_must_not_serve_old_entry` (ignored) | **REPRODUCED** | Fails 3/3 with `KG-5 DEFECT: GET served stale cached entry while cluster DELETE was in flight (got cache HIT)`. Precondition passed (DELETE in-flight at GET time). Paired positive control `pc5_get_after_delete_completes_refetches` passes. |
| **KG-6** | Node `GetRequest` fabricates `val_node_<id>` on missing key | `src/dist/node.rs:406` | `pin_kg6_node_get_request_fabricates_missing_key_body` (pin test) | **REPRODUCED** | Pin test passes: missing key over TCP returns payload starting with `MISS val_node_42`. |
| **TG-1** | DELETE recorded as zero-latency miss in telemetry (F13) | `src/api/server.rs:945-953` | `pin_tg1_delete_records_as_zero_latency_miss_in_telemetry` (pin test) | **REPRODUCED** | Pin test passes: DELETE records `RawObservation(now, 0, 0, false, 0)`, inflating total request count and miss count. |
| **TG-2** | `key_hash` is 0; p95 holds p90; throughput is cumulative counter (F5, F21) | `src/api/server.rs:482,770,946`, `187,1665`, `181` | Evidence note | **REPRODUCED** | Confirmed by code inspection: `RawObservation::new` hardcodes `0` for key_hash; SSE derives p95 from `calculate_percentiles().1` (the p90 percentile); `throughput_req_sec` casts total requests directly. |
| **PIE-1** | `PieController` not constructed in `main()`; legacy stub semantics | `src/main.rs:306`, `src/control/pie.rs` | Evidence note | **REPRODUCED** | Confirmed: `main.rs:306` explicitly passes `pie: None` to `ApiState`. `src/control/pie.rs` remains legacy threshold stub with literal `expected_utility: 85.0`. |

