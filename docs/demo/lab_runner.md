# Phantom Lab runner (library + `phantom-lab` binary)

Status: D-3, owner decision O4-1. Tier of every result: **SIMULATED**.

## What it is

The T1 experiment implementation (formerly test-only, `tests/support/lab`) now lives in the library crate as
`phantom_cache_demo::lab` (`src/lab/`). The test suite, the `phantom-lab` binary and any later HTTP lab API call the
same code (`lab::eval::run_matrix_with_progress`). `tests/support/lab/<module>.rs` are thin `pub use` re-exports of it.

The runner drives the REAL product components in-process (cache with byte accounting, coalescing registry with
generation fencing, `DefaultPolicyExecutor`, the intelligence candidates) on a deterministic virtual-time clock
against a SIMULATED origin (seeded service times, outage/brownout/latency degradation). No network, no wall clock.

Stayed under `tests/support/lab` (need test-only crates, e.g. `reqwest`): `gateway`, `baselines`, `loadgen`,
`origin_proc` (T2 real-process loopback helpers). They are not part of the runner.

## CLI

```
cargo run -j 1 --bin phantom-lab -- list all
cargo run -j 1 --bin phantom-lab -- run \
  --scenarios O-DEG,O-BRN --families W-DI,W-Z --arms BL-1,BL-12b,PIE-Retained \
  --seed-set D --n-seeds 5 --profile smoke --out lab-results/my_run --raw \
  --control BL-1 --treatment PIE-Retained
```

Options: `--seed-set T|D`, `--n-seeds N`, `--profile smoke|standard`, `--arm-params untuned|frozen`, `--raw`,
`--name`, `--overwrite`, `--control`, `--treatment`. Exit codes: 0 ok, 1 run failure, 2 usage/refused request.
Use `--release` builds for the `standard` profile (debug is slow).

Outputs in `--out`: `cells.csv` (one row per cell, raw), `manifest.json`, and with `--raw` `requests.csv` and `fetches.csv`.
The console summary (per-arm means, paired-by-seed relative reduction with 95% BCa CI and sign-flip permutation p)
is computed from the in-memory cells, which are identical to `cells.csv`. It is descriptive, not a preregistered test;
with few seeds the permutation p-value cannot be small (n=3: minimum 0.25).

## Library API (`lab::runner`)

- `RunSpec` (serde `Serialize`/`Deserialize`; defaults: `seed_set="D"`, `profile="smoke"`, `arm_params="untuned"`).
- `validate(&RunSpec) -> Result<Vec<u64>, LabError>`, `cell_count(&RunSpec)`.
- `execute(&RunSpec, &mut dyn FnMut(done, total)) -> Result<RunResult, LabError>`; `RunResult { spec, cells, manifest, out_dir }`.
- `paired_summary(&RunResult, treat, ctrl) -> Vec<EndpointResult>`, `read_cells(dir)`.
- `list_scenarios/arms/families/seed_sets()`.
- `LabError`: `SealedSeedSetRefused`, `SealedFamilyRefused`, `Unknown*`, `EmptySelection`, `InvalidSeedCount`, `OutputExists`, `Io`.

## Determinism

The same `RunSpec` and source give byte-identical `cells.csv`/`requests.csv`/`fetches.csv` (virtual time, seeded RNG streams,
no wall clock in the simulation). Only `manifest.json` timestamps (and source hashes) differ. Verified: a fixed
3-seed dev matrix (2 scenarios x 2 families x 5 arms, raw on) produced the same SHA-256 before the move (legacy test-only
code), after the move (via the test re-exports) and via the binary (cells.csv `9f4b48ce...61e4a`).
Tests: `tests/integration/lab_runner_lib_tests.rs`.

## Manifest and provenance

`manifest.json`: manifest_version, name, `tier: SIMULATED`, `provenance` block (simulated events vs real in-process
components; `real_process_measurements: none`; latencies are virtual microseconds), seed set, seed labels and numeric seeds
(T/D only), seed-list hash, scenarios/families/arms, per-arm configs, arm/sim params, git HEAD and dirty count,
`source_sha256` (incl. `src/lab`, `src/core`, `src/intelligence`), build profile, host OS/arch, real UTC `started_utc`/`ended_utc`,
output file hashes, the `run_spec`. Never mix these numbers with T2 loopback or physical-host measurements.

## Guards and limits

- The sealed seed set S is refused (also spelled `sealed`) before any output is created, and the family `W-SF` (sealed family)
  is refused. Seed values of S are never derived, printed or written by this tool. Sealed evaluation remains the separate gated
  path in `m6_research_evaluation_tests.rs`; it was not run.
- Only seed sets T and D; W-TR (trace) is unavailable.
- The protocol freeze record hashes `tests/support/lab/**`; moving the code invalidates it until amended by the lead.
- Debug-build runs are for mechanics; they are not performance evidence.
- Not covered: HTTP endpoints, cancellation, T2 real-process runs.
