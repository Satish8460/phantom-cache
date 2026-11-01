# Test plan: PHANTOM_DEFAULT_TTL_MS wiring

Status: proposed. Validated in the scratch copy only (results: `evidence/` and the end of this file).

## 1. New tests (fail before, pass after)

File `tests/integration/default_ttl_wiring_tests.rs`, registered as `[[test]] name = "default_ttl_wiring_tests"` in `Cargo.toml`.

| Test | Asserts | Fails before the patch because |
|---|---|---|
| `configured_default_ttl_reaches_the_freshness_engine_through_the_real_router` | Through `create_app` and an in-process origin: unconfigured default is 60 000 (`default_ttl_ms()`, `DEFAULT_TTL_MS_FALLBACK`); a header-less response is MISS then HIT (default 60 s); `set_default_ttl_ms(0)` is ignored; after `set_default_ttl_ms(2000)` a header-less entry is MISS, HIT, and after ~2.6 s is **not** a HIT and triggers an origin refetch; an origin `Cache-Control: max-age=60` entry on the same gateway stays a HIT with no extra origin fetch (origin lifetime wins over the default). | Before the patch the symbols `set_default_ttl_ms`/`default_ttl_ms` do not exist (compile failure). Behavioural check: with the setter present but the two `server.rs` call sites reverted to the literal `60_000` ("mutant", `evidence/04_mutant_*.log`) the entry is still a HIT after 2.6 s and the test fails on the expiry assertion. |
| `default_ttl_key_is_applied_by_main_and_range_validated` | `spec(..).wiring == Wiring::Main`; schema default 60000; accepted: 1, 5000, 31536000000 with no "not honored" warning; rejected with the key named in the error: 0, -1, abc, 1.5, 60s, 31536000001; empty counts as unset (documented layer rule). | `Wiring::NotWired` before; the "not honored" warning is emitted for 5000. |

Determinism notes: real-time sleep of 2.6 s against a 2 s TTL (0.6 s margin); the gateway reads its own wall clock, so the test cannot use a virtual clock. All servers bind `127.0.0.1:0`. The test is a single sequential scenario because the default is process-global; no other test in that binary touches it. Label: loopback, in-process, debug build; it is a correctness test, not a measurement.

## 2. Modified existing test

`tests/integration/m7_config_tests.rs`: `not_wired_default_ttl_is_reported_honestly` asserted the "not honored" warning; replaced by `default_ttl_is_wired_and_no_longer_reported_as_not_honored` (no warning, report has no "(not honored by runtime)", `Wiring::Main`). `defaults_resolve_and_match_schema` (default 60000) and `docs_and_code_list_the_same_keys` (docs row must contain `60000`) stay unchanged and must pass.

## 3. Ceiling (`PHANTOM_MAX_FRESHNESS_MS`): tests for the follow-up only (not in this patch)

Written so the follow-up cannot be merged without them: (a) origin `max-age=3600`, ceiling 2 s => entry expires after ~2 s on the fill path; (b) same on the 304 refresh path (origin 304 with `max-age=3600`, ceiling 2 s => refreshed entry still expires after ~2 s); (c) ceiling unset => today's behaviour byte-for-byte (`freshness_matrix_tests` unchanged); (d) `min(default_ttl, ceiling)`; (e) `stale_cap_violations == 0` after (a)-(b) with `stale-if-error` present; (f) fencing tests (`concurrency_verification_tests`, `m1_http_tests`) unchanged. (b) requires the `src/core/cache.rs` change and a new freeze addendum plus an independent review.

## 4. Regression suites that must pass after integration

Minimum set run in the scratch copy: `default_ttl_wiring_tests`, `m7_config_tests`, `m1_http_tests`, `serving_contract_tests`, `revalidation_304_tests`, `stale_serving_tests`, `m7_sec_fixes_tests`, `canonical_key_roundtrip_tests`, `telemetry_honesty_tests`.

Before merge in the main tree: `cargo test --all-targets -j 1` (baseline 242 tests / 19 targets is stale; the tree has more targets now, compare against a pre-patch run) and `cargo clippy --all-targets -j 1 -- -D warnings`; `git diff --check` for whitespace. Also `--check-config` smoke: `PHANTOM_DEFAULT_TTL_MS=5000 phantom-cache-demo --check-config` must no longer print the "not honored" warning.

## 5. Validation results (scratch copy `D:\Temp\ttl_scratch`, debug profile, loopback, `cargo -j 1`, CARGO_TARGET_DIR=`D:\Personal\_ttl_target`)

Scratch = working tree snapshot of 2026-10-30 (src, tests, benches, docs/user, Cargo.*) plus the patch; `ui/vite.config.js` copied because `m7_config_tests` reads it. The tree is being edited concurrently by other agents; the patch was re-based once and `git apply --check` passes against the tree at the end of this run (HEAD 456bb25, 71 status entries).

| Step | Result | Log |
|---|---|---|
| Build all test targets with the patch | exit 0, 10m04s (first, cold build; later re-synced and rebuilt incrementally) | `evidence/01_build_tests.log` (UTF-16) |
| `default_ttl_wiring_tests` (patched) | 2 passed, exit 0 | `evidence/03_default_ttl_wiring_tests.log` |
| Mutant: the two `server.rs` sites reverted to `60_000`, setter present | exit 101, `default TTL must have expired the header-less entry` (behavioural failing-before) | `evidence/04_mutant_default_ttl_wiring_tests.log` |
| Pre-patch tree | the new test does not compile (symbols absent); not run separately | n/a |
| `m7_config_tests` | 24 passed, exit 0 (after re-syncing the test file; an earlier run failed only because the scratch test file was stale relative to the tree's new `PHANTOM_LAB_*` keys, unrelated to this patch) | `evidence/03_m7_config_tests.log` |
| `m1_http_tests` | 11 passed | `evidence/03_m1_http_tests.log` |
| `serving_contract_tests` | 32 passed | `evidence/03_serving_contract_tests.log` |
| `revalidation_304_tests` | 22 passed | `evidence/03_revalidation_304_tests.log` |
| `stale_serving_tests` | 70 passed, 1 ignored (pre-existing) | `evidence/03_stale_serving_tests.log` |
| `m7_sec_fixes_tests` | 10 passed | `evidence/03_m7_sec_fixes_tests.log` |
| `canonical_key_roundtrip_tests` | 10 passed | `evidence/03_canonical_key_roundtrip_tests.log` |
| `telemetry_honesty_tests` | 15 passed | `evidence/03_telemetry_honesty_tests.log` |
| `evidence/02_summary.txt` | per-target exit codes of the first sweep (m7_config_tests=101 there, superseded by the rerun above) | |

Not run: full `cargo test --all-targets`, clippy `-D warnings`, `--check-config` smoke, release profile. These remain required before integration.
