# V3: Demonstration topology, tenancy and identity model

Agent V3 of 5 (product discovery). Date: 2026-10-30. Status: RESEARCH ONLY. Nothing here is implemented, approved or run. No source, UI, Cargo, git state or network was touched; no experiment was executed. HEAD `456bb25`, branch `feat/publish-phantomcache-core`, large uncommitted tree (72 status lines at start).

Evidence tags used: **[code]** verified by reading `file:line` in this tree; **[doc]** stated in a repo document, not re-verified in code; **[prior]** reported by an earlier agent/handoff; **[assumption]**; **[hypothesis]**; **[external]** general platform knowledge not checked against a primary source here (treat as unverified).

Capability classes: **Existing**, **Partial** (partially implemented), **Proposed** (extension needing new code), **Hypothesis** (research hypothesis), **Prototype** (candidate that can be tried with no src change), **Unsupported** (cannot be done honestly with the current design).

## 0. The three axes (kept separate throughout)

| Axis | Meaning in this document | Where it lives in code today |
|---|---|---|
| **Client** | A requester: a device, browser tab, process or curl. It holds a credential and sends HTTP. | Nowhere as an entity. The gateway sees only a bearer token (and, in non-secure mode, a client-asserted header). There is no client registry, no per-client counter. [code] `src/api/server.rs:985-1044` |
| **Tenant** | A namespace string in the cache key (`tenant_id`), taken from the JWT claim in secure mode. | `CacheKey.tenant_id` `src/core/cache.rs:22-36`; claim `src/security/jwt.rs:39`; derivation `src/api/server.rs:1209-1219` |
| **Cache node** | A gateway process and its in-memory `PartitionedCache`; in a cluster, a member with a u8 id, an mTLS identity and a rendezvous-hash role. | `src/main.rs:204`, `src/dist/node.rs:20-35`, `src/security/cluster.rs:301-317` |

Cardinalities that follow from the code: many clients may share one tenant; one client may hold tokens for several tenants (one token = one tenant claim); one tenant is served by every node that its clients reach, and each node holds its own independent copy (GET is local-only). Any UI or demo text that says "client A's node", "tenant A's cache" or "A's data" without saying which axis is meant is wrong by construction.

## 1. What exists today (verified)

### 1.1 Tenancy and namespacing

1. **Key construction.** `CacheKey { tenant_id, path, query }`; absent tenant becomes the literal `__default__` (`src/core/cache.rs:30-36`). Tenant ids are 1..=128 bytes of `[A-Za-z0-9._-]` (`:38-51`), enforced so that the first `:` of the canonical string `tenant:path[:?query]` is the tenant separator (`:96-116`). Path rejects `.`/`..` segments, control bytes and `:?` (`:60-82`). [code]
2. **Tenant derivation.**
   - Secure mode (JWT validator configured and `require_auth_for_control`): the tenant is the verified token's `tenant_id`; a missing/empty claim is 403 `MissingTenant`, the claim `__default__` is 403 `ReservedTenant`; `x-tenant-id` from the caller is ignored (`src/api/server.rs:1046-1068`, `:1148-1219`, DELETE `:1952-1974`). [code]
   - Non-secure mode (no JWT): the tenant is the client-asserted `x-tenant-id` header, validated only for alphabet/length (`:1126-1134`, `:1215`). Any client can name any tenant. This is a development convenience, not isolation. [code]
   - Upstream: the tenant is forwarded to the origin as `x-tenant-id` taken from the key, never from the client header (`:2649-2679`). So an origin can be tenant-aware (this is the only legitimate way to give two tenants different origin behaviour). [code]
3. **Cross-tenant isolation test.** `test_m1_t18_cross_tenant_isolation` (`tests/integration/m1_http_tests.rs:809`) uses the header (non-secure) path; `m7_security_review_tests.rs:422` covers "tenant comes from the token, never from x-tenant-id". Whether a secure-mode two-token isolation test exists for GET data bleed was not enumerated. [code, partial]
4. **Cache-key confusion surface (what a tenant can and cannot do).**
   - Cannot address another tenant's key in secure mode: tenant is not caller-controlled, `:` is excluded from tenant ids, `:?` from paths. Round-trip is pinned by `canonical_key_roundtrip_tests`. [code + doc]
   - Can still collide with *itself* across query variants (each query string is its own key; `limitations.md` says DELETE per variant). [doc]
   - Shared-secret consequence: any holder of `PHANTOM_JWT_SECRET` can mint any tenant (and any role). Tenant identity is exactly as strong as secret custody. [code] `src/security/jwt.rs:135-139`, `src/main.rs:411-414`
   - Role aliasing: `requester` grants both `cache:read` and `cache:invalidate`; `read`, `invalidate`, `observer`, `operator` are accepted aliases; `admin` satisfies everything (`jwt.rs:57-84`). [code]
5. **Per-tenant quotas: NOT PRESENT.** Capacity is global: `max_entries_per_shard` default 10,000 (`cache.rs:330`, `set_capacity_limits` `:512`), byte budget global (`src/main.rs:205`, `PHANTOM_MAX_CACHE_BYTES` `src/config.rs:114`), 8 shards chosen by hash of the whole key including tenant (`cache.rs:544-550`). No tenant-aware code in `src/core/eviction.rs` (0 occurrences of "tenant"), none in telemetry or the intelligence modules (0 occurrences); `coalesce.rs` has 4 occurrences (not read in depth). A grep for `quota|per_tenant|fair share|rate_limit` over `src/` found nothing relevant (only router per-peer repair bounds, `router.rs:59`). **Consequence: one tenant can evict another tenant's entries and consume its share of the byte budget and origin concurrency cap; there is no per-tenant admission, rate limit or origin-budget.** The origin concurrency cap is gateway-global (`PHANTOM_ORIGIN_CAP_*`, `config.rs:163-169`). [code]
6. **Tenant-scoped invalidation exists and is the only per-tenant control.** `try_invalidate_tenant` bumps a per-tenant epoch; at `MAX_TENANT_EPOCHS = 10,000` distinct tenants it falls back to a global epoch bump that fences *every* tenant (`cache.rs:273`, `:576-607`). Tenant-scope frames can also arrive from peers (`router.rs:118`). So a tenant-epoch exhaustion is a cross-tenant blast radius (bounded, correctness-safe, but it is a mass flush). Not reachable by a secure-mode client who cannot choose tenant ids. [code]
7. **Telemetry is not tenant- or client-attributed.** `/api/events` (SSE), `/api/v1/metrics`, diagnostics buffer: aggregate counters and key fingerprints (`cache.diagnostics.hasher.fingerprint`, `server.rs:2085-2088`). No tenant, client or node dimension on request counters. The SSE stream is a 100 ms interval snapshot, not an event log (`server.rs:959-968`). [code]

**Verdict: can two tenants coexist in one gateway?** Yes, as separate key namespaces with correct data isolation in secure mode [code]. **With quotas/isolation of resources?** No: namespace isolation only; there is no resource isolation (noisy-neighbour, eviction, byte budget, origin cap, freshness policy, per-tenant metrics). Any demo that shows "tenant A is protected from tenant B's flood" would show something the product does not do. Class: namespace isolation **Existing**; resource isolation/quotas **Unsupported today, Proposed**; per-tenant observability **Proposed**.

### 1.2 Origins

- One origin per gateway: `ORIGIN_URL` single URL (`src/main.rs:253`, `src/config.rs:125`); `PHANTOM_ORIGIN_ALLOWLIST` is an SSRF allowlist, not a routing table (`main.rs:400-409`). No host/path-prefix routing, no per-tenant origin. [code]
- LAN constraint: when the bind is non-loopback, `allow_loopback_origin = false` (`main.rs:406`), so a gateway bound to `0.0.0.0` will not fetch from a loopback origin; a LAN origin needs `PHANTOM_ALLOW_LAN_ORIGIN=true` (`config.rs:133`, SSRF relaxation marked "high"). Network-bind plus insecure upstream requires an allowlist (`security/mod.rs:135-144`). [code]
- "Multiple origins" can therefore only be shown as multiple gateways, each with its own `ORIGIN_URL`, or as one origin that branches on `x-tenant-id`. Class: multi-origin per gateway **Unsupported**; origin-side tenant branching **Existing** (origin must be written for it); gateway-per-origin **Existing** (separate processes).

### 1.3 Nodes, membership, identity

- **Standalone gateway** has three placeholder `VirtualNode`s (ids 1-3, each with its own throw-away 4-shard cache) that do not serve traffic (`main.rs:211-218`, `dist/node.rs:38-59`). UI/telemetry label them `simulated`, and list them only in simulation mode (`server.rs:660-681`). [code]
- **mTLS cluster mode** (experimental): activated only when `PHANTOM_CLUSTER_CA`, `PHANTOM_NODE_CERT`, `PHANTOM_NODE_KEY` and `PHANTOM_CLUSTER_MEMBERS_FILE` are all set (`main.rs:281-289`). The local node shares the gateway's HTTP serving cache (`:332`); remote peers are address-only descriptors (`:364-367`). Node id is a u8 1..=255 (`main.rs:298`, `config.rs:149`). [code]
- **Identity (node):** TLS 1.3 only, client-cert mandatory, trust anchored to the cluster CA only (`cluster.rs:591-637`); SAN `node-{id}.{cluster_id}.phantom.internal` strictly parsed (`:65-147`); leaf SHA-256 fingerprint must be in the members file and `active` (`:364-402`). Membership is a versioned JSON file read at startup; the comment says updates need a process restart (`cluster.rs:556`); `ClusterMembershipRegistry::update` and `SessionTracker` revocation exist (`cluster.rs:405-440`, `dist/session.rs`) but a grep found no runtime caller of `update`. [code]
- **Join/leave is static configuration.** There is no join protocol, no gossip, no dynamic discovery. Adding a node = issue a cert with the right SAN, add its fingerprint and address to the members file on every node, restart them. `NodeState::Joining` exists as an enum value only (`node.rs:10-18`). Class: dynamic join/leave **Unsupported**; static N-node cluster **Partial** (experimental; the claims ledger records 3-node maximum exercised, single-host loopback, C6 "Weak"). [code + doc]
- **PKI tooling:** cluster PKI generation exists only in the test harness using the dev-dependency `rcgen` (`tests/support/cluster_harness.rs:198`, `Cargo.toml` dev-deps); `scripts/gen-demo-certs.ps1` produces only an ingress CA and server cert and only if `openssl` is on PATH, otherwise it prints a message and does nothing (`scripts/gen-demo-certs.ps1:20-47`). No shipped command creates node certs/fingerprints/members file. Class: cluster PKI for a hand-run demo **Proposed** (script or bin); ingress server cert **Partial**.

### 1.4 What a node failure actually does today

| Event | Behaviour | Tag |
|---|---|---|
| Standalone gateway process dies | Its clients get connection refused. No client-side or server-side failover exists; no load balancer in the product. | [code: GET never touches the router; router appears only in DELETE `server.rs:1992`] |
| Peer gateway dies (cluster) | Survivors keep serving their own local caches; **reads are unaffected**. A DELETE whose canonical primary is unreachable marks it `Suspect`, tries the first replica as acting primary within 150 ms legs, records owed invalidations for every other member in the in-memory `RepairLedger`, and the repair worker re-delivers after the peer returns (`router.rs:1790-1886`, `:1902`; bounds `REPAIR_MAX_ENTRIES_PER_PEER=1024` etc. `:54-60`). A recovered peer goes Suspect -> Recovering -> Active only via pull reconciliation. | [code] |
| Issuer gateway dies holding unacknowledged invalidations | Those invalidations are lost; the peer serves the stale entry until its freshness lifetime ends. Reproduced on the current tree, loopback, debug build, 3 gateways with fault proxies: peer stale for the whole 10 s window in 4/4 original/flag-off runs. A 204 is not proof of cluster-wide removal. | [reproduced by prior agent, `docs/research/m3_durability/reproduction_report.md` s1-3; `docs/user/limitations.md`]. Proposed fix (durable log, drain) is unintegrated scratch only. |
| `POST /api/control {"event":"NODE_FAILURE"}` (Operator) | Sets node id 2 to `Failed` in `state.nodes` (`server.rs:1089-1092`). In standalone mode this flips a placeholder. In cluster mode `state.nodes` and the router share the same `Arc<VirtualNode>` objects (`main.rs:371-374`, `:430`), so it changes routing/fan-out decisions without any process or network fault. It is a **logical** failure flag, not a failure. Any other event value resets all nodes to Active (`:1093-1099`). | [code] |
| Network partition | Not implemented as a feature. Only the test harness `FaultProxy` (modes Normal/Delay/Blackhole/Partition/DropNewConnections, `tests/support/fault_proxy.rs:21-27`) can inject it, in-process, in `cargo test`. | [code] |

### 1.5 Roles, tokens, TLS, CORS

- Roles: `cache:read`, `cache:invalidate`, `telemetry:read` (Observer), `scenario:operate` (Operator), `admin` (`jwt.rs:4-16`). Claims: `sub, iss, aud, exp, iat?, tenant_id?, roles[]` (`:31-42`). HS256 only; optional `kid` selects a rotation secret, but `main.rs:414` builds the validator with `JwtValidator::new` only, so rotation keys are unwired in the binary [code]; leeway 10 s (`:171`). No `jti`, no revocation list, no maximum-lifetime check: `exp` is whatever the minter wrote. `limitations.md`: token revocation not provided [doc].
- No token issuance tool ships: `create_token` exists only in the library/tests (`jwt.rs:135`; a grep of `docs/user`, `packaging`, `README.md` for token minting found nothing). Operators must mint HS256 JWTs externally. [code + grep]
- Endpoint authorization: GET/DELETE cache need `cache:read` / `cache:invalidate` plus a tenant claim (`server.rs:1153`, `:1955`); `/api/events`, metrics, diagnostics, intelligence need Observer (`:945-951`, `:337-344`); `/api/control` needs Operator (`:1077`); Lab API reads Observer, start/cancel Operator (`lab_api.rs:1178`, `:1680`, `:1936`). The `/api/events` stream is **not closed at token expiry** (no `exp` check in the loop `server.rs:959-968`); the Lab run stream is (`lab_api.rs:2015-2017`, `:2043`). [code]
- Fail-closed startup: non-loopback bind requires a JWT secret and TLS cert/key, or `PHANTOM_ALLOW_INSECURE_HTTP=true` (auth still enforced) (`security/mod.rs:108-131`, `server.rs:263-270`, `:305-311`). Lab API without JWT must be loopback (`lab_api.rs:97-100`). Cluster over a non-loopback bind requires full mTLS material (`cluster.rs:664-688`). [code]
- Ingress TLS: PEM cert/key files, TLS via `axum_server::bind_rustls` (`server.rs:291-299`). No hot reload [doc].
- CORS: exact-origin allowlist, no wildcard, no credentials; development profile allows `http://localhost:3000` and `http://127.0.0.1:3000` only (`config.rs:550`, `server.rs:113-178`). **A UI served from a LAN IP or another device is cross-origin to the gateway and will be blocked unless that exact origin is listed in `PHANTOM_CORS_ORIGINS` (max 16).** Allowed request headers include `x-tenant-id`, `last-event-id` (`server.rs:158-165`). [code]
- Browser streaming: `EventSource` cannot send `Authorization`; the repo chose fetch-streaming with a header (option D in `docs/design/sse_auth_options.md`); the UI reads a pasted token and sends it only in the header (`ui/src/lib/labApi.js:177-178`, `labStream.js`). Query-token and cookie options are documented as rejected/owner-reserved. [code + doc]
- UI gateway selection: default `http://localhost:8085` (`ui/src/lib/config.js:1`), overridable by `VITE_PHANTOM_GATEWAY` or a URL parameter via `resolveGateway` (`ui/src/App.jsx:54-58`; validation rules not read). The Cluster and Intelligence screens are placeholders (`App.jsx:44-45`). [code]

### 1.6 Closest existing "session"-like objects

- Lab runs: a run id, `run.json` state machine, `events.jsonl` append log with monotone event ids, SSE with `Last-Event-ID` resume, max 16 streams, artifact allowlist, manifests with seed sets (`lab_api.rs:1-21`, `:51-59`, `:1025-1033`, `:2020-2045`). This is the nearest precedent for a Demo Session event log and replay. It is a *single-process, single-tenant-agnostic* service ("one process-wide service (no per-tenant isolation)", `docs/user/lab_api.md:65`). Operator-started runs spawn gateway child processes bound to the parent by a Windows job object (`lab_api.rs:219`, `:354`, `main.rs:451`; `lab_live/gateway.rs:72`). Therefore **an Operator token is effectively a remote "start processes on this host" capability** within the lab's bounded kinds; do not hand it to guest devices. [code]
- Prior demo docs: `docs/demo-lab/device-topologies.md` (Topologies A-F), `docs/research/demo_experience_plan.md`. Parts are stale: it says LAN binding is blocked by a hardcoded loopback bind, but `PHANTOM_BIND_ADDR` is now configurable (`config.rs:98`); its Topology E says the router is "disconnected from the HTTP gateway", which is now only true for GET (DELETE uses it, `server.rs:1992`). Its Topology D "Cross-Tenant Isolation" uses the header path (non-secure). Treat as design intent, [prior], not as verified status.

## 2. Single-machine vs multi-device modes

### 2.1 Single-machine mode (the always-available baseline)

Components as separate OS processes on one Windows host. All results from this mode must be labelled **loopback / single host** (CLAUDE.md s4) and shown as such in any UI.

| Element | How (existing config only) | Class |
|---|---|---|
| N gateways | Separate processes, each `PHANTOM_BIND_ADDR=127.0.0.1:<port>` (ports distinct). `lab_live/gateway.rs` already spawns real gateways on free ports (`free_port`, `gateway_env`, kill) | Existing (library/test path), Prototype for a launcher script |
| Origin(s) | One mock origin per gateway, `ORIGIN_URL=http://127.0.0.1:<p>`; `lab_live/origin.rs` is a scripted delay origin | Existing (lab), Prototype for the interactive demo |
| Clients | Browser profiles (distinct `localStorage`, so distinct pasted tokens), curl, `lab_live/loadgen.rs` | Existing |
| Tenants | Distinct `tenant_id` claims in tokens minted per client | Existing |
| Cluster of 3 | `PHANTOM_CLUSTER_*` env, certs generated by test harness only; run via `ClusterHarness`-like launcher | Partial; manual PKI = Proposed |
| Extra addresses | Loopback aliases `127.0.0.2..` as distinct "hosts" for the cluster listener/addresses | **Hypothesis** [external: 127.0.0.0/8 is generally bindable on Windows; not tested here]. Does not create independent network stacks, clocks or failure domains; label as co-located. |
| Containers/VMs (optional) | Not required; see 2.3 | Proposed |

Honest labelling: a 3-process loopback "cluster" demonstrates the protocol and the code path, not physical distribution, not LAN latency, not independent clocks. `device-topologies.md` s1 already states this rule [doc].

### 2.2 Multi-device LAN mode

Reference topology (N >= 4 devices; ids are illustrative):

```
Device 1  Control Center (presenter laptop): browser UI, Operator+Observer token; may also host the pairing service
Device 2  Client A: browser or curl, token(tenant=alpha, cache:read, cache:invalidate)
Device 3  Client B: token(tenant=beta, ...)         [same gateway as A -> two tenants, one node]
Device 4  Cache node 1: gateway, bind LAN IP, TLS, JWT; origin on Device 4 or 5
Device 5  Cache node 2 (optional): second gateway; mTLS cluster peer of node 1
Device 6+ Extra clients / observers (read-only Observer tokens)
```

The same axes in this layout: clients = devices 2, 3, 6+; tenants = alpha, beta (claims, independent of device); nodes = devices 4, 5. A single device can be several clients (several browser profiles) and one client can be pointed at either node. Which node a client talks to is a *manual choice of URL*, because there is no ingress router or load balancer; a demo must say so rather than imply the cluster "routes" the client.

Binding/requirements derived from code [code]:
- Non-loopback bind: JWT secret mandatory; TLS cert/key or `PHANTOM_ALLOW_INSECURE_HTTP=true` (lab-only, warns loudly).
- Gateway on `0.0.0.0` cannot use a loopback origin (`main.rs:406`); origin on a LAN address needs `PHANTOM_ALLOW_LAN_ORIGIN=true` and, with insecure upstream, an allowlist.
- Cluster over LAN needs full mTLS material and a members file; cluster listener address is separate from the HTTP port (`PHANTOM_CLUSTER_LISTEN_ADDR`).
- UI origin must be in `PHANTOM_CORS_ORIGINS`; max 16 entries, exact `scheme://host[:port]`.
- Windows host firewall (inbound TCP for the HTTP port, the cluster port, and the UI/pairing port) is an OS setting and a "system or global setting" under CLAUDE.md s2: **do not change it without owner approval**; prefer a documented manual step with the exact rule printed for the human. [external for exact rules]. Public-profile networks often block inbound by default [external; verify on the target network]. Guest Wi-Fi with client isolation will prevent device-to-device traffic entirely [external]; this is the most common field failure and must be on the preflight list.
- Time: tokens use wall-clock `exp` with 10 s leeway (`jwt.rs:171`); devices with drifting clocks will fail auth confusingly (401 `ExpiredToken` or a not-yet-valid case: `nbf` is not set, so only expiry matters). Pairing should hand out a server timestamp and the UI should show clock skew. [code + assumption]

Class: LAN single-gateway with N clients **Partial** (all pieces exist; no launcher/preflight/docs verified by an actual multi-device run: `docs/research/claims_ledger.md` says only Windows loopback has been exercised, `limitations.md` "Platforms"); LAN two-node cluster **Partial/experimental**; N>3 nodes **Hypothesis**; mixed OS devices **Unsupported** (never exercised).

### 2.3 Physical devices vs VMs vs containers

| Option | Gives | Does not give | Notes / class |
|---|---|---|---|
| Physical devices | Real NICs, switch jitter, independent clocks, real socket resets, real power-off | Reproducibility; control of faults without human action | Gold standard for "distributed" claims. Only Windows x86-64 binary exercised [doc]. Existing build path (`packaging/`). |
| VMs on one host (Hyper-V/VirtualBox/WSL2) | Separate network stacks, easy snapshot/reset, can drop packets at the virtual switch | Independent hardware; NAT/bridge quirks | CLAUDE.md s4: keep WSL2 results labelled separately from loopback and physical. Proposed; install/enable of Hyper-V is a system change needing approval. |
| Containers | Cheap isolation, `docker network disconnect` partition, reproducible images | Limitations list: "no container image" shipped; Windows containers/Docker not present on this machine as far as verified (not checked) | **Proposed**; requires new Dockerfile (not a dependency), and Docker installation is a software install needing approval. |
| Raspberry-class/other OS | Variety | Linux/ARM never built or tested [doc] | **Unsupported** until built and tested. |

Recommendation: design for physical + single-machine first; treat VMs/containers as later, labelled "virtualised network, single physical host".

## 3. Discovery and pairing

### 3.1 Requirements
Temporary scoped credentials; QR or short code; expiry; revocation; no shared password as the primary design; works without internet; fails closed; every credential names a tenant, role set and session.

### 3.2 What the existing JWT stack can do with no new dependency

| Need | Existing support | Gap |
|---|---|---|
| Scoped token (tenant, roles, expiry) | Yes: `tenant_id`, `roles[]`, `exp` validated per request on cache/control/metrics routes (`jwt.rs`, `server.rs:985-1044`) | Nothing mints tokens in the product; need a minter (script or bin). HMAC-SHA256 is available in PowerShell/.NET and Node built-ins; Rust `jsonwebtoken` is already a runtime dependency. **Prototype** with an external script (no dependency, no src change); **Proposed** as a `src/bin` tool (src change, no new crate). |
| Short TTL | Any `exp` accepted | Gateway enforces no maximum lifetime; the minter must be disciplined. Gateway-side `PHANTOM_JWT_MAX_TTL` would be **Proposed** (auth is owner-reserved). |
| Per-session scoping | `iss`/`aud` are global per gateway process, set by `PHANTOM_JWT_ISSUER/AUDIENCE` (`config.rs:110-113`, `main.rs:411-414`). A fresh random secret and `aud=phantom-demo-<session>` per session means tokens from another session or an old one are rejected | Costs a gateway restart per session (cache is in-memory, empty on restart: acceptable and desirable for a demo). **Prototype.** |
| Per-node scoping | Give each node a different `aud` (and same or different secret); a token for node 1 is rejected at node 2 | Makes "client bound to node" enforced rather than conventional. **Prototype.** Costs: cluster frames use mTLS not JWT, so unaffected. |
| Revocation | None. Options inside the current design: expire quickly; restart the gateway with a new secret (kills all tokens and the cache); `kid` rotation exists in `JwtValidator` but is unwired in `main.rs:414` | Fine-grained revocation (kick one device) needs a `jti`/`sub` denylist checked in `authenticate_request`: **Proposed**, touches owner-reserved authentication code, needs a test that fails without it. Interim: do not overload `tenant_id` for revocation (tenants are a data axis, not a device axis). Use short TTL (e.g. 5-15 min, renewed through the pairing service). |
| Open streams after expiry/revocation | `/api/events` keeps streaming after `exp` (no check) | **Proposed**: close at `exp` as the Lab stream already does (`lab_api.rs:2043`). Until then a paired observer remains connected after expiry; mitigated only by tearing down the session. |

Principle: the **JWT secret never leaves the host that mints tokens**; devices receive tokens only. A device that obtains the secret (for example through a "shared password" design) can mint admin tokens for any tenant. This is the reason not to make a shared secret/password the pairing primitive.

### 3.3 Pairing flows (all **Proposed**; they need a new component)

The pairing component ("pairing service") holds the session secret, mints tokens, keeps the roster, and is separate from the gateway so that the cache hot path and owner-reserved auth code are untouched. It could be (a) a small Node script reusing the toolchain already required for the UI (`ui/package.json` shows Vite/React, scripts run by `node --test`, so Node is a present prerequisite; no new npm dependency if only `node:http`, `node:crypto` are used), (b) a PowerShell script, or (c) a new `src/bin` Rust tool (src change, frozen-file implications, no new crate). **Prototype** class for (a)/(b); **Proposed** for (c).

Flow P1: short code (no new dependency).
1. Presenter creates a Demo Session in Control Center; pairing service prints a 6-8 character code from an unambiguous alphabet, valid for e.g. 2 minutes, single use, bound to (tenant, role set, device label).
2. Guest device opens `http://<presenter-ip>:<port>/pair` (or types the URL shown on the presenter screen), enters the code.
3. Service verifies code (constant-time compare, attempt limit, global rate limit), mints a short-lived JWT, returns it once. Code is burned.
4. Device stores the token in memory/session storage only (the UI never persists or logs it today: `labToken.js` header comment).

Flow P2: QR. Same as P1 but the code/URL is rendered as a QR for phones. The repo has no QR encoder and no QR dependency (`Cargo.toml`, `ui/package.json`). Options: new dependency (needs the CLAUDE.md s2 justification), hand-written minimal QR (versions 1-6, byte mode, ~200-300 lines, **Prototype**), or no QR (the short code and URL are enough; QR is convenience). **Hypothesis:** QR adds demo polish but zero security; do not build before the code flow works.

Flow P3: pre-provisioned token by hand (paste). What the UI supports today (`labApi.js`): the operator pastes a token into the connection panel. Existing, acceptable for the presenter device; poor for guests.

Security properties to require of any pairing design (each testable):
- Code entropy and attempt limits sized so online guessing within the TTL is negligible (state the math in the design, e.g. 30-symbol alphabet x 8 chars with 5 attempts per code and a global limit).
- Pairing endpoint itself must be served over TLS on LAN, otherwise the minted token crosses the network in clear (an on-path device can steal it). Token theft = full tenant impersonation until `exp`.
- Pairing never grants Operator or admin to a guest device; Operator is bound to the presenter device by a separate code path (e.g. pairing from the presenter's own loopback only).
- All issuance, redemption, failure, expiry, teardown events recorded in the Demo Session event log with device label and truncated fingerprint of the token (never the token).

### 3.4 TLS on LAN

| Concern | Existing | Gap / options |
|---|---|---|
| Server cert | PEM files via `PHANTOM_TLS_CERT/KEY`; TLS 1.3 per rustls defaults; `scripts/gen-demo-certs.ps1` makes a CA plus a server cert **only if `openssl` is on PATH** (RSA-2048, 90-day server cert, SANs: given host + localhost + IP) | No shipped generator without openssl; the script's message points to an `rcgen` generation that is only in tests. **Proposed:** a cert-generation helper (bin) or documented openssl step. |
| Trust bootstrap for browsers/phones | None | Each device must trust the demo CA or accept a warning. Options: (1) install the demo root CA on each device (requires the user to approve a system trust change on each device; expiry and removal at teardown must be explicit; **risk:** a trusted root that can sign for any site on that device, so use name constraints and delete after the demo [external]); (2) per-session self-signed cert + certificate pinning by the pairing flow (the device downloads the cert fingerprint out-of-band from the QR/URL, e.g. fingerprint shown on the presenter screen and compared by a human); (3) use plain HTTP with `PHANTOM_ALLOW_INSECURE_HTTP=true` on an isolated, presenter-controlled network: allowed by the product only with JWT and loudly marked "lab only"; tokens are then sniffable on the LAN. Recommended default: (3) only on a private presenter-owned hotspot/router with a stated risk banner; (1) or (2) otherwise. |
| Node mTLS | Strong: CA-only trust, SAN + fingerprint + epoch, revocation event plumbing (`cluster.rs`, `session.rs`) | Cert issuance/fingerprint capture/members file are manual; no tooling. **Proposed:** "cluster PKI generator" producing CA, N node certs, members file with fingerprints and addresses. Because membership is read only at startup, adding or revoking a node during a demo means restarting nodes; present it as such. |
| Pairing/UI page TLS | n/a | Same issue as ingress. |
| Browser quirks | CORS allowlist exact origin | Cross-device UI means origin = `http(s)://<host-ip>:<port>`; must be in the list; max 16. |

### 3.5 Remote access (off-LAN)

All **Proposed / not recommended by default**, listed with risk: (a) router port-forward: exposes an experimental gateway to the internet; the product's own warnings (insecure-HTTP banner, `limitations.md`) apply; **Unsupported**. (b) A tunnelling service: introduces a third party that sees tokens/TLS termination, requires installing software; owner approval; labelled "remote, via third party". (c) VPN/overlay network (WireGuard-style) that makes remote devices look LAN-local: best security, but install and key distribution; label the added latency. (d) Screen-share the Control Center only: no remote device participates, so nothing distributed is claimed. Recommendation: remote observers via (d) or a read-only exported session report; remote *participants* only via (c), later.

## 4. Role model

Mapping demo roles onto the existing five token roles plus node identity. "Role" below is a capability bundle; it is orthogonal to tenant.

| Demo role | Token (roles; `tenant_id`) | Can do | Must not be able to | Class |
|---|---|---|---|---|
| Presenter / operator | `scenario:operate` + `telemetry:read`; no tenant needed for control routes (a tenant is required only for cache read/DELETE) | `/api/control` events, start/cancel Lab runs, see all telemetry | Be issued to guests; Operator reaches process-spawning code (s1.6) | Existing roles, Prototype issuance |
| Observer (audience) | `telemetry:read` | `/api/events`, metrics, diagnostics, intelligence status/audit, Lab reads | Read cache bodies (needs `cache:read`+tenant), invalidate, control | Existing |
| Tenant client | `cache:read` (+ `cache:invalidate` if the demo shows DELETE); `tenant_id` mandatory; avoid the `requester` alias (grants both) unless intended | GET/DELETE within its tenant namespace on any gateway that accepts the token | See or change another tenant; read telemetry (unless also given Observer) | Existing |
| Cache node | mTLS cert (SAN `node-N.<cluster>.phantom.internal`, fingerprint in members file); **not** a JWT principal | Send/accept invalidation frames, anti-entropy, pull reconciliation | Be admitted without membership entry; frames from other clusters | Existing (experimental) |
| Admin | `admin` | Everything (aliases all roles) | Never used in demos; mention only as the thing not to hand out | Existing |

Observations:
- Role bundles are free-form strings in `roles[]`; a typo (`"operater"`) yields no permission, not an error. A minter must validate against the allowlist. [code: `has_role` returns false for unknown strings]
- Observer and tenant client are separate: an Observer cannot read cache data and a tenant client cannot see telemetry. For a client-facing "your own hits/misses" view, a client needs a *tenant-scoped telemetry* endpoint that does not exist (s1.1 item 7). Class: **Proposed**. Without it the client device can only show what it measured itself (response headers `X-Cache-Status`, `Age`) which is honest and **Existing**: label "measured by this client".
- Token claim recommendations (all through existing claims, no schema change): `sub` = stable device label (`clientA-laptop`), `aud` = per-session/per-node string, `iss` = fixed, `exp` = minutes, `tenant_id` = tenant for clients only, `roles` = bundle above. Extra session facts (session id, device id) cannot ride in the token as first-class claims because `JwtClaims` ignores unknown fields on deserialisation (not verified whether it denies them; `serde` default ignores) and the server cannot act on them; put them in `sub`/`aud` or in the pairing service's roster. Adding `sid`/`jti` is **Proposed** with the denylist.

## 5. Live event streaming and the Demo Session object

### 5.1 Streaming requirements (derived from what exists)

| Requirement | Existing | Gap |
|---|---|---|
| Push stream of telemetry | `/api/events?schema=2` 100 ms snapshot, fields labelled `measured/simulated/inferred/unavailable`, node `state_src: inferred` in cluster (`server.rs:659-681`) | Snapshot not event log: a viewer cannot reconstruct discrete events (a DELETE from client B at t) |
| Resume after disconnect | Lab run events: monotone `id`, `Last-Event-ID`, `events.jsonl` replay (`lab_api.rs:2020-2045`) | Not available on `/api/events` |
| Attribution (client, tenant, node) | None | **Proposed**: tenant-labelled counters and a bounded structured event stream (request outcome class, tenant, node id, key fingerprint, no key text). Touches telemetry and the read path; the read path has a "zero contention" invariant (CLAUDE.md s5) so any added per-request work needs a benchmark under the project's own evidence rules. |
| Bounded fan-out | Lab stream cap 16 (`MAX_SSE_STREAMS`); `/api/events` has no stream cap that I verified | **Proposed** for LAN with several observers |
| Auth in browser | fetch-stream with header (option D) | `EventSource` simplicity needs owner decision on option C (ticket) |
| Multi-node view | Each gateway streams only itself; the UI cluster screen is a placeholder | **Proposed**: the Control Center connects to every node and merges, labelling each stream's node id and time offset (`useTelemetry` already tracks `offsetMs` per gateway, `App.jsx:56-61`). Cross-device clock skew must be shown, not hidden. |
| Time base | Each node uses its own wall clock | No synchronisation; an event ordering across devices is not meaningful beyond skew bounds. State this on the timeline. |

### 5.2 Demo Session object (Proposed schema; a data contract, not code)

```json
{
  "schema": "phantom.demo-session/1",
  "session_id": "ds-<random128>",
  "created_at_ms": 0, "expires_at_ms": 0, "state": "draft|armed|running|ended|torn_down",
  "mode_label": "single_host_loopback | lan_multi_device | vm_virtual_network",
  "config": {
    "hash_sha256": "<hash of the resolved configuration of every node (config.rs report() order), with secrets redacted>",
    "build": {"version": "...", "commit": "...", "built": "..."},
    "per_node_overrides_allowed": ["bind addr", "ports", "origin url"]
  },
  "seed": {"workload_seed": 0, "scenario_seed": 0, "note": "sealed M6 seeds are never used here"},
  "topology": {
    "nodes": [{"node_id": 1, "host_label": "dev4", "addr": "ip:port", "role": "gateway|gateway+cluster-member", "cluster_id": "...", "cert_fingerprint": "..."}],
    "origins": [{"origin_id": "o1", "attached_to_node": 1, "url": "...", "kind": "mock|real", "scripted_faults": []}],
    "links": [{"from": "node1", "to": "node2", "kind": "mtls-cluster", "fault_layer": "none|app-proxy|os"}]
  },
  "tenants": [{"tenant_id": "alpha", "intended_quota": "none (unsupported)", "nodes": [1,2]}],
  "devices": [{"device_id": "d2", "label": "Client A", "role": "tenant_client", "tenant_id": "alpha",
               "attached_node": 1, "paired_at_ms": 0, "token_exp_ms": 0, "revoked": false,
               "clock_offset_ms": null, "os": "reported by the device, unverified"}],
  "labels": {"distributed": ["..."], "simulated": ["..."], "visualised_only": ["..."]},
  "event_log": {"path": "events.jsonl", "next_id": 0, "sha256_at_end": null},
  "runs": ["<lab run ids referenced, with their own manifests>"],
  "limits": {"max_devices": 16, "max_streams": 16, "max_ttl_s": 900}
}
```

Rules: the session is **evidence-bearing**: event log append-only with monotone ids (same discipline as `RunHandle`), a manifest hash at end, secrets and token values never stored, device-reported values labelled "reported". Reproduction means re-running the scenario with the same seed/config hash on the same topology class, not replaying wall-clock events. Class: **Proposed** (new component; re-use the Lab run state-machine pattern).

## 6. Honest labelling: distributed vs simulated vs visualised

Every UI element and spoken claim in a demo must fall into one of these columns. This is the product rule "never present synthetic data as live" applied to topology.

| Item | Genuinely distributed (real separate processes/hosts, real sockets) | Simulated | Visualised only (drawn from config/inference) |
|---|---|---|---|
| Two or more real gateways with real mTLS invalidation frames | Yes (loopback if one host: say so) | | |
| Fan-out, retry, repair ledger, anti-entropy | Yes in cluster mode (experimental; ledger is in memory only) | | |
| Standalone-gateway node list (ids 1-3) | | Yes: placeholders (`main.rs:211-218`; telemetry labels `simulated`) | |
| Cluster node `state` in the UI | | | **Inferred**: configured/flagged, not probed (`server.rs:663`) |
| `NODE_FAILURE` control event | | Logical flag (no process/network fault) | Drawn as a failure |
| Process kill / host power-off / cable pull | Real failure | | |
| Fault-proxy partition/delay (tests only) | Real TCP disruption on loopback, application-layer | | |
| Rendezvous placement ("key X lives on node 2") | | | Computed from the hash (`route_key`), and for GET there is **no** routing: GET is local-only, so a drawing of requests flowing to the "owner node" is wrong. Only DELETE uses the owner (`server.rs:1992`) |
| Tenants A/B "sharing a node" | Real namespace separation | | Quota/protection bars would be fiction (s1.1 item 5) |
| Client devices | Real if separate devices; browser profiles on one host are co-located clients | | |
| Hit/miss counters, bytes resident | Measured on that node | | Aggregated across nodes by the UI = derived, label it |
| Latency numbers on loopback | Measured (loopback) | | Do not compare with LAN/WAN claims |
| Cluster-wide "linearizable delete" | **Not claimed.** 204 = local purge + primary ack; others best-effort, lost if issuer dies (s1.4) | | |
| Intelligence/PIE decisions | Machinery only, not in serving path (claims ledger C8; `pie: None`, `main.rs:433`) | | Must not be drawn as controlling traffic |

Banner rule: a persistent header chip in Control Center showing `mode_label` (single host loopback / LAN N devices / VM) and "N real nodes, M simulated" computed from the session object, never typed by hand.

## 7. Reset and teardown safety

Principles (CLAUDE.md s1: never destroy work; s3 labelling): the demo tooling must only touch objects it created and recorded in the session object.

| Phase | Safe behaviour | Class |
|---|---|---|
| Reset between scenarios | Cache is in-memory; reset = DELETE of known keys, or tenant invalidation (`try_invalidate_tenant`), or restart of the demo gateways. There is no "flush all" HTTP endpoint that I verified (`try_invalidate_all` exists in the library, `cache.rs:613`; no route found). A restart is the reliable reset. Note a restart loses owed invalidations (known defect s1.4) so reset the whole cluster together, not one node. | Existing (restart), Proposed (admin reset route: auth-reserved) |
| Teardown | (1) mark session `ended`; (2) revoke by expiry: stop the pairing service, let tokens lapse (<= TTL), or stop gateways (the only true revocation today); (3) stop only PIDs started by the launcher (store PID + start time; verify image name before kill, as `lab_live/gateway.rs:178-186` does with `tasklist`/`kill -0`); (4) delete only files under a session directory created by the launcher; (5) remove firewall rules/trust anchors the launcher or human added (list them in the session object as "owed cleanup" and show a checklist; do not auto-modify system trust/firewall without approval). | Prototype |
| Process lifetime | Lab API binds spawned gateways to the parent with a Windows job object (kill on close) (`lab_api.rs:354`, `main.rs:451`); a launcher script should reuse a job object or equivalent so a crashed Control Center does not leave orphan gateways holding ports. Non-Windows unverified. | Partial |
| Data | Session logs and artifacts live under `lab-results/`-style directory (already untracked); guest-device data lives only in the device's tab. No client tokens on disk. | Prototype |
| Repo safety | The launcher must not run git, must not write under tracked source, and must refuse to run if the target directory already exists with unknown contents (never overwrite an unread file). | Proposed rule |
| Shared-demo hygiene | Ports and PIDs from a previous crashed session are detected on start and reported, not killed blindly. | Proposed |

## 8. Network partitions and node failure injection

### 8.1 Without privileged tools (no admin rights, no driver, no firewall edits)

| Method | What it does | Fidelity / caveat | Class |
|---|---|---|---|
| Kill gateway process (Stop-Process / TerminateProcess) | Real node crash; reproduces the lost-invalidation defect when timed | Closest to power loss available unprivileged; honest. | Existing (used by the M3 harness [prior]); Prototype launcher |
| Graceful stop (Ctrl-C/Ctrl-Break) | Exit without draining pending invalidations (`main.rs:509-514`) | On Windows signalling one process needs a process group / CTRL_BREAK (see M3 report s4) | Partial |
| Application-layer fault proxy between nodes (point each node's `PHANTOM_CLUSTER_PEER_ADDRS` at a proxy) | Delay, blackhole, partition, drop-new-connections per link and per direction, no privileges | Faults are at the TCP relay, not the NIC; asymmetric partitions possible by proxying one direction only. A production-grade proxy is **not** in `src`; today it is `tests/support/fault_proxy.rs` (test code). Promoting it to a demo tool = new code outside `tests/` (owner decision, no new dependency needed: tokio is present) | Prototype (copy/adapt) -> Proposed (supported tool) |
| Pause a process (suspend threads; Sysinternals-style or a PowerShell/.NET P-Invoke to `NtSuspendProcess`) | Node unresponsive but sockets stay open: models a GC pause/hang, different from crash | Unprivileged for own processes [external]; may need a tool install | Hypothesis |
| Origin faults | `lab_live/origin.rs` scripted latency/error schedules; mock origin on/off | Real separate process; this is origin failure, not node failure | Existing |
| `NODE_FAILURE` flag | Changes routing/fan-out state only | **Not a failure**: label "logical". Never present as proof of tolerance | Existing (misleading if unlabelled) |
| Unplug cable / disable Wi-Fi on a LAN device | Real | Manual; not reproducible; cannot be scripted by the product | Existing (physical) |

### 8.2 With privileged tools (needs admin rights and owner approval)

| Method | Effect | Risk | Class |
|---|---|---|---|
| Windows Firewall rules (block inbound/outbound for a program or port, e.g. via `netsh advfirewall` or `New-NetFirewallRule`) | Real packet filtering, stateful; directional | Persistent system change if not removed; leaves the host blocked; global setting | Hypothesis; owner approval + mandatory cleanup record |
| WFP-based or `clumsy`-style drivers, `pktmon`, `tc/netem` (Linux), Hyper-V virtual switch policies | Latency, loss, reorder, bandwidth limits | Driver install, system-wide impact | Hypothesis; VM/container variants isolate the blast radius better |
| Container network disconnect (`docker network disconnect`) | Clean partition per container | Needs Docker; none shipped | Proposed |
| VM snapshot/restore | Fast reset of an entire node | Needs hypervisor | Proposed |

Every injection must be a recorded session event (who, when, which link, mode, duration) and visible on the timeline, and the UI must say which layer injected it (application proxy vs OS vs physical). Expected outcomes must come from the actual system: with a partitioned peer, reads stay local and fast, DELETE legs to the primary time out at 150 ms and fail over, the peer is marked Suspect, owed invalidations queue (bounded), and after healing the repair worker plus pull reconciliation return the peer to Active, **unless the issuer died**. Showing "everything heals" without the failure case would mislead.

## 9. Capability classification (summary)

| # | Capability | Class | Evidence / blocker |
|---|---|---|---|
| 1 | Multiple tenants in one gateway, namespace isolation (secure mode) | Existing | `server.rs:1209-1219`, `cache.rs:30-51` |
| 2 | Non-secure header tenants (dev) | Existing; not isolation | `server.rs:1126-1134` |
| 3 | Per-tenant capacity quota / rate limit / fair eviction | Unsupported (Proposed) | s1.1 item 5; hot-path and eviction change |
| 4 | Per-tenant or per-client telemetry | Unsupported (Proposed) | s1.1 item 7 |
| 5 | Multiple origins per gateway | Unsupported | `main.rs:253` |
| 6 | One origin per gateway, several gateways | Existing | separate processes |
| 7 | Origin-side tenant behaviour via `x-tenant-id` | Existing | `server.rs:2677-2679` |
| 8 | Static N-node mTLS cluster | Partial / experimental | claims ledger C6 Weak; 3 nodes max exercised |
| 9 | Dynamic join/leave, discovery | Unsupported | members file read at start; no caller of `update` |
| 10 | Node failure survival of reads | Existing (trivially, reads are local) | GET never routed |
| 11 | Invalidation survival of issuer crash | Unsupported today; fix is Proposed (unintegrated scratch) | M3 reproduction |
| 12 | Client-visible failover / load balancing across nodes | Unsupported | no ingress router |
| 13 | JWT scoped tokens (tenant, role, exp, aud) | Existing | `jwt.rs` |
| 14 | Token minting for guests | Prototype (external script) / Proposed (bin) | no shipped tool |
| 15 | Short-code pairing service | Proposed (Prototype as external script) | new component |
| 16 | QR pairing | Proposed (needs encoder: new dependency or in-house) | no QR code present |
| 17 | Token revocation (fine-grained) | Unsupported today; Proposed denylist | `limitations.md` |
| 18 | Coarse revocation (restart with new secret; expiry) | Existing | |
| 19 | Per-session secret and audience | Prototype | config only |
| 20 | Ingress TLS from PEM | Existing | `server.rs:291-299` |
| 21 | Demo CA + server cert generation | Partial (openssl only) | `gen-demo-certs.ps1` |
| 22 | Cluster PKI generator | Proposed | test harness only |
| 23 | Cross-device UI (CORS) | Partial (config of exact origin) | `config.rs:550` |
| 24 | Browser SSE with auth | Partial (fetch stream; Lab stream) | `sse_auth_options.md` |
| 25 | `/api/events` closed at token expiry | Proposed | |
| 26 | Merged multi-node view | Proposed; cluster screen is a placeholder | `App.jsx:44-45` |
| 27 | Demo Session object and event log | Proposed (pattern exists in Lab runs) | `lab_api.rs` |
| 28 | Process-level failure injection | Existing/Prototype | `lab_live/gateway.rs`, M3 harness |
| 29 | Link-level fault injection as a product feature | Proposed (exists as test code) | `tests/support/fault_proxy.rs` |
| 30 | OS-level partition/latency | Hypothesis (privileged) | |
| 31 | Windows physical LAN demo, 4+ devices | Partial: all pieces, never exercised end to end | `limitations.md` Platforms |
| 32 | Linux / ARM / macOS devices | Unsupported | untested |
| 33 | Containers / VMs | Proposed | no image shipped |
| 34 | Remote participants over internet | Unsupported (Proposed via VPN, later) | s3.5 |
| 35 | Quantitative claims about cluster behaviour on LAN | Unsupported until release-build, multi-host, repeated measurements exist | CLAUDE.md s4 |

## 10. Phased roadmap with acceptance criteria

Each phase is independent, ends in something demonstrable and labelled, and states what it explicitly does not claim. Sizes are not estimated here (no basis).

**Phase 0: Truth audit and rehearsal harness (no src change).**
Deliverables: a one-page "what is real" chart (s6) reviewed against code; a launcher script outside `src/` that starts K gateways + mock origin(s) on loopback from a manifest, records PIDs/ports, tears down only those. Acceptance: (a) start/stop 3 times with no orphan processes and no ports left (`netstat`-style check recorded); (b) refuses to run over an existing unread session directory; (c) the manifest alone reproduces the topology; (d) all UI-facing labels derive from the manifest. Not claimed: any performance or distribution beyond loopback.

**Phase 1: Two tenants, one gateway, honest isolation demo (no src change).**
Deliverables: tokens for `alpha` and `beta` (external minter, per-session secret/aud), a script that shows (i) same path, different data; (ii) DELETE by alpha does not purge beta; (iii) **a flood by alpha evicting beta's entries** shown as a known limitation with measured counters, not hidden. Acceptance: tests/recordings fail if the isolation breaks; the noisy-neighbour effect is reproduced or the claim is withdrawn after a seeded run (release build, repetitions, CI per CLAUDE.md s4 before any number is quoted). Not claimed: quotas, fairness.

**Phase 2: LAN single gateway, N clients (docs + script; no src change if CORS/binds suffice).**
Deliverables: preflight checklist (IP, firewall, client isolation, clock skew, CORS origin, `ALLOW_LAN_ORIGIN`, TLS choice), pairing service v0 (short code, external script, TLS or labelled insecure), roster with expiry. Acceptance: 3 physical devices (presenter, Client A, Client B) complete pairing, run GET/DELETE under their tenants, Observer sees telemetry; guest token cannot call `/api/control` (403); expired token gets 401; wrong code rate-limited; teardown leaves no listening ports and no trust/firewall residue (checked by listing). Evidence tagged "LAN, Windows, debug/release stated". Not claimed: scale, security hardening beyond listed.

**Phase 3: Two-node cluster on LAN with mTLS (cluster PKI tooling).**
Deliverables: cluster PKI generator + members file generator (owner-approved tool); launcher support for nodes on separate devices; fault proxy lifted to a supported demo tool (or manual process-kill only). Acceptance: invalidation issued at node 1 observed at node 2 within a stated bound on the LAN, measured with release build and repetitions (this is a measurement, not a guarantee); kill node 2 mid-DELETE: owed invalidation delivered after restart; kill the *issuer* mid-fan-out: stale peer shown honestly with the M3 defect label until the durability fix is integrated; every failure event on the timeline with layer label. Not claimed: linearizability, bounded staleness beyond origin TTL.

**Phase 4: Demo Session object, merged live view (src/UI change; needs approvals).**
Deliverables: session schema (s5.2) with append-only event log, session-aware Control Center that connects to every node and labels node id, clock offset, `state_src`; optional tenant-labelled counters. Acceptance: reload/reconnect resumes with `Last-Event-ID`; session export hashes verify; every displayed value carries measured/simulated/inferred/unavailable; browser verification at desktop/tablet/mobile widths with retained screenshots (project standard). Not claimed: any value that is not labelled.

**Phase 5: Hardening (owner-reserved auth work).**
Deliverables: `jti`/denylist revocation, stream closure at `exp`, optional max-TTL, wiring of `kid` rotation, per-tenant quotas/admission (design only first). Acceptance: each change ships with a test that fails without it and an independent review pass (CLAUDE.md s5); invariants (generation fencing, epochs, zero-contention read path) shown unchanged by existing suites and a micro-benchmark with CI.

**Phase 6: Virtualised and remote variants (optional).**
VM/container scripts, virtual-switch partitions, VPN-based remote participants; every result labelled "virtualised network".

## 11. Owner-approval decisions (CLAUDE.md s2)

| # | Decision | Why it needs approval | Frozen / reserved files touched |
|---|---|---|---|
| D1 | Build pairing/minting as an external script (Node or PowerShell) vs a `src/bin` Rust tool | New tool; the Rust option is application-code change in a binary target (`Cargo.toml` `[[bin]]`, like `phantom-lab`) | None for script; `Cargo.toml` for bin |
| D2 | Token policy: max TTL, whether guests may ever hold `cache:invalidate`, whether Observer tokens may be issued to guests | Security policy; auth behaviour is owner-reserved | None (policy) |
| D3 | Revocation design (`jti`/denylist) and stream-closes-at-`exp` | Application code in authentication path | `src/security/jwt.rs`, `src/api/server.rs` (auth handlers) |
| D4 | Wire `kid` rotation into `main.rs` | Config/auth behaviour change | `src/main.rs`, `src/config.rs` (new key + `m7_config_tests` matrix) |
| D5 | Per-tenant quotas / tenant-labelled telemetry | Architectural change touching eviction, accounting, read path | `src/core/cache.rs` (invariants I1-I8, fencing, I5/INV-4), `src/core/eviction.rs`, `src/intel/telemetry.rs` |
| D6 | Lift `FaultProxy` from `tests/support` into a supported demo component | New runtime component | new file; copy from `tests/support/fault_proxy.rs` |
| D7 | Cluster PKI generator and members-file tool (uses `rcgen` which is a dev-dependency today) | New dependency if `rcgen` becomes a runtime or tool dependency; or openssl dependency on the host | `Cargo.toml` |
| D8 | QR support: new crate vs in-house encoder vs omit | New dependency decision with purpose/alternatives/maintenance/security | `Cargo.toml` or new module |
| D9 | TLS trust bootstrap policy for LAN (install demo CA vs pin vs insecure-HTTP-on-private-network) | Changes to device trust stores and security posture; must never be silent | None in repo |
| D10 | Firewall rule creation, loopback aliases, Hyper-V/Docker installation | System/global settings and software installation | None in repo |
| D11 | Durable invalidation log and shutdown drain from `docs/research/m3_durability` | Application code in invalidation path; independent review + failing-before test | `src/dist/router.rs`, `src/main.rs`, `src/config.rs`, new `durable_log` |
| D12 | Demo Session API on the gateway (new routes) vs only in the Control Center / launcher | Architecture; new routes on a gateway that today has none for sessions | `src/api/server.rs`, `src/main.rs` |
| D13 | Whether LAN demos may enable `PHANTOM_ALLOW_INSECURE_HTTP` and `PHANTOM_ALLOW_LAN_ORIGIN` | Weakens documented defaults | None (env), but recorded in session |
| D14 | Which files count as "frozen": this agent found no explicit frozen-file list; pre-registration artefacts (`docs/research/preregistration_*freeze*`, `*.pin`, `m6_sealed_results.md`) and sealed-seed handling (`lab_api.rs:104-108`) must not be touched | Clarify scope before any code | confirm list |

## 12. Honest gap list

1. No per-tenant resource isolation of any kind: one tenant can evict others, drain the byte budget and the origin concurrency cap. Only the namespace is isolated.
2. No tenant, client or request attribution in telemetry; SSE is a 100 ms snapshot, not an event stream; no resume on `/api/events`.
3. One origin per gateway; multi-origin only via multiple gateways or an origin that reads `x-tenant-id`.
4. Cluster membership is static, read at startup; no join, leave, discovery, or runtime revocation caller; node ids are u8; at most three nodes ever exercised, on loopback, Windows, debug builds.
5. GET is local-only; no client failover, no ingress router/load balancer. A node failure affects only its own clients and, for DELETE, invalidation propagation.
6. Invalidation durability defect is reproduced: a crash or graceful stop of the issuer loses owed invalidations; the fix exists only as an unintegrated scratch patch.
7. `NODE_FAILURE` is a logical flag, and the standalone node list is placeholders; both are easy to mis-draw as real.
8. No shipped token minting, no revocation, no max-TTL, no `jti`; HS256 shared secret equals the power to mint any tenant/role; `kid` rotation exists but is not wired; `/api/events` outlives token expiry.
9. No cluster PKI, node cert, fingerprint or members-file tooling in the product; `gen-demo-certs.ps1` needs `openssl` and covers ingress only.
10. LAN prerequisites (firewall, client isolation, clock skew, CORS exact origin, loopback-origin refusal on network bind) are known only from code and never exercised on a real multi-device network.
11. QR needs a dependency or in-house code; the short-code flow needs a new component either way.
12. No container image, no Linux/ARM/macOS validation, no VM/containers recipe.
13. Existing demo-lab documents are partly stale (LAN binding, router connection) and must not be quoted as status.
14. No performance or distribution claim is supportable from current evidence (debug, loopback, single host, repetitions limited), per CLAUDE.md s4.
15. Not read or not verified by this agent: `coalesce.rs` tenant uses (4 occurrences), UI `resolveGateway` validation, whether a secure-mode two-token isolation test exists, `/api/events` stream-count limit, and any behaviour on non-Windows OSes.

## 13. Questions for the owner

1. Is a guest device allowed to hold `cache:invalidate`, or are DELETE demonstrations presenter-only?
2. Which trust bootstrap is acceptable for phones/tablets: install a demo CA, certificate pinning by code, or plain HTTP with JWT on a presenter-owned private network?
3. Should pairing/minting live outside the repo toolchain (script) or be a supported `src/bin` tool, and who owns the session secret?
4. Is per-tenant isolation (quotas, fair eviction, tenant telemetry) in scope for the demo milestone, or must the demo state plainly that tenants share resources?
5. Do you approve promoting the test `FaultProxy` into a demo-supported component, and is any privileged fault injection (firewall rules, drivers) allowed on your machines?
6. Are Hyper-V, Docker or WSL2 permitted to be installed/used, and should their results be a separate evidence class?
7. Should the M3 durability fix be integrated before any multi-node demo that includes issuer-crash scenarios, or shown with the defect labelled?
8. Which devices and OSes will actually be available (counts, makes, Wi-Fi vs wired, guest-network isolation)?
9. Do you want the Control Center to show only nodes it can reach and verify, or also configured-but-unprobed nodes (which must then be labelled inferred)?
10. Confirm the list of frozen files and owner-reserved areas so Phase 4/5 scoping is exact.
