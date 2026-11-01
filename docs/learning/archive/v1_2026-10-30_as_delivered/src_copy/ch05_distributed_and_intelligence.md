# Chapter 5: Distributed Systems and Workload Intelligence

This chapter has two halves. The first half explains how several PhantomCache gateways cooperate: who is a member, how they talk, how a key is mapped to a node, and what happens to an invalidation when things break. The second half explains the workload-intelligence code: telemetry, probabilistic data structures, and the adaptive algorithms. In both halves the main skill is the same. You must be able to say exactly what the code does today, what only exists as a library, and what is not guaranteed.

## What you will learn

- How the cluster is wired: static membership, mutual TLS, the binary frame protocol, connection pooling, and what the "heartbeat" really does.
- How rendezvous hashing works (with the maths and real numbers), and the precise, narrow job it does in this code base: it picks the node that gives the synchronous acknowledgement for a DELETE. It does not place data and it does not route reads.
- How an invalidation travels: local purge, primary acknowledgement, asynchronous fan-out, the repair ledger, anti-entropy, and the three different "epochs" that are easy to confuse.
- What consistency PhantomCache can honestly claim, under which failure model, and which cases (an issuer crash, a long partition) have no bound and no test.
- What the "intelligence" is: a telemetry histogram and one small adaptive controller (AIMD) on the serving path, plus a library of detectors and evaluators that are not wired in, plus a legacy stub that is not the intelligence. The sealed experiment (M6) found no benefit from adaptivity.

**Status words.** Every capability below carries one of the labels from the style rules: IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, DEFERRED, BLOCKED, UNVERIFIED. "INDEPENDENTLY VERIFIED" is not used, because no human or external review of this code exists. The G4 reviews quoted here are AI second looks.

**How this chapter was checked.** Code statements were read in the working tree on 2026-10-30 (branch `feat/publish-phantomcache-core`; HEAD `df68eaa` at the time of writing; large uncommitted tree). Older documents were compared with that code. `docs/learning/evidence/ch05_evidence.md` lists each claim, its source and how it was verified. The only thing executed was a tiny standalone program that replicates the router's hash formula (section 5.3).

---

## 5.1 Distributed caching fundamentals, and the choice PhantomCache made

### Simple explanation

One cache in one process is fast and simple. When traffic grows, or you want to survive the loss of a machine, you run several caches. Now three questions appear that did not exist before.

1. **Placement.** Which cache holds which key?
2. **Coherence.** If the data changes, how do all caches learn it?
3. **Failure.** What happens when a cache, or the network between caches, breaks?

Systems answer these differently. Memcached clients and Redis Cluster *partition* the key space: each key lives on exactly one node, chosen by a hash. A read for key `k` is sent to the owner. There is one copy, so coherence is easy, but a lost node loses its keys. Content-delivery caches usually do the opposite: every edge keeps its own copy of whatever it served, and the operator *purges* the key everywhere when the origin changes. That is cheap to read (always local) and hard to keep coherent.

### The correct terms

- **Partitioned (sharded) cache:** each key has one owner; reads are routed to it.
- **Replicated or local-copy cache:** several nodes may hold the same key; invalidation must reach all of them.
- **Invalidation:** removing or marking stale a cached entry because the source of truth changed.
- **Fan-out:** sending one message to many receivers.
- **Anti-entropy:** a background process that compares state between nodes and repairs differences (the term comes from database replication, for example Cassandra and Dynamo-style systems).
- **Failure model:** a written list of the faults you claim to survive (crash, message loss, partition, slow node, malicious node) and the ones you do not.

### Where it lives in PhantomCache

PhantomCache chose the second design. Verified in code: a GET is answered from the gateway's own cache and, on a miss, from the origin. It is never forwarded to another gateway (`src/api/server.rs::get_cache_handler` uses `state.cache`; `RendezvousRouter::forward_key_over_tcp` has no caller outside tests, verified by grep over `src/`). The cluster layer exists to do one job: when a client sends DELETE for a key on any gateway, make every gateway drop its copy. The recorded decision is ADR-003 (Option A1, "Reliable Invalidation Fan-Out + Unified Cache" as the test header phrases it; `docs/research/adr/ADR-003-distributed-architecture.md`). IMPLEMENTED; TESTED (sections 5.5 and 5.6 give what the tests prove).

An earlier investigation (`docs/research/architecture_investigation.md`, 2026-10-28) found that the cluster protocol was "a protocol library and a demo" that did not touch the serving cache. That is **out of date** for the mTLS cluster path. The table below lists what changed, because a judge who has read the old document may quote it.

| Finding in the 2026-10-28 investigation | Current code (read in this tree) |
|---|---|
| A1: peer invalidations purge a side cache, not the HTTP cache | In the mTLS cluster branch the local node is built with `VirtualNode::new_with_cache(node_id, cache.clone())` (`src/main.rs`), so the cluster listener purges the same `PartitionedCache` that serves HTTP. Remote peers are address-only descriptors. |
| A5: anti-entropy has no production caller | `main.rs` calls `RendezvousRouter::start_anti_entropy_worker` (250 ms) in both cluster branches. |
| A6: outbound invalidations never recorded | `send_invalidation_frame_with_seq` and the broadcaster push each outbound record into the local ring (`push_outbound`). |
| A7: one shared sequence counter, false gaps | `DestinationSequencer` keeps one counter per destination node. |
| A9: missed fan-out lost without record | `RepairLedger` records every undelivered invalidation (bounded). |
| A11: local purge after the cluster round trip | `delete_cache_handler` calls `cache.delete` first, then the cluster. |
| A3: GET is local only | **Still true.** |
| A4: peer `GetRequest` fabricates data | **Still true** (section 5.8). |
| A10/G4B-12: heartbeat never changes node state | **Still true** (section 5.2). |

The project note in `CLAUDE.md` section 6 carries the same "partly outdated" warning; the startup banner in `main.rs` was updated to describe the shared cache correctly.

### Trade-offs

Local copies make reads fast and make a node loss harmless to read availability: any gateway can serve any key. The price is coherence. Staleness is possible on every gateway that missed an invalidation, and the system has no read path that could notice. This is the central trade-off of the chapter. (Whether partitioning would have been "better" is our reasoning, not a recorded decision; ADR-003 records why fan-out was chosen, not a comparison against a partitioned design for this exact workload.)

---

## 5.2 Cluster membership and node-to-node communication

### Membership is a static file

A cluster is defined by a JSON file named in `PHANTOM_CLUSTER_MEMBERS_FILE`. It lists the cluster id, a `version`, and for each member: node id (1 to 255, a `u8`), an `active` flag, and the SHA-256 fingerprints of the certificates that member may use. `src/main.rs` loads it once (`security::load_membership_from_file`). The comment in `src/security/cluster.rs` is explicit: "updates to the file require process restart to reload."

`ClusterMembershipRegistry::update` exists. It enforces that the new `version` is strictly greater and sends a revocation message for members that disappear. But a search of `src/` finds no production caller; only tests call it (`tests/integration/security_phase3_cluster_tests.rs`, `m7_security_review_tests.rs`). So **revocation machinery exists and is TESTED, but has no production trigger**. This is finding G4B-07, an owner-named OPEN item in the disposition register. The accurate sentence for a judge is: "membership is static; changing it means editing the file and restarting every node."

The `version` field is what the code calls the **membership epoch**. It is not a failure detector and it does not change at runtime.

### Node states, and which are real

`NodeState` (in `src/dist/node.rs`) has seven values: `Joining`, `Active`, `Degraded`, `Suspect`, `Failed`, `Offline`, `Recovering`. What actually happens to them:

| State | Who sets it | Notes |
|---|---|---|
| `Joining` | `VirtualNode::new` | Initial value; `start_tcp_service` and `main.rs` set `Active`. |
| `Active` | startup; repair/pull workers after reconciliation | Peers in a configured cluster start `Active`. |
| `Degraded` | nobody | Declared, never assigned (grep of `src/`). |
| `Suspect` | data-path failures in the router: connect error, timeout, bad ack, wrong responder | Excluded from `route_key` Pass 1 and from `get_replicas`. |
| `Recovering` | `repair_peer` after a Suspect peer's debts drain; `reconcile_peer_anti_entropy` | Not yet routable; becomes `Active` after a pull reconciliation. |
| `Failed`, `Offline` | `POST /api/control {"event":"NODE_FAILURE"}` (hard-coded to node 2); nothing else | See below. |

`NODE_FAILURE` is a **logical flag**, not a failure. `control_handler` in `src/api/server.rs` does `n.set_state(NodeState::Failed)` for the node with id 2. In a real cluster that node object is the live router entry, so the flag changes routing and makes the router refuse to connect to that peer (`connect_to_node_inner` returns `NodeUnreachable` for `Failed`/`Offline`). Any other event string, including a missing one, sets every node `Active`, silently clearing `Suspect` and `Recovering`. This is finding G4B-03 (OPEN, owner-named). It is a simulation hook that is live-wired. In demo mode it is how the UI "kills" a node; no process dies.

A listener whose own state is `Failed` or `Offline` drops every inbound socket (`start_tcp_service` accept loop), which is how the tests simulate a partition.

### Heartbeats do not detect failure

The connection pool sends a `HeartbeatRequest` to every *idle* pooled connection every 5 seconds (`PoolConfig::heartbeat_interval`, `TlsConnectionPool::probe_idle_connections`). A failed probe discards that connection. **Nothing in `pool.rs` reads or writes a `NodeState`.** A peer that died while idle stays `Active` until a real invalidation call to it fails. The first such call pays its full 150 ms deadline. This is G4B-12 (Medium, OPEN). The heartbeat's real value is to keep idle TCP connections alive and to throw away dead ones before they are used. Document it as "connection keepalive", not "failure detection". Status: IMPLEMENTED (as keepalive), TESTED in `security_phase3c_pooling_tests.rs`; failure detection DESIGNED BUT NOT IMPLEMENTED.

A further caution: because the heartbeat succeeds on a connection that is merely *open*, it also refreshes the pool's idle timer. A pooled connection can therefore live as long as the peer keeps answering, so certificate expiry or rotation is enforced only at handshake time (G4B-08, OPEN, owner-named).

### The wire protocol: PhantomMesh frames

Defined in `src/dist/frame.rs`. Every message is a fixed 28-byte header followed by a payload of at most 1,048,576 bytes.

```
magic "PHAN" (4) | version (1) | msg_type (1) | flags (2) | request_id (8)
| sequence_number (8) | payload_length (4)        -- all big-endian
```

Message types: `GetRequest/GetResponse`, `InvalidateRequest/InvalidateAck`, `HeartbeatRequest/Response`, `AntiEntropySyncRequest/Response`. Keys are at most 4,096 bytes. An `InvalidateRequest` carries: membership epoch (u64), stream sequence (u64), originator node id (u8), scope (`ExactKey`, `Tenant` or `FullCache`) and the key. The ack carries the responder id, a status (`SuccessEvicted`, `SuccessNotPresent`, `IgnoredObsolete`, `RejectedRateLimit`, `InternalError`) and the reclaimed bytes. Status: IMPLEMENTED; TESTED (`security_phase3b_framing_tests.rs`, codec tests in `security_phase4c_antientropy_tests.rs`). What these tests prove: encode/decode round trips and rejection of malformed headers and oversize payloads. They do not prove protocol behaviour under load.

Note the two different sequence numbers. The header's `sequence_number` is per *connection* and is just a counter. The payload's `stream_seq` is per *destination node* and is what anti-entropy reasons about. Section 5.5 uses the second.

### Admission: the mTLS handshake

![Figure 5.1: Cluster topology. Three gateways share nothing except a full mesh of mutually authenticated TLS links; reads never use the mesh.](../assets/ch05_cluster_topology.svg)

Source: `src/main.rs` (cluster branch), `src/dist/node.rs::start_tcp_service`, `src/dist/router.rs::RendezvousRouter`.

![Figure 5.2: How a connection from gateway A to gateway B is admitted.](../assets/ch05_mtls_handshake.svg)

Source: `src/dist/router.rs::connect_to_node_inner`, `src/dist/node.rs::start_tcp_service` and `process_binary_cache_stream`, `src/security/cluster.rs::ClusterTlsConfig::from_pem` and `validate_peer`.

Key points, verified in code (the trust analysis is Chapter 6, section 6.4):

- TLS 1.3 only, both directions. The only trust root is the cluster CA (the web PKI is not loaded); the listener requires a client certificate (`WebPkiClientVerifier`).
- Identity is the certificate's single DNS Subject Alternative Name `node-<id>.<cluster_id>.phantom.internal`; zero or several SANs are rejected. Authorisation is *pinning*: the registry must list that node id as active with a fingerprint matching the leaf certificate, so a valid CA-signed but unpinned certificate is refused.
- Limits: 64 handshake permits and a 1,000 ms handshake timeout on the listener; at most 8 pooled connections per peer (4 idle). 64 silent connections can occupy all permits (G4B-10, OPEN).
- Every frame re-checks the peer against the registry; an `InvalidateRequest` whose `originator_id` differs from the TLS identity closes the connection (`test_l3_invalidate_request_originator_must_match_tls_identity`).
- With no cluster TLS a listener may bind only to loopback (fails closed). A plaintext loopback mode for tests skips the originator check (G4B-17, Low).

The security chapter (Chapter 6) owns the trust analysis. For this chapter the point is architectural: a member is *trusted*. Any pinned member can send `Tenant` or `FullCache` scope invalidations that flush other nodes' caches (G4B-05, DR-027, OPEN, owner-named). The cluster is a closed trust domain, not a hostile network.

### What runs in the other, non-mTLS configurations

Without cluster certificates but with `PHANTOM_ALLOW_INSECURE_CLUSTER` (or a members file but no certs), `main.rs` builds a router over three *placeholder* nodes with no addresses and no listeners. No frame is sent. A DELETE then purges the local cache and reports `504 PrimaryInvalidationTimeout`. With no cluster configuration at all, `router` is `None` and DELETE is purely local. The three demo `VirtualNode`s (ids 1 to 3) with separate caches exist for the simulation endpoints. Status: IMPLEMENTED; the insecure branch is labelled EXPERIMENTAL by its own startup message.

::: judge
**JUDGE MAY ASK**

Question: How do your nodes discover each other and communicate? What if I add a node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Membership is static. A JSON file lists every node, its id and the fingerprint of its certificate. Each gateway reads it at startup. Nodes talk over mutually authenticated TLS 1.3 with a small binary protocol; both sides check the certificate against the cluster CA and against the pinned fingerprint. There is no gossip and no dynamic join. To add a node you edit the file, bump its version and restart the cluster members. A runtime reload and revocation path exists in the library and is tested, but nothing in the shipped gateway triggers it. That is an open finding, G4B-07, and I would not claim dynamic membership.

**If the judge asks further:**

"Why not gossip or a consensus library?" Our reasoning, not a recorded decision: the cluster only needs to deliver invalidations to a small, known set, and static membership with pinned certificates is the simplest thing that gives strong identity. It limits operations (no rolling add without restart) and that is a stated limitation.

**Evidence to know:** `src/security/cluster.rs::ClusterMembershipRegistry::update` (no production caller), `src/main.rs` cluster branch, G4B-07 in `docs/release/g4/cycle1_findings_disposition_register.md`. Status: static membership IMPLEMENTED and TESTED; dynamic membership DESIGNED BUT NOT IMPLEMENTED.
:::

::: judge
**JUDGE MAY ASK**

Question: You have heartbeats. So does the cluster detect a dead node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Not by heartbeat. The heartbeat keeps idle connections alive every five seconds and discards dead ones, but it does not change any node's state. A node becomes Suspect when a real call to it fails, usually the first invalidation after it died, which then spends its 150 millisecond deadline. After that the node is skipped for routing and its owed invalidations go to a repair ledger. So detection is lazy and driven by traffic. If I had more time, wiring heartbeat results into the state machine is the obvious fix; it is recorded as finding G4B-12.

**If the judge asks further:**

"Does the UI show a failed node?" The UI can show a node as Failed after the demo control endpoint sets a flag. That is a logical flag on a router entry, not a detected failure, and the dashboard labels node state as inferred, not measured.

**Evidence to know:** `src/dist/pool.rs::probe_idle_connections` (no `NodeState` access), `src/api/server.rs::control_handler`, G4B-12 and G4B-03. Status: failure detection by heartbeat is not implemented.
:::

---

## 5.3 Rendezvous hashing

### Simple explanation

Imagine each node holds up a numbered ticket for every key, and the key goes to whoever's number is highest. Everyone computes the same numbers, so everyone agrees on the winner without talking. If the winner leaves, the second-highest takes over, and no other key changes hands. That is **rendezvous hashing**, also called **highest random weight (HRW)** hashing (Thaler and Ravishankar, 1998; cited from memory here, not re-verified against the paper in this session).

### The maths

Let the nodes be `n_1 ... n_N` and let `h(k, n)` be a hash of key `k` combined with node `n`, uniform over `[0, 2^64)`. Define the score `s(k, n) = h(k, n) / 2^64`, a number in `[0, 1)`. The **owner** of `k` is `argmax_n s(k, n)`.

*Why it is balanced.* The `N` scores are independent and identically distributed, so each node is the maximum with probability `1/N`.

*Why it is stable.* Remove node `x`. A key's owner changes only if `x` was its maximum, because removing a non-maximum cannot change the argmax. So exactly the keys owned by `x` move, each to its second-highest node, and the rest stay. Adding a node `y` moves exactly those keys for which `y` has the new highest score, about `1/(N+1)` of all keys. This is the **minimal disruption** property, and it needs no ring or virtual nodes.

*Cost.* One hash per node per lookup: `O(N)` time, `O(1)` extra space. With `N <= 255` (ids are `u8`) this is negligible.

*Weighted version.* To give node `i` a capacity weight `w_i`, the standard construction (from the weighted-rendezvous literature, for example the "logarithmic method"; stated from memory here, unverified against a primary source in this session) is

```
score_i = -w_i / ln(u_i),   u_i = h(k, n_i) / 2^64  in (0,1)
owner   = argmax_i score_i
```

Why this works: `-ln(u_i) / w_i` is an exponential random variable with rate `w_i`. The *smallest* of independent exponentials with rates `w_i` belongs to node `i` with probability `w_i / sum(w)`. Taking the argmax of `-w_i/ln(u_i)` is the same as taking the argmin of `-ln(u_i)/w_i`. So a node with weight 2 owns twice the keys of a node with weight 1, and the minimal-disruption property is kept.

A tempting shortcut is to multiply the raw hash by the weight, `w_i * u_i`. That is **not** proportional. With weights 1 and 2, node 2 wins when `2*u_2 > u_1`, which has probability 1 - 1/4 = 0.75, not 2/3 (checked by the QA pass: a 2,000,000-draw simulation gave 0.7499 for the shortcut and 0.6668 for the logarithmic form). This matters below.

### Worked example (real numbers)

![Figure 5.3: Four keys, three nodes, real hash outputs, and what happens when node 3 is removed.](../assets/ch05_rendezvous_example.svg)

Source: `src/dist/router.rs::calculate_hash` and `route_key`; numbers produced by a standalone replica of that formula (reproduced here).

Read the first row. For key `__default__:/a` the three scores are 0.0712 (node 1), 0.3297 (node 2) and 0.0044 (node 3). Node 2 has the highest score, so node 2 is the owner. For `/b` the scores are 0.5482, 0.6587, 0.8331: node 3 owns it. If node 3 is excluded, `/b` falls to node 2 (0.6587) and `/hotel/42` falls to node 1 (0.0584, the higher of its two remaining scores), while `/a` and `/c` do not move. That is the minimal-disruption property in four lines.

::: example
**Measured on a replica, not on the product.** A standalone program (compiled with rustc 1.98.0 on this Windows host, in the session scratchpad) re-implements the router's formula exactly: `DefaultHasher::new()`, hash the key string, hash the node id byte, convert the `u64` to `f64`, multiply by 1.0, take the strict maximum. The QA pass re-compiled and re-ran the same formula (rustc 1.98.0, same host; an AI check, not independent verification) and reproduced every number below and the four-key scores in Figure 5.3 exactly. Over 100,000 keys of the form `__default__:/item/N`:

- 3 nodes own 33,287 / 33,428 / 33,285 keys (ideal 33,333).
- 5 nodes own 20,013 / 19,959 / 20,062 / 19,900 / 20,066 (ideal 20,000).
- Removing node 3 moves 33,285 keys, which is exactly node 3's share; keys moving between nodes 1 and 2: 0.
- Adding node 4 moves 25,002 keys (about 1/4), all of them to node 4.

Limits: one key family, one compiler, one machine; the product has no test for uniformity or minimal disruption (grep of `tests/`), so this replica is the only evidence. It supports the claim "the formula is balanced and minimally disruptive on this key family"; it does not support any claim about other key shapes.
:::

### What the code actually does

`RendezvousRouter::route_key` in `src/dist/router.rs`:

```rust
let hash_val = Self::calculate_hash(key, node.node_id);   // DefaultHasher over (key, node_id)
let capacity_weight = 1.0;
let weight = (hash_val as f64) * capacity_weight;
if weight > highest_weight { ... }                        // strict >, initial 0.0
```

Precisely:

1. **The weight is the constant 1.0.** The code is *plain* rendezvous hashing with the structure of a weighted one. There is no configuration key, node attribute or test for a non-constant weight. If a developer replaced `1.0` by a real capacity, the multiplication would be the non-proportional shortcut above, not the logarithmic form. Say "rendezvous hashing with equal weights; the weighted variant is not implemented". IMPLEMENTED (equal weights); weighted: DESIGNED BUT NOT IMPLEMENTED.
2. **The hash is `std::collections::hash_map::DefaultHasher`** (SipHash-1-3 with fixed zero keys in current Rust). The standard library documents that this algorithm is unspecified and may change between releases (the earlier agent note in `architecture_investigation.md` records the same concern). Two gateways built with different compiler versions could disagree about the owner. Since every gateway also fans out to all members, a disagreement changes only *which* node gives the synchronous ack, not whether others are invalidated, so the damage is small, but it is a real portability risk. A fixed, specified hash would remove it (our reasoning).
3. **Two passes.** Pass 1 skips `Failed`, `Offline`, `Suspect` and `Recovering`. Pass 2 runs only if Pass 1 found nobody, and then allows `Suspect` and `Recovering`. So a Suspect node is effectively removed from ownership until it recovers.
4. **Each gateway uses its own view of node states.** Two gateways may disagree on the owner of a key while their views differ (G4B-09).
5. **`u64 -> f64` loses the low 11 bits** (53-bit mantissa), so two nodes with different hashes can compare equal; the strict `>` then keeps the first node in the list. The effect on balance is negligible. A hash of exactly 0 can never win against the initial `0.0`; probability 2^-64.
6. **`get_replicas(key, n)`** returns the next `n` nodes in descending score order, skipping unhealthy ones. The router uses `n = 2`.

### Which operations are routed

Verified by reading every caller:

| Operation | Uses `route_key`? | Notes |
|---|---|---|
| HTTP `GET` | **No** | Local cache, then origin. |
| HTTP `DELETE` | **Yes**, for the synchronous leg | `invalidate_key_across_cluster`: owner first, then the first healthy replica as "acting primary". Everyone else is invalidated asynchronously. |
| Peer `GetRequest` (`forward_key_over_tcp`) | Yes, but | No production caller; used by tests only. |
| Anti-entropy, repair, pre-warm | No | They iterate over members. |

So rendezvous hashing decides **which node gives the acknowledgement that lets a DELETE return 204**. It does **not** decide where data lives. Every gateway caches whatever it served, whatever the hash says. There is therefore no data to rebalance when membership changes: a membership change alters only which node is "primary" for each key's DELETE. This is a different (and much weaker) role than hashing plays in Redis Cluster or Memcached client libraries, and a judge who assumes a partitioned cache will misjudge it. Say so before they do.

::: caution
The primary can be the issuing gateway itself. `route_key` iterates over *all* members including the local node, and the pre-warm worker even keeps a connection to the local node for this reason. For roughly `1/N` of keys the "primary acknowledgement" that releases the 204 comes from the issuer's own listener over loopback mTLS. In that case a 204 contains no evidence about any other node. See section 5.6.
:::

### Alternatives (our reasoning, not a recorded decision)

*Consistent hashing with a ring and virtual nodes* gives the same minimal disruption with `O(log N)` lookup but needs a ring structure, virtual-node tuning and a stable hash. *Jump consistent hash* handles only node lists that grow and shrink at the end, which does not fit arbitrary node ids. With `N <= 255` and a lookup used only on DELETE, rendezvous hashing's `O(N)` cost is irrelevant and its code is a dozen lines; that is a sound reason to prefer it, but the repository does not record it as the reason.

::: judge
**JUDGE MAY ASK**

Question: You say you use weighted rendezvous hashing. Walk me through it. How does the weight enter, and what does the system do with it?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Each node gets a score for each key from a hash of the key and the node id, and the highest score owns the key. Everyone computes the same scores, so no coordination is needed, and removing a node moves only that node's keys. In weighted form you use minus weight over the natural log of the normalised hash, which makes a node win in proportion to its weight. In our code, honestly, the weight is a constant 1.0, so it is plain rendezvous hashing. I would not claim capacity-aware placement. And the result is only used to choose which node gives the synchronous acknowledgement for a delete. Reads are served from the local cache and are not routed.

**If the judge asks further:**

"Why does that matter?" Because it means no data is placed or rebalanced by the hash. "What would break if you set weights?" The code multiplies the raw hash by the weight, which is not proportional; you would need the logarithmic form. I checked that with a quick calculation: weights 1 and 2 give a 75 percent share to the heavier node, not 67 percent.

**Evidence to know:** `src/dist/router.rs::route_key` (`capacity_weight = 1.0`), `get_replicas`, callers of `route_key` (only `invalidate_key_across_cluster` in production). Replica measurement in section 5.3 (reproduced here, standalone). Status: equal-weight rendezvous IMPLEMENTED; weighted rendezvous DESIGNED BUT NOT IMPLEMENTED; no uniformity test in the repo.
:::

::: judge
**JUDGE MAY ASK**

Question: Walk me through a read. Which node serves it, and where does the hash come in?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The gateway the client contacted serves it. It looks in its own cache; on a miss it fetches from the origin, with single-flight so concurrent misses share one fetch, and stores the result with whatever freshness the origin allowed. No other gateway is consulted, so the hash is not involved. The hash matters for deletes: it picks the node that must acknowledge before the delete returns.

**If the judge asks further:**

"So the same key can be cached on every node?" Yes. That is the design: local copies, with invalidation fan-out to keep them coherent. The cost is that a node that missed an invalidation can serve a stale copy, which is the subject of the next sections.

**Evidence to know:** `src/api/server.rs::get_cache_handler`, `RendezvousRouter::forward_key_over_tcp` (no production caller), ADR-003. Status: local-only GET IMPLEMENTED and TESTED (`known_gaps.rs`, `distributed_consistency_tests.rs`).
:::

---

## 5.4 Epochs and versioning across nodes

The word "epoch" names three unrelated counters in this code base. Mixing them up is the most common way to give a wrong answer in a defence, so keep them apart.

| Counter | Where | Scope | Changes when | What it protects |
|---|---|---|---|---|
| **Tenant epoch** and **global epoch** | `PartitionedCache` (`tenant_epochs`, `epoch_snapshot`, `global_epoch`) | One gateway's cache | `try_invalidate_tenant`, `try_invalidate_all` | A cached entry is stamped with the epochs at fill time; if either differs from the current value the entry is stale |
| **Membership epoch** | `ClusterMembershipRegistry.version` (`get_membership_epoch`) | The whole cluster, configured | Never at runtime (static file) | Intended: reject messages from a node with an obsolete view of who is a member |
| **Stream sequence** (`stream_seq`) | `DestinationSequencer`, `AntiEntropyTracker`, ring buffer | One *directed* pair (sender, receiver) | Every invalidation sent to that receiver | Detect lost or reordered invalidations and replay them |

Per-key **generation** (Chapter 4) is a fourth counter, local to a key's cache state; `delete` bumps it so that an in-flight origin fetch cannot store its result.

### Tenant and global epochs: a worked example

Entries carry `entry_tenant_epoch` and `entry_global_epoch`. A read counts as a hit only if both equal the current values (`get_effective_epochs`, which reads a snapshot `Arc<HashMap>` under an `RwLock`, so the read path takes no exclusive mutex).

::: example
Tenant `acme` has tenant epoch 0 and the global epoch is 0. Key `/a` is filled and stamped (0, 0).

1. A `Tenant` invalidation for `acme` arrives. `try_invalidate_tenant("acme")` makes the tenant epoch 1. The next read of `/a` compares stamp (0, 0) with current (1, 0): different, so the entry is treated as stale and refetched. Nothing was scanned or deleted; a counter moved.
2. A `FullCache` invalidation arrives. `try_invalidate_all` makes the global epoch 1 and clears the tenant map (every tenant epoch reads as 0 again). Old entries carry global epoch 0, which no longer matches, so they cannot come back to life even though tenant counters were reset. New fills are stamped (0, 1).
3. If more than `MAX_TENANT_EPOCHS = 10,000` distinct tenants are invalidated, the cache falls back to a global bump and clears the map (`SEC-4A-01`). Memory stays bounded and no stale entry can be resurrected.
:::

Status: IMPLEMENTED; TESTED (`security_phase4a_invalidation_tests.rs`, `epoch_contention_tests.rs`). Open items owned by Chapter 4: the epochs are re-read inside `finalize_fetch` after the fencing check, so a pre-invalidation body can be stamped with a newer epoch (G4A-01, UNDER-INVESTIGATION, reproduced only with an injected yield point in a sandbox; a patch of about four changed lines is proposed and not applied (`docs/research/g4a01_race/proposal.patch`)); and tenant and global epochs can be observed skewed (G4A-17). I did not re-derive these.

### What epochs do across nodes

Here is the important and easily missed fact: **the DELETE API only ever sends the `ExactKey` scope.** `Tenant` and `FullCache` frames are produced inside the distribution code, as an escalation: when a peer's repair backlog overflows (many keys of one tenant collapse into a tenant flush; too many tenants collapse into a full flush), when a single entry cannot be delivered permanently, or when a revoked member's debts are folded into one flush. `grep` confirms no HTTP handler constructs those scopes. So the tenant and global epochs matter across nodes only on those repair paths. An exact-key delete on a receiver is a `try_delete_canonical`, which bumps the key's generation and detaches any in-flight fetch.

The **membership epoch** is carried in every invalidation and echoed in the ack. The issuer rejects an ack whose epoch differs from its own (`IgnoredObsolete`). But the receiver echoes the *sender's* epoch without comparing it with its own registry (the `InvalidateRequest` handler in `node.rs` has no epoch check; the anti-entropy handler does), so the issuer-side comparison can differ only if the issuer's own registry changed while the request was in flight. The broadcaster path does not compare the ack epoch at all. Because the registry never changes at runtime, this has little practical effect today, but the comment "SEC-4B-05: validate responder identity and membership epoch" overstates what is enforced. This is G4B-06, OPEN, owner-named. Status: epoch carried and echoed IMPLEMENTED; epoch *enforcement* on invalidations NOT implemented.

The **stream sequence** is the useful cross-node version number. Section 5.5 shows how it turns "a message might have been lost" into something a receiver can detect.

::: judge
**JUDGE MAY ASK**

Question: You mention epochs. What is an epoch here, and does it give you a global version of the cache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There is no global version. Inside one gateway, a tenant epoch and a global epoch are counters; each cached entry remembers the counters it was filled under, and bumping a counter makes all older entries instantly stale without scanning. Between gateways we do not synchronise those counters. What crosses the wire for the normal delete is the exact key, plus a per-destination sequence number so the receiver can notice a lost message, and the cluster membership version. The membership version is carried but not enforced on invalidations yet, which is an open finding.

**If the judge asks further:**

"Why can a tenant counter not simply be shared?" Because gateways have independent caches and independent fill times; a shared counter would need agreement, which we do not have. The sequence number plus replay is the lightweight alternative.

**Evidence to know:** `src/core/cache.rs::try_invalidate_tenant`, `try_invalidate_all`, `get_effective_epochs`; `src/dist/node.rs` InvalidateRequest handler; G4B-06, G4A-01. Status: local epochs IMPLEMENTED and TESTED; cross-node epoch enforcement not implemented.
:::

---

## 5.5 Invalidation propagation

### Simple explanation

When a client deletes a key, the gateway that received the request does three things in order. It removes its own copy at once. It asks one chosen node to confirm it removed its copy, and only then tells the client "done". Then, in the background, it tells everyone else, retries if someone does not answer, and writes down any debt it cannot pay yet so that a repair loop keeps trying.

### The path, step by step

![Figure 5.4: DELETE fan-out. Steps 1 to 3 are synchronous; 4 to 8 are asynchronous and live in the issuer's memory.](../assets/ch05_invalidation_fanout.svg)

Source: `src/api/server.rs::delete_cache_handler`, `src/dist/router.rs` (`invalidate_key_across_cluster`, `fanout_targets`, `dispatch_fanout`, `ReplicaBroadcaster`, `RepairLedger`, `repair_peer`).

1. **Local purge first** (`cache.delete`). From this instant, reads on this gateway miss. This is the basis of the per-node guarantee (section 5.6).
2. **Synchronous leg.** `route_key` picks the primary. The gateway sends one `InvalidateRequest` and waits at most 150 ms (`PHANTOM_CLUSTER_TIMEOUT_MS`, default 150; one deadline covers a possible retry on a fresh connection). If the primary is unreachable, it is marked `Suspect` and the first healthy replica from `get_replicas` becomes the *acting primary* (the response carries `x-cache-warning: PrimaryFailover`). A `Rejected` result does not fail over.
3. **Acknowledgement.** If the primary (or acting primary) acks with a success status, the HTTP reply is 204. If it fails, the reply is 504 or 5xx and **every other member is recorded as owed** (`record_owed_to_all`), because the origin changed regardless of whether the cluster could be told.
4. **Fan-out to the rest.** `fanout_targets` selects *all* members except the primary and the local node. This is deliberate (finding S-3): every gateway caches what it serves, so a fan-out to "the primary and two replicas" would leave other gateways stale in clusters of more than three. Healthy members go to the broadcaster; members in `Suspect`, `Recovering`, `Failed` or `Offline` state go straight to the ledger.
5. **Broadcaster.** One worker task per destination (so a blackholed peer ties up one task and its own queue of 256, not everyone), a global limit of 16 concurrent send attempts, and up to four attempts per job with backoff 25, 50, 100 ms and 150 ms bounds on connect, write and read. The per-destination stream number is assigned once and reused on every retry, which keeps the receiver's gap tracking correct. A test derives the worst case as `4 x 300 + 175 = 1375` ms.
6. **Repair ledger.** If delivery fails after retries, if the queue is full, if the task panics, or if the broadcaster is stopped, the debt is recorded in `RepairLedger`, with the same stream number. The ledger is bounded per peer (1,024 entries and 256 KiB of key bytes). Duplicate (scope, key) pairs merge. When over the limit, the tenant with most entries collapses into a single `Tenant` invalidation (at most 16 per peer); when that cannot make progress, the peer is owed one `FullCache` flush. So memory is bounded and correctness is kept, at the price of coarser invalidation (more cache misses) under overload.
7. **Repair loop.** Every 250 ms the push-repair loop sends up to 64 owed entries per peer, using the original stream number. An ack removes the entry. A hard rejection drops the entry and escalates it to a tenant flush, so the receiver cannot be left stale. When a Suspect peer's ledger drains it becomes `Recovering`.
8. **Pull reconciliation.** A second loop, independent of the first so that a slow peer cannot delay repairs to healthy peers, handles peers that are `Suspect` or `Recovering` or that show sequence gaps. The receiver asks the sender for what it missed, using the digest and replay mechanism below. A peer returns to `Active` only after this succeeds and no debt older than its recovery ticket remains.

### Anti-entropy: ring buffer, gaps and replay

Each gateway keeps a ring of the invalidations it *sent*, tagged by destination (`InvalidationRingBuffer`: at most 4,096 records and 4 MiB). Each receiver keeps, per sender, the next sequence it expects (`AntiEntropyTracker`).

::: example
Gateway G sends invalidations numbered 1, 2, 3 to gateway B. Number 2 is lost. B receives 1 (expects 2 next), then 3. Since 3 is greater than the expected 2, B records a gap (2 to 2), keeps 3 in an out-of-order set, and its *highest contiguous acknowledged* sequence stays at 1. The pull loop sends `AntiEntropySyncRequest{highest_acked_seq: 1, ring_digest}` to G. G replays the records it sent to B with sequence greater than 1: records 2 and 3. B applies them (deleting those keys) and advances to 3. If G had already evicted record 2 from its ring (more than 4,096 newer records), the answer is `HistoryOverwritten` and B flushes its **entire** cache with `try_invalidate_all`. That is coarse but safe: B can never keep an entry whose invalidation it cannot prove it saw.
:::

Properties worth stating: the digest is a SHA-256 over the retained window (`compute_digest`); gap and out-of-order bookkeeping is bounded (256 gap ranges, 1,024 buffered sequences per peer, collapsing to a single "resync needed" flag); replay returns only the stream the sender sent to that requester; inbound records are kept in a separate 256-entry accounting list and are never replayed, so a flood of received invalidations cannot push out replayable history; a sequence-mismatch after a sender *restart* cannot be filled, and a successful pull clears such gaps.

### Status table

| Capability | Status | Evidence and what it does **not** prove |
|---|---|---|
| Local purge before cluster call | IMPLEMENTED, TESTED | `test_m3_04a_local_linearizability_immediate_eviction`, `pc5`/`kg5` in `known_gaps.rs`. Per gateway only. |
| Synchronous primary ack with acting-primary failover | IMPLEMENTED, TESTED | `security_phase4b_router_tests.rs` (`..._failover`, `..._strict_150ms_deadline_enforcement`, `..._fail_closed_no_false_204...`). Loopback, in-process or multi-process; not WAN. |
| Fan-out to all members | IMPLEMENTED, TESTED | `test_s3_five_gateway_fanout_reaches_every_peer` (5 gateways). Small N only. |
| Retry, per-peer queue, bounded concurrency | IMPLEMENTED, TESTED | `test_m4_blackholed_peer_keeps_task_count_bounded_and_owes_ledger`, `test_m6_blackholed_peers_do_not_starve_a_healthy_peer`. |
| Repair ledger with bounds and escalation | IMPLEMENTED, TESTED | `cluster_prewarm_repair_tests.rs` (`test_ledger_overflow_*`), `cluster_followup_tests.rs`. |
| Pull anti-entropy (digest, replay, history-overwritten flush) | IMPLEMENTED, TESTED | `security_phase4c_antientropy_tests.rs`, `cluster_followup_tests.rs` (`test_m2_*`, `test_r4_m4_*`). |
| Receiver crash and restart | PARTIAL | A restarted receiver has an empty cache; the sender repairs the logical debt. Sequence-restart behaviour is tested (`test_h1_restarted_sender_invalidations_are_still_applied_by_receiver`). |
| Issuer crash with owed invalidations | **NOT HANDLED in the main tree** | Section 5.7 and 5.9. No test in the main tree kills an issuer. |
| Durable owed-invalidation log (M3) | PROTOTYPE ONLY | Sandbox patch, not integrated (section 5.9). |

::: judge
**JUDGE MAY ASK**

Question: A DELETE returns 204. What exactly has been invalidated at that point?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two things for sure: the gateway that received the DELETE has dropped its own copy, and the chosen primary node for that key has confirmed it dropped its copy, within a 150 millisecond deadline. Every other node is told afterwards, in the background, with retries and a repair ledger, and that is guaranteed only while the issuing gateway stays alive. Sometimes the primary is the issuing gateway itself, so the 204 then says nothing about any remote node. I would not claim that 204 means the whole cluster is invalidated.

**If the judge asks further:**

"Why not wait for everyone?" It would make DELETE latency and availability depend on the slowest or dead node. The design trades that for bounded work and a repair loop. A stricter mode, a delayed 204 until all members ack, was considered in the M3 durability design and rejected as out of scope there; it is an open product decision, not an oversight.

**Evidence to know:** `src/api/server.rs::delete_cache_handler`, `src/dist/router.rs::invalidate_key_across_cluster` (doc comment states the contract), ADR-003 amendment 2026-10-30, G4B-09. Status: IMPLEMENTED and TESTED as described; cluster-wide acknowledgement DESIGNED BUT NOT IMPLEMENTED.
:::

::: judge
**JUDGE MAY ASK**

Question: What if the invalidation message is lost or arrives late?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There are three layers. First, the sender retries up to four times with short backoff. Second, anything still undelivered is written to a per-peer repair ledger and re-sent every 250 milliseconds with the same sequence number until acknowledged. Third, a receiver that notices a gap in the sender's sequence numbers can pull the missed records from the sender's ring buffer; if the history has been overwritten it flushes its whole cache, which is safe but coarse. All of it is bounded in memory. A delayed message is harmless because a delete is idempotent: applying it twice, or after a newer fill, only causes an extra miss. The one case none of this covers is the sender dying while it still owes messages.

**If the judge asks further:**

"How long can a node stay stale?" Until the message is delivered; in the one test that cut a link for two seconds and healed it, convergence was asserted within 800 ms of healing on loopback in a debug build. With a permanent cut or a crashed sender there is no bound other than the entry's own expiry.

**Evidence to know:** `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`, `RepairLedger`, `repair_peer`, `reconcile_peer_anti_entropy`. Status: IMPLEMENTED and TESTED (loopback, debug); sender crash UNHANDLED.
:::

---

## 5.6 Consistency: what is guaranteed, under which failure model

### Definitions

- **Linearizability:** every operation appears to take effect at one instant between its start and its reply, in an order consistent with real time, as seen by all clients of the system.
- **Read-your-delete (per-node):** after a DELETE returns on gateway X, later reads on X do not return the pre-delete body. This is weaker than linearizability across nodes.
- **Eventual consistency:** if no new writes occur and faults heal, all copies converge. It says nothing about how long.
- **Bounded staleness:** a read never returns data older than `B` (a time or a version count). To claim it you must name `B`, the failure model in which it holds, and a test that could falsify it.

### The failure model, written down

| Fault | Covered by a mechanism? | Evidence |
|---|---|---|
| Message loss or delay between live gateways | Yes: retry, ledger, pull | `test_m3_04a_missed_fanout_repaired...`; ADR-003 section 4.1 states loss up to 20 percent, but no test injects a loss *rate*: the fault proxy offers Normal, Delay, Blackhole, Partition and DropNewConnections only |
| Receiver crash and restart | Yes (cache is empty on restart; debts delivered; pull) | `cluster_followup_tests.rs` |
| Transient partition between live gateways | Yes, convergence after healing | one-link test above |
| **Issuer crash with owed invalidations** | **No** | section 5.7; reproduced in sandbox only |
| **Permanent partition** | No bound | entry expiry only |
| Receiver alive but slow beyond the 150 ms legs | Partly: Suspect, ledger | tests with blackholed peers |
| Malicious or compromised member | No (members are trusted) | DR-027, G4B-05 |
| Clock skew | Not relied on: sequences and counters, not wall time (except age-outs) | ADR-003 |
| Origin changes without a DELETE | No: PhantomCache is never told | by design |
| Several hosts, WAN | **UNVERIFIED**: all cluster evidence is single-host loopback | `claims_ledger.md` C6 |

### What the code and tests support

**G1: per-gateway read-your-delete.** The handler purges the local cache before any network call, so a read arriving after the delete began cannot return the old entry from the cache, and an in-flight fetch is detached so later requests do not join it. IMPLEMENTED; TESTED (`test_m3_04a_local_linearizability_immediate_eviction`, `kg5_get_after_delete_start_must_not_serve_old_entry`, `pc5_...`). Two honest limits. First, a comment in `delete_cache_handler` calls this "linearizability"; it is a single-gateway property, and the word should not be used for it in a defence. Second, the register keeps G4A-01 (a pre-invalidation body can survive an acknowledged invalidation through an epoch re-read race) and G4A-08 (a request after an invalidation can join an older in-flight fetch) as unresolved. A statement "no stale body is ever returned after a 204 on the same gateway" is therefore **not** established for all interleavings.

**G2: cross-node convergence after a 204.** ADR-003 stated "any connected reachable peer serves post-invalidation content after Δ ≤ 250 ms". The 2026-10-30 amendment qualifies it: it holds only while the issuing gateway stays alive and for reachable peers. The earlier test header text "P(stale-after-ack) = 0.0 at Δ ≥ 50 ms" is **not supported**: the claims ledger shows the 50 ms was a fixed sleep and cold first-use took longer. Measured (`tests/integration/cluster_propagation_latency_tests.rs`; recorded in `claims_ledger.md`; debug build, loopback, single host, 3 gateways with mTLS and fault proxies in Normal mode; N = 200 trials by 2 peers per run, 4 runs): warm-path P50 about 0.3 ms, P99 about 10 to 14 ms, maximum 15 to 21 ms; the first invalidation to a destination (cold connection plus handshake) took 35 to 60 ms (up to 117 ms under three concurrent copies). A separate harness run in a test comment (`distributed_consistency_tests.rs`) gives different numbers (P50 1.56 to 1.86 ms, P99 30 to 39 ms) with a different metric (it includes one origin fetch), so the two are not comparable; neither is a release-build, multi-host or WAN figure. Pre-warming now opens connections early to avoid the cold-start cost. Use the numbers only as "observed in these loopback debug runs"; the full measurement inventory is Chapter 6, section 6.13.

**G3: fallback staleness cap.** ADR-003 wrote "Staleness ≤ TTL_eff ≤ PHANTOM_MAX_STALE_MS (24 h)". The amendment says this is not true as written. `PHANTOM_MAX_STALE_MS` caps only the *stale-serving window* after expiry. The freshness lifetime itself comes from the origin's `s-maxage`, `max-age` or `Expires` with no gateway ceiling. The 60-second default applies only when the origin gives no freshness information, and it is the literal `60_000` in `server.rs`: `PHANTOM_DEFAULT_TTL_MS` is validated but not wired (CLAUDE.md section 5; `src/config.rs` marks it `Wiring::NotWired`). So the real bound on an unrepaired stale copy is "the entry's own freshness lifetime", which an origin can set to hours.

### The honest statement

> PhantomCache provides, on one gateway, that a read after a completed DELETE does not return the deleted entry (with the open race findings above). Across gateways it provides best-effort asynchronous invalidation with retry, a bounded repair ledger and a pull-based repair, which converged within milliseconds in loopback tests while the issuing gateway stayed alive and the network healed. It does **not** provide cross-node linearizability. It provides **no** staleness bound if the issuing gateway crashes with owed invalidations or if a partition is permanent, other than the entry's freshness lifetime. The test that would falsify the bound claim (kill the issuer after a 204) was reproduced as a failing scenario in a sandbox and is not in the main tree.

::: judge
**JUDGE MAY ASK**

Question: Can two of your nodes serve different values for the same key at the same time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes. By design each gateway has its own copy. They differ for a few milliseconds after a delete while the invalidation is in flight, for as long as a partition lasts if a gateway is cut off but still serving, and indefinitely until expiry if the gateway that issued the delete crashed before telling everyone. They can also differ legitimately with no delete at all, because two gateways fetched at different times and the origin changed in between; each is within its freshness lifetime. We never claim they are the same, we claim a mechanism that makes them converge after a delete when the system is healthy, and we say plainly what is not covered.

**If the judge asks further:**

"What would make it stronger?" Durable owed-invalidations for the issuer, which exists as a tested sandbox design; a gateway-side ceiling on freshness lifetime; and a heartbeat that marks silent peers Suspect. A truly linearizable read would need coordination or a leader, which is a different system.

**Evidence to know:** ADR-003 section 4 and its 2026-10-30 amendment; `docs/research/claims_ledger.md` C6 amendment and qualifier; `tests/integration/distributed_consistency_tests.rs`. Status: convergence IMPLEMENTED and TESTED on loopback; staleness bound UNVERIFIED and not claimed.
:::

::: judge
**JUDGE MAY ASK**

Question: Is your cache linearizable? Do you guarantee bounded staleness?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No to both, and I want to be precise about why. Linearizability across nodes would require every read to be ordered against every delete, and our reads are local, so we do not have it. We have a per-gateway guarantee that a read after a completed delete misses, with two open findings about rare races. Bounded staleness needs a number, a failure model and a test. The number we can give in the healthy, issuer-alive case is milliseconds on loopback in a debug build, which is an observation not a guarantee. In the crash or permanent-partition case the bound is the entry's freshness lifetime, which comes from the origin. I would rather say that than quote a bound I cannot defend.

**If the judge asks further:**

"Your own ADR says 24 hours." The ADR was amended on 2026-10-30 because that statement was wrong: the 24 hour cap bounds the stale-serving window, not the freshness lifetime.

**Evidence to know:** ADR-003 amendment; `claims_ledger.md` C6 qualifier; `src/core/freshness.rs` (no gateway ceiling). Status: neither claim is made.
:::

---

## 5.7 Node failure, recovery, partitions, and concurrency across nodes

![Figure 5.5: One invalidated key in three failures: a receiver that crashes, a link that is cut, and an issuer that crashes.](../assets/ch05_failure_timelines.svg)

Source: `src/dist/router.rs` (broadcaster, ledger, `repair_peer`), `tests/integration/distributed_consistency_tests.rs`, `docs/research/m3_durability/reproduction_report.md`.

### A. A receiver node goes down

Calls to the dead node fail within the 150 ms legs, the node is marked `Suspect`, and its invalidations are recorded in the issuer's ledger. When the node returns, the issuer's repair loop delivers the debts; after a full drain it moves the peer to `Recovering`, and a successful pull reconciliation (or an empty-ledger `InSync` answer) moves it to `Active`. Because the node's cache lived in memory, a restart **empties** it, so no stale entry can survive a receiver crash. The real danger is the opposite case: a receiver that is alive but unreachable. Status: IMPLEMENTED. TESTED for an unreachable peer that heals (`test_m3_04a_suspect_to_active_automatic_recovery`, `test_m10_failed_peer_is_owed_the_invalidation`, `test_h2_failed_delete_still_invalidates_all_peers_after_heal`) and for an in-process peer that restarts on the same address (`test_l2_stale_pooled_connection_is_retried_on_fresh_connection`). The multi-process harness has no kill-and-restart helper in the main tree (it only shuts down), so a real gateway process crash and restart is not exercised there.

### B. A partition (link cut, both sides alive)

There is no leader and no quorum, so there is no split brain in the consensus sense; the two sides simply diverge. The cut-off gateway keeps serving hits from its cache, because **the HTTP read path never consults `NodeState`** (`server.rs` uses node state only for the control endpoint and dashboards). The rule "a Suspect or Recovering node must not serve cache hits" (SEC-POA-04, test `test_p4c_sec09_...`) exists only in the *peer `GetRequest`* handler, which production never sends: real in the protocol code, no effect on what a client sees. After healing, the repair loop converges the cut-off side (one-link test: 2 s fault, convergence asserted within 800 ms of healing, loopback, debug). During a permanent cut nothing bounds the staleness except entry expiry.

### C. The issuer crashes

The invalidations owed to peers live only in the issuer's memory: the broadcaster channel and per-peer queues, `RepairLedger`, the per-destination sequencers and the replay ring. `main.rs` shutdown on a signal prints "cache is in-memory, nothing to flush", drops the server and exits; it calls no router `stop` or drain. A restarted issuer starts with an empty ledger and its streams restart at sequence 1, which receivers ignore as duplicates or obsolete. The receiver cannot even tell that the issuer lost anything.

::: example
**Worked failure example (reproduced in a sandbox, flag off, debug build, loopback, Windows).** Three gateways; the key's primary is gateway 3. All three cache version 1. The origin is changed to version 2. The link from issuer G (1) to B (2) is blackholed. `DELETE` on G returns 204 in 0.59 to 0.95 ms (local purge plus gateway 3's ack). G is killed (`TerminateProcess`) inside B's roughly 1.4 s retry window and restarted. For 10 s B answered the old version on every poll (89 to 91 polls, never converged); the origin advertised `max-age=120`, so B would stay stale for about two minutes. The control run, where G was not killed, converged in 231 to 337 ms. Source: `docs/research/m3_durability/reproduction_report.md`. This is a sandbox reproduction of a defect on the then-current tree; it is **not** a test in the main tree, and it was run with an AI agent's harness, not independently.
:::

### Cross-node concurrency

- *Two DELETEs of the same key from different gateways.* Each sends exact-key invalidations; deletes are idempotent, and each sender has its own sequence stream per destination (the tracker is keyed by originator), so they do not disturb each other. Tested: `test_p4b_duplicate_invalidation_idempotence`, `test_m3_04a_per_destination_sequence_zero_false_gaps`.
- *A GET fill racing an invalidation on the receiver.* An exact-key delete bumps the key's generation and detaches the in-flight fetch, so a fill that started before the delete cannot store its result afterwards. Chapter 4 covers the mechanism and the open G4A-01 and G4A-08 findings.
- *Write ordering is the caller's job.* PhantomCache is never told that the origin changed. The contract is "commit the write at the origin, then DELETE". If the DELETE is sent first, any gateway may refetch the old value and keep it.

::: judge
**JUDGE MAY ASK**

Question: What happens if a node fails? What if the network partitions?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

If a node dies, calls to it fail fast, it is marked suspect and skipped, and the invalidations it missed are queued in a bounded ledger on the sender. When it comes back, its cache is already empty because it is in memory, the ledger is delivered, and it rejoins after a pull reconciliation. If the network splits but both sides are alive, there is no leader, so nothing stops the cut-off side from serving its cached copies; they can be stale until the link heals and the repair loop catches up, or until the entries expire. I would not claim availability and consistency both; we chose availability of reads, and the staleness during a partition is bounded only by the entry's freshness lifetime.

**If the judge asks further:**

"Does a suspect node stop serving?" Not on the HTTP path. The rule exists in the peer protocol handler only. That is a gap I would name before they find it.

**Evidence to know:** `test_m3_04a_suspect_to_active_automatic_recovery`, `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`, `server.rs` (no `NodeState` in the GET path), `test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits` (peer path only). Status: receiver recovery IMPLEMENTED and TESTED (loopback); partition serving is stale by design; no staleness bound.
:::

::: judge
**JUDGE MAY ASK**

Question: What if the gateway that issued the delete crashes before the others got the message?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Then those nodes keep serving the old entry until it expires. The owed messages are only in the issuer's memory, and a restarted issuer starts empty. I reproduced that in a sandbox: three gateways, issuer killed right after answering 204, the third gateway never converged in ten seconds. That defect is open in the main tree. A fix exists as a sandbox design: an opt-in durable log of owed invalidations replayed after restart, with 15 process-level crash tests. It is not integrated, because it touches the invalidation path and needs an independent review and the owner's approval. Until then, the right claim is "guaranteed while the issuer stays alive".

**If the judge asks further:**

"How bad is it?" Bounded by the entry's freshness lifetime, which is the origin's choice, 60 seconds only when the origin gives none. It affects only the entries whose invalidation was still in flight at the crash.

**Evidence to know:** `docs/research/m3_durability/` (`reproduction_report.md`, `design.md`, `remediation/00_status_and_plan.md`), `claims_ledger.md` C6 qualifier, G4B-09. Status: defect REPRODUCED in a sandbox; fix PROTOTYPE ONLY, not integrated.
:::

---

## 5.8 The peer GetRequest stub, DR-029 and DR-027

Two security-flavoured items belong here because they shape what a "cluster member" can do. Chapter 6 owns the full security analysis.

### The `GetRequest` handler fabricates data (DR-029, G4B-30)

In `process_binary_cache_stream` (`src/dist/node.rs`), a peer `GetRequest` is answered like this: if the key is cached, return `HIT <body>`; if not, **create a resource whose body is the string `val_node_<id>`, store it in the node's cache with a 60,000 ms lifetime, and return `MISS val_node_<id>`**. The origin is never contacted. A test pins this behaviour: `pin_kg6_node_get_request_fabricates_missing_key_body` in `known_gaps.rs`. In the mTLS cluster path the node's cache *is* the HTTP serving cache, so a fabricated entry lands in the real cache.

The investigation (`docs/security/investigations/DR-029_cluster_get_cache_poisoning_investigation.md`, an AI agent's report; its reproduction is MEASURED, not independent verification) concluded, and I re-checked in the code:

- The planted key is always tenant `__default__` with a path that does **not** begin with `/`. The handler rejects keys that begin with `/` (round-3 fix M-2) while the HTTP path always starts with `/`. The key spaces are disjoint, so no HTTP request can address a planted entry. The register's earlier claim that a peer can plant a bogus 200 body for any key in the real cache is therefore **not supported**: HTTP-visible poisoning was not demonstrated and appears structurally prevented for the code as it stands.
- What remains: a member can fill entries and bytes with garbage and evict real entries (measured in a small test cache), pollute hit/miss counters, and the stub is a hazard if peer fill is ever routed into the same namespace. Suggested severity: Low as exploited, Medium latent; an owner decision (DR-029).
- Nothing in production sends `GetRequest`; `forward_key_over_tcp` is called only from tests, so the stub is reachable only by a pinned member that chooses to send it.

For the defence, the right wording is: "peer reads are not implemented; the placeholder handler fabricates a value and is a documented design defect with bounded impact, pinned by a test and tracked as DR-029." Status of peer reads: PROTOTYPE ONLY (placeholder), DESIGNED BUT NOT IMPLEMENTED (real peer fill).

### Any member can flush any cache (DR-027, G4B-05)

An `InvalidateRequest` with `Tenant` or `FullCache` scope from any pinned member is applied without authorisation. Tenant strings from peers are not validated against the HTTP tenant alphabet, and each tenant invalidation clones the whole epoch map (the cost the section 5.4 fallback bounds at 10,000 tenants). One `FullCache` frame empties a peer's cache, causing an origin stampede. Inside the stated trust model (members are trusted) this is a blast-radius issue rather than a break-in. It is OPEN and owner-named. The repair ledger itself uses these scopes as an escalation, so removing them is not a one-line change; the design question is the member trust model.

::: judge
**JUDGE MAY ASK**

Question: Your node protocol has a GET request. Can a peer poison your cache with it?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The peer GET is a placeholder, and I will not pretend otherwise: on a miss it invents a value called `val_node_` plus the node id and stores it. No production code sends that request, and it requires a valid cluster certificate pinned in the membership list. The planted key always lives in a key space that HTTP requests cannot reach, because peer keys must not start with a slash and HTTP paths always do. So I can show it does not poison what clients read today. It can still fill the cache with junk and evict real entries, and it is a hazard for any future peer-fill feature. We track it as DR-029, with a test that pins the behaviour, and the fix is waiting on an owner decision.

**If the judge asks further:**

"Why not delete it?" The register records the fix as gated on an owner decision and an independent review (G4B-30). Our reasoning: removing the handler and implementing real peer reads are different directions, and a test pins the current behaviour, so deleting it is a decision rather than a cleanup.

**Evidence to know:** `src/dist/node.rs` GetRequest arm, `tests/integration/known_gaps.rs::pin_kg6_node_get_request_fabricates_missing_key_body`, `m7_sec_round3_tests.rs` (leading slash rejection), DR-029 investigation and review notes, G4B-30. Status: PROTOTYPE ONLY; design defect OPEN.
:::

---

## 5.9 The durability gap and the M3 sandbox work

Section 5.7 showed that owed invalidations vanish with an issuer crash. A separate agent investigated and prepared a fix. Its location and status matter, because a judge may hear "durability was built" and assume it ships.

| Item | Fact |
|---|---|
| Location | `docs/research/m3_durability/` (`design.md`, `reproduction_report.md`, `test_plan.md`, `proposed_patch*.diff`, `remediation/`, `evidence/` logs). |
| Status | **Sandbox only. Not integrated into the main tree.** The defect "is NOT fixed in the main tree, and with the log unset (the default) it would remain present even after integration" (`remediation/00_status_and_plan.md`). |
| What it adds | An opt-in durable log of owed invalidations (`PHANTOM_INVALIDATION_LOG_PATH`; unset = today's behaviour byte for byte), a graceful-shutdown drain with a deadline (`PHANTOM_SHUTDOWN_DRAIN_MS`), harness support for kill and restart, and multi-process crash tests. |
| Mechanism | A group-committed append-only file of CRC-protected records (intent, ack with a stamp watermark, full-flush marker, checkpoint). An intent is durable after the primary acks and before fan-out starts, so a 204 never outruns the record; after a restart the log is replayed into the ledger; an ack removes only intents with a stamp not greater than the one captured before the send. |
| Failure model it claims | Process crash, SIGTERM, restart, and (with `fsync=always`) host reboot. **Not covered:** loss of the log medium, an operator deleting log and sentinel, a second issuer failure before replayed debts are delivered *and* loss of the log, peers gone forever, clock steps. |
| Evidence | Reproduction and fixes validated in a scratch copy, debug build, loopback, Windows: 6 of 8 new unit tests and 1 of 15 process tests fail on the earlier, unfixed version of the patch and pass on the fixed one (`remediation/crash_point_test_matrix.md`). Run by an AI agent; no independent review yet. |
| Gate | Owner approval (application-code change) plus an independent review pass, because it touches the invalidation path (`CLAUDE.md` section 5). |

The claim you may make: "the loss is reproduced, a durable-log design exists and passes its sandbox tests, and it is deliberately not integrated until it is reviewed." The claim you may not make: "invalidations are durable."

::: caution
The main-tree artefacts that mention this problem are documentation, not tests: `docs/research/claims_ledger.md` (C6 qualifier), the ADR-003 amendment, and G4B-09. No test in `tests/` kills an issuer. The risk register entry stays open until integration.
:::

---

## 5.10 Workload intelligence: what exists and where it runs

### Simple explanation

"Workload intelligence" means the gateway watches what traffic it gets (which keys, how fast, how often a request hits), notices patterns (a burst, a hot key, a changing mix), and may adjust a setting in response. Doing that safely needs four pieces: **observe** (measure), **detect** (spot a pattern), **evaluate** (check that a change would help, before applying it), and **adapt** (apply it with a guaranteed way back). The design document `docs/research/phantom_intelligence_phase.md` writes this as *observe, detect, predict, evaluate, adapt, explain*.

### The honest summary

**It is not AI.** There is no model, no training and no learning from data. The mechanisms are classical control and statistics: an additive-increase multiplicative-decrease rate controller (the rule TCP uses for congestion), counting sketches, an exponential moving average, a cumulative-sum change detector, and a normal-approximation confidence interval. One class, `DecisionStumpSelector`, is a single hand-set threshold (`candidates.rs`); it is library-only and learns nothing. Calling this "AI" would be wrong and a judge will test it. The design documents themselves say machine learning is admitted only through a gate (`phantom_intelligence_phase.md` section 6) and nothing has passed it.

**Almost none of it runs in the shipped gateway.** The table separates what is on the live request path from what is a library.

![Figure 5.6: The observe, detect, evaluate, adapt loop. The left column is live code on the serving path; the right column is a library that no production code calls.](../assets/ch05_intelligence_loop.svg)

Source: `src/main.rs` (`intel_executor`, `pie: None`), `src/api/server.rs` (`acquire_origin_permit`, intelligence handlers), `src/intelligence/*.rs`, `src/intel/*.rs`; "no production caller" established by grep over `src/`.

| Component | File | What it is | On the serving path? | Status |
|---|---|---|---|---|
| Latency histogram `AtomicLogLinearHistogram` | `intel/telemetry.rs` | 2,688-bucket histogram | **Written** on every GET; **not read** by any production endpoint (section 5.11) | IMPLEMENTED, TESTED (bucket error) |
| `TelemetryCollector` raw window | `intel/telemetry.rs` | 10,000-sample ring, sorted on demand | Yes: this is what `/api/v1/metrics` percentiles use | IMPLEMENTED, TESTED |
| `OriginMetrics` | `intel/origin_metrics.rs` | 16-bucket origin latency histogram, in-flight and error counters | Yes | IMPLEMENTED |
| AIMD origin gate `OriginGate` + `OriginConcurrencyCapController` | `intelligence/executor.rs`, `candidates.rs` | Adaptive cap on concurrent origin fetches | **Yes, only if `PHANTOM_ORIGIN_CAP_MAX` is set and mode is `canary` or `active`** | IMPLEMENTED, TESTED; benefit NOT established |
| `PHANTOM_INTELLIGENCE` mode switch, `/api/v1/intelligence/status` | `main.rs`, `server.rs` | Reads mode, active candidate id, revert count | Yes (read-only) | IMPLEMENTED |
| `/api/v1/intelligence/audit` and `AuditStore` | `intelligence/audit.rs` | 100-record ring of decisions | Endpoint served; **nothing in `src/` appends to the store** | PARTIAL (empty in production) |
| `WorkloadPipeline`: rate EWMA, Count-Min + Space-Saving heavy hitters, Zipf estimate, working set, CUSUM burst, hot-key emergence | `intelligence/signals.rs` | Detectors | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_signals_tests.rs`) |
| `ShadowEvaluator` and shadow LRU/SIEVE/S3-FIFO/TinyLFU | `intelligence/shadow.rs` | Counterfactual replay on a key sample | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_shadow_tests.rs`) |
| `PieDecisionEngine` | `intelligence/engine.rs` | Guardrails + confidence interval verdict | **No** (tests only) | IMPLEMENTED as library; TESTED (`intel_engine_tests.rs`); statistics unsound (F12) |
| Canary routing `route_bucket`, `check_watchdog`, `safe_apply_policy`, `apply_policy` | `intelligence/executor.rs` | Stratified arms, stall revert, panic revert | **No** (no handler calls them) | IMPLEMENTED as library; TESTED (`intel_executor_tests.rs`, `intel_safety_and_replay_tests.rs`) |
| Other candidates: stale-window controller, admission filter, rate forecaster, hot-key predictor, decision stump | `intelligence/candidates.rs` | Library | No; the lab simulator uses the stale controller and admission filter | IMPLEMENTED as library |
| `CountMinSketch`, `WorkloadProfile` | `intel/sketch.rs`, `intel/workload.rs` | Early sketch and workload presets | **No**; only `tests/unit/wp2_tests.rs` uses them | PROTOTYPE ONLY |
| Legacy `PieController` | `control/pie.rs` | Threshold stub | **No**: `pie: None`; never constructed | Legacy, quarantined |

Everything marked "No" is exercised only by unit and integration tests, and (for the AIMD gate, the stale controller and the admission filter) by the Tier-1 lab simulator. The simulator is a deterministic virtual-time model; a result from it is SIMULATED, not measured on a real gateway.

---

## 5.11 Telemetry: the histogram, with the maths

### Simple explanation

To report "99 percent of requests were faster than X" you either remember every latency (expensive) or you count them in buckets. A **log-linear histogram** uses fine buckets for small values and coarser ones for large values, so the *relative* error stays small everywhere.

### How it works

`AtomicLogLinearHistogram` (`src/intel/telemetry.rs`) takes a latency in microseconds:

- Values below 128 µs get one bucket each (exact, 128 buckets).
- Above that, each power-of-two range (an *octave*, from `2^k` to `2^(k+1)`) is split into 128 equal sub-buckets. There are 20 octaves, so `128 + 20 x 128 = 2,688` buckets.
- The bucket index comes from the position of the highest set bit (a hardware count-leading-zeros instruction) and a shift: constant time, no loop, no lock. Each bucket is an `AtomicU64` incremented with relaxed ordering.

**Error bound.** In octave `k` the bucket width is `2^k / 128`. A bucket is reported by its midpoint, so the worst error is half a width, `2^k / 256`, and since the value is at least `2^k` the relative error is at most `1/256 = 0.39%`. Below 128 µs the error is zero.

::: example
Latency 1,000 µs. The highest set bit is bit 9 (512 to 1,023), so octave 2 (`k = 9`); width is `512/128 = 4`. Offset from the octave base is `1000 - 512 = 488`; sub-bucket `488 / 4 = 122`; bucket index `128 + 2 x 128 + 122 = 506`. The bucket covers 1,000 to 1,003 µs and is reported as its midpoint, 1,002 µs: error 0.2 percent, inside the 0.39 percent bound.
:::

Percentiles are read by walking the buckets in order until the cumulative count reaches `ceil(p x total)`.

Limits: values of 2^27 µs (about 134 s) or more fall into the last sub-bucket and lose the bound. The bound is a bound on *bucketing*, not on sampling error. The bucket mapping and its reconstruction error are TESTED (`telemetry_redesign_tests.rs`).

### A discrepancy you must know

`CLAUDE.md` describes "Telemetry Hot-Path: AtomicLogLinearHistogram ... bounded error <= 0.39%". That is true of the *recording* code. In the current tree no production reader uses the histogram's percentiles (a search for `histogram()` and `calculate_percentiles_histogram_*` finds callers only inside `telemetry.rs` and tests). The percentiles on `/api/v1/metrics` and in the UI come from `TelemetryCollector::calculate_percentiles_us`, which locks the 10,000-sample raw window, filters GET samples, **sorts them** and indexes with `floor(p x n)`; `record_us` also takes two mutexes (`raw_buffer`, `raw_buffer_us`) per observation. So the lock-free histogram is built and tested but the displayed percentiles do not use it. Say: "the histogram is implemented and recorded; the dashboard percentiles are computed from the raw window." Percentile-definition inconsistency (G4A-24 / F24) and the comparison of the two sources are Chapter 6, section 6.12.2. Found by reading and grep, not by running the gateway: UNVERIFIED at runtime.

::: judge
**JUDGE MAY ASK**

Question: How accurate are your latency percentiles, and how do you compute them without slowing the request path?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We built a log-linear histogram: 2,688 counters, constant-time recording, relative error at most 0.39 percent by construction and checked by tests. That is the recording design. In the current gateway the percentiles on the metrics endpoint are actually computed from a 10,000-sample window that is sorted on demand, so they are exact for that window but not the histogram, and the sort and two locks are on the observation path. Switching the endpoint to read the histogram is a small change that has not been made. I would not quote the 0.39 percent as the accuracy of the displayed numbers.

**If the judge asks further:**

"Which is right?" The sorted window is exact for the last 10,000 GETs; the histogram would cover all history at 0.39 percent. They answer different questions, and the repository's percentile conventions are also inconsistent (floor versus nearest-rank) which Chapter 6 discusses.

**Evidence to know:** `src/intel/telemetry.rs` (`value_to_bucket`, `calculate_percentiles_us`, `record_us`), `tests/integration/telemetry_redesign_tests.rs`, `src/api/server.rs` (metrics uses `calculate_percentiles_us`). Status: histogram IMPLEMENTED and TESTED; histogram as the live percentile source NOT the case.
:::

---

## 5.12 Probabilistic structures, hot-key detection and workload patterns

### Count-Min Sketch

A **Count-Min Sketch** estimates how many times each item was seen using a small fixed table. It has `d` rows of `w` counters and `d` hash functions. To count an item, add 1 in each row at the position its hash gives. To estimate, take the **minimum** of its `d` counters. Collisions only add, so the estimate is never too low; it can be too high. With width `w` and depth `d` the overestimate is at most `(e/w) x N` with probability at least `1 - e^-d`, where `N` is the number of counted events (Cormode and Muthukrishnan, 2005; stated from memory and not re-verified against the paper in this session).

In `HeavyHitterDetector` (`src/intelligence/signals.rs`) `d = 4` and `w = 1,024`, so `e/w` is about 0.27 percent of `N` with confidence about 98 percent (`1 - e^-4`). Counters are halved every 10,000 samples so old traffic fades; hence `N <= 10,000` and the error is at most about 27 counts at that confidence. Memory is `4 x 1,024 x 4` bytes = 16 KiB.

The older `src/intel/sketch.rs::CountMinSketch` is a separate, simpler version using `DefaultHasher`. **It is not used by any production or library path** (only `tests/unit/wp2_tests.rs`). It is a prototype; do not describe it as the hot-key detector.

### Space-Saving (top-k)

A sketch estimates any item but does not say *which* items are heavy. **Space-Saving** (Metwally, Agrawal and El Abbadi, 2005; from memory) keeps `k` counters. A seen item increments its counter. A new item, when the table is full, takes over the entry with the smallest count and starts at that count plus one, remembering the inherited count as its possible error.

::: example
Capacity `k = 3`, stream `a a b c d`. After `a a b c` the table is `a:2, b:1, c:1`. Item `d` is new and the table is full, so `d` replaces the minimum (say `b:1`) and becomes `d:2` with error 1: its true count is between 1 and 2. In `HeavyHitterDetector` `k = 64`, so it tracks at most 64 candidate hot keys and finds a linear minimum in `O(k)` per miss.
:::

### The rest of the pipeline

| Detector | Method | Note |
|---|---|---|
| Request rate | EWMA over windows | smoothing factor `alpha` |
| Zipf skew | least-squares slope of `ln(frequency)` against `ln(rank)` over the top 50 | the doc comment says "MLE"; it is a regression, and it is fitted on the Space-Saving counts, which are biased upward |
| Working set | spatially sampled keys (1 in `R`) times `R`, assuming **1 KiB per object** | the 1 KiB is hard-coded, so the byte figure is a guess |
| Burst | one-sided CUSUM with slack and threshold | detects a sustained rate increase |
| Hot-key emergence | a top-10 key whose count is at least 3 times its previous window's count and at least 30 | a fixed rule |

### Where hot keys are actually handled

The shipped gateway does not detect hot keys. `/api/v1/metrics` returns `"hot_keys": {"src": "unavailable", "reason": "not_implemented"}`. What protects an origin from a hot key is **single-flight coalescing** (Chapter 4): concurrent misses on one key share one origin fetch. That needs no detection. A hot key that is already cached is simply served locally by every gateway, which is a benefit of the local-copy design. The detectors above could drive prefetch, admission or per-key caps; none does.

Workload "patterns" in the code are presets (`WorkloadProfile::travel_hotel()`, `ecommerce()`, `ticketing()`, `general_api()`: rate, Zipf skew, catalogue size, read ratio) not used by any gateway code (only an early unit test); the lab (`src/lab/workloads.rs`, Chapter 7) is the maintained workload generator.

::: judge
**JUDGE MAY ASK**

Question: How do you detect and handle hot keys?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two separate things. Handling: concurrent misses for one key are coalesced into a single origin request, so a hot key cannot multiply origin load, and once cached it is served locally by every gateway. That is live and tested. Detection: we have a library detector that combines a Count-Min Sketch with a Space-Saving table of 64 keys and a rule for sudden emergence, tested in isolation, but it is not wired into the gateway, and the metrics endpoint reports hot keys as unavailable. So I would claim hot-key protection through coalescing, and hot-key detection as a built-but-unused component.

**If the judge asks further:**

"What error does the sketch have?" Overestimate only, at most about 0.27 percent of the events in a window with about 98 percent probability for our dimensions, from the standard Count-Min bound. We have tests of its behaviour, not a measured error on production traffic, because there is none.

**Evidence to know:** `src/intelligence/signals.rs::HeavyHitterDetector`, `HotKeyEmergenceDetector`; `src/core/coalesce.rs` (Chapter 4); `server.rs` `"hot_keys"` unavailable; `src/intel/sketch.rs` unused. Status: coalescing IMPLEMENTED and TESTED; detection PROTOTYPE ONLY / library, not wired.
:::

---

## 5.13 The adaptive algorithms, one by one

### A. AIMD origin concurrency cap (the only adaptive control on the serving path)

**Idea.** When the origin is slow or failing, sending it more concurrent requests makes things worse. Cap the number of concurrent origin fetches, lower the cap fast when the origin struggles, raise it slowly when it is healthy.

**AIMD** stands for additive increase, multiplicative decrease, the rule behind TCP congestion control. Increase by a small constant; on trouble, multiply by a factor below one.

**What the code does** (`OriginGate`, `OriginConcurrencyCapController::step`):

```
every N real origin observations (config PHANTOM_ORIGIN_CAP_WINDOW, default 100):
    p99 = nearest-rank 99th percentile of the window's latencies
    err = failures / N            (HTTP status >= 500, or a transport error)
    if p99 > p99_ceiling or err > 5%:  cap = max(min_cap, floor(cap * 0.70))
    else:                              cap = min(max_cap, cap + 1)       # max_cap clamped to 256
```

A request that wants to contact the origin takes a **permit** first (an atomic counter below the cap). If none is free, the gateway answers `503 Service Unavailable` with `Retry-After: 1`. The permit is released by `Drop`, so success, error, timeout and cancellation all release it. Coalesced waiters never take a permit; only the single-flight leader, a bypass request and a background revalidation leader do.

::: example
`min = 2`, `max = 32`, ceiling 60 ms. The controller starts at 32. Window 1 shows p99 = 80 ms: cap becomes `floor(32 x 0.70) = 22`. Window 2, p99 = 90 ms: `floor(22 x 0.70) = 15`. Window 3, p99 = 70 ms: `floor(15 x 0.70) = 10`. The origin recovers and p99 = 50 ms for the next windows: 11, 12, 13 and so on, one step per window. Recovery from 10 back to 32 takes 22 healthy windows, 2,200 origin fetches at the default window of 100. The cap falls by 30 percent per slow window but rises by 1 per good window; that asymmetry is the point of AIMD.
:::

**Where the cap is enforced.** Only when `PHANTOM_ORIGIN_CAP_MAX` is set **and** `PHANTOM_INTELLIGENCE` is `canary` or `active` (`origin_cap_enforcing`). In `off`, `observe` and `advisory` the in-flight count is tracked but nothing is refused. When a cap is set but the mode is `off`, `config.rs` emits a warning that the cap is inert.

**Findings that apply.**
- **"Canary" is not a canary (F14, a recorded limitation under ODR-1).** `canary` mode applies the cap to 100 percent of origin fetches. The 5 percent routing exists only as `route_bucket`, which no handler calls. There is no automatic rollback in the serving path. The mode name over-promises; the config layer warns that the mode is experimental.
- **The "p99" is the window maximum for windows under 100 samples (F15 and the code).** Nearest-rank `ceil(0.99 x n)` equals `n` for `n < 100`. At the default window of 100 it is the second-largest sample. The controller also sees only *completed* fetches; cancelled or timed-out fetches contribute no sample (F15, a plausible survivorship bias, untested).
- **It can behave as a static cap.** In the Scenario B live artefact the cap fell from 32 to about 12 within seconds and stayed there (`AIMD_EFFECTIVE_CAP_NOTE` in `src/lab_live/scenario_b.rs`); the simulator's static cap is this same controller with `min = max` (`t1_sim.rs`). Details and caveats: Chapter 7, section 7.5.2.

Status: IMPLEMENTED; TESTED (`origin_cap_tests.rs`, `intel_executor_tests.rs`, `intel_candidates_tests.rs`: step arithmetic, permit accounting, saturation). What the tests do not prove: that adaptation helps. It is labelled experimental in `docs/user/limitations.md` and in config warnings.

### B. Shadow evaluation (library)

**Idea.** Before changing the eviction policy, test the new policy on *a copy* of the traffic, so a bad idea costs nothing. `ShadowEvaluator` feeds the key hashes of a spatial sample (`key_hash % R == 0`, design value `R = 16`) to two small simulated caches, each a policy implementation (`LruShadow`, `SieveShadow`, `S3FifoShadow`, `TinyLfuShadow`) of the sampled capacity (`C / R`), and compares their hit ratios.

The report computes `delta = hit_ratio_candidate - hit_ratio_baseline` and a 95 percent interval `delta +/- 1.96 x se`, with `se` the standard error of a difference of two **independent** proportions, then marks the result significant if the lower bound exceeds `delta_min` and at least 100 requests were sampled.

**Honest limitations (F13).** The two shadows replay the *same* sampled stream, so their outcomes are correlated (the interval is too wide in that respect), and cache hit sequences are serially dependent and skewed (the interval is too narrow in that respect); the 95 percent label has no stated coverage. Cold-start misses are included and there is no multiple-comparison control. It works on key hashes only, not object sizes. The mechanism (spatially sampled mini-simulation) is the idea behind published designs such as SHARDS and mini-simulation evaluators; the literature status is *not verified from a primary source here*.

Status: IMPLEMENTED as a library; TESTED; not on the serving path.

### C. Decision engine and canary routing (library)

`PieDecisionEngine::evaluate` applies a fixed sequence:

1. Mode `off` or `observe` returns `NoAction`.
2. Inside a 60 s cooldown: `NoAction`.
3. Fewer than 500 requests per arm: `NoAction`.
4. **Hard guardrails** (immediate `Rollback`): any stale-cap violation in the candidate arm; candidate origin errors more than the control's plus 5; candidate P99 more than 10 percent above control.
5. Otherwise the hit-ratio difference with a z-interval; `Commit` if the **lower bound exceeds 0.05** and fewer than 4 switches happened in the hour.

`evaluate_with_provenance` refuses `Canary` or `Active` decisions when the input data is `Simulated` or `Unavailable` (a provenance gate that matches the project rule never to adapt on synthetic metrics).

The three **arms** come from `route_bucket`: the key hash is mixed and reduced modulo 1,024 buckets; 10 buckets (about 1 percent) are the permanent *holdback*, 50 (about 4.9 percent) are the *candidate*, the rest are *control*. Because the split is by key, not by request, the arms see different keys.

**Honest limitations (F12).** The "p99 tracker" updates only when a sample exceeds its current value and moves 5 percent toward it, so it is not a quantile and depends on arrival order. The error guardrail compares raw counts of a 5 percent arm with a 95 percent arm, so it essentially cannot fire unless the candidate's error rate is about 19 times higher. The holdback arm is recorded and never used in the verdict. The comment says "5% relative improvement" while the code compares an absolute difference of 5 percentage points. The interval treats key-partitioned arms as random samples of one population. Tests feed constant latencies, which hide the tracker flaw. G4C says: do not activate on live traffic.

Status: IMPLEMENTED as a library; TESTED; not on the serving path.

### D. Executor, safe fallback and watchdog (library)

`DefaultPolicyExecutor` is the one object that *would* apply a decision. `apply_policy` sets an eviction policy, a stale window or a background-refresh cap on the cache, **clamping** to hard limits (24 h stale cap, 256 refreshes). `revert_to_safe` restores a baseline. `safe_apply_policy` catches a panic, reverts and forces mode `off`. `check_watchdog(now, timeout)` reverts and forces `off` if the intelligence loop's last heartbeat is older than the timeout. This is the "hard caps are enforced by the execution layer regardless of any decision" principle in code.

Limits: none of these methods is called by a request handler or a background task in `src/` (only `acquire_origin_permit`, the snapshot, mode, active candidate and revert count are used). The baseline restores `Sieve`, 24 h and 128 refreshes regardless of the operator's configuration (F09, latent); the watchdog never trips before the first heartbeat and uses a non-monotonic clock (F26). Status: IMPLEMENTED as a library; TESTED; DESIGNED BUT NOT WIRED.

### E. The legacy PIE stub, and why it is not the intelligence

`src/control/pie.rs` defines `PieController`. Its header says "LEGACY, not the PIE (Quarantined)". Its `evaluate_candidate` computes `utility = hit_rate x 100 - p99 x 0.1`, returns `NoAction` below 1,000 samples, and commits if the utility is above 80 or rolls back an "S3-FIFO" candidate below 50. It writes a trace with `expected_utility: 85.0`, a constant, not a prediction. It has no candidate evaluation, no canary, no confidence interval and no counterfactual: the "evaluation" is a threshold on the live aggregate. `main.rs` passes `pie: None`, so it is never constructed in the shipped binary; the only caller is the simulation loop in `server.rs`, guarded by `if let Some(pie)`. It remains so that legacy tests (`p4_pie_tests`, `wp2_tests`) compile. Earlier handoff documents and the README called this "PIE" or "adaptive intelligence". That is wrong; `docs/research/pie_audit.md` records the gap. The real engine is the library in `src/intelligence/`, which is itself not wired.

---

## 5.14 The M6 result: no benefit from adaptivity (sealed)

M6 was a pre-registered, sealed evaluation (Tier 1, simulated; protocol v2.2; 30 sealed seeds; 8 scenarios). The full account is Chapter 7, section 7.6; the mechanical verdict from `docs/research/m6_sealed_results.md` is:

- **H1 (the origin-aware adaptive controller beats the best static configuration found on the tuning data): NOT_SUPPORTED.**
- Peak origin concurrency: supported in 0 of 8 scenarios; the adaptive arm was credibly *worse* in 7 (it adapted up to 8 concurrent fetches where the static baseline held 2).
- Client P99: supported in 2 of 8 (O-HET +29.2 percent, O-GRD +5.6 percent); in five of the other six scenarios P99 was identical in both arms (a saturated, weak discriminator) and O-RT was +4.2 percent, below the 5 percent threshold.
- Constraint failures: fresh-hit loss over 1 point and error-rate increase over 0.5 points, in the same 3 scenarios. Hard-cap violations: 0 in every arm.
- The unconstrained tuning optimum for the adaptive controller was itself a **static cap** (min = max = 2). H2 and H4 (stale-window mechanisms): NOT TESTABLE at the registered horizon. H3, H5, H6 (overhead, oscillation): UNVERIFIED in a simulator.

It shows that, in this simulation, a forced-adaptive AIMD controller did not beat a tuned static cap. It does **not** show that adaptive caching is useless, and it is not a measurement of any real gateway. The research lead's product decision: the static origin-concurrency cap is the defensible feature; the adaptive controller is NOT promoted to the serving path as a claimed benefit, though the code stays behind configuration. A static cap versus no cap is a large effect in the simulator (development data, exploratory, outside H1): SIMULATED, not a production figure.

::: judge
**JUDGE MAY ASK**

Question: What is the actual intelligence in your system? Is it AI?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is not AI and I would not call it that. There is no learned model. On the live path there are two things: latency and hit telemetry, and one small adaptive controller, an additive-increase multiplicative-decrease cap on concurrent origin fetches, the same rule TCP uses. Beside that there is a library for workload detection, shadow evaluation of eviction policies, a statistical decision engine, a canary-routing function and a watchdog. That library is tested but not wired into the gateway. A legacy stub called PIE is a threshold rule and is not constructed in the shipped binary. We also ran a sealed, pre-registered simulation and the adaptive controller did not beat a good static cap, so the defensible feature is the static cap.

**If the judge asks further:**

"Then why build it?" As a research platform: the lab, the sealed protocol and the negative result are real engineering outputs. A negative result reported honestly is a finding. "Could it be wired in?" Yes, behind the provenance gate and the hard caps, but the findings on the decision engine's statistics would need fixing first, and there is no evidence yet that it helps.

**Evidence to know:** `src/intelligence/executor.rs`, `candidates.rs`, `engine.rs`; `src/control/pie.rs`; `docs/research/m6_sealed_results.md`, `pie_audit.md`; G4C F12, F13, F14. Status: AIMD cap IMPLEMENTED and TESTED, benefit NOT established; the rest PROTOTYPE ONLY or library; PIE stub legacy.
:::

::: judge
**JUDGE MAY ASK**

Question: Your own experiment says the adaptive controller does not beat a static cap. Why should anyone use the adaptive mode?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

They should not, on current evidence, and our documentation says so. The sealed result is simulated, so it is not a proof that adaptation never helps, but it gives no evidence that it does. The feature that did show a large effect in the simulator is the static cap itself: bounding concurrent origin fetches protects a slow origin. So the product position is a configurable static cap, with the adaptive mode available as an experimental option. We kept the negative result because hiding it would be worse than not having a positive one.

**If the judge asks further:**

"Is the simulator trustworthy?" It is deterministic and uses the real executor and cache code with a virtual clock, but it is a model of the origin; a loopback experiment with a real gateway (Scenario B) found the controller acted as a static cap near 12, which fits the simulation's lesson. Neither experiment is a multi-host measurement.

**Evidence to know:** `docs/research/m6_sealed_results.md`, `claims_ledger.md` section 6 (C1 NOT SUPPORTED, narrowed to Tier-1), `src/lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`, `docs/user/limitations.md`. Status: adaptive benefit NOT established; static cap IMPLEMENTED.
:::

---

## 5.15 Chapter status summary

| Capability | Status | Key limit |
|---|---|---|
| Static membership, pinned-certificate mTLS cluster | IMPLEMENTED, TESTED | No runtime change or revocation trigger (G4B-07); no certificate-lifetime enforcement on live links (G4B-08) |
| PhantomMesh frame protocol | IMPLEMENTED, TESTED | Codec and loopback behaviour only |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`); no code path lets a heartbeat change node state (G4B-12) |
| Rendezvous hashing, equal weights | IMPLEMENTED | No uniformity test in repo; unspecified `DefaultHasher` |
| Weighted rendezvous hashing | DESIGNED BUT NOT IMPLEMENTED | Weight is a constant 1.0 |
| Routed reads / peer fill | DESIGNED BUT NOT IMPLEMENTED | GET is local only; peer `GetRequest` is a placeholder (DR-029) |
| Exact-key DELETE: local purge, primary ack, fan-out to all members | IMPLEMENTED, TESTED | 204 is not a cluster-wide ack (G4B-09) |
| Retry, per-peer queues, repair ledger, pull anti-entropy | IMPLEMENTED, TESTED | Memory only; single host loopback evidence |
| Durable owed-invalidation log | PROTOTYPE ONLY (sandbox) | Not integrated; needs review and owner approval |
| Cross-node linearizability, bounded staleness | Not claimed | See section 5.6 |
| Membership-epoch enforcement on invalidations | Not implemented | G4B-06 |
| Tenant/global epochs (local) | IMPLEMENTED, TESTED | G4A-01, G4A-17 open (Chapter 4) |
| Telemetry log-linear histogram | IMPLEMENTED, TESTED | Recording and bucket arithmetic only; it is fed but has no production percentile reader, and it is not the source of displayed percentiles |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY (library) | Unit-tested; not wired into the serving path; metrics report hot keys unavailable |
| AIMD origin cap controller | IMPLEMENTED, TESTED (arithmetic and cap unit tests) | On the serving path only when a cap is configured in canary or active mode (default off); benefit not established; "canary" is 100 percent |
| Shadow evaluation, decision engine, canary routing, watchdog | IMPLEMENTED as library | Not wired; statistical flaws F12, F13 |
| Legacy `PieController` | Legacy stub | Not constructed |
| Machine learning | Not present | A hand-set stump in a library |

---

## Key ideas to remember

::: concept
1. **The cluster is local copies plus invalidation fan-out.** Reads are always local. Rendezvous hashing chooses only the node that must acknowledge a DELETE.
2. **A 204 means: this gateway purged, and the primary (possibly itself) acknowledged.** Everyone else is told asynchronously, with retry and a bounded, in-memory repair ledger.
3. **Three different "epochs":** local tenant/global epochs (cache entries), membership epoch (static, not enforced on invalidations), and per-destination stream sequences (loss detection and replay).
4. **Honest consistency:** per-gateway read-your-delete (with open race findings); cross-node convergence in milliseconds on loopback while the issuer lives; no bound if the issuer crashes or a partition is permanent, except the entry's freshness lifetime.
5. **The intelligence is mostly a library.** On the live path: telemetry and one AIMD cap. Not AI. The sealed experiment found no benefit over a static cap.
:::

## What you should now understand

- Why a membership change does not move data in this system, and what it does change.
- How highest-random-weight hashing is derived, why it is minimally disruptive, and why multiplying by a weight is not the weighted form.
- The ordered steps of a DELETE and which of them is synchronous.
- Why a restarted receiver is safe but a crashed issuer is not.
- What the node states mean in practice and which ones are only ever set by a demo control.
- The difference between a count-min sketch, a Space-Saving table and a histogram, and which of them the gateway actually uses on live requests.
- Why `canary` mode is not a canary, and why the legacy PIE is not the intelligence.

---

## Knowledge check

1. Which operations in the shipped gateway call `route_key`, and what does its result decide?
2. State the minimal-disruption property of rendezvous hashing and prove it in two sentences. How many keys moved in the replica experiment when node 3 was removed, and why that number?
3. What are the three things called "epoch", and which of them changes at runtime in a deployed cluster?
4. List the steps from receiving `DELETE` to returning 204. Which steps are asynchronous?
5. A peer is unreachable for 10 seconds. Describe what happens to the invalidation for a key deleted during that time, and which component bounds memory.
6. What does a receiver do when it sees stream numbers 4, 5, 8 from one sender? What if the sender's ring no longer holds record 6?
7. Why does a crashed receiver not serve stale data after restart, while a crashed issuer can leave a peer stale?
8. Give the Count-Min error bound for width 1,024 and depth 4. Is the estimator biased up or down?
9. In `canary` mode, what fraction of origin fetches is subject to the cap?
10. Which of these run on the live request path: Space-Saving, the AIMD gate, the shadow evaluator, `PieController`, the log-linear histogram (recording)?

## Codebase exploration tasks

1. **Trace a DELETE.** Start at `src/api/server.rs::delete_cache_handler`. Follow the call into `RendezvousRouter::invalidate_key_across_cluster` in `src/dist/router.rs`, then `fanout_targets`, `dispatch_fanout` and the `ReplicaBroadcaster` worker closure. Find: where the stream number is assigned, where a failed job is written to the ledger, and the numeric limits (attempts, backoff, timeouts). You should learn how "acknowledged", "retried" and "owed" are different states of one invalidation.
2. **Find what is wired.** Start at `src/main.rs` (the `router` construction and the `intel_executor` block). For each of `forward_key_over_tcp`, `check_watchdog`, `route_bucket`, `CountMinSketch`, `PieController::evaluate_candidate`, use `grep` over `src/` to list callers outside tests and the defining file. You should learn to separate library code from the live path, and to produce evidence for "no production caller".
3. **Replicate the router formula.** In a scratch file outside the repository, copy `calculate_hash` and `route_key`'s selection rule, hash 100,000 keys across 3 and 5 nodes, remove a node, and count the keys that move. Then change the weight to 2.0 for one node using the raw multiplication and measure the share of the heavier node (expect about 75 percent for two nodes with weights 1 and 2, not 67 percent). You should learn why minimal disruption holds and why the weighted form needs a logarithm.

## Challenge questions

1. **Close the issuer-crash gap without a durable log.** Propose a design that needs no disk write. State the new failure model and what remains unprotected. *Model answer:* (a) Delay the 204 until all reachable members ack (synchronous fan-out): removes the loss window for reachable peers at the cost of DELETE latency and availability coupled to the slowest member; a peer that is unreachable still needs a record, so some state must survive. (b) Replicate the owed set to a second gateway (a buddy) before answering 204: survives one crash, adds a message and a new trust and ordering problem, and fails on a double fault. (c) Cap the exposure instead: a gateway-side ceiling on freshness lifetime (for example a configured `s-maxage` clamp) bounds staleness regardless of crashes; it trades hit ratio. Each option moves, rather than removes, the failure model. Only a durable record (as in M3) or a bound on lifetime gives a stated bound after a crash.
2. **Make the weight real.** Suppose nodes have capacities 1, 2 and 4. Write the owner function that gives each node a share proportional to its capacity while keeping minimal disruption, state the share for each node, and say what you would need to change in `route_key` and in what tests. *Model answer:* `score_i = -w_i / ln(u_i)` with `u_i = (h + 0.5) / 2^64` to avoid 0 and 1; shares are 1/7, 2/7, 4/7. Replace `(hash_val as f64) * capacity_weight` with that expression, read weights from the membership file (and add them to the registry and the digest), and add tests for the share within a statistical tolerance over many keys and for minimal disruption when a node is removed. Note that all gateways must use identical weights and the same specified hash, or they will disagree about owners.

## Answers to the knowledge check

::: answers
1. Only `invalidate_key_across_cluster` (DELETE's synchronous leg, with `get_replicas` for the acting primary) in production. `forward_key_over_tcp` also uses it but has no production caller. The result decides which node must acknowledge before 204; it does not place data and does not route reads.
2. Removing a node `x` changes the argmax of a key only if `x` was the maximum, so only keys `x` owned move, each to its next-highest node. In the replica experiment 33,285 of 100,000 keys moved, exactly node 3's share, and 0 keys moved between nodes 1 and 2.
3. (a) Local tenant epoch and global epoch in the cache: change on tenant/full invalidations, including repair escalations. (b) Membership epoch (`registry.version`): static, loaded from the file; does not change at runtime. (c) Per-destination stream sequence: increments with every invalidation sent to a peer. (a) and (c) change at runtime. (A per-key generation is a fourth, local counter.)
4. Local purge (sync) → route and send to primary with 150 ms deadline, possible acting-primary failover (sync) → on ack, reply 204 (sync) → compute fan-out targets → broadcaster per-peer queue with up to 4 attempts (async) → ledger on failure (async) → repair loop and pull loop (async).
5. Calls fail within their deadlines; the peer becomes Suspect; its invalidation goes to the sender's `RepairLedger` (deduplicated, bounded at 1,024 entries and 256 KiB per peer, collapsing to a tenant flush then a full flush on overflow). The repair loop retries every 250 ms with the original sequence once the peer is reachable and not Failed/Offline.
6. It expects 6, sees 8: records a gap 6 to 7, buffers 8, keeps its contiguous acknowledged point at 5. It pulls from the sender, which replays records after 5 for that receiver. If record 6 was evicted from the ring the sender answers `HistoryOverwritten` and the receiver flushes its whole cache.
7. The receiver's cache is in memory, so a restart empties it; nothing stale remains. The issuer's owed invalidations are in its memory only; a restarted issuer has an empty ledger and its restarted sequences are ignored, so a still-alive peer that missed a message keeps its stale entry until expiry.
8. Overestimate at most `(e/1024) x N`, about 0.27 percent of the counted events, with probability at least `1 - e^-4`, about 98 percent. Counters only increase on collisions, so the estimate is biased up, never down.
9. 100 percent of origin fetches (when a cap is configured). The 5 percent arm exists only in an unwired function.
10. On the live path: the AIMD gate (when configured) and the histogram recording. Not on the live path: Space-Saving, the shadow evaluator, `PieController`.
:::
