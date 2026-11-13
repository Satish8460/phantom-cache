# R3: React engineering, component architecture, accessibility, browser inspection, visual regression, performance validation

**Status:** research, PROPOSAL only. **Date:** 2026-10-30. **Writer:** R3 (single writer of this file and `coord/r3.md`). **Nothing was installed, cloned, executed or changed in the repo.** Downloaded text lives in `D:\Temp\toolkit_research\r3\` and is treated as untrusted data.

Evidence tags: **V** verified by reading a fetched file or repo file (path given), **API** GitHub/npm registry JSON fetched 2026-10-30, **M** from my own knowledge, not re-verified, **H** hypothesis. Rate limiting stopped the GitHub API partway (60 unauthenticated calls), so some repos have stars/licence/commit and some only npm data.

---

## 1. Ground truth: what the project already has (V)

`ui/` is React 18.3 + Vite 5 + plain CSS tokens; runtime deps are exactly `react`, `react-dom`, `lucide-react` (`ui/package.json`); dev deps `vite`, `@vitejs/plugin-react`. `ui/test/forbidden.test.mjs:64-66` asserts `Object.keys(pkg.dependencies)` equals those three, so **any runtime dependency also needs that test edited** (a second reason it needs approval). Dev dependencies are not asserted there.

| Existing asset | What it does | Hard limit (from the files) |
|---|---|---|
| `ui/test/*.mjs` (17 files, `node --test`) | Server-render harness (`scripts/render-harness.mjs`); asserts text, roles, `data-*`, view-models | Cannot run effects, state updates, SSE, focus, CSS or animation. Server-render output only |
| `scripts/a11y-lint.mjs` + `check-a11y.mjs` | Regex HTML parser over SSR output in 8 data states per screen; h1, heading order, names, ids, roles | Own header says it "is NOT a replacement for axe-core"; no computed style, no ARIA-validity tables, no real focus; `simulation` screen excluded in `a11y.test.mjs` |
| `scripts/check-contrast.mjs` | WCAG ratios from `light-dark()` token pairs in `tokens.css`, both themes | Only listed pairs; no text over gradients/glass/images |
| `scripts/forbidden-patterns.mjs` | Regex bans (fabricated numbers, `Math.random`, extra deps, hex outside tokens) | Static text only |
| `scripts/cdp-lib.mjs` (151 lines) | Own headless Chrome (throwaway profile, kills only its process tree), viewport, `Emulation.setEmulatedMedia`, screenshots, key events, console+network logs, waits | No pixel comparison, no a11y engine, no trace/Long Animation Frame capture, Chrome path hard-coded to `C:/Program Files/Google/Chrome` |
| 20+ `browser-*.mjs` rounds; `docs/design/remediation/a11y_evidence/` | CDP `Accessibility` tree snapshots, keyboard traversal, reflow 320..1280, forced-colors, reduced-motion toggles (`browser-a11y-final.mjs:279,377,547`) | No rules engine; findings are what the script author thought to assert |
| `docs/design/prototypes/perf/run_perf.mjs` (DD agent) | CDP frame-time rig, 5 reps, 1280x800 | DD v2 section 12 still contains the placeholder `[[PERF_FULL]]` (checked 2026-10-30); only raw `perf_sw.json` exists. I do not quote any DD number |
| `apple-design` skill (installed) | Feel, springs, gestures, materials, typography | Not engineering, not testing, not accessibility rules; overlap only on reduced-motion and focus principles |

**Consequence for every candidate below:** the project already owns a real-browser driver. A candidate earns a dependency only if it adds something CDP scripts cannot cheaply add: a mature rules engine (axe-core), a headless-primitive widget behaviour (focus trap, roving tabindex), or a pixel comparator. Everything else is "reference" or "already covered".

---

## 2. Candidate evidence table

Pins are repo head SHA at fetch (API) and npm `latest` (registry JSON). Skill files were read from `main` of the repo; the SHA is the head returned by the API at the same time.

### 2.1 Agent skills

| Candidate | Exact source | What it is (V) | Classification |
|---|---|---|---|
| vercel `react-best-practices` | `vercel-labs/agent-skills` @ `1a2c79d4e6`, `skills/react-best-practices/SKILL.md` (7,251 B), frontmatter `name`, `description`, `license: MIT`, `metadata` | 70 rules, 8 categories (async waterfalls, bundle, server, client fetching, re-render, rendering, JS, advanced), each `rules/<prefix>-*.md`. Skewed to Next.js/RSC: the `server-*` (10 rules), `async-api-routes`, `bundle-dynamic-imports` (`next/dynamic`), `rendering-hydration-*` rules do not apply to a Vite SPA. Applicable: `rerender-*` (15), `rendering-content-visibility`, `rendering-hoist-jsx`, `rendering-conditional-render`, `js-*`, `client-event-listeners`, `rerender-use-ref-transient-values`, `rerender-transitions`, `rerender-no-inline-components`. `rendering-activity` and `useEffectEvent` rules need React 19.2+ (M), not React 18 | **USEFUL REFERENCE ONLY** (use a ~20-rule subset as a reviewer checklist, not as an always-on skill) |
| vercel `composition-patterns` | same repo, `skills/composition-patterns/SKILL.md` (2,886 B), `license: MIT` | avoid boolean props, compound components, context interface (state/actions/meta), explicit variants, children over render props. `react19-no-forwardref` is explicitly flagged "React 19+ only. Skip if React 18" | **USEFUL REFERENCE ONLY** (good fit for `Panel`, `Metric`, `ChartFrame` growth; skip section 4) |
| vercel `web-design-guidelines` | same repo, `skills/web-design-guidelines/SKILL.md` (1,231 B) | **Instruction is to WebFetch `https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md` before every review and follow what it returns.** The fetched file (8,055 B, MIT, V) is a static rule list: a11y, focus, forms, animation, typography, images, perf, dark mode, i18n, plus a `file:line` output format | **USEFUL REFERENCE ONLY, never as installed** (see conflict C1) |
| vercel `react-view-transitions`, `react-native-guidelines`, `vercel-optimize`, `deploy-to-vercel`, `writing-guidelines` | README V | `<ViewTransition>` is a React canary/19 API (M); the rest are Vercel/RN/prose | **NOT SUITABLE** |
| anthropics `webapp-testing` | `anthropics/skills` @ `ca45fc496a`, `skills/webapp-testing/` (SKILL.md 3,913 B, `scripts/with_server.py`, examples) | **Python** Playwright; `with_server.py` starts servers with `subprocess.Popen(shell=True)` (V lines 68-73) and polls sockets. Decision tree "recon then action": wait `networkidle`, screenshot, derive selectors. LICENSE.txt is Apache-2.0 (V head of file) | **ALREADY COVERED** by `cdp-lib.mjs` (Node, no Python). Idea worth keeping: reconnaissance before action. Note `networkidle` never settles on an SSE page (H): this skill's central wait would hang on `/api/events` |
| anthropics `frontend-design` | same repo, 9,390 B | Art-direction prompt: "design lead ... distinctive ... take aesthetic risk", brainstorm palette/type, lists AI-design clichés. Quality floor line (responsive, focus, reduced motion) is the only engineering content | **NOT SUITABLE for the application UI** (R1 domain; conflicts with tokens/restraint). Reference for the "avoid generic defaults" list only |
| anthropics `web-artifacts-builder`, `theme-factory`, `canvas-design` | same | Bundle React+Tailwind+shadcn artifacts, themes, canvas art | **NOT SUITABLE** (Tailwind/shadcn) |
| Chrome DevTools MCP skills | `ChromeDevTools/chrome-devtools-mcp` @ `3c609da38a`; `skills/a11y-debugging` (6,042 B), `debug-optimize-lcp` (6,608 B), `chrome-devtools` (4,351 B), also `memory-leak-debugging`, `cookie-debugging`, `troubleshooting`, `chrome-devtools-cli` | a11y-debugging: Lighthouse audit, `take_snapshot` a11y tree, `list_console_messages` issue types, keyboard via `press_key`, `evaluate_script` snippets (tap-target size, contrast, orphan inputs, global checks in `references/a11y-snippets.md`). Skills require the MCP server | **USEFUL REFERENCE ONLY** (snippets are copyable ideas for `cdp-lib` scripts); server itself see 2.2 |
| Playwright CLI + skills | `microsoft/playwright-cli`, npm `@playwright/cli` 0.1.22, `playwright-cli install --skills` | Microsoft's own recommendation: coding agents should prefer CLI+skills over MCP for token cost (README V) | **REQUIRES FURTHER INVESTIGATION** (not needed while CDP scripts suffice; skill contents not read) |

I read all skill text above for directive blocks. **Findings (V):** no hidden or encoded instructions, no hooks, no telemetry in the skill files. Only issues: (a) `web-design-guidelines` pulls live remote instructions from a second repository (mutable instruction channel, a supply-chain and prompt-injection vector, since a later commit could change what the agent is told to do); (b) `webapp-testing` runs arbitrary shell strings via `shell=True` by design; (c) chrome-devtools-mcp skills tell the agent to run `npx chrome-devtools-mcp@latest` (unpinned). I did not read `vercel-optimize`, `deploy-to-vercel` or `playwright-cli` SKILL.md files. `git grep`-style scans of the fetched SKILL.md files found no other URLs besides docs links.

### 2.2 Tools and libraries (npm registry JSON, 2026-10-30; sizes are `unpackedSize`, deps are direct dependency counts)

| Candidate | Pin / date | Licence | Weight and scripts | Notes | Classification |
|---|---|---|---|---|---|
| **axe-core** (`dequelabs/axe-core` @ `6b52b84146`) | 4.14.0, 2026-10-05, integrity `sha512-9WTZxEjs...Pg==` | **MPL-2.0** (file-level copyleft; dev-only, not redistributed in the bundle = low obligation; keep licence if vendored) | 3.35 MB, **0 deps**, no install scripts, engines node>=4 | Deque states it finds "on average 57% of WCAG issues automatically" and reports "incomplete" where unsure (README V). Rule list includes `target-size` (WCAG 2.2 2.5.8) (rule-descriptions V). Runs in any page: can be injected through CDP `Runtime.evaluate` from the existing `cdp-lib.mjs` **without Playwright or Puppeteer** | **RECOMMENDED FOR ADOPTION** (dev-only, approval request A1) |
| @axe-core/playwright | 4.14.0 | MPL-2.0 | 47 KB, 1 dep; `prepare` runs `playwright install` only when building from its own repo, not on registry install (scripts metadata) | Needs Playwright | NOT NEEDED if A1 |
| @axe-core/puppeteer, @axe-core/cli | 4.14.0 | MPL-2.0 | cli: 7 deps | cli drives a WebDriver/Chrome | NOT NEEDED |
| @axe-core/react / react-axe | 4.14.0 / 3.5.4 (react-axe **DEPRECATED**, last 2020) | MPL-2.0 | | Console-only dev overlay | **NOT SUITABLE** |
| jest-axe 11.0.0 / vitest-axe 0.1.0 (2022) | MIT | | Need jest/vitest + jsdom | Project uses neither; jsdom cannot compute layout/contrast | **NOT SUITABLE** |
| pa11y 10.0.0 | **LGPL-3.0-only** | 9 deps, puppeteer-based | Wraps axe/HTML_CodeSniffer | Redundant, licence heavier | **NOT SUITABLE** |
| **Playwright** (`microsoft/playwright` @ `018adb570c`) / `@playwright/test` | 1.64.1, 2026-10-14 | Apache-2.0 | `playwright` 4.9 MB, 1 dep (`playwright-core`); browsers are downloaded by a separate `playwright install` (size not verified; hundreds of MB, M) | Strong: auto-wait, traces, `toHaveScreenshot` (default `threshold` 0.2, `maxDiffPixels` unset, baselines suffixed by browser+platform, e.g. `chromium-darwin`; docs V `test-snapshots-js.md:49,95-100`). Animations default to "disabled" in screenshots and `reducedMotion` is a context option (M, not in the fetched page). Can drive installed Chrome with `channel: 'chrome'` (M) | **REQUIRES FURTHER INVESTIGATION**; ALREADY COVERED for current needs; best candidate if owner later wants a second runner |
| Playwright MCP | `@playwright/mcp` 0.0.83 (pre-1.0), 2026-09-28; repo head `0c4107acbe` | Apache-2.0 | 89 KB, 2 deps | Options include `--isolated`, `--allowed-origins` ("does not serve as a security boundary", README V), `--caps vision,pdf,devtools`, `--secrets`, `--allow-unrestricted-file-access`. README itself steers coding agents toward the CLI+skills | **USEFUL REFERENCE ONLY** |
| **Chrome DevTools MCP** | 1.12.0, 2026-10-22 | Apache-2.0 | 13.9 MB unpacked, 0 deps, `prepare` only for its own repo | **Usage statistics are ON by default** (Google, opt-out `--no-usage-statistics`); performance tools "may send trace URLs to the Google CrUX API" (opt-out `--no-performance-crux`); exposes browser content to the MCP client (README V). Supports only Chrome / Chrome for Testing officially | **REQUIRES APPROVAL if ever used** (egress, MCP server, unpinned `@latest`); interactive inspection value is real but overlaps cdp-lib. USEFUL REFERENCE ONLY today |
| Puppeteer | 25.16.0 | Apache-2.0 | **`postinstall: node install.mjs` downloads a browser**; puppeteer-core 6 MB | Same CDP, bigger API | **NOT SUITABLE** (duplicates cdp-lib; install-time download) |
| **Lighthouse** | 13.6.0, repo `970f00feed` | Apache-2.0 | 18.9 MB, **22 deps** | Includes axe-core internally; lab perf scores on loopback say little about this SSE dashboard (H). Useful one-off for unused JS/bundle and best-practice audits | **USEFUL REFERENCE ONLY**; on-demand `npx` run would itself be an unapproved download |
| Lighthouse CI `@lhci/cli` | 0.15.1, last release 2025-06-25 | Apache-2.0 | 15 deps | CI server/upload targets (temporary public storage is a Google-hosted upload, M) | **NOT SUITABLE** |
| web-vitals | 6.2.3 | Apache-2.0 | 1.2 MB, 0 deps | Field RUM library; would be a runtime dependency and the metrics (LCP/CLS) fit a content site, not a streaming ops console. PerformanceObserver is built in | **NOT SUITABLE** (use built-ins in a CDP script) |
| **Storybook** | 10.6.5, repo `c90522ad91` | MIT | 21.7 MB, 17 deps (+ `@storybook/react-vite` 9 deps, addons) | No stories exist; would mean restructuring components into a story catalogue. `@storybook/test-runner` 0.26.0: **repo archived, "Official support ... has ended"** (README V), migrate to `@storybook/addon-vitest` | **NOT SUITABLE now**; REQUIRES FURTHER INVESTIGATION only if the owner wants a component catalogue |
| Chromatic (`chromatic` 18.13.6, `@chromatic-com/storybook` 5.6.0) | MIT client | | | **SaaS: uploads built Storybook and screenshots to Chromatic's cloud.** Data egress of UI containing lab evidence, plus account/token | **NOT SUITABLE; requires explicit owner approval** (cloud service, data egress, billing) |
| Percy (`@percy/cli` 1.32.12) | MIT client | | | SaaS (BrowserStack); same egress | **NOT SUITABLE; requires approval** |
| reg-suit 0.14.5 | MIT | 6 deps | Publishes reports to S3/GCS via plugins (M) | cloud storage | **NOT SUITABLE** |
| BackstopJS 6.3.25 (2024-09-07) | MIT | 19.6 MB, 18 deps; puppeteer-based | Config-driven scenarios, HTML report | Heavy, slow release cadence | **NOT SUITABLE** |
| Loki 0.35.1 (2024-08-27) | MIT | | Storybook-coupled | | **NOT SUITABLE** |
| **pixelmatch** 8.0.0 (2026-10-06) | ISC | 1 dep (`pngjs ^7`), 36 KB | Tiny anti-aliasing-aware comparator (M for algorithm detail) | The only dependency worth considering for pixel diff, approval A2 (optional) | **REQUIRES FURTHER INVESTIGATION** (second-tier; see 6) |
| odiff-bin 4.5.0 | MIT | 0 deps listed, ships native per-platform binary (binary delivery mechanism not verified) | | | USEFUL REFERENCE ONLY |
| looks-same 10.0.1 | MIT | 5 deps | | | NOT SUITABLE |
| @testing-library/react 16.3.3 (+ `@testing-library/dom` 10.4.2) | MIT | needs jsdom (20 deps, 5.4 MB) or happy-dom (7 deps, 9 MB) | Would let `node:test` run effects/hooks in a fake DOM | jsdom has no layout, no real focus ring, no CSS animation; project already has real Chrome | **REQUIRES FURTHER INVESTIGATION** (only for hook logic that cannot be pure functions) |
| Vitest 5.0.5 / @vitest/browser | MIT | 13 deps | Second test runner | Violates "Node's built-in runner" convention | **NOT SUITABLE** |
| eslint-plugin-react-hooks 7.1.1 | MIT | 4 MB | Rules of hooks (+ compiler-derived rules). **Project has no ESLint**; adding it means ESLint + config | **REQUIRES FURTHER INVESTIGATION** |
| eslint-plugin-jsx-a11y 6.10.2 (2024-10-26) | MIT | 15 deps | Static JSX a11y; the repo's lint is SSR-HTML based | Needs ESLint | NOT SUITABLE now |
| React Compiler (`babel-plugin-react-compiler` 1.0.0, 2025-10-07) | MIT | | react.dev: "designed to work best with React 19, but it also supports React 17 and 18" (V `rc_install.md:19`), 18 needs `react-compiler-runtime` and a target setting (M) | Auto-memoization is an optimisation, not a need: the app re-renders once per second | **NOT SUITABLE now**; revisit only after a profile shows a hotspot |
| react-scan 0.5.7 | MIT | 11 deps | Dev render-highlight overlay | Optional diagnostic, but the React `<Profiler>` API is built in | USEFUL REFERENCE ONLY |
| vite-plugin-checker | MIT | | type/lint overlay | project is JS | NOT SUITABLE |
| Radix: `@radix-ui/react-dialog` 1.3.1 (15 deps) / `radix-ui` 1.8.1 (55 deps); React Aria Components 1.22.1 (Apache-2.0, 6.8 MB, 8 deps); Ariakit `@ariakit/react` 0.4.42 (MIT, 1 dep, 1 MB); shadcn (copy-in on Radix + Tailwind) | | | Headless, unstyled; compatible with plain CSS (M). shadcn assumes Tailwind: **NOT SUITABLE** | see section 4 |

---

## 3. Conflicts between candidates and project rules, and neutralisation

| ID | Conflict | Neutralisation |
|---|---|---|
| C1 | `web-design-guidelines` fetches live remote rules each run (instruction channel not pinned). Also contradictory rule pair: "Interactive elements need keyboard handlers (`onKeyDown`)" vs "`<button>` not `<div onClick>`" (a native button needs no handler) | If used, vendor a dated, hash-recorded local copy of `command.md` (8,055 B, MIT, keep copyright line) in `docs/`, never WebFetch at runtime. R1 reached the same position (coord/r1.md). Port the mechanical rules (`transition: all`, `outline: none` without `:focus-visible`, `user-scalable=no`, `onPaste` preventDefault, `<div onClick>`) to `forbidden-patterns.mjs` as tests; leave judgement rules as a manual checklist |
| C2 | vercel guideline "Title Case for headings/buttons (Chicago)", "`&` over and", "Placeholders end with …" vs project copy conventions (sentence case in the existing UI, text-first labels) | Skip the Content & Copy section entirely; project glossary (`docs/glossary.md`) governs wording |
| C3 | vercel suggests libraries (`virtua`, `nuqs`, `better-all`, SWR) | Project rule: no new runtime dependency. Treat as pattern descriptions; implement natively or not at all |
| C4 | vercel `react-best-practices` targets RSC/Next (`next/dynamic`, `after()`, server actions) and React 19 APIs | Apply only the SPA subset listed in 2.1 |
| C5 | `frontend-design` (anthropics) says take "aesthetic risk", pick distinctive type; project: restrained tokens, `apple-design` + DD v2 (glass only functional) | Do not load for app UI. R1 owns the art-direction decision |
| C6 | `networkidle` waiting (webapp-testing) vs permanent SSE connection | Use explicit readiness markers (a `data-` hook or `waitFor(expr)` as `cdp-lib.waitFor` already does) |
| C7 | chrome-devtools-mcp telemetry/CrUX defaults vs "no telemetry, no data egress without approval" | Only with `--no-usage-statistics --no-performance-crux`, pinned version, `--isolated`, as part of an approved request |
| C8 | Any visual regression SaaS vs UI that renders lab evidence (project evidence rules, CLAUDE.md section 4: label measured/simulated) | Not adopted; local diff only |
| C9 | DD v2 permits functional glass (`backdrop-filter`); axe and the contrast script cannot compute contrast over a blurred backdrop | Already handled by DD's worst-case contrast guarantee (`perf/contrast_worst_case.mjs`); axe marks such text "incomplete", treat incomplete as "needs the worst-case check", not as pass |

---

## 4. Component architecture: headless primitives vs hand-built

**What the DD v2 plan needs (V, `phantomcache_design_direction_v2.md` sections 6, 11, 16):** sticky header and rail, a connection popover, a theme menu, a dialog (cancel/restart confirm), tabs, disclosure ("All qualifications"), a mobile solid sheet menu, a run bar. Charts have a table twin.

Assessment (R3 judgement, H where stated):
1. **Native-first covers most of it with zero dependencies:** tabs/disclosure (`<details>`, buttons + `aria-selected`), nav (`aria-current`), toggles. The repo already implements arrow-key navigation in the rail and has keyboard evidence (`a11y_evidence/kbd_*.json`).
2. **The risky widgets are modal dialog, popover/menu, and the mobile sheet**: focus trap, restore-focus, Esc, inert background, outside-click, roving focus, scroll lock. Native `<dialog>` + `showModal()` provides trap/inert/Esc/top-layer, and the Popover API (`popover` attribute) provides light dismiss in current Chromium (M; browser support not re-verified for this project's targets). Prefer these before any library. Verification is cheap with the existing CDP key driver (Tab, Shift+Tab, Esc, focus-returns-to-trigger) plus axe.
3. **If the owner prefers a primitive library**, ranking by fit with "plain CSS, no component library, minimal footprint": Ariakit (1 dep, MIT) or individual Radix packages (`@radix-ui/react-dialog`, `-popover`, `-dropdown-menu`; each pulls ~15 deps through the shared `@radix-ui/*` set) over `radix-ui` umbrella (55 deps) and React Aria Components (6.8 MB, strongest a11y and i18n, styling by data attributes works with plain CSS). All are runtime dependencies: bundle size, `forbidden.test.mjs` change, `lucide-react` precedent only covers icons. React Aria's strength is mobile/touch/screen-reader behaviour that a hand-built dialog rarely matches, so if more than ~3 complex widgets materialise, a spike (one dialog, bundle delta, axe + keyboard + VoiceOver/NVDA check) is justified. **Decision test:** adopt a primitive only when a hand-built widget fails the keyboard script or axe, or needs >150 lines of focus logic.
4. **Authority for pattern correctness:** W3C WAI-ARIA Authoring Practices Guide (APG) patterns and WCAG 2.2 (I did not re-fetch W3C pages; cited from knowledge, M). Components must follow APG keyboard tables whether hand-built or library-based.

Verdict: **hand-built native-first remains the default; headless primitives are REQUIRES FURTHER INVESTIGATION** and no approval is requested now.

---

## 5. Primary authorities proposed

| Concern | PRIMARY authority | Supporting | Why |
|---|---|---|---|
| React engineering | **react.dev docs (rules of hooks, "You might not need an effect", `<Profiler>`) + existing in-repo conventions and tests** | vercel `react-best-practices` SPA subset as checklist; `composition-patterns` for component growth | Official source, version-matched to React 18; vercel set is Next-biased |
| Component architecture | **WAI-ARIA APG + native HTML elements; `docs/design/phantomcache_design_system_spec.md`** | composition-patterns; headless primitive only per decision test | Behavioural correctness is defined by APG, not by a library |
| Accessibility | **WCAG 2.2 AA (W3C) as the standard; axe-core as the automated engine (after approval A1); existing `check-a11y`/`check-contrast`/CDP AX-tree rounds retained; Track 2 manual protocol** | chrome-devtools a11y-debugging snippets | Automated tools find a fraction of issues (Deque: 57% average), so manual keyboard/screen-reader checks stay mandatory; none of the skills replaces them |
| Browser inspection | **`ui/scripts/cdp-lib.mjs`** (own Chrome, own profile, own process tree) | Chrome DevTools MCP only as an approved optional extra | Already works, no egress, no dependency |
| Visual regression | **In-repo deterministic screenshot + local pixel diff built on `cdp-lib.mjs`** (section 6) | pixelmatch (optional A2); Playwright `toHaveScreenshot` semantics as methodology | Local only; no SaaS |
| Performance validation | **In-repo CDP rig (DD's `run_perf.mjs` approach), PerformanceObserver/Long Animation Frames, `<Profiler>`, bundle-size check script; release-style `vite build` measurements** | Lighthouse on demand | Budgets must come from measured runs per CLAUDE.md section 4 (hardware, repetitions, labelled lab) |

---

## 6. Visual regression and performance: concrete low-dependency design (PROPOSAL, needs code approval)

**Zero-dependency pixel diff:** `cdp-lib.shot()` already saves PNGs. A diff can run inside the same headless Chrome: load two PNGs into `Image`, draw to `OffscreenCanvas`/`canvas`, compare `getImageData`, return mismatch ratio and a diff PNG (`canvas.toDataURL`). Node needs no PNG library. This is a few dozen lines and removes the need for pixelmatch (A2 only buys anti-aliasing-tolerant comparison; the same can be coded as per-channel tolerance plus a small-cluster filter).

**Determinism rules (all testable with existing `cdp-lib` methods):** fixed viewport and `deviceScaleFactor`; `Emulation.setEmulatedMedia` for `prefers-reduced-motion: reduce`, `prefers-color-scheme`, `forced-colors`, `prefers-contrast`; fixed clock (inject a fake `Date.now`/`performance.now` before load, or mock the SSE with a recorded frame); data from recorded fixtures (the UI already has `ui/test/fixtures/lab` real captures and `livePayload()`); disable CSS animations and transitions with an injected stylesheet (`*{animation:none!important;transition:none!important}`) for baseline shots, and separately test the animated path; wait on a readiness `data-` marker not `networkidle`; fonts: wait `document.fonts.ready`. Baselines are tied to OS, GPU/software rasteriser and font rendering (Playwright docs say the same, V `test-snapshots-js.md:20,49`), so baselines must be labelled with the Chrome version and OS from `/json/version` (cdp-lib already records `Browser`) and are valid only on this Windows 11 machine. They are **change detectors, not correctness proof**; the screenshot set from the existing browser rounds can seed them.

**Performance budgets (to be set from measurement, not from this document):** gzip size of `vite build` output (script using `node:zlib`), count of DOM nodes, Long Animation Frames/long tasks during a replayed 60 s SSE stream, JS heap after N minutes, React `<Profiler>` commit durations per screen. DD's own measured glass/3D results are still a placeholder (`[[PERF_FULL]]`), so no glass budget is asserted here.

---

## 7. Implementation and validation implications of the proposed experiences

**Status of inputs:** R4's files (`r4_system_objects_and_motion.md`, `r4_lifecycle_and_event_matrices.md`) did **not exist** when I finalised (checked 2026-10-30; `coord/r4.md` had only a placeholder). This section therefore (a) uses facts I verified in the repo and (b) states, per experience, the event data I **assume** and the request sent to R4. Where R4's inventory later says UNAVAILABLE or PROPOSED, the corresponding experience must be rendered as UNAVAILABLE, never animated as if live (CLAUDE.md section 4, "never present synthetic data as live").

### 7.0 Verified baseline of the live data path (V)
- Gateway rate sampler ticks every **1 s** (`src/api/server.rs:462`); `useTelemetry` runs an `EventSource` on `/api/events?schema=2`, a 1 s wall-clock `setInterval(setNow)` for staleness, and capped history of **120 points** (`ui/src/lib/history.js:5`). So today's cadence is order 1 Hz snapshots, not per-request events. SSE emission cadence per message I did not verify (request R4-1).
- Lab run events come via `GET /api/v1/lab/runs/:id/events` (`lab_api.rs:1031`) with `Last-Event-ID` reconnect (`useRunStream.js`), append-only `events.jsonl` (replayable, so deterministic fixtures exist).
- `useTelemetry` calls `setNow` every second, which re-renders the whole provider subtree each second (H: not profiled). This is the main existing re-render source and the baseline against which new animation must be measured.
- Existing motion: 5 keyframes (`pc-spin/slide/appear/fade`) plus 100-200 ms colour transitions (DD v2 section 2, `app.css:270-273`); reduced-motion handled per CSS file.

### 7.1 Per-experience implications

Update frequency/identity columns say what the implementation needs; "R4 must confirm" marks the dependency.

| Experience | Event availability and rate (assumed until R4 answers) | Object identity across screens | Rendering strategy and cost | Testing | A11y obligations | Budget (proposal, to be validated) |
|---|---|---|---|---|---|---|
| **Request Flow** (a request travelling hit/miss/coalesce/origin) | Per-request events are **not** in the 1 Hz snapshot (V baseline). Likely only aggregate counters/rates (REAL) so per-request motion would be SIMULATED/ILLUSTRATIVE unless R4 finds a sampled trace event. R4 must confirm whether any request-level or sampled-trace event exists | A request has no stable id across screens unless the backend supplies a correlation id (R4) | Do **not** animate each request. Animate aggregate flow density from the 1 Hz counters (stroke-dashoffset or CSS transform on a few `<path>` elements, fixed element count independent of traffic). SVG node count fixed (<~100 nodes, H). CSS animation (compositor) over rAF; rAF only if interpolating values between 1 Hz samples, and then one rAF loop, one state write per frame to refs not React state | Pure view-model tests (existing style) for "counter delta -> density bucket"; CDP check that at `prefers-reduced-motion` no running animations (`document.getAnimations().length === 0`); screenshot with animations disabled | Flow state conveyed as text/table twin ("hit ratio 0.9, REAL"); `role="img"` + label or `aria-hidden` for decoration; no motion-only meaning (WCAG 1.4.1, 2.3.3) | No per-request DOM nodes; <= 2 concurrent animations on a phone (DD v2 6.3); frame time p95 measured by CDP rig |
| **Cache Lifecycle** (entry states: fresh, stale, fetching, invalidated, evicted, fenced) | Depends on whether the gateway exposes per-key state or only counts. CLAUDE.md section 5 invariants exist in the core but R4 must say which are in a payload | Key identity is a string; display must handle arbitrary cardinality and unavailable keys; stable `key` for React reconciliation | Aggregate state-count bars unless per-key sampled list is provided; if a bounded list (e.g. top N keys) exists, N <= ~50 rows rendered as keyed `<tr>`, transitions via CSS class change (colour cross-fade 140 ms) | Deterministic: render each state from a fixture (SSR tests) and assert `data-state` + text; CDP assertion of the class transition end state, not mid-animation frames | State is text + icon shape, not colour only; changes announced through one polite live region, rate-limited (no announcing every transition; batch to at most ~1 per 2-5 s, H) | Fixed rows; update coalescing to the 1 Hz tick |
| **Cache and Cluster Space** (nodes, shards, keys placement, peer invalidation) | Cluster is `experimental`/unavailable in many configurations (the UI already has a "cluster experimental" state in `check-a11y.mjs`); remote peers are address-only descriptors (CLAUDE.md section 6). R4 must confirm which per-node telemetry exists | Node ids stable per cluster config; objects must persist identity between Overview and Cluster pages (stable ids in the keys/`data-node`) | Node count small (3 demo nodes, `main.rs:207-209`): SVG or HTML grid fine. For hundreds of nodes/shards, stay in one `<canvas>` or aggregate; budget SVG nodes (<~500 interactive, H) | Fixtures for 1, 3, N nodes; screenshot matrix; focus order test across nodes; unavailable nodes render UNAVAILABLE | Spatial map needs a table/list twin; each node focusable only if interactive; no hover-only info (WCAG 1.4.13) | Re-render node tiles with `React.memo` + stable props so a 1 Hz update touches only changed nodes (H; profile before memoising) |
| **Workload Intelligence** (skew, hot keys, patterns) | `CountMinSketch` unused; hot-key detection is a known gap (CLAUDE.md section 6). Probably UNAVAILABLE or DERIVED client-side only from aggregates, which the forbidden-pattern rules restrict ("no client metric arithmetic") | Key identity only if the backend ships a top-k | Static charts with twin tables; no animation implying prediction. Existing `Charts.jsx` with 120-point cap is the model | View-model fixtures; forbidden-pattern scan extended to new files | Charts need text summary and table (project pattern `ChartFrame`); provenance label per value | 120 points x series; any new series must keep the cap |
| **Experiment Journey** (Phantom Lab run: stages, gates, evidence) | **Best supported**: run events are an ordered, replayable log (V `events.jsonl`, `Last-Event-ID`), so animation can be driven by real events and replayed deterministically | Run id stable across Lab list/detail/evidence (route params) | Event log appended -> list virtualisation not needed below ~1,000 rows (H; `content-visibility: auto` for long logs); new-row highlight fade (DD: one per 250 ms). Progress as `<progress>`/transform fill | Strongest testability: feed recorded `events.jsonl` to the real reducer (existing `lab.stream.test.mjs` pattern), assert model; CDP test of reconnect and cancel flows already exist (`browser-r3-cancel.mjs`) | Run status announced via existing polite live region (`cancel_run_announcements.json` evidence); long logs need keyboard-reachable scroll region (`tabindex=0` + label) | Log row cap and memory check over a long run; measure heap after replaying a large fixture |
| **Adaptive Control** (PIE decisions, canary, caps) | PIE is a legacy stub with `pie: None`, hard-coded `expected_utility = 85.0` (CLAUDE.md section 6b). Any "decision" animation would be fabricated | n/a | **Do not build live motion**; static UNAVAILABLE/PROPOSED state only | Forbidden-pattern test already bans fabricated PIE text; extend with new strings | Explicit "not available" text with reason | none |

### 7.2 Cross-cutting rules (PROPOSAL)

1. **State vs motion separation.** Every animated object has a `data-state` (and for flows a numeric `data-*` value) set by React from the view-model; CSS only maps state to appearance. Tests assert state/text (stable, SSR-testable) and visual snapshots are taken with animations disabled. Motion itself is tested only for: (a) compliance (`document.getAnimations()` empty under reduced-motion; no `transition: all`; no animated `box-shadow/width/height/backdrop-filter`, matching DD v2 V1 acceptance), (b) existence (an animation starts on a real event), (c) frame cost in the perf rig.
2. **rAF vs CSS.** CSS transitions/animations for state changes and looping indicators (compositor, interruptible, free reduced-motion override via media query). rAF only for value interpolation or physics-like springs (apple-design) and then a single shared loop writing to refs/CSS variables, never `setState` per frame. `requestAnimationFrame` is paused in hidden tabs, so tests must not depend on it elapsing (use fixed clock injection).
3. **React state strategy.** Keep one telemetry context; the 1 s `now` tick currently invalidates every consumer. Proposal: split staleness `now` out of the main value or memoise consumers; measure first with `<Profiler>` (H, not measured by me). Streaming updates arrive at 1 Hz, so batching is already natural; do not add per-event `setState` for run logs beyond appending to the reducer.
4. **Reduced motion.** Static equivalent for every motion; verified by emulating `prefers-reduced-motion` (already in `browser-a11y-final.mjs:547`) and by `getAnimations()`; WCAG 2.3.3 is AAA, but project and DD adopt it.
5. **Live regions.** One polite region per screen, rate-limited and coalesced; assertive only for lost connection/run failure. Tests: record `aria-live` text mutations through CDP (existing `cancel_run_announcements.json` approach) and assert count per minute.
6. **SVG node counts.** Hard cap per visual, asserted by a CDP script counting `svg *` nodes in each state fixture.
7. **Budget derivation.** All budget numbers above are proposals marked H until measured on a release build with hardware/OS/seed/repetitions recorded (CLAUDE.md section 4). DD v2's measured glass results were not yet available.

### 7.3 Track 2 accessibility input
`docs/design/remediation/03_validation_accessibility.md` did not exist when I finalised (checked 2026-10-30). I used the existing evidence folder `a11y_evidence/` (`ax_summary.1280.json`, `kbd_*.json`, `cancel_run_announcements.json`) as ground truth for what has been checked. Gap this implies for axe adoption: the evidence is CDP AX-tree plus keyboard traversal, with no rules-engine pass, so an axe pass would be additive, not duplicative.

---

## 8. Recommendations

**Top 5 (ordered):**
1. **Adopt axe-core as a dev-only dependency (A1), injected through `cdp-lib.mjs`**; no Playwright/Puppeteer needed. Unique contribution: rendered-DOM rules engine (ARIA validity, names, `target-size` 2.5.8, contrast where computable, landmarks), covering states the SSR regex lint cannot see.
2. **Build an in-repo deterministic visual-regression and perf-budget layer on `cdp-lib`** (canvas-based diff, media emulation, injected animation-off stylesheet, bundle-size and long-frame checks). No dependency. Needs code approval.
3. **Keep hand-built native-first components (`<dialog>`, Popover API, `<details>`), with a decision test for adopting a headless primitive** (Ariakit or React Aria) only when a widget fails keyboard/axe checks.
4. **Use vercel `web-design-guidelines` and `react-best-practices` only as vendored, version-pinned checklists**, port the mechanical rules into `forbidden-patterns.mjs`; never the runtime WebFetch variant.
5. **Do not adopt Storybook, Chromatic, Percy, BackstopJS, Loki, reg-suit, pa11y, Puppeteer, Lighthouse CI**: heavy, archived, copyleft or cloud egress.

### 8.1 Approval requests (exact format, CLAUDE.md section 2). Nothing below has been run.

**A1. Dev-dependency: `axe-core@4.14.0`** (in `ui/`)
- Purpose: automated WCAG rules pass on the rendered DOM in real headless Chrome, driven by existing CDP scripts. Adds `ui/scripts/check-axe.mjs` (code change, also needs approval).
- Alternatives: keep only in-house lint (cannot see computed/ARIA semantics); `@axe-core/playwright`/`puppeteer` (extra browser driver); Lighthouse (22 deps, axe inside); vendoring `axe.min.js` (no update path, same MPL obligation); manual only.
- Maintenance: Deque, last release 4.14.0 on 2026-10-05, repo pushed 2026-10-29, 458 open issues (API); engines node>=4.
- Security: 0 dependencies, no install scripts (registry JSON), integrity `sha512-9WTZxEjsZ7b13TH8JPmbV2z8CHbl80/2hm3XPEG4JgNdQLK81IBRXmSxHfMAOkSqQeRxT/0dwNDz2GOm3zzpcQ==`; runs only inside our own headless Chrome against localhost; no network, no telemetry known (M).
- Dev-only: yes (`devDependencies`, not bundled; `forbidden.test.mjs` asserts only `dependencies`). Licence MPL-2.0: unmodified use as a tool imposes no obligation on PhantomCache source; if vendored, keep the licence.
- Proposed command (NOT executed): `cd ui && npm install --save-dev --save-exact --ignore-scripts axe-core@4.14.0` (updates `ui/package.json` and `package-lock.json`). Expected location: `ui/node_modules/axe-core/axe.min.js`.

**A2 (optional, lower priority). Dev-dependency: `pixelmatch@8.0.0`** (+ `pngjs@7`)
- Purpose: anti-aliasing-aware PNG diff in Node. Alternative: zero-dependency canvas diff in Chrome (preferred, section 6); `odiff-bin` (native binary); `looks-same` (5 deps). Maintenance: ISC, release 2026-10-06. Security: 1 dependency (`pngjs`), no install scripts. Dev-only: yes. Recommendation: **do not request** unless the canvas diff proves too noisy.

**A3 (conditional, only if a spike shows hand-built widgets fail). Runtime dependency decision for a headless primitive** (Ariakit `@ariakit/react@0.4.42` or specific `@radix-ui/react-*`/React Aria packages): not requested now; would require editing `forbidden.test.mjs`, a bundle-size measurement and an a11y spike report first.

**Not requested but flagged as needing approval if anyone proposes them:** any SaaS (Chromatic, Percy), Chrome DevTools MCP (telemetry on by default, CrUX egress), Playwright MCP/browser downloads, Puppeteer (postinstall browser download), Storybook.

### 8.2 Proposed installation of recommended items (PROPOSAL)
- axe-core: command above; pin `4.14.0`; licence file `ui/node_modules/axe-core/LICENSE` (MPL-2.0) stays in place; no files copied into `.claude/skills`.
- No agent skill is recommended for installation. If the owner wants the vercel checklist: save `command.md` (source `vercel-labs/web-interface-guidelines`, fetched 2026-10-30, 8,055 B, MIT, "Copyright (c) 2025 Vercel Labs") as `docs/design/reference/web_interface_guidelines_2026-10-30.md` with its licence line and a SHA-256 recorded; do not copy its `$ARGUMENTS` frontmatter as a command. (Licence note: `vercel-labs/agent-skills` has no LICENSE file at root `main` (404); README says "MIT" and skills' frontmatter says `license: MIT`; the `web-interface-guidelines` repo has an MIT LICENSE (V).)

---

## 9. Requests and replies

### Requests to R4
1. **R4-1 (rate and shape):** for each of the six experiences, list the events/fields that exist, their source (SSE `/api/events` payload, lab run events, REST), emission cadence (the metrics sampler is 1 s, `server.rs:462`; I did not verify SSE message cadence), cardinality bounds and timestamps. *Why:* sets re-render cost, test fixtures and budgets. *Assumed if unanswered:* ~1 Hz aggregate snapshots, no per-request or per-key events.
2. **R4-2 (identity):** which objects have stable ids across screens (key, node, run, request) and which have none. *Assumed:* run and node ids stable; key and request ids not.
3. **R4-3 (replayability):** which event streams are recorded and replayable (lab `events.jsonl`: yes per my reading) and which are live-only, so I can specify deterministic fixtures. *Assumed:* only the lab stream is replayable.
4. **R4-4 (UNAVAILABLE/PROPOSED list):** anything in the six experiences that is UNAVAILABLE or PROPOSED, so that motion is not specified for it. *Assumed:* Request Flow per-request, Workload Intelligence hot-keys, Adaptive Control decisions are UNAVAILABLE.

### Replies to other agents
- **To R1 (coord/r1.md, C1 `web-design-guidelines` remote-fetch):** agree; position recorded as C1 (vendor a pinned local copy, port mechanical rules to the scanner). R1's proposal that `apple-design` stay the sole art-direction authority is compatible with my section 5.
- **To R2:** (a) motion tests: I need R2's chosen mechanism (CSS/WAAPI/rAF/library); section 7.2 assumes CSS-first. (b) Emil Kowalski `pick-ui-library`-type advice to install recharts/motion conflicts with the no-new-runtime-dependency rule; neutralisation is the same approval gate. (c) Any animation library is a runtime dependency that would also trip `forbidden.test.mjs:66`.
- **To R4:** none requested of R3 yet at time of finalising.

---

## 10. Methodology, sources, what could not be verified

- **Method:** read repo (`ui/package.json`, `scripts/cdp-lib.mjs`, `a11y-lint.mjs`, `check-a11y.mjs`, `check-contrast.mjs`, `forbidden-patterns.mjs`, `test/a11y.test.mjs`, `test/forbidden.test.mjs`, hooks, `server.rs` sampler, DD v2, remediation `02_ui_audit_and_plan.md`, coord files); GitHub REST API (repo and last-commit JSON for ~35 repos, 2026-10-30; throttled after 60 calls) and raw.githubusercontent.com files; npm registry JSON for ~50 packages; react.dev and axe and Playwright docs from their GitHub sources. All saved under `D:\Temp\toolkit_research\r3\` (`va/`, `as/`, `cdm/`, `pwmcp/`, `pwcli/`, `wig/`, `npm/`, `docs/`). No installs, no script execution, no `git` mutation.
- **Date of "now" for staleness:** 2026-10-30; releases within 2 weeks of that date are recent, not proven stable.
- **Not verified:** (1) the GitHub API rate limit blocked stars/licence/commit for react.dev, React, Testing Library, Radix, Ariakit, shadcn, odiff, pixelmatch, Loki, web-vitals, jest-axe, Percy repos; their data are npm-only. (2) I did not run a web search for additional accessibility/responsive/performance-budget Agent Skills, so "find additional candidates" is covered only by the candidates named in the brief plus the discoveries made while reading (Playwright CLI skills, chrome-devtools-mcp skills, `web-interface-guidelines`). A future pass should search skills.sh or equivalent. (3) W3C WAI, APG, Deque University pages were not fetched; WCAG/APG statements are from knowledge (M). (4) Playwright browser download size, Playwright `animations: "disabled"` default and `reducedMotion` option, Popover API/`<dialog>` browser support for this project's browser targets, Radix/React Aria/Ariakit plain-CSS compatibility details, pixelmatch algorithm details, Lighthouse CI upload target: all M. (5) Event cadence of `/api/events` and whether any per-request/per-key events exist: unverified (R4 requests). (6) Re-render cost of the 1 s `now` tick: hypothesis, not profiled. (7) I did not read `playwright-cli`'s skill file, `vercel-optimize`, or the other chrome-devtools-mcp skills beyond the three listed. (8) DD v2 measured performance and Track 2 accessibility documents were placeholders or absent at the time of reading. (9) Maintenance "last commit" for BackstopJS and `storybookjs/test-runner` come from API `pushed_at`; for others, from npm release dates.
