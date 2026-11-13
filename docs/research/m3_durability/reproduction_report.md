# M3 invalidation durability: reproduction report

Status: INVESTIGATION (defect reproduced) + PROPOSED PATCH (validated only in a scratch copy). NOTHING INTEGRATED. INTEGRATION APPROVAL REQUIRED (see `design.md` section 8 and the end of this file).

Evidence tags: [reproduced here] = executed by me on this machine, log in `evidence/`; [code] = read in the repo snapshot; [design] = proposal.
Environment: Windows 11 Home 10.0.26300, loopback only, `dev` (debug) profile, `cargo -j 1`, `CARGO_TARGET_DIR=D:\Personal\_m3_target`, TEMP/TMP=`D:\Temp`. Scratch copy `D:\Temp\m3_scratch` (snapshot of `src`, `tests`, `benches`, `Cargo.toml`, `Cargo.lock` taken from the working tree at HEAD `456bb25` + uncommitted changes, 2026-10-30 ~06:00 +05:30). A byte-identical "original snapshot" is in `D:\Temp\m3_orig`; patches are diffs of scratch against it. The working tree was not modified by this work (only `docs/research/m3_durability/` was created).

## 1. Verdict

**The defect is REPRODUCED (confirmed defect), on the current code.** [reproduced here]

With three real gateway processes (mTLS cluster, fault proxies on every link), a DELETE that the issuer answered `204` but that peer B had not yet acknowledged was lost when the issuer was killed (TerminateProcess, the Windows kill -9 equivalent) and restarted: B kept serving the stale version for the entire 10 s observation window (89-91 polls, never converged) in every flag-off run; the origin advertises `max-age=120`, so B would stay stale for about two minutes (not waited for). The negative control (same scenario, issuer not killed) converged in 231-337 ms. Therefore no existing protection (retry, repair ledger, anti-entropy, pull reconciliation) recovers the invalidation once the issuer's memory is gone, matching the classification in section 12 of the accountability report (confidence was ~90%; now reproduced).

## 2. Scenario (test `m3_kill9_without_log_characterises_lost_invalidation`, file `tests/integration/m3_invalidation_durability_tests.rs` in the scratch copy)

1. Start gateways 1 (issuer G), 2 (victim B), 3 (primary) with `with_proxies=true`; origin `Cache-Control: max-age=120`.
2. Choose a key whose canonical rendezvous primary is node 3 (scan with the production router) so G's synchronous leg succeeds while B is only reached by the asynchronous fan-out.
3. GET the key on 1, 2, 3 (all version 1, cached).
4. Mutate the origin (version 2). Blackhole the G->B proxy link.
5. `DELETE` via G: expect `204`. Observed 0.59-0.95 ms.
6. Immediately kill G (inside B's ~1.4 s delivery-retry window).
7. Heal the link, restart G with identical ports, identity and environment (new harness helper), wait for readiness and pool pre-warm.
8. Poll B every 100 ms for 10 s: any answer with version 2 means converged.

Checks: primary (3) serves version 2 right after the DELETE (synchronous leg worked); B serves version 1 right after the kill (stale, as expected); then B never converges.

## 3. Results

| Run (log in `evidence/`) | Binary | Log | Outcome |
|---|---|---|---|
| `run_patched_binary_flag_off_01.log` | scratch (flag off = original behaviour for kill -9) | off | DELETE 204 (589 us); B stale, `converged_after=None`, 89 stale polls; negative control converged in 336.9 ms; 2 passed |
| `run_failing_before_kill9_log_OFF_01.log` | scratch, `M3_FORCE_LOG_OFF=1`, correct-behaviour test | off | **FAILS** (exit 101): "B still served the stale entry 10s after the issuer restarted (stale polls 91): the owed invalidation was lost" (failing-before evidence) |
| `run_patched_binary_flag_off_02.log` | scratch | off | DELETE 204 (947 us); B stale, 90 stale polls; control converged in 231.3 ms; 2 passed |
| `run_ORIGINAL_code_01.log` | **unmodified source** (original snapshot + harness/test patch only, own target dir `_m3_target_base`) | n/a | DELETE 204 (929 us); B stale, `converged_after=None`, 89 stale polls; negative control passed (B already converged by the time of the peek, 0 ns); 2 passed, exit 0 |

3.1 The ORIGINAL-code row is the decisive one: the defect is present without any of the prototype changes, so it is not an artefact of the patch. Total flag-off / original executions: 4 (3 patched-binary-off incl. the forced-off failing-before run, 1 original), all stale for the whole window; 0 counter-examples.

Caveats [reproduced here, limitations]: Windows only; TerminateProcess is the closest available kill -9 equivalent (no handlers run, same as SIGKILL); loopback; debug build; the kill is issued within about 1 ms of the 204 by design (the window in which the invalidation is only in memory is up to ~1.4 s of retries, longer if B is Suspect). Three flag-off executions, all identical in outcome; this is a deterministic-by-construction scenario (blackholed link), not a statistical race. The repro does not show how often production traffic falls into the window.

## 4. Graceful stop (SIGTERM / Ctrl-C equivalent)

Feasibility on Windows [reproduced here]: `tokio::signal::ctrl_c` only sees CTRL_C_EVENT, which cannot be aimed at one process (GenerateConsoleCtrlEvent accepts a group id only for CTRL_BREAK). The gateway therefore needed a (proposed) `ctrl_break` listener, and the harness spawns each gateway in its own process group (`CREATE_NEW_PROCESS_GROUP`) and sends CTRL_BREAK through a 6-line `extern "system"` declaration (no new dependency). It worked from the test process on this machine (the gateway printed `[shutdown] received Ctrl-Break` and exited with code 0, i.e. through its own shutdown path, not by termination). It needs a console shared with the test runner; under a fully detached runner `GenerateConsoleCtrlEvent` can fail and the graceful tests then report `GRACEFUL_STOP_NOT_FEASIBLE` and return (they do not fail). On Unix the harness uses `kill -TERM` (not exercised here).

Baseline on the current shutdown behaviour [reproduced here, patched binary with `PHANTOM_SHUTDOWN_DRAIN_MS=0`, which is equivalent to today's exit path]: stop requested while B is blackholed, link healed 400 ms later, issuer restarted: **B stale for the whole window (91 polls, never converged)**, so a graceful stop loses the in-flight invalidation exactly like a crash (`evidence/run_patched_graceful_drain_OFF_baseline_02.log`). This confirms the report's claim that SIGTERM has the same loss, deterministically for anything queued.

## 5. Prototype results (scratch copy only)

| Test | Result | Evidence |
|---|---|---|
| Unit tests `dist::durable_log::tests` (ledger round-trip with flush markers and in-flight entries, truncated last record at every cut point, garbled last record, zero-filled tail, mid-file corruption fallback, bad header / wrong node / lost log, size bound under churn, bounded replay, idempotent replay, stale-ack safety, tenant ack, CRC vector, age-out) | 12 passed (first run, exit 0) then 13 passed (after adding the live-ledger test), exit 0, on the current working-tree files plus the patch | `evidence/run_unit_tests_durable_log_01.log`, `_02_on_current_tree.log` |
| `m3_kill9_with_log_peer_converges_after_issuer_restart` (flag ON) | PASS (exit 0): victim stale (v1) right after the kill, converged 1.6-27.5 ms after the restarted issuer was ready; issuer log: `replay outcome Replayed, 4 records, 1 owed invalidation(s) re-queued` | `run_patched_kill9_log_ON_01.log`, `_02.log` |
| `m3_graceful_stop_drain_delivers_inflight_invalidation` (drain 3000 ms, no log) | PASS: `[shutdown] invalidation drain: complete=true elapsed=438.5ms pending_jobs=0`; B had converged before the issuer restarted | `run_patched_graceful_drain_ON_02.log` |
| same, `M3_FORCE_DRAIN_OFF=1` | B stale 91 polls (baseline, section 4) | `run_patched_graceful_drain_OFF_baseline_02.log` |
| `m3_graceful_stop_deadline_expires_log_keeps_debt` (drain 1000 ms expires, log ON) | PASS: issuer printed "some peers still owe acknowledgement; the owed invalidations stay in the durable log", restart replayed 1 owed invalidation, B converged | `run_patched_graceful_deadline_log_ON_02.log` |

## 6. DELETE latency overhead (exploratory; debug build, loopback, one machine; NOT a benchmark)

3 gateways, no fault proxies, N=300 measured DELETEs per configuration after 50 unmeasured, 3 interleaved repetitions (n=900 pooled per row), client-observed wall time of `DELETE` through the issuer (primary leg included), `reqwest` keep-alive client, keys not cached. [reproduced here] `evidence/run_patched_latency_exploratory_01.log`.

| Configuration | p50 | p95 | p99 | mean |
|---|---|---|---|---|
| log off | 0.417 ms | 0.546 ms | 0.654 ms | 0.447 ms |
| log on, fsync=never | 0.474 ms | 0.558 ms | 0.644 ms | 0.488 ms |
| log on, fsync=interval (100 ms) | 0.485 ms | 0.587 ms | 0.845 ms | 0.508 ms |
| log on, fsync=always (group commit) | 1.853 ms | 2.557 ms | 3.569 ms | 1.985 ms |

Reading: the thread hand-off plus write costs about +0.05-0.07 ms at p50 (about +14 %); `fsync=always` adds about +1.4 ms at p50 on this machine's NTFS/SSD (one `sync_data` per group commit; not representative of other storage). Single sequential client, so group commit batching was NOT exercised (no concurrency); concurrent DELETEs would share fsyncs. No confidence intervals beyond the three per-repetition rows in the log (per-repetition p50 varied 0.397-0.432 ms off, 1.836-1.884 ms always). Release-build and concurrent numbers are not available.

## 7. What this does not show

- It does not show the guarantee holds under power loss (no such test) or on other filesystems/OSes (Linux not run).
- It does not exercise concurrent DELETE + restart, kill during compaction, partition + restart with several peers, or a restart racing a peer's pull. These are listed in `test_plan.md` as not yet written.
- The age-out safety net and the ledger/image divergence (design 4.4) are reasoned, not tested end to end.
- The DELETE-handler cancellation hypothesis (client disconnect between purge and intent) is NOT tested and NOT addressed.

## 8. Integration requirements (summary; details in the final report)

Application-code change and new config keys: owner approval (CLAUDE.md section 2) and an independent review pass (invalidation path, section 5). The test patch (harness + tests + `Cargo.toml` registration) is separable and has no runtime effect, but it changes how every multi-process test spawns gateways on Windows (new process group) and so should be re-run against the full cluster suite.

## 9. Validation against the CURRENT working tree (rebase check)

While this work ran, another agent edited `src/main.rs`, `src/config.rs`, `src/api/server.rs` and added `src/api/lab_api.rs` (G2 Lab API). The patches in this directory were therefore re-applied (`patch -p1`, offsets only, no rejects) to a fresh copy of the CURRENT files (`D:\Temp\m3_cur_patched`, plus `docs/user/configuration.md`, `docs/demo/*.md` and `ui/vite.config.js` copied only because existing tests/`include_str!` read them) and re-validated there [reproduced here]:

| Run | Result |
|---|---|
| `cargo test --lib durable_log` | 13 passed, exit 0 (`run_unit_tests_durable_log_02_on_current_tree.log`) |
| all five non-latency M3 tests (`--include-ignored --skip latency`) | 5 passed, exit 0: negative control, kill -9 without log (defect reproduced again, 91 stale polls), kill -9 with log (converged 1.7 ms after readiness), graceful drain, graceful deadline + log (`run_current_tree_plus_patch_m3_tests_all.log`) |
| `cargo test --test m7_config_tests` | first run 2 failed: (a) the test requires a matrix case for every config key (patch 4 adds them); (b) `ui/vite.config.js` missing in the scratch copy (unrelated). After patch 4 + the copied file: 24 passed, exit 0 (`run_m7_config_tests_on_current_tree_plus_patch_02.log`) |
| cluster regression (`cluster_harness_controls_tests`, `security_phase4a_invalidation_tests`, `cluster_followup_tests`, `cluster_prewarm_repair_tests`, `cluster_characterization_tests`) | first attempt did not compile: an existing test builds `ClusterOptions { .. }` without `..Default::default()` (patch 1b fixes it); rerun: 74 passed, 0 failed, exit 0 (characterization 4, followup 42, harness controls 6, prewarm/repair 8, phase4a invalidation 14; `run_cluster_regression_on_current_tree_plus_patch_02.log`) |

Not done: the full 242-test suite, clippy (`-D warnings`) and the whitespace check were NOT run on the patched tree (another agent was running the sweep; clippy on new code is an open item: `durable_log` has several `pub` items used only by tests/`main`, and `main.rs` compiles its own module tree, so dead-code lints must be checked).
