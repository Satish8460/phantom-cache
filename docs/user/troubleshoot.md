# Troubleshoot

Run `phantom-cache-demo --check-config` first; it reports every configuration problem at once.

| Symptom | Cause and fix |
|---|---|
| `[CONFIG ERROR] PHANTOM_REQUIRE_AUTH: invalid boolean 'yes'` | Booleans accept only true/false/1/0. Fix the spelling. |
| `... network-accessible address ... PHANTOM_JWT_SECRET is not configured` | A non-loopback bind needs JWT auth and TLS. Set the secret and certificate/key, or bind to loopback. |
| `secret is N bytes; at least 32 bytes are required` | Use a longer random secret. |
| `ORIGIN_URL: rejected by the upstream security policy` | The origin violates SSRF/port/allowlist/TLS policy. Use https and allowlist it; plaintext non-loopback needs `PHANTOM_ALLOW_INSECURE_UPSTREAM=true` plus an allowlist. |
| `cluster mTLS ...` errors | Cluster rules apply only if any cluster key is set. Remove stray cluster keys or supply the full mTLS set plus members file and listen address. |
| `file '...' cannot be read` | A referenced PEM/JSON path is wrong or unreadable by the service account. |
| `WARNING: unknown environment variable PHANTOM_...` | A misspelled key is ignored. Compare with configuration.md. |
| `WARNING: PHANTOM_DEFAULT_TTL_MS ... not honored` | That key is validated but the runtime still uses 60000 ms. |
| 401 with `X-Auth-Error` ExpiredToken / InvalidIssuer / InvalidAudience / MissingToken / InvalidToken | Fix the token. HS256 only; `exp` is required. |
| 403 with `X-Auth-Error: InsufficientPermissions` | The token lacks the role (see run.md). |
| `X-Cache-Error` `SsrfBlocked` (502), `UpstreamTlsFailed` (502) or `InvalidOrigin` (400) on a miss | `SsrfBlocked`: origin not allowlisted, plaintext upstream not allowed, or a blocked destination. `UpstreamTlsFailed`: upstream TLS verification failed. `InvalidOrigin`: invalid origin URL, disallowed scheme, or disallowed/sensitive port (`src/api/server.rs`, `map_origin_error_to_response`). |
| 503 `CapacityExceeded` | Shard capacity or the origin cap; check `metrics.origin_cap.origin_cap_rejections`. |
| `/health/ready` 503 | See the `checks` object: `config_valid`, `listening`, `draining`. |
| Windows build: link error 1104/1108 or "No space left on device" | The system drive (TEMP) is full. Point `TMP`/`TEMP` at another drive and use `-j 1`. |
| Port already in use | Another process holds the bind address; choose another with `--bind`. |
