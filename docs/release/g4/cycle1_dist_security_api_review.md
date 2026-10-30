# G4 Cycle 1 review G4B: distributed layer, security and HTTP API

Reviewer: G4B, an AI reviewer (Claude). This is NOT human or external independent review.
Date: 2026-10-30. Method: read-only, line-by-line reading of the working tree. No cargo runs, no test runs, no browser, no network, no source edits, no git mutations. Nothing below was reproduced by execution; "CONFIRMED" means confirmed by reading code with the cited `file:line` and a concrete scenario, not by running it.

Tree state at start/end: HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 93 `git status --short` lines at the time I looked (a large uncommitted tree; my only write is this file).

Evidence tags: verified in code (file:line) / assumption / hypothesis. Validity: CONFIRMED = code read + concrete scenario; PLAUSIBLE = mechanism visible but depends on a runtime or library behaviour I did not verify; REFUTED = I looked for it and the code prevents it.

## 0. Scope actually read

Fully read: `src/dist/{node,router,pool,frame,session,anti_entropy}.rs` (frame tests and router/anti_entropy test modules skimmed only), `src/security/{jwt,cluster,mod,tls,upstream}.rs`, `src/main.rs`, `src/api/server.rs` (all handlers, auth, upstream fetch/parse; NOT the v1 payload builder 821-938, the metrics sampler 460-497, or the simulation loop 2948+), `src/api/lab_api.rs` (all of: Windows FFI, run handle, service, router, host/origin guard, start/cancel/SSE/artifacts, evidence; NOT 429-505 path-scrub helpers, 1177-1250 capabilities, 1879-1932 list/get), `src/config.rs` (validate_cross, secret handling, report; not the whole 1069 lines), `src/lab_live/gateway.rs` spawn/env, `src/lab/runner.rs` validate, `src/health.rs` (route list only), `src/intel/diagnostics.rs` hasher, and `src/core/cache.rs` 10-110, 560-630, 1700-1725. Test bodies were NOT read (see section 5, names only).

## 1. Finding counts

| Severity | CONFIRMED | PLAUSIBLE | REFUTED (looked for, not present) |
|---|---|---|---|
| Critical | 0 | 0 | |
| High | 1 | 0 | |
| Medium | 12 (G4B-10 is mixed: handshake part CONFIRMED, accept-loop part PLAUSIBLE) | 0 | |
| Low | 16 | 1 (G4B-25) | |
| Info | 7 | 1 (G4B-35) | |
| REFUTED hunting targets | | | 13 (section 4) |

Total numbered findings: 38 (1 High, 12 Medium, 17 Low, 8 Info).

## 2. Top 10 findings

1. G4B-01 (High) unauthenticated, untimed body reads and no connection/concurrency caps on the HTTP ingress: `src/api/server.rs:1073,1112,1141,1942,220,313-314`.
2. G4B-02 (Medium) `/api/events` SSE: no stream cap, 100 ms payload build per stream, token expiry never enforced: `server.rs:940-982`.
3. G4B-03 (Medium) `/api/control` mutates the LIVE router nodes; missing/unknown `event` resets every node to Active: `server.rs:1086-1099`, `main.rs:374,431`.
4. G4B-04 (Medium) main API has no Host/Origin guard (DNS rebinding of the anonymous dev-mode API incl. DELETE and `/api/control`): `server.rs:198-220` vs `lab_api.rs:1060`.
5. G4B-05 (Medium) any active member can send Tenant/FullCache; peer-supplied tenant strings are unvalidated and each call clones the whole epoch map: `node.rs:547-558`, `cache.rs:576-606`.
6. G4B-06 (Medium) no membership-epoch enforcement on `InvalidateRequest`; the issuer-side epoch check on the ack is vacuous because the receiver echoes the sender's epoch: `node.rs:561-569`, `router.rs:1674`.
7. G4B-07 (Medium) cluster membership is static; revocation machinery (SEC-3C-03) has no production trigger: no non-test caller of `ClusterMembershipRegistry::update`/registry write lock (`main.rs:263-275`).
8. G4B-08 (Medium) pooled and inbound mTLS sessions never age out; the heartbeat refreshes the idle timer, so a certificate that expires or is rotated is not enforced on live connections: `pool.rs:268,375`, `node.rs:355-406`.
9. G4B-09 (Medium) DELETE `204` means "the routed primary acked", and the primary can be the issuer itself; every other member is best-effort with an in-memory issuer-local ledger: `router.rs:1351-1392,1805-1832,137`.
10. G4B-10 (Medium) cluster handshake semaphore (64 x 1 s) is a cheap unauthenticated cluster-wide handshake DoS, and the accept loop exits permanently on any accept error: `node.rs:228-278,242`.

## 3. Findings (all)

Format: ID, severity, validity, location; scenario; preconditions; impact; fix; regression test (not written into the repo).

### G4B-01 High, CONFIRMED: unauthenticated slow-body / no connection and concurrency limits on the ingress
- Location: `server.rs:1112,1141` (GET/`_body: Bytes`), `1942` (DELETE `_body: Bytes`), `1073` (`Json(payload)` before the auth check at 1075-1081), `220` (only `DefaultBodyLimit` 512 KiB and CORS layers), `313-314` and `296-298` (`axum::serve` / `axum_server` with defaults).
- Scenario: the extractors read and buffer the body before the handler body runs, so authentication (handler lines 1148-1200, 1954-1962, 1075-1081) comes after. An unauthenticated client sends `DELETE /api/v1/cache/x` with `Content-Length: 524288` and trickles one byte every few seconds; each connection holds a task and a buffer indefinitely. No `TimeoutLayer`, `ConcurrencyLimitLayer`, connection cap or body-read timeout exists anywhere in `server.rs` (contrast `lab_api.rs:52,1692`, which has a 5 s body timeout). Also an oracle: 413/415/422 are returned before 401.
- Preconditions: network reachability of the ingress. Authentication is not needed.
- Impact: availability. File-descriptor/memory exhaustion at the OS soft limit is the practical bound. Rated High because it is unauthenticated, remote, and trivial; runtime behaviour was not reproduced (hyper's own header-read timeout default and whether axum 0.7 installs a timer were not verified; the body-read half does not depend on that).
- Fix: authenticate before body extraction (take `Request` and parse after auth, as `lab_api.rs:1677-1696` does), drop `Bytes` from GET/DELETE signatures, add `tower::timeout`/`tower_http::timeout::RequestBodyTimeoutLayer`, a connection limit, and `http1::Builder::header_read_timeout` via a custom serve loop.
- Regression test: integration test that opens N sockets, sends headers with a 500 KiB `Content-Length` and stalls; asserts a legitimate authenticated GET still succeeds within a bound and that stalled connections are closed within T seconds (fails today). Second test: unauthenticated `POST /api/control` with invalid JSON must return 401, not 400/415/422.

### G4B-02 Medium, CONFIRMED: `/api/events` has no stream cap and outlives token expiry
- Location: `server.rs:940-982`. Auth is checked once (945-951); the stream is a 100 ms `IntervalStream` that calls `build_schema_v2_payload`/`v1` per tick per connection; no concurrent-stream cap (the Lab SSE has `MAX_SSE_STREAMS=16`, `lab_api.rs:59,2033`), and no expiry check (the Lab SSE ends at `exp`, `lab_api.rs:2015-2017,2055`).
- Scenario: an Observer (or anonymous caller in dev mode) opens thousands of streams; CPU scales linearly (10 payload builds/s each including percentile computation). A token that expires at T keeps streaming telemetry forever.
- Impact: availability and a stale-credential access window (read-only aggregate telemetry, no tenant keys).
- Fix: global/per-principal stream cap, close at `exp`, jitter/shared snapshot broadcast instead of per-stream build.
- Test: open 64 streams with a valid Observer token, expect 429 beyond cap; mint a token with `exp = now+2`, assert the stream ends by `exp+leeway+interval`.

### G4B-03 Medium, CONFIRMED: `/api/control` mutates live router nodes; unknown/missing event resets every node to Active
- Location: `server.rs:1070-1102`; wiring `main.rs:371,374,431` (`ApiState.nodes` is the same `Vec<Arc<VirtualNode>>` moved into the router) and `node.rs:249-254`.
- Scenario: `NODE_FAILURE` sets node id 2 `Failed`. In a real cluster that is a real peer (or the LOCAL node when `PHANTOM_NODE_ID=2`, whose listener then drops every inbound connection, `node.rs:250-254`); `route_key`/`get_replicas` exclude it and `connect_to_node_inner` refuses it (`router.rs:1203`). Every other event string, including a missing `event` key (default "NORMAL", line 1083) or `{}`, takes the `else` branch (1093-1099) and sets EVERY node `Active`, silently overriding `Suspect`/`Recovering` states that gate routing and fan-out (`router.rs:1358,1722,1995,2149`). In dev mode (no JWT) this is anonymous.
- Preconditions: `scenario:operate`/`admin` token, or dev mode.
- Impact: integrity of failure-handling state and cluster availability; the metrics page labels node state "inferred" (`server.rs:663`) but it is an operator-writable flag. The endpoint is a simulation hook that is live-wired in production builds.
- Fix: gate behind `PHANTOM_SIMULATION`/a build feature, refuse when a router with cluster TLS exists, only accept the three documented events and return 400 otherwise, never touch router-owned nodes.
- Test: start a mTLS cluster harness; set peer `Recovering`; `POST /api/control {}` as Operator; assert state unchanged (fails today). `POST {"event":"NODE_FAILURE"}` with a cluster router must return 409/403 and leave state.

### G4B-04 Medium, CONFIRMED: main API has no Host/Origin guard (DNS rebinding) in anonymous mode
- Location: absent in `server.rs:181-221`; the Lab router has it (`lab_api.rs:1060-1094`) but only for `/api/v1/lab/*`.
- Scenario: dev mode on `127.0.0.1:8085` with no JWT. A web page on `attacker.example` (DNS rebinds to 127.0.0.1) issues same-origin `fetch('/api/v1/cache/..', {method:'DELETE', headers:{'x-tenant-id':'victim'}})` or `POST /api/control` with `Content-Type: application/json` (no preflight, because it is same-origin after rebinding). GET reads any tenant's cached data from the dev cache.
- Preconditions: developer browses the web while the gateway runs on loopback without auth (the documented dev default).
- Impact: read/invalidate/state-change of the dev instance from a hostile page. Not reachable in production profile (auth required).
- Fix: apply the `host_origin_guard` logic to the whole app when `security.is_none()`.
- Test: with no JWT, request `GET /api/v1/metrics` with `Host: evil.example:8085` expects 421; matching Lab test exists for Lab only (`lab_api/review_tests*.rs`, names not verified).

### G4B-05 Medium, CONFIRMED: any active member may send Tenant/FullCache scope; peer tenant strings are unvalidated; epoch-map clone amplification (DR-027)
- Location: `node.rs:547-558` and `node.rs:85-90` (`sync_anti_entropy_batch`) pass the peer string straight to `try_invalidate_tenant`; `frame.rs:515-517` only checks UTF-8 and length <= 4096; `cache.rs:576-606` has no charset/length check (HTTP path enforces `[A-Za-z0-9._-]{1,128}` at `cache.rs:39-50`) and does `*snap = Arc::new(guard.clone())` on every call (line 604).
- Scenario: a compromised/misbehaving member sends 10,000 Tenant frames with distinct 4 KiB names: ~40 MiB resident in the map, and each of the later calls clones the whole map under the shard-independent `tenant_epochs` mutex (O(n) strings per call, O(n^2) total); the 10,001st distinct name hits the SEC-4A-01 fallback (`cache.rs:581-593`) and advances `global_epoch`, flushing every tenant. Even without that, one `FullCache` frame flushes everything (the larger DR-027 gap).
- Preconditions: a valid cluster certificate pinned in the registry (or any local process in the plaintext loopback test mode, where the originator check is skipped, `node.rs:507`).
- Impact: cross-tenant cache flush / origin thundering herd and CPU/memory pressure from a single member. Within the stated trust model (members are trusted) this is a blast-radius issue, not a break-in.
- Fix: validate peer tenant ids with `CacheKey` rules before `try_invalidate_tenant`; cap per-peer invalidation rate and scope (ExactKey default; Tenant/FullCache only for an authorisation flag in the registry); avoid full-map clone (persistent map or sharded snapshot).
- Test: mTLS harness peer sends a Tenant frame with a 4096-byte name containing `:` and `\n`; expect the connection closed or `InternalError` and `tenant_epochs.len()` unchanged (fails today). Second: 10,001 distinct valid names complete within a time/alloc budget.

### G4B-06 Medium, CONFIRMED: no membership-epoch check on `InvalidateRequest`; the ack-epoch check is vacuous
- Location: receiver `node.rs:499-577` never compares `payload.membership_epoch` with the registry version (the AntiEntropy handler does, `node.rs:590-596`); it echoes the request epoch in the ack (`node.rs:561-566`); the issuer then checks `ack.membership_epoch != self.get_membership_epoch()` (`router.rs:1674`), which can only differ if the issuer's own epoch changed mid-flight. The broadcaster path does not check the epoch in the ack at all (`router.rs:738-748`).
- Scenario: a node holding an obsolete registry (it still sees a revoked member as active, or vice versa) keeps being treated as an equal authority; an invalidation from epoch 1 is applied under epoch 9. The comment "SEC-4B-05: Validate ... membership epoch" overstates what is enforced.
- Impact: the epoch carries no security weight on the data path; claims in `docs/security/stage-4b-*` that rely on it need re-reading. Because the registry is static (G4B-07) the practical exposure today is small.
- Fix: reject (`IgnoredObsolete`) when `payload.membership_epoch != reg.version` or `<` as a policy decision; ack the RECEIVER's epoch.
- Test: peer sends InvalidateRequest with epoch = registry.version - 1; expect `IgnoredObsolete` and no purge; issuer ack epoch must equal the receiver's.

### G4B-07 Medium, CONFIRMED: membership registry is static; revocation is unreachable in production
- Location: `main.rs:263-275` loads the file once ("updates require process restart", `cluster.rs:555`); `rg 'registry.write|\.update\('` finds no non-test caller of `ClusterMembershipRegistry::update` (`cluster.rs:405`), and `TlsConnectionPool::evict_node` is called only on a stale-connection retry (`router.rs:1642`). The broadcast channel (`cluster.rs:296,347`), `SessionTracker::evict_peer` (`session.rs:42`), and per-request revalidation (`node.rs:394-406`) therefore only ever run in tests.
- Scenario: a member's key is compromised; the only remedy is to edit the file and restart every node (pinned fingerprints), during which the compromised member keeps full invalidation power.
- Impact: Layer-5 "active revocation" is not an operational capability. Also no CRL/OCSP.
- Fix: a reload trigger (SIGHUP/admin endpoint with Admin role and epoch monotonicity), wired to `update`, `evict_node`, `SessionTracker`. Until then state it as a limitation.
- Test: harness loads registry v1, opens a session, swaps to v2 with the node inactive through the production reload path; expects the session closed within 1 s and the pool epoch advanced.

### G4B-08 Medium, CONFIRMED: live mTLS connections never expire; heartbeats refresh the idle timer
- Location: `pool.rs:375` (`checkin` after a successful probe) -> `pool.rs:268` sets `last_used_at = now`; `checkout*` evicts only on `idle_timeout` (`pool.rs:190`); `created_at` is never consulted. Inbound: `process_binary_cache_stream` has no idle or max-age timeout (`node.rs:372-406`).
- Scenario: a cluster certificate expires or is rotated (file replaced + restart of one node); other nodes keep reusing the old authenticated connection indefinitely. A pool connection to a peer lives as long as the peer answers the 5 s heartbeat.
- Impact: certificate expiry and rotation are enforced only at handshake. Combined with G4B-07, no mechanism bounds the lifetime of a trust decision.
- Fix: max connection age (`created_at`), close inbound sessions after an idle/age bound, re-run `validate_peer` and a `NotAfter` check.
- Test: with a 3 s cert and `max_age=2 s`, assert a reused connection is replaced; assert an inbound session is closed after expiry.

### G4B-09 Medium, CONFIRMED: DELETE `204` semantics; issuer-local, in-memory repair ledger
- Location: `router.rs:1351-1392` (`route_key` iterates ALL nodes including the local one), `router.rs:1805-1832` (sync leg only to the primary; others via `fanout_targets`/`dispatch_fanout`, async, `1705-1751`), `server.rs:1990-2013` (local purge then `204` on primary ack). `RepairLedger` is a `Mutex<HashMap>` (`router.rs:137-147`), lost on crash or restart of the issuing gateway.
- Scenario: gateway G issues DELETE; for ~1/N of keys G is the primary and the "ack" is from G itself over loopback mTLS (prewarm even keeps a self connection, `router.rs:2024-2031`). `204` is returned before any remote member has applied it; if G crashes before fan-out/repair completes, remote members stay stale until TTL (60 s fixed, longer with stale-if-error up to `PHANTOM_MAX_STALE_MS`). Different members can also disagree on the primary because each uses its OWN node-state view (`router.rs:1358`).
- Impact: consistency, not confidentiality. Do not claim "invalidation acknowledged cluster-wide" for `204`. The code comment at `router.rs:1783-1789` states the contract correctly; the HTTP surface does not expose it.
- Fix: document precisely; optionally add an `X-Cache-Invalidation: local-primary|peers-pending` header; persist or gossip the ledger if durability is required.
- Test: 3-node harness, DELETE key whose primary is the issuer, kill the issuer right after the 204, assert the documented residual staleness bound on peers (and that no test claims otherwise).

### G4B-10 Medium, CONFIRMED (handshake) / PLAUSIBLE (accept loop): cluster listener DoS
- Location: `node.rs:228-278`: 64-permit semaphore, `try_acquire_owned` failure drops the socket, handshake timeout 1000 ms. `node.rs:239-245`: `Err(_) => break` on ANY `accept()` error ends the listener task for good while the node state remains Active.
- Scenario A (CONFIRMED): an unauthenticated host on the cluster network keeps 64 TCP connections open and silent; every legitimate peer handshake is dropped for up to 1 s per cycle; re-open each second to sustain. Cluster invalidation traffic stops (and the data path marks peers Suspect).
- Scenario B (PLAUSIBLE): transient `EMFILE`/`ECONNABORTED` (fd exhaustion from the flood in A, or the HTTP ingress in G4B-01 sharing the process fd limit) makes `accept` fail once; the cluster listener task exits and never restarts.
- Fix: per-source-IP handshake limits and an accept-error retry-with-backoff loop (as hyper does); restart/alert when the listener ends.
- Test: open 64 raw sockets, then assert a valid peer connects within T; inject an accept error (wrap listener) and assert the loop continues.

### G4B-11 Medium, CONFIRMED: no rate limiting; origin amplification via `x-cache-bypass` and cache-busting; cross-tenant invalidation-pool saturation
- Location: `server.rs:1243-1276` (any authenticated reader can bypass the cache; every bypass is an uncoalesced origin fetch; origin concurrency cap exists only if `PHANTOM_ORIGIN_CAP_MAX` is configured, `main.rs:470-477`); `router.rs:1225-1231` + `pool.rs:99` (8 connections per node; saturation returns `RejectedRateLimit` -> HTTP 503 `PrimaryRateLimited`, and `record_owed_to_all` writes a ledger entry for every member, `router.rs:1836`).
- Scenario: tenant A (authenticated) loops `DELETE` or `GET` with `x-cache-bypass: true`. DELETEs of other tenants start failing 503 because the single per-node pool is shared; origin sees unbounded concurrent fetches.
- Impact: noisy-neighbour DoS across tenants; origin load amplification. SEC-M7-11 (no rate limiting) is CONFIRMED and has real cross-tenant effect through the shared invalidation pool.
- Fix: per-tenant token buckets for DELETE and bypass, default non-zero origin cap, per-tenant slice of the pool.
- Test: tenant A issues 200 DELETE/s; tenant B's single DELETE must still return 204 within the budget.

### G4B-12 Medium, CONFIRMED: heartbeat does not drive node state
- Location: `pool.rs:359-381` (a failed probe only `discard`s the connection; no `NodeState` access anywhere in `pool.rs`); `send_heartbeat` has no write timeout (`pool.rs:414`).
- Scenario: peer dies while idle; it stays `Active` until a data-path failure marks it `Suspect` (`router.rs:1645`). The first DELETE whose primary is that peer spends the full 150 ms (or `PHANTOM_CLUSTER_TIMEOUT_MS`) and then fails over. The README/docs wording "heartbeat" suggests failure detection that does not exist.
- Impact: availability/latency and misleading state; not a security break.
- Fix: feed heartbeat failure/success into `compare_and_set_state`, or rename.
- Test: kill a peer process, wait 2 heartbeat intervals, assert the peer state is Suspect without any data-path call.

### G4B-13 Medium, CONFIRMED: JWT lifecycle (SEC-M7-11)
- Location: `jwt.rs:149-193`. `Validation::new(HS256)` leaves `validate_nbf=false` (a future `nbf` is accepted; the claim is not even deserialised); no maximum lifetime (`exp` may be decades away), no `iat` sanity, no `jti`, no revocation list; `leeway = 10`; rotation keys (`jwt.rs:130`) are never configured by `main.rs:411-424`, so rotation is test-only; secrets are never zeroised; no per-key `exp`. `/api/events` outlives the token (G4B-02).
- Impact: a leaked token is valid until `exp` and cannot be revoked; a leaked shared HS256 secret forges all roles (inherent to HS256). No algorithm confusion (HS256 pinned, `jwt.rs:153`).
- Fix: enforce `nbf`, a max `exp - iat` (e.g. 1 h), optional `jti` deny-list, document key rotation procedure and wire `with_rotation_key` from config.
- Test: token with `nbf = now+3600` must return 401; token with `exp = now + 10 years` must return 401 under the max-lifetime policy.

### G4B-14 Low, CONFIRMED: SEC-M7-09 role aliases and bare-token acceptance
- Location: `jwt.rs:57-84`, `server.rs:991-993`.
- `requester` satisfies BOTH `cache:read` and `cache:invalidate`; `read` and `invalidate` are bare aliases; all comparisons are ASCII-case-insensitive (`"ADMIN"` is admin). `authenticate_request` accepts an `Authorization` value without the `Bearer ` prefix (`Some(h) => h.trim()`) and is case-sensitive on `Bearer ` (so `bearer <jwt>` is taken as a bare string and fails). Impact: privilege creep by alias (a token issued for "requester" can DELETE); no bypass of signature validation. Needs a valid signed token, hence Low.
- Fix: exact role strings only, require `Bearer` (case-insensitive scheme per RFC 9110), delete the aliases or document them in the token contract.
- Test: roles `["requester"]` must get 403 on DELETE (or the alias contract documented and asserted); `Authorization: <jwt>` without scheme must be 401.

### G4B-15 Low, CONFIRMED: SEC-M7-10 SSRF filter range gaps
- Location: `upstream.rs:136-306`. Not blocked: 198.18.0.0/15, 240.0.0.0/4 (except broadcast), 192.0.0.0/24, 192.88.99.0/24, 168.63.129.16 (Azure wire server), IPv6 Teredo 2001::/32, fec0::/10 (site-local), 64:ff9b:1::/48, 3fff::/20; `fd00:ec2::254` is blocked only by exact tail. Handled correctly: IMDS v4/v6 forms, link-local, mapped/compatible v4 via `to_ipv4`, 6to4, NAT64 WKP, multicast, CGNAT, all answers in a DNS set, address pinned for connect with the host kept for SNI/verification (`upstream.rs:309-343`, `server.rs:2582-2631`).
- Why Low: the destination host is the OPERATOR-configured `ORIGIN_URL` (clients control only path/query, encoded at `server.rs:2478-2507`), so reaching the filter requires control of that host's DNS. No redirects are followed (raw HTTP/1.1, 3xx returned as a status).
- Fix: use the IANA special-purpose registry as an allow-the-global-unicast model; add the ranges above.
- Test: table test over the missing ranges expecting `SsrfBlocked`.

### G4B-16 Low, CONFIRMED: origin response headers copied on the bypass path; no `nosniff` on cached responses
- Location: `server.rs:1283-1294` copies every non-hop-by-hop origin header (including `Set-Cookie`, `Access-Control-Allow-Origin`, `Content-Security-Policy`, `Content-Type`) to the client on `BYPASS`/`BYPASS-AUTH`; cached hits serve the origin `Content-Type` verbatim (`server.rs:1566-1571`) without `X-Content-Type-Options`.
- Impact: an origin (or anyone able to influence cached content) can make the gateway origin serve `text/html`; origin CORS headers can widen the policy that SEC-M7-03 intends to be same-origin. `m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit` (test name) covers hits only.
- Fix: allowlist response headers; add `X-Content-Type-Options: nosniff`; force `Content-Disposition`/CSP sandbox for non-JSON types.
- Test: origin returns `Access-Control-Allow-Origin: *` and `Set-Cookie`; bypass response must contain neither.

### G4B-17 Low, CONFIRMED: plaintext cluster listener and startup-check address
- `node.rs:342-346`: with `cluster_tls == None` (loopback bind only, `node.rs:188-196`) sessions are registered as peer 0 and the originator check is skipped (`node.rs:507`), so any local process can send FullCache invalidations. `main.rs` never starts this mode (the insecure branch at `main.rs:380-385` builds placeholder nodes without listeners), so this is test/harness-only.
- `config.rs:819` passes the HTTP bind address to `validate_cluster_startup_security`, not `PHANTOM_CLUSTER_LISTEN_ADDR` (`cluster.rs:664-688`); `cluster.rs:683` reads the raw env var `PHANTOM_STRICT_CLUSTER`. Defence in depth (`node.rs:188`, `router.rs:1211`) saves it, so no exposure found.
- Fix/test: pass the cluster listen address; test `0.0.0.0:7000` listen with HTTP loopback and no certs fails at config.

### G4B-18 Low, CONFIRMED: JWT secret length not enforced on loopback dev
- `config.rs:754-766`: shorter than 32 bytes is only a warning unless `require_auth || network || prod`. A short secret with a JWT-enabled loopback gateway is brute-forceable offline from one token. Fix: always error for HS256 < 32 bytes. Test: `PHANTOM_JWT_SECRET=x` on loopback -> config error.

### G4B-19 Low, CONFIRMED: log injection of the control event
- `server.rs:1084` prints the client string unescaped. Fix: `{:?}` or validate. Test: event `"x\n[Phantom API] fake"` must not produce a second log line.

### G4B-20 Low, CONFIRMED: ingress TLS is not 1.3-only despite the message
- `server.rs:292` prints "TLS 1.3"; `tls.rs:10-22` delegates to `RustlsConfig::from_pem*` with default protocol versions (1.2 and 1.3; the cluster path IS 1.3-only, `cluster.rs:628,634`). Fix: build the rustls `ServerConfig` with `TLS13` or correct the message. Test: a TLS 1.2-only client against the ingress.

### G4B-21 Low, CONFIRMED: diagnostics events leak internal error text to Observers
- `server.rs:1863,1813,2171`: `error_msg: format!("{:?}", err)` stores Debug of `OriginError`/`OriginSecurityError` (resolved internal IPs, `os error` text, origin URL on parse errors); `/api/v1/diagnostics/events` returns it to any Observer. Key fingerprints are 32-bit hashes with a per-process random 64-bit seed (`diagnostics.rs:162-177`), not reversible by dictionary without the seed. Fix: store coarse error classes. Test: an SSRF-blocked fetch must not put an IP into `error_msg`.

### G4B-22 Low, CONFIRMED: anti-entropy responder CPU and ring bookkeeping
- `node.rs:598-600`: each `AntiEntropySyncRequest` hashes the entire ring (up to 4,096 records / 4 MiB, `anti_entropy.rs:9-12,173-191`) under the ring mutex, with no per-peer rate limit; `anti_entropy.rs:112` mixes inbound (peer-chosen) sequences into `newest_seq`, so a peer can set it to `u64::MAX`, changing digests and `InSync` decisions for everyone (`node.rs:604-605`). Authenticated members only. Fix: rate limit; keep inbound sequence space out of `newest_seq`. Test: peer sends InvalidateRequest `stream_seq=u64::MAX`; other peers' sync must still be `InSync` in the clean case.

### G4B-23 Low, CONFIRMED: PHANTOM_NODE_ID not tied to registry or certificate
- `main.rs:298,330-340`: `node_id` is not checked to exist in the registry or to match the node certificate's SAN. If absent from the registry no local node is created, `get_local_node` falls back to `nodes.first()` (`router.rs:987-994`), i.e. a PEER. Fail-closed on the wire (receivers reject a mismatching originator, `node.rs:507-511`) but local bookkeeping runs on the wrong node. Fix: startup check of id in registry and SAN == `node-{id}.{cluster}.phantom.internal`. Test: id not in members file -> exit 2.

### G4B-24 Low, CONFIRMED: invalidation granularity of DELETE
- Key = tenant + path + exact raw query (`server.rs:1255,1982`): `DELETE /x` does not invalidate `/x?v=1`; `/x?` is a different key from `/x`; `//x` aliases `/x` but `///x` does not (`m7_sec_round3_tests::d_leading_double_slash...`). Operators may expect resource-level invalidation. No security impact; document, or add tenant-scope invalidation through HTTP with proper authorisation.

### G4B-25 Low, PLAUSIBLE: Lab artifact/evidence open race
- `lab_api.rs:578-589`: `symlink_metadata` check, then `File::open`, then `f.metadata()`; `f.metadata()` follows the already-opened target so a symlink swapped in between is not detected. Requires local write access to the lab directory. Fix: open with `FILE_FLAG_OPEN_REPARSE_POINT`/`O_NOFOLLOW`, then verify. Not a traversal: names come from a fixed allowlist (`lab_api.rs:2094-2110,2161`) and ids pass `valid_run_id` (`517-525`).

### G4B-26 Low, CONFIRMED: Lab runs never pruned; anonymous mode lets any local user start CPU-heavy jobs
- No retention (`rg 'remove_dir_all|retention|prune'` empty): `runs/` grows without bound; `record_raw` runs write large CSVs. Anonymous loopback Lab (by design, `config.rs:891-897`, `lab_api.rs:97-102`) means any local user can start `t1_matrix` runs (<= 20,000 cells, one at a time). Fix: retention cap; recommend JWT whenever the machine is multi-user.

### G4B-27 Low, CONFIRMED: DefaultHasher routing is not guaranteed stable across builds
- `router.rs:1343-1349`: rendezvous weights use `std::collections::hash_map::DefaultHasher`, whose algorithm Rust does not guarantee across releases; nodes built with different toolchains can disagree on the primary (`route_key` and `get_replicas` then diverge). Fix: an explicit hash (e.g. SHA-256 or SipHash with fixed keys via the `siphasher` crate would need approval; `ring::digest` is already a dependency). Test: golden vectors for `route_key`.

### G4B-28 Low, CONFIRMED: no process-wide panic containment for request handlers
- No `CatchPanicLayer`; many `lock().unwrap()` on shared mutexes (`server.rs:431,1087,1094,771`). A poisoned mutex turns every later request on that route into a panicking task (connection reset). Fix: `unwrap_or_else(|e| e.into_inner())` as the Lab code does.

### G4B-29 Info, CONFIRMED: replay/decode details
- `frame.rs:732` `Vec::with_capacity(record_count)` before validation (up to 65,535 x ~48 B per frame); decoded replay keys may exceed 4,096 bytes (`frame.rs:747-756`, only the encoder caps); `records.len() as u16` can wrap in the encoder (`frame.rs:668`) but the ring cap (4,096) makes it unreachable. `cluster.rs:177` `idx + len` could overflow on 32-bit targets (CA-signed certificates only).

### G4B-30 Info, CONFIRMED: consistency of GetRequest stub with DR-029
- `node.rs:409-481` matches `docs/security/investigations/DR-029_*`: keys starting `/` or failing `validate()` are refused with `ERR 400`, body `val_node_N` is fabricated, written into the shared cache under `__default__` with non-`/` keys, so not HTTP-visible (HTTP keys always begin `/`, `server.rs:1249`). Residual: any member can fill the shared byte budget with up to 4 KiB keys of fabricated entries (resource pressure on tenants), unauthenticated beyond membership; gated on the DR-029 owner decision. Not re-reviewed.

### G4B-31 Info, CONFIRMED: DELETE ignores `require_auth_for_control=false`
- `server.rs:1954-1962` authenticates whenever `security` is Some, GET/metrics honour the flag (`1151`). Fail-closed and unreachable from `main.rs`. Align for clarity.

### G4B-32 Info, CONFIRMED: stale user-facing text
- `config.rs:858` still warns that peer invalidations do not purge this gateway's HTTP cache; `main.rs:373` says the opposite (shared cache). Documentation consistency only.

### G4B-33 Info, CONFIRMED: Lab `scrub_seeds` is a blacklist
- `lab_api.rs:1307-1321` strips five key names from retained verdict JSON. The one retained sealed verdict (`lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`) contains only the key `seed_set` (checked by grep), so nothing leaks today; a future artifact with e.g. a per-cell `seed` field would. Sealed-seed derivation from the API is refused in `runner::validate` (`runner.rs:137,186`) and `LabService::new` refuses to start with the salt in the environment (`lab_api.rs:843`).

### G4B-34 Info, CONFIRMED: broad unused dependency features
- `Cargo.toml`: `axum` `ws`, `tower-http` `fs`, `tokio` `full`: attack-surface hygiene; no use of `ServeDir` or `ws` found in the files read.

### G4B-35 Info, PLAUSIBLE: origin 1xx / unusual statuses reach the client
- `server.rs:2828` treats 1xx as bodiless final responses and the bypass path forwards the status (`1278`); hyper behaviour on a gateway-emitted 101 was not verified.

### G4B-36 Low, CONFIRMED: unlimited authenticated inbound sessions and 1 MiB frame buffers
- `node.rs:330-341`, `frame.rs:838`: after the handshake there is no cap on sessions per peer and no read idle timeout; a member can open thousands of sessions and send 1 MiB headers-only partial frames. Members only. Fix: per-peer session cap and read timeouts.

### G4B-37 Low, CONFIRMED: repair/ledger CPU under adversarial key patterns
- `router.rs:229-302,176-226`: `record` does O(n) `find`/`iter_mut` and, at limits, `collapse_largest_tenant` parses every key under the single ledger mutex; tenant users (with valid tokens) can force this on every rejected DELETE (G4B-11). Escalation to a global flush needs many distinct tenants (`router.rs:178-194`), so cross-tenant blast radius needs several tokens. Fix: index per tenant.

### G4B-38 Info (not a data-loss bug; kept as a note), CONFIRMED mechanism: broadcaster worker drops a job for a peer revoked mid-delivery
- `router.rs:679-690,756` do not record a job when the member is inactive, but `repair_all_peers` converts the debts of non-active members into a pending full flush on the next tick (`router.rs:1083-1087`), so staleness on re-admission is covered. Severity Info; noted because the correctness depends on that interplay.

## 4. Looked for and REFUTED

1. Upstream request smuggling / header injection: request line is `GET <encoded target>` (`server.rs:2478-2507,2639-2642`), Host comes from the parsed config, only validated `x-request-id`, the key tenant, optional Authorization (visible ASCII only, `HeaderValue`), and stored validators (`is_header_safe`) are written (`2658-2688`); client conditional/range headers are skipped. `validate()` rejects control characters, `..`, `:?` in paths (`cache.rs:65-82`).
2. Response splitting from the origin: header names/values pass through `HeaderName`/`HeaderValue` (CR/LF impossible); conflicting Content-Length, CL+TE, unsupported TE and oversize are rejected (`server.rs:2757-2860`).
3. SSRF via DNS rebinding of the origin host: the whole answer set is filtered and the connection is pinned to a validated address (`upstream.rs:309-343`, `server.rs:2582-2590`), TLS verified against the host name.
4. Redirect following: none.
5. Cross-tenant cache key confusion: tenant ids exclude `:`; the first colon is the separator; `:?` is banned in paths (`cache.rs:44-50,80-82`); tenant comes from the verified token in secure mode, `x-tenant-id` is removed from forwarded headers and ignored (`server.rs:1209-1234`), tests `m7_secure_mode_ignores_spoofed_tenant_header` and tenant claim checks (`require_tenant_claim`, `__default__` refused).
6. Auth bypass by route order/method/path: every sensitive handler authenticates itself (no middleware dependency), axum `get` also serves HEAD through the same authenticated handler, no alternative route reaches the cache or control handlers; `/health*` and `/health/ready` expose only status/check booleans (`health.rs:61-80`).
7. JWT algorithm confusion/`none`/missing `exp`/unknown `kid` fallback: HS256 pinned (`jwt.rs:153,168`), unknown kid rejected without fallback (`jwt.rs:157-163`), `iss`/`aud` re-checked. Signature comparison is done inside `jsonwebtoken`/`ring` HMAC verification (constant-time); fingerprint comparisons are over public data.
8. Panics/slicing in frame parsing: all indexing is behind exact-length or cursor checks (`frame.rs:477-526,620-652,701-782`); payload size is checked before allocation (`frame.rs:324-329,838`).
9. SAN/identity parsing: exactly-one DNS SAN required (`node.rs:309`), strict label grammar, fingerprint pin, cluster id match; client AND server verification are mandatory (`cluster.rs:624-637`), TLS 1.3 only, WebPKI roots excluded.
10. Artifact download traversal: fixed allowlist equality lookup, never joins client input (`lab_api.rs:2161`); run ids strictly formatted (`517-525`); reparse points rejected (modulo G4B-25).
11. Lab SSE token expiry and cap: enforced (`lab_api.rs:2015,2033,2055`).
12. Windows process reaping: image path, exact creation FILETIME, self/ancestor exclusion, single handle for check and `TerminateProcess` (no PID-reuse TOCTOU), struct size asserted (`lab_api.rs:181,314-351`); job object with kill-on-close is created before any child is spawned; children get `PHANTOM_*` and `ORIGIN_URL` removed from the environment (`gateway.rs:74-81`), so the JWT secret is not inherited.
13. Secrets in config output: `Kind::Secret` is redacted in `report()`, errors and Debug (`config.rs:434,968,1061`); `JwtValidator` and `SecurityConfig` implement no `Debug`.

## 5. The 10 most important security/distributed tests (from names; bodies NOT read)

I could not verify what these assert beyond their names and what the production code implies. Treat "proves" as intended coverage, not verified coverage.

1. `m7_security_review_tests::m7_http_rbac_matrix_in_secure_mode`: proves the role matrix on the HTTP surface under JWT; does not prove aliases are rejected (G4B-14) or anonymous routes are closed when `security` is `None`.
2. `m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`: proves the algorithm/exp/signature rejections; does not prove `nbf`, lifetime caps or revocation (G4B-13).
3. `m7_secure_mode_ignores_spoofed_tenant_header` together with `security_phase1_tests::test_phase1_at05_and_at06_tenant_cryptographic_isolation`: proves tenant derivation from the token in the tested paths; does not prove isolation on the diagnostics or cluster paths.
4. `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses`: proves the listed IPv4/IPv6 forms; cannot prove the missing ranges in G4B-15.
5. `security_phase3_cluster_tests::test_p3_at04_san_identity_target_mismatch`, `at06_revoked_node_rejection`, `sec03_strict_single_san_multi_san_rejection`, `sec04_strict_tls13_rejection_of_tls12_client`: prove the handshake-time admission rules; do not prove runtime revocation reachable in production (G4B-07) or connection ageing (G4B-08).
6. `security_phase3b_framing_tests::test_p3b_at11_oversized_payload_rejected_before_alloc`, `...truncated_*`, `...invalid_utf8_key_rejected`: prove framing limits; do not exercise per-connection resource caps or slow peers (G4B-10, G4B-36).
7. `security_phase4a_invalidation_tests::test_p4a_sec01_over_10k_tenants_strictly_bounded_and_valid` and `..._tenant_isolation_under_capacity_pressure`: prove the epoch fallback is safe against stale resurrection; do not cover peer-supplied non-conforming tenant ids or the cost of the clone (G4B-05).
8. `security_phase4a_invalidation_tests::test_p4a_malformed_invalidation_frame_terminates_socket`: proves parse errors close the stream; says nothing about authorisation of a well-formed Tenant/FullCache frame (DR-027).
9. `m7_sec_round3_tests::m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant` with `l1_dot_segments_over_http_are_400_and_never_reach_the_origin`: prove upstream header and path hygiene (section 4 items 1-2); do not cover the bypass response-header copy (G4B-16).
10. `cluster_prewarm_repair_tests.rs`, `distributed_consistency_tests.rs`, `known_gaps.rs` (KG-6 pins the GetRequest stub) and `tests/integration/lab_api/review_tests*.rs`: described by file name only; the repair ledger, DR-029 pin and Lab API guards are covered somewhere in them, but I did not verify whether they cover kill-the-issuer durability (G4B-09) or Host/Origin on the main API (G4B-04).

Tests that do NOT exist as far as I could tell by name: slow-body/connection-limit, SSE cap and expiry on `/api/events`, control endpoint against a live cluster, membership-epoch enforcement on InvalidateRequest, peer tenant-id validation, connection max-age, accept-loop resilience, per-tenant rate limits, `nbf`/lifetime.

## 6. What a human or external reviewer must still examine

- Run-time verification of every CONFIRMED-by-reading item above (none executed here), especially G4B-01 (hyper/axum header and body timeout defaults at the pinned versions), G4B-10 (accept error behaviour on Windows and Linux), G4B-13 (`jsonwebtoken` 9.x defaults for `nbf`).
- The threat model decision behind DR-027: are cluster members mutually trusted? The answer determines whether G4B-05/06/22/36 are bugs or accepted risk.
- Token issuance process (not in this repo): who mints tokens, role vocabulary, lifetime, secret storage and rotation. HS256 shared-secret design is an architecture-level choice.
- Dependency advisories (`cargo audit`/`cargo deny`) for axum 0.7, hyper, rustls 0.23, tokio-rustls, jsonwebtoken 9, ring, webpki-roots: not run.
- The unread parts: test bodies; `server.rs` v1 payload and simulation loop; `lab_api.rs` 429-505, 1177-1250, 1879-1932; `config.rs` parsing code; `core/cache.rs` beyond the cited ranges (key-level invariants, fencing); `lab_live/*` beyond `gateway.rs`.
- Operational: deployment behind a reverse proxy (it would mask G4B-01 partially and change Host semantics), file permissions of the cluster membership file/keys/lab directory, firewalling of the cluster port, Windows service account for the job-object behaviour.
- The mTLS PKI: EKU on node certificates (a serverAuth-only certificate cannot be used as a client certificate under webpki), certificate lifetime and rotation procedure given G4B-07/08.
- Independent review of the pending DR-029 fix (`node.rs:409-481`) and the DR-027 authorisation design, as already required by CLAUDE.md for changes touching invalidation.

## 7. Verdict

**YES WITH FIXES** (as a reviewed group, conditional; this is an AI review and is not independent human review).

No Critical finding and no authentication/tenant-isolation bypass was found; the framing, SAN/fingerprint admission, SSRF pinning, upstream request construction, Lab path handling and sealed-seed refusal are sound as read. The group should not be committed as "security reviewed" without the following.

Blocking items (fix or explicitly accept in writing before the group is called reviewed):
1. G4B-01: authenticate before reading bodies, add body/request timeouts and connection/concurrency limits on the ingress (or document that the gateway MUST sit behind a proxy that does).
2. G4B-03: stop `/api/control` from mutating router-owned nodes in a cluster build (and reject unknown events).
3. G4B-05 / DR-027: validate peer tenant ids; decide and document the member trust model; at minimum cap peer-induced epoch-map growth.
4. G4B-06, G4B-07, G4B-08: either implement epoch enforcement, a runtime reload/revocation path and connection ageing, or remove the claims that rely on them (stage-4b/3c acceptance text, SEC-3C-03, SEC-4B-05) and list them as limitations.
5. G4B-02 and G4B-11: cap SSE streams and add at least a basic per-tenant limiter on DELETE and bypass, or record SEC-M7-11 as an open production blocker.

Non-blocking but recommended in the same cycle: G4B-04 (Host guard for anonymous dev mode), G4B-10 (accept-loop and handshake limits), G4B-13/14/15 (SEC-M7-09/10/11 items), G4B-16, G4B-18 to G4B-23.
