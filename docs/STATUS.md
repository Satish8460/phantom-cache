# PhantomCache status (entry point)

**Status:** Active
**Audience:** the product owner, reviewers and any agent that needs the current state of the project
**Last reviewed:** 2026-10-30, about 15:30 (+05:30 machine clock; refreshed by the DOCS agent after UI increments I1 to I4; earlier text of 12:06 is superseded where it differs)
**Supersedes/Superseded by:** none. It does not replace any record; it points to the record that is authoritative for each question.
**Evidence:** every row below links to the document it is taken from. Evidence tags follow `CLAUDE.md` section 4.

## Authority

This page is the entry point for "where does the project stand". It is authoritative only for the compact tables below and for pointing to the right record. It does not restate history or verdicts.

| Question | Authoritative record |
|---|---|
| What happened, when, with which commands and hashes | [`research/progress_ledger.md`](research/progress_ledger.md) (append-only event history) |
| Status of a research claim | [`research/claims_ledger.md`](research/claims_ledger.md) (see its dated reconciliation section) |
| Delivery gate decisions | [`research/g1_verdict.md`](research/g1_verdict.md), [`research/g2_verdict.md`](research/g2_verdict.md); a G3 verdict is not yet written |
| Per-workstream completion (one row each, with independence class) | [`release/workstream_completion_ledger.md`](release/workstream_completion_ledger.md) |
| What was verified, by whom, with which command, and whether it is stale | [`release/verification_register.md`](release/verification_register.md) |
| Task queue, repository protection, blockers (lead-owned) | [`release/execution_register.md`](release/execution_register.md) |
| Configuration keys and defaults | [`user/configuration.md`](user/configuration.md), checked against `src/config.rs` |
| Architectural decisions | the ADRs in [`research/adr/`](research/adr/) (an ADR index is part of the documentation batch; ADR "ACCEPTED" lines were recorded by agents, see below) |
| Per-milestone detail and original objectives | [`research/phantomcache_milestone_accountability_report.md`](research/phantomcache_milestone_accountability_report.md) and the documentation audit [`research/doc_audit/d1_milestone_documentation_audit.md`](research/doc_audit/d1_milestone_documentation_audit.md) |

## 1. Milestones M1 to M8

Four separate questions per milestone. Values: **Yes**, **Partial**, **No**, **Not recorded**. Values are my reading of the cited retained evidence as of 2026-10-30, not a new verification; where the evidence is mixed the value is Partial.

- **Implemented**: the deliverables exist in the repository.
- **Tested**: automated tests exist and passed in a run recorded in the ledger (debug profile; passing tests do not establish correctness).
- **Independently verified**: someone other than the author checked retained evidence or code (reading-only reviews count as Partial).
- **Formally accepted**: a dated owner decision accepting the milestone is recorded in the repository. **Not recorded for any milestone. This is an evidence gap, not proof that no authorization or acceptance was given**; the owner may hold records outside the repository (documentation-audit decision B-8). The "PASS" lines in the M1 to M5 completion reports are agent self-reports and are not carried forward here.

Acceptance-criteria (AC) source for every row: the milestone's execution package (`research/phase_N_execution_package.md`, AC lists as tabulated in the D1 matrix). M8 is defined only in [`research/phase_execution_packages_index.md`](research/phase_execution_packages_index.md) and [`research/phase_8_execution_package.md`](research/phase_8_execution_package.md); `research/mvp_and_milestones.md` has M1 to M7 only.

| M | Implemented | Tested | Independently verified | Formally accepted | Evidence and specific contradictions |
|---|---|---|---|---|---|
| M1 Truth and baseline | Partial | Yes | Partial | Not recorded | Completion report says PASS (263 tests, agent-reported); supported status is "partially implemented, AC not demonstrated". Telemetry honesty and byte-accounting tests have later independent evidence; benchmarks were never re-run on the current tree. [accountability section 2](research/phantomcache_milestone_accountability_report.md), [D1 section 2](research/doc_audit/d1_milestone_documentation_audit.md) |
| M2 Core correctness | Partial | Yes | Partial (code-reading reviews) | Not recorded | `PHANTOM_DEFAULT_TTL_MS` is not wired (literal 60 s default; [`user/limitations.md`](user/limitations.md)). Completion report AC2-6 cites a "200x" speedup that claims ledger C5 records as untraceable. 2 h soak not run. |
| M3 Distributed | Partial | Yes (loopback, debug) | Partial | Not recorded | Completion report says "zero compromises"; the invalidation-durability defect was reproduced 2026-10-30 08:26 ([accountability section 12](research/phantomcache_milestone_accountability_report.md)). Claim C6 "zero stale after 50 ms" is not supported ([claims ledger](research/claims_ledger.md)). ADR-003 "ACCEPTED" was recorded by agents, no owner ratification recorded. **State after 08:26:** defect reproduced on unmodified code and a patch validated in scratch only; the patch is NOT applied to the tree; sandbox remediation (register E2) started and is reported RUNNING; protected-core integration needs independent review and owner final approval ([M3 evidence](research/m3_durability/reproduction_report.md)). |
| M4 Lab and baselines | Partial | Yes | Partial | Not recorded | Completion report AC4-9/AC4-10 are worded differently from the package and cite "r_s = 1.0"; no T1/T2 agreement study was run (claim C7 unverified). Scenario B (G1) and the Lab API (G2) have later independent review. |
| M5 Intelligence and PIE | Partial (machinery only) | Yes (unit level) | No | Not recorded | Completion report AC5-13 says the PIE engine is "constructed in `main()`"; `src/main.rs:433` has `pie: None`, and `PieDecisionEngine` is referenced only in `src/intelligence/engine.rs` (verified by me by grep and by opening the line). Claim C8 unverified. Gate G-RESEARCH is not recorded. |
| M6 Research evaluation | Yes | Yes | Partial | Not recorded | Sealed run executed once: H1 NOT_SUPPORTED, Tier 1 simulation ([`research/m6_sealed_results.md`](research/m6_sealed_results.md)). Integrity chain checked by the lead. G-MVP not met; RT-0 prior-art sweep not closed. The M6 completion report is retracted. |
| M7 Productization | Partial | Partial | Partial (two reading reviews) | Not recorded | No M7 progress or completion record exists. SEC-M7-09/10/11 open; no advisory scan, no graceful drain, no clean-machine reproduction; G-RELEASE not attempted ([`security/m7_security_review.md`](security/m7_security_review.md)). |
| M8 UI/UX | Partial (increments I1 to I4 IMPLEMENTED) | Yes (298 UI tests; browser rounds exploratory) | Partial (AI second looks only) | Not recorded | **Since the previous version:** remediation increments I1 foundations, I2/I2b Phantom Lab, I3 Simulation, I4 shell and pages are implemented and tested; lead re-run 298/298 UI tests, contrast 196/196, a11y lint 74 combinations 0 issues, Vite build ok (14:26); this refresh re-ran tests, contrast and lint (see section 3). Evidence: three browser rounds ([`demo/browser_verification/`](demo/browser_verification/), taken BEFORE I1 to I4), the after-gallery and comparison ([`05`](design/remediation/05_before_after_comparison.md)), an AI usability walkthrough ([`04`](design/remediation/04_validation_usability.md), not human data). **Visual quality is NOT approved; O8-2 is OPEN**; the accessibility track (Track 2) is still running and its report does not exist yet. Not verified: human usability, screen readers, real zoom, release bundle. Stages A to F were derived by an agent (owner decision O8-0 unresolved). Progress: [`design/remediation/00_progress.md`](design/remediation/00_progress.md) (last reviewed 12:50, behind the ledger). |

## 2. Delivery gates G1 to G5

These are the 2026-10-30 delivery gates, not the milestone gates (G-M1...). Source: [`research/project_status_and_accountability_2026-10-30.md`](research/project_status_and_accountability_2026-10-30.md) (snapshot of 11:50), updated with the later records named in each row. AI second looks are not human review.

| Gate | Status | Record |
|---|---|---|
| G1 Scenario B | PASS with dispositions D1 to D5 | [`research/g1_verdict.md`](research/g1_verdict.md) |
| G2 Lab API | PASS with dispositions G2-D1 to G2-D7 | [`research/g2_verdict.md`](research/g2_verdict.md) |
| G3 UI plus browser verification | **Verdict not yet recorded; remediation ACTIVE.** Open high findings: U-01 (a cancelled run still shows a verdict-like "Result at a glance"; backend half at `src/lab_live/scenario_b.rs` needs separate authorization), U-02 (no UI control for the gateway address; default is 8085), U-03 (progress "0 of 2 units" with an empty bar looks stalled); from the AI walkthrough [`04`](design/remediation/04_validation_usability.md), with 11 medium/low items U-04 to U-14. Browser rounds 1 to 3 ([`1`](demo/browser_verification/20261030T041006Z/report.md), [`2`](demo/browser_verification/20261030T045605Z/report.md), [`3`](demo/browser_verification/20261030T055008Z/report.md)) predate I1 to I4; their open low findings (N-6 Overview overflow at 320 px, N-7, residual target sizes) have NOT been re-verified on the current UI. Accessibility track (Track 2) running. Visual quality not approved; O8-2 open. | [`design/remediation/00_progress.md`](design/remediation/00_progress.md), [`05`](design/remediation/05_before_after_comparison.md) |
| G4 two whole-product review cycles | **Not started on the UI** (waits for the UI to stabilise). Rust Cycle 1 (three read-only AI reviewers) is recorded as running in the register; no report exists yet (`docs/release/g4/` is absent at this refresh). | [`release/execution_register.md`](release/execution_register.md) N1 |
| G5 commits and push | **Preparation started, nothing committed.** Classification of the working-tree entries, commit plan and scan procedure are being drafted (read-only); a backup exists. No commit or push has been authorized for an unreviewed group. | [`release/execution_register.md`](release/execution_register.md) (repository protection, N2) |

## 3. Test baseline

| Command | Profile | Result | Date | Log pointer |
|---|---|---|---|---|
| `cargo test --tests -j 1 --no-fail-fast` | debug | 75 targets, **851 passed, 0 failed, 8 ignored** | 2026-10-30, 10:11 to 10:23 | ledger entry 2026-10-30 10:24 (console logs were kept outside the repository and are not retained; decision B-10). **Unchanged since:** no file under `src/`, `tests/`, `benches/` is newer than 10:23:53, and `Cargo.toml`/`Cargo.lock` are older (checked by file modification time on 2026-10-30 about 15:30; mtime is not a content hash). Rust sandbox work (M3 and others) lives outside the tree. |
| `cargo clippy --all-targets -j 1 -- -D warnings` | debug | exit 0 | 2026-10-30, same sweep | same ledger entry |
| `node --test test` (in `ui/`) | Node test runner | **298 passed, 0 failed** (lead 14:26; re-run by the DOCS agent about 15:29, same result, 2.1 s) | 2026-10-30 | [`release/verification_register.md`](release/verification_register.md) V-07. The earlier "212" is obsolete. |
| `check-contrast`, `check-a11y`, `check-forbidden` (ui scripts) | Node scripts | contrast 196/196; a11y lint 74 combinations, 0 issues (static checks only); forbidden-constant scan passed | 2026-10-30, re-run about 15:29 | same register, V-08 to V-10 |
| `vite build` | production | ok (lead, 14:26); not re-run by this refresh | 2026-10-30 | ledger entry 14:26 |
| Targeted release run, 10 suites | release | 209 passed, 0 failed, 6 ignored | 2026-10-30, 07:29 to 07:34 | [`research/g1_verdict.md`](research/g1_verdict.md) |

Older counts (242, 263, 388, 499, 774, 802, 841 and others) are historical, were taken at different dates and scopes, and are debug-profile. They are not comparable with the table above, and `CLAUDE.md` and `README.md` still quote 242. **There is no release-profile general suite yet.** Passing tests do not establish correctness or visual quality.

## 3b. Repository state and recent findings

- Repository (read-only checks, about 15:29): HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, no stash. `git status --short` shows 93 lines because it collapses untracked directories; `git status --short -uall` showed 4,451 entries (36 tracked modified, 4,415 untracked) at this check, against 4,435 recorded by the lead earlier (36 + 4,399); the count is still moving as agents write evidence. **Nothing is committed.** A backup of the tree exists (path and manifest in the register). Most untracked entries are generated or evidence files.
- **DR-029** (cluster `GetRequest` handler fabricates `val_node_<id>` into the node cache): narrowed finding. A debug loopback reproduction (investigator) and an AI second look agree that no HTTP request can reach the fabricated entry (disjoint key spaces). Remaining: a design defect, **Low as exploited today, Medium as a latent hazard**; needs a valid cluster certificate. Not independently verified by a human; no fix applied. The lead's earlier "HTTP clients may be served the value" statement is withdrawn. See [`security/investigations/`](security/investigations/). **DR-027** (any authenticated member can send tenant or full-cache scope invalidations; global-epoch fallback) is the larger insider capability; no fix applied.
- **Protected core:** Rust remediation (M3 durability, DR-029/027, SEC-M7-09/10/11, quotas, intelligence tap) is being developed only in sandbox copies; nothing is merged into the tree without independent review and owner final approval. Several of those items are QUEUED behind M3 in the register.

## 4. What is real and what is simulated

Read [`user/limitations.md`](user/limitations.md) first. In short: numbers in the repository are debug-build, loopback, single-host or simulated; none is a release-grade measurement; adaptive benefit is not established; GET is served from the local cache only; cluster mode is experimental and loopback-tested. Sealed M6 (Tier 1, SIMULATED) found H1 NOT_SUPPORTED. For Scenario B, [`demo/scenario_b_interpretation_notice.md`](demo/scenario_b_interpretation_notice.md) is binding and must accompany any citation. It states: "Not established: adaptive-control merit; production, WAN or multi-host behaviour; origin-p99 benefits; stale-serving effectiveness; pre-registered cap selection; a served-client latency improvement." This does not show that intelligent caching is not viable; it shows that this controller, on these plants, gave no benefit over a static cap.

## 5. Open owner decisions

- Vision decisions D-01 to D-40: [`research/vision/00_consolidated_vision_and_feasibility_report.md`](research/vision/00_consolidated_vision_and_feasibility_report.md) (section 13).
- Documentation decisions B-1 to B-10: [`research/doc_audit/00_consolidated_documentation_audit_and_plan.md`](research/doc_audit/00_consolidated_documentation_audit_and_plan.md) (section 9), including whether and where milestone authorizations and acceptances were given (B-2, B-8).
- Product-direction decisions OD-1 to OD-50: [`research/vision/phantomcache_three_pillar_product_direction.md`](research/vision/phantomcache_three_pillar_product_direction.md). Cross-project discrepancies: [`research/vision/discrepancy_register.md`](research/vision/discrepancy_register.md).
- UI/UX: O8-2 (final acceptance, OPEN) and O8-0 (derivation of stages A to F, unresolved); declaring the UI implementation phase ready (the UI is read-only until then); authorizing the backend half of U-01 and telemetry authentication under JWT (BD-05); whether the lead's review replaces the AG review gate (superseded by the 14:56 directive, per the ledger).
- Rust and repository: M3 durability design choices (fail-open versus fail-closed on log errors, default fsync policy, drain deadline, debt age-out), the DR-029 fix option, owner final approval for every protected-core merge, the v3 evaluation salt (offline), frozen-file changes (addendum-2 batch), and the commit policy (the latest instruction asks for controlled commits, the previous one for none before G4/G5; reconciliation: commit only reviewed groups; see the register).
- Toolkit: a research agent flagged hooks and a downloaded binary in a third-party design skill as a security concern; nothing was installed; any installation needs an owner decision (see `research/design_toolkit/`).

## 5b. Delivery risks

- **Everything is uncommitted.** HEAD is an older snapshot; the work exists only in the working tree and in one local backup on the same machine (not an off-machine copy).
- **No human review has happened.** Every "independent" review so far is an AI second look or a lead re-run; no human design, usability, accessibility or security review is recorded. Authors and reviewers share one tooling environment.
- **Frozen-file merges need owner approval.** Several remediation items touch frozen or protected files (`src/core/cache.rs`, router, main, config, lab); they stay in sandbox copies until reviewed and approved.
- **Evidence staleness.** Browser rounds predate the UI remediation; the Rust sweep (10:23) matches the current tree by file times but does not cover sandbox work; deleted debug binaries cannot be re-verified (storage cleanup).
- **Moving counts.** The working-tree entry count and some register timestamps change as agents write; numbers here are dated snapshots.

## 6. Where to find things

- Event history: [`research/progress_ledger.md`](research/progress_ledger.md)
- Workstream ledger, verification register, execution register: [`release/`](release/)
- UI/UX remediation: [`design/remediation/`](design/remediation/); design-toolkit research: [`research/design_toolkit/`](research/design_toolkit/)
- Milestone detail: [`research/phantomcache_milestone_accountability_report.md`](research/phantomcache_milestone_accountability_report.md), [`research/master_audit_and_execution_plan.md`](research/master_audit_and_execution_plan.md)
- Full status and pending-work review: [`research/project_status_and_accountability_2026-10-30.md`](research/project_status_and_accountability_2026-10-30.md)
- Documentation audit and plan: [`research/doc_audit/`](research/doc_audit/)
- Claims: [`research/claims_ledger.md`](research/claims_ledger.md)
- Execution packages and phase index: [`research/phase_execution_packages_index.md`](research/phase_execution_packages_index.md)
- Sealed M6 results: [`research/m6_sealed_results.md`](research/m6_sealed_results.md)
- Storage cleanup record: [`research/storage_cleanup_2026-10-30.md`](research/storage_cleanup_2026-10-30.md)
- User documentation (install, run, configure, operate, limitations, Lab API): [`user/`](user/)
- Security register and reviews: [`security/`](security/)
- Demo and Scenario B material: [`demo/`](demo/)
- Rules for agents: `CLAUDE.md` at the repository root
