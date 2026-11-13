# Operate

## Health and readiness

- `GET /health` returns `OK` (unchanged).
- `GET /health/live` returns 200 `{"status":"alive"}` while the process serves requests. It checks no dependencies.
- `GET /health/ready` returns 200 `{"status":"ready","checks":{...}}` only when all three hold: configuration validated, the ingress listener accepted a TCP connection from the process itself, and the process is not draining. Otherwise 503 `{"status":"not_ready","checks":{...}}` with the failing check. Readiness drops to 503 as soon as a shutdown signal is received.
- These endpoints are unauthenticated by design (load balancer probes) and expose only booleans.
- Origin reachability is not part of readiness: the gateway can serve cached content and stale content within the cap while the origin is down.

## Shutdown

SIGINT/SIGTERM (Ctrl-C on Windows) makes readiness fail, closes the ingress listener and exits 0. **In-flight request draining with a bounded deadline is not implemented yet**: open connections are dropped when the process exits (it needs a graceful-shutdown hook in `api::server::start_server`; see `docs/release/deferred_integration.md`). The cache is in memory; nothing is flushed and a restart starts cold. **Cluster warning:** pending peer invalidations are not drained either (`src/main.rs:509-514`). A DELETE that returned 204 but has not yet been acknowledged by every peer is held only in this process's memory and is lost on shutdown, crash or kill; a peer that missed it keeps serving the entry until it expires, and peers do not notice the restart. See "Cluster invalidation durability" in [limitations.md](limitations.md). Signal handling was not exercised end to end on the Windows development machine (see the release reproduction record).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Clean shutdown after a signal, or valid `--check-config` / `--help` / `--version` |
| 1 | Runtime failure (listener stopped, panic, runtime could not start) |
| 2 | Invalid configuration, usage error, unreadable referenced file |

A panic inside the server is caught at the top level and reported as exit 1 (the panic message is printed first).

## Metrics

`GET /api/v1/metrics` (role `telemetry:read` when auth is on). Every field carries `src`: `measured`, `unavailable`, or others. An `unavailable` field is not zero. `metrics.origin_static_cap` reports the static cap (`limit`, 0 = none; `in_flight`; `rejections`; measured). When the experimental adaptive controller is configured and intelligence is `canary` or `active`, `metrics.origin_cap` reports `enforcing`, `cap`, `in_flight`, `dispatched`, `origin_cap_rejections`, `controller_steps`. A 503 from the cap has the same body and header as a shard-capacity 503; if `origin_cap_rejections` increases the cap caused it.

**Cluster block (`metrics.cluster`, cluster mode only; otherwise `{src: "unavailable", reason: "cluster_not_configured"}`).** `local` is this gateway's own node (`node_id`, `state`); `peers` lists the OTHER members only (`node_id`, `state`, `idle_connections`, `active_connections`). Every value carries `src` (`measured`): `state` is `{src, v}` with `v` one of `Active`, `Suspect`, `Recovering`, `Failed`, `Offline`, `Degraded`, `Joining`; it is the ROUTING state machine, not a liveness verdict (a dead peer can sit in `Recovering` because nothing sets `Failed` when the health-probe worker is off). Use `reachability` for liveness: `{src: "inferred", v}` with `v` `reachable` (last exchange succeeded), `degraded` (1 or 2 consecutive failed attempts) or `unreachable` (3 or more), derived from the outcome of recent connection and frame attempts, together with `consecutive_failed_attempts`. `reachability` is observation only and does not change routing. `broadcaster` holds cumulative counts: `enqueued` (fan-out tasks accepted: one per DELETE), `sent` (every invalidation frame attempt, including retries), `retried` (attempts that were not the first for their delivery), `acked`, `dropped` (queue overflow only: a task that could not be enqueued) and `exhausted` (deliveries, one per key and destination, that gave up after all attempts or because the destination left the membership). On a healthy cluster `dropped` and `exhausted` stay 0; `exhausted > 0` means a peer did not receive an invalidation from this gateway and may still serve the entry. A DELETE answered `503` with `X-Cache-Error: PrimaryInvalidationBusy` means this gateway's connection pool to the primary was saturated: retry; the invalidation is still delivered in the background and the primary is not marked unhealthy.

**Outcome counters and `X-Cache-Source`.** Every successful cache GET response carries the additive header `X-Cache-Source`: `local` (served from this gateway's cache), `peer` (this gateway had no copy, asked the key's primary peer and stored the answer; no origin request), `origin` (fetched from the origin, including bypass requests), `coalesced` (waited for another request's fetch), `stale` (served stale-while-revalidate or stale-if-error). `X-Cache-Status` is unchanged (a peer fill is still `MISS`). `metrics.outcomes.responses.{local,peer,origin,coalesced,stale}` count the responses per gateway process since start (exactly one counter per response that carries the header; error responses are in none), and `metrics.outcomes.peer_reads_without_hit.{miss,failed,refused_format}` count peer reads that produced no peer response (the request then went to the origin and is counted as `origin`). All values are `measured`. The header is exposed to browsers through CORS.

## Local cluster for demos and checks

`scripts/cluster_launcher.mjs` starts a reproducible local cluster (default: 3 gateways with mutual TLS and a counting origin) and nothing else; it is a development tool, not part of the gateway. It needs Node 18+, `openssl` on the PATH and a built gateway executable. All state (test certificates valid for 30 days, member list, logs, process ids) lives in the directory you pass with `--dir`.

```
node scripts/cluster_launcher.mjs up --exe <gateway executable> --dir <work dir> [--cors-origin http://localhost:3000]
node scripts/cluster_launcher.mjs status --dir <work dir>
node scripts/cluster_launcher.mjs stop-node --dir <work dir> --node 3     # show a dead peer
node scripts/cluster_launcher.mjs start-node --dir <work dir> --node 3    # and its recovery
node scripts/cluster_launcher.mjs down --dir <work dir>
```

Defaults: gateways on `127.0.0.1:18185`, `18186`, `18187` (cluster listeners `19501` to `19503`), origin on `19099`; change them with `--http-port-base`, `--cluster-port-base`, `--origin-port`, `--nodes`, `--cluster-id`. `up` waits until every member is Active in node 1's view and prints the addresses. The origin answers any path with `text/plain` and an ETag; a path containing `/slow/` waits 400 ms, `/swr/` is cacheable for 1 s with stale-while-revalidate, `/__stats` reports its request count. To look at it in the Control Center, serve the UI (`npm run dev` in `ui/`), start the cluster with `--cors-origin <the UI origin>`, and open `<UI>/?gateway=http://127.0.0.1:18185#/cluster`.

The certificates are test certificates: do not reuse them. The check scripts `scripts/cluster_burst_check.mjs`, `scripts/cluster_outcomes_check.mjs`, `scripts/cluster_deadpeer_storm_check.mjs` and `scripts/cluster_peer_fill_staleness.mjs` start their own gateways and take a certificate directory (`--certs`); the launcher's `<work dir>/certs` has the layout they expect (`ca.crt`, `members.json`, `n1.crt`, `n1.p8.key`, ...), so they can use it after `down`.

## Control Center: Cluster screen

The Cluster screen (`#/cluster`) shows what THIS gateway reports in `metrics.cluster` and `metrics.outcomes`: the members with routing state, reachability (inferred) and connection counts; how its cache requests were answered in the words of `X-Cache-Source`; peer reads that produced no peer answer; and the purge fan-out counters. Every value carries a badge with the provenance the gateway stated (Real for measured, Derived with the qualifier inferred, Unavailable with the reason); a value the page cannot trust is withheld, never shown as zero. On a gateway that reports no cluster block (single node, or an older build) the page keeps the capability statement. It is one gateway's view: another gateway may see the cluster differently.

## Runbooks

**Origin outage.** Fresh entries keep serving. Expired entries may be served stale within `PHANTOM_MAX_STALE_MS` (default 24 h, hard cap); beyond it requests fail. Check the metrics origin section and `stale_cap_violations` (must be 0). Restore the origin; no gateway restart is needed.

**Cold start.** After a restart the cache is empty and every first request goes to the origin. Expect an origin load spike. Set `PHANTOM_ORIGIN_CAP_MAX` to bound concurrent origin fetches: it is a static cap enforced in every intelligence mode (503 for a request that finds it full; see `metrics.origin_static_cap`). `PHANTOM_INTELLIGENCE=canary` or `active` is rejected at startup unless `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true` is set.

**High miss rate.** Compare `PHANTOM_MAX_CACHE_BYTES` with the working set; check the byte-budget and eviction counters in the metrics.

**Memory pressure.** Lower `PHANTOM_MAX_CACHE_BYTES`; restart (the budget is applied at start-up).

**Intelligence misbehaves.** Set `PHANTOM_INTELLIGENCE=off` and restart. This release has no authenticated runtime kill-switch endpoint.

**Certificate rotation.** Replace the PEM files and restart; certificates are read once at startup, and a restart drops open connections.

**Upgrade / rollback.** The cache is stateless: run `--check-config` with the new binary, stop the old process, start the new one with the same configuration. Rollback is starting the previous binary. Plan for a cold cache each time. In a cluster, restart one gateway at a time, avoid restarting a gateway that has just issued DELETEs, and re-issue DELETE for deletion-critical keys after a restart (the restarted gateway's own cache is empty, but its peers may still hold entries whose invalidation it owed). The cold cache protects only the restarted gateway's own reads.

**High idle CPU.** See [limitations.md](limitations.md) (sweeper task issue under review).

## Origin behaviour requirements

Because the cache key is tenant + path + query, configure origins so that a response never depends on a client-controlled request header (user id, forwarded-for, `Accept`, cookies). The gateway does not forward those headers (it sends a fixed `Accept: application/json`, the tenant as `x-tenant-id`, and a validated `x-request-id`), so an origin cannot personalise per user through the gateway unless `PHANTOM_FORWARD_UPSTREAM_AUTH=true`, in which case authenticated reads bypass the cache. Paths containing `.`/`..` segments, control characters or spaces are rejected with 400. See [security.md](security.md).
