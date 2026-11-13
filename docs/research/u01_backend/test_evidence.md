# U-01 backend: test evidence (AI-run; sandbox D:\Temp\sbx_u01; target D:\Personal\_sbx_u01_target)

Machine clock 2026-10-30, about 15:50-15:57 IST. Debug profile, mechanics only; no performance claims. Cargo always `-j 1` through with_cargo_lock.ps1.

## BEFORE (unmodified sandbox src, new tests only)

Command: `cargo test -j 1 --test lab_live_scenario_b_tests u01_` . Result: 1 passed; 5 failed (expected). Raw: D:\Temp\sbx_u01_before.txt. Excerpt:
```
[lock acquired by U01] cd D:\Temp\sbx_u01; cargo test -j 1 --test lab_live_scenario_b_tests u01_ 2>&1
    Finished `test` profile [unoptimized + debuginfo] target(s) in 42.79s
     Running tests\integration\lab_live_scenario_b_tests.rs (D:\Personal\_sbx_u01_target\debug\deps\lab_live_scenario_b_tests-34e043344d19b0ef.exe)

running 6 tests
test u01_incomplete::completed_run_with_real_violation_is_still_mechanism_failed ... ok
test u01_incomplete::completed_run_keeps_verdict_semantics ... FAILED
test u01_incomplete::cancelled_with_zero_runs_gets_no_verdict_and_no_h_b1_falsified ... FAILED
test u01_incomplete::cancelled_partial_keeps_raw_diagnostics_but_withholds_verdict ... FAILED
test u01_incomplete::observed_violation_in_a_cancelled_run_is_kept_as_observation_only ... FAILED
test u01_incomplete::not_cancelled_but_missing_runs_is_also_incomplete ... FAILED

failures:

---- u01_incomplete::completed_run_keeps_verdict_semantics stdout ----

thread 'u01_incomplete::completed_run_keeps_verdict_semantics' (13904) panicked at tests\integration\lab_live_scenario_b_tests.rs:573:9:
assertion `left == right` failed
  left: Null
 right: "computed"

---- u01_incomplete::cancelled_with_zero_runs_gets_no_verdict_and_no_h_b1_falsified stdout ----

thread 'u01_incomplete::cancelled_with_zero_runs_gets_no_verdict_and_no_h_b1_falsified' (17124) panicked at tests\integration\lab_live_scenario_b_tests.rs:507:9:
verdicts must be null, got {"AIMD-CAP":{"evidence":{"contention_flagged":false,"mechanism_pass":false,"n_valid_pairs":0,"origin_p99_diff_mean":null,"rel_ci_hi":null,"rel_ci_lo":null,"rel_reduction_mean":null,"success_diff_pp_mean":null},"outcome":"MECHANISM_FAILED","reasons":["origin concurrency exceeded the cap in force in at least one run (H-B1 falsified)"]},"STATIC-CAP":{"evidence":{"contention_flagged":false,"mechanism_pass":false,"n_valid_pairs":0,"origin_p99_diff_mean":null,"rel_ci_hi":null,"rel_ci_lo":null,"rel_reduction_mean":null,"success_diff_pp_mean":null},"outcome":"MECHANISM_FAILE
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- u01_incomplete::cancelled_partial_keeps_raw_diagnostics_but_withholds_verdict stdout ----

thread 'u01_incomplete::cancelled_partial_keeps_raw_diagnostics_but_withholds_verdict' (12284) panicked at tests\integration\lab_live_scenario_b_tests.rs:507:9:
verdicts must be null, got {"STATIC-CAP":{"evidence":{"contention_flagged":false,"mechanism_pass":true,"n_valid_pairs":1,"origin_p99_diff_mean":0.0,"rel_ci_hi":0.0,"rel_ci_lo":0.0,"rel_reduction_mean":0.0,"success_diff_pp_mean":0.0},"outcome":"INCONCLUSIVE","reasons":["only 1 valid paired runs; 10 required"]}}

---- u01_incomplete::observed_violation_in_a_cancelled_run_is_kept_as_observation_only stdout ----

thread 'u01_incomplete::observed_violation_in_a_cancelled_run_is_kept_as_observation_only' (31684) panicked at tests\integration\lab_live_scenario_b_tests.rs:507:9:
verdicts must be null, got {"STATIC-CAP":{"evidence":{"contention_flagged":false,"mechanism_pass":false,"n_valid_pairs":1,"origin_p99_diff_mean":0.0,"rel_ci_hi":0.0,"rel_ci_lo":0.0,"rel_reduction_mean":0.0,"success_diff_pp_mean":0.0},"outcome":"MECHANISM_FAILED","reasons":["origin concurrency exceeded the cap in force in at least one run (H-B1 falsified)"]}}

---- u01_incomplete::not_cancelled_but_missing_runs_is_also_incomplete stdout ----

thread 'u01_incomplete::not_cancelled_but_missing_runs_is_also_incomplete' (27780) panicked at tests\integration\lab_live_scenario_b_tests.rs:507:9:
verdicts must be null, got {"STATIC-CAP":{"evidence":{"contention_flagged":false,"mechanism_pass":true,"n_valid_pairs":1,"origin_p99_diff_mean":0.0,"rel_ci_hi":0.0,"rel_ci_lo":0.0,"rel_reduction_mean":0.0,"success_diff_pp_mean":0.0},"outcome":"INCONCLUSIVE","reasons":["only 1 valid paired runs; 10 required"]}}


failures:
    u01_incomplete::cancelled_partial_keeps_raw_diagnostics_but_withholds_verdict
```

The one passing test, `completed_run_with_real_violation_is_still_mechanism_failed`, is a guard that completed-run semantics are unchanged (passes before and after).

## AFTER (sandbox with fix)

Command: `cargo test -j 1 --test lab_live_scenario_b_tests` (whole target). Result: `65 passed; 0 failed; 2 ignored` (ignored = pre-existing heavy tests). Includes the 6 new u01_ tests, `e2e_cancellation_terminates_children`, `e2e_smoke_three_arms_artifacts_cap_and_no_orphans`, and the lab_api_tests submodule (`scenario_b_cancel_terminates_the_gateway_child`, `scenario_b_via_api_short_phases_is_exploratory_in_debug_and_leaves_no_orphans`). Raw: D:\Temp\sbx_u01_after.txt.

Not run: other test targets (e.g. the separate lab_api review test `d1_summary_carries_the_exact_notes_and_wording` is part of this target and passed), clippy, UI tests, ignored heavy tests, release profile. The existing cancel e2e does not itself assert `verdicts == null`; the cancelled shape is covered by the pure `summarize()` tests. A real cancelled e2e assertion is a suggested follow-up.
