> Notice (2026-10-30): The Status line below ("CONTRACT ONLY ... Nothing below is implemented as a demonstration yet") is out of date. Scenario B has since been implemented: `src/lab_live/` (`scenario_b.rs`, `analysis.rs`), the `phantom-lab scenario-b` command ([runbook](scenario_b_runbook.md), artifact schema [scenario_b_artifacts.md](scenario_b_artifacts.md)), the Lab API routes in `src/api/lab_api.rs`, and one retained experiment, `lab-results/scenario_b/20261030T003932Z/`.
>
> Any citation of that experiment must carry the qualifications in [the dated interpretation notice](scenario_b_interpretation_notice.md), which is binding for those results. This contract is otherwise unchanged and remains the specification the implementation was written against.
>
> Original text below is unchanged.

# Scenario B — Origin slowdown and cascading pressure (acceptance contract)

**Status:** CONTRACT ONLY (2026-10-30). Nothing below is implemented as a demonstration yet. It is the specification that the implementation, the Lab API and the UI must satisfy. A result is only recorded in this file after a real run, with its artifacts.

## 1. Real-world problem
A downstream service slows down. Cache misses and expired entries keep arriving, so concurrent requests pile onto the already slow origin, raising its latency further (positive feedback). Caching alone does not prevent this; request coalescing removes duplicate fetches for the *same key*, but misses for *different keys* still multiply.

## 2. Hypotheses (falsifiable)
- **H-B1 (mechanism, hard invariant):** with the origin concurrency cap enabled (mode Canary/Active with `PHANTOM_ORIGIN_CAP_*`), the number of simultaneous origin connections never exceeds the cap in force. *Falsified by any observation above the cap.* (R-01 tests already check this in isolation; the scenario must show it end to end.)
- **H-B2 (benefit):** under origin latency inflation, enabling the cap lowers peak origin concurrency and origin service latency relative to the identical workload without the cap. *Falsified if the paired difference of peak concurrency or origin p99 is not lower, or its 95% CI includes zero.*
- **H-B3 (cost, must be reported whatever the sign):** the cap sheds some requests (503 or stale) and may change client p99 and error rate. *The scenario reports the shed fraction and the client success rate; a benefit on origin load that costs more client errors than the pre-set limit is reported as a failure of the trade-off, not hidden.*
- **H-B4 (recovery):** after the origin returns to normal, cap, in-flight count and error rate return to baseline within a measured time. *Falsified if they do not return within the run.*
- **H-B5 (stale serving, only where the origin sends stale-while-revalidate / stale-if-error):** entries inside their stale windows are served as STALE during degradation instead of failing, bounded by the hard stale cap.

## 3. Preconditions and supported capabilities (verified in code)
Single-flight coalescing; stale serving with hard cap (H2 fix, sweeper retains entries inside stale windows); R-01 origin cap with AIMD controller fed by measured origin latency/errors, active only in Canary/Active with a configured cap; `metrics.origin_cap` telemetry (`enforcing`, `cap`, `in_flight`, `dispatched`, `origin_cap_rejections`). **Not available:** adaptive stale-window control and admission in the serving path (lab-only, T1); PIE decision records from the serving path. The scenario must not show them as live.

## 4. Tiers (always labeled in the UI and in every artifact)
| Tier | What runs | Label |
|---|---|---|
| T1 | `phantom-lab` virtual-time simulation using the real cache/executor against the lab simulated origin; scenarios O-DEG, O-BRN, O-SLW, O-RT | SIMULATED (virtual time) |
| T2 | the real gateway process (`phantom-cache-demo`) on loopback, a scripted raw-TCP origin with a degradation schedule, and an open-loop load generator in real time | MEASURED_LOOPBACK (single host, shared CPU) |
Arms (T2): **BASE** = single-flight only (cap unconfigured, intelligence off); **STATIC-CAP** = cap fixed (min = max = C); **AIMD-CAP** = cap range [min, max] with the controller. Arms (T1): BL-1, BL-4, BL-12b, PIE-Retained (simulated, labeled as research machinery).

## 5. Workload, fault injection, reproducibility
- Open-loop arrivals (no coordinated omission), Zipf key popularity over N keys, seeded; a fixed catalogue size larger than the cache so misses persist; request rate, duration and seed recorded.
- Origin script (T2): healthy → degraded (service time ×k for D seconds, optional error rate p) → healthy; offsets relative to run start; the origin records every request (arrival time, concurrent connections at arrival, service time, status).
- Repetitions: n ≥ 10 independent runs per arm (T2 is real time and not bit-deterministic); T1 uses paired dev seeds D. Sealed seeds S are never used by this demonstration.
- Controls: same seed and arrival schedule across arms; arms run interleaved (ABAB…) to spread host drift; warm-up excluded and recorded; process, profile (release for reported numbers), host and CPU recorded.

## 6. Collected events and metrics
Per client request: scheduled and actual send time, completion time, latency, HTTP status, `X-Cache-Status`. Per origin request: arrival, concurrent connections at arrival, service time, status. Gateway: periodic `/api/events` snapshots (`metrics.origin_cap`, origin metrics). Percentiles are computed from the raw per-request samples (not from fixed delays); sample counts are always reported. No missing value is replaced by zero.

## 7. Success / failure / inconclusive criteria (fixed before the first reported run)
- **Mechanism pass:** max observed origin concurrent connections ≤ cap in every STATIC-CAP/AIMD-CAP run.
- **Benefit supported:** paired relative reduction in peak origin concurrency ≥ 5% with the 95% CI excluding zero **and** origin p99 not worse **and** client success rate not lower than BASE by more than 1 percentage point (shed 503s count as failures).
- **Benefit not supported:** reduction < 5%, or CI includes zero → reported as not supported.
- **Trade-off failure:** benefit on origin load but client success rate more than 1 pp below BASE → reported as a failure of the trade-off.
- **Inconclusive:** fewer than 10 valid runs, run-to-run variance too large for the CI to exclude a 5% effect, or host contention flagged → reported as inconclusive with the missing evidence stated.

## 8. Evidence artifacts (retained per run, referenced by the UI)
`manifest.json` (source hashes, git HEAD + dirty flag, command line, seed, workload and origin script, profile, host, timestamps, exit code), `client_requests.csv`, `origin_requests.csv`, `gateway_snapshots.jsonl`, `summary.json` (per arm: sample counts, percentiles, shed fraction, success rate, paired differences with CIs), and a `provenance` block labeling tier and measured/derived values.

## 9. Known limitations (stated in the UI next to any result)
Loopback on one host (no network RTT; load generator, gateway and origin share CPU), limited key catalogue, scripted origin rather than a real service, T1 is virtual time, debug-versus-release differences, clock resolution, results apply only to the tested workload family and degradation shape; no claim about production, WAN or multi-host behaviour; the adaptive stale-window/admission mechanisms are not part of the live gateway.

## 10. Implementation tasks implied (tracked in the ledger)
(1) T2 scenario runner inside `phantom-lab` using only existing runtime dependencies (no reqwest: a minimal HTTP/1.1 client over `tokio::net::TcpStream`); (2) scripted degradation origin with per-request recording; (3) open-loop load generator with raw sample output; (4) analysis (paired differences, BCa CI) reusing `src/lab/analysis`; (5) Lab HTTP API (start/progress/cancel/results) invoking the same runner; (6) Phantom Lab UI view reading retained results and live events; (7) browser verification at four viewport sizes in both themes.
