# Chapter 4 evidence file

Chapter: `docs/learning/src/ch04_cache_engine_and_algorithms.md`. Written 2026-10-30 (machine clock). Tree read: branch `feat/publish-phantomcache-core`, HEAD `caeedfd8783a8a7ce69ba1ac53d483734dc0c3c1` when reading began; HEAD was `df68eaab139d` when checked at the end (another actor committed meanwhile; core files not re-diffed), large uncommitted working tree. No cargo build or test was run; no product file was edited; no git write operation was run. Every "verified" below means: opened and read in the current tree. No test was executed by this author, so no TESTED label means "passes"; it means "a named test exercises it and, by reading it, this is what it asserts".

Sources read in full: `src/core/cache.rs` (1883 lines), `src/core/coalesce.rs`, `src/core/eviction.rs`, `src/core/freshness.rs`. Also read: `docs/release/g4/cycle1_core_engine_review.md` (full), `docs/release/g4/cycle1_findings_disposition_register.md` (G4A rows and sections 1 to 2), `docs/research/g4a01_race/{analysis,status,repro_attempts}.md` and `proposal.patch`, `docs/research/eviction_policy_audit.md` (first 60 lines), `src/intelligence/executor.rs` (origin gate and `apply_policy`), `src/intelligence/candidates.rs` (AIMD controller), parts of `src/api/server.rs` and `src/main.rs`, `src/config.rs` registry lines for the cache keys. Tests read (bodies): `concurrency_verification_tests.rs` (model and stress), `coalesce_tradeoffs_tests.rs` (HOL and fan-out), `epoch_contention_tests.rs` (first 110 lines), `invalidation_contract_tests.rs` (lines 28 to 185), `eviction_contract_tests.rs` (lines 40 to 190), `memory_bound_soak_tests.rs::test_waiter_capacity_bound`, `unit/wp1_tests.rs` (flood test header). Other tests are cited by name only (grep of function names).

## Claim table

| # | Claim | Source file::symbol | Status label | How verified | Limits |
|---|---|---|---|---|---|
| 1 | Key is tenant/path/query; missing tenant becomes `__default__`; validation rules (tenant charset and length, path/query caps, control bytes, dot segments, `:?`, canonical cap 4096) | `cache.rs::CacheKey::new`, `validate` | IMPLEMENTED | read lines 15-117 | validate tests not individually read |
| 2 | Canonical round trip tested by property test | `tests/integration/canonical_key_roundtrip_tests.rs` | TESTED (generated inputs) | file exists with `.proptest-regressions`; body not read | not all strings |
| 3 | `CachedResource` stores only status, content type, body, etag, created_at, ttl; other origin headers dropped | `cache.rs::CachedResource`; G4A-07 | IMPLEMENTED; limitation OPEN | read struct; G4A text | |
| 4 | `KeyState` fields and meaning | `cache.rs::KeyState` | IMPLEMENTED | read | |
| 5 | Shard index = DefaultHasher over key, `& shard_mask`; shard count asserted power of two | `cache.rs::get_shard_index`, `new_with_diagnostics` | IMPLEMENTED | read | |
| 6 | Production builds 8 shards; no config key for shard count | `src/main.rs` line 204 `PartitionedCache::new(8)`; `src/config.rs` key list | IMPLEMENTED | read main.rs line; grep of `PHANTOM_*` keys | |
| 7 | `DefaultHasher::new()` keys fixed (deterministic shard choice) | std docs | UNVERIFIED (from memory) | not re-fetched | impact hypothesis only |
| 8 | Collision arithmetic 41.0% / 59.0% | arithmetic 8x7x6x5/8^4 | model, not measurement | computed by hand: 1680/4096 | assumes uniform keys |
| 9 | Per-shard byte budget = total / shards; per-shard entry limit 10,000; default total 268,435,456 | `cache.rs::set_byte_budget`, `new_with_diagnostics`; `config.rs` `PHANTOM_MAX_CACHE_BYTES` | IMPLEMENTED | read | |
| 10 | Lock edges: shard -> epoch_snapshot (read), waiters, pending_freshness, eviction_policy mutex; tenant_epochs -> snapshot write; no nested shard+tenant_epochs | `cache.rs` many | IMPLEMENTED (verified by reading) | read all functions; grep `tenant_epochs` outside cache.rs = none | no loom/shuttle run: deadlock-freedom UNVERIFIED by tool |
| 11 | Hit path takes shard mutex twice (`check_stale_serving` then `begin_fetch_atomic`) | `server.rs` calls at the `check_stale_serving` and `begin_fetch_atomic` lines (1319, 1520 in this tree); `cache.rs` | IMPLEMENTED | read call sites and both functions | cost not measured |
| 12 | HIT response clones body bytes `(*resource.body).clone()` | `server.rs` HIT branch; G4A-26 | IMPLEMENTED | read | |
| 13 | Tokio multi-thread runtime | `main.rs` `Builder::new_multi_thread()` line 174 | IMPLEMENTED | read | |
| 14 | `begin_fetch_atomic` algorithm (hit, epoch drop, join, make room <= 32 evictions, capacity reject, leader registration) | `cache.rs::begin_fetch_atomic` | IMPLEMENTED | read lines 1238-1398 | |
| 15 | Leader registers itself as a waiter, then `into_guard`, then spawns | `server.rs` Initiate branch | IMPLEMENTED | read | |
| 16 | WaiterBatch delivered after the shard lock dropped; Drop delivers | `cache.rs::finalize_fetch`, `coalesce.rs::WaiterBatch` | IMPLEMENTED | read | |
| 17 | 10 clients, 150 ms origin: 1 MISS, 9 COALESCED, elapsed >= 140 ms | `coalesce_tradeoffs_tests.rs::test_negative_effect_head_of_line_blocking` | TESTED (loopback, in-process origin) | read body | not a scale test |
| 18 | Error fan-out: 8 clients, 504 to all | `test_negative_effect_error_fanout_blast_radius` | TESTED | read body | |
| 19 | Waiter limit default 1,000; ~1,000 admitted incl. leader | `cache.rs::max_waiters_per_key` init; leader push in `server.rs` | IMPLEMENTED (arithmetic) | read both | exact count off by one under race with leader push |
| 20 | Waiter limit tested at small values | `serving_contract_tests.rs` (~line 133), `m1_http_tests.rs` (~1203-1255) | TESTED | grep + context lines | bodies not fully read |
| 21 | `test_waiter_capacity_bound` asserts nothing about the bound | `memory_bound_soak_tests.rs` lines 127-156 | verified | read body | |
| 22 | Origin read timeout 5 s | `server.rs` `read_to_end` under `timeout(5000 ms)` | IMPLEMENTED | read lines 2696-2708 | connect timeout elsewhere not analysed |
| 23 | Errors store nothing; no negative caching; backoff only for revalidation (`is_revalidation_due` gates background refresh; `record_revalidation_failure` in SIE/bg paths) | `finalize_fetch`; `server.rs` lines 1339, 1455, 1462, 1783, 1833 | IMPLEMENTED | read finalize; grep call sites | |
| 24 | G4A-08 post-invalidation joiners (code comment "L3") | `cache.rs` ~lines 1290-1314 | IMPLEMENTED defect, OPEN | read the comment and join code | |
| 25 | Origin gate: permit only for leader/bypass/bg-leader; RAII; cap enforced only in Canary/Active with cap set; default mode off; cap unset unless `PHANTOM_ORIGIN_CAP_MAX` | `server.rs::acquire_origin_permit`; `executor.rs::OriginGate`, `origin_cap_enforcing`, `acquire_origin_permit`; `main.rs` 459-478; `config.rs` | IMPLEMENTED | read all | |
| 26 | AIMD step: p99 > ceiling or error rate > 5% -> x0.7 (>= min); else +1 (<= max, clamped 256); starts at max; steps in any mode when cap configured | `candidates.rs::OriginConcurrencyCapController::step`; `executor.rs::record_observation`, `configure_origin_cap` | IMPLEMENTED | read; `record_observation` has no mode check | benefit NOT established |
| 27 | Origin-cap tests exist (names) | `origin_cap_tests.rs` | TESTED (names read; bodies not) | grep fn names | content asserted from names only for the saturation and own-body tests |
| 28 | `finalize_fetch` algorithm, fences, byte checks, second epoch read and stamp | `cache.rs` 1401-1573 | IMPLEMENTED | read; lines match analysis.md (:1462-1466, :1517, :1532-1533) | |
| 29 | Generation fence test | `invalidation_contract_tests.rs::test_delete_fences_in_flight_fill_race` | TESTED | read body | one deterministic ordering |
| 30 | `test_linearizable_delete_prevents_read` single-threaded sequence | same file | TESTED | read body | not a linearizability proof |
| 31 | Abstract protocol model: 9 actions, depth 5, 59,049 sequences; invariants C1 (no stale fill after delete) and C5 (bytes); no invariant on fills after tenant invalidation; mutation detector | `concurrency_verification_tests.rs` | TESTED (model only) | read lines 1-245 | model abstracts away epochs for in-flight fills, so cannot find G4A-01 |
| 32 | Real-cache stress test asserts only ops > 1000 and `audit_bytes` equality, with a fixed `now_ms` | same file ~250-350 | TESTED | read | no epoch-correctness oracle |
| 33 | RAII guards, 3-phase `OriginFetchGuard`, `Aborted` on drop | `coalesce.rs` | IMPLEMENTED | read | |
| 34 | G4A-13 panic cleanup omits byte subtraction and `on_remove` | `coalesce.rs` PHASE_FINALIZING branch | IMPLEMENTED defect, OPEN | read code, confirmed no `bytes` change and no `on_remove` | trigger only the test flag |
| 35 | Pinned = excluded key, in_flight, active ops; all policies check | `cache.rs::evict_with_policy`; `eviction.rs` | IMPLEMENTED | read | |
| 36 | Pinned never evicted (LRU test) | `eviction_contract_tests.rs::test_invariant_i5_pinned_never_evicted` | TESTED (LRU only) | read | not SIEVE/S3 |
| 37 | Byte formula and `audit_bytes` | `cache.rs::calculate_*`, `audit_bytes` | IMPLEMENTED | read | |
| 38 | Byte accounting lifecycle tests | `byte_accounting_tests.rs` names | TESTED (names read; bodies not) | grep names | |
| 39 | I1..I8 individually listed in a source | `ADR-002`, `glossary.md` name them | UNVERIFIED which eight | grep only | |
| 40 | `PartitionedCache::new` leaves byte budget `usize::MAX`; main sets finite | `cache.rs::CacheShard::with_policy`; `main.rs:205` | IMPLEMENTED | read | |
| 41 | MAX_BODY_BYTES 2 MiB | `server.rs` const | IMPLEMENTED | grep | |
| 42 | Epoch structure: snapshot RwLock<Arc<HashMap>>, global AtomicU64, tenant_epochs Mutex | `cache.rs` | IMPLEMENTED | read | |
| 43 | Invalidate costs: delete O(1); tenant bump O(T) clone; all O(T) clear | `cache.rs::try_invalidate_*` | IMPLEMENTED | read | not measured |
| 44 | Snapshot clone happens inside the write-lock scope (`*snap = Arc::new(guard.clone())` with `snap` already write-locked) | `cache.rs` lines 599-605 | this author's reading | read | not in G4A-12 text; impact unmeasured |
| 45 | Overflow fallback at 10,000 tenants raises global epoch, clears map | `cache.rs::try_invalidate_tenant`, `MAX_TENANT_EPOCHS` | IMPLEMENTED, TESTED | read; test name `test_epoch_saturation_fallback_sec4a_preserved` | test body not read |
| 46 | Lazy purge worked example | `begin_fetch_atomic`; `invalidation_contract_tests::test_tenant_flush_ordering_and_lazy_invalidation` | TESTED | read test | 5 ms threshold is host dependent |
| 47 | Sweeper logic, candidate-bounded scan, full shard walk under lock | `cache.rs::sweep_lazy_reclamation`; G4A-03 | IMPLEMENTED defect, OPEN | read code | |
| 48 | G4A-16 validators dropped within a tick | `sweep_lazy_reclamation` expiry window | IMPLEMENTED defect, OPEN | read code | |
| 49 | G4A-01 mechanics and sandbox results (2 of 4 sandbox tests fail unfixed; 4 of 4 pass fixed; 3,160,958 fills, 0 violations in 20 s unhooked; debug; one run) | `docs/research/g4a01_race/*`; `cache.rs` lines | defect CONFIRMED in sandbox only | read status, analysis, repro_attempts, patch; re-opened the code lines | no natural reproduction; no loom; fix NOT integrated; register: UNDER-INVESTIGATION |
| 50 | Main tree still has the second `get_effective_epochs` at the stamp | `cache.rs:1517` | verified | read | |
| 51 | G4A-17 non-atomic epoch pair | `cache.rs::get_effective_epochs` | IMPLEMENTED defect, OPEN (PLAUSIBLE) | read | |
| 52 | Freshness rules and lifetime priority | `freshness.rs::evaluate_response_freshness` | IMPLEMENTED | read lines 139-293 | `freshness_matrix_tests` not read |
| 53 | `PHANTOM_DEFAULT_TTL_MS` NotWired; literal 60_000 at two sites | `config.rs` lines 116-117; `server.rs` 1405, 1688 | verified in code | read registry lines; grep `60_000`, `DEFAULT_TTL`, `default_ttl` | |
| 54 | `PHANTOM_MAX_STALE_MS` wired and default 86,400,000 | `config.rs`, `main.rs:206`, `cache.rs::max_stale_ms` init | IMPLEMENTED | read | |
| 55 | `check_stale_serving` algorithm | `cache.rs` 749-822 | IMPLEMENTED | read | |
| 56 | Cap tests | `stale_serving_tests.rs` names (cap, must-revalidate, SIE, backoff, bg refresh) | TESTED (names; bodies not read, reviewer's table used for what they prove) | grep names; G4A section 5 | counter tautology G4A-18 |
| 57 | Revalidation backoff 1 s doubling to 60 s | `cache.rs::record_revalidation_failure` | IMPLEMENTED | read | |
| 58 | `complete_not_modified` behaviours | `cache.rs` 1025-1082 | IMPLEMENTED | read | `revalidation_304_tests` not read |
| 59 | Conformance gaps G4A-04, -05, -06, -09, -10 | `freshness.rs`, `server.rs`, `diagnostics.rs` | OPEN (AI second look) | G4A-04/05 re-checked by reading `freshness.rs`; G4A-06/09/10 taken from the report, not re-opened | |
| 60 | SIEVE, LRU, S3-FIFO code structure and operations | `eviction.rs` | IMPLEMENTED | read all 700 lines | |
| 61 | `ClockScan` exists, not built by factory, unreferenced elsewhere | `eviction.rs`; grep src and tests | verified | grep | |
| 62 | S3-FIFO tombstone behaviour (`on_remove` leaves queue entries; reinsert duplicates) | `eviction.rs::FaithfulS3Fifo::on_remove`, `on_insert` | IMPLEMENTED defect, OPEN (G4A-02) | read code and traced by hand; no repro test run | trace not executed |
| 63 | S3-FIFO capacity hint inconsistency (1024 vs `max_entries_per_shard`) | `cache.rs::CacheShard::new`, `set_eviction_policy`, `evict_with_policy` | verified | read | |
| 64 | `KeyState.visited` never read by policies; `get_resource*` and stale serves do not call `on_hit` | grep `.visited`; `cache.rs` | verified | grep shows only writes in core | G4A-20 |
| 65 | `select_victim` clones keys per inspection; bounded by 2n+2 (SIEVE), 2(s+m)+4 (S3) | `eviction.rs` | verified | read | |
| 66 | Who calls `set_eviction_policy`: `intelligence/executor.rs` (Canary/Active `apply_policy`, `revert_to_safe`), `control/pie.rs`; none in `main.rs`/`api/` | grep | verified | grep over `src` | whether any normal run selects S3-FIFO: UNVERIFIED |
| 67 | Eviction tests: LRU golden real; SIEVE/S3 weak; O(1) timing test | `eviction_contract_tests.rs` | TESTED (weak) | read bodies | |
| 68 | SIEVE/S3-FIFO/LRU literature descriptions | eviction.rs header cites; audit table | literature, NOT verified from a primary source | not re-read | |
| 69 | Amortized O(1) argument for SIEVE | this author's reasoning | hypothesis (our reasoning) | derived from code | not a repository proof |
| 70 | Memory growth of S3 queues outside accounting | `eviction.rs`; G4A-02/23 | IMPLEMENTED defect | read | |

## Figure verification log

| Figure | What was checked against code |
|---|---|
| 4.1 `ch04_shards.svg` | Fields of `CacheShard` (map, in_flight, bytes, byte_budget, eviction); `shards: Vec<Mutex<CacheShard>>`; mask rule; global fields (`tenant_epochs`, `epoch_snapshot`, `global_epoch`, counters, op id counters); lock edges list matches item 10. |
| 4.2 `ch04_coalescing.svg` | Message order against `begin_fetch_atomic` (leader registration, join with oneshot, waiter limit), the server Initiate branch (leader pushes itself as waiter, spawns task), `finalize_fetch` (detach, take waiters, batch), `WaiterBatch::deliver`. Test numbers from `test_negative_effect_head_of_line_blocking`. |
| 4.3 `ch04_lifecycle.svg` | State transitions and each box against `begin_fetch_atomic` steps 1 to 5 and `finalize_fetch` a to f; exits (delete, epoch drop, TTL/sweeper, eviction); pinned definition; "waiters still get body when rejected" (WaiterBatch built before fences). |
| 4.4 `ch04_epoch_race.svg` | Timeline against `finalize_fetch` lines (check, eviction loop, second read, stamp) and `try_invalidate_tenant` (no shard lock); sandbox facts against `status.md`, `repro_attempts.md` (2 of 4 failing unfixed, 4 of 4 fixed, 3,160,958 fills, 20 s); label "NOT integrated" against the disposition register. |
| 4.5 `ch04_sieve.svg` | List direction and hand movement against `FaithfulSieve::on_insert` (insert at head via sentinel.next, tail = sentinel.prev) and `select_victim` (hand moves via `prev`, clears visited, evicts first unvisited unpinned, wraps to tail); both example calls replayed by hand. |

SVG well-formedness checked with PowerShell `[xml]` load (all five parse). No renderer was run; visual layout (text fit) was estimated from character counts, so overlap is possible and unconfirmed.

## Discrepancies found (document vs code)

1. `CLAUDE.md` section 5 "Faithful Eviction ... O(1)-amortized": SIEVE/LRU O(1) only without pinned keys (G4A-20); worst-case per call O(n); S3-FIFO has G4A-02. `CLAUDE.md` section 6 "evictors named SIEVE/S3-FIFO/LRU are scans" is outdated: current `eviction.rs` uses linked-list/queue structures (`docs/research/eviction_policy_audit.md` records the old scan).
2. `CLAUDE.md` section 5 lock order "shard mutex, then `tenant_epochs`": no code path nests the two. Actual edges listed in item 10.
3. `CLAUDE.md` / `src/config.rs` `PHANTOM_SWEEPER_BATCH_SIZE` described as per-tick shard inspection limit / "shards per tick": code bounds reclaim candidates per shard and scans the whole shard (G4A-03).
4. `invalidation_contract_tests.rs` comment "O(1) Tenant Flush": the epoch bump copies the tenant map (O(T)).
5. `cache.rs` SEC-4A-01 comment "cluster-wide": `global_epoch` is per process.
6. `CLAUDE.md` "Read-path zero contention": only the epoch read; the shard mutex is taken by all paths, twice on a hit.
7. `CLAUDE.md` "`stale_cap_violations` must be 0": satisfied by construction (G4A-18).
8. `CLAUDE.md` line numbers for `60_000` (1077, 1352): now 1405 and 1688; fact holds.
9. `memory_bound_soak_tests.rs::test_waiter_capacity_bound`: name implies a bound test; asserts nothing about the bound.
10. `KeyState.visited`: written, never read by eviction.
11. G4A-12 says the tenant-map clone happens under the `tenant_epochs` mutex; the code also performs it inside the `epoch_snapshot` write-lock scope (this author's reading).
12. The golden-trace test names (SIEVE, S3-FIFO) suggest comparison with a reference trace; they assert only that a hit key is not the first victim.
13. The G4A review quotes a "one-line" fix; the proposal patch changes three lines (declaration, assignment, replaced re-read). Chapter says "small fix".

## UNVERIFIED items

- Whether any normal (non-test) run applies an S3-FIFO eviction candidate (production exposure of G4A-02).
- Which eight invariants are I1 to I8 (not found listed).
- Bodies of: `epoch_contention_tests` after line 110, `stale_serving_tests`, `revalidation_304_tests`, `serving_contract_tests`, `m1_http_tests`, `byte_accounting_tests`, `origin_cap_tests`, `freshness_matrix_tests`, `lab_eviction_tests`, and the fairness tests in `coalesce_tradeoffs_tests`. Chapter cites these by name or by the AI reviewer's description only.
- G4A-06, G4A-09, G4A-10 were taken from the report; not re-opened in `server.rs`/`diagnostics.rs`.
- `DefaultHasher::new()` determinism (std docs recalled from memory).
- Literature for SIEVE, S3-FIFO, LRU, RFC 9111 and RFC 5861 text: not re-read from primary sources.
- Any throughput, latency or contention figure for this engine: none quoted; none verified.
- Visual rendering of the SVGs.

## Cross-chapter notes

- Chapter 5 should state the intelligence mode default (`off`) and who may call `apply_policy`; this chapter only says no call from `main.rs`/`api/` was found.
- Chapter 5 owns peer-driven invalidation (G4A-12 / G4B-05 / DR-027); this chapter only describes the local epoch overflow fallback.
- Chapter 6 should reuse: shard choice is deterministic; per-shard budget skew; `CapacityExceeded` from pinned keys; body 2 MiB; percentiles G4A-24.
- Chapter 7 owns the AIMD benefit discussion; this chapter states only the mechanism and that it starts at its maximum.
- Chapter 8 should carry the G4A-01 status line: UNDER-INVESTIGATION, sandbox-confirmed, fix not integrated.
