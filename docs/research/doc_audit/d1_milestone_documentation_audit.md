# D1 — Milestone Documentation Audit (M1–M8)

**Date:** 2026-10-30 (system clock; audit written after ledger entries up to 11:43 +05:30). **Author:** documentation audit agent D1 of 4. **Mode:** read-only on all repository content except this file. No edits, moves, renames, deletions, cargo, git mutations or browsers.
**Repo state at start and end of audit:** HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 72 `git status --short` entries (matches the ledger's 11:43 entry; `docs/research/doc_audit/` was created by this audit, `docs/research/vision/` by another agent).
**Evidence tags:** [V] verified by me by opening the file or running `ls`/`grep` in this audit; [D] document statement only (I did not re-derive it); [H] hypothesis. I did not run any test, so no test result below is reproduced by me; test counts are quoted from the named document.
**Rules observed:** no status is changed; every historical record, frozen preregistration, sealed artifact and append-only ledger is treated as a record, not an error. Where documents disagree I recommend a dated notice/amendment.

---
## 1. How milestones are defined and numbered

### 1.1 Authoritative definition
- **M1–M7: `docs/research/mvp_and_milestones.md` Revision 2 (2026-10-29)** [V]: milestone map §3, specifications §4 (M1 at line 53 … M7 at line 76), gates §5, WP traceability §6, change log §9. Its header (line 3) says "PROPOSED roadmap" and "Nothing in Milestones 2–7 is authorized by this document".
- **M8: defined only in `docs/research/phase_execution_packages_index.md` (line 4, table line 16) and `docs/research/phase_8_execution_package.md`** [V]. `mvp_and_milestones.md` contains no mention of M8 or "Rev 3" (grep, 0 hits) [V]. The index (§5, last bullet) records that the Rev 3 edit adding M8 was "proposed (not applied)"; the accountability report (§1, line 19) confirms it was never applied. So **the canonical roadmap file is seven milestones; the canonical eight-milestone list exists only in the index.**
- **Per-milestone acceptance criteria (AC) live in the execution packages**, not in the roadmap: `phase_1_execution_package.md` (AC-1..12, §7 line 359), `phase_2` AC2-1..14 (§4), `phase_3` AC3-1..9 (§5), `phase_4` AC4-1..12 (§5), `phase_5` AC5-1..14 (§5), `phase_6` AC6-1..12 (§4), `phase_7` AC7-1..14 (§4), `phase_8` AC8-1..13 (§8) [V: ID lists extracted by grep]. Common rules (CER-1..11) are in the index §4.
- **Reconstruction document:** `phantomcache_milestone_accountability_report.md` §1 (lines 17–32) is the only place that tabulates original objective / deliverables / AC / gaps for all eight milestones. It labels itself "agent-compiled, spot-checked by the lead" [D].

### 1.2 Competing numbering schemes (map)
| Scheme | Where | Unit names | Maps to canonical M1..M8 as |
|---|---|---|---|
| **A. Canonical research roadmap** | `mvp_and_milestones.md` (M1–M7), index (M8), `phase_N_execution_package.md` | Milestone / "Phase N" (index line 4: "Phase N ≡ Milestone N"), steps `M<N>-xx`, work packages `WP-00…WP-37`, research tasks `RT-0…6`, gates `G-M1…G-MVP, G-RELEASE`, owner decisions `O<N>-x` | itself |
| **B. Course work packages** | `docs/implementation/WP1.md…WP5.md`, `WP3_GUIDE.md…`, `FLAGSHIP_DEMO_GUIDE.md`; `docs/PHASE_2_WORK_REPORT.md` (course submission "CCA Phase-2") | WP1–WP5 | pre-roadmap background; not milestones. Collides textually with `WP-01…WP-37` of scheme A and with test files `wp1_tests.rs`…`wp4_tests.rs` |
| **C. "Milestone 1" of the early plan** | `MILESTONE_1_DESIGN_PLAN.md`, `docs/milestone_1/WORK_NOTES.md`, tests `M1-T01…T29` in `tests/integration/m1_http_tests.rs` | Milestone 1 = connect cache/coalescer to the Axum server | **NOT canonical M1.** It is a pre-roadmap HTTP-integration milestone whose outputs (m1_http_tests) are a *starting state* for canonical M1. Same name "M1", different content. The accountability report (§1 line 19) calls it "an early, different Milestone 1". |
| **D. Handoff packages / stages** | `docs/handoff/02_PHASE_HISTORY_AND_STATUS.md`: P1–P4, Phases 3A–3C, 4A–4C; `00_START_HERE.md` ("Phase 4 … Formally Accepted", "Do NOT Begin Phase 5"); `docs/security/phase-4-final-acceptance-report.md`; `docs/security/stage-3c/4b/4c-*.md` | Work Package P1..P4, Phase N, Stage nX | **Security/distributed hardening history that precedes the roadmap.** "Phase 4" here is not M4 (Phantom Lab); "Phase 5" here is not M5. Collides with scheme A's "Phase N ≡ Milestone N". |
| **E. Security roadmap** | `docs/security/implementation-roadmap.md` (5 phases per text, "Phase 1…Phase 6" in the diagram and "Phases 1–6" in `README.md` test list), `phase-1-specification.md`, `phase-2-*`, `phase-3-design.md`, `phase-4-design.md` | Phase 1–6 | Another meaning of "Phase N". Substance of its Phases 1–5 shipped before the roadmap (README lines 265–268). Not mapped to M1–M8. |
| **F. Old roadmap "Phase 5–9"** | `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md` (superseded note at top) | Proposed Phase 5–9 | Superseded by A; `mvp_and_milestones.md` line 3 and accountability §9 ("Redundant/obsolete") say so. |
| **G. Delivery gates G1–G5** | accountability §5; `g1_verdict.md`, `g2_verdict.md`; ledger | G1 Scenario B, G2 Lab API, G3 UI + browser, G4 review cycles, G5 delivery | Cross-cutting 2026-10-30 delivery workstream; **not milestones**. Letter collision with scheme A gates `G-M1…` and with owner-decision/work-package "D-4/D-6/D-7/D-8" (demo packages in `demo_experience_plan.md`) versus index "D-8" (release gate U1 decision). |
| **H. Remediation / finding IDs** | master plan R-01..R-03 (remediation WPs), accountability R-01..R-22 (register), ledger R-01 (origin cap), SEC-M7-01..11, S-1..S-9, BD-01..05, F-01..F-04 | — | **`R-01` means three different things**: master plan "Wire Origin Cap in server.rs", ledger "R-01 origin cap", accountability R-01 "dated addendum for post-sealed-run changes". Also `R-07/R-08/R-09` mean rule-gated optimization IDs in M2 (accountability line 24) and register items in §9. |

### 1.3 Milestone titles
Roadmap Rev 2 titles (M1 Truth & Baseline; M2 Core Correctness, Performance & Freshness; M3 Distributed Correctness & Architecture Gate; M4 Phantom Lab & Static Baselines; M5 Phantom Intelligence & PIE; M6 Research Evaluation & MVP Gate; M7 Productization & Research Release) agree across the roadmap, the index and the execution packages [V]. M8 = "UI/UX Excellence, Apple-Inspired Design System & Interaction Quality" (index line 4) [V].
**Title drift:** `phase_1_execution_package.md` line 1 reads "Milestone 1 of 7 toward the MVP" (it predates M8, and its roadmap statement is "of 7" while the index now has 8) [V]. The master plan title is "M1–M6 Independent Audit and M7–M8 Execution Plan" — the first document to speak of M7–M8 as a pair [V].
**Former numbering:** Rev 1 had six milestones; Rev 2 inserted M5, so "old M5" = M6 and "old M6" = M7 (roadmap §9). Documents authored before 2026-10-29 that say "Milestone 5/6" may mean the old numbering; the README and handoff predate the roadmap and do not use M-numbers (grep of README: no M1..M8 references) [V].

### 1.4 Gate vocabulary
G-M1, G-M2, G-M3 (= G-ARCH + G-DIST), G-M4, G-RESEARCH, G-M5 (PI-1..PI-10), G-MVP, G-RELEASE; plus U1/U2 (M8) and D-8 (index). The phase 6 completion report speaks of "Gate G-MVP / G-M6" — **"G-M6" appears nowhere in the roadmap** (the M6 gate is G-MVP) [V, phase_6_completion_report.md line 11 region].

---
## 2. Milestone × criteria matrix

Column key per milestone: **Primary doc**; **Obj** objectives; **Scope/NG** scope and non-goals; **Arch** architecture/technical decisions; **Stages/Deps** implementation stages and dependencies; **AC/exit** acceptance and exit; **Evidence** referenced experiments/tests/evidence and whether paths exist; **Limits** limitations/risks/unresolved; **Stated status** in each document that states one; **Supported status**.
All "exists" marks are [V] by `ls`/`find`. "Primary doc" is the specification; the record of execution is the completion report.

### M1 — Truth & Baseline
- **Primary doc:** `docs/research/phase_1_execution_package.md` (v3.1, 415 lines). Header: "PROPOSED — NOT READY FOR EXECUTION until the owner verifies v3.1 corrections" [V]. Supporting: `phase_1_progress.md`, `phase_1_completion_report.md`, `phase_1_changes.patch`, `phase_1_snapshot_manifest.md`, `docs/handoff/CORRECTIONS.md`, `ag_task_WP-00.md`, `work_packages.md` (WP-00..06; precedence note), `telemetry_schema_v2.md`, `ui_metric_provenance.md`, `benchmark_suite_spec.md`, `baseline_and_profiling_plan.md`. Not this milestone: `MILESTONE_1_DESIGN_PLAN.md` (scheme C).
- **Obj:** "Make the single-node serving path trustworthy and measurable" (accountability line 23, citing package :20; I did not re-open :20).
- **Scope/NG:** package §1; roadmap §4 M1 non-goals: optimizing, distributed behavior, controller, activating PIE. Scope S0–S11 listed in package §5.
- **Arch:** §6 detailed specs (byte-budget semantics §6.5, benchmark plan §6.3, telemetry catalog §6.4, WP-00 checks §6.2); approval items A-1..A-14 §3.
- **Stages/Deps:** S0–S11 (§5); entry none; authorizes M2, M3 via G-M1.
- **AC/exit:** AC-1..AC-12 (§7); G-M1 = owner acceptance after lead review.
- **Evidence:** `bench-results/` (manifests/, logs/, raw/, r0_summary.json, tooling_report.md) exist [V]; `bench-results/logs/audit_all_targets.log` exists [V]; `benches/` exists. `verify_ac1.py` cited in the completion report is not in the repository tree (see §5).
- **Limits:** approval-number drift A-2…A-9 between `phase_1_progress.md` §3 and package §3 (index §5, bullet 3; accountability line 23); no dated approval record; benchmark raw evidence not re-run on current tree.
- **Stated status:** package header PROPOSED; completion report "GATE G-M1 SATISFIED (PASS)", 263 passing, 4 ignored, 22 targets (lines 6–11, 93); progress log; master plan §2 "PASS"; master plan addendum E "AG-reported PASS, spot-reviews pending"; ledger §C "PARTIALLY VERIFIED"; accountability §2 "PARTIALLY IMPLEMENTED … Acceptance criteria satisfied: not demonstrated"; `CLAUDE.md` §3 and `README.md` line 260 "242 tests, 19 targets" baseline; `docs/handoff/00_START_HERE.md` "242/242".
- **Supported status:** accountability §2 / ledger §C (PARTIALLY VERIFIED); completion report PASS is an agent-reported record, not independent verification. Telemetry honesty tests and byte accounting have *later* independent evidence (ledger §C: 15 + 16 tests [L]).

### M2 — Core Correctness, Performance & Freshness
- **Primary doc:** `phase_2_execution_package.md` (144 lines; "PROPOSED — not authorized"). Supporting: `phase_2_progress.md`, `phase_2_completion_report.md`, `phase_2_changes.patch`, `adr/ADR-002-serving-path-contract.md`, `rfc9111_supported_subset.md`, `eviction_policy_audit.md`, `docs/user/configuration.md`, `ttl_decision/` (recommendation, proposed_patch.diff, test_plan, evidence).
- **Obj / Scope / NG:** roadmap §4 M2; package §1. NG: performance work without baselines, adaptive logic.
- **Arch:** ADR-002 clauses; WP-14 freshness engine (`src/core/freshness.rs` exists [V]); faithful eviction (`src/core/eviction.rs` exists [V]).
- **Stages/Deps:** M2-00..14 (§3); entry G-M1; exit G-M2; rule-gated R-07/08/09 thresholds PROVISIONAL.
- **AC/exit:** AC2-1..14 (§4); G-M2.
- **Evidence:** all test files named by the completion report exist [V] (freshness_matrix_tests, stale_serving_tests, eviction_contract_tests, memory_bound_soak_tests, invalidation_contract_tests, concurrency_verification_tests, parser_robustness_tests). No frozen rules file found (accountability line 24). `default_ttl_wiring_tests.rs` named in the ledger (line 234 entry) **does not exist** — it is part of the *proposed* patch in `ttl_decision/` (not applied) [V].
- **Limits:** `PHANTOM_DEFAULT_TTL_MS` NOT WIRED (CLAUDE.md §5; ledger; accountability); soak is 3,000-request debug test vs AC "2 h/platform" (accountability line 47 vs completion report AC2-10 PASS); 200x eviction speedup claim (completion report AC2-6 "B4 speedup 200×") vs claims ledger C5 UNVERIFIED (accountability §6 table: WITHDRAW).
- **Stated status:** completion report "COMPLETED & ACCEPTED (Gate G-M2 PASS)"; master plan §2 "PASS — None. Core data plane is robust and verified"; ledger §C "PARTIALLY VERIFIED"; accountability "COMPLETE-ish in code, TESTED, correctness review ONLY by reading … not demonstrated".
- **Supported status:** accountability/ledger (tested, partially verified).

### M3 — Distributed Correctness & Architecture Gate
- **Primary doc:** `phase_3_execution_package.md` (127 lines; PROPOSED). Supporting: `phase_3_progress.md`, `phase_3_completion_report.md`, `phase_3_changes.patch`, `adr/ADR-003-distributed-architecture.md` (Status: ACCEPTED, line 3), `architecture_trace_m3.md`, `m3_durability/` (design, reproduction_report, test_plan, proposed_patch*.diff, evidence), `docs/security/stage-*`.
- **Obj / Scope / NG:** roadmap §4 M3. NG: "all tests pass" as proof, linearizability claims.
- **Arch:** candidate architectures A/B/C (§3); G-ARCH decision recorded as Option A1 (ADR-003).
- **Stages/Deps:** M3-xx (§4); entry G-M1 (overlaps M2 on disjoint files); exit G-M3 = G-ARCH + G-DIST.
- **AC/exit:** AC3-1..9 (§5). AC3-9 is satisfied only in its "all claims labelled loopback/injected" clause.
- **Evidence:** cluster tests exist [V] (distributed_consistency_tests, cluster_harness_controls_tests, cluster_characterization_tests); `characterization_report.json` is found in the tree [V]. `tests/support/cluster_harness.rs` exists.
- **Limits:** durability gap reproduced 2026-10-30 08:26 (accountability §12 update); D_max never fixed as a plan number; ADR-003 "ACCEPTED" with agents as decision makers (accountability line 25); C6 "P(stale)=0 at Δ ≥ 50 ms" unsupported; ADR §4.1 / G2 / G3 overclaim (accountability §12) and ADR says anti-entropy every 5 s vs code 250 ms.
- **Stated status:** completion report "100% COMPLETE — ACCEPTED (Gate G-M3 PASS)" and "All ten acceptance criteria … zero compromises or waivers", all four known gaps resolved (line 18); master plan §2 "PASS (Loopback Only)"; ledger §C "WEAK / qualified; claim C6 contradicted by an observed stale HIT"; accountability "PARTIALLY IMPLEMENTED; TESTED on loopback … Acceptance criteria satisfied: no"; claims ledger C6 qualified.
- **Supported status:** accountability §2 + §12 (loopback only, durability defect reproduced).

### M4 — Phantom Lab & Static Baselines
- **Primary doc:** `phase_4_execution_package.md` (135 lines; PROPOSED). Supporting: `phase_4_progress.md`, `phase_4_completion_report.md`, `phase_4_changes.patch`, `benchmark_suite_spec.md`, `hypothesis_and_experiments.md`, `docs/demo/lab_runner.md`, `docs/demo-lab/*` (earlier demo-lab design set, not mapped to M4).
- **Obj / Scope / NG:** roadmap §4 M4. NG: controller; baselines tuned only in the system's favor.
- **Arch:** package §3 (owner-reserved O4-x); `src/lab/`, `src/bin/phantom-lab.rs` exist [V]; three disjoint seed sets T/D/S (index §7).
- **Stages/Deps:** M4-A lab core (after G-M1), M4-B baselines (after G-M2); exit G-M4.
- **AC/exit:** AC4-1..12 (§5).
- **Evidence:** lab tests exist (lab_core_tests etc.). **Criterion text drift:** package AC4-9 = "T1↔T2 agreement study delivered", AC4-10 = "Independent reproduction PASS (fresh checkout; second machine if provided)", AC4-7 = "T1 matrix ≥30 seeds/cell; T2 subset complete". The completion report (lines 225–243) restates AC4-10 as "10 reruns produced identical SHA-256 manifest hashes" and AC4-9 as "100% ordinal ranking agreement (r_s = 1.0) certified; T1 designated PREDICTIVE" [V]. The "r_s = 1.0" pattern is the same as the retracted M6 report's "Spearman r_s = 1.00" (phase_6 banner). Accountability: C7 UNVERIFIED, agreement study and reproduction not done.
- **Limits:** first matrix runner was heuristic multipliers (master plan §2 M4 row) and was replaced by a real Tier-1 runner (accountability line 60); BL-CW, BL-FT, BL-PIE-legacy arms, W-TR, cluster scenarios, viewer not done.
- **Stated status:** completion report "GATE G-M4 PASS (all 12 criteria)"; master plan §2 "PASS (Lab Core) / PARTIAL (Matrix Runner)"; ledger §C "IMPLEMENTED, UNVERIFIED as research evidence"; accountability "PARTIALLY COMPLETE … Acceptance criteria satisfied: no".
- **Supported status:** accountability/ledger.

### M5 — Phantom Intelligence & PIE
- **Primary doc:** `phase_5_execution_package.md` v3.1 (152 lines; PROPOSED) **plus** `phantom_intelligence_phase.md` (PI-1..PI-10 spec, referenced by the roadmap). Supporting: `pie_audit.md`, `intelligence_architecture.md`, `phase_5_preregistration.md` (frozen, digest `74CC27F9…`), `phase_5_progress.md`, `phase_5_completion_report.md`, `phase_5_changes.patch`, `intelligence_prior_art_coverage.md`, `intelligence_e2_headroom_protocol_DRAFT.md`.
- **Obj / Scope / NG:** roadmap §4 M5; package §1 five pillars. NG: counting the legacy stub; PIE on simulated metrics.
- **Arch:** M5a machinery (WP-30..35, 37) vs M5b candidates (WP-36a/b/c, WP-18, WP-19a); modes off|observe|advisory|canary|active (index §3).
- **Stages/Deps:** M5a needs G-M1 + G-M2 + M4 lab core; M5b needs G-M4 + G-RESEARCH. **G-RESEARCH was never recorded** as passed in any document I read (RT-0 "not closed", roadmap §7; accountability line 74).
- **AC/exit:** AC5-1..14 + PI-1..PI-10 (package §5); G-M5.
- **Evidence:** `src/intelligence/` exists [V]; `intel_*_tests` exist.
- **Limits:** [V] `grep -rln PieDecisionEngine src` returns only `src/intelligence/engine.rs`; `src/main.rs` constructs `DefaultPolicyExecutor` and `AuditStore` (lines 468, 480) but not the engine. Claims ledger C8 UNVERIFIED (`claims_ledger.md` lines 29, 71).
- **Stated status:** completion report "GATE G-M5 PASS (all 14 criteria)"; **AC5-13 "PASS: genuine PIE v2 constructed in main()"** (completion report line 190) — contradicted by code [V]; master plan §2 "PASS (Machinery & Dispositions) / UNVERIFIED (Serving Path)"; ledger §C "IMPLEMENTED (machinery) — NOT INTEGRATED"; accountability "IMPLEMENTED (machinery), TESTED, NOT INTEGRATED, central hypothesis NOT SUPPORTED … PI-1 not met"; CLAUDE.md §6b `pie: None` (verified: `src/main.rs:433` `pie: None` [V]).
- **Supported status:** accountability/ledger/code evidence. The completion report's AC5-9, AC5-12, AC5-13 are not supported.

### M6 — Research Evaluation & MVP Gate
- **Primary doc:** `phase_6_execution_package.md` (117 lines; PROPOSED). Supporting (many, chain of records): `preregistration_m6.md` (v1), `preregistration_m6_v2.md`, `preregistration_m6_v2_freeze.json`, `preregistration_m6_v2_1_amendment.md` + `_freeze.json`, `preregistration_m6_v2_2_amendment.md` + `_freeze.json`, `preregistration_m6_v2_2_addendum_1.{md,json,pin}`, `novelty_snapshot_m6.md`, `m6_research_evaluation.md` (RETRACTED banner), `m6_sealed_results.md`, `claims_ledger.md`, `phase_6_progress.md` (retraction note), `phase_6_completion_report.md` (RETRACTED banner on line 1, but body still says "100% COMPLETE … MVP-A"), `phase_6_changes.patch`, `literature_review.md`, `hypothesis_and_experiments.md`, `lab-results/m6_sealed_2026-10-29T234845Z/` [V exists], `g1_verdict.md` (Scenario B), `lab-results/scenario_b/20261030T003932Z/` [V].
- **Obj / Scope / NG:** roadmap §4 M6. NG: thresholds chosen after seeing results; novelty claims without closed prior art.
- **AC/exit:** AC6-1..12, G-MVP, falsifiers F-a..F-f.
- **Evidence:** sealed run executed once (2026-10-30 05:16–05:20 per accountability line 71); verdict H1 NOT_SUPPORTED (`m6_sealed_results.md` lines 3–10 [V]).
- **Limits:** RT-0 not closed; C3/C4/C7 UNVERIFIED; freeze addendum record exists (`preregistration_m6_v2_2_addendum_1.*`) after the freeze test failed by design; the master plan cites a file name `preregistration_m6_v2_freeze_amendment_1.json` that does not exist (actual names above).
- **Stated status:** completion report body "100% COMPLETE — Gate G-MVP / G-M6 PASS … MVP-A (Validated & Production Ready)" under a RETRACTED banner (superseded by independent review 2026-10-29T13:22Z); master plan §2 "FAIL on MVP-A Claims / BLOCKED on Release", addendum E "OPEN/HOLD"; `m6_sealed_results.md` NOT SUPPORTED (MVP-B provisional); accountability "EXECUTED; result NEGATIVE … partly"; ledger §C (stale, 00:05) "OPEN / HOLD, freeze INVALID".
- **Supported status:** negative sealed evaluation executed, Tier 1 simulation; MVP-B provisional; G-MVP not met (accountability §2).

### M7 — Productization & Research Release
- **Primary doc:** `phase_7_execution_package.md` (105 lines; PROPOSED — never authorized in writing per any document I read). **There is no `phase_7_progress.md`, no `phase_7_completion_report.md`, no `phase_7_changes.patch`** [V by `ls`]. Execution record is scattered: master plan "M7 status" addendum (line 225), ledger §C and D, `docs/security/m7_security_review.md`, `docs/release/*` (5 files), `docs/user/*` (9 files), `src/config.rs`, `src/health.rs`, `scripts/`, `packaging/` [V exist], tests `m7_config_tests`, `m7_health_tests`, `m7_security_review_tests`, `m7_sec_fixes_tests`, `m7_sec_round2/3_tests` [V exist].
- **Obj / Scope / NG:** roadmap §4 M7; package §1. NG: venue acceptance, new features.
- **Arch:** package work packages M7-00..M7-12 (note: accountability line 29 says "M7-01..12"; package lists M7-00 to M7-12 — consistent). Security decisions SEC-M7-01..11 (Alternative A) in `m7_security_review.md`.
- **AC/exit:** AC7-1..14; G-RELEASE (needs G-MVP and U1 if D-8). Accountability line 29: "original master plan gives only an informal M7 plan (no AC lines)".
- **Limits:** SEC-M7-09/10/11 open (review doc lines for 09, 10; accountability §2 M7); no advisory scan; no graceful drain (`docs/release/deferred_integration.md` D-1); `/metrics` not built; no clean-machine reproduction; PHANTOM_DEFAULT_TTL_MS unwired; release decisions O7-1..11 owner-reserved.
- **Stated status:** package PROPOSED; master plan M7 addendum "agent-reported, 42 tests"; ledger §C (00:05) "IMPLEMENTED with open blockers; SEC-M7-01/02/03 AUTHORIZED, not started" — but `m7_security_review.md` table says SEC-M7-01/02/03 **Fixed … independent review pending** and the accountability report says implemented and "reviewed twice (reading)"; accountability "PARTIALLY IMPLEMENTED; security gates TESTED; release gates NOT MET".
- **Supported status:** accountability §2 M7 (partial; G-RELEASE not attempted).

### M8 — UI/UX Excellence
- **Primary doc:** `phase_8_execution_package.md` (143 lines; PROPOSED). Its own note states the source addendum was cut off mid-sentence and Stages A–F were derived by the research lead (owner decision O8-0 unresolved). **M8 is absent from `mvp_and_milestones.md`.** Supporting: `docs/design/` (ui_audit, principles, information_architecture, tokens, ui_api_requirements, sse_auth_options, lab_api_contract, phantom_lab_ui) [V exist], `ui_metric_provenance.md`, `demo_experience_plan.md`, `docs/demo/browser_verification/{20261030T041006Z,20261030T045605Z,20261030T055008Z}/report.md` [V exist], `docs/user/lab_api.md`, `ui/test/*` [V], `ui/scripts/*`.
- **Obj / Scope / NG / Arch:** package §1, §4 (design direction), §5 (IA, "screens are contracts — real vs proposed"), §3 (Apple design skill evaluation).
- **Stages/Deps:** Stage A–F, M8-00..16; gates U1 (before G-RELEASE if D-8) and U2.
- **AC/exit:** AC8-1..13 (§8); WCAG 2.2 AA; ≥3-participant usability.
- **Evidence:** package Stage C names `SKILL.md`, `SOURCE.md`, `pull-hig.mjs` — **not present in the repo** [V `find` excluding .local/lab-results]; they are conditional deliverables of the skill-decision step.
- **Limits:** no usability study; no axe/Lighthouse; no screen-reader test; browser rounds observed only on debug build/dev server (`report.md` round 3, "Not verified").
- **Stated status:** package PROPOSED; master plan M8 addendum "IMPLEMENTED (unverified visually), not COMPLETE; 84 tests"; ledger §C (00:05) "no browser verification"; ledger §E "Browser screenshots / accessibility results: none exist"; accountability (06:55) "NOT browser-verified"; **later ledger entries (10:06, 11:16, 11:19) record three real-browser rounds** with retained reports; ledger 09:39 "196 UI tests".
- **Supported status:** IMPLEMENTED; browser-observed in three rounds (headless Chrome 156, dev build, EXPLORATORY) — accountability's "never viewed" is superseded by newer records; AC8 conformance still not demonstrated.

### Matrix summary (one line per milestone)
| M | Primary doc | Own completion record | Authoritative-status source today |
|---|---|---|---|
| M1 | `phase_1_execution_package.md` | `phase_1_completion_report.md` (PASS) | accountability §2 (PARTIAL) |
| M2 | `phase_2_execution_package.md` | `phase_2_completion_report.md` (PASS) | accountability §2 |
| M3 | `phase_3_execution_package.md` | `phase_3_completion_report.md` (PASS) | accountability §2 + §12 |
| M4 | `phase_4_execution_package.md` | `phase_4_completion_report.md` (PASS) | accountability §2 |
| M5 | `phase_5_execution_package.md` + `phantom_intelligence_phase.md` | `phase_5_completion_report.md` (PASS) | accountability §2 + claims ledger C8 |
| M6 | `phase_6_execution_package.md` | `phase_6_completion_report.md` (RETRACTED banner) + `m6_sealed_results.md` | `m6_sealed_results.md` + accountability §2 |
| M7 | `phase_7_execution_package.md` | **MISSING** | accountability §2 + `m7_security_review.md` |
| M8 | `phase_8_execution_package.md` | **MISSING** | accountability §2 + browser_verification reports |

---
## 3. Status disagreements, with sources

Authority chain declared in the documents: ledger header (line 4) says the master plan "holds milestone status … If the two disagree, the master plan's evidence-based conclusion wins". Master plan header (line 3) in turn says it "remains the authoritative milestone/audit status". But the master plan's own §2 table was written 2026-10-29 18:45 and later superseded by its addenda (E, Round 2, "Reconciled status 2026-10-30"); the accountability report (2026-10-30 ~06:55, amended 07:09, 08:26) is the latest tabulation and itself says it "supersedes the M3 wording in sections 2 and 9 where they differ" (§12). **There is no single "current status" document; the master plan's table is stale, and a reader following the ledger's header arrives at stale rows.**

| # | Milestone | Claim A (source) | Claim B (source) | Assessment |
|---|---|---|---|---|
| D1 | M1 | PASS, G-M1 SATISFIED, 263 tests/22 targets (`phase_1_completion_report.md` lines 6–11) | PARTIAL; AC not demonstrated (`phantomcache_milestone_accountability_report.md` §2; ledger §C) | Completion report is an AG self-report (accountability line 23: "AG-reported"). Supported: B. |
| D2 | M1 | Package "PROPOSED — NOT READY FOR EXECUTION" (`phase_1_execution_package.md` header) | Executed and gated | Header never amended after execution; no dated owner approval record (accountability line 23). Supported: package header is an unamended historical state. |
| D3 | M1–M2 baseline | "242 tests, 19 targets" (`CLAUDE.md` §3; `README.md` line 260; `docs/handoff/00_START_HERE.md`; `02_PHASE_HISTORY…` §1) | 263/22 (P1 report); 388 (P4 report "baseline 388 passed"); 499/65 (ledger E); 613; 623+1F; 774/74 targets (accountability §4); 802 (g1_verdict); 841 (g2_verdict); 851/196 UI (ledger 09:39–09:40) | Counts are time-stamped snapshots; CLAUDE.md and README present 242 as "the last independently reproduced baseline", which is stale. The "388" in P4 report coincides with the "388 lines" dependency-tree measure; I did not establish whether 388 is a test count or a dependency count [H]. |
| D4 | M2 | PASS, "None. Core data plane is robust and verified" (`master_audit_and_execution_plan.md` §2 row M2) | "COMPLETE-ish in code, TESTED, correctness review ONLY by reading … not demonstrated" (accountability §2) | Supported: B. Master plan §2 is superseded by its own later sections. |
| D5 | M2 | AC2-10 soak PASS (3,000-request soak) (`phase_2_completion_report.md` line 51) | "the 2 h/platform AC is not met" (accountability line 47; line 24 states soak 2 h/platform PROVISIONAL threshold) | Criterion text in completion report is weaker than package threshold or the provisional rule; needs checking against `phase_2_execution_package.md` §4 before concluding (not done here). |
| D6 | M2 | AC2-6: "B4 speedup 200×" PASS (`phase_2_completion_report.md` line 47) | Claim C5 UNVERIFIED; "200× untraceable" (accountability line 47; claims ledger §4) | Supported: B. |
| D7 | M3 | "100% COMPLETE — ACCEPTED (Gate G-M3 PASS) … zero compromises or waivers" (`phase_3_completion_report.md`) | "PARTIALLY IMPLEMENTED … Acceptance criteria satisfied: no"; ADR-003 "ACCEPTED" by agents; durability defect reproduced (accountability §2, §12 update) | Supported: B. ADR-003 Status line 3 "ACCEPTED (Gate G-ARCH Resolution)" is an agent-recorded decision with no owner signature. |
| D8 | M3 | master plan §2 "PASS (Loopback Only)" | ledger §C "WEAK / qualified"; accountability "Acceptance criteria satisfied: no" | Close in substance, different labels. |
| D9 | M4 | G-M4 PASS, AC4-9/AC4-10 PASS (`phase_4_completion_report.md` lines 8, 240–241) | AC4-10/11 missing; C7 UNVERIFIED (accountability line 61–62) | Plus **criterion-text drift** between package and report (see §2 M4). Supported: B. |
| D10 | M4 | master plan "PASS (Lab Core) / PARTIAL (Matrix Runner)" | ledger "IMPLEMENTED, UNVERIFIED as research evidence" | Compatible. |
| D11 | M5 | G-M5 PASS; AC5-13 PASS "PIE v2 constructed in main()" (`phase_5_completion_report.md` lines 8, 190) | `PieDecisionEngine` referenced only in `src/intelligence/engine.rs` [V]; PI-1 not met (accountability line 67; claims ledger C8 UNVERIFIED) | Supported: B and code. This is the largest completion-report/code disagreement. |
| D12 | M5 | master plan §2 "PASS (Machinery & Dispositions) / UNVERIFIED (Serving Path)" | addendum E "dispositions unverified" | Same document disagrees with itself over time. |
| D13 | M6 | "100% COMPLETE … MVP-A (Validated & Production Ready)" (`phase_6_completion_report.md` body) | Same file, line 1 banner: RETRACTED/SUPERSEDED; master plan §2 "FAIL on MVP-A claims"; sealed H1 NOT_SUPPORTED (`m6_sealed_results.md`) | Banner is correct; body left intact as a record (right approach). Residual risk: first-line readers and tools that extract the "Status"/"Verdict" fields will report MVP-A. |
| D14 | M6 | Claims ledger C6 rows "SUPPORTED" originally (claims_ledger line 58: "C4, C5, C6, C8, C10, C13 … Not yet re-reviewed") | C6 qualified/contradicted; C8 UNVERIFIED | Ledger §5 records re-reviews incrementally; some SUPPORTED statuses may still stand un-reviewed (C4, C5, C10, C13). Out of D1 scope: flag for D2. |
| D15 | M6/Freeze | `docs/research/preregistration_m6_v2_2_freeze.json` (56 files hashed) | Test `test_real_v2_2_freeze_record_verifies_when_present` fails by design after additive changes (accountability line 74); later fixed by addendum 1 (+pin) (g1_verdict criteria table) | Resolved historically by `preregistration_m6_v2_2_addendum_1.*`. Accountability R-01 ("open", P0) is therefore stale. |
| D16 | M7 | ledger §C: SEC-M7-01/02/03 "AUTHORIZED, NOT STARTED" (00:05) | `m7_security_review.md`: "Fixed … independent review pending"; accountability: implemented + reviewed twice | Ledger §C row stale; the ledger is append-only, so later D-log entries (not all read) carry the update. |
| D17 | M7 | accountability R-04 etc. "P0 open" | SEC-M7-09/10/11 still open in `m7_security_review.md` | Consistent. |
| D18 | M8 | "NOT browser-verified … `ui/scripts/browser-verify.mjs` never executed" (accountability §2 M8, §3 G, §8); ledger §E "Browser screenshots / accessibility results: none exist" | `docs/demo/browser_verification/20261030T041006Z/report.md` (D-7 round 1, headless Chrome 156), `…045605Z` (round 2), `…055008Z` (round 3), ledger entries 10:06, 11:16, 11:19 [V reports exist] | Accountability and ledger §E are stale for M8; the later records are authoritative. AC8 still not demonstrated (round 3 "Not verified": real zoom, screen readers, non-text contrast, release bundle). |
| D19 | G-series | accountability §5/§7: "G1 IN PROGRESS — NOT PASSED; G2 NOT STARTED" | `g1_verdict.md` (07:35) PASS with D1–D5; `g2_verdict.md` (09:09) PASS with G2-D1..D7; ledger 09:39 "G3 UI delivered" | Accountability §5 stale. Not a milestone but feeds M4/M8 status. |
| D20 | M1 | `MILESTONE_1_DESIGN_PLAN.md` goal 10 "22 automated integration tests"; its AC "All 23 new integration tests"; `docs/milestone_1/WORK_NOTES.md` "29/29 m1_http_tests, 47 total"; `docs/handoff/02_PHASE_HISTORY…` P1 "32 tests in `m1_http_tests.rs`" | — | Four different counts for the same test file across four docs (22/23/29/32); each probably true at its date; none states a date or commit. This is scheme C, not canonical M1. |
| D21 | Roadmap | Roadmap file lists 7 milestones (`mvp_and_milestones.md`); title "The MVP and the Seven Milestones" | Index lists 8; accountability audits 8; CLAUDE.md does not say how many | M8 not in the authoritative roadmap file (see §1.1). |
| D22 | M5/M6 gate | roadmap/package: G-M5 must precede M6; M5b requires G-RESEARCH | M5 completion PASS and M6 executed 2026-10-29 18:15 within ~15 min of M5 closing at 18:00 (completion report headers) | G-RESEARCH never recorded as satisfied (RT-0 open). Process question for owner, see §6. |
| D23 | Handoff | `docs/handoff/00_START_HERE.md`: "Phase 4 … Formally Accepted … Do NOT Begin Phase 5"; `docs/security/phase-4-final-acceptance-report.md` "Final Acceptance"; `02_PHASE_HISTORY` "ACCEPTED" | CLAUDE.md §6: "The handoff's 'Phase 4 accepted / zero P0' is not supported by these findings"; `docs/handoff/05` top note | Handoff "Phase 4" = security/distributed stage 4 (scheme D), a different thing from M4; the correction notices exist (00_START_HERE has the authoritative notice at top; 05 has a superseded note). `02_PHASE_HISTORY…` has no notice at its top (I read its first 60 lines) — needs checking whether a note exists lower [H]. |
| D24 | Corrections | root `CORRECTIONS.md` (39 lines) vs `docs/handoff/CORRECTIONS.md` (82 lines) — `cmp` shows they differ [V] | Phase 1 report AC-3 cites `docs/handoff/CORRECTIONS.md` with "17 re-verified rows" | Two files with the same name and different content; the root copy is untracked and its relationship to the handoff copy is undocumented. |

---
## 4. Missing or weak documents, with proposed outlines

### 4.0 Strongest milestone document (quality reference)
**Candidate: `phase_6_execution_package.md` is not the strongest as a record; the strongest *record chain* is M6 (preregistration → amendments → freeze records → `m6_sealed_results.md` → retraction banners).** For a single document I identify two references, for different purposes:
1. **Specification quality: `phase_5_execution_package.md` / `phase_1_execution_package.md`** — Phase 1 has a "what changed and how each correction was resolved" section (§0), verified starting state with re-verification instructions for the agent (§2), approval items (§3), ordered steps with ids (§5), detailed specs (§6), numbered AC with a result vocabulary (§7), authorized/prohibited file list (§9), risks and revision record (§11). Phase 5 adds five pillars, modes and defaults, and AC blocking flags.
2. **Honest-record quality: `m6_sealed_results.md` plus the M6 preregistration chain.** What makes it strong: the tier is declared in the first line ("SIMULATED … Not a measurement of any network"), the verdict rule was fixed before the run (amendment v2.2), each endpoint is reported with the number of scenarios supported and constraint failures, hypotheses that are not testable are named as such (H2/H4), retained artifacts are located by path [V exist], and the freeze is hash-verified fail-closed.
3. **Best *completion record*: `docs/research/g1_verdict.md`** (a verdict table with requirement / result MET / PARTIAL / evidence / reviewer, then explicit dispositions D1–D5 each with owner and plan, and a "What G1 does not claim" section). This is the pattern the other completion reports lack: they list PASS with a one-line evidence cell and no reviewer or independence statement.
**What other milestones need that is different (do not copy blindly):** M1–M5 completion reports need an *independence column* (who verified; read vs run) and a *criterion-text-equals-package* check, not the M6 preregistration structure. M7 needs a release-readiness record built around AC7 and open-issue register (a security finding table already exists in `m7_security_review.md`, which is itself a good, narrow example: ID, severity, status, location, finding). M8 needs a design-and-evidence record organized by screen/route/viewport/theme with explicit "not verified" lists (the round-3 browser report has this).

### 4.1 Missing documents (listed as MISSING, not filled)
| Missing | Evidence of absence | Why it matters |
|---|---|---|
| **M7 progress log** (`phase_7_progress.md`) | `ls docs/research` [V] | CER-8 requires one live log per phase. |
| **M7 completion report / release-readiness record** (`phase_7_completion_report.md`) | same | G-RELEASE not attempted, but no record states which of AC7-1..14 are PASS/FAIL/NOT RUN. |
| **M7 patch artifact** (`phase_7_changes.patch`) | same | CER-8 (also no patch for M8). |
| **M8 progress log and completion/design-status report** | same | AC8-1..13 status unrecorded; evidence is split across `docs/design/*`, three browser reports, and ledger entries. |
| **Roadmap Rev 3 (M8 in `mvp_and_milestones.md`)** | grep "M8" 0 hits [V] | Index §5 proposed it; never applied. Non-historical file in a "PROPOSED" state, so an edit is possible — but it is an owner decision (see §6). |
| **Written owner authorization / dated acceptance records for M1..M8** | accountability §0 item 1, line 10: "no document records a written owner authorization of each package or a dated owner sign-off" | Gate G-M# are "explicit owner decisions" (roadmap §5, last line) with no record. |
| **Frozen M2 rules file (R-07/08/09 thresholds, soak)** | accountability line 24 | Rule-gated optimizations (WP-07/08/09) had provisional thresholds; no freeze. |
| **G-RESEARCH record** | none found | Gate for M5b; RT-0 open. |
| **Single "current milestone status" document** | §3 above | Three documents each claim authority. |
| **Glossary of numbering schemes (§1.2) and ID namespaces** | none found | Many ID collisions (R-01, D-8, G1, Phase N, M1). |
| **M1-scheme-C disclaimer** at the top of `MILESTONE_1_DESIGN_PLAN.md` / `docs/milestone_1/WORK_NOTES.md` | I read first 30 lines of the plan and 40 of the notes; neither states it is not canonical M1 | Reader confusion with canonical M1. |
| **ADR for G-ARCH owner ratification** | ADR-003 Status ACCEPTED, decision makers are agents (accountability line 25) | Needs a dated owner decision record. |

### 4.2 Weak documents (exist, but insufficient)
| Document | Weakness | Needs |
|---|---|---|
| `phase_7_execution_package.md` | Starting state PROVISIONAL, "never re-verified at M7-00" (M7 work happened without E0 record); statuses of work (config, health, security) are not mapped to M7-01..12 anywhere | M7 status map: per work package M7-01..12 → status → evidence path |
| `phase_8_execution_package.md` | Source addendum incomplete (O8-0 unresolved); starting state "before Phase 1 UI work — PROVISIONAL" is stale; stages A–F never reconciled | Dated owner-reconciliation note; status map per stage and AC8-x |
| `phase_N_completion_report.md` (N=1..5) | Criteria restated differently from packages (M4); self-attested PASS with no independence column; AC5-13 contradicted by code; AC2-6 includes an unverified claim | Dated notice at top of each, pointing to accountability §2 (see §6 A-2) |
| `mvp_and_milestones.md` | 7 milestones, status "PROPOSED" although used as canonical; no M8 | Dated header notice (owner decision) |
| `progress_ledger.md` §A, §C, §E | §A "Last updated … 00:05" stale; §C milestone table pre-dates reconciliation; §E "none exist" for screenshots | Append-only: add dated pointer entry (the 'Reconciled status' entry already exists in the master plan, not in ledger §C) |
| `master_audit_and_execution_plan.md` §2 | Status table still shows PASS rows for M1–M4, which later sections qualify | The document already carries a "Reconciled status 2026-10-30" section; add a short notice immediately under §2's table (append-only style) |
| `docs/handoff/02_PHASE_HISTORY_AND_STATUS.md` | "Current Disposition: ACCEPTED" for P1–P4, "Authoritative Baseline Date: October 28, 2026" | Dated notice (handoff files are historical; 00_START_HERE and 05 already have notices) |
| `README.md` / `CLAUDE.md` | 242-test baseline statement | Owner decision: CLAUDE.md is a rules file; README line 260 says "last independently reproduced" |

### 4.3 PROPOSED outlines (flagged: proposals, not existing documents, not created)
**[PROPOSAL] `docs/research/milestone_status_index.md`** — single status index (non-historical, dated, append-only history table).
1. Purpose and authority (supersedes none; points to authoritative records). 2. Numbering map (copy of §1.2). 3. Table: M1..M8 × {primary doc, completion record, authoritative status source, supported status label (vocabulary: IMPLEMENTED / TESTED / INDEPENDENTLY VERIFIED / UNVERIFIED / RETRACTED / BLOCKED from ledger header), last-reviewed date, owner decision recorded (Y/N + path)}. 4. Known disagreements with links to dated notices. 5. Change log.

**[PROPOSAL] `docs/research/phase_7_status_record.md`** (M7 record that does not fake a completion report): header (status "NOT COMPLETE; G-RELEASE not attempted"); table M7-00..M7-12 → state → evidence path; AC7-1..14 → PASS/FAIL/BLOCKED/NOT RUN per CER-4 with evidence; security table pointer to `m7_security_review.md`; open items (SEC-M7-09/10/11, drain, advisory scan, /metrics, clean-machine reproduction, owner O7-1..11); evidence index; limitations.

**[PROPOSAL] `docs/research/phase_8_status_record.md`**: O8-0 reconciliation status; stages A–F → state → artifact (ui_audit, principles, IA, tokens, phantom_lab_ui…); AC8-1..13 table; browser evidence per round (viewport × theme × route matrix, with the "Not verified" list of round 3); U1/U2 gate status; usability-study status (NOT RUN); a11y status (automated only).

**[PROPOSAL] Dated notice template** for historical documents: `> **Notice (YYYY-MM-DD, D1 audit):** the status in this document is a record of <date>. Current supported status: see <accountability §2 / milestone_status_index>. Original text below is unchanged.`

---
## 5. Stale references

### 5.1 Expected: deleted build directories (2026-10-30, `storage_cleanup_2026-10-30.md`)
Paths deleted: `D:\Personal\_m3_target`, `_m3_target_base`, `_ttl_target`, `_m7_target_g2b`, `_m7_target_before`, `_m7_target_cpl`, `_m7_target_scratch`, `D:\CargoTarget\m7_clean`, `phantom-cache\target` (and `phantom-cache\Personal_m7_target` metadata). References appear in (grep): `docs/demo/browser_verification/*/report.md`, `docs/demo/scenario_b_runbook.md`, `docs/design/phantom_lab_ui.md`, `docs/release/reproduction_guide.md`, `docs/research/m3_durability/reproduction_report.md`, `docs/research/master_audit_and_execution_plan.md`, `docs/research/progress_ledger.md`, `docs/research/storage_cleanup_2026-10-30.md`, `docs/research/ttl_decision/test_plan.md`. **Keep the text in historical records** (browser reports, master plan, ledger, m3_durability, ttl_decision, cleanup record). Binary hashes recorded in those documents can no longer be re-verified (the ledger 11:40 entry says so).
**Not stale and still on disk [V]:** `D:\Personal\_m7_target`, `D:\Personal\_m7_target.zip`, `D:\Personal\_m7_tmp`, `D:\Temp\*` logs (stageA_*, q1_*, q3_*, clippy_round*, sb_fullsuite.log present), `D:\Personal\phantom-cache-snapshots\phase1…phase6-exit` (11 dirs listed; 4 carry an ACL deny per ledger and could not be read). Several documents that reference `_m7_target` are about a directory that still exists; the browser round-2 report notes the debug exe there was relinked by a later cargo test (hash B3149996…), so hashes quoted for the first-round exe cannot be re-checked.
**Non-historical docs that tell a reader to build into a deleted dir** should be checked: `docs/release/reproduction_guide.md` and `docs/demo/scenario_b_runbook.md` and `docs/design/phantom_lab_ui.md` mention these paths; if they are instructions (not records) a user running them would create the directory themselves, so likely benign [H]; D3/D4 may verify.

### 5.2 Other stale or unresolved references found
| Reference | Where | Finding |
|---|---|---|
| `preregistration_m6_v2_freeze_amendment_1.json` | `master_audit_and_execution_plan.md` (owner-decisions section, ~line 275) | Not found anywhere outside `.local` [V]. Actual files: `preregistration_m6_v2_1_freeze.json`, `preregistration_m6_v2_2_freeze.json`, `preregistration_m6_v2_2_addendum_1.{md,json,pin}`. Name drift in a historical record: leave; add a pointer note. |
| `default_ttl_wiring_tests.rs` | ledger entry 2026-10-30 08:13 | Does not exist; belongs to proposed patch in `ttl_decision/proposed_patch.diff` (not applied). Correct per the document's own wording ("proposed"). |
| `verify_ac1.py` | `phase_1_completion_report.md` line 86 | Not located in repo tree outside `.local` snapshots [H: I searched filenames only for a subset; not confirmed]. |
| `SKILL.md`, `SOURCE.md`, `pull-hig.mjs` | `phase_8_execution_package.md` | Not present (conditional deliverables of the skill decision step). |
| `cells.csv`, `tuning_trials.csv`, `manifest.json`, `summary.json` | accountability, ledger | Live under `lab-results/…` (excluded from my find); not stale. |
| Snapshot roots `D:\Personal\phantom-cache-snapshots\phase*` | completion/progress reports | 11 present [V]; 4 unreadable by ACL (per ledger) — contents unverified. |
| `MILESTONE_1_DESIGN_PLAN.md` file:// links (`file:///d:/Personal/phantom-cache/src/api/server.rs`) and line anchors `main.rs#L57-L58` | lines 25–27 of the plan | Absolute local links; the described state ("`_cache` and `_coalescer` discarded") is historical (scheme C) and no longer true. Leave as record; add scheme-C notice. |
| CLAUDE.md §6 statement "peer invalidations purge a per-node side cache" | CLAUDE.md | Already annotated PARTLY OUTDATED in CLAUDE.md itself; startup message `main.rs:368` still out of date per CLAUDE.md. Out of D1 scope. |
| `docs/research/phase_6_completion_report.md` | "G-M6" | Name not used in roadmap (see §1.4). |
| Statement "`docs/handoff/CORRECTIONS.md` did not yet exist" | index §5 | Historical at 2026-10-29; now exists (82 lines). |

---
## 6. Recommended actions

### A. Safe documentation-only (do not alter any historical record; add new files or dated notices)
- **A-1.** Create the **numbering map / milestone status index** (§4.3 proposal) as a new file under `docs/research/`. No existing file touched.
- **A-2.** Add **dated notices** (not edits of body text) to the top of `phase_1..5_completion_report.md` pointing to accountability §2 and listing the specific over-statements: M4 AC4-9/AC4-10 wording vs package; M5 AC5-13 vs `main.rs`; M2 AC2-6 200x claim. Phase 6 already has a banner. *Classification note: these are historical records; adding a notice is the amendment pattern the project already uses (phase_6 banner, CORRECTIONS.md). Final decision on whether to touch them is the owner's (see C-4).*
- **A-3.** Add a **scheme-C notice** to `MILESTONE_1_DESIGN_PLAN.md` and `docs/milestone_1/WORK_NOTES.md`: "pre-roadmap Milestone 1 (HTTP integration); not canonical M1". Same caution as A-2.
- **A-4.** Write **M7 and M8 status records** (proposed outlines in §4.3) as new files; mark them status records, not completion reports.
- **A-5.** Append dated pointer entries (append-only) to the ledger and master plan: accountability §2 M8 and §5 G-rows are superseded by browser_verification reports and `g1/g2_verdict.md`. The ledger is explicitly append-only (header: "dated entry" correction mechanism).
- **A-6.** Update `docs/research/claims_ledger.md` only via its own dated-section mechanism (it already has §4–§6); feed D2 with: C4/C5/C10/C13 still "SUPPORTED, not re-reviewed" (line 58).
- **A-7.** Add an ID-namespace glossary (R-01 ×3, D-8 ×2, G1 vs G-M1, "Phase N" ×4, WP1 vs WP-01, M1 vs M1) to the index of A-1.
- **A-8.** Reconcile test-count history in a single table (dated, scheme: date / scope / count / source file) in the index; keep `CLAUDE.md` and README wording for the owner (C-2).

### B. Needs owner decision
- **B-1.** Whether `mvp_and_milestones.md` (status "PROPOSED", Rev 2) should receive Rev 3 adding M8 (index §5), and whether it should be re-labelled from PROPOSED to a ratified status.
- **B-2.** Ratification or rejection of agent-recorded decisions: ADR-003 (G-ARCH, "ACCEPTED" by agents), M8 stages A–F (O8-0), A-1..A-14 approval items (drift), M1–M8 package authorizations and gate acceptances (no dated owner sign-offs).
- **B-3.** Whether the `CLAUDE.md` §3 "Reported baseline (242 tests)" and README line 260 should be updated to the latest lead-reproduced counts (CLAUDE.md is a rules file under owner control; changes need owner direction).
- **B-4.** Whether the completion reports for M1–M5 are annotated (A-2) or left byte-identical with the index carrying the correction. Both are consistent with the "historical records are never errors" rule; the project has precedent for both (phase_6 banner prepended; CORRECTIONS.md ledger).
- **B-5.** Which document is *the* authority for milestone status: master plan (per ledger header and its own header), or the accountability report (latest), or a new index (A-1).
- **B-6.** Process question: M6 execution began 15 minutes after M5 closure and G-RESEARCH is unrecorded (D22); owner may wish to record a waiver or a statement that RT-0's open status was accepted.
- **B-7.** Whether M7/M8 should be treated as in-flight milestones with status records (A-4) or deferred until G-MVP/U1 decisions (D-8).
- **B-8.** Naming of root `CORRECTIONS.md` vs `docs/handoff/CORRECTIONS.md` (D24).

### C. Must-not-touch (historical/frozen/sealed/append-only)
- C-1. All frozen preregistration records and hash-verified artifacts: `preregistration_m6*.md`, `preregistration_m6_*freeze.json`, `preregistration_m6_v2_2_addendum_1.{md,json,pin}`, `phase_5_preregistration.md` (digest `74CC27F9…`), `lab-results/m6_sealed_*`, `lab-results/scenario_b/*`, `m6_sealed_results.md` (the freeze test hashes files in the inventory; any change breaks `test_real_v2_2_freeze_record_verifies_when_present` until an addendum 2; g1_verdict §3 lists the frozen inventory).
- C-2. `phase_N_changes.patch`, `phase_N_snapshot_manifest`, snapshot directories (including the four ACL-protected originals and `.local/snapshots`).
- C-3. `progress_ledger.md` D-log entries and `master_audit_and_execution_plan.md` dated sections (append only).
- C-4. `phase_1..6_completion_report.md` and `phase_N_progress.md` bodies (add notices only, if the owner approves B-4).
- C-5. `g1_verdict.md`, `g2_verdict.md`, `storage_cleanup_2026-10-30.md`, `docs/demo/browser_verification/**` (retained evidence), `docs/security/phase-4-final-acceptance-report.md` (dated review report).
- C-6. `docs/handoff/transcripts/` and `transcript_copy.jsonl` (excluded from commit plan per ledger §G).
- C-7. Source code, tests, `Cargo.toml`, `src/lib.rs` (inside freeze inventory) — out of scope for documentation work.

---
## Final summary (also returned to caller)

**Key findings.**
1. The authoritative milestone definition covers M1–M7 only (`mvp_and_milestones.md` Rev 2); M8 exists only in the index and `phase_8_execution_package.md`; the Rev 3 edit was never applied.
2. Eight different numbering schemes/ID namespaces coexist; "Milestone 1" in `MILESTONE_1_DESIGN_PLAN.md` is a different, pre-roadmap milestone; "Phase 4/5" in the handoff means security stages, not M4/M5.
3. Every milestone package is headed "PROPOSED — not authorized"; no written owner authorization or dated acceptance exists for any gate.
4. Completion reports exist for M1–M6 only; M7 and M8 have no progress log, no completion/status record, no patch.
5. Completion reports M1–M5 state PASS; the accountability report states no milestone has independently passed its AC. Documented examples: M5 AC5-13 (code contradicts), M4 AC4-9/AC4-10 (criterion wording differs from package; agreement study vs claimed r_s=1.0), M2 AC2-6 (200x claim UNVERIFIED), M3 (loopback only; durability defect reproduced).
6. M6 completion report body still says MVP-A (banner says retracted); sealed result is H1 NOT_SUPPORTED (Tier 1).
7. The status-authority chain is circular/stale: ledger → master plan §2 (2026-10-29 18:45) → accountability (2026-10-30 06:55) → later `g1/g2_verdict.md` and browser reports.
8. Accountability and ledger §E say M8 was never viewed in a browser; three retained browser rounds exist later the same day. Accountability G1/G2 status is also superseded.
9. Test counts 242/263/388/499/613/623/774/802/841/851 are quoted without a common table; CLAUDE.md and README still cite 242.
10. Stale file names: `preregistration_m6_v2_freeze_amendment_1.json`, `default_ttl_wiring_tests.rs` (proposed only), `verify_ac1.py`.

**Strongest document:** for record honesty `g1_verdict.md` (requirement / result / evidence / reviewer table plus explicit dispositions) and the M6 preregistration chain with `m6_sealed_results.md`; for specification `phase_1_execution_package.md`. Other milestones need an independence column and criterion-equality checks rather than M6's preregistration structure.

**Open questions for the owner.** B-1..B-8 above; plus: was there ever a written authorization for each package (the accountability report found none)? Is the M8 addendum remainder available? Should root `CORRECTIONS.md` be reconciled with the handoff copy? Is 388 a test count or a dependency count in the Phase 4 report?

**Not verified / limits of this audit.** I did not run tests or read every ledger entry (D-log lines ~81–260 skimmed by `grep`/tail only); did not open `phase_N_progress.md` bodies beyond headers; did not read `phantom_intelligence_phase.md`, `pie_audit.md`, ADR bodies, or `docs/user/*` in full; AC2-10 threshold comparison against `phase_2_execution_package.md` §4 was not completed (D5). Statements marked [D] are document statements, not re-derived.
