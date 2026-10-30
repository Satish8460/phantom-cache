# V5 — Demo Center UX, event/API requirements and flagship storyboard

**Date:** 2026-10-30. **Role:** product-discovery research (agent V5 of 5). **Status:** PROPOSAL. Nothing here is implemented unless a table says "Already implemented", and even then no part of the UI has been verified in a real browser by this pass (`docs/design/phantom_lab_ui.md` section 7 and section 9 say the same of the existing work). No code, UI, build or experiment was touched or run. V1/V2 own intelligence, V3 topology/identity, V4 security/failure scenarios; where I depend on them I say so and do not decide for them.

**Evidence tags used:** [code] verified by reading the cited file; [doc] stated by a repository document, not re-verified; [gap] I looked and did not find it (grep/read coverage stated; absence of a hit is not proof of absence); [prop] my proposal; [hyp] hypothesis.

---

## 0. Summary of the position

1. The repository already contains one honest, working pipeline: **seeded run -> real gateway child process on loopback with a scripted degrading origin and open-loop load -> persisted events -> evidence with tiers, notice and limitations** (Scenario B through the Lab API and `#/lab`). That is the spine to build on. [code] `src/api/lab_api.rs`, `src/lab_live/*`; [doc] `docs/design/phantom_lab_ui.md`.
2. What does **not** exist: any per-request or per-decision event stream, any live prediction, any node/topology stream with real peers, any tenant-attack scenario, any flash-crowd generator against the real gateway, any pairing/multi-device mechanism, any report generator that is not just "download `summary.json`". The only live feed is the 100 ms aggregate SSE `/api/events?schema=2` [code `src/api/server.rs:959-968`] and a polled ring of the last 100 diagnostics events [code `server.rs:2154-2171`; names found: `CacheHit`, `CacheStaleServed`, `OriginFetchSuccess`, `OriginFetchNon200`, `OriginFetchFailed`, `CacheInvalidated`, `src/intel/diagnostics.rs`].
3. The user's six-stage story is a good *marketing arc* but a weak *causal chain*, and two of its stages (B predictive demand, D tenant-isolation attack) cannot be shown as live capabilities today. I recommend a **re-sequenced flagship, "Launch Day"** (section 2), in which every stage is either real today, real after a small named build item, or explicitly shown as a labelled replay. Stage B is demoted to an offline/replay act until V1/V2 deliver a verified predictor, and a negative result (Scenario B's recorded `TRADE_OFF_FAILURE`) becomes a *feature* of the demo rather than something to hide.
4. The Demo Center should be **one product shell with a single timeline-and-evidence model**, not "seven playgrounds". A *scenario* is a data object (seed, workload, fault schedule, topology, arms) that any view can render; playgrounds become *lenses* on the same running scenario.

---

## 1. Critique of the user's story and of the seven playgrounds

### 1.1 The six stages as proposed

| Stage | Verdict | Why (evidence) |
|---|---|---|
| A. Normal operation | Keep, but make it a *measured baseline*, not a warm-up screen. | It is the only stage whose numbers every later stage is compared against. Real path exists: Scenario B's `warmup`/`healthy` phases on the real gateway [code `ui/test/fixtures/lab/events_sb.sse`, phases `warmup`, `healthy`]. Needs a longer, audience-paced "steady" phase than the 1-60 s bounds the API enforces per phase [doc `lab_api.md` Limits]; the bound is fine, the *pacing* is a UX matter. |
| B. Predictive demand | **Not demonstrable as a live capability.** | PIE is a legacy threshold stub, `pie: None` in `main.rs`, hard-coded `expected_utility = 85.0` [CLAUDE.md 6b; `docs/research/pie_audit.md`]; the intelligence engine is not in the serving path (UI Connection panel says so: `phantom_lab_ui.md` section 1). No forecast is emitted anywhere [gap: grep of `src/api/server.rs` and `src/api/lab_api.rs` event names found no prediction event]. Showing a "prediction line" now would be exactly the decoration the brief forbids. It can be shown only as (i) an *offline replay* of a T1 simulated run with the predictor evaluated post hoc, labelled SIMULATED, or (ii) after V1/V2 produce a verified shadow predictor. Also causally weak: prediction only matters *before* the surge, so it must precede C and its value is only visible if C is then run twice (with and without acting on it). |
| C. Flash crowd | Keep as the **centre of gravity**; it is where real, tested mechanisms (single-flight coalescing, origin cap, stale-if-error) matter. | Coalescing and the origin cap are real [CLAUDE.md 5, `scenario_B_origin_slowdown.md` section 3]. A flash-crowd *workload* exists only in the T1 simulator (`W-FC`, `src/lab/workloads.rs:192`) [code]. The real-gateway load generator is Zipf-only open-loop (`Workload { seed, n_keys, zipf_alpha, rate_rps, duration_ms, arrival, path_prefix, client_deadline_ms }`, `src/lab_live/loadgen.rs:20-30`) [code]: **no rate-step/hot-set shape on the real gateway**. The `FLASH_CROWD` button on the old dashboard only flips a simulation flag (`server.rs:1086-1088`) [code]. So "flash crowd on the real gateway" = Proposed (small loadgen addition). |
| D. Tenant-isolation attack | **Reframe; do not stage as an "attack that the system defeats".** | What is verified: tenant is part of the cache key; in secure mode a caller-supplied `x-tenant-id` is ignored (`server.rs:1212, 1967`); `invalidate_tenant` exists (`cache.rs:609`); tenant-epoch overflow falls back to global (CLAUDE.md 5). What I could **not** find: any per-tenant quota, per-tenant byte budget or noisy-neighbour control (grep for `quota`, `per_tenant`, `tenant_limit`, `tenant_bytes` in `cache.rs`, `server.rs`, `config.rs` returned nothing) [gap]. Capacity and eviction appear shared across tenants [hyp]. A tenant "flood" may therefore *evict another tenant's hot keys* - a legitimate, honest lab result, but a failure demo, not a defence demo. Safe, true sub-demos: (a) forged-tenant-header rejection in secure mode; (b) tenant-scoped invalidation not touching other tenants; (c) a *measured* noisy-neighbour test whose outcome, positive or negative, is shown. Which of these is staged is V3/V4's call. |
| E. Node failure | Keep, but only in a form the code supports, and label what is real. | The old "NODE_FAILURE" button sets one of three in-process demo nodes to Failed (`server.rs:1089-1091`) - simulation [code]. Real cluster exists only in the mTLS configuration, remote peers are address-only descriptors, effectiveness of anti-entropy not established, invalidation durability is bounded to the issuing process staying alive [CLAUDE.md 6; `docs/user/limitations.md`]. Real multi-process node failure on one machine is Proposed (the Lab API already supervises gateway child processes and a Windows job object; `lab_api.md`). **GET is served from the local cache only**, so "failover" for reads is not a thing to claim; what can be shown is invalidation delivery and repair under a killed peer, with the durability limit stated. |
| F. Evidence report | Keep, and **promote to the thread that runs through every stage**, not a final slide. | The evidence model (tiers, binding notice, limitations verbatim, artifact SHA-256) already exists for Scenario B and M6 [doc `phantom_lab_ui.md` sections 4-5]. A report should be accreting as the demo proceeds. Today there is no report generator beyond artifact download (`src/lab/report_gen.rs` exists but I did not verify what it emits [gap]). |

Structural problems with the sequence itself:

- **A->B->C->D->E->F is a feature tour, not a story.** B (prediction) and C (surge) are linked, but D (a second, unrelated actor) and E (an infrastructure fault) don't follow from C. An audience remembers causes, not lists.
- **Four of six stages claim capabilities that cannot be live today** (B, D, E-as-cluster, and the "automatic" half of C). A demo whose first run contains three "unavailable" screens teaches the wrong thing.
- **The strongest real asset - an honest negative result - has no place in it.** Scenario B's recorded outcome for both capped arms is `TRADE_OFF_FAILURE` (static cap 24: peak origin concurrency about 39 -> 24, -38.3%, at a 4.95 percentage-point client-success cost; the ~12-cap arm: -64.0% at a 17.1-point cost) and "adaptive-control merit" is **not established** [doc `docs/demo/scenario_b_interpretation_notice.md`]. This is the demo's credibility. A story built to "win" in every stage would contradict it.

### 1.2 The seven playgrounds

The brief lists seven playgrounds (the user's list was not part of the files I read; I treat them as seven separate interactive sandboxes). General critique, independent of which seven:

1. **Seven disconnected sandboxes recreate the problem the repository already has**: the old dashboard mixed live and synthetic values (CLAUDE.md 6; `ui_metric_provenance.md`), and `#/simulation` is a bannered island (`ui/src/screens/Simulation.jsx`). Each additional island needs its own provenance policy, reset logic, accessibility pass and test fixtures. Cost scales linearly; trust does not.
2. **Each playground should be a lens, not an app.** All of them can be expressed as `scenario = (topology, workload, fault schedule, policy arms, seed)` plus a *view set*. One scenario runner, one event model, one evidence model.
3. **Sandboxes invite fabricated "interactive" results.** Free-form sliders that update a chart without a backend run are the classic way a demo becomes decoration. Rule [prop]: a control may only (a) edit a *scenario draft*, which is validated by the backend, or (b) send a *live action* the backend executes and acknowledges. A chart never moves because of a client-side computation.
4. **Only some playgrounds can be real now.** Classification: anything that is "start a seeded run and read the result" is already there (T1 matrix, Scenario B). Anything that needs live per-request events, live topology or live decisions is not.

Recommendation: reduce the seven to **three lenses on one scenario** (Traffic/Origin, Topology, Decisions/Evidence) and **one launcher** (scenario library with a "what is real here" badge per scenario).

---

## 2. Recommended flagship journey: "Launch Day"

**Premise.** A product page goes viral. One continuous scenario, one seed, six beats, each caused by the previous one. The audience always knows *why* the screen changed.

| # | Beat | Causal link from previous beat | Maps to user's stage |
|---|---|---|---|
| 0 | **Preflight** (presenter only) | - | new |
| 1 | **A quiet morning** (measured baseline) | - | A |
| 2 | **The link goes viral** (flash crowd, coalescing) | the baseline gives the "before" | C |
| 3 | **The origin starts to struggle** (origin degrades under the surge; cap and stale-if-error act; trade-off shown honestly) | surge load is why the origin slows (in the scripted origin it is a schedule, not load-dependent; this is stated on screen, see 3.3) | Scenario B |
| 4 | **Someone must purge** (urgent invalidation during in-flight fetches; generation fencing; cluster delivery bound) | the incident forces a content fix | E (invalidation half) |
| 5 | **A node goes away** (kill one gateway process; what the rest does; limits) | the ops team is mid-incident | E |
| 6 | **What we can and cannot claim** (evidence report assembled from beats 1-5, tiers, limitations, falsifiers) | everything above | F |

Where the user's other stages go:
- **B (predictive demand)** becomes **Act 2.5 "Could we have seen it coming?"**, a *replay* inserted after beat 2 only when a verified predictor exists (V1/V2). Until then the slot shows a single honest panel: "No predictor is part of the serving path. Offline evaluation: see Evidence" with the retained M6 verdict snapshot (`NOT_SUPPORTED`, tier Retained: simulated) [doc `lab_api_contract.md` section 2]. Status: Unsupported live, Partially (offline) via evidence.
- **D (tenant isolation)** becomes an **optional side act "A noisy neighbour"** (alternative demo 3 below) because it does not fit the incident causality and its outcome is not known.

### 2.1 Per-beat storyboard

For every beat: presenter script; what the audience sees; backend events driving each visual; provenance; live failure handling; honest gap. "Prov." uses the existing vocabulary: MEASURED (loopback, single host), SIMULATED, DERIVED, INFERRED (observed by the page), UNAVAILABLE (`ui/src/lib/provenance.js`, `labProvenance.js`).

#### Beat 0 - Preflight (presenter view only)
- **Script:** "Before we start: here is exactly what is real on this machine." Presenter checks the capability checklist; the audience sees only a title card with the scenario name, seed, build profile and mode label.
- **Sees:** checklist: Lab API enabled; scenario_b available (needs a gateway binary; `capabilities.kinds.scenario_b.available` + `reason`); build profile (`driver_build_profile`); `security_mode`; max one active run; sealed seeds not exposed. Mode banner "LOOPBACK, ONE HOST, SCRIPTED ORIGIN" always visible.
- **Events:** `GET /api/v1/lab/capabilities` [code `lab_api.rs:1177`]. Prov.: configuration, verbatim reasons.
- **Failure/recovery:** if the build is debug, the label says EXPLORATORY and the whole demo is branded so (rule already in the API: "Debug builds are always EXPLORATORY") [doc]. If the Lab API is off the Demo Center shows the "switched off" panel with the variable name, nothing else.
- **Gap:** a "dry run / rehearsal" that starts the scenario without recording as evidence does not exist [gap]. Proposed.

#### Beat 1 - A quiet morning (baseline)
- **Script:** "Steady, skewed traffic. Most requests are hits. The origin barely notices." Presenter names the three numbers to remember: hit ratio, client p99, peak origin concurrency.
- **Sees:** traffic rate, hit/miss/coalesced split, latency distribution, origin in-flight gauge, one cache-decision trace for a sampled request.
- **Events:** (today) `progress` events with `phase: warmup/healthy` [code fixture]; the 100 ms `/api/events?schema=2` aggregate (hits, misses, coalesced waiters, evictions, latency percentiles, `metrics.origin_cap`) [code `server.rs:959`; fields from `ui_metric_provenance.md`]. (needed) per-request decision events, section 4.
- **Prov.:** counters MEASURED; hit ratio over a window DERIVED (server-side, window stated; today `hit_ratio_cumulative` is cumulative, R-UI-03 is open) [doc]. Per-outcome counts per window and latency histogram buckets are **UNAVAILABLE** now (R-UI-09/10 open).
- **Failure:** stream stale (no frame for >25 s UI window; the existing stale/reconnecting banners apply) -> the chart freezes with a "last known" label, never extrapolates.
- **Gap:** the Lab run is a closed batch run (n repetitions, arms interleaved, 1-60 s phases), not an open-ended "run as long as the presenter likes" session. Live exploration against a long-lived gateway needs a *session* kind (Proposed).

#### Beat 2 - The link goes viral (flash crowd)
- **Script:** "Ten times the traffic on a handful of new keys. Watch how many of those become origin requests." Point at coalescing: N concurrent misses for a key produce one fetch.
- **Sees:** request rate step; miss burst; *waiters per in-flight fetch* gauge; origin requests/s staying far below client requests/s; a "this key: 412 waiters -> 1 origin fetch" callout from a real fetch.
- **Events:** needs `coalesce` events (leader/waiter counts) and `origin_fetch` events. Today only the aggregate `coalesced_waiters` counter exists [code `diagnostics.rs` counters list]; waiter-limit rejections are counted (`waiter_limit_rejections`) and surface as `X-Cache-Error: WaiterLimitExceeded` [code `server.rs:2209`].
- **Prov.:** counts MEASURED; "origin requests avoided" DERIVED (= waiters - fetches, formula shown). **No counterfactual "without Phantom" line** unless a real BASE-like arm ran with the same seed; the old dashboard's multiplied "Phantom OFF" was fabricated and must not return [doc `ui_metric_provenance.md` section 2].
- **Failure:** if the load generator cannot keep the open-loop schedule (send lag), say so: `send_lag_ms` exists per request [code `loadgen.rs:94`]; show "offered load not achieved" instead of the target rate. If the host is saturated (loopback, shared CPU), the mode banner already says single-host; add a visible "host contention suspected" flag when lag exceeds a documented threshold [prop].
- **Gap:** real-gateway flash-crowd shape = **Proposed** (add a rate-step/hot-set arrival process to `loadgen`; T1 `W-FC` is a template). Per-key waiter detail = Proposed.

#### Beat 3 - The origin starts to struggle
- **Script:** "Now the origin slows. Caching can't help with *different* keys. The cap protects the origin - and costs us something. Here's the price." The presenter *reads out the cost* before the benefit.
- **Sees:** origin latency/concurrency overlay with the cap line; shed (503) fraction; client success rate; per-arm side by side (BASE vs STATIC-CAP; AIMD shown with its documented caveat); phase band (healthy / degraded / recovery); the binding notice expanded before any chart.
- **Events:** exactly the existing Scenario B stream and summary [code `lab_api.rs`, `ScenarioBResult.jsx`]: `progress{arm, rep, phase, done, total, gateway_pid, tier}`, `result`, `summary.json` (`arms.*`, `paired_vs_base`, `verdicts`, `mechanism_h_b1`), plus gateway `metrics.origin_cap` in the aggregate stream.
- **Prov.:** per-arm values MEASURED loopback; paired differences/intervals DERIVED; verdict DERIVED (mechanical rule); shed per phase and success-only latency **UNAVAILABLE** with reasons (API gaps 1 and 2, `phantom_lab_ui.md` section 6).
- **Honest result to show:** both capped arms `TRADE_OFF_FAILURE`; mechanism H-B1 passes (origin peak exactly 24 in all 10 static runs); AIMD effectively a static cap of ~12; stale-serving was not exercised [doc notice]. The demo script must say "the cap did what it promises mechanically, and the trade-off failed the pre-set criterion". No spin.
- **Failure/recovery:** a run takes real time (repetitions x 4 phases) - the presenter can start it ahead of time and the Demo Center plays the *retained* result with a visible "RECORDED RUN <id>, not live" tag (Retained: measured loopback). Cancel exists and yields a partial-flagged result [doc]. If the gateway child fails to start, the run is `failed` with the error code; never shown as completed.
- **Gap:** the origin slowdown is a *schedule*, not load-dependent latency (the origin-p99 clause is untestable on the scripted origin) [doc notice qualification 4]. The "surge causes the slowdown" link in the story is therefore **narrative, not mechanism**, and the screen must say "scripted degradation". Whether to build a load-dependent origin model is an owner decision (section 9).

#### Beat 4 - Someone must purge (invalidation under load)
- **Script:** "Wrong price on the page. We purge a key while 400 requests are in flight for it." Show that the late response does not overwrite the purge (generation fencing).
- **Sees:** a key timeline: fetch started (gen 7) -> DELETE (gen 8) -> fetch completes -> **discarded, not stored** -> next request fetches fresh. Cluster addition: which peers acknowledged, with the 150 ms primary leg and async repair noted.
- **Events:** `CacheInvalidated` exists in the diagnostics ring (key fingerprint, `op_id`, `op_gen`, outcome, duration) [code `diagnostics.rs`, `server.rs:2096`]; `stale_writes_prevented` and `stale_ops_ignored` counters exist [code]. A per-key causal chain (fetch start/finish/discard) is not emitted as a stream [gap].
- **Prov.:** MEASURED counters; per-peer acks only if cluster mode and then INFERRED/MEASURED per R-UI-17/18 (open).
- **Failure/recovery:** 204 on DELETE is *not* proof the object is gone everywhere [doc `limitations.md`]; the screen must carry that sentence. If cluster is disabled the panel is "single-node: no peers" (UNAVAILABLE), not an empty success.
- **Gap:** fencing demo with a deterministic race needs a scripted slow origin response for one key (the scripted origin has per-request recording; per-key delay injection is not verified [gap]). Proposed. V4 owns the exact security/failure scenario list; this beat should be reconciled with theirs.

#### Beat 5 - A node goes away
- **Script:** "We kill one gateway. Reads from the others are unaffected - they never depended on it. What did depend on it?" Honest answer: pending invalidations the dead issuer had not yet delivered.
- **Sees:** topology with a node turning grey, then the remaining nodes' invalidation lag and anti-entropy activity; a plain-language box: what is guaranteed and what is not.
- **Events:** node health (new), invalidation ledger (new). Today: the aggregate stream's `nodes[]` carries `state` with `state_src` and, in the demo configuration, three in-process demo nodes whose CPU/memory are literals [code `server.rs:1698-1706` as cited by `ui_metric_provenance.md`; **not re-verified by me**]. The existing Cluster screen is an honest placeholder [code `Placeholder.jsx`].
- **Prov.:** only what a real process reports; peer suspicion INFERRED; CPU/memory UNAVAILABLE until real OS counters exist.
- **Failure:** if multi-process spawn is unavailable (non-Windows: no job-object tie, "a hard-killed API process can leave a gateway child behind" [doc `lab_api.md`]), the beat is skipped with "not supported on this platform", not simulated.
- **Gap:** multi-node local cluster as a *Lab run kind* = Proposed; mTLS material generation for a demo cluster is an unresolved packaging problem (insecure mode has no peer addresses [doc `limitations.md`]). Classified Experimental until V3 decides the topology.

#### Beat 6 - What we can and cannot claim (evidence)
- **Script:** "Here's the report the demo wrote about itself." Walk the claim table: claim, evidence, tier, N, interval, limitation, falsifier.
- **Sees:** a report assembled from the beats' runs: per claim a row; negative and inconclusive rows styled identically to positive ones (existing rule, `lab_api_contract.md` section 6); the binding notice reproduced verbatim; artifact hashes; seed and build identity; "reproduce this" command (the UI already generates the equivalent `phantom-lab` command) [doc `phantom_lab_ui.md` section 1].
- **Events/API:** `GET /evidence`, `/evidence/{id}`, `/runs/{id}/artifacts` [code]. Report assembly itself = new (section 4.5).
- **Failure:** if a beat's run is missing or cancelled, its row reads "not run" / "partial", and the report header says "incomplete scenario". The report is never rendered as complete when a beat was skipped.

### 2.2 Alternative short demos (3-5 minutes, stand-alone)

1. **"Same seed, different answer" (reproducibility, 3 min)** - start the same Scenario B spec twice or open two retained runs; show manifest equality of seed/workload and the run-to-run spread. Teaches variance and the 10-repetition rule. Status: Partially implemented (evidence viewing and run launch exist; side-by-side comparison of two runs is Proposed).
2. **"Who pays for protection?" (trade-off, 4 min)** - only the Scenario B result, read as a cost/benefit ledger: origin peak down X, clients failed up Y, per phase. Already implemented for the evidence view; per-phase shed is UNAVAILABLE and shown so.
3. **"A noisy neighbour" (tenant fairness, 4 min)** - two tenants share a gateway; one floods; measure the other's hit ratio and p99 before/during/after. **Outcome unknown** [hyp: shared capacity makes it degrade]. Presented as an experiment with a pre-stated pass criterion. Status: Proposed; needs tenant-labelled load generation (loadgen has no tenant field [gap: `Workload` struct above]). Coordinate with V3/V4.
4. **"The thing we got wrong" (honest-failure, 3 min)** - show the dated correction notice and what changed (the repo already contains `CORRECTIONS.md` and the Scenario B interpretation notice): what was claimed, what verification found, what is permitted now. Status: documentation exists; a UI presentation is Proposed. Strong trust-builder, cheap.

---

## 3. Demo Center information architecture and UX

### 3.1 One product, one model

**Object model [prop]:**
- **Scenario** (versioned data): topology, workload(s), fault schedule, arms, seed, duration/phase plan, tier(s), *capability requirements* (e.g. "needs real multi-process cluster"), narration beats.
- **Session** (a run of a scenario): id, role views attached, timeline cursor, mode (LIVE / RECORDED REPLAY / REHEARSAL), evidence links. A Lab `run_id` is the existing precursor [code]; a session may own several runs (one per arm/beat).
- **Evidence** (immutable): what the existing `/evidence` already is.

Navigation (extends the existing screen registry `ui/src/lib/routes.js`; no separate app):

```
Demo Center  (#/demo)
 |- Scenarios      library cards: name, one-line claim, "what is real here" badge, tier, duration, status
 |- Stage          the running session: timeline + the three lenses
 |    |- Traffic & Origin   (lens 1)
 |    |- Topology           (lens 2)
 |    |- Decisions & Trace  (lens 3: request tracing, cache-decision explanation, predictions, actions)
 |- Compare        two sessions/runs/arms side by side
 |- Evidence       existing #/lab/evidence, report builder
 |- Runs           existing #/lab/runs (history, artifacts)
Live gateway (existing five screens) stay as the "operator console"
```

`#/lab/*` stays addressable (deep links in retained notes); Demo Center is a *front door* over it, not a rewrite. Existing code to reuse: `ConnectionPanel`, `useRunStream` (reconnect, gap detection, `Last-Event-ID`), `ProvenanceBadge`, evidence views, chart components with "view as table" [code `ui/src/screens/lab/*`, `components/*`].

### 3.2 Single machine vs multi-device without changing the experience

Principle [prop]: the UI only ever talks to **one origin: the Demo Center service (the gateway with Lab API)**. Devices are *views*, not participants in the experiment. Therefore "multi-device" means "several browsers attached to one session", and the experience is identical whether they sit on one laptop (several windows) or on a phone and a projector.

- **Roles of devices:** Presenter (controls), Audience (read-only big screen), Observer (read-only, personal device, may explore), Operator-only token never leaves the presenter.
- **Same URL shape:** `#/demo/stage/<session_id>?view=presenter|audience|observer`. Switching views never changes data, only chrome and controls.
- **What multi-device must not change:** the experiment topology. Topology (single process vs several local processes vs real remote nodes) is a *scenario property* with its own badge (LOOPBACK / SINGLE HOST MULTI-PROCESS / MULTI-HOST), displayed identically in every view [prop]. Multi-host is **not run today**; its tier label does not exist yet (`MEASURED_LOOPBACK` is the only measured tier) [doc]. A new tier would need an owner decision.
- **Pairing UX [prop]:** presenter clicks "Share this session" -> a short-lived, read-only *view ticket* (random, single session, Observer-equivalent, expires in minutes) is shown as a QR/URL plus a short code. The audience device opens it; no operator credential is ever in a URL (the existing rule: no credentials in URLs, token only in a header and in memory [doc `phantom_lab_ui.md` section 2]). Because browsers' `EventSource` cannot send headers, a ticket exchanged once for an in-memory header token fits the existing "fetch streaming" design (`sse_auth_options.md` option D) [doc]. Pairing is **Proposed** and security-sensitive (V4 to review): the ticket endpoint, TTL, binding to session, revocation, and whether any non-loopback exposure is acceptable. Anonymous dev mode is loopback-only by design [doc], so pairing across devices *requires* JWT mode or a new tightly-scoped ticket; this is the single largest architectural dependency of the multi-device story.
- **Clock handling:** every event carries server time and a monotonic session offset; views render the timeline from those, never from the local clock, so a laptop and a phone agree (existing code already surfaces client-minus-server offset as INFERRED [code `Overview.jsx` ConnectionPanel]).

### 3.3 Presenter, audience, observer views

| Aspect | Presenter | Audience (projection) | Observer |
|---|---|---|---|
| Purpose | drive the story | be read from 10 m away | self-paced inspection |
| Controls | start/pause/step beat, cancel, reset, replay, marker, share | none | drill-down, filters, pin, export |
| Notes | beat script, "say this / do not say this", live-failure checklist, health of streams | none | glossary, "why this value" popovers |
| Density | full | 3-4 elements per beat, text >= 24 px equivalent | full |
| Always visible | MODE banner (LIVE / RECORDED / REHEARSAL), tier, seed, build profile, stream health | same banner, larger; no controls | same |
| Data | same | same | same |

Presenter-only *notes* must be a layer over identical data; a presenter must not be able to show the audience a number the observer cannot see. The audience view omits detail, never alters values.

Persistent honest-labelling rule for all three: **every value has a provenance badge or lives in a block that has one**, with shape+word (not colour alone), per the existing system [doc `phantom_lab_ui.md` section 7]. In the audience view the badge text is enlarged, not removed.

### 3.4 Topology visualisation design

- **Primary form:** node-link diagram with three layers: clients/load-gen -> gateway node(s) -> origin; peers connected in a ring or mesh according to the real routing (rendezvous router, `src/dist/router.rs`). V3 owns the semantic content (node identity, rendezvous ownership); I only fix the visual contract.
- **Nodes:** state shown by shape+label (active / suspected / failed / unknown), never colour alone; provenance badge on state ("self-reported" MEASURED vs "suspected by peers" INFERRED vs UNAVAILABLE).
- **Edges:** animate only when an event occurred: request, fetch, invalidation frame, anti-entropy round; edge animations are *event-driven playback* of real events at their recorded rate, with a visible sampling note ("showing 1 in 50 requests") when sampled. No ambient decorative motion; reduced motion replaces animation with a counter and a last-event list.
- **Origin:** shown with its in-flight gauge and cap marker; a scripted degradation is labelled "scripted: x4 service time for 10 s", not "origin got slow".
- **Single-node case:** a single gateway box with an explicit "cluster: disabled" tag; no fake peers. The old UI's three demo nodes (shared literal CPU/memory) must not be drawn as a cluster [doc `ui_metric_provenance.md` S-class rows].
- **Text equivalent:** every diagram has an adjacent table (node, state, provenance, last seen, queue) - same "view as table" pattern as the existing charts.
- **Scale limit:** target <= 8 nodes for the stage; more degrades to a summary list. State it.

### 3.5 Live request tracing and cache-decision explanation panel

Goal: pick one request (or one key) and show *why* the cache did what it did, using only events the gateway actually produced.

- **Request trace card [prop]:** `request_id` -> ordered steps with timestamps: received -> auth/tenant resolved -> key computed (tenant + path + query, shown as a short fingerprint, never raw secrets) -> lookup result (HIT fresh / HIT stale-served / MISS / BYPASS) -> if MISS: leader or waiter (coalescing), admission to origin (cap: admitted / rejected with `OriginCapSaturated`), origin fetch (duration, status), fenced store (stored / discarded because generation changed) -> response (status, `X-Cache-Status`, `Age`).
- **Decision explanation:** a one-sentence rule-based reason per step, generated from the event's own fields ("Served stale: entry expired 4.2 s ago, within stale-while-revalidate window, hard cap 24 h not reached"). The text template is part of the contract and listed, not invented by the UI, so it cannot claim more than the event says.
- **What exists:** response headers `X-Cache-Status` (HIT/STALE/MISS...), `X-Cache-Error` and `X-Cache-Error-Reason`, `Age`, `Retry-After`, exposed through CORS [code `server.rs:163-175, 1498, 1556, 2209-2257`]. A server-side diagnostics ring with 6 event names and a key fingerprint, fetched by polling the last N [code]. **What does not exist:** a request id correlating a client request with its internal steps; coalescing leader/waiter events; admission events; a cursor-based stream of diagnostics (the endpoint returns the last 100 each call, so a poller can miss events under load) [code `get_recent_events`, `diagnostics.rs:220`].
- **Honest minimum viable version (works today, Partially implemented):** the *load generator* knows each request it sent and its response headers; Scenario B already records `X-Cache-Status` per client request [doc `scenario_B_origin_slowdown.md` section 6]. A trace card built from client-side record + response headers is truthful but shallow (outcome only). Mark depth "outcome only; internal steps UNAVAILABLE".
- **Sampling is mandatory:** at hundreds of rps the panel traces a sample; the panel states the sampling rule (e.g. "all non-HIT outcomes and 1% of hits") and never presents the sample as the whole.

### 3.6 Prediction-versus-actual visualisation with uncertainty

Applies only once a predictor emits `prediction` events (V1/V2 define content). Design contract [prop]:

- One time axis, **actual** drawn from measured aggregates, **prediction** drawn as a median line plus a stated interval (e.g. 80% and 95% bands) over a *future* horizon; the prediction for time t is **frozen at issue time** (shown by an issue marker) and never redrawn after the fact. Redrawing predictions in hindsight is the commonest way to look better than one was.
- Always shown: horizon, model/version id, training data identity, interval type (calibrated how, on what data), and **observed coverage so far** ("95% band contained the actual in 41 of 50 windows") as DERIVED with N.
- Misses are not hidden: a residual strip under the chart; a "prediction withheld/unavailable" gap for windows where the predictor declined, rather than interpolating.
- Gating: per `phantom_intelligence_phase.md` and CLAUDE.md 6b, no prediction may be shown as *acting* on live traffic until the executor integration exists. Mode label on the panel: OFFLINE REPLAY / SHADOW (not acting) / ACTIVE. Today only OFFLINE REPLAY is possible, from T1 runs [doc `demo_experience_plan.md` D-5 note].
- Accessibility: bands differ by pattern and by a table; the coverage sentence is real text.

### 3.7 Experiment comparison

- **Compare view:** pick any two (or up to four) sessions/runs/arms; the backend, not the browser, supplies the paired differences (UI computes no new metric; rule 4 of the provenance spec [doc]). Existing: Scenario B `paired_vs_base` with BCa intervals and sign-flip p; M6 per-scenario effect sizes with a zero line [doc].
- **Pre-flight compatibility check:** two items are comparable only if seed, workload, fault schedule, build profile and host class match; mismatches are listed ("different gateway binary hash") and the comparison is refused or labelled "NOT A CONTROLLED COMPARISON". Scenario B's pilot-vs-main binary-hash discrepancy is precisely the failure this prevents [doc notice item 6].
- **Zero line, N, interval on every comparison**, and verdicts rendered with equal weight for positive/negative/inconclusive (existing rule).
- Gap: user-started `t1_matrix` runs have no result summary in the API (API gap 3); comparing them needs a server-side summary (Proposed).

### 3.8 Evidence and report generation

- **Report = a document, not a screenshot.** Sections: scenario and seed; environment (OS, build profile, host class, commit and dirty flag from `manifest.json`); per claim: statement, test, N, effect with interval, verdict, tier, limitations, falsifier; verbatim binding notices; artifact list with SHA-256; "how to reproduce" command; "what this does not show".
- **Server-assembled from retained files** (the UI never embeds results; API contract section 6) [doc]. Output formats: HTML (self-contained, printable) and Markdown; PDF only via browser print (no new dependency).
- **Immutability:** the report records the evidence ids and hashes it was built from; regenerating from changed evidence produces a different report id.
- **Citation guard:** copying a number out of the report (copy button) appends its tier and the notice id, mirroring "must_accompany_any_citation" in the existing payload [doc `phantom_lab_ui.md` section 5].
- Status: Experiment viewing exists (Already); report builder is Proposed. `src/lab/report_gen.rs` exists, contents unverified by me.

### 3.9 Reset and deterministic replay (seed)

- **What "deterministic" can honestly mean per tier:**
  - T1 SIMULATED: bit-reproducible given seed (virtual time, `(time_us, seq_id)` total order, `src/lab/events.rs`) [code]. Replay = rerun, and the manifest hash of `cells.csv` should match (the `result` event already carries `cells_csv_sha256`) [code fixture].
  - T2 MEASURED_LOOPBACK: the **workload schedule and the origin script are deterministic** (the schedule depends only on the workload, not the arm: `loadgen.rs:39`) [code]; **outcomes are not bit-deterministic** (real time, OS scheduling) [doc `scenario_B_origin_slowdown.md` section 5]. "Replay" of a T2 run must mean one of two different things and the UI must say which: **Re-run with the same seed** (new measurements, same offered load) vs **Play back the recorded events** (identical to what happened). Never label a re-run as a replay of the same result.
- **Reset [prop]:** "Reset to start of beat N" = discard the *session's* live view state and start a new run of that beat with the same seed; retained evidence is never deleted by reset. A reset requires confirmation and writes a `scenario_reset` event so the evidence trail shows it happened. Cold-cache state at start must be guaranteed (new gateway process per run is how Scenario B does it: `gateway_pid` changes per arm/rep in the fixture) [code].
- **Seed display:** seed and seed *set* (T/D) shown; sealed seeds never shown or accepted (existing rule) [doc].
- **Recorded playback with scrubbing:** a persisted `events.jsonl` already supports replay-from-start with `Last-Event-ID` [doc]; scrubbing requires an index by event number/time (Proposed).

### 3.10 Surviving a live failure (generic playbook)

| Failure | What the audience sees | What the presenter does |
|---|---|---|
| Stream disconnect | "Disconnected - last event #N" banner (exists), charts freeze with "last known" | one click "play recorded run" |
| Lab API off / 401 / 403 | existing explicit panels | switch to recorded mode |
| Gateway child fails to start | run `failed` with code, never "completed" | switch to recorded run of the same beat |
| Host contention (loadgen lag) | "offered load not achieved" flag | continue; the report records it |
| Projector/second device drops | nothing (observer view independent) | re-pair |
| Result is bad for the story | shown as is | the script contains the sentence for it (e.g. beat 3) |

Recorded fallback must always be *labelled* RECORDED; it is the safety net, never a substitute silently swapped in.

---

## 4. Event and API requirements

### 4.1 What exists (so extensions are minimal)

| Existing | Where | Notes |
|---|---|---|
| Per-run SSE `state, progress, result, done, error`, monotonic `id`, `Last-Event-ID` resume, 15 s heartbeat, cap of 16 concurrent streams, closed when token expires (checked every 2 s) | `lab_api.rs` `run_events` [code]; `lab_api.md` | batch-run lifecycle events only; no domain events |
| Persisted `events.jsonl` per run | `lab-results/runs/<id>/` | tail-able; gap-free ids |
| Aggregate live SSE `/api/events?schema=2`, 100 ms | `server.rs:940-982` [code] | metrics with provenance, nodes, capabilities |
| Diagnostics ring (poll last 100) | `server.rs:2154` [code] | no cursor, no request id |
| Auth: JWT roles Observer/Operator/admin, anonymous loopback-only dev mode with Host/Origin guard; `fetch`-streaming with header | `lab_api.md` [doc] | |
| Tiers `MEASURED_LOOPBACK`, `SIMULATED`, `RETAINED_*`, `UNAVAILABLE` | `labProvenance.js` | |

### 4.2 Event taxonomy [prop]

All events share an envelope; kinds below are *domain events* carried in a new stream kind alongside the existing lifecycle events (same `id`/resume mechanics).

**Envelope (required on every event):**

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | `demo-1` |
| `id` | u64 | monotonic per session, gap-free, assigned by the single writer (as in the Lab API) |
| `t_server_ms` | u64 | wall clock at emission, server |
| `t_session_us` | u64 | monotonic offset from session start (used for rendering; avoids clock skew) |
| `kind` | enum | one of the kinds below |
| `tier` | enum | `MEASURED_LOOPBACK` / `SIMULATED` / `DERIVED` / `UNAVAILABLE` (only where a number or state appears) |
| `source` | string | emitting component (gateway node id, loadgen, origin script, runner, server_recovery) so a view can say who observed it |
| `scenario_ref` | `{scenario_id, beat, arm, rep, phase}` | where in the plot this belongs |
| `sampled` | `{rate, rule}` or absent | present iff the stream is a sample; absent means complete |
| `trace_id` | string? | correlation across request / decision / fetch events when applicable |

**Kinds, with required payload fields:**

| Kind | Required fields | Notes / status |
|---|---|---|
| `request` | `trace_id`, `tenant_fp`, `key_fp`, `method`, `sent_at`, `completed_at`, `status`, `x_cache_status`, `latency_ms`, `send_lag_ms`, `bytes?` | Client-side facts exist in Scenario B's records (`ClientRecord`) [code `loadgen.rs:66`]. Server-side emission = new. `key_fp`/`tenant_fp` are fingerprints, never raw (privacy decision R-UI-26) |
| `cache_decision` | `trace_id`, `key_fp`, `outcome` (`hit_fresh`,`hit_stale_served`,`miss`,`bypass`,`rejected`), `entry_age_ms?`, `ttl_remaining_ms?`, `stale_window_ms?`, `reason_code` | Partially derivable from existing diagnostics `CacheHit`, `CacheStaleServed` [code]; `miss` not emitted today [gap] |
| `origin_fetch` | `trace_id`, `key_fp`, `op_id`, `op_gen`, `started_at`, `duration_ms`, `status`, `outcome` (`success`,`non200`,`failed`,`timeout`), `origin_in_flight_at_start` | Exists as `OriginFetchSuccess/Non200/Failed` with `op_id`, `op_gen`, `duration_ms` [code] |
| `coalescing` | `key_fp`, `op_id`, `role` (`leader`,`waiter`), `waiters_now`, `waiter_limit`, `result` | Counter only today; events new |
| `admission` | `trace_id`, `gate` (`origin_cap`, `waiter_limit`, `capacity`), `decision` (`admit`,`reject`,`queue?`), `cap`, `in_flight`, `reason` (e.g. `OriginCapSaturated`) | Rejections observable as headers/counters [code `server.rs:2209-2231`]; admit/reject events new |
| `invalidation` | `trace_id`, `scope` (`key`,`tenant`,`all`), `key_fp?`, `op_gen_before/after`, `origin_node`, `peers[]{node, state: pending/acked/repaired/lost, lag_ms}` | `CacheInvalidated` exists locally [code]; peer states per R-UI-18 open |
| `fencing` | `key_fp`, `op_id`, `op_gen`, `current_gen`, `result` (`stored`,`discarded_stale_generation`,`discarded_epoch`) | counters `stale_writes_prevented`, `stale_ops_ignored` exist [code]; per-event = new |
| `security_decision` | `trace_id`, `check` (`jwt`,`role`,`tenant_resolve`,`forged_tenant_header`,`host_origin`), `result` (`allow`,`deny`), `code` | auth failures answered with generic bodies by design [doc]; V4 owns scope |
| `node_health` | `node_id`, `state`, `state_src` (`self_reported`,`peer_inferred`,`unavailable`), `last_seen_ms`, `process_rss?`, `cpu?` | `nodes[]` exists but CPU/mem literals in demo mode [doc]. Real counters Proposed |
| `prediction` | `prediction_id`, `issued_at`, `horizon_ms`, `target_metric`, `median`, `intervals[{level, lo, hi}]`, `model_id`, `mode` (`offline_replay`,`shadow`,`active`) | **No producer exists.** Content set by V1/V2 |
| `prediction_outcome` | `prediction_id`, `actual`, `within[{level, bool}]` | derived at maturity |
| `action` | `action_id`, `actor` (`policy_id`/`operator`), `mode` (`advisory`,`canary`,`active`), `change` (`from`,`to`), `guardrails_in_force`, `rollback_state`, `reason`, `evidence_ref` | `/api/v1/intelligence/audit` is the only precursor [code route], not authenticated per R-UI-23 [doc]; engine not in serving path |
| `scenario_state` | `state` (`queued`,`running`,`cancelling`,`completed`,`failed`,`cancelled`,`interrupted`,`paused?`,`reset`), `beat`, `phase`, `progress{done,total}` | Already: `state`/`progress` events [code]; `beat`, `paused`, `reset`, `marker` = new |
| `marker` | `text`, `author_role` | presenter annotation for the report timeline |
| `gap` | `from_id`, `to_id`, `reason` (`dropped_backpressure`,`sampled`) | the server states honestly when it dropped events |

### 4.3 Ordering and delivery guarantees

- Per session a **single logical order**: ids assigned by one writer, as in the Lab API (all events appended under one lock, one line per write) [doc]. Cross-process events (several gateways) are merged by the session writer and carry the emitting `source` and its own `t_source_us`; **merge order is arrival order at the session writer, not true causal order**. Causal links go through `trace_id`/`op_id`, not through id adjacency. State this in the UI ("order shown is order observed").
- Resume by `Last-Event-ID`, replay from persisted log (existing).
- Duplicate suppression by id (client already does) [doc].
- `t_session_us` is non-decreasing per source; if a source violates this the event is flagged `clock_regress` rather than reordered silently.

### 4.4 Tiers (volume) and retention

Not every event should be persisted or streamed at full rate. Three tiers [prop]:

| Tier | Contents | Stream rate | Retention |
|---|---|---|---|
| T-A "story" | `scenario_state`, `marker`, `node_health` changes, `action`, `prediction`/`outcome`, `invalidation`, `fencing` discards, `admission` rejects | all, lossless | whole session, persisted, hashed into evidence |
| T-B "aggregates" | per-100 ms or per-second counters, histograms, origin gauge (the existing `/api/events` data, plus `metrics.origin_cap`) | fixed rate | whole session as downsampled series; raw 100 ms window short |
| T-C "trace samples" | `request`, `cache_decision`, `origin_fetch`, `coalescing` | sampled by rule (all non-hit, 1% of hits, all of a pinned key) with `sampled` stated | bounded ring in memory (e.g. last 10k) plus the sampled set persisted; **full raw capture only when `record_raw` is on** (the T1 runner already has `record_raw` for `requests.csv`/`fetches.csv`) [doc] |

Retention bounds must be explicit counts and bytes (project rule, CLAUDE.md 5): per session event-log cap, per-run artifact cap (the API already caps downloads at 64 MiB and bodies at 16 KiB) [doc]; when the cap is hit, a `gap` event is written, not silent truncation.

### 4.5 Backpressure

- **Producers never block the serving path.** Gateway-side emission is into a bounded ring (as `diagnostics` is, capped at a fixed number [code]); on overflow, drop oldest *T-C only* and increment a drop counter that is itself reported (so "0 drops" is a measured claim). T-A is never dropped; if its buffer is full, the session is marked degraded (honest failure).
- **Consumers:** per-client bounded queue; slow consumers get a `gap` event and are resumed from the persisted log by id; the SSE cap (16 concurrent) already exists [doc]; a projector view counts as one.
- **Measure observer overhead**: any new per-request instrumentation changes the system being measured. Before it is enabled in a measured scenario, the overhead must be measured and stated in the evidence (the same discipline that lists "no overhead measurement exists" as an open gap in the Scenario B notice, item 8) [doc]. Rehearsal and trace-heavy modes should be labelled "TRACING ON: latency numbers include instrumentation".

### 4.6 Authentication and authorization for streams

- Reuse the Lab API model: Observer for reads, Operator for start/cancel/reset/pause, anonymous loopback-dev with Host/Origin guard when JWT is off [doc]. Token via header with `fetch` streaming; never in URLs [doc].
- **New (Proposed): view ticket** for audience/observer devices (3.2). Scoped to one session, read-only, short TTL, revocable by the presenter, never grants Operator, not accepted by any other route. Tickets are secrets: not logged, not in the report.
- Tenant and key privacy: events carry fingerprints (`tenant_fp`, `key_fp`), never raw keys or tokens; the Lab API already scrubs server paths and `seed_values` from payloads [doc]; the same scrubber is applied to the new stream. Whether an Observer may see raw keys at all is an owner decision (R-UI-26).
- The sealed-seed rule extends to the new stream: no event may contain a sealed seed, and the process refuses to serve the Lab API with `PHANTOM_SEALED_SALT_HEX` set [doc].

### 4.7 Mapping onto the Lab API (extends vs new)

| Need | Extends | New |
|---|---|---|
| Domain event stream | `GET /runs/{id}/events` could carry new kinds through the same persisted-log mechanism (envelope already has `id`, `event`, `data`, `tier`) | event names beyond `state, progress, result, done, error` change the contract (`lab-1` says exactly those five): a **`lab-2`** or a separate `GET /sessions/{id}/events?tiers=A,B,C` is needed; I recommend a separate session resource to avoid breaking the pinned fixtures and tests in `ui/test/` |
| Scenario catalogue | `GET /capabilities` already returns kinds, arms, scenarios, families, profiles, limits [doc] | `GET /scenarios` with capability requirements and narration beats |
| Session lifecycle | `POST /runs` (+ `cancel`) | `POST /sessions`, `/sessions/{id}/{pause,resume,reset}`, `/sessions/{id}/tickets` |
| Evidence | `GET /evidence*` | `POST /reports` and `GET /reports/{id}` (assembly from evidence ids) |
| Compare | `paired_vs_base` fields in Scenario B summary | `GET /compare?a=&b=` with compatibility result |
| Request trace | none | `GET /sessions/{id}/trace/{trace_id}` |

Constraint that stays: one active run (`409`) [doc]. A multi-beat session must serialise beats or the limit must be raised deliberately; this is an owner decision because the scenario-B runs share CPU on a single host.

---

## 5. Accessibility, responsive, theming, states, projection

Baseline to inherit and not regress: native controls, landmarks, ordered headings, non-colour status (glyph + word), 32 px desktop / 44 px touch targets, light/dark tokens-only CSS, forced-colors, reduced motion, "view as table" under every chart, `role="alert"` and `role="status"` for state changes [doc `phantom_lab_ui.md` sections 7, 9]. All of it is **statically linted and unit-tested, not browser-verified**; re-verification in a real browser at several widths is itself an open item [doc `demo_experience_plan.md` D-7].

New requirements for the Demo Center [prop]:

1. **Reduced motion:** event-driven animations (topology edges, trace steps) are replaced by a stepped list and counters. "Pause" is available regardless of the setting, and the presenter's step control works by keyboard.
2. **Live regions without noise:** high-rate streams must not be announced. Only T-A events and state changes go to `aria-live=polite`; aggregates update silently with a manual "announce current values" button. A visible "N events this second" counter replaces per-event announcements.
3. **Keyboard:** every presenter control reachable and operable; documented shortcuts do not conflict with screen-reader keys; Escape to dismiss confirmations (exists for cancel).
4. **Responsive:** phone width shows a single lens at a time with the timeline pinned at top; tablet two-pane; desktop three-pane. No horizontal page scroll; tables scroll inside their own wrapper (existing).
5. **Light/dark:** tokens only (a test already fails on hex colours in `lab.css`) [doc]; the topology diagram uses the same tokens; contrast checked with the existing script covering token pairs.
6. **States, for every panel:** loading, empty ("no events yet - this is not a failure"), partial (some sources unavailable), error with retry, unauthorised, disabled (feature off), stale, disconnected, reconnecting with gap count, *sampled*, *recorded*, *rehearsal*. Each state has distinct wording; "no data" is never drawn as zero.
7. **Projection mode (audience view):**
   - Scales to 1920x1080 and 4K; minimum body text equivalent to ~24 px at 1080p; metric values >= 48 px; at most 4 primary elements per beat.
   - High-contrast theme variant (still tokens), no thin lines (>= 3 px strokes), no information carried by hue or hover alone.
   - Fixed footer strip, always on: `MODE | TIER | SEED | BUILD | STREAM HEALTH`. This strip is the anti-decoration device: nothing on the stage can claim to be live while the strip says RECORDED.
   - No hover-only content; no scrolling; the screen auto-follows the presenter's beat but is not interactive; a "freeze" indicator shows when the presenter pauses.
   - No auto-playing sound or flashing (avoid > 3 flashes/s).
   - Large-type text equivalents of each chart's headline reading ("Peak origin concurrency fell from about 39 to 24; clients failed 4.95 points more") generated from the same fields as the table, not hand-written per demo.
8. **No fabricated traffic / decorative claims (hard rules):**
   - Any rendered series must be backed by an event or an artifact field; the UI must not generate "ambient" traffic for liveliness, interpolate across gaps, or smooth across `gap` events.
   - No client-side counterfactuals ("without Phantom: +300%").
   - No "saved $" model unless parameters are visible and it is labelled a model (the old $ panel is the cautionary tale) [doc `ui_metric_provenance.md`].
   - Marketing headlines about predictions, autonomy or "intelligent" actions are prohibited unless the corresponding `action`/`prediction` events exist in that session and are tier-labelled.
   - Automated guard: extend the existing `forbidden` / `lab.guard` source tests (no result-like numeric literals; no storage; tokens only) to the Demo Center files [doc].

---

## 6. Implementation mapping, scenario by scenario

Classification: **Already implemented**, **Partially implemented**, **Proposed**, **Experimental**, **Unsupported** (cannot be shown truthfully with the current system; shown as an explicit unavailable panel).

| Item | Class | What exists (cite) | What is missing |
|---|---|---|---|
| Beat 0 Preflight / capability check | Already implemented | `GET /capabilities`, Connection panel [code/doc] | rehearsal mode; presenter checklist UI |
| Beat 1 Baseline (A) | Partially | real gateway + Zipf open-loop load + scripted origin in Scenario B; aggregate stream; hit/miss counters | long/open-ended session; windowed rates and histograms (R-UI-03/09/10); trace samples |
| Beat 2 Flash crowd with coalescing (C) | Partially (mechanism) / Proposed (scenario) | coalescing, waiter limit, counters [code]; T1 `W-FC` workload [code `workloads.rs:192`] | real-gateway rate-step/hot-set arrival process; coalescing events; "origin requests avoided" derived series |
| Beat 3 Origin slowdown + cap (Scenario B) | **Already implemented** (runs, evidence, notice, charts) with stated UI gaps | Lab API + `#/lab` + retained 10-rep result [doc] | per-phase shed, success-only latency (API gaps 1, 2); load-dependent origin (not planned); browser verification |
| Beat 3 adaptive control narrative | Unsupported | AIMD arm effectively static ~12 [doc notice]; engine not in serving path | any claim of adaptive merit |
| Act "Predictive demand" (B) live | **Unsupported** | PIE stub; `pie: None` [CLAUDE.md] | predictor, `prediction` events, V1/V2 gate |
| Act "Predictive demand" offline | Partially | M6 retained evidence (`NOT_SUPPORTED`), T1 runner [doc] | forecast-vs-actual replay view |
| Beat 4 Invalidation + fencing | Partially | fencing + counters + `CacheInvalidated` [code]; DELETE path [code] | per-key causal event chain; scripted per-key slow fetch; scenario packaging |
| Beat 4 cluster delivery view | Experimental | cluster mTLS path, anti-entropy worker, 150 ms primary leg [doc/CLAUDE.md] | peer ack events (R-UI-18), durable log (not provided) |
| Beat 5 Node failure (real processes) | Experimental / Proposed | child supervision and job object in Lab API [doc]; cluster only with mTLS | multi-process cluster run kind; cert provisioning for demo; `node_health` events |
| Node failure via old `NODE_FAILURE` button | Unsupported as a claim | simulation flag only [code `server.rs:1089`] | must remain labelled SIMULATED or be removed from the Demo Center |
| Tenant-isolation attack (D) | Unsupported as a defence demo; Proposed as an experiment | tenant in key; forged header ignored in secure mode; tenant invalidation [code] | tenant-labelled load, fairness metric, any quota (none found [gap]) |
| Beat 6 Evidence report (F) | Partially | evidence viewer, artifacts with SHA-256, reproduce command [doc] | report assembly, citation guard, immutability record |
| Request trace panel | Partially (outcome only from client records) / Proposed (internal steps) | headers; diagnostics ring [code] | `trace_id`, decision/admission/coalescing events |
| Prediction-vs-actual panel | Unsupported (until V1/V2) | none | everything |
| Topology view | Partially (placeholder lists `nodes[]` with provenance) | `Placeholder.jsx` NodeList [code] | real membership (R-UI-17), drawing, events |
| Compare view | Partially | paired stats inside Scenario B summary [doc] | generic compare endpoint, compatibility check |
| Presenter/audience/observer views | Proposed | none (single layout) | role views, projection mode, view tickets |
| Multi-device pairing | Proposed | JWT roles + header streaming [doc] | ticket endpoint, TTL/revocation, security review (V4) |
| Deterministic replay | Partially | T1 bit-reproducible; persisted event log with resume [doc/code] | explicit re-run vs playback distinction in UI, scrubbing index |

### 6.1 Phased roadmap with measurable acceptance criteria

Phase order is by risk and by dependency on honesty infrastructure. Every phase ends with a real-browser verification pass (not yet performed for the existing Lab UI).

**Phase 0 - Make what exists demo-ready (no new backend).**
Scope: Demo Center shell and navigation over existing `#/lab`; Scenarios page (Scenario B, M6 evidence, T1 matrix) with "what is real" badges; mode footer strip; recorded-run playback of retained Scenario B result; "thing we got wrong" page.
Acceptance:
- A first-time viewer, without help, can state from the screen alone whether the numbers are live, recorded, or simulated (usability check, >= 5 participants, >= 90% correct) [prop test].
- 0 screens render a value without a provenance badge or a unavailable reason (automated DOM test over fixtures, same style as the existing `lab.render` tests).
- Scenario B result view still shows the binding notice before any chart (existing test retained).
- Real-browser run at 390/820/1280/1920 px, light and dark, with retained screenshots and console/network log (this satisfies the open D-7).

**Phase 1 - Session, views, replay.**
Scope: presenter/audience/observer views; projection mode; pause/step/mark; reset; explicit "re-run with same seed" vs "play back recorded".
Acceptance:
- Audience view at 1920x1080: every text node >= the stated size; automated check; manual check at 4 m.
- Switching views changes no data value (property test: same fixture, three views, same set of (metric, value, provenance) triples).
- Disconnect the stream mid-beat: banner within the existing stale window; reconnect resumes with zero duplicate events and a reported gap count equal to the true gap (fixture test).
- Re-running a T1 scenario with the same seed reproduces `cells_csv_sha256` exactly; re-running T2 with the same seed reproduces the offered-load schedule (hash of the arrival schedule) but is *not* required to reproduce outcomes, and the UI says so.

**Phase 2 - Domain events (T-A, T-B) and honest traces.**
Scope: `scenario_state` extensions, `marker`, `admission` rejects, `fencing` discards, `invalidation`, `node_health` (single-node), trace samples T-C with client-side + header facts; the `gap` mechanism; instrumentation-overhead measurement.
Acceptance:
- Event log has strictly increasing, gap-free ids per session (property test); a deliberate overflow produces exactly one `gap` event covering the dropped range.
- With tracing on vs off, release build, Scenario B workload, >= 10 paired repetitions: overhead on client p99 and throughput reported with a 95% interval and stated limitations; tracing is not allowed in measured evidence runs unless the interval excludes an agreed bound (owner sets the bound).
- Every trace-card step maps to a named event field (static check against the template list); a request with a missing step renders "step not observed", never an inferred step.
- `stale_cap_violations` remains 0 in all runs (existing invariant).

**Phase 3 - Flash crowd and invalidation beats on the real gateway.**
Scope: rate-step/hot-set arrival process; coalescing events; scripted per-key slow fetch; Beat 2 and Beat 4 as Lab scenarios.
Acceptance:
- For the flash scenario, number of origin fetches observed at the scripted origin equals the sum of `origin_fetch` events (cross-check between two independent counters; mismatch fails the run) .
- Fencing beat: in N scripted races the stale response is never stored (zero `stored` after a newer generation), checked from events and from a follow-up GET; a mutation test that disables fencing makes the beat fail (project rule: a test that fails without the change).
- Offered-load achieved >= a stated fraction of target or the run is flagged.

**Phase 4 - Report and comparison.**
Scope: report assembly; compare endpoint with compatibility check; citation guard.
Acceptance:
- Report regenerated from the same evidence ids is byte-identical; from changed evidence it differs in report id.
- Compare refuses two runs with different binary hashes unless explicitly overridden, and the override is shown in the output.
- Every number in the report resolves to an evidence field (automated link check); negative verdicts rendered with the same typography weight as positive (snapshot test).

**Phase 5 - Multi-device pairing (security review first).**
Scope: view tickets; observer role UX.
Acceptance: V4 sign-off; tickets expire, are revocable, never grant Operator, are absent from logs/URLs/reports (tests mirror the existing "no token in storage/URL" source guards); a second device receives the same event ids as the first.

**Phase 6 - Experimental tracks (only on owner approval and V3/V1/V2 outputs).**
Multi-process local cluster and node-failure beat; predictor panel; tenant-fairness experiment. Each needs its own preregistered acceptance and falsifier before any UI is built.

---

## 7. What is MEASURED / SIMULATED / DERIVED / UNAVAILABLE - quick reference for the storyboard

| Value | Tier today | Note |
|---|---|---|
| Client latency percentiles (Scenario B) | MEASURED loopback | all requests, includes fast 503s; success-only UNAVAILABLE |
| Peak origin concurrency | MEASURED loopback | from the scripted origin's own records |
| Shed fraction | MEASURED (whole run) | per-phase UNAVAILABLE |
| Paired reduction, CI, p | DERIVED | BCa intervals, sign-flip permutation |
| Verdict / label | DERIVED | mechanical rule; `VALID` has a narrow meaning |
| T1 cells | SIMULATED | virtual time on real cache + executor, simulated origin |
| M6 verdicts | RETAINED simulated, DERIVED | `NOT_SUPPORTED` |
| Hit ratio (live aggregate) | MEASURED counters, DERIVED ratio | cumulative today; windowed UNAVAILABLE |
| Coalescing "origin requests avoided" | DERIVED | formula displayed |
| Per-request internal steps | UNAVAILABLE | no trace events |
| Node CPU/memory | UNAVAILABLE in real terms | demo nodes carried literals [doc] |
| Peer invalidation state | UNAVAILABLE | no contract |
| Predictions | UNAVAILABLE | no producer |
| Policy decisions in serving path | UNAVAILABLE | engine not integrated |
| Dollar savings, "Phantom OFF" | prohibited | fabricated in old UI |

---

## 8. Risks

| # | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| R1 | Story pressure re-introduces decoration (a prediction line, a "saved $" number, fake peers) | High / Critical (trust) | footer strip, source-guard tests, "every series has an event" rule, owner sign-off on any new headline claim |
| R2 | The demo's best real result is negative; presenters may soften it | High / High | script contains the sentence; the report builder cannot omit a failed verdict; "thing we got wrong" page |
| R3 | Live runs are slow and fail on stage (child gateway start, host contention, debug build) | Medium / High | rehearsal mode, recorded fallback always available and always labelled |
| R4 | Tracing instrumentation perturbs the system being measured | Medium / High | measure overhead (Phase 2), "TRACING ON" label, no tracing in evidence runs by default |
| R5 | Loopback single-host results over-read as production performance | High / High | permanent tier banner; report limitation text; never show "x faster" across tiers |
| R6 | Event volume overwhelms browser or server | Medium / Medium | tiers, sampling, bounded rings, `gap` events, SSE cap |
| R7 | Pairing opens a network surface in a product whose dev mode is loopback-only | Medium / High | JWT only for cross-device; scoped read-only tickets; V4 review; no pairing on anonymous mode |
| R8 | Wall-clock ordering across processes misread as causality | Medium / Medium | `trace_id`/`op_id` for causality; "order observed" label |
| R9 | The UI has never been verified in a real browser; accessibility claims are static | High / Medium | Phase 0 browser pass with retained evidence before building more |
| R10 | Contract churn breaks fixtures (`lab-1` pinned five event names) | Medium / Medium | separate session stream/version rather than overloading `lab-1` |
| R11 | Multi-process cluster demo depends on mTLS provisioning and platform-specific child management (Windows only exercised) | High / Medium | Experimental label, explicit skip behaviour |
| R12 | Duplicate/competing ideas across V1-V4 and this document | Medium / Low | this document specifies envelope and views; others fill payload content |

Items I could not verify and which materially affect the plan: contents and behaviour of `src/lab/report_gen.rs`; whether the real-gateway load generator can be given a rate-step shape without changing the frozen modules (the Lab API deviation list mentions `src/core/coalesce.rs` is frozen); per-key delay injection in the scripted origin; whether any tenant byte budget exists outside the files grepped; the user's actual list of seven playgrounds (not in the files read); the exact `nodes[]` fields emitted at present.

## 9. Owner decisions needed

1. **Flagship ordering:** accept the "Launch Day" re-sequencing, with B demoted to offline/replay and D moved to an optional side act? Or require B live (which then requires V1/V2 delivery and an engine-in-serving-path decision that CLAUDE.md currently forbids doing "merely for the demo")?
2. **Honest-negative as a feature:** confirm that the demo's Scenario B beat is presented as `TRADE_OFF_FAILURE` with the permitted-conclusion text, and that no beat may be presented in a more favourable form.
3. **Load-dependent origin:** build an origin whose latency depends on its concurrency so that "the surge causes the slowdown" is mechanism rather than narrative? (Large effort; changes what Scenario B means; would need a new protocol and cannot reinterpret the existing result.)
4. **Session model and the "one active run" limit:** keep serialised beats, or permit overlapping runs on one host (affects measurement validity)?
5. **Event contract:** new `demo-1` session stream (recommended) or extend `lab-1`? Who owns the schema and the pinned tests?
6. **Instrumentation budget:** the maximum acceptable measured overhead for per-request tracing, and whether tracing is allowed at all in evidence-grade runs.
7. **Multi-device pairing:** allowed at all? If yes, JWT-only, or a new scoped ticket? Any non-loopback exposure needs an explicit security decision.
8. **New tier labels:** a distinct label for single-host multi-process (and later multi-host) runs, so they are never conflated with `MEASURED_LOOPBACK` single-process.
9. **Privacy of keys and tenants in events:** fingerprints only (recommended), or raw keys for Operators?
10. **Seven playgrounds:** confirm collapsing them into three lenses over one scenario model, and supply the actual list so I can map each one.
11. **Tenant demo:** is a possibly-negative noisy-neighbour experiment acceptable for a showcase, or must D wait until a quota/fairness mechanism exists?
12. **Browser verification gate:** may Phase 0 be funded as a blocker for all new UI work (the existing Lab UI has never been rendered in a browser per its own design doc)?
