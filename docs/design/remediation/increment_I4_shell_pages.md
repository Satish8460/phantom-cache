# Increment I4: Shell, navigation and non-Lab pages

**Status:** Complete pending lead review. **Date:** 2026-10-30. **Writer:** I4 agent (only writer of the files below). Tests are regression guards, not evidence of visual quality; visual quality is judged from the screenshots and by human review.

| Field | Content |
|---|---|
| Increment ID and name | I4, Shell, navigation and non-Lab pages |
| Explicit defects | B-01 (nav weight of dead ends), B-03 (Overview), B-04 (page consistency), B-07 (placeholder developer text), B-08 (footer claim), A-15/A-16 (phone header), N-6 (Overview overflow at 320), hand-offs 1 to 15 from the lead brief |
| Expected outcome | One compact header (name, status chip, theme control from 820 px, Menu below 820 px); side navigation from 820 px, sticky and full height; groups named for contents (Live gateway, Experiments, Explainer, Not available yet) with visible text tags; per-route `document.title` and a true per-route footer; Light/System/Dark theme control; non-Lab pages built from page header + headline-tier sections instead of boxed panels; charts in `ChartFrame`; Overview grouped by meaning with a "Not reported (n)" list; honest placeholders with links to real pages |
| Affected routes and states | all nine routes; loading, live, stale, disconnected (gateway stopped), degraded, error states via `ScreenFrame` (unchanged logic) |
| Files and owners (I4 owns all) | see "File inventory" |
| Spec sections | 1 (type tiers), 2 (gutters 1.5 rem from 820, page header), 4 (no nested panels, L2 phone menu), 5 (solid borders), 6 (reduced motion cross-fade, tint press), 7 (segmented control, ChartFrame, targets), 8 (wayfinding, per-route title), 10 (state grammar, Unavailable = word + reason), 11 (vocabulary), 12 (skip link first, focus after navigation, reflow 320, 24/32/44 targets) |
| apple-design sections | S1 (tint press on nav rows), S7 (menu returns focus to its button, enter and exit by the same path), S14 (reduced motion: `pc-fade`, no transition on nav links), S15 (tiers h1 28 / h2 20 / metric 28, tracking by size), S16 (Purpose, Simplicity: common path first, Familiarity, wayfinding group caption, specific labels, grouping). Overrides: S12 materials not used (solid L2 menu panel), no springs. |
| Skill use (truth) | The Skill tool was called once with a concrete question. It worked as a loader: it returned the full SKILL.md text as instructions; it did not return a separate answer to the question. The pinned SKILL.md and the project rules were also read directly in full, and the design spec was read in full before any edit. |

## Gutter finding (hand-off 5): capture artifact, not an application defect (MEASURED)
`measurements.json`, 1280 px, Overview, headless Chrome 156.0.8078.79:
- Viewport mode: `innerWidth` 1280, `clientWidth` 1265 (15 px scrollbar). Header and `.app-main` right edge 1265. Last content block right edge 1241, `.app-main` padding-right 24 px: a 24 px right margin.
- Layout with no scrollbar (viewport resized to the document height, which is what a full-page capture lays out): `clientWidth` 1280, content right edge 1256 (24 px margin).
- `Page.captureScreenshot` with `captureBeyondViewport` returned a PNG **1265 px wide** (viewport shot 1280 x 800, full-page shot 1265 x 1686). Content was laid out at 1280 and the image was cropped to the scrollbar-reduced 1265, losing 15 px on the right: 24 - 15 = 9 px visible margin (1 px before I4 with the old 16 px padding, which is the "flush" look). The same crop is visible at 390 (full-page PNG 375 wide). So: capture artifact (Chrome full-page capture in this harness). The app itself had and has a proper right gutter; I4 additionally raised it to 24 px at 820 and up. The full-page PNGs in `i4_preview/` still carry the crop; read the viewport PNGs for true margins.

## What changed
- Shell: `Shell.jsx` (Header with status chip, theme control, Menu button; Nav with real nested group lists, tags, theme control and gateway facts in the nav foot), `App.jsx` (menu state, Escape returns focus, skip link moves focus without writing `#main` into the hash router, per-route title and footer), `shell.css` (grid areas, 820 px breakpoint, sticky full-height side column, phone menu panel, reduced motion, forced colors), `routes.js` (groups, tags, `LAB_VIEWS` order = LabScreen tabs, `titleFor`, `footerFor`), `theme.js` (`THEME_OPTIONS`), `index.html` (`theme-color` for both schemes).
- Pages: `ScreenFrame.jsx` (`PageHead` with group caption, `Section`, `NotReportedList`), `Overview.jsx`, `Traffic.jsx`, `CacheMemory.jsx`, `Freshness.jsx`, `Origin.jsx`, `Placeholder.jsx`, `Charts.jsx` (ChartFrame wrapping for Sparkline, new `PercentileChart`, `CapacityChart`), `pages.css`.
- `app.css`: shell and page rules moved out, `.sim-frame` removed (grep: no use anywhere), `.choice input` 24 px rule added (moved from lab.css), touch breakpoint 959 to 819 px, `.disclosure > summary` added to the touch size rule.
- Hand-offs 4 and 8: Overview text now "Items labelled Derived, inferred ..."; `a11y-lint` roles gained dialog, alertdialog, progressbar; harness exports RichNotice, `renderChart`, and takes the new shell props.

## File inventory (line counts)
App.jsx 155, components/Shell.jsx 127, components/Charts.jsx 159, lib/routes.js 119, lib/theme.js 50, screens/Overview.jsx 148, Traffic 40, CacheMemory 47, Freshness 33, Origin 45, Placeholder 81, ScreenFrame 108, styles/app.css 317, shell.css 94 (new), pages.css 49 (new), index.html 16, scripts/a11y-lint.mjs 107, scripts/harness-entry.jsx 78, test/a11y.test.mjs 79, misc.test.mjs 101, render.test.mjs 229, shell.test.mjs 260 (new), lab.fixes.test.mjs 270 (four assertions re-pointed, see below), `i4_preview/i4_gallery.mjs` 213 (new), this record. `ui/dist` rebuilt by `vite build`. Shared components Panel, States, Icon, Metric, ProvenanceBadge, ChartFrame were NOT changed.

## Tests
- Directory form `node --test test` (in `ui`): **258 tests, 258 pass, 0 fail** (suites 0). `npm run check`: forbidden PASSED (0 violations), contrast 196/196, a11y lint 74 combinations 0 issues, tests 258/258. `vite build` ok (CSS 52.29 kB, JS 355.80 kB).
- New `ui/test/shell.test.mjs` (19 tests, pass when run directly) is **not yet registered in `ui/test/index.js`** (lead's file): once registered the directory total becomes 277.
- `render.test.mjs`: sparkline test now asserts the ChartFrame (toggle, table in DOM, axis caption, "Constant at" for flat series); shell test asserts "Simulation" not "Simulation demo"; placeholder test asserts no developer sentence. `a11y.test.mjs`: `simulation` filter removed (sim page is linted in every state; shell linted open and closed). `misc.test.mjs`: titles, footers, Lab view order, theme options. Justification: each follows a behaviour this increment changed.
- `lab.fixes.test.mjs` F-01, F-02, F-03, F-04/F-05 (authorised by the lead): re-pointed to `shell.css` / `pages.css` and the new selectors, each with an `I4:` comment. Intent kept: single-column grid and desktop `minmax(0, 1fr)` column; `.app > * { min-width: 0 }`; no hidden page-level scroll rule (now also scans shell.css); header wraps; Menu hide rule is `.app-header .app-nav__toggle` inside `@media (min-width: 820px)` (more specific than `.btn`); aria-controls points at the list; Escape returns focus to the button and choosing a link closes the menu (asserted in App.jsx, where the state now lives); targets 32 px desktop / 44 px touch (touch breakpoint 819 px); nav icon size; sticky only from 820 px (the panel inside the stretched nav cell is the sticky element). `allCss` now includes shell.css and pages.css, so the fixed-width scans cover more, not less. `tokens.test.mjs` (unregistered, mine) was updated to read the nav press rule from shell.css (15/15). `lab.round2.test.mjs` (not registered in index.js) still has two assertions that name old shell selectors (N-1b `.app-header__meta`, N-3 `.app-nav a` / 959); not run by the directory form; flagged for the lead.

## Browser measurements (own run, headless Chrome, light unless stated; `i4_preview/measurements.json`)
| Check | Result | Status |
|---|---|---|
| Header height | 390, 640, 819, 820, 1280: 56 px; 320: 83 px (two rows: brand, chip and Menu wrap) | TESTED; 320 is two rows by design |
| Nav breakpoint | 819: Menu visible, nav hidden; 820: Menu hidden, 9 links visible, theme control in header | TESTED |
| Menu | Enter on the button opens (aria-expanded true); Escape from a link closes and focus is on the button; a link click closes, navigates, title changes and focus lands on the h1 | TESTED |
| Skip link | first Tab stop, visible at top 8 px; Enter focuses `main`, hash stays `#/traffic`, no "page does not exist" message. Before the fix it wrote `#main` into the hash (pre-existing defect, found and fixed in this increment) | TESTED |
| Keyboard order at 1280 | skip link, Light, System, Dark, nine nav links in group order, Details links, disclosure | TESTED |
| Sticky side nav | scrolled 700 px: panel top 0; nav cell height equals the page row (1498 px, full height) | TESTED |
| Theme control | Dark: `data-theme=dark`, stored `dark`, background `rgb(14,17,22)`; survives reload; Light and System (attribute removed, stored `system`) behave; `aria-pressed` follows | TESTED |
| Titles | all nine routes, Lab sub-routes and not-found: distinct and specific (list in measurements.json) | TESTED |
| Overflow 320/390/640/820/1280 | scrollWidth equals clientWidth on all nine routes (incl. Overview, N-6 closed) | TESTED |
| Targets | at 390 only the skip link (off-screen until focused) is under 44 px; at 1280 the Light/System/Dark and Chart/Table segmented buttons are 26 px high (inside a 32 px control; above the 24 px floor, under the 32 px desktop figure for primary controls) | TESTED, known limitation |
| Reduced motion | emulated: open menu animation `pc-fade`; without it `pc-appear` | TESTED |
| Type tiers | h1 28 px / -0.616 px tracking, h2 20 px, metric value 28 px (equals token), body-dense 13 px, group caption 12 px | TESTED |
| Console | no errors during the run | TESTED |
| Gateway stopped | status chip "Disconnected", zero metrics, "No connection to the gateway" block with Reconnect, nothing claims live data | TESTED |

## Previews and critique
Previews: `docs/design/remediation/i4_preview/{overview,traffic,cache,freshness,origin,cluster,intelligence,simulation,lab_home}__{390,820,1280}__{light,dark}.png` plus `__full.png`, and `gateway_stopped_overview__*` (60 states, 120 PNGs, `manifest.json`). Same file names as the before gallery. Opened by the writer at full resolution: overview 390 light (viewport and full), traffic 390 light, cache 390 light, traffic 1280 light, overview 820 light, cluster 820 light, freshness 1280 dark, origin 390 dark, intelligence 1280 light, simulation 820 dark, lab_home 1280 light, gateway_stopped_overview 390 dark and 1280 light, cache 1280 dark full: 15 images. Not opened: the other 105 (their sizes and overflow were measured by script only).

Honest critique:
- Better: header is one calm row on phones; wayfinding (group caption, named groups, current page by bar and weight); no boxes inside boxes; Overview now separates reported tiles from a short "Not reported (3)" list with reasons and UNAVAILABLE labels; capacity bar shows its scale when empty; placeholders say what they are and link to real pages; the gateway-stopped page states the facts plainly.
- Weaker: Overview groups with a single tile leave a large empty row (one 11 rem tile per section at 820 and 1280); the Chart/Table toggle appears even when a chart has nothing to draw (latency percentiles all unavailable); the status line on Overview repeats the header chip; metric tiles still use `surface-2` wells (a card look) and were not redesigned; at 390 the Origin "other fields" table sits in a small inner scroll box and its field names break mid-word; Simulation page keeps I3's boxed panels (not mine); on the live demo gateway most values are 0 or Unavailable, so the pages look sparse; segmented buttons are 26 px tall on desktop; the sidebar and page share nearly the same grey in light theme, so the column edge relies on a 1 px border. Light and dark themes were compared visually only in the images listed above.
- Not judged: forced colors, screen reader, 200 percent zoom, real users.

## Provenance check
Every tile, table row and list row keeps its badge. The word "Inferred" is gone from user-visible text (DERIVED with qualifier "inferred"). "Not reported" rows carry UNAVAILABLE. The nav tag "Simulated" is a neutral solid tag (dashed and dotted remain reserved for badges). The footer sentence differs by page kind and no longer claims schema-2 telemetry on Lab, Simulation or placeholders. The gateway mode shown in the nav foot reads "Reports mode: ..." and is not a labelled metric (Overview carries the labelled version).

## Compliance table
| Change | Skill / spec | Override | How checked |
|---|---|---|---|
| Header, Menu below 820, theme control | S16 Simplicity, spec 7, 8 | solid L2 menu (S12 skipped) | browser measurements, shell.test |
| Nav groups, tags, current page, group caption | S16 wayfinding, specific labels, spec 8 | none | screenshots, shell.test |
| Press feedback on nav rows | S1, spec 6 | tint, not scale | tokens.test (nav tint), CSS rule |
| Reduced motion for the menu | S14, spec 6 | none | emulated run |
| Page header and sections (tiers) | S15, spec 1, 2, 4 | none | computed styles, shell.test |
| Charts in ChartFrame, table alternative | spec 7, S16 | none | render.test, shell.test |
| Placeholders | S16 Purpose, Responsibility | none | shell.test, screenshots |
| Skip link, focus, titles, footer | spec 8, 12, S16 | none | browser measurements |

## Status per item
IMPLEMENTED and TESTED (regression tests plus own browser run): hand-offs 1, 2, 3, 4, 5 (finding), 6, 7 (rule moved into app.css; lab.css copy still present), 8, 9, 10, 11, 12, 13, 14, 15. INDEPENDENTLY VERIFIED: none. UNVERIFIED: visual quality, forced colors, screen reader, zoom, human first-minute task, light/dark parity beyond the opened images. BLOCKED: entry count, stale age/fraction, cluster and intelligence data need backend contracts (docs/design/ui_api_requirements.md).

## Hand-offs
1. Register `ui/test/shell.test.mjs` in `ui/test/index.js` (lead's file).
2. `.choice input` 24 px rule now lives in `app.css`; the copy in `lab.css` (lines 196-198) can be removed (I4 did not edit lab.css).
3. `lab.round2.test.mjs` (unregistered) N-1b and N-3 name removed shell selectors.
4. Tokens: `.app` redefines `--gutter` to 1.5 rem from 820 px (spec 2); `tokens.css` unchanged.

## Backend dependencies
Entry count, per-policy detail, age and fraction of stale responses, cluster membership/fan-out/recovery, intelligence signals and decisions: not in schema 2; shown as "not reported" with the reason. No backend change made.
