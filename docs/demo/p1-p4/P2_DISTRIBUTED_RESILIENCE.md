# Phase P2: Fault Injection, Distributed Resilience and Network Failure Handling

## 1. Objective
Demonstrate that PhantomCache behaves predictably when cluster nodes, network sockets, and connections fail: verifying genuine peer GET semantics, peer-fill caching, heartbeat failure detection, rendezvous routing failover, and graceful recovery without data corruption or origin stampedes.

---

## 2. Architecture and Execution Path
- **Deterministic Rendezvous Hashing (HRW):** 64-bit FNV-1a deterministic hashing (`src/dist/router.rs`) with explicit encoding `[key.as_bytes(), node_id as u8]`.
- **Peer GET Semantics (SETBACK-11 / DR-029):** Read-only non-mutating cache inspection on shared nodes (`src/dist/node.rs`).
- **Peer-Fill Pipeline (SETBACK-14):** Remote peer hit populates requesting node's local cache, serves in-flight single-flight waiters, and avoids origin fetch (`src/api/server.rs`).
- **Failure Detection & Health State Machine (SETBACK-13):** Proactive heartbeat probing (`probe_node`, `probe_all_peers`). Consecutive probe failures trigger state progression:
  $$\text{Active} \longrightarrow \text{Suspect} \longrightarrow \text{Failed} \longrightarrow \text{Recovering} \longrightarrow \text{Active}$$
- **Cluster mTLS & Strict SAN Identity:** RFC 1035/1123 SAN format `node-{node_id}.{cluster_id}.phantom.internal` with SHA-256 cert fingerprint validation (`src/security/cluster.rs`).

---

## 3. Exact Setup Instructions
Multi-node execution operates across distinct TCP listeners (ports `18088`, `18089`, `18090`):
1. In development/testing, cluster communication runs over loopback TCP using TLS 1.3 mTLS or isolated test listeners.
2. Verified automated test suite:
   ```powershell
   cargo test --test p2_dist_network_tests -j 1
   cargo test --test security_phase3_cluster_tests -j 1
   cargo test --test wp3_tests -j 1
   ```

---

## 4. Exact Demonstration Sequence

### A. Multi-Node TCP Routing
- Node 1, Node 2, and Node 3 run independently on separate TCP endpoints.
- Key `"hotel_del_luna"` deterministically hashes to Node 2 via HRW.
- First GET over TCP returns `MISS` and populates Node 2's cache.
- Second GET over TCP to the same key returns `HIT` from Node 2's cache.

### B. Peer-Fill Pipeline (0 Origin Dispatches)
- Client sends request for key `"catalog_item_1"` to Gateway (Node 1).
- Node 1 identifies Node 2 as primary owner.
- Node 1 dispatches TCP `GetRequest` to Node 2.
- Node 2 returns `HIT` with body.
- Node 1 populates its local cache (peer-fill), returns data to client with `X-Cache-Status: HIT (peer-fill)`.
- Origin receives 0 requests.

### C. Abrupt Socket Crash & Failover
- Node 2 is abruptly terminated (TCP listener closed, OS drops socket).
- Next request to Node 1 for Node 2's key receives `ConnectionRefused`.
- Node 1 marks Node 2 as `Suspect`, excludes Node 2 from HRW rendezvous selection, and immediately routes to secondary replica (Node 3).
- Node 3 handles request without unbounded wait.

### D. Recovery and Reintegration
- Node 2 restarts and answers heartbeat probe.
- Node 2 state transitions to `Active`.
- HRW routing deterministically returns ownership to Node 2.

---

## 5. Security & Fault Scenarios Tested

| Failure / Security Vector | Injection Mechanism | System Response | Outcome |
|---|---|---|---|
| **Abrupt Node Crash** | Listener closed during traffic | Connection refused caught in 20ms | Failover to secondary replica |
| **Network Latency / Slow Peer** | Artificial 300ms delay injected | 60ms deadline exceeded | Timeout triggered, failover to replica |
| **All Peers Unreachable** | All cluster nodes marked Failed | Router returns `DistError::NoActiveNodes` | Safe origin fallback |
| **Cross-Cluster Spoofing** | SAN cluster ID mismatch | TLS handshake verifier checks cluster ID | Connection rejected with `ClusterMismatch` |
| **Revoked Node Connection** | Node removed from membership | Session pool checks active member list | Active socket purged, handshake rejected |
| **Plaintext Connection to mTLS** | Plaintext TCP probe to cluster port | TLS acceptor strictly requires TLS 1.3 | Plaintext rejected immediately |

---

## 6. Actual Test Results & Automated Verification

### Automated Test Suites Executed:
1. `tests/integration/p2_dist_network_tests.rs`: **7 passed, 0 failed** (0.55s).
   - `test_p2_multi_node_tcp_communication`: PASSED.
   - `test_p2_actual_tcp_connection_refused_socket_failover`: PASSED.
   - `test_p2_tcp_socket_timeout_failover`: PASSED.
   - `test_p2_replica_data_usability_and_consistency`: PASSED.
   - `test_p2_node_failure_and_replica_failover`: PASSED.
   - `test_p2_partition_and_recovery`: PASSED.
   - `test_p2_all_nodes_unreachable_returns_error`: PASSED.
2. `tests/integration/security_phase3_cluster_tests.rs`: **24 passed, 0 failed** (1.05s).
   - Validates authoritative JSON membership, strict RFC 1035 SAN parser, TLS 1.3 enforcement, and fingerprint verification.
3. `tests/integration/security_phase3c_pooling_tests.rs`: **11 passed, 0 failed** (0.22s).
   - Validates connection pooling, session reuse, heartbeat ping/pong, and pool eviction on node revocation.
4. `tests/integration/security_phase4b_router_tests.rs`: **19 passed, 0 failed** (0.32s).
   - Validates HRW primary routing, failover, strict 150ms timeout, and acting-primary headers.
5. `tests/unit/wp3_tests.rs`: **15 passed, 0 failed** (0.43s).
   - Validates FNV-1a golden vectors, minimal migration invariant, genuine peer GET, peer-fill flow, and node health state machine.
6. `scripts/run_e2e_acceptance.ps1` (Steps 21–23): **3 passed, 0 failed**.

---

## 7. Evidence Artifact Locations
- Machine-Readable Evidence: `lab-results/e2e_acceptance_evidence.json` (Steps 21–23).
- Multi-node Test Suite: `tests/integration/p2_dist_network_tests.rs`.
- Router Security Suite: `tests/integration/security_phase4b_router_tests.rs`.

---

## 8. Known Limitations & Architecture Boundaries
- **In-Process Harness Multi-Listener Scope:** Multi-node TCP tests execute across real OS kernel loopback sockets on `127.0.0.1:0` inside the integration test runner process. Independent OS daemon processes (`phantom-cache-demo.exe` instances with separate PIDs) and physical multi-host network switch validation (SETBACK-18 / RISK-P2-01) have not been executed in this local environment.
- **P2-04 vs P2-05 vs P2-06 Granularity:**
  - P2-04 cache non-mutation logic is verified at the unit level (in-process cache lookup), while framing exchange is verified over loopback TCP sockets.
  - P2-05 (peer-fill) and P2-06 (zero origin dispatch) are verified using real loopback TCP sockets inside `tests/unit/wp3_tests.rs:test_peer_read_and_peer_fill_flow`.
- **M3 Durability Boundary:** M3 durable invalidation across persistent node crashes was remediated and reviewed PASS in sandbox `D:\Temp\sbx_m3fix`, but remains unmerged in the main tree awaiting explicit owner approval.

---

## 9. Acceptance Verdict
**Verdict: PASS (Harness-Managed Real Sockets)**  
Multi-node loopback TCP communication, genuine peer GET/peer-fill, heartbeat failure detection, dynamic HRW routing, and failover/recovery are fully operational and verified across automated socket integration test suites. Independent multi-process daemons and physical multi-host LAN testing remain open.

