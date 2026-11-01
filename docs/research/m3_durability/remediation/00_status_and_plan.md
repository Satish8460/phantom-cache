# M3 invalidation durability: remediation status, gap analysis, verification, decisions

Date: 2026-10-30. Author: remediation agent M3 (an AI session; not an independent human review).
**State: fix prepared and verified in a sandbox; integration pending independent review and owner approval. The defect is NOT fixed in the main tree, and with the log unset (the default) it would remain present even after integration.**

Scope given: Priority 1 after repository integrity, "proceed with the previously authorised M3 work within its scope; protected-core integration stays behind review and validation gates and the owner's final approval". Constraints honoured: the main tree was not modified (only new files under `docs/research/m3_durability/remediation/`), no git mutation, no new dependency, no protected record edited, no use of the reserved ports, every cargo invocation serialised through `D:\Temp\locks\with_cargo_lock.ps1`, all work in `D:\Temp\sbx_m3` with `CARGO_TARGET_DIR=D:\Personal\_sbx_m3_target`, `TEMP/TMP=D:\Temp`, `-j 1`.

Main-tree observation: `git rev-parse HEAD` was `456bb25930004f1896a84c53e776e7b3ef29ee16` at the start and had moved to `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` by the final check (several commits by other agents; branch `feat/publish-phantomcache-core` unchanged). Not done by this work. The ten main-tree files the patch set touches are byte-identical to the base the patch was diffed against (md5 compared at 16:3x IST), and `patch -p1 --dry-run` of the combined diff against the main tree reports 13 files, 0 failures (re-checked after the last regeneration).

Files in this directory: `00_status_and_plan.md` (this file), `crash_point_test_matrix.md`, `review_findings_resolution.md`, `patches/` (unified diffs against the CURRENT main tree), `evidence/` (raw logs).

## 0. Summary

| Item | Result |
|---|---|
| Gap | An accepted DELETE whose fan-out is unacknowledged lives only in the issuer's memory; kill / crash / graceful stop loses it; peers serve the stale entry until the origin's freshness lifetime ends (no gateway ceiling). Reproduced again: 92 of 92 polls stale over 10 s with the log off. |
| Built | Prior prototype (opt-in durable log, replay, shutdown drain, harness, 5 tests) re-applied cleanly to the CURRENT tree (offsets only, 0 rejects); plus this round: detached DELETE cluster leg (finding 1, which also exists WITHOUT the log), write/fsync-error recovery by rewriting the log (4, 5), bounded back-pressure (6), age-out escalation instead of dropping (3), departed-member collapse (8), crash-point instrumentation behind an off-by-default cargo feature, 15 multi-process crash-point tests, 8 new unit tests. |
| Failing-before | 6 of 8 new unit tests and 1 of 15 process tests fail on the unfixed patch and pass on the fixed one (`crash_point_test_matrix.md` section 3). |
| Findings | Of the 11 preliminary findings: fixed in the sandbox with a failing-before test: 1, 3, 5, 8 (four); mechanism fixed but a platform premise or policy still open: 4 (Linux fsync semantics unverifiable here), 6 (fail-open policy is an owner decision); reviewed, no counterexample, mutation-checked: 2; refuted as unbounded and measured: 9; accepted or owner decision: 7, 10; partly addressed: 11. Details: `review_findings_resolution.md`. |
| New blocker found | The patch edits `Cargo.toml` and `src/lib.rs`, which the M6 v2.2 research freeze pins: `test_real_v2_2_freeze_record_verifies_when_present` fails with "Cargo.toml changed after freeze" on the patched tree and passes on the unpatched base. Integration needs a dated freeze addendum (a protected record, not touched here) decided by the owner. The previous round's patch had the same effect and its regression run did not include this test. |
| Verification | Clippy `-D warnings` clean (both feature settings); full suite 909 passed / 1 failed (the freeze test above) / 12 ignored / 77 targets vs the unpatched base 850 passed / 1 sandbox-artefact failure / 8 ignored / 75 targets; section 6. |
| Unverified | See section 7. |


## 1. The exact current gap (main tree, verified by reading code)

Evidence tags: **[code]** read at `file:line` in the main working tree (HEAD `456bb25` + uncommitted work, 2026-10-30); **[measured]** executed in the sandbox on this machine (Windows 11, loopback, debug build, `-j 1`), log under `evidence/`; **[inferred]** reasoned from code, not executed; **[literature/hypothesis]** not verified here.

### 1.1 What lives only in the issuer's memory

| State | Where [code] | Bound | Survives kill / restart? |
|---|---|---|---|
| Owed-invalidation ledger (`RepairLedger`: per-peer entries, tenant flush entries, `full_flush` marker, in-flight/`again` flags, stamps) | `src/dist/router.rs:137` (struct), `:229` (`record`), bounds `:54-60` (1024 entries, 256 KiB of keys, 16 tenant flushes per peer) | bounded | **No.** Plain `Mutex<HashMap>`; nothing is written anywhere. |
| Fan-out queue jobs and in-flight retries (per-peer channel, 4 attempts, 25..200 ms backoff, 150 ms/attempt) | `router.rs:~645-845`; `WorkerGuard::drop` (`:529`) moves queued jobs into the ledger, which itself dies with the process | bounded | **No.** A job exists only in channel / task memory until delivered or recorded. |
| Per-destination stream sequence counters | `DestinationSequencer`, `router.rs:33-50` (starts at 1) | n/a | **No.** Restarted issuer restarts every stream at 1. |
| Replay ring (outbound/inbound) | `InvalidationRingBuffer`, `src/dist/anti_entropy.rs:45`, 4096 entries / 4 MiB (`:9-12`), held in `VirtualNode.ring_buffer` (`src/dist/node.rs:33`) | bounded | **No.** Empty after restart. |
| Anti-entropy tracker (highest acked seq per peer, gaps) | `AntiEntropyTracker`, `anti_entropy.rs:347` | bounded | **No.** |
| The DELETE itself | `delete_cache_handler`, `src/api/server.rs:1937`; local purge at `:1990`, then `invalidate_key_across_cluster(...).await` | n/a | Not stored anywhere. A DELETE answered 204 whose non-primary fan-out is unacknowledged exists ONLY as the items above. |

Graceful shutdown does not change this: on SIGINT/SIGTERM `main.rs:509-515` prints "cache is in-memory, nothing to flush" and returns (`rt.shutdown_timeout(5 s)` at `main.rs:182`); no drain, no persistence [code]. On Windows only `ctrl_c()` is listened to (`main.rs:~564`), so a console CTRL_BREAK is not even seen as a shutdown request [code].

### 1.2 Which peers become stale, for how long

Only peers whose acknowledgement was still outstanding when the issuer died: the non-primary members (the canonical primary is invalidated synchronously before the 204: `router.rs:1790` `invalidate_key_across_cluster`; the issuer's own cache is purged first, `server.rs:1990`). A peer is exposed when (a) it was Suspect/Recovering/Failed/Offline at DELETE time (deferred straight to the ledger, `router.rs:1705-1750`), (b) its link failed during the 4 bounded retries (~1.4 s window), or (c) the broadcaster queue/semaphore diverted the task to the ledger. In all three the only copy is in memory.

Duration: the peer keeps serving the entry until it expires by the **origin's** freshness lifetime. There is **no gateway TTL ceiling**: `PHANTOM_DEFAULT_TTL_MS` is not wired (`CLAUDE.md` section 5, `src/config.rs:115`), and `PHANTOM_MAX_STALE_MS` (24 h) caps only the stale-serving window after expiry (`server.rs:1318`), not the freshness lifetime. So the bound is "whatever `max-age`/`s-maxage`/`Expires` the origin set" [code]. The reproduction used `max-age=120`.

### 1.3 Which repairs exist and what they cannot recover

| Mechanism | What it repairs [code] | Why it cannot recover an invalidation lost with the issuer |
|---|---|---|
| Bounded retry in the per-peer worker | transient link failures within ~1.4 s | runs in the issuer |
| Repair ledger + repair worker (`repair_all_peers`, 250 ms, `router.rs:1073`; `repair_peer` ~`:1900`) | debts of a living issuer, with tenant/full-flush escalation | the ledger is the thing that is lost |
| Pull anti-entropy (`reconcile_peer_anti_entropy`, `router.rs:2128`) | a node that missed frames pulls from a peer's replay ring and replays or flushes | the restarted issuer's ring is empty and its sequences restart at 1 [code]; a peer pulling from it receives nothing to replay for the lost event. [inferred] the pull digest cannot express "an event I never saw was sent". Measured: no convergence in 89-92 polls over 10 s in 5 of 5 flag-off / original-code runs. |
| Peer-side expiry | everything, eventually | bounded only by the origin freshness lifetime (no ceiling) |
| Client retry of the DELETE | everything, if the client retries | the client was told 204 and has no reason to retry |

### 1.4 Reproduction: what is MEASURED and what is inferred

[measured, prior round, re-confirmed in this round] Three gateways, mTLS, fault proxies, origin `max-age=120`. Key chosen so node 3 is the canonical primary. GET on 1,2,3 (v1) -> origin mutated to v2 -> G->B link blackholed -> `DELETE` via node 1 answered **204** -> node 1 killed with TerminateProcess -> link healed -> node 1 restarted with identical identity and ports -> poll B every 100 ms for 10 s. B served v1 in **every** poll (89-92) in every run without the log (original-code build: 1 run; patched build with the log off: 3 runs; this round, patched build with the log off: 1 run, 92 polls). Negative control (issuer not killed): B converged in 231-337 ms in 4 runs. The primary served v2 right after the DELETE (synchronous leg worked).

[inferred] The ~1.4 s retry window, the 250 ms repair tick and the "peer stays stale until `max-age` expires" extrapolation (about 2 minutes here): the test stops observing after 10 s. [not measured] How often production traffic lands in the window; behaviour on Linux, in release builds, on multiple hosts, or with real network faults; the effect on real clients.

Limits of the reproduction: the kill is issued ~1 ms after the 204 by construction; a blackholed link is a deterministic stand-in for "peer unreachable", not a statistical race. The repro and the fix were originally produced by the same agent; this remediation round (a different agent) re-ran the original-code scenario independently (see section 4).

### 1.5 One more exposure found in this round (not in the earlier report): client abort

[code] The server drops the handler future when the client disconnects. In the main tree the cluster leg runs inline in `delete_cache_handler` (`server.rs:~1993-2000`), so a client that gives up while the primary leg is in flight cancels it: the local purge already happened, the primary may have applied the frame, and **no ledger entry exists for any peer** (`record_owed_to_all` is only reached on an error return, `router.rs:1790+`). [measured, sandbox, on the proposed patch before the fix] test `k21b_client_abort_during_cluster_leg_still_fans_out`: client timeout 40 ms against a 120 ms primary leg: the primary served v2, node B never converged within 8 s (FAILED). This exists with or without the durable log; the log makes it worse (the intent can reach the log while `dispatch_fanout` never runs). The remediation detaches the leg (section 3, finding 1).

## 3. What was built (the patch set)

Base: the CURRENT main tree files, copied to `D:\Temp\sbx_m3_orig` before any change. `patches/*.diff` are `diff -u` of that base against the sandbox (`patch -p1` from the repository root).

| Patch | Contents |
|---|---|
| `00_cargo_toml_features_and_test_registration.diff` | feature `m3-failpoints` (OFF by default); registers `m3_invalidation_durability_tests` and `m3_crash_point_tests` (the latter `required-features`) |
| `01_tests_harness_and_crash_point_tests.diff` | harness: `kill_node`, `restart_node`, `graceful_stop_node`, `request_graceful_stop`, `wait_node_exit`, `gateway_log_lines_containing`, `spawn_node_raw`, `wait_node_ready`, `http_delete_with_timeout`, `ClusterOptions.{origin_max_age_secs, per_node_env}`, process group + CTRL_BREAK on Windows; the 5 M3 tests of the prior round; `m3_crash_point_tests.rs` (15 process tests); one test file fix (`cluster_harness_controls_tests.rs`, `..Default::default()`) |
| `02_src_durable_log_ledger_hooks_drain_detached_leg.diff` | `src/dist/durable_log.rs` (new), `router.rs` (ledger hooks, `drain`, `durable_intent`, failpoints), `main.rs` (open + replay before the repair worker, shutdown drain, `ctrl_break`, failpoints), `server.rs` (detached DELETE leg, one failpoint), `config.rs` (3 keys), `lib.rs` |
| `03_docs_configuration.diff`, `04_m7_config_tests_matrix.diff` | documentation of the 3 keys; the `m7_config_tests` matrix cases the test requires for every key |
| `m3_remediation_combined.diff` | all of the above in apply order |

Changes beyond the prior prototype (this round): see `review_findings_resolution.md`. Not touched: `src/core/**`, `src/intel/**`, `src/dist/frame.rs`, `anti_entropy.rs`, `node.rs`.

## 4. Independent re-run of the original reproduction (task 1)

The prior reproduction's logs were read (`m3_durability/evidence/run_ORIGINAL_code_01.log`, `run_failing_before_kill9_log_OFF_01.log`, ...). In this round, on the unfixed patch rebuilt in a fresh target directory: negative control converged in 231 ms; kill -9 with the log OFF: DELETE 204 (0.9-1.3 ms), B stale 92 polls, `converged_after=None`; kill -9 with the log ON: 1 owed invalidation replayed, B converged 1.8 ms after readiness. MEASURED vs INFERRED: see the gap section 1.4. The reproduction is deterministic-by-construction (blackholed link), not a frequency estimate.


## 5. Open design choices: PROPOSALS for the owner (nothing here is decided)

Decision owner for all rows: the product owner (`D-P1-02` in `docs/research/vision/three_pillar/01_distributed_systems.md`), after the independent review. "Implemented in sandbox" means the sandbox patch contains the mechanism; it does not mean the behaviour is approved.

| # | Choice | Options | Recommendation (PROPOSAL) | Rationale and cost | Status in the sandbox patch |
|---|---|---|---|---|---|
| D1 | Log cannot be written: fail-open vs fail-closed | open: DELETE still 204, loss visible in `LogStats` / log lines. closed: DELETE answers 5xx when the intent cannot be made durable | **fail-open for the first opt-in release, with the recovery mechanisms below; make fail-closed a setting (`PHANTOM_INVALIDATION_LOG_ON_ERROR=open\|closed`) in a follow-up** | A 5xx after the local purge and the primary ack undoes neither (the client is merely told to retry; DELETE is idempotent). Fail-closed buys "204 always means durable" at the price of availability on a disk problem. A deployment whose correctness requirement outranks availability should choose closed. Needs a `DistError` variant and a `server.rs` mapping (small). | Fail-open kept. New: (a) a write error or fsync error no longer degrades silently: the file is rewritten from memory into a new fsynced file and the intent is then genuinely durable (`Writer::rotate`); (b) a full channel applies up to 500 ms of async back-pressure before giving up; (c) `LogStats::{degraded, rotations, not_durable, write_errors}`; (d) a start-up failure to open the log is fatal (exit 2), proven by K15a. **Not done:** wiring `LogStats` to `/api/v1/metrics` (follow-up, needs an API decision). |
| D2 | fsync policy | always (group commit) / interval 100 ms / never | **`always` default** | Only `always` survives power loss / OS crash. Prior exploratory measurement (debug, loopback, NTFS/SSD, single sequential client, NOT a benchmark): p50 0.42 ms off, 0.47 ms never, 0.49 ms interval, 1.85 ms always. Not re-measured this round. | Unchanged. After any fsync error the old file is never trusted again (finding 4). |
| D3 | Age-out of old debts | drop after N days (prototype) / escalate / monotonic | **escalate to one full flush of that peer, never drop; keep 7 days; 0 disables** | The wall clock cannot tell "old" from "stepped forward", and there is no gateway TTL ceiling, so dropping can leave a peer stale beyond 7 days. The flush costs one cold-cache event per affected peer, only after a week of non-delivery or a clock step. A monotonic clock does not survive a restart; a boot counter or a sequence cannot measure elapsed time. A pending flush marker is constant size and never ages. | Implemented (finding 3). Unit-tested; real clock steps are NOT tested (needs a VM, K16). |
| D4 | Shutdown drain default | 0 (today) / 2000 ms (prototype) / larger | **2000 ms, but it must stay below the supervisor's termination grace** | Drain only waits while a peer that is reachable still owes an acknowledgement, and it exits at once when nothing is owed. Cost: planned stop is delayed up to the deadline when a peer is slow. With the log on, an expired deadline loses nothing; with the log off it logs a warning naming the peers. The measured graceful run on this host drained in 443 ms (peer healed after 400 ms). | Unchanged. |
| D5 | Log location and permissions | any path / required private dir | **Document: a dedicated directory on persistent storage, private to the service account; refuse to start if the parent directory is world-writable on Unix (follow-up)** | The file holds canonical keys (paths, query strings, tenant ids): same sensitivity class as cache contents. Unix files are created `0600`; Windows inherits the directory ACL (the harness does not set one). On tmpfs the log AND its sentinel vanish together on reboot, which is indistinguishable from a first start: durability is silently void. Keys cannot be redacted because replay needs them. | Unchanged except tests; the Unix permission check is not implemented. |
| D6 | What a restarted issuer announces; do peers need protocol support | no wire change (prototype) / `boot_id` in the heartbeat or hello frame | **No protocol change for M3 closure.** Treat `boot_id` as a separate item (P1 section 5.2) | The log repairs the **debt**; it does not let a peer **notice** that the issuer lost state. A `boot_id` would let a peer flush or pull when it sees an issuer without a log restart (the log-off case, or a lost log). That is a wire-format decision (frame header or hello), a compatibility break for mixed-version clusters, and belongs to the owner and to P3. Receivers already apply every `InvalidateRequest` regardless of sequence, so replayed frames work without it. | Not implemented (protocol change; owner decision). |
| D7 | Replay amplification | rate limit / "replay once per incarnation" / accept | **Accept as bounded; no limiter** | Replayed debts are acknowledged within one repair tick (250 ms) when the peer is reachable, and the ack clears them; a debt is re-sent only while its ack is missing. A corrupt-log or lost-log fallback is one full flush per peer, itself durable until acknowledged: **measured** K14: the flush ran once, a second restart sent none. A crash loop faster than 250 ms with an unreachable peer repeats at most one delivery attempt per owed item per boot (K11: three boots, one debt each, no growth). The cost is cold caches on peers during a crash loop, not unbounded memory. | Measured (K11, K14), no code. |
| D8 | Departed members | keep per key / drop / collapse | **Collapse to one pending full flush per departed peer** (the ledger's own rule for removed members, `router.rs:~1085`) | Never forgotten, constant size, no per-key replay forever. | Implemented (finding 8), unit-tested. |
| D9 | Windows durability of the compaction rename | `MoveFileExW(WRITE_THROUGH)` / accept | **Accept; document** | `sync_parent` is a no-op off Unix. Process-crash safety does not depend on it; power-loss safety of the rename on NTFS is not established. Fixing it needs a Win32 call without a new dependency (`windows-sys` is not a direct dependency). | Not changed. |

## 6. Verification report (sandbox, debug profile, loopback, one Windows host)

Environment: Windows 11 Home 10.0.26300, rustc/cargo as installed on this host (clippy 1.98.0 docs link), `cargo -j 1`, `CARGO_TARGET_DIR=D:\Personal\_sbx_m3_target`, `TEMP/TMP=D:\Temp`, every invocation through `with_cargo_lock.ps1`. All numbers are from debug builds. Raw logs (ANSI stripped): `evidence/`. Sandbox: `D:\Temp\sbx_m3` (patched), `D:\Temp\sbx_m3_orig` (the CURRENT main-tree files before any change, used as the diff base), `D:\Temp\sbx_m3_base` (full unpatched copy for the baseline run), `D:\Temp\sbx_m3_mut` (mutation copy).

### 6.1 Applying the proposed patch set to the CURRENT tree (task 2)

`patch -p1 --dry-run` of all six prior parts (0, 1, 1b, 2, 3, 4) against the current files: every part applied; `main.rs` hunks with offsets of 5-25 lines, no rejects, no fuzz. Applied for real in the sandbox. Rebase work needed: none for the old parts. The set was then extended (this round) and regenerated as one diff against the current tree: `patches/*.diff`. `patch -p1 --dry-run` of `m3_remediation_combined.diff` against the real main tree: 13 files, 0 failures (exit 0); the ten pre-existing files it touches are byte-identical (md5) to the base. No trailing whitespace and no CR in added lines.

### 6.2 Build, lint, tests

| Check | Command (all `-j 1`, via the lock) | Result | Log |
|---|---|---|---|
| Unfixed patch, unit | `cargo test --lib durable_log` | 13 passed (prior unit tests on the current tree) | `A0_*` |
| Unfixed patch, new unit tests | same, with the 8 new tests added | **15 passed, 6 FAILED** (the 6 are the failing-before tests) | `B1_*` |
| Fixed, unit | same | **21 passed, 0 failed** | `C1_*` |
| Mutation (D6): `apply_ack` ignores `upto` (ExactKey, Tenant and FullCache arms) | same, in `sbx_m3_mut` | **2 failed, 19 passed**: `newer_intent_survives_an_ack_that_started_before_it` and `tenant_ack_clears_covered_exact_entries_but_not_newer_ones` fail, so the watermark logic is covered by tests that can fail. One combined mutation only; not exhaustive | `D1_*` |
| Unfixed patch, process tests | `cargo test --features m3-failpoints --test m3_crash_point_tests --test m3_invalidation_durability_tests` | crash-point target: **14 passed, 1 FAILED (k21b)**; cargo stopped before the second target, which was run separately: 5 passed (graceful tests really executed: Ctrl-Break worked) | `A1_*`, `A2_*` |
| Fixed, process tests | same with `--no-fail-fast --include-ignored --skip latency --test-threads=1` | **15 passed + 5 passed** | `E1_*` |
| Fixed, final source text (after the clippy-only cosmetic edits) | `cargo test --features m3-failpoints --lib --test m3_crash_point_tests --test m3_invalidation_durability_tests -- --include-ignored --skip latency --test-threads=1` | **40 + 15 + 5 passed, 0 failed** | `H3_*` |
| Clippy | `cargo clippy --all-targets -j 1 -- -D warnings` and again with `--features m3-failpoints` | **clean, both** (whole tree, all targets). A first run found 3 findings in new code (`result_unit_err` on the public `append_intent`, `field_reassign_with_default`, `bool_assert_comparison`); fixed (public `LogError` type) and re-run | `G3_*` |
| Whitespace | `grep` of added lines for trailing blanks / CR | none | - |
| Full suite (final) | `cargo test --tests -j 1 --no-fail-fast --features m3-failpoints` | **77 targets, 909 passed, 1 failed, 12 ignored** | `H2_*` |
| Full suite, unpatched base copy (same moment, same sandbox method) | `cargo test --tests -j 1 --no-fail-fast` in `sbx_m3_base` | **75 targets, 850 passed, 1 failed, 8 ignored** | `H0_*` (in `evidence/` after the final copy) |

Comparison with the main-tree baseline recorded in `CLAUDE.md` (851 passed / 0 failed / 8 ignored / 75 targets):
- The unpatched sandbox base reproduces it: 850 passed + the 1 failure = 851, 8 ignored, 75 targets. The single failure in the base is a SANDBOX ARTEFACT: `m6_research_evaluation_tests::test_real_v2_2_freeze_record_verifies_when_present` needs `lab-results/tuning_v2_2*`, which were left out of the copy (48 MB `lab-results/` excluded by instruction). With those two directories copied in, the base run's freeze test PASSES and the patched build fails it (`F2_*`).
- The patched run adds 2 targets (`m3_invalidation_durability_tests`, `m3_crash_point_tests`), 4 ignored tests (3 that need the patch plus the exploratory latency test) and 59 passes: 15 crash-point + 2 non-ignored M3 tests + the 21 `durable_log` unit tests counted twice, because `src/main.rs` compiles its own module tree and runs them in the binary target as well as in the library target (21 x 2 = 42; 42 + 15 + 2 = 59). 850 + 59 = 909.
- The one failure in the patched run is the freeze test, and in the patched sandbox it fails FIRST on `Cargo.toml changed after freeze` (finding N6): the patch edits `Cargo.toml` and `src/lib.rs`, which the M6 v2.2 research freeze pins. This is a real integration consequence, not a sandbox artefact; see `review_findings_resolution.md` N6.
- An intermediate full run (`H1_*`, before test hardening) had 906 passed / 4 failed: the two crash-point failures (`k03`, `k10`) were a test-design race when the binary's tests ran in parallel (armed restart replaying a warm-up debt); fixed and recorded as N8; the other two were the freeze test and a sandbox-only smoke test that needs a `git` HEAD (a throwaway `git init` inside the sandbox fixed it).
- Flakiness: after the hardening the 15 crash-point tests passed in 3 of 3 executions (`E1` was before the hardening but serial; `H2`, `H3` after). Before it: `A1` 14/15 (k21b is the genuine failing-before), `H1` 13/15. Three or four executions are not a reliability estimate.

### 6.3 Existing suites that touch the ledger (task 2)

Covered by the full run (`H2`, all passed): `distributed_consistency_tests`, `cluster_prewarm_repair_tests`, `cluster_followup_tests`, `cluster_harness_controls_tests`, `cluster_characterization_tests`, `cluster_propagation_latency_tests` (m9), `security_phase4a_invalidation_tests`, `security_phase4b_router_tests` (p4b), `security_phase4c_antientropy_tests`, `security_phase3c_pooling_tests`, `p2_dist_network_tests`, `m7_config_tests` (24 tests, includes the matrix cases for the three new keys), `known_gaps`. No existing test needed an assertion change except the unit test `age_out_drops_old_debts_only`, rewritten to the new escalate-not-drop semantics (renamed `age_out_escalates_old_debts_to_a_flush_and_keeps_young_ones_of_other_peers`).

## 7. What remains UNVERIFIED

- No release-profile run of anything (all numbers are debug builds); no latency re-measurement this round (the earlier exploratory table is reproduced in `m3_durability/reproduction_report.md` section 6 and is NOT a benchmark).
- No Linux run (K19): the harness' `kill`/`kill -TERM` path and the Unix file-permission code (`0600`) have never been executed; Linux fsync-error semantics (finding 4) are a literature-level premise.
- No power-loss or OS-crash test (K18); `fsync=always` durability across power loss is untested. NTFS rename durability unverified (D9).
- No multi-host run, no real network faults (only the loopback fault proxy), no cluster larger than 4 nodes.
- No real disk-full, read-only directory or system clock step; those are injected (hooks, timestamps).
- Review is AI-only: the original author, this remediation agent and the sandbox runs are all AI sessions in one workflow. No human or second-party review exists.
- No mutation testing beyond the one `apply_ack` run; no model checking of the ack watermark; no randomized interleaving test.
- Process-level coverage of overflow to tenant/full-flush at scale, mid-file corruption, a crash loop faster than the repair tick, and a DELETE leg in flight at shutdown.
- `LogStats` are not exposed on `/api/v1/metrics`; no UI or telemetry labelling was added.
- The statement "peers serve the stale entry until `max-age` expires" is an extrapolation beyond the 10 s window (`max-age=120`); only 10 s were observed.
- The main tree still contains the defect. With the log unset the integrated build would too.

## 8. Blockers and risks encountered

- Cargo lock contention with other agents: waits of several minutes to ~20 minutes per job; no job failed because of it. The shared lock script is honoured; no cargo job was run outside it.
- One scripting mistake in this session: a `perl` substitution with `|` as the delimiter corrupted the first line of the sandbox's `durable_log.rs`; detected by `grep`, repaired, and the repaired file compiled and passed all 21 unit tests. The main tree was not involved. A stray file `H1.out` created by a mistyped background command in the main repository root was removed (my own file, 1174 bytes, an error message from `pwsh`).
- No blocker remains for the sandbox work. Integration is blocked on: a dated freeze addendum for `Cargo.toml` and `src/lib.rs` (N6, owner decision on a protected record); independent review; owner approval (CLAUDE.md section 2: application-code change and new configuration keys); owner decisions D1-D9.

## 9. Independent-review brief


For the lead to hand to a reviewer who is **not** the author of the original patch (an earlier agent), **not** the author of this remediation (this agent) and ideally **not** an AI session that has read the conclusions in `00_status_and_plan.md`. Give the reviewer the diffs and the code, not the conclusions (CLAUDE.md section 7). Authority for requiring this review: CLAUDE.md section 5 ("Changes touching fencing, epochs, or invalidation require an independent review pass and a test that fails without the change").

**State of the artefact under review:** a patch set prepared and verified in a sandbox (`D:\Temp\sbx_m3`). The main working tree is unchanged. Integration is pending this review and the owner's approval (CLAUDE.md section 2: application-code change and new configuration keys).

### 1. What to review (files and line ranges in the SANDBOX tree after the patch; the diff is `patches/m3_remediation_combined.diff`)

| File | Change | Range (sandbox) |
|---|---|---|
| `src/dist/durable_log.rs` | NEW, ~1790 lines (about 1260 production, rest unit tests) | whole file; reviewing order: format and parser 459-680, `State` (images, collapse, `apply_ack`, `age_out`, `collapse_departed`) 239-455, `DurableLog::open` 756-910, `append_intent` 933-962, `Writer::{rotate, write_batch, sync_file, run, maybe_compact}` 1021-1265 |
| `src/dist/router.rs` | hooks only; no change to routing, fencing, epochs | `RepairEntry.durable_w` (85-90), `RepairLedger.durable` + `attach_durable`/`durable_ack` (159-192), `record` mirror (278-282), `mark_full_flush` mirror (373-378), `take_full_flush`/`flush_finished` (413-417, 471-476), `peek_batch`/`complete`/`drop_permanent` (484-535), `pending_jobs` + `WorkerGuard` (557-600), worker loop (715-850, `durable_w` captured BEFORE the first send, ack only when `delivered`), `durable_intent` + `drain` (1838-1905), call sites in `invalidate_key_across_cluster` (1974-1985, 2019-2025) |
| `src/api/server.rs` | the cluster leg of DELETE runs in a detached `tokio::spawn` and is awaited | 1988-2012 |
| `src/main.rs` | log open + replay BEFORE the repair worker starts, attach after replay; shutdown drain; Windows `ctrl_break`; failpoint calls | 31 (module), 279-283, 376-420, 462, 548-575, 630-650 |
| `src/config.rs`, `src/lib.rs` | three config keys; module declaration | config.rs 122-130, lib.rs 29-35 |
| `Cargo.toml` | feature `m3-failpoints` (off by default); test target `m3_crash_point_tests` has `required-features` | |
| `tests/` | harness (`kill_node`, `restart_node`, `spawn_node_raw`, `wait_node_ready`, CTRL_BREAK, process group), 5 M3 tests, 15 crash-point tests, 8 new unit tests | |

Not touched (verify with `git diff --stat`): `src/core/cache.rs`, `src/core/**`, `src/intel/**`, the wire format (`src/dist/frame.rs`), anti-entropy (`src/dist/anti_entropy.rs`), node handling of inbound frames (`src/dist/node.rs`).

### 2. The 9-point checklist (updated with this round's findings)

1. **Contract match.** Guarantee in `design.md` section 2: "a DELETE answered 204 whose fan-out has not been acknowledged by every non-primary member is re-delivered after any restart of the issuer until acknowledged". Check each exit of `invalidate_key_across_cluster`: success via primary (intent awaited at `router.rs:~1978` before `dispatch_fanout`), success via acting primary (`~2019`, with the suspect primary added as `extra`), `Rejected` and all other errors (debts go through `record_owed_to_all` -> `record` -> `note_owed`, non-awaited, by design: the client got a 5xx). Confirm no path yields 204 without the intent being durable while the log is healthy. NEW: the call now runs inside a spawned task (`server.rs`); confirm the handler still maps every `DistError` as before and that a JoinError (panic in the leg) maps to 500, not 204.
2. **Stamp/watermark soundness (D6).** Re-derive that `ack(upto = w)` cannot clear an intent that follows the delivery's start. Attack: `note_owed` before the ledger lock; `peek_batch` capturing `hw` before the lock; `collapse_largest_tenant` carrying the max stamp; flush absorbing intents; tenant ack clearing exact entries; `restore_full_flush` keeping an older `flush_w`; NEW: the age-out and departed-member escalation bump `State.counter` and create flush stamps: confirm `high_water` is initialised from `state.counter` AFTER those bumps (`open`, `high_water = AtomicU64::new(state.counter)`) so the replayed flush is covered by its own ack. A mutation run (ack ignoring `upto`) is recorded in `evidence/`: confirm it is meaningful.
3. **Replay correctness.** `main.rs` replays through `RepairLedger::record(peer, None, ...)` BEFORE `attach_durable` (no double append). Bounds are enforced separately in `State::add_entry` and in `RepairLedger::record` (acknowledged divergence); the age-out (now escalate-to-flush) is the safety net. Check that escalation really terminates: an acked flush clears the marker.
4. **Failure paths (CHANGED this round).** Write error and fsync error now call `Writer::rotate` (rewrite the image into `<path>.tmp`, `sync_all`, atomic rename, reopen). Check: (a) a crash between `write_all(tmp)` and `rename` leaves the old complete file (K9); (b) a crash after `rename` before reopen leaves the new complete file (K10); (c) `rotate` is attempted on every later batch until it succeeds (`needs_rotation`), `degraded` is set/cleared consistently; (d) Windows rename over an open file (the existing compaction relies on the same behaviour); (e) on Linux, whether a failed `fsync` on the OLD descriptor followed by the rewrite is sufficient (literature-level; NOT testable on this host); (f) torn-tail vs mid-file classification in `parse`; (g) sentinel logic; (h) permissions.
5. **Concurrency and async safety.** No `std::sync::Mutex` held across `.await` (CLAUDE.md section 5). `append_intent` now loops with `tokio::time::sleep(5 ms)` (bounded 500 ms) while the channel is full: verify it never blocks a runtime worker and cannot hold a lock. `DurableLog::close` blocks up to 10 s: it is called only on the shutdown path. Lock order unchanged: the ledger mutex is a leaf; `try_send` under it is non-blocking.
6. **Cancellation (RESOLVED in the sandbox, verify).** Detached leg in `server.rs`. Check that the spawned task owns everything it needs (it clones `Arc<RendezvousRouter>` and the canonical key), that dropping the handler future cannot cancel it, and consider whether unbounded task growth is possible under client-abort floods (each task is bounded by the 150 ms leg timeouts plus the awaited group commit, and by the semaphore/queues already in the router; confirm).
7. **Interaction with fencing and epochs.** The patch does not touch `core/cache.rs`. Replayed Tenant and FullCache debts call `try_invalidate_tenant` / `try_invalidate_all` on peers and so bump tenant/global epochs. Check the flush amplification claims (D7) against the K11 and K14 evidence and decide whether a limiter is nevertheless required.
8. **Docs and config.** Three keys (`PHANTOM_INVALIDATION_LOG_PATH`, `_FSYNC`, `PHANTOM_SHUTDOWN_DRAIN_MS`) and their defaults; `m7_config_tests` matrix; the `m3-failpoints` feature must be absent from every release/CI profile (add a CI guard). The overclaims in ADR-003 4.1 / G2 / G3, `limitations.md`, `operate.md` remain mandatory regardless of this patch.
9. **Regression.** Whole suite, `cargo clippy --all-targets -j 1 -- -D warnings` (also with `--features m3-failpoints`), whitespace check. The harness spawns every gateway in its own process group on Windows (disables Ctrl-C delivery to children). Results of the sandbox runs are in `00_status_and_plan.md` section 6; rerun them independently.

### 3. Invariants the reviewer must confirm are preserved

- **Generation fencing:** an origin fetch records `op_gen`; `finalize_fetch` stores only if `state.generation == op_gen` and epochs still match; `delete` bumps the generation. No change expected: grep the diff for `generation`, `finalize_fetch`, `op_gen` (must not appear).
- **Tenant/global epochs:** no change; but note replayed Tenant/FullCache debts reach `try_invalidate_tenant` / `try_invalidate_all` on peers.
- **Lock order:** shard mutex, then `tenant_epochs` mutex. The diff must not introduce a lock in `core::cache`.
- **No `std::sync::Mutex` across `.await`:** new `Mutex` uses: `DurableLog.join` (sync, shutdown only), `RepairLedger.peers` (existing, leaf).
- **Resource bounds explicit (counts and bytes):** command channel 8192 entries; batch 512; record body <= 8192 B; per-peer image bound equal to the ledger bound (1024 entries, 256 KiB, 16 tenant flushes); file compaction at max(1 MiB, 2x) and a 64 MiB hard cap; back-pressure 500 ms; `close` 10 s. Check that rotation (which rewrites the whole image) is also bounded by those caps.
- **Hard staleness cap / `stale_cap_violations`:** untouched; confirm.
- **Read path zero contention:** untouched; confirm no new lock on the GET path.
- **Default behaviour with the log unset:** byte-for-byte as today, except (a) the DELETE cluster leg is now spawned (applies with or without the log), and (b) the shutdown path now waits up to `PHANTOM_SHUTDOWN_DRAIN_MS` (default 2000 ms) while a reachable peer still owes an acknowledgement. Both must be called out to the owner as default-behaviour changes.

### 4. Tests the reviewer should re-run and attack

- `cargo test -j 1 --lib durable_log` (21 tests; 6 fail on the unfixed patch: see `evidence/`).
- `cargo test -j 1 --features m3-failpoints --test m3_crash_point_tests --test m3_invalidation_durability_tests -- --include-ignored --skip latency --test-threads=1 --nocapture` (needs a console shared with the test runner for the two Ctrl-Break tests).
- The mutation in `evidence/D1_mutation_ack_ignores_upto_2_tests_FAIL.log`: change `apply_ack` to ignore `upto` and confirm the tests fail.
- Independently re-run K1, K4, K5 (the earlier note asked for this because the reproduction and the fix shared an author).
- Add: a model or randomized interleaving test for the ack watermark (none exists); a Linux run of the whole matrix (K19); a crash-consistency run under a filesystem fault injector (K18); a real clock-step run in a VM (K16).

### 5. Human and external review requirements (cannot be met by an AI agent alone)

1. A human engineer who owns the invalidation path signs off the contract in `design.md` section 2 and the default behaviour changes in section 3 above.
2. A person who is not the author reviews the failure-handling code for filesystems other than NTFS (Linux ext4/xfs: fsync-error semantics are literature-level here).
3. Security review of the on-disk artefact: the log contains canonical keys (paths, query strings, tenant ids); decide directory permissions and retention (`D5`).
4. Owner decisions D1..D9 (`00_status_and_plan.md` section 5) recorded in `docs/research/` with the decision owner.
5. Physical or VM power-cut test (K18) and a multi-host run, both labelled manual and non-reproducible; neither exists.
