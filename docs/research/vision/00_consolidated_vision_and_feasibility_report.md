# PhantomCache: Consolidated Vision and Feasibility Report

**Status: RESEARCH / PROPOSAL. Not a plan of record. Nothing here is approved, implemented or run.** Date: 2026-10-30. Author: integration writer (one pass, read-only on code). Written for the product owner.
Tree state at the time of writing: HEAD `456bb25`, branch `feat/publish-phantomcache-core`, 72 dirty status entries (read only; no file other than this one was written; no cargo, git mutation, browser or web search was used in this pass).

**What this document is.** Five research agents (V1 intelligence, V2 forecasting and evaluation, V3 topology/tenancy/identity, V4 security and failure scenarios, V5 Demo Center UX) each wrote a long report in this folder (about 340 KB together). This report integrates them: it leads with conclusions, resolves where they disagreed, checks their most load-bearing code claims against the source, adds my own critique, and ends with a numbered list of decisions for you.

**How claims are tagged** (the project rule in `CLAUDE.md` section 4):

| Tag | Meaning |
|---|---|
| `[V file:line]` | Verified by me in the source in this pass |
| `[R-Vn §x]` | Reported by agent document Vn, section x; I did not re-derive it |
| `[L-R]` | Literature as reported by V1/V2/V4 (bibliographic or snippet level). I did **no** web search, so none of it is verified by me |
| `[A]` / `[H]` | Assumption / hypothesis (may be false; most have a stated way to be falsified) |
| `[P]` | Proposal (a design choice, not a fact) |

**Reading guide.** Busy: read section 0 (executive summary) and section 13 (decisions). Evaluating the vision: sections 1-4, 14, 18. Planning work: sections 10-12, 17. Checking my honesty: Appendix A (what I verified in code) and section 3 (what is unverified).

---

## Table of contents

0. Executive summary
- R. How the five reports disagreed, and how I resolved it
1. Intelligence opportunity map
2. Ranked predictive caching concepts
3. Prior-art and differentiation assessment
4. Proposed intelligence architecture
5. Multi-device demonstration architecture
6. Multi-tenant and identity model
7. Security demonstration catalogue
8. Demo Center UX architecture
9. Single-machine vs multi-device comparison
10. Phased implementation roadmap and dependency order
11. Dependencies and technical risks
12. Measurable acceptance criteria
13. Decisions requiring product-owner approval
14. The flagship demonstration (stages A-F, "Launch Day", interim version)
15. Capability classification table
16. What we will not claim
17. Near-term priorities under the current gates
18. Integrator's critique, and better ideas (clearly my view)
- Appendix A. Code-claim spot-check (verified / not verified / wrong)
- Appendix B. Terms

---

## 0. Executive summary

**What should PhantomCache ultimately become?**
A **deterministic, auditable, multi-tenant caching gateway whose automation is bounded, explainable and rehearsed in advance** [P]. The gateway (cache, single-flight, fenced invalidation, hard caps) is always authoritative. "Intelligence" is a layer that observes, advises and, only after pre-registered evidence, makes small reversible adjustments inside limits that it cannot raise. It never bypasses correctness, freshness, tenant isolation or the hard caps. The product's story is not "an AI cache"; it is "a cache you can trust, rehearse against failure, and audit, with automation that has to earn each privilege".

**Strongest differentiators** (all hypotheses; **none is claimed novel**, see section 3):
1. **Truthful evidence**: every displayed number carries a provenance label (measured / simulated / derived / unavailable) and negative results are retained and shown with equal weight. The repo's own Scenario B outcome (`TRADE_OFF_FAILURE`) is the credibility asset `[R-V5 §1.1]`.
2. **Hard-cap envelope**: intelligence proposes, the executor disposes; stale cap, origin cap, byte budgets, fencing and tenant epochs are not tunable by any intelligence decision `[R-V1 §0]`.
3. **Deterministic, replayable lab** that runs the real cache code against scripted origin behaviour, so operators can rehearse a brownout or a config change before it happens (a "game day" product) `[R-V1 §11.4]`.
4. **Generation-fenced, tenant-scoped invalidation with a stated failure model**; the same family as known techniques (leases, soft purge), so a trust property, not a novelty.
5. **Declared demand as an authorised, expiring, audited object with a measured track record** (the "Demand Calendar"). I found no source for it in the five reports' searches, which is absence of evidence only.

**Worth building now** (dependency-ready or prerequisite; each needs the approvals in section 13):
- A **single bounded, tiered per-request event tap** (one engineering problem serving the intelligence observation layer, the Demo Center stream and tenant counters) [R-V1 §11.1, V5 §4].
- **Per-tenant resource accounting and quotas**. Today only the *namespace* is isolated; capacity, eviction and the origin cap are shared, verified in code (Appendix A #1). This is a prerequisite for every tenant-level feature, and arguably worth more than any forecaster.
- **Durable invalidation (M3)**: a 204 on DELETE is not proof of cluster-wide removal if the issuer dies `[R-V4 §6.3]`; the fix exists only as an unintegrated scratch patch.
- **Truthful Observe mode**: real hot keys (today `hot_keys` is hard-coded "unavailable", `src/api/server.rs:766` `[V]`), regime and flash-crowd alarms, staleness-risk estimates. No actuation.
- **Event-driven bounded warming** (a deterministic "warm/refill these declared keys within a budget" feature), prototyped first as an out-of-process script that needs no gateway change.
- **Demo spine without new backend**: a launcher script, a preflight, recorded-run playback, the existing Scenario B evidence, and a security evidence runner.

**Worth researching** (must beat a stated simple baseline first): seasonal and Bayesian demand forecasting, early flash-crowd classification, origin-pressure forecasting, shadow-selected eviction, per-tenant reallocation, restart warm-up planning. The first gate is a **headroom test** (can even an oracle beat a plain "cron warm"?) before any forecaster is built `[R-V2 §6.8]`.

**Not worth pursuing now:** learned eviction (LRB-style), deep-learning popularity forecasting, RL cap controllers, an LLM in any decision path, intelligence-controlled TTL extension, auto key normalisation, auto-tuning of hard caps, shared cross-tenant models, "AI-powered" or novelty marketing. Reasons in section 16 and `[R-V1 §8]`.

**The honest bottom line.** The decisive blockers to a convincing multi-tenant, multi-node demonstration are **not intelligence**: per-tenant quotas, durable invalidation, per-request observability, a token minter, cluster PKI tooling, and a real multi-device rehearsal that has never been done. Intelligence built before them polishes a prototype. The flagship demonstration (section 14) can be run truthfully on current code in an interim form that is mostly *real mechanisms plus clearly labelled recordings*.

**Top owner decisions** are in section 13; the ten that matter most are D-01 to D-10 there.

---

## R. How the five reports disagreed, and how I resolved it

Format: topic; positions (with section); my resolution. "Verified" means I checked the code (Appendix A).

| # | Topic | Positions | Resolution |
|---|---|---|---|
| R1 | What intelligence to build first | V1 §4 ranks truthful Observe (hot keys, regime alarms) first, then the Demand Calendar and the Advisor. V2 §11.3 says the first product-grade feature should be an **event-driven bounded warm/refill API** with forecasting optional. V5 §6.1 Phase 0 builds no intelligence at all. | Sequence, not rank: event tap, then Observe, then deterministic warm/refill (V2's reframing), then forecasting only if the headroom test passes. V5 Phase 0 runs in parallel because it needs no backend. |
| R2 | Is forecasting the lever? | V1 C1 calls the calendar a prototype candidate and C2 (seasonality) a research hypothesis. V2 §5 predicts calendar-literal warming will capture most of the gain. | Adopt V2: forecasting must beat "warm the declared set at start minus lead" (baseline B3) by a pre-registered margin or be dropped. |
| R3 | Where the warmer lives | V1 §3 C1: an in-gateway scheduler reusing the fill path. V2 §4.1: out-of-process warmer using ordinary GETs first (prototype), in-gateway later. | Out-of-process first. In-gateway only after E9/E11 evidence and an independent review (it touches the fenced fetch path). Limit stated: out-of-process warming only works for content an anonymous request can fetch `[R-V4 §5.2]`. |
| R4 | Where pairing lives | V3 §3.3: separate pairing service (script) that mints tokens. V5 §3.2: a "view ticket" endpoint on the session API. | Prototype as an external script (V3). A gateway-side ticket route only after a security review. Guests are read-only either way. |
| R5 | Where the Demo Session object lives | V3 D12: gateway routes vs launcher. V5 §4.7: a separate session resource beside the Lab API. | In the Control Center / Lab API process, never on the cache hot path. Gateways stay session-unaware. |
| R6 | Are "devices" participants or viewers? | V3: devices are clients and nodes (participants). V5 §3.2: devices are views over one session. | Both exist; section 5 names two device kinds: *participant devices* (clients, nodes) and *view devices* (audience, observer). |
| R7 | Tenant isolation: defence demo or limitation demo | V4 A4 rates cross-tenant leakage prevention "Existing"; V4 S-10 and V3 §1.1.5 say capacity isolation does not exist; V5 §1.1 says "reframe". | Split into two: **confidentiality isolation** (demonstrable today) and **capacity isolation** (a measured limitation until quotas exist). Verified: no quota code (Appendix A #1). |
| R8 | Node failure and "failover" | V4 A8's source matrix expects "replica serves in <2 ms"; V3 §1.4 and V4 §6 say GET is local-only and `NODE_FAILURE` is a flag. | Read failover is **Unsupported**. Verified: `NODE_FAILURE` only sets node id 2 to Failed, `src/api/server.rs:1089` `[V]`. What can be shown: dead node's clients lose service, peers unaffected, owed invalidations repaired, and the issuer-kill defect. |
| R9 | Eviction in the serving path | V1 §1: "probably still the internal enum policies; not confirmed". `CLAUDE.md` §5 says Faithful policies; §6 says evictors are scans. | **V1 was wrong; `CLAUDE.md` §6 is outdated.** The serving shard holds `Box<dyn ShardEvictionPolicy>` built by a factory returning FaithfulSieve/Lru/S3Fifo, `src/core/cache.rs:209,223`, `src/core/eviction.rs:690-698` `[V]`. Whether those are truly O(1) was not re-verified. |
| R10 | LAN transport security | V3 §3.4 allows plain HTTP with JWT on a presenter-owned network as the default; V4 §9 forbids saying "secure" without bounds; V5 §3.2 requires JWT for any cross-device use. | JWT always. TLS with a pinned fingerprint preferred. Plain HTTP only on a presenter-owned private network with a permanent "LAN-INSECURE" banner and short-lived tokens. |
| R11 | Label vocabulary | V2 adds `SYNTHETIC_FORECAST_HARNESS` and `DECLARED`; V3 adds `mode_label` values; V5 asks for a multi-process tier. | One registry (section 8.4): the existing tiers plus `SINGLE_HOST_MULTI_PROCESS`, `LAN_MULTI_DEVICE`, `VIRTUALISED`, `DECLARED` provenance. Needs your approval (D-26). |
| R12 | Duplicate event streams | V1 L1 observation ring; V5 domain events; V3 tenant-attributed counters. | One bounded tap with tiered consumers: aggregate-only for observers, tenant-scoped for tenants, raw only for operators and lab (V1 §11.1 item 6). |
| R13 | Hot-key sketch scope | V1 §1 uses a global `WorkloadPipeline`; V2 §11.2 and V4 §5.2 say a shared sketch leaks cross-tenant per-key frequency. | Per-tenant sketches, tiered by tenant size (V2's memory arithmetic: one 16 KiB sketch per tenant exhausts a 16 MB arena at 1,024 tenants). Any global top-k is operator-only. |
| R14 | Experiment order | V1: E0 overhead, then E2 headroom; V2: E9 oracle-warm before any forecaster; V5: UI browser pass first. | Parallel tracks that gate different things (section 10). None blocks the others. |
| R15 | Cluster-wide warming | V1 X1 (peer-assisted warm-up), V2 §11.1, V4 §5.2. | All agree it is blocked on durable invalidation; warming stays node-local. |
| R16 | Evidence integrity format | V3 session log with sha256; V4 hash chain plus optional Ed25519 (`ring`); V5 report ids and hashes. | One bundle format: hash-chained `events.jsonl`, manifest of file hashes, optional signature, redaction before hashing (section 7.3). |
| R17 | `key_hash` always 0 | `pie_audit.md` says so; V1 §1 says outdated. | V1 is right: GET computes `fnv1a_64` of the canonical key, `src/api/server.rs:1255-1262` `[V]` (read of the surrounding lines). |
| R18 | "UI never rendered in a browser" | V5 header and R9 say so. | Out of date. The progress ledger records browser verification rounds on 2026-10-30 (D-7 round 2 at 11:16, round 3 dispatched) `[R: progress_ledger tail]`. V5's Phase 0 browser gate is already partly in flight. |
| R19 | Audit store use | V1 §1: "not re-checked whether anything appends". | Checked: `AuditStore::append` is defined at `src/intelligence/audit.rs:33`; no call site outside that file exists in `src/` (grep) `[V]`. The store is created in `main.rs:480` and never written in production. |
| R20 | Sizing of the demo | V5 lists seven playgrounds; V5 §9 item 10 admits it never saw the owner's list. | Unresolved: I do not have the list either. I follow V5's "three lenses on one scenario model" and ask for the list (D-39). |

---

## 1. Intelligence opportunity map

### 1.1 Ground truth today (all `[V]` unless marked)

- The intelligence code is a **library that is not wired into the serving path**. `WorkloadPipeline`, `ShadowEvaluator`, `HeavyHitterDetector` and `route_bucket` are referenced only inside `src/intelligence/` (grep). `pie: None` at `src/main.rs:433`; legacy `src/control/pie.rs` hard-codes `expected_utility: 85.0` (line 121). The `AuditStore` exists but is never appended to in production (R19).
- `hot_keys` is hard-coded `{"src":"unavailable","reason":"not_implemented"}` (`src/api/server.rs:766`).
- The only adaptive-control experiment is **negative**: an absolute-ceiling AIMD cap did not beat the best static cap in simulation (H1 NOT_SUPPORTED) `[R-V1 §0]`. The accountability report's own reading: this is neither evidence that intelligence works nor that it cannot `[R-V1 §0]`.
- Effective default object lifetime is a **literal 60 s**: `PHANTOM_DEFAULT_TTL_MS` is declared at `src/config.rs:116` but the freshness call sites pass `60_000` (`src/api/server.rs:1405`, `:1688`) `[V]` (line numbers have moved from those in `CLAUDE.md` §5).
- The gateway writes no request log; telemetry is aggregate and not tenant-attributed (`[R-V3 §1.1.7]`; the 100 ms SSE snapshot at `server.rs:959-968` `[V]`).
- There is **no real traffic trace** in the repo; every workload is synthetic `[R-V1 C2, V2 §1]`.

### 1.2 Where intelligence could help (consolidated, V1+V2)

| Area | Real problem | Strongest non-intelligent baseline | Class (section 15 scale) |
|---|---|---|---|
| Declared demand / preparation | First requests of a known event hit an empty cache; single-flight coalesces only per key | Operator cron warmer plus long TTL/SWR | Prototype candidate (deterministic core) |
| Truthful observation (hot keys, regime, change alarm) | Operators cannot see what is hot; nothing downstream is trustworthy without it | Sampled exact counting | Existing as library; Prototype for wiring |
| Early flash-crowd and metastability detection | Distinguish "cached surge" from scan or collapse signature | Static threshold on rate or origin in-flight | Prototype candidate (Observe) |
| What-if advisor / cap sizing | Operators cannot rehearse cap/TTL/policy changes | Little's-law arithmetic | Prototype candidate |
| Staleness-risk estimate | No honest statement of staleness beyond TTL | Quote TTL | Proposed |
| Key-fragmentation detector | Same content under many keys wastes hit ratio | Operator knowledge | Hypothesis |
| Seasonal forecasting | Rhythm in daily/weekly demand | Seasonal-naive | Hypothesis (blocked: no real trace) |
| Origin-pressure forecasting | Warn seconds before saturation | Persistence / threshold | Hypothesis |
| Shadow-selected eviction/admission | Better hit ratio on some workloads | Best fixed policy per family | Hypothesis (crowded field) |
| Tenant reallocation | Noisy neighbour; per-tenant tuning | Static quotas | Hypothesis (blocked by missing quotas) |
| Adaptive TTL | Fewer revalidations | Static TTL plus SWR | Advisory only; actuation Unsupported |

### 1.3 Cross-cutting findings that reshape the map

1. **Intelligence depends on controls that do not exist** (V1 §11.1): no per-tenant quotas or tenant telemetry; dev mode trusts a client-supplied `x-tenant-id` (so any calendar or warm API there would be an unauthenticated fill-amplification endpoint); durable invalidation missing; no client identity forwarded upstream by default (warming is limited to public content); one origin per gateway; no per-request event stream.
2. **Distribution multiplies origin load.** The origin gate is process-wide (`src/intelligence/executor.rs:75` `[V]`) and GET is local-only, so N gateways can each fetch the same key and reach N times the cap `[R-V1 §11.2; derived]`. Any cap advisor must be cluster-aware. A "peer-fill / owner-fetch" read path is an open architectural question (section 18).
3. **Shared learning is a side channel** (V4 §5.2, V2 §11.2): shared popularity models, shared permit pools, and global audit aggregates leak cross-tenant load. State must be per tenant and endpoints tenant-scoped.
4. **The same detectors serve security** (V1 §11.2, V4 §5.1): rate, unique-key rate and prefix heavy hitters can alert on abuse. Observe-only; enforcement stays deterministic.

---

## 2. Ranked predictive caching concepts

Scores 1-5 (5 best) are **my integrated judgment `[A]`**, derived from V1's table and V2's reframing, not measurements. "Eng. value" is value if it works; "Feasibility" is near-term implementability given existing code and approvals; "Differentiation" is the plausibility it would distinguish the product (not a novelty claim); "Measurable" is whether the existing lab can falsify it; "Research" is the value of the knowledge produced including negative results. Sum is only a tie-breaker.

| Rank | Concept | Eng. value | Feas. | Differ. | Measurable | Research | Sum | Verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | **Event tap plus truthful Observe** (hot keys per tenant, regime and change alarms, origin health) | 5 | 4 | 2 | 4 | 3 | 18 | Build (foundation, not "intelligence" by itself) |
| 2 | **Event-driven bounded warm/refill** (declared keys, budgets, refusal log; includes purge-with-refill) | 5 | 4 | 4 | 4 | 3 | 20 | Prototype now (out-of-process), then build |
| 3 | **Phantom Advisor**: what-if replay, Little's-law cap sizing, game-day config check | 4 | 4 | 4 | 4 | 4 | 20 | Prototype |
| 4 | **Outcome ledger and calibrated decisions** (predicted vs realised, auto-demotion, paired block-bootstrap) | 4 | 4 | 3 | 4 | 4 | 19 | Build alongside 1 |
| 5 | **Staleness-risk estimator** (measured propagation latency distribution, stated failure model) | 4 | 3 | 3 | 3 | 3 | 16 | Build after durable invalidation |
| 6 | **Key-fragmentation detector** (advisory only) | 4 | 3 | 2 | 3 | 3 | 15 | Prototype in lab; needs real or realistic traffic |
| 7 | **Early flash-crowd and metastability early warning** (Observe) | 4 | 4 | 3 | 4 | 3 | 18 | Prototype after 1 |
| 8 | **Restart/post-flush warm-up planner** (node-local) | 4 | 3 | 3 | 4 | 4 | 18 | Prototype (shares scheduler with 2) |
| 9 | **Demand forecasting**: seasonal plus Bayesian blend of declared and observed demand | 3 | 2 | 3 | 3 | 4 | 15 | Research, gated on headroom test E9 and a real trace |
| 10 | **Hot-key refresh-ahead actuation** | 3 | 3 | 2 | 4 | 3 | 15 | Research |
| 11 | **Origin-pressure forecasting** | 3 | 3 | 3 | 3 | 3 | 15 | Research, needs a load-dependent origin plant (E1) |
| 12 | **Shadow-selected eviction/admission and strategy selection** | 3 | 3 | 1 | 5 | 3 | 15 | Research (crowded; the serving cache already uses faithful policies, R9) |
| 13 | **Per-tenant reallocation** | 3 | 1 | 3 | 3 | 3 | 13 | Blocked until quotas exist |
| 14 | **Advisory Cache-Control/TTL recommender** | 3 | 4 | 1 | 3 | 2 | 13 | Low priority; actuation Unsupported |
| 15 | **Deep-learning or learned-eviction models** | 2 | 1 | 1 | 2 | 2 | 8 | Do not pursue now (section 16) |

Why the ordering differs from V1: V1 ranked the Observe layer and the calendar tied at 19. After V2's analysis I rank the *deterministic* warm/refill feature above the *forecast-driven* one because the former is testable, auditable and likely captures most of the gain (R2). The honest expectation to plan for is that the forecaster adds little over a schedule `[R-V2 §5]`.

**One structural caveat I add** (section 18): with the effective default lifetime fixed at 60 s, warming far ahead of an event is not sensible (holding K keys warm for D seconds costs about K x D / TTL origin fetches `[R-V2 §4.2]`; K=1,000, D=1 h gives 60,000 fetches). Predictive preparation is mainly valuable for objects whose origin sets long `max-age`, which narrows the benefit zone and is a reason the headroom test must come first.

---

## 3. Prior-art and differentiation assessment

**Coverage statement.** V1 ran 20 WebSearch queries and V2 about 8 and V4 1; all at abstract or snippet level; no full texts; the search tool is US-focused and indexes blogs better than proceedings; patents were not read; the repo's earlier pass (`intelligence_prior_art_coverage.md`, 52 rows) is reused. **I ran no searches.** Everything below is therefore `[L-R]`. Absence of results is not evidence of absence. The open task RT-0 (a systematic proceedings sweep) remains open.

### 3.1 What incumbents already do (as reported)

| Incumbent / work | What was reported | Source status | Implication for PhantomCache |
|---|---|---|---|
| Varnish | Backend health probes, directors, grace mode (serve expired when backends are down), request coalescing; post-purge warming is done by external scripts | Probes/grace from the Varnish users guide via search `[L-R V1]`; warming claim from community pages only `[L-R V2, unverified]` | "Uses origin health to serve stale" is table stakes. Not a differentiator unless measured and cap-bounded and audited. Multi-backend directors are absent here |
| Fastly | Soft purge marks content stale rather than deleting; surrogate-key purge | Documentation snippet `[L-R V2 §11.4]` | "Purge without a miss storm" is established |
| Cloudflare | Tiered Cache; Cache Reserve (persistent storage); a "Prefetch URLs" feature that appears visitor-side | Snippets; Prefetch interpretation unverified `[L-R V1]` | Persistence and tiering are real operator value PhantomCache lacks (in-memory only) |
| Caffeine / W-TinyLFU | Window/main split adapted online by hill climbing; refresh-after-write | Wiki and maintainer comments `[L-R V1]` | "Adaptive eviction" is not a differentiator |
| Redis | Server-assisted client-side caching: tracking mode and prefix broadcast with a bounded table | redis.io snippet `[L-R V1,V2]` | Server-pushed invalidation with bounded state is a known pattern |
| Netflix Open Connect | Proactive off-peak fill driven by predicted demand | Vendor overview via search summary `[L-R V1 P2]` | Pre-positioning is established, in a different setting (finite catalog, ISP appliances) |
| Memcache at Facebook (leases) | Stale-set prevention | Prior pass `[L-R]` | Generation fencing is a re-derivation, not novel |
| XFetch (probabilistic early refresh), SIEVE, S3-FIFO, TinyLFU, ARC, LRB, GL-Cache, CACHEUS, Darwin, Memshare, Cliffhanger, SHARDS, count-min, Space-Saving | Bibliographic records matched at abstract level | `[L-R]` | Crowded fields; nothing here should be presented as new |
| Prophet, conformal prediction (CQR, adaptive conformal), PBS, Cho and Garcia-Molina, MASE, proper scoring rules | Method references | `[L-R]` | Methods to use, not claims |

### 3.2 What remains unverified (do not assert either way)

- Whether any CDN or cache offers **operator-declared, tenant-scoped, expiring, audited demand entries with safety budgets and a reliability track record**. V1 and V2 found no source in their searches (coverage gap).
- Whether Akamai, CloudFront or Fastly offer scheduled or API-driven prefetch from the origin (conflicting secondary sources).
- Cache-key fragmentation and key-hygiene tooling in incumbents (not searched).
- Cache-side short-horizon origin-pressure forecasting (none found).
- Per-tenant memory isolation in Redis/Varnish (an absence in search results only).
- Post-flush warm-up scheduling for in-memory gateways (not found).

### 3.3 Honest differentiation statement

PhantomCache is **not** ahead of incumbents on warming, soft purge, tiering, persistence, eviction or stale serving; on several of these it is behind (it lacks persistence, multi-origin, dynamic membership). The plausible differentiators are *process and trust properties* (section 0), and the packaging of a deterministic lab with provenance labels. Whether competitors offer comparable rehearsal tooling is **unverified**. No positioning claim should be made before RT-0 (D-38).

---

## 4. Proposed intelligence architecture

### 4.1 Principles (the rules intelligence cannot break) [P]

| # | Rule |
|---|---|
| I-1 | **Intelligence proposes, the execution layer disposes.** Every action is a typed, clamped parameter proposal validated by the executor. |
| I-2 | Intelligence cannot raise or bypass: `PHANTOM_MAX_STALE_MS`, the origin cap, byte and entry budgets (I1-I8), generation and tenant-epoch fencing, the never-evict-in-flight rule, tenant isolation. Hard caps are never auto-tuned. |
| I-3 | Intelligence cannot construct cache entries. A warm is an ordinary fill through the normal fetch path so fencing applies. |
| I-4 | Freshness: intelligence may shorten but never lengthen beyond origin directives or the hard cap. |
| I-5 | All intelligence state is bounded (counts and bytes), tenant-scoped and expiring. Shared state across tenants is limited to aggregate hyper-parameters. |
| I-6 | Every input and output carries provenance (measured / derived / simulated / declared / unavailable). `Simulated` or `Unavailable` inputs force "no action". `Declared` may inform priors and advice, never gate hard caps. |
| I-7 | Every automatic action records why it was *not* taken (named refusal codes); every actuated decision is auditable and reversible. |
| I-8 | Write paths (calendar, warm, policy) exist only in secure mode, scoped to the verified tenant, role-gated (never Observer). |
| I-9 | Mode Off is identical to the static path. Per-actuator kill switch, watchdog, auto-revert. |
| I-10 | Slow loops do not feed fast loops without a change alarm; one writer per actuator. |

### 4.2 Layers

```
 Operator --(declared: calendar entries, caps, envelopes, profiles)--------+
                                                                           v
 L0 DETERMINISTIC GATEWAY (authoritative, always on)
    cache, single-flight, freshness, fenced invalidation, tenant epochs
    HARD CAPS (stale, origin, bytes/entries) + executor: validate, clamp, apply, revert
        ^ one bounded event tap (drop-newest ring, tiered consumers)    | typed, clamped
        |                                                               | proposals only
 L1 OBSERVE   per-tenant sketches, rate/CUSUM, regime, origin health, staleness-risk
 L2 ADVISE    forecasts with intervals, what-if answers, cap sizing, calendar dry-run,
              capacity text; humans decide; everything logged
 L3 SHADOW    "would have warmed/evicted" ledger on live traffic, paired comparison, no exposure
 L4 ACTUATE   canary then bounded active, per actuator, only after its evidence gate
 OFFLINE      lab replay, pre-registered evaluation, model/forecaster selection, calibration
```
This keeps V1's L0-L4 stack and V2's progression ladder. The single event tap feeds L1 and the Demo Center (R12).

### 4.3 Actuator privilege tiers [P]

| Tier | Examples | Condition |
|---|---|---|
| Never intelligence-controlled | Hard caps, tenant isolation, fencing/epochs, key canonicalisation, byte accounting | By construction |
| Advisory forever | TTL changes, tenant quota changes (initially), ML-derived choices, key normalisation, Cache-Control recommendations | Humans apply |
| Eligible for L4 after evidence | Bounded warm fills (declared keys), hot-key refresh-ahead within existing background slots (default 128, hard ceiling 256 `[R-V1 §1]`), eviction policy among faithful implementations, cap value inside an operator [min,max], temporary stale extension within the hard cap | Pre-registered result with CI above the minimum effect, no adverse scenario, T2 confirmation, overhead within budget, deterministic replay, zero cap violations |

### 4.4 Demand Calendar (the declared-demand input) [P, from V2 §2]

A calendar entry is **untrusted, bounded, authorised, expiring input**: one tenant, a bounded selector (explicit keys / prefix / tag set, with hard match limits), a window with optional recurrence, the operator's own uncertainty interval, permitted actions, per-entry budgets (bytes and origin fetches that can only be at or below global and tenant caps), source, approver, hard expiry, auto-revoke after failed materialisations. States: draft, approved, active, expired, revoked, quarantined. Provenance label `DECLARED`. Abuse cases (inflated multiplier, huge selector, self-renewing recurrence, cross-tenant selector, replayed approval, DST confusion, 100x fat-finger) each become a test.

Combining declared demand with history (V2 §2.3, hypothesis RH-F1): treat the declaration as a Gamma prior on the uplift, discount its strength by a learned per-source reliability (Beta posterior of "materialised within interval"), update with observed counts during the window. Before onset there is no data to correct a wrong declaration; only the track record and the hard budgets protect.

### 4.5 Bounded decision layer (from V2 §4)

- **Actions:** A1 warm; A2 admission preparation (weak evidence); A3 refresh-ahead with staggering; A4 capacity recommendation (advisory, lowest risk); A5 alert/no-op.
- **Budgets enforced by the executor:** warm fetches/s below the lesser of a headroom estimate and a hard rate cap; warm fetches use the same origin permit gate at **lower priority** than organic misses (priority does not exist yet) and are dropped, not queued, when the gate is saturated; byte budget (inserts only into free space or displacing provably colder entries); per-tenant budgets (not enforceable until quotas exist); the stale cap is untouched (warm requires a fresh fetch); a **regret governor** demotes sources and the action class when waste exceeds a bound.
- **Sixteen refusal codes** (V2 §4.3), including: entry not active; selector too big or cross-tenant; input provenance Simulated/Unavailable; forecast UNCALIBRATED; origin degraded; any stale-cap violation observed; cache byte pressure; budget exhausted; low source reliability without corroboration; regret governor tripped; insufficient lead time; kill switch; unknown object size; conflicting entries; clock anomaly; cooldown after revert.
- **Explanation record:** extends `DecisionRecord` with what/why/when/confidence/mode/constraints and an after-the-fact outcome. UI rule: a forecast band always shows measured coverage and an `UNCALIBRATED` state.

### 4.6 Multi-timescale loop

Seconds: only guard and revert actions are autonomous (halt warming when origin degrades). Minutes: short-horizon forecast, posterior update. Hourly: calendar activation, warm plan. Daily: reliability update, waste accounting, capacity recommendation, backtest. Weekly: forecaster re-selection by pre-registered rule, versioned config only. Online state is recursive and small; anything involving model selection or training is offline and promoted as a versioned artifact `[R-V2 §3, V1 C13]`.

### 4.7 Where ML is allowed

Only through the existing gate (`phantom_intelligence_phase.md` §6): a non-ML win first, an explicit simple baseline beaten with a confidence interval, bounded resources, replayable decisions. ML is not an MVP goal. The M5 forecasting-competition literature reportedly shows simple local statistical methods remain competitive for high-granularity data `[L-R V2 §2.4]`, which supports the ladder: persistence, seasonal-naive, ETS/Holt-Winters, residual quantiles, conformal intervals, and only then small online learners.

---

## 5. Multi-device demonstration architecture

### 5.1 The three axes (never mix them)

| Axis | Meaning | In code today |
|---|---|---|
| **Client** | A requester: device, browser tab, process, curl. Holds a credential. | Not an entity. The gateway sees a bearer token (and, in non-secure mode, a client-asserted header) `[R-V3 §0]` |
| **Tenant** | A namespace string in the cache key, taken from the verified JWT claim in secure mode | `CacheKey.tenant_id`; derivation at `src/api/server.rs:1209-1219`; non-secure mode takes the `x-tenant-id` header `[V]` |
| **Node** | A gateway process with its own in-memory cache; in a cluster, a member with an mTLS identity | One independent cache per node; GET is local-only `[R-V3 §0]` |

Cardinalities: many clients per tenant; a client may hold tokens for several tenants; a tenant is served by every node its clients reach, each with its own copy. Any UI text saying "client A's node" or "tenant A's cache" without naming the axis is wrong.

### 5.2 Device kinds and reference topology

| Device kind | Role | Credential |
|---|---|---|
| Participant: cache node | Gateway process (and, in a cluster, mTLS member) | JWT secret custody; cluster cert |
| Participant: client | Sends GET/DELETE under a tenant | Short-lived scoped token |
| View: presenter | Control Center, Operator and Observer | Operator token (effectively "start processes on this host", `[R-V3 §1.6]`; never given to guests) |
| View: audience/observer | Read-only | Observer-equivalent, tenant-scoped or aggregate-only |

Reference LAN topology (N >= 4 devices, illustrative) [R-V3 §2.2]: Device 1 Control Center (presenter); Devices 2-3 clients A and B (tenants alpha, beta); Device 4 node 1 (gateway plus mock origin); Device 5 node 2 (optional, mTLS peer); further observers. Which node a client uses is a **manual choice of URL**: there is no ingress router or load balancer.

### 5.3 Hard facts that shape any multi-device demo

All verified or read from code by V3; the ones I confirmed are marked `[V]`.

- A gateway bound to a non-loopback address will not fetch from a loopback origin (`allow_loopback_origin: !is_network_bind`, `src/main.rs:406` `[V]`); a LAN origin needs `PHANTOM_ALLOW_LAN_ORIGIN=true` `[R-V3]`.
- Non-loopback bind requires a JWT secret plus TLS cert/key, or the explicit insecure override `[R-V3 §1.5]`.
- A UI served from another device is cross-origin to the gateway and needs its exact origin in `PHANTOM_CORS_ORIGINS` (max 16); the development default allows only localhost:3000 and 127.0.0.1:3000 (`src/config.rs:550` `[V]`).
- Cluster membership is read from a file at startup; `ClusterMembershipRegistry::update` has no caller in `src/` (Appendix A #3) `[V]`; there is no join protocol. Adding a node means issuing a cert, editing the members file on every node and restarting them.
- No shipped tool mints tokens or generates cluster PKI; `scripts/gen-demo-certs.ps1` needs `openssl` and covers ingress only `[R-V3 §1.3]`.
- Tokens use wall-clock `exp` with 10 s leeway; clock skew between devices fails auth confusingly `[R-V3 §2.2]`.
- Only Windows loopback has ever been exercised; a real multi-device LAN run has never been performed `[R-V3 §2.2, V4 §0]`.

### 5.4 Pairing and credentials [P]

Principle: **the JWT secret never leaves the host that mints tokens; devices receive tokens only.**

- Pairing service (separate from the gateway; an external Node or PowerShell script first, no new dependency): presenter creates a session; a short single-use code (6-8 chars, about 2 minutes) is shown; a guest device redeems it over TLS; the service mints a short-lived JWT (5-15 min) scoped to tenant, role and a per-session or per-node `aud`; the code is burned. Guests never receive Operator or admin. Operator is bound to the presenter device.
- Per-session secret and audience (config only): tokens from another session or an old one are rejected; costs a gateway restart per session, which is acceptable because the cache is in-memory.
- Revocation today: expiry, or restart with a new secret. Fine-grained revocation (a `jti`/`sub` denylist) and closing `/api/events` at token expiry are Proposed and touch owner-reserved authentication code (D-20). `/api/events` does not check expiry inside its loop (`src/api/server.rs:955-972` `[V]`).
- QR is cosmetic and needs an encoder (no QR code exists in the repo); defer.
- TLS trust bootstrap choices and my resolution are in R10.

### 5.5 Failure injection (what can be done honestly)

| Method | Fidelity | Class |
|---|---|---|
| Kill a gateway process | Real node crash; reproduces the lost-invalidation defect when timed | Existing / launcher needed |
| Application-layer fault proxy between nodes (delay, blackhole, partition per direction) | TCP relay faults, no privileges; exists only as `tests/support/fault_proxy.rs` | Prototype (copy) then Proposed (supported tool, D-25) |
| Origin faults via the scripted origin | Real separate process; origin failure, not node failure | Existing |
| `POST /api/control {"event":"NODE_FAILURE"}` | **A logical flag, not a failure.** In dev mode it is also anonymous (`control_handler` only checks auth when security is configured, `server.rs:1070-1081` `[V]`) | Existing; label "SIMULATED/logical" or remove from the Demo Center |
| OS firewall rules, drivers, VMs, containers | Real, but system changes and installs | Hypothesis; needs approval (D-27); results labelled separately |
| Unplug a cable | Real, manual, not reproducible | Existing |

Every injection is a recorded session event naming the layer that injected it. Expected outcomes must come from the real system: reads stay local, DELETE legs to the primary time out at 150 ms and fail over, the peer becomes Suspect, owed invalidations queue (bounded), the repair worker and pull reconciliation return it to Active, **unless the issuer died** `[R-V3 §8.2, V4 §6]`.

### 5.6 Demo Session object (data contract, not code)

V3 §5.2's schema is adopted with V5's envelope: session id, mode label (computed, never typed by hand), resolved configuration hash with secrets redacted, build identity, seed, topology (nodes, origins, links, fault layer), tenants, devices (role, tenant, attached node, token expiry, clock offset, "reported" OS), labels (distributed / simulated / visualised-only), append-only hash-chained event log, referenced lab runs, limits. A persistent header chip shows the mode label and "N real nodes, M simulated" computed from the object `[R-V3 §6]`. Reproduction means re-running with the same seed and config hash on the same topology class, not replaying wall-clock events.

---

## 6. Multi-tenant and identity model

### 6.1 What is true today

| Property | Status | Evidence |
|---|---|---|
| Namespace isolation of cached bodies (secure mode) | **Existing** | Tenant from verified token only; caller `x-tenant-id` ignored (`server.rs:1209-1219` `[V]`); tenant charset excludes `:` so the key prefix is unambiguous `[R-V3 §1.1]` |
| Dev/non-secure mode tenancy | Existing but **not isolation** | `x-tenant-id` header is trusted, validated only for alphabet and length (`server.rs:1126-1134,1215` `[V]`) |
| Per-tenant byte/entry quota, fair eviction, per-tenant origin permits, per-tenant rate limits | **Unsupported** | Grep of `src/` outside lab/intelligence for `quota|per_tenant|tenant_limit|tenant_bytes`: no hits `[V]` |
| Per-tenant or per-client telemetry | **Unsupported** | `src/intel/telemetry.rs` has no tenant dimension `[V]`; SSE is a 100 ms aggregate |
| Tenant-scoped invalidation (HTTP DELETE of the caller's own exact key) | Existing | `server.rs:1982-2000` `[V]` |
| Tenant-epoch exhaustion | Bounded blast radius | `MAX_TENANT_EPOCHS = 10_000` then a global epoch bump (`src/core/cache.rs:273,581` `[V]`) |
| Multiple origins per gateway | **Unsupported** | One `ORIGIN_URL` (`main.rs:253` `[V]`); origin-side tenant branching via forwarded `x-tenant-id` exists `[R-V3 §1.2]` |

**Consequence:** one tenant can evict another's entries and consume its share of the byte budget and the origin cap. Any demo of "tenant A is protected from tenant B's flood" would show something the product does not do. Today the honest demo is the *measured noisy-neighbour effect* (V4 S-10), shown as a limitation.

### 6.2 Roles and tokens

| Demo role | Token roles / claim | May | Must not |
|---|---|---|---|
| Presenter | `scenario:operate` + `telemetry:read` | Control, Lab runs, all telemetry | Be issued to guests |
| Observer (audience) | `telemetry:read` | Streams and metrics | Read cache bodies, invalidate, control |
| Tenant client | `cache:read` (+ `cache:invalidate` only if DELETE is shown); `tenant_id` mandatory | GET/DELETE in its own tenant | See another tenant; read telemetry |
| Cache node | mTLS cert (strict SAN, fingerprint in members file), not a JWT principal | Invalidation frames, anti-entropy | Join without a membership entry |
| Admin | `admin` | Everything | Never used in demos |

Identity weaknesses to disclose or fix: HS256 shared secret (anyone holding it can mint any tenant or role); the `requester` alias grants both read and invalidate (`src/security/jwt.rs:66,71` `[V]`); raw tokens without `Bearer ` are accepted `[R-V4]`; no `jti`, revocation, lockout or maximum lifetime; `kid` rotation exists in the validator but `main.rs` builds the validator with `JwtValidator::new` only (`main.rs:413` `[V]`), so rotation is unwired; unknown role strings are silently ignored (a typo grants nothing).

### 6.3 Peer trust

mTLS cluster identity is strong at the door and total inside: **any authenticated cluster member can send `Tenant` or `FullCache` invalidation scope and the receiver applies it** (`src/dist/node.rs:547-553`; the only check is that the claimed originator equals the TLS identity, `:508`) `[V]`. A compromised member can purge every tenant on every peer. A per-scope authorisation (D-21) is a design decision, not a bug fix I can assert.

### 6.4 Model to build toward [P]

1. **Tenant-scoped resource accounting first** (bytes, entries, origin permits, warm budget), then quotas with minimum guarantees (Memshare-style `[L-R V1 P11]`) so a tenant cannot be starved or steal.
2. **Tenant-scoped observation**: counters and top-k per tenant; observer views tenant-scoped or aggregate-only; global top-k operator-only.
3. **Intelligence state per tenant** (R13); tiered by tenant size; long-tail tenants get only a coarse rate series.
4. **Secure-mode-only write paths** for calendar and warm; denied in dev mode.
5. **Identity hardening** in a dedicated phase with failing-before tests: `jti` denylist, stream closure at expiry, `kid` wiring, optional max TTL, peer scope authorisation. These touch owner-reserved authentication code.
6. **Clients, tenants and nodes stay separate in every UI and spoken claim** (section 5.1).

---

## 7. Security demonstration catalogue

**These demos are not security certification.** They show that a specific control behaves as described under a specific fixture, mode, build profile and host. Penetration testing, fuzzing, dependency scanning, TLS scanning, independent review and multi-host testing have not been done `[R-V4 §9]`. Never use "secure", "zero-trust" or "guaranteed" without the bound and the failure model.

### 7.1 Findings that change how security demos must be framed

1. The earlier `docs/security/security-demo-matrix.md` is a **design document, not a record of working demos**. `tests/security/` does not exist; `src/bin/` holds only `phantom-lab.rs`; the error name `UpstreamDestinationDisallowed` appears only in the docs (all `[V]`, ls and grep). The real errors are `SsrfBlocked` and `UpstreamTlsFailed` (`src/api/server.rs:2277-2284` `[V]`). `phantom-verify` and Ed25519 signing do not exist `[R-V4 §0]`.
2. "Demonstrable today" means a **test exists** (names from V4, which I did not re-run or open). A presentable demo additionally needs an **evidence runner** that does not exist for security scenarios (V4 §3). Passing tests written by the code's own authors do not establish correctness (`CLAUDE.md` §3).
3. `src/lab/manifest.rs:103` hard-codes `git_tree_hash` to `456bb259...` `[V]`, so that manifest field is not evidence of the build's real tree.
4. Dev mode is anonymous and trusts `x-tenant-id` `[V]`; every scenario must name its mode, and dev mode may be shown only as the labelled insecure baseline.
5. The M3 durability defect limits any "revoke/erasure" claim (7.4).

### 7.2 Catalogue (condensed; status uses your three labels)

"Needs-extension" always includes the missing evidence runner. Source IDs are V4's.

| ID | Scenario | What it shows (and does NOT show) | Status |
|---|---|---|---|
| A1 | Rogue TLS endpoint | Client refuses untrusted server cert (TLS library behaviour; nothing about PhantomCache code) | Needs-extension (small raw-TCP rogue listener) |
| A2 | Tampered artifact | Byte flip fails `sha256sum -c`; integrity only, not authenticity unless the sums are signed | Needs-extension (checksum part exists; signed manifest and verifier do not) |
| A3 | SSRF and upstream policy (re-framed: origin is static config, not client input) | Metadata/private ranges, ports, redirects refused; does not prove DNS-rebinding resistance or cover SEC-M7-10 ranges | Demonstrable-today |
| A4 | Cross-tenant leakage and auth bypass | Tenant from token only; header ignored; contrast with dev mode. Not capacity isolation or side channels | Demonstrable-today |
| A5 | Control-plane RBAC | 401/403 and no mutation. The "audit log records unauthorised attempt" expectation is **not implemented** | Demonstrable-today (audit event Unsupported) |
| A6 | Origin TLS failure | Untrusted/expired/mismatched origin cert refused and not cached | Demonstrable-today |
| A7 | Altered evidence detection | Needs hash-chained log, manifest, signature, verifier | Needs-extension |
| A8 | Node authentication | Rogue/revoked/unknown/mismatched node refused | Demonstrable-today (read-path failover Unsupported, R8) |
| S-05 | Unkeyed-header poisoning | Client headers never reach the origin; victim sees clean variant | Demonstrable-today |
| S-06 | Key confusion / path normalisation | Listed probes yield 400 or no cross-tenant key; `%2f` vs `/` and query-reorder are open probes | Demonstrable-today (probes Needs-extension) |
| S-07 | Cache deception | Shows both outcomes honestly: a 200 with no freshness headers **is** cached 60 s; `private/no-store` is not | Demonstrable-today |
| S-08 | Cache-poisoned DoS / error caching | 4xx/5xx not stored; an end-to-end negative test is missing | Needs-extension |
| S-09 | Bypass and unique-key amplification | `x-cache-bypass` skips coalescing, 1:1 to origin, bounded only by the cap; no per-principal limit (`server.rs:1243-1276` `[V]`) | Needs-extension (limit Unsupported) |
| S-10 | Noisy-neighbour eviction | A limitation demo: no quota exists, so interference is expected and must be measured | Needs-extension |
| S-11 | Cross-tenant timing/occupancy channel | Hypothesis only; shared shard locks and eviction exist | Unsupported (no claim either way) |
| S-12 | Token abuse | Forged/expired/mis-scoped refused; theft works until expiry; no revocation or lockout | Demonstrable-today (revocation Unsupported) |
| S-13 | Invalidation authorisation | DELETE confined to caller's tenant and exact key | Demonstrable-today |
| S-14 | Compromised cluster member | Membership equals trust: a valid member can purge all tenants (`node.rs:547-553` `[V]`) | Needs-extension (limitation fixture) |
| S-15 | Repair-ledger overflow blast radius | One tenant's DELETE storm collapses to tenant flush before global flush; attack economics unprobed | Demonstrable-today (economics Needs-extension) |
| S-16 | Resource exhaustion / malformed input | Oversize bodies, keys, frames refused; no HTTP connection-count limit evident | Demonstrable-today (connection exhaustion Unsupported) |
| S-17 | Observer data exposure | Pinned field set; no tenant scoping of metrics/audit | Needs-extension |
| F1-F6 | Distributed failure (node offline, in-flight, routing, invalidation under failure, rejoin, stale-data risk) | See 7.4 | Mixed (7.4) |

### 7.3 Evidence integrity design (existing dependencies only)

Per scenario run, a bundle: append-only `events.jsonl` where each line includes `seq`, `ts`, `kind`, `payload` and `prev = SHA256(previous line)`; raw `transcript/`; the origin fixture's own `origin_log.jsonl`; gateway counters before and after; the redacted effective-config report; environment (OS, build profile, binary hash, real git HEAD, dirty list, computed from reality); a manifest of file hashes plus chain head and count; optional Ed25519 signature of the manifest (`ring` is already a runtime dependency; Ed25519 is not used yet `[R-V4 §7.1]`). A verifier recomputes hashes, chain and signature and exits non-zero at the first divergent `seq`.

Limits that must be stated: truncation is detected only if the final head and count are anchored elsewhere; local timestamps are not trusted evidence; tokens are redacted *before* hashing; the bundle cannot prove the binary under test is the code claimed, that fixtures were honest, or independence of the test author.

### 7.4 Distributed-failure scenarios: what is real

| Scenario | Real today | Honest limit |
|---|---|---|
| Node offline (kill or blackhole) | Real processes behind fault proxies in tests; Active, Suspect, Recovering, Active transitions | Loopback, small N, debug builds; a fault proxy is injection, not a physical partition; the dead node's clients get no service |
| In-flight requests when a node dies | Coalesced waiters get cancellation; permits released on disconnect | No connection draining; no test kills a gateway mid-GET |
| Invalidation during failure | DELETE returns 204 after local purge plus primary (or acting-primary) ack; others are owed in a bounded in-memory ledger and repaired | Holds only while the **issuing process stays alive** and within ledger bounds |
| Rejoin | Peer returns, ledger drains, pull reconciliation promotes it | A restarted issuer has an empty ledger |
| **M3 defect** | Reproduced by a prior agent in a scratch copy: DELETE 204, issuer killed, peer stays stale for the whole 10 s window (4/4 runs); control run converged in 231-337 ms `[R-V4 §6.3]`. The ledger is an in-memory `Mutex<HashMap>` (`src/dist/router.rs:137` `[V]`) and no durable log exists in `src/` (grep) | Fix is an unintegrated scratch patch; owner decision D-05 |
| Stale-data risk | Local purge is immediate; stale serving bounded by the 24 h hard cap | Origin lifetime has no gateway ceiling; propagation latency measured only on warm loopback, debug build (P99 about 10-20 ms, cold 35-117 ms `[R-V4 §6.2]`); the old "50 ms" bound is unsupported |
| Unsupported entirely | Clock skew/NTP, disk faults, OOM, asymmetric partitions beyond proxy modes, cross-host networks, power loss, churn above 5 nodes | State this in any matrix |

Demos that would mislead without a caveat: "revoke everywhere", rolling restarts right after DELETEs, "self-healing cluster" narratives (repair genuinely heals peer outages but not issuer loss), "zero stale reads after 50 ms", and the `NODE_FAILURE` button. Show the healing case and the issuer-loss case side by side.

---

## 8. Demo Center UX architecture

### 8.1 One product shell, one model

Not seven playgrounds. **One scenario runner, one event model, one evidence model**, three lenses (V5 §1.2, §3):

- **Scenario**: versioned data (topology, workloads, fault schedule, arms, seed, phase plan, tier, capability requirements, narration beats). A scenario library page shows a "what is real here" badge per scenario.
- **Session**: a run of a scenario with a timeline cursor and a mode: LIVE, RECORDED REPLAY or REHEARSAL. The existing Lab `run_id` is the precursor.
- **Evidence**: immutable; the existing `/evidence` views.
- Lenses: Traffic and Origin; Topology; Decisions and Trace. Plus Compare, Evidence/Report, Runs. The existing five operator screens stay as the operator console. `#/lab/*` stays addressable; Demo Center is a front door, not a rewrite.
- **Rule:** a control may only edit a scenario draft that the backend validates, or send a live action the backend executes and acknowledges. A chart never moves because of a client-side computation.

### 8.2 Views and always-on honesty

| | Presenter | Audience (projection) | Observer |
|---|---|---|---|
| Controls | start/pause/step, cancel, reset, replay, mark, share | none | drill-down, filters, export |
| Density | full | 3-4 elements per beat, large type | full |
| Data | same | same | same |

A fixed footer strip, never hidden: `MODE | TIER | SEED | BUILD | STREAM HEALTH`. The audience view omits detail, never alters values. Multi-device means several browsers attached to one session; topology (single process, multi-process, multi-host) is a scenario property with its own badge, identical in every view. Time rendering uses server session offsets, never the local clock.

### 8.3 Components that matter for the intelligence story

- **Request trace card** with a stated sampling rule and only observed steps ("step not observed" rather than an inferred step). Honest minimum today: client-side record plus response headers (`X-Cache-Status`, `Age`), depth "outcome only" `[R-V5 §3.5]`.
- **Decision explanation** from templates tied to event fields so text cannot claim more than the event says.
- **Declared vs predicted vs actual**: prediction frozen at issue time, never redrawn; interval, horizon, model id, and **observed coverage so far** always shown; withheld predictions shown as gaps; mode label OFFLINE REPLAY / SHADOW / ACTIVE. Only OFFLINE REPLAY is possible today.
- **Refusal log**: why the system did nothing is a first-class display (section 18).
- **Compare** with a compatibility check (seed, workload, fault schedule, build profile, host class, binary hash) and a refusal or "NOT A CONTROLLED COMPARISON" label; the Scenario B pilot-vs-main binary-hash discrepancy is the failure it prevents.
- **Report builder**: server-assembled from retained evidence ids; negative verdicts styled identically to positive; verbatim notices; artifact hashes; reproduce command; copy button appends tier and notice id.
- **Replay semantics**: T1 simulated runs are bit-reproducible from the seed; T2 loopback runs have deterministic offered load but non-deterministic outcomes, so the UI must distinguish "re-run with the same seed" from "play back the recorded events".

### 8.4 Event model, tiers and label registry

Event envelope (V5 §4.2): schema, gap-free id, server time, session-offset time, kind, tier, source, scenario reference, sampling declaration, trace id. Kinds: request, cache_decision, origin_fetch, coalescing, admission, invalidation, fencing, security_decision, node_health, prediction, prediction_outcome, action, scenario_state, marker, gap. Volume tiers: **T-A story** (lossless, persisted, hashed), **T-B aggregates** (fixed rate), **T-C trace samples** (sampled, bounded ring, `gap` events on drops). Producers never block the serving path; observer overhead must be measured before tracing is enabled in measured scenarios, and "TRACING ON" is labelled. Events carry fingerprints, never raw keys or tokens. Merge order is arrival order at the session writer, not causal order; causality goes through trace and op ids.

Label registry [P, D-26]: existing `MEASURED_LOOPBACK`, `SIMULATED`, `RETAINED_*`, `UNAVAILABLE`; new `SINGLE_HOST_MULTI_PROCESS`, `LAN_MULTI_DEVICE`, `VIRTUALISED`; `SYNTHETIC_FORECAST_HARNESS` (a kind of SIMULATED); provenance value `DECLARED`. Loopback, simulated, WSL2 and physical results are never mixed.

### 8.5 Hard rules (the anti-decoration device)

Every rendered series is backed by an event or artifact field; no ambient traffic, no interpolation across gaps, no client-side counterfactual ("without Phantom: +300%"), no "dollars saved" model unless parameters are visible and it is labelled a model, no "intelligent/autonomous" headline unless the matching `action` or `prediction` events exist in that session with tier labels. Extend the existing source guards (no result-like numeric literals, no storage, tokens only) to Demo Center files. "No data" is never drawn as zero. Each panel defines loading, empty, partial, error, unauthorised, disabled, stale, disconnected, sampled, recorded and rehearsal states. Accessibility baseline is inherited from the Lab UI (non-colour status, reduced motion, tables under charts); high-rate streams are never announced to screen readers.

### 8.6 Surviving a live failure

A recorded fallback is always available and always labelled RECORDED, never silently swapped in. Runs take real time, so the presenter can start them ahead and play the retained result. A bad result is shown as is; the script contains the sentence for it `[R-V5 §3.10]`.

---

## 9. Single-machine vs multi-device comparison

| Dimension | Single machine (loopback, multi-process) | Multi-device LAN (physical) | Virtualised (VMs/containers/WSL2) |
|---|---|---|---|
| What it genuinely proves | Protocol and code paths; real processes and sockets on one stack; reproducible | Real NICs, switch jitter, independent clocks, real resets, real power-off | Separate network stacks, easy reset, virtual-switch faults |
| What it cannot prove | Distribution, LAN latency, independent clocks or failure domains | Reproducibility; scripted faults without human action | Independent hardware; NAT/bridge quirks |
| Result label | `MEASURED_LOOPBACK` / `SINGLE_HOST_MULTI_PROCESS` | `LAN_MULTI_DEVICE` (never run end to end) | `VIRTUALISED`, WSL2 kept separate |
| Setup cost | Low (launcher script plus mock origin) | High: firewall, client-isolated Wi-Fi, CORS, TLS trust, clock skew, PKI, token minting | Medium; installing Hyper-V/Docker is a system change needing approval |
| Readiness today | Partial: gateway spawning exists in `lab_live`; no demo launcher | Partial: every piece exists, never exercised together `[R-V3 §2.2]` | Proposed; no image, no recipe |
| Credentials | Same code paths; tokens per browser profile | Short-lived scoped tokens, TLS or labelled insecure | Same |
| Failure injection | Process kill; app-layer proxy; scripted origin | Plus unplug, Wi-Fi off | Plus virtual-switch policy, snapshot reset |
| Typical failure | Host CPU contention distorts load | Guest Wi-Fi isolation blocks device-to-device traffic (the most common field failure) | Platform quirks |
| Right use | Rehearsal, regression, automated evidence | The flagship live show, once rehearsed twice | Later labelled variants |
| Platform | Windows verified | Windows x86-64 only; Linux/ARM/macOS Unsupported | Unverified |

Principle: a 3-process loopback "cluster" must never be described as distributed in the sense of independent hosts. A phone and a projector attached to one session are views, not participants.

---

## 10. Phased implementation roadmap and dependency order

### 10.1 Dependency rules

1. Nothing in "Phase 1: enablers" is optional for the features that depend on it; the table below states each dependency.
2. No intelligence actuation before: per-tenant accounting (for tenant-visible effects), durable invalidation (for anything cross-node), a bounded event tap with measured overhead, and a pre-registered evidence gate.
3. No demo claim ahead of the capability beneath it; where a capability is missing the demo shows an explicit unavailable panel or a labelled recording.
4. **Dependency-ready work continues now and is not interrupted by this roadmap** (section 17). Items marked "approval" wait for you; nothing here changes the existing freeze, sealed-seed or M6 handling.

### 10.2 Dependency table

| Item | Depends on | Blocks | Ready now? |
|---|---|---|---|
| Launcher/preflight script (outside `src/`) | none | All multi-process demos | Yes (no src change) |
| Recorded-run playback, "what is real" badges | Existing Scenario B evidence | Flagship interim | Needs UI change (approval) |
| Security evidence runner and `phantom-verify` | none (uses `ring`) | A2, A7, evidence in all demos | Approval (new bin) |
| Token minter (script) | none | LAN demos, guest devices | Yes (script) |
| Cluster PKI/members-file tooling | none (or a dependency decision) | Any cluster demo on separate hosts | Approval |
| **Durable invalidation (M3)** | independent review, failing-before test | Cross-node warming, erasure claims, issuer-kill demo as "fixed" | Approval; scratch exists |
| **Per-tenant accounting and quotas** | design; touches fenced cache code | C7 reallocation, tenant warm budgets, tenant-protection demo | Approval; high risk |
| **Bounded per-request event tap** | design; overhead measurement (E0) | Observe, Demo Center traces, tenant counters | Approval; hot path |
| Tenant-scoped telemetry and observer scope | event tap | Tenant client views, safe audit endpoints | After tap |
| Load-dependent origin model (E1) | none | Origin-pressure forecasting, causal "surge slows origin" story | Approval (lab) |
| Multi-origin per gateway | config and routing design | Per-origin intelligence | Defer (D-14) |
| Dynamic membership / join-leave | PKI tooling; registry update wiring (touches auth-adjacent code) | Live join/leave demos | Defer (D-15) |
| Identity hardening (jti, expiry closure, kid) | design | Fine-grained device revocation | Approval; auth reserved |
| Observe-mode intelligence (hot keys, alarms, staleness-risk) | event tap, per-tenant state | Advisor, shadow | After tap |
| Event-driven warm/refill: out-of-process prototype | token minter; secure mode | In-gateway warm, E9/E11 evidence | Yes as a script prototype |
| Headroom test E9 (new `src/lab/demand/` module, protocol v4) | owner-generated salt (offline), approval | All forecasting work | Approval |
| In-gateway warm actuator and priority gate | E9/E11 pass, independent review, per-tenant accounting | Active warming | Later |
| Demo Session (Control Center), views, tickets | event contract (D-07), security review | Multi-device flagship | Later |

### 10.3 Phases

| Phase | Content | Exit criterion (summary; detail in section 12) |
|---|---|---|
| **P0 Truth and rehearsal** (no application code) | Launcher, preflight, corrected security docs, security test inventory run, token minter, interim flagship (section 14), RT-0 prior-art plan, browser verification of the existing Lab UI (in flight) | Interim flagship rehearsed twice on 3 physical devices with zero unlabeled values |
| **P1 Enablers** (approvals; touches frozen areas) | Durable invalidation; event tap; per-tenant accounting design then implementation; evidence runner; cluster PKI tool; load-dependent origin | Each ships with a test that fails without it plus an independent review |
| **P2 Observe and Advise** | Truthful hot keys, regime and flash-crowd alarms, staleness-risk estimator, Advisor what-if, outcome ledger | E0 overhead within budget; coverage and calibration reporting |
| **P3 Deterministic preparation** | Event-driven bounded warm/refill (out-of-process then in-gateway), budgets, refusal log, regret governor | K-C3 safety gates hold (no cap violations, origin peak within 5 percent of baseline) |
| **P4 Research track** | E9 headroom, E10 forecast skill, E11 calendar value, E12 T2 confirmation, E2 cap headroom; shadow then canary only on a pass | Pre-registered kill or promote decisions |
| **P5 Demo Center sessions and multi-device flagship** | `demo-1` session stream, views, pairing, report builder, comparison, flagship live | Section 12 acceptance |
| **P6 Hardening and variants** | Identity hardening, peer scope authorisation, VM/container variants, dynamic membership | Per item |

---

## 11. Dependencies and technical risks

| # | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| T1 | Story pressure reintroduces decoration (fake predictions, "saved $", fake peers) | High / Critical | Footer strip, source guards, every series backed by an event, owner sign-off on headline claims |
| T2 | Per-tenant accounting touches the fenced cache core (I1-I8, I5/INV-4, lock order, zero-contention read path) | Medium / Critical | Design first; independent review; test that fails without the change; micro-benchmark with CI |
| T3 | Event tap perturbs the hot path and the thing measured | Medium / High | Bounded drop-newest ring; measure overhead (release, paired reps); tracing off in evidence runs unless within an agreed bound |
| T4 | Forecasting yields no gain over a schedule (the likely outcome) | High / Medium | Headroom test first; calendar-literal retained; negative result retained as evidence |
| T5 | Effective TTL 60 s makes far-ahead warming uneconomic | High / Medium | Warm lead bounded by min(max lead, c x TTL); resolve `PHANTOM_DEFAULT_TTL_MS` wiring decision |
| T6 | Synthetic-only evidence (no real trace) is circular | High / High | Planted-signal controls, structurally diverse generators, held-out regimes, label SIMULATED; pursue a design partner |
| T7 | Cross-tenant leakage via shared models, permit pools, audit aggregates | Medium / High | Per-tenant state; tenant-scoped endpoints; leakage tests in G9-style worlds |
| T8 | Calendar as an attack surface (fill amplification, masking attacks) | Medium / High | Secure-mode only, authorisation, budgets, expiry, anomaly alerts never silenced by a declaration |
| T9 | M3 durability defect undermines cluster claims | High (known) / High | Integrate with review, or show defect labelled |
| T10 | Multi-device field failures (client-isolated Wi-Fi, firewall, clock skew, CORS) | High / High | Preflight checklist; two rehearsals on the real devices; recorded fallback |
| T11 | Operator token equals remote process spawn | Medium / High | Presenter-only; never pair as guest |
| T12 | Pairing opens a network surface in a loopback-first product | Medium / High | JWT only for cross-device; read-only guests; security review before any gateway ticket route |
| T13 | Loopback single-host results over-read as production performance | High / High | Permanent tier banner; no cross-tier "x faster" |
| T14 | Wall-clock ordering across devices misread as causality | Medium / Medium | Trace and op ids; "order observed" label; show clock skew |
| T15 | Contract churn breaks pinned fixtures (`lab-1`) | Medium / Medium | New `demo-1` stream |
| T16 | Documents drift from code (this report included) | High / Medium | Appendix A shows several V-claims and `CLAUDE.md` lines already drifted; re-verify before relying |
| T17 | Owner-reserved areas touched without review | Medium / High | Mark each decision with frozen-file exposure (section 13) |
| T18 | Scope explosion: five reports propose roughly ten new components | High / High | Section 17 limits near-term work; section 18 recommends a smaller demo |

---

## 12. Measurable acceptance criteria

All thresholds are **proposals**; you set the real bounds (D-08, D-28). Performance numbers require release build, hardware and OS, workload, seed, baseline, repetitions and confidence intervals (`CLAUDE.md` §4); loopback, simulated, WSL2 and physical are reported separately.

| Area | Criterion | Source |
|---|---|---|
| Truthful display | 0 screens render a value without a provenance badge or an unavailable reason (automated DOM test over fixtures); usability check: at least 5 participants, at least 90 percent can say whether numbers are live, recorded or simulated | V5 §6.1 |
| Event integrity | Per-session ids strictly increasing and gap-free; a forced overflow yields exactly one `gap` event for the dropped range | V5 Phase 2 |
| Tap overhead | Release build, at least 10 paired repetitions, tracing on vs off: overhead on client p99 and throughput reported with a 95 percent interval; tracing allowed in evidence runs only if the interval excludes your bound | V5 Phase 2, V1 E0 |
| Hard invariants | `stale_cap_violations`, cap violations, `bytes_audit_equal` failures, invariant violations all 0 in every run and test; tenant isolation violations 0 | CLAUDE.md §5, V2 §6.6 |
| Tenant isolation (capacity) | With quotas: a flooding tenant's effect on a neighbour's hit ratio is bounded to a pre-registered margin (proposal: neighbour hit ratio down no more than 1 percentage point) across N seeds; without quotas the measured degradation is reported as a limitation, not hidden | V2 K-C3, V4 S-10 |
| Warm/refill safety | Origin peak and success-p99 no worse than the no-warm baseline by more than 5 percent in any scenario; wasted warm bytes fraction below 50 percent in the half-false-declarations world with the regret governor on; null world spends below epsilon | V2 K-C2..K-C4 |
| Fencing under warm | N scripted races: a stale response never stored after a newer generation; disabling fencing makes the test fail | V5 Phase 3 |
| Headroom | Oracle improvement over the best realistic baseline at least 10 percent on miss-storm size and primary utility with CI above the margin; otherwise predictive warming is abandoned | V2 K-H |
| Forecast skill | Beats the best of persistence, seasonal-naive, EWMA by MASE below 0.95 in at least 5 of 8 held-out regimes; nominal-90 percent intervals cover 85-95 percent in held-out drift worlds; otherwise shown as UNCALIBRATED | V2 K-F1, K-F4 |
| Promotion to serving path | delta_min exceeded with BCa lower bound above 0 in at least 5 of 8 families, none adverse, T2 confirmation with at least 10 repetitions, overhead within budget, deterministic replay | V1 §6, accountability 11.6 |
| Evidence tamper detection | Planted edit, reorder and interior deletion in a bundle are each detected by the verifier; truncation detected when the head is anchored | V4 §7 |
| Multi-device | 3 physical devices pair, run GET/DELETE under their tenants; Observer sees telemetry; a guest token gets 403 on `/api/control`; an expired token gets 401; wrong code rate-limited; teardown leaves no listening ports or trust/firewall residue (checked by listing) | V3 Phase 2 |
| Cluster | Two-node LAN: invalidation propagation latency reported as a measured distribution (release, repetitions) not a guarantee; kill-the-issuer scenario shows the defect labelled until M3 is integrated | V3 Phase 3 |
| Reproducibility | Same-seed T1 re-run reproduces `cells_csv_sha256`; same-seed T2 re-run reproduces the offered-load schedule hash and the UI says outcomes may differ | V5 Phase 1 |
| Flagship | Every beat is real, a labelled recording, or an explicit unavailable panel; the report marks any skipped beat as incomplete; a presenter-only rehearsal passes twice on the real devices | section 14 |

---

## 13. Decisions requiring product-owner approval

Deduplicated from all five reports (about 70 raw questions collapse to the 40 below). **Touches** uses: **F** = frozen or owner-reserved files (authentication, cache core, fencing, invalidation, pre-registration/sealed handling); **A** = other application code or a new binary/UI (needs approval under `CLAUDE.md` §2); **S** = system or software install or global setting; **N** = policy, documentation or script only. Frozen-file membership is not published (V3 D14); I mark by area and ask you to confirm (D-37).

| # | Decision | Recommendation | Unblocks | Touches |
|---|---|---|---|---|
| D-01 | Adopt the positioning: bounded, auditable, rehearsable gateway; intelligence advisory-first; no "AI-powered" claims | Yes | All messaging and scope | N |
| D-02 | Flagship: adopt the re-sequenced "Launch Day" with Predictive demand as a replay or declared-warm act and Tenant attack as a side act (V5 §9.1) | Yes (section 14) | Demo planning | N |
| D-03 | Present the Scenario B result as `TRADE_OFF_FAILURE` with the permitted-conclusion text; no beat shown in a more favourable form | Yes | Demo credibility | N |
| D-04 | Per-tenant resource isolation (accounting, quotas, fair eviction, tenant telemetry) in scope as a prerequisite | Yes: design first; until built, state "namespace isolation only" | Tenant demos, C7, tenant warm budgets | F (`cache.rs`, `eviction.rs`, `telemetry.rs`) |
| D-05 | Integrate durable invalidation (M3) before any multi-node invalidation or "erasure" demo | Yes, after independent review and a failing-before test; else show the defect labelled | Cross-node warming, issuer-kill demo as fixed | F (`router.rs`, `main.rs`, `config.rs`) |
| D-06 | Build one bounded tiered event tap and measure its overhead (E0) | Yes | Observe, Demo Center, tenant counters | A, hot path (`server.rs`, telemetry) |
| D-07 | Event contract: new `demo-1` session stream, not an extension of `lab-1`; who owns the schema | New stream | Domain events, replay | A (`lab_api.rs`, UI fixtures) |
| D-08 | Approve headroom-first order and proposed kill margins (10 percent miss-storm, 5 percent harm, 1 pp neighbour, 50 percent waste, MASE 0.95) | Yes | All forecasting work | N |
| D-09 | Calendar semantics: who may author/approve (roles, four-eyes), secure-mode only; add `DECLARED` provenance; first form is the event-driven warm/refill API | Yes | Warm feature, Advisor | A (`intelligence/mod.rs`) |
| D-10 | Warm actuator: out-of-process prototype first; in-gateway only after E9/E11 and independent review | Out-of-process first | Early evidence without touching fencing | N (script) now; F later (`cache.rs` fetch path) |
| D-11 | Real traces: pursue a design partner or anonymised trace (key hashes, sizes, timestamps) | Yes | Seasonality, origin forecasting, fragmentation detector, any non-synthetic claim | N |
| D-12 | New pre-registered protocol (v4) for demand evaluation, own domain string, **salt generated by you offline, outside any agent session**, new seed ranges disjoint from burned and E2-proposed ranges | Yes; no agent generates or sees the salt | E9-E12 sealed runs | F (pre-registration area; owner-reserved) |
| D-13 | Build a load-dependent origin model (E1) | Yes before any origin-pressure or cap claim and before "the surge slows the origin" is a mechanism rather than narrative | Origin forecasting, X2, honest causality | A (`lab_live`) |
| D-14 | Multi-origin per gateway | Defer; use gateway-per-origin | Per-origin intelligence | F/A (`main.rs`, `config.rs`, `server.rs`) |
| D-15 | Dynamic membership/join-leave | Defer; present static membership with restart | Live join demos | F (`cluster.rs`, auth-adjacent) |
| D-16 | Cluster PKI/members-file tooling: documented openssl step now; a supported tool later (a runtime `rcgen` dependency needs the `CLAUDE.md` §2 justification) | Documented step first | Any cluster demo on separate hosts | N now; A + Cargo.toml later |
| D-17 | Pairing/minting: external script (Node or PowerShell, no dependency) vs `src/bin` tool; who holds the session secret | Script, secret on presenter device | LAN guest devices | N (script) / A (bin) |
| D-18 | LAN TLS trust bootstrap: pinned fingerprint preferred; plain HTTP only on a presenter-owned private network with a permanent banner; no CA install without per-device consent | As stated | Phone/tablet participants | N; S if CA installed |
| D-19 | Token policy: max lifetime 5-15 min; guests do not hold `cache:invalidate` by default; observers aggregate-only or tenant-scoped | Yes | Pairing design | N |
| D-20 | Identity hardening: `jti` denylist, close `/api/events` at expiry, wire `kid`, optional max TTL | Approve as P6 with failing-before tests | Per-device revocation | F (`jwt.rs`, `server.rs`, `main.rs`, `config.rs`) |
| D-21 | Per-scope authorisation of peer invalidation frames (restrict `Tenant`/`FullCache`) and a tenant-scoped Observer role | Approve design | S-14, multi-tenant observers | F (`node.rs`, `router.rs`, `jwt.rs`) |
| D-22 | Deterministic per-principal limit or disable for `x-cache-bypass`; count denied reads | Approve as a plain limit, not intelligence | S-09, abuse resistance | A (`server.rs`) |
| D-23 | Disposition of SEC-M7-09 (alias, raw-token), -10 (SSRF ranges), -11 (revocation/lockout) | Needs your explicit disposition | Security posture statements | F |
| D-24 | Security evidence runner and `phantom-verify` (hash chain, Ed25519 via existing `ring`) | Approve | A2, A7, all evidence bundles | A (new bin, `Cargo.toml` `[[bin]]`) |
| D-25 | Promote `tests/support/fault_proxy.rs` to a supported demo tool; no privileged fault injection by default | Promote; no privileged | Cluster failure demos | A (new file; copy) |
| D-26 | Label registry: new tiers `SINGLE_HOST_MULTI_PROCESS`, `LAN_MULTI_DEVICE`, `VIRTUALISED`, provenance `DECLARED` | Approve | Honest multi-device reporting | A (`labProvenance.js`, `lab_api`) |
| D-27 | Software installs/system settings (Hyper-V, Docker, firewall rules, loopback aliases) | Not now; document manual steps | VM/container variants | S |
| D-28 | Instrumentation budget: max acceptable measured overhead; tracing in evidence runs | You set the bound; default off | Tap rollout | N |
| D-29 | Session model: keep serialised beats for evidence runs; overlapping only in rehearsal | Serialised | Measurement validity | A (`lab_api.rs`) |
| D-30 | Privacy of keys and tenants in events: fingerprints only; any raw keys operator-only; opt-in, hashed warm-up manifests | Fingerprints only | Event tap, X1 | N |
| D-31 | Correct or supersede `docs/security/security-demo-matrix.md`; fix the hard-coded `git_tree_hash` in `src/lab/manifest.rs:103` | Supersede with section 7; fix hash | Honest evidence manifests | N (doc); F? (`manifest.rs` in the lab area) |
| D-32 | `PHANTOM_DEFAULT_TTL_MS`: wire it or keep the literal 60 s and document | Continue the isolated TTL decision track; warming lead bounds depend on it | Warm economics | F (`freshness.rs`, `server.rs`) |
| D-33 | Update `CLAUDE.md` §6: the serving cache uses faithful eviction (R9) | Yes after you confirm | Removes a false limitation | N (project rules) |
| D-34 | Browser-verification gate as a blocker for new UI work | Already in flight; keep as gate | Any new UI | N |
| D-35 | Peer-fill / owner-fetch read path and cluster-aware origin budgeting: decide research-only now | Research only | Cluster-aware intelligence | F if ever built |
| D-36 | Persistence of calendar, forecaster state, reliability ledger across restart | Defer; load the calendar from config at boot | Recurring entries, long history | A |
| D-37 | Publish the frozen/owner-reserved file list | Yes | Exact scoping of every F/A item | N |
| D-38 | Fund an RT-0 prior-art sweep (proceedings 2022-2026) before any positioning claim | Yes | Any differentiation claim | N |
| D-39 | Supply the seven-playground list and the real audience (operator/security reviewer vs sales) | Needed | Demo scope and tone | N |
| D-40 | Dev mode (anonymous, trusts `x-tenant-id`, anonymous `/api/control`) shown to external audiences only as a labelled insecure baseline | Yes | Safe demos | N |

**The ten that matter most:** D-04 (tenant isolation), D-05 (durable invalidation), D-06 (event tap), D-02 (flagship ordering), D-09/D-10 (calendar semantics and warmer location), D-08/D-12 (evidence protocol), D-13 (load-dependent origin), D-17-D-19 (pairing and token policy), D-24 (evidence runner), D-01/D-03 (positioning and honest negatives).

---

## 14. The flagship demonstration

### 14.1 Mapping your stages A-F to V5's "Launch Day"

| Your stage | Launch Day beat | Real today | Must be built | Truthful interim (runs on current code) |
|---|---|---|---|---|
| **A Normal operation** | Beat 1 quiet morning (measured baseline) | Real gateway, Zipf open-loop load, scripted origin through Scenario B; aggregate counters and 100 ms SSE | Open-ended session; windowed rates and latency histograms; trace samples | Scenario B `warmup`/`healthy` phases, longer presenter-paced phase; counters MEASURED, windowed ratios shown UNAVAILABLE |
| **B Predictive demand** | Act 2.5 "Could we have seen it coming?" (replay) | Offline evidence only: M6 retained verdict `NOT_SUPPORTED`; **no predictor in the serving path** (`pie: None`) | Event-driven warm/refill; Observe events; later a verified predictor | Show (1) the retained negative result, (2) a **declared** (not predicted) warm using the out-of-process script, labelled `DECLARED`, with origin load measured with and without it. Never draw a prediction line |
| **C Flash crowd** | Beat 2 viral link | Single-flight coalescing, origin cap, counters; T1 `W-FC` workload | Rate-step/hot-set arrival process on the real-gateway load generator; coalescing events | Scripted burst of concurrent requests for a handful of cold keys from the client devices; show N concurrent misses become one origin fetch using the origin's own request log versus client count; "origin requests avoided" DERIVED with formula, no counterfactual line |
| **(origin trouble)** | Beat 3 origin struggles | Scenario B fully (runs, evidence, notice, charts) | Load-dependent origin so the surge truly causes the slowdown (otherwise label "scripted degradation") | Retained 10-repetition result with `TRADE_OFF_FAILURE` read out loud, cost before benefit |
| **D Tenant attack** | Optional side act "A noisy neighbour" | Confidentiality isolation (tenant from token, header ignored, tenant-scoped DELETE) real; **no capacity isolation** | Tenant-labelled load; per-tenant quotas to defend | Two tenants on two client devices: prove (a) no data bleed, (b) DELETE of alpha leaves beta, then (c) run the flood as a **measured experiment with a pre-stated criterion** and report the result, whichever way it falls |
| **E Node failure** | Beats 4-5 purge under load, node goes away | Fencing and counters; process kill; repair and anti-entropy in the experimental mTLS cluster | Per-key causal event chain; multi-process cluster run kind; cluster PKI tool; durable log for the "fixed" version | Kill a gateway process: its clients lose service, peers unaffected, owed invalidations repaired after return; then kill the **issuer** to show the M3 defect labelled as a known limit. Do not use the `NODE_FAILURE` button as a real failure |
| **F Evidence report** | Beat 6, but accreting through the whole show | Evidence viewer, artifact hashes, reproduce command | Report assembly, citation guard, hash-chained session log, verifier | Bundle per beat: transcripts, origin log, counters before/after, `SHA256SUMS`; state "integrity only, not authenticity" until signing exists |

### 14.2 Three devices

| Device | Runs | Credentials |
|---|---|---|
| D1 Presenter / Control Center | UI, Lab API, evidence viewer, pairing script | Operator + Observer (never shared) |
| D2 Cache node | Gateway(s) and the scripted origin | Holds the JWT secret |
| D3 Clients | Two browser profiles or curl sessions as tenants alpha and beta | Short-lived scoped tokens |

For stage E on three devices, a second gateway (separate process, loopback-labelled `SINGLE_HOST_MULTI_PROCESS`) is run on D2, or on D1 as an mTLS peer if cluster PKI is available. Be explicit on screen that clients, tenants and nodes are different axes and that the client chooses its node by URL.

### 14.3 What to say, and not say

Say: "numbers are measured on loopback/one LAN, scripted origin, debug or release build X"; "the cap did what it promises mechanically and the trade-off failed the pre-set criterion". Do not say: "predicts", "learns", "protects tenant B", "automatic failover", "everything is purged everywhere", any speed-up factor across tiers.

### 14.4 Recommended build order for the flagship

1. Interim version above (P0). Rehearse twice on the three real devices.
2. Add tenant-labelled load and the noisy-neighbour experiment.
3. Add the event tap, then per-key causal chains for the purge beat.
4. Add quotas and re-run the noisy-neighbour beat as a before/after (section 18 idea).
5. Only then add live warm/refill and, if the headroom test passes, a forecast panel.

---

## 15. Capability classification table

Classes: **Existing** (works in code today), **Partial**, **Proposed** (needs new code), **Hypothesis** (research, may be false), **Prototype candidate** (can be tried with little or no `src` change), **Unsupported** (cannot be shown honestly now).

| Capability | Class | Evidence / blocker |
|---|---|---|
| Namespace isolation in secure mode | Existing | `server.rs:1209-1219` `[V]` |
| Per-tenant quotas / fair eviction / tenant telemetry | Unsupported (Proposed) | Appendix A #1 |
| Multiple origins per gateway | Unsupported | `main.rs:253` `[V]` |
| Static N-node mTLS cluster | Partial (experimental) | 3 nodes at most exercised, loopback `[R-V3]` |
| Dynamic join/leave | Unsupported | No `update` caller `[V]` |
| Client-visible failover / read routing | Unsupported | GET local-only |
| Durable invalidation across issuer crash | Unsupported; fix Proposed (scratch) | M3 `[R-V4]`; no durable log in `src/` `[V]` |
| Faithful eviction in the serving cache | Existing | `cache.rs:209`, `eviction.rs:690-698` `[V]` (O(1) claim not re-verified) |
| Truthful hot keys | Existing (library) / Prototype (wiring) | `hot_keys` hard-coded unavailable `[V]` |
| Per-request event stream | Unsupported (Proposed) | 100 ms aggregate only |
| Intelligence engine in the serving path | Unsupported | `pie: None`; library only `[V]` |
| Audit of intelligence decisions | Partial | Store exists, never appended `[V]` |
| Demand Calendar / event-driven warm | Prototype candidate (deterministic core); Proposed in-gateway | None in code `[R-V1,V2]` |
| Seasonal / Bayesian forecasting | Hypothesis | No real trace; headroom unknown |
| Flash-crowd classifier, metastability warning | Prototype candidate (Observe) | Detectors in library |
| Phantom Advisor (what-if, cap sizing) | Prototype candidate | Lab exists; simulator fidelity unvalidated |
| Staleness-risk estimator | Proposed | Needs propagation metrics |
| Key-fragmentation detector | Hypothesis | Needs realistic traffic |
| Shadow-selected eviction | Hypothesis | Crowded; hit-ratio-only CI is unpaired `[R-V1]` |
| Adaptive origin cap (AIMD) | Existing as lab code; benefit Unsupported | H1 NOT_SUPPORTED |
| Learned eviction, deep forecasting, RL caps, LLM in path | Unsupported | Section 16 |
| Scenario B (real gateway, scripted origin) with evidence | Existing | `[R-V5]`; trade-off failure retained |
| Flash-crowd workload on the real gateway | Proposed | Zipf-only loadgen `[R-V5]` |
| Demo Center shell, scenarios, recorded playback | Proposed (reuses `#/lab`) | UI change |
| Presenter/audience/observer views, projection mode | Proposed | |
| Multi-device pairing | Proposed (Prototype as external script) | No minter shipped |
| Token revocation, stream closure at expiry | Unsupported (Proposed) | `server.rs:955-972` `[V]` |
| Cluster PKI generator | Proposed | Test harness only |
| LAN single gateway with N clients | Partial | Never exercised end to end |
| Linux / ARM / macOS | Unsupported | Untested |
| Hash-chained signed evidence and verifier | Proposed | `phantom-verify` absent `[V]` |
| Security demos A3-A6, A8 (auth), S-05..S-07, S-12, S-13, S-15 | Existing (tests), runner Proposed | `[R-V4]` tests not re-run by me |
| Link-level fault injection as a product feature | Proposed | Test code only |
| OS-level partition/latency | Hypothesis | Privileged |
| Performance or distribution claims on LAN | Unsupported | No release, multi-host, repeated measurements |

---

## 16. What we will not claim

1. That PhantomCache is "intelligent", "AI-powered", "predictive", "self-tuning" or "autonomous" in production. The engine is not in the serving path and the only adaptive-control experiment is negative.
2. That any feature is novel. No verified primary source supports a novelty claim; RT-0 is open.
3. That a forecaster improves hit ratio, latency or origin load. All evidence would be synthetic until a real trace exists.
4. That tenants are protected from each other's load. Only the namespace is isolated.
5. That a DELETE 204 means the object is gone everywhere, or any "linearizable cluster delete", or bounded staleness beyond the origin lifetime and the 24 h cap.
6. That the cluster "fails over" or "routes around" failures for reads. GET is local-only.
7. That `NODE_FAILURE`, `FLASH_CROWD` or the three standalone demo nodes are real faults or real nodes.
8. That loopback or single-host numbers represent production, LAN or WAN performance; any cross-tier "x faster".
9. That the system is "secure", "zero-trust", certified or compliant. Demos show specific controls under specific fixtures.
10. That passing tests prove correctness; most were written by the code's authors and several assert stub behaviour.
11. That debug-build, single-run or simulated numbers are measurements.
12. That the UI's accessibility is proven (static linting plus partial browser rounds; forced-colors and screen readers unverified `[R: ledger]`).
13. That WSL2, VM and physical results are interchangeable.
14. That demo evidence bundles prove authenticity (without signing and an out-of-band key they prove integrity only).
15. That token theft is mitigated beyond short expiry (no revocation).
16. That Linux, ARM or macOS work.
17. That dollar savings or "Phantom OFF" counterfactuals are real. The old UI fabricated them.
18. That predictions act on traffic. Only OFFLINE REPLAY is possible today.
19. That the effective default TTL is configurable. `PHANTOM_DEFAULT_TTL_MS` is not wired `[V]`.
20. That cross-tenant timing and occupancy channels are absent. They are untested.

---

## 17. Near-term priorities under the current gates

Gate recap: no integration of unreviewed changes during this phase; the sealed-seed salt is never generated in an agent session; frozen areas need your approval; dependency-ready work continues. Ordered list:

1. **Keep going on what is already in flight**: browser verification of the Lab UI, the TTL decision track, the M3 durability scratch work, the E2 protocol draft, the baseline and statistics work (all documented in the progress ledger). Nothing in this report asks to interrupt them.
2. **Decide the cheap policy items** that unblock scripts and docs: D-01, D-02, D-03, D-17-D-19, D-30, D-37, D-39, D-40.
3. **Write (outside `src/`) the launcher and preflight script and the token minter**; rehearse the interim flagship on two or three real devices. No application code.
4. **Run the existing security tests as an inventory** (lead-run, `-j 1`): record name, pass/fail, commit, dirty list; do not claim anything until done. Then supersede the incorrect security matrix document (D-31).
5. **Prototype the out-of-process warm script** against the existing gateway in secure mode for public content, with origin-log measurement; label results EXPLORATORY.
6. **Prepare, do not freeze**, the protocol-v4 draft and the Demand-World generator design; prepare the out-of-session freeze procedure for you to execute; keep exploratory dry runs on dev/tuning seeds only.
7. **Fund RT-0** (prior-art sweep) and **seek a real trace** (D-11, D-38).
8. **Design (do not implement) the event tap and per-tenant accounting** for review; these touch the hot path and the fenced cache core.
9. **Ask for decisions on D-04, D-05, D-06** so P1 can start the moment the integration gate opens.
10. Defer: dynamic membership, multi-origin, QR, VMs/containers, remote participants, any in-gateway actuator, any ML.

---

## 18. Integrator's critique, and better ideas (my view, clearly marked)

**Where I think the V1-V5 consensus is wrong or too optimistic**

1. **The benefit zone for predictive preparation may be narrow.** With the effective default lifetime fixed at 60 s, warming ahead of an event is uneconomic except for objects whose origin sets long `max-age`, and pre-warming only works for public content because the gateway forwards no client identity. None of the five reports puts these two facts side by side. The headroom test E9 might well come back negative; plan for that outcome. `[H]`
2. **Single-flight already absorbs the same-key stampede.** The remaining gap is N distinct cold keys at the same instant. An origin cap plus stale serving may already cover most of it; the incremental value of warming must be measured against that, not against "nothing" (V2's baseline set B0-B8 does this; the demo must too).
3. **"Demonstrable today" in V4 is over-generous.** It means a test exists, not that a presentable demo exists. No evidence runner exists, tests were written by the same authors and not re-run in the research pass, and I could not confirm the independence of the "independent reviews". Treat the E statuses as "controls tested", not "demo-ready".
4. **The LAN demo is rated Partial but has never been run.** Windows firewall defaults, client-isolated Wi-Fi, CORS exact-origin rules, loopback-origin refusal on network bind, clock skew and certificate trust are each a likely stage failure. Two full rehearsals on the real devices should be a gate, not a nice-to-have.
5. **Scope.** The five reports together propose around ten new components (event tap, quotas, session service, pairing, runner, verifier, PKI tool, fault proxy, warmer, report builder). V1 itself concedes (§11.3) that the real blockers are durable invalidation, quotas, multi-origin, dynamic membership, persistence and per-request observability, not intelligence. I would cut the Demo Center to: recorded playback, the interim flagship, the evidence bundle, and one live beat, and keep the rest as research.
6. **V1's scores are unweighted and subjective;** rank 1 (Observe wiring) is infrastructure, not differentiation. I treat it as foundation.
7. **Documents drift.** Within this one pass I found V1's eviction uncertainty resolved the other way, V5's "never rendered in a browser" superseded, `CLAUDE.md` §6 "evictors are scans" outdated, `CLAUDE.md` §5's literal-60,000 line numbers moved, and the security matrix referring to files that do not exist. Anything cited from these documents should be re-verified before it goes on a slide.

**Ideas I think are better than, or additional to, the proposals** (all `[H]`, unevaluated, no novelty claim)

- **"Cache Game Day" as the headline product idea.** Combine the Advisor, the deterministic lab and the config check: *rehearse your cache configuration against a scripted or replayed incident before it happens*, with a calibrated what-if answer and a pass/fail against your stated limits. It uses assets the repo already has and that incumbents were not shown to offer (unverified), needs no production traffic, and does not require any controller to beat a static setting.
- **Shadow "would-have-warmed" ledger.** Before any warm actuator exists, run the warm planner in L3 shadow mode against real declared events and count hits it would have saved and origin fetches it would have spent. This measures benefit with zero exposure and produces the data to decide whether actuation is worth building.
- **Make the tenant story a before/after, not a show.** Run the noisy-neighbour experiment first and show the measured harm; build quotas; re-run and show the measured fix with an interval. This is both the most honest and the most compelling tenant demonstration, and it exercises the whole evidence pipeline.
- **A self-testing preflight.** The Demo Center runs a short subset of the existing security and invariant tests at start and prints pass/fail with test ids and commit. It makes "what is real here" a checked fact rather than a badge.
- **Refusal log as a feature.** Display why the system did nothing (the sixteen refusal codes). It is cheap, it is the visible form of "intelligence never bypasses the caps", and most competing dashboards do not show it (unverified).
- **Purge-with-refill and soft purge** (V2): target the cold misses that invalidation itself causes with no demand prediction.
- **Decide peer-fill (owner-fetch) deliberately.** Because the origin gate is per process and GET is local-only, adding nodes can multiply origin fan-in. Whether reads should ask the key's rendezvous owner before the origin changes what signals and caps mean; decide it before building cluster-aware intelligence. Large change, fencing risk, catching up with tiered caching rather than leading.
- **Treat the calendar as a security object from day one.** A compromised or mistaken entry can burn the origin or mask an attack; V2 and V4 agree, and the abuse cases should be written as tests before the feature.
- **Pre-agree what a negative E9 means.** If the oracle shows no headroom, the product keeps the Advisor, the deterministic warm/refill and the observation layer, and says so. A retained negative result is part of the brand, so plan the narrative before the result arrives.

---

## Appendix A. Code-claim spot-check (verified / not verified / wrong)

I opened the cited source myself (read, grep or listing) in this pass. "Verified" means the claim matches the code as read; it does not mean the behaviour was executed. 28 claims checked.

| # | Claim (source) | Result | Evidence I saw |
|---|---|---|---|
| 1 | No per-tenant quotas or tenant-attributed telemetry (V3 §1.1.5/7, V4 §0, V5 §1.1) | **Verified** (absence, by grep) | `quota|per_tenant|tenant_limit|tenant_bytes` over `src/` excluding `lab`/`intelligence`: no hits; no "tenant" in `src/intel/telemetry.rs` |
| 2 | Single static `ORIGIN_URL` per gateway (V3 §1.2) | **Verified** | `src/main.rs:253` reads one `ORIGIN_URL`; `src/config.rs:125` |
| 3 | Cluster membership is static; no runtime caller of registry update (V3 §1.3) | **Verified** (no caller in `src/`; tests not checked) | `ClusterMembershipRegistry::update` defined `src/security/cluster.rs:405`; no use in `main.rs`, `dist/`, `security/` |
| 4 | Dev (non-secure) mode trusts client-supplied `x-tenant-id` (V3, V4 §0.3) | **Verified** | `src/api/server.rs:1209-1219` (header branch only when `state.security` is None), `header_tenant` `:1126-1134` |
| 5 | `NODE_FAILURE` only sets node id 2 Failed; any other event resets all Active; unauthenticated in dev mode (V3 §1.4, V4 §0.6) | **Verified** | `src/api/server.rs:1089-1092` and reset branch; `control_handler` `:1070-1081` checks auth only when security is configured |
| 6 | In cluster mode the API `nodes` and the router share the same node objects, so the flag changes real routing state (V4 §0.6) | **Verified** (code sharing; behaviour not run) | `src/main.rs:374` `RendezvousRouter::new(nodes.clone())` and `:430` `nodes` into `ApiState` |
| 7 | Origin gate is process-wide (V1 §11.2) | **Verified** | `src/intelligence/executor.rs:75-80` "Process-wide origin dispatch gate", atomics |
| 8 | `hot_keys` is unavailable (V1 §1, V5) | **Verified** | `src/api/server.rs:766` |
| 9 | DELETE fans out; RepairLedger is in-memory and bounded; no durable log (V3 §1.4, V4 §6) | **Verified** (structure; M3 behaviour not re-run) | `src/dist/router.rs:137` `Mutex<HashMap>`; `:54-60` bounds; `src/api/server.rs:1995` `invalidate_key_across_cluster`; no "durable" in `src/` |
| 10 | `git_tree_hash` hard-coded in lab manifest (V4 §2 A2) | **Verified** | `src/lab/manifest.rs:103` literal `456bb259...` |
| 11 | Peer invalidation scope not authorised; only originator==TLS identity is checked (V4 §1.2, S-14) | **Verified** | `src/dist/node.rs:508` originator check; `:547-553` `Tenant`/`FullCache` applied with no scope check |
| 12 | `x-cache-bypass` goes to the origin unrouted by the coalescer, gated only by the permit (V4 S-09) | **Verified** (code path; "authenticated first" from V4 test name/comment only) | `src/api/server.rs:1243-1276` `fetch_origin_gated` |
| 13 | `pie: None` and hard-coded `expected_utility = 85.0` (CLAUDE.md §6b) | **Verified** | `src/main.rs:433`; `src/control/pie.rs:121` |
| 14 | `PHANTOM_DEFAULT_TTL_MS` unwired; literal 60,000 passed to freshness (CLAUDE.md §5) | **Verified** (line numbers drifted) | Config key `src/config.rs:116`; literals `src/api/server.rs:1405`, `:1688`; no other reader found |
| 15 | Intelligence library has no production wiring (V1 §1) | **Verified** | `WorkloadPipeline|ShadowEvaluator|HeavyHitterDetector|route_bucket` referenced only under `src/intelligence/` |
| 16 | `AuditStore` is created in `main` but never appended in production (V1 §1, left unchecked) | **Verified** | `append` defined `src/intelligence/audit.rs:33`; created `src/main.rs:480`; no call sites elsewhere in `src/` |
| 17 | Serving cache does not use faithful eviction (V1 §1 "probably not"; CLAUDE.md §6 "evictors are scans") | **WRONG** (V1 uncertain, CLAUDE.md §6 outdated) | `src/core/cache.rs:209,223` `Box<dyn ShardEvictionPolicy>` via `create_shard_eviction_policy`; `src/core/eviction.rs:690-698` returns FaithfulSieve/Lru/S3Fifo. O(1) behaviour itself not verified |
| 18 | `kid` rotation exists but is unwired in the binary (V3 §1.5) | **Verified** | `with_rotation_key` defined `src/security/jwt.rs:130`; `src/main.rs:413` builds `JwtValidator::new` only |
| 19 | Non-loopback bind disables loopback origin (V3 §1.2) | **Verified** | `src/main.rs:406` `allow_loopback_origin: !is_network_bind` |
| 20 | `tests/security/` and `phantom-verify` do not exist; matrix error names not in code (V4 §0.1) | **Verified** | `ls tests/security` fails; `src/bin/` holds only `phantom-lab.rs`; `UpstreamDestinationDisallowed` appears only in `docs/security/security-demo-matrix.md`; real names at `server.rs:2277-2284` |
| 21 | `/api/events` loop has no token-expiry check (V3 §1.5) | **Verified** (in the loop read; auth at entry not re-read) | `src/api/server.rs:955-972` |
| 22 | Standalone mode has three placeholder `VirtualNode`s; cluster local node shares the HTTP cache (V3 §1.3) | **Verified** | `src/main.rs:211-215`; `:327` `VirtualNode::new_with_cache(node_id, cache.clone())` |
| 23 | No "flush all" HTTP route (V3 §7) | **Verified** (absence, by grep) | No `try_invalidate_all`/`invalidate_tenant` in `src/api/server.rs` |
| 24 | Cluster GET handler fabricates `val_node_N` bodies (V4 §1.2) | **Verified** | `src/dist/node.rs:436-458` |
| 25 | Tenant epoch overflow falls back to global (CLAUDE.md §5; V3) | **Verified** (constant and guard) | `src/core/cache.rs:273,581` |
| 26 | `requester` role alias grants read and invalidate (V3 §1.1.4) | **Verified** (alias present; both grants inferred from lines 66-71) | `src/security/jwt.rs:66,71` |
| 27 | GET computes a real `key_hash` (V1 §1, overturning `pie_audit.md` F5) | **Verified** | `src/api/server.rs:1255-1262` (`fnv1a_64` of the canonical key) |
| 28 | Dev CORS allowlist is localhost:3000 and 127.0.0.1:3000 only (V3 §1.5) | **Verified** | `src/config.rs:550` |

**Not verified by me** (reported only): the M3 reproduction numbers; all test names and their results in V4; SEC-M7 item descriptions; the Lab API behaviours (Operator can spawn processes, 16-stream cap, token-expiry closure on the run stream); UI file claims and fixture contents; every literature item; the repo's claims ledger and accountability report; V2's code references into `src/lab` (e.g. `t1_sim.rs:278-320`); the O(1)-amortised property of the eviction policies; whether `FullCache` frames are ever sent by a legitimate path other than repair.

**Claims found wrong or stale:** #17 (V1 and CLAUDE.md §6); line-number drift in CLAUDE.md §5 (#14); V5's "UI never rendered in a browser" (R18); the security matrix's file and error names (#20).

## Appendix B. Terms

- **Single-flight / coalescing**: many concurrent misses for one key produce one origin fetch.
- **Generation fencing**: an origin fetch records a generation; its result is stored only if the generation and epochs are unchanged, so a late response cannot resurrect a deleted key.
- **Tenant epoch**: a counter that invalidates all of a tenant's entries at once.
- **Hard caps**: limits enforced by the execution layer regardless of any decision (stale-serving window, origin concurrency, byte/entry budgets).
- **Provenance label**: measured, derived, simulated, declared, unavailable.
- **Tier (T1/T2)**: T1 is a deterministic simulation using the real cache code in virtual time; T2 is a real gateway process on loopback with a scripted origin and real time.
- **Headroom test**: ask whether an oracle that knows the future beats the best realistic baseline; if not, prediction is not worth building.
- **MASE**: forecast error scaled by a naive baseline's error (below 1 beats the baseline).
- **Conformal / coverage**: a method for prediction intervals; coverage is the fraction of actuals that land inside the stated interval.
- **Canary / holdback**: apply a change to a small randomised subset while another subset is left untouched, to measure the effect.
- **M3**: the invalidation-durability defect (a 204 DELETE can be lost if the issuer dies before the peer is told).
- **Game day**: a rehearsal of a failure before it happens.
