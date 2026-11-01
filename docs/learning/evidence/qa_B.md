# QA agent B report: Chapters 4 and 5 (2026-10-30)

Scope: `src/ch04_cache_engine_and_algorithms.md`, `src/ch05_distributed_and_intelligence.md`, `assets/ch04_*.svg`, `assets/ch05_*.svg`. Tree: branch `feat/publish-phantomcache-core`, HEAD `df68eaa` (read with `git rev-parse HEAD` during QA). This is an AI check ("AI second look"); nothing here is INDEPENDENTLY VERIFIED.

## 1. Edit log (location | defect | fix | evidence)

| Location | Defect | Fix | Evidence |
|---|---|---|---|
| Ch4 intro (tree-state paragraph) | HEAD stated as `caeedfd` then `df68eaa`; "not re-diffed" | Now "HEAD `df68eaa` at the time of writing; `src/core` re-read in the QA pass" | `git rev-parse HEAD`; QA re-read of cache.rs, eviction.rs, freshness.rs |
| Ch4 4.2 caution (poisoned mutexes) | Omitted that `set_eviction_policy` unwraps shard locks | Added | `cache.rs` `set_eviction_policy`: `shard_mutex.lock().unwrap()` |
| Ch4 4.2 lock order | Edge shard to `eviction_policy` mutex stated, inverse not discussed | Added bullet: `set_eviction_policy` releases the `eviction_policy` guard (temporary) before locking shards, so no inverse edge | `cache.rs` `set_eviction_policy`, `get_eviction_policy` |
| Ch4 4.2 shard-choice bullet | Wordy | Tightened, same facts | n/a |
| Ch4 4.3 first paragraph | Repeated the await rule already given in 4.2 | Condensed | n/a |
| Ch4 4.4 AIMD bullet | Duplicated the AIMD parameters owned by Ch5 5.13 | Pointer to Ch5 5.13; kept the unique fact (cap moves in every mode, `record_observation` has no mode check) | `intelligence/executor.rs` |
| Ch4 4.9 | Said only that `ClockScan` is unused; did not mention the legacy string-keyed scan API (`get`/`insert`/`evict_sieve`) | Added sentence; relevant to the CLAUDE.md "evictors are scans" note | `cache.rs` legacy shard section (`evict_sieve` iterates the map) |
| Ch4 4.5 pinning | `is_pinned` also returns true for a key missing from the map | Added | `cache.rs::evict_with_policy` closure, `.unwrap_or(true)` |
| Ch4 exploration task 2 | "two-line change" for the G4A-01 proposal | "a change of four lines (three added, one replaced)" | `docs/research/g4a01_race/proposal.patch` (3 `+`, 1 `-` content lines) |
| Ch4 4.10 | 9-row table repeated evidence columns of 4.4/4.5/4.8 | Compressed to a 2-column table, same facts and test names | n/a |
| Ch5 intro | HEAD "caeedfd then df68eaa because others committed" | "HEAD `df68eaa` at the time of writing" | `git rev-parse HEAD` |
| Ch5 5.2 handshake bullets | Duplicated Ch6 trust model | Condensed, pointer to Ch6 6.4; all numbers kept | `node.rs` semaphore 64, `pool.rs` PoolConfig 8/4 |
| Ch5 5.3 worked example | Replica figures unverified by anyone but the writer | Added: QA re-compiled and re-ran the replica (rustc 1.98.0): every figure reproduced exactly (see section 3) | scratch `D:\Temp\qa_B\r.rs` |
| Ch5 5.3 weights 1 and 2 | Claim asserted without derivation | Added arithmetic 1 - 1/4 = 0.75 and simulation result | section 3 |
| Ch5 5.3 alternatives | Wordy | Tightened | n/a |
| Ch5 5.4 | "three-line patch" for G4A-01 | "about four changed lines" with the path | proposal.patch |
| Ch5 5.6 G2 | Long; duplicated the Ch6 measurement inventory | Condensed; pointer to Ch6 6.13; numbers unchanged | `claims_ledger.md` line 80 |
| Ch5 5.7 B, 5.7 judge box | Wordy; "15 crash points" | "15 process-level crash tests" (the matrix lists 9 failpoints and 15 process tests) | `m3_durability/remediation/00_status_and_plan.md`, `crash_point_test_matrix.md` |
| Ch5 5.7 C worked example | "89 to 92 polls" | "89 to 91 polls" | `m3_durability/reproduction_report.md` ("89-91 polls"; logs 89, 90, 89) |
| Ch5 5.8 DR-029 | Duplicated Ch6; verbose | Condensed, facts kept | `node.rs` lines 419-454 |
| Ch5 5.9 | Mechanism and failure-model rows verbose | Condensed | `00_status_and_plan.md` |
| Ch5 5.11 | Discrepancy paragraph duplicated Ch6 6.12.2; function name ambiguous (two `calculate_percentiles_us`) | Condensed; names `TelemetryCollector::calculate_percentiles_us`; pointer to Ch6 | `telemetry.rs` lines 354 and 500 |
| Ch5 5.13A | Scenario B bullet duplicated Ch7 7.5.2 | Condensed with pointer | `scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE` |
| Ch5 5.14 | "P99 identical in the other scenarios" is wrong: O-RT was +4.2 percent (below threshold); duplicated Ch7 7.6 | Corrected (five of the other six identical, O-RT +4.2 percent); added pointer to Ch7 7.6 | `docs/research/m6_sealed_results.md` line 8 |
| Ch5 structure | Answers block sat after the questions, "What you will learn" was bold text, not a heading | Answers moved to the end under "## Answers to the knowledge check"; heading added (per style file section 4) | style rules |
| Fig 4.4 `ch04_epoch_race.svg` | t2/t3/t4 box text touched or crossed the box border | Box heights increased | rendered PNG viewed |
| Fig 5.3 `ch05_rendezvous_example.svg` | Replica box title ran outside the box | Title shortened, "re-run by QA" | rendered PNG viewed |
| Fig 5.4 `ch05_invalidation_fanout.svg` | Legend and Source line clipped at the right edge | Moved and re-centred | rendered PNG viewed |
| Fig 5.5 `ch05_failure_timelines.svg` | "89 to 92 stale polls" | "89 to 91" | reproduction_report.md |
| Fig 5.6 `ch05_intelligence_loop.svg` | AIMD box text overflowed its box; OBSERVE box said the histogram is "read by /metrics and the UI", contradicting section 5.11 | Re-wrapped and widened; OBSERVE box now says raw window is read, histogram written not read | telemetry.rs; server.rs grep |
| Fig 5.2 `ch05_mtls_handshake.svg` | Step 7 text overflowed its box | Re-wrapped, box enlarged | rendered PNG viewed |

## 2. Verification summary

References sampled and checked against the current tree: about 200 (symbols, files, constants, G4 ids, test names, line numbers). Fixed: 11 factual items in the table above (poll counts, patch size, O-RT, 15 crash tests, HEAD, test of figure claims, histogram-read claim in a figure, two missing code facts). UNVERIFIED items added: none new; the chapters' own UNVERIFIED markers (I1 to I8 list, runtime behaviour of the histogram, no loom run, literature from memory) were checked and left as they are.

Checked and correct (examples): `PartitionedCache::new(8)` at `main.rs:204`; `DefaultHasher` mask in `get_shard_index`; `MAX_TENANT_EPOCHS = 10_000`; `max_entries_per_shard` 10,000; `max_waiters_per_key` 1,000; background refresh limit 128; `MAX_BODY_BYTES` 2 MiB; 5 s origin timeouts; `60_000` at `server.rs:1405` and `:1688` (chapter's numbers are current; CLAUDE.md's 1077/1352 are stale, as the chapter says); `Wiring::NotWired` at `config.rs:116-117`; frame `HEADER_LEN` 28, payload cap 1,048,576, key cap 4,096; pool 8 total/4 idle/5 s heartbeat; handshake semaphore 64; ledger 1024 entries, 256 KiB, 16 tenant flushes, batch 64; per-peer queue 256; 16 send permits; ring 4,096 records and 4 MiB; 256 gap ranges, 1,024 out-of-order, 256 inbound; histogram 128 + 20 x 128 = 2,688 buckets and the 1,000 us worked example (bucket 506, midpoint 1,002 us, 0.2 percent); count-min 4 x 1,024 u32 = 16 KiB; Space-Saving k = 64; engine constants (60 s cooldown, 500 samples, 0.05, 4 per hour, 1.96); `route_bucket` 1,024/10/50; AIMD 0.70, 5 percent, clamp 256, nearest-rank; `pie: None` at `main.rs:433`; anti-entropy worker 250 ms at `main.rs:375,383`; all 40 test names cited in Ch4 and Ch5 exist (grep of `tests/`); all G4A and G4B ids cited exist and match their topics and dispositions in the register; 99 findings, 75 OPEN in the register; no `NodeState` reference in `pool.rs`.

Complexity claims re-derived from code: lookup O(k) + expected O(1); SIEVE `select_victim` bounded by 2n + 2 inspections (code line `map.len() * 2 + 2`); S3-FIFO bound `2(small + main) + 4`; tenant invalidation O(T), T <= 10,000, because `guard.clone()` runs inside the snapshot write-lock scope (confirmed at `cache.rs` `try_invalidate_tenant`); sweeper scans the whole shard (candidate cap only), confirmed.

## 3. Numbers recomputed

| Item | Result |
|---|---|
| 8 x 7 x 6 x 5 / 8^4 | 1,680 / 4,096 = 41.0 percent; complement 59.0 percent (correct) |
| 0x2F3B & 7 | 0xB = 1011 -> 011 = 3 (correct) |
| Stale window example | 60 + min(300, 86,400) = 360 s; age 400 ineligible; SIE: 60 + 86,400 = 86,460 s, age 90,000 gives CapExceeded (correct) |
| Byte example | 6 + F, +120 = 126 + F, replace 150 -> 176 + F, DELETE -> 0 (correct; code subtracts resource bytes then key + F) |
| 9^5 | 59,049 (correct) |
| SIEVE trace (Fig 4.5) | Insert A..E, hits A and C; call 1: hand A (clear) -> B evicted, 2 inspections; call 2: C (clear) -> D evicted, 2 inspections; A survives. Matches `FaithfulSieve::select_victim` (hand starts at tail via sentinel.prev, moves along `prev` toward newer nodes) |
| Epoch timelines (Fig 4.4, 4.7 example) | Consistent with `begin_fetch_atomic`, `finalize_fetch` (fence at line 1462, second read at line 1517, stamp at line 1532) |
| Waiter limit example | leader registers itself (`server.rs`), so 1,000 admitted, 200 rejected for 1,200 requests (correct arithmetic; not a test result, as the chapter says) |
| Rendezvous replica | Re-compiled (rustc 1.98.0, standalone) and run: scores for the four keys identical to Fig 5.3; owners 33,287 / 33,428 / 33,285; five nodes 20,013 / 19,959 / 20,062 / 19,900 / 20,066; remove node 3 moves 33,285, none between nodes 1 and 2; add node 4 moves 25,002, all to node 4. All reproduced exactly. |
| Weights 1 and 2 | Analytical: P(2u2 > u1) = 1 - 1/4 = 0.75. Simulation, 2,000,000 draws: shortcut 0.7499, logarithmic form (-w/ln u) 0.6668 (2/3). Chapter is correct. |
| AIMD example | floor(32 x 0.7) = 22 (22.4), floor(22 x 0.7) = 15 (15.4), floor(15 x 0.7) = 10 (10.5); 10 to 32 = 22 healthy windows = 2,200 fetches (correct) |
| Count-Min | e/1024 = 0.27 percent; 1 - e^-4 = 98.2 percent; 10,000 x 0.0027 = about 27; 4 x 1,024 x 4 B = 16 KiB (correct) |
| Anti-entropy example (sequence 1,2,3 / 4,5,8) | Gap bookkeeping matches `AntiEntropyTracker` description (not re-run) |
| 4 x 300 + 175 | 1,375 ms (correct; test comments say the same) |
| Rendezvous caveat | u64 to f64 loses 11 low bits (64 - 53); correct |

## 4. Claim check

- Lock order (Ch4 4.2): confirmed as the lead stated. `tenant_epochs` is locked only in `try_invalidate_tenant` (`cache.rs:577`) and `try_invalidate_all` (`:614`), each followed by the `epoch_snapshot` write lock, never under a shard lock. Shard-nested edges: `epoch_snapshot` read, `SharedOp.waiters`, `SharedOp.pending_freshness`, `eviction_policy` mutex. CLAUDE.md's "shard then tenant_epochs" is never exercised. The chapter says this and lists it as a discrepancy (4.12 item 2). No change of substance needed.
- G4A-01: chapter text, figure, judge box, Ch5 4.x cross-reference all say: reproduced only with an injected yield point in a sandbox, no natural reproduction (3,160,958 fills in 20 s), proposal NOT integrated, register status UNDER-INVESTIGATION. Confirmed against `status.md`, the register and the current `finalize_fetch` (second read still present).
- Judge answers read for overstatement: no INDEPENDENTLY VERIFIED anywhere (grep); AI reviews are labelled "AI second look"; honest on incomplete features (S3-FIFO G4A-02, issuer-crash loss, no linearizability or staleness bound, heartbeat not driving state, GetRequest stub, not AI, histogram not the displayed percentile source, canary = 100 percent). No performance-superiority claim. Status labels used exactly.
- "Guaranteed" and "linearizable" wording: only in negations or quoted code comments.
- Remaining soft spots: Ch4 4.2 contention bullet "may take the shard mutex twice" is correct for the HIT path (`server.rs:1319` then `:1520`) except when the bypass branch returns earlier; not worth a change. Ch5 5.5 judge box says the primary can be "the issuing gateway itself": matches `route_key` iterating all members.

## 5. Figures

All 11 SVGs (5 for Ch4, 6 for Ch5) rendered with headless Chrome (own throwaway profile `D:/Temp/qa_prof_B`, deleted afterwards) and each PNG opened and read. Fixed: 4.4, 5.2, 5.3, 5.4, 5.5, 5.6 (see edit log). Figures 4.1, 4.2, 4.3, 4.5, 5.1 were legible with no overlaps and content checked against code (shard count, lock edges, waiter limit 1,000, 32/256 eviction bounds, the SIEVE trace, node-state table, mesh, limits). Residual cosmetic: in Fig 5.2 a lane line passes through the last caption text; legible.

## 6. Consistency of shared facts (verified against code)

8 shards, GET local-only, rendezvous plain with constant weight used only for DELETE, DELETE 204 = local purge plus routed primary's ack only, `PHANTOM_DEFAULT_TTL_MS` NotWired (60 s literal), AIMD enforced only with `PHANTOM_ORIGIN_CAP_MAX` set and canary/active, default mode off, canary = 100 percent, `pie: None`, histogram percentiles have no production reader (raw window with floor index, Ch6 owns detail), G4 99 findings, sandbox patches not integrated, "Guarantees linearizability" comment unsupported: all read identically in Ch4 and Ch5 and match the code.

Conflicts found in OTHER chapters (not edited, reported to the lead):
1. `ch08_limitations_roadmap_defence.md` line 34: "HEAD | `caeedfd`"; current HEAD is `df68eaa` (Ch1 line 3 says `df68eaa`). Needs "at the time of writing".
2. `ch02_architecture.md` line 84: demo `VirtualNode`s have "4 shards each". Verified correct (`node.rs:42`, `PartitionedCache::new(4)`); no conflict, noted because Ch4/Ch5 say "8 shards" for the serving cache only.
3. Ch3 line 322 lock-order sentence agrees with Ch4 (no conflict). Ch7 7.6 and Ch5 5.14 now agree on the M6 P99 line (Ch7 says five scenarios identical, which is accurate).

## 7. Word counts

| Chapter | Before | After | Change |
|---|---|---|---|
| Ch4 | 13,065 | 13,010 | -0.4 percent |
| Ch5 | 16,771 | 16,686 | -0.5 percent |

The 12 percent target was NOT met. Both chapters were already dense: nearly every paragraph carries a number, a status label, a discrepancy or a named test, and the duplicated material left (judge boxes, status tables, the 4.12 discrepancy list, the Key ideas and Knowledge check scaffolding) is mandated by the style rules. Cuts that remove only duplication (pointers to Ch6, Ch7, Ch5 sections, compressed tables) were applied; the QA corrections added back roughly the same number of words. Reaching 12 percent would require deleting technical facts or worked examples, which the brief forbids.

## 8. Remaining concerns

- Ch4 and Ch5 together are about 29,700 words; a reader who wants a shorter handbook should decide which of the Ch4 4.12 and Ch5 5.15 summary tables to drop (they restate content from the body).
- Ch5 5.14 and Ch7 7.6 now both carry the M6 numbers (Ch5 keeps them as a labelled summary with a pointer).
- G4 register counts and the claims ledger are "AI second look"; nothing in either chapter is human-reviewed.
- Several test bodies are cited by name only (Ch4 says so); not analysed by QA either.
