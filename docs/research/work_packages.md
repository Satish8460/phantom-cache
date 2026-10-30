# Work Packages for AG (Antigravity)

> **Precedence (2026-10-29):** for Milestone 1 (WP-00…06), `phase_1_execution_package.md` (v3) is authoritative (step ids S0…S11, byte-budget semantics §6.5, benchmark plan §6.3, telemetry catalog §6.4, WP-00 checks §6.2). The WP texts below are retained as the detailed background specs and are overridden wherever they differ.

**Status: PROPOSED. No package is authorized until the owner says so in `implementation_master_plan.md` §9.** Every package: small, independently reviewable, one PR-sized diff. Fields 1–17 per the owner's template. "Size" is relative (S/M/L) with named unknowns, not hours.
Common rules for every package: read `CLAUDE.md` first; run `git rev-parse HEAD`, `git branch --show-current`, `git status --short` before and after; touch only listed files; no commits/pushes/stash/reset/clean unless the owner explicitly asks; Cargo with `-j 1`; report with the evidence format in `implementation_master_plan.md` §8; stop on any listed stop condition.
Track tags: **[P]** product, **[R]** research, **[S]** shared.

## Milestone traceability (canonical roadmap: `mvp_and_milestones.md`)
| Milestone | WPs |
|---|---|
| M1 Truth & Baseline | WP-00, 01, 02 (+02b), 03, 04, 05, 06 |
| M2 Core Correctness, Performance & Freshness | WP-07, 08, 09, 14, 22a (PIE demotion), 23 (single-node verification & soak, new) |
| M3 Distributed Correctness & Architecture Gate | WP-10, 11, 12, 13 |
| M4 Phantom Lab & Static Baselines | WP-15, 16, 17, 20a |
| **M5 Phantom Intelligence & PIE** *(new, Rev 2)* | **WP-30 interfaces, 31 signals/features, 32 shadow evaluator, 33 PIE engine v2, 34 policy execution layer, 35 audit/explainability + UI, 37 safety/overhead/replay** (M5a); **WP-36a/b/c candidates, WP-18 origin-aware control, WP-19a candidate evaluation** (M5b). WP-22b is **superseded by WP-33**. Spec: `phantom_intelligence_phase.md`; audit: `pie_audit.md` |
| M6 Research Evaluation & MVP Gate *(formerly M5)* | WP-19b (integrated evaluation), WP-20b |
| M7 Productization & Research Release *(formerly M6)* | WP-21, 20c, 25 (artifact packaging, new) |
Research tasks RT-0…RT-6 are listed in `mvp_and_milestones.md` §7. "Phase N" in older documents ≡ Milestone N.

## Dependency graph
```
WP-00 ─┬─► WP-01 (docs)            WP-04 (bench harness; needs approvals) ─► WP-07, WP-08, WP-09 (evidence-gated)
       ├─► WP-02 ─► WP-03          WP-05 (reference policies) ──────────────┘ (WP-08)
       └─► WP-06                   WP-02 + WP-04 + WP-05 ─► Gate G-BASE
G-BASE ─► WP-15/16/17 (lab, origin model, static baselines) ─► WP-18 (controller prototype, isolated) ─► WP-19 (evaluation)
G-BASE ─► WP-10 (multi-gateway harness + ADR decision) ─► Gate G-ARCH ─► WP-11..13 (cluster repair, if option A) 
WP-03 + WP-02 ─► WP-20 (lab UI) ; WP-14 (TTL/Cache-Control) independent after WP-02
```

---

## WP-00 — Characterization tests of known defects [S]
1. **ID/title:** WP-00 Executable evidence for the architecture gaps.
2. **Problem:** The ADR's central findings (peer invalidation does not purge the serving cache; gap handling discards the gap; shared stream sequence; DELETE ordering) are documented but not executable. The 242-test baseline passes while these hold.
3. **Desired outcome:** `cargo test --all-targets -j 1` still reports **242 passed + N ignored** (N new), and `cargo test --test known_gaps -j 1 -- --ignored` runs N tests that **fail for exactly the stated reasons today**, turning green only when the corresponding defect is fixed.
4. **Evidence:** ADR-001 §2 (Q1, Q2, Q4, Q5, Q5b); `main.rs:65,237,253`; `node.rs:42,458-509`; `router.rs:142,615`; `anti_entropy.rs:200-231`; `server.rs:830-832`.
5. **Scope:** one new file `tests/integration/known_gaps.rs` + a **required** 3-line `[[test]]` entry appended to `Cargo.toml` (verified: all 17 test targets are declared explicitly; manifest edit with **no dependency changes**; `Cargo.toml` is already uncommitted-modified, so append only). Reuse helpers/patterns from `security_phase4b_router_tests.rs` (gateway + node + router construction) and `m1_http_tests.rs` (mock origin, reqwest client).
6. **Out of scope:** any change in `src/`; fixing any defect; changing existing tests; adding dependencies.
7. **Design:** Each test is `#[tokio::test] #[ignore = "KNOWN GAP <id>: <one line>"]` and asserts the **desired** behavior (so it fails now).
   - **KG-1 (cross-gateway invalidation):** build two gateways **the way `main.rs` does**: per gateway `http_cache = PartitionedCache::new(8)`, `ApiState{cache: http_cache,..}`, a local `VirtualNode` (own listener via `start_tcp_service("127.0.0.1:0")`) and a peer `VirtualNode` for the other gateway with `set_addr` + `Active`, router over both nodes. Origin = local mock returning a body with a counter. Steps: GET on gateway B (fills B's HTTP cache); DELETE on gateway A → expect 204; GET on gateway B → **assert the origin was re-fetched (fresh body)**. Today B serves the stale entry. Note in a comment that the test mirrors `main.rs:234-264` wiring because `main()` is not callable.
   - **KG-2 (reconcile has no caller):** not testable as a behavior test; instead a source-level guard test that greps `src/` for call sites of `reconcile_peer_anti_entropy` outside its definition and asserts ≥1 (ignored; fails today). If the repo avoids source-scanning tests, replace with a doc-only note and skip.
   - **KG-3 (gap not recoverable):** unit-level on `AntiEntropyTracker`: feed seqs 1,2,5 for peer P; assert `get_highest_acked_seq(P) == 2` (the last *contiguous* acked) so that replay would include 3–4. Today returns 5 (fails).
   - **KG-4 (sequence per router, not per peer):** router with 3 nodes; perform invalidations on distinct keys; read, via the nodes' `anti_entropy_tracker`/counters, that each receiver observed a **contiguous** sequence from the originator. Today receivers see interleaved numbers and `anti_entropy_gaps_detected > 0` with zero actual loss.
   - **KG-5 (DELETE order):** gateway with router whose primary acknowledges slowly (use a node `artificial_delay_ms`); issue DELETE and concurrently GET; assert GET after DELETE *start* but before ack does not return the old entry (desired per docs `handoff/01:166`). Today it can. Mark `#[ignore]` and flag as **timing-sensitive** (retry loop ≤ 5 times; if flaky, drop and report).
   - Plus **passing (non-ignored) characterization tests** that pin *current* behavior needed to detect accidental change: e.g., standalone DELETE (no router) purges the local entry and returns 204.
8. **Invariants:** default test run is unchanged apart from the added ignored tests; no network access beyond loopback; no fixed ports (bind `:0`).
9. **Failure behavior:** tests must have explicit timeouts (≤10 s each) and shut down servers/nodes; a hung test fails, not blocks.
10. **Testing:** the deliverable *is* tests. Also run the full suite three times to confirm no flakiness in non-ignored tests.
11. **Benchmarks:** none.
12. **Acceptance:** (a) `cargo test --all-targets -j 1` → 242 passed, 0 failed, N ignored; (b) `cargo test --test known_gaps -j 1 -- --ignored` → each of the N fails with an assertion message naming its KG id (paste output); (c) `git status` shows only the new file (and the manifest line if needed) as new/changed relative to the pre-task state; (d) `cargo clippy --all-targets -j 1 -- -D warnings` clean for the new file (report if pre-existing warnings appear elsewhere — do not fix them).
13. **Documentation:** add a short section "Known-gap tests" to `docs/research/code_findings.md` listing KG ids and the defect each encodes.
14. **Dependencies:** none. Needs approval to add a test file.
15. **Risks/rollback:** if `Cargo.toml` needs an entry, revert that line and the new file (both new/additive; no tracked content depends on them). Risk: reproducing `main.rs` wiring in a test can drift from `main.rs`; mitigated by the comment and by WP-10 later extracting a shared builder.
16. **Size/uncertainty:** M. Unknowns: how existing tests build a multi-node router with mTLS vs insecure loopback (use the insecure loopback approach the existing 4b tests use); timing flakiness of KG-5.
17. **Stop conditions:** if reproducing KG-1 requires modifying `src/`; if any non-ignored test count differs from 242 after the change; if existing tests need edits; if a defect described here appears **not** to reproduce (report immediately — that would falsify an ADR claim).

---

## WP-01 — Documentation reconciliation [P]
1. **Title:** WP-01 Correction note for unsupported handoff claims.
2. **Problem:** `docs/handoff/*` asserts behaviors (self-healing anti-entropy, 15 s Suspect, zero P0, plaintext rejected, diagnostics endpoint/metrics) that the code does not provide (ADR-001 §5).
3. **Outcome:** a reader of `docs/handoff/00_START_HERE.md` is directed to a correction record before relying on any claim.
4. **Evidence:** ADR-001; `architecture_investigation.md` §3.
5. **Scope:** **new** `docs/handoff/CORRECTIONS.md`; **one added line** at the top of `00_START_HERE.md` linking to it. No other edits.
6. **Out of scope:** rewriting handoff documents; editing `docs/security/*`; any code.
7. **Design:** table of claim → source (file:line) → status (unsupported / contradicted / unverified) → evidence link; plus the statement that the 242-test count is reproduced and what it does and does not establish.
8. **Invariants:** only additive; original text preserved.
9–11. n/a.
12. **Acceptance:** every row cites a file:line verified by the author; owner review.
13. **Docs:** this is the doc.
14. **Dependencies:** owner approval (decision #4); WP-00 recommended first.
15. **Risks:** low; rollback = delete the new file and the added line.
16. **Size:** S.
17. **Stop:** if a claim cannot be verified by reading code, mark "unverified", do not assert.

---

## WP-02 — Telemetry backend integrity and provenance schema [S]
1. **Title:** WP-02 Truthful telemetry (schema v2).
2. **Problem:** Verified: DELETE recorded as a zero-latency miss (`server.rs:945-953`); `key_hash` always 0 (`:482,:770,:946`); the field named p95 holds p90 (`:1654-1673`); `throughput_req_sec` is a cumulative counter or random; `hot_keys` literal (`:227-229`); node CPU/memory literals (`:1698-1706`); `backend_load_ratio` is `1 − cumulative hit ratio`; no origin metrics exist.
3. **Outcome:** an SSE v2 payload (and a `GET /api/v1/metrics` snapshot) in which every field carries provenance (`ui_metric_provenance.md` §3); in `live` mode there are no simulated values; DELETE no longer appears as a request in latency/hit statistics; real windowed rates and a real p95; real key hash; origin-fetch counters/latency/in-flight (WP-02b below).
4. **Evidence:** `ui_metric_provenance.md` §1; `code_findings.md` F5, F10, F11, F13.
5. **Scope:** `src/api/server.rs` (SSE handler, `delete_cache_handler` telemetry call, `get_cache_handler` record calls, `spawn_simulation_loop` gating), `src/intel/telemetry.rs` (add key hash, windowed accessors), `src/intel/diagnostics.rs` only for counters needed, `src/main.rs` only if mode selection needs a flag. Tests: new `tests/integration/telemetry_truth_tests.rs`.
6. **Out of scope:** replacing the telemetry data structure (that is WP-09, benchmark-gated); UI (WP-03); removing the simulation loop (keep, but make it opt-in via an explicit `PHANTOM_SIMULATION=true`, default off for the live gateway — **decision flag for the owner**, because it changes demo behavior).
7. **Design:**
   - Provenance enum `Source{Measured,Derived,Simulated,Inferred,Unavailable}`; payload `schema: 2`, `mode: "live"|"simulation"`, fields as `{v, src, window_ms?, as_of_ms}`; keep v1 payload available behind `?schema=1` for backward compatibility until the UI is updated (WP-03), then remove in a later package.
   - Telemetry observations: `RawObservation` gains `kind` (`Get`, `Delete`) and a real `key_hash` (use a stable 64-bit hash of the canonical key; document the hasher; **not** `DefaultHasher` if cross-process stability is needed — decide and record). Percentile/hit-ratio calculations consider `Get` only.
   - Rates: server-side sliding window (e.g., 1 s and 10 s) from monotonic counters (cheap: store last snapshot of counters per tick); `throughput` = Δrequests/Δt measured, `src: derived`.
   - Percentiles: p50/p90/p95/p99 from the existing buffer (do not redesign here); label window = buffer size/time span.
   - In `live` mode: `nodes[].cpu/mem` → process-level RSS/CPU if cheaply available on Windows+Linux without new dependencies, else `unavailable`; `hot_keys` → `unavailable` until a heavy-hitter structure exists (sketch is unused; do not wire it in this package); `avoided_compute_dollars` → removed from live payload.
   - **WP-02b origin-fetch instrumentation (same package, separate commit-sized step):** in the origin-fetch path (`fetch_origin_raw` caller) record: origin requests started/finished, in-flight gauge, latency histogram or buffer, error/timeout counts by class. These are the signals the research controller and baselines need.
8. **Invariants:** no change to cache semantics or response bytes/headers; no new locks on the hit path beyond what exists (counters must be atomics); telemetry failure must never fail a request.
9. **Failure:** if the clock goes backwards, rates report `unavailable` for that tick; poisoned locks handled as elsewhere in the file.
10. **Testing:** (a) unit: DELETE does not change `hit_rate`/percentiles; (b) integration: N real GETs → counters and derived rates consistent within tolerance; p95 ≠ p90 on a crafted latency set; (c) contract test: `live` mode payload contains no `simulated`; (d) existing 242 tests green (update only assertions that depended on the old payload shape, and list each change).
11. **Benchmarks:** none required; do not claim overhead. If the owner wants, record B6 baseline before/after (WP-04).
12. **Acceptance:** all above tests; `grep -n "RawObservation::new(" src` shows no `0` key hash; the SSE v2 sample in the report shows provenance on every field.
13. **Docs:** update `ui_metric_provenance.md` statuses; add the schema to `docs/research/` (`telemetry_schema_v2.md`).
14. **Dependencies:** WP-00 (baseline), owner decisions on simulation default and key hasher.
15. **Risks:** payload change breaks the current UI → v1 compatibility flag; rollback = revert the package (additive tests/new file stay).
16. **Size:** M–L. Unknowns: process CPU/RSS without new deps on Windows; buffer capacity semantics.
17. **Stop:** if making `live` mode honest requires removing the dashboard's only data source for a demo the owner depends on; if a new dependency seems necessary; if `telemetry` is `None` in any production path (report).

---

## WP-03 — UI truthful live view [P/S]
1. **Title:** WP-03 Dashboard consumes real, provenance-labeled data.
2. **Problem:** `ui_metric_provenance.md` §2: hit/miss/coalesced/cost derived from constants; fabricated "Phantom OFF" multipliers; constant charts; hard-coded PIE inspector text; hard-coded `localhost:8085`.
3. **Outcome:** live view shows only server-supplied values with badges; unavailable states; configurable gateway URL (`?gateway=`/env at build); simulation controls and the economics panel moved to a clearly labeled "SIMULATION" view or removed; the PIE panel shows "no adaptive controller active" (until real traces exist).
4. **Evidence:** `App.jsx:53-66,87,93-98,167-168,179,412-455`.
5. **Scope:** `ui/src/App.jsx` (split into components if helpful: `ui/src/components/*`), `ui/src/main.jsx`, `ui/vite.config.js` only for env; add UI contract test setup **only if** it needs no new runtime dependencies (a dev-dependency such as `vitest` requires approval — list it, don't add).
6. **Out of scope:** backend changes; new visualizations of data that does not exist; restyling beyond what is needed.
7. **Design:** a `<Metric value src asOf />` component; one SSE hook that parses schema 2; derived values never computed client-side except formatting/charting; "SIMULATION" mode switches data source to lab outputs later (WP-20) — for now the view only exists behind a flag and is labeled.
8. **Invariants:** no mock/synthetic value renders in live mode; disconnected state shows "disconnected", not last value styled as live.
9. **Failure:** SSE reconnect with backoff; schema mismatch shows an explicit error.
10. **Testing:** fixture-driven tests: (a) `live` fixture with a `simulated` field fails the contract check; (b) `unavailable` renders as empty state; (c) manual run against a real gateway (`npm run build` + `cargo run`) with screenshots in the report.
11. **Benchmarks:** n/a.
12. **Acceptance:** `npm run build` succeeds; contract tests pass (or, if no test runner is approved, a documented manual checklist with screenshots); grep for forbidden constants (`0.94`, `0.88`, `28.4`, `Canary Duration`) returns none in live components.
13. **Docs:** update `ui_metric_provenance.md`.
14. **Dependencies:** WP-02 schema v2 merged.
15. **Risks:** loses the demo's visual story; mitigation: keep the old view under `/lab-demo` labeled SIMULATED if the owner wants it. Rollback: revert UI files.
16. **Size:** M. Unknown: appetite to keep the demo view.
17. **Stop:** if a metric the UI wants has no truthful source — show "unavailable", never improvise; if tests need a new dev-dependency.

---

## WP-04 — Benchmark harness and pilot baseline [S]
1. **Title:** WP-04 Reproducible hot-path benchmarks (B1, B2, B6 pilot).
2. **Problem:** no benchmark of the real path exists; H1–H3, H7–H9 unverified.
3. **Outcome:** `cargo bench` (release) produces B1/B2 results with the run manifest; a pilot establishes run-to-run variation per platform.
4. **Evidence:** `benchmark_suite_spec.md`; no `benches/` dir; no criterion dependency.
5. **Scope:** `Cargo.toml` (`[dev-dependencies]` + `[[bench]]`, **with approved dev-dependencies only**), `benches/hot_path.rs`, `benches/support/*`, `bench-results/README.md`, a manifest-writer helper. No `src/` change; if a benchmark needs an item that is private, **stop** (do not widen visibility in this package).
6. **Out of scope:** optimizations; interpreting results as targets; end-to-end (B8); profiling installs in WSL (separate approval).
7. **Design:** per `benchmark_suite_spec.md` §2–3 (B1, B2, B6-current). Use `PartitionedCache` public API; deterministic seeds; HDR-style histograms for latency distribution if the harness supports it, otherwise raw samples.
8. **Invariants:** benchmarks never run in `cargo test`; they do not modify global state; the harness refuses to run in debug.
9. **Failure:** abort with a message when machine noise gates (e.g., variance above the bound) are violated in the pilot — record, don't hide.
10. **Testing:** a smoke test that the benchmark harness compiles and runs a 1-iteration config.
11. **Benchmarks:** this package *is* the baseline; deliver raw results + a summary table.
12. **Acceptance:** results for B1/B2 on `windows-native` (and `wsl2` if approved), ≥30 repetitions, manifests present; report states the variation bound; no claims beyond it.
13. **Docs:** `baseline_and_profiling_plan.md` updated with the pilot outcome and platform trust decisions.
14. **Dependencies:** owner approval of dev-dependencies (criterion or alternative, `dhat` optional) and of WSL2 Rust install (for the wsl2 platform label).
15. **Risks:** noisy platform → inconclusive; rollback: remove `benches/` and the dev-deps.
16. **Size:** M–L. Unknown: harness crate maintenance/MSRV vs rustc 1.98; WSL2 noise.
17. **Stop:** if a dependency requires a runtime feature or network access at build beyond crates.io fetch of dev-deps; if results are not repeatable within any usable bound (report; do not tune).

---

## WP-05 — Reference policy implementations (isolated) [R/S]
1. **Title:** WP-05 Correct SIEVE, S3-FIFO, LRU, TinyLFU reference implementations for comparison.
2. **Problem:** the production evictors are scans named after other algorithms (F6–F8); comparisons against "SIEVE/S3-FIFO" are invalid without faithful implementations.
3. **Outcome:** a self-contained module/crate with O(1) amortized implementations behind one trait, validated against published behavior on small hand-computed traces, usable by benchmarks and the lab. **Not wired into production.**
4. **Evidence:** `cache.rs:348-479`; algorithm descriptions in the SIEVE (NSDI'24) and S3-FIFO (SOSP'23) papers (read the primary text before implementing).
5. **Scope:** new files only, e.g. `benches/support/refpolicies.rs` or a new workspace crate if the owner approves a workspace change (default: a `cfg(test)`/bench-support module; **no change to `Cargo.toml` members**).
6. **Out of scope:** replacing production evictors (WP-08); variable-size/cost features; concurrency.
7. **Design:** `trait Policy { fn access(&mut self, key:u64)->bool /*hit*/; fn len()...}`; SIEVE: FIFO queue + hand + visited bit (single-threaded reference); S3-FIFO: small (≈10%)/main/ghost queues with 2-bit counters per the paper; LRU: linked hash; TinyLFU: count-min sketch admission + window (state the parameters). Each with a doc comment citing the paper section.
8. **Invariants:** capacity never exceeded; deterministic given a trace.
9–10. **Testing:** golden traces with hand-derived expected hit sequences (record derivations), property tests only if `proptest` approved; cross-check miss ratios on a synthetic Zipf trace against an independent simple implementation in the test.
11. **Benchmarks:** none (algorithmic correctness only).
12. **Acceptance:** golden tests pass; PR description lists paper sections each implementation follows and any deviations.
13. **Docs:** `literature_review.md` notes which implementation corresponds to which paper.
14. **Dependencies:** WP-00 only. 
15. **Risks:** subtle deviations from the papers → mitigated by golden traces; rollback: delete files.
16. **Size:** M. Unknown: precise S3-FIFO parameters (read paper; ask if ambiguous).
17. **Stop:** if the paper text is ambiguous on a parameter; if integration into `src/` appears necessary.

---

## WP-06 — Byte-aware capacity accounting [P]
1. **Title:** WP-06 Bound cache memory in bytes.
2. **Problem:** capacity is entries only (`cache.rs:151,191,558`); worst-case resident bytes ≈ shards × 10,000 × per-entry body cap (2 MiB per [A]) — i.e., unbounded in practice; contradicts the "no unbounded allocation" goal.
3. **Outcome:** a configurable per-shard (or global) byte budget; entries are evicted until both the entry and byte budgets are satisfied; a `bytes_resident` counter exposed via telemetry; documented per-entry overhead model.
4. **Evidence:** `code_findings.md` F12; benchmark B7 will measure accounting error.
5. **Scope:** `src/core/cache.rs` (struct fields, `begin_fetch_atomic` capacity block `:558-568`, `finalize_fetch` fill, `delete*`, eviction functions return the removed size), `src/intel/diagnostics.rs` (counter), `src/main.rs` (env var `PHANTOM_MAX_CACHE_BYTES` default chosen conservatively and recorded), tests in `tests/unit/` or a new file.
6. **Out of scope:** changing eviction algorithms; per-tenant quotas; reworking epochs.
7. **Design:** `entry_bytes = key.len + body.len + content_type.len + etag.len + fixed_overhead` (fixed overhead documented, validated in B7 against RSS); maintain `bytes` in `CacheShard` updated at insert/replace/delete/evict/epoch-invalidation lazy-drop points. Admission rule: a single object larger than the shard byte budget is not cached (`Uncacheable`, counter). Eviction loop: while `entries ≥ max_entries || bytes + new > max_bytes` evict (respecting the "never evict active ops" invariant); if nothing evictable → existing `CapacityExceeded` rejection.
8. **Invariants:** accounted bytes == Σ entry bytes at quiescence (debug assertion + test); the fencing and no-evict-active invariants unchanged; shard lock held for all updates (no extra lock).
9. **Failure:** overflow-safe arithmetic (`checked_*`); poisoned locks as today.
10. **Testing:** unit: accounting after insert/replace/delete/evict/tenant-epoch drop/finalize failure paths; property test (if `proptest` approved) over random op sequences preserving the invariant; integration: fill with large bodies under a small budget and confirm bounded `bytes_resident`; existing 242 green.
11. **Benchmarks:** before/after B1 hit path to check no regression (once WP-04 exists); report, don't claim improvement.
12. **Acceptance:** tests above; B7 shows accounting error within a stated tolerance (set from the first measurement); documented default.
13. **Docs:** `CLAUDE.md` invariants line for bytes; config reference.
14. **Dependencies:** WP-00; WP-04 for benchmarks (not blocking implementation).
15. **Risks:** accounting drift on lazy epoch invalidation paths (entries logically dead but resident) → count them until dropped; rollback: revert package.
16. **Size:** M. Unknown: lazy drop of epoch-stale entries (`get_resource` clears `entry` but keeps `KeyState`).
17. **Stop:** if byte accounting needs changing the `CachedResource` type used by tests widely; if a design requires a global lock.

---

## WP-07 — Epoch lookup contention (evidence-gated) [P]
1. **Title:** WP-07 Remove the global tenant-epoch mutex from the request path **if** B2 shows it matters.
2. **Problem:** H1 (code-confirmed, impact unverified).
3. **Outcome:** hit-path scaling improves measurably (CI excludes the pilot variation bound) with identical semantics.
4. **Evidence:** `cache.rs:251-259,265-307`; **requires B2 results** from WP-04.
5. **Scope:** `src/core/cache.rs` epoch structures and call sites.
6. **Out of scope:** the SEC-4A-01 fallback semantics (must be preserved); eviction.
7. **Design options (choose after data):** (a) snapshot `Arc<HashMap>` swapped on write (read lock-free via `arc_swap`-like pattern — a new dependency needs approval); (b) per-shard epoch table; (c) `RwLock` instead of `Mutex`; (d) sharded epoch map by tenant hash. Keep global epoch atomic.
8. **Invariants:** an entry is served only if its tenant/global epochs equal current; stale-resurrection impossible (SEC-4A-01 tests unchanged and green).
9–10. Tests: existing 4A tests; new concurrent test (invalidate_tenant racing with fills/hits) under `loom` if approved, otherwise stress test with fixed iterations.
11. Benchmarks: B2 before/after (≥30 reps, CI).
12. **Acceptance:** semantics tests green; benchmark improvement with CI, or **the package is abandoned and the finding recorded as "no measurable effect"**.
13. Docs: record result in `claims_ledger.md` C5.
14. Dependencies: WP-04; independent review for any change touching epochs.
15. Risks: subtle memory-ordering bugs → independent review; rollback revert.
16. Size: M; unknown: whether contention is measurable at all.
17. Stop: if B2 shows no effect (stop and report, do not "optimize anyway"); if a new runtime dependency is required.

---

## WP-08 — Eviction redesign (evidence-gated) [P]
1. **Title:** WP-08 Replace scan-based eviction with O(1)-amortized structures **if** B4 shows tail stalls or WP-05 shows large miss-ratio differences.
2. **Problem:** H2; names do not match algorithms.
3. **Outcome:** production evictors with documented algorithms, bounded work per eviction, same safety invariants; policy names reflect reality (rename or implement faithfully).
4. **Evidence:** `cache.rs:348-479`; requires B4 + WP-05.
5. **Scope:** `src/core/cache.rs` eviction + per-key metadata (queue links), `src/control/pie.rs` only for policy names.
6. **Out of scope:** adaptive selection; concurrency redesign beyond per-shard.
7. **Design:** per-shard intrusive FIFO/clock (SIEVE hand over a slab/`VecDeque` with tombstones); S3-FIFO queues + ghost set sized by entries; policy switch semantics (rebuild or lazy migration) specified before coding; never evict keys with active ops or in-flight fills.
8. **Invariants:** as WP-06 plus O(1) amortized eviction.
9. Failure: policy switch during load must not drop active ops.
10. Tests: WP-05 golden traces run against production eviction through its public API; concurrency stress; existing tests green.
11. Benchmarks: B4/B5 before/after vs WP-05 references.
12. **Acceptance:** miss ratio within a stated tolerance of the reference on golden/Zipf traces; eviction p99 reduced with CI, or documented no-change.
13–17: Docs `CLAUDE.md` invariants; Depends WP-04, WP-05, (WP-06 for byte budgets); Risk medium; Size L; Stop: if switching semantics cannot be made safe, or if data does not justify.

---

## WP-09 — Telemetry hot-path redesign (evidence-gated) [P/S]
1. **Title:** WP-09 Replace mutex+VecDeque+full sort with sharded counters and bounded histograms **if** B6/B2 show cost.
2. **Problem:** H3.
3. **Outcome:** `record()` is lock-free or sharded; percentiles from fixed-bucket histograms with documented error; no sort under a global lock.
4. **Evidence:** `telemetry.rs`; requires B6.
5. **Scope:** `src/intel/telemetry.rs`, call sites in `server.rs`, `pie.rs` consumer.
6. **Out of scope:** schema (WP-02).
7. **Design:** log-linear bucketed histogram (e.g., HDR-like, ~1% relative error), per-thread/shard arrays merged on read; windowed variant via rotating buckets.
8–12: invariants: counts exact, percentile error bounded and tested; tests vs exact on random distributions; B6 before/after; acceptance by CI.
13–17: Depends WP-02, WP-04; Risk low–medium; Size M; Stop: if histogram error is unacceptable for the controller's needs (record requirement first).

---

# Later packages (stubs — fields 7–12 are deferred because they depend on gate outcomes; each is re-specified at the gate)

| ID | Title | Track | Gate that unlocks full spec | One-line problem / outcome | Key dependencies / stop |
|---|---|---|---|---|---|
| WP-10 | Multi-gateway fault-injection harness + shared cluster builder + ADR decision | S/P | G-BASE | Extract the `main.rs` cluster wiring into a testable builder (needs `src/` change → approval); harness with N gateways, per-link loss/delay/partition, mock origin; makes KG tests run in CI. Outcome: decision A/B/C recorded with measurements | Stop if extracting the builder changes behavior |
| WP-11 | Cluster repair A1: invalidations purge the serving cache; node↔gateway cache unification | P | G-ARCH = option A | Make `VirtualNode`'s cache the gateway's (or route invalidation into `ApiState.cache`) so KG-1 passes | WP-10; independent review |
| WP-12 | Cluster repair A2: outbound history, per-peer sequence spaces, gap-correct acks, bounded fan-out, durable miss record | P | G-ARCH | Fix KG-3/KG-4, G3–G6; metrics exported | WP-11 |
| WP-13 | Cluster repair A3: reconciliation trigger, node-state recovery, staleness-bound measurement | P/R | G-ARCH | Production caller for reconciliation; bounded staleness measured under injected loss; claim C6 rewritten | WP-12 |
| WP-14 | TTL from `Cache-Control` and revalidation semantics | P | after WP-02 | Replace fixed 60 s; define stale-while-revalidate/stale-if-error support (also needed for the research baselines) | Spec RFC 9111 subset first |
| WP-15 | Deterministic lab core (virtual clock, seeded RNG, replay, scenario library) | S | G-BASE | Reuse `sim/*`; Tier 1 tooling; outputs labeled SIMULATED | WP-04 manifests |
| WP-16 | Real-process controllable origin (latency step, brownout, outage, recovery) + load generator | S | G-BASE | B8/B9 substrate for Tier 2 | WP-02b origin metrics |
| WP-17 | Static origin-protection baselines in the lab: Varnish-style grace/health, nginx-style lock+stale, XFetch, DynamoDB constant-work refresh, Envoy-style adaptive concurrency/admission, retry budget, Farahbakhsh remedies, oracle-tuned static | R | WP-14, WP-15, WP-16 | Each baseline implemented in the isolated harness with its parameters documented and tuned fairly | Independent review of baseline fidelity |
| WP-18 | Origin-aware controller prototype (isolated) with staleness cap and origin-load cap | R | G-RESEARCH (below) | Only after baselines; clamped mechanisms first; controller only if ablations demand | Pre-registered thresholds |
| WP-19 | Evaluation run: Tier 1 and Tier 2, ≥30 seeds, CIs, ablations, negative results | R/S | WP-17, WP-18 | Populates `claims_ledger.md` | Report whatever is found |
| WP-20 | Lab UI: experiment runner, comparison with uncertainty, manifest export | P/R | WP-03, WP-15 | Separate Lab view | |
| WP-21 | Deployment readiness (byte sizing, hot-reload membership, observability export, runbooks, key mgmt, multi-host validation) | P | after G-ARCH | | Physical Linux hosts |
| WP-22 | **Fate of the legacy threshold PIE (`src/control/pie.rs`) — Rev 2 reconciliation:** **22a (M2):** quarantine — keep compiled, label *legacy baseline*, never active on live traffic, re-label `wp2_tests`/`p4_pie_tests` as state-machine tests (see `pie_audit.md` §5–6); **22b is superseded by WP-33** (PIE decision engine v2, M5). PIE itself is **required** for the MVP; only the stub is retired | P/R | 22a after WP-14; replacement via WP-33 | Remove the stub from every product path once WP-33 lands | `p4_pie_tests` assert stub behavior and would change |

## Rev 2 additions — Milestone 5 packages (stubs; full 17-field specs at the M5a/M5b gates; scope and acceptance fixed in `phantom_intelligence_phase.md`)
| ID | Title | Track | Stage | Depends on |
|---|---|---|---|---|
| WP-30 | Phantom Intelligence architecture & interface specification (traits/types, failure semantics, kill switch) | S | M5a | G-M1, G-M2 |
| WP-31 | Signal & feature pipeline (aged per-key sketch with real `key_hash`, heavy hitters, skew, MRC estimate, shift/origin-degradation detection) with provenance | S/P | M5a | WP-02/02b, WP-30 |
| WP-32 | Shadow (counterfactual) evaluator using the reference policies | R/S | M5a | WP-05, WP-31 |
| WP-33 | PIE decision engine v2 (constraint-first utility, shadow + canary, statistics, verdicts, cooldown/hysteresis, advisory/active) — supersedes WP-22b | P/R | M5a | WP-30, 31, 32, 34 |
| WP-34 | Policy execution layer & safe fallback in the cache (policy handle, atomic swap, key-bucket canary partitions, hard caps) — touches `cache.rs`, independent review | P | M5a | WP-06, WP-08 outcome, WP-14 |
| WP-35 | Decision audit & explainability (bounded record store, API, Intelligence UI panel, export) | S/P | M5a | WP-33, WP-03 |
| WP-37 | Intelligence safety, overhead & deterministic-replay verification | S | M5a | WP-33, 34, WP-15 |
| WP-36a | Statistical candidates (shadow-selected eviction, TinyLFU-style admission, hot-key-aware pinning, adaptive refresh-ahead, coalescing limits) | R/P | M5b | G-M4, G-RESEARCH |
| WP-36b | Predictive candidates (rate forecasting, next-key prefetch) | R | M5b | WP-36a |
| WP-36c | ML selector — only through the ML gate (`phantom_intelligence_phase.md` §6) | R | M5b | WP-36a evaluated |
| WP-18 | Origin-aware control (moved to M5b as a PIE candidate family) | R | M5b | G-RESEARCH |
| WP-19a / 19b | Candidate evaluation (M5b) / integrated system evaluation (M6) | R/S | M5b / M6 | WP-17, candidates |

## Package sizing summary (relative)
S: WP-01. M: WP-00, WP-05, WP-06, WP-07, WP-09, WP-03. M–L: WP-02, WP-04. L: WP-08 and all cluster/lab/controller packages. Unknowns are listed per package; no hour estimates are given.
