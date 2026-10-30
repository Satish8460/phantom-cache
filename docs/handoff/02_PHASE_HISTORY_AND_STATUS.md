> Notice (2026-10-30): This handoff file is a historical record of the state on 2026-10-28 and is not edited. Parts of it are superseded: "Phase 4 accepted / zero P0 / formally accepted" and similar statements are not supported by the later audit ([handoff CORRECTIONS](CORRECTIONS.md), `CLAUDE.md` sections 6 and 6b); the test count "242 tests / 19 targets" is a 2026-10-28 baseline, not the current one ([docs/STATUS.md](../STATUS.md)); and descriptions of cluster cache sharing may predate the mTLS cluster path, where the local node shares the serving cache (`src/main.rs:332`).
>
> Original text below is unchanged.

# PhantomCache — Phase History, Audits, and Detailed Status

**Document ID:** `docs/handoff/02_PHASE_HISTORY_AND_STATUS.md`  
**Purpose:** Chronological record of architectural phases, milestones, audit findings, remediations, and authoritative test counts.  
**Audience:** Incoming engineers, auditors, and AI coding assistants.  
**Authoritative Baseline Date:** October 28, 2026  

---

## 1. Executive Summary & Authoritative Test Count Progression

The PhantomCache project evolved across four primary Work Packages (P1–P4) followed by distributed clustering enhancements structured into Phases 3A–3C and 4A–4C.

### Chronological Test Count Progression
1. **Initial Foundation (WP1–WP4):** 17 tests passed across 4 unit test files.
2. **Phase 1 & Phase 2 Hardening:** 92 tests passing across initial security and HTTP suites.
3. **Phase 3 Transport Completion:** 168 tests passing across mTLS, framing, and pooling targets.
4. **Phase 4 Pre-Audit Baseline:** Reported as **220 passing tests across 18 targets**.
5. **Stage 4C Remediation (SEC-4C-01..06):** Reconciled to **234 passing tests across 19 targets** (+6 Stage 4C regression tests, +7 `p2_dist_network_tests` inclusion, +1 Stage 4B overflow test).
6. **Gate B Production-Readiness Remediation (SEC-POA-01..06):** Current authoritative baseline is **242 passing tests across 19 targets** (100% pass rate, 0 failures, 0 ignored).

---

## 2. Phase-by-Phase Technical History

### Work Package P1: Core Engine, Request Coalescing, and SIEVE Eviction
- **Objective:** Build an in-memory HTTP cache eliminating cache stampedes with sub-millisecond lookups.
- **Key Files:** `src/core/cache.rs`, `src/core/coalesce.rs`, `tests/integration/m1_http_tests.rs`, `tests/unit/wp1_tests.rs`.
- **Architectural Decisions:**
  - 64-partition lock striping (`shards: Vec<Mutex<CacheShard>>`) using bitwise power-of-two masking.
  - Visited-Sweep (SIEVE) cache replacement algorithm maintaining single visited bits per entry.
  - Single-flight coalescer dispatching exactly one origin fetch per missing key; subsequent callers receive Tokio broadcast receivers.
  - RAII `FetchGuard` safety guaranteeing waiter notification if the initiator task panics or aborts.
- **Authoritative Tests:** 32 tests in `m1_http_tests.rs`, 5 tests in `wp1_tests.rs`, 5 tests in `coalesce_tradeoffs_tests.rs` (42 tests total).
- **Current Disposition:** **ACCEPTED**.

---

### Work Package P2: Upstream Egress & Anti-SSRF Protections
- **Objective:** Protect enterprise network boundaries from Server-Side Request Forgery (SSRF) and malformed upstream responses.
- **Key Files:** `src/security/upstream.rs`, `tests/integration/security_phase2_upstream_tests.rs`.
- **Architectural Decisions:**
  - Strict IP validation: blocks loopback (`127.0.0.0/8`), private (RFC 1918), link-local (`169.254.0.0/16`), and AWS/GCP cloud metadata IP (`169.254.169.254`).
  - Custom Root CA anchoring via `PHANTOM_UPSTREAM_CA_CERT` for enterprise egress mTLS.
  - Strict RFC 9112 chunked transfer decoder preventing HTTP request smuggling and chunk header overflows.
  - Header sanitization stripping sensitive client credentials (`Authorization`, `Cookie`) from upstream requests unless explicitly allowed.
- **Authoritative Tests:** 23 tests in `security_phase2_upstream_tests.rs`, 15 tests in `security_phase1_tests.rs` (38 tests total).
- **Current Disposition:** **ACCEPTED**.

---

### Work Package P3 & Work Package P4 (Original Baseline)
- **Objective:** Adaptive policy inference engine, discrete-event simulation, and telemetry.
- **Key Files:** `src/control/pie.rs`, `src/intel/telemetry.rs`, `src/sim/`, `tests/unit/wp2_tests.rs`, `tests/unit/wp3_tests.rs`, `tests/unit/wp4_tests.rs`, `tests/integration/p3_traffic_tests.rs`, `tests/integration/p4_pie_tests.rs`.
- **Architectural Decisions:**
  - Count-Min sketch estimating key access frequency under sub-linear memory overhead.
  - Dynamic switching between SIEVE, LRU, and S3FIFO with a mandatory 60-second cooldown period.
  - Coupled discrete-event simulation modeling network transit jitter and an M/M/1 queue ceiling ($15,000\text{ req/s}$).
- **Authoritative Tests:** 6 in `p4_pie_tests.rs`, 5 in `p3_traffic_tests.rs`, 13 across `wp2_tests`, `wp3_tests`, `wp4_tests` (24 tests total).
- **Current Disposition:** **ACCEPTED**.

---

### Phase 3A: Node Identity, Authoritative Membership, and Mutual TLS 1.3
- **Objective:** Enforce zero-trust cryptographic mutual authentication between all cluster nodes.
- **Key Files:** `src/security/cluster.rs`, `tests/integration/security_phase3_cluster_tests.rs`.
- **Architectural Decisions:**
  - Private Cluster Root CA anchoring. Plaintext TCP connections rejected immediately.
  - Mutual TLS 1.3 only; legacy TLS 1.0, 1.1, and 1.2 handshakes are rejected.
  - RFC 1035 SAN identity validation (`node-{id}.{cluster_id}.phantom.internal`).
  - Authoritative membership registry (`ClusterMembershipRegistry`) enforcing leaf certificate SHA-256 fingerprint verification and admission control.
- **Key Audit Finding & Remediation (SEC-3A-01):** SAN identity parser previously accepted multi-SAN certificates; remediated to enforce strict single-SAN pinning.
- **Authoritative Tests:** 24 tests in `security_phase3_cluster_tests.rs` (including 4 Gate B tests).
- **Current Disposition:** **ACCEPTED**.

---

### Phase 3B: Bounded Length-Prefixed Binary Framing
- **Objective:** Eliminate line-injection, buffer overruns, and delimiter ambiguity over inter-node TCP streams.
- **Key Files:** `src/dist/frame.rs`, `tests/integration/security_phase3b_framing_tests.rs`.
- **Architectural Decisions:**
  - 28-byte fixed header: Magic `'P' 'H' 'A' 'N'`, Version 1, Message Type (u8), Flags (u16 BE), Request ID (u64 BE), Sequence (u64 BE), Payload Length (u32 BE).
  - Strict payload bounds: maximum frame size 1,048,576 bytes (1 MiB); maximum key size 4,096 bytes (4 KiB).
  - Malformed magic, unsupported version, or oversized headers abort socket immediately.
- **Authoritative Tests:** 13 tests in `security_phase3b_framing_tests.rs`, 16 unit tests in `src/lib.rs`, 16 in `src/main.rs`.
- **Current Disposition:** **ACCEPTED**.

---

### Phase 3C: Bounded Connection Pooling, Heartbeats, and Active Session Revocation
- **Objective:** Manage inter-node TLS connections with reuse, bounded concurrency, and active revocation.
- **Key Files:** `src/dist/pool.rs`, `src/dist/session.rs`, `tests/integration/security_phase3c_pooling_tests.rs`.
- **Architectural Decisions:**
  - Bounded pool: `max_idle_per_node = 4`, `max_total_per_node = 8`.
  - Acquisition reservation permits prevent socket stampedes to offline peers.
  - Layer 5 active session revocation: `SessionTracker` maps active stream IDs; revoking a member in the registry immediately severs active sockets.
  - 5-second periodic keepalive heartbeats with 1,000ms timeout failover.
- **Authoritative Tests:** 11 tests in `security_phase3c_pooling_tests.rs`.
- **Current Disposition:** **ACCEPTED**.

---

### Phase 4A: Distributed Invalidation Codecs, Dispatching, and Generational Fencing
- **Objective:** Implement distributed invalidation wire framing and in-flight fetch fencing.
- **Key Files:** `src/core/cache.rs`, `src/dist/frame.rs`, `src/dist/node.rs`, `tests/integration/security_phase4a_invalidation_tests.rs`.
- **Architectural Decisions:**
  - Added message types `InvalidateRequest (0x03)` and `InvalidateAck (0x04)`.
  - Key-level generational fencing: invalidation increments `state.generation`, causing concurrent in-flight fetches to fail commit in `finalize_fetch`.
  - Invalidation scopes: `Scope::ExactKey (0x00)`, `Scope::Tenant (0x01)`, `Scope::FullCache (0x02)`.
  - Tenant generational epochs for $O(1)$ tenant-wide invalidation.
- **Key Audit Finding & Remediation (SEC-4A-01):** Capacity exhaustion in `tenant_epochs` (at 10,000 entries) previously risked stale entry resurrection; remediated by atomically advancing `global_epoch` and clearing `tenant_epochs` on capacity exhaustion.
- **Authoritative Tests:** 14 tests in `security_phase4a_invalidation_tests.rs`.
- **Current Disposition:** **ACCEPTED**.

---

### Phase 4B: Router Coordination, Acting-Primary Failover, and Replica Broadcast
- **Objective:** Coordinate cluster-wide invalidations via Rendezvous (HRW) routing, failover, and retries.
- **Key Files:** `src/dist/router.rs`, `src/api/server.rs`, `tests/integration/security_phase4b_router_tests.rs`.
- **Architectural Decisions:**
  - Synchronous canonical primary invalidation with strict 150ms deadline.
  - Acting-Primary Failover: unresponsive primary promoted to `Suspect`; top replica promoted to Acting Primary. Returns HTTP 204 with `X-Cache-Warning: PrimaryFailover`.
  - Asynchronous replica broadcast via `ReplicaBroadcaster` bounded queue (1,024 capacity) with 16-worker semaphore and exponential backoff retries.
  - Fail-closed HTTP gateway: returns `504 Gateway Timeout` (`X-Cache-Error: PrimaryInvalidationTimeout`) if neither primary nor acting primary can be purged. Zero misleading 204s.
- **Authoritative Tests:** 19 tests in `security_phase4b_router_tests.rs`.
- **Current Disposition:** **ACCEPTED**.

---

### Phase 4C: Bounded Anti-Entropy, Ring Buffers, and Digest Reconciliation
- **Objective:** Self-healing reconciliation recovering from packet loss, exhausted retry queues, and partitions.
- **Key Files:** `src/dist/anti_entropy.rs`, `src/dist/node.rs`, `src/dist/router.rs`, `tests/integration/security_phase4c_antientropy_tests.rs`.
- **Architectural Decisions:**
  - Added message types `AntiEntropySyncRequest (0x07)` and `AntiEntropySyncResponse (0x08)`.
  - Dual-bounded `InvalidationRingBuffer`: 4,096 entries and 4.0 MiB payload bytes hard ceiling.
  - Cryptographic state digest (SHA-256 over sequence window) enabling $O(1)$ equality verification with zero data transfer when in sync.
  - Monotonic `AntiEntropyTracker` detecting stream sequence gaps.
  - Deterministic partition flush (`flush_for_anti_entropy_overwrite`) when sequence overwrite occurs (`seq_start < oldest_seq`), advancing global epoch and clearing cache.
- **Audit Findings Remediated (SEC-4C-01..06):**
  - Atomic stream sequence allocation independent of transport connection sequence (`SEC-4C-01`).
  - Strict membership epoch and peer identity validation (`SEC-4C-02`).
  - Empty ring buffer HistoryOverwritten guard (`SEC-4C-03`).
  - Accurate range end on truncated replay batches (`SEC-4C-04`).
  - Monotonic ACK tracker preventing sequence rewinds (`SEC-4C-05`).
  - Autonomous reconciliation probe in router runtime (`SEC-4C-06`).
- **Authoritative Tests:** 18 tests in `security_phase4c_antientropy_tests.rs` (including 4 Gate B tests).
- **Current Disposition:** **ACCEPTED**.

---

### Gate B Production-Readiness Remediation (October 28, 2026)
- **Objective:** Resolve confirmed operational and deployment blockers (SEC-POA-01 through SEC-POA-06).
- **Remediations Implemented & Verified:**
  1. **SEC-POA-01 (Cluster Runtime Wiring):** Wired `main.rs` to validate `PHANTOM_CLUSTER_LISTEN_ADDR`, resolve peer addresses from env or `cluster-members.json`, detect address conflicts, bind listener, and preserve clean standalone mode. Verified by Tests 21–24 in `security_phase3_cluster_tests.rs`.
  2. **SEC-POA-02 & SEC-POA-04 (Safe Node Recovery State Machine & Read Bypass):** Nodes in `Suspect` / `Recovering` state bypass local cache hits and return `MISS` to origin on inbound `GetRequest`. Single-probe atomic reconciliation only promotes nodes to `Active` upon verified `InSync` or safe flush `HistoryOverwritten`. Verified by Tests 15–17 in `security_phase4c_antientropy_tests.rs`.
  3. **SEC-POA-03 (Exact Resident Memory Accounting):** Added `estimated_resident_memory_bytes()` proving resident heap memory is strictly bounded to $\le 4.25\text{ MiB}$ ($\le 4,456,448\text{ bytes}$) with transient peak serialization bounded to $\le 6.5\text{ MiB}$. Verified by Test 18 in `security_phase4c_antientropy_tests.rs`.
  4. **SEC-POA-05 & SEC-POA-06 (Documentation Accuracy):** Labeled loopback micro-benchmarks explicitly in `docs/security/phase-4-design.md`; clarified DELETE 204 eventual consistency and partition boundaries.
- **Final Test Count:** **242 tests passed across 19 targets**.

