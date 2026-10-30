# AG Task Instruction — WP-00 (Characterization tests of known defects)

> **SUPERSEDED (2026-10-29):** the authoritative WP-00 specification is `phase_1_execution_package.md` (v3) §5 step S2 and §6.2 (adds positive controls, `PRECONDITION`/`DEFECT` failure prefixes, valid-reproduction rules, unexpected-outcome handling, a deterministic KG-4 and KG-5, and evidence notes for KG-2). This earlier text is retained for history only; where it differs, v3 wins.

**Authorization status: NOT YET AUTHORIZED.** The owner must approve in `implementation_master_plan.md` §9 before AG starts. Nothing below may be executed until then.

You are AG. You implement exactly this package. You are not asked to decide architecture. If anything below is ambiguous or contradicted by what you see, stop and report (see Stop conditions).

## 0. Read first (in order)
1. `CLAUDE.md` (repo root).
2. `docs/research/adr/ADR-001-present-architecture-and-direction.md` (§1–§3, §5).
3. `docs/research/work_packages.md` → WP-00.
4. Existing patterns: `tests/integration/security_phase4b_router_tests.rs` (gateway + router + nodes, e.g. lines ~395-560), `tests/integration/m1_http_tests.rs` (mock origin, reqwest), `tests/integration/security_phase4c_antientropy_tests.rs` (anti-entropy).
5. Sources under test: `src/main.rs:55-310`, `src/dist/node.rs:36-60,380-510`, `src/dist/router.rs:55-210,600-897`, `src/dist/anti_entropy.rs`, `src/api/server.rs:776-962`.

## 1. Pre-flight (record outputs verbatim in your report)
```
git rev-parse HEAD            # expect 456bb25930004f1896a84c53e776e7b3ef29ee16
git branch --show-current     # expect feat/publish-phantomcache-core
git status --short            # save to pre_status.txt (do not modify/clean anything listed)
cargo test --all-targets -j 1 # expect 242 passed, 0 failed, 19 targets
```
If HEAD/branch differ or the baseline is not 242/0, **stop and report**.

## 2. Deliverable
One new test file `tests/integration/known_gaps.rs`, plus **one required `[[test]]` entry** in `Cargo.toml` (verified: all 17 test targets are declared explicitly there, e.g. `name = "m1_http_tests"`, `path = "tests/integration/m1_http_tests.rs"`; follow that exact format with `name = "known_gaps"`, `path = "tests/integration/known_gaps.rs"`). Note `Cargo.toml` is already modified in the working tree (uncommitted): add the entry **only** by appending those 3 lines; do not reformat or reorder anything. No dependency changes. No edits to existing files other than that manifest entry and the doc addition in §6.

### Tests to write
Each ignored test asserts the **desired** behavior and must **fail today**. Name them `kg1_…`, `kg3_…` etc. Attribute: `#[ignore = "KNOWN GAP KG-n: <one-line defect>"]`.

**KG-1 `kg1_peer_invalidation_must_purge_other_gateways_http_cache`**
Mirror production wiring (`main.rs:234-264`): for each of two gateways G_A, G_B: `http_cache = Arc::new(PartitionedCache::new(8))`; `ApiState { cache: Some(http_cache), router: Some(router_X), nodes, … }` built by hand like the 4b tests; a **local** `VirtualNode` per gateway with `start_tcp_service("127.0.0.1:0")` + `set_addr`; each gateway's router is built over `[local_node, peer_node_of_other]` where the peer node has `set_addr(other_local_addr)` and `set_state(Active)`. Use the insecure loopback transport the existing 4b tests use (do **not** build mTLS here). Mock origin: a loopback `TcpListener` that answers `GET` with a body containing an incrementing counter (copy the helper style from `m1_http_tests.rs`).
Steps: (1) GET `/api/v1/cache/item` on G_B → body v1 (origin call count 1). (2) GET again → served from cache (count still 1). (3) DELETE `/api/v1/cache/item` on G_A → assert status 204. (4) GET on G_B → **assert origin call count is 2 and body is v2**. Today step (4) returns v1 from cache.
Include a comment citing ADR-001 Q1/Q2 and `main.rs:65 vs node.rs:42`.

**KG-3 `kg3_gap_is_not_acknowledged_past`** (unit-level)
```
let mut t = AntiEntropyTracker::new();
t.record_inbound_seq(7, 1, None); t.record_inbound_seq(7, 2, None);
t.record_inbound_seq(7, 5, None); // sequences 3 and 4 missed
assert_eq!(t.get_highest_acked_seq(7), 2); // last contiguous
```
Today returns 5 → fails. Add a second assertion that `pop_gap()` yields `(7, 3, 4)` (this one should pass today; keep it in the same test after the first so the failure message is about acking).

**KG-4 `kg4_each_peer_sees_contiguous_sequence_from_originator`**
Three nodes, one router over them; perform 10 invalidations on distinct keys via `invalidate_key_across_cluster` (wait for the broadcaster to drain; use `broadcaster().acked_count` with a bounded poll, ≤3 s). For each receiving node read its `anti_entropy_tracker` / `cache.counters.anti_entropy_gaps_detected`. Assert `anti_entropy_gaps_detected == 0` on every node when no message was lost. Today it is > 0 on at least one node (shared counter).

**KG-5 `kg5_get_after_delete_start_must_not_serve_old_entry`** (timing-sensitive)
Gateway with router whose primary node has `set_artificial_delay(120)` (`node.rs:137`; the delay is applied in the connection loop at `node.rs:344` — confirm it precedes frame dispatch for `InvalidateRequest`, else adapt or drop this test); prefill gateway cache via GET; spawn DELETE; after 20 ms issue GET; assert the GET does not return the pre-DELETE body. Retry the scenario up to 3 times; if it is flaky in either direction, delete this test from the file and say so in your report.

**KG-2** (reconcile has no production caller): **do not** write a source-scanning test. Instead add the line `KG-2: no test; verified by grep, see ADR-001 Q4` to the module doc comment.

### Passing (non-ignored) pin tests
- `pin_standalone_delete_purges_local_entry_and_returns_204`: gateway with `router: None`, prefill, DELETE → 204, next GET refetches.
- `pin_delete_with_router_purges_local_entry_only_after_success` — **optional**; skip if it needs sleeps.

## 3. Constraints
- Bind every socket to port 0. No fixed ports. Loopback only.
- Every test has an overall `tokio::time::timeout(Duration::from_secs(10), …)`.
- Shut down servers/nodes you start (`VirtualNode::stop_tcp_service()`, `node.rs:142`, and `router.stop()` as existing tests do).
- Do not touch `src/`, existing tests, `Cargo.lock` (if `cargo` rewrites it, report and revert **only** your own change via re-running; do not use git restore/checkout/stash).
- Do not "fix" anything you find. Do not rename or reformat other files.

## 4. Verification commands
```
cargo test --all-targets -j 1                         # 242 passed, 0 failed, N ignored
cargo test --test known_gaps -j 1 -- --ignored        # each KG test FAILS; capture output
cargo test --test known_gaps -j 1                     # pin tests pass
cargo clippy --all-targets -j 1 -- -D warnings        # report result; do not fix unrelated warnings
git status --short                                    # compare to pre_status.txt
git diff --stat -- tests Cargo.toml                   # only expected files
```
Run the full suite **three times** to check that the non-ignored tests are not flaky.

## 5. Evidence to return (paste, don't summarize)
1. Pre-flight outputs and `pre_status.txt` diff vs post (`git status --short`).
2. The three full-suite results.
3. The `--ignored` run output showing each KG failure message.
4. For each KG test: one sentence stating whether the observed failure matched the ADR claim; if any KG test **passes** or fails for a different reason, say so prominently — that falsifies or refines an ADR claim.
5. The file list and `git diff --stat`.
6. Clippy output.

## 6. Documentation
Append a "Known-gap tests" section to `docs/research/code_findings.md` (additive) listing KG ids → defect → file:line, with the observed outcome.

## 7. Stop conditions (stop, report, do not improvise)
- HEAD/branch/baseline differ from §1.
- Reproducing KG-1 appears to require changing `src/` (e.g., missing public constructors).
- Any non-ignored existing test changes outcome.
- A KG test unexpectedly passes.
- You need a new dependency, a fixed port, or a network resource.
- `Cargo.lock` changes in a way you did not intend.
- Anything else surprising about the tree (new modified files you did not touch).

## 8. Out of scope
Fixing defects, refactors, UI, benchmarks, docs other than §6, commits/pushes (do **not** commit; the owner will).
