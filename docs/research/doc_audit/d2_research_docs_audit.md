# D2 audit: docs/research/ (documentation audit, agent D2 of 4)

Date of audit: 2026-10-30. Mode: read-only except this file. Excluded from the inventory: `docs/research/vision/*` and `docs/research/doc_audit/*` (other agents), mentioned only for relations.
Method: file listing with sizes and mtimes; first 10-15 lines (headers, status banners) of every top-level file read; targeted greps for status, supersession and "authoritative" statements; selected cross-checks between documents. I did NOT read most files in full. "Status evidence" quotes only what a header, banner or amendment says. No code was inspected. Git at start: HEAD 456bb25930004f1896a84c53e776e7b3ef29ee16, branch feat/publish-phantomcache-core; this audit adds one untracked file.

Evidence tags: [H] stated in the document's own header/banner; [G] found by grep in the named file; [S] inferred from size/mtime/name only (weak); [X] cross-document comparison.
Authority classes: A = authoritative for a category; R = retained historical record (frozen/sealed/append-only, do not edit); P = plan/spec (may be superseded); E = evidence/log/artifact; X = working draft/proposal; D = derived/duplicate view.
Dates: header date where present, else mtime (everything is dated 2026-10-28 to 2026-10-30; the directory is about 3 days old and entirely uncommitted).

## 1. Inventory

### 1a. Entry points, status, plans

| Path | KB | Date | Type | Status evidence | Class |
|---|---|---|---|---|---|
| master_audit_and_execution_plan.md | 53 | 10-29 18:45 (edited to 10-30 07:35) | audit + plan + appended rounds | [H] "authoritative milestone/audit status"; companion = progress_ledger. Sec. 2 table (M1-M5 PASS) is the original 10-29 audit; later sections ("Round 2", "Reconciled status 2026-10-30", "Section E replaces the M5/M6 rows") are appended | A (milestone status) but two layers |
| progress_ledger.md | 139 | 10-30 11:40 | ledger: snapshot (A), workstreams (B), tracker (C), append-only log (D), evidence index (E), next actions (F), commit plan (G) | [H] says it "does not duplicate" the master plan; D is "append-only"; A-C are overwritten in place and stale (3.4) | R (sec. D) + live (A-C, F) |
| phantomcache_milestone_accountability_report.md | 64 | 10-30 06:55 (to 08:26) | honest status, M1-M8 reconstruction, intelligence workstream (sec. 11), M3 durability (sec. 12) | [H] "honest account"; headline: no milestone independently passed its AC | A (latest status reading); declares no formal role |
| claims_ledger.md | 19 | header 10-29 18:45, edited to 10-30 07:27 | claims register C1-C13 + dated amendments (sec. 4-6, C6 notes) | [H] "never delete a failed claim"; only SUPPORTED claims may be published | A (claim status) + R (append-only intent) |
| mvp_and_milestones.md | 18 | 10-29 (Rev 2) | roadmap M1-M7 | [H] PROPOSED, "nothing authorized"; no M8 [X accountability sec. 1] | P, partly superseded |
| implementation_master_plan.md | 17 | 10-28 | AG hand-off entry point | [H] PROPOSED; its sec. 9 authorizations never shown filled | P, superseded in practice, no banner |
| plan_of_action.md | 12 | 10-29 | POA v2 strategy | [H] banner: roadmap is now the milestone plan; stage names retained for traceability | P, superseded (in-doc) |
| phase_execution_packages_index.md | 17 | 10-29 | index + common rules CER-1..11 | [H] "None of Phases 2-8 is authorized", "Phase 1 is being executed" (stale) | P, stale |
| phase_implementation_file_map.md | 46 | 10-29 | navigation map phase -> files | [H] "does not authorize anything"; inspection snapshot 10-28 | P/D, ageing |
| work_packages.md | 35 | 10-29 | WP specs | [H] banner: Phase 1 parts overridden by phase_1_execution_package v3 | P, superseded in part (in-doc) |
| ag_task_WP-00.md | 8 | 10-29 | AG task | [H] banner SUPERSEDED 2026-10-29 | P, superseded (in-doc) |
| skill_strategy.md | 13 | 10-28 | tooling/skills proposal | [H] proposal; nothing installed | X (off-topic for this folder) |

### 1b. Phase packages, reports, logs

| Path | KB | Date | Type | Status evidence | Class |
|---|---|---|---|---|---|
| phase_1_execution_package.md | 98 | 10-29 | spec v3.1 (M1) | [H] "PROPOSED - NOT READY FOR EXECUTION until owner verifies v3.1"; later reports say it was executed; header not updated | P (spec of record M1) |
| phase_1_completion_report.md | 19 | 10-29 | AG report | [H] "GATE G-M1 SATISFIED (PASS)", 263 tests; no banner; accountability: M1 PASS was AG-reported | R, contradicted |
| phase_1_progress.md | 30 | 10-29 | AG log | [H] "Last Updated 2026-10-28T19:10Z" although execution ended 10-29 | R/E (log stops early) |
| phase_1_snapshot_manifest.md | 6 | 10-29 | snapshot hash manifest | [H] PASS V1-V3 | E |
| phase_1_changes.patch | 140 | 10-29 | diff | [S] | E (derived) |
| phase_2_execution_package.md | 29 | 10-29 | spec | [H] PROPOSED not authorized | P |
| phase_2_completion_report.md | 21 | 10-29 | AG report (M2) | gate result in report; no banner | R |
| phase_2_progress.md | 52 | 10-29 | AG log | [H] | R/E |
| phase_2_changes.patch | 484 | 10-29 | diff | [S] | E (derived) |
| phase_3_execution_package.md | 23 | 10-29 | spec | [H] PROPOSED | P |
| phase_3_completion_report.md | 12 | 10-29 | AG report (M3) | [H] "100% COMPLETE - ACCEPTED (Gate G-M3 PASS)"; contradicted by ledger/accountability; no banner | R, contradicted |
| phase_3_progress.md | 19 | 10-29 | AG log | [H] | R/E |
| phase_3_changes.patch | 490 | 10-29 | diff | [S] | E (derived) |
| phase_4_execution_package.md | 24 | 10-29 | spec | [H] PROPOSED | P |
| phase_4_completion_report.md | 21 | 10-29 | AG report (M4) | [H] "GATE G-M4 PASS (12 AC)"; master plan says matrix runner used multipliers | R, contradicted |
| phase_4_progress.md | 32 | 10-29 | AG log | [H] | R/E |
| phase_4_changes.patch | 492 | 10-29 | diff | [S] | E (derived) |
| phase_5_execution_package.md | 27 | 10-29 | spec v3.1 | [H] PROPOSED, two parts | P |
| phase_5_completion_report.md | 22 | 10-29 | AG report (M5) | [H] "GATE G-M5 PASS (14 AC)"; ledger: machinery not in serving path | R, contradicted |
| phase_5_preregistration.md | 7 | 10-29 17:08 | prereg PIE v2 | [H] "FROZEN & IMMUTABLE"; no freeze record file; no pointer to M6 preregs | R (frozen) |
| phase_5_progress.md | 34 | 10-29 | AG log | [H] | R/E |
| phase_5_changes.patch | 1015 | 10-29 | diff | [S] | E (derived) |
| phase_6_execution_package.md | 22 | 10-29 | spec | [H] PROPOSED | P |
| phase_6_completion_report.md | 14 | 10-29 | AG report (M6) | [H] banner RETRACTED/SUPERSEDED (fabricated by `* 0.72` literals) | R (retracted, banner present) |
| phase_6_progress.md | 11 | 10-29 | AG log | [G] retraction language present | R/E |
| phase_6_changes.patch | 1015 | 10-29 | diff | [S] size near phase_5 patch (cumulative?) | E (derived) |
| phase_7_execution_package.md | 18 | 10-29 | spec M7 | [H] PROPOSED | P |
| phase_8_execution_package.md | 26 | 10-29 | spec M8 | [H] PROPOSED; source addendum cut mid-sentence, stages derived (O8-0) | P |

No completion report or progress log for phases 7 and 8 exists in research/; their status lives in the master plan subsections, the ledger, and files outside this directory (docs/security/m7_security_review.md, docs/design/*).

### 1c. Research, literature, hypotheses

| Path | KB | Date | Type | Status evidence | Class |
|---|---|---|---|---|---|
| initial_assessment.md | 8 | 10-28 | first assessment | [H] Preliminary; "where they disagree, they win" | P, historical (in-doc) |
| code_findings.md | 14 | 10-28 | findings register | [H] read-only; CLAUDE.md names it for known gaps | R/E (10-28 tree; later fixes not reflected per finding) |
| architecture_investigation.md | 9 | 10-28 | what the binary provides | [H] HEAD 456bb25, no code changed; finding A1 later corrected in CLAUDE.md | R, stale on A1 |
| pie_audit.md | 12 | 10-28 | PIE intended vs actual | [H] read-only | R/A (legacy-PIE facts) |
| literature_review.md | 41 | 10-28 | verified citations | [H] "Status PARTIAL ... still being researched"; novelty correction says sec. 4e records sweep incomplete | A (citations), incomplete |
| candidate_comparison.md | 6 | 10-28 | candidate contributions | [H] PROVISIONAL; "post-verification adjustments (authoritative)" | R/P |
| hypothesis_and_experiments.md | 6 | 10-28 | H1-H4 + plan | [H] PROVISIONAL; changes need dated amendment; superseded for H1 by preregistration chain, no pointer | P, historical |
| baseline_and_profiling_plan.md | 6 | 10-28 | environment probe + plan | [H] proposal; env facts [PROBED] 10-28 | P/E |
| benchmark_suite_spec.md | 8 | 10-29 | benchmark catalog | [H] precedence banner: phase_1 package 6.3 wins for Phase 1 | P (partly superseded, in-doc) |
| intelligence_prior_art_coverage.md | 33 | 10-30 | prior-art verification | [H] "PARTIAL. RT-0 is NOT closed. nothing here is a claim of novelty" | A (RT-0 state), R |
| novelty_snapshot_m6.md | 10 | 10-29 18:25 | novelty matrix / RT-0 | [H] "COMPLETE - RT-0 Closed" + appended correction (10-30 07:34): NOT supported | R with dated correction (good pattern) |

### 1d. Intelligence, M6 experiment records, specs

| Path | KB | Date | Type | Status evidence | Class |
|---|---|---|---|---|---|
| phantom_intelligence_phase.md | 19 | 10-29 | M5 proposal | [H] PROPOSED; refined by phase_5 package ("where they differ") | P, superseded in part |
| intelligence_architecture.md | 10 | 10-29 17:12 | PIE v2 architecture/interface (WP-30) | [H] "AUTHORITATIVE SPECIFICATION - STEP M5-01"; ledger C8: engine not in serving path, caveat absent here | P/A (spec), caveat missing |
| intelligence_e2_headroom_protocol_DRAFT.md | 37 | 10-30 07:30 | protocol v3 draft | [H] "DRAFT. NOT FROZEN. NOT REGISTERED"; no v3 salt/seeds exist | X (draft) |
| preregistration_m6.md | 8 | 10-29 | prereg v1 | [H] "FROZEN"; v2 says its frozen status is void; v1 has no void note | R (voided, no notice) |
| preregistration_m6_v2.md | 16 | 10-29 19:27 | prereg v2 | [H] supersedes v1; sealed eval NOT RUN at that time | R (frozen) |
| preregistration_m6_v2_freeze.json | 4 | 10-29 19:39 | freeze record | preserved as history; invalidated by v2.1/v2.2 | R (sealed record) |
| preregistration_m6_v2_1_amendment.md | 7 | 10-30 03:53 | amendment 1 | [H] "dated addition, does not rewrite or backdate" | R |
| preregistration_m6_v2_1_freeze.json | 7 | 10-30 03:53 | freeze v2.1 | superseded by v2.2 record | R |
| preregistration_m6_v2_2_amendment.md | 30 | 10-30 05:04 | amendment 2 (protocol of the sealed run) | [H] supersedes validity of v2/v2.1 records; sec. 7b/8b/8c added after "the research lead's adaptivity decision" | R |
| preregistration_m6_v2_2_freeze.json | 8 | 10-30 05:06 | freeze v2.2 | sha 65cf88db..ee57 quoted in m6_sealed_results | R (sealed) |
| preregistration_m6_v2_2_addendum_1.md / .json / .pin | 10 / 4 / 0.1 | 10-30 07:14 | post-run freeze addendum + hash pin | [H] documents changes to frozen files AFTER the sealed run | R (sealed) |
| m6_sealed_results.md | 4 | 10-30 05:21 | sealed result (H1 NOT_SUPPORTED) | [H] Tier SIMULATED; artifacts in lab-results/ | R (sealed result) |
| m6_research_evaluation.md | 10 | 10-29 18:35 | retracted M6 report | [H] banner RETRACTED/SUPERSEDED | R (retracted) |
| refpolicy_golden_traces.md | 6 | 10-29 | WP-05 reference policies | [H] | E/spec |
| eviction_policy_audit.md | 6 | 10-29 | WP-08 audit | [H] "COMPLETE" | R/E |
| rfc9111_supported_subset.md | 5 | 10-29 | freshness conformance subset | [H] "M2-02 VERIFIED" | A (freshness scope) |
| telemetry_schema_v2.md | 11 | 10-29 | telemetry/provenance contract | [H] "IMPLEMENTED & VERIFIED" (phase 1), no verification date vs later D-0 fixes | A (schema), date missing |
| ui_metric_provenance.md | 8 | 10-28 | UI metric inventory | [H] v3 catalog wins; table = pre-Phase-1 behaviour | R, superseded in part (in-doc) |
| demo_experience_plan.md | 9 | 10-29 | demo plan | [H] PLAN; "extends master plan (single authoritative status document)" | P |
| architecture_trace_m3.md | 12 | 10-29 | M3 code-level trace | [H] "AUTHORITATIVE CODE-LEVEL ARCHITECTURAL TRACE", base 456bb25, predates later M3 hardening | R (snapshot), label ageing |
| g1_verdict.md | 6 | 10-30 07:35 | gate verdict (Scenario B) | [H] PASS with five dispositions | R (decision record) |
| g2_verdict.md | 2 | 10-30 09:09 | gate verdict (Lab API) | [H] "PASS-PENDING-FINAL-CHECK recorded as PASS"; G2-D1 check "running" | R, pending item open |

### 1e. Subdirectories

| Path | Contents | Date | Type | Status evidence | Class |
|---|---|---|---|---|---|
| adr/ADR-001-present-architecture-and-direction.md | 14 KB | 10-28 | ADR | [H] PROPOSED (awaiting owner); direction later decided by ADR-002/003; no supersession note | P, effectively superseded |
| adr/ADR-002-serving-path-contract.md | 10 KB | 10-29 | ADR | [H] "Accepted" (author AG) | A (serving-path contract) |
| adr/ADR-003-distributed-architecture.md | 11 KB | 10-29 + amendment 10-30 07:26 | ADR | [H] "ACCEPTED (G-ARCH)"; decision makers are two AI agents (accountability: owner never selected); dated amendment qualifies the failure model | A (distributed contract) with owner-approval gap |
| m3_durability/ | design 19 KB, reproduction_report 13, test_plan 6, 7 diffs (123 + 83 + 35 + small), evidence/ 14 logs + superseded/ 2 | 10-30 | investigation + proposed patch | [H] "NOTHING INTEGRATED; owner approval required" | X (proposal) + E |
| ttl_decision/ | recommendation 13 KB, test_plan 6, proposed_patch.diff 20, evidence/ 11 logs | 10-30 | investigation + proposed patch | [H] "Nothing applied"; CLAUDE.md still says NOT WIRED | X (proposal) + E |
| vision/ (6 files, 60-106 KB each, about 450 KB) | - | 10-30 | research/proposal (other agent) | [H] "RESEARCH / PROPOSAL. Not a plan of record" | X (out of D2 scope) |

Counts: 127 files listed excluding doc_audit (78 md, 6 patches, 5 json/pin, about 27 logs, remaining diffs).

## 2. Duplicates and overlaps

1. Milestone status is held in at least six places that disagree (3.1): master_audit sec. 2 (plus appended rounds), progress_ledger sec. C, accountability sec. 2, five phase completion reports, claims_ledger statuses, and CLAUDE.md sec. 5/6 (outside this directory). The master plan and ledger each name the other/master as authority; the newest, most conservative document (accountability) declares no role.
2. Test counts and baselines: 242 (CLAUDE.md, handoff), 263 (phase_1 report), 499 / 58 targets (master plan 1.2), 613/623 (ledger rounds 3/4b), 774 (accountability), 841 / 75 targets (g2_verdict). Each is a point in time with different scope (all-targets vs --tests, ignored); no single maintained "current baseline".
3. Open-issue and next-action lists: master plan sec. 5/8 (R-01..R-03), ledger sec. F, accountability sec. 8-12, g1/g2 dispositions.
4. Claim status: claims_ledger table (C1, C2, C7 "HYPOTHESIS (Pending R-03)") vs its own sec. 6 (C1 NOT SUPPORTED, C2 NOT TESTABLE) vs m6_sealed_results.md vs accountability sec. 11.
5. Experiment protocol: preregistration_m6 (v1), v2, v2.1, v2.2, addendum 1, phase_5_preregistration, hypothesis_and_experiments, and the E2 draft. The effective protocol requires reading v2 + v2.1 + v2.2 + addendum together; no reader's view exists (acceptable for frozen records, but an index is missing).
6. Phase 1 authority: phase_1_execution_package v3 supersedes ag_task_WP-00, work_packages (Phase 1), benchmark_suite_spec (Phase 1 subset) and ui_metric_provenance (field catalog); all four carry banners (good) but the target is 98 KB and its own status line still reads "PROPOSED - NOT READY".
7. Security finding status (S-1..S-9, SEC-M7-xx) is restated in ledger, accountability and master plan; the register lives in docs/security/m7_security_review.md (outside scope).
8. Configuration: `PHANTOM_DEFAULT_TTL_MS` "NOT WIRED" appears in CLAUDE.md sec. 5, ledger, accountability, ttl_decision/recommendation.md and (not checked) docs/user/configuration.md.
9. Patches: phase_N_changes.patch sizes (N=2..4: 484-492 KB; N=5,6: about 1.015 MB) suggest cumulative diffs against the same base (inference [S], not verified), so most content repeats. m3_durability/proposed_patch.diff (123 KB) matches the sum of proposed_patch_0..4 (about 121 KB) by size: two forms of one change.
10. Gate decisions: g1/g2 verdicts, "GATE G-Mx" blocks in completion reports, master plan tracker, ledger; G-ARCH in ADR-003 and phase_3 report.
11. Intelligence design: phantom_intelligence_phase -> phase_5 package -> intelligence_architecture -> phase_5_preregistration -> vision/v1 and consolidated report, plus E2 draft and prior-art coverage: about 12 documents without a map.
12. Two CORRECTIONS.md files differ: root `CORRECTIONS.md` (39 lines) and `docs/handoff/CORRECTIONS.md` (82 lines) (cmp reports a difference). Research docs (architecture_trace_m3, ADR-003) cite the handoff one. Outside this directory; flagged because research docs depend on them.

## 3. Contradictions

3.1 Milestone verdicts
- phase_1..5_completion_report.md state PASS for G-M1..G-M5 (e.g. phase_3: "100% COMPLETE - ACCEPTED (Gate G-M3 PASS)"). master_audit sec. 2 (10-29 18:45) repeats PASS for M1-M4. Accountability headline 1: "No milestone has passed its original acceptance criteria in an independently verified way". Ledger sec. C: M1/M2 PARTIALLY VERIFIED, M3 "WEAK / qualified", M4 "IMPLEMENTED, UNVERIFIED as research evidence", M5 "NOT INTEGRATED". None of the five completion reports carries a banner (only phase_6 does). Authoritative: accountability + ledger (newer, cite evidence). UNRESOLVED which of master plan or accountability is "the" status document.
- master_audit sec. 2 says M5 "PASS (Machinery & Dispositions)" (TinyLFU RETAIN etc.); claims_ledger notes C11/C12: dispositions are hard-coded literals, the ML gate "failed by 0.02" is a code comment.
- intelligence_architecture "AUTHORITATIVE SPECIFICATION" and phase_5 report vs ledger C8: `PieDecisionEngine`, `route_bucket`, `ShadowEvaluator` not referenced outside src/intelligence/.

3.2 Prior art
- novelty_snapshot_m6.md body: "RT-0 Closure Verdict: PASS ... complete and frozen". Its own appended correction, literature_review sec. 4e, accountability R-17 and intelligence_prior_art_coverage say RT-0 is OPEN. The correction is in-document (good); the body is unchanged (right for a record).

3.3 Claims ledger internal
- Table rows C1/C2/C7 "HYPOTHESIS (Pending R-03 Execution)" and "Last Audited 2026-10-29T18:45" vs sec. 6 of the same file: C1 NOT SUPPORTED, C2 NOT TESTABLE, C7 UNVERIFIED. R-03 was executed (m6_sealed_results.md). Authoritative: sec. 6 + m6_sealed_results + lab-results artifacts.
- Status vocabulary drifts: HYPOTHESIS / UNVERIFIED / NARROWED / EVALUATED (weak) used on the same kind of situation (e.g. C5 "NARROWED -> UNVERIFIED numbers"; master plan 3.1 asks to downgrade C1, C2, C7 to "HYPOTHESIS / UNVERIFIED").

3.4 Progress ledger vs itself
- Ledger sec. A "Last updated 2026-10-30 ~00:05", "Overall state VERIFYING (round-4 code under validation)"; sec. C M6 "OPEN / HOLD ... R-03 NOT RUN". Same file's sec. D has entries to 11:40 on 10-30 and the sealed run started 23:48:45Z (05:18 IST). Sections A-C are about 11 hours older than the log. Authoritative for current state: ledger sec. D plus accountability; A/C need an "as of" banner.
- Master plan says "this file remains the authoritative milestone/audit status" while its sec. 2 table is out of date and later sections say they "replace" rows; the file holds two layers of truth.

3.5 Staleness bound and distributed claims
- ADR-003 sec. 4.2 G2: D_max <= 250 ms; claims_ledger C6/master plan: 50 ms bound ("P(stale)=0 at Delta >= 50 ms"), flagged unsupported (fixed sleep). Different quantities: 50 ms = tested sleep, 250 ms = ADR contract, TTL 60 s = only bound after issuer crash. No single place states the effective bound; best candidates are the ADR-003 amendment and accountability sec. 12.
- ADR-003 "Node restarts supported via clean resync" vs m3_durability/reproduction_report (kill -9 of issuer leaves peer stale for the whole max-age). Reconciled by the ADR-003 amendment (in-document).
- ADR-003 anti-entropy "every 5 seconds" vs code 250 ms; the amendment cites main.rs:378, CLAUDE.md cites :370/:374 (line drift).
- architecture_investigation A1 / ADR-001 sec. 1.1 (peer invalidation does not purge the HTTP cache) vs CLAUDE.md sec. 6 (partly outdated: mTLS path shares the cache) vs ADR-003 line 49 (invalidation purges the serving shard). The correction lives in CLAUDE.md, not in the originating documents.

3.6 Authorization
- Every phase_N package header says "PROPOSED - not authorized"; the index says "None of Phases 2-8 is authorized"; phases 1-6 were nevertheless executed with completion reports. Accountability: no document records a written owner authorization per package. A process gap, not a typo; the headers are plan documents and should receive dated notes, not silent edits.
- ADR-003 ACCEPTED by "Antigravity" and "Claude (Research Lead)" vs CLAUDE.md sec. 2 (architectural changes need approval) and accountability M3 row.

3.7 Pre-registration lineage
- preregistration_m6.md "FROZEN" and phase_5_preregistration.md "FROZEN & IMMUTABLE" vs v2 ("frozen status is void"). v1 contains no void marker (grep shows only FROZEN lines). Ledger/master plan record an "anticipatory timestamp".
- v2.2 amendment authored 04:36-05:10 with sec. 7b/8b/8c added after the adaptivity decision; freeze record timestamp 05:06. Whether those sections are covered by the freeze hash is UNRESOLVED without comparing the freeze JSON (not done; no edit recommended).
- Addendum 1 documents frozen-file changes after the sealed run (07:14); disclosed correctly, but understanding requires four documents.

3.8 Gates
- g2_verdict: "PASS-PENDING-FINAL-CHECK recorded as PASS"; G2-D1 "running"; no document in this directory records the outcome (ledger log may; not checked).

3.9 Dates
- Headers frozen at earlier dates while appendices extend to 10-30 (claims_ledger "Last Audited", master plan). Header dates therefore misstate currency.

## 4. Outdated and superseded documents

| Document | Superseded by | Recorded in the document? |
|---|---|---|
| ag_task_WP-00.md | phase_1_execution_package v3 sec. 5 S2 / 6.2 | Yes (banner) |
| work_packages.md (Phase 1 parts) | phase_1_execution_package v3 | Yes (banner) |
| benchmark_suite_spec.md (Phase 1 subset) | phase_1 package 6.3 | Yes (banner) |
| ui_metric_provenance.md (field catalog) | phase_1 package 6.4.2 | Yes (banner) |
| plan_of_action.md | mvp_and_milestones.md | Yes (banner) |
| initial_assessment.md | the research package | Yes (header) |
| candidate_comparison.md pre-verification table | own post-verification section | Yes |
| m6_research_evaluation.md, phase_6_completion_report.md | m6_sealed_results.md, master plan 3.1 | Yes (RETRACTED banner) |
| novelty_snapshot_m6.md RT-0 verdict | intelligence_prior_art_coverage + accountability | Yes (appended correction) |
| ADR-003 sections 4.1/4.2/anti-entropy | own amendment 2026-10-30 | Yes (model example) |
| preregistration_m6.md (v1) | preregistration_m6_v2.md | Only in v2; v1 has no pointer |
| v2 / v2.1 freeze JSON and v2.1 amendment validity | v2.2 records | Yes in amendments; JSON cannot carry notes |
| phase_1..5_completion_report.md PASS gates | accountability / ledger | NO |
| master_audit sec. 2 table (M5/M6 rows) | its later "Section E" and rounds | Partly (appended text; table unchanged) |
| claims_ledger table rows C1, C2, C7 | claims_ledger sec. 6 | Partly (appended; table unannotated) |
| progress_ledger sec. A/B/C | its sec. D | NO |
| phase_N execution packages "PROPOSED - not authorized" | actual execution | NO |
| phase_execution_packages_index.md ("Phase 1 is being executed") | - | NO |
| implementation_master_plan.md | everything after | NO |
| mvp_and_milestones.md (no M8) | phase_execution_packages_index.md (M8 only there) | NO (accountability: Rev 3 never applied) |
| architecture_investigation A1, ADR-001 sec. 1, code_findings statuses | CLAUDE.md corrections; later fixes | NO |
| architecture_trace_m3.md ("AUTHORITATIVE") | ADR-003 amendment, accountability sec. 12 | NO |
| intelligence_architecture.md, phase_5_preregistration.md | ledger C8 caveat | NO |
| phase_1_progress.md ("Last Updated 2026-10-28T19:10Z") | phase_1_completion_report | NO (log appears to stop early) |
| baseline_and_profiling_plan.md env facts (10-28) | later environment changes | NO |
| telemetry_schema_v2.md "IMPLEMENTED & VERIFIED" | D-0 telemetry honesty fixes | NO (no verification date) |

Frozen/sealed/append-only (do not edit; use dated amendments or supersession notes): preregistration_m6.md, preregistration_m6_v2.md, all freeze JSON, v2.1 and v2.2 amendments, addendum 1 (.md/.json/.pin), phase_5_preregistration.md, m6_sealed_results.md, g1/g2 verdicts, phase_*_completion_report.md and phase_*_progress.md (historical), phase_*_changes.patch, phase_1_snapshot_manifest.md, claims_ledger.md (append-only by its own rule), progress_ledger.md sec. D, novelty_snapshot_m6.md body, m6_research_evaluation.md, ADR-002/003 bodies, all evidence/ logs including evidence/superseded/, and lab-results/ (outside).

## 5. Placement, fragmentation and size

Over about 60 KB (suggestions only, nothing changed):
- phase_5_changes.patch 1,015 KB, phase_6_changes.patch 1,015 KB, phase_4 492, phase_3 490, phase_2 484, phase_1 140: 2.9 MB of generated diffs in a documentation tree. Suggest an archive/evidence location or snapshot hashes; owner decision (preservation rule).
- progress_ledger.md 139 KB: live dashboard plus append-only log in one file. Suggest a regenerated status file and a separate append-only log.
- phase_1_execution_package.md 98 KB: spec, approval items and correction history together.
- vision/ 00 (106 KB), v1 (79), v5 (69), v2 (73), v3 (61), v4 (60): about 450 KB; the consolidated report's own length weakens its summary role (note only; other agent's scope).
- phantomcache_milestone_accountability_report.md 64 KB: status report plus two topic reports (sec. 11 intelligence, sec. 12 M3 durability) appended.
- m3_durability/proposed_patch.diff 123 KB and proposed_patch_2_src.diff 83 KB (duplicate forms).
- Near threshold: master_audit 53 KB, phase_2_progress 52, phase_implementation_file_map 46, literature_review 41, E2 draft 37, work_packages 36, prior_art_coverage 33.

Placement:
- No README or index in docs/research/: 78 md files, flat, with five naming schemes (phase_N_*, preregistration_*, m6_*, g1/g2, intelligence_*) plus subdirs (adr, m3_durability, ttl_decision, vision, doc_audit).
- Product-engineering material in a research folder: phase_7/8 packages, demo_experience_plan, skill_strategy, ttl_decision/ and m3_durability/ (implementation proposals with patches), telemetry_schema_v2 and rfc9111_supported_subset (specs of shipped behaviour; fit docs/design or a spec folder).
- Decision records split: ADRs in adr/, other decisions as ledger log lines, g1/g2 verdicts, and ADR-shaped recommendation.md/design.md files outside adr/. No decision index. docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md is an earlier-era index.
- g1/g2 verdicts sit loose in research/ rather than a gates folder; the ledger refers to G3/G4.
- Evidence cited with absolute paths on D:\Temp (g2_verdict: `D:\Temp\q3_clippy.log`); the ledger records a storage cleanup at 11:40, so such evidence may be gone.
- Only one file uses a `_DRAFT` suffix; other proposals (m3_durability, ttl_decision, vision) have status only inside the text.

## 6. Authoritative-source map (recommended designation, owner to confirm)

| Category | De facto today | Recommended single authority | Same fact maintained independently in |
|---|---|---|---|
| Milestone status (M1-M8) | master plan sec. 2 (stale rows), ledger sec. C, accountability sec. 2, completion reports | one table with an "as of" date: accountability sec. 2 or a regenerated ledger table; completion reports annotated historical | master plan, ledger, accountability, 5 completion reports, CLAUDE.md |
| Claim status (C1-C13) | claims_ledger (table + appended sections) | claims_ledger.md, table reconciled by dated amendment | claims_ledger vs m6_sealed_results vs accountability sec. 11 vs master plan sec. 4 |
| Experiment protocol (M6) | v2 + v2.1 + v2.2 + addendum 1 | v2.2 amendment + addendum 1 as the effective protocol; others historical | five prereg docs, hypothesis_and_experiments |
| Experiment result (M6) | m6_sealed_results.md | m6_sealed_results.md + lab-results artifacts | claims_ledger sec. 6, accountability, ledger log |
| Next experiment protocol | E2 draft (v3) | the DRAFT until frozen, with a separate freeze record | - |
| Security finding status | docs/security/m7_security_review.md + ledger log | one findings register (outside this directory) | ledger, master plan, accountability, security docs |
| Configuration reference | src/config.rs, docs/user/configuration.md, CLAUDE.md sec. 5 | docs/user/configuration.md checked against src/config.rs; CLAUDE.md links only | CLAUDE.md, ttl_decision, user docs, ledger |
| Run / experiment history | ledger sec. D + lab-results + accountability | ledger sec. D (append-only) and evidence index sec. E | master plan appendices, per-phase progress logs |
| Decisions / ADRs | adr/ + ledger log + verdicts | adr/ plus a decision index; ledger links rather than holds decisions | ledger, g1/g2, recommendation.md files, handoff/07 |
| Limitations | docs/user/limitations.md, claims_ledger limits, accountability, CLAUDE.md sec. 6 | docs/user/limitations.md for users; claims_ledger for research limits | 4+ places |
| Distributed consistency contract | ADR-003 + amendment | same (add owner-approval status) | architecture_trace_m3, master plan 3.4, ledger, claim C6 |
| Serving-path contract | ADR-002 | ADR-002 | rfc9111_supported_subset, phase_2 report |
| Prior art / novelty | intelligence_prior_art_coverage + literature_review | those two; novelty_snapshot_m6 historical | novelty_snapshot_m6, candidate_comparison |
| Telemetry schema | telemetry_schema_v2.md | same, with a "last verified" date | phase_1 package 6.4.2, ui_metric_provenance |
| Agent working rules | CLAUDE.md | CLAUDE.md (short) | handoff/06 |
| Test baseline | none | one replaced line in ledger sec. A | see 2.2 |

## 7. Topic coverage matrix

| Topic | Documents (research/ unless noted) | Coverage |
|---|---|---|
| Product vision and requirements | mvp_and_milestones (MVP definition), vision/00 (proposal), root README | FEW: no requirements doc (functional, non-functional, SLOs, users) |
| Architecture | architecture_investigation, ADR-001/002/003, architecture_trace_m3, docs/handoff/03 | SEVERAL, partly stale; no current as-built doc in research/ |
| Technical specs | phase_N packages, telemetry_schema_v2, rfc9111_supported_subset, intelligence_architecture, docs/design/* | SEVERAL; specs embedded in phase packages |
| Research and prior art | literature_review, candidate_comparison, intelligence_prior_art_coverage, novelty_snapshot_m6, hypothesis_and_experiments | SEVERAL; RT-0 open |
| Intelligence / adaptive control | phantom_intelligence_phase, pie_audit, intelligence_architecture, phase_5_*, E2 draft, vision/v1, v2 | MANY (about 12); no current-state map |
| Distributed design | ADR-003, architecture_trace_m3, phase_3_*, m3_durability/, vision/v3 | SEVERAL |
| Security and threat models | research/: vision/v4 only; docs/security/*, docs/demo-lab/security-* (outside) | In other folders; research/ only references |
| API / integration | none in research/; docs/design/lab_api_contract.md, docs/user/lab_api.md (outside) | NONE here |
| Experiment specs / preregistration | preregistration_m6 family, phase_5_preregistration, E2 draft, hypothesis_and_experiments, benchmark_suite_spec | Well covered but fragmented |
| Runbooks | none in research/; docs/demo/scenario_b_runbook.md, lab_runner.md, docs/user/operate.md (outside) | NONE here |
| Benchmark methodology / results | baseline_and_profiling_plan, benchmark_suite_spec, refpolicy_golden_traces, phase_1 package 6.3; results only in untracked bench-results/ | Methodology yes; NO results document |
| Testing / QA | per-phase AC lists, ledger Stage A rounds, g1/g2, docs/testing/* (outside) | Scattered; no test strategy |
| Release / deployment | phase_7 package; docs/release/* (outside) | Plan here, artifacts outside |
| Operations / troubleshooting | none here; docs/user/operate.md, troubleshoot.md (outside) | NONE here |
| ADRs | adr/ (3) | Exists, no index; ADR-shaped docs outside adr/ |
| Demo lab and user docs | demo_experience_plan; docs/demo-lab/*, docs/demo/*, docs/user/*, docs/design/* (outside); vision/v5 | Many, in other folders |
| Contribution / development guidance | CLAUDE.md (agent rules), skill_strategy, phase index CER rules, handoff/06 | None for human contributors; no CONTRIBUTING at repo root |
| Governance: authorization / decision register | ledger log, g1/g2, accountability decision lists | Scattered; none |
| Glossary | none (notes on "Phase N = Milestone N", PIE naming repeated) | NONE |
| Document index / status register | none in research/ | NONE |

Topics with NO document in research/: index/status register, glossary, operations/troubleshooting, runbooks, API reference, contribution guide, test strategy, requirements, current as-built architecture, benchmark results report, authorization/decision register. Topics with several overlapping documents: milestone status (6), intelligence (about 12), preregistration chain (6 plus draft), architecture (5), gates/verdicts (3 styles).

## 8. Shape of the intelligence / benchmark / experiment / runbook set

- Intelligence: proposal (phantom_intelligence_phase) -> execution package (phase_5) -> spec (intelligence_architecture, labelled authoritative) -> pre-registration (phase_5_preregistration) -> completion report claiming PASS -> audit downgrade (ledger C8-C12: dispositions literal, machinery unwired). The only test of the adaptive hypothesis (M6 sealed) gave NOT_SUPPORTED (m6_sealed_results.md). Follow-ups: prior-art verification (RT-0 open), E2 headroom protocol (DRAFT, no seeds/salt), vision research. No single document says what intelligence code exists, where it runs (lab only), and what is proven; the reader must combine C8, accountability sec. 11 and the sealed results.
- Benchmark: plan (baseline_and_profiling_plan) -> catalog (benchmark_suite_spec) -> mandatory subset (phase_1 package 6.3) -> golden traces (refpolicy_golden_traces) -> outputs in untracked bench-results/. No results document in research/; accountability says no release-profile run exists; the 200x claim is untraceable (C5).
- Experiment: well-structured chain with hashes (freeze JSONs, pin, addendum), dated non-rewriting amendments. Weaknesses: five documents without a guide; v2.2 amendment has late-added sections inside it.
- Runbooks: Scenario B runbook and lab runner docs live in docs/demo/. Research documents cite commands by log path (some on D:\Temp), not by reproducible runbook.

## 9. Recommended actions

### Safe, docs-only (new files or additive dated notes; no frozen record modified)
1. Create `docs/research/README.md`: index and status register (every file, class, status, supersession pointer, reading order). New file only.
2. Add dated "status as of" banners to live/plan documents (not sealed): progress_ledger sec. A/C, phase_execution_packages_index, implementation_master_plan, mvp_and_milestones (M8 note), phase_1..5 execution packages (executed; authorization record absent; see accountability sec. 1).
3. Append dated supersession notices (ADR-003 / novelty_snapshot pattern) to phase_1..5_completion_report.md pointing to the accountability report. Appending is not rewriting, but these are historical records; if the owner treats that as modification, move this to owner decision.
4. Add a dated note beneath claims_ledger table rows C1/C2/C7 ("table rows predate sec. 6; sec. 6 governs") and reconcile the "Last Audited" line.
5. New files: `decisions_index.md` (ADR-001..003, G-ARCH, g1/g2, TTL and durability recommendations, owner decisions from the ledger) and `test_baseline.md` (date, command, profile, count, scope).
6. New reader's guide for the M6 protocol chain ("effective protocol = v2.2 + addendum 1; v1 voided, v2 and v2.1 freeze records invalid"), leaving all five documents unchanged.
7. New one-page "intelligence status map" (what is implemented, where it runs, what the sealed result says) citing C8/C11/C12 and m6_sealed_results.
8. Copy (not move) externally-pathed evidence logs (D:\Temp) into a repo evidence folder, after checking size and secrets.
9. New short benchmark-results note pointing at bench-results/ and stating "no release-profile run exists".
10. Ensure the README lists drafts and proposals (E2 draft, m3_durability, ttl_decision, vision) as class X.

### Owner decision
1. Which document is the milestone status authority (master plan, accountability, or regenerated ledger table); demote the others.
2. Whether and how to record retroactive owner authorization for phases 1-6 and ADR-003 (G-ARCH); whether ADR-001 is marked Superseded.
3. Whether the 2.9 MB of phase patches and the duplicate m3_durability diffs stay in docs/ or move to an archive (no deletion without owner say).
4. Whether to split progress_ledger and the accountability report; whether ttl_decision/ and m3_durability/ move to docs/design or adr/ once approved; whether to integrate the TTL and durability patches (application code, CLAUDE.md sec. 2).
5. Whether CLAUDE.md keeps restating status/config facts (sec. 5/6 carry "PARTLY OUTDATED" markers) or links to authorities.
6. Whether preregistration_m6.md (v1) and phase_5_preregistration.md receive an appended voided/superseded notice.
7. Closing RT-0 and whether M8 enters mvp_and_milestones as Rev 3.
8. Which test-count baseline to quote publicly.

### Must-not-touch (dated amendments or supersession notices only)
preregistration_m6.md, preregistration_m6_v2.md, preregistration_m6_v2_freeze.json, preregistration_m6_v2_1_amendment.md and freeze JSON, preregistration_m6_v2_2_amendment.md and freeze JSON, preregistration_m6_v2_2_addendum_1.{md,json,pin}, phase_5_preregistration.md, m6_sealed_results.md, lab-results/ and the sealed marker, all evidence/ logs and evidence/superseded/, phase_*_changes.patch, phase_1_snapshot_manifest.md, progress_ledger.md sec. D history, claims_ledger.md history, novelty_snapshot_m6.md body, m6_research_evaluation.md and phase_6_completion_report.md (retracted, retained as evidence), g1/g2 verdicts, ADR-002/003 bodies (amend only), CORRECTIONS.md files.

## 10. Top 10 issues
1. No index or status register in docs/research/ (78 md files in a flat folder).
2. Competing status authorities (master plan, ledger, accountability, completion reports) disagree on whether M1-M5 passed; five completion reports claim PASS with no supersession banner.
3. progress_ledger sections A-C are about 11 hours stale relative to its own log (M6 "OPEN/HOLD, R-03 NOT RUN" after the sealed run).
4. claims_ledger table still shows C1/C2/C7 as HYPOTHESIS pending R-03, while its sec. 6 says NOT SUPPORTED / NOT TESTABLE; "Last Audited" is 10-29.
5. Authorization gap: all phase packages read "PROPOSED - not authorized", ADR-003 is ACCEPTED by two agents, and no owner sign-off record exists.
6. Protocol lineage spread over five preregistration documents; v1 and phase_5_preregistration remain labelled FROZEN without a void/supersession note; no reader's guide.
7. Staleness bound stated as 50 ms (tests/ledger), 250 ms (ADR-003) and TTL 60 s (after crash) in different places; claim C6 unsupported; `PHANTOM_DEFAULT_TTL_MS` still unwired while a patch proposal sits in ttl_decision/.
8. Intelligence documentation (about 12 files) includes an "authoritative" spec for machinery not in the serving path (C8) and a hypothesis that the sealed run did not support, with no status map.
9. About 2.9 MB of cumulative-looking patch files and duplicate m3_durability diffs inflate the folder; progress_ledger (139 KB) and accountability report (64 KB) combine dashboards, logs and topic reports.
10. Missing documents: requirements, as-built architecture, test strategy, benchmark results report, glossary, decision register, operations/runbooks, and a contributor guide for humans (some exist in docs/user, docs/demo, docs/design).

## 11. Open questions
1. Did the owner authorize phases 1-6 in writing anywhere outside the repository docs (chat)? The accountability report says no document records it.
2. Are phase_N_changes.patch files cumulative or incremental? (Inferred from size only.)
3. Are v2.2 amendment sec. 7b/8b/8c covered by the v2.2 freeze hash (record written 05:06)? Needs a comparison of the freeze JSON with the document; not done.
4. Was the G2-D1 second independent check completed and recorded? Not seen in this directory.
5. Is ADR-001 meant to become Superseded by ADR-003 or remain the historical "present state" ADR?
6. Is the master plan sec. 2 table meant to remain an audit snapshot (needs an "as of" label) or be the living status table?
7. Should evidence referenced by D:\Temp paths (and affected by the 11:40 storage cleanup) be preserved in-repo?
8. Are root `walkthrough.md` and `MILESTONE_1_DESIGN_PLAN.md` still current? (Out of scope; research docs call the latter an "early, different Milestone 1".)

Limits: top-of-file reads and greps for most documents; items marked inference or UNRESOLVED were not verified; no code or retained evidence artifacts were opened.
