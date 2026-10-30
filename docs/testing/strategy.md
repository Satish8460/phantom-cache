# Test strategy

**Status:** Active
**Audience:** contributors, reviewers, and anyone deciding how much a green test run proves
**Last reviewed:** 2026-10-30
**Authority and related:** the baseline row (count, profile, date, command, log) is owned by [../STATUS.md](../STATUS.md); this page records the method and the history it was built from and must not be used as the status source. Known weak tests: [../research/code_findings.md](../research/code_findings.md). Run procedures: [../contributing.md](../contributing.md), [../release/reproduction_guide.md](../release/reproduction_guide.md). Older documents in this folder (`ENVIRONMENT_READINESS_REPORT.md`, `FLAGSHIP_VALIDATION_REPORT.md`, `V2_RUNTIME_AUDIT.md`, `V3_FLAGSHIP_REALISM_AUDIT.md`, `V4_FLAGSHIP_VISUALIZATION_AUDIT.md`) are earlier audits of the flagship demo, not this strategy.

## 1. How the numbers on this page were obtained

- Target list: read from `[[test]]` entries in [../../Cargo.toml](../../Cargo.toml) and cross-checked against the files on disk (2026-10-30): 72 `[[test]]` entries (4 under `tests/unit/`, 68 under `tests/integration/`), every entry's path exists, and every `.rs` file directly under those two directories is registered. `tests/integration/lab_api/` is compiled as a module of `lab_live_scenario_b_tests` (`#[path = "lab_api/mod.rs"]`), not as separate targets. `tests/support/` is shared helper code.
- Other targets: the library, the `phantom-cache-demo` binary and the `phantom-lab` binary. 72 + 3 = 75, which matches the "75 targets" reported by the sweeps below. That match is arithmetic, not an observation of the Cargo output (cargo was not run for this page).
- Seven bench mains are declared as `[[bench]]` (`harness = false`) in `Cargo.toml` and live in `benches/`; they are excluded from `cargo test --tests`.
- Static `#[test]` / `#[tokio::test]` count (grep over `tests/` and `src/`, 2026-10-30): 840 attributes, 8 `#[ignore]`. This does not reconcile with the 851 passed of the recorded run (property tests, macro-generated cases and doc tests are not counted by a grep). Use the recorded run, not this grep, for any count.

## 2. Recorded baseline (reported, not reproduced by this page)

| Run | Result | Source |
|---|---|---|
| `cargo test --tests -j 1 --no-fail-fast`, debug profile, 2026-10-30 10:11 to 10:23:53 (+05:30), exit 0 | 851 passed, 0 failed, 8 ignored, 75 targets, 730 s, orphan processes 0 | ledger entry of 2026-10-30 10:24 in [../research/progress_ledger.md](../research/progress_ledger.md) (lead-run; log kept on a `D:\Temp` path, which may not be retained) |
| `node --test ui/test` | 212 tests, 212 pass | same ledger entry (lead-run) |
| `cargo clippy --all-targets -j 1 -- -D warnings` | exit 0 (34 s) | same ledger entry |

Earlier counts (242 in [../../CLAUDE.md](../../CLAUDE.md) and the root README, 774 in the accountability report section 4, 841 in [../research/g2_verdict.md](../research/g2_verdict.md)) are older points in time with different commands or trees. They are not errors in themselves; they are not the current baseline. [../STATUS.md](../STATUS.md) owns the current row; if it differs from the table above, STATUS.md wins.

## 3. Layers: what exists, what it proves, what it does not

| Layer | Targets (examples) | Proves | Does not prove |
|---|---|---|---|
| Unit (library and `tests/unit/`) | `wp1_tests` to `wp4_tests`, in-crate tests in `src/dist/frame.rs`, `src/dist/session.rs`, `src/intelligence/mod.rs` | local behavior of clocks, PRNG, cache shards, codecs | system behavior; several early tests are course-work checks ([../PHASE_2_WORK_REPORT.md](../PHASE_2_WORK_REPORT.md)) |
| Serving-path contract | `serving_contract_tests`, `freshness_matrix_tests`, `stale_serving_tests`, `revalidation_304_tests`, `invalidation_contract_tests`, `byte_accounting_tests`, `eviction_contract_tests`, `epoch_contention_tests`, `concurrency_verification_tests`, `canonical_key_roundtrip_tests` (property tests), `parser_robustness_tests`, `origin_cap_tests`, `m1_http_tests`, `coalesce_tradeoffs_tests` | the listed contract clauses ([../research/adr/ADR-002-serving-path-contract.md](../research/adr/ADR-002-serving-path-contract.md)) hold on the tested inputs, in-process or over loopback, debug build | linearizability, bounded staleness, behavior under production load, release-profile timing |
| Telemetry | `telemetry_truth_tests`, `telemetry_redesign_tests`, `telemetry_honesty_tests`, `latency_resolution_tests`, `schema_deprecation_tests` | counters and histograms report what the code recorded; labels are honest in the tested paths | that the numbers describe real workloads |
| Distributed / cluster | `security_phase3*`, `security_phase4*`, `p2_dist_network_tests`, `distributed_consistency_tests`, `cluster_*_tests`, `protocol_model_tests`, `known_gaps` | invalidation fan-out, repair, pre-warm and framing work on loopback with up to 5 gateway processes and a fault proxy | multi-host behavior, loss and delay studies, durability across a crash of the issuing gateway (a documented gap: [../user/limitations.md](../user/limitations.md)) |
| Security | `security_phase1_tests`, `security_phase2_upstream_tests`, `m7_security_review_tests`, `m7_sec_fixes_tests`, `m7_sec_round2_tests`, `m7_sec_round3_tests`, `m7_config_tests`, `m7_health_tests` | the fixed findings stay fixed; configuration validation rejects the tested bad inputs | absence of vulnerabilities: no penetration test, fuzzing, or dependency advisory scan was done ([../security/m7_security_review.md](../security/m7_security_review.md)) |
| Lab and experiment machinery | `lab_*_tests`, `m6_research_evaluation_tests`, `lab_live_scenario_b_tests` (includes `tests/integration/lab_api/`), `lab_runner_lib_tests` | the simulator, runner, analysis code and Lab API behave as specified; the M6 freeze-integrity test fails closed if frozen files change | that any experimental conclusion is true; T1 results are simulations, T2 results are single-host loopback |
| Intelligence | `intel_*_tests`, `p4_pie_tests`, `idle_sweeper_tests` | the `src/intelligence/` modules behave as coded | that adaptive control helps: no benefit is established, and the machinery is not integrated into the serving path (claim C8 in [../research/claims_ledger.md](../research/claims_ledger.md)) |
| Memory soak | `memory_bound_soak_tests` | bounded memory in the configured run (duration and parameters not checked for this page) | a long (for example 2 h) soak; none recorded on the current tree |
| UI node tests | `ui/test/*.test.mjs` (`node --test ui/test`), `npm run check` in `ui/` | view-model logic, provenance labeling, forbidden-token scan, contrast and accessibility lint on static inputs | rendering in a browser, real-user accessibility |
| Browser scripts | `ui/scripts/browser-*.mjs`, run by hand | what the retained rounds in [../demo/browser_verification/README.md](../demo/browser_verification/README.md) observed in headless Chrome | assistive-technology behavior; other browsers |
| Benches | `benches/b1` to `b8e` | nothing yet on the current tree | all performance claims: the benches were not re-run on the current tree |

## 4. Tests that assert stub or defect behavior

A passing result there is not evidence of a feature. Examples recorded in [../research/code_findings.md](../research/code_findings.md): `p4_pie_tests` asserts the legacy PIE stub behavior (finding F3); `known_gaps` contains pin tests that pass because a known defect still exists (for example KG-6, TG-1). When such a defect is fixed the pin test is expected to change. Do not count them toward feature coverage.

## 5. Flaky and timing-sensitive tests

Listed in [../research/phantomcache_milestone_accountability_report.md](../research/phantomcache_milestone_accountability_report.md) section 4: cluster tests with wall-clock windows (the `m9` ratio test, the `l3` read-timeout test, 700 ms windows), `WARM_MEDIAN_BOUND`, and harness port races (a retry was added). The G2 verdict records one flake that did not recur, `test_m5_pool_wait_within_cap_then_exchange_succeeds` ([../research/g2_verdict.md](../research/g2_verdict.md)). Residual risk under heavy machine load remains. A failure in one of these is not proof of a regression, and a single pass is not proof of stability.

Ignored tests (8 in the recorded run) are never run by the sweep: M6 tuning, calibration and sealed evaluation (the tuning and sealed runs were executed explicitly in release), an M3 demo test, an N=1000 latency test, a Scenario B heavy test, and others listed per run in the ledger. Check the ledger before treating "0 failed" as "everything ran".

## 6. Known gaps (as of 2026-10-30)

- No release-profile run of the general suite (release was used only for M6 tuning, the dry run, the sealed run and Scenario B).
- No soak test on the current tree; benches not re-run.
- `cargo fmt --check` has never been run in the project's records, although CI runs it.
- No assistive-technology pass; the browser rounds are automated observations in one browser (Chrome 156 headless), not an accessibility audit.
- Only Windows x86-64 has been exercised; no multi-host (T3) test exists.
- Evidence logs on `D:\Temp` paths are ephemeral; the 2026-10-30 storage cleanup did not touch `D:\Temp`, but retention is not guaranteed ([../research/storage_cleanup_2026-10-30.md](../research/storage_cleanup_2026-10-30.md)).
- The second independent check of the G2 fix round (disposition G2-D1) has no recorded outcome in `docs/research/` (unverified whether it completed).

## 7. Rules

1. A change to fencing, epochs or invalidation needs a test that fails without the change and an independent review ([../../CLAUDE.md](../../CLAUDE.md) section 5).
2. Quote a count only as: number, command, profile, date, tree state. The baseline row lives in [../STATUS.md](../STATUS.md).
3. Tests are evidence of tested behavior. Statistical or performance claims need the standard in [../../CLAUDE.md](../../CLAUDE.md) section 4 (release build, repetitions, intervals, limitations).
4. Keep T1 (simulated), T2 (loopback) and any physical-host results separate ([../glossary.md](../glossary.md)).
