# Appendix F: Important source files and their responsibilities

This appendix is a map from files to what they do, which chapter teaches them, and what state they are in. Every path was checked against the working tree on 2026-10-30. The "Status and notes" column uses the handbook's labels (Ch 8, 8.1) and marks files that are libraries with no serving-path caller, legacy, or unused. "Chapter" names the place that teaches the file; where a file is not analysed in any chapter, the entry says so.

Two facts help when reading the tables. First, `src/main.rs` declares its own copy of the module tree and `src/lib.rs` declares another, so the library and the binary compile the same files twice (Ch 2, 2.2.1). Second, the protected core is `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml` (Ch 8, 8.7.1); the chapters did not edit any of it.

## 1. Entry points and configuration

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/main.rs` | Builds the gateway binary `phantom-cache-demo`: resolves configuration, builds the cache with eight shards, the router, the cluster branch and the Lab block, and starts the servers | Ch 2, 2.1 and 2.2 | IMPLEMENTED; builds demo objects (`_traffic`, `_network`, `_db`, `_coalescer`) that serving never uses; `pie: None` |
| `src/lib.rs` | Registers the library modules so tests, benches and the Lab binary can use them | Ch 2, 2.2.1 | Protected core; declares the module tree a second time |
| `src/config.rs` | Parses command line, file and environment; validates the `KEYS` table; fails closed; prints a redacted effective configuration | Ch 2, 2.9.2 | IMPLEMENTED, TESTED; `PHANTOM_DEFAULT_TTL_MS` is `Wiring::NotWired` |
| `src/health.rs` | Liveness and readiness endpoints (`/health/live`, `/health/ready`) | Ch 6, 6.3 | IMPLEMENTED, TESTED by `m7_health_tests`; readiness reflects local state only |
| `src/bin/phantom-lab.rs` | Command line for Phantom Lab; runs the library code without HTTP and refuses the sealed seed set | Ch 7, 7.1 and 7.4.6 | IMPLEMENTED; second binary in `Cargo.toml` |

## 2. src/core: the cache engine

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/core/cache.rs` | `PartitionedCache`: sharded store, single-flight registry, generations, epochs, byte budget, eviction hooks, sweeper | Ch 4, 4.1 to 4.7 | IMPLEMENTED, TESTED; open findings G4A-01, -03, -08, -12, -13, -17; protected core |
| `src/core/coalesce.rs` | `SharedOp` (the shared result handed to waiters) and the legacy `Coalescer` | Ch 4, 4.4 | `SharedOp` IMPLEMENTED; `Coalescer` kept for compatibility, created with an underscore name and unused in serving (Ch 1, 1.3.1) |
| `src/core/eviction.rs` | `ShardEvictionPolicy` and the SIEVE, LRU and S3-FIFO implementations | Ch 4, 4.9 | IMPLEMENTED, TESTED (weakly); S3-FIFO tombstone finding G4A-02 OPEN; no release benchmark |
| `src/core/freshness.rs` | Decides cacheability and lifetime from `Cache-Control`, `Expires` and validators (`evaluate_response_freshness`) | Ch 4, 4.8 | IMPLEMENTED (RFC 9111 subset); conformance gaps G4A-04 to -06 OPEN; no gateway ceiling on lifetime |

## 3. src/api: HTTP surface

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/api/server.rs` | Axum router, GET and DELETE handlers, origin fetch client, control, metrics and SSE handlers, authentication calls | Ch 2, 2.2 to 2.7; Ch 6, 6.2 to 6.5 | IMPLEMENTED, TESTED; body read before authentication (G4B-01, OPEN); two `60_000` literals for the default lifetime; GET is local only |
| `src/api/lab_api.rs` | Lab HTTP API (`/api/v1/lab/*`), off by default, with Host and Origin guard and body timeout | Ch 2, 2.8; Ch 7, 7.7.1 | IMPLEMENTED, TESTED; holds the only `unsafe` blocks in the product (Windows process kill) |

## 4. src/dist: cluster

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/dist/router.rs` | `RendezvousRouter`: picks the DELETE primary, fans out invalidations, repair ledger, reconciliation | Ch 5, 5.3 and 5.5 | IMPLEMENTED, TESTED (loopback, debug); weight constant 1.0; `forward_key_over_tcp` has no production caller |
| `src/dist/node.rs` | `VirtualNode` and `NodeState`; mTLS listener; frame handlers including `InvalidateRequest` and `GetRequest` | Ch 5, 5.2 and 5.8 | IMPLEMENTED; `GetRequest` handler is a stub that fabricates a value (DR-029, PROTOTYPE ONLY); `Degraded` state never set |
| `src/dist/frame.rs` | PhantomMesh binary frame codec (magic, version, 28-byte header, types) | Ch 5, 5.2 | IMPLEMENTED, TESTED by in-file tests and `security_phase3b_framing_tests` |
| `src/dist/pool.rs` | Connection pool with heartbeat probes every five seconds | Ch 5, 5.2 | heartbeat frames and idle-connection keepalive IMPLEMENTED; heartbeat-based failure detection and state change DESIGNED BUT NOT IMPLEMENTED (G4B-12); connections never age out (G4B-08) |
| `src/dist/session.rs` | Tracks inbound cluster sessions and cancels sockets of a revoked peer | Ch 6, 6.4 and 6.11 | IMPLEMENTED; revocation has no production trigger (G4B-07) |
| `src/dist/anti_entropy.rs` | Ring buffer of sent invalidation records and gap tracking for pull reconciliation | Ch 5, 5.5 | IMPLEMENTED, TESTED; memory only, lost with the issuer |

## 5. src/security: authentication and trust

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/security/mod.rs` | `SecurityConfig` and the fail-closed startup checks (`validate_startup_security`, `validate_upstream_startup_security`) | Ch 2, 2.2.1; Ch 6, 6.2 | IMPLEMENTED, TESTED |
| `src/security/jwt.rs` | `JwtValidator`, `Role`; HS256 only; checks signature, issuer, audience, expiry | Ch 3, 3.8; Ch 6, 6.2 | IMPLEMENTED, TESTED; no revocation, no `nbf` check |
| `src/security/cluster.rs` | Cluster mTLS configuration, SAN and fingerprint identity checks, membership registry, revocation flag | Ch 3, 3.7; Ch 6, 6.4 | IMPLEMENTED, TESTED (loopback); `ClusterMembershipRegistry::update` has no production caller |
| `src/security/tls.rs` | `TlsServerConfig`: optional ingress TLS via `axum-server` and rustls | Ch 3, 3.1 and 3.7 | IMPLEMENTED; small file; not separately analysed beyond the TLS 1.3 wording discrepancy in Ch 3, 3.4 |
| `src/security/upstream.rs` | SSRF policy: allowlist, IP filter, DNS pinning (`resolve_and_pin_origin`, `PreConnectIpFilter`) | Ch 6, 6.8 | IMPLEMENTED, TESTED; range gaps G4B-15 OPEN |

## 6. src/intel: telemetry and early prototypes

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/intel/telemetry.rs` | `AtomicLogLinearHistogram` (2,688 buckets) and the sorted-window percentile function | Ch 5, 5.11 | IMPLEMENTED, TESTED (recording and bucket arithmetic only); fed but no production percentile reader, so not the source of displayed percentiles |
| `src/intel/diagnostics.rs` | Cache counters, diagnostic event buffer and the wall-clock time source | Ch 2, 2.2.1; Ch 4, 4.8 | IMPLEMENTED; all time is wall clock (G4A-10) |
| `src/intel/origin_metrics.rs` | `OriginMetrics`: 16-bucket origin latency histogram plus in-flight and error counters | Ch 5, 5.10 | IMPLEMENTED; used by the live path |
| `src/intel/sketch.rs` | Early `CountMinSketch` | Ch 5, 5.12 | Unused: referenced only by its own file and `tests/unit/wp2_tests.rs`; PROTOTYPE ONLY |
| `src/intel/workload.rs` | Early `WorkloadProfile` presets | Ch 5, 5.10 | Unused outside `tests/unit/wp2_tests.rs`; PROTOTYPE ONLY; the Lab workload library is the maintained one |

## 7. src/intelligence: the intelligence library

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/intelligence/mod.rs` | `IntelligenceMode` (off, shadow, canary, active) and the shared state | Ch 1, 1.7; Ch 5, 5.13 | IMPLEMENTED; protected core; default mode `off` |
| `src/intelligence/executor.rs` | `OriginGate` and `acquire_origin_permit` (live path); policy executor, safe fallback and watchdog (library) | Ch 4, 4.4; Ch 5, 5.13 | Origin gate IMPLEMENTED, TESTED; policy methods and watchdog are a library with no caller in `src/` |
| `src/intelligence/candidates.rs` | `OriginConcurrencyCapController` (AIMD) and other candidate policies | Ch 5, 5.13 | AIMD IMPLEMENTED, TESTED (arithmetic and cap unit tests; serving path only when a cap is configured in canary or active mode); benefit NOT SUPPORTED; other candidates not wired |
| `src/intelligence/signals.rs` | `HeavyHitterDetector` (Count-Min plus Space-Saving), hot-key emergence detector | Ch 5, 5.12 | PROTOTYPE ONLY; not called by the gateway; metrics report hot keys unavailable |
| `src/intelligence/shadow.rs` | `ShadowEvaluator` for replaying traffic through candidate eviction policies | Ch 5, 5.13 | PROTOTYPE ONLY; flaw F13 (correlated shadows) |
| `src/intelligence/engine.rs` | `PieDecisionEngine`: compares candidate and baseline from shadow results | Ch 5, 5.13 | PROTOTYPE ONLY; not wired; flaw F12 |
| `src/intelligence/audit.rs` | Bounded circular buffer of decision records (`AuditStore`) | Ch 1, 1.8; Ch 5, 5.10 | The route exists but nothing in the serving path writes to it, so it serves an empty store |

## 8. src/control and src/sim: legacy

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/control/pie.rs` | The legacy `PieController`: threshold rule with a literal `expected_utility` | Ch 5, 5.13; Ch 8, 8.11 | Legacy stub, PROTOTYPE ONLY; `pie: None` in `main.rs`; tests assert its wiring only (`p4_pie_tests.rs`) |
| `src/sim/traffic.rs`, `src/sim/backend.rs`, `src/sim/network.rs` | Early demo traffic generator, backend database model and network link model | Ch 1, 1.3.2; Ch 2, 2.2.1 | Legacy demo code; built in `main::run` and not used in serving |
| `src/sim/clock.rs`, `src/sim/events.rs`, `src/sim/rng.rs`, `src/sim/scheduler.rs` | Early demo clock, event, random-source and scheduler helpers | Ch 2, 2.2.1 | Legacy demo code; not analysed in a chapter; the maintained simulator is `src/lab/` |

## 9. src/lab and src/lab_live: Phantom Lab

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `src/lab/t1_sim.rs` | T1 cell runner: real cache and executor on a virtual clock against a simulated origin | Ch 7, 7.2 | IMPLEMENTED, TESTED; SIMULATED tier; protected and frozen |
| `src/lab/clock.rs` | Lab `VirtualClock` | Ch 7, 7.2.1 | Exercised only by one test (`lab_core_tests`); `run_cell` does not instantiate it. `src/sim/clock.rs` holds a different struct of the same name (Ch 7, 7.2.1) |
| `src/lab/events.rs` | `EventQueue` with time and sequence tie-breaking | Ch 7, 7.2.2 | IMPLEMENTED, TESTED |
| `src/lab/rng.rs` | `MasterRng`, `StreamRng` (SplitMix64), `AliasSampler`, `SeedSets` | Ch 7, 7.2.3 | IMPLEMENTED, TESTED |
| `src/lab/workloads.rs` | Workload families (uniform, Zipf, scan and others) | Ch 7, 7.2.4 | IMPLEMENTED, TESTED |
| `src/lab/origin_sim.rs` | `SimulatedOrigin` with servers, queue and degradation scenarios | Ch 7, 7.2.5 | IMPLEMENTED; model limits listed in the chapter |
| `src/lab/analysis.rs` | Paired differences, BCa bootstrap, sign-flip test, Holm correction | Ch 7, 7.3 | IMPLEMENTED, TESTED; no reference-implementation cross-check |
| `src/lab/eval.rs` | M6 matrix driver, manifests, endpoint analysis | Ch 7, 7.4 | IMPLEMENTED; edited after the sealed run under a pinned addendum |
| `src/lab/m6_pipeline.rs` | The mechanical sealed-evaluation pipeline and `compute_verdict` | Ch 7, 7.4.4 and 7.6 | Executed once; Tier 1 simulated; not human-reviewed |
| `src/lab/runner.rs` | `RunSpec` to `execute`: the library API used by tests, CLI and Lab API | Ch 7, 7.1 and 7.4.6 | IMPLEMENTED, TESTED |
| `src/lab/matrix_runner.rs` | Scenario matrix runner and `AgreementStudy` (T1 versus T2 rank concordance) | Ch 7, 7.5.3 | IMPLEMENTED; no real agreement study run (UNVERIFIED) |
| `src/lab/manifest.rs` | `RunManifestV2` provenance record | Ch 3, 3.9; Ch 7, 7.4.5 | Not the manifest real runs write; hash of unordered maps is fragile (G4C) |
| `src/lab/eviction_baselines.rs`, `src/lab/tuning.rs`, `src/lab/grid_tuning.rs`, `src/lab/report_gen.rs` | Reference eviction baselines, tuning protocol and v2.2 grids, report and provenance formatter | Ch 4, 4.9 and Ch 7, 7.2.7 (context) | IMPLEMENTED; frozen Lab code; these four files are not analysed line by line in the chapters |
| `src/lab/mod.rs` | Lab module header stating what the Lab is not | Ch 2, 2.8; Ch 7, 7.1 | Header comment says it is not a benchmark or production load test |
| `src/lab_live/scenario_b.rs` | Scenario B driver and verdict rule (T2) | Ch 7, 7.5 | TESTED; exploratory; U-01 defect OPEN (fix sandbox only; integration authorised and in progress, not committed) |
| `src/lab_live/gateway.rs`, `src/lab_live/origin.rs`, `src/lab_live/loadgen.rs` | Spawns the gateway child process, scripted raw-TCP origin, open-loop load generator | Ch 7, 7.5.1 | IMPLEMENTED; origin has no load-dependent slowdown |
| `src/lab_live/analysis.rs`, `src/lab_live/http.rs`, `src/lab_live/cli.rs`, `src/lab_live/mod.rs` | Paired analysis and arm verdicts, small HTTP client, `scenario-b` command line, module root | Ch 7, 7.5 | IMPLEMENTED; `cli.rs` and `http.rs` are not analysed line by line in the chapter |

## 10. Tests, benchmarks and test support

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `tests/integration/m1_http_tests.rs` | HTTP-level tests of miss, hit, coalescing and body limits against a mock origin | Ch 2, 2.4 and 2.5 | TESTED; loopback, debug, one gateway |
| `tests/integration/concurrency_verification_tests.rs` | Exhaustive protocol model, mutation detector, stress schedule, proptest | Ch 3, 3.12; Ch 4, 4.5 | TESTED; the model cannot find G4A-01 |
| `tests/integration/invalidation_contract_tests.rs`, `epoch_contention_tests.rs` | DELETE, tenant and global invalidation contracts; epoch reads under contention | Ch 4, 4.7 | TESTED; do not exercise the G4A-01 gap |
| `tests/integration/byte_accounting_tests.rs`, `memory_bound_soak_tests.rs` | Byte-accounting equality and bounded-memory churn | Ch 4, 4.6 | TESTED (consistency only) |
| `tests/integration/stale_serving_tests.rs`, `freshness_matrix_tests.rs`, `revalidation_304_tests.rs` | Stale-if-error, freshness decisions, 304 revalidation | Ch 4, 4.8 | TESTED; stale-cap counter test is weak (G4A-18) |
| `tests/integration/eviction_contract_tests.rs` | Eviction policy contracts | Ch 4, 4.9 | TESTED (weakly) |
| `tests/integration/coalesce_tradeoffs_tests.rs`, `origin_cap_tests.rs`, `serving_contract_tests.rs` | Single-flight trade-offs, origin cap, serving contracts | Ch 4, 4.4; Ch 2, 2.5 | TESTED |
| `tests/integration/distributed_consistency_tests.rs`, `cluster_followup_tests.rs`, `cluster_propagation_latency_tests.rs`, `known_gaps.rs` | Cluster convergence, follow-ups, propagation latency, pinned known gaps (including DR-029) | Ch 5, 5.5 to 5.8; Ch 6, 6.9 | TESTED (loopback, debug, at most four nodes) |
| `tests/integration/security_phase*_tests.rs` and `m7_sec_*_tests.rs` | Phase security tests: tenant isolation, upstream filter, cluster admission, framing, pooling, invalidation, router, anti-entropy, review rounds | Ch 6, 6.2 to 6.8 | TESTED; no independent review |
| `tests/integration/telemetry_redesign_tests.rs`, `telemetry_honesty_tests.rs`, `telemetry_truth_tests.rs` | Histogram, provenance honesty and telemetry truth | Ch 5, 5.11; Ch 2, 2.8 | TESTED |
| `tests/integration/intel_*_tests.rs`, `p4_pie_tests.rs` | Intelligence library tests; legacy PIE wiring | Ch 5, 5.13 | TESTED as library; `p4_pie_tests.rs` asserts stub behaviour |
| `tests/integration/lab_*_tests.rs`, `lab_api/` | Lab determinism, runner, baselines, Scenario B and Lab API tests | Ch 7, 7.2 to 7.7 | TESTED; determinism shown in one environment |
| `tests/unit/wp1_tests.rs` to `wp4_tests.rs` | Early work-package unit tests (including `CountMinSketch` in `wp2_tests.rs`) | Ch 5, 5.12 | TESTED; early prototype tests |
| `tests/support/cluster_harness.rs`, `tests/support/fault_proxy.rs`, `tests/support/lab/` | Cluster test harness, loopback fault proxy, Lab test doubles | Ch 3, 3.12 | Test support; fault proxy is loopback only |
| `benches/b1_hit_path.rs`, `b2_concurrency.rs`, `b3_coalesce.rs`, `b4_eviction.rs`, `b6_telemetry.rs`, `b7_memory.rs`, `b8e_gateway.rs` | Seven custom-main benchmarks (`harness = false`) | Ch 3, 3.12; Ch 6, 6.12 and 6.13 | IMPLEMENTED; no release-profile result established on the current tree |
| `benches/support/alloc_counter.rs`, `benches/support/refpolicies.rs`, `benches/support/mod.rs` | Counting allocator, reference policies, shared bench helpers | Ch 3, 3.12 | Bench support; `alloc_counter.rs` holds a counting `GlobalAlloc` |

## 11. ui, scripts and packaging

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `ui/package.json`, `ui/vite.config.js`, `ui/index.html` | React 18 and Vite 5 project definition; dependency list pinned by a guard test | Ch 3, 3.11 | IMPLEMENTED; one declared dependency unused |
| `ui/src/App.jsx`, `ui/src/main.jsx` | UI entry and routing between the dashboard screens | Ch 1, 1.8; Ch 3, 3.11 | IMPLEMENTED; read, not run, for the chapters |
| `ui/src/screens/` (`Overview.jsx`, `Traffic.jsx`, `CacheMemory.jsx`, `Freshness.jsx`, `Origin.jsx`, `Simulation.jsx`) | Dashboard screens | Ch 7, 7.7.2 | IMPLEMENTED, automated checks only; visual quality not approved |
| `ui/src/screens/lab/` | Lab screens (About, Start a run, Runs, Evidence; `ScenarioBResult.jsx`; `ConnectionPanel.jsx`) | Ch 7, 7.7.2 | IMPLEMENTED; token kept in memory |
| `ui/src/lib/labProvenance.js`, `ui/src/lib/provenance.js`, `ui/src/lib/schema.js`, `ui/src/lib/sse.js` | Provenance labels, schema handling and the fetch-based stream client | Ch 6, 6.3; Ch 7, 7.7.2 | IMPLEMENTED; browser `EventSource` cannot send `Authorization`, so streams use `fetch` |
| `ui/test/` | `node:test` suite (30 `.test.mjs` files plus helpers and fixtures) | Ch 3, 3.12 | TESTED; one fixture masked U-01 (`lab.view.test.mjs`) |
| `ui/scripts/` | Contrast, accessibility, forbidden-pattern checks and Chrome DevTools Protocol journeys | Ch 3, 3.11 and 3.12 | One Chrome, one machine; no screen-reader test |
| `packaging/build-release.sh`, `build-release.ps1`, `verify-checksums.sh` | Release build with `--locked`, archive and checksums | Ch 2, 2.9.3; Ch 3, 3.10 | IMPLEMENTED; does not commit, tag or publish |
| `packaging/smoke-test.sh`, `smoke-test.ps1`, `phantom.example.conf`, `phantom.production.example.conf` | Smoke test of a bundle and example configurations | Ch 2, 2.9.3; Ch 3, 3.10 | IMPLEMENTED; clean-machine reproduction not done |
| `scripts/reproduce_tests.sh`, `reproduce_tests.ps1`, `cleanroom_build.ps1`, `run_audit_suite.ps1` | Test reproduction, clean-room build and audit suite scripts | Ch 3, 3.12 | Present; whether they were run on a clean machine is UNVERIFIED |
| `scripts/gen-demo-certs.sh`, `gen-demo-certs.ps1` | Generate demonstration cluster certificates | Ch 3, 3.7 | Demo use only |
| `scripts/docs-lint.mjs` | Report-only documentation lint tool | Ch 3, 3.12; Ch 8, 8.8 | Committed tooling |
| `.github/workflows/ci.yml` | CI definition: Linux, format, check, clippy, test; frontend build | Ch 3, 3.10 | Present; no `-j 1`, debug profile; whether it ever ran is UNVERIFIED |
| `Start-Phantom-Demo.bat` | Windows demo launcher | Ch 3, 3.10 | Windows only |

## 12. Key documents and registers

| Path | What it does | Chapter | Status and notes |
|---|---|---|---|
| `docs/STATUS.md` | The project's status page: gates, milestones, evidence gaps | Ch 8, 8.2 | Entry point for current state |
| `docs/user/limitations.md` | User-facing list of what is not provided and known gaps | Ch 2, 2.9.3; Ch 8, 8.3 | Authoritative for limitations |
| `docs/user/configuration.md` | Reference for configuration keys; checked against the key table by `m7_config_tests` | Ch 2, 2.9.2 | Does not prove runtime wiring |
| `docs/release/g4/cycle1_findings_disposition_register.md` | Register of the 99 G4 findings and their status | Ch 8, 8.4.1 | Closes nothing; owner statements set dispositions |
| `docs/release/g4/cycle1_core_engine_review.md`, `cycle1_dist_security_api_review.md`, `cycle1_lab_intelligence_deps_review.md` | The three AI review reports (G4A, G4B, G4C) | Ch 8, 8.4.1 | AI second looks; findings confirmed by reading |
| `docs/release/owner_decisions_register.md` | Owner decisions ODR-1 to ODR-4 | Ch 8, 8.4.3 | Sets which findings are recorded, named or held |
| `docs/release/known_limitations_frozen_code.md` | Limitations recorded in frozen Lab code | Ch 7, 7.4.4; Ch 8, 8.4.5 | Includes what G4C did not read |
| `docs/release/verification_register.md`, `docs/release/workstream_completion_ledger.md`, `docs/release/execution_register.md` | Verification and completion records and the delivery plan | Ch 8, 8.1 and 8.8 | Independence classes; none INDEPENDENTLY VERIFIED |
| `docs/release/deferred_integration.md` | Integration items such as D-1 (connection drain) and D-2 (sweeper shutdown) | Ch 8, 8.3.7 | DEFERRED items |
| `docs/research/claims_ledger.md` | Which project claims are supported, narrowed, amended or untraceable | Ch 6, 6.9.3; Ch 7, 7.5.3 | C6 amended; C7 UNVERIFIED |
| `docs/research/m6_sealed_results.md` | The sealed M6 result and its limits | Ch 5, 5.14; Ch 7, 7.6 | Executed once; simulated; not human-reviewed |
| `docs/research/code_findings.md`, `docs/research/pie_audit.md` | Earlier code findings (F-numbers) and the PIE audit | Ch 5, 5.13; Ch 8, 8.2.2 | Prior-agent records, checked by the chapters |
| `docs/research/g4a01_race/` and `docs/research/m3_durability/` | Sandbox analysis, repro attempts and patches for G4A-01 and for durable invalidation | Ch 4, 4.7; Ch 5, 5.9 | Sandbox only; not integrated |
| `docs/research/adr/ADR-001-present-architecture-and-direction.md`, `ADR-002-serving-path-contract.md`, `ADR-003-distributed-architecture.md` | Architecture decision records; ADR-003 amended 2026-10-30 | Ch 3, 3.0; Ch 5, 5.6 | Agent-recorded, not owner-ratified |
| `docs/research/rt0_prior_art_sweep_2026-10-30.md` | Prior-art sweep; RT-0 still open | Ch 7, 7.8 | PARTIAL; no novelty claim |
| `docs/security/m7_security_review.md` | M7 security review and SEC-M7 items | Ch 6, 6.11; Ch 8, 8.4.6 | AI review; some items open |
| `docs/demo/scenario_b_interpretation_notice.md` | Binding notice on how to read Scenario B | Ch 6, 6.12.3; Ch 7, 7.5 | Binding for quoting Scenario B |
| `docs/glossary.md` | The repository's own glossary, including the tier definitions | Ch 6, 6.13 | Source for tier labels |
| `CLAUDE.md` | Project rules for agents, including the code invariants and known gaps | Ch 1, 1.11; Ch 8, 8.7 | Parts are marked outdated inside the file |
| `bench-results/README.md` and `bench-results/r0_summary.json` | Benchmark status and the 2026-10-28 micro-benchmark summary | Ch 6, 6.12 and 6.13 | No release-profile result on the current tree |
| `lab-results/` | Retained Lab output: sealed M6 run, Scenario B runs, dry runs | Ch 7, 7.4.5 and 7.5 | Evidence folders; do not edit |
