# PhantomCache Consolidated Demonstration & Acceptance Guide

**Document Version:** 2.1.0 (Audited Demonstration & Evidence-Classification Edition)  
**Target Audience:** Evaluators, Demonstration Operators, and Distributed Systems Engineers  
**System Under Test:** PhantomCache Release Binary (`target\release\phantom-cache-demo.exe`) and Control Center (`ui/`)  
**Workspace:** `D:\Personal\phantom-cache`  
**Git HEAD:** `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a`  
**Working Tree State:** Exactly 48 modified tracked files preserved; zero staged, zero committed.  
**Research Freeze Verification:** 56/56 pinned file hashes verified intact (0.05s).  

---

# SECTION A: ENVIRONMENT AND STARTUP

## 1. Prerequisites and Dependencies
- **Operating System:** Windows 10/11 or Windows Server (x86_64) with PowerShell 7 (`pwsh`) or Windows PowerShell 5.1.
- **Node.js & npm:** Node.js v18.0.0+ (Node v20 LTS recommended) and npm v9.0.0+.
- **Rust Toolchain:** `rustc` and `cargo` 1.80+ (only required if recompiling source code).
- **HTTP Client:** `curl.exe` (included in Windows 10/11) and PowerShell `System.Net.Http.HttpClient`.

## 2. Port Allocation Matrix
Ensure the following TCP ports are free before launching:

| Port | Service | Bound Address | Purpose |
|---|---|---|---|
| **8085** | Axum Gateway API | `127.0.0.1:8085` | Primary HTTP cache ingress, telemetry, Lab API, metrics |
| **3000** | Control Center UI | `::1:3000` / `127.0.0.1:3000` | React/Vite development server |
| **9099** | Mock Origin Server | `127.0.0.1:9099` | Local high-performance upstream backend |
| **18088** | Node 1 Cluster Port | `127.0.0.1:18088` | Inter-node mTLS cluster communication (Node 1) |
| **18089** | Node 2 Cluster Port | `127.0.0.1:18089` | Inter-node mTLS cluster communication (Node 2) |
| **18090** | Node 3 Cluster Port | `127.0.0.1:18090` | Inter-node mTLS cluster communication (Node 3) |

## 3. Configuration & Environment Variables
The gateway is configured via process environment variables:

```powershell
# Mandatory for Live Demo with Full Features:
$env:ORIGIN_URL                   = "http://127.0.0.1:9099"      # Upstream origin endpoint
$env:PHANTOM_BIND_ADDR            = "127.0.0.1:8085"             # Gateway listener
$env:PHANTOM_LAB_ENABLED          = "true"                      # Enables /api/v1/lab/* routes
$env:PHANTOM_LAB_GATEWAY_EXE      = (Resolve-Path "target\release\phantom-cache-demo.exe").Path
$env:PHANTOM_ORIGIN_CAP_MAX       = "10"                        # Maximum concurrent fetches to origin
$env:PHANTOM_ORIGIN_CAP_MIN       = "2"                         # Minimum origin concurrency floor
$env:PHANTOM_INTELLIGENCE         = "canary"                    # Controller mode: off, canary, active
$env:PHANTOM_MAX_CACHE_BYTES      = "268435456"                 # 256 MiB resident cache budget
$env:PHANTOM_CORS_ORIGINS         = "http://localhost:3000,http://127.0.0.1:3000"
```

## 4. Local Mock Origin Setup
Start the native C# HTTP mock origin server on port `9099` in background:
```powershell
pwsh -Command @"
Add-Type -TypeDefinition (Get-Content scripts/verify_real_cache_ops.ps1 -Raw | Select-String -Pattern 'public class MockOriginServer[\s\S]*?^}' -AllMatches).Matches[0].Value -Language CSharp
[MockOriginServer]::Start()
Write-Host 'Mock Origin running on http://127.0.0.1:9099'
"@
```
*(Alternatively, execute `pwsh -File scripts/verify_real_cache_ops.ps1`, which starts the origin and runs a verification cycle).*

## 5. Startup Commands & Expected Output

### Standard 1-Click Launch:
```cmd
Start-Phantom-Demo.bat
```

*Expected Terminal Output:*
```text
==============================================
              PHANTOM CACHE
           INITIALIZING SYSTEM
==============================================
Checking environment prerequisites...
Verifying port availability...
Checking binary freshness...
[OK] Release binary target\release\phantom-cache-demo.exe is up-to-date.
Starting Axum API server...
Waiting for API readiness on http://localhost:8085/health...
[OK] API Server Ready (PID: 28200).
Starting React frontend...
Waiting for UI readiness on http://localhost:3000...
[OK] UI Console Ready (PID: 31404).
Launching browser...
==============================================
              SYSTEM READY
==============================================
API Server:  http://localhost:8085  (PID: 28200)
UI Console:  http://localhost:3000  (PID: 31404)
==============================================
Press any key to stop all background processes and exit.
```

## 6. Health Checks & URLs
- **Backend API:** `http://localhost:8085`
- **Frontend Console:** `http://localhost:3000`
- **Health Verification:**
  ```bash
  curl http://localhost:8085/health
  ```
  *Response:* `OK` (HTTP 200)

---

# SECTION B: GATEWAY CONNECTION & CONTROL CENTER

## 1. Gateway Connection Workflow
1. Open `http://localhost:3000` in the browser.
2. In the header bar, click the **Connection Status Chip**.
3. **Gateway Address Form:**
   - Candidate address defaults to `http://localhost:8085`.
   - Click **Test Connection**: Issues parallel GET probes to `/api/v1/metrics` and `/api/v1/lab/capabilities`.
   - Results display: `Metrics: reachable`, `Capabilities: reachable`.
   - Click **Use for this visit only** (stored in page memory) or **Remember on this computer** (`localStorage`).
   - Click **Reset** to return to system default.

## 2. Telemetry Connection vs Subsystem Capabilities
- **Green "Live" Indicator:** Indicates that the Server-Sent Events (SSE) stream (`/api/events?schema=2`) is open and receiving updates every 1000ms.
- **Subsystem Reporting:** The Live indicator does NOT mean all subsystems are active. Overview definition lists report each capability independently:
  - `Cache API`: Available
  - `Origin Metrics`: Available (when `ORIGIN_URL` is set)
  - `Phantom Lab`: Available (when `PHANTOM_LAB_ENABLED=true`)
  - `Cluster Mode`: Disabled (in standalone mode) / Enabled (in multi-node mTLS mode)

---

# SECTION C: P1 — REAL HTTP CACHE & RESOURCE SAFETY SCENARIOS

| Field | Required Information |
|---|---|
| **Scenario 1** | **Cold GET and Cache MISS** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP)** |
| Prerequisites | Gateway on 8085, Mock Origin on 9099 |
| Command | `curl.exe -i http://127.0.0.1:8085/api/v1/cache/data/item1` |
| Expected Result | HTTP 200 OK, `x-cache-status: MISS`, JSON payload fetched from origin |
| UI Observation | Traffic & Latency screen: `cache_misses` counter increments to 1 |
| Evidence | HTTP response headers in `lab-results/e2e_acceptance_evidence.json` (Step 6) |
| Acceptance | Response contains `x-cache-status: MISS` and status code 200 |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 2** | **Origin Fetch and Cache Population** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP)** |
| Prerequisites | Execution of Scenario 1 |
| Command | `curl.exe -s http://127.0.0.1:8085/api/v1/metrics` |
| Expected Result | `metrics.cache_misses.v == 1`, `metrics.bytes_resident.v > 0` |
| UI Observation | Cache & Memory screen: Resident logical bytes increases from 0 B to ~523 B |
| Evidence | Schema 2 metrics payload (`lab-results/e2e_acceptance_evidence.json`, Step 7) |
| Acceptance | Origin records 1 fetch, cache resident bytes strictly greater than 0 |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 3** | **Warm GET and Cache HIT** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP)** |
| Prerequisites | Cached entry for `/data/item1` |
| Command | `curl.exe -i http://127.0.0.1:8085/api/v1/cache/data/item1` |
| Expected Result | HTTP 200 OK, `x-cache-status: HIT`, served from memory |
| UI Observation | Traffic & Latency screen: `cache_hits` increments to 1, origin fetches remain unchanged |
| Evidence | HTTP response headers in `lab-results/e2e_acceptance_evidence.json` (Step 8) |
| Acceptance | Response header `x-cache-status: HIT`; mock origin fetch count strictly unchanged |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 4** | **DELETE / Purge and Subsequent Refetch** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP)** |
| Prerequisites | Cached entry for `/data/item1` |
| Command | `curl.exe -i -X DELETE http://127.0.0.1:8085/api/v1/cache/data/item1 && curl.exe -i http://127.0.0.1:8085/api/v1/cache/data/item1` |
| Expected Result | DELETE returns HTTP 204 No Content; subsequent GET returns HTTP 200 with `x-cache-status: MISS` |
| UI Observation | Cache & Memory: resident entry removed, then repopulated |
| Evidence | HTTP 204 status line in `lab-results/e2e_acceptance_evidence.json` (Steps 13–14) |
| Acceptance | Entry invalidated immediately; subsequent GET refetches from origin |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 5 & 6** | **Ten Concurrent Requests & Single-Flight Coalescing** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP Concurrency)** |
| Prerequisites | Uncached path `/data/slow` (250ms backend origin delay) |
| Command | `pwsh -Command "$h=New-Object System.Net.Http.HttpClient; $t=1..10|%{$h.GetAsync('http://127.0.0.1:8085/api/v1/cache/data/slow')}; [Threading.Tasks.Task]::WaitAll($t); $t|%{$_.Result.Headers.GetValues('X-Cache-Status')}"` |
| Expected Result | Exactly 1 response has `MISS`; remaining 9 responses have `COALESCED` |
| UI Observation | Traffic & Latency screen: `coalesced_waiters` increments by 9 |
| Evidence | Breakdown in `lab-results/e2e_acceptance_evidence.json` (Step 11): 1 MISS, 9 COALESCED |
| Acceptance | Origin backend receives exactly 1 request; 9 waiters coalesced |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 7 & 8** | **TTL, Freshness, Stale Serving & Revalidation** |
| Phase | P1 |
| Test Classification | **Socket / Harness Integration Test** |
| Prerequisites | Origin returns `Cache-Control: public, max-age=1` |
| Command | `cargo test --test freshness_matrix_tests test_rfc9111_stale_serving_and_revalidation -j 1` |
| Expected Result | Fresh entry served directly; expired entry revalidates with `If-None-Match`; 304 Not Modified updates freshness |
| UI Observation | Freshness screen: Revalidations started/ok counters increment |
| Evidence | Integration test output `tests/integration/freshness_matrix_tests.rs` |
| Acceptance | RFC 9111 Section 4 revalidation compliance verified |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 9 (P1-09)**| **Oversized Request Rejection** |
| Phase | P1 |
| Test Classification | **Implementation-Level Code Guardrail (Code Audit Verified)** & **Unit / Harness Test (Upstream Body Cap)** |
| Prerequisites | Gateway or Lab API running |
| Implementation Target | `src/api/lab_api.rs:1693–1701` (`MAX_BODY_BYTES = 16 KiB`) and `src/api/server.rs:2369, 2875` (`MAX_BODY_BYTES = 2 MiB`) |
| Command | `cargo test --test byte_accounting_tests test_oversize_response_bypass -j 1` |
| Expected Result | Ingress bodies $> 16\text{ KiB}$ rejected with HTTP 413 Payload Too Large (`"request body exceeds 16 KiB"`); upstream bodies $> 2\text{ MiB}$ bypass cache without memory leak |
| UI Observation | Error banner rendered without crashing frontend |
| Evidence | Code audit of `src/api/lab_api.rs:1693` & test stdout in `tests/integration/byte_accounting_tests.rs` |
| Acceptance | Hard cap prevents memory saturation; oversized bodies cannot enter serving cache |
| Validation Status | **Code Audit & Harness Test PASS (Not executed as live curl in E2E runner)** |

| Field | Required Information |
|---|---|
| **Scenario 10** | **Oversized Response Rejection** |
| Phase | P1 |
| Test Classification | **Unit / Harness Test** |
| Prerequisites | Gateway configured with `PHANTOM_MAX_CACHE_BYTES=268435456` |
| Command | `cargo test --test byte_accounting_tests test_oversize_response_bypass -j 1` |
| Expected Result | Oversize body streamed to client without caching; `oversize_rejections` counter incremented |
| UI Observation | Cache & Memory: Oversize rejections counter increments |
| Evidence | `src/core/cache.rs` atomic counter |
| Acceptance | Response > budget is not admitted; zero memory leak |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 11** | **Malformed HTTP Request & Framing Rejection** |
| Phase | P1 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | Gateway running |
| Command | `cargo test --test security_phase2_upstream_tests test_p2_conflicting_content_length_rejected -j 1` |
| Expected Result | Conflicting `Content-Length` headers rejected with HTTP 400 Bad Request |
| UI Observation | No corrupted request admitted to pipeline |
| Evidence | `tests/integration/security_phase2_upstream_tests.rs` |
| Acceptance | RFC 9112 Section 6.1 framing violation rejected fail-closed |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 12 (P1-12)**| **Invalid Cache Key and URI Handling** |
| Phase | P1 |
| Test Classification | **Automated API Integration Test (Harness-Managed HTTP Gateway)** & **Unit / Proptest Validation** |
| Prerequisites | Gateway running |
| Command | `cargo test --test m7_sec_round3_tests b_check_order_is_401_then_403_role_then_403_tenant_then_400 -j 1` |
| Expected Result | Unescaped space (`/a%20b`), directory traversal (`/../`), or illegal tenant alphabet (`a b`) rejected with HTTP 400 Bad Request (`X-Cache-Error: InvalidPath` or `InvalidTenantId`) |
| UI Observation | No directory traversal admitted to origin |
| Evidence | `tests/integration/m7_sec_round3_tests.rs:500-524` & `tests/integration/canonical_key_roundtrip_tests.rs` |
| Acceptance | Path validation and canonical key parser reject directory traversal fail-closed; origin requests strictly 0 |
| Validation Status | **Executed PASS (Harness-Managed API & Proptest; not executed in E2E runner)** |

| Field | Required Information |
|---|---|
| **Scenario 13** | **SSRF and Blocked Origin Targets** |
| Phase | P1 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | Gateway with upstream security policy active |
| Command | `cargo test --test security_phase2_upstream_tests test_p2_ssrf_cloud_metadata_blocked -j 1` |
| Expected Result | Connect attempt to `169.254.169.254` blocked with `OriginSecurityError::BlockedIp` |
| UI Observation | Origin screen: Connection error logged |
| Evidence | Log output: `[SSRF FILTER] Rejected outbound connect to link-local cloud metadata` |
| Acceptance | Pre-connect IP filter blocks cloud metadata, IPv6 mapped addresses, and ports 22/25/445 |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 14** | **Unauthorized and Invalid JWT Requests** |
| Phase | P1 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | Gateway with `PHANTOM_JWT_SECRET` configured |
| Command | `cargo test --test security_phase1_tests test_phase1_at03_and_at07_unauthorized_or_invalid_jwt_rejected -j 1` |
| Expected Result | Tampered token or missing Authorization header rejected with HTTP 401 Unauthorized |
| UI Observation | UI reports 401 Unauthorized alert |
| Evidence | `tests/integration/security_phase1_tests.rs` (15/15 passed) |
| Acceptance | Cryptographic JWT verification strictly enforced; control endpoints protected |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 15** | **Tenant and Namespace Isolation** |
| Phase | P1 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | Gateway running with multi-tenant headers |
| Command | `cargo test --test security_phase1_tests test_phase1_at05_and_at06_tenant_cryptographic_isolation -j 1` |
| Expected Result | Tenant B requesting Tenant A's cached key receives `MISS` |
| UI Observation | Tenant partition metrics tracked independently |
| Evidence | `src/security/jwt.rs` SHA-256 tenant prefixing |
| Acceptance | Zero cross-tenant data leakage or partition collision |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 16 (P1-16)**| **Cache Capacity & Eviction Enforcement** |
| Phase | P1 |
| Test Classification | **Unit / Contract Test (Capacity Saturation)** & **Automated API Test (Live Budget Accounting)** |
| Prerequisites | Gateway running |
| Command | `cargo test --test eviction_contract_tests test_byte_budget_enforcement -j 1` |
| Expected Result | Contract test saturates budget, triggers sweeper eviction, and bounds memory; live E2E script Step 12 queries live budget accounting ($1041\text{ B} < 256\text{ MiB}$) |
| UI Observation | Cache & Memory: Evictions counter increments; memory capped |
| Evidence | `tests/integration/eviction_contract_tests.rs` & `lab-results/e2e_acceptance_evidence.json` (Step 12) |
| Acceptance | Memory strictly respects configured budget ceiling |
| Validation Status | **Executed PASS (Contract Test Saturation; Live Script Budget Accounting)** |

| Field | Required Information |
|---|---|
| **Scenario 17** | **Request Cancellation & Resource Cleanup** |
| Phase | P1 |
| Test Classification | **Automated API Test (Real HTTP over TCP)** |
| Prerequisites | Active long-running request |
| Command | `curl.exe -s -X POST -H "Content-Type: application/json" -d "{}" http://127.0.0.1:8085/api/v1/lab/runs/<run_id>/cancel` |
| Expected Result | HTTP 202 Accepted; run transitions to `cancelled`; child process terminated |
| UI Observation | Lab progress bar stops; status displays Cancelled; false VALID suppressed |
| Evidence | `lab-results/e2e_acceptance_evidence.json` (Step 20) |
| Acceptance | Process Job Object terminates child; no leaked OS handles |
| Validation Status | **Executed PASS** |

---

# SECTION D: P2 — THREE-NODE DISTRIBUTED CLUSTER DEMONSTRATION & SECURITY

> [!IMPORTANT]
> **Strict Three-Tier Distinction for Distributed Systems Validation:**  
> 1. **Tier 1 (Demonstrated): Real Socket Integration Tests:** Multiple `VirtualNode` instances bind actual operating system TCP kernel listeners on localhost (`127.0.0.1:0`), exchange wire frames, handle connection refusal, timeouts, and failover inside the cargo integration test process.
> 2. **Tier 2 (Partially Supported): Independent OS Daemon Processes:** Running separate `phantom-cache-demo.exe` processes with distinct OS PIDs and mTLS cluster certificates. Supported by CLI flags, but **NOT** automated in the E2E acceptance runner.
> 3. **Tier 3 (Unsupported / Untested): Physical Multi-Host LAN Validation:** Deploying nodes across distinct physical machines over a network switch (tracked as SETBACK-18 / RISK-P2-01).

| Field | Required Information |
|---|---|
| **Scenario 1–3** | **Node Identity, Configuration, mTLS Setup & Peer Connection** |
| Phase | P2 |
| Test Classification | **Socket Integration Test (Real TLS Sockets in Test Harness)** |
| Prerequisites | Cluster TLS certificates with strict SANs (`node-{id}.phantom-cluster.phantom.internal`) |
| Command | `cargo test --test security_phase3_cluster_tests test_p3_at01_authorized_mtls_connection -j 1` |
| Expected Result | Mutual TLS 1.3 handshake succeeds; peer identity extracted from SAN |
| UI Observation | Overview definition list: Cluster capability reports `experimental` or `enabled` |
| Evidence | `tests/integration/security_phase3_cluster_tests.rs` (24/24 passed) |
| Acceptance | TLS 1.3 handshake verified; RFC 1035 SAN parsed |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 4 (P2-04)**| **Peer GET Semantics (SETBACK-11 / DR-029)** |
| Phase | P2 |
| Test Classification | **Unit Test (In-Process Call / In-Memory Lookup)** & **Socket Integration Test (Wire Framing Handler)** |
| Prerequisites | Node with shared cache |
| Command | `cargo test --test wp3_tests test_shared_cache_node_peer_get_genuine_hit -j 1` |
| Expected Result | Remote peer performs read-only non-mutating cache lookup; misses return genuine `"MISS"` without inserting synthetic values; hits return `"HIT body"` |
| UI Observation | Cache & Memory: Peer miss charges 0 bytes resident |
| Evidence | `tests/unit/wp3_tests.rs:228-299` |
| Acceptance | Non-mutating read-only peer GET semantics strictly verified |
| Validation Status | **Executed PASS (Unit Logic & Loopback Socket Framing; not multi-process)** |

| Field | Required Information |
|---|---|
| **Scenario 5 (P2-05)**| **Peer-Fill Pipeline (SETBACK-14)** |
| Phase | P2 |
| Test Classification | **Socket Integration Test with Real Loopback TCP Sockets** |
| Prerequisites | Node 1 (Remote Primary), Node 2 (Local Gateway) running on loopback TCP |
| Command | `cargo test --test wp3_tests test_peer_read_and_peer_fill_flow -j 1` |
| Expected Result | Node 2 misses locally, queries Node 1 over TCP socket, receives HIT body, populates Node 2's local cache (peer-fill), returns HTTP 200 to client |
| UI Observation | Control Center: Peer-fill hit recorded |
| Evidence | `tests/unit/wp3_tests.rs:324–446` |
| Acceptance | Remote peer hit successfully populates local cache across real TCP socket |
| Validation Status | **Executed PASS (Real Loopback TCP Sockets in Test Harness)** |

| Field | Required Information |
|---|---|
| **Scenario 6 (P2-06)**| **Zero Origin Dispatch on Peer Hit** |
| Phase | P2 |
| Test Classification | **Socket Integration Test with Real Loopback TCP Sockets** |
| Prerequisites | Execution of Scenario 5 |
| Command | `cargo test --test wp3_tests test_peer_read_and_peer_fill_flow -j 1` |
| Expected Result | Request is completely satisfied by peer read over TCP; origin URL is None / origin request counter remains strictly 0 |
| UI Observation | Origin fetches counter remains strictly 0 |
| Evidence | `tests/unit/wp3_tests.rs:404` (zero origin dispatch invariant) |
| Acceptance | Cache hit from peer completely shields origin backend |
| Validation Status | **Executed PASS (Real Loopback TCP Sockets in Test Harness)** |

| Field | Required Information |
|---|---|
| **Scenario 7** | **Peer Miss and Origin Fallback** |
| Phase | P2 |
| Test Classification | **Socket Integration Test with Real Loopback TCP Sockets** |
| Prerequisites | Key misses both local and peer cache |
| Command | `cargo test --test wp3_tests test_peer_miss_falls_back_to_origin -j 1` |
| Expected Result | Peer returns `MISS`; gateway catches miss and fetches from origin backend over TCP; origin counter increments to 1 |
| UI Observation | Origin fetches increments by 1 |
| Evidence | `tests/unit/wp3_tests.rs:449–565` |
| Acceptance | Safe origin fallback without error or hanging |
| Validation Status | **Executed PASS (Real Loopback TCP Sockets in Test Harness)** |

| Field | Required Information |
|---|---|
| **Scenario 8 & 10**| **Abrupt Node Crash & TCP Connection Refusal Failover** |
| Phase | P2 |
| Test Classification | **Socket Integration Test (Real Sockets, Socket Crash Injected)** |
| Prerequisites | Node 1 and Node 2 running; Node 2 listener terminated abruptly |
| Command | `cargo test --test p2_dist_network_tests test_p2_actual_tcp_connection_refused_socket_failover -j 1` |
| Expected Result | Node 1 receives OS `ConnectionRefused` within 20ms; marks Node 2 `Suspect`; fails over to replica |
| UI Observation | Node list: Node 2 transitions from Active to Suspect |
| Evidence | `tests/integration/p2_dist_network_tests.rs` |
| Acceptance | Request served by replica; zero hung client connections |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 9** | **Peer Socket Timeout Handling** |
| Phase | P2 |
| Test Classification | **Socket Integration Test (Real Sockets, Artificial Delay Injected)** |
| Prerequisites | Node 2 injected with 300ms network delay; gateway deadline 60ms |
| Command | `cargo test --test p2_dist_network_tests test_p2_tcp_socket_timeout_failover -j 1` |
| Expected Result | Gateway times out on Node 2 after 60ms; immediately fails over to replica Node 3 |
| UI Observation | Gateway timeout counter recorded |
| Evidence | `tests/integration/p2_dist_network_tests.rs` |
| Acceptance | Bounded timeout strictly enforced; replica serves valid data |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 11–13**| **Heartbeat Failure Detection, Suspect/Failed Transitions & HRW Exclusion** |
| Phase | P2 |
| Test Classification | **Unit Test (State Machine)** & **Socket Integration Test (Dynamic Routing)** |
| Prerequisites | Proactive health probing enabled |
| Command | `cargo test --test wp3_tests test_node_health_state_machine_transitions -j 1` and `test_proactive_failure_detection_and_dynamic_rerouting` |
| Expected Result | Proactive heartbeat detects 3 consecutive failures: `Active -> Suspect -> Failed`; failed node excluded from HRW rendezvous routing |
| UI Observation | Cluster screen: Node state displays `Failed`; routing excludes node |
| Evidence | `tests/unit/wp3_tests.rs:663–771` |
| Acceptance | Failed node dynamically excluded from rendezvous hash ring |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 14** | **Node Recovery and Re-Entry** |
| Phase | P2 |
| Test Classification | **Socket Integration Test (Real Loopback TCP Sockets)** |
| Prerequisites | Previously Failed node restarts TCP service |
| Command | `cargo test --test p2_dist_network_tests test_p2_partition_and_recovery -j 1` |
| Expected Result | Node answers heartbeat; transitions `Recovering -> Active`; HRW restores primary ownership |
| UI Observation | Node state returns to `Active`; traffic resumes |
| Evidence | `tests/integration/p2_dist_network_tests.rs` |
| Acceptance | Recovered primary deterministically resumes key ownership |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 15–16**| **Membership Changes & In-Flight Request Fencing** |
| Phase | P2 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | In-flight invalidations during membership update |
| Command | `cargo test --test security_phase4a_invalidation_tests test_p4a_inflight_fetch_fencing_prevents_stale_writes -j 1`|
| Expected Result | Invalidation increments generation epoch; in-flight fetch detects epoch shift and discards stale write |
| UI Observation | No stale data resurrected |
| Evidence | `tests/integration/security_phase4a_invalidation_tests.rs` |
| Acceptance | Generation fencing guarantees cache consistency during topology shifts |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 17–21**| **Unauthorized Peer, Invalid SAN & Cross-Cluster Rejection** |
| Phase | P2 |
| Test Classification | **Socket Integration Test** |
| Prerequisites | Cluster listener running on TLS port |
| Command | `cargo test --test security_phase3_cluster_tests test_p3_at05_cross_cluster_connection_rejection -j 1` |
| Expected Result | Certificate presenting `other-cluster` rejected with `ClusterSecurityError::ClusterMismatch` |
| UI Observation | Security alert logged; peer rejected |
| Evidence | `tests/integration/security_phase3_cluster_tests.rs` |
| Acceptance | Handshake rejected fail-closed; rogue peer cannot connect |
| Validation Status | **Executed PASS** |

| Field | Required Information |
|---|---|
| **Scenario 22** | **Shutdown & Socket Cleanup** |
| Phase | P2 |
| Test Classification | **Scripted Lifecycle Test** |
| Prerequisites | Multi-node test execution |
| Command | `scripts/run_e2e_acceptance.ps1` (Steps 24–25) |
| Expected Result | Process terminated; OS connection table shows 0 listeners on port 8085 |
| UI Observation | Control Center displays disconnected state pill |
| Evidence | `lab-results/e2e_acceptance_evidence.json` (Step 25) |
| Acceptance | Zero orphaned child processes; sockets released cleanly |
| Validation Status | **Executed PASS** |

---

# SECTION E: P3 — REAL HTTP WORKLOADS & LIVE TELEMETRY

## 1. Workload Categorization & Rigor
To ensure empirical clarity, the workload evaluation is categorized as follows:
- **Deterministic Workload Generation:** Evaluated in `tests/integration/p3_traffic_tests.rs:test_p3_seeded_workload_determinism` using a seeded PRNG (seed 42). Generates bit-identical request sequences and Zipfian rank skews.
- **Actual HTTP Workload Execution:** Evaluated in Step 15 of `scripts/run_e2e_acceptance.ps1` by dispatching 60 real HTTP/1.1 requests via `curl.exe` across 10 distinct cache keys.

## 2. Inspecting Metrics on the Control Center
Navigate to **Traffic & Latency** (`http://localhost:3000/#traffic`):
- **Throughput Tile:** Displays 1s and 10s windowed rate samplers.
- **Hit Ratio Tile:** Displays cumulative hit ratio ($H / (H + M)$) and 10s moving average.
- **Latency Percentiles Tile:** Displays P50, P90, P95, and P99 microsecond-accurate percentiles.
  - *Provenance Rule:* If samples $< 20$, percentiles display grey `UNAVAILABLE` badge. Once $\ge 20$ samples are gathered, green `REAL` badge appears with measured values.
- **Provenance Vocabulary:**
  - `REAL`: Direct atomic measurement from hardware / cache counters.
  - `DERIVED`: Mathematically calculated (ratios, throughput, moving averages).
  - `UNAVAILABLE`: Displayed with reason code when data cannot be accurately measured (never replaced with fake 0s).

---

# SECTION F: P4 — POLICY EVALUATION & ADAPTIVE CONTROLLER

## 1. Eviction Policy Benchmarking (LRU vs S3FIFO vs SIEVE)
Execute the comparative benchmark test under controlled Zipfian traces:
```powershell
cargo test --test p4_pie_tests test_p4_reproducible_policy_benchmark_comparison -j 1
```
*Observable Evidence:*
- **S3FIFO:** Three-queue FIFO (Small, Main, Ghost) filters one-hit wonders effectively.
- **LRU:** Recency-based eviction preserves active working sets but exhibits lower hit ratios under scan-heavy access.
- **Safety Invariant:** All active cache read/write operations strictly preserve memory safety invariants across all policies.

## 2. Origin Concurrency Controller (AIMD)
Verify origin concurrency cap enforcement in the serving path:
1. Ensure gateway is running with `$env:PHANTOM_INTELLIGENCE = "canary"` and `$env:PHANTOM_ORIGIN_CAP_MAX = "10"`.
2. Navigate to **Origin** (`http://localhost:3000/#origin`):
   - Headline: `Enforcing. The gateway is limiting fetches to the origin to the cap shown.`
   - Cap: `10`
   - In-flight fetches: Live atomic gauge.
   - Refused fetches: Increments if concurrency exceeds 10.
3. Automated test:
   ```powershell
   cargo test --test origin_cap_tests -j 1
   ```

## 3. Automatic Guardrail Rollback
```powershell
cargo test --test p4_pie_tests test_p4_pie_adaptive_switching_and_rollback -j 1
```
*Verification:* If a candidate policy violates latency or error regression thresholds (>5%), guardrails automatically rollback the active policy to the safe baseline without dropping resident cache entries.

## 4. Research Integrity & Milestone 6 Hypothesis 1 (M6 H1)
> [!IMPORTANT]
> **Preserved Empirical Finding:** Under Milestone 6 Hypothesis 1 (M6 H1), adaptive control was evaluated against a tuned static baseline. The experimental verdict was **NOT_SUPPORTED** (adaptive policies did not beat a tuned static baseline).  
> Demonstrating P4 confirms that the policy evaluation *mechanism* (AIMD origin gate, guardrails, rollback, and comparative benchmarking) is fully operational and safe. It does **NOT** claim adaptive superiority. Speculative predictive prewarming remains quarantined per ADR-002.

---

# SECTION G: PHANTOM LAB

## 1. Verifying Lab Capabilities
Query the Lab API:
```bash
curl -s http://127.0.0.1:8085/api/v1/lab/capabilities
```
*Expected JSON:*
```json
{
  "security_mode": "anonymous_dev_loopback_only",
  "kinds": {
    "t1_matrix": { "available": true, "tier": "SIMULATED" },
    "scenario_b": { "available": true, "tier": "MEASURED_LOOPBACK" }
  }
}
```

## 2. Starting Scenario B Experiment
```bash
curl -X POST -H "Content-Type: application/json" \
  -d "{\"kind\":\"scenario_b\",\"spec\":{\"repetitions\":1,\"arms\":[\"BASE\",\"STATIC-CAP\"],\"warmup_s\":1,\"healthy_s\":1,\"degraded_s\":1,\"recovery_s\":1,\"rate_rps\":50.0,\"n_keys\":50}}" \
  http://127.0.0.1:8085/api/v1/lab/runs
```
*Response:* Returns `run_id` (e.g. `20261030T180144Z-d1ad2034`) with status `queued` $\to$ `running`.

## 3. Monitoring & Evidence Inspection
- In Control Center: Navigate to **Phantom Lab** $\to$ **Evidence**.
- View real-time progress bar across Warmup, Healthy, Degraded, and Recovery phases.
- On completion, inspect retained artifacts:
  - `run.json`: Run metadata, configuration echo, and final status.
  - `events.jsonl`: Timestamped phase transitions.
  - `summary.json`: Throughput and P50/P95/P99 latency comparisons.
- **Honest Labeling:** Interrupted or exploratory runs display `TRADE_OFF_FAILURE` or `EXPLORATORY` labels, strictly suppressing false `VALID` verdicts (DEC-001).

---

# SECTION H: COMPLETE 26-STEP ACCEPTANCE JOURNEY

To run the automated acceptance suite verifying the backend and API integration:
```powershell
pwsh -File scripts/run_e2e_acceptance.ps1
```

> [!NOTE]
> **Test Scope Notice:** `scripts/run_e2e_acceptance.ps1` is an **Automated Backend & API Integration Suite** operating over real TCP loopback sockets. UI DOM rendering, viewmodels, and provenance contracts are verified separately by the **Browser Test Suite** (`npm --prefix ui test`, 483 passing tests).

### Detailed Step Walkthrough:

1. **Step 1: Start Gateway Binary:** Spawns `target\release\phantom-cache-demo.exe` with origin and lab enabled. Passes when process PID is active. `[Scripted Lifecycle Test]`
2. **Step 2: Backend Health Check:** Issues `GET /health`. Passes when response is `OK`. `[Automated API Test]`
3. **Step 3: Connect UI / Probe APIs:** Probes `/api/v1/metrics` and `/api/v1/lab/capabilities`. Passes when schema is 2 and lab is active. `[Automated API Test]`
4. **Step 4: Verify SSE Stream:** Consumes `/api/events?schema=2` for 2s. Passes when raw data contains `schema: 2`. `[Automated API Test]`
5. **Step 5: Validate Mock Origin:** Queries `http://127.0.0.1:9099/ping`. Passes when mock origin answers on port 9099. `[Automated API Test]`
6. **Step 6: Send Cold GET:** Requests `/api/v1/cache/data/item1`. Passes when header is `X-Cache-Status: MISS`. `[Automated API Test]`
7. **Step 7: Confirm Origin Population:** Checks mock origin counter. Passes when origin observed fetch. `[Scripted State Check]`
8. **Step 8: Send Warm GET:** Requests `/api/v1/cache/data/item1`. Passes when header is `X-Cache-Status: HIT`. `[Automated API Test]`
9. **Step 9: Confirm Zero Origin Fetch:** Compares origin counter before and after warm GET. Passes when count is unchanged. `[Scripted State Check]`
10. **Step 10: Dispatch Concurrent Stampede:** Fires 10 concurrent requests to `/api/v1/cache/data/slow`. Passes when all 10 complete. `[Automated API Test (Concurrency)]`
11. **Step 11: Confirm Single-Flight Coalescing:** Analyzes response headers. Passes when exactly 1 is `MISS` and 9 are `COALESCED`. `[Automated API Test (Concurrency)]`
12. **Step 12: Exercise Cache Memory & Budget:** Queries `/api/v1/metrics`. Passes when `bytes_resident` is $> 0$ under budget. `[Automated API Test (Budget Accounting)]`
13. **Step 13: Purge Cached Entry:** Sends `DELETE /api/v1/cache/data/item1`. Passes when status is `204 No Content`. `[Automated API Test]`
14. **Step 14: Confirm Post-Purge Refetch:** Requests `/api/v1/cache/data/item1`. Passes when header is `X-Cache-Status: MISS`. `[Automated API Test]`
15. **Step 15: Generate Metric Traffic:** Dispatches 60 requests across 10 keys. Passes when `requests_total >= 70` and latency percentiles are computed. `[Automated API Test (Traffic Gen)]`
16. **Step 16: Origin Gate Enforcement:** Queries `/api/v1/metrics`. Passes when `origin_cap.enforcing.v == true` with cap `10`. `[Automated API Test]`
17. **Step 17: Open Phantom Lab:** Queries `/api/v1/lab/capabilities`. Passes when `scenario_b.available == true`. `[Automated API Test]`
18. **Step 18: Start Scenario B:** Posts experiment spec to `/api/v1/lab/runs`. Passes when valid `run_id` is assigned. `[Automated API Test]`
19. **Step 19: Inspect Evidence:** Polls run until terminal state. Passes when state is `completed` and artifacts are generated. `[Automated API Test]`
20. **Step 20: Test Cooperative Cancellation:** Starts a run and issues `POST /api/v1/lab/runs/:id/cancel`. Passes when state is `cancelled` and false VALID is suppressed. `[Automated API Test]`
21. **Step 21: Multi-Node Communication:** Runs `p2_dist_network_tests:test_p2_multi_node_tcp_communication`. Passes when 3 TCP nodes communicate successfully. `[Socket Integration Test]`
22. **Step 22: Verify Peer-Fill Pipeline:** Runs `wp3_tests:test_peer_read_and_peer_fill_flow`. Passes when remote hit populates local cache with 0 origin calls. `[Socket Integration Test]`
23. **Step 23: Test Node Crash & Failover:** Runs socket crash and recovery tests. Passes when connection refusal triggers replica failover and partition recovery. `[Socket Integration Test]`
24. **Step 24: Stop Gateway Cleanly:** Sends termination signal to gateway PID. Passes when process handle exits. `[Scripted Lifecycle Test]`
25. **Step 25: Verify Socket Cleanup:** Inspects TCP table on port 8085. Passes when port 8085 is completely free. `[Scripted Lifecycle Test]`
26. **Step 26: Restart & Readiness:** Starts new gateway process and probes `/health`. Passes when restarted gateway answers `OK`. `[Scripted Lifecycle Test]`

---

# SECTION I: SHUTDOWN AND RESTART

1. **Graceful Teardown:**
   - In the `Start-Phantom-Demo.bat` window, press any key.
   - The script terminates the gateway PID and UI PID, deletes `.api.pid` and `.ui.pid`, and enforces a 2-second OS socket release pause.
2. **Process & Socket Verification:**
   ```powershell
   Get-NetTCPConnection -LocalPort 8085, 3000 -ErrorAction SilentlyContinue
   ```
   *Expected Output:* No connections returned (both ports completely free).
3. **Clean Restart:**
   Execute `Start-Phantom-Demo.bat` or `scripts/run_e2e_acceptance.ps1`. The application starts cleanly without port collision errors.

---

# SECTION J: TROUBLESHOOTING

| Issue | Root Cause | Solution |
|---|---|---|
| **Port 8085 or 3000 Collision** | Stale background process | Run `Stop-Process -Id (Get-NetTCPConnection -LocalPort 8085).OwningProcess -Force` |
| **"The Lab API is switched off"** | `PHANTOM_LAB_ENABLED` was not set | Set `$env:PHANTOM_LAB_ENABLED = "true"` before launching the gateway |
| **"No origin URL configured"** | `ORIGIN_URL` was not set | Set `$env:ORIGIN_URL = "http://127.0.0.1:9099"` and ensure the mock origin is running |
| **Origin Cap says "Not enforcing"** | Intelligence mode is `off` | Set `$env:PHANTOM_INTELLIGENCE = "canary"` or `"active"` |
| **Latency percentiles "Unavailable"** | Fewer than 20 observations | Send at least 20 HTTP requests to gather a statistically valid reservoir sample |
| **CORS failure on non-loopback** | Browser origin not in allowlist | Set `$env:PHANTOM_CORS_ORIGINS = "http://<my-host>:3000"` |
| **JWT 401 Unauthorized** | Token missing or signature invalid | Pass valid bearer token with `Operator` role: `Authorization: Bearer <valid_jwt>` |
