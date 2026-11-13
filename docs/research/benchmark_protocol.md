# Benchmark protocol (lane L5)

Status: protocol, written before the full run. Implementation: `scripts/benchmark_scenario.mjs`, `scripts/lib/bench_stats.mjs`, `scripts/lib/scenario_common.mjs`. Rules: `CLAUDE.md` section 4 (evidence standards). Everything below describes what the script does; the numbers of a run are in that run's `benchmark.summary.md` and are reproduced in the lane handover, not here.

## 1. Question and scope

How does a PhantomCache gateway behave, on ONE Windows host over loopback, when many clients request a Zipf-popular set of real objects, compared with the clients going straight to the origin? Metrics: success rate, throughput, latency percentiles, origin load, hit ratio by `X-Cache-Source`, gateway memory, integrity.

Not in scope: network latency, a remote or slow origin, TLS on the client path, authentication, mixed read/write traffic, invalidation under load, PIE/intelligence modes (all runs use the default `PHANTOM_INTELLIGENCE=off`), any comparison with another cache product.

## 2. Targets (what is compared)

| Label | What the clients talk to | Notes |
|---|---|---|
| `direct` | the origin process itself (`/obj/<i>.<ext>`) | the "no cache" baseline: the same bytes without a gateway |
| `single` | one gateway (`/api/v1/cache/obj/<i>.<ext>`) in front of the origin | default configuration, only `ORIGIN_URL` and `PHANTOM_BIND_ADDR` set |
| `cluster` | three gateways with mutual TLS between them, requests spread round-robin over the three HTTP ports | clients are not key-aware; a request may reach a node that is not the key's primary (then a peer read may happen) |

The gateway binary is a RELEASE build (`D:\ClaudeTemp\audit\phantom_rev6.exe` in the L5 runs). The script records the binary's sha256 and `--version` line; it cannot verify the build profile from the file, so "release" is the programme brief's statement (assumption, not checked by the script).

## 3. Hardware and software capture

The script writes `benchmark.meta.json` with: OS name and build, CPU model, logical CPU count, total and free memory, Node version, a short hash of the host name, gateway path, sha256 and version line, all arguments and the catalog statistics. These are captured by the script (`os` module), not typed in.

## 4. Workload (real bytes, deterministic)

The origin runs as a separate Node process (`node scripts/lib/scenario_common.mjs origin --port N`) and serves a catalog of `--objects` (default 1,000) distinct objects, generated deterministically from the object index:

| Type | Share | Size | Content |
|---|---|---|---|
| `application/json` | 72 % | 1 to 6 KB | seeded pseudo-random "messages" |
| `text/plain` | 16 % | 2 to 8 KB | seeded word lines |
| `image/png` (small) | 10 % | 30 to 60 KB | valid 8-bit RGB PNG (IHDR/IDAT/IEND, correct CRCs), gradient plus seeded grain |
| `image/png` (large) | 2 % | 300 to 900 KB | same generator |

Catalog total about 19.9 MB, mean about 20 KB per object (the script prints the exact figures for its run). Every response carries `Cache-Control: public, max-age=3600` and a strong ETag, so no entry expires during a run and the default 256 MiB byte budget is far above the 19.9 MB working set: this benchmark measures a cache that is NOT under eviction pressure (eviction is covered by `scripts/eviction_scenario.mjs`). The 1 MiB per-entry limit is not exercised (largest object about 900 KB).

Request streams: popularity is Zipf with exponent `--zipf` (default 1.0) over the 1,000 objects, rank-to-object mapping a fixed seeded permutation (so object size is uncorrelated with popularity). The stream for a cell and repetition is a pure function of a seed (`--seed` + 1000 x repetition + cell index), so the three targets see the identical sequence in the same repetition (paired design).

Cells: N in {1,000; 5,000; 10,000} requests x concurrency C in {50; 200; 500} = 9 cells.

Load generator: one Node 24 process, closed loop (C workers, each issues the next request when the previous one is complete), `node:http` with keep-alive agents, no retries. Every response body is hashed (sha256) and compared with the sha256, byte count and Content-Type that the ORIGIN itself reported in its manifest (`/__manifest`, computed by the origin process). A non-200 status, an error, a timeout (30 s) or any mismatch counts as a failed request; the run is reported with every failure sample, and the script exits 1.

## 5. Procedure (per target, per cell, per repetition)

1. Start the target fresh (new gateway process or processes: empty cache). `cluster`: wait until node 1 reports all three members Active.
2. Warm-up: `--warmup` (default 1,000) requests from a different Zipf stream (different seed), same concurrency, not measured. Direct also does it, for symmetry. The warm-up primes connections, JIT and part of the cache; it does NOT fully warm the cache, so the measured hit ratio includes the remaining first-touch misses of a cold-started cache. This is a deliberate definition ("cold start plus a 1,000-request warm-up"), not a steady-state measurement.
3. Reset the origin's request counter, read gateway RSS (`metrics.process.rss_bytes`, as the gateway reports it, label measured by the gateway), run the N measured requests, read RSS again, read the origin's counter.
4. Stop the target.

Order: repetitions are the outer loop, then cells, then targets, so a partial run still has balanced repetitions and slow drift of the host spreads over all targets. Repetitions: at least 10 (`--reps 10`).

## 6. Metrics

| Metric | Definition | Source |
|---|---|---|
| success rate | 200 responses with verified bytes / N | client |
| integrity failures | wrong sha256, byte count or Content-Type | client vs origin manifest |
| throughput | verified responses / measured wall time of the run (req/s) | client clock (`process.hrtime`) |
| p50, p95, p99 | nearest-rank percentiles of the per-request latency (request start to last body byte, ms) over the N requests of the run | client clock |
| origin requests | requests the ORIGIN process received during the measured run | origin counter (not the gateway's) |
| hit ratio by `X-Cache-Source` | share of the N responses with source `local`, `peer`, `origin`, `coalesced`, `stale` | response header (gateway-reported) |
| gateway RSS before/after | `metrics.process.rss_bytes`, summed over the gateways of the target | gateway metrics |

"Hit ratio" is reported as the per-source shares and as origin requests per 100 client requests (the cache offload, from the origin's own counter). `coalesced` responses did not cause their own origin request but were not served from a stored copy either; they are reported separately, not added to hits.

## 7. Statistics

The replication unit is one repetition of one (target, cell). For each metric the script reports the mean over the repetitions and a two-sided 95 % Student-t confidence interval of the mean: `half = t(0.975, n-1) * s / sqrt(n)` with `s` the sample standard deviation; n = 10 gives t = 2.262. The latency percentiles are computed per run and then averaged over repetitions (the CI is over the per-run percentiles; the percentiles of pooled requests are not reported). For gateway versus direct the script also reports the mean and CI of the PAIRED per-repetition ratio (same repetition, same request sequence): throughput ratio, p50 ratio, p99 ratio. A ratio whose CI contains 1 is not distinguishable from "no difference" at this n; the script does not claim significance tests.

Assumptions (not verified): repetitions are independent and the per-repetition metric is roughly normal. Neither is guaranteed (same host, same session, thermal and scheduler drift, a heavy-tailed latency distribution). With few repetitions or a skewed metric the interval may be too narrow or too wide. If n < 2 the interval is printed as absent, never invented. The p99 of a 1,000-request run rests on the 10 slowest requests and is noisy.

## 8. Limitations (must accompany every number from this protocol)

- Loopback on one Windows 11 host: client, origin, and all gateways share the CPUs (24 logical CPUs on the L5 host; cores are not pinned). No network latency, no packet loss, no NIC. Results say nothing about a real deployment.
- The origin is a local Node process serving prebuilt buffers from memory with no think time: the "no cache" baseline is therefore close to the fastest origin conceivable. A real origin with latency, CPU cost or bandwidth limits would make the cache look better. The measured cache-versus-direct ratios are specific to this setup and must not be extrapolated.
- One single-threaded Node client is the load generator and hashes every body. At the highest rates it may itself be the bottleneck (it also shares the host with the targets); throughput numbers are what THIS client observed, not the capacity of the gateway. This is not checked per run (client CPU is not recorded).
- Cold start plus a 1,000-request warm-up (see 5.2): hit ratios include first-touch misses; they are not steady-state hit ratios. Long-running behaviour, memory growth over hours, GC/allocator effects and eviction are not covered here.
- Closed-loop load at fixed concurrency: latency percentiles under closed loop understate queueing relative to an open-loop (fixed arrival rate) test (coordinated omission). No open-loop test is run.
- The workload is read-only, with a fixed Zipf exponent (1.0) and 1,000 objects; other skews, object counts or sizes are not measured.
- Gateway RSS is the whole process and includes allocator slack; it is not the cache byte budget.
- `cluster` uses test certificates created by the existing launcher tooling; mTLS handshakes happen between gateways only.
- Wall-clock time is the host's; background activity on the machine (antivirus, other lanes' processes) is not controlled. The lanes are told to serialise builds, not CPU use: a run may have been disturbed by another lane's work. Repetition-to-repetition spread (reported by the CI) includes that disturbance.

## 9. Reproduce

```
node scripts/benchmark_scenario.mjs --exe <release gateway> --out-dir <dir> --reps 10
node scripts/benchmark_scenario.mjs --summarize --out-dir <dir>      # recompute the tables from benchmark.runs.jsonl
```

Defaults: ports gateways 18230-18232, cluster 19631-19633, origin 19230 (the L5 block; change with `--port-base`, `--cluster-port-base`, `--origin-port`). Certificates are created once with `scripts/cluster_launcher.mjs up`/`down` into `<out-dir>/cluster-work/certs` (needs `openssl` on the PATH). Raw per-run rows: `benchmark.runs.jsonl`; metadata: `benchmark.meta.json`; tables: `benchmark.summary.md` / `.json`.

## 10. Extension: origin profiles (added BEFORE the profile runs)

Reason: section 8 states that the zero-latency in-memory origin is the best case for "no cache". The comparison that matters for a cache is an origin with real service time or limited capacity. The profile runs add four origin profiles; everything else (catalog, Zipf stream, procedure, statistics, targets) is unchanged.

| Profile | Origin behaviour (catalog objects only; `/__stats`, `/__manifest`, `/__reset` are not delayed) |
|---|---|
| O0 | zero-latency in-memory origin (section 4; the existing results) |
| O1 | fixed 25 ms service time: the response starts 25 ms after the request arrives (timer, no CPU work) |
| O2 | fixed 100 ms service time |
| O3 | limited capacity: 100 ms service time, at most 20 requests in service at once; further requests queue in arrival order, so the origin's maximum rate is 20 / 0.1 s = 200 requests/s |

Cells: N = 5,000 requests, C in {200, 500}, 10 repetitions, targets `direct`, `single`, `cluster` on each profile (O0 is re-run in the same session so that all profiles are measured under the same conditions; the earlier O0 table stays as a separate, larger run). Metrics, hit-ratio definition, warm-up (1,000 requests, same profile: the warm-up also pays the origin's service time) and confidence intervals are as in sections 6 and 7. The 30 s per-request client timeout is unchanged; a timeout is a failed request and is reported.

Additional limitations: the service time is simulated with a timer in a Node process (it adds latency without consuming origin CPU; real origins also burn CPU and have variable service times); the capacity limit is a simple queue with a fixed width, not a model of any real server; the gateway's own origin timeouts, retries and any origin concurrency cap (`PHANTOM_ORIGIN_CAP_MAX`, inert unless intelligence is canary/active) are left at defaults. Under O3, `direct` throughput is bounded by the origin's 200 requests/s by construction, so a gateway advantage there shows origin offload, not gateway speed. Reproduce: `node scripts/benchmark_scenario.mjs --exe <gw> --out-dir <dir> --profiles O0,O1,O2,O3 --sizes 5000 --concs 200,500 --reps 10`.
