> Notice (2026-10-30): The status "PROPOSED" below is the state when this package was written and was never amended. Work attributed to M7 exists in the repository, but there is no M7 progress log or completion report, and G-RELEASE was not attempted. No dated written owner authorization of this package is recorded in the repository (an evidence gap, not proof that none was given; the owner may hold one elsewhere). Current milestone status: [docs/STATUS.md](../STATUS.md); per-milestone detail: [accountability report](phantomcache_milestone_accountability_report.md).
>
> Original text below is unchanged.

# Phase 7 Execution Package — M7: Productization & Research Release

**Status: PROPOSED — not authorized.** Requires **G-MVP** (M6 complete; MVP-A or MVP-B recorded by the owner) and, if the owner confirms decision **D-8**, the UI gate **U1** from M8 (see `phase_8_execution_package.md`). Common rules: `phase_execution_packages_index.md` §4. **This phase ships only what the preceding gates support. The release gate works identically for MVP-A and MVP-B (parametrized by the owner's recorded outcome). Research-paper acceptance is not a prerequisite for engineering release.**

## 1. Purpose, scope, non-goals
**Purpose.** Make the validated system installable, operable, secure, observable and documented; prove every public claim traces to evidence; package the reproducible research artifact; assemble release notes and a manuscript that is consistent with the evidence. **Scope:** configuration validation, failure containment, observability export, security review, performance-regression gating, packaging and reproducible deployment, documentation (docs-as-tests), claims audit, release artifacts, release-readiness review. **Non-goals:** new features, new experiments (a new idea ⇒ a new cycle), UI redesign (M8), venue submission, publishing/pushing/tagging (owner actions).

## 2. Starting state (PROVISIONAL; verified at M7-00)
From M1–M6 (to be confirmed, not assumed): frozen integrated build with hashes; claims ledger resolved; MVP outcome; intelligence modes and defaults as decided by the owner; schema-v2 telemetry catalog; configuration reference started in M2/M3/M5; soak and reproduction procedures; the Phase-1 UI plus any M8 progress. Known facts from earlier inspection that bound this phase: the HTTP server binds `PHANTOM_BIND_ADDR` (default `127.0.0.1:8085`) with JWT/RBAC and TLS options, fail-closed startup validation exists for ingress and upstream (`security/mod.rs`) [V], the UI hard-coded its gateway URL before Phase 1 [V], `/health` exists and diagnostics endpoints are `.../summary` and `.../events` (not `/api/v1/diagnostics`) [A]; no `/metrics` or readiness endpoint, no packaging/installer, no CI configuration known, no release process exist [no evidence found].

## 3. Work packages
| ID | Title | Depends on |
|---|---|---|
| M7-00 | Entry verification; release variant declared (MVP-A / MVP-B) | G-MVP |
| M7-01 | Configuration system: validation, defaults, effective-config report | M7-00 |
| M7-02 | Failure containment and operability | M7-01 |
| M7-03 | Observability export and logging hygiene | M7-01 |
| M7-04 | Security review and hardening | M7-01…03 |
| M7-05 | Performance-regression gate | M7-00 |
| M7-06 | Packaging and reproducible deployment | M7-01 |
| M7-07 | Documentation as tests | M7-01…06 |
| M7-08 | Truthful-dashboard and operator-task acceptance (with M8 U1) | M7-03 |
| M7-09 | Claims-to-evidence audit and manuscript consistency | M7-00 |
| M7-10 | Reproducible research artifact (final) | M7-09 |
| M7-11 | Release candidate, release notes, release-readiness review | all |
| M7-12 | Completion report | M7-11 |

### M7-01 Configuration validation
Single documented configuration surface (env vars and optional file — which one is the owner's choice, O7-3): schema with type, unit, range, default, security impact for every key (including `PHANTOM_*` from M1–M5: bind addr, auth, JWT settings, upstream policy/TLS, cluster settings, byte budget, freshness/stale cap, intelligence mode, simulation flag). **Startup validation is fail-closed:** invalid values, conflicting settings (e.g., public bind without TLS/auth, cluster mode without required material, intelligence `active` without caps) produce a clear error and non-zero exit; unknown `PHANTOM_*` variables produce a warning listing the key. An **effective-config report** (command or log at startup) prints all resolved values with secrets redacted and the source of each value (default/env/file). **Tests:** a validation matrix (≥ 1 test per key and per conflict rule), redaction test (no secret ever printed), idempotence, documentation-vs-code check (every key in docs exists in code and vice versa — scripted). **Hot-reload of cluster membership** (RISK-P2-02) only if M3 kept clustering and the owner asks (O7-5).

### M7-02 Failure containment and operability
Graceful shutdown (signal handling, connection draining with a bounded deadline, in-flight fills completed or abandoned with waiters notified), crash behavior defined (panic policy; supervisor-friendly exit codes), request/connection limits and timeouts audited against documented values, startup self-checks (config, listener bind, certificate readability, origin reachability as a *warning*), health vs readiness semantics (readiness = accepting traffic with valid config; any new endpoint is O7-4), intelligence **kill switch** and watchdog verified in the release build, behavior when the origin is down at startup, when memory budget is exhausted, when disk/cert files change. **Tests:** signal/shutdown tests under load (no lost accounting, no leaked tasks), kill-switch latency measurement, restart-under-load (cold start) scenario from M4, resource-exhaustion tests (connection flood, oversized headers/bodies, slow clients). **Acceptance:** documented behavior for each failure; no unbounded growth in any test.

### M7-03 Observability export and logging hygiene
Expose the schema-v2 catalog in a standard scrape format if the owner approves a new endpoint (O7-4: `/metrics`; naming mirrors the catalog; provenance carried in HELP text and a documented mapping because scrape formats cannot carry it); only `measured`/`derived` series are exported, `unavailable` series are *absent* (never 0), simulated series never exist in live mode. Logs: structured, leveled, no secrets/tokens/request bodies/PII (test with a secret-canary: inject recognizable secrets into config/requests and assert they never appear in logs, metrics, diagnostics, error pages, or SSE), request IDs, decision IDs for intelligence events. **Tests:** exporter contract test (names, types, units, absence rule), secret-canary test, log-volume bound test.

### M7-04 Security review and hardening
Deliverables: refreshed threat model (assets, trust boundaries, attackers) updating `docs/security/*`; checklist run with PASS/FAIL/NOT RUN for: ingress authN/Z (JWT validation, role matrix, tenant scoping and cross-tenant tests), admin/control endpoints (every state-changing endpoint requires admin role; the simulation control endpoint is absent or authenticated in live mode), intelligence control plane (mode changes authenticated and audited; no unauthenticated way to enable `active`), SSRF and upstream policy, TLS settings (ingress, upstream, cluster mTLS), HTTP parsing hardening (smuggling cases from M2-12), DoS limits, cache-poisoning and cache-deception cases (`Set-Cookie`, `Authorization`, `Vary`, path normalization, query handling, tenant key construction), secret handling (no secrets in logs/config dumps, key file permissions documented), dependency review (`Cargo.lock` / npm lock inventory, license inventory, advisory check — tools such as `cargo audit`/`cargo deny` are user-space installs but external software ⇒ owner approval O7-6), UI security (no `dangerouslySetInnerHTML`, content-security-policy suggestions, CORS policy, SSE auth). Findings classified (Critical/High/Medium/Low) with fixes or owner-accepted risk. **Acceptance:** zero open Critical/High; each Medium/Low fixed or accepted by the owner in writing.

### M7-05 Performance-regression gate
Scripted comparison of the Phase-1 Core items on the release build against the frozen M6 reference (same platform, VALID, paired/interleaved, ≥ 10 repetitions): **no regression beyond the noise bound** (or an owner-accepted exception); end-to-end item with intelligence in each shipped mode; startup time and idle memory recorded. The gate is a repeatable script, not a one-off. **Acceptance:** PASS only with VALID runs; an environment-unfit platform ⇒ BLOCKED.

### M7-06 Packaging and reproducible deployment
Release build recipe: pinned toolchain, `--locked`, documented `RUSTFLAGS`/profile, build twice and compare binary hashes (reproducibility **reported honestly**: bit-identical or an explained difference); platform matrix decided by the owner (O7-7: Windows x86-64, Linux x86-64 at minimum); SHA-256 checksums; license files and third-party notices; optional SBOM (tool ⇒ O7-6); UI build artifact built to a clean directory; example configurations (minimal, TLS, clustered-if-supported, intelligence modes); service-manager examples as documentation (systemd unit, Windows service wrapper notes) — not installed automatically; upgrade and rollback procedure (stateless in-memory cache ⇒ restart semantics documented, including cold-start behavior and the stale-serving cap's role); container images only if the owner approves a container runtime (O7-6). **Tests:** clean-machine install from the release bundle following only the docs, health check, acceptance scenario script, upgrade N→N+1 and rollback.

### M7-07 Documentation as tests
Docs set: quickstart; configuration reference (generated/checked against code); operations guide and **runbooks** (origin outage and the stale cap, cold start, high miss rate, memory budget pressure, intelligence misbehavior and kill switch, certificate rotation, upgrade/rollback); architecture overview and limits; supported-configurations matrix; API reference derived from the real routes (no invented endpoints); security guide; intelligence guide (modes, safety limits, how to read decision records); glossary of **measured / derived / simulated / inferred / unavailable**; known limitations; FAQ. **Docs QA:** every command block is executed on a clean machine by a script or a second agent; broken commands fail the gate; every number in docs carries a trace tag (M7-09).

### M7-08 Truthful dashboard and operator-task acceptance
Verify with the release build that: live mode shows no simulated values; every field is badged; unavailable renders as such; unknown fields never silently become zero; disconnected/stale-data states are explicit; the lab/simulation views are bannered. **Operator tasks** (scripted, executed by someone who did not build the UI, without reading logs): *T-A* find out why the miss rate rose (Overview → Traffic/Origin), *T-B* confirm the gateway handled an origin outage within the stale cap (Freshness/Origin), *T-C* find and explain the last intelligence decision and how to disable intelligence, *T-D* identify configuration problems from the diagnostics view. Success criteria: completion without internal logs, time and error counts recorded. This package supplies the product-side acceptance; the design-quality work is M8.

### M7-09 Claims-to-evidence audit and manuscript consistency
**Claim register** extracted from README, docs, UI copy, release notes, the manuscript and slides: each performance, freshness, safety, intelligence or distributed statement maps to a `claims_ledger.md` entry + evidence file(s) + conditions (platform, tier, workload family, failure model). **Rules:** SUPPORTED claims only, with their conditions attached; NARROWED claims restated with the regime; REFUTED/NOT EVALUATED claims removed or explicitly framed as negative/limitation; no simulated result presented as measured; no "first"/"novel" statements beyond the M6 novelty snapshot; numbers in text equal numbers in generated tables (scripted diff against the M6 results manifest). A **claims lint** script flags numeric/performance statements without a trace tag. **Manuscript consistency:** terminology (simulated vs measured), baselines named as in M4, hypotheses and endpoints as preregistered (deviations disclosed), limitations and threats to validity sections present and consistent with the register, artifact description matches M7-10. The manuscript is a deliverable draft; **acceptance by a venue is not a gate.**

### M7-10 Research artifact (final)
Bundle: frozen source snapshot/commit reference, pinned toolchain description, scripts regenerating every table/figure from hashed raw data, scenario files/seeds, manifests, license, README (expected runtimes, known sensitivities, platform notes), checksums; independent clean-machine reproduction repeated on the final bundle (from M6-11). Packaging choices (container, vendored crates, archive hosting) are owner-reserved (O7-6/O7-8).

### M7-11 Release candidate and readiness review
RC build → soak (M2 procedure on the RC) → acceptance scenario script end to end (healthy, origin outage, cold start, invalidation, intelligence modes per variant, kill switch) → freeze. **Variant parametrization:** *MVP-A*: also verify validated capability behavior in `canary`/`active` with caps, rollback, holdback, overhead; *MVP-B*: verify static default configuration, intelligence `observe`/`advisory` function and explanations, and that no refuted capability is enabled. Release notes: version, changes, supported configurations, limitations, known issues, MVP statement, evidence references. **No tag, commit or push is performed by an agent;** the owner executes publication.

## 4. Acceptance criteria and exit gate G-RELEASE
| ID | Criterion | Blocking |
|---|---|---|
| AC7-1 | E0; variant declared; frozen inputs hashed | yes |
| AC7-2 | Configuration validation matrix PASS; effective-config report redacts secrets; docs↔code key check PASS | yes |
| AC7-3 | Failure-containment tests PASS (shutdown, restart under load, exhaustion); behaviors documented | yes |
| AC7-4 | Observability contract + secret-canary tests PASS; unavailable series absent | yes |
| AC7-5 | Security checklist complete; zero open Critical/High; Medium/Low dispositioned by the owner | yes |
| AC7-6 | Performance-regression gate VALID and within bound (or owner exception) | yes |
| AC7-7 | Clean-machine install, acceptance script, upgrade and rollback PASS for the release bundle | yes |
| AC7-8 | Docs-as-tests PASS; runbooks exercised at least once each | yes |
| AC7-9 | Dashboard truthfulness checks and operator tasks T-A…T-D PASS | yes |
| AC7-10 | Claims audit: 100% of public claims traced; claims lint clean; manuscript numbers equal generated tables | yes |
| AC7-11 | Research artifact reproduces on a clean machine | yes |
| AC7-12 | Variant-specific verification (A or B) PASS | yes |
| AC7-13 | M8 gate U1 PASS **if** the owner confirmed D-8 | conditional |
| AC7-14 | Ledger clean; no unapproved dependency/tool; no git history changes by agents | yes |
**G-RELEASE passes** when all blocking criteria are PASS (or owner-waived in writing) and the owner approves publication. **Not required:** venue acceptance; M8 gate U2 (unless the owner states otherwise).

## 5. Measurement and telemetry integrity
Release-phase measurements (regression gate, soak, install timings) obey CER-5. Exported metrics follow the provenance rules above. Public performance statements cite VALID results with platform labels and intervals; simulation-derived statements are labeled SIMULATED and are not used for headline claims.

## 6. Risks
| Risk | Containment |
|---|---|
| Overstated claims in README/paper | claims register, lint, owner review, SUPPORTED-only rule |
| Config footguns (public bind without auth, intelligence active without caps) | fail-closed validation matrix, safe defaults |
| Secret leakage via logs/metrics/UI | canary test across all channels |
| Release works only on the dev machine | clean-machine install and docs-as-tests |
| Non-reproducible build | double-build hash comparison, honest report |
| Last-minute regression | performance gate script, soak on RC, freeze rules |
| Third-party/tool supply-chain risk | owner-approved tools only, lockfiles, license inventory |
Known limitations at exit: single-host evidence unless T3 done; lower-trust timing platforms; supported platform matrix limited to what was tested.

## 7. Authorized files / prohibitions
May create/modify with ledger: configuration/validation code, shutdown/containment code, the exporter (if approved), logging hygiene fixes, security fixes found in review, packaging scripts, docs, release notes, artifact scaffolding, UI copy fixes. **Must not:** add features, change measured-result semantics, alter frozen evaluation data, modify M6 results, commit/tag/push, delete files, install software beyond owner-approved user-space tools. CER-3 applies.

## 8. Owner-reserved decisions
O7-1 version number and naming · O7-2 project license/third-party notices acceptance · O7-3 configuration surface (env vs file) · O7-4 new endpoints (`/metrics`, readiness) · O7-5 cluster hot-reload (if clustering shipped) · O7-6 tools/containers/SBOM/audit tooling installs · O7-7 supported platform matrix · O7-8 artifact hosting/packaging choices · O7-9 default intelligence mode in the release · O7-10 acceptance of residual security risks · O7-11 publication actions (commit/tag/push/release). **Agents may decide:** script structure, doc layout, test organization, wording.

## 9. Stop conditions
G-MVP not recorded; a claim without evidence cannot be fixed by rewording (escalate); Critical/High security finding without an agreed fix; performance regression beyond bound with no mitigation; clean-machine install fails twice; secret-canary leak; artifact reproduction fails; any need to change research results or the frozen build outside the bug protocol.

## 10. Completion report
CER-8 plus: release bundle manifest with checksums, configuration matrix results, security checklist table, regression-gate table, install/upgrade/rollback logs, docs-QA results, claims register and lint output, artifact reproduction record, variant verification, residual risks and accepted exceptions, publication checklist for the owner (what the owner must do manually), next steps proposal only.
