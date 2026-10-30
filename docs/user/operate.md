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

`GET /api/v1/metrics` (role `telemetry:read` when auth is on). Every field carries `src`: `measured`, `unavailable`, or others. An `unavailable` field is not zero. When the origin concurrency cap is configured and intelligence is `canary` or `active`, `metrics.origin_cap` reports `enforcing`, `cap`, `in_flight`, `dispatched`, `origin_cap_rejections`, `controller_steps`. A 503 from the cap has the same body and header as a shard-capacity 503; if `origin_cap_rejections` increases the cap caused it.

## Runbooks

**Origin outage.** Fresh entries keep serving. Expired entries may be served stale within `PHANTOM_MAX_STALE_MS` (default 24 h, hard cap); beyond it requests fail. Check the metrics origin section and `stale_cap_violations` (must be 0). Restore the origin; no gateway restart is needed.

**Cold start.** After a restart the cache is empty and every first request goes to the origin. Expect an origin load spike. The origin cap (`PHANTOM_ORIGIN_CAP_MAX`) only enforces in `canary`/`active` intelligence modes, which are experimental.

**High miss rate.** Compare `PHANTOM_MAX_CACHE_BYTES` with the working set; check the byte-budget and eviction counters in the metrics.

**Memory pressure.** Lower `PHANTOM_MAX_CACHE_BYTES`; restart (the budget is applied at start-up).

**Intelligence misbehaves.** Set `PHANTOM_INTELLIGENCE=off` and restart. This release has no authenticated runtime kill-switch endpoint.

**Certificate rotation.** Replace the PEM files and restart; certificates are read once at startup, and a restart drops open connections.

**Upgrade / rollback.** The cache is stateless: run `--check-config` with the new binary, stop the old process, start the new one with the same configuration. Rollback is starting the previous binary. Plan for a cold cache each time. In a cluster, restart one gateway at a time, avoid restarting a gateway that has just issued DELETEs, and re-issue DELETE for deletion-critical keys after a restart (the restarted gateway's own cache is empty, but its peers may still hold entries whose invalidation it owed). The cold cache protects only the restarted gateway's own reads.

**High idle CPU.** See [limitations.md](limitations.md) (sweeper task issue under review).

## Origin behaviour requirements

Because the cache key is tenant + path + query, configure origins so that a response never depends on a client-controlled request header (user id, forwarded-for, `Accept`, cookies). The gateway does not forward those headers (it sends a fixed `Accept: application/json`, the tenant as `x-tenant-id`, and a validated `x-request-id`), so an origin cannot personalise per user through the gateway unless `PHANTOM_FORWARD_UPSTREAM_AUTH=true`, in which case authenticated reads bypass the cache. Paths containing `.`/`..` segments, control characters or spaces are rejected with 400. See [security.md](security.md).
