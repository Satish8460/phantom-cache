# M3 durable-invalidation remediation: independent review

Status: review complete for the items listed in section 9; not a human review.
Audience: product owner, M3 implementer, integration agent.
Last reviewed: Fri Oct 30 17:37 IST 2026 (machine clock, `date`).
Label: **independent of the implementer; AI review, not human review.** Same model family and workflow as the implementer, so shared blind spots are possible.

## 1. Scope, method, independence

Subject: `docs/research/m3_durability/remediation/` (combined diff `patches/m3_remediation_combined.diff`, 13 files; docs patch applied only where stated).
Method: (1) read the main-tree code first (`src/dist/router.rs` RepairLedger/fan-out, `src/main.rs` shutdown, `src/api/server.rs` DELETE handler, `src/dist/node.rs` receiver); (2) read the patch; (3) only then read the implementer documents; (4) ran everything below in fresh sandboxes.
Sandboxes (main tree untouched; no src/tests/Cargo/ui/frozen/sealed file edited): `D:\Temp\sbx_m3_review` (UNPATCHED src; patched tests and Cargo.toml overlaid for baseline runs, original Cargo.toml restored for the baseline freeze run), `D:\Temp\sbx_m3_review_p` (combined diff applied; the `docs/user/configuration.md` hunk failed on first apply because the sandbox had no docs, applied later from `03_docs_configuration.diff`), `D:\Temp\sbx_m3rev_mut` (mutations). `CARGO_TARGET_DIR=D:\Personal\_sbx_m3rev_target`, every cargo call `-j 1` through `with_cargo_lock.ps1`, debug profile, loopback, one Windows 11 host. Logs: `D:\Temp\m3rev_logs\`.
Main-tree check: HEAD `804a7b8`, branch `feat/publish-phantomcache-core`; `git status --short` count rose 111 -> 135 during the session from other agents' work; my only main-tree files are this document and `independent_review_proposals/README_proposals.md`.
Not shown by anything here: power loss, Linux, release profile, multi-host (see section 10).

## 2. Verdicts (points 1-10)

| # | Point | Verdict | One-line basis |
|---|---|---|---|
| 1 | Defect reproduces on unpatched code | **PASS** | section 3 |
| 2 | Durable log correctness | **PARTIAL** | design sound on read and tests; unbounded durability wait (F-03), weak ordering test (F-04), no single-writer guard (F-07) |
| 3 | Concurrency | **PARTIAL** | no std Mutex across await found; drain undercounts main-queue tasks (F-05); in-flight detached leg at shutdown still untested (F-06) |
| 4 | Default-behaviour changes (log unset) | **PARTIAL** | five changes enumerated (section 6); none looks harmful, one existing test class impacted only by the freeze |
| 5 | Failpoints absent from default; test hardening | **PASS** | grep + binary string check; k03/k10 serialisation sound |
| 6 | Freeze blocker | **CONFIRMED, and avoidable** | section 7 |
| 7 | Findings 1-11 and D1-D9 | **PARTIAL ACCEPT** | section 8 |
| 8 | Test quality / mutations | **PASS with one weak spot** | 5 mutations, 4 caught clearly; write-before-ack caught only by k03 |
| 9 | Runs and clippy | **PASS** except the freeze test | section 5 |
| 10 | Residual risks | listed | F-07 to F-12 |

## 3. Reproduction of the defect (point 1)

Code reading (verified in code): `src/api/server.rs:1990-2000` runs the cluster leg inline in the handler after `cache.delete`; `router.rs:~1790-1830` (`invalidate_key_across_cluster`) records owed peers only on error returns, and on success hands non-primary legs to `dispatch_fanout` (memory only: bounded mpsc + `RepairLedger`, `router.rs:137`); `main.rs:~509-515` prints "nothing to flush" and returns. So (a) an issuer crash after the 204 loses unacknowledged fan-out, (b) a client disconnect during the leg drops the handler future and loses the fan-out.
Measured on UNPATCHED src (sandbox `sbx_m3_review`, patched tests overlaid; reproduced here):
- `m3_kill9_without_log_characterises_lost_invalidation`: `delete=204 (1.1 ms) ... converged_after=None stale_polls=92` (10 s window, origin max-age 120 s, so convergence is not by expiry). Log `p2_base_defect_repro.log`.
- `k21b_client_abort_during_cluster_leg_still_fans_out`: FAIL, "B never learned of the DELETE whose client aborted mid cluster-leg" (primary v2, B stale). Log `p2_base_k21b.log`. This is a main-tree defect independent of the log.
- k04, k05, k17 on unpatched src: 3 failed / 0 passed (they need the log). Log `p2_base_k05_k04.log`.
Own minimal repro: not written separately; I reused the implementer's tests after reading them (they spawn real gateway processes with a fault proxy and kill the issuer; nothing mocked). That is a reuse, not an independent construction.

## 4. Findings

Severity: Critical / High / Medium / Low / Info. "verified" = read in code or measured here.

| ID | Area | Finding | Evidence | Sev | Recommendation |
|---|---|---|---|---|---|
| F-01 | Freeze | Patch changes `Cargo.toml` and `src/lib.rs`, the only two pinned files it touches; `test_real_v2_2_freeze_record_verifies_when_present` fails closed with "Cargo.toml changed after freeze". Reproduced. Implementer says avoiding the edit is "not possible": that is not correct (section 7). | `p1_freeze.log`; hash comparison of all `files_sha256` entries: only those two differ in the patched copy | High (blocks default CI) | Prefer avoiding the edits (P-C); otherwise a dated addendum per section 7 |
| F-02 | Concurrency / DELETE | Detached `tokio::spawn` DELETE leg: client abort no longer loses fan-out (k21b passes patched, fails unpatched). Correct. A JoinError (panic) maps to 500 `PrimaryProtocolError`; hygiene `cache.delete` in the Err branch is skipped when the client already left (harmless). The task count is unbounded, but each leg is bounded by ~2x150 ms attempt timeouts plus pool acquisition. | server.rs hunk; `p1_crash_tests.log` k21b ok | Info | Accept; note as default-behaviour change (section 6) |
| F-03 | Durable log | `append_intent` bounds only the enqueue (500 ms); `orx.await` waiting for the writer has no timeout. A stalled disk / hung fsync makes every DELETE (and its detached task) wait unboundedly, which contradicts "bounded back-pressure" and the fail-open stance. | `durable_log.rs` `append_intent`: `match orx.await`; stall hook only used in a unit test that stalls 60 ms | Medium | Bound the wait (P-A) and count it in `not_durable`; add a unit test with a 5 s stall |
| F-04 | Test quality | Write-before-ack ordering is protected by one process test only. Mutation "append_intent returns without waiting for the writer": k03 fails with the right message ("the 204 must not precede the fsync"); k04, k05, k17 still pass (the writer is fast). | `p4_mp1_no_wait_for_durable.log` | Medium | Add a deterministic test: stall hook, assert `append_intent` does not complete before the batch is written/synced, and a process-level variant |
| F-05 | Drain | `drain()` completes when `pending_jobs == 0`, but tasks sitting in the broadcaster's main mpsc (capacity 1024; `enqueue` does not count them) are not counted until the dispatcher moves them to a per-peer queue. A SIGTERM in that window, or a backlog, can report `complete=true` early. With the log on the intent still protects the invalidation; with it off it can be lost. Window is small. | `router.rs` `enqueue`; patch `pending_loop.fetch_add` only at per-peer `try_send` | Low | P-B |
| F-06 | Shutdown | Shutdown with a detached DELETE leg in flight is untested (implementer says so). By reading: the runtime stays alive during the drain window so the leg usually finishes; handlers are not drained (pre-existing); legs are not counted by the drain. Not exercised. | main.rs shutdown hunk | Low | Add a graceful-stop test with a delayed primary leg |
| F-07 | Multi-process | No single-writer guard on the log. A new process started while the old one is still draining (overlap is normal in supervised restarts, now up to 2 s longer) can compaction-rename or rewrite the file while the old process keeps appending to the replaced inode/handle; a second gateway pointed at the same path with another node id triggers the "belongs to node" corrupt fallback (full flush of all peers). | `open`/`rotate`/`maybe_compact` use tmp+rename with no lock file | Medium | Lock file (P-D), or document one-process-per-path and add a startup test |
| F-08 | Replay edge | `parse`: a mid-file record whose length field is corrupted to a value larger than the remaining bytes is classified as a torn tail, so all later records are silently dropped; only crash tearing (not bit rot) is the stated model. A garbled LAST record is also dropped silently (an intent, never acknowledged under fsync=always, so consistent with the crash model). | `parse()` `rest.len() < 8 + blen` branch | Low | Document; optionally require the discarded tail to be < MAX_BODY+8 and otherwise fall back to flush |
| F-09 | Security / ACL | Unix file mode 0600 never executed (no Linux run); on Windows the directory ACL is inherited, `create_dir_all` does not harden it. The log holds canonical keys and tenant ids. | `open_append`, `create_new_file` | Low | Document; add Windows ACL note; private directory (D5) |
| F-10 | Durability | Windows rename durability: `sync_parent` is a no-op off Unix; with `Never`/`Interval` only process-crash safety is claimed. `Hooks` fault-injection code is compiled into production (always `None`). | `sync_parent`, `LogConfig::hooks` | Low / Info | Accept D9 with documentation; consider cfg(test)-gating `Hooks` |
| F-11 | Disk full / degraded | If rotation also fails the log degrades and every DELETE answers 204 without durability; the only signals are stderr lines and `LogStats`, which is not exposed on `/api/v1/metrics` (implementer notes this). Operators opting in get no alert. | `Writer::rotate`, N2 in the implementer doc | Medium | Expose `degraded/not_durable` as measured counters before calling the feature production-ready; or offer fail-closed (D1) |
| F-12 | Receiver / compat | No wire change. Replayed frames carry fresh sequence numbers; the receiver (`node.rs:507-577`) records the sequence in `anti_entropy_tracker` for gaps but applies every InvalidateRequest regardless, so an unpatched peer applies replays; a restarted issuer's low sequences can create spurious gap/reorder bookkeeping at the peer (not measured). | node.rs read; K5/K6 pass | Low | State as a limitation; test with a peer that has seen higher sequences from the same originator (K5 covers one restart only) |
| F-13 | Log growth | Bounded: per-peer image bounded by ledger limits; compaction at max(2x live size, 1 MiB), forced above 64 MiB; Ack records are folded. With `over_cap` and a failing compaction the writer retries every ~100 ms and increments `write_errors` each time (noise). | `maybe_compact` | Low | Rate-limit the error counter/log line |
| F-14 | Clock | Age-out (7 days) escalates to a per-peer full flush, never drops; a forward clock step flushes peers once; backward step does nothing. Unit-tested with injected timestamps only. | `State::age_out`, tests | Info | Accept D3 |

Correctness analysis of the ack watermark (verified by reading; no counterexample found, not proven): stamps are assigned by the single writer thread; `high_water` is stored before the intent waiter is released; every delivery captures `high_water` before sending (worker job start, `peek_batch`, `take_full_flush`); an ack clears only stamps <= that mark, and any intent whose origin change happened after the delivery started has a larger stamp. Note-owed mirrors from the ledger get later stamps and therefore stay owed after an earlier ack (redundant re-delivery, the safe direction). The unsafe direction (log forgets a debt the ledger owes) needs an ack without a delivery; acks only come from `complete/drop_permanent/flush_finished/worker success`. The intent is appended before `dispatch_fanout` and before the response can be built (`router.rs` hunk), so 204 implies the intent was written (and fsynced for `always`), subject to F-03/F-04 and fail-open. Unsafe by design: `note_owed` is a non-blocking mirror and can be dropped when the 8192-slot channel is full (counted as `write_errors`); the 204 path's Intent is unaffected.

## 5. Commands and counts

All `cargo ... -j 1` through the lock; debug; loopback; Windows. "Before" = unpatched src (with patched tests where the test did not exist before).

| Command (sandbox) | Result |
|---|---|
| `git apply`-equivalent `patch -p1` of the combined diff (patched copy) | 12 of 13 files applied; docs hunk skipped only because the sandbox had no `docs/`; applied cleanly from `03_docs_configuration.diff` later; no rejects |
| patched: `cargo build --all-targets` (no features) | exit 0 |
| patched: `cargo test --lib durable_log` | 21 passed, 0 failed |
| patched: `cargo clippy --all-targets -- -D warnings` | exit 0, no warnings |
| patched: same with `--features m3-failpoints` | exit 0, no warnings |
| patched: `cargo test --features m3-failpoints --test m3_crash_point_tests -- --test-threads=1` | 15 passed, 0 failed, 0 ignored |
| patched: `--test m3_invalidation_durability_tests -- --include-ignored --skip latency --test-threads=1` | 5 passed (includes both graceful tests; Ctrl-Break worked under this runner), 1 filtered (latency exploratory) |
| patched: `cargo test --tests --no-fail-fast` (default features; crash tests not built) | 76 targets, 891 passed, 4 failed, 12 ignored |
| ... of those 4 failures | `m6 ... freeze` (real, F-01); `m6 smoke ... git_head` (sandbox has no `.git`); `m7 docs_and_code_list_the_same_keys` and `m7 cors_origins_default_by_profile` (sandbox lacked `docs/user/configuration.md` and `ui/vite.config.js`). After adding those two files and the docs hunk, `m7_config_tests`: 24 passed, 0 failed |
| unpatched: `test_real_v2_2_freeze_record_verifies_when_present` (original Cargo.toml) | 1 passed |
| unpatched: k21b | FAILED (the failing-before test) |
| unpatched: k04, k05, k17 | 3 failed |
| unpatched: `m3_kill9_without_log...` | passed (it characterises the defect: 92 stale polls) |

I did not re-run the full unpatched `cargo test --tests`, so "only NEW failures" for the full suite rests on: freeze passes unpatched and fails patched (measured), the other three failures are sandbox-environment errors with stated causes (m7 pair confirmed fixed by supplying the files; the git_head one not re-run). The implementer's 909/1/12 and 850/1/8 were not reproduced and are not contradicted. The 12 ignored in my default run vs their 12 agrees.
Failing-before unit tests: I did not re-run the implementer's "unfixed patch" (that intermediate state is not in the diff). Reading them: partial-write, fsync, age-out, departed-member and recovery tests assert exactly the stated property (outcome `Replayed`, `rotations == 1`, flush debt, collapsed flush, `degraded` cleared), so they fail for the stated reasons if the mechanism is absent. Guards (`backward_clock_step...`, `stale_tmp...`) pass either way, as the implementer says.

### Mutations (sandbox `sbx_m3rev_mut`, restored after each; `.bak` files confirmed gone)

| Mutation | Tests run | Result |
|---|---|---|
| MU1 `apply_ack` ExactKey covers `upto + 1` (over-ack) | `--lib durable_log` | caught: `newer_intent_survives_an_ack_that_started_before_it` (1 failed, 20 passed) |
| MU2 CRC verification disabled in `parse` | same | caught: `mid_file_corruption_falls_back_to_full_flush_for_every_peer`, `truncated_last_record_is_tolerated...` (2 failed) |
| MU4 repeated intent keeps the OLD stamp (dedupe loses the newer stamp) | same | caught: `newer_intent_survives_an_ack_that_started_before_it` |
| MP1 `append_intent` does not wait for the writer (ack before durable) | k03, k04, k05, k17 | caught only by k03 ("the 204 must not precede the fsync"); k04/k05/k17 passed (F-04) |
| MP2 replay does not re-inject debts into the ledger | k04, k05, k11, k17 | caught by all four (0 passed, 4 failed) |
| MP3 skip the detached leg | not mutated again: the unpatched k21b result is exactly this change | k21b fails |

## 6. Default-behaviour changes with `PHANTOM_INVALIDATION_LOG_PATH` unset (point 4)

1. DELETE cluster leg is a detached task (all cluster-mode DELETEs). Effect: fan-out survives client aborts; JoinError -> 500. Risk low; arguably a bug fix. Existing tests unchanged and passing.
2. Shutdown: in cluster mode, SIGINT/SIGTERM now runs `router.drain` for up to `PHANTOM_SHUTDOWN_DRAIN_MS` (default 2000) and calls `router.stop()` (new). It waits while a peer that is not Failed/Offline (so Suspect/Recovering count) still owes an ack, so planned stops can be delayed up to 2 s whenever such a peer exists. Must stay below the supervisor grace. Non-cluster gateways: no router, no delay.
3. New config keys `PHANTOM_INVALIDATION_LOG_PATH`, `_FSYNC` (default `always`, ignored unless path set), `PHANTOM_SHUTDOWN_DRAIN_MS`; `m7_config_tests` and `docs_and_code_list_the_same_keys` updated (24 pass with docs).
4. Windows: a CTRL_BREAK handler is registered, changing CTRL_BREAK from default termination to graceful shutdown.
5. Data structures: `RepairEntry`/`PeerRepair` gained fields, `pending_jobs` counter, `pub(crate) tenant_of_canonical`; `ReplicaBroadcaster::new` paths unchanged in behaviour (21 cluster/repair suites pass in the implementer's full run; in my default full run all cluster suites passed).
With the log off the original defect (issuer crash after 204) remains by design (`m3_kill9_without_log...` still shows 92 stale polls).

## 7. Freeze blocker (point 6)

Reproduced: patched copy fails `test_real_v2_2_freeze_record_verifies_when_present` with "Cargo.toml changed after freeze" (`p1_freeze.log`); unpatched copy passes. I hashed every `files_sha256` entry of `preregistration_m6_v2_2_freeze.json` in both copies: the patch changes exactly `Cargo.toml` and `src/lib.rs` among pinned files (no other pinned file, `Cargo.lock` unchanged). Both are already listed in addendum 1 with a specific `current_sha256` (the Scenario B hashes), so the verifier rejects any further edit: addendum 1 cannot simply be extended (its JSON/pin are protected records, `src/lab/eval.rs`, which hardcodes addendum version 1, is itself pinned and listed). Options without touching any record:
- **A. Avoid the edits (recommended; not compiled).** `src/lib.rs`: declare the module inside unpinned `src/dist/router.rs` (`pub mod durable_log;`, file `src/dist/router/durable_log.rs`); `main.rs` is not pinned. `Cargo.toml`: no `[features]` / `[[test]]` stanzas; tests at top-level `tests/*.rs` are auto-discovered (`autotests` not disabled, no top-level test files exist), failpoints via `--cfg m3_failpoints` with `#[allow(unexpected_cfgs)]`, needing a dedicated build. Cost: loses `--features`, rebuilds deps for the crash suite, more awkward to run.
- **B. Dated addendum 2** (owner decision, protected record): must list `Cargo.toml` and `src/lib.rs` with `frozen_sha256` (from the freeze record) and NEW `current_sha256`, the reason ("M3: `[features] m3-failpoints`, two `[[test]]` stanzas; `pub mod durable_log;`; no dependency or profile change"), be pinned like addendum 1, and the verifier must accept it: the current verifier only reads addendum 1, so either addendum 1 is re-issued (changes the pinned record) or `src/lab/eval.rs` is edited (another listed file, requiring its own new hash). Either way it re-touches protected files.
- **C. Accept a failing test**: not acceptable for default CI.
Impact on the sealed evidence: both files are additive, no dependency change, so the sealed run's scientific inputs are unaffected, but the verifier by design cannot know that; a human must say so in the addendum.

## 8. Dispositions (point 7)

Findings 1-11 of `review_findings_resolution.md`: 1 (cancellation) agree and strengthen (it is a main-tree defect, reproduced unpatched); 2 agree "no counterexample, not proven" (I reached the same by reading; no model check, still recommended: a randomised interleaving test); 3, 5, 6, 8 agree; 4 agree the mechanism is the right defensive pattern, the Linux premise unverified; 7 agree (defer); 9 agree bounded; 10 agree no protocol change for now; 11 agree "partly addressed": this review is again AI-only.
New items N1-N8: agree, except N6's claim "avoiding the edit is not possible" which is wrong (section 7), and N1 under-states that Suspect/Recovering peers also hold the drain.
Policies: D1 fail-open: **accept for first opt-in release only if the metrics gap F-11 is closed**, else change to fail-closed option available; D2 `always`: accept; D3 escalate, 7 days: accept; D4 drain 2000 ms: accept, document the Suspect/Recovering wait and that the supervisor grace must exceed it; D5 private dir: accept, add Windows ACL note and a single-writer guard (F-07); D6 no protocol change: accept; D7 replay amplification bounded: accept (crash loop with large debt untested); D8 departed collapse: accept (a re-added same-id member is flushed once); D9 Windows rename durability: accept and document, process-crash only.

## 9. Disagreements with the implementer (summary)

1. "Avoiding the Cargo.toml / lib.rs edit is not possible" - possible with trade-offs (section 7).
2. "Bounded back-pressure" - only the enqueue is bounded (F-03).
3. Ordering is "covered" - by one test (F-04).
4. The drain "waits only while a reachable peer owes an ack" - also waits for Suspect/Recovering peers, and undercounts main-queue tasks (F-05).
5. Default-build safety: agree (section "Failpoints" below).
Failpoints (point 5): `grep` shows `#[cfg(feature = "m3-failpoints")]` guards the only env-reading code (`failpoint`, `PHANTOM_M3_TEST_COMPACT_MIN_BYTES`); default `failpoint` is an `#[inline(always)]` empty fn; the default-built `phantom-cache-demo.exe` contains 0 occurrences of `PHANTOM_M3_FAILPOINT` (positive control: the feature-built tests exit 137 at the named points). The crash test target has `required-features`, so default `cargo test` does not build it. A CI guard against enabling the feature in release profiles is not present (N4 recommendation stands). K3/K10 hardening (failpoints only after an intent written by the process, 1.5 s wait for warm-up acks, serialisation) is sound by reading; 15/15 passed in one run here (not a flakiness estimate).

## 10. Not checked / UNVERIFIED

Power loss, OS crash, Linux (0600 mode, fsync-error semantics, `kill`), non-console Ctrl-Break, release profile, multi-host, real network faults, real disk-full / read-only dir / clock step, performance and latency (exploratory test skipped), randomised interleaving or model checking of the ack logic, a graceful stop with a detached leg in flight, the spurious-gap bookkeeping effect at peers after issuer restart (F-12), the full unpatched `cargo test --tests` run, `git_head` smoke test with a repository, UI/telemetry exposure. Statistics: single runs; no confidence intervals.

## 11. Recommendation

**INTEGRATE AFTER FIXES.** The mechanism closes the defect for the opt-in log case, the detached leg fixes a real main-tree bug, tests are real process tests, mutations were caught, clippy is clean with and without the feature.
Blocking items:
1. F-01: resolve the freeze test (prefer option A; otherwise owner-approved dated addendum with the verifier change, listing `Cargo.toml` and `src/lib.rs` plus any file changed to support it, e.g. `src/lab/eval.rs`).
2. F-03: bound the durability wait (or document and test the unbounded behaviour as intended).
3. F-11: expose `degraded` / `not_durable` as measured counters (or ship with fail-closed available), since an operator who opts in must be able to see loss of durability.
Strongly advised: F-04 test, F-07 single-writer guard or documented constraint, F-05, and a graceful-stop-with-in-flight-leg test.
Owner approvals still required by CLAUDE.md section 2 (application-code change, new config keys). Proposal sketches (never applied, uncompiled): `docs/research/m3_durability/independent_review_proposals/README_proposals.md`.
