# Phantom Lab UI (G3)

**Date:** 2026-10-30. **Status:** implemented in `ui/` (route `#/lab`), tested without a browser. **Nothing here has been rendered in a browser**; see section 7. Source of truth for behaviour: `docs/user/lab_api.md` and `docs/design/lab_api_contract.md`. Principles inherited from `docs/design/principles.md` (honesty first, unavailable is not zero, text before graphics) and `docs/design/tokens.md` (tokens, 24/36 px targets, reduced motion).

The Lab screen is a reader and launcher for the real Lab HTTP API. It computes no result. It does not embed results: every number is read from a payload at the moment it is shown. If the API is off, the page says so and shows nothing in its place.

## 1. Screen inventory

Routes are hash routes (no router dependency). `#/lab` replaced the old placeholder.

| Route | View | Purpose | API calls |
|---|---|---|---|
| `#/lab` | Overview (story) | Problem, hypothesis, what the evidence shows, what it does not show, how to read the labels | `GET /capabilities`, `GET /evidence`, `GET /evidence/{id}` for the newest M6 sealed and the newest Scenario B result |
| `#/lab/evidence` | Evidence list | Registered experiments and exploratory runs, verdict chips with equal weight | `GET /evidence` |
| `#/lab/evidence/{id}` | Evidence detail | M6 sealed/dry run (verdicts, effect-size charts with zero line, per-scenario table, falsifiers, limits) or Scenario B (binding notice, trade-off charts, paired differences, mechanism check, limitations) | `GET /evidence/{id}` |
| `#/lab/run` | Start a run | Validated forms for the simulation matrix (T1) and Scenario B (T2); exact RunSpec JSON; equivalent `phantom-lab` command | `GET /capabilities` (limits, catalogue), `POST /runs` |
| `#/lab/runs` | Run history | Newest-first table with state chips | `GET /runs` |
| `#/lab/runs/{id}` | Run detail | Live status over fetch-streaming SSE, progress, cancel with confirmation, result, artifacts with sizes and SHA-256, downloads, settings echoed by the gateway, event log | `GET /runs/{id}`, `GET /runs/{id}/events` (SSE), `GET /runs/{id}/artifacts`, `GET /runs/{id}/artifacts/summary.json` (Scenario B), `GET .../artifacts/{name}` (download), `POST /runs/{id}/cancel` |

Always present on every Lab view: the **Lab status strip** (one compact row of text chips: Lab API state, authentication mode, an active-run link, the debug-build chip "Debug build: results EXPLORATORY", and the chip "Intelligence: not in the serving path"; its **Details** region holds the gateway, schema, availability of each run kind with reasons, the full intelligence statement *"The intelligence engine is not part of the live serving path"*, and the token form, and it **opens by itself only when attention is needed**: an error, API off or unreachable, or a token required and missing) and a sub-navigation (`nav aria-label="Lab sections"`, tabs in workflow order About, Start a run, Runs, Evidence). *Amended 2026-10-30 (increment I2, lead decision (c)); the original requirement, kept here for history, was: "the Connection panel (Lab API enabled or not, authentication mode, availability of each run kind, debug-build label, token input when needed, and the notice "The intelligence engine is not part of the live serving path") and a sub-navigation (`nav aria-label="Lab sections"`).". The honesty content (EXPLORATORY label, intelligence not in the serving path, loopback-only, simulated versus real) is unchanged and stays visible as chips.*

Code map: `ui/src/screens/lab/*` (containers in `LabScreen.jsx`, presentational bodies elsewhere), `ui/src/components/Lab*.jsx`, `ui/src/lib/lab*.js` + `sse.js` (pure), `ui/src/hooks/{LabContext,useLabResource,useRunStream}`, `ui/src/styles/lab.css` (tokens only).

## 2. Connection, authentication and credentials

- Gateway origin: the existing `?gateway=<origin>` / `VITE_PHANTOM_GATEWAY` / default `http://localhost:8085` resolution (`lib/config.js`).
- Anonymous development mode (`capabilities.security_mode = anonymous_dev_loopback_only`): no token field. The gateway checks Host and Origin; the UI must be served from an origin in `PHANTOM_CORS_ORIGINS` (dev profile: `http://localhost:3000`, `http://127.0.0.1:3000`).
- JWT mode: a password-type field. The token is kept in a React `ref` inside `LabProvider`; it is never in a URL, query string, `localStorage`, `sessionStorage`, cookie, log, or the React context value (the context exposes only `hasToken`). It is sent in the `Authorization: Bearer` header of `fetch` calls (`credentials: 'omit'`). Reload forgets it. Changing the token creates a new API client object, which reloads every resource and restarts any open stream with the new credential.
- Live updates use **option D** of `docs/design/sse_auth_options.md`: `fetch` streaming with the `Authorization` header and a `Last-Event-ID` header on resume. `EventSource` is not used by the Lab (a test fails if it appears in Lab code).
- The UI cannot tell the role of a token in advance (the API has no "whoami"). Starting and cancelling are therefore offered whenever the form is valid, and a 403 is shown as "does not have the required role".

## 3. State matrix

Legend: L loading, E empty, X error with retry, U unauthorized (401/403), D disabled (Lab API off), P partial/unavailable fields, S stream states.

| View | L | E | X | U | D | P / other |
|---|---|---|---|---|---|---|
| Connection panel | "Checking" pill | n/a | critical alert (network, server, bad format) | "A token is required" vs "does not have the required role" vs "token has expired" vs "token was not accepted", token field shown | "Lab API off" pill and the sentence `PHANTOM_LAB_ENABLED=true`; Lab sub-views are replaced by a "switched off" panel | debug-build notice lists the gateway's own reasons |
| Overview | state block "Loading the evidence index" | "No retained results to summarise" (no cards) | error block with Retry | same error block, hint to use the connection panel | not reached | if a detail document fails, the card says counts did not load; no counts are invented |
| Evidence list | as above | "No retained experiments found ... This is not a result." | error block | error block | not reached | an entry without a verdict says "No verdict recorded: <reason>" |
| Evidence detail | as above | "No result document in this experiment" (unavailable block) | error block | error block | not reached | M6: sealed note, static research-note limits labelled as static. Scenario B: if the payload lacks the notice text a critical alert says so |
| Start a run | loading | n/a | error block | start errors show as alert (403 role) | not reached | per-field errors with `aria-invalid`; submit disabled with reason; kind unavailable shows the gateway's reason; 409 shows "A run is already active" |
| Runs list | loading | "No runs yet ... This is not a failure." | error block | error block | not reached | states shown: queued, running, cancelling, completed, failed, cancelled, interrupted (glyph + word) |
| Run detail | "Loading the run" | n/a | error block | error block | not reached | see below |

Run detail extras:

- **Stream status banner** (S): `connecting`, `live`, `stale` (no bytes for longer than the heartbeat window, default 25 s against the gateway's 15 s heartbeat), `reconnecting` (backing off, 1 s, 2 s, 4 s ... capped at 30 s, deterministic), `disconnected` (3 or more consecutive failures), `ended` (the `done` event arrived), `failed` (401, 403, 404, 421: retrying cannot help). Each non-live state has a *Reconnect now* button and shows the last event number received and any gap in event numbers. Reconnection resumes with `Last-Event-ID`; replayed duplicates are dropped by event number.
- **Cancel**: a *Cancel run* button opens an inline confirmation (`role="group"`), then a sending state, then "Cancel requested" (also shown when the persisted state is `cancelling`). A cancelled run is flagged partial; summary fields that are `null` are unavailable cells.
- **Interrupted**: explains that the gateway wrote the interrupted marks after a restart and nothing was resumed.
- **Failed**: the gateway's error code and message as an alert; never presented as completed.
- **Result**: Scenario B shows the full result (see 4) fetched from `summary.json` with the Authorization header; the matrix kind shows only what the API reports (cell count and the SHA-256 of `cells.csv`) and states that no statistics are computed for it.
- **Artifacts**: allowlisted names with sizes, SHA-256 (or "Unavailable: live file; not hashed"), what the hash covers, tier, and a *Download* button (fetch with header, then a blob download).

## 4. Charts (all hand-rolled CSS, text first, each with "View as table")

| Chart | Where | What it plots | Why it is not decoration |
|---|---|---|---|
| Origin protection versus client success | Scenario B (evidence and run) | Two aligned bar panels, same arm order: peak origin concurrency (lower = less origin load) and client success rate (higher = fewer failures). Both axes start at zero | The trade-off the verdict records. A lower peak is shown next to the requests it cost |
| Shed requests | Scenario B | Whole-run shed fraction per arm. **Per-phase shed is rendered as an Unavailable panel with the reason** | The API exposes only the whole-run fraction |
| Client latency, all requests | Scenario B | P50, P95, P99 per arm, labelled as including fast-failing 503s. **Success-only latency is an Unavailable panel with the reason** | Prevents reading a shed-induced latency drop as a served-client improvement |
| Effect sizes with 95% intervals | M6 | Per scenario: mean relative change, interval bar, dashed zero line labelled "0 = no change"; adverse rows use a diamond marker and the word "credibly worse" | Shows direction and uncertainty; zero line shows no-effect |

The drawn tracks are `aria-hidden` geometry; the numbers, units and the reading ("supported", "credibly worse", "practically null", "not supported") are real text in the DOM.

## 5. Data contract mapping (field -> source -> provenance)

Tier vocabulary (`lib/labProvenance.js`): `MEASURED_LOOPBACK` -> Measured: loopback; `SIMULATED` -> Simulated; `RETAINED_SIMULATED` / `RETAINED_MEASURED_LOOPBACK` -> Retained: ...; `DERIVED` -> Derived; `UNAVAILABLE` -> Unavailable (also used for unknown tiers). Shape and border style come from the existing five provenance classes; the label carries the precise tier and `data-tier` records it.

| Shown | API field | Badge |
|---|---|---|
| Lab enabled / availability / limits / catalogues | `GET /capabilities` `kinds.*.available`, `reason`, `limits`, `arms`, `scenarios`, `families`, `profiles`, `seed_sets` | none (configuration); reasons verbatim |
| Authentication mode | `security_mode` | none |
| Debug-build label and reasons | `kinds.scenario_b.driver_build_profile`, `label_for_default_experiment`, `label_reasons` | none; text verbatim |
| Evidence kind, times, tier | `items[].kind`, `started_utc`, `ended_utc`, `tier`, `original_tier` | Retained: simulated / Retained: measured loopback |
| M6 hypothesis verdicts | `recorded_result.H1.verdict`, `H2..H5_H6.status` | Derived (rule applied mechanically) |
| M6 "supported in X of N scenarios" | `H1.supported_scenarios_peak_concurrency`, `..._client_p99`; N = length of `per_scenario` | Derived |
| M6 effect means and intervals | `per_scenario[].peak|p99.mean_rel_reduction, ci_lo, ci_hi, adverse, supported, practically_null` | Retained: simulated |
| M6 constraints | `per_scenario[].d_fresh_hit_ratio, d_error_rate, fresh_hit_ok, error_rate_ok` | Retained: simulated |
| M6 falsifiers | `recorded_result.falsifiers.*.fires, note, rule` | Derived |
| M6 seed set / seed-list hash | `provenance.seed_set`, `seed_list_sha256`; `sealed.note` (seed values are never in the payload) | none |
| Scenario B label and meaning | `recorded_result.label`, `label_meaning` (or `interpretation.label_meaning` for older retained summaries), `label_reasons` | none; text verbatim |
| Binding notice | `interpretation.notice_text`, `must_accompany_any_citation`, `notice_applies_to` | verbatim, always expanded, before the charts |
| Scenario B per-arm values | `arms.<arm>.per_run_mean.*`, `client_latency_ms_pooled_scheduled_time.*`, `status_counts_total` | Retained: measured loopback (evidence) / Measured: loopback (run) |
| Paired differences, intervals, p | `paired_vs_base.<arm>.<metric>.{mean_base, mean_treat, mean_diff_treat_minus_base, diff_ci95_bca, relative_reduction_*, permutation_p_sign_flip, n_pairs}` | Derived |
| Arm verdicts and reasons | `verdicts.<arm>.outcome`, `reasons`, `evidence.*` | Derived |
| Mechanism check | `mechanism_h_b1.<arm>.{pass, cap_limit_in_force, max_origin_concurrent_observed_whole_run}` | Retained / Measured |
| Limitations | `limitations`, `interpretation.limitations`, `interpretation_notes`, `provenance.what_ran`, `not_covered` | none; verbatim |
| Run state, times, exit code, progress | `GET /runs/{id}` `state`, `created_utc`, `started_utc`, `ended_utc`, `exit_code`, `progress` | tier badge from `tier` |
| Live progress, phase, arm, repetition | SSE `progress` `arm`, `rep`, `phase`, `done`, `total` | tier badge from the run |
| Echoed settings | `spec` (includes defaults the gateway filled in) | none |
| T1 result | SSE `result` `cells`, `cells_csv_sha256`, `note`, `tier` | Simulated |
| Artifacts | `artifacts[].name, bytes, sha256 | {unavailable, reason}, sha256_of, tier, downloadable` | tier badge, or Unavailable |

Anything not in this table is not displayed. Per-phase shed and success-only latency are shown as Unavailable with a reason (section 6, items 1 and 2).

## 6. API gaps found while building (described, not worked around)

1. **Per-phase shed fractions are not in the evidence or summary payload** (only whole-run `shed_fraction` and `shed_fraction_pp`); per-repetition files are not exposed. The chart is an Unavailable panel.
2. **Success-only (HTTP 200) latency is not exposed**, although the runner records it per repetition (`client_success_p99_ms`) and the binding notice quotes it in prose. The latency chart shows all-request percentiles only, labelled as such, and an Unavailable panel for the other.
3. **Matrix (T1) runs have no result summary in the API**: the `result` event carries a cell count and the hash of `cells.csv`. "Arms side by side" and paired differences exist for Scenario B runs (via `summary.json`) and for M6 evidence, not for user-started matrix runs.
4. **Retained artifacts are not downloadable** (`artifacts_downloadable: false`), so M6 CSVs cannot be inspected through the UI.
5. **M6 `per_scenario` rows do not name the comparator arm**; the page says the comparator is the one named in the protocol.
6. **401/403 bodies are plain text, not `lab-1` JSON**; the UI maps them from status and the `X-Auth-Error` header (exposed by CORS). The Lab-off response is an empty `404`, so "disabled" is inferred from a 404 that is not a `lab-1` document.
7. **No way to learn the role of a token** (no whoami / role in `/capabilities`), so Start and Cancel cannot be pre-disabled for an Observer token.
8. **POST /runs/{id}/cancel requires `Content-Type: application/json`** (415 otherwise), which the contract states for "both POST routes"; the UI sends `{}`.
9. Retained Scenario B summaries written before disposition D1 carry no `label_meaning`; the UI falls back to `interpretation.label_meaning` from the evidence payload.
10. A cancelled Scenario B run still lists `summary.json` (status `CANCELLED`, all statistics `null`). Handled as unavailable cells.
11. A bare family `W-Z` is accepted by the gateway although `docs/user/lab_api.md` describes `W-Z(alpha)`; the form offers both (an optional alpha field rewrites the id).

## 7. Accessibility notes and what is unverified

Implemented (by construction and by static lint):

- Landmarks inherited from the shell (`header`, `nav`, `main`, `footer`); one `h1` per page; ordered headings; every `section` named; a labelled `nav` for Lab sections with `aria-current="page"`.
- Native controls only: `a`, `button`, `fieldset/legend`, `label for`, `input`, `details/summary`, `progress`, `table` with caption and `th scope`. Focus ring from tokens. Targets: buttons and nav items 36 px, checkboxes/radios rows at least 24 px high.
- Status is never colour alone: run states and verdicts have a glyph and a word; provenance badges differ by shape and border style; adverse chart markers differ by shape and by the word "credibly worse".
- Errors: `role="alert"`, `aria-invalid`, `aria-describedby` only when the error exists. Connection and stream changes use `role="status"`.
- Reduced motion: there is no animation added; the global reduced-motion rule applies. Forced-colors and `prefers-contrast` rules are inherited from `app.css`; `lab.css` adds forced-colors borders.
- Dark and light themes: tokens only (a test fails on hex colours and named colours in `lab.css`). The contrast script covers the existing token pairs (102/102); new pairs are all existing token pairs (`critical` on `critical-bg`, `info` on `info-bg`, text on surface).
- Responsive: single column below 1100 px for the run form, chart rows collapse to one column below 640 px, tables scroll horizontally in their wrapper.

**Not verified (no browser was run):** visual quality; real layout and wrapping at 390, 820, 1280 and 1920 px; keyboard order and focus after route changes in a real browser; screen-reader announcements; React effect behaviour (the hooks `useLabResource` and `useRunStream` run only in a browser: their logic is covered through the pure reducers, runner and client with fakes, but a real fetch stream through the Vite dev server was not exercised); downloads via blob URLs; CORS behaviour from `http://localhost:3000` (the gateway's preflight answer was recorded with curl); proxy buffering of streaming `fetch`; the 15 s heartbeat on a real idle stream (parsing is tested with synthetic comment lines); StrictMode double-invocation (the hook guards aborted runs, untested in a browser); the JWT flow in a browser (the 401/403 responses were recorded from a real second gateway instance with a minted token).

## 8. Tests

`ui/test/lab.*.test.mjs` (112 tests): client and error mapping (`lab.api`), SSE parser and reconnect/runner (`lab.stream`), view-models from real captured fixtures (`lab.view`), form validation and command generation (`lab.forms`), server-side render of every state (`lab.render`, including `lintHtml` on each), source guards (`lab.guard`: no storage, no token in URLs, one network module, no result-like numeric literals, tokens-only CSS, no new dependencies). Fixtures: `ui/test/fixtures/lab/*` recorded from the real gateway (debug build `D:\Personal\_m7_target\debug\phantom-cache-demo.exe`, 2026-10-30 09:03 +05:30, newer than `src/api/lab_api.rs` 08:35) with `PHANTOM_LAB_DIR` pointing at a temporary copy of the retained evidence; paths in the fixtures are the server's own `<path>` redactions.

## 9. Changes after the first real-browser verification (2026-10-30)

Fixes for F-01..F-11 (static and render tests in `ui/test/lab.fixes.test.mjs`; browser re-check pending):
- F-01 reflow: `minmax(0, 1fr)` app grid, `min-width: 0` on flex/grid roots, wrapping header meta row, `overflow-wrap: anywhere` on ids/hashes/URLs, tables and `pre` scroll inside their own box, Lab tables no longer height-clipped.
- F-02: below 960 px the navigation is behind a **Menu** disclosure button (`aria-expanded`, `aria-controls`, Escape closes and returns focus, choosing a link closes).
- F-03: targets are at least 32 px (`--target-desktop`) and at least 44 px on narrow or coarse-pointer screens (`--target-touch`); inline links are inline-flex boxes.
- F-04/F-05: nav icons sized by CSS; the side nav is sticky with its own scroll (>= 960 px).
- F-06: focus moves to the heading only when the route key changes (not on load, not under StrictMode double effects); in-app navigation scrolls to top.
- F-07: Escape dismisses the cancel confirmation; focus enters it on "Keep running" and returns to "Cancel run".
- F-08: with the stream reconnecting, disconnected, failed or stale, a non-final state chip reads for example "Running (last known; stream disconnected, last event 5)".
- F-09: roles are read from the pasted JWT payload for display only (`lib/labToken.js`; nothing stored or logged; unreadable tokens show nothing). The Start form warns before submitting when the roles lack Operator or admin; the 403 path shows the role message.
- F-10: inline SVG favicon. F-11: the effect-chart axis is one wrapped sentence (no overlapping captions).
- "VALID" now reads "Run class: VALID (narrow meaning)" with an inline "not a success verdict" explanation. Every chart's "View as table" is closed by default with a visible summary (tested).

## 10. Changes after browser round 2 (2026-10-30)

Static and server-render tests only (`ui/test/lab.round2.test.mjs`); not yet re-checked in a browser.
- N-1a: `.table-wrap` is `position: relative`, so absolutely positioned `.visually-hidden` cell text is contained and clipped by the wrapper instead of widening the page.
- N-1b: `.banner__body` and its descendants use `overflow-wrap: anywhere; min-width: 0` so a long debug-build path wraps at 320 px.
- N-3: checkbox and radio controls are 24 px (was 18); `.lab-check` labels are the full-row target (32 px desktop, 44 px touch) and centre the control. `details > summary` is now named in the target rules (the older `details > summary { min-height: 24px }` outranked the bare `summary` rule), so "View as table" and "Show the last N events" are 32/44 px.
- N-4: the effect-chart axis sentence is `display: block` (it was a 3-column grid item squeezed into the 6.5rem label column).
- N-5: an Operator-only token (scenario:operate without telemetry:read) gets an info notice on the Start form before submit; a 403 on the run page shows `ReadForbiddenBlock` explaining that the token can start runs but not read them (run id shown; acceptance is said to be probable, not confirmed). Roles still come from the pasted token for display only.
- N-2 (cause not reproduced): `useTelemetry` and `useRunStream` now release their connection on `pagehide` (and restart on a persisted `pageshow`); `useLabResource.reload` aborts the request it supersedes. See the report for what was and was not established.

## 11. Changes in increment I2: hierarchy and primary workflow (2026-10-30, UI/UX remediation)

Record: `docs/design/remediation/increment_I2_lab.md`. Tests: `ui/test/lab.hierarchy.test.mjs`, `ui/test/lab.markdown.test.mjs` and the updated `lab.render`, `lab.fixes`, `lab.round2`, `lab.view` tests (regression guards, not visual-quality evidence). Earlier sections above are history and still describe the state at their date.

- **Status strip replaces the Connection panel and its banners** (section 1 line "Always present" amended above). Rows 3 and the Connection-panel row of the state matrix (section 3) now apply to the strip: "Checking" chip, API off, unreachable, unauthorised and token states are chips plus an automatically opened Details region. At most one banner-like row sits above the primary task.
- **Page head with a primary action**: the Lab title, one sentence, and a "Start a run" button (hidden on the run page itself). Tabs: About (was "Overview", renamed because it clashed with the app-level Overview), Start a run, Runs, Evidence. Routes are unchanged.
- **Run form**: fieldset groups What to run / Settings / Review; Scenario B is the default kind when the gateway offers it; presets (Quick check, All three arms; Smallest run for the matrix) only fill fields; advanced settings one level deeper; validation by touched state (no error on a pristine form, inline error after blur, error summary with links after a failed submit); long catalogues are collapsed groups with a count and Select all / Clear; the exact request and terminal command are one closed disclosure; a sticky action bar whose disabled state says why (the button is `aria-disabled`, with the reason beside it, only when starting is impossible).
- **Run page**: a sticky run header (L2) with name, id, state, REAL or SIMULATED badge with its qualifier, elapsed time, progress, live-update status and Cancel; cancel is a native modal `<dialog>` (L3). "Live updates" is a small status chip; the banner exists only for stale, reconnecting, disconnected, failed, or a gap in event numbers. Active runs show Progress (bar and "Where the run is now" taken from the latest progress event; no invented phases) and the last five events; finished runs show "Result at a glance" first, then artifacts (sizes, hashes), then closed folds for settings and the event log. Elapsed time is DERIVED from the gateway's start and end timestamps (or the browser clock while active) and says so, or is UNAVAILABLE with the reason.
- **Evidence pages**: the binding interpretation notice is rendered readably by `lib/miniMarkdown.js` (headings, bold, lists, code; links disabled; never HTML) with a Rendered | Raw toggle (Raw is the payload string unchanged) and a Copy button; an "On this page" list; operational detail (paired tables, workload settings, files, manifest) in closed folds with counts; limitation and interpretation lists are body-size text. Charts are drawn inside the shared ChartFrame with a Chart | Table toggle and a full-width axis caption.
- **Provenance wording** (owner vocabulary): MEASURED tiers display as **Real** always with a qualifier ("loopback, one host", plus "debug build" when known); "Retained" is a separate neutral tag; Derived, Simulated and Unavailable unchanged. No tier maps to REPLAYED or PREDICTED (the backend supplies none).
- **Styles**: `lab.css` uses tokens only and the shared form, tab, dialog, disclosure and chart-frame classes; the four dashed or dotted leftovers are gone; sections are a rule plus whitespace, cards (L1) are used for entities only.
