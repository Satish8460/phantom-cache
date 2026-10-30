# P3: Security, multi-tenancy and resource management (three-pillar product direction)

Workstream P3 of 4 (agent 3). Date: 2026-10-30. Status: RESEARCH AND DESIGN ONLY. Nothing here is implemented, approved, or run. No source, Cargo, CLAUDE.md, git state, test, browser or network was touched; the only file written is this one. Read at HEAD `456bb25`, branch `feat/publish-phantomcache-core`, with a large uncommitted tree. No secret or salt value was read or printed.

This document is not a security certification and does not claim the product is secure. Section 8 says what demonstrations can and cannot show.

## 0. How to read this document

Labels used on every claim:

- **FACT** - verified by reading the cited `path:line` in this session. "Reading only" means I did not run it.
- **FACT-PRIOR** - stated in a repo document or by an earlier agent (V3, V4, A2, M7 review, user docs); I did not re-open the code. Treat as unverified by me.
- **INFERENCE** - my reasoning from facts. Could be wrong; the check is named.
- **RECOMMENDATION** - what I propose. Not a decision.
- **UNRESOLVED** - an open question; no answer is assumed.

Status classes for capabilities: IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, BLOCKED, RETRACTED. A capability can carry several (for example IMPLEMENTED + TESTED + UNVERIFIED independence). Where no code exists, the status is DESIGNED BUT NOT IMPLEMENTED only if this document contains the design; otherwise I say "no code, no design".

Environment of all existing evidence (FACT-PRIOR, `docs/STATUS.md` section 3, M7 review): Windows 11, x86-64, loopback, single host, debug profile, tests written mostly by the same project. The last recorded run was 851 passed, 0 failed, 8 ignored (debug, 2026-10-30). I did not re-run anything. Passing tests do not establish correctness (CLAUDE.md section 3).

Files read: V3, V4, the consolidated report and audit (by targeted search, not every line), A2 matrix (targeted), `docs/security/m7_security_review.md`, `docs/user/security.md`, `limitations.md`, `lab_api.md`, `docs/STATUS.md` (top), `authenticity-design-options.md` (first section), `Cargo.toml`, `src/security/jwt.rs` (all), `src/api/server.rs` (lines 938-1081, 1140-1290, 2410-2430, and searches), `src/dist/node.rs` (499-577), `src/core/cache.rs` (200-340, 480-640, 1195-1540, searches), `src/core/eviction.rs` (trait and SIEVE victim selection), `src/lab/manifest.rs` (85-165), `src/config.rs` (18-36), searches of `src/main.rs`. **Not read in depth:** `src/api/lab_api.rs` (I rely on `lab_api.md`), `src/dist/router.rs`, `src/dist/anti_entropy.rs`, `src/security/cluster.rs` bodies, the other `docs/security/*` files, `src/intel/*`. The other three workstream files (`01_`, `02_`, `04_`) did not exist when I finished reading; the "Requests" in section 12 are therefore not reconciled with them.

## 1. Verified current baseline

### 1.1 Capability table

"Independence" means: has someone other than the author checked it? The M7 review marks the SEC-M7-01/02/03 fixes "independent review pending" and then describes rounds 2 and 3 as independent reviews; V4 section 8.4 notes the provenance of that independence is not established. I therefore write UNVERIFIED for independence everywhere below unless stated.

| # | Capability | Status | Location (FACT unless marked) | Tests (names; not re-run) | Limits and gaps |
|---|---|---|---|---|---|
| 1 | Ingress JWT: HS256 only, iss/aud/exp checked, 10 s leeway | IMPLEMENTED, TESTED, independence UNVERIFIED | `src/security/jwt.rs:149-193` (alg check `:153`, leeway `:171`) | `test_phase1_at03_and_at07_*`, `m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`, `test_phase1_jwt_unknown_kid_rejected_without_fallback` (FACT-PRIOR via M7 review) | One shared symmetric secret: whoever holds it can mint any tenant and any role. No `jti`, no `nbf`, no max-lifetime check, `iat` parsed but unused (`jwt.rs:30-42`). `kid` rotation exists in the validator but `main.rs:414` builds it with `JwtValidator::new` only, so rotation is not wired in the binary. |
| 2 | Role checks per route | IMPLEMENTED, TESTED | `authenticate_request` `server.rs:985-1044`; control `:1070-1081`; SSE `:938-945` | `m7_http_rbac_matrix_in_secure_mode`, `test_phase1_at04_*`, `test_phase1_at10_*` (FACT-PRIOR) | Order 401 then 403 role then 403 tenant then 400. Role strings are matched case-insensitively and include aliases (`jwt.rs:57-84`). |
| 3 | Tenant namespace isolation (secure mode) | IMPLEMENTED, TESTED | tenant from verified claim only `server.rs:1209-1219`; claim required and `__default__` refused `:1052-1068`; key charset in `cache.rs` (FACT-PRIOR V3) | `sec01_tenant_boundary_and_anonymous_cannot_read_authenticated_entries` (exists: `m7_sec_fixes_tests.rs:291`), `test_phase1_at05_and_at06_tenant_cryptographic_isolation`, `m7_secure_mode_ignores_spoofed_tenant_header` | Namespace only. In development mode (no JWT secret) the tenant is the client's `x-tenant-id` header (`server.rs:1215`): not isolation. |
| 4 | Key confusion defences | IMPLEMENTED, TESTED | `CacheKey::validate`, canonical string (FACT-PRIOR V3/V4: `cache.rs:38-116`) | `canonical_key_roundtrip_tests` (proptest), `m7_sec_round2_tests::h1_*`, `m7_sec_round3_tests::l1_*` | Origin-side normalisation differences not modelled. Query-order variants are separate keys (fragmentation). |
| 5 | Unkeyed-input poisoning closed by construction | IMPLEMENTED, TESTED | gateway forwards only gateway-controlled headers (`server.rs:2465-2469` comment; `docs/user/security.md` "Origin contract") | `m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant` (exists: `m7_sec_round3_tests.rs:195`) | Holds only under the origin contract: the origin must not vary on anything but `x-tenant-id`. |
| 6 | Upstream SSRF filter and TLS verification | IMPLEMENTED, TESTED | `src/security/upstream.rs` (FACT-PRIOR M7 review 2.3) | `test_p2_ssrf_*`, `test_redteam_*` (e.g. `test_redteam_default_policy_strips_credentials` exists at `security_phase2_upstream_tests.rs:1015`) | SEC-M7-10: some special ranges not blocked. |
| 7 | Cluster mTLS: TLS 1.3, cluster-CA only, strict SAN, fingerprint registry, per-frame re-validation | IMPLEMENTED, TESTED, experimental | `src/security/cluster.rs`; originator must equal TLS identity `node.rs:505-511` | `test_p3_at06_revoked_node_rejection` (exists), `test_l3_invalidate_request_originator_must_match_tls_identity` (exists: `cluster_followup_tests.rs:520`) | Loopback, 3 nodes typical. Membership read once at start. A valid member is fully trusted (see 1.3). |
| 8 | Membership revocation machinery | IMPLEMENTED in library, TESTED in-process, **not reachable in the shipped binary** | `ClusterMembershipRegistry::update` `cluster.rs:405`; session eviction `dist/session.rs` | `test_p3c_sec03_automated_membership_revocation_wiring`, `test_p3c_per_request_revocation_on_reused_connection` (both exist) | A grep of `src/` for `.update(` outside tests finds only hash-context uses, no registry caller (matches A2 row 2c). Revocation today = edit file and restart. |
| 9 | Peer-frame scope authorization | **no code; design in section 6** | accepts `ExactKey`, `Tenant`, `FullCache` from any active member: `node.rs:533-559` | only legitimate-scope tests exist | Identity-only authorization. |
| 10 | Per-tenant byte/entry quota, fair eviction, per-tenant origin share | **no code; design in section 4** | global shard budget and entry cap `cache.rs:200-224`, `:330`; byte budget set from `PHANTOM_MAX_CACHE_BYTES` `main.rs:205`; victim chosen by the shard policy with no tenant input `cache.rs:1219-1236`; `eviction.rs` trait has no tenant concept (`:16-35`, but the key passed in does carry `tenant_id`) | none | Namespace isolation only. One tenant can evict another's entries and take its share of the byte budget and origin permits. |
| 11 | Rate limits (per IP, principal, tenant), auth-failure counters | **no code; design in section 5** | no limiter in `server.rs` (search for `RateLimit|Semaphore|ConcurrencyLimit|TimeoutLayer` found only origin-timeout and 429/503 mappings) | none | SEC-M7-11. Denied reads are not counted. |
| 12 | Request body limit | IMPLEMENTED, TESTED | `server.rs:220` (512 KiB) | `test_m1_t26_ingress_request_body_limit` (exists) | Body is read before auth on GET/DELETE (`_body: Bytes` extractor, `:1141`; documented). No header-read or body-read timeout found in `server.rs` (search for `header_read_timeout` found nothing; `main.rs` and server builder config not checked). No connection cap found. |
| 13 | Origin concurrency cap | IMPLEMENTED, TESTED, experimental | `acquire_origin_permit` used by `fetch_origin_gated` `server.rs:2416-2426` | `cap_never_exceeded_by_concurrent_distinct_misses` (exists: `origin_cap_tests.rs:164`) | Gateway-global. Overload becomes 503 for everyone. |
| 14 | `x-cache-bypass` | IMPLEMENTED | authenticated first, then straight to origin, not coalesced, not stored `server.rs:1243-1276` | `sec01_cache_bypass_header_does_not_skip_authentication` (exists: `m7_sec_fixes_tests.rs:240`) | Available to every `cache:read` principal. One client request = one origin request. |
| 15 | Token minting, pairing, revocation, per-session scoping | **no shipped tool** | `create_token` exists only as library/test API `jwt.rs:135` | n/a | See sections 2 and 3. |
| 16 | Event stream hygiene | IMPLEMENTED (auth) | `/api/events` requires Observer `server.rs:938-945` | `test_phase1_at10_sse_events_telemetry_rbac` | The loop is a 100 ms interval that rebuilds the payload per connection and never checks token expiry (`server.rs:955-972`); no stream cap found (the Lab stream does close at expiry and caps at 16: `lab_api.md` lines 20 and 29). |
| 17 | Lab API security model | IMPLEMENTED, TESTED, first review incorporated, second pending | `lab_api.md` (FACT-PRIOR; code `lab_api.rs` not read) | `tests/integration/lab_api/` | Process-start capability behind Operator; no rate limit or disk quota for runs; one process-wide service. |
| 18 | Secret redaction in config report/Debug/errors | IMPLEMENTED, TESTED | `src/config.rs` (FACT-PRIOR) | `secrets_never_appear_in_report_debug_errors_or_warnings` | Not tested across logs, metrics, SSE, error pages (M7 review section 3). |
| 19 | Evidence hashing | IMPLEMENTED (SHA-256 of manifest via `ring`) | `manifest.rs:157-164` | lab tests | **`git_tree_hash` is a hard-coded literal** (below). |
| 20 | Build identity stamp | PARTIAL | `BUILD_COMMIT`/`BUILD_DATE` from `option_env!` `config.rs:23-31`; set by `packaging/build-release.ps1:32` and `.sh:28` | none found | Ordinary `cargo build` yields `"unknown"`. Commit only: says nothing about a dirty tree or the binary actually run. No signing, SBOM or advisory scan (`limitations.md`). |

### 1.2 The hard-coded tree hash (verified)

FACT: `src/lab/manifest.rs:103` sets `git_tree_hash: "456bb25930004f1896a84c53e776e7b3ef29ee16".to_string()` and `:104` sets `is_dirty: true` with the comment "Working tree with accepted roadmap phase changes". Both are constants in `RunManifest::new` (`:85-118`), not read from git or from the build. The value is the current HEAD commit id, not a tree hash.
INFERENCE: every manifest created through this constructor states the same tree identity whatever code actually ran, so its `git_tree_hash` is not evidence of the build and must be treated as UNVERIFIED. I did not check which retained records were produced through this constructor. RECOMMENDATION: do not edit retained records; the owner decides how to annotate them (open question U-9).

### 1.3 Facts that bound every pillar

- FACT: Peer `InvalidateRequest` handling checks the originator against the TLS identity (`node.rs:505-511`) and then applies `ExactKey`, `Tenant` (`try_invalidate_tenant`) or `FullCache` (`try_invalidate_all`) with no per-scope or per-member authorization (`node.rs:533-559`). A compromised member can purge every tenant on every peer with one frame per peer. (Matches V4 S-14 and A2 row 10.)
- FACT: `try_invalidate_tenant` bumps a per-tenant epoch and does not delete entries; mismatched entries are dropped lazily on next access or by the sweeper (`cache.rs:576-607`, `:1269-1282`). At 10,000 distinct tenant epochs it falls back to a global epoch bump that fences every tenant (`cache.rs:273`, `:581-593`).
- FACT: shards are chosen by hashing the whole key including tenant (`cache.rs:544-550`), so each tenant spreads over all shards and all tenants share the same shard mutexes.
- FACT: `shard.bytes` is assigned at 13 sites (`cache.rs:444, 674, 744, 953, 1230, 1281, 1352, 1516, 1548, 1623, 1629, 1681, 1687`; grep). Any per-tenant accounting must be correct at all of them.
- FACT: development mode (no JWT secret) is anonymous on every route including `/api/control`, and trusts `x-tenant-id` (`docs/user/security.md` "Without PHANTOM_JWT_SECRET"; `server.rs:1215`).

### 1.4 Open SEC-M7-09, -10, -11 and what they mean for the pillars

| ID | What it is (FACT-PRIOR, M7 review table; `jwt.rs:57-84` and `server.rs:991-993` verified) | Pillar 1 (distributed) | Pillar 2 (intelligence) | Pillar 3 (Lab, UX, demos) |
|---|---|---|---|---|
| SEC-M7-09 | `requester` grants read and invalidate; `read`, `invalidate`, `observer`, `operator` are aliases; matching is case-insensitive (so `ADMIN` is admin); a token without `Bearer ` is accepted | None directly | A "read-only" intelligence endpoint role must not rely on aliases | A guest credential that says `requester` can DELETE. A minter must emit only the five canonical strings in lower case, and the UI must display the real role set, not a friendly name |
| SEC-M7-10 | SSRF filter lets 198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24, Teredo through | Becomes relevant if P1 adds peer-fill or multiple origins: more outbound destinations | Warm fetches are outbound fetches; they inherit the same gap | Origin is one static `ORIGIN_URL` today, so low risk for demos unless the origin's DNS is attacker-influenced |
| SEC-M7-11 | No auth-failure rate limit, no lockout, no revocation, no `nbf`, `iat` unused | A stolen token works on every node that shares the secret until `exp` | A flood of invalid tokens is invisible to any detector (not counted) | **Blocks the mandate**: "kick one device" and "revocation" cannot be built on the existing stack without gateway code. Pairing relies on short expiry as the only revocation until S3 (section 9) |

INFERENCE: "lockout" does not map well to this gateway because it has no login endpoint; failed validation is a stateless per-request check. What applies is per-source rate limiting of invalid tokens and a denylist for valid-but-withdrawn tokens. Lockout by IP would also lock out legitimate users behind one NAT.

Owner dispositions are in section 11.

## 2. Tenancy and identity model

### 2.1 Principals

| Principal | Credential | Granted roles | Tenant claim | Lifetime | Must not be able to |
|---|---|---|---|---|---|
| Operator (presenter or admin on the host) | JWT minted locally on the host that holds the signing secret | `scenario:operate` + `telemetry:read` | none | at most 60 min, one device label, one gateway `aud` | be shown on a guest device, be placed in a URL, be stored on disk by the UI |
| Observer (audience, read only) | JWT | `telemetry:read` only | none (today); tenant-scoped variant proposed (S10) | 10-15 min | read cache bodies, invalidate, call control |
| Tenant client | JWT | `cache:read`, optionally `cache:invalidate`; never the `requester` alias | mandatory, from the tenant registry | 10 min | see or change another tenant, read telemetry beyond its own tier |
| Cache node | mTLS leaf certificate, SAN `node-{id}.{cluster}.phantom.internal`, SHA-256 fingerprint in the membership file | n/a (not a JWT principal) | n/a | cert validity chosen at generation (demo: days) | send scopes or rates outside its capability entry (S6) |
| Guest device | A tenant-client or observer JWT obtained through pairing | the narrowest of the two above | a demo tenant named `demo-<session8>-<label>` | 10 min, renewable only by the presenter's pairing service | ever hold operator or admin; the pairing service refuses by construction |
| Pairing service (system component) | holds the session signing secret | none on the gateway | n/a | one demo session | run on a guest-reachable interface without TLS or the insecure-LAN banner |

FACT: `admin` satisfies every role (`jwt.rs:58-61`), so no demo token carries it.
FACT: `JwtClaims` has no `deny_unknown_fields` (`jwt.rs:30-42`), so tokens can already carry extra claims such as `jti` and `sid`; the gateway ignores them today. INFERENCE: a minter can start emitting `jti` now, and a later denylist needs no token-format change. Check: a test that a token with extra claims validates.

### 2.2 Token claims and scoping

Using only existing claims: `sub` = device label (`clientA-laptop`), `iss` fixed, `aud` = per-session or per-node string, `exp` short, `tenant_id` for tenant clients only, `roles` from the canonical five (lower case).
Additional claims to emit now (ignored today): `jti` (128-bit random), `sid` (session id). Used later by S3.

Scoping options, all with the current validator (FACT: `iss`/`aud` are one string per gateway process, `config.rs:110-113` and `main.rs:411-414` per V3):

- Per-session secret and `aud`: tokens from an old or other session fail. Cost: restart the gateway per session. The cache is in memory, so a restart is a clean reset. (PROTOTYPE-ready.)
- Per-node `aud` (and secret): a token for node 1 is refused at node 2. Cost: a client needs one token per node. Benefit: a stolen node-1 secret cannot mint node-2 tokens. Trade-off, not a free win.
- HS256 trade-off (FACT from the design): every gateway that verifies tokens can also mint them. A compromised guest-facing gateway host therefore exposes its own secret only if secrets are per node or per session; with one shared secret it exposes everything. RECOMMENDATION: per-session secret for demos. For long-lived deployments an asymmetric algorithm would let gateways hold only a verification key; `jsonwebtoken` 9 may support EdDSA, but that is from memory and is **unverified here** (U-3). It also needs a change to the HS256-only check at `jwt.rs:153`.

### 2.3 What can be built on the existing JWT stack, and what needs code

| Need | No new dependency, no gateway change | Needs gateway code (no new crate) | Needs a new dependency or system change |
|---|---|---|---|
| Scoped, short-lived credentials | External minter script using `node:crypto` or PowerShell HMAC-SHA256; or a small Rust bin calling `JwtValidator::create_token` (`jwt.rs:135`) | A maximum-lifetime check at validation (`exp - iat` limit) | none |
| Per-session isolation | New secret and `aud` per session via env; restart | none | none |
| Revocation of a single credential | none (only expiry; restart with a new secret kills all tokens) | `jti` denylist checked in `authenticate_request` (`server.rs:985-1044`), bounded size, entries expire at the token's `exp`; operator-only revoke route | none for a per-node list; propagating revocation across nodes needs a channel (P1) |
| Stream closure at expiry | none | `/api/events` loop checks `exp` as the Lab stream does (FACT-PRIOR `lab_api.md:20`) | none |
| Auth-failure and per-principal limits | none | token buckets in std, bounded map | none |
| Strict roles (SEC-M7-09) | minter allowlist | a config flag turning aliases and bare tokens off | none |
| Asymmetric signing | none | validator change | possibly a direct dependency declaration of an already-present transitive crate (needs the CLAUDE.md section 2 justification) |

### 2.4 Pairing and onboarding

**Why a shared password is rejected** (reasons, in order of weight):
1. No individual revocation: changing it removes everyone; keeping it keeps everyone.
2. No attribution: every event is by the same principal, so audit evidence cannot say who did what.
3. If the "password" is the JWT secret, it can mint admin tokens for every tenant. FACT: any holder of the secret can (`jwt.rs:135-139`).
4. It leaks the way passwords leak: screens, photos, chat, shell history. One leak is permanent until rotated.
5. Human-chosen entropy plus no rate limit (SEC-M7-11) means online guessing is not bounded.
6. It cannot be scoped to a tenant, role or device.
7. Teardown needs a global rotation instead of expiry.

**Design (DESIGNED BUT NOT IMPLEMENTED; prototype as an external script).**

Demo flow (guest device):
1. The presenter creates a demo session. The pairing service generates a session secret and `aud`, starts or restarts the guest-facing gateway with them, and shows an enrollment code.
2. Code: 8 characters from a 32-symbol alphabet without look-alike glyphs, from a CSPRNG, single use, valid 120 s, at most 5 attempts per code, at most 20 failed attempts per minute for the whole service. Arithmetic: 32^8 = 2^40, about 1.1e12 codes. At most 40 failed guesses fit in one code lifetime, so the chance a guesser hits one live code is about 40 / 1.1e12, about 3.6e-11, assuming uniform random codes and working limits. (A 6-character code would be about 4e-8.) This is arithmetic under stated assumptions, not a measurement.
3. The guest enters the code at the pairing page. **Presenter approval:** the request appears on the presenter's screen with device label, source address and a 2-digit confirmation number shown on both screens. Nothing is minted until the presenter approves. This is the defence against someone who photographs the code from the room.
4. On approval the service mints a 10-minute JWT (tenant client or observer), shows the guest a one-time response, and burns the code. Renewal needs the presenter's session to be alive; the minimum viable version has no renewal (re-pair).
5. Every step (code issued, attempted, approved, denied, minted, expired, session ended) goes to the session event log with device label and a truncated SHA-256 of the `jti`. Never the token. Never the code.

Transport: the pairing endpoint and gateway should use TLS. Plain HTTP exposes the minted token to anyone on the LAN. If plain HTTP on a presenter-owned private network is chosen (V3 option 3, consolidated R10), the UI shows a permanent LAN-INSECURE banner and the TTL stays at most 10 minutes. Bearer tokens are not bound to the device that obtained them (no DPoP or mTLS binding exists): a stolen token works from any machine until expiry. State this on the screen, do not hide it.

Operator device onboarding (non-demo): operator tokens are minted only on the host holding the signing key, by a local command, with TTL at most 60 minutes, never through the guest pairing page. QR codes are optional convenience; they add no security and the repo has no QR encoder (V3 section 3.3).

Completion evidence for pairing: see S2 in section 9.

### 2.5 Node identity, bootstrap trust, membership revocation

FACT-PRIOR (V3, A2; code `cluster.rs` bodies not re-read): TLS 1.3 only, client certificate mandatory, trust anchored to the cluster CA, SAN identity parsed strictly, fingerprint and `active` flag checked, per-frame re-validation, epoch on the registry.
FACT: `rcgen` is a dev-dependency (`Cargo.toml`), so node-certificate generation exists only in test code. `scripts/gen-demo-certs.ps1` covers the ingress server certificate and depends on `openssl` being on PATH (FACT-PRIOR V3).

Cluster PKI tooling (DESIGNED BUT NOT IMPLEMENTED):
- Generator command: create a CA (key kept on the presenter or operator machine only), issue N node certificates with the exact SAN, write each node's key and certificate, write the membership file with fingerprints, addresses and `active` flags. Short validity for demos (days).
- Bootstrap trust: the CA public certificate and the fingerprints are the trust root. Node private keys are generated centrally for demos and copied by a trusted channel, then deleted from the generator; for operator deployments a CSR flow is better but needs X.509 parsing support that I have not verified in the dependency tree.
- Dependency choice (owner decision D-PKI): (a) promote `rcgen` from dev-dependency to a dependency of a separate tool target; Cargo.lock already contains `rcgen` and `aws-lc-rs` (FACT: grep of `Cargo.lock`), so the build cost is partly paid, but it is still a new direct dependency; (b) extend the `openssl` script (host dependency, no repo dependency); (c) PowerShell certificate cmdlets (Windows only). `ring` alone cannot encode X.509.
- Membership revocation surface: today only "edit the file, restart every node". A smaller step than dynamic join: a **signed membership document** (Ed25519 via `ring`, public key pinned at node start, strictly increasing `version` as the registry already requires) loaded by a SIGHUP-free periodic file check or an operator-only route that calls `registry.update()`. FACT-PRIOR A2: `update()` enforces epoch monotonicity and fires the revocation channel that evicts open sessions. Needs: a caller (new code in `main.rs` or `server.rs`), a router that tolerates a changed member set (A2 notes `router.nodes` is a fixed vector and that mismatched `membership_epoch` makes anti-entropy handlers drop connections), and a test that a revoked member's next frame on a pooled connection is refused (`test_p3c_per_request_revocation_on_reused_connection` already covers the library side).
- Join is separate and larger (P1 owns it). Revocation-only is the security-relevant minimum.

## 3. Boundaries: where a tenant must not cross

Distributed and intelligent features must not cross tenant boundaries. The rules below are requirements on P1 and P2, not statements about current code:

| Channel | Rule | Why |
|---|---|---|
| Forecast or popularity state | Keyed by registered tenant id, bounded count, never shared per-key statistics across tenants; any global top-k is operator-only | A shared model's decisions reveal who asked for what (V4 section 5.2, V2 per the consolidated report R13) |
| Warm state | A warm fetch is billed to the target tenant's quota (bytes, origin permits) and may not evict another tenant's guaranteed region | A shared permit pool or shared eviction makes warming observable to others |
| Pre-warm on peers | Only after durable invalidation exists (V4 5.2); charged to the key's tenant on the receiving node | Otherwise stale content spreads |
| Telemetry | Tiered (section 4.7); tenants see only their own series | Observer-wide metrics are cross-tenant by construction today (V4 S-17) |
| Timing and occupancy | Shared shard mutexes and shared eviction are a channel that is **unmeasured**; reservations narrow it, do not remove it | FACT: all tenants share shard locks (`cache.rs:544-550`) |
| Peer frames | Scope and rate limited per member (section 6) | Insider blast radius |
| Epoch table | Tenant registry bounds the number of tenants so the 10,000-entry fallback cannot be reached by a client | FACT `cache.rs:273,581` |

## 4. Tenant resource management design

This is the largest gap. Today only the namespace is isolated (1.1 rows 3 and 10).

### 4.1 Goals and non-goals

Goals: (G1) measure per-tenant usage; (G2) enforce per-tenant byte and entry caps; (G3) evict fairly under pressure; (G4) limit each tenant's share of origin permits and bypass; (G5) rate-limit by source, principal and tenant; (G6) expose tenant telemetry in privacy tiers; (G7) behave sensibly in a cluster.
Non-goals: pooling capacity across nodes; making quotas tunable by intelligence (RECOMMENDATION: quota values are operator configuration only, matching the consolidated report invariant that hard caps are never intelligence-tuned); hard real-time latency isolation (shared locks remain).

### 4.2 The tenant registry (prerequisite)

RECOMMENDATION: a startup-loaded policy file (no hot reload exists, consistent with membership and CORS handling) listing tenants and limits, and a mode:
- `closed`: a valid token for an unregistered tenant gets 403 (`UnknownTenant`). Recommended for demos and production. This also bounds tenant cardinality, which makes the epoch-table fallback unreachable by clients.
- `open`: unregistered tenants share one default-limits bucket (or each gets default limits up to a count cap).
Fail-fast validation at startup, as SEC-M7-05 and -08 did for other keys: sum of guarantees at most 80% of the byte budget, each cap at least its guarantee, tenant count at most 1,024 (an explicit bound, number is a proposal not a measurement).
Lesson from `PHANTOM_DEFAULT_TTL_MS` (CLAUDE.md section 5: validated but not wired): every new key needs a test that changing it changes behaviour.

Per-tenant fields: `guaranteed_bytes`, `max_bytes`, `max_entries`, `origin_reserved_permits`, `origin_ceiling_permits`, `bypass_per_min`, `delete_per_min`, `rps`. Meaning: a guarantee is capacity that other tenants' pressure cannot evict; a cap is the most the tenant may hold even if the cache is empty.

### 4.3 Stage A: accounting hook only (minimal first increment)

What: add per-tenant byte and entry counters, observe only, no behaviour change.
Design (INFERENCE-based):
- Store a small tenant slot in `KeyState` at insert so each mutation site has it without hashing a 128-byte string.
- Per-tenant atomic counters in a table behind a read-mostly `RwLock<Arc<...>>`, the same pattern as `epoch_snapshot` (`cache.rs:288`), so a lookup does not take an exclusive lock. Updates happen under the shard mutex already held at each of the 13 sites listed in 1.3. **No new lock, lock order unchanged** (shard mutex, then `tenant_epochs`).
- Extend the audit: `audit_bytes()` (`cache.rs:1827`) gains a check that the sum of tenant bytes equals the sum of shard bytes (a new invariant I9 alongside I1-I8).
- Hit path untouched. Cardinality bounded by the registry (4.2) or an overflow bucket in open mode.
- Export: operator-tier metric only at first.
Why first: it is cheap (A2 and the audit recommend it as one of four forward-compatible hooks), it lets S-10 be measured instead of assumed, and P1/P2 can both read it.
Completion evidence: a property test over random insert/evict/delete/invalidate sequences shows tenant sums equal shard bytes after every step; a test that omits one of the 13 sites fails (so the check is shown to bite); a release-build micro-benchmark with repetitions and a confidence interval showing the miss-path cost, and hit-path cost unchanged within the interval; a measured noisy-neighbour run (4.9) using the new counters.
What could make it fail: a missed mutation site (the 13-site list may be incomplete if code moves); counter drift through panics (`lock_poison_recovered` shows poisoned locks are tolerated, `cache.rs:1414-1417`); hot-path regression; unbounded tenant growth in open mode.

### 4.4 Stage B: caps and fair eviction

Admission and caps (INFERENCE from `cache.rs:1316-1345` and `:1490-1539`):
- At `begin_fetch_atomic` miss, `needed_bytes` is only key plus fixed overhead; the body size is known at `finalize_fetch`. So the byte cap is enforced at finalize, using the existing pattern: try to free space by eviction (up to 32 victims, `cache.rs:1504`), otherwise count a rejection and do not store (the existing `byte_budget_rejections` path, `:1513-1514`). A tenant-cap rejection behaves like that path: waiters still get the response, nothing is stored.
- Entry caps and the global entry limit are checked at begin (`:1339`).
- Order of checks at finalize: generation and epoch checks first (unchanged fencing), quota decision after them, so quotas never override a fencing decision. FACT: fencing stores only if `state.generation == op_gen` and epochs match (CLAUDE.md section 5; `finalize_fetch` at `cache.rs:1401`).

Fair eviction: why not just extend `is_pinned`. FACT: SIEVE `select_victim` skips pinned keys without clearing their `visited` bit and bounds work at about 2n inspections (`eviction.rs:186-219`). A "protected tenant" skip through the same closure would work but each eviction could scan many protected entries. So:
- RECOMMENDATION: a **tenant-aware wrapper** implementing the existing `ShardEvictionPolicy` trait (`eviction.rs:16-35`). It keeps one instance of the chosen faithful policy per tenant per shard and picks which tenant to take a victim from: first the inserting tenant if it is over its cap; else the tenant with the largest positive overage (usage minus guarantee); else the inserting tenant; else refuse the insert. Within a tenant the policy is unchanged, so SIEVE/LRU/S3-FIFO faithfulness is kept per tenant.
- The wrapper needs byte usage. The trait's `on_insert(&CacheKey)` carries no size, so either the wrapper receives the shared usage table from Stage A at construction (no trait change) or the trait grows a size argument (frozen-adjacent change). Prefer the first.
- `rebuild_from` and policy switching (`set_eviction_policy`, `cache.rs:500-509`) must rebuild per-tenant instances. P2 note: an intelligence-driven policy switch must not alter quotas.
- Cost honesty: choosing the victim tenant is O(tenants-with-overage) per eviction; with up to 32 evictions per call under a shard mutex this must be bounded (maintain a small over-quota set) and measured. I do **not** claim O(1) amortized for the wrapper.

Interaction with epochs and fencing (needs owner approval and independent review; CLAUDE.md sections 2 and 5):
1. Epoch-stale entries are resident until swept (`cache.rs:1269-1282`), so after a tenant invalidation the tenant's usage includes dead bytes. Quotas will count them. RECOMMENDATION: accept in the first version, expose a "dead bytes" estimate later, and test that an invalidation storm does not wedge a tenant (the sweeper or next access reclaims).
2. Never evict an entry with active ops or an in-flight fetch (I5): the wrapper must honour `is_pinned`. Consequence: caps are soft by at most the pinned bytes. Bound to be stated from the origin cap times the maximum entry size once measured; I have not derived it.
3. Generation fencing: a rejected store must not change `generation` or epochs. A test must show a quota rejection followed by a DELETE and refetch still obeys fencing.
4. Tenant epoch fallback (global bump) is independent of quotas; the registry (4.2) is what prevents client-driven exhaustion.
5. Per-shard versus global: a tenant's keys spread over 8 shards, but an over-cap tenant may have no evictable entries in the current shard. Then the insert is refused (counted) and the cap overshoots by at most what other shards hold until their next eviction. UNRESOLVED: measure the overshoot; if too loose, use per-shard fractions of the cap plus the global counter.

Modules that must change (all owner-approval items): `src/core/cache.rs` (accounting at 13 sites, admission at finalize, `audit_bytes`); `src/core/eviction.rs` (wrapper); `src/config.rs` (keys, validation, report, and the `m7_config_tests` matrix); `src/api/server.rs` (tenant policy load, origin permits, rate limiting, new routes); `src/intel/telemetry.rs` (counters); `src/main.rs` (wiring). There is no explicit list of "frozen" files in what I read (V3 D14 notes the same); this is a heavy-invariant module, so independent review and a failing-before test are required by CLAUDE.md section 5.

### 4.5 Origin share, bypass and rate limits

- Origin permits (S9): today the cap is gateway-global (`server.rs:2416-2426`, `acquire_origin_permit(exec)` takes no tenant). `fetch_origin_gated` already receives `cache_key`, so a per-tenant counter is reachable. Two-level rule: a tenant may take a permit if its in-flight count is below its reserved number, or if unreserved global capacity exists and it is below its ceiling. Keep reserved sums at most half the global cap. A tenant ceiling below the global cap guarantees others at least cap minus ceiling.
- Bypass: FACT it is 1:1 amplification, uncached, for any `cache:read` principal. RECOMMENDATION: a policy of `allow` (today), `rate_limited` (per-principal token bucket and per-tenant fraction of the ceiling) or `operator_only`. Default for guest tokens: `rate_limited` at a low number.
- Nuance: with `PHANTOM_FORWARD_UPSTREAM_AUTH=true` every authenticated read is `BYPASS-AUTH` and never cached (`server.rs:1236-1241`, `docs/user/security.md`), so cache quotas are moot and origin ceilings are the only protection.
- DELETE: one DELETE becomes a local purge, a synchronous primary leg and fan-out to every member plus repair-ledger entries (FACT-PRIOR V4 6.1). A tenant with `cache:invalidate` can drive that. A per-tenant `delete_per_min` limit is needed. Overflow collapses to a tenant flush and then possibly a full flush on peers (V4 S-15).
- Rate limiting: token buckets from std with bounded maps, no dependency. Keys: source address (before auth, for invalid tokens), `sub`, tenant. Behind a reverse proxy the source address is the proxy; do not trust `X-Forwarded-For` by default. Limits are per node, not cluster-wide.
- HTTP hardening (FACT: none found in `server.rs`): header-read and body-read timeouts, a connection cap, an SSE stream cap with per-stream cost accounted. INFERENCE: each `/api/events` stream rebuilds the payload every 100 ms, so N observer connections cost N times; an Observer token can therefore consume CPU. Check: count streams and CPU under a probe.

### 4.6 Quota semantics in a cluster (P1) and with pre-warming (P2)

- FACT-PRIOR (A2, audit): node caches are independent and capacity does not pool. So a quota is per node. A tenant's total footprint is at most N times its per-node cap; its origin load at most N times its per-node ceiling. A cluster-wide tenant quota is not enforceable without coordination; say so on any cluster screen.
- If P1 adds peer-fill or owner forwarding, a fill is charged to the key's tenant on the node that stores it, not the requester's node, and peer-originated fills need their own per-member rate limit. Otherwise a compromised member can fill any tenant's quota.
- Tenant-scope invalidation frames interact with the epoch table (section 6).
- The tenant policy file should be identical on all nodes; its hash goes in the session object (V3 5.2) and a mismatch is flagged, not silently tolerated.
- Pre-warming (P2): warm fetches use the tenant's origin ceiling at lower priority (priority does not exist yet, consolidated report) and are dropped, not queued, when the tenant's ceiling or the global cap is saturated; they insert only into free guaranteed space or evict the same tenant's entries; a warm entry cannot raise a quota; calendar entries carry per-entry budgets at or below the tenant's quotas. A warm path must never use a privileged identity that the tenant could not use itself, and never warm `BYPASS-AUTH` keys.

### 4.7 Per-tenant telemetry and privacy tiers

FACT-PRIOR: telemetry today has no tenant dimension; SSE is a 100 ms snapshot; any Observer sees global aggregates (V3 1.1 item 7, V4 S-17).

| Tier | Audience | Content | Not included | Needs |
|---|---|---|---|---|
| A tenant-self | a tenant client with a new `telemetry:tenant` role and a tenant claim | own requests, hits, misses, bypasses, rate-limited counts, own bytes and entries against own quota, own origin fetches, own evictions split into "by own pressure" and "by own cap" | global totals, other tenants, node-wide eviction counts, who caused pressure | new role (auth change), tenant counters (Stage A) |
| B observer-aggregate | existing `telemetry:read` | global counters as today | tenant ids, per-tenant series | none |
| C operator | `scenario:operate` | per-tenant table with ids, pressure, rejection counts, per-peer frame audit, key fingerprints (random-seeded, as in diagnostics) | key text | Stage A and S6 |
| D lab or demo only | operator in a demo session with synthetic tenants | key-level event stream | never enabled where real tenants exist | event tap (P2/P4) |

INFERENCE: with shared capacity, a tenant's own eviction count rises when others are active, so Tier A leaks pressure. Reservations close this for the guaranteed region only; above it the channel remains. Coarsening values reduces but does not remove it, so I do not propose coarsening as a mitigation. In a demo where the presenter owns all tenants, the tiers are not being tested and the screen must say so.

Alternative considered: a dedicated `PartitionedCache` instance per high-isolation tenant. It removes shared locks and eviction coupling for that tenant, at the cost of fragmenting the byte budget and doubling accounting paths. Kept as an option, not a recommendation.

### 4.8 What is NOT claimed

No fairness guarantee, no latency isolation, no cluster-wide quota, no resistance to a cache-timing adversary, and no performance figure. Any number needs a release build, hardware and OS, workload, seed, baseline, repetitions and confidence intervals (CLAUDE.md section 4).

### 4.9 Tests that try to falsify isolation

Each is written first against the current tree where possible so that "fails without the change" is demonstrated, not assumed. All are loopback, release build, seeded, repeated; results labelled loopback; thresholds pre-registered, not chosen after seeing data.

| Test | Setup | Attack or probe | Pass condition (to be pre-registered) | Fails today? |
|---|---|---|---|---|
| T-NN noisy neighbour | tenant B warms a working set; measure B's hit ratio and origin fetches | tenant A floods unique large keys | B's hit ratio stays within a pre-registered margin of its no-A baseline once B has a guarantee | expected yes (S-10 hypothesis; CONFIRM by running) |
| T-QX quota exhaustion | A reaches `max_bytes` | A keeps inserting | A evicts only its own entries; others' bytes unchanged; per-tenant audit shows it | cannot run (no quota) |
| T-EP eviction pressure with guarantees | total cache at budget; B within guarantee | A and C above guarantee | B loses nothing to others; the overage tenants lose first | cannot run |
| T-BF bypass flood | A sends bypass at rate R | measure other tenants' origin success | others keep at least their reserved permits | expected: others see 503 (V4 S-09) |
| T-AF auth-failure flood | many invalid tokens from one source | concurrent valid traffic from others | valid traffic latency within margin; failures counted | no counter today |
| T-DF delete flood | A with `cache:invalidate` | DELETE storm in a 3-node cluster | other tenants' entries unaffected; ledger bounded | partly covered: `test_m6_one_tenants_overflow_does_not_flush_other_tenants_on_peer` (exists) |
| T-EE epoch exhaustion | closed mode | try to register or use 10,000+ tenants | refused; no global flush | cannot run |
| T-SC side-channel probe | B plants canary keys, measures own latency and canary survival | A idle vs A active; with and without guarantee | report effect size and interval; **a null result is reported as null** | unknown |
| T-INV invariants | proptest over random ops | all sites | tenant sums equal shard bytes; fencing unchanged | n/a |
| T-RT rename invariance | metamorphic | rename tenants | behaviour identical | n/a |

## 5. Abuse and resource-exhaustion coverage

Test names are from the V4/M7 documents and greps; I confirmed that the ones marked "exists" are present in `tests/`, and did not run any.

| Threat | Current control | Covered by (names) | Gap | Proposal |
|---|---|---|---|---|
| Cache-key confusion | tenant charset, `:?` and dot-segment rejection, round-trip property | `canonical_key_roundtrip_tests`, `m7_sec_round2_tests::h1_*`, `m7_sec_round3_tests::l1_*` | `%2f` vs `/` and mixed-case escapes untested against an origin normaliser (V4 S-06 probe) | S12 probes |
| Poisoning via client headers | only gateway-controlled headers forwarded | `m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant` (exists), `m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit` | compromised origin is out of scope | document |
| Cache deception | storage decided by origin headers; 200 without freshness headers cached 60 s (FACT-PRIOR V4) | `test_phase1_at08_rfc9111_*` | misconfigured origin is cached by design | demo both outcomes honestly |
| Error caching (CPDoS) | only 200 and explicit-freshness 301/404/410 cacheable (FACT-PRIOR `freshness.rs`) | `test_p2_truncated_response_not_cached`, `test_redteam_close_delimited_body_not_cached` (exists) | no end-to-end test that a 400/5xx is never stored (V4 S-08) | S12 |
| Stampede | single-flight coalescing, waiter cap | `coalesce_tradeoffs_tests`, `origin_cap_tests` | cap sheds to 503 for all | S9 |
| Amplification: bypass, unique-key flood | origin cap only | `sec01_cache_bypass_header_does_not_skip_authentication` | no per-principal limit | S9 |
| Request size | 512 KiB body, key length caps | `test_m1_t26_ingress_request_body_limit` | header size, slow-send, connection count untested | S9 |
| Auth failure flood | none | none | not counted, not limited | S3 |
| Invalid-token CPU cost | HS256 verify per request | none | unmeasured | S12 |
| SSE fan-out | none for `/api/events`; Lab stream capped at 16 | `low_events_stream_is_observer_gated` | uncapped, no expiry close | S3, S9 |
| Invalidation flood and fan-out | bounded queues and ledger | `test_m6_*`, `cluster_prewarm_repair_tests::*overflow*` | no per-tenant rate | S9 |
| Peer frame flood | frame size cap 1 MiB | `test_p3b_at11_oversized_payload_rejected_before_alloc` (exists) | no receiver rate limit | S6 |
| Compromised member | identity only | originator test | scope and rate | S6 |
| Epoch-table exhaustion | global fallback is correctness-safe | n/a | mass flush if reachable | registry (4.2) |
| Lab API resource use | one active run, 16 streams, body 16 KiB | `lab_api` tests | no rate limit or disk quota for runs | S4 |

## 6. Peer authorization (S6)

### 6.1 What a compromised member can do today

| Capability | Today | Evidence | After S6 | Residual |
|---|---|---|---|---|
| Purge every tenant on every peer (`FullCache`) | yes | FACT `node.rs:553-558` | refused unless the member has an explicit capability; default off | origin stampede if enabled |
| Purge any tenant (`Tenant`) | yes | FACT `node.rs:547-552` | per-member tenant allowlist or off; rate-limited | allowed tenants still exposed |
| Purge any exact key | yes | FACT `node.rs:534-546` | rate-limited and audited, **not preventable** | see below |
| Poison another member's sequence space | blocked | originator check `node.rs:505-511` | unchanged | none known |
| Learn other tenants' key names | UNRESOLVED | the code comment says replay serves records the node originated for the requester (`node.rs:523-524`); I did not read the anti-entropy request arm (`:578-640`) | check and restrict | INFERENCE: key strings include tenant and path |
| Read cached data via peer GET | no real data | FACT-PRIOR V4: cluster GET returns fabricated `val_node_N` bodies and is pinned by `known_gaps::pin_kg6_*` | **becomes critical if P1 builds peer-fill**: per-tenant read authorization on the wire is then mandatory | design with P1 |
| Drop connections by wrong epoch | possible | FACT-PRIOR A2 | rate limit | nuisance |
| Mint client tokens | yes if the member host holds the HS256 secret | FACT `jwt.rs:135` | per-node or per-session secrets (2.2) | host compromise |

The "cannot be prevented" row matters: every member may be the issuer of a DELETE fan-out, so exact-key invalidation by any member for any tenant is inherent to the current design. Peers cannot verify that a client token authorised it. Options: end-to-end provenance (a signed statement from the issuing gateway including tenant and the `jti` fingerprint, still trusting the issuer), or accept and bound it with rates and audit. RECOMMENDATION: bound and audit; state the residual plainly.

### 6.2 Design (DESIGNED BUT NOT IMPLEMENTED)

- Capability record per member in the membership file (A2 suggests the same extension point): `exact_key: true`, `tenant_scope: none | [tenant ids]`, `full_flush: false`, `anti_entropy_pull: own_streams`, `max_frames_per_s`, `max_flush_per_hour`.
- Receiver checks before applying: capability, then token bucket per member and scope, then apply. A refused frame returns a typed rejection so the issuer does not retry forever.
- Correctness trade-off to settle with P1: the repair ledger escalates to `Tenant` and then `FullCache` on overflow (`router.rs:1920,1978-1983` per V4). If a peer refuses `FullCache`, the issuer's debt cannot be repaid. Alternative: receiver-driven degrade (the receiver flushes its own tenant or cache when it detects a gap). That moves the decision to the party that bears the cost. UNRESOLVED, P1 decides.
- Audit: bounded per-member counters by scope and outcome (operator tier) plus an append-only record of every refused or rate-limited frame (member id, scope, tenant id, key fingerprint, outcome), no key text.
- Tests: a fixture member with a valid certificate sends each scope and exceeds each rate; assertions on the receiving cache (entries survive), on the typed rejection, and on the audit record; plus the existing originator test stays green. Each fails without the change.

## 7. Control-plane safety

- **Roles:** FACT: `/api/control` needs Operator (`server.rs:1075-1081`), events/metrics/diagnostics/intelligence need Observer, Lab reads need Observer and Lab start/cancel need Operator (`lab_api.md` routes table). An Operator-only token cannot read Lab (separate role).
- **Guests never get operator:** enforced by the pairing service's allowlist, by the minter's allowlist (canonical roles only), and by **placement**. RECOMMENDATION (no code): run the Lab API only in a loopback-bound process with `PHANTOM_LAB_ENABLED=true` and run the guest-facing gateway as a separate process with the Lab disabled (FACT-PRIOR `lab_api.md`: the Lab is opt-in per process and refuses non-loopback binds without a JWT secret). `/api/control` still exists on the guest-facing gateway but needs Operator, which guests do not hold. Stronger (needs code, S4): a separate control listener or bind address so guests cannot even reach those routes.
- **Lab API process-start risk:** FACT-PRIOR: `POST /runs` can spawn gateway child processes; the executable comes from `PHANTOM_LAB_GATEWAY_EXE` or the current binary, not from the request; request fields are bounded and `out_dir`/`overwrite` cannot be supplied. So an Operator token is a bounded "start experiments on this host" capability, not arbitrary command execution. INFERENCE: it is still a CPU, disk and port-use capability with no rate limit or disk quota (`lab_api.md` known limits), so it must stay off guest-reachable interfaces.
- **`NODE_FAILURE` and other control events:** FACT-PRIOR V3/V4: logical state flags, and in development mode anonymous; any unknown event value resets nodes to Active. Label as logical in every UI.
- **Audit trail:** FACT: the control handler prints the event name to stdout (`server.rs:1084`); the in-memory diagnostics ring and the 100-record intelligence audit are volatile, not tamper-evident (V4). RECOMMENDATION: an append-only, hash-chained JSONL control audit (reuse the Lab `events.jsonl` discipline) recording time, `sub`, roles, `jti` fingerprint, route, outcome, node id; denied attempts included and counted; file permissions restricted; its chain head included in evidence bundles. Never log tokens or the signing secret; test with a canary secret (the M7 review notes this was not done across logs).
- **Dangerous actions list** for confirmation prompts in the UI (P4): start/cancel runs, `NODE_FAILURE`/reset, any future tenant or full flush route, any future revocation route.

## 8. Security demonstration requirements for external audiences

**Statement (binding for every demo): demonstrations are not certification.** A demo shows that one control behaves as described under one fixture, mode, build profile and host. It does not show absence of vulnerabilities, resistance to an adaptive attacker, correctness under real networks, or compliance with any standard. The tests were mostly written by the same project; penetration testing, fuzzing, dependency advisory scanning, TLS scanning, independent code review of all changes and multi-host testing have not been done (M7 review "Not done"). Words like "secure", "zero trust" or "guaranteed" are not used without the bound and failure model (V4 section 9).

Safe defaults for any session shown to other people:
1. Secure mode only for the guest-facing gateway (JWT secret set, auth required). If development mode is shown, label it "INSECURE BASELINE" on every screen.
2. Per-session secret and `aud`; tokens at most 10 minutes; no operator or admin token leaves the presenter device.
3. Guest-facing gateway has the Lab disabled; control and Lab are loopback only.
4. Origin is a sandbox mock, SSRF allowlist limited to it; no real data; no infrastructure credentials on any demo host.
5. Demo tenants named per session; registry in `closed` mode once S7 exists.
6. TLS where feasible; otherwise LAN-INSECURE banner on a presenter-owned private network.
7. Tokens and codes never displayed after use, never in URLs, never in logs or evidence; screen-share privacy mode (P4).
8. Teardown checklist: stop only recorded PIDs, let tokens lapse or rotate the secret, delete private keys and session directories the launcher created, list any firewall rule or trust anchor a human added (system changes need owner approval).
9. Inviting attacks: only guest tokens against the sandbox, rate limits on, written scope; never against the presenter host or production. UNRESOLVED whether the owner wants this at all.

What each demo shows and does not prove (full catalogue: V4 sections 2-3; additions here):

| Demo | Status today | Shows | Does NOT prove |
|---|---|---|---|
| Confidentiality isolation between tenants (secure mode), with dev-mode contrast | IMPLEMENTED + TESTED (V4 A4) | key namespaces and tenant-from-token | capacity isolation, timing isolation, behaviour with a misconfigured origin |
| Noisy neighbour (limitation) | needs T-NN | measured interference | that quotas fix it, until Stage B exists |
| Scoped short-lived guest credential and expiry | PROTOTYPE ONLY (script) | a guest token cannot call control; an expired token gets 401 | replay resistance, device binding, revocation |
| Revocation | BLOCKED until S3 | n/a today; show expiry only, labelled as such | immediate revocation |
| Peer scope authorization | DESIGNED BUT NOT IMPLEMENTED | n/a; today show the **limitation** with a fixture member | any insider protection |
| Evidence bundle tamper detection | DESIGNED BUT NOT IMPLEMENTED (S11) | edit/reorder/delete detection against a trusted head | authenticity without an out-of-band key; that the code under test was the claimed code |

## 9. Proposals (each with the nine required fields)

Fields: why necessary; builds on; what must change; depends on; how tested; completion evidence; what could make it fail. Owner approval is needed for all application-code changes (CLAUDE.md section 2); independent review is needed where fencing, epochs, invalidation or authentication are touched.

**S1. Minter and token policy.**
- Why: no shipped way to make scoped, short-lived credentials; the mandate needs them.
- Builds on: `JwtValidator::create_token` (`jwt.rs:135`), or `node:crypto`/PowerShell HMAC.
- Change: an external script first (no repo change); optionally a `[[bin]]` later. Allowlist canonical roles in lower case; refuse operator/admin for any guest label; reads the secret from a file or env, never prints it; always emits `jti`, `sid`, `exp` at most TTL.
- Depends on: nothing.
- Tested: script tests with a canary secret (output and logs scanned); validator accepts the tokens; tokens with alias roles are never produced.
- Completion evidence: a recorded run with the canary scan clean, token claims listing, and a test that a token with extra claims validates.
- Could fail: secret leaks through shell history, process list or logs; operators bypass the script.

**S2. Pairing service (code, presenter approval, per-session secret).**
- Why: guests need credentials without sharing a password or the secret.
- Builds on: S1, `aud`/secret per session, Lab-run event-log pattern.
- Change: new component outside the gateway (script first). Gateway untouched.
- Depends on: S1; TLS decision D-TLS; P4 for the approval screen.
- Tested: wrong code, reused code, expired code, over-attempt, global limit, denied approval, guest asks for operator role (refused), token cannot call `/api/control` (403), expired token 401, teardown leaves no listener.
- Evidence: 3-device physical run (presenter, two guests) with the event log and its hash chain; labelled LAN, Windows, build profile.
- Could fail: onlooker redeems the code and the presenter approves wrongly; LAN capture of the token on plain HTTP; clock skew makes tokens look expired (V3 notes `nbf` is not set and leeway is 10 s); guest-network client isolation (V3).

**S3. Gateway auth hardening.** Items: maximum token lifetime; `jti` denylist with operator-only revoke route; close `/api/events` at token expiry; counters for 401/403 by reason; per-source limiter for invalid tokens; strict-roles flag for SEC-M7-09 (aliases and bare tokens off), on by default in the production profile.
- Why: SEC-M7-09 and -11; only way to revoke one device.
- Builds on: `authenticate_request`, Lab stream's expiry check, `lab_api.md` precedent.
- Change: `src/security/jwt.rs`, `src/api/server.rs`, `src/config.rs`, `src/main.rs`; wiring of `kid` rotation optional (`main.rs:414`).
- Depends on: owner disposition D-SEC; independent review.
- Tested: each item has a test that fails without it (revoked `jti` gets 401 on the next request; long-lifetime token refused; stream ends at `exp`; flood counted and limited without blocking other sources; alias refused in strict mode while default mode keeps `sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09` behaviour).
- Evidence: failing-before and passing-after logs, plus the existing auth matrix green.
- Could fail: denylist memory growth; revocation not propagated across nodes (per-node only unless P1 provides a channel); limiter bypass by spoofed source or proxy; breaking existing clients that use aliases.

**S4. Control/data plane separation and control audit.**
- Why: guests on the LAN must not reach control or Lab; operator actions need an audit trail.
- Builds on: Lab opt-in per process; `/api/control` role check; Lab `events.jsonl` discipline.
- Change: no-code interim = two processes; code = optional control bind address, hash-chained audit file.
- Depends on: S3 for `jti` fingerprints.
- Tested: from a non-loopback source the control address is unreachable; audit chain verifier detects an edited line.
- Evidence: a transcript of refused connections; verifier output.
- Could fail: operators still expose the control port; audit file edited with the chain head not anchored (truncation undetected).

**S5. Cluster PKI tooling and signed membership revocation.**
- Why: the mTLS design cannot be demonstrated or operated without certificates and a way to remove a member.
- Builds on: `ClusterMembershipRegistry::update`, revocation channel, session tracker, `ring` Ed25519.
- Change: generator tool (dependency decision D-PKI); a caller of `update()`; router tolerance for member changes (P1).
- Depends on: P1 membership design; D-PKI.
- Tested: revoked member's next frame on a reused connection refused (extends `test_p3c_per_request_revocation_on_reused_connection` to the binary path); rollback of `version` refused.
- Evidence: a multi-process run where a member is revoked without restarting the others.
- Could fail: epoch mismatch handling drops healthy connections (A2 note); private keys left on the generator; dependency or build cost.

**S6. Peer authorization.** See section 6.
- Why: insider blast radius is the whole cluster.
- Builds on: `node.rs:499-559`, member records.
- Change: `src/dist/node.rs`, `src/security/cluster.rs`, `src/dist/frame.rs` (typed rejection if needed).
- Depends on: P1 decision on ledger escalation; S5.
- Tested/evidence: section 6.2.
- Could fail: legitimate escalation refused so debts never clear; false refusals during membership change.

**S7. Tenant registry and accounting hook (Stage A).** See 4.2-4.3. Depends on D-TENANT. Frozen-core change: independent review required.

**S8. Quotas and fair eviction (Stage B).** See 4.4. Depends on S7, an independent review pass, a release-build benchmark. Could fail: wrapper cost under the shard lock; overshoot from per-shard enforcement; dead-byte accounting; interaction with policy switching.

**S9. Origin share, bypass, DELETE and request limits, HTTP hardening.** See 4.5.
- Why: SEC-M7-11 family and S-09 amplification.
- Builds on: origin permit gate, per-key tokens.
- Change: `src/api/server.rs`, executor permit accounting, `src/config.rs`.
- Depends on: S7 for tenant identity of the registry.
- Tested: T-BF, T-AF, T-DF and slow-send/connection-cap probes.
- Evidence: failing-before/passing-after, loopback release runs with intervals.
- Could fail: limiter state per node only; legitimate bursts rejected; proxy source addresses.

**S10. Tenant-scoped telemetry tiers.** See 4.7. Depends on S7, S3 (new role). Could fail: side channel through own-eviction counts; cost on the read path (the read path must stay lock-free for hits; benchmark).

**S11. Evidence integrity and build provenance.**
- Why: evidence must be tamper-evident and say which code ran; the tree hash is a constant today.
- Builds on: `ring` SHA-256 (`manifest.rs:157-164`), `sha256_file` in `eval.rs`, `BUILD_COMMIT` stamp (`config.rs:23`), packaging scripts, `scripts/cleanroom_build.ps1`.
- Change: (1) stop hard-coding: read compile-time stamps and compute the running binary's SHA-256 at run start; if a stamp is `"unknown"` the manifest says unknown and the run is not VALID; (2) extend `packaging/build-release.*` to stamp a source digest and a dirty flag (shell and PowerShell hashing, no Rust dependency; `build.rs` would need a build-dependency, so avoid); (3) bundle format and verifier (below); (4) Ed25519 signing via `ring`.
- Depends on: owner approval for the verifier bin; key-custody decision D-KEY.
- Tested: a tampering corpus: flip a byte, delete a line, reorder lines, truncate the tail, swap the manifest, replace the public key. Each is detected except truncation without an anchored head count; that exception is stated in the verifier output. Verifier written by someone other than the bundle writer.
- Evidence: verifier exit codes on the corpus; a bundle whose `git_tree_hash`/binary digest match an independently computed digest.
- Could fail: an embedded public key proves only internal consistency; dirty-tree digest differs by line endings or untracked files; clock values in logs are untrusted; the same author writes tool and tests.

Bundle contents (minimum): schema version; build identity (commit stamp, source digest or "unknown", dirty flag, running-binary SHA-256, `Cargo.lock` SHA-256, rustc and profile, target); environment (OS, hardware, evidence class: loopback, LAN, VM, simulated); effective redacted configuration and its hash; scenario specification and hash; non-sealed seeds only; topology/session object; token inventory (claims only, `jti` fingerprint, never tokens); event log with hash chain, head and count; counters before and after for each node; origin's own request log; redacted transcripts; test names and results with commit; per-metric provenance labels (measured, simulated, inferred, unavailable; the manifest already has `metric_provenance`, `manifest.rs:112`); the limitations statement; manifest of file hashes; signature, signer key fingerprint; tool versions. Not stated by any bundle: that fixtures were honest, that the author is independent, or that wall-clock times are trustworthy.

**S12. Falsification suite and security evidence runner.**
- Why: tests assert pass/fail but emit no artifact; demos need a runner (V4 section 3).
- Builds on: `ClusterHarness`, `lab_live` gateway and origin helpers, existing mock origins.
- Change: tests and runner code only; no gateway change. Harness extension to run JWT-secured ingress with the multi-process cluster (V4 section 0.4).
- Depends on: S11 for output format.
- Tested: the runner's own tests; each scenario runs at least 5 times with seeds recorded.
- Evidence: bundles, hash-chained, labelled with environment class.
- Could fail: fixtures that make attacks trivially fail; same-author blind spots; results quoted without labels.

## 10. Dependency-ordered stages

| Stage | Content | Source change? | Needs |
|---|---|---|---|
| 0 | S1 minter and per-session profile (script); two-process layout (Lab on loopback only); evidence-bundle layout using existing hashing tools; read-only inventory of which retained manifests used the hard-coded constructor | no | none beyond normal approvals for new files |
| 1 | Falsification probes against the **current** tree: T-NN, T-BF, T-AF, T-DF, SSE stream count, slow-send, denied-read counting; harness extension for JWT plus cluster | tests only | D-TEST (owner approval to add test code and the runner) |
| 2 | S3 auth hardening, S11 build identity (stamp, binary digest), S9 HTTP hardening | yes, auth path | D-SEC, independent review |
| 3 | S7 accounting hook; S6 peer authorization; S5 PKI and revocation | yes | D-TENANT, P1 agreement, D-PKI; independent review (fencing/epoch adjacent) |
| 4 | S8 quotas and fair eviction; S9 origin share and bypass limits; S10 telemetry tiers | yes, frozen-core adjacent | independent review, release benchmarks |
| 5 | S2 productised pairing; verifier and signing; third-party tooling (dependency advisory scan, TLS scan, fuzzing, penetration test) | yes / tooling | owner approval of each tool; QR decision D-QR |

Dependencies: S2 needs S1; S3 precedes S2's revocation story; S6 needs S5 for a trustworthy member set and P1 for escalation semantics; S8 needs S7; S10 needs S7 and S3; S12 needs S11 for artifacts; all evidence claims need Stage 1 baselines before Stage 2-4 changes so improvements are measured against a recorded "before".

## 11. Risks, unresolved questions, owner decisions

Risks:
- R1: the tenant-aware eviction work touches the most invariant-heavy module (CLAUDE.md section 5); mistakes can break byte accounting or fencing. Mitigation: Stage A first, independent review, failing-before tests, property tests.
- R2: quotas that are per node can be mistaken for cluster guarantees. Mitigation: label on every screen.
- R3: security demos can be read as certification. Mitigation: the statement in section 8, "what is not proven" on every demo.
- R4: HS256 shared secret means one leaked secret is total compromise of that domain. Mitigation: per-session or per-node secrets; asymmetric signing only after owner decision.
- R5: guest tokens are bearer tokens with no device binding.
- R6: all existing evidence is debug, loopback, single host, Windows.
- R7: interactions with P1 (durable invalidation, membership) and P2 (state per tenant) are not yet reconciled with their files.

Unresolved (no answer assumed):
- U-1 Whether anti-entropy replay discloses other tenants' key names (needs reading `node.rs:578-640` and `anti_entropy.rs`).
- U-2 Whether the timing/occupancy channel across tenants exists at a measurable size; no measurement exists.
- U-3 Whether `jsonwebtoken` 9 EdDSA is usable without a new dependency (from memory only).
- U-4 Whether `ring` 0.17 exposes the Ed25519 sign and verify calls the design assumes (V4 says yes; I did not read the crate).
- U-5 Header-size limits and timeouts in the HTTP server builder (`main.rs`, server construction) not checked.
- U-6 The overshoot bound of a per-shard-enforced global cap (4.4 item 5).
- U-7 Whether a tenant-scoped `Observer` is wanted at all (V4 open question 7).
- U-8 Whether externally invited attacks are in scope.
- U-9 How retained manifests carrying the hard-coded tree hash should be annotated without editing frozen records.
- U-10 Which files are formally frozen (V3 D14 also unresolved).

Owner decisions (none presumed approved):

| ID | Decision | Options and my recommendation |
|---|---|---|
| D-SEC-09 | SEC-M7-09 disposition | (a) accept and document; (b) strict-roles flag, default on in production profile; (c) remove aliases (breaking). RECOMMENDATION (b) |
| D-SEC-10 | SEC-M7-10 disposition | add the missing ranges with tests, or accept. RECOMMENDATION add, low effort, but `src/security` hardening is owner-reserved |
| D-SEC-11 | SEC-M7-11 disposition | split: counters and per-source limiter; `jti` denylist; no IP lockout. RECOMMENDATION implement the first two (S3) |
| D-TEST | Add falsification test code and the evidence runner (S12) | tests only, no new dependency |
| D-TENANT | Authorize the tenant registry and accounting hook, then quotas | Stage A first; frozen-core change with independent review |
| D-PKI | Cluster certificate tooling | promote `rcgen` to a tool dependency, or extend the openssl script, or Windows-only PowerShell |
| D-TLS | LAN trust bootstrap | install a demo CA, pin a fingerprint by human comparison, or plain HTTP with banner on a private network |
| D-KEY | Evidence signing key custody | per-session ephemeral key plus an out-of-band published release key, or HMAC (integrity to the key holder only) |
| D-QR | QR support | no QR, in-house encoder, or a new crate |
| D-CTRL | Separate control listener | code change in `server.rs`/`main.rs` |
| D-FROZEN | Which files and records are frozen | list needed before any code |
| D-SYS | Firewall rules, trust-store changes, software installs | system changes; need approval and a cleanup record |
| D-TOOL | Dependency advisory scan, TLS scan, fuzzing, penetration test | each needs approval before running |

## 12. Requests to other workstreams

**To P1 (distributed):**
1. Decide whether the repair ledger's escalation to `Tenant` and `FullCache` stays command-style (needs sender capability) or becomes receiver-driven degrade; section 6 depends on it.
2. Membership change and revocation: share one design with S5 (signed, versioned document; router tolerance for a changed member set; tolerant window for `membership_epoch`).
3. If peer-fill or owner forwarding is planned, say so now: peer reads then need per-tenant authorization on the wire, quota charging on the storing node, and per-member rate limits.
4. A channel for propagating token revocation across nodes, or an explicit statement that revocation is per node.
5. State that quotas and origin ceilings are per node unless P1 designs coordination; any cluster view must say so.
6. The durable invalidation log will hold canonical keys (tenant and path): at-rest file permissions and redaction rules.
7. Cluster-wide telemetry must follow the privacy tiers (4.7).
8. Clock skew handling across nodes for token expiry (10 s leeway only).

**To P2 (intelligence):**
1. All state keyed by registered tenant with an explicit count and byte bound; no shared per-key sketches; global top-k operator-only.
2. Warm and calendar actions are billed to the target tenant's quotas and origin ceiling, dropped (not queued) when saturated, use the same credential policy as a client, never touch `BYPASS-AUTH` keys.
3. Write paths (calendar, warm, policy) exist only in secure mode, scoped to the verified tenant, behind a role distinct from Observer.
4. Intelligence may not change quota values, hard caps, or policy in a way that breaks quota semantics; policy switches must preserve per-tenant structures (4.4).
5. Audit and forecast endpoints must be tenant-scoped or aggregate-only; keep the field-set test pattern.
6. Abuse detectors (rate, unique-key rate, bypass rate, auth failure) are observe-only; enforcement is the deterministic limits in S9.
7. Labelled synthetic attack traces must be tagged simulated; no detection-rate claim from synthetic data.

**To P4 (Lab, UX, design):**
1. Guest credential UI: show claims, role, tenant, expiry and a countdown; never show tokens or codes after use; no tokens in URLs or persistent storage; screen-share privacy mode.
2. Presenter approval screen for pairing (device label, source address, confirmation number) and a visible roster with per-device state.
3. Show the mode chip: secure or INSECURE BASELINE, TLS or LAN-INSECURE, loopback or LAN, build profile.
4. A revoke button must say what it does: "denylist active on node X only" or "expires at T", never "revoked everywhere".
5. Labels on every value: measured, simulated, inferred, unavailable; `NODE_FAILURE` shown as logical; quota bars only when quotas exist.
6. An evidence viewer with verify status (verified against trusted head, unverified, unsigned, truncated-possible) and the limitations statement.
7. Telemetry views follow the tiers in 4.7; a guest sees Tier A or B only.
8. Dangerous-action confirmation list from section 7.

## 12b. Replies to what `04_lab_ux_design.md` asks of P3 (read after I finished the main text)

P4 (sections 2.7, 2.8, 3.9) asks P3 to own the scope-claim model, the view ticket and the key fingerprint statement. Provisional answers, all DESIGNED BUT NOT IMPLEMENTED:

1. **Scope claims** `session_id`, `classes`, `privacy`, `tenants[]`: FACT `JwtClaims` ignores unknown claims (`jwt.rs:30-42`), so these can ride in tokens today but **the gateway will not enforce them**. Enforcement exists only where new code reads them: the proposed session stream service (P4's own writer) or a future gateway change under S3/S10. A tenant-sliced Observer therefore cannot be claimed as isolation until that code exists and is tested (P4's test (c) in its scenario, "a stream token scoped to `beta` cannot read `alpha` events", is the right falsifier and would currently have nothing to test).
2. **View ticket** (P4 3.9): compatible with S2. Properties I require: single use, 120 s, redeemed over TLS or under the LAN-INSECURE banner, **presenter approval per redemption**, mints a read-only token of at most 10 minutes, never operator, bearer-only (not device-bound). Revocation is expiry or a per-node denylist (S3); say which.
3. **Key fingerprint:** FACT-PRIOR V4 (`diagnostics.rs:152-177`): per-process random seed, 32-bit. A 32-bit fingerprint is a short pseudonym, not a privacy control: collisions are likely at large key counts and a guessable key space can be tested by anyone who can compute the fingerprint, which requires the seed, so it is safe only while the seed stays inside the process. I did not read the file; the claim is prior-agent only.
4. P4's `tenant_quota` event class cannot be produced until Stage A (S7) exists; it should show "unavailable", not zero.

## 13. Closing statement

Today the product has real authentication, role checks, tenant namespace isolation, SSRF filtering and an mTLS cluster, each with tests written mostly by its own authors. It has no per-tenant resource isolation, no revocation, no auth-failure limiting, no peer-scope authorization, no token or certificate tooling, and an evidence manifest whose tree identity is a constant. The proposals above are designs; none is approved, built or tested, and the demonstrations they would enable are demonstrations, not certification.
