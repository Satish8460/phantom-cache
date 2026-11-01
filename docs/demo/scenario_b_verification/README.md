# Scenario B independent recompute

Independent recomputation of `lab-results/scenario_b/20261030T003932Z` (tier MEASURED_LOOPBACK) from the raw per-request CSVs and gateway snapshots. Written without reading the runner's analysis code (`src/lab_live/analysis.rs`, `scenario_b.rs`); definitions come from `docs/demo/scenario_B_origin_slowdown.md` (section 7), `scenario_b_artifacts.md` and `scenario_b_runbook.md`. The runner code was read only after the independent numbers existed, to explain differences.

## Run

```
node docs/demo/scenario_b_verification/recompute.mjs --root lab-results/scenario_b/20261030T003932Z --out docs/demo/scenario_b_verification
```

Node 24, built-ins only, deterministic (bootstrap PRNG mulberry32, seed 20261030, 10,000 resamples; two consecutive runs give byte-identical `recompute_report.json`). The script reads `--root` only and writes `recompute_report.json`, `recompute_report.md`, `console_output.txt` into `--out`.

## Files

- `recompute.mjs` the script.
- `recompute_report.json` all numbers (per run, per arm, paired statistics, comparison with `summary.json` and `run_metrics.json`).
- `recompute_report.md` generated tables.
- `console_output.txt` console output of the last run.

## Method notes (details in `recompute_report.json` -> `definitions`)

- Analysis window: client `scheduled_ms >= 8000` (equals `in_analysis_window`, verified for every row); origin `arrival_ms_since_load_start >= 8000`. Phases from the manifest: healthy 8-13 s, degraded 13-23 s, recovery 23-31 s.
- Success = HTTP 200 / scheduled requests in window (503 and no-response count as failures). Shed = 503 with `X-Cache-Error: CapacityExceeded`.
- Percentiles: nearest rank `sorted[ceil(p*n)-1]` and linear interpolation; latency column `latency_ms` (done - scheduled); alternatives `service_latency_ms` and 200-only subsets are reported. Three aggregations: pooled, mean of per-run percentiles, difference of pooled.
- Paired statistics use the repetition as the unit (n=10): percentile bootstrap CI, t CI (df 9), exact two-sided sign-flip permutation p (2^10 patterns; minimum attainable 0.001953). Per-request intervals would be wrong because requests inside a run share cache, queue, timer and schedule state (and the seed is shared across arms), so they are not independent.
- Recovery: the spec says only "return to baseline within a measured time". The primary definition here has no tolerance. Variants labelled `POSTHOC` (healthy error rate + 2 pp; in-flight healthy max + 1) were added after reading the runner code, because those tolerances are the runner's; with them the runner's per-run recovery values are reproduced exactly.
- Snapshot clock: the artifacts do not tie `t_ns` to load start; it is estimated by fitting `requests_total` to cumulative client sends (offset about +1 ms in all AIMD runs). The AIMD per-instant cap check is therefore approximate (snapshots every about 265 ms).
- CSV latencies carry 3 decimals while `run_metrics.json` uses full precision, so tiny differences (below 0.0005 ms; occasionally ties in permutation p-values) are expected.
