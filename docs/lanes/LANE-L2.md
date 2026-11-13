# LANE-L2 handover (rewritten for work order D-053, 2026-10-31)

## A0. Follow-ups after the demo-hub order (latest; supersedes the matching lines of section A)

**All 30 required scenarios on Start here.** `ui/src/lib/hub.js` holds ONE list (`SCENARIOS`) with the 30 ids of `D:\ClaudeTemp\lanes\REQUIRED_SCENARIOS.md` in storyline order and the 11 groups of that file (Start, Speed and savings, Safety, Resilience, Freshness and memory, Protection, Operability, Intelligence, Observability, Performance, All), each with a title and a one-line "what you will see". The page shows a prominent "Demonstrate everything" link (`HUB/#/scenario/everything`) at the top, then one compact disclosure per group (first two open, count in the summary), each scenario one row: title, one line, one "Run in the Demo Hub" link (`HUB/#/scenario/<id>`, new window, noopener, no token, loopback-only `?hub=`). Test `starthere.test.mjs` "catalogue" asserts the 30 ids, their order and the group order, and, when the owner file exists on the machine, parses its table and requires ids, order and groups to be identical, so a missing or reordered scenario fails the suite. The page test asserts 30 rows, 11 groups and the Demonstrate-everything link.

**Friendly, no "not available" on a healthy gateway.**
- Nav: Intelligence is a normal "Live gateway" item (no "Not available yet" group) unless the gateway has ANSWERED that it lacks the API (404, old-shape status, or v2 not installed). While the question is still being asked, and when only a sign-in is missing, it is shown as normal (`intelligenceNavLive`), so there is no flash of the old copy. The old copy stays for a gateway that really lacks the API.
- Intelligence screen: rows for values that can never exist on the gateway (reason `not_implemented`, `needs_multi_node_view`, and the always-unavailable "observed effect") are left out instead of printed as Unavailable; values that will appear once samples exist (for example "Not enough samples yet") keep their honest label; every value that is shown keeps its provenance badge.
- Identity chip: "Signed in as <tenant or subject>"; the secure gateway with no token is a friendly "Sign in to see this gateway" card (info tone, one button "Sign in", the token field) and the chip reads "Sign in". Real refusals (expired, wrong audience, forbidden, rejected) keep their calm explanatory states with warning or critical tone.
- Welcome banner: "Welcome to PhantomCache!"; Start here is plain and cheerful.
- Happy-path console check: `browser-l2-auth.mjs` now asserts that, after signing in, no console error, warning or exception appears.
- Not done, stated plainly: other screens (Overview, Traffic, Cache, Freshness, Origin, Cluster) still list "Not reported" rows for values the gateway does not provide, and the header connection word for a missing token is still the shared "Unauthorised" (lib/linkState.js vocabulary, shared with tests). Changing them touches many existing guard tests; not attempted in this time box. Intelligence data in screenshots is fixture data (REV-009 demo build not used in this lane).

Counts after these changes: UI suite 576/576 (561 original-order baseline + 15 new across Start here and the friendly changes), check-contrast 284/284, check-forbidden PASSED, check-a11y 74 combinations 0 issues.

## A. Demo-hub work order: Control Center side (this round)

| Item | Status | Evidence |
|---|---|---|
| 1. 'Start here' first nav item, three-step explanation, connection and identity status, Hub scenario links | DONE | Own group 'Start' at the top of the nav, route #/start, kind 'guide' (own footer). Page: what PhantomCache is, 3 steps, tour button, this page's connection word and signed-in identity, 12 scenario links (ids gallery, stampede, peerfill, scale, security, invalidation, deadnode, freshness, eviction, operability, benchmark, tour) opening HUB/#/scenario/ID in a new window (noopener), glossary (HIT, MISS, Peer, Coalesced, Stale, Tenant, Token). Hub base default http://127.0.0.1:3201, and a ?hub= query is accepted only for http(s) loopback without credentials, otherwise the default plus a visible note. No token is ever in a link. |
| 1b. 'What am I looking at?' help line per screen with a Hub link | DONE | Under the title of Overview (gallery), Traffic (scale), Cache (eviction), Freshness (freshness), Origin (stampede), Cluster (deadnode, peerfill), Intelligence, Simulation, Lab (benchmark) via ScreenFrame. |
| 2. First-visit welcome, friendly offline and token states | DONE | Dismissible banner (Start here / Open the Demo Hub / Dismiss), remembered per tab only (tab storage holds the word 'seen'); never shown on Start here. Start here is not gated by the telemetry state, so offline shows 'No gateway answers at this address ... press Start the demo cluster in the Hub, then Retry now'; a secure gateway without a token shows the 'requires a token' text and points to the chip. |
| 3. Tests, checks, screenshots | DONE | 573/573 UI tests (561 before + 12 new in test/starthere.test.mjs); check-contrast 284/284, check-forbidden PASSED, check-a11y 74 combinations 0 issues. Live: scripts/browser-l2-start.mjs 9/9 PASS (headless Chrome, real secure gateway phantom_rev6.exe): welcome, Start here no-token, 12 deep links, welcome stays dismissed after reload, offline state, refused ?hub= note, phone width no horizontal overflow, no console exceptions. Screenshots read: shots/30-34 in D:\ClaudeTemp\lanes\evidence\L2\. |

Hub-side notes for L1: the hand-off protocol is unchanged (see section B). A Control Center link opened from the Hub must be a plain link of the form UI/?gateway=ORIGIN (no token), opened with window.open and the postMessage reply; the Hub's own #/scenario/<id> routes must exist for the links above.

### A.1 Files changed this round (relative to lane root)
Created: ui/src/lib/hub.js, ui/src/components/HubParts.jsx, ui/src/screens/StartHere.jsx, ui/test/starthere.test.mjs, ui/scripts/browser-l2-start.mjs.
Modified: ui/src/lib/routes.js (Start entry, 'guide' footer), ui/src/lib/tokenSession.js (welcome flag helpers; still the only sessionStorage user), ui/src/App.jsx (route, welcome, hub in context), ui/src/screens/ScreenFrame.jsx (help line, optional ungated), ui/src/styles/pages.css (layout rules, tokens only), ui/scripts/harness-entry.jsx (renderStart, renderWelcome), ui/test/index.js (registers starthere), ui/test/shell.test.mjs, ui/test/shell.v2.test.mjs, ui/test/intelligence.test.mjs, ui/test/auth.test.mjs.

### A.2 Existing tests I had to change (all because Start here is a new first item; no assertion loosened)
shell.test.mjs: group list now starts with 'Start'; nav group count 4 to 5. shell.v2.test.mjs: fold id app-nav-group-3 to -4; 'nine links' to 'ten links'. intelligence.test.mjs (my own): ten links, 'start' first in order. auth.test.mjs (my own): App context regex gained 'hub'.

### A.3 Limitations
- Hub links are plain links; this lane cannot check that the Hub serves #/scenario/<id> (L1 owns it). Cards on Start here are text links in a list, not graphic cards.
- Help lines are hidden on screens behind the telemetry gate (no data yet), by design of ScreenFrame; Lab screens use their own frame and have no help line.
- No axe run; no live test with a running Hub (it did not exist in this lane copy).
- Processes started for the live check (gateway, vite, Chrome) were stopped; ports 3202 and 18210 verified closed.

---

# B. Earlier work (merged): Control Center sign-in, Intelligence screen, identity chip


Lane L2 (UI), 2026-10-31. Worked only in `D:\ClaudeTemp\lanes\L2\`. Evidence: `D:\ClaudeTemp\lanes\evidence\L2\` (shots/, logs/, l2auth.results.json, l2intel.results.json). No token, secret or JWT is in any screenshot, log or this file (password field, shots only; the lane secret lives in `L2\_work\`).

## 1. Status

| Item | Status | Evidence |
|---|---|---|
| 1. Token for the live screens | DONE | Reproduced here against the real release binary `D:\ClaudeTemp\audit\phantom_rev6.exe` with `PHANTOM_JWT_SECRET` (random, lane dir), tokens minted with Node crypto using the claim names of `src/security/jwt.rs` (sub, iss, aud, exp, iat, tenant_id, roles). `browser-l2-auth.mjs`: 27/27 checks PASS (headless Chrome, loopback, debug-free release gateway, one run). Screenshots read by me: 01 requires token, 03 expired, 04 audience, 05 rejected, 06 forbidden, 07 live as tenant-a, 08 identity popover, 11/12 expiry countdown then expired, 13 hand-off window. |
| 2. Intelligence screen | DONE against the contract, with fixtures | Built from `docs/lanes/CONTRACT-pie-api-v1.md` (read as written, 2026-10-31). No gateway served the v2 endpoints, so apart from one real capture (below) the fixtures are CONTRACT-SHAPED, hand-written, NOT captured. Live-browser run `browser-l2-intel.mjs`: 19/19 PASS, but registry/signals/decisions came from a lane-local proxy serving fixtures (screens named `*_FIXTURE`), authentication and everything else from the real gateway. |
| 3. Cluster and Overview for the demo | DONE | Identity chip is in the header on every screen; "Open as another user or on another gateway" hint (no token in URLs) in the chip popover and in the banner. Screenshots 25, 26. |

### Item 1 behaviour (what was built)
- Chip in the header (`[data-identity-chip]`): "Token required", "No token (not required)", "Tenant <id>", "Token expired", "Token not accepted", "Token lacks a role". Popover: gateway answer in words, decoded tenant/subject/roles/issuer/audience/expiry countdown, label "Decoded by this page, not verified; the gateway verifies the token.", paste form, "Keep for this tab" checkbox (default OFF), Forget token, open-as-another hint.
- Banner at the top of the main area only when something must be said (no token on a secure gateway, 401 expired / wrong issuer / wrong audience / not accepted, 403 role, 403 tenant, offered hand-off). A secure gateway with no token shows "This gateway requires a token" and the form, not a broken screen. The header connection word becomes "Unauthorised" (from the gateway's own 401, through the existing `probe` seam of `lib/linkState.js`; no retry countdown is shown because none runs).
- Transport: with a token the live stream is read with `fetch` streaming and an Authorization header (an EventSource cannot send one); without a token it is the existing EventSource plus one rate-limited probe (15 s) of `/api/v1/metrics?schema=2` to find a 401/403. A 401/403 stops the retry timer until the token changes or the person presses Retry now. A token change or Forget clears the values read under the old credential.
- Token handling: held in memory per gateway ORIGIN (never offered to another origin; max 8 origins); optional sessionStorage only when ticked; never in a URL, localStorage, cookie, log, error text or rendered markup (tests assert the signature/payload parts never appear in markup). Only `lib/tokenSession.js` touches storage.
- Hand-off (decision): the page asks its opener with `postMessage({type:'phantomcache:token-request', v:1}, '*')` (no secret). The opener replies to the page origin only with `{type:'phantomcache:token', v:1, gateway:'<origin>', token}`. Accepted only if the sender is `window.opener`, exact field set, `gateway` equals this page's gateway, token is JWT-shaped. Applied at once if the sender origin is this page's origin or loopback; any other origin waits for a click ("Use the offered token"). Held in memory only. Tested: 12 table cases in `auth.test.mjs`, and live (script step 14: second window got tenant-b by message, was live, nothing in its URL or sessionStorage). A paste field exists as the alternative.
- Expiry: the gateway checks a token when a request or stream starts (`src/api/server.rs` `sse_handler`, verified by reading). An open stream therefore keeps running after `exp`; the page says so ("authorised before the token expired") and the next request or reconnect is refused. Reproduced in the 25 s token run.
- Intelligence/diagnostics/metrics: the signed-in client (`createLabClient(...).getJson/openStream`, in `lib/labApi.js`) is the single place with `fetch`, so the existing guard "network access exists in exactly one module" is unchanged. The main UI today reads metrics only through the SSE frame; the Intelligence screen is the other consumer.

### Item 2 behaviour
- `ui/src/lib/intelligenceView.js` (pure), `hooks/useIntelligence.js`, `screens/Intelligence.jsx`. Shows: mode with the fixed statement "Only three modes exist: Off, Observe and Advisory..." and "Applied changes: none"; registry (policy, kind, lifecycle, highest mode, effective state, declared documentation, parameters, required inputs); signals (window, features F*, class, top key HASHES, history) with sample counts; recent decisions newest first with inputs, candidates (selected/rejected, reason, estimated effect with 95% interval, basis), constraints, guardrail events, conflict, observed effect (unavailable, with reason), explanation; engine counters (events queued, dropped, depth, memory upper bound as DERIVED, decisions recorded/retained/suppressed/collapsed); declared configuration and claims (benefit claim none, M6 H1 NOT_SUPPORTED statement).
- Every value shows the gateway's provenance badge, or Unavailable plus the reason. Withheld, never shown as normal: `simulated` values in live mode; `unavailable` carrying a value; unknown provenance; a mode other than off/observe/advisory (flagged "does not recognise"); a recommendation whose `applied` is not exactly `false`; key hashes that are not 16 hex digits; records claiming `simulated:true`.
- States: loading; 401/403 (points to the sign-in banner); no answer; 404 on status = "does not serve the Phantom Intelligence API" plus the old Placeholder statement; old-shape `/status` (REAL on rev6: mode Off only, "reports only the older intelligence status"); `v2.installed:false`; ready. A missing section shows its own reason while the others render. No fake rows: empty decisions say "No decisions have been recorded".
- Polling: status every 15 s always (so the nav knows); signals + newest 20 decisions every 2 s only while the screen is open and the gateway answered; registry once per gateway/token. Aborted on unmount and pagehide. 6 decision requests in the whole 19-check run.
- Nav: Intelligence stays under "Not available yet" (registry default, all old tests untouched) until the gateway reports `v2` installed; then it moves to "Live gateway" after Cluster, loses the tag, and the page caption follows. Implemented in `Shell.jsx` through `TelemetryContext` (the pinned `Nav(...)` prop list is unchanged).
- Assumption stated: contract is read as written. The legacy `intelligence_mode` string is shown as measured (a reading of this process, like `process_mode`). The page is gated by the telemetry stream like every other screen (`ScreenFrame`).

## 2. Files changed or created (relative to lane root `D:\ClaudeTemp\lanes\L2\`)

Created:
- `ui/src/lib/gatewayAuth.js`, `ui/src/lib/tokenSession.js`, `ui/src/lib/telemetryStream.js`, `ui/src/lib/intelligenceView.js`
- `ui/src/hooks/useGatewayAuth.js`, `ui/src/hooks/useIntelligence.js`
- `ui/src/screens/authUi.jsx`, `ui/src/screens/Intelligence.jsx`
- `ui/test/auth.test.mjs` (32 tests), `ui/test/intelligence.test.mjs` (22 tests)
- `ui/test/fixtures/intelligence/` : `README.md`, `status_legacy_real.json` (REAL), `status_v2_advisory.json`, `status_v2_not_installed.json`, `registry.json`, `signals_advisory.json`, `signals_observe_cold.json`, `decisions.json`, `decisions_empty.json` (contract-shaped)
- `ui/scripts/browser-l2-auth.mjs`, `ui/scripts/browser-l2-intel.mjs`, `ui/scripts/capture-pie-fixtures.mjs`

Modified:
- `ui/src/App.jsx` (auth + intel hooks, banner, chip, Intelligence route, context fields `auth`, `intel`, `intelLive`)
- `ui/src/hooks/useTelemetry.js` (token-aware: fetch stream with token, probe without, refusal stops timer; EventSource path kept)
- `ui/src/lib/labApi.js` (added `isGatewayPath`, client methods `getJson`, `openStream`)
- `ui/src/lib/linkState.js` (refused-telemetry branch: no retry countdown/owner; sentence no longer says the page cannot send an Authorization header, says "has no token to send" only when none is set)
- `ui/src/components/Shell.jsx` (Nav regroups Intelligence from context)
- `ui/src/screens/ScreenFrame.jsx` (optional `group` caption override)
- `ui/src/screens/linkUi.jsx` (footnote: "Bearer tokens are kept in this page's memory only and are never saved, unless you tick Keep for this tab ..." keeps the pinned phrase)
- `ui/src/styles/pages.css` (3 layout rules, tokens only)
- `ui/scripts/harness-entry.jsx` (exports `Auth`, `Intel`, `renderIntelligence`; `renderShell` can provide `intelLive`)
- `ui/test/index.js` (registers the two new suites)
- `ui/test/func.gateway.test.mjs` (ONE assertion widened, see section 5)

Lane-local, NOT to copy: `_work/` (secret, tokens, mint.mjs, gateway start script, mock proxy, fixture generator, logs), `_patch_tele.mjs`.

## 3. Test results (all in `D:\ClaudeTemp\lanes\L2\ui`)
- `node --test test` : 561 tests, 561 pass, 0 fail (507 existing + 32 auth + 22 intelligence). Before my first edit the suite was 507/507.
- `node scripts/check-contrast.mjs` 284/284; `check-forbidden.mjs` PASSED; `check-a11y.mjs` 74 combinations, 0 issues (the new components are linted by `lintHtml` inside the new suites, not by that script's list).
- Live: `browser-l2-auth.mjs` 27/27, `browser-l2-intel.mjs` 19/19 (final runs; the first run failed 2 checks because closed `<details>` hide text, fixed by opening Recommend records by default). Two real bugs were found by the browser and fixed with tests: the token holder published a stale snapshot (identity did not appear after the second token), and a reload test used a hash-only navigation.
- Not run: `check-axe.mjs`, no cargo builds (UI lane), no Windows high-DPI or phone-width visual pass (CSS reuses existing classes; no screenshot at phone width was taken).

## 4. Reproduce live
```
# secure gateway (run from L2\_work, creates secret.txt and tokens.json on first use)
node _work\mint.mjs
pwsh -NoProfile -File _work\start_gw.ps1            # phantom_rev6.exe on 127.0.0.1:18210, CORS for http://localhost:3202 and 127.0.0.1:3202
cd ui; node node_modules/vite/bin/vite.js --port 3202 --strictPort --host 127.0.0.1
node _work\mock_pie_proxy.mjs 18211 18210 ui\test\fixtures\intelligence advisory      # fixtures behind the real gateway's auth
node _work\mock_pie_proxy.mjs 18212 18210 ui\test\fixtures\intelligence not_installed
cd ui
$env:L2_MINT='file:///D:/ClaudeTemp/lanes/L2/_work/mint.mjs'
node scripts/browser-l2-auth.mjs http://127.0.0.1:3202 http://127.0.0.1:18210 <tokens.json> <evidence-dir>
node scripts/browser-l2-intel.mjs http://127.0.0.1:3202 http://127.0.0.1:18211 http://127.0.0.1:18210 http://127.0.0.1:18212 <tokens.json> <evidence-dir>
```
Open by hand: `http://127.0.0.1:3202/?gateway=http://127.0.0.1:18210#/` and paste a token in the field. All lane processes (gateway, vite, proxies, Chrome) were stopped and their ports verified closed.

## 5. Limitations, doubts, existing tests I touched
- Intelligence v2 data is fixture data until L4 serves the endpoints. Replace fixtures with real captures via `scripts/capture-pie-fixtures.mjs`, then re-run `intelligence.test.mjs`; a field-name mismatch with the real library output would show as Unavailable rows, not as wrong numbers. The `/status` `v2` block placement is taken from the contract section 3 ("add under key v2").
- ONE existing assertion widened: `ui/test/func.gateway.test.mjs` "only gatewayConfig.js (plus theme.js and Shell.jsx) touch browser storage": added `lib/tokenSession.js` to the allowed list (the brief allows sessionStorage; the file is the only credential holder and `auth.test.mjs` asserts it is the only sign-in file using sessionStorage and that no sign-in file uses localStorage). The test title says so. Nothing else was deleted or loosened; all other guards (network only in `labApi.js`, App never mentions Authorization/getToken/storage, Shell has no "token", AD-10 regexes) pass unchanged. The footnote sentence in `linkUi.jsx` was extended, keeping the pinned phrase; please review that it is still honest.
- The Lab keeps its own memory-only token (separate provider, remounted per gateway). I did not merge the two: the Lab guards forbid storage in its files and a shared token would change the Lab provider. A person using the Lab on a secure gateway still pastes the token there.
- Telemetry on a secure gateway with a token uses fetch streaming; the page-owned backoff and link vocabulary are reused, but `useTelemetry` now has two code paths (EventSource and fetch). Chrome only was tested.
- The no-token probe asks `/api/v1/metrics?schema=2` at most every 15 s while the stream is failing; on a dev (no JWT) gateway that is harmless, on a secure one it produces the 401 the page needs.
- The 403 for a token without a tenant applies to cache routes, not telemetry; the page maps `MissingTenant`/`ReservedTenant` to its own message but the telemetry screens never trigger it.
- Hand-off from a non-loopback opener needs a click by design. The demo console must send the message only to this page's origin and only after receiving `phantomcache:token-request`.
- Display-only decoding can be wrong about the gateway's clock; the text says "by this page's clock".
- Port 3202 CORS: the dev gateway only allows listed origins; the launcher must set `PHANTOM_CORS_ORIGINS` for the UI origin it prints.

## 6. Requests for files I do not own
- Launcher/B4 (`scripts/cluster_launcher.mjs`, demo console): print per-user URLs as `?gateway=<origin>` only (no token), set `PHANTOM_CORS_ORIGINS` to the UI origin, and for the console window use `window.open` plus the `phantomcache:token` message described above (or tell the person to paste from `tokens.json`).
- L4: serve `registry`, `signals`, `decisions` and the `v2` block of `/status` exactly per `CONTRACT-pie-api-v1.md`; send CORS exposure of `x-auth-error` and `www-authenticate` (already present in `src/api/server.rs`).
- Owner: decide whether the board's D-040 list of "tests that encode Intelligence as a placeholder" needs a note; this lane did NOT change them (Intelligence stays "placeholder" in the registry until a gateway reports v2, and the nav promotes it dynamically).
