# Security guide

Full review with evidence: [../security/m7_security_review.md](../security/m7_security_review.md).

## Controls

- Ingress: HS256 JWT (issuer, audience and expiry required), role-based access on every data and telemetry route when enabled, CORS allowlist (production default: no cross-origin access; development default: the local Vite origins), per-tenant key namespacing, TLS 1.3 option, fail-closed startup rules for network binds.
- Upstream: SSRF filter applied to every resolved address (metadata, link-local, private ranges, IPv4-mapped/6to4/NAT64 embeddings), blocked sensitive ports, origin allowlist, TLS verification.
- Cluster (experimental): mutual TLS 1.3, SAN identity, membership registry with certificate fingerprints, monotonic registry epochs.
- Secrets are never printed by the effective-configuration report, `Debug` output or configuration errors.

## Hardening checklist

1. Use `--mode production`; it refuses insecure overrides and simulation.
2. Supply `PHANTOM_JWT_SECRET` (32+ random bytes) from a secret store through the environment. Rotate by restarting with a new value; existing tokens become invalid.
3. Terminate TLS in the process or in front of it; restrict the private-key file to the service account.
4. Set `ORIGIN_URL` to an https origin and list it in `PHANTOM_ORIGIN_ALLOWLIST`; set `PHANTOM_ENFORCE_UPSTREAM_TLS=true`.
5. Leave `PHANTOM_FORWARD_UPSTREAM_AUTH` and `PHANTOM_ALLOW_LAN_ORIGIN` off unless required.
6. Place the gateway behind a reverse proxy or network policy that restricts `/api/control`. Leave `PHANTOM_CORS_ORIGINS` unset unless a browser UI on another origin needs access; list exact origins only.
7. Keep intelligence `off`.

## Access control by route (secure mode: `PHANTOM_JWT_SECRET` set)

| Route | Required |
|---|---|
| `GET /api/v1/cache/*` | valid JWT with `cache:read` (401 missing/invalid with `WWW-Authenticate: Bearer`, 403 lacking the role). Checked before any cache lookup, origin fetch or accounting, including with `x-cache-bypass`. The tenant is the token's `tenant_id`; `x-tenant-id` is ignored. |
| `DELETE /api/v1/cache/*` | `cache:invalidate` |
| `GET /api/v1/metrics`, `/api/events`, `/api/v1/diagnostics/*` | `telemetry:read` (Observer) |
| `GET /api/v1/intelligence/status`, `/audit` | `telemetry:read` (Observer) |
| `POST /api/control` | `scenario:operate` |
| `/health`, `/health/live`, `/health/ready` | none (public by design) |

Without `PHANTOM_JWT_SECRET` (development mode) there is no authentication on any route; use only on loopback.

**Caching of authenticated reads.** `main.rs` sets `enforce_rfc9111_auth_bypass` equal to `PHANTOM_FORWARD_UPSTREAM_AUTH`.
- Default (client `Authorization` is NOT forwarded upstream, `PHANTOM_FORWARD_UPSTREAM_AUTH=false`): the origin response cannot depend on the credential, so authenticated reads use the normal per-tenant cache path. Authentication and `cache:read` are still checked on every request, hit or miss. A response is stored only if RFC 9111 section 3.5 lets a shared cache store a response to an authenticated request (`Cache-Control: public`, `s-maxage` or `must-revalidate`); otherwise every read goes to the origin. **This is a behaviour change from the earlier release, where a JWT gateway never cached authenticated reads.**
- `PHANTOM_FORWARD_UPSTREAM_AUTH=true` (credential forwarded, so the origin may personalise per user): every authenticated read is served `X-Cache-Status: BYPASS-AUTH` and never stored.

**Tenants.** In secure mode the cache namespace is the token's `tenant_id`. `GET`/`DELETE /api/v1/cache/*` require a non-empty `tenant_id` claim: a token without one, or whose claim is the reserved value `__default__`, is rejected with **403** (`X-Auth-Error: MissingTenant` / `ReservedTenant`; the principal is authenticated but has no usable tenant). Tokens for observer/operator routes (metrics, diagnostics, events, intelligence, control) do not need a tenant.

**Request-target safety.** A cache path or query containing a control character, space or DEL (for example `%0d%0a`, `%20`, `%00`) is rejected with 400 `InvalidPath` and never reaches the origin. The upstream request target is additionally percent-encoded to pure ASCII (non-ASCII bytes, brackets and similar are written as `%XX`).

**Error responses.** 401 bodies are generic (`Authentication failed`); the detail is not echoed. `X-Auth-Error` distinguishes expired / wrong issuer / wrong audience only for tokens whose signature verified; everything else is `InvalidToken`. `/api/control` parses its JSON body before authentication (the extractor runs first, a malformed body gets a 4xx without a token check); it changes nothing before auth succeeds. Cache `GET`/`DELETE` handlers read (and ignore) a request body before authentication, bounded by the 512 KiB ingress body limit (413 above it).

### Intelligence endpoints

`GET /api/v1/intelligence/status` returns exactly: `status`, `intelligence_mode` (name), `active_candidate` (candidate id or null), `revert_count`, `truthful` (computed: simulation not enabled) and `truthful_basis` (fixed text). No secrets, key material, request keys or tenant data. A test fails if the field set changes.

`GET /api/v1/intelligence/audit?limit=&offset=` returns decision records (trigger reason, candidate id/kind/parameters/bounds, estimated effect and interval, guardrail result, verdict, explanation, configuration hashes) to an Observer. Records are ordered oldest first (ascending `decision_id`) among the retained records (capacity 100); `offset` counts from the oldest retained record. `limit` defaults to 50 and must be 1..=100; `offset` defaults to 0 and must be a non-negative decimal integer. Anything else is HTTP 400; an offset past the end is HTTP 200 with an empty `records` array. The response also carries `total_records` (lifetime counter), `retained_records`, `limit`, `offset`, `returned` and `ordering`. Cost is bounded by the retained record count.

### CORS

CORS is a browser-origin control, not authentication or authorization. Production default: no cross-origin access. The DEVELOPMENT profile (the default `PHANTOM_MODE`) allows `http://localhost:3000` and `http://127.0.0.1:3000` unless `PHANTOM_CORS_ORIGINS` is set (an explicitly empty value means none); a warning is printed when those defaults are active on a non-loopback bind. The gateway also exposes `WWW-Authenticate` and `X-Auth-Error` to allowed origins so a UI can read auth errors. Allow specific browser origins with `PHANTOM_CORS_ORIGINS` (see [configuration.md](configuration.md#cors-allowlist-phantom_cors_origins)); no wildcard, no credentials. A cross-origin web UI must send `Authorization: Bearer <jwt>` with `fetch` (a browser `EventSource` cannot, so stream SSE with `fetch`). Anything that can reach the port without a browser (curl, scripts, another server) is not restricted by CORS.

### Origin contract: what the gateway sends upstream (round 3)

The cache key is **tenant + path + query**. Therefore **origins must not vary their responses on client-controlled request headers** (`x-user-id`, `x-forwarded-*`, `Accept`, `Cookie`, ...): a variant produced for one user would be stored under the tenant key and served to every user of that tenant, and concurrent misses of different users are coalesced into one origin fetch whose answer all of them receive. To make this safe by construction the gateway forwards **only gateway-controlled headers**:

- a fixed `Accept: application/json` (the client's `Accept` is not forwarded);
- `x-tenant-id` set from the cache key (the token's tenant in secure mode; never the client's value; omitted for the default tenant in development mode);
- `x-request-id` only if the client value matches `[A-Za-z0-9._-]{1,64}` (otherwise dropped);
- `Authorization` only when `PHANTOM_FORWARD_UPSTREAM_AUTH=true` (and then authenticated reads are not cached, `BYPASS-AUTH`);
- the gateway's own cache validators (`If-None-Match` / `If-Modified-Since` from the stored entry).

Every other client header, including all other `x-*` headers, `Cookie` and `x-cache-bypass`, is never forwarded. `Set-Cookie` from an origin is not stored or replayed on a cache hit. **Behaviour change:** earlier builds forwarded every client `x-*` header and `Accept`; an origin that relied on them (for example a test origin steered by an `x-mock-*` header) must be steered by path or query instead.

### Request-target rules (round 3)

A cache path or query containing a control character, space or DEL is rejected (400). A path containing a `.` or `..` segment (including `%2e%2e`) is rejected (400) because an origin would normalise it and the same resource could be cached under several keys. The upstream target is percent-encoded to pure ASCII; a literal `%` in the decoded path is written as `%25`, so a client `%252F` reaches the origin as `%252F`. `/api/v1/cache//x` and `/api/v1/cache/x` are the same key (`/x`); `/api/v1/cache///x` is a distinct key (`//x`). Responses in the cache-bypass path are re-framed by the gateway: the origin's `Content-Length`, `Transfer-Encoding`, `Connection`, `Keep-Alive` and other hop-by-hop headers are not copied.

Cluster peers: a `GetRequest` key from a peer is validated like an HTTP key and a key starting with `/` or containing a dot segment is answered `ERR 400` (it would otherwise address the `__default__` tenant's HTTP entries).

## Known gaps (from the M7 review; not fixed in this release)

- In development mode (no `PHANTOM_JWT_SECRET`) every route, including cache reads and intelligence endpoints, is anonymous.
- The `requester` role alias grants both read and invalidate, and a bare token without the `Bearer ` prefix is accepted (SEC-M7-09).
- The SSRF filter does not block a few unusual special-purpose ranges (SEC-M7-10).
- No rate limiting on authentication failures and no token revocation list (SEC-M7-11). Denied cache reads are not counted in a metric.
- No dependency advisory scan was run (it needs owner-approved tooling).
- The independent review required by `CLAUDE.md` for the SEC-M7-01/02/03 changes is pending.

## Lab HTTP API (opt-in)

`PHANTOM_LAB_ENABLED=true` adds `/api/v1/lab` (starts experiments, spawns gateway child processes, serves retained results). Off by default. With JWT: reads need Observer, start/cancel need Operator; without JWT it is anonymous and allowed only on a loopback bind. See `docs/user/lab_api.md` (security model, limits, known gaps). The independent review of this surface is pending.
