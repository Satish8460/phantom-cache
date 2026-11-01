# UI/UX audit and remediation plan (agent AUDIT)

**Status:** Plan, not implementation. **Date:** 2026-10-30. **Writer:** agent AUDIT (single writer of this file; read-only on everything else). **Nothing here is approved.** O8-2 stays open until an independent review (see `00_progress.md`).

Evidence tags used: **V** verified by reading the file (file:line), **S** observed in a screenshot I opened (path given), **C** computed by me from values in the repo, **H** hypothesis (cause not verified), **R** reported by a prior agent or document (not re-verified), **U** not verified.

## 0. How the skill was used (read this first)

- The Skill tool was tried once with a concrete question ("Which section governs type scale and elevation for a dense engineering UI?", skill `apple-design`). Result: **`Unknown skill: apple-design`**. The skill is installed in the repo (`.claude/skills/apple-design/`) but this running session does not list it (consistent with `00_progress.md` section 1: needs a restart).
- Fallback, stated plainly: **I read the installed file `.claude/skills/apple-design/SKILL.md` in full (291 lines) and `docs/design/apple_design_skill_project_rules.md`. I did not invoke the skill.** Section numbers below are the skill's own headings: S1 Response, S2 Direct manipulation, S3 Interruptibility, S4 Behavior/springs, S5 Velocity handoff, S6 Momentum, S7 Spatial consistency, S8 Hinting, S9 Rubber-banding, S10 Gesture details, S11 Frame smoothness, S12 Materials and depth, S13 Multimodal feedback, S14 Reduced motion and accessibility, S15 Typography, S16 Design foundations (eight principles plus tactical rules), S17 Process.
- Project overrides applied throughout (`apple_design_skill_project_rules.md` sections 3 and 4): **no translucent glass/blur by default (S12 overridden; only solid surfaces and an optional dimming scrim)**; no spring libraries or other new dependency (S4-S6, S9, S10, S13 skipped; CSS easing only); **provenance labels are never hidden or played down**; negative results keep equal visual weight; dense technical information stays dense (progressive disclosure, not removal); no backend change.
- Intent: a precise, restrained engineering product. This plan does not propose more cards, more shadow or larger headings as a goal. Where it adds hierarchy it does so by removing boxes and redundancy, and by giving type, spacing and a small number of surface levels a deliberate job (S15, S16 Craft, Simplicity).

## 1. What I inspected

Source read: `ui/index.html`; `ui/src/styles/tokens.css`, `app.css`, `lab.css` (all in full); `App.jsx`, `components/{Shell,States,Panel,Metric,ProvenanceBadge,LabParts,Icon,Charts}.jsx`, `components/LabCharts.jsx` (first 80 lines); `screens/{Simulation,Placeholder,Overview,ScreenFrame,Traffic}.jsx`; `screens/lab/{LabScreen,ConnectionPanel,RunForm,RunsViews,StoryView,EvidenceViews,ScenarioBResult,Gate}.jsx`; `SimDemo.jsx` (all 517 lines); `lib/routes.js` (first 88 lines). **Not read in full:** `screens/{CacheMemory,Freshness,Origin}.jsx` (judged from screenshots only), `lib/*` other than routes, hooks, `LabCharts.jsx` after line 80. Docs: `docs/design/{tokens,principles,ui_audit,phantom_lab_ui}.md` partially (grep and first lines; **not** read in full), `information_architecture.md` and the others **not read**.

Screenshots opened (26; all existing, none from the B0 gallery because `docs/design/remediation/before/` did not exist when I looked). Base `docs/demo/browser_verification/`:
1. `20261030T045605Z/shots/matrix/desktop_light__lab.png` (1920 wide, light)
2. `.../matrix/desktop_dark__lab.png` (dark)
3. `.../matrix/desktop_light__lab_run.png`
4. `.../matrix/desktop_dark__lab_run.png`
5. `.../matrix/desktop_light__simulation.png`
6. `.../matrix/desktop_dark__simulation.png`
7. `.../matrix/desktop_light__.png` (Overview)
8. `.../matrix/desktop_dark__traffic.png`
9. `.../matrix/desktop_light__lab_evidence_scenario_b_20261030T003932Z.png`
10. `.../matrix/desktop_dark__lab_evidence_m6_sealed_2026_10_29T234845Z.png`
11. `.../matrix/desktop_light__lab_runs.png`
12. `.../matrix/desktop_light__lab_evidence.png`
13. `.../matrix/desktop_dark__cache.png`
14. `.../journey/r2_anon_04_run1_started.png` (1280)
15. `.../journey/r2_anon_run1_final_full.png` (1265x6714 full page)
16. `.../journey/r2_anon_02_run_form_invalid.png`
17. `.../journey/r2_anon_14_run2_cancel_confirm.png`
18. `.../forced_colors_lab_run.png`
19. `.../r2layout/reflow_820_lab_run.png` (820)
20. `.../r2layout/menu_open_390.png` (390)
21. `.../top/top_mobile_dark_lab.png` (390, dark)
22. `20261030T055008Z/shots/r3layout/reflow_390_lab_run.png`
23. `.../r3layout/reflow_390_simulation.png`
24. `.../r3layout/reflow_390_cluster.png`
25. `.../r3layout/reflow_390_origin.png`
26. `.../r3clips/start_form_1280.png`

Coverage: light and dark at 1920 (labelled "desktop"; **1280 is covered only by 14, 17, 18, 26 and the light/dark pair is only at 1920 and 390**), 820 (one image), 390 (six). Pages never seen by me: Freshness, Lab runs at 390, Lab Scenario B detail in dark. Large full-page images were downscaled and are unreliable for detail (images 15, 22, 19, 21); I used them for height and structure only.

## 2. What the existing tests and browser rounds establish (and do not)

Facts: `ui/test/*.mjs` has 17 files; counting `test(` occurrences gives about 218 (the handoff says 212; I did not run them, so **R** for the number). From reading `a11y.test.mjs`, `forbidden.test.mjs`, `check-contrast.mjs`, `a11y-lint.mjs` use:

| Layer | What it asserts | Cannot establish |
|---|---|---|
| Render tests (`render.test.mjs`, `lab.render.test.mjs`) | Server-side React string output contains the expected words, `data-*` hooks, roles, provenance badge present, error text, no `NaN` | Any CSS result: spacing, hierarchy, overflow, colour, density, visual balance. Output is a string, never laid out. |
| A11y lint (`a11y.test.mjs`, `scripts/a11y-lint.mjs`) | Single h1, heading order, accessible names, unique ids, table captions, no positive tabindex; **excludes the `simulation` screen** (a11y.test.mjs line 34 filter) | Real focus order, contrast in context, target size, reading experience. Does not check the Simulation screen at all. |
| Contrast script (`check-contrast.mjs`) | WCAG ratios for listed token pairs, both themes, parsed from `tokens.css` hex values | Non-text contrast of badge borders and chart marks, text over composite backgrounds, anything not in the PAIRS list, light-dark() or non-hex values (parser matches `#rrggbb` only). |
| Forbidden-pattern scan (`forbidden.test.mjs`) | No fixed example numbers, no `Math.random`, no client metric arithmetic, no hex colours outside tokens, no extra dependencies, no `fetch` outside the allowed files; **SimDemo is explicitly exempted** (lines 56-58 and 71) | Anything about design quality. The exemption is why SimDemo can contain `$0.15`, `8000`, hex colours and its own network calls. |
| View-model tests | Mapping of payloads to labelled cells, provenance and unavailable semantics | Presentation. |
| Browser rounds (`browser-*.mjs`, reports in `docs/demo/browser_verification/`) | R: no horizontal overflow at 320/390/640/820 (with a 14 px exception N-6), keyboard order, zoom 200/400, forced colors, flows, cancel/restart, token roles | The round-3 report itself lists "not verified": mobile emulation, real zoom, Safari/Firefox, screen readers, **non-text contrast**. Screenshots were opened for defects, not scored for visual quality; about 70 PNGs were "measured only". |

Conclusion: the suites establish truthfulness, structure, reflow and keyboard reachability. They establish **nothing about visual hierarchy, typographic quality, density, consistency of the state language, or comprehension**. Every item that depends on those is a **human-review item** in sections 5 and 6. Static tests added by this plan are guard rails (tokens exist, no raw markup, states present); they must never be reported as design validation.

## 3. Findings

Severity scale: **Blocking coherence** (the product reads as two or more unrelated products, or a primary task is obstructed or a non-negotiable is violated), **High**, **Medium**, **Low**. "Skill" cites the section that justifies the finding.

### 3.1 A. Design system

| ID | Area | Description | Evidence | Sev | Skill | User impact |
|---|---|---|---|---|---|---|
| A-01 | Type scale | Six steps (12/13/15/18/22/24 px); `--text-lg` unused. Page h1 is 22 px, panel h2 15 px, h3 13 px grey. Body is 15 px but most UI runs at 13 px (`--text-sm`) and 12 px. Hierarchy is carried almost only by weight; the page title and a panel title differ by 7 px. No display or headline tier. h4 (`.lab-subhead`) has no base style. | tokens.css:9-14; app.css:17-20; S7 `desktop_light__.png` | High | S15, S16 Craft | Every page looks the same weight; the eye has no entry point. Long Lab pages (6.7k px) are a wall of equal-weight headings (image 15). |
| A-02 | Tracking and leading | One tracking token (`--tracking-label`, positive, for uppercase). No negative tracking for large type, no size-specific leading except `--leading-tight` 1.25 / `--leading-body` 1.5 applied by element, not by tier. Tables and dense lists use body leading 1.5. | tokens.css:15-17; app.css:13,17 | Medium | S15 | Large numerals and headings look loose; dense tables are taller than needed. |
| A-03 | Metric type | `.metric__value` is 24 px semibold with no tracking and no distinct metric tier; unavailable values drop to 15 px. A tile is `surface-2` inside a `surface` panel (box in box). | app.css:97-104; S7 | Medium | S15, S16 Simplicity | Numbers are not the loudest thing on Overview; three UNAVAILABLE tiles carry the same box weight as the real values. |
| A-04 | Display type | No place defines a display or "answer" size. Pages have no single focal element (for example run result verdict, or gateway health). | S3, S7, S15 | Medium | S15, S16 Simplicity | Users must read everything to find the outcome. |
| A-05 | Surfaces and elevation | `--shadow-1/-2` are defined and **unused** (tokens.md:49 says so, and grep of `ui/src` finds no `var(--shadow`). All layering is a 1 px border; panels contain cards contain chips (Lab Overview: Panel > `.lab-card` > chips; Connection: Panel > two banners). Only two surface levels (`surface`, `surface-2`). | tokens.css:38-40; lab.css:25; S1, S9 | Medium | S12 (solid only), S16 Simplicity | Nesting dilutes hierarchy; nothing signals "this layer is above that one", so future overlays (drawer, dialog) have no defined look. |
| A-06 | Radii and borders | Radii 4/6/8 are applied inconsistently (buttons 6, panels 8, tags 4, pills 999). Borders: 1 px solid everywhere, plus badge variants dashed/dotted/**double 3 px** and banner **double 4 px**. | app.css:91,109-115,130,172 | Low | S16 Craft | Mostly fine; the double borders are visually heavy relative to everything else (images 1, 5). |
| A-07 | Motion | Tokens `--dur-fast/-base`, `--ease-out` exist; only used for background-colour transitions on `.btn` and nav links. **No pointer-down press feedback** (`.btn:active` changes colour only). No transitions on disclosure, banners appearing, progress, tab change. `--dur-base` unused. Loading icon is static (not animated). | app.css:65,139-141; Icon.jsx; tokens.css:42-45 | Medium | S1, S4, S14 | Controls feel inert; a stalled load and a slow load look identical. |
| A-08 | Dark theme duplication | The 40-variable dark palette is written twice (media query block and `[data-theme="dark"]`). Drift risk; `check-contrast.mjs` has to parse both. | tokens.css:97-137 vs 139-177; check-contrast.mjs:26-28 | Low | S16 Craft | None today; a latent inconsistency between OS-dark and manual-dark. |
| A-09 | Unused tokens | `--shadow-1`, `--shadow-2`, `--text-lg`, `--space-7`, `--dur-base` (and `--ease-out` only in two rules) have no consumer. | grep results | Low | S16 Craft | Dead design vocabulary misleads implementers. |
| A-10 | Provenance badge system | Five variants distinguished by glyph, word, colour **and** four border styles (solid, dashed, **double 3px**, dotted); 5 colour pairs. The Lab adds eight tier labels (`MEASURED: LOOPBACK`, `RETAINED: SIMULATED`, ...) that mix two axes (where the number came from, and whether it was retained from an earlier run). Badges appear on every metric tile, every table row and every card. Legend lists six tiers (`StoryView.jsx:109`). Wide text-transform uppercase badges at 12 px are the loudest element on many screens (images 1, 7, 8, 12). | app.css:109-115; ProvenanceBadge.jsx; labProvenance.js (via LabParts.jsx:8-10) | High | S16 Simplicity, Craft | Labels are legible but compete with the values they qualify; two orthogonal ideas share one badge. Must be simplified without hiding any label (project rule). |
| A-11 | Provenance vocabulary | Project vocabulary is REAL / SIMULATED / REPLAYED / PREDICTED / DERIVED / UNAVAILABLE (progress doc, project rules section 4). UI vocabulary is MEASURED / DERIVED / INFERRED / SIMULATED / UNAVAILABLE plus tier labels. No design token or glyph exists for REPLAYED or PREDICTED. | tokens.css:82-91; Icon.jsx SHAPES | High | S16 Familiarity (consistency) | A reviewer checking "can REAL/REPLAYED/PREDICTED be told apart" cannot, because two of the six do not exist in the UI yet. Needs a lead decision (section 7). |
| A-12 | State grammar | The same visual device means different things: **dashed border** = inferred badge (app.css:113), empty/unavailable state block (app.css:133), disabled button (lab.css:16), neutral tag (lab.css:30); **dotted** = unavailable badge and unavailable panel; **double** = simulated. Four tone systems coexist: `.tone-*` pills, `.banner`, `.metric--critical`, `.state-block`. Stale is a dotted underline on a value (app.css:104). | as cited | Blocking coherence | S16 Familiarity, Grouping | Users cannot learn one rule ("dashed = ...") because there are four. Disabled, unavailable and inferred look related though unrelated. |
| A-13 | Loading/progress | Loading is a static arc icon plus text; native `<progress>` is styled only by width, height, `accent-color` and is not drawn at all when the total is unknown. | States.jsx:8-21; Icon.jsx loading; lab.css:68; RunsViews.jsx:113-115 | Medium | S1, S16 feedback (status) | No sign of activity for unknown-total runs; OS-dependent progress look (image 17: empty grey bar at "0 of 4"). |
| A-14 | Focus and keyboard | Strong: 2 px `:focus-visible`, skip link, roving nav, Esc closes menu and cancel confirm. One cosmetic issue: selected radio plus focus ring gives a double ring (image 3). | app.css:26-30; Shell.jsx; RunsViews.jsx:77-107 | Low (positive overall) | S14 | Keep. |
| A-15 | Responsive | Side nav appears at 960 px; at 820 px (tablet) users get the phone layout (single column, nav behind a Menu button) although 820 px is wide enough (image 19). Lab run form two-column begins at 1100 px. Mobile header uses about 190 px for three stacked rows before content (images 20, 22, 25). | app.css:73; lab.css:49; S19/S20 | Medium | S16 Flexibility | Wasted viewport on phones; tablets lose persistent navigation. |
| A-16 | Page titles | `document.title` is never set (no occurrence in `ui/src`); every route shows "PhantomCache Control Center" in the tab. | grep; index.html:8 | Medium | S16 wayfinding | Tabs, history and screen-reader page announcements cannot distinguish pages or a specific run. |
| A-17 | Light/dark/forced colours | Contrast pairs pass by script (R); both themes look coherent in the images; forced-colors mode renders sensibly (image 18). Gap: the sidebar surface ends where the list ends, leaving a visibly different canvas below it at 1920 (images 1, 3, 7). | S 1920 shots; app.css:76-80 | Low | S16 Craft | Slightly unfinished edge. |

### 3.2 B. Information architecture

| ID | Area | Description | Evidence | Sev | Skill | User impact |
|---|---|---|---|---|---|---|
| B-01 | Navigation | Nine primary items in four groups: Live gateway (5), Phantom Lab (1), **Not yet available (2: Cluster, Intelligence)**, Simulated (1). Two of nine items are dead ends and take the same weight as live pages. The Simulation demo sits in primary nav with the warning triangle glyph. | routes.js:6-16; Shell.jsx | Medium | S16 Purpose, wayfinding, specific labels | Users spend attention on destinations that cannot answer anything; group label "Not yet available" is honest but unnecessary primary nav. |
| B-02 | Lab sub-navigation | Tabs: Overview, Evidence, Start a run, Runs. "Overview" duplicates the app-level "Overview"; primary action ("Start a run") is third and has no call to action anywhere on the Lab landing page. | LabScreen.jsx:52-63; routes.js LAB_VIEWS | High | S16 Agency, specific labels, wayfinding | Main workflow is discoverable only if the user reads the tab strip; landing content is a 1,900 px essay (image 1). |
| B-03 | Overview page | Header says "Is the gateway healthy right now?" but shows six equal tiles (three UNAVAILABLE in the captured state) and a 600 px "Other reported fields" table of raw dotted field names (image 7). No grouping by availability. (`principles.md` rule: UI computes nothing; grouping by state is not a computed metric.) | Overview.jsx:66-114; image 7 | Medium | S16 Simplicity (common path first, advanced one level deeper) | The answer to the page's own question is not stated; diagnostic dump sits below it. |
| B-04 | Consistency page to page | Non-Lab pages use `ScreenFrame` + `Panel` + `MetricGrid`; the Lab uses `Panel`, `lab-card`, `lab-fieldset`, `lab-figure-block`, `table-wrap`; the Simulation page uses none of them. Notice vs banner vs state-block vs pill are chosen per file. | components; screens | High | S16 Familiarity | The same fact (for example "unavailable") looks different on Overview (tile), Traffic (bars with the word), Lab (panel with dotted border). |
| B-05 | Concepts for newcomers | Technical terms (AIMD, BCa, paired, BASE, STATIC-CAP, T1/T2, M6) appear with one-line hints at best; the story page explains the problem well but is separate from the run form. 19 arm IDs (BL-0 ... CAND-ADM) in the T1 form have no per-item description in the screenshot (images 22, 19). `CHOICE_HELP` covers only seed set, profile, arm params, raw files. | RunForm.jsx:72-100; images 19, 22 | Medium | S16 Simplicity (concise, clear), Flexibility (expertise) | Newcomers cannot choose; experts must scan 19 raw IDs. |
| B-06 | Density and whitespace | Flat 16 px gap between all blocks and 12 px inside panels: the vertical rhythm does not distinguish a new section from a new paragraph. Lab pages 4.3k to 6.7k px tall (images 15, 22). | app.css:70,91 | Medium | S15 (leading), S16 Craft | Scanning is slow. |
| B-07 | Placeholder pages | Cluster and Intelligence render a state block plus a panel saying "See docs/design/ui_api_requirements.md, section for this screen." (developer text in a product page). `COPY.lab` in `Placeholder.jsx` is dead code. | Placeholder.jsx:9-13,52-55; image 24 | Low | S16 Purpose (copy), Responsibility | Reads as unfinished; wrong audience for the sentence. |
| B-08 | Footer text | "Telemetry schema 2. Values come from the gateway and carry their provenance; this page computes no metrics." on every page including the Lab and Simulation (where the first clause is false: Lab data is not schema-2 telemetry; Simulation is model output). | App.jsx:119-121; images 5, 6 | Medium | S16 Responsibility (accuracy) | A claim that is untrue on two screens. |

### 3.3 C. Interaction quality

| ID | Area | Description | Evidence | Sev | Skill | User impact |
|---|---|---|---|---|---|---|
| C-01 | Pristine-form errors | The run form validates on every render. A user who has touched nothing sees red "Problem: Choose at least one scenario / workload family / arm / Number of seeds is required" four times (images 16, 19, 22, 26) and a Start button that says "Disabled until the form is valid: 4 field(s) need attention." There is no touched state. | RunForm.jsx:164-166,223-230; lab.css:60-61 | High | S16 tactical ("validate inline, not on submit" is about feedback at the right moment, not before interaction), S1, Agency | The first screen of the primary task is a wall of errors. Anxiety for a task that is safe. |
| C-02 | Default path | Default kind is the simulation matrix with **nothing selected** (`initialKind='t1_matrix'`, empty T1 form), so the default state is invalid; the simpler Scenario B form (which has valid defaults, image 16) is second. | RunForm.jsx:159-161; labForms (emptyT1Form) | High | S16 Simplicity (common path first), Agency | Starting any run needs many decisions. |
| C-03 | Form grouping | Every group (radio set, check group, phase lengths) is a bordered `fieldset`; groups nest visually with the page and the "What to run" fieldset; checkbox groups of 11, 7 and 19 items are single long columns (T1 form is 5,170 px tall at 390, image 22; 4,350 px at 820, image 19). No select-all/clear, no selected count. | lab.css:54-57; RunForm.jsx | High | S16 Grouping and mapping, Simplicity | Long, undifferentiated form; keyboard users tab through 40+ checkboxes. |
| C-04 | Units and ranges | Hints below inputs carry the range ("1 to 30."); units are in the label or hint only; no input adornments; seconds, req/s and counts look identical. | RunForm.jsx:67-69,105-127 | Low | S16 Grouping and mapping | Slightly slower entry. |
| C-05 | Primary action placement | Start button sits at the very end of a 5k px form (desktop: after all fields); the "Exact request" preview is beside the form but empty ("Shown when the form is valid.") for most of the task (images 3, 26). | RunForm.jsx:223; lab.css:49-52 | Medium | S16 Agency, Wayfinding | No sense of progress through the form. |
| C-06 | Run in progress | While running, the user sees: a green full-width banner "Live updates: Live / Events are arriving / Last event received: number 7" (a connection fact styled like success), a Status panel with a native progress bar and "0 of 4 units done. arm BASE, repetition 1, phase recovery" (image 17: progress empty while phase says recovery), timestamps, the cancel control below the fold of the panel, then **Result "Waiting for the run to finish"**, artifacts, settings, and the event log **collapsed in a details at the very bottom**. No elapsed time, no phase stepper, no live event tail. | RunsViews.jsx:53-75,109-122,208-278; image 17 | Blocking coherence (primary workflow) | S1, S16 feedback (status/completion), wayfinding | Users cannot tell what is happening, how far along the run is, or whether the stream is healthy versus the run. |
| C-07 | Stream status styling | The live-stream banner uses the success palette when everything is normal, and the same banner component as warnings. | RunsViews.jsx:62-72 | Medium | S16 feedback kinds, A-12 | Alert fatigue; a normal connection does not deserve a banner. |
| C-08 | Cancel and destructive actions | Positive: inline confirmation with the safe choice focused, Esc, explicit consequences, focus return (image 17). Minor: the page-level red box is heavy. | RunsViews.jsx:77-107 | Low (keep) | S16 Agency, S7 | Keep behaviour; restyle only. |
| C-09 | Error recovery | `ErrorBlock` explains cause and fix and offers Retry; auth errors point to "the connection panel above". If the connection panel is collapsed (planned) that pointer must follow. | Gate.jsx | Low | S16 feedback | Keep; update the pointer. |
| C-10 | Press feedback | No `:active` transform on buttons, no row tint on tables with links, no hover on rows. | app.css:139-143 | Medium | S1, S14 | Minor sluggishness. |
| C-11 | Responsive controls | Touch targets are handled well (44 px under 960 px / coarse pointer, 32 px desktop) (R and V in app.css:200-208). Round-3 N-6 (14 px overflow of Overview at 320) is open (R). | app.css:200-208; round-3 report | Low | S14 | Keep. |
| C-12 | Reduced motion | `prefers-reduced-motion` collapses transitions; there are almost no animations to reduce (A-07). The plan adds some; rule: cross-fade only. | app.css:175-177; tokens.css:187-192 | Low | S14 | Must stay true after Increment 1. |
| C-13 | Clipboard | Copy buttons give a polite status; good. | RunForm.jsx:133-153 | Low (keep) | S13 (utility) | Keep. |

### 3.4 D. Phantom Lab in depth

Questions from the brief answered with evidence:

- **Is the main workflow discoverable?** Partly. The route exists as a tab; the landing page is an essay and offers no primary action (B-02). **No.**
- **Do connection panels and banners dominate?** **Yes.** On 1280 px the Connection panel occupies y=175-632 of a 1080 px viewport and the tab strip and form begin at y~650-700 (images 3, 14, 26). It repeats gateway URL, Lab API enabled, "Mode: live", and the intelligence notice (also on the Overview story and "What this does not show", `ConnectionPanel.jsx:97`, `StoryView.jsx:103`). On 390 px it is about 600 px. The debug-build banner prints a Windows file path of the gateway binary.
- **Is starting an experiment intuitive?** No (C-01..C-05).
- **Can users tell what is happening during a run?** No (C-06, C-07).
- **Are telemetry, decisions, outcomes understandable?** Verdict chips (Not supported, Trade-off failure, Unverified) with glyph and word are good and have equal weight (images 1, 12). Reasons are long sentences with numbers. Outcome and evidence are separated by many panels.
- **REAL/SIMULATED/REPLAYED/PREDICTED/DERIVED/UNAVAILABLE distinguishable?** SIMULATED, DERIVED, UNAVAILABLE and measured are distinguishable by glyph, word, colour and border (V and S). REAL is called MEASURED; REPLAYED and PREDICTED do not exist in the UI (A-11).
- **Charts and tables communicate?** Text-first design is sound and every chart has a table alternative (R in `LabCharts.jsx`, S "View as table"). Weaknesses in D-04..D-06.
- **Evidence inspectable without overwhelm?** No: the Scenario B result is about 14 panels and 6,700 px (image 15) with the same limitation statements repeated (D-07).
- **Shows the engineering rather than hides it?** Yes, to a fault: the exact request JSON and equivalent CLI command are shown (good), artifacts with SHA-256, manifest, build profile. The problem is order and weight, not absence.

| ID | Area | Description | Evidence | Sev | Skill | User impact |
|---|---|---|---|---|---|---|
| D-01 | Hierarchy of Lab chrome | Connection panel, two banners and a token form precede the sub-navigation on every Lab view (spec requires this: `phantom_lab_ui.md` line 20 "Always present on every Lab view"). | LabScreen.jsx:272-286; images 3, 11, 12 | Blocking coherence | S16 Simplicity, Purpose, wayfinding | Primary workflow is buried under ~450 px of status that rarely changes. **Fix requires amending that spec line (lead decision).** |
| D-02 | Redundant status | Header pill "Live", "Mode: live", gateway URL, panel pill "Lab API enabled", "Gateway http://localhost:8085" repeated. | Shell.jsx:16-31; ConnectionPanel.jsx:62 | Medium | S16 Simplicity | Noise. |
| D-03 | Raw markup in evidence | The binding interpretation notice is rendered in a `<pre>` as raw Markdown: `# Scenario B - dated ...`, `**Verified:**`, `**Qualifications ...:**`, backticks (image 9; `VerbatimText` in LabParts.jsx:96-98; used EvidenceViews.jsx:180). Monospace 12 px, 24 rem max height, internal scroll. | image 9 | High | S15, S16 Craft, Simplicity | The most important text in the product (it must accompany any citation) is the least readable. Raw asterisks signal an unfinished product. |
| D-04 | Effect-size chart axis text | The axis caption "Axis runs from -318.0% to +18.0%..." is squeezed into a ~100 px wide first column and wraps into 8 lines under the chart (image 10). Cause not verified (**H**: the `li.lab-effect__axis-text` rule at lab.css:93 is ignored by the flex `ul`, or a child-span grid rule at lab.css:85 applies). | image 10; lab.css:85,93; LabCharts.jsx:202 | Medium | S16 Craft | Looks broken. |
| D-05 | Chart legibility | Single series colour for everything; dots are 12 px; scale is dominated by outliers (-318% to +18%) so ~all non-outlier rows cluster at the left; no direction cue ("worse | better") on the axis, only a sentence in a note above; values are far right (up to 1,300 px from the label at 1920 px). Zero-length intervals show as lone diamonds. | image 10; lab.css:79-92 | Medium | S16 Grouping and mapping | A reader has to cross the screen to connect label and value; direction must be inferred from text. |
| D-06 | Chart/table switching | "View as table" is a closed `<details>` under each chart; the user cannot compare side by side or tell the table exists unless they read the small summary. | image 10 | Low | S16 Flexibility | Accessible but undiscoverable. |
| D-07 | Evidence overwhelm | Scenario B result panels: Label and verdicts, Cause and effect (3 charts), Arms side by side, Paired differences (one 8-column table per arm), Mechanism check, Workload and settings, What this does and does not show (about 10 limitation bullets at 13 px, plus interpretation notes that repeat them), Artifacts, Settings used, Event log. Total ~6,700 px (image 15). No on-page navigation, no "result at a glance". | ScenarioBResult.jsx; RunsViews.jsx; image 15 | High | S16 Simplicity (clear hierarchy), Flexibility | Users cannot get the outcome in one glance or jump to the part they need. |
| D-08 | Table numerics | `CellText` wraps numbers in `<span class="num">`, but right-alignment only exists for `td.num` (app.css:162), so numeric columns in Lab tables are left-aligned; units are in column headers inconsistently. | LabParts.jsx:59-68; app.css:162; ScenarioBResult.jsx tables | Medium | S15 (tabular figures), S16 Craft | Hard to compare magnitudes. |
| D-09 | Tables | No row hover or press tint; sticky header exists; no sticky first column; the M6 per-scenario table has 9 columns. | app.css:157-162; EvidenceViews.jsx | Low | S1 (dense rows tint) | Minor. |
| D-10 | Story page | Meta-copy in a product page ("Plain language. No numbers on this card.", "Both results below went through the same rule..."); the intelligence notice appears here and in the Connection panel; left result card has empty bottom space (image 1). Honest-negative-result weight is correct. | StoryView.jsx:53,64,103; image 1 | Low | S16 Purpose | Minor copy and balance. |
| D-11 | Run history table | Good: caption, links as mono IDs, state chip, provenance badge. No filter or kind grouping; refresh button above the panel rather than in it. | image 11; LabScreen.jsx:141 | Low | S16 Grouping | Minor. |
| D-12 | Token form | Always shown when required, with a long privacy paragraph. Good content, but belongs in the collapsed connection details unless a token is required and absent. | ConnectionPanel.jsx:18-43 | Low | S16 Simplicity | Part of D-01. |

### 3.5 E. Legacy Simulation interface (`ui/src/SimDemo.jsx`, `screens/Simulation.jsx`)

Observed in images 5, 6, 23 (1920 and 390) and in all 517 source lines.

| ID | Area | Description | Evidence | Sev | Skill | User impact |
|---|---|---|---|---|---|---|
| E-01 | Visual language | Entire component is inline-styled with its own hex palette (`#050607`, `#FF6B00`, `#4CAF50`, `#F44336`, ...) and **ignores light/dark tokens**: it is a black rectangle with orange text inside a light page (images 5, 6). The forbidden-pattern test exempts the file. | SimDemo.jsx:238-517; forbidden.test.mjs:56-58 | Blocking coherence | S16 Familiarity, Craft; project token rule | Looks like a different product; no theme, no tokens, no focus styling. |
| E-02 | Provenance legibility | The only per-tile provenance marker is "SIMULATED . preset model" / "counterfactual" / "queue model" / "economic model" at **8 px, `#444` on `#0B0C0E`: contrast about 2.0:1 (C: relative luminances 0.058 and 0.004)**. KPI labels are 9 px `#666`. | SimDemo.jsx:~283-319 | Blocking coherence (non-negotiable) | S15, S16 Responsibility; project rule 4 ("never hide or play down a label") | The label that must never be played down is the least legible text on the page. |
| E-03 | Fixed layouts | `repeat(7,1fr)`, `repeat(6,1fr)`, `2fr 1fr`, `1fr 1fr 1fr` grids with no breakpoints. At 390 px: title breaks letter by letter ("PHANT OM CONTR OL"), KPI tiles are clipped at the viewport edge ("BACKEN", "REPLICATIO"), control bar buttons cut off ("RESE"). 11-13 px text; 8-10 px captions. | image 23; SimDemo.jsx:~292,372,420 | High | S15 (Dynamic Type), S16 Flexibility | Unusable on phones; unreadable type. |
| E-04 | Stacked banners | Three layers say the same thing: the component's own orange banner (with the link text "SWITCH TO LIVE TRUTHFUL DASHBOARD", which implies the rest is untruthful), the wrapper `SimulationBanner` (double 4 px border), and `.sim-frame` (another 4 px double border). | Simulation.jsx:14-21; SimDemo.jsx:~249; app.css:172 | Medium | S16 Simplicity | Overstated; one clear banner is enough. |
| E-05 | Logical flag | `PHANTOM: ON/OFF` multiplies incoming p50/p95/p99 by 2/2.5/3 and load by 2.5 and zeroes savings (lines 98-103): a **fabricated counterfactual** displayed as a comparison. Baseline hit rate fixed at 0.70 (line 58). The control's label does not say it is a model assumption. | SimDemo.jsx:58,98-103,~254 | Blocking coherence | S16 Responsibility; CLAUDE.md section 4 | A user can read "Phantom off = 3x p99" as a measurement. |
| E-06 | Dollar and economic tiles | "NET SAVED/1M", "INFRASTRUCTURE COST BREAKDOWN [SIMULATED MODEL]", "STATUS: NOT ECONOMIC", constants `$0.15`, `$0.05`, break-even 8000 req/s (lines 67-71, economics block). Formatting bug: `$-0.15`, `+$-0.15/hr` (image 5). The forbidden-pattern list contains these exact fixed numbers for live screens; SimDemo is the exemption. | SimDemo.jsx:67-71; image 5; forbidden.test.mjs:52 | Blocking coherence | S16 Responsibility; CLAUDE.md section 4 (no counterfactual/cost tiles) | Invented economics presented as analysis. |
| E-07 | Synthetic values as KPIs | Initial state shows hit rate 88%, coalescing 40% next to "0 req/s" (image 5); initial metrics 12050 req/s, 28.4% CPU; timeline starts with a hard-coded "[23:34:01] SYSTEM_BOOT"; header "SEED: 48291" is a literal; "CLOCK" renders once. | SimDemo.jsx:27-56,58-59; ~254 | High | S16 Responsibility | Fabricated records that look like run state. |
| E-08 | False status | `STATUS: CONNECTED` (green) is the initial React state (line 4) before any connection exists; it is "connected" with 0 req/s (image 5). | SimDemo.jsx:4; image 5 | High | S16 feedback (status), Responsibility | Status that is not derived from the connection. |
| E-09 | PIE inspector | "PIE DECISION INSPECTOR [SIMULATED]" shows `Canary Duration: 30s` and `COMMITTED` hard-coded. CLAUDE.md section 6b: the real PIE has no canary. | SimDemo.jsx:~360-380 | High | S16 Responsibility | Presents a capability that does not exist. |
| E-10 | Modals | "WHY PHANTOM?" and "DATA INSPECTOR": `position: fixed; top 30%; left 35%; width 30%`, no `role="dialog"`, no focus trap, no Esc, no scrim; unusable below ~700 px. "Why Phantom" lists unverified product claims ("Balances regional replica hot keys adaptively"). | SimDemo.jsx:~480-515 | High | S7, S14, S16 Agency, Responsibility | Inaccessible; unsupported claims. |
| E-11 | Own network code | Opens its own `EventSource(/api/events)` (legacy schema 1) and `POST /api/control` (lines 92, ~190), duplicating `useTelemetry`, exempt from the no-fetch test. | SimDemo.jsx:89-121,~190; forbidden.test.mjs:71 | Medium | S16 Familiarity | Divergent behaviour, no connection-state integration. |
| E-12 | Accessibility | No labels on SVG charts, no table alternative, status by colour only, 21 px high buttons at 1280 (R, N-7), Simulation excluded from the a11y lint. | SimDemo.jsx charts; a11y.test.mjs:34; round-3 N-7 | High | S14 | Not meaningfully usable with keyboard/AT. |
| E-13 | Glyphs | Emoji and symbols (`⚠️`, `▶`, `⏸`, `✓`) instead of the project's hand-drawn icon set. | SimDemo.jsx | Low | S16 Craft | Inconsistent. |

**Decision for E (recorded as a recommendation, owner/lead to confirm): RETIRE `SimDemo.jsx`.**
Reasons: E-01, E-02, E-05, E-06, E-07, E-08, E-09 are violations of project rules 4 and 6b, not styling issues; migrating them to tokens would make fabricated numbers look more credible. The real simulation capability already exists, honestly labelled, as the Lab "Simulation matrix" (T1) and the retained M6 evidence. The only unique value of the demo (animated topology and request-flow story) can be rebuilt later, if wanted, as a replay viewer over real T1 artifacts (needs a backend or artifact contract; out of scope here).
Route handling: keep `#/simulation` and `?view=sim-demo` valid (tests and bookmarks) and render a short bannered explainer that links to Lab > Start a run (simulation matrix) and Lab > Evidence. Remove the nav item's separate "Simulated" group (nav change is Increment 4).
Fallback if the owner declines retirement: "migrate-lite" in Increment 3 (tokens, responsive grid, remove E-05/E-06/E-07/E-08/E-09/E-10, relabel the flag as `Model assumption (SIMULATED)`). That leaves a much smaller, honest component.

### 3.6 F. Other pages (inventory and decision)

| Page | Observed inconsistencies | Decision | Skill |
|---|---|---|---|
| Overview (`Overview.jsx`) | B-03 (no health statement, raw field dump), nested tiles (A-03), duplicate "Live" info in Connection panel and header. 14 px overflow at 320 (R, N-6). | **Redesign** modestly: tiles grouped reported / not reported, diagnostics in a closed `details`, connection facts condensed to one definition list. | S16 Simplicity, S15 |
| Traffic & latency | Percentile bars all "unavailable" (image 8) render four empty tracks; sparklines show empty boxes with "axis 0.0 to 0.0" (image 8); panels have unequal heights. | **Migrate** to shared chart frame and state grammar. | S16 Craft |
| Cache & memory | Capacity bar at 0% is invisible (image 13); "Not available yet" is another panel. | **Migrate**; show track with end ticks and the numeric text. | S16 Craft |
| Freshness, Origin | Same patterns (screenshots only; source not read). | **Migrate** with the shared components. | S16 Familiarity |
| Cluster, Intelligence (placeholders) | B-01, B-07. | **Migrate** to quiet "not available" presentation; keep routes; consider moving out of the primary nav. | S16 Purpose |
| Simulation demo | E-01..E-13. | **Retire** (see above). | S16 Responsibility |
| Header/footer | A-15, B-08, A-16. | **Redesign** header for phone, footer text per page kind, route-specific `document.title`. | S16 wayfinding |

## 4. Prioritised remediation plan

### 4.0 Principles for all increments
1. DOM contracts (`data-*` attributes, roles, provenance classes `prov prov--*`, tier keys, `data-tier`) are stable unless an increment lists the change and the test update. This keeps provenance tests meaningful while the look changes.
2. No new dependency, no framework change, no backend change. Markdown rendering is a small in-repo function producing React elements (no `dangerouslySetInnerHTML`).
3. Every increment: update the compliance table in its change notes (project rules section 5); run `npm run check` (forbidden, contrast, a11y, tests); record before/after pairs from the B0 gallery; list human-review items; **no agent declares the UI complete**.
4. Order: I1 first and alone; then I2 and I3 in parallel (disjoint files); then I4 (after I3, because it edits the a11y test filter and routes); then I5.
5. Rollback: each increment lands as a separable change set; CSS classes introduced by I1 are additive (old classes stay until the last consumer migrates, removed in I5).

### 4.1 File ownership map (one writer per file; no file appears twice)

| Increment | Owner of these files (exclusive) |
|---|---|
| **I1 Foundations** | `ui/src/styles/tokens.css`, `ui/src/styles/app.css`, `ui/src/styles/shell.css` (new, created by I1, ownership passes to I4 when I1 is merged), `ui/src/components/ProvenanceBadge.jsx`, `ui/src/components/Icon.jsx`, `ui/src/components/States.jsx`, `ui/src/components/Panel.jsx`, `ui/src/components/Metric.jsx`, `ui/scripts/check-contrast.mjs`, `docs/design/tokens.md`, tests: `ui/test/provenance.test.mjs`, new `ui/test/tokens.test.mjs` |
| **I2 Phantom Lab** | `ui/src/styles/lab.css`, `ui/src/screens/lab/*` (all), `ui/src/components/LabParts.jsx`, `ui/src/components/LabCharts.jsx`, `ui/src/lib/labForms.js`, `ui/src/lib/labView.js`, `ui/src/lib/labProvenance.js`, new `ui/src/lib/miniMarkdown.js`, `docs/design/phantom_lab_ui.md`, tests: `ui/test/lab.render.test.mjs`, `lab.forms.test.mjs`, `lab.fixes.test.mjs`, `lab.round2.test.mjs`, `lab.view.test.mjs`, `lab.guard.test.mjs`, new `lab.markdown.test.mjs`, new `lab.hierarchy.test.mjs` |
| **I3 Legacy simulation** | `ui/src/SimDemo.jsx` (retire/migrate-lite), `ui/src/screens/Simulation.jsx`, new `ui/src/styles/sim.css` (imported by Simulation.jsx), `ui/scripts/forbidden-patterns.mjs`, tests: `ui/test/forbidden.test.mjs`, new `ui/test/sim.test.mjs` |
| **I4 Non-Lab pages and nav** | `ui/src/App.jsx`, `ui/src/components/Shell.jsx`, `ui/src/components/Charts.jsx`, `ui/src/lib/routes.js`, `ui/src/lib/theme.js`, `ui/src/screens/{Overview,Traffic,CacheMemory,Freshness,Origin,Placeholder,ScreenFrame}.jsx`, `ui/src/styles/shell.css` (after I1), new `ui/src/styles/pages.css` (imported from `shell.css`), `ui/index.html`, tests: `ui/test/render.test.mjs`, `a11y.test.mjs`, `misc.test.mjs`, `connection.test.mjs`, `viewmodel.test.mjs` (only if view-model touched; it should not be) |
| **I5 Final polish and gallery** | `ui/scripts/browser-gallery.mjs` (B0), `docs/design/remediation/after/*`, `docs/design/remediation/05_*.md`, cleanup removing now-unused legacy classes from `app.css`/`lab.css` **by reassignment from the lead** (those files are I1/I2-owned until I5 starts), docs updates (`docs/design/principles.md`, `ui_audit.md`) |

Cross-increment dependency notes: I2 consumes CSS classes and components created by I1 (listed in 4.2) and must not start before I1 is merged. I3 consumes only `tokens.css`. I4 consumes I1 and, for chart frame sharing, may re-export from I2's `LabCharts.jsx` **only through a shared module created by I1** (`ui/src/components/ChartFrame.jsx`, new, owned by I1, used by I2 and I4).

### 4.2 Increment 1: Foundations

Goal: one coherent design vocabulary that later increments only consume. Skill: S15, S16 (Craft, Simplicity, Familiarity), S1, S4, S14, S12 (solid only).

Changes (files per ownership map):

1. **Type tiers** in `tokens.css` (rem, system fonts, size-specific tracking and leading together, S15):

| Tier | Size | Weight | Leading | Tracking | Use |
|---|---|---|---|---|---|
| `display` | `clamp(1.75rem, 1.4rem + 1.5vw, 2.5rem)` (28-40 px) | 600 | 1.1 | -0.02em | **At most one per page**: the answer (a run verdict figure, the gateway health word). |
| `headline` | `clamp(1.375rem, 1.2rem + 0.8vw, 1.75rem)` (22-28 px) | 600 | 1.2 | -0.015em | page h1 |
| `title` | 1.125rem (18 px) | 600 | 1.3 | -0.01em | section h2 |
| `subtitle` | 1rem (16 px) | 600 | 1.35 | 0 | h3, card titles |
| `body` | 0.9375rem (15 px) | 400 | 1.5 | 0 | prose |
| `dense` | 0.8125rem (13 px) | 400/500 | 1.4 | 0 | tables, form hints, kv lists |
| `caption` | 0.75rem (12 px) | 400/500 | 1.35 | +0.02em (+0.04em when uppercase) | meta, badges, footer |
| `metric` | 1.75rem (28 px), `tabular-nums` | 600 | 1.1 | -0.01em | metric tile value (replaces the 24 px `--text-metric`) |

No size below 12 px anywhere. `h4` gets `subtitle` style at `dense` size. `html { font-size: 100% }` is kept; spacing and type in rem so browser text scaling works (S15).
2. **Spacing rhythm** (keep the 4 px grid): add semantic tokens `--space-inline` (0.5rem), `--space-field` (0.75rem), `--space-group` (1rem), `--space-section` (2rem), `--space-page` (3rem). `.app-main` gap becomes `--space-section`; panel internal gap `--space-group`. Remove `--space-7` or use it as `--space-page`.
3. **Elevation scale (applied)**, solid surfaces only (S12 override): `--elev-0` page (`--bg`), `--elev-1` surface (`--surface`, 1 px border, no shadow), `--elev-2` raised (`--surface` + `--shadow-2` + 1 px border; used by the mobile nav drawer, sticky run header when stuck, popovers), `--elev-3` dialog (`--shadow-2`, `--scrim` token for the dimming layer, no blur). Add `--surface-3` (subtle, for table header and well). Document which components use which level (tokens.md).
4. **Motion tokens**: `--dur-press: 100ms`, `--dur-fast: 120ms`, `--dur-base: 200ms`, `--dur-slow: 300ms` (maximum), `--ease-out`, `--ease-standard: cubic-bezier(0.2, 0, 0, 1)`; no overshoot (S4). Press feedback rule: `.btn:active { transform: scale(0.97); transition: transform var(--dur-press) var(--ease-out); }` and for dense rows/cells `background` tint instead of scale (S1; project rule). Add `.spinner` (rotate 0.9 s linear; under reduced motion it is replaced by a static arc plus the word "Loading") and an indeterminate progress bar (translating gradient segment; under reduced motion a static striped bar). Under `prefers-reduced-motion: reduce`: transforms removed, 200 ms opacity cross-fade kept for disclosure and banner appearance (S14).
5. **State grammar** (replaces A-12). One table in `tokens.md` and in CSS:

| State | Visual | Text | Never |
|---|---|---|---|
| loading | spinner or indeterminate bar, `aria-busy` | "Loading X" | dashed border |
| empty | quiet text block, icon `empty`, solid 1 px | "No X yet. Not a failure." | |
| unavailable | `UNAVAILABLE` badge (dotted, reserved) + reason line | "Unavailable: reason" | a number, zero |
| stale | `STALE` tag with age, value in `--text-2`, no dotted underline | "last received 12 s ago" | |
| error | critical left rule + glyph + cause + fix + Retry | | |
| disabled | `--text-3`, no border change, `not-allowed`, plus reason text linked by `aria-describedby` | | dashed border |
| simulated | `SIMULATED` badge (see 6), and on simulated screens a single banner | | |
| success/completion | transient inline "Done" status text (`role=status`) | | permanent green banner |
Reserve **dashed and dotted borders exclusively for provenance badges** (inferred = dashed, unavailable = dotted); state blocks, disabled buttons and tags move to solid.
6. **Provenance badge simplification** (all labels kept): one anatomy: 20 px high, `caption` size, uppercase, glyph + word, `1px` border. Differentiation: MEASURED and DERIVED tinted fill and solid border; INFERRED outline only, dashed border; SIMULATED tinted fill, **2 px solid** border (replaces double 3 px; still the heaviest badge, and the Simulation banner/frame continue to exist); UNAVAILABLE no fill, dotted border. Reserve tokens and glyph shapes `--prov-replayed`, `--prov-predicted` (rendered nowhere yet). Tier label wording is unchanged; the "RETAINED:" prefix becomes a separate small `retained` tag next to the badge (I2 uses it) so the badge expresses one idea. **DOM contract unchanged.** Badge border contrast for each variant against its surface must be at least 3:1 (new contrast pairs added to `check-contrast.mjs`).
7. **Form-group pattern** (CSS only in I1, used by I2): `.field` (label, control, hint, error; unit suffix slot), `.field-group` (h3 + description + fields; **rule above, no box**), `.choice-list` (checkbox/radio list with `.choice-list__tools` for select all / clear and a `3 of 11 selected` counter), `.form-summary` (error summary), `.form-bar` (sticky action bar, `--elev-2` when stuck). Inline-error style: critical text with glyph, appears only when `data-touched`; `Problem:` pseudo-element prefix removed in favour of a glyph.
8. **Shared pieces**: `.status-strip` (one-line status with chips), `.seg` (segmented control for Chart | Table and theme), `.disclosure` (details styled with chevron, 32/44 px summary), `.prose` (max 70ch, 15 px/1.6, lists, inline code) for rendered markdown, `.section` (heading + content separated by `--space-section`, **no box**), `.stepper` (phase steps). New component `ui/src/components/ChartFrame.jsx` (title, subtitle, provenance badge, Chart/Table toggle, axis note full-width, legend, caption).
9. **Dark-block dedupe**: use `light-dark(<light>, <dark>)` for all theme-dependent tokens with `color-scheme` set by `:root` (OS) and `[data-theme]` (override). One declaration per token. (Baseline widely available since 2024; falls back? The fallback is the light value only for older browsers: **accept or reject at review**; alternative: keep two blocks but generate nothing.) Update `check-contrast.mjs` to parse `light-dark()` pairs. Forced colours unaffected.
10. **Unused tokens**: delete or give each a consumer (`--text-lg` mapped into the scale, `--shadow-*` now used by `--elev-2/3`, `--dur-base` used by disclosure, `--space-7` renamed). Each remaining token must appear in a `tokens.test.mjs` allowlist or be referenced in CSS.
11. **Shell/pages CSS split**: move shell and page-level rules from `app.css` to `shell.css`; `app.css` keeps base, components, controls, charts, tables. Applied with identical visual result first (no change to I4 territory beyond the move).

Acceptance criteria (measurable):
- Type: computed font sizes on every rendered page are one of the tier sizes; none under 12 px (assertable with `getComputedStyle` in the gallery script by I5; static check: no `font-size` literal under 0.75rem in any `ui/src/styles/*.css`).
- Tracking is negative for `display`, `headline`, `title`; zero for `body`; positive for `caption` (static test over tokens).
- Spacing: only token values for margin/padding/gap in `app.css` and `shell.css` (static test; exceptions listed).
- Elevation: `--shadow-*` appear only inside `--elev-2/3` definitions and components listed in tokens.md; panels have no shadow.
- Motion: durations only tokens; max 300 ms; `:active` press present for `.btn`; reduced-motion block removes transforms (static test greps both).
- Contrast: all existing pairs pass; new pairs added: badge border vs surface 3:1 (non-text, 1.4.11), focus ring vs both backgrounds 3:1, `--text-3` on `--surface-3` 4.5:1; `check:contrast` exit 0 with `light-dark()` parsing.
- Single declaration per theme token (static test: count of `--bg:` definitions is 1).
- Dashed/dotted `border-style` appear only in `.prov--inferred` and `.prov--unavailable` (static test over all CSS).
- Target sizes unchanged: 24 px floor, 32 px desktop, 44 px under 960 px or coarse pointer for primary controls.
- No `backdrop-filter` anywhere (static test).

Tests to add or update: `tokens.test.mjs` (above static rules), `provenance.test.mjs` (DOM contract unchanged, all labels still present, `data-tier` unchanged), `check-contrast.mjs` pairs. These are **static guard rails; they do not show that the result looks right.**

Human-review items (I1): side-by-side of the 5 provenance badges in light and dark at 100% and 200% zoom (distinguishable without colour); type ladder specimen page; elevation specimen; reduced-motion behaviour of spinner and disclosure; whether `light-dark()` fallback is acceptable.

Risks: changing `.panel` or `.metric` geometry shifts every screenshot and may change line breaks that tests do not see; `light-dark()` support in the owner's supported browser set (U); renaming tokens breaks CSS in I2-I4 if they merged early (hence ordering); the 28 px metric may overflow narrow tiles (mitigate: tile min width 11 rem, `overflow-wrap: anywhere` already present).

### 4.3 Increment 2: Phantom Lab hierarchy

Goal: primary workflow first; run comprehension; readable evidence; honest labels kept. Skill: S16 (Purpose, Agency, Simplicity, Grouping, wayfinding, feedback kinds, validate inline), S15, S1, S14, S17.

**Prerequisite (lead):** amend `docs/design/phantom_lab_ui.md` line 20 ("Connection panel always present") to "a Lab status strip is always present; the full connection panel is one disclosure away and opens automatically when attention is needed". The two honesty statements stay visible in the strip as text chips (debug-build EXPLORATORY; intelligence not in the live serving path), never hidden. Tests asserting the old panel (`lab.render.test.mjs`) are updated by I2.

Changes:

1. **Lab status strip** replaces `ConnectionPanel` above the tabs: one row with `Lab API enabled` chip, `Auth: anonymous, loopback` (or token state), `1 run at a time`, `EXPLORATORY (debug build)` chip when applicable, `Intelligence: not in serving path` chip, `Details` disclosure. Details contain the existing definition list, availability rows, active-run notice, reasons list (without the raw binary path by default: path shown inside Details only), token form. **Auto-open** when: token required and absent; API disabled or unreachable; an active run exists. `ErrorBlock` auth hint text updated ("Open Details in the Lab status bar"). Remove the duplicate full intelligence notice from the strip; keep it on the Overview story ("What this does not show"). Gateway URL and "Mode" are not repeated (header owns them).
2. **Page head with primary action**: `Phantom Lab` headline, one-sentence description, **primary button "Start a run"** (right aligned, link-button to `#/lab/run`; hidden on the run page itself). Tab labels: `Overview`, `Start a run`, `Runs`, `Evidence` (workflow order; routes unchanged). The Lab "Overview" tab is renamed **"About the experiments"** to stop clashing with the app-level Overview (usability task T7 verifies).
3. **Run form**:
   - Choose kind as two side-by-side choice cards (radio semantic): "Simulation matrix" (SIMULATED badge, "seconds, deterministic") and "Scenario B: origin slowdown" (MEASURED: LOOPBACK badge, "minutes, loopback"). **Default kind**: the first kind that is available, with Scenario B's valid defaults considered (domain choice; lead decides; see section 7).
   - Groups per kind using `.field-group`: T1: Experiment design (scenarios, families, arms), Seeds and profile, Output; SB: Repetitions and arms, Workload (rate, keys, phases), Caps.
   - `.choice-list` with select all / clear and "n of N selected"; long lists (arms: 19) are two columns at >= 640 px; each arm shows the one-line description when `CHOICE_HELP` or the capabilities payload provides one, else nothing (no invented descriptions).
   - **Validation timing**: errors appear for a field after it is blurred/changed (`touched`), or for all fields after the first submit attempt; pristine form shows no errors. An **error summary** (`role=alert`, links to fields) appears after a failed submit attempt. The Start button stays enabled when only untouched-required errors exist so that pressing it explains what is missing (replaces the dead disabled button; project accessibility: disabled buttons without reason are poor); when it must be disabled (not available, run active, submitting) a reason is shown and linked.
   - **Sticky form bar** (`.form-bar`) with `Start run` (primary), validity summary ("3 selections missing"), and `Copy request` shortcut. On mobile the bar is the only sticky element.
   - **Exact request** and **terminal command** move into one `Request` disclosure (closed on < 1100 px, open beside the form on >= 1100 px); the valid-form placeholder text is removed.
   - Presets ("Quick check", "Registered protocol") are **not** added by default; they are a domain decision (section 7). Select-all/clear are neutral and included.
4. **Run detail (in progress)**:
   - **Sticky run header** (`--elev-2` when stuck): state chip, provenance badge, kind, run id (mono, copy), elapsed time (`INFERRED`: client clock minus server `started` timestamp, labelled), **Cancel run** at right (inline confirm retained, restyled, same focus behaviour).
   - **Progress block**: determinate bar with `n of N units`, percent shown only if both received (no arithmetic beyond formatting; projects rule); unknown total uses the indeterminate bar with the text "No progress reported yet." **Phase stepper** from `progress.phase` (Warm-up, Healthy, Degraded, Recovery: names from the payload; unknown phases render as received) with the current step highlighted and arm/repetition text ("BASE, repetition 1 of 10": only if the payload gives the total).
   - **Stream state** becomes a compact chip in the header ("Live updates: live"); it expands to the existing banner (text, last event, gaps, Reconnect) only when `stale`, `reconnecting`, `disconnected`, `failed`. Normal state has no banner (C-07).
   - **Live event tail** visible under the progress block: last 5 events (`aria-live=off` to avoid chatter; summary announcements stay in the existing `role=status` region), "Show all N" opens the full log; the existing log list is kept as the expanded content.
   - **Result at a glance** takes the top slot once the run is terminal: verdict chips with the same size for every outcome, run label chip, one-sentence reading assembled only from existing view-model reasons (no new claims), and up to 3 key figures each with provenance badge. At most one `display`-size element (the verdict word). Cancelled/failed/interrupted show the existing partial-result wording first.
   - **Section order** after the glance: Cause and effect, Arms, Paired differences, Mechanism, Workload and settings, Limitations, Artifacts, Settings used, Event log; an **"On this page" list** (sticky aside >= 1100 px, disclosure on mobile) with anchors and counts.
5. **Evidence readability**:
   - New `miniMarkdown.js`: block parser for `#`..`###` headings, paragraphs, ordered/unordered lists, `**bold**`, `` `code` ``, line breaks; output React elements; unknown syntax is shown as text; never HTML. `VerbatimText` becomes `RichNotice`: rendered view by default in `.prose`, with a `Rendered | Raw` segmented control; Raw is the exact payload string in the current `<pre>` and has a Copy button. The binding notice stays expanded by default and keeps the "must accompany any citation" line.
   - Limitation and interpretation lists use `.prose` (14 px, max 70ch, spacing between items), not 13 px dense bullets; duplicated statements across "limitations", "interpretation notes" and notice are **not** removed (they are binding text), but sections are visually grouped under one heading "What this does and does not show" with sub-headings and an item count.
   - Operational sections (Files in this experiment, Manifest provenance, Artifacts, Settings used, Event log) become closed disclosures with counts in the summary.
6. **Charts** (`LabCharts.jsx` using `ChartFrame`): title, subtitle and badge in the frame; **Chart | Table** segmented control (table is the same DOM as today, not hidden in `details`); axis caption full width below the plot (fixes D-04); direction labels on effect charts ("worse" left of zero, "better" right of zero, taken from the existing "Positive means..." semantics per chart, not new claims); end-of-row values placed adjacent to the bar on wide screens; marks 16 px with 2 px outline; one series colour plus a second token for the comparator where two series exist; an "axis spans a to b" note when an outlier compresses the plot (arithmetic-free: it is the axis range already computed by `scaleLinear`). Numeric table cells are right-aligned (`td` gets `.num` via `CellText` wrapper or table-level CSS).
7. **Surfaces**: Lab sections use `.section` (rule + spacing) instead of nested `.panel` where a panel is only a heading wrapper; cards remain for *entities* (a run, an evidence item, a result card) not for headings. Story page: keep content, remove meta-copy sentences ("Plain language. No numbers on this card."), balance card heights with grid `align-items: stretch`.
8. **Loading and empty states** use the I1 grammar (spinner, quiet empty block).
9. `document.title` for Lab routes: "Phantom Lab: Start a run", "Phantom Lab: Run 2026...": set in `LabScreen.jsx` through an effect (the global route title is I4; to avoid two writers, I4 owns `App.jsx` and exposes `document.title` through a prop-free helper `ui/src/lib/routes.js#titleFor(route)`; I2 does **not** touch the title; see I4).

Acceptance criteria (measurable):
- On a 1280x800 viewport with Lab API enabled and no token needed, the Start-a-run form's first field and the primary button are visible without scrolling (screenshot assertion by Track 3 reviewer; automated browser check for the y position of the first `legend`/`h3` of the form less than 700 px).
- Lab status strip height at 1280 is at most 64 px when closed (browser measurement); at 390 at most 3 rows.
- Pristine run form: zero visible error messages (render test: `data-field-error` count 0 before interaction; after simulated blur on one field only that field shows an error; after failed submit the summary lists all).
- No rendered element in the Lab contains the literal sequences `**`, `` ` `` or a line starting with `# ` in the rendered (not Raw) notice (render test with the real fixture `evidence_scenario_b.json`); Raw view is byte-equal to the payload string (test).
- Phase stepper present when `progress.phase` is reported; elapsed time carries an `INFERRED` badge; indeterminate bar present when total unknown (render tests with `events_sb.sse`, `events_t1.sse` fixtures).
- Stream banner not rendered in `live` state; rendered with Reconnect in the four problem states (existing stream tests updated).
- Each chart renders a visible Chart/Table toggle; table DOM equals the old `details` table (no data lost); axis caption element is a block-level sibling of the plot (test + browser measurement: caption width at least 80% of plot width).
- Heading hierarchy: one h1, h2 per section, h3 within; no skipped levels (existing lint).
- Provenance: every numeric figure still adjacent to a badge or inside a table row with a provenance column (existing provenance tests, extended).
- Target sizes and focus as before; Esc closes cancel confirm; focus returns to Cancel.
- Evidence page: sticky "On this page" has one anchor per section and Tab reaches it once.
- Page height of a completed Scenario B run at 1280 reduced by at least 30% versus image 15 by default state (operational sections closed). Informational, measure in gallery; **not** a pass/fail without review.

Tests (static; list separately from human review): updates to `lab.render`, `lab.forms` (touched/summary behaviour, Select all, preserved validation rules in `labForms.js`), `lab.fixes`, `lab.round2`, `lab.view` (only if view-model additions), new `lab.markdown.test.mjs` (parser cases incl. hostile input `<script>`, nested bold, unbalanced markers, 20,000-char cap), new `lab.hierarchy.test.mjs` (strip, primary action, tab order, no banner in live).
Human review items (I2): does the landing page answer "what is this and how do I start" in 10 seconds; run-in-progress comprehension with a real running gateway (stall, restart, cancel); readability of rendered notice versus raw; chart reading speed (can a reviewer state which arm shed requests within 15 s); no label looks demoted.

Risks: spec amendment may be refused (then keep the panel but collapsed by default with the same auto-open rules: lowest-risk variant); form validation timing changes could hide a required field from an assistive-technology user (mitigate with the summary and `aria-describedby`); markdown rendering changes the exact characters of a binding notice (mitigate with Raw view and a test that `stripMarkers(rendered) == stripMarkers(raw)`); default-kind change is a domain choice; sticky headers can obscure focus (use `scroll-padding-top` and test with keyboard); more disclosures may hide material (acceptance: no binding text inside a closed disclosure).

### 4.4 Increment 3: Legacy simulation consolidation

Goal: remove fabricated content and visual divergence. Skill: S16 (Responsibility, Purpose, Familiarity), S15, S14; project rules 4 and 6b; CLAUDE.md sections 4 and 6b.

Preferred path (recommendation; needs lead/owner confirmation before deleting application code):
1. `SimDemo.jsx`: **delete** (retire). `screens/Simulation.jsx`: becomes an explainer page using shared components (`SimulationBanner`, `StateBlock`, links): "The standalone simulation demo has been retired. Simulation runs are done in Phantom Lab and are labelled SIMULATED." with links to Start a run (simulation matrix) and Evidence (M6). The route and `?view=sim-demo` mapping stay.
2. `scripts/forbidden-patterns.mjs` and `forbidden.test.mjs`: remove the SimDemo exemptions and the `simDemoImporters` expectation; assert that no file under `ui/src` contains an inline `style={{ ... color` literal other than allowlisted chart geometry, that no `EventSource`/`fetch` exists outside the data hooks and the Lab client, and that `counterfactual`, `$`-currency formatting and `avoided_compute_dollars` patterns apply to **all** files.
3. `sim.test.mjs`: Simulation route renders one banner (not three), the retired explainer, links to the two Lab routes, no metrics.

Fallback path ("migrate-lite", if the owner keeps the demo): re-home onto tokens in `sim.css` (no inline colours; theme-aware); responsive (single column below 960 px); min type 12 px; **remove**: the dollar/economics blocks (E-06), the "PIE decision inspector" (E-09), "Why Phantom" claims (E-10), the fixed `SEED`, boot timeline entry, initial KPI values (E-07; show "Waiting for the simulation" state until data arrives), `STATUS: CONNECTED` initial value (derive from `EventSource.readyState`); **relabel** the `PHANTOM: ON/OFF` button as `Model: Phantom assumed on / off (SIMULATED assumption)` with a visible note that off-values are multiplied by fixed factors; replace modals with the shared dialog (focus trap, Esc, scrim; project override allows a scrim); remove stacked banners (keep the wrapper banner only); use the project icon set; add table alternatives and SVG titles; include the screen in the a11y lint.

Acceptance (measurable): no hex colours or inline colour styles in `Simulation.jsx`/`SimDemo.jsx` (retire: file absent); the forbidden scan runs with **zero exemptions**; at 390 px no horizontal overflow and no word broken mid-word; if migrate-lite: all text at least 12 px, simulated label contrast at least 4.5:1 (script), exactly one SIMULATED banner, no currency symbol, no "counterfactual", no hard-coded numbers (existing scanner).
Human review: does the explainer read as an honest retirement and not as an error; if migrate-lite: can a reviewer tell what is model assumption versus received data.
Risks: removing the demo removes a stakeholder-visible show-piece (owner decision); retirement of `SimDemo.jsx` is deletion of a file (read by me; requires explicit approval); tests referencing the demo (`a11y.test.mjs` filter, `render.test.mjs` simulation case, `forbidden.test.mjs`) cross file ownership: I3 edits only its own tests; I4 removes the `simulation` filter in `a11y.test.mjs` and the nav item.

### 4.5 Increment 4: Non-Lab pages, header, navigation

Skill: S16 (Simplicity, wayfinding, Flexibility, Craft), S15, S14.

1. **Header and nav** (`Shell.jsx`, `shell.css`): phone header is one row (brand, status dot with text on wider widths, Menu button) at most 56 px; theme control is a 3-state segmented control (System / Light / Dark) inside the menu on phones and in the header on desktop (each at least 32/44 px); gateway URL and Mode move to a `Connection` popover/disclosure and the Overview connection list (no duplication). Side navigation appears from **820 px** (not 960) when width allows `--nav-width` 13.5 rem plus 28 rem content (breakpoint chosen by a measured reflow check, 320/390/640/820/1280). Sidebar surface extends to the full viewport height (sticky column with background on the grid cell).
2. **Nav content** (`routes.js`): keep nine routes; group labels become "Gateway", "Phantom Lab", and **placeholders and simulation move into a "More" group** (muted, with "not available" / "retired" as visible text, no triangle glyph for the retired simulation page). Item order unchanged for existing tests unless updated by I4.
3. **Route-specific titles**: `titleFor(route)` in `routes.js`, applied in `App.jsx` (e.g. "Traffic & latency - PhantomCache", "Phantom Lab: Runs - PhantomCache"; Lab run ids appear in titles via the Lab sub-route; I4 reads sub from the route so I2 does not touch titles).
4. **Footer** per route kind: live pages keep the schema-2 sentence; Lab: "Lab data is read from the Lab API; labels show where each number came from."; Simulation explainer: no telemetry claim.
5. **Overview**: a plain-language first row stating the gateway state using the existing connection words (no new metric); tiles grouped "Reported" then "Not reported (n)" as a compact list with reasons (grouping only; counts of states are labels, not metrics: principles.md "UI computes nothing" must be re-read by the reviewer); capabilities and connection merged into one definition list; "Other reported fields" in a closed disclosure with count.
6. **Metric tile**: value at `metric` tier, label above at `dense`, one meta line (badge, window, stale tag), reason line; unavailable tile uses quiet type and badge only (drop the duplicated word "Unavailable" next to the badge when the badge says UNAVAILABLE and the reason is shown; keep an accessible text equivalent).
7. **Charts** (`Charts.jsx`): use `ChartFrame`; flat series show "constant at X" instead of "axis 0.0 to 0.0" only via formatting of received min/max (no computation); capacity bar draws its track with end ticks at 0 and budget; percentile bars with all values unavailable collapse to one unavailable row.
8. **Placeholders** (`Placeholder.jsx`): remove the developer sentence; present "What this page will show" and "Needs: a backend contract (see docs/design/ui_api_requirements.md)" as a small footnote; delete dead `COPY.lab`.
9. Responsive pass for N-6 (14 px overflow at 320 on Overview `dd` with badges): `dd` wraps badge below value.
10. `index.html`: add `<meta name="theme-color">` for both schemes (cosmetic).

Acceptance: nav at 820 px shows the side navigation without horizontal scroll; header height at 390 at most 56 px (browser measurement); every route sets a distinct `document.title` (render-harness test and browser check); no overflow at 320/390/640/820/1280 on all routes including Overview (round-3 N-6 closed); metric tile value font size equals the `metric` token; footer text differs by route kind; the simulation filter in `a11y.test.mjs` removed and the explainer page lint-clean; target sizes preserved.
Tests: `render.test.mjs`, `a11y.test.mjs` (lint every route, no filter), `misc.test.mjs` (routes/titles), `connection.test.mjs` if header text moves. Human review: first-minute navigation task (T1, T8), header usability on a phone, whether the "More" grouping hides anything users need.
Risks: moving gateway URL out of the header changes a tested label; nav breakpoint change touches keyboard roving tests; Overview grouping must not suggest a health verdict the gateway did not report.

### 4.6 Increment 5: Final polish and gallery

1. Cross-page consistency pass with the I1 type, spacing, state and badge rules (copy: sentence case, no "Problem:" prefix, consistent "Unavailable: reason"; one phrase for the not-in-serving-path statement).
2. Remove legacy CSS (`.banner--simulated` double border if unused, old `.lab-fieldset`, `.bars*` if replaced) and dead tokens; update `docs/design/tokens.md`, `principles.md`, `phantom_lab_ui.md`, `ui_audit.md`.
3. B0 re-run: `docs/design/remediation/after/` with the same matrix (390/820/1280, light/dark, all routes plus Lab run in progress/final/failed/cancelled, form pristine/invalid) and a before/after contact sheet; include forced-colors and reduced-motion captures.
4. Final run: `npm run check`, build, browser rounds, then the independent visual review (Track 3) and usability tasks (Track 4).
Acceptance: before/after pairs exist for every page and state in the gallery; no raw markup or placeholder text found by a grep over the rendered DOM dump; all static checks green; **the owner's O8-2 decision remains open until the independent review concludes**.

## 5. Acceptance rubric for the independent visual review (Track 3)

Reviewer works from the B0 after-gallery (light and dark; 390, 820, 1280; states listed in I5), the before-gallery for comparison, and a running build. Each criterion is scored **Pass / Partial / Fail** with a screenshot reference and a one-line reason. The reviewer must not be an implementer. Reviewer also opens at least 20 images at full resolution (not downscaled full-page strips).

| # | Criterion | Pass looks like | Skill |
|---|---|---|---|
| V1 | Hierarchy | Within 3 s a viewer names the page's main purpose and the most important element; at most one display-size element; headings clearly ordered by size/weight/space | S15, S16 |
| V2 | Type | Only the tier sizes; no text under 12 px; tracking visibly tighter on headline/metric than on body; tabular figures aligned in columns | S15 |
| V3 | Spacing and rhythm | Sections separated more than paragraphs; consistent gutters; no boxes nested more than two deep except entity cards; nothing cramped or stranded | S15, S16 |
| V4 | Surfaces and elevation | Panels flat with 1 px border; raised layers (sticky header, drawer, dialog) visibly above; no glass/blur; no decorative shadow | S12 override |
| V5 | Provenance | Every number has a visible label; REAL/MEASURED, SIMULATED, DERIVED, INFERRED, UNAVAILABLE distinguishable in greyscale and by a protanopia/deuteranopia simulation; no label demoted relative to the value; negative results equal weight to positive | S16, project rule 4 |
| V6 | State language | loading, empty, unavailable, stale, error, disabled, simulated each look like one distinct thing used consistently on all pages; dashed/dotted reserved for provenance | S16 |
| V7 | Phantom Lab first screen | Primary action visible at 1280x800 without scrolling; status strip at most one row; honesty statements still visible as text | S16 |
| V8 | Run form | Pristine form shows no errors; groups are visually distinct without heavy boxes; select-all and counts present; validation timing as specified; sticky action bar behaves | S16 |
| V9 | Run in progress | State, progress, phase, elapsed, stream state and recent events are visible without scrolling at 1280x800; cancel reachable; no success-coloured banner for a normal connection | S1, S16 |
| V10 | Evidence | No raw markup anywhere; rendered notice readable at 70ch; limitations legible; page navigable with the on-page list; binding text not hidden | S15, S16 |
| V11 | Charts | Axis, units, direction, provenance badge and table alternative present; values connected to labels; legible at 390 | S16 |
| V12 | Dark/light parity | Same hierarchy in both; no washed or muddy surfaces; badge borders visible | S14, S16 |
| V13 | Responsive | 390 and 820 use space sensibly (no phone layout at 820 for navigation); no clipped content; sticky elements do not hide focus | S16 |
| V14 | Motion | Press feedback on buttons at pointer-down; spinner/indeterminate bar present; reduced-motion shows cross-fade only | S1, S14 |
| V15 | Consistency | The same fact (unavailable, running, cancelled) looks the same on Overview, Traffic, Lab | S16 |
| V16 | Restraint | No gratuitous cards, gradients, oversized headings or decoration; dense data retained where needed | S16 Simplicity |
| V17 | Simulation | No fabricated figure, one banner, honest explainer or migrated component | S16, project rules |

Overall outcome is recorded by the reviewer as findings only; the plan does not assume Pass.

## 6. Usability task list (Track 4)

Run with 3 to 5 participants (at least one new to caching/AIMD, one engineer) on a running gateway with Lab enabled, once on desktop 1280 and once on a 390 phone for T1, T3, T8. Observer notes time, errors, hesitations, questions asked, and whether the participant states provenance correctly. Success criteria are pass/fail per task; no coaching.

| Task | Prompt (neutral) | Success criteria |
|---|---|---|
| T1 | "Find where you would run a new experiment and open it." (from app start) | Reaches the run form within 20 s with at most 1 wrong click; names the control used. |
| T2 | "Start a Scenario B run with 3 repetitions and the STATIC-CAP and BASE arms only." | Run starts without reading documentation; at most 1 validation error encountered; participant can say why the first error appeared. |
| T3 | "A run is going. Tell me how far along it is, which phase it is in, and whether the page is still receiving updates." | All three answered correctly within 30 s; distinguishes run state from stream state. |
| T4 | "Stop that run, but change your mind before it stops." (cancel confirm then keep) | Keeps run; no accidental cancel; states consequences from the dialog text. |
| T5 | "When it finishes, what was the outcome, and is it a measurement or a simulation?" | States verdict, run label and the correct provenance (loopback measurement vs simulated) within 45 s. |
| T6 | "Show me the evidence behind that verdict and the main limitation of the result." | Opens the rendered notice or limitations; reads one limitation aloud; no confusion about raw markers; can find the exact request or artifact hash when asked. |
| T7 | "What does 'Overview' mean in this app? Where would you go to see whether the gateway itself is healthy?" | Distinguishes gateway Overview from the Lab's about page (after rename); reaches Overview. |
| T8 | "On the phone, switch to dark theme and find Traffic and latency." | Completes in at most 25 s, no horizontal scrolling, target taps succeed at first attempt. |

Also record: time to first meaningful action in Lab; number of times a participant scrolled past the status strip; whether any participant reads UNAVAILABLE as zero (must be 0).

## 7. What cannot be done without a backend change or owner approval

Needs owner/lead approval (UI only, but beyond this audit's mandate):
1. **Amend the Lab spec**: collapse the always-present Connection panel into a status strip (`phantom_lab_ui.md` line 20; also tests that assert it).
2. **Retire `SimDemo.jsx`** (delete an application file; stakeholder-visible demo) or approve migrate-lite.
3. **Provenance vocabulary**: whether to rename MEASURED to REAL in the UI, and when to introduce REPLAYED and PREDICTED. This plan only reserves tokens and glyphs; the schema field `src:"measured"` is unchanged. Choose a glossary mapping at minimum.
4. **Default run kind and presets** ("Quick check", registered protocol): domain-owned choices.
5. **`light-dark()` adoption** and the minimum browser baseline.
6. **Navigation restructure** ("More" group; moving placeholders out of the primary group) and the Lab tab rename/reorder.
7. New component `ChartFrame` and any shared dialog (not a dependency; in-repo components).

Needs a backend (or artifact contract) change; **reported as dependencies, not done here**:
- **Run progress detail**: total repetitions per arm, phase boundaries with durations, and an ETA are not guaranteed in the SSE `progress` events (only `arm`, `rep`, `phase`, `done`, `total` are consumed today); a true phase stepper with times needs the payload to carry phase start/end (U: check `lab_api_contract.md`).
- **Server-provided elapsed time** (to avoid showing an INFERRED client-clock figure).
- **Arm and scenario descriptions** for the 19 T1 arms and 11 scenarios (today only some `CHOICE_HELP` text exists in the UI); the capabilities payload would need a `description` per item.
- **Retained results as machine-readable summary for M6 and replay** (a future replay viewer to replace the demo's visual story).
- **Server-side pagination/filter of runs** for the history table.
- **Token verification** (roles are read from the pasted token for display only; unchanged).
- **Download of retained evidence files** (the API serves names and sizes only; UI cannot show them).
- Any **PREDICTED** or **REPLAYED** data: none exists yet; PIE is a stub (CLAUDE.md 6b) and must not be activated or displayed.

## 8. Compliance table: planned change to skill sections

Project overrides: "G" no glass/blur, "D" no new dependency, "P" provenance never hidden, "B" no backend change.

| Planned change | Skill sections | Overrides | How it will be checked |
|---|---|---|---|
| Type tiers with size-specific tracking/leading, rem units, tabular numerals, min 12 px | S15, S16 Craft | D (system fonts) | tokens test; computed-style sweep in gallery; V1, V2 |
| Display tier used once per page; metric tier | S15, S16 Simplicity | | review V1, V16 |
| Spacing rhythm tokens, section separation without boxes | S16 Simplicity, Grouping | | review V3 |
| Applied elevation scale, scrim, no blur | S12 (depth part only), S16 Craft | **G** | static grep no `backdrop-filter`; V4 |
| Press feedback, motion tokens <= 300 ms, no overshoot | S1, S4 (damping 1.0), S14 | D (CSS easing only; S4-S6 spring libs skipped) | static test for `:active`; V14 |
| Reduced motion: cross-fade only; spinner replaced by static arc | S14 | | static + manual `prefers-reduced-motion` capture |
| `prefers-contrast` and forced-colors retained | S14 | | existing browser checks; capture in I5 |
| State grammar; dashed/dotted reserved for provenance | S16 Familiarity, Grouping | P | static CSS test; V6, V15 |
| Provenance badge anatomy simplification (labels kept, retained tag split) | S16 Simplicity, Craft | **P** (no label hidden) | provenance tests, V5 |
| Reserved REPLAYED/PREDICTED tokens | S16 Familiarity | P | token test only (not rendered) |
| `light-dark()` single-source dark theme | S16 Craft | | contrast script, A-08 test |
| Form-group pattern, touched-state validation, error summary | S16 tactical (inline validation, grouping and mapping), S1 | | lab.forms tests; V8; T2 |
| Select all/clear, counts | S16 Grouping and mapping | | render test |
| Sticky action bar and sticky run header (`--elev-2` when stuck) | S16 Agency, wayfinding, S12 (layered depth, solid) | G | V7, V9; keyboard focus check |
| Lab status strip replacing connection panel; auto-open on attention | S16 Simplicity (common path first, advanced one level deeper), Purpose | **P** (honesty chips stay visible) | lab.hierarchy tests; V7 |
| Primary "Start a run" action; workflow tab order; tab rename | S16 Agency, Familiarity, specific labels | | T1, T7 |
| Run progress block, phase stepper, live event tail, stream chip | S16 feedback (status, completion, warning, error), S1 | B (no new backend fields) | render tests with SSE fixtures; V9; T3 |
| Cancel confirm restyle only (behaviour kept) | S16 Agency (forgiveness, confirm only destructive) | | existing keyboard test; T4 |
| Rendered markdown with Raw toggle | S15, S16 Craft | D (in-repo parser), P (verbatim preserved) | lab.markdown tests; V10; T6 |
| Evidence "result at a glance", on-page list, operational disclosures | S16 Simplicity, Flexibility | P (binding text never in a closed disclosure) | hierarchy test; V10 |
| ChartFrame: toggle, axis, direction labels, adjacent values | S16 Grouping and mapping, Craft | P | chart tests; V11 |
| Right-aligned numeric cells; row tint on hover/press | S1, S15 (tabular figures) | | CSS static test; V3 |
| SimDemo retirement, explainer, zero scanner exemptions | S16 Responsibility, Purpose | P (no fabricated data) | forbidden/sim tests; V17 |
| Migrate-lite fallback (relabel logical flag as SIMULATED assumption; dialogs) | S16 Responsibility, S14, S7 | P, G (scrim only) | sim tests; V17 |
| Header compaction, theme segmented control, nav at 820, sticky full-height sidebar | S16 Flexibility, wayfinding, S14 targets | | browser measurements; V13 |
| Route-specific `document.title` | S16 wayfinding | | render and browser tests |
| Overview grouping, diagnostics disclosure, metric tile | S16 Simplicity, S15 | P (no value or label removed) | render tests; V1, V5 |
| Placeholder copy; footer per route | S16 Responsibility, Purpose | | render tests |

## 9. Open points and what I did not do

- I did not run tests, a browser, `npm`, `cargo` or git commands that mutate (only `git rev-parse` / `status` count at start). No file other than this one was created or changed.
- I did not read `screens/{CacheMemory,Freshness,Origin}.jsx`, `LabCharts.jsx` beyond line 80, `lib/labView.js`, `lib/labForms.js` or `docs/design/information_architecture.md`; related findings rely on screenshots or grep and are marked accordingly.
- The cause of D-04 (axis caption squeeze) is **unverified (H)**.
- Contrast figure for E-02 is my computation from the hex values (C), not a tool run.
- Claims about the browser rounds come from the round-3 report excerpts I read (R).
- No judgement here establishes that the UI is acceptable or complete; O8-2 remains open.
