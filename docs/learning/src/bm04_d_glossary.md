# Appendix D: Glossary and abbreviations

This glossary lists the terms and abbreviations used in Chapters 1 to 8, in alphabetical order. Each entry is one line in plain English, followed by the place where the chapter teaches it (for example `Ch 4, 4.5` means Chapter 4, section 4.5). The definitions follow the way the chapters use the words. Where a term has a status or a caveat in the chapters, the entry says so. Code names are given as in the source tree; Appendix F maps them to files.

Three conventions apply throughout. A **status label** (IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, DEFERRED, BLOCKED, UNVERIFIED, INDEPENDENTLY VERIFIED) always has the meaning given in Ch 8, 8.1. A finding identifier such as `G4A-01` points to the review register described in Ch 8, 8.4. A tier label (T1, T2) says how a number was produced (Ch 7, 7.1).

**A**


- **Active** (node state): the state of a cluster member that is in service; peers in a configured cluster start in it. One of seven values of `NodeState`, of which `Degraded` is never assigned. (Ch 5, 5.2)
- **ADR** (architecture decision record): a file in `docs/research/adr/` that records an architectural choice and its reason; the chapters separate "recorded decision" from "our reasoning". (Ch 3, 3.0)
- **Advisory scan**: a check of locked dependencies against known vulnerability reports (for example `cargo audit`). None has been run; the chapters list it as DEFERRED. (Ch 3, 3.13)
- **Age**: how old a stored response is; compared with its freshness lifetime to decide fresh or stale. (Ch 4, 4.8)
- **AIMD** (additive increase, multiplicative decrease): the rule behind TCP congestion control, used by the optional origin-concurrency cap controller: add a little when healthy, cut sharply on trouble. IMPLEMENTED, TESTED (arithmetic and cap unit tests); on the serving path only when a cap is configured in canary or active mode; its benefit is NOT SUPPORTED. (Ch 5, 5.13)
- **Amortized O(1)**: some single operations cost more, but the average over a long run is constant per operation; used for eviction. (Ch 4, 4.9)
- **Anti-entropy**: a background process that compares state between nodes and repairs differences; here, a pull of missed invalidation records from the sender's ring buffer. (Ch 5, 5.1 and 5.5)
- **Arc**: the Rust shared-ownership pointer; the epoch snapshot is an `Arc` of a map, so readers share it without an exclusive lock. (Ch 3, 3.5.4)
- **Arm**: one configuration compared in an experiment (for example `BASE`, `STATIC-CAP`, `AIMD-CAP` in Scenario B). Arms differ in configuration, not through hidden multipliers. (Ch 7, 7.2.7 and 7.5.1)
- **Async / await**: Rust's way to park a task while it waits on a socket or timer so a thread can run another task. Never combined with a held `std::sync::Mutex`. (Ch 3, 3.3 and 3.5.1)
- **Atomic**: a counter or flag that threads update without a lock; used for statistics and for epochs read on the hit path. (Ch 3, 3.5.3)
- **Audit store** (decision audit store): a bounded buffer of decision records in `src/intelligence/audit.rs`; the serving path has no producer, so the route returns an empty store. (Ch 1, 1.8; Ch 2, 2.3.1)
- **Authentication / authorisation**: authentication asks "who are you?"; authorisation asks "may you do this?". PhantomCache checks a JWT and then the role. (Ch 6, 6.2)
- **Axum**: the Rust web framework (on hyper and tower) that routes HTTP requests in the gateway. (Ch 3, 3.4)

**B**


- **BASE** (arm): the Scenario B arm with intelligence off and no cap. (Ch 7, 7.5.1)
- **BCa** (bias-corrected and accelerated): a bootstrap confidence interval that corrects for bias and skew; the Lab uses it for paired differences. (Ch 7, 7.3.3)
- **Bearer token**: a credential sent in the `Authorization` header; whoever holds it is treated as the caller. (Ch 6, 6.2)
- **BL-12b**: the tuned static baseline (a static cap and stale window chosen on tuning seeds) used in the sealed M6 evaluation; the code says it is not claimed to be the best deployable static policy. (Ch 7, 7.6 and 7.9)
- **Bootstrap**: draw values with replacement from the sample many times to estimate the spread of a statistic. (Ch 7, 7.3.3)
- **Bounded staleness**: a read never returns data older than a named bound `B`. To claim it you must state `B`, the failure model and a test that could falsify it. Not claimed. (Ch 5, 5.6)
- **Byte budget**: the logical memory limit (`PHANTOM_MAX_CACHE_BYTES`, default 256 MiB), divided across shards. (Ch 4, 4.6)
- **Byte accounting (I1 to I8)**: the invariants that the running byte total equals the recomputed sum of entry sizes, checked by `audit_bytes()`. TESTED for consistency only. (Ch 4, 4.6)

**C**


- **Cache**: a store of copies, keyed so that a later request for the same thing can be answered without calling the origin. (Ch 1, 1.1)
- **Cache key**: the identity of one cached item: tenant plus request path plus query string (`CacheKey`). (Ch 1, 1.1; Ch 2, 2.3.2)
- **Cache poisoning**: an attacker makes the cache store something that other users then receive. The key contains every input that influences the origin response, which limits it. (Ch 6, 6.6)
- **Cache stampede** (thundering herd): many simultaneous misses for one key, each going to the origin. (Ch 1, 1.3.1; Ch 4, 4.4)
- **Cache-Control**: the origin's header that sets lifetime and stale rules (`max-age`, `s-maxage`, `must-revalidate`, `stale-while-revalidate`, `stale-if-error`). (Ch 4, 4.8)
- **Canary** (mode): the setting `PHANTOM_INTELLIGENCE=canary`. In this code it applies the origin cap to all fetches with no rollback in the serving path, so it is not a canary in the usual sense. (Ch 5, 5.13)
- **Canary routing**: a library function that sends a fraction of decisions to a candidate; not wired into the gateway. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Canonical string**: one agreed text form of a key, used on the wire between nodes. (Ch 4, 4.1)
- **Cancellation**: a client giving up mid-request; the origin fetch runs in its own detached task and RAII guards release state, so a cancelled leader does not strand waiters. (Ch 4, 4.4 and 4.5)
- **Cell** (Lab): one run of one scenario, workload family, arm and seed; the sealed M6 evaluation had 12,000 of them (8 scenarios, 5 families, 10 arms, 30 seeds). (Ch 7, 7.2 and 7.6)
- **Claims ledger**: `docs/research/claims_ledger.md`, the record of which project claims are supported, narrowed, amended or untraceable (for example C6 and C7). (Ch 6, 6.9.3; Ch 7, 7.5.3)
- **Cluster CA**: the private certificate authority whose certificates cluster members must chain to. (Ch 3, 3.7; Ch 6, 6.4)
- **Coalescer**: a legacy type kept for compatibility; the serving path coalesces through `PartitionedCache::begin_fetch_atomic` and `SharedOp` instead. (Ch 1, 1.3.1; Ch 2, 2.2.1)
- **Coalescing** (request coalescing): see single-flight. (Ch 4, 4.4)
- **Concurrency gate** (origin gate, `OriginGate`): a counter of permits that limits concurrent origin fetches; enforced only in `canary` or `active` mode with a configured cap. (Ch 4, 4.4; Ch 5, 5.13)
- **Concurrency vs parallelism**: concurrency is many tasks making progress in overlapping time; parallelism is tasks running at the same instant on different cores. (Ch 4, 4.2)
- **Coordinated omission**: a load-test error where a slow response delays later requests, hiding the slowness; closed-loop tests suffer from it, an open-loop generator does not. (Ch 6, 6.12.1 and 6.12.3)
- **CORS**: the browser rule for cross-origin calls; the gateway allows only exact configured origins and never credentials. It is not authentication. (Ch 2, 2.10; Ch 3, 3.4)
- **Count-Min Sketch**: a small fixed table that estimates how often each item was seen; it only overestimates. The library hot-key detector uses one; the older `CountMinSketch` in `src/intel/sketch.rs` is unused. (Ch 5, 5.12)

**D**


- **Debug build / release build**: the Cargo build profiles; all recorded tests ran in debug, and no release-profile suite exists on the current tree. (Ch 6, 6.12.1; Ch 8, 8.2.2)
- **DEC-006**: an owner-approved decision (Option B) behind the `.gitattributes` entry that protects four hash-pinned frozen files from line-ending conversion. (Ch 8, 8.7.1)
- **Decision engine** (`PieDecisionEngine`): the library that compares a candidate policy with a baseline from shadow evaluations; not wired; its statistics have findings F12 and F13. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Default TTL setting** (`PHANTOM_DEFAULT_TTL_MS`): validated and printed but not read at runtime; the effective default lifetime is a fixed 60 seconds. PARTIAL. (Ch 2, 2.6.1; Ch 4, 4.8)
- **DELETE**: `DELETE /api/v1/cache/<path>`; purges locally, waits up to 150 ms for the primary's acknowledgement, then tells other members in the background. 204 is not a cluster-wide acknowledgement. (Ch 2, 2.7.3; Ch 5, 5.5)
- **Deterministic simulation**: a simulation in which the same seed and source give the same bytes, because time and randomness are controlled. (Ch 7, 7.2 and 7.4.1)
- **Disposition register**: `cycle1_findings_disposition_register.md`, which lists the 99 G4 findings and their status. It closes nothing; only owner statements set dispositions. (Ch 8, 8.4.1)
- **DNS pinning**: resolve the origin name once, check every returned address, and connect to the exact address checked, so DNS changes cannot bypass the filter. (Ch 6, 6.8)
- **DR-027** (G4B-05): any active cluster member can send tenant-scope or full-cache invalidations that flush other nodes; owner-named, OPEN. (Ch 5, 5.8; Ch 6, 6.4)
- **DR-029** (G4B-30): the peer `GetRequest` handler fabricates a `val_node_` value for a missing key; unreachable by HTTP because key spaces are disjoint; a hazard for any future peer-fill. PROTOTYPE ONLY, OPEN. (Ch 5, 5.8)

**E**


- **E0, E1, E2**: the first planned intelligence experiments: cost of an event tap, validity of a load-dependent origin plant, and the headroom question (can any per-scenario cap beat one good static cap). Not run. (Ch 8, 8.9.4)
- **Epoch**: a counter that makes old state invalid by moving a number instead of scanning. The chapters name three: tenant and global epochs (per cache), the membership epoch (per cluster, static), and the stream sequence (per sender and receiver pair). (Ch 4, 4.7; Ch 5, 5.4)
- **ETag**: an origin validator stored with an entry and sent back in `If-None-Match` for revalidation. (Ch 4, 4.8)
- **Eventual consistency**: if writes stop and faults heal, all copies converge; it says nothing about how long. (Ch 5, 5.6)
- **Eviction**: removing entries to stay inside the byte budget or entry limit. (Ch 4, 4.9)
- **Error fan-out**: when the origin fails, every waiter on that fetch receives the same error. (Ch 4, 4.4)
- **Event queue**: the Lab's time-ordered to-do list; equal times are ordered by a sequence number so replays are identical. (Ch 7, 7.2.2)

**F**


- **Fail-closed**: refuse to start or refuse a request when a safety condition is not met, instead of continuing. The configuration loader and the startup security gate do this. (Ch 2, 2.2.1; Ch 6, 6.2)
- **Fan-out**: sending one message to many receivers; here, one invalidation to every member. (Ch 5, 5.1 and 5.5)
- **Fencing** (generation fencing): a fetch records a number when it starts and may store its result only if that number is unchanged when it finishes. (Ch 1, 1.3.4; Ch 4, 4.5)
- **Fencing token**: the number recorded at the start and compared at the end. (Ch 4, 4.5)
- **Fingerprint** (certificate): a hash of a member's certificate pinned in the membership file; checked in addition to the chain. (Ch 3, 3.7; Ch 5, 5.2)
- **Frame**: one message of the PhantomMesh binary protocol (magic, version, 28-byte header, type). (Ch 2, 2.2.1; Ch 5, 5.2)
- **Freshness lifetime**: how long a copy may be treated as correct; after it the copy is stale. (Ch 1, 1.1; Ch 4, 4.8)
- **Frozen files**: sources and records whose bytes are pinned by the pre-registration freeze; edits need a signed addendum, so defects there are recorded as limitations. (Ch 7, 7.4.4; Ch 8, 8.6 and 8.7.1)

**G**


- **G1 to G5** (delivery gates): G1 Scenario B, G2 Lab API, G3 UI plus browser verification, G4 two whole-product review cycles, G5 reviewed commits and push. (Ch 8, 8.2.3)
- **G4, G4A, G4B, G4C**: gate G4 and its three AI reviews: G4A core engine (26 findings), G4B distribution, security and API (38), G4C lab, intelligence, freeze and dependencies (35); 99 in all. (Ch 8, 8.4.1)
- **G4A-01**: the reported race in `finalize_fetch`: the epochs are read twice, so an invalidation landing between the reads can let an old body be stored as current. Reproduced only in a sandbox with a test hook; fix not integrated; UNDER-INVESTIGATION. (Ch 4, 4.7; Ch 6, 6.9.4)
- **Gateway**: one PhantomCache process (an HTTP caching proxy in front of an origin). Each gateway has its own cache. (Ch 1, 1.5; Ch 2, 2.1)
- **Generation**: a per-key counter that DELETE bumps; defends against "delete during fetch". (Ch 4, 4.5)
- **Ghost queue**: in S3-FIFO, a queue that remembers recently evicted keys (no bodies). (Ch 4, 4.9)
- **Global epoch**: the one counter that invalidates everything on a node; also the fallback when more than 10,000 tenant epochs exist (SEC-4A-01). (Ch 4, 4.7; Ch 5, 5.4)

**H**


- **H1**: the registered hypothesis that an origin-aware adaptive controller beats the best static configuration. Sealed verdict: NOT_SUPPORTED (simulated). (Ch 7, 7.6; Ch 5, 5.14)
- **Hard staleness cap** (`PHANTOM_MAX_STALE_MS`, default 24 hours): the limit on how long past expiry a stale copy may be served. It bounds stale serving, not the fresh lifetime. (Ch 4, 4.8)
- **Hash map**: a table that finds a key in expected constant time from a hash of the key. (Ch 4, 4.1)
- **Head-of-line blocking**: every waiter waits for the leader, however slow; the origin read has a 5-second timeout. (Ch 4, 4.4)
- **Heartbeat**: a keepalive probe on idle pooled connections every five seconds. It does not change any node state, so it is not a failure detector (G4B-12). (Ch 5, 5.2)
- **Heavy hitter**: an item that accounts for a large share of requests; found with Count-Min plus Space-Saving in the library. (Ch 5, 5.12)
- **Hit / miss**: a hit is answered from a usable cached copy; a miss needs the origin. (Ch 1, 1.1)
- **Hit ratio**: hits divided by total lookups. (Ch 1, 1.1 and 1.2)
- **Holm correction**: a step-down adjustment of p-values across many hypotheses (16 primary ones in M6). (Ch 7, 7.3.3)
- **Hot key**: a key that receives a disproportionate share of requests; handled by coalescing on a miss, not by detection. (Ch 1, 1.3.2; Ch 5, 5.12)
- **HRW** (highest random weight): another name for rendezvous hashing. (Ch 2, 2.7.2; Ch 5, 5.3)
- **HS256**: the symmetric JWT signing algorithm the code accepts; the only one. (Ch 3, 3.8; Ch 6, 6.2)
- **hyper / tower**: hyper is the low-level HTTP implementation; tower supplies reusable service wrappers. Axum sits on both. (Ch 3, 3.4)

**I**


- **Idempotent**: applying the same operation twice has the same effect as once; a repeated delete only causes an extra miss. (Ch 5, 5.5)
- **INDEPENDENTLY VERIFIED**: a status label for a check by a human or external reviewer. Nothing in the project holds it; AI reviews are "AI second looks". (Ch 8, 8.1)
- **Invalidation**: removing or marking a copy as invalid before it would expire. (Ch 1, 1.1 and 1.3.4)
- **Invalidation bus**: the experimental mTLS layer that carries invalidations between gateways; the main distributed feature. (Ch 1, 1.4; Ch 5, 5.5)

**J**


- **JWT** (JSON Web Token): a signed note stating who the caller is, the role and tenant, and when it expires. Checked for signature, issuer, audience and expiry; no revocation. (Ch 3, 3.8; Ch 6, 6.2)

**K**


- **Key**: see cache key. (Ch 1, 1.1)
- **KeyState**: the per-key record holding the entry, generation, active operations and in-flight registration. (Ch 4, 4.1)

**L**


- **Lab API**: the HTTP interface to Phantom Lab (`/api/v1/lab/*`); off by default. (Ch 2, 2.8; Ch 7, 7.7.1)
- **Latency / throughput**: latency is how long one request takes; throughput is requests completed per second. (Ch 1, 1.1)
- **Lazy invalidation**: do not remove entries at invalidation; drop them when next touched or when the sweeper finds them. (Ch 4, 4.7)
- **Leader / waiter**: the leader is the request that starts the origin fetch; waiters join it and receive the same result. (Ch 4, 4.4)
- **Linearizability**: every operation appears to take effect at one instant in real-time order, as seen by all clients. Not claimed. (Ch 5, 5.6)
- **Lock contention**: threads waiting for a lock another thread holds. (Ch 4, 4.2)
- **Lock order**: shard mutex first, then the epoch structures; never the reverse; no `std::sync::Mutex` held across `.await`. (Ch 3, 3.5.4; Ch 4, 4.2)
- **Log-linear histogram**: 2,688 counters with constant-time recording and a relative error of at most 0.39 percent by construction; IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader, so it is not the source of the percentiles the endpoint displays. (Ch 5, 5.11)
- **Logical bytes**: the cache's own size bookkeeping (key, body capacity, headers, fixed overhead); not operating-system memory. (Ch 4, 4.6)
- **Loopback**: network traffic inside one machine; no real network latency. Results are labelled MEASURED_LOOPBACK. (Ch 6, 6.12.1; Ch 7, 7.5)
- **LRU** (least recently used): evict the entry untouched the longest. (Ch 4, 4.9)

**M**


- **M3**: the durable-invalidation milestone; its sandbox prototype is not integrated. (Ch 5, 5.9; Ch 8, 8.9.1)
- **M6**: the milestone of the sealed, pre-registered evaluation of the adaptive controller (H1 NOT_SUPPORTED). (Ch 5, 5.14; Ch 7, 7.6)
- **MEASURED_LOOPBACK**: tier label for numbers from real binaries on one host; separate from SIMULATED. (Ch 2, 2.8; Ch 7, 7.5)
- **Measured / simulated / inferred / unavailable**: the four labels every displayed UI value must carry; synthetic data is never shown as live. (Ch 2, 2.8; Ch 7, 7.7.2)
- **Membership** (static): the cluster's member list in a JSON file read at startup; no join, leave or gossip. (Ch 5, 5.2)
- **ML** (machine learning): none ships; the "intelligence" is classical control and statistics. (Ch 1, 1.7; Ch 5, 5.10)
- **mTLS** (mutual TLS): both sides present certificates; used between nodes with TLS 1.3 only. (Ch 3, 3.7; Ch 5, 5.2)
- **Mutex**: a lock that one thread holds at a time. (Ch 4, 4.2)
- **MVP-B** ("the static gateway"): the research lead's product decision after the negative M6 result: single-flight, correct freshness, and a configurable static origin cap; the adaptive controller is not promoted. (Ch 7, 7.6; Ch 8, 8.6)

**N**


- **Negative caching**: remembering an error response; PhantomCache does not do it, so the next request tries the origin again. (Ch 2, 2.6.3; Ch 4, 4.10)
- **Node**: in the code, a member identity plus a connection, not necessarily a separate machine you can read from. (Ch 2, 2.2.1)
- **NodeState**: the seven-value state enum (`Joining`, `Active`, `Degraded`, `Suspect`, `Failed`, `Offline`, `Recovering`). (Ch 5, 5.2)
- **NotWired**: the configuration marker for a key that is validated but not read at runtime (`Wiring::NotWired`). (Ch 2, 2.6.1; Ch 4, 4.8)

**O**


- **ODR** (owner decision register entries ODR-1 to ODR-4): ODR-1 records frozen-code findings as limitations; ODR-3 is the commit policy; ODR-4 names six findings that must not be dismissed. (Ch 8, 8.4.1 and 8.4.3)
- **OPEN** (finding status): the finding stands; the register closes nothing. Other statuses are RECORDED-LIMITATION, IN-SANDBOX-REMEDIATION, UNDER-INVESTIGATION and REFUTED. (Ch 8, 8.4.1)
- **Open-loop / closed-loop** (load): open-loop fixes arrival times in advance regardless of responses; closed-loop waits for each response. (Ch 6, 6.12.1)
- **Origin**: the system that owns the real data; in PhantomCache, any HTTP server behind `ORIGIN_URL`. (Ch 1, 1.1)
- **Owed invalidation**: an invalidation the issuer still has to deliver to a peer; held in memory only, so lost if the issuer crashes (G4B-09). (Ch 5, 5.5 and 5.7)

**P**


- **Partition** (network): a link cut where both sides stay alive; the cut-off side keeps serving its cached copies. (Ch 5, 5.7)
- **Partitioned cache vs local-copy cache**: a partitioned cache gives each key one owner and routes reads to it; a local-copy cache lets several nodes hold the same key. PhantomCache is the second. (Ch 5, 5.1)
- **Percentile** (P50, P99): the value below which that share of requests fell; each place in the repository computes it differently. (Ch 6, 6.12.1 and 6.12.2)
- **PhantomMesh**: the name of the node-to-node binary frame protocol. (Ch 5, 5.2)
- **Phantom Lab**: the experiment toolkit: deterministic simulator T1, real-gateway loopback runner T2, a Lab API and a UI area. Not a benchmark suite. (Ch 7, 7.1)
- **PIE**: short for Policy Intelligence Engine, the name `CLAUDE.md` (section 6b) gives to the project's planned policy-decision component. The legacy `src/control/pie.rs` (`PieController`) is a threshold stub, never constructed (`pie: None` in `main.rs`), and is not the intelligence; `PieDecisionEngine` in `src/intelligence/engine.rs` is a library that no request calls. Neither is a working canary controller. (Ch 5, 5.13)
- **Pinned**: an entry with an active operation or in-flight fetch; it may not be evicted. (Ch 4, 4.5)
- **Placeholder**: a `KeyState` with no entry, created at the start of a fetch so the key can be tracked and pinned. (Ch 4, 4.5)
- **Poisoned mutex**: a `std::sync::Mutex` left poisoned when a thread panics while holding it; most core paths recover. (Ch 4, 4.2; Ch 6, 6.10)
- **Pre-registration**: writing down the analysis rule before opening the sealed data, so the verdict cannot be chosen afterwards. (Ch 7, 7.4.4)
- **Primary** (canonical primary): the node rendezvous hashing picks to acknowledge a DELETE first. (Ch 5, 5.3 and 5.5)
- **Property test** (`proptest`): a test that runs random inputs and shrinks failures; random search is not proof. (Ch 3, 3.12)
- **Protected core**: `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml`; changes need a failing-first test, independent review and owner approval. (Ch 8, 8.7.1)
- **Provenance label**: the tag on a value saying how it was produced (measured, simulated, inferred, unavailable). (Ch 2, 2.8; Ch 7, 7.7.2)
- **PROTOTYPE ONLY**: status label for code that exists but is not wired into the product or not integrated. (Ch 8, 8.1)

**Q**


- **Quota** (tenant): a per-tenant resource limit. None exists; one tenant can use the whole byte budget. DESIGNED BUT NOT IMPLEMENTED. (Ch 6, 6.6; Ch 8, 8.9.2)

**R**


- **RAII guard**: a Rust object that releases a resource when dropped, so cancellation and panics do not leak permits or in-flight state. (Ch 4, 4.5)
- **Read-through**: the gateway fetches from the origin on a miss and stores the result. (Ch 2, 2.2)
- **Read-your-delete**: after a DELETE returns on a gateway, later reads on that gateway do not return the old body; weaker than linearizability. (Ch 5, 5.6)
- **Rendezvous hashing**: give each node a score from the key and node id; the highest score owns the key. In PhantomCache the weight is a constant 1.0 and the result only picks the DELETE primary. (Ch 5, 5.3)
- **Repair ledger**: a bounded in-memory record of invalidations a peer still owes or is owed; re-sent every 250 ms until acknowledged. (Ch 5, 5.5)
- **Revalidation**: ask the origin whether a stored copy is still good, using `If-None-Match`; a 304 answer renews the copy. (Ch 4, 4.8)
- **Revocation** (certificate): removing a member's trust. The machinery exists but nothing triggers it in a running process. (Ch 5, 5.2; Ch 6, 6.4)
- **RFC 9111**: the HTTP caching standard; PhantomCache implements a subset ("RFC 9111-style"), not a verified compliant cache. (Ch 4, 4.8)
- **Ring buffer**: a fixed-size buffer in which the newest records overwrite the oldest; holds sent invalidation records for replay. (Ch 5, 5.5)
- **Role** (RBAC): the permission carried in the token: `cache:read`, `cache:invalidate`, `telemetry:read`, `scenario:operate`, `admin`. (Ch 2, 2.10; Ch 6, 6.2)
- **RwLock**: a lock allowing many readers or one writer; guards the epoch snapshot. (Ch 3, 3.5.4)
- **rustls**: the Rust TLS library used instead of OpenSSL. (Ch 3, 3.7)

**S**


- **S3-FIFO**: an eviction policy with a small queue, a main queue and a ghost queue. Implemented as `FaithfulS3Fifo`; IMPLEMENTED, PARTIAL (open High finding G4A-02); used only if selected. (Ch 4, 4.9)
- **s-maxage**: the shared-cache lifetime directive; read before `max-age`. (Ch 4, 4.8)
- **SAN** (subject alternative name): the certificate field holding the member's identity; the code requires exactly one DNS name of the expected form. (Ch 3, 3.7; Ch 6, 6.4)
- **SBOM**: a software bill of materials; not provided. DEFERRED. (Ch 8, 8.3.7 and 8.12)
- **Scan resistance**: a one-time scan over many keys should not push out popular entries. (Ch 4, 4.9)
- **Scenario B**: the T2 experiment on a real gateway over loopback with a scripted origin and three arms; both capped arms are TRADE_OFF_FAILURE. (Ch 7, 7.5)
- **Sealed (evaluation)**: an evaluation whose seed set is kept secret until the analysis rule is frozen; the M6 run was executed once. (Ch 7, 7.4.4 and 7.6)
- **SEC-4A-01**: the rule that tenant-epoch overflow falls back to a global epoch bump so memory stays bounded. (Ch 4, 4.7)
- **Seed / seed sets**: a number that fixes a pseudo-random sequence; the Lab has disjoint tuning, development and sealed seed sets. (Ch 7, 7.2.3)
- **Sequence number** (`stream_seq`): a per-destination counter on invalidations so a receiver can notice a gap. (Ch 5, 5.4)
- **Shadow evaluation**: testing a candidate policy on a copy of traffic before changing anything; library only, with flaws F12 and F13. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Shard** (partition, bucket): one independent piece of the cache table with its own mutex; there are eight. (Ch 3, 3.5.2; Ch 4, 4.2)
- **SIEVE**: an eviction policy with a queue, one visited bit per entry and a moving hand; the default policy. (Ch 4, 4.9)
- **Sign-flip permutation test**: a test that randomly flips the sign of each paired difference to see how often chance gives an effect this large. (Ch 7, 7.3.3)
- **Single-flight**: at most one in-flight origin fetch per key; later requests wait for it. The core stampede defence; per gateway only. (Ch 2, 2.5.2; Ch 4, 4.4)
- **Snapshot** (epoch snapshot): an immutable copy of the tenant epoch map that readers use without blocking writers. (Ch 4, 4.7)
- **Space-Saving**: a top-k counting table that replaces its smallest entry when full; 64 slots in the library detector. (Ch 5, 5.12)
- **SplitMix64**: a small pseudo-random generator used for seeded streams and bootstrap resampling. (Ch 7, 7.2.3)
- **SSE** (server-sent events): a one-way HTTP stream from server to browser used by the dashboard feed. (Ch 2, 2.2.1; Ch 3, 3.4)
- **SSRF** (server-side request forgery): making the gateway fetch an address it should not; blocked by the upstream filter with known range gaps (G4B-15). (Ch 6, 6.8)
- **Stale**: older than its freshness lifetime; not automatically unusable. (Ch 1, 1.3.3; Ch 4, 4.8)
- **Stale-if-error**: serve an expired copy, labelled STALE, when the origin fails, within the window the origin allowed. (Ch 2, 2.6.2; Ch 4, 4.8)
- **Stale-while-revalidate** (SWR): serve an expired copy immediately inside the origin's window and refresh it in the background. (Ch 2, 2.6.2; Ch 4, 4.8)
- **State instance id**: an identity for the `KeyState` object; defends against "state removed and recreated during fetch". (Ch 4, 4.5)
- **Static origin cap**: a fixed limit on concurrent origin fetches; TESTED, and the feature that did show an effect. (Ch 5, 5.14; Ch 8, 8.6)
- **Supply chain**: the third-party dependencies in the build (288 locked packages) and the checks on them. (Ch 3, 3.13; Ch 6, 6.10)
- **Suspect** (node state): set when a real call to a peer fails; the node is skipped for routing and owed invalidations go to the repair ledger. (Ch 5, 5.2)
- **Sweeper**: the background task that reclaims expired and invalidated entries each tick (default 100 ms); it scans a shard under its lock (G4A-03). (Ch 3, 3.3; Ch 4, 4.7)

**T**


- **T1** (tier): simulation with a virtual clock, seeded randomness, real cache and policy code and a modelled origin. Labelled SIMULATED. (Ch 7, 7.1 and 7.2)
- **T2** (tier): real gateway processes on loopback with a scripted origin and an open-loop generator. Labelled MEASURED_LOOPBACK. (Ch 7, 7.5)
- **Tenant**: the customer namespace that is the first part of every key; in secure mode it comes only from the token. (Ch 6, 6.6)
- **Tenant epoch**: the per-tenant invalidation counter stamped on each entry. (Ch 4, 4.7; Ch 5, 5.4)
- **TLS 1.3**: the transport encryption version the cluster requires; a TLS 1.2 client is refused by test. (Ch 3, 3.7)
- **Tokio**: the Rust async runtime on which the gateway, Axum and the TLS libraries run. (Ch 3, 3.3)
- **Tombstone**: a leftover marker; in S3-FIFO, a removed key stays in a queue, so queues can grow (G4A-02). (Ch 4, 4.9)
- **TRADE_OFF_FAILURE**: the mechanical Scenario B verdict for both capped arms: origin load fell but client success fell too. (Ch 7, 7.5.2)
- **TTL** (time to live): how long a copy may be treated as correct; see freshness lifetime. (Ch 1, 1.1)

**U**


- **U-01**: an open Lab finding (a cancelled or partial Scenario B run can carry a completed mechanism verdict); the fix is sandbox only; owner-approved integration (DEC-001) is authorised and in progress, not committed. (Ch 7, 7.5.2; Ch 8, 8.8)
- **UNVERIFIED**: status label for something that could not be read or checked, with the reason stated. (Ch 8, 8.1)
- **Upstream filter**: the SSRF defence on the origin side: allowlist, IP filter and pinned address. (Ch 2, 2.10; Ch 6, 6.8)

**V**


- **Virtual clock**: an integer microsecond counter that the simulator advances; nothing waits for real time. (Ch 7, 7.2.1)
- **Vite / React**: the build tool and UI library for the dashboard, with plain CSS and hand-built SVG charts. (Ch 3, 3.11)
- **Vose alias method**: an O(1)-per-sample way to draw keys from a weighted (Zipf) distribution after one setup pass. (Ch 7, 7.2.3)

**W**


- **Waiter**: see leader. At most 1,000 waiters per key; beyond that the gateway answers 503. (Ch 2, 2.5.3; Ch 4, 4.4)
- **Watchdog**: a library check (`check_watchdog`) that reverts to the safe policy and forces mode `off` if the intelligence loop's last heartbeat is older than a timeout; not wired. PROTOTYPE ONLY. (Ch 5, 5.13)
- **Weighted rendezvous hashing**: rendezvous hashing where a node wins in proportion to its weight, using `-w / ln(u)`. DESIGNED BUT NOT IMPLEMENTED; the code multiplies by a constant 1.0. (Ch 5, 5.3)
- **Wiring**: whether a configuration key is read by the running gateway (`Wiring::Main`) or only validated (`Wiring::NotWired`). (Ch 2, 2.6.1 and 2.9.2)

**Z**


- **Zipf** (Zipfian): a popularity pattern in which a few items receive most requests; used by the Lab workload generators. (Ch 1, 1.3.2; Ch 7, 7.2.4)
