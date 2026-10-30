# Known limitations recorded for frozen code (src/lab, src/intelligence)

Status: Recorded. Owner-decided 2026-10-30. Fixes deferred. No addendum 2 exists. No frozen file was modified by this record.
Audience: project owner, human reviewers, external reviewers.
Last reviewed: 2026-10-30 15:42 IST (machine clock).

Source of findings: `docs/release/g4/cycle1_lab_intelligence_deps_review.md` ("G4C"), an AI second look. It is NOT independent human review and NOT external review. All reviews cited here are labelled AI second look.

Evidence tags: verified in code = G4C read the file at the cited line (reported by G4C; line numbers not re-checked by this record's author unless stated). Reproduced here = re-run by the author of this record on 2026-10-30.

## 1. Plain statements about the sealed result

- The sealed simulation result (H1 NOT_SUPPORTED) has NOT been independently reviewed by a human or an external reviewer.
- G4C is an AI second look. It did not read the simulation core. Files G4C did NOT read (G4C section 0): `src/lab/origin_sim.rs`, `src/lab/workloads.rs`, `src/lab/grid_tuning.rs`, `src/lab/tuning.rs`, `src/lab/eviction_baselines.rs`, `src/lab/events.rs`, `src/lab/clock.rs`, the rest of `src/lab/t1_sim.rs` (the event loop; only cell aggregation, percentile, CSV, defect cell and imports were read), `src/intel/diagnostics.rs`, `src/intel/origin_metrics.rs`, most of `src/config.rs`, most tests. G4C also read only part of `src/intelligence/shadow.rs` (lines 1-120 and 330-484; the SIEVE and S3-FIFO shadows were not read). Absence of a finding in those files is absence of review, not evidence of correctness.
- The sealed result is a forced-adapt comparison on a simulator (tier T1), per G4C F33. It says nothing about a real deployment.

## 2. Freeze hash differences (reproduced here, 2026-10-30)

Method: `Get-FileHash` SHA-256 of each path in `files_sha256` of `docs/research/preregistration_m6_v2_2_freeze.json`, compared with the recorded value; `git diff --no-index` against the pre-change copies in `D:\Temp`. No hash value is printed in this document.

Result: 56 entries in `files_sha256`; 0 missing; 4 differ; 52 match. This agrees with G4C section 2.

| File | How it differs (reproduced here) | Addendum 1 `changed_files` entry |
|---|---|---|
| `Cargo.toml` | +4 lines, 0 removed: one blank line and one `[[test]]` stanza (`lab_live_scenario_b_tests`, path `tests/integration/lab_live_scenario_b_tests.rs`) after line 349. No dependency or profile change. | yes; category "additive stanza" |
| `src/lib.rs` | +2 lines, 0 removed: one blank line and `pub mod lab_live;` after line 51. | yes; category "additive mod line" |
| `src/lab/eval.rs` | +108 / -2. The two removed lines are `use std::collections::BTreeMap;` and `return Err(format!("{rel} changed after freeze"));` (the latter inside the refactored loop). Additions: `verify_freeze_core` (refactor, "same checks" per addendum), `verify_freeze_with_addendum`, addendum paths. Not purely additive: it is a verifier refactor, so it needs human judgement. | yes; category "other: verifier (R-01)" |
| `tests/integration/m6_research_evaluation_tests.rs` | +128 / -3. Real-tree test now uses addendum-aware verification; synthetic addendum tests added. Diff content of the 3 removed lines was not itemised here. | yes; category "other: tests (R-01)" |

Relationship to addendum 1 (`docs/research/preregistration_m6_v2_2_addendum_1.json`, `.md`, `.pin`):
- `changed_files` has exactly 4 entries; the set equals the set of differing files (reproduced here).
- For all 4: `frozen_sha256` equals the freeze record's value and `current_sha256` equals the current tree file (reproduced here, 4/4).
- The `.pin` file equals the SHA-256 of the addendum JSON (reproduced here). G4C also reports the addendum's `freeze_record_sha256`, `amendment_sha256`, `addendum_md_sha256` and the sealed `cells.csv` hash all match (reported by G4C; not re-run here).
- The 4 pre-change copies named in the addendum exist outside the repo and each hashes to the frozen value (reproduced here): `D:/Temp/sec3_scratch/Cargo.toml`, `D:/Temp/sec3_scratch/src/lib.rs`, `D:/Temp/r01_eval.rs.pre`, `D:/Temp/r01_m6tests.rs.pre`.
- The addendum states `src/lab_live/**` is outside the frozen scope and is not referenced from `src/lab`, `src/core`, `src/intelligence` or `tests/support` (addendum `lab_live_statement`; G4C section 2 notes `lab_live`, `src/bin/phantom-lab.rs`, `src/intel`, `src/control`, `src/config.rs`, `src/api`, `src/main.rs` are not in the freeze inventory at all).

Caveats:
1. The addendum was written AFTER the sealed run (addendum `written_utc` 2026-10-30 01:45; run marker 2026-10-29 23:48:45 UTC, per G4C).
2. The verifier `src/lab/eval.rs` changed after the sealed run. The diff reads as behaviour-preserving for the original checks, but this is a human acceptance decision (G4C section 6 item iv).
3. The frozen versions of the 4 files are reconstructable only from the non-repo `D:\Temp` copies. HEAD (`3e6e03e`) is an old snapshot and the work is uncommitted, so git cannot reproduce them (G4C F27). `D:\Temp` is non-durable.
4. The verifier pins the exact `rustc --version` string, so a toolchain update makes `verify_freeze` fail closed (G4C F27).
5. Addendum mechanism is transparency, not tamper-proofing (G4C F20).

Line-ending note (not a decision): `core.autocrlf=true` and no `.gitattributes` exist (reproduced here). `git diff` printed "LF will be replaced by CRLF" warnings for both the working files and the `D:\Temp` copies. Hash comparisons here are on the raw bytes on disk and match the record, so no line-ending drift affected the 52 matches; but a checkout, normalisation or commit on another machine could change bytes and break `verify_freeze`. No action is taken or proposed by this record.

## 3. Known limitations (frozen areas: src/lab/**, src/intelligence/**)

Disposition applied to every row unless stated: Recorded as known limitation; owner-decided 2026-10-30; fix deferred. Any code change to these files requires a pinned addendum 2 (JSON plus pin, written by a human decision) before editing, otherwise `verify_freeze` fails. G4C's recommendation (section 6 item 5) was to record items not exercised by the sealed pipeline and to flag F03 and F16 as limitations of the verdict interpretation. Whether an item influenced the sealed result is G4C's statement unless noted; the simulation core was not read, so "not exercised" is not independently established.

Impact key: SEALED = could affect interpretation of the sealed pipeline output; SERVING = affects the gateway serving path; LATENT = code has no caller in serving, lab or pipeline use today (per G4C grep).

| ID | Location | Evidence (G4C) | Severity | Impact | Additional disposition notes |
|---|---|---|---|---|---|
| F03 | `src/lab/m6_pipeline.rs` `compute_verdict` / `run_pipeline`; `src/lab/t1_sim.rs:1327-1366` | Verified in code. A panicked cell becomes a defect row (`p99_us=0`, `peak_outstanding=1`, `invariant_violations=1`); `completeness_problems` is listed but does not set an INVALID H1 verdict (except seed pairing). | Medium, CONFIRMED | SEALED (interpretation of `verdict.json` read standalone). Mitigation per G4C: the sealed test asserts `out.problems.is_empty()` after recording, and the recorded verdict is NOT_SUPPORTED, which this path cannot push that way. | Needed to fix: set H1 to an INVALID value when problems non-empty, plus a test; addendum 2. Treat as limitation of verdict interpretation. |
| F04 | `src/lab/manifest.rs:103-104`, `compute_manifest_hash`, `evaluate_validity` | Verified in code. `RunManifestV2` hard-codes git hash and `is_dirty`, uses `HashMap` (non-deterministic digest), checks 3 of 14 validity items. | Medium, CONFIRMED | Latent. Only used in `tests/integration/lab_core_tests.rs:196`; real runs use the JSON manifest in `eval.rs`. | Do not cite `RunManifestV2` as run provenance. |
| F05 | `src/lab/matrix_runner.rs:78-100` | Verified in code. Positional, tie-unstable concordance, `partial_cmp().unwrap()` can panic on NaN; T2 cells in its test are synthetic fixtures by the test's own comment. | Medium, CONFIRMED | Latent for the sealed pipeline. Any claim "T1 predicts T2" from this code is unsupported. | Do not make that claim. |
| F06 (lab part) | `src/lab/analysis.rs:329-331` | Verified in code. Constant data gives a zero-width BCa CI `(mean, mean)`. Scenario B consequence is in `src/lab_live` (see section 4). Not observed in retained data (assumption). | Medium, PLAUSIBLE | SEALED only if a sealed cell set is degenerate; not checked. | A human should check for zero-variance paired inputs in the sealed analysis. |
| F09 | `src/intelligence/executor.rs:36-45, 191, 362-370`; `src/main.rs:468` | Verified in code. Safe baseline hard-codes a 24 h stale cap and `Sieve`; clamp uses constant `MAX_HARD_STALE_MS`, not `PHANTOM_MAX_STALE_MS`. | Medium, CONFIRMED | LATENT today: `apply_policy`, `safe_apply_policy`, `revert_to_safe`, `check_watchdog`, `route_bucket` have no caller outside `src/intelligence`/`src/lab` (G4C grep). Becomes SERVING and violates the CLAUDE.md stale-cap invariant if any is wired. | Do not wire these callers before the fix. |
| F10 | `src/intelligence/candidates.rs:121-122` | Verified in code. `.min(cap).max(base*10)` order exceeds the 24 h cap for `base_stale_ms > 2.4 h`; test uses base 60 s only. | Medium, CONFIRMED | Lab/library only; T1 cache clamp may mask it (G4C). | |
| F11 | `src/intelligence/candidates.rs:148-149`; `src/config.rs:163-165` | Verified in code. Hard-coded 256 clamp; configured values above it silently ignored; `min > 256` exceeds the "hard cap". | Medium, CONFIRMED | SERVING for the AIMD controller (it is the one candidate used by `main.rs`/`server.rs`, per G4C section 5) when the cap is configured above 256. | Operators should keep `PHANTOM_ORIGIN_CAP_MAX <= 256` and `MIN <= 256` until fixed. |
| F12 | `src/intelligence/engine.rs:94-109, 243` | Verified in code. Non-quantile P99 tracker; guardrail compares raw error counts; holdback unused; CI treats key-partitioned arms as independent. | Medium, CONFIRMED | LATENT (not wired into serving). Do not activate on live traffic (G4C). | |
| F13 | `src/intelligence/shadow.rs:452-465` | Verified in code (shadow.rs read in part). CI assumes independent arms and requests; no multiplicity control; coverage unstated. | Medium, CONFIRMED | LATENT (not wired). Reported 95% interval has no stated coverage. | |
| F14 | `src/intelligence/mod.rs` docs; `src/intelligence/executor.rs:~226` | Verified in code. `PHANTOM_INTELLIGENCE=canary` applies the origin cap to 100% of fetches; the 5% routing, holdback and rollback have no serving caller. | Medium, CONFIRMED | SERVING (mode semantics). The mode name and docs promise a bounded, auto-reverting canary that does not exist. Scenario B artifacts already state canary vs off is not a clean A/B. | Treat the mode as "cap on". |
| F15 | `src/intelligence/executor.rs:101-108`; `src/api/server.rs:2410-2420` (server.rs is outside the frozen scope) | Verified in code; consequence is a hypothesis, not tested. | Medium, PLAUSIBLE | SERVING when the AIMD cap is active: cancelled or timed-out fetches give no latency sample. | Untested hypothesis. |
| F16 | `src/lab/eval.rs` `analyze_endpoints`; `src/lab/t1_sim.rs:~1290`; `src/lab/runner.rs::paired_summary` | Verified in code. A zero control maps to 0% reduction; `peak_outstanding` floored at 1; per-seed means can span different family sets; only `n_seeds` equality is checked. G4C notes `completeness_problems` bounds this for the registered pipeline. | Medium, CONFIRMED | SEALED (verdict interpretation, per G4C). The ad-hoc `phantom-lab run` path has no row-count check. `eval.rs` is also one of the 4 files changed by addendum 1. | Treat as limitation of verdict interpretation. |
| F18 | `src/lab/eval.rs:53-70` | Verified in code. `git_output` returns empty/0 on failure; manifest then reads as clean. | Low, CONFIRMED | SEALED (provenance fields in manifests could read as clean on failure). | Human should confirm the sealed manifest's git fields are real. |
| F19 | `src/lab/eval.rs:703-740` | Verified in code. Sealed-seed leak scan is decimal-substring only. | Low, CONFIRMED | SEALED (defence in depth only, not proof). | |
| F20 | `src/lab/eval.rs:644-700` (`verify_freeze_with_addendum`), comment at `:612-617` | Verified in code. Addendum accepts any change when a matching JSON and pin are written; no additivity check; no git commit binding. | Low, CONFIRMED | SEALED (what the freeze check does and does not prove). | Any future addendum is a human decision. |
| F21 | `src/lab/rng.rs:118-123` | Verified in code. `hex_decode` can panic on non-ASCII input of even byte length. | Low, CONFIRMED | Only in a process already holding the sealed salt variable. | |
| F22 | `src/lab/runner.rs:132-190` | Verified in code. No de-duplication of lists; `W-Z(NaN/inf/-1)` panic inside `AliasSampler::new` (caught as defect cell); arbitrary `out_dir`; `--overwrite` rewrites `cells.csv`. | Low, CONFIRMED | CLI only. | |
| F23 | `src/intelligence/signals.rs` (about :190, :272, :290, :440-475, :502) | Verified in code. Cumulative signals tagged with a window; F10 is client latency labelled origin p99 and `Measured`; OLS fit labelled MLE; 1 KB per object assumed. | Low, CONFIRMED | LATENT (not wired). Violates the CLAUDE.md labelling rule if ever displayed. | |
| F24 (frozen part) | `src/lab/t1_sim.rs:1074` (nearest-rank), `src/intelligence/executor.rs:~130` (nearest-rank), `src/intelligence/signals.rs:502` (floor) | Verified in code. Inconsistent percentile definitions; floor form returns the maximum as p99 for n=100. G4C checked the float form against integer arithmetic for p in {50,90,95,99}, n=1..5000 and found no mismatch. | Low, CONFIRMED | LATENT/definition only. The sealed pipeline's `t1_sim.rs` uses nearest-rank. Non-frozen occurrences (`telemetry.rs`, `loadgen.rs`) are in section 4. | |
| F25 | `src/intelligence/candidates.rs:175-207, 234-239` | Verified in code. Unbounded vectors; MASE denominator off by one; test asserts only `0 < mase < 2`. | Low, CONFIRMED | LATENT. | |
| F26 | `src/intelligence/executor.rs:287, ~357`; `src/intelligence/audit.rs` | Verified in code. Watchdog never trips before first heartbeat; `SystemTime`; unknown ids map to SIEVE; NaN casts to 0; audit store bounds record count but not bytes. | Low, CONFIRMED | LATENT (watchdog); the audit store is in use (`AuditStore::new(100)` in `ApiState`) with a count bound only. | |
| F27 | Freeze evidence (see section 2) | Reproduced here and by G4C. | Low, CONFIRMED | SEALED (reproducibility of the freeze evidence). | Needed: archive the four pre-change copies under `docs/research/` or a tagged commit. Not done. |
| F32 | `src/lab/runner.rs:132-170`, `src/lab/rng.rs:140-170`, `src/lab/eval.rs:123, 902-917` | Info. Description of the sealed workflow guards. `forbidden_sealed_set()` has real sealed seeds only when the salt is in the process environment, so `verify_no_sealed_leak` is vacuous in normal runs. | Info, CONFIRMED | SEALED (what the guards prove). | |
| F33 | Freeze record (tuning), `src/lab/eval.rs analyze_endpoints`, `src/lab/analysis.rs` | Info. Registered arm `tuning_v2_2c` was chosen after seeing tuning data; separate Holm families; shared bootstrap seed across hypotheses. | Info, CONFIRMED | SEALED (researcher degrees of freedom). | |

Counts for this table: 24 rows (F06 and F24 appear as split rows). By G4C severity: Medium 12 (F03, F04, F05, F06, F09, F10, F11, F12, F13, F14, F15, F16), Low 10 (F18, F19, F20, F21, F22, F23, F24, F25, F26, F27), Info 2 (F32, F33).

## 4. Not covered by this record (non-frozen areas; tracked elsewhere)

Their own tracking is the G4C report (`docs/release/g4/cycle1_lab_intelligence_deps_review.md`) and the G4 blocking-items list in its section 6. This record makes no disposition for them.

| ID | Location | Severity (G4C) | Note |
|---|---|---|---|
| F01 | `src/lab_live/scenario_b.rs:658`, `src/lab_live/analysis.rs:131-134`, `ui/test/lab.view.test.mjs:282-290` | High, CONFIRMED | G4C blocking item 1; no addendum needed |
| F02 | `src/lab_live/scenario_b.rs:1261, 710` | Medium, CONFIRMED | G4C blocking item 2 |
| F06 (Scenario B part) | `src/lab_live/analysis.rs:146-185` | Medium, PLAUSIBLE | Lab part of F06 is in section 3 |
| F07 | `src/lab_live/cli.rs:72-100`, `loadgen.rs:40-62` | Medium, CONFIRMED | G4C blocking item 3 |
| F08 | `src/lab_live/gateway.rs:69` | Medium, CONFIRMED / impact PLAUSIBLE | G4C blocking item 3 |
| F17 | `src/lab_live/scenario_b.rs:724, 1168, 1179` | Low, CONFIRMED | |
| F24 (non-frozen part) | `src/intel/telemetry.rs:512-515, 558-603`, `src/lab_live/loadgen.rs:163-171` | Low, CONFIRMED | |
| F28 | `README.md:108`, `Cargo.toml` (no `rust-version`) | Low, CONFIRMED | Fix in README only; do not edit `Cargo.toml` under the freeze |
| F29 | `src/lab_live/http.rs:91, 99` | Low, PLAUSIBLE | |
| F30 | `src/lab_live/scenario_b.rs:690` | Low, PLAUSIBLE | |
| F31 | `src/lab_live/scenario_b.rs:326-336` | Low, PLAUSIBLE | |
| F34 | float percentile hypothesis | Info, REFUTED | Recorded so it is not re-raised |
| Dependency inventory | `Cargo.lock` | Info | 288 locked packages; yank/vulnerability status NOT determined (no `cargo audit` run) |

## 5. What a human or external reviewer must still examine

1. The simulation core that produced the sealed result and that G4C did not read: the `src/lab/t1_sim.rs` event loop, `origin_sim.rs`, `workloads.rs`, `grid_tuning.rs`, `tuning.rs`, `eviction_baselines.rs`, `events.rs`, `clock.rs`.
2. Independent re-derivation of the BCa and permutation results on a reference implementation (for example R `boot` or SciPy `bootstrap`) for several cells; check for zero-variance inputs (F06).
3. Whether editing `src/lab/eval.rs` after the sealed run (addendum 1) is procedurally acceptable, and the line-by-line content of the 3 removed lines in the tests file.
4. Acceptance of the sealed-salt custody claims in the freeze record's `limitations` (readable by same-user processes; generated by an agent process).
5. Whether the sealed manifest's git fields are real (F18) and whether any defect cell occurred in the sealed run (F03; G4C reports the sealed test asserts no problems).
6. `cargo audit` or `cargo deny` and yank status for the locked dependencies.
7. Real-host, release-profile reruns of Scenario B (retained runs are loopback, one host).
8. Run-level checks of the PLAUSIBLE hypotheses (F06, F15, F29-F31).
9. A durable archive of the four pre-change file copies (F27).
10. Any decision to write addendum 2 and fix the rows in section 3.

## Owner confirmation (2026-10-30)
DEC-009: the owner confirmed F16, F18, F19, F20, F21, F22, F23, F25, F26 and F32 as recorded known limitations under ODR-1 (frozen scope only; not fixes; no addendum 2). See docs/release/g4/cycle1_findings_disposition_register.md.
