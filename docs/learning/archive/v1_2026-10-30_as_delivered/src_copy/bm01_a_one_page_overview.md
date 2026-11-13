# Appendix A: PhantomCache on one page

**What it is.** An in-memory HTTP caching gateway written in Rust (a research prototype). It sits between clients and one origin server, answers repeat `GET` requests from memory and sends one request to the origin when many clients ask for the same missing item. Several gateways can run as an experimental cluster. It is not a production system. (Chapter 1, section 1.5; Chapter 8, section 8.11.)

**The problem.** An origin is slow or fragile. Popular items expire together and cause a stampede. Deleted items must not come back. A cache that gets this wrong serves old data or crushes the origin. (Chapter 1, section 1.3.)

**The three pillars and their honest status.**

| Pillar | What exists | Status |
|---|---|---|
| Distributed systems engineering | Single-node engine: 8 shards, single-flight, generation and epoch fencing, byte accounting. Cluster: mTLS, exact-key `DELETE` fan-out, repair, anti-entropy | Engine IMPLEMENTED, TESTED (epoch fencing PARTIAL: G4A-01 race open); cluster IMPLEMENTED, TESTED on loopback only, labelled EXPERIMENTAL, effectiveness UNVERIFIED (durable invalidation and coordinated reads not done) |
| Workload intelligence | A static or AIMD origin concurrency cap; a library of detectors and a decision engine that no request calls | Cap IMPLEMENTED, TESTED (serving path only when a cap is configured in canary or active mode; default off), benefit not established; rest PROTOTYPE ONLY |
| Product, observability, demonstrability | Health and metrics routes, Phantom Lab, React dashboard with provenance labels | IMPLEMENTED, TESTED (automated checks only; visual acceptance UNVERIFIED, no human review) |

**How a request flows.**

1. `GET` is served from the local gateway's cache. A fresh entry is a `HIT`. Reads are never routed to another node.
2. On a miss, the first request becomes the leader and fetches the origin once. Other requests for that key wait and share the answer. The result is stored only if the key's generation and epochs did not change meanwhile.
3. `DELETE` purges locally, then waits up to 150 ms for the rendezvous-chosen primary to acknowledge. A `204` means that, not "gone everywhere". Other members are told in the background.

**Implemented and tested** (tests exercise them; they do not prove correctness, and each has open gaps listed in Chapter 8): coalescing, generation fencing, byte accounting, stale serving under a hard cap, JWT roles, SSRF filtering, mTLS identity, `DELETE` fan-out on loopback. Freshness (an RFC 9111 subset) and epoch fencing are PARTIAL.

**Not done.** The `PHANTOM_DEFAULT_TTL_MS` setting is NOT WIRED (the default lifetime is a literal 60 s). Owed invalidations are lost if the issuing gateway crashes (the durable fix is a sandbox patch, PROTOTYPE ONLY). Coordinated reads are DESIGNED BUT NOT IMPLEMENTED. Weights in rendezvous hashing are a constant. Open defects include the G4A-01 epoch race. No multi-host run, no release-profile suite, no human or external review.

**The contribution and its limits.** A tested caching core with fencing and an optional, off-by-default origin gate, plus a deterministic, preregistered experiment harness that drove the real cache code. Its sealed evaluation (M6) reported hypothesis H1 as NOT_SUPPORTED: a forced-adaptive controller did not beat a tuned static cap, in simulation. It is not AI, not linearizable, has no staleness bound across nodes, and makes no speed claim against other products.

**Status vocabulary.** IMPLEMENTED: code exists. TESTED: a named test exercises it; passing is not correctness. INDEPENDENTLY VERIFIED: a human or external reviewer checked it (nothing here qualifies; AI reviews are an "AI second look"). PARTIAL: some of it works. PROTOTYPE ONLY: exists, not wired or not integrated. DESIGNED BUT NOT IMPLEMENTED: on paper only. DEFERRED: postponed. BLOCKED: cannot proceed. UNVERIFIED: not checked.
