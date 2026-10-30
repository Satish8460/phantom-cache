# A3: Apple design-skill adoption and UI design-quality audit (read-only)

Date of audit: 2026-10-30. Evidence tags: [V-T] verified in transcript record, [V-C] verified in repo file, [V-S] viewed screenshot, [INF] my inference.
Method note: transcript was searched with narrow Node scripts (user-role text records and tool_use records); no whole-file read.

## PART A. The instruction

Named resources: YES, three specific GitHub repositories, in the owner's pasted "ADDENDUM - PhantomCache M8" in the main transcript, line 1358, timestamp 2026-10-28T19:13:36Z, role=user [V-T]. Section 2 "Research and evaluate open-source Apple design skills", verbatim list:
- "Emil Kowalski's `apple-design` skill: https://github.com/emilkowalski/skills"
- "`dickwu/apple-design-skill`: https://github.com/dickwu/apple-design-skill"
- "`ykswang/apple-hig-skill`: https://github.com/ykswang/apple-hig-skill"

Intended role, verbatim: "Verify each candidate's current license, supported agent environments, maintenance activity, design scope, installation requirements, and suitability ... Recommend a minimal combination ... One primary design/interaction skill for implementation guidance. One complementary HIG/accessibility review reference ... Record repository URLs, license evidence ... and the proposed installation method. No installation or external dependency addition is authorized by this planning task."
So the instruction to the owner's planning agent was: evaluate, pick a minimal set, propose installation. Adoption itself was left to a later decision. The owner did not (in any user-text record I found) say "install/use X for the UI". Later owner directives (lines 1744 @2026-10-29T13:21Z, 2215, 2947, 2997 @2026-10-29T18:57Z, 5016) only say "Apple-inspired ... principles: clarity, restraint, hierarchy, precision, purposeful motion" and "Do not simply imitate Apple's visual styling"; none names a skill. Closest later quote (2997): "Create a coherent, premium interface inspired by Apple's design principles: clarity, restraint, hierarchy, precision, purposeful motion, and consistent interactions." No user record mentions "O8-1" (the adoption decision) at all.

Note: records 1524 and 1612 are role=user but are pasted agent summaries, not owner words.

Repo references [V-C]: only docs/research/phase_8_execution_package.md section 3 (evaluation record, "PROPOSED - not authorized"; O8-1 owner decision; M8-02 "Skill evaluation closure and adoption", AC8-3 "adoption (if any) follows section 3 install method; SOURCE.md present") and docs/research/skill_strategy.md (W0, dated 2026-10-28, environment skill inventory). docs/design/principles.md line 5: "No external design skill was installed or adopted; this document applies the principles ... by hand." No hits for emilkowalski/dickwu/ykswang in ui/**, package.json, ledger, claims ledger, STATUS. .claude/ and docs/design/reference/ do not exist; no SOURCE.md.

Design-related skills present in this environment [V-env listing]: artifact-design, artifact-diagramming, dataviz, figma:* (design-to-code, generate-design, generate-library etc.), anthropic-skills:pptx/docx/docs, plus run/built-in-browser. Adobe MCP also advertises a "frontend-design" skill routing but no such skill is in the skill list. None of the three named Apple skills is installed. skill_strategy.md lists dataviz/artifact-design as "listing only" (not read).

Skills actually invoked in the whole project history: I searched tool_use records named exactly "Skill" in the main transcript and all 138 subagent transcripts: ZERO invocations [V-T]. No artifact-design, dataviz, or any skill was ever loaded by any agent. (Tool-results folder not searched for Skill; tool_results cannot contain invocations.)

## PART B. Was the resource actually used?

| Step | Finding | Evidence |
|---|---|---|
| 1 Discovered | YES | owner addendum 2026-10-28T19:13:36Z |
| 2 Fetched/read | YES, as a repository evaluation only | subagent a5da566e0abc16ccb, 2026-10-28T19:15:14Z-19:21:43Z: Bash curl of api.github.com repo/tree JSON and raw.githubusercontent.com files (AGENTS.md, SKILL files, scripts/pull-hig.mjs) into a scratchpad; lead Bash curl of GitHub API licence/HEAD at 2026-10-28T19:25:57Z (main line 1465). No WebFetch/WebSearch for these (WebFetch records were all literature). |
| 3 Installed/loaded | NO | No Skill tool record; no `npx skills` run (grep of Bash/PowerShell inputs found only curl/GitHub API); no .claude/skills; evaluator explicitly did "no installation, no scripts executed" |
| 4 Cited in decisions | YES, in a rejection/deferral sense | phase_8 package section 3 recommends a pinned copy of Emil's `emil-design-eng` (MIT, HEAD a81dbca...) pending owner approval O8-1; rejects ykswang; dickwu only as local reading aid (unlicensed, embeds Apple text). Owner approval of O8-1 never appears. |
| 5 Reflected in tokens/components | NO as a resource | M8 implementer brief (subagent a81226d741cf38a1c, 2026-10-29T13:31:46Z) says verbatim "Do not adopt external skills or install anything; you may follow principles from docs/research/skill_strategy.md." Its report (main line 2122) says "No Apple assets, SF fonts or external skills were used." principles.md confirms. |

Classification: the named skills were READ ABOUT as repositories (licence and file-tree evaluation), never used. UI is "independently implemented similar principles" (honesty/provenance, restraint, tokens, reduced motion) by the agent. Whether the agent read Emil's SKILL content during the evaluation: the evaluator downloaded some of its files (a5da566e line 81 curl of emil-design-eng, review-animations ...), so the evaluator saw them; the implementer did not [V-T]. The owner's doubt is therefore justified: the skill was not applied. It was also not formally rejected by the owner; it was left pending (O8-1 unresolved) and the implementer was told not to adopt it. A decision the owner never made was effectively made by omission.

One deviation: the owner's phrase "Apple-inspired" in later directives was satisfied via a self-authored prose interpretation (principles.md), not the named material.

## PART C. Current UI design quality

Rubric (qualitative, no scores): for each area I state Strong / Adequate / Weak against (a) internal coherence, (b) hierarchy and legibility, (c) evidence in CSS, (d) evidence in screenshots. Light/dark/mobile were viewed in screenshots; interactions (hover, press, transitions in motion) were not observed live [limitation].

Screenshots opened (14), under docs/demo/browser_verification/:
20261030T045605Z/shots/matrix/ desktop_light__.png, desktop_dark__lab_run.png, desktop_light__lab_evidence_scenario_b_20261030T003932Z.png, laptop_light__traffic.png, desktop_dark__simulation.png, desktop_light__cluster.png, mobile_dark__lab.png; 20261030T045605Z/shots/ skip_link_focus.png, mobile_nav_open.png; 20261030T055008Z/shots/ r3layout/m6_dark_1280.png, r3layout/reflow_390_lab_run.png, r3layout/reflow_390_root.png, r3clips/start_form_1280.png, plus 20261030T045605Z/shots/matrix/laptop_light__traffic and the Lab connection view in start_form_1280.png (the latter pair counted above). Tablet-width and forced-colors/reduced-motion shots were NOT opened by me.

1. Type hierarchy and scale. Adequate, restrained, shallow. Scale 12/13/15/18/22/24 px (tokens.css:9-14), h1 22 px, h2 15 px, h3 13 px grey (app.css:17-20). --text-lg (18 px) is defined and unused in the CSS I read [V-C: grep showed no use]. Result [V-S desktop_light__.png]: page title only slightly larger than body; panel titles (15 px semibold) barely differ from body, so hierarchy rests on weight and whitespace. Good: tabular numerals globally (app.css:14), mono only for ids. Weak: no display tier for the key metric beyond 24 px; metric labels and values are close in scale. System fonts only (tokens.css:7), so on Windows renders Segoe/Arial-like; no distinctive typographic voice (neutral, by design).
2. Spacing, rhythm, alignment. Adequate to strong in structure: 4 px grid tokens (tokens.css:23-29), consistent panel padding (app.css:91). Weak spots [V-S]: Overview metric grid uses auto-fill 11rem tiles so the six tiles stop short of the panel width leaving an unbalanced right gap; the Capabilities panel has a large empty lower half beside the taller Connection panel (desktop_light__.png); Cluster/Intelligence placeholders are a huge empty canvas under two small boxes (desktop_light__cluster.png); the sticky nav ends mid-page leaving a dead column (visible in every desktop shot).
3. Navigation and IA. Adequate. Grouped side nav (Live Gateway / Phantom Lab / Not yet available / Simulated), skip link, arrow-key nav, Menu disclosure on narrow screens (Shell.jsx; app.css:55-84). Issues [V-S]: "Not yet available" screens occupy first-level nav; the Phantom Lab flagship is a single nav item with a sub-tab row (Overview / Evidence / Start a run / Runs) of lower visual weight; the Lab page begins with Connection panel and two stacked banners (debug build + "intelligence not part of serving path") that push the actual content below the fold at 1280x800 (start_form_1280.png / desktop_dark__lab_run.png). Honest, but the primary task is buried. On mobile the chrome is three stacked rows (title, status/theme, Menu) before content (reflow_390_root.png, mobile_nav_open.png).
4. Surfaces, borders, shadows, materials. Coherent and very flat: 1 px borders, radius 4-8 px, no blur or gradients (principles.md section 3). Shadow tokens exist (tokens.css:39-40) but are used nowhere (grep count 0), so there is effectively no elevation system; all depth comes from borders and surface-2 fills. Nested panel-in-panel-in-card stacking is heavy in Lab (lab-card inside panel with borders on each, lab.css:25) [V-S m6_dark_1280.png]. Many border styles encode meaning (dashed, double 3-4 px, dotted) which creates visual noise: the SIMULATED banner and sim-frame use double 4 px borders (app.css:130,172) that read as heavy rather than refined.
5. Colour and dark mode. Strong on rigour, adequate on aesthetics. Full light and dark token sets (tokens.css:59-95, 97-177), blue accent, semantic ok/warn/critical/info plus five provenance colours; contrast scripted. Dark mode screenshots are legible and consistent (desktop_dark__lab_run.png). Weaknesses: dark token block is duplicated verbatim twice (tokens.css:97-136 and 139-177), a maintenance risk; ten-plus tinted status colours create a busy palette on Evidence pages (m6_dark_1280.png: red, amber, blue, green chips at once); active-nav and info colours are identical blue, so "selected" and "informational" collide.
6. Motion and feedback. Minimal and policy-driven: only 120 ms background fades on nav and button (app.css:65,139); reduced-motion collapses durations twice (tokens.css:187-192; app.css:175-177). Defined --dur-base, --ease-out beyond fast fade, and --target tokens are barely used. No press state beyond background (btn:active, app.css:141), no pointer-down response, no interruptible transitions, no progress choreography. Appropriate to an ops console, but there is no "interaction polish" layer, which was the stated content of the Emil skill. Not verified live [limitation].
7. Buttons, forms, controls. Adequate but inconsistent. One .btn / .btn--primary / --danger (app.css:139-143; lab.css:17-18) is coherent; native checkboxes and radios at 24 px (lab.css:15) with accent-color are functional but visually default; the Start-a-run form is a long, unbroken stack of ~11 scenario checkboxes then workload, arms, seeds (reflow_390_lab_run.png, start_form_1280.png): dense, uniform, no grouping, no selected-summary, no progressive disclosure. Inline validation "Problem: Choose at least one scenario." shows immediately in red on a pristine form (start_form_1280.png), i.e. errors before interaction. Theme control is a text button reading "Theme: system" with a cycle behaviour (Shell.jsx), not a segmented control. The Simulation screen uses its own bespoke buttons (RUN/PAUSE/0.25x/1x/2x) not the shared .btn, 21 px high at 1280 (report r3 N-7).
8. Responsive behaviour. Strong on mechanical reflow (grids with min(100%,..), WCAG 1.4.10 rules app.css:189-208; 15 routes x 5 widths observed by the browser round). Visual quality at 390 is acceptable but long: tables wrap field names mid-token ("origin_c / ap.cap", reflow_390_root.png); full-page Evidence is thousands of pixels tall. Tablet layout was not viewed by me.
9. Accessibility. Strong. Visible 2 px focus (app.css:26-29; skip_link_focus.png shows a clear focus ring), skip link, targets 32/44 px, forced-colors and prefers-contrast hooks, shape+text+border-style for status. Caveat: desktop_light__cluster.png shows a focus ring on "Cache & memory" while "Cluster" is current (probably keyboard test state, not a defect).
10. Empty/loading/error/success/disabled. Strong in taxonomy (States.jsx: loading/empty/error/disconnected/unavailable/notEnabled; ConnectionBanner; .btn:disabled dashed, lab.css:16), visually plain: dashed grey box with icon and sentence (desktop_light__cluster.png). Loading is a static icon (no skeleton or spinner by policy), and no loading screenshot exists in the sets I opened [not verifiable visually]. Success states barely exist.
11. Restraint and coherence. The live screens and Lab share one language, but the Simulation demo is a different product: neon orange/cyan on black, monospace, uppercase labels, tiny 8-10 px text (desktop_dark__simulation.png) inside a double-border frame. It is legacy SimDemo.jsx (src/SimDemo.jsx) wrapped, not redesigned. Its text sits well under the 12 px minimum claimed in principles.md section 3. This directly contradicts "one type scale / minimum 12 px" and is the strongest coherence defect. Also the product feels like a documentation/report viewer more than an instrument: dense prose, monospace notice dumps (desktop_light__lab_evidence_scenario_b...png shows a long pre-formatted markdown text block with literal ** markup), 1,000+ px of text per verdict.
12. Charts. Text-first bars/effect-size glyphs (LabCharts.jsx, lab.css:71-96) with "View as table" alternatives are honest and accessible; visually sparse (thin bars, small markers, m6_dark_1280.png) and the live Traffic latency "chart" is a set of empty grey tracks labelled "unavailable" (laptop_light__traffic.png). The dataviz skill (palette/validator) was never loaded.

Net visual assessment: disciplined, honest, accessible, plain. It is a competent token-based utility UI. It is not "premium" in the Apple sense: hierarchy is flat, there is no elevation or material language, little imagery of system behaviour, content density high on Lab pages, and one screen is stylistically alien.

## PART D. Gap classification

| Item | Class |
|---|---|
| Use of emilkowalski / dickwu / ykswang skills | Not implemented (evaluated only; owner decision O8-1 unresolved) |
| "Skill genuinely adopted" | None |
| Honesty/provenance badges, data-state taxonomy | Principles adopted without the skill (own design; strong) |
| Clarity, restraint, deference (flat surfaces, system fonts) | Principles adopted without the skill |
| Typographic hierarchy and display scale | Partially implemented |
| Elevation/material system (shadow tokens unused) | Partially implemented (tokens exist, unused) |
| Purposeful motion and interaction feedback (pointer-down response, springs, interruptibility) | Not implemented (only 120 ms fades; reduced-motion implemented) |
| Light and dark themes | Implemented (token level); aesthetic polish Partially |
| Responsive reflow | Implemented mechanically; visual composition Partially |
| Accessibility mechanics (focus, targets, contrast scripts, forced-colors) | Principles adopted (strong) |
| Design coherence across screens | Partially implemented (Simulation demo diverges) |
| Form UX (Start a run) | Partially implemented |
| Loading/skeleton visuals | Not verifiable (no screenshot opened) |
| Formal design approval O8-2 | Not implemented (principles.md line 3: approval not recorded) |
| Any usability test | Not verifiable; none found |

What the verification does and does NOT establish.
- Browser rounds (reports under 20261030T041006Z, 045605Z, 055008Z; headless Chrome 156 over CDP, debug build, one Windows host): establish no horizontal overflow at 320-1280 (74/75 strict), target sizes, focus visibility, favicon, console state, contrast ratios as computed, keyboard path. The reports state PASS means "no failure observed at the sizes, themes and routes tested". They establish nothing about hierarchy, aesthetics, perceived quality, comprehension, task time, or comparison with any reference. Agent viewed only a handful of shots; "about 70 other PNGs were measured only". The 275-screenshot count is measurement volume, not design review.
- The ~212 UI tests (my pattern count 218 across ui/test/*.mjs; claim not re-run) are node:test static CSS/source scans and server-render DOM checks. lab.render.test.mjs line 2 states "No browser ... Visual quality is not tested here." There is no pixel, screenshot, or layout assertion and no design-quality assertion. Passing proves token use, DOM labels, forbidden-pattern absence, and logic, not good design.
- Contrast script and a11y lint are conformance heuristics, not visual review. Tests and scripts were not executed by me.

## PART E. Recommendations

1. Should the named resource be studied before further UI refinement? Yes, but as reference reading, not as a dependency, and only the part that fits. Emil Kowalski's `emilkowalski/skills` is MIT per the project's verified evaluation (not independently re-verified by me now; I did not fetch it) and web-native; it is the one worth reading for interaction and motion standards. dickwu and ykswang are unlicensed and embed Apple text: do not copy into the repo. Use Apple's public Human Interface Guidelines (developer.apple.com/design) and WCAG 2.2 as public guidance for principles only; copy no Apple wording, assets, fonts or icons.
2. The owner must resolve O8-1 explicitly (adopt reviewed Emil file(s) pinned to SHA with LICENSE and SOURCE.md, or formally decline). Today the instruction is neither fulfilled nor closed.
3. Reusable principles to make the PhantomCache design system (all non-proprietary): (a) content-first hierarchy with a defined display/title/section/body/caption scale and a distinct metric tier; (b) one elevation scale actually applied (flat/raised/overlay); (c) interaction standards: immediate pointer-down feedback, 100-200 ms eased transitions, interruptible, reduced-motion path, no animation of live values; (d) states library: loading skeleton, empty, error, success, disabled, stale, unavailable, simulated with one visual grammar and a screenshot-tested gallery; (e) provenance badge (already good) kept as signature element but reduced to fewer border styles; (f) form patterns: grouped fieldsets with select-all/summary, errors only after interaction, sticky submit with live "exact request" preview; (g) tabular/mono numeric rules; (h) one navigation hierarchy with flagship Lab promoted and placeholders collapsed; (i) charts per the dataviz skill's method (own palette) with table alternative; (j) theme control as segmented System/Light/Dark.
4. Component list to formalize: Shell/Header, Nav, PageHead, Panel, MetricTile, ProvenanceBadge, StatusPill, Banner (info/warn/simulated), StateBlock, FieldTable/DataTable, Tabs, FormField/Checkbox group/Radio card, Button (primary/secondary/danger/disabled), Chart primitives, ResultCard/Verdict chip, Markdown notice renderer (replace raw `**` text).

Next steps (separate from evidence; owner approval needed for dependencies, tool installs, and design-approval record):
- Owner decision on O8-1 and O8-2.
- Read Emil's repository files locally (reading needs no install), then write a one-page standards note mapped to tokens.css.
- Restyle or re-home the Simulation demo to the shared tokens or isolate it clearly as a separate legacy view; fix its 21 px controls and sub-12 px text.
- Reduce Lab page chrome above the fold (collapse Connection and banners to a status strip with expandable details).
- Apply or delete unused tokens (shadow, --text-lg, --dur-base); deduplicate dark theme block (generate or use a single selector list).
- Add a human design review (with someone other than the authoring agent) of at least the 14 screens in light/dark at 390/820/1280, and a visual-regression baseline if the owner approves a dev dependency.
- Add screenshots of loading, empty, error, success, disabled states to the review set.

## Unverifiable / not done
- Whether any agent mentally used Emil's content: only the evaluator downloaded it; no record shows the UI implementer read it.
- Tablet, forced-colors, reduced-motion shots, hover/press/animation behaviour: not viewed or observed.
- Licence/HEAD facts for the three repos: relayed from transcript and docs, not re-fetched.
- UI test count 212 vs 218 by my crude pattern count; not run.
- Owner may have given verbal/other-channel approvals not in this JSONL.
