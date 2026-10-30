# V1 - Intelligence Opportunity Map (product-discovery research, agent V1 of 5)

**Status: RESEARCH ONLY, PROPOSED. Not a plan of record. No code, config, or other document was changed; no cargo, git mutation, browser, dependency or experiment was run.** Date: 2026-10-30. Tree at start: HEAD `456bb25`, branch `feat/publish-phantomcache-core`, 72 dirty entries (read only).

**Evidence tags used throughout:** `[V file:line]` verified in code by me in this pass; `[R]` reported by a repo document or prior agent, not re-derived; `[L-V]` literature, bibliographic metadata matched on a primary-source page returned by the search tool (full text NOT read); `[L-S]` literature, secondary/vendor/mirror only; `[L-R]` literature cited from repo docs, not searched; `[A]` assumption; `[H]` hypothesis. "Novelty" is never claimed anywhere in this file.

---

## 0. Framing and what the evidence does and does not say

- The only adaptive-control result in the repo is negative: an absolute-ceiling AIMD origin-concurrency controller, started at max, scored on peak concurrency, did not beat the best static cap in simulation (H1 NOT_SUPPORTED) `[R: m6_sealed_results.md; accountability report 11.3]`. The accountability report's own net statement: this is NOT evidence that workload-aware intelligence, forecasting, hot-key detection or ML-assisted selection is ineffective, and NOT evidence that they work (`phantomcache_milestone_accountability_report.md` 11.3, last paragraph) `[R]`.
- The repo's own recommended direction is a hybrid: deterministic gateway, observation layer, advisory "Phantom Advisor", policy/safety boundary, offline lab; serving-path actuation only after pre-registered evidence (11.5, 11.6) `[R]`. This map extends that recommendation; it does not contradict it.
- This map compares **heuristics, statistical forecasting, online learning, ML, and hybrids** per candidate and always names the strongest non-intelligent baseline. A "declared/operator-informed" input is treated as a first-class alternative to prediction, because it can remove the need to predict at all.
- **Safety invariant for every candidate (design rule, restating CLAUDE.md 5/6b and `phantom_intelligence_phase.md` 1):** intelligence proposes; the execution layer disposes. Intelligence may not alter freshness lifetimes beyond origin directives, `PHANTOM_MAX_STALE_MS`, generation/tenant-epoch fencing, byte/entry budgets (I1-I8), tenant isolation, admission/origin caps, or the never-evict-in-flight rule. Every actuating candidate below is specified as a *clamped parameter proposal* that the executor validates.

## 1. What exists in code today (inventory, all `[V]` unless marked)

| Area | What is there | Evidence |
|---|---|---|
| Modes | `IntelligenceMode {Off, Observe, Advisory, Canary, Active}`, default `Off`; traits `Detector`, `Predictor`, `Evaluator`, `PolicyExecutor`; `SignalWindow` with per-field provenance (`MetricProvenance::{Measured, Derived, Simulated, Unavailable}`) | `src/intelligence/mod.rs:22-50, 96-113, 165-200` |
| Signal pipeline (library) | `WorkloadPipeline` bundling EWMA rate, `HeavyHitterDetector` (count-min + Space-Saving style, aged), spatially-sampled working set, CUSUM `BurstDetector`, `HotKeyEmergenceDetector` | `src/intelligence/signals.rs:382-455`; components at `:27`, `:95-193`, `:256`, `:309-346`, `:353-376` |
| Hard-coded detector constants | CUSUM threshold 500.0 / slack 50.0 in absolute rps-like units (not scale-free); hot-key "emergence" = >=3x previous count and count >=30 | `signals.rs:404`; `signals.rs:363` |
| Shadow evaluation (library) | `ShadowEvaluator` runs LRU/SIEVE/S3-FIFO/TinyLFU shadows on a hash-sampled key stream; compares **hit ratio only**; CI = difference of two independent proportions (same stream feeds both, so independence is wrong) | `src/intelligence/shadow.rs:15-31, 186-393, 418-484` (CI at `:452-463`) |
| Candidates (library) | shadow-selected eviction, frequency admission filter, origin-aware stale controller, AIMD cap controller, EWMA rate forecaster with MASE vs persistence, hot-key emergence predictor, decision stump | `src/intelligence/candidates.rs:29, 65, 97, 135, 175, 251, 278` |
| Forecaster memory | `predictions`, `actuals`, `naive_predictions` are unbounded `Vec`s | `candidates.rs:179-181, 205-207` (violates "keep resource bounds explicit", CLAUDE.md 5) |
| Executor / canary routing | `BucketArm {Holdback, Candidate, Control}`, `route_bucket(key_hash)`, safe baseline config, watchdog, revert counter | `src/intelligence/executor.rs:24-47, 270, 285-300, 390` |
| Origin gate | process-wide atomic in-flight + cap, RAII permit, AIMD controller hook | `executor.rs:78-165` |
| Audit | bounded `AuditStore`; created in `main` | `src/intelligence/audit.rs:17-33`; `src/main.rs:480` |
| Production wiring | Routes `/api/v1/intelligence/status` and `/audit`; `hot_keys` reported `src: "unavailable", reason: "not_implemented"`; **no reference to `WorkloadPipeline`, `ShadowEvaluator`, `HeavyHitterDetector` or `route_bucket` anywhere outside `src/intelligence/`** (grep, this pass). Whether `.append(` is ever called on the audit store was not re-checked here | `src/api/server.rs:213-214, 766`; grep; `[R: accountability report 11.2]` for the append finding |
| Telemetry primitives | `AtomicLogLinearHistogram` (2,688 buckets per CLAUDE.md), `TelemetryCollector`, `OriginMetrics` (origin latency histogram, in-flight, error counts), `CacheCounters`; legacy `CountMinSketch` | `src/intel/telemetry.rs:280-413`; `src/intel/origin_metrics.rs:12-138`; `src/intel/diagnostics.rs:28-150`; `src/intel/sketch.rs:3-42` |
| Real key identity for observation | request handler computes `key_hash = fnv1a_64(canonical key string)` and passes it to telemetry. **This supersedes the "key_hash always 0" finding in `pie_audit.md` section 5/F5, which is now outdated at the call sites seen** | `src/api/server.rs:1261, 1299, 2116-2120` |
| Static workload profiles | `WorkloadProfile` presets (travel, e-commerce, ticketing, general API) are hard-coded constants for the demo/lab, not learned | `src/intel/workload.rs:2-50` |
| Cache-side hooks useful to intelligence | tenant-scoped `CacheKey` (`tenant_id` first segment) | `src/core/cache.rs:24-32, 98-112` |
| | bounded background revalidation slots (`max_background_refresh`, default 128; hard ceiling 256) and per-key revalidation backoff (1 s doubling to 60 s) | `cache.rs:291, 337, 983, 1084-1122`; `executor.rs:21` |
| | freshness lifetime precedence `no-cache` > `s-maxage` > `max-age` > `Expires` > literal default 60 s; no heuristic freshness; `PHANTOM_DEFAULT_TTL_MS` unwired | `src/core/freshness.rs:139-293` via `[R: ttl_decision/recommendation.md F1-F4]`; `src/config.rs:115-118` per CLAUDE.md |
| Faithful eviction | `ShardEvictionPolicy` with FaithfulSieve/Lru/S3Fifo exists in `src/core/eviction.rs:16-56` and is used by the lab (`src/lab/eviction_baselines.rs:12, 223-227`). A grep of `core/cache.rs`, `main.rs`, `api/*.rs` found no use of `core::eviction`, so **the serving cache probably still uses its internal enum policies; not confirmed** (CLAUDE.md section 5 and section 6 disagree on this point; I flag the conflict, not resolve it) | greps this pass |
| Not in the product | any prefetch/pre-warm actuator, any calendar/schedule concept, any request log, any hot-key output, any seasonality model | grep for prewarm/prefetch/calendar/schedule in non-lab `src/` found only revalidation/refresh machinery; `[R: report 11.4]` "gateway writes no request log" |
| Legacy PIE | `src/control/pie.rs` threshold stub, `pie: None` in `main`; quarantined | CLAUDE.md 6b; `[R: pie_audit.md]` |

Conclusion of the inventory `[V + R]`: a **library of plausible detectors and shadow policies exists, none is wired to the serving path or exposed truthfully**, and several have statistical flaws (non-paired CI, hit-ratio-only objective, unbounded vectors, absolute-unit thresholds). Everything below is therefore a proposal on top of unproven parts.

## 2. Literature coverage record (prior art; no novelty claims)

**Method this pass:** WebSearch, `standard` mode, 16 queries, no full texts opened, no WebFetch. "L-V" = title/authors/venue/year matched on a primary-source or proceedings page returned by the search; abstract-level only. The repo's earlier pass (`intelligence_prior_art_coverage.md`, 40 queries, 52 rows) is reused for items it already verified (cited as `PAC id`). **RT-0 remains open (`[R]`); absence of results below is not evidence of absence.**

Queries run (this pass):
1. Flash crowds and denial of service attacks Jung Krishnamurthy Rabinovich WWW 2002
2. Netflix Open Connect proactive caching fill window popularity prediction pre-positioning
3. ADWIN Bifet Gavalda SDM 2007
4. Miniature Simulations / SHARDS Waldspurger
5. Prophet "Forecasting at scale" Taylor Letham
6. hot key detection distributed cache load balancing NetCache SwitchKV
7. Counterfactual Reasoning and Learning Systems Bottou Peters
8. Conformalized quantile regression Romano Patterson Candes
9. CDN cache prefetch/preload/warming API (Cloudflare, Fastly, Akamai, CloudFront)
10. Memshare / Cliffhanger multi-tenant cache memory
11. Page 1954 CUSUM
12. bandit / Thompson sampling cache admission; Holt-Winters request forecasting (poor results)
13. The Tail at Scale (Dean, Barroso)
14. Probabilistically Bounded Staleness (Bailis)
15. Cho and Garcia-Molina freshness / frequency of change
16. learned per-object TTL / update-rate cache expiration

| ID | Work | Class | What was matched | Used by candidate |
|---|---|---|---|---|
| P1 | Jung, Krishnamurthy, Rabinovich, "Flash Crowds and Denial of Service Attacks: Characterization and Implications for CDNs and Web Sites", WWW 2002, https://archives.iw3c2.org/www2002/CDROM/refereed/342.html | L-V (abstract/slides/mirror; full text unread) | flash events vs DoS differ in per-client behavior; CDNs can still fail under flash events; proposes adaptive CDN | C4, X-series |
| P2 | Netflix Open Connect overview (openconnect.netflix.com/Open-Connect-Overview.pdf): proactive off-peak fill driven by predictions | L-S (search summary of vendor page; not opened) | pre-positioning is established practice for a *finite catalog with predictable demand* | C1, C2 |
| P3 | Bifet, Gavalda, "Learning from Time-Changing Data with Adaptive Windowing", SDM 2007, pp. 443-448 | L-V (SIAM listing via search) | ADWIN adaptive window change detector with false-positive/negative bounds | C9, C13 |
| P4 | Page, "Continuous Inspection Schemes", Biometrika 41(1/2):100-115, 1954 | L-S (secondary sources; text unread) | origin of CUSUM | C4, C9 |
| P5 | Waldspurger et al., SHARDS, FAST '15; miniature simulations, ATC '17 | L-V (usenix pages; SHARDS memory/time factors are author-reported) | spatially hashed sampling for MRC and miniature cache simulation | C5, C8, C14 |
| P6 | Taylor, Letham, "Forecasting at Scale", The American Statistician 72(1):37-45, 2018 (Prophet) | L-V (RePEc record) | trend + Fourier seasonality + holiday-style regressors, analyst-in-the-loop; secondary source flags it as not peer reviewed in one summary - unresolved | C2 |
| P7 | NetCache (SOSP 2017), SwitchKV (NSDI 2016), Fan et al. small front-end cache (SOCC 2011) | L-V (venues as returned) | hot-item detection and small caches for load balance in KV clusters; switch-hardware specific | C3, X7 |
| P8 | Bottou, Peters et al., "Counterfactual Reasoning and Learning Systems", JMLR 14:3207-3260, 2013 | L-V (JMLR page) | off-policy/counterfactual estimation from logged randomized decisions | C11 |
| P9 | Romano, Patterson, Candes, "Conformalized Quantile Regression", NeurIPS 2019 | L-V (proceedings page) | distribution-free finite-sample prediction intervals | C12 |
| P10 | Cloudflare "Prefetch URLs" documentation (developers.cloudflare.com/speed/optimization/content/prefetch-urls) | L-S (search summary; appears to be a *visitor-side next-page hint* feature, not event-scheduled edge warming - interpretation unverified) | existence of CDN prefetch features; Akamai/CloudFront/Fastly prefetch status NOT verified (conflicting secondary sources) | C1 |
| P11 | Cidon et al., Memshare (USENIX ATC '17); Cliffhanger (NSDI '16) | L-V | tenant-aware dynamic memory allocation using hit-rate-curve gradients / shadow queues | C7 |
| P12 | Dean, Barroso, "The Tail at Scale", CACM 2013 | L-S (secondary write-ups only) | hedged requests | not pursued (section 8) |
| P13 | Bailis et al., PBS, PVLDB 5(8):776-787, VLDB 2012 | L-V | probabilistic staleness bounds, k-staleness / t-visibility | C12, X6 |
| P14 | Cho, Garcia-Molina, "Synchronizing a Database to Improve Freshness", SIGMOD 2000 (DBLP-confirmed); "Estimating Frequency of Change", ACM TOIT 3(3):256-290, 2003 (DBLP-confirmed, content unread) | L-V (bibliographic only) | refresh scheduling and change-rate estimation | C6, X4 |
| P15 | Lee et al., "An Update-Risk Based Approach to TTL Estimation in Web Caching", WISE 2002 | L-S (KAIST repository + PDF mirror) | per-object TTL from Poisson update model and a user-set risk | C6, X4 |
| P16 | RLCache (arXiv:1909.13839); delayed experience injection (arXiv:1610.09903) | L-S (arXiv listings only) | RL-learned TTLs | not pursued (section 8) |
| P17 | arXiv:2607.00394 "Learning-Augmented Replacement for Semantic Retrieval Buffers" (Thompson-sampling eviction regret bound) | L-S (preprint; peer review unconfirmed; semantic-retrieval setting, not HTTP) | bandit eviction with regret bound under stationarity | C13 |
| PAC | From `intelligence_prior_art_coverage.md` (class A rows): Space-Saving HK1, Count-Min HK2, TinyLFU EV1, ARC EV2, SIEVE EV4, S3-FIFO EV5, CACHEUS EV6, LeCaR EV7, LRB EV8, GL-Cache EV9, LHD EV10, Darwin EV11, AdaptSize EV12, Hyperbolic EV13, LAH EV14, Basu adaptive TTL FR3, Mao freshness FR4, RFC 5861 FR2, Varnish grace OP1, Fastly collapsing OP3, XFetch OP4, Memcache leases OP5, delayed hits OP6, RobinHood OP8, metastable OP9/OP10, Envoy/Netflix limiters CC1-CC3 | as stated there | `[R]` + L-V | all |

**Coverage gaps (stated, not hidden):** no systematic proceedings sweep; ACM DL/IEEE/DBLP/Scholar not queried directly; **no literature found or searched by name for: operator-declared demand calendars feeding a cache; early flash-crowd detection inside a caching gateway; post-flush/restart warm-up scheduling for in-memory gateway caches; per-tenant seasonality for HTTP gateways; origin-pressure forecasting for admission** (this last gap is also recorded in PAC 2.6). The search tool is US-only and indexes blogs better than proceedings. Patents were not read.

## 3. Candidate catalogue

Common column definitions. *Strongest baseline* = the best non-intelligent comparator that must be beaten. *Falsification* = the result that kills the candidate. *Class* uses the brief's five labels. Overheads are estimates `[A]` unless measured; none has been measured (E0 is the planned measurement, `[R: report 11.6]`). Every actuating candidate runs only after the progressive-activation gates in section 6.

### C1. User-informed demand: Demand Calendar + predictive preparation

- **Real problem:** the first requests of a known event (on-sale, flash sale, product launch, scheduled live page, batch import) arrive at an empty or expired cache; N distinct cold keys each cost an origin fetch, concurrently. Single-flight coalesces only per key `[R: report 11.7]`. Reactive intelligence cannot know an event is coming; the operator can.
- **Inputs:** operator-declared entries, authenticated and tenant-scoped: `{tenant, key set or path-prefix/list (bounded), start, end, expected load multiplier or rate (optional), lead time, priority, max_prewarm_bytes, max_origin_fetch_rate}`. Learned side-inputs: observed lift vs declared (calibration), per-key origin latency (`OriginMetrics`, `origin_metrics.rs:82`), freshness lifetime of the key's last response.
- **Algorithm (tiers):** (a) *Deterministic scheduler, no learning:* at `start - lead`, issue rate-limited fills through the normal fill path; schedule refresh-ahead so each key's lifetime spans the event (fetch time = `start - epsilon` if lifetime < lead). (b) *Statistical:* learn per-tenant lead time and lift error from past entries; propose better lead/size defaults. (c) *Optional:* suggest calendar entries from seasonality (C2) for operator confirmation, never auto-activate.
- **Output/action:** pre-warm fetches (via origin permit, low priority), temporary retention priority within a byte budget, temporary raise of background-refresh slots within `MAX_BACKGROUND_REFRESH_CAP` (`executor.rs:21`), optional Little's-law cap advice for the event window (C-X5).
- **Integration point:** new module outside `core/cache.rs` internals; reuses the existing fill/finalize path so generation fencing and tenant-epoch checks apply (`finalize_fetch` stores only if generation/epochs still match, CLAUDE.md 5) `[V by CLAUDE.md; code not re-read]`; calendar store = bounded, in-memory (restart loses it; config-file/env loaded at boot is the durable form); API under `/api/v1/intelligence/` (new, beside `server.rs:213-214`). New code; needs owner approval (CLAUDE.md 2).
- **Benefit over reactive `[H]`:** removes the t0 miss stampede for declared keys; makes origin load predictable; converts a prediction problem into an auditable declaration. Largest where origin cannot absorb the burst and TTL >= lead time. Smallest when events are unscheduled.
- **Overhead `[A]`:** negligible CPU; memory = entries x declared key set (hard-bounded, e.g. count and bytes); extra origin load = the pre-warm itself (accounted as "prewarm waste" if keys are not requested).
- **Failure cases / safety:** wrong time or keys -> wasted origin load and cache pollution (eviction pressure on real hot set): bounded by `max_prewarm_bytes`, per-entry fetch-rate limit, abort if origin p99/error crosses a threshold (reads measured origin health). Pre-warming a key whose TTL expires before the event is useless: scheduler must use lifetime, not assume. Pre-warm must not serve or store data beyond origin freshness directives or across tenants; must obey `Vary`/auth rules (uncacheable responses stay uncacheable, `freshness.rs` F5). Calendar entry is an attack surface (cache-fill amplification): authenticated, per-tenant quota, TTL on entries. Pre-warm must lose to real traffic in the origin gate. Hard caps unaffected.
- **Strongest baseline:** operator cron/curl cache-warmer script; plus generous static TTL/SWR; plus single-flight alone. Third-party warm-up crawlers exist `[L-S, P10 search]`.
- **Prior art:** Netflix Open Connect proactive fill `[L-S, P2]` (different setting: finite catalog, ISP-embedded appliances); CDN prefetch/preload features exist (Cloudflare documented, others unverified) `[L-S, P10]`; Cho & Garcia-Molina refresh scheduling `[L-V bib, P14]`. **No source found for a tenant-scoped declarative demand calendar with safety envelopes in a caching gateway; this is a coverage gap, not evidence of novelty.**
- **Falsification:** in the lab (T1, then T2 loopback), scripted event with N keys at t0, vs (i) reactive+single-flight, (ii) naive cron warmer. Metrics: origin peak concurrency and queue delay in the first 10 s, goodput, p99 of successful responses (never all-request p99, `[R: scenario_b_interpretation_notice]`), prewarm waste fraction. Include mis-specified calendars (time off by +-30 s/5 min, 50% wrong keys). Kill: no improvement over the cron warmer with CI excluding zero, or net harm under mis-specification beyond a pre-registered bound.
- **Differentiation plausibility `[A]`:** moderate. Value lies in the safety envelope, tenant scoping, audit, and calibration loop, not the warming itself.
- **Class: Prototype candidate** (deterministic core needs no intelligence; calibration is a Research hypothesis).

### C2. Calendar/seasonality prediction

- **Real problem:** many workloads have daily/weekly rhythm plus holidays; knowing the next hour's demand and hot set enables preparation and tighter caps without operator input.
- **Inputs:** per-tenant (and per path-prefix) counters bucketed hourly/by 5 min over >= several weeks; optional operator holiday list; optional declared events (C1) as regressors.
- **Algorithm ladder:** seasonal-naive (same slot last week) -> seasonal EWMA / Holt-Winters ETS -> Prophet-style additive decomposition `[L-V, P6]` -> ML. Estimate prediction intervals (C12).
- **Output:** advisory forecast and *suggested* calendar entries; no direct actuation.
- **Integration:** offline or slow background task consuming aggregated counters; new storage of bounded rolling buckets (count = prefixes x 168 hourly buckets x tenants, hard-capped). Not on request path.
- **Benefit `[H]`:** only if repeatable seasonality exists in the deployments that matter. Unknown: **there is no real traffic trace in the repo** (all workloads are synthetic `[R: lab families]`), so seasonality cannot be validated here and synthetic traces would test only what was put in them.
- **Overhead `[A]`:** low (batch). Memory bounded by caps above.
- **Failure cases:** regime change (launch, outage) invalidates the model; holiday misses; cold tenants (no history). Safety: advisory only; wide interval -> no suggestion.
- **Strongest baseline:** seasonal-naive; static operator-authored calendar (C1).
- **Prior art:** Prophet `[L-V, P6]`; Netflix demand prediction for fill `[L-S, P2]`; LSTM popularity forecasting for proactive caching `[L-S, PAC WF1-WF4]`, which also notes weak link between forecast accuracy and cache performance (WF4, abstract-level).
- **Falsification:** on >= 1 real held-out multi-week trace (owner must supply or a design partner), MASE vs seasonal-naive >= 1 -> drop. Without a real trace the candidate cannot be evaluated.
- **Differentiation:** low (generic forecasting). **Class: Research hypothesis** (blocked on data).

### C3. Predictive hot-key detection

- **Real problem:** (i) the cost of a miss on a hot key scales with rate x origin latency (delayed hits, `[L-V, PAC OP6]`); expiry of a hot key is the costly miss; (ii) in a cluster, hot keys need replication/awareness; (iii) operators want to see hot keys. Today `hot_keys` is `unavailable/not_implemented` `[V server.rs:766]`.
- **Inputs:** `key_hash` per request (`server.rs:1261`), hit/miss, tenant, remaining freshness lifetime of the key, origin latency of last fill.
- **Algorithm:** aged count-min + Space-Saving heavy hitters (exists, `signals.rs:95-193`); emergence = velocity test (exists, `signals.rs:353-376`, fixed 3x/30); alternatives: per-key EWMA over a sampled window, sampled exact counting, Misra-Gries `[PAC HK1-HK3]`. Prediction = "will be in top-k at t+delta" using velocity; baseline = current top-k persistence.
- **Output/action:** Observe: tenant-scoped top-k with provenance and error bound (replaces the `unavailable` field). Later, bounded actuation: refresh-ahead before expiry for top-k keys (uses the existing background slots and backoff, `cache.rs:983, 1084`), eviction protection within an entry/byte budget.
- **Integration:** `WorkloadPipeline` consumer in an off-path task fed by a bounded drop-newest ring (spec: `intelligence_architecture.md` 3); surface in `server.rs` hot-keys field; reuse existing structures. Needs the observation hook (application code, approval).
- **Benefit `[H]`:** observation: certain operator value. Actuation (refresh-ahead hot keys): removes synchronous expiry misses for the top keys; XFetch-style probabilistic early refresh `[L-V, PAC OP4]` is the stated alternative.
- **Overhead `[A]`:** sketch update per sampled request, O(depth); memory fixed (spec budget <= 16 MB total `[R]`); must be measured (E0).
- **Failure/safety:** key-hash only (no raw keys -> privacy-safer, but reports must be tenant-scoped; one shared sketch across tenants would leak cross-tenant frequency signal, see C7). Hash collisions inflate counts (bounded by CM error). Adversarial hot-key flooding can poison pinning: pin budgets are hard-capped and decay.
- **Strongest baseline:** sampled exact counting over a window; per-key EWMA.
- **Prior art:** Space-Saving, Count-Min, Misra-Gries `[PAC HK1-HK3]`; Redis/Tencent hot-key tooling `[PAC HK4, L-S]`; NetCache/SwitchKV (hardware/KV-cluster) `[L-V, P7]`.
- **Falsification (E4 in repo plan):** recall of true top-10 < 0.9 at the stated memory across skew and shift -> reject detector; lead-time over a threshold-on-count baseline not > 0 -> reject "predictive" claim (keep detection).
- **Differentiation:** low for detection; unknown for "prediction" `[H]`. **Class: Existing capability (library only, unwired) for detection; Prototype candidate for Observe-mode wiring; Research hypothesis for predictive actuation.**

### C4. Early flash-crowd detection

- **Real problem:** a flash crowd on cached keys is absorbed by the cache; a flash crowd on *uncached or newly-invalidated* keys, or a scan/DoS that looks like one, hits the origin. Detecting the former early, and telling the types apart, gates which response is safe.
- **Inputs:** request rate, cold-miss (never-seen-key) rate, distinct-key rate vs request rate (concentration), miss amplification, origin in-flight and latency, client-diversity if available (`[A]`: not currently an observation; Jung et al. found client behavior distinguishes flash events from DoS `[L-V abstract, P1]`).
- **Algorithm:** CUSUM/EWMA on rate and on cold-miss rate (exists for rate, `signals.rs:309-346`; thresholds must become scale-free, e.g. relative to a robust baseline); ADWIN or Page-Hinkley alternatives `[L-V P3; L-S P4]`; a small rule table (rate up + concentration up + hit ratio stable = flash on cached keys; rate up + distinct-key up + miss up = scan/novel crowd).
- **Output/action:** Observe: typed alert with lead time. Later, bounded reversible actions: raise refresh/coalescing priority for the surging keys, extend stale window *within the hard cap* if origin is also degrading (the existing candidate `candidates.rs:97-131` does this by a rule), pre-compute Little's-law cap advice.
- **Integration:** `WorkloadPipeline` (exists), alert surface in the intelligence status endpoint (new fields).
- **Benefit `[H]`:** earlier trigger than reactive thresholds on origin pressure; mostly value as a *classifier gating other actions*.
- **Overhead:** low `[A]`.
- **Failure/safety:** false alarms trigger needless action (all actions reversible and capped); slow baselines adapt to the crowd (baseline-drift); an alert must never relax a hard cap.
- **Strongest baseline:** static threshold on rate or on origin in-flight/miss rate tuned per scenario; origin-latency threshold.
- **Prior art:** Jung et al. `[L-V, P1]`; CUSUM `[L-S, P4]`, ADWIN `[L-V, P3]`; metastability literature on load spikes `[PAC OP9-OP10]`.
- **Falsification:** matched false-alarm rate (<= pre-registered per hour on non-event traces), detection delay not shorter than the threshold baseline with CI excluding zero -> reject. Evaluate on lab families W-FC, W-HK, W-SC `[R: lab]`.
- **Differentiation:** low-moderate `[A]`. **Class: Prototype candidate (Observe mode).**

### C5. Predictive admission / eviction / retention

- **Real problem:** which objects deserve space; one-hit wonders and scans pollute; misses differ in origin cost.
- **Inputs:** key_hash stream (sampled), size, TTL/remaining lifetime, measured origin latency per class, hit/miss.
- **Algorithm ladder:** faithful static policies (SIEVE, S3-FIFO, LRU) -> frequency admission (TinyLFU/W-TinyLFU) -> cost-aware (GDSF/Hyperbolic/LHD) -> shadow-selected policy (miniature simulation) -> learned (LRB/GL-Cache/LAH). Repo gate: non-ML win first (`phantom_intelligence_phase.md` 6).
- **Output:** policy choice among faithful implementations; admission threshold; retention priority.
- **Integration:** `ShadowEvaluator` (exists, `shadow.rs:418`) off-path; actuation requires a policy handle in `core/cache.rs` (the legacy path is `set_eviction_policy`, global and O(n) rebuild `[R: report 11.4]`); WP-34 in the phase plan. Prerequisite: the serving cache must actually use the faithful policies (see inventory; unconfirmed).
- **Benefit `[H]`:** on workloads where the default policy is not near the best fixed policy. Crowded field; published gains are author-reported on large trace corpora `[L-V, PAC EV4, EV5, EV11, EV14]`, none measured on this gateway.
- **Overhead:** shadow policies cost per sampled request; sampling rate trades accuracy `[L-V, P5]`.
- **Failure/safety:** policy flapping; shadow ignores TTL, size, invalidation (known `[R: 11.2]`); policy switch must keep byte accounting I1-I8 and never evict in-flight (CLAUDE.md 5).
- **Strongest baseline:** best fixed faithful policy per workload family (CAND-1's baseline was the product default, not this `[R: 11.4]`), and W-TinyLFU.
- **Prior art:** extensive (see PAC 2.3). Nothing here should be presented as new.
- **Falsification (E5):** selector not better than the best fixed policy within CI on TTL-aware shadow -> stop. Also: shadow-predicted vs realized miss-ratio agreement must hold in the lab before any actuation.
- **Differentiation:** low (crowded; the repo already demoted it `[R: candidate_comparison.md]`). **Class: Research hypothesis (E5)**; Existing capability only as lab/library code.

### C6. Workload-aware TTL / freshness

- **Real problem:** header-less responses get a literal 60 s (unwired config) `[V config.rs:115-118 per CLAUDE.md; R: ttl_decision F1-F3]`; origins with `max-age` dictate freshness. Tuning could cut origin load or staleness exposure.
- **Inputs:** revalidation outcomes (`revalidation_not_modified` counters, `cache.rs:1054`), ETag/Last-Modified change history, invalidation events, per-key hit counts.
- **Algorithm:** estimate per-class change rate (Poisson model `[L-S, P15]`; Cho & Garcia-Molina `[L-V bib, P14]`); adaptive TTL toward a hit-rate/size target `[L-V, PAC FR3]`; heuristic freshness (RFC 9111 10% rule, not implemented, `[R: ttl_decision alt E]`).
- **Output:** **advisory only:** "recommended `max-age` for class X, estimated unnecessary revalidations Y, estimated staleness exposure Z" for the origin owner or an operator-set default.
- **Integration:** counters exist; analysis offline/slow; recommendation shown in advisor. Actuation would touch the frozen `freshness.rs` and the wiring of `PHANTOM_DEFAULT_TTL_MS` (decision pending `[R: ttl_decision]`).
- **Benefit `[H]`:** modest; accountability report rates adaptive TTL "low value now" (CAND-3 null, stale window inert) `[R: 11.4]`.
- **Failure/safety (asymmetric):** lengthening TTL lengthens the stale window after a lost invalidation, which has a documented durability gap (`[R: report 12]`); no gateway ceiling on origin lifetimes exists. Intelligence must never extend beyond origin directives or the hard stale cap, and should be allowed to *shorten* before it may lengthen.
- **Strongest baseline:** static TTL + SWR/SIE; heuristic freshness.
- **Prior art:** Basu et al., Mao et al., Cho/Garcia-Molina, Lee et al. (see above), RL-TTL preprints `[L-S, P16]`.
- **Falsification:** recommendations not reducing origin revalidations at equal measured staleness exposure on lab traces with origin change processes -> drop.
- **Differentiation:** low. **Class: Proposed extension (advisory TTL recommender); actuation = Unsupported for now.**

### C7. Tenant-specific workload intelligence

- **Real problem:** tenants differ (skew, rate, churn, origin); one global setting is wrong for some. Isolation is a security property, not a tuning knob.
- **Inputs:** per-tenant counters, per-tenant sampled key stream, per-tenant hit/miss, per-tenant bytes.
- **Algorithm:** per-tenant detectors (C3, C4, C9); per-tenant miss-ratio curve by spatial sampling `[L-V, P5]` to advise byte quotas; gradient-based reallocation as in Cliffhanger/Memshare `[L-V, P11]`; hierarchical shrinkage toward a global prior for cold tenants (X8).
- **Output:** per-tenant reports; **advisory** quota/profile recommendations. Active reallocation only with an operator-set min/max per tenant.
- **Integration:** `CacheKey.tenant_id` (`cache.rs:24-32`) gives the partition; sketches keyed per tenant. Whether per-tenant byte quotas exist in the cache today was not checked.
- **Benefit `[H]`:** noisy-neighbor protection and better tenant-level hit ratio; unmeasured.
- **Overhead `[A]`:** memory scales with tenants: hard cap on tracked tenants, LRU of tenant states, "other" bucket.
- **Failure/safety:** **cross-tenant leakage:** a shared sketch or shared model lets one tenant's traffic perturb or reveal another's frequencies; per-tenant state, tenant-scoped APIs, and noise/aggregation for any cross-tenant prior. A malicious tenant could game reallocation (quota stealing): min guarantees as in Memshare's private allocation `[L-V, P11]`. Tenant-epoch and fencing untouched.
- **Strongest baseline:** static equal/weighted quotas.
- **Prior art:** Memshare, Cliffhanger `[L-V, P11]`.
- **Falsification:** per-tenant tuning not beating the static quota on worst-tenant hit ratio / origin load with CI excluding zero in a multi-tenant lab family (a family with heterogeneous tenants does not currently exist `[A]`) -> drop.
- **Differentiation:** moderate if isolation guarantees are made explicit `[A]`. **Class: Proposed extension (Observe/advisory); Research hypothesis for reallocation.**

### C8. Origin-pressure forecasting

- **Real problem:** the origin degrades or saturates; knowing seconds ahead allows earlier protective action (cap, stale extension, refresh deferral).
- **Inputs:** origin in-flight, latency percentiles, error rate (`OriginMetrics`), miss-rate series, arrival rate, rejections from the origin gate.
- **Algorithm ladder:** persistence -> EWMA/Holt (exists, `candidates.rs:175-247`, MASE vs persistence) -> queueing-derived estimate (Little's law: concurrency ~ arrival rate x service time `[L-V, PAC CC10]`) -> ARIMA/ETS -> ML. Prediction target: probability that in-flight exceeds a capacity estimate within horizon h.
- **Output:** lead-time risk score (advisory); input to cap sizing and to the C4 classifier.
- **Integration:** `Predictor` trait (`mod.rs:175`); series from `origin_metrics.rs`.
- **Benefit `[H]`:** conditional on forecastable precursors; a step change (outage) is not forecastable from history. The lab result CAND-5 was mixed (-0.42 on one family, ~+0.2 on three) `[R: report 11.4]`.
- **Failure/safety:** false positives cause needless protective action; the forecast must be an input to a clamped actuator, never to a cap relaxation. Also see unbounded vectors (`candidates.rs:179-181`).
- **Strongest baseline:** persistence / EWMA; static thresholds on in-flight.
- **Prior art:** none found for cache-side short-horizon origin-pressure forecasting (coverage gap `[R: PAC 2.6]`); popularity forecasting exists `[L-S]`.
- **Falsification (E6):** MASE >= 1 on held-out scenarios, or no downstream utility gain -> reject. Requires E1 (load-dependent origin) to be meaningful `[R]`.
- **Differentiation:** unknown `[H]`. **Class: Research hypothesis.**

### C9. Workload classification and change/anomaly detection

- **Real problem:** which regime is the workload in (skewed/scan/flash/shift/churn), and did it change? A regime label and a change alarm gate every other component (don't tune during transients; don't trust stale models after a shift).
- **Inputs:** F1-F12 features (`mod.rs:98-113`): rate, hit ratio, Zipf alpha (`signals.rs:204`), working set (`:256`), burst, emergence, miss amplification, origin health, stale ratio.
- **Algorithm:** rule-based classifier on features -> k-means/GMM on features -> supervised classifier trained on lab families (the lab has labeled families W-Z, W-SC, W-FC, W-SH, W-HK `[R]`); change detection by CUSUM/ADWIN `[L-S P4; L-V P3]`.
- **Output:** regime label + confidence + change alarm (advisory/Observe); used as a *gate* by C5 and C13.
- **Integration:** `WorkloadPipeline.produce_window` (`signals.rs:455`) already assembles features; classifier is a pure function over `SignalWindow`.
- **Benefit `[H]`:** moderate; primary value is protection against acting on stale conclusions.
- **Overhead:** negligible per window `[A]`.
- **Failure/safety:** supervised models trained on synthetic lab families may not transfer (distribution shift; `[R: literature_review]`): require OOD fallback to "unknown" -> no action. Labels derived from synthetic traces must carry provenance `Simulated` when produced in the lab.
- **Strongest baseline:** hit-ratio drop threshold; operator-selected profile.
- **Prior art:** CUSUM, ADWIN, CACHEUS workload primitives `[L-V EV6]`.
- **Falsification:** classification accuracy on held-out *families* (not just seeds) below a pre-registered bar, or no improvement of a downstream decision vs the unconditioned decision -> keep only the change alarm.
- **Differentiation:** low. **Class: Prototype candidate** (detectors exist as library).

### C10. Learning from past decision outcomes

- **Real problem:** every advisory/active decision is an experiment; without recording prediction vs realized effect, the system never learns whether it is helpful and cannot demote bad rules.
- **Inputs:** decision records (inputs + provenance, candidate, predicted effect with interval, realized effect), randomized exposure from bucket routing (`route_bucket`, `executor.rs:270`), holdback arm.
- **Algorithm:** (a) *ledger:* calibration of predicted vs realized per rule; auto-demotion when calibration error exceeds the effect claimed. (b) *off-policy evaluation:* inverse-propensity or doubly-robust estimates from logged randomized decisions `[L-V, P8]`. (c) *bandit tuning of parameters inside envelopes.*
- **Output:** rule scorecards; demote/promote proposals (human-approved); updated priors for confidence (C12).
- **Integration:** `AuditStore` exists (`audit.rs:17-33`) but, per the lead's check, nothing appends to it in production `[R: 11.2]`; wiring it is a prerequisite. Randomization requires key-hash bucket routing (exists as a function, unused).
- **Benefit `[H]`:** safety and honesty: prevents silent drift of rules; calibration data feed C12. Direct performance benefit unproven.
- **Failure/safety:** logged decisions without randomization cannot support causal claims; holdback arm cost (1% per design `[R]`); delayed/confounded outcomes (workload changes between decision and outcome).
- **Strongest baseline:** periodic manual review of audit records; A/B on a lab replay.
- **Prior art:** counterfactual learning systems `[L-V, P8]`; Darwin/CACHEUS learn from outcomes `[PAC]`.
- **Falsification:** ledger-calibrated rules not outperforming the same rules without demotion in a drifting-workload lab scenario -> keep ledger as audit only.
- **Differentiation:** low-moderate `[A]`. **Class: Proposed extension (ledger); Research hypothesis (OPE/bandit).**

### C11. Strategy selection (meta-policy)

- **Real problem:** choose among fixed strategies (eviction policy, admission on/off, cap profile) per regime.
- **Inputs:** C9 regime label, shadow results (C5), outcome ledger (C10).
- **Algorithm:** lookup table from regime (heuristic) -> shadow-argmax with hysteresis (statistical) -> bandit over strategies with guard (Darwin-style `[L-V PAC EV11]`; Thompson-sampling eviction preprint `[L-S, P17]`) -> contextual bandit (deferred: no randomized logs `[R: 11.4]`).
- **Output:** a strategy id within a pre-approved menu; never a free parameter outside the envelope.
- **Integration:** executor `apply_policy` (`executor.rs:346`) with watchdog/revert (`:390`).
- **Benefit `[H]`:** equals the upside of C5 plus regime gating; bounded by best-fixed-per-regime.
- **Failure/safety:** flapping, exploration cost; exploration restricted to shadow or holdback-scale exposure.
- **Strongest baseline:** best fixed strategy per workload family (oracle-tuned) `[R: phantom_intelligence_phase 5]`.
- **Prior art:** LeCaR, CACHEUS, Darwin, LAH, SCION `[PAC EV6-EV14]` - crowded.
- **Falsification:** E5-style: selector <= best fixed within CI.
- **Differentiation:** low. **Class: Research hypothesis.**

### C12. Confidence/uncertainty-aware decisions (cross-cutting)

- **Real problem:** the current CI code treats samples as independent (`shadow.rs:452-463`; engine too `[R: 11.2]`), so "significant" is overstated; point estimates drive decisions.
- **Inputs:** any estimate with a time-ordered sample; for canaries, key-bucket as randomization unit.
- **Algorithm:** block/cluster bootstrap by key-hash bucket and time block; paired shadow comparison (same stream, per-sample paired difference); conformal prediction intervals for forecasts `[L-V, P9]`; PBS-style probabilistic staleness bounds as the *form* of a freshness claim `[L-V, P13]`; abstain (NoAction) if the interval straddles the minimum effect; provenance gate (already in design: `Simulated/Unavailable` forces NoAction `[R: phase doc 1]`).
- **Output:** decision = act only if lower bound > delta_min; every record carries the interval and its method.
- **Integration:** replaces the CI code paths in `shadow.rs`/`engine.rs`; method choice recorded in `DecisionRecord.confidence_interval` (`mod.rs:157`).
- **Benefit `[H]`:** reduces false commits; costs power (more NoAction).
- **Failure/safety:** conformal needs exchangeability, violated by non-stationary workloads (use adaptive/online variants; their coverage is not verified here `[A]`); intervals can still be wrong -> hard caps remain the real safety.
- **Strongest baseline:** current independent-proportion CI (known biased); fixed margin thresholds.
- **Prior art:** CQR `[L-V, P9]`; PBS `[L-V, P13]`; miniature simulation error analysis `[L-V, P5]`.
- **Falsification:** on synthetic data with known effect, measured false-commit and false-rollback rates must meet bounds (PI-3 `[R]`); otherwise the method is rejected.
- **Differentiation:** low (method), moderate as engineering discipline. **Class: Proposed extension.**

### C13. Multi-timescale intelligence (architecture principle)

- **Real problem:** one controller mixing second-scale reaction with day-scale learning oscillates (stacked controllers `[R: Charapko cited in phase doc 4]`).
- **Design:** fast loop (sub-second..seconds): detectors and reversible, clamped, short-lived actions (e.g. temporary stale extension within cap); medium loop (minutes): strategy selection with hysteresis and cooldown; slow loop (hours-weeks): seasonality, calibration, recommendations that need human approval. Each loop has its own privilege level and kill switch; loops communicate only through typed, clamped proposals; one writer per actuator.
- **Benefit `[H]`:** avoids interaction failures; makes auditing tractable.
- **Failure/safety:** coordination cost; slower loops may feed stale conclusions to faster ones (guard with change alarms, C9).
- **Strongest baseline:** a single threshold controller (the legacy stub) - the thing to beat is also its failure mode.
- **Falsification:** stacking test: with two actuators enabled together under oscillating workload, flip rate or constraint violation rises vs each alone -> loops must be serialized.
- **Prior art:** classic control; congestion-control literature `[L-V PAC CC4-CC6]`.
- **Class: Proposed extension.**

### Ideas outside the brief's list

#### X1. Post-flush / restart warm-up planner (cold-start recovery)
- **Problem:** the cache is in-memory (restart and "flush on gap" produce a cold cache; `[R: report 12]` notes flush-on-gap is a coarse recovery and that restart loses state). Cold start is a known metastability trigger `[L-V PAC OP9]`.
- **Inputs/algorithm:** a periodically exported (bounded) *hot-set manifest* (key hashes are insufficient; needs keys, so a tenant-scoped, size-bounded key list with a security review) ; at start/flush, refill in priority order at a rate bounded by the origin gate and measured origin health (token bucket tied to origin p99/error); peer-assisted warm-up from a healthy peer's top-k list in cluster mode **(cluster part is gated on the M3 invalidation-durability fix; see section 11, item 3)**.
- **Integration:** boot path in `main.rs`, cluster sessions (`src/dist/`); new code.
- **Benefit `[H]`:** shortens the post-restart miss storm; same machinery as C1 (shared scheduler).
- **Safety:** warm fetches obey freshness and fencing; the manifest is data at rest: key names can be sensitive -> opt-in, tenant-scoped, encrypted/ACL'd storage (design item). Must not resurrect invalidated keys: warming fetches fresh from origin, never from a stale snapshot of values.
- **Baseline:** do nothing (lazy fill with single-flight); static ramp-up limit.
- **Prior art:** metastable-failure literature on cold caches `[L-V PAC OP9, OP10]`; warm-up tools `[L-S P10]`; no cache-gateway-specific scheduler search done.
- **Falsification:** lab restart scenario: time-to-90%-of-steady-hit-ratio and origin peak vs lazy fill; kill if origin peak is not lower or if goodput during warm-up falls.
- **Differentiation:** moderate `[A]`. **Class: Prototype candidate** (heuristic; shares C1 scheduler).

#### X2. Metastability early-warning (retry/miss amplification detector)
- **Problem:** positive-feedback loops (slow origin -> timeouts -> retries/misses -> slower origin) are the failure the repo's research is motivated by `[R: candidate_comparison I; L-V PAC OP9-OP10]`.
- **Inputs:** miss rate, origin in-flight, origin latency slope, client retry indicators (duplicate requests per key within short window), error rate.
- **Algorithm:** detect the *signature*: in-flight rising while goodput flat/falling and latency rising (the V-E1-2 collapse signature in E1 `[R: e2 draft 4.3]`); simple state machine with hysteresis; Little's-law consistency check (L = lambda x W `[L-V PAC CC10]`).
- **Output:** early-warning alert; recommended (not auto) reduction of cap or enabling bounded wait queue.
- **Integration:** `OriginMetrics` + origin gate snapshot (`executor.rs:65-73`).
- **Benefit `[H]`:** operator lead time; conditional on a load-dependent plant (E1) for evaluation, which the repo lacks `[R]`.
- **Failure:** false alarms; the signature differs per origin. **Safety:** advisory.
- **Baseline:** alert on origin p99 or error-rate threshold.
- **Falsification:** on E1 plant, detection precedes goodput collapse by > baseline lead time at equal false-alarm rate -> else drop.
- **Differentiation:** unknown. **Class: Research hypothesis.**

#### X3. "Phantom Advisor" what-if replay (offline counterfactual from sampled logs)
- **Problem:** operators cannot try cap/TTL/policy changes in production; the repo has a deterministic lab but the gateway writes no request log `[R: 11.4]`.
- **Inputs:** an opt-in, bounded, sampled, tenant-scoped trace (key hash, size, timing, hit/miss, origin latency, freshness lifetime), spatially sampled `[L-V, P5]`.
- **Algorithm:** replay the sampled trace through the lab simulator / miniature cache with candidate configs; report predicted effect with interval; Little's-law cap sizing curve (predicted shed vs peak) `[R: report 11.5]`.
- **Output:** "if you set cap=X / policy=Y / SWR=Z, expect ..." with interval and a calibration score from past predictions (C10).
- **Integration:** trace sampler (new, off-path, opt-in) + lab (exists, offline). No serving-path actuation.
- **Benefit `[H]`:** value even if no controller ever beats a static setting: it helps *choose* the static setting. Aligned with the repo's recommended direction `[R]`.
- **Safety:** trace contains key names -> privacy (hash + tenant-scoped, retention limit); the simulator's fidelity is unvalidated (T1 vs T2 never validated `[R: C7]`), so predictions must be labeled `Simulated` until E7 calibration.
- **Baseline:** operator rule of thumb; Little's-law arithmetic alone.
- **Prior art:** SHARDS / miniature simulation `[L-V, P5]`, libCacheSim `[L-R]`.
- **Falsification (E7):** calibration error larger than the recommended effect -> do not ship predictions.
- **Differentiation:** moderate (packaging: lab + provenance + calibration) `[A]`. **Class: Prototype candidate.**

#### X4. Origin change-rate learning -> `Cache-Control` recommendation report for origin owners
- **Problem:** many inefficiencies are fixed at the origin (missing/overly short `max-age`), not the cache.
- **Inputs:** 304 vs 200 revalidation ratio per path class, ETag change interval, invalidation counts.
- **Algorithm:** Poisson change-rate estimate per class `[L-V bib P14; L-S P15]`; recommend `max-age`/SWR given a staleness-risk target; report only.
- **Integration:** counters exist (`cache.rs:1054`); report endpoint new. No actuation, no freshness change.
- **Benefit `[H]`:** unlocks gains safely by human action; zero serving-path risk.
- **Baseline:** none or heuristic freshness rule.
- **Falsification:** recommendations when applied in a lab do not reduce origin revalidation at equal staleness -> drop.
- **Differentiation:** low. **Class: Proposed extension.**

#### X5. Cache-key fragmentation detector (key hygiene)
- **Problem:** the same content under many keys (tracking query params, parameter order, session ids) collapses hit ratio and inflates working set. Often the largest real-world loss, and fixable by normalization rules.
- **Inputs:** canonical key strings (path + query) with response body hash/ETag per key (the `ETag` is already a validator).
- **Algorithm:** group keys by path; detect query parameters whose variation does not change body hash/ETag (sampled), via sampled equality tests; report candidate ignore-lists. Heuristic/statistical, no ML.
- **Output:** advisory "parameter `utm_*` appears irrelevant for 97% of sampled keys (n=...)", never auto-normalizes (changing keys changes correctness semantics).
- **Integration:** off-path analysis of sampled responses; key canonicalization is a correctness-critical area (tenant separator invariant, `cache.rs:42-46`) so any actuation is out of scope.
- **Benefit `[H]`:** potentially large hit-ratio gain per deployment, entirely dependent on traffic.
- **Safety:** wrongly merging keys would serve wrong content: advisory only, human applies config.
- **Baseline:** operator knowledge; origin `Vary` and CDN query-string rules.
- **Prior art:** not searched (CDN "query string ignore" features are documented by vendors `[L-R, unverified]`). Coverage gap.
- **Falsification:** detector precision on lab-generated fragmentation < pre-registered bar (false "irrelevant" calls are dangerous).
- **Differentiation:** unknown. **Class: Research hypothesis / Prototype candidate (advisory).**

#### X6. Staleness-risk estimator (probabilistic freshness reporting)
- **Problem:** the repo cannot state a staleness guarantee beyond TTL and has a documented invalidation durability gap `[R: report 12]`. Operators need an honest, measured estimate.
- **Inputs:** invalidation delivery latency histogram, anti-entropy repair intervals, peer state, TTL distribution of resident entries.
- **Algorithm:** PBS-style t-visibility estimates `[L-V, P13]` from measured propagation latencies: "probability a read after a DELETE returns stale data within t ms", reported with failure model and bound (CLAUDE.md 4 rule).
- **Output:** measured dashboard field labelled `measured/derived`; never a guarantee.
- **Integration:** dist layer metrics (`src/dist/router.rs`); new code.
- **Benefit `[H]`:** trust, compliance conversations; no actuation risk.
- **Baseline:** state TTL only.
- **Falsification:** estimator vs injected-loss lab ground truth outside interval -> not shippable.
- **Differentiation:** moderate (assurance workstream F `[R: candidate_comparison]`). **Class: Proposed extension.**

#### X7. Hierarchical heavy-hitters over path prefixes and tenants
- **Problem:** hot *groups* (a product category prefix, a tenant) matter for origin fan-in; per-key heavy hitters miss them.
- **Algorithm:** hierarchical heavy hitters over (tenant, path-prefix) levels using the same sketch family `[L-V HK1-HK2]`; HHH literature not searched.
- **Output:** advisory hot-prefix list; basis for prefix-level calendar entries (C1) and coarse refresh priority.
- **Overhead/safety:** extra sketch per level, bounded; tenant-scoped. **Baseline:** per-key top-k plus operator knowledge. **Falsification:** recall/precision of hot prefixes vs exact at fixed memory.
- **Prior art:** gap. **Differentiation:** low. **Class: Prototype candidate.**

#### X8. Empirical-Bayes shrinkage for cold tenants and cold keys
- **Problem:** per-tenant/per-class estimates (C7, C6, C8) are noisy at low volume; naive per-tenant tuning overfits.
- **Algorithm:** hierarchical (partial pooling) estimates: tenant estimate shrinks to a global prior with weight set by sample size; priors updated offline. Pure statistics.
- **Safety:** pooling across tenants shares information: use only aggregate hyper-parameters, never per-key data; evaluate leakage explicitly.
- **Baseline:** global-only or per-tenant-only. **Falsification:** shrinkage estimator error not lower than both baselines on held-out tenants.
- **Prior art:** standard statistics; cache application not searched. **Differentiation:** low. **Class: Research hypothesis.**

#### X9. Decision-budget governor (intelligence that polices its own cost)
- **Problem:** intelligence has CPU/memory cost and risk; there should be a per-feature budget and an automatic off-switch when measured overhead or regret exceeds it.
- **Algorithm:** meters for intelligence CPU time, ring-buffer drops (`overrun_drops` in spec `[R: intelligence_architecture 3]`), memory, and C10 calibration; thresholds -> drop mode from Active to Advisory to Off (the design already has mode downgrade and watchdog `[R]`, `executor.rs:285`).
- **Baseline:** manual config. **Falsification:** governor false-disable rate on healthy workloads above the bound.
- **Prior art:** not searched. **Differentiation:** low. **Class: Proposed extension.**

## 4. Ranked table

Scores 1-5 (5 best), my judgment `[A]`, not measured. *Feasibility* = near-term implementability given existing code and approvals; *Differentiation* = plausibility that the result would distinguish PhantomCache, **not** a novelty claim; *Measurable impact* = can the benefit be falsified/measured with the existing lab; *Research potential* = value of the knowledge produced, including negative results. Sum is unweighted and only for ordering.

| Rank | Candidate | Eng. value | Feasibility | Differentiation | Measurable impact | Research potential | Sum | Class |
|---|---|---|---|---|---|---|---|---|
| 1 | C3 (detection part) + C9 Observe-mode: truthful hot keys, regime/change alarms | 5 | 5 | 2 | 4 | 3 | 19 | Existing (library) / Prototype |
| 2 | C1 Demand Calendar + predictive preparation (deterministic core) | 5 | 3 | 4 | 4 | 3 | 19 | Prototype candidate |
| 3 | X3 Phantom Advisor what-if + Little's-law cap sizing | 4 | 4 | 3 | 4 | 4 | 19 | Prototype candidate |
| 4 | C12 uncertainty-aware decisions + C10 outcome ledger (cross-cutting) | 4 | 4 | 2 | 4 | 4 | 18 | Proposed extension |
| 5 | C4 early flash-crowd detection | 4 | 4 | 3 | 4 | 3 | 18 | Prototype candidate |
| 6 | X1 post-flush/restart warm-up planner (+X2 metastability early warning as Observe) | 4 | 3 | 3 | 4 | 4 | 18 | Prototype / Research hypothesis |
| 7 | C3 (actuation) hot-key refresh-ahead within background-slot budget | 3 | 3 | 2 | 4 | 3 | 15 | Research hypothesis |
| 8 | C5/C11 shadow-selected eviction/admission & strategy selection (E5) | 3 | 3 | 1 | 5 | 3 | 15 | Research hypothesis |
| 9 | X6 staleness-risk estimator | 4 | 3 | 3 | 3 | 3 | 16 | Proposed extension |
| 10 | C7 tenant-specific intelligence (Observe/advisory) | 3 | 3 | 3 | 3 | 3 | 15 | Proposed extension |
| 11 | X4 Cache-Control recommendation report; C6 advisory TTL | 3 | 4 | 1 | 3 | 2 | 13 | Proposed extension |
| 12 | X5 key-fragmentation detector | 4 | 3 | 2 | 3 | 3 | 15 | Research hypothesis |
| 13 | C8 origin-pressure forecasting | 3 | 3 | 3 | 3 | 3 | 15 | Research hypothesis |
| 14 | C2 seasonality prediction | 3 | 2 | 1 | 2 (no real traces) | 3 | 11 | Research hypothesis |
| 15 | C13 multi-timescale (design principle, not separate deliverable) | 4 | 5 | 2 | 2 | 3 | 16 | Proposed extension |

Notes: ranks 7-15 are ordered by judgment, not strictly by the sum (the sum is a tie-breaker). C13 and C12 are principles that should shape everything else rather than compete for a slot. Rank 1-3 tie on sum; ordering reflects that rank 1 is the prerequisite (nothing downstream is trustworthy without truthful observations) and costs least.

## 5. Heuristics vs statistics vs online learning vs ML vs hybrid: summary of where each earns its place

| Technique class | Where it fits here | Why / why not |
|---|---|---|
| Declared/operator information | C1, X4, X5 | Zero prediction error for known events; the cheapest way to beat reactive; risk is operator error (bounded by envelope) |
| Heuristics / rule tables | C4, C9, X2, X7 | Cheap, auditable, deterministic; thresholds must be scale-free and tuned in the lab |
| Statistical estimation/forecasting (EWMA, CUSUM, sketches, Poisson rates, seasonal-naive/ETS) | C2, C3, C6, C8, X3, X6 | Falsifiable offline; no training pipeline on the serving path |
| Online learning / bandits | C11, C10(c) | Only inside a pre-approved menu, exploration limited to shadow/holdback; needs randomized logs (absent today) |
| ML (GBM/NN) | later C5, C2 | Repo gate (5-part) applies; crowded literature; no real traces; defer |
| Hybrid (static safeguards + learned recommendations) | whole architecture | Repo's recommended direction `[R: 11.5]`; keeps learned parts advisory until evidence |

## 6. Proposed intelligence architecture (hybrid, progressively activated)

```
 Operator ---(declared: calendar, profiles, caps, envelopes)---------------+
                                                                           v
 +--------------------------------------------------------------------------------------+
 | L0  DETERMINISTIC GATEWAY (execution layer; authoritative, always on)                |
 |   serving cache + single-flight + freshness + fenced invalidation + tenant epochs    |
 |   HARD CAPS: stale cap, origin cap, byte/entry budgets I1-I8, in-flight never evicted |
 |   executor: validates every proposal (clamp to envelope), atomic apply, safe revert,  |
 |   watchdog, per-actuator kill switch                                                 |
 +--------------------^--------------------------------------------+--------------------+
        sampled, bounded|events (drop-newest ring)                  | typed, clamped
        key_hash,tenant,|hit/miss,latency,origin health             | proposals only
                        v                                            |
 L1 OBSERVE (off-path, Mode=Observe): feature pipeline F1-F12, hot keys, regime, change
    alarms, origin health; every field carries provenance; unavailable => NoAction
                        v
 L2 ADVISE (Mode=Advisory): forecasts with intervals, calendar suggestions, cap sizing,
    TTL/Cache-Control recommendations, key-fragmentation hints, what-if answers;
    recommendations logged in the audit store; humans decide
                        v
 L3 SHADOW: miniature simulations of candidate policies on a hash-sampled stream;
    paired, block-bootstrapped comparison; no exposure risk
                        v
 L4 PROGRESSIVE ACTIVATION (Mode=Canary then Active), per actuator, only after its evidence
    gate: key-bucket randomized exposure + permanent holdback, cooldown/hysteresis,
    guardrails (staleness cap, origin load, p99), auto-revert, decision record

 OFFLINE (not on the serving path): Phantom Lab replay and what-if, pre-registered
    evaluation, model training/calibration with versioned artifacts, seasonality fitting,
    ledger analysis (predicted vs realized), threshold derivation for detectors
```

**Online vs offline split.**

| Online (in process, bounded) | Offline / slow (lab, batch, human-reviewed) |
|---|---|
| sketches, rate/CUSUM, regime features; shadow policies on sampled stream; Little's-law arithmetic; calendar scheduler; warm-up scheduler; decision records | seasonality fitting; model training; detector threshold derivation; what-if replay; calibration of predictions; pre-registered A/B evaluation; literature/novelty checks |

**Actuator privilege tiers (proposal `[A]`).**

| Tier | Examples | Condition |
|---|---|---|
| Never intelligence-controlled | hard caps themselves, tenant isolation, fencing/epochs, freshness semantics beyond origin directives, key canonicalization, byte accounting | by construction |
| Advisory forever | TTL changes, tenant quota changes (initially), any ML-derived choice, key normalization, Cache-Control recommendations | humans apply |
| Eligible for L4 after evidence | pre-warm fills within budget (C1/X1), hot-key refresh-ahead within background slots, eviction policy among faithful implementations, cap value within operator [min,max], temporary stale extension within cap | pre-registered result, CI > 0, no adverse scenario, T2 confirmation, overhead budget, deterministic replay (thresholds from `[R: report 11.6]`) |

**Interface rules.** (1) Intelligence cannot construct cache entries; pre-warm = a request through the normal fill path so fencing applies. (2) Calendar entries and any learned state are bounded in count and bytes, expire, and are tenant-scoped. (3) Intelligence state never shared across tenants except aggregate hyper-parameters (X8). (4) Every displayed intelligence value is labeled measured, derived, simulated, inferred or unavailable (CLAUDE.md 4). (5) Off = identical to the static path (spec: single atomic mode check `[R: intelligence_architecture 5]`).

**Minimum viable sequence `[H]`** (needs owner approval at each application-code step): E0 wire L1 in Observe mode and measure overhead -> truthful hot keys (C3/C9) -> E2 headroom test (already drafted `[R]`) -> advisor outputs (X3, X4, X6) -> deterministic calendar prototype in the lab first (C1) -> only then any L4 actuation by evidence.

## 7. Cross-cutting open technical issues found in existing code (for whoever implements)

1. Non-paired, independence-assuming CI in `shadow.rs:452-463`; hit-ratio-only objective; ignores TTL/size/invalidation `[V + R]`.
2. CUSUM and emergence constants are absolute and hard-coded (`signals.rs:404`, `:363`) `[V]`.
3. Unbounded vectors in `PredictiveRateForecaster` (`candidates.rs:179-181`) `[V]`.
4. AIMD candidate uses an absolute p99 ceiling and a hard-coded 0.05 error trigger `[V candidates.rs:156]`; consistent with the reported failure analysis `[R]`.
5. `pie_audit.md` statement that `key_hash` is always 0 is outdated at the call sites seen (`server.rs:1261`) `[V]`; re-verify before relying on either.
6. CLAUDE.md sections 5 and 6 disagree on whether faithful eviction is in the serving path; grep suggests only the lab uses `core::eviction` `[V partial]`.
7. Audit store created but production appends not re-verified here `[R]`.

## 8. Ideas NOT worth pursuing now (with reasons)

| Idea | Reason |
|---|---|
| Learned per-request eviction in the gateway (LRB / GL-Cache style) | Needs training pipeline and traces; crowded and strong baselines (S3-FIFO, SIEVE, LAH) `[L-V PAC]`; repo ML gate requires a non-ML win first; no real traces |
| Deep-learning popularity forecasting (LSTM etc.) | Needs large real traces (none); literature suggests weak accuracy-to-performance link `[L-S, PAC WF4]`; seasonal-naive/ETS first |
| RL / learned origin-cap controller | The simplest controller already failed under a bad objective; unresolved plant validity (E1); high risk/cost `[R: candidate_comparison H]` |
| Absolute-ceiling AIMD as a promoted feature | Negative result; behaves as threshold switch `[R]`, `[V candidates.rs:155-166]` |
| Markov/n-gram next-key prefetch | No sequential workload, adds origin load, no prefetch actuator `[R: 11.4]` |
| Predicting origin changes to skip invalidation | Would weaken the invalidation guarantee for no measured gain; contradicts fencing invariants |
| Intelligence-controlled extension of freshness lifetimes | Enlarges stale window after lost invalidation (documented durability gap `[R: report 12]`); no ceiling on origin lifetimes exists |
| Auto-normalization of cache keys | Wrongly merging keys serves wrong content; key canonicalization is correctness/security critical |
| Auto-tuning of hard caps (stale cap, origin cap, memory) | Hard caps must be operator-set and independent of intelligence |
| Shared cross-tenant sketches/models over per-key data | Leakage/side channel and gaming; use aggregate hyper-parameters only |
| Hedged requests to origin (tail-at-scale) | Duplicates origin load, the opposite of the protection goal; fits multi-replica origins, not this gateway's constraint `[L-S, P12]` (reasoning, `[A]`) |
| LLM in the decision path | Non-deterministic, unauditable, no replay equality (PI-6 requires identical records from same seed); acceptable only as an offline explanation writer, if at all `[A]` |
| Claiming "AI-powered" or novelty for any candidate | Prior-art sweep (RT-0) is open; repo ledger forbids it `[R]` |
| Per-key adaptive TTL actuation, dynamic stale window by itself | Stale window was inert in sealed runs (PIE-no-stale identical in 1200/1200 cells) `[R: m6_sealed_results]` |

## 9. Open questions for the owner

1. **Real traces / design partner:** is any real multi-week request trace (even anonymized key hashes + sizes + timestamps) obtainable? C2, C8, C9 and X5 cannot be validated without it; synthetic traces test only their own assumptions.
2. **Calendar semantics (C1):** who declares events (operator API, config file, origin hint header, signed webhook), at what granularity (exact keys, prefixes, tenant-wide), and what is the acceptable worst-case wasted origin load per entry?
3. **Is C1 considered "intelligence" for the product narrative,** or a deterministic feature that intelligence calibrates? (Affects positioning and the ML-gate applicability.)
4. **Warm-up manifest (X1) and trace sampling (X3):** is storing key names (even tenant-scoped, opt-in) acceptable from a privacy/security standpoint, or must everything stay hashed?
5. **Measurement tier:** is a T2 (loopback/WSL2) confirmation sufficient for promotion, or is a physical-host run required before any serving-path actuation?
6. **Scope of E1/E2 (load-dependent origin; headroom sweep):** approve them as the gate for any cap-related intelligence (C8, X2, adaptive caps)?
7. **Priority between Observe-mode wiring (E0) and ranking items 2-3:** E0 requires application-code approval (CLAUDE.md 2). Which comes first?
8. **Eviction in serving path:** should the faithful policies replace the internal enum policies before any C5 work (CLAUDE.md sections 5 and 6 conflict)?
9. **Tenant model:** do per-tenant byte quotas exist or are planned? C7 depends on it.
10. **Prior-art budget:** should a dedicated RT-0 sweep (proceedings 2022-2026: demand-aware prewarming, flash-crowd detection in caches, per-tenant cache tuning, cold-start recovery) precede any positioning claim about C1, X1, C4, C7?

## 10. Provenance of this document

Read in full: `CLAUDE.md` (via system context), `phantom_intelligence_phase.md`, `intelligence_prior_art_coverage.md`, `pie_audit.md`, `intelligence_architecture.md`, `m6_sealed_results.md`, `intelligence_e2_headroom_protocol_DRAFT.md`, `candidate_comparison.md`, `phantomcache_milestone_accountability_report.md` sections 11 and 12 (partly), `src/intelligence/mod.rs`, `candidates.rs` (1-300), `signals.rs` (parts), `shadow.rs` (400-484), `executor.rs` (20-90), `intel/workload.rs`; function inventories (grep) for `src/intelligence/*` and `src/intel/*`; targeted greps in `src/api/server.rs`, `src/main.rs`, `src/core/cache.rs`. First 40 lines of `ttl_decision/recommendation.md`. **Not read:** `hypothesis_and_experiments.md`, `literature_review.md` (relied on `[R]` quotes in other documents), `engine.rs`, `audit.rs` bodies, `core/eviction.rs`, `freshness.rs`, `dist/*`. Web: 16 standard-mode searches, no full texts, no WebFetch. File written: this one only.

## 11. Cross-cutting discoveries

Added after the owner clarification that scope does not limit investigation. Inputs: `v3_topology_tenancy_identity.md` (sections 1.1-1.4), `v4_security_and_failure_scenarios.md` (sections 0 and 5), `v5_demo_center_ux_and_storyboard.md` (headings only). Their code claims are `[R: v3/v4/v5]` here unless stated. Four extra WebSearch queries (standard mode, no full texts): Varnish backends/directors/grace docs; Caffeine W-TinyLFU adaptive window; Cloudflare Tiered Cache / Cache Reserve; Redis client tracking. Total queries for this file: 20.

### 11.1 Intelligence depends on controls that do not exist yet

1. **No per-tenant quotas and no tenant-attributed telemetry** `[R: v3 1.1.5, 1.1.7; v4 section 0 item 2]`. C7 (tenant reallocation) is blocked, not merely unproven. C1's per-entry byte budget and tenant-scoped pre-warm have nothing to enforce them, so a pre-warm counter and a per-tenant fill budget must be built first as plain deterministic limits. Namespace isolation exists; resource isolation does not.
2. **Dev mode trusts a client-supplied `x-tenant-id`** `[R: v3 1.1.2; v4 section 0 item 3]`. A Demand Calendar or pre-warm API in that mode is an unauthenticated fill-amplification endpoint. Requirement: calendar, pre-warm and all intelligence write paths disabled unless secure mode, scoped to the verified tenant, role-gated (operator/admin, never observer).
3. **M3 durability defect** (204'd invalidations lost after issuer kill) `[R: v4 section 0 item 5; accountability report 12]`. Cluster-wide warming moves content to peers before an invalidation may reach them (v4 5.2). X1's peer-assisted warm-up and any cross-node refresh-ahead are therefore blocked on durable invalidation; local-node warm-up is unaffected.
4. **The gateway forwards no client identity by default** `[R: v4 5.2, "Authorization-bound warming"]`. Pre-warm may fetch only what an anonymous request could fetch, must skip `BYPASS-AUTH` keys and honor uncacheable responses. This narrows C1 to public/shared content (the realistic launch-day case) and must be stated in any pitch.
5. **One origin per gateway; static cluster membership** `[R: v3 1.2, 1.3]`. Origin-pressure candidates (C8, X2) are single-origin by construction; multi-origin would multiply every origin-health feature by origin id.
6. **No per-request event stream** `[R: v5 section 4.1; v3 5.1]`. The L1 observation ring buffer (`intelligence_architecture.md` section 3: bounded, drop-newest) and the Demo Center event stream are the same engineering problem. Build one bounded event tap with tiered consumers (aggregate-only for observers, tenant-scoped for tenants, raw only for operators/lab) instead of two.

### 11.2 How intelligence, security, multi-tenancy and distribution reinforce each other (and conflict)

- **Same detectors, two uses (reinforce).** Rate/CUSUM (C4), unique-key rate, prefix heavy hitters (X7) and the key-fragmentation detector (X5) are the signals v4 5.1 lists for abuse detection (cache-busting, bypass-header rate, DELETE rate, origin-fetch share by principal). Cache-busting by random query parameters resembles X5's "irrelevant parameter" signature plus a high cold-key rate, so one detector family can alert on both inefficiency and abuse. Output stays observe-only; enforcement stays deterministic limits (v4 5.1 item 2), which agrees with the tier table in section 6. `[H]`
- **Deterministic isolation enables safe intelligence (reinforce).** Per-tenant quotas, permit accounting and epochs are what make "intelligence proposes, executor disposes" enforceable per tenant. Ordering consequence: quotas before C7.
- **Shared learning is a side channel (conflict).** v4 5.2 lists forecast side channel, warm-state oracle, audit leakage and poisoned signals. `/api/v1/intelligence/audit` and `/status` (routes at `src/api/server.rs:213-214` `[V]`) would leak cross-tenant load if fed global aggregates; observer scope must be per tenant or aggregate-only before real audit records flow. Hot-key reports are tenant-scoped; any global top-k is operator-only.
- **Tenant-epoch exhaustion is a cross-tenant blast radius** (`MAX_TENANT_EPOCHS = 10,000` falls back to a global epoch bump `[R: v3 1.1.6]`). A per-tenant invalidation-churn feature (F11 is in the spec, `signals.rs` header `[V]`) would give early warning. This is a security/availability use of the observation layer I had not listed. `[H]`
- **Distribution multiplies origin load (derived finding).** The origin gate is a process-wide gate per gateway (`src/intelligence/executor.rs:75-78`, "Process-wide origin dispatch gate") `[V]`, and GET is served from the local cache only `[R: CLAUDE.md 6]`. With N gateways, effective origin concurrency can reach N x cap and N nodes can each fetch the same key. Consequences: (a) the Little's-law cap advisor (X3) must be cluster-aware (per-node cap = target / N, or a shared budget); (b) there is no cross-node single-flight, so adding nodes can increase per-key origin fan-in. A **peer-fill / owner-fetch path** (on miss, ask the key's rendezvous owner, which exists for invalidation routing `[R: v3 1.3]`, before going to origin) is an architecture candidate that interacts with intelligence. `[H]`; large change; needs owner approval; correctness risk with fencing across nodes; not evaluated. Tiered caching to cut origin traffic is established CDN practice `[L-V: Cloudflare cache documentation via search; Origin Shield and collapsing details not retrieved]`, so this would be catching up, not novel.

### 11.3 Competitive context (verified sources) and a challenge to the "intelligence" premise

| Incumbent | Fact from a source returned by search (pages not read in depth) | Implication |
|---|---|---|
| Varnish | Backend health probes (guide example: poll every 5 s, healthy if 3 of last 5 succeed), directors group and fall back across backends, grace mode serves expired objects when backends are down (varnish.org users guide, vcl-backends) `[L-V]`; waiting-list coalescing `[PAC OP1]` | Origin-health-aware stale serving is table stakes under operator VCL control. "Uses origin health" cannot be the differentiator; at most the measured, audited, cap-bounded form. Directors (multi-backend) are absent here |
| Caffeine / W-TinyLFU | Window-vs-main split adapted online by hill climbing (github.com/ben-manes/caffeine/wiki/Efficiency and maintainer comments) `[L-V]`; `refreshAfterWrite` not retrieved `[PAC OP12, class C]` | Online parameter adaptation already ships in a mainstream cache; "adaptive eviction" is not a differentiator (matches my low C5/C11 scores) |
| Cloudflare edge | Tiered Cache reduces origin traffic via multiple cache locations; Cache Reserve is persistent storage on R2, recommended with Tiered Cache; purge covers edge and Reserve in one request (developers.cloudflare.com/cache, blog) `[L-V]`; the "Prefetch URLs" feature appears visitor-side `[L-S]` | Persistence and tiering are real operator value PhantomCache lacks (in-memory only; restart loses state `[R: report 12]`) |
| Redis | Server-assisted client-side caching: tracking mode sends invalidations for keys a client read, BCAST by prefix, bounded tracking table (redis.io docs) `[L-V]`; default size figure from a secondary guide only | Server-pushed invalidation with bounded state is a known pattern; PhantomCache's peer invalidation and fencing are in the same family, not novel |
| Memcached at Facebook | leases, stale-set prevention `[PAC OP5]` | generation fencing is a re-derivation `[R: candidate_comparison F]` |
| Nginx, ATS, Squid, Fastly | `[PAC OP2, OP3, OP12]` | no new verified comparison |

**Challenge to my own premise.** (1) Operators of Varnish and CDNs buy predictability and control; "intelligence" is unlikely to be the adoption reason, and the only adaptive-control experiment so far is negative `[R]`. (2) What may be hard to get elsewhere (hypothesis; absence in incumbents NOT verified by any systematic search): a deterministic replayable lab for rehearsing origin failure and config changes before production; provenance labels (measured/derived/simulated/unavailable) on every displayed number; generation-fenced tenant-epoch invalidation with a stated failure model; a hard-cap envelope bounding any automation. This points the intelligence value proposition at explainable, bounded, rehearsable advice (X3, C12, X6), not autonomous adaptation. (3) For "useful beyond a demo" the blockers are not intelligence: durable invalidation (M3), per-tenant quotas, multi-origin routing, dynamic membership, persistence and restart behavior, per-request observability, and a truthful config surface (`PHANTOM_DEFAULT_TTL_MS` is documented but unwired). Intelligence ahead of these polishes a prototype.

### 11.4 New architectures and use cases (all `[H]`, unevaluated)

1. **Cache-config CI / game-day mode:** replay a recorded or scripted origin behavior through the real gateway code in the deterministic lab as a regression test ("does this TTL/SWR/cap config survive a 30 s brownout?"). Reuses the lab and X3; gives developers a reason to use the product without production traffic. Prototype candidate.
2. **Declared demand as a first-class API** (C1) doubles as the demo hero (v5 "Launch Day") and a real operator feature: declared vs predicted vs actual with uncertainty (v5 3.6). Lab runs must be labeled simulated or measured per tier.
3. **Cache-busting/abuse early warning** from the shared detector family.
4. **Cluster-aware origin budgeting**, with intelligence advising the split.
5. **Owner-fetch / peer-fill** as the main distributed-coordination question; decide before building cluster-aware intelligence because it changes what signals and caps mean.

### 11.5 Corrections to earlier sections after reading V3-V5

- C1/C7: tenant budgets have no enforcement today (11.1 item 1): prerequisites, not features.
- X1: cluster warm-up blocked on M3 (marked inline).
- Section 6 architecture: add an authz-scope rule: every L1-L4 interface is tenant-scoped, write paths secure-mode-only and role-gated, observer views aggregate-only.
- Pre-warm needs a synthetic internal fill request carrying a tenant identity; whether the current handler structure allows that without a request context was not checked `[A]`.

### 11.6 What other workstreams should know

- V3: quotas, multi-origin and dynamic membership are prerequisites for tenant-level and distributed intelligence; the origin gate is per process, so N nodes mean up to N x cap; peer-fill is the open architecture question.
- V4: the same detector family serves abuse detection; intelligence status/audit endpoints leak cross-tenant aggregates unless scoped; calendar and pre-warm must be secure-mode-only and role-gated.
- V5: one bounded event tap should feed both the Demo Center stream and the observation layer (tiered privacy); Launch Day maps onto C1, C4, X2 and X6 with honest labels.
- V2: no real traces exist, so seasonality and origin-pressure forecasting cannot be validated on synthetic data; MASE vs persistence/seasonal-naive and paired block-bootstrap are the proposed yardsticks.
