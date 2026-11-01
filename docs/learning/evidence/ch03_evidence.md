# Chapter 3 evidence file

Chapter: `docs/learning/src/ch03_technology_and_decisions.md`. Written 2026-10-30 (machine clock). Tree: branch `feat/publish-phantomcache-core`, HEAD `caeedfd` per session start, large uncommitted work. No builds, tests, or git mutations were run. "Read" means opened with Read/sed/grep in this session.

## Claims

| # | Claim | Source (file::symbol) | Status label | How verified | Limits |
|---|---|---|---|---|---|
| 1 | Locked versions: tokio 1.53.1, axum 0.7.9, axum-server 0.7.3, hyper 1.11.1, hyper-util 0.1.20, tower 0.5.3, tower-http 0.5.2 and 0.6.11, rustls 0.23.46, tokio-rustls 0.26.6, ring 0.17.14, jsonwebtoken 9.3.1, serde 1.0.229, serde_json 1.0.151, rand 0.8.8 and 0.10.3, rcgen 0.13.2, reqwest 0.12.28, criterion 0.5.1, hdrhistogram 7.6.0, proptest 1.12.0, aws-lc-rs 1.18.2, aws-lc-sys 0.46.0, openssl 0.10.81; 288 `name =` entries | `Cargo.lock` | IMPLEMENTED | awk/grep over Cargo.lock | count is of `^name` lines |
| 2 | Direct deps, features, dev-deps, 72 `[[test]]`, 7 `[[bench]]` all `harness=false`, 2 bins; no `[profile]`, no `rust-version` | `Cargo.toml` | IMPLEMENTED | Read in full; grep -c | |
| 3 | `tracing`/`tracing-subscriber` declared, zero uses | `Cargo.toml`; grep `-w tracing` over src/tests/benches empty | IMPLEMENTED | grep | cannot exclude macro-generated paths; axum compiles `tracing` itself |
| 4 | `criterion` declared, zero uses; benches use custom `main`, hdrhistogram in b2 and b8e | `Cargo.toml`; `benches/*.rs` | IMPLEMENTED | grep `criterion` over benches/tests/src found only unrelated words | |
| 5 | tower-http only `cors` used; `ServeDir`, WebSocket, `debug_handler` unused | `src/api/server.rs` imports; grep | IMPLEMENTED | grep | |
| 6 | No hyper/reqwest use in `src`; origin client is hand-written HTTP/1.1 over `TcpStream`/`tokio_rustls`, `Connection: close`, pinned address | `src/api/server.rs::fetch_origin_raw`; `src/security/upstream.rs` | IMPLEMENTED, TESTED (suites named, not rerun) | grep `hyper::`, read request-building lines 2534-2710 region | parser not reviewed |
| 7 | Router routes and layers (`DefaultBodyLimit` 512 KiB, `CorsLayer`), serve paths with and without TLS | `src/api/server.rs::create_app_with_lab`, `start_server_with_cors` | IMPLEMENTED | Read lines 190-320 | |
| 8 | Runtime: `new_multi_thread().enable_all()`, whole `block_on` in `catch_unwind` | `src/main.rs::real_main` | IMPLEMENTED | Read lines 168-195 | default worker count assumed from Tokio, not read |
| 9 | Tokio primitives table (oneshot, watch, mpsc, Semaphore 64 and 16, broadcast 64, timeout x17, interval, select, spawn_blocking) | `src/core/cache.rs`, `coalesce.rs`, `dist/node.rs`, `dist/router.rs`, `security/cluster.rs`, `api/lab_api.rs` | IMPLEMENTED | grep with line context | counts are grep line counts |
| 10 | Shards: `Vec<Mutex<CacheShard>>`, `shard_mask`, power-of-two assert, `DefaultHasher::new()`, `PartitionedCache::new(8)` in main, 4 in node | `src/core/cache.rs::PartitionedCache`, `get_shard_index`; `src/main.rs`; `src/dist/node.rs` | IMPLEMENTED | Read 270-345, 540-556; grep `PartitionedCache::new` | |
| 11 | Default max entries per shard 10,000; waiters 1,000; stale cap 86,400,000; sweeper batch 32, 100 ms | `PartitionedCache::new_with_diagnostics` | IMPLEMENTED | Read | |
| 12 | Epoch snapshot: read lock + atomic in `get_effective_epochs`; invalidate path takes `tenant_epochs` then snapshot write lock; clone evaluated under the write lock; overflow fallback bumps `global_epoch` | `src/core/cache.rs` lines ~560-640 | IMPLEMENTED | Read | the "no shard lock in invalidation" statement covers the two functions read, not all code |
| 13 | Lock order shard then epoch RwLock read (sweeper) | `sweep_lazy_reclamation` | IMPLEMENTED | Read | full lock-order audit not done |
| 14 | Waiter push is in its own block before any await; fetch in `tokio::spawn`; waiter awaits `rx` | `src/api/server.rs` Initiate branch | IMPLEMENTED | Read 1620-1700 | other handlers not audited for await-crossing |
| 15 | `OriginGate` atomics and RAII `OriginPermit::drop`; doc comment states no lock across await | `src/intelligence/executor.rs` | IMPLEMENTED | Read 70-100 | |
| 16 | Poison recovery `into_inner` + `lock_poison_recovered` counter on shard locks; other sites `.unwrap()` | `src/core/cache.rs`; G4A-21, G4B-28 | PARTIAL | Read cache.rs; read register rows | register rows are AI review |
| 17 | Atomic ordering counts in cache.rs: 62 lines Relaxed, 7 lines SeqCst | `src/core/cache.rs` | IMPLEMENTED | `grep -c` | line counts, not call counts |
| 18 | Histogram: 128 exact + 20 x 128 = 2,688 buckets; bucket(1000 us) = 506; value back = 1002; max error 1/256 | `src/intel/telemetry.rs::AtomicLogLinearHistogram::{value_to_bucket,bucket_to_value}` | IMPLEMENTED, TESTED (`telemetry_redesign_tests::test_histogram_relative_error_bounded`, not rerun) | Read code and hand-computed the worked example | worked example by hand arithmetic, not executed |
| 19 | `unsafe`: 5 blocks in `src/api/lab_api.rs` (Windows job object, process handle), 1 `unsafe impl GlobalAlloc` in benches; none in core | grep `unsafe` | IMPLEMENTED | grep over src/tests/benches | test file hit is a word in a test name |
| 20 | Eviction structures: arena `Vec<Option<Node>>` + `free_list` + `HashMap` (SIEVE, LRU); `VecDeque` x3 + `HashSet` + `HashMap` (S3-FIFO); `ClockScan` Vec + `retain` | `src/core/eviction.rs` | IMPLEMENTED | grep struct fields; read ClockScan | algorithm complexity not re-derived (Ch 4) |
| 21 | `CountMinSketch` has no use outside its file | `src/intel/sketch.rs` | PROTOTYPE ONLY | `grep CountMinSketch src` | tests not searched |
| 22 | No bitmap types found | grep | n/a | grep for bitmap/BitVec | negative search |
| 23 | Cluster mTLS: TLS13 only, `WebPkiClientVerifier`, client cert; exact one DNS SAN; hand-written DER SAN extractor; SHA-256 fingerprint via `ring` | `src/security/cluster.rs`; `src/dist/node.rs` | IMPLEMENTED, TESTED (`test_p3_sec04_strict_tls13_rejection_of_tls12_client`) | Read 275-295, 620-650; node.rs 290-310 | ordering of chain verification vs SAN parse not tested by me |
| 24 | Ingress TLS built by `RustlsConfig::from_pem_file`; axum-server 0.7.3 uses `ServerConfig::builder()` with no version restriction and ALPN `h2`,`http/1.1`; server prints "TLS 1.3" | `src/security/tls.rs`; `src/api/server.rs:292`; `~/.cargo/registry/src/*/axum-server-0.7.3/src/tls_rustls/mod.rs::config_from_der` | IMPLEMENTED; ingress "TLS 1.3 only" UNVERIFIED | Read registry source | rustls default versions from crate docs, not re-read |
| 25 | JWT: HS256 only; `kid` rotation; leeway 10; claims fields; roles strings | `src/security/jwt.rs` | IMPLEMENTED | Read | |
| 26 | Spec lists HS256/RS256/EdDSA; code HS256 only | `docs/security/phase-1-specification.md` section on algorithms vs `jwt.rs` | discrepancy | Read both | |
| 27 | Recorded rationale: rustls/ring/rcgen to avoid OpenSSL C build failures; axum-server and jsonwebtoken dependency plan | `docs/security/phase-1-specification.md` sections 7, 8 | recorded (spec) | Read | approver not named in what I read |
| 28 | ADRs: 001 PROPOSED, 002 Accepted, 003 ACCEPTED+amendment; owner ratification not recorded; no language/framework ADR | `docs/research/adr/README.md`, ADR files | recorded | Read README, grep ADRs | ADR-001..003 not read in full |
| 29 | "Go/Rust architecture discussions" in a ChatGPT export not in the repo | `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md` row ChatGPT-Prior | reported by handoff | grep | cannot verify the missing file |
| 30 | Binary frame header: magic `PHAN`, version 1, 28-byte big-endian header; 50-byte anti-entropy request | `src/dist/frame.rs` | IMPLEMENTED | grep | other frame types not read |
| 31 | 19 source files derive serde | grep `Serialize|Deserialize` | IMPLEMENTED | grep -l | count is of files in the first listing (19 lines) |
| 32 | No Dockerfile/compose; packaging scripts, example configs, smoke test, demo .bat | repo-wide `find`; `packaging/*` | IMPLEMENTED (scripts) | find + read build-release.sh and smoke-test.sh heads | `.ps1` variants not read |
| 33 | CI workflow content (ubuntu; fmt, check, clippy, test without `-j 1`; frontend `npm ci`, `npm run build`, Node 20) | `.github/workflows/ci.yml` | IMPLEMENTED (file) ; execution UNVERIFIED | Read 61 lines; `git ls-files` shows it tracked | no run history |
| 34 | UI stack and versions: react 18.3.1, vite 5.4.21 installed; `lucide-react` declared and unused; axe-core 4.14.0 pinned; 23 test files; CSS about 1,000 lines | `ui/package.json`, `ui/node_modules/*/package.json`, `ui/src`, `ui/test` | IMPLEMENTED | Read, `wc`, grep | |
| 35 | UI recorded stack and rejected-library list | `docs/design/remediation/07_...record.md` line 9; `docs/research/design_toolkit/toolkit_proposal.md` | recorded | grep and sed | proposal is an AI document |
| 36 | Forbidden-pattern guard is a literal regex tripwire; Lab client uses `(fetchImpl \|\| globalThis.fetch)` | `ui/test/forbidden.test.mjs`, `ui/src/lib/labApi.js` | TESTED (tripwire) | Read | I did not run the UI tests |
| 37 | CDP helper uses Node built-ins, hard-coded Chrome path, `D:/Temp` profile | `ui/scripts/cdp-lib.mjs` | IMPLEMENTED | Read first 30 lines | |
| 38 | Reported baseline 851 pass / 0 fail / 8 ignored / 75 targets, debug, clippy exit 0 | `docs/research/phantomcache_milestone_accountability_report.md` line ~300 | reported by prior agent/lead | grep | not reproduced here |
| 39 | `proptest!` in 4 test files | grep | TESTED | grep | |
| 40 | Supply chain: no git/alt registry, 11 duplicate names, two crypto stacks, advisory scan not run | `docs/release/g4/cycle1_lab_intelligence_deps_review.md`; `docs/STATUS.md` M7 | reported (AI review) ; partly reproduced from lockfile | package count and crate presence reproduced; "0 git sources" not independently counted | |
| 41 | MSRV discrepancy (README 1.75 vs code needing 1.88+) | G4C F28; `Cargo.toml` has no rust-version | reported by AI review | read review text; confirmed Cargo.toml lacks field | I did not compile with an old toolchain |

## Figure checks

- `ch03_stack_layers.svg`: versions copied from Cargo.lock and node_modules (claims 1, 34); "dashed = declared, no use found" matches claims 3, 4, 34; "No Dockerfile" matches claim 32; "28-byte-header frames" matches claim 30; "HS256 only" claim 25.
- `ch03_concurrency_model.svg`: 8 shards (claim 10), `hash & 7` (mask = 7), per-shard fields (cache.rs `CacheShard`), epoch structures (claim 12), sweeper locks one shard at a time (claim 13), invalidate path takes no shard lock (claim 12, limited), gap findings G4A-12/17/21/03/B-28 from the disposition register.
- `ch03_lock_vs_await.svg`: pattern A matches claim 14; hazard B is Rust/Tokio semantics (reasoning); `MutexGuard: !Send` is a language property recalled, not looked up this session.

## Discrepancies found (document versus code, or document versus document)

1. Ingress "TLS 1.3": server.rs log line and Phase 1 spec say TLS 1.3; code does not restrict versions; only cluster has a TLS 1.2 rejection test.
2. JWT algorithms: spec allows HS256, RS256, EdDSA; code rejects all but HS256.
3. "Eliminating OpenSSL C dependencies" (spec section 8) versus `aws-lc-sys` (C) in the lockfile and OpenSSL crates via dev `reqwest`.
4. Histogram comments in `src/intel/telemetry.rs`: header "<= 0.5%" versus function comment and CLAUDE.md 0.39%; "~67.1 seconds" is 2^26 us, the table covers up to 2^27 us (about 134 s).
5. Declared but unused: `tracing`, `tracing-subscriber`, `criterion`, tower-http `fs`, axum `ws`, UI `lucide-react`.
6. README "Tokio 1.38" versus locked 1.53.1 (the Cargo.toml requirement is a minimum); README "cargo 1.75+" versus code needing about 1.88+ (G4C F28).
7. ADR-002 sweeper "lock-hold <= 1 ms, at most 32 entries inspected" versus G4A-03 (batch bounds candidates, not keys inspected).
8. Epoch snapshot design intent ("read path zero contention") versus the O(n) clone under the write lock (G4A-12) and the non-atomic epoch pair (G4A-17).
9. Forbidden-literal UI test is described elsewhere as guaranteeing no other network calls; the regex does not match the Lab client's indirect `globalThis.fetch`.
10. CI workflow runs `cargo test` without `-j 1` and the frontend job does not run `npm test`, unlike the project's local rules and the UI check script.

## UNVERIFIED items

- Whether HTTP/2 is actually served on the TLS ingress (ALPN advertises h2; hyper feature set not checked).
- `DefaultHasher::new()` fixed-key and release-instability semantics (recalled from std docs; no network).
- rustls default protocol versions (TLS 1.2 + 1.3) from crate docs, not re-read.
- That every function in the tree is free of a std guard across `.await`; only the sections named were read.
- Whether `.github/workflows/ci.yml` ever ran.
- Whether the DER SAN parser is reached only after chain verification (comments suggest so; not traced end to end).
- Phase 3 design and framing rationale for the binary protocol (only grep-level).
- npm advisory status; Rust advisory status.
- All test results (nothing was run).
- Existence of a human approver for the Phase 1 spec decisions.

## Cross-chapter notes

- Chapter 2: route list and `create_app_with_lab` details here must match its request-flow figures; the hand-written origin client and `Connection: close` belong in its origin-interaction section.
- Chapter 4: owns SIEVE/LRU/S3-FIFO complexity, G4A-01, G4A-02, G4A-03, G4A-20, and the epoch protocol; this chapter only cites them.
- Chapter 5: `DefaultHasher` is also used by `router.rs::calculate_hash` for rendezvous (weight = `hash as f64`, constant weight 1.0); toolchain-stability point is raised here.
- Chapter 6: ingress TLS version, JWT HS256 only, G4B-01, G4B-28, SBOM/advisory gap, CORS layer, the hand-written DER and HTTP parsers.
- Chapter 7: `rand::random` also seeds synthetic dashboard numbers in `server.rs` (labelling is a Ch 6/7 matter); lab hashing uses `ring`.
