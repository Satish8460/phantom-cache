# PhantomCache — Phase Implementation File Map (navigation layer for AG)

**This file maps work to files. It does not authorize anything.** A phase is authorized only by the owner's written decision after the previous gate (`phase_execution_packages_index.md` CER-1). A mapped work package is not thereby ready, permitted, or implemented. This map complements — and never replaces — the execution packages and the code itself.
**Maintenance owner:** AG updates mapping/execution status per §8; the research lead reviews changes. **Map version:** 1.0 (created 2026-10-29).

## 0. Evidence snapshot and legend
| Item | Value |
|---|---|
| Inspection time | 2026-10-28T19:50Z (read-only) |
| Committed baseline | HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core` |
| Working-tree state | 34 `git status` entries: 16 tracked-modified (`Cargo.lock`, `Cargo.toml`, `README.md`, `docs/PHASE_2_WORK_REPORT.md`, `src/api/server.rs`, `src/control/pie.rs`, `src/core/cache.rs`, `src/core/coalesce.rs`, `src/dist/node.rs`, `src/dist/router.rs`, `src/intel/telemetry.rs`, `src/lib.rs`, `src/main.rs`, `src/sim/traffic.rs`, `tests/unit/wp1_tests.rs`, `tests/unit/wp4_tests.rs`) and 18 untracked entries (`CLAUDE.md`, `MILESTONE_1_DESIGN_PLAN.md`, `bench-results/`, `benches/`, `docs/demo-lab/`, `docs/handoff/`, `docs/milestone_1/`, `docs/research/`, `docs/security/`, `scripts/`, `src/dist/{anti_entropy,frame,pool,session}.rs`, `src/intel/diagnostics.rs`, `src/security/`, `tests/integration/`, `transcript_copy.jsonl`). |
| **Consequence** | **HEAD alone does not contain the current code:** `src/security/`, four `src/dist` files, `src/intel/diagnostics.rs` and the whole `tests/integration/` directory are untracked, so the committed baseline cannot build the current suite. The working tree (and the Phase 1 snapshot) is the only copy. "Baseline" in this map means the working tree at inspection time, not HEAD. |
| Phase 1 progress (AG's own log, **not verified here**) | `phase_1_progress.md`: S0–S3 reported COMPLETE; S4–S11 PENDING. New AG files at inspection: `tests/integration/known_gaps.rs` (642 lines), `benches/b1…b8e` + `benches/support/{mod,alloc_counter,refpolicies}.rs`, `bench-results/{logs,manifests,raw,tools,tooling_report.md}`, Cargo dev-deps `criterion`, `hdrhistogram`, `proptest`, 7 `[[bench]]` entries. |
| Not read by the author | `frame.rs`, `pool.rs`, `session.rs`, `security/{cluster,jwt,upstream}.rs`, most integration tests (roles taken from `docs/handoff/04` [agent-reported]), `docs/handoff/02,03` bodies. |
**Path classifications:** **VERIFIED** (inspected in the working tree; scope of inspection stated) · **EXISTS_UNVERIFIED** (exists; role not confirmed by reading) · **PROPOSED** (named by a package; confirmed absent at inspection; not created) · **MISSING** (referenced but not found) · **CONDITIONAL** (relevant only if a stated decision is taken). Tree state: **T-M** tracked-modified, **T-U** tracked-unmodified (not in `git status`), **U** untracked.
**Execution statuses:** NOT AUTHORIZED · BLOCKED · READY · IN PROGRESS · COMPLETE · NOT RUN. **Mapping statuses:** MAPPED (all paths classified), PARTIAL (some paths unverified/conditional), UNMAPPED.

## 1. AG lookup workflow (required before each work package)
1. Read the applicable execution package section and the WP's acceptance criteria. 2. Find the WP in §5 of this map. 3. Verify every mapped path against the current tree (`git status --short`, `ls`, open the file); record differences in the progress log and §8 change log. 4. Inspect callers, callees, contracts and relevant tests before editing (grep for symbol uses; do not rely on this map's descriptions). 5. Check §6 (shared files) and §7 (gated/prohibited) for conflicts; if two WPs touch one file, follow the stated sequencing. 6. Update this map when discoveries change the plan (PROPOSED→EXISTS, wrong mapping, new dependency). 7. Implement only work the owner has authorized and whose prerequisites (gates) have passed. 8. Run the package's validation commands, store evidence per the package, never in this map. 9. After the WP, update its mapping/execution status here and report new files, unexpected files and unresolved risks in the completion report. **If this map and the code disagree, the code wins; if this map and a package disagree, the package wins — then fix the map.**

## 2. Repository architecture map (code-grounded)
**Crates/targets (VERIFIED, `Cargo.toml`):** package `phantom-cache-demo`; **library** `phantom_cache_demo` from `src/lib.rs` (tests/benches import this name); **binary** `phantom-cache-demo` from `src/main.rs`. **`src/main.rs` re-declares the module tree inline (`pub mod sim {…}` … `pub mod security;`) separately from `src/lib.rs`**, so the binary and the library compile separate copies of the modules (unit tests inside modules run twice). Any new module must be registered in both. No `[features]` table and no `[workspace]` exist. 19 original test targets + `known_gaps` declared as `[[test]]`; 7 `[[bench]]` (harness=false) added by AG.
**Serving path as built (VERIFIED by reading):**
```mermaid
flowchart LR
  C[Client] -->|GET/DELETE /api/v1/cache/*| H[api/server.rs handlers\ncreate_app, get_cache_handler, delete_cache_handler]
  H -->|begin_fetch_atomic / finalize_fetch / delete| PC[(HTTP PartitionedCache\nmain.rs: new(8))]
  H -->|miss: fetch_origin_raw| O[(Origin)]
  H -->|record| TEL[TelemetryCollector + CacheCounters]
  TEL --> SSE[/api/events SSE\nsnapshot + live counters/]
  SSE --> UI[ui/src/App.jsx]
  UI -.->|POST /api/control\n(simulation state only)| H
  SIMLOOP[spawn_simulation_loop\nsynthetic metrics] --> SSE
  MAIN[main.rs wiring] --> H
  MAIN -->|pie: None| H
  H -->|DELETE only, if router Some| R[RendezvousRouter\ninvalidate_key_across_cluster]
  R -->|mTLS TCP frames| N[VirtualNode listener\nnode.rs]
  N --> NC[(node.cache\nSEPARATE PartitionedCache::new(4))]
  PC -. "no link found in src/main.rs" .- NC
  PIE[control/pie.rs\nlegacy threshold stub] -. "constructed only in tests\nand sim loop if Some" .-> SIMLOOP
  SK[intel/sketch.rs CountMinSketch] -. "used only by wp2_tests" .- PIE
```
Solid edges = verified in source. Dashed = absent/disconnected or conditional. **GET never uses the router; peer invalidations purge `node.cache`, not the HTTP cache; `reconcile_peer_anti_entropy` has no `src/` caller; the PIE is not constructed in `main()`; `WorkloadProfile` (`intel/workload.rs`) has no users in `src/`, `tests/` or `benches/`.**
| Area | Real components (classification) | Intended but not integrated |
|---|---|---|
| Entry & HTTP | `src/main.rs` (VERIFIED partial: env wiring, caches, router, ApiState, `pie: None`), `src/api/server.rs` (VERIFIED partial) | config validation module, readiness/metrics endpoints (M7, PROPOSED) |
| Cache core | `src/core/cache.rs` (VERIFIED full), `src/core/coalesce.rs` (VERIFIED lines 1–150) | freshness engine, eviction trait, byte accounting (M1/M2) |
| Telemetry | `src/intel/telemetry.rs` (VERIFIED), `src/intel/diagnostics.rs` (VERIFIED public API), SSE in `server.rs` | schema v2, origin metrics (M1 S7) |
| Distributed | `src/dist/{node,router}.rs` (VERIFIED partial), `anti_entropy.rs` (VERIFIED), `frame/pool/session.rs` (EXISTS_UNVERIFIED), `src/security/{mod,tls}.rs` (VERIFIED partial) + `cluster/jwt/upstream.rs` (EXISTS_UNVERIFIED) | serving-path integration (M3 decision) |
| Origin interaction | `fetch_origin_raw`, `parse_http_response`, `decode_chunked_body` — **private fns inside `server.rs`** (VERIFIED by index) | origin metrics (M1), timeouts contract (M2) |
| Phantom Intelligence / PIE | `src/control/pie.rs` (VERIFIED), `src/intel/sketch.rs`, `workload.rs` (VERIFIED; unused) | `src/intelligence/**` (M5, PROPOSED) |
| Lab / simulation / bench | `src/sim/*` (clock/events/network/rng/scheduler/backend heads VERIFIED; `traffic.rs` VERIFIED); `main.rs` imports `sim::{backend,clock,network,rng,traffic}` (use lines VERIFIED; runtime use unverified); tests using `sim`: `p3_traffic_tests`, `p4_pie_tests`, `wp1_tests`; AG's `benches/` | `lab/**`, `lab-results/` (M4, PROPOSED) |
| UI | `ui/src/App.jsx` (524 lines, VERIFIED partial), `ui/src/main.jsx`, `ui/index.html`, `ui/vite.config.js` (VERIFIED), `ui/package.json` | components/lib/screens/styles (M1 S8, M8, PROPOSED) |
| Tests/CI/scripts | `tests/unit/*`, `tests/integration/*`, `.github/workflows/ci.yml` (VERIFIED partial: push/PR to `main`; ubuntu; `cargo fmt --all -- --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test --all-targets --all-features`), `scripts/gen-demo-certs.{ps1,sh}` (EXISTS_UNVERIFIED), `Start-Phantom-Demo.bat` (VERIFIED head) | packaging scripts (M7, PROPOSED) |
| Docs | `docs/handoff/*` (00,01,04,05,07 read; 06,08 partly), `docs/security/*`, `docs/implementation/*` (WP2 read), `docs/demo-lab/*` (capability audit partly), `docs/research/*` | `docs/design/**` (M8, PROPOSED) |

## 3. File catalog (aliases used in §5)
**Source (all under `src/`).** Role = confirmed responsibility.
| Alias | Path | Tree | Class | Responsibility (confirmed) | Key verified symbols |
|---|---|---|---|---|---|
| S-MAIN | `src/main.rs` | T-M | VERIFIED (partial: module block, lines ~55–310) | process entry; env/config parsing; builds `PartitionedCache::new(8)`, `TelemetryCollector::new(10000)`, 3 demo `VirtualNode`s, cluster router (mTLS) or `None`; upstream policy; JWT security; `ApiState{…, pie: None, router}`; starts server | `main`, `ApiState` construction |
| S-LIB | `src/lib.rs` | T-M | VERIFIED | library module tree (40 lines) | `pub mod sim/core/intel/control/dist/api/security` |
| S-SRV | `src/api/server.rs` | T-M | VERIFIED (partial: 96–300, 740–780, 812–915, 925–955, 1566–1716; fn index) | routes (`/health`, `/api/events`, `/api/control`, `/api/v1/cache[/*key]`, `/api/v1/diagnostics/{summary,events}`), GET/DELETE handlers, SSE, auth, origin fetch + HTTP parsing (private), simulation loop | `create_app`, `start_server`, `ApiState`, `get_cache_handler`, `delete_cache_handler`, `spawn_simulation_loop`, private `fetch_origin_raw/parse_http_response/decode_chunked_body` |
| S-CACHE | `src/core/cache.rs` | T-M | VERIFIED | sharded cache, key states, generation fencing, tenant/global epochs, eviction functions, fills/deletes | `PartitionedCache`, `CacheKey`, `CachedResource`, `KeyState`, `CacheShard`, `begin_fetch_atomic`, `finalize_fetch`, `delete*`, `evict_*_from_shard`, `get_effective_epochs`, `set_capacity_limits` |
| S-COAL | `src/core/coalesce.rs` | T-M | VERIFIED (1–150) | legacy `Coalescer`; `OriginError`, `FinalizeOutcome`, `SharedOp`, `WaiterBatch`, `FetchRegistration`, `OriginFetchGuard` | those types |
| S-TEL | `src/intel/telemetry.rs` | T-M | VERIFIED | `RawObservation`, `TelemetryCollector` (mutex ring, sorted percentiles) | `record`, `calculate_percentiles`, `calculate_hit_rate` |
| S-DIAG | `src/intel/diagnostics.rs` | U | VERIFIED (public API) | `CacheCounters`, snapshots, `DiagnosticBuffer`, event fingerprints | `CacheCounters`, `DiagnosticBuffer::now_ms` |
| S-SKT | `src/intel/sketch.rs` | T-U | VERIFIED | Count-Min sketch (unused by product code) | `CountMinSketch` |
| S-WL | `src/intel/workload.rs` | T-U | VERIFIED | `WorkloadProfile` presets (no users found) | `WorkloadProfile` |
| S-PIE | `src/control/pie.rs` | T-M | VERIFIED | legacy threshold controller (see `pie_audit.md`) | `PieController`, `PieDecision` |
| S-NODE | `src/dist/node.rs` | T-M | VERIFIED (24–64, 380–510) | `VirtualNode` (own cache), TCP listener, GetRequest/Invalidate/Heartbeat/AntiEntropy handlers | `VirtualNode::{new,start_tcp_service,stop_tcp_service,set_artificial_delay}` |
| S-ROUT | `src/dist/router.rs` | T-M | VERIFIED (1–330, 505–897) | HRW routing, `invalidate_key_across_cluster`, `ReplicaBroadcaster`, `reconcile_peer_anti_entropy` | those |
| S-AE | `src/dist/anti_entropy.rs` | U | VERIFIED | `InvalidationRingBuffer`, `AntiEntropyTracker` | `record_inbound_seq`, `get_highest_acked_seq` |
| S-FRAME | `src/dist/frame.rs` | U | EXISTS_UNVERIFIED | binary framing/codecs (per handoff) | — |
| S-POOL | `src/dist/pool.rs` | U | EXISTS_UNVERIFIED | mTLS connection pool/heartbeat (per handoff) | — |
| S-SESS | `src/dist/session.rs` | U | EXISTS_UNVERIFIED | session tracking/revocation (per handoff) | — |
| S-SEC | `src/security/mod.rs` | U | VERIFIED (head) | `SecurityConfig`, startup validation | `validate_startup_security`, `validate_upstream_startup_security` |
| S-TLS | `src/security/tls.rs` | U | VERIFIED (head) | ingress TLS config | `TlsServerConfig::from_pem(_file)` |
| S-SCL / S-JWT / S-UP | `src/security/cluster.rs`, `jwt.rs`, `upstream.rs` | U | EXISTS_UNVERIFIED | cluster mTLS/membership, JWT, upstream policy/SSRF (per handoff) | — |
| S-SIM | `src/sim/{clock,events,network,rng,scheduler,backend}.rs` | T-U | VERIFIED (public heads) | virtual clock, event list, network link, RNG sources, scheduler, M/M/1 backend | `VirtualClock`, `SeededRngSource`, `BackendDatabase` |
| S-TRAF | `src/sim/traffic.rs` | T-M | VERIFIED | workload generator (Zipf sampler O(n) per sample) | `TrafficGenerator`, `KeyDistribution` |
**Build/CI/scripts:** B-CARGO `Cargo.toml` (T-M, VERIFIED) · B-LOCK `Cargo.lock` (T-M, EXISTS_UNVERIFIED) · B-CI `.github/workflows/ci.yml` (T-U, VERIFIED partial) · B-CERT `scripts/gen-demo-certs.{ps1,sh}` (U, EXISTS_UNVERIFIED) · B-BAT `Start-Phantom-Demo.bat` (T-U, VERIFIED head).
**Existing tests** (role: VERIFIED where read, else per `docs/handoff/04` [agent-reported]; all `tests/integration/*` are **U**): T-WP1 `tests/unit/wp1_tests.rs` (T-M; cache/SIEVE/flood/coalescing units; uses `sim`; EXISTS_UNVERIFIED) · T-WP2 `wp2_tests.rs` (T-U; VERIFIED: telemetry, sketch, PIE guardrails; **constructs `RawObservation` by struct literal**) · T-WP3 `wp3_tests.rs` (T-U; router units; VERIFIED by imports) · T-WP4 `wp4_tests.rs` (T-M; ApiState/SSE control; VERIFIED by imports) · T-M1 `m1_http_tests.rs` (32 HTTP-path tests: coalescing, DELETE, tenant isolation, size limits; EXISTS_UNVERIFIED) · T-COAL `coalesce_tradeoffs_tests.rs` (5; EXISTS_UNVERIFIED) · T-P2 `p2_dist_network_tests.rs` (7; multi-node TCP; EXISTS_UNVERIFIED) · T-P3 `p3_traffic_tests.rs` (5; traffic/sim + API; EXISTS_UNVERIFIED) · T-P4 `p4_pie_tests.rs` (6; **VERIFIED partial**: PIE state machine, policy comparison, live in-process PIE path) · T-S1 `security_phase1_tests.rs` (15; ingress JWT/RBAC; EXISTS_UNVERIFIED) · T-S2 `security_phase2_upstream_tests.rs` (23; upstream/SSRF/parser; EXISTS_UNVERIFIED) · T-S3 `security_phase3_cluster_tests.rs` (24; mTLS/config; EXISTS_UNVERIFIED) · T-S3B `security_phase3b_framing_tests.rs` (13) · T-S3C `security_phase3c_pooling_tests.rs` (11) · T-S4A `security_phase4a_invalidation_tests.rs` (14; codecs, epochs, generations — **SEC-4A suite**) · T-S4B `security_phase4b_router_tests.rs` (19; **VERIFIED partial**: HTTP DELETE gateway tests assert statuses/headers only) · T-S4C `security_phase4c_antientropy_tests.rs` (18; calls `reconcile_peer_anti_entropy` directly) · T-KG `known_gaps.rs` (AG, in progress; EXISTS_UNVERIFIED).
**Bench (AG, in progress, EXISTS_UNVERIFIED):** BN-1 `benches/b1_hit_path.rs`, BN-2 `b2_concurrency.rs`, BN-3 `b3_coalesce.rs`, BN-4 `b4_eviction.rs`, BN-6 `b6_telemetry.rs`, BN-7 `b7_memory.rs`, BN-8 `b8e_gateway.rs`, BN-SUP `benches/support/{mod,alloc_counter,refpolicies}.rs`, results `bench-results/**` (generated, untracked).
**UI:** U-APP `ui/src/App.jsx` (T-U at inspection; single 524-line component, 107 inline style props, 0 aria/role, hard-coded gateway URL — VERIFIED partial) · U-MAIN `ui/src/main.jsx` (VERIFIED imports) · U-HTML `ui/index.html` · U-VITE `ui/vite.config.js` (VERIFIED; port 3000) · U-PKG `ui/package.json` (VERIFIED: react, react-dom, lucide-react [0 uses], vite; scripts dev/build only) · U-LOCK `ui/package-lock.json` (npm).
**Authoritative docs:** `docs/research/{phase_execution_packages_index,phase_1…8_execution_package,implementation_master_plan,mvp_and_milestones,phantom_intelligence_phase,pie_audit,claims_ledger,hypothesis_and_experiments,literature_review,benchmark_suite_spec,ui_metric_provenance,work_packages,code_findings,architecture_investigation}.md`, `docs/research/adr/ADR-001-…`, `CLAUDE.md`, `docs/handoff/00–08`, `docs/security/phase-4-design.md`, `docs/implementation/WP2.md`. **MISSING referenced items:** none of the documents named in the task is missing; `docs/handoff/CORRECTIONS.md` is **PROPOSED** (absent).

## 4. Common validation commands (per phase; details in each package)
Windows: `git rev-parse HEAD; git branch --show-current; git status --short` · `cargo test --all-targets -j 1 --locked` (after Phase 1 S3 the lockfile contains dev-dependency additions; follow the package for `--locked` use) · `cargo clippy --all-targets -j 1` (baseline from Phase 1 S1) · `cargo bench -j 1 --bench <id>` (Phase 1 harness; VALID rules in Phase 1 §6.3) · UI: `(cd ui && npm ls --depth=0 && node --test test && npm run build -- --outDir <tmp>)` — never `npm ci` in `ui/` · WSL2: same cargo commands from the synced copy with `CARGO_TARGET_DIR` on ext4. Evidence goes to `bench-results/`, `lab-results/` and the phase completion report, **not** into this map.

## 5. Per-phase work-package maps
Conventions: "modify" = may be edited if authorized and gated; "read" = inspect only. PROPOSED paths do not exist. Each WP's governing text is the cited package section; ACs are the package's `AC<n>-*`.

### 5.1 M1 — Truth & Baseline (`phase_1_execution_package.md` v3.1) — authorized, **IN PROGRESS** (AG)
| WP / step | Mapped paths | Mapping | Execution | Gate / dependency |
|---|---|---|---|---|
| S0 snapshot (no WP id) | snapshot dir outside repo; creates `docs/research/phase_1_progress.md`, `phase_1_snapshot_manifest.md` (both EXISTS_UNVERIFIED, AG) | MAPPED | COMPLETE (per AG log, unverified) | owner approval A-1 |
| S1 baseline verification | read: whole tree; `bench-results/logs/*` (generated) | MAPPED | COMPLETE (per AG log) | S0 |
| S2 WP-00 gap checks | modify: B-CARGO (`[[test]] known_gaps`), T-KG (EXISTS_UNVERIFIED), `docs/research/code_findings.md`; read: S-MAIN, S-NODE, S-ROUT, S-AE, S-SRV, T-S4B (pattern) | MAPPED | COMPLETE (per AG log) | G1 |
| S3 tooling | modify: B-CARGO (dev-deps, `[[bench]]`), B-LOCK; `bench-results/tooling_report.md`; WSL user space | MAPPED | COMPLETE (per AG log) | S2 |
| S4 WP-04 baselines | modify: BN-1…BN-8, BN-SUP; `bench-results/**`; read: S-CACHE, S-TEL, S-SRV (public API only) | PARTIAL (bench files unreviewed) | NOT RUN (benches written; baselines pending per log) | G3 (Core VALID); `src/**` unchanged |
| S5 WP-05 reference policies | `benches/support/refpolicies.rs` (EXISTS_UNVERIFIED), `docs/research/refpolicy_golden_traces.md` (PROPOSED) | PARTIAL | PENDING | S3 |
| S6 WP-01 corrections | `docs/handoff/CORRECTIONS.md` (PROPOSED), `docs/handoff/00_START_HERE.md` (one line) | MAPPED | PENDING | S2 outcomes (KG-1) |
| S7 WP-02/02b telemetry | modify: S-TEL, S-DIAG, S-SRV, S-MAIN, S-LIB (+ S-MAIN module lines), `src/intel/origin_metrics.rs` (PROPOSED); new `tests/integration/telemetry_truth_tests.rs` (PROPOSED); `docs/research/telemetry_schema_v2.md` (PROPOSED); read: S-PIE (untouched) | MAPPED | PENDING | **G3** |
| S8 WP-03 UI | modify: U-APP, U-MAIN, U-VITE, U-PKG (script only); new `ui/src/components/**`, `ui/src/lib/**`, `ui/src/SimDemo.jsx`, `ui/test/**`, `ui/scripts/check-forbidden.mjs` (all PROPOSED) | MAPPED | PENDING | S7c, R-A decision |
| S9 WP-06 byte accounting | modify: S-CACHE (hooks only), S-DIAG, S-MAIN, S-SRV (`bytes_*` fields); new `tests/integration/byte_accounting_tests.rs` (PROPOSED); read: S-COAL (**must stay unchanged**) | MAPPED | PENDING | S7 done, R-A outcome |
| S10 re-baselines R-B/R-C | BN-*, `bench-results/**` | MAPPED | PENDING | S9 |
| S11 verification/report | `docs/research/phase_1_completion_report.md`, `phase_1_changes.patch` (PROPOSED) | MAPPED | PENDING | all |
**Prohibited in M1 (package §9):** `src/control/pie.rs` behavior, `src/dist/*`, `src/security/*`, `src/core/coalesce.rs`, eviction/TTL/epoch logic, runtime dependencies, `.gitignore`, `transcript_copy.jsonl`, `docs/handoff/transcripts/**`. **Existing tests to protect:** all 242 (esp. T-S4A SEC-4A, T-COAL, T-M1); T-P4/T-WP2 untouched.

### 5.2 M2 — Core Correctness, Performance & Freshness (`phase_2_execution_package.md`) — NOT AUTHORIZED (gate G-M1)
| WP | Source (modify) | Existing tests to inspect/extend | New (PROPOSED) | Bench / config / docs | Shared / gated notes |
|---|---|---|---|---|---|
| M2-00 entry | — (docs) | all baselines | — | `phase_2_progress.md` (PROPOSED), frozen-rules file (PROPOSED) | reads Phase 1 report |
| M2-01 serving-path contract | S-SRV (origin fetch/handler; timeouts), S-COAL, S-CACHE | T-M1, T-COAL, T-S2 (origin side) | `tests/integration/serving_contract_tests.rs`; mock-origin helper (location TBD, maybe `tests/support/`) | `docs/research/adr/ADR-002-…` (PROPOSED) | **S-SRV/S-CACHE shared with M1/M3/M5**; fetch fns are private → tests go through HTTP |
| M2-02 freshness engine | `src/core/freshness.rs` (PROPOSED) + S-LIB/S-MAIN module lines; S-CACHE (`KeyState`/`CachedResource` fields); S-SRV (`server.rs` hard-coded TTL site, header parsing) | T-M1, T-WP1 (struct literals ⇒ O2-10), T-S4A | `tests/integration/freshness_matrix_tests.rs` | config keys (PROPOSED); `rfc9111_supported_subset.md` (PROPOSED) | field additions may break literal constructions in tests |
| M2-03 stale serving & cap | S-CACHE, S-SRV, S-TEL/S-DIAG (counters) | T-M1 | `tests/integration/stale_serving_tests.rs` | schema-v2 catalog additions; BN new `b_f2` (PROPOSED) | cap lives in executor layer used by M5 |
| M2-04 invalidation & sweeper | S-CACHE, (sweeper task: S-MAIN spawn) | **T-S4A (must stay green)**, T-M1 | `tests/integration/invalidation_contract_tests.rs` | — | O2-8 for the background task |
| M2-05 latency µs | S-TEL, S-SRV (record sites), S-DIAG | **T-WP2 (literal `RawObservation`)**, T-P4, T-M1 | unit tests in new file | schema catalog | O2-10 if existing tests must change |
| M2-06 eviction | S-CACHE (+ `src/core/eviction/**` PROPOSED), S-PIE (names only, comments) | T-P4 (policy tests), T-WP1, golden traces from Phase 1 `benches/support/refpolicies.rs` | `tests/integration/eviction_policy_tests.rs` | BN-4, new `b_ev` (PROPOSED) | **contract consumed by M5 executor** |
| M2-07 epoch contention (R) | S-CACHE | T-S4A | stress test file (PROPOSED) | BN-1, BN-2 | rule-gated |
| M2-08 telemetry hot path (R) | S-TEL | T-WP2 | accuracy/concurrency tests | BN-6, BN-8 | rule-gated |
| M2-09 concurrency verification | test code (+ optional facade — CONDITIONAL O2-5) | T-S4A, T-COAL | `tests/integration/concurrency_model_tests.rs` | dev-deps (CONDITIONAL) | mutation checks |
| M2-10 soak | none (runner) | — | soak harness (PROPOSED) | `bench-results/soak/**` | needs Phase 1 byte accounting |
| M2-11 PIE quarantine | S-PIE (doc comments only) | T-P4, T-WP2 (unchanged) | — | CLAUDE.md, CORRECTIONS | tests edits ⇒ O2-6 |
| M2-12 parser robustness | S-SRV private parser (CONDITIONAL visibility/refactor ⇒ owner) | T-S2 (23 upstream/parser tests) | `tests/integration/ingress_upstream_robustness_tests.rs` | proptest (Phase 1 dev-dep) | **private fns: cannot test directly without a visibility decision** |
| M2-13 schema v1 retirement (C) | S-SRV SSE handler, U-APP consumers | T-WP4, T-P3, T-P4, T-S1 (v1 assertions) | migration list | — | O2-7 |
| M2-14 docs | `docs/research/**`, CLAUDE.md, CORRECTIONS | — | — | — | — |
Status for all: mapping MAPPED (new files PROPOSED); execution **NOT AUTHORIZED**. **Gate:** G-M1, then G-M2 for exit. **Prohibited:** `src/dist/*`, `src/security/*` (M3/M7), `pie.rs` behavior, runtime deps.

### 5.3 M3 — Distributed Correctness & Architecture Gate (`phase_3_execution_package.md`) — NOT AUTHORIZED (gate G-M1; overlap with M2 only on disjoint files)
| WP | Source | Existing tests | New (PROPOSED) | Bench / config / docs | Shared / gated |
|---|---|---|---|---|---|
| M3-00 entry/trace | read: S-MAIN, S-SRV, S-NODE, S-ROUT, S-AE, S-FRAME, S-POOL | T-KG, T-S4A/B/C, T-P2 | `docs/research/architecture_trace_m3.md` | — | KG-1 outcome gates the phase |
| M3-01 harness | **possibly S-MAIN** (testable builder — CONDITIONAL O3-1); otherwise none | T-S3 (cert/config patterns), T-S4B (gateway patterns) | `tests/support/cluster_harness.rs`, `tests/support/fault_proxy.rs`, `tests/integration/cluster_harness_controls_tests.rs` | B-CERT, `CARGO_BIN_EXE_phantom-cache-demo`, env: `PHANTOM_NODE_ID`, `PHANTOM_CLUSTER_*`, members file (**S-MAIN panics if mTLS lacks members file**) | S-MAIN shared M1/M5/M7 |
| M3-02 characterization | none | T-KG | `tests/integration/cluster_characterization_tests.rs` | `bench-results/cluster/**` | — |
| M3-03 G-ARCH | docs | — | `adr/ADR-003-…` (PROPOSED) | — | **owner decision O3-2** |
| M3-04C gate | S-MAIN (startup refusal), S-SRV (capabilities) | T-S3 (config) | gating tests | env `PHANTOM_EXPERIMENTAL_CLUSTER` (PROPOSED) | O3-8 |
| M3-04A reliable fan-out (CONDITIONAL) | S-NODE, S-ROUT, S-AE, S-FRAME (**wire change ⇒ O3-3**), S-POOL, S-SRV (DELETE order), S-MAIN (wiring), S-CACHE (invalidation target) | **T-S3B, T-S3C, T-S4A, T-S4B, T-S4C, T-P2 must stay green or be explicitly migrated** | multi-process E2E tests (as M3-01) | metrics → schema-v2 catalog | S-CACHE/S-SRV shared with M2 — sequence after M2 edits |
| M3-04D sync fan-out (CONDITIONAL) | S-ROUT, S-SRV, S-NODE | same | E2E tests | — | — |
| M3-04B coordinated reads (CONDITIONAL, sub-spec first) | S-SRV GET path, S-ROUT, S-FRAME | same | sub-spec doc | — | O3-2 |
| M3-05 protocol model (optional) | none | — | model test file / TLA+ (CONDITIONAL O3-4) | — | — |
| M3-06 multi-host (CONDITIONAL) | none | — | — | hardware (O3-5) | — |
| M3-07 docs | README, CORRECTIONS, CLAUDE.md, ledger, `docs/security/phase-4-design.md` (update note, not rewrite) | — | — | — | — |
**Prohibited unless owner-reserved decision recorded:** `src/security/*` (O3-3), eviction/freshness logic (M2), deletion of distributed code (O3-9).

### 5.4 M4 — Phantom Lab & Static Baselines (`phase_4_execution_package.md`) — NOT AUTHORIZED (lab core: G-M1; baselines: G-M2)
| WP | Source / lab paths | Existing assets to inspect | New (PROPOSED) | Config / manifests | Shared / gated |
|---|---|---|---|---|---|
| M4-00 entry | — | Phase 1 manifests, `bench-results/**` | `phase_4_progress.md` | — | — |
| M4-01 lab core | **new `lab/**`** (packaging O4-1: workspace member vs `[[bin]]`+feature; **B-CARGO edit**) | S-SIM (clock, rng, events, scheduler, backend, network, traffic) — reuse candidates | `lab/src/{runner,clock,events,rng,manifest,schema}.rs` | manifest schema v2 (JSON), seed-set files (T/D/S hashes) | B-CARGO shared; no `[features]`/`[workspace]` today |
| M4-02 workloads | `lab/src/workloads/**`; S-TRAF (**read**; its O(n) Zipf sampler not reused as-is) | T-P3 | generator conformance tests | scenario files `lab/scenarios/**` | — |
| M4-03 origin models | `lab/src/origin_sim.rs` | S-SIM backend (M/M/1) | model conformance tests | — | — |
| M4-04 real origin + generator | `lab/src/{origin_proc,loadgen}.rs` | BN-8 (loopback HTTP pattern) | timing-accuracy tests | — | — |
| M4-05 lab gateway + conformance | `lab/src/gateway/**` linking library `phantom_cache_demo` (read: S-CACHE, S-SRV handler semantics) | T-M1 (conformance basis) | conformance suite | — | product handler semantics must not change |
| M4-06 baselines BL-0…BL-12b | `lab/src/policies/**` | — | fidelity tests + checklists | baseline parameter tables | needs M2 freshness (`freshness.rs`) |
| M4-07 eviction baselines | `lab/src/evict/**` or M2 trait | Phase 1 `refpolicies.rs`, M2 trait | golden tests | — | M2 trait |
| M4-08 tuning | `lab/src/tuning/**` | — | leak-check test | tuning logs under `lab-results/` | seed sets T/D/S |
| M4-09 matrix execution | none (runs) | — | — | `lab-results/**` (untracked) | time budget O4-6 |
| M4-10 T1↔T2 study | none | — | — | report | — |
| M4-11 reproduction | none | — | — | reproduction record | O4-8 |
| M4-12 viewer/reports | `lab/src/report/**`; optional minimal viewer reusing U-* components (post-Phase-1) | U-APP components | labeling tests | — | M8 owns polish |
| M4-13 cluster scenarios (CONDITIONAL G-M3) | reuse `tests/support/cluster_harness.rs` (M3, PROPOSED) | — | — | — | — |
**Prohibited:** product request-path semantics, `src/security/*`, `pie.rs`.

### 5.5 M5 — Phantom Intelligence & PIE (`phase_5_execution_package.md`) — NOT AUTHORIZED (M5a: G-M1+G-M2+M4 lab core; M5b: G-M4+G-RESEARCH)
| WP | Source (modify) | Existing assets | New (PROPOSED) | Docs / config | Shared / gated |
|---|---|---|---|---|---|
| M5-00 entry/audit/prereg | read: S-PIE, S-SKT, S-WL | `pie_audit.md`, T-P4, T-WP2 | preregistration file | — | — |
| M5-01 architecture spec | docs; type skeletons in new module | — | `docs/research/intelligence_architecture.md`; `src/intelligence/mod.rs` + S-LIB/S-MAIN module lines | API proposals | **O5-1 approval before code** |
| M5-02 signals/detectors | `src/intelligence/signals/**`; consume S-TEL/S-DIAG/origin metrics; reuse S-SKT (CONDITIONAL, currently unused) | T-WP2 (sketch test) | detector accuracy tests (lab ground truth) | schema-v2 catalog | needs real `key_hash` (M1 S7) |
| M5-03 shadow evaluator | `src/intelligence/shadow.rs` | Phase 1 refpolicies | determinism/accuracy tests | — | — |
| M5-04 executor & fallback | **S-CACHE** (policy handle, canary buckets, hard caps), `src/intelligence/executor.rs` | T-S4A, M2 model tests, T-P4 (policy switch) | executor invariant tests | — | **independent review; S-CACHE shared M1/M2/M3** |
| M5-05 decision engine v2 | `src/intelligence/engine.rs`; S-MAIN (construct engine; replaces `pie: None`) | T-P4, T-WP2 (legacy semantics, unchanged) | decision-table, A/A, planted-effect tests | config `PHANTOM_INTELLIGENCE` (PROPOSED) | O5-1/O5-2 |
| M5-06 audit & API | `src/intelligence/audit.rs`; S-SRV (new endpoints ⇒ O5-1); U-* minimal panel | T-S1 (auth patterns) | audit/reconstruction tests | decision-record schema | S-SRV shared |
| M5-07 safety/overhead/replay | tests + benches | BN-* | fault-injection, overhead, replay tests | `bench-results/**` | — |
| M5-08 PIE in lab | `lab/**` integration | M4 assets | equivalence tests | — | — |
| M5-09…M5-13 candidates (M5b) | lab gateway first (`lab/src/candidates/**`); product only after disposition | M4 baselines | candidate tests | dispositions → `claims_ledger.md` | G-RESEARCH |
| M5-14 integration (owner O5-4) | `src/intelligence/candidates/**` (retained only) | — | equivalence tests | flags | — |
| M5-15 docs | CLAUDE.md, handoff, docs | — | — | — | — |
**Gated:** `src/control/pie.rs` — banner comments only (M2) then retirement per M5-14; hard-cap definitions (M2) change only by owner decision.

### 5.6 M6 — Research Evaluation & MVP Gate (`phase_6_execution_package.md`) — NOT AUTHORIZED (gate G-M5; G-M3 for distributed claims)
| WP | Paths | Notes |
|---|---|---|
| M6-00 entry | `phase_6_progress.md` (PROPOSED); read all prior reports | seed-leak check against `lab-results/` logs |
| M6-01 prior-art closure | `docs/research/literature_review.md` (modify), `docs/research/novelty_snapshot_m6.md` (PROPOSED), coverage-log files (PROPOSED) | research track; no code |
| M6-02 pre-registration | `docs/research/preregistration_m6.md` (PROPOSED), `claims_ledger.md`, `hypothesis_and_experiments.md` (read/amend by dated amendment) | freeze before runs (O6-1) |
| M6-03 design freeze/analysis validation | analysis scripts (location **CONDITIONAL O6-4**, e.g., `analysis/**` PROPOSED) | synthetic known-effect datasets |
| M6-04 build freeze | binary/config hashes in manifests | no source change except bug protocol |
| M6-05/06 evaluation runs | `lab-results/**`, lab binaries (M4), product binary | VALID classes |
| M6-07 ablations | lab scenario files | — |
| M6-08 overhead/failure modes | BN-*, lab scenarios, product binary | modes off…active |
| M6-09 distributed (CONDITIONAL) | M3 harness | — |
| M6-10 statistics | `docs/research/results/**` (PROPOSED), analysis scripts | deterministic |
| M6-11 artifact validation | artifact scaffolding (PROPOSED) | independent executor O6-8 |
| M6-12/13 ledger & MVP gates | `claims_ledger.md`, `docs/research/phase_6_completion_report.md` (PROPOSED) | owner decides MVP outcome O6-5 |
**Prohibited:** product source changes except the bug protocol (ledgered, owner-visible).

### 5.7 M7 — Productization & Research Release (`phase_7_execution_package.md`) — NOT AUTHORIZED (gate G-MVP; U1 if D-8)
| WP | Source (modify) | Existing assets | New (PROPOSED) | Shared / gated |
|---|---|---|---|---|
| M7-01 config validation | S-MAIN (env parsing today), S-SEC (`validate_*` startup checks), `src/config.rs` (PROPOSED + module lines in S-LIB/S-MAIN) | T-S1, T-S2, T-S3 (config/startup tests) | config validation matrix tests | O7-3 surface choice |
| M7-02 containment/operability | S-MAIN, S-SRV (shutdown/limits), S-NODE (if cluster shipped) | T-S3C | shutdown/restart/exhaustion tests | — |
| M7-03 observability | S-SRV (+ `/metrics` ⇒ O7-4), logging sites | schema-v2 catalog | exporter contract & secret-canary tests | O7-4 |
| M7-04 security review | S-SEC, S-JWT, S-UP, S-SCL, S-TLS (fixes only), `docs/security/*` (update) | T-S1, T-S2, T-S3, T-S3B | security checklist records | **`src/security/*` edits owner-reserved** |
| M7-05 perf gate | BN-* | Phase 1/6 manifests | regression-gate script | — |
| M7-06 packaging | `packaging/**` (PROPOSED), B-CARGO (profile notes only), B-BAT (read) | B-CERT | install/upgrade/rollback scripts; docs | B-CI changes CONDITIONAL (below) |
| M7-07 docs-as-tests | `docs/user/**` (PROPOSED), README | — | docs-test script | — |
| M7-08 dashboard acceptance | U-* (fixes) | M8 U1 | operator-task scripts | with M8 |
| M7-09/10 claims & artifact | `docs/research/**`, claims lint script (PROPOSED) | claims ledger | — | — |
| M7-11 RC/release | release notes (PROPOSED) | — | — | owner performs tag/push |
| **CI** | `.github/workflows/ci.yml` (CONDITIONAL: adding jobs/gates is an owner-reserved change) | VERIFIED partial | — | see §9 discrepancy |

### 5.8 M8 — UI/UX Excellence (`phase_8_execution_package.md`) — NOT AUTHORIZED (stage-wise; see package)
| WP / stage | Paths | Notes |
|---|---|---|
| M8-00/01 entry, audit (A) | read: U-APP & Phase 1 results (`ui/src/components/**`, `ui/src/lib/**`, `ui/test/**` PROPOSED by Phase 1); `docs/design/ui_audit.md` (PROPOSED) | audit post-Phase 1 UI, not the pre-Phase-1 snapshot recorded in the package |
| M8-02 skill decision (B) | `.claude/skills/<reviewed>/` or `docs/design/reference/` (PROPOSED, **absent now**; `.claude/` does not exist in the repo) | copy reviewed Emil files only on O8-1; `SOURCE.md` with SHA |
| M8-03…M8-07 direction/IA/tokens/wireframes/gate | `docs/design/{principles,information_architecture,tokens}.md`, `docs/design/prototypes/**` (PROPOSED) | no code before O8-2 |
| M8-08 tokens & base components | `ui/src/styles/**`, `ui/src/components/**` (PROPOSED/Phase 1) , U-APP, U-MAIN, U-HTML | no new deps without O8-3 |
| M8-09 chart/table components | `ui/src/components/charts/**` (PROPOSED) | SVG/CSS; chart library = dependency decision |
| M8-10 screens S1–S11 | `ui/src/screens/**` (PROPOSED) wired only to existing endpoints in S-SRV (`/api/events`, `/api/v1/diagnostics/*`, schema-v2 snapshot) | S6–S8, S10–S11 need backend from M3/M5/M7 (CONDITIONAL) |
| M8-11…M8-16 motion, a11y, perf, usability, gates, docs | `ui/**`, `docs/design/**`, `ui/test/**` | dev tooling for a11y = O8-3 |
**Prohibited:** backend endpoint/schema changes, simulated data in live views, Apple assets/SF fonts/Symbols.

## 6. Shared-file and conflict map
| File | Phases / WPs touching it | Why shared | Contract established first by | Conflict risk & sequencing |
|---|---|---|---|---|
| B-CARGO `Cargo.toml` | M1 S2,S3; M2 (deps CONDITIONAL); M4-01 (lab packaging, `[features]`/workspace); M5 (CONDITIONAL deps); M7 | manifest, `[[test]]`/`[[bench]]`, deps | M1 S3 (dev-deps, benches) | highest-contention file; owner-reserved edits (O2-5, O4-1, O5-5); edit one WP at a time, record exact diff |
| B-LOCK | every dependency change | resolved versions | M1 S3 (additions-only check) | each change must prove no version change to existing packages |
| S-MAIN | M1 S7,S9; M2 (config keys, sweeper); M3 (cluster wiring, gating); M5 (engine construction); M7 (config/validation) | single wiring point; **duplicates module tree with S-LIB** | M1 (env flags) | sequential only; module registration in both S-MAIN and S-LIB each time |
| S-LIB | any new module (`origin_metrics`, `freshness`, `eviction`, `intelligence`, `config`) | module declarations | M1 S7 pattern | trivial but easy to forget the `main.rs` copy |
| S-SRV | M1 S7,S9; M2 (freshness, stale, contract, schema); M3 (DELETE order); M5 (endpoints); M7 (metrics/readiness/shutdown) | all HTTP behavior and SSE | M1 S7 (schema v2) then M2 (freshness) | sequence M1→M2→M3→M5→M7; **no parallel edits**; private parser fns stay private unless owner decides |
| S-CACHE | M1 S9; M2-01…07; M3 (invalidation target); M5-04 (executor) | cache semantics, fencing, eviction | M1 S9 (byte accounting) → M2 (eviction trait, freshness) → M5 (executor) | M2 and M3 may only overlap if M3 avoids S-CACHE until M2 finishes; independent review for fencing/epoch/executor changes |
| S-COAL | M2-01 (contract fixes) | guard/finalize protocol | M2 | **prohibited in M1**; M5 depends on its behavior |
| S-TEL / S-DIAG | M1 S7; M2-05/08; M5 signals | observations, counters | M1 S7 | `RawObservation` literal in T-WP2 constrains changes (O2-10) |
| S-PIE | M2-11 (comments), M5-14 (retire) | legacy stub | — | behavior frozen until M5 |
| S-NODE / S-ROUT / S-AE / S-FRAME | M3 only | cluster protocol | M3 ADR-003 | wire changes O3-3; T-S3B/3C/4A/4B/4C regression suites |
| S-SEC* | M3 (O3-3), M7-04 | security-critical | M7 review | owner-reserved |
| T-KG `known_gaps.rs` | M1 S2 → M3 (promote/retire) | executable gap evidence | M1 S2 | M3 edits only with ADR-003 rationale |
| BN-SUP / `refpolicies.rs` | M1 S5 → M2-06, M4-07, M5-03 | reference policies | M1 S5 | treat as a library; changes need golden-trace re-run |
| Telemetry schema v2 catalog (`telemetry_schema_v2.md`, server payload) | M1 S7 → M2, M3, M5, M7, M8 | provenance contract | M1 S7 | additions need catalog entry + UI handling; removal forbidden without version bump |
| Run-manifest schema | M1 S4 → M4 → M6 | reproducibility | M1 S4, extended M4-01 | versioned |
| U-APP / `ui/src/**` | M1 S8; M2-13; M5-06; M8 (everything) | the whole UI | M1 S8 (components, schema 2) | M8 may not start Stage C coding while M1 S8 edits U-APP |
| U-PKG | M1 S8 (script), M8 (deps) | npm manifest | M1 | dependencies = owner decision |
| B-CI `.github/workflows/ci.yml` | M7 (CONDITIONAL) | CI gates | — | no phase edits it unless owner approves; see §9 |
| Planning docs (`implementation_master_plan.md`, `mvp_and_milestones.md`, `work_packages.md`, index, this map, `claims_ledger.md`, `CLAUDE.md`, handoff) | all | shared records | research lead | AG updates only what its package authorizes; `mvp_and_milestones.md` needs the pending M8 (Rev 3) edit — **not applied while AG executes** |
**Concurrency rule:** two phases/WPs may run concurrently only if the table above shows no shared file in their authorized sets **and** the owner/research lead has confirmed the separation in writing.

## 7. Gated / prohibited areas (quick reference)
| Area | Gate or prohibition |
|---|---|
| `src/**` before Phase 1 G3 | unchanged from S0 until S7 (hash-checked) |
| `src/core/coalesce.rs` | prohibited in M1; M2-01 only |
| `src/control/pie.rs` | M1 untouched; M2 comments only; M5-14 retirement |
| `src/dist/*`, `src/security/*` | M3 (and O3-3 for security) / M7-04; never in M1/M2/M4 |
| Eviction/TTL/epoch/fencing semantics | M1 prohibited (accounting hooks only); M2 only as specified; M5-04 executor only with independent review |
| Product request-path semantics | untouched by M4 (lab gateway is separate) |
| Intelligence modes above `off` as default | after G-MVP (owner) |
| Runtime dependencies | owner-reserved in every phase |
| `Cargo.toml` structure (features/workspace) | M4-01 only, O4-1 |
| UI: backend endpoints/schema | M8 may not add; routes to owning phase |
| Git operations | agents: read-only (CER-3); owner commits/tags/pushes |

## 8. Maintenance protocol
**Update triggers:** inspection disproves a mapping; a PROPOSED file is created or a path is replaced; a dependency or shared-file conflict is found; a WP completes (record actual changed files from the ledger); a gate changes what may be touched. **How:** edit the affected row, set classification (PROPOSED→VERIFIED when created and reviewed), set mapping status (MAPPED/PARTIAL/UNMAPPED) and execution status independently, append to the change log, and cite the progress-log entry. **Do not** paste test results, benchmark numbers or implementation detail here — link to the phase completion report. **Execution status transitions:** NOT AUTHORIZED → READY (owner authorization + entry gate PASS) → IN PROGRESS → COMPLETE | BLOCKED | NOT RUN (per package semantics).
**Change log:** 1.0 (2026-10-29) — initial map; Phase 1 reflects AG's self-reported S0–S3 completion (unverified).

## 9. Discrepancies between the execution packages and current source (report; not corrected here)
1. **CI exists.** `.github/workflows/ci.yml` (push/PR to `main`, ubuntu): `cargo fmt --all -- --check`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`. `phase_7_execution_package.md` §2 states "no CI configuration known" — **incorrect**. Consequences: (a) CI runs `cargo fmt --check` while agents are forbidden to run `cargo fmt`; formatting status of the dirty tree is unknown; (b) CI uses no `-j 1` and `--all-features` although `Cargo.toml` has no `[features]`; (c) the untracked tree cannot build from HEAD in CI; (d) a hosted Linux runner is a possible but uncontrolled timing platform.
2. **Private parser functions.** `fetch_origin_raw`, `parse_http_response`, `decode_chunked_body` are private in `server.rs`; `phase_2` M2-12 plans direct parser tests — they must go through the HTTP surface or need an owner visibility decision.
3. **Duplicated module trees** (`lib.rs` vs `main.rs`): packages mention it for `origin_metrics` only; every new module in M2, M4–M7 needs the same two-place registration.
4. **No `[features]`/`[workspace]`:** `phase_4` proposes `required-features = ["lab"]` or a workspace member — both are manifest structure changes (O4-1).
5. **Handoff rules predate the roadmap:** `docs/handoff/08` forbids "re-architect or refactor" and "begin Phase 5", and uses the old phase numbering; Phase 1 S6 (CORRECTIONS) must reconcile; until then AG follows the execution packages and `CLAUDE.md`.
6. **Approval-number drift** between `phase_1_progress.md` §3 and `phase_1_execution_package.md` v3.1 §3 (e.g., A-4, A-5) — unresolved; later packages cite semantics, not numbers.
7. **Sim modules imported by `main.rs`** (`use sim::…`, lines ~46–50) but no runtime use was verified; the dashboard simulation loop in `server.rs` uses `rand`, not `sim`. M4's plan to reuse `src/sim/*` must verify actual APIs first.
8. **`WorkloadProfile` and `CountMinSketch` are effectively unused** (sketch only in `wp2_tests`): M5 reuse is a design choice, not an integration.
9. **Phase 3 harness assumption:** `main.rs` panics if cluster TLS env is set without a members file; the insecure-cluster branch builds a router over addressless nodes — the harness must use the mTLS path (or O3-1).
10. **Whole `tests/integration/` directory is untracked** (so are `src/security/`, four `src/dist` files, `src/intel/diagnostics.rs`): the "committed baseline" for AC/diff purposes in every phase is the **working tree + snapshot**, not HEAD.

## 10. Unresolved mappings
Roles of `frame.rs`, `pool.rs`, `session.rs`, `security/{cluster,jwt,upstream}.rs` and most integration tests are taken from handoff documents (agent-reported) and must be verified by the first WP that touches them. AG's new `benches/*`, `known_gaps.rs` and `bench-results/tools/benchmarks_runner.py` (a Python script — Python was reported absent on Windows; verify its intended platform) are unreviewed by the author. M3 harness location (`tests/support/` vs lab) is open; M4 lab packaging (O4-1) is open; M6 analysis-script location (O6-4) is open; M5 module location (`src/intelligence/`) is fixed only by M5-01. Line numbers are intentionally omitted except where already verified in the research documents.

## 11. Links to authoritative packages
[M1](phase_1_execution_package.md) · [M2](phase_2_execution_package.md) · [M3](phase_3_execution_package.md) · [M4](phase_4_execution_package.md) · [M5](phase_5_execution_package.md) · [M6](phase_6_execution_package.md) · [M7](phase_7_execution_package.md) · [M8](phase_8_execution_package.md) · [index](phase_execution_packages_index.md) · [master plan](implementation_master_plan.md) · [milestones/MVP](mvp_and_milestones.md) · [ADR-001](adr/ADR-001-present-architecture-and-direction.md) · [PIE audit](pie_audit.md) · [Phantom Intelligence spec](phantom_intelligence_phase.md) · [claims ledger](claims_ledger.md) · [CLAUDE.md](../../CLAUDE.md).
