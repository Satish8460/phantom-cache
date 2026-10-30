# CI recommendations (M7)

`.github/workflows/ci.yml` was not edited. It exists (push/PR to `main`, ubuntu-latest) with `rust-ci` (fmt check, `cargo check --all-targets --all-features`, `cargo clippy ... -D warnings`, `cargo test --all-targets --all-features`) and `frontend-ci` (`npm ci`, `npm run build`). The statement "no CI exists" in `docs/research/phase_7_execution_package.md` and `phase_implementation_file_map.md` is wrong. Changes below are proposals for the owner; adding gates is an owner-reserved change.

## Findings about the current workflow

1. **`cargo test --all-targets` runs seven `harness = false` bench mains** (`b1_hit_path`, `b2_concurrency`, `b3_coalesce`, `b4_eviction`, `b6_telemetry`, `b7_memory`, `b8e_gateway`; `Cargo.toml` `[[bench]]` stanzas). They execute as ordinary programs during `cargo test`, which adds time, and a hosted runner makes any timing they print meaningless as a measurement. Prefer `cargo test --tests` (lib, bin and integration targets) for correctness, and run benches only in a dedicated, labeled job.
2. **Formatting status is unknown.** CI runs `cargo fmt --all -- --check`, but agents are forbidden to run `cargo fmt` and the working tree is large and uncommitted; no run of the check on the current tree was recorded. It may fail on first run. The owner should run `cargo fmt --all -- --check` once and decide whether to format in a dedicated commit.
3. **Clippy status on the current tree is not recorded** (`CLAUDE.md` states it was not re-run). Run `cargo clippy --all-targets -j 1 -- -D warnings` locally before enabling it as a required check. The M7 modules (`config.rs`, `health.rs`) were not clippy-checked by this pass unless the reproduction record says so.
4. **`--all-features`** is passed although `Cargo.toml` has no `[features]`; harmless but misleading.
5. **No `-j 1`.** The hosted runner does not need it; the reference Windows machine does.
6. **The untracked tree cannot build from HEAD** (`tests/integration/`, `src/security/`, several modules are untracked). CI on a pull request will only be meaningful after the owner commits these files.
7. **No `--locked`.** Add `--locked` to every cargo invocation so the lockfile is authoritative (the release scripts use it).
8. **No Windows job.** The only machine the project has exercised is Windows x86-64; add a `windows-latest` job for build and test.
9. **No release job, checksum or artifact upload.** Do not auto-publish; if desired add a manual (`workflow_dispatch`) job that runs `packaging/build-release.sh` and uploads the bundle and `.sha256` as an artifact.
10. **No dependency advisory scan** (`cargo audit` / `cargo deny`); installing them is an owner decision (O7-6).
11. **Frontend job** builds only; add `npm test` if `ui/test` provides a runnable suite (not verified here). Pin Node version per `ui/package.json` engines if present.

## Suggested job layout

```yaml
jobs:
  rust-test:
    strategy: { matrix: { os: [ubuntu-latest, windows-latest] } }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable   # or pin the toolchain used for releases
      - uses: swatinem/rust-cache@v2
      - run: cargo check --locked --all-targets
      - run: cargo test --locked --tests
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - run: cargo fmt --all -- --check          # only after the owner resolves formatting
      - run: cargo clippy --locked --all-targets -- -D warnings
  bench-smoke:                                    # manual, never a performance claim
    if: github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --locked --benches
```

Pin third-party actions by commit SHA if supply-chain policy requires it.
