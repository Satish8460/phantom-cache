# Stage 3C Implementation Progress: Connection Pooling & Active Session Lifecycle

**Module:** PhantomCache Inter-Node Transport  
**Phase:** Stage 3C (Connection Pooling, Keepalive Heartbeats, SessionTracker Active Eviction)  
**Status:** Remediated & Verified (Ready for Final Audit)  
**Date:** October 2026  

---

## 1. Objectives & Scope

Stage 3C completes the Phase 3 cluster inter-node transport security boundary by replacing per-lookup ephemeral connections with a bounded, reusable, mutually authenticated connection pool, implementing real background keepalive heartbeats, and enforcing Layer 5 active session revocation via `SessionTracker`.

### In-Scope Deliverables
1. **Request-ID Correlation & Validation:**
   - Router enforces strict validation: response `request_id` must match the originating `request_id`.
   - Mismatches immediately invalidate and discard the connection to prevent cross-talk.
2. **Bounded TLS Connection Pool (`TlsConnectionPool`):**
   - Enforces `max_idle_per_node = 4`, `max_total_per_node = 8`, and `idle_timeout = 30s`.
   - Atomic checkout and capacity reservation via RAII `ConnectionReservation`.
   - Hard capacity enforcement before network connect or TLS handshake.
   - Zero plaintext fallback: preserves TLS 1.3, private CA anchoring, and membership authorization.
3. **Periodic Keepalive Heartbeats (5-Second Interval):**
   - Lifecycle-managed background worker probing idle pooled connections at `heartbeat_interval`.
   - Active heartbeat probes (`MessageType::HeartbeatRequest (0x05)` / `MessageType::HeartbeatResponse (0x06)`).
   - Detection and automatic eviction of dead or stalled peer connections.
4. **Layer 5 Active Session Eviction (`SessionTracker`):**
   - Inbound sessions registered in `SessionTracker` on `VirtualNode`.
   - Automated wiring: `ClusterMembershipRegistry` broadcasts member revocation events, prompting `VirtualNode` to immediately sever active peer sockets without waiting for subsequent requests.
5. **Eviction Epoch Invalidation:**
   - Epoch-based tracking (`node_epochs`) ensures that connections checked out before a node eviction are recognized as stale and dropped upon checkin.
6. **Per-Request Admission on Reused Connections:**
   - On reused pooled connections, credentials and cluster registry membership are re-validated on every frame as defense-in-depth.

### Explicit Non-Goals (Deferred Workstreams)
- **Distributed Cache Invalidation:** Multi-node invalidation broadcasts, quorum writes ($W = \lfloor N/2 \rfloor + 1$), and vector clock reconciliation belong to the separate Distributed Operations workstream.
- **Durable Cross-Restart Replay Protection:** Receiver-side persistent mutation journals and originator-epoch persistence are deferred to Distributed Operations.

---

## 2. Remediation of Audit Findings (SEC-3C-01 to SEC-3C-05)

### SEC-3C-01: Connection Limit Enforcement & Atomic Reservation
- **Problem:** Split `checkout()` and `try_increment_active()` allowed `router.rs` to ignore capacity results and establish unbounded TCP connections beyond 8.
- **Remediation:** Replaced with atomic `checkout_or_reserve(node_id) -> AcquisitionResult`. If capacity exists, `active_counts` is incremented atomically under lock and returned in an RAII `ConnectionReservation`. If dropped on connect/handshake failure, its `Drop` implementation safely decrements `active_counts`. If `AcquisitionResult::Full`, `router.rs` fails over immediately to the next replica candidate without opening excess sockets.
- **Verification:** Verified by `test_p3c_sec01_concurrency_capacity_cap_and_no_leak` with 16 simultaneous requests to a single node. Active connections never exceeded 8, excess requests failed over, and counters did not leak or underflow.

### SEC-3C-02: Lifecycle-Managed Periodic Heartbeat Scheduler
- **Problem:** Only a synchronous `send_heartbeat()` method existed; no background task probed idle connections.
- **Remediation:** Implemented `start_heartbeat_worker()` and `stop_heartbeat_worker()`. A background task ticks at `heartbeat_interval`, extracts idle connections, issues `send_heartbeat()`, and discards dead sockets. Synchronous callers outside Tokio (such as `wp3_tests`) safely bypass worker spawning via `Handle::try_current()`.
- **Verification:** Verified by `test_p3c_sec02_periodic_heartbeat_scheduler_background_probing`. An idle connection was probed automatically in the background (sequence number incremented), and when the peer was shut down, the worker detected the broken socket and evicted it from the pool.

### SEC-3C-03: Automated Membership Revocation Wiring
- **Problem:** `evict_peer()` was not invoked when `ClusterMembershipRegistry` was updated.
- **Remediation:** Added `revocation_tx: broadcast::Sender<u8>` to `ClusterMembershipRegistry`. When `add_member()` sets `active = false` or `update()` revokes members, the revoked `node_id` is broadcast. `VirtualNode::start_tcp_service` subscribes to this channel and triggers `evict_peer()` immediately, interrupting blocked socket reads.
- **Verification:** Verified by `test_p3c_sec03_automated_membership_revocation_wiring`. Updating `registry` severed an idle mTLS connection without test code calling `evict_peer()`.

### SEC-3C-04: Eviction-Aware Checkin via Epochs
- **Problem:** Connections checked out before `evict_node()` were re-added to the idle pool upon `checkin()`.
- **Remediation:** Added `node_epochs: HashMap<u8, u64>`. When `evict_node()` is called, the node's epoch increments. When `checkin(conn)` runs, if `conn.epoch != current_epoch`, the connection is dropped and decremented from active counts.
- **Verification:** Verified by `test_p3c_sec04_evicted_connection_dropped_on_checkin`. A checked-out connection from an older epoch was dropped upon checkin and not re-pooled.

### SEC-3C-05: Clippy & Whitespace Hygiene
- **Problem:** Clippy errors on `ClusterStream` (`large_enum_variant`) and `SessionTracker` (`type_complexity`); trailing blank line at EOF in `src/lib.rs`.
- **Remediation:** Boxed `TlsStream` in `ClusterStream::Tls(Box<...>)`, introduced `type SessionMap`, and removed the trailing blank line in `src/lib.rs`.
- **Verification:** `cargo clippy --all-targets -j 1 -- -D warnings` passed with zero errors and zero warnings; `git diff --check` passed cleanly.

---

## 3. Verification & Test Evidence

### Stage 3C Integration Suite (`security_phase3c_pooling_tests`)
Executed sequentially with `-j 1`:
```
running 11 tests
test test_p3c_active_session_eviction_on_peer_revocation ... ok
test test_p3c_connection_reuse_across_sequential_requests ... ok
test test_p3c_sec04_evicted_connection_dropped_on_checkin ... ok
test test_p3c_request_id_mismatch_discard_and_failover ... ok
test test_p3c_concurrent_requests_pool_isolation ... ok
test test_p3c_sec01_concurrency_capacity_cap_and_no_leak ... ok
test test_p3c_heartbeat_ping_pong ... ok
test test_p3c_per_request_revocation_on_reused_connection ... ok
test test_p3c_sec03_automated_membership_revocation_wiring ... ok
test test_p3c_heartbeat_failure_eviction ... ok
test test_p3c_sec02_periodic_heartbeat_scheduler_background_probing ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
```

### Full Repository Regression Suite
| Test Target | Scope | Result |
| :--- | :--- | :--- |
| `security_phase3c_pooling_tests` | Stage 3C pooling, correlation, heartbeats, eviction | **11 passed; 0 failed** |
| `security_phase3b_framing_tests` | Stage 3B binary framing, parser limits, mTLS framing | **13 passed; 0 failed** |
| `security_phase3_cluster_tests` | Stage 3A mTLS, SAN identity, CA rollover, membership | **20 passed; 0 failed** |
| `security_phase2_upstream_tests` | Phase 2 SSRF, upstream TLS, HTTP header parser | **23 passed; 0 failed** |
| `security_phase1_tests` | Phase 1 TLS, JWT RBAC, RFC 9111 tenant isolation | **15 passed; 0 failed** |
| `p2_dist_network_tests` | Distributed routing, socket timeouts, replica failover | **7 passed; 0 failed** |
| `coalesce_tradeoffs_tests` | Concurrency fairness, thundering-herd blast radius | **5 passed; 0 failed** |
| `m1_http_tests` | HTTP cache engine, SIEVE eviction, coalescing | **32 passed; 0 failed** |
| `p3_traffic_tests` & `p4_pie_tests` | Traffic generator, PIE controller, adaptive eviction | **11 passed; 0 failed** |
| `wp1_tests` through `wp4_tests` | Core unit suites (coalescer, sketch, router, SSE API) | **18 passed; 0 failed** |
| `cargo test --lib` | Library unit tests (framing, session tracker) | **8 passed; 0 failed** |
| **Total Test Count** | **Repository-wide verification** | **163 passed; 0 failed** |

---

## 4. Acceptance Status
- **Independent Acceptance Audit:** Passed with `ACCEPT` across all five findings SEC-3C-01 through SEC-3C-05.
- **Verification Baseline:** 163 passing tests across 11 suites; zero Clippy warnings (`-D warnings`).

---

## 5. Phase 4 Milestone Transition: Distributed Invalidation & Consistency
Following the acceptance of Stage 3C, the architecture and design investigation for **Phase 4: Distributed Cache Invalidation, Monotonic Consistency, and Anti-Entropy Reconciliation** was executed.

### Core Architectural Decisions Documented
1. **Design Specification:** Published comprehensive architecture specification in [`docs/security/phase-4-design.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-design.md).
2. **Consistency Contract:** Established a Two-Tier Invalidation Model:
   - **Tier 1 (Synchronous Canonical Invalidation):** Gateway invalidates locally and synchronously across mTLS to the rendezvous primary owner, providing immediate read-your-own-writes consistency ($T_{\text{stale}} = 0$) on the canonical routing path.
   - **Tier 2 (Asynchronous Bounded Broadcast & Retries):** Primary broadcasts `InvalidateRequest (0x03)` to secondary replicas over pooled connections with bounded retries and an in-memory ring buffer.
3. **In-Flight Fencing:** Key generation counters in [`src/core/cache.rs`](file:///d:/Personal/phantom-cache/src/core/cache.rs) fence in-flight origin fetches to prevent stale commits from resurrecting evicted keys.
4. **Anti-Entropy Reconciliation:** Periodic keepalive heartbeats (5s) in [`src/dist/pool.rs`](file:///d:/Personal/phantom-cache/src/dist/pool.rs) are extended with digest exchanges (`0x07`/`0x08`) to heal missed invalidations without persistent WAL overhead.
5. **Implementation Staging:** Defined three self-contained work packages (Stage 4A: Codecs & Fencing; Stage 4B: Router Coordination & Retries; Stage 4C: Anti-Entropy & Telemetry) accompanied by 10 deterministic integration test specifications (AT-P4-01 through AT-P4-10).

### 5.1 Pre-Implementation Design Review Remediation
Prior to initiating Stage 4A implementation, a comprehensive correctness review was conducted across six architectural areas:
1. **HTTP Contract & Fail-Closed 504 Semantics:** Defined exact guarantees of `204 No Content` (local + canonical primary invalidated). Resolved timeout fallback contradiction: if primary and acting-primary replicas fail to acknowledge within 150ms, gateway aborts and returns `504 Gateway Timeout` with `X-Cache-Error: PrimaryInvalidationTimeout`.
2. **Stream Sequences vs Key Versions:** Separated outbound node stream sequences (`stream_seq`) used for peer anti-entropy gap detection from local key generation counters (`generation`) used for in-flight fetch fencing. Obsolete filtering strictly never suppresses distinct keys.
3. **Dual-Bounded Anti-Entropy Budget:** Performed exact mathematical worst-case memory calculations showing 4,096 max-size (4 KB) keys consume $\approx 16.32\text{ MB}$. Enforced a hard dual ceiling: `MAX_RING_ENTRIES = 4,096` and `MAX_RING_BYTES = 4,194,304` (4 MB). Clarified that digests detect mismatch while overwritten gaps trigger safe full partition cache flushes (`cache.clear()`).
4. **Fetch Fencing Boundary & SLO Separation:** Documented atomic commit boundary under shard lock. Separated safety invariants from operational SLOs ($P_{99} \le 20\text{ ms}$; 15-second partition suspect limit).
5. **$O(1)$ Tenant-Wide Invalidation:** Introduced `tenant_epochs` in `PartitionedCache` allowing instant tenant purges without table scans, maintaining strict JWT cryptographic tenant isolation.
6. **Framing & Stage 3B/3C Compatibility:** Verified wire layouts against 28-byte `FrameHeader`, enforced payload boundaries ($\le 4,116\text{ bytes}$ for `InvalidateRequest`, exactly 24 bytes for `InvalidateAck`), and confirmed integration with `TlsConnectionPool` and `SessionTracker`.

### 5.2 Gate 0 Authorization & Final Failure Contract Decision
- **Authoritative Decision:** The HTTP invalidation failure contract is strictly **fail-closed**.
- **Rule:** `DELETE /api/v1/cache/*key` returns `204 No Content` IF AND ONLY IF canonical primary (or acting-primary failover) invalidation is acknowledged over authenticated transport.
- **Fail-Closed Abort:** If canonical primary invalidation cannot be acknowledged within 150 ms and no acting primary replica can be reached, the gateway returns `504 Gateway Timeout` with header `X-Cache-Error: PrimaryInvalidationTimeout`.
- **Prohibition:** Returning `204 No Content` on unacknowledged local-only fallback is explicitly prohibited across the system.

### 5.3 Stage 4A Implementation & Initial Verification Evidence
- **Scope Implemented:**
  1. Bounded `InvalidateRequest` and `InvalidateAck` wire codecs in `src/dist/frame.rs` ($\le 4,116\text{ B}$ request, 24B ack).
  2. Bounded dispatch handler in `src/dist/node.rs:process_binary_cache_stream` over authenticated mTLS.
  3. Local cache invalidation (`delete_canonical`, `delete_with_status`) in `src/core/cache.rs`.
  4. Per-key generational in-flight fetch fencing preventing stale resurrection.
  5. $O(1)$ tenant-epoch invalidation (`tenant_epochs` bounded to 10,000 entries) and global cache purge.
- **Initial Verification:**
  - `security_phase4a_invalidation_tests`: 8 passed; 0 failed.
  - `cargo test --lib`: 16 passed; 0 failed (including 8 new framing codec tests).
  - Clippy check: passed with zero warnings.

### 5.4 Stage 4A Independent Audit & Remediation of Finding SEC-4A-01
- **Audit Outcome:** Independent acceptance audit returned `ACCEPT WITH CONDITIONS` identifying SEC-4A-01 (arbitrary eviction in `tenant_epochs` at 10,000 capacity cap resetting tenant epoch to 0, risking resurrection of pre-invalidation cached entries).
- **Remediation Implemented in `src/core/cache.rs`:**
  1. **Synchronized Global-Epoch Fallback:** When `tenant_epochs` reaches 10,000 entries and a new tenant invalidation arrives, `global_epoch` increments via checked arithmetic and `tenant_epochs` clears. Because every pre-existing entry has `entry_global_epoch < current_global_epoch`, all pre-existing entries cluster-wide are immediately fenced, preventing any stale entry resurrection.
  2. **Atomic Epoch Snapshot:** Added `get_effective_epochs(&self, tenant_id: &str) -> (u64, u64)` capturing both tenant and global epochs under `tenant_epochs.lock()`, preventing split-snapshot races.
  3. **Checked Arithmetic:** Switched all epoch and generation increments to `checked_add(1).expect(...)` to eliminate counter wrap-around vulnerabilities.
- **Focused Test Suite Additions:**
  - Added Tests 9–14 in `tests/integration/security_phase4a_invalidation_tests.rs`:
    - Test 9: Regression test verifying that pre-invalidation cached entries are never resurrected after capacity exhaustion.
    - Test 10: Invalidation of $> 10,000$ distinct tenants confirming strict memory ceiling ($\le 10,000$) and valid post-exhaustion tenant invalidations.
    - Test 11: In-flight origin fetch fencing during capacity fallback.
    - Test 12: Tenant isolation under capacity pressure.
    - Test 13: Repeated capacity exhaustion cycles ($10,000 \to \text{fallback} \to 10,000$) and `invalidate_all()` interactions.
    - Test 14: Checked arithmetic overflow verification.
- **Verification Results:**
  - `cargo test --test security_phase4a_invalidation_tests -j 1`: 14 passed; 0 failed.
  - `cargo test --lib -j 1`: 16 passed; 0 failed.
  - `cargo test --test security_phase3c_pooling_tests -j 1`: 11 passed; 0 failed.
  - `cargo clippy --all-targets -j 1 -- -D warnings`: passed with zero warnings.
- **Status:** SEC-4A-01 Remediated and Verified. Ready for secondary independent acceptance audit.





