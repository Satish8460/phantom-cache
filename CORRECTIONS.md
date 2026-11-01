# PhantomCache — Formal Corrections Ledger

This ledger tracks verified corrections, bug fixes, and architectural reconciliations across roadmap phases.

---

## Milestone 1 (M1 — Truth & Baseline)
- **CORR-M1-01 (Baseline Calibration):** Aligned all reported benchmark numbers with physical loopback measurements; eliminated unverified throughput claims in marketing materials.
- **CORR-M1-02 (Known Gap Characterization):** Codified Known Architecture Gaps KG-1 through KG-6 with deterministic pin tests (`tests/integration/known_gaps.rs`).

---

## Milestone 2 (M2 — Core Correctness, Performance & Freshness)
- **CORR-M2-01 (Epoch Mutex Contention):** Replaced shard-wide epoch mutex with atomic lockless generation counters (`AtomicU64`), eliminating cross-thread contention on tenant invalidation.
- **CORR-M2-02 (O(1) Eviction Algorithms):** Replaced $O(N)$ linear scans with true $O(1)$ intrusive SIEVE, S3-FIFO, and LRU eviction queues (`src/core/eviction.rs`).
- **CORR-M2-03 (Telemetry Sampling Overhead):** Replaced blocking sort operations on the serving hit path with fixed-capacity rolling buffers and lockless atomic sampling (`src/intel/telemetry.rs`).

---

## Milestone 3 (M3 — Distributed Correctness & Architecture Gate)
- **CORR-M3-01 (KG-1 / C-01: Unified Invalidation Cache Target):**
  - *Defect:* `VirtualNode::new` created an isolated in-memory cache distinct from the HTTP gateway's `PartitionedCache`, causing peer invalidations to leave serving caches stale ($P(\text{stale}) = 1.0$).
  - *Correction:* Unified the cluster listener cache target with the HTTP serving cache via `VirtualNode::new_with_cache` in `src/main.rs:248` and `src/dist/node.rs:62`.
- **CORR-M3-02 (KG-2 / C-02: Production Anti-Entropy Worker):**
  - *Defect:* `reconcile_peer_anti_entropy` had zero production callers in `src/`.
  - *Correction:* Implemented background reconciliation loop (`start_anti_entropy_worker`) in `src/dist/router.rs` and wired it into production gateway initialization in `src/main.rs`.
- **CORR-M3-03 (KG-3 / C-05: Contiguous Sequence Gap Recovery):**
  - *Defect:* `record_inbound_seq` in `src/dist/anti_entropy.rs` advanced `expected` past sequence gaps, causing `get_highest_acked_seq` to acknowledge missing packets and prevent replay.
  - *Correction:* Implemented contiguous sequence acknowledgment and out-of-order buffering in `AntiEntropyTracker`. Missing sequences are recorded in `detected_gaps` without advancing `highest_acked_seq`.
- **CORR-M3-04 (KG-4 / C-04: Per-Destination Stream Sequencing):**
  - *Defect:* A single global `stream_sequence: AtomicU64` on `RendezvousRouter` was shared across all destination nodes, causing interleaved traffic to look like packet loss (false gap detection).
  - *Correction:* Implemented `DestinationSequencer` partitioning sequence counters per destination node ID. Retries in `ReplicaBroadcaster` reuse the assigned sequence number without incrementing.
- **CORR-M3-05 (KG-5 / C-08: Local Purge Precedence):**
  - *Defect:* `delete_cache_handler` in `src/api/server.rs` deferred local entry deletion until after the cluster RPC round trip succeeded, allowing concurrent local GETs to serve stale pre-delete data.
  - *Correction:* Moved `cache.delete(&cache_key)` to execute immediately at the start of DELETE processing, enforcing Guarantee G1 (per-node linearizability).
- **CORR-M3-06 (C-07: Autonomous Suspect-to-Active State Recovery):**
  - *Defect:* Nodes marked `Suspect` due to transient network failures remained permanently `Suspect`.
  - *Correction:* Connected the periodic anti-entropy reconciliation worker to test link reachability; once the network heals, anti-entropy sync transitions the peer from `Suspect` / `Recovering` back to `Active`.

