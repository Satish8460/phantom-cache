# Commit execution evidence, 2026-11-02 (board D-065 / J13)

**Branch:** `chore/phantomcache-baseline-2026-11-01`, created from `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a`. The original branch `feat/publish-phantomcache-core` (and `origin/feat/publish-phantomcache-core`) still points to `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a`; nothing was merged or cherry-picked into it.
**Procedure for every group (enforced by a script that stops on any failed gate):** HEAD equals the expected parent; the index is empty; the group's file list equals the approved plan CSV; no protected, ignored or unrelated path; hash-sensitive files byte-identical to the recovery checkpoint and not converted on add; explicit paths staged with literal pathspecs (never `git add -A`); staged set equals the list with only additions or modifications; every staged blob equals the filtered working file; no secret-like pattern in the staged diff; then commit, and re-check parent, committed file set, blobs against the working tree, the feature branch ref and the remaining changed-file count.
**Evidence tags:** REPRODUCED here. Per-commit JSON records: `D:\ClaudeTemp\audit\legit\commits\<group>_done.json`.

## 1. Commits
| # | Group | Commit | Parent | Files (A/M) | Change | Normalised CRLF to LF | Secret-pattern hits | Remaining changed files after |
|---|---|---|---|---|---|---|---|---|
| 1 | G0 rules, attributes, agent profile | `dc6faff5dd149e42e84704ae97a8901bc9f74098` | `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` | 5 (3/2) | +182 | 1 (`.gitignore`) | 0 | 1,110 |
| 2 | G5 experimental and benchmark evidence | `04a35acf245a6ef9c681df19df349b39b52edde4` | `dc6faff…` | 255 (255/0) | +538,018 | 0 | 0 | 855 |
| 3 | G4 documentation and records | `3712de2598a3353823ac57dd8f12c2317d0a2b06` | `04a35ac…` | 407 (378/29) | +201,194 / -265 | 15 | 0 | 448 |
| 4 | G1 crate (atomic) | `9e8d2d8890dfeadf75154e34c3a5608a7b316540` | `3712de2…` | 179 (164/15) | +79,662 / -174 | 1 (`tests/unit/wp2_tests.rs`) | 0 | 269 |
| 5 | G2 Control Center UI | `20c283c19cd80a22392d069682d42b853eba7932` | `9e8d2d8…` | 215 (210/5) | +29,098 / -510 | 2 (`ui/package.json`, `ui/package-lock.json`) | 0 | 54 |
| 6 | G3 launchers, scripts, packaging | `2f150c6611b8c0e3e550967574e8f45259bfb505` | `20c283c…` | 48 (47/1) | +7,184 / -28 | 1 (`Start-Phantom-Cluster-Demo.bat`) | 0 | 6 |
| 7 | G6 manifest, browser/UI evidence | `afbcf59ffa498e88a6f5c94ece0b45aba710e390` | `2f150c6…` | 2 (2/0) | +2,042 | 1 (manifest CSV) | 0 | 4 |
| 8 | G7 manifest, generated PDFs | `df15fdb91a2c51939fb418ee2d89c1dcb52349e7` | `afbcf59…` | 1 (1/0) | +8 | 1 (manifest CSV) | 0 | 3 |
| 9 | G8 manifest, local-only files | `eac0c4d62c294864957fa229b214d000c34fa67e` | `df15fdb…` | 1 (1/0) | +29 | 1 (manifest CSV) | 0 | 2 |
| 10 | final records | recorded on the board and in `git log` (this document is part of it) | `eac0c4d…` | plan, group CSV, this document, board | | | | 0 |
Total in commits 1 to 9: 1,113 files. File lists: `D:\ClaudeTemp\audit\legit\commits\<group>_files.txt` and `docs/release/commit_plan_2026-11-01_groups.csv`. Normalised files in G4: `README.md`, the two browser-verification `report.md` files, `a11y_evidence/logs/pids.txt`, four `docs/learning` HTML files, `doc_corrections_DEC020.diff`, `lead_timeline.md`, `lead_working_state.md`, `claims_ledger.md`, `m6_sealed_results.md`, `phase_1_changes.patch`, `ttl_decision/evidence/02_summary.txt`. Whitespace warnings from `git diff --cached --check` (informational only, not a gate): G0 1, G5 4,351 (CSV/JSON rows), G4 1,754, G1 25, G2 10, G3 16, G6 to G8 0.

## 2. Gates and results
| Gate | Result |
|---|---|
| Six repaired PIE tests and WP2 tests before the first commit | `p4_pie_tests` 6 passed, `wp2_tests` 5 passed (debug, `-j 1`) |
| Compiler warnings classified | 12, all dead-code or unused-import in `cluster_followup_tests.rs` (7) and `cluster_prewarm_repair_tests.rs` (5); none in `src/` |
| Clippy, reported separately, no `-D warnings` | exit 0; the same 12 rustc warnings; 0 Clippy-specific lints |
| Committed crate compiles | an archive of commit `9e8d2d8` (G1): `cargo check --locked --offline --all-targets -j 1` exit 0, 3 m 48 s, 12 warnings; G2 to G8 add no Rust code |
| Six evidence files, frozen four, two freeze records byte-identical | working tree equals recovery checkpoint (SHA-256); Git blobs equal checkpoint bytes (12 of 12 at G1); isolated clones with `core.autocrlf=true` and `false`: working-copy SHA-256 mismatches 0 and 0 for all 12 |
| The 22 line-ending normalisations intentional | 20 remained after the two freeze records were protected; all are pure CRLF-to-LF text normalisation (filtered blob equals CR-stripped bytes, 0 lone CR); none is among the `-text` files |
| Evidence archive references resolve | 2,063 of 2,063 archive files re-hash to the committed manifests; of 2,521 references, 2,158 resolve to manifest entries, 43 to directories with entries, 293 to committed files; 17 distinct are placeholders or output directories |
| Legacy cache hook not reintroduced | no `with_cache`/`set_cache` caller of `PieController` in the committed tree; `src/control/pie.rs` blob equals the original HEAD blob |
| Final tree vs recovery checkpoint (4,877 non-ignored checkpoint files) | 1,085 byte-identical; 19 equal to the LF-normalised form; 3 intended differences (`pie.rs` restored to the original HEAD, `p4_pie_tests.rs` repaired, the board); 3,770 not in HEAD by design (`.local` 1,701, evidence kept outside Git 2,035, transcripts and smoke 29, pid/pyc 5) |
| Recovery checkpoint and protected data | 5,024 of 5,024 files re-hash, bundle verifies; `.local/snapshots` 1,730 files present; academic workspace and M3 sandbox present; nothing reset, cleaned, stashed or rewritten; no stash entries |

## 3. Observations that need an owner decision (not acted on)
1. **Hash-pinned files rewritten by `core.autocrlf=true` checkouts.** A scan of the committed tree (869 files at G1) found 34 files whose raw SHA-256 appears inside another committed file; 28 are LF text files without `-text`, so a clone with `core.autocrlf=true` rewrites them to CRLF and their recorded hashes would no longer match in that clone. Examples: `docs/research/preregistration_m6_v2_2_addendum_1.json`, the preregistration documents and amendments, `docs/research/preregistration_m6_v2_freeze.json`, `lab-results/m6_sealed_2026-10-29T234845Z/cells.csv`, tuning `*.csv` and `frozen_arm_params.json`, candidate and dry-run `cells.csv`, `Cargo.lock`, `src/core/cache.rs`, `src/core/coalesce.rs`, and six `.claude/skills/*` files. The committed bytes are unchanged and equal the checkpoint; this is an existing property of text files under `core.autocrlf=true`, which is the reason the frozen four and the 8 files protected here carry `-text`. Observed in an isolated clone: `lab-results/m6_sealed_run_started.marker` (151 to 155 bytes) and the v2.2 addendum JSON (3,723 to 3,785 bytes) differ from the checkpoint under `autocrlf=true`. Proposed: extend `.gitattributes` with `-text` for the pinned set (exact list from `D:\ClaudeTemp\audit\legit\pinned_scan.mjs`), or set a repository-wide `* -text` policy; both need approval.
2. **`Start-Phantom-Cluster-Demo.bat` is stored with LF** (normalised on add). A Windows checkout with `core.autocrlf=true` restores CRLF; with `core.autocrlf=false` the file keeps LF, which `cmd.exe` can mishandle. Proposed: `*.bat text eol=crlf` in `.gitattributes` (needs approval).
3. The 12 compiler warnings (test-helper dead code) remain, so `cargo clippy -- -D warnings` would fail until they are removed or used.
4. Remediations R1 to R6 are unchanged and not implemented; see `docs/release/remediation_proposals_2026-11-01.md`.

## 4. Not verified
`cargo test` beyond `p4_pie_tests` and `wp2_tests`; UI `npm` install, build and tests; the demo end to end; compile of the later commits (they add no Rust); behaviour of any group beyond the stated checks.
