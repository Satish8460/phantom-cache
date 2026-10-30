# G1 verdict - Scenario B close-out (2026-10-30 07:35 +05:30)

**Verdict: PASS (with five formally recorded dispositions).** G1's purpose - the Scenario B results are verified, interpretable and honestly bounded, the freeze evidence chain is intact, and the current source is green - is met. The dispositions below are explicit, evidence-backed and owned; none changes a result. G2 is opened under the file-ownership constraints in section 3.

## 1. Criteria
| Requirement | Result | Evidence | Reviewer |
|---|---|---|---|
| Raw data independently recalculated | MET | separate Node implementation, spec-first: 30/30 runs match run_metrics.json, 161/168 summary values match, 7 differences explained (recovery tolerance definition, CSV rounding); lead's own earlier CSV check (success, peaks) | independent recomputer agent + lead; `docs/demo/scenario_b_verification/` |
| Verdict independently reproduced | MET | TRADE_OFF_FAILURE for both arms re-derived from the contract text | recomputer |
| P99 discrepancy resolved | MET | 2.6% and 4.4 ms are the same quantity (mean paired difference -4.42 ms / mean BASE P99 167.09 ms = 2.645%); all-row P99 mostly a 503 artifact (200-only -2.78 ms, 1.66%) | recomputer |
| Arm fairness assessed | MET with qualification | independent review: valid-with-qualification; canary-vs-off excluded by code reading only | fairness reviewer |
| Pilot selection / tuning leakage disclosed | MET | pilot `D:\Temp\sb_pilot1` documented; not pre-registered; wording in `docs/demo/scenario_b_interpretation_notice.md` | reviewer + lead |
| Origin-cap enforcement verified | MET (STATIC full; AIMD limited by 265 ms snapshot sampling) | permit lifecycle by code; origin peak exactly 24 in all STATIC runs; 503 count == rejection counter (+-1) | reviewer + recomputer |
| Provenance labels correct | PARTIAL -> disposition D1 | label meaning corrected in the dated notice; code strings/summary text unchanged | reviewer |
| Freeze amendment completed | MET | addendum 1 (+pin `bf2435dc...5a33`); fail-closed verifier, 24-case matrix; sealed artifacts and originals unchanged by hash | R-01 writer + lead hashing |
| Current-source tests / Clippy | MET (fmt: disposition D4) | debug sweep exit 0: 802 passed, 0 failed, 7 ignored (tree hash `7607bc18...49f1`); clippy exit 0; targeted release (10 suites) exit 0: 209 passed, 0 failed, 6 ignored, 07:29:44-07:34:42 | lead; logs `D:\Temp\q1_*.log` |
| Independent review findings classified | MET | see ledger entry (no Critical; 3 High qualifications, 5 Medium, 3 Low) | reviewer |
| Valid findings fixed and verified | PARTIAL -> dispositions D1-D3 | wording/interpretation fixed by notice; code-level fixes scheduled | lead |

## 2. Dispositions (explicit, evidence-backed)
- **D1 (labels/wording in code):** `VALID` meaning, AIMD effective-cap label, canary-vs-off sentence, all-request-latency and origin-p99 caveats, pilot disclosure, in `src/lab_live` strings and summary text. Retained artifacts cannot be edited; the dated interpretation notice is binding. Fix at the next `src/lab_live` edit (G2 writer, first task), with a test pinning the strings. Owner: G2 writer. Acceptance: strings present, tests pass.
- **D2 (503 body text for origin-cap rejections):** `OriginError::CapacityExceeded` is reused and says "Cache shard capacity saturated". Valid product defect (misleading text; classification of results unaffected because X-Cache-Error is `CapacityExceeded` in both cases and counts match). Fix in `server.rs` (not frozen) with a failing-first test (distinct body or error code for origin-cap saturation, `Retry-After` unchanged). Owner: G2 writer. Acceptance: test fails before and passes after.
- **D3 (unverifiable post-run edit of `scenario_b.rs`; driver exe hash missing from manifest):** no pre-edit copy exists; the executed code is attributable to the driver exe on disk (mtime predates the edit), whose sha256 is recorded here: `phantom-lab.exe` `1c66135026f98aa99fa456f7001699c0cdd3c8edf253ccebfef8f3f5a0beb0d1`; gateway exe `3e3451b7...2574` matches the manifest. The "clippy-only" claim stays UNVERIFIED. Any claim that needs exact source attribution requires a fresh reproduction; none of the permitted conclusions does.
- **D4 (`cargo fmt --check` fails; 38,042 diff lines):** pre-existing repository state; formatting would rewrite frozen M6 files. Handle only with freeze addendum 2 in a dedicated formatting commit near G5 (P2).
- **D5 (stale-while-revalidate variant, origin-p99 clause, load-feedback loop not exercised):** capabilities remain UNVERIFIED by Scenario B; covered by the future E1 plant and a separate pre-fixed experiment. Nothing may be inferred from Scenario B.

## 3. G2 opening constraints (file ownership)
G2 may begin now. The G2 writer must NOT edit `Cargo.toml`, `src/lib.rs`, `src/lab/**`, `src/core/**`, `src/intelligence/**`, `tests/support/**`, `tests/integration/m6_research_evaluation_tests.rs` (all in the freeze inventory; any change would again fail the freeze test until an addendum 2). New code goes in `src/api/lab_api.rs` declared inside `src/api/server.rs` with `#[path]` (the `api` module is inline in `lib.rs`), tests in an existing registered test target via `#[path]` modules, UI in `ui/`. If a frozen file must change, stop and report.

## 4. What G1 does not claim
No adaptive-control merit, no production/multi-host behaviour, no origin-p99 benefit, no stale-serving verification, no pre-registration of cap values (see the interpretation notice). The M6 sealed verdict is untouched.
