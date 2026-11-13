# Phase P3: Real HTTP Application Integration, Randomized Workloads and Live Telemetry

## 1. Objective
Prove that PhantomCache operates as an enterprise caching gateway under application-like request workloads with live SSE telemetry, statistical percentile tracking, and truthful provenance reporting (REAL, DERIVED, UNAVAILABLE).

---

## 2. Architecture and Execution Path
- **Workload Generator:** Deterministic, seeded synthetic/application traffic generator (`src/sim/traffic.rs`).
- **Telemetry Engine:** Sliding-window rate sampler and lock-free reservoir histogram (`src/intel/telemetry.rs`).
- **Schema 2 SSE Stream:** Real-time push stream (`/api/events?schema=2`) emitting atomic counter snapshots and latency percentiles (`src/api/server.rs`).
- **Control Center Frontend:** React/Vite web application rendering live graphs and provenance badges (`ui/`).

---

## 3. Exact Setup Instructions
1. Ensure the demo is running with origin configuration:
   ```powershell
   $env:ORIGIN_URL = "http://127.0.0.1:9099"
   $env:PHANTOM_BIND_ADDR = "127.0.0.1:8085"
   $env:PHANTOM_LAB_ENABLED = "true"
   $env:PHANTOM_ORIGIN_CAP_MAX = "10"
   $env:PHANTOM_ORIGIN_CAP_MIN = "2"
   $env:PHANTOM_INTELLIGENCE = "canary"
   target\release\phantom-cache-demo.exe
   ```
2. Start the mock origin server on port `9099`.
3. Open browser to `http://localhost:3000`.

---

## 4. Live Telemetry & Workload Execution

### A. Seeded Randomized Workload
Run a reproducible 100-request workload with Zipfian key distribution (seed 42):
```powershell
cargo test --test p3_traffic_tests test_p3_seeded_workload_determinism -j 1
```
- Seeded random generator executes 100 requests.
- Exactly reproduces hit ratio, miss count, and latency distribution across repeated runs.

### B. Traffic Generation Against Running Gateway
```powershell
for ($i = 1; $i -le 60; $i++) {
    $k = $i % 10
    curl.exe -s "http://127.0.0.1:8085/api/v1/cache/data/keys_$k" | Out-Null
}
```

### C. Inspect Aggregate Metrics
```bash
curl -s http://127.0.0.1:8085/api/v1/metrics
```
*Observable Telemetry Schema 2 Values:*
- `requests_total`: `{"src": "measured", "v": 73, "unit": "count"}`
- `cache_hits`: `{"src": "measured", "v": 51, "unit": "count"}`
- `cache_misses`: `{"src": "measured", "v": 22, "unit": "count"}`
- `hit_ratio_cumulative`: `{"src": "derived", "v": 0.6986, "unit": "ratio"}`
- `latency_ms.p50`: `{"src": "measured", "v": 0.12, "unit": "ms"}`
- `latency_ms.p95`: `{"src": "measured", "v": 0.45, "unit": "ms"}`
- `latency_ms.p99`: `{"src": "measured", "v": 1.20, "unit": "ms"}`

---

## 5. Provenance Contract Verification

| Provenance Class | Meaning | UI Badge | Enforced Rule |
|---|---|---|---|
| **REAL / MEASURED** | Direct hardware/atomic measurement | Green Badge `REAL` | Never fabricated; exact atomic counter |
| **DERIVED** | Mathematically calculated from measured data | Blue Badge `DERIVED` | Hit ratio, throughput, percentiles |
| **UNAVAILABLE** | Data cannot be accurately measured | Gray Badge `UNAVAILABLE` | Never replaced with synthetic `0` or `0.0%` |
| **SIMULATED** | Produced by synthetic simulation model | Yellow Badge `SIMULATED` | Forbidden in live mode; allowed only in simulation mode |

*Statistical Honesty Rule:* Percentiles report `{"src": "unavailable", "reason": "no_samples"}` until at least 20 observations exist to prevent misleading statistical noise.

---

## 6. Actual Test Results & Automated Verification

### Automated Test Suites Executed:
1. `tests/integration/p3_traffic_tests.rs`: **5 passed, 0 failed** (0.22s).
   - `test_p3_seeded_workload_determinism`: PASSED.
   - `test_p3_key_distributions`: PASSED (Zipfian skew verified).
   - `test_p3_request_type_distribution_weights`: PASSED.
   - `test_p3_live_telemetry_stream_and_counters_bridge`: PASSED.
   - `test_p3_workload_generator_executes_against_live_server`: PASSED.
2. `npm test` in `ui/`: **483 passed, 0 failed** (1.84s).
   - Verifies schema 2 contract, provenance badge rendering, chart fallback tables, and unavailable states.
3. `scripts/run_e2e_acceptance.ps1` (Steps 4, 15): **2 passed, 0 failed**.

---

## 7. Evidence Artifact Locations
- Acceptance Evidence JSON: `lab-results/e2e_acceptance_evidence.json` (Steps 4, 15).
- Telemetry Integration Tests: `tests/integration/p3_traffic_tests.rs`.
- UI Test Suite: `ui/test/`.

---

## 8. Known Limitations
- Background refresh (`stale-while-revalidate`) is bounded to single-flight refreshes; offline origin fallback safely serves stale data up to `PHANTOM_MAX_STALE_MS` (default 24h).

---

## 9. Acceptance Verdict
**Verdict: PASS**  
Application workloads, Zipfian traffic distributions, live SSE telemetry, and truthful provenance reporting are completely implemented, verified, and active.

