# Intelligence Candidates - Prior-Art Coverage (verification pass, 2026-10-30)

**Status: PARTIAL. RT-0 is NOT closed. Nothing in this file is a claim of novelty for PhantomCache, now or conditionally.**
Scope: the techniques behind the intelligence candidates in `phantomcache_milestone_accountability_report.md` section 11 - concurrency/admission control, hot-key detection, eviction/admission policies (including learned and bandit-based), adaptive TTL and freshness, CDN origin protection, and workload forecasting. Method: web search (WebSearch tool, mode `standard` only, no `extended` runs) to check each citation against a primary-source page *as returned and summarised by the search tool*. **Full texts were not opened or read in this pass** (no WebFetch); "verified" therefore means *bibliographic metadata (title, authors, venue, year) and the abstract-level finding matched a primary-source page that the search returned*. It does not mean the paper's claims were audited. Findings quoted are the authors' abstract-level claims, not independently confirmed results.

## 1. Evidence classes used

| Class | Meaning |
|---|---|
| **A** | **Verified from a primary source**: a publisher / proceedings / standards-body / vendor-documentation / author-institution page was returned by the search and its text matched title, authors, venue and year (any exception is noted in the row). |
| **B** | **Secondary only**: metadata matched only on mirrors, blogs, Wikipedia, course pages, library records or mailing lists; the primary page was not returned. Probably correct, not primary-verified. |
| **C** | **Cited in repo docs only**: appears in `literature_review.md`, `novelty_snapshot_m6.md` or `candidate_comparison.md`; not searched or not confirmed in this pass. The repo docs' own tags (OPENED / SPOT-CHECKED / SNIPPET) are agent-reported and were not re-derived here. |
| **D** | **Not verified**: searched, not found, or the result contradicted the claim. |
Confidence (H/M/L) refers to my confidence that the *bibliographic data in the row is correct*, not to the paper's merit.

## 2. Verified and candidate citations

### 2.1 Concurrency / admission / overload control
| ID | Work | Authors | Venue, year | URL | One-line finding (abstract level) | Class | Conf. |
|---|---|---|---|---|---|---|---|
| CC1 | Envoy "Adaptive Concurrency" HTTP filter docs | Envoy project | docs (current + v1.13/v1.15 pages) | https://www.envoyproxy.io/docs/envoy/latest/configuration/http/http_filters/adaptive_concurrency_filter | Gradient controller: periodically measured ideal RTT (minRTT) vs sampled RTT adjusts the concurrency limit; tunables include `min_rtt_buffer`, `min_concurrency` | A | H |
| CC2 | Netflix Tech Blog, "Performance Under Load" | Netflix (author names not in the result) | vendor blog, c. 2018 (date not shown) | https://netflixtechblog.medium.com/performance-under-load-3e6fa9a60581 | Replaces hand-tuned fixed concurrency limits with adaptive limits; open-sourced implementation | A | M |
| CC3 | Netflix/concurrency-limits (Vegas, Gradient, Gradient2, AIMD, ... limit classes) | Netflix | GitHub repo, Apache-2.0 | github.com/Netflix/concurrency-limits (**URL not returned**; search returned third-party ports and a secondary index) | Delay-based (Vegas / Gradient2) and loss-style (AIMD) limit algorithms; Gradient2 compares long- and short-term latency with a tolerance | B | M |
| CC4 | TCP Vegas: New Techniques for Congestion Detection and Avoidance | L. Brakmo, S. O'Malley, L. Peterson | ACM SIGCOMM '94, pp. 24-35 | https://www2.cs.arizona.edu/projects/xkernel/people/brakmo-papers/abstracts.html (also TR 94-04) | Delay-based congestion avoidance; reported 40-70% higher throughput than Reno with 1/5-1/2 the losses | A | H |
| CC5 | BBR: Congestion-Based Congestion Control | N. Cardwell, Y. Cheng, C. S. Gunn, S. H. Yeganeh, V. Jacobson | ACM Queue 14(5), 2016 | https://research.google/pubs/bbr-congestion-based-congestion-control-2/ | Models bottleneck bandwidth and RTT; paces to the model rather than reacting to loss. *Author list is truncated in the returned snippet; taken from my knowledge and the Queue citation.* | A | M |
| CC6 | Analysis of the Increase and Decrease Algorithms for Congestion Avoidance in Computer Networks | D.-M. Chiu, R. Jain | Computer Networks and ISDN Systems 17(1):1-14, 1989 | https://profiles.wustl.edu/en/publications/analysis-of-the-increase-and-decrease-algorithms-for-congestion-a/ | Additive-increase / multiplicative-decrease converges to an efficient and fair state (binary feedback) | A | H |
| CC7 | RFC 8289 Controlled Delay Active Queue Management (CoDel) | K. Nichols, V. Jacobson, A. McGregor (ed.), J. Iyengar (ed.) | IETF RFC (Experimental), Jan 2018 | https://www.rfc-editor.org/info/rfc8289 | Estimator + setpoint + control loop keeping queue delay low while allowing bursts. The original ACM Queue article (CODEL2012) was **not** returned | A (RFC) / D (2012 article) | H / L |
| CC8 | "Fail at Scale" (CoDel, adaptive LIFO, concurrency control at Facebook) | B. Maurer | ACM Queue / CACM, 2015 | search returned only mirrors (unpaywall record, Colyer blog) | Controlled delay + adaptive LIFO + client-side concurrency control as overload defences | B | M |
| CC9 | Google SRE Book ch. 21 "Handling Overload" | chapter authors not shown in the result | Google SRE book, 2016 | https://sre.google/sre-book/handling-overload/ | Client-side adaptive throttling from requests/accepts ratio with multiplier K | A | M (authors) |
| CC10 | J. D. C. Little, "A Proof for the Queuing Formula: L = lambda W" | J. D. C. Little | Operations Research 9(3):383-387, 1961, DOI 10.1287/opre.9.3.383 | metadata via mirrors; related primary page: https://pubsonline.informs.org/doi/fpi/10.1287/opre.1110.0940 (Little, 50th-anniversary essay) | L = lambda W under finite means and stationarity | B | H |
| CC11 | Breakwater (OSDI'20), DAGOR (SoCC'18), Rajomon (NSDI'25), TopFull (SIGCOMM'24), Protego (NSDI'23) | as in `literature_review.md` 4 / 4b | various | not searched | service-level credit / queue-delay admission and rate control, no cache semantics (repo-doc statement) | C | n/a |

### 2.2 Hot-key detection / heavy hitters
| ID | Work | Authors | Venue, year | URL | Finding | Class | Conf. |
|---|---|---|---|---|---|---|---|
| HK1 | Efficient Computation of Frequent and Top-k Elements in Data Streams (Space-Saving) | A. Metwally, D. Agrawal, A. El Abbadi | ICDT 2005, LNCS 3363, pp. 398-412 (UCSB TR 2005-23); journal version ACM TODS 31(3):1095-1133, 2006 | https://www.cs.ucsb.edu/research/tech-reports/2005-23 | Counter-based summary for frequent/top-k with error guarantees; space shrinks on skewed data | A (ICDT/TR) / B (TODS pages) | H / M |
| HK2 | An Improved Data Stream Summary: The Count-Min Sketch and its Applications | G. Cormode, S. Muthukrishnan | J. Algorithms 55(1):58-75, 2005, DOI 10.1016/j.jalgor.2003.12.001 (prelim. LATIN 2004) | https://www.cs.ox.ac.uk/people/graham.cormode/pubs/html/CormodeMuthukrishnan04CMJalg.html | Sublinear-space sketch for point/range/inner-product queries; 1/epsilon rather than 1/epsilon^2 | A | H |
| HK3 | Finding Repeated Elements (Misra-Gries) | J. Misra, D. Gries | Science of Computer Programming 2(2):143-152, 1982 | Cornell tech-report record https://ecommons.cornell.edu/entities/publication/e26739f8-be64-4927-96a0-64aa2f5fe5a7 ; journal metadata via Wikipedia | Finds values occurring more than n/k times in O(n log k) with O(k) space | B | M |
| HK4 | Production hot-key tooling (Redis `--hotkeys` requires LFU policy; Tencent Cloud, T-Systems docs) | vendors | docs / blogs | e.g. https://intl.cloud.tencent.com/document/product/239/47573 | Hot-key detection exists in managed Redis offerings by counters / monitoring; **no academic hot-key-in-cache paper was found by this query** | B | M |

### 2.3 Eviction, admission, learned and bandit policies
| ID | Work | Authors | Venue, year | URL | Finding | Class | Conf. |
|---|---|---|---|---|---|---|---|
| EV1 | TinyLFU: A Highly Efficient Cache Admission Policy (and W-TinyLFU) | G. Einziger, R. Friedman, B. Manes | arXiv:1512.00727; ACM ToS 13(4), 2017, DOI 10.1145/3149371 (journal data via a BGU record); earlier PDP 2014 | https://arxiv.org/abs/1512.00727 | Frequency-sketch admission filter; W-TinyLFU adds a window | A (arXiv) / B (ToS details) | H / M |
| EV2 | ARC: A Self-Tuning, Low Overhead Replacement Cache | N. Megiddo, D. S. Modha | USENIX FAST '03 | https://www.usenix.org/conference/fast-03/arc-self-tuning-low-overhead-replacement-cache | Adaptive recency/frequency balance, scan-resistant, O(1); 23 traces | A | H |
| EV3 | LIRS | S. Jiang, X. Zhang | ACM SIGMETRICS 2002, DOI 10.1145/511399.511340 (DOI via SciSpace; Wikipedia's DOI looked wrong) | mirrors only (e.g. https://u.cs.biu.ac.il/~wisemay/2os/lru/lirs.pdf) | Ranks blocks by inter-reference recency; reported to beat LRU and contemporaries | B | M |
| EV4 | SIEVE is Simpler than LRU | Y. Zhang, J. Yang, Y. Yue, Y. Vigfusson, K. V. Rashmi | NSDI '24, pp. 1229-1246 | https://www.usenix.org/conference/nsdi24/presentation/zhang-yazhuo | FIFO with a moving hand and visited bit; up to 63.2% lower miss ratio than ARC on 1,559 traces (author-reported) | A | H |
| EV5 | FIFO queues are all you need for cache eviction (S3-FIFO) | J. Yang, Y. Zhang, Z. Qiu, Y. Yue, K. V. Rashmi | SOSP '23 | https://junchengyang.com/publication/sosp23-s3fifo.pdf (author-hosted); repo github.com/Thesys-lab/sosp23-s3fifo | Small/main/ghost FIFOs; lowest mean miss ratio on 10 of 14 datasets (6,594 traces, author-reported) | A | H |
| EV6 | Learning Cache Replacement with CACHEUS | L. V. Rodriguez et al. | USENIX FAST '21, pp. 341-354 | https://usenix.org/conference/fast21/presentation/rodriguez | Workload primitives (LFU-/LRU-friendly, scan, churn); SR-LRU + CR-LFU experts; 329 workloads. *My first query used a wrong title; this is the title returned* | A | H |
| EV7 | Driving Cache Replacement with ML-based LeCaR | G. Vietri et al. | USENIX HotStorage '18 | https://www.usenix.org/conference/hotstorage18/presentation/vietri | Regret-minimisation over LRU/LFU; reported to beat ARC | A | H |
| EV8 | Learning Relaxed Belady for CDN Caching (LRB) | Z. Song, D. S. Berger, K. Li, W. Lloyd | NSDI '20, pp. 529-544 | https://www.usenix.org/conference/nsdi20/presentation/song | GBM approximates Belady with a "relaxed" boundary; 5-24% lower WAN traffic on 6 CDN traces | A | H |
| EV9 | GL-Cache: Group-level learning for efficient and high-performance caching | J. Yang, Z. Mao, Y. Yue, K. V. Rashmi | USENIX FAST '23, pp. 115-134 | https://www.usenix.org/conference/fast23/presentation/yang-juncheng | Group-level learned utility; vs LRB 228x throughput and +7% hit ratio (author-reported) | A | H |
| EV10 | LHD: Improving Cache Hit Rate by Maximizing Hit Density | N. Beckmann, H. Chen, A. Cidon | NSDI '18, pp. 389-403 | https://www.usenix.org/conference/nsdi18/presentation/beckmann | Rank by expected hits per unit space; ~8x less space than LRU for equal hit rate (author-reported) | A | H |
| EV11 | Darwin: Flexible Learning-based CDN Caching | J. Chen, N. Sharma, T. Khan, S. Liu, B. Chang, A. Akella, S. Shakkottai, R. K. Sitaraman | ACM SIGCOMM '23, DOI 10.1145/3603269.3604863 | https://par.nsf.gov/biblio/10451901 | Clustering + neural bandit selecting among admission "experts"; 3-43% higher object hit rate (author-reported). Matches the repo's own correction (first author Chen) | A | H |
| EV12 | AdaptSize | D. S. Berger, R. K. Sitaraman, M. Harchol-Balter | NSDI '17 | https://www.usenix.org/conference/nsdi17/technical-sessions/presentation/berger | Size-aware admission tuned via a Markov model; +30-48% vs Nginx, +47-91% vs Varnish (author-reported) | A | H |
| EV13 | Hyperbolic Caching | A. Blankstein, S. Sen, M. J. Freedman | USENIX ATC '17, pp. 499-511 | https://www.usenix.org/conference/atc17/technical-sessions/presentation/blankstein | Sampled eviction with decaying priorities; pluggable cost/expiry; 10-20% lower miss rate (author-reported) | A | H |
| EV14 | Learning-Augmented Heuristics (LAH / S4-FIFO) | H. Xia et al. (first author as returned; full list not shown) | arXiv:2608.27975; USENIX OSDI '26 | https://www.usenix.org/conference/osdi26/presentation/xia | Learns cache-level parameters of S3-FIFO; +26% mean efficiency vs S3-FIFO; worst-trace +0.8% vs FIFO (author-reported). **Upgrades the repo's "accepted per arXiv comments" to a USENIX OSDI '26 page** | A | M-H |
| EV15 | SCION (arXiv:2605.01055), 3L-Cache (FAST '25), ACME (WDAS 2002), Cliffhanger (NSDI '16), SHARDS (FAST '15), Miniature Simulations (ATC '17), Segcache (NSDI '21), CacheLib (OSDI '20) | as in `literature_review.md` | various | not searched here | policy orchestration, learned eviction, shadow/MRC techniques (repo-doc statements) | C (8 items) | n/a |

### 2.4 Freshness, stale serving, adaptive TTL
| ID | Work | Authors | Venue, year | URL | Finding | Class | Conf. |
|---|---|---|---|---|---|---|---|
| FR1 | RFC 9111 HTTP Caching | R. Fielding (ed.), M. Nottingham (ed.), J. Reschke (ed.) | IETF Internet Standard, June 2022 (obsoletes RFC 7234) | https://datatracker.ietf.org/doc/rfc9111 | Normative HTTP caching; **does not itself define stale-while-revalidate** (IANA registry points to RFC 5861). The repo's remarks on request collapsing / stale conditions in RFC 9111 were not re-checked | A | H |
| FR2 | RFC 5861 HTTP Cache-Control Extensions for Stale Content | M. Nottingham | IETF Informational RFC, May 2010 | https://www.rfc-editor.org/rfc/rfc5861 | Defines `stale-while-revalidate` and `stale-if-error` | A | H |
| FR3 | Adaptive TTL-Based Caching for Content Delivery | S. Basu, A. Sundarrajan, J. Ghaderi, S. Shakkottai, R. Sitaraman | arXiv:1704.04448; IEEE/ACM ToN 26(3), June 2018 | https://arxiv.org/abs/1704.04448 | d-TTL / f-TTL stochastic-approximation TTL control converging to hit-rate / size targets within ~1.3% (500M-request CDN trace) | A | H |
| FR4 | Revisiting Cache Freshness for Emerging Real-Time Applications | Z. Mao, R. Iyer, S. Shenker, I. Stoica | ACM HotNets '24, DOI 10.1145/3696348.3696858; arXiv:2412.20221 | https://conferences.sigcomm.org/hotnets/2024/papers/hotnets24-21.pdf | TTLs are too coarse for real-time freshness; write-reactive freshness model with an adaptive policy (the repo notes a bottleneck-detection sketch, not re-checked) | A | H |
| FR5 | Akamai patents US 10,154,068 and US 10,440,142 | (inventors not retrieved) | patents | not searched | Adaptive SWR window from request rate; TTL adaptation to an offload target (repo-doc reading, **not** re-read; legal scope unassessed) | C (2 items) | n/a |
| FR6 | Labrinidis & Roussopoulos (VLDB '03/'04); Qu, Labrinidis, Mosse "UNIT" (ICDE '06); Azar et al. (PNAS 2018); Kolobov et al. (SIGIR '19) | | | not searched | freshness-versus-load feedback; refresh under a bandwidth budget (repo-doc statements) | C (4 items) | n/a |

### 2.5 CDN / origin protection
| ID | Work | Authors | Venue, year | URL | Finding | Class | Conf. |
|---|---|---|---|---|---|---|---|
| OP1 | Varnish "Grace mode and keep" | Varnish project | docs (6.x/7.x) | https://varnish-cache.org/docs/7.3/users-guide/vcl-grace.html | Grace serves expired objects while one backend fetch refreshes; request coalescing via a waiting list; `std.healthy()` is used in VCL to vary grace by backend health | A | H |
| OP2 | nginx `proxy_cache_lock` / `proxy_cache_use_stale updating` | nginx project | docs | official nginx.org page **not returned**; mailing-list and commit history only (e.g. https://mailman.nginx.org/pipermail/nginx/2016-January/049734.html) | Lock covers new entries; `use_stale updating` limits refresh traffic | B | M |
| OP3 | Fastly request collapsing and serving stale (SWR / stale-if-error) | Fastly | docs | https://www.fastly.com/documentation/guides/concepts/cache/request-collapsing/ | Collapsed followers receive a stale-if-error response chosen by the leader; disabling collapsing makes stale objects unusable | A | H |
| OP4 | Optimal Probabilistic Cache Stampede Prevention (XFetch) | A. Vattani, F. Chierichetti, K. Lowenstein | PVLDB 8(8):886-897, 2015, DOI 10.14778/2757807.2757813 | https://mail.vldb.org/pvldb/volumes/8/paper/Optimal%20Probabilistic%20Cache%20Stampede%20Prevention | Probabilistic early expiration, provably optimal | A | H |
| OP5 | Scaling Memcache at Facebook | R. Nishtala et al. | NSDI '13, pp. 385-398 | https://www.usenix.org/node/172909 | Leases (~10 s token), stale-set prevention, Gutter; the 17K -> 1.3K queries/s figure came from secondary sources only | A (metadata) / B (numbers) | H / L |
| OP6 | Caching with Delayed Hits | N. Atre, J. Sherry, W. Wang, D. S. Berger | ACM SIGCOMM '20 | https://www.pdl.cmu.edu/PDL-FTP/Storage/sigcomm2020-natre_abs.shtml | Queued requests on an in-flight miss ("delayed hits"); Belatedly up to 45% lower latency than Belady; MAD policy | A | H |
| OP7 | Latency Guarantees for Caching with Delayed Hits | Gurushankar, Singer, Subercaseaux (per repo doc) | INFOCOM '25; arXiv:2501.16535 | https://arxiv.org/pdf/2501.16535 | Model follow-up; LRU is O(Zk)-competitive. Title and arXiv id confirmed; authors/venue from the repo doc | A (id) / C (authors) | M |
| OP8 | RobinHood: Tail Latency Aware Caching | D. S. Berger, B. Berg, T. Zhu, S. Sen, M. Harchol-Balter | OSDI '18, pp. 195-212 | https://www.usenix.org/conference/osdi18/presentation/berger | Reallocates cache space across backends from latency feedback; 99.7% within a 150 ms P99 goal vs 70% (author-reported) | A | H |
| OP9 | Metastable Failures in the Wild | L. Huang et al. (slug `huang-lexiang`; full list not shown) | OSDI '22 | https://www.usenix.org/conference/osdi22/presentation/huang-lexiang | Study of metastable-failure incidents; cache loss as a trigger | A | M |
| OP10 | Characterizing Metastable Faults and Failures | A. Farahbakhsh, Q. Lu, L. Alvisi, A. Haeberlen, R. Van Renesse | arXiv:2606.00942, 31 May 2026 (the repo's "submitted to SOSP 2026" not re-checked) | https://arxiv.org/pdf/2606.00942 | Causal model of metastability; Nyx DSL; three case studies | A | H |
| OP11 | Metastable Failures in Distributed Systems (HotOS '21), N. Bronson et al. | | | not searched | position paper (repo doc, full text claimed read) | C | n/a |
| OP12 | Squid `collapsed_forwarding`, Apache Traffic Server stale_response, Cloudflare Always Online, CloudFront origin groups / Origin Shield, Next.js ISR, Caffeine `refreshAfterWrite`, Go `singleflight` | vendors | docs | not searched | collapsing and static/binary stale features (repo-doc statements, ~30 pages claimed opened by a prior agent) | C (7 items) | n/a |

### 2.6 Workload forecasting for caches
| ID | Work | Authors | Venue, year | URL | Finding | Class | Conf. |
|---|---|---|---|---|---|---|---|
| WF1 | DeepCache (LSTM encoder-decoder popularity forecasting + caching policy) | authors not shown in the result | SIGCOMM CCR, 2018 (per the result text) | course-hosted copy https://classpages.cselabs.umn.edu/Spring-2019/csci8980/papers/deepcache.pdf | Forecasts object popularity 1/12/24 h ahead and acts on it | B | L-M |
| WF2 | Proactive Edge Caching with LSTM-based Popularity Prediction | Demirci, Korcak (per result) | venue not shown | https://avesis.marmara.edu.tr/yayin/11e96016-9db8-40b5-8889-e65ff4b98aa8/proactive-edge-caching-with-lstm-based-popularity-prediction | LSTM proactive caching beats benchmarks at small caches (author-reported) | B | L |
| WF3 | Applying Machine Learning Techniques for Caching in Edge Networks: A Comprehensive Survey | authors not shown | arXiv:2006.16864 | https://arxiv.org/pdf/2006.16864 | Survey including popularity prediction | B | L-M |
| WF4 | Feedforward Neural Networks for Caching: Enough or Too Much? | authors not shown | arXiv:1810.06930 | https://arxiv.org/pdf/1810.06930 | Argues prediction accuracy and cache performance can be weakly related | B | L-M |
No source found in this pass addresses *short-horizon origin-pressure* or *request-rate* forecasting for admission/shedding decisions in a cache; the results are popularity prediction for proactive placement. This is a coverage gap, not evidence of absence.

## 3. Counts (rows in section 2; grouped rows count once, item totals in parentheses)
Counted by hand from the Class column (section 8):
- **Class A (verified from a primary source): 35 rows** (CC1, CC2, CC4, CC5, CC6, CC7, CC9; HK1, HK2; EV1, EV2, EV4-EV14; FR1-FR4; OP1, OP3-OP10). Rows with a weaker half (CC7 original 2012 article = D; HK1 TODS pages, EV1 ToS details, OP5 numbers = B; OP7 authors = C) are counted under A, the weaker half is stated in the row.
- **Class B only (secondary / mirror): 11 rows** (CC3, CC8, CC10; HK3, HK4; EV3; OP2; WF1-WF4).
- **Class C (repo docs only, not re-checked): 6 grouped rows** (CC11, EV15, FR5, FR6, OP11, OP12) covering about **27 individual works**.
- **Class D (searched, not found): 0 whole rows; one sub-item**, the 2012 ACM Queue CoDel article inside CC7.
- Rows total: **52** (35 + 11 + 6). Individual works: 35 + 11 single-work rows plus about 27 works inside the C rows, roughly 73.

## 4. Search record (queries used, WebSearch mode `standard`, no `extended` runs)
1. Netflix concurrency-limits GitHub adaptive concurrency limits Vegas AIMD Gradient2
2. Envoy adaptive concurrency filter gradient controller documentation minRTT
3. TCP Vegas: new techniques for congestion detection and avoidance Brakmo Peterson SIGCOMM 1994
4. BBR: Congestion-Based Congestion Control Cardwell Cheng Gunn Yeganeh Jacobson ACM Queue 2016
5. Chiu Jain Analysis of the increase and decrease algorithms for congestion avoidance in computer networks 1989
6. Controlling Queue Delay Nichols Jacobson ACM Queue CoDel RFC 8289
7. Little's law "A Proof for the Queuing Formula: L = lambda W" Operations Research 1961
8. Space-Saving Efficient computation of frequent and top-k elements in data streams Metwally Agrawal El Abbadi ICDT 2005
9. Cormode Muthukrishnan An improved data stream summary: the count-min sketch and its applications J. Algorithms 2005
10. Misra Gries Finding repeated elements Science of Computer Programming 1982
11. TinyLFU: A Highly Efficient Cache Admission Policy Einziger Friedman Manes ACM Transactions on Storage 2017
12. ARC: A Self-Tuning, Low Overhead Replacement Cache Megiddo Modha FAST 2003 usenix
13. LIRS: an efficient low inter-reference recency set replacement policy Jiang Zhang SIGMETRICS 2002
14. SIEVE is Simpler than LRU: an Efficient Turn-Key Eviction Algorithm for Web Caches NSDI 2024 usenix
15. FIFO queues are all you need for cache eviction S3-FIFO SOSP 2023 Juncheng Yang
16. CACHEUS: Evolving Adaptive Caching FAST 2021 Rodriguez usenix (title in my query was wrong; result corrected it)
17. Driving Cache Replacement with ML-based LeCaR HotStorage 2018 Vietri usenix
18. Learning Relaxed Belady for Content Distribution Network Caching NSDI 2020 Song Berger
19. GL-Cache: Group-level learning for efficient and high-performance caching FAST 2023 Yang
20. LHD: Improving Cache Hit Rate by Maximizing Hit Density NSDI 2018 Beckmann Chen Cidon
21. Darwin: Flexible Learning-based CDN Caching SIGCOMM 2023 Chen Sharma Khan Sitaraman
22. RFC 9111 HTTP Caching Fielding Nottingham Reschke June 2022 stale-while-revalidate
23. RFC 5861 HTTP Cache-Control Extensions for Stale Content Nottingham 2010
24. Basu Vaze Towsley Adaptive TTL-based caching for content delivery arXiv 1704.04448 (author names in my query were partly wrong; arXiv id located the paper)
25. Varnish grace mode std.healthy documentation request coalescing waiting list
26. Optimal Probabilistic Cache Stampede Prevention Vattani Chierichetti Lowenstein VLDB 2015
27. Revisiting Cache Freshness for Emerging Real-Time Applications Mao Iyer Shenker Stoica HotNets 2024
28. Characterizing Metastable Faults and Failures Farahbakhsh Lu Alvisi Haeberlen Van Renesse arXiv 2606.00942
29. workload forecasting for cache request rate prediction LSTM CDN popularity prediction proactive caching survey
30. nginx proxy_cache_lock proxy_cache_use_stale updating documentation ngx_http_proxy_module
31. Caching with Delayed Hits Atre Sherry Wang Berger SIGCOMM 2020
32. RobinHood: Tail Latency Aware Caching dynamically reallocating from cache-rich to cache-poor OSDI 2018
33. Scaling Memcache at Facebook Nishtala NSDI 2013 leases thundering herd
34. Google SRE book Handling Overload client-side adaptive throttling accepts requests
35. Fail at Scale reliability in the face of rapid change Ben Maurer ACM Queue 2015 adaptive LIFO CoDel
36. Learning-Augmented Heuristics cache eviction S4-FIFO arXiv 2608.27975
37. hot key detection in distributed cache Redis heavy hitters sketch Space-Saving hot key mitigation paper
38. Fastly serve stale request collapsing stale-if-error documentation
39. AdaptSize orchestrating the content delivery network cache hit ratio Berger Sitaraman Harchol-Balter NSDI 2017
40. Hyperbolic caching: flexible caching for web applications Blankstein Sen Freedman USENIX ATC 2017
(40 queries.) Most queries were *known-item* searches built from the repo's citations, so they test whether a citation exists as stated; they are weak at discovering unknown prior art.

## 5. Corrections and observations against repo documents
- **Darwin**: first author J. Chen, selects admission "experts" - consistent with the repo's own earlier correction.
- **CACHEUS** title is "Learning Cache Replacement with CACHEUS" (FAST '21); the abbreviated form used in some notes is not the paper title.
- **Basu et al.** author list is Basu, Sundarrajan, Ghaderi, Shakkottai, Sitaraman; my query guessed otherwise and was corrected by the arXiv id.
- **RFC 9111 vs stale-while-revalidate**: SWR and SIE are RFC 5861 (Informational), not RFC 9111. Any document that cites 9111 for SWR/SIE should cite 5861.
- **LAH / S4-FIFO**: now seen on a USENIX OSDI '26 presentation page (previously only arXiv comments).
- **Rest of `novelty_snapshot_m6.md`** (not re-verified here): it states "RT-0 Closure Verdict: PASS ... COMPLETE" while `literature_review.md` section 4e says "RT-0 remains open" with named gaps (SOSP all years, 2023 venues, SIGCOMM/SIGMETRICS/VLDB/SIGMOD/etc., Labrinidis VLDB full texts, UNIT, Kolobov, Defcon, MIDAS), and the accountability report lists R-17 "close RT-0 or withdraw novelty claims" as open (section 9) and Definition-of-done level 3 as "Partly". The snapshot's matrix also lists features ("canary bucketing, provenance gating, auto-rollback", "AIMD paced background refresh", "strict RFC 9111 compliance") that the accountability report (11.1) says were never backed by code, and `literature_review.md` says PhantomCache is not RFC-9111-conformant. These are conflicts in the repository's records; this file does not resolve them and edits none of them.

## 6. RT-0 status (honest)
**RT-0 is OPEN.** This pass was a bibliographic verification of 40+ known items, not a systematic prior-art sweep. Coverage gaps (absence of results is not evidence of absence):
- No full text was opened; claims are abstract-level and author-reported.
- ACM Digital Library, IEEE Xplore, DBLP and Google Scholar were not queried directly; the search tool is US-only and indexes blogs better than proceedings.
- No systematic proceedings sweep (OSDI/NSDI/SOSP/ATC/FAST/EuroSys/SoCC/SIGCOMM/SIGMETRICS/VLDB/SIGMOD 2022-2026) was done in this pass; the repo's earlier sweeps are agent-reported and partial (`literature_review.md` 4d).
- Hot-key detection: only Space-Saving, Count-Min, Misra-Gries and vendor docs were confirmed. Cache-specific hot-key literature (for example switch- or proxy-level load-balancing work, per-key replication) was not found or searched by name.
- Concurrency limiting: Netflix repo and the Envoy docs confirm the techniques exist; gateway/CDN-specific adaptive origin limiting literature, bounded-queue studies and Little's-law sizing for caches were not searched.
- Adaptive TTL / forecasting: only Basu et al. and Mao et al. confirmed; origin-pressure forecasting literature was not found; patents unread.
- Vendor documentation beyond Envoy, Varnish and Fastly (nginx primary page, Cloudflare, CloudFront, ATS, Squid, HAProxy) was not opened.
- The original CoDel (ACM Queue 2012) and Breakwater/DAGOR/Rajomon/TopFull/Protego were not verified.

## 7. Technique -> established vs possibly novel in the PhantomCache context -> what a defensible novelty claim would require
**Nothing is claimed novel today.** The right-hand column states what would be needed before *anyone* writes a novelty sentence; it is not a promise that any of it is achievable. All PhantomCache-side evidence is currently negative or absent for adaptive control (M6 H1 NOT_SUPPORTED; AIMD controller behaved as a threshold switch; PIE not integrated) `[doc: m6_sealed_results.md; accountability report 11.2-11.3]`.

| Technique | Established (sources) | Possibly novel in PhantomCache context? | What a defensible novelty claim would require |
|---|---|---|---|
| Static origin concurrency cap | every proxy; Fail at Scale, SRE ch. 21 (CC8, CC9) | No | Not a novelty candidate |
| AIMD / delay-gradient adaptive concurrency limit | Chiu-Jain, Vegas, BBR, Netflix, Envoy (CC1-CC6) | No for the controller; at most the *placement* (cache-resident origin gate coupled to stale-serving under hard caps) | A pre-registered win over best enumerated static and static+bounded-queue on a load-dependent plant (E1-E3), confirmed on a capacity-limited T2 origin; a completed RT-0 including Envoy/Netflix-style limiters as baselines |
| Little's-law cap sizing as an operator advisor | Little 1961 (CC10); capacity planning is classical | Weak: the packaging (predicted shed-versus-peak curve for a cache's origin gate) | Calibration study of predicted vs measured effect (E7); a literature sweep of capacity-planning and admission-control work; no claim for the formula itself |
| Bounded wait queue with timeout shed | CoDel, adaptive LIFO (CC7, CC8) | No | Only an engineering/evaluation claim after H-E2-Q |
| Hot-key / heavy-hitter detection | Space-Saving, Count-Min, Misra-Gries (HK1-HK3); vendor tools (HK4) | No | Accuracy-versus-memory evidence (E4) supports an engineering claim only; novelty would need a cache-specific prior-art sweep that has not been done |
| Frequency admission (TinyLFU-style) | TinyLFU, AdaptSize, Darwin (EV1, EV11, EV12) | No | Not a novelty candidate; `CountMinSketch` is unused in production anyway `[doc: CLAUDE.md section 6]` |
| Eviction (SIEVE, S3-FIFO, ARC, LIRS, LHD, Hyperbolic) and policy selection (LeCaR, CACHEUS, Darwin, SCION, LAH) | EV2-EV14, EV15 | No; crowded (candidate comparison already demotes it) | A TTL-aware shadow selector beating the best fixed policy (E5) with a verified novelty sweep including LAH/SCION; `[doc]` eviction named SIEVE/S3-FIFO/LRU in the repo are scans, not these algorithms |
| Learned eviction/admission (LRB, GL-Cache, LAH) | EV8, EV9, EV14 | No | The repo's own gate: a non-ML win first, then a five-part ML gate |
| Request coalescing / single-flight | Varnish, Fastly, Memcache leases, delayed hits (OP1, OP3, OP5, OP6) | No | Not a novelty candidate |
| Stale-while-revalidate / stale-if-error | RFC 5861 (FR2); vendor docs (OP1, OP3) | No | Not a novelty candidate |
| Adaptive TTL / freshness | Basu et al., Mao et al., Akamai patents (FR3-FR5) | Only the *conjunction* "measured origin health drives stale/refresh/admission under staleness and origin-load caps" is argued in the repo as unsolved, with weak evidence `[doc: literature_review.md 4.2, 4e]` | Positive pre-registered results (currently negative); closed RT-0; head-to-head with Varnish healthy-grace and DynamoDB-style constant-work refresh (a prior agent's baseline list); explicit contrast with Mao et al. and RobinHood. A patent reading by counsel if any commercial claim is intended |
| Origin-pressure / request-rate forecasting | popularity forecasting (WF1-WF4); none found for short-horizon origin load | Unknown; coverage gap | MASE below persistence on held-out scenarios (E6), downstream utility, and a real literature sweep |
| Generation-fenced invalidation, replay, flush-on-gap | Memcache leases, TAO, Redis tracking (repo doc, `literature_review.md` 3) | No (re-derivation by the repo's own assessment) | Formal model, measured staleness bound under injected loss, comparison with lease/versioned approaches |
| Deterministic pre-registered cache/origin lab with a retained negative result | libCacheSim and simulators exist (repo doc); pre-registration is a method | Artifact value only | A claim of novelty is not needed; value is reproducibility and honest negative results |

## 8. Reproducing the counts
The authoritative tally is the Class column of section 2 (52 rows; per-subsection: 2.1 has 7 A / 3 B / 1 C, 2.2 has 2 A / 2 B, 2.3 has 13 A / 1 B / 1 C, 2.4 has 4 A / 2 C, 2.5 has 9 A / 1 B / 2 C, 2.6 has 4 B). If a row's class changes, update section 3.

## 9. What was and was not done
Created by a read-only literature pass plus reading of repo documents named in the brief. No source or existing document was modified. No novelty, performance or superiority claim is made or implied for PhantomCache. Items marked C were not re-verified and must not be cited as verified; items marked B should be opened at the primary source before citation.
