# Lead working state (the lead's own memory aid)

**Status:** Active. **Audience:** the lead (Claude) and the owner. **Purpose:** keep time, context and next steps in one place so nothing is lost across long sessions or context resets. **Rule:** times in this file and in `lead_timeline.md` come from the MACHINE CLOCK (`date`), never from estimates. Append timeline entries with the helper: `pwsh -NoProfile -File D:\ClaudeTemp\claude\d--Personal-phantom-cache\d45f6980-8d59-4418-9545-e9f87e8aef66\scratchpad\lead_log.ps1 "<entry>"` (it stamps the clock and appends to `docs/release/lead_timeline.md`). Update the sections below whenever something starts, finishes or changes. The execution register (`execution_register.md`) is the task register; the progress ledger (`docs/research/progress_ledger.md`) is the event history; this file is the quick "where am I" view.

$12026-10-30 15:35**

## 1. Owner standing rules (do not forget)
- Priority order set 15:27: (1) repository integrity and controlled commits, (2) M3 durability, (3) functional UX ownership, (4) accountability refresh. UI/UX research stays READ-ONLY: no frontend edits until the owner says the implementation phase is ready.
- No commit or push until the owner's gates are met: reconciliation in force = commit only groups that have passed review (verified docs first; code by area after G4 Cycle 1). Nothing is staged or committed yet. Never `git add .`, `git clean`, `git reset --hard`, force-push, indiscriminate restore. Backup exists: `D:\Personal\phantom-cache-backups\20261030T152222`.
- Frozen/sealed records untouched. Never read the sealed salt directory. Owner generates any salt offline. Do not print hex strings or secrets.
- Rust remediation only in SANDBOX copies; merging into the protected core needs independent review and the owner's final approval. One cargo job at a time via `D:\Temp\locks\with_cargo_lock.ps1`.
- No new dependencies or skill installs without approval (toolkit proposal pending). Provenance labels (REAL with qualifiers, DERIVED, SIMULATED, REPLAYED, PREDICTED only when supplied, UNAVAILABLE) never weakened; no fabricated data.
- AI reviews are labelled "AI second look", never "independent" in the human sense. Do not claim completion without evidence.
- Owner kept final authority over architecture, protected-core integration and release decisions.

## 2. Running now (check with ListAgents; confirm with files, not labels)
| Name | Task | Started (clock) | Output / evidence to look for |
|---|---|---|---|
| G5PREP | Classification and commit plan | about 15:27 | DELIVERED (23 lists; C01+C02 committed as 8523cc1, 3e6e03e) |
| M3 | Durable invalidation remediation in sandbox `D:\Temp\sbx_m3` (target `D:\Personal\_sbx_m3_target`) | about 15:27 | `docs/research/m3_durability/remediation/` (status, matrix, patches, evidence) |
| FUNC | Functional UX breakdown | about 15:27 | DELIVERED 06_functional_ux_work_breakdown.md; U-01 UI+backend split lead-confirmed by reading |
| G4A | Cycle 1 review: core cache engine | about 15:27 | DELIVERED (26 findings, verdict YES WITH FIXES; G4A-01 fencing finding lead-confirmed by reading; blocks core commit) |
| G4B | Cycle 1 review: dist, security, API | about 15:27 | DELIVERED (38 findings, 1 High; G4B-01 pre-auth body read lead-confirmed; verdict YES WITH FIXES; blocks dist/API commit) |
| G4C | Cycle 1 review: lab, intelligence, freeze, deps | about 15:27 | DELIVERED (35 items; F01 High = U-01 backend defect; freeze 4/56 files differ, additive; verdict YES WITH FIXES) |
| TK | Design-toolkit integration proposal | about 15:32 | `docs/research/design_toolkit/toolkit_proposal.md` |
| U01 | U-01 backend sandbox fix (ODR-2) | 2026-10-30 15:42 | docs/research/u01_backend/ |
| RACE | G4A-01 investigation, sandbox (ODR-4) | 2026-10-30 15:42 | docs/research/g4a01_race/ |
| LIMITS | Frozen-code limitations record (ODR-1) | 2026-10-30 15:42 | docs/release/known_limitations_frozen_code.md |
| DISP | G4 findings disposition register | 2026-10-30 15:42 | docs/release/g4/cycle1_findings_disposition_register.md |
| V1 | Frontend foundations v2 (tokens, shell, materials, motion), phase A | 2026-10-30 16:1x | docs/design/remediation/increment_V1_foundations_v2.md |
| FLIB | Functional UX logic (resultPresentation, linkState, gatewayConfig, progress) + fixtures | 2026-10-30 16:1x | docs/design/remediation/increment_F_functional_logic.md |
| TRACE | R4 traceability document | 2026-10-30 16:1x | docs/design/remediation/08_r4_traceability.md |
| LRN1..LRN8 | Handbook chapters 1-8 (docs/learning/src/chNN_*.md, evidence/, assets/) | 2026-10-30 16:2x | docs/learning/src/ch0N_*.md; then lead writes back matter, QA, builds PDF with docs/learning/build/build_handbook.mjs (pandoc + Chrome; tested on a sample) |
Agent IDs for SendMessage are in the private scratchpad file `agents.json` (not in the repo).

## 3. Done (with evidence)
- Storage cleanup (Category B all; seven snapshots copied and hash-verified; four protected snapshots left): `docs/research/storage_cleanup_2026-10-30.md`.
- Documentation batch 1 (indexes, glossary, contributing, strategy, lint, notices): verified; accountability refresh (STATUS.md, workstream ledger, verification register, accountability section 13): verified prefix hash.
- Vision/three-pillar package, discrepancy register, DR-029 investigation plus AI review (decision pending), product-vision audit, execution audit.
- UI increments I1-I4 IMPLEMENTED and TESTED (298/298 tests, contrast 196/196, a11y lint 0 issues); NOT visually approved; O8-2 OPEN. Before/after galleries, AI usability walkthrough, Track 2 accessibility (0 blocking, 0 high, 3 medium, 8 low).
- Design-toolkit research R1-R5, DD direction with measured performance (all delivered; integration pending).
- Backup of the working tree (2,961 files; sample verified).

## 3b. Owner approvals received 2026-10-30 (late)
- Toolkit adoption + frontend redesign APPROVED (brief in conversation; record: docs/design/remediation/07_frontend_redesign_implementation_record.md). UI implementation phase is OPEN. Installed: 4 Emil skills, axe-core 4.14.0 dev-only, check-axe.mjs, rubric, external-skills rules. Still owner-reserved: U-01 backend integration, JWT telemetry auth, any telemetry. Provisional until confirmed: illustration caption, eyebrow exception.
- U01 sandbox fix delivered (docs/research/u01_backend/), NOT integrated; needs independent review + owner approval.

## 3e. Owner decisions 2026-10-30 (latest)
- DEC-001 APPROVED WITH CONDITIONS: U-01 integration authorised into the working tree (agent U01INT running); commit/publication NOT authorised (DEC-022). DEC-027 (M3 integration + M6 freeze addendum + policy) BLOCKED ON REVIEW; M3 independent reviewer (M3REV) running. Handbook: chapters 1-8 and QA done; back matter A done, B running; then reconcile, final build, commit (DEC-021). Commits pushed: 8523cc1, 3e6e03e, caeedfd, df68eaa, 9ab179f, 804a7b8.

## 3d. Governance files (owner instruction 2026-10-30)
- docs/design/OWNER_DECISION_REGISTER.md (owns decision status; DEC-001..019, AD-1..7) and docs/design/REDESIGN_EXECUTION_TRACKER.md (agents, critical files, dependencies, risks). Update BOTH on every meaningful event; reconcile before phase transitions; label unconfirmed agent reports REPORTED. U-01 independent review APPROVED (DEC-019) and running (agent ac71211569ef2bc9b): review only; integration needs owner approval (DEC-001).

## 3c. Handbook task (owner brief received 2026-10-30, later message)
- Deliverable: docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK.md (assembled source of truth) + .pdf. Style/rules: docs/learning/src/00_STYLE_AND_RULES.md; chapter specs: 00_CHAPTER_SPECS.md. Phase 1 chapters running; phase 2 lead writes back matter (overview, architecture explanation, tech/algorithm reference, glossary, judge bank, source-file map, implementation-vs-roadmap matrix, ten concepts) from the chapters; phase 3 QA (code refs, contradictions, overclaims, judge answers); phase 4 build + PDF check. No commits unless owner authorises.

## 4. Next (in order)
1. Collect G5PREP output; verify the first eligible commit set (C01 skill adoption, C02 docs system); present the commit plan; commit ONLY verified groups with explicit path lists (`git add -- <paths>`), checking `git diff --cached --stat`; attribution line on every commit.
2. M3: review progress, run the independent-review brief through a separate reviewer, then report; protected merge only with the owner's approval.
3. FUNC breakdown: review it; decide ownership table; ask the owner for U-01 backend authorisation and JWT-telemetry decision; wait for "UI implementation phase ready".
4. G4 Cycle 1 findings: classify, reproduce important ones, fix via sandbox/regression tests, then commit code areas after review.
5. Toolkit proposal: review and present for approval (nothing installed before).
6. Queued engineering (held): SEC-M7-09/10/11, DR-029/DR-027 fix, tenant quotas, advisory scan, intelligence E0-E2, release-profile suite and soak (needs quiet machine), multi-host and human-review packages.

## 5. Open owner decisions
Commit policy RESOLVED by ODR-3; UI implementation phase readiness; U-01 backend authorisation; JWT telemetry authentication; toolkit approvals (axe-core A1 etc.); M3 and DR-029 protected-core merges; the two commitment strings offline check (preregistration files HOLD); OD-1..OD-50 and B-1..B-10; physical devices and human reviewers; offline salt for confirmatory intelligence runs.

## 6. Key paths
Repo `D:\Personal\phantom-cache`; backup `D:\Personal\phantom-cache-backups\20261030T152222`; status list `D:\Temp\status_uall.txt` (older snapshot); cargo lock helper `D:\Temp\locks\with_cargo_lock.ps1`; gateway debug exe `D:\Personal\_m7_target\debug\phantom-cache-demo.exe`; build sandboxes `D:\Temp\sbx_*`, targets `D:\Personal\_sbx_*_target`; research `docs/research/`, release records `docs/release/`.

