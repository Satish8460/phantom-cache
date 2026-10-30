# D4 - Mechanical documentation checks

Date: 2026-10-30. Agent D4 of 4. Read-only on the repo apart from this file.
Evidence tag for every number below: **reproduced here** by script run (not hand-counted). Scripts live outside the repo in
`D:\ClaudeTemp\claude\d--Personal-phantom-cache\d45f6980-8d59-4418-9545-e9f87e8aef66\scratchpad\doc_audit\`:
`audit.mjs` (all checks, writes `out.json`), `moved.mjs` (moved-vs-never-existed heuristic), `audit_lowsim.mjs` (same as audit.mjs, similarity thresholds lowered to 0.2 Jaccard / 0.5 containment, writes `out_lowsim.json`).
Re-run: `node audit.mjs out.json` (about 5 s).

## 0. Scope and limits

- Scanned: **162 markdown files, 2,337,215 bytes** (median 9,899 bytes).
- Excluded: `.local/` (763 snapshot .md files, not docs), `lab-results/`, `node_modules/`, `dist/`, `target/`, `.git/`, `docs/demo/browser_verification/`, and the in-flight outputs of the other audit agents (`docs/research/vision/`, `docs/research/doc_audit/`).
- Not detected: links in non-.md docs, HTML, or fenced code blocks (fenced code is stripped before scanning). Reference-style links are not parsed (none seen).
- "Never existed" vs "moved/deleted" cannot be settled from git: the repo has 2 commits and most of the docs and tests are untracked, so `git log -- <path>` returned nothing for all 48 distinct missing targets. I therefore used a basename search of the working tree (`moved.mjs`) as a proxy for "moved".
- Tick-path detection is heuristic (inline backticked strings that look like repo paths, with a known top-level prefix or `./`/`../`). Treat counts as +/- a few percent.

## 1. Links and path references

2,771 references checked (101+209+1+... markdown links; 2,248+ backticked paths; 7 external http links, not fetched).

| Class | Count | Notes |
|---|---:|---|
| Backtick path, resolves | 2,248 | |
| Markdown link, relative, resolves | 101 | |
| Markdown link, `file:///d:/Personal/phantom-cache/...` (machine-absolute, target exists in repo) | 209 | Works only on this machine; not portable. 8 files. |
| Backtick/`D:\...` path **outside repo** (build dirs, `D:\Temp` logs, snapshots) | 126 in 36 files | 23 are the intentionally-historical deleted build dirs (`_m7_target*`, `_m3_target`, `_ttl_target`, `D:\CargoTarget\m7_clean`, `phantom-cache\target`). Rest are `D:\Temp\*.log`, `D:\Personal\phantom-cache-snapshots\...`, scratch dirs. |
| Markdown link, relative, **target missing** | 9 | 7 are `../src/...#Lnnn` in `docs/research/architecture_trace_m3.md` (wrong depth: one `..` from `docs/research/` lands in `docs/src/`; should be `../../src/`), 1 in `phase_2_completion_report.md` (`../src/core/freshness.rs`, same bug), 1 regex mis-parse in `docs/security/phase-3-design.md` (`[a-z0-9-]{1,30}[a-z0-9]`, not a real link). |
| Backtick path, **target missing** | 67 | see below |
| Anchor missing | 1 | `docs/user/security.md` -> `configuration.md#cors-allowlist-phantom_cors_origins` |
| In-repo absolute that is missing | 1 | `D:\Personal\phantom-cache\target` (deleted 2026-10-30, intentional) |

Classification of the 48 distinct missing relative targets (heuristic):

- **Moved** (same basename exists elsewhere): `../src/core/freshness.rs` (wrong depth), `tests/security_phase1_tests.rs` -> `tests/integration/security_phase1_tests.rs`, `bench-results/README.md` (only other READMEs exist).
- **Never existed or planned-but-not-built** (no basename match anywhere; could equally be deleted, undecidable): `tests/security/test_sec_0{1..9}_*.rs` (about 9 names, planned SEC tests), `tests/security/`, `tests/common/mock_origin.rs`, `scripts/gen-certs.sh`, `src/control/{policies,canary,rollback}.rs`, `src/api/lab.rs` (real file is `src/api/lab_api.rs`), `src/dist/durable_log.rs`, `src/lab/e2/`, `src/lab/analysis`, `benches/hot_path.rs`, `tests/integration/{eviction_policy,concurrency_model,ingress_upstream_robustness,default_ttl_wiring,m3_invalidation_durability}_tests.rs`, `docs/design/{reference,prototypes,screenshots}/`, `ui/prototypes/`, `lab-results/e2_sealed_run_started.marker`, `bench-results/wsl2/cpu_map.json`.
- **Shorthand, not a path**: `docs/handoff/00|01|03|04|05|06|08` (14 refs in `docs/handoff/CORRECTIONS.md`), the regex above, `src/lib`.
- Several of these (the `lab-results/` marker, `bench-results/wsl2/` json) may be gitignored or sealed artifacts on purpose; unverified.

Top offenders (broken or non-portable refs per file; includes the 209 `file:///` links):

| File | Total | Dominant class |
|---|---:|---|
| docs/security/phase-4-design.md | 48 | file:/// absolute |
| docs/research/progress_ledger.md | 39 | outside repo (D:\Temp logs, snapshots) |
| docs/security/trust-model-audit.md | 20 | file:/// absolute |
| docs/handoff/MANIFEST.md | 19 | file:/// absolute |
| docs/research/storage_cleanup_2026-10-30.md | 19 | outside repo; **intentionally historical** (deleted dirs) |
| docs/demo-lab/capability-audit.md | 17 | file:/// absolute |
| MILESTONE_1_DESIGN_PLAN.md | 15 | file:/// absolute |
| docs/handoff/CORRECTIONS.md | 14 | shorthand `docs/handoff/NN` |
| docs/research/phase_2_progress.md | 14 | file:/// absolute + outside |
| docs/research/master_audit_and_execution_plan.md | 12 | outside repo (`D:\Personal\_m7_target`...) historical |
| docs/handoff/00_START_HERE.md | 11 | file:/// absolute |
| docs/security/phase-3-design.md | 10 | file:/// |
| docs/security/security-demo-matrix.md | 8 | missing `tests/security/test_sec_*` |
| docs/research/architecture_trace_m3.md | 7 | wrong-depth `../src` |
| docs/research/phase_implementation_file_map.md | 7 | missing |

Full list of every broken reference: `out.json` -> `links.allBroken`.

## 2. Absolute machine-specific paths

417 occurrences in **60 files** (after excluding URL false positives).

| Pattern (generalised) | Occurrences |
|---|---:|
| `d:\Personal\phantom-cache\{src,docs,ui,tests}` (the file:/// links) | 207 |
| `D:\Personal\phantom-cache-snapshots\<phase-timestamp-sha>` | 25 |
| `D:\Personal\phantom-cache` (bare) | 12 |
| `D:\Personal\_m7_target*`, `_m3_target`, `_m7_tmp` | about 20 |
| `D:\Temp\*` (logs, scratch) | about 60 |
| `D:\CargoTarget\m7_clean` | 5 |
| `D:\Personal\.phantom-sealed-m6\...` (sealed-run directory) | 2 |
| `D:\ClaudeTemp` | 2 |

Top files: progress_ledger.md 93, phase-4-design.md 48, intelligence_prior_art_coverage.md 44 (to verify: likely URLs/drive-like tokens), capability-audit.md 27, trust-model-audit.md 26.

Secrets-adjacency (patterns only, no values printed):
- **User name / identity**: no `C:\Users\<name>` path found. The owner's GitHub handle appears in repo URLs (9 occurrences, `github.com/<handle>/phantom-cache`), and personal strings ("Satish", the owner e-mail handle) appear in README.md (5), docs/PHASE_2_WORK_REPORT.md (5), phase_1_snapshot_manifest.md (2), progress_ledger.md (6). This is identity, not credential.
- **Salt / key**: 2 references to a path of the form `D:\Personal\.phantom-sealed-m6\m6_salt.<ext>` (a sealed-experiment salt file location outside the repo). 15 lines in 7 files co-locate a drive path with the words salt/key/secret (progress_ledger.md 11; phase_1_* 3; preregistration_m6_v2*.md 2; intelligence_e2_headroom_protocol_DRAFT.md 1). I saw only paths and file names, not contents; reviewer should confirm none of these lines embeds a hash or value (not checked line by line). Directory name of the salt location is itself operationally sensitive (it tells an attacker where the sealed value sits) but it is on a local disk outside the repo.
- Other: `169.254.169.254` metadata IP and `192.168.1.100` appear as SSRF/demo examples (not secrets). `.local/snapshots` is excluded from scan but holds 763 .md copies of earlier docs; the same paths probably recur there.

## 3. Status vocabulary

Only **53 of 162** files have a parseable `Status:` line in the first 40 lines; 109 do not. Header status values are free-text, e.g. `PROPOSED (awaiting owner approval)`, `ACCEPTED (Gate G-ARCH Resolution)`, `Accepted`, `COMPLETE - Rule R-08 FIRED`, `AUTHORITATIVE SPECIFICATION`, `AUTHORITATIVE CODE-LEVEL ARCHITECTURAL TRACE`, `AUTHORITATIVE EVIDENCE-BACKED AUDIT CORRECTION`, `PROVISIONAL, ...`, `PLAN, ...`, `DRAFT. NOT FROZEN.`, `PARTIAL. RT-0 is NOT closed.`, `INVESTIGATION + PROPOSED DESIGN`, `SPECIFICATION (2026-10-30)`, `CONTRACT ONLY (2026-10-30)`. Roughly 45 distinct header values for 53 files: no controlled vocabulary.

Body-text term frequency (occurrences / files; case-sensitive as written):

| Term | Occ | Files | Example files |
|---|---:|---:|---|
| PASS | 588 | 46 | phase_4_completion_report (55), phase_2_completion_report (46) |
| Verified | 223 | 52 | phase_5_progress (32), phase_1_progress (19) |
| VERIFIED | 122 | 15 | phase_implementation_file_map (54) |
| COMPLETE / COMPLETED | 103 / 55 | 20 / 14 | phase_6_progress, phase_4_progress |
| PROPOSED / Proposed | 87 / 21 | 22 / 10 | phase_implementation_file_map (61), handoff/05 (11) |
| Active | 79 | 39 | phase-3-design (6) |
| UNVERIFIED | 56 | 14 | claims_ledger (21) |
| PARTIAL | 34 | 6 | literature_review (18) |
| FAIL | 30 | 9 | security-scenario-catalogue (10) |
| ACCEPTED / Accepted | 28 | 12 | handoff/02 (9), handoff/00 (5) |
| BLOCKED | 22 | 12 | progress_ledger |
| REPRODUCED | 21 | 7 | code_findings (9) |
| IMPLEMENTED | 20 | 6 | progress_ledger (10), milestone_accountability_report (5) |
| STALE | 17 | 10 | |
| NOT STARTED | 17 | 3 | progress_ledger (11) |
| OPEN | 16 | 8 | progress_ledger (9) |
| TESTED | 13 | 3 | milestone_accountability_report (8) |
| FIXED / RESOLVED | 12 / 11 | 4 / 4 | phase_1_execution_package (7), phase-4-final-acceptance-report (6) |
| RETRACTED | 6 | 4 | progress_ledger (3), m6_research_evaluation |
| Superseded / SUPERSEDED | 4 / 4 | 2 / 4 | initial_assessment (3), ag_task_WP-00 |
| Draft / DRAFT | 1 / 5 | 1 / 2 | |
| INDEPENDENTLY VERIFIED | 3 | 2 | progress_ledger, milestone_accountability_report |
| CLOSED | 2 | 2 | |
| Archived | 0 | 0 | the term does not exist |
| OUTDATED / PARTLY OUTDATED | 2 / 1 | 1 | CLAUDE.md only |

Findings:
- The canonical lifecycle in the brief (Draft/Proposed/Active/Verified/Superseded/Archived) is not what the docs use. "Archived" never appears; "Superseded" is in 4 files; "Active" is mostly ordinary prose.
- Case variants are used as different values: Verified (52 files) vs VERIFIED (15) vs UNVERIFIED (14), Proposed vs PROPOSED, COMPLETE vs COMPLETED, Accepted vs ACCEPTED.
- Per-file co-occurrence of opposed terms: PASS+FAIL 8 files; VERIFIED+UNVERIFIED 3; INDEPENDENTLY VERIFIED+UNVERIFIED 2; INDEPENDENTLY VERIFIED+RETRACTED 1; IMPLEMENTED+NOT WIRED 2; Active+Superseded 1. Co-occurrence is not proof of conflict; the candidates to review are `progress_ledger.md`, `phantomcache_milestone_accountability_report.md`, `claims_ledger.md`, `master_audit_and_execution_plan.md`.
- Status conflict by design: the handoff header (`docs/handoff/00_START_HERE.md`) says "Phase 4 Remediated, Verified, and Formally Accepted (242/242...)" while `CLAUDE.md` and `docs/research/*` say this is not supported; `docs/handoff/CORRECTIONS.md` is "AUTHORITATIVE" and so is `architecture_trace_m3.md` and `intelligence_architecture.md`: three documents claiming AUTHORITATIVE with different scopes and no precedence rule.

## 4. Dates

- 99 of 162 files contain any date; **63 contain none** (all of `docs/user/*` except limitations, all of `docs/implementation/*`, `docs/testing/*` 4 of 5, most `docs/demo-lab/*`, `docs/release/*` 4 of 5, `CORRECTIONS.md`, `MILESTONE_1_DESIGN_PLAN.md`, `walkthrough.md`, `docs/security/*` 9 files). Full list: `out.json` -> `dates.noDate`.
- 89 files have a date within the first 25 lines; 73 have a "last updated/date:/as of/dated" style header marker.
- Formats (files using): ISO `YYYY-MM-DD` 71; ISO with time `...T..:..` 29; `Month D, YYYY` 13; `D Month YYYY` 5; compact `YYYYMMDD` 6 (snapshot names such as `phase3-20261029T094500Z-...`); `D/M/YYYY` 0. Four date formats in use.
- ISO dates by month: 2026-10 406 occurrences; 2026-08 5, 2026-04 3, 2026-05 1, 2026-09 1, 2020-01 1 (the last few are likely literature references or examples, not checked). Essentially the whole corpus is 2026-10-xx.
- CLAUDE.md quotes a baseline "2026-10-28"; `Today` is 2026-10-30; no doc carries a machine-readable `last_updated` field.

## 5. Terminology variants (occurrences / files)

| Concept and variants | Counts | Notes / examples |
|---|---|---|
| phase vs milestone vs WP | "phase" 700/94; "Phase N" 465/74; "milestone" 158/43; "M#" 2,162/76; "WP#" 520/42; "work package" 38/24 | `phase_implementation_file_map.md` (M# 286), `progress_ledger.md` (M# 172), `work_packages.md` (WP 132). Three overlapping numbering schemes (Phase 1-8, M1-M10, WP-00..WP-37). M-ids seen: M1-M8 (156-389 each), M9 2, M10 3. |
| WP id forms | `WP-NN` (zero-padded, hyphen; research series, 33 distinct ids) vs `WP1..WP5` (no hyphen; `docs/implementation/` older series) | **Collision risk**: `WP-02` (research) and `WP2.md` (implementation) are different things; in-text `WP2` appears 26 times. |
| PIE vs Phantom Intelligence | "PIE" 387/59; "Phantom Intelligence" 54/25; "Policy Intelligence Engine" 5/4 (only pie_audit, CLAUDE.md, 2 more) | The expanded name is almost never spelled out; legacy-stub PIE and new PIE v2 share the acronym (CLAUDE.md warns of this). |
| Tier labels | T1 157/26; T2 118/24; T3 44/7 (mostly the E2 protocol draft); "Tier N" 63/20; MEASURED_LOOPBACK 13/11; "MEASURED" bare 16/8; SIMULATED 49/29; simulated (lc) 153/59; INFERRED 2/2; UNAVAILABLE 10/8 | Two independent tier systems: **T1/T2/T3** (evidence tiers, research docs) and **MEASURED_LOOPBACK/SIMULATED/INFERRED/UNAVAILABLE** (UI/telemetry labels). CLAUDE.md names four UI labels (measured, simulated, inferred, unavailable); only SIMULATED is widely used, INFERRED almost never; `MEASURED` alone vs `MEASURED_LOOPBACK` is inconsistent. |
| tenant vs namespace | tenant 400/62; namespace 5/3 | Effectively settled on "tenant". |
| node vs peer | node 1,021/105; peer 301/53 | Both used for the same remote entity; `m3_durability/design.md` (peer 28) and the accountability report (peer 23). Not formally defined. |
| epoch vs generation | epoch 190/47; generation 139/51 | Distinct in code (tenant/global epoch vs per-key generation) but "generation" also means "generation of docs/artifacts" in MILESTONE_1_DESIGN_PLAN.md (21), so the word is ambiguous. |
| modes | "live mode" 28/16; "sim mode" 6/4; "cluster mode" 9/6; "demo mode" 2/2; "lab mode" 2/2; "insecure cluster" 4/4 | No single mode glossary; UI doc names (`ui_audit.md`, `phantom_lab_ui.md`) differ from backend names. |
| SEC ids | 212 occurrences / 28 files / 38 distinct ids | Id schemes: `SEC-4A-nn`, `SEC-4C-nn`, `SEC-M7-nn`, `SEC-POA-nn`; no registry file. |
| Priority labels | P0 24/15; P1 36/12 | |

Full table: `out.json` -> `terms`.

## 6. Headings, file names, duplicates

Headings (code fences stripped):
- **No H1: 17 files**: README.md, walkthrough.md, docs/demo/README.md, all 9 `docs/implementation/*`, and 5 of 5 `docs/testing/*` that have no H1 (`ENVIRONMENT_READINESS_REPORT`, `FLAGSHIP_VALIDATION_REPORT`, `V2_RUNTIME_AUDIT`, `V3_...`, `V4_...`). Possible cause for README.md: HTML title block; not checked.
- **Multiple H1: 3 files**: docs/PHASE_2_WORK_REPORT.md (2), docs/research/master_audit_and_execution_plan.md (2), docs/research/work_packages.md (2).
- **Skipped levels (e.g. H2 -> H4): 1 file**: docs/PHASE_2_WORK_REPORT.md. (Heading structure is otherwise tidy.)
- First heading is not H1: 1 file.

File naming (162 files):
- lower_snake 96 (all of `docs/research`, `docs/demo`, `docs/design`, `docs/handoff` body, `docs/release`, `docs/testing`'s ones with V#); lower-kebab 24 (`docs/security/*` 16 + `docs/demo-lab/*` 8 + `docs/research/adr/*` 3 are ADR-NNN-kebab); UPPER_SNAKE 26 (root files, handoff, `docs/implementation/*`, `docs/testing/*`); numeric-prefixed `NN_NAME` 9 (handoff only); mixed-case 6 (`ADR-001-...`, `scenario_B_origin_slowdown.md`, `intelligence_e2_headroom_protocol_DRAFT.md`, `ag_task_WP-00.md`); 2 names mix `_` and `-` (`ag_task_WP-00.md`, `storage_cleanup_2026-10-30.md`).
- Per-directory convention: **`docs/security` is kebab, `docs/research` is snake, `docs/demo-lab` is kebab but sibling `docs/demo` is snake** (and `demo` vs `demo-lab` is itself a near-duplicate directory pair). No file has a date prefix; dated names are rare (`storage_cleanup_2026-10-30.md`).
- Same basename, different directory (4 groups): `CORRECTIONS.md` (root) vs `docs/handoff/CORRECTIONS.md`; `README.md` x5; `implementation-roadmap.md` in `docs/demo-lab/` and `docs/security/`; `test_plan.md` in `docs/research/m3_durability/` and `docs/research/ttl_decision/`.
- Root vs handoff `CORRECTIONS.md`: different content (not identical hash), so it is a duplicate **title**, not duplicate content; which is authoritative is stated nowhere I checked.

Duplicate content:
- Identical SHA-256: **0 pairs**.
- Word-5-gram shingle Jaccard >= 0.6: **0 pairs**; containment >= 0.8: 0; even at thresholds lowered to 0.2 Jaccard / 0.5 containment: 0 pairs. (Notable: the overlap between phase progress reports and completion reports is below 0.2, so they are paraphrased rather than copied.) Repetition is in facts (section 7), not text.

## 7. Repeated facts that can drift

**Test counts** (chronology is in the docs themselves; the stated "current" number differs by doc):

| Count | Where stated | Context |
|---|---|---|
| 242 (and "19 targets") | 27 files incl. CLAUDE.md, README.md, handoff/00,02,04,07,08, CORRECTIONS (x2), MANIFEST, ADR-001, phase-4-final-acceptance-report (7), phase_1_* | Reported Phase-4 baseline 2026-10-28. README.md and CLAUDE.md still present it as the baseline; docs/research later numbers supersede it. |
| 303-359 | phase_2_progress (running tally) | Phase 2 |
| 388 / 398 / 435 | phase_3/4/5 progress and completion reports | Phase 3-5 |
| 499 (58 targets) | master_audit_and_execution_plan, accountability report, phase_6_progress/completion, progress_ledger | Phase 6 / "audit log" |
| 595 / 613 / 623 | master_audit_and_execution_plan | audit runs, one run `exit 101, 1 failed` |
| 774 | accountability report | "last full lead sweep" |
| 802 | g1_verdict | |
| 820 | progress_ledger (1), phantom_lab_ui (1, unrelated: viewport px) | |
| 841 | g2_verdict (2), progress_ledger, recompute_report (2) | "75 targets, 841 passed, 8 ignored" |
| 851 | progress_ledger (3), recompute_report (3) | "75 targets, 851 passed, 8 ignored", the latest in the corpus |

Only **2 files** (progress_ledger.md, scenario_b_verification/recompute_report.md) state 851; README.md, CLAUDE.md and 25 other files still state 242 as the suite size. All of these are debug-profile counts and several also differ in whether benches/`harness=false` mains are included (per the accountability report). No single place records "current test count + date + command + commit".

**Other repeated numbers**: `86,400,000` (24 h cap) in 7 files; `60,000 ms` default TTL in 7 files and "60 s" in 37 files; `2,688` histogram buckets in 3 files; `0.39%` error bound in 1 file (phase_2_progress) - CLAUDE.md quotes it, one primary source. `PHANTOM_DEFAULT_TTL_MS` NOT-WIRED status is stated in CLAUDE.md and `docs/research/ttl_decision/recommendation.md` etc.; docs/user/configuration.md and troubleshoot.md state the default without the not-wired caveat in the lines I matched (not read in full).

**Verdict/tier words by file count**: PASS 46; SIMULATED 29; SUPPORTED 14; ACCEPTED 12; MEASURED_LOOPBACK 11; FAIL 9; REFUTED 7; NOT SUPPORTED 6; INCONCLUSIVE 5; RETRACTED 4; CONDITIONAL 2; INDEPENDENTLY VERIFIED 2; GO/NO-GO 0.

**Claim ids** C0..C13 (claims_ledger.md rows C1-C13): mentions across files - C1 15, C6 15, C5 12, C4 11, C3 9, C2 8, C7 8, C8 6, C12 6, C9/C11/C13 5, C10 3. `claims_ledger.md` holds the rows plus a second table "audit re-review" in which C3, C9, C11, C12 are `UNVERIFIED`, C4/C5/C6/C8/C10/C13 `Not yet re-reviewed`, then a later table where C1 is `NOT SUPPORTED (narrowed to Tier-1 simulation)`, C2 `NOT TESTABLE`, C7 `UNVERIFIED`. The same claim id therefore carries up to three statuses in one file, plus `master_audit_and_execution_plan.md` and `phantomcache_milestone_accountability_report.md` restate them. The pattern "C#" also matches unrelated tokens, so counts are upper bounds.

**SEC findings**: 38 distinct ids. Five ids appear in 4-7 files with more than one status token on their lines: SEC-M7-01 (OPEN/PARTIAL/FIXED, 7 files), SEC-M7-02 (OPEN/FIXED, 5), SEC-M7-03 (OPEN/FIXED, 5), SEC-M7-09 (OPEN/ACCEPTED/FIXED, 5), SEC-M7-11 (OPEN/FIXED, 4). This is a line-level word match, so some are history ("was OPEN, now FIXED"); these five are where a drift check should look first. Files: m7_security_review.md (source), progress_ledger.md, master_audit_and_execution_plan.md, accountability report, `docs/release/deferred_integration.md`, `docs/design/sse_auth_options.md`, `docs/user/security.md`. Most-cited: SEC-4A-01 (8 files), SEC-M7-01 (7).

## 8. Size outliers

- Over 60 KB (4): `docs/research/progress_ledger.md` 140,512 B; `docs/research/phase_1_execution_package.md` 97,721; `docs/research/phantomcache_milestone_accountability_report.md` 64,248; `docs/PHASE_2_WORK_REPORT.md` 63,395. Next: master_audit_and_execution_plan 53,545; MILESTONE_1_DESIGN_PLAN 52,809; phase_2_progress 51,878; phase-4-design 49,659.
- Under 200 bytes: **0**. (Smallest files not an issue.)

## 9. Not linked from any index

Index-like entry points found by name: README.md, docs/demo/README.md, docs/demo-lab/README.md, docs/demo/scenario_b_verification/README.md, docs/user/README.md, docs/handoff/00_START_HERE.md. (`docs/research/` has **no README/index** by that name; `phase_execution_packages_index.md` is topical.)

- **Not linked from any of those index files: 123 of 162** (dir breakdown: docs/research 72, docs/security 16, docs/implementation 9, docs/design 8, docs/testing 5, docs/demo 4, docs/release 4, others 1 each).
- **No inbound reference from any other markdown file by path or link: 30**, including 25 that are not mentioned even by basename: all of `docs/implementation/*` (FLAGSHIP_DEMO_GUIDE, WP1, WP3, WP3_GUIDE, WP4, WP4_GUIDE, WP5), `docs/milestone_1/WORK_NOTES.md`, `docs/release/ci_recommendations.md`, `release_notes_template.md`, `docs/research/phase_5_completion_report.md`, `preregistration_m6_v2_2_addendum_1.md`, 8 files in `docs/security/` (authenticity-design-options, phase-2-red-team-review, stage-3c-progress, stage-4b-audit-report-round2, stage-4b-progress, trust-boundary-map, trust-model-audit, plus implementation-roadmap by path), 5 in `docs/testing/`, and `walkthrough.md`. Further 5 with only basename mentions: `docs/demo-lab/README.md`, `docs/demo/scenario_b_runbook.md`, `scenario_b_verification/README.md`, `docs/research/ttl_decision/recommendation.md`, `docs/security/implementation-roadmap.md`.
- `docs/research/phase_5_completion_report.md` being unreferenced while phase 1-4 and 6 completion reports are referenced is a likely oversight.

## Proposed docs-lint (PROPOSAL, not implemented)

A single dependency-free Node script (`scripts/docs-lint.mjs`, built from `audit.mjs`), run locally and in CI, reading an allowlist `docs/.doclint.json`. Exit codes: `0` clean (or only warnings), `1` any error-level finding, `2` tool failure/bad config. Output machine-readable (`--json`) plus a count summary; a `--baseline` mode records current findings so new violations fail while old ones ratchet down.

| Rule | Level | Check |
|---|---|---|
| L1 broken-relative-link | error | Markdown link target missing, or anchor missing (GitHub slug rules). |
| L2 broken-path-ref | warn, error for non-historical docs | Backtick repo-path (known top-level prefix) that does not exist. |
| L3 machine-absolute-path | error | `file:///`, `[A-Z]:\` or `/Users/` paths in docs; autofix hint: rewrite to repo-relative. Allowlisted only inside a fenced "historical record" block or a doc flagged `historical`. |
| L4 secret-adjacent-path | error | Path containing `salt`, `key`, `secret`, `.pem`, `sealed` outside the allowlist; never print the matched file contents. |
| L5 front-matter | error for new docs | Required header lines: `Status:` (controlled set: Draft, Proposed, Accepted, Active, Verified, Superseded, Archived, Historical), `Last updated: YYYY-MM-DD`, optional `Supersedes:`/`Superseded-by:`. Case-sensitive enum. Superseded requires a `Superseded-by` link that resolves. |
| L6 date-format | warn | Only ISO `YYYY-MM-DD`; flag `Month D, YYYY`, `D/M/YYYY`. |
| L7 heading | warn | Exactly one H1, first heading is H1, no skipped levels. |
| L8 filename | warn | Per-directory convention declared in config (e.g. research = lower_snake); duplicate basenames across directories flagged. |
| L9 orphan | warn | Every .md reachable from a declared index (README.md or docs/INDEX.md) within N hops. |
| L10 size | warn | > 60 KB (split or mark archival). |
| L11 fact-drift | error | A small registry `docs/.facts.json` of canonical facts (current test count, command, commit, date; claim-id status; SEC-id status). The linter greps for the numbers in the "stale set" (e.g. 242, 774, 841) in non-historical docs and requires either the canonical value or an adjacent `<!-- fact:test_count -->` marker. Claim and SEC status tables must come from one file; other files may only link. |
| L12 vocabulary | warn | Status words used in tables must belong to the controlled set; flag case variants (Verified/VERIFIED); require tier labels exactly `MEASURED_LOOPBACK`/`SIMULATED`/`INFERRED`/`UNAVAILABLE` in UI/telemetry docs. |

Historical-document handling: the allowlist assigns each path or glob a mode - `historical` (skip L2, L3, L11; require the first lines to carry a banner `Historical record, not current`; still run L1 on repo-internal links), `archived` (skip all except L5 status = Archived/Superseded), or `current` (all rules). Seed `historical` with: `docs/research/storage_cleanup_2026-10-30.md`, `docs/research/progress_ledger.md`, `docs/research/phase_*_progress.md`, `docs/research/phase_*_completion_report.md`, `docs/research/phase_*_execution_package.md`, `docs/handoff/**` (until reconciled), `docs/PHASE_2_WORK_REPORT.md`, and an explicit pattern list of intentionally-deleted build dirs (`_m3_target`, `_m3_target_base`, `_ttl_target`, `_m7_target*`, `D:\CargoTarget\m7_clean`, `phantom-cache\target`, `D:\Temp\*`) that L3 treats as known-gone. Each allowlist entry needs a `reason` and an `expires` date so the list ratchets down.

Suggested rollout: step 1 run in report-only mode with the baseline (all current findings recorded); step 2 enforce L1, L3, L4 on `current` docs first (about 10 of the 15 top offenders are fixable by repo-relative rewrite of the 209 `file:///` links); step 3 introduce L5/L11 for new or touched docs only. Generated `docs/research/doc_audit/` outputs should be excluded to avoid self-reference.

## Housekeeping

No repo file was modified except this one. Git HEAD observed at start: `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 72 status lines at start.
