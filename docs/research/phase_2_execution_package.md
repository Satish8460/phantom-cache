> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work against this package was executed and reported in [phase_2_completion_report.md](phase_2_completion_report.md) (agent-reported PASS; supported status tested, acceptance criteria not demonstrated). No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 2 Execution Package — M2: Core Correctness, Performance & Freshness

**Status: PROPOSED — not authorized.** Authorization requires the owner's written decision after G-M1 (Phase 1 accepted). Common rules: `phase_execution_packages_index.md` §4 (CER-1…CER-11); this package adds phase-specific requirements. Everything labeled **PROVISIONAL** is resolved at step M2-00 against real Phase 1 evidence.

## 1. Purpose, scope, non-goals, expected outcomes
**Purpose.** Turn the single-node gateway into an engine whose behavior is *specified, tested and measured*: a written serving-path contract (coalescing, cancellation, timeouts), HTTP freshness and bounded stale serving, defined invalidation semantics, verified byte-bounded memory, verified concurrency safety, honest eviction implementations, and only those performance changes that measurements justify.
**In scope.** `src/core/*` (cache, coalescer), the origin-fetch and response path in `src/api/server.rs`, `src/intel/telemetry.rs`, freshness/stale configuration in `src/main.rs`, the telemetry schema additions, tests/benches/docs. **Out of scope / non-goals:** distributed behavior (M3); the lab and baselines (M4); any adaptive logic or enabling the PIE (M5); research claims (M6); packaging (M7); UI redesign (M8) beyond keeping the Phase-1 UI truthful for new fields.
**Expected outcomes.** (1) ADR-002 "Serving-path contract" with a test per clause; (2) freshness engine with a documented RFC 9111 supported subset and a hard staleness cap; (3) single-node invalidation contract; (4) eviction implementations whose names match their algorithms (or honest names); (5) every optimization either VALID-improved or DROPPED with evidence; (6) schema v1 retired or formally deprecated; (7) the legacy PIE quarantined.

## 2. Starting state (evidence-backed; PROVISIONAL items re-validated at M2-00)
**Verified before Phase 1 (read by the research lead, line numbers may have shifted):** TTL is a fixed 60,000 ms literal and no `Cache-Control` is parsed ([A] `server.rs:638`; `node.rs:413` [V]); no stale serving exists; fill/delete are generation-fenced (`cache.rs:615-712`) [V]; `begin_fetch_atomic` coalesces through `SharedOp` with `max_waiters_per_key` (default 1,000) [V]; eviction functions are O(shard) scans named SIEVE/S3-FIFO/LRU (`cache.rs:348-479`) [V]; every hit/miss takes the global `tenant_epochs` mutex while holding the shard lock (`cache.rs:251-259`) [V]; telemetry uses one global `Mutex<VecDeque>` and sorts for percentiles; latency has 1 ms resolution (hits record 0) [V]; the upstream fetch uses a 5 s connect timeout ([A] capability audit; read/total timeouts unknown) and a 2 MiB body cap ([A]); `RawObservation` is built by struct literal in `tests/unit/wp2_tests.rs` [V], so its fields cannot change without editing that test; `pie: None` in `main.rs` [V].
**Expected from Phase 1 (PROVISIONAL — do not assume it succeeded):** telemetry schema v2 with provenance, DELETE excluded from latency/hit statistics, origin metrics, byte accounting with invariants I1–I8, reference policies with golden traces, VALID Core baselines on ≥1 platform, a documented noise bound, clippy/whitespace baselines. **M2-00 reads the Phase 1 completion report and records which of these actually hold.**
**Unknowns that must be settled by reading code in M2-01:** where the origin fetch runs relative to the client connection (cancellation), read/total timeouts, how `Uncacheable` outcomes are produced, how `Vary`, `Set-Cookie` and `Authorization` are currently handled, which status codes are stored.

## 3. Work packages
Legend: **M** mandatory · **R** rule-gated (proceeds only if its rule fires on VALID Phase 1/2 data; otherwise recorded DROPPED with evidence) · **C** conditional. IDs map to earlier WP ids.
| ID | Title (legacy) | Type | Depends on |
|---|---|---|---|
| M2-00 | Entry verification and reconciliation | M | G-M1 |
| M2-01 | Serving-path contract audit (coalescing, cancellation, timeouts) | M | M2-00 |
| M2-02 | Freshness engine — `Cache-Control`, validators, revalidation (WP-14) | M | M2-01 |
| M2-03 | Stale-serving contract and hard staleness cap (WP-14) | M | M2-02 |
| M2-04 | Single-node invalidation contract and lazy-epoch reclamation | M | M2-01 |
| M2-05 | Latency measurement resolution and telemetry additions | M | M2-00 |
| M2-06 | Eviction audit, policy interface, faithful implementations (WP-08) | R (naming honesty is M) | M2-00 |
| M2-07 | Epoch lookup contention (WP-07) | R | M2-00 |
| M2-08 | Telemetry hot-path redesign (WP-09) | R | M2-05 |
| M2-09 | Concurrency verification (WP-23) | M | M2-02, M2-04 |
| M2-10 | Memory-bound soak and resource-bound verification | M | M2-02…M2-04 |
| M2-11 | Legacy PIE quarantine (WP-22a) | M | M2-00 |
| M2-12 | Ingress/upstream parser robustness | M | M2-00 |
| M2-13 | Schema v1 retirement and UI migration | C (owner O2-7) | M2-05 |
| M2-14 | Documentation: RFC 9111 subset, ADR-002, invariants | M | all |
Order: M2-00 → {M2-01, M2-05, M2-11, M2-12} → {M2-02 → M2-03, M2-04, M2-06, M2-07 (after paired baselines), M2-08} → M2-09 → M2-10 → M2-13 → M2-14. Files that are edited by several packages (`cache.rs`, `server.rs`) are edited **sequentially**, never in parallel: order within shared files M2-04 → M2-02/03 → M2-06 → M2-07.

### M2-00 Entry verification (CER-2) + decision-rule freeze
Record the real Phase 1 outcomes (test counts, clippy/whitespace baselines, Core VALID/UNFIT table, noise bound, dev-dependencies added, A-number reconciliation between `phase_1_progress.md` and the v3.1 package). **Freeze the rule-gated thresholds** below *before* any M2 optimization measurement (PROVISIONAL defaults, owner may amend once, in writing): **R-07 fires** if the isolated `get_effective_epochs` cost is ≥ 10% of the full-hit median, **or** multi-thread scaling efficiency at 8 threads is < 60% of linear **and** the tenants=1 vs tenants=1000 comparison separates (CIs disjoint); **R-08 fires** if the miss-path eviction-trigger latency p99 at 10,000 entries/shard exceeds 20× the full-hit median, or grows with shard size with log–log slope > 1.2 (CI-separated), or the production-vs-reference miss-ratio gap on seeded Zipf/scan traces exceeds 2 percentage points; **R-09 fires** if `record()` p99 at 8 threads exceeds 5% of the full-hit median or `calculate_percentiles` at the production buffer size exceeds 1 ms. Naming honesty (M2-06) is mandatory regardless. **Acceptance:** E0 report; frozen-rule file hashed in the progress log.

### M2-01 Serving-path contract audit
**Deliverable: ADR-002** with clauses CT-1…CT-10, each = statement + code location + test id + status (VERIFIED / GAP / DESIGN-CHOICE). Clauses to resolve (not assumed): CT-1 single-flight (≤1 origin fetch per key per fencing epoch); CT-2 **initiator cancellation** (client disconnects while the origin fetch runs: does the fetch continue, who completes waiters, can a waiter become the new leader?) — PROVISIONAL required behavior: the origin fetch is **detached from the client connection** and completes/fails waiters; the fill is stored if fencing allows (O2-2 owner decision if the current behavior differs); CT-3 waiter bounds and per-request deadline; CT-4 origin timeout classes (connect, first byte, total) and their mapped status codes/headers; CT-5 panic/abort safety (guard drop notifies waiters; no leaked `in_flight`/`active_op_ids`); CT-6 errors are not cached (or documented negative caching); CT-7 fencing under delete/tenant/global invalidation; CT-8 body-size and header limits with chunked/Content-Length; CT-9 expiry herd behavior; CT-10 resource bounds (waiters, in-flight keys, body bytes).
**Tests (each clause):** deterministic async tests with a controllable mock origin (hold/release, delay, reset, partial body, slow trickle), cancellation tests (drop the client request at defined points), timeout tests, panic-injection test (existing `test_panic_after_waiter_extraction` hook is the pattern), a leak assertion (`in_flight == 0`, `active_op_ids` empty, gauge `origin.in_flight == 0`) after every scenario. Time control: use real timers with margins ≥ 10× jitter; `tokio` `test-util` pause/advance requires a dev feature (O2-5 — owner-reserved; not assumed).
**Acceptance:** every clause has a test and a status; every GAP has a fix in this phase or a recorded owner-accepted limitation.

### M2-02 Freshness engine (WP-14)
**Parser (no new dependency; in-house, total-function, fuzzed in M2-12).** Supported subset (PROVISIONAL, documented in M2-14): response `Cache-Control`: `no-store`, `private` (shared cache ⇒ do not store), `no-cache` (store, freshness lifetime 0, requires conditional revalidation), `max-age`, `s-maxage` (precedence for shared caches), `must-revalidate`/`proxy-revalidate` (forbid stale use), `public`, `stale-while-revalidate`, `stale-if-error`; `Expires` and `Date` when no max-age/s-maxage; `Age` incorporated into current age (`current_age = max(age_header + resident_time, now − date)` using the injected `now_ms`); `ETag`/`Last-Modified` stored as validators; invalid or duplicate directives ⇒ treated as stale/uncacheable per RFC 9111 §4.2.1 (conservative). Not supported (explicit, documented): `Vary` secondary keys, heuristic freshness, range/partial content, request `Cache-Control` from clients (ignored except where the gateway's REST contract says otherwise), `HEAD`.
**Policy decisions.** `Set-Cookie` responses and responses to requests that forwarded `Authorization` are stored only if explicitly shared-cacheable (`public`, `s-maxage`, or `must-revalidate`) — PROVISIONAL; `Vary` present ⇒ uncacheable except `Vary: Accept-Encoding` handled by normalizing request encoding (O2-3 owner decision on scope); stored status codes: 200 by default, 301/404/410 only with explicit freshness (PROVISIONAL, confirm against current code); **default when no freshness information: keep the current fixed 60 s behavior unless the owner decides otherwise (O2-1)** — the config key (e.g., `PHANTOM_DEFAULT_TTL_MS`) makes it explicit. The freshness mode itself is gated by a flag so the old behavior remains selectable until the owner approves the new default.
**Revalidation.** Stale entry with validators ⇒ conditional request (`If-None-Match` / `If-Modified-Since`), coalesced through single-flight like any fetch; `304` updates freshness metadata (documented header merge), keeps body and byte accounting unchanged; `200` replaces; origin errors follow M2-03.
**State additions** in `KeyState`/`CachedResource` (e.g., `expires_at`, `validators`, `stale_until`, `must_revalidate`): any struct literal change must keep existing tests compiling or be listed as an owner-reserved test edit (O2-10).
**Tests.** A freshness matrix (table-driven, ≥ 60 header combinations incl. invalid inputs) asserting stored/not-stored, lifetime, revalidation requirement; property tests: `current_age ≥ 0`, monotonic with `now`, `fresh ⇒ age < lifetime`; conditional-request tests against the mock origin (304/200/5xx); coalescing of revalidation; accounting invariants (Phase 1 I1–I8) still hold when entries are updated by 304.
**Acceptance:** matrix 100% PASS; RFC 9111 supported-subset statement drafted (M2-14); with the flag off, behavior is byte-identical to Phase 1 end-state on the existing 242+ tests.

### M2-03 Stale-serving contract and hard cap
**Contract.** An entry may be served stale only if (a) `stale-while-revalidate` / `stale-if-error` (or a configured default, O2-1) permits it, (b) `must-revalidate` is absent, and (c) `age ≤ lifetime + min(directive window, PHANTOM_MAX_STALE_MS)` — **the hard cap is enforced in the execution layer and cannot be exceeded by any configuration of later intelligence.** Responses served stale carry `Age` and a distinct cache status (e.g., `X-Cache-Status: STALE`, name PROVISIONAL, must be added to UI/schema). Background revalidation: at most `max_background_refresh` in flight (bounded), single-flight per key, failure backoff (PROVISIONAL: initial 1 s, ×2, cap 60 s) so a failing origin is not hammered; after the cap the entry is a miss.
**Telemetry (schema v2 additions, each with provenance):** `stale_served_total`, `stale_age_ms` distribution (derived, windowed), `revalidation_{started,ok,not_modified,error}`, `background_refresh_in_flight`, `stale_cap_violations` (must be 0 — a counter that proves the invariant in production).
**Tests.** Virtual-time property test (injected `now_ms`): ∀ sequences of requests, fills, deletes, origin outages: served age ≤ cap; no stale after `must-revalidate`; delete always wins; origin outage scenario (mock origin returns 5xx/resets) shows SIE behavior and the cap; bounded background refresh under 10,000 concurrent stale requests (origin sees ≤ bound); backoff schedule test.
**Acceptance:** `stale_cap_violations == 0` in all tests including soak (M2-10); zero origin storms in the storm test (origin calls ≤ bound × windows).

### M2-04 Single-node invalidation contract and reclamation
**Contract (to be written in ADR-002, tested):** after `DELETE` returns, no subsequent request on this node observes the pre-delete entry (per-node linearizable); in-flight fills started before the delete are fenced (dropped, counted); tenant and global invalidation are O(1) lazy; **lazy-invalid entries are reclaimed** by a bounded background sweeper (budget: ≤ N entries per tick per shard, lock held per entry batch; PROVISIONAL N and tick) so `bytes_resident` converges after a tenant flush — the sweeper is a new background task and therefore needs the owner's confirmation (O2-8) and a measured lock-hold bound. Epoch-map saturation fallback (SEC-4A-01) is retested after any change.
**Tests.** Existing SEC-4A suite unchanged; new: delete-vs-fill race (barrier-driven), tenant-flush then fill ordering, sweeper convergence (bytes return to expected within a bounded number of ticks), sweeper vs concurrent fills (stress), accounting invariants I1–I8 under sweeping.
**Acceptance:** all pass; sweeper's max lock-hold measured (HDR) and below the bound frozen at M2-00 (PROVISIONAL: ≤ the hit-path p99.9 noise level or ≤ 1 ms).

### M2-05 Latency resolution and telemetry additions
Record request latency with sub-millisecond resolution (µs) **without changing existing public struct fields used by tests**: add a new recording method/field set alongside the 1 ms path (O2-10 if any existing test must change). Keep v1/v2 `latency_ms` outputs (derived from µs) and add `resolution` metadata to the schema. Add the M2-03 counters. **Tests:** unit tests for percentile accuracy vs exact on seeded distributions; schema contract tests (no `simulated` in live; unavailable has no `v`). **Acceptance:** hits show non-zero sub-ms percentiles in a loopback test; overhead measured with the Core items (class VALID) and within the noise bound or recorded.

### M2-06 Eviction audit, policy interface, faithful implementations (WP-08)
**Mandatory audit (naming honesty):** table of each production eviction function vs the published algorithm it names (structures, per-operation complexity, lock-hold time, known behavioral differences), and the Phase 1 miss-ratio comparison to the reference implementations. **Decision:** if R-08 fires, implement faithful, O(1)-amortized policies behind a per-shard trait (SIEVE: FIFO queue + hand + visited bit; S3-FIFO: small ≈10% / main / ghost queues with 2-bit counters; LRU: O(1) list; parameters taken from the Phase 1 WP-05 documentation of the primary sources); if R-08 does not fire, **rename** the production policies to honest names (e.g., `ClockScan`) in code, config values and docs, with a deprecation alias (O2-4 owner decision on rename vs implement). The trait is also the contract M5 depends on: `on_insert`, `on_hit`, `on_remove`, `select_victim(is_pinned)`, `rebuild_from(entries)` for switching, bounded work, pinned states never selected (Phase 1 I5).
**Tests.** Phase 1 golden traces through the production trait; property tests (capacity never exceeded, pinned never evicted, deterministic given a trace); stress with concurrent fills/deletes; accounting invariants. **Benchmarks.** B4 (eviction pressure) and Core hit items before/after, paired, interleaved ABAB in one session, class VALID. **Acceptance:** miss ratio within the frozen tolerance of the reference (PROVISIONAL ≤ 0.5 pp absolute on the seeded traces) and eviction-trigger p99 not worse (CI) — or the package is DROPPED and names fixed.

### M2-07 Epoch lookup contention (WP-07) — R
**Options (choose after data):** (a) read-mostly `RwLock<HashMap>`; (b) per-shard epoch cache refreshed by a version counter; (c) copy-on-write snapshot via `Arc` swap *implemented with std only*; (d) fixed array of atomic epochs indexed by a **keyed** hash of the tenant — **rejected unless the owner accepts the cross-tenant collateral-invalidation risk** (an attacker could force invalidation of colliding tenants; also changes SEC-4A semantics). Any crate such as `arc_swap` is a runtime dependency ⇒ owner-reserved (O2-9).
**Invariants:** entry served only if its tenant/global epochs equal the current ones; no stale resurrection (SEC-4A tests unchanged); lock order documented. **Tests:** SEC-4A suite, new invalidate-vs-hit/fill stress, protocol model test (M2-09). **Benchmarks:** B1 isolated epoch lookup, B2 scaling (tenants {1,1000}); VALID paired comparison. **Acceptance:** improvement CI excludes the noise bound with unchanged semantics, else DROPPED (finding recorded in the ledger as "no measurable effect").

### M2-08 Telemetry hot-path redesign (WP-09) — R
Replace the mutex+VecDeque+sort with sharded per-thread or atomic log-linear histograms (documented relative error ≤ 1%), windowed via rotating buckets; the old API remains for tests. **Invariants:** counts exact; percentile error bounded and tested against exact; no lock on the hit path. **Tests:** accuracy vs exact (property), concurrency (counts conserved under N writers), window rotation correctness. **Benchmarks:** B6 before/after, B8e (telemetry on/off). **Acceptance:** per R-09 and VALID comparison, else DROPPED.

### M2-09 Concurrency verification (WP-23)
**Approach (owner O2-5):** (1) **protocol model tests** — a faithful small model of fill/delete/finalize/coalesce/epoch/eviction-pin interleavings implemented in test code and explored exhaustively (with `loom` or `shuttle` if the owner approves that dev-dependency; otherwise a bounded exhaustive enumerator written in the test) checking invariants: no stale fill stored after delete, no double-fill, waiters always notified, no pinned eviction, bytes accounting equality; (2) **stress tests on the real code** with randomized schedules and fixed seeds, long enough to give coverage evidence (iterations recorded); (3) `proptest` state-machine tests of the public cache API against a reference model. Making real code `loom`-checkable requires a `crate::sync` facade touching many files — **not assumed**; an explicit proposal is part of the report if the owner wants it.
**Acceptance:** all invariants hold in model + stress + proptest runs (counts recorded); a deliberately injected bug (mutation: skip generation check) is caught by each method (proves the tests can fail).

### M2-10 Memory-bound soak
Mixed workload (Zipf α=0.8 over 1M keys, bodies 1 KiB–1 MiB heavy-tailed, 5% invalidations incl. tenant flushes, TTL churn, stale serving enabled, injected origin faults) for the frozen duration (PROVISIONAL 2 h per platform; 4 h if the platform is fit). **Measures:** accounted bytes (≤ budget at all times), RSS (Linux), map sizes, in-flight/waiter gauges, `stale_cap_violations`, panics (must be 0), latency tail drift. **Leak criterion (PROVISIONAL):** fitted RSS slope after warm-up has a 95% CI upper bound ≤ 1 MiB/h and accounted/RSS ratio stable; outcomes classed per CER-5. **Acceptance:** PASS only for VALID runs; an environment-unfit platform ⇒ BLOCKED, not PASS.

### M2-11 Legacy PIE quarantine (WP-22a)
Keep `src/control/pie.rs` compiled and untouched in behavior; add a documented *LEGACY — not the PIE* banner (doc comments only), CLAUDE.md/CORRECTIONS/README pointers, UI label (already "no adaptive controller"). Feature-gating the module or editing `p4_pie_tests`/`wp2_tests` is a test/build change ⇒ **O2-6**. **Acceptance:** `pie: None` unchanged; hash of `pie.rs` differs only by comments (or unchanged); no new caller.

### M2-12 Ingress/upstream parser robustness
Property-based and table-driven tests (structure-aware generators with `proptest`) for the in-house HTTP/1.1 response parser and chunked decoder (`decode_chunked_body`, header limits, smuggling-style ambiguities: duplicate `Content-Length`, `Transfer-Encoding` + `Content-Length`, obs-fold, oversize chunks, truncated bodies), plus ingress path/query/tenant validation limits. `cargo-fuzz` requires nightly/tooling ⇒ O2-9 (not assumed). **Acceptance:** zero panics, bounded allocation (assert max allocated bytes per input class), conformance table PASS.

### M2-13 Schema v1 retirement (conditional, O2-7)
Retire v1 only with the owner's approval (breaking API change): migrate tests asserting v1 shape (list them), keep `?schema=1` returning HTTP 410 or the frozen payload with `Deprecation` header (owner chooses). UI already consumes v2 (Phase 1). **Acceptance:** list of migrated tests, no v1 consumers left in repo (grep evidence).

### M2-14 Documentation
ADR-002 (serving-path contract); `docs/research/rfc9111_supported_subset.md` (supported/unsupported, tested clause table); update `CLAUDE.md` invariants (stale cap, byte accounting, eviction trait), CORRECTIONS (what changed), config reference (all new env/flags with defaults and units), schema catalog additions; claims ledger entry C5 outcome.

## 4. Acceptance criteria and exit gate G-M2 (CER-4 semantics)
| ID | Criterion | Evidence | Blocking |
|---|---|---|---|
| AC2-1 | E0 complete; Phase 1 actual results recorded; decision rules frozen before measurement | M2-00 report, hash | yes |
| AC2-2 | ADR-002 clauses CT-1…CT-10 each VERIFIED or owner-accepted limitation; tests pass | test list, ADR | yes |
| AC2-3 | Freshness matrix PASS; supported-subset doc; flag-off behavior identical to Phase 1 end state | tests, diff | yes |
| AC2-4 | Stale contract: `stale_cap_violations == 0` across tests+soak; background refresh bounded; backoff verified | tests, soak | yes |
| AC2-5 | Invalidation contract + sweeper (if approved) verified; SEC-4A unchanged | tests | yes |
| AC2-6 | Eviction naming honest (implemented faithfully or renamed); R-08 decision recorded with data | audit table | yes |
| AC2-7 | Each rule-gated optimization: VALID improvement (CI excludes noise bound) or DROPPED with VALID evidence; **no optimization merged on EXPLORATORY data** | paired benchmark tables | yes |
| AC2-8 | Core hit-path items show no regression beyond the noise bound vs Phase 1 end state, or the owner accepted it | VALID comparisons | yes |
| AC2-9 | Concurrency verification (model + stress + proptest) PASS and mutation check catches the injected bug | outputs | yes |
| AC2-10 | Soak PASS (VALID) on ≥1 platform; BLOCKED if no fit platform | soak report | yes |
| AC2-11 | Parser robustness PASS; no panic on adversarial inputs | tests | yes |
| AC2-12 | PIE quarantined; `pie: None` unchanged | grep, hash | yes |
| AC2-13 | No runtime dependency added without O2-9; tree/ledger/AC-1-style comparison vs the phase snapshot clean | `cargo tree -e normal`, ledger | yes |
| AC2-14 | Documentation delivered and consistent | docs | yes |
**G-M2 passes** when all blocking criteria are PASS (or have a written owner waiver) and the owner accepts after the research lead's review. **Disabled until G-M2:** none of the M2 behavior defaults changes for users (O2-1); M4 baselines that depend on stale serving and M5 start only after G-M2.

## 5. Benchmark methodology (reuses the Phase 1 harness)
Same classes, manifests, VALID/EXPLORATORY rules and Core items as Phase 1 (§6.3.5 there). Comparisons are **paired and interleaved (ABAB) within one session** to cancel drift, with the same seeds; effect = ratio of medians with percentile-bootstrap 95% CI over ≥ 10 process repetitions; "improvement" requires the CI to exclude the noise bound frozen at M2-00; "no regression" requires the upper CI bound ≤ 1 + noise bound. New benchmarks: B-F1 freshness-metadata overhead on the hit path; B-F2 revalidation/stale-serving throughput with an in-process mock origin (labeled microbenchmark, not an origin-degradation experiment); B-EV eviction-trigger latency distribution vs shard size {1k, 10k, 100k}; B-SW sweeper lock-hold distribution. Windows-native and WSL2 are never pooled. The report states exactly which items ran and their classes.

## 6. Telemetry and measurement integrity
Every new field enters the schema-v2 catalog with source, unit, window, PIE-safety, and the unavailable representation (no `v`). New counters are atomics off the hit path. `stale_*` and `revalidation_*` are `measured`; windowed ratios `derived`. The UI must show unknown new fields as raw labeled values or hide them — never relabel. Simulated values appear only under `mode: simulation`.

## 7. Risk register
| Risk | Failure mode | Containment |
|---|---|---|
| Freshness parsing bug serves wrong/private content | data exposure | conservative defaults (uncacheable on doubt), matrix + fuzz, flag-gated rollout, `Set-Cookie`/`Authorization` rules |
| Stale serving masks origin failure silently | operators misled | cap + counters + UI labels; `stale_cap_violations` |
| Eviction rewrite changes hit ratios | product regression | reference parity tests, paired VALID benchmarks, DROP rule |
| Epoch redesign weakens isolation | cross-tenant effects | reject collateral-invalidation designs unless O2-9/O2-3-style owner approval |
| Sweeper adds lock stalls | tail latency | measured lock-hold bound, per-tick budget, off-switch |
| Over-optimization on noisy platforms | false wins | VALID-only rule, ABAB pairing |
| Model-check coverage illusion | missed interleavings | mutation test proving detection |
| Large shared-file edits conflict | corrupted merges | sequential editing order, ledger, snapshot |
Known limitations at exit: single-node only; no `Vary`; no heuristic freshness; timings on lower-trust platforms; cluster behavior untouched.

## 8. Authorized files (on authorization) and prohibitions
**May modify** (with ledger): `src/core/cache.rs`, `src/core/coalesce.rs`, `src/api/server.rs`, `src/main.rs`, `src/intel/*`, `src/lib.rs`; new files under `src/core/` (freshness, eviction policies), `benches/**`, `tests/**` (new files only; existing tests only where an owner-reserved edit is recorded), `docs/research/**`, `docs/handoff/CORRECTIONS.md`, `ui/**` (schema display only). **Must not modify:** `src/dist/*`, `src/security/*` (M3/M7 own them), `src/control/pie.rs` behavior, `Cargo.toml` runtime dependencies, `.gitignore`. All CER-3 prohibitions apply.

## 9. Owner-reserved decisions (this phase)
O2-1 default behavior when origin sends no freshness info (keep 60 s fixed vs configurable default vs do-not-cache) · O2-2 initiator-cancellation semantics if current behavior differs · O2-3 `Vary` support scope · O2-4 eviction: implement faithfully vs rename · O2-5 concurrency-verification approach and any dev-dependency/feature (`loom`/`shuttle`, `tokio/test-util`) · O2-6 PIE quarantine mechanism beyond comments · O2-7 schema v1 retirement mode · O2-8 background sweeper task · O2-9 any runtime dependency or fuzzing toolchain · O2-10 any edit to existing tests or to public struct fields they use · O2-11 default `PHANTOM_MAX_STALE_MS`. **Agents may decide:** file organization, internal data structures, test structure, parameter ordering in sweeps, equivalent approved crate versions, doc wording.

## 10. Stop conditions (CER-7) specific to M2
E0 mismatch; Phase 1 baseline not VALID where a rule depends on it; a freshness-matrix case reveals a security-relevant default that conflicts with O2-1/O2-3; any change that would alter fencing/epoch semantics beyond the written design; SEC-4A or coalescing tests fail after a change; a measurement shows regression beyond the noise bound; need for a new runtime dependency; sweeper lock-hold above the bound; soak finds a leak or `stale_cap_violations > 0`.

## 11. Completion report
Per CER-8, plus: ADR-002; supported-subset document; the audit tables; every rule-gated WP decision with data; schema additions; list of owner decisions made; list of DROPPED optimizations with evidence; next-phase proposal only.
