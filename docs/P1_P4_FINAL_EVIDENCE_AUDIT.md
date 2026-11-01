# PhantomCache — Final P1–P4 Evidence Audit & Demonstration Acceptance Report

**Audit Date:** 2026-10-30T23:55:00+05:30  
**Auditor:** Antigravity (Pair Programming Senior Software Engineer)  
**Git HEAD:** `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` (`feat/publish-phantomcache-core`)  
**Tracked Working Tree State:** Exactly 48 modified tracked files strictly preserved; 0 staged, 0 committed.  
**Research Freeze Status:** 56/56 pinned file hashes verified intact (0.05s).  
**M3 Sandbox Status:** Isolated in `D:\Temp\sbx_m3fix` (independently reviewed PASS, held pending owner approval).

---

## 1. Line-by-Line Audit of the E2E Acceptance Runner (`scripts/run_e2e_acceptance.ps1`)

The automated acceptance runner was audited line-by-line across all 26 steps. Every step was classified according to its actual execution path, network involvement, transport protocol, and test level.

### Test Classification Grammar:
- **Automated API Test:** Sends real HTTP/1.1 requests over TCP via `curl.exe` or `System.Net.Http.HttpClient` to the running gateway (`127.0.0.1:8085`) or mock origin (`127.0.0.1:9099`).
- **Socket Integration Test:** Spawns compiled Rust integration test binaries (`tests/integration/*`, `tests/unit/wp3_tests.rs`) that bind real OS TCP kernel sockets on localhost (`127.0.0.1:0`) inside the test process.
- **Unit Test:** In-process Rust method invocations testing algorithmic correctness, memory data structures, or pure logic without network sockets.
- **Scripted Lifecycle / State Check:** Performs OS process management, memory counter queries, or socket port availability verification.
- **Browser Test:** Automated component and contract test suite running under Node `--test` (`npm --prefix ui test`), validating DOM viewmodels, schemas, and rendering states.

### Step-by-Step Audit Matrix:

| Step | Script Lines | Operation Executed | Process & Endpoint | Protocol & Transport | Test Classification | Assertion Performed | Evidence Generated |
|---|---|---|---|---|---|---|---|
| **1** | 131–146 | Spawn gateway release binary with origin and lab env | `target\release\phantom-cache-demo.exe` | OS Process Creation (`Start-Process`) | Scripted Lifecycle | Process has not exited after 1s | Process PID (e.g. 28200) recorded in JSON |
| **2** | 148–151 | Query backend health endpoint | Gateway `http://127.0.0.1:8085/health` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | Response body equals `"OK"` | Exact text `"OK"` |
| **3** | 153–161 | Probe Schema 2 metrics and Lab capabilities | Gateway `http://127.0.0.1:8085/api/v1/metrics` and `/api/v1/lab/capabilities` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | `metrics.schema == 2`, `mode == "live"` | Schema 2 JSON payload |
| **4** | 163–168 | Verify live SSE telemetry stream frame | Gateway `http://127.0.0.1:8085/api/events?schema=2` | Real HTTP/1.1 SSE stream over TCP (`curl.exe --max-time 2`) | Automated API Test | Raw stream data matches `schema.:2` | First 120 bytes of SSE event frame |
| **5** | 170–175 | Validate local mock origin service | Mock Origin `http://127.0.0.1:9099/ping` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | Response body matches `"payload_for_/ping"` | Origin endpoint confirmed |
| **6** | 177–182 | Dispatch cold read to uncached path | Gateway `http://127.0.0.1:8085/api/v1/cache/data/item1` | Real HTTP/1.1 over TCP (`curl.exe -i`) | Automated API Test | Response header contains `X-Cache-Status: MISS` | Raw HTTP headers |
| **7** | 184–188 | Confirm origin fetch increment | Mock origin in-memory counter | Native C# atomic read | Scripted State Check | `[E2EMockOriginServer]::requestCount >= 1` | `requestCount` integer |
| **8** | 190–196 | Dispatch warm read to cached path | Gateway `http://127.0.0.1:8085/api/v1/cache/data/item1` | Real HTTP/1.1 over TCP (`curl.exe -i`) | Automated API Test | Response header contains `X-Cache-Status: HIT` | Raw HTTP headers |
| **9** | 198–204 | Confirm origin fetch count strictly unchanged | Mock origin in-memory counter | Native C# atomic read | Scripted State Check | `originCountAfter == originCountBefore` | Before and after integers |
| **10** | 206–214 | Fire 10 concurrent requests to slow origin endpoint | Gateway `http://127.0.0.1:8085/api/v1/cache/data/slow` | Real HTTP/1.1 over TCP (`System.Net.Http.HttpClient`) | Automated API Test (Concurrency) | 10 parallel asynchronous tasks dispatched | 10 task handles |
| **11** | 216–227 | Confirm single-flight coalescing on stampede | Response headers from 10 parallel tasks | Real HTTP/1.1 over TCP | Automated API Test (Concurrency) | `coalesced >= 1` and `misses == 1` | Breakdown: 1 MISS, 9 COALESCED |
| **12** | 229–236 | Query resident cache bytes under budget | Gateway `http://127.0.0.1:8085/api/v1/metrics` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test (Budget Accounting) | `metrics.bytes_resident.v > 0` under budget | `bytes_resident: 1041`, `budget: 268435456` |
| **13** | 238–243 | Purge cached key via DELETE | Gateway `http://127.0.0.1:8085/api/v1/cache/data/item1` | Real HTTP/1.1 over TCP (`curl.exe -i -X DELETE`) | Automated API Test | Status line contains `204 No Content` | Raw HTTP status line |
| **14** | 245–250 | Confirm post-purge refetch misses cache | Gateway `http://127.0.0.1:8085/api/v1/cache/data/item1` | Real HTTP/1.1 over TCP (`curl.exe -i`) | Automated API Test | Response header contains `X-Cache-Status: MISS` | Raw HTTP headers |
| **15** | 252–266 | Generate 60 requests across 10 keys | Gateway `http://127.0.0.1:8085/api/v1/cache/data/keys_*` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test (Traffic Gen) | `requests_total >= 70`, `cache_hits >= 50`, percentiles present | Sample count, P50 latency in JSON |
| **16** | 268–276 | Verify origin concurrency controller gate | Gateway `http://127.0.0.1:8085/api/v1/metrics` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | `metrics.origin_cap.enforcing.v == true`, `cap == 10` | Enforcing boolean, cap value |
| **17** | 278–287 | Query Phantom Lab capabilities | Gateway `http://127.0.0.1:8085/api/v1/lab/capabilities` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | `kinds.scenario_b.available == true` | Tier `MEASURED_LOOPBACK` |
| **18** | 289–310 | Start Scenario B experiment run | Gateway `http://127.0.0.1:8085/api/v1/lab/runs` | Real HTTP/1.1 over TCP (`curl.exe -X POST -H JSON`) | Automated API Test | Response body returns valid `run_id` | `run_id: 20261030T180144Z-d1ad2034` |
| **19** | 312–327 | Poll Scenario B completion and inspect artifacts | Gateway `http://127.0.0.1:8085/api/v1/lab/runs/:id` | Real HTTP/1.1 over TCP (`curl.exe`) | Automated API Test | Run reaches terminal state `completed` | Artifact URLs (`run.json`, `events.jsonl`) |
| **20** | 329–345 | Trigger cooperative cancellation on active run | Gateway `http://127.0.0.1:8085/api/v1/lab/runs/:id/cancel` | Real HTTP/1.1 over TCP (`curl.exe -X POST -H JSON`) | Automated API Test | Run state is `cancelled` or `cancelling` | State transition response |
| **21** | 347–355 | Execute 3-node TCP routing test | Subprocess `cargo test --test p2_dist_network_tests test_p2_multi_node_tcp_communication` | Real OS TCP listeners (`127.0.0.1:0`) in test process | Socket Integration Test | Output contains `test_p2_multi_node_tcp_communication ... ok` | Cargo test stdout |
| **22** | 357–364 | Execute genuine peer-fill pipeline test | Subprocess `cargo test --test wp3_tests test_peer_read_and_peer_fill_flow` | Real HTTP gateway & cluster TCP socket in test process | Socket Integration Test | Output contains `test_peer_read_and_peer_fill_flow ... ok` | Cargo test stdout |
| **23** | 366–375 | Execute TCP socket crash & recovery test | Subprocess `cargo test --test p2_dist_network_tests test_p2_actual_tcp_connection_refused_socket_failover` and `test_p2_partition_and_recovery` | Real OS TCP sockets dropped, `ConnectionRefused` caught | Socket Integration Test | Both test cases return `... ok` | Cargo test stdout |
| **24** | 377–383 | Cleanly terminate running gateway process | `Stop-Process -Id $gatewayProc.Id -Force` | OS Process Termination (`Stop-Process`) | Scripted Lifecycle | Process handle terminates | Terminated PID confirmed |
| **25** | 385–390 | Inspect TCP connection table for released ports | `Get-NetTCPConnection -LocalPort 8085` | OS TCP stack query | Scripted Lifecycle | No active connection or listener on port 8085 | Listener count == 0 |
| **26** | 392–404 | Restart gateway binary and verify readiness | `target\release\phantom-cache-demo.exe` -> `GET /health` | OS Process Spawn and HTTP probe | Scripted Lifecycle | Restarted process answers `OK` on `/health` | Restart PID and health response |

### Crucial Audit Findings on Test Type Distinction:
1. **Zero Browser Automation in the Runner:** `scripts/run_e2e_acceptance.ps1` does **NOT** automate a web browser. It does not launch Chrome or interact with the React DOM. It is an **Automated Backend & API Integration Suite**.
2. **Browser/UI Testing is Covered Separately:** The React Control Center is verified through `npm --prefix ui test` (**483 passing tests** under Node `--test`), which validates contract schemas, viewmodels, provenance badges, chart fallbacks, and unavailable states against real schema 2 payloads.
3. **No Simulation Shortcuts:** All HTTP calls in Steps 2–20 are genuine HTTP/1.1 requests over OS network loopback TCP sockets (`127.0.0.1`).

---

## 2. P1 Security Evidence & Execution Verification (Audit of P1-09, P1-12, P1-16)

Every P1 security requirement was audited against its executed test, input, observed output, and evidence artifact:

```
+-----------------------------------------------------------------------------------------------------------------------------+
|                                              P1 SECURITY & RESOURCE AUDIT TABLE                                             |
+--------+---------------------------------+------------------------------------------+---------------------------------------+
| ID     | Requirement Description         | Executed Test Target                     | Verified Evidence & Classification    |
+--------+---------------------------------+------------------------------------------+---------------------------------------+
| P1-01  | Cold Cache MISS                 | run_e2e_acceptance.ps1:Step 6            | Automated API Test (HTTP 200, MISS)   |
| P1-02  | Origin Fetch & Population       | run_e2e_acceptance.ps1:Step 7            | Scripted State Check (Counter >= 1)   |
| P1-03  | Warm Cache HIT                  | run_e2e_acceptance.ps1:Step 8            | Automated API Test (HTTP 200, HIT)    |
| P1-04  | Zero Origin Dispatch on HIT     | run_e2e_acceptance.ps1:Step 9            | Scripted State Check (Counter ==)     |
| P1-05  | Concurrent Stampede Coalescing  | run_e2e_acceptance.ps1:Step 10-11        | Automated API Test (1 MISS, 9 COALESC)|
| P1-06  | DELETE Purge Invalidation       | run_e2e_acceptance.ps1:Step 13           | Automated API Test (HTTP 204)         |
| P1-07  | Post-Purge Cache MISS           | run_e2e_acceptance.ps1:Step 14           | Automated API Test (HTTP 200, MISS)   |
| P1-08  | RFC 9111 Revalidation / Stale  | freshness_matrix_tests.rs                | Harness Integration Test (304 / ETag) |
| P1-09  | Oversized Request Rejection     | src/api/lab_api.rs:1693                  | Code Audit Guardrail (cl > 16 KiB 413)|
| P1-10  | Oversized Upstream Response Cap | byte_accounting_tests.rs                 | Unit / Harness Test (2MB ResponseLimit)|
| P1-11  | Malformed Framing (RFC 9112)    | security_phase2_upstream_tests.rs        | Socket Integration Test (400 Bad Req) |
| P1-12  | Invalid Cache Key / Traversal   | m7_sec_round3_tests.rs & canonical test  | Automated API Test & Proptest (400)   |
| P1-13  | SSRF Cloud Metadata & Port Block| security_phase2_upstream_tests.rs        | Socket Integration Test (BlockedIp)   |
| P1-14  | Invalid / Tampered JWT Rejected | security_phase1_tests.rs                 | Socket Integration Test (401 Unauth)  |
| P1-15  | Tenant Cryptographic Isolation  | security_phase1_tests.rs                 | Socket Integration Test (Namespace ISO|
| P1-16  | Cache Capacity & Eviction       | eviction_contract_tests.rs & Step 12     | Contract Test & Live Budget Check     |
| P1-17  | Request Cancellation Cleanup    | run_e2e_acceptance.ps1:Step 20           | Automated API Test (HTTP 202, Cancel) |
+--------+---------------------------------+------------------------------------------+---------------------------------------+
```

### Detailed Deep-Dive on P1-09, P1-12, and P1-16:

#### A. P1-09: Oversized Request Rejection
- **Exact Implementation Target:** `src/api/lab_api.rs:1693–1701` (`pub const MAX_BODY_BYTES: usize = 16 * 1024`).
- **Mechanism:** Enforces hard cap on incoming POST bodies to `/api/v1/lab/runs`. Checks `Content-Length > MAX_BODY_BYTES` early and applies `tokio::time::timeout(BODY_READ_TIMEOUT, to_bytes(body, MAX_BODY_BYTES))`.
- **Observed Output on Violation:** HTTP `413 Payload Too Large` with JSON `{"error": "payload_too_large", "message": "request body exceeds 16 KiB"}`.
- **Upstream Origin Response Cap:** Enforces `MAX_BODY_BYTES = 2 * 1024 * 1024` (2 MB) in `src/api/server.rs:2369, 2875`, verified by `tests/integration/byte_accounting_tests.rs:test_oversize_response_bypass` (response streamed un-cached, zero memory leak).
- **Accurate Classification:** **Implementation-Level Ingress Guardrail (Code Audit Verified)** in `lab_api.rs`, combined with **Unit / Harness Test** in `byte_accounting_tests.rs`. It was **NOT** executed as a live curl command in `scripts/run_e2e_acceptance.ps1`.

#### B. P1-12: Invalid Cache Key & URI Traversal
- **Exact Executed Test Target:** 
  1. `tests/integration/m7_sec_round3_tests.rs:b_check_order_is_401_then_403_role_then_403_tenant_then_400`.
  2. `tests/integration/canonical_key_roundtrip_tests.rs` (Property-based testing with `proptest`).
- **Exact Input:** HTTP GET `/api/v1/cache/a%20b` (unescaped space/control character), paths containing `/../` or `/./`, and tenant header containing invalid characters `a b`.
- **Observed Output:** HTTP `400 Bad Request` with response headers `X-Cache-Error: InvalidPath` or `X-Cache-Error: InvalidTenantId`. Origin request counter remains strictly 0.
- **Implementation Mechanism:** `src/core/cache.rs:CacheKey::validate()` strictly rejects C0 control characters, spaces, and path segments matching `.` or `..` before origin dispatch.
- **Accurate Classification:** **Automated API Integration Test (Harness-Managed HTTP Gateway)** and **Unit / Proptest Validation**. It was NOT executed in the live 26-step acceptance script.

#### C. P1-16: Cache Capacity & Eviction Enforcement
- **Exact Executed Test Target:**
  1. `tests/integration/eviction_contract_tests.rs:test_byte_budget_enforcement` and `memory_bound_soak_tests.rs`.
  2. `scripts/run_e2e_acceptance.ps1:Step 12` (Live budget accounting verification).
- **Exact Input:**
  - *Contract Test:* Inserts resources exceeding configured shard byte budget (e.g. 1 MiB budget).
  - *Live Script:* Reads 2 items (~1,041 bytes) under the configured 256 MiB (`268,435,456 B`) ceiling.
- **Observed Output:**
  - *Contract Test:* Sweeper identifies LRU/S3FIFO victims, evicts resident items, and keeps resident bytes below budget.
  - *Live Script:* Returns `metrics.bytes_resident.v: 1041` and `metrics.cache_bytes_budget.v: 268435456` ($1041 < 268435456$).
- **Accurate Classification:** **Unit / Contract Test (Capacity Saturation & Eviction)** and **Automated API Test (Live Budget Accounting)**. Active live capacity saturation and eviction was NOT demonstrated in the live E2E script.

---

## 3. Independent P2 Distributed Systems & Process Audit

### Strict Granularity of P2 Validation Tiers:

The distributed capabilities of PhantomCache exist across three distinct engineering tiers:

```
+----------------------------------------------------------------------------------------------------+
|                                    P2 DISTRIBUTED SYSTEMS TIERS                                    |
+--------+---------------------------------------+-----------+---------------------------------------+
| Tier   | Validation Scope & Architecture       | Status    | Evidence & Scope Description          |
+--------+---------------------------------------+-----------+---------------------------------------+
| Tier 1 | Real Socket Integration Tests         | **PASS**  | Nodes bind real OS TCP kernel sockets |
|        | (Harness-Managed Multi-Listener)      |           | on 127.0.0.1:0 inside test process.   |
|        |                                       |           | TCP stream wire framing, connection   |
|        |                                       |           | refused failover (20ms), timeouts.    |
+--------+---------------------------------------+-----------+---------------------------------------+
| Tier 2 | Independent OS Daemon Processes       | **PARTIAL**| main.rs supports cluster flags & mTLS |
|        | (Multiple PIDs on Host)               |           | but NOT automated or demonstrated in  |
|        |                                       |           | run_e2e_acceptance.ps1 runner.        |
+--------+---------------------------------------+-----------+---------------------------------------+
| Tier 3 | Physical Multi-Host LAN Validation    | **UNTESTED| SETBACK-18 / RISK-P2-01: Multi-host   |
|        | (Cross-Machine Network Switch / VLAN) | / OPEN**  | validation not executed in this local |
|        |                                       |           | single-machine workspace.             |
+--------+---------------------------------------+-----------+---------------------------------------+
```

### Individual Inspection of P2-04, P2-05, and P2-06:

The real-socket classification must **never** be transferred from one test to another. Each test was inspected in isolation:

#### A. P2-04: Peer GET Semantics (SETBACK-11 / DR-029)
- **What was inspected:** `src/dist/node.rs:process_binary_cache_stream` and `tests/unit/wp3_tests.rs`.
- **Unit Logic:** When a node receives a peer `GetRequest`, it performs a read-only non-mutating cache lookup (`node.cache.get_resource`). On a cache miss, it returns genuine `"MISS"` and does **NOT** insert synthetic values (e.g. `val_node_X`) into the cache, nor charge memory bytes.
- **Test Executed:**
  1. `test_shared_cache_node_peer_get_genuine_miss_no_synthetic_data` (wp3_tests.rs:228): Binds a loopback TCP socket, connects via `TcpStream`, sends `PhantomMeshFrame::new_get_request`, and asserts response is `"MISS"` and cache remains 0 bytes resident.
  2. `test_shared_cache_node_peer_get_genuine_hit` (wp3_tests.rs:262): Binds a loopback TCP socket, pre-populates entry, connects via `TcpStream`, sends frame, asserts response is `"HIT body"`.
  3. In-process pure cache logic: `VirtualNode::new_with_cache` inspected in isolation without sockets.
- **Accurate Classification:**
  - Cache Read-Only Non-Mutation Logic: **Unit Test (In-Process Call / In-Memory Lookup)**.
  - Wire Framing Exchange: **Socket Integration Test (Real Loopback TCP Socket)**.
  - It is NOT an independent daemon process test.

#### B. P2-05: Peer-Fill Pipeline (SETBACK-14)
- **What was inspected:** `tests/unit/wp3_tests.rs:test_peer_read_and_peer_fill_flow` (lines 324–446).
- **Execution Topology:**
  - Node 1 binds a loopback TCP socket (`VirtualNode::start_tcp_service("127.0.0.1:0")`).
  - Node 2 binds a loopback TCP socket for its mesh port AND spawns an Axum HTTP gateway on loopback TCP.
  - Client connects to Node 2's HTTP port via `TcpStream`; Node 2 misses locally, queries Node 1 over the kernel loopback TCP socket, receives `HIT authentic_peer_payload_42`, populates Node 2's local cache (peer-fill), and responds HTTP 200 OK to the client.
- **Accurate Classification:** **Socket Integration Test with Real Loopback TCP Sockets**. Both HTTP ingress and inter-node mesh communication use real operating system network sockets, executing within the test harness process.

#### C. P2-06: Zero Origin Dispatch on Peer Hit
- **What was inspected:** `tests/unit/wp3_tests.rs:test_peer_read_and_peer_fill_flow` and `test_peer_miss_falls_back_to_origin`.
- **Execution Details:**
  - In `test_peer_read_and_peer_fill_flow`, Node 2's `origin_url` is configured to `None` (origin completely absent). The request succeeds strictly via peer read over TCP, proving that 0 origin dispatches occurred.
  - In `test_peer_miss_falls_back_to_origin` (lines 449–565), an actual mock origin TCP server (`127.0.0.1:0`) tracks dispatch counts with an `AtomicUsize`. When Node 1 misses, origin fetch count increments to 1; on subsequent cached requests, origin fetch count remains strictly 1 (0 dispatches).
- **Accurate Classification:** **Socket Integration Test with Real Loopback TCP Sockets**.

---

## 4. P3 and P4: Boundaries & Milestone 6 Hypothesis 1 (M6 H1)

To preserve scientific rigor, five concepts that are frequently conflated are explicitly separated:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              P3 / P4 CONCEPTUAL BOUNDARIES                             │
└──────────────────────────────────────────┬─────────────────────────────────────────────┘
                                           │
         ┌─────────────────────────────────┼─────────────────────────────────┐
         ▼                                 ▼                                 ▼
┌─────────────────────────┐   ┌─────────────────────────┐   ┌─────────────────────────┐
│     DETERMINISTIC       │   │       ACTUAL HTTP       │   │    POLICY MECHANISM     │
│   TRAFFIC GENERATION    │   │    WORKLOAD EXECUTION   │   │       VALIDATION        │
│                         │   │                         │   │                         │
│ Seeded PRNG (Seed 42)   │   │ 60 real curl requests   │   │ LRU, S3FIFO, SIEVE      │
│ Deterministic traces    │   │ Live loopback TCP       │   │ AIMD Concurrency Gate   │
│ Reproducible rank skew  │   │ Live SSE metric updates │   │ Guardrail Rollback      │
└─────────────────────────┘   └─────────────────────────┘   └─────────────────────────┘
                                           │
                                           ├─────────────────────────────────┐
                                           ▼                                 ▼
                              ┌─────────────────────────┐       ┌─────────────────────────┐
                              │       COMPARATIVE       │       │  STATISTICALLY PROVEN   │
                              │  PERFORMANCE BENCHMARK  │       │    ADAPTIVE BENEFIT     │
                              │                         │       │                         │
                              │ Relative hit-rates      │       │     NOT SUPPORTED       │
                              │ Zipfian trace replay    │       │   (Milestone 6 H1)      │
                              │ Scenario B lab runner   │       │ Static baseline beats   │
                              │                         │       │ adaptive in sealed run. │
                              └─────────────────────────┘       └─────────────────────────┘
```

1. **Deterministic Workload Generation:** Implemented in `src/sim/traffic.rs` using seeded PRNG (seed 42). Verified in `tests/integration/p3_traffic_tests.rs:test_p3_seeded_workload_determinism`. Produces bit-for-bit identical request sequences and rank skews.
2. **Actual HTTP Workload Execution:** Real HTTP/1.1 traffic sent to the running gateway via `curl.exe` (Step 15 of `run_e2e_acceptance.ps1`: 60 requests across 10 keys). Real sockets, genuine OS networking, and atomic counter updates.
3. **Policy Mechanism Validation:** Algorithmic correctness of LRU, S3FIFO, and SIEVE eviction; AIMD origin concurrency cap enforcement (`metrics.origin_cap.enforcing.v == true`, cap 10); automatic guardrail rollback on regression (>5%).
4. **Comparative Performance Evaluation:** Benchmarking eviction algorithms against the same Zipfian trace in `tests/integration/p4_pie_tests.rs` and live Scenario B runs.
5. **Statistically Supported Adaptive Benefit:** **NOT SUPPORTED**. Milestone 6 Hypothesis 1 (M6 H1) demonstrated that adaptive policy control did not beat a tuned static baseline. P4 acceptance confirms mechanism correctness and origin protection, NOT adaptive superiority. Speculative predictive prewarming remains quarantined per ADR-002.

---

## 5. Re-Evaluation of the Five MC-MVP Acceptance Gates

All five gates are re-evaluated strictly against their original acceptance criteria:

```
+----------------------------------------------------------------------------------------------------+
|                                    MC-MVP GATE STATUS RE-EVALUATION                                |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate   | Scope & Acceptance Pillar             | Status    | Evidence & Remaining Unmet Conditions |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate 1 | Operational Reliability & Launcher    | **PASS**  | 3-cycle start/stop/restart verified   |
|        |                                       |           | (test_demo_lifecycle.ps1 exit 0).     |
|        |                                       |           | Port cleanup verified (0 locks).      |
|        |                                       |           | Unmet Conditions: None.               |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate 2 | Core Correctness & Crash Durability   | **PARTIAL**| In-memory RFC 9111, single-flight     |
|        |                                       |           | coalescing, DEC-001 PASS in main tree.|
|        |                                       |           | M3 durability reviewed PASS in sandbox|
|        |                                       |           | D:\Temp\sbx_m3fix, but UNMERGED in    |
|        |                                       |           | main tree pending owner approval.     |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate 3 | Ingress Hardening & Distributed Net   | **PARTIAL**| SSRF, RFC 9112 framing, JWT RBAC PASS.|
|        |                                       |           | In-process real TCP socket routing    |
|        |                                       |           | and peer-fill PASS. Independent OS    |
|        |                                       |           | daemons & physical multi-host UNMET.  |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate 4 | Workload Intelligence & Origin Guard  | **PARTIAL**| AIMD origin concurrency cap enforcing |
|        |                                       |           | in serving path. Benchmarks pass.     |
|        |                                       |           | Unmet: Prewarming quarantined ADR-002;|
|        |                                       |           | M6 H1 NOT_SUPPORTED preserved.        |
+--------+---------------------------------------+-----------+---------------------------------------+
| Gate 5 | Integrated Control Center & Honest UI | **PASS**  | 483 UI tests pass. Live SSE streams.  |
|        |                                       |           | Provenance badges (REAL/UNAVAILABLE)  |
|        |                                       |           | strictly truthful. Zero fake metrics. |
+--------+---------------------------------------+-----------+---------------------------------------+
```

---

## 6. Repository Preservation Reconfirmation

- **Modified Tracked Files:** Exactly **48 files** preserved (`git status -s | Select-String '^ M'` = 48).
- **Staged Files:** **0 files** (`git status -s | Select-String '^[MADRC]'` = 0).
- **Git Commits:** **0 new commits** created in this session.
- **Research Freeze Integrity:** Verified intact (**56/56 hashes match** in `m6_research_evaluation_tests`).
- **M3 Remediation Sandbox:** Preserved isolated in `D:\Temp\sbx_m3fix` awaiting explicit owner authorization.

---

## 7. Final Acceptance Conclusion

With all evidence classifications strictly corrected and verified:
1. **P1 (Resource Safety):** PASS. Hardened against SSRF, framing exploits, oversized payloads, invalid JWTs, and stampedes.
2. **P2 (Distributed Systems):** PASS at the **Harness-Managed Real Socket Level**. Multi-process daemons and physical multi-host LAN remain open.
3. **P3 (Real Workloads & Telemetry):** PASS. Genuine HTTP traffic, atomic counter updates, truthful provenance badges, and live SSE streaming.
4. **P4 (Policy Evaluation & Origin Protection):** PASS (Mechanism Verified; Superiority Not Claimed). AIMD origin concurrency controller active in serving path; comparative benchmarks operational; M6 H1 NOT_SUPPORTED preserved.
5. **Demonstration Suite:** Complete, executable, and reproducible via [`scripts/run_e2e_acceptance.ps1`](file:///d:/Personal/phantom-cache/scripts/run_e2e_acceptance.ps1) (26/26 steps passing in 23.7s).
