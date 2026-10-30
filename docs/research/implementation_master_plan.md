# PhantomCache — Implementation Master Plan (entry point for AG hand-off)

**Date:** 2026-10-28. **Role split:** research lead/architect (this document) → AG implements bounded packages → owner reviews and gates. **Status: PROPOSED — nothing here is authorized for implementation until §9 says so.**
Supporting documents (all under `docs/research/`): `adr/ADR-001-…` (present architecture, 8 questions, options), `work_packages.md` (specs), `ag_task_WP-00.md` (first task), `benchmark_suite_spec.md`, `ui_metric_provenance.md`, `baseline_and_profiling_plan.md`, `claims_ledger.md`, `hypothesis_and_experiments.md`, `literature_review.md`, `candidate_comparison.md`, `code_findings.md`, `architecture_investigation.md`, `skill_strategy.md`, `plan_of_action.md` (v2 strategy), `initial_assessment.md` (historical, qualified).

## 1. Verified starting state
- **HEAD** `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 0 stashes [V].
- **Modified tracked (16):** `Cargo.lock`, `Cargo.toml`, `README.md`, `docs/PHASE_2_WORK_REPORT.md`, `src/api/server.rs`, `src/control/pie.rs`, `src/core/cache.rs`, `src/core/coalesce.rs`, `src/dist/node.rs`, `src/dist/router.rs`, `src/intel/telemetry.rs`, `src/lib.rs`, `src/main.rs`, `src/sim/traffic.rs`, `tests/unit/wp1_tests.rs`, `tests/unit/wp4_tests.rs`. Their last modification times (≤ 22:48) precede the first research file (23:14): none were touched by this research pass [V].
- **Untracked (16):** `CLAUDE.md` (created this session, approved), `MILESTONE_1_DESIGN_PLAN.md`, `docs/demo-lab/`, `docs/handoff/`, `docs/milestone_1/`, `docs/research/` (this work), `docs/security/`, `scripts/`, `src/dist/{anti_entropy,frame,pool,session}.rs`, `src/intel/diagnostics.rs`, `src/security/`, `tests/integration/`, `transcript_copy.jsonl`. **All are preserved.** The large amount of uncommitted work is itself a risk (no commit has captured Phase 2–4 work); recommend the owner decide when to commit — not done here.
- **Tests:** 242 passed / 0 failed / 19 targets (debug) [REPRO]. Classification: ADR-001 §3 (file-level). **Not established by them:** end-to-end distributed correctness, `main()` wiring, cross-gateway invalidation.
- **What I inspected personally:** `cache.rs` (all), `pie.rs`, `telemetry.rs`, `sketch.rs`, `traffic.rs`, `workload.rs`, `coalesce.rs` (1–150), `anti_entropy.rs` (all), `router.rs` (1–330, 505–897), `node.rs` (24–64, 380–510), `server.rs` (96–300, 740–780, 812–915, 925–955, 1566–1716), `main.rs` (176–310 plus greps), `App.jsx` (1–260, 403–455), tests (4b DELETE gateway tests, wiring greps), docs listed in earlier reports.
- **Agent-reported only [A]:** transitions of `NodeState`; broadcaster counters unread elsewhere; unbounded spawned tasks; absence of per-session timeouts; `frame.rs`/`pool.rs`/`session.rs`/`security/*` skims; handoff 03/06/08 contradictions; most vendor/literature opens.
- **Incomplete investigations:** GET handler body (`server.rs:357-770`) re-read line-by-line; `frame.rs`, `pool.rs`, `security/*` audits; `App.jsx` 260–402/455–524; transcripts' rationale for leaving GET unrouted (not found); literature unresolved leads (Labrinidis, Qu, Azar, Brownout; Rajomon/TopFull/Protego full text; 12+ vendors).

## 2. Architecture decision (recommended; see ADR-001 §7)
**Immediate: C — a trustworthy single-node serving path first; keep the cluster layer compiled, tested and documented as experimental; no distributed claims.** **Long-term: A (local caches + reliable invalidation fan-out), decided at gate G-ARCH with multi-gateway evidence. B deferred.** No distributed code is deleted. Challenge to the stated preference: agreed, with the amendment that WP-00/WP-01 turn the cluster gaps into executable evidence and a correction record now, so the experimental status is explicit and enforced, not implicit.

## 2b. Roadmap: seven milestones and the MVP (canonical: `mvp_and_milestones.md`; Rev 2 inserted M5 Phantom Intelligence & PIE)
**Target: the MVP** — a real, demonstrable caching product + a defensible research paper + a reproducible experimental artifact. A successful demo is not an MVP. Full definition (components M-1…M-10, evidence for each, and the two valid outcomes MVP-A "contribution validated" / MVP-B "contribution not validated: best static configuration + lab + measurement study") is in `mvp_and_milestones.md` §1. Paper acceptance is not an engineering criterion; paper claims ⊆ SUPPORTED entries of `claims_ledger.md`.
| Milestone | Objective | WPs / RTs (ids unchanged) | Gate to next |
|---|---|---|---|
| **M1 Truth & Baseline** (= Phase 1 package) | trustworthy single-node foundation, truthful telemetry/UI, reference algorithms, pre-optimization baselines | WP-00, 01, 02/02b, 03, 04, 05, 06; RT-0, RT-1 | G-M1 |
| **M2 Core Correctness, Performance & Freshness** | evidence-driven optimization, HTTP freshness semantics, verified single-node behavior | WP-07/08/09 (rule-gated), 14, 22a, 23; RT-2, RT-3 | G-M2 |
| **M3 Distributed Correctness & Architecture Gate** | multi-gateway fault-injection evidence; A/B/C decision; implement/verify the chosen contract | WP-10, 11/12/13 (if A); RT-5 optional | G-M3 = G-ARCH + G-DIST |
| **M4 Phantom Lab & Static Baselines** | deterministic lab, controllable origin, scenarios, strong static baselines | WP-15, 16, 17, 20a; RT-4 | G-M4 |
| **M5 Phantom Intelligence & PIE** *(new, Rev 2)* | auditable signals→candidates→evaluation→verdict→executed/advisory decision path; dispositions for each intelligent capability (`phantom_intelligence_phase.md`, `pie_audit.md`) | WP-30…35, 37 (M5a); WP-36a/b/c, **WP-18**, **WP-19a** (M5b, after G-RESEARCH); WP-22b superseded by WP-33 | G-M5 (PI-1…PI-10) |
| **M6 Research Evaluation & MVP Gate** *(formerly M5)* | integrated evaluation, ablations, claims ledger, MVP-A/MVP-B declaration | WP-19b, 20b; RT-2 final | G-MVP |
| **M7 Productization & Research Release** *(formerly M6)* | operability, security, packaging, artifact, manuscript | WP-21, 20c, 25; RT-6 | G-RELEASE |
Tracks visible throughout: **P** product engineering, **R** research/prior art/evaluation, **I** measurement/UI/experimental infrastructure. **Research and novelty validation begin now** (RT-0 runs in parallel with M1), not after the controller. **Phantom Intelligence & PIE are a required MVP component (M-6)**; the legacy threshold `pie.rs` is not the PIE and is never activated against unverified metrics. Only M1 is specified for execution; **M2–M7 are not authorized** and each opens only through its owner-approved gate. Change log: `mvp_and_milestones.md` §9.

## 3. Prioritized dependency graph (work-package view; milestone view in §2b)
```
Phase 0  WP-00 ─► WP-01
Phase 1  WP-02 ─► WP-03          WP-06          WP-04 ─► (pilot) ─► WP-05
Gate G-BASE  (§7)
Phase 2  evidence-gated: WP-07, WP-08, WP-09        WP-14 (TTL)         WP-10 (multi-gateway harness)
Gate G-ARCH (A/B/C decision with evidence)           ─► WP-11 ─► WP-12 ─► WP-13  (only if option A)
Phase 3  WP-15, WP-16 ─► WP-17 (baselines) ─► Gate G-RESEARCH ─► WP-18 (controller prototype) ─► WP-19 (evaluation)
Phase 4  WP-20 (lab UI), WP-21 (deployment), WP-22 (PIE fate)
```
Parallel after WP-00: {WP-01, WP-02, WP-04, WP-05, WP-06}. Shared interface contracts: telemetry schema v2 (WP-02 ↔ WP-03 ↔ WP-20); run manifest format (WP-04 ↔ WP-15 ↔ WP-19); origin-model trait (WP-15 ↔ WP-16 ↔ WP-17).

## 4. Tracks
- **Product:** WP-01, 03, 06, 07, 08, 09, 11–14, 21, 22.
- **Research:** WP-05 (shared), 17, 18, 19; claims C1–C4.
- **Shared foundations:** WP-00, 02, 04, 10, 15, 16, 20.
Rule: research mechanisms live in the lab/harness until a claim reaches SUPPORTED **and** the production cost/benefit is justified; no research idea enters `src/` by default.

## 5. Low-level performance findings (classification)
Per `benchmark_suite_spec.md` §1: **confirmed in code, impact unverified:** global epoch mutex (H1), scan-based eviction (H2), telemetry lock + sort (H3), clone-on-hit (H7), SipHash (H8). **Confirmed (functional):** entry-count capacity (H4), zero `key_hash` (H5), synthetic dashboard metrics (H6). **Unverified:** allocation/scheduler overhead (H9). No target improvement is stated before the baseline.

## 6. Research prerequisites and falsification (controller stays PROVISIONAL)
**Prerequisites before any controller is evaluated (all must be true):** WP-02 live telemetry with origin metrics; WP-04 baselines with a documented variation bound; WP-14 stale-serving semantics defined (SWR/SIE) with a hard staleness cap; WP-15/16 lab and controllable origin; WP-17 baselines implemented and independently reviewed — **Varnish-style health grace, nginx lock+stale, XFetch + single-flight, Memcache lease/stale, DynamoDB-style constant-work refresh, Envoy-style adaptive concurrency and success-rate admission, Breakwater/DAGOR-style queue-delay admission, retry budget, Farahbakhsh remedies, and an oracle-tuned static baseline**; pre-registered thresholds (`hypothesis_and_experiments.md`); the unresolved literature leads opened (Labrinidis & Roussopoulos, Qu et al., Azar et al., Brownout, Rajomon/TopFull/Protego full text, unchecked vendor docs, Akamai patents).
**Falsification (abandon or substantially revise C1 if any):** F-a tuned static baselines (incl. DynamoDB-style constant-work and oracle-tuned static) reach within 10% of the controller on ≥80% of scenarios; F-b gain vanishes under heterogeneous per-key origin latency or gradual degradation; F-c controller worsens p99 by >5% in any scenario vs no-controller; F-d any staleness-cap or origin-cap violation; F-e effect CI includes zero over ≥30 seeds; **F-f** the second literature round finds prior work that already couples measured origin health to stale/refresh/admission under explicit caps (then the contribution reverts to the harness + measurement study, or distributed assurance after G-ARCH).
**Gate G-RESEARCH:** prerequisites satisfied + owner approval to build WP-18.

## 7. Acceptance gates
| Gate | Must be true to pass |
|---|---|
| **G-START** (now) | HEAD/status verified; ADR-001 approved; WP-00/01 spec approved |
| **G-TRUTH** | WP-00 evidence returned; WP-02 merged; live payload has zero unlabeled synthetic fields; DELETE no longer pollutes latency/hit statistics; UI (WP-03) validated against a real gateway (screenshots); 242 baseline preserved |
| **G-BASE** | WP-04 pilot variation bound recorded; B1/B2/B6-current baselines with ≥30 reps and CIs on each approved platform; WP-05 reference policies with golden tests; byte accounting (WP-06) with measured error; optimizations either justified by data or explicitly dropped |
| **G-ARCH** | Multi-gateway fault-injection harness (WP-10) exists; KG tests classified; decision A/B/C recorded with measured staleness numbers under injected loss/partition; docs aligned (WP-01) |
| **G-RESEARCH** | §6 prerequisites |
| **G-EVAL** | WP-19: ≥30 seeds/scenario; CIs; ablations; negative results recorded; Tier 1 vs Tier 2 agreement reported; `claims_ledger.md` updated |
| **G-DEPLOY** | Claims limited to what was validated: multi-host on physical Linux for any network/latency claim; security configuration verified; runbooks |
General: unit tests alone never pass a distributed gate; performance claims require comparative measurements; novelty claims require verified prior-art analysis.

## 8. Required evidence format from AG (every package)
(1) pre/post `git rev-parse HEAD`, `git branch --show-current`, `git status --short`; (2) exact commands run and complete outputs for the package's verification commands; (3) test counts before/after (`passed/failed/ignored`) for the full suite; (4) `git diff --stat` limited to expected files; (5) for benchmarks: manifest + raw data location + summary with CIs; (6) a list of deviations from the spec and why; (7) explicit statement of anything that did not behave as the spec predicted; (8) no commits or pushes.

## 8b. Phase-level workflow (supersedes per-package approval)
Work is now approved and reviewed **per phase**: `phase_1_execution_package.md` is the current entry point for AG. It groups WP-00, 01, 02/02b, 03, 04, 05, 06 into one ordered phase ("Truth & Baseline") with one consolidated report. `work_packages.md` remains the detailed spec reference; where they conflict, the phase document wins. Phases 2–6 are outlined there (§0, §11) and are specified after their entry evidence exists.

## 9. Authorization record
**Update (owner, latest message):** documentation correction (WP-01), isolated WSL2 tooling setup, and development-only benchmark/property-test dependencies (when baseline work begins) are **authorized**. Application-code changes remain subject to explicit phase approval — i.e., the owner's approval of `phase_1_execution_package.md` (v3) including its approval items A-1…A-14 (§3). The text below predates this update; where it lists WP-01 and WP-04 dev-dependencies/WSL as "requires approval", those are now authorized.
**Authorized now (by prior owner approvals):** documentation under `docs/research/`; read-only investigation; `CLAUDE.md` (created).
**Requires explicit owner approval before AG starts:**
- WP-00 (adds a test file; no `src/` change) — **proposed first package**. Instruction: `phase_1_execution_package.md` v3 §5 S2 and §6.2 (`ag_task_WP-00.md` is superseded).
- WP-01 (new `docs/handoff/CORRECTIONS.md` + one linking line in `00_START_HERE.md`).
- WP-02/WP-03/WP-06 (application changes) — spec ready; approve individually.
- WP-04: dev-dependencies (e.g., `criterion`, optionally `dhat`, `proptest`, `loom`), and Rust/tooling install in WSL2.
- Owner decisions: simulation loop default (opt-in vs on) for the live gateway; key-hash hasher; whether to keep the old dashboard as a labeled SIMULATED demo; when/how to commit the large uncommitted tree; policy for `bench-results/`.
**Roadmap Rev 2 (Phantom Intelligence & PIE as Milestone 5, MVP component M-6):** documentation only; **nothing in it is authorized for implementation**, and Phase 1 scope is unchanged. Awaiting owner approval of the seven-milestone sequence (decisions in `phantom_intelligence_phase.md` §10).
**Phase 1 package v3.1 (2026-10-29):** revised after review findings 1–7 (`phase_1_execution_package.md` §11); still **PROPOSED**, not authorized; corrections await the owner's verification.
**Not authorized (no one should start):** controller implementation; Phase-5 soak testing; removing/rewriting the cluster layer; any commit or push.

## 10. Risks, unresolved decisions, stop conditions (global)
- The uncommitted Phase 2–4 work is the project's only copy of that code: a single destructive command would be catastrophic — keep `CLAUDE.md` rules; consider a commit/branch snapshot (owner decision).
- Platform noise (WSL2/Windows) may make some measurements unusable; plan a Linux host.
- Distributed repair may dominate the schedule; keep it gated.
- Literature search is incomplete; do not publish novelty claims.
- **Global stop conditions for AG:** unexpected working-tree changes; baseline test count ≠ 242 at start; any need to modify files outside a package's scope; any ADR claim that fails to reproduce; any dependency not pre-approved; ambiguity about semantics of fencing, epochs, invalidation (ask, do not guess).

## 11. UI / product / research dependency summary
UI (WP-03) waits on telemetry schema (WP-02); lab UI (WP-20) waits on WP-03 and the lab (WP-15); controller UI only if the controller is integrated. Product work (WP-06, 07–09, 14) feeds the baselines and does not depend on research; research (WP-17–19) depends on WP-02/04/14/15/16 and not on the cluster repair.

## 12. Concise review summary (for the owner)
1. **State:** HEAD/branch verified; tree preserved; 242 tests reproduced but they don't prove distributed correctness or `main()` wiring.
2. **Key verified facts:** serving cache and node caches are separate objects (`main.rs:65`, `node.rs:42`); peer invalidations never purge the serving cache; no production reconciliation; sequence tracking discards gaps (`anti_entropy.rs:215,226-231`); DELETE purges local after the cluster round trip; the dashboard's PIE inspector, hit/miss split, cost panel and "Phantom OFF" numbers are hard-coded; the live `throughput` is a cumulative count; `p95` carries p90.
3. **Decision to approve:** immediate C, conditional A, defer B (ADR-001).
4. **First package for AG:** WP-00 (tests only, +WP-01 docs); then WP-02 → WP-03, WP-06, and WP-04/05 in parallel.
5. **Research:** origin-aware controller stays provisional until baselines + the unresolved literature are done; falsifiers are pre-registered.
6. **Approvals needed:** §9 list.
