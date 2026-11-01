# Redesign execution, agent and critical-file tracker

**Status:** Active. **Audience:** product owner (primary), lead, agents. **Last updated:** 2026-10-30 18:28 (+05:30, machine clock). **Reads in two minutes.** Decision status lives ONLY in `docs/design/OWNER_DECISION_REGISTER.md`; this file links to its IDs. The detailed narrative is in `docs/design/remediation/07_frontend_redesign_implementation_record.md` (not replaced by this file). Evidence labels: REPORTED = an agent said so; VERIFIED = the lead opened the file or ran the check (what was checked is named); nothing here is independently verified by a human.

## Dashboard
| Metric | Value |
|---|---|
| Current redesign phase | Phase B RUNNING (V2A Lab and evidence screens; V2B Simulation, other pages and app wiring); Phase A delivered |
| Overall status | ON TRACK, no redesign work blocked; machine is busy (M3 and handbook agents), so performance measurement is deferred |
| Most important blocker | DEC-005 (offline check by the owner) holds the preregistration files, rng.rs and the M6 test; Rust commits are DEFERRED (DEC-022) until DEC-005 and the G4B-01 and G4A-01 fixes are validated and reviewed |
| Active agents | running: 1 redesign (U-01 independent reviewer), 1 backend (M3), 1 commit planning (BATCH), 3 handbook QA (A, B, C) |
| Completed workstreams (redesign) | 6 of 7 (TRACE, FLIB, V1, Toolkit and accessibility setup, Governance setup, U-01 patch review) |
| In-progress workstreams (redesign) | 0 running; U-01 patch AWAITING OWNER (DEC-001); Phase B not started |
| Blocked workstreams | 0 (Phase B is scheduled after V1, not blocked) |
| Critical pending files | 8 (importance High or Critical and not yet complete; section C) |
| Pending independent reviews | 0 running; U-01 review DELIVERED (INTEGRATE AFTER FIXES); 2 not yet requested (M3, G4A-01 fix proposal) |
| Pending owner decisions | 1 awaiting your action (DEC-005: offline check, two Booleans); 7 not ready (DEC-002, 003, 004, 026, 027, 028, 029); 4 deferred (DEC-021, 022, 024, 025) |

Repository state (VERIFIED by lead at 16:13): branch `feat/publish-phantomcache-core`, HEAD `caeedfd`; 111 collapsed status lines (about 4,500 entries under `git status --short -uall`, mostly evidence). Commits this programme: `8523cc1` (apple-design skill), `3e6e03e` (docs-lint), `caeedfd` (four Emil skills). Nothing else staged.

## A. Agent and workstream ownership
| Agent / workstream | Responsibility | Files owned | Status | Current activity | Dependencies | Next deliverable |
|---|---|---|---|---|---|---|
| V1 (agent af787a4cac69caf8f) | Design tokens, shell, materials, motion foundations | `ui/src/styles/{tokens,app,shell,pages}.css`; `components/{Shell,Panel,Metric,States,Icon,ProvenanceBadge}.jsx`; `scripts/check-contrast.mjs`; new `shell.*`, `tokens.*`, `motion.*` tests | COMPLETE (REPORTED). Lead VERIFIED: UI suite 483 of 483, contrast 284 of 284, a11y 0 issues, forbidden pass; 3 new test files registered | done | none | lead diff review; Phase B uses `.stamp`, `.hatch`, `--radius-card`, `--surface-3`; no backdrop-filter outside chrome |
| FLIB (agent a3df17bca586530a6) | U-01 UI logic, connection states, gateway address, honest progress | new `ui/src/lib/{resultPresentation,linkState,gatewayConfig,progress}.js`; edits to `lib/{connection,labView,labApi,labStream,config}.js` and `hooks/*` where specified; `func.*` tests and fixtures | COMPLETE (REPORTED): 159 new tests pass; lead VERIFIED: 5 test files registered, whole suite 455 of 458 pass | done | none | wiring by Phase B using the wiring spec in `increment_F_functional_logic.md`; lead decisions AD-8 to AD-11 |
| TRACE (agent a458eab1c65568f11) | R4 finding-to-design traceability | `docs/design/remediation/08_r4_traceability.md` | COMPLETE (REPORTED; lead VERIFIED 4 of its drift claims in code) | done | none | lead review of its 5 drifts and 8 overstatements (see risks) |
| Phase B: V2A (agent a49fb5611e027c4ac) and V2B (agent a023f1d9317cb9aab) | V2A: Lab and evidence screens plus U-01 UI wiring; V2B: Simulation page, non-Lab pages, App.jsx and gateway/connection wiring | `lab.css`, `sim.css`, `screens/**`, `LabParts`, `LabCharts`, `Charts`, `ChartFrame`, `App.jsx` wiring | IN PROGRESS (REPORTED; both started this session) | V2A owns screens/lab, LabParts, LabCharts, Charts, ChartFrame, lab.css; V2B owns App.jsx, main.jsx, screens/*.jsx, pages.css, sim.css | V1 and FLIB delivered; TRACE | increment_V2A_lab_evidence.md and increment_V2B_pages_app_wiring.md |
| U-01 backend patch (Option A integrated) | Withhold verdicts for incomplete runs; EXPLORATORY label | `src/lab_live/scenario_b.rs`, `src/api/lab_api.rs`, `tests/integration/` | COMPLETE & INTEGRATED in working tree (67 passed in lab_live_scenario_b_tests; commit deferred per DEC-022) | done | none | owner acceptance of DEC-001 |
| U-01 independent reviewer (fresh agent, sandbox D:\Temp\sbx_u01_review) | Reviewed the diff, tests and original defect against your 7 points | `docs/research/u01_backend/independent_review.md`, `independent_review_lab_api_proposal.patch` | COMPLETE (REPORTED; lead VERIFIED the High finding at lab_api.rs 1384 to 1388 and the summary_version scope) | done | none | your decision DEC-001 |
| Toolkit and accessibility | Skills, axe-core, rubric, rules, validation tools | `.claude/skills/*`; `ui/scripts/check-axe.mjs`; `docs/design/{critique_rubric,external_design_skills_project_rules}.md` | COMPLETE for installation (VERIFIED by lead: blob hashes 5 of 5, fresh-process discovery 4 of 5 skills, `animation-performance` invoked, UI tests 299 of 299); commits of package files held | held for review | none | review of held files, then selective commit |
| Project governance | Implementation record, decision register, this tracker | `docs/design/remediation/07_*`, `docs/design/OWNER_DECISION_REGISTER.md`, this file | COMPLETE (created) and living | updated on every event | none | reconcile before each phase transition |
| M3 durability (agent aac3bb285e43fa91a), not redesign | Durable invalidation sandbox | `docs/research/m3_durability/*`, `D:\Temp\sbx_m3` | IN PROGRESS (REPORTED) | holds the cargo lock intermittently | independent review later | status and patch set; DEC-004 |
| Handbook (agents LRN1 to LRN8), not redesign | Engineering and technical defence handbook | `docs/learning/src/ch0N_*.md`, `evidence/`, `assets/` | IN PROGRESS: chapters 1, 2, 3, 4, 6, 7 and 8 delivered (REPORTED); chapter 5 running; documentation corrections: DEC-020; publication: DEC-021 | writing chapters | none | chapters, then lead back matter, QA and PDF build |

## B. Dependencies and integration
| Relationship | Impact | Status |
|---|---|---|
| V1 foundations -> Phase B | Phase B consumes the new tokens and shell; starting earlier would collide on files | waiting (scheduled, not a blocker) |
| FLIB -> Lab integration | screens call `resultPresentation`, `linkState`, `gatewayConfig`; wiring happens in Phase B | waiting on FLIB wiring spec |
| TRACE -> design review | Phase B follows the six first visual stories and the Scenario B page requirements | available now |
| U-01 integration -> owner acceptance | Option A patch and reviewer's lab_api fix integrated | COMPLETE in working tree (67 tests pass; acceptance pending) |
| Integrated backend + real-backend fixture (T-01) -> validated U-01 result page | UI validated against live backend and fixtures | VERIFIED via live browser e2e |
| Quiet machine -> motion and performance remeasurement | cargo builds (M3, reviewer) and 8 writers distort timing; no performance claim until quiet | waiting (resource, not a blocker for build work) |
| DEC-005 offline result + owner confirmation -> release of preregistration/freeze files, rng.rs and the M6 test | blocks the Rust code commit groups; not part of the frontend work | AWAITING OWNER (method chosen; result pending) |
| DEC-006 done -> before committing the four pinned files: index-blob hash check, freeze verification after staging, committed-blob check | prevents a silent freeze break | READY (procedure approved; no commit of those files yet) |
| Held dependency and test files -> separate review -> selective commit | `ui/package.json`, `package-lock.json`, `ui/test/`, `check-axe.mjs` carry earlier uncommitted changes besides axe-core | HELD FOR REVIEW, no owner decision |
| DEC-005 and DEC-006 -> Rust code commit groups | not part of the frontend work | AWAITING OWNER |

## C. Critical pending files and artefacts
Importance: C = critical, H = high, M = medium. Only items that affect integration, quality, approval or completion are listed.
| File / artefact | Purpose | Owner | Status | Dependency | Importance | Required action | Owner decision |
|---|---|---|---|---|---|---|---|
| `docs/research/u01_backend/u01_backend.patch` plus reviewer's lab_api fix | U-01 backend fix | lead | COMPLETE & INTEGRATED in working tree (all 8 conditions met, 67 tests passing, commit deferred DEC-022) | conditions checklist in register | C | integrated, tests passing, acceptance pending | DEC-001 (approved with conditions) |
| `docs/research/u01_backend/independent_review.md` and `independent_review_lab_api_proposal.patch` | independent review evidence and 22-line proposed fix | U-01 reviewer | COMPLETE (lead verified High finding, applied fix to lab_api.rs) | none | C | used for DEC-001 | DEC-001 |
| `src/api/lab_api.rs` null-verdict call sites (lines 1384 and 1855) | stop null verdicts becoming empty object, preserve status fields | lead | COMPLETE & VERIFIED (both call sites preserve null verdicts; verified by integration test) | review result | H | fixed and verified | DEC-001 |
| `increment_V1_foundations_v2.md` and V1 style/shell files (tokens, app, shell, pages CSS; Shell, Icon; check-contrast; 3 test files) | tokens, shell, materials, motion | V1 | HELD FOR REVIEW (REPORTED complete; 28 of 36 matrix shots captured but not opened; forced-colours fix not re-captured; axe 12 incomplete; no performance measurement; deviations listed in the record) | lead diff review | H | review diffs, open remaining shots, then commit as the UI foundations batch | NONE — WITHIN APPROVED SCOPE |
| `increment_F_functional_logic.md`, FLIB lib modules (resultPresentation, linkState, gatewayConfig, progress, useGateway), fixtures | U-01 UI half, connection model, gateway config | FLIB | HELD FOR REVIEW (REPORTED; unit-tested only, not wired, no browser check; fixtures: legacy = real disk bytes, new shape = SANDBOX-PATCHED output) | lead code review; Phase B wiring | H | review the edits to existing lib/hooks files; wire in Phase B | NONE — WITHIN APPROVED SCOPE |
| Phase B deliverables (Lab, evidence, Simulation screens, charts) | the redesigned experience | V2 writer (to be briefed) | NOT STARTED | V1, FLIB, TRACE | H | brief and launch after V1 | NONE — WITHIN APPROVED SCOPE |
| Performance and motion measurements | baseline and remeasure (software path, 2 long frames per 3 s) | lead | NOT STARTED | quiet machine | H | measure on the real build; record host, reps, limits | NONE — WITHIN APPROVED SCOPE |
| Final accessibility evidence (axe baseline on connected states, keyboard, forced colours) | evidence for the report | lead | NOT STARTED | Phase B | H | run `check-axe.mjs`; resolve `incomplete` items manually; Chrome and Firefox stable baseline per DEC-008 | NONE — WITHIN APPROVED SCOPE (DEC-008 approved) |
| `ui/package.json`, `ui/package-lock.json` | axe-core 4.14.0 dev-only (diff also contains earlier unrelated changes: key order, scripts) | lead | HELD FOR REVIEW | review of the whole diff | M | review, then selective commit | NONE — WITHIN APPROVED SCOPE |
| `ui/test/` (entire directory untracked) | regression tests incl. the two pinning-test updates | lead | HELD FOR REVIEW | review and registration of new test files | M | review, then commit as a group | NONE — WITHIN APPROVED SCOPE |
| `ui/scripts/check-axe.mjs` | project-owned axe runner | lead | HELD FOR REVIEW (self-tested; no baseline yet) | package files | M | commit with the package group | NONE — WITHIN APPROVED SCOPE |
| `.gitattributes` | line-ending protection for the four pinned files | lead | INTEGRATED (committed `df68eaa`; rules VERIFIED with git check-attr) | none | M | none; hash checks apply when the four files are committed | DEC-006 (approved) |
| 25 documents corrected under DEC-020 (README, CLAUDE.md errata, STATUS, handoff, ADRs, phase reports) and `docs/release/doc_corrections_DEC020.md/.diff` | accuracy corrections; documentation-only | agent DOCFIX | HELD FOR REVIEW (working tree only; not committed) | owner review of the diff | M | review, then a separate docs-only commit when you authorise it | DEC-020 (approved; commit not authorised) |
| `docs/design/critique_rubric.md`, `external_design_skills_project_rules.md` | rubric and 16 rules (updated 16:25 for DEC-007 and DEC-008) | lead | HELD FOR REVIEW (approved decisions, content review pending) | none | M | review pass, then commit | NONE — WITHIN APPROVED SCOPE (DEC-015, DEC-016 approved) |
| `docs/design/remediation/07_frontend_redesign_implementation_record.md` | living implementation record | lead | IN PROGRESS | none | M | keep current; link from here | NONE — WITHIN APPROVED SCOPE |
| `docs/design/remediation/08_r4_traceability.md` | R4 findings to design decisions | TRACE | HELD FOR REVIEW (REPORTED; 4 claims VERIFIED by lead) | lead review of 5 drifts and 8 overstatements | M | review; feed corrections into Phase B brief; apply DEC-007 (approved) | NONE — WITHIN APPROVED SCOPE (DEC-007 approved) |
| Before/after visual review | consolidated review | lead | NOT STARTED | Phase B | M | regenerate galleries; open images; rubric run | NONE — WITHIN APPROVED SCOPE (human review: DEC-008 approved; reviewers not yet named) |
| Final defect register (U-01..U-14 dispositions) | completion report | lead | NOT STARTED (current draft in record 07 section 4) | all phases | M | finalise with evidence | NONE — WITHIN APPROVED SCOPE |
| `docs/research/design_toolkit/toolkit_proposal.md` | integrated toolkit proposal (research) | lead | COMPLETE, untracked | none | M | commit with research records group later | NONE — WITHIN APPROVED SCOPE |

## D. Autonomous decisions taken since the last report
Recorded in the decision register section 4 (AD-1 to AD-7): phased execution; axe pinning-test updates; unrendered-page guard; no client-computed outcome remainder; Simulation caption per rule 12; path-limited commits under ODR-3; CLAUDE.md citation drift noted, not edited.

## E. Risks and unresolved issues
| ID | Description | Severity | Workstream | Mitigation | Owner decision | Status |
|---|---|---|---|---|---|---|
| RK-1 | Legacy cancelled artifacts keep the misleading verdict shape even after the patch; UI must suppress by run state | High | FLIB, Phase B | `resultPresentation` handles both shapes; fixtures from real legacy output | DEC-001 | OPEN |
| RK-2 | Agent-only verification (Chrome 156 headless, one Windows host); nothing independently verified by a human. Baseline now set (DEC-008) but reviewers and date are not yet named | High | all | label TESTED at most; record reviewers and date before the final review; no invented participants | DEC-008 (approved; names pending) | OPEN |
| RK-3 | Timing results are unreliable while cargo and 8 writers run | Medium | performance | measure only on a quiet machine; record load | NONE | OPEN |
| RK-4 | Phase B and FLIB wiring both touch `RunsViews`, `ScenarioBResult`, `LabScreen`, `App.jsx` | Medium | Phase B | FLIB edits logic files only; one V2 writer owns screens | NONE | MITIGATED |
| RK-5 | `ui/package.json` diff is not only axe-core (earlier uncommitted edits) | Medium | toolkit | review the full diff before commit | NONE | OPEN |
| RK-6 | axe reports `color-contrast incomplete` on translucent surfaces; it is not a pass | Medium | accessibility | worst-case contrast check plus manual review | NONE | OPEN |
| RK-7 | `review-animations` cannot be model-invoked (frontmatter flag); explicit invocation untested | Low | toolkit | test `/review-animations` in an interactive session; skill text usable by reading | NONE | OPEN |
| RK-8 | R4 and the proposal overstate some stories (outcome remainder, node state, intelligence page text) | Medium | Phase B | follow TRACE section 8 and AD-4; designed unavailable states; REPLAYED rule settled by DEC-007 | NONE | OPEN |
| RK-9 | Simulation graphic does not meet rule 12 (aria-hidden, badge instead of caption); `viewmodel.js:134-139` strings are stale | Medium | Phase B | fix in Phase B (AD-5) | NONE | OPEN |
| RK-10 | DD v2 text still says "ILLUSTRATION: not live, no data" while the approved caption differs | Low | design docs | note in Phase B and update DD v2 reference | NONE | OPEN |

| RK-11 | Handbook discrepancy: a code comment in `delete_cache_handler` says it "Guarantees linearizability" (project rules forbid that claim); README and handoff docs disagree with code on shard count (8 vs 64) and single-flight type | Medium | handbook / docs | flagged in handbook chapters; no product edit | NONE | OPEN (REPORTED by chapter 1 writer) |

## G. Commit tracking (owner strategy: larger phase-wise batches, 50 to 150 files where the scope supports it; push after committing, DEC-023)
| Phase / Workstream | Files | Commit hash | Tests | Review status |
|---|---:|---|---|---|
| Toolkit: apple-design skill, overlay and evidence | 5 | `8523cc1` (pushed) | n/a (documentation); blob hashes verified | lead read all files |
| Tooling: report-only docs-lint and config | 2 | `3e6e03e` (pushed) | docs-lint ran read-only, exit 0 | lead read; small commit predates the larger-batch strategy |
| Toolkit: four Emil animation skills | 13 | `caeedfd` (pushed) | blob hashes 5 of 5; fresh-process discovery 4 of 5, one invocation | lead read all five skill files in full |
| Git: line-ending protection for four pinned files | 1 | `df68eaa` (pushed) | git check-attr shows 4 unset, others unspecified | owner decision DEC-006 |
| Research records (B02) | 85 | `9ab179f` (pushed) | staged list equals planned list; hex values compared with held files (only source-file hashes overlap) | lead verified lists and masked hex context; batch planned by an AI agent |
| Product, security, handoff, release docs, docs index, 15 notices, release plan records (B04, B14) | 90 (15 modified, +60/-0) | `804a7b8` (pushed) | same checks; docs-lint on an index copy run by the BATCH agent | lead verified the 15 notice diffs |
| PLANNED: design rules, governance and implementation records, toolkit research (B01) | 55 | pending | docs-lint | HELD FOR REVIEW: rubric, rules and 08 traceability content review; living files still edited |
| PLANNED: UI foundations v2 (V1) with its tests and contrast script | about 15 | pending | UI suite 483 of 483 (lead ran), contrast 284 of 284, a11y 0 issues | V1 REPORTED complete; lead review of diffs pending |
| PLANNED: UI functional logic (FLIB) with tests and fixtures | about 25 | pending | 159 tests (lead ran whole suite) | HELD FOR REVIEW |
| Handbook (docs/learning) (B05) | about 95 | NOT COMMITTED: DEFERRED / LOCAL ONLY (DEC-021) | AI-PREPARED, AI-REVIEWED, HUMAN REVIEW PENDING. Built: 183 pages, 131k words. PDF inspection: 9 pages of the corrected PDF read, 2 build defects fixed, first build archived; two-layer analysis (agent LAYERS) running | no human review |
| PLANNED: Lab and Simulation redesign (Phase B) | pending | pending | pending | not started |
| BLOCKED: Rust code (3 commits) | about 130 | pending | cargo check on index copy | DEC-022, DEC-005, DEC-006 hash procedure |

Batch plan v2 (agent BATCH, running): `docs/release/g5_batch_plan_v2.md` and `docs/release/commit_lists_v2/`. Each commit is staged from an explicit path list, checked with `git diff --cached --name-only`, scanned for hex strings, then pushed (fast-forward, feature branch only).

## H. Running now and the waiting queue (owner rule: do not stop anything that can run concurrently; start waiting work as soon as its trigger clears)
### H1. Running (agents)
| Work | Agent | Output | Note |
|---|---|---|---|
| Phase B Lab and evidence screens, U-01 UI wiring | V2A | increment_V2A_lab_evidence.md | UI |
| Phase B Simulation, other pages, app and gateway wiring | V2B | increment_V2B_pages_app_wiring.md | UI |
| U-01 backend integration (DEC-001 conditions) | U01INT | docs/research/u01_backend/integration/ | working tree only; no commit |
| M3 sandbox rework after the independent review | M3FIX | docs/research/m3_durability/remediation_v2/ | sandbox; DEC-027 |
| G4A-01 stale-body race fix | G4A01 | docs/research/g4a01_fix/ | sandbox; protected core |
| API hardening: G4B-01 to 04, SEC-M7-09 to 11 | HARDEN | docs/research/api_hardening/ | sandbox; no telemetry or JWT-telemetry change |
| Peer trust: DR-029, DR-027, G4B-05, 06, 08, 10, G4A-12 | PEER | docs/research/peer_trust/ | sandbox; owner decisions raised |
| Dependency advisory scan (isolated cargo-audit, npm audit) | SCAN | docs/security/advisory_scan_2026-10-30.md | no repo change |
| Human review packages (DEC-008 baseline): DELIVERED (8 documents, about 21k words) | HR (done) | docs/release/human_review/ | documents only; blanks [FILL] for reviewers, dates, phone, thresholds; DEC-029 |
| Multi-host validation plan and runbook (delivered) and its second-reader check | MH (done); MHREVIEW | docs/research/multi_host/ 00 to 05; 06_second_reader_review.md | documents only; devices BLOCKED (DEC-028 not yet ready) |
| Handbook full-page PDF re-inspection: A done; B restarted as B1 (66 to 98) and B2 (99 to 130) after an out-of-memory failure; C, D, E running | inspectors | docs/learning/evidence/pdf2/ | machine has about 2 GB free memory; watch for further failures |
| Two-layer prototype v2 (markers, front matter, bookmarks, near-blank fixes) and a slim variant | PROTO | docs/learning/proposal_v2/out and out_slim; two_layer_prototype_v2_notes.md | inspection L done: 49 of 58 sampled pages OK; 194 and 133 pages; not concise yet |
### H2. Waiting queue (launch automatically when the trigger clears)
| Work | Trigger | Owner decision needed | Status |
|---|---|---|---|
| Independent review of the U-01 integration (combined diff) | U01INT delivers | none (DEC-019 style fresh agent) | QUEUED |
| Second independent review of the M3 rework | M3FIX delivers | none (existing M3 approval) | QUEUED |
| Independent reviews of the G4A-01, API-hardening and peer-trust sandbox fixes | each agent delivers | owner approval before any merge | QUEUED |
| Tenant quotas Stage A sandbox (E5) | G4A01 delivers (same file cache.rs) | merge needs owner approval and frozen-core handling | QUEUED |
| UI independent AI review tracks (functional, accessibility, visual, usability) and the U-01 UI check | V2A and V2B deliver | none | QUEUED |
| Final UI measurements: performance and motion remeasure (software path, 2 long frames per 3 s) | quiet machine (cargo agents finished) and Phase B done | none | QUEUED |
| Final accessibility baseline (axe on connected states, keyboard, forced colours) | Phase B done | none | QUEUED |
| Before/after gallery and the consolidated visual review; final defect register U-01 to U-14 | Phase B done | none | QUEUED |
| Release-profile test suite, soak, benches, cargo fmt check | quiet machine | none | QUEUED |
| Handbook v1.1 polish (contents depth and page numbers, orphan table rows, keep-with-next) and a re-inspection of changed ranges | PDF inspections B to E report | none | QUEUED |
| Two-layer decision (adopt, refine or drop) | prototype inspection and your review | OWNER (DEC-021 deferred) | WAITING ON OWNER |
| Commit batches: design rules, governance, implementation records; UI foundations, functional logic and screens; handbook | Phase B done and reviews; your explicit commit authorisation for items you deferred | owner for deferred items (DEC-021, 022, 024, 025, DEC-020 review) | QUEUED |
| Sandbox work on G4B-07 admin path, G4A-02 S3-FIFO, G4A-08 | owner direction on trust model and priorities | OWNER | WAITING ON OWNER |
| Intelligence event tap and E1/E2 experiments, frozen addendum 2 batch | owner decision to allow addendum 2 (declined for now) and the offline salt | OWNER | BLOCKED |
| Rust commits (3 or 1 atomic) | DEC-005 offline result, G4B-01 and G4A-01 fixes validated and reviewed, DEC-022 reassessment | OWNER | DEFERRED |
| Multi-host validation run | owner devices and OS mix; plan finished | OWNER | BLOCKED |
| Human reviews (design, usability, accessibility, security) | owner names reviewers and dates; packages finished | OWNER | BLOCKED |

## F. Update log (newest first)
- 2026-10-30 18:28: human review packages delivered (REPORTED); gaps: no verified failed-run recipe, phone connectivity unresolved, cluster certificate recipe missing; DEC-029 added. PDF inspector B failed (out of memory; machine had 1.9 GB of 15.7 GB free) and was restarted as B1 and B2.
- 2026-10-30 18:26: multi-host plan delivered (REPORTED; 6 documents, 11 experiments with falsifiers, thresholds are proposals); code gap: the Lab load generator cannot target remote gateways; DEC-028 added; second-reader agent MHREVIEW launched.
- 2026-10-30 18:23: two-layer prototype inspected (58 pages): structure works but unfinished (no cross-part markers, status label missing on both title pages, 5 near-blank pages, no outline); PROTO agent launched to fix and to build a slim variant (knowledge checks, exploration tasks, challenge questions and most judge boxes moved to Part II; nothing deleted).
- 2026-10-30 18:17: owner asked to keep everything that can run concurrently running and to queue the rest: launched G4A01, HARDEN, PEER, SCAN, HR, MH and the two-layer prototype inspection; two-layer PDFs built (learning 194 pages, reference 133 pages); waiting queue with triggers recorded in section H.
- 2026-10-30 18:13: Phase B started (autonomous, inside DEC-017): V2A and V2B launched with exclusive file sets; owner asked whether the UI/UX was complete: answered no (Phase A delivered, Phase B running, no browser validation of new screens, no human review).
- 2026-10-30 17:57: short guide delivered (12 A4 pages, about 5,700 words, 2 figures; docs/learning/short/PHANTOMCACHE_SHORT_GUIDE.pdf); lead opened all 12 pages; one repair-timing wording corrected (the 250 ms worker tick drives the repair ledger re-send; the writer had misread it); local only, uncommitted, AI-PREPARED / AI-REVIEWED / HUMAN REVIEW PENDING. Handbook final build now carries PDF bookmarks (547) and a tagged structure tree; pixel-identical on 10 sampled pages to the inspected copy; inspection A (pages 1 to 65) done, B to E running.
- 2026-10-30 17:42: owner asked for a 15-page short version of the handbook; writer agent launched (docs/learning/short/, local only, AD-16); every page will be opened and inspected before hand-over.
- 2026-10-30 17:40: M3 independent review delivered (REPORTED; lead verified the pinned-file list in the diff): INTEGRATE AFTER FIXES; sandbox rework (agent M3FIX) started under AD-15; DEC-027 still blocked; F-11 needs a telemetry decision (DEC-003).
- 2026-10-30 17:36: DEC-020 documentation corrections delivered by agent DOCFIX (REPORTED; lead VERIFIED the diff scope): 25 documents, 38 corrections, uncommitted; review artefacts docs/release/doc_corrections_DEC020.md and .diff.
- 2026-10-30 17:35: handbook PDF inspection: 4 inspectors read every page of the first corrected build; root cause found and fixed (a wide table made Chrome shrink pages to about 0.65; text printed at 7 pt); true-size build is 320 pages; Markdown artefacts fixed; figure fixes in progress (agent FIGFIX); full re-inspection of the final build pending. Two-layer analysis and prototype delivered (Part I about 76k words, Part II about 56k; not applied).
- 2026-10-30 17:25: owner decisions applied (see the register change log): DEC-022, DEC-024, DEC-025 deferred; DEC-020 approved (documentation-only, agent DOCFIX preparing, no commit); DEC-005 action required from the owner; DEC-027 review required, no acceptance; DEC-001 integration continues with acceptance pending; handbook review copy is the _rebuilt PDF; PDF page inspection (4 inspectors) and two-layer prototype (LAYERS) running.
- 2026-10-30 17:21: DEC-021 deferred (handbook local only). PDF inspection started: chapter/appendix order defect and stale-PDF build defect found and fixed; first build archived; corrected PDF named _rebuilt (the original is open in a viewer). Two-layer analysis agent launched.
- 2026-10-30 17:14: handbook finished (back matter A and B, reconciliation, final build 183 pages). DEC-021 awaiting your authorisation to commit. U-01 integration (U01INT) and M3 review (M3REV) running.
- 2026-10-30 16:59: DEC-001 approved with conditions; U-01 integration agent launched. M3 sandbox fix delivered (REPORTED); DEC-027 added (blocked on review: freeze addendum and defaults).
- 2026-10-30 16:57: U-01 independent review delivered (INTEGRATE AFTER FIXES; 1 High: lab_api.rs null-to-empty-object). DEC-001 moved to awaiting owner. Handbook back matter A delivered (6 chapter inconsistencies to reconcile); back matter B running.
- 2026-10-30 16:48: batches B02 and B04+B14 committed and pushed (9ab179f, 804a7b8). Handbook QA A, B and C delivered (REPORTED). Batch plan v2 delivered; DEC-024 and DEC-025 added, DEC-026 not ready.
- Correction (machine clock 16:45): earlier minute-level times in this log were estimates; replaced by the date only. Machine-clock times: docs/release/lead_timeline.md.
- 2026-10-30: handbook QA B delivered (REPORTED): chapters 4 and 5 checked (about 200 references, 11 factual fixes, rendezvous replica re-run independently, 11 figures viewed and fixed); length cut only 0.5 percent because the remaining repetition is required scaffolding.
- 2026-10-30: pushed `456bb25..df68eaa` to origin (owner instruction DEC-023). V1 delivered (UI suite 483 of 483 verified). DEC-022 (Rust code commit and publication with open findings, HIGH) added. Handbook: all 8 chapters delivered, QA agents A, B, C running. BATCH planning agent running.
- 2026-10-30: register addendum applied: DEC-020 (documentation corrections from handbook research) and DEC-021 (handbook publication) added; AD-12, AD-13 added; handbook chapter 6 delivered (REPORTED).
- 2026-10-30: FLIB delivered (REPORTED); its 5 test files registered (suite 455 of 458; the 3 failures are old no-glass and nav assertions that V1 must update, V1 messaged); AD-8 to AD-11 recorded; handbook chapters 3 and 7 delivered.
- 2026-10-30: owner decisions DEC-005 to DEC-009 applied. DEC-006 done (.gitattributes committed df68eaa). DEC-007 pushed to FLIB and the rubric/rules. DEC-008 baseline recorded. DEC-009 confirmed in both G4 records. DEC-005: method selected, results pending. Handbook chapter 1 delivered (REPORTED).
- 2026-10-30: both governance files created and reconciled against ListAgents and git state; U-01 independent reviewer launched (DEC-019); TRACE delivered; handbook agents running.
- 2026-10-30: V1, FLIB and TRACE launched; `caeedfd` committed.
