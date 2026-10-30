# PhantomCache — Open Risks, Technical Debt, and Future Roadmap

> **Superseded roadmap (note added 2026-10-28):** the "Proposed Phase 5–9" roadmap below is historical. The current roadmap — seven milestones toward a defined MVP, including a dedicated Phantom Intelligence & PIE milestone — is in `docs/research/mvp_and_milestones.md`; the PIE audit is in `docs/research/pie_audit.md`. The statement "zero open P0 blockers" in §1 is not supported by the code review in `docs/research/adr/ADR-001-present-architecture-and-direction.md`.

**Document ID:** `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md`  
**Purpose:** Prioritized risk register, operational pre-deployment checklist, and proposed future roadmap.  
**Audience:** Systems architects, engineering management, and AI coding assistants.  
**Authoritative Baseline Date:** October 28, 2026  

---

## 1. Prioritized Risk Register

### Priority P0: Critical Correctness, Security, and Deployment Blockers
*All previously identified P0 blockers (SEC-POA-01 through SEC-POA-06) were remediated and verified in Gate B. Currently **ZERO** open P0 code blockers remain in Phase 4.*

---

### Priority P1: Reliability, Performance, and Resource Risks

#### RISK-P1-01: Ephemeral Cold Start Stamping on Multi-Node Concurrent Reboots
- **Evidence:** `src/dist/node.rs`: Nodes restart with empty in-memory caches and sequence tracker set to 0.
- **Impact:** If all cluster nodes reboot concurrently, the entire cluster starts with cold caches. High incoming traffic could cause thundering herds to the origin before caches warm up.
- **Proposed Action:** Implement warm-up cache loading from origin or distributed peer pre-fetching prior to advertising `NodeState::Active`.
- **Dependencies:** Post-Phase 4 operational tooling.
- **Acceptance Criteria:** Rebuilding node warms top 1,000 keys before accepting live traffic.

#### RISK-P1-02: Transient Memory Peak During Replay Batch Serialization
- **Evidence:** `src/dist/frame.rs`: Building a 1 MiB `AntiEntropySyncResponse` payload allocates a temporary `Vec<u8>`.
- **Impact:** While resident heap memory is bounded to $\le 4.25\text{ MiB}$, concurrent replay requests across multiple peers could briefly raise memory by $1\text{ MiB} \times N$.
- **Proposed Action:** Introduce a streaming frame encoder or bound concurrent active replay serializers using a semaphore (e.g. max 2 concurrent serializations).
- **Dependencies:** None.
- **Acceptance Criteria:** Peak heap under 8 concurrent anti-entropy requests remains bounded to $\le 6.5\text{ MiB}$.

---

### Priority P2: Operational, Observability, and Test Gaps

#### RISK-P2-01: Physical Multi-Node Switch Testing
- **Evidence:** All tests run on loopback `127.0.0.1`.
- **Impact:** Loopback does not exhibit MTU packet fragmentation, real switch packet drops, or OS socket buffer starvation under multi-gigabit throughput.
- **Proposed Action:** Set up multi-host testbed (e.g. 3 Linux VMs or bare-metal instances) running automated chaos testing.
- **Dependencies:** External multi-node infrastructure.
- **Acceptance Criteria:** Automated packet loss injection (5% drop rate) demonstrates stable anti-entropy catchup without disconnect loops.

#### RISK-P2-02: Configuration Hot-Reload for Peer Addresses
- **Evidence:** `src/security/cluster.rs:load_membership_from_file`: Authoritative membership JSON is parsed at node boot.
- **Impact:** Adding new peer nodes requires process restart or cluster rollover.
- **Proposed Action:** Add file watcher (`notify` crate) or SIGHUP signal handler to reload membership registry dynamically.
- **Dependencies:** None.
- **Acceptance Criteria:** Registry epoch increments dynamically without tearing down unaffected connections.

---

## 2. Conditions Required Before Production Deployment

Before promoting PhantomCache to live production traffic, the following operational gates must be satisfied:
1. **Physical Environment Validation:** Execute automated smoke tests across distinct physical/virtual host machines over real network hardware.
2. **Authoritative Key Material Provisioning:** Replace test certificates with production PKI certificates anchored to an enterprise HSM or Vault-backed CA.
3. **Capacity & Resource Sizing:** Configure host memory limits (`cgroups` / Docker) with at least 512 MB RAM per node to accommodate cache shards, connection buffers, and OS socket overhead.
4. **Monitoring & Alerting Setup:** Ingest SSE diagnostics (`/api/v1/diagnostics`) into Prometheus/Datadog to monitor `anti_entropy_full_flushes`, `stale_writes_prevented`, and `primary_invalidation_timeouts`.

---

## 3. Proposed Engineering Roadmap (Post-Phase 4)

> [!NOTE]
> The following roadmap items are engineering proposals, not completed commitments. Implementation requires user approval.

### Phase 4 Closure (Current)
- Complete AI handoff and baseline repository preservation.
- Authorize and execute Git commits for Phase 4 milestones upon user instruction.

### Proposed Phase 5: High-Throughput Benchmarking & Soak Testing
- Deploy multi-node cluster harness.
- Execute 24-hour continuous invalidation storm under 100k req/s traffic.
- Measure actual physical NIC latency distributions ($P_{50}, P_{95}, P_{99}, P_{99.9}$).

### Proposed Phase 6: Adaptive / Predictive Caching Research
- Extend Policy Inference Engine (PIE) with predictive pre-fetching based on Markovian path transitions.
- Evaluate reinforcement learning heuristics for dynamic TTL modulation.

### Proposed Phase 7: Operational Resilience & Chaos Engineering
- Automated chaos injection harness (partition injection, node SIGKILL, socket stalling).
- Dynamic membership hot-reloading without process restart.

### Proposed Phase 8: Developer SDK & Client Integration
- Official client libraries (Go, Python, TypeScript) supporting rendezvous hashing on client-side for zero-hop routing.

### Proposed Phase 9: Multi-Region WAN Asynchronous Replication
- Batched asynchronous invalidation shipping across geo-distributed clusters.
- Conflict-free replicated epoch vectors (CRDTs) for cross-datacenter federation.

