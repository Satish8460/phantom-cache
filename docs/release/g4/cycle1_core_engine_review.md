# G4 Cycle 1 - Core Cache Engine Review (reviewer G4A)

Reviewer: G4A, an AI code reviewer (Claude). This is NOT independent human review.
Mode: read-only static review. No cargo runs, no test execution, no git mutations. Nothing in this file was reproduced by running code. "CONFIRMED" below means confirmed by reading the code with a concrete trace, not by an executed failing test.
Tree: branch `feat/publish-phantomcache-core`, HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, large uncommitted working tree (93 status lines at start of the review). Last reported full suite (reported by caller, not reproduced): 851 passed / 0 failed / 8 ignored, debug, 75 targets.

Scope read in full: `src/core/cache.rs` (1883 lines), `coalesce.rs`, `eviction.rs`, `freshness.rs`, `src/intel/diagnostics.rs`, `src/intel/telemetry.rs` (hit-ratio, histogram, ring parts), `src/api/server.rs` lines ~1130-2220 and 2370-2800 (GET, DELETE head, stale/304/bypass, upstream request builder, response parser head). Not read: server.rs PUT/lab/other handlers, `src/dist/**` (only the two invalidation call sites), `src/security/**` (only config defaults).

## 1. Summary counts

| Severity | CONFIRMED | PLAUSIBLE | REFUTED | Total |
|---|---|---|---|---|
| Critical | 0 | 0 | - | 0 |
| High | 2 | 0 | - | 2 |
| Medium | 8 | 2 | - | 10 |
| Low | 9 | 1 | - | 10 |
| Info | 4 | 0 | - | 4 |
| Refuted hypotheses (section 4) | - | - | 7 | 7 |

Total findings: 26 (23 CONFIRMED by reading, 3 PLAUSIBLE).

## 2. Top 10 findings

1. G4A-01 (High) `finalize_fetch` re-reads tenant/global epochs after the fence check and stamps the entry with the NEWER epochs, laundering a pre-invalidation body into a "current" entry. `src/core/cache.rs:1462-1466` vs `:1517,1532-1533`.
2. G4A-02 (High, conditional on S3-FIFO being selected) S3-FIFO `on_remove` leaves tombstones in `small_q`/`main_q`; re-insert duplicates the key; with no eviction pressure the queues grow without bound. `src/core/eviction.rs:461-487`.
3. G4A-03 (Medium) Sweeper "batch size" bounds candidates, not keys inspected: a shard with no reclaimable entries is scanned in full under the shard mutex every tick. `src/core/cache.rs:402-438`.
4. G4A-04 (Medium) Upstream `Age`/`Date` initial age is computed but never applied; freshness lifetime restarts at full max-age on receipt, and the emitted `Age` ignores it. `src/core/freshness.rs:254-276`, `src/api/server.rs:1322,1538,1585`.
5. G4A-05 (Medium) `Expires: 0` / any unparseable `Expires` becomes the 60 s default TTL instead of "already expired" (RFC 9111 5.3). `src/core/freshness.rs:217-219,251-252,269-276`.
6. G4A-06 (Medium) Stale-if-error is applied to every non-200/304 and non-cacheable status (404, 401, 403, 429, 3xx), not only 500/502/503/504. `src/api/server.rs:1774-1790`.
7. G4A-07 (Medium) A cached 301 is served without `Location` (and every origin header except Content-Type/ETag is dropped). `src/api/server.rs:1692-1716`, `src/core/cache.rs:120-127`.
8. G4A-08 (Medium) Requests arriving AFTER an invalidation can still join a pre-invalidation in-flight fetch and receive its body (documented L3, not fixed). `src/core/cache.rs:1290-1314`.
9. G4A-09 (Medium) `forwarded_authorization` is derived from the client's `Authorization` header, not from whether it is forwarded; in the default secure configuration (credential not forwarded) responses without `public`/`s-maxage`/`must-revalidate` are never cached. `src/api/server.rs:1681,1398` vs `src/main.rs:422`.
10. G4A-11 (Medium) Telemetry honesty: `calculate_hit_rate()` returns 1.0 for zero observations; ring hit-rate and counter hit-ratio use different definitions; stale serves are not counted as hits and background revalidations count as misses. `src/intel/telemetry.rs:647-658`, `src/api/server.rs:1319-1517,1357`, `src/core/cache.rs:1347`.

## 3. All findings

Format: ID, severity, validity, location, scenario, impact, fix, regression test (to be written; not written into the repo).

### G4A-01 High / CONFIRMED - epoch laundering in finalize_fetch (TOCTOU)
- Location: `src/core/cache.rs:1462-1466` (fence check against `get_effective_epochs`), then `:1501-1515` (eviction loop, up to 32 evictions), then `:1517` (second `get_effective_epochs`), `:1532-1533` (`state.entry_tenant_epoch = current_tenant_epoch`).
- Scenario: leader L begins fetch at epoch (t=3,g=0); state epochs set to (3,0) at `:1370`. Origin returns body B1 (pre-invalidation). L enters finalize, check at `:1462` sees current (3,0) == state (3,0), passes. Another thread calls `invalidate_tenant(t)` (lock-free w.r.t. the shard mutex, `:576-607`) -> epoch 4. L reaches `:1517`, reads 4, stores B1 with entry epoch 4. Every later read compares entry epoch 4 == current 4 and serves B1 until TTL (up to the origin max-age, even 24 h+).
- Impact: invalidation acknowledged to the caller (and to cluster peers) does not remove B1; violates the generation/epoch fencing invariant (CLAUDE.md section 5) and the "delete wins" semantics. Window is narrow (microseconds, widened by the eviction loop and by shard-lock contention) so it is a rare race, not a steady-state bug.
- Fix: capture `(t, g)` once at `:1462` and stamp the entry with those captured values (an invalidation after the capture then makes the entry epoch-stale and it is purged on first read). Remove the second read.
- Test that fails before / passes after: needs a test hook, or a stress test. Deterministic version: add a `#[cfg(test)]` pause point between `:1466` and `:1517` (or make epochs injectable), run: begin_fetch, call finalize on thread A paused at the hook, call `invalidate_tenant` on thread B, resume A, assert `get_resource` returns `None` and `audit_bytes` is balanced. Stress version: 2 threads, loop fill/invalidate 1e6 times on one key, assert that after each `invalidate_tenant` returns, a subsequent `get_resource` never returns a body whose fetch began before the invalidation (tag bodies with a monotonic id).

### G4A-02 High (conditional on policy) / CONFIRMED - S3-FIFO tombstones grow without bound
- Location: `src/core/eviction.rs:484-487` (`on_remove` only touches `freq` and `ghost_set`; entries stay in `small_q`/`main_q`), `:461-476` (`on_insert` of a key absent from `freq` pushes a new queue entry), `:496,501` (tombstones counted in `small_q.len()`).
- Scenario: S3-FIFO selected (`set_eviction_policy(S3Fifo)`). Crawler fetches unique URLs; each entry expires and the sweeper (`cache.rs:441-446`) or DELETE (`:1626-1628`) calls `on_remove`. Map stays small, byte budget not hit, so `select_victim` never runs and never pops tombstones. Each unique key leaves one `CacheKey` clone (two Strings) in `small_q` forever. Re-fetching the same key after removal creates a duplicate queue entry; later `select_victim` can evict the live entry via the stale duplicate (early eviction) and `small_q.len() >= small_cap` is distorted by tombstones.
- Impact: unbounded memory growth outside the byte accounting (queues are not in `audit_bytes`); policy decisions no longer match the cited algorithm. Default policy is Sieve (`cache.rs:332`), so exposure requires an operator/PIE switching to S3-FIFO.
- Fix: on `on_remove` remove the key from the queue (index map / lazy deletion with a generation counter and periodic compaction), and cap queue length.
- Test: with S3Fifo, loop 100k times {on_insert(unique key); on_remove(key)} and assert internal queue lengths stay <= live count + constant (needs a `#[cfg(test)]` accessor, or measure via a debug `queue_len()`); also {insert A; remove A; insert A} then `select_victim` must not return A while A is MRU.

### G4A-03 Medium / CONFIRMED - sweeper inspection is O(shard), not O(batch)
- Location: `src/core/cache.rs:402-438`. The loop breaks only when `candidates_to_reclaim.len() >= max_per_shard`; non-candidates (fresh entries, active ops) are skipped with `continue` and the whole map is iterated, with a `get_effective_epochs` RwLock read per entry, all under the shard mutex, every `PHANTOM_SWEEPER_INTERVAL_MS`.
- Scenario: shard with 10,000 healthy fresh entries, batch 32: each tick walks 10,000 entries under the lock; all request paths on that shard block for that time (tens to hundreds of microseconds in release, more under contention). HashMap iteration order is stable, so the same prefix is rescanned each tick and reclaimable entries at the tail are reached only after the full walk.
- Impact: contradicts the documented meaning of `PHANTOM_SWEEPER_BATCH_SIZE` ("per-tick shard inspection limit", CLAUDE.md section 5) and the lock-hold bound; `test_sweeper_lock_hold_time_bounded` does not catch it (all entries are epoch-stale there so the 32-candidate limit is reached almost immediately: `tests/integration/invalidation_contract_tests.rs:302-336`).
- Fix: keep a per-shard cursor and inspect at most `max_per_shard` keys per tick (or iterate the eviction order), resuming where the last tick stopped.
- Test: 4 shards, 10,000 fresh entries in each, call `sweep_lazy_reclamation(32, now)` and assert that the number of inspected keys (new counter) is <= 32 per shard and that `max_lock_hold_ns` is within the budget; second part: then invalidate the tenant and assert convergence within ceil(N/32) ticks.

### G4A-04 Medium / CONFIRMED - initial age (Age/Date) ignored
- Location: `src/core/freshness.rs:254-260` computes `initial_age_ms`; `:263-276` returns `ttl_ms` = full max-age; `src/api/server.rs:1714` sets `created_at_ms = now_fetch`; hit/stale code uses `created_at_ms + ttl_ms` (`cache.rs:658,1263,781`) and `calculate_current_age_seconds(created_at, 0, now)` (`server.rs:1322,1538,1585,1887`). `initial_age_ms` is stored in metadata but never read for the freshness test.
- Scenario: upstream CDN returns `Cache-Control: max-age=60`, `Age: 59`. RFC 9111 4.2.3: remaining freshness ~1 s. Gateway serves it as fresh for 60 s (total age ~119 s) and advertises `Age: 0..60`, so downstream caches compound the error.
- Impact: serves content up to 2x (or more with chained caches) beyond the origin-declared lifetime; understated `Age` header. Also the hard stale cap is measured from `created_at_ms`, not from the response generation time.
- Fix: set `created_at_ms = now_fetch - initial_age_ms` (saturating) or subtract `initial_age_ms` from `ttl_ms`; pass `initial_age_ms` to the `Age` computation.
- Test: unit on `evaluate_response_freshness`+store path / HTTP test with mock origin sending `max-age=60, Age: 59`; at +2 s assert MISS/revalidation, and assert emitted `Age >= 59` on first hit.

### G4A-05 Medium / CONFIRMED - `Expires: 0` and unparseable `Expires` get the 60 s default
- Location: `src/core/freshness.rs:217-219` (`expires_str.is_some()` counts as explicit freshness), `:252` (`parse_http_date("0")` -> `None` because `s.len() < 10`, `:390`), `:269-276` (falls through to `default_ttl_ms`).
- Scenario: legacy origin sends `Expires: 0` (or `-1`, `Thu, 01 Jan 1970`-style garbage) and no Cache-Control. RFC 9111 5.3 requires an invalid date, "especially the value 0", to be treated as already expired. Gateway caches it for 60 s. For 404/410/301 the invalid `Expires` additionally makes the status "explicitly fresh".
- Impact: caches responses the origin marked uncacheable-by-expiry (stale data for up to 60 s; possibly user-specific if the origin relies on `Expires: 0` only).
- Fix: if `Expires` is present and unparseable, lifetime = 0 (and do not count it as explicit freshness for 301/404/410 unless parseable).
- Test: `evaluate_response_freshness(200, [Expires: 0], ..., default 60_000)` must return `ttl_ms == 0` (currently 60_000); same for `Expires: garbage`.

### G4A-06 Medium / CONFIRMED - stale-if-error used for non-5xx statuses
- Location: `src/api/server.rs:1692` (only 200 and cacheable 301/404/410 are stored), `:1758` (304), `:1774-1790` (everything else: stale-if-error check then error). Same pattern for network errors at `:1824-1838` (correct there).
- Scenario: resource deleted at origin: origin returns 404 without explicit freshness (so `is_cacheable` false) or 410 without max-age, 401/403 after revoked access, 429, or 302. Entry is expired but inside its stale-if-error window: `check_stale_serving(.., true)` returns Eligible and the gateway delivers the OLD body as success (`:1787`).
- Impact: removal/permission changes at the origin are masked for up to the SIE window (capped at 24 h by default). RFC 5861 scopes stale-if-error to 500/502/503/504 (and connection failures).
- Fix: gate the SIE branch on `status >= 500` (502/503/504, 500).
- Test: HTTP test: entry with `max-age=1, stale-if-error=3600`, expire it, origin now returns 404 (no CC) -> expect 404 propagated, `stale_served_total` unchanged; repeat with 403. Currently returns 200 STALE.

### G4A-07 Medium / CONFIRMED - cached 301 loses Location; origin headers dropped
- Location: `src/core/cache.rs:120-127` (`CachedResource` has status, content_type, body, etag only); `src/api/server.rs:1692` stores 301; hit response built at `:1565-1580` from those fields.
- Scenario: origin `301 Location: /new` with `max-age=300`. First client gets via MISS `Upstream`... the stored resource has status 301 and no Location; clients receive 301 with no `Location`. Also dropped: `Cache-Control`, `Last-Modified`, `Content-Encoding`, `Content-Language`, `Content-Disposition`, `Vary`-consistent headers. `Last-Modified` is parsed only into metadata and never emitted, so downstream `If-Modified-Since` flows rely on the cache insertion time (`matches_if_modified_since(ims, None, created_at)`, `:1480,1548`).
- Impact: functional breakage for redirects; downstream caches/browsers get no freshness information from the gateway; if an origin ever sends `Content-Encoding` the bytes are served without it (corrupt for clients; origin is asked only for `Accept: application/json` so this is unlikely but not prevented).
- Fix: either stop caching 301 until headers are stored, or store an allowlisted header set (Location, Cache-Control, Last-Modified, Content-Encoding, Content-Language, Content-Disposition) in `CachedResource` and account its bytes.
- Test: mock origin 301 + Location + max-age, GET twice; second response must have status 301 and the same Location.

### G4A-08 Medium / CONFIRMED (documented as L3) - post-invalidation requests join pre-invalidation fetch
- Location: `src/core/cache.rs:1290-1314` (join of an existing in-flight op without comparing epochs); comment at `:1290-1294` documents it as deferred.
- Scenario: fetch F for key K starts at epoch 3; `invalidate_tenant` -> 4 (acknowledged to caller); request R after the ack calls `begin_fetch_atomic`, finds F in `in_flight`, waits and receives F's pre-invalidation body. `finalize_fetch` correctly refuses to store it (epoch mismatch -> `stale_writes_prevented`), so exposure is limited to waiters of that one op, but R observed data older than a completed invalidation. `delete` avoids this by detaching `in_flight` (`:1637`); tenant/global invalidation does not.
- Impact: weakens the linearizable-invalidation claim (ADR-003 text at `server.rs:1988-1989` speaks of "immediately miss / observe deletion" for DELETE; that holds, but not for tenant/all invalidations). Window = origin latency (up to the 5 s timeout) per in-flight key.
- Fix: record `(tenant_epoch, global_epoch)` in `SharedOp` and, on join, start a new fetch (detach the old one) if the epochs differ.
- Test: begin fetch (Initiate), `invalidate_tenant`, then `begin_fetch_atomic` same key must return `Initiate` (not `Wait`); finishing the old op must not store.

### G4A-09 Medium / CONFIRMED - `forwarded_authorization` means "client sent Authorization"
- Location: `src/api/server.rs:1681` and `:1398`: `has_auth = headers_for_task.contains_key("authorization")` where `headers_for_task = clean_headers` still contains the client credential; the upstream writer forwards it only if `forward_client_authorization` (`:2646-2667`). `src/main.rs:422` sets `enforce_rfc9111_auth_bypass = forward_client_authorization`, so by default (credential not forwarded) JWT-authenticated requests are NOT bypassed, but `evaluate_response_freshness` (`freshness.rs:231-235`) then treats them as "forwarded auth" and refuses to cache unless the origin sent `public`, `s-maxage` or `must-revalidate`.
- Impact: in the default secure deployment the cache is effectively disabled for ordinary origins (fail-closed: safe, but the hit ratio is ~0 and the behaviour is surprising). Also `has_auth` in 304 merge (`:1385,1764`) has the same meaning.
- Fix: compute `has_auth = forward_auth && client_sent_auth`.
- Test: secure-mode server (JWT on, default config), origin returns plain 200 with no Cache-Control; second authenticated GET must be HIT. Currently MISS/uncacheable.

### G4A-10 Medium / PLAUSIBLE - TTL, staleness and cap use the wall clock
- Location: `src/intel/diagnostics.rs:197-202` (`SystemTime`), used for `created_at_ms`, `now_ms` everywhere.
- Scenario: clock stepped back by D (NTP/VM resume): `created_at + ttl > now` holds for D longer, `age_ms` saturates at 0 (`cache.rs:802`), so fresh and stale windows are extended by D and `stale_cap_violations` (same clock) cannot detect it. A forward step expires everything early (thundering herd). Debug builds were not checked for `as u64` of pre-epoch time (`unwrap_or_default` gives 0).
- Impact: the "hard stale cap strictly enforced" claim holds only relative to the wall clock. Not demonstrated; depends on host clock behaviour.
- Fix: derive `now_ms` from a monotonic base (`Instant` anchored once to `SystemTime` at start) for freshness decisions; keep wall clock only for HTTP-date comparisons.
- Test: inject a clock into `DiagnosticBuffer::now_ms` (or the server) that steps backwards by 1 h and assert a 60 s entry is not served after 61 s of monotonic time.

### G4A-11 Medium / CONFIRMED - hit-rate / hit-ratio honesty
- Locations and facts:
  - `src/intel/telemetry.rs:653-655`: `calculate_hit_rate()` returns `1.0` when there are no GET observations (presented as 100% hit rate with no data; should be `None`/unavailable).
  - Ring definition (`telemetry.rs:656`, observations created at `server.rs:1297-1303,1508-1514,1580,1607,1920-1931`): coalesced waiters and stale serves count as HITs, bypass counts as MISS, errors count as MISS.
  - Counter definition (`telemetry.rs:241-245,258-260` over `cache_hits`/`cache_misses`): hits only from `begin_fetch_atomic` Hit (`cache.rs:1285`); stale serves return before `begin_fetch_atomic` (`server.rs:1320-1517`) so they are in neither counter; coalesced waiters are in neither (`cache.rs:1310`); each background revalidation leader increments `cache_misses` (`cache.rs:1347`, via `server.rs:1357`) even though the user was served.
  - Result: two different "hit ratio" numbers, and under stale-while-revalidate load the counter ratio understates hits and overstates misses.
- Impact: UI/telemetry values are not what their labels claim (CLAUDE.md section 4 telemetry rule); any PIE/analysis using the counter ratio is biased.
- Fix: one documented definition (e.g. hits+stale+coalesced over cacheable GETs), count stale serves explicitly, exclude background revalidation from `cache_misses` (use `revalidation_started`), return `Option` for empty windows.
- Test: 1 miss + 1 hit + 1 stale serve + 1 coalesced waiter: assert documented ratio; empty ring: assert `None`/unavailable not 1.0.

### G4A-12 Medium / PLAUSIBLE - tenant-epoch table: O(n) snapshot per invalidation and peer-controlled tenant strings
- Location: `src/core/cache.rs:581-605` (`guard.clone()` of up to 10,000 `String` keys per invalidation under the `tenant_epochs` mutex); `src/dist/node.rs:548` passes the peer-supplied `payload.key` straight to `try_invalidate_tenant` with no tenant-id validation.
- Scenario: 10,001 distinct tenant strings in invalidation frames (peers are mTLS-authenticated, so this needs a compromised/buggy peer or a bug in the frame sender) trigger the SEC-4A-01 fallback: global epoch +1 and a full cluster-wide cache flush; before that, each invalidation costs a 10k-element map clone.
- Impact: availability (cache flush, CPU), not data exposure. SEC-4A-01 itself behaves as designed (entries get `global_epoch` mismatch, correct).
- Fix: validate tenant ids in `try_invalidate_tenant` with the `CacheKey::validate` charset; store the snapshot as a persistent/COW structure or per-tenant atomics.
- Test: call `try_invalidate_tenant` 10,001 times with distinct tenants and measure total time/allocations; assert invalid tenant strings (`"a:b"`, 300 chars) are rejected.

### G4A-13 Low / CONFIRMED - byte drift in `OriginFetchGuard::drop` PHASE_FINALIZING cleanup
- Location: `src/core/coalesce.rs:274-283`: `shard.map.remove(&self.key)` without `shard.bytes -= calculate_state_bytes(...)` and without `eviction.on_remove`. The placeholder's key+overhead was added at `cache.rs:1352`.
- Scenario: `finalize_fetch` panics after setting PHASE_FINALIZING (the only existing trigger is `test_panic_after_waiter_extraction`, `cache.rs:1446-1451`; any future panic inside finalize behaves the same). Placeholder with no entry is removed from the map but its bytes stay in `shard.bytes`: `audit_bytes()` accounted < recorded (violates I1-I8) and the leak is permanent (the shrink step may later evict real entries to compensate).
- Fix: use the same removal helper as the other paths (subtract `calculate_state_bytes`, call `on_remove`).
- Test: inject the panic (existing flag), drop guard, assert `audit_bytes().0 == audit_bytes().1` (does the existing M1-T23 test assert this? Not verified).

### G4A-14 Low / CONFIRMED - `days_from_civil` overflow on client-controlled dates
- Location: `src/core/freshness.rs:504-515` (`era * 146_097` in i64), reachable from `matches_if_modified_since` on the client's `If-Modified-Since` (`server.rs:1479-1480,1547-1548`) and from origin `Date`/`Expires`. Module doc says "never panics on any input" (`:5`).
- Scenario: `If-Modified-Since: Sun, 06 Nov 9000000000000000000 08:49:37 GMT` parses (fits i64) then overflows. Debug build (`overflow-checks` on by default in dev profile, no profile override in Cargo.toml): panic in the handler task. Release: silent wrap, producing a meaningless comparison (could return 304 or 200 incorrectly; no memory safety issue).
- Fix: bound the year (e.g. 1900..=9999) before arithmetic or use `checked_mul`.
- Test: `parse_http_date("Sun, 06 Nov 9000000000000000000 08:49:37 GMT")` must return `None` and not panic (fails in debug today).

### G4A-15 Low / CONFIRMED - `try_delete_with_status` mutates before a fallible `?`
- Location: `src/core/cache.rs:1663-1675`: `state.entry = None; state.freshness = None;` then `.ok_or(GenerationOverflow)?`; the byte subtraction at `:1680` is skipped on the error path (resident bytes overstated). `delete_with_status` swallows the error to `(false,0)` (`:1704`). Also `reclaimed_bytes = entry.body.len() as u32` truncates bodies >= 4 GiB (`:1666`); `delete_impl` and `get_resource*` ignore generation overflow silently (`:1615,667,737,1274`), which would disable fencing at `u64::MAX`.
- Reachability: 2^64 deletes of one key; effectively unreachable. Listed for correctness hygiene.
- Fix: compute `next_gen` first; apply all mutations after.
- Test: construct `KeyState` with `generation = u64::MAX` (test-only) and assert delete leaves `audit_bytes` balanced and returns an error without mutating.

### G4A-16 Low / CONFIRMED - sweeper discards validator-bearing entries right after expiry
- Location: `src/core/cache.rs:417-432`: retention window is 0 unless SWR/SIE present, and for `must_revalidate` always 0, so an expired entry with an ETag/Last-Modified is deleted within one sweeper tick (default 100 ms).
- Impact: conditional revalidation (`stored_validators`, `server.rs:2444`) works only for requests that arrive within about 100 ms after expiry; `no-cache`/`must-revalidate` responses (ttl 0) lose their validators almost immediately. The 304 path is thereby mostly dead for the exact class of responses that need it. Efficiency/conformance, not safety.
- Fix: retain validator-bearing expired entries for a bounded grace period (still counted and evictable).
- Test: entry `no-cache` + ETag, advance past ttl, run sweep, assert entry still present (peek_resident) so a conditional request can be sent.

### G4A-17 Low / PLAUSIBLE - epoch pair read is not atomic w.r.t. `invalidate_all` / fallback
- Location: `src/core/cache.rs:560-570` reads the snapshot RwLock then `global_epoch` separately; `try_invalidate_all` stores the new `global_epoch` (`:622`) before replacing the snapshot (`:625-629`); `try_invalidate_tenant` fallback likewise (`:587-604`).
- Scenario: reader sees (old tenant epoch 3, NEW global g+1) in the window, finalizes an entry stamped (3, g+1) (this combination is only possible through this window, and G4A-01 makes it easier to hit). After the snapshot clear the tenant epoch reads 0, so the entry is purged on next read - unless the tenant is invalidated three more times before anyone touches the entry (epoch returns to 3 within global g+1), which would resurrect it.
- Impact: extremely narrow; theoretical resurrection of a pre-fallback entry.
- Fix: put (tenant map, global epoch) behind one snapshot (`Arc<EpochView>` swapped atomically).
- Test: model/loom test of reader vs `invalidate_all`; assert no (stale-tenant-epoch, new-global) pair is ever observable.

### G4A-18 Low / CONFIRMED - stale-serving edge cases
- `cache.rs:813`: with `hard_cap_ms == 0` an entry at exactly `age == ttl` is "stale" (fresh test is strict `>`, `:781`) yet passes `age <= ttl + 0` and is served (0 ms stale).
- `cache.rs:793`: in error context `stale_if_error_ms.or(stale_while_revalidate_ms)` lets a response that only declared SWR be served on error.
- `cache.rs:824-829` + `server.rs:1777-1781,1827-1831`: `record_stale_served` is passed an age computed with a later `now` than `check_stale_serving` used, so an entry served at the exact boundary can increment `stale_cap_violations` spuriously; the counter is otherwise unfalsifiable because `Eligible` already implies `age <= lifetime + cap`. The "stale_cap_violations == 0" gate therefore proves little (see test list).
- Fix/test: unit tests for each (hard_cap 0 at age==ttl; SWR-only entry in error context; record with now2 = now1 + 1 at boundary).

### G4A-19 Low / CONFIRMED - tenant alias `__default__`
- Location: `src/core/cache.rs:32,44-50`. `None` tenant maps to the string `__default__`, which also satisfies the tenant charset. A client (insecure mode `x-tenant-id: __default__`, or a token with that tenant claim) shares the keyspace of tenant-less requests, and `__default__` is treated as "no x-tenant-id" upstream (`server.rs:2677`).
- Fix: reject the literal in `validate()` or use a reserved character. Test: `CacheKey::new(Some("__default__"),..).validate()` must fail.

### G4A-20 Low / CONFIRMED - eviction recency not fed by stale serves; pinned scans not O(1)
- `cache.rs:814` marks `state.visited = true` on a stale serve, but no eviction policy reads `KeyState.visited` (they keep their own bits, fed only by `on_hit`, `cache.rs:1284`), so a hot entry served stale never gains recency. `eviction.rs:395-404` (LRU) and `:189-219` (SIEVE) walk past pinned keys, so worst case is O(pinned) per eviction, not O(1); pinned = keys with active ops, so an attacker with slow origin paths can raise eviction cost. Documented O(1) claim holds only without pins.
- Test: 1000 pinned (in-flight) keys at the tail, time `select_victim` and assert bounded work (add an inspected-count accessor).

### G4A-21 Low / CONFIRMED - lock poisoning handling is inconsistent
- Recovered via `into_inner`: shard mutexes (most paths), `tenant_epochs`, `epoch_snapshot`, `waiters`. Not recovered: `get_eviction_policy` (`cache.rs:497`, per-miss), `set_eviction_policy` (`:501,504`), `legacy_shards` (`:1715,1734,1746,1759`), all telemetry mutexes (`telemetry.rs:169,178,191,...,457,464,485,492,501,539,548-686`, called at the end of every GET in `server.rs:1920-1931`), `registration.shared_op.waiters.lock().unwrap()` (`server.rs:1631`), `get_freshness`/`update_freshness_metadata` silently skip on poison (`cache.rs:684,694`).
- Impact: a panic while a telemetry lock is held turns every later GET into a panic (handler task dies); recovered shard mutexes may expose half-applied state (see G4A-13).
- Fix: one `lock_or_recover` helper; test: poison `raw_buffer`, then call `record_us` and assert no panic.

### G4A-22 Low / CONFIRMED - status handling gaps
- `server.rs:1692`: origin 2xx other than 200 (203, 204, 206) and 3xx other than cacheable 301 are treated as upstream errors (client sees the status with the generic body "Upstream Origin Error", `server.rs:2182-2187`; `Retry-After`/`Location` lost). Origin 5xx handling is fine.
- MISS path ignores the client's `If-None-Match` (only HIT and STALE answer 304, `:1545,1477`): a conditional client gets 200 on its first request after expiry. Not incorrect, just wasteful.
- HEAD: handled by the GET route (not verified in the router); Range: header stripped, full 200 returned (acceptable per RFC 9110 14.2).

### G4A-23 Info / CONFIRMED - resource bounds
- `CacheShard::with_policy` sets `byte_budget = usize::MAX` (`cache.rs:222`); the bound exists only because `main.rs:205` calls `set_byte_budget`. Embedders/tests using `PartitionedCache::new` are unbounded in bytes.
- Accounting (`calculate_state_bytes`, `cache.rs:198-202`) omits `FreshnessMetadata` strings (`last_modified`), the `HashSet`/`HashMap` allocations and `Arc` headers; I1-I8 verify internal consistency of a model, not process RSS.
- No maximum TTL: `max-age=18446744073709551615` yields `saturating_add` -> never expires (`freshness.rs:266-268`, `cache.rs:658`).
- Entry count bounded by `max_entries_per_shard` (default 10,000/shard) but pinned (in-flight) keys cannot be evicted, so a slow origin plus many distinct keys can saturate a shard and cause `CapacityExceeded` 503s for that shard (bounded, availability only).

### G4A-24 Info / CONFIRMED - percentile conventions differ
- Raw ring uses index `floor(n*p)` (`telemetry.rs:512-515`, p99 of 100 samples = max); histogram uses `ceil(total*p)` rank (`:360-363`). The two estimators disagree by up to one sample for small n; histogram clamps values above ~67 s into the last bucket (`:310-315`). No panic paths found (indices are clamped).

### G4A-25 Info / CONFIRMED - TTL fallback literal
- `60_000` is passed literally to `evaluate_response_freshness` at `src/api/server.rs:1405` and `:1688` (not `PHANTOM_DEFAULT_TTL_MS`); matches the CLAUDE.md NOT-WIRED note. Effective default is always 60 s.

### G4A-26 Info / CONFIRMED - misc
- Each hit clones the body (`(*resource.body).clone()`, `server.rs:1566`), O(body) allocation per request despite the `Arc` (peak transient memory = concurrent hits x body size, bounded by `MAX_BODY_BYTES`).
- `spawn_sweeper_task` reads the interval once (`cache.rs:465`); `set_sweeper_interval_ms` after spawn has no effect.
- DELETE is exact-key: `/p`, `/p?`, `/p//` and sibling queries are distinct entries and are not purged together.
- `set_eviction_policy`/`set_capacity_limits` rebuild the policy from `HashMap` iteration order (recency lost); `evict_with_policy` rebuilds with capacity hint 1024 on a name mismatch (`cache.rs:1214`), so S3-FIFO `small_cap` can differ from the real `max_entries_per_shard`.

## 4. Hypotheses checked and REFUTED

- R1 Header-based cache poisoning (Cookie, Accept-Encoding, X-Forwarded-*, Host): REFUTED. The upstream request builder forwards only `x-request-id` (validated), `x-tenant-id` (from the key), `Authorization` (only if policy allows) and the stored validators; `Accept: application/json` is fixed (`server.rs:2641-2688`). `x-cache-bypass` and client conditional/range headers are never forwarded.
- R2 Vary poisoning: REFUTED. Anything other than `Vary: Accept-Encoding` (and `*`) makes the response uncacheable (`freshness.rs:238-248`); no `Accept-Encoding` is sent upstream.
- R3 Lock-order inversion shard mutex vs `tenant_epochs`: REFUTED. `tenant_epochs.lock()` appears only in the two invalidation functions (`cache.rs:577,614`) and they never take a shard lock; request paths take shard mutex then only the `epoch_snapshot` RwLock read (read-path zero contention on the exclusive mutex holds).
- R4 Std Mutex held across `.await` in the reviewed GET path: not found in the reviewed ranges (guards are scoped in blocks, e.g. `server.rs:1630-1633`). Not exhaustive over the whole of server.rs.
- R5 Cancellation leaves in-flight state: REFUTED. `FetchRegistration::drop` and `OriginFetchGuard::drop` finalize with `Aborted`; the leader work runs in a spawned task owning the guard (`server.rs:1644`), so client disconnect does not abort it; waiters whose receiver is dropped are ignored (`coalesce.rs:119`).
- R6 Cross-tenant canonical-key collision: REFUTED apart from G4A-19. Tenant charset excludes `:` so the first `:` is the separator; `:?` is rejected in paths (`cache.rs:44-50,80-82`); round trip is pinned by `canonical_key_roundtrip_tests`.
- R7 Eviction of in-flight/active keys (I5): REFUTED for all three policies; `is_pinned` covers `exclude_key`, `in_flight` and `active_op_ids` (`cache.rs:1219-1223`) and every policy checks it before selecting (`eviction.rs:201,398,510,542,562`). Residual: an untracked key counts as pinned (`unwrap_or(true)`) so a policy/map desync would permanently clog a policy slot (no way to produce a desync found).

Byte accounting per mutation path (insert `:1516`, replace `:1516`, delete `:1622-1629`, evict `:1229-1230`, expire/sweep `:441-444`, epoch purge `:665-675,734-744,1272-1282`, 304 refresh `:953`, placeholder reclaim `:1547-1548`): consistent except G4A-13 and G4A-15. Resident size uses `body.capacity()` (conservative).

## 5. The ten most important tests in this area

| # | Test (file) | Proves | Does not prove |
|---|---|---|---|
| 1 | `test_delete_fences_in_flight_fill_race`, `test_linearizable_delete_prevents_read` (`tests/integration/invalidation_contract_tests.rs:79,30`) | DELETE during an in-flight fill rejects the stale store (generation fence) and reads miss immediately. | Tenant/global epoch fencing racing the store (G4A-01), post-invalidation joins (G4A-08); deterministic interleavings only, no schedule exploration. |
| 2 | `test_epoch_saturation_fallback_sec4a_preserved` (`invalidation_contract_tests.rs:341`) and `epoch_contention_tests.rs` (`test_invalidate_during_concurrent_reads_linearizable`, `test_capacity_exhaustion_snapshot_consistency`) | SEC-4A-01 fallback fences old entries; snapshot swap is visible to readers. | Atomicity of (tenant, global) pair (G4A-17); cost of the clone (G4A-12); linearizability with finalize. |
| 3 | `test_hard_staleness_cap_enforced_strictly` (`stale_serving_tests.rs:82`) | `check_stale_serving` returns `CapExceeded` past ttl+cap and `Eligible` inside. | The counter assertion is near-tautological: `Eligible` already implies age <= ttl+cap, so `stale_cap_violations == 0` is true by construction (G4A-18); sets metadata through the unfenced `update_freshness_metadata`, not the production path; wall-clock behaviour (G4A-10). |
| 4 | `test_virtual_time_property_stale_cap_violations_always_zero` (`stale_serving_tests.rs:464`) | A fixed grid of 50 keys x 8 offsets never violates the cap. | Not a property test (no generator/shrinking, fixed grid); same tautology as #3; does not exercise HTTP/SIE paths or clock steps. |
| 5 | `test_stale_if_error_on_origin_outage`, `test_must_revalidate_strictly_forbids_stale_serving` (`stale_serving_tests.rs:241,136`) | SIE after network failure and must-revalidate refusal at the cache API. | SIE status scoping (G4A-06); behaviour on 404/403/429. |
| 6 | `byte_accounting_tests.rs` (`test_lifecycle_*`, `test_t_c6_concurrent_stress`, `test_large_body_adversarial_bounded_accounting`) | `audit_bytes` equality across placeholder/fill/replace/fail/expiry/delete/epoch-stale/contraction/concurrency. | Panic-in-finalize path (G4A-13); memory outside the model (queues, freshness metadata, G4A-02/23). |
| 7 | `test_invariant_i5_pinned_never_evicted`, `test_accounting_invariants_preserved_under_eviction` (`eviction_contract_tests.rs:116,230`) | Pinned keys survive eviction; accounting holds under eviction. | Cost bounds with many pinned keys (G4A-20). |
| 8 | `test_lru_golden_trace_parity`, `test_sieve_golden_trace_parity`, `test_s3fifo_golden_trace_parity` (`eviction_contract_tests.rs:45,69,92`) | Basic ordering behaviour (e.g. S3-FIFO does not evict a freq>=2 key first). Names say "golden trace" but the S3-FIFO test asserts only `victim != key_0` after 10 inserts; no comparison against a reference implementation or a published trace. | Faithfulness to the NSDI'24 / SOSP'23 algorithms; remove/re-insert behaviour (G4A-02). Weak for the "faithful eviction" claim. |
| 9 | `test_o1_scaling_bounded_eviction_time` (`eviction_contract_tests.rs:143`) | 50 SIEVE evictions at 10k entries take < 10 us average on the test host in a debug build. | Asymptotic O(1); fixed wall-clock thresholds are host-dependent (flaky on a loaded machine) and the workload has no pinned keys, no visited bits set. Asserts a timing, not a complexity. |
| 10 | `test_sweeper_lock_hold_time_bounded`, `test_sweeper_vs_concurrent_fills_stress` (`invalidation_contract_tests.rs:302,230`) | Sweeper convergence and lock hold < 1 ms in the all-stale case. | The no-candidate full-scan case (G4A-03); a wall-clock threshold again. |

Other tests that were not read: `revalidation_304_tests.rs`, `serving_contract_tests.rs`, `telemetry_*_tests.rs`, `freshness_matrix_tests.rs`, `concurrency_verification_tests.rs`, `memory_bound_soak_tests.rs`. Tests asserting stub behaviour in this area: none identified in the files read; `known_gaps.rs` was not opened. One test, `revalidation_304_tests.rs:1246`, is `#[ignore]` and documented as non-deterministic.

## 6. What I could not verify (needs execution, a human, or external review)

- Everything dynamic: no test was run; the 851/0/8 result and the passing state of the tests quoted above are reported, not reproduced. None of the findings was demonstrated by a failing test; the races (G4A-01, -08, -17) need a deterministic hook or a loom/shuttle model.
- Clippy, whitespace and release-profile behaviour (G4A-14 depends on `overflow-checks`; no profile override found in `Cargo.toml`).
- Rest of `server.rs` (PUT/lab/admin handlers, router layout, whether HEAD is routed), `src/dist/**` internals, `src/security/**`, `main.rs` configuration wiring beyond lines 205 and 422, whether `PHANTOM_MAX_CACHE_BYTES` has a safe default.
- Whether any in-tree code path switches to S3-FIFO at runtime (PIE or lab API); this determines the exposure of G4A-02.
- The cited literature (SIEVE, S3-FIFO, RFC 9111/5861) was compared from memory of the specifications, not from re-fetched primary sources.
- Whether the idle-timeout/5 s origin timeout bounds waiter latency in practice (affects the G4A-08 window).
- A human or external reviewer should cover: the concurrency model as a whole (loom/shuttle or TLA+), the RFC 9111 subset against the spec text, and the security-relevant header/redirect handling.

## 7. Verdict

Fit to be committed as a reviewed group: YES WITH FIXES.

The core engine is carefully built and, within what I could read, its central invariants hold: generation and instance fencing on DELETE and on concurrent fills, pinning of active/in-flight keys in all three policies, RAII cleanup on cancellation, consistent per-path byte accounting, a conservative upstream header allowlist, and the stale cap enforced by construction inside `check_stale_serving`. No Critical finding and no memory-safety issue were found. However two High items should be fixed before the group is described as "reviewed": G4A-01 (invalidation can be bypassed by re-reading epochs in `finalize_fetch`; one-line fix plus a hooked regression test, and CLAUDE.md requires an independent review pass and a failing-before test for changes to fencing/epochs) and G4A-02 (S3-FIFO unbounded queue growth, or else remove S3-FIFO from the selectable set until fixed). Blocking items: G4A-01, G4A-02. Strongly recommended before any externally quoted behaviour claims: G4A-04 and G4A-05 (RFC 9111 conformance of freshness), G4A-06 and G4A-07 (stale-if-error scope, 301 without Location), G4A-09 (secure-mode caching effectively off), G4A-11 (telemetry values mislabelled; 1.0 hit rate on no data), and a rewrite of the weak "golden trace" and cap-counter tests so that the "faithful eviction" and "stale_cap_violations == 0" claims are actually falsifiable. G4A-03 should be fixed or the `PHANTOM_SWEEPER_BATCH_SIZE` documentation corrected. This review is a single AI pass over a subset of the code and does not replace human or external review.
