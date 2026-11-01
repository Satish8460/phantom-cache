# G5 batch plan v2 (phase-wise commits; proposal only, nothing staged or committed)

**Status:** Draft v2 (agent BATCH; supersedes `docs/release/g5_commit_plan.md` and the lists in `docs/release/commit_lists/`, which are STALE). Needs lead verification at staging time and owner decisions listed in section 9.
**Audience:** product owner, lead (commit operator), reviewers.
**Last reviewed:** 2026-10-30 16:44 IST (machine clock, `date`). Status snapshot taken 16:41:31 IST. HEAD `df68eaab139dc82e894f9b8af17dce138c321f99`, branch `feat/publish-phantomcache-core` (in sync with origin per `git status -sb`), 0 staged entries.
**Strategy applied (owner, 2026-10-30):** substantial, logically grouped, phase-wise commits; 50 to 150 files per commit; more than 150 only for one coherent reviewed deliverable; fewer than 50 when the coherent change is smaller; never pad; commit and push per DEC-023 (plain fast-forward push of the feature branch).
**Evidence tags:** VERIFIED = command run by this agent in this task; REPORTED = stated in a repository document or by another agent, not re-run here; INFERENCE.
**Lists:** `docs/release/commit_lists_v2/*.txt` (one repo-relative path per line; lines starting with `#` are comments). 20 list files exist now (14 batches B01 to B14, 4 HELD lists, 1 NEVER list for CLAUDE.md, 1 NEVER list for transcripts). The list files and this plan are themselves new untracked files (20 + 1 entries) that are not part of the 4,644 accounted below; they belong to B14.

## 1. Live status summary (VERIFIED, `git status --short -uall`, 2026-10-30 16:41:31 IST)

Total **4,644** entries: **37 tracked-modified (M), 4,607 untracked (??), 0 deleted, 0 renamed, 0 staged** (the 4,607 = 4,644 minus 37). Excluding this task's own 21 output files (20 lists + this plan). At 16:44 one more file appeared that is not in the snapshot or in any list: `docs/learning/evidence/qa_B.md` (handbook QA agent B; belongs to B05, which is BLOCKED anyway; the live total was then 4,645 plus the 21 outputs). Handbook QA files will keep arriving; regenerate B05 before staging.

| Class | Entries | Size | M | Notes |
|---|---:|---:|---:|---|
| Batches B01 to B14 (proposed commits) | 880 | about 20.0 MiB | 35 | section 2 |
| HELD lists (H01, H02, H03, H04) | 22 | 0.37 MiB | 2 | owner/lead decisions |
| NEVER without a specific decision: `CLAUDE.md` | 1 | 8 KiB | 0 | |
| Superseded stale lists `docs/release/commit_lists/` | 24 | 0.33 MiB | 0 | not committed (section 7) |
| Evidence bulk (images, `lab-results/`, `bench-results/`) | 2,015 | 279.6 MiB | 0 | owner approval A-8: stays untracked |
| `.local/snapshots/**` | 1,701 | 150.9 MiB | 0 | never staged |
| Transcripts | 3 | 15.2 MiB | 0 | never staged |
| **Total** | **4,644** | about 466 MiB | **37** | arithmetic in section 7 |

Paths that were changing while this plan was written (mtime within 20 minutes of 16:39, so lists for these batches may need regeneration): B01 `docs/design/OWNER_DECISION_REGISTER.md`, `REDESIGN_EXECUTION_TRACKER.md`, `critique_rubric.md`, `external_design_skills_project_rules.md`, `remediation/07_*`, `increment_F_*`, `increment_V1_*`; B03 all of `docs/research/m3_durability/remediation/` (patches and `00_status_and_plan.md`, M3 agent still running); B04 `docs/release/g4/cycle1_findings_disposition_register.md`, `known_limitations_frozen_code.md`; B05 34 of 53 handbook files (QA agents A, B, C running); B06 `ui/src/hooks/useGateway.js`, `useHistory.js`, `useTelemetry.js`, `lib/gatewayConfig.js`, `labProvenance.js`, `progress.js`, `func.gateway/progress` tests; B07 `app.css`, `shell.css`, `tokens.test.mjs`; B08 `ui/test/index.js`, `lab.fixes`, `shell`, `shell.v2` tests; B10 `v1_preview/` logs and capture report. **Every list must be regenerated from a fresh `git status --short -uall` immediately before staging** (rule 1 of section 4).

Clock note: the register and tracker headers say "Last updated 16:55" while the machine clock read 16:38 to 16:44 during this task. This plan uses only `date` output; the lead should reconcile the document timestamps.

## 2. Batch table

Legend. Readiness: READY = no running agent and no open decision blocks it, lead still runs section 5; HELD FOR REVIEW = content review or owner read pending, no agent running; BLOCKED = named decision, finding or running agent; NEVER. Size is working-tree bytes (KiB, MiB = 1,048,576). M/new = tracked-modified / untracked.

| ID | Name / phase | Engineering responsibility | Files (M/new) | Size | Readiness | Order and dependencies | Review evidence so far | Approvals needed |
|---|---|---|---:|---:|---|---|---|---|
| **B02** | Research, audit and engineering records | Documentation: research records | 85 (0/85) | 2,682 KiB | **READY** | First docs commit. Needs nothing. Forward links to B04 (27), B03 (8), B01 (1), H03 (14), CLAUDE.md (2) stay dangling until those land | Agent audits D1-D4 (`docs/research/doc_audit/`); G1 and G2 verdicts (REPORTED); no human review | ODR-3 lead verification; DEC-020 option A may later add errata to ADR-001 (committing it unchanged is allowed, "nothing blocks") |
| **B04** | Product, security, handoff, release docs; docs index, status, glossary; 15 historical notice edits | Documentation: product, release, security, handoff | 88 (15/73) | 970 KiB | **READY** (after B02) | After B02 (107 links point into B02). Forward links to B01 (26), B10 (8), B03 (4), B11-B13 paths (12), CLAUDE.md (21), H01-H03 (19) | Doc audit (agents), docs-lint; verification register and ledger are lead-written; 15 notice edits are +4/-0 each (VERIFIED `git diff --numstat`) | Lead verification. `lead_timeline.md` and `lead_working_state.md` are excluded (H04). README.md and PHASE_2_WORK_REPORT.md excluded (H02) |
| **B01** | Design rules, governance, implementation records, toolkit research, prototypes | Design system documentation; implementation records | 55 (0/55) | 1,146 KiB | **HELD FOR REVIEW** | Any time after B02/B04 (best order: B01, B03, B02, B04 gives the fewest forward links: 29 vs 74, section 3). No code dependency | TRACE (REPORTED, 4 claims lead-VERIFIED); rubric and 16 rules approved as DEC-015/016 but "content review pending"; `08_r4_traceability.md` has 5 drifts and 8 overstatements for lead review; `increment_V1_*` and register/tracker still being edited | Lead content review of `critique_rubric.md`, `external_design_skills_project_rules.md`, `08_*`; DEC-017/018 approved. Prototype perf JSON was measured while the machine was busy (RK-3): keep its labels, no performance claim |
| **B03** | Sandbox investigations and proposed-patch records | Documentation: protected-core proposals (G4A-01, M3, U-01, TTL, phase patches) | 42 (0/42) | 4,301 KiB | **BLOCKED**: running agents M3 (files changed 0 to 4 minutes ago) and U-01 independent reviewer (`independent_review.md` not yet written, list will gain files); content tied to DEC-001 and DEC-004 | After the M3 and U-01 agents finish; independent of code | M3 and RACE reports REPORTED; no independent review yet (tracker: 2 not yet requested) | Lead decides when the M3 and U-01 directories are final. The records do not integrate anything, so DEC-001/DEC-004 gate integration, not committing records (INFERENCE) |
| **B10** | Curated verification evidence (no screenshots) | Testing and validation evidence | 212 (0/212) | 5,007 KiB | **HELD FOR REVIEW**, owner decision with no ID (gap G-4) | After B01 (evidence reports are linked from design docs: 12 broken links resolve) | Exploratory agent runs on a debug build, Chrome 156 headless, one Windows host (RK-2): not independent | Owner: whether JSON/MD/log evidence (4.89 MiB total, all files under 1 MiB) may be tracked while PNG/JPG stay untracked (A-8). Gap G-4 |
| **B05** | Engineering and technical-defence handbook | Documentation: learning handbook | 53 (0/53) | 1,024 KiB | **BLOCKED**: QA agents A, B, C running (34 files changed in the last 20 min); DEC-021 NOT YET READY; DEC-020 AWAITING OWNER (handbook documents code/doc discrepancies) | After B02 and B04 (548 path references into docs/src resolve over time); no PDF or assembled handbook exists yet | Chapters REPORTED delivered (8 of 8); QA not complete | DEC-021 (publication of PDF, figures, evidence), QA completion, lead read-through |
| **B06** | UI functional logic and unit tests | UI: logic, hooks, fixtures | 78 (0/78) | 520 KiB | **HELD FOR REVIEW** | First UI commit (B07 and B08 import it, VERIFIED by import scan). No dependency on other batches | FLIB COMPLETE (REPORTED): 159 new tests; lead ran whole suite (REPORTED 455 of 458 then, later 483 of 483 after V1). Unit-tested only, not wired, no browser check. Fixtures in `ui/test/fixtures/func/sandbox_u01/` are SANDBOX-PATCHED (backend shape not integrated) | Lead review of FLIB diffs; independent U-01 review still running (DEC-001) for the sandbox fixtures only; DEC-007 applied (REPLAYED semantics) |
| **B07** | UI design-system foundations | UI: tokens, shell styles, shared components, token/motion/forbidden checks | 18 (0/18) | 136 KiB | **HELD FOR REVIEW** | After B06 (components import `lib/routes`, `connection`, `theme`, `viewmodel`, `provenance`, `format`: 7 edges). Can be merged with B06 into one 96-file UI commit if the lead prefers | V1 COMPLETE (REPORTED); lead VERIFIED UI suite 483 of 483, contrast 284 of 284, a11y 0 issues, forbidden pass (tracker 16:55); lead diff review pending; css and `tokens.test.mjs` touched 13 to 15 minutes ago | Lead diff review. No owner decision (within DEC-017) |
| **B08** | Lab, evidence, Simulation, Overview screens and app wiring | UI: screens, charts, wiring, render/harness tests, accessibility (static) | 41 (3/38) | 379 KiB | **BLOCKED**: Phase B (Lab, evidence, Simulation redesign and FLIB wiring) NOT STARTED (V2 writer not briefed); contains the pre-Phase-B I1-I4 screens | After B06 and B07 (App.jsx imports 105 files of B06 and 44 of B07). `ui/test/index.js` aggregates the tests of B06, B07 and B08, so it is in B08 | I1-I4 IMPLEMENTED and TESTED, "NOT visually approved; O8-2 OPEN" (REPORTED) | Phase B completion; DEC-001 for validated U-01 result page; human visual review baseline DEC-008 (reviewers not named). Alternative (lead choice): commit the I1-I4 state as an interim, not recommended because Phase B rewrites these files |
| **B09** | UI verification scripts, axe/a11y runners, package manifests | Testing and validation infrastructure; accessibility | 25 (2/23) | 302 KiB | **HELD FOR REVIEW** | After B08 (`check-a11y.mjs` imports `render-harness.mjs`, `a11y-lint.mjs`; both in B08) | `check-axe.mjs` self-tested, no baseline yet; `package.json` diff has earlier unrelated edits (RK-5); 20 `browser-*` scripts are exploratory probes | Lead review of the whole `package.json` and lock diff (DEC-014 approved axe-core 4.14.0 dev-only); decide whether the 20 `browser-*` probes are kept |
| **B11** | Rust backend R1: `src/**`, `Cargo.toml` (dependencies and `[[bin]]`), `Cargo.lock` | Backend, distributed, security, intelligence, lab library (owner-reserved) | 60 (12/48) | 1,223 KiB | **BLOCKED**: DEC-022 (AWAITING OWNER: commit and publish with open G4 findings); DEC-005 (`src/lab/rng.rs`, held, required by `lab/mod.rs`, so B11 gains 1 file on release); DEC-006 hash procedure for `Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`; sandbox merges pending (U-01 touches `lab_live/scenario_b.rs`, `api/lab_api.rs`; M3 touches router/main/config; G4A-01 fix touches `cache.rs`) so the files will change | First Rust commit. Interlocked through `lib.rs`, `main.rs`, `Cargo.toml`, `server.rs` (old plan section 3) | G4 Cycle 1 AI second looks: 99 findings, verdict YES WITH FIXES, several OPEN and owner-named (ODR-4); frozen-code findings recorded as limitations (ODR-1, DEC-009); earlier M2/M3/M7 reading reviews. Test baseline 851 passed, 0 failed, 8 ignored (debug, 2026-10-30 10:11-10:23, REPORTED; not re-run here) | DEC-022, DEC-005, DEC-006, DEC-020 (code strings), DEC-001, DEC-004; protected-core final approval; per CLAUDE.md section 2, the 8 new runtime and 6 dev dependencies must have approval (they are specified in `docs/security/phase-1-specification.md`; approval record not re-verified, gap G-7) |
| **B12** | Rust tests R2: `tests/**` (and `[[test]]` stanzas) | Testing | 96 (3/93) | 1,434 KiB | **BLOCKED**: needs B11; `Cargo.toml` has a `[[test]]` stanza for `m6_research_evaluation_tests.rs` (held, DEC-005), so the manifest will not load without that file (VERIFIED: 72 stanzas: 70 in B12, 1 already tracked, 1 held); DEC-022 | After B11 | Vacuous/flaky tests identified and partly fixed (ledger, REPORTED). Passing tests do not establish correctness and do not authorise a commit | DEC-022, DEC-005, DEC-006 |
| **B13** | Benches, packaging, scripts R3 | Testing and release infrastructure | 23 (0/23) | 68 KiB | **BLOCKED** by B11 (benches compile against the crate; 7 `[[bench]]` stanzas by name) and DEC-022 | After B12 (or B11) | None beyond self-report; no benchmark results are claimed | DEC-022 |
| **B14** | Release plan records | Documentation: release records | 2 now (0/2); +1 plan, +20 lists | 53 KiB (+ lists) | **READY** (commit with or after B04) | No dependencies. `docs/release/g5_working_tree_classification.md` section 6 holds the DEC-005 offline procedure and should land before the register is relied on | This plan and lists (this task) | Lead decides whether to add a SUPERSEDED notice to `g5_commit_plan.md` before committing (it is stale) |

Files not in any batch are in HELD lists H01 to H04, NEVER, and the not-committed classes (section 7).

### 2.1 Proposed commit messages

All end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Test/validation numbers are REPORTED unless noted and must be re-run and replaced by the lead at commit time.

- **B02** `docs(research): add research, audit, ADR, vision and phase records` Body: assessments, ADRs, claims ledger, G1/G2 verdicts, accountability report, progress ledger, documentation audits, three-pillar vision package, Phase 1 to 8 packages and reports. Agent-authored; statuses are as recorded and several earlier PASS statements are retracted inside. Excludes preregistration, freeze and sealed records (held, DEC-005), patches and sandbox investigations. Validation: docs-lint on an index copy, no new secret-adjacent lines beyond the 22 listed (section 6). Refs: ODR-3, DEC-023.
- **B04** `docs: add user, security, handoff, release and demo-lab documentation, docs index and status; add notices to 15 historical documents` Body: `docs/STATUS.md` entry point, docs index, glossary, contributing, testing strategy, user guides, security specs and audits, release records (verification register, workstream ledger, G4 Cycle 1 reports and disposition register, known limitations for frozen code), handoff set without transcripts. The 15 notice edits add 4 lines each and delete nothing. Known stale statements flagged by DEC-020 are NOT corrected here. Validation: docs-lint delta as in section 6. Refs: ODR-1, ODR-3, DEC-009, DEC-020, DEC-023.
- **B01** `docs(design): add design direction, rules, rubric, redesign governance and implementation records` Body: design system spec, direction v2, information architecture, tokens, Lab API contract, critique rubric and external-skill rules, remediation reports 00 to 08 and increments (F, I1 to I4, V1), decision register and execution tracker, design-toolkit research R1 to R5 and proposal, non-image prototype files. Screenshots stay untracked. Refs: DEC-015, DEC-016, DEC-017, DEC-018, `docs/design/remediation/07_frontend_redesign_implementation_record.md`.
- **B03** `docs(research): record G4A-01, M3 durability, U-01 backend and TTL sandbox investigations and phase patches` Body: proposals and evidence only; nothing is integrated into `src/`. Two phase patches are UTF-16LE. Refs: ODR-2, ODR-4, DEC-001, DEC-004.
- **B10** `docs(evidence): add curated browser, Scenario B and remediation verification reports (no screenshots)` Body: reports, manifests, console/network logs and results from exploratory debug-build runs; one host; not independent verification. Refs: RK-2, owner A-8.
- **B05** `docs(learning): add the engineering and technical-defence handbook (source, figures, evidence, build)` Body: eight chapters, 32 SVG figures, per-chapter evidence files, front matter and style rules, build script. Records discrepancies between documents and code without changing either. Refs: DEC-020, DEC-021.
- **B06** `feat(ui): add functional logic for connection state, gateway address, progress and result presentation, with tests and fixtures` Body: `ui/src/lib`, `ui/src/hooks` and 14 test files plus fixtures; withholds mechanism verdicts for incomplete runs on the client (backend fix not integrated; `sandbox_u01` fixtures are labelled SANDBOX-PATCHED). Validation: `node --test` for the listed files (rerun and quote; tracker reports 159 new tests, suite 483 of 483 after V1). Refs: U-01, DEC-001, DEC-007, `increment_F_functional_logic.md`.
- **B07** `feat(ui): add the Instrument-and-Evidence design-system foundations (tokens, shell, shared components) with token, motion and forbidden-pattern checks` Validation to quote after rerun: UI suite 483 of 483, contrast 284 of 284, a11y lint 0 issues, forbidden-constant scan pass (lead-run per tracker; debug Node; static checks only). Refs: DEC-017, `increment_V1_foundations_v2.md`.
- **B08** `feat(ui): add Overview, Traffic, Freshness, Origin, Cache, Simulation and Phantom Lab screens with app wiring and render tests` (wording to be replaced after Phase B). Provenance badges label each value measured, simulated, inferred or unavailable. Refs: DEC-007, DEC-011, DEC-017.
- **B09** `chore(ui): add browser verification scripts, axe-core a11y runner and package manifests` Body: axe-core 4.14.0 dev-only (DEC-014); exploratory probes are not a baseline. Refs: DEC-008, DEC-014, RK-5.
- **B11** `feat(backend): add freshness engine, eviction policies, byte accounting, telemetry, security, mTLS cluster, intelligence machinery, lab library and Lab API; register modules and dependencies` Body: `pie: None` stays in main; no benefit from adaptive control is claimed; open G4 findings are listed in `docs/release/g4/` and `docs/release/known_limitations_frozen_code.md`. Validation to quote after a clean index copy: `cargo fmt --check`, `cargo clippy --all-targets -j 1 -- -D warnings`, `cargo check --all-targets --locked -j 1` (R1 manifest has no `[[test]]`/`[[bench]]` stanzas). Refs: DEC-022, DEC-005, DEC-006, ODR-1, ODR-3, ODR-4.
- **B12** `test: add integration, unit and support harnesses for gateway, cluster, lab, security and intelligence` Body: includes tests that pin known-gap behaviour on purpose (`known_gaps.rs`). Validation: `cargo test --tests -j 1 --no-fail-fast` on the index copy (rerun; baseline 851 passed, 0 failed, 8 ignored is debug, REPORTED). Refs: DEC-022, DEC-006.
- **B13** `chore(bench,pack): add criterion benches, packaging and reproduction scripts` Body: no benchmark results included or claimed. Validation: `cargo bench --no-run -j 1`. Refs: DEC-022.
- **B14** `docs(release): add the G5 classification, superseded commit plan, batch plan v2 and path lists` Refs: ODR-3.

## 3. Order

Constraints found by scanning relative links in the 7 documentation batches and imports in the UI batches (VERIFIED).

1. **Documentation wave (no code dependency): B02, B04 (READY now), then B01, B03, B10 when cleared, then B05, B14.** Minimum forward references: order B01, B03, B02, B04, B05, B10, B14 gives 29 links that point to a batch committed later; the readiness-driven order B02, B04, B01, B03, B05, B10, B14 gives 74. The lead may merge B02 and B04 into one 173-file docs commit to remove 27 + 107-direction forward links (not recommended: different review states).
2. **UI wave: B06, B07, B08, B09.** The tracker planned V1 foundations before FLIB logic, but V1 components import files that live in the FLIB batch (`Shell.jsx` imports `lib/routes`, `connection`, `theme`; `Metric.jsx` imports `lib/viewmodel`; `ProvenanceBadge.jsx` imports `lib/provenance`; `FieldTable.jsx` imports `lib/format`), and `lib/connection.js` was edited by FLIB. So logic first. B06 + B07 can be one 96-file commit once both reviews pass. `ui/test/index.js` imports every test, so it ships with B08; run B06 and B07 tests by explicit file name in their staged copies.
3. **Rust wave (BLOCKED): B11, B12, B13** as the old plan Option B (section 3). Pinned-file consequence in section 4.7.
4. B10 and B05 can be committed any time after their blocks clear; B14 any time.

## 4. Verification procedure for the lead (per batch)

Use Git Bash. `L` = the batch list file, `T` = a scratch directory outside the repository (for example `D:/Temp/g5_v2`). Nothing here writes into the repository except `git add` and `git commit` at the steps marked STAGE and COMMIT. Never `git add .`, `-A`, directories or wildcards; never `git reset`, `git restore`, `git checkout --`, `git clean`, `git stash`.

```
# 0. pre-flight
git rev-parse HEAD; git branch --show-current
git diff --cached --name-only | wc -l                       # must be 0
git status --short -uall > $T/before.txt                    # keep for step 12
# 1. regenerate: strip comments, compare with live status (stale list guard)
grep -v '^#' "$L" | grep -v '^$' | sort > $T/list.txt
git status --short -uall | cut -c4- | sort > $T/live_paths.txt
comm -23 $T/list.txt $T/live_paths.txt                      # must print nothing (every path still a pending change)
# 2. STAGE (explicit list, literal pathspecs)
GIT_LITERAL_PATHSPECS=1 git add --pathspec-from-file=$T/list.txt
# 3. exactness
git diff --cached --name-only | sort | diff - $T/list.txt && echo SAME   # must print SAME
git diff --cached --name-status | cut -c1 | sort | uniq -c              # A and M counts must equal the M/new columns of section 2
git diff --cached --check                                               # whitespace; CRLF warnings expected only for files listed in section 5
# 4. ignore and size
git diff --cached --name-only | git check-ignore --stdin                # must print nothing (note: *draft*.md is globally ignored)
git diff --cached --name-only -z | xargs -0 -I{} git cat-file -s ":{}" | awk '$1>5242880' | wc -l   # must be 0 (5 MiB limit)
# 5. hex and token scan, counts only (do not print matches)
git diff --cached -U0 | grep '^+' | grep -Ec '(^|[^0-9a-fA-F])[0-9a-fA-F]{64}([^0-9a-fA-F]|$)'
git diff --cached -U0 | grep '^+' | grep -Ec 'AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9]{20,}|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.|BEGIN [A-Z ]*PRIVATE KEY'   # must be 0
# per-file hex counts must equal the figures in section 5; additionally compare staged 64-hex values with the held DEC-005 values
#   (script: collect 64-hex from the H03 list files, then for each staged file print only the file name if any value matches; expected: none)
# 6. absolute-path and owner-name counts (counts only)
git diff --cached -U0 | grep '^+' | grep -Ec '[A-Za-z]:[\\/](Users|Personal|Temp|ClaudeTemp)'   # compare with section 5 (cited evidence locations)
# 7. docs-lint on an index copy (writes only under $T)
rm -rf $T/idx $T/head; mkdir -p $T/head
git checkout-index -a --prefix=$T/idx/                         # index = HEAD + staged batch; trailing slash is required
git archive HEAD | tar -x -C $T/head                           # HEAD only
node scripts/docs-lint.mjs --root $T/idx  --json > $T/lint_after.json
node scripts/docs-lint.mjs --root $T/head --json > $T/lint_before.json
node -e "const k=f=>f.rule+'|'+f.file+'|'+f.detail;const a=require('$T/lint_after.json'),b=require('$T/lint_before.json');const B=new Set(b.findings.map(k)),A=new Set(a.findings.map(k));const c=(x,S)=>x.findings.filter(f=>!S.has(k(f))).reduce((r,f)=>(r[f.rule]=(r[f.rule]||0)+1,r),{});console.log('added',c(a,B),'gone',c(b,A))"
# 8. code batches only: cargo on the index copy (CARGO_TARGET_DIR outside the repo, -j 1)
# 9. pinned files (B11, B12): section 4.7
# 10. COMMIT (message from section 2.1, trailer included)
# 11. push per DEC-023 (fast-forward, feature branch only) unless DEC-022 option C applies to code batches
# 12. after: git status --short -uall must equal $T/before.txt minus exactly this list; HEAD advanced by one commit
```

Interpreting the docs-lint delta (step 7). docs-lint is report-only and exits 0. Expected deltas are in section 6. Accept: added `link-broken` equal to the forward-reference figure for the chosen order; `abs-path` and `secret-adjacent` equal to the figures in section 5 and 6 (these are documented evidence locations, rule-and-line only, matches never printed). Stop and investigate: any `link-broken` that points to a file that IS in the batch or in HEAD; any `secret-adjacent` line not in the section 6 list; any new `h1-missing` beyond the one known (`docs/design/prototypes/perf/perf_tables.md:1`); a `status-vocab` or `notice-missing` finding on a file that is not in the list.

CRLF notes. `core.autocrlf=true` and the only `.gitattributes` entries are the four `-text` pins (DEC-006). Working-tree files for the 15 notice edits (B04), `ui/index.html`, `App.jsx`, `main.jsx`, `ui/package*.json`, `Cargo.lock` and others report "LF will be replaced by CRLF" on add; that is normal and committed content is LF. 15 files in B04 are fully CRLF in the working tree; 8 files in B02, 1 in B04, 3 in B10, 1 in B12 and 4 in B03 have mixed or CRLF endings (section 5). Do not renormalise. Two patches in B03 (`phase_5_changes.patch`, `phase_6_changes.patch`) are UTF-16LE and are treated by git as binary.

### 4.7 Hash-pinned file procedure (B11, B12; DEC-006 done, `.gitattributes` committed in `df68eaa`)

Pinned files: `Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, `tests/integration/m6_research_evaluation_tests.rs`. The addendum `docs/research/preregistration_m6_v2_2_addendum_1.json` records `current_sha256` for each.
VERIFIED now (booleans only, nothing printed): the working-tree bytes of all four files equal the recorded `current_sha256`.
Before every commit that stages any pinned file, from the repository root, compare index blobs, not working files:

```
node -e "const fs=require('fs'),cp=require('child_process'),h=require('crypto');const j=JSON.parse(fs.readFileSync('docs/research/preregistration_m6_v2_2_addendum_1.json','utf8'));const seen=new Set();(function w(o){if(o&&typeof o==='object'){if(!Array.isArray(o)&&o.current_sha256&&(o.path||o.file)){const p=o.path||o.file;if(!seen.has(p)){seen.add(p);let ok;try{ok=h.createHash('sha256').update(cp.execFileSync('git',['show',':'+p],{maxBuffer:1<<28})).digest('hex')===o.current_sha256}catch{ok='not-in-index'}console.log(p,ok)}}for(const k in o)w(o[k])}})(j)"
```
Expected: `true` for each pinned path that is staged; `not-in-index` for the others. A `false` stops the commit. After the commit repeat with `git show HEAD:<path>`. Never print or copy the JSON values.
Consequence for Option B (three Rust commits): the R1 `Cargo.toml` (dependencies and `[[bin]]` only, no `[[test]]`/`[[bench]]`) differs from the pinned bytes, so the pinned hash can only match at the last commit that touches `Cargo.toml` (R3), and cargo cannot load the full manifest before the test and bench files exist. This is gap G-5 (owner decision, no ID). Alternative: one atomic Rust commit B11+B12+B13 plus the two released files (about 181 files, 2.7 MiB, 150+ allowed as one coherent deliverable) where the pinned hash matches in a single step. INFERENCE: either needs the lead to build the R1 and R2 `Cargo.toml` states by editing a copy and `git apply --cached`, as in the old plan.

## 5. Per-batch verification results (VERIFIED by this agent at 16:41 IST)

Checks run on every listed path: existence; `git check-ignore --stdin` (batch-wide); size; 64-hex scan (regex `[0-9a-f]{64}` with non-hex boundaries); absolute-path regex (`[A-Za-z]:[\\/](Users|Personal|Temp|ClaudeTemp)` and `/c|d/(Users|Personal|Temp)`); token regex (AKIA, ghp_, sk-, JWT shape, PEM private key header). Matches were counted only, never printed. Also: no path appears in two batches (the classifier reported 0 duplicates; counts add to 4,644). Hex values in every batch file were compared with the 93 distinct 64-hex values found in the 16 held DEC-005 files (which hold both commitment strings per DEC-005): **0 matches**. No file in any list exceeds 5 MiB (largest: see B10 total 5,007 KiB across 212 files, each under 1 MiB). No `.pyc`, image, log with NUL bytes (0 binary files) in any batch.

| Batch | Missing | Ignored | Over 5 MiB | Files with 64-hex (matches) | Files with abs-path (matches) | Token hits | Other |
|---|---:|---:|---:|---|---|---:|---|
| B01 | 0 | 0 | 0 | 5 (8) | 10 (16) | 0 | `docs/design/remediation/{05_before_after_comparison,06_functional_ux_work_breakdown,increment_I3_simulation}.md`, `design_toolkit/{r1_*,toolkit_proposal}.md`; 1 sealed-directory path mention; 2 files name the Windows user |
| B02 | 0 | 0 | 0 | 16 (54) | 6 (25) | 0 | `g1_verdict`, `m6_research_evaluation`, `master_audit_and_execution_plan`, `phase_1..6_{completion_report,progress}` (those that exist), `progress_ledger`, `storage_cleanup_2026-10-30` (file hashes in ledgers); 8 files mixed LF/CRLF; 4 files mention the sealed-directory path (location only) |
| B03 | 0 | 0 | 0 | 8 (1,159) | 2 (6) | 0 | patches and `phase_1_snapshot_manifest.md` (diff content and file hashes), `u01_backend/defect_reproduction.md` (1); 2 UTF-16LE, 4 CRLF; 6 files name the Windows user (paths in diffs) |
| B04 | 0 | 0 | 0 | 2 (3) | 21 (183) | 0 | `docs/demo-lab/trust-and-authenticity-design.md`, `docs/security/security-demo-matrix.md`; 15 notice edits fully CRLF in the working tree, 1 mixed; docs-lint secret-adjacent 15 lines (section 6) |
| B05 | 0 | 0 | 0 | 0 | 4 (5) | 0 | 2 files mention the sealed-directory path; 1 secret-adjacent line (`00_STYLE_AND_RULES.md:12`) |
| B06 | 0 | 0 | 0 | 12 (27) | 2 (10) | 0 | fixtures `lab/*.json`, `func/PROVENANCE.json` (9), `func/legacy/*.run.json`, `lab.view.test.mjs`: artifact and file hashes inside recorded fixtures |
| B07 | 0 | 0 | 0 | 0 | 0 | 0 | |
| B08 | 0 | 0 | 0 | 1 (1) | 0 | 0 | `ui/test/lab.render.test.mjs` |
| B09 | 0 | 0 | 0 | 0 | 7 (20) | 0 | 7 browser scripts mention evidence locations; `ui/package.json` and lock are CRLF in the working tree |
| B10 | 0 | 0 | 0 | 14 (1,469) | 15 (65) | 0 | `gateway_sha256.txt`, `environment.json`, `journey_*.results.json(l)`, `report.md`, `after|before|i2_preview/manifest.json` (image hashes); 3 secret-adjacent lines in 2 `report.md`; 3 files mixed endings |
| B11 | 0 | 0 | 0 | 1 (287) | 0 | 0 | `Cargo.lock` (crate checksums). `src/lab/rng.rs` (held) is the file with commitment strings |
| B12 | 0 | 0 | 0 | 2 (4) | 0 | 0 | `canonical_key_roundtrip_tests.proptest-regressions`, `security_phase3_cluster_tests.rs` |
| B13 | 0 | 0 | 0 | 0 | 0 | 0 | |
| B14 | 0 | 0 | 0 | 0 | 1 (2) | 0 | 2 files mention the sealed-directory path; 1 secret-adjacent line (`g5_commit_plan.md:67`) |
| H01 | 0 | 0 | 0 | 0 | 1 (15) | 0 | `MILESTONE_1_DESIGN_PLAN.md` 3 secret-adjacent lines, 15 drive-path matches |
| H02 | 0 | 0 | 0 | 0 | 0 | 0 | 2 modified; one mixed endings |
| H03 | 0 | 0 | 0 | 10 (192) | 1 (4) | 0 | commitment and file-hash material; 4 secret-adjacent lines in 2 files |
| H04 | 0 | 0 | 0 | 0 | 0 | 0 | `lead_working_state.md` header names the scratchpad helper path (1 match of the word ClaudeTemp; not counted by the drive-path regex because of the text form) |
| N00 | 0 | 0 | 0 | 0 | 0 | 0 | `CLAUDE.md` |

M versus new per batch is in section 2. Interpretation: all hex hits are file or artifact hashes, crate checksums or diff content inside records, with no overlap with the held DEC-005 set (INFERENCE: none is a secret; the only known sensitive material, the salt, lives outside the repository and was not read). Absolute paths are evidence locations of the owner's machine (`D:\Personal\...`, `D:\Temp\...`); publishing them discloses the machine layout and the Windows user name (DEC-023 makes each commit public on push). Count only; docs-lint reports 140 abs-path findings repository-wide.

## 6. docs-lint relation (VERIFIED, `node scripts/docs-lint.mjs --json`, report-only; HEAD copy via `git archive HEAD`, batch files overlaid from the working tree)

HEAD baseline: 31 files, 66 errors, 49 warnings, 1 info (rules: dup-basename 10, orphan 28, reviewed-missing 29, status-missing 28, path-ref-missing 11, fact-source-missing 1, abs-path 8, status-vocab 1; link-broken 0).
Live working tree for reference: 1,058 files, 782 errors, 450 warnings, 23 info.

Individual effect of each documentation batch on top of HEAD (added findings; "gone" are baseline findings that disappear):

| Batch | link-broken added | abs-path added | secret-adjacent added (file:line) | Other notable added | Gone |
|---|---:|---:|---:|---|---|
| B02 | 62 | 15 | 22: `doc_audit/d2_*:291`, `d4_*:80`, `phase_1_completion_report:79`, `phase_1_execution_package:60`, `phase_1_progress:41`, `progress_ledger:142,144,155,159,162,170,178,182,188,190,210,218,220,230,246,248`, `vision/v4_*:93` | path-ref-missing 150, notice-missing 28, fact-drift 57, size 12 | path-ref-missing 1 |
| B04 | 203 | 82 | 15: `demo-lab/README:73,75,77`, `demo-lab/security-threat-model:76,127`, `release/execution_register:8`, `release/verification_register:52`, `security/authenticity-design-options:68`, `security/investigations/DR-029_review_notes:46`, `security/phase-4-design:29,67,121,176,200`, `security/stage-3c-progress:123` | path-ref-missing 243, notice-missing 14 | fact-source-missing 1, path-ref-missing 1 |
| B01 | 0 | 33 | 1: `research/design_toolkit/r3_engineering_a11y_testing:19` | h1-missing 1 (`prototypes/perf/perf_tables.md:1`, known), path-ref-missing 339 | path-ref-missing 5 |
| B03 | 0 | 19 | 0 | dup-basename 6, path-ref-missing 49 | none |
| B10 | 0 | 0 | 3: `browser_verification/20261030T041006Z/report:34`, `20261030T045605Z/report:6,67` | | none |
| B05 | 0 | 10 | 1: `learning/src/00_STYLE_AND_RULES.md:12` | path-ref-missing 548, h1-missing 1, size 7 | none |
| B14 | 0 | 8 | 1: `release/g5_commit_plan.md:67` | path-ref-missing 51 | none |
| H01 | 1 | 16 | 3 | | none |
| H02 | 7 | 0 | 0 | size 1 | none |
| H03 | 0 | 1 | 4: `preregistration_m6_v2:99,100`, `preregistration_m6_v2_1_amendment:23,24` | | none |
| H04 | 0 | 6 | 0 | | none |
| B06 to B09, B11 to B13 | no Markdown files: no docs-lint delta (INFERENCE from rules that scan documents; B09 `.mjs` files not scanned) | | | | |

Cumulative, proposed readiness order B02, B04, B01, B03, B10, B05, B14 on top of HEAD (total `link-broken` after each step): B02 62; +B04 127 (92 added, 27 resolved); +B01 97 (30 resolved); +B03 85 (12 resolved); +B10 73 (12 resolved); +B05 73; +B14 73. **The final 73 remain until other decisions land and are all intentional:** `CLAUDE.md` 23 (untracked file; owner rules, N00), preregistration and sealed targets 25 (H03, DEC-005), paths into `src`/`tests`/`scripts` 15 (resolve with B11 to B13), `CORRECTIONS.md` and `MILESTONE_1_DESIGN_PLAN.md` 5 (H01), `bench-results`/`lab-results` README 3 (evidence bulk, never tracked), the globally ignored `intelligence_e2_headroom_protocol_DRAFT.md` 2 (linked from `docs/research/README.md:82` and the workstream ledger). Committing H02 (README, PHASE_2) would add 7 broken links on its own.
The `*draft*.md` global ignore (`C:\Users\Satish\.gitignore_global:34`) hides `docs/research/intelligence_e2_headroom_protocol_DRAFT.md` from `git status`; it exists on disk, is in no list, and needs gap G-6 (VERIFIED: file exists; `git check-ignore` match).

## 7. Files in no batch, and the arithmetic (VERIFIED at 16:41:31 IST)

| Class | Count | Reason |
|---|---:|---|
| **H01** `CORRECTIONS.md`, `MILESTONE_1_DESIGN_PLAN.md` | 2 | owner rule/plan files: need a specific decision (gap G-2); 5 doc links point at them |
| **H02** `README.md` (M), `docs/PHASE_2_WORK_REPORT.md` (M) | 2 | README positioning changes await DEC-020 (option A edits README); PHASE_2 notice says "unchanged" but 223 lines were rewritten (owner must read the diff; old plan section 6). Gap G-3 |
| **H03** preregistration, amendments, freeze JSONs, addendum and `.pin`, `m6_sealed_results.md`, `phase_5_preregistration.md` (12 files under `docs/research/`), `lab-results/tuning/frozen_arm_params.json`, `lab-results/tuning/tuning_trials.csv`, `src/lab/rng.rs`, `tests/integration/m6_research_evaluation_tests.rs` | 16 | DEC-005 (AWAITING OWNER): two 64-hex commitment strings; the pinned M6 test and `rng.rs` also embed them |
| **H04** `docs/release/lead_working_state.md`, `docs/release/lead_timeline.md` | 2 | the lead's living memory aids; contain scratchpad paths; still being appended (timeline touched 6 minutes before snapshot). Lead decides (no ID needed unless the owner wants them public) |
| **N00** `CLAUDE.md` | 1 | NEVER staged without a specific decision (gap G-1; relation to DEC-020 option A, which would add a dated errata block); 23 doc links point at it |
| **S99** `docs/release/commit_lists/*` (24 old lists) | 24 | superseded by `commit_lists_v2/`; stale; recommended not to commit |
| **X90** evidence bulk | 2,015 | 782 browser-verification PNG/JPG, 12 prototype PNG, 109+210+210+20+220+6+120+42+49 = 986 remediation images (`a11y_evidence`, `after`, `before`, `i1..i4_preview`, `usability_evidence`, `v1_preview`), `docs/design/remediation/legacy/SimDemo.jsx.orig` 1, `bench-results/` 19, `lab-results/` 215 (excluding the 2 tuning files); 279.6 MiB; owner approval A-8 |
| **X91** `.local/snapshots/**` | 1,701 | local snapshots; also hold transcript copies and `secret_*.tsv` files; never staged |
| **X92** `transcript_copy.jsonl`, `docs/handoff/transcripts/*.jsonl` (2) | 3 | ledger hard exclusion (PII, secrets possible) |

Arithmetic: batches 85 + 88 + 55 + 42 + 212 + 53 + 78 + 18 + 41 + 25 + 60 + 96 + 23 + 2 = **880** (B02, B04, B01, B03, B10, B05, B06, B07, B08, B09, B11, B12, B13, B14). Not in a batch: 2 + 2 + 16 + 2 + 1 + 24 + 2,015 + 1,701 + 3 = **3,764**. **880 + 3,764 = 4,644** = live total (37 M + 4,607 untracked). The 37 M files: B04 15, B08 3, B09 2, B11 12, B12 3, H02 2. Checked: 0 deleted, 0 renamed, 0 staged. Not in the 4,644: the ignored `intelligence_e2_headroom_protocol_DRAFT.md` and ignored logs under `.local/` (not shown by `git status`).

## 8. Items that stay out of every commit by rule
`.local/`, transcripts, `bench-results/`, `lab-results/` except the held tuning pair, image evidence, `target/`, `ui/node_modules`, `ui/dist`, the sealed directory (not read), CLAUDE.md, held files. `git add .`, `-A` and directory adds are forbidden.

## 9. Decisions this plan needs

| Gate | ID | Status | Batches affected |
|---|---|---|---|
| Classify the two commitment strings (offline check) | **DEC-005** (HIGH, AWAITING OWNER; method chosen, results pending) | exists | H03; `rng.rs` joins B11, the M6 test joins B12; also blocks B11/B12 compile |
| Commit and publish Rust code with open G4 findings | **DEC-022** (HIGH, AWAITING OWNER; recommendation option C: commit, do not push until G4B-01 and G4A-01 have approved fixes) | exists | B11, B12, B13 |
| Documentation corrections and stale rules (README, handoff, ADR-001, CLAUDE.md errata, two code strings) | **DEC-020** (MEDIUM, AWAITING OWNER) | exists | H02 (README), N00, B02 (ADR-001), B04 (handoff), code strings in B11 |
| Handbook publication | **DEC-021** (LOW, NOT YET READY) | exists | B05 |
| U-01 backend integration | **DEC-001** (CRITICAL, BLOCKED ON REVIEW) | exists | B06 fixtures (sandbox shape), B08, B11 (`scenario_b.rs`, `lab_api.rs`), B03 |
| Protected-core merges (M3, G4A-01 fix, DR-029/027, quotas, addendum 2) | **DEC-004** (HIGH, BLOCKED ON REVIEW) | exists | B03 (records only), B11 |
| Hash-pinned file procedure | **DEC-006** (approved, done) | exists | B11, B12 (section 4.7) |
| JWT telemetry auth; new telemetry/dependencies | DEC-002, DEC-003 | exist, not ready | none now |
| Push policy | DEC-023 (approved) | exists | all batches publish on commit |
| axe-core dev dependency; rubric; external rules | DEC-014, DEC-015, DEC-016 (approved) | exist | B09, B01 (content review still pending) |
| **G-1** Commit `CLAUDE.md` or leave it untracked and accept 23 dangling links | **no ID** (lead to create) | gap | N00, B02, B04, B05 |
| **G-2** Commit `CORRECTIONS.md` and `MILESTONE_1_DESIGN_PLAN.md` | **no ID** (old plan: owner written approval; plan "read fully before staging") | gap | H01 |
| **G-3** Commit `README.md` and `docs/PHASE_2_WORK_REPORT.md` as modified | **no ID** (partly DEC-020; PHASE_2 retitle/rewrite needs owner read) | gap | H02 |
| **G-4** Track curated JSON/MD/log evidence (4.89 MiB) while images stay untracked | **no ID** (earlier owner approval A-8 covers keeping bulk untracked only) | gap | B10 |
| **G-5** Rust commit shape versus the pinned `Cargo.toml` (three commits with partial intermediate manifests, or one atomic commit; manifests cannot load without all stanza files) | **no ID** | gap | B11, B12, B13 |
| **G-6** The globally ignored `*_DRAFT.md` file linked from `docs/research/README.md:82` (force-add explicitly or edit the link) | **no ID** | gap | B02 |
| **G-7** Approval record for the 8 new runtime and 6 new dev dependencies (CLAUDE.md section 2); they are specified in `docs/security/phase-1-specification.md` but no register entry was found | **no ID** (not re-verified) | gap | B11 |

## 10. Top risks
1. B11 to B13 lists will change after M3, U-01, G4A-01 and DEC-020 code edits land; the lists are valid only for the working tree at 16:41.
2. Push-on-commit (DEC-023) publishes absolute paths (183 matches in B04 alone) and the Windows user name; docs-lint treats abs-path as error-level.
3. The 22 + 15 + 3 + 1 secret-adjacent lines are documentation about secrets, paths or salts (not values); the lead should skim them once before the first push.
4. B05 and B03 are still changing; staging them early would freeze partial work.
5. Dependency inversion (V1 components import FLIB-edited logic) means the tracker's planned order (foundations before logic) cannot compile or test.
6. A passing suite does not authorise a commit; the 851/483/284 numbers above are REPORTED and debug-profile.
