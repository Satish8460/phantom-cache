# PhantomCache — Security Specification: Phase 2 (Upstream Transport Security & SSRF Defense)

## 1. Executive Summary

Phase 2 establishes cryptographic authenticity, transport layer security, and SSRF defense on PhantomCache's upstream origin boundary. Prior to Phase 2, PhantomCache connected over raw plaintext HTTP without TLS verification or IP address validation. This phase enforces strict origin validation, WebPKI / custom CA certificate anchoring, pre-connect IP filtering, unconditional port hardening, DNS pinning against rebinding, explicit credential forwarding policy, terminal redirect handling, and response framing integrity checks.

---

## 2. Threat Model & Security Boundaries

### 2.1 Threat Vectors Mitigated

| Threat Vector | Mechanism / Attack | Phase 2 Mitigation |
| :--- | :--- | :--- |
| **Server-Side Request Forgery (SSRF)** | Attacker exploits origin fetcher to query Cloud Metadata (AWS/GCP IMDS) or internal infrastructure. | `PreConnectIpFilter` strictly blocks `169.254.169.254`, `fd00:ec2::254`, link-local `169.254.0.0/16`, `fe80::/10`, multicast, broadcast, and RFC 1918 LAN (by default). |
| **IPv4-Mapped IPv6 Bypass** | Attacker encodes blocked IPv4 in IPv6 syntax (`::ffff:169.254.169.254`). | `PreConnectIpFilter::check_ipv6` extracts IPv4 mapping via `to_ipv4_mapped()` and recursively executes IPv4 checks. |
| **DNS Rebinding & TOCTOU** | Attacker domain returns public IP during check, private IP during connect, or returns a mixed answer set. | `resolve_and_pin_origin` inspects **every** address in DNS answer set (rejects mixed answers) and pins the socket address for connection. |
| **Sensitive Port Exploitation** | Origin configured targeting sensitive services (SSH, Redis, databases). | `HARD_BLOCKED_PORTS` constant blocks ports 20, 21, 22, 23, 25, 53, 69, 110, 143, 137-139, 445, 514, 2049, 2375-2376, 3306, 5432, 6379, 11211, 27017, 9200, 9300, 8500, 2379-2380 unconditionally (non-bypassable). |
| **Man-in-the-Middle (MitM)** | Plaintext or untrusted TLS connection between cache and origin. | Mandatory HTTPS certificate verification via WebPKI roots or custom CA anchor (`PHANTOM_UPSTREAM_CA_CERT`). Untrusted CAs, expired certificates, and SAN mismatches return `502 Bad Gateway`. |
| **Insecure HTTP Leakage** | Plain HTTP permitted on network interfaces. | `PHANTOM_ALLOW_INSECURE_UPSTREAM` defaults to false. On network interfaces, enabling it requires explicit entries in `PHANTOM_ORIGIN_ALLOWLIST`. |
| **Credential Forwarding Leak** | Client bearer tokens forwarded to untrusted or default origins upon cache bypass. | `forward_client_authorization` defaults to `false`. Bearer tokens are stripped unless explicitly allowed by operator configuration. |
| **Open Redirect SSRF** | Upstream returns 3xx redirect to cloud metadata or internal host. | HTTP 3xx responses are terminal; redirects are never followed. |
| **HTTP Response Smuggling / Partial Caching** | Conflicting Content-Length, conflicting CL + chunked, truncated bodies. | Response framing parser detects and rejects conflicting headers, enforces chunked encoding syntax, and discards truncated bodies. |

---

## 3. Core Architecture & Components

### 3.1 Unconditional Port Policy (`validate_port`)

Port validation enforces a strict 4-tier hierarchy:
1. **Hard-Blocked Sensitive Ports:** Non-negotiable, non-bypassable. Even if an operator places `http://example.com:22` in `PHANTOM_ORIGIN_ALLOWLIST`, the request fails with `OriginSecurityError::SensitivePortBlocked`.
2. **Standard Web Ports:** Standard allowed ports (80, 443, 8080, 8443, 9000, 9443) are permitted.
3. **Privileged System Ports (< 1024):** All privileged system ports outside standard web ports are blocked (`OriginSecurityError::DisallowedPort`).
4. **Custom Unprivileged Ports (>= 1024):** If an allowlist is configured, custom unprivileged ports must match an allowlisted origin. If allowlist is unconfigured (local dev mode), non-sensitive ports are permitted.

### 3.2 Pre-Connect IP Filtering (`PreConnectIpFilter`)

Every candidate IP address is validated against:
- `169.254.169.254/32` (AWS/GCP/Azure IMDS IPv4) — Blocked unconditionally.
- `fd00:ec2::254/128` (AWS IMDSv2 IPv6) — Blocked unconditionally.
- `169.254.0.0/16` and `fe80::/10` (Link-Local) — Blocked unconditionally.
- `::ffff:x.x.x.x` (IPv4-mapped IPv6) — Unmapped and checked against IPv4 filter.
- `127.0.0.0/8` and `::1` (Loopback) — Blocked unless `allow_loopback` is explicitly true.
- `0.0.0.0/8`, `255.255.255.255`, `::` (Unspecified / Broadcast) — Blocked unconditionally.
- `224.0.0.0/4` and `ff00::/8` (Multicast) — Blocked unconditionally.
- `100.64.0.0/10` (Carrier-Grade NAT) — Blocked unconditionally.
- RFC 5737 (Documentation test nets) — Blocked unconditionally.
- RFC 1918 / ULA (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`) — Blocked unless `allow_lan` is true.

### 3.3 DNS Rebinding Defense & Socket Pinning (`resolve_and_pin_origin`)

1. **Complete Answer Set Validation:** Every address returned by DNS lookup is checked. If any address is prohibited, the entire answer set is rejected with `OriginSecurityError::SsrfBlocked`.
2. **Socket Address Pinning:** The verified address is pinned and used directly for `TcpStream::connect(pinned_addr)`, eliminating TOCTOU rebinding attacks.

### 3.4 Upstream TLS Handshake (`UpstreamTlsConfig`)

- Employs `tokio-rustls 0.26` and `rustls 0.23`.
- Root store initialized with `webpki-roots 0.26`.
- Custom CA anchoring supported via `PHANTOM_UPSTREAM_CA_CERT` pointing to a PEM file.
- SNI hostname verification strictly enforced. Hostname mismatches, untrusted roots, and expired certificates abort connection and return `502 Bad Gateway` with header `X-Cache-Error: UpstreamTlsFailed`.

### 3.5 Credential Forwarding Invariant

Client `Authorization: Bearer ...` tokens are issued for PhantomCache's audience and MUST NOT leak upstream by default.
- `forward_client_authorization` defaults to `false`.
- When `false`, `Authorization` header is stripped before writing the outbound HTTP request.
- When `true` (`PHANTOM_FORWARD_UPSTREAM_AUTH=true`), credentials are sent exclusively to the configured origin authority.

### 3.6 Response Framing Integrity

In `src/api/server.rs`:
- Multiple conflicting `Content-Length` headers -> rejected with `MalformedFraming`.
- Conflicting `Content-Length` and `Transfer-Encoding: chunked` -> rejected with `MalformedFraming`.
- Chunked decoding strictly validates hexadecimal chunk sizes, CRLF delimiters, and terminal `0\r\n\r\n` chunk.
- Premature EOF / body shorter than `Content-Length` -> rejected with `TruncatedResponse` and discarded from cache.
- 3xx redirects -> treated as terminal response and never followed.

---

## 4. Configuration Environment Variables

| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `PHANTOM_ORIGIN_ALLOWLIST` | String (CSV) | empty | Comma-separated list of allowed origin URLs. |
| `PHANTOM_UPSTREAM_CA_CERT` | Path | None | Path to custom CA PEM bundle for private origins. |
| `PHANTOM_ENFORCE_UPSTREAM_TLS`| Bool | `false` | When true, rejects any `http://` origin. |
| `PHANTOM_ALLOW_INSECURE_UPSTREAM` | Bool | `false` | Allows `http://` upstream in network mode if origin is in `PHANTOM_ORIGIN_ALLOWLIST`. |
| `PHANTOM_ALLOW_LAN_ORIGIN` | Bool | `false` | Allows RFC 1918 / ULA private network origins. |
| `PHANTOM_FORWARD_UPSTREAM_AUTH` | Bool | `false` | When true, forwards client `Authorization` header to configured origin. |

---

## 5. Verification Matrix & Evidence

The Phase 2 security test suite (`tests/integration/security_phase2_upstream_tests.rs`) provides automated proof for 16 security requirements:

| Test Name | Verified Security Property | Status |
| :--- | :--- | :--- |
| `test_p2_valid_upstream_https_success` | Valid HTTPS upstream with trusted CA certificate succeeds and caches. | **PASS** |
| `test_p2_untrusted_ca_rejected` | Self-signed cert without CA anchor fails handshake, returns 502, not cached. | **PASS** |
| `test_p2_expired_cert_rejected` | Expired cert fails handshake, returns 502, not cached. | **PASS** |
| `test_p2_san_mismatch_rejected` | SAN mismatch (`evil.internal` vs `localhost`) fails hostname verification, returns 502. | **PASS** |
| `test_p2_ssrf_cloud_metadata_blocked` | Target `169.254.169.254` and `fd00:ec2::254` blocked before socket connect. | **PASS** |
| `test_p2_ssrf_ipv4_mapped_ipv6_blocked` | `::ffff:169.254.169.254` unmapped to IPv4 and blocked. | **PASS** |
| `test_p2_ssrf_mixed_dns_blocked` | DNS answer set containing blocked address is rejected in its entirety. | **PASS** |
| `test_p2_hard_blocked_ports_unconditional`| Ports 22, 6379, 25, 3306 unconditionally blocked even if in allowlist. | **PASS** |
| `test_p2_insecure_upstream_fail_closed` | Insecure HTTP on network interface rejected without override and allowlist. | **PASS** |
| `test_p2_credential_forwarding_policy` | Bearer token stripped by default; forwarded only when explicitly enabled. | **PASS** |
| `test_p2_redirect_not_followed` | Upstream 302 redirect to IMDS is returned as terminal 302, never followed. | **PASS** |
| `test_p2_conflicting_content_length_rejected` | Differing `Content-Length` headers rejected with `MalformedFraming`. | **PASS** |
| `test_p2_conflicting_cl_and_chunked_rejected` | Conflicting CL and chunked encoding rejected with `MalformedFraming`. | **PASS** |
| `test_p2_truncated_response_not_cached` | Premature TCP close before declared CL rejected with `TruncatedResponse`, not cached. | **PASS** |
| `test_p2_malformed_chunked_encoding_rejected` | Invalid hex chunk size rejected with `MalformedFraming`, not cached. | **PASS** |
| `test_p2_premature_eof_in_chunked_not_cached` | Premature EOF during chunked stream rejected with `TruncatedResponse`, not cached. | **PASS** |

