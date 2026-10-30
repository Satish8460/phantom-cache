# G5 commit plan (proposal only; nothing has been staged or committed)

**Status:** Draft (agent G5PREP; needs owner approval before any staging)
**Audience:** product owner and the G5 commit operator
**Generated:** 2026-10-30. HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`. No pushes are authorised by this document.
**Inputs:** `g5_working_tree_classification.md` (inventory, scans), `commit_lists/NN_*.txt` (explicit path lists, one repo-relative path per line), `progress_ledger.md` section G (owner authorisation of 2026-10-30 and the earlier 12-step order), `STATUS.md`.
**Tags:** FACT (command output / file read), INFERENCE, UNVERIFIED.

## 1. Rules this plan follows

1. Stage by explicit path list only, never `git add -A`, `.`, directories or wildcards. Use `GIT_LITERAL_PATHSPECS=1 git add --pathspec-from-file=<list>` so a path can never be read as a glob.
2. Regenerate `git status --short -uall` and re-derive the lists immediately before staging: the tree was still growing while this plan was written (4,435 to 4,482 entries; FACT). Anything not in a list is unclassified and stays untouched.
3. Nothing is deleted. Historical, frozen and sealed material is preserved; HOLD groups are not staged until the owner releases them.
4. `core.autocrlf=true` and no `.gitattributes` (FACT). See section 6 for the freeze-hash consequence.
5. Every commit ends with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
6. Code groups are committed only after their G4 Cycle 1 area review is recorded (reviews are run separately; the first report file, `docs/release/g4/cycle1_core_engine_review.md`, exists but I did not read it). Documentation groups after the verification in section 5.

## 2. Commit groups, order and sizes

Counts and bytes are from the lists as generated (list file = `docs/release/commit_lists/<id>.txt`). "Mod" = tracked-modified files inside the group.

| # | List id | Files | Bytes | Mod | Purpose | Status |
|---|---|---:|---:|---:|---|---|
| 1 | `01_skill_adoption` (C01) | 5 | 34.6 KiB | 0 | apple-design skill (byte-identical copy, SOURCE.md provenance) + 2 overlay/evidence docs | READY (section 5) |
| 2 | `02_docs_lint_tooling` (C02a) | 2 | 24.1 KiB | 0 | report-only docs-lint script + its config | READY (section 5) |
| 3 | `03_owner_rules_HOLD_OWNER` | 2 | 11.4 KiB | 0 | `CLAUDE.md` (owner-controlled), `CORRECTIONS.md` | HOLD: owner written approval; both are link targets of the index docs |
| 4 | `04_milestone1_plan_REVIEW` | 1 | 52.2 KiB | 0 | `MILESTONE_1_DESIGN_PLAN.md` (has a historical notice; body not fully read by me) | REVIEW: read fully before staging |
| 5 | `05_research_records` (C03) | 101 | 3.0 MiB | 0 | `docs/research/*.md` and subdirs (vision, design_toolkit, doc_audit, adr, m3_durability and ttl_decision prose, ledgers, verdicts, accountability) | after doc verification |
| 6 | `06_research_patches_and_evidence_REVIEW` | 16 | 3.7 MiB | 0 | `phase_N_changes.patch` (2 are UTF-16LE), m3/ttl `proposed_patch*.diff`, evidence, snapshot manifest | REVIEW: optional historical record; owner decides |
| 7 | `07_product_security_release_handoff_docs` (C12) | 67 | 800 KiB | 2 | `README.md`, `PHASE_2_WORK_REPORT.md` (see flags), `docs/user|security|release|demo-lab|handoff (no transcripts)|milestone_1|demo/*.md` | after owner read of the 2 tracked diffs |
| 8 | `08_docs_index_and_historical_notices` (C02b) | 22 | 115 KiB | 15 | `docs/STATUS.md`, `docs/README.md`, glossary, contributing, testing/strategy, research/README, adr/README + the 15 dated notice edits | LAST of the docs wave (section 5.3) |
| 9 | `09_g5_plan_records` | 24+ | 355 KiB | 0 | this plan, the classification and the lists (they change as the tree changes; commit last) | after all |
| 10 | `10_code_core_security_config` | 16 | 279 KiB | 5 | M1/M2/M7: `core/{cache,coalesce,eviction,freshness}`, `intel/{telemetry,diagnostics,origin_metrics}`, `security/*`, `config.rs`, `health.rs`, `control/pie.rs`, `sim/traffic.rs` | wait for area review |
| 11 | `11_code_distributed` | 6 | 204 KiB | 2 | M3: `dist/{node,router,anti_entropy,frame,pool,session}` | cluster had 3 read-only AI review rounds (ledger 01:17, 01:53, 02:16); final re-review result not read by me |
| 12 | `12_code_intelligence` | 7 | 83 KiB | 0 | M5 machinery `src/intelligence/*` (PIE is NOT constructed in `main`; `pie: None`) | wait for area review |
| 13 | `13_code_lab_library_and_cli` | 26 | 347 KiB | 0 | M4/M6/Scenario B: `src/lab/*`, `src/lab_live/*`, `src/bin/phantom-lab.rs`. **`src/lab/rng.rs` embeds the commitment strings: HOLD-DEP (section 4)** | G1 PASS w/ dispositions (`g1_verdict.md`); rng.rs on hold |
| 14 | `14_code_api_main_registration` | 6 | 320 KiB | 5 | `api/server.rs`, `api/lab_api.rs` (G2), `main.rs`, `lib.rs`, `Cargo.toml`, `Cargo.lock` | G2 PASS w/ dispositions (`g2_verdict.md`); M7 two reading reviews (STATUS.md) |
| 15 | `15_tests` | 97 | 1.5 MiB | 3 | `tests/unit`, `tests/integration`, `tests/support`; `m6_research_evaluation_tests.rs` embeds commitments: HOLD-DEP | after 10-14 compile and pass |
| 16 | `16_benches_packaging_scripts` | 23 | 68 KiB | 0 | `benches/*`, `packaging/*`, `scripts/*` except docs-lint | after 14 |
| 17 | `17_ui_source_tests_scripts` | 135 | 1.0 MiB | 4 | `ui/src`, `ui/test` (incl. 19 fixtures), `ui/scripts`, `ui/index.html`, `ui/package.json`, `App.jsx`, `main.jsx` | UI unit tests are reported passing (STATUS.md, agent-reported) |
| 18 | `18_ui_design_docs` | 23 | 469 KiB | 0 | `docs/design/*.md` and `remediation/*.md` (I1-I4 reports) | after 17 |
| 19 | `19_design_prototypes_REVIEW_OWNER` | 23 | 2.1 MiB | 0 | `docs/design/prototypes` (12 PNG, HTML, JSON, MJS) | owner decides on screenshots; default NOT committed |
| 20 | `20_prereg_freeze_sealed_HOLD` | 14 | 142 KiB | 0 | preregistration, amendments, freeze JSONs, addendum + `.pin`, `m6_sealed_results.md`, the two `lab-results/tuning` files the freeze hashes | **HOLD** (64-hex commitments; owner offline classification, section 4) |
| 21 | `21_evidence_curated_PROPOSED_HOLD_OWNER` | 208 | 4.9 MiB | 0 | proposed curated evidence: reports, manifests, results JSON/JSONL/TXT from `docs/demo/*`, `docs/design/remediation/*` (<=1 MiB, no PNG/JPG) | proposal; default NOT committed |
| - | `90_NOT_COMMITTED_evidence_bulk` | 1,954 | 273.8 MiB | 0 | PNG/JPG screenshots, `lab-results/*` except the two tuning files, `bench-results/*` (owner approval A-8 of Phase 1: stays untracked) | never staged by default |
| - | `91_NOT_COMMITTED_local_snapshots` (C10) | 1,701 | 150.9 MiB | 0 | `.local/snapshots/**` (also holds 8 copies of the transcript and 2 `secret_*.tsv`) | never staged |
| - | `92_NEVER_STAGE_transcripts` (C11c) | 3 | 15.2 MiB | 0 | `transcript_copy.jsonl`, `docs/handoff/transcripts/*.jsonl` | never staged (ledger G hard exclusion) |

Differences from the proposal in the task: C03 is split so that frozen patches/evidence (6) and C12-type product docs (7) are separate; C02 is split into tooling (2, no dependencies, can go first) and the index/notice batch (8, which is link-dependent and must go last of the docs); C04 is split into five review areas (10-14); MILESTONE_1_DESIGN_PLAN, CLAUDE.md and CORRECTIONS.md are three separate owner-gated items; C09 is split into a curated proposal (21) and the bulk (90). `docs/handoff` (13 files) is in 7 minus the transcripts. The ledger's 12-step order (docs; core; distributed+security; intelligence; lab; research protocol; ...; UI; evidence tooling) is respected except that research-protocol/freeze (20) stays HOLD.

## 3. Dependencies between groups

- FACT (greps of `crate::` references): `server.rs` references `config`, `health`, `security`, `intelligence`, `core::{cache,coalesce,freshness}`, `dist::{frame,node,router}`, `intel::{diagnostics,origin_metrics,telemetry}` and (via `#[path="lab_api.rs"]`) `lab`, `lab_live`. `cache.rs` needs `eviction`, `freshness`, `diagnostics`; `coalesce.rs` needs `freshness` and `security::upstream`; `router.rs`/`node.rs` need `frame`, `pool`, `session`, `anti_entropy`, `security::cluster`; `config.rs` needs `security`. `src/main.rs` re-declares the whole module tree inline (a second registration besides `lib.rs`, FACT).
- FACT: `Cargo.toml` has 72 `[[test]]` stanzas (68 new), 7 `[[bench]]`, 1 `[[bin]] phantom-lab`; every stanza path exists on disk now, and cargo refuses to load a manifest whose stanza path is missing. The diff of `Cargo.toml` has only 2 hunks, so splitting it needs a hand-edited patch.
- Consequence (INFERENCE, from the above): groups 10-14 are interlocked through `lib.rs`, `main.rs`, `Cargo.toml` and `server.rs`. They can be reviewed and signed off per area, but cannot be committed as five independent compiling commits unless `lib.rs`, `main.rs` and `Cargo.toml` are staged in partial hunks.

Two safe options for the Rust code (the owner chooses; both are verified by the scratch-copy procedure in section 7 before each commit):

- **Option B (recommended: fewest moving parts).** Three commits: R1 = lists 10+11+12+13+14 with a `Cargo.toml` that contains the dependency changes, the `[[bin]]` stanza and no `[[test]]`/`[[bench]]` stanzas; R2 = list 15 plus the `[[test]]` stanzas; R3 = list 16 plus the `[[bench]]` stanzas. The three `Cargo.toml` states are produced by editing a copy of the diff and applying it with `git apply --cached` (text only here; not executed). `Cargo.lock` goes with R1 (it must contain every dependency, including dev-dependencies, to pass `--locked`).
- **Option A (milestone-granular, higher risk).** Order 10, 12, 13, 11, 14 (leaf first, `main.rs`/`server.rs` last), each with partial `lib.rs`/`main.rs`/`Cargo.toml` hunks and compile + clippy in a scratch copy at every step. `server.rs` (2,895 added lines across M1, M2, M5, M7, G2) cannot be split by milestone without rewriting it, so it lands in the last code commit regardless. Not tested by me (UNVERIFIED feasibility).

Other dependencies: 15 needs 10-14 (and `Cargo.toml` registration); 16 needs 10-14; 17 is independent of Rust except that `ui/test` fixtures mirror the Lab API/SSE schema (documented in `docs/design/lab_api_contract.md`, list 18); 18 needs 17 only for narrative; 8 needs 5, 7, and the owner decision on 3 (see section 5.3); 21 needs 18 to make links resolve; 20 is released only after section 4.

## 4. HOLD items and the independent-classification requirement

- `20_prereg_freeze_sealed_HOLD`: preregistration, freeze JSONs and sealed results contain the two 64-hex SHA-256 commitment strings (retired v2, active v2.1). Also embedded: `src/lab/rng.rs:88,93` (list 13) and `tests/integration/m6_research_evaluation_tests.rs:563,724` (list 15). Until the owner classifies those strings offline, do not stage lists 20, and do not stage `src/lab/rng.rs` or that test file; since `rng.rs` is part of the `lab` module that `lib.rs` and `server.rs` need, **R1 and R2 are blocked until the owner decides** (INFERENCE: a commitment is public by design, but that is the owner's classification, not mine).
- The exact comparison procedure for the owner (prints only True/False, never the salt) is in `g5_working_tree_classification.md` section 6. The `D:\Personal\.phantom-sealed-m6` directory was not read.
- Freeze integrity gate (FACT basis: `preregistration_m6_v2_2_addendum_1.json` lists `Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, `tests/integration/m6_research_evaluation_tests.rs` with `frozen_sha256`/`current_sha256`): after R1/R2 are staged, for each such path the bytes that would be committed must hash to the recorded `current_sha256`. Command (text): `git show :<path> | sha256sum` compared by the owner or operator with the JSON value (do not print the value into logs; compare programmatically). Mismatch means the tree changed after the freeze addendum or autocrlf altered bytes. Whether the working tree still equals the recorded hashes is UNVERIFIED (the real-tree test `m6_research_evaluation_tests.rs` is the existing check).
- Recommended owner decision before any commit that contains hashed files: add `.gitattributes` with `-text` (or `eol=lf`) for `docs/research/preregistration_m6*`, `*freeze*`, `*.pin`, `lab-results/**`, and the hashed source files. This is a new file and an architecture-level choice: it needs owner approval and is NOT in any list.
- `docs/research/preregistration_m6_v2_2_addendum_1.pin` (65 bytes) and `m6_sealed_results.md` (mixed CRLF/LF) are in the hold list for the same reason.
- Items excluded by the ledger's hard exclusions and kept out here: transcripts, `bench-results/` (A-8), all of `lab-results/` except `tuning/frozen_arm_params.json` and `tuning/tuning_trials.csv` (both in list 20), `.local/snapshots`, `ui/dist`, `ui/node_modules`, logs.

## 5. FIRST ELIGIBLE SET

### 5.1 Which groups qualify

- **Commit 1: `01_skill_adoption` (5 files).** FACT: self-contained. The link checker finds 0 relative links to anything outside the set and 0 dangling references (the 4 `.md` files link nowhere). FACT: `SKILL.md` and `LICENSE` match both the SHA-256 and the git blob hash recorded in `SOURCE.md` (recomputed locally, only match/no-match was printed). FACT: no ignore rule hits any of the 5 paths. Scan notes: `docs/design/apple_design_skill_adoption_evidence.md:18` has a drive path (a documented evidence location); `SOURCE.md` has 2 lines with public file hashes (not secrets).
- **Commit 2: `02_docs_lint_tooling` (2 files).** FACT: `scripts/docs-lint.mjs` is report-only (the only write is behind the explicit `--baseline FILE` flag, line 301) and `docs/.doclint.json` is read by it; neither links to other repo files. `node scripts/docs-lint.mjs` ran with exit 0 here (no write). The config names 28 allow-list entries all expiring 2027-01-31.
- **NOT first: C02b `08_docs_index_and_historical_notices`.** FACT (link checker over its 22 files plus resolving `docs/` links): it is not self-contained. Relative links from the index docs point to many files that are in later groups.

### 5.2 Dangling references, by commit position of `08_*`

| Scenario | Links to files not in HEAD, not in the set and not yet committed |
|---|---:|
| `08` committed right after 1 and 2 | 150 distinct targets |
| `08` after docs wave 4, 5, 7 (no owner rules, no code) | 37 targets |
| `08` after every non-held group (including owner rules 3) | 21 targets, all intentional: 10 preregistration/sealed targets (`docs/research/m6_sealed_results.md`, `preregistration_m6*.{md,json}`, `...addendum_1.{md,json,pin}`, `phase_5_preregistration.md`), 4 evidence READMEs and 3 browser-verification `report.md` files plus directory links (`docs/demo/browser_verification/**`, `docs/demo/scenario_b_verification/README.md`, `lab-results/README.md`, `bench-results/README.md`), and `docs/research/intelligence_e2_headroom_protocol_DRAFT.md` |

Specific findings:
- FACT: `docs/research/intelligence_e2_headroom_protocol_DRAFT.md` exists but is **ignored by the global gitignore** (`C:\Users\Satish\.gitignore_global:34 *draft*.md`). It would be silently skipped by any directory-level add and `docs/research/README.md:82` links to it. Needs an owner decision: `git add -f -- <that path>` (explicit) or edit the link. It is in no list.
- FACT: `docs/STATUS.md:18,19,65` pointed to `release/workstream_completion_ledger.md` and `release/verification_register.md` when docs-lint first ran; both files now exist (created during this task) and are in list 7. Until list 7 is committed those links are dangling.
- FACT (other docs, same checker): list 5 alone has 5 broken path references to `docs/src/...` (a relative path written without `../`, in `architecture_trace_m3.md` and `phase_2_completion_report.md`), list 7 has 1 broken reference in `docs/security/phase-3-design.md` (a regex placeholder, not a file). These are pre-existing text defects in historical records; do not edit them.
- The 15 notice edits link to `../STATUS.md`, `../handoff/CORRECTIONS.md` (list 7) and, for `walkthrough.md`, root `CORRECTIONS.md` (list 3). Therefore the notice edits must come after lists 3 and 7, or be committed with `08`.

### 5.3 Recommendation

1. Commit 1 and Commit 2 now (7 files, 58.7 KiB). They are the smallest, self-contained, evidence-backed set.
2. Docs wave in this order, each verified: 3 (after owner approval) then 4 (after reading it), 5, 7, then 8 last. Accept the remaining intentional forward references (above) and record them in the commit body; they resolve when 20/21 are released, or are marked as "not committed by policy" for `lab-results/` and `bench-results/`.
3. If the owner prefers to avoid any forward reference in history, hold 8 until 20 and 21 are released.

### 5.4 Exact commands (text only; NOT executed by this agent)

Pre-flight (read-only):
```
git rev-parse HEAD
git branch --show-current
git status --short -uall > D:\Temp\g5_status_before.txt
git diff --cached --stat
```
Commit 1:
```
$env:GIT_LITERAL_PATHSPECS = "1"
git add -- .claude/skills/apple-design/LICENSE .claude/skills/apple-design/SKILL.md .claude/skills/apple-design/SOURCE.md docs/design/apple_design_skill_adoption_evidence.md docs/design/apple_design_skill_project_rules.md
```
(equivalent: `git add --pathspec-from-file=docs/release/commit_lists/01_skill_adoption.txt`)

Verification before `git commit`:
```
git diff --cached --name-only | Sort-Object > D:\Temp\g5_c01_staged.txt
Get-Content docs\release\commit_lists\01_skill_adoption.txt | Sort-Object > D:\Temp\g5_c01_list.txt
fc.exe D:\Temp\g5_c01_staged.txt D:\Temp\g5_c01_list.txt         # must report no differences
git diff --cached --stat                                           # 5 files, 0 deletions
git diff --cached --check                                          # whitespace; expect clean
git diff --cached --name-only | git check-ignore -v --stdin --no-index   # expect no output
git checkout-index -a --prefix=D:/Temp/g5_stage_c01/               # index copy; writes only under D:\Temp
node scripts\docs-lint.mjs --root D:\Temp\g5_stage_c01 --json       # note: tool is absent in that copy until commit 2; run from the repo with --root
```
(Run docs-lint from the repository's `scripts/` with `--root D:\Temp\g5_stage_c01`; expect the only findings to be `status-missing` info for the skill files, which are third-party content, and the known abs-path drive-path line.) Link check for commit 1 is trivially 0 broken (no outgoing relative links, FACT). Provenance re-check: recompute SHA-256 of `SKILL.md` and `LICENSE` from `git show :<path>` and compare to `SOURCE.md` programmatically.

Commit 1 message:
```
feat(skills): adopt the apple-design project skill with provenance

Add .claude/skills/apple-design (SKILL.md and LICENSE copied unmodified from the
emilkowalski/skills repository at a pinned commit; SOURCE.md records the commit, the git
blob SHAs and the SHA-256 of both files, all re-verified against the working copy) and two
overlay documents: PhantomCache-specific constraints on how the skill may be used (no
default glass/blur surfaces, no new dependencies) and the adoption/invocation evidence
(owner decision O8-1). No application code, dependency or build file is touched.

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
```
Commit 2 (after commit 1):
```
git add -- docs/.doclint.json scripts/docs-lint.mjs
git diff --cached --name-only            # must equal 02_docs_lint_tooling.txt (2 lines)
node scripts\docs-lint.mjs --limit 3     # report-only; exit code 0; do not use --strict (140 abs-path findings exist in other docs)
```
Message:
```
docs(tooling): add the report-only docs-lint checker and its configuration

scripts/docs-lint.mjs (Node built-ins only) reports broken relative links, absolute paths,
missing status headers/notices, orphans and secret-adjacent lines (file, line and rule only;
the matched text is never printed). It never edits a file; the only write is behind the
explicit --baseline flag. docs/.doclint.json classifies frozen, historical and archived
areas, each allow-list entry with a reason and a 2027-01-31 expiry.

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
```

## 6. Verification that nothing in HEAD is lost or silently changed (task item 5)

- FACT: 52 tracked files; 36 tracked-modified; 0 deleted, 0 renamed, 0 staged. All changes would be preserved by the commits and the previous content stays in `456bb25` history. A backup exists at `D:\Personal\phantom-cache-backups\20261030T152222` (not touched).
- FACT: 22 of the 36 tracked-modified files are purely additive (0 deletions): the 15 notice edits (+4/-0 each), `Cargo.toml +320`, `src/lib.rs +20`, `src/dist/node.rs +622`, `src/control/pie.rs +31`, `src/sim/traffic.rs +134`, `tests/unit/wp4_tests.rs +14`, `ui/src/main.jsx +8`. The other 14 have deletions:
- Flags (not understood or contradicting a stated claim; do not commit unreviewed): (1) `docs/PHASE_2_WORK_REPORT.md`: notice says "original text below is unchanged" but 223 lines were rewritten (title retitled, names replaced by placeholders). Owner should read the diff. (2) `src/control/pie.rs`: 31 added lines give the quarantined legacy stub an optional cache handle that switches eviction policy; CLAUDE.md 6b says not to extend the stub; the added methods are used by `tests/integration/p4_pie_tests.rs:161,302`. Confirm intent in the Cycle 1 review. (3) `Cargo.toml` adds 8 runtime and 6 dev dependencies with no approval record found by me (UNVERIFIED), required by CLAUDE.md section 2. (4) `ui/package.json` adds scripts only (FACT), so `ui/package-lock.json` (tracked, unchanged) stays consistent.
- Pre-commit check for every tracked file: `git diff --numstat -- <path>` must equal the table in the classification file (section 4); a different number means the file changed after this analysis.
- Per-commit "nothing else changed" check: `git status --short -uall` after the commit must equal `D:\Temp\g5_status_before.txt` minus exactly that commit's list.

## 7. Per-group gates, review status, risk and scratch-copy check

Common procedure for a code group (specify only, not run): after staging, `git checkout-index -a --prefix=D:/Temp/g5_stage_NN/` (a copy of the index = HEAD plus staged files; touches nothing in the repo), then in that copy `$env:CARGO_TARGET_DIR='D:\Temp\g5_target'; cargo check --all-targets --locked -j 1`. Alternative with a worktree: `git worktree add D:\Temp\g5_wt HEAD` (checks out only committed state; it writes `.git/worktrees` metadata; staged content must be overlaid), so the checkout-index copy is preferred. Never reuse `target/` of the repo.

| Group | Tests that must pass before commit (exact) | Review status | Risk |
|---|---|---|---|
| 1, 2 | the section 5.4 checks; `node scripts\docs-lint.mjs --limit 3` exit 0 | none other than the lead's evidence file (provenance re-verified here) | low |
| 3 | owner approval; `node scripts\docs-lint.mjs` link check | owner decision pending | medium (CLAUDE.md is the rules file) |
| 4 | read the whole file; docs-lint | none | low |
| 5, 7, 8 | docs-lint on the staged copy: compare totals with the working-tree run (996 files; errors: 140 abs-path, 11 link-broken, 1 h1-missing) and confirm no NEW finding; `git diff --cached --check`; the secret-adjacent list in the classification file reviewed by the owner | doc audit D1-D4 (`docs/research/doc_audit/`) are agent audits; no owner review recorded | medium (claims accuracy; 140 absolute-path findings; mixed line endings in 8 ledgers) |
| 6 | decide encoding (UTF-16LE patches) | none | low value, 3.7 MiB |
| 10 | `cargo fmt --all -- --check`; `cargo clippy --all-targets -j 1 -- -D warnings`; `cargo test --tests -j 1 --no-fail-fast` (reported baseline 851 passed, 0 failed, 8 ignored, 75 targets; agent-reported, debug) | G4 Cycle 1 core review (running elsewhere); earlier code-reading reviews for M2 | high: shared by every other group |
| 11 | same three cargo commands, plus the cluster targets `cargo test --test cluster_followup_tests --test cluster_prewarm_repair_tests --test security_phase4b_router_tests -j 1` | 3 read-only AI review rounds recorded; residuals documented (issuer restart loses owed invalidations until TTL) | high |
| 12 | same cargo commands; `cargo test --test intel_engine_tests --test intel_safety_and_replay_tests -j 1` | none beyond self-report (M5 "machinery only", `STATUS.md`) | medium |
| 13 | same cargo commands; `cargo test --test lab_core_tests --test lab_live_scenario_b_tests -j 1`; the ignored sealed test is NOT run | G1 verdict | medium; rng.rs HOLD |
| 14 | the three cargo commands above (the full `cargo test --tests -j 1 --no-fail-fast` is the gate) | G2 verdict; M7 reading reviews | high |
| 15 | full `cargo test --tests -j 1 --no-fail-fast` on the staged copy | vacuous/flaky tests were identified by reviews and partly fixed (ledger) | medium |
| 16 | `cargo bench --no-run -j 1` (compile only) in the staged copy; `pwsh -File scripts\reproduce_tests.ps1` is NOT run | none | low |
| 17 | in `ui/`: `node --test test` and `node scripts\check-forbidden.mjs`, `node scripts\check-contrast.mjs`, `node scripts\check-a11y.mjs` (the contents of `npm run check`; run by the operator, not by this agent); `npm run build` | 3 browser rounds on a debug build (exploratory); G3 verdict not recorded | medium |
| 18, 19 | docs-lint | none | low |
| 20 | HOLD | owner | high |
| 21 | docs-lint on staged copy; size sum <= 5 MiB | none | low |

## 8. Proposed commit messages for the remaining groups

(Subject + body; each ends with the required trailer. Bodies deliberately state limits, not achievements.)

- **3 (owner rules):** `docs(rules): add project rules (CLAUDE.md) and the corrections ledger` ... "Adds the owner-controlled project rules file and the formal corrections ledger. Content is the owner's; no code. Committed on the owner's written instruction of <date>."
- **4:** `docs(plan): add the historical Milestone 1 design plan` ... "Pre-mission planning file with a dated notice stating that it describes the pre-wiring code."
- **5:** `docs(research): add research, audit and engineering records` ... "Assessments, ADRs, claims ledger (with its reconciliation), verdicts G1 and G2, accountability report, progress ledger, vision and design-toolkit work, documentation audits. Agent-authored; statuses are as recorded in the files (several prior 'PASS' statements are retracted there). Excludes preregistration, freeze and sealed records."
- **6:** `docs(research): preserve phase change patches and durability/TTL proposals` ... "Historical patches, proposed diffs and evidence logs. Two patches are UTF-16LE."
- **7:** `docs: add user, security, release, handoff and demo-lab documentation; correct README claims` ... "README: removes unverified intelligence/PIE/Count-Min claims and adds evidence labels and configuration notes. docs/PHASE_2_WORK_REPORT.md: adds a dated notice and retitles the course report (see diff). Handoff transcripts are not included."
- **8:** `docs: add the status entry point, documentation index, glossary and historical notices` ... "docs/STATUS.md, docs/README.md, glossary, contributing, testing strategy, research and ADR indexes. 15 older documents receive a 4-line dated notice (no deletions) pointing to the current records. Known forward references to preregistration/sealed records and uncommitted evidence are listed in docs/release/g5_commit_plan.md."
- **10:** `feat(core): freshness engine, pluggable eviction, byte accounting, atomic telemetry, security and config modules` ... "M1/M2/M7 core and support modules; the legacy PIE stub gains a quarantine header and an optional cache handle. Interlocks with groups 11-14 through lib.rs/main.rs/server.rs; verified in a clean index copy with cargo check/clippy/test (record commands and counts in the ledger)."
- **11:** `feat(dist): mTLS cluster framing, connection pool, session, anti-entropy and fan-out invalidation with repair ledger` ... "Residual limits stay documented: owed invalidations are lost if the issuing gateway restarts; delivery to non-primary peers is asynchronous."
- **12:** `feat(intelligence): shadow/advisory/canary engine machinery with audit records (not wired into main)` ... "pie: None remains in main; no benefit from adaptive control is claimed."
- **13:** `feat(lab): deterministic lab library, phantom-lab CLI and Scenario B live runner` ... "Includes the M6 evaluation harness code. Sealed results and preregistration records are committed separately."
- **14:** `feat(api): wire config, health, security, intelligence and the Lab API into the gateway; register modules and dependencies` ... "Registers all new modules in lib.rs and main.rs, adds the Lab API (disabled by default), health routes, tenant and key validation, conditional revalidation; Cargo.toml/Cargo.lock add TLS/JWT dependencies."
- **15:** `test: add integration, unit and support harnesses for the gateway, cluster, lab and security` ... "Passing tests do not establish correctness; some pin known-gap behaviour on purpose (tests/integration/known_gaps.rs)."
- **16:** `chore(bench,pack): add criterion benches, packaging and reproduction scripts` ... "No benchmark results are included or claimed."
- **17:** `feat(ui): design system, routed screens, Lab screens and verification tooling (I1-I4)` ... "Replaces the monolithic App.jsx. Provenance badges label each value measured, simulated, inferred or unavailable."
- **18:** `docs(design): UI design system, remediation reports and Lab API contract`
- **20 (after release):** `docs(research): preregistration, amendments, freeze records and sealed M6 results (frozen)` ... "Records are preserved byte-for-byte; no edits after freeze."
- **21 (if approved):** `docs(evidence): curated verification reports and manifests (no screenshots)`
- **9:** `docs(release): add the G5 working-tree classification, commit plan and path lists`

## 9. Procedure per commit (checklist)

1. Pre-flight (section 5.4) and regenerate the list with the status diff.
2. Stage by list. 3. `git diff --cached --name-only` equals the list (no extra, none missing). 4. `git diff --cached --check`, secret/abs-path scan of the staged diff (patterns in the classification file). 5. Gate commands of the group on a clean index copy. 6. Re-read `git diff --cached --stat`. 7. Commit with the message above. 8. Record hash, file count, commands and exit codes in `progress_ledger.md` (separate docs commit). 9. `git status` equals the pre-flight list minus the committed paths.
No push is performed or implied; the owner's authorisation of 2026-10-30 covers "normal reviewed commit-and-push" only after the relevant gate passes.
