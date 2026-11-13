# U-01 backend contract: Scenario B `summary.json` for incomplete runs

Status: SANDBOX PROPOSAL (AI work, not independently verified). Implements option R2 of
`docs/design/remediation/06_functional_ux_work_breakdown.md` plus the zero-run fix (R4) and G4C F02.
`summary_version` stays `1` (additive keys; three existing keys may now be `null`). Whether to bump it is an owner decision.

## Definition of "incomplete"

`run_completeness.complete == false` when either:

| `incomplete_reason` | Condition | Reachable today |
|---|---|---|
| `"cancelled"` | the cancel flag was raised (`meta.cancelled`, `status == "CANCELLED"`) | yes (Ctrl-C, Lab API cancel) |
| `"runs_missing"` | not cancelled, but some arm has fewer runs than `config.reps` | defensive only |

A run that fails (`run_experiment` returns `Err`) or a process that is killed writes NO `summary.json` (manifest `status: "FAILED"`, or stays `"RUNNING"`). Absence of the file is the signal; there is nothing to mis-label. If a future code path writes a summary for a failed/interrupted run it must go through `summarize()` and is covered by `runs_missing`.

## Keys (all present in every new summary)

| Key | Complete run | Incomplete run |
|---|---|---|
| `status` | `"COMPLETE"` | `"CANCELLED"` (unchanged) |
| `run_completeness` | `{complete:true, incomplete_reason:null, planned_runs:N, completed_runs:N}` | `{complete:false, incomplete_reason:"cancelled"\|"runs_missing", planned_runs:N, completed_runs:M<N}` |
| `verdicts_status` | `"computed"` | `"not_computed"` |
| `verdicts_reason` | `null` | string (fixed text, `VERDICTS_NOT_COMPUTED_REASON`) |
| `verdicts` | object keyed by capped arm (unchanged shape) | **`null`** (was an object with `outcome` strings) |
| `mechanism_h_b1_all_pass` | bool | **`null`** (was bool, `false` for zero runs) |
| `mechanism_h_b1.<arm>.pass` | bool | **`null`** |
| `mechanism_h_b1.<arm>.{runs, violating_reps, max_origin_concurrent_observed_whole_run, cap_limit_in_force}` | observations | observations, unchanged (raw diagnostics kept; `violating_reps` is an observation, not a verdict) |
| `label` | planned label (`VALID` or `EXPLORATORY`) | **always `"EXPLORATORY"`** |
| `label_planned` | same as `label` | the config-derived label (`VALID` possible) |
| `label_reasons` | planned reasons | planned reasons + `"run incomplete (<reason>): the planned label X does not apply to a partial run"` |
| `arms`, `paired_vs_base`, `config`, `limitations`, `provenance`, `interpretation_notes` | unchanged | unchanged (partial data, not a result) |

`ExperimentOutcome.label` (Rust, feeds the Lab API `result` event label) uses the same effective label.

## UI rules (for the lead's UI correction; UI is not edited here)

1. Primary key: run state (`completed` / `cancelled` / `failed`). Secondary: `summary.status`. Tertiary guard: `verdicts_status === "not_computed"` or `run_completeness.complete === false` means no verdict chips, no mechanism sentence, no "H-B1" wording, even if state says completed.
2. `verdicts === null` must be tolerated (`isObj(s.verdicts) ? s.verdicts : {}` already does) and must NOT be rendered as the "needs a BASE arm" message.
3. Display `label` as-is; for an incomplete summary it is `EXPLORATORY` and `label_reasons` explains why. `label_planned` may be shown as "planned (not applicable)".
4. Legacy artifacts (written before this change) keep the old shape (verdicts object, possibly `MECHANISM_FAILED` on a cancelled run, label `VALID`). They have no `verdicts_status`/`run_completeness`; the UI must still suppress verdicts for cancelled state/`status:"CANCELLED"` and must not trust a `VALID` label there. Retained artifacts are never edited.
5. Test fixtures must be produced from real `summarize()` output (the sandbox tests print the shape), not hand-set `verdicts = {}`.

## Known consumers NOT changed in this sandbox (src/api and src/lab_live/cli.rs are out of the targeted patch)

- `src/api/lab_api.rs:1384-1386` and `:1855-1857`: `sj["verdicts"].as_object().unwrap_or_default()` turns `null` into `{}` and loses the signal in the retained-evidence snapshot / result event. They should pass `null` and `verdicts_status` through (follow-up).
- `src/lab_live/cli.rs:151`: `as_object()` is `None` for `null`, so a Ctrl-C run prints no `verdict <arm>` lines. Verified by reading; this is the desired behaviour.
