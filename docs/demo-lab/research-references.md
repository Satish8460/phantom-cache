> Notice (2026-10-30): This document was written on 2026-10-26, before milestones M3 to M8. It is a planning and research record, not a description of the current system, and its phase numbers are not the project's current milestones (see [docs/STATUS.md](../STATUS.md)). Statements about current capability, test counts or missing features may be stale; the specific stale points known to the 2026-10-30 documentation audit are listed below where they apply.
>
> A `phantom-verify` verifier tool, where mentioned here, is a proposal: it does not exist in `src/` or `Cargo.toml`.
>
> Original text below is unchanged.

# PhantomCache — Authoritative Research References & Standards Citations

## 1. Primary Standards & IETF RFCs

### 1.1 HTTP Protocols & Proxy Semantics
* **RFC 9110 — HTTP Semantics (June 2022)**
  - *Relevant Sections:*
    - §7.1 (Intermediaries): Defines the formal roles of Gateway (Reverse Proxy), Intermediary, and Forward Proxy.
    - §7.6 (Message Forwarding): Rules for hop-by-hop header removal (`Connection`, `Keep-Alive`, `Proxy-Authenticate`).
    - §9.3.6 (`CONNECT`): Specifies TCP tunneling semantics through proxies and explains why forward proxying cannot inspect TLS payloads without MITM.
  - *PhantomCache Architectural Decision:* Informed our adoption of Mode B (Reverse Proxy / Application Gateway) over Mode A (Forward Proxy) and Mode C (Transparent Intermediary).

* **RFC 9111 — HTTP Caching (June 2022)**
  - *Relevant Sections:*
    - §3 (Storing Responses in Caches): Criteria for cacheability (`Cache-Control: no-store, private`, `Set-Cookie`).
    - §4.3 (Validation): ETag and If-None-Match conditional request revalidation.
    - §5.2 (Disjoint Responses): Coalescing requests and handling collapsed forwarding.
  - *PhantomCache Architectural Decision:* Dictated the uncacheable header filters in `src/api/server.rs` and the single-flight coalescing lifecycle in `src/core/coalesce.rs`.

* **RFC 8446 — The Transport Layer Security (TLS) Protocol Version 1.3 (August 2018)**
  - *Relevant Sections:*
    - §4.1.2 (Client Hello): Server Name Indication (SNI) and ALPN negotiations.
    - §4.2.8 (Key Share): 1-RTT handshake and forward secrecy guarantees.
  - *PhantomCache Architectural Decision:* Mandated Rustls TLS 1.3 ingress termination and strict upstream SNI verification in `docs/demo-lab/trust-and-authenticity-design.md`.

* **RFC 7519 — JSON Web Token (JWT) (May 2015)**
  - *Relevant Sections:*
    - §4.1 (Registered Claim Names): `sub`, `exp`, `iss`, `aud`.
  - *PhantomCache Architectural Decision:* Established role-based access control (RBAC) tokens for `/api/control` and cryptographic tenant binding.

---

## 2. Cybersecurity Frameworks & Threat Guidance

* **OWASP Top 10:2021 — A10:2021 Server-Side Request Forgery (SSRF)**
  - *Guidance:* Restrict intermediaries from querying untrusted destinations; segment network access; block cloud metadata IP (`169.254.169.254`) and loopback/private CIDRs.
  - *PhantomCache Architectural Decision:* Formulated the upstream destination allowlist and automated IP filtering in `docs/demo-lab/security-threat-model.md` and Scenario 4.

* **OWASP API Security Top 10:2023 — API1:2023 Broken Object Level Authorization (BOLA)**
  - *Guidance:* Do not rely on client-supplied headers for authorization boundaries; bind tenant context to cryptographic session credentials.
  - *PhantomCache Architectural Decision:* Directed the replacement of blind `x-tenant-id` header trust with cryptographic JWT/mTLS token binding.

* **NIST SP 800-207 — Zero Trust Architecture (August 2020)**
  - *Core Tenet:* Location within a physical network does not imply trust. Every transaction, including internal node-to-node cluster communication, must be authenticated and authorized.
  - *PhantomCache Architectural Decision:* Dictated cluster mTLS between distributed nodes and rejection of unauthenticated LAN observer control.

---

## 3. Software Provenance, Supply Chain & Evidence Signing

* **SLSA (Supply-chain Levels for Software Artifacts) Specification v1.0**
  - *Standard:* Non-forgeable provenance capturing the build platform, source commit, and artifact digests (Level 2/3).
  - *PhantomCache Architectural Decision:* Established the release manifest specification (`manifest.json`) in `docs/demo-lab/trust-and-authenticity-design.md`.

* **in-toto Attestation Framework (RFC / CNCF Specification)**
  - *Standard:* Cryptographic chaining of software supply chain steps from commit to deployment.
  - *PhantomCache Architectural Decision:* Guided the design of the demonstration evidence export format (`evidence-report.json`) and hash chaining.

* **Sigstore & Cosign Specification**
  - *Standard:* Keyless signing and container/binary verification using OpenID Connect and Certificate Transparency.
  - *PhantomCache Architectural Decision:* Recommended for production binary release signing.

---

## 4. Workload Identity & Remote Attestation

* **SPIFFE (Secure Production Identity Framework for Everyone) Standard**
  - *Specification:* Defines SPIFFE IDs (`spiffe://phantom.internal/ns/cache/sa/node-1`) and X.509 SVID credentials for service-to-service mutual TLS.
  - *PhantomCache Architectural Decision:* Analyzed in Section 7 of `trust-and-authenticity-design.md`; designated as the production workload identity target for Phase 7.

* **RFC 9334 — Remote ATtestation ProcedureS (RATS) Architecture (January 2023)**
  - *Relevant Concepts:* Attester, Verifier, Relying Party, Claims, Evidence, Endorsements.
  - *PhantomCache Architectural Decision:* Provided the formal conceptual model distinguishing software provenance (validating build digests) from runtime attestation (proving execution inside an uncompromised enclave).

---

## 5. Direct Mapping: Standard -> PhantomCache Architectural Decision

| Standard / Reference | Key Requirement / Constraint | PhantomCache Implementation / Architectural Decision |
| :--- | :--- | :--- |
| **RFC 9110 §9.3.6** | HTTPS forward proxy cannot inspect encrypted `CONNECT` tunnels. | Reject MITM forward proxying; adopt Mode B Reverse Proxy. |
| **RFC 9111 §3.0** | Do not cache `no-store` or private user responses. | Preserved in `src/api/server.rs` `is_uncacheable` check. |
| **RFC 8446** | Ingress and upstream transport security requires modern TLS. | Implement Rustls TLS 1.3 termination and strict SNI upstream. |
| **OWASP A10:2021** | Block SSRF probes to internal network and cloud metadata. | Add upstream allowlist + CIDR block for `169.254.169.254` & `127.0.0.0/8`. |
| **OWASP API1:2023** | Blind trust in `x-tenant-id` leads to tenant impersonation. | Derives tenant ID from authenticated JWT/mTLS claims. |
| **NIST SP 800-207** | Inter-node LAN traffic cannot be trusted implicitly. | Mandate mutual TLS (mTLS) for rendezvous node mesh. |
| **SLSA v1.0** | Cryptographic verification of release binaries before execution. | Implement signed `manifest.json` and `phantom-verify` CLI tool. |
| **in-toto** | Demonstration test results must be tamper-evident. | Design Ed25519 signed demonstration evidence report with hash chains. |

