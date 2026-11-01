# Chapter 8: Limitations, Roadmap and Technical Defence

**State described:** the working tree and the project records as read on 2026-10-30, between about 16:15 and 16:40 on the machine clock (HEAD `df68eaa` at the time of writing). Several agents were still writing files at that time. Counts in this chapter are dated snapshots, not constants.

## What you will learn

- What PhantomCache cannot do today, and which of those limits are design choices and which are defects.
- How the 99 review findings of gate G4 (cycle 1) group into themes, what became of each theme, and which findings the owner has refused to let anyone dismiss.
- Exactly what has been committed, what exists only as an uncommitted file or a sandbox patch, and what nobody has independently verified.
- The planned future architecture (durable invalidation, tenant quotas, multi-host validation, intelligence experiments E0 to E2, the UI redesign) and the gates each step must pass.
- How to answer a hostile technical judge without overclaiming: what to say, what never to say, and what evidence backs each answer.

## 8.1 How to read this chapter

Every capability line carries one status label from the handbook rules. The labels mean:

- **IMPLEMENTED** (code exists; says nothing about whether it works) and **TESTED** (a named test exercises it; we say what it does not prove).
- **INDEPENDENTLY VERIFIED**: a human or external reviewer other than the author checked it. In this project **no item qualifies**. Every "independent" review on record is an AI second look (another AI session in the same tooling environment, or the lead re-running a command); `docs/release/workstream_completion_ledger.md` section 6 and `docs/STATUS.md` section 5b say so.
- **PARTIAL**, **PROTOTYPE ONLY**, **DESIGNED BUT NOT IMPLEMENTED**, **DEFERRED**, **BLOCKED**, **UNVERIFIED**: as in the style file.

Two rules shape the whole chapter. First, a passing test is not a proof of correctness. Several tests in this repository assert stub behaviour (`docs/research/code_findings.md` findings F3 and others). Second, the most recent record wins. Where an older document (the handoff, the README, parts of `CLAUDE.md`) disagrees with the code or with `docs/STATUS.md`, section 8.7 lists the disagreement instead of repeating the old claim.

::: concept
**Three questions for every claim.** (1) Does the code exist? (2) Does a named test exercise it, and what does that test not cover? (3) Has anyone other than its author checked it, and was that someone human? In PhantomCache the honest answer to the third question is "an AI second look at best".
:::

## 8.2 Where the project stands

### 8.2.1 Repository and test facts (read now)

| Fact | Value | How obtained | Label |
|---|---|---|---|
| HEAD | `804a7b8` on branch `feat/publish-phantomcache-core`, at the time of writing (2026-10-30, about 17:10; it was `df68eaa` at about 16:35 and `caeedfd` earlier the same afternoon). Local branch and its remote-tracking branch point at the same commit (all pushed) | `git rev-parse HEAD`, `git branch --show-current` | reproduced here |
| Commits since the first published snapshot `456bb25` | six, all documentation or tooling, no Rust: `8523cc1` (apple-design skill adoption), `3e6e03e` (docs-lint tool), `caeedfd` (four animation skills), `df68eaa` (`.gitattributes` protecting four hash-pinned frozen files from line-ending conversion), `9ab179f` (research records, phase reports, audits and plans), `804a7b8` (product, security, handoff and release documentation, docs index and historical notices) | `git log --oneline -8` | reproduced here |
| Tracked files modified, not committed | 22 (at the time of writing; the Rust, UI and test files modified by the redesign and earlier phases) | `git status --short`, status column | reproduced here |
| Untracked entries | 89 collapsed entries (`git status --short` prints 111 lines in total, tracked and untracked); about 4,510 entries with `-uall` at the time of writing (without `-uall` git collapses directories; the count moves as agents write) | same | reproduced here |
| Stashes | 0 | `git stash list` | reproduced here |
| Rust code committed beyond the first snapshot | none. The modified tracked files include `src/api/server.rs`, `src/core/cache.rs`, `src/dist/router.rs`, `src/main.rs` and others; the `.gitattributes` commit touches no Rust | `git status` list | reproduced here |

In plain words: **almost all the engineering of this project, including the Rust source changes made after the first snapshot, the UI work, the tests and the research records, exists only in the working tree of one machine and in one local backup on the same machine** (`docs/STATUS.md` section 5b; backup path in `docs/release/execution_register.md`). The owner's commit policy (decision ODR-3 in `docs/release/owner_decisions_register.md`) is to commit only individually reviewed file groups. Nothing in the code groups has passed that bar.

### 8.2.2 What the tests say

| Suite | Reported result | Source and limits |
|---|---|---|
| Rust, `cargo test --tests -j 1 --no-fail-fast` | 851 passed, 0 failed, 8 ignored, 75 targets | `docs/release/verification_register.md` V-01, lead re-run on 2026-10-30 between 10:11 and 10:23. Debug profile, Windows, single host. The tree has not changed since by file time (V-04; modification time is not a content hash). Not run by me. |
| Rust, clippy `-D warnings` | exit 0 | V-02. `cargo fmt --check` fails with 38,042 diff lines (G1 disposition D4); it is not clean and was deliberately left alone. |
| Rust, release profile | a targeted run of 10 suites: 209 passed, 0 failed, 6 ignored | V-03. **There is no release-profile general suite.** |
| UI, `node --test test` in `ui/` | 483 passed, 0 failed (30 `*.test.mjs` files), reproduced during the final reconciliation pass of this handbook with `npm test` (2026-10-30, evening, Windows, one host). Earlier records: 298 at 15:29 (V-07) and 299 at 16:07; the suite grew after the redesign added tests | Tests are mostly regex and rendered-markup guards. They do not prove layout or visual quality. |

Inventory by reading and counting only (no test was run): `tests/` holds 97 Rust files and about 821 `#[test]` or `#[tokio::test]` attributes; `src/` holds 19 more; `ui/test` holds 30 `*.test.mjs` files (the count of 483 above is from running them, not from grepping); 9 `#[ignore]` attributes appear in `tests/` and `src/`. The 851 figure comes from the recorded run, not from this count, and a grep cannot see macro-generated tests. The numbers 242 and 263 in `CLAUDE.md`, `README.md` and older reports are historical (see 8.7).

**Why the numbers differ (the one canonical explanation; other chapters point here).** They are different measures, not rival answers. (a) 242 tests in 19 targets is the baseline in `CLAUDE.md`, from an older reproduction (debug, 2026-10-28) before many tests were added. (b) 851 passed and 8 ignored in 75 targets is the recorded run in `docs/STATUS.md` and `docs/release/verification_register.md` at HEAD `456bb25` (debug, Windows, single host); the repository HEAD has since moved only by documentation commits (8.2.1). (c) "About 821 plus 19" and "9 `#[ignore]`" are greps of test attributes: they cannot see macro-generated tests or count parameterised cases, and they are not run results. (d) The UI suite is separate: 483 tests in 30 files at the last run, up from 298 and 299 earlier the same day because the redesign added tests. Quote a number only with its measure.

**Passing tests do not establish correctness.** Examples recorded in the repository: the legacy PIE tests assert the stub's wiring (`tests/integration/p4_pie_tests.rs`, `code_findings.md` F3); the stale-cap violation counter is nearly a tautology (G4A-18); the sealed-evaluation tests that once computed research results from float literals were found hollow (`claims_ledger.md` section 4).

### 8.2.3 The delivery gates

| Gate | Status (docs/STATUS.md section 2 and the ledger) |
|---|---|
| G1 Scenario B | PASS with dispositions D1 to D5. Results are exploratory (debug, loopback, one host). |
| G2 Lab API | PASS with dispositions G2-D1 to G2-D7. |
| G3 UI plus browser verification | Verdict not recorded. Open high findings U-01, U-02, U-03. Visual quality **not approved**; owner decision O8-2 is open. |
| G4 two whole-product review cycles | Rust cycle 1 delivered (three AI reviewers, 99 findings). Cycle 2 and the UI side not started. |
| G5 reviewed commits and push | Preparation done; only documentation groups committed; nothing pushed. |

Formal owner acceptance of any milestone M1 to M8 is **not recorded** anywhere in the repository. `docs/STATUS.md` calls that "an evidence gap, not proof that no authorization or acceptance was given", because the owner may hold records outside the repository.

![Figure 8.1: Status map of the three pillars](../assets/ch08_three_pillar_status.svg)

Source: `docs/research/vision/phantomcache_three_pillar_product_direction.md` sections 2.1 and 5.1 to 5.4; `docs/STATUS.md`; `docs/release/workstream_completion_ledger.md`.

The pillars come from the owner's current mandate: distributed systems engineering, workload intelligence, and product excellence with observability. The product-direction document that defines them is itself a **proposal** ("Nothing in this file is approved, implemented, or run"). The figure shows how little of the aspirational scope exists. The strongest boxes are single-node correctness and the loopback invalidation bus. Everything that makes the system a coordinated, intelligent cache is designed, prototyped, or blocked.

## 8.3 Architectural limitations

An architectural limitation is a boundary of the design, not a bug. Each item below states the limit, the evidence, and the label.

### 8.3.1 One cache per gateway, served locally

GET is served from the local cache only (Chapter 5). The cluster path is an invalidation bus between independent caches, not a distributed cache: `docs/user/limitations.md` says "there is no cross-gateway read-through", and the product-direction document (section 5.2) separates "N independent caches with an invalidation bus" from coordinated distributed caching, "which does not exist". Peer-fill, coordinated placement, cluster capacity and cluster-wide origin protection are **DESIGNED BUT NOT IMPLEMENTED**.

The cluster `GetRequest` handler in `src/dist/node.rs` is a stub that fabricates a `val_node_<id>` value (the format string appears at two sites; verified by grep). DR-029 / G4B-30 records it as a design defect: an authenticated peer can make a node store a fabricated entry. A debug loopback reproduction found no HTTP request that can reach it (disjoint key spaces), so the rating is Low as exploited today and Medium as a latent hazard (`docs/security/investigations/DR-029_cluster_get_cache_poisoning_investigation.md`; AI second look only, no fix applied). Detail: Chapter 6, section 6.6.

### 8.3.2 Placement is plain rendezvous hashing, used for DELETE

`RendezvousRouter::route_key` in `src/dist/router.rs` computes a hash per node and picks the highest. The weight multiplier is the constant `capacity_weight = 1.0` (seen at two sites). The hash is `std::collections::hash_map::DefaultHasher`, which Rust does not guarantee stable across toolchain versions (G4B-27). So routing is unweighted, not health-aware beyond membership, and not safe to compare across differently built gateways. It decides which node is the primary for a DELETE; it does not route reads. Status: **IMPLEMENTED**. The target (a fixed hash with golden vectors, owners derived from a membership epoch) is **DESIGNED BUT NOT IMPLEMENTED**.

### 8.3.3 Invalidations owed by the issuer live in memory

After a DELETE the other members are told asynchronously, with bounded retry and a repair worker, but the ledger of invalidations a peer has not yet acknowledged lives only in the issuing process's memory (Chapter 5; Chapter 6, section 6.9). Kill that process before delivery and the invalidations are lost; peers keep serving the old entry until it expires. The defect was reproduced on unmodified code (`docs/research/m3_durability/reproduction_report.md`: 92 of 92 polls stale over 10 s with the log off, in the sandbox reproduction). `docs/user/limitations.md` warns operators that **a 204 from DELETE is not proof that the object is gone from every gateway**: 204 means the canonical primary, which may be the issuing node itself, acknowledged within a 150 ms leg (G4B-09). The fix is a sandbox patch and is **not integrated** (8.8).

### 8.3.4 Freshness bounds are weak

The only gateway-side staleness mechanics are: origin freshness headers, a default of 60 s when the origin gives none, and the hard stale cap `PHANTOM_MAX_STALE_MS` (24 h by default) that limits how long an expired entry may be served stale. **Origin-supplied lifetimes have no gateway ceiling** (`docs/user/limitations.md`, `src/core/freshness.rs`). `PHANTOM_DEFAULT_TTL_MS` is validated by `src/config.rs` but not wired: `src/api/server.rs` passes the literal `60_000` to `evaluate_response_freshness` (verified at two call sites). Setting the key does nothing. This confirms `CLAUDE.md` section 5. Consequence: you cannot state a "staleness bound" for the product beyond "the origin's lifetime, or 60 s".

TTL, staleness and the cap use the wall clock (`SystemTime`), so a backwards clock step extends windows (G4A-10, PLAUSIBLE, not tested).

### 8.3.5 Tenancy is key isolation, not resource isolation

The cache key includes the tenant id and per-tenant epochs fence invalidation (`src/core/cache.rs`, `CacheKey`, `tenant_epochs`, `MAX_TENANT_EPOCHS = 10_000`; overflow falls back to the global epoch). There are **no per-tenant quotas, no per-tenant byte accounting and no fair eviction** (invariant I9 and Stages A and B of the quota plan are **DESIGNED BUT NOT IMPLEMENTED**; execution register items E5 and the P3 section of the product-direction file). A noisy tenant can fill the byte budget, and any authenticated cluster member can send tenant-scoped or full-cache invalidations (DR-027 / G4B-05, owner-named, open).

### 8.3.6 Platform and evidence scope

Only Windows x86-64 has been exercised. No Linux run exists, so Unix-specific code paths (file permissions, signal handling) are unexecuted. No result is from a physical multi-host network; no cluster larger than four nodes has run (`docs/research/m3_durability/remediation/00_status_and_plan.md` section 7). Every measurement is debug-profile, loopback or simulated. Keep these tiers labelled apart: loopback, simulated (T1), WSL2, physical host.

### 8.3.7 Operations gaps

`docs/user/limitations.md` lists as not provided: Prometheus `/metrics`, certificate hot reload, runtime configuration reload, token revocation, container image, SBOM, dependency advisory scan, code signing. In-flight connection draining and a sweeper shutdown channel are listed as open integration items D-1 and D-2 in `docs/release/deferred_integration.md`; an idle gateway was observed using more than one CPU core (single runs, debug and release, Windows; the cause is a code-reading hypothesis).

## 8.4 Known defects and risks, by theme

### 8.4.1 What the G4 register is

Gate G4 asked for two whole-product review cycles. Cycle 1 on the Rust side produced three reports (`docs/release/g4/cycle1_core_engine_review.md`, `cycle1_dist_security_api_review.md`, `cycle1_lab_intelligence_deps_review.md`): 26 findings from G4A (core engine), 38 from G4B (distribution, security, API), and 35 from G4C (lab, intelligence, freeze, dependencies). Total **99**. The disposition register (`cycle1_findings_disposition_register.md`) adds the status of each. Four caveats govern how to read it:

1. All three reviews are **AI second looks**: one AI pass each, read-only, no tests run. "CONFIRMED" in the reports means confirmed by reading code, not by an executed failing test.
2. The register closes nothing. Only owner statements (ODR-1 to ODR-4) set dispositions.
3. G4C did not read the simulation core of the lab (listed in `docs/release/known_limitations_frozen_code.md` section 1), so the absence of a finding there is absence of review.
4. The register's grouping into commit groups is an inference.

Counts by severity and disposition (register section 3.1): Critical 0; High 4; Medium 37; Low 42; Info 16. Dispositions: 75 OPEN (of which six owner-named), 20 RECORDED-LIMITATION, 2 IN-SANDBOX-REMEDIATION, 1 UNDER-INVESTIGATION, 1 REFUTED.

### 8.4.2 The 99 findings by theme

The themes below are **my grouping** of the register's IDs (the register groups by report, not by theme). I computed the counts with a short script over the register rows and checked that the totals reconcile to 99. A "RECORDED" finding is one the owner decided to record as a known limitation in frozen code (ODR-1). "Sandbox" means a fix exists as an unintegrated patch in a scratch copy.

| Theme | Count | Highest severity | Dispositions | Representative IDs |
|---|---:|---|---|---|
| Fencing, epochs, invalidation races | 4 | High | 1 under investigation (G4A-01), 3 open | G4A-01, -08, -12, -17 |
| Eviction, byte accounting, resource bounds | 8 | High (conditional) | 8 open | G4A-02 (S3-FIFO tombstones), -03 (sweeper scan), -13 (byte drift) |
| HTTP freshness semantics and config wiring | 8 | Medium | 8 open | G4A-04 (upstream `Age` ignored), -05 (`Expires: 0`), -06 (stale-if-error on 4xx), -25 (TTL literal) |
| Telemetry honesty and clocks | 5 | Medium | 5 open | G4A-10 (wall clock), -11 (hit-rate definition), F24 (percentile conventions) |
| Tenancy alias and lock robustness | 2 | Low | 2 open | G4A-19, -21 |
| Ingress, denial of service, rate limiting | 7 | High | 6 open, 1 owner-named open | G4B-01 (slow body before auth), -02, -11 |
| Cluster trust, membership, invalidation delivery | 12 | Medium | 8 open, 4 owner-named open | G4B-05 (DR-027), -06, -07, -08, -09, -27, -30 |
| Authentication, JWT, SSRF | 4 | Medium | 4 open | G4B-13 (`nbf`, lifetime), -14, -15, -18 |
| API surface and control hygiene | 14 | Medium | 13 open, 1 owner-named open | G4B-03 (`/api/control` mutates live nodes), -04, -16, -20 |
| Dependencies | 2 | Info | 2 open | G4B-34, G4C-DEP (no `cargo audit` run) |
| Scenario B and Lab live | 9 | High | 2 in sandbox (F01, F02), 7 open | F01 (U-01), F02, F06, F07, F08 |
| Frozen lab verdict and provenance | 13 | Medium | 10 recorded, 2 open, 1 refuted | F03, F04, F05, F16, F27, F34 (refuted) |
| Intelligence library (not wired) | 10 | Medium | 10 recorded | F09 to F15, F23, F25, F26 |
| Documentation consistency | 1 | Low | 1 open | F28 (README MSRV claim) |
| **Total** | **99** | | 75 open (6 owner-named), 20 recorded, 2 sandbox, 1 under investigation, 1 refuted | |

Reading the table honestly: the **runtime-serving risk is concentrated in four themes**: ingress hardening, cluster trust, fencing races, and freshness semantics. The intelligence and frozen-lab themes carry many findings but mostly in code that is **not wired into serving** (G4C's grep; the register's "LATENT" impact class). They matter for research validity and for any future activation, not for today's request path.

### 8.4.3 The six findings the owner named

ODR-4 names six findings that "must not be dismissed": G4B-01 (the HTTP ingress reads a request body before authentication, with no body timeout or connection limit), G4B-03 (`/api/control` mutates the live router nodes and a missing event sets every node Active), G4B-05 (any active cluster member can send tenant or full-cache invalidation scopes; DR-027), G4B-06 (no membership-epoch check on `InvalidateRequest`; the acknowledgement-epoch check is vacuous because the receiver echoes the sender's epoch), G4B-07 (cluster membership is static and the revocation machinery has no production trigger) and G4B-08 (live mutual-TLS connections never expire, so certificate expiry and rotation are enforced only at handshake). All six are **OPEN**. None has a fix in the tree. G4A-01 is separate: the owner calls it **unconfirmed** and ordered an investigation.

### 8.4.4 Findings I re-checked against the current tree

The register says "AI second look" and the line numbers in reports drift. I opened the current code for several High and Medium items. This is still reading, not execution.

| Finding | What I opened | What I saw | Result |
|---|---|---|---|
| G4A-01 (stale body survives an acknowledged invalidation) | `src/core/cache.rs`, `finalize_fetch` region | The fence check compares `state.generation == op_gen` and the entry epochs to `get_effective_epochs(...)`; later, in the insert block, the code calls `get_effective_epochs` again and stamps `entry_tenant_epoch` and `entry_global_epoch` with the **newly read** values. | Mechanism present in the tree as the reviewers describe it. Natural-schedule reproduction was not achieved (see 8.8). |
| G4A-02 (S3-FIFO `on_remove` leaves tombstones) | `src/core/eviction.rs`, `FaithfulS3Fifo::on_remove` | It removes the key from `freq` and `ghost_set` only; it does not touch `small_q` or `main_q`. | Present. Reachability: `set_eviction_policy` is called only from `src/control/pie.rs` (not instantiated: `pie: None` in `src/main.rs`) and from `src/intelligence/executor.rs` (`apply_policy`, no serving caller per the limitations record). `PartitionedCache::new(8)` starts with the default policy. So S3-FIFO is not reachable in the default serving path. I did not exhaustively search all callers. |
| G4A-03 (sweeper scans the shard) | `src/core/cache.rs`, sweeper loop | The loop iterates `shard.map.iter()` and breaks when `candidates_to_reclaim.len() >= max_per_shard`, so the batch limit bounds candidates, not keys inspected. | Present. |
| G4B-01 / G4B-03 (extractors before authentication) | `src/api/server.rs`: `control_handler`, `get_cache_handler`, `delete_cache_handler`, `create_app_with_lab` | `control_handler` takes `Json(payload)`; the GET and DELETE handlers take `_body: axum::body::Bytes`; all three extract the body before the handler can authenticate. The only limit found is `DefaultBodyLimit` at 512 KiB; no timeout, concurrency or connection layer. `control_handler` also falls through to "set every node Active" for an unknown event. | Present for all three handlers. The runtime effect of a slow body remains unverified. |
| G4B-13 (JWT `nbf` not validated) | `src/security/jwt.rs` | `Validation::new(HS256)` with `leeway = 10`; no `validate_nbf` assignment seen. | Consistent with the finding (library default). Not tested. |
| G4B-27 (unstable hash) | `src/dist/router.rs`, `route_key` | `DefaultHasher` used. | Present. |
| TTL literal (G4A-25, CLAUDE.md) | `src/api/server.rs` | `60_000` passed to `evaluate_response_freshness` at two sites; `src/config.rs` registers the key as not wired. | Present. |
| F28 (README MSRV) | `README.md` | Line says `cargo 1.75+`; `Cargo.toml` has no `rust-version`. G4C says locked crates need about 1.88. | Present; the 1.88 figure is G4C's, I did not re-derive it. |

I did **not** re-check the lab and intelligence findings by execution, and I did not run any test.

### 8.4.5 The sealed result and what the limitations record says

The sealed M6 evaluation (Tier 1, **SIMULATED**; Chapter 7, section 7.6) ran once and found H1 **NOT SUPPORTED** (`docs/research/m6_sealed_results.md`). `docs/release/known_limitations_frozen_code.md` says the result "has NOT been independently reviewed by a human or an external reviewer", that G4C did not read the simulation core, and that the run is a forced-adapt comparison on a simulator. Four of 56 hashed freeze files differ from the frozen values (`Cargo.toml`, `src/lib.rs`, `src/lab/eval.rs`, `tests/integration/m6_research_evaluation_tests.rs`); an addendum written after the run explains all four, and the `eval.rs` verifier change is a refactor (+108/-2), not a purely additive edit. The frozen versions can be rebuilt only from copies outside the repository (G4C F27). Treat the freeze chain as reproduced by the lead and an AI, not audited by a person.

### 8.4.6 Other recorded risks

- **SEC-M7-09, -10, -11** (role alias and bare tokens, SSRF range gaps, rate limiting and JWT lifetime) are open (`docs/security/m7_security_review.md`; execution register E3, queued). SEC-M7-01 to -08 are recorded as fixed and tested.
- **Hostile-input paths**: the HTTP ingress has no host or origin guard in anonymous mode (G4B-04) and `/api/events` has no stream cap (G4B-02).
- **Telemetry honesty**: the hit-rate function returns 1.0 with no data (G4A-11). UI values must be labelled measured, simulated, inferred or unavailable (`CLAUDE.md` section 4); the UI increments I1 to I4 aim at that, but visual and usability acceptance is open.
- **Dependencies**: 288 locked packages, two TLS crypto stacks, broad feature flags; **no advisory scan has been run** (G4C-DEP, execution register E4 queued).

## 8.5 Incomplete and deferred functionality

| Item | State | Why it is not done |
|---|---|---|
| Durable invalidation log, drain on shutdown, restart replay | PROTOTYPE ONLY (sandbox patch set, not integrated) | Protected-core files; needs independent review and owner approval; design choices open (fail-open or fail-closed on log errors, default fsync policy, drain deadline) |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED, but no code path lets a heartbeat change node state (G4B-12) |
| Runtime membership, certificate revocation trigger, connection ageing | PARTIAL | Static member file; machinery without a production trigger (G4B-07, -08) |
| Peer-fill, stable placement, coordinated capacity | DESIGNED BUT NOT IMPLEMENTED | Gated on durable invalidation, a formal fill model and independent review |
| Tenant quotas and fair eviction | DESIGNED BUT NOT IMPLEMENTED | Frozen `src/core/cache.rs`; needs addendum 2 and owner approval |
| Hot-key detection (`HeavyHitterDetector` library) | PROTOTYPE ONLY | Library in frozen `src/intelligence`, unit-tested, not wired into the serving path; `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test (`tests/unit/wp2_tests.rs`) |
| Event tap, pre-warming, forecasting | DESIGNED BUT NOT IMPLEMENTED | Frozen `src/intelligence`; experiments E0, E4, E9 first |
| PIE as a counterfactual canary | PROTOTYPE ONLY | `src/control/pie.rs` is a legacy threshold stub with a literal `expected_utility`; `PieDecisionEngine` is not wired. The `PHANTOM_INTELLIGENCE=canary` mode applies the origin cap to all fetches and has no rollback in the serving path (F14) |
| Release-profile suite, 2 h soak, benches on a quiet machine | DEFERRED | Needs an exclusive machine and a stable tree |
| Multi-host LAN validation | BLOCKED | Needs physical devices (owner) |
| Human design, usability, accessibility and security review | BLOCKED | Needs human reviewers (owner) |
| `/metrics`, TLS hot reload, config reload, token revocation | DEFERRED | Owner decision O7-4 and others |
| Graceful connection draining | DEFERRED | Integration item D-1 |

## 8.6 Design trade-offs

State each trade-off with the alternative. Where the repository does not record why an alternative was rejected, the text says "our reasoning, not a recorded decision".

- **Local-only GET versus read-through.** Local GET gives a hot path with no network round trip and no peer dependency. The price is N independent caches, so each node pays its own cold misses and the origin sees up to N fills of the same object. Read-through (peer-fill) would reduce origin load but adds a failure and consistency surface. The owner's roadmap defers it behind durable invalidation. (Our reasoning for the cost; the deferral is recorded in the product-direction file section 2.2 rule 1.)
- **Fencing, coalescing and the invalidation bus.** The mechanisms are explained in Chapters 4 and 5; the trade-offs are: generations plus epochs cost extra state per key and a second lock, and their weak point is G4A-01 (8.4.4); single-flight protects the origin but waiters share the leader's fate, and requests after an invalidation can join a pre-invalidation fetch (G4A-08, limitation L3); best-effort fan-out with repair keeps the write path fast and converges only while the issuer survives, without linearizability, whereas a consensus log would order better at higher latency and complexity. The repository records the failure model and bounds as clauses in the product-direction document (section 7), not as an ADR that rejects consensus.
- **Static origin cap over adaptive control.** After the sealed negative result, the research lead chose MVP-B: the static gateway with a configurable static origin cap; the adaptive controller is **not** promoted to serving (`m6_sealed_results.md`, "MVP decision"). This is a recorded decision by the research lead, still provisional until the release gates pass.
- **Frozen files versus fixing defects.** The pre-registration freeze protects the integrity of the sealed result. The price is that defects in frozen files (F03 to F26) are recorded as limitations rather than fixed, because any edit needs a signed addendum. That is a deliberate owner decision (ODR-1).
- **Sandbox remediation.** Rust fixes are built in scratch copies and not merged. This protects the protected core and the evidence, but it means the reported defects remain in the tree.

## 8.7 Technical debt and documentation honesty

### 8.7.1 Debt in the tree

- **Everything uncommitted** (8.2.1). The single largest delivery risk.
- **Protected-core gates.** `src/core/**`, `src/lab/**`, `src/intelligence/**`, `src/lib.rs` and `Cargo.toml` are frozen or protected. Changes touching fencing, epochs or invalidation need an independent review pass and a test that fails without the change (`CLAUDE.md` section 5).
- **Preregistration holds.** The two commitment files and the code holding commitment strings are on hold for an owner check offline, which blocks committing the Rust code groups (execution register, owner decisions register "Still open").
- **Formatting.** `cargo fmt --check` fails (38,042 diff lines); formatting would rewrite frozen files, so it waits for addendum 2.
- **Line endings.** `core.autocrlf=true` is set. Commit `df68eaa` added a `.gitattributes` that marks exactly four hash-pinned frozen files `-text` (its comment cites DEC-006, owner-approved, Option B), so their bytes are not converted. There is no repository-wide normalisation, so other files can still change line endings on another machine.
- **No release-profile suite**; the soak (2 h) has not been run.
- **Weak tests.** Tests that pin weak claims (golden trace, cap counter) are flagged for rewrite (G4A section 5); the UI fixture masked U-01 (`ui/test/lab.view.test.mjs`, F01).

### 8.7.2 Statements in project documents that are out of date

I compared the main status-bearing documents with the current code and records.

| Document | Statement | Reality (current evidence) |
|---|---|---|
| `docs/handoff/00_START_HERE.md` | "Phase 4 Remediated, Verified, and Formally Accepted (242/242 tests passing across 19 targets)"; "Do NOT begin Phase 5" | The same file carries a notice that prior claims were audited and corrected (`docs/handoff/CORRECTIONS.md`). Formal acceptance is not recorded for any milestone; the test baseline is now 851 across 75 targets (debug). `CLAUDE.md` section 6 says the "Phase 4 accepted / zero P0" claim is not supported. |
| `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md` | "ZERO open P0 code blockers remain in Phase 4" | Superseded: G4B-01 and others are open; its roadmap is marked historical. |
| `docs/handoff/00_START_HERE.md` | "64-Partition Lock Striping" | `src/main.rs` constructs `PartitionedCache::new(8)` (verified); `new` takes `num_shards` and asserts a power of two. The gateway starts with 8 shards, not 64. A handoff claim, not verified elsewhere. |
| `CLAUDE.md` section 6 | evictors "are scans"; "the only staleness bound is a fixed 60 s TTL"; reported baseline 242 tests | Section 5 of the same file lists `ShardEvictionPolicy` with `FaithfulSieve`, `FaithfulLru`, `FaithfulS3Fifo` (verified present in `src/core/eviction.rs`), so the "scans" note is outdated; origin headers and a stale cap also bound staleness (the 60 s is the default only); 242 is historical. Parts of section 6 already carry "OUTDATED" annotations. |
| `CLAUDE.md` section 6 / `src/main.rs` startup text | "startup message at `main.rs:368`" says peer invalidations do not purge the HTTP cache | The mTLS path builds the local node with `VirtualNode::new_with_cache(node_id, cache.clone())` (verified at `src/main.rs` near line 332), so the local node shares the HTTP serving cache. The insecure-cluster message in `main.rs` now describes a different situation (no listeners, DELETE reports a gateway timeout). But `src/config.rs` still emits "peer invalidations do not purge this gateway's HTTP cache" (verified near line 858) — contradicts the mTLS path (G4B-32). |
| `README.md` | "cargo 1.75+" | False per G4C F28 (about 1.88 needed; not re-derived). |
| `docs/research/claims_ledger.md` C6 | "zero stale reads after 50 ms" | Amended in the same file (section 5, C6 amendment): cold first use took 35 to 60 ms in every isolated run, so "P(stale)=0 at 50 ms" is **not supported**. |
| `docs/research/claims_ledger.md` section 5 / round 2 | "histogram has no callers in `src/`" | Partly outdated. `TelemetryCollector::record` and `record_us` do feed `AtomicLogLinearHistogram`. But no production code reads its percentiles: `src/api/server.rs` calls only the ring-buffer version (`calculate_percentiles_us`), and `calculate_percentiles_histogram_*` have no caller outside tests (grep, 2026-10-30; Chapter 6, section 6.12.2). |
| `docs/research/code_findings.md` (2026-10-28) | evictors are scans; SIEVE "is not SIEVE" | Predates the faithful policies. Treat as historical. |
| `docs/STATUS.md` | UI tests "298" | Superseded: the suite is 483 tests in 30 files after the redesign added tests (measured, 8.2.2). Minor. |

The reading rule that follows: **trust `docs/STATUS.md`, the ledger and verification register for status; trust the code for behaviour; treat the handoff and the README as history.** The register also holds an acknowledged discrepancy of its own: G4A-19 and the G4B refutation of the `__default__` tenant alias point in different directions and nobody has reconciled them (register section 2, discrepancy 1).

## 8.8 Remediation status: what is committed and what is not

![Figure 8.2: Roadmap in dependency order](../assets/ch08_roadmap_stages.svg)

Source: `docs/release/execution_register.md`; `docs/release/lead_working_state.md` section 4; `docs/research/vision/phantomcache_three_pillar_product_direction.md` section 12.

**Documentation and tooling commits** (`8523cc1`, `3e6e03e`, `caeedfd`, `df68eaa`: apple-design skill, docs-lint tool, animation skills, `.gitattributes` for the four frozen files). Applied to the main tree: yes, committed. Evidence: blob hashes checked by the lead.

**U-01 backend fix** (cancelled or partial Scenario B runs must not carry a completed mechanism verdict).

- *Where:* `docs/research/u01_backend/u01_backend.patch`; sandbox `D:\Temp\sbx_u01`. Applied to the main tree: **No, not yet**: the owner approved integration with conditions (DEC-001, `docs/design/OWNER_DECISION_REGISTER.md`) and an integration agent is working in the working tree, but no integration record exists at the time of writing (`docs/research/u01_backend/integration/` is absent), and nothing is committed.
- *Evidence quality:* 5 of 6 new tests fail on unmodified source and pass after the fix; the target ran 65 passed, 0 failed, 2 ignored; debug profile, one test target, no clippy, patch not tested with `git apply` (`u01_backend/status.md`).
- *Gate:* an AI independent review is complete (`independent_review.md`; AI, not human); owner approval is given with conditions (DEC-001, ODR-2); UI contract in `contract.md`. Still open: finishing integration, the conditions checklist, and any human review.

**G4A-01 fence fix proposal** (stamp the epochs captured by the fence check).

- *Where:* `docs/research/g4a01_race/proposal.patch`. Applied: **No**.
- *Evidence quality:* reproduced only with an **injected yield point** in a sandbox (two failing tests, a passing control); 20 s of unhooked three-thread stress, about 3.16 million fills, 0 violations; the natural window is nanoseconds wide and its natural frequency was not measured.
- *Gate:* independent review, a decision on a permanent test hook, owner approval for the protected `src/core/cache.rs`.

**M3 durable invalidation** (opt-in log, replay, shutdown drain, detached DELETE leg, crash-point tests).

- *Where:* `docs/research/m3_durability/` and `remediation/patches/`; sandbox `D:\Temp\sbx_m3`. Applied: **No**.
- *Evidence quality:* sandbox report: 6 of 8 new unit tests and 1 of 15 process tests fail on the unfixed patch and pass on the fixed one; 21 unit tests pass; debug, loopback, one Windows host. The report lists what remains UNVERIFIED (no Linux, no power-loss test, no multi-host, AI-only review). **With the log unset (the default) the defect would remain even after integration.**
- *Gate:* independent review brief, owner approval; touches `router.rs`, `main.rs`, `server.rs`, `config.rs`, `lib.rs`.

**DR-029, DR-027, SEC-M7-09/10/11, advisory scan, quotas, intelligence tap** (execution register E1, E3, E4, E5, E6). Applied: **No**; queued behind M3, nothing written. Evidence: none. Gate: per item.

The M3 status file says "fix prepared and verified in a sandbox" while `docs/release/lead_working_state.md` listed the M3 agent as still running at the last update (discrepancy; I did not resolve it). Nothing in this table has been reviewed by a human.

## 8.9 Future architecture

This section describes plans. Every item is **DESIGNED BUT NOT IMPLEMENTED** or **PROTOTYPE ONLY** unless stated. The source for the stages is the proposal in `docs/research/vision/phantomcache_three_pillar_product_direction.md` sections 12 to 14, which the owner has not approved as a whole.

### 8.9.1 M3: durable invalidation

The idea in plain words: before a gateway tells peers about a delete, it writes the debt ("peer B still owes me an ack for key K") to a file. If it dies and restarts, it reads the file and finishes the job. A small explanation of the terms: a **write-ahead log** records an intent before acting; **replay** reads it after restart; **drain** waits at shutdown for debts to be settled; **fsync policy** decides when bytes are forced to disk.

The sandbox design (`docs/research/m3_durability/design.md`, `remediation/00_status_and_plan.md`): a new `src/dist/durable_log.rs`, ledger hooks in `router.rs`, replay before the repair worker starts, a shutdown drain, and a detached DELETE leg. The open owner choices include fail-open versus fail-closed on log errors, the default fsync policy, the drain deadline and debt age-out. Falsifier: a crash-point matrix of multi-process tests (15 in the sandbox) that kills a node at defined points and checks convergence. Not yet true: Linux, power loss, multi-host.

### 8.9.2 Tenant quotas

Stage A adds per-tenant byte accounting (invariant I9) behind a hook, with a property test over random operations and an "omit one site and the test must fail" check. Stage B adds quotas and fair eviction, checked by a noisy-neighbour suite. Both touch the frozen `src/core/cache.rs` and need addendum 2, owner final approval and independent review. Until then, tenancy is key isolation only (8.3.5).

### 8.9.3 Multi-host validation

A plan and runbook for a physical LAN run are being prepared (execution register N5). It is **BLOCKED** on devices and operating systems from the owner and on human reviewers. The proposed rule is that the label `MULTI_HOST_LAN` is created only after a real run is retained. Until then, every cluster result is loopback on one machine, and the UI must never draw three processes on one host as three machines.

### 8.9.4 Intelligence experiments E0 to E2

The mode ladder is offline, shadow, canary, active, and every decision is validated by a separate safety layer against hard caps (staleness, origin load, memory), which the execution layer enforces regardless of any intelligence decision (`CLAUDE.md` section 6b).

- **E0**: measure the cost of an event tap and observation pipeline and check counts against existing counters. Exploratory first; release build; kill criterion: hot-path adder beyond a budget fixed before the run.
- **E1**: check that a load-dependent origin plant exists and behaves correctly in the simulator (validity work; kill if rank disagreement between simulator tier and real-process tier exceeds 25% of paired scenarios).
- **E2**: the headroom question: can any per-scenario cap beat one good static cap by at least 3 percentage points of goodput? The protocol is a **draft** (`docs/research/intelligence_e2_headroom_protocol_DRAFT.md`). A dry run is exploratory; a confirmatory run needs an E1-validated plant, a new owner-generated salt and a freeze. If E2 returns NO_HEADROOM, adaptive caps are abandoned and only the advisor continues.

These stay **BLOCKED** or exploratory until the owner approves lab changes and generates a salt offline. No agent may generate, store or propose a salt. Any change to `src/lab` or `src/intelligence` needs addendum 2. An ML selector is admitted only after a non-ML selector passes a five-part gate; claim C12 ("ML failed to beat heuristics by 0.020") is UNVERIFIED because the test asserts no comparison (`claims_ledger.md` section 4).

### 8.9.5 The UI redesign

Increments I1 (foundations), I2 and I2b (Phantom Lab), I3 (Simulation page retired to an honest explainer) and I4 (shell and non-Lab pages) are **IMPLEMENTED and TESTED** at the UI-test level (483 tests at the last run; 298 or 299 when the contrast and a11y figures here were recorded; contrast 196/196, a static a11y lint on 74 combinations with 0 issues, Vite build ok at 14:26). The owner has since approved the toolkit and redesign phase, so the UI implementation phase is open (lead working state section 3b). Not done: increment I5 (final polish), the Track 2 accessibility report beyond its raw evidence, re-verification of browser-round findings on the current UI, and human design, usability and screen-reader review. **Visual quality is not approved; owner decision O8-2 is open.** The three high usability findings U-01 (verdict-like result on a cancelled run; backend half unintegrated), U-02 (no control for the gateway address) and U-03 (progress "0 of 2 units" looks stalled) came from an AI walkthrough, not human data.

## 8.10 Production-maturity checklist

Read "Status" as the honest present state. "No" means absent; nothing here is a claim of readiness.

| Area | Requirement | Status |
|---|---|---|
| Correctness | Fencing, byte accounting, freshness tested | TESTED (debug); open G4A-01 and freshness gaps |
| Correctness | Independent review of the protected core | UNVERIFIED (AI second look only; no human) |
| Correctness | Mutation or model evidence that fencing tests fail without the change | DESIGNED BUT NOT IMPLEMENTED |
| Durability | Issuer-crash-safe invalidation | PROTOTYPE ONLY (sandbox; default would stay off) |
| Distribution | Multi-host run on real network | BLOCKED |
| Distribution | Larger than four nodes, WAN | UNVERIFIED (not run); WAN deferred |
| Security | Ingress hardening (auth before body, timeouts, connection limits, rate limits) | PARTIAL; G4B-01, -02, -11 open |
| Security | Cluster peer authorization per scope | DESIGNED BUT NOT IMPLEMENTED (DR-027) |
| Security | JWT lifecycle (`nbf`, max lifetime, revocation) | PARTIAL |
| Security | Dependency advisory scan and SBOM | DEFERRED |
| Security | External or human security review | BLOCKED |
| Performance | Release-profile suite, repeated measurements with intervals | DEFERRED |
| Performance | Any published latency or throughput number | none made; the project declines to publish one |
| Operations | Graceful drain, health endpoints | health endpoints TESTED (`tests/integration/m7_health_tests.rs`); drain DEFERRED |
| Operations | Metrics export, config reload, TLS reload | DEFERRED |
| Operations | Linux and container packaging | UNVERIFIED |
| Operations | Idle CPU behaviour | PARTIAL (observed high; cause hypothesised) |
| Tenancy | Quotas and noisy-neighbour isolation | DESIGNED BUT NOT IMPLEMENTED |
| Observability | Every UI value labelled by provenance | TESTED (static scan `check-forbidden`, DOM tests); visual and usability acceptance open |
| Process | Everything committed and pushed | No (nothing beyond four documentation or tooling commits) |
| Process | Formal owner acceptance of milestones | Not recorded |
| Process | Two review cycles (G4) | PARTIAL (cycle 1 Rust only) |

## 8.11 Claims you must NOT make

| Tempting overclaim | Accurate statement | Evidence |
|---|---|---|
| "PhantomCache is production-ready." | It is a research prototype with an experimental cluster mode. It has no release-profile suite, no advisory scan, open High findings, and has been run only on Windows. | `docs/STATUS.md` sections 3 to 5b; `docs/user/limitations.md`; register 8.4 |
| "It is a distributed cache." | Each gateway has its own cache; the cluster adds an authenticated invalidation bus with repair. GET is served locally. Coordinated reads do not exist. | `docs/user/limitations.md`; product-direction 5.2; `src/dist/router.rs` |
| "DELETE returns 204, so the object is gone everywhere." | 204 means the canonical primary acknowledged within a 150 ms leg; other members are told asynchronously, and owed invalidations are lost if the issuer dies. | G4B-09; `docs/user/limitations.md` durability section; M3 reproduction |
| "Invalidations survive crashes." | Not in the tree. A sandbox patch with an opt-in log exists and is not integrated; the default would still lose them. | `m3_durability/remediation/00_status_and_plan.md` |
| "Zero stale reads after 50 ms" or "bounded staleness." | Not supported. Cold first use took 35 to 60 ms; the only gateway bound is the origin lifetime or 60 s, and the stale cap bounds stale serving only. Origin lifetimes have no ceiling. | `claims_ledger.md` C6 amendment; `freshness.rs`; `limitations.md` |
| "Linearizable" or "strongly consistent." | No. Per-node ordering and eventual convergence under the loopback failure model that was tested. No linearizability claim has a stated failure model, bound and falsifying test. | `CLAUDE.md` section 4; product-direction 7.4 |
| "Adaptive or intelligent caching improves performance." | Not established. The sealed simulation found H1 NOT_SUPPORTED; Scenario B gave TRADE_OFF_FAILURE for both arms. This shows no benefit for this controller on these plants, not that adaptation cannot work. | `m6_sealed_results.md`; `g1_verdict.md`; `scenario_b_interpretation_notice.md` |
| "The PIE is a working canary controller." | `src/control/pie.rs` is a legacy threshold stub with a literal utility; it is `pie: None` in `src/main.rs`. `PHANTOM_INTELLIGENCE=canary` applies the cap to all fetches with no rollback in the serving path. | `pie_audit.md`; `src/main.rs`; F14 |
| "It uses machine learning." | No ML ships. An ML selector is gated and its negative claim is itself unverified. | `claims_ledger.md` C12; `phantom_intelligence_phase.md` |
| "PhantomCache is fast" or "faster than Redis, Nginx or Varnish." | The repository contains benchmark results with a stated profile, host and limits; none establishes a release-profile or comparative claim, and this book makes no superiority claim. | Chapter 6, section 6.13; `claims_ledger.md` C3, C5 |
| "Eviction is 200 times faster" or "controller overhead is below 1% CPU." | Untraceable. The figures appear in completion reports and test literals; no release-build measurement backs them. | `claims_ledger.md` C3, C5 |
| "Independently audited or verified." | Reviewed by AI second looks. No human or external reviewer has checked anything. | ledger section 6; verification register |
| "Secure by design / hardened." | Authentication, roles, mTLS and SSRF filtering exist and are tested. Open: pre-auth body read, live-control mutation, member flush capability, no rate limits, no advisory scan. Not security-certified. | register 8.4.3; `m7_security_review.md` |
| "Multi-tenant isolation with quotas." | Tenant-keyed entries and tenant epochs only. No quotas; any authenticated member can flush any tenant. | 8.3.5; DR-027 |
| "`PHANTOM_DEFAULT_TTL_MS` sets the default lifetime." | Validated but not wired; the effective default is always 60 s. | `src/config.rs`; `src/api/server.rs`; `CLAUDE.md` section 5 |
| "SIEVE, LRU and S3-FIFO are benchmarked and superior." | Faithful implementations exist in `src/core/eviction.rs`; relative performance has no release-grade measurement; S3-FIFO has a tombstone defect (unreachable by default). | README; G4A-02 |
| "The UI is finished / accessible." | Increments I1 to I4 are implemented and pass automated checks. Visual quality is not approved, no screen-reader or human usability test exists. | `STATUS.md` M8 row; verification register V-07 to V-19 |
| "Everything is saved / committed." | Almost everything is uncommitted; one local backup on the same machine. | 8.2.1 |
| "Tests passing means it is correct." | 851 debug tests pass; several assert stub behaviour. | `CLAUDE.md` section 3; 8.2.2 |

## 8.12 Implementation versus roadmap matrix

Exactly one label per capability. "Why" gives the one reason that justifies the label. Reading rule used in every chapter and appendix: TESTED means IMPLEMENTED and a named test exercises it (Chapters 4 and 5 write this "IMPLEMENTED, TESTED"; it is the same label), and "IMPLEMENTED, PARTIAL" is written here as PARTIAL. No row is INDEPENDENTLY VERIFIED: nothing in this repository has been checked by a human or external reviewer.

| Capability | Status | Why |
|---|---|---|
| Single-flight coalescing | TESTED | Named integration tests; coalescing under cancellation reviewed only by reading |
| Generation fencing against DELETE | TESTED | `test_delete_fences_in_flight_fill_race`; a single interleaving |
| Tenant and global epoch fencing | PARTIAL | IMPLEMENTED, but the G4A-01 race is reproduced in a sandbox and its fix is not integrated; G4A-08 and G4A-17 open |
| Byte accounting (I1 to I8) | TESTED | `audit_bytes` equality tests; G4A-13 drift on a rare path |
| Freshness (RFC 9111 subset) | PARTIAL | `Age`, `Expires: 0`, stale-if-error scope gaps (G4A-04 to -06) |
| Default TTL setting | PARTIAL | Validated, not wired |
| Stale-while-revalidate, stale-if-error, hard stale cap | TESTED | Counter-based gate is weak (G4A-18) |
| SIEVE (default) and LRU eviction | TESTED | Weakly: pinned scans are not O(1) (G4A-20); no hit-ratio check against the literature; no release benchmark |
| S3-FIFO eviction | PARTIAL | IMPLEMENTED with the open defect G4A-02 (tombstones); not selected by default |
| Lazy sweeper | PARTIAL | Scans the shard under the lock; shutdown channel dropped (D-2) |
| Telemetry log-linear histogram | TESTED | Recording and bucket arithmetic only (`telemetry_redesign_tests`); the histogram is fed but has no production percentile reader (Chapter 6, 6.12.2) |
| HTTP ingress hardening | PARTIAL | G4B-01, -02, -04, -11 open |
| JWT roles, mTLS identity | TESTED | M7 security tests; lifecycle gaps open |
| SSRF upstream filter | TESTED | Range gaps (G4B-15) |
| Cluster invalidation fan-out, repair, anti-entropy | TESTED | IMPLEMENTED and tested on loopback, debug build, at most four nodes; labelled EXPERIMENTAL in `main.rs`; effectiveness UNVERIFIED; 204 is not a cluster-wide acknowledgement (G4B-09) |
| Durable invalidation | PROTOTYPE ONLY | Sandbox patch, not integrated |
| Graceful connection drain | DEFERRED | Integration item D-1 |
| Heartbeat-based failure detection (node state change) | DESIGNED BUT NOT IMPLEMENTED | Heartbeat frames and idle-connection keepalive are IMPLEMENTED (`pool.rs`), but no code path lets a heartbeat change node state (G4B-12) |
| Runtime membership and revocation | PARTIAL | Machinery, no production trigger |
| Rendezvous routing for DELETE primary | IMPLEMENTED | Unweighted, unstable hash |
| Peer-fill, coordinated reads, stable placement | DESIGNED BUT NOT IMPLEMENTED | A stub handler exists and is a defect, not the feature |
| Multi-host operation | BLOCKED | No devices |
| Tenant keyspace isolation | TESTED | Tenant-keyed entries and epochs |
| Tenant quotas, fair eviction | DESIGNED BUT NOT IMPLEMENTED | Frozen core |
| Rate limiting | DEFERRED | SEC-M7-11 queued |
| Static origin concurrency cap | TESTED | Scenario B and unit tests |
| AIMD cap controller | TESTED | Arithmetic and cap unit tests only; on the serving path only when a cap is configured in canary or active mode (default off); benefit NOT SUPPORTED |
| Signals, shadow evaluator, decision engine | PROTOTYPE ONLY | Not wired; unsound statistics (F12, F13) |
| Legacy PIE | PROTOTYPE ONLY | `pie: None` |
| Intelligence `canary` mode | PARTIAL | Cap only; no canary or rollback in serving |
| Hot-key detection (`HeavyHitterDetector`, Count-Min plus Space-Saving) | PROTOTYPE ONLY | Library in `src/intelligence/signals.rs`, unit-tested and used by the lab simulator's admission filter, not wired into the serving path; the older `CountMinSketch` in `src/intel/sketch.rs` is used only by a unit test |
| Event tap, pre-warming, forecasting | DESIGNED BUT NOT IMPLEMENTED | Experiments E0, E9, E10 first |
| ML selector | DEFERRED | Gate unmet; negative claim unverified |
| Lab API and Scenario B | TESTED | Exploratory; U-01 defect open |
| Sealed M6 evaluation | TESTED | Executed once, Tier 1 simulated, not human-reviewed |
| T1 to T2 (simulator to real) agreement | UNVERIFIED | No agreement study run |
| Release-profile performance measurements | DEFERRED | Needs a quiet machine |
| Dashboard UI I1 to I4 | TESTED | Automated checks only |
| Visual acceptance of the UI (O8-2) | UNVERIFIED | Owner decision open |
| Demo Center and `demo-1` stream | DESIGNED BUT NOT IMPLEMENTED | Proposal only |
| Human review of design, usability, security | BLOCKED | No human reviewer |
| Linux and container deployment | UNVERIFIED | Never run |
| Metrics export, TLS and config reload | DEFERRED | Not provided |
| Dependency advisory scan and SBOM | DEFERRED | Queued |

## 8.13 Preparing to defend the project

The judge boxes are grouped by theme. Their answers are written to be said aloud and to survive follow-ups. They share one rule: **never claim what the evidence labels forbid, and say so before the judge finds it.**

### 8.13.1 Honesty and verification

::: judge
**JUDGE MAY ASK**

Question: Who has independently verified this system?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No human has. Every review so far is an AI second look: a separate AI session or the lead re-running a command, all inside one tooling environment. We label them that way everywhere. What we have is breadth: 851 recorded debug tests, three read-only reviews that produced 99 findings, an independent Node recomputation of the Scenario B results, and a hash-checked freeze chain for the sealed evaluation. What we have not got is a human reviewer, an external security review, or a physical multi-host run. Those are listed as blocked in our register, waiting on reviewers and devices.

**If the judge asks further:**

"Doesn't an AI second look count?" It counts as evidence that a second reader found things the first missed, and it did: G4 found a High finding the authors had not. It does not count as independence, because the reviewers share training, tools and prompts with the authors. A human review is the next step.

**Evidence to know:** `docs/release/workstream_completion_ledger.md` section 6; `docs/release/verification_register.md` (independence classes); status label INDEPENDENTLY VERIFIED: none.
:::

::: judge
**JUDGE MAY ASK**

Question: Your tests pass. Why should I believe the system is correct?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

You should not believe correctness from passing tests, and our own rules say so. The 851 tests run in debug mode on one Windows machine. Some assert stub behaviour, for example the legacy PIE tests. The stale-cap counter is close to a tautology, and one UI fixture even masked a real defect. What the tests do give is regression protection and a set of invariant checks, such as byte accounting equality and generation fencing in single-process cases. For anything stronger we have named gaps: no mutation test showing fencing tests fail without the fence, no model checking, no release-profile run.

**If the judge asks further:**

"What is the weakest test?" The one behind the stale-cap violation counter (G4A-18), because, as the reviewer describes it, the counter is near-tautological (I did not re-derive that). The fix is to rewrite it so that it attempts a violation.

**Evidence to know:** `docs/research/code_findings.md` F3; G4A-18; G4C F01 fixture at `ui/test/lab.view.test.mjs`; `claims_ledger.md` section 4.
:::

### 8.13.2 Consistency and the distributed layer

::: judge
**JUDGE MAY ASK**

Question: A client sends DELETE and gets 204. Is that content guaranteed gone from every node?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

No. A 204 means the issuing gateway purged its own cache and the canonical primary for that key acknowledged within 150 milliseconds. That primary can be the issuer itself. The other members are informed asynchronously with retries, a repair worker and anti-entropy, but the list of invalidations still owed lives only in the issuer's memory. If the issuer is killed first, peers keep serving the old object until its freshness lifetime ends. We reproduced that on unmodified code. A fix with a durable log exists as a sandbox patch, not in the tree, and it is opt-in. So for deletion-critical content our documentation tells operators to set a short `s-maxage`, re-issue DELETEs after any restart, and verify by reading through each gateway.

**If the judge asks further:**

"What staleness bound do you offer?" None beyond the origin's lifetime, or 60 seconds when the origin gives none. We measured a warm loopback propagation P99 of about 10 to 14 ms in a debug build, but cold first use took 35 to 60 ms, so we withdrew the "zero stale after 50 ms" claim.

**Evidence to know:** G4B-09; `docs/user/limitations.md` "Cluster invalidation durability"; `m3_durability/reproduction_report.md`; `claims_ledger.md` C6 amendment; label PROTOTYPE ONLY for durability.
:::

::: judge
**JUDGE MAY ASK**

Question: Is this linearizable? What is your consistency model?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is not linearizable and we do not claim it. Each node orders its own fills and deletes using generation fencing, so a fetch that started before a delete cannot store its result after it. Across nodes the model is eventual convergence of invalidations under a loopback fault model we tested. Concretely: N independent caches and an invalidation bus. We have drafted numbered consistency clauses with a failure model, a bound and a test that could falsify each, but they are drafts waiting on owner ratification, and several are known to be unmet.

**If the judge asks further:**

"Is even the single-node fencing airtight?" Not proven. A reviewer found that the epochs are read twice in the fill path, so a pre-invalidation body can survive an acknowledged tenant invalidation if the invalidation lands in a nanosecond window. We reproduced it only with an injected yield; a 20-second natural stress run saw nothing. A three-line fix is proposed and not applied because the file is protected.

**Evidence to know:** G4A-01; `docs/research/g4a01_race/`; `src/core/cache.rs::finalize_fetch`; product-direction section 7.
:::

::: judge
**JUDGE MAY ASK**

Question: Can one tenant, or one cluster node, hurt another?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Yes, in two ways. First, tenants are separated by key and by invalidation epoch, but there are no quotas, so one tenant can consume the byte budget. Second, any authenticated cluster member can send tenant-scope or full-cache invalidation frames, and the receiver does not check scope against the sender's rights. That is finding DR-027 or G4B-05, and the owner has named it as one that must not be dismissed. The plan is per-member scope authorization and tenant-flood caps, in a sandbox, queued behind the durability work. It needs a valid cluster certificate, so the attacker is an insider or a compromised member.

**If the judge asks further:**

"What about a bad peer fabricating cache content?" The cluster `GetRequest` stub writes a placeholder value into the node cache. We reproduced it, and an AI review agreed that no HTTP request can reach that entry because the key spaces are disjoint. We rate it low as exploited today and medium as a latent hazard.

**Evidence to know:** G4B-05, G4B-30; `docs/security/investigations/DR-029_*`; `src/dist/node.rs` (the `val_node_` format); status PARTIAL for security.
:::

### 8.13.3 Intelligence and performance

::: judge
**JUDGE MAY ASK**

Question: The sealed evaluation says H1 is NOT_SUPPORTED. What does that mean for your "intelligent" caching claim, and what happens to the product?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It means we have no evidence that our adaptive controller beats a well-tuned static origin cap, so we do not claim that it does. In the simulated, pre-registered, run-once evaluation, the adaptive arm did not reduce peak origin concurrency in any of eight scenarios, and in seven it was credibly worse. The unconstrained tuning optimum was itself a static cap. Our research lead therefore chose the static gateway as the minimum viable product and did not promote the adaptive controller to the serving path. The negative result is kept on the record. It does not prove adaptation is useless; it shows no benefit for this controller under this protocol. Next steps are a headroom experiment to see whether any adaptive cap could beat a static one at all.

**If the judge asks further:**

"Was that run reviewed?" By the lead and AI reviewers for integrity of the hash chain, not by a human, and the reviewers did not read the simulation core. Also the P99 endpoint was quantized, and the stale-window mechanism was never exercised.

**Evidence to know:** `docs/research/m6_sealed_results.md`; `known_limitations_frozen_code.md` section 1; status label TESTED, tier SIMULATED.
:::

::: judge
**JUDGE MAY ASK**

Question: What are your performance numbers?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We do not publish a performance figure as a claim, deliberately. The repository does contain benchmark results with a stated profile, host and limits, and the book makes no superiority claim. The numbers in the repository come from simulation, loopback on one host, or benchmark runs whose build profile is not established; none establishes a release-profile or comparative claim, and some earlier figures, like a 200 times eviction speedup and a 0.45 microsecond controller overhead, cannot be traced to any retained artifact, so we withdrew them. There are micro-benchmarks with bootstrap intervals from 2026-10-28 and one release-profile loopback experiment (Scenario B), but there is no release-profile general suite and no soak run, and the micro-benchmarks predate the current tree. A performance claim here would need a release build, stated hardware, workload and seed, a baseline, repetitions and intervals (Chapter 6, section 6.13 lists every retained measurement).

**If the judge asks further:**

"Can you say it is fast?" We can say a hit is an in-memory map lookup behind a shard lock, and show the design. We cannot say how fast or compare it with Nginx or Varnish.

**Evidence to know:** `claims_ledger.md` C3, C5; `docs/user/limitations.md`; `CLAUDE.md` section 4; status DEFERRED.
:::

### 8.13.4 Security and attack surface

::: judge
**JUDGE MAY ASK**

Question: Where would a hostile reviewer attack first?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

At the HTTP ingress and the control endpoint. The ingress extracts request bodies before authentication, with no body timeout, no connection limit and no rate limit, so a slow-sender attack is plausible; the reviewers rated it High and the owner named it must-not-dismiss. `/api/control` parses its JSON before checking the token, and an unknown event sets every router node active. Second, the cluster: static membership, long-lived connections that never re-check certificates, and members who can flush every cache. Third, the supply chain: no advisory scan has been run on 288 locked packages. We have documented all of these. The mitigation today is a stated deployment assumption: put it behind a proxy with limits. We have not made it a proven property.

**If the judge asks further:**

"Did you test any of these exploits?" These findings are confirmed by reading code, not by running attacks. We re-opened the control handler and saw the extractor order ourselves, but did not execute a slow-body test. Runtime verification of the library defaults is still needed.

**Evidence to know:** G4B-01, -02, -03, -05 to -08, -11; G4C-DEP; `src/api/server.rs::control_handler`; status PARTIAL.
:::

### 8.13.5 Process, debt and the future

::: judge
**JUDGE MAY ASK**

Question: What would you do with more time?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

In order: first, get a human to review the protected core, because everything we know about it is AI second looks. Second, integrate the three sandbox fixes after review: the durable invalidation log, the fence fix for the double epoch read and the Scenario B cancelled-run fix, each with a test that fails without it. Third, close the ingress holes: authenticate before reading bodies, add timeouts and limits. Fourth, run a release-profile suite and the soak on a quiet machine, and run the cluster on real hardware over a network. Fifth, only then start tenant quotas and the headroom experiment for intelligence, because a quota built on an unreviewed fencing layer would sit on sand.

**If the judge asks further:**

"Why not just fix them now?" Because the protected core is frozen for evidence integrity and for safety: changes to fencing and invalidation need a failing-first test, an independent review and the owner's approval. We chose not to merge unreviewed changes to the part of the system whose correctness matters most.

**Evidence to know:** `docs/release/execution_register.md`; `docs/release/owner_decisions_register.md` ODR-2 to ODR-4; Figure 8.2.
:::

::: judge
**JUDGE MAY ASK**

Question: What have you chosen not to claim, and what is untested?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

We do not claim production readiness, a distributed cache, linearizability, bounded staleness, adaptive benefit, ML, performance numbers, or independent verification. Untested: any Linux run, any physical network, clusters beyond four nodes, power loss and fsync-error behaviour, a release-profile suite, a two-hour soak, the natural-schedule rate of the fence race, the real behaviour of ingress defaults under attack, human usability and screen readers. Also unverified: whether the simulator ranks scenarios the way a real process would, because no simulator-to-real agreement study was run.

**If the judge asks further:**

"Which of those worries you most?" The unreviewed fencing and invalidation core, because the product's one hard promise is that a deleted object is not served after an acknowledged delete.

**Evidence to know:** 8.10 checklist; 8.11 table; `m3_durability/remediation/00_status_and_plan.md` section 7.
:::

::: judge
**JUDGE MAY ASK**

Question: Your repository is mostly uncommitted. Isn't that a process failure?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

It is a delivery risk and we list it first. The cause is a deliberate policy: commit only file groups that have been individually reviewed, never everything at once, and never groups with unresolved blocking findings. Documentation and tooling groups are committed; the code groups wait for the review gate and an owner check on held preregistration files. The risk is real: one machine and one local backup. Mitigation is to commit reviewed groups as they clear and to make an off-machine copy, which is still an open action.

**If the judge asks further:**

"Is anything pushed?" No push has occurred. Commit and push need an explicit instruction from the owner.

**Evidence to know:** `docs/release/owner_decisions_register.md` ODR-3; `docs/STATUS.md` section 5b; `git log --oneline -8`.
:::

::: judge
**JUDGE MAY ASK**

Question: The README and an older handoff say the system passed a formal acceptance. Why do your records say otherwise?

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

Because those statements came from agent self-reports and were later audited. The handoff's "Phase 4 accepted, zero P0" is not supported by later review: no dated owner acceptance is recorded for any milestone, the M3 completion report said "zero compromises" before a durability defect was reproduced, and the M6 report was retracted. We kept the old documents as history, added correction notices, and made `docs/STATUS.md` and the verification register the entry points. In a defence we would show the register, not the old claims.

**If the judge asks further:**

"Could the owner have accepted milestones elsewhere?" Possibly. The register calls missing acceptance an evidence gap, not proof that none was given.

**Evidence to know:** `docs/handoff/CORRECTIONS.md`; `docs/STATUS.md` section 1; `CLAUDE.md` section 6.
:::

## Key ideas to remember

- The honest ceiling for any capability here is TESTED; nothing is INDEPENDENTLY VERIFIED by a human.
- PhantomCache is N independent caches with an invalidation bus. GET is local. A 204 is not cluster-wide proof.
- The 99 G4 findings are 75 open (six owner-named), 20 recorded limitations, 2 in sandbox, 1 under investigation, 1 refuted. The serving risk concentrates in ingress, cluster trust, fencing and freshness.
- The three fixes (U-01, G4A-01 proposal, M3) are sandbox patches, none committed, none reviewed by a human. U-01 is sandbox only with owner-approved integration (DEC-001) authorised and in progress; G4A-01 and M3 are not integrated.
- H1 is NOT_SUPPORTED in simulation. The product position is a static gateway; adaptive benefit is unestablished.
- Almost all engineering work is uncommitted. Old documents (handoff, README, parts of `CLAUDE.md`) are history.

## What you should now understand

- Why a passing test suite is weak evidence here, with two concrete examples.
- How to explain the difference between a distributed cache and an invalidation bus between local caches.
- What each of the six owner-named findings is, in one sentence each.
- Which statements in older documents are wrong now, and which record to trust instead.
- Why the roadmap is ordered by dependency, and what gate blocks each stage.
- How to answer "what would you not claim" without hedging.

## Knowledge check

1. List the status labels in order of strength, and name the one nothing in the project has earned.
2. What does a DELETE 204 mean in cluster mode? Name the in-memory structure that limits the guarantee.
3. Name the four themes where serving-path risk concentrates, and one finding in each.
4. Why is G4A-01 "UNDER-INVESTIGATION" and not "fixed" or "refuted"? What exactly was reproduced, and how?
5. What does `PHANTOM_DEFAULT_TTL_MS` do today?
6. Why can S3-FIFO's tombstone defect not affect the default serving path, and what did we not check?
7. What is the difference between RECORDED-LIMITATION and closed? Which owner decision created the status?
8. Which three sandbox patches exist, and what must happen before each can enter the tree?
9. What would a NO_HEADROOM result from experiment E2 change in the plan?
10. Give three statements in the handoff or `CLAUDE.md` that are outdated and the record that corrects each.

::: answers
1. IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED are the strong ones in that order; the weak or negative ones are PARTIAL, PROTOTYPE ONLY, DESIGNED BUT NOT IMPLEMENTED, DEFERRED, BLOCKED, UNVERIFIED. Nothing has earned INDEPENDENTLY VERIFIED.
2. The routed canonical primary (possibly the issuer) acknowledged within a 150 ms leg. Other members are told asynchronously. The issuer's ledger of unacknowledged invalidations lives only in memory, so a crash loses it.
3. Fencing/epochs (G4A-01), ingress and rate limiting (G4B-01), cluster trust (G4B-05 or -06/-07/-08), freshness semantics (G4A-04 to -06, -25).
4. The owner called it unconfirmed. It was reproduced only with an injected yield point between the fence check and the epoch re-read in a sandbox; a 20 s natural stress run (about 3.16 million fills) found no violation; the proposed fix is not applied.
5. Nothing at runtime. It is validated and reported only; the effective default is the literal 60,000 ms.
6. `set_eviction_policy` is called only from the unwired legacy PIE and the executor's `apply_policy`, which has no serving caller; the cache starts with its default policy. We did not search every caller exhaustively or run it.
7. RECORDED-LIMITATION means the owner chose to document the finding rather than fix it in frozen code; it stays open and nothing is closed. ODR-1 created it.
8. U-01 backend fix, G4A-01 fence proposal, M3 durable invalidation. Each needs independent review and owner approval; protected-core merges need final approval; failing-first tests are required for fencing and invalidation changes.
9. Adaptive caps are abandoned as a direction; the origin-protection advisor continues.
10. Handoff "Phase 4 formally accepted, 242 tests": `docs/STATUS.md` and `CLAUDE.md` section 6; README "cargo 1.75+": G4C F28; `CLAUDE.md` section 6 "evictors are scans": `src/core/eviction.rs` faithful policies and section 5 of the same file; also "64 partitions": `src/main.rs` starts 8 shards.
:::

## Codebase exploration tasks

1. **Trace the double epoch read.** Start at `src/core/cache.rs::finalize_fetch`. Find the fence check and the insert block, and note where `get_effective_epochs` is called each time. Then read `docs/research/g4a01_race/analysis.md` and `proposal.patch`. You should learn why the shard mutex does not order the epoch bump, and why the lock order shard then `tenant_epochs` is not itself violated.
2. **Follow a DELETE through the cluster.** Start at `src/api/server.rs` (the DELETE handler) and `src/dist/router.rs` (`route_key`, the repair ledger, the broadcaster). Find where the 150 ms leg is set and where unacknowledged invalidations are stored. You should learn exactly what a 204 asserts and where the in-memory debt sits, then compare with `docs/research/m3_durability/design.md`.
3. **Audit one disposition.** Pick any open finding from `docs/release/g4/cycle1_findings_disposition_register.md` (try G4B-13). Open the cited file, decide whether the finding still holds, and write down what you checked and what you could not check. You should learn the difference between "confirmed by reading" and "reproduced".

## Challenge questions

1. A judge says: "You admit the epoch bug is real, yet you ship the cache. Defend that." Give a technically honest answer and say what you would add to the test suite.

   *Model answer:* We do not ship it; it is a research prototype and the finding is open. The bug window is two reads of an epoch counter in one critical section while the epoch writer does not take the shard lock, so it needs an invalidation to land in a nanosecond window and the stored body to be pre-invalidation. It was reproducible only with an injected yield, and a natural stress run found nothing, so frequency is unknown, not zero. I would not claim zero. To add: a deterministic test hook (permanent, behind a test-only feature) that forces the interleaving and asserts the body is not readable after an acknowledged invalidation; a randomised interleaving or loom-style model of the fill and invalidate paths; and a mutation test that removes the fence to prove the tests fail. The fix itself (stamp the epochs captured at the fence check) is small, which is exactly why it needs independent review before it touches the fencing core.

2. Design the minimum evidence package that would let you move the durable invalidation log from PROTOTYPE ONLY to TESTED-and-integrated, and say which part would still not be INDEPENDENTLY VERIFIED.

   *Model answer:* Integrate the patch only after an independent reviewer works from the review brief; keep the crash-point matrix (kill at each defined point, restart, assert convergence) in the suite behind its feature flag; add a Linux run for the signal and fsync-error assumptions; add a power-loss or filesystem-fault test or declare the durability claim limited to process crashes; expose log statistics through telemetry with provenance labels; decide the open policy questions (fail-open or closed, fsync default, drain deadline, age-out) and record them; make the log default-on only after a measured release-build cost. The artefacts that remain unverified are the review itself (if done by AI), the OS-crash durability and the multi-host behaviour. A human reviewer and a physical run are the only ways to change that.
