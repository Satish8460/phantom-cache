# V4: Security demonstration catalogue and distributed-failure scenarios

Status: RESEARCH ONLY (product discovery, agent V4 of 5). Read-only investigation of the working tree (HEAD `456bb25` plus uncommitted tree, 2026-10-30). No code, test or config was changed; no attack was executed; no test was run. Every behavioural statement below is tagged:

- **[code]** verified by reading `file:line` / a named function in this session.
- **[test-name]** a test with that name exists (read by name and sometimes body); it was NOT re-run by me.
- **[doc]** reported by a prior document or agent, not re-verified.
- **[assumption] / [hypothesis]** unverified; a probe is proposed.
- **[lit-V] / [lit-NV]** literature verified from a primary/secondary source in this session / cited from memory, not verified.

This file is not a security certification, and the demos it proposes are not one (section 9).

## 0. Findings that change how security demos must be framed

1. **The user's suggested 8-test matrix (`docs/security/security-demo-matrix.md`) is a design document, not a record of working demos.** `tests/security/` does not exist [code: directory listing]. `phantom-verify` does not exist (`src/bin/` holds only `phantom-lab.rs`; grep of `src scripts packaging Cargo.toml` finds no `phantom-verify`). No Ed25519 code exists in `src/` (grep `Ed25519|signature::` empty). The error names in the matrix (`UpstreamDestinationDisallowed`, `OriginTlsHandshakeFailed`) do not exist; the real ones are `X-Cache-Error: SsrfBlocked` / `UpstreamTlsFailed` / `InvalidOrigin` (`src/api/server.rs:2273-2290`). Test 3's attack procedure (a client chooses the upstream target) does not match the architecture: the origin is one static `ORIGIN_URL`; clients choose only tenant-scoped path and query.
2. **Isolation in the core cache is by key construction, not by separate stores.** One `PartitionedCache` with global shards; the key is `tenant:path[:?query]` (`CacheKey::to_canonical_string`, `src/core/cache.rs:96`). The tenant comes only from the verified JWT in secure mode (`server.rs:1209-1219`, 1964-1974). There is **no per-tenant byte/entry quota** (grep `quota|per_tenant` in `cache.rs`, `coalesce.rs`, `server.rs` found nothing relevant), so isolation is of confidentiality/integrity, not of capacity.
3. **Development mode (no `PHANTOM_JWT_SECRET`) is anonymous and trusts `x-tenant-id`** (`server.rs:1215`, 1970). Any security demo must state which mode; the secure-mode claims do not hold in dev mode. [code]
4. **The cluster integration harness runs the HTTP side without JWT** (`grep JWT tests/support/cluster_harness.rs` empty). So "secure ingress + mTLS cluster + failure injection" is not covered end to end by a multi-process test [code/assumption]. Secure single-node e2e tests exist (`m7_sec_round2_tests::e2e_*`).
5. **M3 invalidation durability defect is reproduced** (`docs/research/m3_durability/reproduction_report.md`; scratch copy only, nothing integrated): a DELETE answered 204 whose async fan-out has not reached a peer is lost if the issuer is killed or gracefully stopped; the peer stays stale until entry expiry. This directly limits any "revoke / right-to-erasure" demo (section 6).
6. `POST /api/control` with `NODE_FAILURE` flips the **state of node id 2 in `state.nodes`** (`server.rs:1089-1091`); in the mTLS cluster path `nodes` (the same `Arc<VirtualNode>`s) is passed to both `RendezvousRouter::new(nodes.clone())` (`main.rs:374`) and the API state (`main.rs:430`), so in cluster mode this toggle probably alters real routing state, not a process. [code for the sharing; behaviour not tested] It is a logical injection and must be labelled so; in dev mode it is anonymous.

## 1. Threat model scoped to the actual architecture

### 1.1 Assets, actors, boundaries

Assets: per-tenant cached response bodies (confidentiality, integrity), freshness/invalidations (integrity of "deleted means gone"), origin capacity, gateway CPU/memory, JWT secret and cluster private keys, telemetry/audit evidence.

Actors: anonymous network client; authenticated tenant client (token with `cache:read` and/or `cache:invalidate`, tenant claim); observer/operator/admin tokens; malicious or compromised origin; malicious/compromised cluster member (holds a valid cluster cert); local OS user on the gateway host; operator error.

Boundaries (`docs/security/trust-boundary-map.md`, partly aspirational): client to gateway (HS256 JWT, optional TLS 1.3); gateway to origin (allowlist, SSRF filter, TLS verify); gateway to peer (mTLS 1.3, SAN identity, membership registry); control plane (`/api/control`, Lab API); telemetry (SSE/metrics/diagnostics/intelligence).

### 1.2 Applicability table

| Threat class | Applies? | How it maps to this system | Existing control (evidence) | Gap |
|---|---|---|---|---|
| Tenant isolation | Yes, central | Namespacing by key prefix; tenant from token only; tenant charset `[A-Za-z0-9._-]`, 1..128 bytes, so first `:` is unambiguous (`cache.rs:38-51`); tenant-less and `__default__` tokens refused (403) (`server.rs:1052-1068`) | `m7_sec_fixes_tests::sec01_tenant_boundary_and_anonymous_cannot_read_authenticated_entries`; `security_phase1_tests::test_phase1_at05_and_at06_tenant_cryptographic_isolation`; `m7_sec_round2_tests::m1_tenantless_and_reserved_tenant_tokens_are_403...`; `security_phase4a_invalidation_tests::test_p4a_tenant_invalidation_isolation` | Shared capacity (no quota, eviction pressure); dev mode trusts header |
| Authn / authz / credential lifecycle | Yes | HS256 JWT, iss/aud/exp, kid rotation, roles (`jwt.rs`); order 401 -> 403 role -> 403 tenant -> 400 | `m7_sec_round3_tests::b_check_order_is_401_then_403_role_then_403_tenant_then_400`; phase1 tests | SEC-M7-09 (alias `requester` = read+invalidate; bare token without `Bearer ` accepted, `server.rs:991-993`); SEC-M7-11 (no revocation, no lockout, `iat` unused, no `nbf`); single shared symmetric secret (anyone who can mint can mint any tenant); rotation = restart |
| Cache poisoning (unkeyed inputs) | Yes, structurally reduced | Gateway forwards only gateway-controlled headers (fixed `Accept`, key-derived `x-tenant-id`, validated `x-request-id`, optional `Authorization`, stored validators) (`server.rs:2534` `fetch_origin_raw`; doc `docs/user/security.md` "Origin contract"). Any `Vary` other than `Accept-Encoding` makes the response uncacheable (`freshness.rs:237-247`). Client conditional/range headers stripped (`server.rs:1225`) | `m7_sec_round3_tests::m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant`, `m1_cross_user_coalescing_one_origin_fetch_and_no_header_leak`, `m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit` | Poisoning via a compromised/misbehaving origin is out of the gateway's power (it caches what the origin says); query-param order/case not canonicalised (fragmentation, not confusion) |
| Key confusion | Yes | Canonical string collisions: tenant `:` rejected; `:?` in path rejected so `from_canonical(to_canonical(k)) == k` (`cache.rs:80`); dot-segments rejected; control chars/space rejected; `//x` aliases `/x`, `///x` is distinct (documented) | `canonical_key_roundtrip_tests` (+ proptest regressions file), `m7_sec_round3_tests::l1_*`, `d_leading_double_slash...`, `m7_sec_round2_tests::h1_*` | Percent-encoding equivalence beyond decoded form (`%2f` vs `/`): gateway decodes once via axum `Path`; origin-side normalisation differences are not modelled (hypothesis, probe in S-06) |
| Cache deception / path normalisation | Partly | No extension-based "static file" heuristics; storage decided by origin headers; but a 200 **without any freshness headers is cached for the 60 s default** (`freshness.rs:273-276`) and the client identity is not forwarded, so within a tenant all principals share one variant by design | Same as poisoning row; `security_phase1_tests::test_phase1_at08_rfc9111_*` | Deception is possible only if the origin serves per-user content on a path without `Cache-Control: private/no-store` AND does not rely on forwarded identity; with `PHANTOM_FORWARD_UPSTREAM_AUTH=true` authenticated reads are `BYPASS-AUTH` (never stored). The origin contract is a deployment precondition, not an enforced property |
| Cross-tenant leakage | Yes | (a) content: prevented by key prefix; (b) coalescing: coalescer is per canonical key hence per tenant; (c) diagnostics fingerprints: per-process random seed, 32-bit (`diagnostics.rs:152-177`); (d) telemetry/audit endpoints readable by any Observer regardless of tenant | sec01/sec02 tests; `m7_sec_fixes_tests::sec02_status_exposes_only_the_documented_fields` | Observers see cross-tenant aggregate metrics by design; metrics are not tenant-scoped (assumption: verify `build_schema_v2_payload` has no per-tenant labels) |
| Timing / side channels | Weakly | `X-Cache-Status: HIT/MISS` and `Age` are shown to the requester; same-tenant only. Cross-tenant channel would have to be via shared capacity/eviction or shared global shard locks | None specific | Not tested; eviction/occupancy channel across tenants is a hypothesis (S-11). JWT signature comparison delegated to `jsonwebtoken` |
| Resource exhaustion | Yes | 512 KiB body limit (`server.rs:220`); key length caps (path 2048, query 4096, canonical 4096); `max_waiters_per_key` 1000 (`cache.rs:331`); origin concurrency cap (`origin_cap_tests`); response size limit; bounded repair ledger (1024 entries/256 KiB per peer); frame cap 1 MiB; 10,000 tenant epochs | `origin_cap_tests::*`, `memory_bound_soak_tests`, `coalesce_tradeoffs_tests`, `security_phase3b_framing_tests::*oversized*` | No per-tenant or per-IP rate limit; no connection cap evident in `server.rs` (grep `Semaphore|max_connections` found only origin permits); unique-key flood evicts others (S-10) |
| Stampede / amplification abuse | Yes | Single-flight coalescing (one origin fetch per key); `x-cache-bypass: true` sends the request straight to the origin, **not coalesced** (`server.rs:1243-1276`, only permit-gated); unique-query flood = N origin fetches | `m7_sec_fixes_tests::sec01_cache_bypass_header_does_not_skip_authentication`; `origin_cap_tests::cap_never_exceeded_by_concurrent_distinct_misses`; `coalesce_tradeoffs_tests` | Bypass is available to every `cache:read` principal (S-09). Cap protects the origin but converts overload into 503 for everyone (shared fate) |
| Invalidation authorization / integrity | Yes, central | HTTP: `DELETE /api/v1/cache/*` needs `cache:invalidate` + tenant claim; scope is **ExactKey of the caller's own tenant only** (`server.rs:1954-1982`). Tenant and FullCache scopes exist only on the cluster wire / repair ledger (`router.rs` 1920, 1978-1983; `node.rs:547-557`). Peer frames: originator id must equal the mTLS identity (`node.rs:505-511`) | `security_phase4a_invalidation_tests::*`, `cluster_followup_tests::test_l3_invalidate_request_originator_must_match_tls_identity`, `m7_sec_fixes_tests::*` | **Any authenticated cluster member can send Tenant or FullCache scope** (`node.rs:547-557`, no per-scope authorization) [code]: a compromised member can purge every tenant on every peer. In dev mode DELETE is anonymous for any tenant |
| Malicious / compromised client | Yes | Token theft = full tenant access until expiry (no revocation) | exp validation | SEC-M7-11 |
| Node identity and inter-node comms | Yes | mTLS 1.3 only, cluster CA only, strict SAN `node-<id>.<cluster>.phantom.internal`, fingerprint registry, re-validation per request on pooled connections (`node.rs:393-406`), registry epoch | `security_phase3_cluster_tests::*` (at01..at22, sec01..sec08), `security_phase3c_pooling_tests::test_p3c_per_request_revocation_on_reused_connection` | Membership read once at startup (revocation needs restart, `m7_security_review.md` 2.4); no CRL/OCSP; a valid member is fully trusted (see above); cluster GET path returns fabricated `val_node_N` bodies and caches them under `__default__` for keys not starting with `/` (`node.rs:439-458`; pinned by `known_gaps::pin_kg6_node_get_request_fabricates_missing_key_body`) |
| Secrets / config / audit | Yes | Secret redaction in config report/Debug/errors (canary test); strict boolean parsing; secret length >= 32 B; production profile refuses insecure overrides | `m7_config_tests` (`secrets_never_appear_in_report_debug_errors_or_warnings`), `m7_sec_fixes_tests` | Canary not tested across logs, metrics, SSE (M7 review section 3); secret in process environment; denied reads not counted; the audit trail is an in-memory ring (`DiagnosticBuffer`, bounded, volatile) and `/api/v1/intelligence/audit` holds 100 records: neither is a tamper-evident audit log |
| Abuse / anomaly detection | Not implemented | Counters exist (requests, bypasses, errors, `capacity_rejections`); no per-principal accounting; 401/403 not counted | n/a | See section 5 |
| Control plane / Lab API | Yes | `/api/control` needs `scenario:operate` in secure mode; Lab API (opt-in `PHANTOM_LAB_ENABLED`) spawns gateway child processes; dev-mode loopback + Host/Origin checks | `docs/user/lab_api.md`, `tests/integration/lab_api/review_tests.rs` | `/api/control` parses JSON before auth (documented); independent review of Lab API pending |
| Supply chain / provenance | Out of product scope for runtime, in scope for demos | `packaging/verify-checksums.sh` checks `SHA256SUMS` only | none signed | No signing, SBOM or advisory scan (`limitations.md`) |
| Request smuggling / response splitting at origin | Yes | Upstream target is percent-encoded ASCII; origin response framing validated (CL/TE conflicts, chunking, truncation); redirects not followed | `security_phase2_upstream_tests::test_p2_conflicting_*`, `test_redteam_*`, `m7_sec_round2_tests::h1_*` | Only the gateway's own parser is tested |
| HTTP hop/loop (gateway-to-gateway forwarding) | **Not applicable** | There is no HTTP forwarding between gateways; peers exchange binary frames only; GET is served from the local cache only (`limitations.md`). Request loops cannot occur at the HTTP layer | n/a | Re-evaluate if cross-gateway read-through is ever added |
| SSRF driven by a client | **Not applicable as an attack from the data plane** | The upstream host is configuration (`ORIGIN_URL`), not client input | SSRF filter still protects against hostile DNS answers and mis-configuration: `test_p2_ssrf_*`, `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses` | SEC-M7-10 unblocked special ranges |
| Browser-side attacks (XSS, CSRF) | Largely N/A for data plane | JSON/cache gateway; auth is a bearer header (not cookie); CORS allowlist, no credentials | `sec03_*` | UI (`ui/`) was not reviewed (M7 review) |
| Multi-region / WAN threats | N/A | Single-host loopback tested only | n/a | Out of scope |

## 2. Scenario catalogue

Format per scenario: topology/config; attacker action; expected defence; actual behaviour (verified or "unverified + probe"); cache and origin events; before/after state; evidence retained; proves; does NOT prove; status.

Status key: **E** Existing capability demonstrable today (an existing test or binary does it; a presenter only needs a reproducible runner and evidence capture); **P** Partially; **X** Needs extension; **U** Unsupported.

Common conventions: secure mode = `PHANTOM_JWT_SECRET` (>=32 B), `--mode production` where possible, origin = deterministic mock (as in `tests/integration/m7_sec_*`), loopback only, fixed token set (tenant A reader, tenant B reader, A invalidator, observer, operator). All attacks are against local fixtures the demo itself starts. Evidence retention for every scenario follows section 7.

### Part A. The user's suggested set (mapped to reality)

**A1 (matrix Test 1) Authorized endpoint vs rogue proxy (untrusted certificate)**
- Topology: gateway with ingress TLS 1.3 (`axum-server` rustls, `docs/user/run.md`); client trusting only the demo CA; a second listener with a self-signed cert.
- Attack: client sends `GET /api/v1/cache/...` with `Authorization` to both.
- Expected: TLS abort at the rogue before any HTTP bytes.
- Actual: the client-side behaviour is the client library's (reqwest/rustls). The gateway-side checks that exist: `security_phase1_tests::test_phase1_at01_valid_tls_connection_succeeds`, `test_phase1_at02_untrusted_expired_or_mismatched_tls_fails`. A rogue-proxy listener is a test fixture, not product code. The matrix's regression file `tests/security/test_sec_01_rogue_proxy_rejection.rs` does not exist [code].
- Events: no cache or origin events (handshake fails). Evidence: client error text + rogue listener byte counter = 0 (must be built).
- Proves: a correctly configured client refuses an untrusted server certificate. Does NOT prove: anything about PhantomCache's own code (it is the TLS library's behaviour); nothing about clients that disable verification.
- Status: **P** (phase1 at01/at02 cover the cert-validation matrix; the "0 bytes of token sent" assertion needs a small raw-TCP rogue listener, no new dependency).

**A2 (Test 2) Tampered release artifact / configuration**
- Actual: `phantom-verify` and signed manifest do not exist. What exists: `packaging/verify-checksums.sh` (SHA256SUMS check), `scripts/cleanroom_build.ps1`, `src/lab/eval.rs::sha256_file`, lab manifests with SHA-256 (`src/lab/manifest.rs:156-161`, `ring`). **Caveat [code]: `src/lab/manifest.rs:103` hard-codes `git_tree_hash` to `456bb25...`, so that manifest field is not evidence of the build's real tree.**
- Expected demo: flip one byte, `sha256sum -c` fails. That demonstrates only integrity against accidental/naive change, because anyone who can alter the binary can alter SHA256SUMS unless the sums file is signed or stored elsewhere.
- Proves: tamper detection relative to a trusted digest. Does NOT prove: authenticity/provenance (needs a signature with an out-of-band trusted key: see section 7, Ed25519 via the existing `ring` dependency).
- Status: **P** (checksum part E; signed-manifest verifier **X**).

**A3 (Test 3) SSRF and unauthorized upstream**
- Re-framing: a client cannot name an upstream, so the honest demos are (i) startup refusal of a bad `ORIGIN_URL`/allowlist, (ii) DNS answers pointing at metadata/private ranges refused at fetch time, (iii) blocked ports, (iv) redirects not followed, (v) credentials not forwarded.
- Actual: 502 `X-Cache-Error: SsrfBlocked` (policy/SSRF), 400 `InvalidOrigin` (bad scheme/port) (`server.rs:2273-2282`). Tests: `test_p2_ssrf_cloud_metadata_blocked`, `test_p2_ssrf_ipv4_mapped_ipv6_blocked`, `test_p2_ssrf_mixed_dns_blocked`, `test_p2_hard_blocked_ports_unconditional`, `test_p2_redirect_not_followed`, `test_p2_credential_forwarding_policy`, `test_redteam_*`, `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses`.
- Events: origin sees nothing; cache stores nothing (`InvalidOrigin`/`SsrfBlocked` are errors, not stored). Evidence: status + header + zero accepted connections on a canary listener at the blocked address.
- Proves: documented ranges/ports are refused, including mapped/6to4/NAT64 forms. Does NOT prove: DNS-rebinding resistance beyond "one resolved address is pinned for the socket" (reading only, M7 review 2.3); ranges in SEC-M7-10 (198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24, Teredo) are allowed; HTTP(S)_PROXY behaviour.
- Status: **E** (with the re-framing); the matrix's literal procedure is **U**.

**A4 (Test 4) Cross-tenant leakage and auth bypass**
- Topology: secure mode, origin returns per-tenant content keyed by the gateway's `x-tenant-id` (the only identity it receives). Tokens: A and B readers.
- Attack: (1) anonymous GET; (2) A reads `/account`; B reads `/account`; (3) A sends `x-tenant-id: B`; (4) A with a tenant-less token; (5) token with tenant `__default__`.
- Expected/actual [test-name]: (1) 401 `WWW-Authenticate: Bearer`, zero origin requests, unchanged counters (`sec01_every_credential_failure_is_401_with_challenge_and_causes_no_side_effects`); (2)-(3) header ignored, key is `A:/account`, B has a separate entry (`sec01_tenant_boundary_and_anonymous_cannot_read_authenticated_entries`, `m7_secure_mode_ignores_spoofed_tenant_header`, phase1 at05/06); (4)-(5) 403 `MissingTenant`/`ReservedTenant` (`m1_tenantless_and_reserved_tenant_tokens_are_403_for_cache_read_and_invalidate`). The matrix's expected "Alice's response is uncacheable per RFC 9111 3.5" is no longer the default: since round-2 M2, with `PHANTOM_FORWARD_UPSTREAM_AUTH=false` authenticated reads are cached per tenant if the response is shareable (`docs/user/security.md`); set it true to see `BYPASS-AUTH`.
- Cache events: HIT/MISS per tenant key; origin: one fetch per tenant. Evidence: transcript of requests/responses, origin request log, `X-Cache-Status`, plus per-key `audit_bytes` if exposed.
- Proves: with the stated origin contract (origin varies only on `x-tenant-id`), tenants A and B never receive each other's bodies, and the claim cannot be spoofed by header. Does NOT prove: isolation of capacity (S-10), absence of side channels, or isolation when the origin personalises on something else; two users of the same tenant share one variant by design.
- Status: **E** (the contrast run in dev mode, where `x-tenant-id` IS trusted, is also available and should be shown, clearly labelled as the insecure baseline).

**A5 (Test 5) Unauthenticated/unauthorized control plane**
- Actual [test-name]: `test_phase1_at03_and_at07_*` (401), `test_phase1_at04_insufficient_role_returns_403_and_no_mutation`, `m7_http_rbac_matrix_in_secure_mode`, `low_events_stream_is_observer_gated`, `low_embedder_case_auth_for_control_disabled_still_validates_presented_tokens`. `/api/control` requires `scenario:operate` (`server.rs:1070-1081`). The matrix's "audit log records SecurityWarning: Unauthorized access attempt" is **not implemented** (denied requests are not counted; no such event) [code, M7 review]. Observation of "cluster remains Active" can use `/api/v1/diagnostics/*` or metrics.
- Proves: unauthenticated/under-privileged callers cannot change state (zero-side-effect assertions exist for cache routes). Does NOT prove: that an Operator cannot do harm (an operator can mark node 2 Failed, section 0.6); rate limiting (none).
- Status: **E** for 401/403 and no mutation; the audit-log expectation is **U**.

**A6 (Test 6) Origin TLS validation failure**
- Actual [test-name]: `test_p2_untrusted_ca_rejected`, `test_p2_expired_cert_rejected`, `test_p2_san_mismatch_rejected`. Gateway answers 502 `X-Cache-Error: UpstreamTlsFailed` (`server.rs:2278-2283`), nothing cached.
- Proves: unverifiable origins are refused and not cached. Does NOT prove: protection when `PHANTOM_ENFORCE_UPSTREAM_TLS=false`/plaintext upstream allowed (documented override); certificate revocation checking.
- Status: **E**.

**A7 (Test 7) Altered or incomplete evidence detection**
- Actual: no Ed25519 signing, no hash-chained event log, no `phantom-verify` [code]. Lab runs write `manifest.json` with SHA-256 manifest hash and per-file hashes in places (`src/lab/manifest.rs`, `scenario_b.rs` `sha256_file`, `source_sha256`); the sealed M6 directory has a manifest. These detect modification only against a trusted copy of the manifest hash.
- Needs: hash-chained JSONL event log + signed head, verifier tool. Design in section 7. Detects: edit of a value, deletion/reorder of an event (chain break), truncation only if the final head hash/count is anchored elsewhere (otherwise a truncated tail is undetectable).
- Status: **X** (all-existing-dependencies design, `ring` provides SHA-256, HMAC and Ed25519).

**A8 (Test 8) Node authentication and distributed failover**
- Actual [test-name]: rogue-node rejection `test_p3_at02_untrusted_ca_rejection`, `at05_cross_cluster_connection_rejection`, `at06_revoked_node_rejection`, `at07_unknown_node_id_rejection`, `at08_certificate_fingerprint_mismatch`, `at09_plaintext_tcp_connect_rejection`; failover `p2_dist_network_tests::test_p2_actual_tcp_connection_refused_socket_failover` and `security_phase4b_router_tests::test_p4b_primary_unreachable_acting_primary_failover`. The matrix's "routes the request to replica in <2 ms, client gets 200 with zero errors" is a claim to be measured, not shown: GET is served from the local cache only (no read-through), so "failover" in this system is about invalidation routing and node-state bookkeeping, not about serving a client request from another node. See section 6.
- Status: **E** for node authentication; **P** for failover with that caveat.

### Part B. Additional scenarios (at least 6; here 14)

**S-05 Cache poisoning by unkeyed headers (X-Forwarded-Host, Cookie, Accept, custom X-)**
- Topology: secure or dev; origin echoes any received header into the body (the test origin `spawn_echo_origin` does this).
- Attack: attacker sends `X-Forwarded-Host: evil`, `Accept: text/evil`, `Cookie`, `x-user-id: admin`, then a victim requests the same URL.
- Defence/actual [test-name]: none of these reach the origin; victim sees the clean variant; `Set-Cookie` from the origin is not replayed (`m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant`, `m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit`).
- Evidence: origin-side received-header log (byte-exact), responses. Proves: the poisoning channel via client headers is closed by construction. Does NOT prove: safety against an origin that varies on something outside headers; poisoning via the URL itself (the attacker's own unique query creates its own key, not a shared one).
- Status: **E**.

**S-06 Key confusion and path-normalisation probes**
- Attack set: `%2e%2e`, `a/../b`, `%0d%0a`, `%20`, `%00`, NUL, `:?` in path, tenant containing `:`, 4097-byte canonical key, `//x` vs `/x` vs `///x`, `%252F`, mixed-case percent-escapes, query-parameter reorder.
- Actual [test-name]: `m7_sec_round2_tests::h1_*`, `m7_sec_round3_tests::l1_*`, `d_leading_double_slash_aliases_single_slash_but_triple_slash_is_a_distinct_key`, `canonical_key_roundtrip_tests` (proptest), `parser_robustness_tests`. Responses: 400 `InvalidPath` / `UriTooLong` (`server.rs:2244-2257`), origin sees nothing.
- Unverified (hypothesis, propose probe): (a) `%2f` vs `/` equivalence and mixed-case `%2F`/`%2f` producing two keys for one origin resource; (b) query reorder `?a=1&b=2` vs `?b=2&a=1` is two keys (fragmentation, an amplification aid, not confusion). Probe: for each variant, record key (via a diagnostics fingerprint equality, since fingerprints are deterministic within a process) and origin request count.
- Proves: no tested input produces a key that addresses another tenant or injects an upstream header. Does NOT prove: no divergence with a particular real origin's normaliser.
- Status: **E** for the listed tests; probes in (a),(b) **P**.

**S-07 Cache deception attempt**
- Topology: origin serves a personalised-looking page at `/account` with NO cache headers (misconfigured origin) and `Cache-Control: private` variant.
- Attack: victim fetches `/account/x.css` (or `/account;a.css`, `/account?.css`); attacker (same tenant) fetches the same URL.
- Expected/actual: [code] the 200 without headers is cached for the 60 s default (`freshness.rs:273-276`) and served to any principal of that tenant; with `private`/`no-store` it is uncacheable (`:205-211`). Authorization is not forwarded by default, so a correctly built origin cannot personalise anyway. Attack across tenants fails by key prefix.
- Proves: what the gateway enforces and what it leaves to the origin (the demo must show the misconfigured-origin case honestly: it IS cached). Does NOT prove protection against a misconfigured origin. The "static extension rule" mitigations of CDNs do not exist here.
- Status: **E** to demonstrate both outcomes (needs a small scripted origin, existing mock origins suffice). Literature: Gil, Web Cache Deception (Black Hat USA 2017) [lit-NV]; Mirheidari et al., "Cached and Confused", USENIX Security 2020 [lit-NV].

**S-08 Cache-poisoned denial of service (error caching)**
- Attack: malformed/oversized header or method override so the origin returns 400/5xx, hoping the gateway stores it for victims. Prior art: CPDoS (Nguyen et al., 2019) [lit-V as reported by SecurityWeek/The Hacker News/Security Affairs; primary paper not opened].
- Defence [code]: only 200, and 301/404/410 with explicit freshness, are cacheable (`freshness.rs:221-225`); 4xx/5xx are not stored; client headers are not forwarded so header-triggered origin errors are not reachable via the gateway. Truncated/close-delimited bodies not cached (`test_p2_truncated_response_not_cached`, `test_redteam_close_delimited_body_not_cached`). Caveat: an origin that returns `404` with `max-age` caches the 404 for that key; whether an attacker can induce that is origin-dependent.
- Evidence: second request still reaches the origin and gets the healthy answer. Proves: error responses are not shared. Does NOT prove: absence of stale-if-error amplification interactions (`stale_serving_tests::test_stale_if_error_on_origin_outage` shows stale served on origin outage by design, bounded by `PHANTOM_MAX_STALE_MS`, default 24 h).
- Status: **P** (the freshness decision is unit-testable now; an end-to-end 400-caching negative test is not in the list I read; propose adding).

**S-09 Origin amplification: `x-cache-bypass` and unique-key floods**
- Attack: a tenant reader sends N requests with `x-cache-bypass: true` for one hot key, and N unique `?cb=i` queries.
- Actual [code]: bypass is authenticated first (`sec01_cache_bypass_header_does_not_skip_authentication`) then goes straight to `fetch_origin_gated` (no coalescing, one origin request per client request) (`server.rs:1265-1276`); unique queries are N cold misses. Defence is the origin concurrency cap (`origin_cap_tests`), giving 503 `CapacityExceeded`/`OriginCapSaturated` + `Retry-After: 1` (`server.rs:2221-2223`) to everyone (shared fate, no per-principal fairness). Cap is "experimental" (`limitations.md`).
- Cache/origin events: origin request count vs client request count; peak in-flight vs cap (`wait_in_flight_zero` helpers). Evidence: origin request log + gateway counters (`cache_bypasses`, `capacity_rejections`).
- Proves: the cap bounds origin concurrency; the amplification factor of bypass is 1:1. Does NOT prove fairness, nor that a legitimate tenant is not starved by an abusive one. A policy that disables or rate-limits bypass per principal does not exist.
- Status: **P** (cap behaviour **E**; per-principal limiting **U**).

**S-10 Noisy-neighbour eviction (cross-tenant capacity interference)**
- Topology: small `PHANTOM_MAX_CACHE_BYTES`; tenant A warms a working set and measures hit ratio; tenant B (valid token) writes many unique large keys.
- Expected: no quota exists, so A's hit ratio drops (eviction is global per shard). [code absence of quota; behaviour is hypothesis] Probe: run with and without B's flood, N seeds, record A's hit ratio and origin load; release build; the harness `phantom-lab` live scenarios could host it.
- Defence that exists: byte accounting invariants (`byte_accounting_tests`, `memory_bound_soak_tests`) bound memory, not fairness.
- Proves (if run): quantifies the interference. Does NOT prove isolation; this scenario is a *demonstration of a limitation* and should be presented as such.
- Status: **X** (probe harness needed; no new dependency).

**S-11 Cross-tenant timing/occupancy side channel (hypothesis)**
- Idea: tenant B infers whether tenant A recently populated a shared shard/eviction structure by measuring B's own latency or eviction of B's canary keys. Requires shared locks or shared eviction order, which this design has (global shard mutex, eviction across tenants).
- Status: **U/X**: no claim either way; do not demo as secure. If studied: pre-register, measure with release build, many repetitions, and report null result honestly.

**S-12 Token abuse: expiry, wrong audience/issuer, kid confusion, alg confusion, raw-token acceptance**
- Actual [test-name]: `test_phase1_at03_and_at07_*`, `test_phase1_jwt_unknown_kid_rejected_without_fallback`, `test_phase1_jwt_key_rotation_support`, `m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`, `m7_secure_mode_...` `low_401_body_is_generic_and_does_not_echo_kid_or_parser_detail`, `sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09`.
- Honest negative demos: (i) stolen valid token works until `exp` (+10 s leeway); no revocation (SEC-M7-11); (ii) `requester` alias can both read and invalidate (SEC-M7-09); (iii) no lockout/rate limit on failed logins.
- Proves: forged/expired/mis-scoped tokens are refused. Does NOT prove: replay resistance, revocation, or clock-skew edge behaviour.
- Status: **E** for rejection; **U** for revocation/rate-limit (show as known gaps).

**S-13 Invalidation authorization: cross-tenant DELETE and unauthorized purge**
- Attack: A with `cache:read` only tries DELETE (403); A with `cache:invalidate` deletes `/x` (affects only `A:/x`); B's `/x` survives; dev mode: anonymous DELETE with `x-tenant-id: B` succeeds (contrast).
- Actual [test-name]: `test_phase1_cache_read_and_invalidate_roles_enforced`, `m1_tenantless_and_reserved_tenant_tokens_are_403_for_cache_read_and_invalidate`, `test_p4a_tenant_invalidation_isolation`. HTTP DELETE can address only ExactKey in the caller's tenant (`server.rs:1982`, `1997`).
- Proves: invalidation is tenant-scoped at the HTTP edge. Does NOT prove anything about a compromised cluster member (S-14), nor that 204 means removed everywhere (section 6).
- Status: **E**.

**S-14 Compromised cluster member (blast radius of a valid certificate)**
- Attack (designed, NOT to be executed against anything but a fixture): a fixture peer holding a valid member cert sends `InvalidateRequest` with `FullCache` or `Tenant` scope; `GetRequest` with synthetic keys.
- Actual [code]: accepted; originator must equal the TLS identity (`node.rs:505-511`) but scope is not authorized (`node.rs:547-557`). Existing tests exercise the scopes legitimately (`test_p4a_full_cache_invalidation_scope`, `cluster_followup_tests::fake_peer*`) and the originator check (`test_l3_invalidate_request_originator_must_match_tls_identity`). A non-member is refused at handshake (phase3 at02/05/06/07/08).
- Proves (as a limitation demo): membership = trust; mTLS prevents outsiders, not insiders. A revoked member is cut only after a restart (static registry) except where `p3c ... revocation` wiring applies (`test_p3c_sec03_automated_membership_revocation_wiring`, `test_p3c_per_request_revocation_on_reused_connection`; relationship to the startup-read registry must be checked).
- Status: **P** (legitimate-scope and originator-spoof tests E; "compromised member purges all tenants" is trivially true from code and needs an explicit fixture test if it will be demoed).

**S-15 Repair-ledger overflow as a cross-tenant blast radius**
- Mechanism [code]: while a peer is unreachable, DELETEs accumulate in a bounded per-peer ledger (1024 entries / 256 KiB); overflow collapses the largest tenant to one tenant-scope flush (max 16), and only then a `FullCache` flush that hits all tenants on that peer (`router.rs:53-60`, 124-135, 176-189).
- Tests: `cluster_followup_tests::test_m6_one_tenants_overflow_does_not_flush_other_tenants_on_peer`, `cluster_prewarm_repair_tests::test_ledger_overflow_collapses_to_tenant_flush`, `test_ledger_overflow_escalates_to_full_flush_when_not_collapsible`.
- Proves: one tenant's DELETE storm during a peer outage costs that tenant its entries first; global flush needs many distinct tenants/single-entry groups. Does NOT prove: that a cheap attacker with several tenant tokens cannot force a global flush (hypothesis; the escalation condition is in `collapse_largest_tenant`).
- Status: **E** (as correctness demo), attack-economics probe **X**.

**S-16 Resource-exhaustion and malformed-input robustness**
- Cases: >512 KiB body (413), 4097-byte canonical key (400/414-class), oversized cluster frame (rejected before allocation: `test_p3b_at11_oversized_payload_rejected_before_alloc`), bad magic/version, truncated frames, legacy ASCII downgrade attempt, malformed invalidation frame terminates socket, handshake capacity saturation fails closed (`node.rs:266`), pool capacity cap (`test_p3c_sec01_concurrency_capacity_cap_and_no_leak`).
- Proves: bounded handling of the listed malformed inputs. Does NOT prove: slow-loris/connection-count resistance on the HTTP port (no test; no connection limit evident), or CPU exhaustion by many valid requests. Idle CPU is already abnormally high (PERF-M7-01).
- Status: **E** (listed) / **U** (HTTP connection exhaustion).

**S-17 Observer data exposure**
- An Observer token reads `/api/v1/metrics`, diagnostics, intelligence audit (trigger reasons, candidate parameters, config hashes). Per-process random-seed key fingerprints appear in diagnostics, not keys.
- Proves: field set of `/intelligence/status` is pinned (`sec02_status_exposes_only_the_documented_fields`). Does NOT prove: no tenant data in metrics (verify), nor that Observer visibility is acceptable in a multi-tenant deployment (policy decision).
- Status: **P**.

### Catalogue summary

| ID | Scenario | Status |
|---|---|---|
| A1 | Rogue TLS endpoint | P |
| A2 | Tampered artifact | P (checksum) / X (signed) |
| A3 | SSRF / upstream policy | E (re-framed) |
| A4 | Cross-tenant leakage and auth bypass | E |
| A5 | Control-plane RBAC | E (audit event U) |
| A6 | Origin TLS failure | E |
| A7 | Evidence tamper detection | X |
| A8 | Node auth and failover | E / P |
| S-05 | Unkeyed-header poisoning | E |
| S-06 | Key confusion / normalisation | E, probes P |
| S-07 | Cache deception | E |
| S-08 | CPDoS / error caching | P |
| S-09 | Bypass and unique-key amplification | P |
| S-10 | Noisy-neighbour eviction | X |
| S-11 | Cross-tenant timing channel | U/X |
| S-12 | Token abuse | E / U |
| S-13 | Invalidation authz | E |
| S-14 | Compromised member | P |
| S-15 | Ledger overflow blast radius | E / X |
| S-16 | Resource/malformed input | E / U |
| S-17 | Observer exposure | P |

## 3. What "demonstrable today" requires beyond the tests existing

The tests above are in-process or spawned-process assertions. A presentable demo additionally needs a runner that: starts fixtures on loopback, executes the scenario with fixed tokens and seeds, captures the request/response transcript, the origin's own request log, gateway counters before/after, and writes an evidence bundle (section 7). That runner does not exist for security scenarios (the Lab API/`phantom-lab` exist for performance scenarios only). Tests assert pass/fail; they do not emit a self-contained artifact. Passing tests do not establish correctness (CLAUDE.md section 3).

## 4. Prior art (for threat-model grounding)

- Web cache poisoning (unkeyed inputs): Kettle, "Practical Web Cache Poisoning", PortSwigger Research, 2018, and "Web Cache Entanglement", 2020 [lit-NV: not retrieved this session].
- Web cache deception: Gil, Black Hat USA 2017 [lit-NV]; large-scale measurement: Mirheidari et al., "Cached and Confused: Web Cache Deception in the Wild", USENIX Security 2020 [lit-NV].
- CPDoS: Nguyen et al., "Your Cache Has Fallen: Cache-Poisoned Denial-of-Service Attack", ACM CCS 2019 [lit-V in secondary reporting only: https://www.securityweek.com/researchers-warn-new-cache-poisoned-dos-attack-method , https://thehackernews.com/2019/10/cdn-cache-poisoning-dos-attack.html ; the primary paper was not opened].
- RFC 9111 (HTTP Caching) section 3.5 (authenticated responses in shared caches) and 4.1 (`Vary`): the project's supported subset is in `docs/research/rfc9111_supported_subset.md` [doc].
- My search covered one web query; absence of other results is not evidence of absence.

## 5. Security-aware intelligence (Phantom Intelligence in a multi-tenant cache)

Current state: intelligence is advisory/experimental, not in the serving path (claims ledger C8), and no benefit is established (C1 NOT SUPPORTED, `limitations.md`). Anything below is design discussion, not capability.

### 5.1 How workload intelligence could complement security (without weakening isolation)

1. **Abuse and anomaly signals as observe-only, per-principal counters.** Candidate signals: per-token request rate, unique-key rate (cache-busting), bypass-header rate, 401/403 rate by source, origin-fetch share caused by a principal, DELETE rate per tenant. Output is an alert/evidence record, not an automatic cache policy change. The execution layer (hard caps, fencing) stays authoritative regardless of any decision (CLAUDE.md 6b).
2. **Enforcement stays deterministic.** If a response action is wanted (per-principal bypass limit, per-tenant token bucket, per-tenant byte quota), implement it as a plain configurable limit in the execution layer, with the intelligence layer at most *recommending* parameters inside operator-set bounds. This keeps the security property explainable and testable without the ML gate.
3. **Scope detectors to the data the actor already owns.** A detector for tenant A may use A's own request stream and aggregate global load; it must not consume B's per-key state.
4. **Detector evaluation needs labelled traffic.** Synthetic attack traces are acceptable only if labelled simulated; no claim of detection rate from synthetic data may be presented as measured (CLAUDE.md section 4). False-positive cost (blocking a flash crowd) must be measured against the existing flash-crowd scenarios.

### 5.2 Leak risks specific to predictive caching/warming across tenants

| Risk | Mechanism | Mitigation to require before any such feature |
|---|---|---|
| Forecast side channel | A shared popularity model (global frequency sketch, shared forecaster, shared shadow-eviction candidate selection) trained on all tenants' accesses reveals through its decisions (what gets warmed, what is admitted) that tenant A requested key K | Per-tenant models/sketches, or differentially-private/noisy aggregates; never share per-key statistics across tenants; hashes of keys are not a protection (key spaces are guessable) |
| Warm-state oracle | Prefetching key K for tenant A makes a later HIT for tenant A faster; if warming consumes a shared origin permit or evicts a shared entry, tenant B observes a latency/hit change | Warming must draw from the *warmed tenant's* budget, never evict other tenants' entries beyond their own quota; warm outside the request path |
| Cross-tenant warm inference through coalescing | Warming issues a single-flight fetch; a concurrent request of another principal joins it (same key = same tenant, so safe), but a shared global permit pool leaks load info | Per-tenant permit accounting |
| Threshold/PIE audit leakage | `/api/v1/intelligence/audit` shows trigger reasons, candidate parameters; derived from global aggregates it leaks cross-tenant load to every Observer | Observer scope per tenant, or aggregate-only fields; keep the pinned field-set test |
| Authorization-bound warming | Warming a key requires the origin to answer; warming with a privileged identity could populate a tenant namespace with content the requester could not fetch | Warm only with the same credentials policy as the requester path (currently the gateway forwards no client identity by default, a constraint to preserve); never warm `BYPASS-AUTH` keys |
| Poisoned training signal | An attacker shapes traffic to steer the controller (e.g., force an eviction policy that hurts a victim) | Rate-bounded, per-principal-capped influence on signals; hard caps unaffected; canary with revert already exists as machinery (C8, unverified) |
| Timing of the controller itself | Policy switches are visible as latency/hit changes to all tenants | Treat as a shared-fate channel; document; limit switch frequency (60 s hold exists in design) |
| Cluster warming | Pre-warming peers with another gateway's hot keys moves a tenant's content to nodes before an invalidation; combined with the M3 defect (section 6) increases stale exposure | Do not warm across nodes without durable invalidation |

## 6. Distributed-failure scenarios (honest support statement)

### 6.1 What the system actually does (all [code] unless stated)

- Placement/routing: rendezvous hashing; GET is served from the local cache only (no read-through, `limitations.md`). Routing matters for invalidation (canonical primary, acting primary) and for peer GETs between nodes.
- DELETE: purge local first (`server.rs:1990`), then synchronous leg to the canonical primary with a 150 ms deadline (`PHANTOM_CLUSTER_TIMEOUT_MS`, `router.rs:1774-1779`), failover to the first replica if the primary is unreachable (primary marked Suspect), then asynchronous fan-out to **all** other members (live ones via bounded broadcaster, non-live ones directly into the repair ledger) (`router.rs:1790-1886`). 204 means local purge + primary/acting-primary ack (`limitations.md`). Errors: 503 `PrimaryRateLimited`, 500 `PrimaryInternalError`/`ObsoleteInvalidation`, 504 `PrimaryInvalidationTimeout` (`server.rs:2016-2078`); on failure every non-local member is recorded as owed (`record_owed_to_all`).
- Repair/anti-entropy: a 250 ms worker re-delivers ledger entries and does pull reconciliation; Suspect -> Recovering after a drained ledger; Recovering -> Active only via successful pull reconciliation; Recovering/Suspect nodes bypass the cache on peer GETs (`node.rs:436-441`, `security_phase4c_antientropy_tests::test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits`). Pre-warm worker opens one pooled connection per member at start.
- Fencing: generation + epochs prevent an in-flight fetch from resurrecting a deleted/invalidated key (`invalidation_contract_tests::test_delete_fences_in_flight_fill_race`, `security_phase4a_invalidation_tests::test_p4a_inflight_fetch_fencing_prevents_stale_writes`).

### 6.2 Scenario table

| Scenario | What can be demonstrated for real | Test / harness | Honest limits |
|---|---|---|---|
| F1 Node offline (process dead / link black-holed) | Real gateway processes behind fault proxies (`FaultMode::{Normal, Delay, Blackhole, Partition, DropNewConnections}`, `tests/support/fault_proxy.rs`); node-state transitions Active -> Suspect -> Recovering -> Active observable via `node_states()` | `cluster_characterization_tests::test_s3_03_partition_and_acting_primary_failover`, `test_s3_08_suspect_stickiness_characterization`, `distributed_consistency_tests::test_m3_04a_suspect_to_active_automatic_recovery`, `cluster_followup_tests::test_m4_blackholed_peer_keeps_task_count_bounded_and_owes_ledger` | Loopback, 3-5 nodes, debug build; a black-holed link is a fault *injection*, not a real network partition. Clients of the dead node get no service (no client-side failover exists) |
| F2 In-flight requests when a node dies | Origin fetch in flight on the issuer: coalesced waiters get `InitiatorCancelled` on cancellation (`server.rs:1613`, `1907`); permits released on disconnect (`origin_cap_tests::permit_released_after_client_disconnect`) | `coalesce_tradeoffs_tests::test_mitigation_waiter_client_timeout_and_abort`, `origin_cap_tests` | No in-flight *connection draining* on shutdown (`limitations.md`); no test kills a gateway mid-GET and checks client-visible outcome |
| F3 Routing during failure | Acting-primary failover for invalidation; suspect primary excluded from routing and retried in broadcaster | `test_p4b_primary_unreachable_acting_primary_failover`, `test_p4b_sec01_suspect_primary_excluded_from_routing_and_retried_in_broadcaster`, `test_p4b_http_gateway_acting_primary_failover_header` (`X-Cache-Warning: PrimaryFailover(<id>)`) | Not a read-path failover (GET local only). `Failed`/`Offline` states are never auto-recovered by repair (`router.rs:1908`) |
| F4 Invalidation during failure | DELETE with an unreachable member still returns 204 if the primary acked; non-live members are owed in the ledger and repaired when reachable | `distributed_consistency_tests::test_h2_failed_delete_still_invalidates_all_peers_after_heal`, `cluster_followup_tests::test_h2_failed_delete_records_all_other_peers_as_owed`, `test_m10_failed_peer_is_owed_the_invalidation` | Guarantee holds **only while the issuing process stays alive** and within ledger bounds (see 6.3) |
| F5 Rejoin / recovery | Peer returns, ledger drains, then pull reconciliation promotes it; sequence-gap bookkeeping; restarted *receiver* still applies invalidations | `test_m8_drained_peer_is_only_recovering_until_pull_reconciliation_then_active`, `test_h1_restarted_sender_invalidations_are_still_applied_by_receiver`, `security_phase4c_antientropy_tests::test_p4c_offline_node_anti_entropy_catchup_and_replay_at_p4_06` | A restarted *issuer* has an empty ledger (M3); a *rejoining node's own cache is in memory and empty after restart*, so rejoin staleness is about nodes that did NOT restart but were partitioned |
| F6 Stale-data risk | Local linearizable purge (`test_m3_04a_local_linearizability_immediate_eviction`); stale serving bounded by hard cap (`stale_serving_tests::test_hard_staleness_cap_enforced_strictly`, `test_invalidation_delete_strictly_wins_over_stale`) | as listed | Freshness lifetime from origin has no gateway ceiling; fallback default 60 s is a literal; `PHANTOM_DEFAULT_TTL_MS` is not wired (CLAUDE.md 5). Propagation latency measured only on warm loopback, debug build: P99 about 10-20 ms, cold first use 35-117 ms (claims ledger C6 amendment); the old "50 ms" bound is not supported |

### 6.3 Where the M3 durability defect makes a demo misleading

- **Defect (reproduced in a scratch copy, 4 flag-off/original runs, 0 counter-examples):** DELETE returned 204 in about 0.6-0.9 ms; issuer killed (TerminateProcess) and restarted; the peer stayed stale for the whole 10 s window (89-91 polls); origin `max-age=120` implies about 2 minutes. Graceful stop loses it identically. The negative control (issuer alive) converged in 231-337 ms (`reproduction_report.md` sections 2-4). Not integrated; flag-off equals current behaviour.
- Demos that would mislead if shown without this caveat:
  1. "Revoke / delete content everywhere" (legal erasure, token-scoped data removal): a 204 followed by a gateway deployment or crash leaves a peer serving the old object until expiry. Do not demo as an erasure guarantee. Show the defect itself as a *limitation* scenario with the kill/restart harness (it exists only in the scratch copy, under `docs/research/m3_durability/`; the working tree's test harness has no kill/restart helper: `cluster_harness.rs` only has `shutdown`, `kill_on_drop`).
  2. Rolling-restart demos: restarting the issuer right after DELETEs is exactly the loss window; the operator guidance in `limitations.md` says to restart one gateway at a time and re-issue DELETEs.
  3. "Self-healing cluster" narratives: repair and anti-entropy genuinely recover from peer outages (F4/F5), but not from issuer loss. Show both, side by side.
  4. Any "zero stale reads after 50 ms" slide: not supported (C6 amendment).
  5. `POST /api/control {"event":"NODE_FAILURE"}` or dashboard buttons: logical state toggles on node id 2 (and synthetic UI data when `PHANTOM_SIMULATION=true`); they are simulated/injected, not physical failure. Label as such; never describe a simulated recovery as real recovery. The 3 demo nodes in the non-cluster path (`main.rs:207-209`) own separate caches and exist for the simulation.
- Combining mTLS cluster failure injection with JWT-secured ingress is untested end to end (section 0.4): a "secure cluster under failure" demo needs a harness extension (pass `PHANTOM_JWT_SECRET` and tokens to `ClusterHarness`), no new dependency.

### 6.4 Failure scenarios that have no support at all

Clock skew/NTP steps (freshness is wall-clock; `Date`/`Age` based; no clock-fault tests found), disk faults (cache is memory-only), process memory pressure/OOM on a peer, asymmetric partitions beyond proxy modes, cross-host networks (Windows loopback only), power loss, and churn above 5 nodes (replicas limited to primary + 2 for failover ranking; all other members get fan-out). Mark these Unsupported in any matrix.

## 7. Evidence integrity design (existing dependencies only)

### 7.1 What `Cargo.toml` already provides [code]

Runtime: `ring` 0.17 (SHA-256/384/512, HMAC, Ed25519/ECDSA verify and sign, constant-time compare, HKDF), `rustls`/`tokio-rustls`/`rustls-pemfile`, `jsonwebtoken` 9, `serde`/`serde_json`, `rand`, `tracing`. Dev: `rcgen` (cert generation), `proptest`, `hdrhistogram`, `reqwest`. Already used: `ring::digest` for lab manifests (`src/lab/manifest.rs:159`) and `sha256_file` (`src/lab/eval.rs`). **No new dependency is needed**; Ed25519 is in `ring::signature` (not used anywhere yet).

### 7.2 Design

1. **Evidence bundle** per scenario run, a directory: `events.jsonl` (append-only), `transcript/` (raw request and response bytes per step), `origin_log.jsonl` (the origin fixture's own record of what it received), `gateway_counters_before.json` / `after.json` (from `/api/v1/metrics`), `config_effective.txt` (the redacted effective-config report), `env.json` (OS, build profile, binary SHA-256, git HEAD, dirty-file list; compute from reality, not literals: see the `manifest.rs:103` hard-coded hash caveat), `manifest.json`.
2. **Hash chain** over `events.jsonl`: each line contains `seq`, `ts`, `kind`, `payload`, `prev = SHA256(previous line bytes)`; the chain head after the last event is recorded in the manifest. Verifier recomputes the chain: detects edit, reorder, interior deletion. Truncation is detected only if the final head and count are anchored elsewhere (printed on screen, committed in a separate trusted place, or signed).
3. **Manifest**: file name to SHA-256 for every file in the bundle, plus the chain head and event count; the manifest is hashed (as `compute_manifest_hash` already does).
4. **Authenticity (optional, closes the "attacker rewrites everything" hole)**: sign the manifest with Ed25519 (`ring::signature::Ed25519KeyPair`); the demo key is generated per session or per release and the public key is distributed out of band. Alternative for single-party demos: HMAC-SHA256 with a session key, which proves integrity to the key holder only. State explicitly which was used.
5. **Time**: timestamps are local clock values and are not trusted evidence; order is given by `seq` and the chain. For an external time anchor you would need a third party (not available).
6. **Pre-registration for any measured claim**: write the plan and its SHA-256 before running (the repo's preregistration pattern: `docs/research/preregistration_m6_v2_*.json` + `.pin`); note the earlier finding that the plaintext sealed seeds and same-timestamp artifacts could not show ordering (claims ledger section 4), so record the hash in a place whose timestamp is independent.
7. **Verifier** (`phantom-verify`-like, can be a `src/bin` addition or a script; needs owner approval as application code per CLAUDE.md section 2): recompute hashes, chain, signature; exit non-zero with the first divergent seq.
8. **Redaction**: bundles must not contain the JWT secret or tokens; tokens in transcripts must be redacted (store a SHA-256 of the token and the claims) before hashing, so the hash covers the redacted form actually shipped. Test with a canary secret as the config tests do.
9. **What a bundle cannot prove**: that the gateway under test was the code claimed (needs binary hash plus reproducible build; clean-room build script exists, `scripts/cleanroom_build.ps1`), that the fixtures were honest, or that tests of the same author are independent.

## 8. Validation plan (before any demo is presented)

1. **Inventory first, no new code**: for each E-status scenario, confirm the cited test exists and runs green on the current tree (`cargo test --test <file> -j 1`, per CLAUDE.md; this research did not run them). Record test name, pass/fail, commit, dirty-file list.
2. **Failing-before checks**: for each defence a demo advertises, identify the commit/test where it was a bug (M7 rounds 1-3 have failing-before logs: `D:\Temp\sec2_before.log`, `sec3_before.log`, ephemeral) and keep a "defence removed" negative control where cheap (dev mode as the contrast for tenant isolation; `PHANTOM_FORWARD_UPSTREAM_AUTH=true` vs false for caching of authenticated reads).
3. **Build the evidence runner** (section 7) as an opt-in tool; run each scenario at least 5 times; for non-deterministic ones (S-09, S-10, S-15, F-series) record seeds, release build, hardware/OS, workload, repetitions, intervals. Loopback, simulated and WSL2/physical results stay in separate columns.
4. **Independent review**: the SEC-M7-01/02/03 fixes are still marked "independent review pending"; round-2 and round-3 are described as independent reviews inside the M7 document, but the provenance of that independence is not established by this research. Any scenario resting on those fixes (A4, A5, S-05, S-13, S-17) carries that qualifier.
5. **Close or accept open items before claiming a posture**: SEC-M7-09, -10, -11 (owner disposition required); M3 durability (owner decision on the proposed patch); per-scope authorization for peer invalidation (design decision); per-tenant quota (design decision); no-rate-limit policy.
6. **Third-party tooling** (needs owner approval, not run here): `cargo audit`/`cargo deny` dependency advisories, a TLS configuration scan, a fuzzing pass on the HTTP/frame parsers (`parser_robustness_tests` and proptest exist but are not fuzzers), a multi-host cluster run.
7. **Falsification tests to add** (each should fail without the defence): (a) 400/5xx never stored (S-08); (b) tenant A eviction-interference bound with a quota (S-10, once a quota exists); (c) peer `FullCache` frame from a non-privileged member is rejected (if scope authorization is added); (d) secure-ingress + cluster + kill/restart end-to-end (F-series with JWT).

## 9. Demos are not security certification

Every scenario here shows that a specific control behaves as described under a specific fixture, mode, build profile and host. None of them establishes the absence of vulnerabilities, resistance to an adaptive attacker, correctness under real network conditions, or compliance with any standard. The authors of the code wrote most of the tests; passing tests do not establish research validity (CLAUDE.md section 3). Penetration testing, fuzzing, dependency scanning, TLS scanning, independent code review and multi-host testing have not been done (M7 review "Not done"). Presentations must (i) name the mode (dev vs secure), (ii) label results loopback/simulated/measured, (iii) show known-gap scenarios (S-09, S-10, S-12, S-14, F-series with M3) next to the successes, and (iv) never use the words "secure", "zero-trust" or "guaranteed" without the bound and the failure model.

## 10. Open questions for the owner

1. Is the target audience for the demo an operator/security reviewer (needs honest limitations and evidence bundles) or a sales/overview audience (needs a narrative)? This changes whether S-10/S-14/M3 are shown as limitations.
2. Approve building a security evidence runner and `phantom-verify` (application code/tooling, no new dependency, Ed25519 via `ring`)?
3. Approve a harness extension to run JWT-secured ingress with the multi-process cluster, and bringing the M3 kill/restart helper from the scratch copy into the test tree (test-only)?
4. Decide on: per-scope authorization for peer invalidation frames; per-tenant byte/entry quota; per-principal rate limit on `x-cache-bypass` and failed auth; counting denied reads in a metric.
5. Disposition for SEC-M7-09 (`requester` alias, raw-token acceptance), -10 (SSRF ranges), -11 (revocation, lockout).
6. Should the M3 durable log + shutdown drain be integrated before any "invalidation" demo? If not, the demo must carry the 6.3 caveats on screen.
7. Is a tenant-scoped Observer role wanted (so a multi-tenant deployment does not expose global metrics/audit to all observers)?
8. Is dev mode (anonymous, trusts `x-tenant-id`) ever to be shown to external audiences? If yes it should be labelled as the insecure baseline in every scenario.
9. Does the owner want `docs/security/security-demo-matrix.md` corrected (it describes non-existent test files, tools and error names) or superseded by this catalogue? (This task was not allowed to edit other files.)
