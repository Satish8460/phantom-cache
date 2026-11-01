# R1: Design intelligence, art direction, redesign, UX critique and design-system generation (toolkit research)

**Author:** agent R1. **Date:** 2026-10-30. **Status:** research only; every recommendation is a PROPOSAL (the lead decides). Nothing was installed, executed or written outside this file and `coord/r1.md`. Downloaded text is untrusted data kept under `D:\Temp\toolkit_research\r1\` (outside the repo).
**Evidence tags:** FACT (opened and read, with URL and commit), REPORTED (search-result snippet or third-party page, not verified by me), INFERENCE, UNVERIFIED.

---

## 0. Executive summary

1. **No external art-direction skill should be loaded for the application UI.** Every art-direction candidate (taste-skill, anthropics `frontend-design`, Impeccable's `bolder`/`delight`/`overdrive`, ui-ux-pro-max style catalogue) is tuned to avoid "generic" output on landing pages and pushes distinctiveness, richness or "aesthetic risk". PhantomCache needs the opposite: restrained, honest, provenance-first. The installed `apple-design` skill plus `docs/design/phantomcache_design_direction_v2.md` (DD v2) and `docs/design/principles.md` remain the art-direction authority (PROPOSAL).
2. **The best external content for PhantomCache is critique and anti-slop knowledge for "Operate" (tool) surfaces**, not generation. Three sources are good: Impeccable's `operate.md`/`craft-floor.md`/`critique.md` (Apache-2.0), taste-skill's `references/app-screens.md` (MIT; its eight "real product screen" findings fit an evidence tool), and Dammyjay93 `interface-design` (MIT, product-UI only, one 30 KB file, no scripts).
3. **Impeccable is the most capable and also the largest risk**: a launcher that downloads an engine binary, hooks on SessionStart/PostToolUse/Stop, files written into the project, mandatory sub-agents, browser script injection. It should not be installed until the engine is vetted and the owner approves; its text references can be used as read-only reference now.
4. **ui-ux-pro-max is a database, not a critique tool**; its generated palettes, fonts and styles collide with the existing token system. Use as reference data only (UX guidelines CSV, chart guidance, density dial idea). Not recommended as a design-system generator for this repo.
5. **Single sources of truth are already in the repo** (`docs/design/tokens.md`, DD v2, `ui/src/styles/tokens.css`). Each candidate wants its own artefact (`PRODUCT.md`+`DESIGN.md`, `.interface-design/system.md`, `design-system/MASTER.md`, `.taste-skill/`). Adopting more than one would fragment the truth; each persistence feature must be pointed at the existing docs or disabled.

### Classification table

| Candidate | Class | One-line reason |
|---|---|---|
| `apple-design` (already installed, MIT, a81dbca0) | KEEP as primary art-direction/feel authority | Principles and motion, no decorative mandate; project rules exist |
| Impeccable (pbakaus, Apache-2.0, skill-v4.8.0) | REQUIRES FURTHER INVESTIGATION (engine binary, hooks); text references are USEFUL REFERENCE ONLY now | Best Operate-mode and Nielsen-scored critique; heavy install surface |
| `interface-design` (Dammyjay93, MIT) | RECOMMENDED FOR ADOPTION (optional, low risk, with project overrides) | Product-UI-only craft skill, static text, no scripts |
| `nielsen-heuristics-audit` (mastepanoski, MIT) | RECOMMENDED FOR ADOPTION (as vendored static reference or interim critique skill) | 9 KB, no scripts, injection-aware, covers the ten heuristics |
| taste-skill (Leonxlnx, MIT) | NOT SUITABLE as a skill; `references/app-screens.md` and `critique.md` are USEFUL REFERENCE ONLY | Landing-page art direction, network image fetch, headless Chrome, "do not trade richness for restraint" |
| anthropics `frontend-design` (Apache-2.0) | USEFUL REFERENCE ONLY (do not load for app UI) | "Aesthetic risk" brief; its anti-cliche list and writing section are good |
| ui-ux-pro-max (nextlevelbuilder, MIT) | USEFUL REFERENCE ONLY | Searchable database; style catalogue is decorative-heavy; Python and file-writing paths |
| Vercel `web-interface-guidelines` / `web-design-guidelines` | USEFUL REFERENCE ONLY (vendor pinned `command.md`) | Skill fetches mutable remote rules at run time (R3 agrees) |
| Carbon status-indicator and Primer data-visualization guidance | USEFUL REFERENCE ONLY | Official, align with provenance "never colour alone" and chart contrast |
| Style Dictionary 5.7.1 (Apache-2.0) + sd-transforms | NOT SUITABLE now | Tokens are hand-written CSS custom properties; a build pipeline plus 13 dependencies for no gain |
| glebis `design-tokens`, plugin87 `design-tokens`, StyleSeed, sickn33 `design-ux` | REQUIRES FURTHER INVESTIGATION (not opened) | Seen only in search snippets |

---

## 1. Methodology (queries, sources, dates, limits)

- **Date of research:** 2026-10-30. Environment: Windows, Git Bash, no Python 3 on the bash PATH (`py` launcher exists), `jq` present. Nothing was run from downloaded content; downloaded files were only read with `cat`, `head`, `grep`, `jq`.
- **Metadata** (stars, licence field, `pushed_at`, open issue+PR counts, default branch, tags) came from the unauthenticated GitHub REST API; the 60-requests-per-hour limit was exhausted after the first pass, so later metadata uses `git ls-remote` (HEAD SHA) and `raw.githubusercontent.com` at pinned SHAs. Stars are popularity only and are not used for ranking.
- **Searches** (WebSearch, standard mode): "Claude Code skill design critique UX audit Nielsen heuristics SKILL.md github"; "Claude skill design tokens Style Dictionary DTCG agent skill github SKILL.md"; "Dammyjay93 interface-design skill Claude Code dashboards tools github". Search-result summaries are REPORTED only; I verified `interface-design`, `nielsen-heuristics-audit` and `web-interface-guidelines` from raw files. Pages fetched: Carbon status indicators (last updated 2026-08-12 per page), Primer data visualization.
- **Read in full:** ui-ux-pro-max `SKILL.md` (16 KB), `plugin.json`, `marketplace.json`, `LICENSE`, `search.py` imports (grep), `design-system/SKILL.md` head; taste-skill `SKILL.md` first 16 KB of 39.5 KB, `install.sh`, `app-screens.md` (first 5.5 KB plus targeted greps), `critique.md` (head), script grep for network/process/file APIs; Impeccable `SKILL.md` (12 KB), `.claude/settings.json`, `.claude-plugin/plugin.json`, `reference/operate.md`, `reference/craft-floor.md`, `reference/critique.md` (head and greps of 45 KB), `reference/hooks.md` (head), launcher `scripts/impeccable` (grep of download logic), `engine-checksums.txt`, `LICENSE`, `NOTICE.md`; anthropics `frontend-design/SKILL.md` (9.4 KB, full) and `marketplace.json`; `interface-design/SKILL.md` (29.6 KB; first 6.5 KB and lines 176-192 plus keyword greps), `plugin.json`, `LICENSE`; mastepanoski `nielsen-heuristics-audit/SKILL.md` (first 3 KB plus greps), `LICENSE`, README head; Vercel `web-design-guidelines/SKILL.md` and `command.md` (head 40 lines).
- **Could not verify:** release dates (only tag names), contributor counts, issue/PR responsiveness (API exhausted; open counts only), the Impeccable engine binary (not downloaded or inspected), `live-browser.js` (563 KB, not read), the rest of taste-skill `SKILL.md`/`motion.md`/`imagery.md`, ui-ux-pro-max `design_system.py` beyond import and write greps, `quick-reference.md`, `cli/` and `stack/scripts/setup.sh`, mastepanoski `references/heuristics.md`, the other 12 mastepanoski skills, `interface-design` remaining files (only SKILL.md was fetched), and whether `docs/design/remediation/03_validation_accessibility.md` exists (it did not, 2026-10-30). Earlier in this run I recorded that DD v2 was absent; it exists (453 lines) and was read afterwards (thanks to R5's notice).
- Coverage of the category is not exhaustive: Refactoring UI (a commercial book, no official skill found), Material/Polaris/Radix/shadcn guidance and WCAG design linters were not surveyed beyond noting that axe/pa11y/Lighthouse are R3's scope.

---

## 2. Candidate evidence

### 2.1 Impeccable (pbakaus/impeccable)

- **Source (FACT):** https://github.com/pbakaus/impeccable, commit `02523aec3e4c4580102d996d1b6a71fbe062e03f` (2026-10-29T18:41:44Z), tags `skill-v4.8.0`, `engine-v0.1.18`, `cli-v4.8.0`. API 2026-10-30: Apache-2.0, ~96.2k stars, created 2025-11-16, 48 open issues+PRs, default branch `main`. `LICENSE` is the full Apache 2.0 text (10.7 KB); `NOTICE.md` credits ehmo/platform-design-skills (MIT) for `ios.md`/`android.md`.
- **Path and name:** `.claude/skills/impeccable/SKILL.md` (12 KB) plus `reference/` (about 50 files: critique 45 KB, new-work 63 KB, live 37 KB, document 27 KB, ...), `scripts/impeccable` (launcher, 10 KB), `scripts/impeccable.cmd`, `scripts/live-browser.js` (563 KB), `scripts/data/font-index.json` (1.1 MB), `.claude/agents/*` (4 sub-agent definitions), `.claude/settings.json` (hooks). Skill name `impeccable`, `user-invocable: true`, 24 sub-commands.
- **What it instructs (FACT, quotes):** "gives you the tools and permission to make design that earns the name out-of-distribution craft. Work as an award-winning design director would"; four modes: Persuade, Operate, Read, Experience, with "On Operate and Read, ambition shows as precision and restraint". `operate.md`: "The tool should disappear into the task"; "One family is often right"; "Product defaults to Restrained"; "Motion conveys state, not decoration"; "No orchestrated page-load sequences"; skeleton states; "Modal as first thought" is laziness. `craft-floor.md` Refuse list: eyebrow above a heading is "a ban"; no gradient text; no glass "as decoration"; no colored border-left above 1px; no sparklines/progress rings standing in for content; no monospace "as a costume"; no emoji for icons; verify contrast 4.5:1/3:1. Same file also says "Reach past transform and opacity: blur, backdrop-filter, clip-path, mask, and shadow belong to the palette". `critique.md`: two isolated sub-agent assessments (design review + detector/browser evidence), Nielsen's ten heuristics scored 0 to 4 ("Most real interfaces score 20-32 out of 40"), cognitive load, persona red flags, snapshot persisted, report must end with questions.
- **Claude Code compatibility:** valid SKILL.md frontmatter; ships `.claude-plugin/plugin.json` and `marketplace.json` (`/plugin marketplace add pbakaus/impeccable`, or `npx impeccable install`). Windows launcher `.cmd` exists. No MCP needed. Needs a sub-agent tool for `critique`.
- **React + Vite fit:** framework-agnostic; detector scans `.jsx/.tsx/.css`; the project has plain CSS tokens, which suits. Some detector rules (overused fonts such as Inter, gradient text, glow shadows) are design opinions and need ignores.
- **Maintenance:** very active (commit the day before research), versioned skill/engine/CLI releases, 48 open items. Contributors not counted (UNVERIFIED).
- **Security and installation (FACT):**
  - Launcher `scripts/impeccable` runs a "self-contained binary that ships next to it or is downloaded once on first run" from `https://github.com/pbakaus/impeccable/releases/download/...` (override `IMPECCABLE_DOWNLOAD_BASE`), verified against `engine-checksums.txt` that ships in the same repository (integrity against corruption, not against a compromised repository). Engine v0.1.18 windows-x64 digest recorded: `8f61e96b0c97001546a824d79b44f46b423b560f3eb1a0cc44a8695a1b395d9f`. Binary NOT downloaded or inspected by me.
  - `SKILL.md` Setup step 1 requires running the launcher at the start of each session (`impeccable context`), which also reads PRODUCT.md/DESIGN.md.
  - `.claude/settings.json` registers command hooks on `SessionStart`, `PostToolUse` (Edit|Write) and `Stop` (30 s) that run the binary and push findings back into the agent as system reminders. `/impeccable hooks on` edits provider manifests and `.impeccable/config.json`; the installer asks consent. Note the project rule that hooks and settings changes require approval.
  - Writes: PRODUCT.md, DESIGN.md, `.impeccable/config.json`, critique snapshots, optional `pin` shortcuts; `live` mode injects a 563 KB script into pages and runs a local server.
  - Prompt-injection surface: very large instruction volume (about 460 KB of reference text, summed from the file listing), the hook injects tool output into context, `critique` forces sub-agents. I found no hidden or hostile directive in what I read; the directives are overt workflow mandates (mandatory banner text, "The question is the LAST thing in the response").
- **Performance implications of what it recommends:** Operate guidance is lean (150-250 ms transitions, no load choreography). `craft-floor` encourages blur/backdrop-filter/clip-path which has measured cost (DD v2 section 3 measured materials); that line conflicts with DD v2's "glass only on functional layers".
- **Overlap with apple-design:** partial. Both cover typography, motion restraint and states; Impeccable adds the critique and audit method, ban list and detector; apple-design adds springs, gestures, materials.
- **Value for PhantomCache:** highest of the candidates for Operate-surface critique: states checklist (default, hover, focus, active, disabled, loading, error, empty), "colour on data/state not chrome", Nielsen scoring. Weak for generation (default posture "bolder", "delight", "overdrive").
- **Licence:** Apache-2.0; to vendor any file keep `LICENSE`, retain notices, mark modifications. SPDX `Apache-2.0`.
- **Class:** REQUIRES FURTHER INVESTIGATION for installation; text-only reference use is allowed now.

### 2.2 taste-skill (Leonxlnx/taste-skill)

- **Source (FACT):** https://github.com/Leonxlnx/taste-skill, commit `12af471f627f866411b4bd785b19646d3672784c` (2026-10-24T23:42:19Z), tags v2.1.5/v2.1.4/v2.1.3; MIT (`LICENSE`, 1 KB, copyright 2026 Leonxlnx); ~105.9k stars; created 2026-02-19; 7 open items.
- **Path/name:** `skills/taste-skill/SKILL.md` (39.5 KB), frontmatter name `design-taste-frontend`; `references/` (app-screens 85 KB, motion 55 KB, imagery 51 KB, critique, colour, copy, layout, type, scenes, tells, research), `library/*.json` (about 2 MB incl. `sites.json` 869 KB), `scripts/{draw,inspect,assets,lint}.mjs` + `lib/{cdp,probe,rules}.mjs`, `assets/*`. Marketplace: `/plugin marketplace add Leonxlnx/taste-skill`, `/plugin install taste-skill@taste-skill`, or `npx skills add ... --skill design-taste-frontend`, or `install.sh` which symlinks into `~/.claude/skills` (global).
- **What it instructs (FACT):** description: "anti-slop frontend skill. Art direction for websites ... landing page, marketing site, portfolio ... Also for app screens". A script (`draw.mjs`) deals three of twenty "directions" and "you choose"; "Adopt it. Do not blend it"; "Find the idea" from the subject; "One live thing" per page; write copy deck; "Taking the ornaments away is only half the job ... do not trade it [richness] for restraint"; sample details "are expected where the brief leaves gaps (dish names, room names and rates ...)... Proof is never invented". App screens follow `references/app-screens.md`: eight findings (colour on data not chrome, ~3% coloured pixels; KPIs are type not tiles, no sparklines on KPIs; flat surfaces; no hero header; charts with almost no chrome; navigation recedes; tables are rules and alignment; "Data states are honest. Zeros print as zeros, missing values as a grey dash, and an empty section keeps its frame"). It also says "Give the product a face": a product mark, "one element only this product has ... otherwise invent one", and "Believable sample data computed from rows, listed in the report".
- **Scripts (FACT by grep):** `assets.mjs` uses `fetch()` against Openverse, Wikimedia Commons, Art Institute of Chicago and the Met APIs and writes image files; `inspect.mjs` starts a local static server and drives Chrome/Edge via `lib/cdp.mjs` (`--headless=new --remote-debugging-port=0 --user-data-dir=<temp>`, `spawn`, `taskkill`); writes `report.json` and screenshots to `.taste-skill/`. Needs Node 22+ and a browser. No install hooks. No hidden directives found in the files read.
- **React+Vite fit:** generic; page run assumes you generate whole sites. The app-screen run suits a dashboard but assumes designing with its dealt "styles".
- **Maintenance:** active (6 days). **Dependencies:** none (Node built-ins). 
- **Performance implications:** recommends scroll scenes, WebGL-like "scenes", photographs, webfonts: not applicable to a dense tool.
- **Overlap with apple-design:** low (different goal). Overlaps with Impeccable on ban lists.
- **Value for PhantomCache:** only `app-screens.md` findings 1-8 and `critique.md` method. The skill itself would push richer, picture-led, "idea-led" design.
- **Class:** NOT SUITABLE as installed skill; USEFUL REFERENCE ONLY for `app-screens.md` and `critique.md` (MIT: keep the notice if quoted).

### 2.3 anthropics/skills `frontend-design`

- **Source (FACT):** https://github.com/anthropics/skills, commit `ca45fc496a3436251f191354091d557da31c15b5` (2026-10-28T19:20:38Z); repo-level licence field is null; each skill carries its own `LICENSE.txt`; `skills/frontend-design/LICENSE.txt` is Apache License 2.0 (10.2 KB). ~184.9k stars, 1,449 open items (issues+PRs).
- **Path:** `skills/frontend-design/SKILL.md` (9.4 KB, frontmatter name `frontend-design`; distributed in the `example-skills` plugin of `.claude-plugin/marketplace.json`, which also bundles eleven other skills; install only the one skill by copying).
- **What it instructs (FACT, quotes):** "Approach this as the design lead at a design studio ... take aesthetic risk if justified"; hero should be "the most characteristic thing in the subject's world"; avoid default typographic tells (single-word accent, ALL-CAPS labels, "unnecessary typographic labels above content"); "Spend your boldness in one place"; "Visual structure is information"; lists five clusters of generated-design defaults (cream+serif+terracotta, near-black+acid accent, broadsheet, SaaS-card kit, template chrome); two-pass plan then review; quality floor "responsive ... visible keyboard focus, reduced motion respected". The long "writing in design" section (sentence case, verbs for CTAs, errors that explain recovery, "An action keeps the same name through the whole flow") is directly useful and non-decorative. It also says "come up with copy and placeholder content", which conflicts with no-fabricated-data unless labelled.
- **Compatibility:** no scripts, no MCP, no dependencies; React+Vite agnostic. Maintenance: active org repo. Security: static text only.
- **Overlap with apple-design:** low. Overlap with Impeccable/taste on anti-slop.
- **Class:** USEFUL REFERENCE ONLY. Do not load for the app UI (R3 reached the same view). Reuse: the "writing in design" section and the cliche list as review prompts.

### 2.4 ui-ux-pro-max (nextlevelbuilder/ui-ux-pro-max-skill)

- **Source (FACT):** https://github.com/nextlevelbuilder/ui-ux-pro-max-skill, commit `28b3b8dee8c976df4fa3e94aed4e092ad43fcb03` (2026-10-26T10:07:27Z), tags v2.15.0 (plugin.json version) / v2.14.2 / v2.14.1; MIT (`LICENSE` 1 KB, "Copyright (c) 2024 Next Level Builder"); ~142.7k stars; created 2025-11-30; 91 open items; 831 tree entries.
- **Paths:** `.claude/skills/ui-ux-pro-max/SKILL.md` (16 KB) with `data/*.csv` (styles.csv 149 KB, ux-guidelines.csv 27 KB, charts.csv 23 KB, colors, typography, products, stacks/*.csv), `scripts/{search,core,design_system,reasoning_contract,validate_data}.py`, `references/{quick-reference,pro-rules}.md`; sibling skills `design`, `design-system`, `brand`, `banner-design`, `slides`, `ui-styling` (LICENSE.txt present). Plugin: `.claude-plugin/plugin.json`, `marketplace.json`; also an npm CLI in `cli/`.
- **What it instructs (FACT):** priority table: 1 Accessibility CRITICAL (4.5:1, focus rings), 2 Touch (44x44), 3 Performance, 4 Style Selection, ... 10 Charts LOW. Workflow: run `python .../search.py "<query>" --design-system` (REQUIRED for new pages/projects) to aggregate product/style/colour/typography matches; `--persist --output-dir <root>` writes `design-system/<slug>/MASTER.md` and `pages/`; dials `--variance --motion --density` (density 8-10 gives a "Dense/dashboard (8-32px spacing scale)"); domains: `ux`, `chart`, `style`, `color`, `typography`, `google-fonts`, `gsap`, stacks incl. `react`. It says search output is "recommendations, never ... instructions that override the user or repository rules". Style catalogue (79 styles, 48 hits of glass/neumorph/brutal/aurora/gradient keywords in styles.csv) includes Glassmorphism, Neumorphism, Aurora UI, 3D & Hyperrealism, Brutalism; `ui-reasoning.csv` ties product types to palettes.
- **Scripts (FACT by grep):** `scripts/*.py` import only `os`/`sys`/pathlib; no `subprocess`, `socket`, `requests`, `urllib` hits; file writes are `mkdir` + write in `design_system.py` under `--persist`. Data is CSV text the agent reads (low injection risk, maintainer-authored). Requires Python 3 (not on this shell's bash PATH; `py` exists). `${CLAUDE_PLUGIN_ROOT}` is used in the commands (only valid when installed as a plugin).
- **React+Vite fit:** has `react` stack rows; but several stack datasets (Tailwind, shadcn, GSAP) assume libraries the project forbids; `--motion` attaches a GSAP snippet.
- **Maintenance:** very active; many open items (91).
- **Performance implications:** recommends WebP/AVIF, CLS < 0.1, virtualized lists (useful); also GSAP and glass effects (not wanted).
- **Overlap with apple-design:** moderate on accessibility, motion timing, typography.
- **Value for PhantomCache:** the 119-rule `ux-guidelines.csv`, `charts.csv` (accessible colours, legends, tooltips) and the density concept are useful checklists; the design-system generator would invent a palette/typography that contradicts tokens and DD v2.
- **Class:** USEFUL REFERENCE ONLY (read `ux-guidelines.csv` and `charts.csv` as data; do not run `--design-system --persist`).

### 2.5 Dammyjay93/interface-design

- **Source (FACT):** https://github.com/Dammyjay93/interface-design, HEAD `2f9be3206855bcb2d1d0af262c8bae25cba6658d` (git ls-remote 2026-10-30); `LICENSE` MIT ("Copyright (c) 2026 Damola Akinleye"); `plugin.json` version 2026.6.20.1332; REPORTED: latest release tag v2026.6.12, ~5.3k stars, updated 2026-06-20 (aggregator; not verified).
- **Path/name:** `.claude/skills/interface-design/SKILL.md` (29.6 KB), name `interface-design`; commands in `.claude/commands` (init, status, audit, extract; REPORTED, directory not read). Install per README: `npx skills add https://github.com/dammyjay93/interface-design --skill interface-design` or `/plugin marketplace add Dammyjay93/interface-design`.
- **What it instructs (FACT):** scope: "Dashboards, admin panels, SaaS apps, tools, settings pages, data interfaces. Not for: Landing pages, marketing sites". Intent first (who, what verb, what feel); Product Domain Exploration (domain concepts min 5, colour world, one signature element, three defaults); "Intent must be systemic"; token names that evoke the product's world; "Native HTML first", then "battle-tested headless primitive (Radix UI, React Aria, ...)", hand-roll only "as a genuine last resort ... no dependency budget"; bind to semantic tokens; save decisions to `.interface-design/system.md` (offers to save; reads it next session). It has "Render It When You Can" (use an inline render tool if present).
- **Compat/security:** single SKILL.md, no scripts found; keyword search found no curl/npx/download directive in the file. A third-party audit page (REPORTED, skills.sh for a mirror `podo/design-agent-skills`) rated an install flow MEDIUM for "download full skill" instructions; I did not see this in the upstream SKILL.md (UNVERIFIED which text it refers to). Install CLI `npx skills` is a third-party tool: use manual copy instead.
- **React+Vite fit:** examples assume Tailwind/shadcn (`bg-card border-border text-muted-foreground`), which the project does not use; the principles (semantic tokens, native first, states) match.
- **Overlap with apple-design:** moderate (hierarchy, restraint, tokens); no motion physics. Adds: domain exploration and "signature element" ideation, which suits infrastructure concepts if bound to real data.
- **Class:** RECOMMENDED FOR ADOPTION (optional), project scope, with the overrides in section 6.

### 2.6 mastepanoski/claude-skills `nielsen-heuristics-audit`

- **Source (FACT):** https://github.com/mastepanoski/claude-skills, HEAD `ac3023830834f9c7c181649f31fbfd48c91837d3`; MIT (`LICENSE`: "Copyright (c) 2026 mastepanoski"); README says 14 skills, version 1.10.0; REPORTED ~44 stars / 574 installs on an aggregator (small, low maturity).
- **Path:** `skills/nielsen-heuristics-audit/SKILL.md` (8.9 KB) with `references/heuristics.md` (not read). Inputs: interface description, screenshots, flows. Scores the ten heuristics, prioritises. Contains a "Security Notice" treating fetched URLs/screenshots as untrusted (OWASP LLM01) with an injection phrase list. No scripts.
- **Value:** a clean, static, low-risk rubric; Impeccable's `critique` is stronger but heavier. Overlap with apple-design low.
- **Class:** RECOMMENDED FOR ADOPTION as a vendored static rubric (maturity low, so treat as text to adapt).

### 2.7 Vercel Web Interface Guidelines

- **Source (FACT):** `vercel-labs/web-interface-guidelines` HEAD `434b7f91364665f2f733b310ec54809bf8f37937`, MIT (copyright 2025 Vercel Labs); `command.md` 8 KB of static rules (accessibility, focus, forms ...). The skill `vercel-labs/agent-skills/skills/web-design-guidelines/SKILL.md` (agent-skills commit `1a2c79d4e6c4a2283dc2c614b6538cbe1ed5ca62`, 2026-10-27; repo licence field null) instructs: "Fetch fresh guidelines before each review" from `raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md`: a mutable remote instruction channel. R3 owns its engineering assessment and agrees: vendor a pinned copy. Rules include Tailwind-flavoured items (`focus-visible:ring-*`).
- **Class:** USEFUL REFERENCE ONLY.

### 2.8 Official design-system guidance

- **Carbon "Status indicators"** (https://carbondesignsystem.com/patterns/status-indicator-pattern/, "last updated Aug 12, 2026" per page): do not rely on colour alone; combine at least two cues (colour, shape, symbol); 3:1 contrast against adjacent colours; icon indicators need a text label. **Primer "Data visualization"** (https://primer.style/product/ui-patterns/data-visualization/): text 4.5:1, axes and marks 3:1, distinct stroke styles and markers per series, 2px lines, legend when more than one item, muted colours need an outline. Both agree with the existing provenance rule (shape and text, never colour alone). Licences of the sites' content not checked (UNVERIFIED). Class: USEFUL REFERENCE ONLY.
- **Style Dictionary** 5.7.1 (npm registry 2026-10-30): Apache-2.0, 13 dependencies, 4.4 MB unpacked, no `postinstall`; `@tokens-studio/sd-transforms` 2.0.3 MIT. Purpose: transform token JSON to many platforms. PhantomCache has one platform and `ui/src/styles/tokens.css` as the source; adding it is a new dev dependency with no benefit. NOT SUITABLE now; revisit only if tokens must be shared with non-web targets.
- **Token skills** (glebis `design-tokens`: DTCG 2025.10, dependency-free Python, REPORTED 337 stars; plugin87 `design-tokens`: three-tier) were seen only in snippets. REQUIRES FURTHER INVESTIGATION (low priority; the repo already has a token doc and tests).

---

## 3. Comparison across the evaluation axes

| Axis | Impeccable | taste-skill | frontend-design | ui-ux-pro-max | interface-design | Nielsen audit (mastepanoski) |
|---|---|---|---|---|---|---|
| Popularity (stars, REPORTED API) | 96k | 106k | 185k (whole repo) | 143k | ~5k (REPORTED) | ~44 (REPORTED) |
| Technical quality (read) | high, detailed, mode-aware | high, measured study, but page-centric | concise, good writing | broad data, mixed | focused, strong ideas | adequate, generic |
| Maturity | versioned engine/CLI/skill | v2.1.x | org-maintained | v2.15 | calendar versions | v1.10 of a small set |
| Suitability for PhantomCache | high for critique; low for generation | low (app-screens ref only) | low | medium-low | medium-high | medium |
| Scripts/binary | binary + hooks + browser script | Node scripts, network, Chrome | none | Python, writes | none | none |
| Licence | Apache-2.0 | MIT | Apache-2.0 | MIT | MIT | MIT |
| Prompt volume | very large | very large | small | large (data) | medium | small |

Popularity is reported but not used for ranking: the two largest star counts (taste, frontend-design) belong to the two candidates least suited to a dense engineering tool.

---

## 4. Conflicts (with PhantomCache rules and with each other)

| ID | Conflict | Sources | Neutralisation in a coordinated workflow |
|---|---|---|---|
| X1 | Decorative / distinctive-by-default: "aesthetic risk", "out-of-distribution craft", "do not trade it [richness] for restraint", `bolder`/`delight`/`overdrive`, glass/aurora/neumorphism catalogue | frontend-design, Impeccable SKILL.md, taste-skill, ui-ux-pro-max styles.csv | Precedence rule: project rules and DD v2 outrank any external skill; those skills load only for named review tasks, never for generation; disable/ignore `bolder|delight|overdrive`; Operate mode only. |
| X2 | Eyebrows and ALL-CAPS labels: banned outright by Impeccable ("a ban, not a default: no brief earns it back") and discouraged by frontend-design; DD v2 specifies page-head eyebrows and "small caps-style labels with wide tracking" | Impeccable craft-floor; frontend-design; DD v2 section 4.4/8 | DD v2 plus the owner's brief win; record as a documented exception (labels carry provenance function). Do not let a detector auto-fix them. |
| X3 | Hero-metric ban ("big number, small label, supporting stats, accent") vs DD v2 large tabular numerals for results | Impeccable; frontend-design; DD v2 | Allowed when the number is real, labelled with provenance and sits in the answer page; note taste-skill's own finding "KPIs are type, not tiles" agrees with DD v2. |
| X4 | Sparklines: Impeccable bans them as filler; taste says none on KPIs; ui-ux-pro-max `chart` domain and Scenario B time series may want them | Impeccable, taste, ui-ux-pro-max | A sparkline is allowed only when plotting received values with axis semantics (DD v2/ R2 decide); ban "decorative" sparklines. |
| X5 | Fonts: taste "the dealt typeface, really loaded" (webfonts), ui-ux-pro-max `google-fonts` domain, Impeccable `overused-font` detector (flags Inter), vs project system font stack and no font files | taste, ui-ux-pro-max, Impeccable | Pin system stack in a project override; add `ignore-value overused-font` equivalent; never fetch webfonts. |
| X6 | Placeholder / invented content: frontend-design ("come up with copy and placeholder content"), taste ("Sample details are expected", "invent" a signature element), ui-ux-pro-max sample data | those three | Hard rule: no invented data. Placeholders only inside prototypes explicitly labelled SIMULATED or UNAVAILABLE; "signature element" must be built from real objects (see section 7). |
| X7 | Fragmented source of truth: PRODUCT.md+DESIGN.md (Impeccable), `.interface-design/system.md`, `design-system/MASTER.md` (ui-ux-pro-max), `.taste-skill/` | all | Adopt at most one persistence file; point it at `docs/design/tokens.md`/DD v2; refuse `--persist`, `init`, `document` unless the lead approves. |
| X8 | Libraries: interface-design (Radix/React Aria/Ark/Headless UI/cmdk; shadcn, Tailwind examples), ui-ux-pro-max (GSAP, shadcn, Tailwind stacks) vs no new runtime dependency and no component library | interface-design, ui-ux-pro-max | Use "native HTML first" and "hand-roll with full contract" branch; reading of "no dependency budget" in interface-design already supports this. Any primitive library needs approval (purpose, alternatives, maintenance, security, dev-only). |
| X9 | Glass/blur: Impeccable permits blur and backdrop-filter as "palette"; apple-design materials; DD v2 permits glass only on functional layers with fallbacks and measured cost; Impeccable also bans "glass as decoration" | Impeccable, apple-design, DD v2 | DD v2 section 4.3/5 governs; "glass never carries data". |
| X10 | Process mandates that compete: Impeccable's required launcher run, banner texts, sub-agent isolation, "question is LAST"; taste's forced direction draw; apple-design's fixed "Initial Response" sentence | Impeccable, taste, apple-design | Run at most one external-process skill per task; the lead assigns a primary; never combine two generation skills. |
| X11 | Motion language: apple-design springs (S3/S4) vs Impeccable "exponential ease-out", taste scroll scenes, ui-ux-pro-max GSAP | various | DD v2 section 6 plus R2: CSS/WAAPI only, 90-280 ms, state-driven. |
| X12 | Impeccable sets `Modal as first thought` as laziness; DD v2 uses dialogs for cancel confirmation | Impeccable operate.md vs DD v2 | Destructive confirmation dialogs are legitimate; keep. |

A coordinated order that neutralises these: (1) PhantomCache rules and DD v2 first; (2) apple-design for feel and motion; (3) external critique/rubric skills only for review, outputting findings against that stack; (4) generators off.

---

## 5. Overlap with the installed `apple-design` skill

`.claude/skills/apple-design/SKILL.md` (MIT, pinned a81dbca0, cited in project docs) teaches fluid, interruptible motion, springs, materials, typography (optical sizing, tracking, leading), reduced motion, eight principles; its first reply is a fixed sentence unless asked a concrete question (project rules section 1 say to ask concretely). None of the candidates teach gesture physics or materials, so apple-design is not replaced. Overlapping areas: typography hierarchy (Impeccable `typeset`, interface-design), restraint and feedback (Impeccable Operate, interface-design), accessibility floor (all). The unique contributions of the candidates are: critique method with scoring (Impeccable, mastepanoski), anti-slop review prompts (frontend-design, Impeccable, taste), domain-specific "signature" ideation (interface-design, taste), honest data states (taste app-screens), ready UX rule data (ui-ux-pro-max).

---

## 6. Proposals (not executed)

### 6.1 Primary authorities (PROPOSAL)

| Role | Proposed primary | Supporting | Why |
|---|---|---|---|
| Art direction | `apple-design` + DD v2 (project documents) | none external | Only authority without a decorative mandate; DD v2 already defines the direction |
| Design-system intelligence | Project docs (`docs/design/tokens.md`, `phantomcache_design_system_spec.md`, `ui/src/styles/tokens.css`) | `interface-design` principles (semantic tokens, native-first), Carbon/Primer as guidance | No generator beats the owner-approved tokens; ui-ux-pro-max/Style Dictionary add churn |
| UX critique | Impeccable `critique` method (after vetting) | interim: vendored Nielsen rubric (mastepanoski), taste `critique.md` passes, R3 tooling | Scored heuristics plus detector evidence; interim path avoids the binary |
| Existing-app redesign | Impeccable "Refinement preserves; redesign replaces" principle and Operate mode, applied by hand, subordinate to DD v2 | `interface-design` for product-UI craft | The redesign direction is set by DD v2; skills only check it |

### 6.2 Installation proposals (APPROVAL REQUIRED; none executed)

All commands copy files from a pinned checkout in a temp folder into the repo's `.claude/skills/`, so no plugin marketplace tracks `main`, no global scope and no hooks are involved. Run only after the owner's explicit approval, and view the diff of every copied file first. Replace `D:\Temp\...` paths as needed.

**A. `interface-design` (RECOMMENDED, optional)**
```
git clone --no-checkout https://github.com/Dammyjay93/interface-design.git D:\Temp\toolkit_install\interface-design
git -C D:\Temp\toolkit_install\interface-design checkout 2f9be3206855bcb2d1d0af262c8bae25cba6658d
# inspect the whole .claude/ tree first (only SKILL.md was read by R1)
# then copy .claude\skills\interface-design -> D:\Personal\phantom-cache\.claude\skills\interface-design
# copy LICENSE -> D:\Personal\phantom-cache\.claude\skills\interface-design\LICENSE   (MIT, Damola Akinleye, must be preserved)
```
Expected location: `.claude/skills/interface-design/SKILL.md` (+ LICENSE). Add a project-rules doc (like `apple_design_skill_project_rules.md`) stating: ignore Tailwind/shadcn/headless-library examples; do not write `.interface-design/system.md` (use `docs/design/tokens.md`); no invented data.

**B. `nielsen-heuristics-audit` (RECOMMENDED, interim critique rubric)**
```
git clone --no-checkout https://github.com/mastepanoski/claude-skills.git D:\Temp\toolkit_install\mastepanoski
git -C D:\Temp\toolkit_install\mastepanoski checkout ac3023830834f9c7c181649f31fbfd48c91837d3
# read skills\nielsen-heuristics-audit\references\heuristics.md first (not read by R1)
# copy skills\nielsen-heuristics-audit -> D:\Personal\phantom-cache\.claude\skills\nielsen-heuristics-audit ; copy LICENSE alongside (MIT, mastepanoski)
```
Alternative with no install: paste the ten heuristics into a project-owned `docs/design/critique_rubric.md` (heuristics are public concepts from Nielsen Norman Group) and add the PhantomCache checks (provenance, unavailable-not-zero, cancelled/failed/interrupted run page, gateway-stopped page, connection state).

**C. Impeccable (HOLD; REQUIRES FURTHER INVESTIGATION)**
Before any install, the owner would need: (1) hash-check and static review of the windows-x64 engine (`8f61e96b...d9f` per `engine-checksums.txt` at the pinned commit), preferably built from source (the repo contains Rust crates); (2) a decision to keep hooks OFF (the supplied `.claude/settings.json` is the repo's own, not copied by a skill-only install); (3) `IMPECCABLE_BIN` set to the vetted binary so the launcher never downloads. Pin: commit `02523aec3e4c4580102d996d1b6a71fbe062e03f` (skill-v4.8.0, engine-v0.1.18). Files to preserve: `LICENSE` (Apache-2.0) and `NOTICE.md`. Expected location: `.claude/skills/impeccable/`. Proposal until then: vendor as read-only reference only `reference/operate.md`, `reference/craft-floor.md` and the Nielsen guide section of `reference/critique.md` under `docs/research/` with Apache notices (not as a skill).

**D. Not to install:** taste-skill (Chrome automation, network assets, global symlinks), frontend-design for app UI, ui-ux-pro-max as a generator, Style Dictionary.

### 6.3 Dev-dependency note
`npx impeccable detect` or `npx skills` would be an npm executable, so a dev-only dependency decision (purpose: design linting; alternatives: the repo's own `ui/scripts/check-*.mjs`; maintenance: active; security: downloads a binary; dev-only: yes). Not requested here; the repo's scripts already cover forbidden patterns, contrast and a11y lint (R3).

---

## 7. How these skills can help communicate PhantomCache's actual objects and workflows

Sources: R4 `r4_system_objects_and_motion.md` (object register O1-O52, lifecycles L1-L7, section 5 coalescing, section 7 stories), R5 coverage notes, DD v2. R1 did not verify R4's code references; they are used as stated by R4 (FACT per R4).

| Real object or flow (R4) | What the candidates can help with | What they cannot do (data or events missing) |
|---|---|---|
| Requests and outcomes: GET/DELETE, HIT/MISS/STALE/COALESCED/shed (O1, O6-O9, O14) | Nielsen "visibility of system status"; Carbon "two cues" so HIT/MISS/STALE/shed are never colour-only; Primer chart rules (distinct strokes, legends); taste app-screens "colour goes on data", "zeros print as zeros"; Impeccable Operate "state-rich vocabulary" | Per-request records are not in the stream (aggregate counters only, O1); a request-level animation would be ILLUSTRATIVE. |
| Request coalescing (O11, R4 section 5) | Critique can demand that "followers vs leaders" never be presented as "requests avoided"; interface-design's domain exploration can name the concept (leader/follower) in plain words | No fan-in count per fetch, no join event, no wait durations, leader not counted (R4 FACT); any per-fetch convergence graphic must be labelled illustrative/replayed. Skills' "signature element" ideation must not invent this. |
| Cache entry lifecycle: fill, fence, TTL, expiry, sweep, evict (L1, O5, O16-O20) | Impeccable state checklist (default/loading/empty/error) applied to each entry state; apple-design S7 spatial consistency for state transitions | No entry-level stream (only `bytes_resident`, `evictions_total`; no entry count); TTL is a literal 60 s; generation-fence and sweeper events absent. State diagrams are documentation, not live data. |
| Invalidation and epochs (O21-O23, O29-O31) | Nielsen error prevention/recovery wording; frontend-design's writing rules (same verb through the flow, errors explain recovery) | No invalidation events in the stream; tenant/full scope has no HTTP route; repair ledger and anti-entropy have counters only in diagnostics. UI can only describe, not show live propagation. |
| Cluster nodes and connection states (O24-O33; DD v2 section 9) | Critique checklists for gateway-stopped and unauthorised states (R5 requests); Carbon status indicators; Impeccable "native expectations" | Node CPU/memory unavailable; `NODE_FAILURE` is a simulated flag (O33) and must render as SIMULATED; Offline/Degraded have no writer (O28). |
| Workload intelligence (O34-O39) | Honest-states guidance ("missing values as a grey dash", UNAVAILABLE blocks with reason) | Hot keys explicitly UNAVAILABLE (O35); pipeline/detectors/forecasters are library only and PREDICTED output does not exist at runtime (O36-O37). No skill can make this live; they can only keep the absence legible. |
| Experiments and runs: kinds, arms, run lifecycle, verdicts, retained evidence (O45-O49, L6) | Impeccable/taste critique of the answer-page structure (answer first, then record); "negative results same size and weight" matches DD v2; Nielsen "match with the real world" for arm names | T1 arms are SIMULATED (O46); verdicts are DERIVED (O48): a critique must check each carries its label; sealed M6 negative verdicts (H1 NOT_SUPPORTED) must not be softened by a "delight" or "make it bolder" pass. |
| Adaptive policy: origin gate, AIMD cap, canary, guardrails, audit (O13-O15, O40-O44, L7) | The only real series is origin cap/in-flight/rejections (R4 story 1); chart guidance (Primer, ui-ux-pro-max `chart`) for that series; honest framing (MEASURED_LOOPBACK notice) | Canary bucket routing has no caller (O41); revert/watchdog have no production caller (O42); audit store empty (O43); legacy PIE is a stub (O44); AIMD behaved as a static cap near 12 in retained data. No skill may present a "decision timeline" as real. |

**Rules for using any candidate here (PROPOSAL):** (1) a skill's ideas about "the one live thing", "signature element" or "scroll scene" may only be bound to R4 objects marked REAL or to retained/REPLAYED data with the stamp; (2) every state a skill's checklist asks for (empty, loading, error, degraded) must map to a real state in R4's lifecycle maps, otherwise it is UNAVAILABLE and shown as such; (3) critique output must list unlabelled values as defects first; (4) no "make it bolder" pass over evidence pages.

---

## 8. Instructions that I checked for hidden or injected content

- No hidden, encoded or tool-targeting instructions found in the files read. Directive blocks found (overt): Impeccable mandatory banners and ordering ("The question is the LAST thing in the response", "Context loading did not run; I'll read the existing project context directly.", sub-agent isolation), taste "Adopt it. Do not blend it", frontend-design process (plan, review, build), ui-ux-pro-max "REQUIRED for new pages/projects" design-system step and "Do not persist unverified output", mastepanoski's anti-injection notice (defensive).
- Install hooks: Impeccable only (SessionStart/PostToolUse/Stop via `.claude/settings.json`, plus manifests for other tools). taste `install.sh` creates symlinks into `~/.claude/skills`. None of the others.
- Network calls: Impeccable launcher (GitHub releases download), `live` mode; taste `assets.mjs` (four public image APIs); Vercel `web-design-guidelines` (fetches rules each run); ui-ux-pro-max none found in Python; others none.
- Shell commands: Impeccable launcher and hooks; taste `spawn` of Chrome and `taskkill`; ui-ux-pro-max Python invocations by the agent; others none.
- Telemetry: none found in the files read; the Impeccable engine and `live-browser.js` were not inspected, so absence is UNVERIFIED for them.
- Permission requests: Impeccable's installer asks consent for hooks; taste/ui-ux-pro-max imply Bash use. None asks to change global settings except `install.sh --all-agents` and `npx skills -g`.

---

## 9. Open items and unverified

1. Impeccable engine binary and `live-browser.js` unaudited; hooks behaviour only read, not run.
2. `interface-design` commands directory and any other files; `heuristics.md` of the Nielsen skill.
3. Release dates, contributor counts and issue responsiveness for all candidates (API exhausted).
4. Remaining 23 KB of taste `SKILL.md`, `motion.md`, `imagery.md`; ui-ux-pro-max `design_system.py` logic beyond writes.
5. Glebis/plugin87 token skills, StyleSeed, sickn33 `design-ux` (snippets only). Also not surveyed: Material 3, Polaris, Radix, shadcn guidance, WCAG linters outside R3's scope.
6. `docs/design/remediation/03_validation_accessibility.md` does not exist at the time of writing; its constraints are not reflected beyond the WCAG 2.2 AA rule in the brief and DD v2 section 15.
7. Whether a plugin-scope install honours `${CLAUDE_PLUGIN_ROOT}` for a manual copy of ui-ux-pro-max (not relevant unless adopted).

---

## 10. Cross-agent exchange summary

- **R4:** used the object register (O1-O52), lifecycles and coalescing deep dive as the basis of section 7. R1's three requests to R4 are in `coord/r1.md`; R4's replies had not appeared when this file was finalised; R4's main file already supplied the answers (provenance vocabulary per object via R5's note, 100 ms stream and ~250 ms retained series).
- **R3:** agreed on pinning `web-design-guidelines` locally (C1); concur that `frontend-design` is not suitable for app UI.
- **R2:** noted their finding that library-recommending skills conflict with the dependency gate; R1 found the same for interface-design and ui-ux-pro-max.
- **R5:** replies to R5's three requests are in `coord/r1.md`. R5 notified me that DD v2 exists (X-05); corrected here.
