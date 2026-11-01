# 05 Before / after comparison (UI/UX remediation, agent G)

Status: working document. Every statement tagged OBSERVED was made by one agent (G) opening both PNGs with the Read tool. This is NOT an independent human review and does not claim visual quality, accessibility conformance or approval. Numbers in the height table are READ FROM THE TWO MANIFESTS (reproduced by script), not estimated.

## 1. Conditions and comparability

| Item | Before | After |
|---|---|---|
| Manifest | `docs/design/remediation/before/manifest.json` | `docs/design/remediation/after/manifest.json` |
| Script | earlier revision of `ui/scripts/browser-gallery.mjs` (old Lab selectors) | current `ui/scripts/browser-gallery.mjs` (current Lab: tabs, choice cards, presets, native dialog) |
| `ui/src` sha256 | `bcbfa3b6...c4960db` (210 files in gallery = 210 PNGs) | `ce7f641e5448441fbc22ba0303ed2e0922ca6c54df7da2785ef45794df80f344` (60 source files); recorded as `uiSrcSha256` and `afterUiSrcSha256` |
| Gateway exe sha256 | B31499965E96...E0E2 | B31499965E96FDB0B98325EECBE26B8A1B202ECC3122121DE1BCFB0BCEA7E0E2 (identical, verified by the run) |
| Chrome | 156.0.8078.79 headless | 156.0.8078.79 headless |
| Emulation | mobile:false, DPR 1, prefers-color-scheme, app theme on System | same |
| Fixture / recipes | anonymous dev gateway, lab dir with 2 evidence sets, Scenario B 1 rep BASE+STATIC-CAP 1 s phases; T1 smoke 1 seed; running Scenario B 5/30/30/30 s then cancelled | same intent, reached through the new UI (Quick check preset, "Smallest run" preset). Run ids differ (different run times). |
| Counts | 210 files, 72 full-page, 0 not reached | 210 files, 72 full-page, 0 not reached |

Distinct states in both manifests: 23 (the task text said 24; both manifests contain 23). All 210 before file names exist in after (0 missing).

Limits on comparability (OBSERVED / from manifests):
1. FULL-PAGE CROP FIX. Before `__full.png` files were cropped to 1265 / 375 px wide (scrollbar gutter, 15 px). After full-page files are 390 / 820 / 1280 px wide (captured by resizing the emulated viewport to the document height; no scrollbar). So full-page widths are NOT strictly comparable. Viewport-only files (`{state}__{w}__{theme}.png`) use the same emulation in both sets and are the comparable ones.
2. Because the layout width differs by 15 px, text wraps slightly differently, so full-page HEIGHT differences contain a small wrapping effect in addition to real changes. The crop itself does not change height.
3. Viewport shots are not always at the same scroll position. OBSERVED: `run_form_t1_empty__1280__light` (after) is scrolled down (header off-screen) because the script clicks a radio with scrollIntoView; before shows the page top. `run_form_t1_invalid__390__dark` (after) is also scrolled. This is a capture artifact, not a UI difference.
4. Different data in some pairs: run ids and timestamps differ; `run_form_t1_invalid` after presses Start run once (failed submit) while before only typed the values, so error visibility is a script difference as well as a UI one.
5. Single run each, single machine; no repetition of the capture.
6. When the viewport was resized for the full-page capture, any `100vh` layout would stretch. Not verified as harmless for every page.

## 2. Pairs opened with the Read tool (24)

All paths are under `docs/design/remediation/`. "Same conditions" = same state, viewport, theme, fixture type; "partly" where listed in section 1.

| # | State / viewport / theme | Before | After | Same cond. |
|---|---|---|---|---|
| 1 | lab_home 1280 light | before/lab_home__1280__light.png | after/lab_home__1280__light.png | yes |
| 2 | run_form_t1_empty 1280 light | before/run_form_t1_empty__1280__light.png | after/run_form_t1_empty__1280__light.png | partly (scroll position, item 3) |
| 3 | run_detail_sb_running 1280 light | before/run_detail_sb_running__1280__light.png | after/run_detail_sb_running__1280__light.png | yes (before shows an earlier moment of the run) |
| 4 | cancel_dialog 1280 light | before/cancel_dialog__1280__light.png | after/cancel_dialog__1280__light.png | yes |
| 5 | run_detail_sb_completed 1280 light | before/...__1280__light.png | after/...__1280__light.png | yes |
| 6 | evidence_m6 1280 light | before/evidence_m6__1280__light.png | after/evidence_m6__1280__light.png | yes |
| 7 | evidence_scenario_b 1280 light | before/evidence_scenario_b__1280__light.png | after/evidence_scenario_b__1280__light.png | yes |
| 8 | runs_history 1280 light | before/runs_history__1280__light.png | after/runs_history__1280__light.png | partly (after has 3 runs incl. cancelled; before opened at the top only) |
| 9 | overview 1280 light | before/overview__1280__light.png | after/overview__1280__light.png | yes |
| 10 | traffic 1280 light | before/traffic__1280__light.png | after/traffic__1280__light.png | yes |
| 11 | cache 1280 light | before/cache__1280__light.png | after/cache__1280__light.png | yes |
| 12 | freshness 1280 light | before/freshness__1280__light.png | after/freshness__1280__light.png | yes |
| 13 | origin 1280 light | before/origin__1280__light.png | after/origin__1280__light.png | yes |
| 14 | cluster 1280 light | before/cluster__1280__light.png | after/cluster__1280__light.png | yes |
| 15 | intelligence 1280 light | before/intelligence__1280__light.png | after/intelligence__1280__light.png | yes |
| 16 | simulation 1280 light | before/simulation__1280__light.png | after/simulation__1280__light.png | yes (page content differs by design, see below) |
| 17 | lab_home 390 light (mobile) | before/lab_home__390__light.png | after/lab_home__390__light.png | yes |
| 18 | cancel_dialog 390 dark (mobile, dark) | before/cancel_dialog__390__dark.png | after/cancel_dialog__390__dark.png | yes |
| 19 | run_form_t1_invalid 390 dark | before/run_form_t1_invalid__390__dark.png | after/run_form_t1_invalid__390__dark.png | partly (items 3, 4) |
| 20 | gateway_stopped_lab_home 1280 light | before/gateway_stopped_lab_home__1280__light.png | after/gateway_stopped_lab_home__1280__light.png | yes |
| 21 | overview 390 dark | before/overview__390__dark.png | after/overview__390__dark.png | yes |
| 22 | run_detail_t1_completed 820 dark | before/run_detail_t1_completed__820__dark.png | after/run_detail_t1_completed__820__dark.png | yes |
| 23 | lab_evidence_list 820 light | before/lab_evidence_list__820__light.png | after/lab_evidence_list__820__light.png | yes |
| 24 | run_detail_sb_completed 1280 light FULL (before cropped) | before/run_detail_sb_completed__1280__light__full.png | after/run_detail_sb_completed__1280__light__full.png | NO (widths 1265 vs 1280) |

Also opened (after only): `after/run_form_t1_invalid__1280__light__full.png`.
Not opened: the other 186 files (including most dark and 820 variants); statements below do not extend to them.

## 3. Observations per pair (all OBSERVED by agent G)

1. lab_home 1280. Before: the first screen is a large Connection card (auth text, schema, two "Available" rows, an amber debug-build banner, a blue intelligence banner) and the tab strip begins at y=706 with page content below the fold. After: header has a primary "Start a run" button, a single row of four status chips (Lab API enabled, Anonymous loopback, Debug build EXPLORATORY, Intelligence not in serving path) with a "Details" link; tabs at y=274 and the "The problem" text visible on first screen. Tab order changed from Overview/Evidence/Start a run/Runs to About/Start a run/Runs/Evidence. Header: the "Mode: live" text and "Theme: system" button became a Light/System/Dark segmented control; left nav gained "Experiments", "Explainer" groups and "Not available" chips on Cluster/Intelligence and a "Simulated" chip on Simulation. The debug-build and not-in-serving-path caveats are still present but reduced to chips (full wording behind "Details", not opened by me). Typography: heading larger and bolder.
2. run_form_t1_empty 1280. Before: after the Connection card the form starts at the fold ("What to run" radio with a SIMULATED badge, "Exact request" panel to the right). After (scrolled): two choice cards with REAL / SIMULATED badges and a one-line description each, "Experiment design" with a "Smallest run" preset and three collapsed groups showing "none selected, N available", and a sticky bar "Start run - 4 things still needed". The simulated/real label is on each card. Preset text says it "says nothing about the comparison".
3. run_detail_sb_running 1280. Before: only the run status chips are visible at the fold bottom (Running, MEASURED: LOOPBACK, id). After: run header card with Running, "REAL | loopback, one host, debug build", Elapsed 13 s with "DERIVED from timestamps", progress "0 of 2", "Live updates: live", Refresh and Cancel run buttons, then Progress with arm/repetition/phase and the latest five events, all on the first screen. Wording change: "MEASURED: LOOPBACK" became "REAL | loopback, one host, debug build".
4. cancel_dialog 1280. Before: the "dialog" is an inline red panel inside the Status card, not modal, the rest of the page is untouched. After: a centered modal with a dimmed backdrop, title "Cancel this run?", "Keep running" (first) and "Yes, cancel the run" buttons. Both state that the result is partial.
5. run_detail_sb_completed 1280. Before: the first screen shows only the Completed chip and id under the Connection card; result is far below. After: header card, "On this page" jump links, "Result at a glance" with "Run class: EXPLORATORY", STATIC-CAP "Inconclusive" (DERIVED), reason "only 1 valid paired runs; 10 required" and the mechanism-check line on the first screen. Verdict is visible without scrolling.
6. evidence_m6 1280. Before: first screen ends at the "RETAINED: SIMULATED" line; verdicts are below the fold. After: SIMULATED + Retained badges, amber SIMULATED banner ("NOT A MEASUREMENT of any network, host or process"), jump links and "Verdicts as recorded" (H1 Not supported, 0 of 8 scenarios on peak origin concurrency, 2 of 8 on client P99) on the first screen. The negative result is shown plainly.
7. evidence_scenario_b 1280. Before: first screen ends at the "RETAINED: MEASURED LOOPBACK" line. After: "REAL | loopback, one host" badge, a blue notice "Real, but only on loopback: one host, scripted origin ... not a production, WAN or multi-host measurement", jump links and the start of the binding interpretation notice (Rendered / Raw / Copy text).
8. runs_history 1280. Before (top of page): Connection card then a Refresh button; the table is below the fold. After: Run history table visible: id, kind, state chips (Cancelled / Completed / Completed), created, progress, provenance (REAL loopback one host / SIMULATED). After page fits the viewport (no scrollbar).
9. overview 1280. Before: one "Key signals" card with six tiles (throughput, hit ratio, GET p99, stale cap violations, resident bytes, origin in flight), Capabilities and Connection cards, all above or near the fold, including UNAVAILABLE tiles. After: page split into Traffic, Freshness guard, Memory and origin sections each with a "Details" link; first screen shows only Throughput and Stale cap violations (one tile each); the unavailable tiles (hit ratio, p99) are not visible on the first screen. Wording change: MEASURED became REAL on the stale-cap tile. Whether unavailable values still appear lower down was not checked.
10. traffic 1280. Same tiles; after has an eyebrow "LIVE GATEWAY", larger H1, no card borders around sections, a Chart/Table toggle on the latency percentile panel and a text line "Unavailable: p50, p90, p95, p99 are not reported yet"; before showed four "unavailable" bar rows. MEASURED became REAL (GET requests). Provenance still explicit; UNAVAILABLE badge kept.
11. cache 1280. After adds a labelled 0 to 256 MiB axis, a Chart/Table toggle and an "X Scale runs from 0..." note under the bar; sections lose their card borders. MEASURED became REAL.
12. freshness 1280. Same tiles; after drops the card frames, MEASURED became REAL; "Not part of the telemetry yet" replaces "Not available yet" heading (text partly seen).
13. origin 1280. After: unavailable notice has a solid left border instead of a dashed box, same text ("No origin values are shown, and none are zero"); table rows' provenance MEASURED became REAL for origin_cap.* counters; UNAVAILABLE stays for origin and cap.
14. cluster 1280. Before: dashed notice plus a card "What this screen needs ... See docs/design/ui_api_requirements.md". After: eyebrow "NOT AVAILABLE YET", notice, then "What this page would show" bullet list under "Nothing below is a value" and "What exists today instead" links. Reads as not-implemented, consistent with the nav chip "Not available". No fabricated numbers in either.
15. intelligence 1280. Same pattern as cluster; "Phantom Intelligence is not enabled / controller: none" retained in both; after lists the would-be content (Mode, Signals, Candidate policies, Decisions, Guardrails) as bullets explicitly "not a value".
16. simulation 1280. Before: a SIMULATED banner and an embedded dark orange "SIMULATED DEMO LAB MODE" dashboard with animated-looking tiles (88% hit rate, 40% coalescing, $-0.15 net saved), visually unlike the rest of the app but labelled synthetic. After: the demo is gone; the page is an explainer ("The earlier simulation demo has been retired. Simulated runs now live in Phantom Lab, labelled as simulated") with "What a simulated run is" and a Simulated-run vs Real-gateway-run comparison and no numbers. This is a removal of the in-app demo, not an improved demo. The simulated/real separation is clearer in wording.
17. lab_home 390 light. Before: header stacks title, Live chip, mode, URL, Theme button and a Menu button across three rows (about 190 px) before the H1; Connection card with a squeezed Authentication table. After: one-row header (PhantomCache, Live, Menu), "Start a run" button beside the H1, four wrapped chips, tab strip at y=359 and "The problem" text on first screen.
18. cancel_dialog 390 dark. Before: inline red panel, buttons "Yes, cancel the run" then "Keep running". After: modal over a dimmed page (dialog box lighter than the dimmed page), "Keep running" first. Backdrop contrast looked adequate to the eye; not measured.
19. run_form_t1_invalid 390 dark. Before (top of page): no error visible on first screen. After: a red summary "5 things need attention before this run can start" listing each field with links, then the cards and a sticky "Start run / 5 things to fix" bar. The sticky bar overlaps the bottom of the Simulation matrix card at this scroll position (card text visible beneath the bar edge).
20. gateway_stopped_lab_home 1280. Before: header "Disconnected", a Connection card with red "Not reachable or not authorised", a repeat of "Could not reach the gateway" in two places (card and tab content), a bearer-token field, then a blue banner. After: Disconnected chip, red-bordered panel with chips, one error block with a Retry button, the token field in the details panel. Defect remaining: the primary "Start a run" button is still shown while the gateway is unreachable (OBSERVED).
21. overview 390 dark. Before: six key-signal tiles in a 2-column grid on the first screen. After: only Traffic and Freshness guard tiles (one each) and the heading of "Memory and origin" on the first screen. Header is much shorter.
22. run_detail_t1_completed 820 dark. Before: at 820 the nav collapses to a Menu button and the first screen is the Connection card; result is below. After: at 820 a persistent sidebar, chips, run header with Completed + SIMULATED, "Result at a glance" labelled "virtual-time simulation on the real cache/executor; not a measurement", Cells run 1, cells.csv SHA-256 and a note "No summary statistics for matrix runs". Defect remaining: the Artifacts table's narrow columns break "run.json", "Unavailable (live file; not hashed)" mid-word at 820 (OBSERVED, both before and after; artifacts column crowded).
23. lab_evidence_list 820 light. Before: Connection card dominates; two experiment cards at the bottom, with RETAINED badges. After: chips, then "Registered experiments" with two cards at the top; each card shows Retained + provenance badge (REAL loopback one host / SIMULATED), run class, hypothesis chips (H1 Not supported, H2 Not testable ..., AIMD-CAP / STATIC-CAP "Trade-off failure") and "Open the full result". Negative results prominently displayed.
24. run_detail_sb_completed FULL 1280. Before: 1265x6746 (cropped); the "What this does and does not show" and interpretation notes are rendered as a long run of small text near the bottom (view downscaled; fine detail not legible to me). After: 1280x2989 (cropped no longer); sections Arms side by side, Paired differences, Mechanism check, Workload, "What this does and does not show", Settings used, Event log are collapsed disclosures with a one-line summary; Cause and effect shows bar charts with REAL | loopback, one host, debug build badges and "UNAVAILABLE" panels for shed fraction per phase and success-only latency. The notes and limitations are still reachable but hidden by default (13 limitations, 7 interpretation notes behind one disclosure). Whether hiding them by default is acceptable for citation use is not decided here.

## 4. Full-page height (from manifests; viewport 1280 light and 390 light)

Before widths: 1265 and 375. After widths: 1280 and 390. Heights in px.

| State | Before 1280 | After 1280 | Before 390 | After 390 |
|---|---|---|---|---|
| run_detail_sb_running | 3290 | 2608 | 5173 | 3961 |
| lab_home | 2665 | 2250 | 5208 | 3716 |
| lab_evidence_list | 1370 | 963 | 2825 | 1723 |
| evidence_m6 | 5033 | 3879 | 7676 | 5838 |
| evidence_scenario_b | 6266 | 4158 | 11106 | 7090 |
| run_form_t1_empty | 3442 | 1637 | 5174 | 2128 |
| run_form_sb_empty | 2070 | 1399 | 3900 | 1992 |
| run_form_t1_invalid | 3465 | 2873 | 5217 | 5046 |
| runs_history | 1140 | 800 | 2254 | 1193 |
| run_detail_sb_completed | 6746 | 2989 | 11905 | 5268 |
| run_detail_t1_completed | 2369 | 1336 | 4168 | 2543 |
| gateway_stopped_lab_home | 926 | 800 | 1534 | 1116 |

Notes: 800 means the page fits the viewport (full-page equals viewport height). Part of the reduction is Lab content collapsed behind disclosures (e.g. t1 form groups, SB result sections) rather than removed; some is the dropped Connection card. run_form_t1_invalid is nearly unchanged at 390 because the error state expands every group. Wrapping differs by the 15 px width change (section 1, item 2).

## 5. What did NOT improve, or got worse (OBSERVED unless marked)

1. Overview first screen shows fewer signals (2 of the previous 6 above the fold at 1280 and 390); unavailable tiles (hit ratio, p99, origin in flight) are not on the first screen. Trade-off between grouping and at-a-glance density; not judged.
2. The in-app simulation demo was removed. Its replacement explains simulation but shows nothing. Not an improvement of that feature.
3. "MEASURED" was replaced by "REAL" on gateway counters (traffic, cache, freshness, origin, overview). I did not verify that "REAL" is a defined provenance term for live counters in the gateway contract; the change makes live counters and loopback Scenario B results share a label (the latter carry "loopback, one host" qualifiers, the former do not) (observation of wording only).
4. Gateway-stopped Lab page still offers a primary "Start a run" button.
5. Artifacts table at 820 still breaks names and hash text mid-word.
6. At 390 dark the sticky "Start run" bar covers the lower part of the Simulation matrix card in the invalid-form shot.
7. Debug-build EXPLORATORY and "intelligence not in serving path" caveats were reduced from full banners to chips on every Lab page; the long text is behind "Details" (not opened). Visibility of the caveats on first screen is reduced in size although still present.
8. Long result pages hide limitations and interpretation notes by default.
9. Page-level scroll position varies in some after captures (script artifact), so some viewport pairs are weaker evidence.
10. Dark theme: only 4 dark pairs opened (18, 19, 21, 22); other dark files were not inspected.
11. No keyboard, screen-reader, contrast measurement or touch-target measurement was done in this comparison.

## 6. What these images do not show

Nothing here establishes that any simulated or unavailable capability is real: Simulation matrix runs are labelled SIMULATED in both sets, Cluster and Intelligence are "not enabled / not available" in both, and Scenario B results are loopback, one host, debug build, EXPLORATORY. The pages show UI state only. No performance claim is made.
