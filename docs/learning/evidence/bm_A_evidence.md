# Back-matter writer A: evidence file (Appendices A, B, C, H)

Written 2026-10-30 16:55 IST. HEAD at the time: `804a7b8` on `feat/publish-phantomcache-core`. Method: read the chapters (Chapter 2 sections 2.1 to 2.8 in full; Chapter 1 sections 1.5 to 1.8; the capability and status tables, judge boxes and key sections of Chapters 3 to 8) and grep the current tree under `src/` and `tests/`. No cargo, no tests run. Status: "verified in code" means a grep hit on the cited symbol or value in the current tree.

## 1. Symbols and facts verified in the current code

| # | Claim used in the appendices | Found at | Result |
|---|---|---|---|
| 1 | Production cache has 8 shards | `src/main.rs:204` `PartitionedCache::new(8)` | verified |
| 2 | `pie: None` in the shipped binary | `src/main.rs:433`; header comment `src/control/pie.rs:5` | verified |
| 3 | `begin_fetch_atomic` | `src/core/cache.rs:1238` | verified |
| 4 | `finalize_fetch` | `src/core/cache.rs:1401` | verified |
| 5 | `delete_impl` | `src/core/cache.rs:1586` | verified |
| 6 | `audit_bytes` | `src/core/cache.rs:1827` | verified |
| 7 | `calculate_state_bytes` | `src/core/cache.rs:198` | verified |
| 8 | `try_invalidate_tenant` | `src/core/cache.rs:576` | verified |
| 9 | `get_effective_epochs` | `src/core/cache.rs:560` | verified |
| 10 | `MAX_TENANT_EPOCHS = 10_000` | `src/core/cache.rs:273` | verified |
| 11 | `epoch_snapshot: RwLock<Arc<HashMap<String,u64>>>` | `src/core/cache.rs:288` | verified |
| 12 | `spawn_sweeper_task`, `sweep_lazy_reclamation` | `src/core/cache.rs:459`, `:386` | verified |
| 13 | `check_stale_serving` | `src/core/cache.rs:749` | verified |
| 14 | `complete_not_modified` | `src/core/cache.rs:1025` | verified |
| 15 | max waiters per key default 1,000 | `src/core/cache.rs:331` | verified |
| 16 | max background refresh default 128 | `src/core/cache.rs:337` | verified |
| 17 | stale cap default 86,400,000 ms | `src/core/cache.rs:336` | verified |
| 18 | `stale_writes_prevented` counter | `src/core/cache.rs:1472`, `src/intel/diagnostics.rs:34` | verified |
| 19 | `WaiterLimitExceeded`, `CapacityExceeded` rejection | `src/core/cache.rs:1305`; `src/api/server.rs:2206-2231` | verified |
| 20 | `SharedOp` | `src/core/coalesce.rs:97` | verified |
| 21 | `route_key`, constant `capacity_weight = 1.0` | `src/dist/router.rs:1351`, `:1363`, `:1381` | verified |
| 22 | `calculate_hash` uses node id and key | `src/dist/router.rs:1343` | verified (body not re-read) |
| 23 | `invalidate_key_across_cluster` | `src/dist/router.rs:1790` | verified |
| 24 | `forward_key_over_tcp` defined, no production caller | defined `src/dist/router.rs:1420`; grep of `route_key` and `forward_key_over_tcp` shows uses only in `router.rs` (per Chapter 2; this author grepped only the definition and `fanout_targets`) | definition verified; "no caller" taken from Chapter 2 and Chapter 5, not re-proved |
| 25 | `fanout_targets`, `RepairLedger`, `reconcile_peer_anti_entropy` | `src/dist/router.rs:1705`, `:137`, `:2128` | verified |
| 26 | Anti-entropy worker every 250 ms in cluster branch | `src/main.rs:375`, `:383` (`from_millis(250)`); definition `router.rs:1000` | verified |
| 27 | `PER_PEER_INFLIGHT = 1` | `src/dist/router.rs:506` | verified |
| 28 | Broadcaster concurrency limit 16 | `src/dist/router.rs:618` `Semaphore::new(16)` | verified |
| 29 | `InvalidationRingBuffer`, 4,096 default ring entries, SHA-256 digest | `src/dist/anti_entropy.rs:45`, `:9`, `:174` | verified |
| 30 | `AntiEntropyTracker` | `src/dist/anti_entropy.rs:347` | verified |
| 31 | Frame magic `PHAN` | `src/dist/frame.rs:5` (`0x50 0x48 0x41 0x4E`) | verified |
| 32 | `ExactKey` scope used by the HTTP DELETE | `src/api/server.rs:1997`; enum `src/dist/frame.rs:57` | verified |
| 33 | Handshake permits 64, 1,000 ms handshake timeout | `src/dist/node.rs:228`, `:273` | verified |
| 34 | Pool heartbeat interval 5 s | `src/dist/pool.rs:101`; `probe_idle_connections` `:360` | verified |
| 35 | `GetRequest` handler fabricates `val_node_<id>` | `src/dist/node.rs:439`, `:447` | verified |
| 36 | Local node built with `new_with_cache` in the cluster path | `src/main.rs:332`; `src/dist/node.rs:62` | verified |
| 37 | Cluster timeout key default 150 | `src/config.rs:159` | verified |
| 38 | TLS 1.3 only, `WebPkiClientVerifier` | `src/security/cluster.rs:623-634` | verified |
| 39 | SAN format `node-{id}.{cluster}.phantom.internal`; `validate_peer`; `load_membership_from_file` | `src/security/cluster.rs:66`, `:365`, `:557` | verified |
| 40 | HS256 only, `JwtValidator` | `src/security/jwt.rs:113`, `:153` | verified |
| 41 | `authenticate_request`, `require_tenant_claim` | `src/api/server.rs:985`, `:1052` | verified |
| 42 | `validate_startup_security` | `src/security/mod.rs:108` | verified |
| 43 | `resolve_and_pin_origin`, `PreConnectIpFilter` | `src/security/upstream.rs:309`, `:119` | verified |
| 44 | Body limit 512 KiB on ingress; origin body cap 2 MiB | `src/api/server.rs:220`; `:2317` | verified |
| 45 | `fetch_origin_raw`, `encode_origin_target` | `src/api/server.rs:2534`, `:2478` | verified |
| 46 | `get_cache_handler`, `delete_cache_handler` | `src/api/server.rs:1136`, `:1937` | verified |
| 47 | `PrimaryFailover` warning header | `src/api/server.rs:2006-2010` | verified |
| 48 | `Retry-After: 1` on capacity refusals | `src/api/server.rs:2210`, `:2223` | verified |
| 49 | `x-cache-bypass` header | `src/api/server.rs:162`, `:1244` | verified |
| 50 | `DefaultBodyLimit` and CORS layer in the router | `src/api/server.rs:220` | verified |
| 51 | `PHANTOM_DEFAULT_TTL_MS` is NOT WIRED; literal `60_000` at two call sites | `src/config.rs:62`, `:117`; `src/api/server.rs:1405`, `:1688` | verified (the Chapter 4 line numbers 1405 and 1688 are correct) |
| 52 | Sweeper defaults 100 ms and 32, described as "shards per tick" | `src/config.rs:121-123` | verified |
| 53 | `PHANTOM_INTELLIGENCE` default `off`, enum values | `src/config.rs:161` | verified |
| 54 | `PHANTOM_ORIGIN_CAP_MAX` and the inert-cap warning | `src/config.rs:163`, `:878-879` | verified |
| 55 | `OriginGate`, `OriginPermit`, `origin_cap_enforcing`, `acquire_origin_permit`, `route_bucket` | `src/intelligence/executor.rs:78`, `:91`, `:217`, `:233`, `:270`; wrapper `src/api/server.rs:2377` | verified |
| 56 | AIMD `step`, factor 0.70 | `src/intelligence/candidates.rs:155`, `:158` | verified |
| 57 | `FrequencyAdmissionFilter`, `OriginAwareStaleController` | `src/intelligence/candidates.rs:65`, `:97` | verified |
| 58 | `IntelligenceMode` | `src/intelligence/mod.rs:24` | verified |
| 59 | `HeavyHitterDetector` (Count-Min plus Space-Saving), CUSUM | `src/intelligence/signals.rs:95`, `:80` header, `:305` | verified |
| 60 | `ShadowEvaluator`, `TinyLfuShadow`, `PieDecisionEngine` | `src/intelligence/shadow.rs:418`, `:310`; `engine.rs:36` | verified |
| 61 | `AtomicLogLinearHistogram`, 2,688 buckets (128 + 20 x 128) | `src/intel/telemetry.rs:274-278`, `:280` | verified |
| 62 | `TelemetryCollector::calculate_percentiles_us` | `src/intel/telemetry.rs:354`, `:500` | verified |
| 63 | Older `CountMinSketch` | `src/intel/sketch.rs:3` | verified (single-use claim taken from Chapter 5) |
| 64 | Eviction trait and three policies | `src/core/eviction.rs:16`, `:49`, `:257`, `:434` | verified |
| 65 | SIEVE is the default policy | `src/core/cache.rs:131`, `:214`, `:332` | verified |
| 66 | Tokio multi-thread runtime in `main` | `src/main.rs:174` | verified |
| 67 | Only `unsafe` blocks are in `lab_api.rs` | `src/api/lab_api.rs:226-321` (grep of `unsafe` over `src/`: only that file in the first hits) | verified for the hits shown |
| 68 | Lab API loopback rule, Host and Origin guard, sealed-salt refusal | `src/api/lab_api.rs:97`, `:1060`, `:18` | verified |
| 69 | T1 `run_cell`, `EventQueue`, `execute` | `src/lab/t1_sim.rs:1102`; `src/lab/events.rs:56`; `src/lab/runner.rs:206` | verified |
| 70 | `MasterRng`, `AliasSampler` | `src/lab/rng.rs:54`, `:230` | verified |
| 71 | `bca_ci_mean`, `sign_flip_p_value`, `HolmBonferroni`, SplitMix64 | `src/lab/analysis.rs:322`, `:365`, `:171`, `:74` | verified |
| 72 | T2 `run_experiment`, `AIMD_EFFECTIVE_CAP_NOTE` (32 to about 12, window max) | `src/lab_live/scenario_b.rs:1204`, `:559` | verified |
| 73 | Named tests exist | `m1_http_tests.rs:612` (t13), `:438` (t09); `invalidation_contract_tests.rs:79`; `origin_cap_tests.rs:233`; `cluster_propagation_latency_tests.rs` | verified |
| 74 | Documents cited exist | `docs/STATUS.md`, `docs/user/limitations.md`, `docs/research/m6_sealed_results.md`, `claims_ledger.md`, `docs/demo/scenario_b_interpretation_notice.md`, `docs/research/g4a01_race/{analysis,status,repro_attempts}.md` and `proposal.patch`, `docs/research/m3_durability/` | verified present; contents not re-read beyond `m6_sealed_results.md` line 19 (MVP-B decision) |
| 75 | Dev and runtime dependency declarations | `Cargo.toml` lines 7-32 (tokio 1.38, axum 0.7, rustls 0.23, ring 0.17, rcgen 0.13, hdrhistogram, proptest) | verified |

Count: 75 items checked against the current tree (about 70 by direct grep, five taken from chapters and noted).

## 2. Figure B.1 check (`docs/learning/assets/bm02_architecture_overview.svg`)

- Checked against Figures 2.1 (components), 5.1 (cluster topology) and 2.6 (Lab and UI) by reading Chapter 2 sections 2.2, 2.7, 2.8 and the Source lines. The new figure shows: ingress with 512 KiB cap and optional JWT (items 41, 44, 50); handlers (46); 8 shards with one mutex each (1); single-flight, fencing, epochs, byte budget, SIEVE default (65); origin path with a gate that is enforced only in canary or active with a cap (55, 53, 54), 5 s and 2 MiB limits (45, 44); cluster router for DELETE only, rendezvous with constant weight, memory-only repair ledger and a 250 ms anti-entropy worker (21, 25, 26, 32); mTLS 1.3 link to peer gateways whose reads never go there (35, 38, 24); telemetry and intelligence drawn dashed as optional or library (59, 60, 61); Lab and UI dashed, off the path (68, 72).
- Render: headless Chrome 900 by 640, own throwaway profile `D:/Temp/bm_prof_A` (deleted afterwards), PNG `D:/Temp/bm_A.png` opened with Read. No text overlaps, no clipped text, every arrow lands on a box edge. Greyscale distinguishable: three fills (light grey, mid grey, white) plus dashed versus solid versus thick borders, with a legend. Minimum text size 13 px. `<title>` and `<desc>` present. viewBox width 900. No external fonts, images or scripts.
- Limit: a drawing of the design as read in code and chapters, not a runtime trace.

## 3. Rules check

- No 64-hex strings (grep `[0-9a-fA-F]{64}` returned 0 in all four appendices). No fenced divs used (0 `:::` lines), so balance is trivial. Tables: every row in every pipe table has the same column count (awk check, no mismatch); the widest table has 5 columns.
- No performance numbers other than configuration constants (shard count, buckets, timeouts, caps) and the 0.39 percent histogram design bound and the 0.70 AIMD factor, all from the chapters. No measured latency or throughput figure appears.
- Word counts after the last edit: A 547, B 3,433 (includes the figure line and source line), C 1,928, H 2,163.
- `D:\Personal\.phantom-sealed-m6` was not read. No file other than the four appendices, the SVG and this file was written.

## 4. Chapter inconsistencies noticed (quoted, not fixed)

1. Chapter 8, section 8.2.1 is stale against the tree. It says "HEAD | `df68eaa` ... (about 16:35; it was `caeedfd` earlier the same afternoon)", "37 (at the time of writing)" tracked files modified and "76 top-level entries" untracked, and "four" commits since `456bb25`. Now: `git rev-parse` gives `804a7b8` (two newer documentation commits, `9ab179f` and `804a7b8`, are in `git log`), and `git status --short` prints 111 lines. The chapter says "at the time of writing", so this is time drift, but the lead should refresh it or mark it clearly.
2. Status labels for the same capability differ between chapters. SIEVE, LRU, S3-FIFO: Chapter 4 section 4.11 "IMPLEMENTED, TESTED (weakly)" and "IMPLEMENTED, defect OPEN" for S3-FIFO; Chapter 8 section 8.12 "SIEVE, LRU, S3-FIFO eviction | PARTIAL". Generation and epoch fencing: Chapter 4 section 4.11 splits "IMPLEMENTED, TESTED" (generation) and "IMPLEMENTED, PARTIAL" (epoch); Chapter 8 section 8.12 has one row "TESTED". Cluster fan-out: Chapter 1 section 1.6 "IMPLEMENTED, TESTED; effectiveness UNVERIFIED"; Chapter 2 judge box "PARTIAL/EXPERIMENTAL for the cluster"; Chapter 8 "TESTED". In Appendix C I used the chapter-4 and chapter-5 labels row by row. A single reconciled label set would help a reader.
3. Chapter 7, section 7.2.1 says `VirtualClock` (`src/lab/clock.rs`) is used only by one test and "Grep of `src/` and `tests/` shows no other user". There is a second, different struct of the same name in `src/sim/clock.rs:16`, used by `src/main.rs:65` and `:197` (`sim::clock::VirtualClock`). The chapter's claim is true of the lab one, but the unqualified grep statement is not true of the name. Suggest "no other user of `lab::clock::VirtualClock`".
4. Two coalescing tests are quoted with different client counts: Chapter 1 and Chapter 2 use `test_m1_t13_barrier_stampede_100_clients` (100 clients), while Chapter 4 section 4.4 (worked example and judge answer) uses "10 simultaneous clients, 1 miss and 9 coalesced". They are different tests, so no error, but a student may conflate them. A one-line pointer would prevent it.
5. Line-number drift against `CLAUDE.md`: `CLAUDE.md` cites `main.rs:327`, `:359`, `:370`, `:374`; the current lines are `:332` (`new_with_cache`) and `:375`, `:383` (anti-entropy workers). The chapters cite symbols, not these lines, so no chapter is wrong. Chapter 4 section 4.12 item 8 (`60_000` at lines 1405 and 1688) is correct.
6. UI test count: Chapter 1 section 1.8 says 298 UI tests; Chapter 8 section 8.2.2 says 298 at 15:29 and 299 after 16:07. Chapter 8 explains the change, Chapter 1 does not.

No factual contradiction was found between the chapters on the facts the architecture appendix depends on: 8 shards, local-only GET, plain rendezvous with constant weight used for DELETE, 204 as primary acknowledgement within 150 ms, 60 s literal with the key NOT WIRED, origin gate enforced only in canary or active with a cap, memory-only repair ledger.
