# Independent review proposals (NEVER APPLIED, UNTESTED sketches)

Status: proposal. Audience: M3 implementer / owner. Written by the independent reviewer (AI review, not human review).
These are sketches against the sandbox tree after `m3_remediation_combined.diff`; none was compiled.

## P-A. Bound the wait for durability (finding F-03)
In `src/dist/durable_log.rs::append_intent`, replace `match orx.await {` with a bounded wait:

    match tokio::time::timeout(Duration::from_secs(2), orx).await {
        Ok(Ok(Ok(stamp))) => Ok(stamp),
        _ => { self.stats.not_durable.fetch_add(1, Ordering::Relaxed); Err(LogError) }
    }

(2 s is an example; choose it from the DELETE SLO. Trade-off: a stuck disk then turns into fail-open instead of an unbounded DELETE.)

## P-B. Count tasks in the broadcaster main queue in `pending_jobs` (finding F-05)
In `ReplicaBroadcaster::enqueue`, `pending_jobs.fetch_add(1)` before `try_send` (undo on failure), and in the dispatcher loop
`pending_jobs.fetch_sub(1)` after the task's per-peer jobs were handed over (and in the `rx.close()` drain). Add a unit test
that calls `drain` immediately after `enqueue` with a stalled dispatcher.

## P-C. Avoid editing the two freeze-pinned files (finding F-01)
1. Declare the module inside the unpinned `src/dist/router.rs` (`pub mod durable_log;` -> file `src/dist/router/durable_log.rs`)
   instead of `src/lib.rs`; `main.rs` is not pinned. Callers become `crate::dist::router::durable_log::...`.
2. Test registration without `Cargo.toml`: place the test files at `tests/m3_invalidation_durability_tests.rs` and
   `tests/m3_crash_point_tests.rs` (Cargo auto-discovers top-level `tests/*.rs`; `autotests` is not disabled) and use
   `#[path = "support/mod.rs"]` style includes.
3. Failpoints without a cargo feature: `#[cfg(m3_failpoints)]` with `RUSTFLAGS="--cfg m3_failpoints"` (separate target dir) plus
   `#[allow(unexpected_cfgs)]` on the two items. Cost: the crash suite needs a dedicated full build.
Result: both pinned hashes unchanged, so `test_real_v2_2_freeze_record_verifies_when_present` keeps passing with no addendum.
Not compiled; options 1-3 are independent (1 alone removes `src/lib.rs`; 2+3 remove `Cargo.toml`).

## P-D. Single-writer guard for the log (finding F-07)
Windows: open the log with `OpenOptionsExt::share_mode(FILE_SHARE_READ)` for the append handle is NOT enough while compaction renames; use
a separate `<path>.lock` file opened with `share_mode(0)` and held for the process lifetime. Unix needs `flock` (libc is not a direct dependency: owner decision).
