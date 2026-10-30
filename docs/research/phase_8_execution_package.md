> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work attributed to M8 exists (Lab UI; three browser verification rounds under `docs/demo/browser_verification/`), but there is no M8 progress log or completion report, and owner decision O8-0 on the agent-derived Stages A to F is unresolved. No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 8 Execution Package — M8: UI/UX Excellence, Apple-Inspired Design System & Interaction Quality

**Status: PROPOSED — not authorized.** Stage-wise authorization (each stage by the owner in writing): **Stage A** (audit) after G-M1/Phase-1 UI work; **Stage B** (direction, artifacts, skill decision) after Stage A; **Stage C** (design system) after Stage B approval **and** G-M2 (telemetry schema stable); **Stage D** screens as their data contracts pass their gates (Overview/Traffic/Origin/Freshness: G-M2; Cluster: G-M3; Lab: G-M4; Intelligence: G-M5); **Stage E/F** verification and documentation continuous. Gates **U1** (before G-RELEASE if the owner confirms **D-8**) and **U2** (after G-MVP). Common rules: `phase_execution_packages_index.md` §4.
**Note on the source request:** the M8 addendum text provided to the research lead ended mid-sentence ("Stage A — Audit and evidence — Inventory existing screens and their…"). Stages A–F below are derived from every requirement stated before the cutoff; **the owner should compare them with the intended remainder and amend** (decision O8-0).

## 1. Purpose, scope, non-goals
**Purpose.** A premium, coherent, engineering-grade interface *inspired by Apple's design discipline* — not an Apple clone and not a generic dashboard template — that is **honest by construction**: measured, derived, simulated, inferred and unavailable information are always distinguishable; visual polish never disguises missing telemetry. **Scope:** React/Vite UI (`ui/`): information architecture, design tokens, theming, components, charts/tables/timelines, screens for real workflows, interaction and motion policy, accessibility, UI performance/robustness, usability evaluation, design documentation. **Non-goals:** new backend capabilities or endpoints (a screen needing one is marked *proposed UX requiring backend support* and routed to the owning phase), native apps, marketing site, use of Apple trademarks/imagery/SF Symbols/SF fonts (their licenses restrict use to Apple platforms — the UI uses a system font stack and the open-source icon set), literal copying of iOS/macOS conventions.

## 2. Starting state (read-only audit, 2026-10-29, before Phase 1 UI work — PROVISIONAL, re-audited in Stage A)
`ui/src/` contains `App.jsx` (524 lines, single component) and `main.jsx` (9 lines); `index.html` 12 lines; Vite config 9 lines. **No CSS files**: 107 inline `style={{…}}` props; hard-coded dark palette (e.g., `#050607`, orange accent `#FF6B00`) and monospace font. **No router, no state library, no chart library** (charts are hand-built SVG polylines; 2 `<svg>` occurrences); dependencies: `react`, `react-dom`, `lucide-react` (declared but **0 uses**), `vite`, `@vitejs/plugin-react`. **Accessibility signals:** 0 `aria-*`/`role` attributes, 15 `<button>` elements, no `prefers-reduced-motion`/`prefers-color-scheme`/`prefers-contrast` handling, no semantic landmarks identified, color-coded status. **Data flow:** one `EventSource` to a hard-coded `http://localhost:8085/api/events` plus `POST /api/control` (simulation control); metrics are mixed live/synthetic with client-side arithmetic and fabricated panels (see `ui_metric_provenance.md`). Phase 1 (AG, in progress) replaces the data layer with schema-v2 provenance components, removes fabricated panels, adds `?view=sim-demo` and temp-dir builds — Stage A must audit the *post-Phase-1* UI, not this snapshot.

## 3. Open-source Apple design skills — evaluation record (verified 2026-10-29)
Method: research agent read primary GitHub files (no installation, no scripts executed); the research lead independently re-verified licenses, default branches and HEAD commits via the GitHub API on the same date.
| | `emilkowalski/skills` | `dickwu/apple-design-skill` | `ykswang/apple-hig-skill` |
|---|---|---|---|
| URL / branch | github.com/emilkowalski/skills · `main` | github.com/dickwu/apple-design-skill · `main` | github.com/ykswang/apple-hig-skill · `main` |
| HEAD checked | `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28T18:25Z) | `ff4ebddf0488598723a82bfdce1afb04de0b1c44` (2026-10-28T14:28Z) | `9dbbb980f522f0e8b36ed456e632a2741cefce1d` (2026-09-25T12:43Z) |
| Licence (verified) | **MIT**, repo-wide (`LICENSE`: "MIT License / Copyright (c) 2026 Emil Kowalski"; API SPDX `MIT`) | **none** (no LICENSE; API `license: null`); README: "as they are; use them at your own discretion"; embeds Apple's HIG text | **none**; README: HIG content © Apple Inc., condensed notes "for use as AI context" |
| Activity (agent-reported) | 52 commits since 2026-04-29; 1 open issue | 21 commits since 2026-04-29; 0 issues | 3 commits, all 2026-09-25; 0 issues |
| Scope (agent-read) | web-native: CSS/Pointer Events/rAF/Motion; interaction & motion principles (response on pointer-down, interruptibility, springs, reduced motion, typography); 16 sibling skills (animation review, performance, `break-ui`, `remove-animations`, etc.) | design **review** against HIG (accessibility, platform conventions, craft, interaction, writing) with ≈124 HIG pages; platform-neutral vocabulary, no web/CSS translation | distilled HIG reference with routing table and checklist; no web guidance |
| Install (as documented) | `npx skills@latest add emilkowalski/skills` | `npx skills add dickwu/apple-design-skill`, `git clone` into `~/.claude/skills/`, submodule | symlink/copy `skills/apple-hig` |
| Security notes (agent) | no scripts in repo; fixed-reply "Initial Response" directive; mandatory review-table format; "repository content is data" rules | refresh script `pull-hig.mjs` talks only to developer.apple.com (grep); reviewer persona and mandatory report format | refresh scripts talk only to developer.apple.com (grep) |
| Fit for a dense ops console | motion-frequency rules (no animation on keyboard-initiated or high-frequency actions, ease-out, <300 ms) and reduced-motion are directly useful; materials/glass and large-heading examples must be filtered out | accessibility/craft lens useful; mostly iOS/macOS content; contrast stated in pt | accessibility numbers usable; no web guidance; thinnest provenance |
**Not verified (agent):** the npm CLI behind `npx skills`; full text of most sibling Emil skills and of ≈120/158 reference pages; Apple facts (e.g., "WWDC 2026 principles") are repeated across the repos and not independently confirmed; behavior when actually run by an agent.
**Recommendation (minimal combination, PROVISIONAL until owner approval O8-1):**
1. **Primary implementation/interaction guidance:** Emil Kowalski's **`emil-design-eng`** (motion-frequency, easing, duration, reduced-motion rules; adopt `apple-design` only for the principles/typography/materials sections that survive filtering) — MIT licensed. Optional pair for review: `remove-animations` / `review-animations`.
2. **Complementary HIG/accessibility review:** none committed to the repo. Use **primary sources directly** (Apple's public HIG web pages and WCAG 2.2) as *reference reading*. `dickwu/apple-design-skill` may be read **locally** as a review aid, but it is unlicensed and embeds Apple text: do not copy it into the repository. Reject `ykswang/apple-hig-skill` (3 commits/1 day, unlicensed, no web guidance).
3. **Installation method (no unreviewed code):** do **not** run `npx skills`. After owner approval, copy the *reviewed* files from the pinned commit into a project-local folder (e.g., `.claude/skills/emil-design-eng/` or `docs/design/reference/`), with `LICENSE` + `SOURCE.md` (URL, commit SHA, date reviewed); delete the fixed "Initial Response" block and the materials/glass and marketing-heading guidance; re-review before bumping the SHA. Whether AG's environment consumes `SKILL.md` skills is **unverified** (the Emil README does not list supported agents) — the content is plain Markdown and can be used as a reference document.
4. Existing environment skills (`dataviz`, `artifact-design`) may inform chart/token work but are not project dependencies.
**No installation or dependency addition is authorized by this document.**

## 4. Design direction (principles translated for a data-dense web console)
**Audience:** infrastructure engineers and researchers using mouse + keyboard, long sessions, often dark environments, high information density. **Principles → practice:** *clarity and deference* → content-first, restrained chrome; *hierarchy* → one type scale, tabular numerals for metrics, consistent alignment; *immediate feedback* → acknowledge interaction within one frame/≈100 ms, optimistic UI only where truthful; *interruptible, purposeful motion* → motion only to explain state change; **no animation on keyboard-initiated or high-frequency actions (live-updating values never animate between ticks)**; durations < 300 ms, ease-out; `prefers-reduced-motion` honored (cross-fade/instant); *spatial consistency* → stable panel positions, predictable drill-down; *materials* → at most one translucent layer (e.g., sticky header) with a solid fallback (`prefers-reduced-transparency`/`prefers-contrast`); no blur in data regions; *typography* → system font stack (SF on Apple platforms, Segoe UI Variable/system-ui elsewhere), monospace for code-like values, tabular figures; *color* → semantic tokens, status never by color alone (icon + text + position), color-blind-safe palettes with ≤ 6 categorical hues and direct labeling; *dark/light themes* → both provided via tokens (dark default for operations; light for documentation/screenshots) with `prefers-color-scheme` + persisted manual override, contrast verified in both; *density* → compact rows (≈28–32 px), sticky table headers, resizable/collapsible panels, user-selectable density (comfortable/compact) only if justified.
**Avoid:** Liquid Glass/blur everywhere, hero-sized marketing headings, oversized rounded cards, animated gradients, decorative loops, skeuomorphic gloss, any visual treatment that could make simulated or unavailable data look live.
**Data-state system (mandatory for every data region):** loading · empty · success (fresh) · stale (data older than the expected interval; shows age) · unavailable (with reason; never 0) · degraded (partial) · error (with next action) · disconnected. **Provenance system:** badges `MEASURED / DERIVED / SIMULATED / INFERRED / UNAVAILABLE` (icon + text, not color-only), consistent placement, window/as-of on hover and focus, simulated regions bannered; no UI-side computation of new metrics.

## 5. Information architecture and user journeys (screens are contracts — real vs proposed)
Rule: **no invented routes, fields or endpoints.** Each screen lists its data source, the phase that makes it real, and its status at M8 start.
| Screen | Journey | Data (existing / needs backend) | Real after |
|---|---|---|---|
| S1 Overview | "Is the system healthy right now?" origin + cache health, active alerts/states, mode of intelligence | schema v2 counters/derived rates/capabilities (**exists after Phase 1**) | G-M1 |
| S2 Traffic & latency | request rate, hit/miss, latency distributions (HDR-style percentiles; µs resolution after M2) | schema v2; µs latency (M2) | G-M1/G-M2 |
| S3 Cache & memory | accounted bytes vs budget, entries, evictions, rejections; RSS (Linux) | byte accounting (Phase 1); eviction counters | G-M1 |
| S4 Freshness | stale-served fraction/age, revalidation, cap violations | M2 counters (**proposed until M2**) | G-M2 |
| S5 Origin | origin rate, latency, errors, in-flight | origin metrics (Phase 1) | G-M1 |
| S6 Key/workload investigation | hot keys, skew, bursts — only where features exist | M5 feature pipeline (**proposed**; today `hot_keys` is unavailable) | G-M5 |
| S7 Cluster | node states, invalidation status, recovery — only the contract M3 establishes | M3 metrics (**proposed**; node states are inferred/simulated today) | G-M3 |
| S8 Phantom Intelligence | mode, signals, candidates and their evaluation, decisions with reasons/uncertainty/guardrails/rollback state | M5 decision API (**proposed**) | G-M5 |
| S9 Phantom Lab | scenario config, deterministic replay, progress, baseline comparisons, provenance of results | lab result files/viewer (M4, **lab-side support proposed**) | G-M4 |
| S10 Configuration & diagnostics | effective config (secrets redacted), validation findings, health/readiness, version | M7 effective-config report (**proposed**) | M7 work |
| S11 Incident & recovery | guided actions (e.g., disable intelligence) with runbook links | authenticated admin API (M5/M7, **proposed**) | G-M5/M7 |
Cross-cutting: global time-range and refresh controls (only if data supports ranges), a data-source legend, keyboard command palette (proposed UX; low priority), shareable deep links (state in URL).

## 6. Staged work packages
| ID | Title | Stage | Depends on |
|---|---|---|---|
| M8-00 | Entry verification; confirm D-8; amend stages against the owner's full addendum | — | owner authorization |
| M8-01 | UI audit (post-Phase 1) | A | Phase 1 S8 complete |
| M8-02 | Skill evaluation closure and adoption (O8-1) | B | M8-01 |
| M8-03 | Design principles and direction record | B | M8-01 |
| M8-04 | Information architecture, journeys, screen-contract matrix | B | M8-01 |
| M8-05 | Design tokens and theming spec | B | M8-03 |
| M8-06 | Wireframes / interactive prototypes and data-state gallery | B | M8-04/05 |
| M8-07 | **Design review gate (owner approval O8-2)** | B | M8-02…06 |
| M8-08 | Design-system implementation: tokens, base components | C | M8-07, G-M2 |
| M8-09 | Data-visualization components (charts, tables, timelines, status) | C | M8-08 |
| M8-10 | Screens per gate (S1–S5, then S7/S9/S8/S6/S10/S11) | D | M8-09, screen gates |
| M8-11 | Interaction and motion implementation/polish | D | M8-10 |
| M8-12 | Accessibility implementation and verification | E | M8-08… |
| M8-13 | UI performance and robustness verification | E | M8-10 |
| M8-14 | Usability evaluation | E | M8-10 |
| M8-15 | Gate U1 verification | E | S1–S5 done |
| M8-16 | Gate U2 verification; documentation and handoff | F | all screens |

### Stage A — M8-01 Audit and evidence (post-Phase 1, read-only first)
Inventory with file:line evidence: every route/view (the app currently has none — `?view=` modes after Phase 1), components, state management (hooks only), data sources (SSE, REST, files) and how each value is obtained, charting approach, CSS strategy (inline styles), icon usage, build configuration, bundle size, accessibility gaps (landmarks, headings, labels, focus order/visibility, keyboard operability, contrast measured not guessed, color-only status, reduced-motion, live-region behavior for updating values), workflow map (what the real tasks are today), and a *truthfulness audit* (every displayed value classified per `ui_metric_provenance.md`; any residual fabricated element is a defect). **Deliverable:** `docs/design/ui_audit.md` with a defect list (ID, severity, evidence). **Acceptance:** two-reader review; no screen undocumented.

### Stage B — Design direction, artifacts, skill decision (before any major implementation)
**M8-02:** re-verify the §3 facts at adoption time (licence, SHA, scripts), review the files to be copied line by line (injection-style content, behavior overrides), apply the install method of §3 only on O8-1; record `SOURCE.md`.
**M8-03:** `docs/design/principles.md` — the translation table of §4 with *rejected* patterns and the reason for each, plus a "when not to animate" policy.
**M8-04:** `docs/design/information_architecture.md` — the screen matrix of §5 (confirmed against real endpoints at that time), navigation model, URL scheme, task flows (flow diagrams as text/ASCII or Mermaid), error and degraded flows.
**M8-05:** `docs/design/tokens.md` — color (semantic roles: surface, text, border, accent, status {ok, warn, critical, info}, provenance {measured, derived, simulated, inferred, unavailable}), type scale (system stack, tabular numerals, sizes/leading/tracking for data vs prose), spacing (4 px grid), radii (small; no giant rounded cards), elevation (minimal), motion tokens (durations, one ease-out curve, reduced-motion variants), breakpoints, density. Both themes with measured contrast ratios (script output attached): body text ≥ 4.5:1, large text/UI components/graphics ≥ 3:1 (WCAG 2.2 AA), focus indicators ≥ 3:1 against adjacent colors, minimum pointer target 24×24 CSS px (WCAG 2.5.8) with a larger default for primary controls.
**M8-06:** wireframes (low fidelity) and **clickable prototypes with fixture data only, always bannered "PROTOTYPE – FIXTURE DATA"**; a data-state gallery showing each state × each provenance type; chart specification sheets (axes, units, percentile display, uncertainty bands, direct labels, colorblind-safe palette with a validator output, accessible data-table alternative for every chart).
**M8-07 Gate:** owner approves the artifacts (O8-2). Coding of Stage C starts only after approval; Stage A/B artifacts live under `docs/design/` (new), prototypes under `ui/prototypes/` or `docs/design/prototypes/` (O8-3 decides location and any tool).

### Stage C — Design-system implementation
**M8-08:** tokens as CSS custom properties (no new dependency); themes via `data-theme` + `prefers-color-scheme`; base components (Panel, Stack/Grid, Button, IconButton, Tabs, Toolbar, Tooltip/Popover (accessible), Banner, Badge, ProvenanceBadge (extends Phase 1 `Metric`), StatusIndicator (icon+text), EmptyState, LoadingState, ErrorState, StaleIndicator, KeyboardShortcut hint); focus management utilities; a living gallery page under a dev-only route; unit tests (node:test) for logic, screenshot checks for visuals. Component libraries or CSS frameworks are **not** adopted by default (O8-3). Icons: `lucide-react` is already declared (ISC) — used consistently or removed; **no SF Symbols**.
**M8-09:** chart primitives (time series, histogram/percentile strips, sparklines, bar/stacked bars, event timeline, topology diagram), tables (sticky header, sortable, keyboard-navigable grid semantics only where needed, virtualization only if measured necessary, column-wise units/alignment), all built on SVG/CSS with accessible names, descriptions and a "view as table" alternative; streaming updates at ≤ 10 Hz without layout thrash; charting library adoption would be a dependency decision (O8-3) with licence/size review.

### Stage D — Screens and interaction
M8-10 implements screens in gate order, each from the approved wireframe, wired only to **existing** endpoints; unsupported panels show an explicit "requires <backend capability>" state (not a fake). Each screen delivers: states gallery coverage, provenance on every value, keyboard path, URL state. M8-11: motion implementation per the policy (hover/press feedback, panel transitions < 300 ms, no animation for live value updates; reduced-motion variants), interruptibility, focus transitions, skeletons only for first load (never to simulate data).

### Stage E — Verification
**Accessibility (M8-12):** target **WCAG 2.2 AA** for all shipped screens: automated checks (an axe-core-based tool is a dev-dependency ⇒ O8-3; otherwise a documented manual checklist plus contrast script), keyboard-only walkthrough of every task (no traps, logical order, visible focus), screen-reader sanity pass (owner or tester with NVDA/VoiceOver — manual evidence), color-vision simulation review of status and charts, zoom to 200% and reflow at 320 CSS px, `prefers-reduced-motion`, `prefers-contrast`, forced-colors sanity, live-region policy (announce state *changes*, not every tick). **Performance/robustness (M8-13):** render cost at 10 Hz streaming with 1,000-row tables, 8-hour SSE session without memory growth (heap snapshots or measured RSS trend of the page), bundle-size budget (PROVISIONAL: set after baseline), reconnect/backoff behavior, malformed/unknown payload handling (fuzzed fixtures), huge/long/empty values (`break-ui`-style stress fixtures: long keys, 10⁹ counters, empty lists, RTL text), clock skew display, slow network. **Truthfulness tests:** contract tests (live fixture containing `simulated` fails; unavailable has no value; unknown fields not coerced), grep-style forbidden-constant checks, "no client-side metric arithmetic" lint, screenshot fixtures per data state. **Usability (M8-14):** scripted operator tasks (T-A…T-D from Phase 7 plus lab and intelligence tasks), ≥ 3 participants who did not build the UI (owner decides who, O8-4), success rate, time, error counts, think-aloud notes; defects triaged into the audit list.

### Stage F — Documentation and handoff (M8-16)
Design-system documentation (tokens, components, usage rules, do/don't examples), contribution guide for UI changes (including the rule that new displayed values require a catalog entry and provenance), screenshot set regenerated from fixtures, UI changelog, accessibility conformance statement with known gaps.

## 7. Gates
**U1 — Truthfulness + accessibility baseline + core IA (required before G-RELEASE if D-8 is confirmed):** S1–S5 implemented from approved wireframes with full data-state and provenance coverage; contract/truthfulness tests PASS; WCAG 2.2 AA checks PASS for these screens; keyboard-only completion of tasks T-A/T-B; both themes pass contrast; reduced-motion honored; no fabricated value anywhere; defect register has no open Critical/High UI defects.
**U2 — Excellence (after G-MVP; not required for release unless the owner says so):** all screens that have real backends (S6–S11 as available); motion/interaction polish verified against the policy; usability evaluation results with defects closed or accepted; performance budgets met; design documentation complete; visual consistency review (two-reader) PASS.

## 8. Acceptance criteria
| ID | Criterion | Blocking for |
|---|---|---|
| AC8-1 | Entry verified; D-8 recorded; addendum remainder reconciled | all |
| AC8-2 | UI audit with evidence; truthfulness audit shows every displayed value classified | A |
| AC8-3 | Skill decision recorded with verified facts; adoption (if any) follows §3 install method; `SOURCE.md` present; no unreviewed script run | B |
| AC8-4 | Design artifacts delivered and **approved by the owner** before Stage C coding | B→C |
| AC8-5 | Tokens/themes: contrast tables PASS in both themes | C |
| AC8-6 | Components cover all data states × provenance types (gallery) | C |
| AC8-7 | Each screen wired only to existing endpoints; unsupported capabilities shown as such | D |
| AC8-8 | No decorative motion; motion policy tests/review PASS; reduced-motion verified | D |
| AC8-9 | WCAG 2.2 AA checklist PASS (automated where approved + manual) | U1/U2 |
| AC8-10 | Performance/robustness checks PASS | U1/U2 |
| AC8-11 | Truthfulness contract tests PASS; no client-side metric computation | U1 |
| AC8-12 | Usability evaluation delivered; defects triaged | U2 |
| AC8-13 | Documentation complete; ledger clean; no unapproved dependency | U2 |
Results use PASS/FAIL/BLOCKED/NOT RUN (CER-4). A prototype or screenshot is never evidence of a working screen.

## 9. Risks
| Risk | Containment |
|---|---|
| Visual polish masks missing data | provenance system, data-state gallery, truthfulness tests, "requires backend" states |
| Apple look copied literally / IP issues | translation table, no trademarks/SF assets, owner review of brand use (O8-5) |
| Skill content pushes unsuitable patterns (glass, large headings, animation) | filtered adoption, written policy, review checklist |
| Supply-chain risk from skills/installers | no `npx` installs, reviewed copies pinned to SHA, licence check |
| Backend churn breaks screens | screen-contract matrix, schema versioning, fixture contract tests |
| Over-engineering the design system | components added only when two screens need them |
| Accessibility treated as polish | gates U1/U2 tie to WCAG checks, keyboard-first tasks |
| Performance regressions in streaming UI | 10 Hz budget tests, long-session leak test |

## 10. Authorized files / prohibitions
May create: `docs/design/**`, `ui/src/**` (components, styles, screens, tests), `ui/test/**`, `ui/scripts/**`, prototype folders, `.claude/skills/<reviewed-skill>/` or `docs/design/reference/` (only after O8-1), screenshots under `docs/design/screenshots/`. May modify `ui/package.json` only for scripts (dependencies ⇒ O8-3). **Must not:** add backend endpoints or change backend semantics, alter telemetry schema, use simulated data in live views, install tools, fetch and run remote scripts, embed Apple assets/fonts/icons, commit/push. CER-3 applies (including: no `npm ci` in `ui/`, builds to temp directories).

## 11. Owner-reserved decisions
O8-0 reconcile stages with the full addendum · O8-1 skill adoption and install method · O8-2 design direction and artifact approval · O8-3 any dependency or tool (charting, accessibility testing, visual regression, fonts, icon set, prototyping tools) · O8-4 usability participants · O8-5 branding/naming/logo and any Apple-related wording · O8-6 theme scope (dark only vs both) · O8-7 whether U2 is required for release · D-8 whether U1 gates G-RELEASE. **Agents may decide:** component internals, file organization within authorized paths, copy wording that does not make claims, token naming within the approved scale.

## 12. Stop conditions
A screen requires a backend capability that does not exist (mark and route, do not fake); a skill file contains behavior-overriding or command-running content that cannot be removed; any displayed value cannot be classified (show unavailable); contrast or keyboard checks fail on a shipped screen; the owner has not approved the design artifacts before Stage C; unapproved dependency needed.

## 13. Completion report
CER-8 plus: audit and truthfulness tables, skill decision record, approved design artifacts (with approval record), token/contrast tables, component and screen inventory with endpoint mapping, accessibility checklist results, performance/robustness results, usability results, defect register, UI documentation index, gates U1/U2 verdicts, next-phase proposal only.
