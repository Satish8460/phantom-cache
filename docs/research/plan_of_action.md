# PhantomCache — Integrated Plan of Action (POA) v2

**Status: PROPOSAL; nothing here is authorized beyond documentation and read-only investigation.** No application code has been changed, nothing installed, nothing committed. `CLAUDE.md` was created with approval.
**Supersedes** POA v1. Inputs: `code_findings.md`, `architecture_investigation.md`, `literature_review.md` (rounds 1–2; one slice pending), `candidate_comparison.md`, `hypothesis_and_experiments.md`, `claims_ledger.md`, `baseline_and_profiling_plan.md`, `skill_strategy.md`.

> **Roadmap update:** the canonical roadmap is now the seven-milestone plan (Rev 2: Phantom Intelligence & PIE inserted as Milestone 5) targeting the MVP in `mvp_and_milestones.md`, with execution detail in `implementation_master_plan.md` and `phase_1_execution_package.md`. The stage names below (S0…P8) are the earlier strategy view and are retained for traceability; where they differ, the milestone plan wins.

## 0. Guiding principle
Build a real system; let the paper prove only what the system demonstrably achieves. Every workstream is tagged:
- **[P] Product** — worth doing for the system regardless of the paper.
- **[R] Research** — exists to test a hypothesis; developed in an *isolated harness* and integrated into production only if it earns it.
- **[S] Shared** — serves both (build once).

## 1. Where we actually are (facts that shape the plan)
1. The single-node serving path (sharded cache, single-flight coalescing, generation fencing, ingress/egress security) is substantive and well tested.
2. The distributed layer is a well-tested **protocol library that is not wired to the serving path**: GET is local-only; peer invalidations purge a per-node side cache rather than the HTTP cache; reconciliation has no production caller; outbound history is not recorded; staleness is bounded only by a fixed 60 s TTL (`architecture_investigation.md`).
3. The adaptive layer is a threshold trigger, the sketch is unused, telemetry is partly synthetic and polluted, evictors are scans named after other algorithms (`code_findings.md`).
4. No measurement of the real hot path exists.
5. Literature: the origin-aware, contract-bounded controller idea survives round 2 narrowly; nothing found implements it, but the search is incomplete (`literature_review.md` §4b).

Implication: **truth and measurement first; repair or honestly scope the distributed layer; build the controller on the single-node path in an isolated harness.**

## 2. Long-term product vision (not limited to a paper) [P]
A trustworthy, efficient, secure caching tier for HTTP origins that: protects the origin under load and failure; serves correct, bounded-stale data with explainable behavior; reports only what it measures; runs single-node or as a coordinated cluster with stated guarantees; is operable (resource limits in bytes, observability, hot-reload, runbooks); and is usable through a UI that shows real state and supports reproducible experiments.
Post-paper ambitions are in §9.

## 3. Workstreams

### S0 Guard rails & skills [S] — days
`CLAUDE.md` (done). Next: benchmark-integrity checklist, claims ledger discipline (done), UI-telemetry contract. Acceptance: seeded-flaw checks in `skill_strategy.md` §7.

### S1 Measurement & truth foundation [S] — first, mandatory
- Hot-path microbenchmarks (release): hit, miss, coalesced, eviction latency distribution, epoch-lock scaling, telemetry cost, clone cost (`baseline_and_profiling_plan.md`).
- Telemetry truth contract: every metric carries `source ∈ {measured, simulated, inferred, unavailable}`; remove DELETE-as-miss pollution; per-key signal (the `key_hash`); throughput as a rate; byte occupancy.
- Reference implementations of SIEVE, S3-FIFO, LRU, TinyLFU in an isolated harness (compare production evictors against them).
- Seeded workload/trace tooling and a run manifest format.
- **Gate:** baselines reproduce within the pre-stated variation on two days; unlabeled synthetic fields = 0.

### P2 Cache-core engineering [P] — evidence-gated
Candidates, each justified only by S1 data: remove the global epoch-mutex dependency; queue-based eviction; byte-based capacity and memory accounting (current worst case is an arithmetic ≈1.3 TB bound); telemetry sharding/histograms; hasher/clone cost; retire legacy `Coalescer`/`legacy_shards`; parse `Cache-Control` (fixed 60 s TTL today). Acceptance: 242 tests still pass; deltas with CIs; fencing changes get independent review and a test that fails without the change.

### P3 Distributed correctness [P, with R potential] — decision first
Decision required (owners, informed by S1 and a fault-injection prototype): **(a)** local caches + working invalidation fan-out; **(b)** coordinated reads via HRW; **(c)** ship single-node honestly and gate the cluster layer as experimental. 
Whatever is chosen, mandatory regardless: (i) a multi-gateway fault-injectable harness with a regression test for the cross-gateway invalidation gap (A1); (ii) rewrite documentation to match behavior (the handoff "accepted / zero P0" claims are unsupported); (iii) per-peer sequence spaces, outbound history, a runtime reconciliation trigger, honest metrics/exports, bounded task fan-out, node-state recovery — only under (a)/(b). 
Research potential [R]: a measured staleness bound under injected loss/partition, a formal model of the fill-vs-invalidate race (TLA+ or similar), flush-metastability analysis (the literature shows the fencing/replay ideas themselves are known: Memcache leases, TAO versions, Redis placeholder, MuCache proofs). Claim C6 is rewritten after the decision.

### S4 Phantom Lab & origin model [S] — after S1
Deterministic lab (virtual clock, seeded RNG, replay, fault injection, parameterized origin models, multi-gateway), plus a real-process controllable origin for Tier 2. One harness, two tiers; simulation never reported as measurement. Reuses `sim/*` and the demo-lab designs. Acceptance: same seed → same output; ≥1 Tier 1 prediction compared against a Tier 2 run.

### R5 Origin-aware control [R] — isolated first
Developed in the harness against the full baseline set (`hypothesis_and_experiments.md`; now includes Envoy/Breakwater-style limiters, retry budgets, Varnish-style binary health, XFetch, Memcache lease/stale, Farahbakhsh remedies, and an oracle-tuned static baseline). Order: simplest clamped mechanisms first (miss shedding, stale extension, origin-concurrency cap), controller only if ablations show static tuning insufficient. Production integration only if C1–C4 reach SUPPORTED and the overhead is acceptable; feature-flagged with a runtime off-switch. The existing PIE is not extended; it is replaced or retired (decision after S1).
Alternatives kept alive: delayed-hit-aware coalescing (active literature, baseline), shadow-evaluated policy selection (crowded; ablation), RL (high risk).

### S6 UI & developer experience [P, supports R]
- **Now (with S1):** provenance badges; remove client-side metric arithmetic; configurable gateway URL (currently hard-coded to `localhost:8085`); unavailable states.
- **Next:** real hot-key/byte-occupancy panels once measured; request explainer (hit/miss/coalesced/rejected, with reason); config and topology view; diagnostics that match the real endpoints.
- **Research UI (lab):** experiment runner, configuration inspector, repeated-trial comparison with uncertainty, manifest export for the paper. Lives with the lab, not in the production UI.
- **Controller UI** (only if R5 is integrated): current policy, evidence, guard/rollback events, overhead.
- **Not worth building:** animated charts over synthetic data; dollar-savings figures without a cost model; per-node CPU/memory until measured.
- Acceptance: a test fails when a simulated value renders without its label.

### S7 Scientific evaluation [S]
As in `hypothesis_and_experiments.md`: baselines, ablations, ≥30 seeds, bootstrap CIs, sensitivity, negative results, Tier 1/2/3 separation, pre-registered thresholds. WSL2/Windows numbers never pooled with Linux-host numbers.

### P8 Deployment readiness [P]
Byte-based sizing, config hot-reload, cold-start behavior, multi-host validation (physical Linux hosts), observability export (Prometheus-style, matching real metric names), runbooks, key management. Existing security work (mTLS, SSRF defenses, JWT) reused. Also: RFC 9111 conformance question — the system is currently an explicit REST gateway, not a transparent HTTP proxy.

## 4. Dependency spine
```
S0 ─► S1 ─┬─► P2 (core, evidence-gated)
          ├─► P3 (distributed: decide, repair/scope)  ─┐
          ├─► S4 (lab) ─► R5 (controller, isolated) ───┼─► S7 evaluation ─► paper
          └─► S6 (UI: truth first, lab UI later) ──────┘
P3 and P2 ─► P8
```
Gate rules: no performance claim before S1 exits; no controller integration before S1+S4 exit and C1 evidence; no distributed guarantee claim before P3's decision and a failure-model-specific test.

## 5. Parallelism and interfaces
Parallel after S1: P2, P3, S4, S6. Agree up front: the telemetry/SSE schema with provenance tags; the experiment manifest format; the origin-model trait. Assign by module (cache core / dist / lab / UI); never two editors on one file; independent review for fencing/epoch/invalidation changes.

## 6. Shared vs isolated (avoid building twice)
- **Shared:** telemetry contract, workload/trace tools, manifests, the multi-gateway fault-injection harness, the origin model.
- **Isolated until proven:** controller prototypes, learned or shadow-selection policies, reference algorithm implementations used for comparison.
- **Production only:** repairs, byte accounting, lock/telemetry fixes, deployment tooling.

## 7. Paper plan
Select contributions only from `claims_ledger.md` entries at SUPPORTED. Provisional frame: *measured, contract-bounded origin-aware miss management evaluated against tuned static and adjacent-field baselines on metastable-style scenarios, with an open evaluation harness.* If C1 fails or narrows, fall back to: the harness and a measurement study of when origin-awareness helps; and/or a distributed-assurance paper (formal model + measured staleness bound) once P3 delivers. Draft related work and methodology early; results only from real evidence. Artifact: code, configs, seeds, raw data, manifests.

## 8. Phase mapping (history → v2)
| Historical | Status | v2 |
|---|---|---|
| P1–P2 (core, ingress/egress security) | tested; core caveats | P2 maintenance |
| P3 (mTLS, framing, pooling) | well tested; not wired to serving | P3 |
| P4 (invalidation, anti-entropy) | **not operational end to end** | P3 repair/scope |
| "Phase 5" soak testing | premature | S1 + P8 |
| "Phase 6" (handoff: adaptive research; demo-lab: HTTP mesh) | **name collision; demo-lab version never done** | R5 and P3 |
| Phases 7–9 (chaos, SDKs, WAN) | proposals | §9 |

## 9. Post-paper roadmap [P]
Coordinated reads / sharding across gateways; transparent reverse-proxy mode and RFC 9111 conformance; hot-reload membership; client SDKs; multi-region replication; richer admission/eviction after correct reference implementations; production-grade chaos tooling.

## 10. Skills (see `skill_strategy.md`)
Create in order: S1 `CLAUDE.md` (done), benchmark-integrity protocol, novelty-review checklist, UI-telemetry contract (with S6), distributed-failure review (with P3). Validate each against seeded-flaw tests.

## 11. Decisions needed from the owners
1. **Distributed architecture:** (a) local + working invalidation, (b) coordinated reads, or (c) single-node-first with the cluster layer gated.
2. Approval to install Rust and tooling in WSL2 (and later a Linux host).
3. Approval to add dev-dependencies when S1 starts (list in `baseline_and_profiling_plan.md` §6).
4. Whether to re-state the handoff documents' unsupported claims in a correction note (documentation only).
5. Whether to finish the pending core literature slice before locking R5's thesis (recommended).

## 12. Biggest uncertainties
(i) Whether tuned static baselines already capture most of the benefit (the falsifiers in `hypothesis_and_experiments.md`). (ii) Search incompleteness. (iii) The effort to make the distributed layer real — it may dominate the schedule.
