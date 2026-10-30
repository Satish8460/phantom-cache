# docs/research index

**Status:** Active (index)
**Audience:** readers navigating the research, milestone and experiment records; contributors adding to them
**Last reviewed:** 2026-10-30
**Authority and related:** current status is in [../STATUS.md](../STATUS.md), not in any file listed here. Status classes below come from the D2 inventory ([doc_audit/d2_research_docs_audit.md](doc_audit/d2_research_docs_audit.md), written from file headers and banners, not full reads) plus the consolidated plan ([doc_audit/00_consolidated_documentation_audit_and_plan.md](doc_audit/00_consolidated_documentation_audit_and_plan.md)). Where D2 gives no evidence the class is **Unclassified**. This index copies no content. Terms: [../glossary.md](../glossary.md).

## How to read this folder

- Files at this level: 66 markdown files; subfolders add 19 (`adr/` 3, `m3_durability/` 3, `ttl_decision/` 2, `vision/` 6, `doc_audit/` 5), counted 2026-10-30. Non-markdown records (6 `phase_N_changes.patch`, 5 preregistration/freeze `.json`/`.pin` files, `evidence/` log folders, `*.diff` proposals) are listed in the groups where they belong.
- **Frozen or append-only (never edit; corrections are dated notices):** preregistrations and amendments, freeze JSON files, the addendum and its pin, sealed results, gate verdicts, Scenario B artifacts and their interpretation notice (outside this folder: [../demo/scenario_b_interpretation_notice.md](../demo/scenario_b_interpretation_notice.md)), the claims ledger history, the progress ledger log (section D), phase completion reports and progress logs, patches, retracted reports. See [../contributing.md](../contributing.md) section 7.
- Several milestone completion reports state PASS; the accountability report and ledger say no milestone passed independently. Read those reports as what an agent reported on the date, not as current status.

**Status classes:** Active (maintained, current for its category), Verified (a named independent check is cited; none is assigned here unless the document itself names it), Proposed (plan, spec or investigation awaiting decision), Draft, Superseded (replaced; successor named), Historical-frozen (retained record, do not edit), Unclassified.
**Authority classes (D2):** A authoritative for a category, R retained historical record, P plan or spec, E evidence or log, X working draft or proposal, D derived or duplicate view.

## 1. Status, plans and entry points

| File | Status class | Authority | Note |
|---|---|---|---|
| [progress_ledger.md](progress_ledger.md) | Active (sections A to C stale; section D append-only) | R + live | event history; hashes and commands |
| [phantomcache_milestone_accountability_report.md](phantomcache_milestone_accountability_report.md) | Active (partly stale: G1/G2 outcomes superseded by the verdicts) | A | latest status reading as of its edit time |
| [project_status_and_accountability_2026-10-30.md](project_status_and_accountability_2026-10-30.md) | Historical-frozen (snapshot report) | R | states itself historical once written |
| [master_audit_and_execution_plan.md](master_audit_and_execution_plan.md) | Active, two layers (section 2 table is the original audit) | A | appended rounds replace some rows |
| [mvp_and_milestones.md](mvp_and_milestones.md) | Proposed (Rev 2; no M8) | P | roadmap M1 to M7 |
| [phase_execution_packages_index.md](phase_execution_packages_index.md) | Proposed (stale: says phase 1 is being executed) | P | adds M8 and common rules |
| [implementation_master_plan.md](implementation_master_plan.md) | Superseded in practice (no banner) | P | first hand-off entry point |
| [plan_of_action.md](plan_of_action.md) | Superseded (banner: by the milestone plan) | P | |
| [phase_implementation_file_map.md](phase_implementation_file_map.md) | Unclassified | P/D | navigation map, snapshot of 2026-10-28 |
| [work_packages.md](work_packages.md) | Superseded in part (Phase 1 parts by `phase_1_execution_package.md`) | P | |
| [ag_task_WP-00.md](ag_task_WP-00.md) | Superseded (banner) | P | |
| [skill_strategy.md](skill_strategy.md) | Proposed | X | tooling proposal; off-topic for this folder |
| [storage_cleanup_2026-10-30.md](storage_cleanup_2026-10-30.md) | Historical-frozen (append dated amendments) | R | cleanup record |

## 2. Phase and milestone records (scheme A; M1 to M8)

Packages are headed "PROPOSED" although phases 1 to 6 were executed; no owner authorization record exists in the repository (evidence gap).

| Milestone | Package | Report | Progress log | Notes |
|---|---|---|---|---|
| M1 | [phase_1_execution_package.md](phase_1_execution_package.md) (Proposed header, spec of record v3.1) | [phase_1_completion_report.md](phase_1_completion_report.md) (Historical-frozen; PASS reported, contradicted) | [phase_1_progress.md](phase_1_progress.md) (Historical-frozen; stops early) | [phase_1_snapshot_manifest.md](phase_1_snapshot_manifest.md) (Historical-frozen, E), `phase_1_changes.patch` (E) |
| M2 | [phase_2_execution_package.md](phase_2_execution_package.md) (Proposed) | [phase_2_completion_report.md](phase_2_completion_report.md) (Historical-frozen) | [phase_2_progress.md](phase_2_progress.md) | `phase_2_changes.patch` |
| M3 | [phase_3_execution_package.md](phase_3_execution_package.md) (Proposed) | [phase_3_completion_report.md](phase_3_completion_report.md) (Historical-frozen, contradicted) | [phase_3_progress.md](phase_3_progress.md) | `phase_3_changes.patch` |
| M4 | [phase_4_execution_package.md](phase_4_execution_package.md) (Proposed) | [phase_4_completion_report.md](phase_4_completion_report.md) (Historical-frozen, contradicted) | [phase_4_progress.md](phase_4_progress.md) | `phase_4_changes.patch` |
| M5 | [phase_5_execution_package.md](phase_5_execution_package.md) (Proposed) | [phase_5_completion_report.md](phase_5_completion_report.md) (Historical-frozen, contradicted by claim C8) | [phase_5_progress.md](phase_5_progress.md) | [phase_5_preregistration.md](phase_5_preregistration.md) (Historical-frozen), `phase_5_changes.patch` |
| M6 | [phase_6_execution_package.md](phase_6_execution_package.md) (Proposed) | [phase_6_completion_report.md](phase_6_completion_report.md) (Superseded: retracted banner) | [phase_6_progress.md](phase_6_progress.md) | `phase_6_changes.patch` (size suggests cumulative with phase 5; unverified) |
| M7 | [phase_7_execution_package.md](phase_7_execution_package.md) (Proposed) | none in this folder | none | see [../security/m7_security_review.md](../security/m7_security_review.md) and the ledger |
| M8 | [phase_8_execution_package.md](phase_8_execution_package.md) (Proposed; derived stages, see its O8-0) | none in this folder | none | see [../design/phantom_lab_ui.md](../design/phantom_lab_ui.md) and the ledger |

Total size of the six patches is about 2.9 MB (D2); they are cumulative-looking diffs kept as evidence.

## 3. Gates and decisions

| File | Status class | Authority | Note |
|---|---|---|---|
| [g1_verdict.md](g1_verdict.md) | Historical-frozen (decision record) | R | G1 Scenario B: PASS with five dispositions |
| [g2_verdict.md](g2_verdict.md) | Historical-frozen; one item open (G2-D1 outcome not recorded here) | R | G2 Lab API |
| [adr/README.md](adr/README.md) | Active (index) | A | ADR-001 to ADR-003 |
| [ttl_decision/recommendation.md](ttl_decision/recommendation.md), [ttl_decision/test_plan.md](ttl_decision/test_plan.md) | Proposed (nothing applied; `proposed_patch.diff`, `evidence/`) | X + E | default TTL wiring |
| [m3_durability/design.md](m3_durability/design.md), [m3_durability/reproduction_report.md](m3_durability/reproduction_report.md), [m3_durability/test_plan.md](m3_durability/test_plan.md) | Proposed (nothing integrated; seven `*.diff` files and `evidence/`) | X + E | invalidation durability; defect reproduced |

## 4. Architecture and code findings

| File | Status class | Authority | Note |
|---|---|---|---|
| [architecture_investigation.md](architecture_investigation.md) | Historical-frozen (stale on finding A1) | R | what the binary provided at HEAD 456bb25 |
| [architecture_trace_m3.md](architecture_trace_m3.md) | Historical-frozen (snapshot labelled authoritative; predates later hardening) | R | code-level trace |
| [code_findings.md](code_findings.md) | Historical-frozen (2026-10-28 tree) | R/E | known gaps; later fixes not reflected per finding |
| [pie_audit.md](pie_audit.md) | Historical-frozen | R/A | legacy-PIE facts |
| [rfc9111_supported_subset.md](rfc9111_supported_subset.md) | Active (self-declared verified; no independent check cited here) | A | freshness scope |
| [telemetry_schema_v2.md](telemetry_schema_v2.md) | Active (self-declared "implemented and verified"; no verification date) | A | telemetry and provenance contract |
| [refpolicy_golden_traces.md](refpolicy_golden_traces.md) | Unclassified | E/spec | reference-policy traces |
| [eviction_policy_audit.md](eviction_policy_audit.md) | Historical-frozen (marked complete) | R/E | |
| [ui_metric_provenance.md](ui_metric_provenance.md) | Superseded in part (banner; field catalog by the phase 1 package) | R | |

## 5. Intelligence and prior art

| File | Status class | Authority | Note |
|---|---|---|---|
| [phantom_intelligence_phase.md](phantom_intelligence_phase.md) | Proposed (refined by the phase 5 package) | P | M5 proposal |
| [intelligence_architecture.md](intelligence_architecture.md) | Proposed (labelled authoritative; caveat of claim C8 absent) | P/A | machinery is outside the serving path |
| [intelligence_e2_headroom_protocol_DRAFT.md](intelligence_e2_headroom_protocol_DRAFT.md) | Draft (not frozen, not registered) | X | next protocol; no salt or seeds exist |
| [intelligence_prior_art_coverage.md](intelligence_prior_art_coverage.md) | Active (RT-0 not closed) | A | prior-art verification |
| [novelty_snapshot_m6.md](novelty_snapshot_m6.md) | Historical-frozen (appended correction: RT-0 verdict not supported) | R | |
| [literature_review.md](literature_review.md) | Active (partial; sweep incomplete) | A | verified citations |
| [candidate_comparison.md](candidate_comparison.md) | Unclassified (header: provisional; has an authoritative post-verification section) | R/P | |
| [vision/00_consolidated_vision_and_feasibility_report.md](vision/00_consolidated_vision_and_feasibility_report.md) and `vision/v1` to `v5` ([v1](vision/v1_intelligence_opportunity_map.md), [v2](vision/v2_forecasting_and_evaluation.md), [v3](vision/v3_topology_tenancy_identity.md), [v4](vision/v4_security_and_failure_scenarios.md), [v5](vision/v5_demo_center_ux_and_storyboard.md)) | Proposed (research only; not a plan of record) | X | about 450 KB; owner decisions D-01 to D-40 listed in the consolidated report |

## 6. Experiments (protocol chain, M6, benchmarks)

The effective M6 protocol is read from v2, v2.1, v2.2 and addendum 1 together (D2 section 6); v1 is voided by v2 but carries no void marker. Do not edit any file in this section.

| File | Status class | Authority | Note |
|---|---|---|---|
| [hypothesis_and_experiments.md](hypothesis_and_experiments.md) | Proposed/historical (provisional; superseded for H1 by the chain, no pointer) | P | H1 to H4 |
| [preregistration_m6.md](preregistration_m6.md) | Historical-frozen (v1; voided by v2) | R | |
| [preregistration_m6_v2.md](preregistration_m6_v2.md), [preregistration_m6_v2_freeze.json](preregistration_m6_v2_freeze.json) | Historical-frozen (v2 supersedes v1; freeze record invalidated by v2.1/v2.2) | R | |
| [preregistration_m6_v2_1_amendment.md](preregistration_m6_v2_1_amendment.md), [preregistration_m6_v2_1_freeze.json](preregistration_m6_v2_1_freeze.json) | Historical-frozen (superseded by v2.2 records) | R | |
| [preregistration_m6_v2_2_amendment.md](preregistration_m6_v2_2_amendment.md), [preregistration_m6_v2_2_freeze.json](preregistration_m6_v2_2_freeze.json) | Historical-frozen (protocol of the sealed run) | R | |
| [preregistration_m6_v2_2_addendum_1.md](preregistration_m6_v2_2_addendum_1.md), [.json](preregistration_m6_v2_2_addendum_1.json), [.pin](preregistration_m6_v2_2_addendum_1.pin) | Historical-frozen (post-run disclosure) | R | |
| [m6_sealed_results.md](m6_sealed_results.md) | Historical-frozen (sealed result: H1 NOT_SUPPORTED, tier SIMULATED) | R | artifacts in [../../lab-results/README.md](../../lab-results/README.md) |
| [m6_research_evaluation.md](m6_research_evaluation.md) | Superseded (RETRACTED banner) | R | retained as evidence |
| [claims_ledger.md](claims_ledger.md) | Active, append-only history (table rows C1/C2/C7 predate its section 6) | A + R | one status per claim id |
| [baseline_and_profiling_plan.md](baseline_and_profiling_plan.md) | Proposed (environment facts dated 2026-10-28) | P/E | |
| [benchmark_suite_spec.md](benchmark_suite_spec.md) | Proposed, superseded in part (banner: phase 1 package 6.3 wins for phase 1) | P | no benchmark results document exists |
| [demo_experience_plan.md](demo_experience_plan.md) | Proposed | P | demo plan |

## 7. Documentation audit

| File | Status class | Authority | Note |
|---|---|---|---|
| [doc_audit/00_consolidated_documentation_audit_and_plan.md](doc_audit/00_consolidated_documentation_audit_and_plan.md) | Proposed (header: nothing executed at writing time) | X | authoritative-source map, standard, owner decisions B-1 to B-10 |
| [doc_audit/d1_milestone_documentation_audit.md](doc_audit/d1_milestone_documentation_audit.md), [d2_research_docs_audit.md](doc_audit/d2_research_docs_audit.md), [d3_other_docs_audit.md](doc_audit/d3_other_docs_audit.md), [d4_mechanical_checks.md](doc_audit/d4_mechanical_checks.md) | Historical-frozen (audit snapshots of 2026-10-30; read-only audits) | R/E | limits: header and grep reads, heuristic path detection |

## 8. Known problems with this folder (from D2)

No document here is the single status authority (use [../STATUS.md](../STATUS.md)); the ledger sections A to C and the claims table are stale against their own later sections; the staleness bound appears as 50 ms, 250 ms and 60 s in different places (see claim C6 and the ADR-003 amendment). The consolidated plan lists owner decisions that are still open. Reorganizing this folder into subfolders was not proposed because it would break existing references.
