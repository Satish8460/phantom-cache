# 06 Functional UX work breakdown (agent FUNC, Priority 2)

**Status:** PLAN ONLY. Nothing here is implemented, approved or started. **Date:** 2026-10-30. **Writer:** agent FUNC (single writer of this file; read-only on the repository).
**Gate G0 (binding):** `ui/**` is NOT modified until (a) the research and evidence-collection phase is closed and (b) the owner states in plain text that the implementation phase is ready. This file is the breakdown to execute at that point. `src/**`, `tests/**` and `docs/user/**` are additionally behind the per-item authorisations marked **AUTHORISATION REQUIRED**.

**Evidence tags:** FACT (file:line read in this pass), REPRODUCED (observed here), INFERENCE, PROPOSAL, UNVERIFIED. Prior-agent reports (04, 05, R4, R5) were used as pointers only; every FACT below was re-read in code.
**Baseline (REPRODUCED here, 2026-10-30, `node --test test` in `ui/` with `TEMP=D:\Temp`, working tree as found, redesign possibly in flight):** 298 tests, 298 pass, 0 fail. Git HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 93 status entries before and after (nothing changed by this pass except this file).

---

## 0. Summary of decisions this document asks for

1. U-01 is **two defects**: a UI defect (renders verdict chips for any terminal state) and a backend data defect (`summarize()` computes and labels verdicts for cancelled runs, and conflates "no runs recorded" with "mechanism violated"). The UI correction needs no backend change and is **WP-F1/F2**. The backend change is **WP-F8, AUTHORISATION REQUIRED**, and the UI correction must stay correct after it (section 3.3).
2. A single **connection vocabulary** (Live, Reconnecting, Disconnected, Unauthorised, Unavailable, plus Stale/Degraded/Checking) with one retry verb, used on every surface (section 4).
3. A **gateway address** feature (visible, editable, tested, persisted non-secret address; token forgotten on any origin change) (section 5).
4. **Progress and recovery semantics** that only show what events support (section 6).
5. Fifteen owner decisions with recommendations (section 9).

---

## 1. Ownership table

Roles. **LEAD-FUNC** = the lead Claude (functional UX owner; sole writer of every file listed in its rows). **REVIEWER** = a separate agent/person who never writes the files under review and runs the independent pass. **DS-IMPL** = owner of the visual redesign increments (I1..I5, `00_progress.md`); DS-IMPL keeps `ui/src/styles/**` and the visual structure of components. **BACKEND-OWNER** = whoever the owner authorises for `src/**`; no row is assigned until the authorisation is granted.

Rule (CLAUDE.md section 7): one writer per file at a time. Where a file is shared between a functional change and a redesign increment, the functional change is split into (i) a pure module that DS-IMPL never touches and (ii) a **wiring slot** executed after DS-IMPL releases the file (or by DS-IMPL's writer in the same increment, with LEAD-FUNC supplying the prop contract). No row below overlaps another row's files.

| WP | Title | Writer | Reviewer | Files MODIFIED (exact) | Files CREATED (exact) | Gate |
|---|---|---|---|---|---|---|
| F0 | Real-backend fixtures and provenance | LEAD-FUNC | REVIEWER | none | `ui/test/fixtures/lab/real/**` (JSON/JSONL/SSE copies), `ui/test/fixtures/lab/real/PROVENANCE.json`, `ui/scripts/capture-lab-fixtures.mjs` | G0 |
| F1 | Result presentation model (U-01 UI, pure) | LEAD-FUNC | REVIEWER | `ui/src/lib/labView.js`, `ui/test/lab.view.test.mjs` | `ui/test/lab.result.test.mjs` | G0, F0 |
| F2 | Result presentation wiring (U-01 UI, JSX) | LEAD-FUNC | REVIEWER | `ui/src/screens/lab/RunsViews.jsx`, `ui/src/screens/lab/ScenarioBResult.jsx`, `ui/src/screens/lab/LabScreen.jsx` (RunDetailPage only), `ui/test/lab.render.test.mjs`, `ui/test/lab.hierarchy.test.mjs` | none | F1; slot after DS-IMPL releases the three screens |
| F3 | Connection state model (pure) | LEAD-FUNC | REVIEWER | `ui/src/lib/connection.js`, `ui/src/lib/labStream.js` (status vocabulary only), `ui/src/hooks/useTelemetry.js`, `ui/test/connection.test.mjs`, `ui/test/lab.stream.test.mjs` | `ui/src/lib/linkState.js`, `ui/test/linkState.test.mjs` | G0 |
| F4 | Gateway address model (pure) | LEAD-FUNC | REVIEWER | `ui/src/lib/config.js`, `ui/src/lib/labApi.js` (add `probe`, text of `ERROR_TEXT.network`), `ui/test/misc.test.mjs`, `ui/test/lab.api.test.mjs`, `ui/test/lab.guard.test.mjs` (see 5.7: guard edits need REVIEWER sign-off) | `ui/src/lib/gatewayStore.js`, `ui/src/hooks/useGateway.js`, `ui/test/gateway.test.mjs` | G0 |
| F5 | Connection and address wiring (JSX) | LEAD-FUNC | REVIEWER | `ui/src/App.jsx`, `ui/src/components/Shell.jsx`, `ui/src/components/States.jsx`, `ui/src/screens/ScreenFrame.jsx`, `ui/src/screens/Overview.jsx`, `ui/src/screens/lab/ConnectionPanel.jsx`, `ui/src/screens/lab/Gate.jsx`, `ui/src/screens/lab/LabScreen.jsx` (LabProvider, LabInner only), `ui/src/hooks/LabContext.js`, `ui/test/shell.test.mjs`, `ui/test/render.test.mjs`, `ui/test/lab.render.test.mjs` | `ui/src/components/GatewayPopover.jsx` | F3, F4; collides with I4 (Shell/App/States) and I2 (Lab screens): see 8 |
| F6 | Progress model and run-form defaults | LEAD-FUNC | REVIEWER | `ui/src/lib/labView.js` (progress functions only; after F1 releases it), `ui/src/lib/labForms.js`, `ui/src/screens/lab/RunForm.jsx`, `ui/src/screens/lab/RunsViews.jsx` (ProgressBlock only; after F2), `ui/test/lab.forms.test.mjs`, `ui/test/lab.round2.test.mjs` | `ui/test/lab.progress.test.mjs` | F1, F2 |
| F7 | Recovery, start-gating, history pagination | LEAD-FUNC | REVIEWER | `ui/src/screens/lab/LabScreen.jsx` (RunsPage, LabInner head action; after F5), `ui/src/screens/lab/RunsViews.jsx` (RunsListBody, copy only; after F6), `ui/test/lab.render.test.mjs` | `ui/test/lab.recovery.test.mjs` | F5, F6 |
| F8 | Backend `summarize()` semantics for non-complete runs | **BACKEND-OWNER: AUTHORISATION REQUIRED, not assigned** | independent REVIEWER mandatory | `src/lab_live/scenario_b.rs`, `src/api/lab_api.rs` (retained snapshot, lines ~1383-1390), `tests/integration/lab_live_scenario_b_tests.rs`, `tests/integration/lab_api/mod.rs`, `docs/user/lab_api.md`, `docs/demo/scenario_b_artifacts.md` | none | owner authorisation (D-1) |
| F8b | Optional: report default phase lengths in `capabilities` | **AUTHORISATION REQUIRED, not assigned** | REVIEWER | `src/api/lab_api.rs`, `docs/user/lab_api.md`, matching test | none | owner authorisation (D-14) |
| F9 | Telemetry stream under JWT | **UI part: owner decision D-3; backend options A/B/C: AUTHORISATION REQUIRED, not assigned** | REVIEWER (security-minded) | UI: `ui/src/hooks/useTelemetry.js`, `ui/src/lib/labApi.js`, `ui/src/lib/sse.js` consumers; backend: `src/api/server.rs` | none | owner decision (BD-05, E-02) |
| F10 | Browser validation and regression | REVIEWER (not the writer of F1..F7) | LEAD-FUNC reads evidence | none | `ui/scripts/browser-func-*.mjs`, evidence under `docs/design/remediation/func_evidence/**` | each WP's exit |

FACT (collision analysis): the screens F2/F5/F6/F7 touch are exactly the ones the redesign owns (`RunsViews.jsx`, `ScenarioBResult.jsx`, `ConnectionPanel.jsx`, `Gate.jsx`, `RunForm.jsx`, `Shell.jsx`, `States.jsx`, `App.jsx`); `ui/scripts/harness-entry.jsx` imports `ConnectionPanel.jsx` by name (`ConnectionPanel.jsx:5` comment), so the default export name and prop names `res, view, hasToken, roles, onToken, onClear, gatewayUrl` must be kept or the harness and `lab.render.test.mjs:134-150` change in the same commit.

---

## 2. Verified facts the plan rests on

| # | Fact | Evidence |
|---|---|---|
| A1 | `summarize()` always evaluates the verdict rule and emits `verdicts`, `mechanism_h_b1`, `mechanism_h_b1_all_pass` whether or not the run was cancelled; the only marker is `"status": "CANCELLED"`. | `src/lab_live/scenario_b.rs:647-731` (esp. 706, 714), called with `cancelled` at `:1262-1263` |
| A2 | For a capped arm with zero recorded runs, `mech_pass = !rs.is_empty() && violating.is_empty()` is `false`. `evaluate_arm` checks `!mechanism_pass` first and returns `MECHANISM_FAILED` with reason "(H-B1 falsified)", taking precedence over "only N valid paired runs". | `scenario_b.rs:658`; `src/lab_live/analysis.rs:129-137` |
| A3 | A cancelled run drops the in-flight arm-run (`Err(Cancelled)` before it is pushed), so cancelling during the first arm-run yields zero runs for every arm. | `scenario_b.rs:1095-1097, 1243-1246` |
| A4 | A failed scenario (`Err` other than Cancelled) returns before `summarize()`; `summary.json` is NOT written, `manifest.json` carries `status: FAILED`. | `scenario_b.rs:1247-1257` |
| A5 | On startup, runs in `queued|running|cancelling` become `interrupted` with `exit_code: null`, `error.code: interrupted`, and `state`/`done` events (`source: server_recovery_on_startup`) appended. The runner is in-process, so no summary is produced by recovery. | `src/api/lab_api.rs:906-957`; `docs/user/lab_api.md:55` |
| A6 | A `completed` run emits a `result` event with `label`, `label_reasons`, `verdicts`; a `cancelled` Scenario B run is finished with NO result event but WITH an artifact index (so `summary.json` is listed). A T1 cancel unwinds a `CancelMarker` panic and finishes with no result and no artifact index. | `lab_api.rs:1778-1791, 1853-1869` |
| A7 | `progress` events carry `done` (completed arm-runs), `total` (= repetitions x arms), `arm`, `rep` (1-based: `p.rep + 1`), `phase`, `gateway_pid`, `tier`. Phase values: `starting`, `warmup`, `healthy`, `degraded`, `recovery`, `drain`, `collect`, `run-complete`. `done` counts finished arm-runs; one arm-run = one fresh gateway process. | `lab_api.rs:1843-1847`; `scenario_b.rs:970-977, 1062-1081, 1091, 1241` |
| A8 | Persisted `events.jsonl` lines carry `ts` (UTC, 1 s resolution); the SSE `data:` payload does not. `events.jsonl` and `run.json` are listed as artifacts for all runs. | scratch artifact below; `docs/user/lab_api.md:51` |
| A9 | Telemetry uses a bare `EventSource`; on `onerror` it sets phase `error` (or `closed` when `readyState===2`), only `closed` triggers our own backoff; the browser retries `error` by itself. No status code is visible. | `ui/src/hooks/useTelemetry.js:32-47`; `ui/src/lib/connection.js:25-27, 58-71` |
| A10 | Server frames every 100 ms; `/api/events` returns 401 before streaming when JWT is on. CORS allow-list includes `authorization` and `last-event-id`. | `src/api/server.rs:940-957, 158-164`; `docs/design/sse_auth_options.md:7` |
| A11 | Lab run stream is fetch streaming with `Authorization` and `Last-Event-ID`, reconnecting 1,2,4..30 s; `reconnecting` for attempts 1-2, `disconnected` from 3; stale after 25 s (heartbeat 15 s). | `ui/src/lib/labStream.js:14, 22-26, 87-98`; `ui/src/hooks/useRunStream.js` |
| A12 | The Lab pill merges "unreachable" and "not authorised" and any non-404 error with no data into "Not reachable or not authorised". | `ConnectionPanel.jsx:69`; asserted by `lab.render.test.mjs:144` |
| A13 | Gateway address: only `?gateway=` then `VITE_PHANTOM_GATEWAY`, else `http://localhost:8085`; an invalid value falls back silently and `gateway.error` is rendered only on Overview; the gateway never changes after load. `labApi` `network` text points at "the address shown in the header", which the header does not show (address lives in the Nav foot, hidden under Menu on phones). | `ui/src/lib/config.js:1-22`; `ui/src/App.jsx:55-61`; `ui/src/screens/Overview.jsx:90`; `ui/src/lib/labApi.js:73`; `ui/src/components/Shell.jsx:118-122` |
| A14 | `LabProvider` builds the Lab client from `baseUrl`; the token is in a `useRef` that survives a `baseUrl` change, i.e. today (if the address were mutable) a token entered for gateway A would be sent to gateway B. | `ui/src/screens/lab/LabScreen.jsx:22-48` |
| A15 | Source guards: `fetch(` is allowed only in `lib/labApi.js` (all source files); the "lab file set" (`screens/lab/`, `components/Lab*`, `lib/lab*`, `lib/sse`, `hooks/(Lab|useLab|useRunStream)`) must contain no `localStorage`, `sessionStorage`, `EventSource`, console logging, or credential-in-URL; the provider must contain `tokenRef = useRef(null)`. | `ui/test/lab.guard.test.mjs:5-45`; `lab.api.test.mjs:196-200`; `lab.fixes.test.mjs:222-231` |
| A16 | The only cancelled-run fixture, `run_cancelled.json`, has `spec.arms = ["BASE"]`; BASE-only runs have no capped arm so `verdicts` is `{}`. No fixture can exercise U-01. The test at `lab.view.test.mjs:282-290` additionally hand-sets `s.verdicts = {}` and `s.paired_vs_base = {}`; it asserts what the author wanted the backend to emit. | `ui/test/fixtures/lab/run_cancelled.json`; `ui/test/lab.view.test.mjs:282-290` |
| A17 | `RunsPage` calls `client.runs({signal})` with no `limit`/`offset` (server default 50, max 200, index covers newest 200); the client already supports both. | `LabScreen.jsx:145-156`; `labApi.js:219`; `lab_api.rs:55, 1138-1167` |

**REPRODUCED here (U-01 data defect, from a REAL backend artifact).** Scratch run `D:\Temp\usab_lab\runs\20261030T085920Z-9c26980f` (produced by the usability agent from the debug gateway, spec `arms=[BASE,STATIC-CAP]`, `repetitions=1`, 20 s phases, cancelled before the first arm-run finished). Its `out/summary.json` has `status: "CANCELLED"`, `mechanism_h_b1.STATIC-CAP = {pass:false, runs:0, max_origin_concurrent_observed_whole_run:null}`, `mechanism_h_b1_all_pass:false`, `verdicts.STATIC-CAP.outcome = "MECHANISM_FAILED"`, reason "origin concurrency exceeded the cap in force in at least one run (H-B1 falsified)", evidence `n_valid_pairs:0`. No run ever started a measurement, yet the artifact says the cap was violated. Disk hashes (not the served bytes; see F0): `summary.json` f4b296ea8bf4afd1d8380f8604130db080e96b2ad871d8ccdb24ce9b7ab6cea3, `events.jsonl` 5520ead6f90f5d5a728390d4ddaf9d32af16e3fba9054b2acf95b777e852797e, `run.json` 444e33d116dc22b9a2ec58c1bbcb912ea058004ac62263f7b82ead287036ad93. This directory is scratch outside the repo; F0 regenerates fixtures rather than depending on it.

**U-01 root cause split.**
- UI (FACT): `RunsViews.jsx:286-295` renders `ScenarioBResult` for any `summary.state==='ready'`, adding only a `Notice` for cancelled/failed/interrupted; `ScenarioBResult.jsx:123-127` renders `Verdicts` and the mechanism sentence unconditionally; `LabScreen.jsx:188` fetches the summary for any terminal state when `summary.json` is indexed. The notice text ("no verdict can be drawn") contradicts the chips beneath it.
- Backend (FACT): A1-A3 above.
- Test masking (FACT): A16.

---

## 3. U-01 correction

### 3.1 (a) UI correction, no backend change (WP-F1 pure, WP-F2 wiring)

**Principle.** A verdict may be shown only when the run's state is `completed` AND the summary agrees (`summary.status === "COMPLETE"`). Everything else is "No verdict", and the raw partial data is kept but demoted and labelled. The decision is one pure function so SSR tests and the live page cannot diverge.

**PROPOSAL: `resultPresentation({ kind, state, terminal, summaryState, summaryView, resultEvent, lastProgress })` in `ui/src/lib/labView.js`** returning `{ mode, headline, detail, tone, showRaw, rawLabel, fetchSummary }`:

| Case (Scenario B unless stated) | `mode` | Result-at-a-glance area shows | `fetchSummary` | Raw summary |
|---|---|---|---|---|
| state `queued`/`running`/`cancelling` | `waiting` | existing "The result appears here when the run completes" (RunsViews.jsx:307-312); no chips | false | none |
| `completed`, summary ready, `summary.status==='COMPLETE'` | `verdict` | today's behaviour (verdict chips per arm, mechanism sentence, charts, tables) | true | n/a |
| `completed`, summary ready, `status!=='COMPLETE'` (contradiction; cannot occur with the current backend, defence only) | `no_verdict` | "No verdict: the run is recorded as completed but its summary says <status>." tone warn | true | disclosure |
| `completed`, summary missing (not indexed, 404) | `unavailable` | existing "No result summary for this run: the run completed but the summary is not available" (RunsViews.jsx:299-305); run-class label and reasons from the `result` event if present (`resultEventLabel`), NO verdict chips | false when not indexed | none |
| `completed`, summary fetched but null/not an object/`scenarioBView` returns null | `unavailable` | the existing "summary.json was not in the expected format" error block (LabScreen.jsx:193); no chips | n/a | download link in Artifacts |
| `cancelled`, summary ready (typical) | `no_verdict` | headline **"No verdict: the run cancelled before finishing"**; detail "N of M runs had finished when it stopped. A verdict needs every planned repetition." (N, M from run record `progress`) | true | disclosure |
| `cancelled`, summary missing/failed to load | `no_verdict` | same headline, no disclosure; one line "No partial summary is available." | true (try once) | none |
| `failed` | `no_verdict` | **"No verdict: the run failed before finishing"** + the existing critical Notice with `error.code`/message (RunsViews.jsx:381-383) | **false** (A4: none exists) | none; artifacts table lists what exists |
| `interrupted` (gateway restart; includes recovery events with `source: server_recovery_on_startup`) | `no_verdict` | **"No verdict: the run was interrupted before finishing"** + existing "Interrupted by a gateway restart" Notice (RunsViews.jsx:376-380) | **false** (A5) | artifacts table only; if `summary.json` is listed the user may download it |
| unknown/unreported state | `no_verdict` | "No verdict: the run state is not known to this page" | false | none |
| `t1_matrix`, `completed`, `result` event present | `verdict`-free result | existing cell count + hash glance (`t1Glance`); never a verdict (the API computes none: RunsViews.jsx:271-273) | n/a | n/a |
| `t1_matrix`, `cancelled`/`failed`/`interrupted` | `no_verdict` | "No result: the run <state> before finishing" (replaces "The run ended as cancelled without a result event", RunsViews.jsx:318) | n/a | none |
| `t1_matrix`, non-terminal | `waiting` | existing line (RunsViews.jsx:319) | n/a | none |

Guard (defence in depth inside the component): `ScenarioBResult` takes `mode` (default derived from `view.status !== 'COMPLETE' ? 'no_verdict' : 'verdict'`, so retained evidence and any future caller are covered). `Verdicts`, the "Mechanism check: passed/failed" sentence, `sb-mechanism` fold and the `Valid paired runs / Mechanism check` hint render ONLY when `mode==='verdict'`. This makes the rule hold even if a caller forgets the state.

**Area-by-area behaviour for non-completed runs (Scenario B).**
- *Run header:* unchanged (state chip, tier, elapsed, progress, live chip, Cancel). The `Notice` "Partial result" in `ScenarioBResult` is removed for `no_verdict` runs; its job moves to the no-verdict block (single message, not two).
- *Result at a glance:* replaced by the no-verdict block (headline + detail above). Tone: neutral for cancelled, warn for interrupted, critical for failed (the run-state tones in `labProvenance.js`). No `VerdictChip`, no `LabelChip` as a verdict, no "Mechanism failed", no "H-B1 falsified".
- *Raw summary disclosure* (cancelled only): `<details class="disclosure">` titled **"Partial diagnostic data (not a result)"**, closed by default, containing: per-arm means table (existing `ArmsTable`, cells already render as Unavailable with reason "not recorded: the run ended before this value was produced", `labView.js:303`), the settings fold, and the limitations (never hidden: P5). It does NOT contain verdicts, the mechanism table, the mechanism sentence, paired tables or charts. Reason: those are statistical/decision outputs on a truncated design; paired tables can exist for completed repetitions but their n is below the registered design. The run-class label appears inside the disclosure as "Run class (planned; the run did not finish): EXPLORATORY" with its reasons.
- *Progress:* terminal, so the bar is replaced by a frozen text line "N of M runs finished before it stopped" (units defined in section 6). No ETA. The existing live chip says "Run ended" / last known.
- *Artifacts:* table unchanged; caption gets a sentence "Written before the run stopped; artifacts of a cancelled, failed or interrupted run can be partial or absent." (conditional on state).
- *Event log:* unchanged (closed fold, count). It remains the evidence of what happened (`state: cancelling`, `finished: cancelled`).
- *Run list:* state chip only (list has no verdict column; A17 unaffected).
- *Screen-reader announcement:* the no-verdict block is `role="status"`; the visually hidden run status line (RunsViews.jsx:374) gains "No verdict" for non-completed terminal runs.

**Acceptance criteria (measurable).**
1. For each real fixture (F0): cancelled-before-first-run-finished (case C1), cancelled-after-one-complete-repetition with three arms (C2), interrupted (I1): rendered HTML (SSR via the harness) contains NONE of the strings `Mechanism failed`, `falsified`, `Trade-off failure`, `Benefit`, `Inconclusive` (as a chip), `data-verdict-arm`, `Valid paired runs`, and contains `No verdict: the run cancelled before finishing` / `...was interrupted before finishing` respectively. Assertion list is explicit in the test.
2. Same fixtures: the raw disclosure exists for cancelled, is closed (`<details>` without `open`), is labelled `Partial diagnostic data (not a result)`, and contains `data-arm-row` rows and the limitations list but no `data-verdict-arm`.
3. A completed run (existing `sb_summary_artifact.json` + `run_sb_final.json`) renders exactly as before: snapshot of the set of `data-verdict-arm`, `data-paired-arm`, section ids equals the pre-change set (regression: P9, P5).
4. Defence in depth: `ScenarioBResult` rendered with a summary whose `status` is `CANCELLED` and no state argument renders no verdict (retained-evidence path).
5. `resultPresentation` is table-tested for every row of the table above, including `state` values `queued`, `cancelling`, an unknown string, and `null`.
6. `failed` and `interrupted` runs do not call `artifactJson('summary.json')` (spy on the fake client: zero calls).
7. T1 matrix: no output contains a verdict string for any state.
8. The hand-edited test (`lab.view.test.mjs:282-290`) is REPLACED, not deleted: it loads the real cancelled `summary.json` and asserts `scenarioBView(real).status==='CANCELLED'` plus the presentation outcome. A comment in the test names the fixture hash.
9. Tests that fail before the change (required): AC-1 against fixture C1 fails on the current tree because `Verdicts` is rendered (A1/A2); this is demonstrated by running the new test file against the unmodified `ScenarioBResult.jsx` and recording the failure in the WP-F2 evidence note.

**Fixtures generated from REAL backend output (WP-F0).** Procedure, run by LEAD-FUNC in the implementation phase; nothing is hand-edited:
1. Use the debug gateway already built (`D:\Personal\_m7_target\debug\phantom-cache-demo.exe`; record its SHA-256 and the git HEAD; if source changed since, rebuild only with owner approval). Start it with `PHANTOM_MODE=development PHANTOM_LAB_ENABLED=true PHANTOM_LAB_DIR=D:\Temp\func_lab PHANTOM_BIND_ADDR=127.0.0.1:8386`, new empty lab dir, `TEMP=D:\Temp`.
2. C1: `POST /api/v1/lab/runs` `{kind:"scenario_b", spec:{repetitions:1, arms:["BASE","STATIC-CAP"], warmup_s:3, healthy_s:2, degraded_s:3, recovery_s:2, rate_rps:40, n_keys:100}}`; after the first `healthy` progress event, `POST .../cancel`. (Unlike the existing `run_cancelled.json`, arms include STATIC-CAP, so the defect appears.)
3. C2: same but `repetitions:2`, three arms; cancel during repetition 2 (so repetition 1 completed: arms have unequal run counts; verdict reasons may read "only 1 valid paired runs; 10 required", i.e. a verdict that is still not a result).
4. I1: start a run, then kill the gateway process (not via cancel), restart the gateway on the same `PHANTOM_LAB_DIR`; the recovery rewrites the run to `interrupted`.
5. K1 (completed, three arms, 1 repetition) can reuse `sb_summary_artifact.json`; regenerate it as K2 only if its provenance cannot be recorded.
6. For each run download the SERVED bytes through the API (`GET /runs/{id}`, `GET /runs/{id}/events` replay or `GET .../artifacts/events.jsonl`, `GET .../artifacts/summary.json`, `.../manifest.json`). Do NOT copy files from the lab dir: the API redacts server paths in JSON artifacts and the recorded hash is of the served bytes (`docs/user/lab_api.md:51`).
7. Write `ui/test/fixtures/lab/real/PROVENANCE.json`: for every file: SHA-256, byte size, run id, request that produced it, gateway exe SHA-256, build profile (debug), git HEAD, capture time, "no hand edits" attestation, and the exact cancel timing. A node test recomputes the hashes and fails if a fixture changed.
8. A failed run cannot be produced deterministically (UNVERIFIED how to force `Err` in the runner without a code change). Use a clearly named SYNTHETIC fixture (`failed.synthetic.json`) shaped by `lab_api.rs:1871` (`scenario_failed`) and A4 (no summary); the test name contains "synthetic".

### 3.2 (b) Backend semantic options (WP-F8) — AUTHORISATION REQUIRED, not assigned

Current contract surface (FACT): the Lab API envelope schema is `lab-1`; `summary.json` is an artifact with its own `summary_version: 1` (`scenario_b.rs:708`) and is documented in `docs/demo/scenario_b_artifacts.md:11, 64-66`; the retained-evidence snapshot copies `status`, `mechanism_h_b1_all_pass`, `verdicts` (`lab_api.rs:1383-1397`); the CLI prints `verdict <arm>` for any run whose summary has an object `verdicts` (`src/lab_live/cli.rs:151-155`), including a Ctrl-C run.

| Option | Change | `lab-1` / artifact effect | Retained and old artifacts | Risks |
|---|---|---|---|---|
| **R1: omit** | when `meta.cancelled`, do not emit `verdicts`, `mechanism_h_b1_all_pass` | envelope unchanged; artifact keys vanish | old cancelled summaries still carry verdicts | absence is ambiguous: `verdicts` is also absent/`{}` for BASE-only completed runs, and a consumer cannot tell "not computed" from "nothing to compare" |
| **R2: explicit null (RECOMMENDED)** | when `meta.cancelled`: `"verdicts": null`, `"mechanism_h_b1_all_pass": null`, `mechanism_h_b1.<arm>.pass: null` (the per-arm observed numbers and `violating_reps` stay), plus `"verdicts_status": "not_computed"` and `"verdicts_reason": "the run was cancelled before all planned repetitions finished"`. The existing `status: "CANCELLED"` already plays the role of `run_state` (it is the experiment status; no new `run_state` field is needed). Completed runs unchanged. | envelope unchanged; additive keys; value type of three existing keys changes (object/bool to null) only for cancelled artifacts; bump `summary_version` to 2 only if the owner wants consumers to switch on it (optional) | old cancelled summaries (including the 2 on disk under scratch and any retained) keep old shape: the UI must handle both forever (section 3.3); retained artifacts are not edited (`lab_api.md:61`) | the evidence snapshot code `sj["verdicts"].as_object().unwrap_or_default()` would turn null into `{}` (`lab_api.rs:1383-1386`), losing the signal: F8 must propagate `null` and `verdicts_status` there |
| R3: per-arm "not computed" outcome | add `Outcome::NotComputed` and emit it per arm | changes the enum surface in `analysis.rs` (`as_str` set, M6 consumers) | same | widest blast radius; touches the mechanical rule module reviewed for M6; not recommended |
| R4: fix only the zero-run conflation | `pass: null` (not false) when `runs==0` and let `evaluate_arm` return INCONCLUSIVE | removes "H-B1 falsified" for empty arms | | still emits a verdict for a cancelled run (C2: "Inconclusive" on partial data); necessary as part of R2, insufficient alone |

**Recommendation: R2**, with R4's zero-run fix included (a cancelled run has no verdicts at all, and independently a completed-run mechanism flag must never be `false` for "zero runs"). Reasoning: explicit null is self-describing; keeps `status` as the single state marker; additive for consumers that switch on `verdicts_status`; the CLI loop at `cli.rs:151` degrades gracefully (`as_object()` is `None`, nothing printed; FACT) which is also a correctness improvement for Ctrl-C runs.

**Files and tests (when authorised).**
- `src/lab_live/scenario_b.rs`: in `summarize()` branch on `meta.cancelled`; keep `h_b1` computation for completed runs; set `mech_pass` to `None` (JSON null) for `rs.is_empty()`.
- `src/api/lab_api.rs` retained snapshot (`:1383-1397`): pass through `null`/`verdicts_status`.
- `src/lab_live/cli.rs`: no change needed (verify by test).
- New/updated tests in `tests/integration/lab_live_scenario_b_tests.rs`: extend `e2e_cancellation_terminates_children` (`:406-430`) with `assert!(o.summary["verdicts"].is_null())`, `assert!(o.summary["mechanism_h_b1_all_pass"].is_null())`, `assert_eq!(o.summary["verdicts_status"], "not_computed")`; a pure unit test calling `summarize()` with `cancelled=true`, zero runs, and with one completed repetition (C2 shape) asserting no `outcome` string anywhere in the serialised summary; a completed-run test asserting `verdicts` is still an object with `outcome` strings (guards against over-suppression; existing `:445`/`heavy_*` already assert this for the ignored heavy test, so add a non-ignored one); API level in `tests/integration/lab_api/mod.rs` (near `:855-872`): after a cancelled run, `GET .../artifacts/summary.json` has `verdicts == null`. Every new assertion must be shown to FAIL on the unchanged `src/` first (CLAUDE.md section 5 rule for changes touching invalidation applies by analogy; here it is the owner's "test that fails without the change" standard).
- Docs: `docs/user/lab_api.md` (a paragraph under the run states: "A cancelled Scenario B run's `summary.json` has `verdicts: null`..."), `docs/demo/scenario_b_artifacts.md:64-66`.
- Independent review needed: YES (changes what an evidence artifact asserts; touches the verdict path; reviewer not the writer; must read `analysis.rs:117-137` and confirm completed-run output is byte-identical for a fixed-seed fixture apart from nothing; i.e. add a golden comparison of a completed `summarize()` output before and after).
- Build/test commands when authorised: `cargo test --all-targets -j 1`, `cargo clippy --all-targets -j 1 -- -D warnings` (CLAUDE.md section 3); baseline there is 242 tests/19 targets (reported, not re-run here).
- Not changing: retained experiment directories, M6 evidence.

### 3.3 (c) Interplay: the UI stays correct after the backend change

The UI rule is keyed on run state first and summary status second, never on the presence or shape of `verdicts`. Consequences: (1) legacy cancelled artifacts (old shape, verdicts present) -> no verdict shown (rule), (2) new cancelled artifacts (`verdicts:null`) -> `scenarioBView` must tolerate `null` (it already does: `isObj(s.verdicts) ? s.verdicts : {}`, `labView.js:325`; add the test), and must not interpret "no verdicts" as the "No arm verdict is recorded ... needs a BASE arm" message for a cancelled run (that message is only for `mode==='verdict'`), (3) a future backend that adds `verdicts_status` is read as an extra guard: if `verdicts_status==='not_computed'` then `no_verdict` regardless of state (so even `completed` with not_computed is safe). The UI correction therefore ships first and independently; the backend change later does not require UI changes except the optional extra read of `verdicts_status` (one line + test, bundled in F8's UI follow-up).

---

## 4. Connection states

### 4.1 What the code can actually distinguish

| Channel | Observations available | Cannot distinguish | Source |
|---|---|---|---|
| Telemetry stream (`EventSource`) | `onopen`; frames (parse OK / violations / unreadable); `onerror` with `readyState` 0 (browser will retry) or 2 (closed); frame age | 401 vs 403 vs network vs CORS vs proxy (no status exposed); the browser's own retry schedule | A9 |
| Capabilities `GET` (fetch) | network failure; 404 without a lab-1 body (Lab off OR not a PhantomCache gateway); 404 lab-1; 401 missing/invalid/expired (via `x-auth-error`); 403 role; 403 `bad_origin`; 421 host guard; 429; 5xx; 200 | network down vs CORS-blocked vs wrong address (all `TypeError`) | `labApi.js:108-133` |
| Run stream (fetch streaming) | connecting, open, silent > 25 s, waiting (attempt n, delay), failed (terminal error), ended | gateway down vs network drop | A11 |

### 4.2 Vocabulary (PROPOSAL; same words on every surface)

| Word | Meaning | Fact kind | Applies to | Fix verb / action |
|---|---|---|---|---|
| **Live** | fresh, contract-clean data | client observation | telemetry; run stream | none |
| **Connecting** | first attempt, nothing received yet | client | all | none (auto) |
| **Reconnecting** | the page lost an established or attempted link and is retrying automatically; attempts 1-2; shows "next try in N s" only when the page owns the timer (section 4.5) | client | telemetry; run stream | **Retry now** |
| **Disconnected** | no answer: attempts >= 3, or the stream closed. The page cannot tell stopped, wrong address, network, or CORS refusal apart (platform limit) | client | all | **Retry now**; "Change address" |
| **Unauthorised** | the gateway answered and refused: 401 (token missing/invalid/expired), 403 role, 403 origin not allowed, 421 host not allowed. The cause is shown in words; each has its own fix | **server fact** (an answer was received) | Lab API; telemetry only when classified by a probe (4.4) | add/replace token; or fix origin/host (cause-specific text from `ERROR_TEXT`) |
| **Unavailable** | the gateway answered and says the feature is off or not provided (Lab API 404, cluster/intelligence disabled); or frames are readable-but-invalid (incompatible version) | **server fact** | Lab API; placeholders; unreadable telemetry as cause "incompatible" | enable on gateway (text); no retry button |
| **Stale** | link believed open, but older than the threshold | client | telemetry; run stream | Retry now |
| **Degraded** | data flows, contract violations withheld | client + server data | telemetry | none |
| **Checking** | capability request in flight | client | Lab strip | none |

Removed/renamed: "Not reachable or not authorised" (A12) splits into Disconnected / Unauthorised / Unavailable. The word "Error" is never a state (it is a cause line). "Telemetry cannot be read" becomes Unavailable with cause "incompatible frames".

**One retry verb:** "Retry now" for re-establishing a connection (replaces "Reconnect now", "Retry" on connection errors; `Gate.jsx:19`, `States.jsx:121`, `ScreenFrame.jsx:79`, `RunsViews.jsx:101`). "Refresh" remains only for re-reading data on a healthy connection (runs list, run detail). (S-11, D-10.)

**Surfaces and exact texts** (PROPOSAL; `linkState.js` is the only source of these strings; components receive `{word, cause, who, fix}`):

| Surface | Shows | Example (disconnected) |
|---|---|---|
| Header indicator (button, opens popover) | glyph + word + host:port | "Disconnected · localhost:8085" |
| Popover | one row per channel (Live telemetry; Lab API; and a run stream row when on a run page), each with word, cause, who knows ("seen by this page" or "answered by the gateway"), last change time, Retry now, address field + Test (section 5) | "Live telemetry: Disconnected. This page got no answer for 12 s (seen by this page)." |
| Lab status strip | chip with the same word; no merged wording | "Lab API: Unauthorised (token required)" |
| Page body (ScreenFrame/Gate) | ONE block with the word as title, cause, `Retry now`, `Change address` | "Disconnected from http://localhost:8085" |
| Run stream chip/banner | same words; "Reconnecting, next try in N s" | "Live updates: Reconnecting" |

Each page shows the channel the page depends on; the header shows telemetry (it has always been the header's meaning) and escalates to "Lab API: <word>" only while the Lab route is mounted and the Lab channel is the worse of the two (severity: Disconnected > Unauthorised > Reconnecting > Unavailable > Stale > Degraded > Live). Off the Lab route the Lab channel is only probed on demand (popover open or Test), so no extra request is made for people not using the Lab.

### 4.3 Timing reconciliation

FACT: telemetry stale after 3 s (`connection.js:4`; server frame every 100 ms, `server.rs:957`: 30 missed frames); run stream stale after 25 s (`labStream.js:14`; heartbeat 15 s). Different cadences, so different thresholds are correct. PROPOSAL: keep both numbers; make each chip/popover state its basis ("no frame for 4 s; frames are sent every 0.1 s" / "silent for 31 s; the gateway sends a heartbeat every 15 s"); never show the same word with different meanings. Align the escalation instead: **Reconnecting for attempts 1-2, Disconnected from attempt 3 in both** (today only the run stream does this: `labStream.js:93`; telemetry goes straight to Disconnected, S-03). Telemetry `attempt` already exists in the reducer (`connection.js:27`); the derivation changes in `deriveConnection`.

### 4.4 Telemetry under JWT (BD-05), what UI-only can do

FACT: with JWT on, `/api/events` answers 401 before streaming (A10); `EventSource` cannot send headers and exposes no status; so the header chip shows Disconnected (closed -> our backoff loop retries forever) while the Lab works with a token (R5 B13, G-15; inference confirmed by A9/A10, not yet observed in a browser: UNVERIFIED).

| Mitigation | What it does | Backend change | Token handling | Status |
|---|---|---|---|---|
| **M1 honest copy** | When telemetry never produced a frame and the Lab channel reports `security_mode: jwt` (or any 401 on a probe), the telemetry state is shown as **Unauthorised** with the cause "live telemetry needs an Authorization header; this page's stream cannot send one yet (BD-05)". Tone info/warn, not critical, because the Lab works. | none | none | PROPOSAL, include in F3/F5 |
| **M2 classifying probe** | When the stream is down (never any frame, or attempts >= 3) do ONE `GET /api/v1/metrics` via `labApi.probe()` (fetch, Authorization if a token exists). 401/403 -> Unauthorised (server fact); network error -> Disconnected; 200 -> "stream blocked, endpoint reachable" (e.g. proxy buffering or the 16-stream limit; cause stated as unknown); then repeat at most every 30 s. | none (`metrics` is auth-gated the same way per `sse_auth_options.md:7`; UNVERIFIED for the exact status codes) | uses in-memory token only | PROPOSAL; probe must live in `labApi.js` (A15) |
| **M3 fetch-streaming telemetry (BD-05 option D)** | Reuse `sse.js` + the `labStream` runner pattern against `/api/events?schema=2` with the Authorization header; telemetry works on a protected gateway, no backend change. CORS: `authorization` is already allowed (A10); preflight for `/api/events` itself UNVERIFIED. | none | needs the token at App level (D-11) | OWNER DECISION D-3; larger change; replaces EventSource semantics (own reconnect; gives a countdown for free) |
| **M4 polling fallback** | Poll `GET /api/v1/metrics` at 1 Hz when the stream is Unauthorised and a token is set (same schema-2 payload per `scenario_b.rs:719`) | none | App-level token | OWNER DECISION; lower fidelity than 100 ms frames; UNVERIFIED payload equality |
| Backend A (query token), B (cookie), C (stream ticket) | per `sse_auth_options.md` | **AUTHORISATION REQUIRED**, not assigned; A not recommended | | NOT assigned |

Recommendation: ship M1 + M2 now (honest, no new credential handling beyond what the Lab already does); decide M3 as the documented interim ("Now / no backend change", `sse_auth_options.md:52-54`); do not touch A/B/C.

### 4.5 Retry ownership (needed for "next try in N s")

FACT: the page owns the timer only when `readyState===2`; the browser's `EventSource` retries silently otherwise (A9), so a countdown cannot be shown honestly in the common case. PROPOSAL (D-7): in `useTelemetry` close the `EventSource` on every `onerror` and schedule our own reconnect with `backoffMs(attempt)` (cap lowered to 10 s for telemetry via a parameter; the Lab stream keeps 30 s). This gives `attempts`, `nextRetryAt`, `lastError` (DD 9.3 item 3) and the same behaviour as the run stream. Risk: recovery after a long outage can lag up to the cap; mitigated by the always-visible Retry now. Test: fake EventSource + fake clock; the reducer gains `retryScheduled{at}`.

### 4.6 Acceptance criteria and tests (WP-F3, F5)

1. `linkState.js` pure function `describeLink({ telemetry, caps, runStream, security })` is table-tested for every cell of 4.1 (>= 20 cases: each Lab error kind in `ERROR_TEXT`, each telemetry kind x attempt 0/1/2/3, each run-stream status), asserting `word`, `who`, `fix`.
2. A source test fails if any of the old strings remain outside `linkState.js`/tests: "Not reachable or not authorised", "Reconnect now", "No connection to the gateway", "Could not reach the gateway" (the latter replaced by the cause line).
3. Same condition, same word on all surfaces: SSR test renders Header, Lab strip, ScreenFrame block and run-stream chip for the same fixture state and asserts the word set is identical.
4. `lab.render.test.mjs:144` is updated: 401 missing -> "Unauthorised"; 502 -> "Disconnected" or "Unavailable (gateway error)" (decide per kind table); this is an intentional test change recorded in the commit message.
5. Telemetry: after `error` x1/x2 with prior payload -> `reconnecting` with data flagged stale; x3 -> `disconnected`; first `message` -> `live` and attempt 0; `reset` action clears payload (needed by section 5).
6. M2: with a fake client returning 401 -> state Unauthorised and no further probes within 30 s (fake clock); 200 -> "stream blocked" cause; network -> Disconnected.
7. Browser: kill and restart the gateway: chip goes Reconnecting (no countdown if browser-owned, countdown if D-7 accepted) then Disconnected after 3 attempts, then Live with the live region announcing "Live" (`App.jsx:136-138`); JWT gateway without a token: chip says Unauthorised with the BD-05 cause (M1), Lab strip says "Token required".

---

## 5. Gateway address

### 5.1 Behaviour specification (PROPOSAL; D-4, D-5)

**Resolution order** (first valid wins; every rejected candidate produces a visible notice): (1) `?gateway=` (this page load only; never persisted unless the user presses "Use and remember"), (2) saved setting (`localStorage` key `phantomcache.gateway.v1`, value `{"url":"<origin>"}` only), (3) `VITE_PHANTOM_GATEWAY` build value, (4) built-in `http://localhost:8085`. Today's order (query, env, default) is a subset (A13). The effective record is `{url, source: 'query'|'saved'|'env'|'default', notes[]}`.

**Validation** (pure `validateGatewayInput(text, {pageOrigin, pageProtocol})` returning `{ok, url?, errors[], notices[]}`):

| Rule | Result |
|---|---|
| empty/whitespace | error "Enter the gateway address, for example http://localhost:8085" |
| missing scheme (`host:port`) | error with the suggested corrected value; never auto-guess silently |
| scheme not http/https | error "Only http and https are allowed" (existing text kept) |
| credentials in URL (`user:pass@`) | error "Do not put a user name or password in the address; paste a token in Details" (new: `config.js` currently drops them silently via `u.origin`) |
| path/query/hash present | accepted, notice "Only the origin <origin> is used; the path was removed" |
| invalid port/host | error "not a valid URL" |
| loopback host (`localhost`, `127.0.0.1`, `[::1]`) | notice "This computer. Without JWT the Lab API only accepts these host names." |
| other host, gateway without JWT | notice (shown after Test, when `security_mode` is known): "Without JWT the gateway refuses non-loopback Host headers (HTTP 421); use a loopback name or enable JWT" (FACT `docs/user/lab_api.md:21`) |
| page served over `https:` and gateway `http:` non-loopback | warning "Browsers block this mixed content" (INFERENCE, platform behaviour) |
| gateway origin differs from page origin | notice naming the page origin: "The gateway's PHANTOM_CORS_ORIGINS must include <page origin>" (replaces the generic `ERROR_TEXT.network.fix`) |

**Test connection** (popover button; does not switch the address): one `capabilities` request and one `metrics` probe through `labApi.js` to the CANDIDATE base; shows per probe the class (network / 401 / 403 role / 403 origin / 421 / 404 / 5xx / ok) and which side knows it ("answered by the gateway" vs "no answer"). No invented cause: network failure is reported as "no answer (stopped, wrong address, or blocked by CORS: this page cannot tell)". 404 without lab-1 body is reported as "answered 404: the Lab API is off, or this is not a PhantomCache gateway".

**Apply** ("Use this address"; optional checkbox "Remember on this browser", default on, D-4): 
1. validate; persist only `{url}` inside `try/catch` (`getStorage` pattern, `theme.js:36-42`), failure is non-fatal and says "not remembered";
2. `useTelemetry(gateway.url)` effect restarts (deps already include the URL: `useTelemetry.js:72`) AND a new reducer action `reset` clears `payload`, `receivedAt`, `violations`, attempts so the old gateway's values never appear under the new address (today they would persist, FACT: the reducer has no reset); `useHistory` series are cleared when the payload goes `null` (add the reset: `useHistory.js` keeps old points);
3. `LabProvider` rebuilds the client (A14) and **clears the token** (`tokenRef.current = null`, roles null, version++) because a credential is bound to the origin it was entered for; a notice says "Token forgotten because the address changed";
4. open Lab streams abort through their effect cleanup (client identity changes; `useRunStream.js:51-56`), and `endedRef` does not matter because the component remounts on route change (see 5);
5. routes carrying an id of the old gateway (`#/lab/runs/<id>`, `#/lab/evidence/<id>`) navigate to the list of the same section (run ids do not exist on the new gateway);
6. address change is disabled while `startRun`/`cancelRun` is pending (those POSTs are not abortable; prevents a late navigation to a run id of the old gateway); the control shows why.

**Visibility.** Header indicator shows host:port next to the word (desktop; DD 7.1), phone shows dot + word and the address row at the top of the Menu sheet and in the popover (U-02: "address hidden on phone"). Lab strip: an always-visible first chip `Gateway: localhost:8085` (today it is in Details only, `ConnectionPanel.jsx:97`). Disconnected/Unauthorised blocks name the address once and carry "Change address". Source is shown in the popover ("set by the page link", "saved in this browser", "build default", "built-in default").

**Invalid or hostile input.** Invalid `?gateway=`/saved/env values: a persistent one-line notice above the page content on EVERY route (not only Overview: B02/A13) "Ignored the address from the page link (not a valid URL). Using http://localhost:8085. Change address". PROPOSAL (D-5, security-relevant): when the effective gateway came from `?gateway=` and is not loopback and differs from the saved/default, show a warning in the popover and the Lab strip "This address came from the page link. Tokens you enter are sent there." and require one confirmation before the first "Use token" for that origin (INFERENCE: today a crafted link could point the UI at another host and the user would type a token into it; this is the existing behaviour, not introduced here).

**Misleading text fix.** `ERROR_TEXT.network.fix` (`labApi.js:73`) becomes: "Check that the gateway is running at <address> (shown in the connection popover and Details). If this page and the gateway have different origins, the gateway PHANTOM_CORS_ORIGINS must include <page origin>." `ErrorBlock` (Gate.jsx) interpolates the two values and adds `Change address`. The static text lives in `labApi.js` (tests assert it: `lab.api.test.mjs`).

### 5.2 Acceptance criteria and tests (WP-F4, F5)

1. `validateGatewayInput`: >= 15 table cases covering each row above, including `http://localhost:8085/x?y#z`, `localhost:8085`, `http://u:p@host`, `ftp://x`, `http://[::1]:8085`, `HTTP://LOCALHOST:8085/` (origin normalised).
2. Resolution: all four sources and their precedence; invalid query with valid saved -> saved used and a notice produced; corrupt saved JSON -> ignored with a notice; storage throwing -> still resolves (`misc.test.mjs:57` pattern).
3. Persistence content test: after Apply, the storage mock holds exactly one key with value `{"url":"http://..."}`; after setting a token and a token-looking string anywhere, no storage write contains it; a forbidden-pattern/guard test asserts `gatewayStore.js` is the only non-theme module touching storage.
4. Token rule: with a fake client factory, set token on gateway A, apply gateway B, then perform any request: the `Authorization` header is absent (fails on the current provider because `tokenRef` survives).
5. Reset rule: after Apply, `state.payload===null` and history series empty before the first frame from B.
6. SSR: header shows host:port; phone menu shows the address; Lab strip chip present without opening Details; Overview and Lab both show the invalid-address notice.
7. Test connection: fake fetch classes map to the table; no request carries a token to the candidate unless the user opted in (default: the existing in-memory token is NOT sent to a candidate other than the current origin).
8. Browser: from `http://localhost:3300/` with gateway on `127.0.0.1:8386`: type the address, Test (reachable), Apply: telemetry goes Live; reload keeps it (saved); `?gateway=` link overrides for that load and says so.

### 5.3 Guard impact (A15)

Persistence code lives in `lib/gatewayStore.js` and `hooks/useGateway.js`, outside the lab file set, so `lab.guard.test.mjs:30-37` (no storage in lab files) stays untouched. The probe lives in `lib/labApi.js` (the only module allowed to call `fetch`). Two guard edits are required and need REVIEWER sign-off, each a deliberate loosening recorded in the commit: (i) `lab.guard.test.mjs` provider assertion `tokenRef = useRef(null)` in `LabScreen.jsx` moves with the token if D-11 is accepted; (ii) the `all`-files storage check, if any, whitelists exactly `lib/gatewayStore.js` and `lib/theme.js`. No guard is weakened for lab files.

---

## 6. Progress and recovery semantics

### 6.1 What events support (A7, A8)

| Item shown | Source | Provenance label | Availability |
|---|---|---|---|
| Unit definition | "1 run = one arm x one repetition = one fresh gateway process" (spec `repetitions` x `arms`) | n/a (definition) | always for Scenario B |
| Runs finished / total | `progress.done`, `progress.total` | REAL (gateway-reported) | after first `progress` event or from `run.json.progress` |
| Current run | `done + 1` of `total`; `arm`; `rep` of `spec.repetitions` | REAL | while running |
| Current phase | `phase`; position k of 4 for warmup/healthy/degraded/recovery; `starting` -> "starting the gateway"; `drain`/`collect` -> "collecting results"; `run-complete` -> "run finished" | REAL | while running |
| Started / ended | `run.json` `started_utc` / `ended_utc` | REAL | when set |
| Elapsed | gateway `started_utc` and browser clock (active) or `ended_utc` (terminal) | DERIVED (existing `elapsedView`, `labView.js:464`) | existing |
| Phase lengths | `run.spec` echo (`warmup_s`..`recovery_s`, defaults filled in by the gateway after start) | REAL | after start |
| Planned minimum duration | `total x (warmup+healthy+degraded+recovery)` seconds | DERIVED, qualifier "minimum by plan; start-up, drain and shut-down add time the page cannot measure" | when the four phase lengths are known (always on the run page; in the form only when all four fields are filled or F8b adds defaults) |
| Estimated remaining | `max(0, plannedMinimum - elapsed)` labelled "at least ... left by plan" ; if elapsed > plannedMinimum: "taking longer than the planned minimum" | DERIVED | Scenario B, while `running`, only when phase lengths known and `started_utc` known |
| Time since last event | browser clock minus time the page received the last event | INFERRED (client clock) | while running |
| Per-phase timestamps | `events.jsonl` `ts` (A8), 1 s resolution | REAL | only via downloading the artifact; not used for live display |
| ETA for T1 matrix | none: cell cost varies | **UNAVAILABLE** (reason shown) | never |
| Server-side elapsed, per-phase progress within a phase | not provided | **UNAVAILABLE** | never |

Overhead per run is not derivable: one observation (usability T2: quick preset 2 x 4 s planned, 11 s observed) suggests about 1.5 s per run but this is a single debug-build sample (UNVERIFIED); therefore only a lower bound is ever shown.

**Progress bar rule (D-9).** Keep `<progress value=done max=total>` strictly on gateway-reported counts. Fix U-03's "looks stalled" with: the unit definition sentence; "Run 1 of 2 in progress: BASE, repetition 1 of 1"; a four-step phase indicator (current phase highlighted, text "Phase 2 of 4: healthy") and the "at least X left by plan" line. Weighted bars are not used (they would blend measured counts with planned lengths). T1 matrix: "cells" instead of "runs" (`lab_api.rs:1807`: `done`/`total` are cells).

### 6.2 Run form defaults and presets (U-04)

FACT: the form opens with repetitions default 10 (`capabilities.sb.limits.repetitions.default`, fixture), all three arms, blank phase fields (gateway defaults 8/5/10/8 s: `scenario_b.rs:94`, but `capabilities` carries only min/max for phases, so the page cannot show the default total without hardcoding: F8b adds `phase_seconds.default` or the estimate is shown only when fields are filled). Presets have no selected state (`RunForm.jsx:116-126`: no `aria-pressed`).
PROPOSAL (D-8): open with **Quick check applied and shown as selected**; second preset "All three arms"; "Custom" state appears when any field diverges from the selected preset. Selected = patch values equal the form (pure function `activePreset(form, presets)`), exposed with `aria-pressed` plus a visible marker (not colour alone). Duration line under presets: "Planned minimum: 8 s (2 runs x 4 s)" for Quick; for "All three arms" with blank phases: "Planned minimum: unavailable here (the gateway fills the phase lengths; they are shown on the run page)" unless F8b. The debug-build EXPLORATORY warning is rendered adjacent to the Start button (U-04, U-10), not below the sticky bar (visual placement belongs to DS-IMPL; the functional requirement is that the warning element is a child of the submit group).

### 6.3 Recovery behaviours

| Situation | Behaviour (PROPOSAL, FACT where cited) |
|---|---|
| Reload mid-run | Route `#/lab/runs/<id>` reloads the record and the stream starts at event 1 and replays (`useRunStream`, `lastId` null; A11); the run continued on the gateway (worker thread, A6). On a JWT gateway the token is lost on reload (P8): the page shows Unauthorised with "The run is still running on the gateway. Paste a token to watch it." (new sentence in `ErrorBlock` for `unauthorized_missing` on a run route). |
| Gateway restarted mid-run | Stream drops -> Reconnecting -> (>= 3 failures) Disconnected with chip "(last known; stream disconnected, last event N)" (existing `staleSuffix`, `RunsViews.jsx:334-336`); when the gateway returns the stream resumes with `Last-Event-ID` and receives the recovery `state: interrupted`/`done` events (A5) -> Interrupted notice and no-verdict block. While disconnected the page says "The run may have finished, been interrupted, or still be running; this page cannot know until the gateway answers." |
| Stream failed (terminal error) | existing "Stream cannot continue" with Retry now (renamed). |
| Cancel race | `cancelling -> completed` is possible (`lab_api.md:65`, `known limits`): when `cancel.phase==='sent'` and the final state is `completed`, show "The run finished before the cancel took effect; this is a normal completed run." (closes G-14). |
| Retry actions | Run page: Retry now (stream), Refresh (record). Lab strip: Retry now (capabilities). Telemetry: Retry now. All map to one `retry()` per channel. |
| Start disabled when unreachable | When capabilities is in `Disconnected`, `Unauthorised` or `Unavailable`, the head "Start a run" (`LabScreen.jsx:281`) and the "Start a run" tab render as an `aria-disabled` control with reason text ("Cannot start: the gateway is unreachable. Retry now / Change address") and the run form body is not shown (it is already behind `ResourceGate`, so only the head action changes). Closes U-14, U-12 (the duplicate-control decision belongs to DS-IMPL). For Operator-less tokens the form's existing messages stay. |
| Run history pagination | `RunsPage` passes `limit: 50` and `offset` state; `RunsListBody` gets "Showing 1-50 of N indexed (newest 200 only)", Previous/Next as buttons (not links), offset reset on gateway change; labApi already validates integers (`labApi.js:135-149`). The 400 error for bad limit is mapped (`errors.json limit_400`). |

### 6.4 Acceptance criteria and tests (WP-F6, F7)

1. `progressView(run, liveModel, spec)` table tests: queued, first progress (`done:0,total:2`, phase starting), mid-run, last run, terminal cancelled, T1 (cells), missing `total`, missing spec phases (estimate -> unavailable with reason), elapsed > planned minimum, negative elapsed (clock skew -> unavailable). No branch returns a number for ETA when inputs are missing.
2. Rendered text never contains "0 of 2 units"; contains the unit definition once and "Run N of M".
3. Real fixture C1 events replayed through `applyRunEvent`: progress goes `done:0` for the whole sequence (matches real behaviour) and the page shows phase steps changing; SSR of three checkpoints.
4. Presets: `activePreset` equals 'quick' on open, 'custom' after editing a phase field, 'full' after choosing it; `aria-pressed` correct; duration text for Quick equals 8 s from the fixture limits.
5. Start gating: SSR of LabInner with caps `network`/`unauthorized`/`disabled` errors: head action is `aria-disabled` with reason; with healthy caps unchanged (P3).
6. Pagination: fake client records `limit=50&offset=0`, then `offset=50`; 400 mapped; Next disabled when `offset+returned>=indexed`.
7. Cancel-race: fixture sequence (cancelling, then completed) renders the explanation.
8. Browser (F10): live Scenario B quick run: progress text/phase steps visible within 2 s of start and never "units"; reload at 50% re-attaches; kill gateway at 50%: Disconnected after about 7 s (1+2+4 s backoff), restart: Interrupted with no-verdict block.

---

## 7. Work packages (ordered)

Legend: **Pre-redesign** = must land before DS-IMPL renders states (pure modules, no visual churn). **Parallel** = can proceed alongside I1..I4 (touches no DS-IMPL file). **Post** = wiring into DS-IMPL-owned files.

| Order | WP | Scope | Depends on | Phase |
|---|---|---|---|---|
| 1 | **F0** | Capture C1, C2, I1, K fixtures from the real debug gateway; provenance + hash test | G0, owner approval to run the local gateway (D-2) | Pre-redesign, parallel |
| 2 | **F1** | `resultPresentation`, `scenarioBView` additions (`isFinal`, `rawStatus`), real-fixture tests, replacement of the masking test | F0 | Pre-redesign, parallel |
| 3 | **F3** | `connection.js` reset/attempt/`reconnecting`/`retryScheduled`, `linkState.js`, `useTelemetry` own retry, tests | G0 | Pre-redesign, parallel |
| 4 | **F4** | `config.js` resolution + validation, `gatewayStore.js`, `useGateway.js`, `labApi.probe`, tests, guard edits | G0 | Pre-redesign, parallel |
| 5 | **F2** | wire U-01 into `RunsViews`, `ScenarioBResult`, `LabScreen.RunDetailPage` | F1; DS-IMPL release of the 3 screens | Post (or inside I2) |
| 6 | **F5** | header indicator + popover, Lab strip, Gate/ScreenFrame/States wording, invalid-address notice, token clear, `LabProvider` baseUrl | F3, F4; DS-IMPL release of Shell/App/States (I4) | Post (inside I4 ideally) |
| 7 | **F6** | progress view, presets, duration | F1, F2 | Post |
| 8 | **F7** | start gating, pagination, recovery copy, cancel-race text | F5, F6 | Post |
| 9 | **F10** | browser validation per WP exit; token-leak inspection; mobile 390/320; light/dark | each WP | continuous |
| gated | **F8** | backend R2 | owner authorisation D-1 | independent; UI unaffected |
| gated | **F8b** | capabilities phase defaults | D-14 | optional |
| gated | **F9** | telemetry under JWT: M3/M4 or backend | D-3 | independent |

Per-WP detail (scope, exit criteria and risks are in sections 3-6; summary here):

| WP | Tests to add (fixture provenance) | Regression risks (R5 preserve list) | Browser validation steps (F10) |
|---|---|---|---|
| F0 | `fixtures.provenance.test.mjs`: recompute SHA-256 of every `real/**` file against `PROVENANCE.json`; assert no file mentions a server path (redaction) | P7 (no seed values must appear in captured specs: capture only T/D-free Scenario B, no seeds exist for it) | none |
| F1 | `lab.result.test.mjs` (table + real C1/C2/I1/K), updated `lab.view.test.mjs`; fixtures real | P5 (limitations never hidden), P9, P11 | none |
| F2 | SSR tests on C1/C2/I1/K via harness; fail-before evidence | P3, P5, P9, P10 (charts hidden for no-verdict: tables stay), P11 | real Scenario B quick run, cancel after 2 s: no verdict; completed run unchanged |
| F3 | `linkState.test.mjs`, extended `connection.test.mjs`, `lab.stream.test.mjs` | P6 (honesty banners), P13 (no new animation) | kill/restart gateway; JWT gateway without token |
| F4 | `gateway.test.mjs`, guard tests | **P8 (token rules)**, P1 (`?gateway=` deep links keep working) | address change flow, reload persistence, `?gateway=` override |
| F5 | updates to `shell.test.mjs`, `render.test.mjs`, `lab.render.test.mjs`; popover keyboard/focus test (Escape returns focus: P2) | P1, P2, P6, P12 (44 px targets), P14 (harness imports `ConnectionPanel` by name), P15 (footers) | 390/820/1280, light/dark, keyboard path, token never in URL/storage |
| F6 | `lab.progress.test.mjs`, `lab.forms.test.mjs` updates | P7 (presets must not expose sealed seeds), P3 | quick run, long preset estimate |
| F7 | `lab.recovery.test.mjs` | P3, P12 | reload mid-run, gateway restart mid-run, pagination with > 50 runs (create runs via the quick preset or use the fixture lab dir) |
| F8 | see 3.2 | n/a (backend) | re-run C1 live: new fixture C1' proves `verdicts:null` and that the UI output is unchanged |

All WPs: `npm run check` equivalent (`check:forbidden`, `check:contrast`, `check:a11y`, `node --test test`) must stay green; production `vite build` is out of scope for this planning pass but required at implementation exit. Baseline 298 passing (REPRODUCED) is the floor; every intentional test change is listed in the WP's commit note.

---

## 8. Sequencing against the visual redesign

- **Must precede the redesign rendering states:** F1 (the set of result modes: verdict / no_verdict / unavailable / waiting) and F3 (the connection vocabulary and its states, including Reconnecting, Unauthorised, Unavailable) so DS-IMPL designs the real set of states instead of the current merged ones. Deliver these as pure modules plus a one-page state catalogue extracted from sections 3.1 and 4.2 (no extra file: this document is the catalogue).
- **Parallel with the redesign:** F0, F1, F3, F4 (only `lib/`, `hooks/`, tests, fixtures, one scripts file).
- **After/inside the redesign increments:** F2 (I2 screens), F5 (I4 shell, `App.jsx`, `States.jsx`), F6/F7 (I2 again). INFERENCE from `00_progress.md`: I2 is conditionally accepted and I4 is gated on I2; if I4 has not started when G0 is granted, the preferred route is that the I4 writer implements F5's wiring from the contract in sections 4-5 with LEAD-FUNC reviewing; otherwise LEAD-FUNC applies F5 after I4's release. Either way no two writers touch `Shell.jsx` or `App.jsx` concurrently.
- **Contract hand-off to DS-IMPL for wiring:** components receive view objects, never raw errors: `LinkView {word, tone, icon, cause, who, retryLabel, retry, changeAddress}`, `ResultMode {mode, headline, detail, tone, showRaw, rawLabel}`, `ProgressView {unit, run:{index,total,arm,rep,repTotal}, phase:{name,step,steps}, plannedMinimum, atLeastLeft, sinceLastEvent}`. Tones reuse existing tokens (`ok|info|warn|critical|neutral`).

---

## 9. Decisions required from the owner (each with a recommendation)

| ID | Decision | Recommendation |
|---|---|---|
| D-1 | Authorise backend R2 (WP-F8: `verdicts:null` + reason for cancelled Scenario B summaries, zero-run `pass:null`) | Yes, after F1/F2 are in; independent review mandatory; retained artifacts untouched |
| D-2 | Permit running the already-built debug gateway on loopback (port 8386, `PHANTOM_LAB_DIR` under `D:\Temp`) to capture real fixtures; no rebuild unless HEAD changed | Yes |
| D-3 | Telemetry under JWT: M1+M2 now; M3 (fetch-streaming, BD-05 option D) yes/no; backend A/B/C | M1+M2 now; approve M3 as the interim; reject A; defer B/C (E-02) |
| D-4 | Persist the non-secret gateway address in `localStorage` (token never) | Yes, with a "Remember on this browser" checkbox default on and a reset-to-default action |
| D-5 | Treat `?gateway=` as untrusted for token entry (warning + one confirmation before the first token to a non-loopback origin that came from the link) | Yes |
| D-6 | Header indicator shows host:port (DD 7.1) | Yes on desktop; phone shows address in Menu and popover |
| D-7 | `useTelemetry` owns reconnect (close on error, own backoff, 10 s cap) to get attempts/countdown | Yes |
| D-8 | Run form opens with Quick check applied and selected | Yes; the full 10 x 3 run remains one click away |
| D-9 | Progress bar strictly gateway counts + phase steps + "at least X left by plan" (no weighted bar) | Yes |
| D-10 | One verb "Retry now" for connections; "Refresh" only for data | Yes |
| D-11 | Lift the (memory-only) token ref to App level so header Test connection, probes and M3 work | Yes only if M3 or header Test under JWT is wanted; otherwise Test uses the Lab-scoped token and is available on the Lab route only. Requires editing the provider guard test |
| D-12 | Whether connection loss is announced assertively (S-17) | Out of scope here; belongs to the accessibility track; keep `role="status"` until decided |
| D-13 | For `interrupted` runs, never auto-fetch `summary.json` (a stale COMPLETE summary may exist after a crash between write and finish) | Yes |
| D-14 | Add `phase_seconds.default` to `capabilities` (F8b) so the form can show the default duration | Optional; otherwise the estimate appears only when phase fields are filled |
| D-15 | Server-side elapsed/ETA/phase timing (G-10, G-11) | Out of scope; stays UNAVAILABLE |

---

## 10. Limits of this plan

- No browser, build or backend was run; the only execution was the existing UI unit suite (298 pass) and read-only file inspection. Every behavioural claim about browsers (EventSource status hiding, CORS vs network indistinguishability, mixed-content blocking) is platform knowledge not re-verified here.
- The defect reproduction (section 2) relies on a scratch artifact produced by another agent; F0 regenerates it before any test depends on it.
- `/api/v1/metrics` status codes under JWT and the CORS preflight for `/api/events` are UNVERIFIED; M2/M3 need a browser check first.
- Estimates are lower bounds by construction; per-run overhead is unmeasured (one debug sample).
- Forcing a real `failed` Scenario B run is UNVERIFIED; its fixture is synthetic and named so.
- Accessibility details (focus order of the popover, live-region politeness), visual treatment and wording tone are DS-IMPL / Track 2 scope; this file fixes states, words, data sources and tests only.
- Nothing here changes provenance rules: no value is introduced that lacks a source label (REAL / DERIVED / INFERRED / UNAVAILABLE), and no estimate is presented as a measurement.
