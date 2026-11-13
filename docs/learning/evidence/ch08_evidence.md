# Chapter 8 evidence file

Chapter: `docs/learning/src/ch08_limitations_roadmap_defence.md`. Written 2026-10-30, about 16:15 to 16:35 machine clock. Read-only investigation: no product file, test, config or other chapter was edited; no cargo run; no git write operation; the sealed-salt directory was not read; no 64-hex commitment string is printed. "Opened" means I read the file or lines in this session.

## 1. Claim table

| Claim | Source (file::symbol) | Status label | How verified | Limits |
|---|---|---|---|---|
| HEAD is `caeedfd`, branch `feat/publish-phantomcache-core`, three docs/tooling commits after `456bb25` | git | reproduced here | `git rev-parse HEAD`, `git branch --show-current`, `git log --oneline -8` | Snapshot at ~16:13 |
| 37 tracked modified, 4,494 untracked, 4,526 total lines in `-uall`, 111 lines without `-uall`, 0 stashes | git | reproduced here | `git status --short -uall` filters, `wc -l`, `git stash list` | Count moves as agents write |
| 851 passed / 0 failed / 8 ignored / 75 targets (debug) | `docs/release/verification_register.md` V-01; `docs/STATUS.md` section 3 | reported by lead re-run (record read) | Opened both | Not re-run by me; debug only |
| `cargo fmt --check` fails, 38,042 diff lines | `docs/research/g1_verdict.md` D4 | reported | Opened | Not re-run |
| 298 UI tests at 15:29; 299 after 16:07 | `verification_register.md` V-07; `docs/release/lead_timeline.md` 16:07 entry | reported | Opened; own grep count of `test(`/`it(` ~299 in 26 files of `ui/test` | grep is not an execution |
| ~821 Rust test attributes in `tests/`, 19 in `src/`, 97 `.rs` files under `tests/`, 9 `#[ignore]` | tests/, src/ | reproduced here (grep) | `grep -rE "#\[(tokio::)?test"` counts | Cannot see macro-generated tests; differs from 851 |
| No milestone has a recorded owner acceptance; no human review anywhere | `docs/STATUS.md` section 1; ledger section 6 | reported by prior agent (docs) | Opened | Owner may hold records elsewhere |
| 99 findings: High 4, Medium 37, Low 42, Info 16; 75 open (6 owner-named), 20 recorded, 2 sandbox, 1 investigating, 1 refuted | `docs/release/g4/cycle1_findings_disposition_register.md` section 3.1 | reported by prior agent; recomputed by me | Opened in full; recomputed with a node script over register rows (99 rows parsed, all assigned to a theme, per-theme dispositions and severities summed to 99) | Themes are my grouping; register is an AI second look |
| Theme counts in table 8.4.2 | register rows | reproduced here | script `scratchpad/th.mjs` (not in repo) mapping each ID to one theme | Mapping is judgement; F24 put under telemetry, G4B-34 and G4C-DEP under dependencies |
| Six owner-named findings G4B-01, -03, -05, -06, -07, -08 | `docs/release/owner_decisions_register.md` ODR-4; register section 1 | reported (record read) | Opened | |
| G4A-01 mechanism present in current tree (fence check; epochs re-read and stamped at insert) | `src/core/cache.rs::finalize_fetch` | verified in code | Opened the region (fence check near the `state.generation == op_gen` compare; insert block with second `get_effective_epochs` and `entry_tenant_epoch` / `entry_global_epoch` assignment) | Reading only; the race itself was reproduced only by RACE with injected yield (`docs/research/g4a01_race/status.md`) |
| G4A-02: `FaithfulS3Fifo::on_remove` removes from `freq` and `ghost_set` only | `src/core/eviction.rs::FaithfulS3Fifo` | verified in code | Opened | Queue contents not traced further |
| S3-FIFO not reachable in default serving path | `src/core/cache.rs::set_eviction_policy` callers: `src/control/pie.rs`, `src/intelligence/executor.rs`; `src/main.rs` (`pie: None`, `PartitionedCache::new(8)`) | verified in code (grep), partly | `grep -rn set_eviction_policy src`; opened main.rs lines | Did not enumerate every caller incl. tests and runtime paths; executor `apply_policy` "no serving caller" is from the limitations record + grep of `apply_policy(` showing only `t1_sim.rs` |
| G4A-03 sweeper iterates `shard.map.iter()` with candidate cap | `src/core/cache.rs` sweeper | verified in code | Opened | |
| G4B-01/-03 control_handler extractor before auth; unknown event sets all nodes Active | `src/api/server.rs::control_handler` | verified in code | Opened | Other routes not traced |
| G4B-13 `nbf` not validated | `src/security/jwt.rs` | verified in code (absence) | Grep shows `Validation::new(HS256)`, `leeway = 10`, no `validate_nbf` | Library default behaviour not verified at runtime |
| G4B-27 `DefaultHasher`; `capacity_weight = 1.0` | `src/dist/router.rs::route_key` | verified in code | grep + earlier output | |
| `PHANTOM_DEFAULT_TTL_MS` not wired; literal `60_000` at two sites | `src/config.rs`, `src/api/server.rs` | verified in code | `grep -rn` shows config.rs key and two `60_000` literals in server.rs | |
| `pie: None` | `src/main.rs` | verified in code | grep | |
| `CountMinSketch` used only by a unit test | `src/intel/sketch.rs`, `tests/unit/wp2_tests.rs` | verified in code | `grep -rn CountMinSketch src tests` | |
| Cluster `GetRequest` stub formats `val_node_<id>` | `src/dist/node.rs` | verified in code | grep (two sites) | Reachability analysis is the investigator's |
| Local cluster node shares the HTTP cache in mTLS path | `src/main.rs` `VirtualNode::new_with_cache(node_id, cache.clone())` | verified in code | grep, line ~332 | Insecure branch not fully read |
| `config.rs` still says peer invalidations do not purge HTTP cache | `src/config.rs` (~858) | verified in code | grep | Contradicts mTLS path (G4B-32) |
| Anti-entropy worker started at 250 ms | `src/main.rs` | verified in code | grep | Effectiveness not established |
| Faithful policies exist | `src/core/eviction.rs` `FaithfulSieve`, `FaithfulLru`, `FaithfulS3Fifo`, trait `ShardEvictionPolicy` | verified in code | grep | Algorithmic faithfulness not reviewed |
| Histogram is recorded into in the telemetry path | `src/intel/telemetry.rs` (`histogram.record`) | verified in code | grep | Read path in serving not checked |
| Gateway starts with `PartitionedCache::new(8)` | `src/main.rs`; `src/core/cache.rs::PartitionedCache::new` | verified in code | Opened | Handoff says 64 partitions |
| M3 reproduction, patch set, tests counts | `docs/research/m3_durability/remediation/00_status_and_plan.md` | reported by prior agent | Opened status file | Sandbox only; AI-only review |
| U-01 sandbox fix results | `docs/research/u01_backend/status.md` | reported by prior agent | Opened | Debug, one target |
| G4A-01 natural stress 0/3.16M, injected-yield repro | `docs/research/g4a01_race/status.md`; register update 15:53 | reported by prior agent | Opened | Sandbox |
| H1 NOT_SUPPORTED, MVP-B | `docs/research/m6_sealed_results.md` | reported (record read) | Opened | T1 simulated; I did not read `lab-results/` |
| Scenario B TRADE_OFF_FAILURE both arms | `docs/research/g1_verdict.md` | reported | Opened | Exploratory |
| C6 amendment: cold first use 35-60 ms; warm P99 ~10-14 ms | `docs/research/claims_ledger.md` section 5 | reported | Opened | Debug, loopback |
| Frozen-file differences (4 of 56) | `docs/release/known_limitations_frozen_code.md` section 2 | reported (reproduced by LIMITS agent) | Opened | I did not re-hash |
| Roadmap stages, experiments E0 to E12, matrices | `docs/research/vision/phantomcache_three_pillar_product_direction.md` sections 2.1, 5, 8.5, 12 | proposal (not approved) | Opened sections | Document says nothing in it is approved |
| Owner approved toolkit and redesign; UI implementation phase open | `docs/release/lead_working_state.md` 3b | reported | Opened | |
| Limitations list (no metrics, no reload, Windows only, idle CPU) | `docs/user/limitations.md` | reported | Opened | Idle CPU cause is a hypothesis |
| Out-of-date statements (handoff, README, CLAUDE.md) | `docs/handoff/00_START_HERE.md`, `05_OPEN_RISKS_AND_NEXT_STEPS.md`, `README.md`, `CLAUDE.md` | verified in docs vs code | Opened and compared | |

## 2. Figure checks

- Figure 8.1 (`assets/ch08_three_pillar_status.svg`): each box label taken from the claim rows above and the matrix in the chapter; cross-checked against vision doc sections 5.1 to 5.4 and `STATUS.md`. Four border styles distinguishable in greyscale (stroke width, dash pattern, double line). Text size 13 to 18 px. No external resources. Generated by a script (not in repo) and then opened as text for syntax; not rendered in a browser (UNVERIFIED rendering).
- Figure 8.2 (`assets/ch08_roadmap_stages.svg`): stages taken from `execution_register.md`, `lead_working_state.md` section 4, vision doc section 12. No calendar dates because none are recorded. Rendering not checked visually (UNVERIFIED); wrap width chosen at 25 characters for 200 px boxes.

## 3. Discrepancies found (document vs code or document vs document)

1. `docs/handoff/00_START_HERE.md` says "64-Partition Lock Striping"; `src/main.rs` builds `PartitionedCache::new(8)`. Other chapters (Chapter 4) must reconcile.
2. `src/config.rs` (~858) says peer invalidations do not purge this gateway's HTTP cache; `src/main.rs` mTLS path shares the HTTP cache with the local node (also G4B-32, `docs/user/limitations.md`).
3. `CLAUDE.md` section 6 says evictors are scans and the only staleness bound is a fixed 60 s; `CLAUDE.md` section 5 and `src/core/eviction.rs` show faithful policies; origin headers and the stale cap also bound staleness. `CLAUDE.md` still cites 242 tests.
4. `docs/research/claims_ledger.md` round 2 says the histogram has no callers in `src/`; `src/intel/telemetry.rs` records into it.
5. `docs/STATUS.md` says 298 UI tests; the lead timeline (16:07) records 299.
6. M3 status file says "fix prepared and verified in a sandbox"; `lead_working_state.md` still lists the M3 agent as running (same time frame, not reconciled).
7. G4A-19 versus G4B refuted item 5 on the `__default__` tenant alias: unreconciled in the register itself.
8. `README.md` says `cargo 1.75+`; G4C F28 says about 1.88 is needed (not re-derived).
9. Handoff documents still say "Phase 4 formally accepted"; a correction notice and `docs/handoff/CORRECTIONS.md` exist; no owner acceptance is recorded.
10. G4 register says `known_limitations_frozen_code.md` did not exist when written; it now does and lists 24 rows; the lead's reconciliation note covers the four-row difference.

## 4. UNVERIFIED items

- Any Rust or UI test result: I ran nothing; all test results are from the registers.
- The 851 vs my grep count difference (macro-generated tests, parameterised cases).
- Per-route ingress behaviour beyond `control_handler`.
- Whether serving percentiles read the new histogram.
- Reachability of S3-FIFO in all paths (only checked `set_eviction_policy` callers by grep).
- The frozen-file hash differences (not re-hashed).
- Rendering of both SVGs.
- The G4 spot-check items other than those listed in chapter section 8.4.4.
- Rust MSRV (1.88).
- Whether the owner holds acceptance records outside the repository.
