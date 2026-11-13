# PhantomCache design direction v2

**Author:** design-direction agent DD. **Date:** 2026-10-30. **Status:** PROPOSAL for owner and lead review. Nothing in `ui/**` was changed.
**Supersedes:** the project rule "no glass by default" (`docs/design/apple_design_skill_project_rules.md` section 3, and the design spec section 4, `docs/design/principles.md` section 3). Owner addendum: glass is allowed for functional layers, with fallbacks and measured cost.
**Companion files (all under `docs/design/prototypes/`):** `materials_perf.html` (performance rig), `perf/run_perf.mjs` (CDP runner), `perf/perf_sw.json` and `perf/perf_gpu.json` (raw results), `perf/contrast_worst_case.mjs`, `perf/palette_contrast.mjs` (computed contrast), `direction_preview.html` (look-and-feel prototype, static, not wired to a gateway).

Tags used: **FACT** (verified in code or image, with citation), **ESTABLISHED GUIDANCE** (from a source I opened), **PROPOSAL** (my design decision), **MEASURED** (run here, numbers in section 12), **UNVERIFIED** (not checked, or only seen as a snippet).

---

## 0. The short version

1. The UI is correct and honest but looks like a default dashboard. The owner's judgement is mostly right (section 1).
2. Direction name: **"Instrument and evidence"**. Solid, quiet, paper-and-ink surfaces carry all data. Glass appears only on the layers that float above content (header, rail, run bar, popovers, dialogs). A rule that is easy to remember and to test: **glass never carries data**.
3. Colour carries meaning, not brand. Neutrals build the hierarchy. Hue is reserved for provenance and state. The primary action is ink-filled, not blue, because blue already means DERIVED and "info" (`tokens.css:88,110,117`).
4. One signature mark: the **ghost outline** (a solid box with a dashed, offset copy behind it: "the cache copy that is not the origin"). It is the logo, and the only 2D depth device in illustrations. No WebGL, no CSS 3D.
5. Typography gets a voice: large tabular numerals with tight tracking for results, small caps-style labels with wide tracking, mono only for identifiers.
6. The status pile of pills is replaced by a **four-tier status hierarchy**; qualifications stay, but as a quiet text strip, not as competing pills.
7. Motion is state-driven and short (90 to 280 ms), with `linear()` or cubic-bezier easing, no dependency, and a cross-fade equivalent for reduced motion. Operational motion exists only when a real event occurred.
8. Evidence pages get a two-level structure: an "answer page" a stranger can read in 60 seconds, then the technical record. The binding notice and the "not established" list stay visible and equal in weight to the results.

---

## 1. Assessment of the current UI (Step 1)

### 1.1 What I read
- CSS: `ui/src/styles/tokens.css` (159 lines), `app.css` (317), `shell.css` (94), `pages.css` (49), `lab.css` (214), `sim.css` (23). Components: `Shell.jsx`, `ChartFrame.jsx`, `Charts.jsx`, `LabCharts.jsx`, `LabParts.jsx`, `ConnectionPanel.jsx` (head), `App.jsx` (lines 45-75), `lib/config.js`, `lib/connection.js` (exports). Screens were inspected through screenshots and file lists, not line by line.
- Docs: `principles.md`, `apple_design_skill_project_rules.md` (full), headings of the spec and token docs. The installed `apple-design` skill was **invoked through the Skill tool** with a concrete question and returned its full text (sections S1 to S17); I used S12 (materials), S14 (reduced motion/transparency/contrast), S4 and S3 (springs, interruptibility) and S16 (principles).
- Screenshots: 21 images opened. After: `lab_evidence_list` 1280 light and dark; `gateway_stopped_lab_home` 1280 light and 390 light; `evidence_scenario_b` 1280 light full page, 820 light, 390 dark; `overview` 1280 dark; `cluster` 1280 light; `run_detail_sb_running` 1280 light; `cancel_dialog` 820 dark; `lab_home` 1280 light; `origin` 390 light; `intelligence` 820 dark; `evidence_m6` 1280 dark; `gateway_stopped_overview` 820 dark; `run_detail_sb_completed` 390 light; `cache` 1280 dark. Before: `evidence_scenario_b` 1280 light, `cache` 390 dark. Widths covered: 390, 820, 1280. Both themes covered.

### 1.2 Material, depth and motion that exist today (FACT)
- No `backdrop-filter`, `blur()`, `perspective`, 3D transform or `will-change` anywhere in `ui/src` (grep over the whole tree: only a comment and `::backdrop { background: var(--scrim) }` at `lab.css:198-202`).
- Animation: five keyframes only, `pc-spin`, `pc-slide`, `pc-appear`, `pc-fade` (`app.css:270-273`), plus 100 to 200 ms colour transitions and the `:active` press scale 0.97 (`app.css:171`). Tokens: durations 100/150/200/300 ms and one easing (`tokens.css:59-63`).
- Elevation: three solid shadow levels (`tokens.css:99-101`), none in dark L1.
- The header is **not sticky** (`shell.css:178-183`); only the nav panel (`shell.css:234`) and the Lab run header (`lab.css:171`) stick.

### 1.3 The owner's observations, one by one

| # | Owner observation | Verdict | Evidence |
|---|---|---|---|
| 1 | Conventional shell | **Confirmed** | Full-width white header with brand text and a status pill, 216 px white side list with caption groups, footer line (`shell.css:167-253`; every 1280 screenshot). The brand is plain text; there is no mark and no identity. |
| 2 | Grey background with white bordered cards | **Confirmed in light, partly refuted in dark and in structure** | Light: `--bg #f4f5f7`, `--surface #ffffff`, `.lab-card` and `.panel` use a 1 px border plus shadow (`tokens.css:78-79`, `lab.css:59`, `app.css:69`). But the code already avoids a box for sections: sections are "rule plus whitespace" (`lab.css:34`, `pages.css:263`), and dark uses a blue-black surface ladder rather than a grey/white pair (`tokens.css:78-82`). The flat look comes from one family of greys, not from card overuse. |
| 3 | Many pills competing | **Confirmed** | First viewport of the Lab Evidence page (`lab_evidence_list__1280__light`) shows 1 header pill, 4 status-strip pills, then on the Scenario B card REAL badge, Retained tag, "Run class: VALID" pill, two verdict pills: about 11 bordered chips of three different shapes (`.pill` radius 999 `app.css:120`, `.prov` radius 4 `app.css:104`, `.tag` radius 4, `shell.css:210`). Equal size, equal weight, so nothing leads. |
| 4 | Uniform typography | **Partly refuted, partly confirmed** | A real scale exists: display 28, headline 20, title 17, body 15, dense 13, caption 12, metric 28, with size-specific tracking (`tokens.css:16-23`). So "no scale" is wrong. But in practice 13 px and 15 px dominate, only weights 400 and 600 are used, there is one family, and numbers have no display treatment. The page reads as one voice. |
| 5 | Disconnected state repeats information in a large red panel | **Confirmed, plus one defect** | `gateway_stopped_lab_home__1280__light`: header pill "Disconnected", red-bordered panel (`lab.css:15`) with "Not reachable or not authorised", the gateway URL, "did not answer the capabilities request", a token form, a paragraph about the intelligence engine (unrelated to the connection), then a second red-edged block "Could not reach the gateway", and the URL again in the nav foot. The error text says "the address shown in the header" but the header shows no address after the remediation (it is in the nav foot). This is a content bug. |
| 6 | Scenario B page long and hard to scan | **Confirmed** | `evidence_scenario_b__1280__light__full` is 4158 px tall. The binding notice (a long text with eight numbered qualifications plus conclusions) comes before "Result at a glance"; the contents list is inline text, not sticky (`lab.css:52`); eight collapsed folds follow. The reader meets limitations before knowing the result. |
| 7 | Cards with weak internal hierarchy | **Confirmed** | The Scenario B card: link-styled title, then a badge row, a Run-class pill, an explanatory sentence, an id, dates, two verdict pills, a link with a long parenthetical. No line says what question the experiment asked; the id is as loud as the verdicts. |
| 8 | Charts do not communicate enough | **Confirmed** | The origin-protection chart splits peak concurrency and client success into two bar lists and tells the reader to "read the two panels together" (`LabCharts.jsx:76-110`, screenshot). Latency is nine bars in three panels. The effect chart (`.lab-effect`) is the only one with intervals. None shows the trade-off itself. |

### 1.4 Other inconsistencies found (FACT)
- **One blue, three meanings.** `--accent #0b5fd1`, `--info #0b5fd1`, `--prov-derived #0b57a8` (`tokens.css:88,110,117`): the primary button, links, info notices and the DERIVED badge share a hue. A user cannot tell "clickable" from "derived" by colour.
- **Three chip shapes** for state, provenance and tag (see row 3).
- Status of retained evidence ("Retained") is a plain outlined tag while it is one of the most important facts on the page.
- The theme control is a three-part segmented control in the header at all widths from 820 px (`shell.css:229`); it takes prime space for a setting used once.
- On 390 px the Lab page puts the full status panel above the content (`gateway_stopped_lab_home__390__light`): the first screen contains no Lab content at all.

### 1.5 What to KEEP (FACT, verified)
- The token architecture: one `light-dark()` declaration per colour, theme override attribute (`tokens.css:1-9`).
- The provenance grammar: glyph + word + border style (solid, dashed, dotted), never colour alone (`app.css:100-112`). It is correct and the strongest existing idea.
- Chart and table twin always in the DOM (`ChartFrame.jsx`), text-first charts, "Unavailable is not zero".
- Rule-and-whitespace sections instead of nested boxes; cards only for entities.
- Fallback handling for `prefers-contrast`, `prefers-reduced-motion`, `forced-colors` already present in each CSS file.
- The contrast checker (`scripts/check-contrast.mjs`) that blends translucent tokens over a solid surface; extend it, do not replace it.
- Measure of 70 to 75 ch on prose (`lab.css:86`).
- Honest copy: limitations, "not established", "not an overall verdict".

### 1.6 What is incomplete
No material or depth system, no motion vocabulary beyond appear/fade/spin, no illustration, no connection settings UI (the gateway URL is resolved once from `?gateway=` or `VITE_PHANTOM_GATEWAY`, `App.jsx:56-58`, `lib/config.js`; there is no way to see or change it in the UI beyond reading the nav foot), no sticky section navigation on long pages.

---

## 2. Research record (Step 2)

| Source | How I used it |
|---|---|
| Apple HIG: Foundations, Materials, Motion, HIG root (URLs given in the brief) | **Opened, but the fetch returned only the page title** (the pages are rendered by script). I did **not** read their text. Where this document echoes HIG ideas (materials separate layers, keep legibility, respect Reduce Transparency; motion should be brief and optional) those come from the summarising tool's general knowledge and from the installed skill, not from the HIG pages. Treat as **UNVERIFIED against the HIG**. |
| Installed `apple-design` skill | **Invoked** (Skill tool, concrete question). Used for S12, S14, S3/S4, S16. Its rule "never stack light translucent on light translucent" and "scroll edge effects instead of hard dividers" are adopted. Its spring-library parts are not (project rule: no new dependency). |
| WCAG 2.2 (`w3.org/TR/WCAG22/`) | **Opened; first 100,000 of 269,706 characters read.** Verified: 1.4.3 (4.5:1, large text 3:1), 1.4.11 (3:1 UI components), 1.4.12 (line height 1.5, paragraph 2x, letter 0.12, word 0.16), 2.4.11 (focus not obscured), 2.4.13 (AAA, 2 px perimeter, 3:1), 2.3.3 (AAA, motion from interactions can be disabled). **2.5.8 target size was beyond the text read**; the 24 CSS px figure is from the project's own docs (`apple_design_skill_project_rules.md` section 4), **UNVERIFIED here**. |
| MDN, Learn CSS performance | **Opened and read.** Animate transforms, opacity (and filter) rather than layout or geometry; avoid animating `box-shadow`; `will-change` is "a last resort"; `contain` and `content-visibility` limit work; measure first. |
| web.dev animations guide | **Opened and read.** Restrict animation to `opacity` and `transform`; blurs and shadows are paint-heavy; `will-change` forces a layer, use sparingly. |
| Search snippets (not opened as pages): backdrop-filter cost, `prefers-reduced-transparency`, `linear()`, dumbbell charts | **Snippets only.** `prefers-reduced-transparency`: Chromium-based supported from 119, Firefox and Safari not (caniuse and MDN listings, via search results); so it cannot be the only fallback. `linear()`: MDN lists Baseline 2023 (snippet). Backdrop-filter cost: no authoritative browser benchmark found; one source gives a "3 to 5 simultaneous blurs" rule of thumb with no measurement behind it. I treat that as **unverified folklore** and rely on section 12 instead. `@starting-style` and View Transitions support: **not confirmed** (the search returned nothing); treated as progressive enhancement only. Dumbbell and small-multiple guidance: package documentation only; no authoritative guidance on intervals found, so chart rules below are **my proposals**. |

Established guidance I rely on: contrast thresholds above; animate only compositor properties; respect reduced motion; test, do not guess, performance.
Everything else in this document is **PROPOSAL**.

---

## 3. Measured performance (Step 3)

Full tables in section 12. The rig is `prototypes/materials_perf.html`, driven by `prototypes/perf/run_perf.mjs` over CDP.

**MEASURED, one host, headless Chrome 156, 5 runs per cell** (tables in section 12; raw JSON in `prototypes/perf/`):
- **Affordable (no long frames at 1280x800 and at 390x844 with 4x CPU throttle, on both the software and the hardware path, medians):** backdrop blur 12 px on 2 surfaces, on 6 surfaces, and on a large header + full-height rail + toolbar set; the same set at 32 px radius; the same set with panels continuously animating opacity/transform beneath it (variants b, c, d, d32, e).
- **Not affordable:** (f) six cards with perspective tilt and a class toggle every 250 ms: at 390 px / 4x it cut the frame count from 719 to 295 in 3 s on the hardware path (29 long frames, p95 25 ms) and produced 20 long frames on the software path; Paint time in the single traced run rose from about 9 to 15 ms to about 245 to 1,550 ms. (h) blur on all 90 data panels at 1280 px: 29 long frames and 150 instead of 179 frames on the software path. A full-viewport 8 px blurred scrim (g) produced 7 long frames (software, 1280 px).
- **Costs that do not show as long frames but are visible in the trace:** animating panels under glass (e) raised "Prepaint" main-thread time about 9x at 1280 px (software path) and 15x to 20x at 390 px / 4x. Cheap in this synthetic page; it will add up in a real page with much more DOM.
- **Decision from this:** limited glass on chrome only is affordable; 3D tilt and glass on data are rejected; the scrim stays dim-only (no blur). Limits are in section 12.4. These are a synthetic page on one machine, not the real app and not a phone.

---

## 4. Design direction (Step 4, items 2 to 4)

### 4.1 Identity statement (PROPOSAL)
PhantomCache is an instrument for people who distrust numbers. The interface should feel like a well-made lab instrument and a printed technical report at once: calm, exact, a little austere, with a few moments of material craft where something genuinely floats. It must never look like a marketing site, a trading terminal, or neon "cyber" infrastructure.

Signature elements:
1. **Paper and ink.** Light theme is warm paper (`#f3f2ee` page, `#fbfbf9` surface), not grey-white. Dark theme is blue-black ink (`#0b0e13`, `#12161d`), designed separately, not inverted.
2. **Solid = evidence, glass = chrome, hatched = not real.** A three-part material language. Data, tables, charts, verdicts and prose always sit on solid surfaces. Glass marks a layer that floats above content. Hatched or dashed grounds (already the grammar for SIMULATED and UNAVAILABLE) mark things that are not live measurements.
3. **The ghost mark.** Solid rounded square with a dashed copy offset behind it (`direction_preview.html`, `.mark`). It is the logo. In illustrations the dashed copy marks the cache entry that is not the origin.
4. **A fine-line technical grid** on the page background only (32 px, 3.5 to 4.5 percent opacity). Static background gradients, no cost at scroll (section 12 rig uses a similar background). It sits behind everything and disappears under solid surfaces. Justified: it gives the otherwise flat page a measured, graph-paper quality and costs nothing.
5. **Numerals as a display element.** Result numbers use 28 px or larger, weight 650, tracking -0.03em, tabular. Labels use 11 px caps with 0.08em tracking.
6. **Ink-filled primary action.** Blue stays for links and DERIVED. The primary button is near-black on light, near-white on dark.

### 4.2 Palette logic
- Neutral ladder carries structure: `--bg`, `--surface`, `--surface-2`, `--surface-3`, `--line`, `--line-strong`, `--ink`, `--ink-2`, `--ink-3`.
- Hue is meaning only: green REAL/ok, blue DERIVED/info/link, amber SIMULATED/warn, red critical/adverse, teal REPLAYED, violet PREDICTED, grey UNAVAILABLE. There is **no brand accent**. Brand = ink + ghost mark.
- Focus ring stays blue (`--focus`), because it is a function, not a meaning.
- **MEASURED (computed):** all 38 text/UI pairs of the proposed palette pass (4.5:1 text, 3:1 UI) in both themes; the lowest text ratio is 5.38 (light, ink-3 on surface-2) and 5.62 (light, REAL on its background); `perf/palette_contrast.json`. Dark lowest text ratio 6.76 (UNAVAILABLE on its background).

Token values (PROPOSAL; these are the values in `direction_preview.html`):

| Token | Light | Dark |
|---|---|---|
| `--bg` | `#f3f2ee` | `#0b0e13` |
| `--surface` | `#fbfbf9` | `#12161d` |
| `--surface-2` (wells, table head) | `#ecebe6` | `#181d26` |
| `--surface-3` (selected, pressed) | `#e2e1db` | `#1f2531` |
| `--line` (decorative hairline) | `#d9d8d1` | `#262d3a` |
| `--line-strong` (UI boundary, 3:1) | `#6c7080` | `#8591a6` |
| `--ink` / `--ink-2` / `--ink-3` | `#14181f` / `#3d4452` / `#575f6e` | `#eef1f6` / `#bcc4d1` / `#a3adbd` |
| `--primary` / `--on-primary` | `#14181f` / `#fbfbf9` | `#eef1f6` / `#0b0e13` |
| Provenance and state text/bg | unchanged hues from `tokens.css:104-126`, backgrounds warmed slightly | unchanged |

### 4.3 Surfaces and materials

Four levels. Only L2 and L3 may be glass.

| Level | Use | Solid or glass |
|---|---|---|
| L0 page | page ground with technical grid | solid |
| L1 content | cards, sections, tables, charts, verdicts, evidence text, forms | **solid always** |
| L2 float (thick) | app header, navigation rail | glass-1 |
| L2 float (thin) | floating run bar, popovers, menus, toasts | glass-2 |
| L3 modal | dialog, with scrim | glass-2 dialog on a **dim-only** scrim (no blur on the scrim; section 12.4) |

**Where glass is used (PROPOSAL):** app header; navigation rail (desktop); floating run/action bar; popovers and menus (gateway settings, theme menu); dialogs; toasts.
**Where glass is NOT used:** data tables, evidence text, notices, charts and chart frames, verdict rows, cards, form fields, anything that states a number or a limitation. Reason: legibility (worst-case contrast, section 4.4) and integrity (a number must never look like chrome).

#### Material token spec (PROPOSAL, values tested in the prototype)

| Token | Light | Dark | Notes |
|---|---|---|---|
| `--glass-1` (header, rail) | `rgb(251 251 249 / 0.90)` | `rgb(18 22 29 / 0.88)` | thick: higher opacity |
| `--glass-2` (run bar, popover) | `rgb(251 251 249 / 0.86)` | `rgb(24 29 38 / 0.84)` | thin; stronger shadow |
| `--blur-1` / `--blur-2` | 14 px / 10 px | same | radius max 14 px (section 12) |
| `--sat` | 1.25 | 1.25 | `saturate()` after blur keeps colour under glass lively |
| `--glass-edge` (inset top highlight) | `rgb(255 255 255 / .85)` | `rgb(255 255 255 / .09)` | `inset 0 1px 0` |
| `--glass-hair` (outline) | `rgb(20 24 31 / .10)` | `rgb(255 255 255 / .08)` | replaces 1 px border, `0 0 0 1px` |
| `--sh-float` | `0 8px 24px rgb(20 24 31 / .14)` | `0 8px 24px rgb(0 0 0 / .45)` | context-aware: the bar floats over text, so it gets a deeper shadow than the header |

Rules: header and rail use a hairline inset shadow, not a border (skill S12 "scroll edge" idea: edge separation only where content actually passes under). Never put one glass layer over another glass layer (the run bar must not overlap the header or rail; popovers open over the solid page, and when a popover must open from the header it is itself `--surface-raised` solid). Glass opacity is **context-aware**: when nothing is scrolled under the header (`scrollY = 0`), the header may use solid `--surface` (optional enhancement, one class toggle on scroll, no per-frame work).

#### Fallback ladder (PROPOSAL; all implemented in `direction_preview.html`)
1. `@supports not (backdrop-filter: blur(1px))` (also the prefixed form): glass tokens fall back to solid `--surface` / `--surface-raised`.
2. `prefers-reduced-transparency: reduce`: solid surfaces, `backdrop-filter: none`. Only Chromium supports this query (snippet, UNVERIFIED), so rule 5 below matters.
3. `prefers-contrast: more`: solid, strong borders (`--line: var(--line-strong)`).
4. `forced-colors: active`: `Canvas`, 1 px `CanvasText` border, no blur.
5. **Contrast guarantee that does not depend on the fallbacks.** Glass alpha is chosen so text on the worst possible backdrop (pure black behind a light glass, pure white behind a dark glass) still passes. **MEASURED (computed, `perf/contrast_worst_case.json`):** light glass over pure black: alpha 0.90 gives ink 14.26, ink-2 7.49, ink-3 5.09, link blue 4.71; alpha 0.85 gives ink-3 exactly 4.50 and blue 4.17 (fails). Dark glass over pure white: alpha 0.88 interpolates between 0.85 (ink-3 5.00, accent 4.56) and 0.90 (5.91, 5.39); so 0.88 passes all four. Therefore: **chrome carrying `--ink-3` or link/accent text uses alpha at least 0.90 (light) and 0.88 (dark)**; alpha 0.84 to 0.86 (glass-2) is allowed only for surfaces that carry `--ink` and `--ink-2` text (run bar status text), which hold at least 5.06 (dark, 0.80) and 5.08 (light, 0.75). Real backdrops are never pure black or white after blur, so real ratios are higher; the guarantee is the floor.
6. Add the glass tokens to `scripts/check-contrast.mjs` as pairs blended over **both** extremes, not over the solid surface only.

### 4.4 Type system (PROPOSAL, system fonts only, no font files)
| Role | Size / leading / tracking / weight | Use |
|---|---|---|
| Display result | 1.75 to 2.25 rem / 1.05 / -0.03em / 650, tabular | the number in a result tile |
| Page title (h1) | clamp(1.625rem, 3.2vw, 2.125rem) / 1.1 / -0.03em / 650 | one per page |
| Section (h2) | 1.25 rem / 1.2 / -0.02em / 650 | |
| Title (h3) | 1.0625 rem / 1.25 / -0.01em / 650 | card title |
| Body | 0.9375 to 1 rem / 1.5 to 1.6 / 0 / 400 | prose uses 1 rem and 68 ch |
| Dense | 0.8125 to 0.875 rem / 1.4 / 0 / 400 to 500 | tables, key-value |
| Eyebrow | 0.6875 rem / 1.3 / +0.08em / 650, caps | group captions, field headers |
| Mono | `ui-monospace`, 0.8125 rem, slashed-zero optional | ids, hashes, URLs only |

Body stays at least 15 px for reading text; nothing below 11 px. Eyebrows are caps labels, never sentences. The result number is the largest thing in its card; the id is the smallest.

### 4.5 Spacing, grid, lines, icons
- Keep the 4 px base. Add a **page rhythm**: 8 / 16 / 24 / 40 for tight / group / section / region. Sections in a long page are separated by 40 px and a hairline, not by boxes.
- Radius: 6 (controls, marks), 10 (cards), 14 (floating bars and dialogs). Pills (radius 999) are retired.
- Fine-line system: the 32 px grid ground, 1 px hairlines, and dotted or dashed lines only where they mean "not real". Tick marks on chart axes use the same hairline.
- Icons: keep the current inline SVG set (`Icon.jsx`), 14 to 16 px, 1.5 px stroke, glyph shape differs per state (check, cross, clock, slash, link-break) so state never depends on colour. Add three: ghost mark, gateway (plug), retained (archive box).

---

## 5. Depth and 3D (Step 4, item 3)

**Decision (PROPOSAL, informed by section 12): no CSS 3D in the product UI.** No `perspective`, no `preserve-3d`, no hover tilt, no WebGL, no new dependency.

Reasons: (a) a tilt on a text-dense card has no meaning; (b) it needs layer promotion for each card and adds per-card transform work (see rig variant f in section 12 for the cost I measured); (c) 2.3.3-style motion from interaction would need a reduced-motion variant anyway; (d) the owner's brief allows 3D only "where it explains a system concept".

Where layered or isometric **illustration** earns its place (all static inline SVG, flat 2D offsets, no transforms at runtime):

| Candidate | What the illustration explains | Label | Integrity rule |
|---|---|---|---|
| Request flow and coalescing (Simulation page, About tab) | N clients to one gateway fetch to one origin call | `ILLUSTRATION: not live, no data` (a SIMULATED-style dashed mark) | no numbers, no animation tied to anything but a user-pressed "step" control (optional) |
| Origin protection cap (Lab About) | a gate on the origin path: requests above the cap are shed (503) | same | the cap value in the drawing is the word "cap", never a number |
| Node relationships (Cluster placeholder, "What this page would show") | three boxes and arrows for membership, fan-out, recovery | `ILLUSTRATION: not live. Cluster state is not reported` | never drawn with node health colours, never with counts; the page keeps its "Not available" notice above it |
| Evidence layers (Evidence list) | "retained files" under "read-only view" as stacked sheets | decorative, aria-hidden | optional, can be dropped |

Rules for all illustrations: dashed outline for anything that is not the origin; ink and ink-2 strokes only (no state colours); always inside a bordered figure with the mark; `role="img"` with a text alternative; the text beside it says what it does not show. Never depict simulated or unavailable infrastructure as live (no status dots, no counters, no animated packets).

---

## 6. Motion system (Step 4, item 4)

ESTABLISHED GUIDANCE (opened sources): animate `transform` and `opacity`; blurs and shadows are paint-heavy; `will-change` is a last resort; respect reduced motion. Everything below is PROPOSAL.

### 6.1 Tokens
```css
--d-press: 90ms; --d-fast: 140ms; --d-base: 200ms; --d-slow: 280ms;   /* nothing above 300 ms */
--ease-out: cubic-bezier(.2,.8,.2,1);          /* default: arrivals */
--ease-in-out: cubic-bezier(.4,0,.2,1);        /* state changes in place */
--ease-settle: cubic-bezier(.2,.8,.2,1);       /* fallback for linear() */
--ease-spring: linear(0, 0.144, 0.385, 0.594, 0.745, 0.845, 0.908, 0.947, 0.969, 0.983, 0.99, 0.995, 1); /* critically damped, computed: 1-(1+8t)e^-8t */
```
`--ease-spring` is used by dialogs, popovers and the run bar entering; wrap with `@supports (transition-timing-function: linear(0, 1))` and fall back to `--ease-out` (MDN snippet says Baseline 2023, **UNVERIFIED** beyond that). No overshoot: there is no gesture with momentum (skill S4: bounce only after a flick).

### 6.2 Patterns
| Pattern | Spec |
|---|---|
| Press | `transform: scale(.97)` 90 ms on pointer-down (exists). Dense rows: tint only. |
| Hover | background tint or border colour, 140 ms. No movement. |
| Selection | nav current marker and tab underline change by `transform: scaleX/translate` 200 ms, ease-out; the selected state is also in the DOM (`aria-current`). |
| Tab and page transition | cross-fade 160 ms of the content region. If `document.startViewTransition` exists use it, else a CSS opacity fade of the new content (**View Transitions support UNVERIFIED**, so it is an enhancement only). Focus moves to the new heading as today. |
| Disclosure | content fades and shifts 4 px (existing `pc-appear`, 200 ms). Do not animate height. |
| Floating layers | popover and dialog enter from the trigger (`transform-origin` at the trigger): opacity 0 to 1 and `scale(.97)` to 1, 200 ms `--ease-spring`. Exit reverses on the same path, 140 ms. The blur radius is **not** animated; only opacity and transform (animating blur radius is UNVERIFIED for cost, so avoided). |
| Progress / operational | `<progress>` value change: transform-based fill 200 ms linear. Spinner only while a request is actually in flight. New event rows in a run log: one 600 ms background highlight fade, at most one highlight per 250 ms. Looping animation is allowed only for the in-flight spinner and the indeterminate bar (existing, tied to real state). |
| Status change | colour cross-fade 140 ms plus the existing polite live-region announcement. No pulsing dots; "recovering" is shown as text with a countdown. |
| Copy confirmation | label changes to "Copied" with a check for 1.6 s, polite live region. No motion. |
| Stagger | first paint of a list of at most 6 items: 30 ms steps, total 200 ms max. Never on data refresh, never on live updates. |

### 6.3 Reduced motion and mobile cost
- `prefers-reduced-motion: reduce`: every transform animation becomes an opacity fade of at most 120 ms; spinner and indeterminate bar become static (existing behaviour, `app.css:281-282`); progress fills jump (state, not motion). Springs fall back to a plain fade.
- On mobile (390 px class): at most 2 simultaneous animations; no glass animation; dialogs fade only. MEASURED context in section 12 (variant e).

---

## 7. Shell and navigation (item 5)

### 7.1 Desktop (820 px and up) (PROPOSAL)
- **Header, 56 px, sticky, glass-1.** Left: ghost mark + "PhantomCache" (second word "Control Center" becomes an eyebrow). Right: **connection indicator** (dot + word + gateway host, one button), then a compact **theme button** (icon, opens a glass-2 menu: Light, System, Dark). The three-part segmented theme control is removed from the header.
- **Rail, 232 px, sticky, glass-1,** groups with eyebrow captions: Live gateway (5), Experiments (Phantom Lab), Explainer (Simulation, tag "Simulated"), **Not available yet** collapsed into one disclosure showing the count ("Not available yet, 2"), expanded by default for first-time users and remembered per viewer (try/catch storage). Tags use the dashed `Simulated` / `Not available` style, as now.
- **Gateway block** at the rail foot: address in mono, "Change address", mode line. Same data as the connection popover; the rail shows it, the header opens the full control.
- No footer line on app pages; the footer sentence moves into the page "About this data" line where it applies.

### 7.2 Tablet 820 px class
Rail stays (it is sticky and glass); main column narrows. Header unchanged.

### 7.3 Phone (390 px class)
Header: mark, connection dot with the word "Live" or "Offline", Menu button. Menu opens a **solid** sheet anchored to the button (no blur on phones, section 12), with the grouped list, theme choice and the gateway block. Run bar on phones is solid (single surface), and at most one glass surface exists on a phone (the header).

### 7.4 Status hierarchy (replaces the pill pile)
| Tier | What it holds | Presentation | Count budget |
|---|---|---|---|
| 1 Critical operational state | gateway connection (connected, unreachable, unauthorised, recovering); a run's state (running, failed, cancelled) | header indicator and, for runs, one state word beside the title; the only things that use a red or amber **filled** treatment | at most 1 visible per context |
| 2 Environment qualifications | Debug build: results EXPLORATORY; loopback, one host; anonymous; Lab API enabled; intelligence not in serving path | one quiet **strip** of text items, each with a 6 px square marker in the state colour, under the page head; the strip shows all of them (they are qualifications and must not be hidden) but as plain text, wrapping | no pills; if more than 4, the rest wrap, never truncate |
| 3 Capability limits | what the build cannot do (Cluster not available, success-only latency unavailable, per-phase shed unavailable) | inline hatched `UNAVAILABLE` block with reason where the value would be; nav tags | stated where relevant |
| 4 Secondary metadata | ids, hashes, timestamps, file counts | mono caption line at the foot of the card | never bordered |

The only bordered marks left are: **provenance** (REAL, DERIVED, SIMULATED, REPLAYED, PREDICTED, UNAVAILABLE) and the **RETAINED stamp**. Run-class text ("Run class: VALID, narrow meaning") becomes a sentence in the card body, with the narrow-meaning clause kept verbatim. No qualification is removed.

---

## 8. Phantom Lab composition (item 6)

Page order: page head (eyebrow, h1, one sentence) > status strip (tier 2) > tabs (About, Start a run, Runs, Evidence) > content.

### 8.1 Evidence list card anatomy (PROPOSAL; see `direction_preview.html`)
Top to bottom, in the order of importance:
1. **Source line:** provenance mark (REAL with its qualifier, or SIMULATED) + RETAINED stamp. First, so the reader knows what kind of thing this is before reading a result.
2. **Title** (h3, 17 px) and **the question** in one sentence ("Does a cap on in-flight origin fetches protect the origin when it slows, and what does it cost clients?"). The question text must come from the existing experiment description strings in the Lab API/UI (lead to confirm the source; do not invent a new claim).
3. **Verdict rows** (icon + word, with the hypothesis or arm name left), each row carrying only its own DERIVED mark. Negative results use the same size and weight as positive.
4. One plain sentence for run class, including the narrow-meaning clause.
5. Action: "Open result" (primary for the newest, secondary otherwise) and the file count.
6. **Meta foot** (tier 4): id in mono, end time, "Read-only".

### 8.2 REAL vs SIMULATED vs retained
- REAL card: solid border, solid ground. SIMULATED card: **dashed border and a faint hatch ground** (`.card[data-kind=sim]` in the prototype), so the difference is visible without reading. UNAVAILABLE: dotted border, hatched well. These reuse the existing border-style grammar; no new meaning is introduced.
- **RETAINED** is a separate axis from provenance and is always shown as the stamp next to the mark. Live activity (a run in progress) uses the run-state word and the spinner, never the stamp. A retained card never moves, animates or shows "Live".
- Colour never carries the distinction alone (glyph, word, border style).

---

## 9. Connection-state design (item 7)

### 9.1 States
| State | Meaning (source: existing connection reducer and Lab capabilities response) | Header indicator | Page-level surface |
|---|---|---|---|
| Unavailable | gateway answers but Lab API/telemetry is not provided (disabled, none) | grey dot "Lab not enabled" | one `UNAVAILABLE` block where the live content would be, no red |
| Unauthorised | 401/403 | amber dot "Sign-in needed" | one compact notice with the token field (in memory only, as today) |
| Disconnected | no answer (network, CORS, process down) | red dot "Gateway unreachable" | one compact notice (below) |
| Recovering | retry in progress (the existing backoff is 1 s to 30 s, `lib/connection.js:5-6`) | amber dot "Reconnecting, next try in 8 s" (text, no pulse) | the same notice with the countdown |

### 9.2 Layout
- **One persistent indicator** (header). Clicking it opens a glass-2 popover with: gateway address (editable), Test connection, token field when relevant, last error and time, "Copy diagnostics" (a text block, shown on request).
- **One concise notice** in the page, at most 3 lines: what is unreachable, that retained evidence is still readable, and the two actions (Retry now, Change address). Troubleshooting (CORS origin hint, process check) and diagnostics sit in a disclosure. The notice does not repeat the URL if the popover shows it; it names it once.
- Keep the left edge bar, but no large red panel. Remove the unrelated intelligence paragraph from the connection notice (it stays in the status strip as a tier-2 qualification).
- On Lab, retained **evidence stays readable when the gateway is unreachable** only if the lead confirms evidence comes from the Lab API (it does today, so it would not load). In that case the notice says so; do not show cached evidence as current.
- Fix the content bug: the "address shown in the header" sentence must match the new placement.

### 9.3 Functional requirements for the lead (not UI work)
1. The gateway URL must be configurable at runtime from the UI. Today it is fixed at load from `?gateway=` / `VITE_PHANTOM_GATEWAY` (`App.jsx:56-58`). Needs: a setter in app state, re-creation of the telemetry stream and Lab client on change, validation of http/https URLs.
2. Persistence: the address is not a secret and may use `localStorage` inside try/catch; **the bearer token must stay in memory only** (existing rule, `ConnectionPanel.jsx` copy).
3. The connection reducer needs to expose `lastError`, `nextRetryAt` and `attempts` for the Recovering text.
4. "Test connection" must reuse the existing capabilities request and show its status code class (network, 401/403, 5xx) without inventing causes.

---

## 10. Scenario B and evidence information architecture (item 8)

### 10.1 First level: the answer page (PROPOSAL)
Order for a retained result:
1. Source line (mark + stamp + ended-at) and title.
2. **The question** (one sentence) and **what was measured** (one sentence: arms, workload, host).
3. **Result at a glance** (anatomy below).
4. **Not established** (always visible, same card, equal weight) and the instruction "read the binding notice before citing".
5. **Binding interpretation notice**, fully rendered and **open** (not collapsed), with Rendered / Raw / Copy text unchanged. It moves from first position to directly after the glance, which changes the order but removes and weakens nothing.
6. Cause and effect charts (section 10.3).
Second level (as today's folds): arms side by side, paired differences, mechanism check, workload and settings, limitations list, files, manifest provenance. Technical investigators land there through the sticky rail.

**Result-at-a-glance anatomy (the prototype):** a bordered header (provenance, stamp, "two separate verdicts, no combined score"); one column per arm; in each column the two quantities that matter side by side (peak origin concurrency change with its interval, client success change against its limit) as large numerals with their units; the arm's verdict as icon + word; then the full-width "Not established" strip. **There is no overall score and no overall verdict.** Verdicts per arm remain as recorded; the UI does not combine them.

### 10.2 Navigation and reading
- **Sticky contents rail** at 1280 px: a 12 rem column to the right of the content (or above on narrower screens as a horizontally scrollable bar under the header), listing the sections, highlighting the one in view (`IntersectionObserver`, no scroll listener); at 820 and 390 px it is the existing inline list made sticky under the header, with `scroll-margin-top` accounting for the header (WCAG 2.4.11: the sticky header must not hide a focused element, so `scroll-padding-top` equals the header height).
- Long-form type: measure 60 to 75 ch (`max-width: 68ch`), 16 px, leading 1.6, paragraph spacing 0.75 em, headings with 2 em space above. Numbered qualification lists keep their numbers and a hanging indent. Identifiers (`scenario_b...`, file paths, env var names) in `code` with a surface-2 well, `overflow-wrap: anywhere`. Support WCAG 1.4.12 overrides (no fixed-height text containers).

### 10.3 Chart redesign proposals
General anti-misleading rules (PROPOSAL): zero baseline for bar/length encodings; axis and unit printed; value labels direct (on the mark), exact figures always in the twin table; intervals drawn only from supplied intervals, never computed; do not connect points that are not a series; no dual axes; no smoothing; a missing value is a hatched block with the reason, never a zero; provenance mark in the chart header; the chart never colours good or bad by itself (adverse direction is shown with shape and the printed rule).

| Chart (file) | Today | Proposal |
|---|---|---|
| `TradeoffChart` (`LabCharts.jsx:76`) origin concurrency and client success | Two separate bar lists, "read together" | **Trade-off plane**: x = change in peak origin concurrency vs BASE (percent), y = change in client success vs BASE (percentage points); one labelled point per arm (direct label), a horizontal line at the registered limit (-1 pp) with its label, shaded "outside the limit" half drawn with hatch. Intervals shown only where the data supplies them. Table twin gives exact values. Below it, the existing two bar lists remain as the "exact per-arm values" figure. Needs a data-shape check with the lead (does the API give success interval?). |
| `ShedChart` | bars per arm, an unavailable block for per-phase | Keep bars (zero baseline, 0 to 100 % for a fraction); label denominators when supplied; keep the hatched `UNAVAILABLE` block for per-phase shed with its reason. |
| `LatencyChart` | three panels, nine bars | **Dot plot by percentile**: rows P50, P95, P99; one dot per arm on a shared ms axis (zero baseline, axis ticks), direct labels. Fewer marks, arms comparable in one glance. The line "These percentiles include fast-fail 503s" stays directly under the chart; success-only stays a hatched unavailable block. |
| `EffectChart` (paired differences, CI) | CI line with dot; zero line | Keep the form. Add: the zero line labelled "no difference", direction captions ("lower is better for origin load"), the pair count n, the adverse marker rule in text, value at the right in tabular figures. |
| `Sparkline` (live) | small line | Add min/max ticks and the time window in the label; break the line at gaps; REAL/DERIVED mark; never extrapolate; "no samples" is a hatched block. |
| `PercentileChart`, `CapacityChart` | bars | Capacity: single track with the budget label at the right end, zero at the left (already the rule, `pages.css:291`); percentiles as in the dot plot. |
| M6 "effect sizes" and "per scenario" | tables of verdicts | **Small multiples**: one tiny panel per scenario (8) with the same axes, so patterns across scenarios are visible; each panel labelled, table twin kept. SIMULATED dashed frame. |

---

## 11. Component set v2 (item 9), mapped to existing code

| Component | v2 spec | CSS / file |
|---|---|---|
| Buttons | primary ink-filled; secondary outlined `--line-strong`; ghost text; danger outlined red. 36 px high (44 on touch), radius 6. Press: scale .97, 90 ms. | `.btn`, `.btn--primary` in `app.css:168-175` (V1) |
| Cards vs sections | card (L1, 10 px radius, 1 px `--line`, no shadow in light, none in dark) only for an entity (experiment, run, result tile). Everything else is a section: hairline + whitespace. | `.panel`, `.lab-card`, `.lab-section--card` (V1 defines the primitive, V2 applies) |
| Tabs | text tabs with a 2 px ink underline that animates `scaleX`; 44 px target on touch. | `.tab` (V1), `.lab-subnav` (V2) |
| Forms | unchanged structure; field wells use `--surface-2`; error summary and inline errors unchanged. | `app.css:200-226` (V1), `.lab-form` (V2) |
| Status chips | retired as a shape. Tier 1 = indicator + word, tier 2 = strip text, provenance = mark, RETAINED = stamp. | `.pill` (V1 removes or restyles), `.lab-status__chips` (V2) |
| Provenance mark | radius 5, 22 px high, word + glyph + optional qualifier; solid / dashed / dotted. | `.prov` `app.css:104-116` (V1) |
| Notices | left edge bar + icon + title; role/aria unchanged; tinted ground. | `Notice` in `LabParts.jsx:55` (V2) |
| Tables | solid L1, sticky `thead` on `--surface-2`, tabular numerals, right-aligned numbers, row tint on hover. No glass ever. | `app.css:255` (V1) |
| Disclosure | chevron + title + count on one line; content fade. | `.disclosure`, `.lab-fold` (V1/V2) |
| Empty / error / unavailable | one grammar: icon + one-line title + one sentence + (reason) + action; hatched well for unavailable values. | `States.jsx` (V1), `UnavailablePanel` `LabCharts.jsx:23` (V2) |
| Floating run bar / popover / dialog | glass-2 / glass-2 / glass dialog, section 4.3. | `.form-bar`, `.popover`, `.dialog` (V1) |

---

## 12. Measurements, budgets and what they allow (items 12, Step 3)

### 12.1 Method (FACT about what was done)
- Rig: `prototypes/materials_perf.html`, one standalone page with a sticky header (56 px), a fixed 216 px rail, a sticky toolbar, a fixed floating action bar, two floating status surfaces, a dialog and scrim, and 90 text-dense panels on a gradient-and-grid background. Variant chosen by `?v=`.
  a solid; b blur 12 px on 2 surfaces (header, action bar); c blur 12 px on 6 (header, rail, toolbar, action bar, 2 floating surfaces; on 390 px the rail is hidden so c has 5); d header + full-height rail + toolbar blur 12 px; d32 as d at 32 px; e as d plus panels (every 7th) continuously animating `transform` and `opacity` beneath the glass; f six cards under `perspective: 900px` with `preserve-3d` and a tilt class toggled every 250 ms; g full-viewport scrim with 8 px blur plus a glass dialog; h **anti-pattern**: d plus blur on all 90 panels. Glass tokens in the rig: `blur(12px) saturate(1.4)`, 78 to 80 percent surface alpha.
- Driver: `prototypes/perf/run_perf.mjs` (Node 24, `ui/scripts/cdp-lib.mjs`, own Chrome with a throwaway profile under `D:/Temp`, killed afterwards: `pidKilled: true` in both JSON files). Per cell: 5 fresh page loads; in-page `requestAnimationFrame` deltas recorded while `scrollTo` advances 1800 px/s for 3 s (first 3 frames dropped). Long frames counted above 17.7 ms (one 60 Hz frame plus 1 ms tolerance) and above 34 ms (two frames). A 6th run per cell was traced (`Tracing`, categories `devtools.timeline`, `cc`, `gpu`) to sum Paint, RasterTask and Prepaint time over all threads (marked *); it is one run, includes tracing overhead and is **not** repeated.
- Viewports: 1280x800 at DPR 1; 390x844 at DPR 2 with `Emulation.setCPUThrottlingRate(4)`.
- Hardware: Windows 11 host, 24 logical processors, Chrome 156.0.8078.79. Path 1 uses `--disable-gpu` (in-page WebGL renderer string: ANGLE Microsoft Basic Render Driver). Path 2 does not (renderer: ANGLE Intel UHD Graphics 0x0000A78B). Path 2's rAF cadence was about 4.2 ms, so the host display ran near 240 Hz; the "long frame" thresholds are therefore generous on that path, so read p95 and max as well.
- Layer count came from `LayerTree` events only on the hardware path and only for some cells (n/a elsewhere; the software path returned an empty tree).

### 12.2 Results


**Path 1: software compositing (--disable-gpu; renderer reported as ANGLE Microsoft Basic Render Driver; frame cadence capped at about 60 Hz, 16.7 ms).** Chrome/156.0.8078.79, 3 s scripted scroll at 1800 px/s, 5 runs per cell (median, then [min-max]).

| Viewport | Variant | Frames in 3 s | Long frames >17.7 ms | >34 ms | p95 frame ms | Max frame ms | Paint ms* | Raster ms* | Prepaint ms* | Layers |
|---|---|---|---|---|---|---|---|---|---|---|
| 1280x800 | a solid | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 7.7 | 156.6 | 7.3 | n/a |
| 1280x800 | b blur12 x2 | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 8.2 | 160.9 | 8.2 | n/a |
| 1280x800 | c blur12 x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 9.1 | 148.9 | 9.3 | n/a |
| 1280x800 | d large (rail+hdr+toolbar) | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 11.2 | 177.8 | 10.3 | n/a |
| 1280x800 | d32 large, blur 32 | 179 | 0 [0-2] | 0 | 16.7 | 16.8 [16.8-33.4] | 8.2 | 131.6 | 7.7 | n/a |
| 1280x800 | e = d + animation under glass | 178 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 6.4 | 146.8 | 65.2 | n/a |
| 1280x800 | f 3D tilt x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 244.8 | 2405.4 | 87.9 | n/a |
| 1280x800 | g blurred scrim + glass dialog | 173 | 7 [0-11] | 0 | 16.8 | 33.4 [16.8-33.4] | 8.6 | 161.4 | 7.2 | n/a |
| 1280x800 | h ANTI: blur on 90 panels | 150 | 29 [3-32] | 0 | 33.4 | 33.4 [33.4-49.9] | 1.7 | 200.4 | 6.6 | n/a |
| 390x844, 4x CPU | a solid | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 15.5 | 106.7 | 24 | n/a |
| 390x844, 4x CPU | b blur12 x2 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 15.2 | 59.6 | 29.7 | n/a |
| 390x844, 4x CPU | c blur12 x6 | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 13.9 | 74.3 | 30.6 | n/a |
| 390x844, 4x CPU | d large (rail+hdr+toolbar) | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 20.8 | 50.9 | 50.8 | n/a |
| 390x844, 4x CPU | d32 large, blur 32 | 179 | 0 [0-0] | 0 | 16.8 | 16.8 [16.8-16.8] | 25.3 | 50.2 | 53.9 | n/a |
| 390x844, 4x CPU | e = d + animation under glass | 178 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 19.6 | 54.3 | 488.6 | n/a |
| 390x844, 4x CPU | f 3D tilt x6 | 159 | 20 [7-34] | 2 | 33.3 | 50.1 [49.9-66.6] | 1435.6 | 1025.1 | 384.7 | n/a |
| 390x844, 4x CPU | g blurred scrim + glass dialog | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 12.7 | 61.8 | 27.4 | n/a |
| 390x844, 4x CPU | h ANTI: blur on 90 panels | 179 | 0 [0-0] | 0 | 16.7 | 16.8 [16.8-16.8] | 5.5 | 72.7 | 28.1 | n/a |

**Path 2: hardware (no --disable-gpu; renderer reported as ANGLE Intel UHD Graphics 0x0000A78B; frame cadence about 240 Hz, 4.2 ms).** Chrome/156.0.8078.79, 3 s scripted scroll at 1800 px/s, 5 runs per cell (median, then [min-max]).

| Viewport | Variant | Frames in 3 s | Long frames >17.7 ms | >34 ms | p95 frame ms | Max frame ms | Paint ms* | Raster ms* | Prepaint ms* | Layers |
|---|---|---|---|---|---|---|---|---|---|---|
| 1280x800 | a solid | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.2] | 9.1 | 2.5 | 21.4 | n/a |
| 1280x800 | b blur12 x2 | 719 | 0 [0-0] | 0 | 4.3 | 4.9 [4.4-12.5] | 6.7 | 1.9 | 21.5 | 10 |
| 1280x800 | c blur12 x6 | 718 | 0 [0-1] | 0 | 4.3 | 4.4 [4.4-24.9] | 7 | 2.1 | 24.1 | n/a |
| 1280x800 | d large (rail+hdr+toolbar) | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-4.7] | 6.6 | 1.8 | 18.8 | 10 |
| 1280x800 | d32 large, blur 32 | 718 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.4] | 7.4 | 2 | 24 | 10 |
| 1280x800 | e = d + animation under glass | 718 | 0 [0-0] | 0 | 4.3 | 4.4 [4.4-4.5] | 8.4 | 3.3 | 287.7 | 23 |
| 1280x800 | f 3D tilt x6 | 717 | 0 [0-2] | 0 | 4.3 | 8.2 [4.9-20.9] | 792.4 | 28 | 225.7 | 64 |
| 1280x800 | g blurred scrim + glass dialog | 718 | 0 [0-5] | 0 | 4.3 | 4.8 [4.5-25.2] | 12 | 3 | 27.9 | n/a |
| 1280x800 | h ANTI: blur on 90 panels | 718 | 0 [0-1] | 0 | 4.3 | 4.6 [4.5-29.2] | 1.9 | 2.5 | 24.8 | n/a |
| 390x844, 4x CPU | a solid | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.3] | 13.9 | 1.3 | 71.8 | n/a |
| 390x844, 4x CPU | b blur12 x2 | 718 | 0 [0-1] | 0 | 4.3 | 8.2 [4.5-20.8] | 18.8 | 1.4 | 124.7 | n/a |
| 390x844, 4x CPU | c blur12 x6 | 719 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.4] | 16.6 | 1.2 | 129.7 | 11 |
| 390x844, 4x CPU | d large (rail+hdr+toolbar) | 718 | 0 [0-0] | 0 | 4.3 | 4.5 [4.4-8.3] | 17.6 | 1.3 | 111.8 | 10 |
| 390x844, 4x CPU | d32 large, blur 32 | 719 | 0 [0-0] | 0 | 4.3 | 4.4 [4.3-5.9] | 18.5 | 1.6 | 134 | 10 |
| 390x844, 4x CPU | e = d + animation under glass | 680 | 0 [0-1] | 0 | 4.4 | 12.6 [4.4-20.8] | 19.2 | 1.6 | 1061.2 | n/a |
| 390x844, 4x CPU | f 3D tilt x6 | 295 | 29 [16-40] | 2 | 25 | 41.6 [33.5-58.5] | 1546.2 | 16.3 | 499.4 | n/a |
| 390x844, 4x CPU | g blurred scrim + glass dialog | 705 | 1 [0-1] | 0 | 4.3 | 20.8 [16.6-20.9] | 14.9 | 1.5 | 104.2 | n/a |
| 390x844, 4x CPU | h ANTI: blur on 90 panels | 718 | 0 [0-2] | 0 | 4.3 | 4.5 [4.4-29.2] | 6.5 | 2.4 | 164.3 | n/a |

\* Paint, Raster and Prepaint: milliseconds summed over all threads during one traced 3 s scroll; compare variants, do not read as per-frame cost.

### 12.3 Reading the table
1. **Backdrop blur on a few chrome surfaces costs almost nothing here.** Variants b to d32 match the solid baseline in frame count and long frames on both paths and both viewports. Raster time does not rise on the software path (a 157 ms, d 178 ms desktop).
2. **Area and count of blurred data matter.** h (90 blurred panels) cuts the software-path frame rate at 1280 px (150 of 179 frames, 29 long frames). The same page at 390 px did not (small area). g shows that one full-viewport blur is not free on the software path.
3. **3D is the expensive one.** f multiplies Paint by roughly 30x (software, desktop) to 100x (hardware, desktop), grows the layer count (hardware path, desktop: 64 against 10), and is the only variant that dropped frames on the mobile-emulated hardware path.
4. **Animation under glass shifts work to the main thread** (Prepaint), which CPU throttling slows; the mobile e row shows the first sign on the hardware path (max frame 12.6 ms median, 20.8 ms worst, 680 frames against 719).
5. Frame counts of about 179 (software) and 719 (hardware) in 3 s are the cadence ceiling of each path, so any lower figure is lost frames.

### 12.4 Budgets and limits derived (PROPOSAL, based on the above and kept conservative)
| Item | Limit | Basis |
|---|---|---|
| Glass surfaces at once, desktop | 4 (measured clean up to 6) | b, c, d, e |
| Glass surfaces at once, phone-class viewport | 2 (header plus at most one floating surface); the menu sheet and the run bar are solid on phones | measured clean at 5 in emulation, but no real phone GPU was tested, so 2 is a safety margin |
| Blur radius | 14 px maximum (32 px measured clean apart from one outlier run; no design need) | d, d32 |
| Which surfaces | header, rail, run bar, popovers, dialog | design rule |
| Never | data panels, tables, charts, cards, evidence text | h; legibility; integrity |
| Scrim | dim only, no blur | g |
| Content animating beneath glass | no continuous animation; single transitions only | e Prepaint |
| 3D transforms, perspective, preserve-3d | none | f |
| Concurrent animations on a phone-class viewport | at most 2 | e, f |
| Layout shift | glass never changes box size; header and run bar are fixed height; reserve scroll padding | design rule (UNVERIFIED by measurement: CLS was not measured) |

### 12.5 Honest limits
- Headless Chrome, **one Windows host with an integrated Intel GPU**; results can differ on a discrete GPU, Apple silicon, Android GPUs and in Safari or Firefox (both implement `backdrop-filter` differently; not tested).
- 390 px "mobile" is viewport + DPR 2 + 4x main-thread throttling. Throttling does not slow the compositor or GPU, so phone GPU fill-rate and tile-memory limits are **not measured**.
- Scroll is driven by `scrollTo` inside `requestAnimationFrame` (main thread). Real touch scrolling is often compositor-driven and cheaper; real flings with a changing backdrop could differ.
- Synthetic page: 90 identical panels, no images, no live updates. The real app has more DOM and live text updates.
- Layer count is partial; CLS, input latency, battery, thermal state and memory were not measured. The traced cells are single runs.
- A real-device check would add: a mid-range Android phone (Chrome) and an iPhone (Safari) opening the same rig by URL with remote debugging, a timeline capture per device, battery and temperature over 5 minutes of scrolling, and a screen-recording frame count.
- Contrast was computed from numbers (`perf/contrast_worst_case.json`, `perf/palette_contrast.json`), not sampled from rendered pixels. The preview screenshots (`prototypes/shots/`) were taken at 1280 light/dark, 820 light, 390 light/dark, a disconnected state, and one run with `prefers-reduced-transparency` emulated (it looked identical to the glass version in that capture, so **the media-query fallback was not visually confirmed**; Chrome emulation support for that feature was not verified).

---

## 13. Responsive plan (item 10)

| Width | Shell | Lab / evidence | Charts |
|---|---|---|---|
| **1280** | glass header, sticky glass rail, floating run bar, sticky contents rail to the right of long pages | two-column experiment cards; result at a glance as a row of arm columns | trade-off plane and dot plot at full width, table twin switch |
| **820** | header + rail kept (rail 232 px leaves about 588 px) | one-column cards or two narrow; contents as a sticky bar under the header; arm columns wrap to a row of two | plots keep axes; labels move above marks |
| **390** | header only glass surface; **solid** menu sheet; solid sticky run bar | single column; status strip becomes 2 lines + "All qualifications"; the connection notice is a compact top banner, never above all content | one panel per screen; plots become horizontal-only; value labels under the mark; table twin default for dense charts |

Each is designed: for example at 390 the result-at-a-glance arm columns stack with the numeral first and a hairline between them (prototype `@media (max-width: 819px)`), and the dumbbell track drops its axis ticks in favour of the printed values.

---

## 14. Dark theme as its own design (item 11)
- Surface ladder by lightness steps, not shadows: `#0b0e13` page, `#12161d` L1, `#181d26` wells, `#1f2531` selected. Light-on-dark uses `#eef1f6` (not pure white) to cut glare.
- Borders are the main separator: `--line #262d3a` for structure, `--line-strong #8591a6` for control boundaries (5.7:1 on surface, computed).
- Glass in dark is darker and **lower-edged**: tinted `rgb(18 22 29 / .88)`, a 9 percent white top highlight, no coloured glow, no neon. Shadows are black at 45 percent only for floating layers.
- Chart colours: dark uses the lighter hue step of each provenance colour (as today) on `--surface`; the dot and interval colours were checked on surface (derived on surface 9.63:1). Hatched grounds use `--line` at low opacity.
- Status hues are the lighter variants (`#7fdba2`, `#8fc0ff`, `#ffc27a`, `#ff8f86`), tested against their backgrounds (lowest 6.76:1).
- Theme switching: switch `color-scheme`; fade page colours 140 ms only when the user toggles (avoid a brightness jump, skill S14), never on load.

---

## 15. Accessibility and performance budgets (item 12)

Accessibility budget (PROPOSAL; checkable):
- Text contrast 4.5:1 and UI 3:1 in both themes, including glass at the **worst-case backdrop** (section 4.3, guarantee 5). Focus ring 2 px, 3:1, never hidden under sticky chrome (WCAG 2.4.11; `scroll-padding-top` = header height; `scroll-padding-bottom` = run bar height).
- Target size 24 px minimum (project rule), 32 px desktop primary, 44 px touch (as today).
- All three preference fallbacks (transparency, contrast, forced colors) verified by a screenshot each.
- Layout survives 1.4.12 text spacing overrides and 200 percent zoom without horizontal scroll at 390 px.
- No information carried by colour or by glass alone.

Performance budget: see the limits derived in section 12. Summary form: at most 2 glass surfaces on a phone-class viewport (header + one), at most 4 on desktop, radius at most 14 px, never glass on data panels, no 3D transforms, at most 2 concurrent animations on mobile, no animation of blur radius.

---

## 16. Implementation specification for two writers (item 13)

**Dependency order:** V1 first (tokens), V2 consumes tokens. V2 may start on structure and use the token names from section 4 once V1 has merged `tokens.css`. The lead owns functional work in section 9.3 and `App.jsx` / `lib/*`.

| Writer | Owns (exclusive) | Does NOT touch |
|---|---|---|
| **V1: foundations v2** | `ui/src/styles/tokens.css`, `app.css`, `shell.css`, `pages.css`; `components/Shell.jsx`, `Panel.jsx`, `Metric.jsx`, `States.jsx`, `Icon.jsx`, `ProvenanceBadge.jsx` (visual only, props unchanged); `scripts/check-contrast.mjs` (add glass worst-case pairs) | `lab.css`, `sim.css`, screens, Lab components, `lib/*`, `App.jsx` |
| **V2: Lab, evidence, explainer** | `ui/src/styles/lab.css`, `sim.css`; `screens/lab/*`; `components/LabParts.jsx`, `LabCharts.jsx`, `Charts.jsx`, `ChartFrame.jsx`; `screens/Simulation.jsx`, `Placeholder.jsx` and other non-Lab screens' markup | tokens, shell, `lib/*` |

### V1 steps and acceptance
1. **Tokens.** Replace the neutral ladder, primary colour, add material, motion and type tokens (sections 4, 6). Keep every existing token name (deprecated aliases stay) so V2 and old screens keep working. *Accept:* `npm run check:contrast` passes with the new values plus the glass worst-case pairs; no existing screen loses contrast (existing tests green).
2. **Materials.** Add `.glass-1`, `.glass-2` utility classes with the fallback ladder; apply to header, rail, `.form-bar`, `.popover`, `.dialog`. *Accept:* a screenshot in each of default, reduced-transparency, prefers-contrast and forced-colors; no `backdrop-filter` on any element inside `main`; a test (or `check-forbidden`) fails if `backdrop-filter` appears in `lab.css`, `pages.css`, or on `.panel`, `table`, `.lab-card`.
3. **Shell.** Sticky header, ghost mark, connection indicator with popover, theme menu, grouped rail with collapsed "Not available yet", gateway block, phone solid sheet. *Accept:* keyboard order and focus not obscured (axe/CDP check), arrow-key nav retained, 390/820/1280 screenshots, header height 56 px.
4. **Components.** Buttons ink-primary, retire `.pill` shape, provenance mark and RETAINED stamp styles, tabs underline, notices. *Accept:* no `border-radius: 999px` remains; at most 3 bordered marks in any card in the screenshot set.
5. **Motion.** Add tokens and patterns, reduced-motion equivalents. *Accept:* CDP check with `prefers-reduced-motion` emulated shows no transform animation; no `transition: all`; no animated `box-shadow`, `width`, `height` or `backdrop-filter`.

### V2 steps and acceptance
1. **Lab head and status strip** (tier 2 text strip; "All qualifications" disclosure keeps every qualification). *Accept:* the qualification strings present in the current screenshots are all still in the DOM (a test lists them).
2. **Experiment card anatomy** (section 8.1) and REAL / SIMULATED / RETAINED treatment. *Accept:* at 1280, each card has at most 3 bordered marks; card shows question, verdict rows, meta foot.
3. **Connection notice** (section 9) and ConnectionPanel simplification; needs the lead's functional items 1, 3, 4. *Accept:* gateway address appears once in the page body; no text refers to "the header" unless it is true; the disconnected first screen at 390 shows Lab content below a notice of 3 lines or fewer.
4. **Scenario B and M6 pages**: reorder per section 10.1, sticky contents rail, long-form type. *Accept:* "Result at a glance" starts within the first 900 px at 1280 and 1400 px at 390; the binding notice is fully rendered and open; "Not established" visible without any click; no overall score element exists in the DOM.
5. **Charts** per section 10.3 (trade-off plane, latency dot plot, effect chart additions, sparkline ticks, M6 small multiples). *Accept:* every chart has axis, unit, provenance mark, direct labels, a table twin with identical values (existing test style), the unavailable block for missing values; a test asserts that plotted values equal table values and that no value is computed in the chart code beyond layout.
6. **Simulation and placeholder illustrations** (section 5) as static SVG with "ILLUSTRATION: not live" labels. *Accept:* no state colours, no numbers, `role="img"` text alternative present.
7. **Visual checks required (both writers):** screenshots at 390, 820, 1280, light and dark, for: Lab About, Evidence list, Scenario B full page, M6, run detail running and completed, cancel dialog, gateway-stopped Lab and Overview, Overview, Cache, Cluster, Intelligence; plus the four fallback modes on the Lab page. Compare with `docs/design/remediation/after/`.
8. **Performance gate:** re-run `prototypes/perf/run_perf.mjs`-style measurement on the **real** built pages (scripted scroll of Scenario B at 1280 and 390 with 4x throttle) and record long-frame counts; budget: no more than the measured-solid baseline plus 2 long frames over 3 s on the software path.

---

## 17. What NOT to do (item 14)
1. **No glass on data**: tables, charts, verdict rows, notices, cards. Legibility, contrast proof and integrity.
2. **No blurred backdrop on data-dense full-width panels or on many small surfaces** (rig variant h degrades even in the software path, section 12).
3. **No CSS 3D, tilt hover, parallax or WebGL.** No explanatory value for dashboards of text and numbers; extra layers; reduced-motion burden.
4. **No decorative live motion** (pulsing dots, drifting backgrounds, animated packets). Motion only for real state.
5. **No animation of blur radius, box-shadow, width or height.** Cost is unverified or known-expensive.
6. **No neon, no glow, no gradient fills on data marks.** The owner asked to avoid cyberpunk; gradients also break the contrast guarantee.
7. **No single overall score, "health" grade, or colour-coded success for experiments.** Verdicts stay distinct, as recorded.
8. **No removal or shortening of qualifications, limitations or provenance to look cleaner.** Reduce their visual noise, not their text.
9. **No new dependencies** (animation libraries, icon packs, font files, 3D engines) without the owner's authorisation.
10. **No cached or placeholder numbers shown while disconnected.** The disconnected design shows less, not more.

---

## 18. Limits of this research (item 15)
- **HIG pages were not read** (only titles came back). HIG statements are UNVERIFIED.
- WCAG 2.2: first 100,000 of about 270,000 characters read; 2.5.8 not read.
- Performance was measured in **headless Chrome on one Windows 11 host**; see section 12 limits (software or desktop GPU, 60 Hz cap, CPU throttling affects only the main thread, no phone GPU, no LayerTree data, no Safari or Firefox). The rig is synthetic (90 identical panels), not the real app.
- The palette and glass values were checked by computed contrast and by eye in the prototype, not in the real app; real screens may show pairs the token list does not cover.
- The prototype is static; it shows layout and materials, not behaviour, and its numbers are copied from the retained Scenario B screenshot for layout testing only (they are not new data).
- Chart proposals need data-shape confirmation (for example whether the API supplies a client-success interval).
- No user testing was done.
