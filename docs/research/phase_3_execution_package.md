> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work against this package was executed and reported in [phase_3_completion_report.md](phase_3_completion_report.md) (agent-reported PASS; supported status partially implemented, weakly verified). No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 3 Execution Package — M3: Distributed Correctness & Architecture Gate

**Status: PROPOSED — not authorized.** Requires the owner's written authorization after G-M1; may overlap M2 only on disjoint files (M3 edits `src/dist/*`, `src/main.rs` cluster wiring, node/serving integration points; M2 owns `cache.rs`/`server.rs` — any shared-file edit is sequenced through the ledger). Common rules: `phase_execution_packages_index.md` §4. **This phase may legitimately end by simplifying the distributed design or by gating it as experimental; it must not assume the current architecture is correct.**

## 1. Purpose, scope, non-goals, outcomes
**Purpose.** Establish what the distributed layer actually guarantees, state a *contract with a failure model*, measure it end to end on multiple real serving nodes, decide the architecture (gate **G-ARCH**) on that evidence, and implement or gate accordingly. **Scope:** the relationship between the HTTP serving cache and `VirtualNode` caches; invalidation propagation, sequence/replay/anti-entropy, membership and recovery, bounded queues, reconnect behavior, DELETE semantics, stale-data bounds under faults. **Non-goals:** adaptive policies, performance tuning of the single-node hot path (M2), the lab's workload library (M4), WAN replication, consensus protocols, linearizability claims.
**Outcomes.** (1) an executable multi-process cluster harness with fault injection; (2) a characterization of present behavior with confidence intervals; (3) ADR-003 with the selected option, contract statement, failure model and evidence; (4) the selected option implemented and verified, or the cluster layer gated; (5) all distributed claims in code/docs/UI reduced to what is tested.

## 2. Starting state (evidence-backed; re-verified at M3-00)
**Verified by the research lead before Phase 1 (line numbers may have shifted):** the HTTP cache is `PartitionedCache::new(8)` (`main.rs:65`) while each `VirtualNode::new` creates its own `PartitionedCache::new(4)` (`node.rs:42`); cluster invalidation handlers purge `node.cache` (`node.rs:485/498/504`), and `main.rs` never links the two caches [V]; GET is served from the local HTTP cache only; `forward_key_over_tcp` and HRW routing (`route_key`, `get_replicas`) are called only inside `router.rs` and tests [V grep]; `reconcile_peer_anti_entropy` has no `src/` caller [V]; the outbound invalidation is never recorded by its sender (`ring_buffer.push` only at `node.rs:475`, on receipt) [V]; `stream_seq` is one router-wide counter shared by all targets and retries (`router.rs:142,615`) [V]; `record_inbound_seq` advances `expected` past a gap and `get_highest_acked_seq` returns `expected−1` (`anti_entropy.rs:200-231`) [V]; DELETE purges the local entry after the cluster round trip (`server.rs:830-832`) [V]; replica broadcast uses a 1,024-slot channel with `try_send` and 3 retries (`router.rs:61,194,110`) [V], drops leave no record, and spawned tasks are not bounded by the channel [A]; `NodeState::Suspect` is sticky in production and Recovering is set only inside reconcile [A]; the cluster `GetRequest` handler fabricates `val_node_<id>` [V]; the insecure-cluster mode builds a router over addressless nodes [A]; the only staleness bound in effect is the fixed TTL [V]. **Phase 1 (PROVISIONAL):** `tests/integration/known_gaps.rs` (KG-1…KG-6) with outcomes, `CORRECTIONS.md`, the `[EXPERIMENTAL]` banner.
**Not established:** behavior of the real binary under multi-process mTLS operation; whether the harness can start real gateways from environment variables alone; true message-loss behavior (only loopback exists).

## 3. Candidate architectures (evaluated at G-ARCH; none is presumed)
| Option | Idea | Guarantee it can offer | Cost / risk |
|---|---|---|---|
| **C** | Single-node supported; cluster layer gated (`PHANTOM_EXPERIMENTAL_CLUSTER=true` required), documented as experimental | single-node semantics; cross-gateway staleness ≤ TTL_eff | lowest; no distributed value claimed |
| **A1** | Independent local caches + reliable invalidation fan-out: unify invalidation target with the serving cache; per-destination sequence spaces; bounded per-origin outbound log; gap-correct acks; resync on reconnect/gap; periodic cheap digests | bounded staleness Δ ≤ D_max under a stated loss/partition model; fallback ≤ TTL_eff | medium–high protocol complexity |
| **A2** | Local caches + best-effort fan-out + **conservative flush on uncertainty** (queue overflow, gap, reconnect ⇒ peer drops affected scope/entire cache, rate-limited) | Δ small when healthy; correctness by over-invalidation; cold-start/metastability risk | medium; hit-ratio cost; flush storms |
| **D** | Synchronous fan-out: DELETE returns after all *reachable* peers ack (timeout ⇒ mark peer, deliver debt on reconnect); TTL bounds the rest | read-after-ack guarantee across reachable peers; unreachable peers ≤ TTL_eff | DELETE latency/availability coupling |
| **B** | Coordinated reads: HRW owner serves reads, others forward | single owner per key (strong single-copy for reads), plus invalidation to owner | highest: forwarding protocol, loops, hot-key amplification, failure fallback |
**Value test (mandatory):** the distributed machinery is only worth its cost if it materially tightens staleness versus the always-available TTL bound. M3-03 quantifies Δ (machinery) versus TTL_eff for the scenarios and for realistic freshness lifetimes; if the improvement is not material, the evidence favors C.

## 4. Work packages
| ID | Title (legacy) | Type | Depends on |
|---|---|---|---|
| M3-00 | Entry verification and architecture trace refresh | M | G-M1 |
| M3-01 | Multi-process cluster harness with link-fault injection (WP-10) | M | M3-00 |
| M3-02 | Characterization of present behavior (baseline for the decision) | M | M3-01 |
| M3-03 | Contract, failure model, option evaluation, **G-ARCH decision** (ADR-003) | M (owner decides) | M3-02 |
| M3-04C | Gate the cluster layer (if Option C) | C | M3-03 |
| M3-04A | Reliable invalidation fan-out (WP-11/12/13) (if A1/A2) | C | M3-03 |
| M3-04D | Synchronous fan-out with invalidation debt (if D) | C | M3-03 |
| M3-04B | Coordinated reads (if B) | C | M3-03 |
| M3-05 | Formal/exhaustive protocol model (RT-5) | optional (O3-4) | M3-03 |
| M3-06 | Physical multi-host validation | C (O3-5) | M3-04* |
| M3-07 | Documentation and claim alignment | M | all |
Order: M3-00 → M3-01 → M3-02 → M3-03 (owner decision) → exactly one of M3-04* (or a redesign proposed to the owner) → M3-05/M3-06 → M3-07.

### M3-00 Entry verification
CER-2, plus a **fresh call-chain trace** (read-only) for GET, DELETE, peer invalidation, replica broadcast, reconnect, heartbeat, reconcile on the then-current code, written as `docs/research/architecture_trace_m3.md` (function, file:line, caller, data structure), explicitly marking where it differs from ADR-001 and Phase 1's KG outcomes. If KG-1 was NOT REPRODUCED in Phase 1, STOP and report (the premise changed).

### M3-01 Multi-process cluster harness (WP-10)
**Requirement.** Tests start **N real gateway processes** (3 by default; 2 and 5 for specific scenarios) from the built binary (`CARGO_BIN_EXE_<name>` in integration tests; name verified at M3-00), configured only through the documented environment variables (`PHANTOM_NODE_ID`, `PHANTOM_CLUSTER_*`, `PHANTOM_BIND_ADDR`, members file, mTLS certificates generated per run by the repository's cert scripts or `rcgen` into a temp directory), plus a **versioned mock origin** (each key carries a monotonically increasing version, with an append-only access/commit log with monotonic timestamps) and a **link-fault proxy** between every pair of cluster listeners implementing: drop connections, blackhole (accept and drop bytes), added delay (fixed/jittered), asymmetric and symmetric partitions with scheduled start/end, bandwidth throttle, and optional frame-aware loss of selected message types — all driven by a seeded script so a run is replayable. Reader load per gateway samples tracked keys at a fixed rate.
**Constraints.** No new runtime dependencies; the proxy is test-support code (tokio + std); if the binary cannot be driven by environment alone (e.g., the insecure mode is unusable, ports cannot be set), the minimal fix is a **src change to extract a testable cluster builder or fix the config path** ⇒ owner-reserved (O3-1); no `pub` widening or test-only hooks otherwise (Phase 1 A-13 principle).
**Staleness measurement definitions (fixed before any run).** For key *k*: origin commit of version *v* at t_c; invalidation issued at t_i (DELETE request received by gateway A); DELETE acknowledged (204) at t_a; a read at t_r on gateway G returns version u. **Stale-after-ack read:** t_r > t_a ∧ u < v. **Time-to-consistency (TTC) at G:** min(t_r − t_a) over reads with u ≥ v (per key, per gateway). **Version regression:** a gateway serves u′ < u after having served u. All timestamps from one host's monotonic clock (loopback), recorded by the harness not by the system under test.
**Tests of the harness itself:** a known-stale control (cluster with invalidation disabled shows stale-after-ack reads), a known-good control (invalidation applied to all caches shows none), proxy fidelity tests (measured delay/loss vs configured within tolerance), determinism test (same seed ⇒ same fault schedule), clean shutdown, no orphaned processes (PID accounting in the ledger).
**Acceptance:** controls behave as stated; harness runs ≥ 30 seeded repetitions of a 3-node scenario unattended; BLOCKED if the binary cannot be driven (O3-1).

### M3-02 Characterization of present behavior
Scenarios (each ≥ 30 seeded runs, results classed per CER-5, injected faults on loopback labeled as such): **S3-1** healthy cluster, DELETE on A, readers on B and C; **S3-2** peer-link message loss p ∈ {0, 1%, 5%, 20%}; **S3-3** partition of one node for d ∈ {1 s, 5 s, 30 s, TTL+10 s} then heal; **S3-4** node restart (cold cache, sequence reset); **S3-5** reconnect storm (flapping link); **S3-6** invalidation burst exceeding the replica queue (e.g., 10,000 DELETEs in 1 s with a slow peer): queue depth, memory, drops, and what happens to the dropped keys; **S3-7** retry exhaustion; **S3-8** Suspect-stickiness: how long until a recovered peer is targeted again; **S3-9** DELETE availability/latency under partition (does 204 require reachable primary? 504 rate); **S3-10** gap handling: with real loss, do sequence gaps correspond to actual losses (false-positive rate) and is anything repaired.
**Outputs:** P(stale-after-ack), TTC distribution (p50/p90/p99/max with bootstrap CIs), version-regression counts, DELETE latency/availability, queue depth/memory, repair events (expected zero in the current code), per scenario; plus a table mapping each Phase 1 KG gap to the observed effect size. **Acceptance:** scenarios VALID or reported with class; the report states which defects have measurable user-visible effect and which are latent.

### M3-03 Contract, option evaluation, G-ARCH (ADR-003) — owner decision
**Contract template (must be filled with measured values, not aspirations):** "Failure model F = {message loss ≤ p, partition ≤ P_max, node restarts allowed, clock skew n/a (no clock dependence), max invalidation rate λ}. Guarantee G1 (per-node): after DELETE returns on node X, no read on X returns the old content. Guarantee G2 (cross-node, reachable peers): after DELETE is acknowledged, any read on a connected peer after Δ returns post-invalidation content, with Δ ≤ D_max (measured P99/P99.9 and max). Guarantee G3 (fallback, always): staleness ≤ TTL_eff = freshness lifetime + applicable stale window ≤ cap. Non-guarantees: linearizability, global ordering, behavior under unbounded faults." Evaluation matrix: stale-after-ack probability, TTC, DELETE availability/latency, read hit-ratio impact, CPU/memory/queue bounds, code/state complexity, operational burden, testability, research value. **The owner selects the option**; the research lead supplies a recommendation. A redesign not listed (e.g., a different propagation mechanism) is allowed if proposed with the same evidence.
**Acceptance:** ADR-003 contains the failure model, selected option, rejected options with reasons, measured evidence references, and the list of claims that will and will not be made.

### M3-04C Gate the cluster layer (if C)
Cluster mode requires `PHANTOM_EXPERIMENTAL_CLUSTER=true` (default off ⇒ startup refuses with a clear message — default change is O3-8); UI `capabilities.cluster = "experimental"`; docs and README claims removed/qualified; KG tests either retired with rationale in ADR-003 or kept as ignored regression markers; **no distributed code is deleted** (retirement is an owner decision O3-9). Tests: startup refusal/acceptance matrix, capability reporting.

### M3-04A Reliable invalidation fan-out (if A1/A2)
Requirements (each with tests in the multi-process harness):
1. **Cache unification:** a received invalidation purges the **serving cache** (not a side cache); mechanism proposed in ADR-003 (shared `Arc<PartitionedCache>` between gateway and node vs invalidation callback) — public API/type changes are owner-reserved (O3-1).
2. **Sequence semantics:** per-(origin→destination) sequence spaces assigned at send time; retries reuse the same sequence number; receivers track the highest **contiguous** sequence per origin and report it as the cumulative ack; a gap is *recorded and acted on* (replay or resync), never silently skipped. Tests: injected loss produces a detected gap, no false gaps without loss (rate 0), retries do not create gaps.
3. **Outbound history:** per-origin bounded log (entries and bytes) of sent invalidations; replay from the peer's acked sequence; when history no longer covers the gap, the peer is told **resync required**.
4. **Resync fallback:** A1: scoped fallback (e.g., global epoch bump limited to the origin's tenants if the log records tenants); A2: rate-limited conservative flush; the choice and its cold-start/metastability implications are an owner decision (O3-7).
5. **Reconciliation triggers (production call paths):** on (re)connect, on detected gap, on resync request, and periodic cheap digest (highest contiguous sequence per origin, not whole-ring hashes). Every trigger has an E2E test.
6. **Bounded fan-out:** per-peer bounded queue with per-key coalescing (latest scope wins); overflow sets `resync_required` for that peer and increments a metric; spawned-task count is bounded (semaphore) — test with 10× overload shows bounded memory and queue depth.
7. **Membership/recovery:** explicit state machine (Active → Suspect on missed heartbeats/IO errors → Recovering on reconnect → Active after reconciliation), timers configured and tested; Recovering nodes **do not serve possibly stale entries** (bypass or revalidate — PROVISIONAL design in ADR-003); heartbeats actually drive transitions (today they do not).
8. **DELETE ordering and response:** local purge first; cluster fan-out follows; 204 semantics defined (local purge + primary ack? all-reachable ack?) — O3-6; response headers reflect degraded fan-out.
9. **Observability:** counters/gauges exported through schema v2 with provenance (queue depth, drops, resyncs, replays, gaps, peer states, last-acked sequence per peer); UI shows only these.
10. **Wire-protocol changes** (new frame fields/versions) are breaking changes ⇒ O3-3; codec tests and fuzz-style property tests for any new frame.
**Acceptance:** re-run S3-1…S3-10 on the new code: stale-after-ack and TTC meet the contract's measured D_max (the numbers are *outputs of M3-02/ADR-003*, not targets invented here); zero false gaps at p=0; repair demonstrated for every loss scenario; bounded memory under overload; no cold-start storm beyond the stated resync policy.

### M3-04D Synchronous fan-out with invalidation debt (if D)
DELETE waits for all reachable peers (timeout T_ack, PROVISIONAL set from M3-02 latency data), marks unreachable peers, records **invalidation debt** (bounded log) delivered on reconnect before the peer serves; tests: all-ack path, one-peer-down path, debt replay, DELETE availability under partition; contract G2 applies to reachable peers.

### M3-04B Coordinated reads (if B)
HRW owner serves GETs; non-owners forward with headers/tenant/auth preserved, hop limit and loop prevention, deadline propagation, owner-failure fallback to local fetch (with stale-serving rules from M2), hot-key amplification control (owner coalescing + optional bounded near-cache with strict invalidation), membership-change behavior (rebalancing without stampede). Requires its own detailed sub-specification approved by the owner before work (O3-2).

### M3-05 Protocol model (optional, O3-4)
Exhaustive exploration of: fill-vs-invalidate race, sequence/ack/replay/resync under message loss/duplication/reordering (across connections), restart. Either TLA+/TLC (requires downloading tooling and a Java runtime — external software ⇒ owner-reserved) or an exhaustive state enumerator written in Rust test code (preferred default). **Acceptance:** invariants (no stale fill stored after acknowledged invalidation; eventual convergence under fair delivery; bounded state) checked; a seeded mutation (skip a sequence check) is detected.

### M3-06 Physical multi-host validation (conditional, O3-5)
Only if network-level claims are planned. ≥ 3 hosts on a LAN with synchronized clocks (document method and residual error) or, better, measurement by relative timestamps from a single observer; repeat the contract-relevant scenarios with real packet loss/latency (e.g., `tc netem`/equivalent — system-level change ⇒ owner-reserved). Without it, all claims are labeled "loopback with injected faults".

### M3-07 Documentation
ADR-003; update ADR-001 status; `CORRECTIONS.md` rows revised; README/handoff claims aligned; `CLAUDE.md` known-gaps updated; claims ledger C6 rewritten with the measured bound and failure model; UI capability labels; configuration reference.

## 5. Acceptance criteria and exit gate G-M3 (= G-ARCH + G-DIST)
| ID | Criterion | Blocking |
|---|---|---|
| AC3-1 | E0 + fresh architecture trace complete; Phase 1 KG outcomes recorded | yes |
| AC3-2 | Harness controls (known-stale, known-good) PASS; proxy fidelity and determinism PASS | yes |
| AC3-3 | S3-1…S3-10 characterization delivered (VALID or class-labeled) | yes |
| AC3-4 | ADR-003 with failure model, contract, evidence, selected option decided **by the owner** | yes |
| AC3-5 | Selected option implemented/gated and verified by multi-process E2E tests (not unit tests alone) | yes |
| AC3-6 | Measured contract values reported with CIs; every claim in docs/UI maps to a tested clause; claims outside the failure model removed | yes |
| AC3-7 | Resource bounds under overload verified (queue depth, memory, tasks) | yes |
| AC3-8 | No orphan processes/files; ledger clean; no unauthorized wire change | yes |
| AC3-9 | Physical multi-host evidence PASS, **or** all network claims explicitly labeled loopback/injected | yes (one of the two) |
**G-M3 passes** when all blocking criteria are PASS (or have written waivers) and the owner accepts. **Disabled until G-M3:** any distributed claim in README/UI/paper, M4/M6 scenarios that require multiple gateways with consistency semantics, productization of cluster features.

## 6. Test and measurement methodology
Everything is an end-to-end multi-process measurement with the definitions in M3-01. N ≥ 30 seeded repetitions per scenario/parameter (same seed set across before/after); event-rate quantities (stale reads) use exact binomial/Wilson intervals, durations use percentile bootstrap; scenario parameters, fault schedules, binary hash, config hash and seeds in a manifest; the harness's own overhead and reader rate are recorded; runs where the harness/proxy misbehaved are INVALID and rerun. Results are never reported as network behavior (loopback). Present-code and new-code runs use identical scenarios for paired comparison.

## 7. Risk register
| Risk | Containment |
|---|---|
| Harness cannot drive the real binary | O3-1 minimal builder extraction; otherwise BLOCKED, not simulated |
| Fault proxy distorts timing | fidelity tests; record proxy latency; INVALID rule |
| Protocol rewrite introduces new bugs | model tests (M3-05), mutation checks, staged merges, snapshot |
| Resync flush causes cold-start storms | O3-7 policy, rate limits, measured with M2 stale-serving, M4 scenarios later |
| Scope creep into option B | separate owner-approved sub-spec |
| Over-claiming from loopback | label rule AC3-9 |
| Decision paralysis | the TTL_eff value test gives a default (C) if machinery is not materially better |

## 8. Authorized files on authorization / prohibitions
May modify with ledger: `src/dist/*`, cluster wiring in `src/main.rs`, the invalidation call sites in `src/api/server.rs` and minimal integration points in `src/core/cache.rs` (sequenced after M2's edits), new test-support under `tests/support/`, `tests/integration/*` (new files), `docs/**`, config reference. **Must not modify:** M2-owned freshness/eviction logic, `src/control/pie.rs`, security-critical TLS verification logic except as a specified, reviewed fix (`src/security/*` edits are owner-reserved O3-3), runtime dependencies. CER-3 prohibitions apply; processes started by tests are tracked and terminated by the harness, not by `kill`-style cleanup of unknown PIDs.

## 9. Owner-reserved decisions
O3-1 src refactor/API change to make the cluster drivable or to unify caches · O3-2 **G-ARCH selection** and any Option-B sub-spec · O3-3 wire-protocol/frame changes, `src/security/*` edits · O3-4 formal-methods tooling · O3-5 multi-host hardware and system-level network emulation · O3-6 DELETE acknowledgement semantics · O3-7 resync fallback (flush scope/rate) · O3-8 default for cluster gating · O3-9 retirement/removal of any distributed code. **Agents may decide:** harness structure, scenario parameter ordering within the declared sets, test names, fault-script formats, internal data structures of new code.

## 10. Stop conditions
KG-1 not reproduced in Phase 1; harness cannot start real processes; ground-truth log inconsistent (INVALID measurement basis); a protocol change would break existing SEC-3/SEC-4 tests; discovery of a security flaw (spoofable originator id, unauthenticated flush) — report immediately and do not ship the affected path; bounded-memory test fails; need for a new runtime dependency.

## 11. Completion report
CER-8 plus ADR-003, architecture trace, scenario tables (before/after), contract statement with measured bounds and failure model, the list of retired/kept KG tests, claim alignment diff, owner decisions taken, deferred work (e.g., WAN, option B if not chosen), next phase proposal only.
