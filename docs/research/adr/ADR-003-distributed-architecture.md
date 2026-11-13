# ADR-003: Distributed Cache Architecture, Failure Model, and Consistency Contract

**Status:** ACCEPTED (Gate G-ARCH Resolution)  
**Date:** 2026-10-29  
**Decision Makers:** Antigravity (Implementation & Verification Agent), Claude (Research Lead Architecture)  
**Governing Specification:** `docs/research/phase_3_execution_package.md` §3 & §4 (M3-03)  
**Related Documents:** `docs/research/architecture_trace_m3.md`, `docs/handoff/CORRECTIONS.md`, `docs/research/adr/ADR-001-present-architecture-and-direction.md`, `docs/research/adr/ADR-002-serving-path-contract.md`  

---

## 1. Context and Problem Statement

In Milestone 2 (Core Correctness, Performance & Freshness), PhantomCache established formal single-node serving contracts (ADR-002), RFC 9111 HTTP freshness, $O(1)$ eviction policies, and microsecond telemetry.

However, as revealed by the Phase 1 Known Gaps audit (`CORRECTIONS.md` C-01..C-08) and characterized empirically in Step M3-02 (`bench-results/cluster/characterization_report.json`), the existing distributed mesh tier exhibited severe structural disconnects:
1. **Cache Partition Disconnect (C-01 / KG-1):** Inbound peer invalidation frames purged an isolated `VirtualNode.cache` instance (`PartitionedCache::new(4)`). The HTTP serving cache (`ApiState.cache`, `PartitionedCache::new(8)`) was completely unlinked. In Scenario S3-1, peer gateways served stale cache hits with **$P(\text{stale-after-ack}) = 1.0$ (100% stale reads)** until local 60s TTL expiration!
2. **Global Stream Sequence Aliasing (C-04 / KG-4):** Invalidation frame sequence numbers were generated from a single router-wide `AtomicU64`. Interleaved traffic across multiple peers generated false-positive sequence gaps under 0% network loss.
3. **Premature Ack Advancement (C-05 / KG-3):** `AntiEntropyTracker::record_inbound_seq` advanced `*expected = stream_seq + 1` across sequence gaps, causing `get_highest_acked_seq` to skip missing sequences and permanently defeat gap recovery.
4. **Missing Outbound Invalidation Log (C-03):** Senders never recorded sent invalidations into their ring buffers, leaving anti-entropy replay empty upon peer reconnection.
5. **Orphaned Anti-Entropy Engine (C-02 / KG-2):** `reconcile_peer_anti_entropy` had zero production call sites in `src/`.
6. **Inverted DELETE Ordering (C-08 / KG-5):** Gateways executed cluster RPCs *before* local cache eviction, allowing concurrent GETs during the 150ms round-trip to serve stale pre-DELETE data.

At Gate **G-ARCH**, PhantomCache must select a defensible, mathematically specified distributed architecture.

---

## 2. Candidate Architecture Evaluation

Five architectural options were evaluated per `phase_3_execution_package.md` §3:

| Option | Architecture Concept | Consistency Guarantee | Empirical Trade-offs & Costs | Verdict |
|---|---|---|---|---|
| **Option C** | Single-Node Only; Gate Cluster as Experimental | Single-node linearizability; cross-gateway staleness $\le \text{TTL}_{eff}$ | Lowest complexity; forfeits all distributed research value and multi-gateway consistency claims. | **REJECTED as primary** (retained as config fallback) |
| **Option A1** | Independent Local Caches + Reliable Invalidation Fan-Out | Bounded staleness $\Delta \le D_{max}$ across connected peers; fallback $\le \text{TTL}_{eff}$ | Medium complexity; requires cache unification, per-destination sequences, and active anti-entropy reconciliation. | **ACCEPTED (Selected Architecture)** |
| **Option A2** | Best-Effort Fan-Out + Conservative Flushes | Correctness via over-invalidation | High hit-ratio degradation; flush storms under transient partitions and flapping links. | **REJECTED** |
| **Option D** | Synchronous Fan-Out | Read-after-ack across reachable peers; offline $\le \text{TTL}_{eff}$ | High DELETE latency coupling; availability degrades with slow peers. | **REJECTED** |
| **Option B** | Coordinated Reads via HRW Owner Forwarding | Strong single-copy reads | Highest complexity; forward loops, hot-key amplification, double network hops on reads. | **REJECTED** |

---

## 3. Decision: Option A1 (Reliable Invalidation Fan-Out)

We formally select **Option A1** as the canonical distributed architecture of PhantomCache.

### 3.1 Architectural Pillars of Option A1

1. **Unified Cache Architecture (Resolving C-01 / KG-1):**
   - The local `VirtualNode` and `ApiState` share the exact same `Arc<PartitionedCache>` instance.
   - When an inbound `InvalidateRequest` frame arrives over TCP/mTLS, `node.cache.try_delete_canonical(&payload.key)` purges the exact in-memory shard that `get_cache_handler` reads from.
2. **Per-Destination Stream Sequencing (Resolving C-04 / KG-4):**
   - Each router maintains independent sequence counters per peer destination (`HashMap<u8, AtomicU64>`).
   - Retries reuse the same sequence number.
   - Eliminates false sequence gaps under healthy network conditions.
3. **Contiguous Cumulative Sequence Tracking (Resolving C-05 / KG-3):**
   - `AntiEntropyTracker` maintains `highest_contiguous_acked` independently from `highest_seen`.
   - Sequence gaps prevent `highest_contiguous_acked` from advancing past missing sequences.
   - Replay queries accurately request retransmission of the unacknowledged contiguous window.
4. **Outbound Invalidation Logging (Resolving C-03):**
   - Senders push outbound invalidation records into their local `InvalidationRingBuffer`.
   - Ring buffer is bounded by dual limits: $\le 4,096$ entries and $\le 4$ MB payload memory.
   - Lagging peers can replay missing invalidations upon reconnection.
5. **Active Production Anti-Entropy Reconciliation (Resolving C-02 / KG-2):**
   - Background periodic reconciliation loop (every 5 seconds) inspects peer anti-entropy state.
   - Reconnection events immediately trigger reconciliation before returning nodes to `Active`.
6. **Bounded Broadcast Queue & Worker Semaphore (Resolving C-06):**
   - `ReplicaBroadcaster` limits in-flight broadcast tasks via strict bounded channel and semaphore.
7. **Active Membership State Recovery (Resolving C-07):**
   - Health probes recover `Suspect` nodes to `Active` upon successful verified communication.
8. **Inverted Local Purge Precedence (Resolving C-08 / KG-5):**
   - In `delete_cache_handler`, local cache eviction / invalidation occurs *first*, preventing local stale reads while cluster round-trip is in flight.

---

## 4. Formal Failure Model and Consistency Contract

### 4.1 Failure Model $\mathcal{F}$
- **Network Message Loss:** Link packet loss $p \le 20\%$ supported via bounded retry and anti-entropy catchup.
- **Partitions:** Transient network partitions $\le P_{max}$ (up to ring buffer retention capacity).
- **Node Restarts:** Supported via clean sequence resync and cold-start catchup.
- **Clock Skew:** Zero clock dependence. All consistency guarantees rely on monotonic sequence numbers, not wall-clock timestamps.

### 4.2 Consistency Guarantees
- **Guarantee G1 (Per-Node Linearizability):** After a `DELETE` request returns HTTP `204 No Content` on gateway $X$, no subsequent read on $X$ shall observe the pre-delete cached body.
- **Guarantee G2 (Cross-Node Eventual Consistency Bound):** After a `DELETE` request is acknowledged on gateway $X$, any subsequent read on a connected, reachable peer gateway $Y$ after $\Delta \le D_{max}$ ($D_{max} \le 250$ ms on healthy loopback/LAN) shall observe post-invalidation content.
- **Guarantee G3 (Fallback Staleness Cap):** Under permanent partitions or link failures exceeding retry capacity, stale serving is strictly bounded by:
  $$\text{Staleness} \le \text{TTL}_{eff} \le \text{PHANTOM\_MAX\_STALE\_MS} \quad (86,400,000 \text{ ms})$$

### 4.3 Explicit Non-Guarantees
- No cross-node linearizability under active partitions.
- No global transactional multi-key ordering.
- No bounded staleness if a network partition persists beyond $\text{PHANTOM\_MAX\_STALE\_MS}$.


## Amendment 2026-10-30 07:26 +05:30 - invalidation durability and staleness bound

Historical record: the text above is unchanged. This amendment qualifies statements that overstate what the implementation guarantees.

Statements qualified:

- Section 4.1 (Failure Model): "**Node Restarts:** Supported via clean sequence resync and cold-start catchup." Qualification: this holds for a restarted *receiver* that pulls missing history. It does not cover a restarted or crashed *issuer* that owed invalidations: those debts are lost, and a peer cannot detect that the issuer restarted (a new stream restarting at sequence 1 is ignored as duplicate or obsolete).
- Section 4.2 G2 (cross-node bound D_max <= 250 ms after a 204): holds only while the issuing gateway stays alive and for connected, reachable peers.
- Section 4.2 G3: "Staleness <= TTL_eff <= PHANTOM_MAX_STALE_MS". Qualified below.
- Section (Active Production Anti-Entropy Reconciliation): "Background periodic reconciliation loop (every 5 seconds)". The code runs the worker every 250 ms (`src/main.rs:370` mTLS cluster branch, `src/main.rs:378` insecure/partial branch). Whether it is effective is a separate question (claims ledger C6).

**Correct qualified guarantee (verified by code reading on this date; not reproduced by a test).**
- `204 No Content` on `DELETE` means: the local cache was purged first (`src/api/server.rs:1952`), and the canonical primary, or the acting primary on failover, acknowledged synchronously within the 150 ms leg (`src/api/server.rs:1957`, `src/dist/router.rs:1781-1789`). All other cluster members are invalidated asynchronously (bounded retry, then re-delivery by the repair worker) and that is guaranteed only while the issuing gateway process stays alive and within the ledger bounds (the source comment at `src/dist/router.rs:1786-1789` states this).
- Invalidations owed to peers that have not yet acknowledged live only in the issuing process's memory (broadcaster channel, per-peer queues, repair ledger, stream sequencers, replay ring). A crash, kill or SIGTERM of the issuer loses them: the shutdown path in `src/main.rs:484-490` drops the server and exits ("cache is in-memory, nothing to flush") and `src/main.rs` contains no call to the router `stop()`/drain functions. A restarted issuer starts with an empty ledger. A peer that had not applied the invalidation then keeps serving the stale entry until it expires, or until a later `DELETE` of the same key reaches it.
- The staleness bound in G3, "Staleness <= TTL_eff <= PHANTOM_MAX_STALE_MS", is **not true as written**. The origin's `s-maxage`, `max-age` or `Expires` sets the freshness lifetime with no gateway ceiling (`src/core/freshness.rs:263-276`), and `PHANTOM_MAX_STALE_MS` caps only the stale-serving window (`src/api/server.rs:1280`, `:1738`, `:1788`), not the freshness lifetime. The 60 s default applies only when the origin gives no freshness information; it is a literal `60_000` at `src/api/server.rs:1367` and `:1650` (`PHANTOM_DEFAULT_TTL_MS` is validated but not wired: `src/config.rs:116-117`).
- No test kills or restarts an issuer with a non-empty ledger. Evidence and remediation options: `docs/research/phantomcache_milestone_accountability_report.md` section 12.

---

## Status note (2026-10-30, owner-approved DEC-020; the text above, including the 07:26 amendment, is unchanged)

The 07:26 amendment already qualifies G3 ("Staleness <= TTL_eff <= PHANTOM_MAX_STALE_MS" is not true as written: the origin freshness lifetime has no gateway ceiling and the 24 h `PHANTOM_MAX_STALE_MS` caps only the stale-serving window) and the "every 5 seconds" anti-entropy interval (code: 250 ms). Additional corrections, checked against the current tree:

- **G1 name.** "Per-Node Linearizability" should be read as gateway-local read-your-delete; see the ADR-002 status note (INV-1) for the open races G4A-01 and G4A-08. No linearizability checker exists.
- **D_max.** The ADR states D_max <= 250 ms, while the distributed tests and the claims ledger use 50 ms, and the ledger records that the 50 ms figure is a fixed sleep and not a measured P99 (`docs/research/claims_ledger.md` C6 and section 5). Neither number is a validated bound; all evidence is loopback and debug profile.
- **Line numbers in the amendment** moved: the local purge is at `src/api/server.rs:1990` and the router call begins at `:1992` (amendment: `:1952`, `:1957`); the anti-entropy worker is started at `src/main.rs:375` and `:383` (amendment: `:370`, `:378`).
- **Cache unification.** In the mTLS cluster branch the local node shares the HTTP serving cache (`VirtualNode::new_with_cache`, `src/main.rs:332`); remote peers are address-only descriptors. "Option A1 ... Cache Unification" is therefore implemented for the local node only, experimental, and tested on loopback.
