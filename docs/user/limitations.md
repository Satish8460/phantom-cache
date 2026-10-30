# Limitations

Stated plainly so that nothing here is mistaken for a claim.

- **Adaptive benefit is not established.** No result shows that Phantom Intelligence (`observe`, `advisory`, `canary`, `active`) improves latency, hit ratio or origin load. Treat those modes and the origin concurrency cap as experimental. The legacy `src/control/pie.rs` is a threshold stub and is not wired in.
- **No performance numbers are published with this release.** Numbers elsewhere in the repository are debug-build, loopback, single-host or simulated; none is a release-grade measurement.
- **Serving model.** GET is served from the local cache only; there is no cross-gateway read-through.
- **Cluster mode is experimental.** In the mTLS cluster configuration the local cluster node shares the gateway's HTTP serving cache, so invalidation frames received from peers purge it (`src/main.rs:332`); GET is still served from the local cache only. An anti-entropy reconciliation worker runs every 250 ms in cluster configurations (`src/main.rs:375`, `:383`); its effectiveness is not established. The insecure/partial cluster configuration (`PHANTOM_ALLOW_INSECURE_CLUSTER` without full mTLS material) has no peer addresses: no invalidation frame is sent or received, and DELETE returns a gateway timeout (`src/main.rs:381`). Membership is static. See "Cluster invalidation durability" below.
- **Staleness.** Entries expire by origin headers (`s-maxage`, `max-age`, `Expires`) or, when the origin gives none, a fixed 60 s default (`PHANTOM_DEFAULT_TTL_MS` is validated but not honored: `src/config.rs:115-116`, literal `60_000` at `src/api/server.rs:1405` and `:1688`). **Origin-supplied lifetimes have no gateway ceiling** (`src/core/freshness.rs:263-276`), and the hard stale cap `PHANTOM_MAX_STALE_MS` limits only how long an expired entry may be served stale (stale-while-revalidate / stale-if-error), not the freshness lifetime. No linearizability or bounded-staleness guarantee beyond that is claimed.
- **Dashboard.** Parts of the SSE stream and UI are synthetic when `PHANTOM_SIMULATION=true`; simulated values are labeled and are not measurements.
- **Shutdown.** No in-flight connection draining yet (see operate.md), and no drain of pending cluster invalidations (`src/main.rs:509-514`; see "Cluster invalidation durability" below).
- **Idle CPU.** The lazy-reclamation sweeper task is started with a shutdown channel whose sender is dropped immediately. An idle process was observed using more than one core of CPU (single runs: debug build about 6.5 CPU-seconds in about 5 s wall; release build 14.8 CPU-seconds in 10 s wall, Windows 11, loopback, no traffic). The probable cause (a `watch::changed()` that is always ready in the select loop) was identified by code reading and has not been isolated experimentally; only single runs exist (no repetitions or intervals).
- **Platforms.** Only Windows x86-64 has been exercised.
- **Not provided:** Prometheus `/metrics` export, TLS certificate hot reload, runtime configuration reload, token revocation, container image, SBOM, dependency advisory scan, code signing.

## Cluster invalidation durability

Status: code reading (2026-10-30), not reproduced by a test; no test kills or restarts a gateway that still owes invalidations.

**What a 204 from `DELETE /api/v1/cache/<key>` means in a cluster.** The issuing gateway purged its own cache first (`src/api/server.rs:1990`) and the canonical primary (or the acting primary on failover) acknowledged within a 150 ms leg (`src/api/server.rs:1995`, `src/dist/router.rs:1781-1789`). Every other member is invalidated asynchronously, with bounded retry and re-delivery by a repair worker, and that is guaranteed **only while the issuing gateway process stays alive** (and within the bounded repair ledger).

**Failure model.** Invalidations that a peer has not yet acknowledged exist only in the memory of the issuing gateway process. If that process crashes, is killed, or receives SIGTERM/Ctrl-C before delivery, they are lost: shutdown does not drain them (`src/main.rs:509-514`), and a restarted gateway starts with an empty ledger. Peers do not detect the restart. A peer that missed the invalidation keeps serving the old entry until it expires (origin freshness lifetime, or 60 s if the origin gave none; origin lifetimes have no gateway ceiling) or until another DELETE of that key reaches it. Out of scope for this statement: disk loss, and partitions beyond the repair bounds.

**So: a 204 is not proof that the object is gone from every gateway.** Do not rely on it for deletion-critical content (legal removal, privacy erasure, revoked content) without the measures below.

**Operator guidance.**
- For deletion-sensitive content have the origin send a short `s-maxage` (seconds, not hours), which bounds how long a missed invalidation can matter.
- Restart or upgrade one gateway at a time, and avoid restarting a gateway right after it issued DELETEs. Peer unavailability during a rolling deploy is exactly when queued invalidations are most likely to be pending.
- After any restart or crash of a gateway, re-issue DELETE for critical keys.
- DELETE every canonical key: the key is tenant + path + query, so each query-string variant is a separate key and needs its own DELETE.
- Verify critical removals by reading the key through each gateway, not by the DELETE status alone.

**Not provided:** durable invalidation log, shutdown drain, restart detection, a TTL ceiling. Remediation options under consideration (none approved or implemented) are in `docs/research/phantomcache_milestone_accountability_report.md` section 12.
