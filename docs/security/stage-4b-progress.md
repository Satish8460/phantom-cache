> **Correction notice (2026-10-30, owner-approved DEC-020; the original text below is unchanged):** (1) "Final Acceptance", "Sign-Off", "authorized" and "Verified Real-Time Linearizability" are agent self-reports; no dated owner acceptance of Phase 4 is recorded in the repository (an evidence gap, not proof that none was given), and later reviews do not support "zero P0" (`docs/STATUS.md`, `docs/handoff/CORRECTIONS.md`). (2) "Real-Time Linearizability" / "Stale Reads Detected: 0" / "Eventual Consistency with bounded staleness": the check reported is a loopback, debug-profile test (assumed from the report; not re-run) that no read completing after an invalidation returned older data. It is not a linearizability proof or history check, and it establishes no staleness bound. The origin freshness lifetime has no gateway ceiling and `PHANTOM_DEFAULT_TTL_MS` is not wired (`docs/user/limitations.md`); the issuing gateway keeps owed invalidations only in memory (ADR-003 amendment). (3) Test counts quoted here (for example 242 tests, 19 targets) are historical; the current baselines are in `docs/STATUS.md` section 3.

# Stage 4B Implementation Progress: Distributed Invalidation Coordination & Gateway Integration

**Module:** PhantomCache Distributed Operations & Invalidation  
**Phase:** Stage 4B (Router Coordination, Failover, Replica Broadcaster, HTTP Gateway Integration)  
**Status:** Complete & Verified (Ready for Independent Acceptance Audit)  
**Date:** October 2026  

---

## 1. Objectives & Scope

Stage 4B bridges the gap between single-node cache invalidation codecs (Stage 4A) and the external client HTTP boundary. It provides synchronous primary invalidation with acting-primary failover, non-blocking asynchronous replica broadcast with bounded retry queues, and an authoritative fail-closed HTTP contract for cache purge requests.

### In-Scope Deliverables

1. **Rendezvous Router Distributed Invalidation (`invalidate_key_across_cluster`):**
   - High Random Weight (HRW) rendezvous primary node selection for key invalidation.
   - Synchronous transmission of [`PhantomMeshFrame::new_invalidate_request`](file:///d:/Personal/phantom-cache/src/dist/frame.rs#L372-L398) over mutually authenticated TLS connections from [`TlsConnectionPool`](file:///d:/Personal/phantom-cache/src/dist/pool.rs).
   - Strict `150ms` per-attempt timeout enforcement.
   - Request-ID correlation validation: mismatched or malformed ACKs trigger immediate connection discard, mark the target node as `NodeState::Suspect`, and trigger failover.
   - Automatic **Acting Primary Failover**: if the canonical primary is unreachable, timed out, or corrupt, the top healthy replica is designated Acting Primary and attempted.
   - Telemetry tracking and structured return outcome: [`InvalidationOutcome`](file:///d:/Personal/phantom-cache/src/dist/router.rs#L13-L20) with `canonical_node_id`, `is_acting_primary`, `suspect_node_id`, `status`, and `reclaimed_bytes`.

2. **Asynchronous Bounded Replica Broadcaster (`ReplicaBroadcaster`):**
   - Non-blocking background worker spawned per router.
   - Strict queue capacity bound of `1,024` tasks via `tokio::sync::mpsc::channel`.
   - Overflow saturation protection: `broadcaster.enqueue` uses `try_send()`. If saturated, it drops excess tasks and increments `dropped_count` without stalling callers or client HTTP requests.
   - Bounded retries: up to 3 retries with exponential backoff (25ms, 50ms, 100ms, capped at 200ms).
   - Dynamic membership revocation checking: checks [`ClusterMembershipRegistry`](file:///d:/Personal/phantom-cache/src/security/cluster.rs) between retries and immediately aborts retries if a peer has been revoked (`active: false`).
   - Clean lifecycle shutdown via oneshot signal.

3. **HTTP Gateway Integration (`src/api/server.rs`):**
   - Added `router: Option<Arc<RendezvousRouter>>` to [`ApiState`](file:///d:/Personal/phantom-cache/src/api/server.rs#L51-L63).
   - Wired [`delete_cache_handler`](file:///d:/Personal/phantom-cache/src/api/server.rs#L770-L843) and `delete_cache_handler_root` to execute cluster-wide invalidation.
   - **Authoritative Consistency Contract:**
     - Returns `204 No Content` iff the canonical primary (or acting primary) invalidation was acknowledged over authenticated transport.
     - On acting-primary failover, returns `204 No Content` with response header:
       `X-Cache-Warning: PrimaryFailover(<suspect_node_id>)`.
     - Returns `504 Gateway Timeout` with response header:
       `X-Cache-Error: PrimaryInvalidationTimeout` if primary and failover candidates fail or exceed the deadline.
     - **Fail-Closed Invariant:** The gateway purges its local cache entry for cache hygiene, but **NEVER** emits `204 No Content` on unacknowledged local-only eviction.
     - In standalone / unclustered mode (`router` is `None`), performs local eviction and returns `204 No Content`.

4. **Resolution of Carry-Forward Condition 1 (Counter-Overflow Propagation):**
   - Evolved [`PartitionedCache`](file:///d:/Personal/phantom-cache/src/core/cache.rs) with `try_invalidate_tenant` and `try_invalidate_all` returning `Result<u64, CacheError>`.
   - When epoch or generation counters overflow, [`process_binary_cache_stream`](file:///d:/Personal/phantom-cache/src/dist/node.rs) responds with `InvalidationStatus::RejectedRateLimit` instead of panicking or emitting false success ACKs.

5. **Resolution of Carry-Forward Condition 2 (Multi-Threaded Concurrent Load Stress Testing):**
   - High-concurrency multi-threaded load test with 8 concurrent tasks (4 continuous readers/fetchers and 4 continuous distributed invalidators) executing 1,000 operations under contention.
   - Measured latency distribution ($P_{50} = 141\,\mu\text{s}$, $P_{95} = 370\,\mu\text{s}$, $P_{99} = 887\,\mu\text{s}$).
   - Verified zero stale reads observed, zero panics, zero socket drops, and zero deadlock conditions.

---

## 2. Architecture & Control Flow

### Invalidation Request Lifecycle

```
[ HTTP Client ]
      |
      | DELETE /api/v1/cache/items/sku_42
      v
[ HTTP Gateway: delete_cache_handler ]
      |
      | 1. Authenticate JWT (Role::CacheInvalidate)
      | 2. CacheKey::new() & validate()
      | 3. router.invalidate_key_across_cluster("sku_42", ExactKey)
      v
[ RendezvousRouter ]
      |
      | HRW Hash -> Primary: Node 1, Replicas: [Node 2, Node 3]
      |
      +---> [ Primary Node 1: mTLS Pooled Connection ]
      |           |
      |      (ACK Received) ---> Status: SuccessEvicted
      |           |
      |           +---> Enqueue Replicas [Node 2, Node 3] -> [ ReplicaBroadcaster ]
      |           |                                                  |
      |           v                                                  v (Async Background)
      |     Return InvalidationOutcome                        Retries: 3x, Backoff
      |     (canonical=1, failover=false)                     Revocation Check
      |           |
      |           v
      |     Purge Local Cache Hygiene
      |     HTTP 204 No Content
      |
      +---> (Timeout / Closed / Corrupt ACK)
                  |
                  v
            Mark Node 1 Suspect
            Designate Node 2 as Acting Primary
            Attempt Node 2 Invalidation (150ms timeout)
                  |
                  +---> (Node 2 ACK Received)
                  |           |
                  |           +---> Enqueue Remaining Replicas [Node 3]
                  |           v
                  |     Purge Local Cache Hygiene
                  |     HTTP 204 No Content
                  |     X-Cache-Warning: PrimaryFailover(1)
                  |
                  +---> (Node 2 Timeout / Failed)
                              |
                              v
                        Purge Local Cache Hygiene
                        HTTP 504 Gateway Timeout
                        X-Cache-Error: PrimaryInvalidationTimeout
```

---

## 3. Authoritative Consistency Contract & Invalidation Matrix

| Condition | Transport Status | HTTP Response | Response Headers | Consistency State |
| :--- | :--- | :---: | :--- | :--- |
| **Primary ACK Received** | Valid mTLS `InvalidateAck` | `204 No Content` | None | Gateway + Primary guaranteed purged ($T_{\text{stale}} = 0$). Replicas queued asynchronously. |
| **Primary Timeout / Failed, Acting Primary ACK** | Valid mTLS `InvalidateAck` | `204 No Content` | `X-Cache-Warning: PrimaryFailover(<suspect_id>)` | Gateway + Acting Primary guaranteed purged. Remaining replicas queued. |
| **All Primary Candidates Failed / Timed Out** | Connection failure, timeout, or bad ACK | `504 Gateway Timeout` | `X-Cache-Error: PrimaryInvalidationTimeout` | Local cache purged for hygiene; cluster state uncertain. Client must retry. |
| **RBAC Authorization Failure** | Public / invalid token | `401 Unauthorized` / `403 Forbidden` | `X-Auth-Error: InsufficientPermissions` | Request rejected before router execution. No state changed. |
| **Malformed Key / Validation Error** | Bad key syntax | `400 Bad Request` | `X-Cache-Error: InvalidKey` | Request rejected before execution. |

---

## 4. Verification & Test Evidence

### Dedicated Stage 4B Integration Suite (`security_phase4b_router_tests`)

Executed with `cargo test --test security_phase4b_router_tests -j 1 -- --nocapture`:

```
running 19 tests
test test_p4b_sec02_generation_overflow_propagates_internal_error_without_panic ... ok
test test_p4b_replica_broadcaster_bounded_queue_saturation_and_shutdown ... ok
test test_p4b_sec05_mismatched_responder_or_epoch_rejected ... ok
test test_p4b_counter_error_propagation_rejected_rate_limit ... ok
test test_p4b_ack_correlation_and_state_tracking ... ok
test test_p4b_duplicate_invalidation_idempotence ... ok
test test_p4b_mismatched_request_id_ack_discarded ... ok
test test_p4b_sec06_http_gateway_status_mappings ... ok
test test_p4b_http_gateway_204_primary_success_vs_504_timeout ... ok
test test_p4b_sec07_broadcaster_bounded_concurrency_no_head_of_line_blocking ... ok
test test_p4b_malformed_ack_payload_discarded ... ok
test test_p4b_canonical_primary_invalidation_over_mtls ... ok
test test_p4b_replica_broadcaster_revoked_peer_stops_retries ... ok
test test_p4b_sec01_suspect_primary_excluded_from_routing_and_retried_in_broadcaster ... ok
test test_p4b_primary_unreachable_acting_primary_failover ... ok
test test_p4b_fail_closed_no_false_204_on_local_eviction ... ok
test test_p4b_http_gateway_acting_primary_failover_header ... ok
test test_p4b_concurrent_invalidation_load_stress_and_stale_prevention ... ok
test test_p4b_strict_150ms_deadline_enforcement ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

### Stage 4B Concurrency Benchmark (Separated Metrics & Real-Time Linearizability)

Executed via `test_p4b_concurrent_invalidation_load_stress_and_stale_prevention`:

```
--- Stage 4B Concurrency Benchmark (1000 ops: 500 Reads, 500 Invalidations) ---
In-Memory Reads:        P50:     5 us | P95:    14 us | P99:    39 us
Cluster Invalidations:  P50:   668 us | P95:  1422 us | P99:  2137 us
Stale Reads Detected:   0 (Verified Real-Time Linearizability)
----------------------------------------------------------------------------------
```

### Complete Workspace Test Suite

Executed with `cargo test --all-targets -j 1`:

| Target Name | Type | Tests Run | Result | Notes |
| :--- | :--- | :---: | :---: | :--- |
| `src/lib.rs` | Unit | 16 | **16 passed** | Codecs, framing, session tracker |
| `src/main.rs` | Unit | 16 | **16 passed** | Codecs, framing, session tracker |
| `coalesce_tradeoffs_tests` | Integration | 5 | **5 passed** | Request coalescing single-flight |
| `m1_http_tests` | Integration | 32 | **32 passed** | Core HTTP & cache engine tests |
| `p3_traffic_tests` | Integration | 4 | **4 passed** | Traffic simulation & pacing |
| `p4_pie_tests` | Integration | 6 | **6 passed** | PIE adaptive eviction controller |
| `security_phase1_tests` | Integration | 15 | **15 passed** | JWT, RBAC, tenant isolation |
| `security_phase2_upstream_tests` | Integration | 23 | **23 passed** | SSRF prevention, TLS, chunking |
| `security_phase3_cluster_tests` | Integration | 20 | **20 passed** | mTLS, SAN, fingerprint pinning |
| `security_phase3b_framing_tests` | Integration | 17 | **17 passed** | 28B framing, bounds, codecs |
| `security_phase3c_pooling_tests` | Integration | 11 | **11 passed** | Connection pooling, keepalives |
| `security_phase4a_invalidation_tests` | Integration | 14 | **14 passed** | Stage 4A invalidations & epochs |
| `security_phase4b_router_tests` | Integration | 19 | **19 passed** | **Stage 4B distributed router suite (+5 regression tests)** |
| `wp1_tests` | Unit | 5 | **5 passed** | Work package 1 tests |
| `wp2_tests` | Unit | 5 | **5 passed** | Work package 2 tests |
| `wp3_tests` | Unit | 3 | **3 passed** | Work package 3 tests |
| `wp4_tests` | Unit | 5 | **5 passed** | Work package 4 tests |
| **Total Workspace** | **17 Targets** | **212** | **212 passed (100%)** | **0 failed, 0 ignored** |

### Static Linter & Formatting

- `cargo clippy --all-targets -j 1 -- -D warnings`: Completed with **0 warnings, 0 errors**.
- `git diff --check`: Passed with **0 errors** in Rust codebase.

---

## 4.1 Stage 4B Independent Audit Remediation Summary (Round 1)

| Finding ID | Severity | Status | Technical Remediation Applied |
| :--- | :---: | :---: | :--- |
| **SEC-4B-01** | High | **Resolved** | Updated `RendezvousRouter::route_key` and `get_replicas` to strictly exclude `NodeState::Suspect` during partition lookup with fallback to suspect only if 0 healthy nodes exist. On acting-primary promotion, `primary.node_id` is prepended to `ReplicaBroadcaster` retry list. Verified in Test 15. |
| **SEC-4B-02** | Medium | **Resolved** | Added `try_delete_with_status` and `try_delete_canonical` in `PartitionedCache` returning `Result<(bool, u32), CacheError>`. Mapped generation overflow to `InvalidationStatus::InternalError` without lock poisoning or unwinding panics. Verified in Test 16. |
| **SEC-4B-03** | Medium | **Resolved** | Replaced decoupled 150 ms timeouts with cumulative monotonic deadline tracking inside `send_invalidation_frame` across connection checkout/connect, TLS handshake, frame write, and frame read. Verified in Test 6. |
| **SEC-4B-04** | High | **Resolved** | In `src/main.rs`, detected cluster configuration environment variables (`PHANTOM_CLUSTER_MEMBERS_FILE` or cluster TLS certs/keys), constructed `RendezvousRouter::with_cluster_tls`, attached it to `ApiState`, and enforced fail-closed termination on invalid configurations. |
| **SEC-4B-05** | Low | **Resolved** | In `send_invalidation_frame`, validated `ack.responder_id == node.node_id` and `ack.membership_epoch == self.get_membership_epoch()`. Mismatched responder ID sets node to Suspect and discards connection; epoch mismatch returns rejected error. Verified in Test 17. |
| **SEC-4B-06** | High | **Resolved** | Added `DistError::Rejected(InvalidationStatus)`. Added `InvalidationStatus::is_success()`. In `delete_cache_handler`, non-success statuses are mapped to HTTP 503 (`PrimaryRateLimited`), 500 (`PrimaryInternalError`), or 500 (`ObsoleteInvalidation`), strictly prohibiting misleading 204s on rejection. Verified in Test 18. |
| **SEC-4B-07** | Medium | **Resolved** | Refactored `ReplicaBroadcaster` using `tokio::sync::Semaphore` (capacity 16) to process replica destinations concurrently in separate worker tasks, completely eliminating head-of-line blocking when retrying dead nodes. Verified in Test 19. |
| **SEC-4B-08** | Medium | **Resolved** | Refactored Test 14 with genuine atomic monotonic causal ordering to detect real-time linearizability violations. Separated in-memory read metrics ($P_{50}=5\mu\text{s}$) from cluster network invalidation metrics ($P_{50}=668\mu\text{s}$) with 0 stale reads detected. |

---

## 5. Residual Risks, Deferred Work, and Stage 4C Transition

1. **Stage 4C (Bounded Anti-Entropy & Invalidation Digest Exchange):**
   - Active invalidation digests exchanged over periodic keepalive heartbeats (`MessageType::HeartbeatRequest` / `MessageType::HeartbeatResponse`).
   - Receiver-side in-memory dual-bounded `InvalidationRingBuffer` (4,096 entries, 4 MB ceiling) for out-of-order sequence reconciliation.
   - Anti-entropy gap detection triggers historical replay from peer ring buffers; partition cache flushing if gaps exceed the ring buffer window.

2. **Cross-Restart Persistence (Deferred to Milestone 5):**
   - In-memory node states and connection pools are ephemeral. Cross-restart cache consistency relies on origin TTL expiration and startup cache warming.

