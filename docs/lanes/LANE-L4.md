# LANE-L4 handover (AG-S, lane copy `D:\ClaudeTemp\lanes\L4\`)

Evidence tags: *reproduced here* = my run in the lane copy (release unless stated, loopback, one Windows 11 host, single runs, no performance claim). Evidence files are in `D:\ClaudeTemp\lanes\evidence\L4\`.

## 1. Status per item

| Item | Status | Evidence |
|---|---|---|
| 1 Static origin cap in every mode | **DONE** | Plain counter in the server layer (`StaticOriginCap`, `acquire_origin_permit` now returns an `OriginAdmission` holding the optional executor permit and the static permit; the frozen executor is untouched and still runs only in canary/active). Set from `PHANTOM_ORIGIN_CAP_MAX` in `main.rs`. Rejection = 503 `OriginCapSaturated`, not counted as an origin dispatch; leader/coalesced/bypass/revalidation semantics unchanged (only the dispatching party takes a permit). New `metrics.origin_static_cap` {limit, in_flight, rejections} (measured). **Test** `test_l4_static_origin_cap_limits_origin_concurrency_in_every_mode` (real gateway processes: cap 3 in mode off and observe, origin peak <= 3, 503s equal counted rejections, permits released; control process without a cap reaches > 3). **Mutation** (static cap forced off): test FAILS (`L4_mut1.log`). **Live** (`static_cap_live.mjs`, reproduced here): 60 concurrent distinct cold keys, 300 ms origin: cap 4 mode off -> origin max in flight **4**, 4x200 + 56x503; cap 4 mode advisory -> max 4; no cap -> max **60**, 60x200 (`cap_live_*.txt`). |
| 2 Canary/active guard | **DONE** | `PHANTOM_INTELLIGENCE=canary|active` is a config error unless `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true` (new key, default false). Live: exit **2**, one `[CONFIG ERROR]` line (`guard_canary.txt`); with the flag it starts (warning kept). Test `test_l4_canary_and_active_are_rejected_without_the_experimental_opt_in`; mutation (guard disabled): FAILS (same log). |
| 3 Security behaviours | **DONE (test added; nothing broken found)** | In-process tests for 401/403/tenant isolation already exist (`m7_sec_fixes_tests`, `security_phase1_tests`). Missing was a real PROCESS: added `test_l4_secure_gateway_process_401_403_and_tenant_isolation` (JWT secret set; no/garbage/wrong-signature token = 401 with `WWW-Authenticate` and no origin contact; role mismatch = 403 both ways; tenant A and B get separate entries with tenant-echoing origin; A's DELETE purges A only; health stays public). Passes. Not mutation-proven (it confirms existing documented behaviour). One thing the test taught me: with `Authorization` present a response with only `max-age` is NOT stored (RFC 9111); it needs `public`/`s-maxage` (documented behaviour holds). |
| 4 PIE v2 wiring | **DONE against L3's CURRENT module; LANE-L3.md did not exist yet** | Copied `src/intelligence_v2` from L3's lane copy (final copy taken just before the release build; L3 was still changing it, so **re-copy L3's final files and re-run `test_l4_pie_v2_routes_and_event_wiring`**). Routes per contract: `/registry`, `/signals`, `/decisions`, `/decisions/:id`, `/policy/:id`, `v2` key added to `/status` (old fields kept, `intelligence_mode` from the v2 mode), all behind `require_observer_if_secure`; 400/404 bodies from the library. `main.rs` installs `Pie` (Off/Observe/Advisory only; canary/active leave it Off) and spawns the 1 s tick+watchdog task only when not Off. One non-blocking `try_record` on the GET path (hash only) and on DELETE (`Invalidation`); a full queue drops and counts; zero effect on responses. Test passes (600 requests, all 200, >= 600 events enqueued, 0 dropped, signals history has the requests, registry/policy/decisions shapes, 400/404, no request key in output). **Mutation** (`pie_record` disabled): test FAILS (`L4_mut2.log`). Hot-path Off vs Observe measurement: script written (`hotpath.mjs`), **NOT RUN** (the machine was busy with the serialised full build; measuring now would be noise). |
| 5 Regression on a release build of the lane copy | **PARTIAL** | Release build OK (67 min wall, mostly waiting for the cargo lock). Reproduced here on that binary: `cluster_burst_check` PASSED (20 cycles), `cluster_outcomes_check` PASSED, `cluster_deadpeer_storm_check` PASSED (400 x 204; it took 2.5 s this time versus 0.4 s in earlier batches with the same code path, the machine was loaded by other lanes, not investigated). Full `cargo test --tests --no-fail-fast` on the lane copy was STILL BUILDING when I wrote this (log `D:\ClaudeTemp\lanes\evidence\L4_full.log`): counts NOT available. `wp3_tests` alone (dev): **41 passed, 0 failed** (37 before, +4 new). |

## 2. Files changed or created (relative to the lane root)
- `src/api/server.rs` (static cap, `metrics.origin_static_cap`, PIE v2 handlers/statics/routes, request-path events, status `v2`)
- `src/main.rs` (`pub mod intelligence_v2;`, `set_static_origin_cap`, PIE v2 install and tick task)
- `src/config.rs` (new key, guard, warning text, key help text)
- `src/lib.rs` (`pub mod intelligence_v2;`)
- `src/intelligence_v2/*` (COPIED from lane L3, not mine; take L3's final)
- `tests/unit/wp3_tests.rs` (4 new tests, helper `l4_slow_origin`)
- `docs/user/configuration.md`, `docs/user/operate.md`, `docs/user/limitations.md`
- `docs/demo/scenario_b_interpretation_notice.md` (copy of the repo file, only because the lane copy lacked it and `include_str!` needs it; do NOT take it, it already exists in the repo)

## 3. Findings for files I do not own (requests)
1. **`tests/integration/m7_config_tests.rs:228` (`origin_cap_semantics`) will fail**: it configures `PHANTOM_INTELLIGENCE=canary` (now rejected without the flag) and asserts the "inert" warning is absent for canary. Needed: add `("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE","true")` to that call. The "inert" warning for the non-canary case still exists but now says the static cap is enforced and only the adaptive part is inert (it still contains the word "inert", so the second assertion holds). Not run by me (full run unfinished), reasoned from the code.
2. **`src/lab_live/scenario_b.rs` (lines ~159, ~163, ~1235) starts gateways with `PHANTOM_INTELLIGENCE=canary`**: they must also set `PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE=true` or Scenario B gateways will exit 2. Also `scripts/run_e2e_acceptance.ps1:138`.
3. Scenario B compares "static cap" with AIMD in canary mode; the new static cap in every mode changes what "BASE (off)" means when `PHANTOM_ORIGIN_CAP_MAX` is set. The lab sets the cap only for capped arms, so I expect no change, not verified.

## 4. Reproduce
- Cargo only through `with_cargo_lock.ps1`, target `D:\ClaudeTemp\lanes\target_L4`, from `D:\ClaudeTemp\lanes\L4`: `cargo test --locked -j 1 --test wp3_tests`; release: `cargo build --release --locked -j 1 --bin phantom-cache-demo`.
- Live cap: `node D:\ClaudeTemp\lanes\evidence\L4\static_cap_live.mjs <exe> <cap|0> [off|observe|advisory]` (ports 18220/19220).
- Guard: `PHANTOM_INTELLIGENCE=canary <exe>` -> exit 2.
- Cluster scripts on my ports: `node scripts/cluster_burst_check.mjs --exe <exe> --certs <certs> --origin-port 19220 --http-port-base 18222 --cluster-port-base 19621`.

## 5. Limitations
- The static cap does not queue: a request that finds it full gets 503 at once (60 concurrent cold keys with cap 4 = 56 x 503). That is the honest behaviour of a fixed limit; a queue or waiting budget would be a different feature.
- The cap and the PIE sink are process-global (the gateway state struct is built field by field in many tests); the cap test therefore uses real processes.
- PIE v2 wiring is verified only against the L3 files copied at the time; the contract's `installed:false` path is not tested in the gateway (L3's library tests cover it).
- No performance claim: hot-path cost was not measured; no full-suite counts.
