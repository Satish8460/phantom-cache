# Chapter 2: System Architecture

> Reading rules for this chapter. Every capability claim carries a status label (IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, UNVERIFIED). "TESTED" means a named test exercises the behaviour, and the text says what the test does not prove. Nothing here is INDEPENDENTLY VERIFIED; every review so far is an AI second look. Code was read in the working tree on 2026-10-30 (branch `feat/publish-phantomcache-core`; at the time of writing HEAD is `df68eaa`; most work is uncommitted). Where an older document disagrees with the code, the code wins and the disagreement is listed in section 2.11.

## What you will learn

- What processes and binaries exist, which ports they use, and the component map of the gateway with each part's responsibility.
- The real HTTP routes, who may call them, and what each one does.
- Step by step what happens on a cache hit, on a miss with coalescing, when the origin fails, and on a DELETE in a cluster.
- How the nodes talk to each other, what the cluster does and does not do, and where Phantom Lab and the UI sit.
- The real configuration keys with defaults and wiring status, and what "deployment" honestly means today (plus a one-page map of the auth boundaries).

## 2.1 Two viewpoints

### 2.1.1 The external user's view

Simple explanation. To a user, PhantomCache is a web address you put in front of your own data service. You ask the gateway for a page. If the gateway already holds a fresh copy, it answers at once. If not, it asks your data service (the "origin"), keeps a copy for next time, and answers. You can also tell the gateway to forget a page.

Terms. The **gateway** is the PhantomCache process. The **origin** is your system of record, set by `ORIGIN_URL`. A **hit** is an answer from memory. A **miss** needs the origin. The response header `X-Cache-Status` tells the caller which happened.

What the user sees (all IMPLEMENTED; routes in section 2.3, header values in section 2.3.3):

- `GET /api/v1/cache/<path>?<query>` returns the origin's body with `X-Cache-Status`, `Age` and, when the origin sent one, `ETag`.
- `DELETE /api/v1/cache/<path>?<query>` removes that one key and returns `204`.
- Failures carry an `X-Cache-Error` header with a short code.
- A browser UI (React, separate dev server) shows telemetry and a "Phantom Lab" for experiments. It is not needed to use the cache.

The cache stores only what a `GET` on the origin returned. There is no write API. To change cached data, change the origin and then `DELETE` the key, or wait for the entry to expire.

### 2.1.2 The internal engineer's view

The engineer sees one Rust binary, `phantom-cache-demo`, built from `src/main.rs`, plus a second binary, `phantom-lab`, built from `src/bin/phantom-lab.rs` (`Cargo.toml`, the single `[[bin]]` entry, plus the default binary from `src/main.rs`). The gateway is a Tokio multi-thread runtime (`tokio::runtime::Builder::new_multi_thread` in `main::real_main`) running an Axum router. State lives in one `PartitionedCache` (8 shards, each a `std::sync::Mutex`), plus optional cluster machinery in `src/dist/`.

Two facts surprise newcomers.

1. `src/main.rs` does not use the library crate for its own modules. It re-declares the module tree (`pub mod core { pub mod cache; ... }` and so on, lines 1 to 56) so that the binary compiles its own copy of the same files that `src/lib.rs` also registers. Tests for the library and for the binary therefore compile the code twice. (verified in code)
2. The gateway has two configuration paths. `src/config.rs` validates everything and `main` applies some keys directly; other keys are still read from the environment by `src/api/server.rs`, `src/dist/router.rs` and `src/security/cluster.rs`, so `main` exports validated values back into the environment before the runtime starts (`cfg.env_exports()`, `std::env::set_var` in `main::real_main`). Section 2.9 lists which is which.

### 2.1.3 What runs where

| Thing | Process | Default port | Status |
|---|---|---|---|
| Gateway (`phantom-cache-demo`) | one OS process | `127.0.0.1:8085` (`PHANTOM_BIND_ADDR`) | IMPLEMENTED |
| Cluster listener (mTLS, binary frames) | same process, second listener | no default; `PHANTOM_CLUSTER_LISTEN_ADDR` | IMPLEMENTED, TESTED (loopback only); labelled EXPERIMENTAL in `main.rs` |
| Origin | your server, or a test origin | from `ORIGIN_URL` | external |
| React UI | `npm run dev` (Vite), a separate process | 3000 (`ui/vite.config.js`) | IMPLEMENTED, development server only |
| `phantom-lab` CLI | separate short-lived process | none | IMPLEMENTED |
| Lab T2 child gateway, scripted origin, load generator | spawned by the Lab, on loopback | a free port picked per run (`lab_live::gateway::free_port`) | IMPLEMENTED |

The gateway does not serve the UI. A search for `ServeDir` or `ServeFile` in `src/` finds nothing, although `tower-http` is built with its `fs` feature. The UI is a Vite dev server on port 3000 that calls the gateway on 8085 (`ui/src/lib/config.js::DEFAULT_GATEWAY`), so the gateway must allow that browser origin (CORS, section 2.10). A built bundle exists in `ui/dist/`, but nothing in `src/` serves it. (verified in code)

## 2.2 The component map

![Figure 2.1: Component map of one gateway process](../assets/ch02_components.svg)

Source: `src/main.rs::run`, `src/api/server.rs::create_app_with_lab`, `::start_server_with_cors`, `src/core/cache.rs::PartitionedCache`, `src/dist/router.rs::RendezvousRouter`, `src/dist/node.rs::VirtualNode`.

Read the figure top to bottom. A request enters the Axum router, which applies the CORS layer and a 512 KiB body limit (`DefaultBodyLimit::max(512 * 1024)`). A cache handler authenticates the caller if JWT is configured, builds a `CacheKey`, and asks the `PartitionedCache`. Only on a miss does the handler call the origin fetcher. Dashed boxes exist only when you turn them on.

### 2.2.1 Responsibilities

| Component | Responsibility | Where | Status |
|---|---|---|---|
| Config loader | Parse CLI, file and environment; validate; fail closed with exit code 2; print a redacted effective configuration | `src/config.rs::resolve`, `::preflight`, `KEYS` | IMPLEMENTED; TESTED by `tests/integration/m7_config_tests.rs` (checks `docs/user/configuration.md` against the key table; does not prove runtime wiring) |
| Startup security gate | Refuse a network bind without JWT; refuse network plaintext without explicit override; refuse insecure upstream without an allowlist | `src/security/mod.rs::validate_startup_security`, `::validate_upstream_startup_security` | IMPLEMENTED |
| Router and layers | Route table, CORS, body limit | `server.rs::create_app_with_lab` | IMPLEMENTED |
| Cache handlers | Authenticate, derive key, hit/miss/stale/bypass logic, map errors to HTTP | `server.rs::get_cache_handler`, `::delete_cache_handler`, `::map_origin_error_to_response` | IMPLEMENTED; TESTED (section 2.4 to 2.7) |
| `PartitionedCache` | Sharded in-memory store, single-flight registry (`in_flight`), generations and epochs, byte budget, eviction, sweeper | `src/core/cache.rs` | IMPLEMENTED; deep treatment in Chapter 4 |
| Freshness engine | Decide cacheability and lifetime from `Cache-Control`, `Expires`, `ETag` and so on | `src/core/freshness.rs::evaluate_response_freshness` | IMPLEMENTED |
| Origin fetcher | SSRF policy, DNS pinning, HTTP/1.1 client with timeouts and size caps | `server.rs::fetch_origin_raw`, `src/security/upstream.rs` | IMPLEMENTED |
| Intelligence executor | Optional origin concurrency permit; default mode `off` | `src/intelligence/executor.rs::acquire_origin_permit` | IMPLEMENTED; cap enforced only in `canary`/`active` with a configured cap (`origin_cap_enforcing`); detail in Chapter 5 |
| Telemetry | Latency and counters for the observation endpoints | `src/intel/telemetry.rs`, `src/intel/diagnostics.rs` | IMPLEMENTED |
| Background tasks | Sweeper (every `PHANTOM_SWEEPER_INTERVAL_MS`, default 100 ms), metrics sampler (1 s), SSE feed (100 ms per connected client) | `cache.rs::spawn_sweeper_task`, `server.rs::spawn_metrics_sampler`, `::sse_handler` | IMPLEMENTED |
| Cluster router | Choose a primary for a key, send invalidation frames, fan out, repair, reconcile | `src/dist/router.rs::RendezvousRouter` | IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED |
| Cluster node and listener | Accept mTLS connections, validate peer identity, handle frames | `src/dist/node.rs::VirtualNode::start_tcp_service`, `::process_binary_cache_stream` | IMPLEMENTED |
| Frame codec | Binary wire format (magic, version, 28-byte header, types) | `src/dist/frame.rs` | IMPLEMENTED; TESTED by the unit tests in the file and `tests/integration/security_phase3b_framing_tests.rs` |
| Lab API | HTTP front door for experiments | `src/api/lab_api.rs::router` | IMPLEMENTED, off by default |
| Standalone demo nodes and snapshot | Three `VirtualNode`s and a placeholder snapshot for the dashboard | `main.rs::run` (`nodes`, `initial_snap`) | PROTOTYPE ONLY: synthetic; the code says so in a comment |

Objects created in `main::run` but never used in serving: `_traffic`, `_network`, `_db` and `_coalescer` are built with underscore names and dropped. The serving path coalesces through `PartitionedCache::begin_fetch_atomic` and `SharedOp`, not through `core::coalesce::Coalescer`. (verified in code)

::: concept
**Key concept: what a "node" is.** In the standalone gateway there is one real cache and three placeholder `VirtualNode` objects with their own small caches (4 shards each) used only for dashboard demos. In the mTLS cluster configuration the local member is a `VirtualNode` built with `new_with_cache`, so it shares the gateway's serving cache. Every other member is an address-only descriptor with no cache of its own in this process. "Node" in the code therefore means "a member identity plus a connection", not "a separate machine you can read from".
:::

::: judge
**JUDGE MAY ASK**

Question: Explain the architecture of your system.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

PhantomCache is a read-through caching gateway written in Rust. A client sends an HTTP GET to the gateway. The gateway looks the key up in an in-memory cache split into eight lock-protected shards. On a hit it answers from memory. On a miss, one request goes to the origin and any other requests for the same key wait for that single answer; the result is stored if it is still valid. A DELETE removes the key locally first, then, in cluster mode, tells the node chosen by rendezvous hashing and waits for its acknowledgement, and then tells the other members in the background. Around that core there are optional parts: JWT authentication, an experimental mTLS cluster, an observation API, and a separate experiment tool called Phantom Lab. Reads are always served from the local gateway's own cache.

**If the judge asks further:**

"Is it a distributed cache?" Honestly: it is N independent caches with an invalidation bus. Each gateway caches locally and they coordinate invalidation. It does not route reads to a key owner, so it is not a partitioned distributed cache in the Redis Cluster or groupcache sense. The cluster work is about keeping invalidation reliable across gateways. "Where does the intelligence sit?" Off the request path by default; section 2.2 and Chapter 5.

**Evidence to know:** `src/main.rs::run`, `src/api/server.rs::create_app_with_lab`, `get_cache_handler`, `src/core/cache.rs::PartitionedCache`; `docs/user/limitations.md`; status IMPLEMENTED, TESTED for the single-gateway path; for the cluster IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED.
:::

## 2.3 The HTTP surface

### 2.3.1 Real routes

These are the routes in `create_app_with_lab` and `lab_api::router`, plus the merged `health::router`. "JWT on" means `PHANTOM_JWT_SECRET` is set, which creates a `SecurityConfig` with `require_auth_for_control: true` in both constructors (`src/security/mod.rs`). Roles are defined in `src/security/jwt.rs::Role`. Chapter 6 owns the security analysis; this table only marks the boundary.

| Method and path | Role needed when JWT is on | Purpose | Status |
|---|---|---|---|
| `GET /health` | none | plain text `OK` | IMPLEMENTED |
| `GET /health/live` | none | `200 {"status":"alive"}` | IMPLEMENTED; TESTED (`m7_health_tests::m7_live_is_always_200_and_ready_tracks_state`) |
| `GET /health/ready` | none | `200` only when config is valid, the listener answered a TCP probe, and the process is not draining; else `503` with the failing checks | IMPLEMENTED; TESTED (same test) |
| `GET /api/v1/cache/*key`, `GET /api/v1/cache` | `cache:read`, and a non-empty `tenant_id` claim (not `__default__`) | read-through cache lookup | IMPLEMENTED; TESTED (`m1_http_tests`) |
| `DELETE /api/v1/cache/*key`, `DELETE /api/v1/cache` | `cache:invalidate`, and a tenant claim | remove one exact key (path plus raw query string) | IMPLEMENTED; TESTED |
| `GET /api/events` | `telemetry:read` | server-sent events every 100 ms; `?schema=2` is current, schema 1 is deprecated (`Deprecation` and `Sunset: Wed, 31 Dec 2026` headers) | IMPLEMENTED |
| `GET /api/v1/metrics` | `telemetry:read` | one JSON snapshot, schema 2 by default | IMPLEMENTED |
| `GET /api/v1/diagnostics/summary`, `/api/v1/diagnostics/events` | `telemetry:read` | counters snapshot; last 100 diagnostic events | IMPLEMENTED |
| `GET /api/v1/intelligence/status`, `/api/v1/intelligence/audit` | `telemetry:read` | state and audit log of the intelligence executor | IMPLEMENTED |
| `POST /api/control` | `scenario:operate` | body `{"event": ...}`; `FLASH_CROWD` sets a workload label, `NODE_FAILURE` sets node 2 to `Failed`, anything else resets labels and sets every node `Active` | IMPLEMENTED; it flips in-memory flags only |
| `GET /api/v1/lab/capabilities`, `/evidence`, `/evidence/:id`, `/runs`, `/runs/:id`, `/runs/:id/events` (SSE), `/runs/:id/artifacts`, `/runs/:id/artifacts/:name` | `telemetry:read` | read Lab state and retained evidence | IMPLEMENTED; exist only if `PHANTOM_LAB_ENABLED=true` |
| `POST /api/v1/lab/runs`, `POST /api/v1/lab/runs/:id/cancel` | `scenario:operate` | start and cancel experiments | IMPLEMENTED; same condition |

Without JWT, the cache routes and the observation routes are open on the bind address, and `x-tenant-id` selects the tenant. The startup gate makes that safe only on loopback: a non-loopback bind without JWT is refused (`StartupSecurityError::NetworkWithoutAuth`). The Lab API without JWT additionally requires a loopback bind (`lab_api::check_lab_bind`) and checks the `Host` and `Origin` headers (`host_origin_guard`, a DNS-rebinding defence). (verified in code)

What does not exist: there is no `PUT`, `POST` or `PATCH` on cache keys, no HTTP route for tenant-wide or whole-cache invalidation (the handler always uses `InvalidationScope::ExactKey`; the other scopes exist only in the cluster frame format and in the epoch methods used by peers and anti-entropy), and no `/metrics` Prometheus endpoint. (verified in code: route list above is complete for `create_app_with_lab`)

### 2.3.2 Cache key and path mapping

The URL path after `/api/v1/cache` becomes the origin path. `GET /api/v1/cache/item1?x=1` asks the origin for `/item1?x=1`.

The cache key is `CacheKey { tenant_id, path, query }` (`src/core/cache.rs::CacheKey`). With no tenant the tenant is the literal `__default__`. The canonical string is `tenant:path` or `tenant:path:?query` (`to_canonical_string`). `CacheKey::validate` rejects tenant ids that are not 1 to 128 bytes of `[A-Za-z0-9._-]`, paths over 2048 bytes, queries over 4096, control characters and spaces, `.` and `..` segments, `:?` inside a path, and canonical strings over 4096 bytes. The query is used exactly as sent: `?a=1&b=2` and `?b=2&a=1` are two keys. (verified in code)

The path of `ORIGIN_URL` is not prepended: `fetch_origin_raw` takes only host and port and builds the request target from the key via `encode_origin_target`, so `http://origin.example/v2` is still asked for `/item1`, not `/v2/item1`. (verified in code; no test read)

### 2.3.3 Response vocabulary

| `X-Cache-Status` | Meaning | Set in |
|---|---|---|
| `HIT` | fresh entry served from memory | `get_cache_handler`, `FetchRegistrationResult::Hit` branch |
| `MISS` | this request was the leader that fetched the origin | `Initiate` branch |
| `COALESCED` | this request waited for another request's fetch | `Wait` branch |
| `STALE` | an expired entry was served under a stale directive, or the leader's fetch failed and stale-if-error applied | stale pre-check, and `is_stale` in the `Wait` and `Initiate` branches |
| `BYPASS` | client sent `x-cache-bypass: true`; origin fetched, nothing stored | bypass branch |
| `BYPASS-AUTH` | a credential was presented and the gateway forwards credentials upstream, so reads skip the cache | bypass branch |

`X-Cache-Error` values come from `map_origin_error_to_response` (for example `Timeout`, `NetworkError`, `ResponseTooLarge`, `WaiterLimitExceeded`, `CapacityExceeded`, `InvalidPath`, `SsrfBlocked`). (verified in code)

## 2.4 Request flow 1: a cache hit

### 2.4.1 Simple explanation

The gateway checks its notebook. The page is in the notebook and still fresh, so it reads it out. The origin is never bothered.

### 2.4.2 Terms and how it works

A **shard** is one of eight independent sub-caches. The key is hashed (`PartitionedCache::get_shard_index` uses `DefaultHasher` and masks with `shard_mask`, which works because the shard count must be a power of two). Only the shard that owns the key is locked, so requests for different shards do not wait for each other. An **epoch** is a counter used to invalidate many entries at once; an entry is valid only if its recorded epochs equal the current ones.

![Figure 2.2: Request sequence for a cache hit](../assets/ch02_seq_hit.svg)

Source: `server.rs::get_cache_handler`, `cache.rs::check_stale_serving`, `::begin_fetch_atomic`.

Steps, as the code does them:

1. **Authenticate first** (`authenticate_request`, role `cache:read`, then `require_tenant_claim`). In secure mode the tenant comes only from the verified token; a caller-supplied `x-tenant-id` is ignored. Without JWT the header is used, and an invalid header value is a `400`, not a silent fallback to the default tenant.
2. **Build and validate the key**, increment `requests_total`.
3. **Stale pre-check.** `check_stale_serving` locks the shard and returns `Ineligible` for a fresh entry, so a fresh entry goes on to step 4. (Eligible only for an expired entry inside its stale window; section 2.6.)
4. **`begin_fetch_atomic`** locks the shard a second time. If the entry's tenant and global epochs equal the current ones and `created_at_ms + ttl_ms > now_ms`, it marks the entry visited (used by the eviction policy), counts a hit and returns `Hit(entry)`.
5. The handler answers `200` with `X-Cache-Status: HIT`, `Age`, `ETag`, answers `304` instead if the client's `If-None-Match` or `If-Modified-Since` matches, and records telemetry.

::: example
**Worked example.** Key `__default__:/item1`. Default budget `PHANTOM_MAX_CACHE_BYTES` is 268,435,456 bytes (256 MiB). `PartitionedCache::set_byte_budget` divides it by the shard count: 256 MiB / 8 = 32 MiB per shard. A single cached item larger than its shard's budget is rejected as oversize at store time (`byte_budget_rejections`, `oversize_rejections`), even though the whole cache has room. The default cap of 10,000 entries per shard (`max_entries_per_shard`) gives at most 80,000 entries in total.
:::

Two shard-lock acquisitions per hit (steps 3 and 4) is a fact about the implementation, not a measurement; this chapter makes no latency claim (Chapter 6 lists the real measurements).

### 2.4.3 What can go wrong

- A hit is only as fresh as the entry's lifetime. Staleness is bounded by the freshness rules in section 2.6, not by any cross-node guarantee.
- Sharding by hash means a very hot key concentrates its load on one shard lock. Detail and mitigation in Chapter 4.
- `begin_fetch_atomic` can return `Rejected(CapacityExceeded)` when a shard is full and nothing is evictable; the client sees `503` with `Retry-After: 1`.

::: judge
**JUDGE MAY ASK**

Question: Walk me through what happens to one request, from the network to the response.

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The request arrives at the Axum router, which applies CORS and a body limit. The cache handler first checks the caller's token if authentication is on, and takes the tenant from the token. It builds a cache key from tenant, path and query and validates it, rejecting anything odd such as control characters or dot segments. It then asks the sharded cache. The key's hash picks one of eight shards and only that shard's lock is taken. If a fresh entry exists, the handler returns it with `X-Cache-Status: HIT`. If not, it goes down the miss path: one leader fetches the origin, others wait, the result is stored if it is still valid, and everyone gets the answer.

**If the judge asks further:**

"Where is the lock held across a network call?" Never: the shard mutex is held only while reading or updating the map. The origin call happens in a separate spawned task with no shard lock held. "What if the client has a token but wrong tenant?" Cross-tenant isolation is by key namespace taken from the token; see Chapter 6 for the analysis and the open findings.

**Evidence to know:** `get_cache_handler`, `PartitionedCache::begin_fetch_atomic`, `CacheKey::validate`; `m1_http_tests::test_m1_t01_cold_cache_miss` and `test_m1_t02_repeat_cache_hit` (TESTED on loopback, debug profile, one gateway; they prove the MISS then HIT behaviour and one origin call, not performance).
:::

## 2.5 Request flow 2: a miss, coalescing and the origin fetch

### 2.5.1 Simple explanation

A hundred people ask for the same missing page at once. Instead of a hundred trips to the origin, one person is sent, and the other ninety-nine wait at the door and share what that one brings back.

### 2.5.2 Terms

**Single-flight** (also called request coalescing): at most one in-progress fetch per key; later arrivals join it. A **leader** starts the fetch; **waiters** join. A **stampede** (or thundering herd) is many simultaneous misses on one key, which would hit the origin together. **Generation** (`op_gen`) is a per-key counter that increases on delete; it lets a finishing fetch discover that the key was deleted while it ran.

![Figure 2.3: Miss with coalescing and origin fetch](../assets/ch02_seq_miss.svg)

Source: `server.rs::get_cache_handler` (`Initiate` and `Wait` branches), `cache.rs::begin_fetch_atomic`, `::finalize_fetch`, `src/core/coalesce.rs` (`FetchRegistration`, `OriginFetchGuard`, `WaiterBatch`).

### 2.5.3 How it works

Inside `begin_fetch_atomic`, under the shard lock, three outcomes are possible after a miss check.

- If an operation for the key is already in `shard.in_flight`, the caller is added to that operation's waiter list (limit `max_waiters_per_key`, default 1,000, else `Rejected(WaiterLimitExceeded)` and `503`) and gets `Wait(rx)`.
- Otherwise the shard may evict (at most 32 victims per attempt) to make room. If it still has no room, `Rejected(CapacityExceeded)`.
- Otherwise it creates the key state if absent, records `op_id` and `op_gen = state.generation`, inserts a `SharedOp` into `in_flight`, counts `cache_misses` and returns `Initiate(registration)`.

The leader registers itself as a waiter on the same shared operation, then runs the origin call in a spawned task (`tokio::spawn` inside the `Initiate` branch). Because the work is in a separate task, a client that disconnects does not cancel the fetch for the others. That is a deliberate property. At HTTP level `m1_http_tests::test_m1_t14_initiator_disconnect_resilience` has a client time out after 50 ms while the origin takes 300 ms, then asserts that later clients get `200` and then a `HIT`. At cache level `serving_contract_tests::test_ct2_initiator_cancellation_detached_fetch` simulates the detached completion. Neither proves behaviour under production load.

The task does this, in order:

1. Takes an origin permit if an intelligence executor is installed (`acquire_origin_permit`). Only the leader takes one; waiters never reach this point. With the executor in its default mode `off` the permit only counts in-flight requests and never refuses.
2. Reads the stored validators of any resident (expired) entry, so the request can be conditional (`stored_validators`).
3. Calls `fetch_origin_raw`: validates the origin URL against the upstream policy, resolves and pins the IP address (`resolve_and_pin_origin`, with a pre-connect filter against loopback and private ranges unless allowed), connects (5 s timeout), does TLS if `https` (5 s), writes a `GET ... HTTP/1.1` request with `Connection: close`, and reads the whole response under one 5 s timeout, capped at 2 MiB of body (`MAX_BODY_BYTES`) and 64 KiB of headers.
4. Evaluates freshness (`evaluate_response_freshness`) and builds a `CachedResource`.
5. Calls `guard.complete(outcome)`, which calls `finalize_fetch`. **The fence:** the result is stored only if `state.generation == op_gen` and the entry's tenant and global epochs still equal the current ones. Otherwise the result is delivered to the waiters but not stored, and `stale_writes_prevented` increases. This is what stops a slow fetch from resurrecting data that a DELETE removed meanwhile.
6. `WaiterBatch::deliver` sends the result to every waiter. The leader sees `MISS`; the others see `COALESCED`.

What is forwarded to the origin is deliberately narrow: a valid client `x-request-id`; `x-tenant-id` from the cache key when the tenant is not `__default__`; `Authorization` only when `PHANTOM_FORWARD_UPSTREAM_AUTH` is set; the gateway's own `If-None-Match` and `If-Modified-Since`; a fixed `Accept: application/json`. The client's conditional and range headers are never forwarded, because the cached copy is shared. (verified in code, `fetch_origin_raw` and `get_cache_handler`)

The origin connection uses `Connection: close`: every fetch opens a new TCP (and TLS) connection, with no pool to the origin. This is a cost per miss that a judge may probe. (verified in code)

::: example
**Worked example: 100 simultaneous requests.** `m1_http_tests::test_m1_t13_barrier_stampede_100_clients` releases 100 clients at once against a mock origin that delays 150 ms, and asserts exactly 1 `MISS`, 99 `COALESCED` and one origin call. Without coalescing the origin would see 100 requests; with it, one. TESTED on loopback, debug profile, one key, one gateway process. It proves collapsing on one key in one process. It does not prove behaviour across gateways (each gateway coalesces independently, so N gateways can send N concurrent fetches for the same key), nor performance under load. Chapter 4, section 4.4 uses a different 10-client test (`coalesce_tradeoffs_tests::test_negative_effect_head_of_line_blocking`) to show the cost to waiters.
:::

### 2.5.4 Trade-offs and what can go wrong

- Waiters share fate with the leader. If the leader's fetch is slow, all waiters wait up to the origin timeout (5 s read timeout).
- A request that arrives after a DELETE does not join the pre-DELETE fetch (`delete_impl` removes the `in_flight` entry), tested by `m1_http_tests::test_m1_t09_get_after_delete_does_not_join_old_fetch`. A request that arrives after a tenant or global epoch change can still join an older in-flight fetch; the code documents this as limitation "L3" in `begin_fetch_atomic` and states the result is kept out of the cache by the epoch check. (verified in code)
- Why this design and not per-key locks held across the origin call, or a separate coalescer object? No recorded decision explains the switch from the unused `core::coalesce::Coalescer` to the shard-integrated `SharedOp`. Our reasoning, not a recorded decision: holding a shard mutex across `.await` is forbidden by the project rules (`CLAUDE.md` section 5), which pushes the design to an in-shard registry plus a detached task.

::: judge
**JUDGE MAY ASK**

Question: What happens on a cache miss? And what if a thousand clients ask for the same missing key?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

On a miss, the shard registers an in-flight operation for that key and the first request becomes the leader. It fetches the origin in its own task, with timeouts and size limits. Every other request for that key during the fetch joins the same operation and waits; the origin sees one request. When the answer arrives the gateway decides whether it may be cached, checks that the key was not deleted or invalidated during the fetch, stores it if all is well, and releases all waiters with the same body. There is a cap of 1,000 waiters per key; beyond that, extra requests are refused with a 503 so memory stays bounded. We tested the 100-client case on loopback.

**If the judge asks further:**

"Is that safe against a DELETE during the fetch?" Yes by design: the fetch records a generation number when it starts, and the store step compares it with the current one; DELETE increments it. "Across gateways?" No: coalescing is per gateway, so three gateways can fetch the same key three times at once. We do not claim cross-gateway coalescing.

**Evidence to know:** `PartitionedCache::begin_fetch_atomic`, `finalize_fetch`; `m1_http_tests::test_m1_t13_*`, `test_m1_t09_*`; `serving_contract_tests` (`test_ct1_single_flight_coalescing`, `test_ct3_waiter_limit_enforcement`, `test_ct7_fencing_under_invalidation`, cache-level); TESTED, loopback and debug build. Chapter 4 for the fencing proof and the open G4A-01 history.
:::

## 2.6 Freshness, stale serving and origin failure

### 2.6.1 What decides how long an entry lives

`evaluate_response_freshness` (`src/core/freshness.rs`) applies RFC 9111-style rules (cacheability, then lifetime from `s-maxage`, `max-age`, `Expires` minus `Date`, and finally a default of 60,000 ms when the origin gave nothing). The full rule list is in Chapter 4, section 4.8. (verified in code)

::: caution
**The default TTL setting is not wired.** `PHANTOM_DEFAULT_TTL_MS` is documented with default 60,000 and validated, but `src/config.rs` marks it `Wiring::NotWired`, and `src/api/server.rs` passes the literal `60_000` to `evaluate_response_freshness` at two call sites (the miss path and the background revalidation path). Setting the key changes nothing; the startup report prints "not honored by runtime" and a warning if you change it from the default. The effective fallback lifetime is always 60 s. Documented default and effective setting are the same number only by coincidence. (verified in code; re-checked 2026-10-30)
:::

### 2.6.2 Stale serving, only as far as the origin allows

The code serves expired data in two situations, both only when the origin's own response carried the matching directive, and always bounded by the hard cap `PHANTOM_MAX_STALE_MS` (default 86,400,000 ms, 24 h). Chapter 4, section 4.8 has the full rules.

- **stale-while-revalidate (SWR):** an expired entry inside its SWR window is returned at once with `X-Cache-Status: STALE`, and a background revalidation starts if due (bounded: `max_background_refresh`, default 128, single-flight).
- **stale-if-error:** if the origin fetch fails, a stale copy inside the stale-if-error window (or, if none, the SWR window) is returned instead of an error.

`must-revalidate` forbids any stale serving, and with no directive the window is zero. The gateway does not invent a "serve stale during an outage" behaviour: if your origin does not send `stale-if-error`, an outage produces errors. (verified in code, `PartitionedCache::check_stale_serving`)

### 2.6.3 Origin failure

![Figure 2.4: What the code does when the origin fails](../assets/ch02_origin_failure.svg)

Source: `server.rs::get_cache_handler` (failure branches), `::map_origin_error_to_response`, `cache.rs::check_stale_serving`.

When the leader's fetch fails, the leader and every waiter get the same outcome: either a stale copy (if eligible) or a mapped error. Errors are not stored; the next request starts a fresh fetch. The mapping is: a timeout is `504`; connect, TLS, framing, size and other network failures are `502`; an origin status code is passed through (404 as `404` with `X-Cache-Error: NotFound`); an origin-cap refusal is `503` with `Retry-After: 1`. What the code does **not** have: a circuit breaker, negative caching of errors, a retry loop on the origin call, or a "degraded mode" switch. (verified in code: no such symbols in the handler; absence not proven for the whole tree)

A 304 reply to a conditional request is handled by `PartitionedCache::complete_not_modified`, which refreshes the stored entry's lifetime instead of replacing its body (`tests/integration/revalidation_304_tests.rs` exists; not read in detail for this chapter).

Evidence for the stale-if-error rule is at cache-component level: `stale_serving_tests::test_stale_if_error_on_origin_outage` builds an entry with a 5 s lifetime and a 30 s stale-if-error window, then calls `check_stale_serving` directly and asserts `Ineligible` in normal context, `Eligible` in error context inside the window, and `Ineligible` beyond it, with `stale_cap_violations` equal to 0. It does not drive the HTTP handler against a failing origin. A handler-level test named `test_http_stale_serving_headers_and_status` exists in the same file; its body was not read for this chapter, so end-to-end stale-if-error through HTTP is UNVERIFIED here.

::: judge
**JUDGE MAY ASK**

Question: What happens when the origin is unavailable?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

If the key is cached and fresh, nothing changes: clients are served from memory and never see the outage. If the entry has expired, the answer depends on what your origin told us when it sent the data. If it sent a stale-if-error or stale-while-revalidate window, we serve the expired copy, labelled STALE, for as long as that window allows, never beyond the 24-hour hard cap. If it sent nothing, we do not guess: the client gets an error, a 504 for a timeout or a 502 for a network failure. Errors are never cached, so recovery is immediate when the origin returns. Only one request per key goes to the failing origin at a time, because of coalescing, and an optional concurrency cap can shed load with a 503, but only in canary or active mode.

**If the judge asks further:**

"Is there a circuit breaker?" No. The protections are single-flight, a 5 second timeout, the optional origin concurrency cap, and the waiter limit. "Does it serve stale on every outage?" No, only with the origin's own directive. "Is stale-if-error tested end to end?" The rule is tested at cache level; I would not claim an HTTP-level outage test without checking.

**Evidence to know:** `get_cache_handler`, `check_stale_serving`, `map_origin_error_to_response`; `stale_serving_tests::test_stale_if_error_on_origin_outage` (TESTED at component level); `PHANTOM_MAX_STALE_MS` (`Wiring::Main`); status IMPLEMENTED.
:::

::: judge
**JUDGE MAY ASK**

Question: Your configuration lists a default TTL setting. Does it work?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No, and the product says so itself. The key is validated and printed, but the freshness engine is called with a fixed 60 seconds in two places in the server. The configuration table marks it not wired, and the startup report prints "not honored by runtime" next to it. Entries with explicit `Cache-Control` or `Expires` use the origin's lifetime; only responses with no freshness information fall back to 60 seconds, and that fallback cannot be changed by configuration today.

**If the judge asks further:**

"Why not fix it?" It is an application-code change to a protected area and needs approval and a test; it is on the known-gap list, not hidden.

**Evidence to know:** `src/config.rs::KEYS` entry `PHANTOM_DEFAULT_TTL_MS` (`Wiring::NotWired`); the two `60_000` literals in `get_cache_handler`; `docs/user/limitations.md`.
:::

## 2.7 DELETE, the cluster, and how nodes communicate

### 2.7.1 Simple explanation

Each gateway keeps its own notebook. When a page changes, someone must tell every notebook to cross it out. DELETE does that: the gateway crosses it out locally at once, then phones the member chosen as "responsible" for that key and waits for it to confirm, and then tells everyone else without waiting.

### 2.7.2 Terms

**Rendezvous (highest random weight, HRW) hashing:** for a key, give each node a score derived from the key and the node id; the highest score wins. Every gateway that knows the member list computes the same winner without talking. The winner is called the **primary** here. **Fan-out:** sending the same message to many members. **mTLS:** both sides present certificates. **Anti-entropy:** a background exchange that repairs differences between members. **Repair ledger:** a bounded in-memory list of invalidations a peer is owed.

### 2.7.3 How DELETE works

![Figure 2.5: DELETE with cluster fan-out](../assets/ch02_seq_delete.svg)

Source: `server.rs::delete_cache_handler`, `src/dist/router.rs::invalidate_key_across_cluster`, `::send_invalidation_frame_with_seq`, `::fanout_targets`, `::dispatch_fanout`, `::record_owed_to_all`, `src/dist/node.rs::process_binary_cache_stream`.

1. Authenticate (`cache:invalidate`, tenant claim), build and validate the key.
2. **Purge locally first** (`cache.delete`): the entry is removed, the generation is incremented, and any in-flight fetch for the key is detached so later GETs do not join it.
3. With no router (standalone), that is all: `204`.
4. With a router, `invalidate_key_across_cluster` computes the primary with `route_key`. The first pass skips nodes that are `Failed`, `Offline`, `Suspect` or `Recovering`; only if no healthy node exists does a second pass accept `Suspect` and `Recovering` nodes. It picks the highest `DefaultHasher(key, node_id)` value (the code multiplies by `capacity_weight = 1.0`, a constant, so this is plain rendezvous hashing, not weighted in effect). It sends an `InvalidateRequest` frame to the primary over a pooled mTLS connection, bounded by one 150 ms deadline (`PHANTOM_CLUSTER_TIMEOUT_MS`, default 150) for the whole leg including one retry on a fresh connection.
5. The primary's listener checks the peer's certificate identity against the membership registry and that the frame's claimed originator equals the authenticated peer, deletes the key in its cache and replies with an `InvalidateAck`. The router checks the responder id and membership epoch in the ack (G4B-06 records that the epoch check is vacuous: OPEN, owner-named).
6. On success the router queues the other members: live members go to a per-peer bounded queue (256 entries, one delivery in flight per peer); `Suspect`, `Failed`, `Offline` or `Recovering` members are recorded as owed in the `RepairLedger`. A repair worker re-delivers owed invalidations while this gateway stays alive; an anti-entropy worker runs every 250 ms in cluster configurations. Chapter 5, section 5.5 owns the detail.
7. If the primary fails to answer, it is marked `Suspect` and the first healthy replica from the same ranking acts as primary; the response then carries `X-Cache-Warning: PrimaryFailover` (with the suspect node id in brackets when known). If that fails too, every other member is recorded as owed and the client gets `504` with `X-Cache-Error: PrimaryInvalidationTimeout` (or a `503` or `500` with `PrimaryRateLimited`, `PrimaryInternalError` and similar codes when the failure was a rejection rather than a timeout).

The client sees `204` only after a primary acknowledged. The local purge in step 2 happens in all cases, including `504`.

Facts about the router that matter for honesty:

- The local node is one of the router's members. If it wins the hash, the "primary" frame goes over mTLS to this process's own cluster listener; no shortcut for self was found in `connect_to_node_inner` or `send_invalidation_frame_with_seq` (UNVERIFIED for the connection pool internals, which were not read in full).
- The membership is static: it is read once from `PHANTOM_CLUSTER_MEMBERS_FILE` at startup (`security::load_membership_from_file`). There is no join or leave protocol in the code path read.
- The repair ledger is in memory. An invalidation owed to a peer is lost if the issuing gateway process dies before delivery. The durability fix (milestone M3) exists only in a sandbox patch and is not integrated; the current status is in `docs/research/m3_durability/` and `docs/user/limitations.md`. (Chapter 5 owns this.)
- In the insecure or partial cluster configuration (`PHANTOM_ALLOW_INSECURE_CLUSTER` or a members file without full mTLS material) the router holds the three placeholder nodes with no addresses, so no frame is sent and a DELETE ends in `504`. `main.rs` prints this at startup. (verified in code)

### 2.7.4 How nodes communicate

| Channel | Transport | Messages | Used by | Status |
|---|---|---|---|---|
| Invalidation | TCP, TLS 1.3 stack via rustls, mutual authentication, one pooled connection per peer, binary frames (`MAGIC`, version 1, 28-byte header, payload up to 1 MiB, key up to 4,096 bytes) | `InvalidateRequest` (scopes `ExactKey`, `Tenant`, `FullCache`), `InvalidateAck` (status, responder id, membership epoch) | `DELETE` | IMPLEMENTED; TESTED on loopback (`security_phase4b_router_tests::test_p4b_http_gateway_204_primary_success_vs_504_timeout`: one plaintext loopback node, a router of one member, and a gateway cache that is a separate object from the node's cache; it checks 204 on a primary acknowledgement and 504 on timeout, not multi-gateway purge or mTLS) |
| Heartbeat | same connection | `HeartbeatRequest`, `HeartbeatResponse` | pool keepalive every 5 s (`PoolConfig.heartbeat_interval`): idle connections that fail the probe are discarded | IMPLEMENTED; in `pool.rs::probe_idle_connections` a failed heartbeat discards the connection and does not set a node state (state changes come from failed invalidation sends) |
| Anti-entropy | same | `AntiEntropySyncRequest`, `AntiEntropySyncResponse` (in sync, replay batch, history overwritten) | background pull reconciliation | IMPLEMENTED; effectiveness not established (`docs/research/claims_ledger.md` C6, per `CLAUDE.md`) |
| Peer read | same | `GetRequest`, `GetResponse` | nothing in production | PROTOTYPE ONLY |

The `GetRequest` handler (`node.rs`) does not read the origin. On a miss it fabricates the body `val_node_<id>`, stores it for 60 s in the node's cache and answers `MISS val_node_<id>`. Its only sender, `RendezvousRouter::forward_key_over_tcp`, has no caller in `src/`: a search for `forward_key_over_tcp` and `route_key` finds uses only inside `router.rs` (the DELETE path uses `route_key`). GET handling in `server.rs` never touches `state.router`. (verified in code by search)

Therefore: **GET is served from the local gateway's cache only.** There is no key ownership for reads, no forwarding to the primary, no remote read. A cluster of N gateways has N independent read caches that are kept coherent by invalidation, not by routing.

::: caution
**Consistency is bounded by what the invalidation path delivers.** A comment in `delete_cache_handler` says the local-first purge "guarantees linearizability". That wording is not supported by the system: there is no formal model, no linearizability test, and other gateways can serve an entry until their own invalidation arrives or the entry expires. Do not repeat it. The accurate claims are: this gateway purges before the cluster round trip; a `204` means a primary acknowledged within 150 ms; other members are invalidated asynchronously with bounded retry and repair while this process lives. Chapter 5 states the failure model and what is not guaranteed.
:::

::: judge
**JUDGE MAY ASK**

Question: How do the nodes communicate?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Nodes talk over a custom binary protocol on TCP wrapped in mutual TLS: each node proves its identity with a certificate whose name is checked against a static membership file, including the certificate fingerprint. The main message is an invalidation: when a key is deleted, the gateway sends it to the member chosen by rendezvous hashing, waits up to 150 milliseconds for an acknowledgement, and then tells the other members in the background, with a bounded queue, retries and a repair worker for members that were unreachable. There is also a heartbeat and a reconciliation exchange. Reads do not use the network between nodes: each gateway answers reads from its own memory.

**If the judge asks further:**

"What if a node is down?" It is marked suspect, it is owed the invalidation in an in-memory ledger, and a repair worker retries; if the issuing gateway restarts before delivery the owed list is lost, which is a known limitation. "Is the membership dynamic?" No, static, read at startup. "Why not use Raft or gossip?" There is no recorded decision; our reasoning: the system only needs to propagate invalidations, not agree on data.

**Evidence to know:** `src/dist/frame.rs` (message types), `src/dist/router.rs::invalidate_key_across_cluster`, `src/dist/node.rs::start_tcp_service`; `docs/user/limitations.md`; status IMPLEMENTED, TESTED (loopback, debug build, at most four nodes); labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED (no multi-host evidence).
:::

::: judge
**JUDGE MAY ASK**

Question: Which node serves a read for a key, and does rendezvous hashing decide that?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The gateway that received the request serves it from its own cache. Rendezvous hashing is used today only to pick the node that must acknowledge a DELETE first. The code for forwarding a read to the key's owner exists as a function but nothing calls it, and the peer-side read handler is a stub that fabricates a value, so I do not claim distributed reads.

**If the judge asks further:**

"Then what does the hash buy you?" A deterministic primary for invalidation, which gives a single acknowledging authority per key and a stable failover order. "Are the weights real?" No, the weight factor is a constant 1.0.

**Evidence to know:** `RendezvousRouter::route_key` (`capacity_weight = 1.0`), `forward_key_over_tcp` (no caller), `process_binary_cache_stream` `GetRequest` arm; `docs/research/adr/ADR-003-distributed-architecture.md` for the intended direction (status: agent-recorded, not owner-ratified per `docs/STATUS.md`).
:::

## 2.8 Where Phantom Lab and the UI sit

![Figure 2.6: Phantom Lab and the UI relative to the gateway](../assets/ch02_lab_ui.svg)

Source: `src/main.rs` (Lab block), `src/api/server.rs::create_app_with_lab`, `src/api/lab_api.rs`, `src/lab_live/gateway.rs`, `src/bin/phantom-lab.rs`, `ui/src/lib/config.js`.

Simple explanation. The gateway is the product. The Lab is a test bench that either simulates the gateway or starts real gateways on your own machine and measures them. The UI is a dashboard and a control panel for the Lab. Neither sits between clients and the origin.

How it works.

- **Lab API.** If `PHANTOM_LAB_ENABLED=true`, `main` builds a `LabService` (root `PHANTOM_LAB_DIR`, default `lab-results`) and the router adds `/api/v1/lab/*`. With the flag off those paths are `404` (tests in `tests/integration/lab_api/`, not read in detail). It starts the library code the tests and CLI also use: `lab::runner::execute` for **T1** (deterministic discrete-event simulation, virtual clock, seeded randomness, real cache and policy code, simulated origin) and `lab_live::scenario_b::run_experiment` for **T2** (a real gateway child process on a free loopback port, a scripted raw-TCP origin, an open-loop load generator).
- **Tier labels.** T1 results are labelled SIMULATED, T2 results MEASURED_LOOPBACK. Neither is a multi-host or production measurement (Chapter 7).
- **Process boundary.** T1 runs inside the serving gateway process. T2 spawns separate gateway processes (`PHANTOM_LAB_GATEWAY_EXE`, default the current executable); `main` binds them to the parent's lifetime with a Windows job object (`bind_children_to_process_lifetime`; hand-declared Windows FFI, the only `unsafe` found in `src/` is in `lab_api.rs`). On a non-Windows host this is UNVERIFIED here.
- **CLI.** `phantom-lab` runs the same library code without HTTP and refuses the sealed seed set.
- **UI.** A React app in `ui/`, served by Vite on port 3000 in development. It reads `GET /api/events?schema=2` (each value is meant to be labelled measured, derived, simulated or unavailable; the v1 payload nulls synthetic values unless `PHANTOM_SIMULATION` is on) and calls `/api/v1/lab/*`. Token handling is Chapter 6.

Security relation: the Lab API can start processes and read retained results, so without JWT it is loopback-only and Host- and Origin-checked, and with JWT it needs `telemetry:read` or `scenario:operate`. The service also refuses to start if `PHANTOM_SEALED_SALT_HEX` is in the environment (`refuse_if_sealed_salt_present`). (verified in code)

::: judge
**JUDGE MAY ASK**

Question: Is the dashboard showing real data? And how does the Lab relate to the real system?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

The observation endpoints return counters and latency measured in the running gateway, and each field is meant to carry a provenance label. Some placeholder values, such as the three demo nodes and the startup snapshot, are synthetic; the gateway only emits them as live when simulation mode is switched on, and otherwise returns null or labels them. The Lab is separate from serving. Its T1 mode is a simulation that reuses the real cache and policy code with a virtual clock, so its numbers are simulated. Its T2 mode starts a real gateway and a scripted origin on one machine, so its numbers are measured, but on loopback only.

**If the judge asks further:**

"Can I trust the dashboard numbers as production performance?" No: loopback, one machine, and some views are simulated or inferred. Check the label before quoting a number.

**Evidence to know:** `build_schema_v1_payload` (provenance map), `PHANTOM_SIMULATION` (`config.rs`), `src/lab/mod.rs` header comment, `src/lab_live/mod.rs`; tests `telemetry_honesty_tests`, `schema_deprecation_tests` (TESTED, details in Chapters 6 and 7).
:::

## 2.9 Configuration and deployment

### 2.9.1 How configuration is resolved

Precedence is command line, then environment, then the `--config` file (`KEY=VALUE` lines), then the default (`src/config.rs` header and `USAGE`). Unknown keys and duplicates in the file are errors, and all errors are collected. Exit code 2 means invalid configuration, 1 runtime failure, 0 clean exit or a valid `--check-config`. Secrets are redacted in the effective-configuration report. CLI flags: `--config`, `--bind`, `--mode`, `--origin`, `--check-config`, `--version`, `--help`. (verified in code)

`PHANTOM_MODE=production` turns on extra errors: it requires `PHANTOM_REQUIRE_AUTH=true` and a JWT secret, forbids `PHANTOM_SIMULATION`, forbids every insecure override and credential forwarding, and makes the default CORS list empty (`config.rs`, production block).

### 2.9.2 The real keys

There are 43 keys in `config::KEYS` (counted). "Main" = applied in `main::run`; "Env" = exported to the process environment and read by `server.rs`, `router.rs` or `cluster.rs`; "NOT WIRED" = validated and reported only.

| Key | Default | Wiring | Notes |
|---|---|---|---|
| `PHANTOM_MODE` | `development` | Main | `production` adds strict rules |
| `PHANTOM_BIND_ADDR` | `127.0.0.1:8085` | Env | non-loopback requires JWT and TLS (or the insecure-HTTP override) |
| `PHANTOM_TLS_CERT`, `PHANTOM_TLS_KEY` | unset | Env | ingress TLS; set together |
| `PHANTOM_ALLOW_INSECURE_HTTP` | `false` | Env | plaintext on a network bind; JWT still mandatory |
| `PHANTOM_REQUIRE_AUTH` | `false` | Env | startup fails unless a JWT secret exists |
| `PHANTOM_JWT_SECRET` | unset | Main | HS256 shared secret; enables RBAC; 32 bytes or more required in production |
| `PHANTOM_JWT_ISSUER` / `PHANTOM_JWT_AUDIENCE` | `phantom-cache-auth` / `phantom-cache-api` | Main | expected claims |
| `PHANTOM_MAX_CACHE_BYTES` | 268435456 (256 MiB) | Main | logical byte budget, split evenly across 8 shards |
| `PHANTOM_DEFAULT_TTL_MS` | 60000 | **NOT WIRED** | literal `60_000` in `server.rs` (section 2.6.1) |
| `PHANTOM_MAX_STALE_MS` | 86400000 (24 h) | Main | hard cap on stale serving |
| `PHANTOM_SWEEPER_BATCH_SIZE` | 32 | Main | see discrepancy D-5 |
| `PHANTOM_SWEEPER_INTERVAL_MS` | 100 | Main | sweeper tick |
| `ORIGIN_URL` | unset | Env (`main` also copies it into `ApiState.origin_url`) | not `PHANTOM_`-prefixed; only host and port are used |
| `PHANTOM_ORIGIN_ALLOWLIST` | unset | Env | non-empty means the origin must match |
| `PHANTOM_ENFORCE_UPSTREAM_TLS`, `PHANTOM_ALLOW_INSECURE_UPSTREAM`, `PHANTOM_ALLOW_LAN_ORIGIN`, `PHANTOM_FORWARD_UPSTREAM_AUTH` | all `false` | Env | upstream policy switches |
| `PHANTOM_UPSTREAM_CA_CERT` | unset | Env | extra CA for upstream TLS |
| `PHANTOM_CLUSTER_ID` | `phantom-cluster` | Env | must equal the label in certificate SANs |
| `PHANTOM_CLUSTER_MEMBERS_FILE` | unset | Env | static membership and fingerprints |
| `PHANTOM_CLUSTER_CA`, `PHANTOM_NODE_CERT`, `PHANTOM_NODE_KEY` | unset | Env | cluster mTLS material |
| `PHANTOM_NODE_ID` | 1 | Env | must exist in the registry |
| `PHANTOM_CLUSTER_LISTEN_ADDR` | unset | Env | required when mTLS is configured |
| `PHANTOM_CLUSTER_PEER_ADDRS` | unset | Env | `id=ip:port,...` overrides membership addresses |
| `PHANTOM_ALLOW_INSECURE_CLUSTER`, `PHANTOM_STRICT_CLUSTER` | `false` | Env | insecure cluster flag (leads to the no-frames mode); strict loopback rule |
| `PHANTOM_CLUSTER_TIMEOUT_MS` | 150 | Env | per-attempt invalidation timeout |
| `PHANTOM_INTELLIGENCE` | `off` | Main | `off`, `observe`, `advisory`, `canary`, `active` |
| `PHANTOM_ORIGIN_CAP_MAX` | unset | Main | unset means no cap configured; enforced only in `canary` and `active` |
| `PHANTOM_ORIGIN_CAP_MIN`, `_P99_CEILING_MS`, `_WINDOW` | 1, 1000, 100 | Main | AIMD controller parameters |
| `PHANTOM_SIMULATION` | `false` | Env | produces synthetic dashboard values |
| `PHANTOM_CORS_ORIGINS` | unset | Main | exact origins; development profile defaults to `http://localhost:3000` and `http://127.0.0.1:3000` |
| `PHANTOM_LAB_ENABLED`, `PHANTOM_LAB_DIR`, `PHANTOM_LAB_GATEWAY_EXE` | `false`, `lab-results`, unset | Main | Lab API switches |
| `PHANTOM_CONFIG` | unset | Main | config file path (same as `--config`) |

Other environment variables found by search are not gateway configuration: `PHANTOM_SEALED_SALT_HEX` (Lab evaluation only, must be absent from the serving process), `RUSTC`, `PROCESSOR_IDENTIFIER`, `NUMBER_OF_PROCESSORS` (Lab provenance). Fixed constants that look like settings but are not configurable: 8 shards (`PartitionedCache::new(8)` in `main.rs`), 10,000 entries per shard, 1,000 waiters per key, 128 background refreshes, the 5 s origin timeouts, the 2 MiB origin body cap, the 512 KiB request body limit, the 150 ms default invalidation deadline (configurable via the key above).

### 2.9.3 What deployment exists

Found by listing the repository (excluding `node_modules`, `target`, `.git`):

- `packaging/build-release.ps1` and `build-release.sh`: build a release bundle with a tarball and SHA256 sums, stamping `PHANTOM_BUILD_COMMIT` and `PHANTOM_BUILD_DATE`. They do not commit, tag or publish. IMPLEMENTED; whether a bundle was built and verified on this tree is UNVERIFIED.
- `packaging/smoke-test.sh` and `.ps1`: run `--version`, `--check-config` on the example, reject an invalid boolean, start the server on a loopback port and probe `/health`, `/health/live`, `/health/ready`. IMPLEMENTED.
- `packaging/phantom.example.conf` and `phantom.production.example.conf` (the production example binds `0.0.0.0:8443` with TLS, auth required, an origin allowlist and intelligence off), plus `packaging/verify-checksums.sh`.
- `Start-Phantom-Demo.bat`: a Windows demo launcher (release build if missing, gateway on 8085, Vite on 3000, opens the browser).
- `.github/workflows/ci.yml`: a CI definition (format check, `cargo check`, clippy, tests, a frontend job). It is a build pipeline, not a deployment; whether it has run is UNVERIFIED.
- `docs/user/` holds install, run, operate, configuration, security and troubleshooting guides.

What does not exist: no `Dockerfile`, no compose file, no Kubernetes manifest, no systemd unit, no Helm chart, and no Windows service wrapper were found. The UI has no production serving path. In-flight connection draining on shutdown is not implemented (the code says so in a comment in `main.rs`: it is a "deferred integration item"); the cache is in memory only, so a restart empties it and there is nothing to flush. The only multi-process evidence in the repository is on one machine over loopback; no physical multi-host deployment has been validated.

::: judge
**JUDGE MAY ASK**

Question: Is this deployable in production today? What exists for deployment?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is a research and engineering prototype with real hardening, not a production-ready product. What exists: a configuration system that fails closed, a production profile that refuses insecure settings, TLS and JWT options, health endpoints for liveness and readiness, release packaging scripts with checksums, a smoke test, and CI definitions. What does not exist: container images, orchestration files, service units, graceful connection draining, a built UI served by the gateway, durable invalidation across restarts, and any multi-host validation. The cluster mode is labelled experimental in the binary itself.

**If the judge asks further:**

"What would you need before production?" The list is in Chapter 8: durable invalidation, tenant quotas, multi-host validation, graceful shutdown, a release-profile benchmark on real hardware, and independent review. "Does the config file cover everything?" Almost: one key, the default TTL, is accepted but ignored.

**Evidence to know:** `packaging/`, `src/config.rs::KEYS`, `src/health.rs`, `main.rs` shutdown comment; `docs/user/limitations.md`; `docs/STATUS.md` (no milestone has a recorded formal acceptance).
:::

## 2.10 Auth boundaries at a high level

Chapter 6 owns the detail. For architecture, know the five boundaries.

- **Ingress.** Optional JWT (HS256, five roles: `cache:read`, `cache:invalidate`, `telemetry:read`, `scenario:operate`, `admin`). Authentication runs before the cache lookup, tenant resolution, coalescing or origin fetch on every cache route, including `x-cache-bypass`. In secure mode the tenant comes only from the token. Ingress TLS is optional (`PHANTOM_TLS_CERT`, `PHANTOM_TLS_KEY`).
- **Browser.** CORS allows only exact configured origins and never credentials; it is not authentication.
- **Upstream.** The origin URL passes the upstream policy (allowlist, IP filter, pinned address) as an SSRF defence.
- **Cluster.** Mutual TLS, certificate SAN identity, fingerprint match against a static registry.
- **Lab.** Off by default; loopback-only without JWT; Host and Origin guard.

Status: IMPLEMENTED; TESTED by the security suites (`security_phase1_tests` to `security_phase4c_antientropy_tests`, `m7_sec_*` tests), which Chapter 6 evaluates. Passing security tests do not make the system secure; the open findings are in the G4 disposition register.

## 2.11 Discrepancies between documents and code

Recorded in full in `docs/learning/evidence/ch02_evidence.md`. The ones a reader should know:

1. `docs/research/adr/ADR-001-present-architecture-and-direction.md` (dated 2026-10-28, status PROPOSED) is partly out of date. It says DELETE purges the local cache only after the cluster round trip, that no code links a node's cache to the serving cache, and that reconciliation has no caller. Current code purges locally first (`delete_cache_handler`), builds the local node with `new_with_cache` in the mTLS path, and starts the anti-entropy worker in `main.rs`. `CLAUDE.md` section 6 already flags parts of this.
2. `src/config.rs` (around line 858) still warns in cluster mode that "peer invalidations do not purge this gateway's HTTP cache". The startup message in `src/main.rs` (around line 373) and `docs/user/limitations.md` now say the local node shares the serving cache in the mTLS path. The `config.rs` warning text is stale.
3. A code comment in `delete_cache_handler` claims "linearizability". No evidence supports that word (section 2.7).
4. `PHANTOM_SWEEPER_BATCH_SIZE` is described as a "shards per tick" limit (`config.rs` unit) and as a shard inspection limit (`CLAUDE.md`). In `PartitionedCache::sweep_lazy_reclamation` the argument is `max_per_shard`: the sweeper visits every shard each tick and stops adding reclamation candidates in a shard at that count; entries that are not reclaimable are still iterated, so the limit bounds reclamations per shard per tick, not entries inspected. Chapter 4 should confirm the lock-hold consequence.
5. `PHANTOM_DEFAULT_TTL_MS` is unwired (section 2.6.1), as `CLAUDE.md` says; re-verified.

## Key ideas to remember

- One gateway process holds the product: Axum router, sharded cache with single-flight, origin fetcher, optional JWT, optional experimental cluster.
- Hit: two short shard-lock acquisitions and no origin call. Miss: one leader fetches, waiters share the answer, a generation and epoch fence decides whether to store it.
- Stale serving happens only when the origin's own headers allow it, and never beyond the 24-hour hard cap. With no directive, an outage means errors.
- GET is local-only. Rendezvous hashing selects the acknowledging primary for DELETE and nothing else. The peer read handler is a stub with no production caller.
- DELETE purges locally first, waits up to 150 ms for the primary's acknowledgement, then fans out in the background; owed invalidations live in memory only.
- Phantom Lab and the UI observe or simulate the gateway; they are not on the serving path. T1 is simulated; T2 is real but on loopback.
- Deployment today means build scripts, a smoke test, example configs and CI. There are no containers or service units.

## What you should now understand

- Which process, port and binary does what, and why the UI is a separate dev server.
- Each real route, who may call it, and which routes exist only when a flag is on.
- The exact order of events on a hit, a miss, a failed origin fetch, and a cluster DELETE.
- Why a DELETE cannot be resurrected by a slow fetch (the generation fence) and why a hit needs no origin call.
- What "node" means in this code and what the cluster really coordinates.
- Which configuration keys are applied, which are exported, and which one is ignored.
- Which sentences you must not say: "distributed reads", "linearizable", "serves stale during any outage", "production ready".

## Knowledge check

1. List the `X-Cache-Status` values and say when each is produced.
2. Why does a hit take the shard lock twice, and what does each acquisition decide?
3. A client sends `GET /api/v1/cache/a?x=1&y=2` and another sends `?y=2&x=1`. How many cache entries and origin fetches can result, and why?
4. State what happens, step by step, when 100 clients request the same missing key and the origin takes 150 ms.
5. When does the gateway serve an expired entry, and what stops it from doing so indefinitely?
6. What does a `204` from `DELETE` guarantee in a cluster, and what does it not guarantee?
7. Which function chooses the primary for a key, what is its weight factor, and which request type uses it?
8. Name three things in the repository that look like deployment but are not production deployment.
9. Where does the Lab's T1 simulation run, and where does T2 run? What label does each result carry?
10. Which configuration key is accepted but has no runtime effect, and where is the literal that overrides it?

::: answers
1. `HIT` (fresh entry), `MISS` (leader fetched), `COALESCED` (waited for another's fetch), `STALE` (expired entry served under a stale directive, or stale-if-error after a failed fetch), `BYPASS` (client asked to bypass), `BYPASS-AUTH` (credential presented while credentials are forwarded upstream).
2. First `check_stale_serving` decides whether an expired entry may be served stale; a fresh entry is `Ineligible`. Then `begin_fetch_atomic` decides hit, wait or initiate. They are separate locked calls.
3. Two distinct keys, because the query string is used verbatim. That can mean two origin fetches and two stored entries for the same logical resource.
4. First request: `Initiate`, becomes leader, spawns the fetch task. Other 99: `Wait`, joined to the same in-flight operation. One origin request. On completion `finalize_fetch` stores the result if generation and epochs match; waiters receive the body; leader reports `MISS`, others `COALESCED`. (Test: `test_m1_t13_barrier_stampede_100_clients`.)
5. Only when the entry's stored freshness metadata has a stale-while-revalidate window (for normal serving) or stale-if-error window (when the origin failed), and `must-revalidate` is absent. The age must be within lifetime plus the smaller of the window and `PHANTOM_MAX_STALE_MS`; past lifetime plus the hard cap the entry is `CapExceeded`.
6. It guarantees: this gateway removed its copy first, and a primary (or acting primary) acknowledged the invalidation within the 150 ms leg. It does not guarantee that other gateways have removed their copies: they are invalidated asynchronously with bounded retry and repair while the issuing process lives, and owed work is lost if that process dies.
7. `RendezvousRouter::route_key`; the weight factor is the constant 1.0 (`capacity_weight`); it is used by the DELETE invalidation path (`invalidate_key_across_cluster`).
8. Examples: `Start-Phantom-Demo.bat` (demo launcher), `.github/workflows/ci.yml` (CI, not deployment), `packaging/smoke-test.*` (verification), the `packaging/*.example.conf` files (examples). No Dockerfile, compose file, or service unit exists.
9. T1 runs in-process in the gateway (Lab API) or in the `phantom-lab` CLI, with a virtual clock; labelled SIMULATED. T2 runs a real child gateway, a scripted origin and a load generator on loopback; labelled MEASURED_LOOPBACK.
10. `PHANTOM_DEFAULT_TTL_MS`; `src/api/server.rs` passes `60_000` to `evaluate_response_freshness` in `get_cache_handler` (miss path and background revalidation path).
:::

## Codebase exploration tasks

1. **Trace a hit.** Start at `src/api/server.rs::get_cache_handler`. Find the `Hit` arm of the `match cache.begin_fetch_atomic(...)`. Then open `PartitionedCache::begin_fetch_atomic` in `src/core/cache.rs` and find the epoch comparison. You should learn which two conditions make an entry a hit and what the `visited` flag is for.
2. **Find the fence.** Start at `PartitionedCache::finalize_fetch`. Find `state.generation == op_gen` and the two epoch comparisons, then find where `delete_impl` increments `generation`. You should learn how a DELETE during a fetch prevents the stale body being stored, and which counter records it (`stale_writes_prevented`).
3. **Prove GET never leaves the node.** Search `src/api/server.rs` for `router` and `.nodes`. Find the single use in `delete_cache_handler`, then search `src/` for `forward_key_over_tcp`. You should learn why the cluster is an invalidation system, not a read-sharding system, and where the peer `GetRequest` stub lives in `src/dist/node.rs`.

## Challenge questions

1. **A hot key under cluster DELETE.** A key is requested 20,000 times per second on each of three gateways and an operator deletes it. Using only what the code does, describe the window in which a client can still see old data on each gateway, and name the two mechanisms that bound it. What would you change to shrink it?

   Model answer. On the gateway that handled the DELETE, the entry is purged before the cluster round trip, so new requests miss and one leader fetches fresh data; fetches already in flight are detached and fenced by the generation, so they cannot store old data. On the other two gateways the old entry remains until their invalidation frame arrives (queued after the primary's ack, normally within milliseconds on a healthy network but with no bound) or until the entry's lifetime ends (the only hard bound). If a peer is unreachable the bound is the lifetime, plus repair delivery whenever it recovers, provided the issuing gateway has not restarted. To shrink the window: make owed invalidations durable (milestone M3, not integrated), send the fan-out before waiting on the primary, set shorter origin lifetimes, or route reads of a key through its owner (a design change that does not exist today).

2. **Why is the origin fetch in a spawned task?** Suppose you moved the fetch inline into the request handler. Name two behaviours that break and one that gets simpler.

   Model answer. If the leader's client disconnects, Tokio drops the handler future and the fetch is cancelled, so every waiter would receive an abort; today the detached task finishes and serves them (`test_ct2_initiator_cancellation_detached_fetch`). Panic or cancellation inside the handler could leave an in-flight entry registered; the guard types (`FetchRegistration`, `OriginFetchGuard`) clean up in `Drop`, but the cleanup would run on the request path. What gets simpler: the leader's own response path, which would no longer need to register as a waiter and wait on a channel.
