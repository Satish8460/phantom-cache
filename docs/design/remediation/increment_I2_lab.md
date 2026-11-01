# Increment I2: Phantom Lab hierarchy and primary workflow (record)

**Status:** IMPLEMENTED and TESTED (regression guards, real-browser measurements by the writer); visual quality UNVERIFIED (no independent review; the critique in section 6 is the writer's own). **Date:** 2026-10-30. **Writer:** I2 agent (single writer of the files in section 5). Authority: `docs/design/phantomcache_design_system_spec.md`, `.claude/skills/apple-design/SKILL.md`, `docs/design/apple_design_skill_project_rules.md`, lead decisions (a) to (d).

**How the skill was used (truthfully):** the Skill tool WAS invoked once with a concrete question (which sections cover focus, error states and hierarchy in a dense form). It worked in this session: it returned the full skill text. The relevant sections were S16 (inline validation, grouping and mapping, common path first, feedback kinds, wayfinding), S1 (instant feedback), S14 (reduced motion), S15 (type, measure, leading), S12 only for the dimming scrim of the cancel dialog (override: no blur). No section of the skill was applied to anything it does not cover (no springs, no gestures).

## 1. Record fields

| Field | Content |
|---|---|
| Increment ID and name | I2 Phantom Lab hierarchy and primary workflow |
| Defects (audit IDs) | D-01, D-02, D-03, D-04, D-05, D-06, D-07, D-08, D-10, D-11, D-12, B-02, B-04 (Lab part), C-01 to C-07 (run form and run page), A-10/A-11 (Lab wording), A-12 (Lab dashed leftovers), A-05 (elevation use in the Lab) |
| Expected outcome | The Lab opens on what it is and its primary action; connection and honesty information is one compact strip; the run form is short, grouped and validates by touch; the run page shows state, tier, elapsed, progress, cancel and the latest events at the top; finished runs lead with the result; evidence text is readable and the Scenario B page is scannable; provenance words follow the owner vocabulary |
| Routes and states | `#/lab` (About), `#/lab/run`, `#/lab/runs`, `#/lab/runs/<id>` (running, cancelling dialog, completed, cancelled, failed, interrupted, stream problems), `#/lab/evidence`, `#/lab/evidence/<id>` (M6, Scenario B); Lab off, unreachable, token required |
| Files and single owner | all files in section 5 are owned by I2; nothing outside OWNS was edited |
| Spec sections used | 1 (type), 2 (page head, banner limit), 3, 4 (L1 cards, L2 sticky run header and action bar, L3 dialog), 5 (borders), 6 (motion), 7 (controls, charts), 8 (status strip, primary workflow), 9 (forms), 10 (state grammar), 11 (provenance), 12 |
| Skill sections and overrides | section 2 below |
| Acceptance criteria | section 3 |
| Tests added or updated | section 4 |
| Functional regression result | `node --test test` in `ui`: 225 tests, 225 pass (the entry file `test/index.js` does not import the two new test files, see hand-offs). New files run directly: `node --test test/lab.hierarchy.test.mjs test/lab.markdown.test.mjs`: 29 tests, 29 pass. `npm run check`: forbidden PASSED, contrast 196/196, a11y lint 74 combinations 0 issues, tests 225/225. `vite build` OK. 2026-10-30 |
| Accessibility regression result | static only: a11y lint 0 issues on 74 states (including all new Lab markup in the tests), contrast 196/196 (tokens unchanged). Real browser (Chrome, headless): Tab order, contents link focus, failed-submit focus on the summary, no horizontal page scroll at 1280 and 390 (scrollWidth equals clientWidth in `measurements.json`). NOT done: screen reader pass, forced-colors and reduced-motion captures, 200% text zoom, the dialog Escape and focus-return path in a browser (code and static tests only) |
| Before screenshots | `docs/design/remediation/before/` |
| After (preview) screenshots | `docs/design/remediation/i2_preview/` (210 PNG + `manifest.json`, same names, 390/820/1280, light and dark, produced by `i2_gallery.mjs`, a copy of `ui/scripts/browser-gallery.mjs` with the I2 selectors). Measurements: `i2_preview/measurements.json` from `i2_measure.mjs` |
| Provenance check | every Lab number still sits next to a badge or in a row with a provenance column (existing and new tests); MEASURED shows as Real with a qualifier on every page rendered by the tests (`lab.hierarchy` "provenance wording across pages"); Retained is a separate tag; no Lab tier maps to REPLAYED or PREDICTED; elapsed is DERIVED "from timestamps" or UNAVAILABLE with the reason; the Raw notice is the payload string unchanged |
| Remaining limitations | section 7 |
| Backend dependencies | section 8 |
| Status per item | section 9 |

## 2. Compliance table

| Change | Skill sections | Project override | How it was checked |
|---|---|---|---|
| Status strip replaces Connection panel and two banners; opens only when attention is needed | S16 Simplicity (common path first, detail one level deeper), Purpose, wayfinding | honesty content stays visible as chips (project rule 4); no glass | `lab.hierarchy` (chips, open/closed logic, no banner above tabs); browser: strip 50 px high at 1280 (2 chip rows), 3 rows at 390 in the screenshots |
| Page head with primary action, tabs in workflow order, "Overview" renamed About | S16 Agency, specific labels, familiarity | none | `lab.render` nav test, `lab.hierarchy` first-screen order; browser: button at y 81 to 113 at 1280 and y 214 to 258 at 390 |
| Run form groups, presets, progressive disclosure, collapsible catalogues with counts and Select all/Clear | S16 Simplicity, grouping and mapping, defaults shown | no new dependency | `lab.render`, `lab.hierarchy` (preset validity, 3 closed groups); browser: matrix form 1723 px (1280) and 2271 px (390) with groups closed, 2215 and 4263 with all open (before 3442 and 5174) |
| Touched-state validation, error summary with links, Start enabled with a status | S16 validate inline, feedback kinds | none | pure-logic tests (`visibleErrors`, `errorSummary`); browser: 0 visible errors pristine; after a failed submit 3 inline errors, focus on the summary |
| Sticky run header (L2), cancel as native modal dialog (L3), small live-status chip, event tail | S1, S3 (never lock input), S16 Agency (confirm only the destructive action), feedback kinds | S12 dim to focus via `::backdrop`, no blur | `lab.render`, `lab.fixes` F-07, `lab.hierarchy`; browser: running page captured, dialog captured at 6 viewport/theme combinations |
| Elapsed time DERIVED or UNAVAILABLE; no invented phases | S16 Responsibility | lead decision (d), project rule 4 | `elapsedView` tests, render tests |
| Result at a glance first, artifacts with sizes and hashes, closed folds with counts | S16 Simplicity, hierarchy | binding wording never inside a closed fold | `lab.hierarchy`; browser: completed Scenario B run captured |
| miniMarkdown, RichNotice (Rendered / Raw / Copy) | S15 (measure 75 ch, leading), S16 Craft | links disabled, no HTML | `lab.markdown` (11 tests); browser: no raw markers, 1 heading, no inner scroll box |
| ChartFrame, Chart/Table toggle, full-width axis caption, direction labels, 16 px marks | S16 grouping and mapping | table always in DOM | `lab.fixes` F-11 and toggle tests; browser: caption width 1017 px equals plot width (before: about 100 px) |
| Provenance wording (Real + qualifier, Retained tag) | S16 Familiarity (consistency) | owner vocabulary, lead decision (a) | `lab.view`, `lab.hierarchy` |
| lab.css rewrite: tokens only, shared classes, no dashed/dotted, no nested cards | S15, S16 | spec sections 4 and 5 | `lab.hierarchy` stylesheet test |

## 3. Acceptance criteria and result

| # | Criterion | Result | Status |
|---|---|---|---|
| 1 | First screen shows what the Lab is, the status strip and "Start a run" without scrolling past connection text, 1280x800 and 390x844 | 1280: h1 y 81, button y 81 to 113, strip y 186 to 236, tabs y 260, first content heading y 317. 390: button y 214 to 258, strip y 353 to 448, tabs y 472, first heading y 541 (the shell header uses about 190 px of that, I4) | TESTED (browser measurement, writer) |
| 2 | Run form groups, default Scenario B, presets, touched validation, error summary, sticky bar, disabled primary states why, progressive disclosure, matrix not 5,000 px at 390 | 1280: first legend y 334, first field y 687 to 723 above the bar (y 742), Start button visible in the bar; pristine: 0 errors; failed submit: summary focused, 3 inline errors; matrix at 390: 2271 px collapsed, 4263 px with all 38 choices open (before 5174 empty). Disabled-with-reason covered by render tests only | TESTED |
| 3 | Run page: sticky header, progress from events, event tail, live status small, glance first, artifacts | implemented and captured; elapsed and header verified in screenshots. Phases are NOT a stepper (section 8) | TESTED for markup, visual UNVERIFIED |
| 4 | Evidence: rendered notice, contents list, shorter Scenario B page, ChartFrame, verbatim wording | Scenario B 1280: 6266 to 5310 px (-15%), 390: 11106 to 9046 px (-19%); M6 1280: 5033 to 3880 (-23%); the plan's 30% target was NOT met for Scenario B (informational, not pass/fail). Notice text equals the payload with markers removed (test) | TESTED; the 30% target missed |
| 5 | lab.css hygiene | tests: no hex, no dashed/dotted/double, no duplicated input/danger/disabled rules, shadows only from tokens | TESTED |
| 6 | Provenance wording | tests over seven rendered pages | TESTED |
| 7 | Negative results same weight | one VerdictChip component, equal-shape test, no outcome-specific CSS | TESTED |
| 8 | Accessibility: headings, focus, Escape, targets, live region, reduced motion | heading order test; native dialog Escape wired (static test); 24 px controls (`.choice input`); live region `data-run-status-live`; reduced motion inherited from app.css (no new motion). Not browser-verified: Escape and focus return | partly TESTED, partly UNVERIFIED |
| 9 | State grammar for each async region | loading, empty, error, unavailable, stale (banner), simulated used through the shared components | TESTED (render tests) |

## 4. Tests

Added: `ui/test/lab.markdown.test.mjs` (11), `ui/test/lab.hierarchy.test.mjs` (18). Updated with justification (comments in the files): `lab.render` (strip, tab order, form, run header, cancel dialog, charts, wording), `lab.fixes` (F-01, F-03, F-07, F-11, toggle), `lab.round2` (N-1b, N-3, N-4), `lab.view` (tier wording). Every change follows from a deliberate design change (status strip, Chart/Table toggle, native dialog, touched validation, provenance vocabulary); no assertion was weakened to pass. All tests are regression guards: they say nothing about visual quality.

## 5. Files (I2 only)

`ui/src/styles/lab.css`; `ui/src/screens/lab/{ConnectionPanel (now the status strip; name kept because the render harness imports it), EvidenceViews, Gate, LabScreen, RunForm, RunsViews, ScenarioBResult, StoryView}.jsx`; `ui/src/components/{LabParts,LabCharts}.jsx`; `ui/src/lib/{labForms,labView,labProvenance,provenance}.js`; new `ui/src/lib/miniMarkdown.js`; `docs/design/phantom_lab_ui.md` (line 20 amended with a dated note, new section 11); tests above; `docs/design/remediation/i2_preview/*` (screenshots, `i2_gallery.mjs`, `i2_measure.mjs`, `measurements.json`); this record. `ui/dist` was rebuilt by `vite build`.

## 6. Critique of the result (writer's own; not approval)

Improved (seen in the screenshots): the Lab opens with a title, a primary button and a one-row strip instead of about 450 px of connection text; the honesty chips are still visible; the run form is a short grouped form whose catalogue groups are collapsed; the run page leads with a header that carries state, tier, elapsed, progress and Cancel; the cancel confirmation is a real dialog; the interpretation notice reads as text; chart captions span the chart; badges are lighter and say Real with their basis.

Not improved or still weak:
- The Scenario B evidence page is still about 5,300 px at 1280. The three charts, the arms table and the binding lists are the bulk, and body-size lists are taller than the old 13 px ones. The 30% height target was missed.
- At 390 the shell header (I4) takes about 190 px, so the Lab content starts low; the run header takes about 150 px.
- The sticky action bar covers the bottom of the form at the initial scroll position and is 94 px high at 390.
- The "Result at a glance" card on a completed run opens with the long "Label VALID means only ..." paragraph before the verdict chips; the verdicts should lead and the paragraph follow (not changed here to keep the binding text order of the payload).
- With the gateway stopped the same "Could not reach the gateway" text appears in the strip Details and in the page body.
- Tab order puts the shell navigation (11 stops at 1280) before the Lab primary action; that is the shell's DOM order.
- The matrix catalogues show raw ids (S1, W-Z, ...) because the API gives no names or descriptions.
- Dense tables (arms, paired differences) are unchanged in density; numbers are right-aligned now.
- Nothing here was tested with people. Whether the landing page answers "what is this and how do I start" in ten seconds is a human-review item.

## 7. Known limitations and defects

Escape and focus return of the cancel dialog, forced colors, reduced motion, 200% zoom and a screen-reader pass were not run in a browser. The gallery covered the anonymous debug gateway only (no JWT, no failed or interrupted run captured). `a11y-lint.mjs` does not know the roles `dialog`, `alertdialog` or `progressbar`; the markup avoids them (native `<dialog>`, decorative indeterminate bar with the text beside it). A leftover throwaway Chrome profile directory from an aborted trial run may exist under `D:/Temp/browser_profile_gal_*`; it was not removed because a glob delete was refused by the session's permission layer.

## 8. Backend dependencies (listed, not worked around)

1. Per-phase progress: progress events carry only the current arm, repetition and phase name and `done`/`total`. A phase stepper with elapsed or remaining time per phase needs the phase list and per-phase timing in the events or the run record.
2. Names and one-line descriptions for the 12 scenarios, 7 families and 19 arms of the matrix in `GET /capabilities` (today ids only).
3. A `registered protocol` block in capabilities if a "Registered protocol" preset is wanted; today the presets are UI-defined from the gateway limits and say so.
4. Server-side elapsed or duration for terminal runs (today derived from `started_utc` and `ended_utc`; labelled DERIVED).
5. Build profile in the run list rows (today only in the run record).

## 9. Status per item

IMPLEMENTED and TESTED: status strip, page head and tabs, run form and validation logic, presets, run header and cancel dialog markup, elapsed logic, miniMarkdown and RichNotice, ChartFrame charts, provenance wording, stylesheet hygiene, documentation update. TESTED in a real browser by the writer only: first-screen positions, form positions and counts, failed-submit focus, contents link, notice rendering, caption widths, page heights. UNVERIFIED: visual quality, dialog Escape and focus return, forced colors, reduced motion, zoom, screen reader, human tasks. INDEPENDENTLY VERIFIED: none. BLOCKED: phase stepper (backend dependency 1).

## 10. I2b follow-up (2026-10-30, after lead review of I2)

Same ownership; browser work on ports 3100 (Vite) and 8185 (debug gateway, anonymous), own processes only (PIDs in `i2_preview/i2b_verification.json` and `manifest.json`), ports free afterwards. No backend, no dependency, no git, no non-Lab file.

### What changed
1. **Result at a glance** leads with the verdict chips (same component and weight for every outcome) and the mechanism line. The long "Label VALID means only ..." statement is the payload text, unchanged, inside a closed disclosure "What the run class label means" directly below the verdicts; the "Why the label is not VALID" reasons stay visible. (`ScenarioBResult.jsx`; test: order, closed, text equals the payload.)
2. **Gateway stopped / API off**: the status strip shows the chip plus one pointer line ("The gateway did not answer the capabilities request. The section below explains ..."); the title and remedy appear once, in the body's error state. (`ConnectionPanel.jsx`.)
3. **Sticky action bar**: `:root:has(.lab-form) { scroll-padding-bottom: 6rem }` keeps focused fields clear of the bar; the bar is one line (button plus a short status, at most two lines clamped), 62 px high at 390 and 820 (was 94 px); status text shortened ("Ready to start.", "N things to fix.").
4. **Matrix choices**: ids are in the mono tier; each open group says "Ids exactly as the gateway lists them; it provides no names or descriptions for them." No description was invented.
5. **Scenario B page length**: Arms side by side, Mechanism check and "What this does and does not show" (limitations and interpretation notes, with counts in the visible heading) are now closed folds; the binding notice and the verdicts stay open; the contents list opens a fold and focuses it (browser-checked). Heights, same method (`scrollHeight`, light theme):

| Page | Before (original) | After I2 | After I2b |
|---|---|---|---|
| Scenario B evidence, 1280 | 6266 | 5310 | **4159** (-34% vs original) |
| Scenario B evidence, 390 | 11106 | 9046 | **7194** (-35%) |
| Completed Scenario B run, 1280 (gallery full-page) | 6746 | 5375 | 2989 (-56%) |

The 30% target is now met for the evidence page (informational, as before). The completed-run figure also reflects I4 shell changes merged meanwhile.

### Browser verification (`i2b_verify.mjs`, `i2b_verification.json`, PNGs in `i2_preview/i2b/`)
| Item | Result |
|---|---|
| Dialog: Enter on Cancel opens it, focus on "Keep running" | PASS |
| Tab inside the dialog | Tab 1 reaches "Yes, cancel the run"; Tab 2 leaves the page document to the browser UI (activeElement is BODY, nothing of the page behind is reachable); Tab 3 returns to "Keep running". That is the browser's normal modal behaviour. The script's own strict flag read FAIL for that reason only |
| Escape closes the dialog, focus lands on "Cancel run" | PASS |
| Confirm path: Yes, then focus lands on the run heading (the Cancel button is gone) | PASS |
| Reduced-motion emulation: dialog animation pc-appear becomes pc-fade, indeterminate bar animation none, disclosure pc-fade | PASS (computed styles; screenshots `lab_home__reduced_motion__1280.png`, `run_detail__reduced_motion__1280.png`) |
| Forced-colors emulation: matches; strip border solid system colour, current tab and checked kind outlined, bar bordered | PASS on computed checks; screenshots `lab_home`, `run_detail`, `run_form` `__forced_colors__1280.png` opened (lab_home) and read as legible; run_detail and run_form screenshots were produced but NOT opened by the writer |
| 200% zoom equivalent (640x400 CSS px): lab_home and run form | PASS: no horizontal page scroll (625 of 625), title, primary button and strip visible on lab_home; run form shows no clipped controls in the opened screenshot |
| Action bar at 390 and 820 with the last advanced field focused | PASS: field bottom 396 above bar top 782 (390), 699 above 1053 (820); bar 62 px; scroll padding 96 px |
| Contents link to the closed limitations fold | PASS: opens it, URL hash unchanged |

Not run: a screen reader, a real 200% browser zoom (a viewport equivalent was used), JWT and failed or interrupted runs, Firefox and Safari.

### Previews
`i2_preview/` was regenerated with the same names (`i2_gallery.mjs --ui-port 3100 --gw-port 8185`): 194 files, 6 states not reached. Stale files from the earlier I2 gallery remain for those names and were NOT refreshed: `run_detail_sb_running` at 1280 light, 390 dark, 820 dark, 1280 dark (the 95 s run had already ended when the sweep reached them) and `run_form_t1_invalid` at 390 light and 820 light (a script selector error). The 390 `__full` images in this regeneration are 844 px tall (the I4 shell no longer lets the page scroll in the capture), so 390 full-page heights are taken from `i2b_verification.json`, not from the PNGs. Before images are untouched.

### Status
IMPLEMENTED and TESTED: items 1 to 5 (render and unit tests; 3 new I2b tests in `lab.hierarchy`). TESTED in a real browser by the writer: dialog keyboard path, reduced motion (computed), forced colors (computed plus one screenshot opened), zoom reflow, action bar clearance, contents link, heights. UNVERIFIED: visual quality, screen reader, real browser zoom, other browsers. INDEPENDENTLY VERIFIED: none.

### Remaining defects and hand-offs
- `lab.fixes` F-01, F-02, F-03 and F-04/F-05 now FAIL (4 of 258): they assert exact shell CSS text in `app.css` (`.app`, `.app-nav`, `.app-header`) that I4 moved or rewrote in `shell.css` and `pages.css`. Not caused by I2b; those assertions test shell rules owned by I4 (the lead or I4 should re-point them; I did not change them).
- Four running-state and two invalid-form previews are stale (above).
- With the gateway down, the strip still carries a pointer line, a token field (if the error is auth-related) and the full intelligence paragraph in Details; only the error title and remedy are no longer duplicated.
- `.choice input` size rule still lives in `lab.css` (hand-off to app.css owner).
