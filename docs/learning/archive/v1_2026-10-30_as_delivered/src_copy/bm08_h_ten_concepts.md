# Appendix H: The ten concepts you must be able to explain without help

For each concept you get five things: a plain-English explanation in three sentences, the three key facts with `file::symbol`, the one thing a judge will probe, and the honest answer in two sentences. Practise saying them aloud with the book closed. Status labels are the chapters' labels. Nothing in this handbook is INDEPENDENTLY VERIFIED by a human, and AI reviews are only an AI second look.

## H.1 Cache hit, miss and origin load

**Explain it.** A hit is an answer from memory and a miss needs the origin, the system of record named by `ORIGIN_URL`. The cache exists to turn many requests into few origin requests, because the origin is slow or fragile. The header `X-Cache-Status` tells the caller which happened (HIT, MISS, COALESCED, STALE, BYPASS, BYPASS-AUTH).

**Three key facts.**

1. The gateway handles `GET` in `src/api/server.rs::get_cache_handler` and sets `X-Cache-Status` per branch (Chapter 2, section 2.3.3).
2. A miss counts in `src/core/cache.rs::begin_fetch_atomic` (`cache_misses`) and the leader then calls `src/api/server.rs::fetch_origin_raw`, which opens a new connection each time (Chapter 2, section 2.5.3).
3. Only what a `GET` on the origin returned is stored. There is no write route (Chapter 2, section 2.1.1).

**The judge will probe:** "How fast is it, and what hit rate do you get?"

**Honest answer.** There is no release-profile, head-to-head or multi-host measurement, and the few micro-benchmark and loopback numbers in the repository come with strict limits (Chapter 6, section 6.13). I can show the mechanism and the tests, not a performance claim.

## H.2 Stampede and single-flight coalescing

**Explain it.** A stampede is many simultaneous misses on one key, which would all hit the origin together. Single-flight lets the first miss become the leader and fetch once, while the others wait and share the answer. The origin sees one request instead of many.

**Three key facts.**

1. `begin_fetch_atomic` registers a `SharedOp` in the shard's in-flight map under the shard lock, with at most 1,000 waiters per key (`src/core/cache.rs`, `src/core/coalesce.rs::SharedOp`).
2. `m1_http_tests::test_m1_t13_barrier_stampede_100_clients` asserts 1 MISS, 99 COALESCED and one origin call, on loopback in a debug build (Chapter 2, section 2.5.3).
3. Only the leader takes an origin permit, and a disconnecting client does not cancel the fetch because it runs in a spawned task (`src/api/server.rs::acquire_origin_permit`; Chapter 4, section 4.4).

**The judge will probe:** "Does this work across several gateways, and what if the origin fails?"

**Honest answer.** Coalescing is per gateway, so N gateways can send N concurrent fetches for one key, and when the leader fails all its waiters fail together. Errors are not cached, so the next request tries again.

## H.3 Sharded locking

**Explain it.** A single lock around the whole cache would make every request wait in line. So the cache is split into 8 shards, each with its own mutex, and a key always maps to the same shard. Requests for keys on different shards run in parallel, and requests on the same shard queue briefly.

**Three key facts.**

1. Production builds `PartitionedCache::new(8)` in `src/main.rs`, and no configuration key changes the count (Chapter 4, section 4.2).
2. The shard index is the key hash masked by the shard count minus one, in `src/core/cache.rs::PartitionedCache`.
3. A hit takes the shard mutex twice (`check_stale_serving`, then `begin_fetch_atomic`), the origin call runs outside any lock, and no `std::sync::Mutex` is held across `.await` (Chapter 4, section 4.2).

**The judge will probe:** "Why 8 shards, and what about one hot key?"

**Honest answer.** I found no recorded reason for 8, and a hot key always lands on one shard, so sharding cannot help it. Contention has not been measured on a release build, and no model checker has been run on the locking.

## H.4 TTL, staleness and the stale cap

**Explain it.** Every stored entry has a lifetime, taken from the origin's `s-maxage`, `max-age` or `Expires`, and 60 seconds if the origin says nothing. After that the entry is expired, and the gateway may serve it stale only if the origin's response allowed it with `stale-while-revalidate` or `stale-if-error`. A hard cap, `PHANTOM_MAX_STALE_MS` (24 hours by default), limits that stale window.

**Three key facts.**

1. `src/core/freshness.rs::evaluate_response_freshness` decides cacheability and lifetime (Chapter 4, section 4.8).
2. `PHANTOM_DEFAULT_TTL_MS` is NOT WIRED: `src/config.rs` marks it `Wiring::NotWired` and `src/api/server.rs` passes a literal `60_000` at two call sites.
3. `src/core/cache.rs::check_stale_serving` enforces the stale window and the cap, and there is no gateway ceiling on an origin-set lifetime (Chapter 2, section 2.6).

**The judge will probe:** "Your documents say staleness is at most 24 hours. Is that true?"

**Honest answer.** No: the 24-hour cap bounds only the stale-serving window after expiry, not the freshness lifetime, which an origin can set to hours. The 60-second default applies only when the origin gives no information, and setting `PHANTOM_DEFAULT_TTL_MS` has no effect.

## H.5 Generation and epoch fencing

**Explain it.** A fetch takes time, and during it a key can be deleted or a tenant invalidated. So a fetch records a generation number when it starts and the cache compares it when the result arrives. If the generation or the tenant and global epochs changed, the result goes to the waiters but is not stored.

**Three key facts.**

1. `src/core/cache.rs::finalize_fetch` stores only if `state.generation == op_gen` and the epochs match, and `delete_impl` bumps the generation (Chapter 4, section 4.5).
2. `invalidation_contract_tests::test_delete_fences_in_flight_fill_race` runs the delete-during-fetch case on one deterministic ordering and asserts nothing is stored and `stale_writes_prevented` rose.
3. Tenant invalidation raises an epoch (`try_invalidate_tenant`, `get_effective_epochs`), and readers use an `RwLock` snapshot without the `tenant_epochs` mutex.

**The judge will probe:** "Is there a race, and has it been reproduced or fixed?"

**Honest answer.** Yes: G4A-01 is a gap between two epoch reads in `finalize_fetch`, confirmed only in a sandbox with a test hook, and the small fix is NOT integrated. A twenty-second unhooked stress found nothing, which does not bound the probability.

## H.6 Invalidation and what a 204 means

**Explain it.** Invalidation means "forget what you saved". A `DELETE` purges this gateway first, then in cluster mode asks one chosen primary to acknowledge within 150 ms, and tells the other members in the background. The `204` is returned after the primary's acknowledgement.

**Three key facts.**

1. `src/api/server.rs::delete_cache_handler` purges locally, then calls `src/dist/router.rs::invalidate_key_across_cluster` (Chapter 2, section 2.7.3).
2. The timeout is `PHANTOM_CLUSTER_TIMEOUT_MS`, default 150, and a silent primary triggers failover with `X-Cache-Warning: PrimaryFailover`.
3. Other members go through a per-peer queue and the in-memory `RepairLedger` (`src/dist/router.rs`), which is lost if the issuing gateway dies (Chapter 5, section 5.5).

**The judge will probe:** "Does 204 mean the object is gone everywhere?"

**Honest answer.** No: a 204 means the primary acknowledged within the window, and for about one key in N the primary is the issuing gateway itself. The other members are invalidated asynchronously with retry and repair while the issuer lives.

## H.7 Rendezvous hashing and why GET is local-only

**Explain it.** Rendezvous hashing scores each node against the key and picks the highest score, so every gateway computes the same winner without talking. Here the winner is the primary that must acknowledge a `DELETE`. Reads never use it: each gateway answers `GET` from its own cache.

**Three key facts.**

1. `src/dist/router.rs::RendezvousRouter::route_key` uses `DefaultHasher` and `capacity_weight = 1.0`, so it is plain rendezvous hashing, with two passes that skip unhealthy nodes (Chapter 5, section 5.3).
2. `forward_key_over_tcp` has no caller, and `get_cache_handler` never touches the router (Chapter 2, section 2.7.4).
3. The peer `GetRequest` arm in `src/dist/node.rs` fabricates `val_node_<id>`, a recorded defect (DR-029), not a feature (Chapter 5, section 5.8).

**The judge will probe:** "Is this a distributed cache?"

**Honest answer.** It is N independent caches joined by an authenticated invalidation bus, not a partitioned cache, and weighted hashing and routed reads are DESIGNED BUT NOT IMPLEMENTED. What the hash buys is one acknowledging authority per key and a stable failover order for `DELETE`.

## H.8 The failure model, and what is NOT guaranteed

**Explain it.** A guarantee is only meaningful with a stated failure model, a bound and a test that could prove it wrong. On one gateway, a read after a completed `DELETE` does not return the deleted entry, subject to the open G4A-01 and G4A-08 (Chapter 4). Across gateways, invalidation is best-effort and converged in loopback tests (debug build, one host) when the issuer stayed alive and the network healed; that is not a guarantee.

**Three key facts.**

1. The failure table in Chapter 5, section 5.6 covers message loss, receiver crash and transient partition, and marks issuer crash, permanent partition and a malicious member as NOT covered.
2. Propagation latency was measured on loopback in a debug build with 3 mTLS gateways (`tests/integration/cluster_propagation_latency_tests.rs`; claims ledger C6), and "zero stale after 50 ms" is recorded as unsupported.
3. The durable owed-invalidation log exists only as a sandbox patch in `docs/research/m3_durability/` and is PROTOTYPE ONLY.

**The judge will probe:** "Is it linearizable? Do you guarantee bounded staleness?"

**Honest answer.** No to both: reads are local, so cross-node linearizability is absent, and the only staleness bound after an issuer crash or permanent partition is the entry's own freshness lifetime. All cluster evidence is single-host loopback, and multi-host behaviour is UNVERIFIED.

## H.9 The origin concurrency gate, and what the evidence shows

**Explain it.** The gate limits how many distinct origin fetches may run at once, to protect a slow origin. It can be a fixed cap, or the cap can move by additive-increase, multiplicative-decrease: cut by 30 percent when a window looks bad, raise by one when it looks good. By default it is off.

**Three key facts.**

1. `src/intelligence/executor.rs::origin_cap_enforcing` is true only in `canary` or `active` mode with `PHANTOM_ORIGIN_CAP_MAX` set, and `off` mode is uncapped (Chapter 4, section 4.4).
2. `OriginConcurrencyCapController::step` in `src/intelligence/candidates.rs` does the arithmetic, and `origin_cap_tests::coalesced_waiters_do_not_consume_permits` shows waiters take no permit.
3. In Scenario B the adaptive arm fell from 32 to about 12 and stayed there (`src/lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`), and both capped arms were labelled `TRADE_OFF_FAILURE` (Chapter 7, section 7.5.2).

**The judge will probe:** "Does adaptive control improve anything?"

**Honest answer.** Not on current evidence: the sealed simulation found H1 NOT_SUPPORTED, and Scenario B showed a cap limits origin concurrency at a cost in refused requests, with the adaptive arm acting like a static cap. A static cap is the defensible feature, and it is not AI.

## H.10 Simulation versus real execution, and the M6 negative result

**Explain it.** A simulation models the system and the clock in software, so it is repeatable but is not the real program. A real run uses the actual gateway on real sockets, but here only on one machine over loopback. The project keeps them apart: T1 is SIMULATED and T2 is MEASURED_LOOPBACK.

**Three key facts.**

1. T1 runs the real cache and gate code with a virtual clock and a simulated origin, through `src/lab/t1_sim.rs::run_cell`. It has no HTTP, TLS or cluster (Chapter 7, section 7.2.6).
2. T2 starts the real gateway binary against a scripted origin (`src/lab_live/scenario_b.rs::run_experiment`), and no T1 versus T2 agreement study has been run (Chapter 7, section 7.5.3).
3. M6 was preregistered and sealed: 30 seeds, 8 scenarios, H1 NOT_SUPPORTED, and the tuned adaptive optimum was itself a static cap (`docs/research/m6_sealed_results.md`; Chapter 7, section 7.6).

**The judge will probe:** "How do you know a simulator reflects reality, and why report a failure?"

**Honest answer.** I do not claim it does: the simulator tests policy logic and cache semantics against a modelled origin, and its predictive power is UNVERIFIED. I report the negative result because the rule was fixed before the data, and hiding it would be worse than not having a positive one.

## H.11 Self-test: ten questions

Answer each aloud in under a minute, then check the chapter.

1. Name the six `X-Cache-Status` values, and say what a MISS means. (Chapter 2, section 2.3.3)
2. A hundred clients ask for one missing key. What does the origin see, and what does that not prove? (Chapter 2, section 2.5.3)
3. Why 8 shards, what does one hot key do to them, and how many times does a hit lock its shard? (Chapter 4, section 4.2)
4. Which setting is NOT WIRED, what actually sets the default lifetime, and what does `PHANTOM_MAX_STALE_MS` bound? (Chapter 4, section 4.8)
5. Walk through a `DELETE` that arrives while a fetch is running. Which numbers are compared, and where is the open race? (Chapter 4, sections 4.5 and 4.7)
6. What exactly does a 204 from `DELETE` tell the client in cluster mode? (Chapter 5, section 5.5)
7. Why is `GET` local-only, what does the hash decide, and what is the `GetRequest` stub? (Chapter 5, sections 5.3 and 5.8)
8. State the failure model, the honest consistency statement, and the one test that would falsify a staleness bound. (Chapter 5, section 5.6)
9. When is the origin gate enforced, what does AIMD do, and what did Scenario B show about the adaptive arm? (Chapter 5, section 5.13; Chapter 7, section 7.5.2)
10. What is the difference between T1 and T2, what did M6 conclude, and what may the project not claim from it? (Chapter 7, sections 7.5 and 7.6)
