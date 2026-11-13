# Chapter 1 evidence table

Written 2026-10-30 (machine clock about 16:15 +05:30). Tree: HEAD `caeedfd8783a8a7ce69ba1ac53d483734dc0c3c1`, branch `feat/publish-phantomcache-core`, `git status --short` 111 lines at start. No tests or builds were run for this chapter. "Opened" means I read the lines in the current tree.

| # | Claim in chapter | Source (file::symbol) | Status label | How verified | Limits |
|---|---|---|---|---|---|
| 1 | Routes `/api/v1/cache/*key` (GET, DELETE), `/health`, `/api/events`, `/api/control`, metrics, diagnostics, intelligence status/audit exist | `src/api/server.rs` router builder (lines ~199-216) | IMPLEMENTED | Opened | Auth detail left to Ch 2/6 |
| 2 | `X-Cache-Status` values HIT, MISS, COALESCED, STALE, BYPASS, BYPASS-AUTH | `src/api/server.rs::get_cache_handler` | IMPLEMENTED, TESTED | Opened handler; grep of header inserts | HIT branch set at two places (not read fully) |
| 3 | Key = tenant + path + query | `src/core/cache.rs::CacheKey::new` called in `get_cache_handler` | IMPLEMENTED | Opened call site; did not open `CacheKey` body | Canonicalisation rules are Ch 4 |
| 4 | Bypass branch returns before cache lookup, never stores | `get_cache_handler` (is_bypass / force_auth_bypass branch) | IMPLEMENTED | Opened | |
| 5 | Stale serving via `check_stale_serving`, background revalidation, bounded slot | `get_cache_handler`, `PartitionedCache::check_stale_serving`, `try_reserve_background_slot` | IMPLEMENTED | Opened handler (to line ~1395) | Not run |
| 6 | 100-client barrier: 1 MISS, 99 COALESCED, origin count 1 | `tests/integration/m1_http_tests.rs::test_m1_t13_barrier_stampede_100_clients` | TESTED (loopback, mock origin, 150 ms delay) | Opened test source; not run | Proves mechanism on a debug build; not performance |
| 7 | Expiry herd: one initiator, others wait | `tests/integration/serving_contract_tests.rs::test_ct9_expiry_herd_coalescing` | TESTED | Opened first ~25 lines of test | Rest of test not read |
| 8 | Single-flight uses `SharedOp` + `begin_fetch_atomic`; waiters are `oneshot` senders; cap `max_waiters_per_key` default 1000 | `src/core/coalesce.rs::SharedOp`; `src/core/cache.rs::begin_fetch_atomic`, `max_waiters_per_key` | IMPLEMENTED, TESTED | Opened | |
| 9 | Legacy `Coalescer` is unused in serving path (`_coalescer` in main; `node.rs` holds one) | `src/main.rs` (`let _coalescer`), `src/dist/node.rs:43` | IMPLEMENTED (legacy) | grep + opened `main.rs` line | I did not check whether `node.rs` ever calls its coalescer |
| 10 | Serving cache has 8 shards; shard index = hash & mask; shards must be power of two | `src/main.rs` `PartitionedCache::new(8)`; `cache.rs::get_shard_index`, `new_with_diagnostics` assert | IMPLEMENTED | Opened | Handoff "64" is outdated |
| 11 | Default eviction SIEVE; LRU and S3-FIFO selectable | `src/core/cache.rs::CacheShard::new`, `enum EvictionPolicy` | IMPLEMENTED | Opened | Complexity/faithfulness is Ch 4 |
| 12 | `finalize_fetch` stores only if generation == op_gen and epochs match; counts `stale_writes_prevented` | `src/core/cache.rs::finalize_fetch` | IMPLEMENTED, TESTED (per other docs) | Opened lines 1401-1475 | Test names for fencing taken from the three-pillar doc, not run/opened |
| 13 | `PHANTOM_DEFAULT_TTL_MS` NOT WIRED; literal 60_000 at two call sites | `src/config.rs` (Wiring::NotWired); `src/api/server.rs` (grep `60_000`: leader path ~1688; background revalidation ~1405) | IMPLEMENTED (as unwired) | Opened config line; grep; opened ~1688 context | `limitations.md` quotes lines 1405/1688; confirmed |
| 14 | `PHANTOM_MAX_STALE_MS` default 24 h, `PHANTOM_MAX_CACHE_BYTES` default 256 MiB | `src/config.rs` key table | IMPLEMENTED | Opened MAX_CACHE_BYTES line; MAX_STALE_MS default from CLAUDE.md/config grep (not opened) | |
| 15 | Freshness lifetimes have no gateway ceiling | `docs/user/limitations.md` citing `freshness.rs:263-276` | IMPLEMENTED (limitation) | Read limitations.md; did NOT open freshness.rs | Reported by doc |
| 16 | GET local only; router used for DELETE | `get_cache_handler` (no router reference), `delete_cache_handler` (`state.router`, `invalidate_key_across_cluster`) | IMPLEMENTED | Opened both; grep `state.router` (only lines 505, 663-664 telemetry, 1992) | |
| 17 | `forward_key_over_tcp` defined, no caller in `src/` | `src/dist/router.rs:1420` | NOT IMPLEMENTED (peer reads) | grep across `src/` | Tests may call it |
| 18 | Cluster `GetRequest` handler is a stub (DR-029) | `src/dist/node.rs` `MessageType::GetRequest` at ~409 | PROTOTYPE ONLY | Confirmed match exists; body not read; characterisation from STATUS.md 3b / audit | Fabrication detail UNVERIFIED by me |
| 19 | `route_key` weight = hash * 1.0 (constant); `DefaultHasher` | `src/dist/router.rs::route_key`, `calculate_hash` | IMPLEMENTED | Opened | |
| 20 | Local-first delete then fan-out; 150 ms default timeout; anti-entropy worker 250 ms | `delete_cache_handler`; `src/config.rs` `PHANTOM_CLUSTER_TIMEOUT_MS` default 150; `src/main.rs` `start_anti_entropy_worker(250ms)` | IMPLEMENTED, TESTED (loopback, debug per STATUS/audit) | Opened | Test results reported, not run |
| 21 | mTLS cluster: local node shares serving cache; startup message "[EXPERIMENTAL] Cluster mode" | `src/main.rs` cluster branch (`new_with_cache`) | IMPLEMENTED | Opened | |
| 22 | Static membership from file; no dynamic join | `src/main.rs`, audit capability matrix | IMPLEMENTED (static); dynamic NOT IMPLEMENTED | Opened cluster branch partially; matrix reported | |
| 23 | Owed invalidations in-memory only; defect reproduced; patch not applied | `docs/user/limitations.md`; `docs/STATUS.md` row M3; `docs/research/m3_durability/` | NOT IMPLEMENTED; patch PROTOTYPE ONLY | Read limitations/STATUS; did not open m3_durability | Reproduction by patch author, not independent |
| 24 | Intelligence default Off; five modes | `src/intelligence/mod.rs::IntelligenceMode`; `src/config.rs` `PHANTOM_INTELLIGENCE` | IMPLEMENTED | Opened | |
| 25 | Origin cap enforced only in Canary/Active with a cap configured | `src/intelligence/executor.rs::origin_cap_enforcing` | IMPLEMENTED, TESTED | Opened | |
| 26 | Cap rejection -> HTTP 503, `X-Cache-Error: CapacityExceeded` | `src/api/server.rs::map_origin_error_to_response`, `acquire_origin_permit` | IMPLEMENTED | Opened | |
| 27 | Coalesced waiters do not consume permits | `tests/integration/origin_cap_tests.rs::coalesced_waiters_do_not_consume_permits` | TESTED | Found by grep (name); body not read | Name only |
| 28 | `pie: None` in main | `src/main.rs` AppState construction | PROTOTYPE ONLY (decision engine not wired) | Opened | |
| 29 | CountMinSketch unused on serving path | `src/intel/sketch.rs`; referenced only in `tests/unit/wp2_tests.rs` | PROTOTYPE ONLY | grep across `src/`, `tests/`, `benches/` (files-with-matches) | |
| 30 | `hot_keys` reported unavailable/not_implemented | `src/api/server.rs` (line ~766) | IMPLEMENTED (as unavailable) | Opened line via grep | |
| 31 | Sealed M6: H1 NOT_SUPPORTED, Tier 1 SIMULATED, adaptive worse in 7/8 peak-origin scenarios | `docs/research/m6_sealed_results.md` | TESTED (simulated); benefit NOT ESTABLISHED | Read the verdict section | Did not open lab-results CSVs; sealed dir not touched |
| 32 | Scenario B notice text (adaptive ~ static 12; not established list) | `docs/demo/scenario_b_interpretation_notice.md` | TESTED (loopback T2) with listed limits | Read the notice; `lab-results/scenario_b/20261030T003932Z` exists (ls) | Raw data not opened |
| 33 | Cluster and Intelligence UI screens are placeholders | `ui/src/App.jsx::renderScreen` | PARTIAL | Opened | Component content not read |
| 34 | 851 passed / 75 targets / 8 ignored (debug, 2026-10-30 10:11-10:23); 298 UI tests | `docs/STATUS.md` section 3 | Reported by STATUS (not reproduced here) | Read | I did not run tests |
| 35 | G4 Cycle 1: 99 findings; G4A-01 UNDER-INVESTIGATION; reviews are AI second looks | `docs/release/g4/cycle1_findings_disposition_register.md` | UNVERIFIED individually | Read first 50 lines | Findings not individually re-checked |
| 36 | History: first README said "intelligent in-memory distributed cache"; cluster modelled; single-node-first owner decision | `docs/research/vision/product_vision_vs_implementation_audit.md` sections 1-4 | Reported by prior agent audit | Read; did not open git history or transcripts | The audit states several gaps |
| 37 | Three-pillar wording and level A-D definitions are a PROPOSAL, not approved | `docs/research/vision/phantomcache_three_pillar_product_direction.md` header and section 1-2 | Proposal | Read lines 1-220 | Owner pillar names ("Distributed Systems Engineering / Workload Intelligence / Product Excellence") taken from the spec |
| 38 | Delete handler comment "Guarantees linearizability" | `src/api/server.rs::delete_cache_handler` | n/a (code comment) | Opened | Contradicts CLAUDE.md section 4 policy |
| 39 | Tenant header ignored in secure mode; roles list | `get_cache_handler`; `docs/user/run.md` | IMPLEMENTED, TESTED | Opened both | Tests not opened |
| 40 | Not provided: Prometheus `/metrics`, container image, SBOM, advisory scan, signing | `docs/user/limitations.md` | UNVERIFIED (doc claim) | Read | |

## Figure verification

- Fig 1.1 (`ch01_request_outcomes.svg`): outcomes HIT, STALE, COALESCED, MISS, BYPASS and the order auth -> key -> lookup were checked against `get_cache_handler`. Stale check precedes fresh-hit path in code (stale returns `STALE` when `check_stale_serving` is Eligible); the figure shows the five outcomes as parallel branches (simplification, stated in the chapter as detail left to Ch 2). The origin-gate caveat (enforced only in canary/active) was checked in `executor.rs`.
- Fig 1.2 (`ch01_stampede.svg`): panel C counts match the asserted values of `test_m1_t13_barrier_stampede_100_clients` (1 MISS, 99 COALESCED, origin count 1). Panels A and B are labelled conceptual.
- Fig 1.3 (`ch01_three_pillars.svg`): every box statement traced to a row above. "Config reference... health routes, Lab API, Scenario B runner" traced to `config.rs`, `health::router`, `lab_api.rs` (`SCHEMA = "lab-1"`), `lab_live/`.

## Discrepancies found (document versus code or later record)

1. README says single-flight is `Coalescer` in `coalesce.rs`; serving path uses `SharedOp`/`begin_fetch_atomic`; `Coalescer` is legacy ("backward compatibility") and `main.rs` binds it to `_coalescer`.
2. README mermaid diagram routes requests through the rendezvous router to a primary; GET never uses the router (DELETE only).
3. README table marks the cluster "Modeled / Simulated"; the mTLS cluster path uses real peers and shares the serving cache; the three demo nodes remain simulated.
4. Handoff `00_START_HERE.md` says 64 partitions; `main.rs` uses 8 shards. It also says "Phase 4 accepted, 242/242"; STATUS says 851 tests, 75 targets, and CLAUDE.md section 6 says the "accepted/zero P0" claim is unsupported.
5. CLAUDE.md and README quote 242 tests / 19 targets; STATUS quotes 851 / 75 (debug).
6. `delete_cache_handler` comment claims "Guarantees linearizability"; project rules forbid that claim without a failure model, bound and falsifying test.
7. `docs/STATUS.md` and `limitations.md` cite HEAD `456bb25` / older line numbers; HEAD is now `caeedfd`. `limitations.md` cites `server.rs:1405` and `:1688` for the `60_000` literal and these still hold; other line numbers in STATUS/audit documents were not re-checked and will drift.
8. `main.rs` still constructs `VirtualClock`, `SeededRngSource`, `TrafficGenerator`, `NetworkLink`, `BackendDatabase` as underscore-prefixed unused locals (leftover simulation scaffolding); not a defect, but it can mislead a reader into thinking the gateway runs on a simulation.
9. The three-pillar proposal and STATUS say the audit store has no producer in the serving path; I confirmed the routes but did not search for producers, so that item is UNVERIFIED by me.

## UNVERIFIED items

- Gateway-side behaviour of the cluster `GetRequest` stub (fabricated value) beyond existence of the match arm.
- That `src/core/freshness.rs` imposes no lifetime ceiling (taken from `limitations.md`).
- Test names for fencing (`invalidation_contract_tests`, `serving_contract_tests` CT-7/INV-2) beyond file existence and CT-1/CT-9.
- Whether `node.rs`'s own `Coalescer` is ever used for requests.
- Contents of `docs/research/m3_durability/` and `lab-results/scenario_b/...`.
- Everything in the 99-finding register beyond its summary header.
- Original rationale for "distributed" (missing early conversation, per the audit).
- Any performance number. None is quoted from a measurement; the worked example in 1.2 is explicitly illustrative.
