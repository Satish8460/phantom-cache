# Documentation corrections under DEC-020

**Status:** Active (working tree only; not staged, not committed)
**Audience:** the product owner, the lead, and the reviewer of the documentation-only change
**Last reviewed:** 2026-10-30, 17:35 (+05:30 machine clock; `date`)
**Authority:** owner decision DEC-020 (2026-10-30) in `docs/design/OWNER_DECISION_REGISTER.md`: correct demonstrably inaccurate or misleading documentation, preserve historical evidence and frozen records, keep the change documentation-only and separately reviewable.
**Companion:** `docs/release/doc_corrections_DEC020.diff` is a unified diff of only the edits made here (`diff -u` between a pre-edit copy and the edited files; paths `a/` and `b/`). The pre-edit copies are in `D:\Temp\docfix_orig\` (non-durable).

## 1. Scope and exclusions

In scope: README.md, an errata section in CLAUDE.md, the handoff entry files, ADR-001/002/003, STATUS.md, the claims ledger and code findings, the phase reports and progress files that use "linearizable" wording, the Phase 4 security documents, and the PHASE_2 work report.

Not edited (by instruction): any code (`src/`, `tests/`, `benches/`, `ui/`, `Cargo.*`), the comment "Guarantees linearizability" in `src/api/server.rs`, the cluster warning string in `src/config.rs`, frozen or sealed records (preregistration files, freeze files, pins, results, sealed evidence, `lab-results/`, `bench-results/`), anything under `docs/learning/`, anything under `docs/release/commit_lists*/`, the decision register and tracker. `D:\Personal\.phantom-sealed-m6` was not read. No git add, commit, stash, restore, reset, clean or checkout was run.

Method: every candidate from the handbook research was re-checked against the current code (grep and reading) before a document was changed. Evidence tags follow CLAUDE.md section 4; "verified in code" means read in this session. Nothing was executed (no cargo run); test counts were not re-run.

Style: historical documents received an added, dated "Correction notice (2026-10-30)" block; their old text was not rewritten. README received small targeted edits only. CLAUDE.md received one appended errata section.

Line-ending note: README.md has mixed CRLF and LF lines. The edit tool normalised endings; they were restored line by line to the original pattern, so the diff shows only the changed lines.

## 2. Corrections made

| # | File | Location | Before (short) | After | Evidence |
|---|---|---|---|---|---|
| 1 | README.md | Section 2 table, "Request Coalescer" row | "Single-flight ... in `src/core/coalesce.rs`" | Serving path uses `PartitionedCache::begin_fetch_atomic` + `SharedOp`; `Coalescer` is legacy, not used by the HTTP path | `src/api/server.rs:1357,1520` (begin_fetch_atomic); `src/core/cache.rs:1238`; `src/core/coalesce.rs:18,97`; `Coalescer` referenced only in `src/main.rs:62,209` and `src/dist/node.rs:2,24,43` |
| 2 | README.md | Section 2 table, "Distributed Router" row | "partition routing" | Used by DELETE invalidation only; GET never uses the router | `src/api/server.rs` router references only at `:74,:97,:505,:660-664,:1992`; no router use in the GET handler |
| 3 | README.md | Section 2 table, "Cluster Topology & Nodes" row | "Modeled / Simulated" | Mixed: demo nodes simulated with separate caches; mTLS cluster path real, experimental, loopback-tested; local node shares serving cache; peers address-only | `src/main.rs:212-214` (demo nodes), `:332` (`new_with_cache`), `:364-366` (peer descriptors) |
| 4 | README.md | Section 3 mermaid | `Bench -->|Requests| Router`, `Router -->|Primary Node| Coalescer`, node label "Single-Flight Request Coalescer" | GET goes to the single-flight path on the local gateway; router edge is dotted and "DELETE only"; label names `begin_fetch_atomic + SharedOp` | same as 1 and 2 |
| 5 | README.md | Section 4, Tokio | "Tokio 1.38" | `Cargo.toml` requires 1.38 or newer, `Cargo.lock` pins 1.53.1 | `Cargo.toml:7`; `Cargo.lock` tokio entry (version 1.53.1) |
| 6 | README.md | Section 5, Rust | "`cargo 1.75+`" | verified on 1.98.0; probably needs about 1.88 (G4C F28, AI-review finding, not re-derived); no `rust-version` in Cargo.toml | `rustc --version` = 1.98.0 (reproduced here); `Cargo.toml` has no `rust-version` (grep); F28 per `docs/learning/evidence/ch03_evidence.md` item 41 (reported by prior agent) |
| 7 | CLAUDE.md | Appended "Errata" item 1 | "Lock order: shard mutex, then `tenant_epochs`" | No code nests them; real edges listed; deadlock-freedom not tool-verified | `src/core/cache.rs:287,576-577,613-614`; `tenant_epochs` not used outside cache.rs (ch04 item 10) |
| 8 | CLAUDE.md | Errata item 2 | "Faithful Eviction ... O(1)-amortized" | O(1) only without pinned keys (G4A-20); S3-FIFO G4A-02 | `src/core/eviction.rs` structures; G4A review `docs/release/g4/cycle1_core_engine_review.md`; `KeyState.visited` written at `cache.rs:660` (ch04 item 64) |
| 9 | CLAUDE.md | Errata item 3 | "evictors ... are scans" (section 6) | Now list or queue structures | `src/core/eviction.rs:49,257,434-439` |
| 10 | CLAUDE.md | Errata item 4 | "Read-path zero contention" | True only for the epoch read; shard mutex on all paths, twice on a hit | ch04 item 11 (reported); not measured |
| 11 | CLAUDE.md | Errata item 5 | "`stale_cap_violations` must be 0" | Nearly unfalsifiable counter (G4A-18) | `docs/release/g4/cycle1_core_engine_review.md` G4A-18; `cache.rs:824-829` (cited by that review) |
| 12 | CLAUDE.md | Errata item 6 | "Telemetry Hot-Path: AtomicLogLinearHistogram ... <= 0.39%" | Histogram written but its percentiles have no production reader; `/api/v1/metrics` uses a sorted 10,000-sample raw window | verified in code: `src/api/server.rs:583`; `src/intel/telemetry.rs:354,500-516,677-683`; grep: no caller of `calculate_percentiles_histogram_*` in `src/api/` or `src/main.rs` |
| 13 | CLAUDE.md | Errata item 7 | `main.rs:327`; startup message `main.rs:368`; `:370/:374`; `60_000` at 1077/1352 | `:332`; message already updated (stale wording is `config.rs:858`); `:375/:383`; `:1405/:1688` | `src/main.rs:332,364,375,383`; `src/config.rs:858`; `src/api/server.rs:1405,1688` |
| 14 | CLAUDE.md | Errata item 8 | "242 tests, 19 targets" | Older baseline; STATUS.md has 851 passed, 8 ignored, 75 targets (debug, `--tests`) | `docs/STATUS.md` section 3 (reported by prior agent; not re-run here) |
| 15 | CLAUDE.md | Errata item 9 | (linearizability wording elsewhere) | The implemented property is gateway-local read-your-delete; no checker or proof; G4A-01 and G4A-08 open | `src/api/server.rs:1990` (local purge first), `:1992` (router); G4A review |
| 16 | docs/handoff/00_START_HERE.md | New correction block under the existing authoritative notice | "64-Partition Lock Striping"; "Formally Accepted"; "242 / 242"; "T_stale = 0"; "enterprise-grade" | 8 shards; no owner acceptance recorded; later baseline in STATUS.md; per-gateway property only; adjective unsupported | `src/main.rs:204` (`PartitionedCache::new(8)`); `docs/STATUS.md` section 1 and 3; `src/api/server.rs:1990` |
| 17 | docs/handoff/01_PROJECT_MASTER_CONTEXT.md | Notice block added after the existing notice | "64 sharded partitions"; "Guaranteed Local Freshness & Bounded Replica Staleness"; "SIEVE Outperforms classic LRU" | 8 shards; narrower property; no retained comparison, "200x" untraceable (ledger C5) | `src/main.rs:204`; claims ledger C5/C6; README item 3 (SIEVE vs LRU not established) |
| 18 | docs/handoff/02_PHASE_HISTORY_AND_STATUS.md | Notice block added | "64-partition"; "ACCEPTED"; "242 across 19 targets" | 8 shards; self-reported; later baseline | `src/main.rs:204`; STATUS.md |
| 19 | docs/handoff/03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md | Notice block added | "Anti-entropy guarantees self-healing eventual consistency"; "deadlock freedom ... strict global order" | Has a production caller (250 ms) but effectiveness not established; order not machine-checked | `src/main.rs:375,383`; ledger C6; CLAUDE.md errata 1 |
| 20 | docs/research/adr/ADR-001-*.md | Status note appended (body unchanged) | DELETE ordering; "peer invalidations do not reach the serving cache"; "anti-entropy has no production caller"; "max staleness is the 60 s TTL" | DELETE purges local first, then primary; local node shares serving cache in mTLS mode; worker started at 250 ms; 60 s is only a no-header default | `src/api/server.rs:1990,1992`; `src/main.rs:332,364-366,375,383`; `:1405,:1688` |
| 21 | docs/research/adr/ADR-002-*.md | Status note appended | INV-1 "Linearizable Single-Key Deletion ... VERIFIED" | Gateway-local read-your-delete; self-labelled status; open G4A-01/-08; single-flight type note | `test_linearizable_delete_prevents_read` (named in the ADR, not re-run); G4A review |
| 22 | docs/research/adr/ADR-003-*.md | Status note appended | G1 "Per-Node Linearizability"; D_max 250 ms (ADR) vs 50 ms (tests and ledger); stale line numbers in the amendment; cache unification | Renamed in effect; discrepancy recorded; refs `:1990/:1992`, `:375/:383`; local-node-only unification. The G3 24 h bound and the 5 s interval were already corrected by the ADR's own 07:26 amendment, so only a pointer was added | amendment text in the ADR; ledger C6 and section 5; `src/main.rs` and `server.rs` lines above |
| 23 | docs/STATUS.md | Section 3b, new first bullet | HEAD `456bb25...` as current; "Nothing is committed" | HEAD at time of writing was `804a7b8` (docs committed since); counts labelled by their measure; baseline not re-run | `git rev-parse --short HEAD` = 804a7b8 at 17:26; `git log` |
| 24 | docs/PHASE_2_WORK_REPORT.md | Correction block after the existing notice | "Original text below is unchanged" | Not accurate: working tree differs from commit 456bb25 by +555/-223 lines (`git diff --numstat`); body not touched | `git diff --numstat -- docs/PHASE_2_WORK_REPORT.md` |
| 25 | docs/research/claims_ledger.md | Correction notice after the header | C6 "Per-node linearizability guaranteed", "P(stale) = 0.0", "TTL_eff = 60s" | Narrowed to gateway-local read-your-delete; 60 s is not a bound | ledger amendments; G4A review; `config.rs:115` NotWired |
| 26 | docs/research/code_findings.md | Correction notice after the header | F15, F18, "DELETE ordering reversed", 242 baseline | Updated facts with current lines | as 13, 20 |
| 27-34 | docs/research/phase_{2,3,4,6}_{completion_report,progress}.md (8 files) | Notice prepended | "linearizable", "zero stale reads", "bounded staleness", "SUPPORTED", PASS lines | Narrower property; self-reports; pointers to ledger and ADR notes | as 15, 21, 25 |
| 35-37 | docs/security/phase-4-final-acceptance-report.md, stage-4b-audit-report-round2.md, stage-4b-progress.md | Notice prepended | "Final Acceptance ... authorized", "Verified Real-Time Linearizability", "Eventual Consistency with bounded staleness" | Agent self-reports; a loopback debug test, not a proof or bound; no ceiling on freshness lifetime | STATUS.md section 1; limitations.md; ADR-003 amendment |
| 38 | docs/security/phase-4-design.md | Notice prepended | "Implemented, Remediated, and Independently Audited"; stale up to TTL | Design intent; audits by agents; no staleness bound | as 37 |

Counts: 25 existing files changed (README.md, CLAUDE.md, 4 handoff, 3 ADR, STATUS.md, PHASE_2_WORK_REPORT.md, claims ledger, code findings, 8 phase files, 4 security files); 38 numbered corrections (rows 27-34 cover 8 files). Two new files were created: this report and the diff.

## 3. Candidates rejected or reduced

| Candidate | Result | Reason |
|---|---|---|
| README "test counts" | Not changed | README already states that its per-area counts are historical and points to STATUS.md; verified by reading the section. |
| README GET/router mermaid, cluster label, coalescer name | Accepted (rows 1-4) | Still true. |
| README `cargo 1.75+` / Tokio 1.38 | Accepted but worded as unverified for 1.88 | F28 is an AI-review finding, not re-derived; no old toolchain was installed. |
| CLAUDE.md "startup message at main.rs:368 still says peer invalidations do not purge" | Dropped as a claim; recorded in errata item 7 as already fixed in `main.rs` | The `main.rs` message (about line 369) now says the local node shares the cache. The stale wording is only in `src/config.rs:858`. |
| CLAUDE.md "anything claiming linearizability" | Reduced to errata item 9 | CLAUDE.md itself makes no such claim; the claim sits in the code comment and in other documents. |
| CLAUDE.md histogram "no production percentile reader" | Accepted with a narrower wording | The histogram percentile functions exist; only their production callers are absent. The displayed percentiles do come from `calculate_percentiles_us` on the raw window. |
| ADR-003 G3 "24 h" bound and the "5 s" interval | Not rewritten | The ADR's own 07:26 amendment already corrects both; only line-number and D_max notes were added. |
| docs/user/limitations.md and docs/user/*.md | No change | Re-read the relevant lines: limitations.md already says 60 s default is not configurable, no cluster-wide guarantee, and cites current line numbers (`:1990`, `:1405`, `:1688`). `configuration.md` already marks `PHANTOM_DEFAULT_TTL_MS` NOT HONORED. |
| docs/security/phase-3-design.md and other security files | No change | No linearizability or bounded-staleness claim found by grep. |
| docs/research/vision/*, doc_audit/*, learning/*, design_toolkit/* | No change | These already state that nothing cluster-wide is claimed, or are out of scope (`docs/learning/`). |
| docs/research/mvp_and_milestones.md, phase_*_execution_package.md | No change | Plans that describe contracts to be tested, with "per-node" wording; not claims of achieved results. |
| docs/handoff/04-08 | No change | Each already carries the dated notice; 08 contains the correct instruction not to claim linearizability. "enterprise-grade" in 08 line 15 is left (see open items). |
| CLAUDE.md claim "AtomicLogLinearHistogram error <= 0.39%" | Not disputed | The arithmetic is implemented and tested; only the reach of the claim was narrowed. |
| AI claims for adaptive control, performance superiority | Checked by grep; no further document needed a change | README already marks the PIE as a legacy stub and the SIEVE-vs-LRU benefit as unestablished; other hits are hypotheses or pre-registered protocols. The handoff SIEVE statement was handled in row 17. |

## 4. Code-side items deliberately NOT changed (for the later reviewed code change, DEC-022)

1. `src/api/server.rs:1989`: comment "Guarantees linearizability: concurrent GETs on this gateway immediately miss / observe deletion." Should say gateway-local read-your-delete, referencing G4A-01 and G4A-08.
2. `src/config.rs:858`: warning "cluster mode is EXPERIMENTAL: peer invalidations do not purge this gateway's HTTP cache". No longer true in the mTLS path where the local node shares the serving cache (`src/main.rs:332`); should be reworded after checking the insecure-cluster branch.
3. `src/api/server.rs:1990` comment text "(C-08 / Guarantee G1 / ADR-003)": same wording problem as item 1.
4. `src/intel/telemetry.rs` header comment "<= 0.5%" versus the 0.39% in other comments, and the "~67.1 seconds" range comment (ch03 evidence item). Not verified again here.
5. `tests/`: header of `distributed_consistency_tests.rs` claims "P(stale-after-ack) = 0.0 ... Delta <= 50ms" (recorded as unsupported in the ledger), and `invalidation_contract_tests.rs` comment "O(1) Tenant Flush" (the epoch bump copies the tenant map). Test files are code and were not touched.
6. `src/core/cache.rs` SEC-4A-01 comment "cluster-wide" (the global epoch is per process).
7. `src/main.rs:209` `_coalescer` and `src/dist/node.rs` `Coalescer` field: unused legacy type kept in code; a code decision, not a documentation one.

## 5. Preservation statement

No historical record was rewritten. Every historical document received only an added, dated correction block (prepended, or appended after its existing notice) with its original text byte-identical below or above it, as the diff shows. The only deletions in the diff (8 lines) are the README lines corrected in rows 1-6. Frozen and sealed records, `docs/learning/`, `docs/release/commit_lists*/`, the decision register, the tracker and all code are untouched. Nothing was staged or committed.

## 6. Uncertain or open

- The PHASE_2 report correction states the +555/-223 figure from `git diff --numstat` against the committed file; the figure includes the notice lines, and the specific body edits were not reviewed.
- The statement in the Phase 4 security notices that the linearizability check was a loopback, debug-profile test is taken from the report text and was not re-run.
- G4A-18, G4A-20, G4A-01, G4A-02, G4A-08 and F28 are cited from the G4 review and handbook evidence files (reported by prior agents); G4A-20 and the `visited` field were additionally checked by grep, and the eviction structures were read. The others were not independently re-derived.
- The test baselines (851/8/75, 298) were not re-run; they are quoted from `docs/STATUS.md`.
- `docs/handoff/08_NEW_AI_ONBOARDING_PROMPT.md` still calls the project "enterprise-grade" and says P1-P4 are "completed and accepted"; its existing notice covers acceptance, but the adjective was not edited.
- Line endings were checked only for README.md and the claims ledger (the only edited files containing CR characters).
- A reviewer should confirm that "gateway-local read-your-delete" is the wording the owner wants for the property; it is the narrowest statement the retained tests support.
