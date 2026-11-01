# QA report A: Chapters 1 to 3

Reviewer: QA agent A (AI second look, not INDEPENDENTLY VERIFIED). Date: 2026-10-30 (machine clock). Tree: branch `feat/publish-phantomcache-core`, HEAD `df68eaa` at the time of writing; `git status --short` shows 113 lines before and after my work (no product file touched).

## 1. Counts

| Item | Result |
|---|---|
| References extracted and checked against the current tree | about 300 (about 75 file paths, about 110 symbols or functions, 25 named tests, all 14 gateway routes plus 9 Lab routes plus 2 health routes, all 43 config keys with defaults and wiring, about 30 crate and package versions, 17 G4 or SEC ids, 8 count claims) |
| Wrong references found and fixed | 9 distinct defects (see section 2) |
| Marked UNVERIFIED by the chapter authors and left as such | the chapters already carry their own UNVERIFIED notes (HTTP/2, ingress TLS 1.3-only by test, DER parser order, CI runs, npm audit, non-Windows lab); I added none and removed none |
| Figures rendered and viewed (headless Chrome, throwaway profile, deleted afterwards) | 12 of 12 SVGs; 3 edited and re-rendered |
| Word counts (wc -w) before to after | Ch1 9,383 to 8,954 (-4.6%); Ch2 10,320 to 10,094 (-2.2%); Ch3 10,359 to 10,286 (-0.7%). Below the 10% target: most remaining text is technical fact, judge boxes or status labels that I would not cut. Counts include the additions below (G4 counts, caveats). |

## 2. Edit log (location | defect | fix | evidence)

| Location | Defect | Fix | Evidence |
|---|---|---|---|
| ch01 Status line; ch01 section 1.10 table (HEAD row); ch02 reading-rules note | HEAD quoted as `caeedfd` | Now "at the time of writing HEAD is `df68eaa`" | `git rev-parse HEAD` = df68eaa |
| ch01 section 1.7 ("I confirmed the routes exist in `build_router`") and 1.8 ("merged in `build_router`") | `build_router` does not exist | `create_app_with_lab` | grep `fn build_router` in `src`: no hit; `src/api/server.rs:191` |
| ch01 section 1.11 | G4 dispositions not stated; sandbox patches not named | Added 75 OPEN (6 owner-named: G4B-01, -03, -05, -06, -07, -08), 20 RECORDED-LIMITATION, 2 IN-SANDBOX-REMEDIATION, 1 UNDER-INVESTIGATION, 1 REFUTED; sandbox patches (U-01, G4A-01 proposal, M3) not integrated | register section 3.1 and line 59 |
| ch01 section 1.4 | Required phrase and the DELETE 204 meaning not stated in one place | Added "N independent caches with an invalidation bus" and "a 204 means the local purge plus the routed primary's acknowledgement" | `delete_cache_handler`, `invalidate_key_across_cluster` |
| ch01 Figure 1.1 (SVG) | STALE box said "inside PHANTOM_MAX_STALE_MS", implying the cap is the window; text touched box edges | Re-worded: inside the origin's stale window, hard-capped by PHANTOM_MAX_STALE_MS; text re-wrapped; `<desc>` updated | `check_stale_serving` (window from SWR/SIE directive, bounded by cap), `src/core/cache.rs` ~line 790-815 |
| ch02 section 2.7.3 step 4 | Said `route_key` skips Suspect/Recovering nodes; omitted the second pass | Added the fallback pass | `src/dist/router.rs::route_key` lines 1351-1393 |
| ch02 section 2.7.3 step 7 | Header value shown as plain `PrimaryFailover` | Notes the optional `(node id)` suffix | `src/api/server.rs` ~2005-2007 |
| ch02 section 2.9.2 `ORIGIN_URL` row | "Env (also read by main)" imprecise | `EnvExport`; `main` copies it into `ApiState.origin_url` | `src/config.rs:125`, `src/main.rs:253` |
| ch02 section 2.7.3 step 5 | G4B-06 (ack-epoch check vacuous) not mentioned | Added with its disposition (OPEN, owner-named) | register row G4B-06 |
| ch02 section 2.11 item 2 | Did not say the `main.rs` startup message is now correct | Added: `config.rs` ~858 stale; `main.rs` ~373 and limitations.md current | `src/config.rs:858`, `src/main.rs:373` |
| ch02 Figure 2.1 (SVG) | "PHANTOM_LAB_ENABLED" and "PHANTOM_JWT_SECRET" text touched or overflowed their box edges | Font set to 11 px for those two lines | rendered PNG |
| ch02 Figure 2.4 (SVG) | Showed `(OriginCapSaturated)` as if it were the header | `(X-Cache-Error: CapacityExceeded)`; OriginCapSaturated is only the reason header | `map_origin_error_to_response`, `src/api/server.rs` ~2217-2231 |
| ch02 What you will learn | 6 bullets (style says 5) | Merged to 5 | style file section 4 |
| ch02 judge box "How do the nodes communicate" / architecture box | Phrase | Added "N independent caches with an invalidation bus" in the distributed-cache answer | consistency list |
| ch03 section 3.11 | "23 `.mjs` test files" | 30 | `ls ui/test/*.mjs` = 30 |
| ch03 section 3.7 | `extract_sans_from_der`, `compute_cert_fingerprint` attributed to `node.rs`; SAN validator function "not recorded" | They are in `src/security/cluster.rs`; validator is `parse_san_node_identity`; node.rs calls them | grep |
| ch03 section 3.12 | Baseline attributed to CLAUDE.md (which says 242) | Attributed to `docs/STATUS.md` section 3; CLAUDE.md quotes older 242 | STATUS.md line 63; CLAUDE.md |
| ch03 judge boxes (10) | Header read "(about 45 seconds spoken)" instead of the mandatory format | Replaced with "(about 30 to 60 seconds spoken, plain English):" | style file section 3 |
| ch03 Figure numbering | Figure 3.3 appeared before Figure 3.2 | Renumbered in order of appearance (lock-vs-await is 3.2, concurrency model 3.3) and fixed the cross-reference | file order |
| ch03 G4A-01 wording (Rust box, concurrency box, test box, key ideas, answer 9) | Called G4A-01 "open"/"known open race"; register says UNDER-INVESTIGATION and the owner calls it unconfirmed | Reworded to "reported race, UNDER-INVESTIGATION, confirmed by AI code reading" | register row G4A-01 |
| ch03 section 3.4 judge answer | "Hyper has years of public use and fuzzing" (unverified) | Replaced with a statement I can support | no evidence in repo |
| ch03 section 3.4 caution (TLS 1.3) | Did not cite the register | Added G4B-20 (CONFIRMED by AI reading, OPEN, Low) | register row G4B-20 |
| ch03 section 3.5.3 | Histogram worked example duplicated Chapter 5, section 5.11 | Replaced by a pointer; I recomputed the example (1000 us -> bucket 506 -> 1002 us, 0.2%, worst 0.39%, 2,688 buckets, about 21 KB) and it is correct; kept the two comment discrepancies; noted Chapter 5's finding that no production reader exists | `value_to_bucket`, `bucket_to_value` |
| ch03 section 3.2 etc. | Duplicate route list, status boilerplate | Replaced by pointer to Chapter 2, section 2.3.1 | trimming |
| Trimming (all three chapters) | Padding and repeats of Chapter 2/4/5 content (freshness rules, shard explanation, DELETE detail, auth boundary list, deployment list) | Shortened or replaced with `see Chapter N, section N.M` pointers; no technical fact, status label, discrepancy or judge box removed | word counts above |

## 3. Claim and judge-answer check

- Status labels: only vocabulary labels used. No INDEPENDENTLY VERIFIED claim anywhere in Chapters 1 to 3; all reviews described as AI second looks.
- Judge answers: no performance superiority claimed; Go/C# and Rust answers say "no recorded comparison"; incomplete features (peer reads, durability, adaptive benefit, ingress TLS 1.3, default TTL) answered honestly. The "Guarantees linearizability" comment is stated as unsupported in Chapters 1 and 2 (not repeated as a guarantee). Chapter 3's test name `test_invalidate_during_concurrent_reads_linearizable` now carries a caveat that the name is not a proof.
- Consistency facts verified in code and read identically in my chapters: 8 shards (`src/main.rs:204`); GET local-only and `forward_key_over_tcp`/`route_key` used only in `router.rs` and DELETE; `capacity_weight = 1.0` (`router.rs:1363,1381`); 204 = local purge plus routed primary's ack; `PHANTOM_DEFAULT_TTL_MS` NotWired (`config.rs:117`, literal `60_000` at `server.rs` 1405 and 1688); G4 counts as above; sandbox patches not integrated.
- Skeleton present in all three: what you will learn (5 bullets), key ideas, what you should now understand, knowledge check with `::: answers`, exploration tasks (3), challenge questions (2), figure captions with `../assets/chNN_name.svg` and a Source line. Fenced divs balanced (Ch1 11/11, Ch2 15/15, Ch3 15/15). Pipe tables: every row has the same column count. No 64-hex strings, no secrets, no marketing words (the only "robust" hits are the test file name `parser_robustness_tests`).

## 4. Figures viewed

| Figure | What I saw | Action |
|---|---|---|
| 1.1 request outcomes | STALE label inaccurate; text touching border | Fixed, re-rendered, clean |
| 1.2 stampede | Legible; the arrow label "100" sits close to the arrowhead; footer text small but readable | None |
| 1.3 three pillars | Legible; borders (solid/dashed/dotted) distinguish states in greyscale | None |
| 2.1 components | Two box texts touching edges | Fixed, re-rendered, clean |
| 2.2 hit sequence | Legible; matches code (two shard-lock calls) | None |
| 2.3 miss sequence | Legible; one annotation crosses a lifeline (readable) | None |
| 2.4 origin failure | `OriginCapSaturated` mislabelled as header | Fixed, re-rendered, clean |
| 2.5 DELETE | Legible; annotations cross lifelines (readable) | None |
| 2.6 Lab and UI | Legible; "max 16 SSE streams, 16 KiB bodies" verified (`MAX_SSE_STREAMS`, `MAX_BODY_BYTES` in `lab_api.rs`) | None |
| 3.1 stack | Versions verified against `Cargo.lock` and `node_modules` (all match); "Node 24 locally" matches `node --version` v24.5.0 | None |
| 3.2 (lock vs await) | Legible | None |
| 3.3 (concurrency model) | Legible; "hash & 7" label near arrow start | None |

## 5. Cross-chapter conflicts for the lead (other chapters, not edited)

1. `ch06_security_reliability_performance.md` M8 line: "242 tests in 19 targets passing (CLAUDE.md, 2026-10-28 ...)" is followed by the 851/75 figure and is labelled as older, so it is consistent, but it lists 242 first; consider leading with 851.
2. `docs/STATUS.md` (source, not a chapter) still records HEAD `456bb25`; Chapter 8 line 33 already says `df68eaa`. Chapters 1 to 3 now match Chapter 8.
3. No chapter 4 to 8 text was found that contradicts: 8 shards, GET local-only, constant weight, DEFAULT_TTL NotWired, G4 counts (Chapter 6 line 696 and Chapter 8 line 120 match the register), or sandbox-not-integrated. Chapter 5 section 5.11 says no production reader of the histogram exists; Chapter 3 now points there. Chapter 4 should confirm its section 4.8 holds the full freshness rule list (Chapter 2 now points to it) and 4.5 the fencing detail (Chapter 1 now points to it).
4. The CLAUDE.md note that the startup message at `main.rs:368` is out of date is itself out of date: `main.rs:373` now describes the shared serving cache correctly, while `src/config.rs:858` still carries the stale warning text.

## 6. Remaining concerns

- Word reduction reached about 3% overall, not 10%. Further cuts would remove facts or judge-box content.
- Not independently re-verified by me: the 8 ignored tests and 851 total (reported by STATUS.md), HTTP/2 serving, DER parser ordering, whether CI ever ran, `npm audit`, behaviour on non-Windows. These stay UNVERIFIED in the text.
- Chapter 1 cites `docs/STATUS.md` quote "does not show that intelligent caching is not viable..." which I located by grep on one line only (line 80 region); wording not compared character by character.
- Judge answer wording in Chapter 3 still contains several "I" statements (first person); acceptable for a defence handbook but inconsistent with Chapters 1 and 2, which mostly use neutral voice.
