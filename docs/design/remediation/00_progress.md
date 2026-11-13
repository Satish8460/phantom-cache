# UI/UX remediation: shared progress document

**Status:** Active (lead-owned; agents report in their own files and the lead copies status here). **Audience:** product owner, UI/UX agents. **Last reviewed:** 2026-10-30 12:50 (+05:30). **Authority:** owner authorization of 2026-10-30 (O8-1 APPROVED; O8-2 OPEN until independent review). Protected boundaries: frozen/sealed evidence, historical acceptance records, CLAUDE.md, README positioning, architecture and consistency contracts, security semantics, backend behaviour.

## Status classes used
IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, BLOCKED. Nothing is "approved": O8-2 stays open.

## 1. O8-1: skill adoption
| Item | Status | Evidence |
|---|---|---|
| Revisit repository, verify contents and licence | IMPLEMENTED and verified by the lead | `docs/design/apple_design_skill_adoption_evidence.md` section 1 (MIT, commit `a81dbca07f2217b7103e510c0f13f8d5b017afc9`, blob hashes match) |
| Pin and install as a project skill | IMPLEMENTED | `.claude/skills/apple-design/` (`SKILL.md`, `LICENSE`, `SOURCE.md`), byte-identical to the pinned commit |
| Discoverable by Claude Code | TESTED: yes in a fresh process; in the running session the first attempt (12:31) failed, then the skill list refreshed (about 12:55) and listed `apple-design` | same file, section 2 |
| Actually invoked | TESTED: Skill-tool invocation by a fresh process (session `6090c5b2...`) and by the lead in the running session (12:55, base directory `.claude/skills/apple-design`). Sub-agent invocation: UNVERIFIED until an agent report shows it; agents started earlier read the installed file and cite sections | same file, sections 2 and 3 |
| Other two repositories | NOT installed (owner instruction) | SOURCE.md |

## 2. Work items
| ID | Work | Owner (single writer) | Output file(s) | Status |
|---|---|---|---|---|
| B0 | Before-gallery (390/820/1280, light/dark) | agent B0 | `docs/design/remediation/before/` (210 PNG + manifest, ui/src hash bcbfa3b6...), `ui/scripts/browser-gallery.mjs` | DONE (lead opened 2 images; B0 opened 14); re-run for after: `node ui/scripts/browser-gallery.mjs --out docs/design/remediation/after` |
| RC | Root-cause analysis of unfulfilled UI/UX instructions | agent RC | `01_root_cause_analysis.md` | DONE (16 rows, top five causes; findings are RC's, partly unverified) |
| DR | Cross-project discrepancy register | agent DR | `docs/research/vision/discrepancy_register.md` | DONE (56 rows; lead verified DR-029) |
| AUDIT | Full UI/UX audit and remediation plan (uses the skill) | agent AUDIT | `02_ui_audit_and_plan.md` | DONE (52 findings, 8 blocking coherence; skill read, not invoked, by AUDIT) |
| SPEC | Shared design-system specification (gate 5.1) | lead | `docs/design/phantomcache_design_system_spec.md` | WRITTEN (Proposed) |
| I1 | Foundations: tokens, elevation, motion, states, badges, forms CSS, dark-theme dedupe, contrast | agent I1 | tokens.css, app.css, shared components, check-contrast, tests, `increment_I1_foundations.md` | **ACCEPTED as foundations only by the lead (Gate B, 13:50)**: IMPLEMENTED and TESTED (lead re-ran: 215/215 tests, contrast 196/196, a11y 0 issues, build ok; no new dependency; lead opened the specimen and the Lab preview); NOT visually approved: the real pages barely change until I2-I4. Decisions: `light-dark()` without fallback accepted; provenance mapping hand-off H1 approved and assigned to I2 |
| I2 | Phantom Lab hierarchy and primary workflow | agent I2 | lab.css, screens/lab/*, LabParts/LabCharts, lib/lab*.js + provenance.js, miniMarkdown, tests, `increment_I2_lab.md` | running (ports 3000/8085) |
| I3 | Legacy Simulation page (retire to honest SIMULATED explainer; archive SimDemo.jsx first) | agent I3 | SimDemo.jsx shim or delete after archive, Simulation.jsx, sim.css, forbidden scanner (zero exemptions), `increment_I3_simulation.md` | **delivered, lead review in progress (14:10)**. Lead verified: archive `docs/design/remediation/legacy/SimDemo.jsx.orig` sha256 `84a7ac45...c686`, 517 lines; `ui/src/SimDemo.jsx` deleted; no source refers to SimDemo; lead opened the 1280 dark preview. IMPLEMENTED (agent: scanner now has zero exemptions; sim tests 8; 223/223 at its last full pass). UNVERIFIED: full-suite state (agent saw 3-5 Lab test failures in I2's files while I2 was editing); screen-reader/forced-colors; human wording review. Lead visual notes: page is clean, honest and legible, but the right edge of the banner and panels sits flush against the viewport edge at 1280 (missing right gutter, to check in I4/I5), the nav entry still says "Simulation demo" (I4), and it is text-heavy with an uninspiring illustration (polish) |
| DR029 | DR-029 security investigation (read-only; scratch reproduction) | investigator | `docs/security/investigations/DR-029_...md` | running (separate track; Gate A pending) |
| I4 | Non-Lab pages, navigation, shell, titles | after I2/I3 | per plan | not started |
| I5 | Final polish, after-gallery, comparison | after I4 | per plan | not started |
| V1-V4 | Validation tracks: functional, accessibility, visual (human review of screenshots), usability | separate reviewers | `docs/design/remediation/03_validation_*.md` | not started |
| G | After-gallery and before/after comparison | agent B0 re-run | `docs/design/remediation/after/` | not started |

## 3. Rules for all writers
- One writer per file; file ownership is fixed in the plan before an increment starts.
- Read `.claude/skills/apple-design/SKILL.md` and `docs/design/apple_design_skill_project_rules.md` first; cite skill sections; fill the compliance table.
- No new dependencies, no backend changes, no changes to protected records. A UI need that requires a backend change is reported as a dependency.
- No agent declares the UI complete. The lead integrates, runs the regression tests, and the independent review decides O8-2.
- Every claim of improvement states IMPLEMENTED / TESTED / INDEPENDENTLY VERIFIED / UNVERIFIED / BLOCKED with evidence.

## 3b. Additional gates (owner instruction of 2026-10-30, section 5)
| Gate | Rule | Status |
|---|---|---|
| 5.1 Design-system foundation gate | Before any page-level work, the lead establishes ONE shared specification (typography tiers and usage; elevation and surface hierarchy; spacing and layout; motion and interaction feedback; form patterns; provenance and system-state presentation; light and dark; accessibility). All increments use it. Agents may not redefine shared tokens or component behaviour. | SPEC WRITTEN (Proposed): `docs/design/phantomcache_design_system_spec.md` (lead, 13:10, informed by the skill invocation and `02_ui_audit_and_plan.md`). I1 must not start until B0 has finished the before-gallery (UI files must stay unchanged until then) |
| 5.2 Increment acceptance gate | Each increment has: defect and expected outcome; affected routes and components; acceptance criteria; skill references; file ownership; before/after evidence; functional and accessibility regression results; remaining limitations. Template: `docs/design/remediation/increment_template.md`. | template written below; no increment started |
| 5.3 Comparable screenshots | Before and after use the same viewports, themes, routes, application states, browser conditions and data conditions; a screenshot comparison must not imply simulated or unavailable capability is real. | enforced by the shared `ui/scripts/browser-gallery.mjs` (agent B0) |
| 5.4 Final quality gate | O8-2 needs separate functional, accessibility, visual and usability verdicts; agent reviews are not human design approval; the lead reconciles findings, fixes critical defects and presents the real application for human review before requesting O8-2. | NOT STARTED |
| 5.5 No premature completion | Skill adoption, design documents, single increments or passing automated tests do not establish completion. The remediation stays ACTIVE until implementation, regression evidence, independent reviews and human design review are complete. | **ACTIVE** |

## 3c. Acceptance checklists (owner coordination message, 14:20)
**I3 status: IMPLEMENTED and TESTED (per its report); acceptance is CONDITIONAL.** Before final acceptance the lead must record each item:
| # | Item | State |
|---|---|---|
| 1 | Independent rerun of the full UI test suite after I2 finishes | OPEN |
| 2 | Forbidden-pattern scanner covers the complete UI with zero exemptions (lead reads `ui/scripts/forbidden-patterns.mjs`, `forbidden.test.mjs`) | OPEN |
| 3 | Both legacy routes `#/simulation` and `?view=sim-demo` still resolve (lead check in a real browser or route tests) | OPEN |
| 4 | Remaining I3 previews inspected by the lead, including mobile (390) and light mode (lead has opened only 1280 dark so far) | OPEN |
| 5 | Right-edge gutter issue corrected or assigned to I4/I5 | ASSIGNED to I4/I5 (observed on the 1280 dark preview; needs a check on other pages) |
| 6 | I3 did NOT read the full design-system spec (only the provenance section by grep); its spec-compliance citations rest on a partial read. **The spec-compliance item is NOT marked complete.** | RECORDED |
The archived original (`legacy/SimDemo.jsx.orig`, sha256 `84a7ac45...c686`) must not be modified.

**I2 completion requirements:** exact file inventory and line counts; regression results; forbidden/contrast/a11y checks; production build; before/after screenshots matching the B0 states; screenshots personally opened by the lead; full-page height comparison of the Scenario B evidence page; honest critique; backend dependencies and hand-offs; skill-use evidence and compliance table. No fabricated elapsed time, phase progress, arm descriptions or provenance.

**Integration gate after I2:** full test suite, `npm run check` and Vite build; inspect the combined Lab and Simulation experience; route navigation, focus management, mobile overflow, light/dark, provenance consistency; resolve the I2/I3 hand-offs before I4; one writer per file.

**I4 may begin only after I2 is accepted.** Known hand-offs for I4: rename the Simulation nav label ("Simulation demo" to "Simulation") and reconsider its "Simulated" nav group; remove the obsolete `simulation` filter in `a11y.test.mjs`; remove unused `.sim-frame` in `app.css` if confirmed safe (shared file: coordinate with the lead); update `Overview.jsx:49` wording ("Items marked Inferred") to the approved provenance vocabulary; review the right-edge gutter issue; inspect `App.jsx` and the shell integration; `.app-nav .app-nav__toggle` hide rule must be kept when the shell moves to `shell.css`; wrap `Charts.jsx` in `ChartFrame`; per-route titles/footers.

**Security track:** DR-029 is separate from UI. The Gate A review is an AI second look and not human security assurance. No source fix is authorised. Narrowed severity (Low as exploited today, Medium as a latent hazard) and the unresolved human-review requirements are preserved.

**Final acceptance (O8-2):** not closed by agent completion or automated checks. Required: consolidated before/after review, human design assessment, accessibility evidence, and an explicit remaining-defect register. Ledger distinguishes IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, BLOCKED and ACCEPTED.

## 4. Log
- 2026-10-30 12:31 skill files installed; 12:35 fresh-process invocation succeeded; 12:45 documentation and agents B0, RC, DR, AUDIT started.
