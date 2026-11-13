# M6 protocol v2.2, freeze addendum 1 (R-01)

Written 2026-10-30 (UTC 01:36 to about 01:46; local +05:30 07:06 to about 07:16) by the R-01 implementing agent at the request of the research lead.
This document does not change the protocol, the hypotheses, the analysis, the seeds, the arms or the sealed result. It documents changes made to frozen files AFTER the sealed run and tells the verifier exactly which differences are accepted.

## 1. What this addendum is anchored to (unchanged documents)

| Document | SHA-256 (verified unchanged 2026-10-30T01:36:42Z) |
|---|---|
| `docs/research/preregistration_m6_v2_2_freeze.json` (frozen 2026-10-29T23:36:24Z = 2026-10-30 05:06:24 +05:30) | `65cf88db98f17c3c149e5569a876f9cfba8408731b02643641caa3f370a4ee57` |
| `docs/research/preregistration_m6_v2_2_amendment.md` | `10d31c2e36de50802922b778b81c5aa6054a8d53e31e3ce883bd29f06d4ea443` |
| `docs/research/preregistration_m6_v2.md` | `492ab03a1b364fae762bc010c3d9500cc2ab8c4f65f9e3e576f3f2dc6f94eb3c` |
| `docs/research/preregistration_m6_v2_1_amendment.md` | `dd2191d8f29061cc2ae2ac5f867545d7df94b2cc68ee97dac65da5384a8c9054` |
| `docs/research/preregistration_m6_v2_1_freeze.json` | `1dbc8aad9943a843dc9edea90b2ad4a4130a94f2f73d5fdada5759432f92c8a1` |
| `docs/research/preregistration_m6_v2_freeze.json` | `66457088363df8ca579cef02fb869b19666c41e7ce3fa3c59cb8b32d71ae07ae` |

All six equal the hashes pinned in the v2.2 freeze record (reproduced here with `sha256sum`).

## 2. Sealed run (references only, nothing re-derived)

- Run-once marker `lab-results/m6_sealed_run_started.marker`: `utc=2026-10-29T23:48:45Z`, `freeze_record_sha256=65cf88db...ee57`, `protocol_version=2.2`.
- Artifacts: `lab-results/m6_sealed_2026-10-29T234845Z/` (manifest: build_profile release, started 2026-10-29T23:48:45Z, ended 2026-10-29T23:50:22Z, 12,000 cells).
- `cells.csv` SHA-256 `3073a04431837a200b0cf962bb56c87b0e7554b4b8a9f6f818d556ec8525ecb5` equals `output_files_sha256["cells.csv"]` in that manifest (reproduced 2026-10-30).
- Verdict as written in `verdict.json` (SIMULATED, T1 virtual time): H1 `NOT_SUPPORTED` (client-p99 supported in 2 scenarios, peak-concurrency in 0); H2 and H4 `NOT_TESTABLE_AT_REGISTERED_HORIZON`; H3, H5, H6 `UNVERIFIED`. This addendum does not reinterpret it.
- The sealed manifest's own `source_sha256` records `Cargo.toml = eec40383...` and `src/lib.rs = 25361ff3...`, i.e. exactly the frozen (pre-change) versions. The sealed run was therefore made with the frozen inventory.
- The sealed test remains run-once and spent. This addendum does not re-enable it.

## 3. Frozen files that changed after the freeze (complete list)

All 56 files of `files_sha256` were re-hashed on 2026-10-30T01:36:42Z. Exactly two differed (before the R-01 code edits). After the R-01 edits two more differ (`src/lab/eval.rs`, `tests/integration/m6_research_evaluation_tests.rs`). The full list is in the JSON; summary:

| Path | Frozen SHA-256 | Current SHA-256 | Category | Who / why |
|---|---|---|---|---|
| `Cargo.toml` | `eec40383ff1fdb25a9de9edfabe61aaaa9e555d6812ee86825b4726a7d9fe9e2` | `bea0e5c079040ce443cccaf7b5bc1057e248cd643849f32a21dfcd4844a788b5` | additive stanza | Scenario B work: added `[[test]] name = "lab_live_scenario_b_tests"` (4 lines incl. blank). No dependency, feature, profile or other stanza changed. |
| `src/lib.rs` | `25361ff30d51baa8f1e13b8afa5696046c0eb5fa9e2a0426b3b2f5be1ee6bf57` | `6ca352b64511c851a1c9d90934e627ea476f41ce4cb43d9f730b1f00ad50cf21` | additive mod line | Scenario B work: added `pub mod lab_live;` (plus one blank line). |
| `src/lab/eval.rs` | `f56da990096673e445e8d180fa0580b3d0d8447f159f1f787a318eee80e4b466` | `cd619534feffa5286e037c40e41375127c0ddaf47434baf152907431e213c327` | other: verifier change (R-01) | `verify_freeze` body moved into private `verify_freeze_core` (identical checks, empty tolerance map, so `verify_freeze` behaves as before); added `verify_freeze_with_addendum`, three path helpers, a forbidden-path predicate, `BTreeSet` import. No simulation, analysis or evaluation code touched. |
| `tests/integration/m6_research_evaluation_tests.rs` | `2aec7dadb770ef1e0428758f5b6b602616d159d873d28483cfbc02b6e000243e` | `dee62a1afe7cce6188c10de1e1e4b7010fba1b7abe841139ce7d38b445ac09f3` | other: tests (R-01) | `test_real_v2_2_freeze_record_verifies_when_present` now calls `verify_freeze_with_addendum`; two new synthetic-root test functions (valid path; 24 fail-closed cases). |

No lint-only edits to frozen files were found: all other 52 frozen files (including every file under `src/lab`, `src/core`, `src/intelligence`, `tests/support`) are byte-identical to the freeze.

### Pre-change copies (can be diffed)

- `Cargo.toml`: `D:\Temp\sec3_scratch\Cargo.toml` hashes to `eec40383...` (equals the frozen hash). `diff` against the current file shows exactly one added hunk, the `lab_live_scenario_b_tests` test stanza (lines 350 to 353).
- `src/lib.rs`: `D:\Temp\sec3_scratch\src\lib.rs` (and several other scratch copies under `D:\Temp\cf*_old`, `cpl_scratch`, `sec2_scratch`) hash to `25361ff3...`. `diff` shows exactly one added hunk: a blank line and `pub mod lab_live;`.
- `src/lab/eval.rs` and the m6 test file: copies taken immediately before the R-01 edits, `D:\Temp\r01_eval.rs.pre` (`f56da990...`) and `D:\Temp\r01_m6tests.rs.pre` (`2aec7dad...`), both equal to the frozen hashes.
- These copies live in a temp directory and are not part of the repository. They are provable only by hash, which is what this addendum relies on; if they are deleted the change is characterised by inspection of the current content only.

## 4. Why `src/lab_live/**` cannot affect T1 results

- `src/lab_live/**` is NOT in the frozen scope (`tracked_source_files` covers `src/lab`, `tests/support/lab`, `src/core`, `src/intelligence` and the listed build inputs only), and was not hashed in the freeze.
- Verified by search (2026-10-30): no file in `src/lab`, `src/core`, `src/intelligence` or `tests/support` mentions `lab_live`. The only references are `src/lib.rs` (the `pub mod` line), `src/bin/phantom-lab.rs`, `src/lab_live/scenario_b.rs` and the new Scenario B test.
- `lab_live` imports only public items of `crate::lab` (`analysis::StatisticalEvaluator`, `eval::{git_dirty_entries, git_head, repo_root, sha256_file, utc_now_iso}`, `rng::{AliasSampler, MasterRng, StreamRng}`) and external crates already in `Cargo.lock`. Dependencies point from `lab_live` to `lab`, never the reverse, so adding the module cannot change the compiled T1 code paths.
- `Cargo.lock` is byte-identical to the freeze, so no dependency version changed. The only Cargo.toml difference is a test target declaration.
- Statement scope: this is inspection evidence plus the byte-identity of the other 52 frozen files, not a re-run. Compiler output equivalence (identical binary or identical determinism hashes after the changes) was NOT re-verified here, and no release rebuild or smoke-matrix re-run was done for R-01. A reviewer who needs that evidence should rerun the fixed smoke matrix (determinism smoke hashes) on a release build; this is not claimed.

## 5. Which historical findings remain valid

- Remain valid as records of what ran: the sealed result artifacts (hash-verified), the tuning artifacts `lab-results/tuning_v2_2*` (hash-verified, listed in the freeze), the dry runs `lab-results/m6_dryrun_*`. They were produced from the frozen inventory and none of them depends on a file that changed after the freeze, except in the sense that `Cargo.toml` and `src/lib.rs` define the build (see above).
- Not attributed: nothing produced after the freeze by `lab_live` or `phantom-lab` live scenarios (Scenario B) is covered by the T1 protocol; those are separate evidence classes and must be labeled as such.
- Not established by this addendum: that the sealed result is correct or generalises. It is a simulation (T1), not a measurement.

## 6. Limitations and contamination notes

- The addendum is written after the sealed run by the same working group. It cannot prove that no other undetected change occurred between the freeze and the run, beyond what the run-once marker and the manifest's `source_sha256` (52 files) record.
- Integrity of the addendum itself rests on a pin: `docs/research/preregistration_m6_v2_2_addendum_1.pin` holds the SHA-256 of the addendum JSON, and the addendum JSON holds the SHA-256 of the freeze record, the v2.2 amendment and this prose. The pin cannot live in `eval.rs` or the test file because those are listed in the addendum (circular). Anyone who can rewrite the JSON, the prose and the pin together can forge a consistent set; detection then needs the hash recorded outside the repository tree (the lead's progress ledger, or a git commit). The addendum JSON hash must be recorded there by the lead.
- The verifier is deliberately strict: an addendum entry for a file that did not change is rejected, as is any further edit of an excused file. Any later edit of `eval.rs` or the m6 test file therefore requires a new, separately pinned addendum (addendum 2). The pin/addendum file names are fixed to addendum 1.
- `rustc` must still equal the recorded toolchain (`rustc 1.98.0 (88d9e12ae 2026-08-18)`).
- Order of events (real clock, UTC): 01:36:19 state check; 01:36:42 re-hash of all 56 files (2 differ) and pinned-doc and sealed-artifact checks; 01:39:36 (file mtime, 07:09:36 +05:30) pre-change copies of eval.rs and the test file taken; verifier and tests edited; clippy `-D warnings` over all targets passed; 01:43:16 final hashes of eval.rs and the test file computed; addendum JSON then written, then the pin.

## 7. Files this addendum anchors

See `changed_files` and `anchor_files` in `preregistration_m6_v2_2_addendum_1.json`. After the JSON is pinned, the standard-suite test `test_real_v2_2_freeze_record_verifies_when_present` runs `verify_freeze_with_addendum` on the real tree.
