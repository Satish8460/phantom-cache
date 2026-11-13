# Appendix B: The complete architecture, explained once

This appendix tells the whole story in one pass, from a client's request to the origin and across the cluster. It does not re-teach. Each paragraph points to the chapter section that owns the detail. Status labels are the ones the chapters use: IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, NOT WIRED, UNVERIFIED. Nothing in this handbook is INDEPENDENTLY VERIFIED by a human.

![Figure B.1: PhantomCache architecture overview. One gateway process, the origin outside it, peer gateways reached only by invalidation frames, and the Lab and UI off the request path.](../assets/bm02_architecture_overview.svg)

Source: Figure 2.1 (`ch02_components.svg`), Figure 5.1 (`ch05_cluster_topology.svg`), Figure 2.6 (`ch02_lab_ui.svg`); `src/main.rs::run`, `src/api/server.rs::create_app_with_lab`, `src/core/cache.rs::PartitionedCache`, `src/dist/router.rs::RendezvousRouter`, `src/intelligence/executor.rs::OriginGate`.

## B.1 What the system is, in one paragraph

PhantomCache is a read-through caching gateway: a Rust program that sits in front of one origin, which is the operator's own data service named by `ORIGIN_URL`. Clients call `GET /api/v1/cache/<path>` and `DELETE /api/v1/cache/<path>` on the gateway. The gateway answers reads from memory when it can and from the origin when it must. There is no write route: to change data you change the origin and then `DELETE` the key, or wait for the entry to expire (see Chapter 2, section 2.1). It is a research prototype. The single-node engine is IMPLEMENTED and TESTED. The cluster is experimental: fan-out, repair and anti-entropy are IMPLEMENTED and TESTED, but only on one host over loopback, and durable invalidation and coordinated reads are not done (see Chapter 8, section 8.12).

## B.2 The process and what runs in it

One OS process, `phantom-cache-demo`, runs a Tokio multi-thread runtime and an Axum router (see Chapter 2, section 2.1.2 and Chapter 3, section 3.3). Inside it live the ingress, the handlers, one `PartitionedCache`, an optional cluster router and listener, and background tasks: a sweeper every 100 ms by default, a metrics sampler, and a server-sent-events feed. The React UI is a separate Vite dev server on port 3000. The gateway does not serve it. The Lab is a separate tool (`phantom-lab`) or, when `PHANTOM_LAB_ENABLED=true`, an extra set of routes (see Chapter 2, sections 2.1.3 and 2.8). The component table is in Chapter 2, section 2.2.1.

One caution about the word "node". In the standalone gateway there is one real cache plus three placeholder `VirtualNode` objects for dashboard demos. Those demo nodes are PROTOTYPE ONLY and synthetic. In the mTLS cluster configuration the local member is built with `VirtualNode::new_with_cache`, so it shares the gateway's serving cache, and remote members are address-only descriptors with no cache in this process (see Chapter 2, section 2.2.1).

## B.3 The front door: ingress, authentication, tenants

A request first meets the router layers: CORS and a 512 KiB body limit. If `PHANTOM_JWT_SECRET` is set, the handler authenticates the caller. The JWT algorithm is HS256 only. The check covers signature, expiry, issuer and audience, then a role: `cache:read` to read, `cache:invalidate` to delete. A non-empty `tenant_id` claim is required. In secure mode the tenant comes only from the verified token, never from the `x-tenant-id` header (see Chapter 6, section 6.2). With no secret the gateway starts only on a loopback bind, and a non-loopback bind without a secret is refused (see Chapter 2, section 2.3.1).

The cache key is the tenant plus the path plus the raw query. With no tenant it is the literal `__default__`. `?a=1&b=2` and `?b=2&a=1` are two keys. A key is validated for length and unsafe characters before use (see Chapter 2, section 2.3.2 and Chapter 4, section 4.1). Tenancy here is key isolation. There are no per-tenant quotas, and any pinned cluster member can flush any tenant's cache (see Chapter 6, section 6.6 and Chapter 8, section 8.3.5).

Known open items at this door: the handlers read the body before they authenticate (G4B-01, OPEN, owner-named), there is no rate limit, there is no token revocation and there is no `nbf` check (see Chapter 6, section 6.2). These are listed rather than hidden. Authentication is IMPLEMENTED and TESTED for the covered paths. The system is not security-certified and has had no penetration test (see Chapter 6, section 6.1).

## B.4 The cache core: 8 shards

The cache is split into 8 shards. Production code builds `PartitionedCache::new(8)` in `src/main.rs`, and no configuration key changes it. Each shard is one `std::sync::Mutex` around a map of keys, an in-flight map, byte counters and one eviction policy. The shard for a key is the hash of the key masked by 7 (see Chapter 4, section 4.2).

Different keys on different shards never wait for each other. Different keys on the same shard queue for a short critical section. One hot key always lands on one shard, so sharding cannot spread it. A hit takes the shard mutex twice in the HTTP path (once in `check_stale_serving`, once in `begin_fetch_atomic`) and the response body is copied. Contention has not been measured on a release build (see Chapter 4, section 4.2 and Chapter 6, section 6.13).

The rule that holds the design together: never hold a `std::sync::Mutex` across `.await`. The leader runs the origin call in a spawned task that holds no shard lock while it waits (see Chapter 3, section 3.5 and Chapter 4, section 4.2). No model checker has been run on the locking, so "no deadlock" is argued from reading, not proven (see Chapter 4, section 4.2).

## B.5 A hit

If the key's entry exists, is not past its lifetime and carries the current tenant and global epochs, the gateway returns it with `X-Cache-Status: HIT` and an `Age` header. Nothing leaves the process. The hit path is IMPLEMENTED and TESTED in `m1_http_tests` (see Chapter 2, section 2.4). Micro-benchmark numbers for the hit path exist, but none is an established release-profile product figure. Quote them only with the conditions given in Chapter 6, section 6.13.

## B.6 A miss: single-flight, then the origin

On a miss, `begin_fetch_atomic` runs under the shard lock and has three outcomes. If a fetch for that key is already in flight, the caller joins its waiter list (at most 1,000 per key, else 503 `WaiterLimitExceeded`). Otherwise the shard evicts, at most 32 victims, to make room, and if there is still no room the request fails with `CapacityExceeded`. Otherwise it records an operation id and the key's current generation and registers the fetch. This request is the leader (see Chapter 2, section 2.5 and Chapter 4, section 4.4).

The leader fetches in its own task, so a client that disconnects does not cancel the fetch for others. Only the leader, a bypass request, or a background revalidation leader takes an origin permit. Waiters never do (see Chapter 4, section 4.4).

### B.6.1 The origin gate

An optional concurrency gate limits distinct origin fetches. It is enforced only when `PHANTOM_INTELLIGENCE` is `canary` or `active` and `PHANTOM_ORIGIN_CAP_MAX` is set. In the default mode `off` the origin path is uncapped, and the gate only counts (`origin_cap_enforcing`). When it refuses, the caller gets a stale copy if one is eligible, else 503 with `Retry-After: 1`. The cap can be static, or can move by an AIMD rule (multiply by 0.70 when the window's p99 or error rate is bad, otherwise add 1). IMPLEMENTED and TESTED for its arithmetic and permit accounting. The benefit of the adaptive version is NOT established (see Chapter 4, section 4.4 and Chapter 5, section 5.13).

### B.6.2 The fetch itself

`fetch_origin_raw` checks the origin against the upstream policy, resolves the name, filters every returned address against loopback, private and metadata ranges, pins the validated address and connects. Timeouts are 5 s per phase, the body cap is 2 MiB and the header cap is 64 KiB. Redirects are not followed. Each fetch opens a new connection (`Connection: close`). The client's `Authorization` header is not forwarded unless `PHANTOM_FORWARD_UPSTREAM_AUTH` is set. The origin host is fixed by the operator, so the main SSRF risk is removed by design (see Chapter 2, section 2.5.3 and Chapter 6, section 6.8).

### B.6.3 Store, fencing and waiters

When the answer arrives, `finalize_fetch` decides whether the cache may keep it, under the shard lock. The body is stored only if the key's generation still equals the recorded one, the state object is the same one, and the tenant and global epochs still match. `DELETE` bumps the generation, so a slow fetch cannot resurrect a deleted key. Otherwise the body goes to the waiters but is not stored, and `stale_writes_prevented` increases (see Chapter 4, section 4.5).

The mechanism is IMPLEMENTED and TESTED, with one honest open item. `finalize_fetch` reads the epochs twice, and an epoch bump that lands between the reads can let an old body be stored with the new stamp. This is G4A-01. It was reproduced only in a sandbox with a test hook, and the small fix lives in `docs/research/g4a01_race/proposal.patch` and is NOT integrated (see Chapter 4, section 4.7). A request that joins an older in-flight fetch after a tenant or global epoch change can also receive the older body once (G4A-08). The generation fence catches `DELETE`. Only epoch equality catches tenant or global invalidation (see Chapter 4, section 4.5).

Cancellation and panics are handled by guards that finalize a fetch as aborted if it is dropped early. By design, eviction skips a key with an active operation or in-flight fetch; this is argued from reading and tested, not model-checked (see Chapter 4, section 4.5).

## B.7 Memory, freshness and eviction

**Bytes.** Each shard keeps a running count of logical bytes: key length, body capacity, content-type and ETag lengths and a fixed per-entry overhead. `audit_bytes()` recomputes the total so tests can compare. This proves the bookkeeping is consistent. It is not the operating system's memory figure, and some structures are not counted (see Chapter 4, section 4.6). The default total budget is 256 MiB, divided by the shard count, plus an entry limit per shard.

**Freshness.** The lifetime of an entry comes from the origin: `s-maxage`, `max-age`, or `Expires` minus `Date`. If the origin says nothing, the gateway uses 60,000 ms. That 60 s is a literal in `src/api/server.rs`, passed at two call sites. The setting `PHANTOM_DEFAULT_TTL_MS` is validated and reported but is NOT WIRED: setting it changes nothing. The documented default and the effective default match only by coincidence. There is no gateway ceiling on an origin-set lifetime (see Chapter 4, section 4.8 and Chapter 2, section 2.6.1).

**Stale serving.** Expired data is served only if the origin's response carried `stale-while-revalidate` or `stale-if-error`, and always inside the hard cap `PHANTOM_MAX_STALE_MS` (24 h by default). The cap bounds the stale-serving window, not the freshness lifetime. `must-revalidate` forbids stale serving. A 304 reply refreshes the stored entry. There is no circuit breaker, no negative caching and no retry loop. Errors are not stored, so recovery is immediate when the origin returns (see Chapter 2, section 2.6 and Chapter 4, section 4.8). Freshness is PARTIAL: it is an RFC 9111 subset with open conformance findings (see Chapter 4, section 4.11).

**Eviction.** The default policy is SIEVE. LRU and S3-FIFO are selectable. All three implement `ShardEvictionPolicy` and skip pinned keys. They are not worst-case O(1) when keys are pinned, S3-FIFO carries an open defect (G4A-02), and there is no hit-ratio or release-profile comparison (see Chapter 4, section 4.9).

**Invalidation inside one process.** `DELETE` removes one key and bumps its generation in O(1). A tenant invalidation raises the tenant's epoch, and entries with an older stamp are treated as absent the next time they are touched or when the sweeper finds them. Readers get the epochs from an `RwLock` snapshot without taking the `tenant_epochs` mutex, but the shard mutex is still taken. The tenant map is capped at 10,000. A new tenant arriving when full raises the global epoch, which invalidates everything on that node. That fallback is peer-triggerable (G4A-12). The sweeper scans each shard in full under its lock (G4A-03) (see Chapter 4, section 4.7).

## B.8 DELETE and the cluster

A `DELETE` purges the local cache first, so later reads on this gateway miss. With no router that is all, and the reply is 204. With a router, the gateway uses rendezvous hashing to choose a primary for the key and sends one `InvalidateRequest` over mutual TLS. It waits at most 150 ms (`PHANTOM_CLUSTER_TIMEOUT_MS`) for an `InvalidateAck`. A 204 means the primary acknowledged in that window. It does not mean every member has purged. If the primary is silent, it is marked Suspect and the first healthy replica acts as primary (`X-Cache-Warning: PrimaryFailover`). If that also fails the client gets 504 and every other member is recorded as owed. The local purge happens in every case (see Chapter 2, section 2.7.3 and Chapter 5, section 5.5).

**Rendezvous hashing** gives each node a score from the key and the node id; the highest score wins. In this code the hash is `DefaultHasher`, the weight factor is the constant 1.0, and the router makes two passes (healthy nodes first, then Suspect and Recovering). So it is plain rendezvous hashing, not weighted. Weighted hashing is DESIGNED BUT NOT IMPLEMENTED. Its use is narrow: it picks which node gives the acknowledgement for a `DELETE`. It does not decide where data lives. About 1 in N keys has the issuer itself as primary, and then the 204 carries no evidence about any other node (see Chapter 5, section 5.3).

**GET is local-only.** `get_cache_handler` never touches the router. The function that would forward a read to the key's owner, `forward_key_over_tcp`, has no caller. The peer-side `GetRequest` handler is a stub that fabricates a value, which is a recorded defect (DR-029), not a feature. So a cluster of N gateways is N independent read caches kept coherent by invalidation, not a partitioned distributed cache. Coordinated reads are DESIGNED BUT NOT IMPLEMENTED (see Chapter 2, section 2.7.4 and Chapter 5, section 5.8).

**The rest of the propagation.** After the primary's ack, every other member (not only replicas) is told asynchronously. A per-peer queue of 256 and up to four attempts handle live members. Members that are not live, and deliveries that fail, go to a bounded repair ledger. A repair loop every 250 ms re-sends owed entries, and a pull-based anti-entropy worker (also 250 ms) repairs sequence gaps using a ring of 4,096 sent records, or flushes the whole receiver cache when history was overwritten (see Chapter 5, section 5.5).

**Membership and transport.** Membership is a static file read at startup. There is no join or leave protocol. The wire format is the PhantomMesh binary frame: magic `PHAN`, a 28-byte header, a payload of at most 1 MiB and keys of at most 4,096 bytes. Connections use TLS 1.3 only with a cluster CA as the sole trust root. Identity is a single DNS SAN `node-<id>.<cluster_id>.phantom.internal`, pinned to a fingerprint in the registry. Every frame is re-checked, and an `InvalidateRequest` whose originator differs from the TLS identity closes the connection. Heartbeats are connection keepalive, not failure detection (see Chapter 5, section 5.2 and Chapter 6, section 6.4). Members are trusted: a compromised member can flush others (G4B-05, DR-027).

**What is and is not guaranteed.** On one gateway, a read after a completed `DELETE` does not return the deleted entry, subject to the open G4A-01 and G4A-08. Across gateways there is best-effort asynchronous invalidation with retry, a bounded in-memory ledger and pull repair, which converged within milliseconds in loopback tests while the issuer stayed alive. The repo does not provide linearizability, and it provides no staleness bound if the issuer crashes with owed invalidations or if a partition is permanent, other than the entry's own freshness lifetime. The durable-log fix is a sandbox patch, PROTOTYPE ONLY, and is not in the tree. All cluster evidence is single-host loopback in a debug build (see Chapter 5, sections 5.6, 5.7 and 5.9; Chapter 8, section 8.11).

## B.9 Observation and intelligence

**Telemetry.** A latency histogram `AtomicLogLinearHistogram` has 2,688 buckets and constant-time recording. Its design error is at most 0.39 percent. In the current tree it is written on every GET but no production endpoint reads it. The percentiles on `/api/v1/metrics` come from a sorted 10,000-sample window. Counters and an origin-latency histogram feed the observation routes (see Chapter 5, section 5.11). Every displayed value should carry a provenance label: measured, simulated, inferred or unavailable. With `PHANTOM_SIMULATION=true` parts of the live event stream are synthetic (see Chapter 2, section 2.8).

**Intelligence.** It is not AI and not machine learning. On the serving path there is only the origin gate described in B.6.1. Everything else is library code that no request calls: the signal pipeline (Count-Min plus Space-Saving heavy hitters, EWMA, CUSUM), the shadow evaluator, the decision engine with its confidence interval, canary routing and a watchdog. These are PROTOTYPE ONLY, and the statistics in the evaluator and engine have recorded flaws (F12, F13). `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test. The legacy `src/control/pie.rs` is a threshold stub, and `main.rs` passes `pie: None`. The hot-keys field of `/api/v1/metrics` reports "unavailable". Hot keys are in practice protected by coalescing (see Chapter 5, sections 5.10, 5.12 and 5.13). The hard caps (stale cap, origin cap, byte budget, fencing) are enforced by the execution layer regardless of any intelligence decision (see Chapter 5, section 5.13 D).

## B.10 Phantom Lab and the UI

Phantom Lab has two tiers, kept apart. T1 is a deterministic discrete-event simulation with a virtual clock and seeded random streams. It drives the real cache, coalescing, fencing and origin-gate code but simulates clients and the origin, and it does not run HTTP, TLS or any cluster. Its results are labelled SIMULATED. T2 starts the real gateway binary as a child process on loopback, against a scripted raw-TCP origin, with an open-loop load generator. Its results are labelled MEASURED_LOOPBACK, on one host, and are not multi-host or production figures (see Chapter 7, sections 7.2 and 7.5).

Experiments compare arms paired by seed, using a BCa bootstrap interval and a sign-flip permutation test, with Holm correction across hypotheses (see Chapter 7, section 7.3). The sealed M6 evaluation (30 sealed seeds, 8 scenarios, simulated) gave hypothesis H1 the verdict NOT_SUPPORTED: a forced-adaptive controller did not beat the best tuned static cap. The best tuned adaptive setting was itself a static cap. Scenario B on a real gateway showed that a cap limits origin concurrency at a cost in refused requests, and that the adaptive arm acted as a static cap near 12. No agreement study between T1 and T2 has been run (see Chapter 7, sections 7.5 and 7.6). The product position that follows is a gateway with a configurable static origin cap.

The Lab API (`src/api/lab_api.rs`) exists only if `PHANTOM_LAB_ENABLED=true`. Without JWT it requires a loopback bind and checks the `Host` and `Origin` headers. The service refuses to start if the sealed salt is present in its environment (see Chapter 2, section 2.8 and Chapter 7, section 7.7). The React dashboard shows telemetry and the Lab. Its Cluster and Intelligence screens are placeholders. It has automated checks only, with no human design or accessibility review (see Chapter 8, section 8.12).

## B.11 Security boundaries in brief

Five zones: clients, the gateway process, the operator-fixed origin, cluster peers and the local host (see Chapter 6, section 6.1). At each boundary the repo has a named check: JWT roles and tenant claims at the ingress; the upstream policy, DNS pinning and an address filter at the origin; certificate pinning and originator matching on the cluster link; a loopback-only rule with Host and Origin checks on the Lab API. The bounds that exist are listed in Chapter 6, section 6.8 (bytes, waiters per key, epoch map, response size, frame size, handshakes, per-peer queues, ledger size). The bounds that do not exist are listed there too: inbound connection count, `/api/events` streams, origin concurrency by default, per-tenant rates. The host environment holds the JWT secret, so a host-level attacker is out of scope.

## B.12 Where the architecture stops

Seven limits shape every defence answer (see Chapter 8, section 8.3).

1. One cache per gateway, served locally.
2. Placement is plain rendezvous hashing, used for `DELETE` only.
3. Owed invalidations live in the issuer's memory.
4. Freshness bounds are weak: the 60 s default is a literal, and origin lifetimes have no ceiling.
5. Tenancy is key isolation, not resource isolation.
6. Evidence is single-host, Windows, debug-profile for the suite, with no release-profile performance claim and no multi-host run.
7. Operations gaps: no metrics export, no config reload, no container image, no advisory scan.

Passing tests establish that an author's expectation held in a build on a host. They do not establish correctness (see Chapter 8, section 8.2.2). Reviews so far are AI second looks, not independent verification.
