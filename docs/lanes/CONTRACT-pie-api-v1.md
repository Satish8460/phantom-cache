# CONTRACT pie-api-v1 (PIE v2 HTTP API) - written by lane L3, 2026-10-31

Status: STABLE for the demo programme. L4 serves these JSON bodies, L2 renders them. L3 provides one library function per endpoint (module `phantom_cache_demo::intelligence_v2::api`) that returns `serde_json::Value`; L4 only adds the HTTP envelope. Additive changes only; a breaking change needs a new schema string (`pie-api-v2`).

## 0. Conventions
- Schema string: every body has `"schema": "pie-api-v1"` and `"status": "ok"`.
- Provenance label `src` is one of `measured | derived | simulated | unavailable`.
  - `measured`: a raw counter or reading taken in this process (queue drops, mode, event counts).
  - `derived`: computed from measured events by a declared formula (rates, ratios, estimates, shadow results).
  - `simulated`: only in Lab runs (`lab_mode`); the live gateway never emits it as a result.
  - `unavailable`: no value. Representation is ALWAYS `{"value": null, "src": "unavailable", "samples": <int>, "reason": "<snake_case_code>"}`.
- Value object `V` (used wherever a number or label is shown): `{"value": <json>, "src": "<label>", "samples": <int>}` plus `"reason"` only when `src = unavailable`, plus optional `"unit"` and `"note"`.
- Static or declared values (registry text, config) are NOT labelled measured: they sit in objects named `config` or in registry fields listed in `doc_fields`, which are documentation values.
- Times are integer milliseconds on the clock the caller supplies (`timestamp_ms`; L4 passes a monotonic process clock). Sizes are bytes. Rates are per second unless a `unit` says otherwise. Ratios are 0..1.
- Key privacy: no request key, tenant id or secret is ever emitted. Keys appear only as 16-hex-digit hashes.
- Errors (L4 maps to HTTP): unknown policy -> HTTP 404 `{"status":"error","error":"unknown_policy","policy_id":"..."}`; unknown decision id -> 404 `{"status":"error","error":"unknown_decision"}`; bad query -> 400 `{"status":"error","error":"bad_request","detail":"..."}`. If PIE v2 is not installed in the process: HTTP 200 `{"status":"ok","schema":"pie-api-v1","installed":false,"src":"unavailable","reason":"intelligence_v2_not_installed"}` for every endpoint.
- Security: same Observer-role rule as the existing intelligence endpoints (L4 applies `require_observer_if_secure`).

## 1. Mode semantics
Only `off`, `observe`, `advisory` exist in v2. `canary` and `active` are rejected at parse time (`parse_mode` returns an error) and cannot be represented.
- `off`: nothing is enqueued, no window, no decision. Serving path cost: one relaxed atomic load.
- `observe`: events are enqueued and windowed into features (F-ids below). No policy runs, no decision records except `Fault`.
- `advisory`: observe plus shadow policies (P1, P2, P3, P5, P6) run and write decision records. Nothing is ever applied; every recommendation carries `"applied": false`.
- A fault (panic, stall, poisoned input, clock jump) sets the mode to `off`, writes a `Fault` record, and never affects serving. There is no runtime upgrade path; only a downgrade (`off` is final until restart).
- The existing `intelligence_mode` field of `/status` stays as it is today (strings `Off|Observe|Advisory`); L4 sets it from the v2 mode when v2 is installed.

## 2. GET /api/v1/intelligence/registry
```
{ "status":"ok","schema":"pie-api-v1","installed":true,
  "registry_version":"1.0.0","max_mode":"advisory",
  "process_mode": V,                       // value "off|observe|advisory", src measured
  "doc_fields":["objective","trigger","expected_effect","evaluation_method","limitations","lifecycle","safety_constraints"],
  "policies":[ {
     "policy_id":"POL-CLASS-1","name":"...",
     "kind":"WorkloadClassification|Eviction|Admission|StampedeWarning|OriginSaturationWarning|RefreshAdvice|DistributedImbalanceAdvice",
     "implemented": true,
     "objective":"...","trigger":"...",
     "required_inputs":[{"feature":"F2","min_samples":100,"allowed_src":["measured","derived"]}],
     "parameters":[{"name":"p2.delta_min","default":0.02,"min":0.0,"max":0.5,"unit":"hit_ratio_abs","current":0.02}],
     "safety_constraints":["..."],"expected_effect":"...","evaluation_method":"...","limitations":"...",
     "version":"1.0.0",
     "lifecycle":"Proposed|Implemented|UnitTested|Integrated|ShadowEvaluated|Retained|Narrowed|Rejected",
     "max_runtime_state":"off|observe|advisory",
     "effective_runtime_state": V          // min(max_runtime_state, process_mode), src measured
  } ] }
```
Policies in v1: POL-CLASS-1 (P1 workload classification), POL-EVICT-1 (P2 eviction shadow selection), POL-ADMIT-1 (P3 frequency-aware admission advice), POL-STAMPEDE-1 (P5), POL-ORIGIN-1 (P6) are `implemented:true`. POL-TTL-1 (P4) and POL-DIST-1 (P7) are listed `implemented:false`, lifecycle `Proposed`, `max_runtime_state:"off"`. The arbiter (P8) is infrastructure and appears in `/status`, not as a policy.

## 3. GET /api/v1/intelligence/status  (add under key `"v2"`; keep all existing fields)
Library returns the `v2` object:
```
{ "schema":"pie-api-v1","installed":true,"registry_version":"1.0.0",
  "mode": V,                         // measured
  "max_mode":"advisory",
  "fault": null | {"decision_id":n,"timestamp_ms":n,"reason_code":"panic|stall|clock_jump|poisoned_input|missing_provenance","detail":"..."},
  "queue": {"capacity":n,"enqueued_total":V,"dropped_total":V,"depth":V},
  "window": {"window_ms":1000,"decision_every_windows":10,"windows_closed":V,"last_window_degraded":V,"last_tick_ms":V},
  "memory": {"bytes": V,             // src derived: sum of declared capacities (upper bound), NOT an allocator measurement
             "budget_bytes": n, "breakdown":{"queue":n,"window_state":n,"shadow_caches":n,"decision_ring":n}},
  "decisions": {"recorded_total":V,"retained":V,"ring_capacity":n,"ring_bytes":V,"suppressed_by_cooldown":V,"collapsed_repeat_noaction":V},
  "config": {"lab_mode":false,"cache_capacity_entries":1024,"shadow_sample_modulus":16,"incumbent_policy":"SIEVE","declared":true},
  "claims": {"benefit_claim":"none","note":"M6 H1 remains NOT_SUPPORTED; advisory output is a shadow estimate, never an applied change"} }
```

## 4. GET /api/v1/intelligence/signals
```
{ "status":"ok","schema":"pie-api-v1","installed":true,"mode":V,
  "window": null | {"index":n,"start_ms":n,"end_ms":n,"duration_ms":n,"events":n,"dropped_events":n,
                    "degraded":bool,"degraded_reason":null|"events_dropped|drain_truncated"},
  "features": { "F1": {"name":"request_rate_rps","unit":"req/s", ...V}, "F2": {...}, ... },   // keyed by feature id
  "top_keys": {"src":"derived","items":[{"key_hash":"00ab...","count":n}]},                   // max 10, hashes only
  "classification": V,                // P1 label (value is a string) or unavailable (mode observe, not enough samples)
  "history":[{"index":n,"end_ms":n,"events":n,"degraded":bool}]                               // last 8 windows, oldest first
}
```
Feature ids: F1 request_rate_rps, F2 hit_ratio, F3 top1_share, F3b top10_share, F4 zipf_alpha, F5 working_set_keys_est, F6 burst_ratio, F7 hot_key_emergence (bool), F8 locality (always unavailable: `not_implemented`), F9 miss_amplification, F10 origin_p99_ms, F10b origin_error_rate, F11 invalidations_per_min, F12 stale_served_ratio, F13 waiters_per_origin_fetch, F15 primary_load_imbalance (always unavailable: `needs_multi_node_view`), F16 peer_read_share, F17 one_hit_wonder_ratio. A feature with fewer than `min_feature_samples` (100) samples, or in a degraded window, is `unavailable` with a reason (`insufficient_samples`, `window_degraded`, `no_window_closed_yet`, ...). Values are never extrapolated.

## 5. GET /api/v1/intelligence/decisions?limit=&offset=&policy_id=
`limit` 1..=200 (default 50), `offset` >= 0, ordering oldest first (same as `/audit`).
```
{ "status":"ok","schema":"pie-api-v1","installed":true,"total_recorded":n,"retained":n,"limit":n,"offset":n,"returned":n,
  "ordering":"oldest_first","records":[ DECISION, ... ] }
```
DECISION (v2):
```
{ "decision_id":n,"timestamp_ms":n,"registry_version":"1.0.0","policy_id":"POL-EVICT-1","policy_version":"1.0.0",
  "mode":"advisory","verdict":"Recommend|NoAction|Observation|Fault","reason_code":"snake_case",
  "inputs":[{"feature":"F2","value":0.41,"src":"derived","samples":512,"window_ms":1000}],
  "candidates":[{"candidate_id":"S3-FIFO","status":"selected|rejected",
                 "rejection_reason":null,   // or "below_margin|dominated|trigger_not_met|constraint|conflict|insufficient_samples|provenance_refused"
                 "estimated_effect":null    // or {"metric":"hit_ratio_delta","value":0.05,"ci95":[0.02,0.08],"src":"derived","basis":"shadow_sampled_stream"}
               }],
  "constraints":[{"name":"stale_cap_unchanged","value":"not_touched","holds":true}],
  "recommendation": null,   // or {"text":"...","structured":{...},"applied":false,"label":"advisory, not applied"}
  "guardrail_events":["provenance_refused:F10"],
  "conflict":{"lever":"cache_replacement|origin_pressure|none","role":"primary|alternative|none","with":["POL-ADMIT-1"]},
  "observed_effect":{"value":null,"src":"unavailable","samples":0,"reason":"not_implemented"},
  "simulated": false,
  "explanation":"deterministic template text" }
```
`Fault` records have `policy_id:"PIE-RUNTIME"`, empty candidates, and `reason_code` in `panic|stall|clock_jump|poisoned_input|missing_provenance`. Repeated identical `NoAction` results for a policy are collapsed (counted in `collapsed_repeat_noaction`), so the ring holds changes, not heartbeats.

## 6. GET /api/v1/intelligence/policy/{id}
```
{ "status":"ok","schema":"pie-api-v1","installed":true,
  "policy": { ...registry entry as in section 2... },
  "runtime": {"last_evaluated_ms": V, "last_verdict": V, "last_reason_code": V, "evaluations": V, "recommendations": V,
              "suppressed_by_cooldown": V, "warning_active": V},
  "recent_decisions":[ DECISION, ... ] }          // up to 20, oldest first
```
Also available (optional route `/decisions/{id}`): `{"status":"ok","schema":"pie-api-v1","record": DECISION}`.

## 7. What the API does NOT say
No benefit claim: M6 H1 stays NOT_SUPPORTED. Advisory estimates come from shadow caches on a sampled key stream; they are not verified on the serving path. `observed_effect` is always `unavailable` in this version.

## 8. Additive extension: insight layer (hot keys, frequency, cardinality, prediction, flash crowd, size and cost)
Added 2026-10-31. Purely additive: no existing field changed meaning. Clients that ignore unknown fields keep working.

### 8.1 New features in `/signals` `features` (same V shape, `unavailable` below min samples, never extrapolated)
F18 key_share_seen_once, F19 key_share_seen_twice, F20 key_share_seen_3plus (shares of distinct sampled keys; sum is 1), F21 top1pct_request_share (needs 100 sampled distinct keys), F22 top10pct_request_share, F23 distinct_keys_est (HyperLogLog p=10), F23b distinct_keys_std_err (3.25 percent of the estimate, keys), F24 distinct_ratio (distinct / requests, capped at 1), F25 distinct_growth (relative to the previous valid window, `no_previous_window` otherwise), F26 high_cardinality (bool, ratio >= `c.high_ratio`), F27 mean_object_size_bytes, F28 one_hit_bytes_share, F29 one_hit_origin_cost_share (F27 to F29 are `unavailable` with reason `size_unknown` / `origin_cost_unknown` when the events carry 0), F30 hot_key_count. F18 to F22 and F27 to F29 come from the 1/16 spatial sample of distinct keys (min 30 sampled keys); a saturated sampler gives `sampler_saturated`.

### 8.2 New top-level blocks in `/signals`
```
"hot_keys": {"src":"derived","samples":n,"method":"space_saving_64_counters","hot_share_threshold":0.02,"note":"...",
   "items":[{"key_hash":"hex16","count_lower":n,"count_upper":n,"error_bound":n,"share":0.2,"confidence":0.98,"hot":true}]},   // max 10, hashes only
"prediction": {"method":"rules_plus_ewma_statistics_not_machine_learning","src":"derived",
   "predicted_hot_next_window":[{"key_hash":"hex16","predicted_share":0.1}],
   "scorecard":{"windows_scored":n,"min_windows":5,"precision":V,"recall":V,   // V is unavailable (reason insufficient_windows_scored) until min_windows
       "baseline_persistence":{"precision":V,"recall":V,"note":"..."}}},       // compare: is the predictor better than persistence?
"flash_crowd": {"detected_in_last_window":bool,"src":"derived","confidence":0.9,"confidence_kind":"heuristic_rule_score",
   "rate_ratio":x,"rate_cusum":x,"rate_fired":bool,"top10_share":x,"concentration_baseline":x,"concentration_cusum":x,
   "concentration_fired":bool,"thresholds":{"rate_h":1.5,"rate_slack":0.5,"conc_h":0.3,"conc_slack":0.05}}
```
`count_lower <= true count <= count_upper` (Space-Saving). `confidence` of a hot key is lower/upper, not a probability. The flash-crowd confidence is a declared rule score (0.6 one signal, 0.9 both), not a calibrated probability.

### 8.3 New decision records (`/decisions`)
- `POL-FLASH-1`: `verdict:"Recommend"`, `reason_code` in `rate_and_concentration_change|rate_change|concentration_change`, `recommendation.applied:false`, text like "Consider pre-warming these hot keys and a longer stale window: ... Nothing was applied.", `recommendation.structured` = `{kind:"flash_crowd_advisory", confidence, confidence_kind, rate_ratio, rate_cusum, top10_share, concentration_*, hot_keys:[{key_hash, share}]}`. Written at the window close where it fires (not only at the decision tick), repeats suppressed by the cooldown. Only in advisory mode.
- `POL-ADMIT-1` (P3): `recommendation.structured` gains `one_hit_bytes_share`, `one_hit_origin_cost_share`, `mean_object_size_bytes`, `high_origin_cost`, `weighting` (`size_and_cost_considered` | `size_unknown_count_only`); new `reason_code` `one_hit_bytes_share_low` (one-hit keys hold too few bytes to pollute memory); `guardrail_events` may list `weight_unavailable:F27:size_unknown`.

### 8.4 Registry additions (`/registry`, `/policy/{id}`)
Five new entries, all `implemented:true`, `lifecycle:"UnitTested"` (never Integrated or Evaluated here): `POL-HOT-1` (HotKeyDetection, max observe), `POL-FREQ-1` (FrequencyObservation, max observe), `POL-CARD-1` (CardinalityEstimation, max observe), `POL-PRED-1` (HotKeyPrediction, max observe), `POL-FLASH-1` (FlashCrowdDetection, max advisory). The detectors compute in observe and advisory modes; only POL-FLASH-1 writes decision records. `POL-TTL-1` stays `implemented:false` (no origin-change signal exists to learn from). New parameters: `h.hot_share`, `c.high_ratio`, `pr.min_windows`, `fc.rate_slack`, `fc.rate_h`, `fc.conc_slack`, `fc.conc_h`, `p3.one_hit_bytes_min`, `p3.origin_cost_warn_us`.

### 8.5 Event input (library API, not HTTP)
`Event` gains `size_bytes: u32` (response body size; 0 = unknown). `origin_latency_us` must carry the real origin fetch latency (0 = unknown or no fetch). With 0 the dependent features are `unavailable`; nothing is estimated.
