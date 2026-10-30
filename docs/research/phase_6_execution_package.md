> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. The M6 evaluation was executed once under the preregistration chain with result H1 NOT_SUPPORTED ([m6_sealed_results.md](m6_sealed_results.md)); the M6 completion report carries a retraction banner. No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 6 Execution Package — M6: Research Evaluation & MVP Gate

**Status: PROPOSED — not authorized.** Requires G-M5 (Phantom Intelligence machinery and candidate dispositions) — and G-M3 for any distributed claim. Common rules: `phase_execution_packages_index.md` §4. **Research track separate from, but connected to, product engineering:** the research lead owns evaluation design, analysis and claims; AG executes runs and builds artifacts; **no analysis choice may be made after outcomes are seen** (§5). **Both outcomes are legitimate: MVP-A (validated) and MVP-B (refuted or narrowed).**

## 1. Purpose, scope, non-goals
**Purpose.** (1) Close the prior-art investigation honestly; (2) freeze falsifiable hypotheses and the analysis plan; (3) evaluate the integrated system and its adaptive capabilities against strong, fairly tuned alternatives with proper statistics, ablations, sensitivity and failure-mode analysis; (4) validate reproducibility; (5) resolve every claim in `claims_ledger.md`; (6) decide, on evidence, MVP-A or MVP-B and verify the engineering MVP gates. **Scope:** evaluation runs on the frozen integrated build, analysis scripts, artifact validation, prior-art closure, ledger. **Non-goals:** new features, new candidates (new ideas ⇒ a new pre-registered cycle), manuscript writing (M7), release engineering (M7), UI polish (M8). A successful demo, a passing test suite, or a controller that executes is **not** evidence of a validated contribution.

## 2. Starting state (PROVISIONAL; verified at M6-00)
Inputs expected from M1–M5: truthful telemetry and VALID baselines; hardened single-node engine; a distributed contract (or gating); the lab (T1 deterministic, T2 loopback), reviewed baselines with frozen tuned configurations, family split and sealed seed lists; Phantom Intelligence machinery with candidate dispositions on held-out families (M5-13); retained candidates behind flags (M5-14, owner-approved) or lab-only; literature record in `literature_review.md` (RT-0 **not closed** as of 2026-10-29: proceedings sweep covered ≈ a dozen USENIX/EuroSys venue-years; SOSP, SIGCOMM, SIGMETRICS, VLDB, SIGMOD, SoCC, IMC and 2023 venues not swept; full texts of Labrinidis 2003/2004, UNIT, Kolobov line, Defcon, MIDAS not read; vendor documentation partially covered; patents partly read). Nothing in this list is assumed satisfied.

## 3. Work packages
| ID | Title | Depends on |
|---|---|---|
| M6-00 | Entry verification; inventory of evidence inputs; confirm sealed seeds unused | G-M5 |
| M6-01 | Prior-art closure (RT-0) and novelty statement | runs from now until G-RESEARCH; final here |
| M6-02 | Hypotheses, endpoints, analysis plan, power analysis → **pre-registration freeze** (RT-1) | M6-00 |
| M6-03 | Evaluation design freeze: scenario×arm matrix, seeds, tooling, analysis code validated on synthetic data | M6-02 |
| M6-04 | Integrated build freeze (commit/tree hash, binaries, configs) | M6-03 |
| M6-05 | Primary evaluation — T1 full matrix | M6-04 |
| M6-06 | Primary evaluation — T2 (and T3 if approved) | M6-05 |
| M6-07 | Ablations and sensitivity analysis | M6-05 |
| M6-08 | Overhead, resource-cost, failure-mode and safety evaluation (PIE modes off/observe/advisory/active) | M6-04 |
| M6-09 | Distributed evaluation (conditional on G-M3) | M6-04 |
| M6-10 | Statistical analysis, tables, figures (deterministic scripts) | M6-05…09 |
| M6-11 | Reproducibility and artifact validation (clean machine) | M6-10 |
| M6-12 | Claims-ledger resolution; MVP-A/B recommendation | M6-10, M6-11, M6-01 |
| M6-13 | Engineering MVP gate verification | M6-12 |
| M6-14 | Completion report | all |

### M6-01 Prior-art closure (RT-0) and novelty
**Protocol (coverage log mandatory; every "none found" statement cites the log).**
1. *Proceedings sweep with a recorded protocol:* venue × year table for 2023–2026 (and 2022 for anchors) covering OSDI, SOSP, NSDI, EuroSys, ATC, FAST, SoCC, SIGCOMM, SIGMETRICS, VLDB, SIGMOD, CIDR, HotOS, HotNets, HotCloud, HotStorage, Middleware, ICDCS, INFOCOM, IMC, TOCS, TOS, ToN, plus arXiv listings (cs.DC, cs.OS, cs.NI, cs.PF, cs.DB); per venue-year: page reached?, titles scanned, candidates flagged, candidates opened (depth: abstract/full text), inaccessible items with the substitute used. DBLP/ACM blocks are handled with alternative indexes (publisher TOCs, arXiv, Semantic Scholar/OpenAlex) and recorded.
2. *Citation chasing* from the closest works (MIDAS, Mao et al., RobinHood, Farahbakhsh et al., Labrinidis & Roussopoulos, UNIT, Kolobov line, delayed-hits papers, Skybridge, SCION, LAH/S4-FIFO, Darwin, overload-control papers) — forward and backward.
3. *Adjacent literatures* explicitly searched: overload control and load shedding, adaptive concurrency limits, retry control, metastable failures, refresh scheduling/age-of-information/crawling, stale serving (RFC 5861, RFC 8767), TTL control, request coalescing/delayed hits, safe online learning/bandits under constraints, cache admission/eviction learning.
4. *Vendor and open-source documentation* for the products listed in `literature_review.md` §4b–4d **and those not yet checked** (HAProxy configuration manual sections, Pingora internals, Varnish MSE/saturation features, Cloudflare Cache Reserve/Tiered Cache/Load Balancing, CloudFront origin controls, Nginx Plus, Azure Front Door, Spring/Resilience4j/Hystrix collapser, Traefik/Caddy cache modules); official docs only, with version/date.
5. *Patents:* Google Patents/other searches (CPC classes for caching/CDN control) with claim-level reading of any hit that mentions origin load, health, staleness or refresh; the already-read Akamai US 10,154,068 and US 10,440,142 re-checked; legal scope is not assessed (flag for counsel if any claim seems relevant).
6. *Independent verification:* a second agent re-runs a sample of the searches with different query formulations and reports discrepancies; the research lead resolves them.
**Closure criteria:** every protocol item COMPLETE or NOT RUN with an owner-accepted reason; a dated **novelty snapshot** (RT-2 final) lists the closest works in a table with explicit differences along the dimensions {measured health signal, adaptive stale window, refresh scheduling, miss admission, coalescing, staleness cap, origin-load cap, metastable evaluation, safety/rollback, evaluation against tuned static baselines}.
**Novelty statement rule:** claims are limited to demonstrated contributions in this system ("integration + dual caps + evaluated against tuned static baselines on specified scenarios"), never "first". **Absence of search results is never evidence of absence.** If substantial overlap is found, STOP and redirect (narrow the claim or switch the contribution to the lab/measurement study) — F-f in `hypothesis_and_experiments.md`.

### M6-02 Hypotheses and pre-registration
Hypotheses (from `hypothesis_and_experiments.md`, restated testably; thresholds are placeholders until frozen): **H1** under partial origin degradation, the validated adaptive capability reduces peak origin concurrency and client p99 relative to BL-12b (family-tuned static, the deployable baseline) by at least δ_min with hit-ratio loss ≤ x pp and zero cap violations; **H2** most of the gain comes from identified mechanisms (shedding/stale extension), tested by ablations; **H3** overhead within the approved budgets; **H4** no oscillation beyond the preregistered bound; **H5** the shadow evaluator's estimates meet their preregistered error bound; **H6** the decision protocol's false-commit/false-rollback rates meet their preregistered bounds. **Null and falsifiers:** H1 is rejected or narrowed under F-a (tuned static within 10% of the controller on ≥ 80% of scenarios), F-b (gain vanishes under heterogeneous per-key origin latency or gradual degradation), F-c (p99 worse by > 5% in any scenario vs no-controller), F-d (any cap violation), F-e (CI includes zero over ≥ 30 seeds), F-f (prior art).
**Pre-registration document** `docs/research/preregistration_m6.md`, hashed and dated *before any evaluation run*: hypotheses; primary endpoints (one per hypothesis); secondary endpoints; scenario × workload × arm matrix; sealed evaluation seeds **S** (generated at M4-01, hash-only, never used in design, tuning or M5 dispositions; leak check at M6-00) and, if the owner approves (O6-2), at least one **sealed family/parameterization** introduced at this step that no designer has seen; held-out families; exclusion rules (INVALID criteria from M4 EV list); analysis plan (below); decision rules; stopping rules (**no data-dependent stopping**); planned deviations protocol. **Power analysis:** from M4/M5 pilot variances, compute the seeds needed for the target minimum detectable effect at power 0.8–0.9 (PROVISIONAL, owner O6-3); minimum 30 seeds (T1) / 10 repetitions (T2, class VALID) regardless. **Amendments:** any change after freeze is a dated amendment with reason; changes motivated by seen outcomes are labeled *post hoc* and results from them are exploratory only.

### M6-03 Evaluation design freeze and analysis validation
Matrix = scenarios (M4 `O-*`) × workload families (tuning families for sanity, **held-out for primary**) × arms {BL-0…BL-12b, retained/narrowed candidates, PIE modes} × seeds. Analysis scripts (deterministic) are written and tested on **synthetic datasets with known effects and on pilot data** — including a no-effect dataset (verifies false-positive rate) — and code-reviewed *before* evaluation data exist. Figures/tables are specified (templates) in advance. **Acceptance:** analysis dry-run reproduces known planted effects within tolerance; reviewed; hashed.

### M6-04 Integrated build freeze
Release-profile build from a recorded commit/tree hash (the working tree may be uncommitted: record the tree hash and a snapshot); binary hashes; configuration files; platform; no source change during M6-05…09 except through the **bug protocol**: a defect found during evaluation ⇒ stop affected cells, document, fix under a new build hash, preregistration amendment, **rerun affected cells with fresh seeds**, keep the old results in the record.

### M6-05/06 Primary evaluation
T1 (≥ 30 seeds/cell, sealed seeds) and T2 (≥ 10 repetitions/cell, class VALID; reduced item sets in the documented order, never n < 10); interleaved/randomized arm order; manifests and validity checks per M4 EV-1…EV-14; INVALID runs rerun; all runs retained. Primary comparisons are **paired by seed**. T3 multi-host only if owner-approved hardware exists (O6-6); otherwise network claims are not made. Time plan: estimate from M4 per-cell durations × cells; submit to the owner as a budget (O6-3) before starting; reduction order: secondary scenarios, then secondary families, never repetitions below the minimum and never the primary endpoints' cells.

### M6-07 Ablations and sensitivity
Ablations: each mechanism of the retained capability disabled in turn (shedding, stale extension, refresh scheduling, concurrency cap, controller feedback vs fixed schedule); eviction/admission choice fixed vs adaptive; shadow vs no shadow; canary vs direct apply. Sensitivity: controller/baseline parameters ±50% (one-at-a-time and a small factorial), origin capacity and latency scale, workload skew α, object-size distribution, TTL distribution, load level, retry policy. Report effect sizes with intervals; identify the regime boundaries (this feeds *narrowing* decisions).

### M6-08 Overhead, resource cost, failure modes, safety
Intelligence modes off/observe/advisory/canary/active on Core items and end-to-end workloads (paired, VALID): hot-path latency, throughput, CPU, RSS and accounted bytes, intelligence structures' memory; failure-mode scenarios — retry storm (`O-RT`), cold start (`O-CO`), outage and slow recovery (`O-OUT`, `O-REC`), flapping (`O-FL`), signal poisoning, intelligence-layer crash/stall (rollback proof in the integrated build), kill-switch latency; safety invariants (cap violations = 0, no worse-than-holdback sustained regression). Also: freshness evaluation (staleness distributions, fraction served stale, age percentiles) and the `stale_cap_violations` counter.

### M6-09 Distributed evaluation (conditional)
Only the contract established in M3 (failure model, measured bounds). Repeat the contract scenarios on the integrated build (multi-process harness); physical network claims require T3. If M3 selected Option C, this package contains no distributed evaluation and the manuscript/README make no distributed claims.

### M6-10 Statistical analysis (preregistered methods)
Paired-by-seed comparisons; effect sizes as ratio of medians (relative change) with BCa bootstrap 95% CIs (≥ 10,000 resamples, seeded) and a non-parametric standardized effect (Cliff's δ) alongside; multiple-comparison control across the family of primary endpoints (Holm–Bonferroni) and a reported (not hidden) count of all comparisons; heavy tails handled via quantile-based metrics with bootstrap intervals; autocorrelation handled by block bootstrap within runs where time series are summarized; **all** scenarios/arms reported including null or negative results; distribution plots (not only means); exact interval methods for proportions; no outlier removal except the INVALID rule; post-hoc analyses labeled exploratory. Output: `docs/research/results/**` (tables, figures generated by scripts from hashed raw data); a results manifest linking every number to its raw files.

### M6-11 Reproducibility and artifact validation
Clean-machine procedure: a fresh checkout at the frozen tree hash, pinned toolchain (rustc/cargo versions, `Cargo.lock`), node version and lockfile, one documented command sequence that regenerates every table/figure from raw data (and, optionally, re-runs T1 fully and a T2 subset). **Validation:** an independent executor (another agent session or person, ideally on a different machine) follows only the README; T1 outputs hash-match (same platform) or match statistically (different platform); regenerated tables equal reported tables. Artifact contents: source snapshot or commit, scripts, configs, seeds, raw data (hashed), manifests, licences, README with expected runtimes and known sensitivities. Heavy packaging choices (containers, vendored dependencies) are O7-x decisions in M7; M6 verifies the procedure works.

### M6-12 Claims-ledger resolution and recommendation
For each ledger claim (C1–C13 plus any new): status SUPPORTED / NARROWED / REFUTED / NOT EVALUATED with evidence pointers (tables, manifests), conditions, limitations and counterexamples. **Decision rules (frozen in M6-02):** *Validate (→ MVP-A)* iff H1 primary endpoint satisfied on held-out families with the preregistered CI/effect, constraints and overhead satisfied, F-a…F-f not triggered, T1 and T2 directions agree, and reproducibility PASS. *Narrow* iff the effect holds only within an identified regime (documented boundary and detector) — the capability may ship only inside that regime or in advisory mode (owner chooses). *Reject* otherwise — recorded as a negative result with conditions. **Outcome → shipping configuration:** MVP-A: validated capability available in `canary`/`active` with proven guardrails, flagged, default per owner decision; MVP-B: best supported static configuration (BL-12b-derived or better) as default, intelligence in `observe`/`advisory` (functional), the measurement study and negative/narrowed result documented. The research lead issues a recommendation; **the owner decides** (O6-5).

### M6-13 Engineering MVP gate verification
Concrete gates, each with evidence (and independent of research outcome): E-1 full test suite and lint baselines green on the frozen build; E-2 no open P0/P1 defects (defect register); E-3 resource bounds verified (soak from M2 repeated on the integrated build, bytes, queues, tasks); E-4 hard caps verified in the integrated build with intelligence active (adversarial tests); E-5 telemetry truthfulness contract tests pass (live payload has no simulated fields; unavailable has no `v`); E-6 intelligence defaults safe (`off` unless the owner decided otherwise) with kill switch and watchdog tested; E-7 rollback proof reproduced on the integrated build; E-8 documentation matches behavior (CORRECTIONS cleared or current); E-9 distributed claims limited to the M3 contract; E-10 reproducibility PASS; E-11 security checklist inputs delivered to M7; E-12 claims ledger fully resolved. **MVP definition components M-1…M-11 (`mvp_and_milestones.md` §1) are checked one by one.**

## 4. Acceptance criteria and exit gate G-MVP
| ID | Criterion | Blocking |
|---|---|---|
| AC6-1 | E0; evidence inventory; sealed seed set S verified unused by any M4/M5 run (log grep + hash check); sealed family (if approved) never referenced | yes |
| AC6-2 | Prior-art protocol complete with coverage log; independent verification done; novelty snapshot written; no unresolved SUBSTANTIAL overlap (or claim narrowed accordingly) | yes |
| AC6-3 | Pre-registration frozen (hash/date) before any evaluation run; amendments logged | yes |
| AC6-4 | Analysis code validated on synthetic known-effect and null datasets; code-reviewed before data | yes |
| AC6-5 | Primary evaluation complete per matrix with VALID classes; INVALID reruns logged; deviations listed | yes |
| AC6-6 | Ablations and sensitivity complete or NOT RUN with owner-accepted reasons | yes |
| AC6-7 | Overhead/failure-mode/safety evaluation PASS or findings recorded with decisions | yes |
| AC6-8 | Statistical reporting follows the plan: all comparisons reported, intervals/effect sizes, correction applied, post-hoc labeled | yes |
| AC6-9 | Reproducibility: independent clean-machine reproduction PASS | yes |
| AC6-10 | Claims ledger fully resolved with evidence; every claim has conditions and limitations | yes |
| AC6-11 | Engineering MVP gates E-1…E-12 PASS (or owner waiver) | yes |
| AC6-12 | Recommendation (MVP-A/MVP-B/narrowed) delivered by the research lead; **owner decision recorded** | yes |
**G-MVP passes** when all blocking criteria are PASS (or waived in writing) and the owner records the MVP outcome. **Disabled until G-MVP:** any public performance or intelligence claim, enabling `canary`/`active` by default, M7 release work.

## 5. Research-integrity rules (apply throughout)
Never convert simulation into a real-system claim (T1 evidence is labeled SIMULATED; T2 is loopback). Never report unavailable as zero. A completed benchmark is not a valid benchmark. Measurement validity is judged separately from outcome. Preserve negative and inconclusive results. No tuning on held-out/sealed data; no re-running until a result looks good (reruns only for INVALID, logged). No post-hoc endpoint substitution. Candidate/baseline contamination checks (A/A) are rerun on the integrated build. All tools/versions/configs/platform limitations recorded.

## 6. Risks
| Risk | Containment |
|---|---|
| Hindsight bias / forking paths | pre-registration, sealed seeds, analysis dry-run, post-hoc labeling |
| Weak baselines inflate gains | M4 fidelity review, oracle upper bound, equal tuning budgets |
| Prior-art surprise late | closure now, novelty snapshot, claim narrowing rule |
| Noisy platform | VALID-only, interleaving, labeled platform, BLOCKED if unfit |
| Evaluation bug discovered mid-run | bug protocol: amend, fresh seeds, keep old results |
| Time overrun | documented reduction order; primary cells protected |
| Over-claiming from loopback | label rules; T3 only with hardware |
| Reproducibility fails on another machine | catch in M6-11; fix before gate |

## 7. Authorized files / prohibitions
May create: `docs/research/preregistration_m6.md`, `docs/research/results/**`, analysis scripts (location per O6-4), `lab-results/**` (untracked), artifact scaffolding, claims ledger updates, completion report. **No product source changes** except fixes via the bug protocol (owner-visible, ledgered). CER-3 prohibitions apply; no commits, no deletion of raw data.

## 8. Owner-reserved decisions
O6-1 approve and freeze the pre-registration · O6-2 primary endpoints and δ_min if not already fixed · O6-3 seeds/power analysis outcome and the evaluation time budget · O6-4 analysis/plotting tooling and any dependency · O6-5 **MVP-A / MVP-B / narrowed outcome** · O6-6 T3 hardware/network emulation · O6-7 claims scope and novelty statement wording · O6-8 independent executor for artifact validation · O6-9 any waiver of AC6/E-gates. **Agents may decide:** run scheduling within the frozen matrix order, script internals, figure styling within templates, report layout.

## 9. Stop conditions
Pre-registration violated or unfrozen at run start; contamination evidence (candidate affects baseline arm); sealed seed reuse detected; INVALID rate above the preregistered bound; prior-art overlap judged substantial; artifact reproduction fails twice; analysis dry-run fails to recover planted effects; need for a source change outside the bug protocol.

## 10. Completion report
CER-8 plus: prior-art coverage log and novelty snapshot, pre-registration (hash/date) with amendments, evaluation matrix coverage table, primary/secondary results with intervals (all comparisons), ablations/sensitivity, overhead/failure-mode tables, reproducibility record, ledger resolution table, E-gate table, recommendation and the owner's recorded decision, claim list that M7 may use (and the explicit list of claims it may **not** make), next-phase proposal only.
