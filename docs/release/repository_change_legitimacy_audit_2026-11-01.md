# Repository change legitimacy audit, 2026-11-01

**Verdict: HEALTHY WITH CLEANUP CANDIDATES, with one process risk that outranks everything else: almost none of this work is committed.**
**Status:** read-only audit, complete for the stated coverage (section 9). Nothing in the repository was modified, staged, deleted or cleaned. Outputs of this audit: this report and board entry D-061 / section J9 (both new or edited by the audit; everything else listed below was measured before they were added).
**Evidence tags:** VERIFIED-IN-CODE (read the file, cited), REPRODUCED (command run here), INFERRED, UNVERIFIED, NOT RUN. Cargo, npm and tests were **NOT RUN**: no statement here says anything compiles or passes. Four read-only reviewers inspected the code areas; their key claims were spot-checked by the lead where stated.

## 1. Baseline (REPRODUCED)
| Item | Value |
|---|---|
| HEAD / branch | `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` / `feat/publish-phantomcache-core` |
| Commits on branch / stashes | 8 / 0 |
| Tracked files at HEAD | 233 |
| Staged changes | 0 |
| Tracked and modified | 51 files, 14,826 insertions, 977 deletions (identical with whitespace and CR ignored, so this is real content, not line-ending churn) |
| Untracked files (`git ls-files --others --exclude-standard`) | 4,825 (515 MB with the 51 modified files) |
| Lines in a normal `git status --short` | 190 (untracked directories collapse) |
| Ignored files | 17,215 (9,888 `target/debug`, 2,027 `target/release`, 5,148 `ui/node_modules`, 3 `ui/dist`, 66 under `docs`, 37 `lab-results`, 29 `.local`, remainder logs) |
| Working-tree fingerprint (taken before this audit added files) | manifest of `kind, path, sha256` for 4,875 files (board file excluded): SHA-256 `53A51E9D069928859CB8CC5CE3520A1BA1E436C5D7D5FACFA393C001F8F78453`; `git diff` of tracked files excluding the board: `57BB4A6C96853271DD8DC790CC784999A4E4A36F8AC261C880B8718D666B1AE7`. Files: `D:\ClaudeTemp\audit\legit\{inventory.csv, fingerprint_manifest.txt, status_*.txt}` |

**Source of the "4,000+":** the editor lists untracked files one by one. It is 4,825 untracked files plus 51 modified, not 4,000 modified source files. Only 69 files are Rust source (59 untracked and 10 modified under `src/`).

## 2. Counts by class (all 4,876 changed files; classification by path and type, REPRODUCED)
| Class | Files | MB | Notes |
|---|---|---|---|
| 1 Source (`src/*.rs`) | 69 | 1.3 | 24,129 untracked lines of Rust in `src/`, plus the 10 modified files |
| 2 Supporting: tests | 186 | 2.3 | 33,911 untracked test lines (Rust) |
| 2 Supporting: tooling, UI, launchers | 173 | 1.5 | `ui/src` 11,179 lines, `ui/test` 7,520, `ui/scripts` 4,382, `scripts` 5,839 |
| 2 Supporting: config / manifests | 5 | 0.1 | Cargo.toml (+320), Cargo.lock, ui package files |
| 2 Supporting: documentation and records | 409 | 32.5 | includes 29 modified tracked docs |
| 3 Generated: browser / UI evidence (screenshots, driver JSON) | 2,028 | 241.4 | `docs/design/remediation`, `docs/demo/browser_verification`, `live_demo_shots` |
| 3 Generated: PDFs | 7 | 36.7 | regenerable from `docs/learning/src` |
| 4 Experimental evidence: `lab-results/` | 264 | 46.8 | sealed M6, dry runs, scenario_b, smoke, tuning |
| 4 Experimental: benches and `bench-results/` | 29 | 0.3 | |
| 4 Snapshots / backups: `.local/snapshots` | 1,701 | 150.9 | 8 phase snapshots at base `456bb25` |
| 6 Hygiene / uncertain | 5 | 1.2 | 3 pid files, `transcript_copy.jsonl`, `.claude/agents` |

**Flags (overlap with the classes above; counts are not additive):**
| Flag | Files | Basis |
|---|---|---|
| Redundant: byte-identical to another file (extra copies only) | 1,818 (520 hash groups, 190.2 MB) | SHA-256 over all 4,876 files, REPRODUCED. 523 `.local` files (109.7 MB) equal files in the live tree; about 47 MB of redundancy sits inside `docs/` |
| Questionable | about 146 | 139 files in folders labelled superseded, aborted or contaminated; 3 stale pid files; 2 `.pyc` files; `transcript_copy.jsonl` |
| Uncertain | 3 | two raw AI-session transcripts in `docs/handoff/transcripts/` and `.claude/agents/general-purpose.md` |
| Valid and necessary, no flag | at least 2,900 | remainder; a lower bound because flagged files overlap |

## 3. Largest contributors
`docs/design/remediation` 1,102 files / about 115 MB (909 PNG); `docs/demo/browser_verification` 938 files / about 122 MB (818 PNG, three timestamped rounds, five e2e runs); `.local/snapshots` 1,701 files / 150.9 MB (only 349 distinct hashes, 33.5 MB unique; the 12.7 MB handoff transcript appears in 7 snapshots); `docs/learning` 136 files / 44.7 MB (seven PDFs, 36.7 MB, two copies of the 8.2 MB v1 handbook); `lab-results` 264 files / 46.8 MB. PNG and JPG together are 1,839 files / 239 MB of the 515 MB total.

## 4. Prioritised findings
| ID | Sev | Affected | Evidence | Recommended disposition |
|---|---|---|---|---|
| PROC-1 | **High** | the whole tree | REPRODUCED: HEAD tracks 233 files; 4,825 files plus 51 modifications are uncommitted, including all of `src/intelligence*`, `src/config.rs`, `src/security/`, `src/lab*`, `tests/integration/` (reviewers confirmed untracked) | The only protection is copies in `.local/snapshots`, `phantom-cache-backups` and `D:\ClaudeTemp`. Owner decision needed on a commit plan (not done here; the repository rules forbid committing without an explicit request). Resolve LAB-2 and LAB-5 first |
| PIE-1 | **High** | `src/control/pie.rs:35-58, 94-108` | VERIFIED-IN-CODE (lead re-checked the diff: `with_cache`, `set_cache`, `set_eviction_policy` added) | CLAUDE.md 6b says do not extend this legacy stub. The diff makes it write the live eviction policy; threshold formula and `expected_utility: 85.0` remain; Rollback never resets `active_policy`. Unreachable today (`pie: None`, `src/main.rs:434`) but a latent hazard. Owner decides: revert to HEAD behaviour or keep for `p4_pie_tests` only |
| LAB-2 | **High** (process) | `.gitignore` (33 lines) | REPRODUCED: `git check-ignore` matches nothing for `.local/`, `*.pid`, `*.pyc`, `transcript_copy.jsonl` | `git add -A` would sweep in about 4,800 files including snapshots and `.pyc`. Add ignore rules after owner review |
| LAB-5 | **High** (privacy) | `transcript_copy.jsonl`; `docs/handoff/transcripts/*.jsonl` (1.2 MB, 12.7 MB) | REPRODUCED: root copy is byte-identical to `…session_1_oct11_to_oct25.jsonl` (SHA-256 `0c518df5…`). Reviewer: first record concerns unrelated coursework; no private-key or common token patterns found by a pattern search (16 "secret/api_key" word hits not individually examined) | Raw AI-session exports. Owner decision before any publish. Root copy is an exact duplicate |
| CORE-1 | Medium | `src/core/cache.rs:1463-1471, ~1517-1521` | VERIFIED-IN-CODE by reviewer; matches known open finding G4A-01 (epoch re-read after the check can stamp a pre-invalidation body with a post-invalidation epoch) | Investigate. CLAUDE.md requires an independent review and a failing-first test for fencing/epoch changes |
| CORE-2 | Medium | `src/core/cache.rs:277, 1707-1757` (`legacy_shards`, legacy `insert/get/evict_sieve`) | VERIFIED-IN-CODE: called only from tests (`wp1_tests.rs:31`, `security_phase4a_invalidation_tests.rs:259`, intel tests); still purged on every peer invalidation | Old and new cache stores coexist. Tests that insert into the legacy store prove nothing about the store the gateway serves from. Migrate tests, then remove-candidate |
| PIE-6 | Medium | `src/main.rs:1-58` vs `src/lib.rs` | VERIFIED-IN-CODE: `main.rs` re-declares the module tree (crate compiled twice; process-global statics such as `GLOBAL_INTEL_EXECUTOR`, `PIE_V2` exist once per copy; `sim::events` missing in the binary) | Integrate (`use phantom_cache::…`) with owner approval; drift hazard |
| SEC-1 | Medium | `src/api/server.rs` (no Host/Origin check; lab API has one at `lab_api.rs:1060-1094`) | VERIFIED-IN-CODE | With no JWT secret (allowed on loopback) DELETE, `/api/control` and intelligence routes are anonymous; DNS rebinding possible (INFERRED). Port the guard |
| SEC-2 | Medium | `src/api/server.rs:1583-1597` | VERIFIED-IN-CODE: BYPASS path copies origin headers after setting `X-Cache-Status` / `X-Cache-Source` | An origin can overwrite the telemetry-honesty headers and pass `Set-Cookie`. Allow-list needed |
| SEC-3 | Low | `src/api/server.rs:3213-3219` | VERIFIED-IN-CODE: stored ETag / Last-Modified written raw into request lines; bare `\n` not checked | Sanitise on store |
| PIE-5 | Medium | `src/api/server.rs:534-546` (`pie_record`) | VERIFIED-IN-CODE: `origin_latency_us: 0`, `size_bytes: 0` hard-coded | Size and cost policies cannot fire on live traffic; UI must show them as unavailable (board already records this follow-up) |
| TEST-1 | Medium | `tests/unit/wp4_tests.rs:8`; `wp2_tests.rs:49-88`; `tests/integration/p4_pie_tests.rs:156-200`; `m7_security_review_tests.rs:421-438`; `invalidation_contract_tests.rs:30` | VERIFIED-IN-CODE by reviewers: a tautology; tests that assert the legacy threshold stub; a test named "adaptive" that checks a formula; a spoofed-tenant test that only asserts "not 401/403"; a test named "linearizable" that is a sequential check; wall-clock bounds (`< 5ms`) | Relabel or retire; do not cite as evidence of behaviour. Meaningful tests also exist (read-your-delete race `wp3_tests.rs:1811`, pinned-eviction contract, JWT alg/exp tests, v2 negative controls) |
| DOC-1 | Medium | `src/api/server.rs:2419` ("Guarantees linearizability"); `src/config.rs:859-861`; stale line citations in CLAUDE.md and `docs/user/configuration.md:44` | VERIFIED-IN-CODE | Comment and warning overclaim relative to `docs/user/limitations.md`; `PHANTOM_DEFAULT_TTL_MS` is still NOT WIRED (`src/config.rs:116-117`; literal `60_000` at `server.rs:1708`, `:2096`). Code/doc fixes need owner approval |
| PIE-3 | Info | `src/intelligence/` (v1), `src/intelligence_v2/` (v2), `src/control/pie.rs` (legacy) | VERIFIED-IN-CODE | Three generations coexist as a layering: v2 depends on v1 (`policies.rs:5-7`), v2 is the live shadow engine and cannot apply a policy (`applied: false`, no cache handle), v1 `apply_policy` is reachable only under canary/active with `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE` and has no runtime caller outside lab/tests (INFERRED from grep). Preserve both; document the layering once |
| CORE-3/9/5/6 | Low | `cache.rs:1715` and ~40 other `.lock().unwrap()` sites; `node.rs:87-111` (anti-entropy metric counts records, not evictions, and swallows `try_delete` errors); `cache.rs:1203-1216` (policy rebuilt O(n) under shard lock); `cache.rs:1437` (panic hook in production code) | VERIFIED-IN-CODE | Investigate for consistency; none is a demonstrated failure |
| LAB-10 | Low | `Cargo.toml:30` | `criterion` dev-dependency; reviewer found no use in `benches/` or `tests/` (benches are `harness = false`). The lead's grep found the word only in one test file and one source file, not examined as imports | Investigate; dev-dependency rules require justification |
| LAB-12 | Low | `scripts/` | Reviewer: `live_browser_walkthrough.mjs` and `run_demo_origin.ps1` have no references; 12 one-off `ui/scripts/browser-r*.mjs` drivers are not in npm scripts | Preserve with the evidence they produced |
| SEC-8 | Info | `src/security/upstream.rs` | VERIFIED-IN-CODE: per-address checks, IMDS/RFC1918/CGNAT blocking, pinned connection, no redirect following; gaps: `198.18.0.0/15`, `192.0.0.0/24` not blocked; hand-rolled HTTP/1.1 client is a maintenance surface | Preserve |
| SEC-4..7 | Info | JWT, auth matrix, bind/transport rules, cluster mTLS | VERIFIED-IN-CODE: HS256 pinned, iss/aud/exp enforced, no route bypasses JWT when a secret is set except `/health*`, non-loopback bind needs JWT plus TLS or the lab flag, mTLS is CA-only TLS 1.3 with fingerprint pinning | Preserve. Add PIE v2 route auth tests (none found in files read) |
| CORE-7/8/10/11 | Info | eviction, frame decoding, wiring, `sim/traffic.rs` | VERIFIED-IN-CODE: `ShardEvictionPolicy` wired; pinned keys not evicted; Sieve/LRU are O(pinned) per call (G4A-20, "O(1) amortized" comment overstated); frame parsing bounds-checked, 1 MiB cap; all new core modules reachable from runtime; traffic types simulator-only | Preserve |
| LAB-1/3/4 | Info | `lab-results/`, browser/remediation evidence, `.local/snapshots` | Reviewer + lead: sealed M6 folder (marker records freeze SHA-256 `65cf88db…`), scenario_b (30 runs, loopback), tuning are evidence; browser/remediation images are driver output with before/after sets that differ (0 of 211 after-images equal a before-image); `.local` holds 240 distinct files (16 MB) that exist nowhere else | Preserve until committed. Dry runs, smoke outputs and superseded/aborted folders are owner decisions |

## 5. Answers to the central questions
- **Accidental modifications:** none found in the 51 tracked files beyond PIE-1. The tracked diff is substantive (server +3,470, wp3 tests +2,249, cache +1,807, router +1,605, Cargo.lock +1,800, node +796) and not whitespace churn. The `ui/src/App.jsx` change (-504 lines) is a rewrite whose content moved into imported screens and components (reviewer: no unimported UI file found by name match, INFERRED).
- **Duplicated or coexisting implementations:** legacy vs partitioned cache store (CORE-2), three intelligence generations (PIE-3), duplicated module tree (PIE-6), `.local` snapshot copies, duplicate PDFs and transcripts. No duplicate anti-entropy implementation (`anti_entropy.rs` holds the tracker, `router.rs` the caller).
- **Disconnected code:** legacy `pie.rs` (unwired), v1 `apply_policy` (no runtime caller), `sim/traffic.rs` types (tests only), `invalidate_tenant/_all` (test-only).
- **Build artefacts in the tree:** none tracked. Ignored `target/` (kept, needed by the launcher) and `ui/node_modules`. Two `.pyc` files are untracked under `bench-results/tools/__pycache__`.
- **Files unrelated to PhantomCache:** none by content except the transcripts' unrelated coursework opening (LAB-5).
- **Regressions or security weaknesses:** none critical. See SEC-1..3 and CORE-1.

## 6. Preserve unquestionably
All `src/`, `tests/`, `benches/`, `ui/src`, `scripts/`, `packaging/` content; `lab-results/m6_sealed_2026-10-29T234845Z` and its marker, `tuning*`, `scenario_b`; `bench-results/`; the board, monitor-state, handoff, research and release documents; `CLAUDE.md`, `CORRECTIONS.md`, `MILESTONE_1_DESIGN_PLAN.md`; `.local/snapshots` (until commits exist, because 240 distinct files exist only there).

## 7. Safely regenerable (no action taken; owner decision)
Smoke outputs under `lab-results/smoke_*` (26 files, README says do not commit); the two `.pyc` files; the three stale pid files; `docs/learning` PDFs (regenerable from source, two identical 8.2 MB copies); screenshot sets produced by `ui/scripts/browser-*.mjs` (about 225 MB, 40 MB hash-duplicate). Exact duplicates inside `.local/snapshots` (about 110 MB equal to the live tree) become redundant once the tree is committed.

## 8. Needs further investigation
PIE-1 disposition; CORE-1 with a failing-first test; CORE-2 test migration; PIE-6; SEC-1/2/3; `criterion`; the `docs/research/pie_audit.md:37` statement that canary and a trace store are absent from `src/` (outdated); whether the sealed freeze hashes still verify (not run).

## 9. Coverage and limits
Read in full by reviewers: `finalize_fetch`, `evict_with_policy`, epoch code, frame decoders, node invalidation and anti-entropy handlers, eviction `select_victim`, P1/P6 policies, HyperLogLog, CUSUM, JWT validation, upstream fetch checks, cluster mTLS paths. **Skimmed or not read:** `router.rs` (about 1,600 lines) and `pool.rs`, the `coalesce.rs` diff, the stale-serving/hard-cap code, P2/P3/P5 and the arbiter, `lab_api.rs` beyond its auth, most `ui/test` files against their sources, `docs/user/lab_api.md` and `operate.md` against code, the bodies of the cluster attack tests, and the 29 modified tracked documents individually. Sealed-file contents and freeze hashes were not re-verified. **Cargo, npm, tests and the demo were NOT RUN**, so no behavioural claim is made.

## 10. Overall assessment
The work is overwhelmingly intentional and organised: source, tests and registrations are consistent (every `[[test]]` path exists and every integration test file is registered, per reviewer), security controls are real and enforced, and evidence is labelled and indexed. About 83 percent of the files (4,029 of 4,876) and 92 percent of the megabytes (about 476 of 515) are generated evidence, snapshot copies, PDFs and experimental results, not source. The count is inflated by generated and duplicate material, not by accidental source edits. The serious items are not volume: it is that this work is uncommitted (PROC-1), one rule-breaking edit to the legacy stub (PIE-1), an open epoch-fencing race (CORE-1), tests that overclaim (TEST-1), missing ignore rules (LAB-2) and raw session transcripts (LAB-5). Passing tests do not establish correctness here.
