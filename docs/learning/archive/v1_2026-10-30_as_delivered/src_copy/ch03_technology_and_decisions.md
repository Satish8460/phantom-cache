# Chapter 3: Technology Stack and Engineering Decisions

**Status of this chapter:** written 2026-10-30 from the current working tree (branch `feat/publish-phantomcache-core`, uncommitted work present; at the time of writing HEAD is `df68eaa`). Versions are the values in `Cargo.lock` and `ui/node_modules`. Nothing was built or run for this chapter; every statement is from reading files, `Cargo.lock`, and the vendored `axum-server 0.7.3` source in the local cargo registry. Nothing here is INDEPENDENTLY VERIFIED by a human; reviews cited are AI second looks.

## What you will learn

- What each technology in PhantomCache is, in plain English, why the system needs it, and exactly where it is used (`file::symbol`).
- Which technology choices have a **recorded** decision in the repository and which are only **our reasoning after the fact**. This chapter keeps the two apart on purpose.
- The concurrency model: Tokio tasks, `std::sync::Mutex` shards, atomics, the `RwLock` epoch snapshot, and the rule "never hold a std lock across `.await`".
- The real dependency facts: versions, features, unused declarations, two crypto stacks, no Dockerfile, no advisory scan.
- How to answer the hard "why Rust / why Tokio / why Axum / why not Go or C# / why sharded mutexes / why plain CSS" questions without overclaiming.

## 3.0 How to read this chapter: recorded decision versus reasoning

A judge will ask "why did you choose X?". There are two honest kinds of answer.

- **Recorded decision.** A file in the repository says X was chosen and gives the reason. You can point to it.
- **Reasoning after the fact.** No file records the choice. You can still explain why X fits the problem, but you must say "this is our reasoning, not a recorded decision". Claiming a rejected alternative was evaluated when nothing in the repository shows it would be inventing history.

I searched `docs/research/adr/` (ADR-001, ADR-002, ADR-003 and the index), `docs/security/phase-1-specification.md`, `docs/handoff/`, `docs/design/`, `docs/research/design_toolkit/`, and the README. The result:

| Choice | Recorded decision? | Where | Status |
|---|---|---|---|
| Rust as the language | **No.** README lists it ("Memory-safe, zero-cost abstractions") as a fact of the stack, not as a decision with alternatives. `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md` says early "Go/Rust architecture discussions" happened in a ChatGPT conversation that is **not in the repository**. | README section 4; handoff 07 | Reasoning only |
| Tokio runtime | **No selection record.** ADR-002 clause CT-2 records a *use* of it: the origin fetch is detached with `tokio::spawn` so a disconnecting client does not cancel it. | `docs/research/adr/ADR-002-serving-path-contract.md` | Use recorded; choice not |
| Axum | **No.** README lists it. | README section 4 | Reasoning only |
| 8 power-of-two mutex shards | **Described, not decided.** ADR-001 (status PROPOSED) describes "8 shards (HTTP) x default 10,000 entries/shard" as present state. The power-of-two requirement is an `assert!` in code. | ADR-001; `src/core/cache.rs::PartitionedCache::new_with_diagnostics` | Reasoning only |
| rustls, `ring`, `rcgen` instead of OpenSSL | **Yes, a risk-table rationale.** "Use pure-Rust `ring` / `rcgen` and Rustls, eliminating OpenSSL C dependencies" to avoid Windows C-library build failures. | `docs/security/phase-1-specification.md` section 8 | Recorded (but see 3.7: the lockfile still contains C code) |
| `jsonwebtoken` 9.3 and `axum-server` with `tls-rustls` | **Yes, listed** as the dependency plan, with a one-line purpose each. No alternatives are listed. | same file, section 7 | Recorded (plan, no alternatives) |
| React 18 + Vite 5, plain CSS tokens, hand-built SVG, `node:test`, CDP scripts | **Partly.** The UI remediation record states the stack. The toolkit proposal records **rejected** libraries (chart libraries, React Flow, Radix and others) with reasons ("rejected on need, not licence"). A guard test pins the dependency list. | `docs/design/remediation/07_frontend_redesign_implementation_record.md`; `docs/research/design_toolkit/toolkit_proposal.md`; `ui/test/forbidden.test.mjs` | Recorded for what was rejected and why; the original choice of React is not |
| Docker / containers | **Nothing exists.** No `Dockerfile`, no compose file. | repo search | Not applicable |

Two more facts matter for defence. ADR-002 and ADR-003 name AI agents as authors and acceptors, and the ADR index says the owner's ratification is "not recorded in the repository" (no record is not proof of no approval). And nothing in this table was reviewed by a human outside the project.

![Figure 3.1: The technology stack by layer, with locked versions. Dashed boxes are declared but not used.](../assets/ch03_stack_layers.svg)

Source: `Cargo.toml`, `Cargo.lock`, `ui/package.json`, `ui/node_modules/{react,vite}/package.json`, `src/main.rs`, `src/api/server.rs`, `src/security/*`, `src/dist/*`.

## 3.1 The dependency picture in one table

Direct runtime dependencies from `Cargo.toml`, with the resolved versions in `Cargo.lock` (288 locked packages, lockfile v4; reproduced by reading the lockfile, and consistent with `docs/release/g4/cycle1_lab_intelligence_deps_review.md`, an **AI** review, not independent).

| Crate (declared) | Locked | What it is for here | Used in `src`? |
|---|---|---|---|
| `tokio` 1.38, features `full` | 1.53.1 | async runtime, timers, channels, TCP | yes, everywhere |
| `axum` 0.7, features `ws`, `macros` | 0.7.9 | HTTP routing, extractors, SSE | yes; **no WebSocket and no `#[debug_handler]` use found** |
| `axum-server` 0.7, `tls-rustls` | 0.7.3 | TLS listener for the ingress | yes (`security/tls.rs`, `server.rs`) |
| `hyper` / `hyper-util` / `tower` | 1.11.1 / 0.1.20 / 0.5.3 | **transitive** (via axum); not declared | not named in `src` (`hyper::` has zero hits) |
| `tower-http` 0.5, `cors`, `fs` | 0.5.2 (a second copy, 0.6.11, is also locked) | CORS layer | only `cors`; **`ServeDir`/`fs` unused** |
| `serde`, `serde_json` | 1.0.229 / 1.0.151 | JSON for API bodies, lab artifacts, JWT claims | yes |
| `jsonwebtoken` 9 | 9.3.1 | JWT sign/verify | yes (`security/jwt.rs`) |
| `rustls` 0.23, `tokio-rustls` 0.26, `rustls-pemfile` 2.2, `webpki-roots` 0.26 | 0.23.46 / 0.26.6 / 2.2.0 / 0.26.11 | TLS for cluster mTLS, origin HTTPS, ingress | yes |
| `ring` 0.17 | 0.17.14 | SHA-256 | yes (5 files in `src`; fingerprints, anti-entropy ring digest, lab hashes) |
| `rand` 0.8 | 0.8.8 (a second copy, 0.10.3, is transitive) | seeds, run ids, **synthetic dashboard numbers** | yes |
| `tokio-stream`, `futures-util` | 0.1.19 / 0.3.34 | SSE interval stream, stream helpers | yes |
| `url` | 2.5.8 | origin URL parsing | yes |
| `tracing`, `tracing-subscriber` | 0.1.44 / 0.3.x | structured logging | **no use found in `src`, `tests`, `benches`**; logs are `println!`/`eprintln!` |

Dev-dependencies: `jsonwebtoken`, `rcgen` 0.13.2, `reqwest` 0.12.28, `hdrhistogram` 7.6.0, `proptest` 1.12.0, and `criterion` 0.5.1 (declared, **no use found**; the seven benches have `harness = false` and a custom `main`).

Verification: I counted file-level `grep` hits over `src/`, `tests/`, `benches/` (`grep -rln "tracing::" src` returned nothing). "No use found" describes the current tree; it does not prove that a macro-generated or feature-gated use cannot exist. Axum itself pulls in `tracing` and `tokio-tungstenite` through its own features (`Cargo.lock`), so those crates are compiled regardless.

::: caution
Unused declarations (`tracing`, `tracing-subscriber`, `criterion`, the `ws` and `fs` features, UI `lucide-react`) widen the supply chain for no benefit. They are harmless to correctness. They are a fair point for a hostile reviewer, and the honest answer is "declared, not used, a tidy-up for the owner to approve" (the UI audit item UA-15 and the toolkit proposal TK-26 record `lucide-react` as an open owner decision).
:::

## 3.2 Rust

### 1. What it is, in simple English

Rust is a compiled systems language. The compiler checks, before the program runs, that memory is used safely: no use after free, no two threads changing the same data without a lock. Programs run as native machine code with no garbage collector.

### 2. Why PhantomCache needs it

A cache gateway sits on the hot path of every request, holds shared mutable state across many threads, and must not stall unpredictably. Rust makes "this data is shared between threads" explicit in the type system (`Arc`, `Mutex`, `Send`), which is what the core engine does.

### 3. Why it was selected

**Not recorded** (see 3.0). The README presents the choice as a stack fact. Anything below about *why* is our reasoning.

### 4 and 5. Alternatives, and why they were not selected

Go and C# are the two alternatives a judge will name (see the judge box). Nothing in the repository shows they were evaluated against Rust, so I cannot say "they were rejected because...". The handoff index only says that Go/Rust discussions happened in a conversation that is not available.

### 6. How it is used

- Edition 2021 (`Cargo.toml`). No `rust-version` field, no `rust-toolchain` file, no `[profile]` section, so release builds use Cargo defaults (no LTO setting, unwinding panics). The README claims "cargo 1.75+"; the G4C review (F28) found the code needs at least 1.88 (uses `slice::as_chunks`, `is_multiple_of`) and was only verified on 1.98.0 (reported by that review; I did not rebuild).
- `unsafe`: **five `unsafe` blocks in `src/api/lab_api.rs`** (Windows job-object and process-handle calls, each with a `// SAFETY:` comment, for killing child processes) and one `unsafe impl GlobalAlloc` in `benches/support/alloc_counter.rs` (a counting allocator for the memory bench). None in the cache engine (`src/core`, `src/intel`, `src/dist`). I grepped `unsafe` over `src`, `tests`, `benches`. Status: IMPLEMENTED; not reviewed by a human.
- Panics: the runtime is built in `src/main.rs::real_main` with `tokio::runtime::Builder::new_multi_thread().enable_all()`, and the whole `block_on` is wrapped in `std::panic::catch_unwind`. There is no per-request panic containment layer (G4B-28, OPEN).

### 7. Advantages and limits

Advantages: data-race freedom for safe code, no GC pauses, one binary. Limits: slow compiles (the project mandates `-j 1` on this Windows machine), a steep learning curve, and **memory safety does not mean logic safety**: the most serious reported cache finding (G4A-01, a stale-body race; UNDER-INVESTIGATION, confirmed by AI code reading, the owner calls it unconfirmed) would be a logic race that Rust's type system does not prevent. Say that in the defence.

::: judge
**JUDGE MAY ASK**

Question: Why did you choose Rust for a cache?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

A cache gateway keeps shared mutable state that many threads touch on every request, and it sits on the latency path. Rust gives me native speed with no garbage collector, and the compiler forces me to be explicit about what is shared between threads. That removes whole classes of memory and data-race bugs. I want to be straight about the record: the repository does not contain a written decision comparing languages. The README states the stack, and early language discussions happened in a conversation that is not in the repository. So this is my reasoning for why Rust fits, not a documented trade study.

**If the judge asks further:**

"Does Rust make the cache correct?" No. Rust prevents memory errors and data races on safe code. It does not prevent logic races. Our G4A-01 finding, where an old response body can survive an acknowledged invalidation, would be a logic race in safe Rust, and it is UNDER-INVESTIGATION (an AI code reading confirmed it; the owner calls it unconfirmed). I would say Rust reduced one risk class and did nothing for that one.

**Evidence to know:** `Cargo.toml` (edition 2021, no `[profile]`, no `rust-version`); README section 4; `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md` (ChatGPT-Prior row: file not in repository); `grep unsafe`: five blocks in `src/api/lab_api.rs`, none in `src/core`; G4A-01 in `docs/release/g4/cycle1_findings_disposition_register.md` (UNDER-INVESTIGATION). Status: IMPLEMENTED; language choice UNVERIFIED as a recorded decision.
:::

::: judge
**JUDGE MAY ASK**

Question: Why not Go or C#? Go has goroutines, a good HTTP server, and is the language of many caches.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Both would work, and both have strong HTTP stacks and concurrency support. I did not run a head-to-head, and the repository has no record of one, so I will not claim Go or C# were rejected on evidence. My reasoning for Rust is control: no garbage collector means no collector pauses in the tail latency, and ownership makes sharing visible. The cost is real: Rust is slower to write, and a Go service would have been faster to build. If tail latency from the collector turned out not to matter for this workload, Go would be a defensible choice.

**If the judge asks further:**

"Can you show Rust gave better latency than Go?" No. The repository contains no Go or C# implementation and no comparison. Our benchmarks have never been run in a release profile across languages. I would not claim a performance advantage over either. Also note we run on Windows loopback; any such comparison would need a release build, a fixed workload, a seed, and repetitions (CLAUDE.md section 4).

**Evidence to know:** absence of any Go/C# artefact (searched `docs/` and `src/`); `bench-results/README.md` (no release-profile result established); handoff 07 (language talk not in repository). Status: reasoning, not recorded; no performance claim.
:::

## 3.3 Async programming and Tokio

### 1. Simple explanation

A web server spends most of its time **waiting** for a socket, the origin or a timer. Blocking one operating-system thread per waiting request wastes memory and scheduling time. **Async** code lets a task say "I am waiting, run someone else", using `.await`. A **runtime** (here Tokio) owns a small pool of OS worker threads and schedules many lightweight **tasks** across them. A *task* is a unit of async work; a *thread* is an OS resource.

### 2. Why PhantomCache needs it

One process must serve many connections, hold hundreds of requests waiting on one origin fetch (coalescing), run timers (sweeper, heartbeats, anti-entropy) and talk to peers. All of that is waiting, so async fits.

### 3. Why it was selected

No selection record. The strongest recorded link is ADR-002 CT-2: the origin fetch runs in a spawned task so it survives a client disconnect, which is a design that *depends on* a runtime with detached tasks. Test named by the ADR: `test_ct2_initiator_cancellation_detached_fetch` (the ADR marks it VERIFIED; I did not rerun it; it proves the fetch completes after the initiator drops, not that every cancellation path is safe).

### 4 and 5. Alternatives

`async-std`, `smol`, or plain OS threads. Why not chosen: not recorded. Our reasoning: axum 0.7 and `tokio-rustls` (both already required) are built on Tokio, so a second runtime would be a cost with no benefit.

### 6. How it is used (`file::symbol`)

| Tokio feature | Where | Purpose |
|---|---|---|
| multi-thread runtime | `src/main.rs::real_main` (`Builder::new_multi_thread().enable_all()`; default worker count, no `worker_threads` setting found) | run everything |
| `tokio::spawn` | `src/api/server.rs` (leader fetch task; background revalidation task), `src/core/cache.rs::spawn_sweeper_task` | detach work from a request |
| `oneshot` | `src/core/cache.rs::FetchRegistrationResult::Wait`, `src/core/coalesce.rs` | give the one fetch result to many waiters |
| `watch` | `spawn_sweeper_task` (shutdown flag), `src/dist/node.rs`, `src/dist/session.rs` | broadcast a stop or state change |
| `mpsc` (bounded) | `src/dist/router.rs` (`channel::<PeerJob>(PER_PEER_QUEUE)`, replica-invalidation queue of 1024) | queue work per peer with back-pressure |
| `Semaphore` | `src/dist/node.rs` (64 handshake slots), `src/dist/router.rs` (16 permits) | cap concurrent handshakes / fan-out |
| `broadcast` | `src/security/cluster.rs` (`revocation_tx`, capacity 64) | tell connections a peer was revoked |
| `time::timeout`, `time::interval` | `server.rs::fetch_origin_raw` (17 uses of `tokio::time::timeout` in `src`), sweeper | bound every network wait |
| `select!` | `spawn_sweeper_task` | wait on tick or shutdown |
| `spawn_blocking` | `src/api/lab_api.rs` (lab artifact reads) | keep file work off the async workers |

A second thread pool exists only for the Lab: `lab_api.rs` starts named OS threads (`std::thread::Builder`) for experiment runs, wrapped in `catch_unwind`.

### 7. Advantages and limits

Advantages: one runtime for HTTP, TLS and timers; bounded channels and semaphores give explicit resource caps. Limits: a blocking call inside an async task stalls a worker thread (see 3.5). Cancellation is subtle: dropping a future cancels it at the next `.await`, which is why the code uses RAII guards (`OriginPermit::drop` releases the gate slot; `OriginFetchGuard`) so cleanup happens on cancellation too. Status: IMPLEMENTED; the cancellation and guard behaviour is TESTED by several tests but G4A-13 (byte drift in a guard drop path) is OPEN.

::: judge
**JUDGE MAY ASK**

Question: Why Tokio, and what does it give you that threads would not?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Most of what the gateway does is wait on sockets and timers. Tokio lets a few worker threads serve thousands of waiting requests, and gives me the building blocks I need for the design: `oneshot` channels to hand one origin result to many coalesced waiters, bounded channels and semaphores to cap work, and timeouts around every network wait. I did not compare runtimes; the repository records the use of detached tasks in ADR-002 but not a runtime selection. The practical reason is that Axum and `tokio-rustls` are built on Tokio, so it is the natural fit.

**If the judge asks further:**

"What breaks if a task blocks?" It occupies a worker thread. The sweeper takes shard locks synchronously on a worker; the G4A-03 finding says one sweep tick scans a whole shard under its mutex, so a very large shard could hold a worker longer than the documented 1 ms bound. That is OPEN. The main protection today is that critical sections are short and contain no I/O.

**Evidence to know:** `src/main.rs::real_main`; `src/core/cache.rs::spawn_sweeper_task`; ADR-002 CT-2 and test `test_ct2_initiator_cancellation_detached_fetch`; G4A-03, G4A-13 in the disposition register. Status: IMPLEMENTED; sweeper lock-hold bound has a test (`test_sweeper_lock_hold_time_bounded`) but G4A-03 says the batch limit bounds candidates, not keys inspected.
:::

## 3.4 Axum, hyper and tower

### 1. Simple explanation

**hyper** is the low-level HTTP implementation: it parses requests and writes responses. **tower** is a small interface (`Service`) plus reusable wrappers called layers (timeouts, CORS, limits). **Axum** is a web framework on top: it maps a URL and method to an async function (a *handler*), extracts parts of the request (path, headers, JSON), and turns return values into responses. PhantomCache only names Axum and `tower-http` in its code; hyper and tower arrive as transitive dependencies.

### 2. Why PhantomCache needs it

It needs to expose a cache API, health probes, a metrics API, an SSE stream for the UI, and a Lab API, with shared state and middleware (body limit, CORS).

### 3. Why it was selected

Not recorded as a decision. The Phase 1 security specification lists `axum-server` (for TLS) in its dependency plan with the purpose "Ingress TLS 1.3 termination", but it does not compare it with alternatives.

### 4 and 5. Alternatives

`actix-web`, `warp`, or using `hyper` directly. Not recorded. Our reasoning: Axum follows the Tokio and tower ecosystem the project already uses, and extractors keep handlers short.

### 6. How it is used

- Router: `src/api/server.rs::create_app_with_lab` builds `Router::new()`, merges `crate::health::router` and, only when a Lab service is passed, the `/api/v1/lab/*` routes. The route table is in Chapter 2, section 2.3.1; auth is Chapter 6.
- Shared state: `.with_state(Arc<ApiState>)`. `ApiState` holds the `PartitionedCache` and several `std::sync::Mutex` fields (for example `snapshot`, `workload_type`).
- Layers: `DefaultBodyLimit::max(512 * 1024)` and a `tower_http::cors::CorsLayer` built from `CorsConfig` (`PHANTOM_CORS_ORIGINS`).
- Serving: with TLS, `axum_server::bind_rustls(addr, tls_cfg.rustls_config).serve(app.into_make_service())`; without TLS, `tokio::net::TcpListener::bind` then `axum::serve(listener, app)`. `validate_startup_security` fails closed on an unsafe combination (non-loopback bind without TLS or auth, unless explicitly allowed).
- SSE: `axum::response::sse` with a `tokio_stream` `IntervalStream` (10 Hz telemetry). The browser UI does not use the SSE route through `EventSource` for the Lab; it uses fetch-streaming so it can send an `Authorization` header (`ui/src/lib/sse.js`, per the G3 UI notes in `docs/research/progress_ledger.md`).
- **The origin client is hand-written.** `src/api/server.rs::fetch_origin_raw` opens a `tokio::net::TcpStream` to a **pre-validated, pinned address**, optionally wraps it in `tokio_rustls`, writes a literal `GET ... HTTP/1.1\r\nHost: ...\r\nConnection: close` request, and reads with `read_to_end` through a size limit and `tokio::time::timeout`. There is no `hyper` client and no `reqwest` in the runtime path (`reqwest` is dev-only). The pinning comment in `src/security/upstream.rs` ("Pin the first validated address for the socket connection") is why: the address that passed the SSRF check is the address that is connected to. Trade-off (our reasoning): the team controls every byte and can pin addresses, but now owns an HTTP/1.1 response parser; the parser has its own tests (`parser_robustness_tests`, proptest) and its own findings. Status: IMPLEMENTED; TESTED by `security_phase2_upstream_tests`, `parser_robustness_tests`; not independently verified.

### 7. Advantages and limits

Advantages: small handlers, composable layers, no custom server loop. Limits: there is no request-timeout layer on the ingress; the G4B-01 finding concerns a pre-authentication body read (Chapter 6 owns it). Axum's TLS listener config advertises ALPN `h2` and `http/1.1` (`axum-server-0.7.3/src/tls_rustls/mod.rs::config_from_der`, read from the registry source). Whether HTTP/2 is actually served depends on hyper's enabled features, which I did **not** verify: UNVERIFIED.

::: caution
**Discrepancy.** `server.rs` prints "Starting Axum server with TLS 1.3" and the Phase 1 specification says "Ingress TLS 1.3 termination", but the ingress config comes from `RustlsConfig::from_pem_file`, which in `axum-server 0.7.3` calls `ServerConfig::builder()` with no protocol-version restriction. By rustls documentation (not re-verified here) that allows TLS 1.2 and 1.3. The only test that rejects a TLS 1.2 client is for the **cluster** listener (`test_p3_sec04_strict_tls13_rejection_of_tls12_client`). Ingress "TLS 1.3 only" is therefore UNVERIFIED by test; the G4 register records the mismatch as G4B-20 (CONFIRMED by AI reading, OPEN, Low); the ingress tests (`test_phase1_at01`, `at02`) check that valid certificates work and untrusted ones fail.
:::

::: judge
**JUDGE MAY ASK**

Question: Why Axum rather than another framework, and why did you write your own HTTP client for the origin?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Axum is a thin framework on hyper and tower, on the same Tokio runtime as the rest of the system, so routing, state, body limits and CORS were a few lines each. I did not compare frameworks and the repository does not record a comparison. For the origin side I wrote a small HTTP/1.1 client because I needed to connect to an address I had already validated against SSRF rules, send a fixed minimal header set, cap the response size, and time out every step. That is a deliberate control decision, and the cost is that I own a response parser, so it has its own tests and its own review findings.

**If the judge asks further:**

"Is your parser as safe as hyper's?" I cannot claim that. Hyper is a widely used public library; I have not reviewed its test history, and ours has only unit and property tests (`parser_robustness_tests`) and AI review, not independent review. The Connection: close, no keep-alive design also means every miss pays a fresh TCP (and TLS) handshake; that cost has not been measured in a release build.

**Evidence to know:** `src/api/server.rs::create_app_with_lab`, `fetch_origin_raw`; `src/security/upstream.rs`; `tests/integration/parser_robustness_tests.rs`, `security_phase2_upstream_tests.rs`; `Cargo.lock` (`hyper` is transitive). Status: IMPLEMENTED, TESTED; decision not recorded.
:::

## 3.5 Concurrency primitives and the concurrency model

This is the most important section of the chapter for your defence. Read it twice.

### 3.5.1 The two kinds of waiting

- **Async waiting** (`.await`): the task is parked and the thread runs someone else. Used for sockets, timers, channel receives.
- **Synchronous locking** (`std::sync::Mutex::lock()`): the *thread* waits until the lock is free. Used here for very short in-memory sections.

`tokio::sync::Mutex` exists and can be held across `.await`, but it is slower and is not used for the cache shards. PhantomCache uses `std::sync::Mutex` because each critical section is a few hash-map operations with no I/O. The **rule** (CLAUDE.md section 5, and the comment on `OriginGate` in `src/intelligence/executor.rs`) is: never hold a `std::sync` guard across an `.await`. Figure 3.2 shows why.

![Figure 3.2: Why a std Mutex guard must not be held across an await.](../assets/ch03_lock_vs_await.svg)

Source: `src/api/server.rs` (`Initiate` branch: the `waiters.lock()` push sits in its own `{ ... }` block before any await), `src/core/cache.rs::spawn_sweeper_task`, `src/intelligence/executor.rs::OriginGate` doc comment.

The compiler helps: `std::sync::MutexGuard` is `!Send`, so a guard alive across an `.await` inside a `tokio::spawn` task (or an Axum handler, which must be `Send`) normally does not compile. That is a language rule about `Send`; it is not a proof that the tree contains no violation, and I did not audit every function. The G4C review reports it read the executor code and found no std Mutex held across `.await` there (AI review).

### 3.5.2 Shards: the partitioned mutex buckets

**Simple explanation.** One lock for the whole cache means every request waits in one line. Split the cache into N independent buckets, each with its own lock; a request goes to the bucket its key hashes to. Two requests collide only if their keys land in the same bucket.

**How it works** (`src/core/cache.rs::PartitionedCache`):

```text
shard_index(key) = hash(key) & shard_mask        // shard_mask = num_shards - 1
guard = shards[shard_index].lock()                // one std Mutex per shard
... look up / insert in guard.map ...             // short, no I/O
drop(guard)
```

`new_with_diagnostics` asserts `num_shards.is_power_of_two()`, so `& mask` replaces a modulo. `src/main.rs` builds the serving cache with `PartitionedCache::new(8)`; the per-node demo caches in `src/dist/node.rs` use 4. The hash is `std::collections::hash_map::DefaultHasher::new()` over the `CacheKey` (`get_shard_index`).

**Worked example (illustrative numbers).** Eight shards, mask `0b111`. Suppose a key hashes to `0x9A3F...B5`. The low three bits of `0xB5` (`1011 0101`) are `101` = 5, so the key lives in shard 5. If the hash were uniform and requests independent, the chance that two simultaneous requests need the same shard is 1/8 = 12.5%; with one global lock it is 100%. The uniformity and independence assumptions are exactly what a hot key breaks: 10,000 requests for one popular key all use one shard, so sharding gives that key no benefit (Chapter 4 covers hot keys).

Each shard (`CacheShard`) holds the key map (`HashMap<CacheKey, KeyState>`), the single-flight map (`in_flight: HashMap<CacheKey, Arc<SharedOp>>`), byte accounting, and a boxed eviction policy. Default `max_entries_per_shard` is 10,000.

**What can go wrong.**

- *Hot key or skew*: one shard becomes the bottleneck (our reasoning; measured by `b2_concurrency` bench source, but no release-profile result exists in the repository).
- *Predictable shard choice* (hypothesis, not tested): `DefaultHasher::new()` uses fixed keys, so shard choice is deterministic and not secret; by std documentation (recalled, not re-verified) the algorithm is **not guaranteed stable across Rust releases**. Consequences: (a) a client that can choose many keys could aim them all at one shard; (b) rendezvous routing uses the same hasher (`src/dist/router.rs::calculate_hash`), so nodes built with different toolchains could disagree about ownership. The cluster therefore assumes one binary on all nodes; I found no document stating that requirement (UNVERIFIED as an explicit requirement). The inner `HashMap`s use the randomised `RandomState`, which is fine.
- *Poisoning*: if a thread panics while holding a std lock, the lock becomes poisoned. Cache shard locks are recovered with `Err(p) => p.into_inner()` and a counter `lock_poison_recovered`. Other sites still call `lock().unwrap()` and would panic (G4A-21 and G4B-28, both OPEN). Status: PARTIAL.
- *Sweeper hold time*: `sweep_lazy_reclamation` loops over shards and holds each shard's lock while it scans `shard.map.iter()`; the batch limit stops *collecting candidates*, not iterating (G4A-03, OPEN).

::: judge
**JUDGE MAY ASK**

Question: Why partitioned mutex buckets? Why not a lock-free concurrent map, or one `RwLock`?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The cache keeps more than a key and a value. Each key has a generation, active operations, an in-flight fetch registration, and eviction bookkeeping, and the correctness rules in the system, like "never evict a key with an active fetch" and "only store a fetched result if the generation still matches", need several of those updated together. A mutex around a shard makes that multi-field update trivially atomic. Splitting into eight shards cuts contention without giving that up. I should be clear that the repository does not record a comparison with lock-free maps; ADR-001 only describes the shard layout as it was. This is my reasoning, and the cost is that a hot key or a long critical section still serialises on one lock.

**If the judge asks further:**

"Have you measured the contention benefit?" The benchmark sources exist (`benches/b2_concurrency.rs`), but the repository states no release-profile benchmark has been established on the current tree (`bench-results/README.md`), so I do not quote a speed-up. "Why 8?" It is a constant in `main.rs`, not tuned; a power of two is required by the mask.

**Evidence to know:** `src/core/cache.rs::PartitionedCache`, `get_shard_index`, `CacheShard`; `src/main.rs` (`PartitionedCache::new(8)`); `tests/integration/concurrency_verification_tests.rs`; G4A-03, G4A-21. Status: IMPLEMENTED; TESTED for correctness under stress and a proptest model, which does not prove performance or absence of every race.
:::

### 3.5.3 Atomics

An **atomic** is a number the hardware updates in one indivisible step (`fetch_add`), with no lock. PhantomCache uses them for counters and flags that do not need to change together with other data.

- `AtomicU64` / `AtomicUsize` counters (hit, miss, revalidation counts, `next_op_id`) and settings (`max_stale_ms`, `sweeper_batch_size`, `max_background_refresh`) in `PartitionedCache`, read with `Ordering::Relaxed`. In `cache.rs` I counted 62 lines using `Relaxed` and 7 using `SeqCst` (a `grep -c` of lines).
- `global_epoch: AtomicU64`, loaded and stored with `SeqCst`.
- `OriginGate` (`src/intelligence/executor.rs`): `in_flight` and `cap` are atomics; `OriginPermit::drop` does `fetch_sub`.
- `AtomicLogLinearHistogram` (`src/intel/telemetry.rs`): `Vec<AtomicU64>` buckets; `record` is two `Relaxed` `fetch_add` calls.

*Memory ordering in one paragraph.* `Relaxed` guarantees the increment is not lost but promises nothing about when other threads see it relative to other memory. That is fine for a counter you only display. It is **not** fine for a value that guards other data. A reader of the histogram can see `total_count` and the buckets at slightly different moments, so a percentile read concurrently with writes is approximate (our reasoning; a snapshot-consistency test was not checked).

**The histogram in one line.** `AtomicLogLinearHistogram` holds 2,688 `AtomicU64` buckets (128 exact plus 20 octaves of 128), about 21 KB. Chapter 5, section 5.11 owns the maths and the worked example (1000 us lands in bucket 506 and reads back as 1002 us, error 0.2%; worst case 0.39%). I recomputed that example from `value_to_bucket` and `bucket_to_value`: it is correct. Status: IMPLEMENTED; TESTED (`telemetry_redesign_tests::test_histogram_relative_error_bounded` checks round-trip error on its sampled values, not every input). Chapter 5 also records that no production reader of this histogram was found; this chapter covers only the recording design.

::: caution
Two comments in `src/intel/telemetry.rs` disagree with the code: a section header says "relative error <= 0.5%" while the function comment says 0.39% (the code supports 0.39%), and the constant comment "2^7 to 2^27 (128 us to ~67.1 seconds)" is wrong by a factor of two: 2^27 microseconds is about 134 seconds (67.1 s is 2^26). The bucket arithmetic is unaffected. The product is not edited.
:::

### 3.5.4 The RwLock epoch snapshot

An **epoch** is a counter that says "everything stamped with an older number is stale" (Chapter 4 explains invalidation). Every request must read its tenant's epoch. If that read had to take the `tenant_epochs` mutex, every request would queue on one lock.

Design (`PartitionedCache`): `tenant_epochs: Mutex<HashMap<String, u64>>` is the writable truth, touched only by invalidations. `epoch_snapshot: RwLock<Arc<HashMap<String, u64>>>` is a read-only copy. `get_effective_epochs` takes the **read** lock, looks up the tenant, and takes the `global_epoch` atomic. Many readers share a `RwLock` read lock at once. An invalidation updates the map under the mutex, then replaces the snapshot.

The honest details:

- The replacement is `let mut snap = epoch_snapshot.write()...; *snap = Arc::new(guard.clone());`. The O(n) clone of up to `MAX_TENANT_EPOCHS = 10_000` entries runs **while the write lock is held**, so readers pause for it: "readers never block" holds except during an invalidation. G4A-12 (PLAUSIBLE, OPEN) flags the clone and unvalidated peer-supplied tenant strings. A smaller fix (our reasoning): build the new `Arc` before taking the write lock.
- The tenant and global epochs are read separately, so a reader can see a mixed pair during `invalidate_all` (G4A-17, PLAUSIBLE, OPEN).
- Overflow: when 10,000 tenants exist and a new one arrives, the code increments `global_epoch` and clears the map (SEC-4A-01). That fences everything, so it is a cache flush.
- Tests: `tests/integration/epoch_contention_tests.rs` (four tests including `test_concurrent_epoch_reads_scaling` and `test_invalidate_during_concurrent_reads_linearizable`). I did not rerun them. The word "linearizable" in that test name is a label, not a proof: the test shows behaviour under its own workload, not linearizability and not a bound on reader pause time.

**Lock order** (CLAUDE.md section 5; I confirmed the parts I read): shard mutex first; `get_effective_epochs` (RwLock read plus an atomic) is called from inside shard-locked code such as the sweeper; the invalidation functions take `tenant_epochs` then the snapshot write lock and take no shard lock. Under that pattern there is no cycle.

![Figure 3.3: The concurrency model: tasks, eight shard mutexes, the epoch snapshot, atomics, and the rules.](../assets/ch03_concurrency_model.svg)

Source: `src/core/cache.rs::PartitionedCache` (fields), `get_effective_epochs`, `try_invalidate_tenant`, `try_invalidate_all`, `spawn_sweeper_task`; `src/main.rs` (`PartitionedCache::new(8)`); G4A-03, G4A-12, G4A-17, G4A-21, G4B-28.

### 3.5.5 Arc, channels, and the other mutexes

- `Arc<T>`: shared ownership across tasks. The cache, the state, bodies (`CachedResource.body: Arc<Vec<u8>>`) are `Arc`s. A cache hit clones the `Arc` pointer for the resource but the handler still copies the body bytes into the response (`(*resource.body).clone()` in the response-building branches of `get_cache_handler`; the "body clone per hit" is noted in G4A-26).
- `oneshot` for coalescing: waiters are `Vec<oneshot::Sender>` in `SharedOp.waiters` (itself a `std::sync::Mutex`); the leader pushes its own sender in a tiny block, then spawns the fetch, then awaits its receiver. Bounded by `max_waiters_per_key` (default 1,000).
- Other `std::sync::Mutex` uses: `eviction_policy`, `AuditStore` behind a process-wide `OnceLock` (`GLOBAL_INTEL_AUDIT`), router counters, the repair ledger, the OriginGate's observation window and AIMD controller, telemetry buffers. All are meant to be short; none is verified free of await-crossing by me.

::: judge
**JUDGE MAY ASK**

Question: What are the concurrency implications of your design? Where could it deadlock or stall?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The rule is simple: short synchronous critical sections under `std::sync::Mutex`, and no lock held across an `await`. The slow work, the origin fetch, runs in a separate spawned task with no lock held, and the result is handed to waiters through channels. The lock order is shard first, then the epoch structures, and the invalidation path never takes a shard lock, so I see no lock cycle in what I read. The real risks I know of are these: a long sweep scan holds a shard lock on a worker thread (G4A-03), the epoch snapshot rebuild clones up to 10,000 entries while readers wait (G4A-12), and some locks still panic on poison (G4A-21). All three are open findings, not fixed.

**If the judge asks further:**

"Is it linearizable?" I do not claim that. I claim the single-flight and generation-fence rules, which have a protocol-model test and a stress test, and I tell you honestly that G4A-01 is a reported race (UNDER-INVESTIGATION) where an old body might survive an acknowledged invalidation. "How do you know there are no deadlocks?" I do not have a proof; I have a lock-order argument from reading the code and stress tests (`test_real_cache_concurrent_stress_schedule`) that did not hang, which shows absence of hangs in those schedules, not absence of deadlock.

**Evidence to know:** `src/core/cache.rs` (`get_effective_epochs`, `try_invalidate_tenant`), `src/intelligence/executor.rs::OriginGate` doc comment; `tests/integration/concurrency_verification_tests.rs`, `epoch_contention_tests.rs`; G4A-01, -03, -12, -17, -21, G4B-28 in the disposition register. Status: IMPLEMENTED; TESTED (stress and model); not INDEPENDENTLY VERIFIED.
:::

## 3.6 Data structures as actually used

| Structure | Where | Why this one (our reasoning) | Notes |
|---|---|---|---|
| `HashMap<CacheKey, KeyState>` per shard | `CacheShard.map` | O(1) average lookup by key | `CacheKey` includes tenant and path (Chapter 4) |
| `HashMap` + `HashSet` | in-flight map, tenant epochs, S3-FIFO ghost set and frequency map | membership tests | |
| `Vec<Option<Node>>` + `free_list: Vec<usize>` + `HashMap<CacheKey, usize>` | `FaithfulSieve`, `FaithfulLru` (`core/eviction.rs`) | an **arena-based doubly linked list**: nodes live in a `Vec`, links are indices, freed slots are reused. Gives O(1) unlink and move without unsafe pointers | `hand: usize` for SIEVE |
| `VecDeque<CacheKey>` x 3 + `HashSet` + `HashMap<_, u8>` | `FaithfulS3Fifo` | FIFO queues (small, main, ghost) | G4A-02: `on_remove` leaves tombstones; OPEN; default policy is SIEVE |
| `Vec<CacheKey>` + `HashMap<_, bool>` | `ClockScan` | legacy O(n) scan fallback (`on_remove` uses `retain`) | not the default |
| `Vec<AtomicU64>` of 2,688 | `AtomicLogLinearHistogram` | lock-free log-linear latency histogram | see 3.5.3 |
| `VecDeque` ring buffers | diagnostics, audit store, anti-entropy ring, engine switch history | bounded history with O(1) push/pop at both ends | capacities are explicit |
| `BTreeMap` / `BTreeSet` | `config.rs` (sorted config keys), `phantom-lab` reports, `intelligence/mod.rs` parameters, anti-entropy `resync_needed` | deterministic iteration order for output and hashes | |
| fixed array `[u64; 15]` | `origin_metrics::LATENCY_BUCKETS_MS` | coarse origin latency buckets | |
| `Vec<Vec<u32>>` | `intel/sketch.rs::CountMinSketch` | a probabilistic frequency counter | **no use outside its own file** (`grep CountMinSketch src`); PROTOTYPE ONLY |
| bitmaps | none found | | I searched for bitmap/bit-vector types and found none; "visited" flags are `bool`s |

Complexity claims (CLAUDE.md "O(1) amortised") belong to Chapter 4; the G4A-20 finding says pinned-key scans are not O(1), OPEN.

## 3.7 The TLS and mTLS stack

### 1. Simple explanation

**TLS** encrypts a connection and lets the client check the server's identity with a certificate. **mTLS** (mutual TLS) also makes the server check the *client's* certificate, so each side proves who it is. PhantomCache uses TLS in three separate places: the ingress (browser or client to gateway), the origin (gateway to your backend over HTTPS), and the cluster (node to node).

### 2. Why it needs it

The cluster carries invalidation commands; a forged one would purge or poison caches. Ingress TLS protects tokens in transit. (Trust analysis is Chapter 6.)

### 3. Why selected

**Recorded:** `docs/security/phase-1-specification.md` section 8 lists "Windows OpenSSL / C library build failures" as a risk and mitigates it with "pure-Rust `ring` / `rcgen` and Rustls, eliminating OpenSSL C dependencies". `docs/security/phase-3-design.md` plans `tokio_rustls::TlsAcceptor` / `TlsConnector` for the cluster.

### 4 and 5. Alternatives

OpenSSL (via `native-tls` or `openssl`), `aws-lc-rs` directly. The recorded rejection reason for OpenSSL is the Windows build risk above. No other comparison is recorded.

### 6. How it is used

- **Cluster mTLS**: `src/security/cluster.rs` builds `rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])` with `WebPkiClientVerifier` (client certificate mandatory), and a matching `ClientConfig` with `with_client_auth_cert`. After the handshake, `src/dist/node.rs` reads `peer_certificates()`, then calls `security::cluster::extract_sans_from_der` (a **hand-written ASN.1/DER walker**, not an X.509 library) to pull DNS SANs; exactly one SAN is required (SEC-3A-03). `compute_cert_fingerprint` is SHA-256 via `ring::digest`, and `parse_san_node_identity` enforces the `node-<id>.<cluster>` shape ("SAN node label must start with 'node-'"); both are in `src/security/cluster.rs`. Status: IMPLEMENTED; TESTED by `security_phase3_cluster_tests` including TLS 1.2 rejection.
- **Origin TLS**: `src/security/upstream.rs::UpstreamTlsConfig` with `webpki_roots::TLS_SERVER_ROOTS` (Mozilla roots as data) or a custom CA PEM; `ClientConfig::builder()` with default versions; no client auth.
- **Ingress TLS**: `axum-server` as in 3.4; optional; see the TLS 1.3 discrepancy.
- Cryptographic provider: `rustls 0.23.46` depends on `aws-lc-rs` (C code via `aws-lc-sys 0.46.0`, needs a C toolchain); `ring 0.17.14` is also a direct dependency. **Two crypto stacks are linked** (per `Cargo.lock` and the G4C inventory). Separately, dev-only `reqwest` pulls `native-tls` and `openssl 0.10.81` (on Windows it uses `schannel`, per the G4C review).

### 7. Advantages and limits

Advantages: memory-safe TLS logic, no OpenSSL link on Windows runtime paths, strict TLS 1.3 on the cluster. Limits: the recorded rationale "eliminating OpenSSL C dependencies" is only partly true: the default rustls provider is C-backed `aws-lc-sys`, and the dev tree has OpenSSL crates. The hand-written DER parser must be right on attacker-influenced bytes; I did not review it, and that it runs only after rustls has verified the chain is the intent per the comments, UNVERIFIED by test here. Demo certificates come from `scripts/gen-demo-certs.*`; never put real keys in the repository.

::: judge
**JUDGE MAY ASK**

Question: Why rustls and not OpenSSL for the mTLS between nodes? And why do you have both `ring` and `aws-lc-rs`?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

There is a recorded reason: the Phase 1 specification chose Rustls with `ring` and `rcgen` to avoid OpenSSL C-library build problems on Windows, where the project is developed. Rustls also lets me pin the cluster to TLS 1.3 only with a mandatory client certificate in a few lines, and a test checks that a TLS 1.2 client is refused. On the second part: the lockfile has two crypto libraries because I use `ring` directly for SHA-256, and rustls 0.23 defaults to `aws-lc-rs`. So the claim "no C crypto" in the spec is not literally true of the build, and I would recommend choosing one provider.

**If the judge asks further:**

"Is the node identity check sound?" It requires a certificate chaining to the cluster CA, exactly one DNS SAN of the expected form, and a fingerprint match against a registry. That is static membership; revocation is a broadcast flag (`revocation_tx`). It has been tested on loopback with generated certificates. It has not had independent security review, and there is no real multi-host deployment evidence.

**Evidence to know:** `src/security/cluster.rs` (`builder_with_protocol_versions`, `WebPkiClientVerifier`, `extract_sans_from_der`, `compute_cert_fingerprint`); `docs/security/phase-1-specification.md` section 8; `Cargo.lock` (`aws-lc-sys`, `ring`, `openssl`); `test_p3_sec04_strict_tls13_rejection_of_tls12_client`. Status: IMPLEMENTED, TESTED (loopback); not INDEPENDENTLY VERIFIED.
:::

## 3.8 JWT: jsonwebtoken

**Simple.** A JSON Web Token is a signed note: it says who you are, which role you hold, and when it expires; the server checks the signature with a shared secret.

**Why needed.** The control, delete, telemetry and Lab routes must be limited by role. **Why selected:** recorded as a dependency plan in the Phase 1 spec (`jsonwebtoken = "9.3"`, purpose "encoding, decoding, and cryptographic validation"); no alternative listed.

**How used (`src/security/jwt.rs`).** `JwtValidator` decodes with `jsonwebtoken::decode_header`, **rejects any algorithm other than HS256**, selects a secret by `kid` (rotation map) or the primary, and runs `Validation::new(HS256)` with `leeway = 10`. `JwtClaims` fields: `sub`, `iss`, `aud`, `exp`, optional `iat`, `tenant_id`, `roles`. Roles map to strings such as `cache:read`, `cache:invalidate`, `telemetry:read`, `scenario:operate`, `admin`. Status: IMPLEMENTED; TESTED (`security_phase1_tests`, `m7_sec_*`).

**Discrepancy.** The Phase 1 specification lists HS256, **RS256 and EdDSA** as allowed algorithms; the code accepts HS256 only. A symmetric HS256 secret means every verifier can also mint tokens, which is acceptable for a single gateway and weak for a multi-party setup. Revocation, `nbf`/`jti` handling are open (SEC-M7-09/10/11; Chapter 6). Limit: the library is a general one; correct use (algorithm pinning, required claims) is our code's job.

## 3.9 serde, JSON and the binary frame protocol

**Simple.** `serde` turns Rust structs into text or bytes and back. `serde_json` is the JSON format. Most of the system's inputs and outputs are JSON: API bodies, lab artifacts (`manifest.json`, `summary.json`), JWT claims, SSE events, the audit store.

**Where.** 19 source files derive `Serialize`/`Deserialize`; the lab uses it heavily (`src/lab/*`, `src/lab_live/*`). **Not used for the node-to-node wire.** `src/dist/frame.rs` defines a hand-written binary protocol: magic bytes `0x50 0x48 0x41 0x4E` ("PHAN"), version 1, a 28-byte big-endian header (flags, request id, sequence number, payload length), and fixed-size payloads such as the 50-byte anti-entropy sync request (8 epoch + 8 seq + 1 + 1 reserved + 32 digest). Our reasoning for a binary frame: bounded, length-checked parsing with no allocation driven by untrusted JSON; the cost is a protocol to version and fuzz by hand (`security_phase3b_framing_tests`, `parser_robustness_tests`). The choice is not recorded as an ADR; it is described in the Phase 3 design (not re-read in full for this chapter).

**Limits.** Hashing JSON is fragile: the G4C review found `compute_manifest_hash` (`src/lab/manifest.rs`) serialises a struct containing `HashMap`s, whose iteration order differs per process, so the digest of one logical manifest can differ (the production lab path uses a different manifest in `eval.rs`; this struct is only used by a test). Use `BTreeMap` where bytes get hashed.

## 3.10 Build, packaging and deployment: what exists and what does not

| Item | Exists? | Evidence |
|---|---|---|
| `Dockerfile`, `docker-compose`, Kubernetes manifests, Helm | **No** | searched the repository outside `node_modules` for `Dockerfile*` and `docker-compose*`: none; grep in `packaging/`, README, `docs/user` found no container references |
| Release build scripts | Yes | `packaging/build-release.sh` / `.ps1`: `cargo build --release --locked --bin phantom-cache-demo`, stamps `PHANTOM_BUILD_COMMIT` and `PHANTOM_BUILD_DATE` (read via `option_env!` in `src/config.rs`), packs a `.tar.gz` with checksums; "does NOT commit, tag, push or publish" |
| Smoke test of a bundle | Yes | `packaging/smoke-test.sh` / `.ps1`: `--version`, `--check-config`, rejects `PHANTOM_REQUIRE_AUTH=yes`, starts on a loopback port and probes `/health`, `/health/live`, `/health/ready` |
| Example configs | Yes | `packaging/phantom.example.conf`, `phantom.production.example.conf` (KEY=VALUE; precedence command line > environment > file > defaults) |
| Demo launcher | Yes (Windows) | `Start-Phantom-Demo.bat` (checks node/npm, port 8085) |
| Binaries | Two | `phantom-cache-demo` (`src/main.rs`) and `phantom-lab` (`src/bin/phantom-lab.rs`) |
| Process manager (systemd unit etc.) | **No** | not found |
| CI | A workflow file exists | `.github/workflows/ci.yml`: `ubuntu-latest`; `cargo fmt --check`, `cargo check`, `clippy -D warnings`, `cargo test` (note: **no `-j 1`, debug profile, all features**); frontend job: Node 20, `npm ci`, `npm run build` (it does **not** run `npm test`). Whether it ever ran on GitHub: UNVERIFIED (no run history in the repo) |
| Release-profile benchmarks | No result on record | `bench-results/README.md` |
| Clean-machine reproduction | Not done | `docs/STATUS.md` (M7 row) |

Honest statement for a judge: **PhantomCache has no container image and no orchestration**. It is one binary plus scripts, and the cluster path is validated on one host's loopback. Say "deployment packaging for a single binary exists; container and multi-host deployment are not built". CI runs on Linux while all measured work was on Windows, so a reproducible Linux CI result is UNVERIFIED. Chapter 2, section 2.9.3 covers the same ground from the architecture side.

## 3.11 The frontend: React, Vite, plain CSS, hand-built SVG

### 1. What it is

React builds the user interface from components that re-render when data changes. Vite is the build tool and dev server. CSS styles the page; here it is plain CSS files with variables ("tokens"). SVG is the vector image format used for charts.

### 2. Why needed

Operators need to see live telemetry and, in Phantom Lab, run and read experiments. The repository rule is that every displayed value is labelled measured, simulated, inferred or unavailable (CLAUDE.md section 4), which the UI enforces with a `ProvenanceBadge` component.

### 3. Why selected

The stack is **recorded as fact** in `docs/design/remediation/07_frontend_redesign_implementation_record.md`: "React 18 + Vite 5 single-page app; plain CSS with tokens (`ui/src/styles/*.css`); hand-built SVG charts with table twins; dependency-free CDP scripts (`ui/scripts/cdp-lib.mjs`) for browser evidence; `node:test` tests. Only dev-dependency change: axe-core 4.14.0." The choice of React over other frameworks is **not** recorded. What *is* recorded is a list of **rejected** libraries in `docs/research/design_toolkit/toolkit_proposal.md`: chart libraries (d3, Chart.js, recharts and others), React Flow, Excalidraw, Mermaid in-app, headless primitive libraries (Radix etc.; "adopt only if a hand-built widget fails the keyboard script or axe"), motion libraries, Storybook and visual-regression services. Stated reasons: size, dependency count, canvas not readable by assistive technology, "rejected on need, not licence". Their test for revisiting: one chart exceeds about 2,000 SVG nodes, or live series outrun the backend.

### 4 and 5. Alternatives and why not (where evidence exists)

Chart libraries: rejected for the reasons above (recorded). Tailwind, shadcn: the proposal notes they need Tailwind and fail the dependency guard (`forbidden.test.mjs`). Other frameworks (Vue, Svelte): no record.

### 6. How it is used

- `ui/package.json`: dependencies `react` ^18.3.1 (installed 18.3.1), `react-dom`, `lucide-react` (**declared, zero imports**); devDependencies `vite` ^5.2.0 (installed 5.4.21), `@vitejs/plugin-react`, `axe-core` 4.14.0 pinned. `ui/vite.config.js` sets the dev port 3000.
- Plain CSS: `ui/src/styles/{tokens,app,shell,pages,sim,lab}.css` (about 1,000 lines). `tokens.css` declares colours once with `light-dark(...)`, so light and dark are designed separately; browser baseline Chrome 123, Safari 17.5, Firefox 120 with deliberately no fallback. A test enforces that colours live only in `tokens.css` (`ui/test/forbidden.test.mjs`).
- Hand-built SVG: `ui/src/components/Charts.jsx` (for example a sparkline `<svg className="spark" viewBox=...>`), `LabCharts.jsx` (CSS charts each with a "View as table" twin).
- Guard tests: `forbidden.test.mjs` asserts the exact dependency lists, greps every UI source file for a literal `fetch(`, `XMLHttpRequest`, `sendBeacon` or `WebSocket` (the Lab client calls the network through `(fetchImpl || globalThis.fetch)(...)` in `ui/src/lib/labApi.js`, which that regex does not match, so this test is a tripwire, not proof that the UI contacts only the gateway), and blocks the old fabricated numbers ("94%", "PIE trace", ...).
- Token handling: held in a React ref, never in URL or storage (G3 notes; the D-7 browser verification observed this; Chapter 6 owns the security view).
- Tests: `node --test ui/test` (30 `.mjs` test files), `check:contrast`, `check:a11y`, `check:axe`, `check:forbidden`. The count changed during the day as the redesign added tests (298 to 299 to 483); the one canonical explanation is in Chapter 8, section 8.2.2 (AI/lead-run, not independently verified).
- Browser evidence: `ui/scripts/cdp-lib.mjs` launches the user's installed Chrome headless with a throwaway profile and drives it over the Chrome DevTools Protocol using Node built-ins only; scripts such as `browser-journey.mjs`, `browser-a11y.mjs`, `browser-r3-*.mjs`. Hard-coded Chrome path and `D:/Temp` profile directory: Windows-specific.

### 7. Advantages and limits

Advantages: a very small dependency set (react, react-dom, vite, plugin) and no chart library to audit; every chart has a table twin for accessibility. Limits: more code to write and maintain by hand (custom charts, SSE client), `lucide-react` is dead weight, and visual quality **is not approved** (STATUS: O8-2 open). Axe results: the first run was false-clean because the page had not rendered (lesson recorded in the remediation record). All browser evidence is from one Chrome version on one machine; no human usability or screen-reader study has been done.

::: judge
**JUDGE MAY ASK**

Question: Why React with plain CSS and no UI or chart libraries? Isn't that reinventing the wheel?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Two reasons, and one of them is recorded. The recorded part: the toolkit proposal evaluated chart libraries, graph libraries and headless component libraries and rejected them on need: bundle size, dependency count and accessibility of canvas charts, with a rule for when to revisit, for example a chart that exceeds about two thousand SVG nodes. A guard test pins the dependency list so it cannot grow silently. The other part is my reasoning: the project's rule is that every number on screen is labelled measured, simulated, inferred or unavailable, and hand-built components let that label be part of every chart rather than something a library hides. The cost is real: more code, a hand-built SSE client, and one unused declared dependency.

**If the judge asks further:**

"Is the UI good?" I can say it is tested, not that it is approved. Tests, contrast and lint checks pass in the project's records; browser rounds found real defects (overflow at narrow widths, small tap targets) that were then fixed by the UI writer, and visual quality is still marked not approved by the owner. There has been no human usability study or screen-reader test. "Why React and not something lighter?" There is no recorded comparison.

**Evidence to know:** `ui/package.json`; `ui/test/forbidden.test.mjs`; `docs/design/remediation/07_frontend_redesign_implementation_record.md`; `docs/research/design_toolkit/toolkit_proposal.md` (rejected list); `docs/STATUS.md` M8 row. Status: IMPLEMENTED, TESTED; visual quality unapproved; React choice not recorded.
:::

## 3.12 Testing and benchmarking tools

| Tool | Role | Where | What it does not prove |
|---|---|---|---|
| `cargo test` | unit and integration tests | `tests/unit/wp1..wp4_tests.rs`, 72 `[[test]]` targets in `Cargo.toml`, many in `tests/integration/` | pass/fail on one machine, debug profile |
| In-crate tests | module tests | `#[test]` in `src/` | |
| `proptest` 1.12 | property-based tests (random inputs, shrinking) | 4 `proptest!` blocks: `byte_accounting_tests`, `canonical_key_roundtrip_tests` (regression file kept), `concurrency_verification_tests` (random op sequences against a model), `parser_robustness_tests` | random search is not proof; default case counts |
| `rcgen` 0.13 | generates test certificates in memory | cluster and ingress TLS tests | |
| `reqwest` 0.12 | real HTTP client in tests | 34 test files | |
| `tests/support/` | cluster harness, `fault_proxy.rs`, lab test doubles | | fault proxy is loopback only |
| Custom bench mains | seven benches, `harness = false`: `b1_hit_path`, `b2_concurrency`, `b3_coalesce`, `b4_eviction`, `b6_telemetry`, `b7_memory` (with `alloc_counter.rs`, a counting `GlobalAlloc`), `b8e_gateway` | `benches/` | **no release-profile result on the current tree** per `bench-results/README.md` |
| `hdrhistogram` 7.6 | high-resolution latency recording in benches | `b2_concurrency`, `b8e_gateway` | a different histogram from the one in `src/intel/telemetry.rs` |
| `criterion` | declared; no use found | `Cargo.toml` | |
| `node:test` | UI unit and contract tests | `ui/test/` | |
| CDP scripts | real-browser journeys and a11y probes | `ui/scripts/` | one Chrome, one machine |
| `scripts/` | `reproduce_tests`, `cleanroom_build`, `run_audit_suite`, `docs-lint.mjs` | | |

Reported baseline (`docs/STATUS.md` section 3; `CLAUDE.md` still quotes the older 242 tests in 19 targets): 851 passed, 0 failed, 8 ignored, 75 targets, debug profile, on 2026-10-30 (reported by the lead and agents; I did not rerun it). The counts are different measures (Chapter 8, section 8.2.2). The project's own rule is that passing tests do not establish correctness; several tests assert stub behaviour (`docs/research/code_findings.md`). A mutation experiment recorded in the progress ledger (disabling auth gates failed 13 of 22 and 4 of 10 tests) is a useful example of how the project checks whether tests can fail, and is also approximate by its own admission.

::: judge
**JUDGE MAY ASK**

Question: You have hundreds of tests. Do they prove your concurrent cache is correct?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No, and I would be wary of anyone who said yes. The suite is large, and it includes useful kinds of test: a protocol model checked exhaustively, a mutation detector that checks the model can fail, concurrent stress schedules over real threads, and property tests with random operation sequences. Those give real confidence in specific invariants. But they run in a debug build on one machine, stress tests sample interleavings rather than enumerating them, and a reported race, G4A-01 (UNDER-INVESTIGATION), has not been settled by the suite. I would say: tested for the invariants named, not proven, and not independently reviewed.

**If the judge asks further:**

"What would you add?" A deterministic concurrency checker such as loom for the shard protocol, a G4A-01 regression test that fails without the fix, and release-profile runs. None of those is in the repository today, and I would describe them as future work, not as done.

**Evidence to know:** `tests/integration/concurrency_verification_tests.rs` (`test_exhaustive_protocol_model_invariants`, `test_protocol_model_mutation_detector`, `test_real_cache_concurrent_stress_schedule`, proptest); G4A-01; `docs/research/claims_ledger.md`. Status: TESTED (named invariants); not INDEPENDENTLY VERIFIED. (`loom` is not a dependency; this is a suggestion, our reasoning.)
:::

## 3.13 Supply-chain observations (verified list)

All from `Cargo.lock` and the G4C inventory unless marked.

- 288 packages, lockfile v4, **no git or alternate-registry sources** (G4C, reproduced by that review; I confirmed the package count with `grep -c '^name'`). A build with `--locked` (used by `packaging/build-release.*`) uses exactly these.
- 11 duplicated crate names, including `rand` (0.8.8 and 0.10.3), `tower-http` (0.5.2 and 0.6.11), `webpki-roots` (0.26.11 and 1.0.11).
- Two crypto stacks (`ring`, `aws-lc-rs`/`aws-lc-sys`); OpenSSL crates only through dev-only `reqwest`.
- `tokio` `full` and `tower-http` `fs`/axum `ws` widen features beyond use.
- Licences are permissive (MIT / Apache-2.0 / ISC); `webpki-roots` is a CDLA-Permissive data licence (G4C reading of registry manifests).
- **The vulnerability and yank status of every locked version is NOT determined.** `cargo audit` / `cargo deny` has not been run (G4C; STATUS M7: "no advisory scan"). A claim that dependencies are "free of known vulnerabilities" must not be made. `docs/PHASE_2_WORK_REPORT.md` itself says clean linting does not imply dependency safety.
- `Cargo.toml` and `Cargo.lock` are modified relative to HEAD in the working tree, and `Cargo.toml` is one of four files whose hash differs from the M6 freeze record (expected; G4C).
- npm side: `react`, `react-dom` and unused `lucide-react` at runtime; three dev packages; `npm audit` was not run (UNVERIFIED).

::: judge
**JUDGE MAY ASK**

Question: Are your dependencies safe? Have you scanned them?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The dependency set is small and permissively licensed, the lockfile has no git or alternate-registry sources, and release builds use `--locked`. But I have not run a vulnerability scan: neither `cargo audit` nor `cargo deny` has been executed, and the project's own status page lists that as a gap. So I cannot say the locked versions are free of known advisories. I can also tell you what I would tidy: some declared crates are unused, and two crypto libraries are linked.

**If the judge asks further:**

"Why two crypto libraries?" I use `ring` directly for SHA-256, and rustls defaults to `aws-lc-rs`. Consolidating on one is an owner decision that I have not made.

**Evidence to know:** `Cargo.lock`; `docs/release/g4/cycle1_lab_intelligence_deps_review.md` (dependency inventory, AI review); `docs/STATUS.md` (M7 row); `docs/PHASE_2_WORK_REPORT.md` scope note. Status: inventory VERIFIED by reading; advisory status UNVERIFIED.
:::

## Key ideas to remember

::: concept
- Separate **recorded decisions** (rustls instead of OpenSSL; rejected UI libraries; detached fetch task in ADR-002) from **reasoning after the fact** (Rust, Tokio, Axum, sharded mutexes, React). Never invent a comparison that the repository does not contain.
- The concurrency rule is: short synchronous `std::sync::Mutex` sections, no guard across `.await`, slow work in a separate task, results passed by `oneshot`.
- Eight power-of-two shards give lower contention only for spread-out keys; a hot key serialises on one shard. The epoch `RwLock` snapshot keeps reads off the writer mutex, except while a snapshot is rebuilt.
- Rust prevents memory errors and data races, not logic races (G4A-01 is UNDER-INVESTIGATION).
- No Dockerfile exists; the system is one binary plus scripts; no dependency advisory scan has been run.
:::

## What you should now understand

- Why a std Mutex guard held across an await is a hazard and how the code avoids it (Figure 3.2).
- How `hash & mask` selects a shard and what sharding does and does not buy.
- Why the epoch snapshot exists, what it costs on invalidation, and the lock order.
- That the latency histogram is lock-free atomics (the maths is in Chapter 5, section 5.11).
- Which technology claims are recorded and which are your own reasoning, and the exact discrepancies: ingress TLS 1.3 label, HS256-only versus the spec, "no OpenSSL C" versus `aws-lc-sys`, unused declared dependencies, histogram comments.

## Knowledge check

1. Name two technology choices with a recorded rationale in the repository and two without one.
2. Why does the cache use `std::sync::Mutex` instead of `tokio::sync::Mutex`, and what rule must then hold?
3. A key hashes to `...0x6C`. With 8 shards, which shard? (Use the low three bits.)
4. Why does the `RwLock<Arc<HashMap>>` snapshot exist, and when do readers still wait?
5. What does `PartitionedCache::new_with_diagnostics` assert about `num_shards`, and why?
6. List three declared dependencies with no use found in `src`.
7. What is the difference between the ingress TLS configuration and the cluster TLS configuration with respect to protocol versions?
8. Why is the origin client hand-written, and what is the cost?
9. Does the existence of 72 test targets and a passing baseline prove the cache is race-free? Give one concrete counter-example from the repository.

::: answers
1. Recorded: rustls/ring/rcgen to avoid OpenSSL build problems (Phase 1 spec section 8); rejected UI chart/graph/headless libraries (toolkit proposal); detached `tokio::spawn` fetch (ADR-002 CT-2). Not recorded: Rust, Axum, React, the 8-shard layout.
2. Critical sections are short, in-memory, and have no I/O, so the cheaper std Mutex is enough. The rule: never hold the guard across an `.await` (it would block a worker thread and, because `MutexGuard` is `!Send`, usually does not compile in a spawned task).
3. `0x6C` = `0110 1100`; low three bits `100` = 4, so shard 4.
4. Every request needs its tenant's epoch; reading from a read-only snapshot avoids queueing on the writer mutex. Readers wait while an invalidation holds the snapshot write lock, which includes the O(n) clone of up to 10,000 entries (G4A-12).
5. That `num_shards.is_power_of_two()`, so `hash & (num_shards - 1)` equals hash modulo `num_shards` without a division.
6. `tracing`, `tracing-subscriber`, `criterion` (dev), plus unused features `ws`, `fs`, and UI `lucide-react`.
7. Cluster: `builder_with_protocol_versions(&[&TLS13])` plus mandatory client certificates, with a TLS 1.2 rejection test. Ingress: `RustlsConfig::from_pem_file` with no version restriction in this code; "TLS 1.3 only" is UNVERIFIED.
8. To connect to a pre-validated pinned address, send a fixed header set, cap size and time out each step. Cost: the project owns an HTTP response parser and pays a fresh connection per miss (`Connection: close`).
9. No. Tests are debug-profile, sample interleavings, and G4A-01 (a reported case where an old body can survive an acknowledged invalidation) is UNDER-INVESTIGATION despite the suite.
:::

## Codebase exploration tasks

1. **Find the lock scope.** Start at `src/api/server.rs`, search for `FetchRegistrationResult::Initiate`. Find the block that pushes `my_tx` into `registration.shared_op.waiters`, and the first `.await` after it. What you should learn: the guard is dropped before any await, and the slow work is in a spawned task.
2. **Trace a key to a shard.** Start at `src/core/cache.rs::get_shard_index` and `PartitionedCache::new_with_diagnostics`. Compute the shard of a key by hand using any 64-bit value; then open `benches/b1_hit_path.rs` to see how the bench isolates `get_shard_index` and `get_effective_epochs`. You should learn the cost components of a hit and what the bench would measure.
3. **Audit one dependency.** Pick `tracing` or `criterion`: run a search over `src`, `tests`, `benches`, and check `Cargo.lock` for who depends on it. You should learn how to distinguish "declared", "compiled" and "used".

## Challenge questions

1. A new requirement says the epoch snapshot must never pause readers more than 100 microseconds, even with 10,000 tenants. Propose a design change and say how you would test it.
   *Model answer:* Build the new `Arc<HashMap>` (or a persistent/copy-on-write map) before taking the write lock, and swap only the pointer under the write lock, so the pause is a pointer store. Better: avoid cloning altogether with per-tenant atomics or a sharded map, which also removes the O(n) cost (the fix suggested in G4A-12). Combine the tenant map and global epoch in one `Arc<EpochView>` so one atomic swap publishes both, fixing G4A-17. Test: a concurrent-readers benchmark with 10,000 tenants and a stream of invalidations, recording reader latency percentiles in a release build, with seeds and repetitions (CLAUDE.md section 4), plus a regression test that the epoch pair is never observed mixed.
2. Hot-key skew puts 60% of traffic on one shard. Compare three responses and say which you would try first and why.
   *Model answer:* (a) More shards: does not help, one key still maps to one shard. (b) Replicating hot entries across shards or a small read-through front cache of `Arc` resources: removes lock contention for reads of that key but complicates invalidation and the fencing rules. (c) Reducing critical-section length (no body clone under the lock, `Arc` the body, which the code already does for resources): cheap and safe. First try (c) and measure; only then consider (b) because it touches fencing, which CLAUDE.md says needs an independent review pass and a failing-without-the-change test. State that no measured evidence yet shows contention is the bottleneck.
