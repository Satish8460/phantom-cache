# Appendix E: Consolidated judge-question bank

This appendix collects every judge box from Chapters 1 to 8 into one bank, organised by theme.

**Count.** Counted with `grep` on the fenced-div opener `::: judge`, the chapters hold 93 judge boxes: Chapter 1, 7; Chapter 2, 9; Chapter 3, 10; Chapter 4, 12; Chapter 5, 16; Chapter 6, 15; Chapter 7, 12; Chapter 8, 12. All 93 are below, each exactly once. The themes hold: Fundamentals 5, Architecture 8, Technology 11, Algorithms 13, Distributed systems 15, Security 11, Innovation 7, Evaluation 12, Limitations and honesty 11.

**How to use it.** Each entry gives the question, a short answer taken in substance from the chapter, a pointer to the section where the topic is taught, and the evidence to know. The answers are compressed. They keep the chapter's qualifiers, status labels and open-finding caveats, and they are never stronger than the chapter. Before an oral defence, read the full box in the chapter: it also has the "if the judge asks further" follow-ups. Status labels have the meaning given in Ch 8, 8.1. Nothing in this project is INDEPENDENTLY VERIFIED; every review on record is an AI second look.

## Ten hard questions to rehearse first

These ten are the questions where an overclaim does the most damage. Rehearse them until the honest answer comes out without hedging in the wrong places.

1. A DELETE returned 204. Is the object gone from every node? (DIS-8, DIS-7)
2. Is it linearizable? Do you guarantee bounded staleness? (DIS-12, DIS-13)
3. What if the gateway that issued the delete crashes before the others hear? (DIS-15)
4. Is there a race that could resurrect a deleted value, and is it fixed? (ALG-4, ALG-5)
5. What is the actual intelligence? Is it AI? (INN-4, INN-5)
6. Your own experiment says the adaptive controller does not beat a static cap. Why should anyone use it? (INN-6)
7. Who has independently verified this system? (LIM-2)
8. What are your performance numbers? (EVA-11)
9. Your tests pass. Why should I believe the system is correct? (EVA-12, LIM-1)
10. Where would a hostile reviewer attack first? (SEC-11)

## Never say

These are the tempting overclaims from Ch 8, 8.11, each with the accurate sentence to say instead.

| Do not say | Say instead |
|---|---|
| "PhantomCache is production-ready." | A research prototype with an experimental cluster mode; no release-profile suite, no advisory scan, open High findings, run only on Windows. |
| "It is a distributed cache." | Each gateway has its own cache; the cluster adds an authenticated invalidation bus with repair. GET is served locally. |
| "DELETE returns 204, so the object is gone everywhere." | 204 means the canonical primary acknowledged within a 150 ms leg; others are told asynchronously, and owed invalidations are lost if the issuer dies. |
| "Invalidations survive crashes." | Not in the tree. A sandbox patch with an opt-in log exists and is not integrated. |
| "Zero stale reads after 50 ms" or "bounded staleness." | Not supported. The only gateway bound is the origin lifetime or 60 s; the stale cap bounds stale serving only. |
| "Linearizable" or "strongly consistent." | No. Per-node ordering and eventual convergence under the loopback failure model that was tested. |
| "Adaptive or intelligent caching improves performance." | Not established: H1 NOT_SUPPORTED (simulated); Scenario B TRADE_OFF_FAILURE for both arms. It shows no benefit for this controller on these plants, not that adaptation cannot work. |
| "The PIE is a working canary controller." | `src/control/pie.rs` is a legacy threshold stub, `pie: None` in `main.rs`; `canary` mode applies the cap to all fetches with no rollback in the serving path. |
| "It uses machine learning." | No ML ships. An ML selector is gated and its negative claim is itself unverified. |
| "PhantomCache is fast" or "faster than Redis, Nginx or Varnish." | The repository contains benchmark results with a stated profile, host and limits; none establishes a release-profile or comparative claim, and this book makes no superiority claim (Ch 6, 6.13). |
| "Eviction is 200 times faster" or "controller overhead is below 1% CPU." | Untraceable; no release-build measurement backs them. |
| "Independently audited or verified." | Reviewed by AI second looks; no human or external reviewer has checked anything. |
| "Secure by design / hardened." | Authentication, roles, mTLS and SSRF filtering exist and are tested; open items include the pre-auth body read, live-control mutation, member flush capability, no rate limits and no advisory scan. Not security-certified. |
| "Multi-tenant isolation with quotas." | Tenant-keyed entries and tenant epochs only; no quotas; any authenticated member can flush any tenant. |
| "`PHANTOM_DEFAULT_TTL_MS` sets the default lifetime." | Validated but not wired; the effective default is always 60 s. |
| "SIEVE, LRU and S3-FIFO are benchmarked and superior." | Faithful implementations exist; no release-grade comparison; S3-FIFO has a tombstone defect (unreachable by default). |
| "The UI is finished / accessible." | Increments I1 to I4 are implemented and pass automated checks; visual quality is not approved; no screen-reader or human usability test exists. |
| "Everything is saved / committed." | Almost everything is uncommitted; one local backup on the same machine. |
| "Tests passing means it is correct." | 851 debug tests pass; several assert stub behaviour. |

## 1. Fundamentals

### FUN-1. Why is caching necessary at all? Why not just make the database faster?

**Answer.** A faster database helps but does not remove the basic problem: most traffic repeats, and a cache answers the repeat from memory so it never reaches the database. Database load drops by roughly the miss rate (at a 95% hit ratio, one request in twenty gets through; illustrative numbers). Caching only works when requests repeat and a slightly old answer is acceptable. No speed-up is claimed for PhantomCache: the repository has no release-build performance measurement.

**Where taught:** Ch 1, 1.2. **Evidence to know:** `docs/user/limitations.md`; `docs/STATUS.md` section 3; counters in `PartitionedCache`; IMPLEMENTED (counters), performance benefit UNVERIFIED on a release build.

### FUN-2. Why not let the application query the database directly and keep the architecture simple?

**Answer.** For a low-traffic service that is the right answer; a cache is an extra moving part. You need one when callers repeat reads, the database is scarce or traffic surges. The cost is a second system that can be stale, so you must be able to say how stale. The origin is any HTTP server behind `ORIGIN_URL`, and the tests use scripted or mock origins, which show correct coalescing and caching but not how a real production database behaves.

**Where taught:** Ch 1, 1.2. **Evidence to know:** `src/main.rs` (`ORIGIN_URL`); `tests/integration/m1_http_tests.rs`; `docs/demo/scenario_b_interpretation_notice.md` item 4; IMPLEMENTED, TESTED against scripted origins only.

### FUN-3. What does caching introduce that you did not have before? What are the costs?

**Answer.** Speed and origin protection, paid for with four problems: stale data, invalidation (a race against in-flight fetches), stampedes and hot keys. PhantomCache addresses stampedes with coalescing and the invalidation race with generation fencing, and bounds stale serving with a configurable cap. It eliminates none of them: staleness is bounded only by the origin's lifetime or a 60-second default, and the cluster invalidation path is experimental. Memory pressure and one more component to operate are further costs.

**Where taught:** Ch 1, 1.3.4. **Evidence to know:** `PartitionedCache::begin_fetch_atomic`, `::finalize_fetch`, `::delete`; `src/config.rs`; `coalesce_tradeoffs_tests.rs`; IMPLEMENTED, TESTED (loopback, debug); G4A-01 UNDER-INVESTIGATION.

### FUN-4. Why do you need a distributed cache rather than one big cache node?

**Answer.** One node has a ceiling on memory, CPU and network and is a single point of failure. Several gateways raise throughput and survive one failure, but then you must keep copies consistent. Today the main distributed feature is reliable invalidation: a delete is sent to every node with retries and anti-entropy. Reads are not distributed, so it is a set of independent gateways with a shared invalidation bus, not a pooled cache; capacity does not add up and the origin can see up to N times one node's load.

**Where taught:** Ch 1, 1.4. **Evidence to know:** `get_cache_handler` (local only), `delete_cache_handler`; `src/dist/router.rs::route_key` (weight 1.0); the "[EXPERIMENTAL] Cluster mode" message; invalidation fan-out IMPLEMENTED, TESTED (loopback, debug build, at most four nodes), effectiveness UNVERIFIED; peer reads and durability NOT IMPLEMENTED.

### FUN-5. In one minute, what is PhantomCache?

**Answer.** An in-memory HTTP caching gateway in Rust that answers repeat requests from memory and sends one origin request when many clients ask for the same missing item (single-flight coalescing). It bounds stale serving, tracks bytes against a budget, keeps tenant keys separate and supports authenticated delete; several gateways can exchange invalidations over mutual TLS. On top sit an experimental layer that can cap origin concurrency and a lab for reproducible experiments. It is a research prototype: there are tests for the engine and invalidation path, but no release-grade numbers, no multi-host run and no measured benefit from the adaptive layer.

**Where taught:** Ch 1, 1.5. **Evidence to know:** `README.md`; `docs/STATUS.md` sections 1 to 4; `docs/user/limitations.md`; `docs/research/m6_sealed_results.md`; IMPLEMENTED/TESTED engine, PARTIAL cluster, PROTOTYPE ONLY intelligence; nothing INDEPENDENTLY VERIFIED.

## 2. Architecture

### ARC-1. Explain the architecture of your system.

**Answer.** A read-through caching gateway: the client sends GET, the gateway looks up an in-memory cache split into eight lock-protected shards; a hit answers from memory, a miss sends one request to the origin while others wait, and the result is stored if still valid. DELETE removes the key locally, tells the node chosen by rendezvous hashing and waits for its acknowledgement, then tells the other members in the background. Optional parts are JWT authentication, an experimental mTLS cluster, an observation API and Phantom Lab. Reads are always served from the local cache, so this is N independent caches with an invalidation bus.

**Where taught:** Ch 2, 2.2.1. **Evidence to know:** `src/main.rs::run`; `src/api/server.rs::create_app_with_lab`, `get_cache_handler`; `PartitionedCache`; IMPLEMENTED, TESTED for the single-gateway path; for the cluster IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED.

### ARC-2. Walk me through what happens to one request, from the network to the response.

**Answer.** The Axum router applies CORS and a body limit; the handler checks the token if authentication is on and takes the tenant from it; it builds and validates a key from tenant, path and query (control characters and dot segments are rejected). The key's hash picks one of eight shards and only that shard's lock is taken. A fresh entry returns with `X-Cache-Status: HIT`; otherwise the miss path runs: one leader fetches, others wait, the result is stored if still valid. The shard mutex is never held across a network call.

**Where taught:** Ch 2, 2.4. **Evidence to know:** `get_cache_handler`, `begin_fetch_atomic`, `CacheKey::validate`; `test_m1_t01_cold_cache_miss`, `test_m1_t02_repeat_cache_hit` (TESTED on loopback, debug, one gateway; they show MISS then HIT and one origin call, not performance).

### ARC-3. What happens on a cache miss? And what if a thousand clients ask for the same missing key?

**Answer.** The first request becomes the leader and fetches the origin in its own task with timeouts and size limits; every other request for that key joins the same operation, so the origin sees one request. The gateway checks the result may be cached and that the key was not deleted or invalidated meanwhile, stores it if so, and releases all waiters. Waiters are capped at 1,000 per key; extra requests get 503. Coalescing is per gateway, so three gateways can fetch the same key three times; the 100-client case was tested on loopback.

**Where taught:** Ch 2, 2.5. **Evidence to know:** `begin_fetch_atomic`, `finalize_fetch`; `test_m1_t13_*`, `serving_contract_tests` (`test_ct1_*`, `test_ct3_*`, `test_ct7_*`); TESTED, loopback, debug. Fencing and G4A-01 are in Ch 4.

### ARC-4. What happens when the origin is unavailable?

**Answer.** A fresh cached key is served as normal. For an expired entry it depends on what the origin allowed: with a stale-if-error or stale-while-revalidate window the expired copy is served, labelled STALE, never beyond the 24-hour hard cap; with nothing, the client gets 504 (timeout) or 502 (network failure). Errors are not cached, and coalescing keeps it to one request per key at a time. There is no circuit breaker, and stale-if-error is tested at cache level, not as an HTTP-level outage.

**Where taught:** Ch 2, 2.6.3. **Evidence to know:** `get_cache_handler`, `check_stale_serving`, `map_origin_error_to_response`; `stale_serving_tests::test_stale_if_error_on_origin_outage` (TESTED at component level); `PHANTOM_MAX_STALE_MS` (`Wiring::Main`).

### ARC-5. The origin goes down. What does the user see, and what protects the origin when it comes back?

**Answer.** A fresh entry is still served; an expired one marked stale-if-error is served as stale up to the 24-hour cap unless marked must-revalidate; a key with nothing usable fails for every request waiting on that fetch, and the next request tries again. On return, single-flight limits a stampede to one request per key; across many keys there is no limit by default, only the optional origin cap, which enforces only in canary or active mode. Stale-if-error is applied to more status codes than the standard allows (G4A-06), and there is no negative caching or circuit breaker.

**Where taught:** Ch 4, 4.10. **Evidence to know:** `stale_serving_tests.rs`, `coalesce_tradeoffs_tests.rs`, `origin_cap_tests.rs`; G4A-06; IMPLEMENTED, TESTED on loopback with in-process mock origins.

### ARC-6. Your configuration lists a default TTL setting. Does it work?

**Answer.** No, and the product says so. The key is validated and printed, but the freshness engine is called with a fixed 60 seconds in two places in the server; the configuration table marks it not wired and the startup report prints "not honored by runtime". Entries with explicit `Cache-Control` or `Expires` use the origin's lifetime; only responses with no freshness information fall back to 60 seconds, and that cannot be changed by configuration today. Fixing it is an application-code change that needs approval and a test.

**Where taught:** Ch 2, 2.6.1 (box in 2.6.3); Ch 4, 4.8. **Evidence to know:** `src/config.rs::KEYS` (`PHANTOM_DEFAULT_TTL_MS`, `Wiring::NotWired`); the two `60_000` literals in `get_cache_handler`; `docs/user/limitations.md`; PARTIAL.

### ARC-7. Is the dashboard showing real data? And how does the Lab relate to the real system?

**Answer.** The observation endpoints return counters and latency measured in the running gateway, each field meant to carry a provenance label. Some values, such as the three demo nodes and the startup snapshot, are synthetic and are emitted as live only when simulation mode is on. The Lab is separate from serving: T1 reuses the real cache and policy code on a virtual clock, so its numbers are simulated; T2 runs a real gateway and scripted origin on one machine, so its numbers are measured, on loopback only. Do not quote dashboard numbers as production performance.

**Where taught:** Ch 2, 2.8. **Evidence to know:** `build_schema_v1_payload` (provenance map); `PHANTOM_SIMULATION`; `src/lab/mod.rs`; `src/lab_live/mod.rs`; `telemetry_honesty_tests`, `schema_deprecation_tests` (TESTED).

### ARC-8. Is this deployable in production today? What exists for deployment?

**Answer.** It is a research and engineering prototype with real hardening, not a production-ready product. What exists: a configuration system that fails closed, a production profile that refuses insecure settings, TLS and JWT options, health endpoints, release packaging scripts with checksums, a smoke test and CI definitions. What does not: container images, orchestration files, service units, graceful connection draining, a gateway-served UI build, durable invalidation across restarts and any multi-host validation. Cluster mode is labelled experimental in the binary.

**Where taught:** Ch 2, 2.9.3. **Evidence to know:** `packaging/`; `src/config.rs::KEYS`; `src/health.rs`; the `main.rs` shutdown comment; `docs/user/limitations.md`; `docs/STATUS.md`.

## 3. Technology

### TEC-1. Why did you choose Rust for a cache?

**Answer.** The gateway keeps shared mutable state that many threads touch on every request and sits on the latency path; Rust gives native speed with no garbage collector and forces explicit sharing, removing classes of memory and data-race bugs. The repository does not contain a written language decision, so this is reasoning, not a documented trade study. Rust does not prevent logic races: G4A-01 would be a logic race in safe Rust and is UNDER-INVESTIGATION.

**Where taught:** Ch 3, 3.2. **Evidence to know:** `Cargo.toml`; README section 4; `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md`; five `unsafe` blocks in `src/api/lab_api.rs`, none in `src/core`; IMPLEMENTED; language choice UNVERIFIED as a recorded decision.

### TEC-2. Why not Go or C#?

**Answer.** Both would work and have strong HTTP and concurrency support. No head-to-head was run and the repository records none, so no claim is made that they were rejected on evidence. The reasoning for Rust is control: no collector pauses and visible ownership. The cost is real: Rust is slower to write, and no latency advantage over Go or C# can be shown.

**Where taught:** Ch 3, 3.2. **Evidence to know:** absence of any Go or C# artefact; `bench-results/README.md` (no release-profile result established); reasoning, not recorded; no performance claim.

### TEC-3. Why Tokio, and what does it give you that threads would not?

**Answer.** Most of the gateway waits on sockets and timers; Tokio lets a few worker threads serve thousands of waiting requests and supplies `oneshot` channels (one origin result to many waiters), bounded channels, semaphores and timeouts. No runtime comparison was done; Axum and `tokio-rustls` are built on Tokio, so it is the natural fit. A blocked task occupies a worker thread: the sweeper scans a whole shard under its mutex (G4A-03, OPEN), so a very large shard could hold a worker longer than the documented 1 ms bound.

**Where taught:** Ch 3, 3.3. **Evidence to know:** `src/main.rs::real_main`; `cache.rs::spawn_sweeper_task`; ADR-002 CT-2; `test_sweeper_lock_hold_time_bounded` (the batch limit bounds candidates, not keys inspected); IMPLEMENTED.

### TEC-4. Why Axum rather than another framework, and why did you write your own HTTP client for the origin?

**Answer.** Axum is a thin framework on hyper and tower on the same runtime, so routing, state, body limits and CORS were a few lines each; no framework comparison is recorded. The origin client is a small HTTP/1.1 client written to connect to an address already validated against SSRF rules, send a fixed minimal header set, cap the response size and time out every step. The cost is owning a response parser, which has unit and property tests and AI review only, and `Connection: close` means each miss pays a fresh handshake, unmeasured in a release build.

**Where taught:** Ch 3, 3.4. **Evidence to know:** `create_app_with_lab`, `fetch_origin_raw`; `src/security/upstream.rs`; `parser_robustness_tests.rs`, `security_phase2_upstream_tests.rs`; IMPLEMENTED, TESTED; decision not recorded.

### TEC-5. Why partitioned mutex buckets? Why not a lock-free concurrent map, or one `RwLock`?

**Answer.** Each key has a generation, active operations, an in-flight registration and eviction bookkeeping, and rules like "never evict a key with an active fetch" need several of these updated together; a mutex around a shard makes that atomic, and eight shards cut contention without giving that up. The repository records no comparison with lock-free maps; this is reasoning. The cost is that a hot key or long critical section still serialises on one lock, and the contention benefit has not been measured on a release build.

**Where taught:** Ch 3, 3.5.2. **Evidence to know:** `PartitionedCache`, `get_shard_index`; `PartitionedCache::new(8)` in `main.rs`; `concurrency_verification_tests.rs`; G4A-03, G4A-21; TESTED for correctness, not for performance.

### TEC-6. Why partitioned mutexes? Why not one lock, a lock-free map, or a concurrent hash map crate? (Chapter 4 version)

**Answer.** One lock serialises every request; eight shards let different keys proceed in parallel, with a key always mapping to the same shard by hash and mask. Critical sections are short because the origin fetch is outside the lock. A lock-free or lock-striped map would scale further, and no benchmark here compares them; plain mutexes are used because entry, byte count, in-flight record and eviction list must change together. Eight is small and hard-coded; the decision record for the number was not found.

**Where taught:** Ch 4, 4.2. **Evidence to know:** `PartitionedCache`, `get_shard_index`; `PartitionedCache::new(8)`; IMPLEMENTED; contention not measured.

### TEC-7. What about lock contention? How do you know it is not a bottleneck?

**Answer.** It is not known: there is no contention measurement on a physical host. From the code, hit-path lock holds are short, the origin fetch is outside the lock and epoch checks use a read lock. Contention would appear on one hot key (always one shard) and in the sweeper, which scans a whole shard under its lock each tick (G4A-03, OPEN). The measurement to make is per-shard hold and wait time under a skewed workload, on a release build, on a named host, with repetitions.

**Where taught:** Ch 4, 4.2. **Evidence to know:** `docs/release/g4/cycle1_core_engine_review.md` G4A-03; `epoch_contention_tests.rs` (epoch reads, not shard contention); IMPLEMENTED, performance UNVERIFIED.

### TEC-8. What are the concurrency implications of your design? Where could it deadlock or stall?

**Answer.** Short synchronous critical sections under `std::sync::Mutex`, no lock across `.await`, slow work in a spawned task, results handed over channels; lock order is shard first, then epoch structures, and the invalidation path never takes a shard lock, so no cycle was seen in the reading. Known stall risks, all open: a long sweep holds a shard lock (G4A-03), the epoch snapshot rebuild clones up to 10,000 entries (G4A-12), and some locks panic on poison (G4A-21). There is no deadlock proof, only a lock-order argument and stress tests that did not hang; linearizability is not claimed.

**Where taught:** Ch 3, 3.5.5. **Evidence to know:** `get_effective_epochs`, `try_invalidate_tenant`; `OriginGate` doc comment; `concurrency_verification_tests.rs`, `epoch_contention_tests.rs`; G4A-01, -03, -12, -17, -21, G4B-28; IMPLEMENTED; TESTED (stress and model); not INDEPENDENTLY VERIFIED.

### TEC-9. Why rustls and not OpenSSL for the mTLS between nodes? And why do you have both `ring` and `aws-lc-rs`?

**Answer.** The recorded reason (Phase 1 specification) is to avoid OpenSSL C-library build problems on Windows; rustls also pins the cluster to TLS 1.3 with a mandatory client certificate, and a test refuses a TLS 1.2 client. Two crypto libraries are linked because `ring` is used directly for SHA-256 and rustls 0.23 defaults to `aws-lc-rs`, so the spec's "no C crypto" is not literally true; choosing one is an owner decision. The identity check (chain, one DNS SAN, fingerprint match) was tested on loopback with generated certificates and has had no independent review.

**Where taught:** Ch 3, 3.7. **Evidence to know:** `src/security/cluster.rs`; `docs/security/phase-1-specification.md` section 8; `Cargo.lock`; `test_p3_sec04_strict_tls13_rejection_of_tls12_client`; IMPLEMENTED, TESTED (loopback); not INDEPENDENTLY VERIFIED.

### TEC-10. Why React with plain CSS and no UI or chart libraries? Isn't that reinventing the wheel?

**Answer.** The recorded part: the toolkit proposal rejected chart, graph and component libraries on bundle size, dependency count and canvas-chart accessibility, with a guard test pinning the dependency list. The other part is reasoning: every number on screen carries a measured, simulated, inferred or unavailable label, and hand-built components keep that label inside each chart. The cost is more code and a hand-built SSE client. The UI is tested, not approved; there is no human usability or screen-reader test, and no recorded comparison explains React over something lighter.

**Where taught:** Ch 3, 3.11. **Evidence to know:** `ui/package.json`; `ui/test/forbidden.test.mjs`; `docs/design/remediation/07_frontend_redesign_implementation_record.md`; `docs/research/design_toolkit/toolkit_proposal.md`; `docs/STATUS.md` M8 row; IMPLEMENTED, TESTED; visual quality unapproved.

### TEC-11. Are your dependencies safe? Have you scanned them?

**Answer.** The dependency set is small and permissively licensed, the lockfile has no git or alternate-registry sources and release builds use `--locked`. But neither `cargo audit` nor `cargo deny` has been run, so freedom from known advisories cannot be claimed. Some declared crates are unused and two crypto libraries are linked; consolidating is an owner decision.

**Where taught:** Ch 3, 3.13. **Evidence to know:** `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md`; `docs/STATUS.md` (M7 row); inventory VERIFIED by reading, advisory status UNVERIFIED.

## 4. Algorithms

### ALG-1. What stops a cache stampede? What happens to the origin when a hot key expires?

**Answer.** Single-flight per key: under the shard lock the first miss registers an in-flight operation, later misses wait on a channel, and the one origin result goes to everyone. A test (`test_negative_effect_head_of_line_blocking`, 10 clients; a separate 100-client HTTP test, `test_m1_t13_barrier_stampede_100_clients`, also exists) fires 10 simultaneous requests at a slow origin and gets 1 miss and 9 coalesced answers; waiters are capped at 1,000 per key and the origin read times out after 5 seconds. Limits: if the origin fails all waiters fail together, there is no negative caching, and across many different keys there is no protection unless the optional origin cap is on (off by default).

**Where taught:** Ch 4, 4.4. **Evidence to know:** `src/core/coalesce.rs`; `begin_fetch_atomic`; `coalesce_tradeoffs_tests.rs`; `origin_cap_tests.rs`; G4A-08 (OPEN); IMPLEMENTED and TESTED on loopback.

### ALG-2. What happens with a hot-key overload? Say one key gets most of the traffic.

**Answer.** Two problems. A hot key that misses is coalesced into one origin request. A hot key that hits still takes the same shard mutex twice in the GET path and the HTTP layer copies the body per response; the key lives in one shard and is not replicated, because GET is local only. The shard mutex and the body copy are expected to be the limit, but this is unmeasured. The intelligence modules have detection research, which is not a serving-path fix for hit-path hot keys.

**Where taught:** Ch 4, 4.4. **Evidence to know:** `src/api/server.rs` (HIT branch clones the body); G4A-26; `check_stale_serving`, `begin_fetch_atomic`; IMPLEMENTED, hot-hit behaviour UNVERIFIED.

### ALG-3. How do you stop a stale response overwriting a newer one, for instance after a DELETE?

**Answer.** Each key has a generation; a fetch records it at start, DELETE bumps it, and the fetch stores its body only if generation, state identity and tenant and global epochs still match, all under the shard mutex. Otherwise the body is dropped and `stale_writes_prevented` rises; waiters still get their response. This is not called linearizable: a completed DELETE makes later reads on that node miss (a single-threaded order is tested), but there is no schedule-exploring proof, and for tenant-wide invalidation two findings are open (G4A-01, G4A-08).

**Where taught:** Ch 4, 4.5. **Evidence to know:** `finalize_fetch`, `delete_impl`; `invalidation_contract_tests.rs`; `concurrency_verification_tests.rs` (exhaustive model, 59,049 sequences, with no invariant about fills after tenant invalidation, so it cannot find G4A-01); IMPLEMENTED, TESTED for DELETE.

### ALG-4. Is there a race that could resurrect a deleted value?

**Answer.** One suspected race, the first High finding in the project's own review, treated as under investigation, not fixed. A fetch checks the epochs when it finishes but reads them a second time after the check, so an invalidation landing in that gap can let the old body be stored as current. It was reproduced only with an artificial pause in a sandbox copy; a short unhooked stress run found nothing. A proposed fix passes sandbox tests but is not in the product code and has not been independently reviewed.

**Where taught:** Ch 6, 6.9.4. **Evidence to know:** `finalize_fetch`, `get_effective_epochs`; `docs/research/g4a01_race/status.md`, `proposal.patch`; `test_p4a_inflight_fetch_fencing_prevents_stale_writes` (passes; does not exercise the gap); G4A-01 UNDER-INVESTIGATION; IMPLEMENTED with a known hole; fix PROTOTYPE ONLY.

### ALG-5. You say fencing prevents stale writes. Is there a race? Has it been reproduced? Is it fixed?

**Answer.** Yes, one known race. In the last step of a fetch the cache checks the tenant epochs, does more work, reads the epochs again and stamps the entry with the second reading; tenant invalidation does not take the shard lock, so a bump in that gap makes an old body look current. It was reproduced only in a sandbox with a test hook; a twenty-second stress run without the hook found nothing, which does not prove it cannot happen. A small fix passes in the sandbox but is not integrated because fencing changes need independent review and a failing-first test; the register keeps it open and blocking. The DELETE fence is tested; the tenant-epoch fence has this gap.

**Where taught:** Ch 4, 4.7. **Evidence to know:** `docs/research/g4a01_race/` (analysis, status, repro_attempts, proposal.patch); G4A-01 in the disposition register; defect CONFIRMED in sandbox, fix NOT integrated.

### ALG-6. Explain epochs. Why not just delete the keys? What does invalidation guarantee?

**Answer.** Deleting every key of a tenant means walking the table under locks. Instead each tenant has a version number stamped on every entry; invalidation increases it, and older entries are treated as missing when touched, with a background sweeper reclaiming the rest. A fetch that started before the bump also cannot store its answer. The guarantee is local to one node and lazy, with two documented exceptions and one open race; past 10,000 tenants the node raises a global epoch instead, which is safe but heavy and can be triggered by a peer (G4A-12, open). It is not atomic across nodes.

**Where taught:** Ch 4, 4.7. **Evidence to know:** `try_invalidate_tenant`, `get_effective_epochs`; `epoch_contention_tests.rs`, `invalidation_contract_tests.rs`; G4A-08, G4A-12, G4A-17; IMPLEMENTED, TESTED for basic and overflow paths.

### ALG-7. How do you know the cache does not leak memory? Is the byte count the real memory use?

**Answer.** The count is logical (key size, body capacity, header strings, fixed overhead), and the code can recompute it from the maps and compare. Tests drive every path that changes an entry and assert the totals match, including a concurrent stress run; that shows the bookkeeping is consistent, not that the operating system sees the same number, because freshness strings, hash-table slack and S3-FIFO queues are not counted. One known path drifts: a panic mid-finalize (G4A-13, open). No measured resident-memory figure for a release build exists.

**Where taught:** Ch 4, 4.6. **Evidence to know:** `audit_bytes`, `calculate_state_bytes`; `byte_accounting_tests.rs`; `memory_bound_soak_tests.rs`; G4A-13, G4A-23; IMPLEMENTED, TESTED (consistency only).

### ALG-8. How does expiry work? What stops the cache serving very old data? Is the default TTL configurable?

**Answer.** The origin's `Cache-Control` decides lifetime (shared-cache max-age, then max-age, then `Expires`); with no information the lifetime is 60 seconds. `PHANTOM_DEFAULT_TTL_MS` is not wired, so changing it does nothing. An expired entry is served only if the origin allowed stale-while-revalidate or stale-if-error, never with must-revalidate, and never beyond the 24-hour cap (`PHANTOM_MAX_STALE_MS`, wired); the cap limits stale serving, not fresh lifetime. Known gaps: upstream `Age` ignored, wall-clock time, stale-if-error applied to more status codes than the RFC allows. It is "RFC 9111-style", not compliant; no conformance suite was run.

**Where taught:** Ch 4, 4.8. **Evidence to know:** `src/config.rs`; `src/api/server.rs` literal `60_000`; `check_stale_serving`; `freshness.rs::evaluate_response_freshness`; G4A-04, -05, -06, -10, -18, -25; IMPLEMENTED (subset), TESTED for cap and SIE paths.

### ALG-9. Why SIEVE as the default? Is eviction really O(1)? You also have S3-FIFO; is it safe?

**Answer.** SIEVE keeps a queue and a visited bit per entry, so a hit only sets a bit and does not rewrite a list under the shard lock; eviction advances a hand that clears bits. No recorded decision makes it the default, and no test measures its hit ratio against LRU on a real workload. It is expected O(1) for hit, insert and remove and amortized O(1) for eviction when nothing is pinned, not worst-case O(1). S3-FIFO has an open High finding: a removed key stays in the queue, so queues can grow and early evictions can happen; it is used only if selected.

**Where taught:** Ch 4, 4.9. **Evidence to know:** `src/core/eviction.rs` (`FaithfulSieve`, `FaithfulLru`, `FaithfulS3Fifo`); `eviction_contract_tests.rs`; G4A-02, G4A-20 (OPEN); `docs/research/eviction_policy_audit.md`; SIEVE and LRU IMPLEMENTED, TESTED (weakly); S3-FIFO IMPLEMENTED, PARTIAL (G4A-02 OPEN); literature UNVERIFIED. "Faithful" is the code's name, not a verified property.

### ALG-10. State the time and space complexity of the cache operations.

**Answer.** With n entries in a shard and key length k (at most 4,096 bytes): lookup is O(k) hashing then expected O(1); insert and delete are expected O(1) plus O(1) policy bookkeeping; eviction is amortized O(1) with no pinned keys; coalescing a request is O(1); memory is bounded by the byte budget and entry limit. Three exceptions: tenant invalidation is O(T) in tenants with a recorded epoch (up to 10,000), the sweeper scans O(n) of a shard under its lock each tick, and switching policy is O(n) per shard. A hit also copies the body.

**Where taught:** Ch 4, 4.9 (box) and 4.1. **Evidence to know:** `cache.rs` (`get_shard_index`, `try_invalidate_tenant`, `sweep_lazy_reclamation`, `set_eviction_policy`); G4A-03, -12, -20, -26; asymptotics from reading code, none measured here.

### ALG-11. You say you use weighted rendezvous hashing. How does the weight enter, and what does the system do with it?

**Answer.** Each node scores each key from a hash of key and node id and the highest score owns the key; everyone computes the same scores, and removing a node moves only its keys. Weighted form uses minus weight over the natural log of the normalised hash. In the code the weight is a constant 1.0, so it is plain rendezvous hashing; no capacity-aware placement is claimed. The result only picks the node that acknowledges a DELETE first; reads are not routed. The code multiplies the raw hash by the weight, which is not proportional (a quick standalone calculation gave 75% for weights 1 and 2, not 67%).

**Where taught:** Ch 5, 5.3. **Evidence to know:** `src/dist/router.rs::route_key` (`capacity_weight = 1.0`), `get_replicas`; only `invalidate_key_across_cluster` calls `route_key` in production; equal-weight rendezvous IMPLEMENTED; weighted DESIGNED BUT NOT IMPLEMENTED; no uniformity test in the repo.

### ALG-12. How accurate are your latency percentiles, and how do you compute them without slowing the request path?

**Answer.** The log-linear histogram has 2,688 counters, constant-time recording and relative error at most 0.39 percent by construction, checked by tests. But the percentiles on the metrics endpoint are computed from a 10,000-sample window sorted on demand, exact for that window, with the sort and two locks on the observation path. So the 0.39 percent is the recording design, not the accuracy of displayed numbers. Percentile conventions across the repository are also inconsistent (floor versus nearest-rank).

**Where taught:** Ch 5, 5.11. **Evidence to know:** `src/intel/telemetry.rs` (`value_to_bucket`, `calculate_percentiles_us`, `record_us`); `telemetry_redesign_tests.rs`; `src/api/server.rs`; histogram IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader and is not the live percentile source.

### ALG-13. How do you detect and handle hot keys?

**Answer.** Handling is live: concurrent misses for one key are coalesced into one origin request, and a cached hot key is served locally by every gateway. Detection is a library detector (Count-Min Sketch plus a 64-key Space-Saving table and an emergence rule), tested in isolation but not wired into the gateway; the metrics endpoint reports hot keys as unavailable. So claim hot-key protection through coalescing and hot-key detection as a built-but-unused component. The sketch overestimates only, with error bounded by the standard Count-Min result; there is no measured error on production traffic.

**Where taught:** Ch 5, 5.12. **Evidence to know:** `src/intelligence/signals.rs::HeavyHitterDetector`, `HotKeyEmergenceDetector`; `src/core/coalesce.rs`; `server.rs` `"hot_keys"` unavailable; `src/intel/sketch.rs` unused; coalescing IMPLEMENTED and TESTED; detection PROTOTYPE ONLY.

## 5. Distributed systems

### DIS-1. How do the nodes communicate?

**Answer.** A custom binary protocol on TCP inside mutual TLS: each node proves identity with a certificate whose name and fingerprint are checked against a static membership file. The main message is an invalidation, sent to the member chosen by rendezvous hashing, which has up to 150 milliseconds to acknowledge, then to the others in the background with a bounded queue, retries and a repair worker. There are also heartbeats and a reconciliation exchange. Reads do not use the network between nodes. A down node is marked suspect and owed the invalidation in an in-memory ledger, lost if the issuer restarts first.

**Where taught:** Ch 2, 2.7.4. **Evidence to know:** `src/dist/frame.rs`; `router.rs::invalidate_key_across_cluster`; `node.rs::start_tcp_service`; `docs/user/limitations.md`; IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED (no multi-host evidence). The choice over Raft or gossip has no recorded decision.

### DIS-2. How do your nodes discover each other? What if I add a node?

**Answer.** Membership is static: a JSON file lists every node, its id and certificate fingerprint, read at startup. Nodes talk over mutually authenticated TLS 1.3 with a small binary protocol; there is no gossip and no dynamic join. To add a node you edit the file, bump its version and restart the members. A runtime reload and revocation path exists in the library and is tested, but nothing in the shipped gateway triggers it (G4B-07, open); dynamic membership is not claimed.

**Where taught:** Ch 5, 5.2. **Evidence to know:** `ClusterMembershipRegistry::update` (no production caller); `src/main.rs` cluster branch; G4B-07; static membership IMPLEMENTED and TESTED; dynamic membership DESIGNED BUT NOT IMPLEMENTED. Avoiding gossip or consensus is our reasoning, not a recorded decision.

### DIS-3. Which node serves a read for a key, and does rendezvous hashing decide that?

**Answer.** The gateway that received the request serves it from its own cache. Rendezvous hashing today only picks the node that must acknowledge a DELETE first. The code to forward a read to the key's owner exists as a function but nothing calls it, and the peer-side read handler is a stub that fabricates a value, so distributed reads are not claimed. The hash gives a deterministic primary for invalidation; the weight is a constant 1.0.

**Where taught:** Ch 2, 2.7.4. **Evidence to know:** `RendezvousRouter::route_key`, `forward_key_over_tcp` (no caller), the `GetRequest` arm of `process_binary_cache_stream`; ADR-003 (agent-recorded, not owner-ratified per `docs/STATUS.md`).

### DIS-4. Walk me through a read. Which node serves it, and where does the hash come in?

**Answer.** The contacted gateway looks in its own cache; on a miss it fetches from the origin with single-flight and stores the result with whatever freshness the origin allowed. No other gateway is consulted, so the hash is not involved; it matters only for deletes. The same key can be cached on every node: local copies with invalidation fan-out. The cost is that a node that missed an invalidation can serve a stale copy.

**Where taught:** Ch 5, 5.3. **Evidence to know:** `get_cache_handler`; `forward_key_over_tcp` (no production caller); ADR-003; local-only GET IMPLEMENTED and TESTED (`known_gaps.rs`, `distributed_consistency_tests.rs`).

### DIS-5. You have heartbeats. So does the cluster detect a dead node?

**Answer.** Not by heartbeat. The heartbeat keeps idle connections alive every five seconds and discards dead ones, but changes no node's state. A node becomes Suspect when a real call to it fails, usually the first invalidation after it died, which spends its 150 ms deadline; after that it is skipped for routing and its owed invalidations go to a repair ledger. Detection is lazy and traffic-driven; wiring heartbeat results into the state machine is the obvious fix (G4B-12). The UI can show a node as Failed only after the demo control sets a flag, which is inferred, not detected.

**Where taught:** Ch 5, 5.2. **Evidence to know:** `src/dist/pool.rs::probe_idle_connections` (no `NodeState` access); `server.rs::control_handler`; G4B-12, G4B-03; failure detection by heartbeat is not implemented.

### DIS-6. What is an epoch here, and does it give you a global version of the cache?

**Answer.** There is no global version. Within one gateway, tenant and global epochs are counters stamped on entries; bumping one makes older entries stale without scanning. Between gateways the counters are not synchronised. What crosses the wire for a normal delete is the exact key, a per-destination sequence number to notice lost messages, and the membership version, which is carried but not enforced on invalidations (open finding G4B-06). A shared tenant counter would need agreement, which the system lacks.

**Where taught:** Ch 5, 5.4. **Evidence to know:** `try_invalidate_tenant`, `try_invalidate_all`, `get_effective_epochs`; the `InvalidateRequest` handler in `src/dist/node.rs`; G4B-06, G4A-01; local epochs IMPLEMENTED and TESTED; cross-node epoch enforcement not implemented.

### DIS-7. A DELETE returns 204. What exactly has been invalidated at that point?

**Answer.** Two things for sure: the gateway that received the DELETE dropped its own copy, and the chosen primary confirmed it dropped its copy, within a 150 millisecond deadline. Every other node is told afterwards, in the background, with retries and a repair ledger, guaranteed only while the issuer stays alive. Sometimes the primary is the issuer itself, so 204 then says nothing about any remote node. A delayed 204 until all members acknowledge was considered in the M3 design and rejected there as out of scope; it is an open product decision.

**Where taught:** Ch 5, 5.5. **Evidence to know:** `delete_cache_handler`; `invalidate_key_across_cluster` (doc comment states the contract); ADR-003 amendment 2026-10-30; G4B-09; IMPLEMENTED and TESTED as described; cluster-wide acknowledgement DESIGNED BUT NOT IMPLEMENTED.

### DIS-8. A client sends DELETE and gets 204. Is that content guaranteed gone from every node?

**Answer.** No. 204 means the issuing gateway purged its own cache and the canonical primary acknowledged within 150 milliseconds, and that primary can be the issuer itself. Other members are informed asynchronously with retries, a repair worker and anti-entropy, but the list of invalidations still owed lives only in the issuer's memory. If the issuer is killed first, peers keep serving the old object until its freshness lifetime ends; this was reproduced on unmodified code, and the durable-log fix is a sandbox patch, opt-in, not in the tree. For deletion-critical content the documentation advises a short `s-maxage`, re-issuing DELETEs after any restart, and verifying through each gateway.

**Where taught:** Ch 8, 8.13.2 (see also Ch 5, 5.5 and 5.7). **Evidence to know:** G4B-09; `docs/user/limitations.md` "Cluster invalidation durability"; `m3_durability/reproduction_report.md`; `claims_ledger.md` C6 amendment; durability PROTOTYPE ONLY. No staleness bound is offered beyond the origin's lifetime (60 s when the origin gives none); a "zero stale after 50 ms" claim was withdrawn.

### DIS-9. What if the invalidation message is lost or arrives late?

**Answer.** Three layers: the sender retries up to four times with short backoff; anything undelivered goes to a per-peer repair ledger and is re-sent every 250 milliseconds with the same sequence number until acknowledged; and a receiver that sees a gap can pull the missed records from the sender's ring buffer, flushing its whole cache if the history was overwritten (safe but coarse). All of it is bounded in memory. A late message is harmless because a delete is idempotent. The one case none of this covers is the sender dying while it owes messages; with a permanent cut or crashed sender the only bound is the entry's own expiry.

**Where taught:** Ch 5, 5.5. **Evidence to know:** `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound` (loopback, debug; a two-second cut, convergence asserted within 800 ms of healing); `RepairLedger`, `repair_peer`, `reconcile_peer_anti_entropy`; IMPLEMENTED and TESTED; sender crash UNHANDLED.

### DIS-10. What happens if an invalidation is lost or delayed? Can two nodes serve different values?

**Answer.** Yes, briefly. After a DELETE the issuing gateway misses immediately and the primary has acknowledged before 204; others are told in the background with retries, a repair record and a gap-detecting pull. On a warm loopback cluster propagation was mostly milliseconds, with a first-use cold start of tens of milliseconds, in debug builds, and during that window another node can serve the old value. If a peer is down it can stay stale until repaired; if the issuer crashes first the owed record is lost and the peer stays stale until expiry. The claim is eventual repair while the issuer stays alive, not bounded staleness.

**Where taught:** Ch 6, 6.9.3 to 6.9.4. **Evidence to know:** `invalidate_key_across_cluster`, `RepairLedger`; `distributed_consistency_tests`, `cluster_followup_tests`, `cluster_propagation_latency_tests`; claims ledger C6 amendment and qualifier; G4B-09 (OPEN); M3 design PROTOTYPE ONLY; status PARTIAL. The test that would falsify the issuer-alive claim (blackhole a peer, DELETE, kill the issuer) does not exist.

### DIS-11. Can two of your nodes serve different values for the same key at the same time?

**Answer.** Yes, by design each gateway has its own copy. They differ for milliseconds after a delete while the invalidation is in flight, for as long as a partition lasts if a gateway is cut off but serving, and until expiry if the issuing gateway crashed before telling everyone. They can also differ with no delete, because two gateways fetched at different times and the origin changed in between. The claim is a mechanism that converges after a delete when the system is healthy, with what is not covered stated plainly.

**Where taught:** Ch 5, 5.6. **Evidence to know:** ADR-003 section 4 and its 2026-10-30 amendment; `claims_ledger.md` C6 amendment and qualifier; `distributed_consistency_tests.rs`; convergence IMPLEMENTED and TESTED on loopback; staleness bound UNVERIFIED and not claimed.

### DIS-12. Is your cache linearizable? Do you guarantee bounded staleness?

**Answer.** No to both. Linearizability across nodes would order every read against every delete, and reads are local. There is a per-gateway guarantee that a read after a completed delete misses, with two open findings about rare races. Bounded staleness needs a number, a failure model and a test: in the healthy, issuer-alive case the observation is milliseconds on loopback in a debug build, which is not a guarantee; in the crash or permanent-partition case the bound is the entry's freshness lifetime, set by the origin. The ADR's earlier "24 hours" statement was amended on 2026-10-30 because the cap bounds the stale-serving window, not the freshness lifetime.

**Where taught:** Ch 5, 5.6. **Evidence to know:** ADR-003 amendment; `claims_ledger.md` C6 qualifier; `src/core/freshness.rs` (no gateway ceiling); neither claim is made.

### DIS-13. Is this linearizable? What is your consistency model?

**Answer.** Not linearizable, and no such claim is made. Each node orders its own fills and deletes with generation fencing; across nodes the model is eventual convergence of invalidations under the tested loopback fault model: N independent caches and an invalidation bus. Numbered consistency clauses with a failure model, a bound and a falsifying test have been drafted, but they await owner ratification and several are known to be unmet. Even single-node fencing is not proven airtight: the double epoch read (G4A-01) was reproduced only with an injected yield, and a three-line fix is proposed and not applied because the file is protected.

**Where taught:** Ch 8, 8.13.2. **Evidence to know:** G4A-01; `docs/research/g4a01_race/`; `src/core/cache.rs::finalize_fetch`; product-direction section 7.

### DIS-14. What happens if a node fails? What if the network partitions?

**Answer.** If a node dies, calls to it fail fast, it is marked suspect and skipped, and the invalidations it missed are queued in a bounded ledger on the sender; when it returns its cache is already empty (memory only), the ledger is delivered and it rejoins after a pull reconciliation. If the network splits with both sides alive there is no leader, so the cut-off side can serve stale copies until the link heals or the entries expire. Reads chose availability; staleness during a partition is bounded only by the entry's freshness lifetime. A suspect node does not stop serving on the HTTP path; the rule exists only in the peer handler.

**Where taught:** Ch 5, 5.7. **Evidence to know:** `test_m3_04a_suspect_to_active_automatic_recovery`, `test_m3_04a_missed_fanout_repaired_by_antientropy_within_bound`; no `NodeState` in the GET path of `server.rs`; `test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits` (peer path only); receiver recovery IMPLEMENTED and TESTED (loopback); no staleness bound.

### DIS-15. What if the gateway that issued the delete crashes before the others got the message?

**Answer.** Those nodes keep serving the old entry until it expires. The owed messages are only in the issuer's memory and a restarted issuer starts empty. This was reproduced in a sandbox: three gateways, the issuer killed right after answering 204, and the third gateway did not converge in ten seconds. It is open in the main tree. A fix exists as a sandbox design (an opt-in durable log replayed after restart, with 15 process-level crash tests) but is not integrated; until then the right claim is "guaranteed while the issuer stays alive". The damage is bounded by the entry's freshness lifetime, and it affects only entries whose invalidation was in flight.

**Where taught:** Ch 5, 5.7 and 5.9. **Evidence to know:** `docs/research/m3_durability/` (`reproduction_report.md`, `design.md`, `remediation/00_status_and_plan.md`); `claims_ledger.md` C6 qualifier; G4B-09; defect REPRODUCED in a sandbox; fix PROTOTYPE ONLY.

## 6. Security

### SEC-1. How do you stop an unauthorised user reading or deleting cached data?

**Answer.** With a JWT secret configured, every cache read, delete, metrics, stream and control call needs a signed token; the gateway checks signature, issuer, audience and expiry, then that the role allows the action (`cache:read`, `cache:invalidate`), and takes the tenant from the token, never a header, before any lookup or fetch. Without a secret the gateway starts only on loopback and refuses a network address without a secret and TLS. Not claimed: tokens cannot be revoked before expiry, there is no not-before check, and the issuer is outside this repository. In the GET and DELETE handlers the body is extracted before authentication (G4B-01, High, owner-named, OPEN).

**Where taught:** Ch 6, 6.2. **Evidence to know:** `jwt.rs::JwtValidator::validate`; `server.rs::authenticate_request`, `require_tenant_claim`; `m7_sec_fixes_tests::sec01_*`, `m7_security_review_tests`, `m7_http_rbac_matrix_in_secure_mode`; G4B-01, -13, -14; IMPLEMENTED, TESTED for covered paths; revocation, `nbf` and lifetime caps absent.

### SEC-2. Your control endpoint changes cluster node state. Who can call it, and what stops a demo control from breaking production?

**Answer.** In secure mode it needs the `scenario:operate` role; without a JWT secret it is anonymous, possible only on loopback. It is a demo control and is not isolated from the real router: in a cluster process it operates on the same node objects the router uses, and an unknown event sets every node Active (G4B-03, owner-named, open). Marking a node Failed changes the DELETE primary and makes the node drop incoming connections, without corrupting cache data. It is not safe to expose in production.

**Where taught:** Ch 6, 6.3. **Evidence to know:** `server.rs::control_handler`; `main.rs` (nodes shared with the router); the accept loop in `node.rs`; G4B-03 (OPEN, ODR-4); IMPLEMENTED, not isolated; no cluster test.

### SEC-3. How do your nodes trust each other, and what if one node is compromised?

**Answer.** Mutual TLS 1.3 with a private cluster CA: a certificate with exactly one DNS name encoding node id and cluster id, a node id in the membership file and marked active, and a fingerprint matching the pinned one, re-checked on every frame; an invalidation frame's claimed sender must equal the certificate identity. A compromised member is trusted: it can send tenant-wide or full-cache invalidations (past 10,000 tenant names costing every tenant its cached data). Revocation exists in code and tests but is not triggered in a running process; the remedy is to remove the member from the file and restart. The peer `GetRequest` stub does not expose HTTP entries (keys starting with `/` are refused); that conclusion is an AI investigation, not independent review.

**Where taught:** Ch 6, 6.4. **Evidence to know:** `security/cluster.rs::ClusterTlsConfig::from_pem`, `validate_peer`; `node.rs::start_tcp_service`; `security_phase3_cluster_tests`, `cluster_followup_tests::test_l3_invalidate_request_originator_must_match_tls_identity`; G4B-05/06/07/08, DR-027; admission IMPLEMENTED and TESTED; scope authorisation DESIGNED BUT NOT IMPLEMENTED.

### SEC-4. Your node protocol has a GET request. Can a peer poison your cache with it?

**Answer.** The peer GET is a placeholder: on a miss it invents a value named `val_node_` plus the node id and stores it. No production code sends the request, it needs a valid pinned cluster certificate, and the planted key lives in a key space HTTP requests cannot reach because peer keys must not start with a slash. So it does not poison what clients read today, but it can fill the cache with junk and evict real entries and is a hazard for any future peer-fill. It is tracked as DR-029 with a test that pins the behaviour; the fix awaits an owner decision.

**Where taught:** Ch 5, 5.8. **Evidence to know:** `src/dist/node.rs` GetRequest arm; `known_gaps.rs::pin_kg6_node_get_request_fabricates_missing_key_body`; `m7_sec_round3_tests.rs`; DR-029, G4B-30; PROTOTYPE ONLY; design defect OPEN.

### SEC-5. What happens if someone sends a malicious request: a huge body, a slow body, a crafted path?

**Answer.** Crafted paths and keys are handled well: strict alphabet and length limits, control characters and dot segments rejected with 400, and the path re-encoded before going upstream, with tests. A huge body is capped at 512 KiB and gets 413. A slow body is the weakness: no read timeout or connection cap on the main API, and the body is read before authentication (G4B-01, the High-severity security finding, open and owner-named). The fix is known and already done in the Lab API but not applied to the main API, so it is not hardened against slow-client exhaustion.

**Where taught:** Ch 6, 6.5. **Evidence to know:** `create_app_with_lab` (`DefaultBodyLimit`); `get_cache_handler` signature; `lab_api.rs::runs_start`; `test_m1_t26_ingress_request_body_limit`, `h1_illegal_bytes_in_path_are_400_and_origin_receives_nothing`; G4B-01; key validation TESTED; slow-body protection absent. Changing `server.rs` needs the owner's decision.

### SEC-6. How is tenant isolation enforced, and could one tenant ever see another's data?

**Answer.** The tenant is part of every cache key and, in secure mode, comes only from the signed token; tenant names are restricted so one cannot spoof another's prefix, and an invalidation bumps only that tenant's epoch. Tests cover tenant-claim handling, spoofed headers and cross-tenant reads. Limits: isolation is logical, in one process and one shared byte budget, with no per-tenant quotas, so a heavy tenant can evict others; a cluster member can address any tenant in an invalidation; and `__default__` is refused in secure mode but the key validator still accepts it (G4A-19 not closed). Only path, query and tenant influence the origin response and all are in the key; origins must not vary responses on other client data.

**Where taught:** Ch 6, 6.6. **Evidence to know:** `CacheKey::new`, `validate`; `server.rs::require_tenant_claim`; G4A-19, G4B-05, G4B-11, G4B-30 / DR-029; TESTED on HTTP paths; quotas DESIGNED BUT NOT IMPLEMENTED.

### SEC-7. How do you prevent resource exhaustion and protect the origin from overload?

**Answer.** Memory is bounded by a byte budget with exact accounting and fixed caps on waiters, the tenant epoch map, frames, repair queues, origin response size and timeouts; many requests for one key collapse to one fetch; an optional origin concurrency cap sheds load with stale serving or 503, and tests show permits are always released. Missing: the cap is off by default and per process, cache-busting keys or the bypass header reach the origin without limit, and there is no rate limit or connection cap on the HTTP side (G4B-01, -02, -11). DoS resistance is not claimed. In the loopback experiment a static cap reduced peak origin concurrency but lowered client success, reported as a trade-off.

**Where taught:** Ch 6, 6.8. **Evidence to know:** `intelligence/executor.rs` origin gate; `origin_cap_tests.rs`; `lab-results/scenario_b/*/summary.json`; `scenario_b_interpretation_notice.md`; G4B-11 (OPEN); bounded memory IMPLEMENTED and TESTED; rate limiting DEFERRED (SEC-M7-11 queued).

### SEC-8. Could a user make your gateway fetch an internal address (SSRF)?

**Answer.** A user cannot choose the host: the origin is one configured URL and users supply a validated, re-encoded path and query. If the origin name resolved to a private or metadata address, the gateway resolves it, checks every returned address against a block list and connects to the exact address it checked, so DNS changes in between cannot help. Dangerous ports are blocked, redirects are not followed and client credentials are not forwarded by default. Known gap: a few rarely used reserved ranges are missing from the block list (G4B-15), and rebinding defence was reviewed by reading, not an automated rebinding test.

**Where taught:** Ch 6, 6.8. **Evidence to know:** `security/upstream.rs::resolve_and_pin_origin`, `PreConnectIpFilter`, `UpstreamPolicy::validate_target`; `m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses`, `test_p2_ssrf_cloud_metadata_blocked`, `test_p2_ssrf_mixed_dns_blocked`; G4B-15; IMPLEMENTED, TESTED; range gaps OPEN. `PHANTOM_ALLOW_LAN_ORIGIN` is flagged high-risk.

### SEC-9. Is the system memory-safe, and how do you manage third-party risk?

**Answer.** Our cache, security and cluster code contains no `unsafe`; the only unsafe in the product is a small commented Windows-only block in the Lab for killing child processes. Third-party crates have their own unsafe code, unaudited. Versions are pinned in `Cargo.lock` (288 packages) and TLS is in rustls on the product path, but no `cargo audit` has been run, some features are enabled and unused, and there are two crypto providers; the scan is a release gate needing a human decision. Rust stops memory corruption, not logic failure: lock poisoning and no catch-panic layer are findings G4A-21 and G4B-28.

**Where taught:** Ch 6, 6.10. **Evidence to know:** `grep unsafe` result (`lab_api.rs` Windows module; bench alloc counter); `Cargo.toml`, `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (G4C-DEP); IMPLEMENTED; scan UNVERIFIED.

### SEC-10. Can one tenant, or one cluster node, hurt another?

**Answer.** Yes, in two ways. Tenants are separated by key and invalidation epoch, but there are no quotas, so one tenant can consume the byte budget. And any authenticated cluster member can send tenant-scope or full-cache invalidation frames without the receiver checking scope against the sender's rights (DR-027 or G4B-05, owner-named, must not be dismissed); the plan is per-member scope authorisation and tenant-flood caps, in a sandbox, queued behind durability. It needs a valid cluster certificate, so the attacker is an insider or compromised member. The `GetRequest` stub writes a placeholder value, but key spaces are disjoint: low as exploited today, medium as a latent hazard.

**Where taught:** Ch 8, 8.13.2. **Evidence to know:** G4B-05, G4B-30; `docs/security/investigations/DR-029_*`; `src/dist/node.rs` (`val_node_` format); status PARTIAL for security.

### SEC-11. Where would a hostile reviewer attack first?

**Answer.** At the HTTP ingress and the control endpoint: bodies are extracted before authentication with no body timeout, connection limit or rate limit, so a slow-sender attack is plausible (High, owner-named), and `/api/control` parses its JSON before checking the token. Second, the cluster: static membership, long-lived connections that never re-check certificates, and members who can flush every cache. Third, the supply chain: no advisory scan on 288 locked packages. All are documented; the stated mitigation is a deployment assumption (a proxy with limits), not a proven property. These findings are confirmed by reading code, not by running attacks.

**Where taught:** Ch 8, 8.13.4. **Evidence to know:** G4B-01, -02, -03, -05 to -08, -11; G4C-DEP; `server.rs::control_handler`; status PARTIAL.

## 7. Innovation

### INN-1. What is novel about PhantomCache?

**Answer.** No single mechanism is claimed as new: coalescing, stale-while-revalidate, leases against stale writes, adaptive concurrency limits and anti-entropy all exist in Varnish, nginx, Memcached, Envoy and Cassandra, and the project's own prior-art record is open and says novelty may not be claimed. What can be defended is the combination and the method: explicit invariants (fencing, epochs, stale caps, origin gate, byte accounting) with tests, and a deterministic simulator that drives the real cache code with a preregistered mechanical verdict and an honest negative result. Whether the combination is better is unknown; no head-to-head measurement exists.

**Where taught:** Ch 7, 7.8 (box in 7.9). **Evidence to know:** `docs/research/rt0_prior_art_sweep_2026-10-30.md` (status PARTIAL, RT-0 OPEN); `novelty_snapshot_m6.md` (correction notice); `claims_ledger.md` sections 6-7.

### INN-2. Why not just use Redis or Memcached?

**Answer.** For a general key-value cache you should; they are mature and feature-rich. PhantomCache is a different shape: an HTTP gateway in front of an origin that applies HTTP freshness rules, serves stale under a hard cap, coalesces misses and caps origin concurrency, which with Redis you would write in the application. Memcached leases and Redis client tracking address parts of the stale-write and invalidation problem; generation fencing is the same family of idea. The trade-off is less maturity, no persistence and no clustering depth, and no benchmark shows it is faster.

**Where taught:** Ch 7, 7.8.2 (box in 7.9). **Evidence to know:** `rt0_prior_art_sweep_2026-10-30.md` T3 (Facebook leases, OPENED-DIRECT) and T10 (Redis tracking); `src/core/cache.rs` (in-process cache); Chapters 2 and 4.

### INN-3. What is the actual intelligence in PhantomCache? Is it AI?

**Answer.** It is not machine learning and not called AI. The word covers telemetry (counters and a latency histogram), a library of statistical signals and a decision engine for shadow evaluation written for experiments, and one control rule on the live path: an origin-concurrency cap, off unless canary or active mode is on and a cap configured, fixed or AIMD. When tested in a pre-registered sealed simulation, the adaptive version did not beat the best fixed cap (hypothesis not supported). The working part today is a static origin cap; the adaptive research is a prepared lab, not a proven feature.

**Where taught:** Ch 1, 1.7. **Evidence to know:** `intelligence/mod.rs::IntelligenceMode`; `executor.rs::origin_cap_enforcing`; `main.rs` (`pie: None`); `m6_sealed_results.md`; `scenario_b_interpretation_notice.md`; `CLAUDE.md` section 6b; origin gate IMPLEMENTED and TESTED; decision engine PROTOTYPE ONLY; adaptive benefit NOT ESTABLISHED (H1 NOT_SUPPORTED, simulated).

### INN-4. What is the actual intelligence in the system? (Chapter 7 version)

**Answer.** Adaptive control, not machine learning. The part touching live traffic is an origin concurrency cap with an optional AIMD controller: if measured origin P99 exceeds a ceiling or errors exceed 5%, cut the allowed concurrency by 30%, otherwise add one. The library also has a dynamic stale-window rule, a frequency-based admission filter and a shadow and canary framework that the serving path does not call. The legacy PIE controller is a threshold stub that is not instantiated. The adaptive version did not beat a static cap, so the product decision is to ship the static cap path as the MVP.

**Where taught:** Ch 7, 7.8.7 (box in 7.9). **Evidence to know:** `intelligence/candidates.rs::OriginConcurrencyCapController::step`; `cycle1_lab_intelligence_deps_review.md` section 5; `m6_sealed_results.md`.

### INN-5. What is the actual intelligence in your system? Is it AI? (Chapter 5 version)

**Answer.** It is not AI: there is no learned model. On the live path there are latency and hit telemetry and one AIMD cap on concurrent origin fetches, the rule TCP uses. Beside it, a library covers workload detection, shadow evaluation of eviction policies, a statistical decision engine, canary routing and a watchdog; it is tested but not wired into the gateway. The legacy PIE stub is a threshold rule and is not constructed in the shipped binary. The sealed pre-registered simulation showed the adaptive controller did not beat a good static cap, so the defensible feature is the static cap.

**Where taught:** Ch 5, 5.14. **Evidence to know:** `intelligence/executor.rs`, `candidates.rs`, `engine.rs`; `src/control/pie.rs`; `m6_sealed_results.md`, `pie_audit.md`; G4C F12, F13, F14; AIMD cap IMPLEMENTED, TESTED (arithmetic and cap unit tests; serving path only when a cap is configured in canary or active mode), benefit NOT established; the rest PROTOTYPE ONLY or library.

### INN-6. Your own experiment says the adaptive controller does not beat a static cap. Why should anyone use the adaptive mode?

**Answer.** On current evidence they should not, and the documentation says so. The sealed result is simulated, so it does not prove adaptation never helps, but it gives no evidence that it does. The feature that showed a large effect in the simulator is the static cap itself, so the product position is a configurable static cap with adaptive mode as an experimental option. The negative result was kept because hiding it would be worse than lacking a positive one. The simulator is deterministic and uses the real executor and cache code, and a loopback run (Scenario B) found the controller acting as a static cap near 12, but neither is a multi-host measurement.

**Where taught:** Ch 5, 5.14. **Evidence to know:** `m6_sealed_results.md`; `claims_ledger.md` section 6 (C1 NOT SUPPORTED, narrowed to Tier-1); `lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`; `docs/user/limitations.md`; adaptive benefit NOT established; static cap IMPLEMENTED.

### INN-7. What is your strongest technical contribution?

**Answer.** Two things, ranking the method above the mechanisms. First, the evaluation discipline: a deterministic simulator running the real cache and executor, a preregistered rule applied by code, a hash-committed sealed set, and a negative result reported. Second, the cache core's explicit invariants (generation fencing, epochs, hard staleness cap, byte accounting), each with tests. Neither is called a breakthrough: careful engineering and honest reporting. The strongest evidence is the M6 sealed run (12,000 cells, 30 sealed seeds, H1 NOT_SUPPORTED), which has not been independently reviewed.

**Where taught:** Ch 7, 7.8.4 (box in 7.9). **Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`; `src/lab/m6_pipeline.rs::compute_verdict`; `docs/release/known_limitations_frozen_code.md` section 1.

## 8. Evaluation

### EVA-1. You have hundreds of tests. Do they prove your concurrent cache is correct?

**Answer.** No. The suite includes useful kinds of test: a protocol model checked exhaustively, a mutation detector, concurrent stress schedules over real threads and property tests. They give real confidence in specific invariants, but run in a debug build on one machine, stress tests sample interleavings rather than enumerate them, and the reported race G4A-01 is not settled by the suite. The honest statement is tested for the invariants named, not proven, not independently reviewed. Adding a deterministic checker such as loom, a G4A-01 regression test that fails without a fix, and release-profile runs is future work, not done.

**Where taught:** Ch 3, 3.12. **Evidence to know:** `concurrency_verification_tests.rs` (`test_exhaustive_protocol_model_invariants`, `test_protocol_model_mutation_detector`, `test_real_cache_concurrent_stress_schedule`, proptest); G4A-01; `claims_ledger.md`; TESTED (named invariants). `loom` is not a dependency.

### EVA-2. How did you measure performance, and why should I trust the numbers?

**Answer.** There are two kinds of real measurement, both on one Windows laptop on loopback: micro-benchmarks repeated ten times in separate processes with bootstrap intervals, and a 30-run loopback experiment on origin protection built in release. The micro-benchmarks' build profile is not established (the benchmark README says no release-profile result is established), they predate the current tree, no other cache was measured, and no multi-host run exists. The same benchmark repeated across four sessions gave non-overlapping intervals, so the within-session interval understates machine variation and no single figure is quoted. None establishes a release-profile or comparative claim and the book makes no superiority claim; treat them as local, dated evidence, not a product claim.

**Where taught:** Ch 6, 6.12.3 (see 6.13 for every measurement). **Evidence to know:** `bench-results/r0_summary.json`, `bench-results/manifests/*.json`, `benches/*.rs`; `claims_ledger.md`; status MEASURED (local, dated), not INDEPENDENTLY VERIFIED.

### EVA-3. Is your benchmark fair? Could you be fooling yourselves?

**Answer.** The micro-benchmarks compare the implementation with itself, not a competitor, so the main risks are noise and coordinated omission, handled by repetitions and recorded limits (hybrid-core laptop, Windows scheduler). For the origin-cap experiment a fairness review found real problems, published in a notice: the adaptive arm collapsed to a fixed cap near the minimum, arms differ by more than the cap because capped arms run in canary mode, success-only latency must be reported separately, cap values were chosen after one pilot run, and the origin has no load-dependent slowdown. The micro-benchmark and gateway load are closed-loop; Scenario B's generator is open-loop. The result is informative about mechanics, not evidence the adaptive controller works.

**Where taught:** Ch 6, 6.12.3. **Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`; `lab-results/scenario_b/20261030T003932Z/summary.json`; `bench-results/r0_summary.json` `limitations_verbatim`; MEASURED_LOOPBACK with recorded qualifications.

### EVA-4. Your results are on loopback with a simulator. How do you know this reflects reality?

**Answer.** Simulation models the system and workload on a virtual clock (repeatable, fast); a real run has the actual gateway process on real sockets, even on one laptop. They are labelled apart: T1 simulated, loopback measured on real binaries. The simulated study found no adaptive benefit and the loopback study found a cap lowers peak origin load at a client-success cost; neither says anything about a real network, several machines or production traffic, and loopback shares one CPU between load generator, gateway and origin. No agreement study between T1 and T2 was run (claims ledger C7: UNVERIFIED, F05), so no "simulation predicts reality" claim is made.

**Where taught:** Ch 6, 6.13. **Evidence to know:** `docs/glossary.md` section 5 tiers; `lab-results/README.md`; claims ledger C1, C7; Chapter 7; MEASURED_LOOPBACK and SIMULATED kept separate.

### EVA-5. What evidence do you have for your claims? How do you know your results are not an artefact of the simulator?

**Answer.** Each tier shows something different. T1 runs the real cache code against a modelled origin and tests policy logic and cache semantics, not networking; T2 runs the real gateway on loopback with a scripted origin. No agreement study was run, so it cannot be said the simulator predicts reality. The simulation is deterministic and tested for determinism, and the analysis is preregistered and mechanical; the main result is negative, which is not what a flattering simulator would give, but that does not show it is right. The simulator could hide a real benefit: the dynamic stale window never bound at the registered horizon, and the T2 origin has no overload feedback.

**Where taught:** Ch 7, 7.5 and 7.8.5 (box in 7.9). **Evidence to know:** `src/lab/matrix_runner.rs::AgreementStudy` and its synthetic-fixture test; claims ledger C7 UNVERIFIED; `verdict.json` H2 section.

### EVA-6. What is the difference between your simulation and a real test?

**Answer.** In T1 the clock is virtual, origin and clients are models, and the real cache and policy code run in-process as a library; nothing touches a network or HTTP stack, and results are exactly repeatable. In T2 the real gateway runs as a child process, the load generator uses real sockets and real time, and the origin is a small scripted server: real, but loopback on one machine with shared CPU, no network latency and an origin with no queue. Neither is a production or multi-host measurement; the interface labels them Simulated versus Real with the qualifier "loopback, one host". T2 alone is not enough because real-time runs are not repeatable, cost minutes each and cover one scenario.

**Where taught:** Ch 7, 7.5.3 (box in 7.9). **Evidence to know:** `src/lab/t1_sim.rs` header; `src/lab_live/mod.rs`; `ui/src/lib/labProvenance.js`; `scenario_b_interpretation_notice.md`.

### EVA-7. How would I reproduce one of your experiments?

**Answer.** Every run folder has a manifest with source file hashes, git head, build profile, seeds, scenarios and arm configurations. Check out the same source, build in release and run the same `RunSpec` with the `phantom-lab` command; the cells file should be byte-identical on the same machine. For the sealed M6 run nobody can regenerate the seeds without the secret salt, but the analysis can be re-run on the retained cells to check the verdict. T2 Scenario B is real time, so a re-run gives a fresh sample that should agree within the intervals, not byte for byte. Cross-platform determinism has not been tested.

**Where taught:** Ch 7, 7.4.6 (box in 7.9). **Evidence to know:** `src/lab/eval.rs::run_matrix_with_progress` (manifest fields); `docs/release/reproduction_guide.md` (exists; not read for the chapter); `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes`.

### EVA-8. What does "H1 NOT_SUPPORTED" actually say, and what does it not say?

**Answer.** On 30 sealed seeds, 8 origin-degradation scenarios and 4 held-out workload families plus one sealed family, an AIMD controller allowed to range at least 4 times its minimum did not reduce peak origin concurrency in any scenario versus the tuned static baseline, and was credibly worse in seven. It reduced client P99 in two scenarios and broke the hit-ratio and error-rate limits in three. It does not say adaptive caching cannot work, says nothing about real networks and does not test overhead or the stale-window rule. The baseline BL-12b is not claimed to be the best deployable static policy.

**Where taught:** Ch 7, 7.6 (box in 7.9). **Evidence to know:** `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json`; `docs/research/m6_sealed_results.md` "What this does and does not establish".

### EVA-9. Scenario B shows a 64% drop in origin load for the adaptive arm. Isn't that a win?

**Answer.** No. The adaptive arm fell from 32 to about 12 concurrent fetches within two seconds and stayed there, behaving as a static cap of about 12. Client success dropped by about 17 percentage points because refused requests count as failures, and the mechanical verdict for both capped arms is trade-off failure. The experiment shows that a cap is enforced by the real binary and that the price is refused requests. Why it stayed near 12 (a window maximum of 30 samples against an absolute 60 ms ceiling) is our reading of the notice and code, not a recorded root-cause analysis.

**Where taught:** Ch 7, 7.5.2 (box in 7.9). **Evidence to know:** `docs/demo/scenario_b_interpretation_notice.md`; `lab-results/scenario_b/20261030T003932Z/summary.json`; `lab_live/scenario_b.rs::AIMD_EFFECTIVE_CAP_NOTE`.

### EVA-10. The sealed evaluation says H1 is NOT_SUPPORTED. What does that mean for your "intelligent" caching claim, and what happens to the product?

**Answer.** There is no evidence the adaptive controller beats a well-tuned static origin cap, so that claim is not made. In the simulated, pre-registered, run-once evaluation the adaptive arm did not reduce peak origin concurrency in any of eight scenarios and in seven was credibly worse; the unconstrained tuning optimum was itself a static cap. The research lead chose the static gateway as the minimum viable product and did not promote the adaptive controller to the serving path. It does not prove adaptation is useless. The run was reviewed by the lead and AI reviewers for hash-chain integrity, not by a human, the reviewers did not read the simulation core, the P99 endpoint was quantised and the stale-window mechanism was never exercised.

**Where taught:** Ch 8, 8.13.3 (see Ch 7, 7.6). **Evidence to know:** `docs/research/m6_sealed_results.md`; `known_limitations_frozen_code.md` section 1; status TESTED, tier SIMULATED. Next step: a headroom experiment.

### EVA-11. What are your performance numbers?

**Answer.** None are published, deliberately. The numbers in the repository come from simulation, loopback on one host, or benchmark runs whose build profile is not established; none is a release-grade product measurement, and earlier figures such as a 200 times eviction speedup and a 0.45 microsecond controller overhead are untraceable to any retained artefact, so they were withdrawn. There are micro-benchmarks with bootstrap intervals from 2026-10-28 that predate the current tree, and one release-profile loopback experiment (Scenario B), but no release-profile general suite and no soak run. A claim would need a release build, stated hardware, workload and seed, a baseline, repetitions and intervals. A hit is an in-memory map lookup behind a shard lock; how fast, and how it compares with Nginx or Varnish, cannot be said.

**Where taught:** Ch 8, 8.13.3 (every retained measurement is in Ch 6, 6.13). **Evidence to know:** `claims_ledger.md` C3, C5; `docs/user/limitations.md`; `CLAUDE.md` section 4; status DEFERRED.

### EVA-12. Your tests pass. Why should I believe the system is correct?

**Answer.** You should not believe correctness from passing tests, and the project's rules say so. The 851 tests run in debug mode on one Windows machine; some assert stub behaviour (the legacy PIE tests), the stale-cap counter is close to a tautology and a UI fixture masked a real defect. The tests do give regression protection and invariant checks such as byte-accounting equality and generation fencing in single-process cases. For more, there are named gaps: no mutation test showing fencing tests fail without the fence, no model checking, no release-profile run. The weakest test is the one behind the stale-cap violation counter (G4A-18), described by the reviewer as near-tautological; this was not re-derived.

**Where taught:** Ch 8, 8.13.1 (see Ch 8, 8.2.2). **Evidence to know:** `docs/research/code_findings.md` F3; G4A-18; G4C F01 fixture at `ui/test/lab.view.test.mjs`; `claims_ledger.md` section 4.

## 9. Limitations and honesty

### LIM-1. Your tests pass. Does that mean the system is correct and ready?

**Answer.** No. The tests are evidence, not proof. The last full recorded run is 851 passing tests in a debug build on one Windows machine, which shows the behaviours the authors thought to check hold in that setting, not release builds, Linux, several machines or sustained load. Some tests assert known stubs. Every review so far is an AI second look, and the status page says nothing has been formally accepted by the owner and findings remain open. It is described as a research prototype with a tested core and an experimental cluster.

**Where taught:** Ch 1, 1.11. **Evidence to know:** `docs/STATUS.md` sections 1 to 3b; the disposition register; `docs/user/limitations.md`; IMPLEMENTED, TESTED (debug, loopback); INDEPENDENTLY VERIFIED: none. Open items to name: lost owed invalidations on issuer crash, member-wide purges, the unwired default TTL, no release-profile suite.

### LIM-2. Who has independently verified this system?

**Answer.** No human has. Every review so far is an AI second look: a separate AI session or the lead re-running a command, all inside one tooling environment. What exists is breadth: 851 recorded debug tests, three read-only reviews with 99 findings, an independent Node recomputation of the Scenario B results and a hash-checked freeze chain for the sealed evaluation. A human reviewer, an external security review and a physical multi-host run are blocked, waiting on reviewers and devices. An AI second look counts as evidence that a second reader found things the first missed (G4 found a High finding the authors had not), but not as independence.

**Where taught:** Ch 8, 8.13.1. **Evidence to know:** `docs/release/workstream_completion_ledger.md` section 6; `docs/release/verification_register.md` (independence classes); label INDEPENDENTLY VERIFIED: none.

### LIM-3. What is still unresolved? Give me the honest list.

**Answer.** There are 99 findings from three AI reviews: 75 open, 20 recorded as limitations in frozen research code, 2 in sandbox remediation, 1 under investigation and 1 refuted; no Critical. The High ones are the unauthenticated slow-body weakness at the HTTP ingress, a suspected timing hole in the stale-write fence, a conditional S3-FIFO memory growth problem (no live caller selects it) and a Lab verdict labelling bug whose fix is in a sandbox. The owner-flagged Medium items are the control endpoint touching the live router, member authorisation for tenant-wide invalidations, the missing epoch check on invalidation frames, static membership with no live revocation, and connections that never age out. No dependency scan, penetration test or fuzzing has been done, and none of the reviews is independent human review.

**Where taught:** Ch 6, 6.14 (see Ch 8, 8.4). **Evidence to know:** `cycle1_findings_disposition_register.md` section 3; `owner_decisions_register.md` ODR-1 to ODR-4; `known_limitations_frozen_code.md`; all findings OPEN unless the owner dispositions them. The counts are the disposition agent's; "CONFIRMED" means confirmed by reading code, and items cited were re-read, not exploited.

### LIM-4. What would you improve first, and what are your real bottlenecks?

**Answer.** Security first: authenticate before reading the body and add timeouts and connection limits to the main API (the one High finding that is unauthenticated and remote); then an owner decision on cluster member authorisation; then durable owed invalidations. For performance, the evidence shows the hit path is cheap and contention appears with many threads and many tenants; this should be profiled on a quiet machine with a release build before changing anything, plus a multi-host open-loop experiment, because everything measured is on one laptop. No features before the High findings are dispositioned, and no throughput numbers without full conditions.

**Where taught:** Ch 6, 6.14. **Evidence to know:** G4B-01, G4B-05, G4B-09; `bench-results/r0_summary.json` B2 results; `docs/release/lead_working_state.md`; status: plan, not implementation.

### LIM-5. Where are established products stronger than PhantomCache?

**Answer.** Many places: production maturity and tooling in Redis, Memcached, Varnish and nginx; cross-node duplicate suppression and hot-item mirroring in groupcache, where PhantomCache's GET is local-only; invalidation durability, where cross-node invalidations are best-effort after the first ack and owed invalidations are lost if the issuer dies; adaptive concurrency in Envoy and Netflix using relative signals while PhantomCache's absolute ceiling did not beat a static cap; eviction research over thousands of traces; per-tenant quotas, which are absent; and head-to-head performance numbers, which do not exist. First fix: invalidation durability, then a load-dependent test origin, then a relative-signal controller under a new preregistration.

**Where taught:** Ch 7, 7.8.6 (box in 7.9). **Evidence to know:** claims ledger C6 qualifier; `src/dist/router.rs` (constant weight); `rt0_prior_art_sweep_2026-10-30.md` T7, T10; Ch 8 limitations.

### LIM-6. You report that your adaptive controller failed. Why should I trust anything else?

**Answer.** Reporting the failure is the reason to trust the process: the rule was written before the sealed seeds were used and applied mechanically by code. The weaknesses are listed: the amendment forcing the controller to be adaptive was chosen after seeing development results, the stale-window mechanism was never exercised at that horizon, the primary P99 includes quickly refused requests, and nobody independent has reviewed the pipeline. The failure does not mean adaptive control is useless; it means this controller, with an absolute ceiling, shows no benefit in this simulation. A new hypothesis (relative-signal controller, load-dependent origin) needs a new protocol.

**Where taught:** Ch 7, 7.6 (box in 7.9). **Evidence to know:** `preregistration_m6_v2_2_amendment.md` sections 3, 5, 7b; `m6_sealed_results.md`; `rt0_prior_art_sweep_2026-10-30.md` T7.

### LIM-7. What would you improve with more time? (Chapter 7 version)

**Answer.** In order: make invalidations durable; make GET able to use the owner, or at least cluster-wide single flight, which is where groupcache is ahead; make the Scenario B origin load-dependent and add a cap-free canary arm; run a real T1-to-T2 agreement study; write a new preregistration for a relative-signal controller; run an overhead benchmark in release on a named host; and get an independent human review of the Lab core and a BCa cross-check against a reference implementation. Nothing is claimed faster or more consistent than anything until a fair head-to-head run exists.

**Where taught:** Ch 7, 7.8.5 (box in 7.9). **Evidence to know:** `docs/release/known_limitations_frozen_code.md` section 5; claims ledger C3, C6, C7; Ch 8 roadmap.

### LIM-8. What would you do with more time? (Chapter 8 version)

**Answer.** In order: get a human to review the protected core, since everything known about it is AI second looks; integrate the three sandbox fixes after review (durable invalidation log, the fence fix for the double epoch read, the Scenario B cancelled-run fix), each with a test that fails without it; close the ingress holes; run a release-profile suite and soak on a quiet machine and the cluster on real hardware over a network; only then start tenant quotas and the headroom experiment. The protected core is frozen for evidence integrity and safety: changes to fencing and invalidation need a failing-first test, independent review and the owner's approval.

**Where taught:** Ch 8, 8.13.5 (roadmap 8.8 to 8.9). **Evidence to know:** `docs/release/execution_register.md`; `owner_decisions_register.md` ODR-2 to ODR-4; Figure 8.2.

### LIM-9. What have you chosen not to claim, and what is untested?

**Answer.** Not claimed: production readiness, a distributed cache, linearizability, bounded staleness, adaptive benefit, ML, performance numbers or independent verification. Untested: any Linux run, any physical network, clusters beyond four nodes, power loss and fsync-error behaviour, a release-profile suite, a two-hour soak, the natural-schedule rate of the fence race, ingress defaults under attack, and human usability and screen readers. Also unverified: whether the simulator ranks scenarios as a real process would, because no agreement study was run. The worry that ranks highest is the unreviewed fencing and invalidation core, because the product's one hard promise is that a deleted object is not served after an acknowledged delete.

**Where taught:** Ch 8, 8.13.5 (checklist 8.10, claims 8.11). **Evidence to know:** 8.10 checklist; 8.11 table; `m3_durability/remediation/00_status_and_plan.md` section 7.

### LIM-10. Your repository is mostly uncommitted. Isn't that a process failure?

**Answer.** It is a delivery risk and it is listed first. The cause is a deliberate policy: commit only individually reviewed file groups, never everything at once and never groups with unresolved blocking findings. Documentation and tooling groups are committed; code groups wait for the review gate and an owner check on held preregistration files. The risk is real: one machine and one local backup. The mitigation is to commit reviewed groups as they clear and to make an off-machine copy, still an open action. No push has occurred; commit and push need an explicit owner instruction.

**Where taught:** Ch 8, 8.13.5 (see 8.2.1). **Evidence to know:** `docs/release/owner_decisions_register.md` ODR-3; `docs/STATUS.md` section 5b; `git log --oneline -8`.

### LIM-11. The README and an older handoff say the system passed a formal acceptance. Why do your records say otherwise?

**Answer.** Those statements came from agent self-reports and were later audited. The handoff's "Phase 4 accepted, zero P0" is not supported by later review: no dated owner acceptance is recorded for any milestone, the M3 completion report said "zero compromises" before a durability defect was reproduced, and the M6 report was retracted. The old documents are kept as history with correction notices, and `docs/STATUS.md` and the verification register are the entry points. The owner may hold acceptance records elsewhere; the register calls the absence an evidence gap, not proof that none was given.

**Where taught:** Ch 8, 8.13.5 (see 8.7.2). **Evidence to know:** `docs/handoff/CORRECTIONS.md`; `docs/STATUS.md` section 1; `CLAUDE.md` section 6.
