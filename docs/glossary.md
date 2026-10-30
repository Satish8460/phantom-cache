# Glossary and ID-scheme map

**Status:** Active (reference)
**Audience:** readers of the research, security, handoff and demo documents, who meet several numbering schemes that reuse the same words and letters
**Last reviewed:** 2026-10-30
**Authority and related:** derived from [research/doc_audit/d1_milestone_documentation_audit.md](research/doc_audit/d1_milestone_documentation_audit.md) (section 1.2) and checked against the cited sources below. Where this page and a source disagree, the source wins; report the difference. Current status is in [STATUS.md](STATUS.md), not here.

Each entry names a source. "Verified in code" means the code line was opened for this page; other entries are as stated by the cited document.

## 1. Numbering schemes (the same word means different things)

| Scheme | Unit names | Where it lives | Do not confuse with |
|---|---|---|---|
| A. Canonical roadmap | Milestone M1 to M8 = "Phase 1 to 8" (the phase index says "Phase N = Milestone N"); steps `M<N>-xx`; work packages `WP-00` to `WP-37`; research tasks `RT-0` to `RT-6`; owner decisions `O<N>-x`; gates `G-M1` to `G-M5`, `G-ARCH`, `G-DIST`, `G-RESEARCH`, `G-MVP`, `G-RELEASE` | [research/mvp_and_milestones.md](research/mvp_and_milestones.md) (M1 to M7), [research/phase_execution_packages_index.md](research/phase_execution_packages_index.md) (adds M8), `research/phase_N_execution_package.md` | schemes B to H below |
| B. Course work packages | `WP1` to `WP5` (no hyphen) | [implementation/WP1.md](implementation/WP1.md) to `WP5.md`, [PHASE_2_WORK_REPORT.md](PHASE_2_WORK_REPORT.md); test files `wp1_tests.rs` to `wp4_tests.rs` ([../tests/unit/](../tests/unit/wp1_tests.rs)) | `WP-01` (scheme A) |
| C. Early "Milestone 1" | HTTP integration milestone; tests `M1-T01...`; [../tests/integration/m1_http_tests.rs](../tests/integration/m1_http_tests.rs) | [../MILESTONE_1_DESIGN_PLAN.md](../MILESTONE_1_DESIGN_PLAN.md), [milestone_1/WORK_NOTES.md](milestone_1/WORK_NOTES.md) | canonical M1 (Truth and Baseline) |
| D. Handoff packages and stages | `P1` to `P4`, Phases 3A to 3C, 4A to 4C, Stage 4B/4C | [handoff/02_PHASE_HISTORY_AND_STATUS.md](handoff/02_PHASE_HISTORY_AND_STATUS.md), [security/stage-4c-progress.md](security/stage-4c-progress.md) | "Phase 4" here is a security/distributed stage, not M4; the handoff's "Phase 4 accepted" is not supported (see [../CLAUDE.md](../CLAUDE.md) section 6) |
| E. Security roadmap | Phase 1 to 5 (text) / 1 to 6 (diagram, README) | [security/implementation-roadmap.md](security/implementation-roadmap.md), `security/phase-N-*.md` | scheme A phases |
| F. Old roadmap | proposed "Phase 5 to 9" | [handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md](handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md) (superseded note at top) | scheme A |
| G. Delivery gates | `G1` to `G5` | section 3 below | `G-M1...` (scheme A) |
| H. Finding and remediation IDs | `R-nn`, `SEC-M7-nn`, `SEC-4A-01`, `SEC-4C-01..06`, `S-1..S-9`, `KG-1..6`, `C-01..C-08`, `CORR-Mx-nn`, `BD-nn`, `F-nn` | section 4 below | claim ids `C1..C13` |

Former numbering: roadmap Rev 1 had six milestones; Rev 2 inserted M5, so a pre-2026-10-29 "Milestone 5/6" may mean today's M6/M7 ([research/doc_audit/d1_milestone_documentation_audit.md](research/doc_audit/d1_milestone_documentation_audit.md) section 1.3).

## 2. Milestones, phases, work packages

- **M1 to M8**: M1 Truth and Baseline; M2 Core Correctness, Performance and Freshness; M3 Distributed Correctness and Architecture Gate; M4 Phantom Lab and Static Baselines; M5 Phantom Intelligence and PIE; M6 Research Evaluation and MVP Gate; M7 Productization and Research Release; M8 UI/UX Excellence. M8 appears in the phase index but not in the roadmap file (`mvp_and_milestones.md` stops at M7). Sources: [research/mvp_and_milestones.md](research/mvp_and_milestones.md), [research/phase_execution_packages_index.md](research/phase_execution_packages_index.md). Whether a milestone passed is a status question: see [STATUS.md](STATUS.md) and [research/phantomcache_milestone_accountability_report.md](research/phantomcache_milestone_accountability_report.md).
- **Phase N (research folder)**: equals Milestone N in scheme A. Packages are headed "PROPOSED", yet phases 1 to 6 were executed; no owner authorization record exists in the repository (evidence gap, not proof of absence): [research/doc_audit/00_consolidated_documentation_audit_and_plan.md](research/doc_audit/00_consolidated_documentation_audit_and_plan.md) section 2.
- **WP-NN versus WP1..WP5**: see scheme table above. Files: [research/work_packages.md](research/work_packages.md) (scheme A) versus `implementation/WP*.md` (scheme B).

## 3. Gates

- **G-M1 to G-M5, G-ARCH, G-DIST, G-RESEARCH, G-MVP, G-RELEASE**: roadmap gates, defined in [research/mvp_and_milestones.md](research/mvp_and_milestones.md) (exit-gate row). `G-ARCH` is the distributed-architecture decision recorded in [research/adr/ADR-003-distributed-architecture.md](research/adr/ADR-003-distributed-architecture.md). An owner record for `G-RESEARCH` was not found.
- **G1 to G5 (delivery gates, 2026-10-30)**: G1 Scenario B ([research/g1_verdict.md](research/g1_verdict.md)), G2 Lab API ([research/g2_verdict.md](research/g2_verdict.md)), G3 UI plus browser verification, G4 review cycles, G5 delivery (commit plan). Definition table: [research/phantomcache_milestone_accountability_report.md](research/phantomcache_milestone_accountability_report.md) section 5 (written when G1 was in progress; later outcomes are in the verdicts and [STATUS.md](STATUS.md)). These are not milestones.
- **Gate B**, **U1/U2**, **D-n** (owner decisions such as D-7 browser verification, D-8 release gate): used in [research/demo_experience_plan.md](research/demo_experience_plan.md) and the phase index. `D-8` has two meanings in different documents (D1 audit section 1.2).

## 4. IDs for findings, claims and risks

- **R-01 has three meanings.** (1) Master plan remediation package "Wire Origin Cap in server.rs" ([research/master_audit_and_execution_plan.md](research/master_audit_and_execution_plan.md) line 94). (2) The ledger's "R-01 origin cap" entry ([research/progress_ledger.md](research/progress_ledger.md)). (3) Accountability register item R-01, "dated addendum for post-sealed-run additive changes" ([research/phantomcache_milestone_accountability_report.md](research/phantomcache_milestone_accountability_report.md) line 169). Always cite the document with the id.
- **C1..C13 (claim ids)**: research claims with a status each, in [research/claims_ledger.md](research/claims_ledger.md). Not the same as `C-01..C-08` (known-gap corrections in [../CORRECTIONS.md](../CORRECTIONS.md) / [handoff/CORRECTIONS.md](handoff/CORRECTIONS.md)). Status vocabulary: HYPOTHESIS, IMPLEMENTED, EVALUATED, SUPPORTED, NARROWED, REFUTED (header of the ledger), with wording drift noted in the audit. The table rows for C1, C2 and C7 predate the ledger's section 6, which governs.
- **SEC-M7-nn**: M7 security review findings 01 to 11, register in [security/m7_security_review.md](security/m7_security_review.md). **SEC-4A-01** (tenant epoch overflow falls back to the global epoch) and **SEC-4C-01..06**: security stage findings ([../CLAUDE.md](../CLAUDE.md) section 5; [handoff/02_PHASE_HISTORY_AND_STATUS.md](handoff/02_PHASE_HISTORY_AND_STATUS.md)).
- **KG-1..6 / C-01..C-08 / CORR-Mx-nn**: known architecture gaps and their corrections ([../CORRECTIONS.md](../CORRECTIONS.md); a longer, different file exists at [handoff/CORRECTIONS.md](handoff/CORRECTIONS.md); both are retained as they are).
- **I1..I8, INV-4, I5**: byte-accounting and no-eviction-with-active-ops invariants ([../CLAUDE.md](../CLAUDE.md) section 5; tests [../tests/integration/byte_accounting_tests.rs](../tests/integration/byte_accounting_tests.rs)).
- **RT-0 to RT-6**: research tasks; RT-0 is prior-art closure and is not closed ([research/intelligence_prior_art_coverage.md](research/intelligence_prior_art_coverage.md)).
- **H1 to H6**: experiment hypotheses ([research/hypothesis_and_experiments.md](research/hypothesis_and_experiments.md), [research/preregistration_m6_v2_2_amendment.md](research/preregistration_m6_v2_2_amendment.md)). The sealed M6 result for H1 is NOT_SUPPORTED ([research/m6_sealed_results.md](research/m6_sealed_results.md)).
- **WP and step ids inside tests** (`CT-1..CT-10`, serving-path contract clauses) are defined in [research/adr/ADR-002-serving-path-contract.md](research/adr/ADR-002-serving-path-contract.md).

## 5. Evidence tags, tiers and labels

- **Evidence tags** (claims in prose): verified in code (cite `file:line`), reproduced here, reported by a prior agent or handoff, assumption, hypothesis, literature. Defined in [../CLAUDE.md](../CLAUDE.md) section 4. Individual documents abbreviate them (for example `[V]`, `[A]`, `[L]`, `[H]`); each document defines its own letters in its header, and the letters differ between documents.
- **Experiment tiers T1, T2, T3**: T1 = simulation (deterministic virtual-time model, may drive real code), T2 = real single-host (loopback), T3 = physical multi-host. Sources: evidence-class line in [research/claims_ledger.md](research/claims_ledger.md), [research/hypothesis_and_experiments.md](research/hypothesis_and_experiments.md) (Tier 3), and the mapping in [design/lab_api_contract.md](design/lab_api_contract.md) (`SIMULATED` = T1, `MEASURED_LOOPBACK` = T2). Never mix tiers in one number.
- **Lab and UI provenance labels**: Lab API values carry `SIMULATED`, `MEASURED_LOOPBACK`, `RETAINED_*` (read from a retained artifact), `DERIVED` or `UNAVAILABLE` ([design/lab_api_contract.md](design/lab_api_contract.md), vocabulary mapping in [design/phantom_lab_ui.md](design/phantom_lab_ui.md)). The user-facing and project-rule vocabulary is measured, simulated, inferred, unavailable ([user/README.md](user/README.md), [../CLAUDE.md](../CLAUDE.md) section 4). `INFERRED` was not found as an identifier in `src/`, `ui/src/`, the design docs or the telemetry schema (search of 2026-10-30); treat it as a documentation label. Missing is never shown as zero.
- **Scenario B label `VALID`**: means only release binaries (path heuristic), at least 10 repetitions, BASE present, one host, scripted origin ([demo/scenario_b_interpretation_notice.md](demo/scenario_b_interpretation_notice.md) item 8). It is not a statement about adaptive-control merit.
- **TRADE_OFF_FAILURE**: mechanical Scenario B verdict (defined by the contract in [demo/scenario_B_origin_slowdown.md](demo/scenario_B_origin_slowdown.md); independent recomputation in [demo/scenario_b_verification/README.md](demo/scenario_b_verification/README.md)); read it with the interpretation notice.

## 6. Modes

Names verified in [../src/config.rs](../src/config.rs) and [../src/intelligence/mod.rs](../src/intelligence/mod.rs):

- **`PHANTOM_MODE`**: `development` (default) or `production` (config.rs line 96; CLI `--mode`). There is no mode named "secure". Production mode adds startup requirements (config.rs around line 940). Documentation: [user/configuration.md](user/configuration.md), [user/security.md](user/security.md).
- **`PHANTOM_INTELLIGENCE`**: `off` (default), `observe`, `advisory`, `canary`, `active` (config.rs line 161; enum `IntelligenceMode` in `src/intelligence/mod.rs` line 24). The canary mode exposes a bounded share of traffic; all non-`off` modes are experimental and a benefit is not established ([user/limitations.md](user/limitations.md)). The origin concurrency cap is enforced only in `canary` or `active` (config.rs line 878).
- **`PHANTOM_SIMULATION=true`**: makes the dashboard stream synthetic values; they are labeled simulated and are never measurements ([user/limitations.md](user/limitations.md)).
- **Cluster modes**: mTLS cluster (experimental) versus the insecure/partial cluster flag; behavior differences are in [user/limitations.md](user/limitations.md).

## 7. Concepts

- **PIE versus Phantom Intelligence.** Phantom Intelligence is the observe, detect, predict, evaluate, adapt, explain capability and the M5 milestone ([research/phantom_intelligence_phase.md](research/phantom_intelligence_phase.md)); `src/intelligence/` holds its modules. The **PIE** is its Policy Intelligence Engine (the project's chosen expansion going forward: [research/pie_audit.md](research/pie_audit.md) naming note). Other documents expand PIE as "Policy Inference Engine" (handoff) or "Policy Intelligence and Evaluation" (README/PHASE_2 report). `src/control/pie.rs` is a legacy threshold stub, not wired in (`pie: None` in `main.rs`; [../CLAUDE.md](../CLAUDE.md) section 6b); `PieDecisionEngine` in `src/intelligence/engine.rs` is separate machinery that, per claim C8, is not integrated into the serving path ([research/claims_ledger.md](research/claims_ledger.md)).
- **Tenant / namespace.** A `CacheKey` is tenant id + path + optional query (verified in code: `src/core/cache.rs` lines 23 to 27). A principal without a tenant claim uses the shared `__default__` namespace (comment at `src/api/server.rs` line 1047). Each query-string variant is a separate key ([user/limitations.md](user/limitations.md)).
- **Node / peer / gateway.** "Gateway" is the HTTP process. In cluster documents a "node" is a cluster member (`VirtualNode`, `src/dist/node.rs`) and a "peer" is another member seen from one gateway; documents use the two words loosely. The local node can share the gateway's HTTP serving cache in the mTLS cluster path while remote peers are address-only ([../CLAUDE.md](../CLAUDE.md) section 6; re-verify against [../src/main.rs](../src/main.rs) before relying on it).
- **Epoch.** Counters used to invalidate many keys at once: per-tenant epochs, a global epoch, and an epoch snapshot read without an exclusive lock ([../CLAUDE.md](../CLAUDE.md) section 5). Also "registry epochs" for cluster membership ([user/security.md](user/security.md)). Different thing from generation.
- **Generation and fencing.** Each key's state has a generation; an origin fetch records it as `op_gen`, and `finalize_fetch` stores the result only if the generation and epochs still match; `delete` bumps the generation ([../CLAUDE.md](../CLAUDE.md) section 5; contract clauses in [research/adr/ADR-002-serving-path-contract.md](research/adr/ADR-002-serving-path-contract.md)). Tests: [../tests/integration/serving_contract_tests.rs](../tests/integration/serving_contract_tests.rs).
- **Single-flight / coalescing.** Concurrent misses for one key share one origin fetch ([../src/core/coalesce.rs](../src/core/coalesce.rs)).
- **Hard staleness cap.** `PHANTOM_MAX_STALE_MS` bounds how long an expired entry may be served stale; it does not bound the freshness lifetime ([user/limitations.md](user/limitations.md)).
- **Anti-entropy.** Cluster reconciliation of missed invalidations ([research/adr/ADR-003-distributed-architecture.md](research/adr/ADR-003-distributed-architecture.md)); effectiveness not established (claim C6).

## 8. Process words

- **Freeze.** A dated, hash-pinned record of an experiment protocol or its files before a run (`preregistration_m6_v2_*_freeze.json`). A frozen record is never edited.
- **Addendum.** A dated document that discloses changes made to frozen material after the fact ([research/preregistration_m6_v2_2_addendum_1.md](research/preregistration_m6_v2_2_addendum_1.md)).
- **Pin.** A file recording the hash of an addendum so that later verification can detect change ([research/preregistration_m6_v2_2_addendum_1.pin](research/preregistration_m6_v2_2_addendum_1.pin)).
- **Sealed.** Seeds or results that are not to be inspected or reused before the registered run; M6 results: [research/m6_sealed_results.md](research/m6_sealed_results.md). Salt values are never recorded in documents.
- **Amendment / supersession notice / banner.** The only permitted way to correct a historical record: append a dated note and leave the body unchanged ([contributing.md](contributing.md)).
- **Status vocabulary** for document headers: Draft, Proposed, Active, Verified, Superseded, Archived ([contributing.md](contributing.md)).
- **Ledger.** [research/progress_ledger.md](research/progress_ledger.md): chronological event record; its log section is append-only. The claims ledger is separate.
