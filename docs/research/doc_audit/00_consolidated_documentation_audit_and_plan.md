# PhantomCache documentation audit: consolidated findings and proposed plan

**Date:** 2026-10-30. **Status:** Proposed (nothing in this plan has been executed). **Audience:** the product owner, who decides the items marked OWNER.
**Sources:** four read-only audits in this folder: `d1_milestone_documentation_audit.md`, `d2_research_docs_audit.md`, `d3_other_docs_audit.md`, `d4_mechanical_checks.md`. Their limits apply: D1 to D3 read mostly headers, banners and greps, not full texts; D4's counts come from script runs (162 markdown files, 2.34 MB) with heuristic path detection (plus or minus a few percent). Items below tagged **[L]** were re-checked by me; the rest are as reported by those audits.
**Rule observed:** no document was edited, moved, renamed or deleted. Historical, frozen, sealed and append-only records are never "fixed"; discrepancies get dated amendments or supersession notices.

## 1. Inventory in one table

| Area | Files | State |
|---|---|---|
| `docs/research/` | 127 files, 78 md; flat, no index; subfolders `adr/`, `m3_durability/`, `ttl_decision/`, `vision/`, `doc_audit/` | Rich, mixed quality; the record-keeping pattern is good, status and authority are the weak points |
| `docs/security/` | 17 md | Mostly frozen phase records; `security-demo-matrix.md` describes things that do not exist |
| `docs/demo/`, `docs/demo-lab/` | 11 md + 914 evidence files; 9 md | `demo-lab` partly stale; evidence directories have no READMEs |
| `docs/user/` | 9 md | **Best maintained** (config keys and API routes match the code in both directions) |
| `docs/design/`, `implementation/`, `testing/`, `release/`, `handoff/`, `milestone_1/` | 8, 9, 5, 5, 11, 1 md | `implementation/` and `testing/` have no H1 and are unlinked; `handoff/` mixes banner-less claims ("formally accepted", "zero known gaps") with a banner on some files |
| Root | README, CLAUDE.md, CORRECTIONS, MILESTONE_1_DESIGN_PLAN, walkthrough, `docs/PHASE_2_WORK_REPORT.md` | README carries two test counts (220 and 242); CLAUDE.md is owner-controlled |

## 2. Quality and consistency assessment

**What works (keep and copy):**
- The preregistration chain (v2, v2.1, v2.2, addendum 1, freeze JSONs, pin): dated, hash-pinned, non-rewriting.
- The pattern "dated amendment, body unchanged" in `phase-4-final-acceptance-report.md`, `scenario_b_interpretation_notice.md`, the ADR-003 amendment, the M6 retraction banners and the novelty-snapshot correction notice.
- `g1_verdict.md` as a verdict template (requirement, result, evidence, reviewer, dispositions).
- `docs/user/*` and the Lab API route table.

**What does not work:**
1. **No status authority.** Six documents state milestone status and disagree (master plan section 2, ledger sections A to E, accountability report, five phase completion reports, handoff). The chain "ledger defers to master plan defers to accountability report" is itself stale: the accountability report still says G1 in progress, G2 not started and the UI never browser-verified; the gate verdicts and three retained browser rounds supersede that.
2. **Milestone M8 is not in the roadmap file** (`mvp_and_milestones.md` defines M1 to M7; the Rev 3 edit was never applied). M7 and M8 have no progress, completion or status record.
3. **No owner authorization record** for any phase or gate, although every package is headed "PROPOSED, not authorized" and phases 1 to 6 were executed. ADR-003 is "ACCEPTED" with two agents as decision makers. `G-RESEARCH` is unrecorded although M5b and M6 ran.
4. **Completion reports contradict the evidence:**
   - M1 to M5 say PASS; the accountability report says none passed independently.
   - M5 AC5-13 says the intelligence engine is constructed in `main()`; `src/main.rs:433` has `pie: None` **[L]**.
   - M4's report rewords AC4-9/AC4-10 and cites "r_s = 1.0".
   - M2 cites the 200x speedup (unverified, C5); M3 says "zero compromises" next to a reproduced durability defect.
   - The M6 completion report body still says "MVP-A, production ready" under a retraction banner.
5. **Stale living status:** `progress_ledger.md` sections A to C are about 11 hours behind its own log (M6 still "OPEN/HOLD"); `claims_ledger.md` table lists C1/C2/C7 as "HYPOTHESIS (Pending R-03)" while its own section 6 says C1 NOT SUPPORTED and C2 NOT TESTABLE.
6. **Test-count and fact drift:** 242 appears in 27 files including CLAUDE.md and README; later counts 303, 388, 435, 499, 623, 774, 802, 820, 841, 851 have no common table. Five SEC ids (01, 02, 03, 09, 11) carry mixed statuses across 4 to 7 files. The staleness bound appears as 50 ms (a sleep in a test, claim C6 unsupported), 250 ms (ADR-003) and 60 s (the only bound after an issuer crash).
7. **Terminology collisions:** eight numbering schemes and ID namespaces (phase, M#, WP-NN vs WP1..5, security phases, handoff stages, `R-01` meaning three things); two CORRECTIONS.md files (39 vs 82 lines); two tier systems (T1/T2/T3 vs MEASURED_LOOPBACK/SIMULATED/INFERRED/UNAVAILABLE); "node" vs "peer"; PIE expanded two ways.
8. **Stale or non-existent references:** `security-demo-matrix.md` points at eight test paths and a `phantom-verify` tool that do not exist and uses error names absent from `src/`; `troubleshoot.md` names `UpstreamDestinationDisallowed` where the code returns `SsrfBlocked`, `UpstreamTlsFailed` or `InvalidOrigin`; `docs/demo-lab/*` still says LAN binding is blocked and the router is disconnected; the Scenario B contract says "nothing implemented"; `preregistration_m6_v2_freeze_amendment_1.json` is referenced but does not exist (real files: `preregistration_m6_v2_2_addendum_1.*`).
9. **Mechanical (D4):** 9 broken relative links (8 are wrong-depth `../src/...`), 1 missing anchor, 67 backticked repo paths that do not exist (many planned-but-never-built), 209 `file:///d:/...` links that only work on this machine, 417 absolute machine paths in 60 files, 63 files with no date, only 53 of 162 with a `Status:` header (about 45 free-text values), 17 files without an H1, 123 of 162 files not linked from any index, 4 files over 60 KB.
10. **Missing document types:** glossary, ADR index, contribution/development guide, API reference, current test strategy and baseline, benchmark methodology and results report, operations/troubleshooting runbook index, as-built architecture (the "authoritative" intelligence architecture describes machinery outside the serving path), a research-folder README.

**One safety observation (needs a human glance).** D4 found 15 lines in 7 files that put a drive path next to "salt", "key" or "secret". I separately scanned the docs for 64-hex strings near those words: 8 matches in the preregistration documents and freeze JSONs, all on lines that also mention commitment, hash or digest **[L]**. I did not read or print the values and cannot tell a public commitment from a value that should not be there. Because earlier amendments record that one salt was exposed and retired, someone other than me should confirm that no live salt value appears in any document before any commit (this is already a G5 step).

## 3. Authoritative-source map (proposed)

The principle: each fact lives in exactly one place; everything else links to it or states a date.

| Information | Authoritative document | Others |
|---|---|---|
| Milestone and gate status | **New `docs/STATUS.md`** (one page, updated at each gate; a dated table) | Ledger, master plan and accountability report link to it; their historical sections are not edited |
| Event history (what happened, hashes, commands) | `progress_ledger.md` (append-only) | n/a |
| Research claims and their status | `claims_ledger.md` (one status per claim id; supersession by dated row) | |
| Experiment protocols | The preregistration chain (frozen) | Runbooks link to the exact version |
| Security finding status | `docs/security/` register (SEC ids) | One status per id; others cite the id and date |
| Configuration reference | `docs/user/configuration.md`, generated or checked against `src/config.rs` | |
| Test baseline (counts and profile) | **New table in `docs/STATUS.md`** (count, profile, date, command, log) | CLAUDE.md and README link to it instead of quoting a number |
| Decisions | `docs/research/adr/` plus **new ADR index**; owner ratification recorded in the ADR | |
| Gate verdicts | `g1_verdict.md`, `g2_verdict.md`, new `g3_verdict.md` | |
| Terminology | **New `docs/glossary.md`** (numbering schemes mapped, tiers, modes, PIE) | |

## 4. Proposed structure (minimal; no index for the sake of an index)

Keep paths stable. Do not move existing files; add only these:

- `docs/README.md`: one-page map answering "what is it, how to build and run, where is X". Links only.
- `docs/STATUS.md`: authoritative status (section 3).
- `docs/glossary.md`.
- `docs/research/README.md`: grouped index of the 78 files with status class (Active, Verified, Superseded, Historical-frozen) and authority, no content copying.
- `docs/research/adr/README.md`: ADR index (ADR-001 marked Superseded by ADR-003 only if you confirm).
- `docs/contributing.md` and `docs/testing/strategy.md` (new).
- Per evidence directory a short `README.md` (`docs/demo/browser_verification/`, `scenario_b_verification/`, `lab-results/`, `bench-results/`): what it is, tier, how produced, do not edit.
- `docs/api/` is not needed; link `docs/user/lab_api.md` as the API reference and add the missing routes there.

Not proposed: reorganising `docs/research/` into subfolders (it would break the many existing references; revisit after the index exists).

## 5. Standard and templates

**Header block** (front matter or the first lines, same in every non-evidence document):
`Status:` one of **Draft, Proposed, Active, Verified, Superseded, Archived** (exact case) · `Audience:` · `Last reviewed:` ISO date · `Supersedes/Superseded by:` links · `Evidence:` links.
"Verified" requires a named independent check. Historical records keep their original wording and receive a banner (`Historical: see ...`) only by appending, never by rewriting the body.

**Templates (short, only the sections that apply):**
1. *Milestone record*: objective, scope and non-goals, dependencies, decisions (links to ADRs), stages, acceptance criteria (verbatim from the package), evidence table with an **independence** column, limitations and risks, status, gate sign-off line (owner/date).
2. *Gate verdict*: copy `g1_verdict.md`.
3. *ADR*: context, decision, alternatives, consequences, ratified by (owner/date).
4. *Experiment record*: tier, protocol version and hash, seeds policy, command, environment, raw artifacts, result, limits.
5. *Runbook*: purpose, prerequisites, steps, expected output, failure modes, cleanup.
6. *Security finding*: id, severity, status, evidence, fix, retest.
7. *Research note*: question, method, evidence tags, conclusion, open questions.

**Vocabulary:** one controlled status set (above) in headers; evidence tags stay as in CLAUDE.md section 4. UI tiers keep MEASURED_LOOPBACK, SIMULATED, INFERRED, UNAVAILABLE; "T1/T2/T3" stays as experiment tiers, defined in the glossary with the mapping.

**Linking:** repo-relative links only; no `file:///` or drive paths in current documents.

## 6. Document-by-document plan with risk class

Risk classes: **R0** new file, no existing file touched; **R1** edit to a living document; **R2** append-only dated notice on a historical document (body unchanged); **R3** touches a frozen/sealed/owner-controlled item (needs owner approval, or must not be touched).

| Item | Action | Risk |
|---|---|---|
| `docs/STATUS.md`, `docs/README.md`, `glossary.md`, `research/README.md`, `adr/README.md`, `contributing.md`, `testing/strategy.md`, evidence READMEs | Create | R0 |
| New M7 and M8 status records (outlines in D1 section 4) | Create, clearly labelled as reconstructed from evidence | R0 |
| `g3_verdict.md` | Create after the N-6/N-7 re-check | R0 |
| Milestone completion reports (phase 1 to 5), `phase_*_execution_package` headers | Append dated notice pointing to `STATUS.md` and the accountability report | R2 (OWNER: B-4, notices vs. a separate index only) |
| `progress_ledger.md`, `master_audit_and_execution_plan.md`, accountability report | Append a dated "status pointer and corrections" entry (G1, G2, G3, vision phase, cleanup); do not edit old text | R2 |
| `claims_ledger.md` | Append dated reconciliation rows (C1, C2, C7 statuses per its section 6); do not rewrite the original table | R2 |
| `security-demo-matrix.md`, `docs/demo-lab/*`, Scenario B contract | Append "Superseded/Stale since 2026-10-30" banner naming what is wrong and where truth lives | R2 |
| `troubleshoot.md`, `docs/user/*` line-number references, README API list | Correct in place (living documents) | R1 |
| README: test counts, `cargo run`, API list | Edit (`cargo run` fix itself touches `Cargo.toml`: OWNER) | R1 / R3 |
| `walkthrough.md`, `MILESTONE_1_DESIGN_PLAN.md`, `docs/implementation/*`, `docs/testing/*`, handoff 01 to 04 and 06 to 08, `MANIFEST.md`, `PHASE_2_WORK_REPORT.md` | Add status banner (Historical) and, where missing, an H1; do not edit bodies | R2 |
| Root `CORRECTIONS.md` vs `docs/handoff/CORRECTIONS.md` | Cross-link in a new note; renaming is OWNER | R2 / R3 |
| `CLAUDE.md` sections 5 and 6 | List of suggested edits delivered to you (60 s literals now `server.rs:1405`/`:1688`; faithful eviction is in the serving cache at `cache.rs:209`; the insecure-cluster remark; 242 baseline) | R3 (OWNER) |
| 209 `file:///` links, 9 broken relative links, 1 anchor | Rewrite to relative form in **living** documents only | R1 |
| Preregistrations, freeze records, sealed results, G1/G2 verdicts, browser reports, storage record, patches, snapshots, handoff bodies, `lab-results/`, `bench-results/` | Do not touch | R3 / must-not-touch |
| Phase `*_changes.patch` bulk (about 2.9 MB), 140 KB ledger, 64 KB accountability report | Leave; consider splitting the ledger's dashboard from its log only with your approval | R3 (OWNER) |
| `docs-lint` script (D4 proposal) | Create `scripts/docs-lint.mjs` plus `docs/.doclint.json` allowlist with expiries; report-only with baseline first; no new dependency | R0 (script) |

## 7. Maintenance approach
- **When code, an experiment or a gate changes, update `docs/STATUS.md` in the same change** and append one ledger entry; nothing else restates status.
- **docs-lint in CI or as a pre-commit check** (report-only first, then enforce links, absolute paths and secret-adjacent paths on `current` documents; status and fact-drift rules on new or touched documents).
- **Quarterly (or per release) review pass** using the D1 to D4 prompts as a checklist.
- **Review-before-merge rule for claims:** any document that states "verified" must cite the independent check.

## 8. Prioritised roadmap

| Priority | Work | Owner | Depends on | Effort (est.) |
|---|---|---|---|---|
| P0 | `docs/STATUS.md` plus a dated pointer entry in ledger, master plan and accountability report (G1/G2/G3, cleanup, vision); claims-ledger reconciliation rows | Lead | None | 0.5 day |
| P0 | Secret-adjacent line check by someone other than the author (15 lines, 7 files) | Reviewer agent | None | 1 hour |
| P1 | Banners on stale/superseded documents; fix `troubleshoot.md`, README facts, broken links in living documents | Doc writer | P0 | 0.5 to 1 day |
| P1 | `docs/README.md`, `glossary.md`, `research/README.md`, ADR index, evidence READMEs | Doc writer | P0 | 1 day |
| P2 | M7 and M8 status records; test strategy; contributing guide; M3 and TTL ADRs | Doc writer plus lead | Owner decisions B-1..B-8 | 1 to 2 days |
| P2 | `docs-lint` script and allowlist, baseline report | Doc writer | P1 | 1 day |
| P3 | Optional: split ledger dashboard from log, README restructure | Owner decision | | |

These do not touch source or frozen files and can run in parallel with G3/G4; they must not run while another agent is editing the same files, so the status-pointer entries (appends) go first and one writer per file applies thereafter.

## 9. Decisions for you

| ID | Decision | My recommendation |
|---|---|---|
| B-1 | Add Rev 3 of `mvp_and_milestones.md` with M8 (and relabel from PROPOSED), or record M8 only in the new `STATUS.md`? | Record in `STATUS.md`; leave Rev 2 unchanged and add a dated pointer |
| B-2 | Ratify or reject agent-recorded decisions: ADR-003, M8 stages A to F (O8-0), approvals A-1 to A-14, and the unrecorded `G-RESEARCH` gate | Record each as ratified, rejected or waived with a date, in the ADR index; no retroactive rewriting |
| B-3 | Which document is the status authority? | `docs/STATUS.md` (new) |
| B-4 | Dated notices on the M1 to M5 completion reports, or a separate index that carries the correction while those reports stay byte-identical? | Notices (append-only), because a reader opening a report should see the caveat |
| B-5 | CLAUDE.md edits (I will hand you the list) | Approve after reading it |
| B-6 | README: split into course-project front door and engineering front door? `cargo run`: document `--bin` or add `default-run` to `Cargo.toml` (frozen)? | Document `--bin` now; `default-run` in the addendum-2 batch |
| B-7 | Two `CORRECTIONS.md`: rename or cross-link? | Cross-link only |
| B-8 | Did you authorize phases 1 to 6 in writing outside the repository? If yes, point me to it and I will cite it | Yours to answer |
| B-9 | Quoting a test baseline: adopt "851 passed, 0 failed, 8 ignored, 75 targets, debug, 2026-10-30" as the baseline row, replacing the cited 242 | Yes, with the date and the command |
| B-10 | Evidence cited on `D:\Temp` paths: copy the needed logs into the repository, or label them "ephemeral, not retained"? (The storage cleanup did not touch `D:\Temp`.) | Label as ephemeral unless a claim depends on it; copy only those |

## 10. What I did and did not verify
- **Verified by me:** `pie: None` at `src/main.rs:433`; the 8 hex-near-"salt/secret" matches are on lines mentioning commitment, hash or digest (values not read).
- **Not verified by me:** every other count and finding above is as reported by D1 to D4 (see their limits). In particular I have not re-run the link checker, and D4's backticked-path list is heuristic.

Nothing will be executed until you approve a batch. Suggested first batch: P0 plus P1 (all R0/R1/R2, no frozen files).
