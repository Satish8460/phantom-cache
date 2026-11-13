# Short Guide verification record (2026-10-30, AI-prepared, human review pending)

**Result.** PHANTOMCACHE_SHORT_GUIDE.pdf: 12 pages (counted as `/Type /Page` objects by build_short.mjs; also seen as 12 images). Ceiling was 15, so 3 pages are unused (see end). Words: 5,710 (`wc -w` on the .md, including title block and tables).

**Overflow probe.** Headless Chrome (own throwaway profile, repo CDP helper `ui/scripts/cdp-lib.mjs`), print media emulated, viewport 665: clientWidth 650, scrollWidth 650, body right edge 650, elements with right edge beyond 650: none.

**Pages opened as images (all 12, final build), defects found and fixed.**
Pages 1 to 12 were opened through Chrome's PDF viewer (page-fit) and read. Defects fixed: (a) architecture figure was too small to read at 64 percent width, enlarged to 92 percent; (b) a third figure (epoch race) left half a page blank and its text was illegible at the size that fits, so it was removed; (c) the fan-out figure enlarged to 94 percent; (d) bold lead-ins stranded at page bottoms ("What the evidence supports", "More questions to rehearse") converted to level-3 headings that keep with the next block; (e) a near-empty last page was filled with the extra questions, six habits and key terms; (f) the roadmap table cut from 23 to 19 rows. Remaining cosmetic points, accepted: the section 6 and section 8 tables continue across a page break with a repeated header row; page 8 and page 4 end with short white gaps; self-test item 10 falls on page 12.

**Figures used (2 of at most 3).** `../assets/bm02_architecture_overview.svg` (section 2) and `../assets/ch05_invalidation_fanout.svg` (section 4), both unedited, by relative path. No new SVG was needed. The figure texts carry some numbers I did not re-verify (queue of 1,024 broadcaster tasks, 16 permits, 1,024 ledger entries, "repair loop every 250 ms"); the guide's prose does not repeat them. See discrepancy 2 below.

**Facts re-verified in the current code (file::symbol, line).**
1. src/main.rs::run: `PartitionedCache::new(8)` (204).
2. src/core/cache.rs: `shard_mask: num_shards - 1` (324); hash `& self.shard_mask` (549, 557).
3. cache.rs: `max_waiters_per_key` initial 1_000 (331); `OriginError::WaiterLimitExceeded` returned from `begin_fetch_atomic` (1238, 1305).
4. cache.rs: default `EvictionPolicy::Sieve` (332).
5. cache.rs: eviction loop bound `evictions_done < 32` (1328).
6. cache.rs::finalize_fetch: store only if `state.generation == op_gen` and both entry epochs equal `get_effective_epochs` (1401 to 1465); `stale_writes_prevented` incremented (1472).
7. cache.rs::delete_impl: `generation.checked_add(1)` (1586 ff.).
8. cache.rs: `MAX_TENANT_EPOCHS = 10_000` (273) and full-map check in `try_invalidate_tenant` (576, 581).
9. cache.rs::get_effective_epochs, check_stale_serving, audit_bytes exist (560, 749, 1827).
10. src/config.rs: `PHANTOM_DEFAULT_TTL_MS` default 60000 (116); `PHANTOM_MAX_STALE_MS` default 86400000 (119).
11. src/api/server.rs: literal `60_000` passed at two sites (1405, 1688).
12. config.rs: `PHANTOM_CLUSTER_TIMEOUT_MS` default 150 (159); `PHANTOM_INTELLIGENCE` default off (161); `PHANTOM_ORIGIN_CAP_MAX` (163); `PHANTOM_SWEEPER_INTERVAL_MS` default 100 (123); `PHANTOM_MAX_CACHE_BYTES` default 268435456 (114).
13. src/dist/router.rs::route_key: `DefaultHasher` (1344) and `capacity_weight = 1.0` (1363, 1381).
14. router.rs::forward_key_over_tcp defined (1420); only callers are in tests/integration/p2_dist_network_tests.rs (no call under src/).
15. src/main.rs: `pie: None` (433).
16. src/api/server.rs: `DefaultBodyLimit::max(512 * 1024)` (220); `MAX_BODY_BYTES = 2 MiB` (2317).
17. src/dist/node.rs: `format!("val_node_{}", node.node_id)` in the GetRequest arm (439, 447).
18. server.rs::delete_cache_handler (1937) calls `invalidate_key_across_cluster` (1995); `PrimaryFailover` header (2006).
19. src/security/jwt.rs: HS256 only, `header.alg != HS256` rejected (136, 153); roles `cache:read`, `cache:invalidate` (6, 8).
20. src/intelligence/candidates.rs: decrease factor 0.70 (158).
21. src/intelligence/executor.rs::origin_cap_enforcing: Canary or Active and cap configured (217).
22. src/intel/telemetry.rs: `HISTOGRAM_TOTAL_BUCKETS` commented 2,688 (274 to 278).
23. router.rs: `PER_PEER_QUEUE = 256` (509); `start_anti_entropy_worker` (1000), called with 250 ms in main.rs (375, 383).
24. src/lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE: fell from 32 to about 12 (559).
25. docs/research/m6_sealed_results.md: H1 NOT SUPPORTED (line 6), 30 sealed seeds (14).
26. tests: test_delete_fences_in_flight_fill_race (tests/integration/invalidation_contract_tests.rs:79); test_m1_t13_barrier_stampede_100_clients (tests/integration/m1_http_tests.rs:612).
27. src/core/eviction.rs::FaithfulSieve with a `visited` flag (49, 44).
28. server.rs: X-Cache-Status values HIT (1556), STALE (1498), COALESCED (1598), MISS (1892), BYPASS-AUTH (1280).

**Discrepancies between the handbook and the code (report; the guide uses the code's truth).**
1. The handbook says `forward_key_over_tcp` "has no caller". It has no production caller but is called from tests/integration/p2_dist_network_tests.rs. The guide says "no production caller (only tests call it)".
2. The handbook (and the fan-out figure) say the repair loop runs "every 250 ms". I found the 250 ms constant only for the anti-entropy worker (main.rs:375, 383); the repair loop in router.rs uses a backoff schedule (around lines 2050 to 2110). The guide says "a repair loop with backoff" and "250 ms tick" for anti-entropy only. The figure still shows the handbook's wording.
3. Handbook line numbers for the 60_000 literal differ slightly (code: 1405, 1688); CLAUDE.md says about 1077 and 1352. The guide gives no line numbers for this.

**Not re-verified in code (taken from the handbook as stated there).** The G4 finding counts and dispositions (99; 75 open; and so on), High findings, G4B-xx and F-xx descriptions; the 851 recorded debug tests; the 288 locked packages and two TLS stacks; Scenario B "static arm peaked at 24 in all 10 runs" and its release-build record; the 3,160,958 fills and 20 s stress run for G4A-01; frame limits (1 MiB payload, 4,096-byte keys), ring of 4,096 records, queue and attempt counts beyond PER_PEER_QUEUE; 5 s phase timeouts; the 100 ms sweeper tick (config default checked, not the loop); that U-01 integration is "in progress and not accepted"; that the 3 demo VirtualNodes exist (main.rs:207 to 209 per CLAUDE.md). No test, cargo command or sealed-evaluation file was run or read.

**Space unused.** Three pages remain under the 15-page ceiling. They would have held: a legible full-size epoch-race figure (it did not fit without a blank half page), a request-sequence figure for hit, miss and coalescing (handbook Ch 2 figures), a fuller G4 table (more than 12 rows), and two or three more judge answers from Appendix E (for example on SSRF, byte accounting and the UI).

**Build.** `node docs/learning/short/build_short.mjs` (Node built-ins; pandoc at C:/Users/Satish/AppData/Local/Pandoc/pandoc.exe; reads only the short guide .md; uses ../build/handbook.css by reference plus short.css). It also writes PHANTOMCACHE_SHORT_GUIDE.html here as an intermediate. Nothing outside docs/learning/short/ was written (the scratch folder D:\Temp\short_pages holds probe scripts and page images). Git HEAD 804a7b8d and branch feat/publish-phantomcache-core were unchanged; no git write command was run.
