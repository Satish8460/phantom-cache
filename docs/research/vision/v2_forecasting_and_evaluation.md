# V2 - Forecasting Architecture and Evaluation Plan (Demand Calendar + bounded warming)

**Status: PROPOSAL / RESEARCH DESIGN. Not approved, not implemented, nothing run.** Author: product-discovery agent V2 of 5, 2026-10-30. Read-only pass; no source, UI, cargo, git, browser or experiment activity. This file is the only file written. Every statement about performance is a hypothesis; no number below is a measurement.
Scope: forecasting architecture, decision layer, multi-timescale loop, evaluation plan. The broad opportunity map and prior-art survey belong to V1 and are not repeated here.

## 0. Conventions

**Element classes** (used in every table): **E** = Existing (in the repo today, cite file:line); **PX** = Proposed extension (new code or schema, needs owner approval per CLAUDE.md section 2); **RH** = Research hypothesis (may be false; has a kill criterion); **PC** = Prototype candidate (cheap enough to build to test an RH, still needs approval); **U** = Unsupported (no evidence or no data path today; do not claim).

**Evidence tags:** `[code f:l]` read in this pass; `[doc]` stated in a repo document, not re-derived; `[lit-V]` literature item whose bibliographic record and abstract-level claim I matched to a primary or publisher page via WebSearch this pass (full text NOT opened); `[lit-NV]` literature or textbook knowledge NOT verified this pass; `[assumption]`; `[estimate]` arithmetic from stated assumptions, not a measurement.

**Tier labels** (CLAUDE.md section 4): T1 = `SIMULATED` (deterministic virtual time driving the real cache/executor code, `src/lab/mod.rs:1-9`); T2 = `MEASURED_LOOPBACK` (real gateway process on loopback, real time, `src/lab_live/scenario_b.rs:22`). A third label is needed for this work, **`SYNTHETIC_FORECAST_HARNESS`** (count-series forecasting experiments with no cache in the loop); it is a kind of `SIMULATED` and must carry that label too. There is no `MEASURED` real-workload tier available: `W-TR` (trace) is explicitly UNAVAILABLE (`src/lab/t1_sim.rs:438`).

**Inputs read:** CLAUDE.md sections 4-6b; `docs/research/phantom_intelligence_phase.md`; `intelligence_architecture.md`; `intelligence_e2_headroom_protocol_DRAFT.md`; `intelligence_prior_art_coverage.md`; `hypothesis_and_experiments.md`; `phantomcache_milestone_accountability_report.md` sections 11.4-11.6; code: `src/intelligence/{candidates,signals,mod}.rs`, `src/lab/{workloads,rng,analysis,manifest,t1_sim}.rs` (targeted ranges), `src/lab_live/{loadgen,origin,scenario_b}.rs` (struct level). Not read line by line: `executor.rs`, `shadow.rs`, `engine.rs`, `eval.rs`, `matrix_runner.rs`, `m6_pipeline.rs`, `src/lab_live/gateway.rs`.

## 1. Ground truth: what exists that the forecasting design can stand on

| Fact | Evidence | Consequence for this design |
|---|---|---|
| A rate forecaster exists as a lab candidate: EWMA vs last-value persistence, with a MASE helper (denominator = in-sample one-step naive MAE). It stores unbounded `Vec`s of predictions/actuals, has no seasonality, no intervals. | `candidates.rs:175-247` [code] | Class **E** for "EWMA + MASE vs persistence"; its unbounded history violates the explicit-bounds rule (CLAUDE.md section 5), so it is a lab toy, not a production forecaster. Its MASE is non-seasonal; with seasonal series the denominator must be the seasonal-naive error (see 5.3). |
| Hot-key emergence predictor is a velocity rule with hard-coded 10/15 count thresholds. | `candidates.rs:251-270` [code] | **E**, no calibration, no uncertainty. Baseline only. |
| Heavy hitters: Count-Min 4x1024 `u32` (16 KiB) + 64-entry Space-Saving list, linear scans, **count-based** halving every 10,000 samples. | `signals.rs:83-85, 94-191` [code] | **E**. Decay is per sample, not per unit time, so the effective memory horizon shrinks as load rises; a forecaster needs time-based decay (PX). CM bound: width w=1024, depth d=4 gives additive overestimate <= (e/w)N ~ 0.27% of N with probability >= 1-e^-4 ~ 98.2% [lit-V: Cormode-Muthukrishnan, `intelligence_prior_art_coverage.md` HK2; the (e/w, e^-d) parametrisation is standard CM theory, [lit-NV] for the exact constants]. |
| CUSUM burst detector, working-set (sampled), Zipf-alpha estimator, `WorkloadPipeline::produce_window` producing a `SignalWindow` with per-field provenance. | `signals.rs:256, 309-350, 382, 455`; `intelligence/mod.rs:40-115` [code] | **E** signals. Provenance enum (`Measured/Derived/Simulated/Unavailable`) exists; **no `Declared` value** (PX, 2). |
| Intelligence is a library; production `pie: None`; legacy `pie.rs` is a stub; the gateway writes no request log. | CLAUDE.md 6b; accountability report 11.4 [doc] | Everything here is offline/advisory-first. No forecasting claim can rest on gateway data today (**U**). |
| `PolicyKind` has `Eviction, Admission, RefreshEarly, StaleServing, ConcurrencyShedding` - **no warming/prefetch kind**; no proactive-warm API in `src/` (grep for prefetch/prewarm/refresh-ahead finds only SWR background refresh and a peer-handshake `prewarm_peer`). | `intelligence/mod.rs:117-123`; `cache.rs:165-174, 291, 353`; `dist/router.rs:2011` [code] | A warm actuator is **PX**. `max_background_refresh` (default 128) is an existing bounded knob for SWR refreshes only. |
| T1 dispatch already has a "no client leader" path used for background refresh: `leader_dispatch(leader: None, ...)`, through the same breaker, `try_acquire_origin_permit` and cap check. | `t1_sim.rs:821-860` [code] | A simulated warm fetch can reuse this path, so it is gated by construction (good for hard-cap evaluation). |
| T1 has no tenant dimension (no `tenant` identifier in `t1_sim.rs`); a single 400-key catalog, 250 req/s, TTL 3 s, warm-up 8 s + 20 s window, fixed 1024 B bodies. | `t1_sim.rs:278-320`; `workloads.rs:20-27` (`RequestSpec`: `body_size_bytes`, `ttl_ms: 60_000` constants) [code] | Tenant quotas, size skew and per-object freshness cannot be evaluated in T1 without PX. `ObjectSizeDistribution` exists (`workloads.rs:381`) but wiring into the fetch path was not checked here. |
| Workload generators: Zipf, scan, hot-set shift, flash crowd (fixed 2x multiplier, 8 keys, 75% concentration, step in/out), diurnal (sinusoid; T1 uses period = measure_us/2 = 10 s), invalidation, hot-key emergence (exponential ramp), mixture. | `workloads.rs:75-499`; `t1_sim.rs:408-440` [code] | **E**. All are single-scale and parametric; none has a calendar, weekly structure, drifting phase, declared metadata, or a non-materialising event. They are components for the planted-signal generators (6.4). |
| T2 `Workload`: stationary Zipf over `n_keys`, Poisson or Regular arrivals, one rate, deterministic schedule independent of arm. | `loadgen.rs:14-60` [code] | T2 cannot express time-varying demand or key groups today (PX). The arm-independent schedule property is exactly what a paired design needs. |
| Paired statistics: BCa mean CI, sign-flip permutation test, Holm adjustment; plus a `SyntheticAnalysisValidator` with planted +8%/-8%/null effects on the *analysis pipeline*. | `analysis.rs:322, 365, 405, 196-280` [code] | **E** and reusable. The planted validator tests the statistics, not the system; 6.5 extends the idea to the system. |
| Seed machinery: named independent streams (`MasterRng::stream`), tuning/dev sets, sealed derivation from an external salt via SHA-256 commitment, burned ranges, leak check. | `rng.rs:54-80, 129-200` [code] | **E**. Reusable pattern; new domain string required. |
| E2 draft (protocol v3) already specifies a headroom-first logic, utility ordering, statistics, T3/V3/S3 seeds, freeze/marker discipline, and owner-generated offline salt. | `intelligence_e2_headroom_protocol_DRAFT.md` sections 1, 6, 8, 10 [doc] | This design deliberately copies its structure (6.7) and does not duplicate it. It concerns origin-concurrency caps, not demand forecasting. |

## 2. Demand Calendar

### 2.1 Purpose and trust model
An operator (or an integration) declares *expected future demand* that history cannot know: a campaign, a launch, a scheduled report run, a batch export, a product drop. The calendar is **untrusted, bounded, authorised, expiring input** that can only (a) raise or lower a forecast prior and (b) *permit* bounded actions. It can never raise a hard cap, extend staleness beyond `PHANTOM_MAX_STALE_MS`, or bypass the origin gate. Every value derived from it is labelled `DECLARED` (new provenance value) and a decision resting on it must say so. Class: schema **PX**; usefulness of declared demand **RH-D1** (see 6.7, F2/F3).

### 2.2 Schema (proposal; field names illustrative)
```
DemandEntry {
  entry_id            : string (ULID)            # immutable
  schema_version      : u16
  state               : draft | approved | active | expired | revoked | quarantined
  tenant              : TenantId                 # exactly one; an entry cannot reference another tenant
  selector            : one of
        { kind: "explicit",  keys:    [string]        , max 1,000 }
      | { kind: "prefix",    prefix:  string          , tenant-scoped }
      | { kind: "tag_set",   tags:    [string]        , any|all }
      | { kind: "group_ref", group_id: string }        # operator-named key group
      selector_limits : { max_matched_keys, max_matched_bytes }   # entry-level hard bound; matching is capped, never unbounded enumeration
  window              : { start, end (UTC), tz_label, recurrence: none | RRULE-subset(daily|weekly|monthly, count<=N),
                          ramp: step | linear(ramp_s) }
  expected_demand     : { form: multiplier_on_baseline | extra_rps | share_of_tenant_traffic,
                          point, lo, hi, interval_level (e.g. 0.8),   # operator's own uncertainty; lo<=point<=hi required
                          p_materialise: optional prior in (0,1] }    # "how sure am I this happens"
  object_hints        : { expected_size_bytes?, expected_unique_keys? }   # optional; advisory
  actions_permitted   : { warm: bool, admission_prep: bool, refresh_ahead: bool, capacity_advice: bool,
                          max_lead_time_s, per_entry_budget: { bytes, origin_fetches } }  # budgets can only be <= global/tenant caps
  source              : operator_ui | api_integration | imported_schedule | inferred_by_system
  authorization       : { principal, role, approval_ref, approved_at, approver != author (optional policy) }
  expiry              : { hard_expires_at, auto_revoke_after_n_failed_materialisations }
  provenance_label    : "DECLARED"                 # constant; never "MEASURED"
  audit               : { created_at, created_by, last_modified_by, change_log_hash }
}
```
Calendar-level bounds (all explicit counts/bytes, as CLAUDE.md section 5 requires): max entries per tenant and global, max selector bytes, max recurrence expansion horizon, max matched keys per entry, max total declared warm bytes per day per tenant. Entries are validated at write time and re-validated at activation; a failed validation moves the entry to `quarantined`, never silently to `active`.

Security/abuse cases the schema must survive (each becomes a test, 6.4 G5/G9): inflated multiplier, huge selector, self-renewing recurrence, entry spamming, cross-tenant selector, replayed approval, clock/timezone confusion (DST), entry edited after approval (hash mismatch -> quarantine), a trusted-but-wrong operator (fat finger: 100x multiplier).

### 2.3 How declared demand combines with history

Notation: forecast target is the request count (or rate) `Y_g(t)` of a **demand unit** `g` = tenant aggregate, key group, or a heavy-hitter key, over a horizon step `h`. Components:

1. **Baseline from history:** `B_g(t+h) = L_g * S_g^day(t+h) * S_g^week(t+h) * S_g^month(t+h)` (multiplicative seasonality; level `L_g`, seasonal indices from the seasonal model). Daily/weekly/monthly components are enabled only when the series has enough history (>= 2 full cycles is the usual minimum rule of thumb [lit-NV]); otherwise the component is `Unavailable` and the forecast says so. Weekly/monthly seasonality are **U** today (no history store; a gateway restart loses state, no persistence design exists).
2. **Short-term trend / level shift:** Holt-style trend or a CUSUM-triggered level reset (`BurstDetector`, `signals.rs:309`) so that a regime change is not smoothed into the seasonal profile.
3. **Object freshness context:** forecast is of *requests*, but value of a warm depends on freshness: the warm entry is only useful if it is still fresh (or within the stale cap) when requests arrive. Inputs: object TTL as effectively configured (default lifetime is a literal 60 s because `PHANTOM_DEFAULT_TTL_MS` is not wired, CLAUDE.md section 5), invalidation/churn rate per group (`observe_invalidation`, `signals.rs:447`), time since last origin validation.
4. **Origin conditions:** measured origin p99, error rate, gate in-flight/cap (`f10_*` signals, `intelligence/mod.rs` SignalWindow; `executor.rs:65-130` snapshot) - used as **gates and cost multipliers**, not as demand predictors (v1: do not forecast origin health to decide demand).
5. **Declared overlay:** `D_g(t)`: multiplier or additive rate with interval `[lo,hi]`, window, ramp.

**Combination rule (proposal, RH-F1).** Treat the declared overlay as a *prior* on the multiplicative uplift `u` of baseline demand in the window, and update it with observed counts as the window unfolds:
- Observed counts in an elapsed sub-window `Delta`: `n ~ Poisson(u * B * Delta)` (overdispersion handled by a negative-binomial variant with a dispersion estimated from history residuals).
- Prior on `u`: Gamma with mean `m_decl` (the operator's point) and variance set by moment-matching the operator's `[lo,hi]` interval, then **inflated by a source-reliability discount** `rho_s in (0,1]`: effective prior strength `kappa = rho_s * kappa_nominal`. `rho_s` is learned per source/operator from the *track record of materialised vs declared* (a Beta posterior on "event materialised within declared interval", starting from a weak, pre-registered prior, e.g. Beta(1,1)).
- Posterior mean (Gamma-Poisson conjugacy [lit-NV, textbook]): `E[u | n] = (alpha0 + n) / (beta0 + B*Delta)`. Declared information dominates *before* the window (no data), observed information dominates after enough counts. This is the "simple Bayesian shrinkage of declared vs observed demand".
- **Non-materialisation:** use a two-component prior `pi * Gamma(decl) + (1-pi) * Gamma(point mass near 1)` where `pi = p_materialise` (operator's) shrunk toward the source's historical rate. Posterior mixture weights update by the marginal likelihood of observed counts (closed form for Gamma-Poisson). Early in the window (little data) the decision rests on `pi` - which is exactly why the budget (4.2) must cap the downside of acting on a prior that never materialises.
- Output is a **predictive distribution** (or its quantiles) for `Y_g` over each future step, not a point.

Honest caveat: before onset, *there is no data to correct a wrong declaration*; only the track record `rho_s` and the hard budgets protect. This asymmetry (cheap upside, bounded downside) is the entire business case for bounded warming and is what experiments G3/G5 must test.

### 2.4 Candidate forecasters, cheap to expensive

State sizes are `[estimate]` for 4-8 byte numbers; none measured. "Series" = one demand unit. Budget rule: the intelligence arena is capped at <= 16 MB (`intelligence_architecture.md` section 3 [doc]), so per-series cost must be multiplied by an explicitly capped number of series.

| # | Forecaster | State per series | Update CPU | Gives intervals? | Needs | Class / role |
|---|---|---|---|---|---|---|
| F0 | Persistence (last value) | 8 B | O(1) | no | - | E (`candidates.rs:206`), mandatory baseline |
| F1 | Seasonal-naive (value one season ago; season = day or week, at a resolution e.g. hourly) | m slots x 4 B (m=24 -> 96 B; m=168 -> 672 B) | O(1) | via residual ring | >= 1 season | PX baseline; the benchmark that MASE must beat for seasonal series |
| F2 | EWMA / simple exponential smoothing | 8-16 B | O(1) | via residual ring | - | E (`candidates.rs:175`, unbounded Vecs) -> PX bounded rewrite |
| F3 | Holt (level+trend), damped optionally | 24 B | O(1) | via residual ring | - | PX |
| F4 | Holt-Winters (additive/multiplicative seasonal; one or two seasons) | (m+2) x 8 B; m=24: ~208 B; m=168: ~1.4 KB | O(1) per step (O(m) only at init) | via residual ring | >= 2 seasons for init | PX; classical method [lit-NV: Winters 1960, not searched] |
| F5 | Time-decayed counters / count-min with *time-based* decay for per-key rates | CM 16 KiB per sketch (existing shape) + decay timestamp; per-key `(count, last_ts)` 12 B for tracked heavy hitters | O(d) per event (d=4) | no (point) | - | E structure (`signals.rs:94`), PX time decay |
| F6 | Quantile forecasts: empirical residual quantiles per seasonal slot (fixed ring of N residuals, P^2 or sorted-ring), optionally scaled | 64-256 residuals x 4 B = 0.25-1 KB per series (x slots if slot-specific) | O(log N) or O(1) amortised | **yes** | F1-F4 point forecaster | PX; recommended default interval engine |
| F7 | Split-conformal / adaptive conformal intervals around any point forecaster (ACI adjusts the miscoverage level online by a one-parameter gradient step; target is long-run coverage under shift) | residual ring + 1 float | O(1)-O(log N) | **yes, with long-run coverage target, not conditional guarantee** | F0-F4 | PX / RH-F4. ACI paper: Gibbs and Candes, NeurIPS 2021, arXiv:2106.00170 [lit-V, abstract level]. Marginal/long-run coverage only; coverage during a flash crowd is NOT guaranteed. |
| F8 | Bayesian shrinkage of declared vs observed demand (Gamma-Poisson / mixture, 2.3) | 2 floats + mixture weight + per-source Beta (2 floats) per entry; ~100-200 B per active entry | O(1) | **yes** (closed-form predictive) | calendar entry | RH-F1; PC (small, testable) |
| F9 | Intermittent-demand estimators for sparse per-key series (Croston / SBA type) | ~24 B per tracked key | O(1) | weak | long tails of sparse keys | [lit-NV: Croston 1972, Syntetos-Boylan 2005 - not searched]; PC only for tracked keys |
| F10 | Small online learners (online ridge/linear on lags + calendar-feature vector of dimension d ~ 10-20, pinball-loss SGD for quantiles) | O(d^2) ~ 1-3 KB (ridge) or O(d) (SGD) | O(d^2) / O(d) | yes via quantile heads | many series or features; a stable feature set | RH. Only for the few highest-value aggregate series (<= tens); gated by the ML gate (`phantom_intelligence_phase.md` section 6 [doc]): needs a non-ML win first. |
| F11 | Offline-trained gradient-boosted / deep sequence models | model artifact, MBs | inference O(trees) | yes | traces, training pipeline | **U** in the MVP; out of scope; ML gate applies. Note the M5 competition outcome that simple local statistical methods can remain competitive for tail/uncertainty on high-granularity data (abstract-level, [lit-V: M5 Uncertainty, IJF 38(4) 2022; organisers' summary, full text not opened]), consistent with the ordering above. |

**Granularity policy (bounds, not just costs).** Three levels: (a) tenant aggregate - one series per tenant, F4+F6 allowed; (b) key *group* (prefix/tag/calendar-selector) - at most `G_max` groups per tenant (proposal: 16-64), F2/F3 + F6, F4 only for the top few; (c) individual key - **only** for heavy hitters (<= 64-4096 per node, the Space-Saving/CM tracked set) and for explicitly declared keys, using F5/F2. Never per-key for the whole catalog. `[estimate]` worked example: 1,000 tenants x (1 aggregate F4 weekly-hourly 1.4 KB + 0.75 KB residual ring) ~ 2.2 MB; plus 16 groups/tenant x F2+ring ~ 16 x 0.3 KB x 1,000 ~ 4.8 MB; total ~ 7 MB, inside 16 MB only with these caps. 64 groups/tenant with F4 would be ~ 140 MB: not allowed. These are arithmetic on assumed sizes; E0 (overhead measurement) must confirm.

**Selection principle:** a heavier forecaster is admitted only if it beats the best cheaper one by a pre-registered margin on held-out regimes (same logic as the repo's ML gate and E5/E6 kill rules). Default expectation (RH): F1/F4 + F6/F7 + F8 is the ceiling worth building; F10 is a stretch.

### 2.5 Calibrated prediction intervals and "confidence"
- **Interval engines:** F6 (empirical residual quantiles by slot), F7 (conformal/ACI), parametric NB for short count windows. Report quantiles `{0.1, 0.5, 0.9, 0.95}`; at least one upper quantile is what warming decisions use (they are asymmetric: under-warming costs a miss storm, over-warming costs bytes/fetches).
- **"Confidence" is defined operationally, not as a score:** `confidence(a) = P_forecast(demand >= threshold that makes action a worthwhile)` read from the predictive distribution, and it is shown to operators only if the forecaster's **rolling empirical coverage** at that level is within a pre-registered tolerance (proposal: nominal 90% interval must cover 85-95% over the last N=30 matured predictions per class). If not, status = `UNCALIBRATED`: the forecast is displayed as `unavailable`/advisory-only and warming that depends on it is refused (4.3 R4).
- **Calibration monitoring online (PX):** per series class keep hit/miss of the nominal-q interval in a fixed ring; coverage and mean width ("sharpness") are exported as telemetry; PIT/reliability diagrams offline.
- **What cannot be promised:** conditional coverage during regime change; coverage for first-ever events (no residual history); coverage of `DECLARED`-prior forecasts before the source has a track record (cold start -> wide intervals by construction; ACI/conformal do not fix a wrong prior). State these in the UI as limits, not footnotes.

## 3. Multi-timescale loop (online vs offline)

| Scale | Online (in gateway, bounded, off hot path) | Offline / control plane | Maturity |
|---|---|---|---|
| **Seconds** (1-10 s) | Rate EWMA, CUSUM burst, heavy-hitter table, origin health, gate occupancy, warm-execution pacing, *stop conditions* (halt warming when origin degrades or caps near). Only guard/revert actions are autonomous at this scale. | - | Signals E (`signals.rs`); pacing/stop PX |
| **Minutes** | Hot-key emergence, short-horizon rate forecast (F2/F3), warm queue progress vs plan, within-window posterior update of declared uplift (F8), realised-vs-predicted interval check | - | F2/F3/F8 PX/PC |
| **Hourly** | Intraday seasonal slot update, calendar activation (entry -> `active` when `start - lead_time` is reached), warm plan computation for the next window (set, bytes, fetches, schedule), refresh-ahead staggering plan | Residual/coverage ledger roll-up | PX |
| **Daily** | Seasonal profile refresh, calendar expiry sweep, per-source reliability `rho_s` update from matured events, wasted-warm accounting, capacity recommendation (bytes, origin cap via Little's law, as in accountability report 11.5 [doc]) | Backtest of forecaster set on the last K days (rolling-origin, 6.2) | PX / RH |
| **Weekly** | Weekly seasonal index refresh | Forecaster re-selection by backtest vs pre-registered rule; hyper-parameter change via versioned config only (never auto-tuned on the serving path); report to owner; calendar hygiene (stale recurring entries) | RH |

Online/offline rule: online state is recursive and small (O(1) updates); anything involving model selection, hyper-parameters, or training is offline and promoted as a versioned artifact (consistent with `phantom_intelligence_phase.md` section 6 (2) [doc]). Persistence: forecaster state must survive restart or be explicitly re-warmed (cold-start -> `UNCALIBRATED` for one season); persistence design is **PX, not designed here**.

**Progression ladder** (extends `IntelligenceMode`, `intelligence/mod.rs:24`; modes shown in `intelligence_architecture.md` 2.1 as Off/Observe/Advisory/Canary/Active [doc]; a distinct `Shadow` step is added below):

| Stage | Behaviour | Exit evidence (pre-registered) | Demotion trigger |
|---|---|---|---|
| 0 Off | No collection | - | - |
| 1 Observe | Signals + forecasts computed, nothing shown as advice; coverage ledger accumulating | E0 overhead acceptable; >= 1 season of data for seasonal classes; calibration within tolerance on matured forecasts | any invariant violation, overhead breach |
| 2 Advisory | Operator sees "would warm X keys / Y bytes at T because ...", with intervals and DECLARED/MEASURED labels; no actuation | N matured events; advisory precision/recall (predicted-hot vs realised-hot) above pre-registered floor; operator can explain >= x% of recommendations (usability check, PI-8 [doc]) | calibration failure |
| 3 Shadow | Counterfactual computed on live traffic: "had we warmed, hits saved = ..., origin fetches spent = ..." from the arrival stream (no exposure) | Shadow-estimated benefit vs T1/T2 prediction within stated error; no harm projected | disagreement with later canary |
| 4 Canary | Warm only a key-hash bucket subset (<= 5%, stratified buckets B=1024 per `intelligence_architecture.md` Pillar 4; note 8 shards give 12.5% minimum granularity, `phantom_intelligence_phase.md` WP-33 [doc]) or one low-risk tenant; **interference caveat:** warmed bytes compete with the control arm for cache capacity, so bucket canaries are biased toward optimism unless a byte-quota partition is used (RH, to be tested in T1) | Pre-registered thresholds (6.7), zero hard-cap violations | any guardrail breach -> instant revert |
| 5 Active-bounded | Warming under full budgets and the refusal list, per-tenant opt-in | Continuous: regret governor (4.4) in force | governor trip -> Advisory |

For calendar entries recurring daily/weekly, the first occurrences run as stage 2-3 by policy even if the forecaster is promoted, because the entry's own track record (`rho_s`) is unknown.

## 4. Decision layer: bounded warming, admission preparation, capacity recommendations

### 4.1 Actions
| Action | What it does | Actuator today | Class |
|---|---|---|---|
| A1 Warm | Fetch predicted/declared keys *before* the window so the first request hits | None. Candidate implementations: (i) in-gateway low-priority fetch task through the normal fetch + origin-gate + fencing path (`finalize_fetch` stores only if generation/epochs unchanged, CLAUDE.md section 5); (ii) **out-of-process warmer issuing ordinary GETs** to the gateway (no gateway change; requests must be tagged so telemetry does not count them as organic demand) | PX (i), **PC (ii)** |
| A2 Admission preparation | Reserve/prioritise bytes for the expected hot set (e.g., raise protection of warmed entries, or lower admission threshold for declared keys); must respect byte accounting I1-I8 and never evict active/in-flight keys | Admission filter exists in lab (`candidates.rs:65`); `CountMinSketch` unused in prod (CLAUDE.md 6) | RH; weak evidence - E2 draft reports admission inert at low caps [doc] |
| A3 Refresh-ahead scheduling | Keep a declared hot set fresh through the window with staggered refresh (avoid synchronised expiry); XFetch-style probabilistic early refresh is the known technique (Vattani et al., PVLDB 2015 [lit-V via `intelligence_prior_art_coverage.md` OP4]) | `RefreshEarly` kind name exists (`mod.rs:120`), no actuator found | PX |
| A4 Capacity recommendation | Advisory text/number: needed bytes, shard/entries, origin cap (Little's-law sizing) for the forecast upper quantile | none; advisor concept in accountability report 11.5 [doc] | PX, advisory only, lowest risk |
| A5 Alert / no-op | "Declared event not seen", "forecast uncalibrated", "refused because ..." | audit store `audit.rs` [code] | E (storage) / PX (content) |

### 4.2 Budgets (all explicit counts/bytes; all enforced by the execution layer, not by the forecaster)
1. **Origin load budget:** warm fetches/s <= `min(beta * origin_headroom_est, hard_warm_rate_cap)`; every warm fetch must acquire an origin permit through the same gate as organic misses (`try_acquire_origin_permit`, `executor.rs:228`) at **lower priority** than organic misses (priority/preemption does not exist today: PX), and is dropped, not queued, when the gate is saturated or origin health is degraded. Cost of *holding* a warm set fresh is not one fetch but `K * D / TTL`: `[estimate]` K=1,000 keys, TTL=60 s (the effective default lifetime), D=1 h gives 60,000 origin fetches (16.7/s) before any user request; this cost model, not the forecast, decides whether warming a day ahead is sane (it is not: warm lead time is bounded by `min(max_lead_time, c * TTL)` unless refresh-ahead is also budgeted).
2. **Byte budget:** warm bytes in flight + resident warmed bytes <= `min(f_warm * cache_bytes, tenant_quota_remaining)`; warming may only insert into free space or displace entries that a shadow estimate says are colder than the warm candidate (otherwise it is a net loss). Never evicts a key with active ops/in-flight fetch.
3. **Tenant quotas:** per-tenant warm fetch and byte budgets; a tenant cannot spend another's; global sum <= global cap; noisy-neighbour test G9.
4. **Freshness / stale cap:** warming never changes staleness semantics: warmed entry gets TTL from the origin response at fetch time; `PHANTOM_MAX_STALE_MS` strictly enforced; `stale_cap_violations` must stay 0 (CLAUDE.md 5). Warming by serving stale-to-fresh is out; warm requires a fresh fetch. Invalidation racing a warm is handled by generation fencing (must be tested with warm fetches in the loop, since fencing was verified for organic fetches only).
5. **Spend-per-entry cap and self-throttle:** per `DemandEntry.actions_permitted.per_entry_budget`; plus the **regret governor** (4.4).

### 4.3 Explicit refusal conditions (each is a named, logged reason code; each gets a negative test)
R1 entry not `active`, expired, revoked, quarantined, or authorisation missing/invalid; R2 selector exceeds `max_matched_keys/bytes` or crosses tenant; R3 required input has provenance `Simulated`/`Unavailable` (input rule, `phantom_intelligence_phase.md` section 1 [doc]) - note `Declared` is a *permitted but labelled* provenance for the prior only, never for gating decisions; R4 forecast `UNCALIBRATED` or insufficient history; R5 origin degraded (error rate or p99 above threshold, or gate in-flight/cap above x%); R6 any stale-cap violation or invariant violation observed anywhere; R7 cache byte pressure above high-water such that the warm set would displace hotter entries; R8 tenant or global budget exhausted; R9 source reliability `rho_s` below floor *and* no corroborating observed evidence; R10 regret governor tripped (waste fraction above bound over trailing window); R11 insufficient lead time to complete within budget (do not start a warm that cannot finish); R12 mode Off / kill switch / watchdog tripped (`intelligence_architecture.md` section 4.1 [doc]); R13 object size unknown or above per-object max; R14 conflicting/overlapping entries unresolved; R15 clock anomaly (skew/jump/DST ambiguity in the window definition); R16 in cooldown after a revert.

### 4.4 Regret governor (RH-D2)
Track for each action class a trailing ledger: `wasted_warm_bytes` (warmed, never hit before expiry/eviction), `wasted_warm_fetches`, `hits_lost_to_warm_evictions`, `origin_cost_of_warm`. If waste fraction over the last N events exceeds a pre-registered bound, demote the source (`rho_s` down) and the action class (Active -> Advisory). Purpose: the system must be *unable* to keep wasting origin capacity on declarations that never materialise, even if the operator keeps making them.

### 4.5 Explanation record (extends `DecisionRecord`, `intelligence/mod.rs:150-163`, which today carries: id, timestamp, mode, trigger_reason, candidate, estimated_effect, confidence_interval, guardrails_passed, verdict, explanation, config hashes)
Proposed additional fields (PX), answering what / why / when / confidence / action / constraints:
- **what:** action kind (A1-A5), demand unit(s), #keys, bytes, expected fetches.
- **why:** forecast decomposition (baseline, seasonal components, trend, declared uplift, posterior weight of declared vs observed), contributing `entry_id`(s), source and its `rho_s`; inputs with provenance labels (`DECLARED`, `MEASURED`, `DERIVED`, `UNAVAILABLE`).
- **when:** scheduled start/finish, window start, lead time, expiry.
- **confidence:** predictive quantiles, the operationally-defined `confidence(a)`, current rolling coverage and calibration status for this class.
- **action/mode:** advisory / shadow / canary / active; exposure; alternatives considered and rejected with reason codes (R1-R16 or cost).
- **constraints:** each budget with used/limit/status; hard caps and their status.
- **after the fact (filled later):** realised demand vs interval, hits actually gained (or shadow estimate), waste, whether the entry "materialised" (feeds `rho_s`).
`AuditStore::explain_decision` (`audit.rs:79`) and `export_json` (`audit.rs:105`) are the existing hooks. UI rule: every displayed number carries measured/simulated/inferred/declared/unavailable (CLAUDE.md section 4).

## 5. Hypotheses carried by this design (each has a kill criterion in 6.8)
- **RH-F1:** declared+observed Bayesian shrinkage forecasts demand during declared windows better (pinball loss, MASE) than history-only and than calendar-literal, when declarations are informative; and does not do worse when they are not.
- **RH-F2:** bounded warming from calendar+forecast reduces miss-storm size / origin peak / p99 around events relative to the strongest reactive baseline at <= the stated waste.
- **RH-F3:** seasonal forecasters (F1/F4) beat persistence/EWMA on tenant aggregates with daily structure (the only mildly plausible claim; still needs held-out regimes).
- **RH-F4:** residual-quantile/ACI intervals stay calibrated under drift and flash crowds well enough to be shown.
- **RH-D1:** declared demand is useful *after* a source-reliability discount.
- **RH-D2:** a regret governor bounds waste from non-materialising declarations without suppressing true ones.
- **RH-H (headroom):** an oracle that knows future demand beats the best reactive/calendar-literal baseline by a material margin. **If false, everything above except calendar-literal warming and capacity advice is moot.** This is tested first (E9).
Likely honest outcome to plan for: the calendar-literal baseline (warm the declared set at start - lead, within budgets) captures most of the available gain and the forecaster adds little (a hypothesis about the result, not a result).

## 6. Evaluation plan

### 6.1 Questions and what answers them
| Q | Question | Experiment | Tier |
|---|---|---|---|
| Q0 | Is there headroom: oracle-warm vs best realistic baseline? | **E9** (6.8) | T1 |
| Q1 | Forecast skill: does each forecaster beat the correct naive baseline on held-out regimes? | **E10** (extends E6 [doc]) | SYNTHETIC_FORECAST_HARNESS |
| Q2 | Calendar value: calendar vs history-only vs calendar-literal under true/false/mixed declarations | **E11** | T1 (events) + harness |
| Q3 | Calibration: coverage/sharpness under drift, flash crowd, regime change | E10 | harness |
| Q4 | Safety: hard caps, refusals, tenant isolation, abuse | E11 + unit/property tests | T1; tests |
| Q5 | Does the actuation layer behave as simulated (warm vs TTL, gate interplay, overhead)? | **E12** | T2 MEASURED_LOOPBACK |
| Q6 | Overhead and memory bounds of forecasters | E0 (exists in plan [doc]) | T2 / microbench release |

### 6.2 Forecast evaluation protocol (E10)
Rolling-origin ("time-series cross-validation") evaluation: fit on `[0,t]`, forecast `t+1..t+H`, advance; horizons matched to the loop scales (e.g., 1 min, 1 h, 1 day, 1 week in the *compressed* clock of 6.4). Never random shuffling. Scoring: per-series then aggregate per seed; seeds are the units of analysis (cells within a seed are not independent, as in the E2 draft). Models listed and hyper-parameter grids are fixed in the pre-registration; hyper-parameters tuned **only** on the tuning seeds and, preferably, on generator regimes disjoint from the sealed regimes (6.5).

### 6.3 Baselines
**Forecast level:** F0 persistence; F1 seasonal-naive (the benchmark MASE must use for seasonal series); F2 EWMA; "same as last event" for recurring entries; operator-declared point taken literally (no history).
**Decision level (all under identical budgets and identical arrival traces per seed, paired):**
- **B0 static reactive:** current product behaviour (single-flight, TTL, SWR; no warming). Mandatory floor.
- **B1 strongest reactive eviction/admission:** LRU, SIEVE, S3-FIFO, W-TinyLFU-style. Caveat: product evictors with these names are scans (CLAUDE.md section 6); use the lab/shadow implementations (`lab/eviction_baselines.rs`, `intelligence/shadow.rs:39-310`) only after confirming they are faithful (not verified here). These test whether *eviction choice alone* removes the event cold-start - the repo's own hypothesis doc says eviction is not expected to be the lever [doc].
- **B2 prefetch-on-miss:** on a miss for key k, also fetch its prefix/tag siblings up to a budget (the naive "warmer" a practitioner would write). Also **constant-work refresh** (every hit triggers async refresh) from `hypothesis_and_experiments.md` baseline 14 [doc].
- **B3 calendar-literal ("cron warm"):** warm exactly the declared selector at `start - fixed_lead` with fixed rate, no forecasting. **The most important simple baseline**; its omission is how a forecaster gets credit for what a schedule does.
- **B4 history-only forecast-warm:** forecasts and warms with no calendar.
- **B5 popularity-top-K-of-yesterday:** static warm of the previous period's top-K within budget.
- **B6 random warm of equal budget:** control for "any warming helps a cold cache".
- **B7 oracle-warm:** knows the realised future intensity and key set (but not noise realisations unless stated), chooses the warm set under the same budgets. Defines headroom and regret.
- **B8 oracle-eviction (Belady-style) on the same trace** as an upper bound for the eviction axis [lit-NV: classical Belady MIN].
Not a baseline: any ML model (ML gate).

### 6.4 Synthetic generator designs ("Demand World")
Architecture: a **generator** that is separate from the system under test and from the forecaster code, producing (a) a ground-truth intensity `lambda_k(t)` per key group, (b) a realised request trace (non-homogeneous Poisson or negative-binomial thinning to produce overdispersion), and (c) **calendar entries via an independent "declaration process"** whose relation to the truth is controlled by explicit planted parameters. Implemented as `WorkloadGenerator`s (trait at `workloads.rs:28`) in a **new module** so M6/E2-frozen files stay untouched (the E2 draft's isolation argument, section 10 item 5 [doc]).

`lambda_k(t) = base_k * Season(t) * Trend(t) * Events(t) * Noise`. `Season` is drawn from a **profile library with several structural forms** (sinusoid; piecewise hour-of-day bumps; random low-order Fourier; weekday/weekend two-level; month-end spikes) so that no single forecaster family matches the generator by construction.

Time compression: the day/week/month hierarchy is mapped to virtual time by a factor `c`, preserving the **ratio season-length / TTL** as the invariant (production effective TTL 60 s vs day 86,400 s is a ratio of 1,440; T1 uses TTL 3 s so a "day" would be 4,320 virtual s). This is a stated simplification whose mapping to production is unverified. Cost note `[estimate/assumption]`: E2 draft estimated ~37.5 ms per 130 s T1 cell at 250 req/s; a 4,320 s "day" would be ~33x longer (~1.2 s/cell by linear extrapolation) and a 7-day week ~8 s/cell; therefore **forecast skill is evaluated at the count-series level (harness)**, and **T1 cells are event-centred windows** (e.g., 130-600 s around an event, with the seasonal state pre-aged from a harness-generated history), not full weeks.

| ID | World | Planted truth | Declaration process (independent of forecaster) | What it tests |
|---|---|---|---|---|
| G1 | Stationary Zipf, no events | no signal at all | none / decoys | **Null control:** system should do ~nothing; any "benefit" or large spend is a pipeline defect |
| G2 | Seasonal (daily+weekly, several shapes) with amplitude A, phase phi, noise dispersion | known season | none | forecaster skill, intervals; MASE vs seasonal-naive |
| G2d | G2 with **drift**: phase drift delta per season, amplitude change, level shift at changepoint, holiday outlier days | known drift schedule | none | stale seasonal profile; adaptivity; coverage under shift |
| G3 | **True declared flash crowd:** onset `t_e`, multiplier `M ~ LogNormal`, key set `K_true` (size, concentration, ramp) | `lambda` bump | declared `(m_d, [lo,hi], window, selector)` with planted errors: bias `eps = log(m_d/M)` in {0, +log 3, -log 3}; timing offset in {0, +-10%, +-50% of window}; selector precision/recall (Jaccard) in {1.0, 0.7, 0.4} | value when declarations are right and degradation under planted error; T-direction and magnitude sensitivity |
| G4 | **Undeclared (surprise) flash crowd** | bump | none | forecaster must not harm; reactive response parity |
| G5 | **Declared but never materialises** (fraction `q` of entries real, `q` in {0, 0.25, 0.5, 1}); includes adversarial variants: inflated multiplier x100, huge selector, repeated renewals | none for the fake entries | as declared | wasted warm bytes/fetches; learning of `rho_s`; regret-governor and budget containment; abuse resistance |
| G6 | **Wrong forecasts by mis-specification:** wrong season length, DST/holiday shift, trend break | known | none | graceful degradation; refusal on UNCALIBRATED |
| G7 | Catalogue drift: Zipf rank permutation rate, new-object arrivals, size skew (needs `ObjectSizeDistribution` wiring) | known | optional | warm-set staleness, byte accounting realism |
| G8 | Overlap with origin degradation: any `O-*` scenario (`t1_sim.rs:340-390`) coincides with a declared window | origin step | true declaration | warm must back off (R5); no origin-load harm vs B0 |
| G9 | Multi-tenant: one tenant declares a huge event; others steady | per-tenant | one tenant | quotas; other tenants' hit ratio unchanged (needs a tenant dimension in T1: PX) |
| G10 | Real trace replay | - | - | **U**: no trace in repo (`t1_sim.rs:438`); required before any claim beyond synthetic |

### 6.5 Avoiding circular evaluation (planted-signal design)
Circularity risks and countermeasures:
1. **Generator-forecaster identity.** If data are generated from a Holt-Winters-like process, HW wins by construction. Countermeasure: structurally diverse profile library (above); forecasters and generators in separate modules with no shared code; report robustness across *all* profile families, and results on families never used for tuning.
2. **Declaration process leaking into the forecaster's prior.** The shrinkage prior is parameterised by the operator's interval and `rho_s`; the declaration error model is independent. Test the prior both calibrated and miscalibrated.
3. **Tuning on the evaluation worlds.** Hyper-parameters and thresholds are chosen on tuning seeds *and* tuning regimes; sealed seeds use held-out regimes (new parameter ranges), mirroring the repo's held-out family idea (`t1_sim.rs:378-380`).
4. **Planted-signal end-to-end validation (positive/negative/harm controls).** Extend the `SyntheticAnalysisValidator` idea (`analysis.rs:196-280`, which plants +8%/-8%/0% on already-computed numbers) to the *system*: (a) positive control - a world with a planted known benefit (an oracle-warm yields a computable reduction in cold-miss origin fetches `= N_hot_keys_first_touch`); the pipeline must recover that quantity within tolerance; (b) null control G1/G5(q=0) - pipeline must report no benefit and bounded waste; (c) harm control - a world where warming is planted to hurt (cache just big enough that warm bytes evict hot entries) - the pipeline must detect harm. A pipeline failing any control is INVALID, as the E2 draft does for its validity checks.
5. **Self-referential metrics.** No metric uses the forecaster's own output as ground truth; "hot set" truth comes from generator intensity, not from the heavy-hitter sketch under test.
6. **Weak-baseline risk** (the repo's CAND-1/CAND-4 dispositions used a product-default and a straw-man baseline, accountability report section 9 [doc]): B3 and B7 are mandatory.
7. **Model-cache coupling.** Effects depend on TTL/capacity of the simulator; sensitivity grid over TTL, cache size, catalogue, event size, and T2 confirmation (E12). T1 results are never promoted without T2 agreement on sign and rank (analogous to E2 V-E1-4).

### 6.6 Metrics
- **Cache/client:** request hit ratio, byte hit ratio, fresh-hit ratio, time-to-first-hit after onset, **miss-storm size** (misses in the first W seconds of an event), p50/p95/p99/p99.9 for *successful* requests and for all requests (report both; all-request latency is survivorship-biased, `scenario_b_interpretation_notice.md` item 3 as cited in the E2 draft [doc]), goodput `G` (E2 definition `(fresh+miss_served+stale_served)/requests`).
- **Origin:** origin requests total and *attributable to warm*, peak concurrency, origin queue-wait p99 (not recorded in T1 today; E1/E2 add the column), shed count, error rate.
- **Waste/cost:** wasted warm bytes fraction, wasted warm fetch fraction, hits lost to warm-induced evictions, warm origin cost per saved miss.
- **Regret:** `regret = U(oracle B7) - U(policy)` on the primary utility, per event and per seed; also ex-post regret of each forecasted-warm decision (hindsight best action under same budgets).
- **Forecast accuracy:** MASE (scale = in-sample seasonal-naive MAE for seasonal series, one-step naive for non-seasonal; Hyndman and Koehler, IJF 22(4):679-688, 2006 [lit-V, abstract level; full text not opened]) - note `candidates.rs:220` uses only the non-seasonal form; **pinball loss** at tau in {0.1,0.5,0.9,0.95}, scaled by the baseline's pinball loss; interval score and CRPS from quantiles (Gneiting and Raftery, JASA 102(477):359-378, 2007 [lit-V, abstract level]); avoid sMAPE/MAPE (degenerate near zero counts) [lit-V in the sense that Hyndman-Koehler state many measures are degenerate in common situations; abstract level].
- **Calibration:** empirical coverage at 80/90/95%, coverage error, reliability/PIT histogram, mean interval width (sharpness), coverage conditional on regime (stationary / drift / event), time-to-recover coverage after a shift.
- **Detection/decision quality:** precision/recall of predicted-hot vs realised-hot key sets (top-k by true intensity), lead time of correct warm completion relative to onset, refusal precision (refused when it should have: planted-bad worlds) and refusal recall (did not refuse in good worlds).
- **Safety (hard gates, INVALID triggers):** `stale_cap_violations_*`, `cap_violations`, `bytes_audit_equal`, `invariant_violations` (existing fields in `CellSummary`, `t1_sim.rs:447-485`) all zero; tenant isolation violations zero.
- **Overhead:** CPU per update, resident bytes of intelligence state vs explicit caps, decision-loop CPU (E0 style, release build, CIs).
**Utility ordering** (lexicographic with tolerance bands, as the E2 draft argues against weighted sums): safety gates -> non-inferior goodput/hit ratio and success p99 -> origin peak/queue/miss-storm size -> waste. Thresholds: owner decisions.

### 6.7 Building on T1 Lab and T2 runner, and tier labelling

**T1 (SIMULATED):**
- Add Demand-World generators (6.4) as `WorkloadGenerator` implementations plus a time-varying-rate/key-group layer (PX; current generators are single-scale).
- Add a warm actuator that issues fetches through `leader_dispatch(None, ...)` (`t1_sim.rs:822`) with a priority class and the budgets of 4.2 (PX; touches `src/lab`, needs approval; ideally new module like `src/lab/demand/` to keep frozen M6/E2 files untouched).
- Add a tenant dimension for G9 (PX) or restrict G9 to unit-level tests.
- Add columns: warm fetches, warm bytes, wasted warm bytes, miss-storm size, origin queue-wait p99 (shared with E1).
- Cells labelled `tier=SIMULATED`; the harness outputs labelled `SIMULATED / SYNTHETIC_FORECAST_HARNESS`; outputs carry the generator regime id, compression factor `c`, seed label.
- Reuse `RunManifestV2`/`ExecutionTier` (`manifest.rs:20-75`) for provenance.

**T2 (MEASURED_LOOPBACK):** `scenario_b` already provides the pattern: real gateway process, scripted origin (`OriginScript`, `origin.rs:21-34`), open-loop load with arm-independent schedule (`loadgen.rs:40`), interleaved arms across repetitions. Needed: (i) a load generator with a rate schedule and key groups (PX), (ii) an **out-of-process warmer** (PC) so no gateway change is required for a first T2 test; (iii) origin script with an "event" phase. T2 tests only **minute-scale compressed events**: it validates warm-vs-TTL timing, origin-gate interplay, overhead, and agreement of sign/rank with T1. T2 cannot validate hourly/daily/weekly forecasting (real time too long) - say so in every T2 report. Loopback results are never mixed with T1 (CLAUDE.md section 4); WSL2/physical results separate.

**What cannot be tested with the tools at hand:** real seasonality, real operator behaviour (are declarations ever accurate?), real catalogue dynamics, multi-node effects, TLS/network.

### 6.8 Experiment sequence with kill/falsification gates (proposed ids E9-E12; E0-E8 per `phantomcache_milestone_accountability_report.md` 11.6 [doc])
Order is deliberately headroom-first, mirroring E2.

| Exp | Content | Proceed only if | Kill / falsification (PROPOSED thresholds; owner decisions) |
|---|---|---|---|
| **E9 Oracle-warm headroom** (T1; G3/G4 worlds; B7 vs best of {B0,B1,B2,B3,B5}) | Is any warming strategy worth building? | - | **K-H:** pooled oracle improvement over the best realistic baseline < pre-registered margin on both miss-storm size (propose < 10%) and primary utility, with BCa CI upper bound below it, in true-declaration worlds -> **abandon predictive warming**; keep capacity advice and calendar-literal at most |
| **E10 Forecast skill and calibration** (harness; G2, G2d, G6; F0-F8) | Do seasonal/quantile/Bayesian forecasters beat the right naive baseline on held-out regimes, with calibrated intervals? | E9 passes (for the decision use) - skill itself can be run in parallel as it is cheap | **K-F1:** no forecaster beats the best of {F0,F1,F2} by MASE < 1 - margin (propose 0.95) in >= 5/8 held-out regimes (cf. E6 kill MASE >= 1 [doc]) -> no forecaster ships; **K-F4:** nominal-90% interval coverage outside [85%, 95%] in held-out drift worlds -> intervals not displayed as confidence, `UNCALIBRATED` |
| **E11 Calendar value and robustness** (T1 event cells + harness; G3-G5, G8, G9) | Does calendar+forecast beat calendar-literal and history-only; safe under bad declarations? | E9 pass | **K-C1:** forecast-warm not non-inferior-better than **B3** by margin (propose 10% on primary utility) -> forecaster rejected, calendar-literal retained (most likely outcome, see 5); **K-C2:** in G5 (q<=0.5) wasted warm bytes fraction > bound (propose 50%) *with* governor active -> declared-only evidence warming rejected; **K-C3:** any hard-cap/stale-cap violation, or origin peak/p99 worse than B0 by > 5% in any scenario (echo F-c [doc]), or non-warmed tenant hit ratio down > 1 pp -> reject; **K-C4:** null-world (G1) spend > epsilon or "benefit" found -> pipeline INVALID |
| **E12 T2 actuation confirm** (MEASURED_LOOPBACK) | T1 conclusion on sign/rank survives real process | E9, E11 pass | **K-T:** sign or rank of top arms disagrees with T1 in > 25% of paired cells (cf. E1 kill [doc]) -> T1 conclusions not promoted; overhead over budget -> reject |

Disposition language follows the repo's Retain/Narrow/Reject (`candidates.rs:308-316`; `phantom_intelligence_phase.md` section 5 [doc]). Promotion thresholds to a serving path follow accountability report 11.6 [doc] (delta_min, BCa lower bound > 0 in >= 5/8, none adverse, T2 confirmation with >= 10 repetitions, overhead within budget, zero hard-cap violations, deterministic replay).

### 6.9 Pre-registration requirements
This would be a **new protocol** (provisional name "protocol v4"), not an amendment of v2.x or the v3 E2 draft, with: its own **domain string**, its own **salt**, and its own seed sets (tuning / validation / sealed), pairwise disjoint from T (10001..10060), D (20001..20030), the burned range (30001..30030), the spent M6 sealed set, and from E2's *proposed* 40001..40030 and 50001..50020 (`rng.rs:129-200`; E2 draft section 6 [doc]). **No seed values, salt, commitment or freeze record are generated or proposed here.**
**Salt:** generated by the **owner**, offline, outside any agent session and outside agent-readable paths; only `SHA-256(salt)` is committed; the salt is supplied via an environment variable in an agent-free shell for the single sealed run; agents never see it (the E2 draft's O-3 recommendation [doc]; the v2 salt was created inside an agent session, a documented weakness).
Must be frozen *before* any sealed run: hypotheses and kill thresholds (6.8); the forecaster set and hyper-parameter grids; generator code and regime tables with the tuning/held-out partition; declaration-process error model; compression factor `c`; TTL/capacity grid; metric definitions including MASE scale choice and quantile levels; utility ordering and tolerances; unit of analysis (seed) and statistics (BCa 95% CI, sign-flip, Holm per `analysis.rs:322, 365, 405`; fixed bootstrap RNG seed); validity checks (determinism re-run byte-identical, completeness, positive/negative/harm controls, hard-invariant zeros); run-once marker and fail-closed freeze verification patterned on `eval.rs:531, 644, 703, 902` (`verify_freeze`, `verify_freeze_with_addendum`, `scan_for_seed_leak`, `create_run_once_marker`); release build and `rustc` version; git HEAD and dirty count; dated addendum mechanism for post-freeze changes. Estimating power: seed-level SD of effects is unknown -> run a validation stage first and fix the sealed seed count before the freeze (E2 draft section 8 rule: no data-dependent size change after freeze). Exploratory runs on tuning/dev seeds are labelled EXPLORATORY and cannot trigger a kill.
Disclosure items to carry forward: shape-level contamination (designers have seen M6 aggregate results); T1 time compression; Poisson/NB assumption of arrivals; synthetic declarations are authored by the researchers.

## 7. Element classification summary
| Element | Class |
|---|---|
| Rate EWMA + MASE-vs-persistence helper (unbounded Vecs, non-seasonal) | E (`candidates.rs:175-247`) |
| Hot-key velocity rule; CUSUM; heavy hitters (CM+SS); working set; Zipf-alpha; SignalWindow with provenance | E (`candidates.rs:251`; `signals.rs:94-350, 382-455`; `mod.rs:40-115`) |
| Audit store / DecisionRecord / explain_decision | E (`audit.rs`; `mod.rs:150`) |
| Origin permit gate and background-refresh limit; fenced fetch | E (`executor.rs:65-252`; `cache.rs:165-174, 291, 353`; CLAUDE.md 5) |
| T1 sim, workloads, seeds, BCa/sign-flip/Holm, manifests; T2 scenario runner | E (`lab/*`, `lab_live/*`) |
| `Declared` provenance value; Demand Calendar schema, validation, storage, authorisation | PX |
| Time-based decay counters; bounded seasonal and quantile forecasters F1-F4, F6 | PX |
| Warm actuator (in-gateway), priority class at origin gate, regret governor, refusal list R1-R16 | PX |
| Out-of-process warmer using ordinary GETs (no gateway change) | PC |
| Gamma-Poisson / mixture shrinkage of declared vs observed (F8) | PC + RH-F1 |
| Conformal/ACI intervals (F7); calibration monitor | RH-F4 / PX |
| Capacity recommendation (A4, Little's-law) | PX, advisory; low risk |
| Admission preparation (A2) | RH (weak) |
| Refresh-ahead with staggering (A3) | PX (technique known, XFetch [lit-V metadata]) |
| Demand-World generators G1-G9, harness, event-centred T1 cells, T2 event runner | PX |
| Online learners (F10) | RH, ML-gated |
| Offline deep/GBM forecasting (F11); multi-day/monthly seasonality on live data; real-trace validation (G10); any claim that forecasting improves hit ratio, latency or origin load | **U** |
| Persistence of forecaster state across restart | PX, not designed |

## 8. Literature check log (this pass; WebSearch `standard` mode; abstracts/records only, no full text opened)
| Item | Result |
|---|---|
| Hyndman and Koehler, "Another look at measures of forecast accuracy", IJF 22:679-688, 2006 (MASE) | `[lit-V]` bibliographic record and abstract-level claim matched (Monash repository, RePEc). Exact formulas not read. |
| Gibbs and Candes, "Adaptive Conformal Inference Under Distribution Shift", NeurIPS 2021, arXiv:2106.00170 | `[lit-V]` NeurIPS proceedings page; abstract-level: long-run coverage under shift via one-parameter online update. JMLR 2024 follow-up (arbitrary shifts) also returned. |
| Gneiting and Raftery, "Strictly Proper Scoring Rules, Prediction, and Estimation", JASA 102(477):359-378, 2007, DOI 10.1198/016214506000001437 (CRPS, interval score, quantile scoring) | `[lit-V]` records matched; theorems not read. |
| Makridakis et al., M5 Uncertainty competition, IJF 38(4):1365-1385, 2022; Accuracy competition IJF 2022 | `[lit-V]` records matched; takeaway that simple local statistical methods can be competitive for high-granularity data/tails is abstract-level from a companion record, not independently confirmed. |
| Count-Min (Cormode-Muthukrishnan 2005), Space-Saving, XFetch/Vattani 2015, TinyLFU, SIEVE, S3-FIFO | Verified in `intelligence_prior_art_coverage.md` (HK1, HK2, OP4, EV1, EV4, EV5) as bibliographic metadata by a prior pass; not re-searched here. |
| Winters 1960 (Holt-Winters), Croston 1972, Syntetos-Boylan 2005, Belady MIN, Gamma-Poisson conjugacy, rolling-origin evaluation, "2 seasons to initialise" rule | `[lit-NV]` standard knowledge, **not searched**; do not cite as verified. |
| Proactive caching / popularity forecasting (DeepCache, LSTM edge caching, surveys) | Only `[lit-NV]`/secondary entries in `intelligence_prior_art_coverage.md` WF1-WF4 (class B); coverage gap for short-horizon origin-load forecasting and for **operator-declared demand calendars in caches**. **No novelty claim is made.** Absence of results is not evidence of absence; related industry practice (cache warming/pre-purge, scheduled-event capacity planning, "pre-scaling" before known events) exists and was not surveyed here - a V1 matter. |

## 9. Threats to validity and honest limits
- All decision-level evidence would be synthetic; declaration quality in real operations is unknown (the single most important unknown for RH-D1).
- Time compression (TTL 3 s sim vs 60 s effective production default; day -> virtual seconds) is untested for transfer.
- Poisson/NB arrivals understate real burstiness and correlation; flash-crowd shapes are researcher-authored.
- Single-node, single-origin; peer-invalidation and cluster effects excluded.
- Bucket canaries interfere through shared capacity (3, stage 4); shadow estimates ignore origin side effects.
- No power calculation is possible before seed-level variance is observed.
- Overhead and memory numbers in 2.4 are arithmetic estimates and need E0.
- Passing tests asserting stub behaviour say nothing about forecast quality (CLAUDE.md section 3); the current `candidates.rs` tests (not read here) should not be cited as evidence.

## 10. Open owner questions (see also 11.6)
1. **Is operator-declared demand in scope for the MVP at all**, or should the first deliverable be capacity advice (A4) plus calendar-literal warming (B3) with forecasting deferred until E9/E10? (Recommendation: the latter.)
2. Approve the **headroom-first order** (E9 before building any forecaster) and its proposed kill margins (10% miss-storm, 5% harm, 1 pp neighbour, 50% waste, MASE margin 0.95)?
3. Who may author/approve calendar entries (roles, four-eyes policy), and what is the tenant model for quotas (the T1 lab has no tenants)?
4. Approve a new `Declared` provenance value and the rule that it may inform priors and advice but never gate hard-cap decisions?
5. Permit an **out-of-process warmer** prototype (no gateway change) for T2, versus an in-gateway warm actuator (touches the fenced fetch path, independent review required)?
6. Real data: can any real traffic trace or gateway request log be obtained (G10)? Without it, forecasting claims stay `SIMULATED`.
7. Persistence: must forecaster state and per-source reliability survive restarts (needs a storage decision)?
8. Seed/salt: confirm the owner generates the protocol-v4 salt offline (as proposed for E2), and choose seed ranges disjoint from all burned and E2-proposed ranges.
9. Code location: new module (e.g., `src/lab/demand/`) preserving M6/E2 frozen files, as E2's O-5 recommends?
10. Time-compression factor and the TTL to emulate (3 s lab vs 60 s effective default): fix by decision, or run a sensitivity grid?
11. Overhead and memory budgets for the intelligence arena (16 MB cap in the architecture doc is a design value, not measured).

## 11. Cross-cutting discoveries

Added after the owner clarification that scope limits duplication, not investigation. Inputs: `v3_topology_tenancy_identity.md` section 1 (tenancy), `v4_security_and_failure_scenarios.md` sections 5-6 (leakage, M3 defect), `v5_demo_center_ux_and_storyboard.md` (event taxonomy), plus four WebSearch lookups (snippet level; no full texts). Items are hypotheses unless tagged otherwise. No novelty claim is made anywhere in this section.

### 11.1 Real-code gaps that constrain sections 2-4 (taken from V3/V4 [doc], not re-verified by me)
| Gap | Consequence for this design |
|---|---|
| No per-tenant quotas; capacity, byte budget and eviction are global; one tenant can evict another's entries (V3 1.1 item 5) | The "tenant quota" budget in 4.2 item 3 is **not enforceable today**. Until per-tenant byte/entry/origin-permit accounting exists (PX, touches `cache.rs`, independent review), warming is only safe as **advisory** or under one global warm budget, and warm inserts can harm other tenants (G9 becomes a known-failing scenario to demonstrate). Per-tenant quotas are a **prerequisite** of active warming and arguably more valuable than any forecaster. |
| Telemetry is not tenant- or client-attributed; SSE is a 100 ms snapshot, not an event log; no request log (V3 1.1 item 7) | Per-tenant forecasting cannot be fed from current telemetry. Inputs need an Observe-mode per-tenant aggregate (counts per group per minute), not a per-request stream. V5's event taxonomy should include `tenant`, `group_id`, `entry_id` and `warm=true` tags. |
| Single origin per gateway, one global origin gate (V3 1.2) | Origin budgets are per gateway, not per tenant or per origin; multi-origin warming is out of scope. |
| Static cluster; peers address-only; M3 durability defect (a DELETE is lost if the issuer dies before the peer receives it; peer stays stale up to object lifetime) (V4 6.3) | **Do not warm across nodes**, and do not refill a key whose invalidation may have been lost. Forecasts and warms stay node-local until durable invalidation exists (V4 5.2 reaches the same conclusion). |
| Non-secure mode lets any client assert `x-tenant-id` (V3 1.1 item 2) | A Demand Calendar write path has integrity only in secure mode; refuse it (or label `UNAUTHENTICATED`) otherwise. |

### 11.2 Intelligence, security, tenancy and distribution reinforce (and conflict with) each other
1. **Forecast leakage is a design constraint (agrees with V4 5.2).** The existing heavy-hitter detector is one global Count-Min + Space-Saving over `key_hash` (`signals.rs:94-191`), not tenant-keyed. A shared model that warms or admits on cross-tenant popularity leaks "tenant A requested K" through hits/latency. Rule: per-tenant sketches and forecasters; no cross-tenant per-key statistics; hashing keys is not protection. Memory arithmetic `[estimate]`: the current sketch shape is 4x1024x4 B = 16 KiB, so one per tenant exhausts the 16 MB arena at 1,024 tenants. A per-tenant sketch must shrink (e.g. 4x256 `u16` = 2 KiB, ~2 MB per 1,000 tenants) at a larger additive error (e/256 ~ 1.06% of N [CM theory, [lit-V] via HK2 for the structure]), or tenants must be tiered (top-N tenants get sketches; the long tail gets only a coarse tenant-level rate series). A real accuracy/isolation trade-off to measure.
2. **Calibrated intervals double as anomaly detectors, enforcement stays deterministic (agrees with V4 5.1).** An observation far above a calibrated upper quantile is a principled alarm with a known false-alarm rate, instead of a fixed threshold. RH-S1: this gives fewer false positives on *declared* flash crowds than fixed thresholds. Corollary: the calendar is a security-sensitive object (authorisation, rate limits, audit, size bounds), because a compromised or mistaken entry can mask an attack or burn the origin; a `DECLARED` entry must never silence anomaly alerts above a size bound.
3. **Poisoned-signal / steering risk.** Traffic shaped by an attacker shapes that tenant's forecast and its source reliability. Mitigations to test: per-principal influence caps, forecasts clipped by hard budgets, warm spend drawn from the tenant's own budget. Add an adversarial-demand variant to G5 (attacker trains the forecaster to over-warm).
4. **Warm-state side channels (V4 5.2).** Warming that consumes a shared permit pool or evicts shared entries is observable by other tenants. Per-tenant permit accounting and eviction domains are required before active warming; this ties the forecasting roadmap to the tenancy roadmap.
5. **Credentials.** The gateway forwards no client identity by default (V4 5.2). Never warm a key whose origin response depends on caller identity; the out-of-process warmer is therefore limited to shared/unauthenticated content, a real limit on use cases.
6. **Distributed coordination.** With rendezvous routing each node sees only its shard of a tenant's demand. Options (RH): node-local forecasts (simple, no leakage); or gossip tenant-level *rates only* over the mTLS peer channel (needs a partial-failure and membership policy; membership is static/manual today). Default: node-local until E9 shows headroom.

### 11.3 Challenging my own assumptions
- **Is forecasting the right lever?** For campaigns, launches and scheduled jobs the information is known to a human or arrives with a deploy; no statistics are needed. An **event-driven bounded warm/refill API** (webhook from CMS/CI/purge, by tag or prefix, under the 4.2 budgets) is deterministic, auditable and testable, and is B3 (calendar-literal) generalised from a clock to an event. I now think this should be the first product-grade feature, with forecasting an optional layer that must beat it (K-C1 in 6.8).
- **Purge-with-refill (RH-X1).** A common source of cold misses in a cache with invalidation is the purge itself. Soft purge (mark stale, serve within the stale cap, revalidate) plus bounded refill of the purged heavy-hitter set targets an observable cause with no demand prediction. Whether a soft purge by tag/prefix exists in the HTTP API was not checked. Cheap to test in T1 with the invalidation workload (`workloads.rs:305`) as an added world G11.
- **Object change rate may be a better signal than demand for freshness.** Revalidation outcomes and invalidation history give per-group change rates; with demand they decide whether holding a set warm is worth `K*D/TTL` refresh cost (4.2). The repo's adaptive-TTL candidate was null (CAND-3 [doc]) but used no change-rate signal. RH only; lab generators have no object-change process (`RequestSpec.ttl_ms` is constant, `workloads.rs:20-27`).
- **Cost heterogeneity.** The value of a warm is P(hit before expiry) x origin cost of the miss. My utility ordering (6.6) ignores per-key cost; the lab already models heterogeneous origin cost (`O-HET`, `t1_sim.rs` scenario table; `origin_sim.rs:294`). Warming should prefer expensive keys.
- **Forecast error is the wrong primary metric for operators.** They care about incidents avoided. Keep MASE/pinball for diagnosis, but make event-level outcomes (miss-storm size, origin peak, time to recover) the primary evidence; accuracy and cache performance can be weakly related (WF4 [lit-NV, secondary, `intelligence_prior_art_coverage.md`]).
- **Time compression limits seasonality results.** Compression preserves the season/TTL ratio but not cache capacity versus daily working set. Treat T1 daily-pattern results as mechanism tests only.

### 11.4 Differentiation against established caches (what is verifiable and what is not)
| Fact | Source status | Implication |
|---|---|---|
| Fastly soft purge marks content stale instead of deleting, so it can be served while a fresh copy is fetched; enabled by a request header on URL or surrogate-key purges; purge-all cannot be soft; stale-while-revalidate also requires stale-if-error | `[lit-V]` Fastly documentation (soft purges) returned by WebSearch; snippet level | Soft purge by key is established; "purge without a miss storm" is not a differentiator by itself |
| Varnish: grace serves stale while refreshing; xkey soft purge keeps grace behaviour; after a soft purge the next hit may not occur before grace ends so no background fetch fires; post-purge warming is done by third-party scripts or cron | `[lit-NV]` community guide, mailing-list threads, add-on vendor pages; **no official Varnish page returned** | Warming outside the cache by script is common practice, consistent with the out-of-process warmer prototype. Not verified from primary docs |
| Many CDNs offer no prefetch from origin; people warm by sending requests | `[lit-NV]` a CDN-guide page and WordPress plugin pages; **no Cloudflare or CloudFront primary page returned** | Do not claim "CDNs cannot warm" without primary sources |
| Redis client tracking: default mode keeps per-client key tables (server memory cost); broadcast mode uses prefixes; tracking table bounded by `tracking-table-max-keys` | `[lit-V]` redis.io client-side caching reference, snippet level. "No per-tenant memory quota" is an absence in search results, not evidence | A per-tenant resource-isolation comparison with Redis is unverified, and PhantomCache does not have that feature yet |
| Request collapsing, stale serving, probabilistic early refresh | `[lit-V]` via `intelligence_prior_art_coverage.md` OP1, OP3, OP4 | Established; not differentiators |

Candidate differentiators, all hypotheses and none claimed novel: (a) labelled, pre-registered evidence with retained negative results (a process property; competitors' practices not surveyed); (b) declared demand as an authorised, expiring, audited object with a measured track record (I found no cache documentation describing this in four searches, which is absence of evidence only); (c) refusal logging, i.e. every automatic action records why it was not taken; (d) hard caps independent of intelligence (a testable safety property, not a market claim). A competitive claim needs the sweep V1 owns.

### 11.5 New architectures and use cases worth testing (ordered by cost of falsification)
1. **Event-driven bounded warm/refill API** for CMS/CI/deploy hooks. Developer value: a CI step "deploy, then refill the previous version's top-N keys within budget" with an origin-cost report. Tested by B3 in E9.
2. **Pre-flight what-if advisor** ("campaign at 10:00, +5x on `/promo/*`: expected origin peak, hit ratio, required budget?"). Runs the lab offline on declared parameters plus a generator calibrated to Observe-mode aggregates; output labelled `SIMULATED` with intervals; no actuation risk. Needs lab calibration from real aggregates (U today) and E7-style calibration against T2.
3. **Post-event report** (declared vs happened, what the cache did, waste, source-reliability update): an accountability loop for operators.
4. **Calendar dry-run** against recorded aggregates, to catch fat-finger multipliers before activation.
5. **Capacity recommendations** from forecast upper quantiles (advisory; needs E7 calibration before being called reliable).
6. **Anomaly alerts from calibrated intervals** (11.2 item 2), useful independent of warming.

### 11.6 What other workstreams should know
- **V1:** warming and forecasting are not the cheapest first feature; event-driven refill and purge-with-refill test the same actuator deterministically. Per-tenant quotas are a prerequisite for any active predictive action.
- **V3:** forecasts must be per-tenant and node-local; a "tenant protected from a neighbour's flood" demo of warming is impossible until quotas exist; a tenant-scoped Observe aggregate is a new requirement; calendar APIs need secure mode.
- **V4:** treat the calendar as a security-sensitive object; the poisoned-forecast and warm-state-oracle worlds (G5 adversarial, G9) are candidate security scenarios, labelled `SIMULATED`, likely failing on today's code.
- **V5:** a forecast band must always show its measured coverage and an `UNCALIBRATED` state; the "Launch Day" flash-crowd beat can honestly use calendar-literal (declared, not predicted) warming while the forecaster is unvalidated; events need `warm=true` and `entry_id` tags.
- **E2 / protocol owners:** headroom-first logic transfers; keep the demand protocol separate with its own owner-generated offline salt; E1's origin queue-wait column is reusable.
