# DR-029 Investigation: cluster `GetRequest` handler fabricates and stores a body in the shared serving cache

Status: investigation report (read-only on the repo; no fix applied, none authorised).
Date: 2026-10-30. Investigator: DR-029 investigator agent.
Repo state at start and end: HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, 94 entries in `git status --short` before and after (this file is the one addition, made after the final count).
Subject: `src/dist/node.rs:409-482` (handler), pinned by `tests/integration/known_gaps.rs:562` (KG-6), listed as DR-029 in `docs/research/vision/discrepancy_register.md:45`.

Evidence classes used: IMPLEMENTED (verified by reading code, `file:line`), TESTED (an existing repo test asserts it), MEASURED (my own scratch reproduction: debug build, loopback, test harness, nothing more), INDEPENDENTLY VERIFIED (a second party other than the original finder; none exists yet), UNVERIFIED, BLOCKED. My reproduction is MEASURED and is not independent verification of the lead's finding: I am a second reader of the same code, briefed with the finding, and my reading largely agrees with the lead on the handler and disagrees on one consequence (section 1).

## 1. Executive finding

1. The lead's code reading is correct on the handler (IMPLEMENTED, MEASURED): an authenticated cluster member that sends a `GetRequest` for an uncached key makes the node store a fabricated 200 `text/plain` resource with body `val_node_<id>`, TTL 60 000 ms, no ETag, in the node's cache (`node.rs:446-458`). In the mTLS production path that cache is the HTTP serving cache (`main.rs:332`). A second peer `GetRequest` for the same key then returns `HIT val_node_1` (MEASURED, R1-R3).
2. The register's stated consequence is NOT supported: "An authenticated peer can plant a bogus 200 body for any key in the gateway's real cache" and "HTTP clients may then be served" the value. The planted key is always tenant `__default__` with a path that does **not** start with `/` (and carries no query). The cluster handler rejects any key beginning with `/` (`node.rs:419`, round-3 fix M-2, TESTED by `m7_sec_round3_tests.rs:331`), while the HTTP GET path always normalises its path to start with `/` (`server.rs:1249-1253`). The two key spaces are disjoint as `CacheKey` structs (the cache map is keyed by `CacheKey`, `cache.rs:22`, derives Hash/Eq), so no HTTP request can address a planted entry. MEASURED: five HTTP spellings of the corresponding key all reached the origin or HTTP-fetched entries; no HTTP response contained `val_node_` (R4).
3. What does remain, all IMPLEMENTED and partly MEASURED: (a) fabricated data is stored in, and served from, the production cache's cluster-namespace to any authenticated member; (b) a member can fill cache entries and bytes with garbage and evict real HTTP entries (MEASURED: 20 of 20 unvisited HTTP entries evicted by 3 000 peer keys in a 64 KiB-budget test cache); (c) planted entries pollute cache hit/miss counters (IMPLEMENTED, not measured); (d) the stub is a ready-made hazard for the peer-fill design: any change that lets HTTP reads reach non-`/` keys, or that routes a peer fill into the same namespace, turns it into real poisoning; (e) a fabricated-data path violates the project rule against fabricated values.
4. Classification: a **design defect with a bounded, authenticated, availability/resource-consumption and integrity-of-cluster-namespace impact**. It is **not** an exploitable remote vulnerability from outside the cluster trust domain, and **HTTP-visible cache poisoning is not demonstrated and is, for the code I read, structurally prevented**. Suggested working severity: Low to Medium. The register's "high integrity impact" should be restated (section 9).

Severity rationale (CVSS-like reasoning as an explicit aid only; no score is claimed): attack vector is network but only to the cluster listener, which must be bound to loopback or have mTLS (`node.rs:185-195`); privileges required are high (a CA-signed client certificate whose SAN names a node id present and active in the membership registry, with a matching fingerprint: `node.rs:263-330`, `cluster.rs:365-402`); complexity low once the identity exists; no user interaction; scope unchanged; confidentiality none (a peer can only read entries in the non-`/` namespace, which only this handler writes); integrity low and confined to that namespace; availability low to medium (cache eviction, memory/entry-slot pressure, bounded by the cache budget). A member holding that identity can already flush every peer cache (DR-027, `discrepancy_register.md:43`), so the marginal capability this handler adds to a compromised member is small (eviction of chosen survivors, counter pollution, fabricated reads inside its own namespace). It matters more as a standing hazard for the peer-fill roadmap than as a present exploit.

## 2. Reachability (objective 1)

| Step | Evidence | Class |
|---|---|---|
| The only non-test caller of `start_tcp_service` is `main.rs:334`, inside the mTLS branch that requires `PHANTOM_CLUSTER_CA`, `PHANTOM_NODE_CERT`, `PHANTOM_NODE_KEY` (`main.rs:277-279`, 255-296), `PHANTOM_CLUSTER_MEMBERS_FILE` (else startup aborts, `main.rs:284-289`) and `PHANTOM_CLUSTER_LISTEN_ADDR` (`main.rs:300-304`) | `grep start_tcp_service src` gives `main.rs:334` only (outside `node.rs` itself); `src/bin/phantom-lab.rs` does not call it | IMPLEMENTED |
| Local node there: `VirtualNode::new_with_cache(node_id, cache.clone())`, `set_cluster_tls`, then `start_tcp_service(&cluster_listen_addr)` | `main.rs:332-334` | IMPLEMENTED |
| Insecure / partial cluster mode (`PHANTOM_ALLOW_INSECURE_CLUSTER` or members file without certs): no listener is started; the three placeholder nodes are built with `VirtualNode::new` and never bound | `main.rs:380-381`, nodes at `main.rs:207-210`; `start_tcp_service` not called | IMPLEMENTED |
| `start_tcp_service` without `cluster_tls` accepts plaintext frames (`node.rs:342-345`) but only on a loopback bind; a non-loopback bind without TLS fails closed (`node.rs:185-195`). Production `main.rs` never reaches that state. Tests do (`known_gaps.rs:564`, `security_phase3b_framing_tests.rs:133`) | `node.rs:185-195,342-345` | IMPLEMENTED, TESTED |
| Client side: `forward_key_over_tcp` (`router.rs:1420`) is the only sender of `new_get_request` in `src` (`router.rs:1445`); grep of `src/` and `tests/` shows it is called only from tests | grep | IMPLEMENTED |

Consequence: in a production-shaped process the handler is reachable only through the mTLS cluster port, and no gateway code ever sends a `GetRequest`. The handler is reachable solely by a peer that crafts the frame. Whether any deployment binds `PHANTOM_CLUSTER_LISTEN_ADDR` to a non-loopback interface exposed beyond the trust domain is an operator matter I did not examine (UNVERIFIED); with mTLS that exposure still requires a valid certificate.

## 3. Authentication and authorization boundary (objective 2)

Path for one inbound connection (`node.rs`):

1. Accept; drop if node state is `Failed` or `Offline` at accept time only (`:250`). State is not re-checked per frame.
2. Handshake bounded by a 64-permit semaphore and 1000 ms timeout (`:263-280`). rustls server config demands client authentication through `WebPkiClientVerifier` against the cluster CA (`cluster.rs:623-629`).
3. Peer certificate must exist (`:296-300`), have exactly one DNS SAN (`:309`), parse to `node-<id>.<cluster>.phantom.internal` (`parse_san_node_identity`, `cluster.rs:70`).
4. `validate_peer`: cluster id match, node id present in registry, member `active`, certificate fingerprint in the member's fingerprint list (`cluster.rs:365-402`, called at `node.rs:324`).
5. Session registered under `identity.node_id` (`:330`) so revocation can cancel it (`:152-168`).
6. Per frame: `read_frame` enforces magic, version, header and `MAX_FRAME_PAYLOAD_BYTES` 1 MiB (`frame.rs:14,793+`); then identity is re-validated against the registry on every request (`:393-407`).
7. `GetRequest` arm: `extract_key` (UTF-8, payload at most `MAX_KEY_LENGTH_BYTES` 4 096, `frame.rs:419-430`; failure closes the connection), then `starts_with('/')` and `CacheKey::validate` (`:419`).

Not checked (IMPLEMENTED, by absence in the code above): no authorization scope or role for GetRequest; the authenticated `identity` is not passed to the handler at all (it is used only for admission and session tracking), so there is no per-peer or per-tenant decision, no per-peer rate limit, no per-session request or byte quota, no check that the key belongs to this node by rendezvous placement, no check that the peer is permitted to read or write the tenant, no node-state check beyond the Recovering/Suspect branch (section 6), and no check that the sender is not the local node id. The semaphore bounds handshakes only. Plaintext loopback path (tests only): identity is `None`, so none of 4 and 6 applies.

## 4. Key control and collision analysis (objectives 3, 4, 5)

Cluster key construction: `CacheKey::new(None, key, None)` (`node.rs:415`): tenant is hard-coded `__default__`, `query` is `None`, `path` is the payload string. The peer controls only `path`: any UTF-8 string up to 4 096 bytes that (a) does not start with `/`, (b) passes `validate()`: no bytes at or below 0x20 or 0x7f, no `.` or `..` segments, no `":?"`, canonical length at most 4 096 (`cache.rs:38-90`). Not rejected: an empty string, non-ASCII, `:` elsewhere, `%`, `?`, `#`. The peer cannot choose a tenant (the `__default__` tenant is the only target) and cannot set query.

HTTP key construction: `norm_path` is forced to start with `/` (`server.rs:1249-1253`, same in the DELETE path at `:1976-1980`), tenant is the JWT `tenant_id` in secure mode (`__default__` is rejected as reserved, `server.rs:1054`) or the `x-tenant-id` header in dev mode (`header_tenant`, `:1129`; `__default__` is accepted there), query from `RawQuery`. Both paths call `begin_fetch_atomic` on the same `PartitionedCache` in the shared-cache configuration.

Collision result: for a cluster entry to be hit by HTTP, the HTTP path would have to equal a string not beginning with `/`; it cannot, for every route (`/api/v1/cache/*key` and the root route, `server.rs:202-209`, `1108-1114`, all go through `norm_path`). MEASURED (R4): after the peer planted `dr029_probe`, HTTP GETs of `dr029_probe`, `/dr029_probe` (i.e. `//dr029_probe` in the URL), `dr029_probe` with `x-tenant-id: __default__`, `dr029_probe` with `x-tenant-id: node-1`, and `%2Fdr029_probe` returned the origin body (`{"origin_key":...}`); the first was a MISS and went to the origin (origin request count 1), the later `__default__`-tenant and `//` variants were HITs of the HTTP-fetched `/dr029_probe` entry, not of the planted one; the node-1 tenant was a separate MISS. No response contained `val_node_`. Peer `GetRequest "/dr029_probe"` returned `ERR 400` (R5), confirming the cluster side cannot reach HTTP keys either.

Vary headers: the cache key has no Vary component (the HTTP cached representation is shared across clients: `server.rs:1222-1224`), so Vary does not open a path to the cluster namespace. Anonymous/dev mode versus JWT mode: no difference for the collision question, because the separator is the leading `/`, not the tenant. In JWT mode the `__default__` tenant is unreachable by HTTP entirely, so a cluster entry there is doubly isolated.

Is the stored resource complete enough to be served? Fields: status 200, content type `text/plain`, body `val_node_<id>`, `etag: None`, `created_at_ms: now`, `ttl_ms: 60 000` (`node.rs:446-458`). It is a structurally valid `CachedResource`; if an HTTP lookup ever reached it, the Hit branch (`server.rs:1520+`) would serve it as an ordinary hit with `Age`, conditional handling by `If-Modified-Since` against `created_at_ms`, no ETag. That is a conditional statement about a path I showed does not exist; I did not test it (it cannot be reached without changing source).

Residual collision caveats (UNVERIFIED, hypotheses): (i) anything other than the checked HTTP handlers that constructs `CacheKey` from external input and skips `norm_path` (e.g. lab APIs in `src/api/lab_api.rs`, `src/lab_live/`; I did not trace them); (ii) invalidation frames use canonical strings (`__default__:<path>`) and `from_canonical_string` (`cache.rs:100-116`), so a `Tenant("__default__")` or `FullCache` invalidation and an `ExactKey` of `__default__:<key>` will remove planted entries; that is by design, not a collision with HTTP.

## 5. Behaviour when the entry is planted (objective 7 mechanics)

- Store path is the normal one: `begin_fetch_atomic` (`cache.rs:1238`) registers the fetch, `guard.complete(Success)` calls `finalize_fetch` (`cache.rs:1401`), which stores only if `state.generation == op_gen` and the tenant and global epochs still match (`cache.rs:1463-1465`). Fencing therefore applies (IMPLEMENTED); an invalidation racing the plant prevents the store and increments `stale_writes_prevented`. The plant records the current epochs, so a later epoch advance (Tenant/FullCache invalidation) removes it on next lookup (`cache.rs:1250-1268`).
- Accounting: planted entries are charged to the shard byte budget like any entry. MEASURED: `audit_bytes()` accounted equals recorded: (458, 458) after one plant; (64 286, 64 286) and (64 900, 64 900) after the floods. The byte-accounting invariant is not bypassed. There is no per-tenant byte quota in `cache.rs` (grep for tenant quota terms found none), so the planted `__default__` tenant draws on the same global shard budgets as everyone (IMPLEMENTED by absence).
- Lifetime: fixed 60 s from the plant; the freshness engine, stale-while-revalidate and `PHANTOM_MAX_STALE_MS` are not involved. A peer can re-plant after expiry or after invalidation at will: nothing limits repeats (MEASURED: 3 000 distinct keys in 327 ms over one connection, debug loopback).
- Counters: `begin_fetch_atomic` increments `cache_hits` or `cache_misses` (`cache.rs:1280-1290`, `1344`), so peer traffic changes the same counters the dashboards and the intelligence layer read (IMPLEMENTED; not measured).
- Eviction pressure: inserting a key that does not fit evicts other entries, up to 32 per insertion, excluding only the inserted key (`cache.rs:1326-1335`). MEASURED (R7, R8; test cache with a 64 KiB budget, 4 shards, 3 000 distinct 12-byte keys): a victim that had been fetched once and then hit (visited) stayed resident; 20 of 20 victims fetched once (unvisited) were evicted. Total evictions 2 861 and 2 900 respectively (most of them peer entries evicting each other). This shows the mechanism, not a production impact: the production budget is `PHANTOM_MAX_CACHE_BYTES` and a per-shard entry cap (default 10 000, `cache.rs:330`), 8 shards in `main.rs:203`, and which entries survive depends on the configured eviction policy. By reading, an attacker would need on the order of the byte budget or 80 000 distinct keys (8 shards x 10 000) to displace entries cache-wide; keys of up to 4 096 bytes make byte-filling cheaper in request count (UNVERIFIED arithmetic).

## 6. Node-state behaviour (objective 6)

| Node state at request time | Branch (`node.rs`) | Stores? | MEASURED (R6) |
|---|---|---|---|
| Active (set by `start_tcp_service`, `:198`) | `begin_fetch_atomic`: Hit returns `HIT <body>`, Initiate stores and returns `MISS val_node_<id>`, Wait returns `COALESCED`, Rejected returns `ERR 503` | Yes on Initiate | MISS then HIT |
| Joining, Degraded | same branch as Active (only Recovering and Suspect are special-cased, `:436`) | Yes | Joining: MISS then HIT; Degraded: MISS then HIT |
| Recovering, Suspect | SEC-POA-04 comment at `:437`: returns `MISS val_node_<id>` without touching the cache | No (no lookup, no registration, no store) | Recovering: after returning to Active the same key was MISS again (not stored); Suspect: same |
| Failed, Offline | connection dropped at accept (`:250-254`); existing pooled sessions are not re-checked per frame | not applicable at accept | not tested |

So the Recovering/Suspect branch avoids storing, and also avoids serving; it still returns the fabricated body to the caller, which is the same fabricated-data property without the cache effect. Joining and Degraded are not protected. In production the local node's state is set Active at start; I did not trace whether `router.rs` ever marks the local shared node Suspect (calls at `router.rs:1212-1645` act on `node` objects iterated by the router, which includes the local node in the node list; UNVERIFIED whether the local node is ever among them).

## 7. Reproduction (objective: safe reproduction)

Scratch only; no repo file other than this document was written.

- Scratch copy: `D:\Temp\dr029_scratch` (copied `src`, `tests`, `benches`, `Cargo.toml`, `Cargo.lock`, and `docs/demo/scenario_b_interpretation_notice.md`, which `src/api/lab_api.rs:67` `include_str!`s). Added to the scratch `Cargo.toml` only: `[[test]] name = "dr029_repro"`. Test source: `D:\Temp\dr029_scratch\tests\integration\dr029_repro.rs` (two tests).
- Environment: Windows 11, debug profile, loopback. `CARGO_TARGET_DIR=D:\Personal\_dr029_target`, `TEMP=TMP=D:\Temp`. Did not use ports 3000, 8085, 18086 or `D:\Personal\_m7_target`. All listeners bound to `127.0.0.1:0` and stopped at test end; no process left running.
- Harness: test CA via `rcgen`; node 1 (`VirtualNode::new_with_cache(1, cache)`, mTLS, as `main.rs:332`) and an in-process HTTP app over the same `PartitionedCache` (`create_app`, no security config, mock origin on loopback); legitimate peer node 2 with a CA-signed certificate in the registry connecting through `ClusterTlsConfig::client_connector()`. No authentication or authorization weakened.
- Commands and results:
  1. `cargo test -j 1 --test dr029_repro -- --nocapture`: cold build 5 min 59 s (12:54:18 to 13:00:18 IST), exit 0, 1 test passed (finished in 0.38 s), no warnings.
  2. After adding the second test: `cargo test -j 1 --test dr029_repro -- --nocapture --test-threads 1`: 10.8 s, exit 0, 2 passed (0.70 s).
  3. `cargo test -j 1 --test known_gaps pin_kg6 -- --nocapture`: 10.3 s, exit 0, 1 passed (`pin_kg6_node_get_request_fabricates_missing_key_body`), 9 filtered out.
  Logs: `D:\Temp\dr029_run1.log`, `dr029_run2.log`, `dr029_run3.log`.
- Results (MEASURED in a debug loopback test harness; nothing is claimed about production configurations I did not run): R1 `MISS val_node_1`; R2 `HIT val_node_1`; R3 cache holds `__default__:dr029_probe` status 200, `text/plain`, body `val_node_1`, ttl 60 000, no etag; R4 no HTTP response served the fabricated value (five spellings, section 4); R5 `/`-prefixed peer key `ERR 400`; R6 state matrix in section 6; R7/R8 eviction results in section 5.
- Not tested: JWT secure mode (HTTP side), non-loopback binds, real certificates from the deployment tooling, multi-process, release profile, the production eviction policy defaults and budget, repeat runs (each result is one run; no confidence intervals are claimed or needed for the yes/no structural observations, but the flood timing (327 ms for 3 000 keys) is a single debug run and must not be quoted as a rate).

## 8. Existing tests and gaps (objective 8)

Tests that establish current behaviour (all assert the stub or its guards; none asserts "a peer cannot change HTTP-visible state"):

| File | Test(s) | Asserts |
|---|---|---|
| `tests/integration/known_gaps.rs:562` | `pin_kg6_node_get_request_fabricates_missing_key_body` | plaintext loopback node, missing key returns `MISS val_node_42` (pins the stub) |
| `tests/integration/m7_sec_round3_tests.rs:331` | `m2_cluster_get_request_keys_in_the_http_namespace_or_invalid_are_rejected` | keys `/x`, `/`, `a b`, `x\ty`, `a/../b` give `ERR 400` and do not leak a seeded `/x` body; a legitimate key still gives `MISS val_node_7`. This is the key guard that keeps the namespaces apart; uses `VirtualNode::new` and plaintext loopback, not the shared-cache mTLS configuration |
| `tests/integration/security_phase3b_framing_tests.rs` | `test_p3b_at11_valid_binary_get_request_over_mtls` (:85), `..._over_loopback` (:133), plus framing tests at ~:272-442 | `MISS val_node_1` then `HIT val_node_1`; frame validation |
| `tests/integration/security_phase4c_antientropy_tests.rs:739` | `test_p4c_sec09_read_attempts_during_recovery_bypass_cache_hits` | a Recovering node returns `MISS` despite a cached entry (SEC-POA-04) |
| `tests/integration/p2_dist_network_tests.rs` | `test_p2_multi_node_tcp_communication`, failover, timeout, replica and partition tests | `MISS/HIT val_node_` bodies over routed TCP |
| `tests/integration/security_phase3c_pooling_tests.rs` (:115, :199), `security_phase3_cluster_tests.rs` (:124, :781) | pooling and cluster identity tests | `MISS val_node_N` as the payload that proves a round trip |

Consequence for any fix: at least six test files use the stub body as their round-trip probe. Removing or failing the handler breaks them unless they are migrated to another message type; this is a cost to state to the owner, not a reason to keep the stub on the production node.

Untested security properties (UNVERIFIED because untested): HTTP-visible poisoning (none of my reproduction is in the repo); the shared-cache mTLS configuration with a `GetRequest` (only my scratch test); tenant collision; state-dependent store behaviour for Joining/Degraded/Suspect/Recovering (only Recovering served-versus-cached is tested, not "stored or not"); mTLS-only reachability and absence of a GetRequest listener in the insecure branch; per-peer rate and entry-count bounds; counter pollution; eviction of HTTP entries by peer fills.

Proposed tests (not written into the repo). Each fails on current code and passes after the corresponding fix:

1. `peer_get_request_does_not_write_serving_cache`: mTLS peer sends `GetRequest` for N uncached keys against a node built with `new_with_cache`; assert `cache.audit_bytes() == (0,0)`, entry count unchanged, `cache_misses`/`cache_hits` unchanged (fails today: bytes 458, counters change).
2. `peer_get_request_never_returns_fabricated_body`: assert the response is a typed error/unsupported or a body that is not `val_node_*`; a second request for the same key must not return `HIT` (fails today).
3. `peer_flood_cannot_evict_http_entries`: seed K unvisited HTTP entries, send 3 000 peer `GetRequest` keys, assert all K still `HIT` over HTTP and `evictions_total` unchanged (fails today: 0 of 20 survive in my run).
4. `http_cannot_observe_cluster_namespace`: regression guard for the property that currently holds: after a peer request, no HTTP spelling of the key (including `x-tenant-id: __default__`, `//`, `%2F`) returns a `val_node_` body (passes today, must keep passing; protects against a future key-normalisation change).
5. `peer_get_request_state_matrix`: Joining, Degraded, Recovering, Suspect, Active, each followed by a switch to Active; assert no state stores (fails today for Joining and Degraded).
6. `get_request_handler_unreachable_without_mtls_in_production_constructor`: process-level or constructor-level test that the production wiring in `main.rs` never starts a plaintext listener and that a non-loopback bind without TLS fails (partly exists via the fail-closed check in `node.rs:185-195`; add the main-path assertion).
7. `shared_cache_node_rejects_stub_branch`: if the recommended structural option is adopted, constructing the node with `new_with_cache` and sending `GetRequest` yields the unsupported error while `VirtualNode::new` keeps the legacy stub for harness tests.
8. `peer_get_request_rate_and_entry_bound`: once a per-peer budget exists, assert refusal after the bound.

## 9. Impact, preconditions, blast radius (objective 7)

Preconditions: a certificate signed by the cluster CA whose single DNS SAN is `node-<id>.<cluster>.phantom.internal`, an `active` registry entry for that id whose fingerprint list contains the certificate, and network reach to the cluster listener of a gateway running the mTLS configuration. Without those, the TLS handshake or registry check ends the connection before the handler runs (IMPLEMENTED, TESTED by the phase-3 cluster tests, not re-run here). Compromised-member and same-trust-domain scenarios satisfy them. Reachability from outside the trust domain: none shown; an unauthenticated or unknown-CA client is dropped at the handshake; a revoked member is dropped at the next frame (`node.rs:393-407`) or by session eviction.

Blast radius per planted key: one `__default__` tenant entry in the non-`/` namespace, 60 s, charged against the shared budget; repeated at will. Aggregate: bounded by the cache entry and byte caps, which means a member can degrade hit ratio and displace real entries (MEASURED mechanism in a small test cache). Not shown: serving fabricated data to any HTTP client, cross-tenant read or write, origin request amplification (planted entries do not call the origin), or stale-cap or fencing violations (fencing and byte accounting held in every run).

Unknown: effect with the production eviction policy and budget; whether other code paths read non-`/` keys; whether any deployment exposes the cluster port beyond the trust domain; the real-world frequency of a member being compromised relative to DR-027, which already grants flush authority.

## 10. Remediation alternatives

| Option | What it does | Pros | Cons / cost | Peer-fill and KG-6 |
|---|---|---|---|---|
| A. Delete the `GetRequest` arm (unknown message type, connection closed) | removes the surface | simplest, no fabricated data | breaks six test files that use it as a probe; `forward_key_over_tcp`, prewarm and failover tests need another probe; changes wire behaviour (connection close) | clean slate for the fill protocol (`01_distributed_systems.md` S2.2 says replace the stub); KG-6 test must be deleted or rewritten, which needs owner approval since it pins a known gap |
| A'. Keep the arm but return an explicit `ERR 501 unsupported` without touching the cache | fail closed, protocol stays framed | no cache effect; connection survives | same test churn (tests expect `MISS val_node_`); error code is new wire behaviour | KG-6 pin becomes "stub removed"; peer-fill adds a distinct `FillRequest` |
| B. Read-only lookup, no fabricated fill (return `HIT` if present else `MISS` with no body, no registration) | keeps a read probe | no write to cache | there is no read-only lookup API in `cache.rs`: `begin_fetch_atomic` registers a fetch (`cache.rs:1238`); a new API or a guard abandon path is application-code design; with the `/` guard the only entries it could ever hit are none (nothing else writes non-`/` keys), so it is a no-op wrapper | would give peer-fill a read primitive, but peer-fill needs tenant-aware keys, so this arm would not be reused as is |
| C. Separate cache for the cluster protocol (stub keeps a private `PartitionedCache`) | isolates by construction | no HTTP-cache effect, no eviction or counter effect; existing harness tests keep working if they use `VirtualNode::new` | still fabricated data on the wire; `new_with_cache` nodes need the stub disabled or pointed at a private cache; two caches to reason about | acceptable as an interim harness only; peer-fill needs real shared data, so the private cache is a dead end |
| D. Per-scope authorization | require a "cluster-read" capability per peer | aligns with `03_security_tenancy_resources.md` per-scope authorization and with DR-027 | does not remove fabricated data or eviction effect; no scope model exists today (identity-only) | prerequisite for peer-fill, not a fix for this finding |
| E. Gate behind a config flag, default off (production never enables) | flag in `config.rs`, off in `main.rs` | tests can opt in; reversible | a flag is a new configuration surface and a default-off stub that can be switched on in production; weaker than structural removal | KG-6 stays valid under the flag |
| F. Structural guard: the fabricating branch runs only on nodes that own a private cache (`VirtualNode::new`), never on `new_with_cache` nodes | production shared-cache node fails closed, harness nodes unchanged | smallest test churn (tests use `VirtualNode::new`, except `known_gaps.rs:113-117` which do not send GetRequest); impossible to enable by config; keeps KG-6 test valid | leaves the stub in the codebase; relies on a constructor distinction | peer-fill removes the stub later; KG-6 retargeted to "stub confined to private-cache nodes" |

Recommended: F combined with A' semantics: on a node whose cache is the serving cache (`new_with_cache` / `set_cache`), `GetRequest` returns an explicit unsupported error without any cache access; the legacy stub stays only for private-cache harness nodes and is deleted when the fill protocol (S2.2) replaces it. Reasoning: it removes the production exposure with the least test and wire churn, cannot be re-enabled by configuration, does not extend the stub into peer-fill, does not weaken KG-6 (the pinned test still passes because it uses a private-cache node, and a second pin for the shared-cache refusal is added), and does not presuppose that deleting the handler is the final architecture. This is a recommendation, not an approved change; I did not test that the churn estimate holds beyond reading the listed test files.

Review required before landing: application-code change to `src/dist` (owner approval per `CLAUDE.md` section 2); independent security review because it touches the cluster protocol; proposed tests 1, 2, 3, 5 and 7 must fail without the change; confirm `m7_sec_round3_tests.rs:331` (key guard) still passes; confirm no test relies on `HIT val_node_` on a shared-cache node; run the full `cargo test --all-targets -j 1` and clippy.

## 11. Relation to the three-pillar package and peer-fill

- `three_pillar/01_distributed_systems.md` row 21 (line 61) already classes the handler PROTOTYPE ONLY and unsafe to extend, and S2.2 (line 375) plans to replace it with `FillRequest/Response`. This investigation agrees and adds the measured result that the stored entry is not HTTP-visible today. Open question 3 (line 425) assumed the stub stays until S2.2; this report argues for confining it earlier (Option F) because the cost is small and the exposure is a standing hazard.
- `03_security_tenancy_resources.md` line 308 marks "read cached data via peer GET" as becoming critical if peer-fill is built; lines 234 and 508 already require per-tenant authorization on the wire, quota charged on the storing node and per-member fill rate limits. The current handler has none of these (section 3), which is why it must not be the seed of peer-fill.
- `discrepancy_register.md` DR-029 (line 45): recommend amending the "Consequence" cell from "plant a bogus 200 body for any key in the gateway's real cache" to "plant fabricated entries in the non-`/` `__default__` namespace of the shared cache; not HTTP-visible in the configurations tested", severity reassessed as in section 1, and the verification column to "Verified (handler, namespace separation, resource effect) by lead and by a second reader; independent review pending". DR-027 (any member can flush every cache) is the larger authorization gap and bounds this finding's marginal severity.
- KG-6: not removed or rewritten here. Under Option F the pin test stays valid; under Option A or A' it must be replaced, which is an owner decision.

## 12. Evidence summary

| Claim | Class |
|---|---|
| Handler stores fabricated `val_node_<id>`, 60 s, no etag, on Initiate (`node.rs:446-458`) | IMPLEMENTED; TESTED (KG-6 pin and others); MEASURED (R1-R3) |
| Local node shares the HTTP cache in the mTLS path (`main.rs:332`) | IMPLEMENTED; MEASURED (harness mirrors it) |
| Handler reachable only via mTLS port in production wiring; insecure branch starts no listener | IMPLEMENTED |
| Auth chain: client cert, one SAN, registry, fingerprint, per-frame revalidation; no GetRequest-level authorization or rate limit | IMPLEMENTED; admission TESTED by phase-3 tests (not re-run) |
| Cluster keys cannot collide with HTTP keys (leading `/` rule on both sides) | IMPLEMENTED; TESTED for the cluster guard (`m7_sec_round3_tests.rs:331`); MEASURED for five HTTP spellings |
| HTTP clients can be served the fabricated value | NOT supported by code or measurement; no test in the repo covers it |
| Recovering/Suspect do not store; Joining/Degraded/Active store | IMPLEMENTED; MEASURED |
| Fencing and byte accounting hold for planted entries | IMPLEMENTED; MEASURED (`audit_bytes` equal) |
| A member can evict HTTP entries by flooding | MEASURED mechanism in a 64 KiB-budget debug harness; production impact UNVERIFIED |
| Counter pollution | IMPLEMENTED, not measured |
| Severity (Low to Medium) and CVSS-like reasoning | analysis, UNVERIFIED by anyone but me |
| The finding as a whole | not INDEPENDENTLY VERIFIED: lead's reading plus my reading plus my own reproduction; independent review pending |
| Real-deployment exposure, release-profile behaviour, other readers of non-`/` keys | UNVERIFIED |

## 13. Unknown and owner decisions required

Unknown: production eviction policy and budget effects; whether the local shared node can enter Suspect/Recovering; whether lab or other APIs build keys without `norm_path`; whether any operator binds the cluster port outside the trust domain; whether any external tooling sends `GetRequest`.

Owner decisions: (1) approve a src change in `src/dist/node.rs` (Option F with A' semantics recommended, or A / A'); (2) whether to keep, retarget or delete the KG-6 pin and migrate the six test files' probe; (3) accept the amended DR-029 wording and severity; (4) whether to schedule DR-027 per-scope authorization together with this fix; (5) independent security review assignment; (6) whether the stub may remain on private-cache harness nodes until S2.2.
