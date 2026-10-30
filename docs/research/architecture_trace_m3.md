> Notice (2026-10-30): Dated trace (2026-10-29). Two staleness caveats for readers: (1) the relative links to source files, such as `../src/api/server.rs#L732-L1110`, have the wrong depth for this directory (the correct prefix is `../../src/`) and do not resolve; they are left unchanged; (2) the line ranges refer to the code of 2026-10-29 and may have drifted since (the source files were edited afterwards; for example, the shared-cache construction `VirtualNode::new_with_cache` is now at `src/main.rs:332`). Use symbol names rather than the line ranges. The same wrong-depth link pattern also occurs in `docs/research/phase_2_completion_report.md` (`../src/core/freshness.rs`).
>
> Original text below is unchanged.

# PhantomCache Architecture Trace — M3: Distributed Layer Call Chains & Gaps

**Document ID:** `docs/research/architecture_trace_m3.md`  
**Authoritative Specification:** `docs/research/phase_3_execution_package.md` §4 (Step M3-00)  
**Related Documents:** `docs/research/adr/ADR-001-present-architecture-and-direction.md`, `docs/handoff/CORRECTIONS.md`, `docs/research/phase_2_completion_report.md`  
**Working Branch:** `feat/publish-phantomcache-core`  
**Base HEAD:** `456bb25930004f1896a84c53e776e7b3ef29ee16`  
**Execution Agent:** Antigravity (AG)  
**Date:** 2026-10-29T15:18:00+05:30 (IST)  
**Status:** AUTHORITATIVE CODE-LEVEL ARCHITECTURAL TRACE  

---

## 1. Executive Purpose

This document establishes the verified code-level call chains for all seven primary operations across the gateway serving tier (`src/api/server.rs`), the core caching tier (`src/core/`), and the distributed mesh tier (`src/dist/`).

Every trace entry documents the exact source lines, executing callers, mutated data structures, and the structural disconnects between the HTTP serving cache and the cluster protocol.

---

## 2. Call-Chain Traces (Current Codebase)

### 2.1 Trace 1: HTTP GET Request (Local Serving Path)
- **Entry Point:** `get_cache_handler` in [`src/api/server.rs:732-1110`](../src/api/server.rs#L732-L1110).
- **Execution Path:**
  1. Authenticates request and extracts tenant (`authenticate_request`, `server.rs:748-765`).
  2. Constructs `CacheKey::new(tenant_id, &norm_path, raw_query)` (`server.rs:777`).
  3. Checks stale serving eligibility: `cache.check_stale_serving` (`server.rs:881`).
     - If eligible: returns stale payload immediately (`X-Cache-Status: STALE`), schedules background revalidation (`server.rs:901-925`).
  4. Core fetch registration: `cache.begin_fetch_atomic(&cache_key, now_ms)` (`server.rs:939`).
     - **Hit:** Returns `CachedResource`. Hot path acquires shard read lock, updates `eviction.on_hit()`, reads payload (`cache.rs:430-495`).
     - **Wait:** Coalesces onto existing `SharedOp` (`coalesce.rs:180-240`).
     - **Initiate:** Current request becomes leader (`cache.rs:465-475`).
  5. Leader fetches from upstream origin: `fetch_origin_stream` / `fetch_origin_raw` (`server.rs:990-1045`).
  6. Leader completes fetch: `guard.complete(FinalizeOutcome::Success(res))` (`server.rs:1052`).
  7. Finalizes in shard: `cache.finalize_fetch` stores payload, verifies generation fence, updates byte accounting ($I1–I8$), calls `eviction.on_insert()` (`cache.rs:610-675`).
- **Cluster Interaction:** **ZERO.** In production, HTTP GET is 100% local. The gateway never contacts cluster peers, never routes via HRW, and never consults mesh nodes (`state.router` is ignored on GET).

---

### 2.2 Trace 2: HTTP DELETE Request (Local & Cluster Purge)
- **Entry Point:** `delete_cache_handler` in [`src/api/server.rs:1533-1675`](../src/api/server.rs#L1533-L1675).
- **Execution Path:**
  1. Validates authentication and claims (`server.rs:1550-1565`).
  2. Constructs and validates `CacheKey` (`server.rs:1573-1577`).
  3. **Cluster Round Trip Executed First (KG-5 / C-08 Disconnect):**
     - Calls `router.invalidate_key_across_cluster(&canonical_key, InvalidationScope::ExactKey)` (`server.rs:1581-1586`).
     - In `router.rs:716-720`: Computes primary node via HRW `self.route_key(key)`.
     - Sends invalidation frame to primary node over TCP/mTLS: `self.send_invalidation_frame(&primary, key, scope, timeout_ms)` (`router.rs:720`).
     - If primary fails: sets primary to `NodeState::Suspect`, falls back to acting primary (`router.rs:745-754`).
     - Enqueues background replica broadcast to replica nodes (`router.rs:724-730`).
  4. **Local Serving Cache Purge Executed Second:**
     - Only after the cluster primary acknowledges success: `cache.delete(&cache_key)` (`server.rs:1590`).
     - If cluster returns 5xx error: local cache is NOT deleted (`server.rs:1603-1620`).
     - If cluster times out (504): local cache is deleted as "hygiene" fallback (`server.rs:1645`).
  5. Records telemetry observation (`RawObservationUs::new_delete`, `server.rs:1660`).
  6. Returns `204 No Content` to caller (`server.rs:1601`).
- **Structural Disconnect:** Any concurrent GET arriving on this gateway while the cluster round trip is in flight (150ms timeout) hits `cache` and serves the old cached entry (reproduced in `known_gaps.rs::kg5_get_after_delete_start_must_not_serve_old_entry`).

---

### 2.3 Trace 3: Inbound Peer Invalidation (Frame Receipt)
- **Entry Point:** `VirtualNode::start_tcp_service` listener in [`src/dist/node.rs:360-560`](../src/dist/node.rs#L360-L560).
- **Execution Path:**
  1. Accepts inbound TCP connection, negotiates mTLS (`node.rs:370-385`).
  2. Reads frame: `read_frame(&mut stream)` (`node.rs:395`).
  3. Dispatches `MessageType::InvalidateRequest`:
     - Extracts payload: `frame.extract_invalidate_request()` (`node.rs:459`).
     - Records stream sequence: `node.anti_entropy_tracker.lock().unwrap().record_inbound_seq(...)` (`node.rs:465-472`).
     - Pushes to circular ring buffer: `node.ring_buffer.lock().unwrap().push(...)` (`node.rs:475-481`).
     - **Purges VirtualNode's Private Cache (KG-1 / C-01 Disconnect):**
       - Exact key: `node.cache.try_delete_canonical(&payload.key)` (`node.rs:485`).
       - Tenant: `node.cache.try_invalidate_tenant(&payload.key)` (`node.rs:498`).
       - Full cache: `node.cache.try_invalidate_all()` (`node.rs:504`).
     - Sends `InvalidateAck` frame back to sender (`node.rs:511-526`).
- **Structural Disconnect:** `node.cache` is an isolated `PartitionedCache::new(4)` instantiated inside `VirtualNode::new` (`node.rs:42`). The HTTP serving cache in `ApiState.cache` is a separate `PartitionedCache::new(8)` instantiated in `main.rs:65`. Inbound invalidation frames never touch `ApiState.cache`! Gateway B continues serving stale HTTP content until TTL.

---

### 2.4 Trace 4: Outbound Replica Broadcast
- **Entry Point:** `ReplicaBroadcaster::enqueue` in [`src/dist/router.rs:110-185`](../src/dist/router.rs#L110-L185).
- **Execution Path:**
  1. `RendezvousRouter::invalidate_key_across_cluster` enqueues `ReplicaInvalidationTask` to bounded mpsc channel (`capacity = 1024`, `router.rs:61, 724`).
  2. Broadcaster loop pops task (`router.rs:88`).
  3. **Task Spawn Spawning (C-06 Disconnect):**
     - Immediately spawns a separate `tokio::spawn` task per task (`router.rs:103`).
     - Spawned tasks compete for 16-permit semaphore `sem.acquire_owned()` (`router.rs:104`).
     - Under high invalidation rates, spawned tasks accumulate without bound while waiting for permits.
  4. **Shared Stream Sequence (KG-4 / C-04 Disconnect):**
     - Pops global `stream_seq = self.stream_sequence.fetch_add(1)` (`router.rs:142`).
     - Global counter is shared across all destination peers and retry attempts.
     - Destination peers observe missing sequence numbers and falsely trigger gap detection.
  5. **Outbound Ring Buffer Never Populated (C-03 Disconnect):**
     - Outbound invalidations are sent via TCP, but never pushed into the sender's `ring_buffer`.

---

### 2.5 Trace 5: Connection Establishment & Reconnect
- **Entry Point:** `TlsConnectionPool::acquire_connection` in [`src/dist/pool.rs:120-210`](../src/dist/pool.rs#L120-L210).
- **Execution Path:**
  1. Checks idle connection queue for `peer_node_id` (`pool.rs:135`).
  2. If empty: opens new TCP stream to `node.get_addr()`, performs TLS handshake with `rustls` (`pool.rs:155-180`).
  3. Assigns connection sequence number `sequence_number = 1` (`pool.rs:185`).
  4. Returns `PooledConnection` guard; on drop, returns connection to idle pool if healthy (`pool.rs:90-115`).

---

### 2.6 Trace 6: Heartbeat & Membership State Transitions
- **Entry Point:** `TlsConnectionPool::start_heartbeat_worker` in [`src/dist/pool.rs:240-330`](../src/dist/pool.rs#L240-L330).
- **Execution Path:**
  1. Periodic timer ticks every 1,000 ms (`pool.rs:250`).
  2. Iterates over active cluster nodes, sends `PhantomMeshFrame::new_heartbeat_request` (`pool.rs:275`).
  3. On success: reads `HeartbeatResponse`.
  4. On timeout or I/O failure: marks node as `NodeState::Suspect` (`pool.rs:310`).
- **Structural Disconnect (C-07):** Heartbeats carry no state mutations. Nothing in the heartbeat worker returns a `Suspect` node to `Active` after recovery. Nodes remain permanently `Suspect` unless manually mutated via admin API or process restart.

---

### 2.7 Trace 7: Anti-Entropy Reconciliation
- **Entry Point:** `RendezvousRouter::reconcile_peer_anti_entropy` in [`src/dist/router.rs:787-890`](../src/dist/router.rs#L787-L890).
- **Execution Path:**
  1. Sets `local_node` and `peer_node` to `NodeState::Recovering` (`router.rs:793-797`).
  2. Retrieves `highest_acked_seq` from `local_node.anti_entropy_tracker` (`router.rs:806-810`).
  3. Sends `AntiEntropySyncRequest` frame with acked sequence and local ring digest (`router.rs:813-825`).
  4. Peer node evaluates request against its local `ring_buffer` (`node.rs:530-580`):
     - If peer ring buffer contains range: returns `SyncResponseInSync` or `SyncResponseReplayBatch`.
     - If peer ring buffer was overwritten: returns `SyncResponseOverwrittenHistory`.
  5. Local node receives response:
     - On Replay: applies `local_node.sync_anti_entropy_batch(&records)`.
     - On Overwrite: executes `local_node.flush_for_anti_entropy_overwrite()`.
     - Restores `NodeState::Active` on verified completion.
- **Structural Disconnect (KG-2 / C-02):** Grep across all `src/` confirms **zero production callers** for `reconcile_peer_anti_entropy`. It is never called by any heartbeat loop, connection reconnect, or background timer.

---

## 3. Known Gaps Verification Matrix (KG-1 through KG-6)

| Gap ID | Description | Source Reference | Baseline Test Status | Reproduction Finding |
|---|---|---|---|---|
| **KG-1** | Peer invalidation does not purge serving HTTP cache | `src/main.rs:65, 300` vs `src/dist/node.rs:42, 485` | `tests/integration/known_gaps.rs::kg1` (ignored) | **CONFIRMED REPRODUCIBLE.** `VirtualNode` owns private `PartitionedCache(4)`; `ApiState` owns `PartitionedCache(8)`. No bridge exists. |
| **KG-2** | Anti-entropy reconciliation has no production caller | `src/dist/router.rs:787` | `tests/integration/known_gaps.rs::kg2` (ignored) | **CONFIRMED REPRODUCIBLE.** Grep across `src/` yields zero callers outside test code. |
| **KG-3** | Gap handling advances acked sequence past missing seq | `src/dist/anti_entropy.rs:215` | `tests/integration/known_gaps.rs::kg3` (ignored) | **CONFIRMED REPRODUCIBLE.** `expected` is incremented to `stream_seq + 1`, causing `get_highest_acked_seq` to skip missing entries. |
| **KG-4** | Shared router stream sequence causes false gaps | `src/dist/router.rs:142, 615` | `tests/integration/known_gaps.rs::kg4` (ignored) | **CONFIRMED REPRODUCIBLE.** Single `AtomicU64` shared across all peer destinations interleaves sequence numbers. |
| **KG-5** | DELETE purges local entry after cluster round-trip | `src/api/server.rs:1581-1590` | `tests/integration/known_gaps.rs::kg5` (ignored) | **CONFIRMED REPRODUCIBLE.** Cluster call precedes local `cache.delete`; concurrent GET serves stale cache HIT during 150ms window. |
| **KG-6** | Cluster GetRequest handler fabricates missing body | `src/dist/node.rs:417` | `tests/integration/known_gaps.rs::pin_kg6` (PASS) | **CONFIRMED REPRODUCIBLE.** Handler returns `"MISS val_node_<id>"` literal instead of contacting origin. |

---

## 4. Key Architectural Conclusions for Step M3-01 / M3-03

1. **Cache Unification is Mandatory for Consistency:** `VirtualNode` must share the same `Arc<PartitionedCache>` instance as `ApiState.cache`. When `VirtualNode` receives an invalidation frame over TCP, it must purge the exact memory partition that `get_cache_handler` reads from.
2. **Per-Destination Stream Sequencing:** Sequence counters must be tracked per peer destination (`HashMap<u8, AtomicU64>`), not globally on the router.
3. **Outbound Ring Buffer Logging:** The sending node must record its sent invalidations in a local ring buffer so that lagging peers can replay them during anti-entropy catchup.
4. **Active Reconciliation Triggering:** A background task or connection-recovery trigger must call `reconcile_peer_anti_entropy` upon reconnect or detected gap.
5. **Local Purge Precedence:** `delete_cache_handler` must invalidate or fence the local cache key *before* initiating the cluster broadcast round-trip.

