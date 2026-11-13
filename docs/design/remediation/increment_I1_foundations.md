# Increment I1: Foundations (record)

**Status:** IMPLEMENTED and TESTED (static and rendered-markup guards); visual quality UNVERIFIED (not judged by an independent reviewer). **Date:** 2026-10-30. **Writer:** I1 agent (single writer of the files in section 5). Authority: `docs/design/phantomcache_design_system_spec.md`, `.claude/skills/apple-design/SKILL.md`, `docs/design/apple_design_skill_project_rules.md`.

**How the skill was used (truthfully):** the Skill tool WAS invoked once with a concrete question ("what tracking and leading should small caption and metric tiers use, and how should press feedback and reduced motion work?"). It worked in this session (it returned the full skill text; the earlier audit saw `Unknown skill`). I also read the installed `.claude/skills/apple-design/SKILL.md` in full through that result. Skill sections are cited S1 to S17.

## 1. Record fields

| Field | Content |
|---|---|
| Increment ID and name | I1 Foundations |
| Explicit defect(s) (audit IDs) | Audit section 4.2 items 1 to 10 (11, the shell/pages CSS split, is NOT done: `shell.css` is reserved for I4). Root causes addressed: no type hierarchy tiers, unused elevation and shadow tokens, duplicated dark palette, dashed/double/dotted borders outside badges (A-12), inconsistent state grammar, heavy provenance badge anatomy, no press feedback, no shared form-group or chart-frame primitives. |
| Expected outcome | A shared vocabulary exists and is applied to base elements: type ladder on `h1`..`h4` and tier classes; L1 panel shadow and L2/L3 classes; motion tokens and press feedback; one state grammar; simplified badges with REAL / DERIVED wording; form-group, tab, segmented, dialog, menu classes; `ChartFrame`. Later increments consume them. Pages are NOT yet reorganised (see section 8). |
| Affected routes and states | All routes, indirectly (base elements, panel, metric, badge, state block, buttons changed). No route markup was edited by I1 except the shared components' DOM (badge gained `prov__word`, optional `prov__q`; metric stale tag; `StateBlock` gained kinds and `detail`). |
| Files and owners | Section 5. Single owner: I1 agent. |
| Spec sections used | 1, 2 (radii, spacing tokens), 3, 4, 5, 6, 7 (buttons, inputs, tabs, tables, charts, dialogs), 9 (forms), 10, 11, 12 (focus, forced colours, targets unchanged). Not used: 8 (shell patterns; I4). |
| Skill sections and overrides | Compliance table, section 3. |
| Acceptance criteria | (a) `node --test test` all pass; `npm run check` exit 0; `vite build` ok with no new dependency; (b) contrast script covers all listed pairs in both themes and passes; (c) `tokens.test.mjs` rules hold; (d) computed-style probes in headless Chrome: h1 28 px, h2 20 px, body 15 px, caption tracking 0.12 px, badge border styles solid/dashed/dotted as specified, `.btn:active` transform scale(0.97), none under reduced motion. Human-review items: section 7. |
| Tests added or updated | New `ui/test/tokens.test.mjs` (15 tests: tiers, aliases, motion tokens, single-source theme, elevation use, press feedback, reduced motion, dashed/dotted scope, no `backdrop-filter`, min font size, unused tokens, forced colours, `ChartFrame`, state grammar, `Panel`). `ui/test/provenance.test.mjs`: 3 tests added (REAL/DERIVED display, label override and `data-tier`, reserved REPLAYED/PREDICTED); the 9 existing telemetry-model tests are unchanged. The DOM contract the existing render tests rely on (`prov prov--<key>`, `data-provenance`, `data-tier`, `<svg>`, `data-state-kind`, roles) is unchanged, so no other test was edited. |
| Functional regression result | `cd ui; node --test test`: 215 tests, 215 pass, 0 fail (212 before; +3). `npm run check`: forbidden PASSED; contrast 196/196; a11y lint 74 combinations, 0 issues; tests 215/215. `npx vite build` ok (CSS 40.8 kB, JS 339.9 kB, unchanged dependency list). 2026-10-30. |
| Accessibility regression result | Static only: contrast script (text 4.5:1, UI 3:1, both themes, tinted states, new REPLAYED/PREDICTED hues), a11y lint, `tokens.test.mjs` (reduced-motion block, forced colours keep badge border style). In headless Chrome (CDP, one run, 1280 and 390): no horizontal overflow on the 8 routes and specimens screenshotted (scrollWidth equals clientWidth), press transform and its reduced-motion removal confirmed by forcing `:active`. NOT run: the full browser a11y/journey rounds (`browser-a11y.mjs`, `browser-journey.mjs`), keyboard walk-through, real zoom, forced-colors emulation, target-size sweep. UNVERIFIED. |
| Before screenshots | `docs/design/remediation/before/` (B0 gallery, not mine; 8+ opened: lab_home 1280 light/dark, run_detail_sb_completed 1280 light, run_form_t1_invalid 1280 light, overview 1280 light and 390 dark, simulation 1280 dark, lab_evidence_list 1280 dark). |
| After screenshots | `docs/design/remediation/i1_preview/` (21 files, `__1280__`/`__390__`, light and dark; 1280x800 viewport; not the B0 gallery script, so not strictly comparable: it is a throwaway CDP script, Chrome headless, debug gateway with the copied M6 and Scenario B files, `PHANTOM_MODE=development`). `specimen.html` renders the new classes in isolation (static markup, not data). Opened by me: lab_home light and dark, overview 1280 light, overview 390 dark, lab_run 1280 light, evidence_sb dark, simulation dark, specimen light and dark, specimen form viewport. |
| Provenance check | Badges: every value keeps a glyph, a word and a border pattern (solid for REAL, DERIVED; dashed for SIMULATED, REPLAYED, PREDICTED; dotted for UNAVAILABLE); REAL keeps a basis qualifier slot; INFERRED shown as DERIVED with qualifier "inferred" (seen on Overview Connection rows). REPLAYED and PREDICTED are not drawn by any screen; the telemetry model still rejects them (tested). Not changed by I1: tier wording from `lib/labProvenance.js` still says "Measured: loopback", so Lab badges still show the old word MEASURED, which now disagrees with REAL elsewhere (hand-off H1). The simulation frame lost its double border but keeps a solid orange frame and the banner; `SimDemo.jsx` is untouched (I3). No simulated or unavailable capability was made to look real; no label was hidden. |
| Remaining limitations | Section 8. |
| Dependencies on backend or owner approval | None. Owner decisions touched: `light-dark()` without fallback (section 6). |
| Status per item | Section 4. |

## 2. What changed (by file)

- `ui/src/styles/tokens.css` (rewritten): type tier tokens (size, leading, tracking, weight), single-source `light-dark()` palette, `color-scheme` with `data-theme` override, `--surface-raised/overlay`, `--tint-hover/press`, `--scrim`, `--shadow-1..3`, motion tokens, semantic spacing, radii 4/8/12, REPLAYED/PREDICTED provenance hues, deprecated `--text-*` aliases, `prefers-contrast` and `prefers-reduced-transparency`. Removed `--text-lg`, `--space-7`, the dark duplicates. One value tuned for contrast: dark `--text-3` #9aa4b5 to #a5afc0 (it failed 4.5:1 on the raised surface with the pressed tint).
- `ui/src/styles/app.css` (rewritten, same class names kept): type tiers on headings; panel L1 with shadow; nested panel guard; L2/L3 classes (`.menu .popover .elev-2 .sticky-bar .scrim .dialog`); press and hover tints; `.btn` variants, disabled, busy; tabs, segmented control; form-group classes; table row tints; state-block kinds; spinner and indeterminate bar; disclosure; badge anatomy; chart-frame; reduced-motion and forced-colours blocks; base text inputs; a global `[hidden]` rule (a `.btn { display:inline-flex }` rule had made the hidden Menu button visible at desktop width during preview; fixed and noted).
- Components: `ProvenanceBadge.jsx` (display words, qualifier, reserved classes, `DISPLAY`/`RESERVED_SRC` exports), `Icon.jsx` (+replayed, predicted, chevron), `States.jsx` (kinds, `Spinner`, `DisabledReason`, `StaleTag`, `InlineError`, `InlineSuccess`; `ConnectionBanner`, `SimulationBanner`, `StatusPill` unchanged), `Panel.jsx` (`level`, `plain`, title class), `Metric.jsx` (stale tag with glyph), new `ChartFrame.jsx` (title, badge, Chart/Table toggle with the table always in the DOM, full-width axis line with X, Y and unit, caption).
- `ui/scripts/check-contrast.mjs` (rewritten): reads `light-dark()` pairs including `rgb()` with alpha, blends tints over surfaces, 98 pairs times 2 themes. Also usable as a module (exports `ratio`, `parseThemes`).
- `docs/design/tokens.md` (rewritten to match), tests (above), `docs/design/remediation/i1_preview/*`.

## 3. Compliance table

| Change | Skill sections | Project override applied | How checked |
|---|---|---|---|
| Type tiers with size-specific tracking and leading, `rem`, tabular numerals; display steps down at 30rem | S15 (tracking, leading, Dynamic Type, hierarchy from weight plus size plus leading), S16 Craft | Minimum 12 px; system fonts only | `tokens.test.mjs` (tiers, negative/zero/positive tracking, ordering); headless Chrome computed sizes (28/20/15 px, 0.12 px caption tracking) |
| Elevation L0-L3, `--shadow-1..3` applied to panel/menu/dialog/sticky; dark steps | S12 (depth, size encodes thickness, dim to focus) | **S12 translucency and blur rejected: solid surfaces only, scrim without blur** | `tokens.test.mjs` (usage, no literal shadows, no `backdrop-filter`); screenshots opened |
| Motion tokens, press scale 0.97 at 100 ms, tint on rows/tabs/nav, hover tint, focus ring, busy and disabled | S1 (respond on pointer-down), S3 (no input lock), S4 (damping 1.0, no bounce), S7 (menu origin custom property), S11 (transform and opacity only) | No springs, no new dependency (S4-S6, S9, S13 skipped) | `tokens.test.mjs`; CDP forced `:active`: `matrix(0.97...)`; UNVERIFIED by feel or slow-motion review |
| Reduced motion: cross-fade only, no scale, loops static | S14 | Spec section 6 | `tokens.test.mjs`; CDP: transform none and spinner animation none under reduce |
| Borders: dashed/dotted only on badges; no double | S16 Craft, Simplicity | Spec section 5 | `tokens.test.mjs` (4 `lab.css` leftovers bounded, hand-off H3) |
| One state grammar with glyph plus text | S16 tactical (four feedback kinds, wayfinding), S13 utility | Spec section 10 | `tokens.test.mjs` rendered markup (11 kinds); a11y lint |
| Badge simplification, REAL/DERIVED wording, reserved REPLAYED/PREDICTED | S16 Simplicity, Familiarity | **Provenance never hidden or played down (non-negotiable)**; DOM contract kept | `provenance.test.mjs` (+3), render tests unchanged and passing, specimen screenshot |
| Form-group classes (touched-state validation, sticky action bar, choice list) | S16 (grouping and mapping, inline validation, Agency) | Spec section 9 | Specimen screenshot only: **no product form uses them yet (I2)**; classes UNVERIFIED in a real form |
| Single-source dark theme (`light-dark()`) | S14 (themes, avoid abrupt jumps: no transition added) | Browser baseline Chrome 123, Safari 17.5, Firefox 120; no fallback | `tokens.test.mjs`; `check-contrast`; Chrome 156 screenshots light and dark; older browsers UNVERIFIED |
| Unused tokens removed or used | S16 Craft ("nothing is random") | none | `tokens.test.mjs` allowlist test |
| Contrast script extended | S14 (contrast), S16 | Text 4.5:1, UI 3:1 | `node scripts/check-contrast.mjs` 196/196 |
| `ChartFrame` | S16 (wayfinding, label what a thing is) | Table alternative always in the DOM; badge on every chart | `tokens.test.mjs` markup test; not rendered in a page yet |

## 4. Status per item

| Item | Status |
|---|---|
| Type tiers and aliases | IMPLEMENTED, TESTED (static + computed style in Chrome) |
| Elevation applied | IMPLEMENTED, TESTED (static); look UNVERIFIED |
| Motion tokens, press feedback, reduced motion | IMPLEMENTED, TESTED (static + forced pseudo-state); feel UNVERIFIED |
| Border rules | IMPLEMENTED for `app.css`, `tokens.css`; BLOCKED for `lab.css` leftovers (not mine) |
| State grammar | IMPLEMENTED, TESTED (markup); not yet used by all screens |
| Badge anatomy and vocabulary | IMPLEMENTED, TESTED; mapping at the source (`lib/provenance.js`, `labProvenance.js`) BLOCKED pending lead (H1) |
| Form-group CSS | IMPLEMENTED; UNVERIFIED in use |
| Single-source theme | IMPLEMENTED, TESTED; older-browser behaviour UNVERIFIED |
| Contrast script | IMPLEMENTED, TESTED (196/196) |
| `ChartFrame` | IMPLEMENTED, TESTED (markup); not adopted |
| Shell/pages CSS split (plan item 11) | NOT DONE by design (`shell.css` is I4's) |
| Independent review of any of the above | none: nothing here is INDEPENDENTLY VERIFIED |

## 5. Files (owner: I1 agent)

`ui/src/styles/tokens.css` (159 lines), `ui/src/styles/app.css` (366), `ui/src/components/ProvenanceBadge.jsx` (48), `Icon.jsx` (75), `States.jsx` (147), `Panel.jsx` (22), `Metric.jsx` (45), `ChartFrame.jsx` (42, new), `ui/scripts/check-contrast.mjs` (115), `ui/test/provenance.test.mjs` (151), `ui/test/tokens.test.mjs` (226, new), `docs/design/tokens.md` (116), `docs/design/remediation/i1_preview/` (21 files incl. `specimen.html`), this record.

## 6. Decisions that need the lead or owner

1. **`light-dark()` without fallback.** Browser baseline is stated; no `@supports` copy of the palette (it would recreate the duplicated block). Accept or request a fallback.
2. **Heading sizes changed globally** (`h1` 28 px, `h2` 20 px, `h3` 17 px; panel titles stay h2 elements at the 17 px title tier through `.panel__title`; `h3` inside a panel is dense semibold). Un-migrated screens therefore look bigger in places; this follows spec section 1.
3. Radii follow the spec (md 8, lg 12), so panels, buttons and inputs are slightly rounder than before.
4. Dark `--text-3` brightened (contrast).

## 7. Hand-offs for the lead (files I do not own; no edits made)

- **H1 (provenance wording, single mapping place).** `ui/src/lib/provenance.js` `PROVENANCE` still has `measured.label = 'Measured'` and `inferred.label = 'Inferred'`; `ui/src/lib/labProvenance.js` `TIERS` labels are "Measured: loopback", "Retained: measured loopback", "Derived", etc. The badge now draws REAL / DERIVED + "inferred" from its own `DISPLAY` map, but any caller passing `label` (all Lab tier badges) still shows MEASURED, so Lab and non-Lab disagree. Proposal: make `lib/provenance.js` the single source by adding `display: 'Real'` and `qualifier` fields (inferred: word `Derived`, qualifier `inferred`) and have `ProvenanceBadge` read them; in `labProvenance.js` change `MEASURED_LOOPBACK.label` to a word-plus-qualifier pair (`Real` + `loopback · single host`), `RETAINED_*` labels to a separate small tag (as the audit item 6 proposes), and update `lab.render.test.mjs` line 210 (expects "Measured: loopback") and `lab.view.test.mjs`. Also `screens/Overview.jsx:49` note says "Items marked Inferred ...": would read "Derived · inferred". I did not edit them.
- **H2 (lab.css, I2).** Adopt `.field`, `.field-group`, `.choice-list`, `.form-summary`, `.form-bar`, `.tabs/.tab`, `.dialog/.scrim`, `ChartFrame`. Remove duplicated input and `.btn--danger` rules (now in `app.css`; identical values, so no conflict). `.lab-error::before { content: "Problem: " }` should go in favour of `InlineError` (glyph + text).
- **H3 (lab.css dashed leftovers).** `.btn:disabled { border-style: dashed }` (overrides my solid disabled button, so Lab disabled buttons are still dashed), `.lab-tag` dashed, `.lab-effect__zero` dashed, `.lab-unavail-panel` dotted. Replace with solid; the test `PENDING_I2_DASHED` list becomes empty when done and the list can be deleted.
- **H4 (Shell.jsx / shell.css, I4).** The 960 px rule hiding the Menu toggle is now `.app-nav .app-nav__toggle` (specificity fix after `.btn` got `display:inline-flex`); keep it when the shell moves to `shell.css`. `.lab-subnav a` should become `.tab`. `.app-header__brand` uses the title tier. A status strip is not provided (spec section 8): I4 builds it.
- **H5 (Charts.jsx, I4 and LabCharts.jsx, I2).** Wrap sparklines and bars in `ChartFrame` (it never draws data; callers pass chart and table nodes).
- **H6 (SimDemo.jsx, I3).** Inline colours and `font-size` below 12 px remain; `.sim-frame` is now a solid frame.
- **H7 (a11y.test filter, forbidden tests).** Untouched. `tokens.test.mjs` does not scan `SimDemo.jsx`.

## 8. Critique (honest) and what remains for I2 to I4

**Improved (seen in screenshots, one viewport per page, my own eyes only):** headings have a clearer ladder (28/20/17/15/13/12) and the Overview and Lab titles read as titles; panels have a faint shadow in light and a lighter surface step in dark; provenance badges are lighter and read consistently (REAL, DERIVED with a visible "inferred" qualifier, dotted UNAVAILABLE); the double borders on the simulation banner and frame are gone; state blocks carry a coloured left rule plus glyph and text; the specimen shows disabled, busy, error and pristine-versus-touched fields as designed.

**Did not improve (expected, because foundations do not change page structure):** Lab still opens with the large Connection panel and two banners above the task (audit A-1 and the two-banner rule), so "Start a run" is still not the first action; the Lab sub-navigation, forms, charts and Lab cards are I2 and still use old class names (dashed Lab disabled buttons, "Problem:" prefix, MEASURED wording); the Simulation demo is unchanged inside its frame (I3); Overview still lists five tiles then two panels with no status strip, and the global header and Menu pattern are I4; nested boxes (cards in panels) remain wherever screens use `.lab-card` or `.metric`; long pages are still long. Visible new side effects: banner text is 15 px and headings are larger, so some pages are a little taller; the Overview Connection rows now show a heavier "DERIVED · inferred" badge.

**Not looked at:** every Lab sub-page at 390 and 820, evidence pages at 390, light-mode simulation, Freshness/Origin/Cluster pages, any page in forced-colors, high contrast or 200% zoom. Not compared against the B0 gallery pixel by pixel.

**Human-review items:** the six badges side by side at 100% and 200% zoom in light and dark (specimen); type ladder specimen; elevation specimen (light L1 shadow is faint, dark relies on surface steps); reduced-motion behaviour of spinner and disclosure; acceptance of `light-dark()` without fallback; whether 17 px panel titles over 13 px dense content feel right.

## 9. Environment and cleanup

Own throwaway processes only: gateway `phantom-cache-demo.exe` pid 5980 and Vite pid 33460 (started with `--port 3000 --strictPort`, killed by recorded pid), headless Chrome pids 11096 and the first run's instance (closed by `Browser.close()`, profile `D:/Temp/browser_profile_i1prev_*` removed by it). Lab dir `D:\Temp\i1_lab` (copies of verdict/manifest/analysis/summary files only) removed. Ports 3000 and 8085 had no listener before and after (a few FIN_WAIT/TIME_WAIT socket entries from closed connections remained). Preview script lives outside the repo in the session scratchpad. `git rev-parse HEAD` 456bb25930004f1896a84c53e776e7b3ef29ee16, branch `feat/publish-phantomcache-core`, `git status --short` 94 lines before and after (my session start and end). No git mutation, no dependency change, no Rust or cargo.
