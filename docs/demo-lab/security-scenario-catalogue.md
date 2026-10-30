> Notice (2026-10-30): This document was written on 2026-10-26, before milestones M3 to M8. It is a planning and research record, not a description of the current system, and its phase numbers are not the project's current milestones (see [docs/STATUS.md](../STATUS.md)). Statements about current capability, test counts or missing features may be stale; the specific stale points known to the 2026-10-30 documentation audit are listed below where they apply.
>
> A `phantom-verify` verifier tool, where mentioned here, is a proposal: it does not exist in `src/` or `Cargo.toml`.
>
> Original text below is unchanged.

# PhantomCache — Controlled Cybersecurity Scenario Catalogue

## 1. Scenario Framework & Safety Principles

All cybersecurity demonstrations in this catalogue follow strict isolation principles:
1. **Isolated Mock Upstreams Only:** Security tests interact exclusively with controlled, local mock origin fixtures (`tests/common/mock_origin.rs`). Zero requests are sent to public or unapproved networks.
2. **Defensive Default Runtime:** Vulnerable behaviors (e.g., accepting unkeyed headers or disabled tenant checks) are **never introduced into the default production code**. Where contrast testing is needed, an isolated test harness configures a dedicated insecure test fixture.
3. **Reproducible Workloads:** Every scenario uses deterministic seeds, explicit key distributions, and fixed request counts.
4. **Verifiable Assertions:** Pass/Fail criteria rely on machine-verifiable HTTP status codes, response headers, cryptographic checks, and audit log hash verification.

---

## 2. Detailed Scenario Specifications (Scenarios 1 – 10)

### Scenario 1 — Authorized vs. Rogue Proxy (Certificate Mismatch & Rejection)
* **Threat & Objective:** An attacker deploys a rogue proxy on the LAN (e.g., ARP spoofing port 8085) to intercept credentials or serve poisoned cache items. Prove that the client rejects the unauthorized proxy before transmitting application payloads.
* **Preconditions & Trust Assumptions:** Requester trusts `Demo CA Root`. Authorized PhantomCache presents a certificate for `cache.lab.internal` signed by `Demo CA Root`. Rogue proxy presents a self-signed or mismatched certificate (`evil.internal`).
* **Topology:** Topology C or F (Requester, Authorized Cache, Rogue Proxy, Origin).
* **Controlled Workload:** Requester sends `GET https://cache.lab.internal:8085/api/v1/cache/auth-token` to:
  1. Authorized PhantomCache endpoint.
  2. Rogue Proxy endpoint (`https://192.168.1.99:8085/api/v1/cache/auth-token`).
* **Expected Defensive Behavior:**
  - Connection to Authorized Cache succeeds over TLS 1.3 with valid SNI.
  - Connection to Rogue Proxy fails at the TLS handshake boundary (`CERT_UNTRUSTED` or `HOSTNAME_MISMATCH`). The client TCP socket is terminated immediately before sending HTTP request bytes.
* **Observable Telemetry:**
  - Client logs: `SSL_connect: Certificate verify failed: self-signed certificate in certificate chain`. Zero application bytes transmitted.
  - Rogue Proxy logs: `Handshake failed; 0 HTTP requests received`.
* **Pass/Fail Criteria:**
  - **PASS:** Ingress connection terminated at TLS layer; zero application data leaked to rogue endpoint.
  - **FAIL:** Client silently accepts insecure cert or bypasses verification.
* **Automated Regression Test:** `tests/security/test_sec_01_rogue_proxy_rejection.rs`.
* **Cleanup:** Stop rogue listener process.
* **Known Limitations:** Does not prevent attack if client user manually bypasses browser TLS warning (`--insecure`).

---

### Scenario 2 — Cache-Key Confusion & Poisoning Resistance
* **Threat & Objective:** An attacker attempts to poison the cache for legitimate users by injecting unkeyed query parameters, malformed URI encodings, or custom headers (e.g., `X-Forwarded-Host: evil.com`) into a cold request. Prove that PhantomCache either incorporates relevant parameters into the cache key or strips untrusted hop-by-hop headers.
* **Preconditions:** Mock origin reflects `Host` and query parameters in synthetic responses.
* **Topology:** Topology A (Requester, PhantomCache, Mock Origin).
* **Controlled Workload:**
  1. Attacker sends `GET /api/v1/cache/item?sort=asc&evil_param=payload` with header `X-Forwarded-Host: evil.com`.
  2. Legitimate requester sends clean `GET /api/v1/cache/item?sort=asc`.
* **Expected Defensive Behavior:**
  - PhantomCache canonicalizes the query string (`sort=asc&evil_param=payload` != `sort=asc`).
  - Cache miss occurs for the clean request. The clean requester receives authentic origin content, not the attacker's reflected payload.
  - Hop-by-hop headers (`X-Forwarded-*`) are stripped before constructing upstream origin requests.
* **Observable Telemetry:** Observer logs two distinct `CacheMiss` events with different key fingerprints (`fp_attacker` vs `fp_victim`). Zero poison reuse.
* **Pass/Fail Criteria:**
  - **PASS:** Legitimate user receives unaffected origin response with `X-Cache-Status: MISS` (then cached cleanly).
  - **FAIL:** Legitimate user receives attacker-tainted response (`X-Cache-Status: HIT` with `evil.com`).
* **Automated Regression Test:** `tests/integration/m1_http_tests.rs::test_m1_t19_uncacheable_headers_and_bypass`.
* **Cleanup:** Invalidate `/api/v1/cache/item`.

---

### Scenario 3 — Cross-Tenant Isolation & Forged Tenant Header Rejection
* **Threat & Objective:** Tenant Alpha attempts to access or invalidate cached sensitive data belonging to Tenant Beta by passing forged headers (`x-tenant-id: beta`). Prove that tenant identity is cryptographically bound and responses never bleed across tenant boundaries.
* **Preconditions:** Mock origin returns `{"tenant": "alpha", "balance": 100}` for Alpha and `{"tenant": "beta", "balance": 50000}` for Beta at identical path `/account/balance`.
* **Topology:** Topology D (Two Requesters, PhantomCache, Origin, Observer).
* **Controlled Workload:**
  1. Authenticated Tenant Beta fetches `/account/balance` -> Cached.
  2. Attacker (Tenant Alpha) presents Alpha credentials but injects header `x-tenant-id: beta` requesting `/account/balance`.
* **Expected Defensive Behavior:**
  - Gateway validates Alpha's JWT token, establishes authenticated tenant as `alpha`, and overwrites/discards the spoofed `x-tenant-id: beta` header.
  - Gateway queries cache key `alpha:/account/balance`. Result is a cold fetch for Alpha's account. Alpha receives balance `100`, never Beta's `50000`.
* **Observable Telemetry:** Diagnostic log records: `SecurityWarning: Ingress header x-tenant-id mismatch with authenticated token claim; header overridden to alpha`.
* **Pass/Fail Criteria:**
  - **PASS:** Zero leakage of Tenant Beta's payload to Tenant Alpha.
  - **FAIL:** Tenant Alpha receives balance `50000` from cache.
* **Automated Regression Test:** `tests/integration/m1_http_tests.rs::test_m1_t18_cross_tenant_isolation`.
* **Cleanup:** Purge `/account/balance` for both tenants.

---

### Scenario 4 — Unauthorized Upstream Destination & SSRF Mitigation
* **Threat & Objective:** An attacker attempts to exploit PhantomCache as an SSRF proxy to scan internal networks, access loopback services (`127.0.0.1:22`), or steal cloud credentials (`http://169.254.169.254/latest/meta-data/`).
* **Preconditions:** PhantomCache upstream allowlist is configured with `allowed_origins = ["https://origin.lab.internal:9000"]`.
* **Topology:** Topology F (Attacker, PhantomCache, Mock Origin, Dummy Loopback Service).
* **Controlled Workload:** Attacker requests:
  1. `GET /api/v1/cache/metadata` with forged destination `http://169.254.169.254/latest/meta-data/`.
  2. `GET /api/v1/cache/admin` with origin configured to redirect (`HTTP 302 Location: http://127.0.0.1:22`).
* **Expected Defensive Behavior:**
  - Gateway validates destination before opening socket. Disallowed host/IP is immediately rejected with HTTP 403 Forbidden (`X-Cache-Error: UpstreamDestinationDisallowed`).
  - Outbound redirects to private/loopback IPs are blocked. Zero TCP packets leave to `169.254.169.254` or loopback ports.
* **Observable Telemetry:** Error counter `ssrf_rejections_total` increments. Diagnostic event: `OriginFetchBlocked: IP 169.254.169.254 in restricted link-local range`.
* **Pass/Fail Criteria:**
  - **PASS:** HTTP 403 returned; zero socket connection attempts to forbidden addresses.
  - **FAIL:** PhantomCache connects to loopback or metadata IP.
* **Automated Regression Test:** `tests/security/test_sec_04_ssrf_mitigation.rs`.
* **Cleanup:** None.

---

### Scenario 5 — Cache-Bypass & Resource Exhaustion (Unique-Key DoS)
* **Threat & Objective:** An attacker attempts to trigger out-of-memory (OOM) crashes by flooding PhantomCache with 200,000 unique keys while setting `X-Cache-Bypass: true`.
* **Preconditions:** Shard capacity configured to 10,000 entries max.
* **Topology:** Topology A (Load Generator, PhantomCache, Origin, Observer).
* **Controlled Workload:** 200,000 unique `GET /api/v1/cache/rand_<uuid>` requests sent at 5,000 QPS.
* **Expected Defensive Behavior:**
  - Cache entries rise to 10,000, then PIE/SIEVE eviction actively purges cold victims on every miss.
  - Active operations in flight are protected from eviction.
  - Total cache entries never exceed the hard limit of 10,000.
  - Process memory remains strictly bounded; process does not crash or exhaust heap.
* **Observable Telemetry:** Dashboard displays steady hit/miss ratio, `evictions_total` rises smoothly to ~190,000, `active_operations` remains bounded, process RSS memory remains flat.
* **Pass/Fail Criteria:**
  - **PASS:** Process survives 200,000 requests without crash; shard size <= 10,000 entries.
  - **FAIL:** OOM crash, panics, or shard size exceeds configured limit.
* **Automated Regression Test:** `tests/unit/wp1_tests.rs::test_core_200k_unique_key_flood_strict_capacity`.
* **Cleanup:** Reset cache counters.

---

### Scenario 6 — Coalescing & Origin Stampede Suppression Under Failure
* **Threat & Objective:** 200 concurrent clients request the exact same cold key simultaneously while the origin backend is experiencing a 500ms delay or intermittent failure. Prove single-flight coalescing and zero orphaned waiters.
* **Preconditions:** Mock origin configured with 500ms artificial latency.
* **Topology:** Topology D (Multiple Requesters, PhantomCache, Delayed Origin).
* **Controlled Workload:** 200 simultaneous concurrent asynchronous requests for `GET /api/v1/cache/heavy-report`.
* **Expected Defensive Behavior:**
  - Exactly 1 request initiates an upstream TCP fetch (`FetchRegistrationResult::Initiate`).
  - Remaining 199 requests register as waiters (`FetchRegistrationResult::Wait`).
  - Upon origin return, all 200 clients receive identical 200 OK responses.
  - Origin access log confirms exactly 1 request was received.
* **Observable Telemetry:** `cache_misses = 1`, `coalesced_waiters = 199`, `origin_requests = 1`. Observer logs `CoalesceBatchDeliver: count=199`.
* **Pass/Fail Criteria:**
  - **PASS:** Origin QPS == 1; all 200 clients resolve successfully within 520ms.
  - **FAIL:** Stampede occurs (origin QPS > 1) or waiters deadlock/hang.
* **Automated Regression Test:** `tests/integration/m1_http_tests.rs::test_m1_t13_barrier_stampede_100_clients`.
* **Cleanup:** Purge `/heavy-report`.

---

### Scenario 7 — Invalidation & Stale Response Elimination
* **Threat & Objective:** Data on the origin changes; an attacker attempts to exploit race conditions between cache invalidation (`DELETE`) and concurrent reads (`GET`) to leave stale data in cache.
* **Preconditions:** Key `/inventory/product_9` exists with value `v1`. Origin updates value to `v2`.
* **Topology:** Topology A or E (Requester, PhantomCache Node, Origin).
* **Controlled Workload:** Concurrent interleaved execution:
  - Client 1 sends `DELETE /api/v1/cache/inventory/product_9`.
  - Client 2 sends `GET /api/v1/cache/inventory/product_9`.
* **Expected Defensive Behavior:**
  - Generation tracking in `CacheShard` increments. Invalidation marks old entry stale immediately.
  - Concurrent `GET` either joins the newer fetch generation or fetches fresh `v2` from origin.
  - Never returns `v1` after invalidation acknowledgement.
* **Observable Telemetry:** SSE stream logs `CacheInvalidated` followed by `OriginFetchSuccess (v2)`.
* **Pass/Fail Criteria:**
  - **PASS:** Subsequent GET strictly returns `v2`.
  - **FAIL:** Stale `v1` served after DELETE acknowledged.
* **Automated Regression Test:** `tests/integration/m1_http_tests.rs::test_m1_t04_invalidation_race_key_exists`.
* **Cleanup:** Purge key.

---

### Scenario 8 — Telemetry Authorization, Observer Reconnection & Tamper Detection
* **Threat & Objective:** An unauthorized client attempts to trigger cluster resets via `/api/control`. An attacker modifies a saved demonstration JSON report to inflate performance numbers.
* **Preconditions:** Observer has read-only token; attacker has zero token.
* **Topology:** Topology C (Observer, PhantomCache, CLI Verifier).
* **Controlled Workload:**
  1. Attacker sends `POST /api/control` with `{ "event": "NODE_FAILURE" }` without auth header.
  2. Evaluator modifies `metrics.cache_hits` in `evidence-report.json` and runs `phantom-verify`.
* **Expected Defensive Behavior:**
  - `/api/control` rejects request with HTTP 401 Unauthorized.
  - `phantom-verify` detects hash chain mismatch and Ed25519 signature failure:
    `[SECURITY ALERT] Demonstration evidence signature verification failed! Digest mismatch at event 42.`
* **Observable Telemetry:** `auth_rejections_total` increments; audit log records security alert.
* **Pass/Fail Criteria:**
  - **PASS:** HTTP 401 on unauthenticated control; `phantom-verify` catches modified report.
  - **FAIL:** Control executed unauthenticated; verifier accepts altered report.
* **Automated Regression Test:** `tests/security/test_sec_08_evidence_tamper_detection.rs`.
* **Cleanup:** Restore clean evidence file.

---

### Scenario 9 — Cache Timing Side-Channel Quantification
* **Threat & Objective:** An attacker on the local network measures response latency distributions (0.1ms for cache hit vs 15ms for origin miss) to deduce whether a target victim has requested a sensitive URL.
* **Preconditions:** Requester queries `/user/confidential_document`.
* **Topology:** Topology A (Requester/Attacker, PhantomCache, Origin with 20ms fixed delay).
* **Controlled Workload:** Attacker sends 100 requests measuring high-resolution TCP socket round-trip time (RTT).
* **Expected Defensive Behavior & Analysis:**
  - The experiment quantifies the measurable latency differential (HIT: mean 0.42ms, stddev 0.08ms; MISS: mean 20.8ms, stddev 1.1ms).
  - Defensible Architecture: Documents that timing side-channels are an inherent property of caching. Verifies that resources configured with `Cache-Control: private` or `no-store` are never stored in public shards, preventing cross-user timing inference.
* **Observable Telemetry:** Latency percentile histogram in UI clearly differentiates hit/miss clusters.
* **Pass/Fail Criteria:**
  - **PASS:** Quantified leakage documented; private/sensitive routes proven uncacheable.
  - **FAIL:** Uncacheable private routes stored in public cache.
* **Automated Regression Test:** `tests/security/test_sec_09_timing_side_channel_analysis.rs`.
* **Cleanup:** None.

---

### Scenario 10 — Distributed-Node Authentication, Rogue Node Rejection & Failover
* **Threat & Objective:** A rogue machine on the LAN attempts to join the rendezvous cache cluster over TCP. An authorized node crashes abruptly. Prove unauthorized node is rejected, and traffic safely fails over to secondary replicas.
* **Preconditions:** Nodes 1, 2, 3 have cluster mTLS certificates. Rogue Node 4 has an invalid certificate.
* **Topology:** Topology E (Cluster Nodes 1..3, Rogue Node 4, Origin).
* **Controlled Workload:**
  1. Rogue Node 4 attempts TCP connection to Node 1.
  2. Authorized Node 2 TCP listener is terminated abruptly (`stop_tcp_service()`).
  3. Client requests key hashed to Node 2.
* **Expected Defensive Behavior:**
  - Node 1 rejects Node 4 during mTLS handshake (`UnknownClusterCA`).
  - Node 1 detects socket failure connecting to Node 2 (`ConnectionRefused`), marks Node 2 as `NodeState::Suspect`, and immediately fails over to replica Node 3 over TCP.
  - Client receives valid response without application error.
* **Observable Telemetry:** Telemetry logs: `ClusterAuthFailure: Peer 192.168.1.99 rejected`. UI node status updates: `Node 2: OFFLINE`, `Failover: Node 3 Active`.
* **Pass/Fail Criteria:**
  - **PASS:** Rogue node rejected; client request transparently served by replica Node 3.
  - **FAIL:** Rogue node accepted into ring; client receives HTTP 500/502 on Node 2 failure.
* **Automated Regression Test:** `tests/integration/p2_dist_network_tests.rs::test_p2_actual_tcp_connection_refused_socket_failover`.
* **Cleanup:** Restart Node 2.

