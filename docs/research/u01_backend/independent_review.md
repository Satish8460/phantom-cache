# U-01 backend patch: independent review

Status: REVIEW COMPLETE. Independent of the implementer; AI review, not human review.
Audience: project owner / lead (integration decision).
Last reviewed: Fri Oct 30 16:55 IST 2026 (machine clock, `date`).

## 1. Scope and method

Reviewed `docs/research/u01_backend/u01_backend.patch` (diff -u against main-tree `src/lab_live/scenario_b.rs` plus new tests in `tests/integration/lab_live_scenario_b_tests.rs`). Nothing in the main tree was modified except the two new files in `docs/research/u01_backend/` (this file and `independent_review_lab_api_proposal.patch`).

- Fresh sandbox `D:\Temp\sbx_u01_review` (src, tests, benches, Cargo.toml, Cargo.lock, `docs/demo/scenario_b_interpretation_notice.md`); not the implementer's sandbox. `CARGO_TARGET_DIR=D:\Personal\_sbx_u01rev_target`. All cargo runs `-j 1`, through `with_cargo_lock.ps1 -Label U01REV`. Debug profile only.
- Sandbox base check: main-tree `scenario_b.rs` sha256 starts `8f40517c7d49...` = the base the implementer states. `git apply --check` on the patch inside the sandbox: clean (both files).
- Main-tree guard: HEAD was `caeedfd` at session start and `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` at the end (another agent committed meanwhile; not me; the implementer recorded `3e6e03e`). Branch `feat/publish-phantomcache-core`; `git status --short` = 111 entries before and after. `src/lab_live/scenario_b.rs`, `analysis.rs`, `lab_api.rs` unchanged by me.

## 2. Independence statement

Order of work: (a) read main-tree `scenario_b.rs` (summarize, run_experiment, compute_run_metrics), `analysis.rs`, `lab_api.rs` call sites, `cli.rs`, `labView.js`; (b) wrote my own tests (`tests/integration/u01rev_tests.rs` in the sandbox, 6 tests, own `RunMetrics` fixtures via `Default`) and ran them on the UNPATCHED copy; (c) only then read the patch's test hunk. I read the patch's source hunks before my tests (needed for the task) but did not read `status/defect_reproduction/test_evidence/contract/independent_review_brief.md` until after my reproduction was written and run. I did not read the sealed M6 directory.

## 3. Reproduction evidence (unpatched copy)

Command: `cargo test -j 1 --test u01rev_tests -- --nocapture --test-threads=1` (log `D:\Temp\sbx_u01_review\unpatched_u01rev.log`). Result: 2 passed, 4 failed.

| Test (mine) | Unpatched | Why |
|---|---|---|
| rev_cancelled_zero_runs_has_no_mechanism_failed_verdict | FAIL | zero runs gives `verdicts.*.outcome=MECHANISM_FAILED`, `mechanism_h_b1_all_pass=false` (scenario_b.rs:658 `!rs.is_empty()`, analysis.rs:131-134) |
| rev_cancelled_partial_gives_no_completed_outcome | FAIL | 2 reps done of 10 gives a completed-looking INCONCLUSIVE |
| rev_cancelled_summary_label_is_not_valid | FAIL | `label` stays `VALID` on `status=CANCELLED` (scenario_b.rs `"label": meta.label`). Isolated: the only assertion in the test |
| rev_e2e_cancel_writes_no_verdict_and_not_valid (real gateway, real cancel) | FAIL | on-disk `summary.json`: `status="CANCELLED"`, `verdicts={"AIMD-CAP":{...,"outcome":"MECHANISM_FAILED"...}...}`. Label was EXPLORATORY only because this is a debug build |
| rev_partial_keeps_raw, rev_golden_completed_cases | ok | raw-diagnostics and completed-run baselines (dumped to `golden_before/`) |

So the defect is reproduced end to end from a real cancelled run, not only from a pure call.

## 4. Results after the patch

| Run | Command | Result |
|---|---|---|
| My tests, patched | `cargo test -j 1 --test u01rev_tests` | 6 passed, 0 failed, 0 ignored. e2e on disk: `status=CANCELLED label=EXPLORATORY verdicts=null all_pass=null`; `o.label=EXPLORATORY` |
| Target, unpatched baseline | `cargo test -j 1 --test lab_live_scenario_b_tests -- --test-threads=1` | 59 passed, 0 failed, 2 ignored |
| Target, patched | same | 65 passed, 0 failed, 2 ignored (59 + the 6 implementer tests; no new failure) |
| Target, patched + my lab_api proposal | same | 65 passed, 0 failed, 2 ignored |

The target includes the `lab_api` submodule (Lab API scenario_b tests: cancel terminates child, short-phase run, notes/wording, retained evidence). Not run: other targets, clippy, UI tests, ignored heavy tests, release profile.

## 5. The seven points

### Point 1: cancelled/failed/interrupted runs cannot produce completed mechanism verdicts. PASS (new artifacts); PARTIAL for legacy artifacts

Writers of `summary.json` (grep over `src/`): only `run_experiment` at scenario_b.rs:1265 (patched ~1310). Paths:
- Cancel (`ScenarioError::Cancelled` from `run_one`, Ctrl-C in `cli.rs` and Lab API cancel both set the flag): breaks the loop, calls `summarize` with `cancelled=true`. Patched: verdicts null, all_pass null, pass null, label EXPLORATORY. Verified on disk by my e2e.
- Failed run (`Err(e)`, scenario_b.rs:1247-1257): writes manifest `FAILED` and returns; no summary. Verified by reading. Lab API maps it to state `failed` with no result (lab_api.rs ~1875).
- Killed process: no summary, manifest stays `RUNNING`. Verified by reading (no code runs).
- `runs_missing` branch: defensive, unreachable today (a non-cancelled loop either completes every run or returns Err). Harmless.
- Cancel raised after the last run completes: `cancelled=false`, status COMPLETE, complete verdicts. Correct (the design finished).
- Legacy artifacts already on disk keep the old shape. See R1.
- A complete design containing INVALID runs (valid=false) still gets verdicts (via n_valid_pairs / INCONCLUSIVE). Pre-existing, unchanged, not the U-01 defect. Note `mechanism_violated` is computed for invalid runs too (compute_run_metrics:408).

### Point 2: completed-run behaviour unchanged. PASS

Same inputs through `summarize` before/after, six cases (`golden_before` vs `golden_after`): A benefit 10 reps (BENEFIT_SUPPORTED), B 1 rep (INCONCLUSIVE), C genuine violation at rep 4 (MECHANISM_FAILED, all_pass false), D complete design with one invalid run, E three arms 10 reps (BENEFIT_SUPPORTED both capped arms), F planned EXPLORATORY with reasons. `diff` shows ONLY additions in every case: `label_planned`, `run_completeness{complete:true,...}`, `verdicts_status:"computed"`, `verdicts_reason:null`. No value of verdicts, evidence, mechanism, arms, paired_vs_base, label or label_reasons changed. `serde_json` has no `preserve_order` (grep of Cargo.toml/Cargo.lock empty), so keys are sorted and key order is not a variable.

### Point 3: EXPLORATORY classification. PASS with one test gap

`effective_label`: incomplete returns EXPLORATORY plus the planned reasons plus an "incomplete" reason; complete returns the planned label untouched. `summarize` and `run_experiment` both use `incomplete_reason(cfg, runs, cancelled)` so `summary.json` and `ExperimentOutcome.label` agree (checked at patched run_experiment tail). `ExperimentOutcome.label` consumers: cli.rs:148 (prints), lab_api.rs:1864 (result event; only reached for non-cancelled, so the label there is the planned/complete one), tests. `ExperimentOutcome.summary["label_reasons"]` (used at lab_api.rs:1864) is the effective list. Manifest `label_planned` (write_manifest, via `label_for`) is intentionally planned and unchanged; the detail view (lab_api.rs:1416) shows manifest `label_planned` next to the snapshot `label`, consistent naming. Cannot-be-VALID path: none found. Gap: no test can show `VALID` suppression through `run_experiment`/`o.label` (debug builds are never VALID, so e2e always yields EXPLORATORY); the line is covered only by reading and the pure `summarize` tests. See F6.

### Point 4: raw diagnostics preserved. PASS

My `rev_partial_keeps_raw` (3 reps, violation at rep 1) passes both before and after: `violating_reps=[1]`, `max_origin_concurrent_observed_whole_run=99`, `cap_limit_in_force=24`, `runs=3`, `arms.BASE.runs=3`, `paired_vs_base.STATIC-CAP` object, `config`, `limitations` all present. Zero-run case: `max_origin_concurrent_observed_whole_run` null, `runs` 0. Only `pass`, `mechanism_h_b1_all_pass`, `verdicts`, `label` change (plus additive keys). Caveat F4: `paired_vs_base` is still emitted for partial runs.

### Point 5: the two null-verdict call sites in `src/api/lab_api.rs`. FAIL (not handled; unchanged by the patch)

Current lines opened: 1384-1387 (retained-evidence/detail `verdict_snapshot`) and 1855-1858 (`result` event, non-cancelled `Ok(o)` only). Both do `X["verdicts"].as_object().map(...).unwrap_or_default()` into a `serde_json::Map`. `Value::Null.as_object()` is `None`, so `unwrap_or_default()` yields an EMPTY map and the payload carries `"verdicts": {}`. Site 1 is reachable with the patch: any new cancelled `scenario_b` artifact read through `GET /evidence` (snapshot) or the evidence detail gets `"verdicts": {}` alongside `verdicts_status` that the snapshot does not copy. That is the same misleading shape as the UI fixture (`verdicts={}`: "no arm verdicts", indistinguishable from a legacy run with no capped arm). The detail path (`verdict_full`, lab_api.rs ~1397-1400) passes the whole summary so it keeps null. Site 2 is effectively unreachable with the patch (a non-cancelled run is always complete), so it is hygiene. Correct handling: keep `null` when the source is not an object, and copy `verdicts_status`, `verdicts_reason`, `run_completeness`, `label_planned` into the snapshot. Minimal fix proposed as `independent_review_lab_api_proposal.patch` (never applied to the main tree). I applied it only in the sandbox on top of the patch: compiles, `lab_live_scenario_b_tests` 65 passed, 0 failed, 2 ignored. No test exercises a cancelled-run snapshot; the proposal is not covered by a new test (UNVERIFIED beyond "no regression"). UI parser `ui/src/lib/labView.js:124` (`isObj(snapshot.verdicts) ? ... : {}`) already tolerates null, so the UI side needs no change for the null form.

### Point 6: regression tests genuinely reproduce the defect. PARTIAL

Reproduced by me on the unpatched copy rather than taken from the implementer's report. I ran the implementer's `u01_` tests only in the patched state (the target passes: 65). Reading them:
- 5 of 6 are real regressions (verdict null, all_pass null, pass null, status, EXPLORATORY). They fail for the right reason: the first assertion is `verdicts.is_null()`.
- The label assertion comes after the verdict assertion in `assert_incomplete`, so on unpatched code the label defect is never the failing line. My isolated `rev_cancelled_summary_label_is_not_valid` closes that.
- `completed_run_keeps_verdict_semantics` fails on unpatched code only because it asserts the NEW keys (`verdicts_status`, `run_completeness`), so it is not a pure behaviour guard; `completed_run_with_real_violation_is_still_mechanism_failed` is the real guard and passes both ways (cannot fail on the defect, by design).
- `no_outcome_anywhere(&s["verdicts"])` on a null value is vacuous (the string "null"); it adds nothing beyond `is_null`. `no_outcome_anywhere(&s["mechanism_h_b1"])` is meaningful.
- No test for `run_experiment`/`ExperimentOutcome.label` and none asserts the real cancelled e2e `summary.json` (implementer admits this). My e2e covers the on-disk file.

### Point 7: no unrelated behaviour changed. PASS with documentation gap

Every hunk is justified: new pure helpers (`incomplete_reason`, `effective_label`, constant), `pass` null, verdict loop guarded, `h_b1` null, additive keys, run_experiment label. `summary_version` stays 1 (additive keys plus three keys that may become null: a type change for `verdicts` object-to-null, `mechanism_h_b1_all_pass` bool-to-null, `mechanism_h_b1.*.pass` bool-to-null; owner decision whether to bump). Consumers: `labView.js` (lines 325 and 353-364 already read `verdicts_status`, `run_completeness`, `label_planned`; `typeof ... === 'boolean'` guards give null), `cli.rs:151` prints no verdict lines on null (correct), `lab_api.rs` see Point 5, artifact index (`lab_api.rs:1927`, `2104`) uses file names only (read, not run). The retained evidence `scenario_b/20261030T003932Z` is a completed summary: unaffected (no new keys; the existing lab_api evidence test depends on it). Docs `docs/demo/scenario_b_artifacts.md` (lines 24, 66) and `lab_api.md` are not updated by the patch and do not describe the new keys or the nulls.

## 6. Findings

| ID | Point | Finding | Evidence | Severity | Recommendation |
|---|---|---|---|---|---|
| F1 | 1,3,4 | Defect real; patch fixes it for newly written summaries | my 4 failing unpatched tests; on-disk cancelled summary before/after | Info | none |
| F2 | 5 | `lab_api.rs:1384-1387` converts null verdicts to `{}` in the evidence snapshot (reachable for any new cancelled artifact); site 1855-1858 same but unreachable | lab_api.rs lines opened; `Value::Null.as_object()==None` | High (integration blocker for UI correctness: the misleading `{}` returns through the API) | apply the proposal (`independent_review_lab_api_proposal.patch`) together with the patch, add a test with a cancelled summary |
| F3 | 1 | Legacy cancelled artifacts keep verdicts object, possibly MECHANISM_FAILED, and `label: VALID`; no `verdicts_status` | patch changes only the writer | Medium | UI/API must also key on `status=="CANCELLED"`; never edit retained artifacts; consider a read-side override in the snapshot |
| F4 | 4 | `paired_vs_base` (with BCa CI, perm p from a handful of pairs) is still emitted for partial runs next to `verdicts:null` | my rev_partial_keeps_raw dump `P_partial3.json` | Low-Medium | accepted as raw diagnostics only if the UI labels it partial; contract says so; consider a note key |
| F5 | 7 | `summary_version` unchanged though three keys change type to null | patched summarize | Low | owner decision; bump to 2 or document |
| F6 | 3,6 | No test proves `ExperimentOutcome.label`/VALID suppression through `run_experiment`; label assertion trails verdict assertion in the implementer's helper; one test vacuous on null; one "guard" test fails only on new keys | test hunk lines 178-201, 245-259 | Low | add my isolated label test and an e2e assertion on `summary.json` (both exist in the sandbox `u01rev_tests.rs`) |
| F7 | 7 | `scenario_b_artifacts.md` / `lab_api.md` do not document the null shapes and new keys | docs lines 24, 66 | Low | update docs with the integration |
| F8 | extra | `completed_runs` counts invalid runs too; `planned_runs`=reps*arms | patch `run_completeness` | Info | name is fine; clarify in contract |
| F9 | extra | Main-tree HEAD moved from the implementer's `3e6e03e` to `804a7b8` while the patch base (scenario_b.rs sha) is unchanged | git rev-parse; sha256 | Info | recheck sha before applying |
| F10 | extra | invalid runs in a complete design still produce verdicts, `mechanism_violated` computed for invalid runs | scenario_b.rs:408, summarize | Info | pre-existing, out of U-01 scope |

Counts: Critical 0, High 1 (F2), Medium 2 (F3, F4), Low 3 (F5, F6, F7), Info 4 (F1, F8, F9, F10).

## 7. Disagreements with the implementer

1. "5 of 6 new tests fail on unmodified source": true, but one of the five (`completed_run_keeps_verdict_semantics`) fails only because it asserts new keys, so it is not evidence of the defect (4 of 6 are). Their own note says only that the guard passes.
2. `defect_reproduction.md` itself concedes that the label defect is shown only by reading; I show it with an isolated failing test.
3. They list the `lab_api.rs` null-to-`{}` coercion as "follow-up/residual". I rate it a blocker for the API/UI correctness the fix is meant to deliver (F2), since `{}` re-creates the exact shape the UI fixture hand-sets.
4. "`runs_missing` ... defensive": agreed; unreachable.
5. The contract says the UI must tolerate legacy shapes; agreed, and I add that the API snapshot could do this read-side (F3).
Agreed: failed/killed runs write no summary; summary_version unchanged; raw diagnostics retained; paired_vs_base still emitted; Cargo.toml untouched.

## 8. Residual risks

Legacy cancelled artifacts (F3); partial `paired_vs_base` (F4); `verdicts` type change for any external reader (none found in-repo besides those listed); the `{}` coercion until F2 is fixed; debug-only evidence.

## 9. Not checked

Release-profile runs; clippy; other test targets; the ignored heavy tests; the UI test suite and any browser rendering; `git apply` into the real main tree (checked only in the sandbox); a Lab API test with a cancelled summary snapshot (none exists; proposal unproven by a new test); artifact-index code beyond grep; Windows-path edge cases; whether any file outside `src/`, `tests/`, `ui/` reads `summary.json` (docs only grepped); concurrent main-tree edits by other agents after 16:55.

## 10. Recommendation

INTEGRATE AFTER FIXES. The `summarize`/`run_experiment` change is correct, minimal and additive for completed runs (6 golden cases byte-identical aside from added keys; baseline 59 pass vs patched 65 pass, no new failure; defect reproduced and fixed end to end on a real cancelled run). Blocking item: handle null `verdicts` in `src/api/lab_api.rs:1384-1387` (and 1855-1858) with the proposal in `independent_review_lab_api_proposal.patch`, plus a test with a cancelled summary. Non-blocking: add the isolated label test and e2e summary.json assertion (F6), update docs (F7), decide on `summary_version` (F5).
