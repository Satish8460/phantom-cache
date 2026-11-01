# P1–P4 Acceptance Matrix

This matrix provides the phase-by-phase implementation, real execution, security/failure verification, evidence retention, and verdict for each milestone in the P1–P4 demonstration programme.

| Phase | Core Capability | Real Execution Verified | Security & Failure Verification | Retained Evidence Artifacts | Verdict |
|---|---|---|---|---|---|
| **P1** | **Resource Safety & Ingress Hardening**<br>RFC 9111 cache semantics, single-flight coalescing, logical byte budgeting, SSRF defense, JWT authentication & RBAC. | Executed cold GET (MISS), warm GET (HIT), DELETE invalidation (204), 10-client concurrent stampede (1 MISS, 9 COALESCED). | Oversize body rejection (413), SSRF cloud metadata blocked, invalid JWT fail-closed, tenant isolation verified. | `lab-results/e2e_acceptance_evidence.json`<br>`tests/integration/security_phase1_tests.rs`<br>`tests/integration/security_phase2_upstream_tests.rs` | **PASS** |
| **P2** | **Distributed Resilience & Fault Injection**<br>Deterministic HRW (FNV-1a), genuine peer GET, peer-fill caching, heartbeat probing (`Active` $\to$ `Suspect` $\to$ `Failed`), cluster mTLS with strict SAN. | Real multi-node TCP communication across 3 distinct nodes; peer hit avoids origin call; peer miss falls back safely. | Abrupt socket crash connection refused failover; 60ms timeout handling; cross-cluster mismatch rejection; revoked peer eviction. | `lab-results/e2e_acceptance_evidence.json`<br>`tests/integration/p2_dist_network_tests.rs`<br>`tests/integration/security_phase3_cluster_tests.rs`<br>`tests/unit/wp3_tests.rs` | **PASS (Harness-Managed Real Sockets)**<br>*(Independent OS daemons & physical multi-host LAN open)* |
| **P3** | **Real HTTP Workloads & Live Telemetry**<br>Zipfian request distributions, rate sampling, P50/P95/P99 latency reservoir, Schema 2 SSE stream, provenance contract (REAL/DERIVED/UNAVAILABLE). | Dispatched 60+ requests across varying keys; observed atomic counter increments; live SSE stream delivery to browser. | Percentiles unavailable under 20 samples; zero synthetic 0s; simulated data rejected in live mode; UI contract validated. | `lab-results/e2e_acceptance_evidence.json`<br>`tests/integration/p3_traffic_tests.rs`<br>`ui/test/` (483 passing tests) | **PASS (API-Driven)** |
| **P4** | **Adaptive Policy Evaluation & Guardrails**<br>Eviction comparisons (LRU vs S3FIFO vs SIEVE), AIMD origin concurrency controller, automatic rollback, Phantom Lab Scenario B live runner. | Scenario B live execution spawned gateway process, ran phases, persisted artifacts; comparative eviction benchmark executed. | Cooperative cancellation; false VALID suppression (DEC-001); rollback on regression; M6 H1 NOT_SUPPORTED preserved. | `lab-results/e2e_acceptance_evidence.json`<br>`lab-results/runs/`<br>`tests/integration/p4_pie_tests.rs`<br>`tests/integration/lab_live_scenario_b_tests.rs` | **PASS (Mechanism Verified; Superiority Not Claimed)** |

---

## Acceptance Verification Summary
- **Total Test Suites Executed:** 16 targeted suites.
- **Total Rust Tests Passed:** Over 230 passing integration/unit tests.
- **Total UI Frontend Tests Passed:** 483 passed (0 failed).
- **26-Step Acceptance Journey:** 26/26 passed (100% pass rate in 23.7s).
- **Research Freeze Record:** 56/56 file hashes verified intact (0.05s).
- **Working Tree Preservation:** Exactly 48 modified tracked files strictly preserved.
