# PhantomCache documentation map

**Status:** Active (index)
**Audience:** anyone new to the repository: users, contributors, reviewers, the product owner
**Last reviewed:** 2026-10-30
**Authority:** this page is a map and holds no facts of its own. Current milestone, gate and test-baseline status lives in [STATUS.md](STATUS.md). Terminology: [glossary.md](glossary.md). Rules for contributors and agents: [contributing.md](contributing.md) and [../CLAUDE.md](../CLAUDE.md).

## What PhantomCache is

PhantomCache is a research prototype of an in-memory HTTP caching gateway with request coalescing (single-flight), written in Rust. It sits between clients and an origin, serves cached responses, and can limit concurrent origin fetches. Cluster operation, Phantom Intelligence (adaptive control, `off` by default) and the lab and dashboard tooling are experimental.

Read [user/limitations.md](user/limitations.md) before relying on anything. In short:

- A benefit from adaptive control has not been established. The only pre-registered test of the adaptive hypothesis (M6, simulated) did not support it: [research/m6_sealed_results.md](research/m6_sealed_results.md).
- No release-grade performance numbers are published. Numbers in the repository are debug-build, loopback, single-host or simulated, and each one carries a label.
- The Scenario B loopback experiment has a binding interpretation. Cite it only together with its qualifications: [demo/scenario_b_interpretation_notice.md](demo/scenario_b_interpretation_notice.md).
- Only Windows x86-64 has been exercised.

## How to build and run

The package defines two binaries, so `cargo run` needs a `--bin` argument ([../Cargo.toml](../Cargo.toml): the package is named `phantom-cache-demo` and builds from `src/main.rs`; a second binary `phantom-lab` is declared in a `[[bin]]` entry with source [../src/bin/phantom-lab.rs](../src/bin/phantom-lab.rs); there is no `default-run` key, so Cargo cannot choose for a bare `cargo run` (standard Cargo behavior, not run here), which is what the root README still shows).

```
cargo build --release --locked -j 1
cargo run --bin phantom-cache-demo -j 1 -- --check-config   # print effective configuration, exit
cargo run --bin phantom-cache-demo -j 1 -- --origin http://127.0.0.1:9000
cargo run -j 1 --bin phantom-lab -- list all                # lab runner CLI
```

On this Windows machine always pass `-j 1` to Cargo. Details and the verified procedures are in [user/install.md](user/install.md), [user/run.md](user/run.md) and [demo/lab_runner.md](demo/lab_runner.md). Dashboard UI (optional): `cd ui && npm ci && npm run dev`, see [../README.md](../README.md) section 6.2.

## Where to find things

| You want | Go to |
|---|---|
| Current status, gates, test baseline | [STATUS.md](STATUS.md) |
| Install, configure, run, operate, troubleshoot | [user/README.md](user/README.md) (configuration keys: [user/configuration.md](user/configuration.md); Lab API: [user/lab_api.md](user/lab_api.md)) |
| What is not established or not implemented | [user/limitations.md](user/limitations.md) |
| Architecture decisions | [research/adr/README.md](research/adr/README.md) (ADR index), [research/architecture_trace_m3.md](research/architecture_trace_m3.md), [research/architecture_investigation.md](research/architecture_investigation.md) |
| Security model and findings | [user/security.md](user/security.md), [security/m7_security_review.md](security/m7_security_review.md), [security/trust-boundary-map.md](security/trust-boundary-map.md), phase records in [security/](security/) |
| Experiments and evidence | protocol chain and results: [research/README.md](research/README.md) (sections Experiments and Results); claims and their status: [research/claims_ledger.md](research/claims_ledger.md); Scenario B: [demo/scenario_B_origin_slowdown.md](demo/scenario_B_origin_slowdown.md), [demo/scenario_b_runbook.md](demo/scenario_b_runbook.md) |
| Phantom Intelligence and the PIE | [research/phantom_intelligence_phase.md](research/phantom_intelligence_phase.md), [research/pie_audit.md](research/pie_audit.md), [research/intelligence_architecture.md](research/intelligence_architecture.md), [research/intelligence_prior_art_coverage.md](research/intelligence_prior_art_coverage.md) |
| Demo and lab | [demo/README.md](demo/README.md), [demo/lab_runner.md](demo/lab_runner.md), [demo-lab/README.md](demo-lab/README.md) (partly stale; see its banner if present), UI design: [design/phantom_lab_ui.md](design/phantom_lab_ui.md), [design/lab_api_contract.md](design/lab_api_contract.md) |
| Operations, release, reproduction | [user/operate.md](user/operate.md), [user/troubleshoot.md](user/troubleshoot.md), [release/reproduction_guide.md](release/reproduction_guide.md), [release/artifact_manifest.md](release/artifact_manifest.md) |
| Tests | [testing/strategy.md](testing/strategy.md) |
| Terminology and ID schemes | [glossary.md](glossary.md) |
| Contributing, doc templates, evidence rules | [contributing.md](contributing.md) |
| Event history (append-only) | [research/progress_ledger.md](research/progress_ledger.md) |
| Documentation audit and plan | [research/doc_audit/00_consolidated_documentation_audit_and_plan.md](research/doc_audit/00_consolidated_documentation_audit_and_plan.md) |
| Handoff packages (historical; superseded where [../CLAUDE.md](../CLAUDE.md) or the research records disagree) | [handoff/00_START_HERE.md](handoff/00_START_HERE.md) |

## Retained evidence directories

These hold generated artifacts and are not edited. Each has a short README:
[demo/browser_verification/](demo/browser_verification/README.md), [demo/scenario_b_verification/](demo/scenario_b_verification/README.md), [../lab-results/](../lab-results/README.md), [../bench-results/](../bench-results/README.md).

## Reading rules

- A passing test does not establish correctness or research validity. Several tests assert stub behavior ([research/code_findings.md](research/code_findings.md)).
- Evidence tags and the labels measured, simulated, inferred, unavailable are defined in [../CLAUDE.md](../CLAUDE.md) section 4 and [glossary.md](glossary.md).
- Historical, frozen, sealed and append-only records are never rewritten; corrections appear as dated notices. See [contributing.md](contributing.md).
