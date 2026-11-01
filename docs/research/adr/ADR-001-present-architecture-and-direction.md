# ADR-001 — Present Architecture, Gaps, and Recommended Direction

**Status:** PROPOSED (awaiting owner approval). **Date:** 2026-10-28. **HEAD:** `456bb25` on `feat/publish-phantomcache-core`; 16 tracked files modified and 16 untracked entries pre-existed this work (list in `implementation_master_plan.md` §1). Nothing in `src/` or `tests/` was changed by the research pass.
**Evidence tags:** **[V]** I read the cited lines myself. **[A]** agent-reported, not re-read. **[I]** my inference from [V] code, not executed. Nothing here was executed except `cargo test --all-targets -j 1` (242 passed).

## 1. Present behavior (what the binary does)

### 1.1 Construction and ownership [V]
- `main.rs:65`: `let cache = Arc::new(PartitionedCache::new(8))` — the HTTP serving cache; passed to `ApiState.cache` (`main.rs:299-300`).
- Standalone mode: `main.rs:69-71` creates three `VirtualNode::new(1..3)` as dashboard/demo nodes. Each `VirtualNode::new` (`node.rs:38-42`) owns a **separate** `PartitionedCache::new(4)`.
- Clustered mode (`main.rs:182-269`, requires CA + cert + key + members file + `PHANTOM_CLUSTER_LISTEN_ADDR`): for each member, the local node is `VirtualNode::new(node_id)` + `start_tcp_service` (`main.rs:237-243`); each peer is `VirtualNode::new(m_id)` with an address and state forced to `Active` (`main.rs:253-256`). The router is built over **all members, including the local node** (`main.rs:262-264`).
- Insecure/no-TLS cluster flag (`main.rs:265-266`): router over the three addressless demo nodes (every DELETE will fail, [A] `router.rs:318-321`).
- No code links a node's `cache` to `ApiState.cache` (grep of `.cache` in `main.rs`: only `:300`) [V].

### 1.2 Request chains
**GET** (`get_cache_handler`, `server.rs:357-770`) [V by earlier reads; handler body not re-read in this pass]: authenticate → derive `CacheKey` → `state.cache.begin_fetch_atomic` (`cache.rs:494`) → Hit | Wait (coalesced) | Initiate (`fetch_origin_raw`, then `finalize_fetch`, `cache.rs:615`) | Rejected → telemetry `record`. It never reads `state.router` or any `NodeState`. `route_key`/`get_replicas`/`forward_key_over_tcp` are called only inside `router.rs` (`:445,:488,:514,:520,:716,:717`) and from tests [V grep].
**DELETE** (`delete_cache_handler`, `server.rs:776-962`) [V `:812-913`]:
1. build `CacheKey`, validate;
2. if `state.router` is `Some`: `router.invalidate_key_across_cluster(canonical_key, ExactKey)` (`router.rs:709`) → `route_key` (HRW primary among router nodes, excludes non-Active) → `send_invalidation_frame` (150 ms) → on primary success enqueue replicas (`get_replicas(key,2)`) to `ReplicaBroadcaster::enqueue` (`router.rs:194`, `try_send`); on primary failure mark Suspect and try the first eligible replica as acting primary;
3. on `outcome.status.is_success()` → `cache.delete(&cache_key)` then **204** (`server.rs:830-843`); on `Rejected` → 5xx and **no** local delete (`:844-869`); on Err (timeout/unreachable/no nodes) → local `cache.delete` ("hygiene") then **504** (`:871-908`);
4. if no router: local `cache.delete`, **204** (`:910-912`).
**Peer-side invalidation** (`node.rs:458-509`) [V]: `record_inbound_seq`, `ring_buffer.push` (the only push, `node.rs:475`), then `node.cache.try_delete_canonical / try_invalidate_tenant / try_invalidate_all` — i.e. the **node's own** cache.
**Peer GetRequest** (`node.rs:385-441`) [V]: if node is `Recovering`/`Suspect` → returns the literal `"MISS val_node_<id>"`; else `node.cache.begin_fetch_atomic`; on a miss it **fabricates** `val_node_<id>`, stores it for 60 s, replies `MISS`. No origin contact. No production sender (only `forward_key_over_tcp`, tests).
**Anti-entropy** [V code]: `reconcile_peer_anti_entropy` (`router.rs:787`) has no caller in `src/` (grep `reconcile`). It sets nodes Recovering/Active, sends a sync request carrying `highest_acked_seq` and the local ring digest.

### 1.3 Resource and invariant facts [V]
Cache: 8 shards (HTTP) × default 10,000 entries/shard (`cache.rs:191`); node caches 4 shards. Capacity counts entries; body cap 2 MiB at ingress of origin responses ([A] `MAX_BODY_BYTES`). TTL: 60,000 ms literal (`server.rs:638` [A], `node.rs:413` [V]); no `Cache-Control` parsing [A]. Global `tenant_epochs` mutex taken on each hit/miss path while the shard lock is held (`cache.rs:251-259, 313-346, 494-612`).

## 2. The eight questions

| # | Question | Answer | Evidence |
|---|---|---|---|
| 1 | GET on gateway A when gateway B receives an invalidation? | **Nothing changes.** A's GET is served from A's `ApiState.cache`. B's invalidation handler purges B's *node* cache, which B's own HTTP GETs do not read either. | §1.1, §1.2 [V] |
| 2 | What does DELETE→204 guarantee? | With a router: *this gateway's* HTTP entry for the key was removed **after** a primary node acknowledged the frame within 150 ms (ack responder id and membership epoch checked, `router.rs:684-694`). The acknowledging node purged its **node cache**, not any other gateway's HTTP cache. Replicas are best-effort. Without a router: local purge only. Concurrent GETs in the ≤~300 ms window can hit the old local entry (purge happens after the round trip). 504 also purges locally. | `server.rs:821-913` [V] |
| 3 | A replica misses an invalidation after retries are exhausted? | The broadcaster task ends silently after `0..=3` attempts (`router.rs:110`); `try_send` failure only increments `dropped_count` (`:194-201`). Counters are read nowhere else ([A]). The key is not retried, flagged, or recorded. | [V] code; "read nowhere" [A] |
| 4 | Can it detect/repair the stale replica, via which production path? | **No production path.** Reconciliation has no caller; the gap queue has no consumer; the sender never records outbound invalidations (`ring_buffer.push` only at `node.rs:475`, on receipt). The only repair is TTL expiry (60 s) or eviction. | grep `reconcile`, `pop_gap`; [V] |
| 5 | Does sequence-gap detection represent real per-peer history gaps? | **No.** `stream_seq` is one `AtomicU64` per router shared by all targets and incremented per attempt, including retries (`router.rs:142` inside the retry loop, `:615`, `:223`). Each receiver sees a subset of numbers, so nearly every invalidation looks like a gap. Receivers key by `originator_id`, which is 0 in insecure mode. | [V] |
| 5b | (new) If reconciliation ran, would it recover a detected gap? | **No, by construction.** `record_inbound_seq` advances `expected` to `stream_seq+1` on a gap (`anti_entropy.rs:215`) and `get_highest_acked_seq` returns `expected-1` (`:226-231`); reconciliation sends that as `highest_acked_seq` (`router.rs:806-810`), so the responder replays only records *after* the latest seen number and never the gap. `pop_gap` (which could drive a targeted replay) has no caller. | [V] reading; consequence [I] |
| 6 | A suspect node returns — can it serve stale data before reconciliation? | The `Suspect`/`Recovering` bypass exists only in the cluster `GetRequest` handler, which nothing calls in production (`node.rs:394-400`). The HTTP GET path ignores `NodeState` entirely; **a restarted gateway starts with an empty cache** (no persistence), and a gateway that never restarted keeps serving its HTTP cache regardless of state. Transitions: `Suspect` is set by routing failures; nothing returns it to `Active` except `/api/control`, listener start, or the uncalled reconcile ([A] `server.rs:328`, `node.rs:174`). | [V] for GET/bypass; transitions [A] |
| 7 | Actual consistency and staleness guarantees? | Per gateway: reads are served from a local cache with generation-fenced fills (a fill that races a local `delete` is dropped — `cache.rs:668-690`). Cross-gateway: **none**; maximum staleness is the 60 s TTL (or eviction), independent of any DELETE or partition. No linearizability, no bounded staleness beyond TTL, no read-your-writes across gateways. | §1, [V] |
| 8 | Which tests show these behaviors end to end vs only functions? | See §3. No test runs `main()`; none asserts that a second gateway's HTTP cache is purged; the DELETE-over-HTTP tests assert only status codes and headers. | §3 |

## 3. Test baseline classification (file-level; individual test bodies not all read)
Reproduced: **242 passed / 0 failed / 19 targets**, debug profile, `-j 1` [REPRO].
| Class | Targets | What they exercise |
|---|---|---|
| **HTTP path via `create_app` + `ApiState` built by hand** | `m1_http_tests` (32), `coalesce_tradeoffs_tests` (5), `security_phase1_tests` (15), `security_phase2_upstream_tests` (23), `p3_traffic_tests` (5), `p4_pie_tests` (6, partly), `wp4_tests` (5) | Real HTTP over loopback against the handlers; closest to production request paths. `ApiState` is assembled in each test, **not** by `main()`; all but `security_phase4b_router_tests` use `router: None` |
| **HTTP + router + loopback TCP node** | `security_phase4b_router_tests` (19), few cases | DELETE status/headers (e.g. `:417-492` asserts only 204/504 and the `x-cache-error` header). The gateway's `cache` and `node1`'s cache are separate objects; **no assertion that any HTTP cache entry is purged by a peer** |
| **Protocol/component over loopback sockets** | `p2_dist_network_tests` (7), `security_phase3_cluster_tests` (24; sec05–08 test config-parsing helpers, not `main()`), `security_phase3b_framing_tests` (13), `security_phase3c_pooling_tests` (11), `security_phase4a_invalidation_tests` (14), `security_phase4c_antientropy_tests` (18) | Codecs, pooling, mTLS, node handlers, `reconcile_peer_anti_entropy` called directly by the test |
| **Unit** | lib (16), bin (16, re-compiled modules; `main.rs` itself has no tests), `wp1–wp3_tests` | Data structures, simulation |
No target is a multi-gateway end-to-end test. The agent's count of 226 test attributes vs 242 reported passing was not reconciled.

## 4. Intended behavior (documents)
- `docs/security/phase-4-design.md`: "ephemeral caching tier", two-tier invalidation (primary sync + replicas async), anti-entropy via heartbeat digests, read-your-writes to the local gateway or primary, stale up to TTL on partitioned replicas.
- `docs/demo-lab/implementation-roadmap.md` Phase 6: wire HRW into the gateway for coordinated reads (never done).
- `docs/handoff/01,03,05,08`: self-healing anti-entropy, periodic probes, 15 s keepalive→Suspect, `primary_invalidation_timeouts`, "zero open P0".

## 5. Gaps (present vs intended)
G1 peer invalidations do not reach the serving cache; G2 no production reconciliation; G3 no outbound history; G4 global per-router sequence; G5 gap handling discards the gap; G6 lossy broadcast with no record and unbounded spawned tasks [A]; G7 sticky Suspect [A]; G8 DELETE ordering differs from the docs; G9 fabricated cluster GET; G10 documentation claims unsupported by code; G11 TTL is the only freshness control.

## 6. Options
| | A. Local caches + reliable invalidation fan-out | B. Coordinated reads via HRW | C. Trustworthy single-node first; cluster gated |
|---|---|---|---|
| Matches documented intent | **Yes** (Phase 4 design) | Roadmap Phase 6 | Neither; defers |
| What must be built | Share/route peer invalidations into `ApiState.cache`; record outbound history; per-peer sequence spaces; gap-aware replay; reconciliation trigger; honest metrics; bounded fan-out; recovery state machine; multi-gateway tests | A: all of the above *plus* request forwarding with headers/tenant/auth, loop prevention, hot-key amplification control, forwarded-failure semantics, a real node GET handler | Telemetry truth, byte accounting, hot-path fixes, baselines, controller on the single-node path; keep cluster code compiled and tested but documented as experimental |
| Guarantee achievable | Bounded staleness ≈ f(invalidation delivery, reconcile period, TTL) — **measurable** under fault injection | Stronger (single owner per key) but availability/latency cost; consistency still depends on invalidation of the owner | Single-node semantics only (strong, simple) |
| Correctness risk | Medium (distributed protocol) | High | Low |
| Performance | No extra hop on reads | Extra hop on most reads | Best |
| Research value | Measured-staleness / model-checked fencing (known ideas; assurance paper) | Larger design space, more novelty risk | Lets the origin-aware controller (the leading research thread) proceed independently |
| Cost | Large | Largest | Smallest |

## 7. Decision (recommended)
**Immediate: C. Long-term: A (conditional). B deferred.**
Reasons: (1) the leading research thread (origin-aware control) acts on the per-gateway origin-fetch path and does not need the cluster; (2) the cluster layer currently provides no end-to-end guarantee and its documentation overstates it, so correctness work on it is a larger, riskier investment that should follow measured baselines and a multi-gateway fault-injection harness; (3) A matches the documented intent and the smallest delta, so it is the natural target once the harness exists; B changes the product's semantics and should be motivated by measured need.
**Challenge to the stated preference:** C is right, with one amendment — do not "park" the cluster code silently. Do now, cheaply: (a) a characterization test that *demonstrates* G1 (ignored/expected-fail), (b) a correction note so no one relies on the false claims, (c) a decision gate (G-ARCH) after the baseline is in which A is specified in detail. Do not delete any distributed code.
**Consequences:** distributed claims are suspended in all documents until G-ARCH passes with multi-gateway evidence; C6 in `claims_ledger.md` stays unachievable.

## 8. Open items / not verified
Handler bodies of GET (`server.rs:357-770`) not re-read line by line in this pass (earlier reads and agent trace); `node.rs` transitions and broadcaster-counter usage are [A]; `security/*` and DER parsing not audited; transcripts give no recorded rationale for leaving GET unrouted; whether HRW selecting the *local* node as primary produces a TCP self-connection under mTLS was not traced.

---

## Status note (2026-10-30, owner-approved DEC-020; the ADR body above is unchanged and is a record of the tree at 456bb25 on 2026-10-28)

Re-checked against the current tree. Statements in the body that no longer hold:

- **DELETE ordering (the DELETE paragraph, question 2, gap G8).** `delete_cache_handler` now purges the local cache FIRST (`cache.delete(&cache_key)`, `src/api/server.rs:1990`) and only then calls the router (`:1992`, `invalidate_key_across_cluster`). The body's "local delete after the primary acknowledged" and "no local delete on Rejected" describe the older code. Verified in code by reading.
- **Peer invalidations and the serving cache (the peer-side invalidation paragraph, question 2, gap G1).** In the mTLS cluster branch the local node is built with `VirtualNode::new_with_cache(node_id, cache.clone())` (`src/main.rs:332`), so it shares the HTTP serving cache and invalidation frames received from peers purge it. Remote peers are address-only descriptors (`VirtualNode::new` plus `set_addr`, `src/main.rs:364-366`). The three demo nodes (`src/main.rs:212-214`) and the insecure or partial cluster branch use separate placeholder nodes without listeners. Whether a second real gateway's HTTP cache is purged end to end is tested only on loopback and is not established beyond that.
- **Anti-entropy has no production caller (the anti-entropy paragraph, gap G2).** `RendezvousRouter::start_anti_entropy_worker` is now called with a 250 ms period in both cluster branches (`src/main.rs:375`, `:383`) and calls `reconcile_peer_anti_entropy`. It runs only in cluster configurations; its effectiveness is not established (`docs/research/claims_ledger.md` C6).
- **Line numbers** in the body refer to the 456bb25 tree and are stale.
- **"Maximum staleness is the 60 s TTL" (question 7).** The 60 s value is a literal default used only when the origin supplies no freshness information (`PHANTOM_DEFAULT_TTL_MS` is not wired, `src/api/server.rs:1405`, `:1688`); origin `max-age` and similar headers set the lifetime with no gateway ceiling. See the ADR-003 amendment.
- Gateway-local read-your-delete is the strongest delete property established. This ADR makes no linearizability claim and none should be inferred.
