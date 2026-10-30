# D3 documentation audit: non-research documentation

Date: 2026-10-30. Agent D3 of 4. Read-only audit; this is the only file written. Repo state when read: HEAD 456bb25, branch feat/publish-phantomcache-core, 72 status lines. Out of scope (D1/D2): docs/research/*, milestone records.
Evidence tags: V = verified in code/Cargo.toml by reading or grep (file:line); I = inferred (nothing was run: no cargo, no browser); R = reported by the doc only.

## 1. Inventory

Dates are file mtimes (weak evidence; most files are uncommitted). Freshness: OK / PARTIAL / STALE / HISTORICAL (frozen record, never an error).

| Path | Purpose | Audience | Date | Freshness |
|---|---|---|---|---|
| README.md (285 l) | Project intro, real-vs-simulated table, build, API, tests | newcomers | after 10-29 | PARTIAL: F1-F5 |
| CLAUDE.md (59 l) | Agent rules, invariants, config ref, known gaps | AI agents / owner | 10-29/30 | PARTIAL: section 2.F (owner-controlled) |
| CORRECTIONS.md (39 l, root) | M1-M3 corrections ledger (CORR-Mx-nn) | maintainers | undated | PARTIAL: stale line ref; same filename as docs/handoff/CORRECTIONS.md |
| MILESTONE_1_DESIGN_PLAN.md (764 l) | M1 design plan | historical | undated | HISTORICAL; describes the pre-wiring state ("_cache discarded", main.rs:57-58 in its text); no banner |
| walkthrough.md (~15 l) | Notes on V3 flagship demo hardening ("sorted at each 100 ms interval") | unclear | undated | STALE/orphan: root CORRECTIONS CORR-M2-03 says sort-on-hit-path was replaced; no date or status |
| docs/PHASE_2_WORK_REPORT.md (661 l) | Course submission report (placeholders for names/USN unfilled) | academic reviewers | 2026-10 | HISTORICAL; linked from README s9 |
| docs/implementation/ (9: WP1-WP5, WP3/4/5_GUIDE, FLAGSHIP_DEMO_GUIDE) | First-phase work-package specs | historical | 10-11 | HISTORICAL; no banner; "WP1-5" collides with docs/research/work_packages.md "WP-nn" |
| docs/testing/ (5: ENVIRONMENT_READINESS, FLAGSHIP_VALIDATION, V2/V3/V4 audits) | Audits of the simulation-era flagship demo | historical | 10-11 | HISTORICAL; no banner |
| docs/demo/README.md + Start-Phantom-Demo.bat | Flagship demo launch guide | presenter | 10-11 | PARTIAL: ports 8085/3000 match bat:26-37 and ui/vite.config.js:7 (V); Showcase/Explorer text describes simulated behaviour without a SIMULATED label |
| docs/demo/lab_runner.md | phantom-lab T1 runner (SIMULATED) | researchers | 10-30 | OK (src/bin/phantom-lab.rs exists, V) |
| docs/demo/scenario_B_origin_slowdown.md | Scenario B acceptance contract | researchers | 10-30 | STALE status: "CONTRACT ONLY ... Nothing below is implemented" while src/lab_live, runbook, artifacts doc, lab_api.rs and lab-results/scenario_b exist |
| docs/demo/scenario_b_artifacts.md, scenario_b_runbook.md | Artifact schema, how to run | researchers | 10-30 | OK (headers only checked) |
| docs/demo/scenario_b_interpretation_notice.md | Dated binding correction for retained experiment | anyone citing results | 10-30 07:14 | OK; the model for dated amendments |
| docs/demo/browser_verification/ (3 entries), scenario_b_verification/ (5 entries) | Evidence (screenshots, recomputation); listed, not read | reviewers | 10-30 | EVIDENCE |
| docs/demo-lab/ (9) | Multi-device LAN lab and cyber trust research suite | planners | 10-26 | STALE in parts (F11); no banner |
| docs/security/ (17) | Phase 1-4 specs, red team, stage progress and audits, final acceptance, M7 review, demo matrix | security reviewers | 10-27..30 | MIXED: section 2.C |
| docs/design/ (8) | M8 UI IA, tokens, principles, Lab API contract, UI audit, SSE auth options | UI devs | 10-29/30 | mostly OK; lab_api_contract.md status "SPECIFICATION" though implemented |
| docs/user/ (9) | Install, configure, run, operate, troubleshoot, security, limitations, lab_api | operators | 10-29/30 | best-maintained set; stale line refs and one stale error name (F12, F15) |
| docs/release/ (5) | Artifact manifest, CI recommendations, deferred integration, reproduction guide, release-notes template | release owner | 10-29/30 | OK; honest about unknowns |
| docs/handoff/ (00-08, MANIFEST, CORRECTIONS, 2 jsonl transcripts of 1519 and 6493 lines) | AI-to-AI handoff package | AI agents | 10-28/29 | HISTORICAL with partial banners (00, 05, CORRECTIONS) F13 |
| docs/milestone_1/WORK_NOTES.md | M1 verification notes (47 tests) | historical | 10-26 | HISTORICAL, no banner |
| packaging/ (7) | build-release.ps1/.sh, smoke-test, verify-checksums, 2 example confs | operators | 10-29 | OK; example conf keys match config.rs (spot check) |
| scripts/ (6) | cleanroom_build, gen-demo-certs, reproduce_tests, run_audit_suite | devs | 10-27/29 | OK; script headers exist; no scripts/README |
| benches/ (b1,b2,b3,b4,b6,b7,b8e + support/) | Benchmarks | researchers | 10-29 | no README or header index; b5 absent; spec is in docs/research (D2) |
| ui/ | No README or .md files (package.json, vite.config.js, scripts/, test/) | UI devs | n/a | MISSING |
| bench-results/ (tooling_report.md), lab-results/ | Retained artifacts | reviewers | 10-29/30 | EVIDENCE; no index doc |
| .github/workflows/ci.yml | CI | maintainers | n/a | per docs/release/ci_recommendations.md: no `-j 1`, benches run as tests, fmt status unknown (R) |

## 2. Freshness findings (evidence on both sides)

### A. README.md
- F1 Test counts contradict inside README: s8 code comment "220 automated tests across 18 test targets" vs the next paragraph "242 tests in 19 targets"; Cargo.toml now declares 72 `[[test]]` stanzas (V, count by grep). No doc I read gives a current verified count.
- F2 README s6.1 says `cargo run` starts the backend. src/bin/phantom-lab.rs exists (V) and Cargo.toml has no `default-run` and no `[[bin]]` (V, grep), so `cargo run` is likely ambiguous (I, not run). Other docs use `--bin phantom-cache-demo` (docs/demo/scenario_b_runbook.md:9, docs/release/reproduction_guide.md:21).
- F3 README s7 API reference lists `/health`, `/health/live|ready`, `/api/control`, `/api/events` only. Actual routes (V): `/api/v1/cache/*key` and `/api/v1/cache` GET/DELETE, `/api/v1/metrics`, `/api/v1/diagnostics/{summary,events}`, `/api/v1/intelligence/{status,audit}` (src/api/server.rs:199-214), 9 `/api/v1/lab/*` routes (lab_api.rs:1025-1033). The product's core endpoint is missing from README (it is in docs/user/run.md). README SSE sample shows `"version": "1.0"`; the server builds `"schema": 2` payloads (server.rs:498, 724); whether the legacy frame still has "version" was not checked.
- F4 README s8 describes tests as "PIE adaptive canary guardrails" and "SIEVE vs LRU vs S3-FIFO comparative benchmarks", wording at odds with README s1/s2 and CLAUDE.md 6b (PIE is a stub). README s6.2 says UI on "5173 / 3000"; vite.config.js:7 pins 3000 (V). README prerequisites "cargo 1.75+" vs docs/user/install.md "only 1.98.0 has been used".
- F5 README links docs/user, docs/release, docs/PHASE_2_WORK_REPORT, docs/implementation, docs/testing. It does not link docs/security, docs/handoff, docs/research, docs/design, docs/demo, docs/demo-lab.
- Positive: README s1/s2 evidence labels and the s6.5 configuration pointer are accurate and honest.

### B. docs/user/ vs code
- Config keys: every `PHANTOM_*` key in src/config.rs (lines 96-185: MODE, BIND_ADDR, TLS_*, ALLOW_INSECURE_*, REQUIRE_AUTH, JWT_*, MAX_CACHE_BYTES, DEFAULT_TTL_MS, MAX_STALE_MS, SWEEPER_*, ORIGIN_ALLOWLIST, ENFORCE_UPSTREAM_TLS, ALLOW_LAN_ORIGIN, FORWARD_UPSTREAM_AUTH, UPSTREAM_CA_CERT, CLUSTER_*, NODE_*, STRICT_CLUSTER, INTELLIGENCE, ORIGIN_CAP_*, SIMULATION, CORS_ORIGINS, LAB_*, CONFIG) is in docs/user/configuration.md, and the reverse (V, compared by reading the grep output). No missing or invented key found. Defaults 127.0.0.1:8085 and sweeper keys agree with CLAUDE.md.
- Lab routes: docs/user/lab_api.md table matches lab_api.rs:1025-1033 (V).
- F12 Line refs: configuration.md (PHANTOM_DEFAULT_TTL_MS row) cites server.rs `:1367`, `:1650`; the `60_000` literals are at server.rs:1405 and :1688 (V). docs/user/limitations.md cites `src/main.rs:327`; `VirtualNode::new_with_cache(node_id, cache.clone())` is at main.rs:332 (V). Prefer symbol names over line numbers.
- F15 docs/user/troubleshoot.md says "403 `UpstreamDestinationDisallowed`". The gateway maps SSRF/policy rejections to 502 `SsrfBlocked`, `InvalidOrigin` (400) and `UpstreamTlsFailed` (502) (V, server.rs:2262-2300 area; the string `UpstreamDestinationDisallowed` occurs nowhere in src/, grep returns nothing). Same wrong name appears in docs/security/security-demo-matrix.md. This is the one factual error found in a living user doc.
- docs/user/run.md endpoint table omits lab routes (covered by lab_api.md; add pointer). troubleshoot.md is 21 lines, startup-centric.

### C. docs/security/
- F14 security-demo-matrix.md (145 l, 8 tests): every "Automated Regression Test" path (`tests/security/test_sec_01_...rs` to 08) is absent: no tests/security/ and no tests/common/ directory exists (V, ls). `phantom-verify`, `release-manifest.json`, `release.pub` appear nowhere in src/ or Cargo.toml (V, grep). `OriginTlsHandshakeFailed`, `CERT_UNTRUSTED` and `UpstreamDestinationDisallowed` do not occur in src/ (V, grep); real codes are `SsrfBlocked`, `UpstreamTlsFailed`, `InvalidOrigin`. "Observed Evidence" blocks (digests, log lines) are illustrative; no run produced them. Treat the file as design intent; it needs a dated "not evidence" notice. Real security test evidence: Cargo.toml targets `security_phase1..4c_*`, `m7_sec_*`, and docs/security/m7_security_review.md.
- phase-4-final-acceptance-report.md has a dated amendment (~line 160, "Historical record: the text above is unchanged") qualifying the "ACCEPTED" verdict at line 154: the right pattern. phase-1-specification, phase-3-design, phase-4-design, stage-4b/4c-progress also matched a keyword grep for historical/superseded wording. The other security files (authenticity-design-options, implementation-roadmap, phase-2-*, stage-3c-progress, stage-4b-audit-report-round2, stage-4c-audit-report, trust-boundary-map, trust-model-audit) did not match: no visible freshness banner (keyword grep only, not a full read).
- "Phase 5" has three meanings: docs/security/implementation-roadmap.md (5 security phases), docs/demo-lab/implementation-roadmap.md:14 (Phase 5 Controlled Security Scenarios), docs/handoff/05 (Proposed Phase 5-9, itself marked superseded).
- m7_security_review.md and docs/user/security.md are current and list what was not done (pentest, fuzzing, advisory scan, multi-host). Strong.

### D. docs/demo-lab/
- F11 device-topologies.md line 26 "blocked by hardcoded loopback binding (127.0.0.1:8085)" and line 94 "Requires Phase 1 (server LAN binding)": stale, PHANTOM_BIND_ADDR is configurable and non-loopback is allowed with JWT and TLS (config.rs:98; docs/user/configuration.md cross-field rules; V). Line 65 "hardcoded localhost:8085 in ui/src/App.jsx": stale, UI has `DEFAULT_GATEWAY` plus `VITE_PHANTOM_GATEWAY` override (ui/src/lib/config.js:1, App.jsx:57, SimDemo.jsx:20-21; V). Line 165 "disconnected from the HTTP gateway ... scheduled for Phase 6": stale, mTLS cluster mode builds the router with the shared serving cache and starts the anti-entropy worker (main.rs:332, 359-374; V), still experimental. No banner; mtime 10-26 predates M3-M8.
- demo-lab/README.md cites "73 passing tests"; superseded, unlabeled.

### E. docs/handoff/ and root ledgers
- F13 00_START_HERE.md has a dated notice pointing to CORRECTIONS.md, but body still says "Phase 4 Remediated, Verified, and Formally Accepted", "adaptive Policy Inference Engine (PIE)" (CLAUDE.md expands PIE as Policy Intelligence Engine), "Do NOT Begin Phase 5". MANIFEST.md says "Zero known gaps". 05 has a supersession note. 01-04, 06-08 have no banner (grep).
- Two different files named CORRECTIONS.md: docs/handoff/CORRECTIONS.md (claims audit, 2026-10-29) and root CORRECTIONS.md (M1-M3 fix ledger). The root one is not linked from README; its CORR-M3-01 cites `main.rs:248`, now :332 (V).

### F. CLAUDE.md (owner-controlled: suggestions only, nothing edited)
- S1 s5 TTL bullet: "~lines 1077 and 1352" -> server.rs:1405 and :1688 (V); better to say "grep `60_000` in src/api/server.rs".
- S2 s5 says Faithful eviction policies exist; s6 says "evictors named SIEVE/S3-FIFO/LRU are scans". The serving shard holds `eviction: Box<dyn ShardEvictionPolicy>` (cache.rs:209, V). Remove or date-qualify the "scans" clause (internal contradiction).
- S3 s6 insecure-cluster remark: the branch is now checked: main.rs:380-381 prints that the router holds placeholder nodes 1-3 without listeners or peer addresses, no invalidation frame is sent or received, DELETE purges only the local HTTP cache and reports PrimaryInvalidationTimeout (server.rs:2066 uses that name) (V). The "startup message at main.rs:368" remark is outdated: the mTLS-path message at main.rs:370 now says the node shares the serving cache.
- S4 s3 baseline "242 tests, 19 targets": Cargo.toml declares 72 test targets now (V); mark as superseded once a new count is reproduced.
- S5 s6 "dashboard mixes live and synthetic values" and "only staleness bound is a fixed 60 s TTL": reconcile with schema-v2 provenance labels (docs/user/run.md, docs/design/ui_audit.md) and with PHANTOM_MAX_STALE_MS (hard cap, s5). Re-verify and date.
- S6 s5 config reference duplicates 4 keys; point to docs/user/configuration.md as canonical.
- S7 "Authoritative sources" omits docs/user, docs/design, docs/release.

## 3. Contradictions between documents
1. Test baselines: 47 (milestone_1/WORK_NOTES), 73 (demo-lab/README), 220/18 and 242/19 (README), 234 (handoff/02, R), 72 declared targets (Cargo.toml). Different dates, rarely labeled.
2. "Phase 4 accepted / zero P0": acceptance report, handoff/00, MANIFEST vs CLAUDE.md s6, handoff/CORRECTIONS, handoff/05 banner.
3. PIE meaning: Policy Intelligence Engine (CLAUDE.md, stub) vs Policy Inference Engine (handoff/00, adaptive) vs README "Phantom Intelligence ... legacy stub not active" vs configuration.md modes off/observe/advisory/canary/active.
4. Cluster cache sharing: device-topologies "disconnected"; handoff/CORRECTIONS "separate VirtualNode.cache"; root CORRECTIONS CORR-M3-01 "unified"; limitations.md "shared (main.rs:327)". Truth (V): shared in mTLS path (main.rs:332), none in insecure path (main.rs:381).
5. Scenario B: contract "nothing implemented" vs runbook, artifacts, Lab API and retained results; interpretation notice is binding for those results but not linked from the contract.
6. docs/design/lab_api_contract.md "SPECIFICATION" vs docs/user/lab_api.md "implemented in G2".
7. Rust version: README "1.75+" vs install.md.
8. Eviction: CLAUDE.md s5 vs s6 (S2).
9. Error names: troubleshoot.md/security-demo-matrix.md vs server.rs (F15).
10. UI port: README "5173 / 3000" vs vite config 3000.
11. Two CORRECTIONS.md files.

## 4. Terminology conflicts (glossary input)

| Term | Conflicting uses |
|---|---|
| Phase | P1-P4 work packages (handoff); security Phases 1-4 and Stages 3A-4C; demo-lab Phase 1-6; handoff "Phase 5-9 proposed"; docs/research phase_1..8 execution packages (D2); "Phase 2 work report" (academic CCA Phase-2) |
| Milestone | M1-M8 (README, CORRECTIONS, release docs "M7", design docs "M8-04"); "seven milestones toward MVP" (handoff/05); "M5 = Phantom Intelligence" (CLAUDE.md 6b) |
| WP | WP1-WP5 (docs/implementation, wp1_tests..wp4_tests) vs WP-00/WP-01 (docs/research/work_packages.md) |
| Identifiers | G1-G3, D-3/D-4, O4-1/O7-6/O8-2 (owner decisions), BD-05, R-UI-nn, R-01, CORR-Mx-nn, KG-1..6, C-01..C-08, SEC-4A-01, INV-4, I1-I8, Gate B |
| Tier | SIMULATED (T1) / MEASURED_LOOPBACK (T2) / RETAINED_* / UNAVAILABLE (lab); README Real/Loopback/Simulated; UI measured/simulated/inferred/unavailable; "Architectural Tier", "Client Tier" in README |
| Mode | PHANTOM_MODE development/production; PHANTOM_INTELLIGENCE off/observe/advisory/canary/active; demo "Showcase/Explorer"; "Mode B Reverse Proxy" (security phase-1); "insecure/partial cluster" vs "mTLS cluster"; "MVP-A/B" (release template) |
| Status words | Accepted, Remediated/Complete/Verified, CONTRACT ONLY, SPECIFICATION, implemented, AUTHORITATIVE, EXPLORATORY/VALID, HISTORICAL, EXPERIMENTAL; evidence tags V/R/D/U (ui_audit) vs verified/reproduced/reported/assumption/hypothesis/literature (CLAUDE.md s4) |
| Node | simulated VirtualNode, real cluster member, gateway process, "Device" (LAN lab) |
| Cache | PartitionedCache, serving cache, HTTP cache, side cache, VirtualNode.cache |
| Freshness | default TTL (not honored), max-age/s-maxage, stale cap, SWR/SIE (not defined in user docs) |

## 5. Newcomer journey: the ten questions

| # | Question | Answered by | Gap |
|---|---|---|---|
| 1 | What is PhantomCache | README s1-s3; docs/user/README.md; handoff/00 s2 | descriptions differ in scope (demo vs gateway) |
| 2 | Why it exists | README s1 | no research question/hypotheses in README; they live in docs/research (D2), unlinked |
| 3 | Build and run | docs/user/install.md, run.md; README s6; packaging/ | `cargo run` ambiguity (F2); no 5-minute quickstart with a stub origin; Windows `-j 1`/TEMP only in troubleshoot.md and CLAUDE.md |
| 4 | How components interact | README s3 diagram; handoff/03; security/trust-boundary-map.md | no current architecture overview; README diagram partly simulation-era |
| 5 | Reproduce experiments | docs/release/reproduction_guide.md; docs/demo/lab_runner.md; scenario_b_runbook.md; scripts/reproduce_tests.* | scattered, no index; sealed evaluation procedure only in research (D2) |
| 6 | How intelligence is evaluated | docs/user/limitations.md (not established); research docs (D2) | MISSING outside research; no user-facing explanation of modes |
| 7 | How security and distributed behaviour are validated | m7_security_review.md; security_phase*/m7_sec_* targets; handoff/04; research (D2) | security-demo-matrix.md misleads (F14); no claim-to-test map; multi-host validation absent by admission |
| 8 | Implemented vs remaining | limitations.md; handoff/CORRECTIONS.md; research claims ledger (D2); CLAUDE.md s6 | fragmented in 4 places; README has no status section |
| 9 | Where is the evidence | bench-results/, lab-results/, docs/demo/*_verification/, release docs | MISSING index by claim and tier; no READMEs in those directories |
| 10 | How to contribute | CLAUDE.md s1-s3 (agent-oriented), handoff/06 | MISSING human CONTRIBUTING / dev guide |

## 6. Missing document types (all PROPOSALS; none created)
- PROPOSAL CONTRIBUTING.md or docs/dev/development.md: toolchain, `-j 1` and TEMP on Windows, running subsets of the ~72 test targets, fmt/clippy gates vs CI, adding a config key (config.rs + configuration.md + example conf), review requirement for fencing/epoch changes, branch and commit policy.
- PROPOSAL operations/troubleshooting expansion (operate.md, troubleshoot.md): metrics glossary, runbooks per error code (503 CapacityExceeded, 502 SsrfBlocked, stale-cap violations, peer Suspect/quarantine), sizing, retained-artifact housekeeping.
- PROPOSAL ADR/decision log index: ADR-001..003 exist under docs/research/adr (D2) but are not discoverable from README; owner decisions (O4-1, O7-6, O8-2) are scattered. Outline: ID, date, status, context, decision, evidence, supersedes.
- PROPOSAL docs/user/api.md: all routes with role required, headers (X-Cache-Status, X-Cache-Error, X-Cache-Warning, x-cache-bypass, x-tenant-id), error catalogue from server.rs, metrics schema-2 provenance labels.
- PROPOSAL docs/GLOSSARY.md from section 4.
- PROPOSAL docs/testing/test_strategy.md (current): target groups (wp*, m1_http, security_phase*, m7_*, lab_*, intel_*, proptest), what tests do and do not establish (structure of handoff/CORRECTIONS s2), determinism and seeds, benches-run-as-tests caveat, CI mapping. Existing docs/testing/ is historical.
- PROPOSAL docs/INDEX.md: doc sets with status (current/historical/evidence) and reading order, linked from README. Plus READMEs for ui/, benches/ (what b1..b8e measure, b5 gap, release-only rule), scripts/, bench-results/, lab-results/.
- PROPOSAL current architecture overview labelling implemented / experimental / simulated.

## 7. Recommended actions

### Safe, docs-only (new files or dated amendments; no source edits)
1. Dated "design intent, not evidence; error names and test paths do not exist" banner on docs/security/security-demo-matrix.md.
2. Dated amendment banners on docs/demo-lab/{device-topologies,capability-audit,implementation-roadmap,README}.md.
3. Status amendment on docs/demo/scenario_B_origin_slowdown.md (implemented; link runbook and interpretation notice).
4. "Historical, pre-M3" banners on walkthrough.md, MILESTONE_1_DESIGN_PLAN.md, docs/milestone_1/WORK_NOTES.md, docs/implementation/*, docs/testing/*.
5. Correct living user docs: troubleshoot.md error names (F15), configuration.md and limitations.md line refs (F12) (replace with symbol names). These are living docs, not frozen records.
6. Update docs/design/lab_api_contract.md status line to point at the implementation.
7. Dated amendment in handoff/MANIFEST.md and banners on handoff 01-04, 06-08 pointing to handoff/CORRECTIONS.md.
8. Create new docs from section 6.

### Owner decision
- CLAUDE.md edits S1-S7.
- README as newcomer front door: fix `cargo run` (document `--bin phantom-cache-demo`, or add `default-run` which is a Cargo.toml change), refresh API section, test counts, links to security/handoff/research/design; whether README stays the course-project page.
- Same-name CORRECTIONS.md files: cross-link only, or rename (needs approval).
- PHASE_2_WORK_REPORT placeholders.
- CI changes (`-j 1`, bench mains, fmt status) per docs/release/ci_recommendations.md.
- Which test count becomes the canonical baseline (needs a reproduced run; not done here).

### Must-not-touch
- Frozen and retained records: docs/handoff/* bodies and transcripts, transcript_copy.jsonl, security phase/stage/audit/acceptance report bodies, docs/research patches, preregistration and freeze files (D2), docs/demo/scenario_b_verification/, browser_verification/, lab-results/, bench-results/. Use dated amendments only.
- CLAUDE.md (owner), src/, Cargo.toml, ci.yml, packaging/ scripts.

## 8. Top 10 issues
1. security-demo-matrix.md: 8 non-existent test files, non-existent `phantom-verify`, non-existent error names, unobserved "Observed Evidence" (F14).
2. troubleshoot.md (living) names `403 UpstreamDestinationDisallowed`; code returns 502 `SsrfBlocked` / `UpstreamTlsFailed` / 400 `InvalidOrigin` (F15).
3. README `cargo run` likely broken since the second binary was added (F2, inferred).
4. README test counts self-contradict (220/18 vs 242/19); 72 test targets declared; no current verified baseline anywhere (F1).
5. README API section omits cache, metrics, diagnostics, intelligence and lab routes (F3).
6. CLAUDE.md s5/s6: stale line numbers, internal eviction contradiction (cache.rs:209), insecure-cluster remark now answerable (main.rs:380-381) (owner).
7. docs/demo-lab/device-topologies.md and siblings describe a pre-M3 world with no banner (F11).
8. Scenario B contract says "nothing implemented" though implemented and retained results exist.
9. No glossary, ADR index, contribution guide, API reference, or current test strategy; phase/milestone/WP/tier/mode/PIE used inconsistently.
10. Handoff body (00, MANIFEST) still says "formally accepted", "zero known gaps"; evidence directories have no index; two CORRECTIONS.md files; line-number citations in living docs rot.

## 9. Open questions
- Current test count and target count at HEAD (needs `cargo test --all-targets -j 1`; not run).
- Does `cargo run` fail without `default-run`? (inferred only)
- Does legacy `/api/events` still emit `"version": "1.0"` as README says?
- Which un-bannered security and demo-lab documents are intended as living vs frozen? (keyword grep, not full read)
- Should README remain the course-project front door (s9 "Academic Submission") or be split from an engineering front door?
- Not read in depth: handoff 01-04 and 06-08 bodies, PHASE_2_WORK_REPORT body, MILESTONE_1_DESIGN_PLAN body, security phase-3/4 designs, docs/implementation/*, docs/testing/*, scenario_b docs beyond headers, scripts/packaging bodies beyond headers.
