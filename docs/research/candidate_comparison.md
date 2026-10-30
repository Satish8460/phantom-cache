# Candidate Research Contributions — Comparison

**Status: PROVISIONAL, updated after literature round 1.** Table scores were set before verification; the "Post-verification adjustments" section below supersedes them where they differ. Citations are verified in `literature_review.md`; "[LIT?]" tags in the table are the original unverified recollections.

## Post-verification adjustments (authoritative)
- **A/B (origin-aware miss management): novelty 3, survives, narrowly.** Nearest work: RobinHood (OSDI'18, backend-latency-driven cache *allocation*), Varnish `std.healthy` grace (binary, operator-written), Memcache leases/Gutter (static). No integrated feedback controller with a staleness/origin-load contract found; search was not systematic.
- **C (delayed hits): novelty 1–2, risk 5.** SIGCOMM'20, ATC'22, INFOCOM'25, NSDI'26 (RFB/LBU). Use MAD/RFB-style policies as baselines.
- **D (safe policy selection): novelty 1–2, risk 5.** SCION (2026), LAH/S4-FIFO (OSDI'26, with a worst-case +0.8% vs FIFO result), Darwin, CACHEUS, ACME. Drop as thesis.
- **E (cost-aware eviction): novelty 1.** GDS/GDSF, Hyperbolic.
- **F (invalidation): novelty 1–2.** Fencing, replay, flush-on-gap are all in Memcache/TAO/Redis. Becomes publishable only with a formal model (MuCache-style), a measured staleness bound (PBS/Polaris-style) under injected loss, and a flush-metastability analysis. Strong as an *assurance* workstream.
- **G (lab): unchanged; high multiplicative value.**
- **I (new): metastable-recovery study.** Cold-restart, retry-storm and slow-origin scenarios against a cache-side contract-bounded controller and tuned static baselines. This is the evaluation framing of A/B, motivated directly by HotOS'21 ("systematic approach … remains an open problem").

Scale: 1 (poor) – 5 (strong). *Cost* and *Risk*: 5 = expensive / risky.

| # | Candidate | Problem | Closest prior work [LIT?] | Novelty | Importance | Depth | Cost | Eval feasibility | Risk |
|---|---|---|---|---|---|---|---|---|---|
| A | **Origin-feedback miss management**: controller sets stale-grace, early refresh, coalescing deadlines and miss admission from measured origin latency/error/queue signals, with hard safety caps | Origin overload and tail latency during degradation; hit ratio alone ignores it | RFC 5861 SWR/SIE, XFetch, circuit breakers, AIMD concurrency limits, DAGOR/Breakwater overload control (none cache-state-aware, as I recall) | 3 | 5 | 4 | 3 | 4 | 3 |
| B | **Cache-state-aware load shedding**: when the origin is saturated, shed *misses* (not hits), prioritize by key value/cost | Overload control treats requests uniformly; a cache knows which requests will hit | Overload-control literature (uniform or priority-based) | 3 | 4 | 3 | 2 | 5 | 2 |
| C | **Delayed-hit-aware eviction/coalescing** | Misses on in-flight keys cost less than fresh misses; eviction should account for it | "Caching with delayed hits" (SIGCOMM'20 [LIT?]) and follow-ups | 2 | 3 | 4 | 3 | 4 | 4 (probably already done) |
| D | **Safe policy selection by ghost-cache shadow evaluation** with a non-regression guard | Switching policies safely | LeCaR, CACHEUS, ARC, miniature simulations, Darwin [LIT?] | 2 | 3 | 3 | 4 | 4 | 4 (crowded) |
| E | **Cost-aware eviction with measured per-key fetch cost** | Misses differ in cost | GDSF, Hyperbolic, LHD, AdaptSize [LIT?] | 1–2 | 3 | 2 | 2 | 5 | 3 |
| F | **Bounded-staleness invalidation, quantified & model-checked** | Stale reads after missed invalidations | Leases, Meta cache consistency work, PBS [LIT?] | 2 | 4 | 4 | 4 | 3 | 3 |
| G | **Reproducible deterministic cache/origin lab** (virtual clock, replay, fault injection) as an artifact | Fair, repeatable evaluation of origin-aware caching | libCacheSim (offline replay), `turmoil`/`madsim` [LIT?] | 2 (artifact value) | 4 | 2 | 3 | 5 | 1 |
| H | **Learned origin-aware policy (RL/bandit)** instead of hand-built control | Same as A | Learned caches (LRB, Darwin) fail under shift [LIT?] | 3 | 3 | 5 | 5 | 2 | 5 |

## Observations
1. **A and B are the same research thread.** B (shed misses, protect hits) is the cheapest, most testable piece; A is the larger controller. Together: "origin-aware miss management".
2. **C is the most dangerous overlap.** Request coalescing *is* the delayed-hit phenomenon. If SIGCOMM'20-era work already models it, any claim about coalescing-aware eviction must be framed as an extension, not a discovery.
3. **D and E are well-trodden.** Use them as baselines and ablations, not as headline contributions.
4. **F is an engineering/assurance contribution** unless a new bound is found. Valuable for credibility and for the correctness workstream, weak as the paper's thesis.
5. **G is not novel but makes everything else credible.** Its value is multiplicative.
6. **H is high-risk.** Learned policies need training data and have documented distribution-shift fragility [LIT?]. Choose it only if a simple controller measurably fails.

## Smallest defensible original contribution (provisional)
> A cache-resident controller that uses measured origin health to jointly bound (i) extra origin concurrency, (ii) staleness, and (iii) miss admission, and that provably never exceeds user-set hard caps, evaluated against SWR + XFetch + an AIMD concurrency limiter + circuit breaker on origin-degradation workloads.

It would be original **only if** the literature shows nobody has jointly evaluated these mechanisms against origin saturation. The agents' findings decide this.

## Realistic guarantees
- **Hard (by construction, testable):** maximum staleness ≤ configured cap; origin concurrency ≤ configured cap; controller outputs clamped to ranges; memory bounded.
- **Empirical only:** lower origin peak load / p99 than baselines; non-oscillation under tested workloads.
- **Not achievable without extra assumptions:** a formal "never worse than the best fixed policy" guarantee for arbitrary workloads. A fallback-with-guard design can bound the *damage window*, not eliminate it.

## What would change this ranking
- If the literature shows A/B already exist: pivot to F + G (assurance-and-artifact paper) or to a negative-result/measurement study of when origin-awareness helps.
- If a tuned static SWR + concurrency limiter matches the controller (see falsifiers): the honest contribution is the negative result plus the lab.
