# Increment I3: Legacy simulation consolidation

**Status:** Complete pending lead review. **Date:** 2026-10-30. **Writer:** I3 agent (only writer of the files below).

| Field | Content |
|---|---|
| Increment ID and name | I3, Legacy simulation consolidation (retire the SimDemo pseudo-dashboard, replace with a SIMULATED explainer) |
| Explicit defects | E-01 to E-13 of `02_ui_audit_and_plan.md` section 3.5 (all removed with the file); root cause "legacy exemption" in `01_root_cause_analysis.md` (scanner exemption removed) |
| Expected outcome | `#/simulation` and `?view=sim-demo` show a plain explainer: nothing is measured, what a T1 simulated run is and is not, Simulated versus Real (loopback) runs, what the page does not do, one static labelled illustration, links to Phantom Lab (`#/lab/run`, `#/lab/evidence`, `#/lab/runs`). No metrics, no counterfactual, no economics, no status, no timeline, no PIE panel, no modal. |
| Affected routes and states | `simulation` (route id), `?view=sim-demo`. Static page: no loading or data states (it reads no telemetry). |
| Files and owners (I3 owns all) | `ui/src/screens/Simulation.jsx` (102 lines, rewritten), `ui/src/styles/sim.css` (23, new), `ui/src/SimDemo.jsx` (deleted, archived), `ui/scripts/forbidden-patterns.mjs` and `ui/scripts/check-forbidden.mjs` (exemption and importer rule removed), `ui/test/forbidden.test.mjs` (edited), `ui/test/sim.test.mjs` (70, new), `ui/test/tokens.test.mjs` (title text only), `ui/test/index.js` (one import line added; shared entry, see hand-offs), `docs/design/remediation/legacy/SimDemo.jsx.orig`, `i3_preview/` |
| Design-system spec sections | Section 11 (provenance vocabulary, badge anatomy), section 10 (states, simulated banner), L1 panels without nesting (section 4), type tiers display/headline/body/caption |
| apple-design sections | S15 (tiers, size-specific tracking via tokens, rem spacing), S16 Purpose (retire what does not earn its place), Responsibility (no fabricated data, labels never played down), Simplicity (one banner, common path first), Familiarity (same Panel, badge, banner as the rest), S14 (no motion at all, so reduced motion is trivially met). Overrides: S12 materials not used; no springs. The Skill tool worked: asked a concrete question, it loaded the full SKILL.md text. |
| Acceptance criteria | See compliance table below |
| Tests added or updated | `sim.test.mjs` (8 tests): routes resolve; one h1, SIMULATED badge, banner before content, "not measured from a gateway"; no currency symbol, CONNECTED, COMMITTED, `SEED:`, SYSTEM_BOOT, canary, counterfactual, and no digits in visible text (apart from "(T1)"); states what it does not do and links to Lab; no inline style attributes, no nested panel; `sim.css` has no hex/rgb, no literal font size, no double/dashed/dotted; no fetch/EventSource/useState/dialog in the page; a11y lint clean. `forbidden.test.mjs`: the "SimDemo is mounted only through the Simulation screen" test is replaced by "no exempt files, SimDemo.jsx absent, no source refers to SimDemo"; the two `SimDemo.jsx` skips (colour and network scans) removed, so those scans cover every file. `tokens.test.mjs`: only the test title lost the "SimDemo not scanned" remark (no code exception existed). **Tests are regression guards, not evidence of visual quality or browser accessibility.** |
| Functional regression result | In `ui`: `npm run check` earlier in this increment: forbidden scan 0 violations with zero exemptions, contrast 196/196, a11y lint 74 combinations 0 issues, `node --test test` 223/223. `vite build` passes (to `D:\Temp\i3dist`, not the repo `dist`). A later re-run after I2's concurrent edits showed 3 to 5 failures, all in `lab.render.test.mjs` / `lab.view.test.mjs` (Lab story, M6 detail, Scenario B notice, charts, tiers): I2's files, not touched by I3. My files (sim, forbidden, tokens, render, a11y tests) pass (52/52 in that run). Whether the Lab failures settle is UNVERIFIED. |
| Accessibility regression | Browser pass (headless Chrome, no gateway): all text at least 12 px (computed minimum 12), no horizontal overflow at 390/820/1280 in light and dark, zero `style` attributes in `main`, Tab reaches the three Lab links after the nav with a visible 2 px outline, link targets 32 px high, a link click navigates to `#/lab/run`. No screen-reader pass, no forced-colors or reduced-motion run (the page has no motion). |
| Before screenshots | `docs/design/remediation/before/simulation__{390,820,1280}__{light,dark}.png` |
| After screenshots | `docs/design/remediation/i3_preview/simulation__{390,820,1280}__{light,dark}.png` (full page, same names), `i3_checks.json` (measurements) |
| Provenance check | Every simulated statement carries the SIMULATED badge (3 badges plus the banner); the one REAL badge describes real-gateway runs and carries the "loopback" qualifier. The page shows no value. The illustration is badged "Simulated illustration" and has no numbers. |
| Remaining limitations | (1) Shell nav label still reads "Simulation demo", group "Simulated" (I4). (2) The `.sim-frame` rule in `app.css` is now unused. (3) The wording "drives the real cache and executor code" is taken from `ui/src/lib/labProvenance.js`, not re-verified against the Rust code. (4) The coalescing description (leader, waiters share one fetch) is a general account of single-flight, not verified against `src/core/coalesce.rs`. (5) The preview ran without a gateway, so the shell header shows "Disconnected": true, not staged. (6) `lab.guard.test.mjs` line 11 still filters `SimDemo.jsx` (I2's file; harmless). |
| Dependencies on backend or owner approval | Deleting `SimDemo.jsx` is covered by the lead decision; archived and hash-verified first. No backend dependency. |

## Archive
`docs/design/remediation/legacy/SimDemo.jsx.orig`, SHA-256 `84a7ac45d337a3bbf3580dfefcc800876f9706324f9e6fdf6f082ccf18dfc686` (identical to the original `ui/src/SimDemo.jsx` at copy time, compared with `sha256sum`; re-verified at the end). Only `screens/Simulation.jsx` imported it (grep over `ui/src`, `ui/scripts`, `ui/test`), so no shim was needed.

## Compliance table
| Criterion | Result | Status |
|---|---|---|
| No inline colour or font-size styles | 0 `style` attributes in DOM and source | TESTED, INDEPENDENTLY VERIFIED (browser) |
| No text below 12 px | computed minimum 12 px, 6 viewports x 2 routes | TESTED (script measurement, not by eye) |
| No hard-coded hex | scan covers all files; `sim.css` test | TESTED |
| No double borders; dashed/dotted only on badges | `sim.css` test; badges are I1 components | TESTED |
| display/headline/body tiers, L1 panels, no nesting | headline lead, Panel components; nesting test | TESTED, reviewed in screenshots |
| I1 States and badge components | SimulationBanner, Panel, ProvenanceBadge | IMPLEMENTED |
| 390/820/1280, light and dark, no horizontal scroll | scrollWidth equals clientWidth in all captures | INDEPENDENTLY VERIFIED (own browser run) |
| Keyboard reachable, no modal | Tab sequence recorded; no dialog | TESTED (browser) |
| Honest wording | reviewed; no numbers, no result claims | IMPLEMENTED; human review UNVERIFIED |
| Routes preserved | parseRoute test and browser load of both routes | TESTED |
| Scanner has zero exemptions | `EXEMPT_FILES` removed; test asserts absence | TESTED |
| Screen-reader behaviour, forced colors | not run | UNVERIFIED |

## Critique of the previews (opened 390/light, 820/light, 1280/dark)
- Works: one banner, clear order (what, what not, where to go); SIMULATED badges legible in both themes; no overflow at 390; stacked steps read well on a phone.
- Weak: text-heavy, five panels for little content; comparison panels have uneven content heights; the illustration is plain (three dots to one square, meaning carried by the numbered list) and large at wide widths; the headline lead partly repeats the banner; list text wraps at 70 ch inside a wider panel, leaving a ragged right edge at 1280. The REAL loopback badge sits on a description, not a result, which is correct but could confuse at a glance. Polish items, not honesty issues; left as is.

## Hand-offs
- I4 (`App.jsx`, Shell, `routes.js`): rename nav entry "Simulation demo" to "Simulation" and reconsider its group; drop the `simulation` filter in `a11y.test.mjs` (the page is lint-clean; `sim.test.mjs` lints it). `App.jsx` imports only `./screens/Simulation.jsx`; no `SimDemo` import exists.
- I1/I4: remove the unused `.sim-frame` rule in `app.css`.
- I2: `lab.guard.test.mjs` line 11 can drop its `SimDemo.jsx` filter; the Lab test failures seen at the end are in your files.
- Lead: `ui/test/index.js` (shared entry) gained one line importing `sim.test.mjs`; without it `node --test test` does not run the new tests.
