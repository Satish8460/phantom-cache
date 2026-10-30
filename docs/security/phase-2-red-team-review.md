# PhantomCache — Phase 2 Red-Team Security Review & Verification Report

**Review Date:** October 2026  
**Target Phase:** Phase 2 — Upstream Transport Security and SSRF Defense  
**Branch:** `feat/publish-phantomcache-core`  
**Git Working Tree Status:** Dirty (Phase 1 + Phase 2 uncommitted work preserved; 0 commits, 0 pushes, 0 staged files)  
**Overall Verdict:** **ACCEPT WITH VERIFIED REMEDIATIONS**  

---

## 1. Executive Summary

An adversarial red-team code audit and penetration verification pass was conducted on the completed Phase 2 implementation of PhantomCache. Phase 2 introduces:
1. Mandatory upstream HTTPS certificate validation and hostname verification using Rustls and WebPKI roots, with optional custom private CA root injection.
2. Anti-SSRF multi-layer defense featuring pre-connect IP address filtering, DNS pinning, strict destination allowlisting, and hard-blocking of sensitive infrastructure ports.
3. Strict HTTP/1.1 response framing parsing (`Content-Length` and `Transfer-Encoding: chunked`), preventing request/response desynchronization, response splitting, and cache poisoning.
4. Tenant credential boundary isolation guaranteeing that caller `Authorization` headers are stripped by default and never forwarded to arbitrary upstream origins.

The initial implementation achieved 104 passing tests. However, adversarial auditing identified **2 High**, **3 Medium**, and **2 Low** security vulnerabilities. All confirmed vulnerabilities have been remediated with minimal safe patches and verified against 7 dedicated red-team regression tests. The full test suite now comprises **111 tests passing (0 failures)** with zero Clippy warnings.

---

## 2. Review Baseline & Git Environment

- **Repository Root:** `d:\Personal\phantom-cache`
- **Branch:** `feat/publish-phantomcache-core` (synchronized with upstream tracking)
- **Git State:**
  - Modified files: `src/main.rs`, `src/api/server.rs`, `src/core/coalesce.rs`, `src/core/cache.rs`, `Cargo.toml`, `Cargo.lock`
  - Untracked Phase 2 modules: `src/security/upstream.rs`, `tests/integration/security_phase2_upstream_tests.rs`, `docs/security/phase-2-specification.md`
  - Zero files staged (`git add` was not executed)
  - Zero commits or pushes performed

---

## 3. Acceptance Criteria Traceability Matrix

| Test ID | Security Requirement | Status | Verification Evidence |
| :--- | :--- | :---: | :--- |
| **AT-P2-01** | Upstream HTTPS TLS 1.2/1.3 with valid CA and matching SAN succeeds | **PASS** | `test_p2_valid_upstream_https_success` |
| **AT-P2-02** | Upstream TLS fails closed on untrusted CA certificate | **PASS** | `test_p2_untrusted_ca_rejected` |
| **AT-P2-03** | Upstream TLS fails closed on expired server certificate | **PASS** | `test_p2_expired_cert_rejected` |
| **AT-P2-04** | Upstream TLS fails closed on SAN / hostname mismatch | **PASS** | `test_p2_san_mismatch_rejected` |
| **AT-P2-05** | SSRF: Cloud metadata IP `169.254.169.254` blocked pre-connect | **PASS** | `test_p2_ssrf_cloud_metadata_blocked` |
| **AT-P2-06** | SSRF: IPv4-mapped, IPv4-compatible, 6to4, NAT64 blocked | **PASS** | `test_p2_ssrf_ipv4_mapped_ipv6_blocked`, `test_redteam_ipv4_compatible_and_transition_ipv6_ssrf_blocked` |
| **AT-P2-07** | SSRF: Multi-address DNS answer sets checked and pinned | **PASS** | `test_p2_ssrf_mixed_dns_blocked` |
| **AT-P2-08** | Sensitive infrastructure ports unconditionally hard-blocked | **PASS** | `test_p2_hard_blocked_ports_unconditional` |
| **AT-P2-09** | Plaintext HTTP upstream rejected by default in network mode | **PASS** | `test_p2_insecure_upstream_fail_closed`, `test_redteam_insecure_upstream_empty_allowlist_fails_closed` |
| **AT-P2-10** | Upstream HTTP redirects (301/302/307/308) never followed | **PASS** | `test_p2_redirect_not_followed` |
| **AT-P2-11** | Caller credentials stripped by default and forwarded only under policy | **PASS** | `test_p2_credential_forwarding_policy`, `test_redteam_default_policy_strips_credentials` |
| **AT-P2-12** | Response framing validation rejects malformed/conflicting framing | **PASS** | `test_p2_conflicting_cl_and_chunked_rejected`, `test_p2_malformed_chunked_encoding_rejected`, `test_redteam_unsupported_transfer_encoding_rejected` |

---

## 4. Adversarial Findings and Remediations

### Finding 1: IPv6 Transition Mechanism SSRF Bypass (6to4, NAT64, IPv4-Compatible)
- **Severity:** **HIGH** (CVSS: 7.5)
- **Component:** `src/security/upstream.rs:PreConnectIpFilter::check_ipv6`
- **Vulnerability Description:**
  The pre-connect filter checked `ip.to_ipv4_mapped()`, which strictly checks the `::ffff:0:0/96` prefix. An attacker or malicious DNS server could supply IPv4-compatible IPv6 addresses (`::169.254.169.254`), 6to4 relay addresses (`2002:a9fe:a9fe::/48`), or Well-Known NAT64 prefixes (`64:ff9b::169.254.169.254`). These bypassed the IPv4 metadata filters while resolving to the cloud metadata service on dual-stack hosts.
- **Root Cause:**
  Incomplete unmapping of transition mechanisms prior to IP evaluation. Additionally, Rust's `to_ipv4()` converts `::1` into `0.0.0.1`, which caused loopback to be evaluated as broadcast IPv4 if loopback was checked after unmapping.
- **Remediation:**
  1. Updated `src/security/upstream.rs` to check `ip.is_loopback()` and `ip.is_unspecified()` first.
  2. Integrated `ip.to_ipv4()` for both IPv4-compatible and IPv4-mapped addresses.
  3. Extracted and checked embedded IPv4 addresses from 6to4 (`2002::/16`, segments 1 and 2) and NAT64 (`64:ff9b::/96`, segments 6 and 7).
  4. Blocked IPv6 documentation (`2001:db8::/32`) and discard prefixes (`100::/64`, RFC 6666).
- **Regression Test:** `test_redteam_ipv4_compatible_and_transition_ipv6_ssrf_blocked`.

---

### Finding 2: Insecure Upstream Permitted Arbitrary Non-Loopback Destinations with Empty Allowlist
- **Severity:** **HIGH** (CVSS: 7.4)
- **Component:** `src/security/upstream.rs:UpstreamPolicy::validate_target`
- **Vulnerability Description:**
  In `validate_target`, the condition for non-loopback HTTP destinations was written as:
  ```rust
  if !is_allowlisted && !self.allowed_origins.is_empty() {
      return Err(OriginSecurityError::OriginNotAllowlisted(...));
  }
  ```
  If `allowed_origins` was empty and `allow_insecure_upstream` was true, any arbitrary external HTTP destination bypassed allowlist verification.
- **Root Cause:** Inverted check allowing an empty allowlist to act as a wildcard for insecure HTTP.
- **Remediation:**
  Hardened `validate_target` to require allowlist inclusion unconditionally:
  ```rust
  if !is_allowlisted {
      return Err(OriginSecurityError::OriginNotAllowlisted(origin_normalized));
  }
  ```
- **Regression Test:** `test_redteam_insecure_upstream_empty_allowlist_fails_closed`.

---

### Finding 3: Default Credential Forwarding Failed Open Without Upstream Policy
- **Severity:** **MEDIUM** (CVSS: 6.5)
- **Component:** `src/api/server.rs:fetch_origin_raw`
- **Vulnerability Description:**
  In `fetch_origin_raw`, credential forwarding evaluated as:
  ```rust
  let forward_auth = policy.map(|p| p.forward_client_authorization).unwrap_or(true);
  ```
  If no explicit `UpstreamPolicy` was attached to `ApiState`, client credentials (`Authorization` header) were forwarded to upstream origins by default.
- **Root Cause:** Insecure default fallback (`unwrap_or(true)` instead of `unwrap_or(false)`).
- **Remediation:**
  Changed default fallback to `unwrap_or(false)` in `src/api/server.rs:fetch_origin_raw`. Updated unit tests to specify explicit opt-in policy when credential forwarding is under test.
- **Regression Test:** `test_redteam_default_policy_strips_credentials`.

---

### Finding 4: Unsupported Transfer-Encoding Values Accepted Without Validation
- **Severity:** **MEDIUM** (CVSS: 5.8)
- **Component:** `src/api/server.rs:fetch_origin_raw`
- **Vulnerability Description:**
  If an upstream server responded with an unsupported `Transfer-Encoding` such as `gzip`, `compress`, or `identity`, PhantomCache treated the response as close-delimited plaintext, potentially serving uncompressed raw tokens or corrupted cache entries.
- **Root Cause:** Absence of strict RFC 9112 §6.1 validation for `Transfer-Encoding`.
- **Remediation:**
  Enforced strict rejection of any `Transfer-Encoding` value other than `chunked` with `OriginSecurityError::MalformedFraming`.
- **Regression Test:** `test_redteam_unsupported_transfer_encoding_rejected`.

---

### Finding 5: Shared Cache Poisoning via Close-Delimited Premature Connection Termination
- **Severity:** **MEDIUM** (CVSS: 5.3)
- **Component:** `src/api/server.rs` (cache insertion logic)
- **Vulnerability Description:**
  Responses without explicit framing (`Content-Length` or `Transfer-Encoding: chunked`) are close-delimited. If the upstream TCP connection closed prematurely during body streaming, a truncated payload was inserted into the shared cache with a 60-second TTL, poisoning the cache for subsequent callers.
- **Root Cause:** Absence of framing completeness validation prior to shared cache insertion.
- **Remediation:**
  In `src/api/server.rs`, responses lacking both `Content-Length` and `Transfer-Encoding` are flagged as `is_uncacheable = true`. They are streamed to the initiating caller but never inserted into `PartitionedCache`. Additionally, hardened `decode_chunked_body` to correctly ignore chunk extensions (RFC 9112 §7.1.1) and enforce terminal CRLF bounds.
- **Regression Tests:** `test_redteam_close_delimited_body_not_cached`, `test_redteam_chunk_extensions_parsed_safely`.

---

### Finding 6: Information Disclosure in Public Error Responses
- **Severity:** **LOW / DEFENSE-IN-DEPTH** (CVSS: 3.1)
- **Component:** `src/api/server.rs:map_origin_error_to_response`
- **Vulnerability Description:**
  Raw internal error strings (e.g., DNS error details, OS socket error codes `WSAECONNREFUSED 10061`, TLS engine alerts) were rendered in HTTP response bodies returned to clients.
- **Root Cause:** Unsanitized error serialization in HTTP gateway error mapper.
- **Remediation:**
  Sanitized all public response bodies to return generic error descriptions (`"Upstream TLS handshake failed"`, `"Origin connection failed"`), while retaining structured error codes in the `X-Cache-Error` header for debugging.
- **Verification:** Verified across all error tests.

---

### Finding 7: Unstructured Startup Panic for Upstream Security Violations
- **Severity:** **LOW / OPERATIONAL** (CVSS: 2.8)
- **Component:** `src/main.rs`, `src/security/mod.rs`
- **Vulnerability Description:**
  Upstream security misconfigurations (e.g., network bind with `PHANTOM_ALLOW_INSECURE_UPSTREAM=true` but empty `PHANTOM_ORIGIN_ALLOWLIST`) panicked with ad-hoc string messages, bypassing the structured `StartupSecurityError` type used by Phase 1.
- **Remediation:**
  Added `StartupSecurityError::NetworkInsecureUpstreamWithoutAllowlist(SocketAddr)` and `validate_upstream_startup_security` in `src/security/mod.rs`. Integrated the check into `src/main.rs` and `src/api/server.rs:start_server`.
- **Regression Test:** `test_redteam_startup_security_upstream_validation`.

---

## 5. Verification Commands and Output

### 1. Dedicated Phase 2 Security Integration Test Suite
```pwsh
cargo test --test security_phase2_upstream_tests -j 1
```
- **Exit Code:** `0`
- **Output:**
```text
running 23 tests
test test_p2_hard_blocked_ports_unconditional ... ok
test test_p2_ssrf_ipv4_mapped_ipv6_blocked ... ok
test test_p2_ssrf_cloud_metadata_blocked ... ok
test test_redteam_ipv4_compatible_and_transition_ipv6_ssrf_blocked ... ok
test test_p2_ssrf_mixed_dns_blocked ... ok
test test_redteam_startup_security_upstream_validation ... ok
test test_redteam_insecure_upstream_empty_allowlist_fails_closed ... ok
test test_p2_insecure_upstream_fail_closed ... ok
test test_p2_malformed_chunked_encoding_rejected ... ok
test test_p2_conflicting_cl_and_chunked_rejected ... ok
test test_p2_premature_eof_in_chunked_not_cached ... ok
test test_redteam_unsupported_transfer_encoding_rejected ... ok
test test_p2_truncated_response_not_cached ... ok
test test_redteam_default_policy_strips_credentials ... ok
test test_p2_redirect_not_followed ... ok
test test_redteam_chunk_extensions_parsed_safely ... ok
test test_p2_conflicting_content_length_rejected ... ok
test test_redteam_close_delimited_body_not_cached ... ok
test test_p2_credential_forwarding_policy ... ok
test test_p2_san_mismatch_rejected ... ok
test test_p2_untrusted_ca_rejected ... ok
test test_p2_expired_cert_rejected ... ok
test test_p2_valid_upstream_https_success ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

### 2. Full Repository Test Suite
```pwsh
cargo test --tests -j 1
```
- **Exit Code:** `0`
- **Test Binary Summary:**
  - `coalesce_tradeoffs_tests`: 5 passed
  - `m1_http_tests`: 32 passed
  - `p2_dist_network_tests`: 7 passed
  - `p3_traffic_tests`: 5 passed
  - `p4_pie_tests`: 6 passed
  - `security_phase1_tests`: 15 passed
  - `security_phase2_upstream_tests`: 23 passed
  - `wp1_tests`: 5 passed
  - `wp2_tests`: 5 passed
  - `wp3_tests`: 3 passed
  - `wp4_tests`: 5 passed
  - **Total:** **111 passed; 0 failed; 0 ignored**

### 3. Clippy Static Analysis
```pwsh
cargo clippy --all-targets -j 1 -- -D warnings
```
- **Exit Code:** `0`
- **Output:**
```text
Checking phantom-cache-demo v0.1.0 (D:\Personal\phantom-cache)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.71s
```

---

## 6. Scope Integrity & Git Discipline

- No git commits, no git pushes, and no branch merges were performed.
- All files in the working directory (including uncommitted Phase 1 work and unstaged changes) were preserved without destruction or modification of unrelated logic.
- Commands were executed with `-j 1` to ensure deterministic execution and prevent paging file exhaustion on Windows.

---

## 7. Residual Risks & Phase 3 Recommendations

1. **DNS Rebinding Window:**
   DNS pinning validates and connects to the identical socket address resolved during pre-connect filtering. However, if an upstream origin hostname resolves to dynamic short-lived IPs, TTL-based caching of socket addresses could be considered for long-lived keepalive connections.
2. **Distributed Node-to-Node Security (Phase 3 Scope):**
   Inter-node cluster traffic (`src/dist/node.rs`, TCP replica communication) currently runs over unauthenticated TCP sockets. Phase 3 must establish:
   - Node-to-node mutual TLS (mTLS) with dedicated internal CA certificates.
   - Cryptographic node identity verification preventing rogue peers from joining the hash ring or receiving replica updates.
   - Signed inter-node gossip and invalidation messages.

---

## 8. Final Conclusion

With all 7 red-team findings resolved, verified, and backed by automated regression tests, the Phase 2 implementation of **Upstream Transport Security and SSRF Defense** satisfies all architectural and security requirements and is **accepted**.

