# Scenario B (T2) runbook

Tier: MEASURED_LOOPBACK. Implementation: `src/lab_live/`, command `phantom-lab scenario-b`. Artifact schema: `docs/demo/scenario_b_artifacts.md`.

## Build (Windows, this repository)

```
set TEMP=D:\Temp & set TMP=D:\Temp & set CARGO_TARGET_DIR=D:\Personal\_m7_target
cargo build --release -j 1 --bin phantom-cache-demo --bin phantom-lab
```
Reported numbers require the RELEASE gateway and RELEASE `phantom-lab`; any debug component makes the experiment label EXPLORATORY.

## Run

```
D:\Personal\_m7_target\release\phantom-lab.exe scenario-b --out lab-results\scenario_b\<utc>
```
Defaults (the documented experiment): 10 repetitions x 3 arms (interleaved, rep-major, arm order rotated each repetition); seed 20261030 (+ repetition); Poisson arrivals at 300 req/s, Zipf alpha 0.9 over 5000 keys, 1 KiB bodies;
gateway cache budget 262144 B (the catalogue does not fit, misses persist); origin base service 20 ms, `Cache-Control: max-age=3`;
phases (relative to load start): warm-up 8 s (excluded), healthy 5 s, degraded 10 s (service time x6 = 120 ms, error rate 0), recovery 8 s.
Arms: BASE (`PHANTOM_INTELLIGENCE=off`, no cap), STATIC-CAP (`canary`, min = max = 24), AIMD-CAP (`canary`, min 12, max 32, p99 ceiling 60 ms, window 30).
One run takes about 31 s plus start-up; the full experiment about 17 minutes. Start it in the background and log to a file.

Useful options: `--smoke MS` (1 repetition, 4 phases of MS ms; mechanics check), `--reps`, `--arms`, `--rate`, `--keys`, `--alpha`, `--mult`, `--error-rate`, `--max-age`, `--swr S` / `--sie S` (origin sends stale-while-revalidate / stale-if-error),
`--static-cap`, `--aimd-*`, `--gateway PATH`. Full list: `phantom-lab scenario-b --help`. Ctrl-C cancels: the child is killed, manifest status CANCELLED, exit code 130.

## What is verified before measuring

For every run the driver reads `/api/v1/metrics` after the gateway is ready: capped arms must report `origin_cap.enforcing = true` and `cap` equal to the configured value; BASE must report `enforcing = false` and no cap. Otherwise the whole experiment aborts (exit 1, manifest FAILED) with the gateway log tail. Every snapshot of a capped run must also report `enforcing = true`.

## Host hygiene

One experiment at a time, nothing else heavy on the machine, no browsers. Runs whose load-generator send lag p99 exceeds 100 ms are flagged `contention_flag`; any flag makes the verdict INCONCLUSIVE.
Timer resolution of the host is recorded in the manifest (`timer_probe`). Do not compare these numbers with T1 (SIMULATED) or with physical-host results.

## Tests

`cargo test -j 1 --test lab_live_scenario_b_tests` (unit tests for schedule determinism, Zipf sampler, percentiles, the mechanical rule, run metrics, origin degradation; end-to-end smoke with 3 real debug gateway processes; cancellation; about 40 s). `-- --ignored` adds a 10-repetition short-phase run.
