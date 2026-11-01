# Scenario B (T2) artifact schema

Tier of every artifact: **MEASURED_LOOPBACK** (real `phantom-cache-demo` process, loopback, real time, one host, shared CPU).
Contract: `docs/demo/scenario_B_origin_slowdown.md`. Writer: `phantom-lab scenario-b` (`src/lab_live/`).

## Directory layout

```
lab-results/scenario_b/<utc>/
  manifest.json            experiment-level provenance (written at start with status RUNNING, rewritten at the end)
  summary.json             experiment-level analysis and mechanical verdicts
  runs/rep<NN>_<ARM>/      one directory per (repetition, arm)
    client_requests.csv    one row per scheduled request (all phases; analysis window flagged)
    origin_requests.csv    one row per request the scripted origin served
    gateway_snapshots.jsonl one JSON object per /api/v1/metrics poll
    gateway.log            last 200 lines of the gateway's stdout+stderr
    run_metrics.json       per-run derived metrics (the inputs of summary.json)
```

## manifest.json

`manifest_version`, `tier`, `scenario`, `status` (RUNNING | COMPLETE | CANCELLED | FAILED), `exit_code` (0 complete, 130 cancelled, 1 failed, null while running),
`started_utc` / `ended_utc` (real UTC, second resolution), `command_line`, `seed` (repetition r uses seed + r for arrivals, keys and the origin RNG; identical across the arms of repetition r),
`label_planned` + `label_reasons` (VALID only for release gateway + release driver + >= 10 repetitions + BASE present; otherwise EXPLORATORY with the reasons),
`profile` (`driver_profile`, `gateway_profile` inferred from the binary path), `gateway` (`exe`, `exe_sha256`, environment keys set; inherited `PHANTOM_*` / `ORIGIN_URL` variables are removed),
`git` (`head`, `dirty_entries`), `source_sha256` (every `src/lab_live/*.rs` and `src/bin/phantom-lab.rs`), `host` (OS, arch, CPU string from `PROCESSOR_IDENTIFIER`, logical CPUs; RAM is not collected),
`workload`, `phases`, `origin_script`, `arms`, `arm_configs`, `max_cache_bytes`, `reps`, `snapshot_interval_ms`, `extra.timer_probe` (overshoot of a 1 ms tokio sleep on this host).

## client_requests.csv (all times in ms from the load start `epoch`)

`idx,key,scheduled_ms,send_ms,done_ms,send_lag_ms,latency_ms,service_latency_ms,status,x_cache_status,x_cache_error,bytes,error,in_analysis_window`

- `scheduled_ms`: arrival time fixed by the seed before the run (open loop). `send_ms`: when the request was first written (0 = never written). `send_lag_ms = send - scheduled`.
- `latency_ms = done - scheduled` (coordinated-omission-safe, used for all reported client percentiles). `service_latency_ms = done - send` (what a closed-loop tool would report).
- `status`: HTTP status; **0 = no HTTP response** (see `error`: `deadline`, `connect: ...`, `read: ...`). Such a request counts as a failure and its latency is its completion time (the deadline), never dropped.
- `x_cache_status` (HIT, MISS, COALESCED, STALE, ...; empty if the header is absent), `x_cache_error` (e.g. `CapacityExceeded` for a shed request). `in_analysis_window = 1` when `scheduled_ms >= warm-up`.

## origin_requests.csv

`arrival_ms_since_load_start,conc_at_arrival,service_ms,status,degraded,path_hash`

- `conc_at_arrival`: requests in service on the origin at this arrival, including this one (the slot is released just before the response bytes are written).
- `service_ms`: measured wall time between request parsed and the end of the scripted sleep (nominal + timer overshoot). `degraded = 1` when the arrival fell inside the degraded interval. `status` 200 or 503 (scripted error rate).
- `path_hash` is the FNV-1a hash of the request target (the key itself is in `client_requests.csv`).

## gateway_snapshots.jsonl

`{"t_ns", "schema", "origin_cap": {"enforcing","cap","in_flight","dispatched","rejections","controller_steps"}, "requests_total","cache_hits","cache_misses","coalesced_waiters","stale_served_total","stale_cap_violations","bytes_resident","evictions_total"}`
Source: `GET /api/v1/metrics` (the schema-2 payload that `/api/events?schema=2` streams; plain JSON is used instead of SSE). A value the gateway reports as unavailable is `null` (never 0); `origin_cap.cap` is `null` when uncapped.

## run_metrics.json

Per-run derived metrics. The analysis window is `scheduled_ms >= warm-up` (client) and arrival offset `>= warm-up` (origin). Key fields: `n_scheduled_window`, `n_with_response_window`, `n_no_response_window`,
`success_rate` (2xx / scheduled; 503 shed and 5xx count as failures), `shed_fraction` (503 with `X-Cache-Error: CapacityExceeded`), `other_5xx_fraction`, `status_counts`, `cache_status_counts`,
`client_p50/p90/p95/p99/max_ms` (nearest rank on raw samples), `origin_peak_window` / `origin_peak_degraded` / `origin_peak_whole_run`, `origin_p99_service_ms`, `origin_requests_window`,
`cap_limit`, `mechanism_violated` (whole-run origin peak > cap limit), gateway fields (`gateway_cap_min/max`, `gateway_in_flight_max`, `gateway_rejections_final`, ...), recovery fields (`error_rate_healthy`, `recovery_error_s`, `recovery_in_flight_s`),
`send_lag_p99_ms`, `contention_flag` (send-lag p99 > 100 ms), `valid` + `invalid_reasons` (infrastructure soundness only: gateway alive, enforcement verified, every scheduled request sent, snapshots present; outcomes never decide validity), `gateway_exit_status` (the child is killed, so a non-zero exit code is expected).

## summary.json (schema version 2)

`summary_version` (2), `tier` (MEASURED_LOOPBACK), `label` (EXPLORATORY for incomplete runs; VALID only if planned VALID and complete), `label_planned`, `label_reasons`, `status` (COMPLETE | CANCELLED), `run_completeness` (`complete`, `incomplete_reason`, `planned_runs`, `completed_runs`), `verdicts_status` ("computed" | "not_computed"), `verdicts_reason` (null | explanation), `provenance` (what ran; measured / derived / simulated lists), `config`, `limitations`, and:

- `arms.<ARM>`: runs, valid runs, pooled client latency percentiles with sample count `n` (pooled over valid runs, analysis window), per-run means, status and cache-status totals, `recovery_h_b4`.
- `mechanism_h_b1.<ARM>`: `cap_limit_in_force`, `max_origin_concurrent_observed_whole_run`, `violating_reps`, `pass` (boolean for complete runs, null for incomplete runs); `mechanism_h_b1_all_pass` (boolean for complete runs, null for incomplete runs).
- `paired_vs_base.<ARM>.<metric>`: `n_pairs`, means, `mean_diff_treat_minus_base`, `diff_ci95_bca`, `relative_reduction_mean`, `relative_reduction_ci95_bca`, `permutation_p_sign_flip` (95% BCa with 10,000 resamples and the sign-flip permutation test from `src/lab/analysis`, applied to per-repetition paired values; pairs where either arm's run is invalid are dropped).
- `verdicts.<ARM>`: On completed runs: object with `outcome` (MECHANISM_FAILED | INCONCLUSIVE | BENEFIT_NOT_SUPPORTED | TRADE_OFF_FAILURE | BENEFIT_SUPPORTED), `reasons`, `evidence` (the exact inputs of the rule). On incomplete or cancelled runs: `verdicts` is strictly `null` (never an empty object or false verdict). The rule is `lab_live::analysis::evaluate_arm` (unit-tested for every outcome). Precedence: mechanism failure; fewer than 10 valid pairs; contention; CI includes zero and its upper bound >= 5% (cannot exclude a 5% effect: inconclusive); reduction < 5% or CI not excluding zero (not supported); success more than 1 pp below BASE (trade-off failure); origin p99 worse (not supported); otherwise supported. The "cannot exclude 5%" reading of the contract's inconclusive clause is an interpretation made explicit here.
