# Default TTL decision (O2-1 / R-06): recommendation

Status: INVESTIGATION + PROPOSED PATCH. Nothing here has been applied to the working tree. The patch was validated only in a scratch copy (`D:\Temp\ttl_scratch`, see `evidence/`). Integration needs owner/lead approval (application code in `src/api/server.rs`, `src/config.rs`, `src/main.rs`, a `Cargo.toml` `[[test]]` entry, and edits to existing tests and user docs). It does **not** touch `src/core/*` or `src/lab/**` (M6 freeze).

Evidence tags: [V] verified in code (file:line), [R] reproduced in scratch (log path), [A] assumption, [H] hypothesis, [L] literature, not re-verified from a primary source here.

## 1. Facts established

| # | Fact | Tag |
|---|---|---|
| F1 | `PHANTOM_DEFAULT_TTL_MS` is declared with range 1..31_536_000_000, default `60000`, `Wiring::NotWired(..)` at `src/config.rs:116-118`. A non-default value only produces the warning at `config.rs:855-858` and the "(not honored by runtime)" report mark (`:950-951`). It is read nowhere else in `src/`. | V |
| F2 | `src/api/server.rs` passes a literal `60_000` to `evaluate_response_freshness` at two sites: background revalidation 200 path (`:1401` in the tree at investigation time; earlier docs cite `:1367`) and the foreground fill path (`:1684`; earlier `:1650`); line numbers drift because `server.rs` is being edited concurrently. | V |
| F3 | `evaluate_response_freshness` (`src/core/freshness.rs:139-293`) computes the lifetime in priority order: `no-cache` => 0 (`:263-264`), `s-maxage` (`:265`), `max-age` (`:267`), `Expires` minus `Date` (`:269-272`), else `default_ttl_ms` (`:273-276`). The default is therefore used only for a cacheable response that has none of those. | V |
| F4 | There is **no heuristic freshness**. `Last-Modified` is captured only as a validator (`freshness.rs:174-177`, stored in `FreshnessMetadata.last_modified`); it never feeds the lifetime. RFC 9111 allows heuristic freshness but does not require it [L]. | V |
| F5 | Uncacheable (never gets a TTL): close-delimited body (`:149`), `no-store`, `private`, invalid `Cache-Control` (`:206-214`), status other than 200 (301/404/410 only with explicit freshness, `:217-229`), `Authorization`/`Set-Cookie` without `public`/`s-maxage`/`must-revalidate` (`:231-235`), `Vary` other than `Accept-Encoding` (`:237-248`). The default TTL changes none of these. | V |
| F6 | 304 merge: `merge_not_modified` calls the evaluator with `stored_ttl_ms` **as** `default_ttl_ms` (`freshness.rs:325`) and keeps `stored_ttl_ms` unless the 304 carries `Cache-Control` or `Expires` (`:335-337`). `complete_not_modified` feeds it the resident entry's `ttl_ms` (`src/core/cache.rs:1042-1049`). So a 304 never consults the configured default; a header-less entry keeps the lifetime it was stored with. Wiring the fill path is sufficient for the 304 path, no change needed there. | V |
| F7 | `docs/research/rfc9111_supported_subset.md:41` documents "Default TTL Fallback (O2-1): ... `PHANTOM_DEFAULT_TTL_MS = 60000` ms (preserving Phase 1 baseline behavior)". `docs/user/configuration.md:44`, `limitations.md:9`, `troubleshoot.md:14` and `CLAUDE.md` section 5 all state the key is not honored. `docs/release/deferred_integration.md` D-3 already describes this exact fix. | V |
| F8 | Tests that rely on the 60 s fallback: every test that uses a header-less mock origin (for example `tests/integration/m1_http_tests.rs`, whose mock sends only `ETag`, lines 98-116); `freshness_matrix_tests.rs` passes `default_ttl_ms` explicitly to the pure function (unaffected); `m7_config_tests.rs:38` asserts the schema default 60000 and `:369-374` asserts the **not-honored warning** (this one must change with the patch). | V |
| F9 | Lab: the live lab origin sends `Cache-Control: max-age=<N>` with default 3 (`src/lab_live/cli.rs:23`, `origin.rs:38`). T1/T2 and Scenario B therefore never use the default TTL; their results are insensitive to this decision, and no lab result can be cited for or against any default value. | V (code), A (that no lab scenario omits max-age) |
| F10 | Hard stale cap `PHANTOM_MAX_STALE_MS` (default 24 h) bounds serving **past** expiry via `check_stale_serving` (`server.rs:1280-1285`, `cache.rs:345-349`). It does not shorten the fresh lifetime. | V |
| F11 | There is no gateway ceiling on origin-supplied lifetimes: `max-age=31536000` is stored as 365 days (`freshness.rs:267-268`, `saturating_mul`). | V |

## 2. Alternatives

Staleness exposure after a lost or late invalidation equals the **remaining freshness lifetime** of the entry (F11, plus the stale window if `stale-while-revalidate`/`stale-if-error` is present, capped by `PHANTOM_MAX_STALE_MS`). For header-less responses that is at most the default TTL. Origin load for a continuously requested key is about one fetch per lifetime per gateway (single flight). Figures below are arithmetic under that assumption (A), not measurements.

| Option | Staleness bound (header-less) | Origin load, 10,000 hot keys | Compatibility | Verdict |
|---|---|---|---|---|
| A. Keep 60 s as default, honor the key (**recommended**) | 60 s | ~167 fetch/s | identical to today when unset | **Recommended** |
| B. Lower default (10 s / 30 s) | 10 s / 30 s | ~1000 / ~333 fetch/s | silent behaviour change for every header-less origin; more origin load; no evidence it is needed | Operators can choose it via the key; do not change the default |
| C. Higher default (300 s+) | 300 s+ | ~33 fetch/s | changes staleness exposure upward silently, worsens the lost-invalidation window | Reject as a default; allowed via the key |
| D. No default: treat header-less responses as uncacheable | 0 (not cached) | every request reaches origin | breaks the Phase 1 baseline, `rfc9111_supported_subset.md:41`, and every header-less test origin (F8); the key's minimum is 1 so it could not even be expressed as 0 | Reject |
| E. Heuristic freshness (10 percent of `Last-Modified` age, with ceiling) | derived | derived | not implemented (F4); needs a change in `src/core/freshness.rs` (frozen), a Date/Last-Modified parse policy, and new tests; changes behaviour for origins that send `Last-Modified` only | Out of scope; candidate for a later, separately approved phase [A] |

No workload evidence exists in this repository that favours B or C over A (F9), so the default stays 60 s. The decision the patch encodes is: the **mechanism** (a documented, validated, honored key) rather than a new **value**.

## 3. Trade-offs

* Origin load vs staleness: they trade off 1/TTL against TTL; see the table. The operator, who knows the origin's capacity and tolerance, should choose; the gateway should not change the safe historical value silently.
* Cold start: an empty cache pays one origin fetch per key regardless of TTL; a short TTL re-pays it more often. Restart loses all entries (in-memory), so the TTL is also the only bound after a restart that lost invalidation state (accountability report section 12).
* Compatibility: with the key unset the effective default is 60000 as before (the global is initialised to 60000, and the schema default is 60000). Only deployments that already set the key (none found in `packaging/`, `scripts/`, `README.md`, `.github/`) would change behaviour, and then to the value they asked for.
* Security/privacy: the default applies only to responses that already passed every cacheability gate (F5). It never makes a `private`/`no-store`/authenticated response cacheable. A larger value lengthens the window in which revoked or corrected data can still be served from a shared cache; a very small value (for example 1 ms) behaves like "do not cache header-less responses" and is a safe degenerate setting. `0` stays invalid (config rejects it, `set_default_ttl_ms(0)` is ignored).

## 4. Interaction with invalidation durability and a freshness ceiling

Because staleness after a lost invalidation equals the remaining lifetime, the real exposure is not the 60 s default but any **origin-supplied** lifetime (F11): `max-age=86400` means up to 24 h of undetected stale data after a lost peer invalidation, plus the stale window. A gateway-side ceiling `PHANTOM_MAX_FRESHNESS_MS` (unset by default = today's behaviour) would bound it.

Evaluation of adding it **in this patch**: not recommended, for correctness reasons rather than cost.

1. A fill-path-only clamp (`server.rs`, two sites) is easy and safe, but incomplete. A 304 refresh takes its lifetime from the 304's own `Cache-Control`/`Expires` inside `merge_not_modified` -> `refresh_entry_lifetime_for_guard` (`cache.rs:1042-1051`). That code is in `src/core`, which is frozen. A ceiling that holds on fills but not on revalidations would be a misleading guarantee.
2. A complete ceiling needs either a one-line clamp at `cache.rs:1050` (`ttl_ms.min(ceiling)`), or a parameter on `merge_not_modified`; both are in the M6 freeze and need a new freeze addendum, an independent review (CLAUDE.md section 5: changes touching fencing/epochs/invalidation), and a test that fails without the change.
3. The interaction with `stale-while-revalidate`/`stale-if-error` needs a stated bound: total serve-without-contact is ceiling + min(stale window, `PHANTOM_MAX_STALE_MS`). Whether the ceiling should also clamp the stale window is a design decision for the owner.
4. Semantics change: the gateway would be deliberately shorter-lived than the origin's declared freshness. I believe RFC 9111 permits a cache to treat a response as stale earlier than its freshness lifetime [L, not verified against the primary text here]; it must be documented as a deliberate deviation from "origin is authoritative", and it raises origin load for long-lived resources.

Recommendation: ship the TTL wiring now; track the ceiling as a separate follow-up (suggested id R-06b) with the design above, default off, with the cache.rs clamp, a freeze addendum and an independent review. When designed, `ceiling` must also clamp the configured default TTL (`min(default_ttl, ceiling)`) so the two keys cannot contradict. A reasonable operator starting point would be a ceiling in the range of minutes to an hour for deployments with multi-gateway invalidation, but that value is an assumption and needs a workload and a failure-model argument (CLAUDE.md section 4).

## 5. Concrete recommendation

1. Keep the effective default at **60 000 ms** (no behaviour change when the key is unset).
2. Make `PHANTOM_DEFAULT_TTL_MS` honored: `Wiring::NotWired` -> `Wiring::Main`; `main.rs` calls `api::server::set_default_ttl_ms(cfg.get_u64("PHANTOM_DEFAULT_TTL_MS"))`; both `server.rs` fill sites call `default_ttl_ms()` instead of the literal. The 304 path needs no change (F6).
3. Keep the existing validation (1..31 536 000 000 ms); no tightening, to avoid breaking existing config files. A value of 0 is rejected by config and ignored by the setter.
4. Update the user docs and the one existing test that asserts "not honored" (`m7_config_tests.rs`), add `tests/integration/default_ttl_wiring_tests.rs` (+ a `[[test]]` entry).
5. Do not add heuristic freshness, "no default", or the freshness ceiling in this patch.

Design note: the value travels through a process-wide atomic in `server.rs` (the same pattern as `GLOBAL_INTEL_EXECUTOR`) rather than a new `ApiState` field, because `ApiState` is constructed by struct literal in about 46 places (tests, benches, `main.rs`), so a new field would force edits across many existing test files. Cost: global state, so tests that change it must live in their own test binary (the new file does). If the owner prefers a field, the follow-up is mechanical but touches those files.

## 6. Integration requirements (main tree)

* Approval: owner/lead approval for application-code change in `src/api/server.rs`, `src/config.rs`, `src/main.rs`; `Cargo.toml` edit adds one `[[test]]` entry only (no dependency changes).
* Freeze: **no frozen file is touched** (`src/core/*`, `src/lab/**` untouched). No freeze addendum needed for this patch. A ceiling would need one.
* Apply: `git apply --check docs/research/ttl_decision/proposed_patch.diff` then apply; no `git add`/commit unless the owner asks. The diff is against the working tree as of this investigation (it modifies already-modified files; re-check if those files changed since).
* Must run afterwards: see `test_plan.md` (the full suite is `cargo test --all-targets -j 1`; clippy `-D warnings`).
* Docs to update with CLAUDE.md approval (not in the patch): CLAUDE.md section 5 "NOT WIRED" entry; `docs/research/claims_ledger.md`/`master_audit_and_execution_plan.md` mentions; `docs/release/deferred_integration.md` D-3 status; `rfc9111_supported_subset.md:41` is already correct once the key is honored.
* Risk: low. The only runtime change with the key unset is `60_000` literal -> an atomic load initialised to 60 000.
