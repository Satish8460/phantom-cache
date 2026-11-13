# U-01 backend: brief for a separate reviewer

The author is an AI (this is AI work, not independent verification). Patch: `docs/research/u01_backend/u01_backend.patch` (unified diff vs the main tree for `src/lab_live/scenario_b.rs` and `tests/integration/lab_live_scenario_b_tests.rs`; sandbox files in D:\Temp\sbx_u01). Contract: contract.md. Nothing is applied to the main tree.

Check, with line numbers in the SANDBOX files (`D:\Temp\sbx_u01\...`):

1. `src/lab_live/scenario_b.rs:585-593` `incomplete_reason`: cancelled OR any arm with fewer runs than `cfg.reps`. Is counting `RunMetrics` per arm right (invalid runs still count as runs; `valid` is separate)? Could a legitimately complete experiment have fewer runs than `reps` (e.g. duplicate arms in `cfg.arms`)?
2. `:596-605` `effective_label`: incomplete forces `EXPLORATORY` and appends a reason; the complete path returns the planned values unchanged. Check that the added `label_planned` key and the reason string do not break `label_reasons` consumers.
3. `:684-691` mechanism `pass` is `null` when incomplete; `runs`, `violating_reps`, `max_origin_concurrent_observed_whole_run`, `cap_limit_in_force` unchanged. The old zero-run expression (`!rs.is_empty() && ...`, `:684`) now only matters for complete summaries, where zero runs is unreachable.
4. `:727-733` `evaluate_arm` is skipped when incomplete; `src/lab_live/analysis.rs` is NOT modified. `paired_vs_base` is still computed and emitted for incomplete runs (raw diagnostics). Decide whether partial paired CIs should also be withheld.
5. `:736-745`, `:771`, `:773` `verdicts` and `mechanism_h_b1_all_pass` are `null` iff incomplete; `verdicts_status` `computed`/`not_computed`; `run_completeness` object.
6. `:1307` `run_experiment` returns `ExperimentOutcome.label` as the effective label (feeds the Lab API result event, `src/api/lab_api.rs:1864`). `write_manifest` keeps the PLANNED label under the already-named `label_planned` key (unchanged).
7. Not changed, needs a decision: `src/api/lab_api.rs:1384-1386` and `:1855-1857` (`as_object().unwrap_or_default()` turns null verdicts into `{}` and drops `verdicts_status`); docs `docs/demo/scenario_b_artifacts.md`, `docs/user/lab_api.md`; `summary_version`.
8. Tests `tests/integration/lab_live_scenario_b_tests.rs:450-595` (`mod u01_incomplete`): re-run against a clean copy to confirm the failures are for the stated reason; confirm `run_of` builds genuine `RunMetrics` via `compute_run_metrics`; confirm the completed-run tests assert pre-existing outcomes (`INCONCLUSIVE` with 1 pair, `MECHANISM_FAILED` on a real violation). Only `completed_run_with_real_violation_is_still_mechanism_failed` is a pure unchanged-behaviour guard; `completed_run_keeps_verdict_semantics` also asserts the new additive keys.
9. CLAUDE.md: lab tooling, not fencing/epochs/invalidation. No Cargo.toml change, no new dependency, no new `[[test]]` stanza (tests added to the already registered target `lab_live_scenario_b_tests`).
10. Legacy artifacts (old shape) are untouched; confirm the UI correction handles both shapes (contract.md, "UI rules").
