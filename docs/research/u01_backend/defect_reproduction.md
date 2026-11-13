# U-01 defect reproduction (AI work)

Defect (lead verified): `src/lab_live/scenario_b.rs:658` (unmodified) `mech_pass = !rs.is_empty() && violating.is_empty()`; `src/lab_live/analysis.rs:131-134` `evaluate_arm` returns `MECHANISM_FAILED` ("H-B1 falsified") when `!mechanism_pass`. An arm with no runs (cancelled before it ran) therefore gets a completed mechanism verdict. Separately the planned `label` (config-only, `label_for`) is copied unchanged into cancelled summaries (`scenario_b.rs:1261-1262` and the `"label": meta.label` line, both before the patch).

Reproduction: pure call of the public `summarize()` on the UNMODIFIED sandbox (copy of the main tree; `scenario_b.rs` sha256 8f40517c7d49f3fb7053a3a9201e64c9fc525a3feb78c415196e5b7d42c16c3b, identical to the main tree at copy time), via new tests in `tests/integration/lab_live_scenario_b_tests.rs` (`mod u01_incomplete`). Reproduced outputs (from D:\Temp\sbx_u01_before.txt, excerpt in test_evidence.md):

1. Cancelled, zero runs, arms BASE/STATIC-CAP/AIMD-CAP: `verdicts.STATIC-CAP.outcome = "MECHANISM_FAILED"`, reason "origin concurrency exceeded the cap in force in at least one run (H-B1 falsified)", `evidence.n_valid_pairs = 0`, `mechanism_pass=false`. No run ever executed. Same for AIMD-CAP.
2. Cancelled after one repetition (BASE + STATIC-CAP, reps=10): `verdicts.STATIC-CAP.outcome = "INCONCLUSIVE"` ("only 1 valid paired runs; 10 required"): a completed-looking verdict on a partial design.
3. Cancelled with an observed over-cap run: `MECHANISM_FAILED` presented as a verdict.
4. Not cancelled but runs missing: verdict object emitted.
5. Label: tests pass a planned `VALID`; old code keeps it (`"label": meta.label`). The `label != VALID` assertion in `assert_incomplete` comes after the `verdicts` assertion, so the label defect is shown by code reading, not by an isolated failing line.

Before the fix: 1 passed (completed-run real-violation guard), 5 failed. Full output in test_evidence.md.

Scope note: a hard-failed run (`run_experiment` returns `Err`) and a killed process write no `summary.json`, so no verdict can be mis-assigned there; the real route to the defect is cancel (plus the defensive `runs_missing` case).
