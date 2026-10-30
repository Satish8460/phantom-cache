> Notice (2026-10-30): This handoff file is a historical record of the state on 2026-10-28 and is not edited. Parts of it are superseded: "Phase 4 accepted / zero P0 / formally accepted" and similar statements are not supported by the later audit ([handoff CORRECTIONS](CORRECTIONS.md), `CLAUDE.md` sections 6 and 6b); the test count "242 tests / 19 targets" is a 2026-10-28 baseline, not the current one ([docs/STATUS.md](../STATUS.md)); and descriptions of cluster cache sharing may predate the mTLS cluster path, where the local node shares the serving cache (`src/main.rs:332`).
>
> Links in the text below, including `file:///d:/...` links, are unchanged and work only on the original machine.
>
> Original text below is unchanged.

# PhantomCache — Conversation, Decision, and Transcript Index

**Document ID:** `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md`  
**Purpose:** Comprehensive index of all user requirements, design sessions, audit findings, decisions, and conversation transcript preservation.  
**Audience:** Incoming AI assistants, systems architects, and project stakeholders.  
**Authoritative Baseline Date:** October 28, 2026  

---

## 1. Transcript Source Inventory & Availability Status

To ensure complete knowledge preservation across AI assistant transitions, the conversation history was audited and preserved in `docs/handoff/transcripts/`:

| Source ID | Filename / Location | Date Range | Size | Description & Contents | Availability Status |
| :--- | :--- | :---: | :---: | :--- | :--- |
| **AG-Session-1** | [`docs/handoff/transcripts/ag_conversation_session_1_oct11_to_oct25.jsonl`](file:///d:/Personal/phantom-cache/docs/handoff/transcripts/ag_conversation_session_1_oct11_to_oct25.jsonl) (copy of `transcript_copy.jsonl`) | Oct 11, 2026 – Oct 25, 2026 | 1.28 MB | Steps 0 to 1,344 of initial Antigravity session (`3ee3c52a-c7bd-475f-a0aa-faa69eff6e74`). Covers initial repository analysis, Phase 2 project report creation, M1 HTTP tests, and Phase 3 transport design. | **PRESERVED IN FULL** |
| **AG-Session-2** | [`docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl`](file:///d:/Personal/phantom-cache/docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl) | Oct 25, 2026 – Oct 28, 2026 | 13.34 MB | Complete raw transcript of current Antigravity session (`eaca48fa-9967-405d-b4d3-54047091810e`). Covers Stage 4A, SEC-4A-01 remediation, Stage 4B coordination, Stage 4C anti-entropy, and Gate B production remediations. | **PRESERVED IN FULL** |
| **ChatGPT-Prior** | *External Chat Export* | Prior to Oct 11, 2026 | N/A | The original ChatGPT conversation where the foundational PhantomCache concept, Phase 1 requirements, and initial Go/Rust architecture discussions occurred. | **FILE NOT IN REPOSITORY**<br/>*(Core design decisions, specifications, and academic report structure were incorporated into repository documents `docs/PHASE_2_WORK_REPORT.md` and initial prompts. If raw conversation JSON is required, user must provide the file).* |

---

## 2. Chronological Timeline of Major Events & Milestones

```text
2026-10-11
  ├── Step 0: User initiates project analysis for Course Project Work Report on PhantomCache (BCS703B).
  ├── Foundation analysis of sharded mutexes, visited-sweep (SIEVE) replacement, and M/M/1 queue model.
  └── Delivery of docs/PHASE_2_WORK_REPORT.md and automated test suites (WP1–WP4).

2026-10-15 – 2026-10-22
  ├── Milestone 1 Design & Ingress Security hardening (P1 & P2).
  ├── Zero-trust ingress with JWT validation and role-based access control (Phase 1).
  └── Upstream egress SSRF defense, private IP blocklists, custom CA anchoring (Phase 2).

2026-10-23 – 2026-10-25
  ├── Phase 3 Cluster Transport Hardening (Stages 3A, 3B, 3C).
  ├── Mutual TLS 1.3 with private cluster CA and RFC 1035 SAN identity pinning (3A).
  ├── Bounded length-prefixed binary framing (PhantomMeshFrame) eliminating delimiter injection (3B).
  └── Bounded connection pooling, 5s heartbeats, and Layer 5 active session revocation (3C).

2026-10-26 – 2026-10-27
  ├── Phase 4 Distributed Invalidation & Consistency Kickoff.
  ├── Stage 4A: Codecs, generational fencing, and tenant epochs.
  ├── Audit Finding SEC-4A-01: Tenant epoch capacity exhaustion stale-entry resurrection risk.
  ├── SEC-4A-01 Remediation: Synchronized global-epoch fallback when tenant map reaches 10,000 entries.
  ├── Stage 4B: Router coordination, acting-primary failover (150ms deadline), and replica broadcaster.
  └── Stage 4C: Bounded anti-entropy, ring buffers (4,096 items / 4 MB), and digest reconciliation.

2026-10-28
  ├── Autonomous End-to-End Audit across P1–P4 and Stages 3A–4C.
  ├── Remediation of SEC-4C-01..06 in anti-entropy (monotonic streams, gap detection, cold boot flush).
  ├── Test suite reconciled from 220 to 234 tests across 19 targets.
  ├── Gate A Production Readiness Review (identified SEC-POA-01 through SEC-POA-06).
  ├── Gate B Verified Narrow Remediation:
  │     ├── SEC-POA-01: Clustered runtime listener and peer address wiring in main.rs & cluster.rs.
  │     ├── SEC-POA-02 & SEC-POA-04: Safe recovery state machine & cache read bypass in node.rs & router.rs.
  │     ├── SEC-POA-03: Exact resident heap memory accounting (<= 4.25 MiB) in anti_entropy.rs.
  │     └── SEC-POA-05 & SEC-POA-06: Accurate benchmark labels & DELETE 204 consistency boundaries.
  ├── Full verification: 242 tests passing across 19 targets, Clippy clean.
  └── Delivery of Complete AI Handoff Package (docs/handoff/).
```

---

## 3. Major Architectural Decisions & Dispositions

### Decision 1: Caching Proxy vs Distributed Database Consensus
- **Context:** Early discussions explored Raft or Paxos for cluster state replication.
- **Decision:** Explicitly reject distributed database consensus. PhantomCache is an ephemeral caching tier in front of an authoritative origin. Distributed 2PC write locks introduce severe latency and write-side deadlocks during partitions.
- **Outcome:** Two-tier invalidation architecture: synchronous rendezvous primary purge ($T_{\text{stale}} = 0$) + asynchronous bounded replica broadcast + periodic self-healing anti-entropy.

### Decision 2: Decoupling Stream Sequences from Cache Key Generations
- **Context:** An initial proposal suggested using a single sequence number per node to discard older invalidations.
- **Problem Identified:** If sequence 20 arrives for `key_A`, a subsequent sequence 15 for `key_B` would be falsely dropped as obsolete, causing stale data resurrection for `key_B`.
- **Decision:** Strictly separate stream sequences (`stream_seq: u64` in Router/AntiEntropyTracker for gap detection) from key generations (`generation: u64` in KeyState for in-flight fetch fencing).

### Decision 3: SEC-4A-01 Synchronized Global-Epoch Fallback
- **Context:** Stage 4A audit identified that capping tenant epochs at 10,000 entries with LRU or random eviction could resurrect stale entries if an evicted tenant reused an epoch.
- **Decision:** When `tenant_epochs.len() >= 10,000`, atomically increment `global_epoch` and clear `tenant_epochs`. All pre-existing entries across the entire cluster fail the global epoch match ($old \ne new$) and are cleanly invalidated.

### Decision 4: Safe Recovery & Read Bypass During Anti-Entropy (SEC-POA-02 / SEC-POA-04)
- **Context:** Gate A review found that a reconnecting node could serve stale cache hits before anti-entropy catchup finished.
- **Decision:** Inbound `GetRequest` on nodes in `Suspect` or `Recovering` state bypasses cache hits and returns `MISS` to origin. Nodes are promoted to `Active` **only** upon verified `InSync` or complete safe flush `HistoryOverwritten`.

---

## 4. Key Cross-References to Source Code & Reports

| Topic / Decision | Authoritative Documentation | Source Code Implementation | Integration Tests |
| :--- | :--- | :--- | :--- |
| **Phase 4 Final Acceptance** | [`docs/security/phase-4-final-acceptance-report.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-final-acceptance-report.md) | Entire repository | All 19 targets (242 tests) |
| **Phase 4 Architecture** | [`docs/security/phase-4-design.md`](file:///d:/Personal/phantom-cache/docs/security/phase-4-design.md) | `src/dist/`, `src/core/cache.rs` | Phase 4 integration tests |
| **Cluster Transport & mTLS** | [`docs/security/phase-3-design.md`](file:///d:/Personal/phantom-cache/docs/security/phase-3-design.md) | `src/security/cluster.rs` | `security_phase3_cluster_tests.rs` |
| **Upstream SSRF & Egress** | [`docs/security/phase-2-specification.md`](file:///d:/Personal/phantom-cache/docs/security/phase-2-specification.md) | `src/security/upstream.rs` | `security_phase2_upstream_tests.rs` |
| **Zero-Trust Ingress & RBAC** | [`docs/security/phase-1-specification.md`](file:///d:/Personal/phantom-cache/docs/security/phase-1-specification.md) | `src/api/server.rs` | `security_phase1_tests.rs` |

