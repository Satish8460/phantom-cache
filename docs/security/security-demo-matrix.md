> Notice (2026-10-30): This matrix is design intent, not evidence. (1) The test paths it names (`tests/security/test_sec_*.rs`, `tests/common/mock_origin.rs`) do not exist: `tests/` holds only `integration`, `support` and `unit`. The implemented security tests are the Cargo targets `security_phase1_tests` to `security_phase4c_antientropy_tests`, `m7_security_review_tests` and `m7_sec_*` (`Cargo.toml`). (2) `phantom-verify`, `release-manifest.json` and `release.pub` appear nowhere in `src/` or `Cargo.toml`. (3) The error names `OriginTlsHandshakeFailed`, `CERT_UNTRUSTED` and `UpstreamDestinationDisallowed` are not in `src/`; the gateway answers with `X-Cache-Error` `InvalidOrigin` (400), `SsrfBlocked` (502) or `UpstreamTlsFailed` (502) (`src/api/server.rs:2262-2300`). (4) Every "Observed Evidence" block is illustrative; no run produced it.
>
> For security behaviour that is implemented and tested, see [docs/security/m7_security_review.md](m7_security_review.md) and [docs/user/security.md](../user/security.md).
>
> Original text below is unchanged.

# PhantomCache — Controlled Security Demonstration Matrix

## 1. Safety Principles for Security Testing

1. **Isolated Local Fixtures:** All tests execute against controlled local mock origins (`tests/common/mock_origin.rs`) and ephemeral ports. Zero requests escape to external networks.
2. **Defensive Default Runtime:** Insecure behaviors (such as accepting untrusted tenant headers) are never introduced into default production builds. Contrast testing is conducted using isolated test harnesses.
3. **Deterministic & Reproducible:** Every test uses explicit seeds, synthetic data, and machine-assertable pass/fail criteria.

---

## 2. Detailed Demonstration Test Specifications (Tests 1 – 8)

### Test 1 — Authorized Endpoint vs. Rogue Proxy (Untrusted Certificate)
* **Objective:** Verify that an application client aborts connection to an unauthorized rogue proxy presenting an untrusted certificate before transmitting application payloads.
* **Lab Setup:**
  - Authorized PhantomCache: Listens on `https://cache.lab.internal:8443` with cert signed by `Demo CA Root`.
  - Rogue Proxy: Listens on `https://192.168.1.99:8443` with a self-signed certificate (`CN=evil.internal`).
  - Client Trust Store: Contains only `Demo CA Root`.
* **Attack Procedure:** Client attempts to send `GET /api/v1/cache/secrets` with `Authorization: Bearer confidential_token` to:
  1. Authorized endpoint.
  2. Rogue proxy endpoint.
* **Expected Defensive Behavior:**
  - Authorized: TLS 1.3 handshake succeeds; request processed.
  - Rogue Proxy: Client TLS stack detects untrusted certificate authority (`CERT_UNTRUSTED`) and immediately aborts TCP connection. Zero bytes of HTTP payload or authorization tokens are transmitted.
* **Observed Evidence:** Client logs: `SSL_connect error: self-signed certificate in certificate chain`. Rogue proxy logs: `TLS handshake aborted; 0 HTTP bytes received`.
* **Cleanup:** Terminate rogue proxy process.
* **Automated Regression Test:** `tests/security/test_sec_01_rogue_proxy_rejection.rs`.

---

### Test 2 — Modified Release Artifact or Configuration Tampering
* **Objective:** Verify that an independent reviewer detects an unauthorized, tampered binary or modified configuration before deployment.
* **Lab Setup:**
  - Official release manifest (`release-manifest.json`) containing SHA256 digest of approved binary and signed with `release.pub`.
  - Tampered binary: 1 byte modified in binary `.rodata` section.
* **Attack Procedure:** Evaluator runs `phantom-verify`:
  ```bash
  phantom-verify --binary target/release/phantom-cache --manifest release-manifest.json --pubkey release.pub
  ```
* **Expected Defensive Behavior:** `phantom-verify` calculates the SHA256 of the binary on disk, compares it against the signed manifest digest, detects mismatch, and halts with non-zero exit code.
* **Observed Evidence:** Console output:
  ```text
  [CRITICAL SECURITY ALERT] Binary digest mismatch!
  Expected: 8f4b238a164c8d9e72f10b25d36e812f8e12470a12e8b5d381c8411b43d2e99a
  Calculated: 9a11c43d2e8b15d972f10b25d36e812f8e12470a12e8b5d381c8411b43d2e111
  STATUS: VERIFICATION REJECTED. DO NOT EXECUTE.
  ```
* **Cleanup:** Restore original binary.
* **Automated Regression Test:** `tests/security/test_sec_02_provenance_tamper_detection.rs`.

---

### Test 3 — Unauthorized Upstream Destination & SSRF Mitigation
* **Objective:** Verify that PhantomCache rejects attempts to query disallowed upstream hosts, cloud metadata endpoints, or loopback services.
* **Lab Setup:** PhantomCache configured with `allowed_origins = ["https://origin.lab.internal:9000"]`.
* **Attack Procedure:** Attacker submits requests targeting disallowed destinations:
  1. Cloud Metadata: `GET /api/v1/cache/meta` with upstream target `http://169.254.169.254/latest/meta-data/`.
  2. Localhost Admin: `GET /api/v1/cache/admin` with target `http://127.0.0.1:22`.
* **Expected Defensive Behavior:** The gateway evaluates the destination against the allowlist and IP blacklist before creating a socket. The fetch is rejected immediately with HTTP 403 Forbidden (`X-Cache-Error: UpstreamDestinationDisallowed`). Zero TCP SYN packets leave the host.
* **Observed Evidence:** HTTP response status `403 Forbidden`; error header `X-Cache-Error: UpstreamDestinationDisallowed`. Diagnostic event: `OriginFetchBlocked: Destination not allowlisted`.
* **Cleanup:** None.
* **Automated Regression Test:** `tests/security/test_sec_03_ssrf_mitigation.rs`.

---

### Test 4 — Cross-Tenant Cache Leakage & Authorization Bypass
* **Objective:** Verify that an unauthenticated user cannot access cached data fetched by an authenticated user, and that Tenant Alpha cannot access Tenant Beta's cached responses.
* **Lab Setup:**
  - Mock origin returns `{"user": "alice", "ssn": "000-12-3456"}` for Alice and `{"balance": 50000}` for Tenant Beta.
* **Attack Procedure:**
  1. User Alice fetches `/api/v1/cache/profile` with `Authorization: Bearer alice_token`.
  2. Unauthenticated attacker sends `GET /api/v1/cache/profile` without credentials.
  3. Attacker sends `GET /api/v1/cache/account` claiming `x-tenant-id: beta` while presenting Tenant Alpha credentials.
* **Expected Defensive Behavior:**
  - Alice's response is flagged with `Authorization` context. Under RFC 9111 §3.5, it is marked uncacheable for shared users.
  - The unauthenticated attacker receives HTTP 401 Unauthorized, never a cache hit of Alice's data.
  - The gateway ignores the spoofed `x-tenant-id: beta` header, validates Alpha's token, and keys the request under `alpha:/account`, completely isolating Beta's balance.
* **Observed Evidence:** Attacker receives HTTP 401 on private profile. Tenant Alpha receives `{"balance": 0}` (Alpha's account), never Beta's `50000`.
* **Cleanup:** Purge `/profile` and `/account`.
* **Automated Regression Test:** `tests/security/test_sec_04_tenant_and_auth_isolation.rs`.

---

### Test 5 — Unauthenticated Access to Privileged Controls
* **Objective:** Verify that `/api/control` strictly blocks unauthenticated or unauthorized users from mutating cluster state.
* **Lab Setup:** PhantomCache running with RBAC enabled.
* **Attack Procedure:**
  1. Attacker sends `POST /api/control` with `{ "event": "NODE_FAILURE" }` with no `Authorization` header.
  2. Attacker sends `POST /api/control` presenting an observer token (`role: observer`).
* **Expected Defensive Behavior:**
  - Request 1 is rejected with HTTP 401 Unauthorized.
  - Request 2 is rejected with HTTP 403 Forbidden (`InsufficientPermissions`).
  - Server state is not modified; cluster nodes remain `Active`.
* **Observed Evidence:** HTTP 401 / 403 returned. Cluster snapshot remains `Active`. Audit log records: `SecurityWarning: Unauthorized access attempt to /api/control`.
* **Cleanup:** None.
* **Automated Regression Test:** `tests/security/test_sec_05_control_plane_rbac.rs`.

---

### Test 6 — Origin TLS Validation Failure
* **Objective:** Verify that PhantomCache refuses to fetch responses from an origin presenting an expired, untrusted, or mismatched TLS certificate.
* **Lab Setup:** Mock origin configured with an expired TLS certificate (`EXPIRED_DATE = 2020-01-01`).
* **Attack Procedure:** Client requests a cold key `/api/v1/cache/secure-catalog`. PhantomCache initiates upstream TLS connection to the misconfigured origin.
* **Expected Defensive Behavior:** PhantomCache's upstream TLS client detects `CERT_DATE_INVALID` during the handshake, drops the socket, and returns HTTP 502 Bad Gateway (`X-Cache-Error: OriginTlsHandshakeFailed`) to the client. Zero unencrypted or unauthenticated upstream responses are accepted or cached.
* **Observed Evidence:** Client receives HTTP 502. Diagnostic event: `OriginFetchFailed: TLS certificate validation error: certificate has expired`.
* **Cleanup:** Restore valid mock origin certificate.
* **Automated Regression Test:** `tests/security/test_sec_06_origin_tls_validation.rs`.

---

### Test 7 — Altered or Incomplete Telemetry Evidence Detection
* **Objective:** Verify that an evaluator can detect if an audit report was modified post-execution or if telemetry events were dropped.
* **Lab Setup:** A valid demonstration report `evidence-report.json` generated and signed with Ed25519.
* **Attack Procedure:**
  1. Attack A (Modification): Modify `cache_hits: 950` to `cache_hits: 999` in `evidence-report.json`.
  2. Attack B (Omission): Delete event `seq_0042` from the events array.
* **Expected Defensive Behavior:** `phantom-verify` detects:
  - In Attack A: Digital signature verification failure (`InvalidSignature`).
  - In Attack B: Hash chain breakdown at event `seq_0043` (`HashChainBroken: Hash mismatch at sequence 43`).
* **Observed Evidence:** `phantom-verify` outputs:
  ```text
  [SECURITY ALERT] Demonstration evidence tampering detected!
  Signature: INVALID
  Hash Chain Integrity: BROKEN at event sequence 43.
  ```
* **Cleanup:** Restore original report.
* **Automated Regression Test:** `tests/security/test_sec_07_evidence_tamper_detection.rs`.

---

### Test 8 — Node Authentication and Distributed Failover
* **Objective:** Verify that an unauthorized node cannot join the cluster mesh, and that abrupt failure of an authorized node triggers automatic failover without client errors.
* **Lab Setup:** Nodes 1 and 2 running with cluster mTLS certificates. Rogue Node 3 has an invalid certificate.
* **Attack Procedure:**
  1. Rogue Node 3 attempts TCP connection to Node 1.
  2. Node 2 TCP listener is terminated abruptly (`stop_tcp_service()`).
  3. Client requests key mapped to Node 2.
* **Expected Defensive Behavior:**
  - Node 1 rejects Node 3 during mTLS handshake (`UnknownClusterCA`).
  - Node 1 receives TCP `ConnectionRefused` from Node 2, marks Node 2 `Suspect`, and routes request to replica Node 1 in `<2ms`.
  - Client receives valid 200 OK response with zero errors.
* **Observed Evidence:** Telemetry records: `ClusterAuthFailure: Peer rejected`. Request served successfully with `X-Cache-Status: MISS` (from replica).
* **Cleanup:** Restart Node 2.
* **Automated Regression Test:** `tests/integration/p2_dist_network_tests.rs::test_p2_actual_tcp_connection_refused_socket_failover`.

