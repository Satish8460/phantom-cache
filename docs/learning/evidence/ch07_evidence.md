# Chapter 7 evidence file

Chapter: `docs/learning/src/ch07_lab_and_innovation.md`. Date of work: 2026-10-30 (machine clock). Method: files opened with Read, Grep, shell `sed`/`cat`, PowerShell JSON reads, and four web pages fetched with WebFetch (summaries from a small model: class OPENED-VIA-TOOL). No cargo builds, no tests run, no browser run, no git mutation, no edits outside the three allowed locations. `D:\Personal\.phantom-sealed-m6` was not read. No 64-hex commitment was printed.

## Claim table

| Claim | Source (file::symbol) | Status label | How verified | Limits |
|---|---|---|---|---|
| Lab T1 simulator drives real cache/executor on a virtual-time event loop | `src/lab/t1_sim.rs::run_cell`, header comment | IMPLEMENTED, TESTED | Read header and `run_cell`; tests listed below | Not independently reviewed (G4C did not read the loop) |
| Event ordering by `(time_us, seq_id)` min-heap | `src/lab/events.rs::EventId`, `EventQueue::schedule` | IMPLEMENTED, TESTED | Read file; test name `test_t1_event_engine_total_ordering` seen in `lab_core_tests.rs` (body not read) | Test body not opened |
| `VirtualClock` is not used by `run_cell` | `src/lab/clock.rs`; grep `VirtualClock` over `src`, `tests` | verified in code | Grep: only `clock.rs`, `lab_core_tests.rs`, plus an unrelated `src/sim/clock.rs` used by `main.rs` | Grep over `src` and `tests` only; `tests/support/lab/clock.rs` exists (older copy), not read |
| Virtual time `now_ms = t/1000` passed to real cache | `t1_sim.rs::start_attempt` | verified in code | Read lines | none |
| Only wall clock is in `shrink_shard` of the product cache | `t1_sim.rs` header | UNVERIFIED | Quoted the author's statement | Not proven |
| SplitMix64 streams, `MasterRng::stream` by name | `src/lab/rng.rs::StreamRng`, `MasterRng` | IMPLEMENTED, TESTED | Read; tests `test_t1_master_rng_seed_splitting_determinism`, `test_t1_stream_independence` named | Bodies not opened |
| Alias sampler worked example | `rng.rs::AliasSampler::new`, `sample` | IMPLEMENTED | Read algorithm, traced by hand | Test body not opened |
| Seed sets T=10001..=10060, D=20001..=20030, sealed derived from salt, burned 30001..=30030 | `rng.rs::SeedSets` | IMPLEMENTED, TESTED | Read; commitment constant exists in source but was not printed | Salt custody weakness is documented in G4C/freeze record, not independently checked |
| Workload families and mapping | `t1_sim.rs::make_workload`, `workloads.rs` | IMPLEMENTED | Read; W-TR panics with "no trace data" | All synthetic |
| Open loop arrivals in T1 | `t1_sim.rs` `Ev::Arrive` branch (comment "open loop") | verified in code | Read | none |
| Origin model: c servers, queue, scenarios, CRN faults | `src/lab/origin_sim.rs::SimulatedOrigin::fetch`, `with_fault_crn` | IMPLEMENTED, TESTED (`lab_origin_tests.rs` exists; `test_brownout_faults_use_common_random_numbers_across_dispatch_orders` named) | Read `fetch` and `DegradationScenario` | Not independently reviewed; header says M/M/c, M6 uses constant service |
| `SimParams::standard()` values (250 rps, 400 keys, alpha 0.95, TTL 3000 ms, 8 servers, 60 ms, 8 s warm-up, 20 s measure) | `t1_sim.rs::SimParams::standard` | verified in code | Read | |
| Eight M6 scenario definitions | `t1_sim.rs::scenario_for`, `M6_SCENARIOS` | verified in code | Read | |
| Arm definitions BL-0..BL-12b, PIE-* | `t1_sim.rs::arm_config` | verified in code | Read | "PIE" naming is historical, differs from `src/control/pie.rs` |
| `percentile` nearest-rank | `t1_sim.rs::percentile` | verified in code | Read | Other places differ (G4A-24 / F24) |
| BCa implementation (B=10,000, jackknife acceleration) | `src/lab/analysis.rs::StatisticalEvaluator::bca_ci_mean` | IMPLEMENTED, TESTED on planted data | Read; tests named in G4C and `m6_research_evaluation_tests.rs` | No reference-implementation cross-check (G4C 6(ii)) |
| Sign-flip permutation exact for n<=16, else 20,000 flips | `analysis.rs::sign_flip_p_value` | IMPLEMENTED | Read | |
| Holm adjust | `analysis.rs::HolmBonferroni::adjust` | IMPLEMENTED | Read | |
| Synthetic validator uses percentile bootstrap | `analysis.rs::SyntheticAnalysisValidator::verify_analysis_pipeline`, `evaluate_paired`, `bootstrap_ci` | verified in code | Read | |
| Paired relative reduction, supported rule, adverse flag | `src/lab/eval.rs::analyze_endpoints` | IMPLEMENTED | Read | zero-control maps to 0 (G4C F16) |
| Worked toy BCa example (r = [0.10,0.02,-0.05,0.08,0.05]) | n/a (invented illustration) | illustration, labelled invented | Computed with a node script in the shell: mean 0.04, exact sign-flip p = 8/32 = 0.25, percentile CI approx [-0.01, 0.082] with an ad hoc LCG, fraction below mean 0.465, jackknife a = -0.0469 | z0 approx -0.09 computed by hand from 0.465; not run through project code |
| Runner refuses seed set S and family W-SF | `src/lab/runner.rs::validate` | IMPLEMENTED, TESTED (`sealed_seed_set_and_family_are_refused_before_any_output`, named) | Read `validate` | |
| Determinism tests | `lab_runner_lib_tests.rs::same_runspec_twice_gives_identical_cells_and_bytes`; `lab_reproduction_tests.rs::test_t1_10_rerun_bitwise_determinism` (body read: compares 4 fields over 10 reruns) | TESTED | Read reproduction test; runner test name only | One host, one process |
| Manifest fields for T1 matrix | `eval.rs::run_matrix_with_progress` manifest `json!` | verified in code | Read | |
| Interleaving rule | same function | verified in code | Read | |
| `RunManifestV2` limited and test-only | `src/lab/manifest.rs`; grep `RunManifestV2` hits only manifest.rs, matrix_runner.rs, 3 integration tests | verified in code | Read; grep | matrix_runner.rs hit is from `MatrixRunner` pattern, not RunManifestV2 |
| `AgreementStudy` positional matches, synthetic T2 fixtures | `src/lab/matrix_runner.rs::AgreementStudy::evaluate_concordance`; `tests/integration/lab_matrix_and_agreement_tests.rs::test_t1_t2_agreement_study_concordance` comment | verified in code | Read both | |
| C7 UNVERIFIED | `docs/research/claims_ledger.md` sections 6-7 | reported (ledger) | Read | |
| M6 pipeline matrix 8 x 5 x 10 x 30 = 12,000 | `m6_pipeline.rs::pipeline_*`, `expected_cell_count`; `lab-results/m6_sealed_*/manifest.json` cell_count 12000; cells.csv has 12,001 lines | verified (code + retained file) | Read; PowerShell line count | |
| M6 verdict NOT_SUPPORTED with fields | `lab-results/m6_sealed_2026-10-29T234845Z/verdict.json` | reproduced by reading retained artifact | PowerShell ConvertFrom-Json of H1, falsifiers, H2 | Analysis not re-run; sealed seeds not available |
| M6 per-scenario numbers (-1.57..-3.00, +29.2%, +5.6%, constraints) | `docs/research/m6_sealed_results.md` | reported by project document | Read | Not re-derived from analysis_primary.csv |
| Sealed run on dirty tree (64 status entries), release, Windows, 23:48:45 to 23:50:22 UTC | sealed `manifest.json` | verified (retained artifact) | PowerShell | |
| Preregistration chain and amendment 7b disclosure | `preregistration_m6_v2.md`, `preregistration_m6_v2_2_amendment.md` sections 1, 3, 5, 7b | verified (document read) | Read headings and sections 1, 2, 7b | Freeze JSON hashes not printed |
| Freeze differences 4 of 56 | `docs/release/known_limitations_frozen_code.md` section 2; G4C section 2 | reported (AI second look) | Read | I did not recompute hashes |
| G4C is AI second look; did not read event loop | `known_limitations_frozen_code.md` section 1 | reported | Read | |
| G4C F01/F02 not integrated (U-01 sandbox) | `docs/release/g4/cycle1_findings_disposition_register.md` lines 15, 50 | reported | Grep and read lines | I did not re-test current code behaviour |
| T2 Scenario B design, arms, phases, workload | `src/lab_live/scenario_b.rs::ScenarioBConfig::default_experiment`, `gateway_spec`, `Phases` | IMPLEMENTED | Read | |
| T2 load schedule reuses T1 RNG and AliasSampler | `src/lab_live/loadgen.rs::schedule` | verified in code | Read | |
| Gateway env control, kill_on_drop, snapshot polling | `src/lab_live/gateway.rs::gateway_env`; `scenario_b.rs` (snapshot_interval_ms 250) | verified in code | Read grep output | |
| Mechanical rule order | `src/lab_live/analysis.rs::evaluate_arm` | verified in code | Read | G4C F01 caveat |
| Retained Scenario B numbers | `lab-results/scenario_b/20261030T003932Z/summary.json`, `manifest.json` | reproduced by reading retained artifact | PowerShell JSON | Not re-run; independent recomputation is claimed in `docs/demo/scenario_b_verification/` (not opened) |
| Interpretation notice qualifiers | `docs/demo/scenario_b_interpretation_notice.md`; constants in `scenario_b.rs` | verified (document + code) | Read | |
| Lab API routes, limits, security properties | `src/api/lab_api.rs` header, constants, `router` | IMPLEMENTED; TESTED (tests dir exists, names `review_tests*.rs`) | Read header and route list | Test bodies not opened; handlers not read in full |
| Lab config keys `PHANTOM_LAB_ENABLED`, `PHANTOM_LAB_DIR`, `PHANTOM_LAB_GATEWAY_EXE` | `src/config.rs`, `src/main.rs:439-443` | verified in code | Grep | |
| UI provenance vocabulary | `ui/src/lib/labProvenance.js::TIERS` | IMPLEMENTED | Read | UI not run |
| UI nav entries Simulation / Cluster / Intelligence | `ui/src/lib/routes.js` | verified in code | Grep | |
| AIMD step arithmetic (0.7, +1, 5% error) | `src/intelligence/candidates.rs::OriginConcurrencyCapController::step` | verified in code | Read | 256 clamp (F11) |
| Cap enforced only in canary/active | `src/intelligence/executor.rs::origin_cap_enforcing`; `src/config.rs` warning | verified in code | Read | |
| Serving path calls only gate/permit functions | G4C section 5 | reported (AI second look) | Read | Grep not repeated |
| Router weight constant 1.0 | `src/dist/router.rs` (`capacity_weight = 1.0`) | verified in code | Grep | |
| Anti-entropy worker exists | `src/dist/router.rs::start_anti_entropy_worker`, `reconcile_peer_anti_entropy` | verified in code (symbols exist) | Grep | Effectiveness not established (ledger C6) |
| RT-0 OPEN | `docs/research/rt0_prior_art_sweep_2026-10-30.md` header | reported (document) | Read in full | |
| novelty snapshot correction | `docs/research/novelty_snapshot_m6.md` last section | verified (document) | Read | |
| Facebook leases, XFetch, S3-FIFO | rt0 sweep T3, T4 | literature: OPENED-DIRECT by the sweep author, not by me | Read the sweep's own statements | Not independently opened here |
| Varnish, nginx, Fastly, RFC 9111, Redis tracking, Envoy, Netflix | rt0 sweep T5-T7, T10 | literature: OPENED-VIA-TOOL by the sweep author | Read the sweep | Not re-fetched here |
| groupcache README statements | https://raw.githubusercontent.com/golang/groupcache/master/README.md | literature, OPENED-VIA-TOOL (this session) | WebFetch summary | Tool summary, not verbatim |
| Go singleflight doc sentence | https://pkg.go.dev/golang.org/x/sync/singleflight | literature, OPENED-VIA-TOOL (this session) | WebFetch | same |
| Caffeine README (frequency and recency eviction; refresh-after-write; async loading; page does not name W-TinyLFU and does not mention stampede coalescing) | https://github.com/ben-manes/caffeine | literature, OPENED-VIA-TOOL (this session) | WebFetch | same; absence in the summary is not proof the project lacks the feature |
| Cassandra repair uses Merkle trees | https://cassandra.apache.org/doc/latest/cassandra/managing/operating/repair.html | literature, OPENED-VIA-TOOL (this session) | WebFetch | same |
| Rendezvous hashing original paper citation (Thaler and Ravishankar, ToN 6(1), 1998) | WebSearch result summary (an arXiv paper's reference list, DBLP) | literature, SECONDARY (not opened at primary) | WebSearch | Not verified from the paper itself |
| Redis and Memcached "more mature" statements | general knowledge | assumption | none | Not a repository finding |

## Tests named but whose bodies I did not open

`test_t1_event_engine_total_ordering`, `test_t1_master_rng_seed_splitting_determinism`, `test_t1_stream_independence`, `test_vose_alias_sampler_accuracy_and_o1_speed`, `test_seed_sets_disjointness_and_sealed_leak_detection`, `same_runspec_twice_gives_identical_cells_and_bytes`, `sealed_seed_set_and_family_are_refused_before_any_output`, the `lab_api` review tests. Their names are cited; what they assert is taken from their names and from the G4C test-quality table (`cycle1_lab_intelligence_deps_review.md` section 4).

## Discrepancies found (document versus code or evidence)

1. `src/lab/mod.rs` header says T2 helpers remain in `tests/support/lab`; `src/lab_live/` now implements T2. (`tests/support/lab/` also still exists with older copies; not read.)
2. `VirtualClock` is not used by `run_cell`; event timestamps act as the virtual clock.
3. `matrix_runner.rs::AgreementStudy::evaluate_concordance` doc/comment names Spearman rank concordance; the code counts positional matches. Its test uses synthetic T2 fixtures.
4. `RunManifestV2` documents EV-1..EV-14; only EV-1, EV-2, EV-11 implemented; hard-coded git hash and `is_dirty`; test-only. The manifest real runs write is built in `eval.rs`.
5. `origin_sim.rs` header says M/M/c; M6 uses constant service times.
6. `analysis.rs` header lists BCa for the module; `bootstrap_ci` (percentile) is what `evaluate_paired` and the synthetic validator use.
7. `novelty_snapshot_m6.md` states RT-0 PASS; its own correction notice and the later sweep say OPEN.
8. `claims_ledger.md` section 1 table statuses for C1, C2, C7 are older than section 7's reconciliation.
9. `docs/demo/README.md` describes a demo launcher (Showcase/Explorer modes, `Start-Phantom-Demo.bat`); its relation to the current Lab is unclear. UNVERIFIED relevance.
10. The sealed manifest records 64 git status entries (dirty tree at run time); freeze documents rely on per-file hashes. Not a contradiction, but a reader may assume a clean tree.

## UNVERIFIED items

- Cross-platform and cross-Rust-version determinism of T1.
- That the only wall clock in T1 is `shrink_shard` and does not affect outcomes.
- BCa agreement with a reference implementation.
- Independent recomputation of Scenario B (document `docs/demo/scenario_b_verification/` not opened).
- Re-derivation of per-scenario M6 numbers from `analysis_primary.csv` (I read `verdict.json` only).
- UI behaviour in a browser; Lab API handler behaviour beyond the module header, constants and route list.
- Current status of G4C F01/F02 beyond the register line.
- `docs/release/reproduction_guide.md` content (exists, not read).
- The rendezvous-hashing paper text; Redis/Memcached current documentation.
