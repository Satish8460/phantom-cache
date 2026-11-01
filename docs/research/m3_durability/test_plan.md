# M3 durability: test plan

Status: INVESTIGATION + PROPOSED PATCH. Tests were run only in scratch copies (`D:\Temp\m3_scratch`, `D:\Temp\m3_scratch_base`, `D:\Temp\m3_cur_patched`). INTEGRATION APPROVAL REQUIRED (`design.md` section 8). Per CLAUDE.md section 5, a change to invalidation needs an independent review and a test that fails without the change: the failing-before evidence is `evidence/run_failing_before_kill9_log_OFF_01.log`.

Evidence tags: [reproduced here] = executed by me, log in `evidence/`; [not run] = specified, not executed.

## 1. Harness additions (safe to integrate; `tests/support/cluster_harness.rs`, ~150 lines)

| Helper | Purpose |
|---|---|
| `GatewayNode.bin_path`, `GatewayNode.env` | retain the full spawn specification (binary, environment, ports, identity) |
| `ClusterOptions.origin_max_age_secs`, `VersionedMockOrigin::start_with_max_age` | long `max-age` so a stale entry outlives the repair bound |
| `ClusterOptions.per_node_env` (`{id}`, `{dir}` placeholders) | per-node env such as a log path |
| `kill_node` | TerminateProcess / SIGKILL, bounded wait for reaping |
| `restart_node(id, env_overrides)` | identical command line / env / ports, log APPENDED, waits for `/health/ready` and the NEW process's pre-warm marker |
| `graceful_stop_node`, `request_graceful_stop`, `wait_node_exit` | CTRL_BREAK to the child's process group on Windows (6-line `extern "system"` declaration, no new dependency), `kill -TERM` on Unix |
| `gateway_log_lines_containing` | grep the full gateway log across restarts |

Side effect to review: every gateway is now spawned with `CREATE_NEW_PROCESS_GROUP` on Windows (needed to aim CTRL_BREAK). This disables Ctrl-C delivery to the children; the harness never relied on it (teardown is `start_kill`). Run the whole cluster suite after integrating.

## 2. Tests

| Test | Needs src patch | Can falsify | Result |
|---|---|---|---|
| `m3_negative_control_issuer_alive_peer_converges` | no | that the scenario itself (blackhole, heal, retry, repair) converges when nobody dies; guards against a broken harness | pass x3 [reproduced here] (`run_patched_binary_flag_off_01/02`, `run_ORIGINAL_code_01`) |
| `m3_kill9_without_log_characterises_lost_invalidation` | no | the claim "accepted-but-unacknowledged invalidations die with the issuer": PASSES while the defect exists, FAILS when something recovers the invalidation | pass x4 (3 scratch binary / flag off, 1 original source) [reproduced here] |
| `m3_kill9_with_log_peer_converges_after_issuer_restart` | yes, `#[ignore]` | the claim "with the log, B converges after kill -9 + restart" | pass with log ON x2 (`run_patched_kill9_log_ON_01/02`); **FAIL with `M3_FORCE_LOG_OFF=1`** (`run_failing_before_kill9_log_OFF_01`) |
| `m3_graceful_stop_drain_delivers_inflight_invalidation` | yes, `#[ignore]` | drain delivers the queued retry before exit | pass with drain 3000 ms; baseline `M3_FORCE_DRAIN_OFF=1` loses it (B stale 91 polls) [reproduced here] |
| `m3_graceful_stop_deadline_expires_log_keeps_debt` | yes, `#[ignore]` | an expired drain deadline keeps the debt in the log | pass [reproduced here] |
| `m3_delete_latency_overhead_exploratory` | yes for log-on rows, `#[ignore]` | nothing (measurement) | numbers in `reproduction_report.md` section 6 |

Unit tests in `src/dist/durable_log.rs` (`cargo test --lib durable_log`), 13 tests [reproduced here, see `evidence/run_unit_tests_durable_log_*.log`]:
`round_trip_with_acks_flush_markers_and_clean_marker`, `live_ledger_round_trip_including_inflight_complete_and_flush_marker` (real `RepairLedger` with `peek_batch` in-flight entries, `complete`, `mark_full_flush` / `take_full_flush` / `flush_finished`), `newer_intent_survives_an_ack_that_started_before_it` (stamp watermark), `fully_acked_log_compacts_to_empty_on_close`, `truncated_last_record_is_tolerated_and_earlier_records_survive` (every cut point inside the last record, a garbled last record, a zero-filled tail), `appends_after_torn_tail_recovery_are_readable`, `mid_file_corruption_falls_back_to_full_flush_for_every_peer` (also: fallback markers are durable and cleared by an ack), `bad_header_wrong_node_and_lost_log_all_fall_back`, `replay_is_idempotent_and_bounded` (3000 keys in one tenant collapse to a tenant debt; 50 duplicates dedup; two replays equal), `file_size_stays_bounded_under_churn_via_compaction` (4000 intent+ack pairs stay < 128 KiB), `tenant_ack_clears_covered_exact_entries_but_not_newer_ones`, `crc32_matches_the_standard_check_value`, `age_out_drops_old_debts_only`.

The ledger's `in_flight` / `again` flags are intentionally NOT persisted: the stamp watermark replaces them (design 4.2).

## 3. How to run (scratch layout; `-j 1` on this machine)

```
cargo test -j 1 --test m3_invalidation_durability_tests -- --nocapture --test-threads=1                  # current code + test patch
cargo test -j 1 --test m3_invalidation_durability_tests -- --ignored --nocapture --test-threads=1        # needs src patch
M3_FORCE_LOG_OFF=1 cargo test -j 1 --test m3_invalidation_durability_tests m3_kill9_with_log -- --ignored   # failing-before
cargo test -j 1 --lib durable_log
```
Check first that no `cargo.exe` / `rustc.exe` is alive (another agent may be sweeping the suite).

## 4. Not yet written / not run (do not read the above as complete coverage)

- Concurrent DELETEs + restart (many keys in flight, group commit under load).
- Kill during compaction (tmp file present, rename not yet done) and kill between rename and reopen.
- Partition + restart with several owed peers; a peer that is permanently gone (age-out).
- Log directory read-only / disk full (fail-open path and `degraded` flag).
- Restart racing a peer's pull reconciliation.
- Power-loss behaviour of `fsync=always` (cannot be tested in userland).
- Linux runs of the multi-process tests (`kill -9` / `SIGTERM` path of the harness is written but never executed).
- Release-build and concurrent latency measurements; wiring of `LogStats` into `/api/v1/metrics`.
- Mutation check that D6 (stamp watermark) fails when `apply_ack` ignores `upto`.
- Independent review.
