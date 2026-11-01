---
title: "PhantomCache Engineering and Technical Defence Handbook, Part II: Technical Reference"
subtitle: "PROPOSAL (two-layer structure): tables, judge bank, glossary, source map, inventories"
---

> **Two-layer proposal.** This is Part II, the Technical Reference. It is consulted, not read in order. Part I (`learning.md` / `learning.pdf`) teaches the system. <!--gen-->
> Every item here was moved unchanged from the original handbook; section numbers (4.11, 8.12, Appendix G) are the original ones. Where a section was split, its heading is repeated here and marked as a continuation. <!--gen-->
> The status labels are defined in the front matter of Part I. <!--gen-->

# R1. Reference matrices, route and configuration tables <!--gen-->

### 3.1 The dependency picture in one table

Direct runtime dependencies from `Cargo.toml`, with the resolved versions in `Cargo.lock` (288 locked packages, lockfile v4; reproduced by reading the lockfile, and consistent with `docs/release/g4/cycle1_lab_intelligence_deps_review.md`, an **AI** review, not independent).

| Crate (declared) | Locked | What it is for here | Used in `src`? |
|---|---|---|---|
| `tokio` 1.38, features `full` | 1.53.1 | async runtime, timers, channels, TCP | yes, everywhere |
| `axum` 0.7, features `ws`, `macros` | 0.7.9 | HTTP routing, extractors, SSE | yes; **no WebSocket and no `#[debug_handler]` use found** |
| `axum-server` 0.7, `tls-rustls` | 0.7.3 | TLS listener for the ingress | yes (`security/tls.rs`, `server.rs`) |
| `hyper` / `hyper-util` / `tower` | 1.11.1 / 0.1.20 / 0.5.3 | **transitive** (via axum); not declared | not named in `src` (`hyper::` has zero hits) |
| `tower-http` 0.5, `cors`, `fs` | 0.5.2 (a second copy, 0.6.11, is also locked) | CORS layer | only `cors`; **`ServeDir`/`fs` unused** |
| `serde`, `serde_json` | 1.0.229 / 1.0.151 | JSON for API bodies, lab artifacts, JWT claims | yes |
| `jsonwebtoken` 9 | 9.3.1 | JWT sign/verify | yes (`security/jwt.rs`) |
| `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2.2, `webpki-roots` 0.26 | 0.23.46 / 0.26.6 / 2.2.0 / 0.26.11 | TLS for cluster mTLS, origin HTTPS, ingress | yes |
| `ring` 0.17 | 0.17.14 | SHA-256 | yes (5 files in `src`; fingerprints, anti-entropy ring digest, lab hashes) |
| `rand` 0.8 | 0.8.8 (a second copy, 0.10.3, is transitive) | seeds, run ids, **synthetic dashboard numbers** | yes |
| `tokio-stream`, `futures-util` | 0.1.19 / 0.3.34 | SSE interval stream, stream helpers | yes |
| `url` | 2.5.8 | origin URL parsing | yes |
| `tracing`, `tracing-subscriber` | 0.1.44 / 0.3.x | structured logging | **no use found in `src`, `tests`, `benches`**; logs are `println!`/`eprintln!` |

Dev-dependencies: `jsonwebtoken`, `rcgen` 0.13.2, `reqwest` 0.12.28, `hdrhistogram` 7.6.0, `proptest` 1.12.0, and `criterion` 0.5.1 (declared, **no use found**; the seven benches have `harness = false` and a custom `main`).

Verification: I counted file-level `grep` hits over `src/`, `tests/`, `benches/` (`grep -rln "tracing::" src` returned nothing). "No use found" describes the current tree; it does not prove that a macro-generated or feature-gated use cannot exist. Axum itself pulls in `tracing` and `tokio-tungstenite` through its own features (`Cargo.lock`), so those crates are compiled regardless.

::: caution
Unused declarations (`tracing`, `tracing-subscriber`, `criterion`, the `ws` and `fs` features, UI `lucide-react`) widen the supply chain for no benefit. They are harmless to correctness. They are a fair point for a hostile reviewer, and the honest answer is "declared, not used, a tidy-up for the owner to approve" (the UI audit item UA-15 and the toolkit proposal TK-26 record `lucide-react` as an open owner decision).
:::


### 3.6 Data structures as actually used

| Structure | Where | Why this one (our reasoning) | Notes |
|---|---|---|---|
| `HashMap<CacheKey, KeyState>` per shard | `CacheShard.map` | O(1) average lookup by key | `CacheKey` includes tenant and path (Chapter 4) |
| `HashMap` + `HashSet` | in-flight map, tenant epochs, S3-FIFO ghost set and frequency map | membership tests | |
| `Vec<Option<Node>>` + `free_list: Vec<usize>` + `HashMap<CacheKey, usize>` | `FaithfulSieve`, `FaithfulLru` (`core/eviction.rs`) | an **arena-based doubly linked list**: nodes live in a `Vec`, links are indices, freed slots are reused. Gives O(1) unlink and move without unsafe pointers | `hand: usize` for SIEVE |
| `VecDeque<CacheKey>` x 3 + `HashSet` + `HashMap<_, u8>` | `FaithfulS3Fifo` | FIFO queues (small, main, ghost) | G4A-02: `on_remove` leaves tombstones; OPEN; default policy is SIEVE |
| `Vec<CacheKey>` + `HashMap<_, bool>` | `ClockScan` | legacy O(n) scan fallback (`on_remove` uses `retain`) | not the default |
| `Vec<AtomicU64>` of 2,688 | `AtomicLogLinearHistogram` | lock-free log-linear latency histogram | see 3.5.3 |
| `VecDeque` ring buffers | diagnostics, audit store, anti-entropy ring, engine switch history | bounded history with O(1) push/pop at both ends | capacities are explicit |
| `BTreeMap` / `BTreeSet` | `config.rs` (sorted config keys), `phantom-lab` reports, `intelligence/mod.rs` parameters, anti-entropy `resync_needed` | deterministic iteration order for output and hashes | |
| fixed array `[u64; 15]` | `origin_metrics::LATENCY_BUCKETS_MS` | coarse origin latency buckets | |
| `Vec<Vec<u32>>` | `intel/sketch.rs::CountMinSketch` | a probabilistic frequency counter | **no use outside its own file** (`grep CountMinSketch src`); PROTOTYPE ONLY |
| bitmaps | none found | | I searched for bitmap/bit-vector types and found none; "visited" flags are `bool`s |

Complexity claims (CLAUDE.md "O(1) amortised") belong to Chapter 4; the G4A-20 finding says pinned-key scans are not O(1), OPEN.


### 4.11 Capability status table

| Capability | Status | Evidence and limits |
|---|---|---|
| Sharded key map, 8 shards in production | IMPLEMENTED, TESTED | `cache.rs`; many tests; contention not measured |
| Single-flight and coalescing, waiter limit | IMPLEMENTED, TESTED | `coalesce_tradeoffs_tests`, `serving_contract_tests`, `m1_http_tests`; loopback only |
| Generation fencing against DELETE | IMPLEMENTED, TESTED | `test_delete_fences_in_flight_fill_race`; single interleaving |
| Tenant and global epoch fencing | IMPLEMENTED, PARTIAL | known race G4A-01 (sandbox-reproduced, fix not integrated); G4A-08, G4A-17 open |
| Epoch overflow fallback to global epoch | IMPLEMENTED, TESTED | `test_epoch_saturation_fallback_sec4a_preserved`; peer-triggerable (G4A-12) |
| Lazy sweeper | IMPLEMENTED, PARTIAL | G4A-03 (full scan), G4A-16 (validators dropped) |
| Freshness engine (RFC 9111 subset) | IMPLEMENTED, PARTIAL | `freshness_matrix_tests` (not analysed here); G4A-04, -05, -06, -10 |
| `PHANTOM_DEFAULT_TTL_MS` | NOT WIRED (setting has no effect) | `config.rs` `Wiring::NotWired`; literal 60,000 in `server.rs` |
| `PHANTOM_MAX_STALE_MS` hard cap | IMPLEMENTED, TESTED | cap tests; counter weak (G4A-18); wall clock (G4A-10) |
| Revalidation with 304, backoff, bounded background refresh | IMPLEMENTED, TESTED | `revalidation_304_tests`, `stale_serving_tests`; one test `#[ignore]`d |
| SIEVE (default) and LRU eviction | IMPLEMENTED, TESTED (weakly) | pinned scans not O(1) (G4A-20); no hit-ratio check against literature |
| S3-FIFO eviction | IMPLEMENTED, PARTIAL | defect G4A-02 OPEN (tombstones); not selected by default |
| Byte accounting I1 to I8 | IMPLEMENTED, TESTED (consistency) | `byte_accounting_tests`; G4A-13, G4A-23 |
| Origin concurrency gate (AIMD) | IMPLEMENTED, TESTED (arithmetic and cap unit tests) | on the serving path only when a cap is set in canary or active mode; default off; `executor.rs`, `origin_cap_tests`; benefit not established |
| Hot-hit scalability (read lock, replication) | DESIGNED BUT NOT IMPLEMENTED | no code found; not claimed |
| Loom or model-checker verification of locking | NOT DONE | only an abstract model test exists |
| Independent human review of this engine | NOT DONE | G4A is an AI second look |


### 5.15 Chapter status summary

| Capability | Status | Key limit |
|---|---|---|
| Static membership, pinned-certificate mTLS cluster | IMPLEMENTED, TESTED | No runtime change or revocation trigger (G4B-07); no certificate-lifetime enforcement on live links (G4B-08) |
| PhantomMesh frame protocol | IMPLEMENTED, TESTED | Codec and loopback behaviour only |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`); no code path lets a heartbeat change node state (G4B-12) |
| Rendezvous hashing, equal weights | IMPLEMENTED | No uniformity test in repo; unspecified `DefaultHasher` |
| Weighted rendezvous hashing | DESIGNED BUT NOT IMPLEMENTED | Weight is a constant 1.0 |
| Routed reads / peer fill | DESIGNED BUT NOT IMPLEMENTED | GET is local only; peer `GetRequest` is a placeholder (DR-029) |
| Exact-key DELETE: local purge, primary ack, fan-out to all members | IMPLEMENTED, TESTED | 204 is not a cluster-wide ack (G4B-09) |
| Retry, per-peer queues, repair ledger, pull anti-entropy | IMPLEMENTED, TESTED | Memory only; single host loopback evidence |
| Durable owed-invalidation log | PROTOTYPE ONLY (sandbox) | Not integrated; needs review and owner approval |
| Cross-node linearizability, bounded staleness | Not claimed | See section 5.6 |
| Membership-epoch enforcement on invalidations | Not implemented | G4B-06 |
| Tenant/global epochs (local) | IMPLEMENTED, TESTED | G4A-01, G4A-17 open (Chapter 4) |
| Telemetry log-linear histogram | IMPLEMENTED, TESTED | Recording and bucket arithmetic only; it is fed but has no production percentile reader, and it is not the source of displayed percentiles |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY (library) | Unit-tested; not wired into the serving path; metrics report hot keys unavailable |
| AIMD origin cap controller | IMPLEMENTED, TESTED (arithmetic and cap unit tests) | On the serving path only when a cap is configured in canary or active mode (default off); benefit not established; "canary" is 100 percent |
| Shadow evaluation, decision engine, canary routing, watchdog | IMPLEMENTED as library | Not wired; statistical flaws F12, F13 |
| Legacy `PieController` | Legacy stub | Not constructed |
| Machine learning | Not present | A hand-set stump in a library |



### 6.11 The security and failure-scenario matrix

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


### 8.10 Production-maturity checklist

Read "Status" as the honest present state. "No" means absent; nothing here is a claim of readiness.

| Area | Requirement | Status |
|---|---|---|
| Correctness | Fencing, byte accounting, freshness tested | TESTED (debug); open G4A-01 and freshness gaps |
| Correctness | Independent review of the protected core | UNVERIFIED (AI second look only; no human) |
| Correctness | Mutation or model evidence that fencing tests fail without the change | DESIGNED BUT NOT IMPLEMENTED |
| Durability | Issuer-crash-safe invalidation | PROTOTYPE ONLY (sandbox; default would stay off) |
| Distribution | Multi-host run on real network | BLOCKED |
| Distribution | Larger than four nodes, WAN | UNVERIFIED (not run); WAN deferred |
| Security | Ingress hardening (auth before body, timeouts, connection limits, rate limits) | PARTIAL; G4B-01, -02, -11 open |
| Security | Cluster peer authorization per scope | DESIGNED BUT NOT IMPLEMENTED (DR-027) |
| Security | JWT lifecycle (`nbf`, max lifetime, revocation) | PARTIAL |
| Security | Dependency advisory scan and SBOM | DEFERRED |
| Security | External or human security review | BLOCKED |
| Performance | Release-profile suite, repeated measurements with intervals | DEFERRED |
| Performance | Any published latency or throughput number | none made; the project declines to publish one |
| Operations | Graceful drain, health endpoints | health endpoints TESTED (`tests/integration/m7_health_tests.rs`); drain DEFERRED |
| Operations | Metrics export, config reload, TLS reload | DEFERRED |
| Operations | Linux and container packaging | UNVERIFIED |
| Operations | Idle CPU behaviour | PARTIAL (observed high; cause hypothesised) |
| Tenancy | Quotas and noisy-neighbour isolation | DESIGNED BUT NOT IMPLEMENTED |
| Observability | Every UI value labelled by provenance | TESTED (static scan `check-forbidden`, DOM tests); visual and usability acceptance open |
| Process | Everything committed and pushed | No (nothing beyond four documentation or tooling commits) |
| Process | Formal owner acceptance of milestones | Not recorded |
| Process | Two review cycles (G4) | PARTIAL (cycle 1 Rust only) |


### 8.12 Implementation versus roadmap matrix

Exactly one label per capability. "Why" gives the one reason that justifies the label. Reading rule used in every chapter and appendix: TESTED means IMPLEMENTED and a named test exercises it (Chapters 4 and 5 write this "IMPLEMENTED, TESTED"; it is the same label), and "IMPLEMENTED, PARTIAL" is written here as PARTIAL. No row is INDEPENDENTLY VERIFIED: nothing in this repository has been checked by a human or external reviewer.

| Capability | Status | Why |
|---|---|---|
| Single-flight coalescing | TESTED | Named integration tests; coalescing under cancellation reviewed only by reading |
| Generation fencing against DELETE | TESTED | `test_delete_fences_in_flight_fill_race`; a single interleaving |
| Tenant and global epoch fencing | PARTIAL | IMPLEMENTED, but the G4A-01 race is reproduced in a sandbox and its fix is not integrated; G4A-08 and G4A-17 open |
| Byte accounting (I1 to I8) | TESTED | `audit_bytes` equality tests; G4A-13 drift on a rare path |
| Freshness (RFC 9111 subset) | PARTIAL | `Age`, `Expires: 0`, stale-if-error scope gaps (G4A-04 to -06) |
| Default TTL setting | PARTIAL | Validated, not wired |
| Stale-while-revalidate, stale-if-error, hard stale cap | TESTED | Counter-based gate is weak (G4A-18) |
| SIEVE (default) and LRU eviction | TESTED | Weakly: pinned scans are not O(1) (G4A-20); no hit-ratio check against the literature; no release benchmark |
| S3-FIFO eviction | PARTIAL | IMPLEMENTED with the open defect G4A-02 (tombstones); not selected by default |
| Lazy sweeper | PARTIAL | Scans the shard under the lock; shutdown channel dropped (D-2) |
| Telemetry log-linear histogram | TESTED | Recording and bucket arithmetic only (`telemetry_redesign_tests`); the histogram is fed but has no production percentile reader (Chapter 6, 6.12.2) |
| HTTP ingress hardening | PARTIAL | G4B-01, -02, -04, -11 open |
| JWT roles, mTLS identity | TESTED | M7 security tests; lifecycle gaps open |
| SSRF upstream filter | TESTED | Range gaps (G4B-15) |
| Cluster invalidation fan-out, repair, anti-entropy | TESTED | IMPLEMENTED and tested on loopback, debug build, at most four nodes; labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED; 204 is not a cluster-wide acknowledgement (G4B-09) |
| Durable invalidation | PROTOTYPE ONLY | Sandbox patch, not integrated |
| Graceful connection drain | DEFERRED | Integration item D-1 |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`), but no code path lets a heartbeat change node state (G4B-12) |
| Runtime membership and revocation | PARTIAL | Machinery, no production trigger |
| Rendezvous routing for DELETE primary | IMPLEMENTED | Unweighted, unstable hash |
| Peer-fill, coordinated reads, stable placement | DESIGNED BUT NOT IMPLEMENTED | A stub handler exists and is a defect, not the feature |
| Multi-host operation | BLOCKED | No devices |
| Tenant keyspace isolation | TESTED | Tenant-keyed entries and epochs |
| Tenant quotas, fair eviction | DESIGNED BUT NOT IMPLEMENTED | Frozen core |
| Rate limiting | DEFERRED | SEC-M7-11 queued |
| Static origin concurrency cap | TESTED | Scenario B and unit tests |
| AIMD cap controller | TESTED | Arithmetic and cap unit tests only; on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT SUPPORTED |
| Signals, shadow evaluator, decision engine | PROTOTYPE ONLY | Not wired; unsound statistics (F12, F13) |
| Legacy PIE | PROTOTYPE ONLY | `pie: None` |
| Intelligence `canary` mode | PARTIAL | Cap only; no canary or rollback in serving |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY | Library in `src/intelligence/signals.rs`, unit-tested and used by the lab simulator's admission filter, not wired into the serving path; the older `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test |
| Event tap, pre-warming, forecasting | DESIGNED BUT NOT IMPLEMENTED | Experiments E0, E9, E10 first |
| ML selector | DEFERRED | Gate unmet; negative claim unverified |
| Lab API and Scenario B | TESTED | Exploratory; U-01 defect open |
| Sealed M6 evaluation | TESTED | Executed once, Tier 1 simulated, not human-reviewed |
| T1 to T2 (simulator to real) agreement | UNVERIFIED | No agreement study run |
| Release-profile performance measurements | DEFERRED | Needs a quiet machine |
| Dashboard UI I1 to I4 | TESTED | Automated checks only |
| Visual acceptance of the UI (O8-2) | UNVERIFIED | Owner decision open |
| Demo Center and `demo-1` stream | DESIGNED BUT NOT IMPLEMENTED | Proposal only |
| Human review of design, usability, security | BLOCKED | No human reviewer |
| Linux and container deployment | UNVERIFIED | Never run |
| Metrics export, TLS and config reload | DEFERRED | Not provided |
| Dependency advisory scan and SBOM | DEFERRED | Queued |


## Appendix C: Technology and algorithm reference

Use this appendix to look something up fast. It adds no new claims. Each row repeats what a chapter says and points to it. Status labels are the chapters' labels: IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, NOT WIRED. A TESTED label means a named test exercises the thing. It does not mean the thing is correct. Nothing here is INDEPENDENTLY VERIFIED by a human. Where a row names a symbol, it is written `path::symbol`. Complexity entries are the chapters' own reasoning unless a measurement is cited, and no performance number in this appendix is a measurement.


### C.1 Technologies

| Technology | What it is, and why it is here | Where in the code path (`path::symbol`) | Status | Chapter |
|---|---|---|---|---|
| Rust | Memory-safe systems language. Chosen for safety and no garbage collector. The only `unsafe` found in `src/` is Windows FFI in the Lab API | all of `src/`; `src/api/lab_api.rs` (the `unsafe` blocks) | IMPLEMENTED | Ch 3, section 3.2; Ch 6, section 6.10 |
| Tokio | Async runtime: timers, channels, TCP, spawned tasks. The leader fetch runs as a spawned task | `src/main.rs` (`tokio::runtime::Builder::new_multi_thread`) | IMPLEMENTED | Ch 3, section 3.3 |
| Axum (hyper and tower arrive transitively) | HTTP routing, extractors, server-sent events. No WebSocket use found | `src/api/server.rs::create_app_with_lab` | IMPLEMENTED, TESTED | Ch 3, section 3.4 |
| tower-http CORS layer | Lets the browser UI on port 3000 call the gateway. The `fs` feature is declared but unused | `src/api/server.rs::create_app_with_lab` (layer) | IMPLEMENTED | Ch 3, sections 3.1 and 3.4 |
| `std::sync::Mutex` per shard | One lock per bucket of keys, so unrelated keys do not wait | `src/core/cache.rs::PartitionedCache`, `CacheShard` | IMPLEMENTED, TESTED | Ch 3, section 3.5.2; Ch 4, section 4.2 |
| `RwLock` epoch snapshot | Readers see the tenant epochs without the exclusive mutex | `src/core/cache.rs::PartitionedCache::epoch_snapshot`, `get_effective_epochs` | IMPLEMENTED, TESTED | Ch 3, section 3.5.4; Ch 4, section 4.7 |
| Atomics | Lock-free counters and histogram buckets (relaxed ordering) | `src/intel/telemetry.rs::AtomicLogLinearHistogram`; `src/intel/diagnostics.rs` | IMPLEMENTED, TESTED | Ch 3, section 3.5.3 |
| `DefaultHasher` (SipHash) | Picks the shard and scores nodes in rendezvous hashing. The algorithm is unspecified by the standard library | `src/core/cache.rs` (shard index); `src/dist/router.rs::calculate_hash` | IMPLEMENTED; portability risk | Ch 4, section 4.2; Ch 5, section 5.3 |
| rustls and tokio-rustls | TLS 1.3 for cluster mTLS, origin HTTPS and ingress. Pure Rust, no OpenSSL | `src/security/cluster.rs::ClusterTlsConfig` | IMPLEMENTED, TESTED | Ch 3, section 3.7; Ch 5, section 5.2 |
| `ring` | SHA-256 for certificate fingerprints, the anti-entropy digest and lab hashes | `src/dist/anti_entropy.rs::InvalidationRingBuffer::compute_digest` | IMPLEMENTED | Ch 3, section 3.1 |
| `jsonwebtoken` | Verifies HS256 tokens: signature, expiry, issuer, audience | `src/security/jwt.rs::JwtValidator::validate` | IMPLEMENTED, TESTED | Ch 3, section 3.8; Ch 6, section 6.2 |
| serde and serde_json | JSON for API bodies, JWT claims and lab artifacts | `src/api/server.rs`; `src/security/jwt.rs`; `src/lab/` | IMPLEMENTED | Ch 3, section 3.9 |
| PhantomMesh frames | Custom binary protocol: magic `PHAN`, 28-byte header, payload up to 1 MiB, key up to 4,096 bytes | `src/dist/frame.rs` | IMPLEMENTED, TESTED | Ch 3, section 3.9; Ch 5, section 5.2 |
| Upstream policy, DNS pinning, IP filter | Stops the origin fetch reaching private or metadata addresses | `src/security/upstream.rs::resolve_and_pin_origin`, `PreConnectIpFilter` | IMPLEMENTED, TESTED | Ch 6, section 6.8 |
| Static cluster membership file | Fixed member list with certificate fingerprints, read once at startup | `src/security/cluster.rs::load_membership_from_file` | IMPLEMENTED, TESTED | Ch 5, section 5.2 |
| React, Vite, hand-built SVG | The dashboard and Lab screens. A separate dev server, not served by the gateway | `ui/src/App.jsx` | IMPLEMENTED, TESTED (automated checks only) | Ch 3, section 3.11; Ch 7, section 7.7 |
| Test tools: proptest, reqwest, rcgen, hdrhistogram | Property tests, real HTTP in tests, in-memory certificates, bench latency recording. `criterion` is declared but unused | `tests/`; `benches/` | IMPLEMENTED | Ch 3, section 3.12 |


### C.2 Algorithms and mechanisms

| Mechanism and what it does | Cost or complexity | Where (`path::symbol`) | Status; failure mode in brief | Chapter |
|---|---|---|---|---|
| Sharded mutexes: hash the key, lock one of 8 buckets | Lookup O(key length) plus expected O(1); at most 8-way parallelism; contention not measured | `src/core/cache.rs::PartitionedCache::new` (8 in `main.rs`) | IMPLEMENTED, TESTED; a hot key cannot spread, long critical sections block a shard | Ch 4, section 4.2 |
| Single-flight coalescing: first miss leads, others wait and share | O(1) per request; at most 1,000 waiters per key | `src/core/cache.rs::begin_fetch_atomic`; `src/core/coalesce.rs::SharedOp` | IMPLEMENTED, TESTED; waiters share the leader's fate, per gateway only | Ch 4, section 4.4 |
| Generation fencing: store a fetch result only if the key's generation is unchanged | O(1) under the shard lock | `src/core/cache.rs::finalize_fetch`, `delete_impl` | IMPLEMENTED, TESTED; tested on one interleaving, no model checker | Ch 4, section 4.5 |
| Epoch fencing: tenant and global version stamps; invalidate by raising the epoch | Tenant bump O(T), T at most 10,000 | `src/core/cache.rs::try_invalidate_tenant`, `get_effective_epochs` | IMPLEMENTED, PARTIAL; G4A-01 race (fix not integrated), G4A-08, G4A-17 | Ch 4, section 4.7 |
| Epoch overflow fallback: a new tenant on a full map raises the global epoch | Flushes the whole node, O(T) | `src/core/cache.rs::MAX_TENANT_EPOCHS` | IMPLEMENTED, TESTED; a peer can trigger it (G4A-12) | Ch 4, section 4.7 |
| TTL and freshness: lifetime from `s-maxage`, `max-age`, `Expires`, else 60 s | O(1) per response | `src/core/freshness.rs::evaluate_response_freshness`; literal `60_000` in `src/api/server.rs` | IMPLEMENTED, PARTIAL; `PHANTOM_DEFAULT_TTL_MS` is NOT WIRED, no ceiling on origin lifetimes | Ch 4, section 4.8 |
| Stale serving and hard cap: serve expired data only inside origin-declared windows, capped at `PHANTOM_MAX_STALE_MS` | O(1) per check | `src/core/cache.rs::check_stale_serving` | IMPLEMENTED, TESTED; cap counter is weak (G4A-18), cap bounds stale window only | Ch 4, section 4.8 |
| Revalidation with 304: refresh a stored entry's lifetime | O(1) | `src/core/cache.rs::complete_not_modified` | IMPLEMENTED, TESTED; validators dropped early by the sweeper (G4A-16) | Ch 4, section 4.8 |
| Lazy sweeper: reclaim stale and expired entries in the background | Scans each shard in full under its lock every tick | `src/core/cache.rs::spawn_sweeper_task`, `sweep_lazy_reclamation` | IMPLEMENTED, PARTIAL; lock held for the whole scan (G4A-03) | Ch 4, section 4.7 |
| SIEVE (default): queue plus visited bit plus a moving hand | Hit O(1); eviction amortized O(1) with no pinned keys | `src/core/eviction.rs::FaithfulSieve` | IMPLEMENTED, TESTED (weakly); pinned keys make a scan costly (G4A-20) | Ch 4, section 4.9 |
| LRU: recency list, victim at the back | O(1); worst case O(p) for p pinned keys | `src/core/eviction.rs::FaithfulLru` | IMPLEMENTED, TESTED (weakly); every hit rewrites the list | Ch 4, section 4.9 |
| S3-FIFO: small, main and ghost queues | Amortized O(1) (our reasoning) | `src/core/eviction.rs::FaithfulS3Fifo` | IMPLEMENTED, PARTIAL (defect G4A-02 OPEN); queues can outgrow live keys, not the default | Ch 4, section 4.9 |
| Byte accounting: running logical byte count per shard, with an audit | O(1) per update; audit walks all entries | `src/core/cache.rs::calculate_state_bytes`, `audit_bytes` | IMPLEMENTED, TESTED (consistency only); drift on a panic path (G4A-13), uncounted structures | Ch 4, section 4.6 |
| Origin concurrency gate: a permit counter, released on drop | O(1) atomic | `src/intelligence/executor.rs::OriginGate`, `origin_cap_enforcing` | IMPLEMENTED; enforced only in canary or active with a cap, uncapped by default | Ch 4, section 4.4; Ch 5, section 5.13 |
| AIMD cap controller: multiply by 0.70 on a bad window, else add 1 | O(1) per observation; steps once per window | `src/intelligence/candidates.rs::OriginConcurrencyCapController` | IMPLEMENTED, TESTED (arithmetic and cap unit tests); on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT established, behaved as a static cap | Ch 5, section 5.13; Ch 7, section 7.6 |
| Log-linear latency histogram: 2,688 buckets, bit tricks | O(1) record; design error at most 0.39 percent | `src/intel/telemetry.rs::AtomicLogLinearHistogram` | IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader | Ch 5, section 5.11 |
| Count-Min sketch (4 by 1,024) in the heavy-hitter detector | O(d) per item, overestimates only | `src/intelligence/signals.rs::HeavyHitterDetector` | PROTOTYPE ONLY (library); not wired. The older `src/intel/sketch.rs::CountMinSketch` is used only by a unit test | Ch 5, section 5.12 |
| Space-Saving top-k (k = 64) | O(k) per miss | `src/intelligence/signals.rs::HeavyHitterDetector` | PROTOTYPE ONLY (library); metrics report hot keys as unavailable | Ch 5, section 5.12 |
| EWMA, CUSUM burst, Zipf estimate | O(1) per window; Zipf over the top 50 | `src/intelligence/signals.rs` | PROTOTYPE ONLY (library); working-set size assumes 1 KiB per object | Ch 5, section 5.12 |
| Shadow evaluation and decision engine | Replay on a sampled stream (one in R keys) | `src/intelligence/shadow.rs::ShadowEvaluator`; `src/intelligence/engine.rs::PieDecisionEngine` | IMPLEMENTED as library, not wired; unsound intervals (F12, F13) | Ch 5, section 5.13 |
| Rendezvous hashing: highest score of (key, node) wins, equal weights | O(N) per key for N members | `src/dist/router.rs::RendezvousRouter::route_key` (`capacity_weight = 1.0`) | IMPLEMENTED; weighted version DESIGNED BUT NOT IMPLEMENTED; routes `DELETE` only, `GET` is local | Ch 5, section 5.3 |
| Invalidation fan-out: per-peer queue of 256, up to four attempts | Bounded queues and attempts | `src/dist/router.rs::invalidate_key_across_cluster`, `fanout_targets` | IMPLEMENTED, TESTED; 204 is not a cluster-wide acknowledgement | Ch 5, section 5.5 |
| Repair ledger: bounded record of owed invalidations | 1,024 entries per peer, collapses to a coarser flush | `src/dist/router.rs::RepairLedger` | IMPLEMENTED, TESTED; memory only, lost if the issuer dies | Ch 5, sections 5.5 and 5.7 |
| Anti-entropy: ring of sent records, sequence gaps, pull replay | Ring of 4,096 records; digest is SHA-256 | `src/dist/anti_entropy.rs::InvalidationRingBuffer`, `AntiEntropyTracker`; `src/dist/router.rs::reconcile_peer_anti_entropy` | IMPLEMENTED, TESTED; effectiveness beyond loopback UNVERIFIED | Ch 5, section 5.5 |
| mTLS admission: TLS 1.3, cluster CA, one SAN, fingerprint pinning, originator match | Per connection and per frame check | `src/security/cluster.rs::ClusterTlsConfig`, `validate_peer`; `src/dist/node.rs::start_tcp_service` | IMPLEMENTED, TESTED; no revocation trigger (G4B-07), members are trusted (G4B-05) | Ch 5, section 5.2; Ch 6, section 6.4 |
| HS256 JWT with roles and tenant claim | One signature check per request | `src/security/jwt.rs::JwtValidator::validate`; `src/api/server.rs::authenticate_request` | IMPLEMENTED, TESTED; no revocation, no `nbf`, shared secret can mint any token | Ch 6, section 6.2 |
| SSRF pre-connect filter with DNS pinning | One resolve and filter per origin fetch | `src/security/upstream.rs::resolve_and_pin_origin` | IMPLEMENTED, TESTED; range gaps open (G4B-15) | Ch 6, section 6.8 |
| Discrete-event simulation (T1): virtual time, event heap with sequence tie-break | O(log n) per event (heap) | `src/lab/t1_sim.rs::run_cell`; `src/lab/events.rs::EventQueue` | IMPLEMENTED, TESTED (determinism on one machine); a model of the origin, not the HTTP gateway | Ch 7, section 7.2 |
| Seeded random streams and alias sampling | One stream per purpose, seeded | `src/lab/rng.rs::MasterRng`, `AliasSampler` | IMPLEMENTED, TESTED | Ch 7, section 7.2 |
| BCa bootstrap, sign-flip permutation test, Holm correction | B = 10,000 resamples; exact sign-flip for n at most 16 | `src/lab/analysis.rs::bca_ci_mean`, `sign_flip_p_value`, `HolmBonferroni` | IMPLEMENTED, TESTED on synthetic data; not cross-checked against a reference | Ch 7, section 7.3 |


### C.3 How to use the tables in a defence

1. If a judge names a mechanism, find its row, say the status label first, then the failure mode. Do not state a label stronger than the one here.
2. Every complexity statement here is arithmetic or code reading. If asked for a number, say there is no release-profile measurement, and point to Chapter 6, section 6.13 for the few measurements that exist and their limits.
3. If a row says "library" or "not wired", say so before the judge asks. The honest answer is the strong one.

## Appendix G: Implementation versus roadmap matrix

This appendix presents the matrix from Chapter 8, section 8.12, with one added column that points to the chapter that teaches each capability. There are 44 rows. Each row carries exactly one status label, and the labels, capability names and "Why" text are unchanged from Chapter 8. Only the grouping is new: the rows are grouped under the three pillars of Chapter 1 (distributed systems engineering, workload intelligence, product excellence with observability) and, inside the first pillar, by area. Nothing here is INDEPENDENTLY VERIFIED by a human; the labels describe the state of the working tree and the project records on 2026-10-30.


### G.1 Status vocabulary

| Label | Meaning in this handbook |
|---|---|
| IMPLEMENTED | Code exists. This says nothing about whether it works. |
| TESTED | A named test exercises it. The chapter says what the test does not prove. |
| INDEPENDENTLY VERIFIED | A human or external reviewer other than the author checked it. No item in the project qualifies; AI reviews are "AI second looks". |
| PARTIAL | Part of the capability exists, with named gaps. |
| PROTOTYPE ONLY | Exists in a library, a sandbox or a scratch patch, but is not integrated into the product path. |
| DESIGNED BUT NOT IMPLEMENTED | There is a design or a proposal, and no working code. |
| DEFERRED | Not done yet; it is queued behind a condition, a decision or a quiet machine. |
| BLOCKED | Cannot proceed without a resource the project lacks, such as physical devices or human reviewers. |
| UNVERIFIED | No check has been run, or none could be run; the reason is stated. |

Label counts across the 44 rows: TESTED 15, PARTIAL 8, DEFERRED 6, DESIGNED BUT NOT IMPLEMENTED 5, PROTOTYPE ONLY 4, UNVERIFIED 3, BLOCKED 2, IMPLEMENTED 1. INDEPENDENTLY VERIFIED appears in no row. Reading rule: TESTED means IMPLEMENTED and a named test exercises it (Chapters 4 and 5 write "IMPLEMENTED, TESTED"; same label); "IMPLEMENTED, PARTIAL" is written PARTIAL.


### G.2 Pillar 1: Distributed systems engineering

#### G.2.1 Single-node cache engine

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Single-flight coalescing | TESTED | Named integration tests; coalescing under cancellation reviewed only by reading | Ch 4, 4.4 |
| Generation fencing against DELETE | TESTED | `test_delete_fences_in_flight_fill_race`; a single interleaving | Ch 4, 4.5 |
| Tenant and global epoch fencing | PARTIAL | IMPLEMENTED, but the G4A-01 race is reproduced in a sandbox and its fix is not integrated; G4A-08 and G4A-17 open | Ch 4, 4.7 |
| Byte accounting (I1 to I8) | TESTED | `audit_bytes` equality tests; G4A-13 drift on a rare path | Ch 4, 4.6 |
| Freshness (RFC 9111 subset) | PARTIAL | `Age`, `Expires: 0`, stale-if-error scope gaps (G4A-04 to -06) | Ch 4, 4.8 |
| Default TTL setting | PARTIAL | Validated, not wired | Ch 2, 2.6.1; Ch 4, 4.8 |
| Stale-while-revalidate, stale-if-error, hard stale cap | TESTED | Counter-based gate is weak (G4A-18) | Ch 4, 4.8 |
| SIEVE (default) and LRU eviction | TESTED | Weakly: pinned scans are not O(1) (G4A-20); no hit-ratio check against the literature; no release benchmark | Ch 4, 4.9 |
| S3-FIFO eviction | PARTIAL | IMPLEMENTED with the open defect G4A-02 (tombstones); not selected by default | Ch 4, 4.9 |
| Lazy sweeper | PARTIAL | Scans the shard under the lock; shutdown channel dropped (D-2) | Ch 4, 4.7 |

#### G.2.2 Security, ingress and tenancy

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| HTTP ingress hardening | PARTIAL | G4B-01, -02, -04, -11 open | Ch 6, 6.5 |
| JWT roles, mTLS identity | TESTED | M7 security tests; lifecycle gaps open | Ch 6, 6.2 and 6.4 |
| SSRF upstream filter | TESTED | Range gaps (G4B-15) | Ch 6, 6.8 |
| Rate limiting | DEFERRED | SEC-M7-11 queued | Ch 6, 6.8; Ch 8, 8.3.7 |
| Tenant keyspace isolation | TESTED | Tenant-keyed entries and epochs | Ch 6, 6.6 |
| Tenant quotas, fair eviction | DESIGNED BUT NOT IMPLEMENTED | Frozen core | Ch 6, 6.6; Ch 8, 8.9.2 |

#### G.2.3 Cluster and distribution

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Cluster invalidation fan-out, repair, anti-entropy | TESTED | IMPLEMENTED and tested on loopback, debug build, at most four nodes; labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED; 204 is not a cluster-wide acknowledgement (G4B-09) | Ch 5, 5.5 |
| Durable invalidation | PROTOTYPE ONLY | Sandbox patch, not integrated | Ch 5, 5.9; Ch 8, 8.9.1 |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`), but no code path lets a heartbeat change node state (G4B-12) | Ch 5, 5.2 |
| Runtime membership and revocation | PARTIAL | Machinery, no production trigger | Ch 5, 5.2; Ch 6, 6.4 |
| Rendezvous routing for DELETE primary | IMPLEMENTED | Unweighted, unstable hash | Ch 5, 5.3 |
| Peer-fill, coordinated reads, stable placement | DESIGNED BUT NOT IMPLEMENTED | A stub handler exists and is a defect, not the feature | Ch 5, 5.8; Ch 8, 8.3.1 |
| Multi-host operation | BLOCKED | No devices | Ch 8, 8.9.3 |


### G.3 Pillar 2: Workload intelligence

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Static origin concurrency cap | TESTED | Scenario B and unit tests | Ch 4, 4.4; Ch 5, 5.13 |
| AIMD cap controller | TESTED | Arithmetic and cap unit tests only; on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT SUPPORTED | Ch 5, 5.13 and 5.14 |
| Signals, shadow evaluator, decision engine | PROTOTYPE ONLY | Not wired; unsound statistics (F12, F13) | Ch 5, 5.12 and 5.13 |
| Legacy PIE | PROTOTYPE ONLY | `pie: None` | Ch 5, 5.13 |
| Intelligence `canary` mode | PARTIAL | Cap only; no canary or rollback in serving | Ch 5, 5.13 |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY | Library in `src/intelligence/signals.rs`, unit-tested and used by the lab simulator's admission filter, not wired into the serving path; the older `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test | Ch 5, 5.12 |
| Event tap, pre-warming, forecasting | DESIGNED BUT NOT IMPLEMENTED | Experiments E0, E9, E10 first | Ch 8, 8.5 and 8.9.4 |
| ML selector | DEFERRED | Gate unmet; negative claim unverified | Ch 5, 5.10; Ch 8, 8.5 |


### G.4 Pillar 3: Product excellence, observability and demonstrability

#### G.4.1 Telemetry, Lab, evidence and UI

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Telemetry log-linear histogram | TESTED | Recording and bucket arithmetic only (`telemetry_redesign_tests`); the histogram is fed but has no production percentile reader (Chapter 6, 6.12.2) | Ch 5, 5.11; Ch 6, 6.12.2 |
| Lab API and Scenario B | TESTED | Exploratory; U-01 defect open | Ch 7, 7.5 and 7.7 |
| Sealed M6 evaluation | TESTED | Executed once, Tier 1 simulated, not human-reviewed | Ch 7, 7.6; Ch 5, 5.14 |
| T1 to T2 (simulator to real) agreement | UNVERIFIED | No agreement study run | Ch 7, 7.5.3 |
| Release-profile performance measurements | DEFERRED | Needs a quiet machine | Ch 6, 6.12 and 6.13 |
| Dashboard UI I1 to I4 | TESTED | Automated checks only | Ch 3, 3.11; Ch 7, 7.7.2 |
| Visual acceptance of the UI (O8-2) | UNVERIFIED | Owner decision open | Ch 8, 8.2.3 and 8.9.5 |
| Demo Center and `demo-1` stream | DESIGNED BUT NOT IMPLEMENTED | Proposal only | Ch 8, 8.9.5 |
| Human review of design, usability, security | BLOCKED | No human reviewer | Ch 8, 8.2.3 and 8.13.1 |

#### G.4.2 Operations and release

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Graceful connection drain | DEFERRED | Integration item D-1 | Ch 8, 8.3.7 and 8.5 |
| Linux and container deployment | UNVERIFIED | Never run | Ch 2, 2.9.3; Ch 3, 3.10 |
| Metrics export, TLS and config reload | DEFERRED | Not provided | Ch 8, 8.3.7 and 8.5 |
| Dependency advisory scan and SBOM | DEFERRED | Queued | Ch 3, 3.13; Ch 6, 6.10 |


### G.5 Reconciled labels

Earlier drafts gave four capabilities different labels in Chapter 5, Chapter 4 and Chapter 8. They were reconciled before this appendix was regenerated, and Chapter 8, 8.12 is the source of the rows above. The decisions, so that a reader who meets an older copy is not surprised:

::: caution
**Heartbeat.** Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`). Heartbeat-based failure detection and state change is DESIGNED BUT NOT IMPLEMENTED (G4B-12): no code path lets a heartbeat change a node's state. The same split is used in Chapters 2, 5 and 8 and in Appendices D and F.

**Hot-key detection.** `HeavyHitterDetector` (Count-Min plus Space-Saving) exists, is unit-tested and is used by the lab simulator's admission filter, but no request handler or background task calls it: PROTOTYPE ONLY (library). Only the event tap, pre-warming and forecasting are DESIGNED BUT NOT IMPLEMENTED.

**Telemetry log-linear histogram and AIMD cap controller.** Both are TESTED in the sense above (histogram: recording and bucket arithmetic; AIMD: arithmetic and cap unit tests). The histogram is fed but has no production percentile reader. The AIMD gate is on the serving path only when a cap is configured in canary or active mode (default off), and its benefit is NOT SUPPORTED.

**Fencing, eviction, cluster.** Generation fencing is TESTED (one interleaving); epoch fencing is PARTIAL (G4A-01 race, fix not integrated). SIEVE and LRU are TESTED weakly; S3-FIFO is PARTIAL (open defect G4A-02). The cluster fan-out, repair and anti-entropy are IMPLEMENTED and TESTED on loopback only, labelled EXPERIMENTAL in `main.rs`, effectiveness UNVERIFIED.
:::

## Tables extracted from Part I sections <!--gen-->

### 1.6 Pillar 1: Distributed systems engineering (tables) <!--gen-->

| Level | Capability | Status | Where | What the evidence does and does not show |
|---|---|---|---|---|
| A. Single node | Sharded map, per-shard mutex, byte budget, eviction behind a trait (SIEVE default, LRU, S3-FIFO selectable) | IMPLEMENTED, TESTED | `src/core/cache.rs::PartitionedCache`, `CacheShard::new` (Sieve), `src/core/eviction.rs` | Unit and integration tests pass in a debug build. Not measured on a release build; no soak run. |
| A. Single node | Single-flight coalescing | IMPLEMENTED, TESTED | `begin_fetch_atomic`, `SharedOp` | 100-client barrier test, one origin fetch. Loopback only. |
| A. Single node | Generation and epoch fencing, byte-accounting invariants | IMPLEMENTED, TESTED | `finalize_fetch`, `delete`, `audit_bytes()` | Code-reading reviews by AI only. G4A-01 (a stale-body race) UNDER-INVESTIGATION. |
| A. Single node | HTTP freshness (RFC 9111 subset), stale cap | IMPLEMENTED, TESTED | `src/core/freshness.rs` | Default TTL setting NOT WIRED; no ceiling on origin-set lifetimes. |
| B. Multi-node foundation | mTLS identity, framing, pooled connections, static membership | IMPLEMENTED, TESTED | `src/security/cluster.rs`, `src/dist/frame.rs`, `pool.rs`, `src/main.rs` | Loopback, debug, Windows. Identity-only authorisation. No certificate rotation. |
| B. Multi-node foundation | Key invalidation fan-out, repair ledger, anti-entropy worker | IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED | `src/dist/router.rs`, `anti_entropy.rs`, `node.rs` | Claim of zero stale after 50 ms is recorded as unsupported (claims ledger C6). |
| B. Multi-node foundation | Durable invalidation (survives issuer crash) | NOT IMPLEMENTED in the tree; patch PROTOTYPE ONLY in `docs/research/m3_durability/` | n/a | Defect reproduced by the patch author, not independently. |
| C. Coordinated reads | Peer reads, pooled capacity, stable placement | NOT IMPLEMENTED (DESIGNED BUT NOT IMPLEMENTED as a goal) | cluster `GetRequest` handler is a stub | `docs/research/vision/` marks this a gated goal. |
| D. Intelligent distribution | Cluster-aware intelligence, coordinated pre-warming | NOT IMPLEMENTED | no file under `src/intelligence/` or `src/intel/` mentions a router or peer (reported by the audit) | n/a |

### 2.3 The HTTP surface (tables) <!--gen-->

| Method and path | Role needed when JWT is on | Purpose | Status |
|---|---|---|---|
| `GET /health` | none | plain text `OK` | IMPLEMENTED |
| `GET /health/live` | none | `200 {"status":"alive"}` | IMPLEMENTED; TESTED (`m7_health_tests::m7_live_is_always_200_and_ready_tracks_state`) |
| `GET /health/ready` | none | `200` only when config is valid, the listener answered a TCP probe, and the process is not draining; else `503` with the failing checks | IMPLEMENTED; TESTED (same test) |
| `GET /api/v1/cache/*key`, `GET /api/v1/cache` | `cache:read`, and a non-empty `tenant_id` claim (not `__default__`) | read-through cache lookup | IMPLEMENTED; TESTED (`m1_http_tests`) |
| `DELETE /api/v1/cache/*key`, `DELETE /api/v1/cache` | `cache:invalidate`, and a tenant claim | remove one exact key (path plus raw query string) | IMPLEMENTED; TESTED |
| `GET /api/events` | `telemetry:read` | server-sent events every 100 ms; `?schema=2` is current, schema 1 is deprecated (`Deprecation` and `Sunset: Wed, 31 Dec 2026` headers) | IMPLEMENTED |
| `GET /api/v1/metrics` | `telemetry:read` | one JSON snapshot, schema 2 by default | IMPLEMENTED |
| `GET /api/v1/diagnostics/summary`, `/api/v1/diagnostics/events` | `telemetry:read` | counters snapshot; last 100 diagnostic events | IMPLEMENTED |
| `GET /api/v1/intelligence/status`, `/api/v1/intelligence/audit` | `telemetry:read` | state and audit log of the intelligence executor | IMPLEMENTED |
| `POST /api/control` | `scenario:operate` | body `{"event": ...}`; `FLASH_CROWD` sets a workload label, `NODE_FAILURE` sets node 2 to `Failed`, anything else resets labels and sets every node `Active` | IMPLEMENTED; it flips in-memory flags only |
| `GET /api/v1/lab/capabilities`, `/evidence`, `/evidence/:id`, `/runs`, `/runs/:id`, `/runs/:id/events` (SSE), `/runs/:id/artifacts`, `/runs/:id/artifacts/:name` | `telemetry:read` | read Lab state and retained evidence | IMPLEMENTED; exist only if `PHANTOM_LAB_ENABLED=true` |
| `POST /api/v1/lab/runs`, `POST /api/v1/lab/runs/:id/cancel` | `scenario:operate` | start and cancel experiments | IMPLEMENTED; same condition |

| `X-Cache-Status` | Meaning | Set in |
|---|---|---|
| `HIT` | fresh entry served from memory | `get_cache_handler`, `FetchRegistrationResult::Hit` branch |
| `MISS` | this request was the leader that fetched the origin | `Initiate` branch |
| `COALESCED` | this request waited for another request's fetch | `Wait` branch |
| `STALE` | an expired entry was served under a stale directive, or the leader's fetch failed and stale-if-error applied | stale pre-check, and `is_stale` in the `Wait` and `Initiate` branches |
| `BYPASS` | client sent `x-cache-bypass: true`; origin fetched, nothing stored | bypass branch |
| `BYPASS-AUTH` | a credential was presented and the gateway forwards credentials upstream, so reads skip the cache | bypass branch |

### 2.9 Configuration and deployment (tables) <!--gen-->

| Key | Default | Wiring | Notes |
|---|---|---|---|
| `PHANTOM_MODE` | `development` | Main | `production` adds strict rules |
| `PHANTOM_BIND_ADDR` | `127.0.0.1:8085` | Env | non-loopback requires JWT and TLS (or the insecure-HTTP override) |
| `PHANTOM_TLS_CERT`, `PHANTOM_TLS_KEY` | unset | Env | ingress TLS; set together |
| `PHANTOM_ALLOW_INSECURE_HTTP` | `false` | Env | plaintext on a network bind; JWT still mandatory |
| `PHANTOM_REQUIRE_AUTH` | `false` | Env | startup fails unless a JWT secret exists |
| `PHANTOM_JWT_SECRET` | unset | Main | HS256 shared secret; enables RBAC; 32 bytes or more required in production |
| `PHANTOM_JWT_ISSUER` / `PHANTOM_JWT_AUDIENCE` | `phantom-cache-auth` / `phantom-cache-api` | Main | expected claims |
| `PHANTOM_MAX_CACHE_BYTES` | 268435456 (256 MiB) | Main | logical byte budget, split evenly across 8 shards |
| `PHANTOM_DEFAULT_TTL_MS` | 60000 | **NOT WIRED** | literal `60_000` in `server.rs` (section 2.6.1) |
| `PHANTOM_MAX_STALE_MS` | 86400000 (24 h) | Main | hard cap on stale serving |
| `PHANTOM_SWEEPER_BATCH_SIZE` | 32 | Main | see discrepancy D-5 |
| `PHANTOM_SWEEPER_INTERVAL_MS` | 100 | Main | sweeper tick |
| `ORIGIN_URL` | unset | Env (`main` also copies it into `ApiState.origin_url`) | not `PHANTOM_`-prefixed; only host and port are used |
| `PHANTOM_ORIGIN_ALLOWLIST` | unset | Env | non-empty means the origin must match |
| `PHANTOM_ENFORCE_UPSTREAM_TLS`, `PHANTOM_ALLOW_INSECURE_UPSTREAM`, `PHANTOM_ALLOW_LAN_ORIGIN`, `PHANTOM_FORWARD_UPSTREAM_AUTH` | all `false` | Env | upstream policy switches |
| `PHANTOM_UPSTREAM_CA_CERT` | unset | Env | extra CA for upstream TLS |
| `PHANTOM_CLUSTER_ID` | `phantom-cluster` | Env | must equal the label in certificate SANs |
| `PHANTOM_CLUSTER_MEMBERS_FILE` | unset | Env | static membership and fingerprints |
| `PHANTOM_CLUSTER_CA`, `PHANTOM_NODE_CERT`, `PHANTOM_NODE_KEY` | unset | Env | cluster mTLS material |
| `PHANTOM_NODE_ID` | 1 | Env | must exist in the registry |
| `PHANTOM_CLUSTER_LISTEN_ADDR` | unset | Env | required when mTLS is configured |
| `PHANTOM_CLUSTER_PEER_ADDRS` | unset | Env | `id=ip:port,...` overrides membership addresses |
| `PHANTOM_ALLOW_INSECURE_CLUSTER`, `PHANTOM_STRICT_CLUSTER` | `false` | Env | insecure cluster flag (leads to the no-frames mode); strict loopback rule |
| `PHANTOM_CLUSTER_TIMEOUT_MS` | 150 | Env | per-attempt invalidation timeout |
| `PHANTOM_INTELLIGENCE` | `off` | Main | `off`, `observe`, `advisory`, `canary`, `active` |
| `PHANTOM_ORIGIN_CAP_MAX` | unset | Main | unset means no cap configured; enforced only in `canary` and `active` |
| `PHANTOM_ORIGIN_CAP_MIN`, `_P99_CEILING_MS`, `_WINDOW` | 1, 1000, 100 | Main | AIMD controller parameters |
| `PHANTOM_SIMULATION` | `false` | Env | produces synthetic dashboard values |
| `PHANTOM_CORS_ORIGINS` | unset | Main | exact origins; development profile defaults to `http://localhost:3000` and `http://127.0.0.1:3000` |
| `PHANTOM_LAB_ENABLED`, `PHANTOM_LAB_DIR`, `PHANTOM_LAB_GATEWAY_EXE` | `false`, `lab-results`, unset | Main | Lab API switches |
| `PHANTOM_CONFIG` | unset | Main | config file path (same as `--config`) |

### 3.10 Build, packaging and deployment: what exists and what does not (tables) <!--gen-->

| Item | Exists? | Evidence |
|---|---|---|
| `Dockerfile`, `docker-compose`, Kubernetes manifests, Helm | **No** | searched the repository outside `node_modules` for `Dockerfile*` and `docker-compose*`: none; grep in `packaging/`, README, `docs/user` found no container references |
| Release build scripts | Yes | `packaging/build-release.sh` / `.ps1`: `cargo build --release --locked --bin phantom-cache-demo`, stamps `PHANTOM_BUILD_COMMIT` and `PHANTOM_BUILD_DATE` (read via `option_env!` in `src/config.rs`), packs a `.tar.gz` with checksums; "does NOT commit, tag, push or publish" |
| Smoke test of a bundle | Yes | `packaging/smoke-test.sh` / `.ps1`: `--version`, `--check-config`, rejects `PHANTOM_REQUIRE_AUTH=yes`, starts on a loopback port and probes `/health`, `/health/live`, `/health/ready` |
| Example configs | Yes | `packaging/phantom.example.conf`, `phantom.production.example.conf` (KEY=VALUE; precedence command line > environment > file > defaults) |
| Demo launcher | Yes (Windows) | `Start-Phantom-Demo.bat` (checks node/npm, port 8085) |
| Binaries | Two | `phantom-cache-demo` (`src/main.rs`) and `phantom-lab` (`src/bin/phantom-lab.rs`) |
| Process manager (systemd unit etc.) | **No** | not found |
| CI | A workflow file exists | `.github/workflows/ci.yml`: `ubuntu-latest`; `cargo fmt --check`, `cargo check`, `clippy -D warnings`, `cargo test` (note: **no `-j 1`, debug profile, all features**); frontend job: Node 20, `npm ci`, `npm run build` (it does **not** run `npm test`). Whether it ever ran on GitHub: UNVERIFIED (no run history in the repo) |
| Release-profile benchmarks | No result on record | `bench-results/README.md` |
| Clean-machine reproduction | Not done | `docs/STATUS.md` (M7 row) |

### 5.10 Workload intelligence: what exists and where it runs (tables) <!--gen-->

| Component | File | What it is | On the serving path? | Status |
|---|---|---|---|---|
| Latency histogram `AtomicLogLinearHistogram` | `intel/telemetry.rs` | 2,688-bucket histogram | **Written** on every GET; **not read** by any production endpoint (section 5.11) | IMPLEMENTED, TESTED (bucket error) |
| `TelemetryCollector` raw window | `intel/telemetry.rs` | 10,000-sample ring, sorted on demand | Yes: this is what `/api/v1/metrics` percentiles use | IMPLEMENTED, TESTED |
| `OriginMetrics` | `intel/origin_metrics.rs` | 16-bucket origin latency histogram, in-flight and error counters | Yes | IMPLEMENTED |
| AIMD origin gate `OriginGate` + `OriginConcurrencyCapController` | `intelligence/executor.rs`, `candidates.rs` | Adaptive cap on concurrent origin fetches | **Yes, only if `PHANTOM_ORIGIN_CAP_MAX` is set and mode is `canary` or `active`** | IMPLEMENTED, TESTED; benefit NOT established |
| `PHANTOM_INTELLIGENCE` mode switch, `/api/v1/intelligence/status` | `main.rs`, `server.rs` | Reads mode, active candidate id, revert count | Yes (read-only) | IMPLEMENTED |
| `/api/v1/intelligence/audit` and `AuditStore` | `intelligence/audit.rs` | 100-record ring of decisions | Endpoint served; **nothing in `src/` appends to the store** | PARTIAL (empty in production) |
| `WorkloadPipeline`: rate EWMA, Count-Min + Space-Saving heavy hitters, Zipf estimate, working set, CUSUM burst, hot-key emergence | `intelligence/signals.rs` | Detectors | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_signals_tests.rs`) |
| `ShadowEvaluator` and shadow LRU/SIEVE/S3-FIFO/TinyLFU | `intelligence/shadow.rs` | Counterfactual replay on a key sample | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_shadow_tests.rs`) |
| `PieDecisionEngine` | `intelligence/engine.rs` | Guardrails + confidence interval verdict | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_engine_tests.rs`); statistics unsound (F12) |
| Canary routing `route_bucket`, `check_watchdog`, `safe_apply_policy`, `apply_policy` | `intelligence/executor.rs` | Stratified arms, stall revert, panic revert | **No** (no handler calls them) | IMPLEMENTED as library; TESTED (`intel_executor_tests.rs`, `intel_safety_and_replay_tests.rs`) |
| Other candidates: stale-window controller, admission filter, rate forecaster, hot-key predictor, decision stump | `intelligence/candidates.rs` | Library | No; the lab simulator uses the stale controller and admission filter | IMPLEMENTED as library |
| `CountMinSketch`, `WorkloadProfile` | `intel/sketch.rs`, `intel/workload.rs` | Early sketch and workload presets | **No**; only `tests/unit/wp2_tests.rs` uses them | PROTOTYPE ONLY |
| Legacy `PieController` | `control/pie.rs` | Threshold stub | **No**: `pie: None`; never constructed | Legacy, quarantined |

### 8.2 Where the project stands (tables) <!--gen-->

| Fact | Value | How obtained | Label |
|---|---|---|---|
| HEAD | `804a7b8` on branch `feat/publish-phantomcache-core`, at the time of writing (2026-10-30, about 17:10; it was `df68eaa` at about 16:35 and `caeedfd` earlier the same afternoon). Local branch and its remote-tracking branch point at the same commit (all pushed) | `git rev-parse HEAD`, `git branch --show-current` | reproduced here |
| Commits since the first published snapshot `456bb25` | six, all documentation or tooling, no Rust: `8523cc1` (apple-design skill adoption), `3e6e03e` (docs-lint tool), `caeedfd` (four animation skills), `df68eaa` (`.gitattributes` protecting four hash-pinned frozen files from line-ending conversion), `9ab179f` (research records, phase reports, audits and plans), `804a7b8` (product, security, handoff and release documentation, docs index and historical notices) | `git log --oneline -8` | reproduced here |
| Tracked files modified, not committed | 22 (at the time of writing; the Rust, UI and test files modified by the redesign and earlier phases) | `git status --short`, status column | reproduced here |
| Untracked entries | 89 collapsed entries (`git status --short` prints 111 lines in total, tracked and untracked); about 4,510 entries with `-uall` at the time of writing (without `-uall` git collapses directories; the count moves as agents write) | same | reproduced here |
| Stashes | 0 | `git stash list` | reproduced here |
| Rust code committed beyond the first snapshot | none. The modified tracked files include `src/api/server.rs`, `src/core/cache.rs`, `src/dist/router.rs`, `src/main.rs` and others; the `.gitattributes` commit touches no Rust | `git status` list | reproduced here |

| Suite | Reported result | Source and limits |
|---|---|---|
| Rust, `cargo test --tests -j 1 --no-fail-fast` | 851 passed, 0 failed, 8 ignored, 75 targets | `docs/release/verification_register.md` V-01, lead re-run on 2026-10-30 between 10:11 and 10:23. Debug profile, Windows, single host. The tree has not changed since by file time (V-04; modification time is not a content hash). Not run by me. |
| Rust, clippy `-D warnings` | exit 0 | V-02. `cargo fmt --check` fails with 38,042 diff lines (G1 disposition D4); it is not clean and was deliberately left alone. |
| Rust, release profile | a targeted run of 10 suites: 209 passed, 0 failed, 6 ignored | V-03. **There is no release-profile general suite.** |
| UI, `node --test test` in `ui/` | 483 passed, 0 failed (30 `*.test.mjs` files), reproduced during the final reconciliation pass of this handbook with `npm test` (2026-10-30, evening, Windows, one host). Earlier records: 298 at 15:29 (V-07) and 299 at 16:07; the suite grew after the redesign added tests | Tests are mostly regex and rendered-markup guards. They do not prove layout or visual quality. |

| Gate | Status (docs/STATUS.md section 2 and the ledger) |
|---|---|
| G1 Scenario B | PASS with dispositions D1 to D5. Results are exploratory (debug, loopback, one host). |
| G2 Lab API | PASS with dispositions G2-D1 to G2-D7. |
| G3 UI plus browser verification | Verdict not recorded. Open high findings U-01, U-02, U-03. Visual quality **not approved**; owner decision O8-2 is open. |
| G4 two whole-product review cycles | Rust cycle 1 delivered (three AI reviewers, 99 findings). Cycle 2 and the UI side not started. |
| G5 reviewed commits and push | Preparation done; only documentation groups committed; nothing pushed. |

# R2. Source-file map <!--gen-->

## Appendix F: Important source files and their responsibilities

This appendix is a map from files to what they do, which chapter teaches them, and what state they are in. Every path was checked against the working tree on 2026-10-30. The "Status and notes" column uses the handbook's labels (Ch 8, 8.1) and marks files that are libraries with no serving-path caller, legacy, or unused. "Chapter" names the place that teaches the file; where a file is not analysed in any chapter, the entry says so.

Two facts help when reading the tables. First, `src/main.rs` declares its own copy of the module tree and `src/lib.rs` declares another, so the library and the binary compile the same files twice (Ch 2, 2.2.1). Second, the protected core is `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml` (Ch 8, 8.7.1); the chapters did not edit any of it.


### 1. Entry points and configuration

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/main.rs` | Builds the gateway binary `phantom-cache-demo`: resolves configuration, builds the cache with eight shards, the router, the cluster branch and the Lab block, and starts the servers | Ch 2, 2.1 and 2.2 | IMPLEMENTED; builds demo objects (`_traffic`, `_network`, `_db`, `_coalescer`) that serving never uses; `pie: None` |
| `src/lib.rs` | Registers the library modules so tests, benches and the Lab binary can use them | Ch 2, 2.2.1 | Protected core; declares the module tree a second time |
| `src/config.rs` | Parses command line, file and environment; validates the `KEYS` table; fails closed; prints a redacted effective configuration | Ch 2, 2.9.2 | IMPLEMENTED, TESTED; `PHANTOM_DEFAULT_TTL_MS` is `Wiring::NotWired` |
| `src/health.rs` | Liveness and readiness endpoints (`/health/live`, `/health/ready`) | Ch 6, 6.3 | IMPLEMENTED, TESTED by `m7_health_tests`; readiness reflects local state only |
| `src/bin/phantom-lab.rs` | Command line for Phantom Lab; runs the library code without HTTP and refuses the sealed seed set | Ch 7, 7.1 and 7.4.6 | IMPLEMENTED; second binary in `Cargo.toml` |


### 2. src/core: the cache engine

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/core/cache.rs` | `PartitionedCache`: sharded store, single-flight registry, generations, epochs, byte budget, eviction hooks, sweeper | Ch 4, 4.1 to 4.7 | IMPLEMENTED, TESTED; open findings G4A-01, -03, -08, -12, -13, -17; protected core |
| `src/core/coalesce.rs` | `SharedOp` (the shared result handed to waiters) and the legacy `Coalescer` | Ch 4, 4.4 | `SharedOp` IMPLEMENTED; `Coalescer` kept for compatibility, created with an underscore name and unused in serving (Ch 1, 1.3.1) |
| `src/core/eviction.rs` | `ShardEvictionPolicy` and the SIEVE, LRU and S3-FIFO implementations | Ch 4, 4.9 | IMPLEMENTED, TESTED (weakly); S3-FIFO tombstone finding G4A-02 OPEN; no release benchmark |
| `src/core/freshness.rs` | Decides cacheability and lifetime from `Cache-Control`, `Expires` and validators (`evaluate_response_freshness`) | Ch 4, 4.8 | IMPLEMENTED (RFC 9111 subset); conformance gaps G4A-04 to -06 OPEN; no gateway ceiling on lifetime |


### 3. src/api: HTTP surface

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/api/server.rs` | Axum router, GET and DELETE handlers, origin fetch client, control, metrics and SSE handlers, authentication calls | Ch 2, 2.2 to 2.7; Ch 6, 6.2 to 6.5 | IMPLEMENTED, TESTED; body read before authentication (G4B-01, OPEN); two `60_000` literals for the default lifetime; GET is local only |
| `src/api/lab_api.rs` | Lab HTTP API (`/api/v1/lab/*`), off by default, with Host and Origin guard and body timeout | Ch 2, 2.8; Ch 7, 7.7.1 | IMPLEMENTED, TESTED; holds the only `unsafe` blocks in the product (Windows process kill) |


### 4. src/dist: cluster

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/dist/router.rs` | `RendezvousRouter`: picks the DELETE primary, fans out invalidations, repair ledger, reconciliation | Ch 5, 5.3 and 5.5 | IMPLEMENTED, TESTED (loopback, debug); weight constant 1.0; `forward_key_over_tcp` has no production caller |
| `src/dist/node.rs` | `VirtualNode` and `NodeState`; mTLS listener; frame handlers including `InvalidateRequest` and `GetRequest` | Ch 5, 5.2 and 5.8 | IMPLEMENTED; `GetRequest` handler is a stub that fabricates a value (DR-029, PROTOTYPE ONLY); `Degraded` state never set |
| `src/dist/frame.rs` | PhantomMesh binary frame codec (magic, version, 28-byte header, types) | Ch 5, 5.2 | IMPLEMENTED, TESTED by in-file tests and `security_phase3b_framing_tests` |
| `src/dist/pool.rs` | Connection pool with heartbeat probes every five seconds | Ch 5, 5.2 | heartbeat frames and idle-connection keepalive IMPLEMENTED; heartbeat-based failure detection and state change DESIGNED BUT NOT IMPLEMENTED (G4B-12); connections never age out (G4B-08) |
| `src/dist/session.rs` | Tracks inbound cluster sessions and cancels sockets of a revoked peer | Ch 6, 6.4 and 6.11 | IMPLEMENTED; revocation has no production trigger (G4B-07) |
| `src/dist/anti_entropy.rs` | Ring buffer of sent invalidation records and gap tracking for pull reconciliation | Ch 5, 5.5 | IMPLEMENTED, TESTED; memory only, lost with the issuer |


### 5. src/security: authentication and trust

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/security/mod.rs` | `SecurityConfig` and the fail-closed startup checks (`validate_startup_security`, `validate_upstream_startup_security`) | Ch 2, 2.2.1; Ch 6, 6.2 | IMPLEMENTED, TESTED |
| `src/security/jwt.rs` | `JwtValidator`, `Role`; HS256 only; checks signature, issuer, audience, expiry | Ch 3, 3.8; Ch 6, 6.2 | IMPLEMENTED, TESTED; no revocation, no `nbf` check |
| `src/security/cluster.rs` | Cluster mTLS configuration, SAN and fingerprint identity checks, membership registry, revocation flag | Ch 3, 3.7; Ch 6, 6.4 | IMPLEMENTED, TESTED (loopback); `ClusterMembershipRegistry::update` has no production caller |
| `src/security/tls.rs` | `TlsServerConfig`: optional ingress TLS via `axum-server` and rustls | Ch 3, 3.1 and 3.7 | IMPLEMENTED; small file; not separately analysed beyond the TLS 1.3 wording discrepancy in Ch 3, 3.4 |
| `src/security/upstream.rs` | SSRF policy: allowlist, IP filter, DNS pinning (`resolve_and_pin_origin`, `PreConnectIpFilter`) | Ch 6, 6.8 | IMPLEMENTED, TESTED; range gaps G4B-15 OPEN |


### 6. src/intel: telemetry and early prototypes

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/intel/telemetry.rs` | `AtomicLogLinearHistogram` (2,688 buckets) and the sorted-window percentile function | Ch 5, 5.11 | IMPLEMENTED, TESTED (recording and bucket arithmetic only); fed but no production percentile reader, so not the source of displayed percentiles |
| `src/intel/diagnostics.rs` | Cache counters, diagnostic event buffer and the wall-clock time source | Ch 2, 2.2.1; Ch 4, 4.8 | IMPLEMENTED; all time is wall clock (G4A-10) |
| `src/intel/origin_metrics.rs` | `OriginMetrics`: 16-bucket origin latency histogram plus in-flight and error counters | Ch 5, 5.10 | IMPLEMENTED; used by the live path |
| `src/intel/sketch.rs` | Early `CountMinSketch` | Ch 5, 5.12 | Unused: referenced only by its own file and `tests/unit/wp2_tests.rs`; PROTOTYPE ONLY |
| `src/intel/workload.rs` | Early `WorkloadProfile` presets | Ch 5, 5.10 | Unused outside `tests/unit/wp2_tests.rs`; PROTOTYPE ONLY; the Lab workload library is the maintained one |


### 7. src/intelligence: the intelligence library

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/intelligence/mod.rs` | `IntelligenceMode` (off, shadow, canary, active) and the shared state | Ch 1, 1.7; Ch 5, 5.13 | IMPLEMENTED; protected core; default mode `off` |
| `src/intelligence/executor.rs` | `OriginGate` and `acquire_origin_permit` (live path); policy executor, safe fallback and watchdog (library) | Ch 4, 4.4; Ch 5, 5.13 | Origin gate IMPLEMENTED, TESTED; policy methods and watchdog are a library with no caller in `src/` |
| `src/intelligence/candidates.rs` | `OriginConcurrencyCapController` (AIMD) and other candidate policies | Ch 5, 5.13 | AIMD IMPLEMENTED, TESTED (arithmetic and cap unit tests; serving path only when a cap is configured in canary or active mode); benefit NOT SUPPORTED; other candidates not wired |
| `src/intelligence/signals.rs` | `HeavyHitterDetector` (Count-Min plus Space-Saving), hot-key emergence detector | Ch 5, 5.12 | PROTOTYPE ONLY; not called by the gateway; metrics report hot keys unavailable |
| `src/intelligence/shadow.rs` | `ShadowEvaluator` for replaying traffic through candidate eviction policies | Ch 5, 5.13 | PROTOTYPE ONLY; flaw F13 (correlated shadows) |
| `src/intelligence/engine.rs` | `PieDecisionEngine`: compares candidate and baseline from shadow results | Ch 5, 5.13 | PROTOTYPE ONLY; not wired; flaw F12 |
| `src/intelligence/audit.rs` | Bounded circular buffer of decision records (`AuditStore`) | Ch 1, 1.8; Ch 5, 5.10 | The route exists but nothing in the serving path writes to it, so it serves an empty store |


### 8. src/control and src/sim: legacy

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/control/pie.rs` | The legacy `PieController`: threshold rule with a literal `expected_utility` | Ch 5, 5.13; Ch 8, 8.11 | Legacy stub, PROTOTYPE ONLY; `pie: None` in `main.rs`; tests assert its wiring only (`p4_pie_tests.rs`) |
| `src/sim/traffic.rs`, `src/sim/backend.rs`, `src/sim/network.rs` | Early demo traffic generator, backend database model and network link model | Ch 1, 1.3.2; Ch 2, 2.2.1 | Legacy demo code; built in `main::run` and not used in serving |
| `src/sim/clock.rs`, `src/sim/events.rs`, `src/sim/rng.rs`, `src/sim/scheduler.rs` | Early demo clock, event, random-source and scheduler helpers | Ch 2, 2.2.1 | Legacy demo code; not analysed in a chapter; the maintained simulator is `src/lab/` |


### 9. src/lab and src/lab_live: Phantom Lab

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/lab/t1_sim.rs` | T1 cell runner: real cache and executor on a virtual clock against a simulated origin | Ch 7, 7.2 | IMPLEMENTED, TESTED; SIMULATED tier; protected and frozen |
| `src/lab/clock.rs` | Lab `VirtualClock` | Ch 7, 7.2.1 | Exercised only by one test (`lab_core_tests`); `run_cell` does not instantiate it. `src/sim/clock.rs` holds a different struct of the same name (Ch 7, 7.2.1) |
| `src/lab/events.rs` | `EventQueue` with time and sequence tie-breaking | Ch 7, 7.2.2 | IMPLEMENTED, TESTED |
| `src/lab/rng.rs` | `MasterRng`, `StreamRng` (SplitMix64), `AliasSampler`, `SeedSets` | Ch 7, 7.2.3 | IMPLEMENTED, TESTED |
| `src/lab/workloads.rs` | Workload families (uniform, Zipf, scan and others) | Ch 7, 7.2.4 | IMPLEMENTED, TESTED |
| `src/lab/origin_sim.rs` | `SimulatedOrigin` with servers, queue and degradation scenarios | Ch 7, 7.2.5 | IMPLEMENTED; model limits listed in the chapter |
| `src/lab/analysis.rs` | Paired differences, BCa bootstrap, sign-flip test, Holm correction | Ch 7, 7.3 | IMPLEMENTED, TESTED; no reference-implementation cross-check |
| `src/lab/eval.rs` | M6 matrix driver, manifests, endpoint analysis | Ch 7, 7.4 | IMPLEMENTED; edited after the sealed run under a pinned addendum |
| `src/lab/m6_pipeline.rs` | The mechanical sealed-evaluation pipeline and `compute_verdict` | Ch 7, 7.4.4 and 7.6 | Executed once; Tier 1 simulated; not human-reviewed |
| `src/lab/runner.rs` | `RunSpec` to `execute`: the library API used by tests, CLI and Lab API | Ch 7, 7.1 and 7.4.6 | IMPLEMENTED, TESTED |
| `src/lab/matrix_runner.rs` | Scenario matrix runner and `AgreementStudy` (T1 versus T2 rank concordance) | Ch 7, 7.5.3 | IMPLEMENTED; no real agreement study run (UNVERIFIED) |
| `src/lab/manifest.rs` | `RunManifestV2` provenance record | Ch 3, 3.9; Ch 7, 7.4.5 | Not the manifest real runs write; hash of unordered maps is fragile (G4C) |
| `src/lab/eviction_baselines.rs`, `src/lab/tuning.rs`, `src/lab/grid_tuning.rs`, `src/lab/report_gen.rs` | Reference eviction baselines, tuning protocol and v2.2 grids, report and provenance formatter | Ch 4, 4.9 and Ch 7, 7.2.7 (context) | IMPLEMENTED; frozen Lab code; these four files are not analysed line by line in the chapters |
| `src/lab/mod.rs` | Lab module header stating what the Lab is not | Ch 2, 2.8; Ch 7, 7.1 | Header comment says it is not a benchmark or production load test |
| `src/lab_live/scenario_b.rs` | Scenario B driver and verdict rule (T2) | Ch 7, 7.5 | TESTED; exploratory; U-01 defect OPEN (fix sandbox only; integration authorised and in progress, not committed) |
| `src/lab_live/gateway.rs`, `src/lab_live/origin.rs`, `src/lab_live/loadgen.rs` | Spawns the gateway child process, scripted raw-TCP origin, open-loop load generator | Ch 7, 7.5.1 | IMPLEMENTED; origin has no load-dependent slowdown |
| `src/lab_live/analysis.rs`, `src/lab_live/http.rs`, `src/lab_live/cli.rs`, `src/lab_live/mod.rs` | Paired analysis and arm verdicts, small HTTP client, `scenario-b` command line, module root | Ch 7, 7.5 | IMPLEMENTED; `cli.rs` and `http.rs` are not analysed line by line in the chapter |


### 10. Tests, benchmarks and test support

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `tests/integration/m1_http_tests.rs` | HTTP-level tests of miss, hit, coalescing and body limits against a mock origin | Ch 2, 2.4 and 2.5 | TESTED; loopback, debug, one gateway |
| `tests/integration/concurrency_verification_tests.rs` | Exhaustive protocol model, mutation detector, stress schedule, proptest | Ch 3, 3.12; Ch 4, 4.5 | TESTED; the model cannot find G4A-01 |
| `tests/integration/invalidation_contract_tests.rs`, `epoch_contention_tests.rs` | DELETE, tenant and global invalidation contracts; epoch reads under contention | Ch 4, 4.7 | TESTED; do not exercise the G4A-01 gap |
| `tests/integration/byte_accounting_tests.rs`, `memory_bound_soak_tests.rs` | Byte-accounting equality and bounded-memory churn | Ch 4, 4.6 | TESTED (consistency only) |
| `tests/integration/stale_serving_tests.rs`, `freshness_matrix_tests.rs`, `revalidation_304_tests.rs` | Stale-if-error, freshness decisions, 304 revalidation | Ch 4, 4.8 | TESTED; stale-cap counter test is weak (G4A-18) |
| `tests/integration/eviction_contract_tests.rs` | Eviction policy contracts | Ch 4, 4.9 | TESTED (weakly) |
| `tests/integration/coalesce_tradeoffs_tests.rs`, `origin_cap_tests.rs`, `serving_contract_tests.rs` | Single-flight trade-offs, origin cap, serving contracts | Ch 4, 4.4; Ch 2, 2.5 | TESTED |
| `tests/integration/distributed_consistency_tests.rs`, `cluster_followup_tests.rs`, `cluster_propagation_latency_tests.rs`, `known_gaps.rs` | Cluster convergence, follow-ups, propagation latency, pinned known gaps (including DR-029) | Ch 5, 5.5 to 5.8; Ch 6, 6.9 | TESTED (loopback, debug, at most four nodes) |
| `tests/integration/security_phase*_tests.rs` and `m7_sec_*_tests.rs` | Phase security tests: tenant isolation, upstream filter, cluster admission, framing, pooling, invalidation, router, anti-entropy, review rounds | Ch 6, 6.2 to 6.8 | TESTED; no independent review |
| `tests/integration/telemetry_redesign_tests.rs`, `telemetry_honesty_tests.rs`, `telemetry_truth_tests.rs` | Histogram, provenance honesty and telemetry truth | Ch 5, 5.11; Ch 2, 2.8 | TESTED |
| `tests/integration/intel_*_tests.rs`, `p4_pie_tests.rs` | Intelligence library tests; legacy PIE wiring | Ch 5, 5.13 | TESTED as library; `p4_pie_tests.rs` asserts stub behaviour |
| `tests/integration/lab_*_tests.rs`, `lab_api/` | Lab determinism, runner, baselines, Scenario B and Lab API tests | Ch 7, 7.2 to 7.7 | TESTED; determinism shown in one environment |
| `tests/unit/wp1_tests.rs` to `wp4_tests.rs` | Early work-package unit tests (including `CountMinSketch` in `wp2_tests.rs`) | Ch 5, 5.12 | TESTED; early prototype tests |
| `tests/support/cluster_harness.rs`, `tests/support/fault_proxy.rs`, `tests/support/lab/` | Cluster test harness, loopback fault proxy, Lab test doubles | Ch 3, 3.12 | Test support; fault proxy is loopback only |
| `benches/b1_hit_path.rs`, `b2_concurrency.rs`, `b3_coalesce.rs`, `b4_eviction.rs`, `b6_telemetry.rs`, `b7_memory.rs`, `b8e_gateway.rs` | Seven custom-main benchmarks (`harness = false`) | Ch 3, 3.12; Ch 6, 6.12 and 6.13 | IMPLEMENTED; no release-profile result established on the current tree |
| `benches/support/alloc_counter.rs`, `benches/support/refpolicies.rs`, `benches/support/mod.rs` | Counting allocator, reference policies, shared bench helpers | Ch 3, 3.12 | Bench support; `alloc_counter.rs` holds a counting `GlobalAlloc` |


### 11. ui, scripts and packaging

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `ui/package.json`, `ui/vite.config.js`, `ui/index.html` | React 18 and Vite 5 project definition; dependency list pinned by a guard test | Ch 3, 3.11 | IMPLEMENTED; one declared dependency unused |
| `ui/src/App.jsx`, `ui/src/main.jsx` | UI entry and routing between the dashboard screens | Ch 1, 1.8; Ch 3, 3.11 | IMPLEMENTED; read, not run, for the chapters |
| `ui/src/screens/` (`Overview.jsx`, `Traffic.jsx`, `CacheMemory.jsx`, `Freshness.jsx`, `Origin.jsx`, `Simulation.jsx`) | Dashboard screens | Ch 7, 7.7.2 | IMPLEMENTED, automated checks only; visual quality not approved |
| `ui/src/screens/lab/` | Lab screens (About, Start a run, Runs, Evidence; `ScenarioBResult.jsx`; `ConnectionPanel.jsx`) | Ch 7, 7.7.2 | IMPLEMENTED; token kept in memory |
| `ui/src/lib/labProvenance.js`, `ui/src/lib/provenance.js`, `ui/src/lib/schema.js`, `ui/src/lib/sse.js` | Provenance labels, schema handling and the fetch-based stream client | Ch 6, 6.3; Ch 7, 7.7.2 | IMPLEMENTED; browser `EventSource` cannot send `Authorization`, so streams use `fetch` |
| `ui/test/` | `node:test` suite (30 `.test.mjs` files plus helpers and fixtures) | Ch 3, 3.12 | TESTED; one fixture masked U-01 (`lab.view.test.mjs`) |
| `ui/scripts/` | Contrast, accessibility, forbidden-pattern checks and Chrome DevTools Protocol journeys | Ch 3, 3.11 and 3.12 | One Chrome, one machine; no screen-reader test |
| `packaging/build-release.sh`, `build-release.ps1`, `verify-checksums.sh` | Release build with `--locked`, archive and checksums | Ch 2, 2.9.3; Ch 3, 3.10 | IMPLEMENTED; does not commit, tag or publish |
| `packaging/smoke-test.sh`, `smoke-test.ps1`, `phantom.example.conf`, `phantom.production.example.conf` | Smoke test of a bundle and example configurations | Ch 2, 2.9.3; Ch 3, 3.10 | IMPLEMENTED; clean-machine reproduction not done |
| `scripts/reproduce_tests.sh`, `reproduce_tests.ps1`, `cleanroom_build.ps1`, `run_audit_suite.ps1` | Test reproduction, clean-room build and audit suite scripts | Ch 3, 3.12 | Present; whether they were run on a clean machine is UNVERIFIED |
| `scripts/gen-demo-certs.sh`, `gen-demo-certs.ps1` | Generate demonstration cluster certificates | Ch 3, 3.7 | Demo use only |
| `scripts/docs-lint.mjs` | Report-only documentation lint tool | Ch 3, 3.12; Ch 8, 8.8 | Committed tooling |
| `.github/workflows/ci.yml` | CI definition: Linux, format, check, clippy, test; frontend build | Ch 3, 3.10 | Present; no `-j 1`, debug profile; whether it ever ran is UNVERIFIED |
| `Start-Phantom-Demo.bat` | Windows demo launcher | Ch 3, 3.10 | Windows only |


### 12. Key documents and registers

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `docs/STATUS.md` | The project's status page: gates, milestones, evidence gaps | Ch 8, 8.2 | Entry point for current state |
| `docs/user/limitations.md` | User-facing list of what is not provided and known gaps | Ch 2, 2.9.3; Ch 8, 8.3 | Authoritative for limitations |
| `docs/user/configuration.md` | Reference for configuration keys; checked against the key table by `m7_config_tests` | Ch 2, 2.9.2 | Does not prove runtime wiring |
| `docs/release/g4/cycle1_findings_disposition_register.md` | Register of the 99 G4 findings and their status | Ch 8, 8.4.1 | Closes nothing; owner statements set dispositions |
| `docs/release/g4/cycle1_core_engine_review.md`, `cycle1_dist_security_api_review.md`, `cycle1_lab_intelligence_deps_review.md` | The three AI review reports (G4A, G4B, G4C) | Ch 8, 8.4.1 | AI second looks; findings confirmed by reading |
| `docs/release/owner_decisions_register.md` | Owner decisions ODR-1 to ODR-4 | Ch 8, 8.4.3 | Sets which findings are recorded, named or held |
| `docs/release/known_limitations_frozen_code.md` | Limitations recorded in frozen Lab code | Ch 7, 7.4.4; Ch 8, 8.4.5 | Includes what G4C did not read |
| `docs/release/verification_register.md`, `docs/release/workstream_completion_ledger.md`, `docs/release/execution_register.md` | Verification and completion records and the delivery plan | Ch 8, 8.1 and 8.8 | Independence classes; none INDEPENDENTLY VERIFIED |
| `docs/release/deferred_integration.md` | Integration items such as D-1 (connection drain) and D-2 (sweeper shutdown) | Ch 8, 8.3.7 | DEFERRED items |
| `docs/research/claims_ledger.md` | Which project claims are supported, narrowed, amended or untraceable | Ch 6, 6.9.3; Ch 7, 7.5.3 | C6 amended; C7 UNVERIFIED |
| `docs/research/m6_sealed_results.md` | The sealed M6 result and its limits | Ch 5, 5.14; Ch 7, 7.6 | Executed once; simulated; not human-reviewed |
| `docs/research/code_findings.md`, `docs/research/pie_audit.md` | Earlier code findings (F-numbers) and the PIE audit | Ch 5, 5.13; Ch 8, 8.2.2 | Prior-agent records, checked by the chapters |
| `docs/research/g4a01_race/` and `docs/research/m3_durability/` | Sandbox analysis, repro attempts and patches for G4A-01 and for durable invalidation | Ch 4, 4.7; Ch 5, 5.9 | Sandbox only; not integrated |
| `docs/research/adr/ADR-001-present-architecture-and-direction.md`, `ADR-002-serving-path-contract.md`, `ADR-003-distributed-architecture.md` | Architecture decision records; ADR-003 amended 2026-10-30 | Ch 3, 3.0; Ch 5, 5.6 | Agent-recorded, not owner-ratified |
| `docs/research/rt0_prior_art_sweep_2026-10-30.md` | Prior-art sweep; RT-0 still open | Ch 7, 7.8 | PARTIAL; no novelty claim |
| `docs/security/m7_security_review.md` | M7 security review and SEC-M7 items | Ch 6, 6.11; Ch 8, 8.4.6 | AI review; some items open |
| `docs/demo/scenario_b_interpretation_notice.md` | Binding notice on how to read Scenario B | Ch 6, 6.12.3; Ch 7, 7.5 | Binding for quoting Scenario B |
| `docs/glossary.md` | The repository's own glossary, including the tier definitions | Ch 6, 6.13 | Source for tier labels |
| `CLAUDE.md` | Project rules for agents, including the code invariants and known gaps | Ch 1, 1.11; Ch 8, 8.7 | Parts are marked outdated inside the file |
| `bench-results/README.md` and `bench-results/r0_summary.json` | Benchmark status and the 2026-10-28 micro-benchmark summary | Ch 6, 6.12 and 6.13 | No release-profile result on the current tree |
| `lab-results/` | Retained Lab output: sealed M6 run, Scenario B runs, dry runs | Ch 7, 7.4.5 and 7.5 | Evidence folders; do not edit |

# R3. Glossary and abbreviations <!--gen-->

## Appendix D: Glossary and abbreviations

This glossary lists the terms and abbreviations used in Chapters 1 to 8, in alphabetical order. Each entry is one line in plain English, followed by the place where the chapter teaches it (for example `Ch 4, 4.5` means Chapter 4, section 4.5). The definitions follow the way the chapters use the words. Where a term has a status or a caveat in the chapters, the entry says so. Code names are given as in the source tree; Appendix F maps them to files.

Three conventions apply throughout. A **status label** (IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, DEFERRED, BLOCKED, UNVERIFIED, INDEPENDENTLY VERIFIED) always has the meaning given in Ch 8, 8.1. A finding identifier such as `G4A-01` points to the review register described in Ch 8, 8.4. A tier label (T1, T2) says how a number was produced (Ch 7, 7.1).

**A**


- **Active** (node state): the state of a cluster member that is in service; peers in a configured cluster start in it. One of seven values of `NodeState`, of which `Degraded` is never assigned. (Ch 5, 5.2)
- **ADR** (architecture decision record): a file in `docs/research/adr/` that records an architectural choice and its reason; the chapters separate "recorded decision" from "our reasoning". (Ch 3, 3.0)
- **Advisory scan**: a check of locked dependencies against known vulnerability reports (for example `cargo audit`). None has been run; the chapters list it as DEFERRED. (Ch 3, 3.13)
- **Age**: how old a stored response is; compared with its freshness lifetime to decide fresh or stale. (Ch 4, 4.8)
- **AIMD** (additive increase, multiplicative decrease): the rule behind TCP congestion control, used by the optional origin-concurrency cap controller: add a little when healthy, cut sharply on trouble. IMPLEMENTED, TESTED (arithmetic and cap unit tests); on the serving path only when a cap is configured in canary or active mode; its benefit is NOT SUPPORTED. (Ch 5, 5.13)
- **Amortized O(1)**: some single operations cost more, but the average over a long run is constant per operation; used for eviction. (Ch 4, 4.9)
- **Anti-entropy**: a background process that compares state between nodes and repairs differences; here, a pull of missed invalidation records from the sender's ring buffer. (Ch 5, 5.1 and 5.5)
- **Arc**: the Rust shared-ownership pointer; the epoch snapshot is an `Arc` of a map, so readers share it without an exclusive lock. (Ch 3, 3.5.4)
- **Arm**: one configuration compared in an experiment (for example `BASE`, `STATIC-CAP`, `AIMD-CAP` in Scenario B). Arms differ in configuration, not through hidden multipliers. (Ch 7, 7.2.7 and 7.5.1)
- **Async / await**: Rust's way to park a task while it waits on a socket or timer so a thread can run another task. Never combined with a held `std::sync::Mutex`. (Ch 3, 3.3 and 3.5.1)
- **Atomic**: a counter or flag that threads update without a lock; used for statistics and for epochs read on the hit path. (Ch 3, 3.5.3)
- **Audit store** (decision audit store): a bounded buffer of decision records in `src/intelligence/audit.rs`; the serving path has no producer, so the route returns an empty store. (Ch 1, 1.8; Ch 2, 2.3.1)
- **Authentication / authorisation**: authentication asks "who are you?"; authorisation asks "may you do this?". PhantomCache checks a JWT and then the role. (Ch 6, 6.2)
- **Axum**: the Rust web framework (on hyper and tower) that routes HTTP requests in the gateway. (Ch 3, 3.4)

**B**


- **BASE** (arm): the Scenario B arm with intelligence off and no cap. (Ch 7, 7.5.1)
- **BCa** (bias-corrected and accelerated): a bootstrap confidence interval that corrects for bias and skew; the Lab uses it for paired differences. (Ch 7, 7.3.3)
- **Bearer token**: a credential sent in the `Authorization` header; whoever holds it is treated as the caller. (Ch 6, 6.2)
- **BL-12b**: the tuned static baseline (a static cap and stale window chosen on tuning seeds) used in the sealed M6 evaluation; the code says it is not claimed to be the best deployable static policy. (Ch 7, 7.6 and 7.9)
- **Bootstrap**: draw values with replacement from the sample many times to estimate the spread of a statistic. (Ch 7, 7.3.3)
- **Bounded staleness**: a read never returns data older than a named bound `B`. To claim it you must state `B`, the failure model and a test that could falsify it. Not claimed. (Ch 5, 5.6)
- **Byte budget**: the logical memory limit (`PHANTOM_MAX_CACHE_BYTES`, default 256 MiB), divided across shards. (Ch 4, 4.6)
- **Byte accounting (I1 to I8)**: the invariants that the running byte total equals the recomputed sum of entry sizes, checked by `audit_bytes()`. TESTED for consistency only. (Ch 4, 4.6)

**C**


- **Cache**: a store of copies, keyed so that a later request for the same thing can be answered without calling the origin. (Ch 1, 1.1)
- **Cache key**: the identity of one cached item: tenant plus request path plus query string (`CacheKey`). (Ch 1, 1.1; Ch 2, 2.3.2)
- **Cache poisoning**: an attacker makes the cache store something that other users then receive. The key contains every input that influences the origin response, which limits it. (Ch 6, 6.6)
- **Cache stampede** (thundering herd): many simultaneous misses for one key, each going to the origin. (Ch 1, 1.3.1; Ch 4, 4.4)
- **Cache-Control**: the origin's header that sets lifetime and stale rules (`max-age`, `s-maxage`, `must-revalidate`, `stale-while-revalidate`, `stale-if-error`). (Ch 4, 4.8)
- **Canary** (mode): the setting `PHANTOM_INTELLIGENCE=canary`. In this code it applies the origin cap to all fetches with no rollback in the serving path, so it is not a canary in the usual sense. (Ch 5, 5.13)
- **Canary routing**: a library function that sends a fraction of decisions to a candidate; not wired into the gateway. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Canonical string**: one agreed text form of a key, used on the wire between nodes. (Ch 4, 4.1)
- **Cancellation**: a client giving up mid-request; the origin fetch runs in its own detached task and RAII guards release state, so a cancelled leader does not strand waiters. (Ch 4, 4.4 and 4.5)
- **Cell** (Lab): one run of one scenario, workload family, arm and seed; the sealed M6 evaluation had 12,000 of them (8 scenarios, 5 families, 10 arms, 30 seeds). (Ch 7, 7.2 and 7.6)
- **Claims ledger**: `docs/research/claims_ledger.md`, the record of which project claims are supported, narrowed, amended or untraceable (for example C6 and C7). (Ch 6, 6.9.3; Ch 7, 7.5.3)
- **Cluster CA**: the private certificate authority whose certificates cluster members must chain to. (Ch 3, 3.7; Ch 6, 6.4)
- **Coalescer**: a legacy type kept for compatibility; the serving path coalesces through `PartitionedCache::begin_fetch_atomic` and `SharedOp` instead. (Ch 1, 1.3.1; Ch 2, 2.2.1)
- **Coalescing** (request coalescing): see single-flight. (Ch 4, 4.4)
- **Concurrency gate** (origin gate, `OriginGate`): a counter of permits that limits concurrent origin fetches; enforced only in `canary` or `active` mode with a configured cap. (Ch 4, 4.4; Ch 5, 5.13)
- **Concurrency vs parallelism**: concurrency is many tasks making progress in overlapping time; parallelism is tasks running at the same instant on different cores. (Ch 4, 4.2)
- **Coordinated omission**: a load-test error where a slow response delays later requests, hiding the slowness; closed-loop tests suffer from it, an open-loop generator does not. (Ch 6, 6.12.1 and 6.12.3)
- **CORS**: the browser rule for cross-origin calls; the gateway allows only exact configured origins and never credentials. It is not authentication. (Ch 2, 2.10; Ch 3, 3.4)
- **Count-Min Sketch**: a small fixed table that estimates how often each item was seen; it only overestimates. The library hot-key detector uses one; the older `CountMinSketch` in `src/intel/sketch.rs` is unused. (Ch 5, 5.12)

**D**


- **Debug build / release build**: the Cargo build profiles; all recorded tests ran in debug, and no release-profile suite exists on the current tree. (Ch 6, 6.12.1; Ch 8, 8.2.2)
- **DEC-006**: an owner-approved decision (Option B) behind the `.gitattributes` entry that protects four hash-pinned frozen files from line-ending conversion. (Ch 8, 8.7.1)
- **Decision engine** (`PieDecisionEngine`): the library that compares a candidate policy with a baseline from shadow evaluations; not wired; its statistics have findings F12 and F13. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Default TTL setting** (`PHANTOM_DEFAULT_TTL_MS`): validated and printed but not read at runtime; the effective default lifetime is a fixed 60 seconds. PARTIAL. (Ch 2, 2.6.1; Ch 4, 4.8)
- **DELETE**: `DELETE /api/v1/cache/<path>`; purges locally, waits up to 150 ms for the primary's acknowledgement, then tells other members in the background. 204 is not a cluster-wide acknowledgement. (Ch 2, 2.7.3; Ch 5, 5.5)
- **Deterministic simulation**: a simulation in which the same seed and source give the same bytes, because time and randomness are controlled. (Ch 7, 7.2 and 7.4.1)
- **Disposition register**: `cycle1_findings_disposition_register.md`, which lists the 99 G4 findings and their status. It closes nothing; only owner statements set dispositions. (Ch 8, 8.4.1)
- **DNS pinning**: resolve the origin name once, check every returned address, and connect to the exact address checked, so DNS changes cannot bypass the filter. (Ch 6, 6.8)
- **DR-027** (G4B-05): any active cluster member can send tenant-scope or full-cache invalidations that flush other nodes; owner-named, OPEN. (Ch 5, 5.8; Ch 6, 6.4)
- **DR-029** (G4B-30): the peer `GetRequest` handler fabricates a `val_node_` value for a missing key; unreachable by HTTP because key spaces are disjoint; a hazard for any future peer-fill. PROTOTYPE ONLY, OPEN. (Ch 5, 5.8)

**E**


- **E0, E1, E2**: the first planned intelligence experiments: cost of an event tap, validity of a load-dependent origin plant, and the headroom question (can any per-scenario cap beat one good static cap). Not run. (Ch 8, 8.9.4)
- **Epoch**: a counter that makes old state invalid by moving a number instead of scanning. The chapters name three: tenant and global epochs (per cache), the membership epoch (per cluster, static), and the stream sequence (per sender and receiver pair). (Ch 4, 4.7; Ch 5, 5.4)
- **ETag**: an origin validator stored with an entry and sent back in `If-None-Match` for revalidation. (Ch 4, 4.8)
- **Eventual consistency**: if writes stop and faults heal, all copies converge; it says nothing about how long. (Ch 5, 5.6)
- **Eviction**: removing entries to stay inside the byte budget or entry limit. (Ch 4, 4.9)
- **Error fan-out**: when the origin fails, every waiter on that fetch receives the same error. (Ch 4, 4.4)
- **Event queue**: the Lab's time-ordered to-do list; equal times are ordered by a sequence number so replays are identical. (Ch 7, 7.2.2)

**F**


- **Fail-closed**: refuse to start or refuse a request when a safety condition is not met, instead of continuing. The configuration loader and the startup security gate do this. (Ch 2, 2.2.1; Ch 6, 6.2)
- **Fan-out**: sending one message to many receivers; here, one invalidation to every member. (Ch 5, 5.1 and 5.5)
- **Fencing** (generation fencing): a fetch records a number when it starts and may store its result only if that number is unchanged when it finishes. (Ch 1, 1.3.4; Ch 4, 4.5)
- **Fencing token**: the number recorded at the start and compared at the end. (Ch 4, 4.5)
- **Fingerprint** (certificate): a hash of a member's certificate pinned in the membership file; checked in addition to the chain. (Ch 3, 3.7; Ch 5, 5.2)
- **Frame**: one message of the PhantomMesh binary protocol (magic, version, 28-byte header, type). (Ch 2, 2.2.1; Ch 5, 5.2)
- **Freshness lifetime**: how long a copy may be treated as correct; after it the copy is stale. (Ch 1, 1.1; Ch 4, 4.8)
- **Frozen files**: sources and records whose bytes are pinned by the pre-registration freeze; edits need a signed addendum, so defects there are recorded as limitations. (Ch 7, 7.4.4; Ch 8, 8.6 and 8.7.1)

**G**


- **G1 to G5** (delivery gates): G1 Scenario B, G2 Lab API, G3 UI plus browser verification, G4 two whole-product review cycles, G5 reviewed commits and push. (Ch 8, 8.2.3)
- **G4, G4A, G4B, G4C**: gate G4 and its three AI reviews: G4A core engine (26 findings), G4B distribution, security and API (38), G4C lab, intelligence, freeze and dependencies (35); 99 in all. (Ch 8, 8.4.1)
- **G4A-01**: the reported race in `finalize_fetch`: the epochs are read twice, so an invalidation landing between the reads can let an old body be stored as current. Reproduced only in a sandbox with a test hook; fix not integrated; UNDER-INVESTIGATION. (Ch 4, 4.7; Ch 6, 6.9.4)
- **Gateway**: one PhantomCache process (an HTTP caching proxy in front of an origin). Each gateway has its own cache. (Ch 1, 1.5; Ch 2, 2.1)
- **Generation**: a per-key counter that DELETE bumps; defends against "delete during fetch". (Ch 4, 4.5)
- **Ghost queue**: in S3-FIFO, a queue that remembers recently evicted keys (no bodies). (Ch 4, 4.9)
- **Global epoch**: the one counter that invalidates everything on a node; also the fallback when more than 10,000 tenant epochs exist (SEC-4A-01). (Ch 4, 4.7; Ch 5, 5.4)

**H**


- **H1**: the registered hypothesis that an origin-aware adaptive controller beats the best static configuration. Sealed verdict: NOT_SUPPORTED (simulated). (Ch 7, 7.6; Ch 5, 5.14)
- **Hard staleness cap** (`PHANTOM_MAX_STALE_MS`, default 24 hours): the limit on how long past expiry a stale copy may be served. It bounds stale serving, not the fresh lifetime. (Ch 4, 4.8)
- **Hash map**: a table that finds a key in expected constant time from a hash of the key. (Ch 4, 4.1)
- **Head-of-line blocking**: every waiter waits for the leader, however slow; the origin read has a 5-second timeout. (Ch 4, 4.4)
- **Heartbeat**: a keepalive probe on idle pooled connections every five seconds. It does not change any node state, so it is not a failure detector (G4B-12). (Ch 5, 5.2)
- **Heavy hitter**: an item that accounts for a large share of requests; found with Count-Min plus Space-Saving in the library. (Ch 5, 5.12)
- **Hit / miss**: a hit is answered from a usable cached copy; a miss needs the origin. (Ch 1, 1.1)
- **Hit ratio**: hits divided by total lookups. (Ch 1, 1.1 and 1.2)
- **Holm correction**: a step-down adjustment of p-values across many hypotheses (16 primary ones in M6). (Ch 7, 7.3.3)
- **Hot key**: a key that receives a disproportionate share of requests; handled by coalescing on a miss, not by detection. (Ch 1, 1.3.2; Ch 5, 5.12)
- **HRW** (highest random weight): another name for rendezvous hashing. (Ch 2, 2.7.2; Ch 5, 5.3)
- **HS256**: the symmetric JWT signing algorithm the code accepts; the only one. (Ch 3, 3.8; Ch 6, 6.2)
- **hyper / tower**: hyper is the low-level HTTP implementation; tower supplies reusable service wrappers. Axum sits on both. (Ch 3, 3.4)

**I**


- **Idempotent**: applying the same operation twice has the same effect as once; a repeated delete only causes an extra miss. (Ch 5, 5.5)
- **INDEPENDENTLY VERIFIED**: a status label for a check by a human or external reviewer. Nothing in the project holds it; AI reviews are "AI second looks". (Ch 8, 8.1)
- **Invalidation**: removing or marking a copy as invalid before it would expire. (Ch 1, 1.1 and 1.3.4)
- **Invalidation bus**: the experimental mTLS layer that carries invalidations between gateways; the main distributed feature. (Ch 1, 1.4; Ch 5, 5.5)

**J**


- **JWT** (JSON Web Token): a signed note stating who the caller is, the role and tenant, and when it expires. Checked for signature, issuer, audience and expiry; no revocation. (Ch 3, 3.8; Ch 6, 6.2)

**K**


- **Key**: see cache key. (Ch 1, 1.1)
- **KeyState**: the per-key record holding the entry, generation, active operations and in-flight registration. (Ch 4, 4.1)

**L**


- **Lab API**: the HTTP interface to Phantom Lab (`/api/v1/lab/*`); off by default. (Ch 2, 2.8; Ch 7, 7.7.1)
- **Latency / throughput**: latency is how long one request takes; throughput is requests completed per second. (Ch 1, 1.1)
- **Lazy invalidation**: do not remove entries at invalidation; drop them when next touched or when the sweeper finds them. (Ch 4, 4.7)
- **Leader / waiter**: the leader is the request that starts the origin fetch; waiters join it and receive the same result. (Ch 4, 4.4)
- **Linearizability**: every operation appears to take effect at one instant in real-time order, as seen by all clients. Not claimed. (Ch 5, 5.6)
- **Lock contention**: threads waiting for a lock another thread holds. (Ch 4, 4.2)
- **Lock order**: shard mutex first, then the epoch structures; never the reverse; no `std::sync::Mutex` held across `.await`. (Ch 3, 3.5.4; Ch 4, 4.2)
- **Log-linear histogram**: 2,688 counters with constant-time recording and a relative error of at most 0.39 percent by construction; IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader, so it is not the source of the percentiles the endpoint displays. (Ch 5, 5.11)
- **Logical bytes**: the cache's own size bookkeeping (key, body capacity, headers, fixed overhead); not operating-system memory. (Ch 4, 4.6)
- **Loopback**: network traffic inside one machine; no real network latency. Results are labelled MEASURED_LOOPBACK. (Ch 6, 6.12.1; Ch 7, 7.5)
- **LRU** (least recently used): evict the entry untouched the longest. (Ch 4, 4.9)

**M**


- **M3**: the durable-invalidation milestone; its sandbox prototype is not integrated. (Ch 5, 5.9; Ch 8, 8.9.1)
- **M6**: the milestone of the sealed, pre-registered evaluation of the adaptive controller (H1 NOT_SUPPORTED). (Ch 5, 5.14; Ch 7, 7.6)
- **MEASURED_LOOPBACK**: tier label for numbers from real binaries on one host; separate from SIMULATED. (Ch 2, 2.8; Ch 7, 7.5)
- **Measured / simulated / inferred / unavailable**: the four labels every displayed UI value must carry; synthetic data is never shown as live. (Ch 2, 2.8; Ch 7, 7.7.2)
- **Membership** (static): the cluster's member list in a JSON file read at startup; no join, leave or gossip. (Ch 5, 5.2)
- **ML** (machine learning): none ships; the "intelligence" is classical control and statistics. (Ch 1, 1.7; Ch 5, 5.10)
- **mTLS** (mutual TLS): both sides present certificates; used between nodes with TLS 1.3 only. (Ch 3, 3.7; Ch 5, 5.2)
- **Mutex**: a lock that one thread holds at a time. (Ch 4, 4.2)
- **MVP-B** ("the static gateway"): the research lead's product decision after the negative M6 result: single-flight, correct freshness, and a configurable static origin cap; the adaptive controller is not promoted. (Ch 7, 7.6; Ch 8, 8.6)

**N**


- **Negative caching**: remembering an error response; PhantomCache does not do it, so the next request tries the origin again. (Ch 2, 2.6.3; Ch 4, 4.10)
- **Node**: in the code, a member identity plus a connection, not necessarily a separate machine you can read from. (Ch 2, 2.2.1)
- **NodeState**: the seven-value state enum (`Joining`, `Active`, `Degraded`, `Suspect`, `Failed`, `Offline`, `Recovering`). (Ch 5, 5.2)
- **NotWired**: the configuration marker for a key that is validated but not read at runtime (`Wiring::NotWired`). (Ch 2, 2.6.1; Ch 4, 4.8)

**O**


- **ODR** (owner decision register entries ODR-1 to ODR-4): ODR-1 records frozen-code findings as limitations; ODR-3 is the commit policy; ODR-4 names six findings that must not be dismissed. (Ch 8, 8.4.1 and 8.4.3)
- **OPEN** (finding status): the finding stands; the register closes nothing. Other statuses are RECORDED-LIMITATION, IN-SANDBOX-REMEDIATION, UNDER-INVESTIGATION and REFUTED. (Ch 8, 8.4.1)
- **Open-loop / closed-loop** (load): open-loop fixes arrival times in advance regardless of responses; closed-loop waits for each response. (Ch 6, 6.12.1)
- **Origin**: the system that owns the real data; in PhantomCache, any HTTP server behind `ORIGIN_URL`. (Ch 1, 1.1)
- **Owed invalidation**: an invalidation the issuer still has to deliver to a peer; held in memory only, so lost if the issuer crashes (G4B-09). (Ch 5, 5.5 and 5.7)

**P**


- **Partition** (network): a link cut where both sides stay alive; the cut-off side keeps serving its cached copies. (Ch 5, 5.7)
- **Partitioned cache vs local-copy cache**: a partitioned cache gives each key one owner and routes reads to it; a local-copy cache lets several nodes hold the same key. PhantomCache is the second. (Ch 5, 5.1)
- **Percentile** (P50, P99): the value below which that share of requests fell; each place in the repository computes it differently. (Ch 6, 6.12.1 and 6.12.2)
- **PhantomMesh**: the name of the node-to-node binary frame protocol. (Ch 5, 5.2)
- **Phantom Lab**: the experiment toolkit: deterministic simulator T1, real-gateway loopback runner T2, a Lab API and a UI area. Not a benchmark suite. (Ch 7, 7.1)
- **PIE**: short for Policy Intelligence Engine, the name `CLAUDE.md` (section 6b) gives to the project's planned policy-decision component. The legacy `src/control/pie.rs` (`PieController`) is a threshold stub, never constructed (`pie: None` in `main.rs`), and is not the intelligence; `PieDecisionEngine` in `src/intelligence/engine.rs` is a library that no request calls. Neither is a working canary controller. (Ch 5, 5.13)
- **Pinned**: an entry with an active operation or in-flight fetch; it may not be evicted. (Ch 4, 4.5)
- **Placeholder**: a `KeyState` with no entry, created at the start of a fetch so the key can be tracked and pinned. (Ch 4, 4.5)
- **Poisoned mutex**: a `std::sync::Mutex` left poisoned when a thread panics while holding it; most core paths recover. (Ch 4, 4.2; Ch 6, 6.10)
- **Pre-registration**: writing down the analysis rule before opening the sealed data, so the verdict cannot be chosen afterwards. (Ch 7, 7.4.4)
- **Primary** (canonical primary): the node rendezvous hashing picks to acknowledge a DELETE first. (Ch 5, 5.3 and 5.5)
- **Property test** (`proptest`): a test that runs random inputs and shrinks failures; random search is not proof. (Ch 3, 3.12)
- **Protected core**: `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml`; changes need a failing-first test, independent review and owner approval. (Ch 8, 8.7.1)
- **Provenance label**: the tag on a value saying how it was produced (measured, simulated, inferred, unavailable). (Ch 2, 2.8; Ch 7, 7.7.2)
- **PROTOTYPE ONLY**: status label for code that exists but is not wired into the product or not integrated. (Ch 8, 8.1)

**Q**


- **Quota** (tenant): a per-tenant resource limit. None exists; one tenant can use the whole byte budget. DESIGNED BUT NOT IMPLEMENTED. (Ch 6, 6.6; Ch 8, 8.9.2)

**R**


- **RAII guard**: a Rust object that releases a resource when dropped, so cancellation and panics do not leak permits or in-flight state. (Ch 4, 4.5)
- **Read-through**: the gateway fetches from the origin on a miss and stores the result. (Ch 2, 2.2)
- **Read-your-delete**: after a DELETE returns on a gateway, later reads on that gateway do not return the old body; weaker than linearizability. (Ch 5, 5.6)
- **Rendezvous hashing**: give each node a score from the key and node id; the highest score owns the key. In PhantomCache the weight is a constant 1.0 and the result only picks the DELETE primary. (Ch 5, 5.3)
- **Repair ledger**: a bounded in-memory record of invalidations a peer still owes or is owed; re-sent every 250 ms until acknowledged. (Ch 5, 5.5)
- **Revalidation**: ask the origin whether a stored copy is still good, using `If-None-Match`; a 304 answer renews the copy. (Ch 4, 4.8)
- **Revocation** (certificate): removing a member's trust. The machinery exists but nothing triggers it in a running process. (Ch 5, 5.2; Ch 6, 6.4)
- **RFC 9111**: the HTTP caching standard; PhantomCache implements a subset ("RFC 9111-style"), not a verified compliant cache. (Ch 4, 4.8)
- **Ring buffer**: a fixed-size buffer in which the newest records overwrite the oldest; holds sent invalidation records for replay. (Ch 5, 5.5)
- **Role** (RBAC): the permission carried in the token: `cache:read`, `cache:invalidate`, `telemetry:read`, `scenario:operate`, `admin`. (Ch 2, 2.10; Ch 6, 6.2)
- **RwLock**: a lock allowing many readers or one writer; guards the epoch snapshot. (Ch 3, 3.5.4)
- **rustls**: the Rust TLS library used instead of OpenSSL. (Ch 3, 3.7)

**S**


- **S3-FIFO**: an eviction policy with a small queue, a main queue and a ghost queue. Implemented as `FaithfulS3Fifo`; IMPLEMENTED, PARTIAL (open High finding G4A-02); used only if selected. (Ch 4, 4.9)
- **s-maxage**: the shared-cache lifetime directive; read before `max-age`. (Ch 4, 4.8)
- **SAN** (subject alternative name): the certificate field holding the member's identity; the code requires exactly one DNS name of the expected form. (Ch 3, 3.7; Ch 6, 6.4)
- **SBOM**: a software bill of materials; not provided. DEFERRED. (Ch 8, 8.3.7 and 8.12)
- **Scan resistance**: a one-time scan over many keys should not push out popular entries. (Ch 4, 4.9)
- **Scenario B**: the T2 experiment on a real gateway over loopback with a scripted origin and three arms; both capped arms are TRADE_OFF_FAILURE. (Ch 7, 7.5)
- **Sealed (evaluation)**: an evaluation whose seed set is kept secret until the analysis rule is frozen; the M6 run was executed once. (Ch 7, 7.4.4 and 7.6)
- **SEC-4A-01**: the rule that tenant-epoch overflow falls back to a global epoch bump so memory stays bounded. (Ch 4, 4.7)
- **Seed / seed sets**: a number that fixes a pseudo-random sequence; the Lab has disjoint tuning, development and sealed seed sets. (Ch 7, 7.2.3)
- **Sequence number** (`stream_seq`): a per-destination counter on invalidations so a receiver can notice a gap. (Ch 5, 5.4)
- **Shadow evaluation**: testing a candidate policy on a copy of traffic before changing anything; library only, with flaws F12 and F13. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Shard** (partition, bucket): one independent piece of the cache table with its own mutex; there are eight. (Ch 3, 3.5.2; Ch 4, 4.2)
- **SIEVE**: an eviction policy with a queue, one visited bit per entry and a moving hand; the default policy. (Ch 4, 4.9)
- **Sign-flip permutation test**: a test that randomly flips the sign of each paired difference to see how often chance gives an effect this large. (Ch 7, 7.3.3)
- **Single-flight**: at most one in-flight origin fetch per key; later requests wait for it. The core stampede defence; per gateway only. (Ch 2, 2.5.2; Ch 4, 4.4)
- **Snapshot** (epoch snapshot): an immutable copy of the tenant epoch map that readers use without blocking writers. (Ch 4, 4.7)
- **Space-Saving**: a top-k counting table that replaces its smallest entry when full; 64 slots in the library detector. (Ch 5, 5.12)
- **SplitMix64**: a small pseudo-random generator used for seeded streams and bootstrap resampling. (Ch 7, 7.2.3)
- **SSE** (server-sent events): a one-way HTTP stream from server to browser used by the dashboard feed. (Ch 2, 2.2.1; Ch 3, 3.4)
- **SSRF** (server-side request forgery): making the gateway fetch an address it should not; blocked by the upstream filter with known range gaps (G4B-15). (Ch 6, 6.8)
- **Stale**: older than its freshness lifetime; not automatically unusable. (Ch 1, 1.3.3; Ch 4, 4.8)
- **Stale-if-error**: serve an expired copy, labelled STALE, when the origin fails, within the window the origin allowed. (Ch 2, 2.6.2; Ch 4, 4.8)
- **Stale-while-revalidate** (SWR): serve an expired copy immediately inside the origin's window and refresh it in the background. (Ch 2, 2.6.2; Ch 4, 4.8)
- **State instance id**: an identity for the `KeyState` object; defends against "state removed and recreated during fetch". (Ch 4, 4.5)
- **Static origin cap**: a fixed limit on concurrent origin fetches; TESTED, and the feature that did show an effect. (Ch 5, 5.14; Ch 8, 8.6)
- **Supply chain**: the third-party dependencies in the build (288 locked packages) and the checks on them. (Ch 3, 3.13; Ch 6, 6.10)
- **Suspect** (node state): set when a real call to a peer fails; the node is skipped for routing and owed invalidations go to the repair ledger. (Ch 5, 5.2)
- **Sweeper**: the background task that reclaims expired and invalidated entries each tick (default 100 ms); it scans a shard under its lock (G4A-03). (Ch 3, 3.3; Ch 4, 4.7)

**T**


- **T1** (tier): simulation with a virtual clock, seeded randomness, real cache and policy code and a modelled origin. Labelled SIMULATED. (Ch 7, 7.1 and 7.2)
- **T2** (tier): real gateway processes on loopback with a scripted origin and an open-loop generator. Labelled MEASURED_LOOPBACK. (Ch 7, 7.5)
- **Tenant**: the customer namespace that is the first part of every key; in secure mode it comes only from the token. (Ch 6, 6.6)
- **Tenant epoch**: the per-tenant invalidation counter stamped on each entry. (Ch 4, 4.7; Ch 5, 5.4)
- **TLS 1.3**: the transport encryption version the cluster requires; a TLS 1.2 client is refused by test. (Ch 3, 3.7)
- **Tokio**: the Rust async runtime on which the gateway, Axum and the TLS libraries run. (Ch 3, 3.3)
- **Tombstone**: a leftover marker; in S3-FIFO, a removed key stays in a queue, so queues can grow (G4A-02). (Ch 4, 4.9)
- **TRADE_OFF_FAILURE**: the mechanical Scenario B verdict for both capped arms: origin load fell but client success fell too. (Ch 7, 7.5.2)
- **TTL** (time to live): how long a copy may be treated as correct; see freshness lifetime. (Ch 1, 1.1)

**U**


- **U-01**: an open Lab finding (a cancelled or partial Scenario B run can carry a completed mechanism verdict); the fix is sandbox only; owner-approved integration (DEC-001) is authorised and in progress, not committed. (Ch 7, 7.5.2; Ch 8, 8.8)
- **UNVERIFIED**: status label for something that could not be read or checked, with the reason stated. (Ch 8, 8.1)
- **Upstream filter**: the SSRF defence on the origin side: allowlist, IP filter and pinned address. (Ch 2, 2.10; Ch 6, 6.8)

**V**


- **Virtual clock**: an integer microsecond counter that the simulator advances; nothing waits for real time. (Ch 7, 7.2.1)
- **Vite / React**: the build tool and UI library for the dashboard, with plain CSS and hand-built SVG charts. (Ch 3, 3.11)
- **Vose alias method**: an O(1)-per-sample way to draw keys from a weighted (Zipf) distribution after one setup pass. (Ch 7, 7.2.3)

**W**


- **Waiter**: see leader. At most 1,000 waiters per key; beyond that the gateway answers 503. (Ch 2, 2.5.3; Ch 4, 4.4)
- **Watchdog**: a library check (`check_watchdog`) that reverts to the safe policy and forces mode `off` if the intelligence loop's last heartbeat is older than a timeout; not wired. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Weighted rendezvous hashing**: rendezvous hashing where a node wins in proportion to its weight, using `-w / ln(u)`. DESIGNED BUT NOT IMPLEMENTED; the code multiplies by a constant 1.0. (Ch 5, 5.3)
- **Wiring**: whether a configuration key is read by the running gateway (`Wiring::Main`) or only validated (`Wiring::NotWired`). (Ch 2, 2.6.1 and 2.9.2)

**Z**


- **Zipf** (Zipfian): a popularity pattern in which a few items receive most requests; used by the Lab workload generators. (Ch 1, 1.3.2; Ch 7, 7.2.4)

# R4. Judge-question bank <!--gen-->

## Appendix E: Consolidated judge-question bank

This appendix collects every judge box from Chapters 1 to 8 into one bank, organised by theme.

**Count.** Counted with `grep` on the fenced-div opener `::: judge`, the chapters hold 93 judge boxes: Chapter 1, 7; Chapter 2, 9; Chapter 3, 10; Chapter 4, 12; Chapter 5, 16; Chapter 6, 15; Chapter 7, 12; Chapter 8, 12. All 93 are below, each exactly once. The themes hold: Fundamentals 5, Architecture 8, Technology 11, Algorithms 13, Distributed systems 15, Security 11, Innovation 7, Evaluation 12, Limitations and honesty 11.

**How to use it.** Each entry gives the question, a short answer taken in substance from the chapter, a pointer to the section where the topic is taught, and the evidence to know. The answers are compressed. They keep the chapter's qualifiers, status labels and open-finding caveats, and they are never stronger than the chapter. Before an oral defence, read the full box in the chapter: it also has the "if the judge asks further" follow-ups. Status labels have the meaning given in Ch 8, 8.1. Nothing in this project is INDEPENDENTLY VERIFIED; every review on record is an AI second look.


### Ten hard questions to rehearse first

These ten are the questions where an overclaim does the most damage. Rehearse them until the honest answer comes out without hedging in the wrong places.

1. A DELETE returned 204. Is the object gone from every node? (DIS-8, DIS-7)
2. Is it linearizable? Do you guarantee bounded staleness? (DIS-12, DIS-13)
3. What if the gateway that issued the delete crashes before the others hear? (DIS-15)
4. Is there a race that could resurrect a deleted value, and is it fixed? (ALG-4, ALG-5)
5. What is the actual intelligence? Is it AI? (INN-4, INN-5)
6. Your own experiment says the adaptive controller does not beat a static cap. Why should anyone use it? (INN-6)
7. Who has independently verified this system? (LIM-2)
8. What are your performance numbers? (EVA-11)
9. Your tests pass. Why should I believe the system is correct? (EVA-12, LIM-1)
10. Where would a hostile reviewer attack first? (SEC-11)


### Never say

These are the tempting overclaims from Ch 8, 8.11, each with the accurate sentence to say instead.

| Do not say | Say instead |
|---|---|
| "PhantomCache is production-ready." | A research prototype with an experimental cluster mode; no release-profile suite, no advisory scan, open High findings, run only on Windows. |
| "It is a distributed cache." | Each gateway has its own cache; the cluster adds an authenticated invalidation bus with repair. GET is served locally. |
| "DELETE returns 204, so the object is gone everywhere." | 204 means the canonical primary acknowledged within a 150 ms leg; others are told asynchronously, and owed invalidations are lost if the issuer dies. |
| "Invalidations survive crashes." | Not in the tree. A sandbox patch with an opt-in log exists and is not integrated. |
| "Zero stale reads after 50 ms" or "bounded staleness." | Not supported. The only gateway bound is the origin lifetime or 60 s; the stale cap bounds stale serving only. |
| "Linearizable" or "strongly consistent." | No. Per-node ordering and eventual convergence under the loopback failure model that was tested. |
| "Adaptive or intelligent caching improves performance." | Not established: H1 NOT_SUPPORTED (simulated); Scenario B TRADE_OFF_FAILURE for both arms. It shows no benefit for this controller on these plants, not that adaptation cannot work. |
| "The PIE is a working canary controller." | `src/control/pie.rs` is a legacy threshold stub, `pie: None` in `main.rs`; `canary` mode applies the cap to all fetches with no rollback in the serving path. |
| "It uses machine learning." | No ML ships. An ML selector is gated and its negative claim is itself unverified. |
| "PhantomCache is fast" or "faster than Redis, Nginx or Varnish." | The repository contains benchmark results with a stated profile, host and limits; none establishes a release-profile or comparative claim, and this book makes no superiority claim (Ch 6, 6.13). |
| "Eviction is 200 times faster" or "controller overhead is below 1% CPU." | Untraceable; no release-build measurement backs them. |
| "Independently audited or verified." | Reviewed by AI second looks; no human or external reviewer has checked anything. |
| "Secure by design / hardened." | Authentication, roles, mTLS and SSRF filtering exist and are tested; open items include the pre-auth body read, live-control mutation, member flush capability, no rate limits and no advisory scan. Not security-certified. |
| "Multi-tenant isolation with quotas." | Tenant-keyed entries and tenant epochs only; no quotas; any authenticated member can flush any tenant. |
| "`PHANTOM_DEFAULT_TTL_MS` sets the default lifetime." | Validated but not wired; the effective default is always 60 s. |
| "SIEVE, LRU and S3-FIFO are benchmarked and superior." | Faithful implementations exist; no release-grade comparison; S3-FIFO has a tombstone defect (unreachable by default). |
| "The UI is finished / accessible." | Increments I1 to I4 are implemented and pass automated checks; visual quality is not approved; no screen-reader or human usability test exists. |
| "Everything is saved / committed." | Almost everything is uncommitted; one local backup on the same machine. |
| "Tests passing means it is correct." | 851 debug tests pass; several assert stub behaviour. |


### 1. Fundamentals

#### FUN-1. Why is caching necessary at all? Why not just make the database faster?

**Answer.** A faster database helps but does not remove the basic problem: most traffic repeats, and a cache answers the repeat from memory so it never reaches the database. Database load drops by roughly the miss rate (at a 95% hit ratio, one request in twenty gets through; illustrative numbers). Caching only works when requests repeat and a slightly old answer is acceptable. No speed-up is claimed for PhantomCache: the repository has no release-build performance measurement.

**Where taught:** Ch 1, 1.2. **Evidence to know:** `docs/user/limitations.md`; `docs/STATUS.md` section 3; counters in `PartitionedCache`; IMPLEMENTED (counters), performance benefit UNVERIFIED on a release build.

#### FUN-2. Why not let the application query the database directly and keep the architecture simple?

**Answer.** For a low-traffic service that is the right answer; a cache is an extra moving part. You need one when callers repeat reads, the database is scarce or traffic surges. The cost is a second system that can be stale, so you must be able to say how stale. The origin is any HTTP server behind `ORIGIN_URL`, and the tests use scripted or mock origins, which show correct coalescing and caching but not how a real production database behaves.

**Where taught:** Ch 1, 1.2. **Evidence to know:** `src/main.rs` (`ORIGIN_URL`); `tests/integration/m1_http_tests.rs`; `docs/demo/scenario_b_interpretation_notice.md` item 4; IMPLEMENTED, TESTED against scripted origins only.

#### FUN-3. What does caching introduce that you did not have before? What are the costs?

**Answer.** Speed and origin protection, paid for with four problems: stale data, invalidation (a race against in-flight fetches), stampedes and hot keys. PhantomCache addresses stampedes with coalescing and the invalidation race with generation fencing, and bounds stale serving with a configurable cap. It eliminates none of them: staleness is bounded only by the origin's lifetime or a 60-second default, and the cluster invalidation path is experimental. Memory pressure and one more component to operate are further costs.

**Where taught:** Ch 1, 1.3.4. **Evidence to know:** `PartitionedCache::begin_fetch_atomic`, `::finalize_fetch`, `::delete`; `src/config.rs`; `coalesce_tradeoffs_tests.rs`; IMPLEMENTED, TESTED (loopback, debug); G4A-01 UNDER-INVESTIGATION.

#### FUN-4. Why do you need a distributed cache rather than one big cache node?

**Answer.** One node has a ceiling on memory, CPU and network and is a single point of failure. Several gateways raise throughput and survive one failure, but then you must keep copies consistent. Today the main distributed feature is reliable invalidation: a delete is sent to every node with retries and anti-entropy. Reads are not distributed, so it is a set of independent gateways with a shared invalidation bus, not a pooled cache; capacity does not add up and the origin can see up to N times one node's load.

**Where taught:** Ch 1, 1.4. **Evidence to know:** `get_cache_handler` (local only), `delete_cache_handler`; `src/dist/router.rs::route_key` (weight 1.0); the "[EXPERIMENTAL] Cluster mode" message; invalidation fan-out IMPLEMENTED, TESTED (loopback, debug build, at most four nodes), effectiveness UNVERIFIED; peer reads and durability NOT IMPLEMENTED.

#### FUN-5. In one minute, what is PhantomCache?

**Answer.** An in-memory HTTP caching gateway in Rust that answers repeat requests from memory and sends one origin request when many clients ask for the same missing item (single-flight coalescing). It bounds stale serving, tracks bytes against a budget, keeps tenant keys separate and supports authenticated delete; several gateways can exchange invalidations over mutual TLS. On top sit an experimental layer that can cap origin concurrency and a lab for reproducible experiments. It is a research prototype: there are tests for the engine and invalidation path, but no release-grade numbers, no multi-host run and no measured benefit from the adaptive layer.

**Where taught:** Ch 1, 1.5. **Evidence to know:** `README.md`; `docs/STATUS.md` sections 1 to 4; `docs/user/limitations.md`; `docs/research/m6_sealed_results.md`; IMPLEMENTED/TESTED engine, PARTIAL cluster, PROTOTYPE ONLY intelligence; nothing INDEPENDENTLY VERIFIED.


### 2. Architecture

#### ARC-1. Explain the architecture of your system.

**Answer.** A read-through caching gateway: the client sends GET, the gateway looks up an in-memory cache split into eight lock-protected shards; a hit answers from memory, a miss sends one request to the origin while others wait, and the result is stored if still valid. DELETE removes the key locally, tells the node chosen by rendezvous hashing and waits for its acknowledgement, then tells the other members in the background. Optional parts are JWT authentication, an experimental mTLS cluster, an observation API and Phantom Lab. Reads are always served from the local cache, so this is N independent caches with an invalidation bus.

**Where taught:** Ch 2, 2.2.1. **Evidence to know:** `src/main.rs::run`; `src/api/server.rs::create_app_with_lab`, `get_cache_handler`; `PartitionedCache`; IMPLEMENTED, TESTED for the single-gateway path; for the cluster IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED.

#### ARC-2. Walk me through what happens to one request, from the network to the response.

**Answer.** The Axum router applies CORS and a body limit; the handler checks the token if authentication is on and takes the tenant from it; it builds and validates a key from tenant, path and query (control characters and dot segments are rejected). The key's hash picks one of eight shards and only that shard's lock is taken. A fresh entry returns with `X-Cache-Status: HIT`; otherwise the miss path runs: one leader fetches, others wait, the result is stored if still valid. The shard mutex is never held across a network call.

**Where taught:** Ch 2, 2.4. **Evidence to know:** `get_cache_handler`, `begin_fetch_atomic`, `CacheKey::validate`; `test_m1_t01_cold_cache_miss`, `test_m1_t02_repeat_cache_hit` (TESTED on loopback, debug, one gateway; they show MISS then HIT and one origin call, not performance).

#### ARC-3. What happens on a cache miss? And what if a thousand clients ask for the same missing key?

**Answer.** The first request becomes the leader and fetches the origin in its own task with timeouts and size limits; every other request for that key joins the same operation, so the origin sees one request. The gateway checks the result may be cached and that the key was not deleted or invalidated meanwhile, stores it if so, and releases all waiters. Waiters are capped at 1,000 per key; extra requests get 503. Coalescing is per gateway, so three gateways can fetch the same key three times; the 100-client case was tested on loopback.

**Where taught:** Ch 2, 2.5. **Evidence to know:** `begin_fetch_atomic`, `finalize_fetch`; `test_m1_t13_*`, `serving_contract_tests` (`test_ct1_*`, `test_ct3_*`, `test_ct7_*`); TESTED, loopback, debug. Fencing and G4A-01 are in Ch 4.

#### ARC-4. What happens when the origin is unavailable?

**Answer.** A fresh cached key is served as normal. For an expired entry it depends on what the origin allowed: with a stale-if-error or stale-while-revalidate window the expired copy is served, labelled STALE, never beyond the 24-hour hard cap; with nothing, the client gets 504 (timeout) or 502 (network failure). Errors are not cached, and coalescing keeps it to one request per key at a time. There is no circuit breaker, and stale-if-error is tested at cache level, not as an HTTP-level outage.

**Where taught:** Ch 2, 2.6.3. **Evidence to know:** `get_cache_handler`, `check_stale_serving`, `map_origin_error_to_response`; `stale_serving_tests::test_stale_if_error_on_origin_outage` (TESTED at component level); `PHANTOM_MAX_STALE_MS` (`Wiring::Main`).

#### ARC-5. The origin goes down. What does the user see, and what protects the origin when it comes back?

**Answer.** A fresh entry is still served; an expired one marked stale-if-error is served as stale up to the 24-hour cap unless marked must-revalidate; a key with nothing usable fails for every request waiting on that fetch, and the next request tries again. On return, single-flight limits a stampede to one request per key; across many keys there is no limit by default, only the optional origin cap, which enforces only in canary or active mode. Stale-if-error is applied to more status codes than the standard allows (G4A-06), and there is no negative caching or circuit breaker.

**Where taught:** Ch 4, 4.10. **Evidence to know:** `stale_serving_tests.rs`, `coalesce_tradeoffs_tests.rs`, `origin_cap_tests.rs`; G4A-06; IMPLEMENTED, TESTED on loopback with in-process mock origins.

#### ARC-6. Your configuration lists a default TTL setting. Does it work?

**Answer.** No, and the product says so. The key is validated and printed, but the freshness engine is called with a fixed 60 seconds in two places in the server; the configuration table marks it not wired and the startup report prints "not honored by runtime". Entries with explicit `Cache-Control` or `Expires` use the origin's lifetime; only responses with no freshness information fall back to 60 seconds, and that cannot be changed by configuration today. Fixing it is an application-code change that needs approval and a test.

**Where taught:** Ch 2, 2.6.1 (box in 2.6.3); Ch 4, 4.8. **Evidence to know:** `src/config.rs::KEYS` (`PHANTOM_DEFAULT_TTL_MS`, `Wiring::NotWired`); the two `60_000` literals in `get_cache_handler`; `docs/user/limitations.md`; PARTIAL.

#### ARC-7. Is the dashboard showing real data? And how does the Lab relate to the real system?

**Answer.** The observation endpoints return counters and latency measured in the running gateway, each field meant to carry a provenance label. Some values, such as the three demo nodes and the startup snapshot, are synthetic and are emitted as live only when simulation mode is on. The Lab is separate from serving: T1 reuses the real cache and policy code on a virtual clock, so its numbers are simulated; T2 runs a real gateway and scripted origin on one machine, so its numbers are measured, on loopback only. Do not quote dashboard numbers as production performance.

**Where taught:** Ch 2, 2.8. **Evidence to know:** `build_schema_v1_payload` (provenance map); `PHANTOM_SIMULATION`; `src/lab/mod.rs`; `src/lab_live/mod.rs`; `telemetry_honesty_tests`, `schema_deprecation_tests` (TESTED).

#### ARC-8. Is this deployable in production today? What exists for deployment?

**Answer.** It is a research and engineering prototype with real hardening, not a production-ready product. What exists: a configuration system that fails closed, a production profile that refuses insecure settings, TLS and JWT options, health endpoints, release packaging scripts with checksums, a smoke test and CI definitions. What does not: container images, orchestration files, service units, graceful connection draining, a gateway-served UI build, durable invalidation across restarts and any multi-host validation. Cluster mode is labelled experimental in the binary.

**Where taught:** Ch 2, 2.9.3. **Evidence to know:** `packaging/`; `src/config.rs::KEYS`; `src/health.rs`; the `main.rs` shutdown comment; `docs/user/limitations.md`; `docs/STATUS.md`.


### 3. Technology

#### TEC-1. Why did you choose Rust for a cache?

**Answer.** The gateway keeps shared mutable state that many threads touch on every request and sits on the latency path; Rust gives native speed with no garbage collector and forces explicit sharing, removing classes of memory and data-race bugs. The repository does not contain a written language decision, so this is reasoning, not a documented trade study. Rust does not prevent logic races: G4A-01 would be a logic race in safe Rust and is UNDER-INVESTIGATION.

**Where taught:** Ch 3, 3.2. **Evidence to know:** `Cargo.toml`; README section 4; `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md`; five `unsafe` blocks in `src/api/lab_api.rs`, none in `src/core`; IMPLEMENTED; language choice UNVERIFIED as a recorded decision.

#### TEC-2. Why not Go or C#?

**Answer.** Both would work and have strong HTTP and concurrency support. No head-to-head was run and the repository records none, so no claim is made that they were rejected on evidence. The reasoning for Rust is control: no collector pauses and visible ownership. The cost is real: Rust is slower to write, and no latency advantage over Go or C# can be shown.

**Where taught:** Ch 3, 3.2. **Evidence to know:** absence of any Go or C# artefact; `bench-results/README.md` (no release-profile result established); reasoning, not recorded; no performance claim.

#### TEC-3. Why Tokio, and what does it give you that threads would not?

**Answer.** Most of the gateway waits on sockets and timers; Tokio lets a few worker threads serve thousands of waiting requests and supplies `oneshot` channels (one origin result to many waiters), bounded channels, semaphores and timeouts. No runtime comparison was done; Axum and `tokio-rustls` are built on Tokio, so it is the natural fit. A blocked task occupies a worker thread: the sweeper scans a whole shard under its mutex (G4A-03, OPEN), so a very large shard could hold a worker longer than the documented 1 ms bound.

**Where taught:** Ch 3, 3.3. **Evidence to know:** `src/main.rs::real_main`; `cache.rs::spawn_sweeper_task`; ADR-002 CT-2; `test_sweeper_lock_hold_time_bounded` (the batch limit bounds candidates, not keys inspected); IMPLEMENTED.

#### TEC-4. Why Axum rather than another framework, and why did you write your own HTTP client for the origin?

**Answer.** Axum is a thin framework on hyper and tower on the same runtime, so routing, state, body limits and CORS were a few lines each; no framework comparison is recorded. The origin client is a small HTTP/1.1 client written to connect to an address already validated against SSRF rules, send a fixed minimal header set, cap the response size and time out every step. The cost is owning a response parser, which has unit and property tests and AI review only, and `Connection: close` means each miss pays a fresh handshake, unmeasured in a release build.

**Where taught:** Ch 3, 3.4. **Evidence to know:** `create_app_with_lab`, `fetch_origin_raw`; `src/security/upstream.rs`; `parser_robustness_tests.rs`, `security_phase2_upstream_tests.rs`; IMPLEMENTED, TESTED; decision not recorded.

#### TEC-5. Why partitioned mutex buckets? Why not a lock-free concurrent map, or one `RwLock`?

**Answer.** Each key has a generation, active operations, an in-flight registration and eviction bookkeeping, and rules like "never evict a key with an active fetch" need several of these updated together; a mutex around a shard makes that atomic, and eight shards cut contention without giving that up. The repository records no comparison with lock-free maps; this is reasoning. The cost is that a hot key or long critical section still serialises on one lock, and the contention benefit has not been measured on a release build.

**Where taught:** Ch 3, 3.5.2. **Evidence to know:** `PartitionedCache`, `get_shard_index`; `PartitionedCache::new(8)` in `main.rs`; `concurrency_verification_tests.rs`; G4A-03, G4A-21; TESTED for correctness, not for performance.

#### TEC-6. Why partitioned mutexes? Why not one lock, a lock-free map, or a concurrent hash map crate? (Chapter 4 version)

**Answer.** One lock serialises every request; eight shards let different keys proceed in parallel, with a key always mapping to the same shard by hash and mask. Critical sections are short because the origin fetch is outside the lock. A lock-free or lock-striped map would scale further, and no benchmark here compares them; plain mutexes are used because entry, byte count, in-flight record and eviction list must change together. Eight is small and hard-coded; the decision record for the number was not found.

**Where taught:** Ch 4, 4.2. **Evidence to know:** `PartitionedCache`, `get_shard_index`; `PartitionedCache::new(8)`; IMPLEMENTED; contention not measured.

#### TEC-7. What about lock contention? How do you know it is not a bottleneck?

**Answer.** It is not known: there is no contention measurement on a physical host. From the code, hit-path lock holds are short, the origin fetch is outside the lock and epoch checks use a read lock. Contention would appear on one hot key (always one shard) and in the sweeper, which scans a whole shard under its lock each tick (G4A-03, OPEN). The measurement to make is per-shard hold and wait time under a skewed workload, on a release build, on a named host, with repetitions.

**Where taught:** Ch 4, 4.2. **Evidence to know:** `docs/release/g4/cycle1_core_engine_review.md` G4A-03; `epoch_contention_tests.rs` (epoch reads, not shard contention); IMPLEMENTED, performance UNVERIFIED.

#### TEC-8. What are the concurrency implications of your design? Where could it deadlock or stall?

**Answer.** Short synchronous critical sections under `std::sync::Mutex`, no lock across `.await`, slow work in a spawned task, results handed over channels; lock order is shard first, then epoch structures, and the invalidation path never takes a shard lock, so no cycle was seen in the reading. Known stall risks, all open: a long sweep holds a shard lock (G4A-03), the epoch snapshot rebuild clones up to 10,000 entries (G4A-12), and some locks panic on poison (G4A-21). There is no deadlock proof, only a lock-order argument and stress tests that did not hang; linearizability is not claimed.

**Where taught:** Ch 3, 3.5.5. **Evidence to know:** `get_effective_epochs`, `try_invalidate_tenant`; `OriginGate` doc comment; `concurrency_verification_tests.rs`, `epoch_contention_tests.rs`; G4A-01, -03, -12, -17, -21, G4B-28; IMPLEMENTED; TESTED (stress and model); not INDEPENDENTLY VERIFIED.

#### TEC-9. Why rustls and not OpenSSL for the mTLS between nodes? And why do you have both `ring` and `aws-lc-rs`?

**Answer.** The recorded reason (Phase 1 specification) is to avoid OpenSSL C-library build problems on Windows; rustls also pins the cluster to TLS 1.3 with a mandatory client certificate, and a test refuses a TLS 1.2 client. Two crypto libraries are linked because `ring` is used directly for SHA-256 and rustls 0.23 defaults to `aws-lc-rs`, so the spec's "no C crypto" is not literally true; choosing one is an owner decision. The identity check (chain, one DNS SAN, fingerprint match) was tested on loopback with generated certificates and has had no independent review.

**Where taught:** Ch 3, 3.7. **Evidence to know:** `src/security/cluster.rs`; `docs/security/phase-1-specification.md` section 8; `Cargo.lock`; `test_p3_sec04_strict_tls13_rejection_of_tls12_client`; IMPLEMENTED, TESTED (loopback); not INDEPENDENTLY VERIFIED.

#### TEC-10. Why React with plain CSS and no UI or chart libraries? Isn't that reinventing the wheel?

**Answer.** The recorded part: the toolkit proposal rejected chart, graph and component libraries on bundle size, dependency count and canvas-chart accessibility, with a guard test pinning the dependency list. The other part is reasoning: every number on screen carries a measured, simulated, inferred or unavailable label, and hand-built components keep that label inside each chart. The cost is more code and a hand-built SSE client. The UI is tested, not approved; there is no human usability or screen-reader test, and no recorded comparison explains React over something lighter.

**Where taught:** Ch 3, 3.11. **Evidence to know:** `ui/package.json`; `ui/test/forbidden.test.mjs`; `docs/design/remediation/07_frontend_redesign_implementation_record.md`; `docs/research/design_toolkit/toolkit_proposal.md`; `docs/STATUS.md` M8 row; IMPLEMENTED, TESTED; visual quality unapproved.

#### TEC-11. Are your dependencies safe? Have you scanned them?

**Answer.** The dependency set is small and permissively licensed, the lockfile has no git or alternate-registry sources and release builds use `--locked`. But neither `cargo audit` nor `cargo deny` has been run, so freedom from known advisories cannot be claimed. Some declared crates are unused and two crypto libraries are linked; consolidating is an owner decision.

**Where taught:** Ch 3, 3.13. **Evidence to know:** `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md`; `docs/STATUS.md` (M7 row); inventory VERIFIED by reading, advisory status UNVERIFIED.


### 4. Algorithms

#### ALG-1. What stops a cache stampede? What happens to the origin when a hot key expires?

**Answer.** Single-flight per key: under the shard lock the first miss registers an in-flight operation, later misses wait on a channel, and the one origin result goes to everyone. A test (`test_negative_effect_head_of_line_blocking`, 10 clients; a separate 100-client HTTP test, `test_m1_t13_barrier_stampede_100_clients`, also exists) fires 10 simultaneous requests at a slow origin and gets 1 miss and 9 coalesced answers; waiters are capped at 1,000 per key and the origin read times out after 5 seconds. Limits: if the origin fails all waiters fail together, there is no negative caching, and across many different keys there is no protection unless the optional origin cap is on (off by default).

**Where taught:** Ch 4, 4.4. **Evidence to know:** `src/core/coalesce.rs`; `begin_fetch_atomic`; `coalesce_tradeoffs_tests.rs`; `origin_cap_tests.rs`; G4A-08 (OPEN); IMPLEMENTED and TESTED on loopback.

#### ALG-2. What happens with a hot-key overload? Say one key gets most of the traffic.

**Answer.** Two problems. A hot key that misses is coalesced into one origin request. A hot key that hits still takes the same shard mutex twice in the GET path and the HTTP layer copies the body per response; the key lives in one shard and is not replicated, because GET is local only. The shard mutex and the body copy are expected to be the limit, but this is unmeasured. The intelligence modules have detection research, which is not a serving-path fix for hit-path hot keys.

**Where taught:** Ch 4, 4.4. **Evidence to know:** `src/api/server.rs` (HIT branch clones the body); G4A-26; `check_stale_serving`, `begin_fetch_atomic`; IMPLEMENTED, hot-hit behaviour UNVERIFIED.

#### ALG-3. How do you stop a stale response overwriting a newer one, for instance after a DELETE?

**Answer.** Each key has a generation; a fetch records it at start, DELETE bumps it, and the fetch stores its body only if generation, state identity and tenant and global epochs still match, all under the shard mutex. Otherwise the body is dropped and `stale_writes_prevented` rises; waiters still get their response. This is not called linearizable: a completed DELETE makes later reads on that node miss (a single-threaded order is tested), but there is no schedule-exploring proof, and for tenant-wide invalidation two findings are open (G4A-01, G4A-08).

**Where taught:** Ch 4, 4.5. **Evidence to know:** `finalize_fetch`, `delete_impl`; `invalidation_contract_tests.rs`; `concurrency_verification_tests.rs` (exhaustive model, 59,049 sequences, with no invariant about fills after tenant invalidation, so it cannot find G4A-01); IMPLEMENTED, TESTED for DELETE.

#### ALG-4. Is there a race that could resurrect a deleted value?

**Answer.** One suspected race, the first High finding in the project's own review, treated as under investigation, not fixed. A fetch checks the epochs when it finishes but reads them a second time after the check, so an invalidation landing in that gap can let the old body be stored as current. It was reproduced only with an artificial pause in a sandbox copy; a short unhooked stress run found nothing. A proposed fix passes sandbox tests but is not in the product code and has not been independently reviewed.

**Where taught:** Ch 6, 6.9.4. **Evidence to know:** `finalize_fetch`, `get_effective_epochs`; `docs/research/g4a01_race/status.md`, `proposal.patch`; `test_p4a_inflight_fetch_fencing_prevents_stale_writes` (passes; does not exercise the gap); G4A-01 UNDER-INVESTIGATION; IMPLEMENTED with a known hole; fix PROTOTYPE ONLY.

#### ALG-5. You say fencing prevents stale writes. Is there a race? Has it been reproduced? Is it fixed?

**Answer.** Yes, one known race. In the last step of a fetch the cache checks the tenant epochs, does more work, reads the epochs again and stamps the entry with the second reading; tenant invalidation does not take the shard lock, so a bump in that gap makes an old body look current. It was reproduced only in a sandbox with a test hook; a twenty-second stress run without the hook found nothing, which does not prove it cannot happen. A small fix passes in the sandbox but is not integrated because fencing changes need independent review and a failing-first test; the register keeps it open and blocking. The DELETE fence is tested; the tenant-epoch fence has this gap.

**Where taught:** Ch 4, 4.7. **Evidence to know:** `docs/research/g4a01_race/` (analysis, status, repro_attempts, proposal.patch); G4A-01 in the disposition register; defect CONFIRMED in sandbox, fix NOT integrated.

#### ALG-6. Explain epochs. Why not just delete the keys? What does invalidation guarantee?

**Answer.** Deleting every key of a tenant means walking the table under locks. Instead each tenant has a version number stamped on every entry; invalidation increases it, and older entries are treated as missing when touched, with a background sweeper reclaiming the rest. A fetch that started before the bump also cannot store its answer. The guarantee is local to one node and lazy, with two documented exceptions and one open race; past 10,000 tenants the node raises a global epoch instead, which is safe but heavy and can be triggered by a peer (G4A-12, open). It is not atomic across nodes.

**Where taught:** Ch 4, 4.7. **Evidence to know:** `try_invalidate_tenant`, `get_effective_epochs`; `epoch_contention_tests.rs`, `invalidation_contract_tests.rs`; G4A-08, G4A-12, G4A-17; IMPLEMENTED, TESTED for basic and overflow paths.

#### ALG-7. How do you know the cache does not leak memory? Is the byte count the real memory use?

**Answer.** The count is logical (key size, body capacity, header strings, fixed overhead), and the code can recompute it from the maps and compare. Tests drive every path that changes an entry and assert the totals match, including a concurrent stress run; that shows the bookkeeping is consistent, not that the operating system sees the same number, because freshness strings, hash-table slack and S3-FIFO queues are not counted. One known path drifts: a panic mid-finalize (G4A-13, open). No measured resident-memory figure for a release build exists.

**Where taught:** Ch 4, 4.6. **Evidence to know:** `audit_bytes`, `calculate_state_bytes`; `byte_accounting_tests.rs`; `memory_bound_soak_tests.rs`; G4A-13, G4A-23; IMPLEMENTED, TESTED (consistency only).

#### ALG-8. How does expiry work? What stops the cache serving very old data? Is the default TTL configurable?

**Answer.** The origin's `Cache-Control` decides lifetime (shared-cache max-age, then max-age, then `Expires`); with no information the lifetime is 60 seconds. `PHANTOM_DEFAULT_TTL_MS` is not wired, so changing it does nothing. An expired entry is served only if the origin allowed stale-while-revalidate or stale-if-error, never with must-revalidate, and never beyond the 24-hour cap (`PHANTOM_MAX_STALE_MS`, wired); the cap limits stale serving, not fresh lifetime. Known gaps: upstream `Age` ignored, wall-clock time, stale-if-error applied to more status codes than the RFC allows. It is "RFC 9111-style", not compliant; no conformance suite was run.

**Where taught:** Ch 4, 4.8. **Evidence to know:** `src/config.rs`; `src/api/server.rs` literal `60_000`; `check_stale_serving`; `freshness.rs::evaluate_response_freshness`; G4A-04, -05, -06, -10, -18, -25; IMPLEMENTED (subset), TESTED for cap and SIE paths.

#### ALG-9. Why SIEVE as the default? Is eviction really O(1)? You also have S3-FIFO; is it safe?

**Answer.** SIEVE keeps a queue and a visited bit per entry, so a hit only sets a bit and does not rewrite a list under the shard lock; eviction advances a hand that clears bits. No recorded decision makes it the default, and no test measures its hit ratio against LRU on a real workload. It is expected O(1) for hit, insert and remove and amortized O(1) for eviction when nothing is pinned, not worst-case O(1). S3-FIFO has an open High finding: a removed key stays in the queue, so queues can grow and early evictions can happen; it is used only if selected.

**Where taught:** Ch 4, 4.9. **Evidence to know:** `src/core/eviction.rs` (`FaithfulSieve`, `FaithfulLru`, `FaithfulS3Fifo`); `eviction_contract_tests.rs`; G4A-02, G4A-20 (OPEN); `docs/research/eviction_policy_audit.md`; SIEVE and LRU IMPLEMENTED, TESTED (weakly); S3-FIFO IMPLEMENTED, PARTIAL (G4A-02 OPEN); literature UNVERIFIED. "Faithful" is the code's name, not a verified property.

#### ALG-10. State the time and space complexity of the cache operations.

**Answer.** With n entries in a shard and key length k (at most 4,096 bytes): lookup is O(k) hashing then expected O(1); insert and delete are expected O(1) plus O(1) policy bookkeeping; eviction is amortized O(1) with no pinned keys; coalescing a request is O(1); memory is bounded by the byte budget and entry limit. Three exceptions: tenant invalidation is O(T) in tenants with a recorded epoch (up to 10,000), the sweeper scans O(n) of a shard under its lock each tick, and switching policy is O(n) per shard. A hit also copies the body.

**Where taught:** Ch 4, 4.9 (box) and 4.1. **Evidence to know:** `cache.rs` (`get_shard_index`, `try_invalidate_tenant`, `sweep_lazy_reclamation`, `set_eviction_policy`); G4A-03, -12, -20, -26; asymptotics from reading code, none measured here.

#### ALG-11. You say you use weighted rendezvous hashing. How does the weight enter, and what does the system do with it?

**Answer.** Each node scores each key from a hash of key and node id and the highest score owns the key; everyone computes the same scores, and removing a node moves only its keys. Weighted form uses minus weight over the natural log of the normalised hash. In the code the weight is a constant 1.0, so it is plain rendezvous hashing; no capacity-aware placement is claimed. The result only picks the node that acknowledges a DELETE first; reads are not routed. The code multiplies the raw hash by the weight, which is not proportional (a quick standalone calculation gave 75% for weights 1 and 2, not 67%).

**Where taught:** Ch 5, 5.3. **Evidence to know:** `src/dist/router.rs::route_key` (`capacity_weight = 1.0`), `get_replicas`; only `invalidate_key_across_cluster` calls `route_key` in production; equal-weight rendezvous IMPLEMENTED; weighted DESIGNED BUT NOT IMPLEMENTED; no uniformity test in the repo.

#### ALG-12. How accurate are your latency percentiles, and how do you compute them without slowing the request path?

**Answer.** The log-linear histogram has 2,688 counters, constant-time recording and relative error at most 0.39 percent by construction, checked by tests. But the percentiles on the metrics endpoint are computed from a 10,000-sample window sorted on demand, exact for that window, with the sort and two locks on the observation path. So the 0.39 percent is the recording design, not the accuracy of displayed numbers. Percentile conventions across the repository are also inconsistent (floor versus nearest-rank).

**Where taught:** Ch 5, 5.11. **Evidence to know:** `src/intel/telemetry.rs` (`value_to_bucket`, `calculate_percentiles_us`, `record_us`); `telemetry_redesign_tests.rs`; `src/api/server.rs`; histogram IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader and is not the live percentile source.

#### ALG-13. How do you detect and handle hot keys?

**Answer.** Handling is live: concurrent misses for one key are coalesced into one origin request, and a cached hot key is served locally by every gateway. Detection is a library detector (Count-Min Sketch plus a 64-key Space-Saving table and an emergence rule), tested in isolation but not wired into the gateway; the metrics endpoint reports hot keys as unavailable. So claim hot-key protection through coalescing and hot-key detection as a built-but-unused component. The sketch overestimates only, with error bounded by the standard Count-Min result; there is no measured error on production traffic.

**Where taught:** Ch 5, 5.12. **Evidence to know:** `src/intelligence/signals.rs::HeavyHitterDetector`, `HotKeyEmergenceDetector`; `src/core/coalesce.rs`; `server.rs` `"hot_keys"` unavailable; `src/intel/sketch.rs` unused; coalescing IMPLEMENTED and TESTED; detection PROTOTYPE ONLY.


### 5. Distributed systems

#### DIS-1. How do the nodes communicate?

**Answer.** A custom binary protocol on TCP inside mutual TLS: each node proves identity with a certificate whose name and fingerprint are checked against a static membership file. The main message is an invalidation, sent to the member chosen by rendezvous hashing, which has up to 150 milliseconds to acknowledge, then to the others in the background with a bounded queue, retries and a repair worker. There are also heartbeats and a reconciliation exchange. Reads do not use the network between nodes. A down node is marked suspect and owed the invalidation in an in-memory ledger, lost if the issuer restarts first.

**Where taught:** Ch 2, 2.7.4. **Evidence to know:** `src/dist/frame.rs`; `router.rs::invalidate_key_across_cluster`; `node.rs::start_tcp_service`; `docs/user/limitations.md`; IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED (no multi-host evidence). The choice over Raft or gossip has no recorded decision.

#### DIS-2. How do your nodes discover each other? What if I add a node?

**Answer.** Membership is static: a JSON file lists every node, its id and certificate fingerprint, read at startup. Nodes talk over mutually authenticated TLS 1.3 with a small binary protocol; there is no gossip and no dynamic join. To add a node you edit the file, bump its version and restart the members. A runtime reload and revocation path exists in the library and is tested, but nothing in the shipped gateway triggers it (G4B-07, open); dynamic membership is not claimed.

**Where taught:** Ch 5, 5.2. **Evidence to know:** `ClusterMembershipRegistry::update` (no production caller); `src/main.rs` cluster branch; G4B-07; static membership IMPLEMENTED and TESTED; dynamic membership DESIGNED BUT NOT IMPLEMENTED. Avoiding gossip or consensus is our reasoning, not a recorded decision.

#### DIS-3. Which node serves a read for a key, and does rendezvous hashing decide that?

**Answer.** The gateway that received the request serves it from its own cache. Rendezvous hashing today only picks the node that must acknowledge a DELETE first. The code to forward a read to the key's owner exists as a function but nothing calls it, and the peer-side read handler is a stub that fabricates a value, so distributed reads are not claimed. The hash gives a deterministic primary for invalidation; the weight is a constant 1.0.

**Where taught:** Ch 2, 2.7.4. **Evidence to know:** `RendezvousRouter::route_key`, `forward_key_over_tcp` (no caller), the `GetRequest` arm of `process_binary_cache_stream`; ADR-003 (agent-recorded, not owner-ratified per `docs/STATUS.md`).

#### DIS-4. Walk me through a read. Which node serves it, and where does the hash come in?

**Answer.** The contacted gateway looks in its own cache; on a miss it fetches from the origin with single-flight and stores the result with whatever freshness the origin allowed. No other gateway is consulted, so the hash is not involved; it matters only for deletes. The same key can be cached on every node: local copies with invalidation fan-out. The cost is that a node that missed an invalidation can serve a stale copy.

**Where taught:** Ch 5, 5.3. **Evidence to know:** `get_cache_handler`; `forward_key_over_tcp` (no production caller); ADR-003; local-only GET IMPLEMENTED and TESTED (`known_gaps.rs`, `distributed_consistency_tests.rs`).

#### DIS-5. You have heartbeats. So does the cluster detect a dead node?

**Answer.** Not by heartbeat. The heartbeat keeps idle connections alive every five seconds and discards dead ones, but changes no node's state. A node becomes Suspect when a real call to it fails, usually the first invalidation after it died, which spends its 150 ms deadline; after that it is skipped for routing and its owed invalidations go to a repair ledger. Detection is lazy and traffic-driven; wiring heartbeat results into the state machine is the obvious fix (G4B-12). The UI can show a node as Failed only after the demo control sets a flag, which is inferred, not detected.

**Where taught:** Ch 5, 5.2. **Evidence to know:** `src/dist/pool.rs::probe_idle_connections` (no `NodeState` access); `server.rs::control_handler`; G4B-12, G4B-03; failure detection by heartbeat is not implemented.

#### DIS-6. What is an epoch here, and does it give you a global version of the cache?

**Answer.** There is no global version. Within one gateway, tenant and global epochs are counters stamped on entries; bumping one makes older entries stale without scanning. Between gateways the counters are not synchronised. What crosses the wire for a normal delete is the exact key, a per-destination sequence number to notice lost messages, and the membership version, which is carried but not enforced on invalidations (open finding G4B-06). A shared tenant counter would need agreement, which the system lacks.

**Where taught:** Ch 5, 5.4. **Evidence to know:** `try_invalidate_tenant`, `try_invalidate_all`, `get_effective_epochs`; the `InvalidateRequest` handler in `src/dist/node.rs`; G4B-06, G4A-01; local epochs IMPLEMENTED and TESTED; cross-node epoch enforcement not implemented.

#### DIS-7. A DELETE returns 204. What exactly has been invalidated at that point?

**Answer.** Two things for sure: the gateway that received the DELETE dropped its own copy, and the chosen primary confirmed it dropped its copy, within a 150 millisecond deadline. Every other node is told afterwards, in the background, with retries and a repair ledger, guaranteed only while the issuer stays alive. Sometimes the primary is the issuer itself, so 204 then says nothing about any remote node. A delayed 204 until all members acknowledge was considered in the M3 design and rejected there as out of scope; it is an open product decision.

**Where taught:** Ch 5, 5.5. **Evidence to know:** `delete_cache_handler`; `invalidate_key_across_cluster` (doc comment states the contract); ADR-003 amendment 2026-10-30; G4B-09; IMPLEMENTED and TESTED as described; cluster-wide acknowledgement DESIGNED BUT NOT IMPLEMENTED.

#### DIS-8. A client sends DELETE and gets 204. Is that content guaranteed gone from every node?

**Answer.** No. 204 means the issuing gateway purged its own cache and the canonical primary acknowledged within 150 milliseconds, and that primary can be the issuer itself. Other members are informed asynchronously with retries, a repair worker and anti-entropy, but the list of invalidations still owed lives only in the issuer's memory. If the issuer is killed first, peers keep serving the old object until its freshness lifetime ends; this was reproduced on unmodified code, and the durable-log fix is a sandbox patch, opt-in, not in the tree. For deletion-critical content the documentation advises a short `s-maxage`, re-issuing DELETEs after any restart, and verifying through each gateway.

**Where taught:** Ch 8, 8.13.2 (see also Ch 5, 5.5 and 5.7). **Evidence to know:** G4B-09; `docs/user/limitations.md` "Cluster invalidation durability"; `m3_durability/reproduction_report.md`; `claims_ledger.md` C6 amendment; durability PROTOTYPE ONLY. No staleness bound is offered beyond the origin's lifetime (60 s when the origin gives none); a "zero stale after 50 ms" claim was withdrawn.

#### DIS-9. What if the invalidation message is lost or arrives late?

**Answer.** Three layers: the sender retries up to four times with short backoff; anything undelivered goes to a per-peer repair ledger and is re-sent every 250 milliseconds with the same sequence number until acknowledged; and a receiver that sees a gap can pull the missed records from the sender's ring buffer, flushing its whole cache if the history was overwritten (safe but coarse). All of it is bounded in memory. A late message is harmless because a delete is idempotent. The one case none of this covers is the sender dying while it owes messages; with a permanent cut or crashed sender the only bound is the entry's own expiry.

**Where taught:** Ch 5, 5.5. **Evidence to know:** `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound` (loopback, debug; a two-second cut, convergence asserted within 800 ms of healing); `RepairLedger`, `repair_peer`, `reconcile_peer_anti_entropy`; IMPLEMENTED and TESTED; sender crash UNHANDLED.

#### DIS-10. What happens if an invalidation is lost or delayed? Can two nodes serve different values?

**Answer.** Yes, briefly. After a DELETE the issuing gateway misses immediately and the primary has acknowledged before 204; others are told in the background with retries, a repair record and a gap-detecting pull. On a warm loopback cluster propagation was mostly milliseconds, with a first-use cold start of tens of milliseconds, in debug builds, and during that window another node can serve the old value. If a peer is down it can stay stale until repaired; if the issuer crashes first the owed record is lost and the peer stays stale until expiry. The claim is eventual repair while the issuer stays alive, not bounded staleness.

**Where taught:** Ch 6, 6.9.3 to 6.9.4. **Evidence to know:** `invalidate_key_across_cluster`, `RepairLedger`; `distributed_consistency_tests`, `cluster_followup_tests`, `cluster_propagation_latency_tests`; claims ledger C6 amendment and qualifier; G4B-09 (OPEN); M3 design PROTOTYPE ONLY; status PARTIAL. The test that would falsify the issuer-alive claim (blackhole a peer, DELETE, kill the issuer) does not exist.

#### DIS-11. Can two of your nodes serve different values for the same key at the same time?

**Answer.** Yes, by design each gateway has its own copy. They differ for milliseconds after a delete while the invalidation is in flight, for as long as a partition lasts if a gateway is cut off but serving, and until expiry if the issuing gateway crashed before telling everyone. They can also differ with no delete, because two gateways fetched at different times and the origin changed in between. The claim is a mechanism that converges after a delete when the system is healthy, with what is not covered stated plainly.

**Where taught:** Ch 5, 5.6. **Evidence to know:** ADR-003 section 4 and its 2026-10-30 amendment; `claims_ledger.md` C6 amendment and qualifier; `distributed_consistency_tests.rs`; convergence IMPLEMENTED and TESTED on loopback; staleness bound UNVERIFIED and not claimed.

#### DIS-12. Is your cache linearizable? Do you guarantee bounded staleness?

**Answer.** No to both. Linearizability across nodes would order every read against every delete, and reads are local. There is a per-gateway guarantee that a read after a completed delete misses, with two open findings about rare races. Bounded staleness needs a number, a failure model and a test: in the healthy, issuer-alive case the observation is milliseconds on loopback in a debug build, which is not a guarantee; in the crash or permanent-partition case the bound is the entry's freshness lifetime, set by the origin. The ADR's earlier "24 hours" statement was amended on 2026-10-30 because the cap bounds the stale-serving window, not the freshness lifetime.

**Where taught:** Ch 5, 5.6. **Evidence to know:** ADR-003 amendment; `claims_ledger.md` C6 qualifier; `src/core/freshness.rs` (no gateway ceiling); neither claim is made.

#### DIS-13. Is this linearizable? What is your consistency model?

**Answer.** Not linearizable, and no such claim is made. Each node orders its own fills and deletes with generation fencing; across nodes the model is eventual convergence of invalidations under the tested loopback fault model: N independent caches and an invalidation bus. Numbered consistency clauses with a failure model, a bound and a falsifying test have been drafted, but they await owner ratification and several are known to be unmet. Even single-node fencing is not proven airtight: the double epoch read (G4A-01) was reproduced only with an injected yield, and a three-line fix is proposed and not applied because the file is protected.

**Where taught:** Ch 8, 8.13.2. **Evidence to know:** G4A-01; `docs/research/g4a01_race/`; `src/core/cache.rs::finalize_fetch`; product-direction section 7.

#### DIS-14. What happens if a node fails? What if the network partitions?

**Answer.** If a node dies, calls to it fail fast, it is marked suspect and skipped, and the invalidations it missed are queued in a bounded ledger on the sender; when it returns its cache is already empty (memory only), the ledger is delivered and it rejoins after a pull reconciliation. If the network splits with both sides alive there is no leader, so the cut-off side can serve stale copies until the link heals or the entries expire. Reads chose availability; staleness during a partition is bounded only by the entry's freshness lifetime. A suspect node does not stop serving on the HTTP path; the rule exists only in the peer handler.

**Where taught:** Ch 5, 5.7. **Evidence to know:** `test_m3_04a_suspect_to_active_automatic_recovery`, `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`; no `NodeState` in the GET path of `server.rs`; `test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits` (peer path only); receiver recovery IMPLEMENTED and TESTED (loopback); no staleness bound.

#### DIS-15. What if the gateway that issued the delete crashes before the others got the message?

**Answer.** Those nodes keep serving the old entry until it expires. The owed messages are only in the issuer's memory and a restarted issuer starts empty. This was reproduced in a sandbox: three gateways, the issuer killed right after answering 204, and the third gateway did not converge in ten seconds. It is open in the main tree. A fix exists as a sandbox design (an opt-in durable log replayed after restart, with 15 process-level crash tests) but is not integrated; until then the right claim is "guaranteed while the issuer stays alive". The damage is bounded by the entry's freshness lifetime, and it affects only entries whose invalidation was in flight.

**Where taught:** Ch 5, 5.7 and 5.9. **Evidence to know:** `docs/research/m3_durability/` (`reproduction_report.md`, `design.md`, `remediation/00_status_and_plan.md`); `claims_ledger.md` C6 qualifier; G4B-09; defect REPRODUCED in a sandbox; fix PROTOTYPE ONLY.


### 6. Security

#### SEC-1. How do you stop an unauthorised user reading or deleting cached data?

**Answer.** With a JWT secret configured, every cache read, delete, metrics, stream and control call needs a signed token; the gateway checks signature, issuer, audience and expiry, then that the role allows the action (`cache:read`, `cache:invalidate`), and takes the tenant from the token, never a header, before any lookup or fetch. Without a secret the gateway starts only on loopback and refuses a network address without a secret and TLS. Not claimed: tokens cannot be revoked before expiry, there is no not-before check, and the issuer is outside this repository. In the GET and DELETE handlers the body is extracted before authentication (G4B-01, High, owner-named, OPEN).

**Where taught:** Ch 6, 6.2. **Evidence to know:** `jwt.rs::JwtValidator::validate`; `server.rs::authenticate_request`, `require_tenant_claim`; `m7_sec_fixes_tests::sec01_*`, `m7_security_review_tests`, `m7_http_rbac_matrix_in_secure_mode`; G4B-01, -13, -14; IMPLEMENTED, TESTED for covered paths; revocation, `nbf` and lifetime caps absent.

#### SEC-2. Your control endpoint changes cluster node state. Who can call it, and what stops a demo control from breaking production?

**Answer.** In secure mode it needs the `scenario:operate` role; without a JWT secret it is anonymous, possible only on loopback. It is a demo control and is not isolated from the real router: in a cluster process it operates on the same node objects the router uses, and an unknown event sets every node Active (G4B-03, owner-named, open). Marking a node Failed changes the DELETE primary and makes the node drop incoming connections, without corrupting cache data. It is not safe to expose in production.

**Where taught:** Ch 6, 6.3. **Evidence to know:** `server.rs::control_handler`; `main.rs` (nodes shared with the router); the accept loop in `node.rs`; G4B-03 (OPEN, ODR-4); IMPLEMENTED, not isolated; no cluster test.

#### SEC-3. How do your nodes trust each other, and what if one node is compromised?

**Answer.** Mutual TLS 1.3 with a private cluster CA: a certificate with exactly one DNS name encoding node id and cluster id, a node id in the membership file and marked active, and a fingerprint matching the pinned one, re-checked on every frame; an invalidation frame's claimed sender must equal the certificate identity. A compromised member is trusted: it can send tenant-wide or full-cache invalidations (past 10,000 tenant names costing every tenant its cached data). Revocation exists in code and tests but is not triggered in a running process; the remedy is to remove the member from the file and restart. The peer `GetRequest` stub does not expose HTTP entries (keys starting with `/` are refused); that conclusion is an AI investigation, not independent review.

**Where taught:** Ch 6, 6.4. **Evidence to know:** `security/cluster.rs::ClusterTlsConfig::from_pem`, `validate_peer`; `node.rs::start_tcp_service`; `security_phase3_cluster_tests`, `cluster_followup_tests::test_l3_invalidate_request_originator_must_match_tls_identity`; G4B-05/06/07/08, DR-027; admission IMPLEMENTED and TESTED; scope authorisation DESIGNED BUT NOT IMPLEMENTED.

#### SEC-4. Your node protocol has a GET request. Can a peer poison your cache with it?

**Answer.** The peer GET is a placeholder: on a miss it invents a value named `val_node_` plus the node id and stores it. No production code sends the request, it needs a valid pinned cluster certificate, and the planted key lives in a key space HTTP requests cannot reach because peer keys must not start with a slash. So it does not poison what clients read today, but it can fill the cache with junk and evict real entries and is a hazard for any future peer-fill. It is tracked as DR-029 with a test that pins the behaviour; the fix awaits an owner decision.

**Where taught:** Ch 5, 5.8. **Evidence to know:** `src/dist/node.rs` GetRequest arm; `known_gaps.rs::pin_kg6_node_get_request_fabricates_missing_key_body`; `m7_sec_round3_tests.rs`; DR-029, G4B-30; PROTOTYPE ONLY; design defect OPEN.

#### SEC-5. What happens if someone sends a malicious request: a huge body, a slow body, a crafted path?

**Answer.** Crafted paths and keys are handled well: strict alphabet and length limits, control characters and dot segments rejected with 400, and the path re-encoded before going upstream, with tests. A huge body is capped at 512 KiB and gets 413. A slow body is the weakness: no read timeout or connection cap on the main API, and the body is read before authentication (G4B-01, the High-severity security finding, open and owner-named). The fix is known and already done in the Lab API but not applied to the main API, so it is not hardened against slow-client exhaustion.

**Where taught:** Ch 6, 6.5. **Evidence to know:** `create_app_with_lab` (`DefaultBodyLimit`); `get_cache_handler` signature; `lab_api.rs::runs_start`; `test_m1_t26_ingress_request_body_limit`, `h1_illegal_bytes_in_path_are_400_and_origin_receives_nothing`; G4B-01; key validation TESTED; slow-body protection absent. Changing `server.rs` needs the owner's decision.

#### SEC-6. How is tenant isolation enforced, and could one tenant ever see another's data?

**Answer.** The tenant is part of every cache key and, in secure mode, comes only from the signed token; tenant names are restricted so one cannot spoof another's prefix, and an invalidation bumps only that tenant's epoch. Tests cover tenant-claim handling, spoofed headers and cross-tenant reads. Limits: isolation is logical, in one process and one shared byte budget, with no per-tenant quotas, so a heavy tenant can evict others; a cluster member can address any tenant in an invalidation; and `__default__` is refused in secure mode but the key validator still accepts it (G4A-19 not closed). Only path, query and tenant influence the origin response and all are in the key; origins must not vary responses on other client data.

**Where taught:** Ch 6, 6.6. **Evidence to know:** `CacheKey::new`, `validate`; `server.rs::require_tenant_claim`; G4A-19, G4B-05, G4B-11, G4B-30 / DR-029; TESTED on HTTP paths; quotas DESIGNED BUT NOT IMPLEMENTED.

#### SEC-7. How do you prevent resource exhaustion and protect the origin from overload?

**Answer.** Memory is bounded by a byte budget with exact accounting and fixed caps on waiters, the tenant epoch map, frames, repair queues, origin response size and timeouts; many requests for one key collapse to one fetch; an optional origin concurrency cap sheds load with stale serving or 503, and tests show permits are always released. Missing: the cap is off by default and per process, cache-busting keys or the bypass header reach the origin without limit, and there is no rate limit or connection cap on the HTTP side (G4B-01, -02, -11). DoS resistance is not claimed. In the loopback experiment a static cap reduced peak origin concurrency but lowered client success, reported as a trade-off.

**Where taught:** Ch 6, 6.8. **Evidence to know:** `intelligence/executor.rs` origin gate; `origin_cap_tests.rs`; `lab-results/scenario_b/*/summary.json`; `scenario_b_interpretation_notice.md`; G4B-11 (OPEN); bounded memory IMPLEMENTED and TESTED; rate limiting DEFERRED (SEC-M7-11 queued).

#### SEC-8. Could a user make your gateway fetch an internal address (SSRF)?

**Answer.** A user cannot choose the host: the origin is one configured URL and users supply a validated, re-encoded path and query. If the origin name resolved to a private or metadata address, the gateway resolves it, checks every returned address against a block list and connects to the exact address it checked, so DNS changes in between cannot help. Dangerous ports are blocked, redirects are not followed and client credentials are not forwarded by default. Known gap: a few rarely used reserved ranges are missing from the block list (G4B-15), and rebinding defence was reviewed by reading, not an automated rebinding test.

**Where taught:** Ch 6, 6.8. **Evidence to know:** `security/upstream.rs::resolve_and_pin_origin`, `PreConnectIpFilter`, `UpstreamPolicy::validate_target`; `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses`, `test_p2_ssrf_cloud_metadata_blocked`, `test_p2_ssrf_mixed_dns_blocked`; G4B-15; IMPLEMENTED, TESTED; range gaps OPEN. `PHANTOM_ALLOW_LAN_ORIGIN` is flagged high-risk.

#### SEC-9. Is the system memory-safe, and how do you manage third-party risk?

**Answer.** Our cache, security and cluster code contains no `unsafe`; the only unsafe in the product is a small commented Windows-only block in the Lab for killing child processes. Third-party crates have their own unsafe code, unaudited. Versions are pinned in `Cargo.lock` (288 packages) and TLS is in rustls on the product path, but no `cargo audit` has been run, some features are enabled and unused, and there are two crypto providers; the scan is a release gate needing a human decision. Rust stops memory corruption, not logic failure: lock poisoning and no catch-panic layer are findings G4A-21 and G4B-28.

**Where taught:** Ch 6, 6.10. **Evidence to know:** `grep unsafe` result (`lab_api.rs` Windows module; bench alloc counter); `Cargo.toml`, `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (G4C-DEP); IMPLEMENTED; scan UNVERIFIED.

#### SEC-10. Can one tenant, or one cluster node, hurt another?

**Answer.** Yes, in two ways. Tenants are separated by key and invalidation epoch, but there are no quotas, so one tenant can consume the byte budget. And any authenticated cluster member can send tenant-scope or full-cache invalidation frames without the receiver checking scope against the sender's rights (DR-027 or G4B-05, owner-named, must not be dismissed); the plan is per-member scope authorisation and tenant-flood caps, in a sandbox, queued behind durability. It needs a valid cluster certificate, so the attacker is an insider or compromised member. The `GetRequest` stub writes a placeholder value, but key spaces are disjoint: low as exploited today, medium as a latent hazard.

**Where taught:** Ch 8, 8.13.2. **Evidence to know:** G4B-05, G4B-30; `docs/security/investigations/DR-029_*`; `src/dist/node.rs` (`val_node_` format); status PARTIAL for security.

#### SEC-11. Where would a hostile reviewer attack first?

**Answer.** At the HTTP ingress and the control endpoint: bodies are extracted before authentication with no body timeout, connection limit or rate limit, so a slow-sender attack is plausible (High, owner-named), and `/api/control` parses its JSON before checking the token. Second, the cluster: static membership, long-lived connections that never re-check certificates, and members who can flush every cache. Third, the supply chain: no advisory scan on 288 locked packages. All are documented; the stated mitigation is a deployment assumption (a proxy with limits), not a proven property. These findings are confirmed by reading code, not by running attacks.

**Where taught:** Ch 8, 8.13.4. **Evidence to know:** G4B-01, -02, -03, -05 to -08, -11; G4C-DEP; `server.rs::control_handler`; status PARTIAL.


### 7. Innovation

#### INN-1. What is novel about PhantomCache?

**Answer.** No single mechanism is claimed as new: coalescing, stale-while-revalidate, leases against stale writes, adaptive concurrency limits and anti-entropy all exist in Varnish, nginx, Memcached, Envoy and Cassandra, and the project's own prior-art record is open and says novelty may not be claimed. What can be defended is the combination and the method: explicit invariants (fencing, epochs, stale caps, origin gate, byte accounting) with tests, and a deterministic simulator that drives the real cache code with a preregistered mechanical verdict and an honest negative result. Whether the combination is better is unknown; no head-to-head measurement exists.

**Where taught:** Ch 7, 7.8 (box in 7.9). **Evidence to know:** `docs/research/rt0_prior_art_sweep_2026-10-30.md` (status PARTIAL, RT-0 OPEN); `novelty_snapshot_m6.md` (correction notice); `claims_ledger.md` sections 6-7.

#### INN-2. Why not just use Redis or Memcached?

**Answer.** For a general key-value cache you should; they are mature and feature-rich. PhantomCache is a different shape: an HTTP gateway in front of an origin that applies HTTP freshness rules, serves stale under a hard cap, coalesces misses and caps origin concurrency, which with Redis you would write in the application. Memcached leases and Redis client tracking address parts of the stale-write and invalidation problem; generation fencing is the same family of idea. The trade-off is less maturity, no persistence and no clustering depth, and no benchmark shows it is faster.

**Where taught:** Ch 7, 7.8.2 (box in 7.9). **Evidence to know:** `rt0_prior_art_sweep_2026-10-30.md` T3 (Facebook leases, OPENED-DIRECT) and T10 (Redis tracking); `src/core/cache.rs` (in-process cache); Chapters 2 and 4.

#### INN-3. What is the actual intelligence in PhantomCache? Is it AI?

**Answer.** It is not machine learning and not called AI. The word covers telemetry (counters and a latency histogram), a library of statistical signals and a decision engine for shadow evaluation written for experiments, and one control rule on the live path: an origin-concurrency cap, off unless canary or active mode is on and a cap configured, fixed or AIMD. When tested in a pre-registered sealed simulation, the adaptive version did not beat the best fixed cap (hypothesis not supported). The working part today is a static origin cap; the adaptive research is a prepared lab, not a proven feature.

**Where taught:** Ch 1, 1.7. **Evidence to know:** `intelligence/mod.rs::IntelligenceMode`; `executor.rs::origin_cap_enforcing`; `main.rs` (`pie: None`); `m6_sealed_results.md`; `scenario_b_interpretation_notice.md`; `CLAUDE.md` section 6b; origin gate IMPLEMENTED and TESTED; decision engine PROTOTYPE ONLY; adaptive benefit NOT ESTABLISHED (H1 NOT_SUPPORTED, simulated).

#### INN-4. What is the actual intelligence in the system? (Chapter 7 version)

**Answer.** Adaptive control, not machine learning. The part touching live traffic is an origin concurrency cap with an optional AIMD controller: if measured origin P99 exceeds a ceiling or errors exceed 5%, cut the allowed concurrency by 30%, otherwise add one. The library also has a dynamic stale-window rule, a frequency-based admission filter and a shadow and canary framework that the serving path does not call. The legacy PIE controller is a threshold stub that is not instantiated. The adaptive version did not beat a static cap, so the product decision is to ship the static cap path as the MVP.

**Where taught:** Ch 7, 7.8.7 (box in 7.9). **Evidence to know:** `intelligence/candidates.rs::OriginConcurrencyCapController::step`; `cycle1_lab_intelligence_deps_review.md` section 5; `m6_sealed_results.md`.

#### INN-5. What is the actual intelligence in your system? Is it AI? (Chapter 5 version)

**Answer.** It is not AI: there is no learned model. On the live path there are latency and hit telemetry and one AIMD cap on concurrent origin fetches, the rule TCP uses. Beside it, a library covers workload detection, shadow evaluation of eviction policies, a statistical decision engine, canary routing and a watchdog; it is tested but not wired into the gateway. The legacy PIE stub is a threshold rule and is not constructed in the shipped binary. The sealed pre-registered simulation showed the adaptive controller did not beat a good static cap, so the defensible feature is the static cap.

**Where taught:** Ch 5, 5.14. **Evidence to know:** `intelligence/executor.rs`, `candidates.rs`, `engine.rs`; `src/control/pie.rs`; `m6_sealed_results.md`, `pie_audit.md`; G4C F12, F13, F14; AIMD cap IMPLEMENTED, TESTED (arithmetic and cap unit tests; serving path only when a cap is configured in canary or active mode), benefit NOT established; the rest PROTOTYPE ONLY or library.

#### INN-6. Your own experiment says the adaptive controller does not beat a static cap. Why should anyone use the adaptive mode?

**Answer.** On current evidence they should not, and the documentation says so. The sealed result is simulated, so it does not prove adaptation never helps, but it gives no evidence that it does. The feature that showed a large effect in the simulator is the static cap itself, so the product position is a configurable static cap with adaptive mode as an experimental option. The negative result was kept because hiding it would be worse than lacking a positive one. The simulator is deterministic and uses the real executor and cache code, and a loopback run (Scenario B) found the controller acting as a static cap near 12, but neither is a multi-host measurement.

**Where taught:** Ch 5, 5.14. **Evidence to know:** `m6_sealed_results.md`; `claims_ledger.md` section 6 (C1 NOT SUPPORTED, narrowed to Tier-1); `lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`; `docs/user/limitations.md`; adaptive benefit NOT established; static cap IMPLEMENTED.

#### INN-7. What is your strongest technical contribution?

**Answer.** Two things, ranking the method above the mechanisms. First, the evaluation discipline: a deterministic simulator running the real cache and executor, a preregistered rule applied by code, a hash-committed sealed set, and a negative result reported. Second, the cache core's explicit invariants (generation fencing, epochs, hard staleness cap, byte accounting), each with tests. Neither is called a breakthrough: careful engineering and honest reporting. The strongest evidence is the M6 sealed run (12,000 cells, 30 sealed seeds, H1 NOT_SUPPORTED), which has not been independently reviewed.

**Where taught:** Ch 7, 7.8.4 (box in 7.9). **Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`; `src/lab/m6_pipeline.rs::compute_verdict`; `docs/release/known_limitations_frozen_code.md` section 1.


### 8. Evaluation

#### EVA-1. You have hundreds of tests. Do they prove your concurrent cache is correct?

**Answer.** No. The suite includes useful kinds of test: a protocol model checked exhaustively, a mutation detector, concurrent stress schedules over real threads and property tests. They give real confidence in specific invariants, but run in a debug build on one machine, stress tests sample interleavings rather than enumerate them, and the reported race G4A-01 is not settled by the suite. The honest statement is tested for the invariants named, not proven, not independently reviewed. Adding a deterministic checker such as loom, a G4A-01 regression test that fails without a fix, and release-profile runs is future work, not done.

**Where taught:** Ch 3, 3.12. **Evidence to know:** `concurrency_verification_tests.rs` (`test_exhaustive_protocol_model_invariants`, `test_protocol_model_mutation_detector`, `test_real_cache_concurrent_stress_schedule`, proptest); G4A-01; `claims_ledger.md`; TESTED (named invariants). `loom` is not a dependency.

#### EVA-2. How did you measure performance, and why should I trust the numbers?

**Answer.** There are two kinds of real measurement, both on one Windows laptop on loopback: micro-benchmarks repeated ten times in separate processes with bootstrap intervals, and a 30-run loopback experiment on origin protection built in release. The micro-benchmarks' build profile is not established (the benchmark README says no release-profile result is established), they predate the current tree, no other cache was measured, and no multi-host run exists. The same benchmark repeated across four sessions gave non-overlapping intervals, so the within-session interval understates machine variation and no single figure is quoted. None establishes a release-profile or comparative claim and the book makes no superiority claim; treat them as local, dated evidence, not a product claim.

**Where taught:** Ch 6, 6.12.3 (see 6.13 for every measurement). **Evidence to know:** `bench-results/r0_summary.json`, `bench-results/manifests/*.json`, `benches/*.rs`; `claims_ledger.md`; status MEASURED (local, dated), not INDEPENDENTLY VERIFIED.

#### EVA-3. Is your benchmark fair? Could you be fooling yourselves?

**Answer.** The micro-benchmarks compare the implementation with itself, not a competitor, so the main risks are noise and coordinated omission, handled by repetitions and recorded limits (hybrid-core laptop, Windows scheduler). For the origin-cap experiment a fairness review found real problems, published in a notice: the adaptive arm collapsed to a fixed cap near the minimum, arms differ by more than the cap because capped arms run in canary mode, success-only latency must be reported separately, cap values were chosen after one pilot run, and the origin has no load-dependent slowdown. The micro-benchmark and gateway load are closed-loop; Scenario B's generator is open-loop. The result is informative about mechanics, not evidence the adaptive controller works.

**Where taught:** Ch 6, 6.12.3. **Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`; `lab-results/scenario_b/20261030T003932Z/summary.json`; `bench-results/r0_summary.json` `limitations_verbatim`; MEASURED_LOOPBACK with recorded qualifications.

#### EVA-4. Your results are on loopback with a simulator. How do you know this reflects reality?

**Answer.** Simulation models the system and workload on a virtual clock (repeatable, fast); a real run has the actual gateway process on real sockets, even on one laptop. They are labelled apart: T1 simulated, loopback measured on real binaries. The simulated study found no adaptive benefit and the loopback study found a cap lowers peak origin load at a client-success cost; neither says anything about a real network, several machines or production traffic, and loopback shares one CPU between load generator, gateway and origin. No agreement study between T1 and T2 was run (claims ledger C7: UNVERIFIED, F05), so no "simulation predicts reality" claim is made.

**Where taught:** Ch 6, 6.13. **Evidence to know:** `docs/glossary.md` section 5 tiers; `lab-results/README.md`; claims ledger C1, C7; Chapter 7; MEASURED_LOOPBACK and SIMULATED kept separate.

#### EVA-5. What evidence do you have for your claims? How do you know your results are not an artefact of the simulator?

**Answer.** Each tier shows something different. T1 runs the real cache code against a modelled origin and tests policy logic and cache semantics, not networking; T2 runs the real gateway on loopback with a scripted origin. No agreement study was run, so it cannot be said the simulator predicts reality. The simulation is deterministic and tested for determinism, and the analysis is preregistered and mechanical; the main result is negative, which is not what a flattering simulator would give, but that does not show it is right. The simulator could hide a real benefit: the dynamic stale window never bound at the registered horizon, and the T2 origin has no overload feedback.

**Where taught:** Ch 7, 7.5 and 7.8.5 (box in 7.9). **Evidence to know:** `src/lab/matrix_runner.rs::AgreementStudy` and its synthetic-fixture test; claims ledger C7 UNVERIFIED; `verdict.json` H2 section.

#### EVA-6. What is the difference between your simulation and a real test?

**Answer.** In T1 the clock is virtual, origin and clients are models, and the real cache and policy code run in-process as a library; nothing touches a network or HTTP stack, and results are exactly repeatable. In T2 the real gateway runs as a child process, the load generator uses real sockets and real time, and the origin is a small scripted server: real, but loopback on one machine with shared CPU, no network latency and an origin with no queue. Neither is a production or multi-host measurement; the interface labels them Simulated versus Real with the qualifier "loopback, one host". T2 alone is not enough because real-time runs are not repeatable, cost minutes each and cover one scenario.

**Where taught:** Ch 7, 7.5.3 (box in 7.9). **Evidence to know:** `src/lab/t1_sim.rs` header; `src/lab_live/mod.rs`; `ui/src/lib/labProvenance.js`; `scenario_b_interpretation_notice.md`.

#### EVA-7. How would I reproduce one of your experiments?

**Answer.** Every run folder has a manifest with source file hashes, git head, build profile, seeds, scenarios and arm configurations. Check out the same source, build in release and run the same `RunSpec` with the `phantom-lab` command; the cells file should be byte-identical on the same machine. For the sealed M6 run nobody can regenerate the seeds without the secret salt, but the analysis can be re-run on the retained cells to check the verdict. T2 Scenario B is real time, so a re-run gives a fresh sample that should agree within the intervals, not byte for byte. Cross-platform determinism has not been tested.

**Where taught:** Ch 7, 7.4.6 (box in 7.9). **Evidence to know:** `src/lab/eval.rs::run_matrix_with_progress` (manifest fields); `docs/release/reproduction_guide.md` (exists; not read for the chapter); `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes`.

#### EVA-8. What does "H1 NOT_SUPPORTED" actually say, and what does it not say?

**Answer.** On 30 sealed seeds, 8 origin-degradation scenarios and 4 held-out workload families plus one sealed family, an AIMD controller allowed to range at least 4 times its minimum did not reduce peak origin concurrency in any scenario versus the tuned static baseline, and was credibly worse in seven. It reduced client P99 in two scenarios and broke the hit-ratio and error-rate limits in three. It does not say adaptive caching cannot work, says nothing about real networks and does not test overhead or the stale-window rule. The baseline BL-12b is not claimed to be the best deployable static policy.

**Where taught:** Ch 7, 7.6 (box in 7.9). **Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`; `docs/research/m6_sealed_results.md` "What this does and does not establish".

#### EVA-9. Scenario B shows a 64% drop in origin load for the adaptive arm. Isn't that a win?

**Answer.** No. The adaptive arm fell from 32 to about 12 concurrent fetches within two seconds and stayed there, behaving as a static cap of about 12. Client success dropped by about 17 percentage points because refused requests count as failures, and the mechanical verdict for both capped arms is trade-off failure. The experiment shows that a cap is enforced by the real binary and that the price is refused requests. Why it stayed near 12 (a window maximum of 30 samples against an absolute 60 ms ceiling) is our reading of the notice and code, not a recorded root-cause analysis.

**Where taught:** Ch 7, 7.5.2 (box in 7.9). **Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`; `lab-results/scenario_b/20261030T003932Z/summary.json`; `lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`.

#### EVA-10. The sealed evaluation says H1 is NOT_SUPPORTED. What does that mean for your "intelligent" caching claim, and what happens to the product?

**Answer.** There is no evidence the adaptive controller beats a well-tuned static origin cap, so that claim is not made. In the simulated, pre-registered, run-once evaluation the adaptive arm did not reduce peak origin concurrency in any of eight scenarios and in seven was credibly worse; the unconstrained tuning optimum was itself a static cap. The research lead chose the static gateway as the minimum viable product and did not promote the adaptive controller to the serving path. It does not prove adaptation is useless. The run was reviewed by the lead and AI reviewers for hash-chain integrity, not by a human, the reviewers did not read the simulation core, the P99 endpoint was quantised and the stale-window mechanism was never exercised.

**Where taught:** Ch 8, 8.13.3 (see Ch 7, 7.6). **Evidence to know:** `docs/research/m6_sealed_results.md`; `known_limitations_frozen_code.md` section 1; status TESTED, tier SIMULATED. Next step: a headroom experiment.

#### EVA-11. What are your performance numbers?

**Answer.** None are published, deliberately. The numbers in the repository come from simulation, loopback on one host, or benchmark runs whose build profile is not established; none is a release-grade product measurement, and earlier figures such as a 200 times eviction speedup and a 0.45 microsecond controller overhead are untraceable to any retained artefact, so they were withdrawn. There are micro-benchmarks with bootstrap intervals from 2026-10-28 that predate the current tree, and one release-profile loopback experiment (Scenario B), but no release-profile general suite and no soak run. A claim would need a release build, stated hardware, workload and seed, a baseline, repetitions and intervals. A hit is an in-memory map lookup behind a shard lock; how fast, and how it compares with Nginx or Varnish, cannot be said.

**Where taught:** Ch 8, 8.13.3 (every retained measurement is in Ch 6, 6.13). **Evidence to know:** `claims_ledger.md` C3, C5; `docs/user/limitations.md`; `CLAUDE.md` section 4; status DEFERRED.

#### EVA-12. Your tests pass. Why should I believe the system is correct?

**Answer.** You should not believe correctness from passing tests, and the project's rules say so. The 851 tests run in debug mode on one Windows machine; some assert stub behaviour (the legacy PIE tests), the stale-cap counter is close to a tautology and a UI fixture masked a real defect. The tests do give regression protection and invariant checks such as byte-accounting equality and generation fencing in single-process cases. For more, there are named gaps: no mutation test showing fencing tests fail without the fence, no model checking, no release-profile run. The weakest test is the one behind the stale-cap violation counter (G4A-18), described by the reviewer as near-tautological; this was not re-derived.

**Where taught:** Ch 8, 8.13.1 (see Ch 8, 8.2.2). **Evidence to know:** `docs/research/code_findings.md` F3; G4A-18; G4C F01 fixture at `ui/test/lab.view.test.mjs`; `claims_ledger.md` section 4.


### 9. Limitations and honesty

#### LIM-1. Your tests pass. Does that mean the system is correct and ready?

**Answer.** No. The tests are evidence, not proof. The last full recorded run is 851 passing tests in a debug build on one Windows machine, which shows the behaviours the authors thought to check hold in that setting, not release builds, Linux, several machines or sustained load. Some tests assert known stubs. Every review so far is an AI second look, and the status page says nothing has been formally accepted by the owner and findings remain open. It is described as a research prototype with a tested core and an experimental cluster.

**Where taught:** Ch 1, 1.11. **Evidence to know:** `docs/STATUS.md` sections 1 to 3b; the disposition register; `docs/user/limitations.md`; IMPLEMENTED, TESTED (debug, loopback); INDEPENDENTLY VERIFIED: none. Open items to name: lost owed invalidations on issuer crash, member-wide purges, the unwired default TTL, no release-profile suite.

#### LIM-2. Who has independently verified this system?

**Answer.** No human has. Every review so far is an AI second look: a separate AI session or the lead re-running a command, all inside one tooling environment. What exists is breadth: 851 recorded debug tests, three read-only reviews with 99 findings, an independent Node recomputation of the Scenario B results and a hash-checked freeze chain for the sealed evaluation. A human reviewer, an external security review and a physical multi-host run are blocked, waiting on reviewers and devices. An AI second look counts as evidence that a second reader found things the first missed (G4 found a High finding the authors had not), but not as independence.

**Where taught:** Ch 8, 8.13.1. **Evidence to know:** `docs/release/workstream_completion_ledger.md` section 6; `docs/release/verification_register.md` (independence classes); label INDEPENDENTLY VERIFIED: none.

#### LIM-3. What is still unresolved? Give me the honest list.

**Answer.** There are 99 findings from three AI reviews: 75 open, 20 recorded as limitations in frozen research code, 2 in sandbox remediation, 1 under investigation and 1 refuted; no Critical. The High ones are the unauthenticated slow-body weakness at the HTTP ingress, a suspected timing hole in the stale-write fence, a conditional S3-FIFO memory growth problem (no live caller selects it) and a Lab verdict labelling bug whose fix is in a sandbox. The owner-flagged Medium items are the control endpoint touching the live router, member authorisation for tenant-wide invalidations, the missing epoch check on invalidation frames, static membership with no live revocation, and connections that never age out. No dependency scan, penetration test or fuzzing has been done, and none of the reviews is independent human review.

**Where taught:** Ch 6, 6.14 (see Ch 8, 8.4). **Evidence to know:** `cycle1_findings_disposition_register.md` section 3; `owner_decisions_register.md` ODR-1 to ODR-4; `known_limitations_frozen_code.md`; all findings OPEN unless the owner dispositions them. The counts are the disposition agent's; "CONFIRMED" means confirmed by reading code, and items cited were re-read, not exploited.

#### LIM-4. What would you improve first, and what are your real bottlenecks?

**Answer.** Security first: authenticate before reading the body and add timeouts and connection limits to the main API (the one High finding that is unauthenticated and remote); then an owner decision on cluster member authorisation; then durable owed invalidations. For performance, the evidence shows the hit path is cheap and contention appears with many threads and many tenants; this should be profiled on a quiet machine with a release build before changing anything, plus a multi-host open-loop experiment, because everything measured is on one laptop. No features before the High findings are dispositioned, and no throughput numbers without full conditions.

**Where taught:** Ch 6, 6.14. **Evidence to know:** G4B-01, G4B-05, G4B-09; `bench-results/r0_summary.json` B2 results; `docs/release/lead_working_state.md`; status: plan, not implementation.

#### LIM-5. Where are established products stronger than PhantomCache?

**Answer.** Many places: production maturity and tooling in Redis, Memcached, Varnish and nginx; cross-node duplicate suppression and hot-item mirroring in groupcache, where PhantomCache's GET is local-only; invalidation durability, where cross-node invalidations are best-effort after the first ack and owed invalidations are lost if the issuer dies; adaptive concurrency in Envoy and Netflix using relative signals while PhantomCache's absolute ceiling did not beat a static cap; eviction research over thousands of traces; per-tenant quotas, which are absent; and head-to-head performance numbers, which do not exist. First fix: invalidation durability, then a load-dependent test origin, then a relative-signal controller under a new preregistration.

**Where taught:** Ch 7, 7.8.6 (box in 7.9). **Evidence to know:** claims ledger C6 qualifier; `src/dist/router.rs` (constant weight); `rt0_prior_art_sweep_2026-10-30.md` T7, T10; Ch 8 limitations.

#### LIM-6. You report that your adaptive controller failed. Why should I trust anything else?

**Answer.** Reporting the failure is the reason to trust the process: the rule was written before the sealed seeds were used and applied mechanically by code. The weaknesses are listed: the amendment forcing the controller to be adaptive was chosen after seeing development results, the stale-window mechanism was never exercised at that horizon, the primary P99 includes quickly refused requests, and nobody independent has reviewed the pipeline. The failure does not mean adaptive control is useless; it means this controller, with an absolute ceiling, shows no benefit in this simulation. A new hypothesis (relative-signal controller, load-dependent origin) needs a new protocol.

**Where taught:** Ch 7, 7.6 (box in 7.9). **Evidence to know:** `preregistration_m6_v2_2_amendment.md` sections 3, 5, 7b; `m6_sealed_results.md`; `rt0_prior_art_sweep_2026-10-30.md` T7.

#### LIM-7. What would you improve with more time? (Chapter 7 version)

**Answer.** In order: make invalidations durable; make GET able to use the owner, or at least cluster-wide single flight, which is where groupcache is ahead; make the Scenario B origin load-dependent and add a cap-free canary arm; run a real T1-to-T2 agreement study; write a new preregistration for a relative-signal controller; run an overhead benchmark in release on a named host; and get an independent human review of the Lab core and a BCa cross-check against a reference implementation. Nothing is claimed faster or more consistent than anything until a fair head-to-head run exists.

**Where taught:** Ch 7, 7.8.5 (box in 7.9). **Evidence to know:** `docs/release/known_limitations_frozen_code.md` section 5; claims ledger C3, C6, C7; Ch 8 roadmap.

#### LIM-8. What would you do with more time? (Chapter 8 version)

**Answer.** In order: get a human to review the protected core, since everything known about it is AI second looks; integrate the three sandbox fixes after review (durable invalidation log, the fence fix for the double epoch read, the Scenario B cancelled-run fix), each with a test that fails without it; close the ingress holes; run a release-profile suite and soak on a quiet machine and the cluster on real hardware over a network; only then start tenant quotas and the headroom experiment. The protected core is frozen for evidence integrity and safety: changes to fencing and invalidation need a failing-first test, independent review and the owner's approval.

**Where taught:** Ch 8, 8.13.5 (roadmap 8.8 to 8.9). **Evidence to know:** `docs/release/execution_register.md`; `owner_decisions_register.md` ODR-2 to ODR-4; Figure 8.2.

#### LIM-9. What have you chosen not to claim, and what is untested?

**Answer.** Not claimed: production readiness, a distributed cache, linearizability, bounded staleness, adaptive benefit, ML, performance numbers or independent verification. Untested: any Linux run, any physical network, clusters beyond four nodes, power loss and fsync-error behaviour, a release-profile suite, a two-hour soak, the natural-schedule rate of the fence race, ingress defaults under attack, and human usability and screen readers. Also unverified: whether the simulator ranks scenarios as a real process would, because no agreement study was run. The worry that ranks highest is the unreviewed fencing and invalidation core, because the product's one hard promise is that a deleted object is not served after an acknowledged delete.

**Where taught:** Ch 8, 8.13.5 (checklist 8.10, claims 8.11). **Evidence to know:** 8.10 checklist; 8.11 table; `m3_durability/remediation/00_status_and_plan.md` section 7.

#### LIM-10. Your repository is mostly uncommitted. Isn't that a process failure?

**Answer.** It is a delivery risk and it is listed first. The cause is a deliberate policy: commit only individually reviewed file groups, never everything at once and never groups with unresolved blocking findings. Documentation and tooling groups are committed; code groups wait for the review gate and an owner check on held preregistration files. The risk is real: one machine and one local backup. The mitigation is to commit reviewed groups as they clear and to make an off-machine copy, still an open action. No push has occurred; commit and push need an explicit owner instruction.

**Where taught:** Ch 8, 8.13.5 (see 8.2.1). **Evidence to know:** `docs/release/owner_decisions_register.md` ODR-3; `docs/STATUS.md` section 5b; `git log --oneline -8`.

#### LIM-11. The README and an older handoff say the system passed a formal acceptance. Why do your records say otherwise?

**Answer.** Those statements came from agent self-reports and were later audited. The handoff's "Phase 4 accepted, zero P0" is not supported by later review: no dated owner acceptance is recorded for any milestone, the M3 completion report said "zero compromises" before a durability defect was reproduced, and the M6 report was retracted. The old documents are kept as history with correction notices, and `docs/STATUS.md` and the verification register are the entry points. The owner may hold acceptance records elsewhere; the register calls the absence an evidence gap, not proof that none was given.

**Where taught:** Ch 8, 8.13.5 (see 8.7.2). **Evidence to know:** `docs/handoff/CORRECTIONS.md`; `docs/STATUS.md` section 1; `CLAUDE.md` section 6.

## The full chapter judge boxes that are not in Part I <!--gen-->

*Each box is unchanged. Boxes sit under the heading of the section where they were taught.* <!--gen-->

### Chapter 1: The Problem and the Vision <!--gen-->

#### 1.2 Why applications need caching <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why is caching necessary at all? Why not just make the database faster?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Making the database faster helps, but it does not remove the basic problem. Most real traffic repeats: many users ask for the same few items. A database answers every repeat from scratch, which costs a query, a connection and often disk or network time. A cache answers the repeat from memory in a fraction of the time and, more importantly, the repeat never reaches the database. The database load then drops by roughly the miss rate. If the hit ratio is 95%, the database sees one request in twenty. That headroom is what keeps the database alive during a traffic surge. A faster database is still worth having, but it is a one-off multiplier. A cache changes the shape of the load.

**If the judge asks further:**

Caching only works when requests repeat and when a slightly old answer is acceptable. If every request is unique, or if every answer must be perfectly current, a cache gives little or nothing. I would not claim a specific speed-up for PhantomCache: this repository contains no release-build performance measurement. The numbers I use to explain the idea are illustrative.

**Evidence to know:** `docs/user/limitations.md` (no published performance numbers); `docs/STATUS.md` section 3 (no release-profile suite); counters in `src/core/cache.rs::PartitionedCache` and `GET /api/v1/metrics`. Status: IMPLEMENTED (counters); performance benefit UNVERIFIED on a release build.
:::

::: judge
**JUDGE MAY ASK**

Question: Why not let the application query the database directly and keep the architecture simple?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

For a low-traffic service that is the right answer. A cache is an extra moving part with its own failure modes, so you should add one only when you need it. You need it when many callers repeat the same reads, when the database is the scarce resource, or when traffic arrives in sudden surges. In those cases direct access means the database load equals the full request rate, and any surge goes straight through. A gateway in front of the database turns a thousand identical simultaneous requests into one database query, and keeps working reads going when the database is slow. The cost is that you now own a second system that can be stale, so you must be able to say how stale and how it is corrected.

**If the judge asks further:**

PhantomCache is an HTTP gateway, not a database client library. The origin is any HTTP server reached through `ORIGIN_URL`, so the "database" in this project is really whatever sits behind that URL. I should also say that the project's tests use scripted origins and a mock origin. They show the gateway coalesces and caches correctly. They do not show how a real production database behaves behind it.

**Evidence to know:** `src/main.rs` (`ORIGIN_URL`), `tests/integration/m1_http_tests.rs` (mock origin), `docs/demo/scenario_b_interpretation_notice.md` item 4 (scripted origin has no load-dependent latency). Status: IMPLEMENTED, TESTED against scripted origins only.
:::

#### 1.3 Where caching goes wrong <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What does caching introduce that you did not have before? What are the costs?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

A cache gives speed and origin protection, and in return it introduces four problems. Stale data: a copy can lag the origin, so you must decide how stale is acceptable and enforce it. Invalidation: when the origin changes, every copy must be corrected, and that is a race against in-flight fetches. Stampedes: when a popular copy expires, many requests miss together and hit the origin at once. And hot keys: a few popular keys concentrate load on one lock or one node. PhantomCache addresses stampedes with request coalescing and addresses the invalidation race with generation fencing. It bounds stale serving with a configurable cap. It does not eliminate any of these: staleness is still bounded only by the origin's lifetime or a 60-second default, and the cluster invalidation path is experimental.

**If the judge asks further:**

I can add memory pressure as a fifth cost: the cache must evict, and PhantomCache keeps a logical byte budget (`PHANTOM_MAX_CACHE_BYTES`, default 256 MiB) with accounting invariants audited by `audit_bytes()`. A sixth is operational: one more component to run, monitor and secure. Finally, the cache can hide origin problems or amplify an origin failure if a leader's error is shared with many waiters, which the project tests as a trade-off.

**Evidence to know:** `src/core/cache.rs::begin_fetch_atomic`, `::finalize_fetch`, `::delete`; `src/core/coalesce.rs::SharedOp`; `src/config.rs` (`PHANTOM_MAX_STALE_MS`, `PHANTOM_DEFAULT_TTL_MS` NotWired, `PHANTOM_MAX_CACHE_BYTES`); `tests/integration/coalesce_tradeoffs_tests.rs`. Status: IMPLEMENTED, TESTED (loopback, debug); G4A-01 UNDER-INVESTIGATION.
:::

#### 1.7 Pillar 2: Workload intelligence <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What is the actual intelligence in PhantomCache? Is it AI?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is not machine learning, and I would not call it AI. The word covers three things. First, telemetry: counters and a latency histogram. Second, a library of statistical signals and a decision engine that can compare a candidate policy against a baseline in a shadow evaluation, written for experiments. Third, one control rule on the live path: an origin-concurrency cap that is off unless you turn on the canary or active mode and configure a cap. The cap can be fixed or adjust itself with an additive-increase, multiplicative-decrease rule. When we tested whether the adaptive version beats the best fixed cap, in a pre-registered, sealed simulation, it did not: the hypothesis was not supported. So the honest answer is that the useful, working part today is a static origin cap, and the adaptive research is a prepared lab, not a proven feature.

**If the judge asks further:**

The decision to publish a negative result rather than tune until it passed is deliberate. The sealed run was executed once. Its limits are stated: it is a simulation, it covers one workload set, and the performance endpoint is quantised. It does not show adaptation can never help. The project's rules also require that intelligence never override the hard caps: the staleness cap, the origin cap, the byte budget and the fencing rules are enforced in the execution layer regardless of any decision.

**Evidence to know:** `src/intelligence/mod.rs::IntelligenceMode`, `src/intelligence/executor.rs::origin_cap_enforcing`, `src/main.rs` (`pie: None`), `docs/research/m6_sealed_results.md`, `docs/demo/scenario_b_interpretation_notice.md`, `CLAUDE.md` section 6b. Status: origin gate IMPLEMENTED, TESTED; decision engine PROTOTYPE ONLY; adaptive benefit NOT ESTABLISHED (H1 NOT_SUPPORTED, simulated).
:::

#### 1.11 Where to look for the truth <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Your tests pass. Does that mean the system is correct and ready?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No. The tests are evidence, not proof. The last full recorded run is 851 passing tests in a debug build on one Windows machine. That tells me the behaviours the authors thought to check hold in that setting. It does not tell me about release builds, Linux, several physical machines, or sustained load. Some tests assert the behaviour of known stubs. Every review so far is an AI second look, not a human review. The project's own status page says that nothing has been formally accepted by the owner, and that a set of review findings are still open. So I describe it as a research prototype with a tested core and an experimental cluster, and I list what is not yet shown.

**If the judge asks further:**

I can point to the specific open items: the reproduced loss of owed invalidations when the issuing node crashes, any member of the cluster being able to send tenant-wide purges, the unwired default TTL, and the absence of a release-profile suite. I can also say what I would test next: a release-profile run with repetitions on a stated host, and a multi-host run.

**Evidence to know:** `docs/STATUS.md` sections 1 to 3b; `docs/release/g4/cycle1_findings_disposition_register.md`; `docs/user/limitations.md`. Status: IMPLEMENTED, TESTED (debug, loopback); INDEPENDENTLY VERIFIED: none.
:::

### Chapter 2: System Architecture <!--gen-->

#### 2.4 Request flow 1: a cache hit <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Walk me through what happens to one request, from the network to the response.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The request arrives at the Axum router, which applies CORS and a body limit. The cache handler first checks the caller's token if authentication is on, and takes the tenant from the token. It builds a cache key from tenant, path and query and validates it, rejecting anything odd such as control characters or dot segments. It then asks the sharded cache. The key's hash picks one of eight shards and only that shard's lock is taken. If a fresh entry exists, the handler returns it with `X-Cache-Status: HIT`. If not, it goes down the miss path: one leader fetches the origin, others wait, the result is stored if it is still valid, and everyone gets the answer.

**If the judge asks further:**

"Where is the lock held across a network call?" Never: the shard mutex is held only while reading or updating the map. The origin call happens in a separate spawned task with no shard lock held. "What if the client has a token but wrong tenant?" Cross-tenant isolation is by key namespace taken from the token; see Chapter 6 for the analysis and the open findings.

**Evidence to know:** `get_cache_handler`, `PartitionedCache::begin_fetch_atomic`, `CacheKey::validate`; `m1_http_tests::test_m1_t01_cold_cache_miss` and `test_m1_t02_repeat_cache_hit` (TESTED on loopback, debug profile, one gateway; they prove the MISS then HIT behaviour and one origin call, not performance).
:::

#### 2.5 Request flow 2: a miss, coalescing and the origin fetch <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What happens on a cache miss? And what if a thousand clients ask for the same missing key?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

On a miss, the shard registers an in-flight operation for that key and the first request becomes the leader. It fetches the origin in its own task, with timeouts and size limits. Every other request for that key during the fetch joins the same operation and waits; the origin sees one request. When the answer arrives the gateway decides whether it may be cached, checks that the key was not deleted or invalidated during the fetch, stores it if all is well, and releases all waiters with the same body. There is a cap of 1,000 waiters per key; beyond that, extra requests are refused with a 503 so memory stays bounded. We tested the 100-client case on loopback.

**If the judge asks further:**

"Is that safe against a DELETE during the fetch?" Yes by design: the fetch records a generation number when it starts, and the store step compares it with the current one; DELETE increments it. "Across gateways?" No: coalescing is per gateway, so three gateways can fetch the same key three times at once. We do not claim cross-gateway coalescing.

**Evidence to know:** `PartitionedCache::begin_fetch_atomic`, `finalize_fetch`; `m1_http_tests::test_m1_t13_*`, `test_m1_t09_*`; `serving_contract_tests` (`test_ct1_single_flight_coalescing`, `test_ct3_waiter_limit_enforcement`, `test_ct7_fencing_under_invalidation`, cache-level); TESTED, loopback and debug build. Chapter 4 for the fencing proof and the open G4A-01 history.
:::

#### 2.6 Freshness, stale serving and origin failure <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What happens when the origin is unavailable?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

If the key is cached and fresh, nothing changes: clients are served from memory and never see the outage. If the entry has expired, the answer depends on what your origin told us when it sent the data. If it sent a stale-if-error or stale-while-revalidate window, we serve the expired copy, labelled STALE, for as long as that window allows, never beyond the 24-hour hard cap. If it sent nothing, we do not guess: the client gets an error, a 504 for a timeout or a 502 for a network failure. Errors are never cached, so recovery is immediate when the origin returns. Only one request per key goes to the failing origin at a time, because of coalescing, and an optional concurrency cap can shed load with a 503, but only in canary or active mode.

**If the judge asks further:**

"Is there a circuit breaker?" No. The protections are single-flight, a 5 second timeout, the optional origin concurrency cap, and the waiter limit. "Does it serve stale on every outage?" No, only with the origin's own directive. "Is stale-if-error tested end to end?" The rule is tested at cache level; I would not claim an HTTP-level outage test without checking.

**Evidence to know:** `get_cache_handler`, `check_stale_serving`, `map_origin_error_to_response`; `stale_serving_tests::test_stale_if_error_on_origin_outage` (TESTED at component level); `PHANTOM_MAX_STALE_MS` (`Wiring::Main`); status IMPLEMENTED.
:::

#### 2.7 DELETE, the cluster, and how nodes communicate <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How do the nodes communicate?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Nodes talk over a custom binary protocol on TCP wrapped in mutual TLS: each node proves its identity with a certificate whose name is checked against a static membership file, including the certificate fingerprint. The main message is an invalidation: when a key is deleted, the gateway sends it to the member chosen by rendezvous hashing, waits up to 150 milliseconds for an acknowledgement, and then tells the other members in the background, with a bounded queue, retries and a repair worker for members that were unreachable. There is also a heartbeat and a reconciliation exchange. Reads do not use the network between nodes: each gateway answers reads from its own memory.

**If the judge asks further:**

"What if a node is down?" It is marked suspect, it is owed the invalidation in an in-memory ledger, and a repair worker retries; if the issuing gateway restarts before delivery the owed list is lost, which is a known limitation. "Is the membership dynamic?" No, static, read at startup. "Why not use Raft or gossip?" There is no recorded decision; our reasoning: the system only needs to propagate invalidations, not agree on data.

**Evidence to know:** `src/dist/frame.rs` (message types), `src/dist/router.rs::invalidate_key_across_cluster`, `src/dist/node.rs::start_tcp_service`; `docs/user/limitations.md`; status IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED (no multi-host evidence).
:::

::: judge
**JUDGE MAY ASK**

Question: Which node serves a read for a key, and does rendezvous hashing decide that?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The gateway that received the request serves it from its own cache. Rendezvous hashing is used today only to pick the node that must acknowledge a DELETE first. The code for forwarding a read to the key's owner exists as a function but nothing calls it, and the peer-side read handler is a stub that fabricates a value, so I do not claim distributed reads.

**If the judge asks further:**

"Then what does the hash buy you?" A deterministic primary for invalidation, which gives a single acknowledging authority per key and a stable failover order. "Are the weights real?" No, the weight factor is a constant 1.0.

**Evidence to know:** `RendezvousRouter::route_key` (`capacity_weight = 1.0`), `forward_key_over_tcp` (no caller), `process_binary_cache_stream` `GetRequest` arm; `docs/research/adr/ADR-003-distributed-architecture.md` for the intended direction (status: agent-recorded, not owner-ratified per `docs/STATUS.md`).
:::

#### 2.8 Where Phantom Lab and the UI sit <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Is the dashboard showing real data? And how does the Lab relate to the real system?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The observation endpoints return counters and latency measured in the running gateway, and each field is meant to carry a provenance label. Some placeholder values, such as the three demo nodes and the startup snapshot, are synthetic; the gateway only emits them as live when simulation mode is switched on, and otherwise returns null or labels them. The Lab is separate from serving. Its T1 mode is a simulation that reuses the real cache and policy code with a virtual clock, so its numbers are simulated. Its T2 mode starts a real gateway and a scripted origin on one machine, so its numbers are measured, but on loopback only.

**If the judge asks further:**

"Can I trust the dashboard numbers as production performance?" No: loopback, one machine, and some views are simulated or inferred. Check the label before quoting a number.

**Evidence to know:** `build_schema_v1_payload` (provenance map), `PHANTOM_SIMULATION` (`config.rs`), `src/lab/mod.rs` header comment, `src/lab_live/mod.rs`; tests `telemetry_honesty_tests`, `schema_deprecation_tests` (TESTED, details in Chapters 6 and 7).
:::

#### 2.9 Configuration and deployment <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Is this deployable in production today? What exists for deployment?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is a research and engineering prototype with real hardening, not a production-ready product. What exists: a configuration system that fails closed, a production profile that refuses insecure settings, TLS and JWT options, health endpoints for liveness and readiness, release packaging scripts with checksums, a smoke test, and CI definitions. What does not exist: container images, orchestration files, service units, graceful connection draining, a built UI served by the gateway, durable invalidation across restarts, and any multi-host validation. The cluster mode is labelled experimental in the binary itself.

**If the judge asks further:**

"What would you need before production?" The list is in Chapter 8: durable invalidation, tenant quotas, multi-host validation, graceful shutdown, a release-profile benchmark on real hardware, and independent review. "Does the config file cover everything?" Almost: one key, the default TTL, is accepted but ignored.

**Evidence to know:** `packaging/`, `src/config.rs::KEYS`, `src/health.rs`, `main.rs` shutdown comment; `docs/user/limitations.md`; `docs/STATUS.md` (no milestone has a recorded formal acceptance).
:::

### Chapter 3: Technology Stack and Engineering Decisions <!--gen-->

#### 3.2 Rust <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why not Go or C#? Go has goroutines, a good HTTP server, and is the language of many caches.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Both would work, and both have strong HTTP stacks and concurrency support. I did not run a head-to-head, and the repository has no record of one, so I will not claim Go or C# were rejected on evidence. My reasoning for Rust is control: no garbage collector means no collector pauses in the tail latency, and ownership makes sharing visible. The cost is real: Rust is slower to write, and a Go service would have been faster to build. If tail latency from the collector turned out not to matter for this workload, Go would be a defensible choice.

**If the judge asks further:**

"Can you show Rust gave better latency than Go?" No. The repository contains no Go or C# implementation and no comparison. Our benchmarks have never been run in a release profile across languages. I would not claim a performance advantage over either. Also note we run on Windows loopback; any such comparison would need a release build, a fixed workload, a seed, and repetitions (CLAUDE.md section 4).

**Evidence to know:** absence of any Go/C# artefact (searched `docs/` and `src/`); `bench-results/README.md` (no release-profile result established); handoff 07 (language talk not in repository). Status: reasoning, not recorded; no performance claim.
:::

#### 3.3 Async programming and Tokio <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why Tokio, and what does it give you that threads would not?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Most of what the gateway does is wait on sockets and timers. Tokio lets a few worker threads serve thousands of waiting requests, and gives me the building blocks I need for the design: `oneshot` channels to hand one origin result to many coalesced waiters, bounded channels and semaphores to cap work, and timeouts around every network wait. I did not compare runtimes; the repository records the use of detached tasks in ADR-002 but not a runtime selection. The practical reason is that Axum and `tokio-rustls` are built on Tokio, so it is the natural fit.

**If the judge asks further:**

"What breaks if a task blocks?" It occupies a worker thread. The sweeper takes shard locks synchronously on a worker; the G4A-03 finding says one sweep tick scans a whole shard under its mutex, so a very large shard could hold a worker longer than the documented 1 ms bound. That is OPEN. The main protection today is that critical sections are short and contain no I/O.

**Evidence to know:** `src/main.rs::real_main`; `src/core/cache.rs::spawn_sweeper_task`; ADR-002 CT-2 and test `test_ct2_initiator_cancellation_detached_fetch`; G4A-03, G4A-13 in the disposition register. Status: IMPLEMENTED; sweeper lock-hold bound has a test (`test_sweeper_lock_hold_time_bounded`) but G4A-03 says the batch limit bounds candidates, not keys inspected.
:::

#### 3.4 Axum, hyper and tower <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why Axum rather than another framework, and why did you write your own HTTP client for the origin?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Axum is a thin framework on hyper and tower, on the same Tokio runtime as the rest of the system, so routing, state, body limits and CORS were a few lines each. I did not compare frameworks and the repository does not record a comparison. For the origin side I wrote a small HTTP/1.1 client because I needed to connect to an address I had already validated against SSRF rules, send a fixed minimal header set, cap the response size, and time out every step. That is a deliberate control decision, and the cost is that I own a response parser, so it has its own tests and its own review findings.

**If the judge asks further:**

"Is your parser as safe as hyper's?" I cannot claim that. Hyper is a widely used public library; I have not reviewed its test history, and ours has only unit and property tests (`parser_robustness_tests`) and AI review, not independent review. The Connection: close, no keep-alive design also means every miss pays a fresh TCP (and TLS) handshake; that cost has not been measured in a release build.

**Evidence to know:** `src/api/server.rs::create_app_with_lab`, `fetch_origin_raw`; `src/security/upstream.rs`; `tests/integration/parser_robustness_tests.rs`, `security_phase2_upstream_tests.rs`; `Cargo.lock` (`hyper` is transitive). Status: IMPLEMENTED, TESTED; decision not recorded.
:::

#### 3.5 Concurrency primitives and the concurrency model <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What are the concurrency implications of your design? Where could it deadlock or stall?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The rule is simple: short synchronous critical sections under `std::sync::Mutex`, and no lock held across an `await`. The slow work, the origin fetch, runs in a separate spawned task with no lock held, and the result is handed to waiters through channels. The lock order is shard first, then the epoch structures, and the invalidation path never takes a shard lock, so I see no lock cycle in what I read. The real risks I know of are these: a long sweep scan holds a shard lock on a worker thread (G4A-03), the epoch snapshot rebuild clones up to 10,000 entries while readers wait (G4A-12), and some locks still panic on poison (G4A-21). All three are open findings, not fixed.

**If the judge asks further:**

"Is it linearizable?" I do not claim that. I claim the single-flight and generation-fence rules, which have a protocol-model test and a stress test, and I tell you honestly that G4A-01 is a reported race (UNDER-INVESTIGATION) where an old body might survive an acknowledged invalidation. "How do you know there are no deadlocks?" I do not have a proof; I have a lock-order argument from reading the code and stress tests (`test_real_cache_concurrent_stress_schedule`) that did not hang, which shows absence of hangs in those schedules, not absence of deadlock.

**Evidence to know:** `src/core/cache.rs` (`get_effective_epochs`, `try_invalidate_tenant`), `src/intelligence/executor.rs::OriginGate` doc comment; `tests/integration/concurrency_verification_tests.rs`, `epoch_contention_tests.rs`; G4A-01, -03, -12, -17, -21, G4B-28 in the disposition register. Status: IMPLEMENTED; TESTED (stress and model); not INDEPENDENTLY VERIFIED.
:::

#### 3.7 The TLS and mTLS stack <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why rustls and not OpenSSL for the mTLS between nodes? And why do you have both `ring` and `aws-lc-rs`?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There is a recorded reason: the Phase 1 specification chose Rustls with `ring` and `rcgen` to avoid OpenSSL C-library build problems on Windows, where the project is developed. Rustls also lets me pin the cluster to TLS 1.3 only with a mandatory client certificate in a few lines, and a test checks that a TLS 1.2 client is refused. On the second part: the lockfile has two crypto libraries because I use `ring` directly for SHA-256, and rustls 0.23 defaults to `aws-lc-rs`. So the claim "no C crypto" in the spec is not literally true of the build, and I would recommend choosing one provider.

**If the judge asks further:**

"Is the node identity check sound?" It requires a certificate chaining to the cluster CA, exactly one DNS SAN of the expected form, and a fingerprint match against a registry. That is static membership; revocation is a broadcast flag (`revocation_tx`). It has been tested on loopback with generated certificates. It has not had independent security review, and there is no real multi-host deployment evidence.

**Evidence to know:** `src/security/cluster.rs` (`builder_with_protocol_versions`, `WebPkiClientVerifier`, `extract_sans_from_der`, `compute_cert_fingerprint`); `docs/security/phase-1-specification.md` section 8; `Cargo.lock` (`aws-lc-sys`, `ring`, `openssl`); `test_p3_sec04_strict_tls13_rejection_of_tls12_client`. Status: IMPLEMENTED, TESTED (loopback); not INDEPENDENTLY VERIFIED.
:::

#### 3.11 The frontend: React, Vite, plain CSS, hand-built SVG <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why React with plain CSS and no UI or chart libraries? Isn't that reinventing the wheel?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two reasons, and one of them is recorded. The recorded part: the toolkit proposal evaluated chart libraries, graph libraries and headless component libraries and rejected them on need: bundle size, dependency count and accessibility of canvas charts, with a rule for when to revisit, for example a chart that exceeds about two thousand SVG nodes. A guard test pins the dependency list so it cannot grow silently. The other part is my reasoning: the project's rule is that every number on screen is labelled measured, simulated, inferred or unavailable, and hand-built components let that label be part of every chart rather than something a library hides. The cost is real: more code, a hand-built SSE client, and one unused declared dependency.

**If the judge asks further:**

"Is the UI good?" I can say it is tested, not that it is approved. Tests, contrast and lint checks pass in the project's records; browser rounds found real defects (overflow at narrow widths, small tap targets) that were then fixed by the UI writer, and visual quality is still marked not approved by the owner. There has been no human usability study or screen-reader test. "Why React and not something lighter?" There is no recorded comparison.

**Evidence to know:** `ui/package.json`; `ui/test/forbidden.test.mjs`; `docs/design/remediation/07_frontend_redesign_implementation_record.md`; `docs/research/design_toolkit/toolkit_proposal.md` (rejected list); `docs/STATUS.md` M8 row. Status: IMPLEMENTED, TESTED; visual quality unapproved; React choice not recorded.
:::

#### 3.12 Testing and benchmarking tools <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: You have hundreds of tests. Do they prove your concurrent cache is correct?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No, and I would be wary of anyone who said yes. The suite is large, and it includes useful kinds of test: a protocol model checked exhaustively, a mutation detector that checks the model can fail, concurrent stress schedules over real threads, and property tests with random operation sequences. Those give real confidence in specific invariants. But they run in a debug build on one machine, stress tests sample interleavings rather than enumerating them, and a reported race, G4A-01 (UNDER-INVESTIGATION), has not been settled by the suite. I would say: tested for the invariants named, not proven, and not independently reviewed.

**If the judge asks further:**

"What would you add?" A deterministic concurrency checker such as loom for the shard protocol, a G4A-01 regression test that fails without the fix, and release-profile runs. None of those is in the repository today, and I would describe them as future work, not as done.

**Evidence to know:** `tests/integration/concurrency_verification_tests.rs` (`test_exhaustive_protocol_model_invariants`, `test_protocol_model_mutation_detector`, `test_real_cache_concurrent_stress_schedule`, proptest); G4A-01; `docs/research/claims_ledger.md`. Status: TESTED (named invariants); not INDEPENDENTLY VERIFIED. (`loom` is not a dependency; this is a suggestion, our reasoning.)
:::

### Chapter 4: Core Cache Engine and Algorithms <!--gen-->

#### 4.2 Sharded mutexes (partitioned buckets) <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why partitioned mutexes? Why not one lock, a lock-free map, or a concurrent hash map crate?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

One lock serializes every request. Splitting the table into 8 shards, each with its own lock, lets requests for different keys proceed in parallel, and a key always maps to the same shard by a hash and a bit mask, so there is no cross-shard coordination. The critical sections are short because the slow origin fetch happens outside the lock. I do not claim this is the fastest design. A lock-free or lock-striped concurrent map would scale further, and I have no benchmark in this repository that compares them. The reason for plain mutexes is that the cache needs several things to change together under one lock: the entry, the byte count, the in-flight record and the eviction list. That is simple to reason about with one mutex per shard and much harder lock-free.

**If the judge asks further:**

"Eight is small." Yes: at most 8-way parallelism for cache state, and the count is hard-coded in `main.rs`. One hot key lives in one shard, so sharding does not help it. The decision record for the number 8 was not found; I would say it is our reasoning, not a recorded decision.

**Evidence to know:** `src/core/cache.rs::PartitionedCache`, `get_shard_index`; `src/main.rs` (`PartitionedCache::new(8)`); IMPLEMENTED; contention not measured.
:::

::: judge
**JUDGE MAY ASK**

Question: What about lock contention? How do you know it is not a bottleneck?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

I do not know that it is not a bottleneck, because I have no contention measurement on a physical host to quote. What I can say from the code: lock hold times are short on the hit path (a map lookup, an epoch read, a small clone); the origin fetch is outside the lock; epoch checks use a read lock that does not block other readers. What I can also say is where contention would appear: one hot key, which is always one shard; and the sweeper, which scans a whole shard under its lock each tick, which a reviewer flagged as G4A-03 and is still open.

**If the judge asks further:**

"What would you measure?" Lock hold time and wait time per shard under a skewed workload on a release build, on a named host, with repetitions and confidence intervals. The repository's performance data is discussed in Chapter 6; none of it isolates shard lock contention (UNVERIFIED for that purpose).

**Evidence to know:** `docs/release/g4/cycle1_core_engine_review.md` G4A-03; `tests/integration/epoch_contention_tests.rs` (checks epoch reads and monotonicity, not shard mutex contention); status IMPLEMENTED, performance UNVERIFIED.
:::

#### 4.4 Request coalescing, single-flight and stampedes <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What stops a cache stampede? What happens to the origin when a hot key expires?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Single-flight per key. Under the shard lock the first miss registers an in-flight operation; later misses for the same key find it and wait on a channel. When the one origin fetch returns, its result goes to everyone. A test (`test_negative_effect_head_of_line_blocking`) fires 10 simultaneous requests at a slow origin and gets 1 miss and 9 coalesced answers. Waiters per key are capped at 1,000, and the origin read times out after 5 seconds. Two limits to state: if the origin fails, all waiters fail together, and there is no negative caching, so the next request tries again. And across many different keys there is no protection unless the optional origin concurrency cap is switched on, which it is not by default.

**If the judge asks further:**

"Does the expired entry cause a herd?" If the response carried stale-while-revalidate, expired entries inside that window are served immediately and refreshed in the background by one leader (bounded to 128 background refreshes by default). Without it, an expired entry is a miss and coalescing applies.

**Evidence to know:** `src/core/coalesce.rs`; `cache.rs::begin_fetch_atomic`; `coalesce_tradeoffs_tests.rs`; `origin_cap_tests.rs`; G4A-08 (OPEN); status IMPLEMENTED and TESTED on loopback.
:::

::: judge
**JUDGE MAY ASK**

Question: What happens with a hot-key overload? Say one key gets most of the traffic.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two different problems. If the hot key is a miss, coalescing turns the burst into one origin request. If the hot key is a hit, every request still takes the same shard mutex, twice in the GET path, and the HTTP layer copies the body for each response. The hot key lives in one shard, so the cache cannot spread it over cores, and it is not replicated to other nodes: GET is served from the local cache only. I would expect the shard mutex and the body copy to be the limit, but I have not measured it. What exists is detection and policy research in the intelligence modules (Chapter 5), which is not a serving-path fix for hit-path hot keys.

**If the judge asks further:**

"How would you fix it?" Options I would test, none implemented: store the body once as `Arc` and stream it without cloning; take the hit path with a read lock; replicate hot entries per core. Those are proposals, not design decisions in the repository.

**Evidence to know:** `src/api/server.rs` (HIT branch clones the body); G4A-26; `cache.rs::check_stale_serving` and `begin_fetch_atomic`; status IMPLEMENTED, hot-hit behaviour UNVERIFIED.
:::

#### 4.6 Memory management and byte accounting <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How do you know the cache does not leak memory? Is the byte count the real memory use?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The byte count is a logical count, not the process memory. It adds up key size, body capacity, header strings and a fixed per-entry overhead, and the code can recompute it from the maps and compare. The tests drive every path that changes an entry and assert the recomputed and running totals match, including a concurrent stress run. That shows the bookkeeping is consistent. It does not show the operating system sees the same number, because some structures are not counted: freshness strings, hash-table slack, and the S3-FIFO queues. And there is one known path where the totals drift, a panic in the middle of finalizing, which is open as G4A-13.

**If the judge asks further:**

"What bounds the real memory?" The byte budget, the entry limit, the waiter limit and the 2 MiB origin response body limit bound the counted part. A soak test (`memory_bound_soak_tests.rs`) runs churn with accounting checks. I have no measured resident-memory figure for a release build to quote.

**Evidence to know:** `cache.rs::audit_bytes`, `calculate_state_bytes`; `byte_accounting_tests.rs`; G4A-13, G4A-23; status IMPLEMENTED, TESTED (consistency only).
:::

#### 4.7 Invalidation and epochs <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Explain epochs. Why not just delete the keys? What does invalidation guarantee?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Deleting every key of a tenant means walking the table under locks, which is slow and blocks requests. Instead each tenant has a version number. Every entry is stamped with the version when stored. To invalidate, we increase the number. Any entry with an older stamp is treated as missing the next time anyone touches it, and a background sweeper reclaims the rest. A fetch that started before the bump also cannot store its answer, because its stamp no longer matches. The guarantee is local to one node and it is lazy: after the call returns, reads on that node do not serve the old entries, with two documented exceptions and one open race. The bump itself is not free either: it copies the tenant-version map, up to 10,000 entries.

**If the judge asks further:**

"What if the tenant list overflows?" Past 10,000 tenants the node raises a global epoch instead, which invalidates everything on that node. That is safe but heavy, and a peer can trigger it by sending many tenant names (G4A-12, open).

"Is it atomic across nodes?" No. That is Chapter 5; here the guarantee is single process.

**Evidence to know:** `cache.rs::try_invalidate_tenant`, `get_effective_epochs`; `epoch_contention_tests.rs`, `invalidation_contract_tests.rs`; G4A-08, G4A-12, G4A-17; status IMPLEMENTED, TESTED for the basic and overflow paths.
:::

#### 4.8 Freshness, TTL, stale serving and revalidation <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How does expiry work? What stops the cache from serving very old data? Is the default TTL configurable?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The origin's `Cache-Control` decides the lifetime: shared-cache max-age first, then max-age, then `Expires`; with no information we use 60 seconds. Be careful with the last point: there is a setting called `PHANTOM_DEFAULT_TTL_MS`, but it is not wired. The code passes a fixed 60,000 to the freshness function, so changing the setting does nothing today. For stale data, the cache serves an expired entry only if the origin allowed it with stale-while-revalidate or stale-if-error, never if must-revalidate is set, and never beyond a hard cap of 24 hours by default, which is `PHANTOM_MAX_STALE_MS` and is wired. The cap limits stale serving, not fresh lifetime. Known gaps: the upstream Age header is ignored, time comes from the wall clock, and stale-if-error is applied to more status codes than the RFC allows.

**If the judge asks further:**

"Is it RFC 9111 compliant?" It implements a subset, with the gaps above. I would say "RFC 9111-style", not "compliant". No conformance suite was run (UNVERIFIED).

**Evidence to know:** `src/config.rs` (`PHANTOM_DEFAULT_TTL_MS`, `Wiring::NotWired`); `src/api/server.rs` literal `60_000`; `cache.rs::check_stale_serving`; `freshness.rs::evaluate_response_freshness`; G4A-04, -05, -06, -10, -18, -25; status IMPLEMENTED (subset), TESTED for cap and SIE paths.
:::

#### 4.9 Eviction <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: State the time and space complexity of the cache operations.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Let n be the entries in a shard and k the key length, at most 4,096 bytes. Lookup: hash the key, O(k), then expected O(1) in the hash map. Insert and delete: expected O(1) plus the eviction policy bookkeeping, also O(1). Eviction: amortized O(1) with no pinned keys. Coalescing a request: O(1). Memory is O(entries x entry size), bounded by the byte budget and the entry limit. Three honest exceptions: tenant invalidation is O(T) in the number of tenants with a recorded epoch, up to 10,000; the sweeper scans O(n) of a shard under its lock every tick; and switching policy is O(n) per shard. Also a hit copies the body, so it costs O(body size) as well.

**If the judge asks further:**

"Space beyond the cache?" Per-key state with a set of active ops, a waiter list per in-flight key (up to 1,000), the eviction structures, and for S3-FIFO possibly unbounded queues.

**Evidence to know:** `cache.rs` (`get_shard_index`, `try_invalidate_tenant`, `sweep_lazy_reclamation`, `set_eviction_policy`); G4A-03, G4A-12, G4A-20, G4A-26; status: asymptotics from reading code, none measured here.
:::

#### 4.10 What happens when the origin fails <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: The origin goes down. What does the user see, and what protects the origin when it comes back?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It depends on what is cached. A fresh entry is still served; the origin is not involved. An expired entry that the origin marked with stale-if-error is served as stale up to the 24-hour hard cap, unless it is marked must-revalidate. A key with nothing usable fails for every request that was waiting on that one fetch, with the same error, and the next request tries again, since errors are not cached. When the origin returns, a returning stampede is limited by single-flight to one request per key. Across many keys there is no limit by default; there is an optional concurrency cap for the origin that only enforces in canary or active intelligence mode. Two caveats: stale-if-error is applied to more status codes than the standard allows (G4A-06), and there is no negative caching or circuit breaker.

**If the judge asks further:**

"Does the AIMD cap help?" Its effect is not established in this chapter: it starts at its maximum and reduces only if observed p99 latency or errors cross a threshold. The evidence discussion is in Chapters 5 and 7.

**Evidence to know:** `stale_serving_tests.rs`, `coalesce_tradeoffs_tests.rs`, `origin_cap_tests.rs`; G4A-06; status IMPLEMENTED, TESTED on loopback with in-process mock origins.
:::

### Chapter 5: Distributed Systems and Workload Intelligence <!--gen-->

#### 5.2 Cluster membership and node-to-node communication <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How do your nodes discover each other and communicate? What if I add a node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Membership is static. A JSON file lists every node, its id and the fingerprint of its certificate. Each gateway reads it at startup. Nodes talk over mutually authenticated TLS 1.3 with a small binary protocol; both sides check the certificate against the cluster CA and against the pinned fingerprint. There is no gossip and no dynamic join. To add a node you edit the file, bump its version and restart the cluster members. A runtime reload and revocation path exists in the library and is tested, but nothing in the shipped gateway triggers it. That is an open finding, G4B-07, and I would not claim dynamic membership.

**If the judge asks further:**

"Why not gossip or a consensus library?" Our reasoning, not a recorded decision: the cluster only needs to deliver invalidations to a small, known set, and static membership with pinned certificates is the simplest thing that gives strong identity. It limits operations (no rolling add without restart) and that is a stated limitation.

**Evidence to know:** `src/security/cluster.rs::ClusterMembershipRegistry::update` (no production caller), `src/main.rs` cluster branch, G4B-07 in `docs/release/g4/cycle1_findings_disposition_register.md`. Status: static membership IMPLEMENTED and TESTED; dynamic membership DESIGNED BUT NOT IMPLEMENTED.
:::

::: judge
**JUDGE MAY ASK**

Question: You have heartbeats. So does the cluster detect a dead node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Not by heartbeat. The heartbeat keeps idle connections alive every five seconds and discards dead ones, but it does not change any node's state. A node becomes Suspect when a real call to it fails, usually the first invalidation after it died, which then spends its 150 millisecond deadline. After that the node is skipped for routing and its owed invalidations go to a repair ledger. So detection is lazy and driven by traffic. If I had more time, wiring heartbeat results into the state machine is the obvious fix; it is recorded as finding G4B-12.

**If the judge asks further:**

"Does the UI show a failed node?" The UI can show a node as Failed after the demo control endpoint sets a flag. That is a logical flag on a router entry, not a detected failure, and the dashboard labels node state as inferred, not measured.

**Evidence to know:** `src/dist/pool.rs::probe_idle_connections` (no `NodeState` access), `src/api/server.rs::control_handler`, G4B-12 and G4B-03. Status: failure detection by heartbeat is not implemented.
:::

#### 5.3 Rendezvous hashing <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: You say you use weighted rendezvous hashing. Walk me through it. How does the weight enter, and what does the system do with it?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Each node gets a score for each key from a hash of the key and the node id, and the highest score owns the key. Everyone computes the same scores, so no coordination is needed, and removing a node moves only that node's keys. In weighted form you use minus weight over the natural log of the normalised hash, which makes a node win in proportion to its weight. In our code, honestly, the weight is a constant 1.0, so it is plain rendezvous hashing. I would not claim capacity-aware placement. And the result is only used to choose which node gives the synchronous acknowledgement for a delete. Reads are served from the local cache and are not routed.

**If the judge asks further:**

"Why does that matter?" Because it means no data is placed or rebalanced by the hash. "What would break if you set weights?" The code multiplies the raw hash by the weight, which is not proportional; you would need the logarithmic form. I checked that with a quick calculation: weights 1 and 2 give a 75 percent share to the heavier node, not 67 percent.

**Evidence to know:** `src/dist/router.rs::route_key` (`capacity_weight = 1.0`), `get_replicas`, callers of `route_key` (only `invalidate_key_across_cluster` in production). Replica measurement in section 5.3 (reproduced here, standalone). Status: equal-weight rendezvous IMPLEMENTED; weighted rendezvous DESIGNED BUT NOT IMPLEMENTED; no uniformity test in the repo.
:::

::: judge
**JUDGE MAY ASK**

Question: Walk me through a read. Which node serves it, and where does the hash come in?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The gateway the client contacted serves it. It looks in its own cache; on a miss it fetches from the origin, with single-flight so concurrent misses share one fetch, and stores the result with whatever freshness the origin allowed. No other gateway is consulted, so the hash is not involved. The hash matters for deletes: it picks the node that must acknowledge before the delete returns.

**If the judge asks further:**

"So the same key can be cached on every node?" Yes. That is the design: local copies, with invalidation fan-out to keep them coherent. The cost is that a node that missed an invalidation can serve a stale copy, which is the subject of the next sections.

**Evidence to know:** `src/api/server.rs::get_cache_handler`, `RendezvousRouter::forward_key_over_tcp` (no production caller), ADR-003. Status: local-only GET IMPLEMENTED and TESTED (`known_gaps.rs`, `distributed_consistency_tests.rs`).
:::

#### 5.4 Epochs and versioning across nodes <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: You mention epochs. What is an epoch here, and does it give you a global version of the cache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There is no global version. Inside one gateway, a tenant epoch and a global epoch are counters; each cached entry remembers the counters it was filled under, and bumping a counter makes all older entries instantly stale without scanning. Between gateways we do not synchronise those counters. What crosses the wire for the normal delete is the exact key, plus a per-destination sequence number so the receiver can notice a lost message, and the cluster membership version. The membership version is carried but not enforced on invalidations yet, which is an open finding.

**If the judge asks further:**

"Why can a tenant counter not simply be shared?" Because gateways have independent caches and independent fill times; a shared counter would need agreement, which we do not have. The sequence number plus replay is the lightweight alternative.

**Evidence to know:** `src/core/cache.rs::try_invalidate_tenant`, `try_invalidate_all`, `get_effective_epochs`; `src/dist/node.rs` InvalidateRequest handler; G4B-06, G4A-01. Status: local epochs IMPLEMENTED and TESTED; cross-node epoch enforcement not implemented.
:::

#### 5.5 Invalidation propagation <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What if the invalidation message is lost or arrives late?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There are three layers. First, the sender retries up to four times with short backoff. Second, anything still undelivered is written to a per-peer repair ledger and re-sent every 250 milliseconds with the same sequence number until acknowledged. Third, a receiver that notices a gap in the sender's sequence numbers can pull the missed records from the sender's ring buffer; if the history has been overwritten it flushes its whole cache, which is safe but coarse. All of it is bounded in memory. A delayed message is harmless because a delete is idempotent: applying it twice, or after a newer fill, only causes an extra miss. The one case none of this covers is the sender dying while it still owes messages.

**If the judge asks further:**

"How long can a node stay stale?" Until the message is delivered; in the one test that cut a link for two seconds and healed it, convergence was asserted within 800 ms of healing on loopback in a debug build. With a permanent cut or a crashed sender there is no bound other than the entry's own expiry.

**Evidence to know:** `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`, `RepairLedger`, `repair_peer`, `reconcile_peer_anti_entropy`. Status: IMPLEMENTED and TESTED (loopback, debug); sender crash UNHANDLED.
:::

#### 5.6 Consistency: what is guaranteed, under which failure model <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Can two of your nodes serve different values for the same key at the same time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes. By design each gateway has its own copy. They differ for a few milliseconds after a delete while the invalidation is in flight, for as long as a partition lasts if a gateway is cut off but still serving, and indefinitely until expiry if the gateway that issued the delete crashed before telling everyone. They can also differ legitimately with no delete at all, because two gateways fetched at different times and the origin changed in between; each is within its freshness lifetime. We never claim they are the same, we claim a mechanism that makes them converge after a delete when the system is healthy, and we say plainly what is not covered.

**If the judge asks further:**

"What would make it stronger?" Durable owed-invalidations for the issuer, which exists as a tested sandbox design; a gateway-side ceiling on freshness lifetime; and a heartbeat that marks silent peers Suspect. A truly linearizable read would need coordination or a leader, which is a different system.

**Evidence to know:** ADR-003 section 4 and its 2026-10-30 amendment; `docs/research/claims_ledger.md` C6 amendment and qualifier; `tests/integration/distributed_consistency_tests.rs`. Status: convergence IMPLEMENTED and TESTED on loopback; staleness bound UNVERIFIED and not claimed.
:::

#### 5.7 Node failure, recovery, partitions, and concurrency across nodes <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What happens if a node fails? What if the network partitions?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

If a node dies, calls to it fail fast, it is marked suspect and skipped, and the invalidations it missed are queued in a bounded ledger on the sender. When it comes back, its cache is already empty because it is in memory, the ledger is delivered, and it rejoins after a pull reconciliation. If the network splits but both sides are alive, there is no leader, so nothing stops the cut-off side from serving its cached copies; they can be stale until the link heals and the repair loop catches up, or until the entries expire. I would not claim availability and consistency both; we chose availability of reads, and the staleness during a partition is bounded only by the entry's freshness lifetime.

**If the judge asks further:**

"Does a suspect node stop serving?" Not on the HTTP path. The rule exists in the peer protocol handler only. That is a gap I would name before they find it.

**Evidence to know:** `test_m3_04a_suspect_to_active_automatic_recovery`, `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`, `server.rs` (no `NodeState` in the GET path), `test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits` (peer path only). Status: receiver recovery IMPLEMENTED and TESTED (loopback); partition serving is stale by design; no staleness bound.
:::

#### 5.8 The peer GetRequest stub, DR-029 and DR-027 <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Your node protocol has a GET request. Can a peer poison your cache with it?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The peer GET is a placeholder, and I will not pretend otherwise: on a miss it invents a value called `val_node_` plus the node id and stores it. No production code sends that request, and it requires a valid cluster certificate pinned in the membership list. The planted key always lives in a key space that HTTP requests cannot reach, because peer keys must not start with a slash and HTTP paths always do. So I can show it does not poison what clients read today. It can still fill the cache with junk and evict real entries, and it is a hazard for any future peer-fill feature. We track it as DR-029, with a test that pins the behaviour, and the fix is waiting on an owner decision.

**If the judge asks further:**

"Why not delete it?" The register records the fix as gated on an owner decision and an independent review (G4B-30). Our reasoning: removing the handler and implementing real peer reads are different directions, and a test pins the current behaviour, so deleting it is a decision rather than a cleanup.

**Evidence to know:** `src/dist/node.rs` GetRequest arm, `tests/integration/known_gaps.rs::pin_kg6_node_get_request_fabricates_missing_key_body`, `m7_sec_round3_tests.rs` (leading slash rejection), DR-029 investigation and review notes, G4B-30. Status: PROTOTYPE ONLY; design defect OPEN.
:::

#### 5.11 Telemetry: the histogram, with the maths <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How accurate are your latency percentiles, and how do you compute them without slowing the request path?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We built a log-linear histogram: 2,688 counters, constant-time recording, relative error at most 0.39 percent by construction and checked by tests. That is the recording design. In the current gateway the percentiles on the metrics endpoint are actually computed from a 10,000-sample window that is sorted on demand, so they are exact for that window but not the histogram, and the sort and two locks are on the observation path. Switching the endpoint to read the histogram is a small change that has not been made. I would not quote the 0.39 percent as the accuracy of the displayed numbers.

**If the judge asks further:**

"Which is right?" The sorted window is exact for the last 10,000 GETs; the histogram would cover all history at 0.39 percent. They answer different questions, and the repository's percentile conventions are also inconsistent (floor versus nearest-rank) which Chapter 6 discusses.

**Evidence to know:** `src/intel/telemetry.rs` (`value_to_bucket`, `calculate_percentiles_us`, `record_us`), `tests/integration/telemetry_redesign_tests.rs`, `src/api/server.rs` (metrics uses `calculate_percentiles_us`). Status: histogram IMPLEMENTED and TESTED; histogram as the live percentile source NOT the case.
:::

#### 5.12 Probabilistic structures, hot-key detection and workload patterns <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How do you detect and handle hot keys?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two separate things. Handling: concurrent misses for one key are coalesced into a single origin request, so a hot key cannot multiply origin load, and once cached it is served locally by every gateway. That is live and tested. Detection: we have a library detector that combines a Count-Min Sketch with a Space-Saving table of 64 keys and a rule for sudden emergence, tested in isolation, but it is not wired into the gateway, and the metrics endpoint reports hot keys as unavailable. So I would claim hot-key protection through coalescing, and hot-key detection as a built-but-unused component.

**If the judge asks further:**

"What error does the sketch have?" Overestimate only, at most about 0.27 percent of the events in a window with about 98 percent probability for our dimensions, from the standard Count-Min bound. We have tests of its behaviour, not a measured error on production traffic, because there is none.

**Evidence to know:** `src/intelligence/signals.rs::HeavyHitterDetector`, `HotKeyEmergenceDetector`; `src/core/coalesce.rs` (Chapter 4); `server.rs` `"hot_keys"` unavailable; `src/intel/sketch.rs` unused. Status: coalescing IMPLEMENTED and TESTED; detection PROTOTYPE ONLY / library, not wired.
:::

### Chapter 6: Security, Reliability and Performance <!--gen-->

#### 6.2 Authentication and authorisation <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How do you stop an unauthorised user reading or deleting cached data?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

When a JWT secret is configured, every cache read, delete, metrics, stream and control call needs a signed token. The gateway checks the signature, issuer, audience and expiry, then checks that the token's role allows the action: `cache:read` to read, `cache:invalidate` to delete. The tenant is taken from the token, never from a client header. The check runs before any cache lookup or origin fetch. If no secret is configured the gateway only starts on a loopback address, and it refuses to bind to a network address without a secret and TLS. What I would not claim: tokens cannot be revoked before they expire, there is no not-before check, and the token issuer is outside this repository.

**If the judge asks further:**

Ask about ordering. In the GET and DELETE handlers the body is extracted before the handler authenticates (G4B-01, High, owner-named, OPEN; see 6.5). The Lab API does it in the right order with a 5-second body timeout.

**Evidence to know:** `src/security/jwt.rs::JwtValidator::validate`, `src/api/server.rs::authenticate_request`, `require_tenant_claim`; tests `m7_sec_fixes_tests::sec01_*`, `m7_security_review_tests::m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering`, `m7_http_rbac_matrix_in_secure_mode`; G4B-01, G4B-13, G4B-14. Status: IMPLEMENTED, TESTED for the covered paths; revocation, `nbf`, lifetime caps absent.
:::

#### 6.3 Management and API access <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Your control endpoint changes cluster node state. Who can call it, and what stops a demo control from breaking production?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In secure mode it needs the `scenario:operate` role. Without a JWT secret it is anonymous, which is only possible on a loopback address. But I will be honest: it is a demo control, and today it is not isolated from the real router. In a cluster process it operates on the same node objects the router uses, and an unknown event sets every node Active. The register has it as G4B-03, an owner-named open finding. The fix options are to refuse it when a cluster router exists or gate it behind the simulation flag. I would not claim it is safe to expose in production.

**If the judge asks further:**

Ask whether this changes cache correctness. Marking a node Failed changes which node is the routing primary for DELETE and makes the node drop incoming connections at accept time; it does not corrupt cache data. It can make invalidations go to a different node, which the repair machinery then has to reconcile.

**Evidence to know:** `src/api/server.rs::control_handler`; `src/main.rs` (nodes shared with the router); `src/dist/node.rs` accept loop drops sockets in Failed state; G4B-03 (OPEN, owner-named, ODR-4). Status: IMPLEMENTED, not isolated; no cluster test.
:::

#### 6.5 Input validation, body limits, timeouts and connection management <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What happens if someone sends a malicious request: a huge body, a slow body, a crafted path?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Crafted paths and keys are handled well: the key is validated against a strict alphabet and length limits, control characters and dot-segments are rejected with 400, and the path is re-encoded before it goes upstream, so header injection and path traversal into another tenant's key space are blocked. There are tests for those. A huge body is capped at 512 KiB and gets 413. A slow body is the weakness: there is no read timeout or connection cap on the main API, and the body is read before authentication. That is finding G4B-01, the one High-severity security finding, still open and owner-named. The fix is known and already done in our Lab API; it has not been applied to the main API, and I would not call the main API hardened against slow-client exhaustion.

**If the judge asks further:**

Ask what you would do first. Move authentication ahead of body extraction by taking the raw request in the handler, add a body-read timeout and a connection limit, and add the two regression tests the reviewer proposed (stalled sockets must not block a legitimate request; an unauthenticated `POST /api/control` with invalid JSON must return 401, not 400). Say the fix needs the owner's decision because `server.rs` changes are application-code changes.

**Evidence to know:** `src/api/server.rs::create_app_with_lab` (`DefaultBodyLimit`), `get_cache_handler` signature, `src/api/lab_api.rs::runs_start`; tests `m1_http_tests::test_m1_t26_ingress_request_body_limit`, `m7_sec_round2_tests::h1_illegal_bytes_in_path_are_400_and_origin_receives_nothing`; G4B-01 (OPEN, owner-named). Status: key validation TESTED; slow-body protection absent.
:::

#### 6.6 Tenant isolation and cache poisoning <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: How is tenant isolation enforced, and could one tenant ever see another's data?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The tenant is part of every cache key, and in secure mode it comes only from the signed token, never from a header. Tenant names are restricted so one cannot spoof another's prefix, and an invalidation bumps a per-tenant epoch so it only affects that tenant. Tests cover tenant-claim handling, spoofed headers, and a tenant reading another's entry. The limits: isolation is logical, inside one process and one shared byte budget; there are no per-tenant quotas, so a heavy tenant can evict others. A cluster member can address any tenant in an invalidation. And the literal tenant name `__default__` is refused in secure mode but the key validator still accepts it, which is why G4A-19 is not closed.

**If the judge asks further:**

Ask about cache poisoning. Explain the header whitelist: the only inputs that influence what the origin returns are the path, the query and the tenant, all part of the key, so a client cannot choose a variant that other users get. The contract is that origins must not vary responses on other client data.

**Evidence to know:** `src/core/cache.rs::CacheKey::new`, `validate`; `src/api/server.rs::require_tenant_claim`; tests named above; G4A-19, G4B-05, G4B-11, G4B-30 / DR-029. Status: TESTED on HTTP paths; quotas DESIGNED BUT NOT IMPLEMENTED.
:::

#### 6.8 Resource exhaustion, origin overload and SSRF <!--gen-->

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

#### 6.9 Failure recovery, lost invalidations and races <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: What happens if an invalidation is lost or delayed? Can two nodes serve different values?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes, two nodes can briefly serve different values, and I can say exactly when. After a DELETE, the issuing gateway misses immediately. The primary has acknowledged before we return 204. Other nodes are told in the background, with retries and a repair record for any peer that cannot be reached, and a pull mechanism that detects gaps. On a warm loopback cluster we measured propagation mostly in milliseconds, with a first-use cold start of tens of milliseconds, in debug builds. During that window another node can serve the old value. If a peer is down, it can stay stale until repaired. The case we do not handle: if the issuing gateway crashes before the peer is repaired, the record of what was owed is in memory only and is lost, so that peer stays stale until the entry's own expiry. A durable log is designed and prototyped but not integrated. So I claim eventual repair while the issuer stays alive, not a bounded staleness guarantee.

**If the judge asks further:**

Ask what the bound is. State the failure model (issuer alive, loopback), say the measured window is milliseconds on warm connections, and say there is no general bound if the issuer dies. The test that would falsify the issuer-alive claim is a multi-process test that blackholes a peer, sends DELETE, then kills the issuer; it does not exist.

**Evidence to know:** `src/dist/router.rs::invalidate_key_across_cluster`, `RepairLedger`; `src/main.rs` shutdown; tests `distributed_consistency_tests`, `cluster_followup_tests`, `cluster_propagation_latency_tests`; claims ledger C6 amendment and qualifier; G4B-09 (OPEN); M3 design (PROTOTYPE ONLY). Status: PARTIAL.
:::

#### 6.10 Memory safety and supply chain <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Is the system memory-safe, and how do you manage third-party risk?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The code is Rust, and our own cache, security and cluster code contains no `unsafe`. The only unsafe code in the product is a small Windows-only block of operating-system calls in the Lab, used to kill child processes cleanly; each block is commented. Third-party crates have their own unsafe code that we have not audited. For dependencies, we pin versions in `Cargo.lock` (288 packages), keep TLS in rustls rather than OpenSSL for the product path, and I can name the open gaps: nobody has run a vulnerability scan such as `cargo audit` yet, some features such as websockets and file serving are enabled but unused, and there are two crypto providers. The scan is a release gate that needs a human decision.

**If the judge asks further:**

Ask about panics. Rust stops memory corruption, not logic failure. We have findings about lock poisoning handling and no catch-panic layer, tracked as G4A-21 and G4B-28.

**Evidence to know:** `grep unsafe` result (lab_api.rs Windows module; benches alloc counter), `Cargo.toml`, `Cargo.lock`, `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (G4C-DEP). Status: IMPLEMENTED; scan UNVERIFIED.
:::

#### 6.12 Evaluating performance correctly <!--gen-->

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

#### 6.14 Reliability, health and bottlenecks <!--gen-->

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

### Chapter 7: Phantom Lab, Experimentation and Innovation <!--gen-->

#### 7.9 Judge boxes <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: Why not just use Redis or Memcached?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

For a general key-value cache, you should. They are mature, widely supported and have more features. PhantomCache is a different shape: an HTTP gateway that sits in front of an origin, applies HTTP freshness rules, serves stale content under a hard cap, coalesces concurrent misses and caps origin concurrency. With Redis you would write that logic in the application. Memcached's leases and Redis client tracking solve parts of the stale-write and invalidation problem; our generation fencing is the same family of idea. The honest trade-off is that we offer less maturity, no persistence and no clustering depth, and no benchmark shows we are faster.

**If the judge asks further:**

"Could you put Redis behind your gateway?" That is a plausible design, but it is not what exists. The cache is in-process memory per node.

**Evidence to know:** `docs/research/rt0_prior_art_sweep_2026-10-30.md` T3 (Facebook leases, OPENED-DIRECT) and T10 (Redis tracking); `src/core/cache.rs` (in-process cache); Chapters 2 and 4.
:::

::: judge
**JUDGE MAY ASK**

Question: What is the actual intelligence in the system?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is adaptive control, not machine learning. The part that touches live traffic is an origin concurrency cap with an optional AIMD controller: if the measured origin P99 exceeds a ceiling or errors exceed 5%, cut the allowed concurrency by 30%; otherwise add one. Around it the library has a dynamic stale-window rule, a frequency-based admission filter, and a shadow and canary framework, but the serving path does not call the shadow and canary parts today. The legacy PIE controller is a threshold stub that is not even instantiated. And the evidence says the adaptive version did not beat a static cap.

**If the judge asks further:**

"Why keep it?" It is available behind explicit configuration, labelled experimental, and the result is published as a negative finding. The product decision, recorded by the research lead, is to ship the static cap path as the MVP.

**Evidence to know:** `src/intelligence/candidates.rs::OriginConcurrencyCapController::step`, `docs/release/g4/cycle1_lab_intelligence_deps_review.md` section 5, `docs/research/m6_sealed_results.md`.
:::

::: judge
**JUDGE MAY ASK**

Question: What is your strongest technical contribution?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two things, and I would rank the method above the mechanisms. First, the evaluation discipline: a deterministic simulator that runs the real cache and executor, a preregistered rule applied by code, a hash-committed sealed set, and a result we reported even though it was negative. Second, the cache core's explicit invariants: generation fencing, epochs, a hard staleness cap and byte accounting, each with tests. I would not call either a breakthrough. They are careful engineering and honest reporting.

**If the judge asks further:**

"Show me the strongest evidence." The M6 sealed run: 12,000 cells, 30 sealed seeds, verdict H1 NOT_SUPPORTED, produced by `m6_pipeline::compute_verdict`. It has not been independently reviewed, and I say so.

**Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`, `src/lab/m6_pipeline.rs::compute_verdict`, `docs/release/known_limitations_frozen_code.md` section 1.
:::

::: judge
**JUDGE MAY ASK**

Question: Where are established products stronger than PhantomCache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Many places. Production maturity and tooling in Redis, Memcached, Varnish and nginx. Cross-node duplicate suppression and hot-item mirroring in groupcache, where our GET is local-only. Invalidation durability: our cross-node invalidations are best-effort after the first ack, and owed invalidations are lost if the issuing node dies. Adaptive concurrency in Envoy and Netflix use relative signals, ours uses an absolute ceiling and did not beat a static cap. Eviction research with thousands of traces. Per-tenant resource quotas are absent. And we have no head-to-head performance numbers.

**If the judge asks further:**

"What would you fix first?" The invalidation durability gap, then a load-dependent test origin, then a relative-signal controller under a new preregistration.

**Evidence to know:** claims ledger C6 qualifier; `src/dist/router.rs` (constant weight); `docs/research/rt0_prior_art_sweep_2026-10-30.md` T7, T10; Chapter 8 section on limitations.
:::

::: judge
**JUDGE MAY ASK**

Question: How would I reproduce one of your experiments?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Every run folder has a manifest with the source file hashes, git head, build profile, seeds, scenarios and arm configurations. Check out or confirm that source, build in release, and run the same `RunSpec` with the `phantom-lab` command; the cells file should be byte-identical on the same machine. For the sealed M6 run, nobody can regenerate the seeds without the secret salt, but anyone can re-run the analysis on the retained cells and check the verdict. The T2 Scenario B experiment is real time, so a re-run gives a fresh sample that should agree within the intervals, not byte for byte.

**If the judge asks further:**

"Would it be identical on my laptop?" I have not tested cross-platform determinism. The tests ran in one environment.

**Evidence to know:** `src/lab/eval.rs::run_matrix_with_progress` (manifest fields), `docs/release/reproduction_guide.md` (exists; not read for this chapter), `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes`.
:::

::: judge
**JUDGE MAY ASK**

Question: What is the difference between your simulation and a real test?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In T1 the clock is virtual, the origin and the clients are models, and the real cache and policy code run in-process as a library. Nothing touches a network or the HTTP stack. Results are exactly repeatable. In T2 the real gateway binary runs as a child process, the load generator uses real sockets and real time, and the origin is a small scripted server. It is real but only on loopback on one machine with shared CPU, so no network latency, and the origin has no queue. Neither is a production or multi-host measurement. The interface labels them differently: Simulated versus Real with the qualifier "loopback, one host".

**If the judge asks further:**

"Why is T2 not enough on its own?" Real-time runs are not repeatable, cost minutes each, and cover one scenario. T1 covered 12,000 cells in about 97 seconds of test time.

**Evidence to know:** `src/lab/t1_sim.rs` header, `src/lab_live/mod.rs`, `ui/src/lib/labProvenance.js`, `docs/demo/scenario_b_interpretation_notice.md`.
:::

::: judge
**JUDGE MAY ASK**

Question: You report that your adaptive controller failed. Why should I trust anything else?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Reporting the failure is the reason to trust the process. The rule that produced it was written before the sealed seeds were used, and the code applied it mechanically. I also list the weaknesses: the amendment that forced the controller to be adaptive was chosen after seeing development results, the stale-window mechanism was never exercised at that horizon, the primary P99 includes quickly refused requests, and nobody independent has reviewed the pipeline. What the failure does not mean is that adaptive control is useless. It means this controller, with an absolute ceiling, shows no benefit in this simulation.

**If the judge asks further:**

"What would make adaptive control win?" A relative-signal controller, a load-dependent origin and workloads where a static setting cannot fit all phases. That is a new hypothesis needing a new protocol.

**Evidence to know:** `preregistration_m6_v2_2_amendment.md` sections 3, 5, 7b; `m6_sealed_results.md`; `docs/research/rt0_prior_art_sweep_2026-10-30.md` T7.
:::

::: judge
**JUDGE MAY ASK**

Question: What would you improve with more time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In order: make invalidations durable so a crashed issuer does not leave stale entries; make GET able to use the owner, or at least cluster-wide single flight, since that is where groupcache is ahead; make the Scenario B origin load-dependent and add a cap-free canary arm; run a real T1-to-T2 agreement study; write a new preregistration for a relative-signal controller; run an overhead benchmark in release on a named host; and get an independent human review of the Lab core and a BCa cross-check against a reference implementation.

**If the judge asks further:**

"What would you not do?" I would not claim the product is faster or more consistent than anything until a fair head-to-head run exists.

**Evidence to know:** `docs/release/known_limitations_frozen_code.md` section 5; claims ledger C3, C6, C7; Chapter 8 roadmap.
:::

::: judge
**JUDGE MAY ASK**

Question: What does "H1 NOT_SUPPORTED" actually say, and what does it not say?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It says that on 30 sealed seeds, 8 origin-degradation scenarios and 4 held-out workload families plus one sealed family, an AIMD controller allowed to range at least 4 times its minimum did not reduce peak origin concurrency in any scenario versus the tuned static baseline, and it was credibly worse in seven. It reduced client P99 in two scenarios, and it broke the hit-ratio and error-rate limits in three. It does not say adaptive caching cannot work, it does not say anything about real networks, and it does not test overhead or the stale-window rule.

**If the judge asks further:**

"What was the tuned static baseline?" BL-12b: a static cap and stale window chosen on tuning seeds. The code comment says it is not claimed to be the best deployable static policy.

**Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`, `docs/research/m6_sealed_results.md` "What this does and does not establish".
:::

### Chapter 8: Limitations, Roadmap and Technical Defence <!--gen-->

#### 8.13 Preparing to defend the project <!--gen-->

::: judge
**JUDGE MAY ASK**

Question: A client sends DELETE and gets 204. Is that content guaranteed gone from every node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No. A 204 means the issuing gateway purged its own cache and the canonical primary for that key acknowledged within 150 milliseconds. That primary can be the issuer itself. The other members are informed asynchronously with retries, a repair worker and anti-entropy, but the list of invalidations still owed lives only in the issuer's memory. If the issuer is killed first, peers keep serving the old object until its freshness lifetime ends. We reproduced that on unmodified code. A fix with a durable log exists as a sandbox patch, not in the tree, and it is opt-in. So for deletion-critical content our documentation tells operators to set a short `s-maxage`, re-issue DELETEs after any restart, and verify by reading through each gateway.

**If the judge asks further:**

"What staleness bound do you offer?" None beyond the origin's lifetime, or 60 seconds when the origin gives none. We measured a warm loopback propagation P99 of about 10 to 14 ms in a debug build, but cold first use took 35 to 60 ms, so we withdrew the "zero stale after 50 ms" claim.

**Evidence to know:** G4B-09; `docs/user/limitations.md` "Cluster invalidation durability"; `m3_durability/reproduction_report.md`; `claims_ledger.md` C6 amendment; label PROTOTYPE ONLY for durability.
:::

::: judge
**JUDGE MAY ASK**

Question: Is this linearizable? What is your consistency model?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is not linearizable and we do not claim it. Each node orders its own fills and deletes using generation fencing, so a fetch that started before a delete cannot store its result after it. Across nodes the model is eventual convergence of invalidations under a loopback fault model we tested. Concretely: N independent caches and an invalidation bus. We have drafted numbered consistency clauses with a failure model, a bound and a test that could falsify each, but they are drafts waiting on owner ratification, and several are known to be unmet.

**If the judge asks further:**

"Is even the single-node fencing airtight?" Not proven. A reviewer found that the epochs are read twice in the fill path, so a pre-invalidation body can survive an acknowledged tenant invalidation if the invalidation lands in a nanosecond window. We reproduced it only with an injected yield; a 20-second natural stress run saw nothing. A three-line fix is proposed and not applied because the file is protected.

**Evidence to know:** G4A-01; `docs/research/g4a01_race/`; `src/core/cache.rs::finalize_fetch`; product-direction section 7.
:::

::: judge
**JUDGE MAY ASK**

Question: Can one tenant, or one cluster node, hurt another?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes, in two ways. First, tenants are separated by key and by invalidation epoch, but there are no quotas, so one tenant can consume the byte budget. Second, any authenticated cluster member can send tenant-scope or full-cache invalidation frames, and the receiver does not check scope against the sender's rights. That is finding DR-027 or G4B-05, and the owner has named it as one that must not be dismissed. The plan is per-member scope authorization and tenant-flood caps, in a sandbox, queued behind the durability work. It needs a valid cluster certificate, so the attacker is an insider or a compromised member.

**If the judge asks further:**

"What about a bad peer fabricating cache content?" The cluster `GetRequest` stub writes a placeholder value into the node cache. We reproduced it, and an AI review agreed that no HTTP request can reach that entry because the key spaces are disjoint. We rate it low as exploited today and medium as a latent hazard.

**Evidence to know:** G4B-05, G4B-30; `docs/security/investigations/DR-029_*`; `src/dist/node.rs` (the `val_node_` format); status PARTIAL for security.
:::

::: judge
**JUDGE MAY ASK**

Question: The sealed evaluation says H1 is NOT_SUPPORTED. What does that mean for your "intelligent" caching claim, and what happens to the product?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It means we have no evidence that our adaptive controller beats a well-tuned static origin cap, so we do not claim that it does. In the simulated, pre-registered, run-once evaluation, the adaptive arm did not reduce peak origin concurrency in any of eight scenarios, and in seven it was credibly worse. The unconstrained tuning optimum was itself a static cap. Our research lead therefore chose the static gateway as the minimum viable product and did not promote the adaptive controller to the serving path. The negative result is kept on the record. It does not prove adaptation is useless; it shows no benefit for this controller under this protocol. Next steps are a headroom experiment to see whether any adaptive cap could beat a static one at all.

**If the judge asks further:**

"Was that run reviewed?" By the lead and AI reviewers for integrity of the hash chain, not by a human, and the reviewers did not read the simulation core. Also the P99 endpoint was quantized, and the stale-window mechanism was never exercised.

**Evidence to know:** `docs/research/m6_sealed_results.md`; `known_limitations_frozen_code.md` section 1; status label TESTED, tier SIMULATED.
:::

::: judge
**JUDGE MAY ASK**

Question: What would you do with more time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In order: first, get a human to review the protected core, because everything we know about it is AI second looks. Second, integrate the three sandbox fixes after review: the durable invalidation log, the fence fix for the double epoch read and the Scenario B cancelled-run fix, each with a test that fails without it. Third, close the ingress holes: authenticate before reading bodies, add timeouts and limits. Fourth, run a release-profile suite and the soak on a quiet machine, and run the cluster on real hardware over a network. Fifth, only then start tenant quotas and the headroom experiment for intelligence, because a quota built on an unreviewed fencing layer would sit on sand.

**If the judge asks further:**

"Why not just fix them now?" Because the protected core is frozen for evidence integrity and for safety: changes to fencing and invalidation need a failing-first test, an independent review and the owner's approval. We chose not to merge unreviewed changes to the part of the system whose correctness matters most.

**Evidence to know:** `docs/release/execution_register.md`; `docs/release/owner_decisions_register.md` ODR-2 to ODR-4; Figure 8.2.
:::

::: judge
**JUDGE MAY ASK**

Question: What have you chosen not to claim, and what is untested?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We do not claim production readiness, a distributed cache, linearizability, bounded staleness, adaptive benefit, ML, performance numbers, or independent verification. Untested: any Linux run, any physical network, clusters beyond four nodes, power loss and fsync-error behaviour, a release-profile suite, a two-hour soak, the natural-schedule rate of the fence race, the real behaviour of ingress defaults under attack, human usability and screen readers. Also unverified: whether the simulator ranks scenarios the way a real process would, because no simulator-to-real agreement study was run.

**If the judge asks further:**

"Which of those worries you most?" The unreviewed fencing and invalidation core, because the product's one hard promise is that a deleted object is not served after an acknowledged delete.

**Evidence to know:** 8.10 checklist; 8.11 table; `m3_durability/remediation/00_status_and_plan.md` section 7.
:::

::: judge
**JUDGE MAY ASK**

Question: Your repository is mostly uncommitted. Isn't that a process failure?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is a delivery risk and we list it first. The cause is a deliberate policy: commit only file groups that have been individually reviewed, never everything at once, and never groups with unresolved blocking findings. Documentation and tooling groups are committed; the code groups wait for the review gate and an owner check on held preregistration files. The risk is real: one machine and one local backup. Mitigation is to commit reviewed groups as they clear and to make an off-machine copy, which is still an open action.

**If the judge asks further:**

"Is anything pushed?" No push has occurred. Commit and push need an explicit instruction from the owner.

**Evidence to know:** `docs/release/owner_decisions_register.md` ODR-3; `docs/STATUS.md` section 5b; `git log --oneline -8`.
:::

::: judge
**JUDGE MAY ASK**

Question: The README and an older handoff say the system passed a formal acceptance. Why do your records say otherwise?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Because those statements came from agent self-reports and were later audited. The handoff's "Phase 4 accepted, zero P0" is not supported by later review: no dated owner acceptance is recorded for any milestone, the M3 completion report said "zero compromises" before a durability defect was reproduced, and the M6 report was retracted. We kept the old documents as history, added correction notices, and made `docs/STATUS.md` and the verification register the entry points. In a defence we would show the register, not the old claims.

**If the judge asks further:**

"Could the owner have accepted milestones elsewhere?" Possibly. The register calls missing acceptance an evidence gap, not proof that none was given.

**Evidence to know:** `docs/handoff/CORRECTIONS.md`; `docs/STATUS.md` section 1; `CLAUDE.md` section 6.
:::

# R5. Discrepancy and evidence inventories <!--gen-->

### 1.10 How the vision and the code relate (and where documents disagree)

The vision documents describe more than the code does, and some older documents describe things the code no longer does. A judge may probe this. Know the shape of it.

1. **The original idea was a distributed cache.** The first published README called it "an intelligent in-memory distributed cache", but the cluster was a modelled three-node simulation (about 117 lines in `src/dist` at that commit, per the audit). The reasoning behind "distributed" is not recoverable from the repository.
2. **Sequencing was an owner decision:** single-node first, keep the distributed code (audit section 4).
3. **Positioning drifted.** Agent-written definitions (MVP as a "single-binary gateway", a conditional distributed gate, an accepted ADR with no recorded owner ratification) narrowed the story. The audit concludes this was mostly unintended dilution and says there is no evidence of abandonment: "evidence gaps, not conclusions".
4. **Older handoff documents overstate.** `docs/handoff/00_START_HERE.md` still says "Phase 4 Remediated, Verified, and Formally Accepted (242/242 tests passing)" and describes "64-Partition Lock Striping". The notice at the top points to `docs/handoff/CORRECTIONS.md`, and `CLAUDE.md` section 6 says the "Phase 4 accepted / zero P0" claim is not supported. In the code the serving cache is built with 8 shards (`src/main.rs`).

**Discrepancies between documents and the current code, collected.**

| Document says | Code or later record says | Source of the correction |
|---|---|---|
| README: single-flight is the `Coalescer` in `coalesce.rs` | Serving path uses `SharedOp` plus `PartitionedCache::begin_fetch_atomic`; `_coalescer` in `main.rs` is unused | `src/main.rs`, `src/core/cache.rs` |
| README mermaid: requests go through the rendezvous router to a primary node, then to the coalescer | GET never uses the router; it is used for DELETE | `src/api/server.rs::get_cache_handler`, `::delete_cache_handler` |
| README table: "Cluster Topology & Nodes: Modeled / Simulated" | Partly outdated: with mTLS config the local node shares the serving cache and peers are real addresses; the three demo nodes remain simulation | `src/main.rs` cluster branch |
| Handoff: 64 partitions | 8 shards in `main.rs` | `PartitionedCache::new(8)` |
| Handoff and CLAUDE.md/README: 242 tests, 19 targets | 851 passed, 75 targets (debug, 2026-10-30) | `docs/STATUS.md` section 3 |
| Handoff: "adaptive Policy Inference Engine (PIE) monitoring access skew" | `pie: None`; adaptive benefit not established | `src/main.rs`, `m6_sealed_results.md` |
| CLAUDE.md section 6 (part): peer invalidations purge a side cache | Outdated for mTLS cluster mode: shares the serving cache | CLAUDE.md's own bracketed note; `src/main.rs` |
| CLAUDE.md section 6: "evictors named SIEVE/S3-FIFO/LRU are scans" | Outdated; faithful implementations exist (Chapter 4 verifies complexity) | three-pillar document; `src/core/eviction.rs` |
| Code comment: local-first delete "Guarantees linearizability" | The project must not claim linearizability | `src/api/server.rs::delete_cache_handler`, CLAUDE.md section 4 |
| STATUS.md section 3b/6: HEAD `456bb25` | At the time of writing HEAD is `df68eaa`, after documentation commits | `git rev-parse HEAD` on 2026-10-30 |

---


### 2.11 Discrepancies between documents and code

Recorded in full in `docs/learning/evidence/ch02_evidence.md`. The ones a reader should know:

1. `docs/research/adr/ADR-001-present-architecture-and-direction.md` (dated 2026-10-28, status PROPOSED) is partly out of date. It says DELETE purges the local cache only after the cluster round trip, that no code links a node's cache to the serving cache, and that reconciliation has no caller. Current code purges locally first (`delete_cache_handler`), builds the local node with `new_with_cache` in the mTLS path, and starts the anti-entropy worker in `main.rs`. `CLAUDE.md` section 6 already flags parts of this.
2. `src/config.rs` (around line 858) still warns in cluster mode that "peer invalidations do not purge this gateway's HTTP cache". The startup message in `src/main.rs` (around line 373) and `docs/user/limitations.md` now say the local node shares the serving cache in the mTLS path. The `config.rs` warning text is stale.
3. A code comment in `delete_cache_handler` claims "linearizability". No evidence supports that word (section 2.7).
4. `PHANTOM_SWEEPER_BATCH_SIZE` is described as a "shards per tick" limit (`config.rs` unit) and as a shard inspection limit (`CLAUDE.md`). In `PartitionedCache::sweep_lazy_reclamation` the argument is `max_per_shard`: the sweeper visits every shard each tick and stops adding reclamation candidates in a shard at that count; entries that are not reclaimable are still iterated, so the limit bounds reclamations per shard per tick, not entries inspected. Chapter 4 should confirm the lock-hold consequence.
5. `PHANTOM_DEFAULT_TTL_MS` is unwired (section 2.6.1), as `CLAUDE.md` says; re-verified.


### 3.13 Supply-chain observations (verified list)

All from `Cargo.lock` and the G4C inventory unless marked.

- 288 packages, lockfile v4, **no git or alternate-registry sources** (G4C, reproduced by that review; I confirmed the package count with `grep -c '^name'`). A build with `--locked` (used by `packaging/build-release.*`) uses exactly these.
- 11 duplicated crate names, including `rand` (0.8.8 and 0.10.3), `tower-http` (0.5.2 and 0.6.11), `webpki-roots` (0.26.11 and 1.0.11).
- Two crypto stacks (`ring`, `aws-lc-rs`/`aws-lc-sys`); OpenSSL crates only through dev-only `reqwest`.
- `tokio` `full` and `tower-http` `fs`/axum `ws` widen features beyond use.
- Licences are permissive (MIT / Apache-2.0 / ISC); `webpki-roots` is a CDLA-Permissive data licence (G4C reading of registry manifests).
- **The vulnerability and yank status of every locked version is NOT determined.** `cargo audit` / `cargo deny` has not been run (G4C; STATUS M7: "no advisory scan"). A claim that dependencies are "free of known vulnerabilities" must not be made. `docs/PHASE_2_WORK_REPORT.md` itself says clean linting does not imply dependency safety.
- `Cargo.toml` and `Cargo.lock` are modified relative to HEAD in the working tree, and `Cargo.toml` is one of four files whose hash differs from the M6 freeze record (expected; G4C).
- npm side: `react`, `react-dom` and unused `lucide-react` at runtime; three dev packages; `npm audit` was not run (UNVERIFIED).

::: judge
**JUDGE MAY ASK**

Question: Are your dependencies safe? Have you scanned them?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The dependency set is small and permissively licensed, the lockfile has no git or alternate-registry sources, and release builds use `--locked`. But I have not run a vulnerability scan: neither `cargo audit` nor `cargo deny` has been executed, and the project's own status page lists that as a gap. So I cannot say the locked versions are free of known advisories. I can also tell you what I would tidy: some declared crates are unused, and two crypto libraries are linked.

**If the judge asks further:**

"Why two crypto libraries?" I use `ring` directly for SHA-256, and rustls defaults to `aws-lc-rs`. Consolidating on one is an owner decision that I have not made.

**Evidence to know:** `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (dependency inventory, AI review); `docs/STATUS.md` (M7 row); `docs/PHASE_2_WORK_REPORT.md` scope note. Status: inventory VERIFIED by reading; advisory status UNVERIFIED.
:::


### 4.12 Where this chapter disagrees with other documents

These are differences between sources, recorded rather than fixed. Details and file references are in the evidence file.

1. **`CLAUDE.md` "O(1)-amortized eviction"**: true for SIEVE and LRU only when nothing is pinned; per-call worst case is O(n); S3-FIFO carries G4A-02. `CLAUDE.md` section 6 "evictors are scans" is outdated.
2. **`CLAUDE.md` "lock order: shard, then `tenant_epochs`"**: no code path nests them. The real nesting edges are shard to `epoch_snapshot` (read), `waiters`, `pending_freshness` and the `eviction_policy` mutex.
3. **"Per-tick shard inspection limit" for `PHANTOM_SWEEPER_BATCH_SIZE`** (`CLAUDE.md`, and "shards per tick" in `src/config.rs`): the code bounds candidates reclaimed per shard, and scans the whole shard (G4A-03).
4. **"O(1) tenant flush"** (test comment, `invalidation_contract_tests.rs`): the bump copies the tenant epoch map, O(T) up to 10,000.
5. **SEC-4A-01 comment "cluster-wide"**: the global epoch is per process.
6. **"Read-path zero contention"**: true for the epoch read; the shard mutex is still taken by every request, twice on a hit.
7. **"`stale_cap_violations` must be 0"**: true, but near-tautological (G4A-18).
8. **`CLAUDE.md` line numbers for the `60_000` literal** (1077, 1352) are stale; the fact holds (lines 1405 and 1688 in this tree).
9. **The `test_waiter_capacity_bound` soak test** is named as if it tests the bound but asserts nothing about it; the bound is covered elsewhere.
10. **`KeyState.visited`** looks like an eviction input but no policy reads it.


### 6.13 Every real measurement in the repository

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


### 7.10 Discrepancies found while writing this chapter

| # | Document says | Code or retained evidence shows |
|---|---|---|
| 1 | `src/lab/mod.rs` header: T2 helpers "remain in `tests/support/lab`" | `src/lab_live/` now exists and is the T2 implementation |
| 2 | Virtual clock described as the simulation clock | The Lab `VirtualClock` (`src/lab/clock.rs`) is used only in `lab_core_tests.rs`; `run_cell` uses event timestamps (a different `VirtualClock` exists in `src/sim/clock.rs`) |
| 3 | `matrix_runner.rs` return comment "rank_concordance_spearman" | computes the fraction of positional matches, not Spearman |
| 4 | `RunManifestV2` with EV-1 to EV-14 | 3 of 14 checks; hard-coded git hash; test-only |
| 5 | `origin_sim.rs` header "M/M/c" | M6 uses constant service time (M/D/c-like) |
| 6 | `analysis.rs` header "BCa" for the module | the synthetic validator and `evaluate_paired` use a percentile bootstrap |
| 7 | `novelty_snapshot_m6.md` "RT-0 PASS" | its own correction notice, `rt0_prior_art_sweep`, and STATUS: RT-0 OPEN |
| 8 | `claims_ledger.md` section 1 table (C1, C2, C7 HYPOTHESIS) | section 7 reconciliation: C1 NOT SUPPORTED, C2 NOT TESTABLE, C7 UNVERIFIED |
| 9 | `docs/demo/README.md` describes a demo launcher and Showcase/Explorer modes | not the Lab; it appears to be an older demo guide (not investigated further) [UNVERIFIED relevance] |



### 8.4 Known defects and risks, by theme

#### 8.4.1 What the G4 register is

Gate G4 asked for two whole-product review cycles. Cycle 1 on the Rust side produced three reports (`docs/release/g4/cycle1_core_engine_review.md`, `cycle1_dist_security_api_review.md`, `cycle1_lab_intelligence_deps_review.md`): 26 findings from G4A (core engine), 38 from G4B (distribution, security, API), and 35 from G4C (lab, intelligence, freeze, dependencies). Total **99**. The disposition register (`cycle1_findings_disposition_register.md`) adds the status of each. Four caveats govern how to read it:

1. All three reviews are **AI second looks**: one AI pass each, read-only, no tests run. "CONFIRMED" in the reports means confirmed by reading code, not by an executed failing test.
2. The register closes nothing. Only owner statements (ODR-1 to ODR-4) set dispositions.
3. G4C did not read the simulation core of the lab (listed in `docs/release/known_limitations_frozen_code.md` section 1), so the absence of a finding there is absence of review.
4. The register's grouping into commit groups is an inference.

Counts by severity and disposition (register section 3.1): Critical 0; High 4; Medium 37; Low 42; Info 16. Dispositions: 75 OPEN (of which six owner-named), 20 RECORDED-LIMITATION, 2 IN-SANDBOX-REMEDIATION, 1 UNDER-INVESTIGATION, 1 REFUTED.

#### 8.4.2 The 99 findings by theme

The themes below are **my grouping** of the register's IDs (the register groups by report, not by theme). I computed the counts with a short script over the register rows and checked that the totals reconcile to 99. A "RECORDED" finding is one the owner decided to record as a known limitation in frozen code (ODR-1). "Sandbox" means a fix exists as an unintegrated patch in a scratch copy.

| Theme | Count | Highest severity | Dispositions | Representative IDs |
|---|---:|---|---|---|
| Fencing, epochs, invalidation races | 4 | High | 1 under investigation (G4A-01), 3 open | G4A-01, -08, -12, -17 |
| Eviction, byte accounting, resource bounds | 8 | High (conditional) | 8 open | G4A-02 (S3-FIFO tombstones), -03 (sweeper scan), -13 (byte drift) |
| HTTP freshness semantics and config wiring | 8 | Medium | 8 open | G4A-04 (upstream `Age` ignored), -05 (`Expires: 0`), -06 (stale-if-error on 4xx), -25 (TTL literal) |
| Telemetry honesty and clocks | 5 | Medium | 5 open | G4A-10 (wall clock), -11 (hit-rate definition), F24 (percentile conventions) |
| Tenancy alias and lock robustness | 2 | Low | 2 open | G4A-19, -21 |
| Ingress, denial of service, rate limiting | 7 | High | 6 open, 1 owner-named open | G4B-01 (slow body before auth), -02, -11 |
| Cluster trust, membership, invalidation delivery | 12 | Medium | 8 open, 4 owner-named open | G4B-05 (DR-027), -06, -07, -08, -09, -27, -30 |
| Authentication, JWT, SSRF | 4 | Medium | 4 open | G4B-13 (`nbf`, lifetime), -14, -15, -18 |
| API surface and control hygiene | 14 | Medium | 13 open, 1 owner-named open | G4B-03 (`/api/control` mutates live nodes), -04, -16, -20 |
| Dependencies | 2 | Info | 2 open | G4B-34, G4C-DEP (no `cargo audit` run) |
| Scenario B and Lab live | 9 | High | 2 in sandbox (F01, F02), 7 open | F01 (U-01), F02, F06, F07, F08 |
| Frozen lab verdict and provenance | 13 | Medium | 10 recorded, 2 open, 1 refuted | F03, F04, F05, F16, F27, F34 (refuted) |
| Intelligence library (not wired) | 10 | Medium | 10 recorded | F09 to F15, F23, F25, F26 |
| Documentation consistency | 1 | Low | 1 open | F28 (README MSRV claim) |
| **Total** | **99** | | 75 open (6 owner-named), 20 recorded, 2 sandbox, 1 under investigation, 1 refuted | |

Reading the table honestly: the **runtime-serving risk is concentrated in four themes**: ingress hardening, cluster trust, fencing races, and freshness semantics. The intelligence and frozen-lab themes carry many findings but mostly in code that is **not wired into serving** (G4C's grep; the register's "LATENT" impact class). They matter for research validity and for any future activation, not for today's request path.

#### 8.4.3 The six findings the owner named

ODR-4 names six findings that "must not be dismissed": G4B-01 (the HTTP ingress reads a request body before authentication, with no body timeout or connection limit), G4B-03 (`/api/control` mutates the live router nodes and a missing event sets every node Active), G4B-05 (any active cluster member can send tenant or full-cache invalidation scopes; DR-027), G4B-06 (no membership-epoch check on `InvalidateRequest`; the acknowledgement-epoch check is vacuous because the receiver echoes the sender's epoch), G4B-07 (cluster membership is static and the revocation machinery has no production trigger) and G4B-08 (live mutual-TLS connections never expire, so certificate expiry and rotation are enforced only at handshake). All six are **OPEN**. None has a fix in the tree. G4A-01 is separate: the owner calls it **unconfirmed** and ordered an investigation.

#### 8.4.4 Findings I re-checked against the current tree

The register says "AI second look" and the line numbers in reports drift. I opened the current code for several High and Medium items. This is still reading, not execution.

| Finding | What I opened | What I saw | Result |
|---|---|---|---|
| G4A-01 (stale body survives an acknowledged invalidation) | `src/core/cache.rs`, `finalize_fetch` region | The fence check compares `state.generation == op_gen` and the entry epochs to `get_effective_epochs(...)`; later, in the insert block, the code calls `get_effective_epochs` again and stamps `entry_tenant_epoch` and `entry_global_epoch` with the **newly read** values. | Mechanism present in the tree as the reviewers describe it. Natural-schedule reproduction was not achieved (see 8.8). |
| G4A-02 (S3-FIFO `on_remove` leaves tombstones) | `src/core/eviction.rs`, `FaithfulS3Fifo::on_remove` | It removes the key from `freq` and `ghost_set` only; it does not touch `small_q` or `main_q`. | Present. Reachability: `set_eviction_policy` is called only from `src/control/pie.rs` (not instantiated: `pie: None` in `src/main.rs`) and from `src/intelligence/executor.rs` (`apply_policy`, no serving caller per the limitations record). `PartitionedCache::new(8)` starts with the default policy. So S3-FIFO is not reachable in the default serving path. I did not exhaustively search all callers. |
| G4A-03 (sweeper scans the shard) | `src/core/cache.rs`, sweeper loop | The loop iterates `shard.map.iter()` and breaks when `candidates_to_reclaim.len() >= max_per_shard`, so the batch limit bounds candidates, not keys inspected. | Present. |
| G4B-01 / G4B-03 (extractors before authentication) | `src/api/server.rs`: `control_handler`, `get_cache_handler`, `delete_cache_handler`, `create_app_with_lab` | `control_handler` takes `Json(payload)`; the GET and DELETE handlers take `_body: axum::body::Bytes`; all three extract the body before the handler can authenticate. The only limit found is `DefaultBodyLimit` at 512 KiB; no timeout, concurrency or connection layer. `control_handler` also falls through to "set every node Active" for an unknown event. | Present for all three handlers. The runtime effect of a slow body remains unverified. |
| G4B-13 (JWT `nbf` not validated) | `src/security/jwt.rs` | `Validation::new(HS256)` with `leeway = 10`; no `validate_nbf` assignment seen. | Consistent with the finding (library default). Not tested. |
| G4B-27 (unstable hash) | `src/dist/router.rs`, `route_key` | `DefaultHasher` used. | Present. |
| TTL literal (G4A-25, CLAUDE.md) | `src/api/server.rs` | `60_000` passed to `evaluate_response_freshness` at two sites; `src/config.rs` registers the key as not wired. | Present. |
| F28 (README MSRV) | `README.md` | Line says `cargo 1.75+`; `Cargo.toml` has no `rust-version`. G4C says locked crates need about 1.88. | Present; the 1.88 figure is G4C's, I did not re-derive it. |

I did **not** re-check the lab and intelligence findings by execution, and I did not run any test.

#### 8.4.5 The sealed result and what the limitations record says

The sealed M6 evaluation (Tier 1, **SIMULATED**; Chapter 7, section 7.6) ran once and found H1 **NOT SUPPORTED** (`docs/research/m6_sealed_results.md`). `docs/release/known_limitations_frozen_code.md` says the result "has NOT been independently reviewed by a human or an external reviewer", that G4C did not read the simulation core, and that the run is a forced-adapt comparison on a simulator. Four of 56 hashed freeze files differ from the frozen values (`Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, `tests/integration/m6_research_evaluation_tests.rs`); an addendum written after the run explains all four, and the `eval.rs` verifier change is a refactor (+108/-2), not a purely additive edit. The frozen versions can be rebuilt only from copies outside the repository (G4C F27). Treat the freeze chain as reproduced by the lead and an AI, not audited by a person.

#### 8.4.6 Other recorded risks

- **SEC-M7-09, -10, -11** (role alias and bare tokens, SSRF range gaps, rate limiting and JWT lifetime) are open (`docs/security/m7_security_review.md`; execution register E3, queued). SEC-M7-01 to -08 are recorded as fixed and tested.
- **Hostile-input paths**: the HTTP ingress has no host or origin guard in anonymous mode (G4B-04) and `/api/events` has no stream cap (G4B-02).
- **Telemetry honesty**: the hit-rate function returns 1.0 with no data (G4A-11). UI values must be labelled measured, simulated, inferred or unavailable (`CLAUDE.md` section 4); the UI increments I1 to I4 aim at that, but visual and usability acceptance is open.
- **Dependencies**: 288 locked packages, two TLS crypto stacks, broad feature flags; **no advisory scan has been run** (G4C-DEP, execution register E4 queued).


### 8.7 Technical debt and documentation honesty

#### 8.7.1 Debt in the tree

- **Everything uncommitted** (8.2.1). The single largest delivery risk.
- **Protected-core gates.** `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml` are frozen or protected. Changes touching fencing, epochs or invalidation need an independent review pass and a test that fails without the change (`CLAUDE.md` section 5).
- **Preregistration holds.** The two commitment files and the code holding commitment strings are on hold for an owner check offline, which blocks committing the Rust code groups (execution register, owner decisions register "Still open").
- **Formatting.** `cargo fmt --check` fails (38,042 diff lines); formatting would rewrite frozen files, so it waits for addendum 2.
- **Line endings.** `core.autocrlf=true` is set. Commit `df68eaa` added a `.gitattributes` that marks exactly four hash-pinned frozen files `-text` (its comment cites DEC-006, owner-approved, Option B), so their bytes are not converted. There is no repository-wide normalisation, so other files can still change line endings on another machine.
- **No release-profile suite**; the soak (2 h) has not been run.
- **Weak tests.** Tests that pin weak claims (golden trace, cap counter) are flagged for rewrite (G4A section 5); the UI fixture masked U-01 (`ui/test/lab.view.test.mjs`, F01).

#### 8.7.2 Statements in project documents that are out of date

I compared the main status-bearing documents with the current code and records.

| Document | Statement | Reality (current evidence) |
|---|---|---|
| `docs/handoff/00_START_HERE.md` | "Phase 4 Remediated, Verified, and Formally Accepted (242/242 tests passing across 19 targets)"; "Do NOT begin Phase 5" | The same file carries a notice that prior claims were audited and corrected (`docs/handoff/CORRECTIONS.md`). Formal acceptance is not recorded for any milestone; the test baseline is now 851 across 75 targets (debug). `CLAUDE.md` section 6 says the "Phase 4 accepted / zero P0" claim is not supported. |
| `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md` | "ZERO open P0 code blockers remain in Phase 4" | Superseded: G4B-01 and others are open; its roadmap is marked historical. |
| `docs/handoff/00_START_HERE.md` | "64-Partition Lock Striping" | `src/main.rs` constructs `PartitionedCache::new(8)` (verified); `new` takes `num_shards` and asserts a power of two. The gateway starts with 8 shards, not 64. A handoff claim, not verified elsewhere. |
| `CLAUDE.md` section 6 | evictors "are scans"; "the only staleness bound is a fixed 60 s TTL"; reported baseline 242 tests | Section 5 of the same file lists `ShardEvictionPolicy` with `FaithfulSieve`, `FaithfulLru`, `FaithfulS3Fifo` (verified present in `src/core/eviction.rs`), so the "scans" note is outdated; origin headers and a stale cap also bound staleness (the 60 s is the default only); 242 is historical. Parts of section 6 already carry "OUTDATED" annotations. |
| `CLAUDE.md` section 6 / `src/main.rs` startup text | "startup message at `main.rs:368`" says peer invalidations do not purge the HTTP cache | The mTLS path builds the local node with `VirtualNode::new_with_cache(node_id, cache.clone())` (verified at `src/main.rs` near line 332), so the local node shares the HTTP serving cache. The insecure-cluster message in `main.rs` now describes a different situation (no listeners, DELETE reports a gateway timeout). But `src/config.rs` still emits "peer invalidations do not purge this gateway's HTTP cache" (verified near line 858) — contradicts the mTLS path (G4B-32). |
| `README.md` | "cargo 1.75+" | False per G4C F28 (about 1.88 needed; not re-derived). |
| `docs/research/claims_ledger.md` C6 | "zero stale reads after 50 ms" | Amended in the same file (section 5, C6 amendment): cold first use took 35 to 60 ms in every isolated run, so "P(stale)=0 at 50 ms" is **not supported**. |
| `docs/research/claims_ledger.md` section 5 / round 2 | "histogram has no callers in `src/`" | Partly outdated. `TelemetryCollector::record` and `record_us` do feed `AtomicLogLinearHistogram`. But no production code reads its percentiles: `src/api/server.rs` calls only the ring-buffer version (`calculate_percentiles_us`), and `calculate_percentiles_histogram_*` have no caller outside tests (grep, 2026-10-30; Chapter 6, section 6.12.2). |
| `docs/research/code_findings.md` (2026-10-28) | evictors are scans; SIEVE "is not SIEVE" | Predates the faithful policies. Treat as historical. |
| `docs/STATUS.md` | UI tests "298" | Superseded: the suite is 483 tests in 30 files after the redesign added tests (measured, 8.2.2). Minor. |

The reading rule that follows: **trust `docs/STATUS.md`, the ledger and verification register for status; trust the code for behaviour; treat the handoff and the README as history.** The register also holds an acknowledged discrepancy of its own: G4A-19 and the G4B refutation of the `__default__` tenant alias point in different directions and nobody has reconciled them (register section 2, discrepancy 1).


# R6. Restated explanations (kept for reference) <!--gen-->

### 4.0 How to read the status labels

Labels used here: **IMPLEMENTED** (code exists), **TESTED** (a named test exercises it; the text says what it does not prove), **PARTIAL**, **UNVERIFIED**, **OPEN** (a finding in the G4 register that is not closed). Nothing in this chapter is **INDEPENDENTLY VERIFIED**. The G4 reviews quoted here are "AI second look": one AI pass reading code, no human review. See `docs/release/g4/cycle1_core_engine_review.md`, section 0 of that file.

Three kinds of sources disagree at times: `CLAUDE.md` section 5, older reports, and the code. This chapter follows the code and lists differences in section 4.12.


## Appendix B: The complete architecture, explained once

This appendix tells the whole story in one pass, from a client's request to the origin and across the cluster. It does not re-teach. Each paragraph points to the chapter section that owns the detail. Status labels are the ones the chapters use: IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, NOT WIRED, UNVERIFIED. Nothing in this handbook is INDEPENDENTLY VERIFIED by a human.

![Figure B.1: PhantomCache architecture overview. One gateway process, the origin outside it, peer gateways reached only by invalidation frames, and the Lab and UI off the request path.](assets/bm02_architecture_overview.svg)

Source: Figure 2.1 (`ch02_components.svg`), Figure 5.1 (`ch05_cluster_topology.svg`), Figure 2.6 (`ch02_lab_ui.svg`); `src/main.rs::run`, `src/api/server.rs::create_app_with_lab`, `src/core/cache.rs::PartitionedCache`, `src/dist/router.rs::RendezvousRouter`, `src/intelligence/executor.rs::OriginGate`.


### B.1 What the system is, in one paragraph

PhantomCache is a read-through caching gateway: a Rust program that sits in front of one origin, which is the operator's own data service named by `ORIGIN_URL`. Clients call `GET /api/v1/cache/<path>` and `DELETE /api/v1/cache/<path>` on the gateway. The gateway answers reads from memory when it can and from the origin when it must. There is no write route: to change data you change the origin and then `DELETE` the key, or wait for the entry to expire (see Chapter 2, section 2.1). It is a research prototype. The single-node engine is IMPLEMENTED and TESTED. The cluster is experimental: fan-out, repair and anti-entropy are IMPLEMENTED and TESTED, but only on one host over loopback, and durable invalidation and coordinated reads are not done (see Chapter 8, section 8.12).


### B.2 The process and what runs in it

One OS process, `phantom-cache-demo`, runs a Tokio multi-thread runtime and an Axum router (see Chapter 2, section 2.1.2 and Chapter 3, section 3.3). Inside it live the ingress, the handlers, one `PartitionedCache`, an optional cluster router and listener, and background tasks: a sweeper every 100 ms by default, a metrics sampler, and a server-sent-events feed. The React UI is a separate Vite dev server on port 3000. The gateway does not serve it. The Lab is a separate tool (`phantom-lab`) or, when `PHANTOM_LAB_ENABLED=true`, an extra set of routes (see Chapter 2, sections 2.1.3 and 2.8). The component table is in Chapter 2, section 2.2.1.

One caution about the word "node". In the standalone gateway there is one real cache plus three placeholder `VirtualNode` objects for dashboard demos. Those demo nodes are PROTOTYPE ONLY and synthetic. In the mTLS cluster configuration the local member is built with `VirtualNode::new_with_cache`, so it shares the gateway's serving cache, and remote members are address-only descriptors with no cache in this process (see Chapter 2, section 2.2.1).


### B.3 The front door: ingress, authentication, tenants

A request first meets the router layers: CORS and a 512 KiB body limit. If `PHANTOM_JWT_SECRET` is set, the handler authenticates the caller. The JWT algorithm is HS256 only. The check covers signature, expiry, issuer and audience, then a role: `cache:read` to read, `cache:invalidate` to delete. A non-empty `tenant_id` claim is required. In secure mode the tenant comes only from the verified token, never from the `x-tenant-id` header (see Chapter 6, section 6.2). With no secret the gateway starts only on a loopback bind, and a non-loopback bind without a secret is refused (see Chapter 2, section 2.3.1).

The cache key is the tenant plus the path plus the raw query. With no tenant it is the literal `__default__`. `?a=1&b=2` and `?b=2&a=1` are two keys. A key is validated for length and unsafe characters before use (see Chapter 2, section 2.3.2 and Chapter 4, section 4.1). Tenancy here is key isolation. There are no per-tenant quotas, and any pinned cluster member can flush any tenant's cache (see Chapter 6, section 6.6 and Chapter 8, section 8.3.5).

Known open items at this door: the handlers read the body before they authenticate (G4B-01, OPEN, owner-named), there is no rate limit, there is no token revocation and there is no `nbf` check (see Chapter 6, section 6.2). These are listed rather than hidden. Authentication is IMPLEMENTED and TESTED for the covered paths. The system is not security-certified and has had no penetration test (see Chapter 6, section 6.1).


### B.4 The cache core: 8 shards

The cache is split into 8 shards. Production code builds `PartitionedCache::new(8)` in `src/main.rs`, and no configuration key changes it. Each shard is one `std::sync::Mutex` around a map of keys, an in-flight map, byte counters and one eviction policy. The shard for a key is the hash of the key masked by 7 (see Chapter 4, section 4.2).

Different keys on different shards never wait for each other. Different keys on the same shard queue for a short critical section. One hot key always lands on one shard, so sharding cannot spread it. A hit takes the shard mutex twice in the HTTP path (once in `check_stale_serving`, once in `begin_fetch_atomic`) and the response body is copied. Contention has not been measured on a release build (see Chapter 4, section 4.2 and Chapter 6, section 6.13).

The rule that holds the design together: never hold a `std::sync::Mutex` across `.await`. The leader runs the origin call in a spawned task that holds no shard lock while it waits (see Chapter 3, section 3.5 and Chapter 4, section 4.2). No model checker has been run on the locking, so "no deadlock" is argued from reading, not proven (see Chapter 4, section 4.2).


### B.5 A hit

If the key's entry exists, is not past its lifetime and carries the current tenant and global epochs, the gateway returns it with `X-Cache-Status: HIT` and an `Age` header. Nothing leaves the process. The hit path is IMPLEMENTED and TESTED in `m1_http_tests` (see Chapter 2, section 2.4). Micro-benchmark numbers for the hit path exist, but none is an established release-profile product figure. Quote them only with the conditions given in Chapter 6, section 6.13.


### B.6 A miss: single-flight, then the origin

On a miss, `begin_fetch_atomic` runs under the shard lock and has three outcomes. If a fetch for that key is already in flight, the caller joins its waiter list (at most 1,000 per key, else 503 `WaiterLimitExceeded`). Otherwise the shard evicts, at most 32 victims, to make room, and if there is still no room the request fails with `CapacityExceeded`. Otherwise it records an operation id and the key's current generation and registers the fetch. This request is the leader (see Chapter 2, section 2.5 and Chapter 4, section 4.4).

The leader fetches in its own task, so a client that disconnects does not cancel the fetch for others. Only the leader, a bypass request, or a background revalidation leader takes an origin permit. Waiters never do (see Chapter 4, section 4.4).

#### B.6.1 The origin gate

An optional concurrency gate limits distinct origin fetches. It is enforced only when `PHANTOM_INTELLIGENCE` is `canary` or `active` and `PHANTOM_ORIGIN_CAP_MAX` is set. In the default mode `off` the origin path is uncapped, and the gate only counts (`origin_cap_enforcing`). When it refuses, the caller gets a stale copy if one is eligible, else 503 with `Retry-After: 1`. The cap can be static, or can move by an AIMD rule (multiply by 0.70 when the window's p99 or error rate is bad, otherwise add 1). IMPLEMENTED and TESTED for its arithmetic and permit accounting. The benefit of the adaptive version is NOT established (see Chapter 4, section 4.4 and Chapter 5, section 5.13).

#### B.6.2 The fetch itself

`fetch_origin_raw` checks the origin against the upstream policy, resolves the name, filters every returned address against loopback, private and metadata ranges, pins the validated address and connects. Timeouts are 5 s per phase, the body cap is 2 MiB and the header cap is 64 KiB. Redirects are not followed. Each fetch opens a new connection (`Connection: close`). The client's `Authorization` header is not forwarded unless `PHANTOM_FORWARD_UPSTREAM_AUTH` is set. The origin host is fixed by the operator, so the main SSRF risk is removed by design (see Chapter 2, section 2.5.3 and Chapter 6, section 6.8).

#### B.6.3 Store, fencing and waiters

When the answer arrives, `finalize_fetch` decides whether the cache may keep it, under the shard lock. The body is stored only if the key's generation still equals the recorded one, the state object is the same one, and the tenant and global epochs still match. `DELETE` bumps the generation, so a slow fetch cannot resurrect a deleted key. Otherwise the body goes to the waiters but is not stored, and `stale_writes_prevented` increases (see Chapter 4, section 4.5).

The mechanism is IMPLEMENTED and TESTED, with one honest open item. `finalize_fetch` reads the epochs twice, and an epoch bump that lands between the reads can let an old body be stored with the new stamp. This is G4A-01. It was reproduced only in a sandbox with a test hook, and the small fix lives in `docs/research/g4a01_race/proposal.patch` and is NOT integrated (see Chapter 4, section 4.7). A request that joins an older in-flight fetch after a tenant or global epoch change can also receive the older body once (G4A-08). The generation fence catches `DELETE`. Only epoch equality catches tenant or global invalidation (see Chapter 4, section 4.5).

Cancellation and panics are handled by guards that finalize a fetch as aborted if it is dropped early. By design, eviction skips a key with an active operation or in-flight fetch; this is argued from reading and tested, not model-checked (see Chapter 4, section 4.5).


### B.7 Memory, freshness and eviction

**Bytes.** Each shard keeps a running count of logical bytes: key length, body capacity, content-type and ETag lengths and a fixed per-entry overhead. `audit_bytes()` recomputes the total so tests can compare. This proves the bookkeeping is consistent. It is not the operating system's memory figure, and some structures are not counted (see Chapter 4, section 4.6). The default total budget is 256 MiB, divided by the shard count, plus an entry limit per shard.

**Freshness.** The lifetime of an entry comes from the origin: `s-maxage`, `max-age`, or `Expires` minus `Date`. If the origin says nothing, the gateway uses 60,000 ms. That 60 s is a literal in `src/api/server.rs`, passed at two call sites. The setting `PHANTOM_DEFAULT_TTL_MS` is validated and reported but is NOT WIRED: setting it changes nothing. The documented default and the effective default match only by coincidence. There is no gateway ceiling on an origin-set lifetime (see Chapter 4, section 4.8 and Chapter 2, section 2.6.1).

**Stale serving.** Expired data is served only if the origin's response carried `stale-while-revalidate` or `stale-if-error`, and always inside the hard cap `PHANTOM_MAX_STALE_MS` (24 h by default). The cap bounds the stale-serving window, not the freshness lifetime. `must-revalidate` forbids stale serving. A 304 reply refreshes the stored entry. There is no circuit breaker, no negative caching and no retry loop. Errors are not stored, so recovery is immediate when the origin returns (see Chapter 2, section 2.6 and Chapter 4, section 4.8). Freshness is PARTIAL: it is an RFC 9111 subset with open conformance findings (see Chapter 4, section 4.11).

**Eviction.** The default policy is SIEVE. LRU and S3-FIFO are selectable. All three implement `ShardEvictionPolicy` and skip pinned keys. They are not worst-case O(1) when keys are pinned, S3-FIFO carries an open defect (G4A-02), and there is no hit-ratio or release-profile comparison (see Chapter 4, section 4.9).

**Invalidation inside one process.** `DELETE` removes one key and bumps its generation in O(1). A tenant invalidation raises the tenant's epoch, and entries with an older stamp are treated as absent the next time they are touched or when the sweeper finds them. Readers get the epochs from an `RwLock` snapshot without taking the `tenant_epochs` mutex, but the shard mutex is still taken. The tenant map is capped at 10,000. A new tenant arriving when full raises the global epoch, which invalidates everything on that node. That fallback is peer-triggerable (G4A-12). The sweeper scans each shard in full under its lock (G4A-03) (see Chapter 4, section 4.7).


### B.8 DELETE and the cluster

A `DELETE` purges the local cache first, so later reads on this gateway miss. With no router that is all, and the reply is 204. With a router, the gateway uses rendezvous hashing to choose a primary for the key and sends one `InvalidateRequest` over mutual TLS. It waits at most 150 ms (`PHANTOM_CLUSTER_TIMEOUT_MS`) for an `InvalidateAck`. A 204 means the primary acknowledged in that window. It does not mean every member has purged. If the primary is silent, it is marked Suspect and the first healthy replica acts as primary (`X-Cache-Warning: PrimaryFailover`). If that also fails the client gets 504 and every other member is recorded as owed. The local purge happens in every case (see Chapter 2, section 2.7.3 and Chapter 5, section 5.5).

**Rendezvous hashing** gives each node a score from the key and the node id; the highest score wins. In this code the hash is `DefaultHasher`, the weight factor is the constant 1.0, and the router makes two passes (healthy nodes first, then Suspect and Recovering). So it is plain rendezvous hashing, not weighted. Weighted hashing is DESIGNED BUT NOT IMPLEMENTED. Its use is narrow: it picks which node gives the acknowledgement for a `DELETE`. It does not decide where data lives. About 1 in N keys has the issuer itself as primary, and then the 204 carries no evidence about any other node (see Chapter 5, section 5.3).

**GET is local-only.** `get_cache_handler` never touches the router. The function that would forward a read to the key's owner, `forward_key_over_tcp`, has no caller. The peer-side `GetRequest` handler is a stub that fabricates a value, which is a recorded defect (DR-029), not a feature. So a cluster of N gateways is N independent read caches kept coherent by invalidation, not a partitioned distributed cache. Coordinated reads are DESIGNED BUT NOT IMPLEMENTED (see Chapter 2, section 2.7.4 and Chapter 5, section 5.8).

**The rest of the propagation.** After the primary's ack, every other member (not only replicas) is told asynchronously. A per-peer queue of 256 and up to four attempts handle live members. Members that are not live, and deliveries that fail, go to a bounded repair ledger. A repair loop every 250 ms re-sends owed entries, and a pull-based anti-entropy worker (also 250 ms) repairs sequence gaps using a ring of 4,096 sent records, or flushes the whole receiver cache when history was overwritten (see Chapter 5, section 5.5).

**Membership and transport.** Membership is a static file read at startup. There is no join or leave protocol. The wire format is the PhantomMesh binary frame: magic `PHAN`, a 28-byte header, a payload of at most 1 MiB and keys of at most 4,096 bytes. Connections use TLS 1.3 only with a cluster CA as the sole trust root. Identity is a single DNS SAN `node-<id>.<cluster_id>.phantom.internal`, pinned to a fingerprint in the registry. Every frame is re-checked, and an `InvalidateRequest` whose originator differs from the TLS identity closes the connection. Heartbeats are connection keepalive, not failure detection (see Chapter 5, section 5.2 and Chapter 6, section 6.4). Members are trusted: a compromised member can flush others (G4B-05, DR-027).

**What is and is not guaranteed.** On one gateway, a read after a completed `DELETE` does not return the deleted entry, subject to the open G4A-01 and G4A-08. Across gateways there is best-effort asynchronous invalidation with retry, a bounded in-memory ledger and pull repair, which converged within milliseconds in loopback tests while the issuer stayed alive. The repo does not provide linearizability, and it provides no staleness bound if the issuer crashes with owed invalidations or if a partition is permanent, other than the entry's own freshness lifetime. The durable-log fix is a sandbox patch, PROTOTYPE ONLY, and is not in the tree. All cluster evidence is single-host loopback in a debug build (see Chapter 5, sections 5.6, 5.7 and 5.9; Chapter 8, section 8.11).


### B.9 Observation and intelligence

**Telemetry.** A latency histogram `AtomicLogLinearHistogram` has 2,688 buckets and constant-time recording. Its design error is at most 0.39 percent. In the current tree it is written on every GET but no production endpoint reads it. The percentiles on `/api/v1/metrics` come from a sorted 10,000-sample window. Counters and an origin-latency histogram feed the observation routes (see Chapter 5, section 5.11). Every displayed value should carry a provenance label: measured, simulated, inferred or unavailable. With `PHANTOM_SIMULATION=true` parts of the live event stream are synthetic (see Chapter 2, section 2.8).

**Intelligence.** It is not AI and not machine learning. On the serving path there is only the origin gate described in B.6.1. Everything else is library code that no request calls: the signal pipeline (Count-Min plus Space-Saving heavy hitters, EWMA, CUSUM), the shadow evaluator, the decision engine with its confidence interval, canary routing and a watchdog. These are PROTOTYPE ONLY, and the statistics in the evaluator and engine have recorded flaws (F12, F13). `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test. The legacy `src/control/pie.rs` is a threshold stub, and `main.rs` passes `pie: None`. The hot-keys field of `/api/v1/metrics` reports "unavailable". Hot keys are in practice protected by coalescing (see Chapter 5, sections 5.10, 5.12 and 5.13). The hard caps (stale cap, origin cap, byte budget, fencing) are enforced by the execution layer regardless of any intelligence decision (see Chapter 5, section 5.13 D).


### B.10 Phantom Lab and the UI

Phantom Lab has two tiers, kept apart. T1 is a deterministic discrete-event simulation with a virtual clock and seeded random streams. It drives the real cache, coalescing, fencing and origin-gate code but simulates clients and the origin, and it does not run HTTP, TLS or any cluster. Its results are labelled SIMULATED. T2 starts the real gateway binary as a child process on loopback, against a scripted raw-TCP origin, with an open-loop load generator. Its results are labelled MEASURED_LOOPBACK, on one host, and are not multi-host or production figures (see Chapter 7, sections 7.2 and 7.5).

Experiments compare arms paired by seed, using a BCa bootstrap interval and a sign-flip permutation test, with Holm correction across hypotheses (see Chapter 7, section 7.3). The sealed M6 evaluation (30 sealed seeds, 8 scenarios, simulated) gave hypothesis H1 the verdict NOT_SUPPORTED: a forced-adaptive controller did not beat the best tuned static cap. The best tuned adaptive setting was itself a static cap. Scenario B on a real gateway showed that a cap limits origin concurrency at a cost in refused requests, and that the adaptive arm acted as a static cap near 12. No agreement study between T1 and T2 has been run (see Chapter 7, sections 7.5 and 7.6). The product position that follows is a gateway with a configurable static origin cap.

The Lab API (`src/api/lab_api.rs`) exists only if `PHANTOM_LAB_ENABLED=true`. Without JWT it requires a loopback bind and checks the `Host` and `Origin` headers. The service refuses to start if the sealed salt is present in its environment (see Chapter 2, section 2.8 and Chapter 7, section 7.7). The React dashboard shows telemetry and the Lab. Its Cluster and Intelligence screens are placeholders. It has automated checks only, with no human design or accessibility review (see Chapter 8, section 8.12).


### B.11 Security boundaries in brief

Five zones: clients, the gateway process, the operator-fixed origin, cluster peers and the local host (see Chapter 6, section 6.1). At each boundary the repo has a named check: JWT roles and tenant claims at the ingress; the upstream policy, DNS pinning and an address filter at the origin; certificate pinning and originator matching on the cluster link; a loopback-only rule with Host and Origin checks on the Lab API. The bounds that exist are listed in Chapter 6, section 6.8 (bytes, waiters per key, epoch map, response size, frame size, handshakes, per-peer queues, ledger size). The bounds that do not exist are listed there too: inbound connection count, `/api/events` streams, origin concurrency by default, per-tenant rates. The host environment holds the JWT secret, so a host-level attacker is out of scope.


### B.12 Where the architecture stops

Seven limits shape every defence answer (see Chapter 8, section 8.3).

1. One cache per gateway, served locally.
2. Placement is plain rendezvous hashing, used for `DELETE` only.
3. Owed invalidations live in the issuer's memory.
4. Freshness bounds are weak: the 60 s default is a literal, and origin lifetimes have no ceiling.
5. Tenancy is key isolation, not resource isolation.
6. Evidence is single-host, Windows, debug-profile for the suite, with no release-profile performance claim and no multi-host run.
7. Operations gaps: no metrics export, no config reload, no container image, no advisory scan.

Passing tests establish that an author's expectation held in a build on a host. They do not establish correctness (see Chapter 8, section 8.2.2). Reviews so far are AI second looks, not independent verification.
