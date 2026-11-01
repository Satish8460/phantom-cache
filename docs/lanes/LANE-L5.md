# Lane L5 handover: scenario scripts and benchmark

Author: implementer lane L5. Working copy: `D:\ClaudeTemp\lanes\L5\`. Gateway under test: `D:\ClaudeTemp\audit\phantom_rev6.exe` (release per the brief; sha256 `4087bd7e9536edf1f23cdfe820a870f66e0a0a3f0424252e97f8a07c3c6349ee`, version line `phantom-cache-demo 0.1.0 (commit unknown, built unknown)`). Nothing under `D:\Personal\phantom-cache` was written except this file. No product source, Cargo or package file was changed. Evidence: `D:\ClaudeTemp\lanes\evidence\L5\` (per-scenario `*.result.json`, `*.log.txt`, `*.console.txt`, gateway logs; `bench_full\` raw rows and tables). Evidence tags: "reproduced here" = measured by these scripts on this host; "verified in code" = read in the lane copy.

## 1. Status per item

| Item | Status | Evidence |
|---|---|---|
| 1 Freshness scenario | DONE | `scripts/freshness_scenario.mjs`: 63 assertions passed, 0 failed, exit 0, about 12 s. Real PNG/JSON bytes, sha256 per response. |
| 2 Eviction and memory scenario | DONE | `scripts/eviction_scenario.mjs`: 27 passed, 0 failed, exit 0, about 7 s (now also runs the large-object checks with the default budget). |
| 3 Operability scenario | DONE | `scripts/operability_scenario.mjs`: 31 passed, 0 failed, exit 0, about 13 s (5 runs; one early failure was my own log files being appended across runs, fixed). |
| 4 Benchmark + protocol | DONE | `docs/research/benchmark_protocol.md` written first; `scripts/benchmark_scenario.mjs`: 270 runs (3 targets x 9 cells x 10 repetitions), 100 % success and 0 integrity failures in every run, exit 0, about 4.5 min. Tables in section 5. |

Not done: no screenshots (a browser adds nothing here); Control Center link behaviour out of scope as briefed; no open-loop benchmark; client CPU not recorded.

## 2. Scenario table

| Scenario | Assertions passed / failed | Findings |
|---|---|---|
| freshness | 63 / 0 | L5-F-01 (possible defect); observations O-1..O-3 |
| eviction | 27 / 0 | none; observations O-4, O-5 |
| operability | 31 / 0 | L5-O-01 (observation); O-6 |
| benchmark (zero-latency origin) | 270 runs, 0 failed requests, 0 integrity failures | O-7: the gateway is slower than direct in this setup |
| benchmark (origin profiles O0-O3, section 5b) | 240 runs; 237 with 100 % success; 3 runs of `direct` on O3 had client timeouts; 0 integrity failures | O-8: with 25 ms / 100 ms origins the gateway wins at most cells |

### Findings (possible product defects, with reproduction)

- **L5-F-01 (low/medium; verified in code, reproduced here).** An entry that expires WITHOUT a `stale-while-revalidate`/`stale-if-error` window is refetched in full, without `If-None-Match`, once the lazy sweeper (default every 100 ms) has removed it. The stored ETag is then not used and an origin that would answer 304 re-sends the whole body. Code: `sweep_lazy_reclamation` (`src/core/cache.rs:386-440`) removes expired entries when the stale window is 0 (no stale directive, or `must-revalidate`); validators for a foreground refetch come from the resident entry (`stored_validators`, `src/api/server.rs:2743`, used at `:1936`). Reproduce: `node scripts/freshness_scenario.mjs`, case `reval`: `Cache-Control: public, max-age=1` + ETag, origin honours If-None-Match, wait 1.4 s, request: the origin log shows the second request without `If-None-Match` (`freshness.log.txt`). Conditional revalidation does work inside a stale window (cases `swr-changed`, `swr-304`: the background request carries `If-None-Match`; a 304 restarts the entry age) and for `no-cache` when the request arrives within the sweeper tick. Impact: bandwidth and origin load for large objects; correctness unaffected. Whether it is intended is the owner's call.
- **L5-O-01 (observation, not verified in code).** With the origin down (loopback connection refused), an uncached key takes about 2.05 s to be answered `502` (`X-Cache-Error: NetworkError`); 50 concurrent such requests also take about 2 s. A refused loopback connection is normally immediate, so this looks like a fixed delay or retry in the origin fetch path. Reproduce: stop the origin, `GET /api/v1/cache/<uncached>`, time it (`operability.log.txt` case O4; the freshness run also shows 2053 ms with the origin gone, including a stale-if-error answer).

### Observations (reproduced here)

- O-1 The answer to the full refetch of an expired entry is `X-Cache-Status: MISS`, `X-Cache-Source: origin`, `Age: 0` (case `reval`).
- O-2 `no-store`, `private`, `Vary: Accept-Language` and `Set-Cookie` (without `public`) responses are never cached (3 requests = 3 origin requests, all MISS/origin); the `public, max-age=60` control is cached (1 origin request, then HIT/local). `no-cache`+ETag: every request reaches the origin (3 origin requests), the later ones with `If-None-Match`, status MISS/origin. Client `If-None-Match` matching (also weak `W/"c1"`) gets 304 HIT with no body and no origin request.
- O-3 Staleness cap (`PHANTOM_MAX_STALE_MS=3000`, `max-age=1`, directive window 600 s, origin returning 500): the entry was served STALE with the old bytes from age 1 s to 3.9 s and never after lifetime+cap = 4 s (tolerance 250 ms); from 4.3 s the client got honest 500s. Both the SWR and the SIE variant. `stale_cap_violations` stayed 0; `stale_served_total` counted the stale answers (13 and 18). The stale-if-error window is honoured (`stale-if-error=2`, entry older than 3 s -> 500). Without a directive an origin 500 is not masked by stale bytes. Single-flight: 5 concurrent stale GETs caused exactly 1 origin revalidation. With the origin process gone, an expired entry with `stale-if-error` was served STALE; an uncached key got 502; the gateway stayed ready. SWR stale answers returned in 1-2 ms while the origin was delayed 700 ms.
- O-4 Eviction (8 MiB budget; 22.6 MB of one-hit PNG traffic plus 10 hot keys re-read every 10 inserts; default policy): `bytes_resident` max sampled 8,140,015 of 8,388,608 (never above), 239 evictions, hot keys 10/10 still HIT, the 30 oldest one-hit keys 0/30 HIT, the 30 newest 30/30 HIT. Accounting: 40 images, 2,826,815 body bytes sent, `bytes_resident` 2,845,525 (468 B per entry overhead, `basis: logical_accounting`); after DELETE of all 352 keys `bytes_resident` 0. Process RSS: 11.5 MB at start, 22.6 MB under pressure, 13.3 MB after delete (whole process, not the budget). **Large objects (CORRECTED; the first version of this handover said a 1.2 MiB image is never cached, which is wrong in general).** The rule (verified in code): the gateway builds the cache with 8 shards (`PartitionedCache::new(8)`, `src/main.rs:204`) and `set_byte_budget` gives each shard `PHANTOM_MAX_CACHE_BYTES / 8` (`src/core/cache.rs:1843-1850`); an entry is NOT stored when key + fixed overhead + body is larger than ONE shard's budget (`src/core/cache.rs:1496-1499`, counted in `oversize_rejections` and `byte_budget_rejections`). So the per-entry limit is budget/8: 1 MiB under the 8 MiB budget my eviction phase uses, 32 MiB under the default 256 MiB. Reproduced here with `scripts/eviction_scenario.mjs` on both budgets: at 8 MiB a 900 KB PNG is cached (HIT/local on the second GET), a 1.2 MiB PNG (1,259,111 B) is served with correct bytes but not cached (second GET MISS, `oversize_rejections` +2); at the default 256 MiB the same 1.2 MiB PNG IS cached (second GET HIT/local, no rejections), which agrees with the owner's test. A 2.5 MiB PNG (2,621,771 B) gets `502` with `X-Cache-Error: ResponseTooLarge` on both budgets: the origin fetch refuses any body above `MAX_BODY_BYTES` = 2 MiB (`src/api/server.rs:2616`; checks at `:3122` Content-Length and `:3156` body; mapping to 502 at `:2499-2503`), before the cache budget is consulted (rejection counters unchanged). The gateway stayed ready and no truncated 200 was returned. The eviction policy cannot be selected by configuration, so only the default was tested.
- O-5 Metrics expose `byte_budget`, `bytes_resident`, `evictions_total`, `byte_budget_rejections`, `oversize_rejections`, `capacity_rejections`, `process.rss_bytes`, hit/miss counters, `revalidations.*`, `stale_served_total`, `stale_cap_violations`, `outcomes.responses.*`, origin error counters. Per-key and per-policy eviction detail is not exposed.
- O-6 Operability: `PHANTOM_REQUIRE_AUTH=yes` -> exit 2 with exactly one `[CONFIG ERROR]` line naming the key; four simultaneous problems -> exit 2 and four lines; `PHANTOM_MAX_STALE_MS=0`, a non-boolean, `REQUIRE_AUTH=true` without a secret -> exit 2; `--check-config` exits 2 / 0; unknown `PHANTOM_*` variables print `WARNING: unknown environment variable ... is ignored` and the gateway starts. `/health` 200 `OK`; `/health/live` `{"status":"alive"}`; `/health/ready` `{"checks":{"config_valid":true,"draining":false,"listening":true},"status":"ready"}`. Kill and restart under a running client: 60 requests OK before, 61 `ECONNREFUSED` during the 2 s outage, ready 106 ms after spawn, 79 requests after with 0 errors, all bytes correct, first response per key after restart is MISS (cold). Origin down: cached key HIT, uncached keys honest 502 (50 concurrent too), gateway alive and ready, recovered with no restart. Abuse: 10 KB and 70 KB paths -> 414, 1 MB path -> 431, 100 KB query -> 414, control characters / `%00` / `%0d%0a` / space / `..` / `%2e%2e` / `./` / HTTP/9.9 / binary garbage -> 400, invalid method -> 405; 5 MiB bodies: GET and DELETE -> 413, POST -> 405; 100 concurrent control-character requests -> 400; no 5xx, gateway alive and serving correct bytes afterwards. A 100 KB request header was accepted (404 only because the key did not exist).
- O-7 See section 5.
- O-8 See section 5b: with 25 ms or 100 ms origin service time the gateway is faster than direct in most cells (not at O1 C=500 for single, where the CI includes 1, and cluster is slower there), and with a capacity-limited origin it keeps the origin at 10-17 % of the load; at zero latency it is still slower.

## 3. Files created (relative to the lane root `D:\ClaudeTemp\lanes\L5\`; the owner copies exactly these)

- `scripts/freshness_scenario.mjs`
- `scripts/eviction_scenario.mjs` (CHANGED in the second round: large-object phase on both budgets, `X-Cache-Error` captured)
- `scripts/operability_scenario.mjs`
- `scripts/benchmark_scenario.mjs` (CHANGED: `--profiles`, profile column in rows and tables)
- `scripts/lib/bench_stats.mjs`
- `scripts/lib/scenario_common.mjs` (CHANGED: origin `--delay-ms` / `--max-concurrent`; not in the brief's list: shared PNG/JSON/text generator, scriptable origin, gateway spawn, HTTP client, recorder; named so it does not collide with L1's `jwt_mint.mjs` / `content_origin.mjs`)
- `docs/research/benchmark_protocol.md` (CHANGED: section 10 origin profiles, written before the profile run)
- `docs/lanes/LANE-L5.md` (this file, written directly to `D:\Personal\phantom-cache\docs\lanes\`)

No existing file was edited. The benchmark calls the existing `scripts/cluster_launcher.mjs up` then `down` once, only to create test certificates.

## 4. Reproduce live

```
cd D:\ClaudeTemp\lanes\L5
node scripts/freshness_scenario.mjs   --exe D:\ClaudeTemp\audit\phantom_rev6.exe     # ports 18230,18231, origin 19230
node scripts/eviction_scenario.mjs    --exe D:\ClaudeTemp\audit\phantom_rev6.exe     # 18232, origin 19230
node scripts/operability_scenario.mjs --exe D:\ClaudeTemp\audit\phantom_rev6.exe     # 18233, origin 19230
node scripts/benchmark_scenario.mjs   --exe D:\ClaudeTemp\audit\phantom_rev6.exe --out-dir <dir> --reps 10   # 18230-18232, 19631-19633, origin 19230
node scripts/benchmark_scenario.mjs --summarize --out-dir <dir>
# origin profiles O0-O3 (section 5b):
node scripts/benchmark_scenario.mjs --exe D:ClaudeTempauditphantom_rev6.exe --out-dir <dir> --profiles O0,O1,O2,O3 --sizes 5000 --concs 200,500 --reps 10
```
Run one at a time (shared origin port; the benchmark is timing-sensitive). Each exits 0 only if every assertion held and writes `<out-dir>\<name>.result.json` and `.log.txt` (default `D:\ClaudeTemp\lanes\evidence\L5\<name>`). Results on this host: freshness 63/0, eviction 27/0, operability 31/0, benchmark 270 runs 0 failures, all exit 0. All processes I started were stopped (only TIME_WAIT sockets remain on the L5 ports).

## 5. Benchmark results (reproduced here; CLAUDE.md section 4 labels)

Setup as captured by the script (`benchmark.meta.json`): Windows_NT 10.0.26300 (Windows 11 Home Single Language), Intel Core i7-14650HX, 24 logical CPUs, 15.7 GiB RAM (about 2.4 GiB free at start: the host was busy), Node v24.5.0. Gateway `phantom_rev6.exe` (release per the brief; not verifiable by the script). Loopback only, one host; origin = local Node process serving prebuilt in-memory buffers (no latency); client = one Node process, closed loop, keep-alive, sha256 of every body checked against the origin's own manifest. Catalog: 1,000 distinct objects, 19,872,624 bytes (mean about 19.9 KB: 720 JSON 1-6 KB, 160 text 2-8 KB, 120 PNG at 30-60 KB or 300-900 KB), Zipf exponent 1.0, seeds `20261031 + 1000 x rep + cell`. Each run: cold start of the target, 1,000-request warm-up (other seed, not measured), then the measured run. Cells N in {1,000; 5,000; 10,000} x C in {50; 200; 500}. 10 repetitions per cell and target (t = 2.262). Values are mean ± half-width of the 95 % Student-t interval over repetitions; p50/p95/p99 are per-run percentiles averaged over repetitions. Protocol and limitations: `docs/research/benchmark_protocol.md`. Raw rows: `D:\ClaudeTemp\lanes\evidence\L5\bench_full\benchmark.runs.jsonl`.

Headline (this setup only, no extrapolation): success rate 100 % and 0 integrity failures in all 270 runs (1,440,000 measured requests in total, plus warm-up). The cache cuts origin load (origin requests per 100 client requests, mean: single 5.9 at N=10,000 to 17 at N=1,000; cluster 11 to 31), but it does NOT raise throughput or lower latency here. Against the direct origin the single gateway reached 0.75 to 0.88 of the direct throughput (paired per-repetition ratio; the CI includes 1 only for N=5,000 C=500, 0.88 ± 0.13) and the 3-node cluster 0.59 to 0.83 (all CIs below 1); p50 latency was 1.12 to 2.82 times the direct value and p99 1.13 to 3.13 times. The direct baseline is an in-memory local origin with no think time, the best case for "no cache"; a slower origin would change the picture, and this benchmark does not measure that.

### Full table (copied from `benchmark.summary.md`)

| target | N | C | reps | success (min) | req/s (mean ± 95% CI) | p50 ms | p95 ms | p99 ms | origin req / 100 req | local % | peer % | origin % | coalesced % | RSS before→after MB (sum of gateways) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| cluster | 10000 | 200 | 10 | 100.00% | 11197 ± 842 | 15.1 ± 0.9 | 30.5 ± 2.8 | 99.5 ± 21.1 | 11.0 ± 0.1 | 84.6 | 4.3 | 11.0 | 0.1 | 61 ± 1 → 101 ± 2 |
| cluster | 10000 | 50 | 10 | 100.00% | 10936 ± 689 | 3.2 ± 0.2 | 13.7 ± 2.6 | 27.2 ± 3.9 | 11.0 ± 0.2 | 84.5 | 4.3 | 11.0 | 0.1 | 57 ± 1 → 100 ± 1 |
| cluster | 10000 | 500 | 10 | 100.00% | 9173 ± 816 | 44.8 ± 3.0 | 116.5 ± 15.3 | 252.5 ± 58.1 | 10.9 ± 0.2 | 84.7 | 4.2 | 10.9 | 0.2 | 64 ± 1 → 103 ± 1 |
| cluster | 1000 | 200 | 10 | 100.00% | 5604 ± 807 | 19.5 ± 2.5 | 100.2 ± 24.7 | 104.5 ± 25.0 | 24.0 ± 0.7 | 70.0 | 5.7 | 24.0 | 0.4 | 60 ± 1 → 70 ± 1 |
| cluster | 1000 | 50 | 10 | 100.00% | 7532 ± 1122 | 3.6 ± 0.9 | 24.3 ± 7.0 | 30.3 ± 8.8 | 24.5 ± 0.8 | 69.5 | 5.7 | 24.5 | 0.3 | 57 ± 1 → 66 ± 2 |
| cluster | 1000 | 500 | 10 | 100.00% | 3355 ± 514 | 86.4 ± 12.7 | 277.1 ± 50.1 | 285.1 ± 51.1 | 24.8 ± 0.5 | 68.9 | 5.3 | 24.8 | 1.0 | 65 ± 2 → 76 ± 2 |
| cluster | 5000 | 200 | 10 | 100.00% | 9627 ± 694 | 15.9 ± 0.9 | 44.4 ± 7.5 | 106.4 ± 17.9 | 15.5 ± 0.2 | 79.3 | 4.9 | 15.5 | 0.3 | 61 ± 2 → 89 ± 1 |
| cluster | 5000 | 50 | 10 | 100.00% | 10296 ± 893 | 3.5 ± 0.5 | 14.3 ± 2.7 | 28.0 ± 5.9 | 15.5 ± 0.2 | 79.5 | 4.8 | 15.5 | 0.2 | 57 ± 1 → 84 ± 2 |
| cluster | 5000 | 500 | 10 | 100.00% | 7834 ± 849 | 42.5 ± 1.9 | 237.2 ± 45.7 | 271.3 ± 50.2 | 15.6 ± 0.3 | 79.0 | 4.8 | 15.6 | 0.6 | 64 ± 1 → 93 ± 2 |
| direct | 10000 | 200 | 10 | 100.00% | 14054 ± 825 | 12.1 ± 0.5 | 19.4 ± 1.6 | 85.0 ± 16.9 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 10000 | 50 | 10 | 100.00% | 15952 ± 690 | 2.7 ± 0.1 | 5.7 ± 0.5 | 9.0 ± 1.0 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 10000 | 500 | 10 | 100.00% | 11983 ± 999 | 32.5 ± 1.6 | 52.6 ± 5.4 | 195.2 ± 44.3 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 1000 | 200 | 10 | 100.00% | 8000 ± 839 | 13.3 ± 1.4 | 74.0 ± 12.7 | 76.3 ± 13.0 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 1000 | 50 | 10 | 100.00% | 12833 ± 1176 | 2.8 ± 0.2 | 7.0 ± 1.4 | 19.6 ± 3.7 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 1000 | 500 | 10 | 100.00% | 4453 ± 429 | 31.8 ± 3.5 | 196.0 ± 22.1 | 197.7 ± 22.0 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 5000 | 200 | 10 | 100.00% | 12731 ± 827 | 12.9 ± 0.8 | 21.0 ± 1.7 | 75.9 ± 14.8 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 5000 | 50 | 10 | 100.00% | 15330 ± 1405 | 2.9 ± 0.2 | 5.7 ± 1.0 | 10.6 ± 3.9 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| direct | 5000 | 500 | 10 | 100.00% | 9573 ± 811 | 34.4 ± 4.0 | 203.5 ± 25.8 | 213.7 ± 26.4 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| single | 10000 | 200 | 10 | 100.00% | 11916 ± 697 | 13.9 ± 0.6 | 25.7 ± 2.4 | 104.0 ± 42.6 | 5.9 ± 0.1 | 93.9 | 0.0 | 5.9 | 0.2 | 24 ± 1 → 39 ± 1 |
| single | 10000 | 50 | 10 | 100.00% | 13183 ± 616 | 3.0 ± 0.1 | 7.1 ± 0.8 | 21.1 ± 2.9 | 5.9 ± 0.1 | 94.0 | 0.0 | 5.9 | 0.1 | 24 ± 1 → 39 ± 1 |
| single | 10000 | 500 | 10 | 100.00% | 10536 ± 629 | 37.4 ± 1.8 | 85.6 ± 8.5 | 219.0 ± 50.7 | 5.9 ± 0.1 | 94.0 | 0.0 | 5.9 | 0.2 | 27 ± 1 → 41 ± 1 |
| single | 1000 | 200 | 10 | 100.00% | 6375 ± 661 | 17.2 ± 2.6 | 82.1 ± 13.9 | 86.4 ± 14.9 | 16.9 ± 0.5 | 82.5 | 0.0 | 16.9 | 0.6 | 24 ± 1 → 30 ± 1 |
| single | 1000 | 50 | 10 | 100.00% | 9594 ± 943 | 3.3 ± 0.4 | 19.2 ± 2.8 | 23.4 ± 3.6 | 17.0 ± 0.7 | 82.7 | 0.0 | 17.0 | 0.3 | 23 ± 1 → 28 ± 1 |
| single | 1000 | 500 | 10 | 100.00% | 3499 ± 836 | 92.9 ± 33.4 | 246.1 ± 45.2 | 261.0 ± 50.7 | 17.1 ± 0.6 | 81.6 | 0.0 | 17.1 | 1.3 | 25 ± 1 → 33 ± 3 |
| single | 5000 | 200 | 10 | 100.00% | 10809 ± 556 | 14.7 ± 0.9 | 31.9 ± 2.8 | 87.3 ± 11.2 | 9.3 ± 0.1 | 90.5 | 0.0 | 9.3 | 0.2 | 24 ± 1 → 36 ± 1 |
| single | 5000 | 50 | 10 | 100.00% | 11506 ± 825 | 3.3 ± 0.2 | 10.0 ± 2.4 | 23.0 ± 3.5 | 9.3 ± 0.2 | 90.4 | 0.0 | 9.3 | 0.2 | 23 ± 1 → 35 ± 1 |
| single | 5000 | 500 | 10 | 100.00% | 8353 ± 812 | 40.1 ± 3.8 | 223.6 ± 38.8 | 241.1 ± 42.0 | 9.3 ± 0.2 | 90.3 | 0.0 | 9.3 | 0.3 | 26 ± 1 → 38 ± 3 |

Paired ratios gateway / direct (same repetition, same request sequence; mean ± 95% CI of the per-repetition ratio; < 1 means the gateway is slower for throughput, > 1 means higher latency for p50/p99):

| target | N | C | pairs | throughput ratio | p50 ratio | p99 ratio |
|---|---|---|---|---|---|---|
| single | 1000 | 50 | 10 | 0.75 ± 0.06 | 1.19 ± 0.10 | 1.21 ± 0.13 |
| single | 1000 | 200 | 10 | 0.80 ± 0.08 | 1.32 ± 0.24 | 1.13 ± 0.06 |
| single | 1000 | 500 | 10 | 0.79 ± 0.19 | 2.82 ± 0.76 | 1.33 ± 0.25 |
| single | 5000 | 50 | 10 | 0.76 ± 0.07 | 1.17 ± 0.10 | 2.61 ± 0.84 |
| single | 5000 | 200 | 10 | 0.85 ± 0.04 | 1.14 ± 0.06 | 1.19 ± 0.13 |
| single | 5000 | 500 | 10 | 0.88 ± 0.13 | 1.18 ± 0.14 | 1.17 ± 0.26 |
| single | 10000 | 50 | 10 | 0.83 ± 0.05 | 1.12 ± 0.04 | 2.42 ± 0.52 |
| single | 10000 | 200 | 10 | 0.85 ± 0.07 | 1.15 ± 0.06 | 1.26 ± 0.45 |
| single | 10000 | 500 | 10 | 0.88 ± 0.04 | 1.16 ± 0.06 | 1.17 ± 0.22 |
| cluster | 1000 | 50 | 10 | 0.59 ± 0.08 | 1.29 ± 0.28 | 1.57 ± 0.36 |
| cluster | 1000 | 200 | 10 | 0.70 ± 0.08 | 1.48 ± 0.20 | 1.38 ± 0.23 |
| cluster | 1000 | 500 | 10 | 0.76 ± 0.12 | 2.78 ± 0.51 | 1.46 ± 0.27 |
| cluster | 5000 | 50 | 10 | 0.68 ± 0.06 | 1.24 ± 0.19 | 2.92 ± 0.54 |
| cluster | 5000 | 200 | 10 | 0.76 ± 0.04 | 1.23 ± 0.07 | 1.43 ± 0.15 |
| cluster | 5000 | 500 | 10 | 0.83 ± 0.12 | 1.25 ± 0.11 | 1.31 ± 0.30 |
| cluster | 10000 | 50 | 10 | 0.69 ± 0.05 | 1.20 ± 0.04 | 3.13 ± 0.72 |
| cluster | 10000 | 200 | 10 | 0.80 ± 0.06 | 1.24 ± 0.06 | 1.25 ± 0.35 |
| cluster | 10000 | 500 | 10 | 0.77 ± 0.04 | 1.38 ± 0.07 | 1.33 ± 0.22 |


### 5b. Origin profiles (slow and capacity-limited origins; reproduced here)

Defined in `docs/research/benchmark_protocol.md` section 10 before the run. Profiles: **O0** zero-latency in-memory origin (re-run in this session); **O1** fixed 25 ms service time; **O2** fixed 100 ms; **O3** 100 ms service time with at most 20 requests in service at once, the rest queued (origin ceiling 200 req/s by construction). Service time is a timer in the origin process (no CPU cost). Everything else as in section 5: N = 5,000, C = 200 and 500, 10 repetitions, cold start + 1,000-request warm-up on the same profile, same seeds on every profile, host and gateway as above. Raw rows: `D:ClaudeTemplanesevidenceL5ench_profilesenchmark.runs.jsonl`; console `bench_profiles.console.txt`.

Success and integrity: 240 runs, 0 integrity failures. 237 runs had 100 % success. **Three `direct` runs on O3 had client timeouts** (30 s per-request client timeout; O3 direct C=200 rep 2: 396 of 5,000 failed; C=500 rep 4: 500 failed; C=200 rep 10: 200 failed, one ECONNRESET). No gateway run failed. The cause of the origin stalls beyond the expected queueing (200 requests / 200 req/s is about 1 s) was not investigated; it may be the origin process or host load. Because of those failures the O3 `direct` rows (mean success 92.08 % at C=200 and 90.00 % at C=500) are computed from the verified responses only, and their p95/p99 and the O3 paired ratios have very wide intervals (e.g. O3 single C=200 throughput ratio 48.77 ± 62.02): read them as "direct on a saturated origin is unstable", not as a precise factor.

Reading, in the order of the table (no extrapolation; this is a simulated origin on loopback):
- O0 (zero latency): the gateway is still slower than direct: single throughput ratio 0.85 ± 0.03 (C=200) and 0.88 ± 0.03 (C=500); cluster 0.77 ± 0.06 and 0.77 ± 0.05.
- O1 (25 ms): single is faster at C=200 (ratio 1.97 ± 0.11, p50 0.33 ± 0.02 of direct) and indistinguishable from direct at C=500 (0.98 ± 0.09, CI contains 1; p99 1.19 ± 0.12, higher). Cluster 1.69 ± 0.14 at C=200 but SLOWER at C=500 (0.86 ± 0.07, p99 1.25 ± 0.07).
- O2 (100 ms): faster everywhere: single 4.79 ± 0.12 (C=200) and 2.13 ± 0.11 (C=500); cluster 3.72 ± 0.10 and 1.94 ± 0.12. p50 is 0.05 to 0.23 of direct; p99 is 0.80 to 1.09 of direct, i.e. the tail is not clearly improved at C=500 (single 1.02 ± 0.06, cluster 1.09 ± 0.05, slightly higher).
- O3 (capacity-limited): direct is bounded by the origin (146 ± 53 and 162 ± 36 req/s, with the timeouts above); single reaches 1457 ± 216 and 1296 ± 180 req/s, cluster 1047 ± 21 and 953 ± 30. The cache sends 9.3 to 9.4 (single) or 16.5 to 16.7 (cluster) origin requests per 100 client requests, so the origin sees 10 to 17 % of the load. Tail latency of the cluster at C=500 is large (p99 1806.7 ± 180.2 ms) because the origin queue is still shared by the misses.
- Origin requests per 100 client requests are the same on every profile within the CIs (O0 to O3: single 9.3-9.4, cluster 15.5-16.7), as expected, since the workload and cache are identical; the time cost of a miss is what changes.
- The cluster is slower than the single gateway on O0, O1 and O3 here and about equal on O2 at C=500; it also sends 4-5 % of responses as peer fills. I did not investigate why.

Full table (profile, target, N, C; mean ± 95 % CI over 10 repetitions; `origin req / 100 req` is counted by the origin process):

| profile | target | N | C | reps | success (min) | req/s (mean ± 95% CI) | p50 ms | p95 ms | p99 ms | origin req / 100 req | local % | peer % | origin % | coalesced % | RSS before→after MB (sum of gateways) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| O0 | cluster | 5000 | 200 | 10 | 100.00% | 10338 ± 1162 | 15.4 ± 1.0 | 42.2 ± 9.4 | 87.2 ± 18.7 | 15.7 ± 0.2 | 79.2 | 4.9 | 15.7 | 0.3 | 61 ± 1 → 89 ± 1 |
| O0 | cluster | 5000 | 500 | 10 | 100.00% | 8476 ± 1085 | 41.3 ± 3.1 | 225.7 ± 65.1 | 247.4 ± 71.6 | 15.5 ± 0.3 | 79.1 | 5.0 | 15.5 | 0.3 | 63 ± 1 → 91 ± 1 |
| O0 | direct | 5000 | 200 | 10 | 100.00% | 13427 ± 984 | 12.2 ± 0.8 | 20.9 ± 3.3 | 72.9 ± 11.9 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O0 | direct | 5000 | 500 | 10 | 100.00% | 10920 ± 983 | 30.8 ± 1.6 | 169.6 ± 38.1 | 178.2 ± 37.3 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O0 | single | 5000 | 200 | 10 | 100.00% | 11341 ± 749 | 13.7 ± 0.8 | 32.8 ± 1.6 | 86.5 ± 16.4 | 9.4 ± 0.1 | 90.3 | 0.0 | 9.4 | 0.3 | 24 ± 1 → 36 ± 1 |
| O0 | single | 5000 | 500 | 10 | 100.00% | 9603 ± 953 | 36.6 ± 2.8 | 182.3 ± 31.9 | 199.4 ± 36.9 | 9.3 ± 0.2 | 90.4 | 0.0 | 9.3 | 0.4 | 25 ± 1 → 38 ± 1 |
| O1 | cluster | 5000 | 200 | 10 | 100.00% | 9135 ± 647 | 11.6 ± 0.7 | 70.3 ± 11.0 | 109.2 ± 16.8 | 16.1 ± 0.2 | 78.3 | 4.4 | 16.1 | 1.2 | 61 ± 1 → 90 ± 1 |
| O1 | cluster | 5000 | 500 | 10 | 100.00% | 7313 ± 731 | 40.3 ± 2.7 | 244.7 ± 30.4 | 279.6 ± 32.4 | 15.8 ± 0.2 | 78.2 | 4.7 | 15.8 | 1.3 | 64 ± 1 → 93 ± 2 |
| O1 | direct | 5000 | 200 | 10 | 100.00% | 5405 ± 170 | 32.5 ± 1.8 | 47.3 ± 2.0 | 110.6 ± 9.4 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O1 | direct | 5000 | 500 | 10 | 100.00% | 8485 ± 527 | 40.6 ± 4.6 | 213.0 ± 25.6 | 224.7 ± 25.5 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O1 | single | 5000 | 200 | 10 | 100.00% | 10649 ± 421 | 10.8 ± 0.6 | 56.5 ± 2.7 | 87.5 ± 9.8 | 9.4 ± 0.1 | 89.1 | 0.0 | 9.4 | 1.5 | 24 ± 1 → 37 ± 1 |
| O1 | single | 5000 | 500 | 10 | 100.00% | 8285 ± 700 | 35.7 ± 2.5 | 222.7 ± 33.5 | 267.0 ± 39.2 | 9.3 ± 0.2 | 89.0 | 0.0 | 9.3 | 1.7 | 25 ± 1 → 38 ± 1 |
| O2 | cluster | 5000 | 200 | 10 | 100.00% | 6607 ± 158 | 5.0 ± 0.5 | 122.4 ± 2.2 | 160.5 ± 11.2 | 16.5 ± 0.2 | 77.3 | 4.0 | 16.5 | 2.2 | 61 ± 1 → 90 ± 2 |
| O2 | cluster | 5000 | 500 | 10 | 100.00% | 7520 ± 400 | 24.6 ± 2.7 | 182.1 ± 18.8 | 278.0 ± 23.0 | 16.7 ± 0.3 | 75.7 | 3.9 | 16.7 | 3.7 | 64 ± 2 → 94 ± 2 |
| O2 | direct | 5000 | 200 | 10 | 100.00% | 1777 ± 10 | 109.2 ± 0.2 | 118.9 ± 3.5 | 174.7 ± 8.5 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O2 | direct | 5000 | 500 | 10 | 100.00% | 3889 ± 125 | 114.3 ± 5.0 | 247.4 ± 25.7 | 256.7 ± 26.1 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O2 | single | 5000 | 200 | 10 | 100.00% | 8510 ± 208 | 5.1 ± 0.7 | 111.8 ± 1.8 | 138.6 ± 6.1 | 9.4 ± 0.1 | 87.1 | 0.0 | 9.4 | 3.5 | 25 ± 1 → 37 ± 1 |
| O2 | single | 5000 | 500 | 10 | 100.00% | 8278 ± 425 | 25.7 ± 2.2 | 168.5 ± 18.5 | 260.3 ± 22.3 | 9.3 ± 0.2 | 86.0 | 0.0 | 9.3 | 4.7 | 27 ± 1 → 40 ± 1 |
| O3 | cluster | 5000 | 200 | 10 | 100.00% | 1047 ± 21 | 33.4 ± 22.4 | 888.9 ± 62.4 | 969.2 ± 33.6 | 16.5 ± 0.3 | 77.2 | 4.0 | 16.5 | 2.3 | 59 ± 1 → 87 ± 1 |
| O3 | cluster | 5000 | 500 | 10 | 100.00% | 953 ± 30 | 163.8 ± 61.5 | 1600.2 ± 224.1 | 1806.7 ± 180.2 | 16.7 ± 0.4 | 75.6 | 3.8 | 16.7 | 3.9 | 63 ± 1 → 91 ± 1 |
| O3 | direct | 5000 | 200 | 10 | 92.08% | 146 ± 53 | 1095.1 ± 1.8 | 24758.7 ± 53383.2 | 117378.0 ± 182336.6 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O3 | direct | 5000 | 500 | 10 | 90.00% | 162 ± 36 | 2736.4 ± 6.8 | 23817.2 ± 47584.9 | 24228.6 ± 47696.9 | 100.0 ± 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | n/a |
| O3 | single | 5000 | 200 | 10 | 100.00% | 1457 ± 216 | 99.6 ± 36.5 | 445.1 ± 84.7 | 648.7 ± 95.7 | 9.4 ± 0.1 | 88.8 | 0.0 | 9.4 | 1.8 | 23 ± 1 → 35 ± 1 |
| O3 | single | 5000 | 500 | 10 | 100.00% | 1296 ± 180 | 267.9 ± 82.8 | 1110.3 ± 107.0 | 1432.4 ± 109.2 | 9.3 ± 0.2 | 88.3 | 0.0 | 9.3 | 2.5 | 25 ± 1 → 38 ± 1 |

Paired ratios gateway / direct (same repetition, same request sequence; mean ± 95% CI of the per-repetition ratio; < 1 means the gateway is slower for throughput, > 1 means higher latency for p50/p99):

| profile | target | N | C | pairs | throughput ratio | p50 ratio | p99 ratio |
|---|---|---|---|---|---|---|---|
| O0 | single | 5000 | 200 | 10 | 0.85 ± 0.03 | 1.13 ± 0.07 | 1.19 ± 0.13 |
| O0 | single | 5000 | 500 | 10 | 0.88 ± 0.03 | 1.19 ± 0.08 | 1.13 ± 0.05 |
| O1 | single | 5000 | 200 | 10 | 1.97 ± 0.11 | 0.33 ± 0.02 | 0.79 ± 0.05 |
| O1 | single | 5000 | 500 | 10 | 0.98 ± 0.09 | 0.90 ± 0.13 | 1.19 ± 0.12 |
| O2 | single | 5000 | 200 | 10 | 4.79 ± 0.12 | 0.05 ± 0.01 | 0.80 ± 0.05 |
| O2 | single | 5000 | 500 | 10 | 2.13 ± 0.11 | 0.23 ± 0.02 | 1.02 ± 0.06 |
| O3 | single | 5000 | 200 | 10 | 48.77 ± 62.02 | 0.09 ± 0.03 | 0.45 ± 0.18 |
| O3 | single | 5000 | 500 | 10 | 12.37 ± 11.26 | 0.10 ± 0.03 | 0.41 ± 0.12 |
| O0 | cluster | 5000 | 200 | 10 | 0.77 ± 0.06 | 1.27 ± 0.08 | 1.19 ± 0.12 |
| O0 | cluster | 5000 | 500 | 10 | 0.77 ± 0.05 | 1.34 ± 0.12 | 1.36 ± 0.12 |
| O1 | cluster | 5000 | 200 | 10 | 1.69 ± 0.14 | 0.36 ± 0.03 | 0.99 ± 0.13 |
| O1 | cluster | 5000 | 500 | 10 | 0.86 ± 0.07 | 1.02 ± 0.14 | 1.25 ± 0.07 |
| O2 | cluster | 5000 | 200 | 10 | 3.72 ± 0.10 | 0.05 ± 0.00 | 0.92 ± 0.07 |
| O2 | cluster | 5000 | 500 | 10 | 1.94 ± 0.12 | 0.22 ± 0.03 | 1.09 ± 0.05 |
| O3 | cluster | 5000 | 200 | 10 | 38.13 ± 48.87 | 0.03 ± 0.02 | 0.68 ± 0.26 |
| O3 | cluster | 5000 | 500 | 10 | 9.50 ± 9.28 | 0.06 ± 0.02 | 0.51 ± 0.14 |


### Reading notes and limits

- The ratio table pairs the same repetition and the same request sequence: throughput ratio below 1 means the gateway was slower than the direct origin; p50/p99 ratio above 1 means higher latency.
- `local % + peer % + origin % + coalesced %` is 100 % of the responses; `coalesced` waited for another request's fetch. The hit ratio includes first-touch misses of a cold-started cache (warm-up is only 1,000 requests), so it is not a steady-state hit ratio.
- The cluster serves 4.3 to 5.7 % of responses from a peer (`X-Cache-Source: peer`) because the client is not key-aware (round robin).
- RSS is the sum over the gateways of `metrics.process.rss_bytes` as the gateway reports it, before and after the measured run (single about 24 → 39 MB, 3-node cluster about 60 → 100 MB at N=10,000). It is not a bound; no long-run memory test was done.
- The single Node client may be a bottleneck at the highest rates (direct reached about 16,000 req/s at C=50) and shares the host with the targets; client CPU was not recorded. Do not read throughput as gateway capacity.
- Closed-loop load understates tail latency under overload (coordinated omission). Repetition independence and normality are assumptions. Other lanes were running on the same host, which inflates the spread. The p99 of a 1,000-request run rests on its 10 slowest requests.

## 6. Known limitations and uncertainties

- "Release build" is taken from the brief; the scripts record only the exe sha256 and version line.
- Freshness timings use whole-second `max-age` values and real sleeps; the cap-boundary assertions have a 250 ms tolerance and could flake on a heavily loaded host (they passed in every run, 4 of 4).
- The eviction scenario tests the default policy only (no selector). Thresholds I chose: hot keys >= 90 % HIT and old one-hit keys <= 10 % HIT (measured 100 % and 0 %); `bytes_resident` within 2 % of body bytes (measured 0.66 % over).
- Operability abuse cases assert "4xx or closed connection, no 5xx", not exact codes. The kill in O3 is a hard process kill; graceful shutdown and draining were not tested (operate.md says draining is not implemented).
- The cluster benchmark reuses test certificates generated by the launcher into `D:\ClaudeTemp\lanes\evidence\L5\bench_smoke\cluster-work\certs`.
- The `reval` case records L5-F-01 as a finding without failing the run; if the owner decides it is a defect and fixes it, turn the finding into an assertion.

## 7. Requests for files I do not own

None required.
