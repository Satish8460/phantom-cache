# PhantomCache — The MVP and the Seven Milestones

**Status:** PROPOSED roadmap. **Revision 2 (2026-10-29):** Phantom Intelligence & PIE inserted as a dedicated **Milestone 5** and made a required MVP component. **Revision 1 (2026-10-28)** had six milestones; the change log is in §9. Supersedes the "Phase 0–9" numbering in `docs/handoff/05` and the phase map in `plan_of_action.md` v2; WP identifiers are preserved. **Nothing in Milestones 2–7 is authorized by this document.** Milestone 1 = the already-specified `phase_1_execution_package.md` (scope unchanged).
Terminology: **Milestone** = major objective with an exit gate; "Phase N" in older documents ≡ Milestone N **of Revision 1 for N ≤ 4** (unchanged in Revision 2). **WP-xx** = implementation work package; **RT-xx** = research task (documentation/evaluation design/prior art; no application code). Established decisions are stated plainly; **[PROPOSAL]** marks items awaiting owner approval.

## 1. The MVP (explicit definition)
**Value proposition.** A single-binary HTTP caching gateway that *protects an origin under load and degradation* — single-flight coalescing, bounded and explainable staleness, byte-bounded memory, trustworthy observability, and an **auditable intelligence layer (Phantom Intelligence / PIE) that evaluates and, where safe, adapts policies** — together with a reproducible laboratory. The paper reports what the system demonstrably contributes; the product's value does not depend on the paper's claims; paper acceptance is **not** an engineering acceptance criterion.

**The MVP is achieved only when ALL of the following hold, each with the stated evidence:**
| # | Component | Must be true | Evidence |
|---|---|---|---|
| M-1 | **Cache engine** | Single-node engine with generation-fenced fills, coalescing, byte-bounded memory, correct HTTP freshness semantics (`Cache-Control`, SWR/SIE subset with a hard staleness cap), verified under concurrency and a bounded-memory soak | M2 exit report |
| M-2 | **Truthful observability** | Every operator-visible number is measured or labeled derived/simulated/inferred/unavailable; origin metrics exist; no synthetic value in live mode | M1 + contract tests; UI screenshots against a live gateway |
| M-3 | **Correctness & failure behavior** | A *stated* distributed contract (or an explicit enforced "single-node supported; cluster experimental") with fault-injection evidence for exactly the claims made | M3 gate record |
| M-4 | **Controlled experiments** | Deterministic lab, controllable real-process origin, scenario library, run manifests, ≥30 seeds per scenario | M4 exit report |
| M-5 | **Credible baselines** | Independently reviewed strong static mechanisms, incl. oracle-tuned static | fidelity review record |
| **M-6** | **Phantom Intelligence & PIE (REQUIRED)** | A real, integrated, auditable path **signals → candidate policies → evaluation (shadow and/or canary) → verdict → executed or advisory decision**, constructed in the production binary (not only in tests), with provenance-gated inputs, hard caps enforced by the execution layer, safe failure, bounded overhead, and user-visible explanations; each intelligent capability carries a recorded **disposition** (retained / narrowed / rejected) | M5 exit report; criteria PI-1…PI-10 in `phantom_intelligence_phase.md` §7 |
| M-7 | **Validated central research contribution — *if the evidence supports one*** | Prior-art closure; hypothesis pre-registered; controller implemented only after G-RESEARCH; measured effect with CIs and ablations | `claims_ledger.md` at SUPPORTED, or an honest NARROWED/REFUTED record |
| M-8 | **Usable UI** | Live operations view (truthful), Intelligence view (real decision records), lab view (configure/replay/compare/export); panels only where backend capability exists | M7 usability checks |
| M-9 | **Reproducible research artifact** | Pinned toolchain, manifests, raw data, scripts, one-command regeneration on a clean machine | clean-machine reproduction log |
| M-10 | **Operability** | Install/run docs, configuration reference, byte-based sizing, security review of shipped defaults, runbooks | M7 gate record |
| M-11 | **Paper (deliverable, not a gate)** | Manuscript reporting measured results, ablations, limitations, reproducible methods — limited to SUPPORTED claims | claims↔paper trace |

**Intelligence outcomes (each a valid result; the *machinery* must always be real):**
- **Validated intelligence:** ≥1 capability shows benefit over appropriate baselines (95% CI, pre-registered effect) with safety and overhead constraints satisfied ⇒ PIE ships in **active** mode for that capability.
- **Narrowed intelligence:** only the subset that survives evaluation ships active; unsupported predictive/adaptive capabilities stay disabled (advisory or off).
- **Negative research outcome:** a hypothesis is rejected with evidence and limitations documented (valid science). The product still ships a *functional* PIE in **advisory/shadow** mode (real signals, real evaluation, real decision records, no automatic action), plus the best static configuration. **A nonfunctional, hard-coded or entirely simulated PIE is not product-ready and cannot satisfy M-6.**
- The MVP does **not** depend on a complex ML model beating every baseline; it depends on M-6's machinery criteria being met and every capability's disposition being evidence-based.

**Two valid MVP outcomes (neither is a "failed" one):**
- **MVP-A (contribution validated):** M-1…M-11 including ≥1 validated intelligent capability active in PIE; the paper claims what the ledger supports.
- **MVP-B (contribution not validated):** M-6 machinery met with dispositions narrowing or rejecting capabilities; PIE active only for what survived (possibly nothing — then advisory mode); product ships the best static origin-protection configuration; the paper reports the lab, baselines, measurement study and the negative/narrowed result.
**Not an MVP:** a demo that looks right; passing unit tests; a hard-coded or simulated PIE; a controller without baselines; simulation presented as system measurement; a paper whose claims outrun `claims_ledger.md`; a cluster described as distributed-correct without fault-injection evidence.

## 2. Three parallel tracks (visible in every milestone)
- **P — Product engineering:** engine, correctness, performance, distributed contract, intelligence integration, deployment.
- **R — Research / prior art / evaluation:** literature closure, hypotheses, baselines' fidelity, candidate evaluation, claims, manuscript.
- **I — Measurement / UI / experimental infrastructure:** telemetry, benchmarks, lab, origin model, manifests, UI, artifact packaging.
**Research starts now.** RT-0 (prior-art closure) is under way and continues until G-RESEARCH; novelty is re-validated at every gate (RT-2). Novelty is never deferred until after a controller exists.

## 3. Milestone map
| | M1 Truth & Baseline | M2 Core Correctness, Performance & Freshness | M3 Distributed Correctness & Architecture Gate | M4 Phantom Lab & Static Baselines | **M5 Phantom Intelligence & PIE** | M6 Research Evaluation & MVP Gate | M7 Productization & Research Release |
|---|---|---|---|---|---|---|---|
| **P** | WP-00, 01, 06, S7d (banner) | WP-07/08/09 (rule-gated), **WP-14**, **WP-22a** (quarantine legacy PIE stub), **WP-23** | WP-10, **G-ARCH**, WP-11/12/13 (if A) | — | **WP-30…35, 37** (M5a); integrate validated candidates, **WP-36a/b/c, WP-18** (M5b) | integrated build; fixes | **WP-21**, security review, packaging, runbooks, multi-host validation |
| **R** | **RT-0**, RT-1 | RT-2, RT-3 | RT-2; optional RT-5 | RT-2; RT-4 | **G-RESEARCH**, **WP-19a** candidate evaluation, ML gate | **WP-19b** integrated evaluation, RT-2 final, claims | **RT-6** manuscript |
| **I** | WP-02/02b, 03, 04, 05 | benchmark re-runs | multi-gateway harness | **WP-15, 16, 17**, B5/B8/B9, **WP-20a** | PIE-in-lab replay, **WP-35** Intelligence UI panel, overhead benchmarks | **WP-20b** compare/export UI | **WP-20c**, **WP-25** |
| **Entry** | none (authorized: docs, WSL tooling, dev-deps; app changes need phase approval) | G-M1 | G-M1 (may overlap M2) | G-M1 origin metrics + M2 WP-14 | **M5a:** G-M1 + G-M2 + WP-15 (overlaps M4). **M5b:** G-M4 + G-RESEARCH | G-M5 | G-MVP |
| **Parallel** | RT-0 ∥ all | M2 ∥ M3 | ∥ M2, ∥ M4 lab core | WP-15/16 ∥ M3 | M5a ∥ M4 | RT-0/2 throughout | WP-20c ∥ RT-6 ∥ WP-21 |
| **Exit gate** | **G-M1** | **G-M2** | **G-M3** = G-ARCH + G-DIST | **G-M4** | **G-M5** (PI-1…PI-10) | **G-MVP** | **G-RELEASE** |

## 4. Milestone specifications

### Milestone 1 — Truth & Baseline
Objective, deliverables, WPs and acceptance exactly as `phase_1_execution_package.md` (AC-1…AC-12): WP-00, 01, 02/02b, 03, 04, 05, 06, S7d (banner); research RT-0, RT-1 in parallel. **Non-goals:** optimizing, touching distributed behavior, any controller, activating PIE. **Gate G-M1:** owner acceptance after my consolidated review; authorizes M2 and M3. **Relevance to M5:** the schema v2 provenance, real `key_hash`, origin metrics and byte accounting delivered here are the inputs M5 consumes; no scope was added to M1 for this.

### Milestone 2 — Core Correctness, Performance & Freshness
Objective: verified single-node engine with proper HTTP freshness semantics and only data-justified optimizations. Deliverables: WP-07/08/09 only if their pre-registered rules fire; **WP-14** `Cache-Control`/validators/conditional revalidation, `stale-while-revalidate`/`stale-if-error` with a configurable **hard staleness cap**; **WP-22a** quarantine the legacy threshold PIE (`pie.rs`): it stays compiled and labeled *legacy baseline*, is never active on live traffic, and its tests are re-labeled as state-machine tests (replacement is WP-33 in M5); **WP-23** single-node verification (loom, property tests, parser fuzz, bounded-memory soak); schema v1 removal. Exit (G-M2): freshness semantics tests, concurrency checks, soak within tolerance, every optimization CI-separated or dropped, RFC 9111 supported-subset statement. **Non-goals:** performance work without baselines; adaptive logic.

### Milestone 3 — Distributed Correctness & Architecture Gate
Objective: replace unproven distributed claims with an explicit contract backed by evidence. Deliverables: **WP-10** multi-gateway fault-injection harness; **G-ARCH** decision (A/B/C) with measurements; WP-11/12/13 if A; enforced gating if C; optional RT-5 formal model. Exit **G-M3 = G-ARCH + G-DIST**: every distributed claim maps to a failure model, a bound and a falsifying test. **Non-goals:** "all tests pass" as proof; linearizability claims; building B without measured need.

### Milestone 4 — Phantom Lab & Static Baselines
Objective: reproducible experimental platform and strong static baselines before any adaptive logic. **WP-15** deterministic lab (virtual clock, seeded RNG, replay, scenario library); **WP-16** controllable real-process origin and open-loop load generator; **WP-17** reviewed baselines (Varnish-style health grace, nginx-style lock+stale, XFetch+single-flight, Memcache lease/stale, DynamoDB constant-work refresh, Envoy-style adaptive concurrency/admission, Breakwater/DAGOR-style admission, retry budget, Farahbakhsh remedies, oracle-tuned static); B5/B8/B9; **WP-20a** lab UI; RT-4. Exit **G-M4:** one-command scenario × baseline matrix, ≥30 seeds, fidelity review, stated simulation-vs-real discrepancy. **Non-goals:** controller; baselines tuned only in the system's favor.

### Milestone 5 — Phantom Intelligence & Policy Intelligence Engine (new)
Full specification: **`phantom_intelligence_phase.md`**; audit of what exists: **`pie_audit.md`**.
- **Objective:** a real, auditable intelligence path from trustworthy signals to policy evaluation to executed/advisory decisions, and evidence-based dispositions for each intelligent capability.
- **Staging:** **M5a machinery** (WP-30 interfaces, WP-31 signal/feature pipeline incl. hot-key and shift detection with real key hashes, WP-32 shadow evaluator, WP-33 PIE decision engine v2 with constraint-first utility, canary/rollback/cooldown, statistics, kill switch, WP-34 policy execution layer and safe fallback, WP-35 audit/explainability + Intelligence UI, WP-37 safety/overhead/replay verification) — may overlap M4; **M5b candidate intelligence** (WP-36a statistical candidates, **WP-18** origin-aware control, WP-36b predictive candidates, WP-36c ML selector under a strict gate, **WP-19a** candidate evaluation) — requires G-M4 and G-RESEARCH.
- **Exit criteria (G-M5):** PI-1…PI-10 evidenced; every candidate has a disposition; legacy stub out of all product paths; PIE constructed in `main()`; docs updated.
- **Non-goals / not completion:** ML for its own sake; a dashboard of decisions that never happened; PIE acting on simulated or unverified metrics; benefit claims without baselines; counting the legacy stub as the engine.
- **Gate G-M5:** my review + owner acceptance; authorizes M6.

### Milestone 6 — Research Evaluation & MVP Gate (formerly M5)
Objective: integrated, system-level evaluation and the MVP declaration. Sequence: (1) RT-0 closed and RT-2 final novelty review (a G-RESEARCH prerequisite already satisfied for M5b is re-confirmed here); (2) **WP-19b** integrated Tier 1 + Tier 2 evaluation of the product build (PIE active/advisory/off, vs baselines), ablations, sensitivity, falsifier tests F-a…F-f, negative results; (3) `claims_ledger.md` final; (4) **WP-20b** comparison/export UI; (5) MVP-A or MVP-B declared. Exit **G-MVP:** every ledger claim SUPPORTED/NARROWED/REFUTED with evidence; integrated build passes the full suite and the acceptance scenario script end to end. **Non-goals:** thresholds chosen after seeing results; novelty claims without the closed prior-art record.

### Milestone 7 — Productization & Research Release (formerly M6)
**WP-21** deployment readiness, security review, packaging, physical multi-host validation for any network claim; **WP-20c** UI polish and operator task-script usability checks (incl. "why did the policy change?"); **WP-25** artifact packaging; **RT-6** manuscript. Exit **G-RELEASE:** clean-machine reproduction, operator task script without internal logs, security items closed or documented, manuscript claims traceable to SUPPORTED entries. **Non-goals:** venue acceptance; new features.

## 5. Dependency and gate summary
```
RT-0 (prior-art closure) ═════════════════════════════════════════════════════►(closed before G-RESEARCH)
M1 ─G-M1─► M2 ─G-M2─┐
   └────► M3 ─G-M3──┼─► M4 ─G-M4─┐
                    │            ├─► M5b ─┐
          M5a (needs G-M1,G-M2,WP-15; overlaps M4) ──────┘        │
                                         G-RESEARCH ──► (M5b)     ▼
                                                      M5 ─G-M5─► M6 ─G-MVP─► M7 ─G-RELEASE
```
| Gate | Authorizes | Needs |
|---|---|---|
| G-M1 | M2, M3 | Phase 1 AC-1…AC-12 + owner acceptance |
| G-M2 | M5a (with WP-15), M4 items needing WP-14 | semantics, concurrency, soak, optimization evidence |
| G-M3 (G-ARCH + G-DIST) | cluster-dependent scenarios; distributed claims | contract + fault-injection evidence |
| G-M4 | M5b | lab + reviewed baselines, ≥30 seeds |
| G-RESEARCH | WP-18 and M5b candidates | RT-0 closed (incl. remaining sweep gaps), pre-registered thresholds, owner approval |
| G-M5 | M6 | PI-1…PI-10, dispositions |
| G-MVP | M7 | ledger resolved; MVP-A or MVP-B declared |
| G-RELEASE | release | clean-machine reproduction etc. |
Each gate is an explicit owner decision after my consolidated review; nothing starts automatically.

## 6. WP/RT traceability (identifiers unchanged unless noted)
| Milestone | Existing WPs | New / re-mapped |
|---|---|---|
| M1 | 00, 01, 02(+02b), 03, 04, 05, 06 | RT-0, RT-1 |
| M2 | 07, 08, 09, 14, 22 (→ **22a** quarantine) | 23, RT-2, RT-3 |
| M3 | 10, 11, 12, 13 | RT-5 (optional) |
| M4 | 15, 16, 17, 20 (→ 20a) | RT-4 |
| **M5** | **18** (moved from old M5), **19 (→ 19a)**, 22b is **superseded by WP-33** | **30, 31, 32, 33, 34, 35, 36a/b/c, 37**, G-RESEARCH |
| M6 (old M5) | 19 (→ **19b**), 20 (→ 20b) | RT-2 final |
| M7 (old M6) | 21, 20 (→ 20c) | 25, RT-6 |

## 7. Research tasks (RT)
RT-0 prior-art closure (status: see `literature_review.md` §4d–4e; **not closed** — proceedings sweep incomplete: SOSP, SIGCOMM, SIGMETRICS, VLDB, SIGMOD, SoCC, IMC, 2023 venue-years, arXiv listings not scanned; Labrinidis 2003/2004 and UNIT full texts not obtained); RT-1 freeze pre-registration before WP-18/WP-36; RT-2 novelty snapshot at each gate; RT-3 related-work draft; RT-4 methodology; RT-5 formal model (optional); RT-6 manuscript.

## 8. Principles
Product vision exceeds the paper. Paper claims ⊆ SUPPORTED ledger entries. Simulation ≠ measurement. Intelligence must be real before it is branded; ML is gated. The legacy threshold PIE is not the PIE.

## 9. Change log
- **Rev 2 (2026-10-29):** inserted M5 "Phantom Intelligence & PIE"; former M5 → M6, former M6 → M7; WP-18 and WP-19a moved into M5; WP-19 split (19a candidates in M5, 19b integrated in M6); WP-22b superseded by WP-33; WP-20 split retained (20a/20b/20c: now M4/M6/M7); MVP gained required component M-6 and defined intelligence outcomes; added WP-30…37. M1–M4 and all pre-existing WP ids unchanged. Documents updated: `implementation_master_plan.md`, `work_packages.md`, `phase_1_execution_package.md` (map/title only), `plan_of_action.md` (note), `claims_ledger.md`, `CLAUDE.md`, `docs/handoff/05` (pointer).
- **Rev 1 (2026-10-28):** six milestones; MVP-A/MVP-B defined.
