# SSE authentication options (BD-05 / R-UI-01)

**Date:** 2026-10-29. **Status:** options and recommendation only. Nothing here is implemented. Authentication behaviour is owner-reserved (CLAUDE.md section 2); this document needs an owner decision before any code changes.

## Problem (verified in code)

`/api/events` and `/api/v1/metrics` call `authenticate_request(&headers, sec, Role::Observer)` when `security.require_auth_for_control` is on (`src/api/server.rs`, `sse_handler`, `metrics_handler`). The only accepted credential is the `Authorization` header (`Bearer <jwt>` or a bare token). A browser `EventSource` cannot set request headers, so a UI served to a browser cannot open the stream on a protected gateway. `fetch()` can set the header, so polling `/api/v1/metrics` works; only the push stream is blocked.

Related, owner-reserved and not changed by this document: SEC-M7-01 (anonymous GET), SEC-M7-02 (unauthenticated intelligence endpoints), SEC-M7-03 (permissive CORS). CORS matters for every option below because a cross-origin UI needs `Access-Control-Allow-Credentials` and a non-wildcard origin for cookies, and `Access-Control-Allow-Headers: authorization` for fetch.

Assumed failure/threat model: the gateway is reachable by browsers on an operator network, possibly through a reverse proxy that logs URLs; JWTs are bearer credentials with a role claim (`Observer`); the SSE stream is read-only telemetry.

## Options

### A. Query-string token (`/api/events?token=<jwt>`)
- Change: handler also reads `token` from the query.
- Pros: trivial; works with `EventSource`; no new endpoints.
- Cons: the long-lived JWT lands in browser history, proxy/access logs, `Referer` (if the page navigates), and crash reports. Hard to redact reliably. Any log reader gains an Observer credential for its whole lifetime.
- Mitigation only partial: short JWT lifetime (but then the stream must reconnect with a new token every few minutes, and `EventSource` auto-reconnect reuses the old URL).
- Verdict: weakest. Not recommended.

### B. Cookie session (`Set-Cookie` after login; `EventSource` with `withCredentials`)
- Change: a login endpoint exchanges credentials/JWT for an `HttpOnly; Secure; SameSite=Strict` cookie; SSE and metrics accept the cookie; CSRF defence for any state-changing route that accepts the cookie.
- Pros: no secret in URLs; browser handles expiry/renewal; `HttpOnly` blocks script theft.
- Cons: introduces server-side session state or a signed-cookie format, a login surface, CSRF exposure (all `POST`/`DELETE` control routes must then reject cookie-authenticated requests lacking a CSRF token or custom header), and CORS credentials configuration (cannot use the current permissive wildcard). Largest design and review surface; changes the auth model for control routes too.
- Verdict: appropriate if a full operator console with login is planned. More than this read-only stream needs.

### C. Short-lived single-use stream ticket
- Change: new authenticated endpoint `POST /api/v1/stream-ticket` (Bearer header, `Observer` role) returns an opaque random ticket, TTL about 30 s, bound to the role and optionally the client IP, usable exactly once. `EventSource` connects to `/api/events?ticket=<opaque>`; the server consumes the ticket on connect and then streams without further checks.
- Pros: the real credential never appears in a URL; a leaked ticket is worthless after one use or 30 s; read-only scope; `EventSource` works; no cookies, so no CSRF and no CORS-credentials change (the `POST` is a normal fetch with an `Authorization` header). Small code and test surface: one in-memory map with expiry and a size bound.
- Cons: one more endpoint; the ticket store is per process (fine for a single gateway; in a cluster each node issues its own); auto-reconnect needs the client to fetch a fresh ticket (the UI would implement reconnect itself, which it largely needs anyway for its connection-state display); an open stream outlives token expiry unless the server also enforces a maximum stream lifetime (recommend closing the stream at the JWT `exp` or at a fixed cap, e.g. 15 minutes, so the client re-ticketing re-validates the role).
- Verdict: best fit for read-only SSE.

### D. Fetch-based streaming client (`fetch` + `ReadableStream`, `Authorization` header)
- Change: none on the server. The UI parses the `text/event-stream` body itself (or uses a small, vendored SSE-over-fetch reader).
- Pros: no server auth change at all; the JWT stays in a header; works today; no new attack surface.
- Cons: loses `EventSource` built-ins (automatic reconnect, `Last-Event-ID`); the UI must implement parsing, backoff and abort handling; the UI has a no-new-dependency rule so this is hand-written code to test; some proxies buffer streaming `fetch` bodies differently; the token lives in JS memory (same exposure as for the existing polling calls).
- Verdict: the zero-backend-change option and a sound interim. Needs UI work and tests in `ui/`, not backend work.

## Comparison

| Concern | A query token | B cookie | C ticket | D fetch stream |
|---|---|---|---|---|
| Credential in URL/logs | yes (long-lived) | no | no (single-use, 30 s) | no |
| CSRF exposure | no | yes | no | no |
| CORS change needed | no | yes (credentials, non-wildcard) | no | header allow-list only |
| Server change | small | large | small-medium | none |
| `EventSource` auto-reconnect | yes but stale token | yes | no (re-ticket) | no (hand-written) |
| Token expiry handling | poor | good | good with stream cap | UI handles |
| New attack surface | log leakage | session + CSRF | ticket store (bounded) | none |

## Recommendation (for the owner to decide)

1. **Now / no backend change:** option **D** (fetch-based streaming client in the UI), because it needs no change to authentication behaviour, which is owner-reserved.
2. **If native `EventSource` semantics or simpler clients are wanted:** option **C** (single-use stream ticket), scoped to the read-only `Observer` stream. If approved, require: ticket entropy of at least 128 bits from the OS CSPRNG, single use, TTL at most 30 s, bounded store size with eviction of expired entries, `Observer` role only, no log of the ticket value, a maximum stream lifetime so role/expiry are re-checked, and tests that fail without the change (expired, reused, wrong role, missing).
3. **Avoid A.** Choose **B** only if a login-based operator console is a goal in its own right.

Tests to write if C is approved (each must fail without the feature): ticket issued only with a valid Observer token; second use rejected; use after TTL rejected; ticket from `Observer` cannot reach control routes; SSE closes at the stream cap; ticket value absent from diagnostics.

## Unverified

Proxy buffering behaviour for streaming `fetch` and the exact CORS configuration in `create_app` were not tested here. Cookie attributes above are standard guidance, not verified against this codebase because no cookie auth exists.
