# M7 Security Review

Scope: ingress JWT/RBAC routes, upstream SSRF validation, cluster mTLS controls (`src/security/**`, `src/api/server.rs` auth call sites, `src/main.rs` startup), and the tests that cover them. Method: reading the code and running the tests; evidence tags follow `CLAUDE.md` section 4 (verified in code with `file:line`, reproduced here). Line numbers are for the working tree at review time (HEAD `456bb25`, large uncommitted tree) and will drift.

Not done: penetration testing, fuzzing, dependency advisory scan (`cargo audit`/`cargo deny` need owner approval, O7-6), TLS configuration scanning, review of `ui/`, review of the origin-fetch HTTP parser beyond the existing tests, multi-host cluster testing. A review by the author of the changes is not independent.

## 1. Results summary

| ID | Severity | Status | Finding |
|---|---|---|---|
| SEC-M7-04 | High (release blocker) | **Fixed** (`src/main.rs`, `src/config.rs`) | `main` called `validate_cluster_startup_security` unconditionally; for any non-loopback bind it demands cluster mTLS material, so a single (non-cluster) network-facing node could never start. Now only evaluated when clustering is requested. |
| SEC-M7-05 | High | **Fixed** (`src/config.rs`) | Booleans such as `PHANTOM_REQUIRE_AUTH=yes` were silently read as `false` (`v == "true" \|\| v == "1"`), disabling auth on a typo. Unrecognized values are now a startup error. |
| SEC-M7-06 | Medium | **Fixed** (`src/config.rs`) | No minimum JWT secret length. HS256 secrets under 32 bytes are now an error when auth is required, the bind is a network address, or mode is `production`; otherwise a warning. |
| SEC-M7-07 | Medium | **Fixed** (`src/security/upstream.rs`) | `validate_target` compared `host_str()` with `"::1"`, but the `url` crate returns `"[::1]"`, so loopback IPv6 (and `127.0.0.0/8` other than `127.0.0.1`) origins were never recognized as loopback. Fail-closed functional defect (valid origin refused), not a bypass. Now uses the parsed host. Test `m7_upstream_policy_loopback_literals_follow_the_same_rule` failed before the change. |
| SEC-M7-08 | Low | **Fixed** (`src/config.rs`, `src/main.rs`) | Invalid `PHANTOM_BIND_ADDR` silently fell back to `127.0.0.1:8085`; invalid `PHANTOM_MAX_CACHE_BYTES`/`PHANTOM_NODE_ID` fell back to defaults. All are now errors (fail-fast). |
| SEC-M7-01 | Medium | **Fixed** (`src/api/server.rs` `get_cache_handler`; owner-approved Alternative A; **independent review pending**) | With JWT enabled, `GET /api/v1/cache/*` now requires a valid token with `cache:read`, checked first (before tenant resolution, counters, cache lookup, coalescing, permit acquisition, origin fetch), including on the `x-cache-bypass` path. Missing/malformed/expired/wrong-aud/wrong-iss/bad-signature -> 401 with `WWW-Authenticate: Bearer`; valid token without `cache:read` -> 403. Tenant comes only from the token. Before: anonymous requests were served for the default tenant and could trigger origin fetches. Tests: `m7_sec_fixes_tests::sec01_*` (10 tests: valid read + HIT, every 401 case with zero origin requests counted at a raw-TCP origin and unchanged counters, 403 roles, bypass header, root path, tenant A/B isolation and anonymous-cannot-read, SEC-M7-09 raw-token characterization, default-config `BYPASS-AUTH` characterization, dev mode unchanged, health public) and `m7_security_review_tests::m7_fix_anonymous_cache_read_is_rejected_in_secure_mode` (was `m7_finding_anonymous_cache_read_is_not_rejected_in_secure_mode`, which asserted `status != 401`). Residual: anonymous access remains in development mode (no secret); denied reads are not counted in a metric; 401 responses reveal which check failed via `X-Auth-Error` (existing convention); with the default `enforce_rfc9111_auth_bypass=true` authenticated reads are always `BYPASS-AUTH` (pre-existing, `main.rs` does not change it). |
| SEC-M7-02 | Medium | **Fixed** (`src/api/server.rs` `intelligence_*_handler`; Alternative A; **independent review pending**) | With security enabled both endpoints require Observer (`telemetry:read`, or admin) with the same policy as metrics/diagnostics: 401 missing/invalid, 403 insufficient role; checked before query validation. `/audit` takes `limit` (default 50, 1..=100) and `offset` (>= 0); invalid values -> 400; offset past the end -> 200 with empty `records` and totals; ordering is oldest-first among retained records; the handler no longer loops over the lifetime counter (it was O(total x retained)), so cost is bounded by the store capacity (100). `/status` exposes only `status`, `intelligence_mode`, `active_candidate`, `revert_count`, `truthful`, `truthful_basis` (a test pins the field set). Tests: `m7_sec_fixes_tests::sec02_*` (5 tests) and `m7_fix_intelligence_endpoints_require_observer_in_secure_mode` (was `m7_finding_...unauthenticated...`, which asserted neither 401 nor 403). Residual: audit records (trigger reasons, candidate parameters, config hashes) are visible to any Observer; anonymous access in development mode; no rate limiting (SEC-M7-11). |
| SEC-M7-03 | Low | **Fixed** (`src/api/server.rs` `CorsConfig`/`create_app_with_cors`, `src/config.rs` `PHANTOM_CORS_ORIGINS`, `src/main.rs`; Alternative A; **independent review pending**) | `CorsLayer::permissive()` replaced by an exact-origin allowlist (default: no cross-origin access; development profile: `http://localhost:3000`, `http://127.0.0.1:3000`; production: none). Methods GET/POST/DELETE/OPTIONS; request headers Authorization, Content-Type, x-tenant-id, x-cache-bypass; exposed X-Cache-Status, Age, Retry-After, X-Cache-Error, Content-Type; max-age 600; never `Allow-Credentials`. Validation (no wildcard/path/query/fragment/userinfo, http/https, no empty entries, max 16) fails startup with exit code 2 in every profile. `main` builds the policy from the resolved config and starts the server with `start_server_with_cors`. Tests: `m7_sec_fixes_tests::sec03_*` (7 tests, 6 of them on the real router: default none, allowed origin + Vary + expose, disallowed origins, preflight for Authorization, no-Origin unaffected, authz still enforced, validation) and `m7_config_tests::cors_origins_*` (3 tests) and `m7_fix_cors_is_same_origin_by_default` (was `m7_finding_cors_is_permissive`, which asserted `access-control-allow-origin: *`). Residual: CORS is a browser control only and does not restrict non-browser clients; the end-to-end `main` -> router wiring is covered by construction (`CorsConfig::with_origins(cfg.cors_origins())`) and unit tests, not by a spawned-binary test; a dev-profile gateway bound to a network address also allows the localhost Vite origins (harmless for other hosts, set the variable to override). |
| SEC-M7-09 | Low | Open (documented) | Role alias `requester` grants both `cache:read` and `cache:invalidate` (`src/security/jwt.rs:57`). `authenticate_request` also accepts a raw token without the `Bearer ` prefix (`src/api/server.rs:704` onward). |
| SEC-M7-10 | Low | Open | SSRF filter does not block 198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24 or Teredo (2001::/32) (`src/security/upstream.rs:136`, `:208`). Resolution still filters every returned address, and these ranges are unusual on the public internet. Not fixed because the owner reserves `src/security` hardening beyond release blockers. |
| SEC-M7-11 | Low | Open | No rate limiting or lockout for failed authentication; no token revocation (`jwt.rs` validates signature, iss, aud, exp only; `iat` is parsed but unused; no `nbf`). |
| PERF-M7-01 | Not a security finding; High for operations | Open (reported) | Idle process CPU (single runs, Windows 11, no traffic): debug build about 6.5 CPU-seconds in about 5 s wall; release build (clean-room bundle) 14.78 CPU-seconds in 10 s wall. Probable cause by code reading: `start_server` drops the sweeper shutdown sender immediately, making `shutdown.changed()` always ready in `spawn_sweeper_task`. Not isolated experimentally. See `docs/release/deferred_integration.md` D-2. |

Zero open Critical. Open High: none after the fixes above. Medium/Low findings need the owner's written disposition (AC7-5).

## 2. Control-by-control evidence

### 2.1 Ingress authentication (JWT)

| Control | Evidence | Tests |
|---|---|---|
| HS256 only; other algorithms rejected | `src/security/jwt.rs:153` | `m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering` (alg none, HS384) |
| `iss`, `aud`, `exp` validated with 10 s leeway; explicit re-check of iss/aud | `jwt.rs:169-171` and after | `test_phase1_at03_and_at07_unauthorized_or_invalid_jwt_rejected` (missing, malformed, expired, wrong iss, wrong aud); m7 test above (missing `exp` rejected, forged payload, wrong secret) |
| Key rotation by `kid`; unknown `kid` rejected without falling back to the primary secret | `jwt.rs:157-164` | `test_phase1_jwt_key_rotation_support`, `test_phase1_jwt_unknown_kid_rejected_without_fallback`, m7 test (ghost kid) |
| Garbage tokens never panic | `jwt.rs:149` returns `Result` | m7 test (empty, dots, 100 kB input) |
| Startup fail-closed: auth required without secret; network bind without auth/TLS; insecure-HTTP override never bypasses auth | `src/security/mod.rs:108` | `test_phase1_at09_startup_security_fail_closed_rules`, config tests `conflict_rules`, `production_profile` |
| Secret strength | `src/config.rs` (`MIN_JWT_SECRET_BYTES`) | `jwt_secret_length_rules` |

Not covered: token replay, stolen-token revocation, clock-skew edge cases around the 10 s leeway, algorithm confusion beyond HS256/HS384/none, very large tokens against the 512 KiB body limit (headers are limited by hyper defaults, not tested here), timing side channels of signature comparison (delegated to `jsonwebtoken`).

### 2.2 Authorization (RBAC) and tenancy

| Route | Required role | Evidence | Tests |
|---|---|---|---|
| `POST /api/control` | `scenario:operate` | `server.rs:765-772` | `test_phase1_at03...`, `test_phase1_at04_insufficient_role_returns_403_and_no_mutation`, `m7_http_rbac_matrix_in_secure_mode` |
| `GET /api/v1/metrics`, `/api/events` | `telemetry:read` | `server.rs:573-580`, `:605-612` | `test_phase1_at10_sse_events_telemetry_rbac`, m7 matrix |
| `GET /api/v1/diagnostics/*` | `telemetry:read` | `server.rs:1829`, `:1849` | `test_phase1_at10_diagnostics_protected_with_observer_role`, m7 matrix |
| `DELETE /api/v1/cache/*` | `cache:invalidate` | `server.rs:1655` | `test_phase1_cache_read_and_invalidate_roles_enforced`, m7 matrix |
| `GET /api/v1/cache/*` | `cache:read` (SEC-M7-01 fixed; 401/403 before any cache or origin work) | `get_cache_handler` (top) | `test_phase1_cache_read_and_invalidate_roles_enforced`, `m7_sec_fixes_tests::sec01_*` |
| `/api/v1/intelligence/status`, `/audit` | Observer (`telemetry:read`) (SEC-M7-02 fixed) | `require_observer_if_secure` | `m7_sec_fixes_tests::sec02_*` |
| `/health`, `/health/live`, `/health/ready` | none, by design | `src/health.rs` | `m7_health_tests` |

Role matrix (admin satisfies all, no role satisfies none, each specific role only itself, unknown roles nothing): `src/security/jwt.rs:57`, test `m7_role_matrix`.

Tenant isolation: in secure mode the tenant is the token's `tenant_id`; a client `x-tenant-id` is ignored (`server.rs` comment "caller-supplied x-tenant-id is NEVER trusted"). Tests: `test_phase1_at05_and_at06_tenant_cryptographic_isolation`, `m7_secure_mode_ignores_spoofed_tenant_header` (asserts the request is authorized under the token tenant; it does not itself prove key placement, which at05/06 cover). `Authorization` on a request forces RFC 9111 shared-cache bypass: `test_phase1_at08_rfc9111_auth_bypass_and_no_cache_poisoning`.

Not covered: cache-deception via path normalization and `Vary` combinations beyond the existing at08 tests; per-endpoint behavior when `require_auth_for_control` is set to false programmatically; CSRF/CORS interplay.

### 2.3 Upstream SSRF

| Control | Evidence | Tests |
|---|---|---|
| Every resolved address is filtered; one bad address taints the answer set | `upstream.rs:309-331` (`resolve_and_pin_origin`) | `test_p2_ssrf_mixed_dns_blocked` |
| IMDS 169.254.169.254, link-local, loopback (unless allowed), private ranges (unless LAN allowed), CGNAT, multicast, documentation ranges | `upstream.rs:136-205` | `test_p2_ssrf_cloud_metadata_blocked`, `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses` (25 blocked, 4 allowed, lax filter still blocks IMDS) |
| IPv4-mapped, IPv4-compatible, 6to4, NAT64 embeddings; fd00:ec2::254; ULA; fe80::/10 | `upstream.rs:208-300` | `test_p2_ssrf_ipv4_mapped_ipv6_blocked`, `test_redteam_ipv4_compatible_and_transition_ipv6_ssrf_blocked`, m7 filter test |
| Sensitive ports hard-blocked even if allowlisted; privileged ports blocked | `upstream.rs:89` | `test_p2_hard_blocked_ports_unconditional`, `m7_upstream_policy_validate_target` (8 ports) |
| Scheme, userinfo, query/fragment restrictions; allowlist exactness; plaintext needs flag plus allowlist; `enforce_tls` | `upstream.rs:412` onward | `test_p2_insecure_upstream_fail_closed`, `test_redteam_insecure_upstream_empty_allowlist_fails_closed`, m7 validate_target test |
| Loopback literal recognition (fixed) | `upstream.rs:448` | `m7_upstream_policy_loopback_literals_follow_the_same_rule` |
| Redirects not followed; credential forwarding off by default; response framing attacks | `server.rs` origin fetch | `test_p2_redirect_not_followed`, `test_p2_credential_forwarding_policy`, `test_redteam_default_policy_strips_credentials`, framing tests |
| Upstream TLS verification (untrusted CA, expired, SAN mismatch) | `upstream.rs` `UpstreamTlsConfig` | `test_p2_untrusted_ca_rejected`, `test_p2_expired_cert_rejected`, `test_p2_san_mismatch_rejected` |
| Startup: configured origin must pass the same policy | `src/config.rs` | `conflict_rules` |

Not covered: DNS rebinding between resolution and connect beyond pinning one resolved address (the pin is used for the socket; verified by reading `server.rs:2139`, not by a rebinding test), the ranges in SEC-M7-10, proxies/`HTTP(S)_PROXY` environment behavior, redirects to other schemes (redirects are not followed at all).

### 2.4 Cluster mTLS

| Control | Evidence | Tests |
|---|---|---|
| TLS 1.3 only; client certificates mandatory; trust anchored only to the cluster CA (web PKI excluded) | `cluster.rs:623-640` | `test_p3_sec04_strict_tls13_rejection_of_tls12_client`, `test_p3_at02_untrusted_ca_rejection`, `test_p3_at09_plaintext_tcp_connect_rejection` |
| Strict SAN identity `node-<id>.<cluster>.phantom.internal`; single-SAN rule | `cluster.rs:70` | `test_p3_at22_malformed_identity_and_illegal_san`, `test_p3_sec03_strict_single_san_multi_san_rejection`, `m7_cluster_identity_and_registry_edge_cases` |
| Membership: cluster id, node known and active, certificate fingerprint match | `cluster.rs:365` | `test_p3_at04..at08`, m7 registry test |
| Registry epoch strictly increases; revocations broadcast | `cluster.rs:405` | `test_p3_at16_stale_registry_epoch_isolation`, m7 registry test |
| Membership file validation (duplicate ids, 64-hex lowercase fingerprints, versions) | `cluster.rs:453` onward | `test_p3_sec01_json_membership_validation_edge_cases` |
| Startup rules (network bind needs complete mTLS; partial material rejected) | `cluster.rs:664` | `test_p3_startup_security_fail_closed_validation`, config tests; SEC-M7-04 fix |

Not covered: certificate revocation lists or OCSP (revocation is only via the membership registry, which is read once at startup so an update needs a restart), key-file permissions (a warning exists only on unix, `config::preflight`), multi-host operation, the experimental status of cluster invalidation semantics (`docs/user/limitations.md`).

## 3. Secret handling

`PHANTOM_JWT_SECRET` is read once; the effective-config report, `Debug` for `Config` and all configuration errors redact or omit it (`src/config.rs`; tests `secrets_never_appear_in_report_debug_errors_or_warnings`, using a canary string across default, error and warning paths). Not covered: logs, metrics, diagnostics, SSE and error pages (the M7-03 secret-canary test across all channels was not performed); the secret remains in the process environment and is visible to the same OS user.

## 4. Fixes made (all with tests; no existing test weakened)

1. `src/main.rs` + `src/config.rs`: cluster mTLS validation only when clustering is requested (SEC-M7-04). Tests `single_network_node_does_not_require_cluster_material`, `network_cluster_without_mtls_material_fails`.
2. `src/config.rs`: strict booleans, secret length, fail-fast bind/numeric parsing (SEC-M7-05, -06, -08). Tests `booleans_reject_typos_instead_of_reading_false`, `jwt_secret_length_rules`, validation matrix.
3. `src/security/upstream.rs`: loopback detection by parsed host (SEC-M7-07). Test failed before the change and passes after.

## 5. Recommended owner dispositions

SEC-M7-01/02/03 are fixed pending the independent review CLAUDE.md requires; SEC-M7-09..11 stay open: accept or schedule. Run `cargo audit` once approved.

## 6. SEC-M7-01/02/03 fix record (Alternative A)

Files: `src/api/server.rs` (auth ordering in `get_cache_handler`, `require_observer_if_secure`, audit pagination, `CorsConfig`, `create_app_with_cors`, `start_server_with_cors`, `WWW-Authenticate: Bearer` on all 401s), `src/config.rs` (`PHANTOM_CORS_ORIGINS`, `parse_cors_origins`, `Config::cors_origins`, report line), `src/main.rs` (wiring), `Cargo.toml` (test target only). Conversions of `m7_finding_*` tests are noted in their doc comments. SEC-M7-09, -10 and -11 are untouched and remain open; the raw-token acceptance is recorded by `sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09`. This fix record is by the author of the change and is not an independent review.

## 7. Round-2 review fixes (independent review of SEC-M7-01/02/03)

Tests: `tests/integration/m7_sec_round2_tests.rs` (in-process router tests and `e2e_*` tests that spawn the real gateway binary), `m7_config_tests::cors_origins_*`. Failing-before evidence was produced on a scratch copy of the pre-round tree (`D:\Temp\sec2_scratch`, log `D:\Temp\sec2_before.log`).

| ID | Severity | Status | Fix |
|---|---|---|---|
| H1 | High (pre-existing) | **Fixed** | `CacheKey::validate` rejects control characters (C0), DEL and space in path and query (400 `InvalidPath`); the upstream request target is built by `encode_origin_target` (RFC 3986 percent-encoding to pure ASCII, valid `%XX` kept, `?` in a path encoded), so a validation bug alone cannot inject headers or a second request. Before: `GET /api/v1/cache/a%20HTTP/1.1%0d%0a...` reached the origin with injected bytes. |
| M1 | Medium | **Fixed** | Secure-mode `GET`/`DELETE /api/v1/cache/*` require a non-empty `tenant_id` claim, and the reserved value `__default__` is rejected (403, `X-Auth-Error: MissingTenant`/`ReservedTenant`). Tenant-less observer tokens still read metrics. |
| M2 | Medium (lead decision) | **Fixed** | `main.rs`: `enforce_rfc9111_auth_bypass = PHANTOM_FORWARD_UPSTREAM_AUTH`. Default: authenticated reads use the per-tenant cache path (stored only if RFC 9111 3.5 allows). Forwarding enabled: `BYPASS-AUTH`. Documented behaviour change. |
| M3 | Docs | **Fixed** | CORS docs state the production default and the development default honestly. |
| Lows | Low | **Fixed** | `WWW-Authenticate` and `X-Auth-Error` exposed via CORS; set-but-empty `PHANTOM_CORS_ORIGINS` = none; userinfo redacted from CORS errors; startup warning for dev CORS defaults on non-loopback binds; generic 401 body; telemetry/diagnostics assertions in zero-side-effect tests; `require_auth_for_control=false` branch and `/api/events` gating tested; pagination first-wins tested; body reads before auth are bounded by the 512 KiB ingress limit and documented (unchanged, the existing m1 t26 test requires the 413); `/api/control` JSON-before-auth documented. |

Residual: SEC-M7-09/10/11 unchanged and open. Non-ASCII bytes in the `x-*` headers forwarded upstream are skipped by `to_str` (unchanged). The encoder keeps a valid `%XX` escape from the decoded path literally, so a client `%252F` reaches the origin as `%2F` (unchanged from before; the gateway has always forwarded the decoded path).

## 8. Round-3 review fixes

Tests: `tests/integration/m7_sec_round3_tests.rs` (15 tests); failing-before evidence from a scratch copy of the pre-round tree (`D:\Temp\sec3_scratch`, log `D:\Temp\sec3_before.log`).

| ID | Status | Fix |
|---|---|---|
| M-1 (Medium) | **Fixed** | `fetch_origin_raw` forwards only gateway-controlled headers (fixed `Accept`, key-derived `x-tenant-id`, validated `x-request-id`, optional `Authorization`, stored validators). Client `x-*`, `Accept`, `Cookie`, `x-cache-bypass` are never forwarded. `m1_http_tests` mock origin is now steered by one-shot per-path plans instead of `x-mock-*` client headers (11 call sites; assertions unchanged). |
| M-2 (Medium) | **Fixed** | `src/dist/node.rs` GetRequest: key must pass `CacheKey::validate` and not start with `/` (answered `ERR 400`); legitimate cluster-internal keys unchanged. |
| L-1 | **Fixed** | `%` in the decoded path is always `%25`; valid `%XX` pass-through only for the raw query; `.`/`..` path segments rejected (400); direct unit test of `encode_origin_target`; leading `//` behaviour documented and tested. |
| (a) bypass framing | **Fixed** | the bypass path no longer copies origin `Content-Length`, `Transfer-Encoding`, `Connection`, `Keep-Alive` and other hop-by-hop headers onto the re-framed response. |
| (b), (c), (d) | Tests/doc | check order 401 -> 403 role -> 403 tenant -> 400 tested; fallback-branch and DELETE `X-Auth-Error` codes tested; `x-cache-warning` added to CORS expose list. |

Residual: SEC-M7-09/10/11 unchanged and open. An origin that varies on `Host`-independent client data outside request headers (client IP is not forwarded either) is not possible through the gateway; an origin that personalises on `x-tenant-id` is by design per tenant.
