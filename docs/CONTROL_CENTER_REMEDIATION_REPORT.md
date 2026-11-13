# Control Center Remediation Report (investigation and fix design only)

Date: 2026-10-31 (machine clock). Author: Claude (independent audit). Status: **no application code was modified**; nothing was staged, committed, reset, stashed or discarded. This file is the only file created.

Evidence tags used below: **[code]** read in source, **[unit]** existing unit-test run, **[http]** raw HTTP against a gateway process, **[browser]** real Chrome via CDP, **[multi]** several independent gateway processes, **[not run]** not executed.

---

## 1. Executive verdict

The earlier verdict, "not demo-ready", **stands for the Control Center UI, but three of my earlier findings are corrected below**.

- The backend behaves correctly where I tested it: single-flight coalescing, DELETE-then-miss, per-status error responses, counter accounting, cross-node invalidation in all three directions, and peer read-through. **[http] [multi]**
- The UI misreports what the backend did, in four places that matter for a demonstration: the stampede verdict (hard-coded), the cold-GET and post-DELETE verdict (wrong header and a truthy `"0"`), the traffic-generator hit/miss counts, and the HRW placement calculator (does not implement the router's algorithm). All four are **CONFIRMED**.
- **Corrections to my earlier report.** (a) "GET never uses a peer" was wrong: on a local miss the gateway asks the primary peer over mTLS first (`src/api/server.rs:1688-1735`), and I observed it. (b) The "invalidation lost after a peer restart" finding is **not reproduced** in three controlled reruns; it is downgraded to an unexplained single observation with named hypotheses. (c) The inert origin cap is a **documented design limitation**, not a defect; the defect is only that AG's report presents it as protection.
- The existing UI test suite currently fails **10 of 483** tests. They are the project's own guard tests, and several fail because of files AG added or changed. A demonstration should not be accepted while its own guard tests are red.

The fixes below are small, independent and mostly confined to `ui/src`. No cluster rewrite and no change to quarantined intelligence is needed.

## 2. Current repository baseline

| Item | Value |
|---|---|
| Branch / HEAD | `feat/publish-phantomcache-core` / `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a` |
| Working tree | 158 status entries, 48 modified tracked, 0 staged; identical at the start and end of this task |
| Source vs previous audit | No file under `src`, `ui/src`, `scripts`, `Cargo.toml` or `Cargo.lock` is newer than the binary I built for the previous audit, so the tested source **is** the current source |
| Binary used | Built by me from the current tree: `cargo build --release --locked -j 1` into `D:\ClaudeTemp\audit\target`, exit 0. `Cargo.lock` untouched |
| AG's running gateway | Started 00:52, built 00:51, `server.rs` last edited 00:55; I did not test that binary (see F13) |
| Report file | Did not exist before this task |
| Ports used by me | UI 13000, gateways 18185-18187, cluster 19501-19503, origin 19099 (8085 was held by AG's process). All my processes were stopped; AG's were not touched |

## 3. Finding-by-finding revalidation

IDs F1-F17 follow my previous report; F18-F21 are new.

| ID | Finding | Status | Revised severity | Evidence |
|---|---|---|---|---|
| F1 | Stampede verdict and "exactly 1 origin fetch" are hard-coded | **CONFIRMED** | P1 | [code] `InteractiveConsole.jsx` `handleStampede`: `cacheVerdict: 'COALESCED'` and fixed snippet; [browser] warm key gave 10 HIT, 0 coalesced, 0 origin fetches while the card said COALESCED |
| F2 | UI reads `x-cache`; backend sends `X-Cache-Status`; `Age: 0` read as HIT | **CONFIRMED** | P1 | [code] `handleGet`: `get('x-cache') \|\| (get('age') ? 'HIT' : 'MISS')`; [http] a MISS carries `Age: 0`; [browser] cold GET and GET-after-DELETE both displayed HIT while the backend recorded MISS and an origin fetch |
| F3 | Traffic-generator hit/miss classification | **CONFIRMED** | P2 | [code] `TrafficGenerator.fireOne`: hit if `age !== '0'`; [browser] generator 58 hits / 16 misses vs backend 73 / 1 |
| F4 | HRW calculator does not match the Rust router | **CONFIRMED** | P1 | [code] see 4.C; [measured] agreement with Rust on 5,000 keys: 33.5 % (chance for 3 nodes is 33.3 %) |
| F5 | Origin cap inert; AG shows "cap 10" as protection | **DESIGN LIMITATION** (cap) + **unsupported claim** (AG report) | P3 for the report wording | [code] `executor.rs` `origin_cap_enforcing()` is true only in Canary/Active; startup warning says the same; [http] cap 3, 20 slow distinct keys: origin saw 20 concurrent fetches, `enforcing:false`, `controller_steps:0` |
| F6 | Peer invalidation lost after a node restart | **NOT REPRODUCED** under control; one unexplained observation | P2 (unconfirmed) | [multi] 3 reruns with outages of ~3 s, ~4 s and ~27 s, 6 and 40 frames of prior traffic: delivery succeeded in all directions every time. Original single observation kept as unexplained |
| F7 | Cluster not demonstrable from the UI | **PARTIALLY CONFIRMED** | P2 | [http] `/api/v1/cluster` is 404; no cluster metric keys; no repo script generates cluster certificates. But the distributed behaviour itself exists and works [multi] |
| F8 | Generator: unbounded in-flight; "Dispatched" counts completions | **CONFIRMED** | P2 | [browser] 50 req/s on 1.5 s origin: 190 in flight; [code] `sent` is incremented on completion |
| F9 | History wiped on reload; not keyed by gateway | **CONFIRMED** | P2 | [browser] storage 6,055 chars before reload, 213 immediately after; [code] `useHistory` effect clears when `payload` is null on mount |
| F10 | "Idle / Awaiting traffic" hides real reasons | **PARTIALLY CONFIRMED** | P3 | [code] only one reason string is rewritten; other `reasonText` values are preserved; the section title is generic |
| F11 | Demo script asserts nothing and always reports success | **CONFIRMED** | P2 | [code] `live_interactive_demo.mjs`: every click is `if (btn)`, no assertions, unconditional success banner, hard-coded `.gemini` path |
| F12 | DELETE and error presentation | **CONFIRMED** | P3 | [browser] absent-key DELETE says "successfully purged"; 400/414 cards carry a MISS pill |
| F13 | AG's running binary may predate the final source | **BLOCKED** | n/a | Binary 00:51 vs `server.rs` 00:55; I did not test that process |
| F14 | Capacity bar 2 % floor | **CONFIRMED** | P3 | [code] `Charts.jsx` `CapacityBar`; contradicts its own docstring ("geometry only") |
| F15 | Console title contrast | **PARTIALLY CONFIRMED** | P3 | AG's own screenshots show a very faint title on the white card; I did not measure contrast. `npm run check:contrast` was not run |
| F16 | "RSS" on Windows is working-set size | **CONFIRMED (terminology)** | P3 | [code] `K32GetProcessMemoryInfo` → `WorkingSetSize`. FFI layout, ABI and pseudo-handle use are correct |
| F17 | Linux page size hard-coded to 4096 | **CONFIRMED** | P3 | [code] `get_process_rss_bytes` (linux) |
| F18 | My earlier claim "GET never uses a peer; per-node caches" | **FALSE POSITIVE (my error)** | n/a | [code] `server.rs:1688-1735`; [multi] origin fetches per key matched the predicted primary in 9 of 9 keys (see 4.D) |
| F19 | Peer-served responses are labelled `X-Cache-Status: MISS` | **CONFIRMED** | P2 | [multi] non-primary nodes served from the primary with no origin fetch yet reported MISS |
| F20 | Cluster-mode warning text contradicts the implementation | **CONFIRMED** | P3 | [code] `config.rs:858` says peer invalidations do not purge the HTTP cache; the `main.rs` message says they do; [multi] they do |
| F21 | Browser stampede is not truly simultaneous | **CONFIRMED (design limit of `fetch`)** | P2 | [browser] cold-key run: 1 miss, 4 coalesced, 5 hits for 10 requests (6-connection browser limit) |

**Also found, outside my earlier list**

- **UI test suite red. [unit]** `node --test test` in `ui/`: 483 tests, 473 pass, **10 fail**. The failing tests are the project's own guards: no fixed example numbers, colours only in `tokens.css`, no network calls outside `lib/labApi.js`, no storage use outside the allow-listed files (`hooks/useHistory.js` is cited), placeholder and overview rules, and a lint-clean-in-every-state check. The network, storage and colour failures are directly attributable to AG's new code (raw `fetch` in two new components, `sessionStorage` in `useHistory.js`, inline hex colours). I did not run the suite against `HEAD`, so any failure that pre-dates AG's work is not separated out. The working tree was unchanged by the run.
- **Peer-fill freshness (hypothesis, not tested).** [code] the peer-fill path stores `created_at_ms = now` and `ttl_ms = 60_000`, so an entry that is old on the primary appears fresh on the node that filled from it. Whether this lengthens effective staleness is not established. It needs a dedicated test before anyone claims a bound.

## 4. Root-cause analysis and proposed fixes

For every item: root cause, fix, files, alternative, recommendation, regression risk, verification, acceptance.

### 4.A Cache status parsing (F2, F3, F12, F19)

**Contract (authoritative) [code].** The gateway sets `X-Cache-Status` to one of `HIT`, `MISS`, `COALESCED`, `STALE`, `BYPASS`, `BYPASS-AUTH` on cache responses (`server.rs` around lines 1322, 1538, 1596, 1638, 1986). It is exposed to browsers by CORS (`expose_headers`). Error responses carry `X-Cache-Error` and **no** `X-Cache-Status`. `Age` is present on MISS, HIT, STALE and COALESCED (the integer seconds since creation), so it carries no hit/miss information.

**Root cause.** `InteractiveConsole.handleGet` and `TrafficGenerator.fireOne` were written against a header name (`x-cache`) the gateway does not send, with a fallback that infers the verdict from `Age`. `'0'` is a truthy string in JavaScript, and a fresh hit has `Age: 0` too, so both fallbacks are wrong in different directions.

**Proposed fix.** One pure function, shared by both components, in a new `ui/src/lib/cacheStatus.js`:

```js
const KNOWN = new Set(['HIT','MISS','COALESCED','STALE','BYPASS','BYPASS-AUTH']);
export function parseCacheResponse(status, headers) {
  const raw = headers.get('x-cache-status');
  const err = headers.get('x-cache-error');
  if (raw && KNOWN.has(raw.toUpperCase())) return { kind: 'cache', verdict: raw.toUpperCase(), error: null };
  if (raw)  return { kind: 'unexpected', verdict: 'UNKNOWN', error: null, note: `unrecognised X-Cache-Status "${raw}"` };
  if (err)  return { kind: 'gateway-error', verdict: 'N/A', error: err };
  if (status >= 200 && status < 300) return { kind: 'non-cache', verdict: 'N/A', error: null }; // e.g. /health
  return { kind: 'error', verdict: 'N/A', error: `HTTP ${status}` };
}
```

- Never read `Age` for a verdict. Show `Age` only as `Age` (the real header).
- Error and non-cache responses show no cache-verdict pill (fixes the "MISS" pill on 400/414 and on `/health`).
- DELETE: show "Delete acknowledged (HTTP 204)". The gateway returns 204 whether or not the key existed (idempotent), so the UI must not say "purged"; it can say "key removed if present". [http]
- Files: new `lib/cacheStatus.js`; `components/InteractiveConsole.jsx`; `components/TrafficGenerator.jsx`.
- Alternative: have the backend add a distinct `X-Cache-Status: PEER` (see F19). Recommended as a **follow-up**, not part of the demo fix, because it changes an API contract; for now the UI should state that MISS may include a peer-filled response.
- Regression risk: low; pure function, and the existing contract tests (`ui/test/contract.test.mjs`) should be extended, not replaced.
- Verification: new unit tests in `ui/test/` for the table of inputs (each of the six values, missing header, `X-Cache-Error`, 2xx without header, 4xx/5xx, lower-case values). Browser: cold GET shows MISS; second GET shows HIT; a 414 shows no cache pill.
- Acceptance: for every request in a browser run, the displayed verdict equals the `X-Cache-Status` recorded by an independent raw-HTTP check of the same request, and the backend counter deltas agree.

### 4.B Stampede demonstration (F1, F21)

**Root cause.** `handleStampede` ignores the responses it collects (`x-cache`, `age`) and writes a fixed verdict and a fixed origin-fetch claim.

**Proposed fix.** Derive everything from observed data:

1. Use a **unique cold key** per click: `/api/v1/cache/stampede/<epoch-ms>-<random>`. A reused key is a warm key, which is exactly how the original demo went wrong.
2. Take `/api/v1/metrics` before and after (fields exist: `requests_total`, `cache_hits`, `cache_misses`, `coalesced_waiters`, `origin.requests_started`).
3. Fire N requests; tally by `parseCacheResponse` verdict.
4. Report observed counts and the metrics deltas side by side. Do not state "exactly 1 origin fetch"; state `origin fetches observed: <delta of origin.requests_started>`.
5. Mark the result **not valid** if `requests_total` delta ≠ N (other traffic interfered) or the key was not cold (first response not MISS).
6. Default N = 6 and say why: browsers cap HTTP/1.1 connections per origin at 6, so ten requests are not simultaneous (observed: 1 MISS, 4 COALESCED, 5 HIT). If a true 10-way burst is wanted, use the raw-HTTP harness (`D:\ClaudeTemp\audit\harness.mjs` pattern) or a server-side tool, not the browser.

Backend truth to preserve (reproduced): cold key, 10 raw concurrent requests, 300 ms origin → 1 MISS + 9 COALESCED, 1 origin fetch, `coalesced_waiters` +9. [http]

- Files: `InteractiveConsole.jsx`; reuse `lib/cacheStatus.js` and an existing metrics reader (`useTelemetry`/`lib/schema.js`) rather than a new fetch path.
- Alternative: add a backend `POST /api/v1/diagnostics/stampede` that performs the burst and returns the evidence. Larger and creates a new API; not recommended for the demo fix.
- Regression risk: low.
- Verification: deterministic test = fresh gateway + counting origin with 300 ms delay + unique key; expected: verdict tally sums to N, `MISS` = 1, `COALESCED + HIT` = N-1, origin delta = 1. A second click on a **new** key repeats the result; a second click on the **same** key shows N HIT, 0 origin fetches and is labelled as a warm-key run.
- Acceptance: the card shows only values present in responses or metrics deltas; no literal "COALESCED" or origin count exists in the component source (test: grep the source).

### 4.C HRW placement calculator (F4)

Exact comparison, Rust `RendezvousRouter::calculate_hash` / `route_key` / `get_replicas` (`src/dist/router.rs`) versus `ui/src/screens/Placeholder.jsx`:

| Aspect | Rust (authoritative) | JavaScript (current) |
|---|---|---|
| Hash width | 64-bit FNV-1a, offset `0xcbf29ce484222325`, prime `0x100000001b3` | 32-bit FNV-1a, offset `0x811c9dc5`, prime `0x01000193` |
| Input and order | UTF-8 bytes of the **canonical key**, **then** one byte `node_id` | the string `"<id>:<key>"` (node first, with a colon) |
| Encoding | UTF-8 bytes | UTF-16 code units (`charCodeAt`) |
| Key string | canonical key `"<tenant>:<path>"` (default tenant `__default__`, path with leading `/`, optional `":?"+query`), e.g. `__default__:/catalog/product-492` | the raw text typed, e.g. `/api/v1/cache/catalog/product-492` (includes the route prefix, no tenant) |
| Weight | `hash as f64` (rounds a u64 to a 53-bit mantissa) | the u32 value |
| Tie-break | strict `>` over nodes in configured order, so the first node wins; `get_replicas` uses a stable descending sort | `Array.sort` descending on u32 |
| Node filter | skips Failed/Offline/Suspect/Recovering | none (assumes all up) |
| Output | owner = highest weight | prints all scores and labels the top as owner |

I also confirmed the canonical form empirically: using `__default__:/pf/kN` the predicted primary matched the observed origin-fetch pattern for 9 of 9 keys. [multi]

**Fix.** Implement exactly the Rust algorithm in JS with `BigInt`: `h = (h ^ byte) * prime & mask` per byte, then `h ^= BigInt(nodeId); h = h * prime & mask`; weight = `Number(h)` (JS rounds BigInt to double to nearest-even, the same as Rust's `as f64`); first node wins ties. Build the key as `"__default__:" + path` and say so in the UI (and strip the `/api/v1/cache` prefix, since the router hashes the path as the gateway stores it). Remove the badge text "matching src/dist/router.rs" unless the vectors below pass.

**Preventing silent divergence.** One shared vector file, `tests/vectors/hrw_vectors.json` (key string, node ids, expected 64-bit hash per node as hex, expected primary and replica order). A Rust test (next to the existing `test_fnv1a_deterministic_golden_vectors`) and a `node --test` test in `ui/test/` both read it. Either side changing makes the other fail.

- Alternative: expose the router's decision from the backend (`GET /api/v1/cluster/route?key=`) and have the UI display it. Best for authority, but it is a new API and only works against a running cluster. Recommended as a later step. For the demo, the shared-vector approach is the small, safe one.
- Files: `ui/src/screens/Placeholder.jsx` (or a new `lib/hrw.js`), new vectors file, one Rust test, one UI test.
- Regression risk: low (display only).
- Verification: vectors test on both sides; plus a cluster check: for 9 keys, the owner the UI computes equals the node where the first GET causes exactly one origin fetch (as in my run).
- Acceptance: UI owner equals Rust owner for every vector and for a 5,000-key cross-check; agreement 100 % rather than 33.5 %.

### 4.D Peer behaviour and invalidation after restart (F6, F18, F19, F20)

**What actually happens on GET [code][multi].** On a local miss, if the remote primary is another node, the gateway forwards a read to it over cluster mTLS (`forward_key_over_tcp`, `server.rs:1688-1700`). If the primary answers `HIT`, the body is stored locally as a "peer fill" (`server.rs:1705-1735`) and **no origin request is made**. Observed over 9 keys (first GET on one node, then the other two): when the first GET landed on the primary, origin fetches = 1; otherwise 2 or 3 (the primary was empty when asked, or was asked last). So "distributed GET" is real but read-through only: it works when the primary already holds the key. The responses are still labelled `MISS` (F19).

**Invalidation path [code].** `DELETE` purges locally, then `invalidate_key_across_cluster` (`router.rs:985-1070`) sends to the primary and waits for its ack, then enqueues the replicas for a **bounded** background broadcast (`router.rs:100-232`: up to 4 attempts, 25-200 ms back-off, 150 ms timeouts; then dropped). The receiver applies every authenticated invalidation without sequence rejection (`node.rs:562-640`). Anti-entropy (`start_anti_entropy_worker`, 250 ms) is the repair path. `get_replicas` and `route_key` exclude Suspect/Failed/Offline/Recovering nodes (`router.rs:749-771`). The health-probe worker exists (`start_health_probe_worker`) but `main.rs` starts **only** the anti-entropy worker, so there is no failure detector (consistent with the documented limitation).

**Evidence on the restart observation.**

- Observed once (earlier audit): after restarting node 3, a DELETE issued on node 1 did not reach it for 30 s, while DELETEs from nodes 2 and 3 did. The call still returned 204.
- Reruns **[multi]** (script `restart_exp.mjs`, `restart_exp2.mjs`, `restart_exp3.mjs` in `D:\ClaudeTemp\audit`): healthy cluster all directions OK; control (outage with no sender attempts), treatment (node 1 attempted DELETEs during the outage), heavy warm-up (40 frames, three directions), and a ~27 s outage: **all delivered**, in all three sender directions, at +6 s, +12 s and +30 s.
- Conclusion: the failure is **not reproducible on demand**. I cannot state a root cause. Treat as an intermittent, unexplained observation.

**Hypotheses (each needs validation before any code change).**

| # | Hypothesis | Basis | How to validate |
|---|---|---|---|
| H1 | Stale idle pooled connections (up to 4 per peer, `pool.rs`, idle timeout 30 s) consume the replica fan-out's 4 attempts after the peer restarts | [code] fan-out retries are bounded and a reused connection can be stale; the pool comments say so | Instrument per-attempt outcomes; restart a peer with ≥4 idle pooled connections and issue a DELETE within the idle timeout, before heartbeat pruning |
| H2 | Per-peer active-count exhaustion (`max_total_per_node = 8`) makes `acquire` return `Full` → treated as unreachable | [code] `checkout_or_reserve` / `Full` path | Expose pool counts; run a long soak with churn and check for a count that does not return to idle level |
| H3 | A peer left in `Recovering`/`Suspect` in the sender's table is never selected | [code] state filters; `reconcile_peer_anti_entropy` sets Recovering | **Weakened:** `acquire_connection_to_node` sets Suspect again on a connect failure, and my 27 s outage did not stick. Validate only with state exposure |
| H4 | My original harness had a timing/environment confound | The original sequence was run once, not instrumented | Re-run the exact original sequence with the instrumentation below |

**Smallest useful step (not a rewrite).** Before changing behaviour, make the failure observable:

1. Add a `cluster` block to `/api/v1/metrics`: per peer `state`, pool `idle`/`active`, and the existing broadcaster counters (`sent`, `retried`, `acked`, `dropped`; the atomics already exist in `router.rs`). Label them `measured`.
2. Log, at INFO, each replica fan-out that exhausts its attempts without an ack.
3. Re-run the restart matrix. Only if a failure is then captured, apply the matching minimal fix:
   - H1: when a **reused** connection (`use_count > 0`) fails, retry once with a freshly opened connection without spending an attempt.
   - H2: add a leak assertion to the pool and fix the specific path found.
   - H3: make `reconcile_all_peers` also retry peers in `Recovering`.
   - Separately, consider a final unacknowledged-replica safety net: leave the key to anti-entropy and report `replicas_unconfirmed` in the DELETE outcome. This is a semantic change; it needs the owner's decision, not just a fix.
- Files (instrumentation only): `src/dist/router.rs`, `src/api/server.rs` metrics builder. These files are in the frozen core list (`src/core` is frozen; `src/api` and `src/dist` are not, but changes touching fencing/invalidation require the independent review and a failing-without-the-change test per `CLAUDE.md` section 5) and the work must not be merged from the M3 sandbox.
- Tests to add: restart matrix both directions, repeated invalidation (≥20 keys in a row), reconnect after restart, peer down during DELETE (204 within the timeout, peer converges after restart).
- Acceptance: with the instrumentation in place, 20 consecutive restart cycles, each in three sender directions, deliver every invalidation within 2 s with `dropped = 0`, or the metrics show exactly where one was lost.

**Not claimed:** that three daemons plus mTLS prove distributed correctness. Cross-gateway staleness is unbounded by design today (see `docs/user/limitations.md`).

**Warning text (F20).** Replace the stale string at `src/config.rs:858` with the accurate statement already in `main.rs`. It is code, so it needs an approved change. Verify by starting a cluster node and comparing both outputs.

### 4.E Origin concurrency cap (F5)

**Enforcement path [code].** `main.rs:460-480` reads `PHANTOM_INTELLIGENCE` and, if `PHANTOM_ORIGIN_CAP_MAX` is set, configures the cap on the executor. `origin_cap_enforcing()` (`executor.rs:217`) is true only when the mode is Canary or Active **and** a cap is configured. In Off, Observe and Advisory the permit path (`acquire_origin_permit`) only counts in-flight and never rejects. Requests reach it from `server.rs` (`acquire_origin_permit` at the leader fetch, line 1739, and the bypass/background paths).

**Classification.** This is an intentional quarantine boundary, documented in the code comment, in a startup warning and in `docs/user/limitations.md` ("Treat those modes and the origin concurrency cap as experimental"). It is **not** a defect. It is also not a demo fix: I do **not** recommend enabling Canary to make a demo look protected. The controller's adaptive value remains `NOT_SUPPORTED` (M6 H1).

**What to change for the demo.**

- UI/report wording only: the existing `OriginGate.jsx` already refuses to draw a gate when not enforcing. Keep that. Retract the report claim that the origin screen "shows an active AIMD controller with cap: 10".
- Demonstrate what is real: origin **concurrency observation** (`origin.in_flight`, `dispatched`) and the measured Scenario B results from the Lab (separately labelled, release build, loopback).
- **Future implementation task, separate from the demo:** a reviewed, owner-approved run with the cap enforced, using the Lab harness against a slow origin, to show the cap rejecting beyond N and the AIMD steps. Needs the intelligence mode decision first; out of scope here.
- Verification: with default config, 20 slow distinct keys → origin concurrency = 20, `enforcing:false` displayed as such (reproduced). Do not test enforcement here.

### 4.F Traffic generator (F3, F8)

**Root cause.** All counters are updated on completion; `sent` therefore means completed; nothing bounds in-flight; fetches are not abortable; component state resets on navigation; the default key pattern has no `{id}`, so all requests hit one key and the "sustained load" exercises only cache hits.

**Proposed design.**

```text
state: { dispatched, completed, failed, hit, miss, coalesced, stale, other, inFlight, dropped }
fireOne(): if inFlight >= MAX_IN_FLIGHT { dropped++; return }  // backpressure, default 6
           dispatched++; inFlight++; controller = new AbortController(); track it
           try { resp = await fetch(url, {signal}); verdict = parseCacheResponse(...); completed++; tally verdict; if !ok failed++ }
           catch(e) { if aborted → cancelled++ else failed++ } finally { inFlight--; untrack }
scheduler: drift-corrected setTimeout loop (not setInterval) targeting the requested rate
stop(): clearTimeout; abort all tracked requests (cancelled counter); inFlight → 0
unmount: stop()
default key: /api/v1/cache/item-{id} with an id range
```

- Show five separate numbers: dispatched, completed, failed, in flight, dropped-by-backpressure; plus the verdict tally from `parseCacheResponse`. Rename the "Dispatched" tile to match its meaning.
- Navigation: either lift the generator into app-level state so it survives navigation, or stop it on unmount and say so in the UI. I verified that today unmounting stops the timer cleanly (0 requests in the next 3 s) and that rapid start/stop leaves no stray requests; keep that behaviour.
- Browser limit: the browser cannot generate more than ~6 concurrent connections to one origin, so this tool cannot stress an origin. Say so in the UI; use the Lab or a raw-HTTP tool for load.
- Files: `TrafficGenerator.jsx` (+ `lib/cacheStatus.js`).
- Alternative: move load generation to the Lab/server side. Larger; later.
- Regression risk: low.
- Verification: unit tests with a fake `fetch` (counters add up: dispatched = completed + failed + cancelled + inFlight; inFlight never exceeds MAX); abort-on-stop test; browser test against the counting origin: UI hit/miss tallies equal the gateway's metrics deltas.
- Acceptance: after any run, UI hits + misses (+ coalesced + stale) equals the gateway `requests_total` delta, and each matches the backend per-category delta.

### 4.G History persistence (F9)

**Root cause.** `useHistory` loads the stored series at mount, then the effect runs with `payload === null` and replaces both the state and the stored copy with an empty series. The key `phantomcache.history.v1` also does not include the gateway address.

**Fix.** Only clear on a **gateway change** (compare the stored `gatewayUrl` with the current one), not on a null payload at start; store `{gatewayUrl, savedAt, series}`; discard series older than a bound (for example 5 minutes) and mark the gap so a sparkline does not draw a line across it. Also fix the guard test: either add `hooks/useHistory.js` to the allow-list in `ui/test/func.gateway.test.mjs` **by owner decision**, or move storage access into the already-allowed module `lib/gatewayConfig.js`. Which is right is a design decision (the test intentionally restricts storage).

- Verification: unit test for the three cases (same gateway reload keeps points; different gateway drops them; stale beyond the bound drops them); browser: reload keeps the sparkline.
- Acceptance: stored length after reload equals the length before (within one sample).

### 4.H Remaining lower-severity items

| ID | Fix | Files | Verification |
|---|---|---|---|
| F10 | Keep the original title ("Not reported") or say "Not reporting yet"; show each metric's own reason; only use "idle" wording for the one specific missing-window-samples reason | `Overview.jsx` | unit test over the reason mapping |
| F11 | Rewrite `scripts/live_interactive_demo.mjs` to assert: fail the process on any missing button, compare displayed verdicts with backend counter deltas (via `/api/v1/metrics` and an independent origin counter), cold-key stampede, DELETE then GET, exit non-zero on failure; take paths from arguments, not a hard-coded `.gemini` directory | `scripts/live_interactive_demo.mjs` | run against a counting origin; deliberately break a label and confirm the script fails |
| F12 | See 4.A (DELETE wording; no pill on errors) | `InteractiveConsole.jsx` | unit test |
| F14 | Remove the 2 % floor; instead print the exact value text, and (optionally) draw a minimal non-zero tick that is visibly not a percentage | `Charts.jsx` | unit test: 494 B / 256 MiB draws width ≈ 0 and shows "494 B" |
| F15 | Run `npm run check:contrast` (not run) and fix the console panel title colour using the design tokens | `InteractiveConsole.jsx`, `tokens.css` | contrast check passes; no inline hex colours (guard test) |
| F16 | Label Windows "Process working set (RSS-equivalent)" | `server.rs` label/UI catalog | wording check |
| F17 | Read `VmRSS` from `/proc/self/status` (kB) instead of `statm` × 4096, or use the system page size | `server.rs` (linux fn) | cannot be tested on this Windows host; **[not run]**, needs a Linux CI job |
| Presets | The presets only fill the address field; they use `localhost` and fixed ports 8085-8087 and do not connect. Either rename them "Fill address" or generate them from configuration. Note the CORS allowlist must contain the page origin for whichever host spelling is used | `linkUi.jsx` | browser: click preset, press "Use for this visit", header shows the new gateway |
| F20 | See 4.D | `config.rs` | startup output |

## 5. Severity reassessment

I applied three tests: does it make a displayed claim false, does it affect data or consistency, and how easy is it to hit.

- **P1 (F1, F2, F4):** each makes a primary demonstration action display something the backend contradicts, and F4 carries a false provenance badge. They are not P0: no data is corrupted, the backend is correct, and nothing is unsafe.
- **P2 (F3, F6, F7, F8, F9, F11, F19, F21, red UI tests):** weakens the demo or its verifiability. F6 stays P2 only because it is unreproduced; if instrumentation captures a lost invalidation it becomes P1 (a consistency problem).
- **P3 (F5 wording, F10, F12, F14-F17, F20, presets).**
- **No P0.** The previous report's P0 for the hard-coded verdict is lowered to P1 for the reason above; I do not retain a severity merely because it appeared before.

## 6. Prioritised work packages

| WP | Scope | Depends on | Size | Independent? |
|---|---|---|---|---|
| WP1 | `lib/cacheStatus.js` + its unit tests; adopt in console and generator labels | none | S | yes (frontend only) |
| WP2 | Console: cold-key stampede derived from responses and metrics deltas; DELETE wording | WP1 | M | frontend only |
| WP3 | Traffic generator rewrite (counters, backpressure, abort, key pattern) | WP1 | M | frontend only |
| WP4 | HRW: BigInt implementation, shared vector file, Rust vector test, UI test | none | M | Rust test + frontend; vectors file shared |
| WP5 | History persistence fix and storage-guard resolution | owner decision on the guard | S | frontend only |
| WP6 | Restore the red UI guard tests (network module rule, colour tokens, fixed numbers, placeholder/overview rules): route fetches through `lib/` or amend the guards by decision; move colours to tokens | WP1-WP3 | M | frontend only |
| WP7 | Assertion-based demo script (replaces `live_interactive_demo.mjs`) | WP1-WP3 | M | frontend + scripts |
| WP8 | Cluster observability: `cluster` metrics block + failed-fan-out logging, then the restart matrix test | none | M-L | backend + test; needs independent review (invalidation) |
| WP9 | Wording-only items: origin cap statement in the report, config warning (F20), presets, capacity bar, RSS label | none | S | mostly frontend; F20 is backend text |
| WP10 | Optional later: backend `PEER` status value, route-decision API, Linux RSS | WP4/WP8 | M | needs API decision |

**Suggested order to restore credibility with the least change:** WP1 → WP2 → WP3 → WP6 → WP7 (UI becomes truthful and green), WP4 in parallel, WP5 and WP9 as small follow-ups, WP8 as the separate backend track. WP8 is the only package that touches the invalidation path, so it carries the independent-review requirement. Exit criterion for the whole set: the UI test suite is green, the new assertion-based browser run passes against a fresh gateway and counting origin, and the HRW vectors pass on both sides.

## 7. Verification strategy

| # | Level | Test | Expected observable result | Executed in this audit? |
|---|---|---|---|---|
| 1 | Unit (frontend) | `parseCacheResponse` table, generator counters with fake fetch, history cases, reason mapping | assertions as in 4.A/4.F/4.G | **[not run]** (nothing implemented) |
| 2 | Backend | single-flight: cold key, N concurrent, counting origin | 1 MISS + N-1 COALESCED, origin delta 1, `coalesced_waiters` +N-1 | Reproduced by raw HTTP **[http]** (not as a repository test) |
| 3 | Integration (multi-process) | three gateways, mTLS | health OK; no client cert and a rogue-CA cert rejected; TLS 1.3 | **[multi]** executed |
| 4 | Restart/reconnect | restart matrix with instrumentation | all directions deliver ≤ 2 s; counters show 0 dropped | Executed without instrumentation (3 reruns, passed); instrumented version **[not run]** |
| 5 | HRW vectors | shared JSON read by Rust and JS tests | 100 % agreement | JS-vs-Rust comparison on 5,000 keys **[measured]** (33.5 % agreement today); vector tests **[not run]** |
| 6 | Generator | accounting and cancellation tests | dispatched = completed + failed + cancelled + inFlight; inFlight ≤ cap; stop aborts | Behaviour observed in browser (clean stop, clean unmount) **[browser]**; unit tests **[not run]** |
| 7 | Browser | assertion-based walkthrough on a fresh stack | displayed verdicts equal backend `X-Cache-Status` and counter deltas | AG's script run not repeated; my own browser runs **[browser]** found the defects above |
| 8 | Regression | existing UI suite and Rust tests | UI suite green; Rust baseline unchanged | UI suite run: **473 pass / 10 fail [unit]**; `cargo test` **[not run]** |

Evidence classes are kept apart: code inspection, unit-test evidence (the existing UI suite only), raw-HTTP integration, real-browser, multi-process loopback. A successful build is not acceptance evidence, and none of the above is a performance measurement.

## 8. Risks, dependencies and unresolved questions

- **F6 unresolved.** If a lost invalidation can occur, it is a consistency problem independent of the UI. Do not close it on the strength of three clean reruns.
- **Peer-fill freshness** (created_at = now, ttl 60 s): unverified; may lengthen effective staleness. Needs its own test before any bound is stated.
- **Storage guard** (WP5/WP6): whether to widen the allow-list is a project-owner decision; I did not assume it.
- **F13:** I did not test AG's binary. Rebuild and retest from the final tree.
- **Backend `PEER` status** changes an API contract: owner decision.
- **Frozen/reviewed areas:** fencing, epochs and invalidation changes need an independent review and a test that fails without the change (`CLAUDE.md` section 5). The M3 sandbox must not be merged to satisfy this report.
- **Freeze hashes and quarantine:** M6 H1 remains `NOT_SUPPORTED`. No intelligence mode or experimental flag was changed, and no research-freeze file was touched.

## 9. What was not tested or could not be verified

- The 56 research-freeze hashes: no manifest was located; **not verified**.
- AG's activity transcript and its original build-exit-code-1 record were not available; the claim is unreconciled.
- AG's running binary (00:51 build) was not exercised.
- `cargo test`, `cargo clippy` and any Rust unit test were **not run**.
- `npm run check:contrast`, `check:a11y`, `check:axe` and `check:forbidden` scripts were **not run** (they may write evidence files; I only ran `node --test test`, which left the working tree unchanged).
- Linux behaviour (page size, `/proc` RSS) and non-Windows FFI targets: not testable on this host.
- Enforced origin cap and AIMD behaviour: deliberately not tested (quarantine).
- The F6 original observation: not reproduced; root cause unknown.
- Peer-fill freshness and the effective staleness of peer-filled entries: not tested.
- Whether any UI test failure pre-dates AG's work: not separated (no `HEAD` comparison).
- No performance claim is made anywhere in this report.

## 10. Reproduction material (kept outside the repository)

`D:\ClaudeTemp\audit\`: `harness.mjs`, `h2.mjs`, `h3.mjs` (raw HTTP), `browser.mjs`, `browser2.mjs` (Chrome via the repo's `cdp-lib.mjs`), `cluster.mjs`, `cluster2.mjs`, `cluster3.mjs`, `restart_exp*.mjs`, `peerfill.mjs`, `hrw_cmp.mjs`, `origin.mjs` (counting origin), `static.mjs`, certificates in `certs\`, outputs `*.out`, UI test log `ui_tests.out`. None of it is in the working tree.
