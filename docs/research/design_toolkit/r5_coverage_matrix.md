# R5 coverage and regression register

**Owner:** R5. **Version:** pass 1 of 3, 2026-10-30 ~09:48Z. **Status:** research; no code or test was changed. **Tags:** FACT (file:line / evidence path), INFERENCE, PROPOSAL, UNVERIFIED.
**Agent files present when written:** `00_coordination.md`, `coord/r1..r4.md`, `r2_motion_materials_diagrams_dataviz.md`. Absent: R1/R3/R4 main files, `toolkit_proposal.md`, Track 2 `03_validation_accessibility.md`. DD `phantomcache_design_direction_v2.md` present (453 lines).

## 0. How to read this register
- This is a **register of relevant combinations**, not the full cross product (6 viewports x 3 themes x ~12 workflows x 3 scenarios x ~8 states is more than 1,700 cells). Rows are chosen where the UI code, the backend, or an earlier defect makes a combination distinct.
- **Status vocabulary** (honest): `TESTED` = an automated test in `ui/test/**` (node:test, server-side render of components, no DOM events, about 300 tests) and/or an agent-driven headless-Chrome run exists for that combination. `IMPLEMENTED` = code exists, no test or observation found for this combination. `PROPOSED` = exists only as a redesign/toolkit proposal. `UNAVAILABLE` = the backend or contract does not provide it. `NOT COVERED` = no code, no test, no observation. `INDEPENDENTLY VERIFIED` = reviewed by a human or a party independent of the agent that built it: **0 rows qualify**. Every browser round was run by agents; the lead independently viewed two screenshots of round 1 (`docs/demo/browser_verification/20261030T041006Z/report.md` line 3) and nothing else. Agent walkthroughs (USAB, G, browser rounds) are not human verification.
- **Evidence keys:** BV1/BV2/BV3 = `docs/demo/browser_verification/20261030T041006Z|045605Z|055008Z/report.md` (Chrome 156.0.8078.79 headless, Windows 11, debug-build gateway, loopback). GAL = remediation gallery `docs/design/remediation/after/*.png` (210 PNG: 390/820/1280 x light/dark; manifest `after/manifest.json`). AUD = `02_ui_audit_and_plan.md` (IDs A-/B-/C-/D-). USAB = `04_validation_usability.md` (U-01..U-14; agent walkthrough, not human). G = `05_before_after_comparison.md` (observations 1-24, section 5 items). F-xx/N-xx = browser-verification findings. DD = `docs/design/phantomcache_design_direction_v2.md`. UT = `ui/test/<file>`.
- **Responsible agent** = who must make sure the combination is covered in the toolkit proposal (R1 art direction, R2 motion/materials/diagrams/dataviz, R3 engineering/testing, R4 system facts, L = lead/owner decision, B = backend/owner).
- **"Redesign changes?"** = whether DD v2 / a toolkit proposal would change this behaviour (Y/N/MAYBE). Y means the combination must be re-verified after the redesign.

## 1. Axes
| Axis | Values | Verified by project? |
|---|---|---|
| Viewport (CSS px) | 320, 390, 640, 820, 1280, 1920 | 320: layout metrics only, strict reflow on 15 routes (BV3: 74/75 pass), clips; no screenshot review of each state. 390/820/1280: full gallery light+dark (GAL) and driver matrix. 640: zoom emulation 640x400 (reflow metric only). 1920: driver matrix rounds 1-2 (14 routes x 2 themes) and F-04 icon check; none of the Lab run states. Breakpoints in CSS: 480 (app.css:293), 640 (lab.css:173), 819/820 (shell.css:62, app.css:313), 959 (lab.css:182), `pointer: coarse` (app.css:313, lab.css:182). |
| Device class | desktop (1280/1920), tablet (820), phone (390/320) | emulated viewports only; `mobile:true` 390 inconsistent (BV2 line 64 vs 33), `mobile:false` authoritative. No real device, no real touch (BV1 line 31; BV3). |
| Theme | light, dark, system (app control Light/System/Dark `theme.js`, `Shell.jsx:21`) | light+dark: gallery and matrix. System: emulated via `prefers-color-scheme` only; behaviour when OS theme changes at runtime NOT COVERED. Forced colors and `prefers-contrast: more`: emulation applied, screenshots NOT examined (BV3). |
| Browser | Chrome | Headless Chrome 156 only. Firefox, Safari/WebKit, Edge: NOT COVERED (all BV reports list it as unverified). `EventSource` + `fetch` streaming + `<dialog>.showModal` are used (browser baseline undefined; AUD section 7 item 5). |
| Input | keyboard, mouse, touch | keyboard: scripted CDP (a11y_evidence/kbd*.json); mouse: scripted clicks; touch: NOT COVERED except target sizes (BV3 N-3). Screen reader: NOT COVERED. |
| Gateway state (telemetry SSE, header chip) | loading, live, stale, disconnected, error, degraded (`connection.js:49-71`) | `reconnecting`, `unauthorised`, `unavailable` are NOT states of this reducer. |
| Lab API state | loading, ready, error{disabled, unauthorized_missing/invalid/expired, forbidden_role, forbidden_origin, host_guard, run_active, invalid_spec, busy, not_found, too_large, bad_request, server, network, unexpected} (`labApi.js:14-80`) | partly: BV1-3 JWT journeys cover missing, expired, wrong audience, operator-only; 421/403 origin NOT observed in a browser. |
| Run state | queued, running, cancelling, completed, failed, cancelled, interrupted (`labProvenance.js:46-54`; `lab_api.md:43`) | completed, running, cancelled in GAL; failed and interrupted only as fixtures (UT lab.render/lab.stream); queued NOT observed. |
| Run stream state | idle, connecting, live, stale, reconnecting, disconnected, ended, failed (`labStream.js:84-98`) | unit-tested reducer (lab.stream.test.mjs, 23 tests); stale (25 s) and heartbeat not exercised in a browser (BV1/2/3, `lab_api.md:65`). |
| Scenario / run kind | Scenario B (T2, REAL loopback), T1 simulation matrix (SIMULATED), retained M6 sealed/dry run (RETAINED SIMULATED), retained Scenario B (RETAINED REAL), tuning/smoke (`labProvenance.js:90-97`) | Scenario B and T1 via UI; M6 and retained SB from copied evidence. |
| Provenance | REAL(+qualifier), DERIVED, SIMULATED, REPLAYED, PREDICTED, UNAVAILABLE, neutral Retained | REPLAYED and PREDICTED are reserved only (`labProvenance.js:8`); no backend data exists. |
| Auth mode | anonymous dev loopback, JWT (Observer / Operator / Admin) | both exercised (BV1 to BV3), JWT only for the Lab; telemetry under JWT NOT exercised (see B13). |

## 2. Register
Columns: **ID** | **Device / viewport** | **Theme** | **Workflow** | **Scenario** | **State** | **Current implementation** | **Expected behaviour; backend data** | **Deps / owner** | **Status; evidence** | **Missing coverage / known defects** | **Redesign changes?**

### A. Launch and navigation
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A01 | Desktop 1280 | light, dark | Launch, default route | none | live | `#/` -> Overview, focus not moved on first load (`App.jsx:87-93`) | Overview tiles from schema 2; `/api/events?schema=2` | R3 | TESTED: GAL overview, BV1-3 matrix, UT shell/render | G section 5 item 1: only 2 of 6 signals above fold; U-13 no orientation sentence | Y |
| A02 | Desktop 1920 | light, dark | Launch | none | live | same; sticky rail | same | R3 | TESTED: BV1/2 matrix (14 routes x 2 themes); F-04 fixed | no Lab run states at 1920 | Y |
| A03 | Tablet 820 | light, dark | Launch, nav | none | live | sticky rail from 820 (`shell.css:62`) | rail + main | R1, R3 | TESTED: GAL 820, BV matrix | rail leaves ~588 px (DD 7.2); artifacts table breaks words at 820 (G item 5) | Y |
| A04 | Phone 390 | light, dark | Launch, Menu open/close | none | live | one-row header, Menu button, Escape returns focus (`App.jsx:106-112`) | nav panel from Menu | R1, R3 | TESTED: GAL 390, BV3 regression "Menu button" PASS | U-10 Menu open state; "Enter on Menu" weakly supported (BV3); no touch | Y |
| A05 | Phone 320 | light | Launch | none | live | same | no horizontal scroll | R3 | TESTED (metrics only): BV3 reflow | N-6: `#/` scrolls 14 px at 320 (OPEN); no screenshot review; dark at 320 NOT COVERED | MAYBE |
| A06 | Zoom 200%/400% (640x400, 320x256 emulated) | light | Launch, reflow | none | live | emulated by viewport size | WCAG 1.4.10 | R3 | TESTED: BV3 (14/15 pass at 400%) | real browser zoom NOT COVERED | Y |
| A07 | Desktop | light | Deep link cold load `#/lab/runs/<id>` and `#/lab/evidence/<id>` | Scenario B | live | `parseLabSub` regex + id charset (`routes.js:71-86`); title per route (`routes.js:96`) | direct entry works; unknown id -> not found block | R3 | TESTED: UT shell/misc; BV1 journeys | deep link while Lab off or token missing -> error block (see C03); legacy `?view=sim-demo` honoured (`routes.js:31`) | MAYBE (router/IA) |
| A08 | All | all | Unknown route | none | live | falls to Overview with notice "That page does not exist" (`App.jsx:139-141`) | same | R3 | TESTED: UT shell | `#/lab/run/<id>` returns not-found by design (`routes.js:77`) | N |
| A09 | Desktop, Phone | all | Browser back/forward and focus | none | live | hash router, `window.scrollTo(0,0)` + heading focus on navigation (`App.jsx:87-93`) | focus lands on H1 | R3 | TESTED: BV2 F-06 FIXED (scripted) | screen reader announcement NOT COVERED | Y |
| A10 | Desktop | all | Page close/restore (bfcache) | none | live | SSE closed on `pagehide`, reopened on persisted `pageshow` (`useTelemetry.js:53-63`) | no leaked sockets | R3 | IMPLEMENTED; BV3 N-2 not reproduced (21 cold loads) | cause of earlier N-2 stall unknown; multi-tab to one gateway NOT COVERED (6-connection limit) | N |

### B. Gateway configuration, connection, disconnection, recovery
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| B01 | All | all | Choose gateway address | none | n/a | only `?gateway=` or `VITE_PHANTOM_GATEWAY`, http(s) only; default `http://localhost:8085` (`config.js:4-22`); address shown in Menu/rail foot (`Shell.jsx:118-122`) and Lab Details | UI field to enter/test address | L, R1, R3 | NOT COVERED (no UI); DD 9.3 PROPOSED | U-02 (high); no runtime setter, no persistence; address hidden on phone until Menu open | Y |
| B02 | All | all | Invalid `?gateway=` | none | n/a | falls back to default and records `gateway.error` | tell the person | R3 | IMPLEMENTED; error text shown only on Overview (`Overview.jsx:90`) | silent fallback on every other page, including Lab | N |
| B03 | Desktop | light, dark | First connect | none | loading | header pill "Connecting to the gateway", pages show loading block (`ScreenFrame.jsx:72-74`) | no values until first frame | R3 | TESTED: UT connection/render | long wait with no timeout message beyond "connecting" | N |
| B04 | Desktop 1280 | light, dark | Live telemetry | live | live | header "Live" pill (`KIND_PILL`, `Shell.jsx:7-14`); badge "Derived (inferred)" on Live in Overview | steady | R3 | TESTED: GAL, UT | idle gateway shows 0 req/s (U-13) | MAYBE |
| B05 | Desktop 1280 | light, dark | Gateway stopped, no data yet | any | disconnected | pill red "Disconnected"; page StateBlock "No connection to the gateway" + Reconnect now (`ScreenFrame.jsx:74-83`) | retry auto + manual | R1, R3 | TESTED: GAL gateway_stopped_overview, UT | wording differs from Lab ("Could not reach the gateway") - see integration section 6 | Y |
| B06 | Desktop | light, dark | Gateway stops after data | any | disconnected/stale | content kept, ConnectionBanner "Disconnected (last frame Ns ago)" + Reconnect now (`States.jsx:108-125`) | last values flagged | R1, R3 | TESTED: UT, BV1 journey (kill gateway); USAB T6 | U-08 sidebar keeps "Reports mode: live" and URL while down (`Shell.jsx:118-122`); no "restart the gateway" hint | Y |
| B07 | Phone 390 | light, dark | Gateway stopped, Lab page | Lab | disconnected | Lab status pill "Not reachable or not authorised" (red) + ErrorBlock "Could not reach the gateway" + Retry; primary "Start a run" button still shown (`LabScreen.jsx:281`) | separate unreachable from unauthorised; no Start | R1, R3 | TESTED: GAL gateway_stopped_lab_home 390 | U-14 (merged causes, Start enabled); G section 5 item 4 OPEN | Y |
| B08 | All | all | Auto-reconnect | any | reconnecting | browser EventSource retries; reducer sets phase error -> `disconnected`; closed streams use 1 s..30 s backoff (`connection.js:73-76`, `useTelemetry.js:38-46`) | "Reconnecting, next try in Ns" | R1, R3 | IMPLEMENTED; no `reconnecting` kind; no `nextRetryAt` exposed | DD 9.1/9.3 asks for it; USAB T6 recovered silently in about 12 s and nothing announced it (U-08) | Y |
| B09 | All | all | Recovery announced | any | live after disconnected | live region announces `CONNECTION_TEXT[kind].title` (`App.jsx:136-138`) | announced | R3 | IMPLEMENTED; assistive tech NOT COVERED | screen-reader behaviour UNVERIFIED | MAYBE |
| B10 | All | all | Stale (no frame 3 s) | any | stale | amber "Data is stale" pill + banner (`connection.js:4,58-67`) | values flagged | R3 | TESTED: UT connection | real-network stale (throttled) NOT COVERED | N |
| B11 | All | all | Frames unreadable / contract violation | any | error / degraded | error block; degraded banner "N violation(s)" (`ScreenFrame.jsx:84-100`) | withheld values | R3 | TESTED: UT schema/contract | no browser run with a bad frame | N |
| B12 | All | all | Lab API switched off (404 non-lab body) | Lab | disabled | pill "Lab API off" + StateBlock "The Lab is switched off" (`LabScreen.jsx:286-290`) | explicit not-enabled | R1 | TESTED: UT lab.render | wording "not enabled / off / not available" differs (integration section 6) | Y |
| B13 | Desktop | all | JWT gateway: live telemetry | any | unauthorised | `useTelemetry` opens a bare `EventSource` with no credential (`useTelemetry.js:32`); gateway requires `Authorization` for `/api/events` when JWT on (`sse_auth_options.md:7`, `docs/user/security.md:28`) | INFERENCE: header shows Disconnected/loading although Lab works with a token | B (owner-reserved BD-05), L | NOT COVERED: no browser run of non-Lab routes with JWT | cannot distinguish 401 from network failure with EventSource; ESCALATION E-02 | MAYBE |
| B14 | Desktop | all | Cross-origin refused (403 bad_origin / 421 host) | Lab | forbidden_origin / host_guard | mapped to dedicated messages (`labApi.js:35-42,119-120`) | explicit cause | R3 | TESTED: UT lab.api | no browser observation; telemetry SSE would just show Disconnected | N |
| B15 | Phone/Desktop | all | Connection state across screens | any | each | five surfaces: header chip, page banner, Lab status strip, per-resource ErrorBlock, run-detail stream chip | one condition -> consistent words/colours | R1, R3 | IMPLEMENTED (inconsistent) | see integration file section 6 table | Y |
| B16 | Desktop | light | Retained evidence while gateway down | Lab | disconnected | evidence is fetched from the Lab API, so nothing loads (ResourceGate error) | DD 9.2 says say so | R1 | IMPLEMENTED | no cached evidence by design; message must say retained data is also unreachable | Y |

### C. Token entry and roles
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| C01 | Desktop | light, dark | JWT on, no token | Lab | unauthorized_missing | pill "Not reachable or not authorised"; chip "Token required"; Details open with token form (`ConnectionPanel.jsx:51-76`) | "A token is required" | R1 | TESTED: BV1/2 jwt journey, UT | pill text still merges two causes (U-14) | Y |
| C02 | Desktop, Phone | light, dark | Token entered (Observer) | Lab | ready | token in a ref only, never URL/storage; new client per change (`LabScreen.jsx:22-48`) | reads work; Start explains | R3 | TESTED: BV2 jwt, token-leak inspection PASS (BV2 line 39) | token lost on reload (by design); no session-expiry UX | N |
| C03 | Desktop | all | Deep link needing a token | Scenario B | unauthorized | ErrorBlock + "Open Details in the Lab status strip above" (`Gate.jsx:18`) | pointer valid while strip collapses | R1 | TESTED: UT lab.render | pointer text must follow any redesign of the strip (AUD C-09) | Y |
| C04 | Desktop, Phone | light | Expired token (before) | Lab | unauthorized_expired | message "The token has expired" via `x-auth-error` (`labApi.js:117`) | fresh token | R3 | TESTED: BV2 jwt | needs CORS to expose header (UNVERIFIED across origins) | N |
| C05 | Desktop | light | Token expires mid-stream | Scenario B | live -> reconnecting/failed | gateway closes stream when token expires (`lab_api.md:21`); stream re-opens, gets 401 -> `failed` (terminal) | clear "token expired" on run page | R3, R1 | NOT COVERED (no browser run); reducer tested | BV1/2 list "token expiry mid-stream" as unverified | MAYBE |
| C06 | Desktop, Phone | light | Operator-only token before submit | Scenario B | operator | hint "This token cannot start runs..." for observer-only; operator warning silent pre-submit fixed? BV3 N-5 PASS | warn pre-submit | R1 | TESTED: BV3 N-5 | roles read from the token for display only, not verified (`labToken.js`) | N |
| C07 | Desktop 1280, Phone 390 | light | Operator token after submit | Scenario B | forbidden_role on GET | `ReadForbiddenBlock` shows run id and "probably accepted; not confirmed" (`Gate.jsx:50-63`) | no dead end | R1 | TESTED: BV3 N-5 (1280 and 390) | cannot show run progress: by backend design (Operator cannot read) | Y |
| C08 | Desktop | light | Observer cannot start | Scenario B | observer | Start disabled with reason; 403 on POST message | clear | R3 | TESTED: BV1 (403-on-POST path unexercised then), BV2 | F-09 by design | N |
| C09 | Desktop | light | Admin | both | admin | full flow incl. authenticated artifact download | works | R3 | TESTED: BV2 | none | N |

### D. Experiment configuration and validation
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| D01 | Desktop 1280 | light, dark | Open Start form | Scenario B | initial | two choice cards (REAL / SIMULATED), presets "Quick check"/"Smallest run" (`RunForm.jsx:116-123`); limits from capabilities | valid defaults | R1 | TESTED: GAL run_form_sb_empty | U-04: no preset selected state, no duration estimate, heavy default (10 reps x 3 arms) | Y |
| D02 | Desktop 1280 | light | Open Start form | T1 matrix | initial, empty | groups "none selected, N available", sticky bar "Start run - 4 things still needed" | no red on pristine form | R1 | TESTED: GAL run_form_t1_empty | AUD C-01/C-02 fixed in I2 per G obs 2 | Y |
| D03 | Phone 390 | dark | Invalid form after Start press | T1 | invalid | summary "5 things need attention" + sticky bar overlaps card (G obs 19) | errors linked | R1, R3 | TESTED: GAL run_form_t1_invalid 390 dark | G item 6/U-10 sticky bar overlap OPEN | Y |
| D04 | Desktop | all | Validation parity with backend | both | 422 | form mirrors limits; server authoritative (`labApi.js:47-50`, `labForms.js`) | server message shown | R3 | TESTED: UT lab.forms (14) | limits drift between UI and backend UNVERIFIED | N |
| D05 | All | all | Sealed seed set | T1 | refused | seed sets T/D only offered; typing S/sealed -> local error (`labForms.js:8,77-79`); API refuses 422 | sealed seeds unreachable | R3 | TESTED: UT lab.forms, lab.guard | PRESERVE (see section 3) | N |
| D06 | Desktop | light | Advanced settings | Scenario B | collapsed groups | fold groups with counts | defaults filled by gateway echoed on run page | R1 | TESTED: UT, GAL | unit/range hints only in hint text (AUD C-04) | Y |
| D07 | Desktop | all | Debug-build warning near Start | Scenario B | debug gateway | chip "Debug build: results EXPLORATORY" in strip; detail in Details (`ConnectionPanel.jsx:88,117`) | warning adjacent to Start | R1 | TESTED: UT, BV3 | U-04 phone: warning below sticky Start; G item 7: caveat reduced to chip | Y |
| D08 | Desktop | all | Run kinds unavailable | both | not available | AvailabilityRow shows reason (`ConnectionPanel.jsx:10-19`) | form disabled with reason | R3 | TESTED: UT | no screenshot of `available:false` | N |
| D09 | Phone 390/320 | light | Form inputs touch size | Scenario B | form | radios/checkboxes 24x24, 2 text inputs 36 px (BV3 N-3) | 44 px target | R3 | TESTED (metrics): BV3 | WCAG 2.5.8 AA passes at 24, AAA 44 not met | Y |
| D10 | Desktop | all | Form while capabilities loading | both | loading | ResourceGate "Loading the run limits" | no form until limits known | R3 | TESTED: UT | none | N |
| D11 | Desktop | all | Copy equivalent request / CLI | both | valid | "Exact request" panel + copy with polite status (AUD C-13) | shown | R3 | TESTED | token must not appear in it (UT forbidden) | N |
| D12 | Desktop | all | Run already active (409) | both | run_active | mapped message "A run is already active"; strip link "A run is active: open it" | one-run limit | R1 | TESTED: UT api | no browser observation | N |

### E. Execution, progress, cancel, failure, interruption
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| E01 | Desktop 1280 | light, dark | Start accepted -> run page | Scenario B | queued/running | POST then hash to `#/lab/runs/<id>` (`LabScreen.jsx:121-137`) | immediate feedback | R1 | TESTED: GAL run_detail_sb_running, BV1/2 | `queued` state never observed (state exists in `RUN_STATES`) | Y |
| E02 | Desktop 1280 | light, dark | Running, progress | Scenario B | running | native `<progress>` "0 of 2 units", arm/rep/phase, elapsed DERIVED from timestamps (`RunsViews.jsx:181-204`) | determinate progress | R1, R2, B | TESTED: GAL | U-03 (unit undefined, bar empty 45 s); ETA/phase timing needs backend (AUD section 7) | Y |
| E03 | Phone 390 | dark | Running, progress | Scenario B | running | sticky head chips + MiniProgress | same | R1 | TESTED: GAL run_detail_sb_running 390 | not reviewed at 320 | Y |
| E04 | Desktop 1280 | light | Running, T1 | T1 | running | progress events throttled ~200 ms (`lab_api.md:47`) | live progress | R1 | NOT COVERED live in browser (BV2/3 "live T1 progress" unverified); UT stream fixture events_t1.sse | T1 cancellation at next cell boundary | Y |
| E05 | Desktop, Phone | all | Run stream live/stale/reconnecting/disconnected/failed | any | stream | chip normally; banner only for 4 problem states or event gaps (`RunsViews.jsx:78-118`) | non-live state declared; "last known" suffix on chip | R1, R2 | TESTED: UT lab.stream/render (reducer, SSR) | no browser run of stale (25 s) / backoff / failed; AUD C-07 | Y |
| E06 | Desktop 1280, Phone 390 | light, dark | Cancel confirm | Scenario B | confirm | native `<dialog>.showModal`, focus on "Keep running", Escape closes unless sending (`RunsViews.jsx:155-179`) | safe default | R1, R3 | TESTED: GAL cancel_dialog (6 files), BV3 cancel script | `dialog` support in Firefox/Safari NOT COVERED; focus return tested in Chrome only | Y |
| E07 | Desktop | light | Cancel requested (`cancelling`) | Scenario B | cancelling | Notice "Cancel requested", button removed, focus to heading (`RunsViews.jsx:349,387-389`) | cooperative stop | R1 | TESTED: UT, BV | cancel on a run that ends `completed` (race, `lab_api.md:65`) shows no explanation | Y |
| E08 | Desktop 1280, Phone 390 | light, dark | Cancelled run result | Scenario B | cancelled | Notice "Partial result ... no verdict can be drawn" yet verdict chips still rendered (`RunsViews.jsx:288-293`, `ScenarioBResult.jsx:123-124`) | no verdict for a cancelled run; backend summary: `status: CANCELLED`, verdicts present (MECHANISM_FAILED when no valid runs, `scenario_b.rs:659,700,714`) | B + R1 + R3 | IMPLEMENTED; contradicts own notice. Tests use fixture `verdicts = {}` (`lab.view.test.mjs:282-290`) | **U-01 OPEN (high)**; E-01 escalation; fixture does not match backend | Y |
| E09 | Desktop | all | Failed run | Scenario B | failed | red alert "The run failed (code)" + message; "No result summary for this run" (`RunsViews.jsx:381-383,299-305`) | no verdict | R1 | TESTED: UT (fixture) | not reproduced in a browser; partial `summary.json` may or may not exist (backend: write only on runner completion UNVERIFIED) | Y |
| E10 | Desktop | all | Interrupted by restart | any | interrupted | Notice "Interrupted by a gateway restart" (`RunsViews.jsx:376-380`); tone warn, glyph disconnected | no result | R1 | TESTED: UT stream/render fixture | no browser run (requires gateway restart mid-run); `gateway_reap` outcome not shown | Y |
| E11 | Phone | all | Persistence errors | any | terminal | Notice "Persistence errors" (`RunsViews.jsx:392`) | history may be incomplete | R1 | IMPLEMENTED; UT? UNVERIFIED | never observed | N |
| E12 | Desktop | light | Leave run page while running and return | Scenario B | running | stream restarts via key remount; `Last-Event-ID` resume (`useRunStream.js:34-43`) | log replay | R3 | TESTED: UT lab.stream | browser not covered | N |
| E13 | Desktop | light | Run reaches terminal -> record re-read | Scenario B | completed | `done` event triggers `reloadRun` (`LabScreen.jsx:181-184`) | exit code, artifact index | R3 | TESTED: BV | race between `done` and artifacts listing UNVERIFIED | N |
| E14 | Desktop | light | Run page: 404 unknown id | any | not_found | ErrorBlock "Not found" | clear | R3 | TESTED: UT api | none | N |
| E15 | Phone 320 | light | Running page | Scenario B | running | BV3 reflow pass (320) | no overflow | R3 | TESTED (metrics) | no visual review; not dark | MAYBE |
| E16 | Desktop | light | Slow / offline network during run | any | reconnecting | `reconnecting` for attempts 1-2, `disconnected` from 3 (`labStream.js:93`) | words and countdown | R1 | TESTED: UT reducer | no throttled-network browser run | Y |

### F. Results, comparisons, charts, tables, interpretation
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F01 | Desktop 1280 | light, dark | Completed Scenario B result (run) | Scenario B | completed | "Result at a glance" verdict chips per arm (DERIVED), run class chip, charts with Chart/Table toggle (`ScenarioBResult.jsx:115-216`) | answer first | R1, R2 | TESTED: GAL run_detail_sb_completed | U-05 no head-to-head line; U-11 three status words; binding notice not on run page (see F09) | Y |
| F02 | Phone 390 | light, dark | Completed result | Scenario B | completed | folded sections, tables scroll inside `.table-wrap` | no page-level scroll | R3 | TESTED: GAL 390, BV3 N-1a PASS | 5,268 px tall (G section 4) | Y |
| F03 | Desktop | light | Charts: tradeoff/shed/latency | Scenario B | with data | `LabCharts.jsx` HTML/SVG, marks `aria-hidden`, text carries values | plot received values only | R2, R3 | TESTED: UT | AUD D-04/D-05 (axis caption, single colour); N-4 fixed | Y |
| F04 | Desktop | light | Chart: unavailable parts | Scenario B | partial data | `UnavailablePanel` (shed per phase, success-only latency) | never zero | R2 | TESTED: GAL | none | Y |
| F05 | Desktop | all | Paired differences table | Scenario B | completed | BCa 95% CI, permutation p (DERIVED) in closed fold | citation-safe | R1 | TESTED: UT | hidden by default (G item 8) | Y |
| F06 | Desktop | all | T1 result | T1 | completed | "Cells run" + SHA of cells.csv + note "No summary statistics for matrix runs" (`RunsViews.jsx:262-276`) | backend gives cell count + hash only | B | TESTED: GAL run_detail_t1_completed 820 dark | no results for T1 in UI beyond download; by backend design | Y |
| F07 | Desktop | all | T1/SB result absent after terminal | both | no summary | StateBlock "No result summary for this run" | honest | R1 | TESTED: UT | n/a | N |
| F08 | Phone/Desktop | light | Result while summary loads / fails | Scenario B | loading/error | "Loading the result summary", ErrorBlock (`RunsViews.jsx:297-298`) | no stale result | R3 | TESTED: UT | none | N |
| F09 | Desktop 1280 | light, dark | Binding interpretation notice | Scenario B | retained | rendered mini-markdown in evidence page (D-03 fixed), "Rendered / Raw / Copy" (G obs 7) | verbatim, citable | R1 | TESTED: UT lab.markdown (11), GAL | PRESERVE; run page shows `label_meaning` and limitations from summary, not the retained notice file | Y |
| F10 | Desktop | all | Verdict weight parity | both | negative/positive | every outcome same size via one component (`labProvenance.js:61-64`) | equal prominence | R1, R2 | TESTED: UT, BV1 | PRESERVE; any new colour/motion must not favour positive | Y |
| F11 | Desktop | all | "Run class: VALID" | Scenario B | retained | blue chip + narrow-meaning text | not read as success | R1 | TESTED | U-07 (high-risk misreading) | Y |
| F12 | Desktop | all | Unknown tier / verdict / state strings | any | unexpected | fall back to UNAVAILABLE / neutral (`labProvenance.js:33-36,56-59,77-81`) | never invent | R3 | TESTED: UT provenance | none | N |
| F13 | Desktop | all | Elapsed time | any | running/terminal | DERIVED from timestamps or UNAVAILABLE+reason (`RunsViews.jsx:120-137`) | labelled | R2 | TESTED | server-provided elapsed UNAVAILABLE (AUD section 7) | Y |
| F14 | Desktop 1920 | light | Result pages at 1920 | Scenario B | completed | not in the matrix (routes with ids only at 1280/820/390 in GAL) | readable measure | R1, R3 | NOT COVERED | line length at 1920, chart width | Y |

### G. Evidence, provenance, retention, artifacts
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| G01 | Desktop 820 | light | Evidence list | retained | ready | two cards, Retained tag + provenance badge, hypothesis chips (G obs 23) | `GET /evidence` | R1 | TESTED: GAL | no pagination/filter; empty list not rendered in a browser | Y |
| G02 | Desktop | all | Evidence list empty | retained | empty | UT fixture only | explicit empty state with next step | R1 | IMPLEMENTED (UT?) UNVERIFIED | GAL has no empty-evidence state | Y |
| G03 | Desktop 1280 | light, dark | M6 sealed evaluation | retained SIMULATED | ready | SIMULATED+Retained badges, banner "NOT A MEASUREMENT", verdict list (G obs 6) | H1 not supported shown plainly | R1, R2 | TESTED: GAL evidence_m6 (all 6 combos) | U-06 jargon (H1..H6, BCa, F-a) | Y |
| G04 | Phone 390 | dark | M6 / SB evidence | retained | ready | table scroll in-box | no overflow | R3 | TESTED: BV3 | 7,090 px tall | Y |
| G05 | Desktop | all | Retained Scenario B | retained REAL | ready | "REAL (loopback, one host)" + Retained tag, blue notice | qualifier mandatory | R1 | TESTED: GAL | REAL without qualifier used on live counters (G item 3 of section 5) | Y |
| G06 | Desktop | all | Artifacts of a live run | both | terminal | table with name, size, SHA-256 (covers), provenance, Download (`RunsViews.jsx:229-260`) | allowlisted names | R3 | TESTED: GAL, BV2 download PASS | 820 column breaks words (G item 5) | Y |
| G07 | Desktop | all | Artifact download | both | ready/error | fetch with Authorization -> blob -> anchor (`LabScreen.jsx:158-223`) | no token in URL | R3 | TESTED: BV2 admin download; UT api | blocked-download / large (64 MiB) error `too_large` not observed | N |
| G08 | Desktop | all | Retained evidence files download | retained | n/a | "listed by name and size; not downloadable here" (`EvidenceViews.jsx:34`) | UNAVAILABLE in API (AUD section 7) | B | UNAVAILABLE | state it plainly, no dead button | N |
| G09 | Desktop | all | Sealed seed values | retained | n/a | evidence payload scrubbed of seeds; UI offers no sealed set | unreachable | R3 | TESTED: UT lab.guard, forbidden | PRESERVE | N |
| G10 | Desktop | light | Runs list | both | many runs | table, newest first, "shown N of M indexed (index covers newest K)" (`RunsViews.jsx:35-44`); client supports limit/offset but page never passes them (`LabScreen.jsx:147`) | pagination UI | B, R1 | IMPLEMENTED (no pagination) | no filter/sort; 200-run index bound (`lab_api.md:55`) | Y |
| G11 | Desktop | light | Runs list empty | any | empty | StateBlock "No runs yet", not a failure | explicit | R1 | TESTED: UT, GAL | none | Y |
| G12 | Desktop | all | Provenance vocabulary | all | all | MEASURED shown as REAL; inferred as "Derived (inferred)"; Retained neutral tag (`provenance.js:3-13`, `labProvenance.js:3-10`) | six words | R1, R2 | TESTED: UT provenance (13) | REPLAYED/PREDICTED reserved with no data; "REAL" on live counters has no qualifier while Lab REAL has "loopback, one host" | Y |

### H. Simulation page
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| H01 | Desktop 1280 | light, dark | Simulation explainer | simulation | n/a | static explainer, SIMULATED banner and badges, no numbers; legacy demo retired (`Simulation.jsx:1-60`, G obs 16) | stays honest | R1, R2 | TESTED: GAL, UT sim | owner asked about "explicitly simulated behaviour": none left to animate; R2/R1 proposals must not reintroduce a fake live dashboard | Y |
| H02 | Phone 390 | all | Simulation controls | simulation | n/a | BV3 N-3 recorded buttons at 21 px (RUN, PAUSE...) on `#/simulation` BEFORE retirement? BV3 predates I3 (UNVERIFIED which build) | no controls remain | R3 | NOT COVERED after retirement | re-run N-3 on current page | N |
| H03 | All | all | Gateway reports simulation mode | any | mode=simulation | banner on non-Simulation pages (`App.jsx:100,142-146`); `simulated_in_live` rejected by `readField` (`provenance.js:37`) | never presented as live | R3 | TESTED: UT provenance/viewmodel | "mode" word has 3 meanings (see integration section 6) | Y |

### I. Cluster and Intelligence
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| I01 | Desktop 1280 | light, dark | Cluster page | n/a | not enabled / no contract | StateBlock + "What this page would show ... Nothing below is a value" (`Placeholder.jsx:56-81`) | honest not available; backend node list has id/state only | R1, R2 | TESTED: GAL | U-09 mixed wording ("Not available" vs "not enabled"); repo path in product copy (`Placeholder.jsx:78`) | Y |
| I02 | Desktop | light | Intelligence page | n/a | not enabled | same pattern; "controller: none" | no PIE data (PIE is a legacy stub, CLAUDE.md 6b) | R1, R2 | TESTED: GAL | R2 proposes only illustrative loop diagrams; must carry label and not suggest a working controller | Y |
| I03 | Phone 390 | dark | Cluster / Intelligence | n/a | placeholder | same | same | R1 | TESTED: GAL | none | Y |
| I04 | All | all | Cluster with node list | n/a | no_contract | NodeList table with per-node provenance (`Placeholder.jsx:32-54`) | only reported fields | R4 | TESTED: UT viewmodel | CPU/memory UNAVAILABLE per R4 finding 4 | N |

### J. Live data pages (Overview, Traffic, Cache, Freshness, Origin)
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J01 | Desktop 1280 | light, dark | Overview | live | live | sections Traffic / Freshness guard / Memory and origin with Details links | schema 2 | R1 | TESTED: GAL | G item 1 (fewer tiles above fold) | Y |
| J02 | Desktop | all | Traffic & latency | live | live | percentiles often UNAVAILABLE ("p50..p99 not reported yet"), Chart/Table toggle | never zero | R2 | TESTED: GAL | hot_keys unavailable (`not_implemented`, R4 finding 2) | Y |
| J03 | Desktop | all | Cache & memory | live | live | 0..256 MiB axis labelled | measured bytes | R2 | TESTED: GAL | none | Y |
| J04 | Desktop | all | Freshness | live | live | stale cap violations tile; `PHANTOM_DEFAULT_TTL_MS` NOT WIRED (CLAUDE.md 5) | do not show configured TTL as effective | R4 | TESTED: GAL | any new freshness diagram (R2) must not draw 60 s as configurable | Y |
| J05 | Desktop | all | Origin | live | live | origin_cap counters REAL; origin latency UNAVAILABLE | no zeros | R2 | TESTED: GAL | none | Y |
| J06 | All | all | Each live page while stale | live | stale | banner + content flagged (`ScreenFrame.jsx:90-100`) | not trustworthy as current | R1 | TESTED: UT | no screenshot of stale state in GAL (GAL has stopped, not stale) | Y |
| J07 | All | all | Each live page while degraded | live | degraded | banner + violations withheld | explicit | R1 | TESTED: UT | no screenshot | Y |
| J08 | Phone 390 | dark | Overview | live | live | two-column grid | no overflow | R3 | TESTED: GAL, BV3 | 320 overflow N-6 OPEN | Y |
| J09 | Desktop | all | Idle gateway (0 req/s) | live | idle | tiles show 0.0 | explain zero vs unavailable | R1 | TESTED | U-13 | Y |
| J10 | Desktop | all | Telemetry under real traffic | live | busy | NOT observed (BV1/2/3 "0 req/s") | charts/sparklines respond | R2, R3 | NOT COVERED | sparkline history (`useHistory`) never exercised with moving data in a browser | Y |

### K. Theme, input modality, accessibility
| ID | Device/vp | Theme | Workflow | Scenario | State | Current implementation | Expected; backend data | Deps/owner | Status; evidence | Gaps/defects | Redesign? |
|---|---|---|---|---|---|---|---|---|---|---|---|
| K01 | All | light/dark/system | Theme switch persists | n/a | n/a | Light/System/Dark control; stored in localStorage with try/catch (`theme.js:36-41`) | persists; system follows OS | R3 | TESTED: UT tokens/shell | OS change at runtime NOT COVERED; private-window storage failure TESTED? UNVERIFIED | Y |
| K02 | All | dark | Dark parity | all | all | same text length and focusable count (BV2) | parity | R3 | TESTED: BV1/2 matrix | only 4 dark pairs reviewed by G; dark contrast 0 failures (min 4.98:1, solid backgrounds only) | Y |
| K03 | All | light/dark | Contrast | all | all | `check-contrast.mjs` 6,342 checks, 0 fail | AA | R3 | TESTED | non-text and focus-ring contrast not measured; glass/transparency (DD) would invalidate solid-background assumption | Y |
| K04 | Desktop | all | Keyboard traversal | all | all | every Tab stop has 2 px outline; skip link first; arrow roving nav | operable | R3 | TESTED: scripted | arrow-key nav test partly harness artifact (BV2 line 9) | Y |
| K05 | Phone | all | Touch | all | all | `pointer: coarse` rules (`app.css:313`, `lab.css:182`) | 44 px | R3 | NOT COVERED on a device | target sizes measured only | Y |
| K06 | All | all | Reduced motion | all | all | animations collapsed (`app.css:277`, `shell.css:86`) | no motion | R2, R3 | TESTED: BV (0 animations) | any new motion (R2) must keep this and carry identical text | Y |
| K07 | All | all | Forced colours / high contrast | all | all | `forced-colors` and `prefers-contrast: more` rules exist | readable | R3 | IMPLEMENTED; screenshots not examined (BV3) | NOT COVERED visually | Y |
| K08 | All | all | Screen reader | all | all | landmarks, live regions (`App.jsx:136`), run status region (`RunsViews.jsx:374`) | announcements | R3 | NOT COVERED (no AT used, BV1 line 31) | `aria-live` behaviours UNVERIFIED | Y |
| K09 | All | all | Non-Chrome browsers | all | all | n/a | works | R3 | NOT COVERED | `<dialog>`, `fetch` streaming, `EventSource` | Y |

## 3. Preserve list (must not regress; each is FACT-backed and has an owner)
| # | Item | Evidence | Check |
|---|---|---|---|
| P1 | Routes and deep links: `#/<screen>`, `#/lab`, `#/lab/run`, `#/lab/runs[/id]`, `#/lab/evidence[/id]`, legacy `?view=sim-demo`; unknown route falls back with notice; titles per route | `routes.js:5-106`, `App.jsx:139` | UT shell + browser deep-link test (A07) |
| P2 | Heading focus on navigation, skip link (first Tab), Escape closes Menu and returns focus | `App.jsx:87-122` | BV2 F-06 recheck |
| P3 | Run lifecycle: start -> run page, stream with `Last-Event-ID` resume, terminal states never reconnect, `done` re-reads the record, cancel is confirmed in a modal dialog with safe default | `LabScreen.jsx:121-224`, `useRunStream.js`, `RunsViews.jsx:155-179` | E01..E16 |
| P4 | Provenance words and rules: REAL always with basis qualifier (and build) in Lab, Retained is a neutral tag not part of the word, UNAVAILABLE is never zero, unknown tiers fall to UNAVAILABLE, simulated never accepted in a live payload | `labProvenance.js`, `provenance.js:37` | UT provenance |
| P5 | Binding Scenario B interpretation text verbatim (notice and `label_meaning`, limitations, interpretation notes never inside a closed fold per the file comment; but G item 8 notes limitations are in one disclosure by default: owner to confirm which holds) | `ScenarioBResult.jsx:6-9,197-216`, `docs/demo/scenario_b_interpretation_notice.md` | UT lab.markdown, lab.hierarchy |
| P6 | Honesty banners: debug build EXPLORATORY, "intelligence not in serving path", SIMULATED banner on Simulation and on simulated gateways, "Nothing below is a value" on placeholders | `ConnectionPanel.jsx:88-129`, `App.jsx:142`, `Placeholder.jsx` | UT hierarchy/render |
| P7 | Sealed-seed unreachability in the UI (T/D only; S/sealed refused locally and by API; no seed values rendered) | `labForms.js:8,77-79`, `lab_api.md:27` | UT lab.forms/guard |
| P8 | Token handling: memory only (ref), Authorization header only, never in URL/storage/log/copy text; stream via fetch not EventSource | `LabScreen.jsx:22-48`, `labApi.js:3-5` | forbidden.test + BV token-leak inspection |
| P9 | Verdicts of equal size/weight regardless of sign; negative results not de-emphasised | `labProvenance.js:61-64`, BV1 | UT + visual |
| P10 | Every chart has a table alternative and text values; marks `aria-hidden` | `ChartFrame.jsx`, `LabCharts.jsx` | UT a11y |
| P11 | Honest unavailable: "Unavailable" + reason, never 0 | `viewmodel.js`, `States.jsx` | UT |
| P12 | 44 px targets under 960 px / coarse, 24 px floor on desktop; no horizontal page scroll 320..1280 (except open N-6) | `app.css:313`, BV3 | R3 layout script |
| P13 | Reduced motion and reduced transparency respected | `app.css:277`, `tokens.css:155` | BV |
| P14 | Tests/tools: `ui/test` (about 300 tests, node:test), `check-a11y/contrast/forbidden`, render harness `ui/scripts/harness-entry.jsx` imports `ConnectionPanel.jsx` by name (`ConnectionPanel.jsx:5`) | file comments | `npm test` equivalent |
| P15 | Footer/purpose sentences true per page kind (data / lab / sim / placeholder) | `routes.js:108-119` | UT shell |

## 4. Count of rows by primary status (pass 1, mechanical count)
Counted with awk on the Status column (first token) of the 115 register rows in section 2. The label TESTED is generous: it means a unit/SSR test and/or an agent-driven Chrome run exists, NOT human verification; many TESTED rows rest only on fixture rendering (no DOM events) or one agent screenshot.

| Status | Rows | Row IDs (non-TESTED) |
|---|---|---|
| TESTED | 93 | all others |
| IMPLEMENTED (no test or observation of this combination) | 11 | A10, B02, B08, B09, B15, B16, E08, E11, G02, G10, K07 |
| PROPOSED (primary) | 0 | B01 and B08 have a DD proposal but no code; counted as NOT COVERED / IMPLEMENTED |
| UNAVAILABLE | 1 | G08 |
| NOT COVERED | 10 | B01, B13, C05, E04, F14, H02, J10, K05, K08, K09 |
| INDEPENDENTLY VERIFIED | 0 | none |
| Total | 115 | |

Defect-bearing rows (a known open defect or contradiction regardless of status): E08 (U-01), B01 (U-02), B07 (U-14), B15, B13, D03, D07, A05 (N-6), F11 (U-07), G10, J01.

## 5. Open items to reconcile in pass 2
1. Re-run the awk recount of section 4 after rows are added.
2. Add rows for each new state/screen introduced by R1 (art direction), R2 (animated states), R3 (new tests) and R4 (new objects) once their main files exist.
3. Confirm with R4 the facts behind F06, E09, E10 (what `summary.json` and events exist for failed/interrupted Scenario B runs).
