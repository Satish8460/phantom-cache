# Architecture Investigation — What the Running Binary Actually Provides

**Date:** 2026-10-28. **Mode:** read-only. HEAD `456bb25`. No code was changed.
**Method:** a delegated agent traced code paths (report preserved in session); I then re-verified the load-bearing claims myself (marked **[V]** with the lines I read). Everything else is **[AGENT]**: reported by the agent from code, not independently re-read. Nothing was executed except the earlier test run.

## 1. Headline findings

| # | Finding | Status | Evidence |
|---|---|---|---|
| A1 | **Peer invalidations do not reach the cache that serves HTTP GETs.** The HTTP cache is `PartitionedCache::new(8)` in `main.rs:65`, stored in `ApiState.cache` (`main.rs:300`). Each cluster `VirtualNode::new` (`node.rs:42`) creates its **own** `PartitionedCache::new(4)`, and the cluster listener's invalidation handler purges `node.cache` (`node.rs:485/498/504`). `main.rs` never links them (grep of `.cache` in `main.rs`: only line 300). | **CONFIRMED [V]**; scope limited to `src/` — tests may wire differently (not checked) | `main.rs:65,237,253,300`; `node.rs:42,485` |
| A2 | **Consequence:** after `DELETE` returns 204 on gateway A, gateway B's HTTP cache is never purged by any cluster message; B serves the old entry until TTL (60 s) or eviction. | **CONFIRMED by inference from A1 [V]** (not executed) | A1 + GET path uses only `state.cache` |
| A3 | **GET is local-only.** `get_cache_handler` uses `state.cache` then the origin; `state.nodes` is used for dashboard and `/api/control` only; `forward_key_over_tcp`/`route_key`/`get_replicas` are called only inside `router.rs` and tests. | **CONFIRMED [V]** (grep, earlier) | `server.rs:357-770`; `router.rs:445,488,514` |
| A4 | **The cluster listener's `GetRequest` handler fabricates data**: on a miss it returns `"MISS val_node_<id>"` and caches that string for 60 s; it never contacts the origin. Nobody in production sends `GetRequest`. | **[AGENT]** `node.rs:385-441` | |
| A5 | **`reconcile_peer_anti_entropy` has no production caller**, and no indirect trigger exists (heartbeats are empty; gap queue has no consumer; no timer in `main.rs`). SEC-4C-06 ("no caller in the cluster runtime") was "remediated" by adding the function and a test that calls it. | **CONFIRMED for the function [V]**; indirect-trigger search **[AGENT]** | grep `reconcile` → definition only; `docs/security/stage-4c-audit-report.md:65-69` |
| A6 | **Outbound invalidations are never recorded by their sender.** `ring_buffer.push` has one call site: on receipt (`node.rs:475`). A lagging replica therefore cannot be replayed what the origin gateway sent. | **CONFIRMED [V]** (grep earlier: one `push`) | `node.rs:475` |
| A7 | **`stream_seq` is one counter shared across targets and retries** (`router.rs:142` inside the retry loop; `:615`). Each receiver sees a subset of values, so gap detection is dominated by false positives; each retry consumes a new number. | **CONFIRMED [V]** | `router.rs:142,615,223` |
| A8 | Replay/InSync design is unsound with multiple originators: ring indexed by raw seq mixes originators' counters; digests of different rings rarely match. | **[AGENT]**, inferred, not run | `anti_entropy.rs:112`, `node.rs:554-555` |
| A9 | **Missed replica invalidations are lost without record**: `try_send` failure only increments a counter; retry exhaustion just ends the task; counters are read nowhere outside `router.rs`. The 1,024-slot queue is drained immediately into unbounded spawned tasks that wait on a 16-permit semaphore, so memory is not bounded as documented. | **[AGENT]** counters/queue partly **[V]** (`router.rs:61,194-201,110`) | |
| A10 | **Suspect is effectively sticky in production**; Recovering is set only inside reconcile; heartbeats never change `NodeState`; `get_replicas` excludes Suspect nodes so they are never re-targeted. A 150 ms deadline includes a cold TLS handshake (inferred). | **[AGENT]** | `router.rs:318-422,488,745,775` |
| A11 | **DELETE ordering differs from the docs**: the local purge happens *after* the cluster round trip and only on success (and as "hygiene" on timeout); a primary rejection leaves the local entry and returns 5xx. Docs/diagram put the local purge first. | **CONFIRMED [V]** | `server.rs:830-832,871-873`; `docs/handoff/01:166` |
| A12 | **Staleness bound = TTL only**, and the only TTL used is 60,000 ms (`server.rs:638`, `node.rs:413`); `Cache-Control: max-age` is not parsed. | **[AGENT]** (checked all `ttl_ms:` literals) | |
| A13 | Insecure-cluster mode builds a router over addressless nodes; every DELETE then ends 504 and marks nodes Suspect. | **[AGENT]** | `main.rs:265`, `router.rs:318-321,745` |

Smaller **[AGENT]** items (unverified by me): `InvalidateRequest` does not check the originator id against the peer certificate (the spoof check exists only for sync requests); no per-session idle timeout or read timeout on established cluster sessions; loopback plaintext cluster fallback contradicts the handoff's "plaintext rejected"; `DefaultHasher` for HRW is not guaranteed stable across Rust versions; `detected_gaps` is unbounded and never drained.
Not found by the agent: deadlocks or `std::sync::Mutex` guards held across `.await` in `pool.rs`/`router.rs`/`session.rs`; network-reachable `unwrap` in `frame.rs`/`security/`.

## 2. What the system actually is (as built)
A single-gateway HTTP cache (explicit REST API, local `PartitionedCache`, 60 s TTL) with single-flight coalescing and generation fencing, **plus** a cluster-protocol subsystem (mTLS, framing, pooling, invalidation/anti-entropy codecs) that is well tested in isolation but **not wired to the serving path**. The "distributed cache" is, at present, a protocol library and a demo.
Intended architecture per `docs/security/phase-4-design.md`: *local cache plus distributed invalidation* ("an ephemeral caching tier"). The earlier demo-lab roadmap's Phase 6 ("wire RendezvousRouter into the Axum gateway") was never implemented and is silently replaced in the handoff roadmap by "Adaptive / Predictive Caching Research" (`docs/handoff/05:77`). Phase numbers collide across three documents.

## 3. Handoff claims not supported by code
| Claim | Source | Finding |
|---|---|---|
| Distributed invalidation keeps caches fresh; self-healing anti-entropy | `handoff/01:19` | Not wired (A1, A5) |
| Phase 4 "accepted … zero open P0 blockers" | `handoff/00`, `05:13`, `08:16` | Acceptance rests on tests that call functions the product never calls (A5) |
| Local purge precedes cluster invalidation | `handoff/01:166` | Reversed (A11) |
| Plaintext cluster connections rejected | `handoff/03:12` | Loopback plaintext fallback exists [AGENT] |
| 15 s heartbeat → Suspect; periodic sync probe | `handoff/03:99,157` | No such code [AGENT] |
| Metric `primary_invalidation_timeouts`; endpoint `/api/v1/diagnostics` | `handoff/05:59` | Not in `src/` [AGENT] |
| 242 tests pass | `handoff/04` | Reproduced: 242 passed. (The agent counted 226 test attributes; difference unexplained — likely macro/count convention; not investigated.) |
`handoff/06` states that passing tests are not proof of architecture; the Phase 4 acceptance contradicts that rule.

## 4. Classification
**Confirmed defects relative to the project's own documentation:** A1/A2, A5, A6, A7, A9, A10, A11, plus the handoff-vs-code mismatches in §3.
**Design choices (not defects, but must be stated honestly):** local-only GET; fixed 60 s TTL; synthetic dashboard fields; no read forwarding.
**Unverified:** whether tests wire node caches to the HTTP cache; the transcripts' reasoning for leaving GET unrouted (grep over very long JSONL lines failed with `grep`; ripgrep returned nothing useful); DER/SAN parser and `upstream.rs`; the cold-TLS Suspect scenario.

## 5. Implications for the product and the research
1. **The first product decision is architectural:** (a) *local caches + working invalidation fan-out* (make peer invalidations purge the serving cache, record outbound history, trigger reconciliation, per-peer sequences); (b) *coordinated reads* (route GETs by HRW; a real distributed cache); or (c) *ship single-node honestly* and move the cluster layer behind an experimental flag until wired. Each has different guarantees and research implications. This is a decision for the owners, informed by measurements; it is not made here.
2. **No distributed claim (staleness bound, recovery, T_stale) can be made today.** The measurable bound is "≤ TTL (60 s) per gateway cache, with no cross-gateway invalidation".
3. **The origin-aware controller is independent of this** — it operates per gateway on the origin fetch path and does not need the cluster. That favors starting the research controller on the single-node path while the distributed track is repaired.
4. Claim C6 (staleness bound) in `claims_ledger.md` is **not currently achievable**; it must be rewritten after the architecture decision.
5. Shared work: a fault-injectable multi-gateway harness serves both the repair (regression tests that would have caught A1) and the research (Tier 1/2 experiments).
