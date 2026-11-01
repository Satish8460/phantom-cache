# U01 backend remediation status

Agent U01 (AI), 2026-10-30, machine clock about 15:41-16:00 IST. Sandbox only. State: DONE in sandbox, NOT integrated, NOT committed.

- Main-tree guard: HEAD 3e6e03e9e2eb0ef6281f98d6876fce1fc1f3d15f, branch feat/publish-phantomcache-core, 107 `git status --short` entries at start. The main-tree `src/lab_live/scenario_b.rs` has the same sha256 as the sandbox base (8f40517c...c16c3b). Only `docs/research/u01_backend/` was written in the main tree.
- Sandbox: D:\Temp\sbx_u01 (src, tests, benches, Cargo.toml, Cargo.lock, plus docs/demo/scenario_b_interpretation_notice.md, which `include_str!` in lab_api.rs needs). Target dir D:\Personal\_sbx_u01_target. No dependency or Cargo.toml change.
- Files changed in sandbox: `src/lab_live/scenario_b.rs` (+59/-13); `tests/integration/lab_live_scenario_b_tests.rs` (new `mod u01_incomplete`, 6 tests, in the already registered target).
- Tests first: 5 of 6 new tests FAIL on unmodified source, then pass after the fix; whole target 65 passed, 0 failed, 2 ignored (includes the cancel e2e and the Lab API Scenario B cancel tests). See test_evidence.md.
- Behaviour: cancelled or partial summaries (defensively also any arm with fewer runs than reps) get `verdicts: null`, `mechanism_h_b1_all_pass: null`, `mechanism_h_b1.<arm>.pass: null`, `verdicts_status: "not_computed"`, `run_completeness{...}`, `label: "EXPLORATORY"` plus `label_planned`. Raw per-run metrics and mechanism observations are kept. Completed runs: verdict values unchanged (additive keys only). Failed runs and killed processes write no summary.
- UI contract: contract.md. UI not touched.
- Files here: u01_backend.patch, defect_reproduction.md, test_evidence.md, contract.md, independent_review_brief.md.
- Residual risks: (1) `lab_api.rs` retained snapshot and result event still coerce null verdicts to `{}`; CLI unchanged (outside the targeted patch); (2) `summary_version` still 1; (3) legacy cancelled artifacts keep the old misleading shape, the UI must handle it; (4) `paired_vs_base` still emitted for partial runs; (5) debug-profile, one test target only, no clippy; (6) patch made with `diff -u`, not tested with `git apply`.
