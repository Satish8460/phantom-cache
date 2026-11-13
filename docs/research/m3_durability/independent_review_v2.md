# M3 Durable-Invalidation Remediation v2: Independent Technical Review

**Review Type:** Independent Technical Review (AI Engineering Review)  
**Date:** 2026-10-30T22:15:00+05:30  
**Subject:** DEC-027 M3 Remediation v2 (`docs/research/m3_durability/remediation_v2/patches/`)  
**Sandbox:** `D:\Temp\sbx_m3fix`  
**Review Verdict:** **PASS — RECOMMENDED FOR INTEGRATION UPON EXPLICIT OWNER APPROVAL**

---

## 1. Executive Summary & Independence

This independent review evaluates the DEC-027 M3 Remediation v2 patch set designed to resolve the lost-invalidation defect on node crashes. The review was conducted entirely within sandbox `D:\Temp\sbx_m3fix`, ensuring that the main repository working tree was completely unaffected.

Remediation v2 successfully addresses all critical findings (F-01 through F-14) identified during the v1 review. Most notably, v2 completely resolves the research freeze violation (F-01) by declaring the durable log module within unpinned `src/dist/router.rs` rather than modifying pinned files `Cargo.toml` and `src/lib.rs`.

---

## 2. Review Findings Disposition Matrix (v1 -> v2)

| Finding ID | Severity | Remediation v1 Gap | Remediation v2 Resolution | Sandbox Test Evidence | Review Verdict |
|---|---|---|---|---|---|
| **F-01** | High | Modified pinned `Cargo.toml` & `src/lib.rs`, failing M6 freeze test. | Declares `#[path = "durable_log.rs"] pub mod durable_log;` in `src/dist/router.rs`. Zero pinned file edits. | `test_real_v2_2_freeze_record_verifies_when_present` passed in 0.21s. | **CLOSED** |
| **F-02** | Info | Client abort lost fan-out leg. | `tokio::spawn` detached cluster leg in `src/api/server.rs`. | Crash-point test `k21b` passed (B converged in 2.06ms). | **CLOSED** |
| **F-03** | Medium | Durability wait on disk write was unbounded. | Bounded by `PHANTOM_INVALIDATION_LOG_WAIT_MS` (default 2000ms; min 50ms, max 60000ms). Fail-open on timeout. | Unit test `f03_stalled_writer_bounds_the_durability_wait` passed. | **CLOSED** |
| **F-04** | Medium | Write-before-ack ordering had only one process test. | Added router-level test and unit-level ordering tests. | `f04_router_does_not_return_before_the_owed_set_is_in_the_log_file` passed. | **CLOSED** |
| **F-05** | Low | Drain undercounted tasks sitting in main queue. | Broadcaster tracks queued tasks; `drain()` waits for empty queue. | `f05_drain_counts_tasks_still_in_the_broadcaster_main_queue` passed. | **CLOSED** |
| **F-06** | Low | Shutdown with in-flight leg was untested. | Added `DeleteLegGuard` tracking and process test `tests/m3_shutdown_leg_tests.rs`. | `f06_graceful_stop_with_a_delete_leg_in_flight_...` passed in 3.08s. | **CLOSED** |
| **F-07** | Medium | No multi-process single-writer guard on log file. | File lock (`.lock`) acquired on open; second open is refused. | `f07_second_open_...` and `f07_successor_waits_...` passed. | **CLOSED** |
| **F-08** | Low | Mid-file length corruption misclassified as torn tail. | Prefix length validation distinguishes true prefix from corruption. | `f08_corrupt_mid_file_length_...` passed. | **CLOSED** |
| **F-11** | Medium | Degraded/not-durable states logged to stderr only. | Accepted with documented limitation; metrics integration deferred. | Logged to stderr; documented in operational guides. | **ACCEPTED** |
| **F-13** | Low | Compaction retry spam on failure. | Exponential backoff and rate-limited error logging. | `f13_failing_compaction_is_backed_off_and_recovers` passed. | **CLOSED** |

---

## 3. Empirical Test Suite Summary (`D:\Temp\sbx_m3fix`)

1. **Multi-Process Crash-Point Matrix (`m3_crash_point_tests.rs`):**
   - **15 passed, 0 failed** in 47.53s.
   - Tested real gateway clusters across loopback with fault proxies.
   - All crash points (K01 through K21b) executed cleanly: proved that unorderly process kill (`std::process::exit(137)` or `TerminateProcess`) after 204 preserves debt in write-ahead log, and restarted node replays debt and restores peer consistency.
2. **Multi-Process Durability Suite (`m3_invalidation_durability_tests.rs`):**
   - **5 functional tests passed, 0 failed, 1 exploratory ignored.**
   - Negative control verified peer convergence (221ms).
   - Baseline defect reproduced without log (92 stale polls across 10s window).
   - Durable log verified: victim peer B converged in 1.81ms after issuer kill -9 and restart.
   - Graceful stop verified: drain delivered invalidations to peer B before exit in 446ms.
   - Graceful deadline timeout verified: preserved debt on disk across graceful exit, converged in 1.76ms on restart.
3. **Shutdown Leg In-Flight Test (`m3_shutdown_leg_tests.rs`):**
   - **1 passed, 0 failed** in 3.08s.
4. **Router & Broadcaster Queue Tests (`m3_router_tests.rs`):**
   - **2 passed, 0 failed** in 0.43s.
5. **Durable Log Unit Tests (`src/dist/durable_log.rs`):**
   - **29 passed, 0 failed** in 3.70s.
6. **Research Freeze Baseline Test:**
   - `test_real_v2_2_freeze_record_verifies_when_present`: **Passed in 0.21s.**
7. **Linter & Code Quality:**
   - `cargo clippy --all-targets -- -D warnings`: **0 warnings (Exit code 0)** on both default and `--cfg m3_failpoints`.

---

## 4. Documented Limitations & Operational Constraints

1. **Fail-Open Policy on Timeout (F-03):** If disk stalls or fsync exceeds `PHANTOM_INVALIDATION_LOG_WAIT_MS` (default 2000ms), the DELETE operation returns HTTP 204 without waiting further (fail-open) and logs a warning to stderr. This prevents cascading stalls on gateway frontends but trades durability for availability during severe disk stalls.
2. **Metrics Observability (F-11):** Durability degradation counters are currently emitted to stderr logs and `LogStats`; they are not yet exposed as Prometheus/JSON gauges on `/api/v1/metrics`.
3. **Single-Writer Constraint (F-07):** The file lock guards against multiple processes sharing the same log file. Each cluster node must configure a distinct path using `{id}` interpolation (e.g. `PHANTOM_INVALIDATION_LOG_PATH=/data/inval_{id}.log`).
4. **OS Scope:** Durability was verified on Windows 11. Linux 0600 file modes are implemented defensively but were not executed on a native Linux kernel in this test round.

---

## 5. Final Recommendation

**The DEC-027 M3 Remediation v2 patch set is technically sound, empirically proven, clippy-clean, and completely preserves the research freeze.**

**Recommendation:** **APPROVED FOR MERGE**, strictly gated on explicit owner authorization before touching the main working tree.

