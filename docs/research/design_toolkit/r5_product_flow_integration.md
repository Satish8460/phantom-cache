# R5: product-wide integration assessment (systems integration and product-flow supervision)

**Owner:** R5. **Last updated:** 2026-10-30 ~09:48Z (review pass 3a of 3: passes 1, 2 and the R4 matrices review done; pass 3b waits for the R1 main file, `toolkit_proposal.md` and Track 2).
**Agent files that existed at this update:** `00_coordination.md`; `coord/r1.md`, `coord/r2.md`, `coord/r3.md`, `coord/r4.md`; `r2_motion_materials_diagrams_dataviz.md` (read sections 0, 1, 10 to 12), `r3_engineering_a11y_testing.md` (read sections 6 to 8, grep elsewhere), `r4_system_objects_and_motion.md` (read sections 2 L4 to L7, 4.2, 6, 7 to 9; 26 items spot-checked in total, see 8.4); `r1_design_intelligence_and_critique.md` (read sections 0, 6.2, 7, 10). **Appeared after the first update:** `r4_lifecycle_and_event_matrices.md` (40 KB; read M0.1 to M0.4, grep elsewhere). **Not yet present:** `r1_design_intelligence_and_critique.md`, `toolkit_proposal.md`, Track 2 `03_validation_accessibility.md`. Present as input: `docs/design/phantomcache_design_direction_v2.md` (DD, 453 lines; sections 7, 9, 13 read), `docs/design/remediation/a11y_evidence/` (matrix at 320/390/640/820/1280 JSON; not analysed).
**Scope:** research only. No code, test, dependency, skill or git change. Companion: `r5_coverage_matrix.md` (115 rows, IDs cited below as A01..K09) and `coord/r5.md` (requests, replies, conflict log).
**Tags:** FACT (file:line or evidence path), INFERENCE, PROPOSAL, UNVERIFIED. "Agent-observed" means a headless-Chrome run or screenshot read by an agent; there is no human verification of any UI state in the repository.

---

## 1. Executive assessment
1. The product is coherent at the level of honesty rules (provenance, unavailable-not-zero, placeholders) and weakest at **state representation across surfaces** and at **incomplete-run presentation**. The recommendations of R1 to R4 cannot be considered complete until they address the ten gaps in section 8 and the six open conflicts in section 9.
2. **The most serious product defect is U-01, and it is wrong to treat it as UI-only.** A cancelled Scenario B run shows a verdict (for example "Mechanism failed") because the backend writes one: `summarize()` always computes verdicts and `mech_pass = !rs.is_empty() && violating.is_empty()` (`src/lab_live/scenario_b.rs:659,700`) yields a failing mechanism verdict when no repetition finished; status is `"CANCELLED"` (`:714`). The UI renders the verdicts under its own "no verdict can be drawn" notice (`ui/src/screens/lab/ScenarioBResult.jsx:123-124`, `RunsViews.jsx:288-293`). The UI tests pass because the fixture is hand-edited to `verdicts = {}` (`ui/test/lab.view.test.mjs:282-290`, `lab.render.test.mjs:613-626`), not real backend output. FACT. Status: OPEN. Needs a lead/owner decision (section 12, E-01).
3. **Gateway connection is modelled five different ways** (section 6) and two states that the DD direction relies on, `Recovering` and `Unauthorised`, are not distinguishable by the telemetry code (`ui/src/lib/connection.js:49-71`). Under JWT the telemetry stream cannot authenticate from a browser (EventSource has no header; `docs/design/sse_auth_options.md:7`; `ui/src/hooks/useTelemetry.js:32`), so on a protected gateway the header chip is expected to read Disconnected while the Lab works with a pasted token (INFERENCE, never observed; matrix B13). This is owner-reserved (BD-05).
4. Verification is Chrome 156 headless only, agent-driven, one machine, debug-build gateway. There are **0 independently verified rows** of 115. Verified viewport range is 320 to 1920 for layout metrics but screenshots exist only for 390/820/1280 (and 1920 for 14 routes in rounds 1 and 2).
5. Any redesign (DD v2: header connection popover, rail, glass, new status hierarchy) touches P1 to P15 (section 10) and invalidates assumptions of the contrast checker (solid backgrounds only).

## 2. Product map and navigation (FACT: `ui/src/lib/routes.js:5-106`)
```
App (hash router; focus -> H1 on navigation; live region = telemetry connection title)
 Header: brand | connection chip (telemetry SSE) | theme (>=820) | Menu (<820)
 Nav groups: Live gateway {Overview, Traffic & latency, Cache & memory, Freshness, Origin}
             Experiments {Phantom Lab}   Explainer {Simulation [Simulated]}   Not available yet {Cluster, Intelligence}
 Data pages  -> ScreenFrame gate on telemetry (loading | disconnected | error | ready+banner)
 Lab         -> LabProvider (client per gateway+token) -> status strip -> tabs About | Start a run | Runs | Evidence
                 #/lab  #/lab/run  #/lab/runs[/<id>]  #/lab/evidence[/<id>]
 Simulation  -> static explainer, no telemetry dependency except the simulated-gateway banner
 Cluster/Intelligence -> Placeholder (honest "not available"; node table if nodes[] present)
```
Two independent data planes: **telemetry SSE** (`/api/events?schema=2`, 100 ms frames `src/api/server.rs:959`, rates sampled every 1 s `:462`, client history 120 points at >= 500 ms gaps `ui/src/lib/history.js:5-6`) for the five Live pages; **Lab API** (`/api/v1/lab/*`, `lab-1` documents, fetch streaming for run events) for everything under Phantom Lab. They have separate failure states, separate auth handling and separate recovery paths. This split is the root of the inconsistencies in section 6.

## 3. Journey inventory (every important journey; text diagrams)
Legend: `[S]` screen, `->` transition, `{x}` precondition, `!` failure branch, `(M:A01)` matrix row.

**J1 Launch and navigation (A01..A10).** `{gateway reachable?}` Load `#/` -> [Overview] (loading -> live) -> choose route via rail/Menu -> focus H1. `! unknown hash` -> Overview + "That page does not exist". `! bad ?gateway=` -> silent default (only Overview shows the error). Legacy `?view=sim-demo` -> Simulation.

**J2 Gateway configuration and connection (B01..B16).** `{none}` Gateway chosen by `?gateway=` / env / default `localhost:8085` -> no UI to change (gap G-01). Connect: loading -> live | disconnected | error. Disconnected with no data -> StateBlock + Reconnect now; with data -> banner + stale content. Recovery is automatic (browser retry / 1 s..30 s backoff) and silent (U-08). `! JWT` -> telemetry Disconnected regardless of token (B13, INFERENCE).

**J3 Token entry (C01..C09).** `{Lab API enabled, JWT on}` [Lab] pill "Not reachable or not authorised" + chip "Token required" -> Details opens -> paste token (memory only) -> new client -> resources reload. Roles are read from the token for display only. Branches: expired / invalid / wrong role / operator-only (can start, cannot read: `ReadForbiddenBlock`, `Gate.jsx:50-63`). Session expiry mid-stream: stream closed by gateway, UI ends in `failed` (C05, not observed).

**J4 Experiment configuration (D01..D12).** [Lab] -> Start a run (head button and tab: U-12) -> [Start form] choose kind (Scenario B REAL / T1 SIMULATED) -> preset (fills only) -> groups -> validate (local mirror of limits; server authoritative; sealed seed refused locally) -> Start. `! 409` run active, `! 422` invalid spec, `! 403` role, `! network`.

**J5 Execution (E01..E16).**
```
Start -> POST /runs -> hash #/lab/runs/<id>
  [Run detail] state: queued -> running -> (cancelling) -> completed | failed | cancelled | interrupted
  Stream phases: connecting -> live <-> stale(25 s silent) -> reconnecting(<3 tries) -> disconnected(>=3) | failed(terminal HTTP error) | ended(done)
  Cancel: button -> modal dialog (focus on Keep running) -> POST cancel (202/200) -> "Cancel requested" -> cancelled (or completed: race, lab_api.md:65)
  Interrupted: gateway restart rewrites queued/running/cancelling -> interrupted (+ events, source server_recovery_on_startup)
  Terminal -> reload record -> artifacts list -> (Scenario B) fetch summary.json
```
FACT sources: `labProvenance.js:46-54`, `labStream.js:84-98`, `RunsViews.jsx:155-179,349-392`, `LabScreen.jsx:170-253`, `lab_api.md:43-55`, `src/api/lab_api.rs:604-720` (state_rank; `request_cancel` accepts queued or running -> cancelling).

**J6 Results and interpretation (F01..F14).** terminal -> [Result at a glance: verdict chips (DERIVED) + run class] -> charts (+ table twin) -> arms -> paired differences -> mechanism -> settings -> "What this does and does not show". Incomplete terminal states: cancelled -> Partial notice but verdicts still drawn (E08, U-01); failed/interrupted -> "No result summary for this run" when no summary.json, otherwise result with "Treat anything below as incomplete".

**J7 Evidence and provenance (G01..G12).** [Evidence list] -> [Evidence detail: M6 sealed (RETAINED SIMULATED) | Scenario B (RETAINED REAL)] -> binding interpretation notice (Rendered/Raw/Copy) -> files "not downloadable here". Run artifacts: table -> authenticated blob download.

**J8 Simulation (H01..H03).** [Simulation] static explainer with SIMULATED banner; the earlier interactive demo is retired (G observation 16). Simulated runs live in Lab as T1.

**J9 Cluster and Intelligence (I01..I04).** [placeholder] state from capabilities: loading | not_enabled | unavailable; "What this page would show"; links to Overview and Lab evidence. R4 D2: `capabilities.controller` is keyed on the legacy `pie` field (`src/api/server.rs:510-513`: `pie.is_some()` -> active else "none"), so the Intelligence page says controller none while the AIMD origin cap controller runs and reports under `origin_cap`; the Lab strip chip says "Intelligence: not in serving path". Three statements about one fact (section 6, row S-12).

**J10 Live data pages (J01..J10).** gate -> sections with provenance badges -> banner when stale/degraded/disconnected. Telemetry under real traffic never observed (J10).

## 4. Device, viewport, theme and gateway-state inventory
| Dimension | Findings | Verified by project |
|---|---|---|
| Viewports | CSS breakpoints at 480, 640, 819/820, 959 and `pointer: coarse`; nav switches at 820 (`shell.css:62`); touch-target floor 44 px under 819/coarse (`app.css:313`) | 390, 820, 1280 light+dark: gallery (210 PNG) and matrix. 1920: 14 routes x 2 themes (rounds 1, 2) only. 320 and 640: layout/reflow metrics only (BV3: 74/75 pass, `#/` fails at 320 by 14 px, N-6). Real-device: none. |
| Device-specific behaviour | Phone: theme control and gateway address moved into Menu (`Shell.jsx:113-123`); sticky Start bar overlaps content (U-10, G section 5 item 6); tablet 820: rail leaves about 588 px (DD 7.2) and artifact columns break words (G item 5). No hover-vs-touch differences are modelled; no touch gestures exist. | gaps: no touch test, no landscape phone, no 320 screenshot review, no 1920 Lab run states |
| Themes | light / dark / system via `[data-theme]` and `prefers-color-scheme`; `forced-colors`, `prefers-contrast: more`, `prefers-reduced-motion`, `prefers-reduced-transparency` handled in CSS | dark/light parity PASS by text length and focus count (BV2); only 4 dark pairs examined by G; forced-colors screenshots saved, not examined; runtime OS-theme change not tested |
| Browsers | Chrome-only verification. App needs `EventSource`, `fetch` streaming, `<dialog>.showModal`, CSS `:has`? (UNVERIFIED), `pointer: coarse` | Firefox, Safari/WebKit: NOT COVERED (K09); no minimum browser baseline decided (AUD section 7 item 5) |
| Input | keyboard roving nav, skip link, Esc closes Menu/dialog | scripted CDP only; no screen reader (K08), no touch (K05) |
| Gateway states (telemetry) | loading, live, stale (>3 s), disconnected, error, degraded (6) | `reconnecting`, `unauthorised`, `unavailable` absent |
| Lab states | capabilities: loading, disabled, 401 (3 causes), 403 (2 causes), 421, network; per-resource loading/error/ready+refreshing; run stream 8 phases | partly observed |
| Auth | anonymous dev loopback; JWT Observer/Operator/Admin | both exercised in Lab; telemetry under JWT not exercised |

## 5. State machines (FACT unless marked)
**Telemetry connection** (`connection.js`): phases connecting|open|error|closed; derived kind = no payload: down -> disconnected, parse error -> error, else loading; payload: down -> disconnected, age > 3000 ms -> stale, violations or parse error -> degraded, else live. INFERENCE: during an automatic retry with prior data the phase is `connecting` (not down), so the chip returns to live/stale without a "reconnecting" word until the first new frame; the next-retry time is not exposed (DD 9.3 item 3 requests it).
**Run state** (`lab_api.rs` `state_rank`): queued(0) < running(1) < cancelling(2) < terminal(3). `request_cancel` moves queued or running to cancelling (so queued -> cancelling is possible, `lab_api.rs:709-716`); R4's L6 diagram omits queued -> cancelling and the documented cancelling -> completed race (`lab_api.md:65`); recovery rewrites queued/running/cancelling to interrupted (`lab_api.md:55`).
**Run stream** (`labStream.js`): idle -> connecting -> open -> (waiting -> connecting)* -> ended | failed; terminal errors (401, 403, 404, 421, disabled) stop retrying.
**Resources** (`useLabResource.js`): idle/loading/ready(refreshing)/error; a failed reload keeps earlier data visible with an error block above it (`Gate.jsx:37-41`).

## 6. Consistency audit (one condition must read the same on every surface)
| ID | Condition / term | Surface A | Surface B | Surface C/D | Issue | Evidence | Owner |
|---|---|---|---|---|---|---|---|
| S-01 | Gateway unreachable | Header chip: "Disconnected" (critical, glyph) | Data page: "No connection to the gateway" + Reconnect now (`ScreenFrame.jsx:74`) | Lab pill: "Not reachable or not authorised" (critical); Lab block: "Could not reach the gateway" + Retry (`labApi.js:71`); run stream: "Disconnected" in a warn-tone banner (`RunsViews.jsx:95`) | four phrasings, two tones, three retry verbs (Reconnect now / Retry / Refresh) | files cited | R1, R3 |
| S-02 | Unreachable vs not authorised | Lab pill merges them (`ConnectionPanel.jsx:69`) | ErrorBlock separates 401/403 causes (`Gate.jsx:10-24`) | telemetry chip cannot tell (EventSource) | U-14; DD 9.1 wants 4 states | R1, L |
| S-03 | Reconnecting | run stream says "Reconnecting" for attempts 1-2, "Disconnected" from 3 (`labStream.js:93`) | telemetry has no such word, goes straight to Disconnected | DD 9.1 "Recovering ... next try in 8 s" | same event, different escalation threshold and vocabulary | R1, R3 |
| S-04 | Stale threshold | telemetry 3 s (`connection.js:4`) | run stream 25 s (`labStream.js:14`) | "stale" tag with age (`States.jsx:76`) | thresholds are defensible (different heartbeats) but never explained to the reader | R1 |
| S-05 | Live / ok colour | header "Live" is ok-tone, labelled Derived (inferred) on Overview | run stream normal state is a small chip, not a banner (AUD C-07 fixed) | run "Completed" is ok tone | acceptable; keep one meaning of green = "healthy/complete" and do not use it for "retained" | R1, R2 |
| S-06 | Not available / not enabled / off | nav tag "Not available", group "Not available yet" | Placeholder: "Cluster mode is not enabled" (`viewmodel.js:128`) | Lab: "The Lab API is not enabled" (title), pill "Lab API off", StateBlock "The Lab is switched off"; chips "Not available" for run kinds | five phrases, one concept missing: is it unbuilt, disabled by config, or reported unavailable? U-09 | R1 |
| S-07 | "mode" | sidebar "Reports mode: live" (`Shell.jsx:121`) and Overview "mode" with a Real badge (`Overview.jsx:94`) = telemetry live/simulation | Lab Details "Authentication ... anonymous_dev_loopback_only" = security mode | Intelligence "Mode" and "Cluster mode" = controller/cluster capability | four meanings; U-08 | R1, R4 |
| S-08 | REAL qualifier | Live counters: "Real" with no basis (G section 5 item 3; `ProvenanceBadge`) | Lab: "Real (loopback, one host[, debug build])" always qualified (`labProvenance.js:12-19`) | R2 proposes diagrams/animation needing a label for "illustration" (not in the six words) | same word for a live gateway counter and a scripted loopback measurement; qualifier rule is only enforced in Lab | R1, R2, L |
| S-09 | Run state word vs tone | Cancelled = neutral, Interrupted = warn, Failed = critical, Completed = ok (`labProvenance.js:50-53`) | Verdict "Inconclusive" = warn, "Not supported" = critical | a Completed run with Not supported verdicts displays green + red | U-11 three status words for one run; keep, but teach the difference between run state and verdict | R1 |
| S-10 | Elapsed / progress | Elapsed DERIVED (client clock) or UNAVAILABLE | "0 of 2 units" (unit undefined, U-03) | progress events carry arm/rep/phase | server elapsed unavailable (AUD section 7) | R1, B |
| S-11 | Retry/refresh vocabulary | "Reconnect now" (telemetry, run stream) | "Retry" (ErrorBlock) | "Refresh" (run page, runs list), "Reload this page" (Lab off) | four verbs for "ask again" | R1 |
| S-12 | Intelligence | nav "Intelligence" tagged "Not available"; page "controller: none" | Lab chip "Intelligence: not in serving path" | Overview/Origin show `origin_cap` controller counters as REAL (R4 D2) | three statements; the AIMD origin-concurrency gate is real and in the serving path, PIE is not | R1, R4 |
| S-13 | Stale UI string | `viewmodel.js:139` "Lab runs are offline files today and are not served by the gateway" (only on a `lab` placeholder path) | Lab API now serves runs (`lab_api.rs:1031`) | R4 D10 | dead/contradicting copy; check if reachable | R3 |
| S-14 | Footers and honesty banners | footer sentence per page kind (`routes.js:108`); DD proposes removing the footer | the simulated-gateway banner appears on every non-Simulation page when `mode=simulation` | keep text true per page kind (P15) | R1 |
| S-15 | Gateway address placement | Menu/rail foot (phone: only inside Menu) | Lab Details | error text says "address shown in the header" (`labApi.js:73`) | the header does not show it (U-02) | R1, R3 |
| S-16 | Two "Start a run" controls | head button | tab | U-12 | R1 |
| S-17 | ARIA severity of connection loss | ConnectionBanner role=status (polite) (`States.jsx:113`) | error StateBlock role=alert | R3 proposes assertive for lost connection and run failure (`r3` 7.2 item 5) | a change of existing behaviour; decide once | R3, R1 |

## 7. Missing or weak screens, transitions and recovery experiences (gaps list)
| # | Gap | Evidence | Backend support | Owner |
|---|---|---|---|---|
| G-01 | Gateway address entry/test/persistence; invalid `?gateway=` feedback outside Overview | `config.js:4-22`; U-02; DD 9.3 | none needed (client) but needs lead decision on persistence | R1, R3, L |
| G-02 | Unauthorised vs disconnected vs unavailable distinction for **telemetry** | `connection.js`; BD-05 | needs auth design for SSE (owner-reserved) | L |
| G-03 | Recovering/reconnecting state with next-retry time for telemetry | `connection.js` (no `nextRetryAt`); DD 9.3 | client only | R3, R1 |
| G-04 | Cancelled-run presentation: verdicts suppressed, "No result: run cancelled" | U-01; E08 | backend omits or UI suppresses (X-01) | B/L, R1 |
| G-05 | Failed and interrupted run presentation verified in a browser; failed-run partial summary semantics | E09, E10 | what a failed run's `summary.json` contains is UNVERIFIED | R4 |
| G-06 | `queued` state never seen; presentation unknown at 390 | `RUN_STATES` | queued exists in API | R1, R3 |
| G-07 | Partial/slow/offline network, throttled stream, stale run stream (25 s), token expiry mid-stream, bfcache restore | E16, C05, B10 | n/a | R3 |
| G-08 | Run history pagination, filter, sort; 200-run index bound | G10; `lab_api.md:55` | `limit/offset` supported by API and client; UI passes none | R1, R3 |
| G-09 | Retained evidence files not downloadable; empty evidence list | G02, G08 | API serves names/sizes only | B (UNAVAILABLE) |
| G-10 | Run progress detail: unit definition, phase x of N, ETA | U-03; AUD section 7 | needs phase boundaries in events (backend) | B, R1, R2 |
| G-11 | Server-provided elapsed time | S-10 | backend | B |
| G-12 | Non-Chrome, touch, screen reader, forced-colors visual review, real devices | K05, K07..K09 | n/a | R3 |
| G-13 | Per-route 1920 coverage of Lab states and 320 screenshots in dark | A05, F14 | n/a | R3 |
| G-14 | Cancel race (cancelling -> completed) explanation | `lab_api.md:65`; E07 | none | R1 |
| G-15 | Lab pages while telemetry is down but Lab is up (and the reverse): header says Disconnected, Lab works (or JWT case) | B15 | none | R1, R3 |
| G-16 | Multi-tab / 6-connection browser limit with several run-stream tabs (`busy` 429 message exists) | BV3 N-2; `labApi.js:51` | gateway limits streams | R3 |

## 8. Cross-agent supervision

### 8.1 R1 (design intelligence, art direction)
Status: main file `r1_design_intelligence_and_critique.md` (46 KB) appeared at pass 3a; read sections 0, 6.2, 7, 10 and the coord replies. Do art direction and design intelligence fit the actual workflows? **Yes, by exclusion:** R1 proposes no external art-direction skill for the application UI (apple-design stays; frontend-design, taste-skill, Impeccable's bolder/delight/overdrive rejected as conflicting with the restrained token system), which fits a dense engineering/evidence tool and protects P9 (verdict weight parity, R1 section 7 rule 4 "no make-it-bolder pass over evidence pages"). R1's replies to R5 (coord/r1.md "Replies to R5"): keep strip chips and connection indicator as separate objects (DD 7.4); the rubric will include the cancelled/failed/interrupted run page and the gateway-stopped page as required review targets; skip link, `data-screen-heading` focus and `#/lab/...` deep links are must-not-regress. Gaps: (a) R1 section 7 maps R4 objects to critique help but does not mention U-01 or the connection-state inconsistency table (integration section 6); a rubric that checks "same condition, same words" across surfaces (S-01..S-17) is not stated; R1 itself proposes "one connection component, same on every screen" as a must-pass item, which is the right test but leaves the decision to the lead. (b) R1 relies on R4's code references without verification (stated), which R5 has now spot-checked. (c) R1 recommends `interface-design` (optional) and a `nielsen-heuristics-audit` copy (approval required, `.claude/skills` changes; commands listed, none run): a skill-installation decision for the lead and owner (add to E-07), with the license MIT requirement to keep LICENSE files. (d) Nielsen heuristics on this product are untested: no artefact shows the rubric applied to any current PhantomCache screen. (e) Conflict X2 (eyebrows/all-caps: Impeccable bans, DD v2 specifies) and X3 (hero-metric ban vs DD large numerals) are logged by R1 as resolved in favour of DD v2 and real data; R5 agrees for results but notes the Overview "Key signals" tiles (U-13, G item 1) are the place where a hero-metric rule actually bites.

### 8.2 R2 (motion, materials, diagrams, dataviz)
Does it ground motion in real behaviour? Yes in principle: section 0 and 10 rules (diagram not from received data is ILLUSTRATION; no animation implies liveness unless driven by REAL events with timestamps shown; cluster/intelligence static). Verified grounding claims (FACT): no `backdrop-filter` in `ui/src/styles` (grep: none); keyframes at `app.css:270-273`; reduced-motion at `app.css:277`; reduced-transparency at `tokens.css:155`; Chart/Table frame exists. Findings:
- R2-1 **Vocabulary**: "ILLUSTRATION" is not one of the six words; a label that is not a provenance badge needs lead approval or must stay a caption (X-04).
- R2-2 **Connection pulse**: R2 table row "Gateway connection states ... pulse for alive" is the only motion on a state that the code models as 6 kinds; "alive" is DERIVED (inferred) client-side (`provenance.js:13`). Acceptable only if reduced-motion text is identical and it animates on change, not continuously.
- R2-3 **Materials**: R2 recommends no glass skill and notes text-over-blur contrast risk; DD v2 supersedes the "no glass by default" rule. Positions differ (X-06); the contrast tooling assumes solid backgrounds (BV: "solid backgrounds only").
- R2-4 **Missing**: no mention of 320/390/touch cost, of how run-state transitions (cancelled/failed/interrupted) should look, or of what motion is allowed when a stream is `stale`/`reconnecting` (must stop). Request R2-3.
- R2-5 R2 assumes per-object event rates (≤ 1 update/s) pending R4; the stream frame is 100 ms and history samples at >= 500 ms (section 2), which is an input R2 should adopt.
- R2-6 R2 correctly keeps the Simulation page non-animated; H01 shows no live simulation remains to animate. Any new simulation motion must carry SIMULATED and must not reintroduce a fake live dashboard (retired by I3).

### 8.3 R3 (engineering, a11y, testing)
Covers all devices and flows? **Not yet.** Verified claims (FACT): `forbidden.test.mjs:64-66` asserts exactly react, react-dom, lucide-react (and vite, plugin-react as dev); `lab_api.rs:1031` events route; `a11y_evidence/` files exist. Gaps and corrections:
- R3-1 **Test plan is organised by proposed "experiences"** (Request Flow, Cache Lifecycle, etc., section 7.1) rather than by the existing journeys; JWT, disconnected, cancelled, failed, interrupted, 320 and 1920, Firefox/WebKit and touch do not appear in its plan (grep of the file for those terms finds no plan item). Required additions in section 11.
- R3-2 **Rate assumption is off**: R3 says the live path is "order 1 Hz snapshots" and names the 1 s `setNow` tick as the main re-render source (7.0). The SSE frame cadence is 100 ms (`server.rs:959`); each frame produces a new reducer state (`connection.js:33-42`) and a provider re-render; only rates are sampled at 1 s (`server.rs:462`). So the 10 Hz provider update is likely the larger source (INFERENCE; R3 itself marks "not profiled"). R3 should measure before budgeting.
- R3-3 **Assertive live region** for lost connection (7.2 item 5) changes current polite behaviour (S-17).
- R3-4 **Approval requests** are in the required format (8.1): A1 `axe-core@4.14.0` dev-only with purpose, alternatives, maintenance, security (0 deps, no install scripts), licence MPL-2.0, exact command (not run); A2 `pixelmatch@8.0.0` optional, recommended not to request; A3 conditional headless primitive (runtime dependency, would need editing `forbidden.test.mjs`). R5 note: A1 also implies a code change (`ui/scripts/check-axe.mjs`); both must be approved separately (CLAUDE.md section 2). Also: `lucide-react` is a runtime dependency listed in `package.json` but no `lucide` import exists in `ui/src` (grep, 0 matches): not in R3's scope, but a candidate for a dependency cleanup decision (UNVERIFIED that nothing else imports it, e.g. the harness).
- R3-5 Fixture realism (Section 2 item 2): add a test that builds a cancelled/failed fixture from real backend output (a Rust-side captured `summary.json`) so UI tests cannot drift (X-01).
- R3-6 R3 says its baseline cannot use a recorded SSE for visual regression without state fixtures; the existing `ui/test/fixtures/lab/*` already includes run_cancelled, events_cancelled.sse, errors.json, capabilities_jwt.json, which is a solid base.

### 8.4 R4 (system objects, lifecycles, telemetry)
Overall accurate against code in the spot-checks below. Missing for the product-flow view: transitions queued -> cancelling and cancelling -> completed in L6; how failed/interrupted Scenario B runs persist partial summaries (G-05); the telemetry stream's auth limitation (BD-05).
Spot-check log (R5 opened the source):
| # | R4 claim | Source checked | Result |
|---|---|---|---|
| 1 | D1 `capacity_weight = 1.0` constant, plain HRW | `src/dist/router.rs:1363,1381` | CONFIRMED |
| 2 | D2 `capabilities.controller` keyed on `pie` | `src/api/server.rs:510-513` | CONFIRMED (`pie.is_some()` -> "active" else "none") |
| 3 | D4 local node shares the serving cache | `src/main.rs:332` | CONFIRMED (`VirtualNode::new_with_cache(node_id, cache.clone())`) |
| 4 | D6 stream percentiles computed by sorting raw ring | `src/intel/telemetry.rs` `calculate_percentiles_us` (sorts `raw_buffer_us`); ring capacity `main.rs:254` = 10000 | CONFIRMED |
| 5 | D7 default TTL literal 60_000 at server.rs:1405,1688 | `src/api/server.rs:1405,1688` | CONFIRMED (CLAUDE.md quotes ~1077/1352: line numbers drifted) |
| 6 | D8 anti-entropy worker called at main.rs:375,383 | `src/main.rs:375,383` | CONFIRMED |
| 7 | D9 AuditStore created, nothing appends | `src/main.rs:480` creates it; no append found by a grep over `src/intelligence/*.rs` and `server.rs` | CONFIRMED by grep only (INFERENCE) |
| 8 | D10 viewmodel text "Lab runs are offline files today" | `ui/src/lib/viewmodel.js:139` | CONFIRMED verbatim |
| 9 | D5 `hit_ratio_cumulative` field in schema 2 | `src/api/server.rs:740` | field CONFIRMED; the semantic claim (excludes coalesced/stale/bypass) not re-derived |
| 10 | L4 promotion needs no debt at or before ticket | `src/dist/router.rs:2133-2150` | CONFIRMED (comment and `has_debt_at_or_before`) |
| 11 | 4.2 Traffic keeps three sparklines in page history | `ui/src/screens/Traffic.jsx:33-35` (throughput derived, hit ratio derived, p99 measured) | CONFIRMED |
| 12 | 4.2 Simulation: static 3 dots -> 1 line -> origin square, no numbers | `ui/src/screens/Simulation.jsx` `CoalescingIllustration` | CONFIRMED |
| 13 | L6 run lifecycle | `src/api/lab_api.rs:604-720`, `lab_api.md:43,55,65` | PARTLY: omits queued -> cancelling, cancelling -> completed race, interrupted from any of queued/running/cancelling |
| 14 | 4.2 provenance words "Real", "Derived (inferred)" | `ui/src/lib/provenance.js:7-13` | CONFIRMED |
| 15 | Section 1.3 line citations (R4 asked R5 to check these, tree is dirty): `cache.rs:749` `check_stale_serving`, `:1025` `complete_not_modified`, `:1401` `finalize_fetch`, `:576` `try_invalidate_tenant`, `:824` `record_stale_served` | `src/core/cache.rs` at those lines | 5 of 5 CONFIRMED (function definitions at the cited lines) |
| 16 | `main.rs:433` `pie: None`; `freshness.rs:139` `evaluate_response_freshness` | `src/main.rs:433`, `src/core/freshness.rs:139` | 2 of 2 CONFIRMED |
| 17 | "Policy default Sieve (`cache.rs:327`)" | `src/core/cache.rs:332` (`eviction_policy: Mutex::new(EvictionPolicy::Sieve)`); line 327 is `next_op_id` | claim TRUE, citation off by 5 lines (drift) |
14 claims opened in the main-file table above plus the line-citation checks (rows 15 to 17: 8 of 9 exact, 1 drifted); 13 of the first 14 confirmed in full or in the relevant part, 1 partly (L6).
R4 answered R5 requests 1 to 3 in `r4_lifecycle_and_event_matrices.md` M0.2 to M0.4. It independently confirms the telemetry-under-JWT limitation (`src/api/server.rs:940-952`, read by R5), lists the Scenario B progress phases (`starting, warmup, healthy, degraded, recovery, collect, run-complete`; R5 confirmed them in `src/lab_live/scenario_b.rs` at lines 977, 833, 1065, 836, 1091, 1241) and the 16-stream limit (`src/api/lab_api.rs:59`, `MAX_SSE_STREAMS = 16`: confirmed), and classes the telemetry and run-stream kinds as CLIENT observations. M0.2 lists `cancelled` for both run kinds and marks the verdict content of a cancelled summary as UNVERIFIED by R4; R5 verified it (section 1 item 2). Still open from R4: failed-run partial-summary contents (G-05) and the L6 correction. Totals: 14 main-file claims + 3 matrices-file claims + 9 line citations = 26 items opened; 24 exact or confirmed in the relevant part, 1 partly (L6), 1 citation drifted (`cache.rs:327` vs 332).

## 9. Cross-agent conflicts and dependencies (status)
| ID | Positions | Status |
|---|---|---|
| X-01 | U-01: UI should suppress verdicts for cancelled/failed/interrupted vs backend should omit them; today neither, tests mask it | OPEN, escalated (section 12) |
| X-02 | Telemetry auth under JWT (BD-05) vs DD connection states (Unauthorised) | OPEN, owner |
| X-03 | Gateway address UI (DD 9.3) requires state/persistence design; R3 test plan and R1 art direction both depend on it | OPEN, lead |
| X-04 | R2 "ILLUSTRATION" label vs six-word provenance vocabulary | OPEN, lead |
| X-05 | R1 and R2 coord say DD v2 and Track 2 are absent; DD exists, Track 2 still absent | OPEN, notify R1/R2/R3 (R3 section 7.3 written without Track 2; `a11y_evidence/` suggests it is in progress) |
| X-06 | Glass: DD v2 supersedes "no glass by default"; R2 recommends no glass skill and flags text-over-blur contrast; R3 asserts no glass budget until DD results exist | OPEN, lead |
| X-07 | Re-render source: R3 (1 s tick) vs code (100 ms frames) | OPEN, R3 to measure |
| X-08 | Live-region politeness: R3 assertive vs current polite | OPEN, R3/R1 |
| X-09 | R2 assumed ≤ 1 update/s; R4/R3 stream facts say 100 ms frames, 1 s rate samples, 250 ms Scenario B snapshots | OPEN, R2 to adopt R4 numbers |

## 10. Existing functionality at risk during redesign (see preserve list P1..P15 in the matrix)
1. **Header changes (DD 7.1):** connection chip becomes a button opening a popover; theme control moves to an icon menu. Risks: `[data-theme-option]` selectors used by tests and browser scripts, keyboard/touch operability of the popover, the live region tied to the chip (`App.jsx:136`), P2 skip-link order.
2. **Rail "Not available yet" collapsed (DD 7.1):** hides the honesty signal for Cluster and Intelligence if collapsed by default; the nav `aria-hidden` tag plus visually-hidden suffix (`Shell.jsx:103-105`) must survive.
3. **Footer removal (DD 7.1):** footers are the page-kind honesty sentences (P15); moving them to "About this data" must keep one per page kind.
4. **Glass/blur:** invalidates the contrast checker's solid-background assumption (BV), increases risk on phones and `prefers-reduced-transparency` (`tokens.css:155`).
5. **Status-strip restyle:** `ConnectionPanel.jsx` is imported by name by the render harness (`ConnectionPanel.jsx:5`); the error pointer "Open Details in the Lab status strip above" (`Gate.jsx:18`) must follow any rename; honesty statements (debug build, intelligence not in serving path) must stay always visible (P6).
6. **Result page re-composition (DD 10):** risk of moving the binding notice or limitations behind folds (P5, G item 8), and of re-introducing verdict emphasis asymmetry (P9).
7. **Charts redesign (R2/DD 10.3):** keep table twin, text values, unavailable panels, `aria-hidden` marks (P10, P11).
8. **Any new dependency (R2/R3):** `forbidden.test.mjs:64-66` fails on a runtime dependency change; dev-dependencies likewise asserted (line 65).
9. **Motion:** reduced-motion collapse (P13) must apply to any new transitions; focus management after cancel dialog (`RunsViews.jsx:342-348`).
10. **Routes:** hash IDs and titles (P1) and `data-*` hooks used by browser scripts (`data-screen-heading`, `data-cancel-dialog`, `data-lab-status`, `data-run-id`).

## 11. Required regression and end-to-end validation (concrete)
Fixtures to exist (real, not hand-edited): captured `summary.json` for a **cancelled** and a **failed** Scenario B run from the backend; run event logs ending in `interrupted` (recovery) and in `failed`; JWT capabilities (exists: `capabilities_jwt.json`).
| ID | Test | Viewports / themes | State covered (matrix) |
|---|---|---|---|
| T-01 | Cancelled Scenario B run page shows no verdict chip, no "Mechanism failed", and the Partial notice (acceptance for X-01) | 390/820/1280, light+dark | E08 |
| T-02 | Failed and interrupted run pages (alert, recovery notice, "No result summary") | 390, 1280, light+dark | E09, E10 |
| T-03 | Gateway stop/start: header chip, page block, Lab pill/ErrorBlock, run-stream banner all show the same cause and one recovery path; recovery is announced | 320, 390, 1280; light+dark | B05..B09, B15 |
| T-04 | JWT: no token / expired / wrong role / operator-only; **non-Lab routes under JWT** (expected Disconnected: document actual behaviour) | 390, 1280; light | C01..C09, B13 |
| T-05 | Slow/throttled network and silent stream (stale 25 s, reconnect 1-2-4 s, disconnected at attempt 3, failed on 401) | 1280; light | E05, E16, C05 |
| T-06 | Start form: pristine, invalid, presets, sticky bar overlap, 24/44 px targets; both kinds | 320, 390, 820, 1280; light+dark | D01..D12 |
| T-07 | Cancel dialog: focus start, Escape, sending state, focus return, `cancelling` state; Firefox and WebKit if approved | 390, 1280 | E06, E07 |
| T-08 | Result and evidence pages: interpretation notice verbatim, limitations visible, verdict parity, table twin, no page-level horizontal scroll | 320, 390, 640, 820, 1280, 1920; light+dark | F01..F14, G03..G06 |
| T-09 | Runs list: empty, 1, many, bounded index note; pagination if added | 390, 1280 | G10, G11 |
| T-10 | Cluster/Intelligence/Simulation/Overview in simulated-mode gateway and in stale/degraded states | 390, 1280 | H03, I01..I03, J06, J07 |
| T-11 | Keyboard-only run through the main journey (J1 to J6) and a screen-reader pass (human) | 1280, 390 | K04, K08 |
| T-12 | Touch pass on a real phone and tablet (human) | 390-class, 820-class | K05 |
| T-13 | Token-leak and forbidden-pattern scan on the final bundle (token not in URL, storage, DOM text, copied request) | any | P8, C02 |
| T-14 | Visual regression baselines labelled with Chrome version and OS, animations off, fixed clock | all gallery states | all TESTED rows |
| T-15 | Theme: switch at runtime, OS change while on System, forced-colors and prefers-contrast screenshots examined | 390, 1280 | K01, K07 |
| T-16 | Real-traffic telemetry soak (60 s) with moving values: sparklines, 100 ms frames, re-render cost | 1280 | J10, X-07 |
Independent verification: at least T-11, T-12 and a human review of E08/E09/E10/B05..B07 should be recorded as INDEPENDENTLY VERIFIED when done; none is today.

## 12. Escalations and owner decisions (clearly marked)
| ID | Decision | Why it cannot be settled by the research agents |
|---|---|---|
| **E-01** | U-01 cancelled/failed run verdicts: (a) backend writes no verdicts when the run is not complete, (b) UI suppresses verdicts when run state is not `completed`, (c) both. | (a) changes the Lab backend/artifact contract (`scenario_b.rs`); (b) is an application-code change. Both need approval (CLAUDE.md section 2). Also decide whether retained artifacts remain unedited (`lab_api.md:61`). |
| **E-02** | Telemetry stream authentication under JWT (BD-05, `sse_auth_options.md`): choose an option, or accept that non-Lab pages are unavailable on protected gateways and say so in the UI. | Security architecture, owner-reserved. |
| **E-03** | Gateway address UI: runtime setter and whether to persist the address in `localStorage` (token stays in memory only). | Product behaviour + storage; DD 9.3 asks the lead. |
| **E-04** | Provenance vocabulary: whether "REAL" may appear unqualified on live gateway counters; whether "ILLUSTRATION" is a label; whether REPLAYED/PREDICTED may appear before any backend data exists. | Owner vocabulary decision (2026-10-30) already covers six words. |
| **E-05** | Minimum browser baseline and whether Firefox/WebKit verification is required. | Product scope. |
| **E-06** | Glass and `backdrop-filter` (DD v2 supersedes the rule) vs contrast tooling that assumes solid backgrounds. | Design direction + accessibility policy. |
| **E-07** | Dependencies: R3 A1 `axe-core@4.14.0` (dev-only, MPL-2.0, 0 deps, requires a code change `check-axe.mjs`), A2 `pixelmatch@8.0.0` (R3 recommends not requesting), A3 headless primitive (runtime; would require changing `forbidden.test.mjs`). R2: GSAP (non-OSI licence), diagram-design (Google Fonts fetch) not recommended. | Dependency approvals (CLAUDE.md section 2). |
| **E-08** | Intelligence page wording vs reality: the AIMD origin gate runs in the serving path while `capabilities.controller` says "none" (R4 D2). | Presentation of a capability fact. |
| **E-09** | `lucide-react` listed as runtime dependency with no import in `ui/src` (UNVERIFIED for scripts). | Dependency hygiene; only after verification. |

## 13. Prioritised recommendations (PROPOSAL)
1. **P0:** Resolve E-01 and add T-01 with a real backend fixture before any Lab results redesign.
2. **P0:** Decide E-02 and document what non-Lab pages show on a JWT gateway; until then, treat B13 as an unverified risk in any connection-state design.
3. **P1:** One connection-state vocabulary and component set shared by header, data pages, Lab strip and run page (S-01..S-04, S-11, S-17), including `Unauthorised` for Lab and `Recovering` with next retry; Lab `Start a run` hidden or disabled while unreachable (U-14, G item 4).
4. **P1:** Gateway address UI (E-03) with a visible address in the header on all viewports (S-15, U-02).
5. **P1:** Terminology pass: "Not available / not enabled / off" (S-06), "mode" (S-07), Retry verbs (S-11), Intelligence statements (S-12).
6. **P1:** Close the test-realism gap: fixtures from backend output; add T-02..T-05; add 320/1920 screenshots of Lab states.
7. **P2:** Run history pagination/filter UI using existing `limit/offset` (G-08); empty-evidence state; run page `queued` look.
8. **P2:** Define "unit" and show phase position for progress (U-03), pending backend phase data.
9. **P2:** Non-Chrome and touch verification plan with an approval request for any tooling (E-05, T-12).
10. **P3:** Human independent review of the main journeys; record results as INDEPENDENTLY VERIFIED rows.

## 14. Coverage summary (from `r5_coverage_matrix.md` section 4, mechanical count)
115 rows: TESTED 93, IMPLEMENTED 11 (A10, B02, B08, B09, B15, B16, E08, E11, G02, G10, K07), UNAVAILABLE 1 (G08), NOT COVERED 10 (B01, B13, C05, E04, F14, H02, J10, K05, K08, K09), PROPOSED (primary) 0, INDEPENDENTLY VERIFIED 0. "TESTED" is not human verification and often means SSR fixture rendering or one agent-observed screenshot.

## 15. Review log
- Pass 1 (~09:35Z to 09:42Z): UI source, three browser rounds, docs 02/04/05, DD 7/9/13, R2 sections 0/1/10, coord files. Output: matrix and this file.
- Pass 2 (~09:48Z): R3 and R4 main files appeared; 14 R4 claims and 4 R3 claims spot-checked in source; telemetry cadence correction; queued -> cancelling transition verified; matrix unchanged except counts mechanically recomputed.
- Pass 3a (~09:55Z): R4 matrices file M0.2 to M0.4 reviewed; three more claims confirmed (phases, 16-stream limit, JWT SSE auth). Phase names are now known, which narrows G-10 (a phase position can be shown from `phase`; totals and timings are still not in events).
- Pass 3b: pending (R1 main file, `toolkit_proposal.md`, Track 2 `03_validation_accessibility.md`; re-count the matrix; add rows for any new states or screens that R1 to R4 propose).
