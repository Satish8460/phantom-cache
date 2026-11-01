# R2: motion, materials, 3D/spatial, diagrams and data visualisation toolkit research

**Owner:** R2. **Date:** 2026-10-30. **Status:** research only; every recommendation is a PROPOSAL (not executed, nothing installed). Downloaded text is untrusted data kept in `D:\Temp\toolkit_research\r2\`.
**Evidence tags:** VERIFIED-FETCHED (read from fetched primary file or registry JSON in this session), SECONDARY (search-result summary, not opened), LITERATURE/KNOWLEDGE (from background knowledge, not verified here), UNVERIFIED.

## 0. Executive summary

| Area | Proposed primary authority | Rationale |
|---|---|---|
| Motion | Emil Kowalski skills subset (pinned a81dbca0, MIT), subordinated to the project's reduced-motion and provenance rules; apple-design stays installed for physical/gesture principles | Restraint-first, includes `remove-animations` and `find-animation-opportunities`; no library required |
| Materials | Project tokens + `apple-design` (already installed) + the DD agent's measured results when published; no glass skill | Dense engineering app; text-over-blur is a contrast risk; `prefers-reduced-transparency` already handled in `ui/src/styles/tokens.css:155` |
| Diagrams | In-repo hand-built SVG with a project-local diagram spec; `diagram-design` as USEFUL REFERENCE (not installed) | 30 KB skill + 46 Python scripts, editorial aesthetic and a first-run question gate; its static-by-default and accessible-SVG rules are worth copying |
| Data visualisation | In-repo `ChartFrame` contract (chart/table toggle, provenance, unavailable panels) plus Claude's bundled `dataviz` skill for method (description only read; see section 8) | No chart library justified; existing charts are HTML/CSS/SVG |

Top licence finding: the `gsap-skills` repo is MIT, but GSAP the library is NOT MIT (proprietary "Standard no charge" licence, section 3.2).

## 1. Project grounding (VERIFIED in repo)

- `ui/package.json`: only react, react-dom, lucide-react; vite, @vitejs/plugin-react dev.
- `ui/src/components/Charts.jsx` (159 lines): `Sparkline` (svg, `role="img"`, aria-label summary), `PercentileBars`/`PercentileChart`, `CapacityBar`/`CapacityChart`; bar fills are `aria-hidden` spans.
- `ui/src/components/LabCharts.jsx` (258 lines): `TradeoffChart`, `ShedChart`, `LatencyChart`, `EffectChart` (mean with confidence interval per scenario, left/right of line = arm worse/better), `UnavailablePanel` (role note). Header comment: charts "plot received values only and compute nothing but a linear scale"; marks are `aria-hidden`, text carries values.
- `ui/src/components/ChartFrame.jsx`: `figure` with `aria-labelledby`, Chart/Table segmented toggle (`aria-pressed`), props for provenance `src`, `qualifier`, `unitLabel`. This is already the table-alternative pattern.
- CSS: tokens `--dur-fast`, `--ease-out`; keyframes `pc-spin/pc-slide/pc-appear/pc-fade` (`app.css:270-273`) with `prefers-reduced-motion` block (`app.css:277`, `shell.css:86`); `prefers-reduced-transparency` in `tokens.css:155`. No `backdrop-filter` found in `ui/src/styles/*.css`.
- Consequence: the project is already CSS/SVG only. Any recommendation requiring a runtime dependency must clear the "no new dependency without approval" rule and prove need.

## 2. Candidate: emilkowalski/skills (motion)

- Repo: https://github.com/emilkowalski/skills, HEAD `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28T18:25:28Z); GitHub API: MIT (`LICENSE`: "Copyright (c) 2026 Emil Kowalski"), 51,578 stars, 2 open issues, not archived (SECONDARY metrics, stars not used for ranking). The installed `apple-design` is pinned to the same commit, so the whole repo at that SHA is the natural pin.
- Skills at that SHA (path `skills/<name>/SKILL.md`): animate (+RECIPES.md), animate-expo, animation-performance, animation-vocabulary, apple-design, ask-sonner, break-ui (+CATALOG.md), emil-design-eng, find-animation-opportunities, improve-animations (+AUDIT.md, PLAN-TEMPLATE.md), mobile-native, pick-ui-library, prototype (+PICKER.md), remove-animations, review-animations (+STANDARDS.md), write-swift. Top-level `performance-cheatsheet.md`, `ui-cheatsheet.md`.
- I read all SKILL.md files except animate-expo, ask-sonner, mobile-native, write-swift (not fetched; irrelevant: React Native, Sonner, Swift) in `D:\Temp\toolkit_research\r2\emil\`.
- Key rules (quoted briefly from emil-design-eng/STANDARDS): "Never animate keyboard-initiated actions" (100+/day: "No animation. Ever."); "Never use ease-in for UI animations"; "UI animations should stay under 300ms"; use `@starting-style` instead of mount-effect hacks; reduced motion "means fewer and gentler animations, not zero" (keep opacity/colour, remove movement); specify exact properties, never `transition: all`; `will-change` "is the last step, not the first"; remove-animations: stagger cap 30 ms/item, spinners under 300 ms are glitches.
- Claude Code compatibility: standard SKILL.md frontmatter (`name`, `description`; `disable-model-invocation: true` on review-animations, pick-ui-library, prototype). No scripts, no hooks, no MCP. README install is `npx skills@latest add emilkowalski/skills` (mutable HEAD; do not use, pin instead).
- Directive blocks found (analysed, not followed): (a) nearly every skill has "Initial Response: when first invoked without a specific question, respond only with ..." (harmless persona greeting; suppresses useful output, so neutralise); (b) emil-design-eng "MUST use a markdown table with Before/After columns" for reviews (format directive; compatible with docs but can conflict with a project report format); (c) `pick-ui-library` step 3 "install/wire it up" and a table recommending recharts, motion, NumberFlow, zustand, Liveline, etc.; (d) `improve-animations` Hard Rule 2: "No mutating operations" (aligned with project rules); (e) `remove-animations` rule 3: "Ask before removing a dependency" (aligned). No network calls, shell commands, telemetry or permission requests in the SKILL.md files themselves; the only URLs are references (easing.dev, easings.co, emilkowal.ski, library homepages).
- Fit with React 18 + Vite + plain CSS: strong. Most guidance is CSS (`cubic-bezier`, `transform`, `@starting-style`, WAAPI); library sections (Framer Motion `useReducedMotion`, Sonner, Vaul) are optional and must be ignored.
- Gaps vs PhantomCache: no provenance concept; "Demo data / Worst case toggle" in `break-ui` would create synthetic data in the UI (see conflicts); motion guidance is interaction-centred, not data-state-centred (no guidance on animating measured series or run progress). Persona is consumer-product polish ("Beauty is leverage"), whereas the product rule is purposeful motion only.
- Overlap with `apple-design`: apple-design covers springs, gestures, translucent materials, typography, reduced motion; emil-design-eng covers easing, durations, frequency-based motion budgeting, enter/exit, review. Overlap on reduced-motion; complementary otherwise. Neither covers data-viz motion.
- Classification and per-skill verdict:

| Skill | Class | Unique contribution / note |
|---|---|---|
| animation-performance | RECOMMENDED FOR ADOPTION | Composite-only properties, main-thread-busy diagnosis, "name the cause before changing code"; fits a UI with live polling |
| review-animations (+STANDARDS) | RECOMMENDED FOR ADOPTION | Strict review checklist; manual-invoke only |
| remove-animations | RECOMMENDED FOR ADOPTION | Subtraction audit; aligned with "no decoration for its own sake" |
| find-animation-opportunities | RECOMMENDED FOR ADOPTION | Read-only, restraint-first, states what not to animate |
| emil-design-eng | USEFUL REFERENCE ONLY (adopt as reference text, large 27 KB) | Core numbers; greeting and format directives to neutralise |
| animate (+RECIPES) | USEFUL REFERENCE ONLY | Decision order is good; it writes implementations, so use only under a task brief |
| improve-animations | REQUIRES FURTHER INVESTIGATION | Read-only audit plus plan templates; produces plans for other agents; check against multi-agent rules |
| animation-vocabulary | USEFUL REFERENCE ONLY | Vocabulary for briefs |
| apple-design | ALREADY COVERED BY EXISTING SKILL | Installed, same SHA |
| break-ui | USEFUL REFERENCE ONLY | Worst-case data idea valuable for long run names/huge counts; the in-UI "Demo data" toggle is NOT acceptable unless labelled SIMULATED and kept out of production views |
| pick-ui-library | NOT SUITABLE | Recommends installing dependencies; contradicts approval rule |
| prototype | NOT SUITABLE | Builds visual variant switchers; scope creep |
| mobile-native, write-swift, animate-expo, ask-sonner | NOT SUITABLE | Out of scope |

- PROPOSAL (not executed): install the four RECOMMENDED skills as pinned copies, not via `npx`:
  `git -C <tmp> clone https://github.com/emilkowalski/skills` is forbidden in the repo, so fetch files only: for each of `animation-performance`, `review-animations`, `remove-animations`, `find-animation-opportunities`, download `https://raw.githubusercontent.com/emilkowalski/skills/a81dbca07f2217b7103e510c0f13f8d5b017afc9/skills/<name>/SKILL.md` (and `review-animations/STANDARDS.md`) into `.claude/skills/<name>/`, plus `LICENSE` from the same SHA saved as `.claude/skills/<name>/LICENSE` (MIT notice must travel with copies). Then append a short local overlay (section 9) rather than editing the upstream text. Verify against the way `apple-design` was installed (not inspected here; open question).

## 3. Candidate: greensock/gsap-skills and GSAP

### 3.1 The skills repo
- https://github.com/greensock/gsap-skills HEAD `aed9cfd3277740755f6bfc1155c7aa645403b760` (commit 2026-04-21; repo pushed 2026-07-29); API: MIT, 17,521 stars, 14 open issues. `.claude-plugin/plugin.json` v1.0.0 (`"skills": "./skills/"`), also Cursor/Copilot/Gemini manifests, `AGENTS.md`, `CLAUDE.md` (content: the single line `AGENTS.md`, i.e. a pointer), `examples/` (react, vue, nuxt, vanilla).
- Skills: gsap-core, gsap-timeline, gsap-scrolltrigger, gsap-plugins, gsap-utils, gsap-react, gsap-performance, gsap-frameworks. The ones I read (core, plugins, react, performance) instruct `npm install gsap` / `@gsap/react`, and gsap-plugins says "Do not generate an .npmrc with a GreenSock auth token ... or tell users to sign up for Club GSAP". No shell hooks or telemetry found in the files read; `examples/*/package.json` not read.
### 3.2 GSAP licence (VERIFIED-FETCHED)
- npm registry `gsap@3.15.0` `license` field: "Standard 'no charge' license: https://gsap.com/standard-license"; `@gsap/react@2.1.2`: "SEE LICENSE AT https://gsap.com/standard-license". The gsap-skills README says "GSAP is 100% free including every plugin" (after the Webflow acquisition).
- https://gsap.com/standard-license/ (fetched via summariser, effective 2025-04-30, modified 2025-05-30): commercial use covered at no charge, all formerly members-only plugins included; proprietary (Webflow retains ownership, not stated as open source or MIT); Prohibited Uses include using GSAP in tools that let users build animations visually without code if that "materially assists in creating a solution that competes with Webflow's visual animation building capabilities"; no reverse engineering to build competing products; AI-generated GSAP code not a Prohibited Use. Caveat: summariser output, read the licence text directly before any decision.
- Implication: a free-of-charge proprietary licence in an open-source repository is not an OSI-open dependency; the repo's own licence status (check `Cargo.toml`/LICENSE of PhantomCache, not examined) determines whether this is acceptable. Also, PhantomCache's Phantom Lab is not an animation builder, so the Prohibited Use is unlikely but must be re-read.
- Size: `gsap` npm unpacked 6.26 MB (all plugins and typings; the bundled-and-gzipped cost for core-only is vendor-claimed ~tens of KB, UNVERIFIED, no measurement made).
- Value for PhantomCache: low. Data-state transitions are CSS-sufficient; GSAP's strengths (ScrollTrigger, SplitText, MorphSVG, complex timelines) target marketing pages. Classification: **NOT SUITABLE for adoption; USEFUL REFERENCE ONLY** for timeline/sequencing thinking (gsap-timeline) and the gsap-performance checklist, which duplicates animation-performance. Reconsider only if a scripted multi-step explainer (Simulation page) cannot be expressed with CSS animations and WAAPI, and only after licence review and approval.

## 4. Candidate: cathrynlavery/diagram-design

- https://github.com/cathrynlavery/diagram-design HEAD `0cf73c3fcc56667ad91a9a04c439ac6ccfabc9d1` (2026-10-27T20:01:34Z); API: MIT, 63,118 stars, 46 open issues, not archived (SECONDARY). Plugin v2.6.136; `skills/diagram-design/SKILL.md` (30,874 bytes, `license: MIT`, version 2.6), multi-host manifests (.claude-plugin, .codex-plugin, .factory-plugin, .qoder-plugin, .agents), `commands/` (doctor, export-diagram, import-drawio, import-excalidraw, import-mermaid, profile), `docs/adr/` (16 ADRs), `scripts/` (about 46 verify-/lint-/build- Python files plus vendored icon SVGs), `PRIVACY.md`, `SECURITY.md`, `THIRD_PARTY_LICENSES.md` (Tabler MIT, Simple Icons CC0, log-z/logos MIT, Devicon).
- Provides: 45 diagram types as self-contained HTML with inline SVG (architecture, sequence, state machine, timeline, swimlane, data flow, deployment, dependency, heatmap, dumbbell, slopegraph, waterfall, box plot, etc.), semantic patterns, optional animation (`none` default, `reveal`, `step`, `loop`; ADR 0003: reveal is the only sanctioned autoplay), import from draw.io/Mermaid/Excalidraw by "redraw, never convert", label-geometry rules (arrow labels 14 chars max, 6-10 px mask gap), accessible SVG contract (`<title>`/`<desc>` with prefixed IDs), complete static frame without JS, reduced motion shows static frame.
- Directive/behaviour findings: section 0 "style guide gate": on first diagram it must "pause and ask the user" (six options incl. website URL fetch, which makes the agent fetch pages); philosophy "highest-quality move is usually deletion", "Target density: 4/10", "Above 9 nodes, it's probably two diagrams", "Coral is editorial ... 1-2 focal nodes". Needs `python3 scripts/self_check.py` and PNG export needs a headless browser (`references/export.md`, not read). PRIVACY.md: no telemetry or accounts, but generated diagrams load fonts from Google Fonts (`fonts.googleapis.com`, `fonts.gstatic.com`) at view time, a third-party request.
- Compatibility with this project: output is standalone HTML/SVG, not React components; useful as design source only. Colour/typography come from its own style guide unless overridden by profile; PhantomCache tokens and provenance badges would have to be mapped manually.
- Conflicts: (a) Google Fonts fetch vs self-hosted/offline posture; (b) density target 4/10 and node limit 9 vs cluster/lab diagrams that may need more elements (split into several figures is acceptable); (c) accent-colour "editorial focal" semantics could be mistaken for provenance colours; (d) the first-run gate asks questions and optionally fetches websites (not allowed in unattended agents; neutralise by pre-supplying a project profile); (e) no concept of illustration-vs-live labelling.
- Value: moderate as a source of conventions (label masks, accessible title/desc IDs, static-first animation, step/reveal modes for Simulation explainer). Cost of installing: a 30 KB instruction file triggers on many diagram requests; 46 scripts of Python that I did not audit (only listed). Classification: **USEFUL REFERENCE ONLY**, with REQUIRES FURTHER INVESTIGATION if the lead wants to install (audit `scripts/` and `references/export.md`, pin 0cf73c3f, preserve `LICENSE` and `THIRD_PARTY_LICENSES.md`, set a project profile with self-hosted fonts). Not examined: references/*.md type files, `animation.md`, `scripts/*.py` contents.

## 5. Motion engineering: platform techniques and libraries

All platform claims are LITERATURE/KNOWLEDGE unless marked.
- **CSS transitions/keyframes + WAAPI**: sufficient for PhantomCache (state changes, progress, connection states). Composite-only properties (`transform`, `opacity`) avoid layout/paint (documented in web.dev "Animations guide", LITERATURE). Never animate `width/height/top/left` on data bars; for bar fills use `transform: scaleX()` with `transform-origin: left`, but note that scaled text/strokes distort, so scale only the fill element (the existing `bars__fill` uses `width` percentages: a candidate for a measured change, not a defect).
- **`@starting-style`**: MDN: "Baseline 2024, Newly available" (VERIFIED-FETCHED, since Aug 2024). Replaces mount-flag effects for entry animations; provide fallback = no animation.
- **`linear()` easing**: MDN page for `@starting-style` did not mention it; support UNVERIFIED here (knowledge: Baseline 2023). Use to encode spring-like curves in CSS without a library; generate points offline; a reduced-motion variant drops movement.
- **View Transitions API**: same-document `document.startViewTransition()` (MDN page fetched, Baseline status not shown in extracted text, UNVERIFIED). Useful for route/tab morphs, but `view-transition-name` uniqueness and focus/live-region handling need testing; for a dense dashboard that polls, avoid wrapping frequent state updates. Reduced motion: set `::view-transition-old/new(*) { animation: none }`.
- **`animation-timeline` / scroll-driven animation**: not recommended; Baseline uncertain; decorative.
- **Motion (Framer Motion)**: npm `motion@14.8.0` MIT, 937 KB unpacked, 2 deps; `framer-motion@14.8.0` 7.96 MB unpacked, 3 deps (registry, VERIFIED-FETCHED). Gives layout animations/springs/`useReducedMotion`; React 18 compatible (not verified for 14.x peer ranges here: UNVERIFIED). Bundle size depends on LazyMotion (vendor claims a few KB, UNVERIFIED). Not needed.
- **@formkit/auto-animate** 0.10.0 MIT, 59 KB unpacked, 0 deps: automatic list enter/exit; fails "purposeful motion" test for list churn on a live table.
- **GSAP**: section 3.
- **Number tweening** (NumberFlow): animating live metrics conflicts with the rule that values are labelled and exact; a value that interpolates through numbers never measured is a provenance risk. Do not tween measured numbers; fade a "updated" indicator instead.
- Performance (documented costs, nothing measured here): each animated element on its own compositor layer consumes memory; `will-change` over-use increases memory (MDN, LITERATURE); SVG animation with many nodes triggers paint per frame, so keep animated SVG node counts low; transitions on `filter` or `backdrop-filter` are expensive.

### Motion rules for PhantomCache (PROPOSAL, merges Emil + project rules)
1. Motion must answer one of: where did this come from / what changed / what is in progress / what is the state transition. Otherwise delete.
2. Never animate measured values; animate only the container or an "updated" cue. Interpolation through unmeasured values is forbidden.
3. Live-polled surfaces (gateway state, run progress): no entrance animation per poll; state change cue only on transitions.
4. Reduced motion: remove movement and looping; keep opacity/colour/outline changes and textual state. Any indeterminate looping indicator (e.g. `pc-slide`) needs a static equivalent plus visible text (current: see `app.css:277`).
5. WCAG 2.2 (LITERATURE): 2.2.2 Pause, Stop, Hide for motion over 5 s that starts automatically; 2.3.3 Animation from Interactions is AAA but adopt; 2.5.8 target size 24 px; 1.4.11 non-text contrast 3:1 for state indicators.

## 6. Materials and glass

- **Apple HIG Materials / Liquid Glass**: reference for hierarchy (material communicates layer, not decoration). Apple's content is native-platform; web translation is the apple-design skill (already installed, includes "translucent materials and depth").
- **`backdrop-filter`**: MDN "Baseline 2024: Newly available" (VERIFIED-FETCHED). "Backdrop root" caveat (VERIFIED-FETCHED): `opacity < 1`, `filter`, `mask`, `clip-path`, `mix-blend-mode`, `will-change` of these on an ancestor limit what a child's blur sees; so glass inside an animated opacity container silently loses its backdrop.
- **`prefers-reduced-transparency`**: MDN: "Limited availability", experimental (VERIFIED-FETCHED). Many users with the setting enabled may be undetectable; therefore any glass must be safe when the preference is not detected: opaque-enough base fill so that text contrast passes (WCAG 1.4.3/1.4.11) over the worst-case backdrop, not the typical one. The project already reacts to it at `tokens.css:155`.
- **"Liquid glass" CSS (SVG `feDisplacementMap` in `backdrop-filter`)**: SECONDARY (search summaries of practitioner posts and WordPress plugin pages, not opened): refraction works in Chromium only; raw turbulence displacement is jagged; stacked animated glass layers stutter on phones; opaque fallback required. No primary source verified, no maintained open-source reference found that I can recommend. Classification: **NOT SUITABLE** for the dense data surfaces.
- Honest cost/value: glass over live charts reduces legibility and makes provenance badges hard to keep at 4.5:1; blur forces readback of the backdrop each frame on scroll or underlying change (documented cost class, not measured here). Defer actual numbers to the DD agent's measured backdrop-filter results (file `docs/design/phantomcache_design_direction_v2.md` did not exist when I finished; see section 10).
- Where a material may be defensible: one persistent non-data layer (top bar or a modal scrim) with an opaque fallback and a single blur radius, never beneath tables or charts, never animated, never containing provenance labels directly over variable content.
- Rule (PROPOSAL): materials express layer hierarchy only. Maximum one blurred layer visible at a time; no `backdrop-filter` inside scrolling or animating-opacity ancestors; no SVG displacement; tokens `--material-*` must define an opaque fallback; test contrast against the lightest and darkest data states.

## 7. 3D and spatial

- three 0.187.0 MIT, 21 MB unpacked, 0 deps; @react-three/fiber 9.8.2 MIT, 2.4 MB unpacked, 10 deps (registry, VERIFIED-FETCHED). WebGL/WebGPU adds: canvas with no inherent accessibility tree, GPU memory and context-loss handling, keyboard and focus alternatives to build, bundle weight, and headless-test difficulty.
- Value for a dense engineering app: none identified. Cluster topology and cache hierarchy are 2D graphs; depth encodes nothing. 3D charts distort comparison (one community skill states "NEVER use 3D charts", SECONDARY; consistent with standard visualisation guidance, LITERATURE).
- CSS 3D transforms (`perspective`, `rotateX`): acceptable only for a static isometric illustration of layers (e.g. gateway / cache / origin stack) that is labelled ILLUSTRATION; compositing cost modest if static; no animation of perspective.
- Classification: three.js / r3f: **NOT SUITABLE** (no WebGL dependency without proof of value). Cobe globe (listed by Emil's pick-ui-library): NOT SUITABLE (decorative). Static 2.5D SVG: **USEFUL REFERENCE ONLY**.

## 8. Diagrams and data visualisation

### 8.1 Diagram tooling (registry facts VERIFIED-FETCHED, others as marked)
| Tool | Facts | Verdict |
|---|---|---|
| Hand-built SVG (React components) | zero dependency, themeable by tokens, can carry `<title>/<desc>`, labels and provenance | PRIMARY for UI illustrations |
| Mermaid 12.2.0 | MIT, 122 MB unpacked, 23 deps; text-to-diagram, good for docs (GitHub rendering of mermaid fences: knowledge, UNVERIFIED here) | USEFUL for docs/ only, not UI runtime |
| D2 | Go binary; not fetched | UNVERIFIED, REQUIRES FURTHER INVESTIGATION |
| Excalidraw 0.18.1 | MIT, 46.8 MB unpacked, 31 deps | NOT SUITABLE for UI; hand-drawn look conflicts with engineering precision |
| @xyflow/react (React Flow) 12.12.0 | MIT, 1.2 MB, 3 deps; interactive node graph | NOT SUITABLE: invites "live topology" impression; use only if a real, measured topology is ever exposed |
| elkjs 0.12.1 | licence EPL-2.0 OR GPL-3.0-or-later | flag; layout engine only; not needed |
| diagram-design | section 4 | USEFUL REFERENCE ONLY |

Conventions to adopt (LITERATURE/KNOWLEDGE, consistent with diagram-design): one idea per node; arrows carry information; solid = synchronous call, dashed = async/optional/return; sequence diagrams for request flow (lifelines: client, gateway, coalescer, cache, origin); state diagrams for lifecycles; timeline swimlane strips for failure/recovery with a labelled time axis (or ordinal "steps" axis when no clock is real). Every diagram that is not a render of measured state carries a visible ILLUSTRATION label and `<desc>` stating that no values are live; no animated "packets" implying real traffic unless driven by REAL events.

### 8.2 Data visualisation libraries (VERIFIED-FETCHED registry)
| Library | Licence | Unpacked | Deps | Note for PhantomCache |
|---|---|---|---|---|
| d3 7.9.0 | ISC | 871 KB | 30 | Use `d3-scale`/`d3-array` modules only if needed; the current "linear scale only" is simple enough in code |
| @observablehq/plot 0.6.17 | ISC | 1.5 MB | 3 | Good grammar for dot plots/faceting; produces SVG with limited a11y; dependency |
| @visx/shape 4.0.0 | MIT | 227 KB | 5 | React primitives; fits React but adds a family of packages |
| vega-lite 6.5.0 / vega-embed 7.3.0 | BSD-3 | 5.9 MB / 0.6 MB | 6 / 8 | Declarative, strong for small multiples; heavy runtime |
| uPlot 1.6.32 | MIT | 545 KB | 0 | Fast canvas time-series; canvas is opaque to assistive tech; only if a series exceeds SVG capacity |
| Chart.js 4.6.1 | MIT | 6.2 MB | 1 | Canvas; limited accessible structure |
| recharts 3.10.1 | MIT | 7.5 MB | 11 | Recommended by Emil's list; generic look; no |

Verdict: no charting library now. Triggers to revisit: more than about 2,000 SVG nodes per chart, or live series updating faster than the polling the backend supports. Neither is evidenced.

### 8.3 Chart design guidance for PhantomCache (LITERATURE/KNOWLEDGE: Cleveland, Tufte, Wilke "Fundamentals of Data Visualization", WCAG; not re-verified here)
- **Paired comparison (Scenario B arms)**: dumbbell/slope plot or paired dot plot with the same axis, difference shown separately with its interval (the existing `EffectChart` does mean with CI; keep centred zero line and the explicit left/right wording). Direction wording must be text, not colour only.
- **Uncertainty**: show interval as a line plus a table column; label what the interval is (confidence level, n, method) from received data; if the backend gives none, show UNAVAILABLE, never a drawn band.
- **Percentile latency**: percentile ladder (p50/p90/p99/p99.9) as dot-and-line on one axis; use log scale only with a stated axis; do not connect percentiles into a smooth "distribution" curve; histogram only if raw buckets exposed (the telemetry histogram has 2,688 buckets per CLAUDE.md, unknown whether exposed).
- **Origin concurrency vs cap**: step-line against a horizontal cap line (labelled with its source) and visible violation markers; sampled steps, not smoothed.
- **Small multiples**: same scales across panels; one panel per scenario or phase; label each with provenance.
- **Timelines**: event strip with ordinal or real time clearly labelled; markers for phase boundaries; for runs, a progress bar with textual "step n of m / unknown" (no fake ETA).
- **Accessibility**: always provide the table view (existing ChartFrame toggle), a text summary (existing `aria-label` summary), colour-independent encodings (shape, label, pattern), 3:1 for graphical objects, no information by hover only, keyboard-focusable data points only if there is a table alternative that already carries it.
- **Zero/UNAVAILABLE states** are first-class marks (existing `UnavailablePanel`); never interpolate across gaps.
- **Community dataviz skills**: SECONDARY (search listing only, not opened): DreambaseAI/skills (`dreambase-visualization-design`), community-access/accessibility-agents (`data-visualization-accessibility`), skills.sh moderndegree/charlieviettq, Smithery kyegomez, daemon-blockint-tech. Install counts are low (about 2 to 132 per the listing); GitHub API for DreambaseAI/anthropics/skills was rate-limited (HTTP 403) so licence/commit not verified. Classification: **REQUIRES FURTHER INVESTIGATION** (read the SKILL.md before any use; treat as untrusted). Claude's bundled `dataviz` skill: it is listed in this session with a description covering form heuristics, a colour formula with a runnable validator, `references/palette.md`; I did not load or read its contents and cannot say whether it is public. It targets artifacts and plotting libraries generically, so its palette/validator must be mapped onto PhantomCache tokens and provenance rules. Classification: **REQUIRES FURTHER INVESTIGATION** (read it, then decide whether its colour validator duplicates `ui/scripts/check-contrast.mjs`).

## 9. Conflicts and neutralisation

| Conflict | Sources | Neutralisation (PROPOSAL) |
|---|---|---|
| Install libraries (recharts, motion, NumberFlow, GSAP) | Emil pick-ui-library; gsap-skills; Emil animate/performance mention Framer Motion | Do not install pick-ui-library; project overlay: "No dependency may be added; propose to the owner with justification" |
| Greeting "respond only with ..." on first invocation | Emil skills | Overlay: ignore Initial Response blocks when a task is given |
| Before/After table review format mandated | emil-design-eng | Overlay: use the project's review format; table optional |
| Demo/worst-case data toggle in UI | break-ui | Allowed only as SIMULATED-labelled test fixture, never shipped in live views |
| Emil "Beauty is leverage", polish-first | emil-design-eng | Subordinate to purposeful motion and WCAG; if a rule argues for motion that has no state meaning, delete it |
| Keyboard actions never animate vs apple-design continuity/interruptibility | Emil vs apple-design | Not contradictory: for keyboard-triggered and high-frequency actions, no motion; for pointer-driven, follow apple-design |
| Number tweening / counters | Emil list, GSAP, Motion | Forbidden for measured values (provenance) |
| Editorial accent colour, density 4/10, 9-node cap | diagram-design | Map accents away from provenance colours; split diagrams |
| First-run question gate and website fetch | diagram-design | Provide a project profile in advance; never fetch brand websites in agent sessions |
| Google Fonts requests | diagram-design | Self-host or use system fonts |
| `will-change` guidance differences | animation-performance (last step) vs common blanket advice | Follow animation-performance |
| Glass aesthetics vs contrast | apple-design materials vs WCAG 1.4.3 and text over live data | Section 6 rule |

Overlay text to store alongside the pinned skills (PROPOSAL): `.claude/skills/<name>/PROJECT_OVERLAY.md` is not recommended (editing outside the skill); better a single project rule file referenced from CLAUDE.md, decided by the lead.

## 10. Representing PhantomCache's actual object lifecycles

Status of inputs: R4's files (`r4_system_objects_and_motion.md`, `r4_lifecycle_and_event_matrices.md`) and `coord/r4.md` findings were empty when I finished; `docs/design/phantomcache_design_direction_v2.md` and `docs/design/remediation/03_validation_accessibility.md` did not exist. The table below uses the objects named in the project instructions (CLAUDE.md sections 5 to 6b, which state they were verified by earlier reading, not re-verified by me) with PROVISIONAL status marks; R4's REAL/DERIVED/SIMULATED/PROPOSED/UNAVAILABLE statuses supersede mine. Re-check when R4 publishes.

| R4 object/flow (provisional) | Meaningful technique | Illustration-only (label) | Needs telemetry (status by R4) | Cost |
|---|---|---|---|---|
| Request coalescing (many callers, one origin fetch) | Static sequence diagram (client x N, coalescer, origin) with fan-in; CSS state tag when a real in-flight count is shown | Animated fan-in/out | Per-key in-flight waiters and coalesce hit counters over time; PROPOSED/UNAVAILABLE until R4 confirms | Low (SVG) |
| Cache entry lifecycle and freshness (fresh, stale-servable up to hard cap, expired, evicted, fetch in flight; generation fencing) | State machine diagram; entry-age bar with TTL and stale cap marked from config values | Any single-entry "life" animation | Per-entry age, generation and fetch state are not exposed unless R4 shows it; note default TTL not wired (CLAUDE.md) so never draw the configured value as effective | Low-medium |
| Distributed invalidation (epochs, peers, anti-entropy) | Sequence/timeline diagram of epoch advance and peer propagation, ordinal axis | Cluster topology drawings; moving "messages" | Per-peer ack/lag timestamps; effectiveness not established (claims ledger C6 per CLAUDE.md); UNAVAILABLE | Medium; high misleading-risk |
| Workload patterns (Zipf, bursts, key sets) | Small multiples of the generator's declared parameters (SIMULATED label) | Traffic animation | Realised request series from the generator (if the lab API exposes it) | Low |
| Experiment comparison (Scenario B paired arms) | Dumbbell/paired dot plot with interval, table toggle, per-phase small multiples, origin concurrency vs cap step-line | None needed | Per-arm samples, n, interval method, cap values and violation counts; partly present in LabCharts props | Medium; core value |
| Adaptive control / PIE (observe, detect, adapt, explain) | Decision timeline of recorded decisions with evidence references, if the backend records them | Control-loop diagram | Decision log with inputs; today PIE is a legacy threshold stub (`pie: None`), so anything beyond an ILLUSTRATION loop diagram is UNAVAILABLE | Medium; blocked on backend |
| Run progress | Determinate/indeterminate bar with textual state | Spinner as decoration | Run phase events with timestamps | Low |
| Gateway connection states | State chip with transition cue only on change (opacity/outline), reduced-motion identical text | Pulse for "alive" | Connection state is a client observation (DERIVED), keep separate from server health | Low |

Rules that follow: (1) a diagram not generated from received data is labelled ILLUSTRATION in text next to the figure and in `<desc>`; (2) no animation may imply liveness, throughput or topology unless driven by REAL events whose timestamps are shown; (3) cluster/intelligence illustrations are static; step-through (reveal/step modes in the diagram-design style) is acceptable for the Simulation explainer if labelled SIMULATED or ILLUSTRATION; (4) 3D adds nothing to any row.

## Requests to R4 (open)
1. For each object above: list of events with names, emission mechanism, rate (per second typical/max), and cardinality (keys, peers, arms) so charts can budget SVG nodes. Assumed if unanswered: ≤ 1 update/s per surface, ≤ 200 marks per chart.
2. Do percentile histograms leave the server as raw buckets (2,688 log-linear) or only pre-aggregated percentiles? Assumed: pre-aggregated only, so no histogram charts.
3. Which timestamps exist (server wall clock, monotonic, per-sample) for origin concurrency vs cap series and for invalidation propagation? Assumed: sample index only, so ordinal axes.
4. Does Scenario B expose per-sample or only per-arm summaries with intervals (method, level, n)? Assumed: summaries only.
5. Is any per-entry or per-key state observable (age, generation, in-flight)? Assumed: no.
6. Any decision log from the adaptive controller? Assumed: none.

## Replies to R4
None requested yet of R2. Standing answer for later requests: animation is feasible with CSS/WAAPI at low cost for state transitions of ≤ 20 elements; per-event particle-style animation is not recommended; anything animating more than about 200 SVG nodes needs a measurement first.

## 11. Methodology and limits

- Dates: all fetches 2026-10-30. Tools: GitHub API and raw.githubusercontent.com via curl into `D:\Temp\toolkit_research\r2\{emil,gsap,dd,npm,...}`; npm registry `latest` JSON; WebFetch for MDN and gsap.com (summarised by a small model: figures not quoted verbatim from raw HTML); WebSearch (two queries: agent skills for data visualisation; liquid glass CSS backdrop-filter recipes).
- Not verified: GSAP gzip bundle sizes; Motion/LazyMotion sizes; React 18 peer ranges for motion 14.x; `linear()` and View Transitions Baseline status; anthropics/skills contents (GitHub API rate-limited); DreambaseAI, community dataviz skills; D2; Apple HIG pages (not fetched); contents of `dataviz` bundled skill; diagram-design `references/*`, `scripts/*` source and install behaviour; emil skills animate-expo, ask-sonner, mobile-native, write-swift; how `apple-design` was originally installed; PhantomCache repository licence; DD agent and R4 outputs (absent at writing time).
- No code was executed from downloaded material; no installs; no repo files other than this file and `coord/r2.md` were written. Python was not available as `python` on PATH, so no Python was used; JSON was inspected with grep and node reading registry JSON only.
- Stars, issues and similar API metrics are context, not ranking criteria.

## 12. Final proposals (for lead decision; none executed)
1. Adopt pinned subset of Emil skills (animation-performance, review-animations, remove-animations, find-animation-opportunities); install by pinned raw file copy with LICENSE; apple-design unchanged.
2. Do not add any animation, chart, 3D or diagram runtime dependency; revisit with measured triggers.
3. Adopt the motion rules in section 5 and the materials rule in section 6 as project rules.
4. Treat diagram-design and GSAP skills as reference only; extract conventions into a local diagram spec.
5. Read the bundled `dataviz` skill and DD agent measurements before finalising the dataviz authority.

## 13. Update after reading R4 coord findings and DD v2 (2026-10-30, late)

Sources read: `coord/r4.md` (R4 findings 1 to 5, FACT with file:line by R4, not re-verified by R2) and `docs/design/phantomcache_design_direction_v2.md` (453 lines). R4's main files and `03_validation_accessibility.md` were still absent; DD v2 sections 3 and 12 still contained the placeholders `[[PERF_SUMMARY]]` and `[[PERF_FULL]]`, so NO measured backdrop-filter or 3D numbers were available to me.

R4 facts applied to section 10 (supersede the provisional marks there):
- Coalescing: only a REAL aggregate (`coalesced_waiters` vs misses) exists; per-fetch fan-in and join events are UNAVAILABLE. A per-fetch "N callers converge" animation is therefore illustration-only (static, labelled ILLUSTRATION). A REPLAYED per-request view (`x_cache_status` MISS/COALESCED/HIT) is possible only for retained Scenario B runs and must be labelled REPLAYED with the run id.
- Hot keys: schema 2 reports `unavailable (not_implemented)`: render the existing UnavailablePanel, never a key-skew chart.
- Adaptive control: the only real intelligence in the serving path is the origin concurrency gate (cap, in_flight, dispatched, rejections, controller_steps). Visualise exactly that (step-line of in_flight against cap, rejection markers); a PIE/decision-loop diagram is ILLUSTRATION or PROPOSED only.
- Cluster: GET is local-only, "weighted" hashing has constant weight, NODE_FAILURE is a logical flag: topology/failure-recovery diagrams must be labelled ILLUSTRATION; no animated message flow.
- Retained evidence provides a REAL loopback, debug-labelled time series (`gateway_snapshots.jsonl` about 250 ms cadence; `client_requests.csv`; `origin_requests.csv`): the only source that justifies a timeline/step-line chart. Label as REPLAYED, loopback, debug build.

Reconciliation with DD v2 (positions recorded, lead to decide):
- DD v2 allows glass on floating chrome only (header, rail, run bar, popovers, dialogs), solid for all data, worst-case-backdrop contrast guarantee, fallbacks for `@supports`, reduced transparency, and forced colors, budget of at most 2 glass surfaces on phone and 4 on desktop, radius at most 14 px, no animation of blur radius, no CSS 3D, no WebGL. R2 agrees with this scope; R2's section 6 "one blurred layer at a time" is stricter than DD's 4-on-desktop. Evidence for either number is the DD measurement, not yet filled in. R2 defers to DD once numbers exist. Agreement: no 3D, no SVG displacement refraction, no glass over charts or tables.
- DD v2 states `prefers-reduced-transparency` is Chromium-only (snippet, UNVERIFIED by DD); R2's MDN fetch says "Limited availability, experimental" (VERIFIED-FETCHED). Consistent. DD's guarantee 5 (contrast safe without relying on the query) matches R2's advice.
- Motion: DD lists durations 100/150/200/300 ms and keyframes `pc-*`; R2's Emil-derived rules (no keyboard animation, 300 ms ceiling, no ease-in on UI) fit. Not checked: whether any existing control uses ease-in.
- Diagrams: DD's "ghost outline" mark as the only 2D depth device aligns with R2's static 2.5D illustration conclusion.

Replies to R4: none was addressed to R2 in `coord/r4.md` at this time.

## Cross-agent exchanges
R2 read `coord/00_coordination.md`, `coord/r3.md`, `coord/r4.md` and DD v2. R3 expects R2 to say whether motion uses WAAPI, CSS or a library: answer, CSS and WAAPI only, no library (section 5). R3's candidate conflict (R1/R2 skills suggesting Tailwind, framer-motion or shadcn vs the no-dependency rule) is confirmed for Emil's `pick-ui-library` and gsap-skills; neutralisation in section 9. Section 10 "Requests to R4" are open; R4's findings partly answer 1, 2, 5 and 6 (no per-fetch coalescing events; hot keys unavailable; no decision log; retained runs provide about 250 ms snapshots), recorded in section 13.
