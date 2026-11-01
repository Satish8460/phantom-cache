# P1–P4 Unified Demonstration Runbook

This runbook guides an operator or evaluator through the complete, reproducible end-to-end demonstration of PhantomCache across all four engineering phases:
- **P1:** Hardening, Resource Safety and Malicious-Input Safeguards
- **P2:** Fault Injection, Distributed Resilience and Network Failure Handling
- **P3:** Real HTTP Application Integration, Randomized Workloads and Live Telemetry
- **P4:** Adaptive Policy Selection, Runtime Switching and Guardrails

---

## 1. Quick Automated Acceptance Run (One Command)
To execute the automated 26-step verification covering all phases in under 30 seconds:

```powershell
pwsh -File scripts/run_e2e_acceptance.ps1
```

*Expected Terminal Summary:*
```text
==========================================================
 ACCEPTANCE SUITE FINISHED: 26/26 STEPS PASSED in ~24s
 Machine-readable evidence written to: lab-results\e2e_acceptance_evidence.json
==========================================================
```

---

## 2. Interactive Operator Demonstration Walkthrough

### Step 1: Clean Environment Launch
1. Ensure ports `8085`, `3000`, and `9099` are not bound.
2. Launch the PhantomCache Demo with origin and Lab enabled:
   ```cmd
   Start-Phantom-Demo.bat
   ```
   *(Or in PowerShell with environment variables:)*
   ```powershell
   $env:ORIGIN_URL = "http://127.0.0.1:9099"
   $env:PHANTOM_BIND_ADDR = "127.0.0.1:8085"
   $env:PHANTOM_LAB_ENABLED = "true"
   $env:PHANTOM_ORIGIN_CAP_MAX = "10"
   $env:PHANTOM_ORIGIN_CAP_MIN = "2"
   $env:PHANTOM_INTELLIGENCE = "canary"
   target\release\phantom-cache-demo.exe
   ```
3. Start the mock origin server on port `9099`:
   ```powershell
   pwsh -File scripts/verify_real_cache_ops.ps1
   ```
4. Access the **PhantomCache Control Center** at `http://localhost:3000`.

---

### Step 2: P1 Demonstration — Cache Operations & Coalescing
1. **Cold GET (Cache Miss):**
   ```bash
   curl -i http://localhost:8085/api/v1/cache/catalog/product_101
   ```
   Observe in response header: `X-Cache-Status: MISS`.
   In UI (**Traffic & Latency**): `cache_misses` counter increments to 1.
2. **Warm GET (Cache Hit):**
   ```bash
   curl -i http://localhost:8085/api/v1/cache/catalog/product_101
   ```
   Observe in response header: `X-Cache-Status: HIT`.
   In UI: `cache_hits` counter increments to 1; origin fetch count remains strictly 1.
3. **Concurrent Single-Flight Coalescing:**
   Dispatch 10 concurrent requests to `/api/v1/cache/data/slow`.
   Observe: 1 request fetches from origin (`MISS`), while 9 are `COALESCED`. Origin backend is protected from request stampedes.
4. **Invalidation (DELETE):**
   ```bash
   curl -i -X DELETE http://localhost:8085/api/v1/cache/catalog/product_101
   ```
   Observe: `HTTP/1.1 204 No Content`.
   Subsequent GET returns `X-Cache-Status: MISS` and fetches fresh data.

---

### Step 3: P2 Demonstration — Distributed Cluster Resilience
1. Execute the multi-node test verifying real TCP communication across 3 distinct nodes:
   ```powershell
   cargo test --test p2_dist_network_tests test_p2_multi_node_tcp_communication -j 1
   ```
2. Verify peer GET and peer-fill (Node 1 reads key owned by Node 2, populates local cache, 0 origin calls):
   ```powershell
   cargo test --test wp3_tests test_peer_read_and_peer_fill_flow -j 1
   ```
3. Verify fault detection and abrupt socket crash failover:
   ```powershell
   cargo test --test p2_dist_network_tests test_p2_actual_tcp_connection_refused_socket_failover -j 1
   ```
   Node 2 socket drops $\to$ Node 1 detects `ConnectionRefused` $\to$ Node 2 marked `Suspect` $\to$ immediate failover to Node 3.
4. Verify partition recovery:
   ```powershell
   cargo test --test p2_dist_network_tests test_p2_partition_and_recovery -j 1
   ```
   Node 2 returns $\to$ health probe passes $\to$ Node 2 resumes primary HRW ownership.

---

### Step 4: P3 Demonstration — Live Telemetry & Application Workloads
1. Generate realistic Zipfian traffic to populate percentile distributions:
   ```powershell
   for ($i = 1; $i -le 60; $i++) {
       $k = $i % 10
       curl.exe -s "http://127.0.0.1:8085/api/v1/cache/data/keys_$k" | Out-Null
   }
   ```
2. In the Control Center, inspect the screens:
   - **Overview:** Green Live status chip, real throughput, hit ratio, resident memory.
   - **Traffic & Latency:** Real-time request graphs, P50/P95/P99 latency observations (with honest `REAL` badges).
   - **Origin:** Shows `Origin concurrency controller: Enforcing (Cap: 10)` with atomic in-flight and dispatched counts.
   - **Cache & Memory:** Resident logical bytes and budget utilization under 256 MiB limit.

---

### Step 5: P4 Demonstration — Phantom Lab & Policy Evaluation
1. In the Control Center, click **Phantom Lab** in the navigation bar.
2. Observe available experiment: **Scenario B: Origin Slowdown & Degraded Upstream**.
3. Launch Scenario B with 1 repetition:
   ```bash
   curl -X POST -H "Content-Type: application/json" \
     -d '{"kind":"scenario_b","spec":{"repetitions":1,"arms":["BASE","STATIC-CAP"],"warmup_s":1,"healthy_s":1,"degraded_s":1,"recovery_s":1,"rate_rps":50.0,"n_keys":50}}' \
     http://127.0.0.1:8085/api/v1/lab/runs
   ```
4. Observe the live run progress bar in the UI.
5. On completion, click **Evidence** to inspect the retained `run.json`, `events.jsonl`, and latency percentiles.
6. Verify honest labeling: exploratory or interrupted runs display honest `TRADE_OFF_FAILURE` or `EXPLORATORY` verdicts, strictly suppressing false `VALID` badges.

---

### Step 6: Clean Shutdown
1. Stop the application via `Ctrl+C` or closing the terminal.
2. Verify ports `8085` and `3000` are completely released:
   ```powershell
   Get-NetTCPConnection -LocalPort 8085, 3000 -ErrorAction SilentlyContinue
   ```
   No listeners remain. All child processes and sockets are cleanly reclaimed.

