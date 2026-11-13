# Chapter 6: Security, Reliability and Performance

This chapter answers three questions a judge will ask about any system that sits in front of other people's data: who can do what to it, what happens when parts of it fail, and how fast it really is. It is written against the working tree as read on 2026-10-30 (branch `feat/publish-phantomcache-core`; HEAD `df68eaa` at the time of writing, with substantial uncommitted work on top). Every claim carries a status label. Almost nothing here is **INDEPENDENTLY VERIFIED**: the G4 reviews cited are "AI second look" reviews (one AI pass reading code, running nothing). The M7 documents call review rounds 2 and 3 "independent", but we could not establish that a human performed them, so this chapter does not use that label.

## What you will learn

- How the gateway decides who may read, invalidate, observe and operate, and which parts of that design are complete and which are open findings.
- How cluster nodes trust each other (mutual TLS, certificate identity, a static registry), and what a trusted peer can still do.
- Where the gateway bounds its resources (bytes, bodies, waiters, queues, ledgers) and where it does not (connections, streams, origin concurrency by default).
- What happens to a DELETE when a peer is down, and which failure (issuer crash) loses an invalidation.
- How to evaluate performance correctly: what each percentile in the repository means, which measurements exist, and what each can and cannot support.

## 6.1 Threat model and trust boundaries

**Simple version.** A security review starts by asking "who are we afraid of, and what do they want?" A cache is attractive for four reasons. It holds data that other users asked for (confidentiality). It decides what answer users see (integrity). It shields a slower system (availability of the origin, and of itself). And it can be made to fetch URLs on someone's behalf (a stepping stone into a private network).

**Terms.**

- *Asset*: something worth protecting. Here: cached bodies per tenant, the origin's capacity, the JWT signing secret, the cluster private keys, the gateway's own CPU and memory.
- *Trust boundary*: a line where data passes between parties that do not trust each other equally. Every check belongs on a boundary.
- *Attack surface*: the set of inputs an outsider can reach.
- *Tenant*: one customer or application whose keys must not mix with another's. In PhantomCache the tenant is a string in the cache key.
- *Defence in depth*: several independent checks on one path, so one mistake does not open it.

![Figure 6.1: Trust boundaries and attack surface of the gateway, the origin, cluster peers and local operator inputs](../assets/ch06_trust_boundaries.svg)

Source: `src/api/server.rs::create_app_with_lab`, `src/security/*`, `src/dist/node.rs::start_tcp_service`, `src/main.rs`.

**Where it lives.** The system has five zones. Clients (Z1) reach the ingress listener, the HTTP server in `src/api/server.rs`. The ingress and the cache core run in one process (Z2). The origin (Z3) is one operator-configured URL (`ORIGIN_URL`); a client chooses only the path and query, never the host. Cluster peers (Z4) reach a separate mTLS listener started only when the cluster certificate files and a membership file are configured (`src/main.rs`). The local host (Z5) supplies configuration, keys and files. The token issuer is **not in this repository**: who mints tokens, with what lifetime, is outside what the code can promise.

**Adversaries we reason about** (our framing, not a recorded threat model; the G4B review explicitly says the cluster-member trust decision is an owner decision still open, section 6.4):

1. An anonymous network caller.
2. A caller with a valid token for tenant A who wants tenant B's data or wants to exhaust shared resources.
3. A compromised or buggy cluster member.
4. A malicious or misbehaving origin response.
5. A local user on the same host.

**What the design does not claim.** There is no protection against a host-level attacker who can read the process environment (the JWT secret is an environment variable; the M7 review says so). There is no claim of resistance to a determined distributed denial-of-service attack. There has been no penetration test, no fuzzing run of the HTTP surface, no TLS configuration scan and no dependency advisory scan (`docs/security/m7_security_review.md`, "Not done"). **UNVERIFIED** for all of these: no one has run them.

## 6.2 Authentication and authorisation

**Simple version.** Authentication answers "who are you?"; authorisation answers "are you allowed to do this?". PhantomCache uses bearer tokens: the client sends `Authorization: Bearer <token>`, and the gateway checks the token's signature and claims.

**Terms.**

- *JWT* (JSON Web Token): a signed bundle of claims. *HS256* means one shared secret both signs and verifies, so any holder of the secret can mint any token.
- *Claims* used here: `sub` (who), `iss` (issuer), `aud` (audience), `exp` (expiry), `iat` (issued at), `tenant_id`, `roles`.
- *RBAC* (role-based access control): permissions come from roles in the token.
- *Revocation*: invalidating a token before it expires.

**How it works in the code** (**IMPLEMENTED**, **TESTED**):

1. `src/security/jwt.rs::JwtValidator::validate` decodes the header, rejects any algorithm other than HS256, picks the secret (the default secret, or a rotation secret chosen by `kid`; an unknown `kid` is an error with no fallback), and asks the `jsonwebtoken` 9.3.1 library to verify signature, `exp`, `iss` and `aud`, with 10 seconds of leeway. It then compares `iss` and `aud` again itself.
2. `Role` has five values: `cache:read`, `cache:invalidate`, `telemetry:read` (Observer), `scenario:operate` (Operator) and `admin`. `JwtClaims::has_role` lets `admin` satisfy every role.
3. `src/api/server.rs::authenticate_request` extracts the token, validates it, checks the role, and returns 401 (with `WWW-Authenticate: Bearer` and an `X-Auth-Error` class) or 403.
4. `require_tenant_claim` then demands a non-empty `tenant_id` that is not the reserved value `__default__`; otherwise 403.
5. `src/main.rs` builds the validator only if `PHANTOM_JWT_SECRET` is set. With no secret, `ApiState.security` is `None` and every route is anonymous.

**Startup is fail-closed** (**IMPLEMENTED**, **TESTED**). `src/security/mod.rs::validate_startup_security` refuses to start a non-loopback bind without a JWT secret, and without TLS unless `PHANTOM_ALLOW_INSECURE_HTTP=true` (even then, authentication stays mandatory). `src/config.rs` makes unknown boolean spellings an error rather than silently "false" (SEC-M7-05), and requires a JWT secret of at least 32 bytes when auth is required, the bind is a network address or the mode is `production` (SEC-M7-06).

::: example
**Worked example: what a request needs.** A client sends `DELETE /api/v1/cache/users/7` with a token whose `roles` is `["cache:read"]` and `tenant_id` is `acme`. Step 4 finds the signature, issuer, audience and expiry fine, but the role check for `cache:invalidate` fails: **403**, `X-Auth-Error: InsufficientPermissions`, and nothing in the cache changes. With `roles: ["cache:invalidate"]` and `tenant_id: acme`, the key becomes `acme:/users/7` and that exact key is purged. The same token cannot touch `globex:/users/7`, because the tenant comes only from the verified token (secure mode ignores the `x-tenant-id` header).
:::

**What is not done** (each an open register entry; check the current disposition in `docs/release/g4/cycle1_findings_disposition_register.md`):

| Gap | Verified in current code | Finding and disposition |
|---|---|---|
| No `nbf` (not-before) check. `Validation::new` leaves `validate_nbf` false and the code never sets it. | `jwt.rs::validate`; crate source `jsonwebtoken-9.3.1/src/validation.rs` | G4B-13 (SEC-M7-11): OPEN |
| No maximum token lifetime, no `jti`, no revocation list. `iat` is parsed but unused. | `jwt.rs` has no such code | G4B-13: OPEN |
| Rotation keys exist in `JwtValidator` but `main.rs` never configures any. | `main.rs` calls `JwtValidator::new` only | G4B-13: OPEN |
| `requester` and `read`/`invalidate` aliases; case-insensitive roles; a token without the `Bearer ` prefix is accepted. | `jwt.rs::has_role`, `authenticate_request` | G4B-14 (SEC-M7-09): OPEN (a characterisation test pins the bare-token behaviour) |
| No rate limit or lockout on failed authentication. | a search of `src/` for limiter code finds none (the Lab API uses fixed caps only) | G4B-13/G4B-11: OPEN |
| Short JWT secret on loopback dev is a warning, not an error. | `config.rs` validation | G4B-18: OPEN |

**Trade-offs.** HS256 with one shared secret is simple and fast, and it is the choice the code makes; the repository records no comparison with asymmetric signing (RS256/EdDSA), where only the issuer holds the private key. That would be *our reasoning*: an asymmetric scheme would let the gateway verify without being able to mint, which matters once the gateway and the issuer are run by different parties. The G4B review calls the shared-secret design an architecture-level choice that a human must confirm.

**What can go wrong.** A stolen token works until `exp` (no revocation). Anyone who reads the gateway's environment can mint an Admin token. A very long `exp` is accepted.

::: judge
**JUDGE MAY ASK**

Question: How do you stop an unauthorised user reading or deleting cached data?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

When a JWT secret is configured, every cache read, delete, metrics, stream and control call needs a signed token. The gateway checks the signature, issuer, audience and expiry, then checks that the token's role allows the action: `cache:read` to read, `cache:invalidate` to delete. The tenant is taken from the token, never from a client header. The check runs before any cache lookup or origin fetch. If no secret is configured the gateway only starts on a loopback address, and it refuses to bind to a network address without a secret and TLS. What I would not claim: tokens cannot be revoked before they expire, there is no not-before check, and the token issuer is outside this repository.

**If the judge asks further:**

Ask about ordering. In the GET and DELETE handlers the body is extracted before the handler authenticates (G4B-01, High, owner-named, OPEN; see 6.5). The Lab API does it in the right order with a 5-second body timeout.

**Evidence to know:** `src/security/jwt.rs::JwtValidator::validate`, `src/api/server.rs::authenticate_request`, `require_tenant_claim`; tests `m7_sec_fixes_tests::sec01_*`, `m7_security_review_tests::m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`, `m7_http_rbac_matrix_in_secure_mode`; G4B-01, G4B-13, G4B-14. Status: IMPLEMENTED, TESTED for the covered paths; revocation, `nbf`, lifetime caps absent.
:::

## 6.3 Management and API access

**What the surface is.** Besides the cache routes, the ingress exposes management routes. All are listed in `create_app_with_lab`:

| Route | Who may call (secure mode) | Notes |
|---|---|---|
| `GET/DELETE /api/v1/cache/*key` and `/api/v1/cache` | `cache:read` / `cache:invalidate` + tenant | DELETE authenticates whenever security is on, even if `require_auth_for_control` were false (G4B-31, info) |
| `POST /api/control` | `scenario:operate` | demo control; see below |
| `GET /api/events` (SSE) | `telemetry:read` | one JSON payload every 100 ms per open stream |
| `GET /api/v1/metrics`, `/diagnostics/summary`, `/diagnostics/events`, `/intelligence/status`, `/intelligence/audit` | `telemetry:read` | audit paging is bounded (limit 1..100) |
| `GET /health`, `/health/live`, `/health/ready` | public | status only |
| `/api/v1/lab/*` | Observer to read, Operator to start/cancel | exists only when `PHANTOM_LAB_ENABLED=true` |

**`/api/control` is the weakest management route.** In `control_handler`, `NODE_FAILURE` sets node 2's state to Failed, `FLASH_CROWD` flips a workload label, and any other value, or a missing `event`, sets *every* node to Active. In a cluster-configured process the `nodes` in `ApiState` are the same objects the router uses for DELETE routing (`main.rs` builds `RendezvousRouter::new(nodes.clone())` and then stores `nodes` in `ApiState`), so an Operator token can mark a live router node Failed. That is G4B-03, owner-named, OPEN. **Status:** IMPLEMENTED; reachable consequence verified by reading `main.rs` and `server.rs`, not by running a cluster; no test drives `/api/control` against a live cluster (the existing tests call it for the role matrix only). A related low finding: the event string is printed to the log unescaped (G4B-19).

**`/api/events` has no stream cap, and a stream outlives its token.** `sse_handler` authenticates once, then runs an interval timer that builds the full payload every 100 ms for each open connection. There is no counter of open streams and no check of `exp` after the stream starts (G4B-02, OPEN). Compare `src/api/lab_api.rs`: `MAX_SSE_STREAMS = 16` (429 beyond), a stream that ends when the opening token expires, and the token passed only in a header (never a URL). No test covers the main-API stream cap or expiry (checked: no test mentions it).

**A second consequence for the UI.** A browser `EventSource` cannot send an `Authorization` header. The UI therefore reads streams with `fetch` and its own parser (`ui/src/lib/sse.js`, design note `docs/design/sse_auth_options.md`), and the token lives in a React ref in memory only (`ui/src/screens/lab/LabScreen.jsx`: `getToken: () => tokenRef.current`). `ui/src/lib/linkState.js` tells the user when live telemetry may be refused for lack of a header. The owner decision on JWT telemetry authentication is listed as open in `docs/release/lead_working_state.md`.

**No Host or Origin guard on the main API.** The Lab API rejects a wrong `Host` in anonymous mode (DNS-rebinding defence, `lab_api.rs`); the main API does not. In anonymous loopback development mode a web page could use DNS rebinding to call DELETE or `/api/control` (G4B-04, OPEN). CORS is an exact-origin allowlist, no wildcard, never credentials (`CorsConfig`; tests `sec03_*`) but CORS is not authentication and does not stop a non-browser caller.

**Health endpoints** (`src/health.rs`, **IMPLEMENTED**, **TESTED** by `m7_health_tests`): `/health/live` is 200 while the process can answer. `/health/ready` is 200 only when configuration is valid, the TCP listener answered a self-probe, and the process is not draining. It does *not* check the origin, the cluster or the cache. A node whose origin is down is still "ready". That is a deliberate narrowness, not a defect, but it means readiness is a weak signal for a load balancer.

::: judge
**JUDGE MAY ASK**

Question: Your control endpoint changes cluster node state. Who can call it, and what stops a demo control from breaking production?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In secure mode it needs the `scenario:operate` role. Without a JWT secret it is anonymous, which is only possible on a loopback address. But I will be honest: it is a demo control, and today it is not isolated from the real router. In a cluster process it operates on the same node objects the router uses, and an unknown event sets every node Active. The register has it as G4B-03, an owner-named open finding. The fix options are to refuse it when a cluster router exists or gate it behind the simulation flag. I would not claim it is safe to expose in production.

**If the judge asks further:**

Ask whether this changes cache correctness. Marking a node Failed changes which node is the routing primary for DELETE and makes the node drop incoming connections at accept time; it does not corrupt cache data. It can make invalidations go to a different node, which the repair machinery then has to reconcile.

**Evidence to know:** `src/api/server.rs::control_handler`; `src/main.rs` (nodes shared with the router); `src/dist/node.rs` accept loop drops sockets in Failed state; G4B-03 (OPEN, owner-named, ODR-4). Status: IMPLEMENTED, not isolated; no cluster test.
:::

## 6.4 Peer-to-peer trust

**Simple version.** Gateways in a cluster tell each other "forget key X". If anyone could send that message, anyone could empty your cache or, worse, keep a node serving old data. So nodes prove who they are with certificates, and the receiving node checks that the sender is on a list.

**Terms.**

- *TLS* encrypts and authenticates a connection. *mTLS* (mutual TLS) makes both sides present certificates.
- *CA* (certificate authority): the party whose signature makes a certificate believable. Here a private cluster CA, with web roots excluded.
- *SAN* (subject alternative name): a certificate field naming the holder. Here exactly one DNS SAN of the form `node-<id>.<cluster>.phantom.internal`.
- *Fingerprint*: a hash of the certificate, pinned in the membership file, so a different CA-signed certificate for the same node id is refused.
- *Admission*: the decision to let an authenticated connection act as a member.

**How it works** (**IMPLEMENTED**, **TESTED**):

1. `ClusterTlsConfig::from_pem` (`src/security/cluster.rs`) builds both server and client configs with TLS 1.3 only, a root store containing only the cluster CA, and a mandatory client-certificate verifier.
2. `VirtualNode::start_tcp_service` (`src/dist/node.rs`) accepts a socket, drops it if the node is Failed or Offline, takes one of 64 handshake permits (`try_acquire_owned`; no permit means the socket is dropped), and allows 1,000 ms for the handshake.
3. It requires a peer certificate, extracts SANs, requires **exactly one**, and parses the node identity.
4. `ClusterMembershipRegistry::validate_peer` checks the cluster id, that the node id is in the registry and `active`, and that the certificate fingerprint matches one pinned for that member.
5. The connection is registered in a `SessionTracker` so a revocation can cancel it.
6. For every frame on a reused connection the identity is checked against the registry again. For `InvalidateRequest` the `originator_id` in the frame must equal the authenticated node id (L3 fix) or the connection is closed.
7. Frame parsing (`src/dist/frame.rs::read_frame`) fails fast on bad magic, rejects an unknown version or type, and rejects a payload longer than 1 MiB before allocating it; keys over 4,096 bytes are refused.

```
accept -> [64-permit semaphore] -> TLS 1.3 handshake (1 s) -> client cert present?
  -> exactly one DNS SAN? -> parse node-<id>.<cluster>.phantom.internal
  -> registry: cluster id, node id, active, fingerprint pinned?
  -> SessionTracker.register -> per-frame re-check + originator == TLS identity
```

**What a trusted member can still do.** Admission answers "is this a cluster member?", not "may this member do *this*?". After admission:

- Any member can send `Tenant` or `FullCache` scope invalidations. The only check is the originator id (`node.rs`, the `InvalidateRequest` arm). A member can therefore flush any tenant or every entry. Peer-supplied tenant strings are not validated, and each new tenant string adds to the epoch map; at 10,000 entries (`MAX_TENANT_EPOCHS`) `try_invalidate_tenant` falls back to bumping the *global* epoch and clearing the map (SEC-4A-01), which is safe against stale resurrection (tested) but turns the next such frame into a full flush. Every tenant invalidation clones the whole map under the `tenant_epochs` mutex (O(n), n up to 10,000). This is DR-027, G4B-05 (owner-named, OPEN) and the same mechanism as G4A-12 (OPEN). It needs a protected-core change plus an independent review.
- `InvalidateRequest` carries a `membership_epoch`, but the receiver does not compare it with its own; it echoes the sender's value in the ack, so the sender's check of the ack is vacuous (G4B-06, owner-named, OPEN). Anti-entropy requests *do* check the epoch (`security_phase4c_antientropy_tests::test_p4c_sec02_mismatched_epoch_rejected`).
- Membership is read once at startup (`load_membership_from_file`; the doc comment says updates need a restart). `ClusterMembershipRegistry::update` and the revocation broadcast exist and are tested (`test_p3c_sec03_automated_membership_revocation_wiring`), but a search of `src/` finds no production caller of `update` (G4B-07, owner-named, OPEN).
- An established connection is never aged out; certificate expiry and rotation are enforced at handshake only, and the pool's idle timer is refreshed by the heartbeat (G4B-08, owner-named, OPEN).
- The `GetRequest` handler fabricates a body. See 6.6 (DR-029).
- The 64-permit handshake semaphore with a 1 s timeout can be held by an attacker who opens sockets and stalls; the accept loop exits permanently on any `accept()` error (G4B-10, OPEN); authenticated sessions and 1 MiB frame buffers are not capped per peer (G4B-36, OPEN).

**The plaintext fallback.** If no cluster TLS config is set, the listener accepts plaintext frames; this is allowed only on loopback and the production `main.rs` path never reaches it. Tests use it deliberately (`known_gaps.rs`, `security_phase3b_framing_tests`). The startup check for it uses the HTTP bind address, not the cluster listen address (G4B-17, OPEN).

**Static membership is a design fact, not a bug.** Nodes are listed in a JSON file; there is no discovery and no join protocol. That is simple and auditable and it is why revocation needs a restart today. Whether cluster members should be mutually trusted is the open owner decision behind DR-027.

::: caution
**Do not say "the cluster is authenticated, so it is safe."** The honest sentence is: "Only certificate-holding members can connect, and each is checked against a pinned list on every frame; what a member may do once admitted is not restricted, and membership cannot change without a restart."
:::

::: judge
**JUDGE MAY ASK**

Question: How do your nodes trust each other, and what if one node is compromised?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Nodes use mutual TLS 1.3 with a private cluster CA. A connecting node must present a certificate with exactly one DNS name that encodes its node id and the cluster id, the node id must be in a membership file and marked active, and the certificate's fingerprint must match the one pinned for it. The gateway re-checks this on every frame, and an invalidation frame's claimed sender must equal the certificate's identity. If one node is compromised, the limit is real: a compromised member is trusted. It can send tenant-wide or full-cache invalidations, which flushes caches and, past 10,000 distinct tenant names, costs every tenant their cached data. It cannot read other nodes' cached bodies through this protocol, because the peer read path only returns what that node itself holds under a restricted key space. Revocation exists in code and tests but is not triggered in a running process; you would remove the member from the file and restart.

**If the judge asks further:**

Press on "cannot read". Peer `GetRequest` is a stub (DR-029, section 6.6): it returns "HIT" for the node's own cached bodies under non-HTTP keys and writes a fabricated `val_node_N` entry for missing ones. Keys starting with `/` are refused, so HTTP entries are not exposed (verified by reading). The residual harm is resource use and namespace pollution; that conclusion is an AI investigation, not independent review.

**Evidence to know:** `src/security/cluster.rs::ClusterTlsConfig::from_pem`, `validate_peer`; `src/dist/node.rs::start_tcp_service`, `process_binary_cache_stream`; tests `security_phase3_cluster_tests` (AT-01..AT-10, SEC-03, SEC-04), `cluster_followup_tests::test_l3_invalidate_request_originator_must_match_tls_identity`; G4B-05/06/07/08, DR-027. Status: IMPLEMENTED, TESTED for admission; authorisation of scopes DESIGNED BUT NOT IMPLEMENTED.
:::

## 6.5 Input validation, body limits, timeouts and connection management

**Simple version.** Treat every byte from outside as hostile until proven otherwise: check size, shape and meaning before using it, and do not wait forever for a slow sender.

![Figure 6.2: The request path of a cache GET in secure mode, with each protection point numbered and the open gaps marked](../assets/ch06_request_path_protections.svg)

Source: `src/api/server.rs::get_cache_handler`, `authenticate_request`, `require_tenant_claim`, `fetch_origin_raw`; `src/core/cache.rs::CacheKey::validate`, `finalize_fetch`; `src/security/upstream.rs`.

**Key validation** (`CacheKey::validate`, **IMPLEMENTED**, **TESTED** by `m7_sec_round2_tests::h1_*`, `parser_robustness_tests::test_ingress_*_boundary_validation`, `canonical_key_roundtrip_tests`). A cache key is `tenant:path` or `tenant:path:?query`. The checks: tenant id 1 to 128 bytes of `[A-Za-z0-9._-]` (so it can never contain the separator colon); path at most 2,048 bytes; query at most 4,096; the whole canonical string at most 4,096 (the cluster frame limit); no byte at or below 0x20 or 0x7f anywhere (stops header injection through percent-decoded `%0d%0a`); no `.` or `..` segments (so `/a/../b` cannot alias another key); no `:?` inside a path (so two different keys cannot produce one canonical string). On top of that the upstream request line is re-encoded by `encode_origin_target`, a second layer.

**Origin response parsing** (`parse_http_response`): conflicting `Content-Length`, `Content-Length` with chunked, unsupported transfer encodings, bad or overflowing chunk sizes, truncated bodies and close-delimited bodies are all rejected or not cached; headers over 64 KiB and bodies over 2 MiB are rejected (`MAX_HEADER_BYTES`, `MAX_BODY_BYTES`); redirects are not followed. Tests: `parser_robustness_tests` (including two proptest cases that feed arbitrary bytes and assert no panic), `security_phase2_upstream_tests`.

**Timeouts toward the origin** (`fetch_origin_raw`): 5 s to connect, 5 s for the TLS handshake, 5 s to read the response; the read is wrapped in `take(MAX_TOTAL_RESPONSE_BYTES + 1)` so a hostile origin cannot stream forever.

**Body limits toward the gateway.** `create_app_with_lab` applies `DefaultBodyLimit::max(512 * 1024)` to the whole router; `test_m1_t26_ingress_request_body_limit` sends a 600 KiB body and expects 413. That proves the size cap, not time: the test sends at full speed. A client that promises 500 KiB and trickles bytes holds a connection and a buffer, and there is no body-read timeout, no `ConcurrencyLimitLayer` and no connection cap in `server.rs`. `get_cache_handler` and `delete_cache_handler` take `_body: axum::body::Bytes` and `control_handler` takes `Json(payload)`, so the extractor reads the body *before* the handler authenticates (verified in code in the current tree). This is **G4B-01**, High, owner-named, OPEN. The G4B reviewer did not reproduce it at run time and did not verify hyper's and axum's own defaults at the pinned versions, so the ordering is **verified in code** and the slow-body exhaustion is **UNVERIFIED** as a measured effect. The Lab API shows the intended pattern: `runs_start` authenticates first, then reads the body inside `tokio::time::timeout(BODY_READ_TIMEOUT, to_bytes(body, MAX_BODY_BYTES))` (5 s, 16 KiB).

::: caution
**Operational mitigation, not a code fix.** Running the gateway behind a reverse proxy that enforces request timeouts and connection limits would blunt G4B-01 in practice. The register records "authenticate before body extraction, add timeouts and limits, or document a mandatory fronting proxy" as the owner's choice. Nothing in the repository today requires or documents such a proxy.
:::

**Connection management.** Inbound HTTP: whatever hyper and axum defaults give, no explicit limits. Outbound origin: one fresh TCP (and TLS) connection per origin request with `Connection: close`; no origin connection pool. Cluster: a pool per node (`pool.rs`: at most 8 total and 4 idle connections per destination, 30 s idle timeout, 5 s heartbeat), with capacity reserved atomically (`test_p3c_sec01_concurrency_capacity_cap_and_no_leak`). Invalidations from all tenants share those 8 connections, so a tenant that floods DELETEs can saturate the local pool and others see 503 `PrimaryRateLimited` (G4B-11 part, OPEN).

::: caution
**Misleading status name.** `InvalidationStatus::RejectedRateLimit` (sent to clients as `PrimaryRateLimited`) is not a rate limiter. `router.rs` returns it when the *local* connection pool stays saturated for half the leg budget: it is backpressure on a shared resource, and it deliberately does not mark the peer Suspect. There is no rate limiting anywhere in the product (G4B-11); do not read the name as evidence that there is.
:::

::: judge
**JUDGE MAY ASK**

Question: What happens if someone sends a malicious request: a huge body, a slow body, a crafted path?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Crafted paths and keys are handled well: the key is validated against a strict alphabet and length limits, control characters and dot-segments are rejected with 400, and the path is re-encoded before it goes upstream, so header injection and path traversal into another tenant's key space are blocked. There are tests for those. A huge body is capped at 512 KiB and gets 413. A slow body is the weakness: there is no read timeout or connection cap on the main API, and the body is read before authentication. That is finding G4B-01, the one High-severity security finding, still open and owner-named. The fix is known and already done in our Lab API; it has not been applied to the main API, and I would not call the main API hardened against slow-client exhaustion.

**If the judge asks further:**

Ask what you would do first. Move authentication ahead of body extraction by taking the raw request in the handler, add a body-read timeout and a connection limit, and add the two regression tests the reviewer proposed (stalled sockets must not block a legitimate request; an unauthenticated `POST /api/control` with invalid JSON must return 401, not 400). Say the fix needs the owner's decision because `server.rs` changes are application-code changes.

**Evidence to know:** `src/api/server.rs::create_app_with_lab` (`DefaultBodyLimit`), `get_cache_handler` signature, `src/api/lab_api.rs::runs_start`; tests `m1_http_tests::test_m1_t26_ingress_request_body_limit`, `m7_sec_round2_tests::h1_illegal_bytes_in_path_are_400_and_origin_receives_nothing`; G4B-01 (OPEN, owner-named). Status: key validation TESTED; slow-body protection absent.
:::

## 6.6 Tenant isolation and cache poisoning

**Simple version.** Tenant isolation means customer A never sees customer B's cached answer. Cache poisoning means an attacker makes the cache store a wrong answer that other users then receive.

**How isolation works.** The tenant is the first part of the key (`CacheKey::new` uses `__default__` when no tenant is given). In secure mode the tenant comes only from the verified token; a client's `x-tenant-id` header is ignored and is not forwarded (`get_cache_handler`: `else if state.security.is_some() { None }`). In development mode (no secret) the header is trusted, because there is no identity to compare it with. Tenant ids exclude `:`, so the first colon is always the separator, which prevents one tenant string from impersonating another's key prefix. Epochs (Chapter 4) make `invalidate_tenant` affect only that tenant's entries.

**The `__default__` question (G4A-19 versus the G4B refutation).** G4A-19 says a token with tenant `__default__` would share the tenant-less key space; G4B's refuted-hypothesis list says `require_tenant_claim` refuses that value. Reading the current code, both are right for different paths. In secure mode `require_tenant_claim` returns 403 `ReservedTenant` for `__default__` and 403 `MissingTenant` for an empty or absent claim, on GET and DELETE (`m7_sec_round2_tests::m1_tenantless_and_reserved_tenant_tokens_are_403_for_cache_read_and_invalidate`). But `CacheKey::validate` accepts the literal `__default__`, so in *development* mode `x-tenant-id: __default__` addresses the tenant-less space, and a peer invalidation frame can name it. G4A-19 stays OPEN because the proposed fix is to reject the literal in `validate()`. Net: **TESTED** in the secure HTTP path; **not closed** elsewhere.

**What tests prove and do not prove.** `security_phase1_tests::test_phase1_at05_and_at06_tenant_cryptographic_isolation`, `m7_sec_fixes_tests::sec01_tenant_boundary_and_anonymous_cannot_read_authenticated_entries`, `m7_sec_round2_tests::e2e_m2_tenant_isolation_and_denials_on_the_cached_path` and `m7_security_review_tests::m7_secure_mode_ignores_spoofed_tenant_header` prove isolation on the tested HTTP paths with a mock origin. They do not prove isolation for the diagnostics endpoints (events are not tenant-scoped; any Observer sees them), for the cluster path (a member can address any tenant), or under resource pressure from another tenant (there are no per-tenant quotas; the byte budget is shared; G4B-11, G4A-23).

**Poisoning paths we looked for, and what stops each:**

| Poisoning idea | Protection | Evidence |
|---|---|---|
| A client header changes the origin's answer and the variant is stored for everyone | Only gateway-chosen headers go upstream: a validated `x-request-id`, the key's tenant, optional `Authorization` when `PHANTOM_FORWARD_UPSTREAM_AUTH`, stored validators. Client `x-*`, `Accept`, `Cookie` never forwarded. Contract: origins must not vary on client headers. | `fetch_origin_raw`; `m7_sec_round3_tests::m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant` |
| An authenticated personalised answer is stored and served to others | With forwarding on, authenticated reads are BYPASS-AUTH. With forwarding off (default), an answer is stored only if the origin marks it shareable (public / s-maxage / must-revalidate). | `m7_sec_round2_tests::e2e_m2_*`; `security_phase1_tests::test_phase1_at08_*` |
| `Set-Cookie` from the origin replayed to another user | Not stored or replayed (only content type and ETag are kept) | `m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit` |
| A path like `/a/../b` aliases a victim's key | `.`/`..` segments rejected | `l1_dot_segments_over_http_are_400_and_never_reach_the_origin` |
| A truncated or ambiguous origin response is stored | Not cached | `security_phase2_upstream_tests::test_p2_truncated_response_not_cached` and siblings |
| A race stores a pre-invalidation body | Generation fencing; one known hole | 6.9 (G4A-01) |
| A peer writes into the shared cache (DR-029) | See below | `known_gaps::pin_kg6_node_get_request_fabricates_missing_key_body` |

**DR-029 in plain terms.** The cluster `GetRequest` handler (`node.rs`) answers from the node's serving cache. On a miss it does not report "not found": it builds a body `val_node_<id>`, completes the fetch as a success and so *stores* that fabricated 200 response (TTL 60 s) under tenant `__default__`. In the mTLS cluster path that cache is the HTTP serving cache (`main.rs` builds the local node with `new_with_cache`). The DR-029 investigation (AI, MEASURED on a debug loopback harness) found HTTP clients cannot reach those entries, because the handler refuses keys starting with `/` and every HTTP key starts with `/` (`test_m2_cluster_get_request_keys_in_the_http_namespace_or_invalid_are_rejected`). Residual issues: fabricated data in a production cache (against the project's no-fabricated-data rule); a member can fill the byte budget and evict real entries (20 of 20 unvisited HTTP entries were evicted in a small test cache); no per-peer quota. No gateway code ever sends a `GetRequest`; only tests do. Status: **IMPLEMENTED** (the stub), **TESTED** (the pin test asserts the fabricated behaviour: a characterisation, not approval); DR-029 / G4B-30 OPEN pending an owner decision and independent review.

::: judge
**JUDGE MAY ASK**

Question: How is tenant isolation enforced, and could one tenant ever see another's data?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The tenant is part of every cache key, and in secure mode it comes only from the signed token, never from a header. Tenant names are restricted so one cannot spoof another's prefix, and an invalidation bumps a per-tenant epoch so it only affects that tenant. Tests cover tenant-claim handling, spoofed headers, and a tenant reading another's entry. The limits: isolation is logical, inside one process and one shared byte budget; there are no per-tenant quotas, so a heavy tenant can evict others. A cluster member can address any tenant in an invalidation. And the literal tenant name `__default__` is refused in secure mode but the key validator still accepts it, which is why G4A-19 is not closed.

**If the judge asks further:**

Ask about cache poisoning. Explain the header whitelist: the only inputs that influence what the origin returns are the path, the query and the tenant, all part of the key, so a client cannot choose a variant that other users get. The contract is that origins must not vary responses on other client data.

**Evidence to know:** `src/core/cache.rs::CacheKey::new`, `validate`; `src/api/server.rs::require_tenant_claim`; tests named above; G4A-19, G4B-05, G4B-11, G4B-30 / DR-029. Status: TESTED on HTTP paths; quotas DESIGNED BUT NOT IMPLEMENTED.
:::

## 6.7 Data exposure and token handling

**What leaks, and where.**

- *Errors.* 401 bodies are generic and do not echo the `kid` or parser detail (`m7_sec_round2_tests::low_401_body_is_generic_and_does_not_echo_kid_or_parser_detail`). The `X-Auth-Error` header does name a class (`ExpiredToken`, `InvalidAudience`, ...), which helps a legitimate client and also tells an attacker which check failed.
- *Diagnostics.* Observers can read diagnostic events that may contain internal error text such as IPs and OS error strings (G4B-21, OPEN). They are not tenant-scoped.
- *Origin headers on the bypass path.* Response headers from the origin are copied except hop-by-hop/framing ones; no `nosniff` is added to cached responses (G4B-16, OPEN).
- *Secrets.* `PHANTOM_JWT_SECRET` is read once; the effective-config report, `Debug` and config errors redact it (test `secrets_never_appear_in_report_debug_errors_or_warnings`). The M7 review did not test logs, SSE or error pages for the secret, and the secret remains in the process environment. The Lab refuses to start if the sealed-seed salt variable is present in its environment (`refuse_if_sealed_salt_present`). Lab artifacts may include absolute paths and command lines (F17, OPEN).
- *Ingress TLS* is not restricted to 1.3 although a startup message says "TLS 1.3" (G4B-20, OPEN): `TlsServerConfig::from_pem_file` uses the library defaults. The *cluster* TLS is TLS 1.3 only and is tested (`test_p3_sec04_strict_tls13_rejection_of_tls12_client`).

**Token handling.** Tokens travel only in the `Authorization` header (the Lab API refuses tokens in URLs by design). The UI keeps the token in memory (a React ref) and never writes it to storage. `ui/src/lib/labToken.js` only decodes the `roles` claim for a display hint; its header says the page never verifies, stores or logs a token. `localStorage` is used for the theme (`ui/src/lib/theme.js`), a navigation fold state (`components/Shell.jsx`) and the saved gateway address, a URL and nothing else (`ui/src/lib/gatewayConfig.js`); a grep of `ui/src` found no other storage use. A raw token without the `Bearer ` prefix is accepted by the gateway (G4B-14).

## 6.8 Resource exhaustion, origin overload and SSRF

**Simple version.** Denial of service does not need a break-in; it needs a way to make you spend more than the attacker spends. The question for each resource is "what is the bound, and who sets it?"

**Bounds that exist** (**IMPLEMENTED**; most **TESTED**):

| Resource | Bound | Where |
|---|---|---|
| Cache bytes | logical byte budget, default 256 MiB (`PHANTOM_MAX_CACHE_BYTES`); oversize bodies never stored; invariants checked by `audit_bytes` | `core/cache.rs`, `config.rs`; `byte_accounting_tests`, `memory_bound_soak_tests` |
| Waiters per key | 1,000 (`max_waiters_per_key`) then 503 | `core/cache.rs` |
| Tenant epoch map | 10,000 entries then global-epoch fallback | `MAX_TENANT_EPOCHS`; `security_phase4a_invalidation_tests::test_p4a_sec01_*` |
| Origin response | 64 KiB headers, 2 MiB body, 5 s phases | `server.rs` |
| Ingress body | 512 KiB | `DefaultBodyLimit` |
| Cluster frame | 1 MiB payload, 4,096-byte key | `dist/frame.rs` |
| Cluster handshakes | 64 concurrent, 1 s each | `node.rs` |
| Per-peer delivery | 1 in-flight, queue 256; overflow goes to the repair ledger | `router.rs` (`PER_PEER_INFLIGHT`, `PER_PEER_QUEUE`) |
| Repair ledger | 1,024 entries and 256 KiB of key bytes per peer, 16 tenant flushes, then collapse to a bigger flush | `router.rs` (`REPAIR_*`); `cluster_prewarm_repair_tests::test_ledger_overflow_*` |
| Replay ring | 4,096 entries / 4 MiB | `anti_entropy.rs` |
| Lab API | 16 SSE streams, 16 KiB bodies, 64 MiB artifacts, caps on seeds/cells/reps | `lab_api.rs` |

**Bounds that do not exist** (each an open finding; none is covered by a test because the behaviour is absent):

- Number of inbound HTTP connections, and time to send a body (G4B-01).
- Number of `/api/events` streams (G4B-02).
- Origin concurrency **by default**. The origin permit gate is present only when `PHANTOM_ORIGIN_CAP_MAX` is set *and* `PHANTOM_INTELLIGENCE` is `canary` or `active` (`config.rs` key text; `origin_cap_tests::off_mode_is_uncapped`). Coalescing limits *one key* to one fetch; distinct keys are not limited. A token holder can send many distinct cache-busting query strings, or `x-cache-bypass: true` (which needs only `cache:read`), and each reaches the origin (G4B-11, OPEN). The bypass header is never forwarded to the origin.
- Per-tenant request or invalidation rate (G4B-11, SEC-M7-11).
- Cluster sessions per peer (G4B-36), anti-entropy responder work (G4B-22), repair-ledger CPU under adversarial key patterns (G4B-37).
- The epoch-map clone per tenant invalidation, quadratic-looking under a peer flood (G4B-05/G4A-12).
- Process-wide panic containment: handlers use `lock().unwrap()` on shared mutexes (for example in `control_handler`), so a poisoned lock panics the request task (G4B-28/G4A-21, OPEN).

**Origin overload protection that does exist.** When the cap *is* configured, the gate is a counter of permits: a request that cannot get a permit is served stale if eligible, otherwise 503 (`origin_cap_tests::saturation_serves_stale_if_eligible_else_503`); permits are released on error, timeout, client disconnect and future cancellation. The cap value can adapt by AIMD, but Chapter 5 explains why the evidence says it behaved like a fixed cap. The measured cost of a cap is in 6.13, item M5: static cap 24, peak origin concurrency down 38.3 percent (95 percent BCa CI 36.7 to 40.2) at a 4.9 percentage-point client success cost, `TRADE_OFF_FAILURE`; loopback, scripted origin, 10 paired repetitions.

**SSRF (server-side request forgery).** Simple version: if an attacker can make your server fetch a URL, they can reach things only your server can reach: cloud metadata addresses, internal admin ports, databases.

PhantomCache removes the largest share of this risk by design: the origin host is fixed by the operator in `ORIGIN_URL`, and a request can influence only path and query. The remaining protections guard against a misconfigured or rebinding origin name:

1. `UpstreamPolicy::validate_target` (**TESTED**: `m7_upstream_policy_validate_target`, `security_phase2_upstream_tests`): only http/https; no userinfo, query or fragment in the base URL; hard-blocked ports (SSH, SMTP, Redis, Memcached, databases, Docker, etcd, Consul and others); privileged ports outside 80/443 blocked; with an allowlist, the origin must match; plain http to a non-loopback host needs both `PHANTOM_ALLOW_INSECURE_UPSTREAM` and an allowlist entry.
2. `resolve_and_pin_origin`: resolve the name, run **every** returned address through `PreConnectIpFilter` (loopback, private, link-local, metadata, IPv4-mapped and transition IPv6 forms), reject the whole answer if any one is forbidden, then connect to the one validated address, so a DNS answer cannot change between check and connect. The same check runs at startup for `ORIGIN_URL` (`config.rs`).
3. Redirects are not followed; the gateway never forwards the client's `Authorization` unless configured.

Gaps: the filter misses some reserved ranges (198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24, Teredo, `fec0::/10`, an Azure wire address; G4B-15 / SEC-M7-10, OPEN); `PHANTOM_ALLOW_LAN_ORIGIN` deliberately relaxes the private-range rule; DNS rebinding beyond pinning the first answer was reviewed by reading, not by a rebinding test (M7 review).

::: judge
**JUDGE MAY ASK**

Question: How do you prevent resource exhaustion and protect the origin from overload?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Memory is bounded by a byte budget with exact accounting, and many internal structures have fixed caps: waiters per key, the tenant epoch map, frames, repair queues, the origin response size and timeouts. Many requests for one key collapse into a single origin fetch. An optional origin concurrency cap sheds load with stale-serving or 503, and tests show permits are always released. What is missing: the cap is off by default and applies per process, not per tenant. Distinct cache-busting keys, or the bypass header, reach the origin without limit, and there is no rate limit or connection cap on the HTTP side. Those are open findings G4B-01, G4B-02 and G4B-11. I would not claim DoS resistance.

**If the judge asks further:**

Ask about the cost of the cap. We have a real loopback experiment: a static cap reduced peak origin concurrency but lowered client success because shed requests count as failures. We report that as a trade-off, not a win. Latency percentiles over all requests include fast 503s and do not show improvement for served clients.

**Evidence to know:** `src/intelligence/executor.rs` origin gate; `tests/integration/origin_cap_tests.rs`; `lab-results/scenario_b/*/summary.json` and `docs/demo/scenario_b_interpretation_notice.md`; G4B-11 (OPEN). Status: bounded memory IMPLEMENTED and TESTED; origin rate limiting DEFERRED (SEC-M7-11 queued; no code exists).
:::

::: judge
**JUDGE MAY ASK**

Question: Could a user make your gateway fetch an internal address (SSRF)?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

A user cannot choose the host: the origin is one configured URL, and users supply only a path and query, which are validated and re-encoded. If the origin name itself resolved to a private or metadata address, the gateway resolves it, checks every returned address against a block list, and connects to the exact address it checked, so DNS changing between the check and the connection cannot help. Dangerous ports like SSH, Redis and databases are blocked unconditionally, redirects are not followed, and the client's credentials are not forwarded by default. The known gap is that the block list is missing a few rarely used reserved ranges, tracked as G4B-15, and the rebinding defence was reviewed by reading plus the pinning design, not by an automated rebinding test.

**If the judge asks further:**

Ask about the LAN switch. `PHANTOM_ALLOW_LAN_ORIGIN` exists for private origins and is flagged high-risk in the config reference; network binds with an insecure upstream refuse to start without an allowlist.

**Evidence to know:** `src/security/upstream.rs::resolve_and_pin_origin`, `PreConnectIpFilter`, `UpstreamPolicy::validate_target`; tests `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses`, `test_p2_ssrf_cloud_metadata_blocked`, `test_p2_ssrf_mixed_dns_blocked`; G4B-15. Status: IMPLEMENTED, TESTED; range gaps OPEN.
:::

## 6.9 Failure recovery, lost invalidations and races

### 6.9.1 What "failure" means here

A failure model says which faults you design for. For the invalidation path, the stated model in the project documents is: packet loss, transient partitions, a slow or briefly unreachable peer, with the *issuing gateway process staying alive*. Process crash of the issuer is outside that model today. The claims ledger says so explicitly: C6 "remains qualified to single-host loopback AND to the issuer-alive failure model" (`docs/research/claims_ledger.md`, C6 qualifier).

### 6.9.2 One DELETE, step by step

![Figure 6.3: What happens to one DELETE when a peer is unreachable, and the case that loses the invalidation](../assets/ch06_lost_invalidation_flow.svg)

Source: `src/api/server.rs::delete_cache_handler`, `src/dist/router.rs::invalidate_key_across_cluster`, `RepairLedger`, `start_anti_entropy_worker`, `src/main.rs` shutdown path.

1. `delete_cache_handler` authenticates, builds the key and calls `cache.delete` on the *local* cache first. The code comment ("Guarantees linearizability", `server.rs`) is stronger than anything proven. What the tests support is per-node read-your-delete on the issuer: `distributed_consistency_tests::test_m3_04a_local_linearizability_immediate_eviction` does GET (MISS), GET (HIT), DELETE (204), then asserts the next read on the same gateway is a MISS with the new version.
2. `invalidate_key_across_cluster` sends the invalidation to the rendezvous primary with a 150 ms per-attempt timeout (`PHANTOM_CLUSTER_TIMEOUT_MS`). A silent primary is marked Suspect and the first replica acts as primary. (Full fan-out walk-through: Chapter 5.)
3. **HTTP 204 means "local purge plus primary or acting-primary acknowledgement".** It does not mean every peer applied it (G4B-09, OPEN; the register asks for exact wording in documentation). `x-cache-warning: PrimaryFailover` marks a failover.
4. All other active members are told *asynchronously* by per-peer workers (one in flight, queue of 256). A full queue, an error or a stopped worker diverts the job to the `RepairLedger`.
5. A repair worker re-sends owed jobs when the peer answers (64 per tick) with the *same* stream sequence number, so the receiver's contiguous-sequence tracker has no permanent gap. The anti-entropy worker (250 ms tick in `main.rs`) lets a peer notice gaps in a sender's sequence and pull a replay of that sender's own records. A recovering peer bypasses its cache for reads until reconciled (`security_phase4c_antientropy_tests::test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits`).
6. Coverage of the happy and repair paths is broad: `cluster_followup_tests` (about 40 tests), `cluster_prewarm_repair_tests`, `distributed_consistency_tests::test_h2_failed_delete_still_invalidates_all_peers_after_heal`, `security_phase4b_router_tests`, with in-process nodes and a fault-injecting TCP proxy (`tests/support/fault_proxy.rs`: delay, drop, blackhole, partitions). These are loopback, a handful of nodes, debug profile unless stated. They are not multi-host, long-running or crash tests.

### 6.9.3 The lost-invalidation case

The `RepairLedger` and the replay ring live in the issuing process's memory. The shutdown path in `src/main.rs` stops the ingress listener and exits ("cache is in-memory, nothing to flush"); it does not drain the router. If the issuer crashes, or receives SIGTERM, while a peer is owed an invalidation, that debt disappears. The peer keeps its old entry until it expires on its own. How long that is depends on the origin: the literal 60 s default applies only when the origin sends no freshness information (`PHANTOM_DEFAULT_TTL_MS` is **not wired**; `server.rs` passes a literal `60_000`), and an origin `max-age` or `s-maxage` has no gateway ceiling. `PHANTOM_MAX_STALE_MS` caps only the stale window beyond freshness. So the old claim "staleness is bounded by 60 s" is not a general bound. **No test kills the issuer** (searched: no test or doc names such a test). The M3 durable-log design (`docs/research/m3_durability/`, env-gated log, shutdown drain) was prototyped and passed its own sandbox tests; it is **PROTOTYPE ONLY**, not integrated, and needs owner approval and an independent review.

::: example
**Worked example.** Origin sets `max-age=300` for `/price/42`. Gateways A, B, C all hold it. A client DELETEs it at A. A purges locally and gets the primary's ack (B is primary). C is unreachable, so A records "C is owed `acme:/price/42`" in memory. One second later A is killed. C never hears. C serves the old price until `max-age=300` elapses, up to 300 s, because nothing remembers that C was owed. If A had stayed alive, the repair worker would have delivered the frame as soon as C answered.
:::

**Measured propagation delay.** The claims ledger (C6 amendment, 2026-10-30) records `cluster_propagation_latency_tests` on loopback (one host, Windows 11, 24 logical CPUs, **debug profile**, three gateway processes with mTLS): warm path P50 about 0.3 ms, P99 about 10 to 14 ms, max 15 to 21 ms in four runs of 200 trials by 2 peers; N=1000: P99 19.8 ms, max 49.2 ms; the first invalidation to a destination (cold connection plus mTLS handshake) 35 to 60 ms, up to 117 ms under concurrent copies. The earlier "P(stale)=0 after 50 ms" came from a fixed sleep and is **not supported**. These numbers are **reported** from the ledger, not re-run here; the CSV logs are under `D:\Temp` and not retained in the repository (UNVERIFIED). A connection pre-warm worker was added since (`start_prewarm_worker`, `test_prewarm_*`); its effect on the cold-start number is not recorded as measured.

### 6.9.4 The stale-body race (G4A-01)

**Simple version.** A request starts fetching a page from the origin. While it waits, someone invalidates that page. When the fetch finishes, the cache must not store the old page. The fence compares a counter (the *generation* and the epochs) seen when the fetch began with the counter now.

**The hole.** In `finalize_fetch` (`src/core/cache.rs`) the fence compares the epochs stamped on the entry placeholder with the current ones. If they match, the code stores the resource and then calls `get_effective_epochs` a *second* time and stamps the entry with those, possibly newer, epochs. An invalidation that bumps the tenant or global epoch between the check and the second read leaves a pre-invalidation body stamped as current. The epoch bump does not take the shard mutex, so nothing orders it against that window (mechanism: Chapter 4, section 4.5). G4A-01 is High, owner-labelled **UNCONFIRMED**, disposition UNDER-INVESTIGATION. The RACE agent's sandbox work (`docs/research/g4a01_race/status.md`, AI second look) **reproduced it deterministically with an injected yield point** in a debug build (two failing tests, one passing control) and **did not reproduce it** in a 20 s three-thread unhooked stress (about 3.16 million fills, 0 violations); the window is nanoseconds wide, and absence of a hit is not evidence it is unreachable. The proposal (stamp the epochs captured by the fence check) is a sandbox patch under which all four sandbox tests pass. **It has not been applied** to `src/`; the current code still re-reads. Related: G4A-08 (a request after an invalidation can join a pre-invalidation in-flight fetch; limitation L3) and G4A-17 (the epoch pair read is not atomic against `invalidate_all`). Fencing changes need a failing-first test and an independent review (CLAUDE.md section 5).

::: judge
**JUDGE MAY ASK**

Question: What happens if an invalidation is lost or delayed? Can two nodes serve different values?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes, two nodes can briefly serve different values, and I can say exactly when. After a DELETE, the issuing gateway misses immediately. The primary has acknowledged before we return 204. Other nodes are told in the background, with retries and a repair record for any peer that cannot be reached, and a pull mechanism that detects gaps. On a warm loopback cluster we measured propagation mostly in milliseconds, with a first-use cold start of tens of milliseconds, in debug builds. During that window another node can serve the old value. If a peer is down, it can stay stale until repaired. The case we do not handle: if the issuing gateway crashes before the peer is repaired, the record of what was owed is in memory only and is lost, so that peer stays stale until the entry's own expiry. A durable log is designed and prototyped but not integrated. So I claim eventual repair while the issuer stays alive, not a bounded staleness guarantee.

**If the judge asks further:**

Ask what the bound is. State the failure model (issuer alive, loopback), say the measured window is milliseconds on warm connections, and say there is no general bound if the issuer dies. The test that would falsify the issuer-alive claim is a multi-process test that blackholes a peer, sends DELETE, then kills the issuer; it does not exist.

**Evidence to know:** `src/dist/router.rs::invalidate_key_across_cluster`, `RepairLedger`; `src/main.rs` shutdown; tests `distributed_consistency_tests`, `cluster_followup_tests`, `cluster_propagation_latency_tests`; claims ledger C6 amendment and qualifier; G4B-09 (OPEN); M3 design (PROTOTYPE ONLY). Status: PARTIAL.
:::

::: judge
**JUDGE MAY ASK**

Question: Is there a race that could resurrect a deleted value?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There is one suspected race, the first High finding in our own review, and we are treating it as under investigation rather than fixed. A fetch that started before an invalidation checks a version counter when it finishes, but reads the epoch counters a second time after the check. An invalidation that lands in that gap can let the old body be stored as current. An agent reproduced it only with an artificial pause in a sandbox copy; a short unhooked stress run found nothing. A proposed fix is written and passes its sandbox tests, but it is not in the product code and has not been independently reviewed. So I describe fencing as designed to prevent stale writes, tested for the common orderings, with one known timing hole.

**If the judge asks further:**

Ask why not just fix it. Because the project rule says fencing changes need a test that fails without the change and an independent review, and the owner asked us to investigate rather than auto-fix an unconfirmed finding.

**Evidence to know:** `src/core/cache.rs::finalize_fetch`, `get_effective_epochs`; `docs/research/g4a01_race/status.md`, `proposal.patch`; `security_phase4a_invalidation_tests::test_p4a_inflight_fetch_fencing_prevents_stale_writes` (passes today; does not exercise the gap); G4A-01 (UNDER-INVESTIGATION), G4A-08, G4A-17. Status: IMPLEMENTED with a known hole; fix PROTOTYPE ONLY.
:::

## 6.10 Memory safety and supply chain

**Memory safety.** Rust prevents use-after-free and data races in safe code; the question is how much `unsafe` there is. In `src/`, `unsafe` occurs only in `src/api/lab_api.rs`, in a `#[cfg(windows)]` module that hand-declares a few Win32 functions (job objects that kill child gateways when the Lab dies; process reaping by image path and creation time). It has five `unsafe` blocks, each with a `SAFETY:` comment, and two `extern "system"` blocks (verified by grep in the current tree). There is no `#![forbid(unsafe_code)]`. `benches/support/alloc_counter.rs` has an `unsafe impl GlobalAlloc` that wraps the system allocator to count allocations (bench-only). Dependencies (tokio, rustls, ring, aws-lc-sys) contain their own `unsafe` and C code, which we did not audit. **IMPLEMENTED**: no `unsafe` in the cache, security, cluster or HTTP code. **UNVERIFIED**: correctness of the Win32 FFI beyond reading; no Miri or sanitiser run.

**Panics and poison.** Memory safety is not the same as no crash. `std::sync::Mutex` poisons if a thread panics while holding it; the core cache recovers with `into_inner` and counts it (`lock_poison_recovered`) but telemetry mutexes and handler code call `.lock().unwrap()` (G4A-21, G4B-28, both OPEN), and there is no `CatchPanicLayer`. A panic in one handler task does not take down the process (tokio isolates tasks) but a poisoned shared mutex would make later requests that touch it panic.

**Supply chain.** `Cargo.lock` lists 288 packages (counted by `[[package]]` entries). Direct dependencies include tokio ("full" features), axum 0.7 (with the unused `ws` feature), tower-http (with unused `fs`), axum-server with `tls-rustls`, rustls 0.23, tokio-rustls, webpki-roots, jsonwebtoken 9, ring 0.17, serde and tracing; dev-only: reqwest (pulls `native-tls`/`openssl` bindings), rcgen, criterion, hdrhistogram, proptest. Two TLS crypto providers are present: `ring` (direct) and `aws-lc-rs` (rustls default). **Nobody has run `cargo audit` or `cargo deny`**; yanked or vulnerable status is **not determined** (G4C-DEP, OPEN, release gate; M7 review "Not done"). A CI workflow (`.github/workflows/ci.yml`) runs fmt, check, clippy with `-D warnings` and tests on Ubuntu, plus a frontend job; it has no advisory step. Whether CI has ever run: UNVERIFIED (the tree is uncommitted; CI triggers on pushes and pull requests to `main`).

::: judge
**JUDGE MAY ASK**

Question: Is the system memory-safe, and how do you manage third-party risk?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The code is Rust, and our own cache, security and cluster code contains no `unsafe`. The only unsafe code in the product is a small Windows-only block of operating-system calls in the Lab, used to kill child processes cleanly; each block is commented. Third-party crates have their own unsafe code that we have not audited. For dependencies, we pin versions in `Cargo.lock` (288 packages), keep TLS in rustls rather than OpenSSL for the product path, and I can name the open gaps: nobody has run a vulnerability scan such as `cargo audit` yet, some features such as websockets and file serving are enabled but unused, and there are two crypto providers. The scan is a release gate that needs a human decision.

**If the judge asks further:**

Ask about panics. Rust stops memory corruption, not logic failure. We have findings about lock poisoning handling and no catch-panic layer, tracked as G4A-21 and G4B-28.

**Evidence to know:** `grep unsafe` result (lab_api.rs Windows module; benches alloc counter), `Cargo.toml`, `Cargo.lock`, `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (G4C-DEP). Status: IMPLEMENTED; scan UNVERIFIED.
:::

## 6.11 The security and failure-scenario matrix

How to read the matrix. *Protection that exists* is only what the code does today (verified by reading the symbol). *Tests* names what exercises it; a test proves only what its assertion says. *Unresolved* gives the finding ID and its disposition in `docs/release/g4/cycle1_findings_disposition_register.md` (last reviewed 2026-10-30 15:42 IST). "OPEN*" means OPEN and owner-named in ODR-4 (must not be dismissed). Review line numbers have drifted; every row was re-checked against symbols in the current tree and none of the cited High or Medium findings is fixed. Each row is split across two tables with the same row numbers so that both fit a page.

**Table 6.11a. Threats and the protections that exist.**

| # | What could go wrong, and how | Protection that exists | Where (file::symbol) |
|---|---|---|---|
| S01 | Anonymous caller reads or deletes cache data: Request without a token | Fail-closed startup; with a secret, 401/403 before any cache or origin work | `security/mod.rs::validate_startup_security`; `server.rs::get_cache_handler`, `delete_cache_handler`, `authenticate_request` |
| S02 | Forged or tampered token: alg=none, wrong alg, bad signature, wrong iss/aud, expired, unknown kid | HS256 only; library verification plus explicit iss/aud re-check; unknown kid rejected | `jwt.rs::JwtValidator::validate` |
| S03 | Stolen or leaked token reused: Token valid until `exp`; no `nbf`, `jti`, max lifetime or revocation | Short leeway only (10 s) | `jwt.rs` |
| S04 | Privilege confusion by role alias: `requester` grants read and invalidate; bare token without `Bearer`; case-insensitive | Documented, pinned by a characterisation test | `jwt.rs::has_role`; `authenticate_request` |
| S05 | Wrong-role caller operates the system: Observer calls DELETE, reader calls control | Per-route role check, 403 with no side effects | `authenticate_request` at each handler |
| S06 | Control endpoint breaks the live router: `POST /api/control` with NODE_FAILURE or an unknown event | Operator role in secure mode | `server.rs::control_handler`; `main.rs` nodes shared |
| S07 | Management stream exhausts the gateway or outlives revocation: Many `/api/events` connections; token expires mid-stream | Authenticated once at open | `server.rs::sse_handler` |
| S08 | DNS rebinding drives local API: Web page rebinds to 127.0.0.1 | CORS exact-origin allowlist (not a defence against non-browsers) | `server.rs::CorsConfig` |
| S09 | Unauthenticated slow or huge body: Trickled body; Content-Length 500 KiB | 512 KiB cap (413); hyper/axum defaults | `server.rs::create_app_with_lab` |
| S10 | Header or path injection to origin or into another key: `%0d%0a`, `..`, `:?`, non-ASCII | Strict `validate`; re-encoded target; only whitelisted headers sent | `cache.rs::CacheKey::validate`; `server.rs::encode_origin_target`, `fetch_origin_raw` |
| S11 | Tenant A reads tenant B: Spoofed `x-tenant-id`; tenant-less or `__default__` token | Tenant from token only; reserved/empty tenant is 403; tenant alphabet excludes `:` | `server.rs::require_tenant_claim`; `cache.rs::CacheKey::new` |
| S12 | Cache poisoning through client-controlled headers: Variant chosen by a header is stored for all | Only gateway-chosen headers go upstream; `Set-Cookie` not replayed; non-shareable authenticated answers not stored | `server.rs::fetch_origin_raw`; storage allowlist |
| S13 | Cluster peer plants data (DR-029): Authenticated member sends `GetRequest` for a missing key | Keys starting `/` or failing `validate` refused; mTLS | `node.rs::process_binary_cache_stream` (GetRequest arm) |
| S14 | Compromised member flushes tenants or all data: Tenant/FullCache frames; many tenant strings | Admission; originator equals TLS identity; epoch map capped with safe global fallback | `node.rs` InvalidateRequest arm; `cache.rs::try_invalidate_tenant` |
| S15 | Forged or replayed cluster invalidation from a stale era: Old-epoch frame accepted | Anti-entropy epoch checked; ack correlation by request id | `node.rs` AntiEntropy arm |
| S16 | Revoked or expired node stays connected: Certificate revoked or expired after handshake | Per-frame registry re-check; revocation broadcast and session cancel (in code) | `session.rs`, `pool.rs`, `security/cluster.rs::update` |
| S17 | Cluster listener exhausted: Stalled handshakes; many authenticated sessions; accept error | 64 handshake permits, 1 s timeout; frame cap 1 MiB; fail-closed drop | `node.rs::start_tcp_service`; `frame.rs::read_frame` |
| S18 | Origin overloaded by misses: Cache-busting keys; `x-cache-bypass`; flash crowd | One fetch per key (coalescing); optional concurrency cap with stale-or-503 | `server.rs::acquire_origin_permit`; `intelligence/executor.rs` |
| S19 | SSRF to internal address: Origin name resolves to private or metadata IP | Port and scheme policy; every resolved IP filtered; connection pinned | `upstream.rs::UpstreamPolicy::validate_target`, `resolve_and_pin_origin` |
| S20 | Malicious origin response: Conflicting lengths, bad chunking, huge body, slow body | Strict parser; 2 MiB/64 KiB caps; 5 s timeouts; truncated not cached | `server.rs::parse_http_response`, `decode_chunked_body` |
| S21 | Stale body survives an invalidation (race): Invalidation lands between fence check and epoch re-read | Generation and epoch fence | `cache.rs::finalize_fetch` |
| S22 | Invalidation delayed or lost to a down peer: Peer unreachable, queue full | Per-peer queues, repair ledger, anti-entropy pull, recovery bypass | `router.rs`; `anti_entropy.rs` |
| S23 | Invalidation lost on issuer crash: Issuer killed before repair | none (ledger and ring in memory) | `main.rs` shutdown path |
| S24 | Memory exhaustion: Many or large entries; many waiters; tenant flood | Byte budget, oversize rejection, waiter cap, tenant-map cap, ledger caps | `cache.rs`; `router.rs` |
| S25 | Hot or poisoned eviction state: S3-FIFO tombstones grow unbounded | SIEVE is the default; S3-FIFO only selectable through the executor or legacy PIE, which have no live caller (grep) | `core/eviction.rs` |
| S26 | Request amplification of invalidation work: Rejected DELETEs force ledger scans; pool saturation by one tenant | Ledger bounds; bounded pool | `router.rs::RepairLedger` |
| S27 | Secret or internal detail leaks: Config dump, error text, diagnostics | Redaction; generic 401 bodies | `config.rs`; `server.rs` |
| S28 | Weak transport on ingress: TLS 1.2 or plaintext allowed | Network bind needs TLS unless explicitly overridden (auth still required) | `security/tls.rs`; `mod.rs` |
| S29 | Process panic or poisoned lock: Panic while holding a shared mutex | Core cache recovers from poison and counts it | `cache.rs` lock helpers |
| S30 | Vulnerable dependency: Known advisory in a crate | Lockfile pins versions | `Cargo.lock` |
| S31 | Wall-clock jump extends freshness: Clock stepped backwards | none (TTL and stale cap use `SystemTime`) | `intel/diagnostics.rs::DiagnosticBuffer::now_ms` |
| S32 | Misleading statistics hide an incident: Hit rate 1.0 with no data; mixed percentile rules | Schema-2 payload labels values measured/derived/unavailable | `server.rs::build_schema_v2_payload`; `telemetry.rs` |

**Table 6.11b. The same rows: tests and what remains unresolved.**

| # | Tests that validate it | What remains unresolved (finding, disposition) |
|---|---|---|
| S01 | `m7_sec_fixes_tests::sec01_*`, `m7_http_rbac_matrix_in_secure_mode`, `security_phase1_tests::test_phase1_at03_and_at07_*` | Without a secret, loopback is fully anonymous by design; G4B-04 (no Host/Origin guard): OPEN |
| S02 | `m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`, `test_phase1_jwt_unknown_kid_rejected_without_fallback` | Shared-secret design (anyone with the secret mints Admin): architecture decision, no ID |
| S03 | none for `nbf`/lifetime/revocation (no test mentions `nbf`) | G4B-13 (SEC-M7-11): OPEN |
| S04 | `sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09` | G4B-14 (SEC-M7-09): OPEN |
| S05 | `sec01_role_without_cache_read_is_403_and_causes_no_side_effects`, `test_phase1_at04_*`, `low_zero_side_effects_include_telemetry_and_diagnostics` | none cited |
| S06 | role matrix only | G4B-03: OPEN*; G4B-19 log injection: OPEN |
| S07 | `low_events_stream_is_observer_gated` (auth only) | G4B-02: OPEN (no cap, no expiry close); Lab API has both |
| S08 | `sec03_*` | G4B-04: OPEN |
| S09 | `test_m1_t26_ingress_request_body_limit` (size only) | G4B-01: OPEN*; High; runtime effect UNVERIFIED |
| S10 | `h1_*`, `l1_*`, `m1_client_x_headers_*` (round 2/3), `parser_robustness_tests` | G4B-24: DELETE is exact-key (aliases like `/x?`, `//x`): OPEN |
| S11 | `m7_secure_mode_ignores_spoofed_tenant_header`, `m1_tenantless_and_reserved_tenant_tokens_are_403_*`, `test_phase1_at05_and_at06_*` | G4A-19: OPEN (validator accepts the literal); no per-tenant quota (G4A-23 Info; G4B-11) |
| S12 | `m1_client_x_headers_*`, `m1_set_cookie_*`, `e2e_m2_*` | G4A-09 (secure default: personalised answers not cached unless marked shareable): OPEN; G4A-07 (cached 301 loses `Location`): OPEN |
| S13 | `m2_cluster_get_request_keys_*`, `known_gaps::pin_kg6_*` (pins the stub) | G4B-30 / DR-029: OPEN, owner decision pending; resource use and eviction by a member |
| S14 | `test_p4a_sec01_*`, `test_l3_invalidate_request_originator_must_match_tls_identity` | G4B-05 / DR-027: OPEN*; G4A-12: OPEN; needs protected-core change |
| S15 | `test_p4c_sec02_mismatched_epoch_rejected`, `test_p4b_mismatched_request_id_ack_discarded` | G4B-06: OPEN* (InvalidateRequest epoch not checked; ack epoch check vacuous) |
| S16 | `test_p3_at06_*`, `test_p3c_sec03_*`, `test_p3c_per_request_revocation_on_reused_connection` | G4B-07: OPEN* (no production trigger); G4B-08: OPEN* (no connection age or NotAfter check) |
| S17 | `test_p3_at10_tls_handshake_timeout`, `test_p3b_at11_oversized_payload_rejected_before_alloc` | G4B-10, G4B-36, G4B-22: OPEN |
| S18 | `origin_cap_tests::*`, `coalesce_tradeoffs_tests` | G4B-11: OPEN (cap off by default; no per-tenant limit) |
| S19 | `m7_ssrf_filter_*`, `test_p2_ssrf_*` | G4B-15 (SEC-M7-10): OPEN (range gaps) |
| S20 | `parser_robustness_tests` (with proptest), `security_phase2_upstream_tests` | G4B-35: odd 1xx statuses reach the client (PLAUSIBLE, Info): OPEN |
| S21 | `test_p4a_inflight_fetch_fencing_prevents_stale_writes` (does not hit the gap) | G4A-01: UNDER-INVESTIGATION (High); sandbox fix not applied; G4A-08, G4A-17: OPEN |
| S22 | `cluster_followup_tests`, `distributed_consistency_tests`, `cluster_prewarm_repair_tests` | G4B-09, G4B-12 (heartbeat does not drive state): OPEN |
| S23 | none | G4B-09: OPEN; M3 durable log PROTOTYPE ONLY |
| S24 | `byte_accounting_tests`, `memory_bound_soak_tests`, `test_p4a_sec01_over_10k_tenants_*` | G4A-23 (accounting omits metadata; pinned keys): OPEN; G4A-13 byte drift in a panic path: OPEN |
| S25 | `eviction_contract_tests` | G4A-02: OPEN (High, conditional) |
| S26 | `test_ledger_overflow_*`, `test_m6_one_tenants_overflow_does_not_flush_other_tenants_on_peer` | G4B-37, G4B-11: OPEN |
| S27 | `secrets_never_appear_in_report_debug_errors_or_warnings`, `low_401_body_is_generic_*` | G4B-21, G4B-16: OPEN; log/SSE channels not scanned for secrets |
| S28 | `test_phase1_at01_*`, `test_phase1_at09_startup_security_fail_closed_rules` | G4B-20: OPEN (not 1.3-only); G4B-17: OPEN |
| S29 | `lock_poison_recovered` counter paths | G4A-21, G4B-28: OPEN |
| S30 | none (no scan run) | G4C-DEP: OPEN; G4B-34 (broad features): OPEN |
| S31 | none | G4A-10: OPEN (PLAUSIBLE) |
| S32 | `telemetry_honesty_tests`, `telemetry_truth_tests` | G4A-11, G4A-24, F24: OPEN |

Rows S06, S09, S14, S15, S16 together are the owner-named set (ODR-4). The register lists no Critical finding and four High: G4A-01, G4A-02, G4B-01, and F01 (Lab verdicts, in sandbox remediation).

## 6.12 Evaluating performance correctly

**Simple version.** A performance number is only meaningful with its context: what ran, on what, under what load, how many times, and compared with what. "10 million operations per second" with no context is a slogan.

### 6.12.1 The vocabulary

- *Latency*: time for one request, start to finish; always a distribution. *Throughput*: completed requests per second.
- *Percentile* (P50, P95, P99): the value below which that share of requests fall. P99 = the slowest 1 percent starts here. Tail percentiles need many samples: with 100 samples P99 is one observation.
- *Hit rate* (hit ratio): hits divided by lookups. *Origin request reduction*: how much fewer requests reach the origin than without the cache. These differ: coalescing and stale serving change origin load without changing the hit ratio the same way.
- *Open-loop versus closed-loop load*: a closed-loop client sends the next request only after the previous finishes, so when the system slows, the client slows too and hides the queue (this is *coordinated omission*). An open-loop client sends on a schedule regardless.
- *Contention*: threads waiting for the same lock; throughput that stops scaling with threads and a growing tail are its signs. *Warm-up*, *repetition*, *confidence interval (CI)*: discard start-up effects, repeat in separate processes, report a range, not a point.

### 6.12.2 How percentiles are computed in each place (G4A-24, F24)

The repository computes percentiles at least five ways, and they give slightly different answers on the same data. Comparing one place's P99 with another's is invalid unless you know both rules. This chapter owns the detail.

| Place | Rule | Source |
|---|---|---|
| Live gateway `latency_ms` (schema-2 metrics and `/api/events`) | `TelemetryCollector::calculate_percentiles_us`: take the ring `raw_buffer_us` (capacity `TelemetryCollector::new(10000)` in `main.rs`), which holds the last up-to-10,000 observations of kind GET *and* DELETE; keep the GET ones; sort; index `floor(n * q)` clamped to `n-1` (the median uses `(n*0.5) as usize % n`). An empty window returns `(0,0,0,0)`, not "unavailable" | `intel/telemetry.rs::TelemetryCollector::calculate_percentiles_us`, called from `server.rs` |
| Log-linear histogram (2,688 buckets) | cumulative count, target rank `ceil(total * q)`, resolved to a bucket; documented error at most 0.39 percent. **No production reader:** `TelemetryCollector::record` and `record_us` feed it, but `calculate_percentiles_histogram_*` and `histogram()` have no caller outside tests (grep of `src/`; `server.rs` calls only the ring version), so its percentiles are never served | `telemetry.rs::AtomicLogLinearHistogram::calculate_percentiles_us` |
| Scenario B and Lab loadgen | nearest-rank, `ceil(q * n)`, `None` for empty | `lab_live/loadgen.rs::percentile` |
| T1 simulator | nearest-rank `ceil` | `lab/t1_sim.rs::percentile` (frozen) |
| AIMD controller window | nearest-rank on a window of observations | `intelligence/executor.rs::record_observation` |
| Intelligence signals | floor index | `intelligence/signals.rs` (F23/F24) |
| Micro-benchmarks | HdrHistogram, 3 significant digits | `benches/b2_concurrency.rs`, `b8e_gateway.rs` |

So for 100 samples, `ceil(0.99 * 100)` is rank 99, the 99th smallest value, but `floor(0.99 * 100)` used as a zero-based index is index 99, the 100th (the maximum). That one-sample difference is the whole of G4A-24. For small windows the difference is the entire tail: the AIMD "p99" of a 30-sample window is the window maximum, as the Scenario B notice points out. A separate 16-bucket `OriginLatencyHistogram` (`intel/origin_metrics.rs`) serves origin latency percentiles in the metrics snapshot; it is a different structure again.

**Hit rate caveats.** `hit_ratio_cumulative` in schema 2 is `hits / (hits + misses)` since process start, labelled "derived", and reports "unavailable" with no data (`build_schema_v2_payload`). A cumulative ratio cannot show a recent collapse; use `hit_ratio_10s` or `hit_ratio_1s` for that. The older ring-based `calculate_hit_rate()` returns **1.0 when there is no data**, and different parts of the code count coalesced waiters, stale serves and background revalidations differently (G4A-11, OPEN). Always ask: what counts as a hit?

**Throughput caveat.** `throughput_req_s_*` are windowed samplers; older documents describe throughput as a cumulative counter over uptime (a known gap from the Phase 1 tests). Check which field you are quoting.

### 6.12.3 Benchmark fairness and baselines

A fair comparison changes one thing: same release build, same host and load state, warm-up discarded, same workload and seed, enough repetitions in separate processes, arms run in the same session or order-randomised, a plausible baseline (not a straw man), and a stated limit on what the result generalises to. The repository has **no head-to-head comparison against Redis, Memcached, Varnish or any other cache**, and no end-to-end "origin without a cache" baseline on the real gateway except Scenario B's BASE arm (no cap). Internal baselines exist: static cap versus AIMD cap versus none (Scenario B); eviction policies against reference policies in `benches/support/refpolicies.rs` and in the simulator. We therefore cannot say PhantomCache is faster or better than any other product, and this handbook never does.

::: judge
**JUDGE MAY ASK**

Question: How did you measure performance, and why should I trust the numbers?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We do not publish a performance figure as a claim. The repository does contain benchmark results with a stated host, profile and limits, and every one is listed in section 6.13. They are of two kinds, both on one Windows laptop and loopback only. First, micro-benchmarks of the cache hit path and concurrency, repeated ten times in separate processes with bootstrap confidence intervals, with large spread between sessions. Their build profile is not established: the runner scripts look for release binaries, but the benchmark directory's own README says no release-profile result is established. Second, a 30-run loopback experiment on origin protection, which used release builds. I would tell you what they do not show: no other cache was measured, no multi-host or network run exists, the benchmarks are closed-loop, and the numbers were taken on an earlier version of the code and not re-run on the current tree. None of them establishes a release-profile or comparative claim, and this book makes no superiority claim. So treat them as local, dated evidence, not as a product claim.

**If the judge asks further:**

Be ready to explain the spread: the same benchmark repeated in four sessions the same day gave gateway throughput results whose confidence intervals did not overlap (the sessions are listed in 6.13), so the within-session interval understates machine variation. That is why we do not quote a single figure.

**Evidence to know:** `bench-results/r0_summary.json`, `bench-results/manifests/*.json`, `benches/*.rs`; `docs/research/claims_ledger.md`; this chapter 6.13. Status: MEASURED (local, dated), not INDEPENDENTLY VERIFIED.
:::

::: judge
**JUDGE MAY ASK**

Question: Is your benchmark fair? Could you be fooling yourselves?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

For the micro-benchmarks, the fairness question is about what we compare: they measure one implementation against itself (hit path, component costs, with and without telemetry), not against a competitor, so the main risks are noise and coordinated omission. We controlled noise with repetitions and recorded the limits, including a hybrid-core laptop and a Windows scheduler. For the origin-cap experiment we did have an independent fairness review, and it found real problems that we published in a notice: the adaptive arm collapsed to a fixed cap near the minimum, the arms differ by more than the cap because the capped arms run in canary mode, success-only latency must be reported separately, the cap values were chosen after one pilot run, and the origin has no load-dependent slowdown. So I say: informative about mechanics, not evidence that the adaptive controller works.

**If the judge asks further:**

Ask about coordinated omission. The micro-benchmark and gateway load are closed-loop, and the project's own limitation text says so. Scenario B's load generator is open-loop: latency is measured from the scheduled send time.

**Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`; `lab-results/scenario_b/20261030T003932Z/summary.json`; `bench-results/r0_summary.json` `limitations_verbatim`. Status: MEASURED_LOOPBACK, with recorded qualifications.
:::

## 6.13 Every real measurement in the repository

We searched `benches/`, `bench-results/`, `lab-results/`, `docs/testing/` and the claims ledger. The table lists what exists, with the conditions required by the project's evidence rules. Anything not in the table is not a measurement we can quote. Keep the tiers apart: T1 is **simulated**, loopback is **MEASURED_LOOPBACK**, and nothing here is multi-host or WAN.

**M1. Micro-benchmarks R0 (2026-10-28) and passes R-A, R-B, R-C** (`bench-results/r0_summary.json`, `manifests/r0_baseline_manifest.json`, `ra_`, `rb_`, `rc_manifest.json`).

- *Measured:* B1 hit path and component costs; B2 concurrency throughput and latency; B6 telemetry; B8e gateway loopback GET.
- *Profile and host:* build profile **UNVERIFIED**. The runner scripts (`bench-results/tools/*_runner.py`) look for binaries in `target/release/deps`, but the manifests carry no profile field and `bench-results/README.md` states that no release-profile benchmark result is established by that directory. Windows 11 x86_64, hybrid-core laptop, Balanced power plan, affinity mask for P-cores 0 to 15, rustc 1.98.0; tree at `456bb25` with a dirty flag, **not the current tree**.
- *Workload and seed:* B1: 4 KiB body, 64-byte key, tenants 1 and 1,000. B2: 100,000-key universe, Zipf alpha 0.8 from a fixed xorshift state, 2 s warm-up, 5 s windows, threads 1, 8, 16. B8e: cached key, 16 concurrent clients, in-process mock origin, 2 s warm-up. Bootstrap CI seed 42.
- *Repetitions:* 10 process-level repetitions per benchmark; the CI is a bootstrap 95 percent interval of the medians.
- *Supports (re-read from `r0_summary.json` for this chapter):* hit 110.4 ns (CI 107.6 to 112.8) with 1 tenant and 118.5 ns with 1,000; epoch lookup 9.7 ns and 18.2 ns; fill about 1,037 ns. B2: 1 thread 2.5 M ops/s; 8 threads 10.2 M (1 tenant) but 5.0 M (1,000 tenants); 16 threads 8.1 M and 4.4 M. B8e: about 75.9 k req/s telemetry off and 74.2 k on (overlapping CIs), P50 about 201 to 206 microseconds. Evidence of order of magnitude, and that more tenants cost throughput under contention.
- *Does not support:* per-operation latency uses a timer around each call, so P50 values of 200 ns and 400 ns sit at a few timer ticks (inference: Windows timer resolution). B8e telemetry-off throughput differs between passes (75.9 k R0, 77.6 k R-A, 60.9 k R-B, 79.2 k R-C), and the R0 and R-B intervals do not overlap, so session-to-session spread exceeds within-session CIs. Closed-loop. No comparison with any other cache. Hit-path numbers are library calls, not HTTP. Not re-run on the current tree. Do not quote as release-profile product performance.

**M2. B3 coalescing, B4 eviction pressure, B7 memory and allocation** (same runner, `noncore_results`). Coalescing with N waiters and a 10 ms simulated origin; eviction latency for SIEVE, LRU and S3-FIFO as implemented (Zipf 0.8 and churn); allocation counts. Same profile and host as M1, 10 repetitions. Supports: B7 allocation counts agreed exactly across repetitions (14.0088 allocations per fill in every repetition: a determinism check); B3 asserts one origin call. Does not support: the values were not tabulated here, so read the raw arrays before quoting; B4 measures eviction implementations that the G4A review rates as scans or partly incorrect (S3-FIFO: G4A-02).

**M3. `bench-results/cluster/characterization_report.json`.** Phase 3 characterisation of distributed gaps (cited by ADR-003). Profile, host, workload and repetitions: not checked. A historical record of the gaps only. **UNVERIFIED**: not opened in detail for this chapter.

**M4. Claims ledger C6 amendment** (from `cluster_propagation_latency_tests`). Invalidation propagation latency between gateways: **debug** profile; loopback; Windows 11, 24 logical CPUs; 3 processes with mTLS; per-trial distinct key, warm then DELETE; 4 runs of 200 trials by 2 peers plus one run of 1,000. Supports: warm P50 about 0.3 ms, P99 about 10 to 20 ms; cold first use 35 to 117 ms; refutes "P(stale)=0 at 50 ms". Does not support: release profile or multi-host claims; raw CSVs are not retained in the repo; no loss was injected in those trials.

**M5. Scenario B** (`lab-results/scenario_b/20261030T003932Z/`: `summary.json`, `manifest.json`, `runs/`). Origin slowdown, three arms: BASE, STATIC-CAP 24, AIMD-CAP 12 to 32.

- *Profile and host:* release driver and gateway; one Windows host, 24 logical CPUs; tier MEASURED_LOOPBACK, label VALID (the notice says "release" was judged by a path heuristic).
- *Workload and seed:* Poisson arrivals, 300 req/s, Zipf 0.9 over 5,000 keys, 31 s runs (8 s warm-up, 5 s healthy, 10 s degraded at 6x service time, 8 s recovery), scripted origin base 20 ms. Seeds: base 20261030, run r uses base+r.
- *Repetitions:* 10 per arm, paired by seed.
- *Supports (re-read from `summary.json` for this chapter):* peak origin concurrency (window metric) fell 38.3 percent (95 percent BCa CI 36.7 to 40.2) with static cap 24 and 64.0 percent (CI 62.5 to 65.3) for the AIMD arm, at client success costs of 4.9 and 17.1 percentage points; both arms `TRADE_OFF_FAILURE` by the mechanical rule; cap enforcement correct (static arm peak exactly 24 in all 10 runs).
- *Does not support:* the AIMD arm sat near its floor of 12, so it is "static 12 versus static 24"; all-request percentiles include fast 503s; the scripted origin has no load-dependent latency; stale serving was not exercised; cap values were chosen after a pilot, not preregistered; canary mode only on capped arms; loopback; not production (see the notice).

**M6. `lab-results/m6_sealed_*` and dry runs.** T1 **simulated** evaluation of adaptive control (30 sealed seeds), detailed in Chapter 7. The verdict is H1 NOT_SUPPORTED (0 of 8 scenarios on peak origin concurrency, 2 of 8 on P99; `docs/research/m6_sealed_results.md`). It says nothing about real latency, throughput or hardware.

**M7. `docs/testing/*.md`** (V2, V3, V4 audits, validation reports). Historical audits of the simulation-era demo, each with a 2026-10-30 notice. None of their numbers is a gateway measurement.

**M8. Test suite baseline.** 242 tests in 19 targets passing (CLAUDE.md, 2026-10-28, debug, reproduced once); the latest recorded run is 851 passed in 75 targets (Chapter 8; `docs/release/verification_register.md` V-01); the counts are different measures, explained in Chapter 8, section 8.2.2. A pass is not correctness.

**How to quote these honestly.** Give the file, the profile, the host, the workload and seed, the repetitions and the limits, together. For example: "On one hybrid-core Windows 11 laptop (profile not established), at commit `456bb25` plus uncommitted changes, ten repetitions, the in-process hit path had a median of about 110 ns (95 percent bootstrap CI 108 to 113); this is a library call, not an HTTP request, and was not re-run on the current tree." Never: "PhantomCache serves 10 million requests per second."

::: judge
**JUDGE MAY ASK**

Question: Your results are on loopback with a simulator. How do you know this reflects reality, and what is the difference between simulation and a real run?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Simulation means we model the system and the workload in software on a virtual clock, which is repeatable and fast and good for exploring many scenarios. A real run means the actual gateway process handles real network sockets, even if it is on the same laptop. We keep them labelled apart: T1 is simulated, loopback is measured on real binaries. Our simulated study found that the adaptive controller gave no benefit over a fixed cap, and our loopback study found a cap lowers peak origin load at a client-success cost. Neither says anything about a real network, several machines or production traffic. Loopback removes network round trips and shares one CPU between the load generator, the gateway and the origin. To close the gap we would need a multi-host run with a realistic origin and open-loop load.

**If the judge asks further:**

Ask whether T1 predicts T2. No agreement study was run (claims ledger C7: UNVERIFIED), and the agreement code is a recorded limitation (F05). We make no "simulation predicts reality" claim.

**Evidence to know:** `docs/glossary.md` section 5 tiers; `lab-results/README.md`; claims ledger C1, C7; Chapter 7. Status: MEASURED_LOOPBACK and SIMULATED kept separate.
:::

## 6.14 Reliability, health and bottlenecks

**Health and readiness** (6.3). Readiness reflects only local state, so a load balancer would keep sending traffic to a node whose origin is down; that node would answer misses with 502/504 and hits normally.

**Failure handling in the request path.** Origin timeouts give 504; transport errors 502; a saturated cap 503 (or stale if eligible); stale-if-error serves a stale entry within the hard cap `PHANTOM_MAX_STALE_MS` (default 24 h; `stale_cap_violations` must be 0) but is applied to every non-200/304 status rather than only 5xx (G4A-06, OPEN). A fetch task that is cancelled releases its permit and its single-flight registration through drop guards (`origin_cap_tests`); G4A-13 notes one byte-accounting drift in a panic path.

**Node failure.** A node is marked Suspect when a primary times out and recovers through reconciliation; heartbeats refresh pool idle timers and prune dead connections but do not change node state (G4B-12, OPEN), and `NODE_FAILURE` from `/api/control` is a logical flag, not a failure detector. There is no automatic membership change: a dead member stays in the registry and each DELETE retries it through the ledger.

**Where the time goes (bottlenecks).** What the evidence supports:

1. *The hit path is cheap and the contended path is not.* The R0 numbers show a hit costing about 110 ns single-threaded, but 8 to 16 threads on 1,000 tenants reach roughly half the throughput of 1 tenant (5.0 M versus 10.2 M ops/s at 8 threads) and a P99 several times higher (6.8 microseconds versus 2.3). That is consistent with contention on shard mutexes and shared structures; the cause was not isolated by a profiler (**hypothesis**).
2. *Per-request body clone and per-hit telemetry.* G4A-26 notes a body clone per hit; B8e shows no distinguishable telemetry overhead at gateway level in those runs.
3. *Epoch-map clone per tenant invalidation* is O(n) up to 10,000 entries (code reading; no benchmark).
4. *Sweeper scans.* `PHANTOM_SWEEPER_BATCH_SIZE` bounds candidates, not keys inspected, and a tick can scan a full shard under its mutex (G4A-03, OPEN).
5. *Idle CPU.* The M7 review recorded 14.78 CPU-seconds in 10 s wall on an idle release process (PERF-M7-01) and attributed it by code reading to the sweeper's shutdown channel; change D-2 keeps the sender alive. Whether the idle burn is gone was not re-measured (**UNVERIFIED**).
6. *The origin.* Under a miss storm the origin is the bottleneck by design; that is what the cap is for.

::: judge
**JUDGE MAY ASK**

Question: What would you improve first, and what are your real bottlenecks?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Security first: authenticate before reading the body and add timeouts and connection limits to the main API, because that is the one High finding that is unauthenticated and remote; then give the owner a decision on cluster member authorisation so a member cannot flush other tenants; then make owed invalidations durable so an issuer crash does not lose them. For performance, the evidence shows the hit path is cheap and contention appears with many threads and many tenants; I would profile that on a quiet machine with a release build before changing anything. I would also run a multi-host experiment with an open-loop load generator, because everything we measured is on one laptop.

**If the judge asks further:**

Ask what you will not do. We will not add features before the High findings are dispositioned, and we will not quote throughput numbers without the full conditions.

**Evidence to know:** G4B-01, G4B-05, G4B-09; `bench-results/r0_summary.json` B2 results; `docs/release/lead_working_state.md` queued engineering. Status: plan, not implementation.
:::

::: judge
**JUDGE MAY ASK**

Question: What is still unresolved? Give me the honest list.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There are 99 findings from three AI reviews; 75 are open, 20 are recorded as limitations in frozen research code, 2 are in sandbox remediation, 1 is under investigation, and 1 was refuted. There is no Critical finding. The High ones are: an unauthenticated slow-body weakness on the HTTP ingress; a suspected timing hole in the stale-write fence; a conditional S3-FIFO memory growth problem, where we found no live caller that selects S3-FIFO; and a Lab verdict labelling bug whose fix is in a sandbox. The Medium security items the owner flagged as must-not-dismiss are: the control endpoint touching the live router, member authorisation for tenant-wide invalidations, the missing epoch check on invalidation frames, static membership with no live revocation, and connections that never age out. We have not run a dependency scan, a penetration test or fuzzing. None of the reviews is independent human review.

**If the judge asks further:**

Say plainly that the register's counts are the DISP agent's, built from the reviewers' reports, and that "CONFIRMED" there means confirmed by reading code. We re-read the code for every item we cite and found them still present; we did not execute exploits.

**Evidence to know:** `docs/release/g4/cycle1_findings_disposition_register.md` section 3 (counts), `docs/release/owner_decisions_register.md` ODR-1 to ODR-4, `docs/release/known_limitations_frozen_code.md`. Status: all findings OPEN unless the owner dispositions them.
:::

## Key ideas to remember

::: concept
- Authentication runs on every sensitive route when a secret is set; the tenant comes only from the token. Gaps: no revocation, no `nbf`, and (G4B-01) auth runs after the body is read.
- Peer trust is certificate-based and strong at the door (TLS 1.3, one SAN, registry, fingerprint) and weak inside it: any admitted member can flush any tenant (DR-027).
- Memory is bounded and tested; connections, SSE streams and origin concurrency are not bounded by default.
- A DELETE returns 204 after the local purge and the primary's ack; other peers are repaired in the background from an in-memory ledger that an issuer crash loses.
- Percentile rules differ across the code base; hit-rate definitions differ; always state the rule.
- Every measurement in the repo is local, dated and loopback; none supports a comparison with another product.
:::

## What you should now understand

- The five trust zones and the checks at each boundary; why "read body, then authenticate" is a vulnerability and how the Lab API avoids it.
- How the JWT is checked and which standard checks are missing; the layers of peer admission and what a member can still do.
- Why `CacheKey::validate` and the header whitelist prevent injection and poisoning, and what DR-029 is.
- Exactly what a 204 on DELETE means, when an invalidation is lost, and the G4A-01 race in one paragraph.
- Reading a benchmark: profile, host, workload, seed, repetitions, CI, limits, and what each measurement in the repo supports.

## Knowledge check

1. In which order do the GET handler's body extraction, authentication and key validation happen, and which finding records the problem?
2. A token has `roles: ["cache:read"]` and `tenant_id: "__default__"`. What does GET return and why?
3. Name three JWT lifecycle controls that are absent from `jwt.rs`.
4. A cluster member sends a `Tenant`-scope invalidation naming 10,001 distinct tenant strings. What does the cache do, and which two findings describe it?
5. What exactly does an HTTP 204 from `DELETE` guarantee in a cluster?
6. Why can a peer's fabricated `val_node_N` entry not be served to an HTTP client? What harm remains?
7. Give the nearest-rank (`ceil`) and floor-index results for P99 of 100 sorted samples, and say which code uses which.
8. In `r0_summary.json`, B8e telemetry-on and telemetry-off throughput have overlapping intervals. What may you conclude, and what may you not?
9. Why is the claim "PhantomCache uses a hit path of about 110 ns" incomplete as stated?
10. Which fix would you not apply without a failing-first test and an independent review, and why?

::: answers
1. The extractors (`_body: Bytes`, `Json(payload)`) read the body first; then the handler authenticates (`authenticate_request`), then requires a tenant (`require_tenant_claim`), then validates the key (`CacheKey::validate`). Finding G4B-01 (OPEN, owner-named, High).
2. 403 with `X-Auth-Error: ReservedTenant`: `require_tenant_claim` refuses the reserved literal so the token cannot address the tenant-less key space (`m1_tenantless_and_reserved_tenant_tokens_are_403_*`).
3. `nbf` validation, maximum lifetime, `jti`/revocation (also: no rotation key configured in `main.rs`; `iat` unused). G4B-13.
4. `try_invalidate_tenant` hits `MAX_TENANT_EPOCHS` (10,000), bumps the global epoch and clears the map, which fences everything (safe against resurrection, tested by `test_p4a_sec01_*`) but flushes every tenant's entries. Each call also clones the map. G4B-05/DR-027 and G4A-12.
5. Local purge on the issuer plus an acknowledgement from the rendezvous primary or acting primary. Other members are updated asynchronously and are guaranteed only while the issuer process stays alive (G4B-09).
6. The handler rejects keys starting with `/` and every HTTP key begins with `/` after `norm_path`; the fabricated key lives under `__default__` without a leading slash. Remaining harm: fabricated data in a production cache, byte-budget filling and eviction of real entries, no per-peer quota.
7. `ceil(0.99*100)=99`: the 99th smallest value (index 98). `floor(0.99*100)=99` as an index: the 100th value, the maximum. Ceil: `loadgen::percentile`, `t1_sim::percentile`, the histogram, the AIMD window. Floor index: the live ring `TelemetryCollector::calculate_percentiles_us` and `signals.rs`.
8. That these runs show no distinguishable telemetry cost at 16 concurrent clients on that host. You may not conclude zero cost, or the same on other hardware; the CIs were within one session and sessions differed by up to about 30 percent.
9. It omits profile, host, workload, repetitions, CI, and that it is an in-process library call on an older tree; it is not an HTTP latency or a product claim.
10. Anything in `finalize_fetch` (G4A-01), epochs or invalidation (G4B-05/06): CLAUDE.md section 5 requires a test that fails without the change and an independent review, because a mistake can serve stale data.
:::

## Codebase exploration tasks

1. **Trace an authenticated DELETE.** Start at `src/api/server.rs::delete_cache_handler`. Follow `authenticate_request`, `require_tenant_claim`, `CacheKey::validate`, `cache.delete`, then `RendezvousRouter::invalidate_key_across_cluster` in `src/dist/router.rs`. Find every place a status code is chosen. You should learn what 204, 503 `PrimaryRateLimited` and 504 mean, and where the repair ledger is written.
2. **Read the peer admission path.** Start at `src/dist/node.rs::start_tcp_service`. Find the semaphore, the SAN count check, `validate_peer`, and the `originator_id` comparison in the `InvalidateRequest` arm. Then search for any use of `payload.membership_epoch` in that arm. You should learn what is checked at admission versus per frame, and see G4B-06 for yourself.
3. **Compare two body handlers.** Read `src/api/lab_api.rs::runs_start` beside `src/api/server.rs::get_cache_handler`. List the differences in order of operations, timeouts and caps. You should learn what a fix for G4B-01 looks like and why the Lab API already has it.

## Challenge questions

1. *Design a fix for G4B-01 without breaking existing tests.* Which axum or tower facilities do you use, where does authentication move to, what happens to the 413 test, and what new tests prove it? **Model answer.** Change the cache handlers to take `Request<Body>`, authenticate from the headers, then read the body with `tokio::time::timeout(..., to_bytes(body, limit))` as `runs_start` does (or drop the body entirely for GET and DELETE, since they need none); add a tower body-read timeout layer and a connection concurrency limit at the listener; keep `DefaultBodyLimit` so `test_m1_t26_ingress_request_body_limit` still returns 413 for authenticated callers (and note the oracle: unauthenticated oversized requests should now get 401 first). New tests: N stalled sockets with a long Content-Length must not block a legitimate GET and must be closed within a bound; unauthenticated `POST /api/control` with invalid JSON returns 401. This is an application-code change and needs the owner's approval.
2. *Make an issuer crash safe.* Specify a failure model, what to persist, when to flush, and a test that would fail on today's code. **Model answer.** Model: issuer process crash or SIGTERM, restart on the same disk; OS crash only with synchronous flush. Persist the owed set (peer, scope, canonical key, sequence), appended before the 204 is returned for fan-out targets and removed on ack; on restart reload into the `RepairLedger` and send; drain on graceful shutdown with a deadline. This is the M3 design (sandbox, not integrated). Test: a multi-process harness that blackholes peer C, sends DELETE through A, kills A with SIGKILL, restarts A, and asserts C stops serving the key within a bound; today it would fail because nothing remembers C was owed.
