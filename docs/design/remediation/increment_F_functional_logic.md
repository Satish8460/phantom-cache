# Increment F: functional UX pure logic (agent FLIB)

Date: 2026-10-30 (machine clock). Status class for every item: **IMPLEMENTED + TESTED (unit/SSR, node:test)**. Nothing is wired into screens; nothing was browser-validated. Spec: `06_functional_ux_work_breakdown.md` (F0, F1, F3, F4, progress 6.1), owner approval 2026-10-30, DEC-007.

## Objective
Pure-logic layer for U-01 (no verdict on incomplete runs), one connection vocabulary, a gateway-address model, and honest progress, with real-backend fixtures. Screens (`screens/**`, `components/**`, `styles/**`, `App.jsx`) are untouched; wiring is specified below.

## Files
New (src): `ui/src/lib/resultPresentation.js`, `linkState.js`, `gatewayConfig.js`, `progress.js`, `ui/src/hooks/useGateway.js`.
Edited, additive and compatible: `lib/connection.js` (state fields `lastError`, `retryAt`; actions `reset`, `retryScheduled`; `error` accepts `now`), `lib/labStream.js` (`retryAt` in stream state; `error`/`closed` dispatches now pass `now`), `lib/labApi.js` (`probe`, `testGatewayConnection`, `networkFix`; `ERROR_TEXT.network.fix` no longer claims the address is in the header), `lib/labView.js` (`scenarioBView` also returns `verdictsStatus`, `verdictsReason`, `labelPlanned`, `completeness`), `lib/labProvenance.js` (comment only, DEC-007), `hooks/useTelemetry.js` (reset on gateway change, passes `now`, records `retryScheduled`), `hooks/useHistory.js` (clears series when payload goes null).
Tests (register in `ui/test/index.js`, lead): `func.result.test.mjs` (90), `func.link.test.mjs` (34), `func.gateway.test.mjs` (23), `func.progress.test.mjs` (10), `func.provenance.test.mjs` (2) = **159**. Helpers: `test/helpers/funcFixtures.mjs`, `test/helpers/linkHelpers.mjs`. Fixtures: `ui/test/fixtures/func/**` + `PROVENANCE.json` (hash test).

## F0 fixtures and provenance
- LEGACY (unpatched main-tree debug gateway, disk files from `D:/Temp/usab_lab/runs/...`, outside the repo; byte copies, sha256 in PROVENANCE.json): cancelled `20261030T085920Z-9c26980f` (summary `f4b296ea...ea3`, run.json, events.jsonl) and completed `20261030T085851Z-5ce0cf9f`. Disk bytes, not API-served bytes (served copies redact paths and hash differently). The cancelled summary contains local path strings.
- NEW CONTRACT: **SANDBOX-PATCHED BACKEND OUTPUT, not integrated.** Real `summarize()` serde_json output from the U-01 sandbox (`D:\Temp\sbx_u01`) via a sandbox-only dump test (`flib_dump_shapes`, added only in the sandbox file; main-tree Rust untouched), `cargo test -j 1` through `with_cargo_lock.ps1`, `CARGO_TARGET_DIR=D:\Personal\_sbx_u01_target`, debug. Three summaries: cancelled 0/30, cancelled 2/20, complete. The metric VALUES are the sandbox tests' synthetic inputs; the SHAPE is real. T-01 (integrated backend) pending.
- Reproduction of U-01 in the current UI (before/after): test `U-01 REPRODUCED in the current UI` SSR-renders `RunDetailBody` with the legacy cancelled fixture and asserts the page shows `data-verdict-arm="STATIC-CAP"`, "Mechanism failed", "falsified" next to "no verdict can be drawn" while `resultPresentation('cancelled', summary).showVerdicts === false`. It passes today (defect present). **Screen agent: set `SCREENS_WIRED = true` in that test when wiring; the same assertions then require absence.**
- No failed/interrupted real fixture: not producible deterministically; test names say synthetic where state is overridden.

## F1 resultPresentation(runState, summary, {runKind, progress})
Returns `{kind: 'verdicts'|'incomplete'|'unavailable', mode ('verdict'|'no_verdict'|'waiting'|'unavailable'|'matrix'), reason (code), headline, detail, tone, showVerdicts, showMechanismVerdict, showLabel, label, plannedLabel, labelTrusted, labelReasons, legacy, shape, fetchSummary, rawDiagnostics{available,label,listedOnlyInArtifacts}, counts}`. Order: run state, then `summary.status === 'COMPLETE'`, then `verdicts_status === 'not_computed'` / `run_completeness.complete === false`. The shape of `verdicts` is never the signal. Legacy cancelled/failed/interrupted: no verdict, VALID distrusted (only EXPLORATORY can be shown as a class), `failed`/`interrupted` never auto-fetch the summary (D-13). `applyPresentation(view, p)` strips verdicts/mechanism pass from a `scenarioBView` (defence in depth). Table test: 7 states x 10 summary shapes (70 cases) plus targeted cases (unknown state, T1, contradictions, null verdicts).

## F3 linkState
`describeTelemetryLink(connState, now, {probe, security, includeJwtInference})`, `describeLabLink(resource)`, `describeRunStreamLink(stream, status, now)`, `describeLink({...})`, `worstLink`, `LINK_WORDS`, `LINK_TEXT`, `RETRY_LABEL = 'Retry now'`. Words: Live, Connecting, Reconnecting, Disconnected, Unauthorised, Unavailable, Stale, Degraded, Checking (plus non-link "Run ended", "Not watching"). Views are frozen and carry `word, tone, icon, cause, detail, who ('page'|'gateway'), fix, retry, retryLabel, changeAddress, attempts, nextRetryAt, nextRetryInMs, retryOwner, lastError, hasStaleData, basis, notes`. Same condition gives identical word and `cause` on every channel (tested). Attempts 1-2 Reconnecting, 3+ Disconnected for telemetry and run stream.
Honesty: telemetry alone can NEVER be Unauthorised (EventSource exposes no status). Only a classified probe passed by the caller (`labApi.probe`, a real gateway answer) can. "Recovering" is not a state. The BD-05 JWT inference is an opt-in labelled note (`includeJwtInference`, default off, owner decision D-3), never changes the word.
Decision flagged: Lab 5xx maps to Unavailable ("the gateway, or a proxy in front of it, answered with an error") because the page cannot tell gateway from proxy. 403 origin / 421 host map to Unauthorised per spec 4.2, with the specific cause in `detail`/`fix`.

## F4 gatewayConfig
`validateGatewayInput`, `resolveGatewayConfig` (query -> saved -> env -> default; a rejected candidate falls through to the next valid source and leaves a visible `notes[]` entry, `status: 'fallback'`), `createGatewayStore` (`getSnapshot/subscribe/apply/reset/validate/setLock`; snapshot has `url, source, sourceText, status, notes, remembered, storageAvailable, needsLinkConfirmation, hostPort, epoch, locked`), `originChanged`, `describeProbe`, storage helpers (every access try/catch; works when storage or the accessor throws; `apply` returns `persisted: 'saved'|'not_saved'|'skipped'`). Persisted value is exactly `{"url":"<origin>"}` under `phantomcache.gateway.v1`; credentials in the URL are rejected (message never echoes them); path/query/hash are dropped with a notice. Tokens: the module has no token concept; the provider must forget the token when `epoch` changes. Test proves no `Authorization` is sent to the new origin once the ref is cleared (and documents that today's provider does not clear it, A14).

## Progress (6.1)
`progressView({kind,state,progress,spec,startedMs,endedMs,nowMs,lastEventAtMs})`: counts from the gateway only; bar only when `done > 0` and not terminal (never 0-of-N); "Run N of M in progress: ARM, repetition r of R"; phase step k of 4; planned minimum and "At least X left by plan" as a lower bound, "overdue" text, every missing input returns `unavailable` with a reason; ETA always unavailable; T1 counts cells with no estimate. A REAL event log (cancelled fixture) is replayed through `applyRunEvent` in a test.

## DEC-007 (provenance)
Encoded as comments in `labProvenance.js` and tested in `func.provenance.test.mjs`: retained Scenario B evidence = word "Real" + qualifier + separate `retained` flag; no tier maps to REPLAYED or PREDICTED; an API tier string "REPLAYED"/"PREDICTED" is treated as unknown. No logic change was needed (the existing mapping already satisfied it).

## Wiring spec for the screen agent
1. Result (`RunsViews.jsx` ResultSection, `LabScreen.jsx`): `const p = resultPresentation(state, summaryJsonOrNull, {runKind: run.kind.key, progress: run.progress})`. Fetch summary only if `p.fetchSummary` (needs the raw JSON; keep `scenarioBView` for display). If `p.mode==='no_verdict'` render `p.headline`/`p.detail` (`role="status"`, tone `p.tone`) plus, when `p.rawDiagnostics.available`, a closed `<details>` titled `p.rawDiagnostics.label` containing arms table, settings, limitations only. Pass `applyPresentation(view, p)` to `ScenarioBResult`; show the class label only if `p.showLabel` using `p.label` (show `p.plannedLabel` as "planned, not applicable"). Do not use `verdicts` emptiness to decide anything.
2. Connection: replace `deriveConnection`/`CONNECTION_TEXT`/`STREAM_TEXT`/`ConnectionPanel` pill wording with `describeTelemetryLink(state, now)`, `describeLabLink(res)`, `describeRunStreamLink(stream, status, now)`; render `view.word`, `view.cause`, `view.detail`, `view.retryLabel` (button label), `view.changeAddress`. Header uses `worstLink`. Retired strings: "Reconnect now", "Not reachable or not authorised", "No connection to the gateway", "Could not reach the gateway". Pass `now` ticking from `useTelemetry` (`now`). Run stream: also need `stream.retryAt` (already set by reducer).
3. Gateway: in `App.jsx` `const store = useGatewayStore({location: window.location, envValue: import.meta.env.VITE_PHANTOM_GATEWAY}); const gw = useGateway(store);` use `gw.url` for `useTelemetry`, `eventsUrl`, LabProvider `baseUrl`; show `gw.notes` on every route and `gw.hostPort`/`gw.sourceText`. Popover: `store.validate(text,{securityMode})`, Test via `testGatewayConnection({baseUrl: candidate})` + `describeProbe`, Apply `store.apply(text,{remember})`, `store.reset()`. LabProvider: when `gw.epoch` changes set `tokenRef.current = null`, roles null, bump version. While `startRun`/`cancelRun` is pending call `store.setLock('a run request is in progress')` then `setLock(null)`. Navigate away from `#/lab/runs/<id>` and `#/lab/evidence/<id>` after an origin change. If `gw.needsLinkConfirmation`, require a confirmation before the first "Use token".
4. Telemetry JWT probe (optional, owner D-3): after outage, one `client.probe()`, map `error.kind` to `{kind}` and pass as `opts.probe`; at most every 30 s (not implemented here, caller-side).
5. Progress: `progressView({kind: run.kind.key, state, progress: live.model.progress || run.progress, spec: run.specRaw, startedMs: run.startedMs, endedMs: run.endedMs, nowMs, lastEventAtMs})`; render `text`, `phase`, `atLeastLeft.text` or `.reason`, `<progress>` only if `bar.render`. Replaces `progressText` ("units").
6. Guard test note: `lab.guard` requires the provider keep `tokenRef = useRef(null)`; unchanged by this wiring.

## Tests: before and after
Baseline given: 299 pass. Now `npm test` (index.js, my files not registered): 296 pass, **3 fail, none in my files**: `no backdrop-filter and no glass anywhere in the styles`, `F-04/F-05 nav icons ... sticky`, `shell.css: navigation from 820 px ...` (CSS assertions failing against `styles/**` being edited concurrently by the redesign; not touched by me). The 159 new tests pass when run directly: `node --test test/func.*.test.mjs` (after registration they run inside index.js). `check:forbidden` pass, `check:a11y` 74 combos 0 issues, `check:contrast` 284/284.

## UNVERIFIED
- `/api/v1/metrics?schema=2` status codes under JWT; CORS preflight; whether a browser really hides EventSource status (platform knowledge).
- Behaviour of the changed `useTelemetry`/`useHistory` hooks in a browser (no React effect testing exists here; only module import, SSR and reducer tests).
- Sandbox shape is not the integrated backend; `lab_api.rs` retained-snapshot coercion of null verdicts to `{}` (contract.md known consumer) is not handled because the Evidence path is outside this scope.
- D-7 (telemetry closes the EventSource on every error and owns the retry) is NOT implemented: only reducer support (`retryScheduled`) and the existing closed-state timer are recorded; `retryOwner: 'browser'` is reported honestly meanwhile.

## Limitations / notes
- Git HEAD moved during the work (3e6e03e -> df68eaab, another agent); working tree otherwise untouched by me beyond the files above.
- The fixture legacy files are disk copies outside the repo's recorded evidence; paths inside them are local.
- `resultPresentation` treats a completed summary without `status` as unavailable (conservative; all real summaries have it).

## Open questions for the owner
1. Lab 5xx as "Unavailable" (vs "Disconnected")?
2. Enable the opt-in JWT inference note for telemetry (D-3), or build the probe (M2)?
3. Implement D-7 (page-owned telemetry retry with countdown)?
4. Confirm `?gateway=` non-loopback confirmation step (D-5) as a hard gate before token entry.


## Lead decisions on the open questions (2026-10-30 16:35)
1. Lab 5xx = Unavailable (AD-8). 2. JWT telemetry note stays off; no probe (AD-9, owner-reserved DEC-002). 3. Implement D-7 in the wiring phase (AD-10). 4. D-5 non-loopback token-entry confirmation is a hard gate (AD-11). Recorded in docs/design/OWNER_DECISION_REGISTER.md section 4. The null-verdict coercion in lab_api.rs for retained snapshots is part of DEC-001.
