# PhantomCache: three-pillar product direction (integrated proposal)

**Date:** 2026-10-30. **Role:** integration writer. **Status: PROPOSAL for the product owner. Nothing in this file is approved, implemented, or run.** It changes no source, UI, Cargo, CLAUDE.md, README, STATUS.md, acceptance criterion, milestone definition or frozen record. It is not an owner approval of anything, and an agent-written decision (including ADR "ACCEPTED" lines) is not treated as an owner approval anywhere below.
**Repo state read:** HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 93 changed or untracked paths (unchanged by this task). Line numbers are working-tree numbers of 2026-10-30 and will drift.
**Inputs (final, not edited):** `three_pillar/01_distributed_systems.md` (**P1**), `02_intelligence.md` (**P2**), `03_security_tenancy_resources.md` (**P3**), `04_lab_ux_design.md` (**P4**); `docs/research/rt0_prior_art_sweep_2026-10-30.md`; `product_vision_vs_implementation_audit.md` and `audit_inputs/*`; `docs/STATUS.md`; `CLAUDE.md` (read only); `mvp_and_milestones.md`; `phase_3` and `phase_5` execution packages (acceptance tables). I did not run cargo, tests, a browser, or any git mutation. I opened the files and lines listed in Appendix B myself.

## Contents
0. How to read this file
1. Executive product definition (proposed wording)
2. The three governing pillars and how they interlock (flash-crowd example)
3. Original vision and audit findings
4. Current verified implementation baseline (4.1 single-node, 4.2 distributed, 4.3 intelligence, 4.4 product, security, observability)
5. Capability matrices
6. Proposed target architecture (six logical layers)
7. Distributed consistency and failure contracts
8. Intelligence architecture and evaluation principles
9. Security, multi-tenancy and resource-management architecture
10. Phantom Lab and Demo Center product direction
11. Design-system and UI/UX adoption decisions
12. Dependency-aware implementation roadmap (merged DAG)
13. Flagship demonstration specification
14. Milestone and acceptance-criteria mapping (PROPOSAL)
15. Risks, trade-offs and unresolved technical questions
16. Owner decisions and authorization boundaries
- Appendix A: Reconciliation notes
- Appendix B: Spot-check table (claims I verified myself)

---

## 0. How to read this file

**Tags.** **FACT** = I or a draft opened the cited file and line and it says this (facts about behaviour I did not run are facts about code text). **INFERENCE** = reasoning from facts; may be wrong. **RECOMMENDATION** = proposal only. **UNRESOLVED** = cannot be settled from the evidence. A claim marked **[checked]** was re-opened by me (Appendix B). A claim marked **[P1]**, **[P2]**, **[P3]**, **[P4]**, **[audit]**, **[STATUS]** is carried from that source and not re-verified by me.

**Status classes** (exactly these): IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, UNVERIFIED, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, BLOCKED, RETRACTED. P1 also used NOT IMPLEMENTED (no code and no design); I keep it only where no design exists at all. A row can carry several classes. **TESTED always means: loopback, debug build, Windows 11, one host, tests mostly written by the same project, unless a row says otherwise.** No distributed, intelligence or security capability reaches INDEPENDENTLY VERIFIED in the strong sense today ([STATUS] reading-only reviews count as Partial).

**Evidence rules carried from CLAUDE.md section 4.** Performance claims need a release build, hardware, OS, workload, seed, baseline, repetitions, intervals and limits. Loopback, simulated, WSL2 and physical-host results stay separate. No guarantee without failure model, bound and a falsifying test. No novelty claim without a primary source. Every displayed UI value is labelled; synthetic data is never shown as live.

**Governing principle of this file.** The product vision drives the architecture. Implementation maturity limits only what may be *claimed*, never what is *designed*. Current limitations, milestone reporting or test convenience do not redefine the product, and no original acceptance criterion is reworded here.

**Numbers I do not give.** No percentages of completion, no confidence figures, no schedule dates. Where I repeat an effort statement it names its draft. (The drafts themselves give almost none; P1 states it has no basis for effort estimates.)

---

## 1. Executive product definition (proposed wording)

**RECOMMENDATION (for the owner to accept, amend or reject; not in force):**

> PhantomCache is an origin-protecting HTTP caching gateway that runs as one node or as a cluster of gateways. It rests on three equal pillars.
> **1. Distributed systems engineering.** A deterministic, fenced, byte-bounded cache engine, and a cluster whose consistency and failure behaviour is written down as numbered clauses, each with a failure model, a bound and a test that could falsify it. Reliable distributed invalidation comes first; coordinated distributed caching (peer-fill, stable placement, cluster capacity and origin protection) and intelligent distributed caching (cluster-aware observation, prediction, coordinated pre-warming) are gated research-and-implementation goals with explicit evidence gates.
> **2. Workload intelligence.** A layer that observes, explains and, only where it measurably beats strong static baselines, adapts policy. The deterministic engine owns correctness; a separate safety and policy layer validates every intelligence decision against hard caps; a negative result stays on the record and does not stop the search for a real advantage.
> **3. Product excellence, observability and demonstrability.** A Phantom Lab and Demo Center in which every value is labelled REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED or UNAVAILABLE, security is a visible product capability, and the design system is approved by the owner after an independent human design review.
> Every claim states its failure model, its evidence tier, its topology (one process, one host, or several hosts) and its limits.

**Why this wording differs from earlier text.** The audit's proposed product definition ([audit] section 8.1) was a two-stage definition (Stage 1 reliable invalidation cluster; Stage 2 coordinated reads and distributed intelligence "gated by evidence"). The owner's mandate makes distributed caching and intelligence equal pillars, not afterthoughts. The wording above keeps the gating discipline but removes any hint that C and D are optional. The current README opening ("An in-memory caching gateway ... (research prototype) ... Distributed operation is experimental.", `README.md:9` [checked]) is accurate about maturity but says nothing about direction; section 14.4 proposes replacement text. **UNRESOLVED (owner):** whether the original differentiator was Stage 1 only or the wider vision ([audit] section 9 item 1); this file proceeds on the owner's current mandate that all three pillars are equal.

**What the product is not (carried constraints).** It is not a distributed database ([audit] cites `docs/handoff/01_PROJECT_MASTER_CONTEXT.md:27`, agent-authored), not linearizable across nodes, not WAN-capable (deferred, not dismissed: P1 5.6), and not security-certified (P3 section 8). Adaptive-control merit is **not established** (STATUS section 4; Scenario B notice).

---

## 2. The three governing pillars and how they interlock

### 2.1 The pillars

| Pillar | Owner-stated scope | The part that exists today | The part that is a gated goal |
|---|---|---|---|
| **1 Distributed systems engineering** | Levels A single-node foundation; B multi-node foundation (real processes, identity, authenticated comms, distributed invalidation, reliable propagation and recovery, explicit contracts, cluster observability); C coordinated distributed caching; D intelligent distributed caching | A plus part of B: fenced single-node engine; authenticated invalidation bus with repair and anti-entropy; static membership; loopback evidence only | Rest of B (durability, heartbeat-driven state, runtime membership, cluster view, multi-host evidence); all of C; all of D |
| **2 Workload intelligence** | Measurable advantage over strong static baselines; deterministic engine owns correctness; safety/policy layer validates every decision; AIMD/PIE negative result preserved, not read as proof against intelligence | Mode parsing, origin-concurrency gate, two HTTP routes; a library of signals, shadow evaluator and decision engine not wired into serving | Truthful observation, audit producer, safety validator, warm actuator, declared-demand calendar, forecasting; each behind an experiment and a kill criterion |
| **3 Product excellence, observability, demonstrability** | Phantom Lab, Demo Center, six tier labels, security as product capability, design-system decision O8-1/O8-2, independent human design review | Lab API `lab-1`, `#/lab` screens, Scenario B evidence, tokens and about eleven components, JWT and tenant isolation, mTLS cluster identity | Session stream `demo-1`, event taps, topology and decision-trace views, pairing, tenant quotas, evidence signing, design review |

### 2.2 Interlock rules (the pillars constrain each other)

1. **Pillar 2 cannot outrun pillar 1.** Cross-node warming moves content to peers; with a lost invalidation (reproduced, section 4.2) a warmed copy stays stale for the object's whole origin-set lifetime. Cross-node warming therefore waits for durable invalidation and the synchronization lease (P1 S1.1, S1.2; P2 section 5). Local, origin-sourced warming through the normal fill path is not blocked, because fencing applies to it like any fill.
2. **Pillar 3 cannot invent what pillars 1 and 2 do not emit.** The Demo Center needs per-event data (membership, invalidation legs, decisions, predictions) that no component emits today (P4 1.5). Until it exists, the UI shows UNAVAILABLE with the owning pillar named, never a decorative substitute.
3. **Pillar 1 and 2 cannot weaken pillar 3's honesty rules.** A logical flag (`NODE_FAILURE`, `server.rs:1086-1091` [checked]) is SIMULATED and never drawn as a failed node; a three-process single-host run is never drawn as three machines.
4. **Security (P3) is a precondition of the other two.** Peer-fill needs per-tenant authorization on the wire; pre-warming is an amplification path unless writes are role-gated and tenant-scoped; the demo stream needs scoped read tokens.
5. **The deterministic engine and its hard caps are authoritative.** Intelligence proposes; the safety and policy layer and the engine dispose (CLAUDE.md 6b; P2 2.1).

### 2.3 The integrating example: the flash crowd (step by step)

Owner scenario: many clients; nodes observe, detect and estimate; nodes coordinate pre-warming; the deterministic engine enforces limits; observability records decisions and outcomes; the Lab shows prediction versus execution versus result and compares reproducibly with a static baseline. Merged from P2 3.1, P4 stage 4 and P1 level definitions. "Today" is the status of the component as of 2026-10-30.

| Step | What happens | Component (layer) | Pillar | Status today |
|---|---|---|---|---|
| 1 | Many clients, one tenant surges on N previously cold keys | Lab load generator `src/lab_live/loadgen.rs` (`Workload` has no rate-step, key-group or tenant field, lines 20-30 [checked]); T1 simulator family W-FC `t1_sim.rs:424` [P2] (E) | 3 | T1 half IMPLEMENTED, TESTED; T2 generator lacks rate schedule: DESIGNED BUT NOT IMPLEMENTED |
| 2 | Each node observes key-level traffic through a bounded, drop-newest event tap | Event tap (E); handler already computes `key_hash` (`server.rs:1261` [checked]) | 2, 3 | DESIGNED BUT NOT IMPLEMENTED (no tap). `hot_keys` returns `unavailable/not_implemented` (`server.rs:766` [checked]) |
| 3 | The crowd is detected and classified (cached keys vs cold keys vs scan) | `BurstDetector`, `HotKeyEmergenceDetector` in `src/intelligence/signals.rs` (C) | 2 | PROTOTYPE ONLY: library, tested at unit level, not wired, constants not scale-free [P2] |
| 4 | Demand and the keys involved are estimated: operator-declared overlay plus observed counts | Forecaster (C); Demand Calendar schema (V2 section 2.2) | 2 | DESIGNED BUT NOT IMPLEMENTED; gated by E9/E10 |
| 5 | Decision: warm, hold, or NoAction, with predicted effect and interval | Decision engine (C) | 2 | `PieDecisionEngine` exists in `src/intelligence/engine.rs`, no production caller; `pie: None` (`main.rs:433` [checked]) |
| 6 | The safety and policy layer validates against budgets and refusal reasons | Safety/policy validator L6 (D) | 2, 3 | DESIGNED BUT NOT IMPLEMENTED. Only live limiters: origin gate, byte budget, stale cap |
| 7 | Nodes coordinate: each warms keys it will serve, within a share of a cluster origin budget (strong coordination needs peer-fill) | Distributed coordination (B) | 1, 2 | Share rule `cluster cap / N_configured` DESIGNED (P1 7.2 R1). Cross-node warming BLOCKED on durable invalidation + sync lease; peer-fill is a gated Level C goal |
| 8 | Warm fetches run through the normal fill path under the origin gate; a warm loses to an organic miss | Fill path and `acquire_origin_permit` (A, D) | 1, 2 | Gate exists, first-come, no priority classes; no internal-request context for warms [P2] (not checked) |
| 9 | The decision, the prediction and the realized outcome are recorded | Audit store (E) | 2 | `AuditStore` exists, no producer; record has no realized-effect field [P2] |
| 10 | The Lab shows prediction versus execution versus result, labelled, and compares with a static baseline (same seed, paired) | Lab runner, compare view (E, F) | 3 | Generic runner and sealed-protocol machinery exist; no warm arm, no compare endpoint, no prediction event |

**INFERENCE (P2 3.1):** only step 1 (T1 half) and the generic runner of step 10 exist. The scenario is a build-out of steps 2 to 9, and E9 (oracle-warm headroom) should run before any warm actuator is built, to learn whether the build is worth it. A truthful interim that runs on current code is in section 13.

---

## 3. Original vision and audit findings (condensed)

Source: [audit] (A1 chronology, A2 capability matrix, A3 design audit; the lead re-checked a handful of items). Evidence gaps stay gaps.

**FACT.** The first commit's README called PhantomCache "an intelligent in-memory distributed cache" and labelled the cluster "Modeled / Simulated" (three-node in-process model, about 117 lines of `src/dist`). Original acceptance criteria were structural (implemented, tests passed); no latency, staleness or availability target existed. **GAP (largest):** the original rationale, node-coordination expectations and criteria probably lived in a ChatGPT conversation recorded as missing (`docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md:24` [audit]); the file is not in the repository.

**FACT (recorded owner decisions).** Single-node-first sequencing was an explicit owner decision (CC:683: "My preference is to establish a trustworthy single-node foundation first ... Do not delete or discard existing distributed code"). The owner called earlier distributed work "historical examples ... not mandatory future features" (CC:142), and asked "a local cache with distributed invalidation or a genuinely distributed cache with coordinated reads?" (CC:427) with no recorded answer. M3 was re-affirmed in scope (CC:1744, CC:1879). The first use of "multi-node USP" in the record is the owner's request of 2026-10-30 (CC:5738).

**FACT/INFERENCE (drift).** The drift is in definitions, positioning and reporting layered on that decision, written by agents with no recorded owner approval: MVP definition where "single-node supported; cluster experimental" satisfies M-3 (`mvp_and_milestones.md:14`, status PROPOSED); G-M3 made conditional (`phase_execution_packages_index.md:23`); AC3-4 reworded (package: "selected option decided **by the owner**", `phase_3_execution_package.md:99` [checked]; the completion report says PASS on "ADR-003 ... Yes"); ADR-003 marked ACCEPTED by two agents; D_max "<= 50 ms" (report) vs "<= 250 ms" (ADR-003); P2 (distributed fault simulation) relabelled; README opening changed. **GAP, not conclusion:** no owner approval is recorded for the MVP wording, the README line or ADR-001/002/003; any may have been given outside the repository (STATUS: "evidence gap, not proof").

**Findings that cut the other way.** Cluster work was extensive and owner-driven; ADR-003 rejected single-node as primary; cluster mode is not gated off; "experimental" is consistent with the owner's anti-overclaim rules.

**Direct answer ([audit] section 2).** Mainly (a) unintentional dilution in agent-written definitions and reporting, and (b) a multi-node vision never written down as requirements, plus one intentional element (the order of work). Not "silently abandoned".

**Capability counts from the A2 matrix:** of 30 rows, Implemented 3, Tested 9, Independently verified 0, Partial 6, Prototype only 3, Not implemented 8, Superseded 1. This file's sections 4.2 and 5 refine that with P1's 28-row table; the two do not disagree on substance.

**Architectural assessment ([audit] section 6, A2 judgement, not measurements).** The narrow vision (reliable multi-gateway invalidation) can evolve on the existing foundations; the wider vision (shared capacity, coordinated reads, cluster budgets, distributed intelligence) needs added layers on the same transport and identity, not a rewrite; WAN needs redesign. The audit gave percentages (80/70/75); **I do not repeat them** (P1 also declines them: they are judgement, not measurement). Fundamental to the current Option A1 design: independent per-node caches (capacity does not pool), local GET (origin load multiplies with N), invalidation-only coherence (staleness bound rests on origin-set lifetime). Reaching the wider vision means adding a coordinated-read layer (peer-fill or owner-forward), not only closing gaps.

---

## 4. Current verified implementation baseline

Columns: capability | class | where | tests (names from documents; not run by me) | environment | independence | limits. Unless a row says otherwise: debug build, Windows 11 x86-64, loopback, one host, last recorded full run 851 passed, 0 failed, 8 ignored, 75 targets (STATUS section 3; the counts in CLAUDE.md and README, 242, are historical). Passing tests do not establish correctness. "Indep." = a checker other than the author.

### 4.1 Single-node (pillar 1 level A)

| Capability | Class | Where | Tests / evidence | Indep. | Limits |
|---|---|---|---|---|---|
| Generation fencing: store only if `generation == op_gen` and tenant/global epochs match; DELETE bumps generation and detaches the in-flight entry | IMPLEMENTED, TESTED | `src/core/cache.rs:1457-1469`, `:1609-1638` [checked] | `invalidation_contract_tests`, `serving_contract_tests` CT-7/INV-2 [P1] | Code-reading reviews only | One process. No cross-node version |
| Tenant epochs; overflow at 10,000 tenants falls back to a **global** epoch bump | IMPLEMENTED, TESTED | `cache.rs:581-593` [checked] | SEC-4A-01 tests [P1] | reading only | Correct but means any caller that can reach 10,001 tenant ids flushes everyone (section 9) |
| Single-flight coalescing with waiter cap | IMPLEMENTED, TESTED | `src/core/coalesce.rs`, `cache.rs` | `coalesce_tradeoffs_tests` [P3] | partial | Per key only: N distinct cold keys cost N origin fetches |
| HTTP freshness: lifetime from `s-maxage`, `max-age`, `Expires`, else default | IMPLEMENTED, TESTED | `src/core/freshness.rs:263-276` [checked] | RFC 9111 subset tests [P3] | partial | **No gateway ceiling on lifetime**; default is a literal `60_000` because `PHANTOM_DEFAULT_TTL_MS` is NotWired (`config.rs:115` [checked]) |
| Stale-serving hard cap `PHANTOM_MAX_STALE_MS` (default 24 h) | IMPLEMENTED, TESTED | `cache.rs:290,336,389`; `server.rs:1318` [checked] | `stale_cap_violations` must be 0 (CLAUDE.md) | partial | Caps stale *serving* after expiry only, not freshness lifetime (Appendix A, R5) |
| Eviction: SIEVE default; LRU and S3-FIFO selectable; faithful implementations behind `ShardEvictionPolicy` | IMPLEMENTED, TESTED | `cache.rs:209,214,223,332`; `eviction.rs:690-699` [checked] | eviction tests [P1/P3] | partial | CLAUDE.md section 6 ("evictors ... are scans") is outdated; complexity claims not re-verified by me (Appendix A, R6) |
| Byte accounting I1-I8, `audit_bytes()`, never evict active/in-flight | IMPLEMENTED, TESTED | `cache.rs` (13 mutation sites for `shard.bytes`, P3 1.3) | `byte_accounting_tests` | reading only | Global per process; no per-tenant accounting |
| Origin concurrency cap (static or AIMD) | IMPLEMENTED, TESTED; static cap INDEPENDENTLY VERIFIED by reading and recomputation (`g1_verdict.md`) [P2] | `intelligence/executor.rs`; `server.rs:1276,1365,1647` [P2] | `origin_cap_tests`, `intel_executor_tests` | partial | Enforced only in Canary/Active (`executor.rs:217-220` [checked]); gateway-global; no priority classes; overload is 503 for everyone |
| Truthful telemetry primitives | IMPLEMENTED | `src/intel/telemetry.rs`, `origin_metrics.rs`, `diagnostics.rs` | unit/integration | partial | Histogram has no serving callers per audit C5 [P2]; windowed ratios UNAVAILABLE |
| Configuration reference with wiring status | IMPLEMENTED, TESTED | `src/config.rs` | `m7_config_tests` | partial | One known NotWired key (`PHANTOM_DEFAULT_TTL_MS`) |
| Soak, release-profile suite, Linux, any multi-host run | UNVERIFIED (never run) | n/a | n/a | n/a | STATUS: no release-profile general suite; "Only Windows x86-64 has been exercised" |

### 4.2 Distributed (pillar 1 levels B, C, D)

Condensed from P1 section 1.2 (28 rows). "Reads local" is the key architectural fact: GET is served from the local cache only; the router is used by DELETE (`server.rs:1990-1995` [checked]) and not by GET.

| Capability | Class | Where | Tests / evidence | Indep. | Limits |
|---|---|---|---|---|---|
| mTLS identity and admission (TLS 1.3, cluster CA, SAN, fingerprint pinning, `active` flag), per-frame revalidation | IMPLEMENTED, TESTED | `src/security/cluster.rs`, `dist/node.rs:393-406` [P1] | `security_phase3_*` suites (24/13/11 reported) | reading only | Identity only, no capability; no cert rotation; PKI generation exists only in test code (`rcgen` is a dev-dependency, `Cargo.toml:28` [checked]) |
| Framing: 28-byte header, 1 MiB payload cap, 4 KiB key cap, eight message types | IMPLEMENTED, TESTED | `dist/frame.rs:22-31` [checked] | codec tests | none | Version 1 only; an unsupported type terminates the connection (`node.rs:661-664` [checked]) |
| Connection pool, per-node bounds, pre-warm of **connections** | IMPLEMENTED, TESTED | `dist/pool.rs`, `router.rs` | phase3c suite | none | "Pre-warm" opens connections, not cache content |
| Static membership read once from file | IMPLEMENTED, TESTED | `main.rs:263-275`, `u8` ids at `main.rs:298` [checked] | phase3 suite | none | Revocation = edit file and restart |
| Runtime membership update/revocation `ClusterMembershipRegistry::update` | PROTOTYPE ONLY (library; no non-test caller [checked]) | `cluster.rs:405` | `test_p3c_*` | none | Unreachable from the binary |
| Dynamic join/leave, gossip | NOT IMPLEMENTED (design now: P1 5.1) | `NodeState::Joining` is only an enum value | none | n/a | |
| Key invalidation: local purge first, then synchronous leg to canonical primary (150 ms), failover to one acting primary, async fan-out to everyone else | IMPLEMENTED, TESTED | `server.rs:1990-1995`; `router.rs:1790-1860`; default timeout 150 ms [checked] | `distributed_consistency_tests`, `cluster_followup_tests`, `cluster_prewarm_repair_tests`, `lab_cluster_scenario_tests` [P1] | authors only | A 204 proves local purge plus primary (or acting primary) ack. For keys whose primary is the issuer, the "primary ack" comes from the issuer's own listener (Appendix A, R4) |
| Invalidation scopes ExactKey / Tenant / FullCache | IMPLEMENTED on the wire; only ExactKey reachable from HTTP | `node.rs:533-559` [checked] | ledger-collapse tests | none | Any active member may send any scope for any tenant |
| Repair ledger (1024 entries, 256 KiB keys, 16 tenant flushes per peer; collapse to tenant, then full flush); 64 entries per 250 ms tick | IMPLEMENTED, TESTED; **durability NOT IMPLEMENTED** | `router.rs:54-60,137-480` [checked: bounds at 56-58] | in-process tests | none | In memory; dies with the issuer |
| Sequences, replay ring, contiguous-ack tracker | IMPLEMENTED, TESTED | `router.rs`, `anti_entropy.rs` | `protocol_model_tests` permutes 4 actions on one cache | none | Receiver applies every InvalidateRequest regardless of sequence; the model test is single-node and does not assert the in-between interleaving (`tests/integration/protocol_model_tests.rs:95-106` [checked]) |
| Anti-entropy pull reconciliation (250 ms worker) | IMPLEMENTED, TESTED; effectiveness UNVERIFIED (claims-ledger C6 "weak") | `main.rs:375,383` [checked]; handler `node.rs:578-660` | `security_phase4c_antientropy_tests` | none | Digest is over invalidation records, not cache contents; an epoch mismatch closes the connection (`node.rs:590-595` [checked]) |
| Peer state machine Active/Suspect/Recovering/Failed/Offline | IMPLEMENTED (partly), TESTED | `node.rs:10-18`, `router.rs` | `test_m3_04a_*`, `test_m8_*` | none | Detection is a side effect of invalidation traffic. The heartbeat only probes idle pooled connections (`pool.rs:360-380` [checked]); its payload is empty (`pool.rs:~410` [checked]) |
| Issuer crash/restart with owed invalidations | **Defect REPRODUCED** (by the patch author, 4 of 4 original/flag-off runs); durable log PROTOTYPE ONLY | `docs/research/m3_durability/` (`proposed_patch_2_src.diff`, not applied) | `reproduction_report.md`: 3 gateways, mTLS, fault proxies, Windows loopback, debug | not independent | Peers cannot detect an issuer restart. Needs owner approval and an independent review |
| Cluster GET (`GetRequest` handler) | PROTOTYPE ONLY, unsafe to extend | `node.rs:409-462` [checked] | `known_gaps.rs` pins the stub | none | Fabricates `"MISS val_node_<id>"` and, on a miss, stores that fabricated resource through `guard.complete(Success)` into the node's serving cache (Appendix A, R9). No production caller of `forward_key_over_tcp` |
| Placement: rendezvous hashing for DELETE routing only | IMPLEMENTED | `router.rs:1343-1366` [checked] | n/a | none | `DefaultHasher` (unspecified across Rust versions: LITERATURE); `route_key` skips non-healthy nodes, so owners depend on each node's health view |
| Per-process resources (byte budget, entry cap, origin cap) | IMPLEMENTED per process | `config.rs`, `main.rs` | `byte_accounting_tests`, `origin_cap_tests` | partial | N nodes = N budgets; origin sees up to N times the cap |
| Multi-process harness and TCP fault proxy | IMPLEMENTED (test-only) | `tests/support/cluster_harness.rs` (`kill_on_drop` only at `:494` [checked]; no kill/restart helper in the working tree) | `cluster_harness_controls_tests` | none | Not a shipped feature |
| Cluster observability (cluster view, per-peer ledger depth, Prometheus) | NOT IMPLEMENTED (design: P1 S1.5) | n/a | n/a | n/a | |
| Distributed intelligence, coordinated pre-warming | NOT IMPLEMENTED | no file under `src/intelligence/` or `src/intel/` references a router or peer [P1, audit] | n/a | n/a | |
| Multi-host physical evidence; Linux; more than 5 nodes | UNVERIFIED (never run) | n/a | n/a | n/a | |
| Demo `POST /api/control NODE_FAILURE`, three placeholder nodes | PROTOTYPE ONLY (logical flag) | `server.rs:1086-1091`, `main.rs:212-214` [P1] | n/a | n/a | Must be labelled SIMULATED |
| ADR-003 consistency contract (G1, G2, G3) | DESIGNED; G2/G3 as written RETRACTED by the 2026-10-30 amendment | ADR-003 | partly tested | agents only; no owner ratification recorded | Replaced in section 7 by numbered clauses |

### 4.3 Intelligence (pillar 2)

From P2 section 1. Wired into serving: mode parsing and storage (`main.rs:459-470`), the origin gate, two HTTP routes. Everything else is a library used by tests and the lab.

| Component | Class | Where | Wired? | Notes |
|---|---|---|---|---|
| Mode enum Off/Observe/Advisory/Canary/Active, default Off; config `PHANTOM_INTELLIGENCE` accepts exactly those five | IMPLEMENTED, TESTED | `intelligence/mod.rs:22-36`, `config.rs:161` [checked] | yes | No `shadow` value. Doc comment says Canary is "bounded live traffic exposure (f <= 5%)" but code only enforces the origin cap in Canary/Active (`executor.rs:217-220` [checked]); `route_bucket`/`apply_policy` have no serving caller [checked by grep] |
| Origin gate (static cap / AIMD, RAII permit) | IMPLEMENTED, TESTED; AIMD RETRACTED as promoted feature | `executor.rs` | yes | Scenario B `TRADE_OFF_FAILURE`; sealed M6 H1 NOT_SUPPORTED (Tier 1 SIMULATED). Preserved, not proof against intelligence |
| Audit store + `/api/v1/intelligence/audit`, `/status` | IMPLEMENTED, TESTED | `audit.rs`, `server.rs:213-214,389-458` | routes yes | **No producer**; serves an empty store [P2] |
| Signal pipeline (EWMA, Space-Saving, Zipf, CUSUM, hot-key emergence) | IMPLEMENTED, TESTED (unit); accuracy UNVERIFIED | `signals.rs` | no | CUSUM threshold 500, slack 50; emergence = 3x and >= 30 are absolute, not scale-free [P2] |
| Shadow evaluator | IMPLEMENTED, TESTED | `shadow.rs` | no | CI treats two policies on one stream as independent [P2] |
| Decision engine `PieDecisionEngine` | IMPLEMENTED, TESTED, UNVERIFIED independently | `engine.rs` | no | `pie: None` (`main.rs:433` [checked]); P99 tracker not a quantile (reported) |
| Candidates (admission, stale controller, AIMD, EWMA forecaster, emergence predictor, stump) | PROTOTYPE ONLY (lab) | `candidates.rs` | lab only | Forecaster vectors are unbounded `Vec<f64>`: violates the explicit-bounds rule |
| Legacy threshold PIE | RETRACTED / quarantined | `src/control/pie.rs` | no | Do not extend (CLAUDE.md 6b) |
| Pre-warm of the cache | DESIGNED BUT NOT IMPLEMENTED | none | no | Only connection-pool pre-warm exists |
| Lab T1 simulator, T2 live Scenario B | IMPLEMENTED, TESTED | `src/lab/t1_sim.rs`, `src/lab_live/*` | offline | T1: constant-service-time origin (no load feedback); T2: pure-delay scripted origin; M6 sealed set spent |
| RT-0 prior-art closure | **OPEN** (PARTIAL) | `rt0_prior_art_sweep_2026-10-30.md` | n/a | 21 queries; makes no novelty claim; earlier "RT-0 PASS" snapshot not supported (STATUS) |

### 4.4 Product, security, observability (pillar 3)

From P3 section 1 and P4 section 1.

| Capability | Class | Where | Tests / evidence | Limits |
|---|---|---|---|---|
| Ingress JWT: HS256 only, iss/aud/exp, 10 s leeway | IMPLEMENTED, TESTED, independence UNVERIFIED | `security/jwt.rs:153,171` [checked] | `m7_*`, `test_phase1_*` [P3] | One shared symmetric secret; no `jti`/`nbf`/max-lifetime; rotation (`kid`) not wired in the binary [P3]; SEC-M7-09/10/11 open |
| Role checks; tenant namespace isolation (secure mode) | IMPLEMENTED, TESTED | `server.rs:985-1081`, `:1209-1219` [P3] | `m7_http_rbac_matrix_in_secure_mode`, `sec01_*` | Namespace only; in development mode the tenant is the client's `x-tenant-id` header (not isolation) |
| Per-tenant byte/entry quota, fair eviction, per-tenant origin share | **no code; design (P3 section 4)** | n/a | none | One tenant can evict another's entries and take its permits |
| Rate limits, auth-failure counters, connection/header timeouts | **no code; design** | n/a | none | SEC-M7-11 |
| Peer scope authorization | **no code; design (P3 section 6)** | `node.rs:533-559` [checked] | only legitimate scopes tested | A compromised member can purge any tenant or everything |
| Token minting, pairing, revocation | no shipped tool; design (P3 section 2) | `create_token` is library only | n/a | Revocation today = expiry or restart with a new secret |
| Evidence manifest tree identity | **UNVERIFIED** | `lab/manifest.rs:103-104` [checked] | n/a | `git_tree_hash` is the constant HEAD commit id, `is_dirty: true` constant (Appendix A, R11) |
| Lab API `lab-1` (16 SSE streams, 15 s heartbeat, one active run, persisted `events.jsonl`) | IMPLEMENTED, TESTED; G2 PASS with dispositions; second security review pending | `lab_api.rs:50,59,60` [checked] | `tests/integration/lab_api/` | Process-start capability behind Operator; no run rate/disk quota |
| Real-gateway lab runs (Scenario B) | IMPLEMENTED, TESTED; G1 PASS with dispositions | `lab_live/*`; `GatewaySpec` has one exe, one origin (`gateway.rs:20-28` [checked]) | `lab_live_scenario_b_tests` | **No multi-gateway run kind**; pure-delay scripted origin |
| `/api/events` 100 ms aggregate snapshot; diagnostics ring (512, lossy `try_lock`) | IMPLEMENTED | `server.rs:200,959`; `diagnostics.rs` [P4] | n/a | No per-request, per-decision or per-peer event; no request/trace id |
| UI: Lab screens, evidence, tokens, provenance badges | IMPLEMENTED; three headless-browser rounds (exploratory); G3 verdict not recorded | `ui/src/*` | about 212 UI tests (static scans and server-render checks) [STATUS] | No independent human design review, no usability test; Cluster and Intelligence screens are placeholders |
| Legacy Simulation screen | PROTOTYPE ONLY | `ui/src/SimDemo.jsx` (517 lines; own `EventSource` at `:92`, POST `/api/control` at `:177` [checked]) | n/a | Different visual language; fabricated-looking cost tiles [P4] |
| Packaging, build stamp | PARTIAL | `packaging/`, `BUILD_COMMIT` via `option_env!` | none found | Ordinary `cargo build` yields `"unknown"`; no signing/SBOM/advisory scan |

---

## 5. Capability matrices

One table per area. Class = today's class. Gate = what must be true before the capability may be *claimed* (not before it may be designed).

### 5.1 Single-node

| Capability | Today | Target | Gate for a claim |
|---|---|---|---|
| Fenced fill/delete correctness | IMPLEMENTED, TESTED | add model/mutation evidence; independent review | mutation test that fails without fencing; independent reviewer |
| Freshness incl. TTL ceiling and wired default TTL | PARTIAL (`PHANTOM_DEFAULT_TTL_MS` NotWired; no ceiling) | ceiling + wiring (P1 S1.7) | owner decision OD-14; test fails without change |
| Byte accounting incl. per-tenant (I9) | global only | Stage A accounting hook (P3 S7) | property test over random ops; one-site-omitted test fails; release micro-benchmark with interval |
| Per-tenant caps and fair eviction | none | Stage B (P3 S8) | noisy-neighbour suite (T-NN...), independent review |
| Origin protection incl. bounded wait queue and warm priority | cap only | queue is deterministic (P2 I6) | E2 headroom result on an E1-validated plant |
| HTTP hardening (timeouts, connection cap, stream cap) | partial | P3 S9 | probes T-AF, slow-send |
| Release-profile soak and suite | none | needed for any performance claim | release run with interval |

### 5.2 Distributed, Levels A to D

| Level | Capability | Today | Target | Gate |
|---|---|---|---|---|
| A | single-node foundation | see 5.1 | see 5.1 | see 5.1 |
| B | real multi-process nodes, identity, authenticated comms | IMPLEMENTED, TESTED (loopback) | + shipped PKI tool; runtime membership | independent review of cluster code; release-profile physical LAN study (P1 S1.8) |
| B | reliable propagation + recovery incl. issuer crash | repair/anti-entropy TESTED; **issuer-crash defect REPRODUCED**; log PROTOTYPE ONLY | durable log + drain, reviewed (P1 S1.1) | K1-K20 crash matrix, independent review, Windows and Linux |
| B | explicit consistency/failure contracts | clauses C1-C13 drafted (section 7) | owner-ratified | each clause has a falsifier and status |
| B | cluster observability | NOT IMPLEMENTED | P1 S1.5 | labelled measured/inferred/unavailable; honesty tests |
| B | peer authorization | identity only | capability per member (P3 S6 + P1 S1.3) | forbidden-scope tests fail without change |
| C | stable placement | `DefaultHasher`, health-dependent | fixed hash, owners from membership epoch (P1 S2.1) | cross-build agreement test; 1/N movement property test |
| C | peer-fill / coordinated reads | NOT IMPLEMENTED; stub fabricates | O2 peer-fill, shadow first (P1 S2.2-S2.3) | evidence gate P1 4.3 (model + multi-process tests + independent review + physical benefit/cost study) |
| C | coordinated capacity, cluster origin protection, tenant-aware allocation | none (per node) | R1 static share, then R3 owner-concentrated origin access (P1 S2.4); tenant allocation (S2.5) | origin load vs N under fixed workload |
| D | cluster-aware observation, hot-key/flash-crowd detection, coordinated pre-warm, placement/admission, cross-node optimisation | NOT IMPLEMENTED | signals with provenance, then detection, then pre-warm, then optimisation (P1 S3.1-S3.2; P2 I12) | single-node gate passed first (P2 reply to P1); durable invalidation + sync lease; offline, shadow, canary, active ladder |
| WAN | multi-region | deferred, not dismissed | own failure model, ADR and gate | not claimable from Level B/C clauses |

**Distinction kept throughout (owner mandate).** *Distributed invalidation* = N independent caches plus a bus that deletes entries (what exists). *Coordinated distributed caching* = nodes cooperate on reads, placement, capacity and origin load (Level C; does not exist). Calling the first "a distributed cache" in a demo or claim is the specific mistake this file is built to prevent. UI wording at Level B: "N independent caches with an invalidation bus".

### 5.3 Intelligence

Capabilities from P2 section 3, ranked by P2 (judgement, not measurement). "Kill" = the pre-registered abandon condition.

| ID | Capability | Today | Action class | Kill / stop | Gate |
|---|---|---|---|---|---|
| I1 | Truthful workload observation (hot keys, regime, burst) | library, NOT WIRED; accuracy UNVERIFIED | none (observation) | recall < 0.9 at stated memory (E4) or hot-path adder beyond budget (E0) | E0, E4 |
| I2 | Miss-cause split, origin-pressure signals | counters exist; split DESIGNED | none | counts disagree with existing counters | agreement test |
| I3 | Declared-demand pre-warming (Demand Calendar) + deterministic scheduler | DESIGNED BUT NOT IMPLEMENTED | A1 warm | E9 oracle gain below margin abandons predictive warming (calendar-literal may remain); not better than cron warmer (K-C1); harm to origin peak or other tenants (K-C3) | E9, E11, E12 |
| I4 | Hot-key refresh-ahead | DESIGNED | A1 | not better than XFetch early refresh with interval excluding zero | T1 hot-key worlds |
| I5 | Early flash-crowd detection/classification | PROTOTYPE ONLY | gates other actions | not faster than threshold baseline at matched false alarms | T1 families |
| I6 | Origin-protection advisor (Little's-law sizing) and bounded wait queue | gate IMPLEMENTED; AIMD NOT SUPPORTED; advisor DESIGNED | A6 within operator [min,max] | E2 NO_HEADROOM on E1-validated plant | E1, E2 (BLOCKED on owner approval of lab changes and v3 salt) |
| I7 | Eviction/admission strategy selection | PROTOTYPE ONLY | A6, A2 | not better than best fixed faithful policy | E5 with TTL/size-aware shadow |
| I8 | TTL / Cache-Control recommender | DESIGNED | A4 (advisory only) | no reduction of revalidations at equal staleness exposure | lab traces |
| I9 | Per-tenant policy advice | BLOCKED (no quotas, no tenant telemetry) | A5 | no worst-tenant gain | needs W3 S7/S8 |
| I10 | Restart/flush warm-up planner | DESIGNED | A1 | origin peak not lower than lazy fill | shares I3 scheduler |
| I11 | Outcome ledger, calibration, regret governor | PARTIAL (store, no producer) | demotion | no better than no demotion under drift | planted-signal worlds |
| I12 | Distributed signal aggregation, warm coordination | BLOCKED | n/a | | durable invalidation, P1 ownership, P3 privacy |
| I13 | Seasonality forecasting | BLOCKED on data (no real multi-week trace) | n/a | MASE >= 1 vs seasonal-naive on held-out real trace | trace or design partner |
| I14 | Key-fragmentation detector | DESIGNED; prior art not searched | report only | precision below bar | |

### 5.4 Product (observability, Lab, Demo Center, security as capability, design)

| Capability | Today | Target | Gate |
|---|---|---|---|
| Six-label tier vocabulary + topology class + fault layer | partial vocabulary (MEASURED/DERIVED/SIMULATED/INFERRED/UNAVAILABLE/RETAINED_*) | REAL/SIMULATED/REPLAYED/PREDICTED/DERIVED/UNAVAILABLE with rules R1-R6 (P4) | DOM test: every value badged; logical-flag fault never drawn as failed |
| Event layer `demo-1` (single writer, gap-free ids, class A/B/C, sampling, resume) | none (`lab-1` and 100 ms snapshot only) | P4 section 2 | property tests: gap-free ids, one `gap` event per dropped range, no duplicates on resume |
| Per-request and per-decision tracing | outcome-only client-side records (`loadgen.rs`) | gateway taps (owner approval, benchmark) | overhead protocol with interval; owner bound OD-27 |
| Demo Center (Scenarios, Stage with lenses, Compare, Evidence, Runs) | `#/lab` only | P4 section 3 | real-browser rounds + independent human review |
| Compare/report builder | Scenario B `paired_vs_base` only | backend-computed, compatibility check | refuses mismatched binary hash; byte-identical regeneration |
| Security as visible capability (pairing, scoped tokens, revocation, evidence integrity) | partial (JWT, roles) | P3 S1-S4, S11 | each demo lists what it does NOT prove |
| Design system (type tiers, elevation, state grammar, motion standard) | tokens + about 11 components; shadow tokens defined but unused (reported) | P4 5.4 | owner approval AND independent human design review (O8-2) |
| Pairing and projection mode | none | tickets + audience view | P3 sign-off |
| Multi-device topology | never run | `MULTI_HOST_LAN` tier label created only after a real run | physical run retained |

---

## 6. Proposed target architecture (logical layers, not new services)

**RECOMMENDATION.** Six logical layers over the existing single gateway binary. They are a way to decide where code and tests belong and who may call whom; they are not processes. "New" means a module that does not exist today. `src/bin/` exists; I did not read its contents.

| Layer | Responsibility | Existing modules reused | New (proposed) | Runs where |
|---|---|---|---|---|
| **A Deterministic cache execution** | Correctness: fencing, single-flight, freshness, eviction, byte accounting, origin cap, stale cap, tenant epochs. Owns every guarantee. | `src/core/{cache,coalesce,eviction,freshness}.rs`; handlers in `src/api/server.rs`; origin gate in `src/intelligence/executor.rs` (logically layer A/D though it lives under `intelligence/`) | tenant accounting hook and quota admission (P3 S7/S8; invariant-heavy, independent review); TTL ceiling (P1 S1.7); warm-only gate class (P2 P-5) | per node |
| **B Distributed coordination** | Identity, membership, invalidation propagation, repair, anti-entropy, failure detection; later placement, fill protocol, cluster budget share | `src/dist/{node,router,frame,pool,session,anti_entropy}.rs`, `src/security/cluster.rs` | durable owed-invalidation log; heartbeat with incarnation id and issued-sequence advert; runtime membership admin; stable placement; `FillRequest/Response` replacing the stub; signal frames (Level D) | per node, peer to peer; no central coordinator proposed |
| **C Intelligence** | Observe, features, forecast, shadow evaluation, decide. Proposes only. | `src/intelligence/{signals,shadow,engine,candidates,audit,mod}.rs` | event-tap consumer; Demand Calendar and scheduler; calibration monitor | per node (state keyed by tenant); cluster aggregation only as tenant-scoped aggregates (Level D) |
| **D Safety and policy** | Deterministic validator for every intelligence decision and every cross-boundary request: budgets, refusal reasons, peer capabilities, rate and quota limits, role checks. Does not import forecaster or model code. | `security/{jwt,upstream,cluster}.rs`, role checks in `server.rs`, clamps in `executor.rs`, `PHANTOM_MAX_STALE_MS` | L6 validator with action interface A1-A7 (P2 P-4); peer capability field on `MemberNode` (P1 S1.3, P3 S6); tenant registry; per-principal limiters; `jti` denylist | per node; membership and tenant policy files identical on all nodes, hash compared |
| **E Observability and experimentation** | Telemetry, diagnostics, audit, session stream, Lab runner, evidence | `src/intel/{telemetry,diagnostics,origin_metrics}.rs`, `src/lab/*`, `src/lab_live/*`, `src/api/lab_api.rs` | one bounded event tap with tiered consumers shared by intelligence audit and the Lab (P2 2.1 rule 6); `demo-1` session writer; multi-gateway run kind; compare/report; evidence bundle and verifier | taps per node; the session writer on the presenter/control host |
| **F Product and control plane** | Operator console, Demo Center, pairing, minting, configuration, packaging | `ui/*`, `src/config.rs`, `src/health.rs`, `/api/control`, `packaging/`, `scripts/` | Demo Center shell, pairing service, minter, cluster PKI tool, optional control listener | control host; guests get read-only views |

### 6.1 Data flow (one request, one decision)

```
 client --HTTP--> [A] ingress: authn/authz (D) -> cache lookup / single-flight / origin gate (A+D) -> origin
                    | (one atomic mode check, one bounded push)
                    v
              [E] bounded drop-newest event tap -> [C] observe -> features -> forecast -> decide
                    |                                                          | (action record)
                    |                                                          v
                    |                                   [D] safety/policy validator --refuse/clamp--> audit [E]
                    |                                                          | accept
                    |                                                          v
                    |                       [A] executes via the NORMAL paths (a warm is a fill request;
                    |                           finalize_fetch generation/epoch checks still apply)
                    v
              [E] session writer -> demo-1 stream -> [F] Demo Center / Lab
 DELETE --> [A] local purge first -> [B] sync leg to primary, async fan-out, ledger, anti-entropy -> peers [A at peer]
```

### 6.2 Where the safety and policy layer sits

1. **Between C and A.** Every intelligence action goes through one validator entry point (grep-checkable). Intelligence never writes cache entries directly. It may never change fencing, tenant epochs, key canonicalization, hard caps or quota values, extend freshness beyond origin directives or the stale cap, or evict an active or in-flight key (P2 2.2; CLAUDE.md 5, 6b).
2. **Between B and A on the receive side.** Peer frames are checked for capability and rate before they touch the cache (P1 S1.3, P3 S6). Today `node.rs:533-559` applies any scope from any active member [checked].
3. **At the edge of F.** Roles, tenant resolution, per-principal and per-tenant limits, optionally a separate control bind so guests cannot reach control routes.
4. **Failure rule.** Any stall, stale input or UNAVAILABLE provenance gives NoAction and, for actuators, a return to the static baseline (`revert_to_safe`, `executor.rs:390` [P2]). Off is identical to the static path.

### 6.3 Per node, per cluster, per control plane

| Where | Lives there | Never lives there |
|---|---|---|
| **Per node** | cache, fencing, origin cap and quotas (enforced per node), tenant accounting, peer-frame validation, event tap, observation, features, shadow evaluation, validator, actuation, local ledger and (proposed) durable log | any other node's raw data or authority |
| **Cluster (between nodes)** | invalidation frames, repair, anti-entropy, heartbeats with incarnation and sequence adverts, (Level C) fill frames, (Level D) tenant-scoped aggregate signals | cross-tenant per-key statistics; raw keys without a privacy decision; a coordinator that owns correctness; consensus (none proposed) |
| **Control plane** | session writer and `demo-1` stream, Lab runner, pairing and minter, PKI tool, evidence signing key, report builder, design-system assets | the signing secret on guest-facing hosts; operator tokens on guest devices |

**Cluster totals are not pooled.** Quotas and origin ceilings are per node unless P1 S2.4/S2.5 land; a tenant's footprint is at most N times its per-node cap and its origin load at most N times its per-node ceiling. Every cluster view must say so (P1 7.1; P3 4.6; Appendix A, R7).

### 6.4 Reuse versus new

Reuse, no throw-away: framing and codec (additive message types), mTLS identity and registry, connection pool, rendezvous router and node-state machine, repair ledger, anti-entropy ring, per-node fencing, multi-process harness and fault proxy, Lab `RunHandle`/`events.jsonl`/`Tail` pattern, JWT roles. New: a read path, a consistency contract and a control plane on the same transport and identity. Level B needs no change to `core/cache.rs`; Level C uses the existing in-flight registration API; P3 Stage A/B and P2 warm priority do touch the cache and gate code.

---

## 7. Distributed consistency and failure contracts

Source: P1 section 3, taken in substance. **Failure model F (the only one these clauses are written against):** crash-stop of any process; message loss, delay, duplication and reordering on a LAN; partitions that heal; restart with an empty in-memory cache; no Byzantine members; no WAN; no disk loss unless stated. Not in F: clock steps, disk full or corruption, asymmetric partitions beyond what the fault proxy expresses, more than 5 nodes (never exercised), host power loss.
**Standing preconditions:** P0.1 the operator changes the origin first, then issues DELETE; P0.2 an origin fetch that starts after the change returns the new content; P0.3 members are authenticated and not malicious for correctness clauses (malice is P3's); P0.4 no clause depends on synchronized clocks.
**Terms:** issuer I (receives the DELETE), canonical primary P (`route_key`), acting primary, peer. "Established" means a test exists whose body a draft author read and which is reported passing on loopback/debug; it never means independently verified.

### 7.1 Clauses C1 to C13

| # | Guarantee G | Explicit non-guarantee NG | Falsifier | Status |
|---|---|---|---|---|
| **C1 Local read-after-delete** | After 204 on I, no later read on I is served the pre-delete body (local purge precedes any cluster action, `server.rs:1990` [checked]; DELETE detaches the in-flight entry, `cache.rs:~1638` [checked]) | Nothing about other nodes; a request that coalesced onto a pre-DELETE fetch before the DELETE may still get that body | Concurrent GET/DELETE history check on one node under the fault proxy; mutation that moves the purge after the cluster call | **Established** (reported) |
| **C2 Meaning of a 204** | Local purge done, and P (or the acting primary) acknowledged within the 150 ms leg, **except** when P is I itself, in which case the ack is I's own | Anything about any other peer; durability of owed work | Blackhole one non-primary peer: 204 and peer still stale until repair; kill P between receipt and apply: no 204 | Established by code reading and characterisation tests; **the "P is I" case is untested** (code reading supports it, Appendix A R4) |
| **C3 Propagation to live peers** | Each reachable, non-faulty peer receives the invalidation by bounded retry (up to 4 attempts, 25-200 ms backoff, 150 ms per attempt) and applies it. The 250 ms bound is a healthy-LAN *target* | Any bound under loss, delay beyond the retry budget, or load. "Zero stale at 50 ms" is RETRACTED | Release-profile multi-host study, N >= 1000 per cell, P50/P99/P99.9/max with intervals, plus delay and loss cells | **Not established** at LAN or release. Loopback/debug measurement only (P99 about 10-20 ms; N=1000 max 49.2 ms; cold first use 35-117 ms; reported in the claims ledger) |
| **C4 Peer offline or partitioned, issuer alive** | While I stays alive and within ledger bounds an unreachable peer is owed the invalidation; after heal the repair worker (250 ms tick, 64 entries per tick) delivers it; the peer goes Suspect, Recovering, Active only after debts are repaired and a pull reconciliation succeeds. Overflow collapses to tenant flush, then full flush: coarser, never silently lost | Any staleness bound during the partition other than the entry's remaining freshness lifetime; nothing if I dies during the partition | Partition one peer for T, DELETE K times, heal, assert convergence; vary K past 1024 | **Established on loopback** |
| **C5 Issuer crash or kill** | Today: **none**. A peer that had not applied an invalidation owed by a crashed issuer stays stale until the entry expires (REPRODUCED). With the durable log, adopted and reviewed: an accepted DELETE not acknowledged by every non-primary member is re-delivered after any restart of I; bound = time until I restarts + repair latency (eventual, not D_max) | Disk loss, deleted log and sentinel, a second failure before delivery plus log loss, fsync modes other than `always` under power loss, a permanently dead issuer; anything about a DELETE that did not return 204 | Crash-point matrix K1-K20 (7.3) | **Not established**; defect reproduced, fix is a prototype |
| **C6 Graceful shutdown of I** | Today none (shutdown drops queued work, `main.rs:509-514` [P1]). With drain: delivered within `PHANTOM_SHUTDOWN_DRAIN_MS`, else kept in the log | | Stop with a blackholed peer, heal 400 ms later, restart | **Not established** (scratch-copy test only) |
| **C7 Receiver restart** | A restarted node has an empty cache so cannot serve pre-restart content; applies later invalidations normally | No warm restart; origin surge on return | Fill, restart receiver while an invalidation is missed, GET must miss | Established by construction and `test_h1_*` |
| **C8 Staleness vs TTL** | After an invalidation is *entirely* lost to a node, that node can serve the entry until its freshness lifetime ends, plus the stale-serving window if the response allowed it. Lifetime comes from the origin (`freshness.rs:263-276` [checked]); default 60 s only when the origin gives nothing | "Staleness <= 24 h" and "staleness <= TTL" in any form that hides the origin-controlled lifetime | Origin sends `max-age=86400`, invalidation blackholed forever, measure | Honest form established by code reading; ADR-003 G3 form RETRACTED. Making G3 true needs a TTL ceiling (OD-14) |
| **C9 Ordering** | Invalidations are idempotent tombstones; the receiver applies every one regardless of sequence or issuer (`node.rs:533-559` [checked]); the final state does not depend on order | Cross-issuer or cross-key ordering; multi-key atomicity. Delete-then-refill is protected only by per-node fencing and P0.1/P0.2 | Two issuers delete the same key with reordering via the fault proxy | Code reading only; no multi-process reordering test found |
| **C10 Fill vs invalidation (origin fills)** | An origin fill that began before an invalidation was applied on the same node cannot be stored after it (generation bump and detach; `finalize_fetch` refuses on changed generation, `cache.rs:1457-1469` [checked]) | Any protection if the frame is never applied (that is C3 to C5) | Multi-process test with a slow origin returning the old body while the invalidation arrives mid-fetch | Single-node established; **multi-node not tested** |
| **C11 Membership, revocation, admission** | A frame from a peer not in the registry or not active is rejected on every frame, not only at connect | Today nothing changes the registry at runtime ([checked]: no non-test caller of `update`), so "revoked" means "file edited and process restarted" | End-to-end revocation test through an admin path | Library established; end to end **not established** |
| **C12 Read semantics (Level B)** | Each node answers from its own cache; a client that always uses one node sees C1 | Monotonic reads across nodes; read-your-writes across nodes before C3; linearizability across nodes | Load-balancer-flip client showing version regression | Non-guarantee by design |
| **C13 Authorization of scope** | Today identity only | Any limit on which scope or tenant a member may invalidate. At 10,000 tenant epochs a member forces a **global** epoch bump (`cache.rs:581-593` [checked]); FullCache does it directly (`node.rs:553-558`) | Valid member sends FullCache, or 10,001 distinct tenants, and the peer is flushed | Non-guarantee; P3 owns the fix |

### 7.2 Proposed Level C clauses (not established; Stage 2 acceptance targets)

| # | Clause |
|---|---|
| **CC1 Owner-first invalidation** | For every DELETE, the key's owner (from the membership epoch) has applied the invalidation before any non-owner is sent the frame, or the owner is marked as owing it and refuses fills until repaired |
| **CC2 Fill fencing** | A fill is registered as an in-flight operation on the requester before the request is sent, so the requester's local fence covers a concurrent invalidation; the owner never serves a cached body that predates an invalidation it has applied |
| **CC3 No fill across placement disagreement** | If requester and owner disagree on membership epoch the fill is refused and the requester goes to the origin |
| **CC4 No stale resurrection from a lagging owner** | A node that cannot prove it is synchronized (lease expired, gap detected, Suspect or Recovering) refuses fills. The lease uses each node's own monotonic clock and heartbeats that advertise "issued up to sequence n" and an incarnation id |
| **CC5 No lifetime extension** | A filled entry keeps its original creation time and remaining freshness; peer-fill never lengthens life |
| **CC6 Failure fallback** | Owner unreachable, slow or refusing: the requester fetches from the origin under its own cap; it never returns an error caused only by the peer |

New hazards (P1 4.3): H1 requester fills before the owner applied (owner-first ordering); H2 response in flight when the invalidation lands (register the fill as an in-flight operation); H3 owner mismatch (placement from membership epoch, not health); **H4 stale resurrection from a lagging owner, the most dangerous new hazard** (synchronization lease; length and false-positive rate are research questions); H5 issuer restart (incarnation id plus durable log, complementary).
**Evidence gate before Level C may be claimed:** (1) a multi-node protocol model with seeded-mutation checks that each of CC1-CC4 is necessary; (2) multi-process tests provoking H1-H5 with controlled delay; (3) independent review of fill and fence code plus a test that fails without the change; (4) release-profile, multi-host measurement of benefit and cost on at least 3 physical devices against a Level B plus external load balancer baseline. **A legitimate outcome of the gate is that the owner decides Level C is not worth it for the target workloads; that is not a failure of the vision.**

### 7.3 Crash-point matrix for the durable-log review

P1 6.3 specifies 20 kill points K1-K20: before the primary leg; after primary ack; mid group-commit; **after `sync_data`, before fan-out (the most important correct case)**; after dispatch; after some acks; mid-drain; drain deadline expired; mid-compaction; between rename and reopen; during boot replay; a permanently gone peer; torn tail at every byte; log deleted with sentinel present; read-only or full disk; clock stepped forward; concurrent DELETE storm plus kill; power loss; Linux kill -9 and SIGTERM; Windows service stop. The reviewer must not be the patch author or the reproduction author and receives the code, not P1's conclusions. K18 (power loss) needs a physical or VM power cut; K19 and K20 have never run.
**Preliminary design findings from P1 6.4 (not the commissioned review; UNVERIFIED):** a cancellation window widened by the patch (client disconnect between primary ack and fan-out; recommend a detached task); wall-clock age-out can silently drop debts after a forward clock step; fsync-failure semantics on Linux; partial `write_all` leaves mid-file garbage (safe but coarse full flush); fail-open is weakest under load; Windows rename durability unverified; debts for departed members; replay amplification (a crash-looping issuer repeatedly flushes peers); peers still cannot detect an issuer restart without the incarnation id.

### 7.4 Explicit non-guarantees (print on any cluster screen or document)

- No linearizability, monotonic reads or read-your-writes across nodes.
- No staleness bound other than the entry's origin-set lifetime plus the stale-serving window, until a TTL ceiling (OD-14) and C3 evidence exist.
- A 204 does not mean the object is gone everywhere. For about 1/N of keys it means the issuer purged itself.
- No guarantee after issuer loss without the durable log, and none under disk loss even with it.
- No cluster-wide quota or origin cap; capacity does not pool; origin load can be N times a per-node cap.
- No WAN or multi-region claim; no claim for more than 5 nodes, Linux, release builds or multiple hosts (none exercised).
- No Byzantine tolerance; an authenticated member is trusted for scope until S6 lands.
- Failure detection is a side effect of invalidation traffic; `Failed` and `Offline` are not produced by probes.

**Other P1 findings relayed.** The receiver does not enforce the membership epoch on InvalidateRequest (the echoed epoch is compared with the sender's own current epoch, trivially equal; only the anti-entropy handler enforces it, `node.rs:590-595` [checked]). P2's earlier "fill race for origin fills" is closed by the local fence; the real exposure is a missed frame. `architecture_trace_m3.md` and ADR-003's "5 s" reconciliation interval are stale; the worker runs every 250 ms (`main.rs:375` [checked]).

---

## 8. Intelligence architecture and evaluation principles

Source: P2 sections 2 to 7; RT-0 sweep.

### 8.1 Position

**FACT.** The only intelligence on the serving path is an origin-concurrency gate (static cap or AIMD) and mode parsing. AIMD did not beat a static cap in sealed M6 (H1 NOT_SUPPORTED, Tier 1 SIMULATED) or in Scenario B (`TRADE_OFF_FAILURE`, MEASURED_LOOPBACK). **That result is preserved as it stands.** It is not evidence that intelligence is not viable; the accountability report (11.3) says so itself. **INFERENCE (P2):** a defensible advantage must come from decisions the deterministic engine cannot make alone: knowing demand before it arrives (declared or detected), choosing where to spend a bounded origin budget, and telling the operator what they cannot see. **RECOMMENDATION:** build intelligence as a bounded advisor first (observe, explain, recommend), add one narrow actuator (budgeted pre-warming from a declared calendar) behind the safety layer, and let every further capability earn its place against a strong static baseline with a kill criterion. Nothing is "validated" before a pre-registered result says so.

### 8.2 Layers L0 to L8

L0 deterministic engine (authoritative) -> bounded drop-newest event tap -> L1 observe (counters, sketches, per-field provenance) -> L2 features -> L3 forecast (persistence, EWMA, seasonal-naive, Bayesian shrinkage of declared vs observed; intervals) -> L4 shadow evaluation (miniature simulations, paired comparison, no exposure) -> L5 decision (candidate action or NoAction, with predicted effect and interval) -> **L6 safety/policy validator** (deterministic; may refuse, clamp, expire) -> L7 actuation through the normal paths -> L8 outcome (realized effect, audit record, calibration, fallback). Intelligence proposes; L6 and L0 dispose; L6 does not import forecaster or model code; a prediction that never changes a decision is not an adaptive capability.

### 8.3 Closed action interface (anything not listed is impossible by construction)

| Action | Asks | Key preconditions checked by L6 | Budgets |
|---|---|---|---|
| **A1 Warm request** | Fetch key K for tenant T through the normal fill path before expected demand | mode >= Canary (Advisory records only); verified tenant, secure mode; key cacheable and not auth-bound (`BYPASS-AUTH` never warmed); remaining lifetime < lead time; origin healthy; warm queue not full; cooldown; active calendar entry or detector basis. **A warm must lose to an organic miss at the origin gate** (needs a warm-only gate class: new code in a safety-critical path) | warm fetches/s, in-flight warm fetches, warm bytes, per-entry, per-tenant, daily |
| **A2 Admission hint** | Mark a key class admit-first or probation | never denies a key with active ops or an explicit warm | hint table entries |
| **A3 Retention hint** | Protect a bounded set from eviction for a while | cannot override I1-I8 or the in-flight protection | protected bytes percent, entries |
| **A4 TTL recommendation** | Suggest `max-age`/SWR for a path class | advisory; never above origin directive or stale cap; lengthening blocked while invalidation durability is unproven | count |
| **A5 Origin-share suggestion** | Suggest the split between tenants and between warm and organic | clamped to operator min/max; needs per-tenant quotas (do not exist) | share table |
| **A6 Strategy switch** | Choose from a pre-approved menu (eviction policy among faithful implementations, cap within `[MIN,MAX]`, stale window within the cap) | hysteresis, cooldown, evidence gate passed, watchdog alive; global until per-tenant structures exist, so Advisory until then | switches per hour |
| **A7 Alert / no-op** | Typed alert or NoAction | always allowed | alert rate |

Every action carries an envelope (`action_id`, `kind`, exactly one verified `tenant`, `mode_at_issue`, `inputs` with provenance, `predicted_effect` with interval, `budget_claim`, `issued_at`, hard short `expires_at`, `revert_plan`, `reason_codes_if_refused`) and writes one audit record whether accepted or refused. Refusal reasons are named and each needs a negative test: **P2-R1 to R16** from the V2 design plus R17 (ring overrun) and R18 (audit store full); the prefix avoids collision with P4's rules and P3's risks (Appendix A, R12).
**Never:** change fencing or tenant epochs; extend freshness beyond origin directives or `PHANTOM_MAX_STALE_MS`; raise hard caps; change key canonicalization; bypass the origin gate; evict in-flight keys; read another tenant's state; write cache entries directly.

### 8.4 Mode ladder

| Mode | Behaviour | Promotion criterion | Demotion trigger |
|---|---|---|---|
| Off (default) | no tap; identical to static path | E0 overhead measured | n/a |
| Observe | tap and features; provenance on every field; nothing shown as advice | E4 accuracy within bound; no invariant violation in the window | overhead breach, drop rate above bound, any invariant violation |
| Advisory | recommendations with labels and intervals, audited, never applied | calibration on matured predictions in tolerance; precision/recall floor | calibration or audit failure |
| **Shadow (proposed; does not exist)** | counterfactual on live traffic, no exposure | counterfactual estimator validated in the lab | disagreement with later canary |
| Canary | real actions on a bounded slice (<= 5% key-hash buckets or one low-risk tenant), permanent holdback | pre-registered evidence, T2 confirmation, interference check | any guardrail breach: immediate revert to Advisory |
| Active | full budgets, per-tenant opt-in, regret governor | canary exit and **owner approval** | regret governor trip, calibration failure, repeated reverts |

**FACT:** `PHANTOM_INTELLIGENCE` accepts exactly off, observe, advisory, canary, active [checked]; today `canary` and `active` only turn on cap enforcement. Calling a mode "Canary" before key-bucket exposure exists is misleading: rename or document before any new action reuses the word (OD-36).

### 8.5 Experiment sequence

All v3/v4 sealed runs need new protocols and **owner-generated** salts and seed sets; no agent generates, stores or proposes one. Exploratory runs on tuning data are labelled EXPLORATORY and cannot trigger a kill. Confirmatory runs follow a frozen pre-registration with fail-closed freeze and a run-once marker. Designers have seen aggregate M6 results for the same scenario shapes, so "caps differ by scenario" is a prior belief, not a surprise.

| Exp | Question | Kind / tier | Baselines | Kill / stop |
|---|---|---|---|---|
| **E0** | Cost of the event tap and observation pipeline; do counts match existing counters? | exploratory first; MEASURED_LOOPBACK, release build, hardware stated | tap off vs signals-only vs full | hot-path adder beyond a budget fixed before the run |
| **E4** | Hot-key detector accuracy vs exact counts | exploratory then confirmatory; T1 and microbench | exact counts, per-key EWMA | recall < 0.9 at the stated memory |
| **E1** | Does a load-dependent origin plant exist and behave right? | validity work, SIMULATED | kappa=0 regression, Little's-law check | T1 vs T2 rank disagreement > 25% of paired scenarios |
| **E2** (DRAFT headroom protocol) | Can any per-scenario cap beat one good static cap by >= 3 pp goodput? | dry run EXPLORATORY now; confirmatory needs E1 validity, a new owner salt and a freeze | enumerated static caps, NOCAP, best single, per-scenario oracle, static plus wait-queue | NO_HEADROOM on an E1-validated plant abandons adaptive caps; the advisor continues |
| **E9** | Oracle-warm headroom: is any warming worth building? | exploratory first; SIMULATED | B0 reactive, B1 strongest eviction/admission, B2 prefetch-on-miss, **B3 calendar-literal cron warm (most important)**, B5 yesterday's top-K, B6 random warm of equal budget, B7 oracle | K-H: oracle gain below a pre-registered margin (V2 proposes 10%) abandons predictive warming |
| **E10** | Forecast skill and calibration | SYNTHETIC_FORECAST_HARNESS (SIMULATED) | persistence, seasonal-naive, EWMA, literal declaration | MASE < 1 - margin in >= 5 of 8 held-out regimes, else no forecaster ships; coverage outside [85%, 95%] = UNCALIBRATED (both PROPOSED) |
| **E11** | Calendar value and robustness vs cron warm; bad declarations | confirmatory | B3, B4, B6 | K-C1 to K-C4 |
| **E12** | Does the T1 sign and rank survive a real process? | confirmatory, >= 10 repetitions, release, MEASURED_LOOPBACK | same arms | sign or rank disagreement > 25% of paired cells |
| E5-E8 | eviction selection, origin-pressure forecast, advisor calibration, ML selector | confirmatory | best fixed faithful policy; persistence | selector not better than best fixed within CI; ML only after a non-ML selector passes the five-part ML gate |

Order (P2): E0 and E4 in parallel with E1/E2 (lab only); E9 before any warm actuator; E10 and E11 only if E9 passes; E12 last. The E2 exploratory dry run needs no salt but needs owner approval for lab changes (OD-28, OD-31).

### 8.6 Evaluation principles

1. Strong static baselines first; recompute earlier CAND-1/CAND-4 dispositions (weak baselines) before citing them.
2. Planted-signal designs with a null world (system must do about nothing), a positive control and a harm control; mis-specified and fake declarations are first-class worlds.
3. Anti-circularity: no metric uses the forecaster's output as ground truth; generator, forecaster and decision code share no modules; test on held-out regimes.
4. The seed is the unit of analysis; paired differences, BCa intervals, sign-flip, Holm across scenarios; report per-scenario and worst-scenario results.
5. Objective before data: goodput first, origin queue delay second; success-only latency beside all-request latency.
6. Negative results are kept (H1 NOT_SUPPORTED, `TRADE_OFF_FAILURE`, NO_HEADROOM, K-H) as findings, never tuned away.
7. Tier labels on everything; T1, harness, loopback, WSL2 and physical are never mixed in one number. `DECLARED` may trigger a bounded action but never satisfy or relax a safety check.
8. No performance claim without release build, hardware, OS, workload, seed, baseline, repetitions and intervals.
9. Promotion thresholds in the accountability report 11.6 are PROPOSED (delta_min 5%, BCa lower bound > 0 in >= 5 of 8 scenarios, none adverse, T2 confirmation >= 10 repetitions, overhead in budget, zero hard-cap violations, deterministic replay); the owner confirms them.

### 8.7 RT-0 and novelty

RT-0 (prior-art closure) is **OPEN (PARTIAL)**. The 2026-10-30 sweep ran 21 queries, graded sources OPENED-DIRECT / OPENED-VIA-TOOL / SNIPPET / SECONDARY, and makes **no novelty claim, now or conditionally**. The earlier `novelty_snapshot_m6.md` "RT-0 PASS" is not supported (STATUS). Practitioners already run cron warmers, XFetch-style early refresh, Varnish grace and nginx locks, S3-FIFO/SIEVE/W-TinyLFU and multi-tenant reserved minimums (P2 sections 3, 8). "Uses origin health" cannot be the differentiator. The candidate differentiators are a tenant-scoped declarative calendar with a safety envelope and the evidence discipline; the sweep records coverage gaps, not novelty. A dedicated RT-0 plan needs owner funding (OD-35).

### 8.8 What blocks intelligence

Per-tenant quotas, tenant-attributed telemetry and verified identity on write paths (P3); durable invalidation and the sync lease for anything cross-node (P1); the truthful tap and audit producer (P2 P-1 to P-3); owner approval of lab changes and the v3 salt (E1/E2); a real multi-week trace for seasonality (I13). The scope limit stays in every pitch: the gateway forwards no client identity by default, so pre-warming can fetch only what an anonymous request could fetch (public or shared content).

---

## 9. Security, multi-tenancy and resource-management architecture

Source: P3. This is a design, not a certification. Tests were mostly written by the same project; penetration tests, fuzzing, advisory scans, TLS scans, independent review of all changes and multi-host testing have not been done (M7 review "Not done").

### 9.1 Identity and principals

| Principal | Credential | Roles | Tenant claim | Lifetime | Must not |
|---|---|---|---|---|---|
| Operator | locally minted JWT | `scenario:operate`, `telemetry:read` | none | <= 60 min | appear on a guest device, in a URL, or on disk via the UI |
| Observer | JWT | `telemetry:read` | none (tenant-scoped variant proposed) | 10-15 min | read bodies, invalidate, call control |
| Tenant client | JWT | `cache:read`, optionally `cache:invalidate`; never the `requester` alias | mandatory | 10 min | touch another tenant |
| Cache node | mTLS leaf, SAN `node-{id}.{cluster}.phantom.internal`, pinned fingerprint | n/a | n/a | set at generation | send scopes or rates beyond its capability entry (S6) |
| Guest device | tenant-client or observer JWT via pairing | the narrower of the two | `demo-<session8>-<label>` | 10 min | ever hold operator or admin |

`admin` satisfies every role, so no demo token carries it. HS256 means every verifying gateway can also mint: use a per-session secret for demos; asymmetric signing is an owner decision with a validator change at `jwt.rs:153` [checked].

### 9.2 Quotas, abuse, peer authorization

**Tenant resource management (P3 section 4).** Today only the namespace is isolated; one tenant can evict another's entries and consume its permits. Design: a startup-loaded tenant registry in `closed` mode (unregistered tenant gets 403, which also makes the 10,000-tenant epoch fallback unreachable by clients); **Stage A** accounting hook only (per-tenant counters at the 13 `shard.bytes` mutation sites, a new invariant I9 in `audit_bytes()`, no new lock, hit path untouched); **Stage B** caps at `finalize_fetch` after the fencing checks (quota never overrides fencing) and a tenant-aware wrapper around `ShardEvictionPolicy` (O(1) amortized is not claimed for the wrapper). Honest caveats: epoch-stale resident bytes count toward quotas; caps are soft by at most the pinned bytes; per-shard enforcement can overshoot a global cap (P3 U-6). No fairness, latency-isolation or cluster-wide claim is made.
**Origin share and abuse.** Per-tenant origin permits (reserved and ceiling); `x-cache-bypass` policy (allow, rate_limited, operator_only; it is 1:1 amplification today); per-tenant DELETE rate; token-bucket limiters on source, principal, tenant; header-read and body-read timeouts; connection and SSE stream caps. The threat-coverage table is in P3 section 5.
**Peer authorization (S6).** A valid member can purge every tenant (FullCache), any tenant (Tenant) or any exact key. Exact-key invalidation by any member is inherent to the design and can only be bounded and audited, not prevented. Design: a capability record per member in the membership file (`exact_key`, `tenant_scope`, `full_flush`, `anti_entropy_pull`, rate limits), a receiver check before apply, a typed rejection, a bounded audit without key text. **Open interaction (UNRESOLVED, P1 and P3):** the repair ledger escalates to Tenant then FullCache on overflow; if a peer refuses FullCache the issuer's debt cannot be repaid. P1 leans to keep Tenant command-style with a capability and make FullCache receiver-driven (the issuer sends a typed "resync required", the receiver degrades itself); it needs a joint decision (OD-17) and the multi-node model.
**Open items inherited.** SEC-M7-09 (aliases, bare tokens), -10 (SSRF special ranges), -11 (no auth-failure limiting, no revocation, no `nbf`). "Lockout" maps poorly (no login endpoint): what applies is per-source rate limiting of invalid tokens and a `jti` denylist; IP lockout would lock out users behind one NAT.

### 9.3 Pairing and credentials for demos

Rejected: a shared password (no individual revocation, no attribution, it can mint admin if it is the secret, it leaks, it cannot be scoped). Design (DESIGNED BUT NOT IMPLEMENTED; prototype as an external script first): per-session secret and `aud`; enrollment code of 8 symbols from a 32-symbol alphabet, single use, 120 s, <= 5 attempts per code, <= 20 failures per minute service-wide (arithmetic: about 3.6e-11 chance of hitting a live code under the stated assumptions; arithmetic, not a measurement); **presenter approval per redemption** with a two-digit confirmation on both screens; a 10-minute read-only JWT; every step logged with a truncated hash of `jti`, never the token or code. Bearer tokens are not device-bound: say so on screen. Plain HTTP on a private network needs a permanent LAN-INSECURE banner (OD-22).

### 9.4 Control plane

Operator-only control routes; the Lab API only in a loopback-bound process, with the guest-facing gateway as a separate process with the Lab disabled (no-code interim); an optional separate control listener (code); a hash-chained append-only control audit (never logging tokens or secrets; canary-secret scan); a dangerous-actions list for UI confirmation (start or cancel runs, `NODE_FAILURE`/reset, any tenant or full flush, any revocation). `NODE_FAILURE` is a logical flag (`server.rs:1086-1091` [checked]).

### 9.5 Evidence integrity

**Finding (checked): `src/lab/manifest.rs:103-104` sets `git_tree_hash` to the literal `"456bb25930004f1896a84c53e776e7b3ef29ee16"` (the HEAD *commit* id, not a tree hash) and `is_dirty: true` as constants.** Every manifest built through `RunManifest::new` therefore states the same tree identity whatever code ran; its `git_tree_hash` is UNVERIFIED and is not evidence of the build. Which retained records used this constructor has not been inventoried (P3 U-9). Retained records are not edited; the owner decides how to annotate them (OD-21). Proposal S11: read compile-time stamps and compute the running binary's SHA-256 at run start; `"unknown"` makes a run not VALID; a hash-chained bundle with a verifier written by someone other than the bundle writer; Ed25519 signing via `ring` (P3 U-4: not verified that the calls are exposed). A hash chain proves integrity, not authenticity, until signing and key custody exist.

### 9.6 Demo safety and what a security demo proves

Binding statement: **demonstrations are not certification.** Safe defaults for any shown session: a secure-mode gateway only (JWT secret set; if development mode is shown label it "INSECURE BASELINE" on every screen), per-session secret and `aud`, tokens <= 10 minutes, Lab disabled and control loopback-only on the guest-facing gateway, sandbox origin with an SSRF allowlist, no real data, per-session demo tenants, tokens and codes never shown after use, a teardown checklist, invited attacks only against the sandbox with written scope (OD-25). "Secure mode" in P2-P4 means a JWT secret is configured (`require_observer_if_secure` checks `sec.require_auth_for_control`, `server.rs:337-344` [checked]); it is not a `PHANTOM_MODE` value (Appendix A, R3).

| Demo | Status today | Shows | Does NOT prove |
|---|---|---|---|
| Tenant data isolation (secure mode) with dev-mode contrast | IMPLEMENTED, TESTED | key namespaces, tenant from token | capacity or timing isolation; misconfigured origin |
| Noisy neighbour | needs T-NN | measured interference (expected to show interference today: a legitimate on-stage result) | that quotas fix it |
| Scoped short-lived guest credential and expiry | PROTOTYPE ONLY (script) | guest cannot call control; expired token gets 401 | replay resistance, device binding, revocation |
| Revocation | BLOCKED until S3 | expiry only, labelled | immediate revocation |
| Peer scope authorization | DESIGNED BUT NOT IMPLEMENTED | today the **limitation**, with a fixture member | any insider protection |
| Evidence bundle tamper detection | DESIGNED BUT NOT IMPLEMENTED (S11) | edit, reorder, delete detection against a trusted head | authenticity without an out-of-band key |

---

## 10. Phantom Lab and Demo Center product direction

Source: P4 sections 1 to 3, 6 to 8. Everything is DESIGNED BUT NOT IMPLEMENTED unless stated. P4 ran nothing (no browser, build or test); its screenshot observations are not an independent design review.

### 10.1 Tier vocabulary and rules

Six tiers: **REAL** (produced by the actual backend on actual processes in this session; carries `topology_class`, `build_profile`, `source`), **SIMULATED** (T1 virtual time or a logical flag; carries `sim_kind`), **REPLAYED** (played back from a recorded log; carries original run id, tier and time), **PREDICTED** (a forecast frozen at issue time; carries `prediction_id`, horizon, `model_id`, interval), **DERIVED** (computed server-side from labelled values by a stated rule; carries formula and inputs), **UNAVAILABLE** (not produced; reason code). Adopted modifications (OD-41, Appendix A R1): `INFERRED` stays as a **basis qualifier** on REAL or DERIVED (`basis: observed_by_peer | observed_by_page`); `DECLARED` is an **input-kind** attribute on actions and predictions, not a tier.
Rules (P4's R1-R6, renamed **LR1-LR6** here to avoid collision): **LR1** a topology class on every REAL value, computed from node-reported `host_id` fingerprints, never typed by a presenter (`SINGLE_PROCESS`, `SINGLE_HOST_MULTI_PROCESS`, `MULTI_HOST_LAN`; the last may not appear until a real multi-host run exists); **LR2** a fault layer on every injected fault (`process_kill`, `app_proxy`, `os`, `logical_flag`); **LR3** a `logical_flag` fault is SIMULATED and may not draw a node as failed; **LR4** no client-side computation of a metric; **LR5** no counterfactual line without a named real baseline arm with the same seed; **LR6** a non-removable mode strip `MODE | TOPOLOGY CLASS | BUILD | SEED | STREAM HEALTH | TRACING`.

### 10.2 Events and the `demo-1` stream

Recommended: a **new `demo-1` session stream** with `lab-1` unchanged (it pins five event names and UI fixtures). Reuse the mechanics: single-writer gap-free ids, `events.jsonl` persistence, `Last-Event-ID` resume, fetch-streaming with an Authorization header, 15 s heartbeat, stream cap, close on token expiry (`lab_api.rs:2020-2080` [P4]). New: a session writer merging per-node taps, scoped read tokens, view tickets, per-node taps with cursors, `POST/GET /sessions...`, `/compare`, `/reports`.
**Single envelope (every event):** `schema`, `session_id`, `id` (writer-assigned, gap-free), `src`, `src_seq` (per source), `t_src_wall_ns`, `t_src_mono_ns`, `t_session_us`, `clock{offset_est_ns, offset_err_ns, method}`, `kind`, `tier`, `topology_class`/`build_profile` (REAL), `node_id`, `tenant_fp`/`key_fp`, `client_id`, `trace_id`, `op_id`/`op_gen`, `inv_id`, `cause[]`, `scenario_ref`, `sampled{rate,rule}`, `reason_code`.
**Kinds and payload owners:** `request`, `cache_decision`, `origin_fetch`, `coalescing`, `admission`, `eviction`, `fencing` (core); `invalidation_issue/send/recv/ack/durability`, `node_health`, `membership`, `fault_injected/cleared`, `warm_plan`/`warm_result` (P1); `workload_observation`, `prediction`, `prediction_outcome`, `capacity_estimate`, `action`, `decision_trace` payload (P2); `safety_decision`, `security_decision`, `tenant_quota` (P3/core); `scenario_state`, `marker`, `provenance`, `gap`, `aggregate` (P4). Equivalents exist today only partly: five diagnostic event names (`CacheHit`, `CacheInvalidated`, `CacheStaleServed`, `OriginFetchFailed`, `OriginFetchSuccess`), aggregate counters and response headers. No `miss`, coalescing, admission, eviction, fencing, membership or peer-ack event exists, and there is no request or trace id (P4 grep coverage: `server.rs`, `diagnostics.rs`). `warm_plan`/`warm_result` appear in P4's stage 3 but not in its kind table; I added them with P1 as owner.
**Three data classes:** A "story" (lossless; if its buffer fills the session is marked DEGRADED), B aggregates (fixed rate), C trace samples (sampled deterministically by hash of `trace_id`/`key_fp`, with the rate stated). Class C is off by default in evidence-grade runs and carries a `TRACING ON` label otherwise. **UNRESOLVED:** a sampler with no lock on the hit path. The existing diagnostics ring uses `try_lock` and drops on contention; any per-request emission touches the read-path zero-contention invariant and needs approval and a benchmark (CLAUDE.md 2, 4, 5). No claim is made that the cost is small.
**Ordering and clock:** displayed order is the order observed by the writer; causality only through `cause[]`, `trace_id`, `op_id`, `inv_id`; per-source gap detection; writer-side round-trip clock-offset estimates with error bounds; cross-node durations from one source's timestamps wherever possible. No hybrid logical clock is proposed (P1 agrees). The per-DELETE `inv_id` is issuer-assigned and logged, not carried on the wire (Appendix A, R2).

### 10.3 Information architecture and views

Demo Center `#/demo`: Scenarios (library with a "what is real here" strip); Stage (timeline and lenses: Topology and Traffic; Cache and Invalidation; Decisions; Resources and Tenants); Compare (two to four arms, backend-computed; refuses or labels NOT A CONTROLLED COMPARISON when seed, workload, fault schedule, build hash, topology class, host class or tracing differ); Evidence (existing `#/lab/evidence`, reports, retention); Runs. The operator console (five live screens) stays. The owner's seven "playgrounds" were not available to P4 or to me; they become scenarios and lenses once supplied (UNRESOLVED, OD-50).
Roles: **Presenter** (drive, inject faults with layer labels, mark, reset, re-run, share), **Observer** (inspect, pin, export), **Audience** (read from a distance). Invariant: switching view changes chrome and density only; the set of (metric, value, tier) triples at a session cursor is identical across the three.
Views: **topology** (<= 8 nodes on stage, a table equivalent, edges animate only on recorded events, a killed node is drawn failed only after `node_health` or `fault_injected`, single-node shows "cluster: disabled" with no fake peers); **request and decision trace** (each step an event field; missing steps render "step not observed"; an outcome-only trace card is truthful today); **prediction-versus-actual** (frozen at issue time and never redrawn, observed coverage with N, residual strip, OFFLINE REPLAY / SHADOW / ACTIVE label, visible `input_kind`; **no PREDICTED value may be shown as acting on live traffic before executor integration exists**); **comparison**; **evidence browser and report builder** (byte-identical regeneration; integrity, not authenticity); **reset, replay and re-run** as three different words (a re-run is never labelled a replay of the same result); **pairing** (view ticket; P3 owns the design); **projection mode** (body >= 24 px-equivalent, metrics >= 48 px, <= 4 primary elements, no hover-only content; proposals, untested); a **states grammar** (loading, empty, partial, error, unauthorised, disabled, stale, disconnected, sampled, replayed, rehearsal, simulated, predicted, unavailable; "no data" is never zero).
Observer overhead (P4 2.6; bound is OD-27): paired A/B with tracing off and on, release build, >= 10 interleaved repetitions, paired difference of client p50/p99 and throughput with a 95% interval.

---

## 11. Design-system and UI/UX adoption decisions

Source: P4 section 5. P4 fetched the GitHub API and raw files with `WebFetch` on 2026-10-30; `WebFetch` passes pages through a small summariser, so the facts below are tool summaries, not byte-exact text. Nothing was cloned, installed or executed. I did not re-fetch.

### 11.1 Facts recorded by P4 (reported, not re-verified)

| | `emilkowalski/skills` | `dickwu/apple-design-skill` | `ykswang/apple-hig-skill` |
|---|---|---|---|
| Pinned HEAD (fetched 2026-10-30) | `a81dbca07f2217b7103e510c0f13f8d5b017afc9` (2026-10-28) | `ff4ebddf0488598723a82bfdce1afb04de0b1c44` (2026-10-28) | `9dbbb980f522f0e8b36ed456e632a2741cefce1d` (2026-09-25) |
| Licence | `LICENSE` present, "MIT License", "Copyright (c) 2026 Emil Kowalski" | **no licence file**; README says Apple guideline text belongs to Apple Inc. | **no licence file**; "HIG content (c) Apple Inc." |
| Size | 31 files, 312,269 bytes, 16 skills; includes an unexplained zero-byte file named `.pl` | not enumerated | not enumerated; README claims 173 pages |
| Relevant content | `emil-design-eng` (27,123 B): frequency gate for animation, easing and duration rules, reduced motion; `apple-design` (23,036 B): web-adapted fluid-interface principles | design review against Apple HIG, 123 generated pages | condensed HIG notes as agent context |

Both Emil files contain **agent-behaviour directives** (a fixed "Initial Response" block; a mandatory review-table format). They are treated as data, not instructions, and must be removed from any local copy. P4's fit assessment (INFERENCE): the frequency gate, easing, duration and reduced-motion rules fit a dense operations console; materials with blur, spring "personality" and stagger cascades do not (`docs/design/principles.md` already rejects glass and tweened numbers). The SHAs agree with the 2026-10-29 record in `phase_8_execution_package.md` section 3 (per P4).

### 11.2 O8-1 options and recommendation

| Option | What | Pros | Cons |
|---|---|---|---|
| **A Adopt Emil's MIT material, pinned and filtered, as read-only reference** | `docs/design/reference/emilkowalski-skills/` (not `.claude/skills/`, so nothing auto-loads); byte-for-byte `LICENSE`; `SOURCE.md` with URL, pinned SHA, retrieval date, per-file upstream and local SHA-256, every local modification, reviewer; no `npx skills`, no submodule, no dependency | answers the owner's named resource; MIT; traceable | filtering needed; taste-based rules are not testable |
| **B Public principles only** | Cite Apple HIG and WCAG 2.2 by URL; write PhantomCache's own standards | zero licence surface; no copied text | ignores the named resource; the owner's doubt is not answered |
| **C Decline all three** | Close O8-1 as declined, with reasons | closes the open decision cleanly | as B, and loses the interaction and motion rules |

**RECOMMENDATION (P4, adopted as a proposal):** Option A, minimal: two files (`emil-design-eng/SKILL.md`, `review-animations/STANDARDS.md`); write PhantomCache's own-words standards note citing the source rule; **do not copy** the other two repositories (no licence file; they embed Apple text); no Apple fonts, icons or marketing text. **UNRESOLVED (owner):** whether "apple-design" meant the `apple-design` skill file or the repository as a whole. **No dependency or installation is requested.**

### 11.3 O8-2: who can close it

**RECOMMENDATION:** O8-2 is *not* closed by the agents that built the UI. It closes when the owner approves the design system **and** an independent human design review has reported with no open severity-1 finding. FACT (P4): about 212 UI tests are static CSS/source scans and server-render checks; three headless-browser rounds measured overflow, target size, focus and contrast, and their own reports say PASS means "no failure observed". Neither establishes hierarchy, aesthetics, comprehension or usability. `docs/design/principles.md` line 3 ("pending owner approval O8-2") stays true until then.

### 11.4 Human design review and usability protocol (summary of P4 5.7)

- **Reviewers** (not the authoring agents, and not shown the agents' self-assessment first): one product or visual designer with data-dense operations UI experience; one accessibility specialist (WCAG 2.2 AA, keyboard and a screen reader); one engineer who did not build the UI. The owner chooses the people (OD-44).
- **Materials:** a build at a stated commit; fixture-driven prototypes always bannered "PROTOTYPE - FIXTURE DATA"; a state gallery.
- **Screens (minimum 16):** Overview, Traffic, Origin, Lab landing, Start a run, Run detail (live, stale, disconnected), Evidence Scenario B, Evidence M6, Demo Center Scenarios, Stage presenter, Stage audience, Topology (3 nodes, 1 failed), Decision trace, Prediction-vs-actual, Compare, Report; plus the state gallery and the consolidated Simulation lens.
- **Matrix:** light and dark; 390, 820 and 1280 px (1920 for audience view); keyboard-only; 200% and 400% zoom; forced-colors; reduced motion.
- **Protocol:** independent written rubric before discussion; severities S1 blocker, S2 major, S3 minor; findings logged with screen, width, theme, screenshot and recommended change; written response from the authoring team; re-review of every S1/S2 fix; unedited report under `docs/design/review/<date>/`.
- **Pass criteria (proposals):** no open S1; S2 count under an owner-set number; every screen in both themes at all three widths; the accessibility specialist reports no WCAG 2.2 AA failure that blocks a task; every tier badge judged legible by all reviewers.
- **Usability study (formative, not statistical):** 5-8 participants (at least 3 engineers who have not seen the project, at least 2 non-technical audience members; recruitment and consent per owner, O8-4); tasks T1 "is this number live, replayed or simulated?", T2 why did a request miss, T3 how long did invalidation take to reach node-3 and how sure is the screen, T4 the cost of the cap in Scenario B, T5 start a run and explain what will be sent, T6 failed node vs logical flag, T7 read the headline in audience view, T8 recover from a disconnected stream. Report counts, not percentages with intervals. It cannot show statistical usability, long-session fatigue or real incident conditions.

### 11.5 Design-system contents and the Simulation screen

Target (P4 5.4): type tiers (display, metric, title, section, body, label, caption, mono; tabular numerals; system fonts); an applied elevation scale (the `--shadow-1/2` tokens are defined at `tokens.css:39-40` and reportedly unused); motion standards from the frequency gate plus the existing "live values never animate"; one state grammar; form patterns; six-tier provenance badges with icon shape plus word; chart primitives each with a table alternative (hand-rolled SVG/CSS; any library or visual-regression tool is owner decision O8-3 and none is requested; any palette must be validated in both themes); theme control; a status strip in place of stacked banners; a Markdown notice renderer (no literal `**`). The legacy Simulation screen (`SimDemo.jsx`, 517 lines, own `EventSource` at `:92`, POST to `/api/control` at `:177` [checked]) is a different product inside the same shell. Interim: re-home it under shared tokens, relabel `KILL NODE` as "Set logical flag (no process stopped)", remove dollar and "Phantom OFF" counterfactual tiles unless parameters are visible and labelled MODEL. Target: delete it and fold its content into a Simulation lens on the Demo Center (OD-45). Acceptance: one type scale and one component set; a source-guard test failing on font sizes under 12 px and raw hex colours; no tile without a tier badge; no client-side counterfactual.

---

## 12. Dependency-aware implementation roadmap (merged DAG)

**Conventions.** Item IDs carry the workstream: `W1-` = P1 (S-numbers from P1 section 9), `W2-` = P2 (P-numbers, section 7), `W3-` = P3 (S-numbers, section 9), `W4-` = P4 (proposal letters A-K in section 6, stage numbers S0-S12 in section 7 written `ST`). Stages 0 to 4 are the owner's. **They are not authorization to rewrite M1-M8**; section 14 maps them to milestones as a proposal only.
**Touch classes** (what approval an item needs, CLAUDE.md sections 1, 2, 5): **DOC** docs under `docs/research/` (no approval needed); **DOC+** docs elsewhere (README, STATUS, ADR, user docs; owner decision first); **TEST** test or harness code (treated as needing approval; P1 S0.4, P3 D-TEST); **TOOL** scripts or standalone tools outside `src/`; **APP** application code under `src/` (approval + explanation first); **INV** APP that touches fencing, epochs, invalidation or byte accounting (additionally: independent review pass and a test that fails without the change); **FRZ** touches or sits next to frozen records or the frozen M6 lab files (owner-reserved; P3 notes the frozen-file list is itself undecided, OD-23); **DEP** new dependency or install; **UI** `ui/`; **SYS** system or global setting. No effort or date estimates are given; none of the drafts has a basis for them (P1 says so explicitly).

### 12.1 Stage content and the DAG

```
OWNER DECISIONS ─► Z1 contract ratify ─┐
 (OD-1,2,5,6,8,9,14)                    ├─► Z4 independent review commission ─┐
 Z2 protocol model ───────────────┐      │                                     ▼
 Z3 harness kill/restart ─────────┴──────┴──► [S1.1 durable log + drain] ─► [S1.2 heartbeat + incarnation + sync advert] ─► [S1.5 cluster observability]
                                                         │                         │                                          │
 Z6 minter/PKI-script/bundle layout ─► W3-S3 auth hardening ─► W3-S5/W1-S1.4 PKI + runtime membership + revocation ─► W3-S6/W1-S1.3 peer capabilities ─┤
 Z7 falsification probes (before-state) ─► W3-S7 tenant accounting (I9) ─► W3-S8 quotas + fair eviction ─► W3-S9 origin share/bypass                        ▼
 W2-P-1 tap + E0 ─► W2-P-2 observe ─► W2-P-3 audit producer ─► W2-P-4 validator ─► W2-P-5 warm actuator ─► W2-P-6 calendar        [W1-S1.8 Level B physical LAN gate]
        ▲                                       (E4, E9 before P-5; E10/E11 before forecasting)                                          │
 W4-ST4 demo-1 writer (A,B) ─► W4-ST5 client traces ─► W4-ST6 multi-gateway + topology ─► W4-ST7 gateway taps ─► W4-ST8 flash crowd + invalidation stages
                                                                                                                         │
 W1-S2.1 placement ─► W1-S2.2 fill protocol ─► W1-S2.3 shadow peer-fill (needs Z2 fill model, S1.1, S1.2, independent review) ─► flagged peer-fill ─► W1-S3.1/S3.2 Level D
 W4-ST9 intelligence lenses (needs W2 events) ─► W4-ST10 compare/report ─► W4-ST11 pairing + physical run ─► W4-ST12 design review round 2 + usability ─► O8-2 decision
```

### 12.2 Proposals table (every significant proposal)

Columns: **Why** necessary; **Builds on**; **Depends on**; **Tested / completion evidence**; **Fails if**; **Touch**. Detail lives in the cited draft section; this table does not repeat it.

**Stage 0: Truth and contracts (documents, tests-only, owner decisions)**

| ID | Proposal | Why | Builds on | Depends on | Tested / completion evidence | Fails if | Touch |
|---|---|---|---|---|---|---|---|
| Z1 = W1-S0.1/S0.2 | Replace ADR-003 contract text with numbered clauses (section 7); correct overstated wording (G2/G3, "50 ms", "Node restarts: supported") in user docs by dated notice | A 204 is read as cluster-wide removal; retracted claims still circulate | ADR-003 amendment, claims ledger, `limitations.md` | OD-2 | Each clause has a falsifier and status; dated owner ratification recorded | Owner prefers other wording; statuses change after tests; temptation to edit frozen records (add notices instead) | DOC / DOC+ |
| Z2 = W1-S0.3 | Multi-node protocol model (state machine or extended enumerator) for invalidation, issuer crash, later fill | Only a single-node, weak model exists | `tests/integration/protocol_model_tests.rs` style, seeded mutations | none | Model passes; each seeded mutation fails | Too abstract to matter; state explosion | TEST (new file) |
| Z3 = W1-S0.4 | Harness kill, restart, graceful-stop helpers and the five M3 tests (`proposed_patch_1_harness_tests.diff`) | No crash claim is testable without them | `tests/support/cluster_harness.rs` (`kill_on_drop` only today) | OD-8 | Whole cluster suite green plus K-matrix harness usable | `CREATE_NEW_PROCESS_GROUP` disables Ctrl-C delivery; detached runners fall back to GRACEFUL_STOP_NOT_FEASIBLE | TEST (+ Cargo test registration) |
| Z4 = W1-S0.5 | Commission the independent review of the durable-log patch (checklist P1 6.2) | CLAUDE.md section 5; none exists | P1 6.2-6.4, K1-K20 | OD-6 | Written review by someone who is not the author, given the code not P1's conclusions | Reviewer anchored on this file | DOC |
| Z5 = W1-S0.6 | Decide four cheap hooks: placement hash, TTL ceiling, tenant accounting hook, wire version | Cheap now, expensive after features land (A2) | `ring` SHA-256 already used | OD-9, 11, 14, 19 | Recorded decisions | Hooks conflict with P3 design | DOC |
| Z6 = W3 Stage 0 | Minter script (S1), per-session profile, two-process layout (Lab on loopback only), evidence-bundle layout, read-only inventory of which retained manifests used the hard-coded constructor | Scoped credentials and honest provenance are prerequisites for any demo | `JwtValidator::create_token` (`jwt.rs:135`) or `node:crypto`/PowerShell HMAC | none for the inventory; OD-8 for new files | Recorded run with canary-secret scan clean; token with extra claims validates | Secret leaks via shell history or logs | TOOL / DOC |
| Z7 = W3 Stage 1 | Falsification probes against the **current** tree: T-NN, T-BF, T-AF, T-DF, SSE stream count, slow-send, denied-read counting; harness extended for JWT plus cluster | Establishes the recorded "before" every later improvement is measured against | `ClusterHarness`, `lab_live` helpers | OD-8 | Loopback, release, seeded, repeated, thresholds pre-registered | Fixtures make attacks trivially fail; results quoted without labels | TEST |
| Z8 = W4-ST0/ST1 | Close O8-1/O8-2 definitions; independent human review of the 16 existing screens and the state gallery; usability T1 on today's labels | The UI has never had a human review | section 11.4 | OD-38, 39, 44 | Review report with unedited reviewer text | Reviewers disagree; polish displaces backend work | DOC (read-only) |
| Z9 = W2 RT-0 plan | Fixed venues, years, recorded queries, full reads of the closest candidates | RT-0 is OPEN; no positioning claim without it | RT-0 sweep | OD-35 | A plan; a coverage record | Search tooling is US-only and indexes blogs better than proceedings | DOC |
| Z10 | Proposed STATUS.md structure and README text (section 14) | Reporting must not lead with single-node strength | STATUS | OD-4 | Owner-approved wording | Positioning change without owner decision | DOC+ |

**Stage 1: Reliable distributed foundation (Level B) with its security and observability preconditions**

| ID | Proposal | Why | Builds on | Depends on | Tested / completion evidence | Fails if | Touch |
|---|---|---|---|---|---|---|---|
| W1-S1.1 | Integrate the durable owed-invalidation log and shutdown drain, plus P1 6.4 changes (detached cluster leg, age-out safety, fsync-error rotation) | Closes the reproduced issuer-crash defect | `RepairLedger`, `m3_durability/proposed_patch_2_src.diff` | Z3, Z4, OD-5, OD-7 | K1-K20; unit tests; mutation test for the stamp/watermark logic; full suite and clippy; independent review passed; Windows and Linux | Fail-open masks problems under load; fsync semantics; replay amplification | APP + **INV** (`router.rs`, `main.rs`, `config.rs`, `lib.rs`, new `durable_log.rs`, `server.rs` for the detached leg) |
| W1-S1.2 | Heartbeat-driven peer state with incarnation id and issued-sequence advert | Detect dead peers when idle; detect issuer restart and lost stream tails | `pool.rs` heartbeat, `AntiEntropyTracker`, `NodeState` | OD-11, Z2 | Fault-proxy tests (kill, blackhole, restart, flap); false-positive study | Flapping demotes healthy nodes; mixed-version rollout (v1 peers drop the connection on unknown types) | APP (`frame.rs`, `pool.rs`, `router.rs`, `node.rs`) |
| W1-S1.3 + W3-S6 | Peer capability and rate limit per member for Tenant/FullCache scope; typed rejection; bounded audit | A compromised member can flush the cluster | `MemberNode`, `validate_peer`, `RejectedRateLimit` | OD-17 (ledger escalation), P3 design | Fixture member sends each scope and exceeds each rate; tests fail without the change | Legitimate escalation refused so debts never clear | APP (security) + **INV-adjacent** |
| W1-S1.4 + W3-S5 | Runtime membership (authenticated admin update, swappable node list, tolerant epoch window) with signed versioned membership document and revocation | Revocation and joins must not need cluster-wide restarts | `registry.update()`, revocation channel, `evict_peer` | W3-S3 (who may call it), OD-12, OD-15 | Revoke under traffic: next frame refused and sessions die; rollback of version refused | Registry races; split membership views; keys left on the generator | APP + DEP decision |
| W1-S1.5 | Cluster observability (per-peer state with provenance, ledger depth, gaps, last-ack age, propagation histogram, incarnation, drain) | You cannot operate or demonstrate what you cannot see | in-process getters (`pending_count`, `has_gaps_for`) | W1-S1.2 for meaningful states | Schema and honesty tests: inferred values never shown as measured | Synthetic values leaking in as live | APP + UI |
| W1-S1.6 | Shipped cluster PKI and members-file generator | Only test code can create node certs today | `rcgen` (dev), `openssl` script | OD-12 | Generated material passes `validate_peer`; a clean machine forms a 3-node cluster from docs | New runtime dependency approval; key handling | TOOL / DEP |
| W1-S1.7 | Gateway TTL ceiling and wiring of `PHANTOM_DEFAULT_TTL_MS` | Makes clause C8 and the 24 h statement true | `PHANTOM_MAX_STALE_MS`, `freshness.rs:263-276` | OD-14 | Origin sends huge `max-age`; clamp asserted; test fails without the change | Breaks origins relying on long lifetimes | APP (`core/freshness.rs`, config) **INV-adjacent** |
| W1-S1.9 | Opt-in partition policy (lease-clamp or bypass) | Bounded staleness under isolation | W1-S1.2 heartbeats | W1-S1.2, Z2, OD-10 | Partition test: staleness vs lease length, measured bound with interval | False positives; hit-ratio loss | APP (read-path hook; fencing-adjacent) |
| W1-S1.8 | **Level B evidence gate**: release-profile physical LAN study | Level B cannot be claimed otherwise | harness, fault proxy | W1-S1.1 to S1.5, S1.6; devices; OD-13, OD-24 | Section 13 / P1 section 8: >= 3 devices, Windows and Linux, intervals, labelled LAN-PHYSICAL | Hardware unavailable (BLOCKED); results worse than D_max | TOOL (+ SYS for firewall) |
| W3-S3 | Gateway auth hardening: max token lifetime, `jti` denylist with operator-only revoke, close `/api/events` at `exp`, 401/403 counters by reason, per-source invalid-token limiter, strict-roles flag | SEC-M7-09 and -11; the only way to revoke one device | `authenticate_request`, Lab stream expiry check | OD-18 | Each item has a test that fails without it; existing auth matrix green | Denylist memory growth; per-node-only revocation; breaking clients that use aliases | APP (`jwt.rs`, `server.rs`, `config.rs`, `main.rs`) |
| W3-S9 (part) | HTTP hardening: header/body-read timeouts, connection cap, SSE stream cap | No limits found in `server.rs` | | OD-18 | T-AF, slow-send, stream-count probes | Legitimate bursts rejected | APP |
| W3-S11 | Build identity: stop hard-coding `git_tree_hash`; stamp source digest, dirty flag, running-binary SHA-256; hash-chained bundle + verifier by a different author | Evidence must say which code ran | `manifest.rs:157-164`, `packaging/build-release.*` | OD-21 | Tampering corpus: flip, delete, reorder, truncate (truncation caught only with an anchored head), swap manifest | Embedded key proves only consistency | APP/TOOL + **FRZ-adjacent** (`src/lab/manifest.rs`) |
| W3-S7 | Tenant registry (`closed` mode) and accounting hook (Stage A, invariant I9) | Prerequisite for any quota, fair eviction, tenant telemetry | `epoch_snapshot` read-mostly pattern, `audit_bytes` | OD-19, Z7 (before-state) | Property test over random ops; test omitting one of 13 sites fails; release micro-benchmark with interval | Missed mutation site; counter drift on panic; hit-path regression | APP + **INV** (`cache.rs`, `config.rs`, `server.rs`, telemetry, `main.rs`) |
| W2-P-1 | Bounded drop-newest event tap and E0 measurement | Nothing downstream is trustworthy without truthful observation; hot path must not block | `intelligence_architecture.md` section 3; `key_hash` at `server.rs:1261` | OD-26, OD-27 | Release overhead benchmark off/signals/full with intervals; consumer-stall fault injection | Adder too large; ring overruns under bursts | APP (request path) **read-path invariant** |
| W2-P-2 | Observe-mode consumer: truthful `hot_keys`, regime, burst | Replaces `unavailable/not_implemented` | `WorkloadPipeline` (`signals.rs`) | W2-P-1 | E4 recall vs exact counts; tenant-scoping tests | Sketch error; hot-key flooding; cross-tenant leakage | APP |
| W2-P-3 | Audit producer and extended `DecisionRecord` (refusal reason, realized effect, interval method) | `/audit` serves an empty store | `audit.rs`, `mod.rs:149` | W2-P-2 or lab producer | Round-trip tests; deterministic replay gives identical records | Store capacity 100 loses history | APP |
| W4-ST4 (A) | `demo-1` session writer and envelope, classes A and B | Nothing streams per-event data across nodes with resume | Lab `RunHandle`, `Tail`, `Last-Event-ID` | OD-40, P3 token scopes | Gap-free ids; one `gap` event per dropped range; no duplicates on resume | Writer becomes a single point of failure; stream cap (16) reached | APP (new module; not `lab-1`) |
| W4-ST5 (E, part) | Outcome-only trace cards from loadgen and headers | Truthful today; internals labelled UNAVAILABLE | `ClientRecord`, `X-Cache-Status` | W4-ST4 | Cards labelled "internals UNAVAILABLE" | Mistaken for a full trace | UI |

**Stage 2: Distributed caching capabilities (Level C), behind its evidence gate**

| ID | Proposal | Why | Builds on | Depends on | Tested / completion evidence | Fails if | Touch |
|---|---|---|---|---|---|---|---|
| W1-S2.1 | Stable placement: fixed hash (SHA-256 truncation via existing `ring`), owners from the membership epoch, not from health | Prerequisite for any owner-based read | `route_key`, registry epoch | OD-9, Z5 | Same owners on Windows and Linux builds; 1/N movement property test | Hash quality; ties | APP (`router.rs`) |
| W1-S2.2 | `FillRequest/Response` protocol replacing the stub (delete the fabricating branch) | The current handler fabricates data and stores it | framing, pool, mTLS | W1-S2.1, W1-S1.2, OD-11 | Codec tests; tenant isolation; hop limit; fabricated-body negative tests | Protocol bugs leak tenants | APP (`frame.rs`, `node.rs`, `router.rs`) + **INV** |
| W1-S2.3 | Shadow-mode peer-fill, then flagged peer-fill with owner single-flight and CC1-CC6 | The structural fix for N-fold origin load | S2.1, S2.2, per-node fencing | Z2 (fill model), S1.1, S1.2, independent review, OD-1 | Hazard tests H1-H5; dead and lagging owner; epoch mismatch; **evidence gate 7.2** | Lagging-owner resurrection; fill storms; owner hotspot | APP (GET miss path) + **INV** |
| W1-S2.4 | Cluster origin budget R1 (documented per-node share), then R3 (owner-concentrated origin access) | Honest origin protection at N > 1 | `PHANTOM_ORIGIN_CAP_*` | R3 needs S2.3 | Origin load vs N under a fixed workload | Skewed traffic starves some nodes | DOC/config; APP for R3 |
| W3-S8 + W1-S2.5 | Per-tenant caps and fair eviction (tenant-aware `ShardEvictionPolicy` wrapper); cluster allocation semantics | Fairness, noisy-neighbour control | W3-S7 | W3-S7, independent review, release benchmark | T-NN, T-QX, T-EP, T-EE, T-INV | Wrapper cost under the shard lock; per-shard overshoot; dead-byte accounting | APP + **INV** (`cache.rs`, `eviction.rs`) |
| W3-S9 (rest) | Origin share per tenant, bypass policy, DELETE rate | SEC-M7-11 family and amplification | origin permit gate | W3-S7 | T-BF, T-AF, T-DF | Limiter state per node only | APP |
| W3-S10 | Tenant-scoped telemetry tiers (A tenant-self, B observer aggregate, C operator, D lab) | Observer-wide metrics are cross-tenant today | | W3-S7, W3-S3 | Tier tests; no leakage | Side channel through own-eviction counts | APP |
| W4-ST6 (H) | Multi-gateway run kind and topology lens | The integrating demonstration needs three real processes | `lab_live` spawn/kill, scripted origin | W1-S1.4/S1.6 PKI, W1-S1.5, OD-12, OD-41 | Failed-node vs logical-flag test; rehearsal logs | PKI provisioning; Windows-only supervision | APP (`src/lab_live`) / TEST |
| W4-ST7 (E, K) | Gateway event taps (class C): cache_decision, coalescing, admission, fencing, invalidation legs; observer-overhead measurement | "Show why" needs emitted facts | W2-P-1 shared tap | W2-P-1, OD-27, OD-26 | Overhead protocol with intervals; every trace step maps to a named field | Contends with the read path; sampling hides the rare case | APP **read-path invariant** |
| W4-ST8 | Flash-crowd loadgen (rate step, key groups, tenant labels), per-key origin delay, fencing mutation test | Stages 3-6 of the flagship need it | `lab_live/loadgen.rs` | ST6, ST7 | Cross-check: origin fetches seen by the scripted origin equal the sum of `origin_fetch` events | Scripted origin cannot do per-key delay (unverified) | TEST / APP (lab_live) |

**Stage 3: Intelligence and adaptation**

| ID | Proposal | Why | Builds on | Depends on | Tested / completion evidence | Fails if | Touch |
|---|---|---|---|---|---|---|---|
| W2-P-7 | Load-dependent origin plant (E1) and E2 arms in **new modules** | No cap analysis is meaningful without a plant where load matters | `origin_sim.rs`, `t1_sim.rs` | OD-28, OD-29 | V-E1-0..5 validity checks; T1/T2 rank agreement | The contention law is an assumption | APP (`src/lab`) **FRZ-adjacent** |
| W2-P-8 | Demand-World generators and a T1 warm arm (E9) | Cheapest way to learn whether warming has headroom | `WorkloadGenerator`, `leader_dispatch` | W2-P-7 columns | Null, positive, harm controls; determinism re-run | Generator-shape contamination | APP (`src/lab/demand/`) |
| W2-P-9 | Shadow evaluator fixes: paired block-bootstrap CI, TTL/size/invalidation aware | Current CI overstates significance | `shadow.rs` | none | Synthetic data with known effect: false-commit and false-rollback rates | Less power | APP |
| W2-P-4 | Safety/policy validator (L6) with action interface A1-A7 | Makes "intelligence cannot bypass correctness" structural | `PolicyExecutor` trait, clamps | OD-26, W3-S7 (verified tenant identity, quotas) | Per-refusal negative tests; property test: no accepted action exceeds a budget | Validator bugs become the single point of failure | APP (new module) |
| W2-P-10 | Outcome ledger, calibration monitor, regret governor | Honest demotion; needed before any actuator | audit store | W2-P-3, W2-P-4 | Planted-signal worlds; no false demotion in the null world | Too-sensitive governor disables good actions | APP |
| W2-P-5 | Warm actuator (A1) and warm-only origin-gate class | I3, I4, I10 need a way to fill; a warm must lose to an organic miss | normal fill path, `finalize_fetch` fencing | W2-P-4, W2-P-1, **E9 not killed**, independent review | Fencing race tests (invalidate during warm); noisy-neighbour; gate priority tests | Fill path assumes a client request; amplification abuse | APP + **INV-adjacent** |
| W2-P-6 | Demand Calendar store and deterministic scheduler | Declared demand removes the need to predict | V2 section 2 schema | W2-P-4, W2-P-5, W3-S3 (role-gated writes), OD-32 | Schema abuse tests; E9 then E11 then E12 | Restart loses entries; unauthenticated calendar is an amplification endpoint | APP |
| W2-P-12 | Add `Shadow` to the mode enum; document or rename `canary` | The ladder needs it; today `canary` is mislabelled | `mod.rs:22-36`, `config.rs:161` | OD-36 | Config and mode tests | Breaks existing configs | APP |
| W2-P-11, P-14 | Advisor (Little's-law sizing); hot-key refresh-ahead | Value even if no controller beats a static cap | E1 plant; background refresh slots | W2-P-7, W2-P-2, W2-P-4 | E7 calibration; T1 hot-key worlds vs XFetch | Simulator fidelity unvalidated | APP |
| W1-S3.1 / S3.2 + W2-P-13 | Cluster-aware observation (tenant-scoped aggregates), then distributed hot-key/flash-crowd detection, coordinated pre-warm, placement/admission, cross-node optimisation | The Level D vision | S2.3, S1.5, P2 single-node gate | Single-node gate passed (E4, not-killed E9, E11, E12 on one node), durable invalidation (S1.1), sync lease (S1.2), P3 privacy tiers | Offline, shadow, canary, active ladder; baseline: strongest static plus Level C | Prefetch waste; stale exposure; benefit unproven even single-node | APP |
| W4-ST9 (F) | Intelligence lenses: decision trace, prediction-vs-actual, capacity | Shows the central claim and any miss | | W2 events | Coverage and a documented miss | No forecaster passes the gate (OFFLINE REPLAY only is a valid outcome) | UI |

**Stage 4: Product excellence (runs alongside; early review, late review)**

| ID | Proposal | Why | Builds on | Depends on | Tested / completion evidence | Fails if | Touch |
|---|---|---|---|---|---|---|---|
| W4-ST2 (J) | Design-system foundations; Simulation lens step 1 | Coherence, accessibility | `tokens.css`, `app.css`, `lab.css` | OD-38, OD-39, Z8 findings | Extended source guards; screenshots reviewed | Polish displaces backend work | UI |
| W4-ST3 | Demo Center shell over the existing Lab; mode strip; REPLAY of retained Scenario B; "the thing we got wrong" page | The tier honesty must exist before data does | existing `#/lab` | W4-ST2 | Real-browser rounds 390/820/1280/1920 light and dark | | UI |
| W4-ST10 (G) | Compare, report builder, evidence retention | A defensible comparison and a durable record | `paired_vs_base`, evidence viewer | ST8 | Refuses mismatched binary hash; byte-identical regeneration | Hosts drift | APP + UI |
| W3-S2 + W4-ST11 (I) | Pairing service, scoped tickets, audience view, physical multi-device run; creation of the `MULTI_HOST_LAN` label | Multi-device demonstrations | minter, `aud` per session | W3-S1/S3, OD-22, OD-43, OD-41 | Tickets expire and never grant Operator; second device receives identical event ids; P3 sign-off | Non-loopback exposure unacceptable | TOOL + APP + UI |
| W3-S12 | Falsification suite and security evidence runner; third-party scans (OD-25) | Tests emit pass/fail but no artifact | S11 format | W3-S11 | Each scenario >= 5 runs with seeds | Same-author blind spots | TEST/TOOL |
| W4-ST12 | Independent design review round 2 and usability study on the built Demo Center | O8-2 | | ST3-ST11 | Signed review, study notes, O8-2 decision | | DOC |

### 12.3 Critical path and parallel work

**Distributed critical path (INFERENCE from the dependencies above):** owner decisions OD-2, 5, 6, 8 -> Z3 (harness) and Z4 (independent review commissioned) -> **W1-S1.1** (durable log and drain) -> **W1-S1.2** (heartbeat, incarnation, issued-sequence advert) -> **W1-S1.5** (cluster observability) -> **W1-S1.8** (Level B physical LAN gate). Level C (W1-S2.x) then follows through Z2 (fill model), S2.1, S2.2, S2.3; Level D waits behind S2.3 and the intelligence single-node gate.
**Intelligence critical path:** OD-26 -> W2-P-1/E0 -> W2-P-2 -> W2-P-3 -> W2-P-4 -> (E9 not killed) -> W2-P-5 -> W2-P-6. W2-P-4 also needs verified tenant identity and quotas from W3-S7.
**Demo critical path to the full flagship (section 13):** W4-ST4 -> W4-ST6 (needs PKI W1-S1.6, W1-S1.5) -> W4-ST7 (needs W2-P-1) -> W4-ST8 -> W4-ST9 (needs W2 events) -> W4-ST10 -> W4-ST11 -> W4-ST12. **The merged longest chain is the flagship chain, and it contains W1-S1.1/S1.2/S1.5 as well as W2-P-1/P-4/P-5.** The honest interim flagship (section 13) can be built earlier from items in Stages 0 and 1.
**Run in parallel (no mutual dependency):** Z1, Z2, Z3, Z6, Z7, Z8, Z9 (all documents, tests or tools); E4 and E1/E2 lab work with W2-P-1; W3-S3/S9 with W1-S1.1; W4-ST2 with W4-ST4; W4-ST6 prototype with W4-ST5. W3-S7 should wait for Z7 so the before-state is recorded.
**Blocked on owner decisions (cannot start):** everything marked APP/INV (needs approval and explanation first); W1-S1.1 (OD-5, 6, 7); W1-S1.7 (OD-14); W1-S1.6 (OD-12); W3-S7 (OD-19); E1/E2 confirmatory (OD-28, 29, 30); Level C committed vs gated (OD-1); physical devices and firewall (OD-13, 24); design resources (OD-38).
**Touch frozen files or invariant-heavy application code:** the INV items (W1-S1.1, S2.2, S2.3, W3-S7, W3-S8, W2-P-5, W1-S1.7), the read-path items (W2-P-1, W4-ST7), and the FRZ-adjacent items (W2-P-7, W3-S11). The frozen-file list is itself undecided (OD-23); until the owner decides, treat `src/lab/manifest.rs`, the M6 evaluation files, preregistrations and sealed results as owner-reserved.

---

## 13. Flagship demonstration specification: "Launch Day on three nodes" (merged)

Merged from P4 section 4 (stage script and interim versions) with P1, P2, P3 mechanisms. **The full version does not exist today.** The honest rule: a stage whose capability is not built renders an UNAVAILABLE panel naming the missing capability and the owning pillar; no canned success panels.

**Cast.** Nodes `node-1..3` (three real gateway processes with authenticated peers); clients A (tenant `alpha`), B (`beta`), C (flash-crowd generator, `alpha`), each with its own scoped token; a scripted origin (a real separate process; its faults are schedules, labelled "scripted"); a controller/validator; the presenter's Control Center. **Today's constraints:** a Lab run spawns one gateway per arm (`gateway.rs:20-28` [checked]); cluster mode needs mTLS material, a members file, listen and peer addresses (`main.rs:281-383`); GET is served from the local cache only, so "cache behaviour across nodes" means each node's own cache, with clients routed by URL; invalidation durability is bounded to the issuer staying alive; the PIE engine is not in the serving path.

### 13.1 Stage table

| Stage | Full mechanism and pillars | Runnable on current code (truthful interim) | Label and topology | Proves | Does NOT prove |
|---|---|---|---|---|---|
| **0 Preflight** | Capability checks, host ids, clock probes (W4) | Connection panel + `GET /capabilities`; manual checklist. A debug build brands the session EXPLORATORY | REAL; topology class computed | The environment is what the strip says | Anything about performance |
| **1 Three authenticated nodes and clients** | Session service spawns `node-1..3` with mTLS (W1-S1.6, W3-S5), issues scoped tokens (W3-S1) | **One node only**; show "cluster: disabled". Do not draw three nodes unless three processes report themselves | REAL; `SINGLE_HOST_MULTI_PROCESS` (or `MULTI_HOST_LAN` after a real run) | Peers authenticated each other (if the handshake event is real); three processes exist | Production grade; three hosts |
| **2 Baseline load and per-node cache behaviour** | Zipf traffic per tenant to different nodes (client picks the node by URL) | Scenario B warmup/healthy phases on one real gateway; counters REAL; windowed ratios UNAVAILABLE | REAL counters; hit ratio per window DERIVED once server-side | Baseline numbers for later comparison | Same-key consistency across nodes (each node has its own copy) |
| **3 Known future demand (declared), warm** | Calendar -> warm plan (W2-P-6) -> per-node warm within a share of a cluster origin budget (W1 R1); cross-node coordination only after S1.1/S1.2; peer-fill is Level C | Out-of-process script warms keys by ordinary GETs before the surge; labelled DECLARED / EXTERNAL SCRIPT; origin load measured with and without it, two real runs, same seed | REAL; `input_kind=declared`; **no PREDICTED label** (nothing was forecast) | A declared warm reduces origin fetches during the surge, if the paired measurement shows it | That the system predicts anything; cross-node coordination (see Appendix A R13) |
| **4 Unexpected flash crowd; detection; capacity; response; limits** | Steps 1-10 of section 2.3: rate-step load, observation, detection (DERIVED "detected, not predicted" unless a forecast exists), capacity estimate, validated warm, `safety_decision` always visible | Scripted burst of concurrent requests for a few cold keys: N concurrent misses become one origin fetch, shown by the origin's own request log against client count; "origin requests avoided" DERIVED with the formula displayed; origin cap and shed from real counters. No detection or capacity panel (UNAVAILABLE). The `FLASH_CROWD` button is **not** used | REAL for mechanics; scripted degradation labelled | Coalescing and cap mechanics on a real gateway | Adaptivity (Scenario B notice: adaptive-control merit not established) |
| **5 Prediction vs actual; comparison vs static policies** | Same seed run as STATIC, DECLARED-WARM, ADAPTIVE on release binaries; Compare view with N, paired differences, intervals, zero line | Retained Scenario B 10-repetition result vs a static cap, `TRADE_OFF_FAILURE` read out cost-first; prediction panel says "No predictor in the serving path", with the retained M6 `NOT_SUPPORTED` snapshot labelled REPLAYED | arms REAL; differences DERIVED; forecasts PREDICTED only if they exist | Measured benefit **or documented failure** for this workload, seed, build and host class | Generalisation, production performance, any cross-tier "x faster" |
| **6 Invalidation propagation under load; fencing** | DELETE for a hot key while fetches are in flight; fencing discards the late response; issuer fans out; per-peer legs and DERIVED propagation time with error bound (W1-S1.5) | Scripted race on one gateway (needs a per-key origin delay, UNVERIFIED), check by GET that nothing stale was stored, `stale_writes_prevented` before and after; screen carries "a 204 on DELETE is not proof the object is gone everywhere" | REAL; single node: propagation panel UNAVAILABLE | Fencing prevents overwrite in the scripted race (needs a mutation test that fails when fencing is disabled) | Cluster-wide delivery bounds |
| **7 Node failure and recovery** | `process_kill` fault, Suspect, owed invalidations, restart, repair and anti-entropy | Kill one of two hand-started gateways: clients of the other are unaffected (reads are local); the killed node's clients lose service. **Do not claim failover** | REAL if a real process kill; a logical flag is SIMULATED (LR3) | What a node loss does and does not affect | Automatic failover |
| **8 Issuer failure and durable recovery** | Kill the issuer before peers receive an invalidation; with the durable log the restarted issuer completes delivery (W1-S1.1) | Run the reproduction and **show the defect as a known limit**, labelled "KNOWN LIMITATION - durable log not implemented". If it accidentally does not reproduce, report "not reproduced this time" | REAL | The limit exists. After W1-S1.1: durable recovery under failure model F | Correctness under arbitrary partitions |
| **9 Security demos** | Forged `x-tenant-id` ignored; tenant-scoped DELETE; stream token scoped to `beta` cannot read `alpha`; node without a certificate rejected; expired token closes the stream | (a) forged header and (b) tenant-scoped DELETE only; the rest UNAVAILABLE (needs W3-S3, S5, S10) | REAL | Specific checks behave as described under one fixture | Security certification (section 9.6) |
| **10 Consistency evidence and report** | Session report from per-stage bundles; a consistency check reads each node's final state | Per-stage artifact bundle with `SHA256SUMS`, "integrity only, not authenticity" | REAL; consistency DERIVED with timing | What was recorded | Anything outside the recorded failure model |

### 13.2 Single machine versus multi-device

| | Single machine | Physical multi-device |
|---|---|---|
| Topology class | `SINGLE_HOST_MULTI_PROCESS` | `MULTI_HOST_LAN` (never run in the repository; label does not exist yet, OD-41) |
| Use | development, rehearsal, deterministic replay of T1, repeatable runs | claims about real network latency, partitions, clock skew, separate failure domains |
| UI must say | "3 processes on 1 host: shared CPU, memory and kernel; loopback network" in the strip and on each node box | "N hosts", computed from distinct `host_id`s, with per-host OS and CPU as reported by the device |
| Must never happen | being shown as three machines | a single-host result shown as multi-host |
| Extra requirements | none | JWT mode, TLS, CORS exact origins, `PHANTOM_ALLOW_LAN_ORIGIN`, clock-skew display, pairing tickets, firewall approval (OD-24), >= 3 devices, Windows and Linux separately |

### 13.3 Live-demo failure handling

Stream disconnect shows `DISCONNECTED - last event #N`, charts freeze with "last known", and the presenter may switch to a recorded stage labelled REPLAYED; a node that fails to start is shown FAILED with its log tail, and the session continues in a visibly degraded mode or switches to replay; loadgen lag shows "offered load not achieved" (`send_lag_ms`); tracing that perturbs numbers is shown as TRACING ON and switched off for the measured arm; an unfavourable result is shown with the same weight as a favourable one and the presenter reads the pre-written sentence. A recorded fallback is a safety net, never a silent substitute.

---

## 14. Milestone and acceptance-criteria mapping (PROPOSAL)

**This section edits, rewords and removes nothing.** Original acceptance criteria stay exactly as written in their execution packages. Mapping to stages is a proposal for owner discussion.

### 14.1 Stages to milestones and gates

| Proposed stage | Maps to (existing) | Notes |
|---|---|---|
| 0 Truth and contracts | M1 residuals (truth and baseline), AC3-4 owner decision, doc audit B-1..B-10, claims-ledger corrections | No application code. The owner decisions OD-1 to OD-6 are the gating items |
| 1 Reliable distributed foundation | M3 (G-M3 = G-ARCH + G-DIST) completion; M2 residuals (TTL ceiling, wired default TTL, soak); M7 security items (SEC-M7-09/10/11, PKI, advisory scan) | The durable log is the M3 item the audit reports as open; physical-LAN evidence closes AC3-9's "or" |
| 2 Distributed caching capabilities | **No existing milestone.** M3's non-goal says "building B without measured need"; ADR-001 deferred Option B | A new gate (proposed name G-DIST-C) would be an owner addition; Level C is an owner decision (OD-1), not an implementation default |
| 3 Intelligence and adaptation | M5 (machinery, PI-1..PI-10, AC5-1..14), M6 (research evaluation, G-MVP), G-RESEARCH (RT-0) | M5's "PIE constructed in `main()`" is currently unmet (`main.rs:433` `pie: None`) |
| 4 Product excellence (alongside) | M4 (Lab), M7 (productization), M8 (UI/UX; gates U1, U2, D-8) | O8-2 closure rule proposed in section 11.3 |

**Delivery gates G1-G5 (STATUS section 2, the 2026-10-30 gates, not G-M1..).** G1 Scenario B and G2 Lab API: PASS with dispositions; retained, no change proposed. G3 (UI plus browser verification): verdict not recorded; the independent human design review (Z8) and usability study (W4-ST12) would be *additional evidence for the owner's G3 decision*, not a replacement of any headless-browser round. G4 (two whole-product review cycles) and G5 (commits and push): not started; this proposal does not touch them. Nothing is committed (CLAUDE.md section 1).

### 14.2 Original criteria retained verbatim (quoted, not reworded)

M3 (`phase_3_execution_package.md:96-104`): AC3-1 "E0 + fresh architecture trace complete; Phase 1 KG outcomes recorded"; AC3-2 "Harness controls (known-stale, known-good) PASS; proxy fidelity and determinism PASS"; AC3-3 "S3-1…S3-10 characterization delivered (VALID or class-labeled)"; AC3-4 "ADR-003 with failure model, contract, evidence, selected option decided **by the owner**"; AC3-5 "Selected option implemented/gated and verified by multi-process E2E tests (not unit tests alone)"; AC3-6 "Measured contract values reported with CIs; every claim in docs/UI maps to a tested clause; claims outside the failure model removed"; AC3-7 "Resource bounds under overload verified (queue depth, memory, tasks)"; AC3-8 "No orphan processes/files; ledger clean; no unauthorized wire change"; AC3-9 "Physical multi-host evidence PASS, **or** all network claims explicitly labeled loopback/injected".
M5 (`phase_5_execution_package.md:114-127`): AC5-1 to AC5-14 as written, including AC5-13 "`PHANTOM_INTELLIGENCE` defaults to `off`; legacy stub not in any product path; `pie: None` replaced by the v2 engine constructed in `main()` in the off/observe/advisory path".
M1, M2, M4, M6, M7, M8 criteria (AC-1..AC-12 of M1 and the respective AC lists): retained as written in their packages; I did not need to quote them and did not reread each. MVP definition M-1..M-11 in `mvp_and_milestones.md` is PROPOSED status there and is not touched here.

### 14.3 Retained criteria that are not met today (evidence-based; none is reworded)

| Criterion | Why not met | Evidence |
|---|---|---|
| AC3-4 | No dated owner decision on the selected option is recorded; ADR-003 "ACCEPTED" by two agents | STATUS M3 row; [audit] F1 |
| AC3-5 / AC3-6 | Cluster tests are loopback/debug; C6 "zero stale after 50 ms" is not supported; claims outside the failure model remain in earlier documents | STATUS; claims ledger C6; section 7 |
| AC3-9 | The "or all network claims labelled loopback" branch is what was used; physical multi-host evidence has never run | [audit] F5 |
| AC5-13 | `pie: None` at `main.rs:433` [checked]; `PieDecisionEngine` has no production caller | STATUS M5 row; P2 |
| AC5-3 | E4 never run; detector accuracy tables absent | P2 |
| AC5-8 | `AuditStore` has no producer | P2 |
| Other AC5 items | STATUS records M5 "independently verified: No"; unit-level tests exist; planted-effect detection power, rollback under load and replay on the product path are not shown. Not enumerated criterion by criterion | STATUS M5 row |

(Criteria that STATUS marks Partial for M1, M2, M4 are not enumerated criterion by criterion here.)

### 14.4 NEW criteria proposed for owner approval (not in force)

| ID | Proposed criterion | Why it is new |
|---|---|---|
| NC-1 | Every cluster/distributed claim in docs and UI maps to a numbered clause C1-C13/CC1-CC6 with failure model, bound and falsifier; clauses ratified by a dated owner decision | Strengthens AC3-6 without rewording it |
| NC-2 | Durable-log integration passes the K1-K20 matrix (Windows and Linux) and an independent review before any statement that issuer crash is survived | New content for the M3 durability item |
| NC-3 | Level B claim requires a release-profile, physical-LAN, >= 3-device propagation study with intervals, labelled LAN-PHYSICAL | Makes AC3-9's physical branch the only claim route for network claims |
| NC-4 | Level C claim requires the evidence gate of section 7.2 (model, multi-process tests, independent review, benefit/cost study); Level D claim requires the intelligence single-node gate | No existing milestone covers C and D |
| NC-5 | Tenant isolation claims beyond namespaces require the falsification suite (T-NN, T-QX, T-EP, T-BF, T-AF, T-DF, T-EE) with before-and-after records | P3 gap; no criterion today |
| NC-6 | Every UI value carries a tier; topology class and fault layer are computed, never typed; a `logical_flag` never renders as a failed node | Formalises UI honesty beyond M-2 |
| NC-7 | O8-2 closes only with owner approval plus an independent human design review with no open S1 | P4 proposal |
| NC-8 | Intelligence promotion requires the thresholds of the accountability report 11.6 (PROPOSED there) confirmed by the owner, with a named strongest static baseline per capability and a kill criterion recorded in advance | Makes the baseline discipline a gate |
| NC-9 | The flagship and any demo state what each stage proves and does not prove; an unbuilt capability renders UNAVAILABLE | Product honesty |
| NC-10 | Evidence manifests record the real build identity (source digest, dirty flag, binary SHA-256); `"unknown"` makes a run not VALID | Fixes the `manifest.rs:103-104` finding |

### 14.5 Proposed STATUS.md structure (proposal only; no percentages)

Keep the existing authority table and milestone table. Add a leading table with **five areas**, each with the four-question columns already used (Implemented, Tested, Independently verified, Formally accepted) plus **Evidence environment** and **Claim limit**:

1. **Single-node** (engine, fencing, freshness, byte accounting, origin protection).
2. **Distributed** with sub-rows A, B, C, D and a column "distributed claim?" (levels in section 5.2).
3. **Intelligence** (observe, advise, shadow, actuate; one row per capability with disposition and kill criterion).
4. **Product, security, observability** (Lab, Demo Center, tier labels, tenancy, identity, evidence integrity, design review).
5. **Release readiness** (clean-machine reproduction, release-profile suite, advisory scan, packaging, G-RELEASE).

Rules: never report a single test count as overall health; each row carries an independence column and an environment column (loopback, WSL2, physical, simulated); "Not recorded" is never converted to "No" or "Yes".

### 14.6 Proposed README positioning text (proposal only; README is not edited)

> PhantomCache is an origin-protecting HTTP caching gateway (research prototype). It runs as one node or as a cluster of gateways. Today a cluster is N independent caches joined by an authenticated, fenced invalidation bus; reads are served from each node's own cache, and a DELETE is not guaranteed to have reached every node. Coordinated distributed caching and distributed workload intelligence are staged goals that must pass published evidence gates before they are claimed. Adaptive behaviour is advisory unless evidence shows it beats a strong static configuration; the evidence to date does not establish that. Every number shown by the Lab is labelled real, simulated, replayed, predicted, derived or unavailable.

---

## 15. Risks, trade-offs and unresolved technical questions

Merged and deduplicated across P1 section 10, P2 section 8, P3 section 11, P4 section 7.1 and the audit. **Severity is my judgement for owner visibility (HIGH, MEDIUM, LOW), not a measurement.** "Src" names the draft(s).

### 15.1 Risks

| # | Risk | Sev. | Mitigation or owner action | Src |
|---|---|---|---|---|
| K1 | **Single-author evidence.** Every cluster test, the M3 reproduction and the fix were produced by agents in one workflow; all results are Windows, loopback, debug, <= 5 nodes | HIGH | Independent review (Z4); physical-LAN, release, two-OS study (W1-S1.8); label every result | P1, P3, STATUS |
| K2 | **Issuer-crash defect is open** (reproduced, fix is a prototype); lost invalidation leaves a peer stale for the origin-set lifetime, which has no gateway ceiling | HIGH | W1-S1.1 and W1-S1.7; until then print the non-guarantees (7.4) | P1, STATUS |
| K3 | **A compromised or buggy member can flush every tenant** (FullCache, or 10,001 tenant ids) and the stub GET handler can store fabricated entries in a peer's serving cache | HIGH | W1-S1.3/W3-S6; delete the stub in W1-S2.2; decide whether to disable it sooner (needs an approved code change) | P1, P3 |
| K4 | **No per-tenant resource isolation.** One tenant can evict another's entries and take its origin permits; unmeasured today | HIGH | W3-S7/S8/S9; Z7 records the before-state; present the noisy-neighbour result honestly | P3 |
| K5 | **Lagging-owner stale resurrection (H4)** in peer-fill is worse than Level B exposure; the lease design is untested | HIGH (for Level C) | Model, mutation checks, shadow mode first, kill switch; the gate in 7.2 | P1 |
| K6 | **Intelligence ahead of correctness** polishes a prototype; warming before invalidation durability widens stale exposure | MEDIUM | Ordering in section 12; cross-node warming blocked | P2, P1 |
| K7 | **The best real intelligence result may be negative** (AIMD already was) | MEDIUM | Kill criteria; negative results equally weighted in UI; "valid outcome" wording | P2, P4 |
| K8 | **Fabricated or decorative UI**: logical flag shown as failure, single-host shown as multi-host, counterfactual lines, synthetic data presented live | HIGH (to credibility) | LR1-LR6; computed topology class; honesty tests; human review | P4 |
| K9 | **Event layer touches the read-path zero-contention invariant** and byte accounting is the most invariant-heavy module | HIGH | Approval, benchmark, independent review, tests that fail without the change; tracing off in evidence runs | P2, P3, P4 |
| K10 | **Shared HS256 secret** means a leaked secret is total compromise of that domain; guest tokens are bearer tokens without device binding; revocation is expiry or per-node denylist | MEDIUM | Per-session secrets; asymmetric signing is an owner decision; say so on screen | P3 |
| K11 | **Security demos read as certification** | MEDIUM | The binding statement in 9.6 on every demo; "what it does not prove" column | P3, P4 |
| K12 | **Pre-warming is an amplification path** if the calendar is unauthenticated or over-permissive; warmed bytes can evict hotter entries; bucket canaries are biased | MEDIUM | Secure-mode, role-gated, tenant-scoped writes; byte-quota partition | P2 |
| K13 | **Hash and protocol stability**: `DefaultHasher` is unspecified across Rust releases (LITERATURE); v1 peers drop the connection on unknown message types (`node.rs:661-664` [checked]), so rolling upgrades need a design | MEDIUM | Fixed hash (OD-9); version negotiation (OD-11) | P1 |
| K14 | **Time**: freshness is wall-clock; JWT `exp` has 10 s leeway; a clock step can silently age out durable debts; no clock-fault tests | MEDIUM | Age-out safety in W1-S1.1; observer-clock convergence measurement | P1, P3 |
| K15 | **Heartbeat and lease parameters unknown**; false positives could make partition policies harmful | MEDIUM | Defaults come from LAN experiments, not from drafts | P1 |
| K16 | **Peer-fill benefit unproven**; if key reuse is low Level C may not pay | MEDIUM | The gate may legitimately end in "not worth it" (OD-1) | P1 |
| K17 | **Platform**: Windows only, loopback only; Linux never built; supervision and CTRL_BREAK are Windows-specific; durable-log fsync and rename semantics differ | MEDIUM | Two-OS gate; K19/K20 | P1, P4 |
| K18 | **Scale limits**: `u8` node ids (255), failover ranking of primary plus 2 replicas, churn above 5 nodes never exercised | LOW | State the tested size | P1 |
| K19 | **Design effort displaces backend work**; reviewers disagree; the UI has never had a human review | MEDIUM | Early review (Z8) and late review (ST12) | P4 |
| K20 | **Design-resource text contains agent directives** | LOW | Store as data; `SOURCE.md` lists edits; filter | P4 |
| K21 | **Original vision source missing** (ChatGPT export); the levels here are a design consistent with the mandate, not a recovery of the original requirement | MEDIUM | OD-47 | audit, P1 |
| K22 | **Authorization gaps in the record**: MVP wording, README line, ADR-001/002/003, M1-M8 packages have no recorded owner approval (an evidence gap, not a conclusion) | MEDIUM | OD-2, 3, 4 | audit |
| K23 | **Staleness of internal documents**: CLAUDE.md sections 5/6 vs code (eviction, anti-entropy), `architecture_trace_m3.md`, ADR-003 "5 s"; README and CLAUDE.md still quote 242 tests | LOW (but misleads agents) | Dated corrections by the owner or lead; no edits by this file | P1, audit, Appendix A |
| K24 | **Test-count inflation as health**: 851 passing debug tests are not correctness or research validity | MEDIUM | STATUS rule in 14.5 | STATUS, CLAUDE.md |

### 15.2 Trade-offs the owner should see

| Trade-off | Options | Note |
|---|---|---|
| Fail-open vs fail-closed durable log | 204 stays available vs 204 implies durable intent | A 5xx after local purge and primary ack does not undo them; owner states what a DELETE must mean (OD-5) |
| Partition policy | serve-until-origin-lifetime vs lease-clamp vs bypass | Availability vs staleness vs hit ratio; no default recommended (OD-10) |
| Per-node tokens/secrets vs per-session secrets | tighter blast radius vs client complexity | P3 2.2 |
| Peer-fill option | O1 owner-forward vs **O2 peer-fill on miss (P1 candidate)** vs O3 hybrid vs O4 replicate hot keys | O2 keeps the hit path and current fan-out; O1 changes the serving contract on every read |
| Cluster origin budget | R1 static share (now), R3 owner-concentrated (with Level C), R4 leases / R5 gossip AIMD (research) | R2 membership-aware share can exceed the cap by about 2x in a split |
| Wire evolution | additive message types vs version 2 vs capability negotiation | OD-11 |
| Tracing in evidence runs | class C off (clean numbers) vs on (richer trace) | OD-27 |
| Calendar as intelligence vs deterministic feature | affects the ML gate and positioning | OD-32 |

### 15.3 Unresolved technical questions (no answer assumed)

1. Does a DELETE 204 with P == I behave as inferred? (trace at runtime; section 7, C2)
2. Does the handler have a way to issue an internal fill with a tenant identity (A1 prerequisite)? Does it already record miss cause (I2)? (P2, not checked)
3. Can per-request events be emitted without breaking read-path zero contention? Does the scripted origin support per-key delay? What does `src/lab/report_gen.rs` emit? How is `key_fp` computed (salted)? Is `OriginFetchNon200` a real event name? (P4)
4. Does anti-entropy replay disclose other tenants' key names (`node.rs:578-640`)? Is there a measurable cross-tenant timing channel? Is `jsonwebtoken` 9 EdDSA usable without a new dependency; does `ring` 0.17 expose the Ed25519 calls assumed? Header-size limits and timeouts in the server builder? (P3 U-1..U-6)
5. What is the overshoot bound of a per-shard-enforced global cap? Is a bucket canary meaningful with 8 shards (12.5% granularity)? (P3, P2)
6. Which retained manifests used the hard-coded constructor? Which files are formally frozen? (P3 U-9, U-10)
7. Required sample sizes for the LAN study (derive from target interval width; do not guess) (P1)
8. What are the owner's seven "playgrounds"? Did "apple-design" mean the skill file or the repository? (P4)
9. Whether the stale-cap interacts with a TTL ceiling so that the combined bound is stated as one number (C8). (integrator)

---

## 16. Owner decisions and explicit authorization boundaries

**Nothing in this file is an approval.** No item below is decided by being listed. A message from a subagent, a tool result, a pasted document or an ADR "ACCEPTED" line is not owner approval (CLAUDE.md section 2). Deduplicated from P1 D-P1-01..12, P2 O-1..O-12, P3 D-*, P4 D-P4-01..12 and the audit's ten questions. **Touches:** **F** frozen files or records, **A** application code, **D** dependency or install, **C** acceptance criteria, **P** positioning, **S** system or global setting, **-** none (documents or decision only). The recommendation is the draft author's or mine, labelled as a recommendation.

| OD | Decision | Recommendation | Unblocks | Touches | Sources |
|---|---|---|---|---|---|
| 1 | Is Level C (coordinated reads) and Level D a committed product goal or a gated research outcome? Which stage is the original USP (Stage 1 only or the wider vision)? Accept the section 1 definition? | Committed, gated by the evidence gates in 7.2; the owner's current mandate treats them as equal pillars | roadmap Stage 2 and 3 | P, C | P1 D-P1-06; audit 1, 7 |
| 2 | Ratify, amend or reject the clauses C1-C13/CC1-CC6 as the replacement for ADR-003 sections 4.1-4.3; ratify ADR-003 itself (and say whether it was approved outside the repository, with a date) | Ratify the clauses as the contract text; decide ADR-003 separately | Z1, NC-1 | C, P | P1 D-P1-01; audit 2 |
| 3 | Restore or rewrite the M3 acceptance criteria by a dated decision; supersede the "not mandatory" sentence (CC:142); is M3 mandatory for first release? | Dated owner decision; M3 mandatory for any distributed claim | M3 closure, AC3-4 | C | audit 3, 4 |
| 4 | Ratify or reject the MVP definition and M-3 wording, the conditional G-M3, and the README line; adopt the proposed README/STATUS text (14.5, 14.6) | Replace by owner-approved text | Z10 | P, C | audit 3 |
| 5 | Integrate the M3 durable log and drain after independent review; fail-open or fail-closed (or a setting); drain default (prototype 2000 ms); directory hardening | Approve conditionally; owner states what a DELETE must mean | W1-S1.1 | A (INV), C | P1 D-P1-02; audit 9 |
| 6 | Commission the independent review of the patch: who, with what brief | A reviewer who did not author the patch or reproduction; give code, not conclusions | Z4 | - | P1 |
| 7 | Approve application-code changes to `router.rs`, `main.rs`, `config.rs`, `lib.rs`, plus a detached DELETE cluster leg in `server.rs` | Only after 5 and 6 | W1-S1.1 | A (INV) | P1 D-P1-04 |
| 8 | Approve adding test, harness and falsification-runner code (S0.4, W3 Stage 1, S12); no new dependency | Approve | Z3, Z7, S12 | A (tests), D none | P1 S0.4; P3 D-TEST |
| 9 | Placement hash (SHA-256 truncation via existing `ring`, or another) and the rule "owners come from the membership epoch, not health" | Approve | W1-S2.1 | A | P1 D-P1-03 |
| 10 | Partition policy: serve-until-origin-lifetime, lease-clamp, or bypass; per-deployment setting? | No default until LAN measurements exist | W1-S1.9 | A | P1 D-P1-05 |
| 11 | Wire/version policy: heartbeat, fill and signal message evolution; are wire changes acceptable before 1.0? | Additive types plus negotiated capability | W1-S1.2, S2.2 | A, C (AC3-8) | P1 D-P1-08 |
| 12 | Cluster PKI tooling: promote `rcgen` to a tool dependency, extend the `openssl` script, or Windows-only PowerShell | Decide after weighing; script has no repo dependency | W1-S1.6, W3-S5, ST6 | D | P1 D-P1-09; P3 D-PKI |
| 13 | Is physical multi-host evidence (>= 3 devices, Windows and Linux) required for any cluster claim? | Yes for any network claim (NC-3) | W1-S1.8 | C | P1 D-P1-10; audit 6 |
| 14 | Gateway TTL ceiling and wiring of `PHANTOM_DEFAULT_TTL_MS` | Approve; it makes C8 true | W1-S1.7 | A, C | P1 D-P1-07 |
| 15 | Is dynamic join/leave in scope, and who holds the trust decision? | Operator-signed registry; revocation first, join later | W1-S1.4 | A | P1 D-P1-11 |
| 16 | WAN and multi-region deferred (proposed) or commissioned? Handoff Phase 7 (chaos) and 9 (WAN replication): in or out? | Deferred, not dismissed | scope | P | P1 D-P1-12; audit 10 |
| 17 | Ledger escalation semantics: Tenant command-style with capability, FullCache receiver-driven (P1's lean) | Decide jointly, after the multi-node model | W1-S1.3, W3-S6 | A, C | P1 12.1; P3 12 |
| 18 | SEC-M7-09/10/11 dispositions; auth hardening S3 (strict roles flag, `jti` denylist, expiry close, limiter); HTTP hardening | (b) strict-roles flag default on in production profile; add SSRF ranges; counters and per-source limiter; no IP lockout | W3-S3, S9 | A | P3 D-SEC-* |
| 19 | Authorize the tenant registry and Stage A accounting hook, then quotas (Stage B) | Stage A first; independent review | W3-S7, S8, S10 | A (INV) | P3 D-TENANT |
| 20 | Separate control listener / bind (code); two-process interim | Two processes now | W3-S4 | A | P3 D-CTRL |
| 21 | Evidence signing key custody; verifier bin; annotation of retained manifests that carry the constant tree hash (no edits to retained records) | Per-session ephemeral key plus an out-of-band release key; annotate by dated notice | W3-S11, NC-10 | A, F | P3 D-KEY, U-9 |
| 22 | LAN trust bootstrap (demo CA, human fingerprint comparison, or plain HTTP with banner on a private network); QR support | Banner and TTL <= 10 min if plain HTTP; no QR | W3-S2, ST11 | S, D (QR) | P3 D-TLS, D-QR |
| 23 | Which files and records are formally frozen? Is a freeze addendum-2 acceptable? | Publish the list before any code | W3-S11, W2-P-7, W2-P-1 | F | P3 D-FROZEN, U-10; P2 O-2 |
| 24 | System changes: firewall rules, trust-store changes, installs, OS-level fault injection | Documented manual steps with exact rules printed for the human; cleanup record | W1-S1.8, ST11 | S | P3 D-SYS; P1 |
| 25 | Third-party security tooling (advisory scan, TLS scan, fuzzing, pen test) and externally invited attacks | Each needs separate approval; sandbox only | W3-S12 | D, S | P3 D-TOOL, U-8 |
| 26 | Approve E0 (application-code change for the event tap) and the order E0/E4 vs E1/E2; application-code changes for event taps | Defer taps until payloads are agreed | W2-P-1, W4-ST7 | A (read path) | P2 O-1; P4 D-P4-12 |
| 27 | Maximum acceptable measured tracing overhead; tracing in evidence runs? | Tracing off in evidence runs unless the interval excludes the bound | W4-ST7 | A | P4 D-P4-07 |
| 28 | Approve `src/lab` changes (new modules for E1/E2/E9); freeze addendum-2 | New modules, not edits to frozen files | W2-P-7, P-8 | F, A | P2 O-2 |
| 29 | Salt and seed custody for any v3/v4 sealed run | Owner generates offline outside any agent session; no agent proposes values | E2 confirmatory, E9, E11 | F | P2 O-3 |
| 30 | E2 margin definition (3 pp goodput vs 3% relative loss reduction) and other thresholds (V2: 10% miss-storm margin, MASE margin, coverage band, waste bound) | Owner fixes before any run | E2, E9-E11 | C | P2 O-4, O-12 |
| 31 | May the E2 exploratory dry run on dev seeds run now? | Yes (labelled EXPLORATORY; cannot trigger a kill) | E1/E2 start | A (lab) | P2 O-5 |
| 32 | Is the Demand Calendar "intelligence" for the narrative, or a deterministic feature that intelligence calibrates? | Deterministic feature with intelligence calibrating | ML-gate scope, positioning | P | P2 O-6 |
| 33 | Is storing key names (warm-up manifests, trace samples) acceptable, or everything hashed? | Hashed by default; opt-in encrypted with ACL | I10, trace sampling | A | P2 O-7 |
| 34 | Is T2 loopback confirmation enough for promotion, or must a physical-host run precede any serving-path actuation? | Physical-host run before Active | Canary/Active | C | P2 O-8 |
| 35 | Fund a dedicated RT-0 plan (fixed venues, years, recorded queries) before any positioning claim | Yes | G-RESEARCH, positioning | P | P2 O-9 |
| 36 | Add `Shadow` to the mode enum; rename or document `canary` | Add; document today's meaning | W2-P-12 | A | P2 O-10 |
| 37 | Obtainable real multi-week trace or design partner | Seek one | I13 | - | P2 O-11 |
| 38 | **O8-1**: adopt, decline or reading-only for Emil's repository; copy anything from the other two? | Option A minimal; copy nothing from the other two | design-resource folder | D none | P4 D-P4-01 |
| 39 | **O8-2**: approval = owner approval AND an independent human design review, no open S1 | Yes | ST12 | C | P4 D-P4-03 |
| 40 | New `demo-1` stream vs extending `lab-1` | New stream | W4-ST4 | A | P4 D-P4-02 |
| 41 | Tier labels as in 10.1 (INFERRED as a qualifier, DECLARED as an input kind); create `MULTI_HOST_LAN` and `SINGLE_HOST_MULTI_PROCESS` labels | Yes; computed from `host_id` | all UI | P | P4 D-P4-04, D-P4-08 |
| 42 | Reorder the flagship into ten stages with the prediction stage demoted until P2 delivers | Yes | section 13 | - | P4 D-P4-05 |
| 43 | Multi-device pairing allowed? JWT only or scoped ticket? any non-loopback exposure? | Allowed only with scoped tickets and P3 sign-off | W4-ST11 | A, S | P4 D-P4-06 |
| 44 | Who conducts the design review and usability study; recruitment (O8-4) | Owner chooses the people | Z8, ST12 | - | P4 D-P4-09 |
| 45 | Keep a toy Simulation page or fold it into the Demo Center lens | Fold in; interim re-home first | ST2 | - | P4 D-P4-10 |
| 46 | Is a possibly-negative noisy-neighbour experiment acceptable on stage? | Yes, as a measured experiment with a pre-stated criterion | demo of K4 | P | P4 D-P4-11 |
| 47 | Does the ChatGPT export exist? Add it to `docs/handoff/transcripts/`. Did the earlier P2 and P4 (2025-10-25 definitions) get superseded by M3/M5? | Provide if it exists | the vision record | - | audit 5, 8 |
| 48 | Which distributed-intelligence capabilities belong to the original vision (cluster-wide hot-key view, coordinated pre-warming, cluster-aware origin cap)? | Owner states | Level D scope | P | audit 7 |
| 49 | Disposition of the section-1 product definition wording | Accept, amend or reject | all | P | integrator |
| 50 | Supply the seven playgrounds' names | Owner supplies | Demo Center IA | - | P4 7.2 |

### 16.1 Allowed now without approval vs needs approval

Per CLAUDE.md sections 1 and 2 (documentation under `docs/research/` and read-only investigation need no extra approval; application code, tests treated conservatively, dependencies, installs, system settings, new phases, architecture changes, acceptance criteria and positioning need approval; git add/commit/push only on a plain-text request).

| Work | Allowed without approval | Needs approval |
|---|---|---|
| Documents under `docs/research/` (this file, models specified as documents, test plans, review rubrics, RT-0 plan, experiment protocols) | **Yes** | - |
| Read-only investigation (reading code, tracing, source reading, reproducing a claim without changing files) | **Yes** | - |
| Writing the independent-review brief and the K1-K20 plan as documents | **Yes** | The review itself needs a reviewer (OD-6) |
| Edits to README, STATUS.md, CLAUDE.md, `docs/user/`, ADRs, any frozen record | - | Owner decision (OD-2, 3, 4); frozen records are never edited, only noticed |
| Test or harness code, falsification runner (Z2, Z3, Z7) | - | OD-8 |
| Scripts and standalone tools outside `src/` (minter, bundle layout) | - | Treat as needing approval for new files outside `docs/research/` (OD-8) |
| Application code under `src/` (all APP and INV items) | - | Explanation first, then approval; INV items also need an independent review pass and a failing-before test |
| Any new runtime or dev dependency (e.g. `rcgen` promotion, QR crate, a chart library) | - | OD-12, 22, 38 with the per-dependency justification (purpose, alternatives, maintenance, security, dev-only) |
| Installing software, firewall rules, trust anchors, OS-level fault injection | - | OD-24 |
| UI changes under `ui/` | - | Owner direction; O8-1/O8-2 decisions for the design system |
| New phases, new milestone, architectural changes (Level C, `demo-1`) | - | OD-1, 40 |
| Sealed evaluation runs, salts, seeds | - | Owner generates (OD-29) |
| Reading and recording design-resource facts (no install) | **Yes** (reading only) | Copying files into the repository needs OD-38 |
| `git add`, commit, push | - | Only on the owner's plain-text request (CLAUDE.md section 1) |

---

## Appendix A: Reconciliation notes

Each note names the conflicting sources, what the code or record shows, and the decision taken **in this document** (a document convention, not an owner approval). No draft was edited.

**R1 Tier vocabulary.** Sources: P4 2.2 (six tiers REAL/SIMULATED/REPLAYED/PREDICTED/DERIVED/UNAVAILABLE, with INFERRED and DECLARED left open), P2 (Measured/Derived/Simulated/Unavailable; `SYNTHETIC_FORECAST_HARNESS`; `MEASURED_LOOPBACK`; `DECLARED`), P1 and P3 (measured/inferred/unavailable/simulated), code today (`provenance.js`: MEASURED, DERIVED, SIMULATED, INFERRED, UNAVAILABLE, RETAINED_*). **Decision:** the six tiers are the display tiers (owner's list). Mapping: MEASURED and MEASURED_LOOPBACK -> REAL (with `topology_class`); `SYNTHETIC_FORECAST_HARNESS` and T1 -> SIMULATED (with `sim_kind`); RETAINED_* -> REPLAYED; INFERRED -> a **basis qualifier** on REAL or DERIVED (P4 recommendation, OD-41); DECLARED -> **input-kind** on actions and predictions. P2's rule that DECLARED may inform but never gate is kept, clarified as: a declared input may trigger a bounded action but can never satisfy or relax a safety check. P4's `SINGLE_PROCESS` topology class applies to REAL values only; T1 is SIMULATED, so it does not need one (P4 wrote T1 as the `SINGLE_PROCESS` example).

**R2 Event taxonomy and the demo-1 envelope.** Sources: P4 2.3 (envelope and kinds), P1 12.1 (event list; correction: the wire has no per-DELETE `inv_id`), P2 10 (accepts `workload_observation`, `prediction`, `prediction_outcome`, `capacity_estimate`, `action`, `safety_decision`, `decision_trace`), P3 12b (`tenant_quota` cannot be produced until Stage A; show UNAVAILABLE, not zero; scope claims are not enforced by the gateway). **Decision:** one envelope (10.2). `inv_id` is issuer-assigned and logged, not on the wire; the on-wire identifier is (issuer, destination, stream sequence) now and (issuer incarnation, destination, stream sequence) after W1-S1.2. P2's `decision_trace` is a payload of `action`. `warm_plan`/`warm_result` (named in P4 stage 3 and P1 section 12) are added to the kind table with P1 as owner. P3's observation: the scope claims (`session_id`, `classes`, `privacy`, `tenants[]`) can ride in tokens today (`JwtClaims` ignores unknown claims) but nothing enforces them until new code reads them; a tenant-sliced Observer cannot be claimed as isolation until then.

**R3 Mode names.** Sources: P3/P4/P2 use "secure mode" and "development mode"; `src/config.rs:96` defines `PHANTOM_MODE` as `Kind::Enum(&["development", "production"])` with default `development` [checked]; `PHANTOM_INTELLIGENCE` accepts exactly `off`, `observe`, `advisory`, `canary`, `active` (`config.rs:161`) [checked]; the enum in `intelligence/mod.rs:22-36` has no Shadow. **Decision:** there is **no `secure` value of `PHANTOM_MODE`**. "Secure mode" in the drafts means a JWT secret is configured and auth is required (`require_observer_if_secure` tests `sec.require_auth_for_control`, `server.rs:337-344`); development mode with no secret is anonymous. `production` additionally requires auth, a 32-byte JWT secret and forbids simulation and insecure overrides. Shadow is a proposal (W2-P-12, OD-36). The doc comment on Canary (`mod.rs:28-29`, "f <= 5%") describes behaviour the code does not implement.

**R4 Meaning of a DELETE 204 when the issuer is the key's primary.** Sources: P1 1.3 item 1 (no remote assurance; inference), ADR-001 section 8 (left open). **Checked:** `server.rs:1990` purges locally first; `router.invalidate_key_across_cluster` (`router.rs:1790+`) calls `route_key`, which iterates *all* nodes including the local one (`router.rs:1356-1366`); the local node has its own listener address (`start_tcp_service` calls `set_addr`, `main.rs:334`, `node.rs:185`) and `connect_to_node_inner` has no local-node special case (`router.rs:1196-1215`), so the primary leg dials the issuer's own cluster listener, and `Ok(...)` on that leg triggers `dispatch_fanout` (`router.rs:1817-1823`) and a 204. **Established by code reading:** for keys whose canonical primary is the issuer, the "primary ack" is the issuer's own listener after the issuer already purged locally. **Not established:** that this happens at runtime (not traced or tested). **Position:** P1 is supported by the code I read. A 204 means "local purge, and primary or acting primary applied"; for about 1/N of keys it means "I purged myself and no remote confirmation was obtained synchronously". Treat clause C2 accordingly until a test shows the local-primary case. Also noted: the response carries `x-cache-warning: PrimaryFailover` only when the acting primary was used (`server.rs` near 2008).

**R5 Staleness bound after a lost invalidation.** Sources: ADR-003 G3 ("<= 24 h"), P1 C8, CLAUDE.md section 5 (`PHANTOM_MAX_STALE_MS` hard cap). **Checked:** freshness lifetime comes from `s-maxage`, then `max-age`, then `Expires`, else the configured default (`freshness.rs:263-276`), with no gateway ceiling; the default is the literal `60_000` because `PHANTOM_DEFAULT_TTL_MS` is NotWired (`config.rs:115-116`); `PHANTOM_MAX_STALE_MS` is the stale-*serving* cap (`cache.rs:290,336,389`; `server.rs:1318`) and bounds serving *after* expiry only. **Decision:** the correct statement is P1's C8: after total loss of an invalidation a node can serve until the entry's origin-set freshness lifetime ends, plus the permitted stale window; "staleness <= 24 h" is false until a TTL ceiling exists (OD-14). The two caps are different mechanisms and must not be conflated in any document.

**R6 Does the serving cache use the faithful eviction policies?** Sources: CLAUDE.md section 5 ("Faithful Eviction: ... FaithfulSieve, FaithfulLru, FaithfulS3Fifo") vs section 6 ("evictors named SIEVE/S3-FIFO/LRU are scans"); P2 I7 ("`core::eviction` used by the lab only ... not re-confirmed"); P3 (uses `ShardEvictionPolicy` in the serving design). **Checked:** `CacheShard` holds `eviction: Box<dyn crate::core::eviction::ShardEvictionPolicy>` (`cache.rs:209`), constructed by `create_shard_eviction_policy` (`cache.rs:223,506,532,1214`), default `EvictionPolicy::Sieve` (`cache.rs:214,332`); `create_shard_eviction_policy` returns `FaithfulSieve`, `FaithfulLru` or `FaithfulS3Fifo` (`eviction.rs:690-699`). **Result:** the serving cache **does** use the faithful policies. CLAUDE.md section 6's "evictors ... are scans" is outdated; CLAUDE.md section 5 is right; **P2's statement is wrong** and is corrected here. I did not re-verify the O(1)-amortized claim (P3 notes a bounded scan of about 2n inspections in SIEVE victim selection, `eviction.rs:186-219`, which I did not open). CLAUDE.md is not edited by this task; a dated correction is for the owner or lead.

**R7 Cluster quota semantics.** Sources: P1 7.4 (assumes total quota Q split as Q/N per node), P3 4.6 (quotas are per node; capacity does not pool; cluster-wide tenant quota not enforceable without coordination). **Decision:** these agree once stated precisely. Every node enforces its own configured quota; "Q/N" is an *operator configuration convention* (set each node's quota to Q divided by the configured node count), not a coordinated cluster quota; a tenant's total footprint is at most N times the per-node cap. Owner-weighted allocation is a Level C item (W1-S2.5). Every cluster screen says "per node".

**R8 Ledger escalation: command-style vs receiver-driven.** Sources: P3 6.2 and 12 (asks P1), P1 12.1 (lean: Tenant command-style with per-peer capability; FullCache receiver-driven). **Decision:** unresolved; recorded as OD-17 with P1's lean as the working proposal. Receiver-driven FullCache needs a new typed message, therefore interacts with OD-11, and should be checked in the multi-node model (Z2).

**R9 The cluster GET stub.** Sources: P1 row 21 (fabricates and stores `MISS val_node_<id>`), P3 6.1 and A2 (fabricated body, pinned by `known_gaps`). **Checked** (`node.rs:409-462`): the `GetRequest` arm builds `CacheKey::new(None, key, None)` from the peer-supplied string; if the node is Recovering or Suspect it returns `"MISS val_node_<id>"` without touching the cache (`SEC-POA-04`); otherwise `begin_fetch_atomic`: on a hit it returns `HIT <body>`; on `Initiate` it **fabricates** a `CachedResource` with body `val_node_<id>`, `ttl_ms: 60_000`, calls `guard.complete(FinalizeOutcome::Success(res))`, which (via `finalize_fetch`, `cache.rs:1457-1469`) stores it if generation and epochs still match, and returns `"MISS val_node_<id>"`. **Result:** P1, P3 and A2 are all correct. INFERENCE (untested): a peer can plant a fabricated entry in the serving cache under a key of its choosing; if a client later requests that canonical key the gateway could serve `val_node_<id>`, so the consequence is cache poisoning by an authenticated member, not only resource consumption. The handler is live in the cluster listener; `forward_key_over_tcp` has no production caller. The stub is to be removed in W1-S2.2; whether to disable it sooner is an application-code decision (OD-7).

**R10 Peer-fill option choice.** Sources: P1 4 (O2 candidate, shadow first), P2 (treats peer-fill as an undecided architecture decision; says "avoid duplicate warming" does not mean one node warms for all because GET is local), P3 (peer reads need per-tenant authorization on the wire), P4 stage 3 (says P1 "coordinates which node warms which keys"). **Decision:** carry O2 as the *candidate under the gate*, not a decision (OD-1). Until Level C passes its gate, "coordination" in warming means: a per-node share of a cluster origin budget (`cluster cap / N_configured`, R1), nodes warming only keys their own traffic will request, and no node warming a key already warm and fresh locally. The stronger forms ("owner warms, others fetch") are peer-fill dependent. P4's flagship Stage 3 wording is read in this narrower sense.

**R11 Manifest `git_tree_hash`.** Sources: P3 1.2, P4 (reported manifests per run). **Checked:** `manifest.rs:103` is the literal `456bb25930004f1896a84c53e776e7b3ef29ee16` (the HEAD commit id, not a tree hash) and `:104` is `is_dirty: true` with a comment, both constants. P3's finding is correct. Retained records stay unedited (OD-21).

**R12 Naming collisions.** P2's refusal reasons R1-R18, P4's rules R1-R6, P3's risks R1-R7, P1's options R1-R5 for resources and the audit's R-numbers. **Decision:** here P2's become `P2-R1..R18`, P4's rules `LR1..LR6`, P1's resource options stay "R1..R5 (P1 7.2)", and this appendix's notes are `R1..R14` only inside this appendix.

**R13 Cross-node warming wording.** P4 Stage 3 says "BLOCKED on P1 (coordination) and P2 (calendar input)". P2 5 says local origin-sourced warming is *not* blocked and only cross-node warming is. **Decision:** both are right for different warming; section 13 and the roadmap use P2's split.

**R14 Other contradictions found.**
(a) Line numbers: P1's `main.rs` citations (`:332` shared-cache constructor, `:375` and `:383` anti-entropy workers, `:380-384` insecure branch) are correct in the working tree [checked]; CLAUDE.md quotes older numbers (327, 359, 368, 370, 374). I first mis-read these as off by one and corrected myself after a grep; no discrepancy.

---

## Appendix B: Spot-check table

Claims taken from the four drafts (and the audit/CLAUDE.md where noted) that I opened myself on 2026-10-30. "Verified" = the cited text says what the draft says. File paths are relative to the repository root.

| # | Claim | Draft section | File:line checked | Result |
|---|---|---|---|---|
| 1 | DELETE purges locally before the cluster call, then calls the router | P1 1.2 #8, C1 | `src/api/server.rs:1988-1998` | Verified (purge at 1990, router call 1992-1995) |
| 2 | `route_key` iterates all nodes including the local one; no local special case in the connect path | P1 1.3 item 1 | `src/dist/router.rs:1352-1366`, `:1196-1215`; `src/main.rs:331-341`; `node.rs:181-186` | Verified by code reading; runtime behaviour untraced (inference stands) |
| 3 | `route_key` skips Suspect/Recovering/Failed/Offline nodes (health-dependent owner) | P1 4.5 | `router.rs:1357-1360` | Verified |
| 4 | Hash is `DefaultHasher` over (key, node_id) cast to `f64` | P1 4.5 | `router.rs:1343-1349`, `:1363-1365` | Verified |
| 5 | Freshness lifetime from `s-maxage`, `max-age`, `Expires`, else default; no ceiling | P1 1.3 #7, C8 | `src/core/freshness.rs:263-276` | Verified |
| 6 | `PHANTOM_DEFAULT_TTL_MS` is NotWired; literal 60_000 | P1 C8, P3, CLAUDE.md | `src/config.rs:115-117` | Verified |
| 7 | `PHANTOM_MAX_STALE_MS` default 24 h, stale-serving cap | P1 C8 | `config.rs:119`; `cache.rs:290,336,389` | Verified |
| 8 | Cluster GET handler fabricates `val_node_<id>` and completes it as Success (stores) | P1 row 21, P3 6.1 | `src/dist/node.rs:409-462` | Verified (also Recovering/Suspect branch returns MISS without cache) |
| 9 | Scope handling: ExactKey/Tenant/FullCache applied with no per-member authorization | P1 C13, P3 1.3 | `node.rs:533-559` | Verified |
| 10 | Epoch mismatch closes the anti-entropy connection | P1 1.2 #12 | `node.rs:586-597` (check at 590-595) | Verified |
| 11 | Unsupported message type terminates the connection | P1 5.5 | `node.rs:658-666` | Verified |
| 12 | Eight cluster message types | P1 1.1 | `src/dist/frame.rs:20-32` | Verified |
| 13 | InvalidateRequest payload carries epoch, stream seq, originator, scope, key only | P1 1.2 #14 | `frame.rs:157-166` | Verified |
| 14 | Heartbeat only probes idle pooled connections; payload is empty | P1 1.2 #15, 2.3 | `src/dist/pool.rs:360-380`, `:404-412` | Verified |
| 15 | `ClusterMembershipRegistry::update` has no non-test caller | P1 row 6, P3 #8 | `src/security/cluster.rs:405`; grep of `src/` for `.update(` (only `signals.rs:483`, `t1_sim.rs:1059`, unrelated) | Verified |
| 16 | Local node shares the HTTP serving cache; anti-entropy worker started in both cluster branches | P1 1.1, CLAUDE.md | `main.rs:332`, `:375`, `:383` (grep) | Verified |
| 17 | `pie: None` in the live state | P2, P4, STATUS | `main.rs:433` | Verified |
| 18 | Tenant epoch overflow falls back to a global epoch bump | P1 1.3 #5, P3 1.3 | `src/core/cache.rs:579-595` | Verified |
| 19 | Fencing: store only if generation and tenant/global epochs match | P1 row 13 | `cache.rs:1455-1470` | Verified |
| 20 | DELETE bumps generation and detaches the in-flight entry | P1 row 13, C1 | `cache.rs:1609-1638` | Verified |
| 21 | Repair bounds and `REPAIR_BATCH_PER_TICK = 64`; default cluster timeout 150 ms | P1 C3, C4 | `router.rs:56-58`; `router.rs:1772-1780`; `config.rs:159` | Verified |
| 22 | `protocol_model_tests` asserts only initiate < complete < invalidate | P1 1.3 #3 | `tests/integration/protocol_model_tests.rs:95-106` | Verified (file is under `tests/integration/`) |
| 23 | Harness has `kill_on_drop` only; no kill/restart helper in the working tree | P1 row 24 | `tests/support/cluster_harness.rs:494` (grep) | Verified for what the grep covers |
| 24 | Mode enum has exactly five values; config accepts those five; no Shadow | P2 2.3 | `src/intelligence/mod.rs:22-36`; `config.rs:161` | Verified |
| 25 | `PHANTOM_MODE` is `development`/`production` only; no `secure` | owner brief | `config.rs:96` | Verified |
| 26 | Origin cap enforced only in Canary/Active | P2 1 | `src/intelligence/executor.rs:215-220` | Verified |
| 27 | `hot_keys` returns `unavailable/not_implemented`; `key_hash` computed in the handler | P2 1, I1 | `server.rs:766`, `:1261` | Verified |
| 28 | `route_bucket`/`apply_policy` have no serving caller | P2 1 | grep over `src/` (hits only in `executor.rs`, `t1_sim.rs:1067`, `mod.rs:197` trait) | Verified |
| 29 | "`core::eviction` is used by the lab only" | P2 I7 | `cache.rs:209,214,223,332`; `eviction.rs:690-699` | **WRONG.** Serving cache uses `Box<dyn ShardEvictionPolicy>` with default SIEVE (R6) |
| 30 | CLAUDE.md section 6 "evictors ... are scans" | CLAUDE.md | same lines | **Outdated** (CLAUDE.md section 5 is correct) |
| 31 | `manifest.rs` hard-codes `git_tree_hash` (HEAD commit id) and `is_dirty: true` | P3 1.2 | `src/lab/manifest.rs:101-105` | Verified |
| 32 | JWT: HS256 only; 10 s leeway | P3 1.1 #1 | `src/security/jwt.rs:150-155`, `:169-172` | Verified |
| 33 | `NODE_FAILURE` sets node id 2 to Failed (logical flag) | P1 row 17, P4 1.2.8 | `server.rs:1086-1091` | Verified |
| 34 | `require_observer_if_secure` gates on `require_auth_for_control` | P2 5 | `server.rs:337-344` | Verified |
| 35 | Lab schema `lab-1`, 16 SSE streams, 15 s heartbeat | P4 1.1.1 | `src/api/lab_api.rs:50,59,60` | Verified |
| 36 | `Workload` has no tenant, rate-step or key-group field; `GatewaySpec` has one exe and one origin | P4 1.1.5, 1.1.6 | `src/lab_live/loadgen.rs:20-30`; `gateway.rs:20-28` | Verified |
| 37 | `SimDemo.jsx` is 517 lines with its own `EventSource` and a POST to `/api/control` | P4 1.3.4 | `ui/src/SimDemo.jsx:92,177`; `wc -l` | Verified |
| 38 | `ring` 0.17 is a runtime dependency; `rcgen` is dev-only | P1 4.5, P3 2.5 | `Cargo.toml:18,24,26-28` | Verified |
| 39 | README opening text | audit F7, section 1 | `README.md:9` | Verified |
| 40 | AC3-4 and the other AC3 wording; AC5-13 wording | audit F1, STATUS | `phase_3_execution_package.md:96-104`; `phase_5_execution_package.md:114-127` | Verified |
| 41 | STATUS test baseline 851/0/8, 75 targets | P2, P1 | `docs/STATUS.md` section 3 | Verified |
| 42 | Diagnostics ring capacity 512, `try_lock` drop; five diagnostic event names; no trace id | P4 1.2.3-1.2.7 | not opened by me | Not verified (carried from P4) |
| 43 | Timing/side-channel and shard-lock claims (shards chosen by whole-key hash, `cache.rs:544-550`) | P3 1.3 | not opened by me | Not verified (carried from P3) |
| 44 | Durable-log patch contents and the reproduction results | P1 6.1 | not opened by me | Not verified (carried from P1 and `reproduction_report.md`) |
| 45 | Design-resource facts (SHAs, licences, file sizes) | P4 5.1 | not fetched by me | Not verified (carried from P4; tool-summarised) |

**Tally.** 45 claims listed: 39 verified (row 2 with the stated residual that runtime behaviour was not traced), 1 wrong (row 29, P2), 1 outdated document statement (row 30, CLAUDE.md), 4 not verified (rows 42-45, carried from drafts and marked as such). No line-number discrepancy found in P1's citations. I found no other discrepancy in the claims I opened.

---
*End of document. This file is a proposal; it changes no code or record and approves nothing.*

---
## Correction note (2026-10-30, lead, after the DR-029 investigation)
Where this document and its drafts call the cluster GET stub "cache poisoning by an authenticated member": the investigation (docs/security/investigations/DR-029_cluster_get_cache_poisoning_investigation.md) found that the fabricated entries are stored in the non-'/' __default__ namespace and are **not** reachable by HTTP clients (peer keys starting with '/' are rejected, node.rs:419; HTTP keys always start with '/', server.rs:1249-1252). The accurate statement is: a valid cluster member can store fabricated entries and consume cache capacity through the cluster port; HTTP-visible poisoning was not demonstrated. The remediation proposals (disable or fail-close the stub on shared-cache nodes, then replace it by the fill protocol) are unchanged but remain owner decisions.
