# Scenario B — dated interpretation and correction notice (2026-10-30 07:14 +05:30)

Applies to the retained experiment `lab-results/scenario_b/20261030T003932Z/` (unchanged; no artifact was edited). Sources: independent fairness review, independent recomputation (`docs/demo/scenario_b_verification/`), lead re-derivation from raw CSVs. This notice supersedes any looser wording in `summary.json` text fields, the runbook or earlier ledger entries.

**Verified:** both capped arms are mechanically `TRADE_OFF_FAILURE` (contract rule reproduced independently). Enforcement of the cap was mechanically correct for the static arm (origin peak exactly 24 in all 10 runs).

**Qualifications (must accompany any citation of these results):**
1. The nominal adaptive arm (AIMD 12-32, ceiling 60 ms, window 30) fell from 32 to about 12-13 within ~2 s and stayed near 12 in every phase; it also shed ~2% in the healthy phase. The comparison is effectively static cap ~12 versus static cap 24. It does not demonstrate adaptive-controller effectiveness or ineffectiveness.
2. The controller's 30-sample "p99" is the window maximum (nearest rank), compared with a 60 ms ceiling that is sensitive to timer overshoot and healthy-origin variation.
3. Latency percentiles over all requests include fast-fail 503s and are NOT evidence of improved latency for served clients. Success-only latency must be reported separately (AIMD 200-only P99 difference -2.78 ms, 1.66%; STATIC no evidence of an effect). The all-request P99 figures 2.6% / -4.42 ms are the same quantity (mean of paired per-run differences over mean BASE P99).
4. The scripted origin has no load-dependent latency: the contract's origin-p99 clause is untestable here; observed origin-p99 differences are timer noise.
5. Stale-serving code exists but the workload did not exercise it (origin sent only `max-age=3`); stale-while-revalidate / stale-if-error were not run; `stale_served_total` = 0.
6. Cap values (static 24; AIMD 12-32) were chosen after a single-repetition pilot (`D:\Temp\sb_pilot1`, same seed as repetition 1, different gateway binary hash - reason unestablished) and were not pre-registered. Thresholds (5%, 1 pp, >= 10 runs) were not changed.
7. The main-run gateway binary matches its manifest; `src/lab_live/scenario_b.rs` now differs from the recorded source hash (post-run clippy-only edit claimed; no pre-edit copy exists; UNVERIFIED). The driver executable predates the edit.
8. Arm fairness (canary vs off) is excluded by code reading only; no cap-free canary arm or overhead measurement exists. Tier `MEASURED_LOOPBACK`, label `VALID` mean only: release binaries (path heuristic), >= 10 repetitions, BASE present, one host, scripted origin.

**Permitted conclusion:** On this single-host release-gateway workload, static cap 24 reduced peak origin concurrency from about 39 to 24 (-38.3%, repetition-level 95% CI 36.6-40.1%) at a client-success cost of 4.95 percentage points (all in the degraded phase). The effectively ~12-cap arm reduced peak concurrency by 64.0% at a 17.1-point success cost.

**Not established:** adaptive-control merit; production, WAN or multi-host behaviour; origin-p99 benefits; stale-serving effectiveness; pre-registered cap selection; a served-client latency improvement.

**Future work (hypothesis only):** a bounded wait queue (and stale-on-saturation with SWR/SIE origins) may reduce shedding at cap 24 but cannot rescue cap ~12 (throughput ~89 rps vs ~180 rps demand). It must be evaluated under a NEW pre-fixed protocol and must not be used to reinterpret or modify the completed Scenario B results.
