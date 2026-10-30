# docs-lint baseline run

Date: 2026-10-30. Writer W4. Evidence tag for all numbers: reproduced here (script run, deterministic: two consecutive `--json` runs were byte-identical).

Snapshot: run 2026-10-30 11:58:05 to 11:58:06 +05:30 (earlier run 11:57:43). **942 markdown files scanned** (frozen 773, current 114, historical 55). Other writers (W1, W2, W3) were editing docs concurrently, so counts will drift; `docs/STATUS.md` did not yet exist at the time of the run, and `docs/README.md` (new, W2) was still being written.

## How to run

```
node scripts/docs-lint.mjs                      # human summary, 25 findings per rule
node scripts/docs-lint.mjs --json               # full machine output (no timestamps; deterministic)
node scripts/docs-lint.mjs --limit 100          # more lines per rule in human output
node scripts/docs-lint.mjs --baseline FILE      # write current findings as the accepted baseline (only file it ever writes)
node scripts/docs-lint.mjs --new-only FILE      # report only findings not in that baseline
node scripts/docs-lint.mjs --strict             # exit 1 if any error-level finding remains (never used automatically)
```

Exit code: 0 always; 2 on tool/config failure; 1 only with `--strict` and at least one error-level finding. The script is report-only: it never modifies, rewrites or fixes any file. Config: `docs/.doclint.json`. No baseline file was written into the repo.

## Modes (docs/.doclint.json)

First matching `classify` glob wins; default `current`. Every entry has a reason and expiry 2027-01-31 (an expired entry still applies but raises `allowlist-expired`).

| Mode | Rules that run |
|---|---|
| current | all |
| historical | link-broken, anchor-broken, secret-adjacent, size, notice-missing (warn: needs `Notice (` or `Historical` in first 40 lines) |
| archived | links, secret, size, Status must be Archived or Superseded |
| frozen | secret-adjacent only |

Seeded: frozen = `.local/**`, `lab-results/**`, `bench-results/**`, `docs/demo/browser_verification/**`, `docs/demo/scenario_b_verification/**`, `preregistration_m6_*`, `**/*freeze*`. Historical = progress_ledger, `phase_*_progress/completion/execution/patch*`, storage_cleanup record, `docs/handoff/**`, `docs/security/phase-*` and `stage-*`, `docs/research/vision/**`, `docs/research/doc_audit/**`. Deleted build directories (`_m3_target`, `_m3_target_base`, `_ttl_target`, `_m7_target_*`, `CargoTarget\m7_clean`, `phantom-cache\target`) are ignored by abs-path with reason "deleted 2026-10-30 (storage cleanup record)".

## Rules

| Rule | Level | Applies to | Check |
|---|---|---|---|
| link-broken | error | current, historical, archived | relative markdown link target missing (code spans and fences skipped) |
| anchor-broken | error | same | `file.md#anchor` with no matching heading (GitHub slug rules; `#L12` ignored) |
| abs-path | error | current | `file:///` or `X:\` drive path, except ignore patterns |
| secret-adjacent | error | all modes | drive path or 64+ hex on a line with salt/secret/key/password words. Output is file, line, rule only; the matched text is never printed or stored |
| status-missing / status-vocab / status-case | error / error / warn | current | `Status:` in first 40 lines, first word exactly Draft/Proposed/Active/Verified/Superseded/Archived; case variants warn |
| reviewed-missing / reviewed-format | error | current | `Last reviewed:` ISO date in first 40 lines |
| h1-missing | error | current | no H1 outside fences |
| filename-convention | warn | current | per-directory convention from config |
| dup-basename | warn | current | same basename in several directories (README.md exempt) |
| orphan | warn | current | not linked (markdown link or backticked repo path) from an index file within 1 hop; index = README.md, docs/STATUS.md, docs/INDEX.md, handoff 00 |
| size | warn | not frozen | over 60 KB |
| path-ref-missing | warn | current | backticked repo path (known prefix) that does not exist |
| fact-drift | warn | current | test-count style numbers differing from registry (851 passed / 0 failed / 8 ignored / 75 targets); line marker `<!-- fact:test_baseline -->` exempts |
| sec-status-spread, claim-status-spread | info | current | ids with several status words across files; never an error |
| fact-source-missing / mismatch | info / warn | registry | `docs/STATUS.md` missing, or lacks the canonical number |

## Baseline counts (942 files)

Totals: **520 error, 241 warn, 11 info**.

| Rule | Level | Count | Top offenders |
|---|---|---:|---|
| secret-adjacent | error | 185 (140 under `.local/` and other frozen; 45 elsewhere) | progress_ledger.md 16, security/phase-4-design.md 5, MILESTONE_1_DESIGN_PLAN.md 3, demo-lab/README.md 3 |
| reviewed-missing | error | 112 | essentially every current doc (field not yet adopted) |
| abs-path | error | 97 (69 file:///, 28 drive paths; lines, not occurrences) | security/trust-model-audit.md 18, MILESTONE_1_DESIGN_PLAN.md 15, demo-lab/capability-audit.md 9, demo-lab/README.md 8, master_audit_and_execution_plan.md 7 |
| status-missing | error | 78 | CLAUDE.md, CORRECTIONS.md, README.md, ... |
| status-vocab | error | 28 | demo-lab/implementation-roadmap.md, demo/lab_runner.md, design/lab_api_contract.md, ... |
| link-broken | error | 20 | docs/README.md 12 (new, targets not yet created), architecture_trace_m3.md 7 (wrong `../src` depth), phase_2_completion_report.md 1 |
| anchor-broken | error | 0 | |
| orphan | warn | 73 | CORRECTIONS.md, MILESTONE_1_DESIGN_PLAN.md, many demo-lab and security docs |
| fact-drift | warn | 64 | master_audit_and_execution_plan.md 16, accountability report 9, g1_verdict.md 4, work_packages.md 4 |
| notice-missing | warn | 52 | all of docs/handoff/** (no `Notice (` or `Historical` line) |
| path-ref-missing | warn | 29 | security-demo-matrix.md 8, security-scenario-catalogue.md 5, implementation/WP2.md 3 |
| size | warn | 8 | PHASE_2_WORK_REPORT, accountability report, phase_1_execution_package, progress_ledger, ... |
| status-case | warn | 6 | ADR-001, phantom_intelligence_phase, mvp_and_milestones, ... |
| filename-convention | warn | 5 | scenario_B_origin_slowdown.md, ag_task_WP-00.md, ..._DRAFT.md, project_status_and_accountability_2026-10-30.md |
| dup-basename | warn | 4 | implementation-roadmap.md (demo-lab vs security), test_plan.md (m3_durability vs ttl_decision) |
| sec-status-spread | info | 7 ids | SEC-M7-01 (CLOSED/FIXED/OPEN/PARTIAL, 4 files), SEC-M7-09 (6 files), SEC-M7-10, -11, -02, -03, -04 |
| claim-status-spread | info | 4 ids | C1, C2, C7, C12 |

Secret-adjacent findings need human triage: the script cannot tell a path or hash near the word "key" from a real secret. The earlier D4 pass saw only paths, not values; this run likewise reports only locations. No hex string or value appears in the script output (checked: no 40+ hex run in the JSON or text output).

## False positives and noise

- secret-adjacent: "key" is a common word (cache key, key-value); a drive path in a sentence about keys fires. Frozen snapshot copies in `.local/` repeat the same lines (140 of 185).
- abs-path counts lines, not occurrences; the ignore list matches by substring, so any path containing `_m7_target` is ignored.
- fact-drift is a number heuristic ("242 tests", "774/774"); it also catches historical statements in current docs and unrelated figures such as per-phase counts. Level is warn for that reason. It cannot confirm the canonical value is right until `docs/STATUS.md` exists.
- claim-status-spread labels show the table row prefix (e.g. `| **C12`); counts are upper bounds.
- orphan uses a 1-hop link graph from index files; documents reachable only via two hops are reported.
- link-broken on `docs/README.md` reflects files other writers had not yet created.
- notice-missing is expected for handoff files until W3 notices land.
- reviewed-missing and status-missing are expected everywhere until headers are adopted; use `--new-only` with a baseline to ratchet.

## What the script cannot check

- Whether a document's content is true, current, or consistent with code; whether a "Verified" status has the required independent check.
- Non-markdown files (HTML, JSON, patches, source comments) and links inside fenced code or inline code.
- External URLs (never fetched); reference-style links; links with spaces or brace-style regex text (skipped as not paths).
- Duplicate content (only basenames), terminology drift, date formats other than `Last reviewed`, heading-level skips, multiple H1s.
- Whether a missing backticked path was moved, planned or deleted (no git history for most files).
- Secrets in any form other than a drive path or 64+ hex near the configured words (no entropy scan).
- Case variants of status words outside the `Status:` header line.

## Verification of no modification

Before and after the runs: `git status --short` line count 75 both times with identical content (diff empty), and SHA-256 of a 20-file sample of docs (every 8th markdown file under `docs/` plus README.md and CLAUDE.md) unchanged. HEAD 456bb25, branch feat/publish-phantomcache-core. Concurrent writers were active, so the sample was chosen to avoid this audit directory; unchanged hashes show the script did not write, not that nobody else did.
