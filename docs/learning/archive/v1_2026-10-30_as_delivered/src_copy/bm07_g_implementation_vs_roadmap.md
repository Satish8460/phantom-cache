# Appendix G: Implementation versus roadmap matrix

This appendix presents the matrix from Chapter 8, section 8.12, with one added column that points to the chapter that teaches each capability. There are 44 rows. Each row carries exactly one status label, and the labels, capability names and "Why" text are unchanged from Chapter 8. Only the grouping is new: the rows are grouped under the three pillars of Chapter 1 (distributed systems engineering, workload intelligence, product excellence with observability) and, inside the first pillar, by area. Nothing here is INDEPENDENTLY VERIFIED by a human; the labels describe the state of the working tree and the project records on 2026-10-30.

## G.1 Status vocabulary

| Label | Meaning in this handbook |
|---|---|
| IMPLEMENTED | Code exists. This says nothing about whether it works. |
| TESTED | A named test exercises it. The chapter says what the test does not prove. |
| INDEPENDENTLY VERIFIED | A human or external reviewer other than the author checked it. No item in the project qualifies; AI reviews are "AI second looks". |
| PARTIAL | Part of the capability exists, with named gaps. |
| PROTOTYPE ONLY | Exists in a library, a sandbox or a scratch patch, but is not integrated into the product path. |
| DESIGNED BUT NOT IMPLEMENTED | There is a design or a proposal, and no working code. |
| DEFERRED | Not done yet; it is queued behind a condition, a decision or a quiet machine. |
| BLOCKED | Cannot proceed without a resource the project lacks, such as physical devices or human reviewers. |
| UNVERIFIED | No check has been run, or none could be run; the reason is stated. |

Label counts across the 44 rows: TESTED 15, PARTIAL 8, DEFERRED 6, DESIGNED BUT NOT IMPLEMENTED 5, PROTOTYPE ONLY 4, UNVERIFIED 3, BLOCKED 2, IMPLEMENTED 1. INDEPENDENTLY VERIFIED appears in no row. Reading rule: TESTED means IMPLEMENTED and a named test exercises it (Chapters 4 and 5 write "IMPLEMENTED, TESTED"; same label); "IMPLEMENTED, PARTIAL" is written PARTIAL.

## G.2 Pillar 1: Distributed systems engineering

### G.2.1 Single-node cache engine

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Single-flight coalescing | TESTED | Named integration tests; coalescing under cancellation reviewed only by reading | Ch 4, 4.4 |
| Generation fencing against DELETE | TESTED | `test_delete_fences_in_flight_fill_race`; a single interleaving | Ch 4, 4.5 |
| Tenant and global epoch fencing | PARTIAL | IMPLEMENTED, but the G4A-01 race is reproduced in a sandbox and its fix is not integrated; G4A-08 and G4A-17 open | Ch 4, 4.7 |
| Byte accounting (I1 to I8) | TESTED | `audit_bytes` equality tests; G4A-13 drift on a rare path | Ch 4, 4.6 |
| Freshness (RFC 9111 subset) | PARTIAL | `Age`, `Expires: 0`, stale-if-error scope gaps (G4A-04 to -06) | Ch 4, 4.8 |
| Default TTL setting | PARTIAL | Validated, not wired | Ch 2, 2.6.1; Ch 4, 4.8 |
| Stale-while-revalidate, stale-if-error, hard stale cap | TESTED | Counter-based gate is weak (G4A-18) | Ch 4, 4.8 |
| SIEVE (default) and LRU eviction | TESTED | Weakly: pinned scans are not O(1) (G4A-20); no hit-ratio check against the literature; no release benchmark | Ch 4, 4.9 |
| S3-FIFO eviction | PARTIAL | IMPLEMENTED with the open defect G4A-02 (tombstones); not selected by default | Ch 4, 4.9 |
| Lazy sweeper | PARTIAL | Scans the shard under the lock; shutdown channel dropped (D-2) | Ch 4, 4.7 |

### G.2.2 Security, ingress and tenancy

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| HTTP ingress hardening | PARTIAL | G4B-01, -02, -04, -11 open | Ch 6, 6.5 |
| JWT roles, mTLS identity | TESTED | M7 security tests; lifecycle gaps open | Ch 6, 6.2 and 6.4 |
| SSRF upstream filter | TESTED | Range gaps (G4B-15) | Ch 6, 6.8 |
| Rate limiting | DEFERRED | SEC-M7-11 queued | Ch 6, 6.8; Ch 8, 8.3.7 |
| Tenant keyspace isolation | TESTED | Tenant-keyed entries and epochs | Ch 6, 6.6 |
| Tenant quotas, fair eviction | DESIGNED BUT NOT IMPLEMENTED | Frozen core | Ch 6, 6.6; Ch 8, 8.9.2 |

### G.2.3 Cluster and distribution

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Cluster invalidation fan-out, repair, anti-entropy | TESTED | IMPLEMENTED and tested on loopback, debug build, at most four nodes; labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED; 204 is not a cluster-wide acknowledgement (G4B-09) | Ch 5, 5.5 |
| Durable invalidation | PROTOTYPE ONLY | Sandbox patch, not integrated | Ch 5, 5.9; Ch 8, 8.9.1 |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`), but no code path lets a heartbeat change node state (G4B-12) | Ch 5, 5.2 |
| Runtime membership and revocation | PARTIAL | Machinery, no production trigger | Ch 5, 5.2; Ch 6, 6.4 |
| Rendezvous routing for DELETE primary | IMPLEMENTED | Unweighted, unstable hash | Ch 5, 5.3 |
| Peer-fill, coordinated reads, stable placement | DESIGNED BUT NOT IMPLEMENTED | A stub handler exists and is a defect, not the feature | Ch 5, 5.8; Ch 8, 8.3.1 |
| Multi-host operation | BLOCKED | No devices | Ch 8, 8.9.3 |

## G.3 Pillar 2: Workload intelligence

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Static origin concurrency cap | TESTED | Scenario B and unit tests | Ch 4, 4.4; Ch 5, 5.13 |
| AIMD cap controller | TESTED | Arithmetic and cap unit tests only; on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT SUPPORTED | Ch 5, 5.13 and 5.14 |
| Signals, shadow evaluator, decision engine | PROTOTYPE ONLY | Not wired; unsound statistics (F12, F13) | Ch 5, 5.12 and 5.13 |
| Legacy PIE | PROTOTYPE ONLY | `pie: None` | Ch 5, 5.13 |
| Intelligence `canary` mode | PARTIAL | Cap only; no canary or rollback in serving | Ch 5, 5.13 |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY | Library in `src/intelligence/signals.rs`, unit-tested and used by the lab simulator's admission filter, not wired into the serving path; the older `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test | Ch 5, 5.12 |
| Event tap, pre-warming, forecasting | DESIGNED BUT NOT IMPLEMENTED | Experiments E0, E9, E10 first | Ch 8, 8.5 and 8.9.4 |
| ML selector | DEFERRED | Gate unmet; negative claim unverified | Ch 5, 5.10; Ch 8, 8.5 |

## G.4 Pillar 3: Product excellence, observability and demonstrability

### G.4.1 Telemetry, Lab, evidence and UI

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Telemetry log-linear histogram | TESTED | Recording and bucket arithmetic only (`telemetry_redesign_tests`); the histogram is fed but has no production percentile reader (Chapter 6, 6.12.2) | Ch 5, 5.11; Ch 6, 6.12.2 |
| Lab API and Scenario B | TESTED | Exploratory; U-01 defect open | Ch 7, 7.5 and 7.7 |
| Sealed M6 evaluation | TESTED | Executed once, Tier 1 simulated, not human-reviewed | Ch 7, 7.6; Ch 5, 5.14 |
| T1 to T2 (simulator to real) agreement | UNVERIFIED | No agreement study run | Ch 7, 7.5.3 |
| Release-profile performance measurements | DEFERRED | Needs a quiet machine | Ch 6, 6.12 and 6.13 |
| Dashboard UI I1 to I4 | TESTED | Automated checks only | Ch 3, 3.11; Ch 7, 7.7.2 |
| Visual acceptance of the UI (O8-2) | UNVERIFIED | Owner decision open | Ch 8, 8.2.3 and 8.9.5 |
| Demo Center and `demo-1` stream | DESIGNED BUT NOT IMPLEMENTED | Proposal only | Ch 8, 8.9.5 |
| Human review of design, usability, security | BLOCKED | No human reviewer | Ch 8, 8.2.3 and 8.13.1 |

### G.4.2 Operations and release

| Capability | Status | Why (as stated in Ch 8, 8.12) | Chapter pointer |
|---|---|---|---|
| Graceful connection drain | DEFERRED | Integration item D-1 | Ch 8, 8.3.7 and 8.5 |
| Linux and container deployment | UNVERIFIED | Never run | Ch 2, 2.9.3; Ch 3, 3.10 |
| Metrics export, TLS and config reload | DEFERRED | Not provided | Ch 8, 8.3.7 and 8.5 |
| Dependency advisory scan and SBOM | DEFERRED | Queued | Ch 3, 3.13; Ch 6, 6.10 |

## G.5 Reconciled labels

Earlier drafts gave four capabilities different labels in Chapter 5, Chapter 4 and Chapter 8. They were reconciled before this appendix was regenerated, and Chapter 8, 8.12 is the source of the rows above. The decisions, so that a reader who meets an older copy is not surprised:

::: caution
**Heartbeat.** Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`). Heartbeat-based failure detection and state change is DESIGNED BUT NOT IMPLEMENTED (G4B-12): no code path lets a heartbeat change a node's state. The same split is used in Chapters 2, 5 and 8 and in Appendices D and F.

**Hot-key detection.** `HeavyHitterDetector` (Count-Min plus Space-Saving) exists, is unit-tested and is used by the lab simulator's admission filter, but no request handler or background task calls it: PROTOTYPE ONLY (library). Only the event tap, pre-warming and forecasting are DESIGNED BUT NOT IMPLEMENTED.

**Telemetry log-linear histogram and AIMD cap controller.** Both are TESTED in the sense above (histogram: recording and bucket arithmetic; AIMD: arithmetic and cap unit tests). The histogram is fed but has no production percentile reader. The AIMD gate is on the serving path only when a cap is configured in canary or active mode (default off), and its benefit is NOT SUPPORTED.

**Fencing, eviction, cluster.** Generation fencing is TESTED (one interleaving); epoch fencing is PARTIAL (G4A-01 race, fix not integrated). SIEVE and LRU are TESTED weakly; S3-FIFO is PARTIAL (open defect G4A-02). The cluster fan-out, repair and anti-entropy are IMPLEMENTED and TESTED on loopback only, labelled EXPERIMENTAL in `main.rs`, effectiveness UNVERIFIED.
:::
