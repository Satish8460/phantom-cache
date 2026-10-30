# PhantomCache — Skill and Capability Strategy

**Date:** 2026-10-28. **Status:** Proposal. Nothing in this document has been installed, created, or configured. No skills were created; no settings were changed.
This is workstream W0 of `plan_of_action.md`.

## 1. Evidence legend
- **[VERIFIED-ENV]**: observed in this session's environment (tool/skill listing, files on disk, command output).
- **[READ]**: I opened the skill's instruction file.
- **[UNVERIFIED-EXT]**: an external resource I know of from memory. Existence, version, license and maintenance state are **not** checked. Must be inspected before adoption.

## 2. Inventory of what actually exists here

### 2.1 Skills (listed in this session; files under `~/.claude/skills/synced/…`) [VERIFIED-ENV]
| Skill | Relevance to PhantomCache | Read? |
|---|---|---|
| `deep-research` | Literature review: a coordinator that spawns research subagents and a report-writer. Directly reusable for related-work work. Its own rules say the coordinator should not do research itself. | [READ] header only |
| `skill-creator` | Creating and evaluating our own skills, with an eval/benchmark loop (`scripts/`, `eval-viewer/`, `agents/`). Needed for §5. | [READ] header only |
| `code-review`, `simplify`, `security-review` | Independent review of diffs; usable for review gates. Not Rust-specific. | listing only |
| `dataviz`, `artifact-design`, `artifact-diagramming` | Chart/diagram quality for the UI and paper figures. | listing only |
| `init` | Generates `CLAUDE.md`. Candidate vehicle for persistent project rules (see §5, S1). | listing only |
| `docs`, `docx`, `pdf`, `pptx`, `xlsx` | Paper/report/artifact packaging. Of limited use for a LaTeX/Markdown paper workflow. | listing only |
| `run` | Launch and drive the app to confirm a change in the real app. Useful for UI/telemetry validation. | listing only |
| `loop`, `schedule` | Recurring or scheduled jobs (e.g., nightly benchmark runs). | listing only |
| `update-config`, `fewer-permission-prompts`, `keybindings-help` | Harness configuration. **Out of scope** until approved. | listing only |
| `figma:*`, `claude-api`, browser/computer-use skills, `morning`, `google-workspace`, `import-memory`, `plugin-authoring` | Not relevant to PhantomCache's goals. | — |

**Missing (not in the listing):** no Rust-specific skill, no distributed-systems or TLA+ skill, no benchmarking/statistics skill, no systems-paper-writing skill. [VERIFIED-ENV by absence from listing.]

### 2.2 Agents [VERIFIED-ENV]
`general-purpose` and `claude` (full tools), `Explore` (read-only search), `Plan` (design, read-only), `claude-code-guide` (questions on Claude Code itself), `statusline-setup`. There are **no specialist** (Rust, distributed-systems, statistics) agents. Specialization has to come from our own briefs/skills. `Workflow` orchestration exists but requires explicit user opt-in.

### 2.3 Tooling on this Windows machine [VERIFIED-ENV by `command -v`]
| Present | Absent |
|---|---|
| `cargo`/`rustc` 1.98.0, Node + npm (`ui/node_modules` present), Java (JRE path only; version and JDK-vs-JRE not checked) | `python`/`python3`, `perf`, `cargo-flamegraph`, `cargo-criterion`, `hyperfine`, `cargo-nextest`, `cargo-llvm-cov`, `cargo-fuzz`, `cargo-deny`, `cargo-audit`, TLC (TLA+) |
`Cargo.toml` has no `criterion` and no `[[bench]]`. Absence of Python matters: statistical analysis would be done in Rust, R, or Node unless Python is installed (a decision for the user).
Also: Windows has no `perf`; profiling there realistically means ETW/WPA, `samply`, or running on Linux/WSL. This is a real constraint on the baseline workstream and is flagged as a decision in the POA.

### 2.4 External resources worth evaluating [UNVERIFIED-EXT]
Adopt only after inspecting source, license, and maintenance, and only with the user's approval.
- **Benchmark/profiling:** Criterion.rs, `cargo-flamegraph`, `samply`, `hyperfine`, DHAT, `tokio-console`.
- **Concurrency correctness:** `loom`, `shuttle`, `proptest`, `cargo-fuzz`, Miri.
- **Deterministic simulation:** `turmoil`, `madsim` (Rust deterministic simulation for Tokio-style code).
- **Cache research artifacts:** `libCacheSim` and its trace collections (to be checked in the literature review for exact traces and licenses).
- **Formal methods / fault testing:** TLA+ with TLC or Apalache; Jepsen-style history checking methodology.
- **Statistics:** methodology from systems-benchmarking literature (e.g., Georges et al. on statistically rigorous evaluation; Kalibera & Jones on benchmarking with repetition). Citations unverified.
Each of these is "worth evaluating", not "recommended for installation".

## 3. Project-wide core skill matrix

Eight candidate categories collapsed to **five capability bundles** plus **one standing guard**:

| Bundle | Covers | Class | How we deliver it |
|---|---|---|---|
| **K1 Research Intelligence** | Search, source verification, novelty, gap analysis | Available, needs adaptation | `deep-research` workflow + a short PhantomCache **verification checklist** (primary source opened, claims tagged, corrections logged). Skill S3 below. |
| **K2 Systems & Algorithms** | Rust, Tokio, memory, locks, cache algorithms, complexity | Missing as a skill; **no value in a generic one** | Delivered as reference notes inside S2 and the benchmark protocol, not a standalone skill. Use `Explore`/`Plan` agents + profiling tools once installed. |
| **K3 Distributed Correctness** | Consistency claims, invalidation races, recovery, formal reasoning | Missing, worth creating (S5) | Checklist-driven review + optional TLA+ models. |
| **K4 Experimentation & Evaluation** | Workloads, benchmarks, statistics, reproducibility, baselines | Missing, worth creating (S2) | PhantomCache benchmark-integrity protocol + tool adoption. |
| **K5 Product Truthfulness (UI/telemetry)** | Measured/simulated/inferred/unavailable labeling | Missing, worth creating (S4) | Data-provenance contract + checklist. |
| **G0 Change Safety (standing guard)** | Working-tree preservation, approval boundaries, `-j 1` | Available but needs adaptation | `CLAUDE.md` (via `init`) from `docs/handoff/06` rules (S1). Cheap; high value. |

Security/reliability (the user's category 5) is not its own bundle: it is already well covered by the existing `security-review` skill and by tests in the repo; threat-modeling lives in `docs/security/`. It is folded into K3 (failure semantics) and S5.
Architecture documentation and engineering coordination are folded into S1: the existing handoff plus `docs/research/` already do the job; the missing piece is a *short invariants file*, not a skill.

## 4. Phase-to-skill mapping

Phase labels refer to `plan_of_action.md` (W-stages), with the historical phases for maintenance.

| Stage | Primary bundles | Existing helpers | Notes |
|---|---|---|---|
| Historical P1–P2 (core, ingress, egress security) | K2, K5 (telemetry honesty), G0 | `security-review`, `code-review` | Maintenance only. |
| Historical P3–P4 (mTLS, framing, invalidation, anti-entropy) | K3, G0 | `security-review`, `code-review` | Needs re-examination of staleness claims (see `code_findings.md` F14–F18). |
| W1 Measurement & baselines | K4, K2, K5 | none | Hard dependency for everything after. |
| W2 Cache core | K2, K4 | `code-review`, `simplify` | Only after W1 baselines exist. |
| W3 Origin model & lab | K4, K3 | `loop` for nightly runs | Simulated vs measured separation. |
| W4 Controller | K4, K1 | `code-review` | Needs W1 and W3. |
| W5 Distributed correctness | K3, K4 | `security-review` | Parallel to W4 after W1. |
| W6 UI | K5 | `dataviz`, `artifact-design`, `run` | Starts with W1 (truthful data), not at the end. |
| W7 Evaluation | K4, K1 | `dataviz`, `xlsx` | |
| W8 Paper | K1, K4 | `deep-research`, `docs` | |
| W9 Deployment readiness | K3, K5, G0 | `security-review` | |

## 5. Prioritized PhantomCache-specific skills worth creating

Only five, in this order. None has been created.

| # | Skill | Verdict | Why / scope |
|---|---|---|---|
| **S1** | **Change-Safety + Project Invariants** (as `CLAUDE.md`, not a standalone skill) | **Create first** | Encodes: no destructive git, no commit/push without instruction, `-j 1`, verify HEAD/branch/status before and after, no new phase without approval, plus a ≤1-page list of code invariants (generation fencing, tenant epoch rules, lock order shard→tenant_epochs). Lowest cost, immediate payoff. Uses the `init` skill as the vehicle. |
| **S2** | **Benchmark Integrity** | Create second, together with W1 | A protocol/checklist: record hardware, OS, build profile (release vs debug), seed, repetitions, warm-up, baseline identity; reject comparisons of debug builds, single runs, or numbers from `sim/` presented as measurements; require confidence intervals. Includes a results-manifest template. |
| **S3** | **Research Novelty Review** | Create third (small) | Procedure: claim → search → open primary source → record verified fields → label (solved / partly / open) → log corrections. Wraps `deep-research`; adds a citation-verification table format used in `literature_review.md`. |
| **S4** | **UI Telemetry Integrity** | Create with W6 start | Data-provenance contract: every field has `source ∈ {measured, simulated, inferred, unavailable}`; no UI-side arithmetic that fabricates a metric; tests that fail when a simulated field lacks a label. This is more *code contract + test* than prose. |
| **S5** | **Distributed Failure Review** | Create when W5 starts | Checklist: for each guarantee, name the model (partition, loss, reorder, crash, restart), the invariant, the mechanism, the test that falsifies it; forbid "bounded staleness" without a stated bound and measurement. |

**Not creating (unnecessary or duplicative):** a Rust-expert skill, an algorithms skill, a paper-writing skill (the checklist in S3 plus normal drafting suffice), an architecture-documentation skill (the invariants file in S1 suffices), a separate performance-investigation skill (merged into S2: "profile before recommending").

## 6. Order, dependencies, expected value

1. S1 now (zero dependencies, prevents the main risk category in this repo).
2. S2 with W1 start; depends on the decision about profiling environment (Windows vs WSL/Linux).
3. S3 immediately useful for the in-flight literature work.
4. S4 when UI telemetry work starts; depends on a decision about the SSE schema.
5. S5 when W5 starts; depends on completing the inspection of `router.rs`/`node.rs`/`frame.rs`/`anti_entropy.rs`.

## 7. Validation strategy (does a skill actually help?)
Use `skill-creator`'s eval approach: for each skill, 3–5 representative prompts from real PhantomCache situations, run with and without the skill, compare. Examples:
- S2: give it a deliberately flawed benchmark report (debug build, n=1, simulated numbers) and check whether the flaws are flagged.
- S4: give it the current `spawn_simulation_loop` output and check that it reports the unlabeled synthetic fields (known ground truth: `code_findings.md` F11).
- S3: give it a claim from `initial_assessment.md` §3 with a planted wrong attribution and check that it is caught.
- S5: give it the "T_stale = 0" statement and check that it asks for the failure model and bound (known open item: F18).
A skill is kept only if it beats the no-skill run on these seeded tests; otherwise it is deleted.

## 8. Maintenance strategy
- Each skill file has an **owner**, a "last validated against commit <sha>" line, and a list of source documents it depends on.
- Re-validate S1 and S2 whenever `Cargo.toml` profile settings, test count or toolchain change; S4 whenever the SSE schema changes; S5 whenever the wire protocol or recovery state machine changes.
- Invariants in S1 are mirrored by tests wherever possible, so staleness is detected by a failing test, not by reading.
- Quarterly (or per-milestone) review: delete skills not invoked.

## 9. Risks of this workstream
- Process overhead outweighing value: hence five items, one of which is a text file.
- Third-party skills or crates introduce supply-chain risk: nothing external is adopted without source inspection and explicit approval.
- Skills encode wrong assumptions that become stale: addressed by the validation and maintenance rules above.

## 10. Decisions needed from the user
1. Permission to create S1 (`CLAUDE.md`) — it is a new file in the repo root.
2. Where benchmarks and profiling should run: native Windows, WSL, or a Linux host/VM (this affects tooling, flamegraphs, and the credibility of results).
3. Whether to install Python or use R/Rust/Node for statistics.
4. Permission to add dev-dependencies (`criterion`, etc.) when W1 starts.
