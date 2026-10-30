# ADR-002: Serving-Path Contract (Coalescing, Cancellation, Timeouts, and Fencing)

**Status:** Accepted  
**Phase:** Phase 2 (M2 — Core Correctness, Performance & Freshness)  
**Author:** Antigravity (AG)  
**Date:** 2026-10-29  
**Replaces / Extends:** Formalizes serving path behaviors from Phase 1 baseline into verified clauses CT-1 through CT-10.

---

## 1. Context

PhantomCache acts as a high-throughput caching reverse proxy. When multiple concurrent clients request the same uncached or expired resource ("flash crowd" / "cache stampede"), the gateway must prevent origin overload, enforce request fairness, handle client disconnections gracefully, and preserve cache consistency under concurrent invalidations.

This document establishes the definitive Serving-Path Contract across ten core clauses (CT-1 through CT-10), tracing each to its implementation and automated regression tests in `tests/integration/serving_contract_tests.rs`.

---

## 2. Serving-Path Contract Clauses

### Clause CT-1: Single-Flight Coalescing
- **Specification:** For any given `CacheKey` and fencing epoch, at most one origin fetch is initiated concurrently. Concurrent requests for the same key register as waiters on a `SharedOp` and await completion of the single in-flight operation.
- **Code Locations:** `src/core/cache.rs` (`begin_fetch_atomic`), `src/core/coalesce.rs` (`InitiateRegistration`, `WaitRegistration`).
- **Status:** **VERIFIED**
- **Test:** `test_ct1_single_flight_coalescing`

### Clause CT-2: Initiator Cancellation & Detached Origin Fetch
- **Specification:** The origin fetch is decoupled from the client connection lifecycle via `tokio::spawn`. If the initiator client disconnects, cancels, or times out while the origin fetch is in progress:
  1. The origin fetch task continues to completion.
  2. The response is cached if headers, status, and generation fencing permit.
  3. All remaining registered waiters on `SharedOp` receive the completed result.
- **Code Locations:** `src/api/server.rs` (lines 885–940, `tokio::spawn` enclosing `fetch_origin_raw` and `guard.complete`).
- **Status:** **VERIFIED**
- **Test:** `test_ct2_initiator_cancellation_detached_fetch`

### Clause CT-3: Waiter Bounds & Overflow Protection
- **Specification:** The number of waiters coalesced on a single in-flight key is bounded by `max_waiters_per_key` (default: 1,000). Excess requests beyond the limit do not block indefinitely; they fail immediately with `FetchRegistrationResult::WaitFailed(WaiterLimitExceeded)` and receive HTTP 503 Service Unavailable / 504 Gateway Timeout.
- **Code Locations:** `src/core/coalesce.rs` (lines 35–45), `src/core/cache.rs` (line 120).
- **Status:** **VERIFIED**
- **Test:** `test_ct3_waiter_limit_enforcement`

### Clause CT-4: Origin Timeout Hierarchy
- **Specification:** Origin interactions enforce strict timeout boundaries:
  1. *Connect timeout:* Default 5 seconds. If TCP connection cannot be established, fail with `OriginError::Timeout` -> HTTP 504.
  2. *Read/Response timeout:* Configurable total deadline. Stalled origin reads trigger timeout and deliver `OriginError::Timeout` to all waiters.
- **Code Locations:** `src/api/server.rs` (`fetch_origin_raw`, timeout wrappers).
- **Status:** **VERIFIED**
- **Test:** `test_ct4_origin_timeout_mapping`

### Clause CT-5: Panic and Abort Safety
- **Specification:** If an in-flight origin fetch panics, aborts, or is terminated before `complete()` is called, `InitiateGuard::drop` executes deterministically:
  1. Shard `in_flight` registration is removed.
  2. Shard `active_op_ids` for the key is cleared.
  3. All waiting clients receive `Err(OriginError::Aborted)` immediately rather than hanging indefinitely.
  4. Memory and metrics (`origin.in_flight`) return to clean zero state.
- **Code Locations:** `src/core/coalesce.rs` (`impl Drop for InitiateGuard`).
- **Status:** **VERIFIED**
- **Test:** `test_ct5_panic_abort_safety`

### Clause CT-6: Error Caching (Negative Caching Contract)
- **Specification:** By default, origin error responses (HTTP 4xx, 5xx) and transport failures are uncacheable. They are dispatched to current waiters to fulfill the flight, but no `CachedResource` entry is stored in the cache map. Subsequent requests initiate a fresh fetch.
- **Code Locations:** `src/api/server.rs` (status == 200 gate for cache insertion).
- **Status:** **VERIFIED**
- **Test:** `test_ct6_errors_uncacheable`

### Clause CT-7: Fencing Under Concurrent Invalidation
- **Specification:** If a `DELETE` or tenant epoch invalidation occurs while an origin fetch is in-flight:
  1. The delete immediately advances the shard generation / key state epoch.
  2. When the in-flight fetch completes, `guard.complete()` detects generation mismatch (`op_gen != current_gen`).
  3. The stale response is rejected from cache storage (`stale_ops_ignored` incremented).
  4. The pre-delete state is never resurrected.
- **Code Locations:** `src/core/cache.rs` (`complete_operation` generation check, `invalidate`).
- **Status:** **VERIFIED**
- **Test:** `test_ct7_fencing_under_invalidation`

### Clause CT-8: HTTP Framing and Body Size Limits
- **Specification:** Responses lacking explicit framing (`Content-Length` or `Transfer-Encoding: chunked`) are marked uncacheable to prevent truncated cache poisoning. Responses exceeding the maximum body cap (default: 2 MiB / 2,097,152 bytes) are rejected or marked uncacheable.
- **Code Locations:** `src/api/server.rs` (`has_explicit_framing`, body length limits).
- **Status:** **VERIFIED**
- **Test:** `test_ct8_framing_and_body_limits`

### Clause CT-9: Expiry Herd Coalescing
- **Specification:** When an existing resident cache entry expires or becomes stale, concurrent requests encountering the expired entry do not stampede the origin. The first request transitions to `Initiate`, and subsequent requests wait on the single revalidation/fetch flight.
- **Code Locations:** `src/core/cache.rs` (`begin_fetch_atomic` on expired entry).
- **Status:** **VERIFIED**
- **Test:** `test_ct9_expiry_herd_coalescing`

### Clause CT-10: Strict Resource Bounds
- **Specification:** The serving path maintains strict resource bounds at all times:
  1. Memory capacity bounded by logical byte accounting (Invariants I1–I8).
  2. Origin in-flight count bounded and tracked via atomic gauges (`OriginMetrics`).
  3. Waiter queue depths bounded per key.
- **Code Locations:** `src/core/cache.rs`, `src/intel/origin_metrics.rs`.
- **Status:** **VERIFIED**
- **Test:** `test_ct10_resource_bounds`

---

## 3. Conformance and Test Traceability

Every clause defined in this ADR is verified by executable integration tests in `tests/integration/serving_contract_tests.rs`.

---

## 4. Single-Node Invalidation & Reclamation Contract (M2-04)

### Clause INV-1: Linearizable Single-Key Deletion
- **Specification:** After a `DELETE` operation returns HTTP 200/204 to the caller, no subsequent request on this node may observe the pre-delete entry. The deletion is immediately linearizable with respect to subsequent local requests.
- **Code Locations:** `src/core/cache.rs` (`try_delete_with_status`, `delete`), `src/api/server.rs` (`delete_cache_handler`).
- **Status:** **VERIFIED**
- **Test:** `test_linearizable_delete_prevents_read`

### Clause INV-2: In-Flight Fill Fencing under Deletion
- **Specification:** If an origin fetch was initiated before a `DELETE` operation was executed, completion of the fetch after the deletion must NOT resurrect the deleted data. The in-flight generation check (`state.generation == op_gen`) drops the stale write and increments `stale_writes_prevented`.
- **Code Locations:** `src/core/cache.rs` (`complete_operation`).
- **Status:** **VERIFIED**
- **Test:** `test_delete_fences_in_flight_fill_race`

### Clause INV-3: $O(1)$ Lazy Tenant and Global Epoch Invalidation
- **Specification:** Tenant-wide and cluster/global cache invalidations are executed in $O(1)$ time by atomically bumping tenant or global epoch counters. Subsequent reads on entries with older epochs immediately evaluate as cache misses without requiring immediate full-table scans.
- **Code Locations:** `src/core/cache.rs` (`invalidate_tenant`, `invalidate_all`, `get_effective_epochs`).
- **Status:** **VERIFIED**
- **Test:** `test_tenant_flush_ordering_and_lazy_invalidation`

### Clause INV-4: Bounded Background Sweeper Reclamation
- **Specification:** Lazy-invalidated and expired entries are reclaimed by an asynchronous background sweeper task (`sweep_lazy_reclamation`). To prevent hit-path latency interference, the sweeper operates under strict bounds:
  1. *Per-tick work budget:* At most $N \le 32$ entries inspected per shard per tick (O2-8).
  2. *Lock-hold bound:* Shard mutex lock hold time during sweeping is strictly bounded to $\le 1$ ms.
  3. *Active-op safety:* Entries with active operations or in-flight single-flight registrations are never swept.
- **Code Locations:** `src/core/cache.rs` (`sweep_lazy_reclamation`, `spawn_sweeper_task`).
- **Status:** **VERIFIED**
- **Test:** `test_sweeper_lock_hold_time_bounded`, `test_sweeper_convergence_after_tenant_flush`

### Clause INV-5: Accounting Invariant Convergence
- **Specification:** The background sweeper subtracts accurate entry sizes (`calculate_state_bytes`) upon reclamation, ensuring `bytes_resident` converges to the expected baseline following a tenant or global flush while preserving logical byte accounting invariants $I1$ through $I8$.
- **Code Locations:** `src/core/cache.rs` (`sweep_lazy_reclamation`, `audit_bytes`).
- **Status:** **VERIFIED**
- **Test:** `test_sweeper_vs_concurrent_fills_stress`

