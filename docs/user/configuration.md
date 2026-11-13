# Configuration

PhantomCache is configured through a validated, centralized layer (`src/config.rs`).

**Sources and precedence** (highest first): command-line flag, environment variable, `--config` file, built-in default. An empty value counts as "not set" at that layer.

**Fail-fast.** Invalid values, conflicting settings and unreadable files stop startup with exit code 2 and one `[CONFIG ERROR]` line per problem. Unknown `PHANTOM_*` environment variables produce a warning; unknown keys in a configuration file are errors. Booleans accept only `true`, `false`, `1`, `0` (case-insensitive): any other word (for example `yes`) is rejected. Before this layer existed such a typo was silently read as `false`, which could disable `PHANTOM_REQUIRE_AUTH`.

## Command line

| Flag | Equivalent key | Meaning |
|---|---|---|
| `--config <file>` | `PHANTOM_CONFIG` | Read `KEY=VALUE` lines from a file |
| `--bind <ip:port>` | `PHANTOM_BIND_ADDR` | Ingress listen address |
| `--mode <profile>` | `PHANTOM_MODE` | `development` or `production` |
| `--origin <url>` | `ORIGIN_URL` | Origin base URL |
| `--check-config` | none | Validate, print the effective configuration, exit 0 (valid) or 2 (invalid); the server is not started |
| `--version`, `--help` | none | Version and build stamp; usage |

## Configuration file

Plain `KEY=VALUE` lines using the exact key names below. `#` starts a full-line comment; an optional `export ` prefix and surrounding double quotes are accepted. No other syntax (no TOML/YAML, no inline comments). `PHANTOM_CONFIG` cannot appear inside a file. Keep files that contain `PHANTOM_JWT_SECRET` readable only by the service account. An example is in `packaging/phantom.example.conf`.

## Effective-configuration report

At startup (and with `--check-config`) the process prints every key with its resolved value and source (`default`, `file`, `env`, `cli`). Secrets print as `<redacted>`; unset values print as `(unset)`; keys that are validated but not honored by the runtime are marked `(not honored by runtime)`. Warnings are listed at the end. The report is also what `Config` prints with `{:?}`: secrets never appear.

## Key reference

"applied by main" means the validated value is applied directly by the binary. "env (exported)" means code in the server still reads the process environment, so the binary exports the validated, source-merged value before starting. "NOT HONORED" means the value is validated and reported but the runtime ignores it today.

| Key | Type/unit | Default | Wiring | Security impact |
|---|---|---|---|---|
| `PHANTOM_MODE` | enum `development`, `production` | `development` | applied by main | high: `production` enforces strict fail-closed rules |
| `PHANTOM_BIND_ADDR` | ip:port (IP literal) | `127.0.0.1:8085` | env (exported) | high: non-loopback requires JWT auth and TLS |
| `PHANTOM_TLS_CERT` | file path (PEM chain) | unset | env (exported) | high; set together with `PHANTOM_TLS_KEY` |
| `PHANTOM_TLS_KEY` | file path (PEM key) | unset | env (exported) | high: restrict file permissions |
| `PHANTOM_ALLOW_INSECURE_HTTP` | bool | `false` | env (exported) | high: plaintext ingress on a network interface, lab use only; JWT auth stays mandatory |
| `PHANTOM_REQUIRE_AUTH` | bool | `false` | env (exported) | high: refuse to start without `PHANTOM_JWT_SECRET` |
| `PHANTOM_JWT_SECRET` | secret, at least 32 bytes | unset | applied by main | critical: HS256 key; never printed |
| `PHANTOM_JWT_ISSUER` | string | `phantom-cache-auth` | applied by main | medium |
| `PHANTOM_JWT_AUDIENCE` | string | `phantom-cache-api` | applied by main | medium |
| `PHANTOM_MAX_CACHE_BYTES` | bytes, 1.. | `268435456` | applied by main | medium: bounds memory |
| `PHANTOM_DEFAULT_TTL_MS` | ms, 1..31536000000 | `60000` | NOT HONORED | low: `src/api/server.rs` passes a literal 60000 to the freshness engine (the two `60_000` call sites, currently `:1405` and `:1688`); setting this key has no runtime effect, the effective default lifetime is always 60 s and applies only when the origin sends no freshness information |
| `PHANTOM_MAX_STALE_MS` | ms, 1..31536000000 | `86400000` | applied by main | medium: hard stale-serving cap (24 h); caps only the stale window, not the freshness lifetime an origin sets with `s-maxage`/`max-age`/`Expires` (no gateway ceiling) |
| `PHANTOM_SWEEPER_BATCH_SIZE` | shards per tick, 1..1024 | `32` | applied by main | low: above 32 warns |
| `PHANTOM_SWEEPER_INTERVAL_MS` | ms, 1..3600000 | `100` | applied by main | low |
| `ORIGIN_URL` | http(s) URL | unset | env (exported) | high: checked against the SSRF policy at startup |
| `PHANTOM_ORIGIN_ALLOWLIST` | comma-separated URLs | unset | env (exported) | high: when set, the origin must match |
| `PHANTOM_ENFORCE_UPSTREAM_TLS` | bool | `false` | env (exported) | high: reject http origins |
| `PHANTOM_ALLOW_INSECURE_UPSTREAM` | bool | `false` | env (exported) | high: plaintext upstream; needs an allowlist on network binds |
| `PHANTOM_ALLOW_LAN_ORIGIN` | bool | `false` | env (exported) | high: relaxes SSRF filter for private ranges |
| `PHANTOM_FORWARD_UPSTREAM_AUTH` | bool | `false` | env (exported) | high: forwards client Authorization upstream; when true every authenticated read is BYPASS-AUTH (never cached); when false, shared-cacheable (public / s-maxage / must-revalidate) authenticated reads are cached per tenant |
| `PHANTOM_UPSTREAM_CA_CERT` | file path (PEM) | unset | env (exported) | medium: extra trusted CA for upstream TLS |
| `PHANTOM_CLUSTER_ID` | label `[a-z0-9-]{1,32}` | `phantom-cluster` | env (exported) | medium: must equal the SAN cluster label |
| `PHANTOM_CLUSTER_MEMBERS_FILE` | file path (JSON) | unset | env (exported) | high: authoritative membership and fingerprints |
| `PHANTOM_CLUSTER_CA` | file path (PEM) | unset | env (exported) | high |
| `PHANTOM_NODE_CERT` | file path (PEM) | unset | env (exported) | high |
| `PHANTOM_NODE_KEY` | file path (PEM key) | unset | env (exported) | high: restrict file permissions |
| `PHANTOM_NODE_ID` | integer 1..255 | `1` | env (exported) | medium |
| `PHANTOM_CLUSTER_LISTEN_ADDR` | ip:port | unset | env (exported) | high: required when cluster mTLS is configured |
| `PHANTOM_CLUSTER_PEER_ADDRS` | `id=ip:port,...` | unset | env (exported) | medium |
| `PHANTOM_ALLOW_INSECURE_CLUSTER` | bool | `false` | env (exported) | high: plaintext cluster, loopback development only |
| `PHANTOM_STRICT_CLUSTER` | bool | `false` | env (exported) | medium: on loopback require full mTLS material |
| `PHANTOM_CLUSTER_TIMEOUT_MS` | ms, 1..600000 | `150` | env (exported) | low |
| `PHANTOM_INTELLIGENCE` | enum `off`, `observe`, `advisory`, `canary`, `active` | `off` | applied by main | medium: `canary` and `active` are REJECTED at startup (exit 2) unless `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true`; benefit is not established |
| `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE` | bool | `false` | applied by main | high: explicit opt-in for `canary`/`active`; used only by the research lab |
| `PHANTOM_SIMULATION` | bool | `false` | env (exported) | medium: enables SYNTHETIC dashboard values |
| `PHANTOM_ORIGIN_CAP_MAX` | concurrent origin fetches, 1.. | unset | applied by main | medium: unset means the cap is not configured |
| `PHANTOM_ORIGIN_CAP_MIN` | concurrent origin fetches, 1.. | `1` | applied by main | low: must be at most `PHANTOM_ORIGIN_CAP_MAX` |
| `PHANTOM_ORIGIN_CAP_P99_CEILING_MS` | ms, 1..600000 | `1000` | applied by main | low |
| `PHANTOM_ORIGIN_CAP_WINDOW` | origin observations, 1.. | `100` | applied by main | low |
| `PHANTOM_CORS_ORIGINS` | comma-separated exact origins `scheme://host[:port]`, max 16; empty = none | unset (development profile: `http://localhost:3000`, `http://127.0.0.1:3000`; production: none) | applied by main | medium: lists browser origins that may read responses cross-origin; it is not authentication |
| `PHANTOM_LAB_ENABLED` | bool | `false` | applied by main | high: exposes `/api/v1/lab` (starts experiments, spawns processes); with JWT reads need Observer and start/cancel need Operator; without JWT only on a loopback bind (startup error otherwise). See `docs/user/lab_api.md` |
| `PHANTOM_LAB_DIR` | directory path | `lab-results` | applied by main | medium: runs are written under `<dir>/runs`; retained experiments under `<dir>` are read read-only; ignored unless `PHANTOM_LAB_ENABLED=true` |
| `PHANTOM_LAB_GATEWAY_EXE` | file path (must exist) | unset (this executable) | applied by main | high: executable spawned by `scenario_b` runs; a debug binary makes the run EXPLORATORY |
| `PHANTOM_CONFIG` | file path | unset | applied by main | low: same as `--config` |

## Origin concurrency cap

`PHANTOM_ORIGIN_CAP_MAX` is a **static cap on concurrent origin fetches, enforced in every intelligence mode** (`off`, `observe`, `advisory` and the experimental modes). It is a plain counter compared with a fixed limit in the server layer (`src/api/server.rs`): not adaptive, not a policy, not part of the Phantom Intelligence engine. Only the single-flight leader, a bypass request or a background revalidation takes a permit; coalesced waiters never do. A request that finds the cap full is rejected at once with HTTP 503 and is not counted as an origin dispatch. Without `PHANTOM_ORIGIN_CAP_MAX` the origin path is uncapped (the default). Read the live state in `metrics.origin_static_cap` (`limit`, `in_flight`, `rejections`, all measured).

Additionally, `PHANTOM_ORIGIN_CAP_MAX` is the upper bound of the experimental AIMD controller (`PHANTOM_ORIGIN_CAP_MIN`, `_P99_CEILING_MS`, `_WINDOW`), which is enforced only in `canary` or `active`; that part is inert otherwise and startup says so. A benefit of the adaptive controller has not been established (M6 H1 is NOT_SUPPORTED).

A request rejected by the cap returns HTTP 503 with the same body and header as a shard-capacity rejection (`CapacityExceeded`). To tell them apart, read `metrics.origin_cap.origin_cap_rejections` in `GET /api/v1/metrics`: if it increments, the cap rejected the request.

## Cross-field rules (fail closed)

- Non-loopback `PHANTOM_BIND_ADDR` requires `PHANTOM_JWT_SECRET` and TLS (`PHANTOM_TLS_CERT` + `PHANTOM_TLS_KEY`), or `PHANTOM_ALLOW_INSECURE_HTTP=true` (auth still required).
- `PHANTOM_TLS_CERT` and `PHANTOM_TLS_KEY` must be set together.
- `PHANTOM_REQUIRE_AUTH=true` requires `PHANTOM_JWT_SECRET`.
- A JWT secret shorter than 32 bytes is an error when auth is required, the bind is a network address, or mode is `production`; otherwise a warning.
- Non-loopback bind with `PHANTOM_ALLOW_INSECURE_UPSTREAM=true` requires `PHANTOM_ORIGIN_ALLOWLIST`.
- `ORIGIN_URL` must pass the same upstream policy used for every request.
- Cluster rules apply only when clustering is requested (members file, listen address, any cluster certificate, or `PHANTOM_ALLOW_INSECURE_CLUSTER`). A single network-bound node needs no cluster material. Partial mTLS material is an error; complete mTLS needs `PHANTOM_CLUSTER_MEMBERS_FILE` and `PHANTOM_CLUSTER_LISTEN_ADDR`; a non-loopback bind with clustering requires full mTLS.
- `production` mode requires `PHANTOM_REQUIRE_AUTH=true`, a JWT secret, `PHANTOM_SIMULATION=false`, and forbids `PHANTOM_ALLOW_INSECURE_HTTP`, `PHANTOM_ALLOW_INSECURE_UPSTREAM`, `PHANTOM_ALLOW_INSECURE_CLUSTER` and `PHANTOM_FORWARD_UPSTREAM_AUTH`.

## Production defaults

The defaults are safe for local use (loopback bind, no simulation, intelligence off, no origin cap). For a network deployment run `--mode production`; see `packaging/phantom.production.example.conf`.

## CORS allowlist (`PHANTOM_CORS_ORIGINS`)

CORS is a **browser-origin control, not authentication or authorization.** It only tells a browser which web pages may read responses from the gateway. curl, scripts and any non-browser client ignore it entirely; every request still needs a valid JWT when auth is enabled.

- **Production default: no cross-origin access.** (The development profile differs, see the next item.) No `Access-Control-Allow-Origin` header is sent for any `Origin` (same-origin only). Requests without an `Origin` header (curl, same-origin pages) are unaffected.
- **Development profile** (`PHANTOM_MODE=development`, which is the DEFAULT mode, so an unconfigured gateway is in it) allows exactly the Vite dev-server origins `http://localhost:3000` and `http://127.0.0.1:3000` when `PHANTOM_CORS_ORIGINS` is unset (port from `ui/vite.config.js`). Set the variable to replace that list. **Set but empty (`PHANTOM_CORS_ORIGINS=`) means no cross-origin access, also in development**; unset keeps the profile default. Startup prints a warning when the development defaults are active on a non-loopback bind.
- **Production profile** has no default allowlist and never falls back to a permissive policy.
- **Format:** comma-separated exact origins `scheme://host[:port]`; `http` or `https` only; no `*` wildcard (not even in a host), no path (including a trailing `/`), query, fragment or userinfo; no empty entries; at most 16 entries. Hosts are lowercased and default ports dropped, so the value matches the browser's `Origin` header. Any violation is a startup error in every profile (exit code 2), aggregated with other configuration errors.
- **What is sent for an allowed origin:** `Access-Control-Allow-Origin: <that origin>` and `Vary: Origin`; allowed methods `GET, POST, DELETE, OPTIONS`; allowed request headers `Authorization, Content-Type, x-tenant-id, x-cache-bypass`; exposed response headers `X-Cache-Status, Age, Retry-After, X-Cache-Error, X-Cache-Warning, WWW-Authenticate, X-Auth-Error, Content-Type`; `Access-Control-Max-Age: 600`. **`Access-Control-Allow-Credentials` is never sent** (bearer header only, no cookies).
- `--check-config` prints an `effective CORS origins:` line stating the list and whether it came from the variable or the profile default.

## Upstream request headers (origin contract)

The cache key is tenant + path + query. Origins must not vary responses on client-controlled request headers. The gateway forwards only a fixed `Accept: application/json`, `x-tenant-id` (from the key), a validated `x-request-id` (`[A-Za-z0-9._-]{1,64}`), the stored validators, and `Authorization` only when `PHANTOM_FORWARD_UPSTREAM_AUTH=true`. Client `x-*` headers, `Accept` and `Cookie` are never forwarded. See [security.md](security.md#origin-contract-what-the-gateway-sends-upstream-round-3).
