# G5 working-tree classification (read-only inventory)

**Status:** Draft (prepared by agent G5PREP; not owner-reviewed)
**Audience:** product owner, G5 commit operator
**Generated:** 2026-10-30, read-only. HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16` (FACT), branch `feat/publish-phantomcache-core` (FACT). Nothing is staged (`git diff --cached --stat` is empty, FACT); no tracked file is deleted or renamed (FACT: 0 `D`/`R` entries).
**Evidence tags:** FACT (command output), INFERENCE, UNVERIFIED. Companion files: `g5_commit_plan.md`, `commit_lists/NN_*.txt`.

## 0. Method and caveats

- Inventory source: `git status --short -uall` (re-run at the end of this task: **4,454 entries, 36 tracked-modified, 4,418 untracked**; the owner's earlier list had 4,435). FACT: the tree is still moving. Between the owner's list and this run the entries grew by 19, for example `docs/design/remediation/a11y_evidence/*` and `docs/release/workstream_completion_ledger.md` appeared. All counts below are for the 4,454-entry snapshot and must be regenerated immediately before any staging.
- Classification is path-rule based (script in the agent scratchpad, not in the repo) and then spot-checked by reading. Ignored files are not in the status list (FACT: `git check-ignore --no-index` over all 4,454 paths matched nothing). Separately, `git status --ignored` shows 100 ignored items (mostly `*.log`) inside `lab-results/scenario_b`, `.local/snapshots`, `docs/research`, `docs/demo`, `bench-results/logs`, `ui/node_modules`, `ui/dist`; these can never be committed by a directory-level `git add` and are not part of any list.
- The sealed-salt directory and any salt/secret file contents were NOT read. Two files whose names contain "secret" (`.local/snapshots/phase1-.../meta/secret_handling.tsv`, `secret_scan.tsv`) were skipped by the content scans.

## 1. Why there are about 4,435 entries

One commit exists since the project start (`456bb25`, 52 tracked files, 2026-10-11) and every later change is uncommitted. The entries are dominated by evidence, not by code:

| Contributor | Entries | Bytes | Share of entries |
|---|---:|---:|---:|
| `.local/snapshots/` (7 phase entry/exit snapshots: full tree copies, including 8 copies of the 12.5 MiB transcript) | 1,701 | 150.9 MiB | 38.2% |
| `docs/design/` (1,044 under `remediation/`: before/after/preview/a11y screenshots; 23 `prototypes/`) | 1,079 | 115.4 MiB | 24.2% |
| `docs/demo/` (891 `browser_verification/` rounds 1-3: 782 PNG, 80 JSON, 17 JSONL) | 901 | 119.0 MiB | 20.2% |
| `lab-results/` (122 Scenario B, 5 dry runs, 1 sealed run, tuning, smoke) | 217 | 46.5 MiB | 4.9% |
| `docs/research/` (131), `tests/` (97), `ui/` (135), `docs/security|user|release|handoff|demo-lab|...` | about 500 | about 25 MiB | 11% |
| `src/`, `benches/`, `scripts/`, `packaging/`, `.claude/`, root files | about 125 | about 2.5 MiB | 3% |

FACT: 88% of entries (3,914 of 4,454) and 95% of bytes (435 of 459 MiB) are class (e) evidence; only 123 + 193 entries (7%) are source and test files. FACT: of the 2,162 evidence-bulk entries (group C09: browser verification, remediation screenshots/JSON, lab-results, bench-results), 1,721 are PNG/JPG binaries.

## 2. Classification summary

Classes: (a) required source (Rust `src/`, UI `ui/src` + `ui/index.html` + `ui/package.json`, Cargo); (b) required tests and test infrastructure (`tests/`, `ui/test`, `ui/scripts`, `benches/`, `packaging/`, non-doc `scripts/`); (c) documentation and engineering records; (d) generated / cache; (e) frozen, sealed or historical evidence; (f) unknown or potentially important.

| Class | Entries | Bytes | Tracked-modified |
|---|---:|---:|---:|
| (a) required source (Rust src, UI src, Cargo) | 123 | 1.6 MiB | 16 |
| (b) required tests and test infrastructure | 193 | 2.1 MiB | 3 |
| (c) documentation and engineering records | 208 | 4.2 MiB | 17 |
| (d) generated / build output / cache | 12 | 180.8 KiB | 0 |
| (e) frozen, sealed or historical evidence | 3914 | 435.4 MiB | 0 |
| (f) unknown or potentially important | 4 | 15.2 MiB | 0 |
| **Total** | **4454** | **458.8 MiB** | **36** |

Class (d) (12 entries, 181 KiB): 10 `__pycache__/*.pyc` copies inside `.local/snapshots` and 2 in `bench-results/tools/__pycache__`. Not deleted; they stay untracked and should be added to an exclude rule only with owner approval (see plan section 7).

Class (f) (4 entries, 15.2 MiB), unknown or potentially important, listed so none is lost:

| Path | Bytes | Why unknown |
|---|---:|---|
| `transcript_copy.jsonl` | 1.28 MB | raw conversation copy dated Oct 25; may hold secrets or PII (UNVERIFIED: not opened beyond pattern counts: 4,845 absolute-path lines in the two handoff transcripts + this file) |
| `docs/handoff/transcripts/ag_conversation_session_1_oct11_to_oct25.jsonl` | 1.28 MB | same |
| `docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl` | 12.7 MiB | same; >5 MB |
| `docs/design/remediation/legacy/SimDemo.jsx.orig` | small | `.orig` backup of an old UI file; keep (historical), role not confirmed |

Records judged required-to-keep but owner-controlled (class (c), not unknown): `CLAUDE.md` (untracked, 7.9 KB; the file says it is "checked into the codebase" yet it is untracked, FACT), `CORRECTIONS.md` (3.7 KB), `MILESTONE_1_DESIGN_PLAN.md` (53 KB, has a dated historical notice at the top, FACT; body not fully read).

## 3. Per-directory counts and bytes

| Directory | Entries | Bytes | Tracked-mod | Classes |
|---|---:|---:|---:|---|
| .local/snapshots | 1701 | 150.9 MiB | 0 | d,e |
| docs/demo | 901 | 119.0 MiB | 0 | c,e |
| docs/design | 1079 | 115.4 MiB | 0 | c,e,f |
| lab-results/ | 217 | 46.5 MiB | 0 | e |
| docs/handoff | 13 | 14.0 MiB | 0 | e,f |
| docs/research | 131 | 6.8 MiB | 0 | c,e |
| (repo root files) | 8 | 1.4 MiB | 4 | a,c,f |
| tests/integration | 73 | 1.4 MiB | 0 | b |
| ui/src | 60 | 401.6 KiB | 2 | a |
| ui/test | 45 | 357.1 KiB | 0 | b |
| docs/security | 19 | 327.1 KiB | 0 | c |
| bench-results/ | 19 | 263.4 KiB | 0 | d,e |
| ui/scripts | 28 | 257.7 KiB | 0 | b |
| src/lab | 17 | 233.3 KiB | 0 | a |
| src/api | 2 | 220.4 KiB | 1 | a |
| src/dist | 6 | 204.5 KiB | 2 | a |
| src/core | 4 | 125.5 KiB | 2 | a |
| docs/ (top-level files) | 6 | 119.0 KiB | 1 | c |
| docs/demo-lab | 9 | 106.5 KiB | 0 | c |
| src/lab_live | 8 | 104.8 KiB | 0 | a |
| tests/support | 21 | 92.3 KiB | 0 | b |
| src/intelligence | 7 | 83.4 KiB | 0 | a |
| src/ (files) | 4 | 74.6 KiB | 2 | a |
| docs/release | 8 | 56.7 KiB | 0 | c |
| src/security | 5 | 56.1 KiB | 0 | a |
| docs/user | 9 | 54.9 KiB | 0 | c |
| benches/ | 10 | 45.6 KiB | 0 | b |
| src/intel | 3 | 36.7 KiB | 1 | a |
| scripts/ | 7 | 28.8 KiB | 0 | b,c |
| .claude/ | 3 | 25.7 KiB | 0 | c |
| docs/testing | 6 | 23.5 KiB | 5 | c |
| docs/implementation | 9 | 23.2 KiB | 9 | c |
| tests/unit | 3 | 14.3 KiB | 3 | b |
| packaging/ | 7 | 11.1 KiB | 0 | b |
| src/bin | 1 | 8.6 KiB | 0 | a |
| src/sim | 1 | 5.5 KiB | 1 | a |
| src/control | 1 | 4.2 KiB | 1 | a |
| docs/milestone_1 | 1 | 4.1 KiB | 0 | c |
| ui/ (files) | 2 | 1.5 KiB | 2 | a |

Sub-breakdown of `docs/design/`:

| Subdir | Entries | Bytes |
|---|---:|---:|
| remediation | 1044 | 113.1 MiB |
| prototypes | 23 | 2.1 MiB |
| phantomcache_design_direction_v2.md | 1 | 64.8 KiB |
| phantom_lab_ui.md | 1 | 25.4 KiB |
| ui_audit.md | 1 | 13.9 KiB |
| phantomcache_design_system_spec.md | 1 | 11.2 KiB |
| tokens.md | 1 | 10.9 KiB |
| lab_api_contract.md | 1 | 7.1 KiB |
| sse_auth_options.md | 1 | 6.8 KiB |
| ui_api_requirements.md | 1 | 6.6 KiB |
| principles.md | 1 | 5.4 KiB |
| information_architecture.md | 1 | 4.8 KiB |
| apple_design_skill_project_rules.md | 1 | 4.5 KiB |
| apple_design_skill_adoption_evidence.md | 1 | 4.3 KiB |

Sub-breakdown of `docs/demo/`:

| Subdir | Entries | Bytes |
|---|---:|---:|
| browser_verification | 891 | 118.6 MiB |
| scenario_b_verification | 5 | 412.6 KiB |
| scenario_B_origin_slowdown.md | 1 | 7.9 KiB |
| scenario_b_artifacts.md | 1 | 6.8 KiB |
| lab_runner.md | 1 | 4.4 KiB |
| scenario_b_interpretation_notice.md | 1 | 3.5 KiB |
| scenario_b_runbook.md | 1 | 2.8 KiB |

Sub-breakdown of `.local/snapshots/`:

| Subdir | Entries | Bytes |
|---|---:|---:|
| phase1-20261028T185855Z-456bb25 | 312 | 33.9 MiB |
| phase5-exit-20261029T124500Z-456bb25 | 274 | 21.1 MiB |
| phase4-exit-20261029T113000Z-456bb25 | 247 | 19.8 MiB |
| phase5-entry-20261029T113500Z-456bb25 | 247 | 19.8 MiB |
| phase3-exit-20261029T104000Z-456bb25 | 224 | 19.4 MiB |
| phase4-entry-20261029T104500Z-456bb25 | 224 | 19.4 MiB |
| phase2-20261029T063500Z-456bb25 | 173 | 17.6 MiB |

Sub-breakdown of `lab-results/`:

| Subdir | Entries | Bytes |
|---|---:|---:|
| scenario_b | 122 | 27.5 MiB |
| m6_dryrun_2026-10-29T232620Z | 9 | 2.4 MiB |
| m6_dryrun_2026-10-29T230826Z | 9 | 2.4 MiB |
| m6_dryrun_2026-10-29T231301Z | 9 | 2.4 MiB |
| m6_dryrun_2026-10-29T225701Z | 9 | 2.4 MiB |
| m6_sealed_2026-10-29T234845Z | 9 | 2.4 MiB |
| smoke_lib_det_a | 4 | 2.1 MiB |
| smoke_lib_det_b | 4 | 2.1 MiB |
| m6_dryrun_2026-10-29T230705Z_ABORTED_INCOMPLETE_by_my_own_timeout | 1 | 1.8 MiB |
| smoke_m6_raw | 4 | 349.2 KiB |
| candidates_m5 | 7 | 119.3 KiB |
| tuning_v2_2c | 2 | 81.8 KiB |
| tuning_v2_2 | 2 | 81.7 KiB |
| tuning_v2_2c_round1 | 2 | 62.7 KiB |

By extension (all entries):

| Ext | Entries | Bytes |
|---|---:|---:|
| .png | 1654 | 220.1 MiB |
| .jsonl | 88 | 137.7 MiB |
| .csv | 121 | 44.9 MiB |
| .md | 996 | 14.5 MiB |
| .patch | 26 | 11.3 MiB |
| .rs | 795 | 11.2 MiB |
| .jpg | 77 | 8.9 MiB |
| .json | 301 | 5.9 MiB |
| .mjs | 75 | 722.7 KiB |
| .js | 42 | 617.5 KiB |
| .lock | 9 | 592.7 KiB |
| .jsx | 52 | 534.7 KiB |

## 4. Tracked-modified files (36): lines added/removed vs HEAD and change type

FACT source: `git diff --numstat` plus reading the diffs. Workstream labels are INFERENCE from `docs/research/progress_ledger.md` (section G plan), `docs/STATUS.md` and identifier counts in the added lines.

| File | + | - | Change type and workstream |
|---|---:|---:|---|
| `Cargo.lock` | 1800 | 30 | regenerated for new dependencies (TLS, JWT, benches). The 30 removed lines are re-sorted entries (`http 1.5.0`, `http-body 1.1.0` are still present, FACT). M3/M7 security + M1 benches |
| `Cargo.toml` | 320 | 0 | purely additive: 8 runtime deps (`axum-server`, `jsonwebtoken`, `rustls`, `tokio-rustls`, `rustls-pemfile`, `webpki-roots`, `url`, `ring`), 6 dev-deps (`jsonwebtoken`, `rcgen`, `reqwest`, `criterion`, `hdrhistogram`, `proptest`), 68 `[[test]]`, 7 `[[bench]]`, 1 `[[bin]]` (`phantom-lab`). Two diff hunks only. CLAUDE.md section 2 requires owner approval for each new dependency; the approval record was NOT located by me (UNVERIFIED) |
| `README.md` | 70 | 19 | claim corrections (removes "intelligent", Count-Min/PIE claims; adds evidence labels, config/ops section, health routes). Deliberate, matches CORRECTIONS/claims work. Docs |
| `docs/PHASE_2_WORK_REPORT.md` | 555 | 223 | **not just a notice**: a 4-line dated notice plus a retitled report (title changed to "Prototype"), anonymised student names replaced by bracket placeholders, a TOC added. The notice says "original text below is unchanged", but 223 lines were removed or rewritten (FACT from numstat). Flag for owner read before commit. HEAD keeps the original |
| 9 files `docs/implementation/*.md` | 4 | 0 | dated "Notice (2026-10-30)" blockquote prepended; original text unchanged (FACT: +4/-0 each) |
| 5 files `docs/testing/*.md` (ENVIRONMENT_READINESS, FLAGSHIP_VALIDATION, V2, V3, V4) | 4 | 0 | same notice pattern |
| `walkthrough.md` | 4 | 0 | same notice pattern (links to root `CORRECTIONS.md` and `docs/STATUS.md`) |
| `src/api/server.rs` | 2895 | 41 | largest mixed diff: M7 security (`security::` 70 hits, TLS 28, config), tenant validation and 414 key cap (M2/S-3 follow-up), conditional revalidation/304 (M2), diagnostics/origin metrics (M1), intelligence wiring (M5), `#[path="lab_api.rs"] pub mod lab_api` (G2 Lab API), health routes. References nearly every new module, so it cannot be committed before them |
| `src/core/cache.rs` | 1807 | 22 | M2 core: freshness/revalidation, eviction trait, epochs (89 hits), sweeper, byte accounting, canonical key |
| `src/core/coalesce.rs` | 264 | 4 | M2: `Coalescer` finalize outcomes, depends on `core::freshness` and `security::upstream` |
| `src/dist/node.rs` | 622 | 0 | M3: TLS listener, frame/session/anti-entropy hooks, test controls |
| `src/dist/router.rs` | 2211 | 5 | M3: fan-out to all peers, RepairLedger, pre-warm pool, frame protocol, anti-entropy worker |
| `src/intel/telemetry.rs` | 633 | 14 | M1: Mutex raw buffer replaced by atomic log-linear histogram (31 hits), observation kinds, origin metrics |
| `src/control/pie.rs` | 31 | 0 | legacy-quarantine doc header plus optional `cache` field with `with_cache`/`set_cache` that switches the eviction policy on commit/rollback. Used by `tests/integration/p4_pie_tests.rs:161,302` (FACT). CLAUDE.md 6b says do not extend the stub; this addition keeps old tests compiling against the new `EvictionPolicy`. NOT fully understood as intended scope: owner/reviewer to confirm |
| `src/lib.rs` | 20 | 0 | module registration only: `core::{eviction,freshness}`, `intel::{diagnostics,origin_metrics}`, `dist::{anti_entropy,frame,pool,session}`, `security`, `intelligence`, `config`, `health`, `lab`, `lab_live` |
| `src/main.rs` | 468 | 5 | binary re-declares the module tree inline (duplicates `lib.rs` registration) and adds config/health/lab wiring, TLS cluster setup, `pie: None` kept |
| `src/sim/traffic.rs` | 134 | 0 | additive request-type and workload-weight types (M4 lab workloads, used by `p3_traffic_tests`); INFERENCE |
| `tests/unit/wp1_tests.rs` | 64 | 2 | adds a 200k unique-key flood capacity test (M2) and `ObservationKind` adaptation (M1 schema) |
| `tests/unit/wp2_tests.rs` | 4 | 1 | small API adaptation (file has mixed CRLF/LF, FACT) |
| `tests/unit/wp4_tests.rs` | 14 | 0 | `ObservationKind` adaptation |
| `ui/index.html` | 5 | 1 | title, color-scheme/theme-color meta, inline icon (M8 I1) |
| `ui/package.json` | 6 | 1 | adds `test`, `check:*` and `check` scripts only; no new dependency (FACT) |
| `ui/src/App.jsx` | 139 | 508 | rewritten into a routed shell using new components/hooks/lib/styles (M8 UI remediation I1-I4). HEAD keeps the old monolith |
| `ui/src/main.jsx` | 8 | 0 | theme applied before first paint |

All modifications are intended or explained, except two that I flag as NOT fully understood: (1) `docs/PHASE_2_WORK_REPORT.md` (large rewrite contradicting its own notice), (2) `src/control/pie.rs` (functional addition to a quarantined stub). `Cargo.toml` dependency approvals are UNVERIFIED.

## 5. Safety scans (read-only; no value printed)

Scans covered every non-binary entry (patterns only, results are file:line:rule). Procedure and results:

| Check | Procedure | Result |
|---|---|---|
| Large files >5 MiB | `fs.stat` over all entries | FACT: 9 files: the 12.7 MiB handoff transcript and its 8 snapshot copies. None else |
| Binary files | extension + NUL byte in first 8 KB | FACT: 1,721 in C09 (PNG/JPG), 15 in snapshots, 12 design prototype PNGs, 2 research patches `phase_5_changes.patch` / `phase_6_changes.patch` (they start with a UTF-16LE BOM `ff fe`, so git will treat them as binary and `git diff` will be unreadable; INFERENCE: written by PowerShell redirection) |
| `.gitignore` conflicts | `git check-ignore -v --no-index` over all 4,454 paths | FACT: no match; no listed path is ignored. Ignored companions (100) exist but are outside the lists |
| Key/secret file names | name regex `\.(key|pem|pfx|p12|crt|cer|der)$`, `.env*`, `salt`, `secret`, `id_rsa`, `.pin` | FACT: none in any commit list except `docs/research/preregistration_m6_v2_2_addendum_1.pin` (in the HOLD list 20) and the two `secret_*.tsv` files (in the NEVER list 91) |
| Absolute-path scan | `[A-Za-z]:[\\/](Users|Personal|Temp|ClaudeTemp)`, `/c|d/(Users|Personal|Temp)`, plus docs-lint `file:///` | FACT counts of lines (files): C01 1 (1); C02 10 (5); C03 172 (37); C03E 54 (6); C04 src 0; C05 tests 4 (2); C06 ui+design 30 (16); C07 0; C08 15 (6); C12 171 (29); handoff transcripts 4,845 (4). Docs mostly cite evidence locations (D:\Temp logs), which is the project's provenance convention; src has none. The `file:///` links inside 8 historical docs predate this work |
| Secret-adjacent | line has salt/secret/private key/password/api key/credential AND a drive path or 64+ hex, or token-shape (`AKIA`, `ghp_`, `sk-`, JWT, PEM header) | FACT, file:line:rule only. C03: `docs/research/doc_audit/d2_research_docs_audit.md:291`, `d4_mechanical_checks.md:79,80`, `phase_1_completion_report.md:79`, `phase_1_execution_package.md:60`, `phase_1_progress.md:41`, `progress_ledger.md:142,144,170,182,188,190,218,220,240,246`, `vision/v3_topology_tenancy_identity.md:71`, `vision/v4_security_and_failure_scenarios.md:93` (rule kw+drivepath: documents naming the external sealed directory path, not a value). C04: `src/config.rs:175` (kw+drivepath, text only), `src/lab/rng.rs:88` (kw+hex64: the retired commitment in a doc comment). C05: `tests/integration/lab_api/mod.rs:640`, `lab_api/review_tests_c.rs:146`, `m7_sec_fixes_tests.rs:722`, `m7_security_review_tests.rs:187` (kw+drivepath), `m6_research_evaluation_tests.rs:563,724` (kw+hex64: commitments), `m7_security_review_tests.rs:39` (token-shape: an unsigned `alg=none` JWT used as a negative-test input; INFERENCE from the test name and the assertion `is_err()` on line 40). C06: `ui/scripts/browser-gallery.mjs:8`, `ui/test/fixtures/lab/errors.json:2`. C08: prereg docs/freeze JSON (commitments, see section 6). C12: `docs/demo-lab/research-references.md:74`, `docs/demo-lab/security-scenario-catalogue.md:83`, `docs/release/execution_register.md:8`, `docs/security/m7_security_review.md:18`, `docs/user/security.md:53`. `docs/demo-lab/README.md:73,75,77` flagged by docs-lint. None of these were inspected for a real secret value beyond the rule match; owner/reviewer to confirm each is a path/hash, not a credential (UNVERIFIED) |
| 64+ hex strings | regex `[0-9a-f]{64,}` | FACT: hash values in manifests, freeze records, SOURCE.md (public file hashes), patches (724 lines in 5 C03E files). The two sealed-salt commitments are enumerated in section 6 |
| CRLF / LF | count `\r\n` vs bare `\n` | FACT: mixed in C03 (8 files: `claims_ledger.md`, `demo_experience_plan.md`, `g1_verdict.md`, `g2_verdict.md`, `master_audit_and_execution_plan.md`, `novelty_snapshot_m6.md`, `phantomcache_milestone_accountability_report.md`, `progress_ledger.md`), C05 (`tests/unit/wp2_tests.rs`), C08 (`m6_sealed_results.md`), C12 (`README.md`, `scenario_b_interpretation_notice.md`), plus 3 in evidence and 23 in snapshots. CRLF-only: the 15 notice-edited tracked docs |
| docs-lint | `node scripts/docs-lint.mjs --json` (report-only; reads, writes nothing) | FACT: 996 files scanned; errors: 140 abs-path (48 files), 11 link-broken (3 files), 1 h1-missing (`docs/design/prototypes/perf/perf_tables.md:1`); warnings: 84 fact-drift, 72 orphan, 57 path-ref-missing, 42 notice-missing. Full output reproduced by the command, not stored in the repo |
| Dangling links in first-eligible set | see plan section 5 | C01: 0. C02 index/notices: 3 nonexistent targets plus about 160 forward references |

Line-ending risk (FACT: `core.autocrlf=true`, no `.gitattributes`): git will print "LF will be replaced by CRLF" for files with LF working copies. Committed blobs normalise CRLF to LF. Freeze/preregistration records hash working-tree bytes (for example `Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, the M6 test file in `preregistration_m6_v2_2_addendum_1.json`, FACT). A mixed or CRLF file committed through autocrlf can have a blob hash different from the hash recorded in a freeze. See plan section 6 for the verification gate.

## 6. The two 64-character commitment strings (OWNER, offline)

FACT: two different SHA-256 commitment strings exist. "Retired v2" (documented as an exposed, never-used salt) and "active v2.1" (the commitment to the salt held outside the repo). Files containing them (file:line only, found by exact string match, values not printed):

| String | Files |
|---|---|
| retired v2 | `docs/research/preregistration_m6_v2.md:100`, `preregistration_m6_v2_1_amendment.md:23`, `preregistration_m6_v2_1_freeze.json:10`, `preregistration_m6_v2_2_freeze.json:14`, `preregistration_m6_v2_freeze.json:5`, `src/lab/rng.rs:88`, `tests/integration/m6_research_evaluation_tests.rs:563,724` |
| active v2.1 | `docs/research/preregistration_m6_v2_1_amendment.md:24`, `preregistration_m6_v2_1_freeze.json:9`, `preregistration_m6_v2_2_freeze.json:13`, `src/lab/rng.rs:93` (the compiled constant), and 18 copies under `lab-results/` and `.local/snapshots/` manifests |

Consequence: HOLD applies not only to the C08 prereg/freeze files but also to `src/lab/rng.rs` (code group 13) and `tests/integration/m6_research_evaluation_tests.rs` (group 15) until the owner releases the strings. If the owner classifies a commitment as public by design (it is SHA-256 of a 32-byte salt, FACT from `src/lab/rng.rs:142-146`: `sha256_hex(salt) == commitment`), these files may be released; that is the owner's call, not mine.

Owner/offline comparison procedure (PowerShell, prints only True/False; the salt never reaches the console, logs, history or the clipboard; run in a fresh `powershell` session, not through an agent):

```powershell
$hex  = (Get-Content -Raw 'D:\Personal\.phantom-sealed-m6\m6_salt_v2_1.hex').Trim()
$b    = [byte[]]::new($hex.Length / 2)
for ($i = 0; $i -lt $b.Length; $i++) { $b[$i] = [Convert]::ToByte($hex.Substring(2*$i, 2), 16) }
$got  = ([System.Security.Cryptography.SHA256]::HashData($b) | ForEach-Object { $_.ToString('x2') }) -join ''
$line = (Select-String -Path 'D:\Personal\phantom-cache\src\lab\rng.rs' -Pattern 'SEALED_SALT_COMMITMENT_SHA256: &str' -Context 0,1).Context.PostContext[0]
$want = ([regex]::Match($line, '[0-9a-f]{64}')).Value
($got -eq $want)                      # expect True; print nothing else
Remove-Variable hex,b,got,line,want
```

The hash is over the decoded bytes of the hex salt (not over the hex text), exactly as `derive_sealed_seeds` does. Then the owner decides (a) whether the commitment strings may be committed, (b) whether the retired v2 string needs any additional handling. Do not paste any output other than True/False.

## 7. Unknowns and open items

1. `transcript_copy.jsonl` and `docs/handoff/transcripts/*` content (PII/secrets): not reviewed, excluded.
2. `docs/design/remediation/legacy/SimDemo.jsx.orig`: keep or drop is an owner decision (not touched).
3. `docs/research/preregistration_m6_v2_2_addendum_1.pin`: 65 bytes, role documented in the addendum (a SHA-256 pin of the addendum JSON); HOLD.
4. Approval records for new Cargo dependencies and dev-dependencies (CLAUDE.md section 2): not found by me.
5. `.github/workflows/ci.yml` (tracked, unchanged) runs `cargo ... --all-features` and `cargo fmt --all -- --check`; the repo has no `[features]` table, and no record shows `cargo fmt --check` was run on the current tree (UNVERIFIED). Gate risk for G5.
6. `docs/research/phase_5_changes.patch` / `phase_6_changes.patch` are UTF-16LE; unreadable in review diffs.
7. 100 ignored files (logs) sit inside evidence directories; any provenance that points to them cannot be recovered from a clone.
8. The tree is still being written (4,435 to 4,454 during this task).
9. FACT: the global gitignore `C:\Users\Satish\.gitignore_global` (not the repo `.gitignore`) also ignores `*draft*.md`, `*notes*.txt`, `*todo*.md`, `scratch/`, `*.log`, `.env*`, `*.local.yaml`. It silently hides `docs/research/intelligence_e2_headroom_protocol_DRAFT.md` (linked from `docs/research/README.md:82`); it needs an explicit owner decision (`git add -f` of that one path, or edit the link). `docs/research/m3_durability/evidence/` (logs) is ignored the same way. Neither is in any commit list.
