# lab-results (retained experiment outputs)

**Status:** Active (evidence directory index; the contents are retained, frozen or sealed records)
**Audience:** reviewers of the M6 evaluation and Scenario B results
**Last reviewed:** 2026-10-30
**Authority and related:** [../docs/research/m6_sealed_results.md](../docs/research/m6_sealed_results.md), the protocol chain in [../docs/research/README.md](../docs/research/README.md) section 6, [../docs/demo/scenario_b_artifacts.md](../docs/demo/scenario_b_artifacts.md), [../docs/demo/scenario_b_interpretation_notice.md](../docs/demo/scenario_b_interpretation_notice.md), [../docs/demo/scenario_b_verification/README.md](../docs/demo/scenario_b_verification/README.md), [../docs/demo/lab_runner.md](../docs/demo/lab_runner.md).

## What this is

| Path | Tier | Produced by and cited in |
|---|---|---|
| `m6_sealed_2026-10-29T234845Z/` and `m6_sealed_run_started.marker` | SIMULATED (T1) | the sealed M6 evaluation, protocol v2.2 ([m6_sealed_results.md](../docs/research/m6_sealed_results.md)); sealed, never edit |
| `m6_dryrun_*`, including the folder whose name ends `ABORTED_INCOMPLETE_by_my_own_timeout` | SIMULATED (T1), development seeds | dry runs before the sealed run; the aborted folder is kept as a record |
| `tuning/`, `tuning_v2_*`, `tuning_v2_2*` | SIMULATED (T1) | arm-parameter tuning on tuning seeds; per the ledger, `tuning/frozen_arm_params.json` and `tuning/tuning_trials.csv` are hashed by the v2 freeze |
| `candidates_m5/`, `decision_traces.json`, `decision_traces_manifest.json` | Unclassified (tier not checked) | M5 candidate computations and decision traces |
| `scenario_b/20261030T003932Z/` | MEASURED_LOOPBACK (T2) | the Scenario B experiment (30 runs); interpret only with the notice; independently recomputed in `docs/demo/scenario_b_verification/` |
| `smoke_*` | exploratory | smoke and determinism checks; evidence of no claim |
| `runs/` | none | empty at the time of writing |

Tier definitions: [../docs/glossary.md](../docs/glossary.md) section 5. Never mix T1 and T2 numbers.

## Rules

- Do not edit, rename or delete anything here. Frozen and sealed material has hash records in `docs/research/preregistration_m6_v2_*freeze.json` and the addendum pin; changing a file breaks verification (the freeze-integrity test fails closed by design).
- Do not open salt or seed-secret material, and never copy a salt value into any document or into this directory. This README was written without reading any such directory.
- Hash and manifest pointers: the `manifest.json` in each run folder (for example `scenario_b/20261030T003932Z/manifest.json`, `m6_sealed_2026-10-29T234845Z/manifest.json`) and the freeze JSON files under `docs/research/`.
- Retention: about 48 MB in total (measured with `du`, 2026-10-30), of which `scenario_b/` is about 28 MB. Per the ledger commit plan, the two tuning files above must be available so the freeze is verifiable from a clone, and the large generated smoke outputs (`smoke_lib_det_*`, about 2.1 MB each per the ledger) stay local and uncommitted. Large smoke outputs are exploratory and should not be committed. Everything else is an owner decision; run a secret scan before any commit.
