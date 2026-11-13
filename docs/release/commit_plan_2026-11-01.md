# Phased commit plan, 2026-11-01 (board D-062 / J10)

**Status (revision 3, 2026-11-02): EXECUTED.** The nine groups were committed in the order G0, G5, G4, G1, G2, G3, G6, G7, G8 on branch `chore/phantomcache-baseline-2026-11-01` (started from `804a7b8`), followed by a final records commit; the original branch `feat/publish-phantomcache-core` is untouched. Commit hashes, per-commit gates and results: `docs/release/commit_execution_evidence_2026-11-02.md`. Changes made during execution (owner-approved in D-064): `-text` for the two SHA-256-pinned preregistration freeze records; the tenth records commit. The text below is the plan as written before execution; sections 5.3 and 6 describe the procedure that was followed. `git add -A` was never used (CLAUDE.md section 1).
**Original status line (revision 2):** PLAN AND VERIFICATION ONLY. Nothing had been staged, committed, reset, cleaned or stashed; no history was rewritten.
**Evidence tags:** REPRODUCED (run here), VERIFIED-IN-CODE (read, cited), REPORTED (read-only reviewer), INFERRED, NOT RUN.

## 1. Owner decisions recorded (2026-11-01, after D-062)
| # | Decision | Status |
|---|---|---|
| 1 | Approve the nine-group plan, subject to dependency and buildability verification | Verified in part; the order changed (section 5) |
| 2 | `-text` for the six hash-sensitive evidence files only | Done in `.gitattributes`; verified (section 2.4) |
| 3 | Keep the 278 MB of screenshots and PDFs outside Git, with SHA-256 manifests and preserved backup copies | Done (section 2.5) |
| 4 | Raw transcripts and smoke outputs local and ignored; review `.claude/agents` | Done; `.claude/agents/general-purpose.md` reviewed (section 2.6) |
| 5 | Repair `p4_pie_tests`, preserving all six tests, no legacy cache hook | Done; compiles (section 2.3); tests NOT RUN |
| 6 | Code fixes are separate changes with independent review and regression evidence | Proposals only: `docs/release/remediation_proposals_2026-11-01.md` |

## 2. Work performed
### 2.1 Recovery checkpoint (REPRODUCED; re-verified after all later work)
`D:\Personal\phantom-cache-backups\20261101T223413_pre_commit_plan_D062\`: 5,024 files (51 tracked-modified, 4,826 untracked, 147 ignored user files; `target/`, `ui/node_modules/`, `ui/dist/` excluded as regenerable), `repo_all_refs.bundle` (8 refs, verifies), `tracked.patch` (applies to a pristine HEAD export), `manifest.csv` (SHA-256), state records. **Re-verified after the `.gitattributes`, `.gitignore`, `pie.rs`, test and archive work: 5,024 of 5,024 files re-hash to the manifest (0 mismatches, 0 missing), no file newer than the creation window, bundle verifies. The checkpoint is untouched and still holds the pre-restore `pie.rs` and the pre-repair test.** Independent unique-content check of `.local/snapshots`: 249 distinct contents (16.0 MB) absent from the live tree, each re-hashed from the checkpoint (0 mismatches); four snapshot-internal manifests (956 entries) match their files; `.local/` itself was not changed.

### 2.2 `src/control/pie.rs` and the empty-diff / modified-status discrepancy (REPRODUCED)
Restored to HEAD content (blob `59af6dfb033eb8e05bf27a93bf8c2bf73b45a75b`, equal to HEAD). **Cause of the "modified" status with an empty diff:** the index entry recorded a size of 3,169 bytes (the CRLF checkout form produced by `core.autocrlf=true`), but the first restore wrote the LF blob (3,068 bytes; the 101-byte difference equals the file's 101 lines). Git treats a recorded-size mismatch as "modified" without comparing content, whereas `git diff` compares text and finds nothing. Shown without touching the real index: `git ls-files --debug` (size 3169 vs file 3068), `git diff-files --raw` (M with a null hash), and `git update-index --refresh` on a **copy** of the index (via `GIT_INDEX_FILE`, real index hash unchanged) left the entry flagged. **Resolution:** the file was rewritten once in the checkout form (CRLF, 3,169 bytes); `git status` no longer lists it and the blob hash still equals HEAD's. No index manipulation was performed.

### 2.3 Repair of `tests/integration/p4_pie_tests.rs` (VERIFIED-IN-CODE; narrow, authorised)
Only the two tests that used the removed hook were touched; all six tests remain. `test_p4_pie_adaptive_switching_and_rollback`: the controller is created without a cache; assertions use `evaluate_candidate` decisions and `get_active_policy()` (SIEVE → S3-FIFO after Commit), the Rollback decision, and the cooldown NoAction; the assertion that the cache rolls back was removed because HEAD's Rollback branch does not reset `active_policy` (CORRECTIONS C-14), so none is made about the label afterwards. `test_p4_live_pie_control_path_under_concurrent_traffic`: same removal of `with_cache`; after the Commit it now asserts the **serving cache's eviction policy stays SIEVE**, which guards against the removed hook returning. Both carry a "LEGACY STUB TEST" comment; names are unchanged (renames stay in the proposals). The four other tests (LRU order, S3-FIFO one-hit-wonder, all-policies active-op safety, reproducible benchmark comparison) were not edited. **Compile: verified** by `cargo check --locked --offline --all-targets -j 1` (section 5.2). **Run: NOT RUN.**

### 2.4 `.gitattributes` and line endings (REPRODUCED)
Added `-text` for exactly: `bench-results/manifests/{r0_baseline,ra,rb,rc}_manifest.json`, `bench-results/r0_summary.json`, `lab-results/e2e_acceptance_evidence.json`. For each, `git hash-object` with and without filters is now equal. The four frozen files remain unconverted. Files whose committed bytes would still differ from the working bytes: **22** (down from 28), all text sources and documents (for example `.gitignore`, `README.md`, `Start-Phantom-Cluster-Demo.bat`, `tests/unit/wp2_tests.rs`, `ui/package*.json`, a few docs); list `D:\ClaudeTemp\audit\legit\eol_convert_paths.txt` (pre-change list; the six are the difference). This is ordinary CRLF-to-LF normalisation on add and was not changed.

### 2.5 Evidence kept outside Git (REPRODUCED)
Archive: `D:\Personal\phantom-cache-backups\evidence_archive_20261101\` (files + manifests). Manifests (committed, small): `docs/release/evidence_archive/manifest_g6_browser_ui_evidence.sha256.csv` (2,028 files, 241.4 MB), `manifest_g7_generated_pdfs.sha256.csv` (7, 36.7 MB), `manifest_g8_local_only.sha256.csv` (28, 18.5 MB: the two transcripts and 26 smoke files), plus `README.md` there. Each copy was hashed against its source at copy time; the archive was then re-hashed independently against the manifests: **2,063 of 2,063 match, 0 missing.** `.gitignore` rules (extension-scoped under the three evidence directories, `/docs/learning/**/*.pdf`, `/docs/handoff/transcripts/`, `/lab-results/smoke_*/`) hide exactly those 2,063 files: measured newly hidden 2,063, expected 2,063, hidden-but-unexpected 0, expected-but-visible 0. Markdown indexes (for example `docs/design/remediation/00_progress.md`, `docs/demo/browser_verification/README.md`), the sealed M6 folder, `lab-results/README.md`, `docs/handoff/MANIFEST.md` and the manifests remain visible; no tracked file matches an ignore rule. Consequence to know: some documents (handoff manifest, conversation index, `docs/release/g5_working_tree_classification.md`, the browser-verification READMEs) reference files that now exist only locally and in the archive.

### 2.6 `.claude/agents/general-purpose.md` review (VERIFIED by reading)
2,005 bytes, no secrets. It is a project-specific profile: read-only on the repository, writes only inside `D:\Personal\phantom-cache-academic`, evidence-tagging rules mirroring CLAUDE.md. Reusable across sessions, and `.claude/skills/` is already tracked. Caveats for the owner: it names the machine-specific absolute path `D:\Personal\phantom-cache-academic` and the role "AG". Recommended: commit in G0 (done in the grouping), optionally parameterising the path later.

### 2.7 `.gitignore` effects (REPRODUCED against the checkpoint manifest)
First pass: 1,707 newly hidden (`.local/` 1,701, 3 pid, 2 pyc, root `transcript_copy.jsonl`); second pass: 2,063 (section 2.5). In total 3,770 untracked paths hidden, **0 source, test, document-index or sealed-evidence files hidden**, 0 newly visible except new reports.

## 3. Transcript secret scan (REPRODUCED; no values reproduced)
`transcript_copy.jsonl`: 16 matching lines, none a credential (task text, search commands, the `"secret_hash"` test placeholder, "zero credentials" scan reports); 0 hits for JWT-shaped strings, long hex, `sk-`/`ghp_`/`AKIA`/`xox` prefixes, PEM headers, `Bearer <20+>` and `name = <16+ chars>` assignments. Session 2 (`ag_conversation_session_2…`): 124 lines mention the words (mostly the `PHANTOM_JWT_SECRET` setting name); same pattern checks 0, except 4 `Bearer <20+>` lines whose shapes are hyphenated lowercase placeholder words in test code. Pattern-based; not every line was read. Privacy note: the first record concerns unrelated coursework; both transcripts are now local-only.

## 4. Investigations and remediation (no code changed)
Full proposals with test strategies, gates and owner questions: **`docs/release/remediation_proposals_2026-11-01.md`**. Summary: **R1 G4A-01** — code-path race CONFIRMED by reading (`cache.rs:1462` fence vs `:1517` re-read and `:1532` stamp; bumps take no shard mutex); deterministic reproduction only with an injected hook (prior sandbox); natural reachability UNVERIFIED (20 s unhooked stress, 3.16 M fills, 0 violations; absence is not evidence); G4A-08 remains open. **R2 anonymous control/invalidation** — documented intent: anonymous only in development mode on loopback (`docs/user/security.md`), authenticated roles in secure mode; main router has no Host/Origin guard; options A/B/C with owner questions. R3 bypass headers, R4 duplicate module tree, R5 validator hardening and test renames (SEC-3 downgraded to not exploitable), R6 unscheduled items.

## 5. Commit groups, order and dependency verification
### 5.1 Groups (REPRODUCED from `docs/release/commit_plan_2026-11-01_groups.csv`; 1,113 grouped files plus this plan and its CSV)
Visible changed files: 51 tracked-modified + 1,064 untracked = 1,115; grouped 1,113 (+ the plan and CSV added last). 0 overlaps, 0 unclassified.
| Order | Group | Contents | Files |
|---|---|---|---|
| 1 | G0 | `.gitignore`, `.gitattributes`, `CLAUDE.md`, `CORRECTIONS.md`, `.claude/agents/general-purpose.md` | 5 |
| 2 | G5 | `lab-results/**` (sealed M6, tuning, scenario_b, candidates, dry runs), `bench-results/**` | 255 |
| 3 | G4 | documentation and records (`docs/**` text, `README.md`, `MILESTONE_1_DESIGN_PLAN.md`, board, reports, remediation proposals) | 407 (+ plan, CSV) |
| 4 | G1 | `Cargo.toml`, `Cargo.lock`, `src/**`, `tests/**`, `benches/**` (atomic) | 179 |
| 5 | G2 | `ui/**` | 215 |
| 6 | G3 | `Start-Phantom-*.bat`, `scripts/**`, `packaging/**` | 48 |
| 7 | G6 | manifest and README for browser/UI evidence kept outside Git | 2 |
| 8 | G7 | manifest for generated PDFs kept outside Git | 1 |
| 9 | G8 | manifest for local-only transcripts and smoke outputs | 1 |
**Changes to the owner-approved plan:** G4 and G5 now precede G1 (reason below); G6/G7/G8 contain manifests only, because the evidence itself stays outside Git. The board is committed last within G4 because it keeps changing.

### 5.2 Dependency safety (REPRODUCED / VERIFIED-IN-CODE)
- **Compile-time:** exactly one found by grep over `src/`, `tests/`, `benches/`: `src/api/lab_api.rs:67` `include_str!("../../docs/demo/scenario_b_interpretation_notice.md")`, a G4 file. G1 before G4 would not compile (INFERRED from the `include_str!` rule; not run). `Cargo.toml` registers 72 `[[test]]` targets, 1 `[[bin]]` and 7 `[[bench]]` by path; `cargo check --all-targets` (below) loaded all of them, which confirms the paths exist (the earlier reviewer also found every integration test file registered). No `build.rs`.
- **Runtime test data** (cross-group references found by a path-literal scan, heuristic): `src/lab/eval.rs` and `m6_research_evaluation_tests.rs` read `docs/research/preregistration_m6_v2*` files and `lab-results/tuning*/…` (G4, G5); `src/config.rs` and a test compare `docs/user/configuration.md`; `intel_candidates_tests.rs` reads `lab-results/candidates_m5/…`. So G1 tests need G4 and G5 present to pass.
- **UI and scripts:** `ui/package.json` references `scripts/` (G3); UI fixtures reference `lab-results/scenario_b/…` (G5); scripts reference `lab-results`/`bench-results` (G5) and the built release binary. None blocks a commit; they are runtime or launch references.
- **Resulting order** G0, G5, G4, G1, G2, G3 makes every runtime data dependency available when it is needed; G2/G3 reference each other only at run time.
- **Compile check of the crate state (REPRODUCED):** an export of HEAD with the working-tree content of G0, G4, G5 and G1 (850 files overlaid) in `D:\Temp\export_g1_state` ran `cargo check --locked --offline --all-targets -j 1` with a separate target dir: **exit 0, `Finished` in 4 m 15 s, 0 errors, 14 warnings** (dead-code and unused warnings in test targets). `--locked` also confirms `Cargo.lock` agrees with `Cargo.toml`. This covers all lib, bin, test and bench targets including the repaired `p4_pie_tests`. The target directory (0.92 GB, created by this check) was removed; the export source and the log `D:\ClaudeTemp\audit\legit\cargo_check_g1_state.log` were kept.
- **Not verified:** `cargo test` (NOT RUN), `cargo clippy -D warnings` (NOT RUN; 14 warnings exist, so clippy with `-D warnings` may fail), `npm` build or tests for G2, intermediate states (for example G0+G5 alone) are doc/data-only and unchanged from HEAD's crate, whose own buildability was not checked, and the `include_str!` ordering claim was reasoned, not run.

### 5.3 Coherence of each commit
| After commit | State | Verified? |
|---|---|---|
| 1 G0 | rules, attributes, agent profile; no code change | by reading |
| 2 G5 | evidence data added; no code consumes it yet | by reading |
| 3 G4 | documents added; includes the file `lab_api.rs` will embed | by reading |
| 4 G1 | crate compiles with all targets (section 5.2); tests runnable once data is present (G5, G4 present) | compile REPRODUCED; tests NOT RUN |
| 5 G2 | UI; independent of Rust; runtime links to G3 | npm NOT RUN |
| 6 G3 | launchers and scripts consume G1 build output and G2 | by reading |
| 7-9 G6-G8 | manifests only | by reading |
After commit 9 the working tree should differ from HEAD only by ignored/local-only paths, the plan files (if added last), and nothing in `git status` except intended leftovers.

## 6. Procedure per commit (all steps need explicit approval)
1. Record `git rev-parse HEAD`, `git branch --show-current`, `git status --short`.
2. Stage **explicit paths only**, taken from the group's rows in the CSV; never `-A`, `.`, or a directory glob spanning groups.
3. `git diff --cached --name-only` must equal the group's list exactly; `git diff --cached --stat` shows no unexpected file; nothing ignored (`.local/`, evidence, transcripts) is staged.
4. For each staged file compare `git hash-object` (filtered) against the working file; for the 22 conversion files expect LF blobs, for the frozen four and the six `-text` files expect equality; keep the checkpoint manifest as the content reference.
5. Commit with the session attribution line; no push.
6. After the last commit re-hash every committed blob (`git show HEAD:path`) against the checkpoint manifest (target: 0 mismatches except the 22 normalised files and the files intentionally edited after the checkpoint: `p4_pie_tests.rs`, `pie.rs` (equal to HEAD), `.gitignore`, `.gitattributes`, the board and new reports).

## 7. Remaining open items (owner)
1. Approve staging and commits group by group (none authorised now).
2. R1 to R6 in the remediation proposals (separate changes, independent review).
3. Optional: parameterise the machine path in `.claude/agents/general-purpose.md`; decide on the 14 compiler warnings and `clippy -D warnings` before the first release-quality commit.
4. The 22 text files normalised to LF on add (no action unless you want more `-text` entries).
5. Hotspot transport and Phase 1 decisions (separate track).

## 8. Limits
`cargo test`, `cargo clippy`, `npm`, and all behaviour tests NOT RUN. The cross-group path scan is heuristic (string literals only). Reviewer findings are reading-based. Groups were computed from the tree at this time; any later edit requires regenerating the CSV.
