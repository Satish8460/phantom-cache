# bench-results (Phase 1 baseline artifacts)

**Status:** Active (evidence directory index; the contents are retained, unedited records)
**Audience:** reviewers checking the M1 baseline and tooling records
**Last reviewed:** 2026-10-30
**Authority and related:** [../docs/research/phase_1_execution_package.md](../docs/research/phase_1_execution_package.md) (the tooling report cites its section 6.6 as specification), [../docs/research/phase_1_completion_report.md](../docs/research/phase_1_completion_report.md) (historical; its PASS claim is contradicted, current status is in [../docs/STATUS.md](../docs/STATUS.md)), [../docs/research/benchmark_suite_spec.md](../docs/research/benchmark_suite_spec.md), [../docs/testing/strategy.md](../docs/testing/strategy.md).

## What this is

Outputs of the Milestone 1 (Truth and Baseline) work, written by the execution agent Antigravity (AG), timestamp 2026-10-28 in `tooling_report.md`:

- `tooling_report.md`, `wsl_check.sh`, `wsl_inspect.sh`: tooling and environment probes.
- `r0_summary.json` and `manifests/` (`r0_baseline_manifest.json`, `ra_manifest.json`, `rb_manifest.json`, `rc_manifest.json`): run manifests recording the git head, a source hash and platform labels.
- `logs/`: test, clippy and dependency logs, including `audit_all_targets.log`, an earlier audit sweep (499 passed, 65 targets including bench mains, per the ledger; an old tree).
- `cluster/characterization_report.json`: the M3 characterization of the distributed gaps, cited by [ADR-003](../docs/research/adr/ADR-003-distributed-architecture.md).
- `tools/`: the Python runners that produced the raw results. `raw/` is empty. `tools/__pycache__/` is generated and not evidence.

## Tier and labels

Baseline and loopback characterization on one Windows machine, 2026-10-28 and 2026-10-29. No release-profile benchmark result is established by this directory; the accountability report states that no release-profile run of the general suite exists and the benches were not re-run on the current tree. Treat any number here as local, single-host and dated, and do not quote it as a measurement without the conditions required by [../CLAUDE.md](../CLAUDE.md) section 4. Profile and workload per file were not checked for this README (unverified).

## Rules

- Do not edit the contents. They record the tree as it was (the manifests pin git head `456bb25` with a dirty flag).
- Retention: the owner approval recorded in the ledger (A-8 of Phase 1) keeps this directory untracked and uncommitted ([../docs/research/progress_ledger.md](../docs/research/progress_ledger.md)). Size about 500 KB (measured with `du`, 2026-10-30).
- Hash pointers: each manifest's own `src_hash`. No separate checksum list exists (unverified).
