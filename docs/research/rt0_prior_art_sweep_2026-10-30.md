# RT-0 prior-art coverage record (sweep of 2026-10-30)

**Status: PARTIAL. RT-0 is still OPEN after this sweep. This file makes NO novelty claim, now or conditionally.**
**Date:** 2026-10-30. **Author:** workstream P2 (workload intelligence), three-pillar package. **Files changed:** this file only (new). No source, config, frozen record or earlier prior-art document was edited.
**Relation to earlier records:** `intelligence_prior_art_coverage.md` (2026-10-30, 40 queries, bibliographic check), `vision/v1_intelligence_opportunity_map.md` section 2 and 11.3 (20 queries), and `novelty_snapshot_m6.md`. The last one says "RT-0 PASS"; its own correction notice and `STATUS.md` say that is not supported. This sweep neither confirms nor refutes the snapshot's matrix. Its venue list and patent reading were not re-done here.

## 0. How to read the evidence classes

| Class | Meaning in this file |
|---|---|
| **OPENED-DIRECT** | I retrieved the primary document and read its text myself (local text extraction of the PDF, then reading the relevant parts). Strongest class available here. |
| **OPENED-VIA-TOOL** | The primary page was retrieved with the WebFetch tool. The tool passes the page through a small summarizer model and returns the summary. I did not read the page verbatim. Treat as "primary page exists and says roughly this", not as a quotation. Short quotes below are tool-reported. |
| **SNIPPET** | A primary URL appeared in search results. The page was not opened. Only the search tool's summary is available. Counts as secondary. |
| **SECONDARY** | Blog, vendor marketing, course page, mirror, forum, third-party summary. |
| **NOT FOUND / NOT SEARCHED** | Stated explicitly. |

"Verified-primary" below means OPENED-DIRECT or OPENED-VIA-TOOL on a primary source (paper, standard, vendor documentation page, project README). It never means that I checked the paper's claims. Performance numbers in papers are the authors' own and were not reproduced.

Tool limits met: WebSearch is US-only and indexes blogs better than proceedings. Three WebFetch attempts on Varnish docs failed (two redirects, one empty summary); the fourth succeeded. The Netflix Tech Blog returned HTTP 403 (not opened). The binary PDFs of Facebook NSDI'13, S3-FIFO and XFetch were retrieved and converted with `pdftotext` locally.

## 1. Query log

21 WebSearch queries: 19 `standard`, 2 `extended` (extended used only where standard results were thin: calendar-aware warming and Twemcache hot keys). Independent queries were sent in the same turn.

| # | Mode | Query (shortened) | Result quality |
|---|---|---|---|
| Q1 | standard | proactive cache warming prefetching CDN predictive popularity pre-positioning | patents, vendor guides, one paper (MIRA) |
| Q2 | standard | scheduled cache warming before known traffic event, warm API Fastly/Cloudflare/Akamai/CloudFront | no official vendor docs returned; plugins and blogs |
| Q3 | standard | Scaling Memcache at Facebook leases thundering herd stale sets NSDI 2013 | course pages and notes only; paper not returned |
| Q4 | standard | hot key detection mitigation Redis Memcached Twemcache flash crowd | practitioner blogs, one Salesforce engineering post |
| Q5 | standard | TinyLFU Einziger Friedman Manes | arXiv and author pages |
| Q6 | standard | ARC Megiddo Modha FAST 2003 | USENIX pages |
| Q7 | standard | 3L-Cache / LHD / Cacheus learning-based replacement | USENIX FAST'25, FAST'21 pages, OSDI'26 PDF link |
| Q8 | standard | request coalescing collapsed forwarding Varnish nginx Envoy | blogs plus Fastly docs; nothing on Envoy |
| Q9 | standard | RFC 5861 stale-while-revalidate stale-if-error | rfc-editor, datatracker |
| Q10 | standard | Netflix concurrency-limits AIMD Vegas Gradient2 | GitHub ports, Netflix blog, explainers |
| Q11 | standard | load shedding Breakwater DAGOR Protego | USENIX / arXiv pages |
| Q12 | standard | multi-tenant cache fairness FairRide Memshare Cliffhanger | arXiv, USENIX, Berkeley pages |
| Q13 | standard | Redis client-side caching tracking BCAST | redis.io plus tutorials |
| Q14 | standard | Fastly soft purge | Fastly docs |
| Q15 | standard | consistent hashing with bounded loads | arXiv, Google Research |
| Q16 | standard | Cloudflare purge propagation architecture | Cloudflare blog, patents, third-party |
| Q17 | **extended** | demand-aware cache prewarming for scheduled events, flash sale, launch | practitioner blogs and one patent; no peer-reviewed work found |
| Q18 | standard | Envoy HTTP cache filter coalescing stale-while-revalidate | Envoy cache filter pages; nothing on coalescing or SWR found |
| Q19 | standard | S3-FIFO SOSP 2023 abstract | author PDF and blogs |
| Q20 | **extended** | Twemcache / Pelikan / Segcache hot key skew load balancing | Segcache pages only; nothing on hot keys |
| Q21 | standard | origin protection CDN origin shield collapsing adaptive concurrency stampede | blogs, Vercel, Cloudflare blog; nothing on adaptive limits |

Fetches: 28 WebFetch attempts. 4 failed outright (3 Varnish attempts: two redirects and one empty summary; 1 Netflix blog HTTP 403). 3 returned binary PDFs that the tool could not read; I converted those locally with `pdftotext`. The rest returned page summaries. Sources are listed per topic below.

## 2. Findings by topic

Each topic gives: sources, class, what the source establishes, which PhantomCache hypothesis it bears on, residual gaps. Capability ids (I1-I14) are defined in `vision/three_pillar/02_intelligence.md` section 3.

### T1. Proactive / predictive cache warming and prefetching (web, CDN, KV)

| Source | Class | Establishes |
|---|---|---|
| Koch, Werner, Rizk, Steinmetz, "MIRA: Proactive Music Video Caching using ConvNet-based Classification and Multivariate Popularity Prediction", IEEE MASCOTS 2018. `kom.tu-darmstadt.de/publications/KWR18-2` | OPENED-VIA-TOOL (landing page only; paper text not read) | One academic data point: proactive caching with past-view popularity prediction raised hit rate by "up to 4.5%" (author-reported, a music-video setting). |
| Patents US 8,886,769, 10,356,201, 9,871,881, 8,886,742, 9,161,080 (search results) | SNIPPET | CDN operators have patented popularity ranking and pre-positioning. Legal scope not read. |
| Netflix Open Connect overview (cited by earlier records) | NOT re-opened here (earlier record: SNIPPET) | Off-peak proactive fill exists for a finite catalogue. |
| Vendor / blog guides (Speed Kit, CacheFly, Aerospike academy, others) | SECONDARY | Practitioner advice: warm after purge or deploy, rate-limit warm-up, check cacheability and Vary first, require sustained popularity before warming. Numbers are anecdotal. |

Establishes: pre-warming is established practice. At least one peer-reviewed result reports a modest gain from popularity-predicted pre-fetching in a different setting. Practitioner sources warn that unthrottled warming can overload the origin.
Bears on: I3 (declared-demand pre-warming), I4, I10, experiment E9. The "warming can hurt the origin" warning supports the warm-budget design.
Gaps: no proceedings sweep for 2022-2026 warming work; no gateway-specific warming literature found; patents unread; Open Connect not re-opened.

### T2. Calendar- or schedule-aware warming

| Source | Class | Establishes |
|---|---|---|
| Q2 and Q17 results: warm-up tools (Warmup Rocks, plugins), cron-before-spike advice, a flash-sale checklist, a USPTO application on moving online stores between servers for flash-sale demand (US 11,893,614) | SECONDARY / SNIPPET | Scheduling a warm-up job before a known event (cron or CI step) is common practice. Vendor claims about Cloudflare/Fastly/Akamai "warmup endpoints" were NOT confirmed in any official documentation. |
| Cache vendor page arguing for ML-driven predictive warming | SECONDARY (marketing) | A product claim only. |

Establishes: the simple baseline (an operator cron warmer) exists and is widely used. That is the baseline I3 must beat.
Bears on: I3. This confirms the requirement to compare against a calendar-literal cron warmer (V2 baseline B3).
Gaps: **no peer-reviewed source found for a tenant-scoped declarative demand calendar with safety envelopes in a caching gateway. This is a coverage gap, not evidence of novelty.** Official docs of Cloudflare, Fastly, Akamai and CloudFront on warming or prefetch were not retrieved.

### T3. Flash-crowd and hot-key detection and mitigation; stampede protection

| Source | Class | Establishes |
|---|---|---|
| Jung, Krishnamurthy, Rabinovich, "Flash Crowds and Denial of Service Attacks", WWW 2002. `archives.iw3c2.org/www2002/CDROM/refereed/342.html` | OPENED-VIA-TOOL (full-paper page) | Flash events and DoS differ in client distribution, cluster overlap with past visitors (42.7% and 82.9% for two flash events vs about 0.6-14% for attacks, tool-reported), and per-client rates. New objects during a flash event miss and burst to the origin (peak near 450 req/s with 1,000 caches in one trace). A proposed adaptive CDN cut simulated origin peaks by roughly 50x and 20x (authors call it preliminary). |
| Nishtala et al., "Scaling Memcache at Facebook", NSDI '13, pp. 385-398. PDF | **OPENED-DIRECT** (section 3.2.1 read) | Leases: a 64-bit token bound to the key; a delete invalidates the token, which prevents stale sets. Rate-limiting tokens ("once every 10 seconds per key" by default) mitigates thundering herds; for a set of herd-prone keys the peak database query rate fell from 17K/s to 1.3K/s over one week (authors' data). |
| Vattani, Chierichetti, Lowenstein, "Optimal Probabilistic Cache Stampede Prevention", PVLDB 8(8), 2015 (XFetch). PDF | **OPENED-DIRECT** (abstract and introduction read) | Randomized early recomputation before expiry, with optimality claims. The introduction also lists background recomputation and locking as alternatives. |
| Redis, Segcache, Salesforce, codemia, oneuptime and similar | SECONDARY / SNIPPET | Practitioner mitigations: local L1 caches, key splitting, replicas, TTL jitter, application-level detection. Memcached exposes no per-key access statistics. Nothing found on Twemcache hot-key handling (two searches). |

Establishes: flash crowds on new or expired objects hit the origin (supports the plant for the integrated scenario). Leases and probabilistic early refresh are older, published mitigations of the same family as single-flight and refresh-ahead. Early detection of flash events versus attacks has a published characterization (client-side features, which a gateway may not have).
Bears on: I4 (XFetch is its strongest simple baseline), I5 (Jung et al. features), I2, and the existing single-flight coalescer (not a novelty claim).
Gaps: no search by name for Space-Saving/Count-Min (covered in `intelligence_prior_art_coverage.md` HK1-HK2 as bibliographic only); NetCache/SwitchKV not re-opened; Twemcache-specific hot-key literature NOT FOUND; hierarchical heavy hitters not searched.

### T4. Eviction and admission: TinyLFU, ARC, SIEVE, S3-FIFO, learning-based

| Source | Class | Establishes |
|---|---|---|
| Einziger, Friedman, Manes, "TinyLFU", arXiv:1512.00727 (v1/v2 Dec 2015); journal version ACM TOS 2017 (DOI from a search result, not opened) | OPENED-VIA-TOOL (abstract page) | Frequency-based admission with a compact approximate-LFU structure; W-TinyLFU. |
| Megiddo, Modha, "ARC", FAST '03. `usenix.org/conference/fast-03/...` | OPENED-VIA-TOOL (abstract page) | Self-tuning recency/frequency balance; scan resistance; 23 traces; SPC1-like example 9.19% (LRU) vs 20% (ARC) at 4 GB (authors). |
| Zhang et al., "SIEVE is Simpler than LRU", NSDI '24 | OPENED-VIA-TOOL | Up to 63.2% lower miss ratio than ARC on 1,559 traces; 5 production libraries; fewer than 20 changed lines on average (authors). |
| Yang et al., "FIFO Queues are All You Need for Cache Eviction" (S3-FIFO), SOSP '23 | **OPENED-DIRECT** (abstract, introduction) | Three static FIFO queues; 6,594 traces from 14 datasets; lowest mean miss ratio on 10 of 14 datasets; 6x throughput of optimized LRU at 16 threads (authors). Insight: one-hit-wonder ratio is much higher in short sub-sequences. |
| Song et al., "Learning Relaxed Belady" (LRB), NSDI '20 | OPENED-VIA-TOOL | GBM approximates Belady; 5-24% less WAN traffic on 6 CDN traces; prototype in Apache Traffic Server (authors). |
| Zhou et al., "3L-Cache", FAST '25, pp. 237-254 | OPENED-VIA-TOOL | Low-overhead learned eviction; 60.9% lower CPU than HALP, 94.9% lower than LRB; 4,855 traces; 6.4x LRU overhead for small caches (authors). |
| Cacheus (FAST '21), LHD (NSDI '18), Darwin, LAH (OSDI '26) | SNIPPET (this sweep); bibliographic rows exist in `intelligence_prior_art_coverage.md` | Exist; not opened. |

Establishes: the field is crowded. Strong simple baselines (S3-FIFO, SIEVE, W-TinyLFU) are published and shipped. Learned policies have published overhead costs.
Bears on: I7 (strategy selection). Its baseline must be the best fixed faithful policy, not the product default. Also on the eviction-policy menu for the action interface.
Gaps: none of these results was measured on this gateway. Full texts of SIEVE, ARC, TinyLFU, LRB and 3L-Cache not read. Whether the serving cache uses the faithful policies is a separate repo question (see 02, section 1).

### T5. Request coalescing / collapsed forwarding

| Source | Class | Establishes |
|---|---|---|
| nginx `ngx_http_proxy_module` docs (`nginx.org`) | OPENED-VIA-TOOL | `proxy_cache_lock` default off; waiters wait up to `proxy_cache_lock_timeout` (default 5 s) then go to the origin and the response is not cached; `proxy_cache_lock_age` 5 s; `proxy_cache_use_stale updating`; `proxy_cache_background_update`. |
| Varnish users guide, grace mode (`varnish.org/docs/users-guide/vcl-grace`) | OPENED-VIA-TOOL (fourth attempt) | One backend request, others held on a waiting list; grace and keep add together (example 10 s + 1 min = 70 s past TTL); `std.healthy` used to vary grace by backend health. |
| Fastly request collapsing guide | OPENED-VIA-TOOL | Waiting-list collapsing; uncacheable or failed fetches can serialize waiters ("several minutes" in bad cases, tool-reported); mitigations: hit-for-pass, timeouts. |
| RFC 9111 section 4 | OPENED-VIA-TOOL (first 100,000 of 108,250 characters) | The standard describes collapsing of multiple requests into one forward request on a miss. |
| Envoy cache filter docs | SNIPPET | No mention of coalescing or SWR found in the snippets. **Not established either way.** |

Establishes: single-flight/collapsing is standard in Varnish, nginx and Fastly, and described in RFC 9111. Collapsing is per cache node (per Fastly's scope description; cluster-wide collapsing needs a shield or tier).
Bears on: the existing coalescer, the distributed "no cross-node single-flight" gap (02 section 5), and the E9 baseline B0.
Gaps: Envoy, Pingora, ATS, Squid, HAProxy docs not opened; cluster-wide collapsing literature not searched.

### T6. Stale-while-revalidate / stale-if-error; RFC 5861 and RFC 9111

| Source | Class | Establishes |
|---|---|---|
| RFC 5861 (`rfc-editor.org/rfc/rfc5861`) | OPENED-VIA-TOOL | Informational RFC (Independent Submission, May 2010), M. Nottingham. Defines `stale-while-revalidate` and `stale-if-error` with `delta-seconds`. It does not cover request collapsing. |
| RFC 9111 (`rfc-editor.org/rfc/rfc9111`) | OPENED-VIA-TOOL (partial) | Internet Standards Track (STD 98). Does not define SWR/SIE by name; section 4.2.4 allows stale responses when permitted by a client, origin extension such as RFC 5861, or an out-of-band contract. Unrecognized directives must be ignored (section 5.2.3). |
| Fastly soft purges guide | OPENED-VIA-TOOL | Soft purge marks content stale instead of removing it; recommends ETag/Last-Modified or SWR plus SIE; purge-all cannot be soft. |

Establishes: SWR and SIE are informational extensions, standardized behaviour for stale use is in RFC 9111. The repo's own supported-subset claim (`rfc9111_supported_subset.md`) was not re-checked here.
Bears on: I8 (TTL recommender), the stale-window ceilings in the action interface, and the finding that the dynamic stale window never bound in sealed M6.
Gaps: errata (EID 2255 appeared in a search snippet) not read; the last 8,250 characters of RFC 9111 not read.

### T7. Adaptive concurrency limits (Netflix, Envoy; AIMD, Vegas, Gradient)

| Source | Class | Establishes |
|---|---|---|
| Netflix `concurrency-limits` README (GitHub) | OPENED-VIA-TOOL | Java library applying TCP congestion-control ideas; Vegas (delay-based, queue estimate `L*(1 - minRTT/sampleRTT)`, alpha/beta thresholds) and Gradient2 (long vs short latency averages). As summarized by the tool, AIMD appears only in gRPC client guidance, not in the algorithms section. Grounded in Little's law. |
| Envoy adaptive concurrency filter docs | OPENED-VIA-TOOL | Gradient controller: periodically pins the limit to `min_concurrency` to measure minRTT; `gradient = (minRTT + B) / sampleRTT`, `new limit = gradient * old limit + headroom`, headroom is the square root of the limit. Docs page header showed a development version. |
| Netflix Tech Blog "Performance Under Load" | SNIPPET only (fetch returned HTTP 403) | Launch post for the library (per search summary). |
| TCP Vegas (SIGCOMM '94), BBR, Chiu-Jain AIMD, CoDel RFC 8289 | bibliographic rows in `intelligence_prior_art_coverage.md` (not opened here) | Background literature. |

Establishes: adaptive concurrency limiting is published and shipped in two mainstream systems. Both compare a measured latency against an estimated minimum or long-run baseline, not an absolute ceiling. This is consistent with the repo's own finding that its absolute-ceiling AIMD behaved as a threshold switch. It is not evidence about whether a relative controller would beat a static cap on a caching gateway.
Bears on: I6, experiments E1-E3, and the preserved negative M6/Scenario B result (H1 NOT_SUPPORTED stands).
Gaps: no study of adaptive limits for cache-origin protection specifically was found (two searches); the Netflix code was not read; bounded wait queue studies not searched.

### T8. Load shedding and overload control

| Source | Class | Establishes |
|---|---|---|
| DAGOR (WeChat, arXiv:1806.04075), Breakwater (OSDI '20), Protego (NSDI '23) | SNIPPET (search summaries; primary URLs returned, not opened) | Service-level overload control with priority admission (DAGOR), server-driven credits from queueing delay (Breakwater), marginal-throughput credits for lock contention (Protego). Figures quoted by the search tool were not checked. |
| Google SRE book ch. 21, Fail at Scale | bibliographic rows in earlier record | Client-side adaptive throttling; CoDel and adaptive LIFO. |

Establishes: shedding with priorities and credits is a studied area. Scenario B's shed-vs-peak trade-off is the expected shape of any cap.
Bears on: I6 (bounded wait queue, shed policy), the warm-versus-organic priority rule.
Gaps: none of these was opened; Rajomon, TopFull not verified; cache-specific shedding not searched.

### T9. Multi-tenant cache fairness and partitioning

| Source | Class | Establishes |
|---|---|---|
| Cidon, Rushton, Rumble, Stutsman, "Memshare", arXiv:1610.08129 (ATC '17 version not opened) | OPENED-VIA-TOOL (abstract page) | Per-application guaranteed memory plus a shared pool allocated by hit-rate gradient; on a week-long memcached trace combined hit rate 84.7% to 90.8%, misses down 39.7% (authors). |
| FairRide (NSDI '16), Cliffhanger (NSDI '16), Tingo, OpuS | SNIPPET | Isolation versus strategy-proofness trade-off (FairRide); shadow queues for hit-rate gradients. |

Establishes: reserved minimums plus dynamic sharing is a published design. Cheating by spurious accesses is a known attack on shared caches.
Bears on: I9 (per-tenant advice) and the tenant-protection rules. PhantomCache has namespace isolation but no per-tenant resource quotas (per V1/V3, reported).
Gaps: HTTP-gateway multi-tenant literature not found; FairRide, Cliffhanger not opened.

### T10. Distributed cache invalidation (Redis, Fastly, Cloudflare)

| Source | Class | Establishes |
|---|---|---|
| Redis client-side caching guide and `CLIENT TRACKING` | OPENED-VIA-TOOL | Server tracks keys per connection and pushes invalidations; BCAST by prefix; if a connection drops, the client flushes its whole client-side cache. |
| Fastly soft purge guide | OPENED-VIA-TOOL | See T6. |
| Cloudflare "Instant Purge" blog | OPENED-VIA-TOOL | Per-machine index plus local purge queue; purge masks hits before disk deletion; global P50 149 ms (Aug 2024) vs 1,570 ms (May 2022); no p99 given; behaviour under partitions and index loss not described. |

Establishes: bounded-state server-pushed invalidation, soft purge and fast fan-out purge are all established. Redis shows the conservative recovery pattern (flush on disconnect) that resembles PhantomCache's flush-on-gap.
Bears on: the cluster-wide warming blocker in 02 section 5 (the reproduced M3 durability defect is a repo fact, not from these sources) and the claims table.
Gaps: no peer-reviewed invalidation-durability literature searched; Cloudflare's partition behaviour is undocumented in the retrieved page.

### T11. Consistent hashing and bounded-load placement

| Source | Class | Establishes |
|---|---|---|
| Mirrokni, Thorup, Zadimoghaddam, "Consistent Hashing with Bounded Loads", arXiv:1608.01350 (SODA 2018 per search result, not opened) | OPENED-VIA-TOOL (abstract) | Capacity cap `ceil(c*m/n)`, c = 1+eps; moves grow by O(1/eps^2) for eps <= 1. |
| AAAI 2021 revisit (cascaded overflow) | SNIPPET | Exists. |

Establishes: bounded-load placement is a known fix for hot nodes under skew. PhantomCache uses rendezvous routing (repo docs; not re-read here); its hot-key behaviour was not evaluated.
Bears on: I12 (distributed hot-key handling). Note that bounded loads balances client-to-server assignment; a single hot key still maps to one owner unless replicated.
Gaps: rendezvous-hash literature not searched; hot-key replication literature not searched.

## 3. Proposed claims table

| Statement about PhantomCache | May claim today? | Basis |
|---|---|---|
| "Pre-warming and prefetching are established practice, so PhantomCache's calendar warming is a standard idea with a safety envelope" | **May say it** (as context) | T1, T2: patents, vendor practice, MIRA (OPENED-VIA-TOOL, landing page) |
| "PhantomCache's Demand Calendar is novel" | **MAY NOT** | Absence of results is not evidence of absence (T2 gap). No sweep of 2022-2026 proceedings was done. |
| "Tenant-scoped, audited, budget-bounded pre-warming with hard-cap enforcement is a differentiator" | **MAY NOT** (hypothesis only) | Not verified against incumbents; no implementation exists in the product |
| "PhantomCache coalesces concurrent misses (single-flight)" | May state as a standard mechanism | T5: nginx, Varnish, Fastly, RFC 9111 describe it. No novelty. |
| "Generation fencing / leases for stale sets is new" | **MAY NOT** | T3: Facebook leases (OPENED-DIRECT) are the same family |
| "Hot-key detection and probabilistic early refresh are new" | **MAY NOT** | T3: XFetch (OPENED-DIRECT); sketch literature in earlier record |
| "Adaptive origin concurrency control improves latency or origin protection" | **MAY NOT** | Sealed M6 H1 NOT_SUPPORTED (Tier 1, SIMULATED); Scenario B notice. T7 shows mainstream limiters use relative signals, which the repo's AIMD did not. |
| "A relative or Little's-law-derived cap beats the best static cap" | **MAY NOT** (hypothesis; E1-E3) | No evidence in repo or in the T7 sources about caching gateways |
| "Eviction strategy selection beats the best fixed policy" | **MAY NOT** | T4: crowded; no measurement on this gateway; E5 not run |
| "Learned eviction is worth shipping" | **MAY NOT** | T4: published overhead; repo ML gate not passed |
| "SWR/SIE are supported per RFC 5861 / RFC 9111" | May say "implemented in part"; conformance claim only per `rfc9111_supported_subset.md` | T6 establishes status of the RFCs only; repo conformance not re-verified here |
| "Server-pushed, bounded-state invalidation with a stated failure model exists in the same family as Redis tracking and Fastly/Cloudflare purge" | May say it | T10; and the repo's own limitation: invalidations are lost if the issuer dies (reproduced M3 defect) |
| "Cluster-wide invalidation is durable" | **MAY NOT** | Repo record (accountability section 12 update): defect reproduced; not fixed in the integrated tree |
| "Per-tenant resource isolation exists" | **MAY NOT** | Namespace isolation only (V1/V3, reported). T9 shows what a resource-isolation design looks like. |
| "RT-0 is closed" | **MAY NOT** | Open (this record, `intelligence_prior_art_coverage.md`, `STATUS.md`) |

## 4. What RT-0 still does not cover

1. **No systematic proceedings sweep** (OSDI, NSDI, SOSP, ATC, FAST, EuroSys, SoCC, SIGCOMM, SIGMETRICS, VLDB, SIGMOD, 2022-2026). The search tool did not reach ACM DL, IEEE Xplore, DBLP or Google Scholar directly.
2. **Full texts** were read only for three papers (Facebook NSDI'13 sections 3.2.1, S3-FIFO abstract and introduction, XFetch abstract and introduction). Every other "OPENED-VIA-TOOL" item is a summarized abstract or documentation page.
3. **Patents** were not read. Akamai US 10,154,068 and US 10,440,142 (cited by `novelty_snapshot_m6.md`) were not re-opened here.
4. **Vendor documentation** not opened: Cloudflare cache/prefetch docs, AWS CloudFront, Akamai, ATS, Squid, HAProxy, Pingora, Envoy cache filter (snippets only).
5. **Topics with no source found**: operator-declared demand calendars feeding a gateway cache; early flash-crowd detection inside a caching gateway; restart or flush warm-up scheduling for in-memory gateway caches; per-tenant seasonality for HTTP gateways; origin-pressure forecasting for admission; Twemcache hot-key handling.
6. **Not searched**: hierarchical heavy hitters; cache-key fragmentation or query-string normalization; peer-fill / owner-fetch and tiered caching literature; probabilistic staleness bounds beyond the V1 bibliographic row; rendezvous hashing; hedged requests; LLM/ML-based cache tuning products.
7. **Author-reported numbers** (hit-rate gains, throughput multipliers, trace counts) are quoted as such. None was reproduced.
8. **The M6 novelty snapshot's matrix and venue sweep** were not re-verified. Its "RT-0 PASS" stays corrected by its own notice.
9. A real closure needs a dated plan: a fixed list of venues and years, query strings recorded, each hit triaged, and the closest candidates read in full. That plan is an owner decision (see `02_intelligence.md` section 8, O-9).
