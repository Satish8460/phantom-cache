# LANE L3 handover (round 2): PIE v2 library with hot keys, frequency, cardinality, prediction, flash crowd, size/cost weighting

Written 2026-10-31. Lane copy: `D:\ClaudeTemp\lanes\L3\`. HARD RULES kept: only Off/Observe/Advisory, nothing is applied to serving, no canary/active, hard caps untouched, no model training, no benefit claim; M6 H1 stays NOT_SUPPORTED.

## 1. What L4 must change (exact new Event fields)
`intelligence_v2::signals::Event` is now:
```rust
pub struct Event {
    pub key_hash: u64,
    pub latency_us: u32,
    pub origin_latency_us: u32, // REAL origin fetch latency in us; 0 = unknown / no fetch
    pub outcome: Outcome,
    pub origin: OriginResult,
    pub size_bytes: u32,        // NEW: response body size in bytes; 0 = unknown
}
```
`src/api/server.rs` `pie_record(...)` currently builds the Event with `origin_latency_us: 0`. In my lane copy I added `size_bytes: 0,` there only so it compiles (NOT a deliverable change to server.rs beyond that one line). L4 must pass the real body length and the real origin fetch duration (the miss path knows both). With 0, the dependent features (F27, F28, F29, P3 weighting) say `unavailable` and nothing is estimated.

## 2. Status per item
| # | Item | Status | Evidence |
|---|---|---|---|
| 1 | Hot-key detection: Space-Saving, 64 counters per window, hash only, lower/upper count, error bound, share, confidence (lower/upper), `hot` above `h.hot_share` (default 0.02), deterministic | DONE | `insight.rs`, `signals.rs` (`TopK::entries`); test `hot_key_tracker_reports_bounded_hash_only_hot_keys` (true count inside bounds, no key text leaks, unavailable below min) |
| 2 | Frequency observation, one-hit-wonder detection: F18/F19/F20 (keys seen once/twice/3+), F21/F22 (request share of hottest 1%/10% of distinct sampled keys) | DONE | test `frequency_summary_counts_once_twice_and_repeated_keys`; unavailable below 30 sampled keys (F21: 100) |
| 3 | High cardinality: HyperLogLog p=10 (exactly 1024 bytes of registers, deterministic hash), F23 estimate, F23b standard error (3.25 percent), F24 ratio, F25 growth vs previous window, F26 flag (`c.high_ratio`, default 0.5) | DONE | tests `hyperloglog_estimates_...` (500/5000/50000 keys within 13 percent = 4 sigma), `cardinality_features_flag_high_cardinality_and_growth` |
| 4 | Hot-key prediction (rules plus EWMA, stated as not machine learning) with online scorecard (precision, recall, windows scored, unavailable until `pr.min_windows`=5) and a persistence baseline for comparison | DONE | test `prediction_scorecard_is_unavailable_until_enough_windows_and_reports_failure_honestly` (steady key: 1.0/1.0; a key that moves every window: 0/0, reported as such) |
| 5 | Flash-crowd detection: CUSUM on rate ratio (F6) and on top-10 concentration (F3b); decision record POL-FLASH-1 with evidence, heuristic confidence (0.6 one signal, 0.9 both), advisory text, `applied:false`; written at window close, not only at the decision tick | DONE | test `flash_crowd_is_detected_with_evidence_and_an_advisory_that_applies_nothing` (constructed trace, seeded frozen `FlashCrowdWorkload` positive control; stationary Zipf and calm traces never fire; observe mode writes no record) |
| 6 | Admission advice weighs size and origin cost: Event `size_bytes`, F27 mean size, F28 one-hit bytes share, F29 one-hit origin-cost share; P3 returns `one_hit_bytes_share_low` when one-hit keys hold less than `p3.one_hit_bytes_min` (0.3) of sampled bytes, annotates the advice with bytes/cost shares and a high-origin-latency warning; unknown size/cost -> `unavailable` and count-only advice | DONE | test `p3_weighs_object_size_and_origin_cost_and_says_unavailable_when_unknown` (three cases) |
| 7 | Registry + contract + memory | DONE | five new registry entries (POL-HOT-1, POL-FREQ-1, POL-CARD-1, POL-PRED-1 max `observe`; POL-FLASH-1 max `advisory`), all `UnitTested`, none `Integrated`/`Evaluated`; contract section 8 added additively; memory estimate now includes the HLL (inside `Accum`), the per-key size/cost map, predictor tracks (max 128) and bigger snapshots; test `new_detectors_are_registered_honestly_and_their_state_is_bounded` |
| 8 | Out of scope, kept out | HELD | POL-TTL-1 stays `implemented:false`; no live admission/eviction change; no training |

## 3. Files changed or created (relative to lane root; the owner copies exactly these)
- NEW `src/intelligence_v2/insight.rs`, NEW `src/intelligence_v2/tests/insight_tests.rs`
- CHANGED `src/intelligence_v2/{mod.rs (one `pub mod insight;`), signals.rs, registry.rs, policies.rs, engine.rs, api.rs, tests.rs}`
- CHANGED outside my files, one line only: `src/api/server.rs` (`size_bytes: 0,` in `pie_record`, so the lane copy compiles; L4 replaces it with the real value)
- `D:\Personal\phantom-cache\docs\lanes\CONTRACT-pie-api-v1.md` (section 8 appended), this file
- No Cargo.toml / Cargo.lock change. Frozen files untouched. Existing test edits: `tests.rs` helpers got `size_bytes: 0`, and two count assertions changed because the registry legitimately grew (implemented policies 5 -> 10, registry entries 7 -> 12); every other existing assertion is unchanged and still passes.

## 4. Tests
Command (lane copy, cargo lock, own target dir): `cargo test --lib intelligence_v2 -j 1 --locked`.
Result (debug build, reproduced here): **37 passed, 0 failed** (28 existing + 9 new; 19 other lib tests filtered out).
Clippy: `cargo clippy --lib -j 1 --locked -- -D warnings` clean, and `cargo clippy --profile test --lib -j 1 --locked -- -D warnings` (covers my in-module tests) clean after one test-only fix. `--tests` over the whole repo is NOT clean, but only because of other people's file `tests/integration/cluster_prewarm_repair_tests.rs` (unused imports), not my code.

## 5. API additions (see contract section 8)
`signals_json` gains features F18..F30 and the blocks `hot_keys`, `prediction`, `flash_crowd`; `decisions_json` gains POL-FLASH-1 records and richer POL-ADMIT-1 structured fields; `registry_json` lists five more policies. Nothing existing changed meaning.

## 6. Limits (honest)
- Statistical rules only (Space-Saving, HLL, EWMA, CUSUM). No machine learning, no training. Thresholds are declared defaults, not pre-registered and not tuned; nothing here is an evaluation result and there is no benefit claim.
- Frequency and size/cost features use a 1/16 spatial sample of distinct keys inside ONE window: keys that repeat only across windows look like one-hit wonders; with fewer than 30 sampled keys (low traffic or short window) they are unavailable. Use a longer `window_ms` for light demos.
- Hot keys: 64 counters, so keys below roughly 1/64 of a window may be missed; reported counts are Space-Saving bounds; "confidence" is lower/upper bound, not a probability.
- HLL: about 3.25 percent standard error (1 sigma); poorer for very small counts; per window only.
- The predictor can only be as good as persistence plus a trend term; the scorecard compares both and the user should read it before believing the predictor. Predictions are not scored across thin or degraded windows.
- Flash-crowd confidence is a rule score (0.6/0.9), the rate baseline needs two earlier windows, and a long crowd is absorbed into the baseline after a few windows (the EWMA adapts), so only the onset fires.
- P3 byte weighting is key-weighted over sampled keys with known size, not request-weighted; origin cost share covers only keys whose first request in the window was a miss with a known origin latency (cache hits carry no origin latency). The shadow TinyLFU still measures hit ratio, not byte hit ratio.
- Adaptive TTL is not implemented and not claimed; there is no origin-change signal to learn from.
- Lifecycle stays `UnitTested`. Moving to `Integrated` needs L4's wiring verified with real events; `Evaluated` never applies until a pre-registered experiment exists.

## 7. Clippy and final run
Final run after the last edit (reproduced here, debug build, lane copy): clippy clean as above, `cargo test --lib intelligence_v2` = 37 passed, 0 failed.
