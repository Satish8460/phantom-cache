# Appendix C: Technology and algorithm reference

Use this appendix to look something up fast. It adds no new claims. Each row repeats what a chapter says and points to it. Status labels are the chapters' labels: IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, NOT WIRED. A TESTED label means a named test exercises the thing. It does not mean the thing is correct. Nothing here is INDEPENDENTLY VERIFIED by a human. Where a row names a symbol, it is written `path::symbol`. Complexity entries are the chapters' own reasoning unless a measurement is cited, and no performance number in this appendix is a measurement.

## C.1 Technologies

| Technology | What it is, and why it is here | Where in the code path (`path::symbol`) | Status | Chapter |
|---|---|---|---|---|
| Rust | Memory-safe systems language. Chosen for safety and no garbage collector. The only `unsafe` found in `src/` is Windows FFI in the Lab API | all of `src/`; `src/api/lab_api.rs` (the `unsafe` blocks) | IMPLEMENTED | Ch 3, section 3.2; Ch 6, section 6.10 |
| Tokio | Async runtime: timers, channels, TCP, spawned tasks. The leader fetch runs as a spawned task | `src/main.rs` (`tokio::runtime::Builder::new_multi_thread`) | IMPLEMENTED | Ch 3, section 3.3 |
| Axum (hyper and tower arrive transitively) | HTTP routing, extractors, server-sent events. No WebSocket use found | `src/api/server.rs::create_app_with_lab` | IMPLEMENTED, TESTED | Ch 3, section 3.4 |
| tower-http CORS layer | Lets the browser UI on port 3000 call the gateway. The `fs` feature is declared but unused | `src/api/server.rs::create_app_with_lab` (layer) | IMPLEMENTED | Ch 3, sections 3.1 and 3.4 |
| `std::sync::Mutex` per shard | One lock per bucket of keys, so unrelated keys do not wait | `src/core/cache.rs::PartitionedCache`, `CacheShard` | IMPLEMENTED, TESTED | Ch 3, section 3.5.2; Ch 4, section 4.2 |
| `RwLock` epoch snapshot | Readers see the tenant epochs without the exclusive mutex | `src/core/cache.rs::PartitionedCache::epoch_snapshot`, `get_effective_epochs` | IMPLEMENTED, TESTED | Ch 3, section 3.5.4; Ch 4, section 4.7 |
| Atomics | Lock-free counters and histogram buckets (relaxed ordering) | `src/intel/telemetry.rs::AtomicLogLinearHistogram`; `src/intel/diagnostics.rs` | IMPLEMENTED, TESTED | Ch 3, section 3.5.3 |
| `DefaultHasher` (SipHash) | Picks the shard and scores nodes in rendezvous hashing. The algorithm is unspecified by the standard library | `src/core/cache.rs` (shard index); `src/dist/router.rs::calculate_hash` | IMPLEMENTED; portability risk | Ch 4, section 4.2; Ch 5, section 5.3 |
| rustls and tokio-rustls | TLS 1.3 for cluster mTLS, origin HTTPS and ingress. Pure Rust, no OpenSSL | `src/security/cluster.rs::ClusterTlsConfig` | IMPLEMENTED, TESTED | Ch 3, section 3.7; Ch 5, section 5.2 |
| `ring` | SHA-256 for certificate fingerprints, the anti-entropy digest and lab hashes | `src/dist/anti_entropy.rs::InvalidationRingBuffer::compute_digest` | IMPLEMENTED | Ch 3, section 3.1 |
| `jsonwebtoken` | Verifies HS256 tokens: signature, expiry, issuer, audience | `src/security/jwt.rs::JwtValidator::validate` | IMPLEMENTED, TESTED | Ch 3, section 3.8; Ch 6, section 6.2 |
| serde and serde_json | JSON for API bodies, JWT claims and lab artifacts | `src/api/server.rs`; `src/security/jwt.rs`; `src/lab/` | IMPLEMENTED | Ch 3, section 3.9 |
| PhantomMesh frames | Custom binary protocol: magic `PHAN`, 28-byte header, payload up to 1 MiB, key up to 4,096 bytes | `src/dist/frame.rs` | IMPLEMENTED, TESTED | Ch 3, section 3.9; Ch 5, section 5.2 |
| Upstream policy, DNS pinning, IP filter | Stops the origin fetch reaching private or metadata addresses | `src/security/upstream.rs::resolve_and_pin_origin`, `PreConnectIpFilter` | IMPLEMENTED, TESTED | Ch 6, section 6.8 |
| Static cluster membership file | Fixed member list with certificate fingerprints, read once at startup | `src/security/cluster.rs::load_membership_from_file` | IMPLEMENTED, TESTED | Ch 5, section 5.2 |
| React, Vite, hand-built SVG | The dashboard and Lab screens. A separate dev server, not served by the gateway | `ui/src/App.jsx` | IMPLEMENTED, TESTED (automated checks only) | Ch 3, section 3.11; Ch 7, section 7.7 |
| Test tools: proptest, reqwest, rcgen, hdrhistogram | Property tests, real HTTP in tests, in-memory certificates, bench latency recording. `criterion` is declared but unused | `tests/`; `benches/` | IMPLEMENTED | Ch 3, section 3.12 |

## C.2 Algorithms and mechanisms

| Mechanism and what it does | Cost or complexity | Where (`path::symbol`) | Status; failure mode in brief | Chapter |
|---|---|---|---|---|
| Sharded mutexes: hash the key, lock one of 8 buckets | Lookup O(key length) plus expected O(1); at most 8-way parallelism; contention not measured | `src/core/cache.rs::PartitionedCache::new` (8 in `main.rs`) | IMPLEMENTED, TESTED; a hot key cannot spread, long critical sections block a shard | Ch 4, section 4.2 |
| Single-flight coalescing: first miss leads, others wait and share | O(1) per request; at most 1,000 waiters per key | `src/core/cache.rs::begin_fetch_atomic`; `src/core/coalesce.rs::SharedOp` | IMPLEMENTED, TESTED; waiters share the leader's fate, per gateway only | Ch 4, section 4.4 |
| Generation fencing: store a fetch result only if the key's generation is unchanged | O(1) under the shard lock | `src/core/cache.rs::finalize_fetch`, `delete_impl` | IMPLEMENTED, TESTED; tested on one interleaving, no model checker | Ch 4, section 4.5 |
| Epoch fencing: tenant and global version stamps; invalidate by raising the epoch | Tenant bump O(T), T at most 10,000 | `src/core/cache.rs::try_invalidate_tenant`, `get_effective_epochs` | IMPLEMENTED, PARTIAL; G4A-01 race (fix not integrated), G4A-08, G4A-17 | Ch 4, section 4.7 |
| Epoch overflow fallback: a new tenant on a full map raises the global epoch | Flushes the whole node, O(T) | `src/core/cache.rs::MAX_TENANT_EPOCHS` | IMPLEMENTED, TESTED; a peer can trigger it (G4A-12) | Ch 4, section 4.7 |
| TTL and freshness: lifetime from `s-maxage`, `max-age`, `Expires`, else 60 s | O(1) per response | `src/core/freshness.rs::evaluate_response_freshness`; literal `60_000` in `src/api/server.rs` | IMPLEMENTED, PARTIAL; `PHANTOM_DEFAULT_TTL_MS` is NOT WIRED, no ceiling on origin lifetimes | Ch 4, section 4.8 |
| Stale serving and hard cap: serve expired data only inside origin-declared windows, capped at `PHANTOM_MAX_STALE_MS` | O(1) per check | `src/core/cache.rs::check_stale_serving` | IMPLEMENTED, TESTED; cap counter is weak (G4A-18), cap bounds stale window only | Ch 4, section 4.8 |
| Revalidation with 304: refresh a stored entry's lifetime | O(1) | `src/core/cache.rs::complete_not_modified` | IMPLEMENTED, TESTED; validators dropped early by the sweeper (G4A-16) | Ch 4, section 4.8 |
| Lazy sweeper: reclaim stale and expired entries in the background | Scans each shard in full under its lock every tick | `src/core/cache.rs::spawn_sweeper_task`, `sweep_lazy_reclamation` | IMPLEMENTED, PARTIAL; lock held for the whole scan (G4A-03) | Ch 4, section 4.7 |
| SIEVE (default): queue plus visited bit plus a moving hand | Hit O(1); eviction amortized O(1) with no pinned keys | `src/core/eviction.rs::FaithfulSieve` | IMPLEMENTED, TESTED (weakly); pinned keys make a scan costly (G4A-20) | Ch 4, section 4.9 |
| LRU: recency list, victim at the back | O(1); worst case O(p) for p pinned keys | `src/core/eviction.rs::FaithfulLru` | IMPLEMENTED, TESTED (weakly); every hit rewrites the list | Ch 4, section 4.9 |
| S3-FIFO: small, main and ghost queues | Amortized O(1) (our reasoning) | `src/core/eviction.rs::FaithfulS3Fifo` | IMPLEMENTED, PARTIAL (defect G4A-02 OPEN); queues can outgrow live keys, not the default | Ch 4, section 4.9 |
| Byte accounting: running logical byte count per shard, with an audit | O(1) per update; audit walks all entries | `src/core/cache.rs::calculate_state_bytes`, `audit_bytes` | IMPLEMENTED, TESTED (consistency only); drift on a panic path (G4A-13), uncounted structures | Ch 4, section 4.6 |
| Origin concurrency gate: a permit counter, released on drop | O(1) atomic | `src/intelligence/executor.rs::OriginGate`, `origin_cap_enforcing` | IMPLEMENTED; enforced only in canary or active with a cap, uncapped by default | Ch 4, section 4.4; Ch 5, section 5.13 |
| AIMD cap controller: multiply by 0.70 on a bad window, else add 1 | O(1) per observation; steps once per window | `src/intelligence/candidates.rs::OriginConcurrencyCapController` | IMPLEMENTED, TESTED (arithmetic and cap unit tests); on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT established, behaved as a static cap | Ch 5, section 5.13; Ch 7, section 7.6 |
| Log-linear latency histogram: 2,688 buckets, bit tricks | O(1) record; design error at most 0.39 percent | `src/intel/telemetry.rs::AtomicLogLinearHistogram` | IMPLEMENTED, TESTED (recording and bucket arithmetic only); it is fed but has no production percentile reader | Ch 5, section 5.11 |
| Count-Min sketch (4 by 1,024) in the heavy-hitter detector | O(d) per item, overestimates only | `src/intelligence/signals.rs::HeavyHitterDetector` | PROTOTYPE ONLY (library); not wired. The older `src/intel/sketch.rs::CountMinSketch` is used only by a unit test | Ch 5, section 5.12 |
| Space-Saving top-k (k = 64) | O(k) per miss | `src/intelligence/signals.rs::HeavyHitterDetector` | PROTOTYPE ONLY (library); metrics report hot keys as unavailable | Ch 5, section 5.12 |
| EWMA, CUSUM burst, Zipf estimate | O(1) per window; Zipf over the top 50 | `src/intelligence/signals.rs` | PROTOTYPE ONLY (library); working-set size assumes 1 KiB per object | Ch 5, section 5.12 |
| Shadow evaluation and decision engine | Replay on a sampled stream (one in R keys) | `src/intelligence/shadow.rs::ShadowEvaluator`; `src/intelligence/engine.rs::PieDecisionEngine` | IMPLEMENTED as library, not wired; unsound intervals (F12, F13) | Ch 5, section 5.13 |
| Rendezvous hashing: highest score of (key, node) wins, equal weights | O(N) per key for N members | `src/dist/router.rs::RendezvousRouter::route_key` (`capacity_weight = 1.0`) | IMPLEMENTED; weighted version DESIGNED BUT NOT IMPLEMENTED; routes `DELETE` only, `GET` is local | Ch 5, section 5.3 |
| Invalidation fan-out: per-peer queue of 256, up to four attempts | Bounded queues and attempts | `src/dist/router.rs::invalidate_key_across_cluster`, `fanout_targets` | IMPLEMENTED, TESTED; 204 is not a cluster-wide acknowledgement | Ch 5, section 5.5 |
| Repair ledger: bounded record of owed invalidations | 1,024 entries per peer, collapses to a coarser flush | `src/dist/router.rs::RepairLedger` | IMPLEMENTED, TESTED; memory only, lost if the issuer dies | Ch 5, sections 5.5 and 5.7 |
| Anti-entropy: ring of sent records, sequence gaps, pull replay | Ring of 4,096 records; digest is SHA-256 | `src/dist/anti_entropy.rs::InvalidationRingBuffer`, `AntiEntropyTracker`; `src/dist/router.rs::reconcile_peer_anti_entropy` | IMPLEMENTED, TESTED; effectiveness beyond loopback UNVERIFIED | Ch 5, section 5.5 |
| mTLS admission: TLS 1.3, cluster CA, one SAN, fingerprint pinning, originator match | Per connection and per frame check | `src/security/cluster.rs::ClusterTlsConfig`, `validate_peer`; `src/dist/node.rs::start_tcp_service` | IMPLEMENTED, TESTED; no revocation trigger (G4B-07), members are trusted (G4B-05) | Ch 5, section 5.2; Ch 6, section 6.4 |
| HS256 JWT with roles and tenant claim | One signature check per request | `src/security/jwt.rs::JwtValidator::validate`; `src/api/server.rs::authenticate_request` | IMPLEMENTED, TESTED; no revocation, no `nbf`, shared secret can mint any token | Ch 6, section 6.2 |
| SSRF pre-connect filter with DNS pinning | One resolve and filter per origin fetch | `src/security/upstream.rs::resolve_and_pin_origin` | IMPLEMENTED, TESTED; range gaps open (G4B-15) | Ch 6, section 6.8 |
| Discrete-event simulation (T1): virtual time, event heap with sequence tie-break | O(log n) per event (heap) | `src/lab/t1_sim.rs::run_cell`; `src/lab/events.rs::EventQueue` | IMPLEMENTED, TESTED (determinism on one machine); a model of the origin, not the HTTP gateway | Ch 7, section 7.2 |
| Seeded random streams and alias sampling | One stream per purpose, seeded | `src/lab/rng.rs::MasterRng`, `AliasSampler` | IMPLEMENTED, TESTED | Ch 7, section 7.2 |
| BCa bootstrap, sign-flip permutation test, Holm correction | B = 10,000 resamples; exact sign-flip for n at most 16 | `src/lab/analysis.rs::bca_ci_mean`, `sign_flip_p_value`, `HolmBonferroni` | IMPLEMENTED, TESTED on synthetic data; not cross-checked against a reference | Ch 7, section 7.3 |

## C.3 How to use the tables in a defence

1. If a judge names a mechanism, find its row, say the status label first, then the failure mode. Do not state a label stronger than the one here.
2. Every complexity statement here is arithmetic or code reading. If asked for a number, say there is no release-profile measurement, and point to Chapter 6, section 6.13 for the few measurements that exist and their limits.
3. If a row says "library" or "not wired", say so before the judge asks. The honest answer is the strong one.
