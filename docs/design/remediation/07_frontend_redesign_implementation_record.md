# Frontend redesign: implementation record

**Status:** Active (the one continuously updated record; lead-owned). **Audience:** product owner, reviewers, implementing agents. **Started:** 2026-10-30 15:59 (+05:30, machine clock). **Last updated:** 2026-10-30 16:45 (+05:30, machine clock). **Authority:** owner approval and implementation brief of 2026-10-30 ("UI/UX Toolkit Adoption and Product Redesign"). Design authority: `apple-design` plus DD v2 (`docs/design/phantomcache_design_direction_v2.md`). Status classes: IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, BLOCKED, ACCEPTED, DEFERRED. Nothing here is independently verified; O8-2 (design approval) stays open until human review and the owner's explicit approval.

**Governance files:** owner decisions live in `docs/design/OWNER_DECISION_REGISTER.md`; agents, critical files, dependencies and risks live in `docs/design/REDESIGN_EXECUTION_TRACKER.md`. This record keeps the narrative and does not duplicate their status.

## 1. Objective, in plain words
Make the PhantomCache UI a credible professional instrument: clear hierarchy, honest provenance, smooth purposeful motion, accessible and responsive in light and dark, with strong storytelling built only on what the system can truthfully show. Backend behaviour, protected files and evidence are not changed by this programme.

## 2. Architecture (unchanged by design)
React 18 + Vite 5 single-page app; plain CSS with tokens (`ui/src/styles/*.css`); hand-built SVG charts with table twins; dependency-free CDP scripts (`ui/scripts/cdp-lib.mjs`) for browser evidence; `node:test` tests. Only dev-dependency change: axe-core 4.14.0.

## 3. Work breakdown and status (the owner's 15 steps)
| Step | Work | Status | Evidence |
|---|---|---|---|
| 1 | Read project instructions, DD v2, apple-design | DONE (lead) | this record, section 7 |
| 2 | R4 findings mapped to design decisions | RUNNING (agent TRACE) | `docs/design/remediation/08_r4_traceability.md` (pending) |
| 3 | Install and verify the four animation skills | IMPLEMENTED and TESTED (blob and SHA-256 verified); fresh-session discovery: see 5.1 | `.claude/skills/{animation-performance,review-animations,remove-animations,find-animation-opportunities}/` |
| 4 | axe-core integration (dependency, `check-axe.mjs`, tests) | IMPLEMENTED and TESTED (tool self-test; full baseline pending) | section 5.2 |
| 5 | Project-owned UX critique rubric | IMPLEMENTED (needs human review) | `docs/design/critique_rubric.md`; `docs/design/external_design_skills_project_rules.md` |
| 6 | Audit design system, routes, components, defects | PARTLY DONE earlier (02 audit, 03/04 validation, 05 before/after); refreshed by rubric runs | `docs/design/remediation/` |
| 7 | Tokens and shared component foundations | IMPLEMENTED and TESTED (V1; UI suite 483 of 483, contrast 284 of 284); held for lead diff review | `increment_V1_foundations_v2.md` |
| 8 | Product shell and navigation | IMPLEMENTED and TESTED (V1); connection indicator slot and Change address not built (Phase B with FLIB modules) | same |
| 9 | Lab and experiment evidence experience (inside U-01 / T-01 boundaries) | PLANNED phase B (agent V2) after V1 tokens; pure logic by agent FLIB now | `increment_F_functional_logic.md` (pending) |
| 10 | Simulation explanatory page | PLANNED phase B; earlier I3 retired the interactive demo (archive `docs/design/remediation/legacy/SimDemo.jsx.orig`, sha256 `84a7ac45...86dc`); to be re-checked against the new brief | |
| 11 | Remaining pages and shared patterns | PLANNED phase B | |
| 12 | Purposeful motion and reduced-motion | V1 builds tokens and patterns; review with the four skills after phase B | |
| 13 | Responsive, accessibility, performance validation | PLANNED; performance baseline needs a quiet machine (cargo builds are running) | |
| 14 | Consolidated before/after review | PLANNED | |
| 15 | Remaining-defect register and completion report | PLANNED | |

## 4. Defect register U-01..U-14 (dispositions; nothing silently closed)
| ID | Summary | Disposition now |
|---|---|---|
| U-01 | cancelled/failed/interrupted run shows a completed verdict | UI half: agent FLIB (resultPresentation, wiring in phase B). Backend half: sandbox fix done (`docs/research/u01_backend/`), NOT integrated; separate independent review and owner approval pending. Not validated until the integrated backend and T-01 real-backend fixture exist |
| U-02 | no gateway-address interaction | FLIB (gatewayConfig) then UI wiring in phase B |
| U-03 | empty progress bar, "0 of 2 units" | FLIB (progress) then phase B |
| U-04..U-10 | duration estimate and presets, buried result, M6 jargon, "VALID" wording, stale "Reports mode: live", "not available" wording, mobile sticky overlap | phase B (V1 for sticky bar; V2 for Lab copy and structure) |
| U-11..U-14 | low items from Track 4 | to be dispositioned with evidence in the consolidated review |

## 5. Toolkit adoption evidence
### 5.1 The four Emil skills (IMPLEMENTED, TESTED)
- Source: `https://github.com/emilkowalski/skills` at commit `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28, "add remove-animations skill"), the same commit as the installed `apple-design`. MIT. Method: partial clone into `D:/Temp/toolkit_install/emil` (outside the repo), `git show <commit>:<path>` export, all five files read in full before copying (no scripts, hooks, network, installs or settings changes; only prose pointers to other Emil skills that are NOT installed).
- Installed (each folder also has `LICENSE` and `SOURCE.md` with blob and SHA-256 hashes): `animation-performance/SKILL.md`; `review-animations/SKILL.md` + `STANDARDS.md`; `remove-animations/SKILL.md`; `find-animation-opportunities/SKILL.md`. Verification: the git blob hash of each copied file equals the hash recomputed from the pinned object (5 of 5 OK); `LICENSE` sha256 equals the one recorded for `apple-design`.
- Findings: every skill starts with an "Initial Response" greeting directive (overridden by project rule 5); `review-animations` has `disable-model-invocation: true`, so it may need explicit invocation. Fresh-session discovery and invocation: see the line below.
- Fresh-session discovery and invocation (TESTED 2026-10-30 16:2x, fresh headless process from the repo directory using the extension's claude.exe, no tools for the listing): the model-visible skill list contained animation-performance, apple-design, find-animation-opportunities and remove-animations. review-animations is NOT in that list because its frontmatter sets disable-model-invocation: true; its explicit user invocation (/review-animations) is UNVERIFIED. Invocation: animation-performance was invoked through the Skill tool by the fresh process and its answer matched the file (composite-only tier; never transition: all). The other three were discovered but not invoked in this test. The running session's own skill list is fixed at start and does not show the new skills.

### 5.2 axe-core (IMPLEMENTED, TESTED)
- `ui/package.json`: `devDependencies["axe-core"] = "4.14.0"` (exact), script `check:axe`; installed with `npm install --save-dev --save-exact --ignore-scripts`; lockfile gained exactly one package entry (`dev: true`, MPL-2.0, integrity prefix `sha512-9WTZxEjs...` matches the registry value recorded in R3). Licence file stays in `node_modules/axe-core/LICENSE`; unmodified use as a dev tool; axe source is never copied into the repo.
- Tests: `ui/test/forbidden.test.mjs` and `ui/test/lab.guard.test.mjs` (both pinned the dev-dependency list; the proposal mentioned only the first) now expect `['@vitejs/plugin-react','axe-core','vite']` and the exact pin; a new test asserts no file under `ui/src/` references axe-core. `npm test` 299 of 299 pass; `check:forbidden`, `check:contrast` (196 of 196) and `check:a11y` (0 issues) pass.
- `ui/scripts/check-axe.mjs` (project-owned): injects axe over CDP into the tool's own throwaway Chrome, per route x theme x viewport; records violations and `incomplete` separately (`incomplete` is never a pass); `--gate` exits non-zero on violations or unrendered cells; has a readiness guard.
- Self-test (TESTED, not a baseline): Chrome 156.0.8078.79, Vite dev server, gateway NOT running (disconnected state), routes overview, lab_run_form, simulation, viewports 390 and 1280, light and dark = 12 cells: 0 violations, 12 `incomplete` (all `color-contrast`, 2 to 21 nodes per cell). **Lesson recorded:** the first run lacked the readiness guard and produced false "clean" 390 px cells (axe ran before the app rendered: 5 passes versus 21 to 26 after the guard). A clean axe result is meaningless unless the page rendered; the guard now flags `notRendered`.
- Limits: axe finds only part of WCAG issues; this run did not cover live data, run pages or the gateway-connected states; `color-contrast incomplete` needs the worst-case check (`check-contrast.mjs`) and manual review; no assistive technology involved.

## 6. Decisions taken (and why)
- Phased execution to respect file ownership: V1 (styles, shell, shared components) and FLIB (pure logic and tests in `lib/`, `hooks/`) in parallel; V2 (Lab, evidence, Simulation, charts) after V1 tokens exist, wiring FLIB's modules.
- Illustration caption and eyebrow exception adopted as provisional working rules (rules doc 11 and 12), pending the owner's explicit word.
- Performance baseline and remeasurement deferred until the machine is quiet; no performance claim is made from the current runs.

## 7. Inputs read for step 1
`CLAUDE.md`; `docs/design/phantomcache_design_direction_v2.md` (headings and section 16 in full); `.claude/skills/apple-design/SKILL.md` and its project rules; the four Emil skills (full text); `docs/research/design_toolkit/toolkit_proposal.md` (provided in full by the owner's selection); `ui/package.json`, `ui/scripts/cdp-lib.mjs`, `ui/test/forbidden.test.mjs`.

## 8. Known limitations and remaining work (kept current)
- No human visual, usability or assistive-technology review has happened; the earlier before/after review opened 24 of 210 image pairs.
- The earlier automated accessibility and browser results (Chrome 156 headless, one Windows host) are historical baselines, not proof for the new implementation.
- Backend U-01 integration, JWT telemetry authentication and any telemetry additions remain owner-reserved and are NOT part of this programme.
- Remaining: phase A results, phase B, motion review with the four skills, performance remeasure, responsive and accessibility validation, final report.

## 9. Commit tracking (phase-wise batches; push after committing)
| Phase / Workstream | Files | Commit hash | Tests | Review status |
|---|---:|---|---|---|
| Toolkit: apple-design skill, overlay and evidence | 5 | `8523cc1` (pushed) | n/a (documentation); blob hashes verified | lead read all files |
| Tooling: report-only docs-lint and config | 2 | `3e6e03e` (pushed) | docs-lint ran read-only, exit 0 | lead read; small commit predates the larger-batch strategy |
| Toolkit: four Emil animation skills | 13 | `caeedfd` (pushed) | blob hashes 5 of 5; fresh-process discovery 4 of 5, one invocation | lead read all five skill files in full |
| Git: line-ending protection for four pinned files | 1 | `df68eaa` (pushed) | git check-attr shows 4 unset, others unspecified | owner decision DEC-006 |
| PLANNED: documentation, design rules, governance and implementation records | pending | pending (batch plan v2, agent BATCH) | docs-lint on index copy | HELD FOR REVIEW |
| PLANNED: UI foundations v2 (V1) with its tests and contrast script | about 15 | pending | UI suite 483 of 483 (lead ran), contrast 284 of 284, a11y 0 issues | V1 REPORTED complete; lead review of diffs pending |
| PLANNED: UI functional logic (FLIB) with tests and fixtures | about 25 | pending | 159 tests (lead ran whole suite) | HELD FOR REVIEW |
| PLANNED: handbook (docs/learning) | about 80 | pending | build and QA | QA running |
| PLANNED: Lab and Simulation redesign (Phase B) | pending | pending | pending | not started |
| BLOCKED: Rust code (3 commits) | about 130 | pending | cargo check on index copy | DEC-022, DEC-005, DEC-006 hash procedure |

See the tracker section G for the same table kept current; commit hashes are filled in as batches land.
