# Run

## Local, no authentication (loopback only)

```
phantom-cache-demo --origin http://127.0.0.1:9000
curl -i http://127.0.0.1:8085/api/v1/cache/some/path
```

A request for `/api/v1/cache/<path>` is served from the in-memory cache or fetched from `ORIGIN_URL` on a miss. `DELETE /api/v1/cache/<path>` invalidates a key (in a cluster, 204 does not prove removal on every gateway; see [limitations.md](limitations.md)). With the default loopback bind no authentication is required. Without `ORIGIN_URL` a miss cannot be filled.

Validate before starting: `phantom-cache-demo --check-config` prints the effective configuration (source of each value, secrets redacted) and exits 0 or 2.

## With authentication and TLS

1. Choose a JWT secret of at least 32 random bytes and give it to the process through the environment, not on the command line: `PHANTOM_JWT_SECRET`.
2. Set `PHANTOM_REQUIRE_AUTH=true`, `PHANTOM_TLS_CERT`, `PHANTOM_TLS_KEY`, and a non-loopback `PHANTOM_BIND_ADDR` if remote access is needed. See `packaging/phantom.production.example.conf` and run with `--mode production`.
3. Clients send `Authorization: Bearer <HS256 JWT>` with claims `iss` (default `phantom-cache-auth`), `aud` (default `phantom-cache-api`), `exp`, optional `tenant_id`, and `roles`.

| Role (claim value) | Allows |
|---|---|
| `cache:read` | `GET /api/v1/cache/*` |
| `cache:invalidate` | `DELETE /api/v1/cache/*` |
| `telemetry:read` | `/api/v1/metrics`, `/api/v1/diagnostics/*`, `/api/events` |
| `scenario:operate` | `POST /api/control` |
| `admin` | all of the above |

The tenant of an authenticated request comes from the token's `tenant_id`; a client-supplied `x-tenant-id` header is ignored in secure mode. Aliases `requester` (read + invalidate), `observer`, `operator` are also accepted. See [security.md](security.md) for known gaps (anonymous reads, unauthenticated intelligence endpoints).

## Endpoints

| Path | Notes |
|---|---|
| `GET /health` | Plain `OK` |
| `GET /health/live`, `GET /health/ready` | Liveness / readiness JSON; see [operate.md](operate.md) |
| `GET/DELETE /api/v1/cache/*key` | Cache read / invalidate |
| `GET /api/v1/metrics` | Schema-v2 metrics; each value labeled with its source (`measured`, `unavailable`, ...) |
| `GET /api/v1/diagnostics/summary`, `/events` | Counters and recent events |
| `GET /api/v1/intelligence/status`, `/audit` | Intelligence mode and audit records |
| `POST /api/control`, `GET /api/events` | Dashboard scenario control and SSE stream. Parts of the stream are SIMULATED unless a value is explicitly labeled measured |

## Dashboard

Optional React UI in `ui/`. If you enable `PHANTOM_SIMULATION=true` the figures it shows are synthetic.
