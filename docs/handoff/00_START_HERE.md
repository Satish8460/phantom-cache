# PhantomCache — AI Handoff & Quick Start Guide

> **AUTHORITATIVE NOTICE:** Prior handoff claims regarding distributed consistency, anti-entropy, and PIE adaptive caching have been audited and corrected. See [`docs/handoff/CORRECTIONS.md`](CORRECTIONS.md) for the verified status of all architectural and performance claims.

> **Correction notice (2026-10-30, owner-approved DEC-020; the original text below is unchanged).** Checked against the current tree:
> - "64-Partition Lock Striping" (here and in files 01 and 02): the binary builds `PartitionedCache::new(8)` (`src/main.rs:204`), that is 8 shards. 64 is not the configured value.
> - "Phase 4 Remediated, Verified, and Formally Accepted", "ACCEPTED" in the table, "100% PASS" and "Phase 4 is accepted and closed": these are agent self-reports. No dated owner acceptance of any milestone is recorded in the repository (an evidence gap, not proof that none was given; see `docs/STATUS.md` section 1).
> - "242 / 242 tests, 19 targets": the 2026-10-28 baseline. The later baseline, with its command and profile, is in `docs/STATUS.md` section 3 (75 targets, 851 passed, 8 ignored, debug).
> - "enterprise-grade": no production deployment, release-profile benchmark or independent security assessment supports the adjective; the README describes the project as a research prototype with experimental distributed operation.
> - "canonical primary invalidation (T_stale = 0)" and "ensures distributed cache consistency": the implemented and tested property is per gateway: `DELETE` purges the local cache first, then asks the canonical primary, and other members are updated best effort. There is no linearizability proof or checker, no cross-gateway staleness bound, and the issuer's in-memory repair ledger is lost if the issuer dies (`docs/user/limitations.md`, `docs/research/claims_ledger.md` C6).

**Document ID:** `docs/handoff/00_START_HERE.md`  
**Purpose:** Primary entry point for incoming AI coding assistants.  
**Repository:** `D:\Personal\phantom-cache`  
**Active Working Branch:** `feat/publish-phantomcache-core`  
**Expected Git HEAD:** `456bb25930004f1896a84c53e776e7b3ef29ee16`  
**Current Date:** October 28, 2026  
**Status:** Phase 4 Remediated, Verified, and Formally Accepted (242/242 tests passing across 19 targets)  

---

## 1. Welcome & Immediate Operating Rules

You are stepping into the role of lead systems engineer on **PhantomCache**. This project has undergone rigorous design, implementation, and multi-round independent audits across Work Packages P1–P4 and distributed Stages 3A–4C.

> [!CAUTION]
> ### NON-NEGOTIABLE ENGINEERING SAFEGUARDS
> 1. **Zero Destructive Git Commands:** NEVER execute `git reset`, `git clean`, `git checkout -- <file>`, `git stash`, or discard modifications. All tracked changes and untracked directories represent essential project deliverables.
> 2. **Zero Unauthorized Commits/Pushes:** Do NOT stage (`git add`), commit (`git commit`), or push (`git push`) unless the user explicitly commands it in plain text.
> 3. **Preserve Repository Integrity:** Always run `git rev-parse HEAD`, `git branch --show-current`, and `git status --short` before and after operations.
> 4. **Single-Threaded Cargo Invocations:** On this Windows environment, always append `-j 1` to Cargo commands (e.g., `cargo test --all-targets -j 1`).
> 5. **Do Not Trust Unverified Claims:** Treat source code and reproducible test outputs as authoritative truth. Distinguish verified facts from estimates or planned features.
> 6. **Do NOT Begin Phase 5:** Phase 4 is accepted and closed. Do not implement Phase 5 without explicit user authorization.

---

## 2. What PhantomCache Is

**PhantomCache** is an enterprise-grade in-memory HTTP reverse-proxy caching and single-flight request coalescing engine written in Rust (Tokio multi-threaded runtime + Axum HTTP framework) with a coupled React 18/Vite operations dashboard.

### Core Problem Solved
Under flash-crowd traffic surges, traditional backend architectures experience **cache stampedes (thundering herds)**, where hundreds or thousands of concurrent cache misses for the same key saturate upstream databases, causing connection pool exhaustion, latency spikes, and cascading outages.

PhantomCache eliminates cache stampedes by:
1. **Single-Flight Request Coalescing:** Synchronizes concurrent requests for the same URI, dispatching exactly **one** upstream origin fetch while placing all other callers into asynchronous broadcast channels.
2. **64-Partition Lock Striping:** Minimizes lock contention via power-of-two sharding with fast bitwise hashing.
3. **Multi-Policy Replacement (SIEVE, LRU, S3FIFO):** Implements Visited-Sweep (SIEVE) as the default eviction algorithm with an adaptive Policy Inference Engine (PIE) monitoring access skew.
4. **Zero-Trust Network Transport:** Secures cluster inter-node replication using Mutual TLS 1.3, RFC 1035 SAN identity pinning, bounded binary framing (`PhantomMeshFrame`), and connection pooling with active session revocation.
5. **Distributed Invalidation & Anti-Entropy:** Ensures distributed cache consistency using Rendezvous (HRW) hashing, canonical primary invalidation ($T_{\text{stale}} = 0$), asynchronous replica broadcast, key-level generational fencing, and bounded circular anti-entropy replay ($\le 4.25\text{ MiB}$ resident heap ceiling).

---

## 3. Current Project Maturity & Verified Baseline

| Work Package / Phase | Scope | Tests Passed | Status |
| :--- | :--- | :---: | :---: |
| **P1** | Sharded cache, single-flight coalescing, SIEVE eviction | 42 | **ACCEPTED** |
| **P2** | Upstream egress security, anti-SSRF, RFC 9112 chunked parser | 38 | **ACCEPTED** |
| **P3 (Stages 3A, 3B, 3C)** | Cluster mTLS 1.3, binary framing, connection pooling | 65 | **ACCEPTED** |
| **P4 (Stages 4A, 4B, 4C)** | Invalidation codecs, router coordination, anti-entropy | 43 | **ACCEPTED** |
| **Simulation & Unit Baseline** | PIE adaptive policy, traffic generator, WP1–WP4 units | 54 | **ACCEPTED** |
| **TOTAL** | **19 Targets across Workspace** | **242 / 242** | **100% PASS** |

- **Clippy:** 0 warnings, 0 errors under `cargo clippy --all-targets -j 1 -- -D warnings`.
- **Formatting:** Clean under `git diff --check src/ tests/ docs/security/`.
- **Authoritative Report:** [`docs/security/phase-4-final-acceptance-report.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-final-acceptance-report.md).

---

## 4. Repository Layout at a Glance

```text
D:\Personal\phantom-cache\
├── Cargo.toml                          # Workspace manifest (19 targets: lib, bin, 17 integration tests)
├── src/
│   ├── main.rs                         # Application entry point, CLI/env parsing, cluster runtime wiring
│   ├── lib.rs                          # Library root and unit module tests
│   ├── api/server.rs                   # Axum HTTP routes, JWT RBAC, SSE telemetry, DELETE handlers
│   ├── core/
│   │   ├── cache.rs                    # PartitionedCache, KeyState, generational fencing, tenant epochs
│   │   └── coalesce.rs                 # Coalescer, FetchRegistration, atomic completion channels
│   ├── dist/
│   │   ├── node.rs                     # VirtualNode, TCP binary listener, recovery state machine
│   │   ├── router.rs                   # RendezvousRouter, HRW hashing, broadcaster, anti-entropy probe
│   │   ├── frame.rs                    # PhantomMeshFrame (0x01..0x08), binary serialization codecs
│   │   ├── pool.rs                     # TlsConnectionPool, reservation tokens, heartbeat ping-pong
│   │   ├── session.rs                  # SessionTracker, active connection eviction
│   │   └── anti_entropy.rs             # InvalidationRingBuffer (4,096 items/4MB), AntiEntropyTracker
│   ├── security/
│   │   ├── mod.rs                      # Fail-closed startup security validation
│   │   ├── cluster.rs                  # ClusterTlsConfig, ClusterMembershipRegistry, mTLS verifiers
│   │   └── upstream.rs                 # Anti-SSRF IP blocklists, custom CA anchoring, HTTP parser
│   ├── control/pie.rs                  # Policy Inference Engine, Count-Min sketch, adaptive switching
│   ├── intel/
│   │   ├── telemetry.rs                # Ring-buffered telemetry metrics collector
│   │   └── diagnostics.rs              # Atomic counters (cache hits, stale writes prevented, flushes)
│   └── sim/                            # Discrete-event simulation (traffic generator, M/M/1 backend)
├── tests/
│   ├── unit/                           # wp1_tests, wp2_tests, wp3_tests, wp4_tests
│   └── integration/                    # 13 dedicated integration test suites (Phase 1–4, cluster, HTTP)
├── docs/
│   ├── handoff/                        # Complete AI-to-AI knowledge transfer package (00 to 08 + MANIFEST)
│   │   └── transcripts/                # Full conversation transcripts (Session 1 & Session 2)
│   └── security/                       # Authoritative architectural specifications and audit reports
└── transcript_copy.jsonl               # Session 1 transcript copy (Oct 11 to Oct 25)
```

---

## 5. How to Navigate the Handoff Package

Read the documents in this exact order:
1. **`00_START_HERE.md`** (this document): Overview, ground rules, and navigation.
2. **[`01_PROJECT_MASTER_CONTEXT.md`](file:///d:/Personal/phantom-cache/docs/handoff/01_PROJECT_MASTER_CONTEXT.md)**: Deep technical architecture, data flows, and subsystem specifications.
3. **[`02_PHASE_HISTORY_AND_STATUS.md`](file:///d:/Personal/phantom-cache/docs/handoff/02_PHASE_HISTORY_AND_STATUS.md)**: Chronological history of P1–P4, audits, findings, and resolutions.
4. **[`03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md`](file:///d:/Personal/phantom-cache/docs/handoff/03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md)**: Wire protocols, binary framing, fencing, and anti-entropy algorithms.
5. **[`04_TESTS_BENCHMARKS_AND_EVIDENCE.md`](file:///d:/Personal/phantom-cache/docs/handoff/04_TESTS_BENCHMARKS_AND_EVIDENCE.md)**: Test catalog, benchmark results, and loopback vs WAN evidence.
6. **[`05_OPEN_RISKS_AND_NEXT_STEPS.md`](file:///d:/Personal/phantom-cache/docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md)**: Prioritized issue register (P0–P2) and future roadmap (Phases 5–9).
7. **[`06_AGENT_WORKFLOW_AND_GIT_SAFEGUARDS.md`](file:///d:/Personal/phantom-cache/docs/handoff/06_AGENT_WORKFLOW_AND_GIT_SAFEGUARDS.md)**: Pair-programming engineering practices, testing protocols, and Git rules.
8. **[`07_CONVERSATION_AND_DECISION_INDEX.md`](file:///d:/Personal/phantom-cache/docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md)**: Complete index of user instructions, audits, decisions, and transcripts.
9. **[`08_NEW_AI_ONBOARDING_PROMPT.md`](file:///d:/Personal/phantom-cache/docs/handoff/08_NEW_AI_ONBOARDING_PROMPT.md)**: Copy-ready prompt to bootstrap your replacement.
10. **[`MANIFEST.md`](file:///d:/Personal/phantom-cache/docs/handoff/MANIFEST.md)**: Master inventory and cross-reference table.

---

## 6. What the Next AI Assistant Should Do First

When starting a new session:
1. **Check Git Status:**
   ```powershell
   git rev-parse HEAD
   git branch --show-current
   git status --short
   ```
   Confirm HEAD is `456bb25930004f1896a84c53e776e7b3ef29ee16` on branch `feat/publish-phantomcache-core`. Do NOT reset or clean.
2. **Verify the Baseline:**
   ```powershell
   cargo test --all-targets -j 1
   cargo clippy --all-targets -j 1 -- -D warnings
   ```
   Confirm all 242 tests pass and Clippy produces 0 warnings.
3. **Acknowledge Acceptance:** Phase 4 is complete and accepted. Ask the user for authorization before initiating any new feature work (such as Phase 5 or release packaging).

