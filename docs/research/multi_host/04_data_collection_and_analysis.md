# Multi-host validation: data collection and analysis

**Status:** Draft for owner review. **Audience:** operator, analyst, independent reviewer. **Last reviewed:** 2026-10-30 18:18 (+05:30, machine clock).

Tags: FACT (file:line), PROPOSAL, UNVERIFIED. No script described here exists; section 6 lists what to write later.

---

## 1. Evidence layout (reuses the Lab manifest approach)

The Lab layout for Scenario B is the model (FACT `docs/demo/scenario_b_artifacts.md`): an experiment-level `manifest.json` written at start with status RUNNING and rewritten at the end, a `summary.json`, and one directory per run. The multi-host study adapts it:

```
multi-host-results/<study-id>/<utc>/            (outside the repository; owner decides where it is archived)
  prereg/preregistration_<id>.md                 frozen thresholds; sha256 recorded in manifest
  manifest.json                                  experiment-level provenance
  SHA256SUMS                                     hashes of every file below (itself hashed into manifest.json)
  hosts/<host>/hardware.json                     CPU, RAM, NIC, link speed, MTU, OS, kernel/build, disk type
  hosts/<host>/clock_samples.csv                 offset samples before, during, after
  runs/<exp>_rep<NN>_<arm>/
    trials.csv                                   one row per (trial, peer): see section 2
    client_requests.csv                          background workload (Scenario B schema, section 2.3)
    origin_requests.csv                          origin-side arrivals and concurrency
    node<k>/gateway.log                          full stdout/stderr (not truncated)
    node<k>/metrics.jsonl                        /api/v1/metrics snapshots (250 ms)
    node<k>/netstats_before.txt, netstats_after.txt
    faults.jsonl                                 one record per fault event (02, section 7)
    verification.jsonl                           fault verification outputs
    run_metrics.json                             derived per-run metrics
  summary.json                                   experiment-level analysis and verdicts
  run_log.md                                     operator log, deviations, cleanup sign-off
```

## 2. Files and fields

### 2.1 `manifest.json`

Reuse the Scenario B fields where they apply (FACT scenario_b_artifacts.md): `manifest_version`, `scenario`, `status` (RUNNING, COMPLETE, CANCELLED, FAILED), `exit_code`, `started_utc`, `ended_utc`, `command_line`, `seed` (and `fault_seed`), `label_planned` + `label_reasons`, `profile` (`driver_profile`, `gateway_profile`), `gateway` (`exe`, `exe_sha256`, environment keys set; secrets redacted), `git` (`head`, `dirty_entries`), `source_sha256`, `host`, `workload`, `phases`, `extra.timer_probe`.

Additions (PROPOSAL): `study_id`, `tier_label` (section 5 of `00`), `build` (`G` or `M3`, patch sha256 for M3, independent-review status at run date), `topology` (option, node count, roles to host mapping), `hosts[]` (hostname alias, role, OS, CPU, cores, RAM, NIC, link speed, MTU, IP, disk type), `network` (switch model, VLAN, isolation check result, RTT matrix, loss check), `clock` (sync daemon, max observed offset per host, method), `prereg_sha256`, `tools` (versions of tc/iptables/clumsy/iperf/openssl), `fault_catalog`, `operators`, `cleanup_verified` (bool, who, when). The label rule for Scenario B (`VALID` only for release gateway + release driver + >= 10 repetitions + baseline present) is adopted: anything else is `EXPLORATORY` with reasons.

### 2.2 `trials.csv` (propagation, E1 to E3, E7, E8)

`trial_id, exp, rep, arm, method(P1|P2), issuer, peer, key, origin_version_before, origin_version_after, t0_ns, t_delete_sent_ns, t_204_ns, status_delete, t_first_nonstale_ns, status_after, x_cache_after, version_after, stale_polls_before_conv, polls_after_conv, violation_count, propagation_ms (= t_first_nonstale - t_204), stale_window_ms (same definition; kept as a separate column so the definition can be changed without losing the raw timestamps), path (cold|warm), valid, invalid_reason`.

All timestamps are on the observer's monotonic clock for method P1. For method P2 add `peer_clock_offset_ms` (measured) and use `t_204` from the issuer's local clock (the DELETE is issued by a local agent). Raw timestamps are never discarded.

### 2.3 `client_requests.csv`, `origin_requests.csv`, `gateway_snapshots`

Schema as Scenario B (FACT scenario_b_artifacts.md): `idx,key,scheduled_ms,send_ms,done_ms,send_lag_ms,latency_ms,service_latency_ms,status,x_cache_status,x_cache_error,bytes,error,in_analysis_window` (latency = done minus scheduled; status 0 = no response, counted as a failure); origin `arrival_ms_since_load_start,conc_at_arrival,service_ms,status,degraded,path_hash`. Add `target_node` to the client file. Metrics snapshots: `GET /api/v1/metrics` JSON with `nodes[]`, `origin_cap`, `requests_total`, `cache_hits`, `cache_misses`, `stale_served_total`, `stale_cap_violations`, `bytes_resident`, `evictions_total` (FACT same file; a value reported as unavailable is `null`, never 0).

### 2.4 `faults.jsonl`

One object per event: see `02`, section 7.

### 2.5 `node_states.csv` (E4 to E6)

`t_ns, observer_node, target_node, state` from `nodes[]` of each node's metrics, 100 ms cadence. Derive `T_detect` (fault start to first non-Active) and recovery time (fault end to Active). The state labels are the gateway's own view and are labelled "inferred" by the gateway (FACT harness comment `cluster_harness.rs` above `node_states`, around line 703).

### 2.6 `identity_cases.csv` (E10)

`case_id, node, change, restart_nodes, expected, observed_log_line, observed_state, admitted(bool), notes`. Admission of an invalid identity is the highest-severity finding.

### 2.7 Clock evidence

`clock_samples.csv`: `host, t_ref_utc, offset_ms, source, method`. P2 results are valid only if `max(abs(offset))` over the run is within the bound pre-registered in `00` (6.4).

## 3. Hashing and retention

- `sha256sum` (or `Get-FileHash`) over every file; `SHA256SUMS` hashed into `manifest.json`; the manifest hash and the preregistration hash are recorded in the owner's decision/evidence log. The binary hash and `git` state are in the manifest.
- Write-once: after COMPLETE nothing in the run directory is edited; corrections go in a new file with a pointer.
- Keys, CA key and JWT secret are never in the bundle. Certificates and their fingerprints are.
- Retention location and duration are an owner decision (`05`, D10). Raw logs may contain IPs; the bundle is private unless the owner publishes a redacted subset.
- Rule from the repository: do not touch frozen/hash-pinned evidence and never read the sealed M6 directory. Study evidence is separate from it.

## 4. Analysis

### 4.1 Cleaning (decided in advance)

- Drop invalid trials only by the rules in `00` 6.4; list every dropped trial with its reason in `exclusions.csv`.
- The cold path (first invalidation to a destination in a session) is stored with `path=cold` and analysed separately.
- Non-200 responses during polling are not convergence (FACT harness behaviour); count them and report their rate.

### 4.2 Propagation distribution (E1, E3, E7)

For each ordered pair and each condition:
1. Report n, min, P50, P90, P99, P99.9 (only if n >= 1,000), max (nearest rank on raw values, the same estimator the Lab uses, FACT `src/lab_live/loadgen.rs:163`).
2. P99 interval: percentile bootstrap, 10,000 resamples, resampling whole **repetitions** first and trials within repetitions second (a two-level bootstrap), because trials inside one session are not independent (shared connection state, shared clock). Report the interval and the method. If the number of repetitions is below 10 the interval is labelled unreliable.
3. Compare conditions (fault versus no-fault control; physical versus loopback) with the difference of medians and of P99 with bootstrap intervals; avoid p-values as the primary result; if one is reported, state the test and that it is exploratory.
4. Plot the empirical CDF with a log time axis for every pair on one figure with the threshold drawn. Show all raw points for the tail (rug or strip). Label the tier on the plot.
5. Check the threshold exactly as pre-registered: `upper CI of P99 <= T_prop` AND `max <= T_prop_max`.

### 4.3 Count outcomes (E2 violations, convergence failures, identity admissions)

Report `k of n` with the Clopper-Pearson 95 percent interval. For `k = 0` state the upper bound (about `3/n`), never "zero errors". A single violation of a hard threshold ends that experiment as FALSIFIED for the stated condition, regardless of the interval.

### 4.4 Time-to-X metrics (E4 to E6, E8, E8m)

`T_detect`, `T_heal`, `T_replay`: per trial values, medians with bootstrap intervals, and the maximum. For E8 report the proportion of trials with the peer still stale at 10 s and the stale duration (bounded by the origin's `max-age`; if the observation window ends first, the value is right-censored: report it as "> window", and use a censored-data estimate, for example Kaplan-Meier, not a mean).

### 4.5 Throughput and latency (background workload)

Client latency percentiles from `latency_ms` (scheduled to done), by phase; send-lag P99 per run with the 100 ms contention flag (FACT scenario_b_artifacts.md); hit ratio from the status mix; origin peak and P99 service time from `origin_requests.csv`. Single-node baseline alongside.

### 4.6 E11

Per arm and repetition: origin peak concurrency (whole run and degraded window), shed fraction, client P99 in the degraded window, `origin_cap` enforcement flags. Compare the arm means with intervals; the mechanical rule from Scenario B (cap bounds origin concurrency) is the only one inherited; claims about benefit are not made without the pre-registered comparison, and the M6 result (no benefit from adaptivity, FACT claims_ledger.md section 6) is not tested here.

### 4.7 Cross-tier comparison

Present the physical table beside the loopback rehearsal table with a clear tier column. State the ratio of P99 values as an observation, never as a correction factor. Do not add them.

## 5. Reporting negative and null results

1. Every pre-registered experiment gets a row in the results table with outcome one of: **NOT FALSIFIED** (survived this test under these conditions), **FALSIFIED** (a falsifier occurred), **INCONCLUSIVE** (insufficient or invalid data, or interval straddles the threshold), **NOT RUN** (with reason). "Confirmed" and "proved" are not used.
2. A falsification is reported with the same prominence as a pass: first, in the summary, with the trial ids, raw values, the fault conditions and a link to the raw logs. The claims ledger is updated by the owner/lead after review; this study does not edit it.
3. Unexpected observations (for example a gateway hang, an unexplained 5xx, a state that never returned to Active) are reported as findings even if outside the hypotheses.
4. If the whole physical study contradicts the loopback result, say so first in the executive summary.
5. Report what was not tested and what the design could not detect (`00` section 4).
6. Failed or aborted sessions are listed with their cause; they are not rerun silently. A rerun is a new session with a new id; both appear.
7. Deviations from the pre-registration are listed with date and reason; results are shown under the registered rule and, separately, under the amended rule.
8. Wording rules: no linearizability, no "bounded staleness" or "guarantee" without failure model, bound and the test that could falsify it (CLAUDE.md section 4). Measurements carry tier, build, hardware, OS, n, seed and interval.

## 6. Tools to write later (list only; none is written)

Writing any of them is a code addition and needs owner approval (CLAUDE.md section 2); none should modify the gateway.

| # | Tool | Purpose | Contract |
|---|---|---|---|
| T1 | `mh-origin` | Standalone versioned origin on its own host | HTTP GET `/<path>` returns JSON `{ "version": N }` plus `Cache-Control: max-age=<cfg>`; control endpoint to set a per-key or global version atomically and to log the version change time; optional scripted service time and degradation phases like Scenario B (base service 20 ms, x6 degraded); request log in `origin_requests.csv` format with concurrency at arrival. Reuses the logic of `VersionedMockOrigin` (test code) and `src/lab_live` origin. |
| T2 | `mh-probe` | Remote load generator and propagation prober | Open-loop Poisson/Zipf scheduler (reuse the Lab sampler and CSV schema), targets a list of nodes, JWT bearer, `trial` subcommand implementing 6.1 of the runbook, writing `trials.csv` with monotonic timestamps, per-poll interval 1 ms, flags non-200 separately. Must report its own send lag. |
| T3 | `mh-token` | Mint HS256 JWTs for the lab secret | Claims `sub, iss, aud, exp, iat, tenant_id, roles` (FACT `src/security/jwt.rs:31`); roles `cache:read`, `cache:invalidate`, `telemetry:read` (FACT docs/user/security.md); short expiry; prints the token only. |
| T4 | `mh-pki` | Cluster PKI and membership generator | Mirror `generate_cluster_pki` (FACT `cluster_harness.rs:198` to `:266`): CA, per-node leaf with SAN `node-<id>.<cluster>.phantom.internal`, fingerprints, `members.json` with `version`, rotation and revocation variants (add/remove fingerprint, `active:false`, version bump, expired cert). |
| T5 | `mh-faultd` / wrapper around `FaultProxy` | Cross-host proxy for the loopback/WSL2 rehearsal and optional physical use | Standalone binary around `tests/support/fault_proxy.rs`; control port to set mode; adds a random-loss mode (does not exist). |
| T6 | `mh-orchestrate` | Controller scripts | Start/stop nodes via SSH/WinRM, apply and verify faults with watchdogs, run the experiment list, collect files, hash, write the manifest, perform the cleanup check. |
| T7 | `mh-analyze` | Analysis | Implements section 4: tables, bootstrap, Clopper-Pearson, Kaplan-Meier, CDF plots, exclusions report, threshold verdicts from the preregistration file. |
| T8 | `mh-verify-bundle` | Evidence verification | Recompute `SHA256SUMS`, check the manifest/prereg hashes, check required files per experiment. |
| T9 | `mh-keyscan` (E8 only) | Find a key whose canonical primary is a chosen node | Needs the gateway's rendezvous router (library code) so the key choice matches production (FACT: the sandbox did this, reproduction_report.md section 2). |
| T10 | Clock-offset sampler | Periodic offset capture per host | Wraps `chronyc tracking` / `w32tm`. |

Design note: the existing Lab runner cannot be pointed at remote gateways (it spawns local gateways, FACT `scenario_b.rs:155`, flag list in `cli.rs:31` to `:34`). T2 and T1 should therefore be new binaries or a new subcommand sharing the sampler, schema and percentile code, not a modification of Scenario B's behaviour.
