# Reproduction guide

How to rebuild the release bundle, rerun the tests and rerun the benchmarks. Everything here is a procedure, not a result. Results from runs are recorded in `docs/release/` records or the completion report with the conditions attached.

## 1. Prerequisites

- Rust toolchain (reference: rustc 1.98.0 / cargo 1.98.0, Windows 11 x86-64). Record yours with `rustc -Vv`.
- POSIX: `sh`, `tar`, `sha256sum` or `shasum`, `curl` (smoke test). Windows: PowerShell 5.1+.
- Free disk space on the drive that holds `TEMP` (the MSVC linker writes there). If the system drive is full, set `TMP`/`TEMP` to another drive; builds fail with link error 1104/1108 or "No space left on device" otherwise.
- Network access to crates.io unless the cargo registry cache already holds every locked crate.

## 2. Build a release bundle

```
# POSIX
packaging/build-release.sh
# Windows PowerShell
powershell -ExecutionPolicy Bypass -File packaging\build-release.ps1
```

Both run `cargo build --release --locked -j 1 --bin phantom-cache-demo`, stamp the binary with `PHANTOM_BUILD_COMMIT` (`git rev-parse HEAD`, suffixed `-dirty` when the tree has uncommitted changes) and `PHANTOM_BUILD_DATE` (UTC), and write `dist/phantom-cache-<version>-<target>/` plus an archive and `.sha256`. Use `CARGO_TARGET_DIR` for a fresh build directory and `PHANTOM_DIST_DIR` for the output location. The working tree at the time of M7 is uncommitted, so a bundle built from it is stamped `-dirty` and is not a release.

## 3. Reproducibility check

Build twice with two different `CARGO_TARGET_DIR` values, then compare `SHA256SUMS` entries for `phantom-cache-demo[.exe]`. Report the result honestly: identical, or different with an explanation (embedded build date differs by design: `PHANTOM_BUILD_DATE` changes between runs; export a fixed `PHANTOM_BUILD_DATE` before building to compare). Absolute paths embedded by rustc can also differ if the source directories differ. Bit-for-bit reproducibility across machines is not claimed.

## 4. Clean-room build and smoke test

`scripts/cleanroom_build.ps1 -WorkDir <new empty dir>` copies the tree (excluding `target`, `.git`, `node_modules`, logs) into the new directory, builds a bundle there with a fresh target directory, verifies `SHA256SUMS`, and runs `packaging/smoke-test.ps1`. This is a clean-room (fresh source copy and fresh target) test on the same machine. It is not a clean-machine test: the toolchain and the cargo registry cache are shared.

## 5. Re-run the tests

```
scripts/reproduce_tests.sh [log-dir]          # POSIX
scripts\reproduce_tests.ps1 [-LogDir dir]     # Windows
```

Equivalent to `cargo test --all-targets -j 1`. Note that `--all-targets` also executes the seven `harness = false` benchmark programs. The scripts write the log and an `environment.txt` (UTC date, rustc, HEAD, number of dirty files). Passing tests do not establish correctness or research validity (`CLAUDE.md` section 3).

Focused security and M7 suites:

```
cargo test -j 1 --test security_phase1_tests --test security_phase2_upstream_tests \
  --test security_phase3_cluster_tests --test security_phase3b_framing_tests \
  --test security_phase3c_pooling_tests --test security_phase4a_invalidation_tests \
  --test security_phase4b_router_tests --test security_phase4c_antientropy_tests \
  --test known_gaps --test m7_config_tests --test m7_security_review_tests --test m7_health_tests
```

## 6. Re-run benchmarks

Benchmarks are `harness = false` programs: `b1_hit_path`, `b2_concurrency`, `b3_coalesce`, `b4_eviction`, `b6_telemetry`, `b7_memory`, `b8e_gateway`. Run one with `cargo bench -j 1 --bench b1_hit_path`. Existing scripted procedures and previous logs are in `scripts/run_audit_suite.ps1` and `bench-results/`.

A number is a measurement only if it comes from a release build with the hardware, OS, workload, seed, baseline, repetitions and confidence intervals recorded, and with loopback, simulated, WSL2 and physical-host results kept separate. This guide provides no performance results, and M7 did not run a performance-regression gate (M7-05): that needs the frozen M6 reference and a fit platform.

## 7. Verify an installed bundle

See `docs/user/install.md`: checksum verification, `--version`, `--check-config`, and `packaging/smoke-test.*`.

## 8. Recorded runs (M7, 2026-10-29, Windows 11 x86-64, rustc 1.98.0)

These are records of what was run, not guarantees.

- **Clean-room build** (`scripts/cleanroom_build.ps1`, tree copied to `D:\Temp\m7_clean_src_run3`, fresh `CARGO_TARGET_DIR=D:\CargoTarget\m7_clean`, `--release --locked -j 1`): build succeeded (release profile, final compile step 1 m 45 s after dependencies; the dependency compile happened in earlier attempts of the same target dir), `SHA256SUMS` verified, `packaging/smoke-test.ps1` passed (`--version`, `--check-config` on the shipped example, exit 2 on an invalid boolean, `/health` OK, `/health/live` 200, `/health/ready` 200). Commit stamp was `unknown` because the copy has no `.git`. This was not a clean-machine test.
- The first two clean-room attempts found and led to fixes of three script bugs: git probe failing under `ErrorActionPreference=Stop` outside a repository, robocopy `/XD dist` also excluding `src/dist`, and stderr of the binary aborting the smoke test.
- Not done: building the bundle twice to compare binary hashes (reproducibility is unreported), upgrade N to N+1 and rollback, POSIX scripts (`*.sh` were not executed on any machine), Linux, signal-driven shutdown at process level, load or soak runs, performance-regression gate.
