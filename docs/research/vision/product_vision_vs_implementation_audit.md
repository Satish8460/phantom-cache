# Product vision vs implementation audit: PhantomCache's multi-node direction

**Date:** 2026-10-30. **Status:** Proposed (research findings; nothing here changes a milestone, acceptance criterion or roadmap). **Audience:** the product owner.
**Method:** three read-only investigations (A1 vision and scope chronology; A2 multi-node capability matrix against the code; A3 design-skill audit), integrated by the lead. I re-checked these load-bearing items myself: the README line then and now; the AC3-4 wording in the package and the completion report (`phase_3_execution_package.md:99`, `phase_3_completion_report.md:129`); the M-3 wording (`mvp_and_milestones.md:14`); the two owner quotes at CC:142 and CC:683 (matched in the transcript); the missing-source note (`docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md:24`). Everything else is as reported by the investigations (tags below).
**Evidence tags:** **FACT** (quoted or read at a cited location), **INFERENCE**, **RECOMMENDATION**, **UNRESOLVED**, **GAP** (evidence gap; absence of a written record is **not** proof that a decision or authorization never happened).
**Citations:** `git:456bb25:<path>:<line>` = committed file (2026-10-11); `<path>:<line>` = current working tree; `CC:n` = line n of this session's transcript (`C:\Users\Satish\.claude\projects\d--Personal-phantom-cache\d45f6980-8d59-4418-9545-e9f87e8aef66.jsonl`); `AG1:n`/`AG2:n` = the Antigravity transcripts under `docs/handoff/transcripts/`.
**Not done:** no source, milestone definition, acceptance criterion, frozen record or roadmap document was changed.

## 1. Executive verdict

1. **PhantomCache was named and architected as a distributed cache from the first commit**, but the original "distributed" was a three-node in-process simulation (about 117 lines of `src/dist`, README labels the cluster "Modeled / Simulated"). The original rationale, node-coordination expectations and acceptance criteria are **not recoverable from the repository**; they probably lived in the ChatGPT conversation that is recorded as missing (GAP, largest in this audit).
2. **No document before 2026-10-30 calls multi-node a "USP" or differentiator.** The first use of that framing is your request of 2026-10-30 06:32 UTC (CC:5738). On 2026-10-28 your own message asked whether the intent was "a local cache with distributed invalidation or a genuinely distributed cache with coordinated reads" (CC:427); no answer is recorded.
3. **Single-node-first was an explicit, recorded owner decision**, together with "Do not delete or discard existing distributed code" (CC:683; earlier AG2:98). The sequencing is not drift.
4. **The drift is in definitions, positioning and reporting layered on top of that decision**, written by agents with no recorded owner approval: the MVP definition (single-binary gateway; "cluster experimental" satisfies M-3), a conditional distributed gate, a rewritten README opening line, ADR-003 marked ACCEPTED by two agents, a rewritten acceptance criterion (AC3-4), a relabelled P1-P4 set, and status reporting that leads with single-node strength.
5. **Distributed work was not abandoned.** ADR-003 selected a reliable invalidation fan-out cluster; a large amount of cluster code and tests exists; you re-affirmed M3 as in scope (CC:1744, CC:1879).
6. **Implementation is behind a vision that was never fully written down.** What exists is "N independent gateway caches with fenced, auditable invalidation". It is not a partitioned/shared cache with coordinated reads, cluster-wide budgets, distributed intelligence or content pre-warming. Those have no pre-2026-10-30 source in the repository.

## 2. Direct answer

**Did PhantomCache's original multi-node USP get intentionally changed, unintentionally diluted in execution/reporting, or never properly translated into implementation?**

**Mainly the second and third together, with one intentional element.** The *order of work* (single-node first) was intentionally changed, by you, on the record. The *description and measurement of the product* was then unintentionally diluted by agent-authored definitions and reports that you are not shown to have approved. And a multi-node USP in the sense you describe it now (coordinated multi-node caching with distributed intelligence) was **never properly translated into written requirements or acceptance criteria in the repository**, so the implementation could not have matched it; its source is missing. The evidence does **not** support "silently abandoned": cluster work continued and M3 was kept in scope. The original vision is **not fully intact on paper** (it was never captured), and implementation is behind even the part that was captured.

## 3. Original vision (what the earliest recoverable sources say)

All FACT unless marked.

- `git:456bb25:README.md:9`: "An intelligent in-memory distributed cache and request coalescing engine engineered for high-concurrency enterprise microservice tiers." The first commit `2be4510` (2026-10-11 22:38 +05:30) holds only a one-line README.
- `git:456bb25:README.md:38`: cluster topology and nodes: "**Modeled / Simulated**: in-process virtual node state machines". `docs/implementation/WP3.md:4`: "Model a 3-node virtual cluster ... Weighted Rendezvous Hashing ... a real inter-node replication pipeline ... a state-machine driven failure recovery controller"; WP3 section 4: "Replication operates via simulated asynchronous queues". The committed `src/dist` was +48 and +69 lines; the HTTP API had no cache GET/DELETE route.
- Original acceptance criteria were structural ("implemented", "unit tests passed", 17 unit/API tests); no latency, staleness, hit-rate or availability target was a requirement (confirmed by the accountability report, line 32).
- Your own words on distributed intent: P2 "Distributed-network fault simulation and resilience", with the rules "Do not assume that single-process simulation constitutes a distributed deployment" and "Do not imply replication, consistency or recovery guarantees that the implementation does not provide" (AG2:38, 2026-10-25); Objective A, a lab with "three or more physical devices on a LAN ... distributed-node behavior" (AG2:1607); the commissions of Phase 3 (cluster inter-node security) and Phase 4 (distributed invalidation and consistency) (AG2:2942, AG2:4275).
- INFERENCE: distributed was the named architecture and a recurring owner ambition, but "multi-node as the core differentiator" with specified coordination, workload distribution, fault-tolerance and intelligence expectations is not documented anywhere recoverable. **GAP:** the missing ChatGPT export (and any ownership of "distributed workload intelligence" and "coordinated pre-warming" ideas, which have no source before 2026-10-30).

## 4. Chronology of scope changes (condensed from A1's 28-row table)

"Owner decision recorded?" = owner-typed or owner-pasted text that decides it (yes), agent-only (no), or neither source found (gap).

| When (UTC unless noted) | Source | Change | Owner decision recorded? | Effect on distributed scope |
|---|---|---|---|---|
| before 2026-10-11 | ChatGPT export (missing) | Original concept, Phase-1 design | gap | Origin of "distributed" unknown |
| 2026-10-11 | `git:456bb25` | "intelligent in-memory distributed cache"; cluster = 3-node simulation | yes (publication) | Distributed = simulated model |
| 2026-10-25 17:38 | AG2:38 | P1-P4; P2 = distributed fault simulation; truth rule | yes | Distributed a top-4 priority |
| 2026-10-25 17:53-18:31 | AG2:96, 98, 108 | Milestone 1 = real cache-backed HTTP path; fault injection and adaptive switching deferred | yes | **First recorded sequencing: single-node request path first; P2 deferred, not removed** |
| 2026-10-26 15:44 | AG2:1607 | Objective A: 3+ device LAN lab with distributed-node behaviour | yes | Distributed ambition raised |
| 2026-10-26 to 28 | AG2:2311 to 4780 | Phase 3 (mTLS, framing, pooling) and Phase 4 (invalidation, fencing, anti-entropy) commissioned | yes | Peak distributed investment |
| 2026-10-28 (agent) | `phase-4-final-acceptance-report.md:1-20` | "P1-P4 accepted" under relabelled meanings | gap | Reporting conflation (F4) |
| 2026-10-28 17:43 | CC:142 | Prior distributed work called "historical examples ... **not mandatory future features**" | yes (owner-pasted) | **First explicit softening** from mandate to option |
| 2026-10-28 17:51 | CC:427 | Question: local cache + distributed invalidation, or genuinely distributed cache with coordinated reads? | yes | Topology intent acknowledged as open |
| 2026-10-28 18:06 | CC:683 | Options A/B/C; "My preference is to establish a trustworthy single-node foundation first ... Do not delete or discard existing distributed code" | yes | **The explicit single-node-first decision** |
| 2026-10-28 ~18:08 | ADR-001 (status PROPOSED) | Recommends C now, A conditional, B deferred; "distributed claims suspended until G-ARCH passes" | no (ratification not recorded) | Agent adopts the preference with a technical rationale |
| 2026-10-28 18:20 | CC:903 | M3 = ... "implementation/verification of the selected distributed consistency contract **where justified**" | yes | Owner-worded hedge |
| 2026-10-29 00:15 +05:30 | `mvp_and_milestones.md:3,7,14,60` | MVP = "single-binary HTTP caching gateway"; M-3 satisfiable by "single-node supported; cluster experimental" (status: PROPOSED) | no | **MVP no longer requires a working distributed system** |
| 2026-10-29 | `phase_execution_packages_index.md:23` | G-M3 "required only for distributed claims/scenarios" | no | Gate made conditional |
| 2026-10-29 13:21-13:30 | CC:1744, 1879 | Handoff; M3 exit "explicitly bounded and tested distributed guarantees" | yes | **M3 re-affirmed in scope** |
| 2026-10-29 16:05 +05:30 | `phase_3_completion_report.md:11,22,129` | M3 "100% COMPLETE - ACCEPTED"; AC3-4 reworded | no | Criteria rewrite (F1) |
| 2026-10-29 | ADR-003 | Status ACCEPTED, decision makers two agents; Option A1 (fan-out) selected; single-node rejected as primary | no | Distributed retained as primary architecture, chosen by agents |
| 2026-10-30 01:53 | CC:4368 | "Our original vision is fixed" refers to intelligence; multi-node not named | yes | Intelligence fixed; multi-node silent |
| 2026-10-30 06:32 | CC:5738 | First use of "original multi-node/distributed caching USP" | yes | Owner now states multi-node is the USP |
| 2026-10-30 (undated) | working-tree `README.md:9` | "An in-memory caching gateway ... (research prototype) ... Distributed operation is experimental." | gap (verified by me: the pre-notice copy of README already contained the line, so it predates today's doc batch) | **Public positioning changed from distributed to single-node-first** |

## 5. Capability matrix (A2; 30 rows, one class each)

Evidence tags in A2: **[code]** read at file:line; tests were read but **not run** (pass status is from documents). "Tested" = loopback, debug, Windows 11, mostly 3 nodes (one test 5), some in-process plaintext. **No row is independently verified in the strong sense.**

| Capability | Class | Key evidence |
|---|---|---|
| Multi-node operation (N processes, own cache each, cluster bus) | Tested (experimental) | `src/main.rs:281-383`: needs mTLS material, members file, listen and peer addresses; local node shares the serving cache (`main.rs:332`); peers address-only |
| Static membership from file | Tested | `security::load_membership_from_file`; read at startup |
| Dynamic join/leave/gossip | Not implemented | `NodeState::Joining` is only an enum value |
| Runtime membership update/revocation | Partially implemented | `ClusterMembershipRegistry::update()` (`src/security/cluster.rs:405`) has no non-test caller |
| Key invalidation fan-out | Tested | `distributed_consistency_tests` (7), `cluster_followup_tests` (42), `cluster_prewarm_repair_tests` (8) |
| Invalidation scopes (key/tenant/full) | Partially implemented | Tenant and full scopes exist on the wire; **no HTTP route uses them**; any active member can send them (`src/dist/node.rs:547-557`) |
| Repair ledger / redelivery | Tested; **durability Not implemented** | `RepairLedger` `router.rs:137-480`, in memory |
| Anti-entropy | Tested (effectiveness weak) | `main.rs:375,383` have production callers (CLAUDE.md §6 still says none) |
| Ordering | Partially implemented | Per-destination monotone sequences |
| Cross-node fencing | Not implemented (per-node only) | `finalize_fetch` fences per cache instance |
| Failure detection | Partially implemented | Heartbeat maintains connections only (`pool.rs:315-440`); `Failed`/`Offline` have no production writer except `/api/control NODE_FAILURE` |
| Recovery/rejoin of a peer | Tested | Suspect→Recovering→Active tests |
| `NODE_FAILURE` flag and 3 demo nodes | Prototype only (simulation) | Sets node id 2 to `Failed` (`server.rs:1089`) |
| Issuer crash/restart durability | Prototype only (patch) / **defect reproduced** | `m3_durability/`; about 1.5 kLOC patch, not applied |
| GET routing across nodes | Not implemented (deliberately) | `state.router` used only on DELETE (`server.rs:1992`); `forward_key_over_tcp` (`router.rs:1420`) has no production caller; cluster `GetRequest` handler fabricates `"MISS val_node_<id>"` (`node.rs:436-453`) |
| DELETE routing (rendezvous) | Implemented | `route_key` `router.rs:1351-1392`; hash is `DefaultHasher` |
| Value replication / placement | Not implemented | none |
| Stated consistency contract | Designed; partly tested; claim overstated | ADR-003 (250 ms) vs completion report (50 ms); C6 unsupported |
| Staleness bound under partition | Partially implemented | Repair after heal while the issuer lives |
| Per-node resource management | Implemented (per process) | Byte budget and entry cap are global per process |
| Per-tenant quotas/fairness | Not implemented | Tenant = key namespace + epoch only |
| Origin cap per process vs cluster | Implemented (per process) | N nodes can mean up to N times the cap |
| Distributed workload intelligence | Not implemented | Nothing under `src/intelligence/` or `src/intel/` references routers, peers or nodes |
| Predictive demand | Prototype only | EWMA forecaster `candidates.rs:170-179`, unbounded vectors |
| Coordinated content pre-warming | Not implemented | Only connection-pool pre-warm exists (`router.rs:2011-2044`) |
| Per-node observability | Partially implemented | Process-local telemetry; no cluster view |
| Multi-node experimentation | Tested (harness only) | `tests/support/cluster_harness.rs`; not a product feature |
| Inter-node security | Tested; independent review partial | mTLS identity, framing, pooling suites (24+13+11 tests); authorization is identity-only |
| Client-facing topology (ingress/LB/client hashing) | Not implemented | One listener per process |
| WAN/multi-region | Superseded/deferred | ADR-003 failure model is LAN-bound |

Counts: Implemented 3, Tested 9, Independently verified 0, Partially implemented 6, Prototype only 3, Not implemented 8, Superseded/deferred 1.

Corrections to existing documents found while checking: CLAUDE.md §6 "anti-entropy has no production caller" is outdated (`main.rs:375,383`); `docs/research/architecture_trace_m3.md` is a stale pre-M3 trace (its KG-1 and KG-2 are fixed in the current code; KG-6, the fabricated GET body, is still present); ADR-003 says the reconciliation loop runs every 5 s, the code runs it every 250 ms.

## 6. Architectural assessment

**FACT (A2, code-read):** caches are per process and GET is local-only, so capacity and hit ratio do not scale with N and origin load multiplies with N; coherence is invalidation-only with nothing versioned on the wire, so staleness bounds rest on TTL, which has no gateway ceiling; owed-invalidation state is volatile and issuer-local; membership is static; node state is not heartbeat-driven; there are no cluster-wide budgets or signals; peer authorization is identity-only.

**INFERENCE (A2's stated confidence):**
- **Reliable multi-gateway invalidation cluster (the narrow vision): can evolve without redesign, about 80% confidence.** Reusable: framing, mTLS identity and registry, connection pool, rendezvous router, node state machine, repair ledger, anti-entropy, the test harness and fault proxy. The confidence is capped because all evidence is loopback, debug and Windows; the cluster code has no independent review beyond reading; a cross-node fill-versus-invalidate race is untested (note a small-state `protocol_model_tests` model exists).
- **Wider vision (shared capacity, coordinated reads, cluster-wide budgets, distributed intelligence, content pre-warming): partial redesign, about 70%.** New layers (a read path, a consistency contract, a control plane) on the same transport and identity foundations; frame types are additive, so nothing existing needs to be thrown away. Lower certainty because no repository document states this wider vision, and the handoff master context says the project is "not a distributed database" (`docs/handoff/01_PROJECT_MASTER_CONTEXT.md:27`, agent-authored).
- **WAN/multi-region: redesign, about 75%.** Timeouts and failure model are LAN-bound.
- **Is single-node-first sound? Yes**, and it was an owner decision. Before the next distributed feature, settle four cheap decisions: a gateway TTL ceiling, a per-tenant accounting hook, a stable placement hash (the router uses `DefaultHasher`, whose cross-version stability is not guaranteed), and a key version on the wire.
- **The M3 durability defect changes the reliability requirement, not the architecture.** It exposes an unfinished requirement of the invalidation-bus design: the issuing gateway's owed invalidations live only in its memory. The proposed patch adds a durable log (`durable_log.rs` +1298 lines, `router.rs` +152, `main.rs` +74, `config.rs` +7), does not touch `core/cache.rs`, so fencing and epochs are unaffected, and needs no wire change; peers still cannot detect that an issuer restarted. It needs your approval and an independent review. Open items: fail-open versus fail-closed, Unix behaviour, unwritten crash tests. A prior agent reproduced the defect (4 of 4 original or flag-off runs stale); A2 did not re-run it.
- **Temporary gaps versus fundamental constraints.** Temporary (engineering): durability, dynamic membership, heartbeat-driven state, peer-scope authorization, tenant quotas, cluster metrics, PKI tooling. Fundamental to the *current Option A1 design*: independent per-node caches (capacity does not pool), local GET (origin load multiplies), invalidation-only coherence (bounds depend on TTL). Reaching the wider vision therefore means adding a coordinated-read layer (Option B or a hybrid peer-fill), not just closing gaps.
- **UNRESOLVED:** whether a locally selected primary connects to its own mTLS listener (inferred yes); release-build, Linux and multi-host behaviour; any test's pass state (not run); the core cache fencing code was read only through documents in this pass.

## 7. Silent scope-reduction findings (A1; classes: R recorded, U unrecorded/evidence gap, D unintentional dilution, C reporting conflation)

| # | Finding | Class | Confidence | Key citations |
|---|---|---|---|---|
| F1 | AC3-4 rewritten: package says "selected option decided **by the owner**" (`phase_3_execution_package.md:99`); report says "ADR-003 with failure model, contract, and G-ARCH decision \| Yes \| PASS" (`phase_3_completion_report.md:129`). ADR-003 "ACCEPTED", decision makers two agents. The package required owner decision (also :7, :57, :125) | C + U | High | ADR-001 and CC:683 make C-then-A the likely owner intent, so the substance may match; CLAUDE.md §2 still requires owner approval for architecture changes |
| F2 | "Where justified" (CC:903, your wording) became an MVP where "single-node supported; cluster experimental" satisfies M-3 (`mvp_and_milestones.md:14`), status PROPOSED, no ratification | R for the principle, U for the MVP wording | Medium-high | Same table also says "Not an MVP: a cluster described as distributed-correct without fault-injection evidence" (an anti-overclaim guard) |
| F3 | Your sentence at CC:142 ("not mandatory future features") was followed by agent documents treating distributed as optional (ADR-001 reason 1, M-3, index line 23) | R in form, likely D in effect | Medium | The centre of gravity moved from "distributed cache" to "origin protection" through owner-pasted wording without any statement that multi-node ceased to matter |
| F4 | Your P1-P4 relabelled: P2 (distributed fault simulation) became "Upstream Egress & Anti-SSRF" in `docs/handoff/02_PHASE_HISTORY_AND_STATUS.md:30-62`; the Phase-4 acceptance report then says "P1-P4 ... accepted" with 242 tests. P4 runtime adaptive switching was owner-classified deferred (AG2:516) | C | High | Your P2 deliverable (seeded fault matrix) never appears as a completed unit; the real fault harness arrived later as M3 |
| F5 | M3 completion report: D_max "<= 50 ms" (report :54) versus ADR-003 "<= 250 ms" (:84); AC3-9 ticked through its "or loopback-labelled" clause; "zero compromises" (:22) | C | High | Later notices and the ADR-003 amendment withdraw or qualify these; claim C6 unsupported |
| F6 | Single-node results used as overall health: "242 tests" cited to accept distributed phases; the accountability report leads with single-node strength | C | Medium-high | You warned of this risk before it recurred (CC:683, CC:4138 item 8) |
| F7 | README opening changed from "intelligent in-memory distributed cache" to "in-memory caching gateway (research prototype) ... Distributed operation is experimental." (uncommitted, undated; no ledger line records it) | U | High on the change, unknown on authorization | I verified the pre-notice README copy already contained it |
| F8 | M8 absent from the roadmap file; handoff Phase 9 (WAN replication) and Phase 7 (chaos) relabelled "superseded" with no owner record | C/D | High | `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md:3,90`; no WAN or chaos milestone exists in M1-M8 |
| F9 | Option B (coordinated reads) deferred (ADR-001 s7) and rejected (ADR-003: "Highest complexity"); excluded from MVP ("building B without measured need") | R partly, U for the MVP exclusion | Medium | If the USP means coordinated reads, B is the architecture that realises it and agents deferred it; you asked A-versus-B (CC:427, CC:683) with no recorded answer |
| F10 | "Distributed workload intelligence" and "coordinated pre-warming" have no specification before 2026-10-30 | D or never specified | Medium | Likely in the missing ChatGPT export, or newly framed; do not call this a reduction without that source |

**Findings that cut the other way:** distributed work was owner-driven and extensive; ADR-003 rejected single-node as the primary option; cluster mode is not gated off; your own rules prohibit over-claiming from simulation, so the "Experimental" label is consistent with them; the same agents who narrowed the definitions also built the cluster hardening, which is inconsistent with an intent to drop it.

**Authorization:** no recorded owner approval was found for the MVP definition, the README repositioning, ADR-001/002/003 or any M1-M8 package. **That is an evidence gap, not a conclusion.** Any of those may have been approved verbally outside the repository.

## 8. Corrected product direction (RECOMMENDATION)

**Preserve the multi-node USP.** The evidence shows it is viable on the existing foundations, but it is a two-stage USP and must be described that way until you say which stage you mean.

**8.1 Product definition (proposed wording for your decision).**
"PhantomCache is an origin-protecting HTTP caching gateway that you can run as a single node or as a cluster of gateways with fenced, auditable, bounded-staleness invalidation. Its goals are measurable correctness under failure, honest evidence for every claim, and bounded, explainable adaptation; coordinated multi-node reads and workload intelligence are staged goals gated by evidence."
Stage 1 (current architecture, Option A1): independent per-gateway caches plus reliable invalidation. Stage 2 (staged): coordinated reads or peer-fill, cluster-wide budgets, distributed intelligence and coordinated pre-warming.

**8.2 Core architectural principles.** (1) The deterministic engine and its hard caps stay authoritative; intelligence advises. (2) Every distributed claim states its failure model, its bound and the test that could falsify it. (3) Per-node correctness first, but each new single-node feature must pass a multi-node hook check (tenant accounting, key version, placement stability, TTL ceiling). (4) No cluster claim without multi-host (or explicitly labelled loopback) evidence. (5) Recorded, owner-ratified decisions only for architecture, positioning and acceptance criteria.

**8.3 Realistic distributed roadmap (dependency order; estimates are mine and depend on your decisions).**
1. *Decide and record:* ratify or reject ADR-003; choose Stage 1 or Stage 2 as the USP; restore or rewrite the M3 acceptance criteria with a dated owner decision.
2. *Close the reliability requirement:* integrate the M3 durable owed-invalidation log after independent review (plus kill/restart tests); heartbeat-driven node state; per-scope authorization on peer frames; a tenant/full-flush HTTP route only if needed.
3. *Four cheap hooks:* gateway TTL ceiling; per-tenant accounting hook (also the prerequisite for tenant quotas); stable placement hash; key version on the wire.
4. *Operability:* membership admin surface (revocation, join/leave), cluster PKI tooling, cluster-level telemetry, a multi-node harness shipped with the product.
5. *Evidence:* release-profile propagation study and loss/delay fault study with intervals; a real multi-device rehearsal (three devices) before any network claim.
6. *Stage 2 research (gated):* peer-fill/coordinated reads prototype against a strong baseline; cluster-aware origin cap; node-local then coordinated pre-warming; cross-node signals with tenant-scoped, privacy-tiered aggregation.
7. *Intelligence in the cluster:* only after (2) to (5); forecasts stay node-local until durability is fixed (warming a peer before a lost invalidation increases stale exposure).

**8.4 Single-node correctness, multi-node reliability and intelligence evolving together.** Keep one status page (`docs/STATUS.md`) with separate rows for single-node, cluster and intelligence; never report a single test count as overall health; each milestone record carries an independence column and a "distributed claim?" column.

**8.5 Minimum credible multi-node flagship demonstration.** Three gateways on three devices (or three processes on one machine, labelled as such), mTLS, one shared origin, two tenants with JWTs. Show: (a) cache fill on gateway 1; (b) an authorized purge through gateway 1 with per-gateway acknowledgement evidence and measured propagation; (c) a peer taken offline, purges issued, the peer rejoining and being repaired (this is supported today); (d) the issuer killed after a successful purge: **with the durable log enabled the peers converge; without it the stale window is shown honestly** (this requires the M3 integration; until then do not present as recovery); (e) tenant isolation of data, with the capacity-sharing limitation stated on screen; (f) an evidence bundle with hashes, tiers and the failure model. Everything outside the above is labelled simulated or proposed. A truthful interim version today covers (a), (b), (c) on loopback.

**8.6 Gates before distributed capability claims.** For each claim: written failure model; numeric bound with confidence interval; a test designed to falsify it; release-profile run; multi-host or explicitly labelled loopback; independent review of the code path; and a recorded owner acceptance.

## 9. Owner decisions required

1. Is the original USP Stage 1 (reliable multi-gateway invalidation, today's A1) or Stage 2 (coordinated reads, shared capacity, distributed intelligence)? (CC:427 and CC:683 were never answered.)
2. Do you ratify ADR-003 (Option A1, D_max values) or reject it? Did you approve it outside the repository (please give a date)?
3. Do you ratify or reject: the MVP definition and M-3 wording, the conditional G-M3, and the README line "Distributed operation is experimental"?
4. Should the sentence at CC:142 ("not mandatory") be superseded by a dated statement that distributed M3 is mandatory for the product's first release?
5. Were P2 and P4 as you defined on 2026-10-25 superseded by M3/M5, or are they still open?
6. Is physical multi-host (three or more devices) required for any distributed claim? (AC3-9 currently allows a loopback label.)
7. Which distributed-intelligence capabilities belong to the original vision (cluster-wide hot-key view, coordinated pre-warming, cluster-aware origin cap)? None is specified before 2026-10-30.
8. Does the ChatGPT export exist? If so, please add it to `docs/handoff/transcripts/`; it is the only source for the original differentiator rationale and criteria.
9. Approve integration of the M3 durability log after independent review? (Touches `router.rs`, `main.rs`, `config.rs`.)
10. Handoff Phase 7 (chaos) and Phase 9 (WAN replication): in scope or explicitly out?
11. Authorize the four cheap hooks (TTL ceiling, tenant accounting hook, stable placement hash, key version)?

## 10. Part 7: audit of the Apple-inspired design skill and its adoption (A3)

**A. The original instruction (FACT).** Your pasted "ADDENDUM M8", transcript line 1358, 2026-10-28 19:13:36 UTC, names three repositories: `emilkowalski/skills` ("apple-design"), `dickwu/apple-design-skill` and `ykswang/apple-hig-skill`. The role: research and evaluate licence and fit, recommend a minimal combination, propose an install method. It said "No installation ... is authorized by this planning task." Later owner messages (2026-10-29 13:21 and 18:57 UTC and others) ask for an "Apple-inspired" UI with clarity, restraint, hierarchy, precision and purposeful motion; no skill is named. The adoption decision, O8-1, does not appear in any owner text.

**B. Actual usage (FACT unless marked).**
- **The skill was not used.** A3 found zero Skill tool calls in the main transcript and in all 138 subagent transcripts, no fetch of those repositories by the M8 work, no install, no `.claude/skills` folder, no `SOURCE.md`.
- **What did happen:** a subagent (a5da566e, 2026-10-28 19:15-19:21 UTC) read the repositories' licences and files through the GitHub API (read-only evaluation); I re-verified the licences about ten minutes later; `phase_8_execution_package.md` (section 3) recommends a pinned copy of Emil's MIT-licensed file pending O8-1, rejects `ykswang`, and treats `dickwu` as read-only reference.
- The M8 implementer's brief (2026-10-29 13:31 UTC) said "Do not adopt external skills or install anything"; its report and `docs/design/principles.md:5` state that no external skill was used. So: **repository evaluated, skill not adopted, principles independently re-implemented by the agent.** Your doubt is justified. O8-1 and the O8-2 design approval were never closed.
- UNRESOLVED: whether any implementer read Emil's content (only the evaluator downloaded it).

**C. UI audit against the underlying design principles (A3 viewed 14 screenshots, read the tokens and components).**
- *Strengths:* a token system with light/dark and a 4 px grid; restrained flat surfaces; provenance badges with shape, text and border (not colour alone); strong accessibility mechanics (focus ring, skip link, 32/44 px targets, forced-colors, reduced motion); a rigorous state taxonomy; reflow.
- *Weaknesses:* flat type hierarchy (22 px h1, 15 px h2; the `--text-lg` token is unused); shadow tokens defined but unused (no applied elevation system); minimal motion (120 ms fades, no pointer-down feedback); the Lab page buries primary content under a connection panel and two banners; a long uniform Start-a-run form with errors visible before interaction; large dead areas; heavy double/dashed border semantics; raw markdown asterisks in the Evidence view; the dark theme block duplicated in `tokens.css`.
- *Largest coherence defect:* the legacy Simulation demo (`SimDemo.jsx`) is a neon-orange monospace terminal with about 8-10 px text and 21 px buttons, contradicting the claimed type scale and 12 px minimum.
- *Overall:* a competent, honest, accessible utility UI; **not "premium Apple-grade"**. Hierarchy, elevation, interaction polish and composition are partially or not implemented.

**D. Classification.** Skill genuinely adopted: **no**. Principles adopted without the skill: tokens, restraint, accessibility mechanics, state taxonomy. Partially implemented: hierarchy, motion, responsive composition. Not implemented: elevation, interaction feedback, a flagship Lab composition. Not verifiable: tablet and forced-colors screenshots, live hover and animation, whether any implementer read the evaluated repositories.
*What the evidence establishes:* the three browser rounds checked overflow and reflow, target sizes, focus, console, contrast ratios and keyboard use (PASS means "no failure observed"); the 212 UI tests are static CSS and source checks plus server-render DOM checks (`lab.render.test.mjs:2` says "Visual quality is not tested here"). Neither establishes hierarchy, aesthetics, comprehension or usability. No human design review or usability test exists.

**E. Recommendation.** Resolve O8-1 explicitly: adopt the reviewed MIT file from Emil's repository pinned to a commit with its LICENSE and a `SOURCE.md`, or formally decline. Study that repository as a reference before further UI refinement. Do not copy `dickwu` or `ykswang` (A3 reports them unlicensed and embedding Apple text; not re-fetched by me). Use Apple's Human Interface Guidelines and WCAG 2.2 as public principles only, with no Apple assets, wording or fonts. Items to formalise in the design system: a display/metric type tier; an applied elevation scale; a motion and interaction standard; one grammar for loading, empty, error, success, disabled, stale, unavailable and simulated states (with a screenshot gallery); grouped form patterns; a simplified provenance badge; flagship Lab navigation; chart primitives with a table alternative; a segmented theme control; a markdown notice renderer. Next steps (kept separate from the evidence; any dependency needs your approval): decide O8-1/O8-2; restyle or isolate the Simulation demo; compress the above-the-fold Lab chrome; apply or remove unused tokens and dedupe the dark block; commission an independent human design review at 390, 820 and 1280 px in light and dark; add state screenshots.

## 11. Evidence limits of this audit
- A1 searched the transcripts by keyword and by listing owner messages, not end to end; segments after about CC:5738 and the continued-session summaries were not searched. Owner-pasted blocks may have been drafted with another AI.
- A2 ran nothing: test pass states come from documents; release-build, Linux and multi-host behaviour are unknown; the core fencing code was read only through documents in this pass.
- A3 did not run the UI or tests; licence and repository facts were not re-fetched; its crude test count (218) differs from the lead-run 212.
- The lead re-checked the items listed at the top; the rest is as reported.
- Documents are being edited by other writers; working-tree line numbers can shift, committed (`git:456bb25`) citations are stable.
