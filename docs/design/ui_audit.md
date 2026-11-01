# UI Audit (M8-01) and Truthfulness Table

**Date:** 2026-10-29. **Scope:** `ui/` after Phase 1 (the schema-v2 `TruthfulDashboard` in the old `ui/src/App.jsx`, 405 lines) and after the M8 rewrite described in section 4. **Backend read for this audit:** `src/api/server.rs` routes (lines 104-119), `build_schema_v2_payload` (284-545), SSE handler (584-680), health/intelligence/diagnostics handlers (196-250, 1793-1830). `docs/research/telemetry_schema_v2.md` was read in full.

Evidence tags: **V** verified in code (file:line), **R** reproduced by a test or script in this change, **D** claimed by a design/research document only, **U** not verified.

## 1. Backend surface the UI can use today (V)

| Endpoint | Used by the UI | Notes |
|---|---|---|
| `GET /api/events?schema=2` (SSE, 100 ms) | Yes, the only data source | `server.rs:105,597-607`. Without `?schema=2` it serves legacy schema 1, which the UI refuses. |
| `GET /api/v1/metrics` (JSON, same payload) | No | Could serve a polling fallback. |
| `GET /health` | No | Returns the literal text `OK` (`server.rs:197`); no readiness detail. `/health/live` and `/health/ready` also appear (`src/health.rs:78-79`), unread. |
| `GET /api/v1/diagnostics/summary`, `/events` | No | Counter snapshot and recent events. Schema unspecified in the research docs; not consumed until a contract exists. |
| `GET /api/v1/intelligence/status`, `/audit` | No | Exist (`server.rs:118-119, 200-247`) but are unauthenticated and unpaginated; no UI contract. See `ui_api_requirements.md`. |
| `POST /api/control` | Only inside the SIMULATED demo | Mutates simulation state only. |

## 2. Truthfulness table: every value the UI displays

Provenance classes: measured / derived / simulated / inferred / stale / unavailable. "stale" is a display state of a value that was measured or derived, not a provenance class the gateway emits.

| Screen | Displayed value | Source | Class shown | Notes |
|---|---|---|---|---|
| All | Connection status pill | Client: EventSource state and frame age | inferred | `lib/connection.js` |
| Overview | Last frame age; frames received / unreadable | Client | inferred | Not gateway data. |
| Overview | Client minus server clock | Client receipt time minus `as_of_ms` | inferred | Includes network delay; informational. |
| Overview | Schema, mode | `schema`, `mode` | measured (config) | Per spec section 2.1. |
| Overview | Server time of last frame | `as_of_ms` | measured | |
| Overview | Gateway uptime | `uptime_ms` | measured | Backend reports 0 if telemetry is absent (`server.rs:468`), see BD-04. |
| Overview | Capabilities (cluster, controller, origin metrics) | `capabilities.*` | derived | Shown with their words; unknown values shown verbatim. |
| Overview | Key signals (6 tiles) | See catalog rows below | per field | |
| Overview | "Other reported fields" | Any provenance field in `metrics` not in the UI catalog | per field | Surfaces new backend fields (for example from R-01) without assuming names. |
| Traffic | `throughput_req_s_1s` / `_10s`, `hit_ratio_1s` / `_10s` | same names | derived | Windows stated on the tile. |
| Traffic | `hit_ratio_cumulative` | same | derived | Backend returns 1.0 with no traffic; the tile carries a note (BD-02). |
| Traffic | `requests_total`, `cache_hits`, `cache_misses`, `coalesced_waiters` | same | measured | Cumulative since start. |
| Traffic | `latency_ms.p50/p90/p95/p99/samples` | same object | measured | Ring-buffer window; size is 10 000 in `main.rs:249` (V) but the UI states only "gateway ring buffer". |
| Traffic | Three sparklines | Client history of `throughput_req_s_1s`, `hit_ratio_1s`, `latency_ms.p99` | derived / derived / measured | Each point is a received value; unavailable becomes a gap. History is page-local, at most 120 points. |
| Cache & memory | `bytes_resident`, `byte_budget` | same | measured | `basis: logical_accounting`; tile note says it is not process memory. |
| Cache & memory | Capacity bar | geometry of the two values above | n/a | No percentage is displayed. |
| Cache & memory | `evictions_total`, `capacity_rejections`, `byte_budget_rejections`, `oversize_rejections` | same | measured | |
| Cache & memory | `process.rss_bytes` | same | measured / unavailable | `unsupported_platform` on Windows (V `server.rs:278-281`). |
| Freshness | `stale_served_total`, `stale_cap_violations` | same | measured | Violations render critical when non-zero. |
| Freshness | `revalidations.started/ok/not_modified/error/in_flight` | same | measured | |
| Origin | `origin.requests_started/requests_completed_ok/in_flight` | same | measured | Whole block unavailable if `origin` has `src: unavailable`. |
| Origin | `origin.errors.timeout/network/status_5xx/other` | same | measured | |
| Origin | `origin.latency_ms.p50/p95/p99` | same | derived | |
| Origin, Overview | Any other `origin*` / unknown `metrics.*` provenance field (for example `origin_cap.*`) | as received | per field | Generic table; booleans shown as true/false. |
| Cluster | `nodes[]` table (id, state, state provenance, cpu, memory) | `nodes` | per `state_src` | Only when `capabilities.cluster = experimental`. See BD-03. |
| Cluster, Lab, Intelligence | Placeholder text | `capabilities.*` | n/a | Contains no numbers (R: `viewmodel.test.mjs`). |
| Simulation demo | Everything | `SimDemo.jsx`, model output | simulated | Wrapped by a permanent SIMULATED banner and a double border; reachable only at `#/simulation` or the legacy `?view=sim-demo`. |

Unavailable and missing fields render as the words "Unavailable" or "Not reported" plus the reason, never as 0 or a dash (R: `render.test.mjs`).

## 3. Defect register

### 3.1 UI defects found in the pre-M8 schema-v2 dashboard (all addressed by the M8 rewrite unless noted)

| ID | Severity | Evidence | Defect | Status |
|---|---|---|---|---|
| UA-01 | High | old `App.jsx:349` | In-flight origin fetches used `?? '0'`: a missing value was displayed as 0. | Fixed: missing renders "Not reported". |
| UA-02 | High | old `App.jsx:319` | Sample count used `|| 0`, the same zero-for-missing pattern. | Fixed. |
| UA-03 | High | old `App.jsx:355` | `metrics.origin.errors.status_5xx.v` dereferenced with no guard; a partial origin object throws and blanks the page. | Fixed: all reads go through `readField`. |
| UA-04 | High | old `App.jsx:77` | `metric.v.toLocaleString()` throws when `v` is null or not a number. | Fixed (R: `provenance.test.mjs`). |
| UA-05 | High | old `App.jsx:120` | Frames with `src: simulated` in live mode were displayed with a SIMULATED badge instead of being rejected, contrary to the spec rule that live mode has none. | Fixed: withheld and counted as a contract violation (degraded state). |
| UA-06 | High | old `App.jsx:106,123,161` | "AS OF" showed the browser clock at receipt, not the server's `as_of_ms`; it looked like a server timestamp. | Fixed: server time and client age shown separately and labelled. |
| UA-07 | High | old `App.jsx:130-132` | On error the status flipped to DISCONNECTED but last values stayed on screen unmarked; no staleness detection if the stream stalled silently. | Fixed: stale and disconnected states mark values stale and show age. |
| UA-08 | Medium | old `App.jsx:203,387-395` | "No adaptive controller active" and "`pie: None`" were static text, shown even if `capabilities.controller` were `active`. | Fixed: capability-driven placeholder. |
| UA-09 | Medium | old `App.jsx:194` | Static string "Rendezvous cluster router active (ADR-001)" for cluster experimental. | Removed. |
| UA-10 | Medium | old `App.jsx:400` | Footer claimed "RFC 9111 COMPLIANT CACHE ENGINE"; `CLAUDE.md` section 6 lists known conformance gaps. Unsupported claim. | Removed. |
| UA-11 | Medium | old `App.jsx` (whole) | 100+ inline styles; fonts of 8 to 10 px; muted text `#666` on `#0B0C0E` is 3.41:1 and `#555` on `#050607` is 2.72:1 (computed), below WCAG AA 4.5:1. | Fixed: tokens, minimum 0.75rem text, contrast script. |
| UA-12 | Medium | old `App.jsx` (whole) | No landmarks (`main`, `nav`), no skip link, no focus styles, no ARIA, fixed `repeat(6,1fr)` grids that do not reflow. | Fixed; see section 5 for limits. |
| UA-13 | Medium | old `App.jsx:20-40` | Provenance conveyed by badge colour plus text only; no shape cue. | Fixed: shape plus text plus border style. |
| UA-14 | Medium | `ui/test/contract.test.mjs:4-36` | The existing contract tests exercise a validator **copied into the test file**, not UI code, so they cannot fail when the UI regresses. | Kept unchanged (not weakened). New tests import the real `src/lib` modules. |
| UA-15 | Low | `package.json` | `lucide-react` declared, zero imports. | Left in place (removing a dependency needs owner decision O8-3). |
| UA-16 | Low | old `App.jsx:109` | Gateway URL taken from the query string with no scheme validation. | Fixed: only http and https are accepted (R: `misc.test.mjs`). |
| UA-17 | Medium | `SimDemo.jsx` (whole) | 109 inline styles, hard-coded fake numbers, fake initial state, second `h1`. Not fixed beyond the heading level: the file is legacy, exempt from the live-code scan, and only mounted under a SIMULATED banner. | Accepted, bannered; heading lowered to `h2`. |
| UA-18 | Low | `ui/dist`, `ui/ui.log` | Generated artefacts present in the tree (gitignored `dist`). | Not touched. |

### 3.2 Backend and contract observations (not fixed here; routed to the backend owners)

| ID | Severity | Evidence | Observation |
|---|---|---|---|
| BD-01 | High | `server.rs:516-547` (`unwrap_or(0)`) | When `state.cache` is `None`, `stale_served_total`, `stale_cap_violations` and all `revalidations.*` are emitted as **measured 0**. The spec says unavailable must be an object with no `v`. A zero here is a fabricated measurement. |
| BD-02 | Medium | `server.rs:327,330` | `hit_ratio_cumulative` is `1.0` with no hits or misses, labelled derived. The UI adds a note; the gateway should emit `unavailable` (`insufficient_window`) instead. Same pattern for counters when the cache is absent (`(0, 0, 0, ...)`, line 330). |
| BD-03 | Medium | `server.rs:434` | `nodes[].state_src` is `"simulated"` in **live** mode for a standalone gateway. The live-mode contract walker checks only keys named `src`, so it does not catch this. The UI shows the node table (cluster experimental only) with a SIMULATED badge, but a live payload carrying simulated data should be a contract failure. |
| BD-04 | Low | `server.rs:491` | `uptime_ms` falls back to `0` without telemetry. |
| BD-05 | High | `server.rs:589-595` | When `require_auth_for_control` is on, `/api/events` demands a bearer token. Browser `EventSource` cannot set headers, so the UI cannot connect to an authenticated gateway. See `ui_api_requirements.md` R-UI-01. |
| BD-06 | Low | `server.rs:102` | CORS is `permissive()`. Acceptable for development; flag for production hardening. |
| BD-07 | Low | `server.rs:200-247` | Intelligence status and audit handlers perform no authentication, and the audit handler returns every retained record unpaginated. |
| BD-08 | Low | `server.rs:197` | `/health` returns `OK` only. A `crate::health` router with `/health/live` and `/health/ready` was merged at `server.rs:121` while this audit was written; its payload was not read and the UI does not call it (R-UI-07). |
| BD-09 | Info | `server.rs:413-427,552` | Concurrent R-01 work added `metrics.origin_cap` with a **boolean** `v` (`enforcing`). The UI initially rejected non-numeric values; it now accepts booleans. The fields have no dedicated tile and are listed generically. |

Line numbers refer to `server.rs` as read on 2026-10-29 during concurrent edits; they may have drifted.

## 4. What changed (M8 implementation)

`ui/src/App.jsx` is now a shell only. New modules: `lib/` (pure, tested: provenance, schema, connection, catalog, viewmodel, format, history, routes, theme, config), `hooks/`, `components/`, `screens/`, `styles/`. Screens: Overview, Traffic & latency, Cache & memory, Freshness, Origin; Cluster, Lab and Intelligence placeholders; the bannered Simulation demo. Navigation is hash routing (`#/traffic`); `?view=sim-demo` still works. Design documents: `principles.md`, `information_architecture.md`, `tokens.md`, `ui_api_requirements.md`.

## 5. Accessibility: what was checked and what was not

| Check | Method | Result |
|---|---|---|
| Colour contrast of every token pair, three theme definitions | `npm run check:contrast` (R) | 102 of 102 pass; see `tokens.md` |
| Static structure: one `h1`, heading order, unique ids, ARIA references, named buttons and links, labelled SVG, table captions and `th scope`, sections named, no positive `tabindex`, no inline colour, provenance badge on every metric | `npm run check:a11y` over 8 screens and 8 data states (64 renders) (R) | 0 issues |
| Keyboard behaviour of the nav arrow keys | Unit test of `nextNavIndex` (R) | Pass; the DOM event wiring itself is **not** tested |
| Focus moves to the screen heading after navigation | Code only (`App.jsx`) | **Not verified in a browser** |
| Visible focus ring, reduced-motion, forced-colors, `prefers-contrast`, 200% zoom, 320 px reflow, touch target size | CSS written to the design; **no browser was available** | **Not verified** |
| Screen reader announcement (`aria-live` region announces connection changes only) | Markup inspected | **Not verified with NVDA, VoiceOver or JAWS** |
| axe-core, Lighthouse, visual regression | Not run: no new dependencies were allowed | **Not run** |
| Colour-vision simulation | Not run; provenance and status use shape and text in addition to colour by construction | **Not verified** |

The lint is a regex and tag tokenizer over server-rendered HTML. It cannot evaluate computed styles, focus order, live behaviour or layout. It is a regression guard, not an accessibility conformance claim.

## 6. Unverified items

Everything visual (layout, spacing, appearance in both themes, responsive behaviour) is unverified because no browser was available. The SSE client code (`useTelemetry.js`) was never run against a live gateway; its reconnect and backoff logic is only covered through the pure reducer tests. Bundle size is 210 kB raw, 65 kB gzip as built on 2026-10-29 (one build, no budget set).
