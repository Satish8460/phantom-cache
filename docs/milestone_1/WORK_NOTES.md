> Notice (2026-10-30): Historical Milestone 1 verification notes. Its test count ("47/47 passed") is the count of that moment, not the current baseline (see [docs/STATUS.md](../STATUS.md)). Corrections to Milestones 1 to 3 are recorded in [CORRECTIONS.md](../../CORRECTIONS.md).
>
> Original text below is unchanged.

# Milestone 1 Implementation Work Notes & Verification Report

## Objective
Connect PhantomCache's core partitioned cache (`PartitionedCache`) and single-flight coalescer (`OriginFetchGuard`) to Axum HTTP server over real TCP with mock origin, generation-based invalidation, atomic ownership tracking, panic-resilient RAII cleanup, lightweight non-blocking diagnostics, and 23 integration tests.

## Key Design Commitments & Decisions
- **Single Shard Mutex:** Both `shard.map: HashMap<CacheKey, KeyState>` and `shard.in_flight: HashMap<CacheKey, Arc<SharedOp>>` reside under a single `std::sync::Mutex<CacheShard>`, eliminating all cross-component TOCTOU races between lookup, in-flight registration, generation checks, and invalidation.
- **Canonical `CacheKey`:** Composite key `CacheKey { tenant_id, path, query }` directly indexes all maps, preventing string delimiter bugs and cross-tenant leakage.
- **Strict Capacity Invariant & Active State Protection:** Per-shard capacity `max_entries_per_shard` is strictly bounded. If capacity is saturated and every entry in the shard is actively in-flight or has running operations, eviction will not drop live lifecycle state; the incoming request is rejected with `503 Service Unavailable` (`X-Cache-Error: CapacityExceeded`).
- **Bounded Stream Accumulation & Wire Protocol Validation:** `fetch_origin_raw` enforces 64 KB header limit, 2 MB body limit, strict `Content-Length` validation (rejects truncated or oversized payloads), and chunked transfer decoding. Oversized bodies are never cached.
- **Multi-Phase `OriginFetchGuard` with Two Commit Boundaries:**
  - *Commit Boundary 1 (Waiter Extraction & In-Flight Isolation):* Detaches the in-flight entry from `shard.in_flight` and extracts waiters into stack-owned `WaiterBatch`.
  - *Commit Boundary 2 (State Mutation & Lifecycle Accounting):* Updates `shard.map` only if `state.state_instance_id == self.state_instance_id` and `active_op_ids.remove(&op_id)` succeed.
  - *Stack Unwinding Guarantee:* Stack unwinding during panics drops `WaiterBatch`, guaranteeing all waiters receive responses without relying on `try_lock()`.
  - *Lock Poison Recovery:* Mutex poisoning is recovered safely via `.into_inner()` on subsequent requests, recording `lock_poison_recovered`.
- **Non-Blocking Diagnostics:** `DiagnosticBuffer` (512 slots) uses `Mutex::try_lock()`. On contention, events are dropped and `counters.diagnostic_events_dropped` increments, ensuring zero latency impact on request threads. Key fingerprints use keyed SipHash (`k_xxxxxxxx`).

## Verification Results
- `cargo check --all-targets`: Passed with 0 errors.
- `cargo clippy --all-targets -- -D warnings`: Passed with 0 warnings.
- Unit Test Suite (`cargo test --test wp1_tests --test wp2_tests --test wp3_tests --test wp4_tests`): 18/18 passed (including `test_core_200k_unique_key_flood_strict_capacity`).
- Integration Test Suite (`cargo test --test m1_http_tests`): 29/29 passed (including adversarial tests M1-T24 through M1-T29).
- Full Repository Test Suite (`cargo test -j 1`): 47/47 passed (0 failed, 0 ignored).

| Test Group | Tests Run | Result | Notes |
| :--- | :--- | :--- | :--- |
| `m1_http_tests` | 29 | 29 PASSED | Real-TCP Axum & Mock Origin on `127.0.0.1:0` with `reqwest`, covering M1-T01 to M1-T29 (lifecycle, invalidation, cross-tenant isolation, 100-client coalescing, saturation, body limit, chunked/oversized framing, and strict capacity bounds) |
| `wp1_tests` | 5 | 5 PASSED | Virtual clock, seeded PRNG, coalescer, partitioned ops, and 200k unique key sustained flood under strict per-shard memory bounds |
| `wp2_tests` | 5 | 5 PASSED | PIE controller, telemetry buffer, Count-Min sketch |
| `wp3_tests` | 3 | 3 PASSED | Rendezvous hash router, node failure handling |
| `wp4_tests` | 5 | 5 PASSED | SSE state projection, control endpoint, live health check |
| **Total** | **47** | **47 PASSED** | **100% Green** |

