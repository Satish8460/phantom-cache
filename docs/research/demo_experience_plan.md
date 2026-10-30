# PhantomCache — End-to-End Demonstration Plan (Phantom Lab experience)

**Status:** PLAN, authorized by the owner directive of 2026-10-29. Nothing below is implemented unless a row says so. Supersedes nothing; extends `master_audit_and_execution_plan.md` (the single authoritative status document) and the M8 package.
**Rule:** every displayed value is measured / derived / simulated / estimated / unavailable. A demo that cannot run the real code path is not shown as a capability.

## 1. The story a viewer must be able to follow (acceptance journey)
1. **Problem + hypothesis** (screen: Lab / Overview intro): origin overload and tail latency under partial origin degradation; hypothesis H1 as pre-registered (`preregistration_m6_v2.md`, when frozen). Stated as a hypothesis, not a result.
2. **Baseline:** replay a seeded workload against the best tuned *static* configuration (BL-12b) in the real cache/gateway code path: hits/misses, origin requests, in-flight origin concurrency, latency distribution, errors.
3. **Perturbation:** choose a scenario (origin slowdown, brownout, flash crowd, retry storm, flapping, hot-key shift) from the scenario catalogue.
4. **System response:** show only mechanisms that exist and are wired: single-flight coalescing (exists), origin concurrency cap (R-01, exists; must be configured), stale serving under the hard cap (exists), invalidation (single-node exists; cluster only within its verified bounds), adaptive decisions (only after the engine is integrated; today: unavailable).
5. **Fair comparison:** same seed, same workload, static vs adaptive, paired by seed, effect size with CI, overhead, regressions. Result may be negative; it is displayed identically.
6. **Why it matters + limits:** plain-language mechanism, operating conditions, failure model, evidence tier (SIMULATED T1 / LOOPBACK T2 / multi-host not run).
7. **Reproduce:** one command/button reruns from retained manifest (seed, workload params, config, source hash); raw results retained under `lab-results/`.

## 2. Dependency-ordered work packages
| ID | Work | Depends on | Owner/files (single writer) | Status |
|---|---|---|---|---|
| D-0 | Honest backend telemetry: BD-01/02/03/05, S-7 (no measured zeros, no simulated provenance in live mode, browser-compatible auth for SSE, schema-1 synthetic fields labeled/removed) | M7 finishes `server.rs` health edit | `src/api/server.rs` | NOT STARTED (blocked by file ownership) |
| D-1 | S-1/S-2 failing tests then fixes (304 refresh, fenced metadata) so the cache behaviour shown is correct | D-0 sequencing on `server.rs`/`cache.rs` | cache + server | NOT STARTED; S-1 stays "likely, not reproduced" |
| D-2 | Real T1 runner over product components (R-02) | none | `tests/support/lab/**` | IN PROGRESS (agent) |
| D-3 | **Promote the runner to a runnable library module** (`src/lab/`, no new dependency) plus a `[[bin]]` or subcommand so the demo does not depend on `cargo test`; the product binary and tests share one implementation (no second mock) | D-2 stable | new `src/lab/**`, `Cargo.toml` (needs owner decision O4-1: bin vs workspace) | PROPOSED |
| D-4 | Lab API: `POST /api/v1/lab/runs` (scenario, baseline set, seed set, n), `GET /api/v1/lab/runs[/{id}]`, `GET .../events` (SSE progress), results with manifest, per-cell raw data link, CI, provenance; control endpoints behind auth; sealed seeds never exposed | D-3, D-0 | `src/api/lab.rs` (new) + route registration | PROPOSED |
| D-5 | Decision-trace API: real executor/engine decisions with reasons, guardrails, canary outcome, rollback; **requires engine integration (claim C8 is currently UNVERIFIED: engine not in serving path)** | decision: integrate or narrow C8 | `src/intelligence/**`, `server.rs` | BLOCKED on decision |
| D-6 | Lab UI: Phantom Lab screen (scenario picker, run controls, live progress, side-by-side static vs adaptive, paired-difference + CI panel, evidence/provenance drawer, run history, reproduce action), progressive disclosure; replaces the placeholder from M8 | D-4 contract | `ui/**` | PROPOSED |
| D-7 | Browser verification at desktop/tablet/mobile widths, light/dark, reduced motion, keyboard, axe-equivalent checks, live SSE reconnect/stale/disconnected states, against a running backend; screenshots retained | D-6 | verification | NOT RUN (no browser verification exists; M8 report) |
| D-8 | Demo script, known-limitations page, evidence index linking each displayed claim to experiment id + raw files | D-2..D-7 | `docs/demo/**` | PROPOSED |
| D-9 | R-03 genuine evaluation → results feed D-6 as retained evidence (not recomputed in the UI) | R-02, protocol freeze | `lab-results/**` | BLOCKED on D-2 + freeze |

## 3. Novelty (what may be shown, per ledger status today)
| Contribution | Mechanism visible? | Real path? | May be presented as |
|---|---|---|---|
| Origin-health-aware control (cap + stale extension + miss shedding) vs tuned static | cap: yes (R-01); stale extension/shedding: only if executor drives them in serving path | partly | **Hypothesis under test**, not a result |
| Single-flight coalescing + generation fencing | yes | yes | Engineering property (tested) |
| Faithful O(1) eviction policies | yes | yes (numbers untraced) | Implemented; speedup figures UNVERIFIED |
| Shadow evaluation / canary / rollback | machinery only | no (C8) | Not presented as product capability until integrated |
| ML selector | no real evaluation | no | Not claimed |
| Distributed invalidation | loopback happy path | partial | Qualified, with the S-3 limits shown |
Prior art (Varnish grace/std.healthy, MIDAS, HotNets'24, RobinHood, DynamoDB refresh) is cited from `literature_review.md` / `novelty_snapshot_m6.md`; RT-0 closure is not complete and is shown as such.

## 4. Gates for "demo complete"
All of: D-0 done with tests; D-2/D-3 done and mutation-checked (disabling the mechanism removes the effect; a planted effect is detected); D-4 contract documented and tested; D-6 implemented against the real API; D-7 executed in a real browser with retained evidence; D-9 results (positive or negative) consumed from retained files; D-8 delivered. A green build alone does not satisfy any gate.

## 5. Decisions needed from the owner
O4-1 (lab packaging: library module + bin vs workspace member); whether to integrate the PIE engine into the serving path now (D-5) or narrow claim C8; browser tooling for D-7 (available browser integration vs manual).

## 6. Owner decisions recorded (2026-10-29)
- **O4-1 approved:** library module plus dedicated binary, no new dependency; demo and tests must call the same experiment implementation over the real cache / executor / gateway behavior; reusable config, seeded workloads, event collection, metrics, result serialization; deterministic documented execution without `cargo test`; no unnecessary workspace restructuring. Before refactoring, inspect R-02's changes and coordinate file ownership; do not overwrite in-progress work.
- **D-5 / C8:** claim C8 is narrowed to "intelligence machinery implemented, not integrated into the serving path" until integration is proven. `PieDecisionEngine`, `route_bucket` and `ShadowEvaluator` are NOT to be integrated into serving merely for the demo. Prerequisite order: architecture inventory -> decision contract, inputs/outputs, safety invariants, measurable hypothesis -> validation on real serving telemetry vs strong static baselines -> shadow/canary/rollback only when specified and testable -> guarded integration with deterministic tests, bounded resources, rollback -> overhead + benefit measurement. Until then the UI may show architecture and offline experiment results but must not portray the machinery as controlling live traffic.
- **D-7:** use available browser automation if possible; no new dependency without evaluation; evidence required at desktop/tablet/mobile widths, light and dark; journey from seeded run to results, overflow, keyboard/focus, loading/running/completed/error/stale/disconnected states, SSE reconnect and authorization failures, accessibility checks, console/network errors; retain screenshots, results, environment and limitations. If impossible: report the exact blocker and a manual procedure; no visual-verification claim until it has occurred.
- **Tooling inventory (2026-10-29):** no browser MCP tool is available in this session (tool search returned only Figma/Weave tools). Chrome and Edge are installed at their default paths; `ui/node_modules` contains no playwright/puppeteer/selenium/axe. Candidate approach without a new dependency: headless Chrome/Edge driven via the DevTools protocol using Node built-ins (needs Node version check), or `--headless --screenshot` for static captures.
