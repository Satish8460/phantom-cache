# Chapter 6 evidence file

Chapter: `docs/learning/src/ch06_security_reliability_performance.md`. Written 2026-10-30 (machine clock about 16:15 to 17:00 IST). Tree: branch `feat/publish-phantomcache-core`, uncommitted work over `3e6e03e`. No product file, test or config was edited; no cargo build or test was run; no git command that changes state was run; the sealed M6 directory was not opened; no token, secret or 64-hex string was printed.

Method label for "how verified": **read** = opened the file/symbol in the current tree; **grep** = searched `src/`/`tests/` for the symbol or its absence; **ran** = executed a command (only `node` JSON inspection, `grep`, `ls`, XML well-formedness check were run); **crate source** = read the vendored crate in the local cargo registry.

## 1. Claims table

| Claim | Source file::symbol | Status label | How verified | Limits |
|---|---|---|---|---|
| JWT validation is HS256-only, unknown `kid` rejected without fallback, iss/aud re-checked, 10 s leeway | `src/security/jwt.rs::JwtValidator::validate` | IMPLEMENTED, TESTED | read | tests `m7_jwt_rejects_alg_none_*`, `test_phase1_jwt_unknown_kid_rejected_without_fallback` read by name and header only for most; test bodies not all opened |
| `nbf` is not validated; `Validation::new` leaves `validate_nbf` false | `jwt.rs::validate`; `jsonwebtoken-9.3.1/src/validation.rs::Validation::new` | IMPLEMENTED (gap) | read; crate source (resolves the G4B reviewer's "verify jsonwebtoken 9.x defaults") | applies to the locked version 9.3.1 |
| No `jti`, no max lifetime, no revocation; `iat` parsed, unused | `jwt.rs::JwtClaims` | IMPLEMENTED (absence) | read; grep `iat|jti` in `src/security/jwt.rs` | |
| No test mentions `nbf` | `tests/` | absence | grep `nbf` over `tests/` returned nothing | grep over text only |
| Rotation keys exist but `main.rs` never configures one | `src/security/jwt.rs::with_rotation_key`; `src/main.rs` (`JwtValidator::new` only) | IMPLEMENTED (gap) | read `main.rs` security block | |
| Role aliases (`requester`, `read`, `invalidate`), case-insensitive, bare token accepted | `jwt.rs::has_role`; `server.rs::authenticate_request` | IMPLEMENTED; characterised by test | read; test name `sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09` | |
| Startup fail-closed rules (network bind needs secret, TLS unless override; require-auth needs secret) | `src/security/mod.rs::validate_startup_security`, `validate_upstream_startup_security`; `server.rs::start_server_with_cors` | IMPLEMENTED, TESTED | read | tests `test_phase1_at09_startup_security_fail_closed_rules` by name |
| Short JWT secret on loopback dev is a warning only | `src/config.rs` (secret length block) | IMPLEMENTED (G4B-18 OPEN) | read | |
| Auth is after body extraction in GET, DELETE and control | `server.rs::get_cache_handler` (`_body: axum::body::Bytes`), `delete_cache_handler` (same), `control_handler` (`Json(payload)`) | IMPLEMENTED (gap, G4B-01) | read signatures and first statements | Run-time slow-body exhaustion NOT reproduced: UNVERIFIED as an effect (the G4B reviewer also did not reproduce it) |
| Only layers on the app: `DefaultBodyLimit` 512 KiB and CORS; no timeout/concurrency/connection limit | `server.rs::create_app_with_lab` | IMPLEMENTED | read; grep `TimeoutLayer|ConcurrencyLimit|Semaphore` in `server.rs` found no ingress use | does not cover hyper defaults |
| Body-limit test sends full-speed 600 KiB and expects 413 | `tests/integration/m1_http_tests.rs::test_m1_t26_ingress_request_body_limit` | TESTED (size only) | read test body | proves size cap, not time |
| Lab API authenticates before reading the body, 5 s body timeout, 16 KiB cap, 16 SSE streams, closes stream at token expiry, Host guard in anonymous mode | `src/api/lab_api.rs::runs_start`, consts `MAX_BODY_BYTES`, `BODY_READ_TIMEOUT`, `MAX_SSE_STREAMS`, `Tail::expired` use, `check_lab_bind` | IMPLEMENTED | read | tests in `tests/integration/lab_api/*` not read (names only) |
| `/api/control` NODE_FAILURE sets node 2 Failed; any other or missing event sets all nodes Active | `server.rs::control_handler` | IMPLEMENTED (G4B-03 OPEN) | read | |
| In cluster mode `ApiState.nodes` are the same Arc nodes the router holds | `src/main.rs` (`RendezvousRouter::new(nodes.clone())`, then `nodes` into `ApiState`) | IMPLEMENTED | read | not run against a live cluster; no test (grep `api/control` in tests: role matrix only) |
| Node in Failed/Offline state drops sockets at accept | `src/dist/node.rs::start_tcp_service` accept loop | IMPLEMENTED | read | |
| `/api/events` SSE has no stream cap and no expiry check after open; builds payload every 100 ms | `server.rs::sse_handler` | IMPLEMENTED (G4B-02 OPEN) | read | CPU cost per payload not measured |
| No test of main-API SSE cap/expiry | `tests/` | absence | grep `too_many|MAX_SSE|sse.*cap` | name/text grep |
| No Host/Origin guard in main API | `server.rs` | IMPLEMENTED (absence, G4B-04) | grep `MISDIRECTED|"host"|header::HOST` in `server.rs` found nothing | |
| CORS exact-origin allowlist, no credentials | `server.rs::CorsConfig::layer` | IMPLEMENTED, TESTED | read; tests `sec03_*` by name | |
| Health: live always 200; ready needs config valid + listener probe + not draining; origin/cluster not checked | `src/health.rs`; `main.rs::wait_for_listener` | IMPLEMENTED, TESTED | read | `m7_health_tests` by name |
| UI keeps token in memory, uses fetch-based SSE because EventSource cannot send Authorization | `ui/src/screens/lab/LabScreen.jsx` (`tokenRef`), `ui/src/lib/sse.js`, `labApi.js`, `linkState.js` | IMPLEMENTED | read excerpts; grep `localStorage|sessionStorage` in `ui/src` | UI not reviewed beyond this |
| Cluster TLS: TLS 1.3 only, cluster CA only, client cert required | `src/security/cluster.rs::ClusterTlsConfig::from_pem` | IMPLEMENTED, TESTED | read | `test_p3_sec04_strict_tls13_rejection_of_tls12_client` by name |
| Admission: 64 handshake permits, 1 s timeout, one DNS SAN, `validate_peer` (cluster id, node id, active, fingerprint), session registration, per-frame re-validation, originator equals TLS identity | `src/dist/node.rs::start_tcp_service`, `process_binary_cache_stream`; `cluster.rs::validate_peer`, `parse_san_node_identity` | IMPLEMENTED, TESTED | read | tests by name (`security_phase3_cluster_tests`, `cluster_followup_tests::test_l3_*`) |
| `InvalidateRequest` arm has no membership-epoch check; ack echoes `payload.membership_epoch` | `node.rs::process_binary_cache_stream` | IMPLEMENTED (G4B-06 OPEN) | read; grep `membership_epoch` in node.rs (only the ack construction and the anti-entropy check) | |
| Tenant/FullCache scope accepted from any admitted member; tenant strings unvalidated | `node.rs` InvalidateRequest arm | IMPLEMENTED (G4B-05, DR-027 OPEN) | read | |
| Epoch map capped at 10,000 with global-epoch fallback; each tenant invalidation clones the map under the mutex | `src/core/cache.rs::try_invalidate_tenant`, `MAX_TENANT_EPOCHS` | IMPLEMENTED, TESTED | read | tests prove safety not cost; cost not benchmarked |
| Registry read once; no production caller of `update`; revocation machinery tested | `cluster.rs::load_membership_from_file`, `update`; grep `\.update(` over `src/` for registry | IMPLEMENTED (gap, G4B-07 OPEN) | read; grep | |
| Pool: max 8 total, 4 idle, 30 s idle, 5 s heartbeat; connections not aged out | `src/dist/pool.rs::PoolConfig::default` | IMPLEMENTED (G4B-08 OPEN) | read | |
| Accept loop `Err(_) => break` | `node.rs` accept loop | IMPLEMENTED (G4B-10 plausible part CONFIRMED by reading) | read | no accept-error test |
| Plaintext cluster listener fallback exists when no cluster TLS | `node.rs` (`else` branch) | IMPLEMENTED | read | G4B-17 note on startup check address from register, not re-read in `security/cluster.rs::validate_cluster_startup_security` (read: it takes a bind addr) |
| Frame: 28-byte header, bad magic fails fast, 1 MiB payload cap before allocation, 4,096-byte key | `src/dist/frame.rs` consts, `read_frame` | IMPLEMENTED, TESTED | read | `security_phase3b_framing_tests` by name |
| GetRequest stub fabricates `val_node_<id>`, stores it under `__default__`; rejects keys starting with `/` | `node.rs` GetRequest arm; `known_gaps.rs::pin_kg6_*` | IMPLEMENTED, TESTED (pin) | read both | DR-029 investigation conclusions are AI and MEASURED on debug loopback, not independent |
| Fabricated entry unreachable from HTTP (leading-slash rule) | `node.rs`; `server.rs` `norm_path`; `m7_sec_round3_tests::m2_cluster_get_request_keys_*` | TESTED | read DR-029 report summary + code | |
| `CacheKey::validate` rules (tenant alphabet/len, path/query caps, control bytes, dot segments, `:?`, canonical length) | `src/core/cache.rs::CacheKey::validate` | IMPLEMENTED, TESTED | read | |
| `validate()` accepts the literal `__default__`; secure mode refuses it | `cache.rs::validate` (no check); `server.rs::require_tenant_claim` | IMPLEMENTED | read both | resolves the G4A-19 / G4B refuted-item-5 apparent conflict (different paths) |
| Secure mode ignores `x-tenant-id`; tenant from token | `server.rs::get_cache_handler`, `delete_cache_handler` | IMPLEMENTED, TESTED | read | |
| Only gateway-chosen headers go upstream; `x-cache-bypass` never forwarded | `server.rs::fetch_origin_raw` | IMPLEMENTED, TESTED | read | `m7_sec_round3_tests` by name |
| Origin client: 5 s connect, 5 s handshake, 5 s read, `take(MAX_TOTAL_RESPONSE_BYTES+1)`, 64 KiB/2 MiB caps, `Connection: close` | `server.rs::fetch_origin_raw`, consts | IMPLEMENTED | read | |
| `UpstreamPolicy::validate_target`, hard-blocked ports, `resolve_and_pin_origin` checks every address and pins the first | `src/security/upstream.rs` | IMPLEMENTED, TESTED | read | range gaps G4B-15 from register; ranges not independently enumerated |
| Origin permit gate only when cap configured and mode canary/active; off mode uncapped | `config.rs` key text for `PHANTOM_ORIGIN_CAP_MAX`; `server.rs::acquire_origin_permit` (None executor => no permit); test name `origin_cap_tests::off_mode_is_uncapped` | IMPLEMENTED, TESTED | read config text and server fn | the executor mode check itself read only via config text |
| No rate limiter in `src/` | grep `ratelimit|rate_limit|limiter|governor` | absence | grep (excluding `src/lab`) | text search |
| Waiters per key 1,000 | `cache.rs::max_waiters_per_key` default | IMPLEMENTED | read | |
| DELETE order: auth, key, local `cache.delete`, router call; status mapping 204/503/500/504; `x-cache-warning: PrimaryFailover` | `server.rs::delete_cache_handler` | IMPLEMENTED | read | |
| Comment calls this "linearizability" | `delete_cache_handler` comment | discrepancy (overclaim in comment) | read | see discrepancies |
| Primary leg 150 ms default; acting-primary failover; owed recording | `src/dist/router.rs::cluster_timeout_ms`, `invalidate_key_across_cluster` | IMPLEMENTED | read | |
| Repair ledger caps (1,024 entries, 256 KiB, 64 per tick, 16 tenant flushes); per-peer queue 256, in-flight 1 | `router.rs` consts | IMPLEMENTED, TESTED | read consts; tests by name | |
| Anti-entropy worker started with 250 ms | `src/main.rs` (both cluster branches) | IMPLEMENTED | read | effectiveness not established (claims ledger) |
| Shutdown does not drain the router; ledger in memory | `src/main.rs` select on `shutdown_signal` (comment: "cache is in-memory, nothing to flush"); grep `stop` in router (stop fns exist, not called there) | IMPLEMENTED (gap, G4B-09) | read | |
| No test kills the issuer | `tests/` | absence | grep `issuer crash|kill-issuer|kill -9` over `tests/` returned nothing; G4B report agrees | a test could exist under another name; UNVERIFIED to that extent |
| Propagation latency measurements | `docs/research/claims_ledger.md` C6 amendment | reported by prior agent (MEASURED by them, debug, loopback) | read ledger | raw logs in `D:\Temp` not retained: UNVERIFIED; not re-run |
| G4A-01: `finalize_fetch` compares placeholder epochs to current, then re-reads epochs and stamps entry | `cache.rs::finalize_fetch` | IMPLEMENTED (hole present, UNDER-INVESTIGATION) | read the function in the current tree | reproduction only in sandbox (status.md, AI) |
| Sandbox proposal not applied to `src/` | `docs/research/g4a01_race/status.md`; code read shows re-read still present | PROTOTYPE ONLY | read | |
| M3 durable log is design + scratch prototype only | `docs/research/m3_durability/design.md` first lines | PROTOTYPE ONLY | read | |
| `unsafe` only in `lab_api.rs` Windows module (5 blocks, 2 extern blocks) and bench alloc counter | `src/api/lab_api.rs`, `benches/support/alloc_counter.rs` | IMPLEMENTED | grep `unsafe|extern "system"|extern "C"` over `src/`, `benches/`, `tests/` | third-party crates not audited |
| 288 locked packages; direct deps and feature observations | `Cargo.toml`, `Cargo.lock` | IMPLEMENTED | read Cargo.toml; `grep -c '^\[\[package\]\]'` = 288 | advisory status UNVERIFIED (no scan) |
| CI workflow has fmt, check, clippy -D warnings, tests, frontend job; no advisory step | `.github/workflows/ci.yml` | IMPLEMENTED | read first 60 lines | remainder of file not read; whether CI ever ran UNVERIFIED |
| Percentile rules per place | `telemetry.rs` (histogram ceil at `AtomicLogLinearHistogram::calculate_percentiles_us`; ring floor at `TelemetryCollector::calculate_percentiles_us` and `_v2`), `loadgen.rs::percentile` (ceil), `t1_sim.rs::percentile` (ceil), `executor.rs::record_observation` (ceil), `signals.rs` (floor) | IMPLEMENTED | read each | signals.rs line read as one excerpt |
| Live `latency_ms` uses the 10,000-observation ring, not the histogram; histogram percentile fns have no non-test caller in `src/` | `server.rs::build_schema_v2_payload` line calling `tel.calculate_percentiles_us()`; `main.rs` `TelemetryCollector::new(10000)`; grep `calculate_percentiles_histogram` | IMPLEMENTED | read, grep | tests may call the histogram methods (not grepped in `tests/`) |
| `hit_ratio_cumulative` = hits/(hits+misses) since start, "derived"; "unavailable" with no data; ring `calculate_hit_rate` returns 1.0 with no data | `server.rs::build_schema_v2_payload`; `telemetry.rs::calculate_hit_rate` | IMPLEMENTED | read | mixed counting of coalesced/stale from register (G4A-11), not re-derived |
| R0/R-A/R-B/R-C benchmark numbers | `bench-results/r0_summary.json`, `manifests/*.json` | MEASURED (local, dated), reported by prior agent AG | ran `node` to print medians/CIs (src_hash fields not printed) | release profile inferred from runner script path `target/release/deps`; tree `456bb25` dirty; not re-run |
| Benchmark workload: 100,000 keys, Zipf 0.8 via fixed xorshift, 2 s warm-up, 5 s windows; B8e 16 clients, closed-loop | `benches/b2_concurrency.rs`, `b8e_gateway.rs`, `bench-results/tools/benchmarks_runner.py` (bootstrap seed 42), `r0_summary.json` `limitations_verbatim` | IMPLEMENTED | read | fixed RNG state not a parameter; value deliberately not printed in the chapter |
| Between-session spread: telemetry-off throughput 75.9 k, 77.6 k, 60.9 k, 79.2 k req/s (R0, RA, RB, RC) | the four manifests | MEASURED | ran `node` over manifests | |
| Scenario B figures (cap reductions, success costs, config, seeds, phases) | `lab-results/scenario_b/20261030T003932Z/summary.json`, `manifest.json`; `docs/demo/scenario_b_interpretation_notice.md` | MEASURED_LOOPBACK, label VALID per file | ran `node` on summary/manifest (hash fields not printed); read notice | qualifications in the notice apply; raw `runs/` not opened |
| `docs/testing/*.md` are historical audits of the simulation-era demo | headers of the five files | n/a | read headers (each has a 2026-10-30 notice) | |

## 2. G4 findings re-checked against the current tree

For every finding the chapter cites, the code at the cited symbol was read in the current tree; line numbers in the reviews have drifted and were not used. None was found fixed. "Present" means the mechanism the finding describes is still in the code; it does not mean an exploit was run.

| Finding | Disposition in register (last review 2026-10-30 15:42 IST) | Check performed | Result |
|---|---|---|---|
| G4A-01 | UNDER-INVESTIGATION | read `finalize_fetch` | present (second epoch read) |
| G4A-02 | OPEN (High, conditional) | grep callers of `set_eviction_policy`/`apply_policy` | S3-FIFO selectable only via `executor::apply_policy` (callers: `lab/t1_sim.rs`) and legacy `control/pie.rs` (`pie: None`); not read: `eviction.rs::on_remove` itself |
| G4A-08, G4A-17, G4A-12 | OPEN | read epoch functions | G4A-12 clone present; G4A-08/17 not independently re-derived |
| G4A-19 | OPEN | read `validate`, `require_tenant_claim` | partly: secure HTTP path refuses; validator accepts literal |
| G4A-11, G4A-24, F24 | OPEN | read telemetry percentile and hit-rate code | present |
| G4B-01 | OPEN, owner-named | read handler signatures, `create_app_with_lab` | present |
| G4B-02 | OPEN | read `sse_handler` | present |
| G4B-03 | OPEN, owner-named | read `control_handler`, `main.rs` | present |
| G4B-04 | OPEN | grep Host guard | absent |
| G4B-05 | OPEN, owner-named | read node InvalidateRequest arm, `try_invalidate_tenant` | present |
| G4B-06 | OPEN, owner-named | read node arm | present |
| G4B-07 | OPEN, owner-named | grep registry `update` callers | no production caller |
| G4B-08 | OPEN, owner-named | read pool config/`last_used_at` | present (idle refresh only) |
| G4B-09 | OPEN | read `main.rs` shutdown, `invalidate_key_across_cluster` | present |
| G4B-10 | OPEN | read accept loop | accept error breaks loop; semaphore 64/1 s |
| G4B-11 | OPEN | grep limiter; read config key text | present |
| G4B-13, G4B-14 | OPEN | read `jwt.rs`; crate source for `nbf` | present |
| G4B-15 | OPEN | not re-enumerated | taken from register and M7 review |
| G4B-20 | OPEN | read `tls.rs` | consistent (defaults) |
| G4B-30 / DR-029 | OPEN | read GetRequest arm and pin test | present |
| G4C-DEP | OPEN | read lock and Cargo.toml | no scan evidence in repo |
| F01 | IN-SANDBOX-REMEDIATION | read `docs/research/u01_backend/status.md` | sandbox only, not integrated |

## 3. Figure verification

- Figure 6.1 (`assets/ch06_trust_boundaries.svg`): routes list checked against `create_app_with_lab` route definitions; roles against each handler's `authenticate_request` call; zone facts against `main.rs` (single `ORIGIN_URL`, cluster branch) and `node.rs`. Default byte budget 256 MiB from `config.rs` key table. XML well-formedness checked with PowerShell `[xml]`.
- Figure 6.2 (`assets/ch06_request_path_protections.svg`): step order checked against `get_cache_handler` (body extractor in signature, auth, tenant, key validation, bypass decision, lookup) and `fetch_origin_gated`/`fetch_origin_raw` (permit, policy, pin, timeouts); status mappings checked against `map_origin_error_to_response` (400 key errors, 414 URI too long, 503 cap/waiter, 502/504). The 413 comes from `DefaultBodyLimit`. Step 8 text about the stale cap is from CLAUDE.md section 5 plus `cache.rs` (the hard cap is enforced in the execution layer; `stale_cap_violations` near-tautological per G4A-18: not drawn).
- Figure 6.3 (`assets/ch06_lost_invalidation_flow.svg`): DELETE order from `delete_cache_handler`; 150 ms, failover and owed recording from `invalidate_key_across_cluster`; queue 256, ledger caps, 64 per tick from `router.rs` consts; "promotion waits for repaid debt" is from test names only (stated in the figure); crash branch from `main.rs` shutdown and the claims ledger C6 qualifier.

## 4. Discrepancies found (document versus code, or document versus document)

1. `delete_cache_handler` comment says the local-first purge "Guarantees linearizability". Code and tests support only per-node read-your-delete on the issuer; CLAUDE.md section 4 forbids unqualified linearizability claims. (comment only; not fixed)
2. CLAUDE.md section 5 ("Telemetry Hot-Path: `AtomicLogLinearHistogram` ... bounded error <= 0.39%") describes the histogram, but the live `latency_ms` percentiles served by schema 2 come from the raw 10,000-observation ring with floor-index percentiles; the histogram's percentile methods have no non-test caller in `src/` (grep). The error bound therefore does not describe the served percentiles.
3. Register discrepancy 1 (G4A-19 versus the G4B refuted item 5) is resolvable: both are true on different paths (secure HTTP refuses `__default__`; `CacheKey::validate` and peer frames accept it; dev mode header accepts it).
4. `main.rs` startup message for the mTLS cluster branch says the local node shares the HTTP cache (current), while `src/config.rs` text and an older `main.rs` message elsewhere say otherwise (G4B-32, as reported; the `config.rs:858` text was not re-opened here). CLAUDE.md section 6 already marks the old statement outdated.
5. `src/security/mod.rs` and `server.rs` log "TLS 1.3" for ingress but `TlsServerConfig` applies library defaults (G4B-20). Cluster TLS is 1.3-only.
6. Claims ledger C6 original text "staleness bounded by TTL 60 s" is contradicted by its own qualifier (literal 60_000 only without origin freshness; `PHANTOM_DEFAULT_TTL_MS` not wired; no gateway ceiling on `max-age`). Chapter uses the qualified form.
7. M7 review calls rounds 2 and 3 "independent review"; this handbook's style rule reserves INDEPENDENTLY VERIFIED for human or external review. Not established who performed them: the chapter avoids the label.
8. CLAUDE.md section 3 says "242 tests, 19 targets" (2026-10-28); `docs/testing/strategy.md` counts 75 targets in a later sweep. Not reconciled here (Chapter 8 owner).
9. `bench-results/README.md` says "Profile and workload per file were not checked (unverified)". This chapter checked the runner script (release binary path) and the bench sources for workload; the profile claim remains an inference from the script path.

## 5. UNVERIFIED items

- Run-time effect of G4B-01 (slow-body exhaustion), hyper/axum defaults at pinned versions.
- Run-time behaviour of `/api/control` against a live cluster; of G4B-10 accept-error behaviour.
- Whether any test kills the issuer (none found by text search).
- Raw propagation-latency logs (under `D:\Temp`, non-durable) and the numbers themselves (read from the ledger, not re-run).
- Release-profile status of the R0..RC bench binaries (inferred); that they are not representative of the current tree.
- `bench-results/cluster/characterization_report.json` content (not opened in detail).
- Contents of test bodies other than those explicitly read: most test claims are by name and header.
- Win32 FFI correctness beyond reading; third-party `unsafe`.
- Whether CI has ever executed; advisory scan status of any dependency.
- The five G4B-15 range gaps (taken from the register and M7 review, not enumerated here).
- Idle CPU after the D-2 sweeper fix (PERF-M7-01) not re-measured.

## 6. Cross-chapter notes

- Chapter 4 should own the explanation of epochs/fencing mechanics and the G4A-01 detail; Chapter 6 refers to them and gives the security view. If Chapter 4 describes `finalize_fetch` differently from "second epoch read after the fence check", reconcile against `cache.rs::finalize_fetch` (read 2026-10-30).
- Chapter 5 owns AIMD behaviour (static cap in effect) and routing facts; Chapter 6 only cites Scenario B results.
- Chapter 7 owns Scenario B and M6 in depth; the measurement table in Chapter 6 section 6.13 must stay consistent with its numbers (peak concurrency -38.3 percent CI 36.7 to 40.2, success cost 4.9 pp for STATIC-CAP; AIMD arm -64.0 percent, 17.1 pp).
- Chapter 8 owns the findings summary by theme; counts used here (99; 75 open including 6 owner-named; 20 recorded limitations; 2 sandbox remediation; 1 under investigation; 1 refuted; 0 Critical; 4 High) are from the register section 3.
- Chapter 2 should use the same route list and the same statement "GET is served from the local cache only; DELETE routes by rendezvous".
