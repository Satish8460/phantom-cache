# PhantomCache — Authoritative Corrections Ledger

**Document ID:** `docs/handoff/CORRECTIONS.md`  
**Authoritative Specification:** `docs/research/phase_1_execution_package.md` v3.1 (WP-01, Step S6)  
**Governing Documents:** `docs/research/adr/ADR-001-present-architecture-and-direction.md`, `docs/research/architecture_investigation.md`, `docs/research/pie_audit.md`, `docs/research/mvp_and_milestones.md`  
**Execution Agent:** Antigravity (AG)  
**Date:** 2026-10-29  
**Status:** AUTHORITATIVE EVIDENCE-BACKED AUDIT CORRECTION  

---

## 1. Purpose and Notice to Engineers

This document serves as the formal corrections ledger for PhantomCache. Earlier handoff documentation (`docs/handoff/00`–`08`), repository README, and milestone design plans contained aspirational claims, architectural assumptions, and documentation drift that did not reflect the reality of the compiled binary (`HEAD 456bb25`).

Every claim in this ledger has been independently verified by reading `src/` source code lines and executing reproducible regression tests.

---

## 2. What the 242-Test Suite Baseline Establishes and Does Not Establish

At the conclusion of Phase 4 handoff, 242 tests passed across 19 targets (`cargo test --all-targets -j 1`).

### What the 242 tests DO establish:
1. **Core Data Structures:** Unit correctness of power-of-two partitioned cache sharding, generation fencing, single-flight coalescing wait batches, and in-memory SIEVE/LRU/S3-FIFO data structures.
2. **Network Codecs & Framing:** Correct binary serialization, deserialization, and cyclic redundancy checksum validation for `PhantomMeshFrame`.
3. **Ingress & Upstream Security:** JWT authentication validation, RBAC claims enforcement, SSRF IP blocking, and RFC 9112 chunked transfer decoder edge cases.
4. **Isolated Component Handlers:** Correct HTTP handler dispatch when `ApiState` is manually constructed in memory with `router: None`.

### What the 242 tests DO NOT establish:
1. **Multi-Gateway Serving Consistency:** Zero tests run multiple HTTP gateways concurrently to assert that an invalidation on Gateway A purges the HTTP cache of Gateway B.
2. **Production Cluster Invalidation:** Zero tests assert that peer invalidations reach `ApiState.cache`. Tests for distributed invalidation purge a separate `VirtualNode.cache` instance.
3. **Self-Healing Anti-Entropy:** `reconcile_peer_anti_entropy` was tested by calling the function directly inside unit test code; zero production callers exist in `src/`.
4. **Adaptive Policy Intelligence (PIE):** Zero tests evaluate PIE decision quality against live origin workloads; existing tests inject synthetic telemetry and assert the state machine transition only.
5. **Production Main Wiring:** Zero integration tests boot the binary via `main()` to verify end-to-end environment parsing and service lifecycle.

---

## 3. Authoritative Claims and Corrections Table

| # | Topic | Documented Claim | Claim Location | Status | Verified Evidence & Reality in Source Code |
|---|---|---|---|---|---|
| **C-01** | Serving-Path Invalidation | Peer invalidations purge the distributed cache tier across all cluster nodes. | `docs/handoff/01:19`, `docs/security/phase-4-design.md` | **CONTRADICTED** | **`src/main.rs:65, 300` vs `src/dist/node.rs:42, 485`**<br>The HTTP serving cache is `PartitionedCache::new(8)` in `ApiState.cache`. Each `VirtualNode` owns a *separate* `PartitionedCache::new(4)`. Cluster invalidation frames purge `node.cache`, never `ApiState.cache`. Gateway B continues serving stale HTTP entries until TTL. Validly reproduced in `tests/integration/known_gaps.rs::kg1_peer_invalidation_does_not_purge_serving_cache`. |
| **C-02** | Anti-Entropy Healing | Self-healing anti-entropy background loop automatically reconciles missed invalidations across nodes. | `docs/handoff/01:19`, `docs/handoff/03:157` | **CONTRADICTED** | **`src/dist/router.rs:787` (`reconcile_peer_anti_entropy`)**<br>Grep across all `src/` confirms zero production callers. No background timer, no event loop trigger, and no heartbeat trigger calls reconciliation. Validly reproduced in `tests/integration/known_gaps.rs::kg3_anti_entropy_has_no_production_caller`. |
| **C-03** | Outbound Invalidation History | Nodes maintain outbound invalidation rings to replay missed invalidations to lagging peers. | `docs/handoff/01:166`, `docs/handoff/03:99` | **CONTRADICTED** | **`src/dist/node.rs:475`**<br>`ring_buffer.push` has exactly one call site in the entire codebase: on inbound frame receipt in `VirtualNode`. The sending gateway never records what it sent; replay is impossible by construction. |
| **C-04** | Sequence-Gap Detection | Sequence numbers track per-peer history to detect gaps reliably. | `docs/handoff/01:166`, `docs/security/phase-4-design.md` | **CONTRADICTED** | **`src/dist/router.rs:142, 223, 615`**<br>`stream_seq` is a single `AtomicU64` per router shared across all destinations and incremented per attempt (including retries). Receiving peers see sporadic subsequences, making normal traffic appear as false-positive sequence gaps. |
| **C-05** | Gap-Aware Replay | Detected sequence gaps trigger targeted replay of missed invalidation events. | `docs/security/phase-4-design.md` | **CONTRADICTED** | **`src/dist/anti_entropy.rs:215-231`**<br>When `record_inbound_seq` detects a gap, it immediately advances `expected = stream_seq + 1`, and `get_highest_acked_seq` returns `expected - 1`. The gap is discarded; reconciliation replays only records *after* the latest seen seq. `pop_gap` has zero callers. |
| **C-06** | Lossy Broadcast Queue | Broadcast queue is strictly bounded to 1,024 items with backpressure. | `docs/security/phase-4-design.md` | **CONTRADICTED** | **`src/dist/router.rs:110, 194-201`**<br>`ReplicaBroadcaster::enqueue` immediately drains items into unbounded spawned Tokio tasks (`tokio::spawn`) that compete for a 16-permit semaphore. Failed sends only increment an unread `dropped_count` counter. Memory is not bounded as documented. |
| **C-07** | Cluster Node State Recovery | Heartbeats and health probes return Suspect/Recovering nodes to Active. | `docs/handoff/03:157`, `docs/handoff/05:59` | **CONTRADICTED** | **`src/dist/node.rs:174, 394-400`**<br>`Suspect` is set on routing failures. Heartbeat frames carry no status mutations. Nothing in the production runtime returns a Suspect node to Active except manual invocation of `/api/control` or process restart. |
| **C-08** | DELETE Request Ordering | Local HTTP cache purge precedes cluster broadcast for fail-safe invalidation. | `docs/handoff/01:166` | **CONTRADICTED** | **`src/api/server.rs:830-843, 871-908`**<br>In `delete_cache_handler`, cluster round-trip happens *first*. Local `cache.delete` occurs *after* primary ACK. On timeout (504), local cache is deleted as "hygiene"; on rejection (5xx), local cache is NOT deleted. Ordering is inverted from documentation. |
| **C-09** | Cluster GetRequest Handler | Mesh nodes forward cache misses to peer nodes or upstream origin. | `docs/demo-lab/implementation-roadmap.md` | **CONTRADICTED** | **`src/dist/node.rs:385-441`**<br>On a cache miss, `node.rs` synthesizes the literal mock string `"MISS val_node_<id>"`, stores it for 60 s, and replies. It never contacts origin. In production, no gateway sends `GetRequest` (GET is 100% local-only in `server.rs:357-770`). Validly reproduced in `tests/integration/known_gaps.rs::pin_kg6_node_get_request_fabricates_missing_key_body`. |
| **C-10** | Freshness & Staleness Guarantees | HTTP caching obeys RFC 9111 `Cache-Control` freshness headers; bounded staleness guaranteed. | `docs/handoff/01:19`, `README.md` | **CONTRADICTED** | **`src/api/server.rs:638`, `src/dist/node.rs:413`**<br>Origin responses are cached with a fixed hardcoded literal: `ttl_ms: 60_000` (60 seconds). `Cache-Control: max-age`, `s-maxage`, `no-cache`, and `must-revalidate` are ignored. Cross-gateway staleness is bounded only by the 60 s TTL. |
| **C-11** | PIE Adaptive Utility & Canary | Multi-objective utility optimization with canary guardrails and rollback cooldowns. | `README.md:34`, `docs/implementation/WP2.md` | **CONTRADICTED** | **`src/control/pie.rs:57-75`**<br>There is zero canary traffic splitting, zero exposure limiting, zero confidence interval calculation, and zero backend load weighting in `src/`. Utility is hardcoded to `hit_rate * 100 - p99 * 0.1`. |
| **C-12** | PIE Count-Min Access Skew | PIE tracks access skew via Count-Min sketch and rolls back if hit ratio degrades. | `docs/handoff/01:109` | **CONTRADICTED** | **`src/intel/sketch.rs` vs `src/control/pie.rs:77-89`**<br>`CountMinSketch` has zero callers in `src/` (used only by unit tests). The rollback branch in `pie.rs` triggers ONLY if candidate name is `"S3-FIFO"` and utility < 50; for all other candidates, degradation yields `NoAction`. |
| **C-13** | PIE Production Implementation Status | PIE decision loop, canary rules, and cooldown controllers implemented and completed. | `docs/implementation/WP2.md` (COMPLETED) | **CONTRADICTED** | **`src/main.rs:306`**<br>`main.rs` initializes `ApiState` with `pie: None`. `PieController` is never constructed in the production binary. It is only called inside `spawn_simulation_loop` (`server.rs:1684`) driven by a synthetic simulation clock. |
| **C-14** | PIE Rollback State Desynchronization | Rollback safely restores active policy and synchronizes controller state. | `docs/implementation/WP2.md` | **CONTRADICTED** | **`src/control/pie.rs:83-89`**<br>The Rollback branch sets the cache eviction policy to SIEVE and sets the cooldown timestamp, but fails to mutate `*active_policy`. Consequently, `get_active_policy()` reports `"S3-FIFO"` while the cache runs SIEVE. |
| **C-15** | Bounded PIE Memory | Decision traces are bounded and audited. | `docs/implementation/WP2.md` | **CONTRADICTED** | **`src/control/pie.rs:98`**<br>`traces.lock().push(trace)` appends to an unbounded `Vec<DecisionTrace>`. In a long-running process with simulation enabled, this represents an unbounded memory leak. |
| **C-16** | Plaintext Cluster Connections | Insecure / plaintext cluster connections are rejected by default. | `docs/handoff/03:12` | **UNSUPPORTED** | **`src/main.rs:265-266`, `src/dist/router.rs:318-321`**<br>Loopback plaintext fallback configuration exists. When enabled without TLS, router points to addressless demo nodes, causing every DELETE to return 504. |
| **C-17** | Diagnostic Endpoints & Metrics | Operational metrics such as `primary_invalidation_timeouts` exposed at `/api/v1/diagnostics`. | `docs/handoff/05:59` | **UNSUPPORTED** | **`src/api/server.rs`**<br>`primary_invalidation_timeouts` does not exist in `src/`. `/api/v1/diagnostics/summary` exposes only `CacheCounters` and static node snapshots. |

---

## 4. Architectural Realities and Canonical Roadmap

PhantomCache as currently built is:
1. **A high-performance single-gateway HTTP cache** featuring lock-striped sharding, single-flight coalescing, Visited-Sweep (SIEVE) eviction, and upstream HTTP security.
2. **A standalone cluster protocol library** with mutual TLS framing and routing primitives that are not yet wired into the serving path.
3. **A simulation-driven demo UI** that displays synthetic cluster and intelligence metrics when running in standalone mode.

To transition from these prototypes to production-grade software and defensible research, the canonical roadmap is structured into eight phases:

- **M1 — Truth & Baseline:** Establish empirical baselines, truthful telemetry v2, byte accounting, and characterization tests.
- **M2 — Core Correctness, Performance & Freshness:** Implement RFC 9111 freshness, stale serving caps, eviction policy traits, and microsecond latency.
- **M3 — Distributed Correctness & Architecture Gate:** Resolve single-node vs distributed serving path wiring; build multi-node fault injection harness.
- **M4 — Phantom Lab & Static Baselines:** Deterministic replay lab, M/M/1 origin simulation, and competitive static baselines.
- **M5 — Phantom Intelligence & PIE:** Genuine predictive intelligence, shadow evaluation, canary guardrails, and audit logging.
- **M6 — Research Evaluation & MVP Gate:** Falsifiable evaluation, prior art closure, and MVP-A / MVP-B verdict.
- **M7 — Productization & Research Release:** Operational hardening, configuration validation, security audits, and release artifacts.
- **M8 — UI/UX Excellence & Apple-Inspired Design System:** Production-grade operations console with truthful metric provenance and accessibility.

See [`docs/research/mvp_and_milestones.md`](../research/mvp_and_milestones.md) and [`docs/research/pie_audit.md`](../research/pie_audit.md) for full architectural audits and specifications.

