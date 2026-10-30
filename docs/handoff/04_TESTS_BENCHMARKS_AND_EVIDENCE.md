> Notice (2026-10-30): This handoff file is a historical record of the state on 2026-10-28 and is not edited. Parts of it are superseded: "Phase 4 accepted / zero P0 / formally accepted" and similar statements are not supported by the later audit ([handoff CORRECTIONS](CORRECTIONS.md), `CLAUDE.md` sections 6 and 6b); the test count "242 tests / 19 targets" is a 2026-10-28 baseline, not the current one ([docs/STATUS.md](../STATUS.md)); and descriptions of cluster cache sharing may predate the mTLS cluster path, where the local node shares the serving cache (`src/main.rs:332`).
>
> Original text below is unchanged.

# PhantomCache — Tests, Benchmarks, and Verification Evidence

**Document ID:** `docs/handoff/04_TESTS_BENCHMARKS_AND_EVIDENCE.md`  
**Purpose:** Comprehensive inventory of automated test targets, commands, benchmark measurements, and verification evidence.  
**Audience:** Test engineers, QA automation specialists, and AI coding assistants.  
**Authoritative Baseline Date:** October 28, 2026  

---

## 1. Automated Test Suite Target Inventory (242 Tests Across 19 Targets)

The complete PhantomCache test harness is executed sequentially with single-threading (`-j 1`) to guarantee deterministic timing, socket isolation, and deadlock prevention.

### Command
```powershell
cargo test --all-targets -j 1
```

### Complete Target Breakdown
| # | Target Name | File Path | Tests Passed | Status | Execution Time |
| :-: | :--- | :--- | :---: | :---: | :---: |
| 1 | `phantom_cache_demo` (lib unit) | `src/lib.rs` | 16 | **PASS** | 0.00s |
| 2 | `phantom_cache_demo` (bin unit) | `src/main.rs` | 16 | **PASS** | 0.00s |
| 3 | `coalesce_tradeoffs_tests` | `tests/integration/coalesce_tradeoffs_tests.rs` | 5 | **PASS** | 0.60s |
| 4 | `m1_http_tests` | `tests/integration/m1_http_tests.rs` | 32 | **PASS** | 0.32s |
| 5 | `p2_dist_network_tests` | `tests/integration/p2_dist_network_tests.rs` | 7 | **PASS** | 0.55s |
| 6 | `p3_traffic_tests` | `tests/integration/p3_traffic_tests.rs` | 5 | **PASS** | 0.16s |
| 7 | `p4_pie_tests` | `tests/integration/p4_pie_tests.rs` | 6 | **PASS** | 1.07s |
| 8 | `security_phase1_tests` | `tests/integration/security_phase1_tests.rs` | 15 | **PASS** | 1.01s |
| 9 | `security_phase2_upstream_tests` | `tests/integration/security_phase2_upstream_tests.rs` | 23 | **PASS** | 0.04s |
| 10 | `security_phase3_cluster_tests` | `tests/integration/security_phase3_cluster_tests.rs` | **24** | **PASS** | 1.22s |
| 11 | `security_phase3b_framing_tests` | `tests/integration/security_phase3b_framing_tests.rs` | 13 | **PASS** | 0.24s |
| 12 | `security_phase3c_pooling_tests` | `tests/integration/security_phase3c_pooling_tests.rs` | 11 | **PASS** | 0.21s |
| 13 | `security_phase4a_invalidation_tests`| `tests/integration/security_phase4a_invalidation_tests.rs` | 14 | **PASS** | 0.04s |
| 14 | `security_phase4b_router_tests` | `tests/integration/security_phase4b_router_tests.rs` | 19 | **PASS** | 0.32s |
| 15 | `security_phase4c_antientropy_tests`| `tests/integration/security_phase4c_antientropy_tests.rs` | **18** | **PASS** | 1.01s |
| 16 | `wp1_tests` | `tests/unit/wp1_tests.rs` | 5 | **PASS** | 1.56s |
| 17 | `wp2_tests` | `tests/unit/wp2_tests.rs` | 5 | **PASS** | 0.00s |
| 18 | `wp3_tests` | `tests/unit/wp3_tests.rs` | 3 | **PASS** | 0.00s |
| 19 | `wp4_tests` | `tests/unit/wp4_tests.rs` | 5 | **PASS** | 0.00s |
| **TOTAL** | **19 Targets** | | **242** | **100% PASS** | **~8.35s** |

---

## 2. Static Analysis & Lint Verification

### 2.1 Clippy Verification
```powershell
cargo clippy --all-targets -j 1 -- -D warnings
```
- **Result:** Finished with exit code 0. Zero warnings, zero errors across all 19 targets.

### 2.2 Whitespace & Formatting Verification
```powershell
git diff --check src/ tests/ docs/security/
```
- **Result:** Exit code 0. Zero trailing whitespace or formatting defects.

---

## 3. Reconciliation of Historical Test Count Claims

| Report / Snapshot | Reported Target Count | Reported Test Count | Discrepancy & Authoritative Reconciliation |
| :--- | :---: | :---: | :--- |
| **Stage 4B Completion Report** | 18 targets | 220 tests | Baseline before Stage 4C independent audit. |
| **Stage 4C Audit Report** | 19 targets | 234 tests | Reconciled (+14 tests): +6 Stage 4C regression tests (`test_p4c_sec01`..`06`), +7 `p2_dist_network_tests`, +1 Stage 4B overflow test (`test_p4b_sec02`). |
| **Gate B Remediation Report** | 19 targets | **242 tests** | Reconciled (+8 tests): +4 cluster wiring tests (`test_p3_sec05`..`08` in `security_phase3_cluster_tests`), +4 anti-entropy tests (`test_p4c_sec07`..`10` in `security_phase4c_antientropy_tests`). |

---

## 4. Benchmark Methodology & Measurement Disclaimers

### 4.1 Measured vs Estimated Performance Metrics
| Metric | Measured Value | Measurement Environment | Scope & Limitations |
| :--- | :---: | :--- | :--- |
| **Cache Hit Latency** | $P_{50} = 10\text{ ms}$, $P_{99} = 15\text{ ms}$ | Simulated discrete-event engine (`sim::traffic`) | Includes synthetic network jitter (10ms ± 2ms) and M/M/1 queuing. In-memory hash lookup is $\le 10\ \mu\text{s}$. |
| **In-Flight Coalescing** | 100% stampede suppression | `tests/integration/coalesce_tradeoffs_tests.rs` | 100 concurrent requests coalesce to exactly 1 origin fetch. |
| **Multi-Node TCP Latency** | $P_{99} \le 20\text{ ms}$ | In-process loopback sockets (`127.0.0.1`) | **Loopback Micro-Benchmark Only.** Does not reflect physical multi-rack network hops or cross-switch MTU. |
| **Tenant Invalidation** | $\le 2.0\text{ ms}$ for 10k entries | `tests/integration/security_phase4a_invalidation_tests.rs` | $O(1)$ tenant epoch increment; no shard scanning. |
| **Ring Buffer Resident Memory** | $\le 4.25\text{ MiB}$ | `tests/integration/security_phase4c_antientropy_tests.rs` | `estimated_resident_memory_bytes()`: 4.0 MiB payload + 160 KiB slots + 65 KiB allocator padding. |

### 4.2 Explicit Untested Scenarios (Physical & Scale Limitations)
1. **Multi-Rack Physical Deployments:** All automated networking tests execute over localhost loopback (`127.0.0.1`). Real physical NIC buffer scheduling, packet loss across switches, and MTU fragmentation have not been physically bench-tested.
2. **WAN / Multi-Region Latency:** Cross-datacenter replication across $> 50\text{ ms}$ links is not supported in Phase 4.
3. **Heavy Long-Running Invalidation Storms:** While tested up to 200,000 keys in memory, sustained 24-hour invalidation churn under memory pressure requires Phase 5 soak testing.

