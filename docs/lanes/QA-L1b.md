# QA-L1b: first-time-user test of the Demo Hub (snapshot of 2026-10-31 10:18)

Tester: assistant L1b. Tags: everything below was reproduced here unless marked "verified in code" (file:line in the snapshot `D:\ClaudeTemp\lanes\L1b\scripts\`, identical in size and mtime to L1's files at 10:09-10:12).

Setup (as the student's batch file does it, on separate ports): `node scripts/demo_flow.mjs --exe D:\ClaudeTemp\audit\phantom_rev7.exe --dir D:\ClaudeTemp\lanes\L1b\_work --console --console-port 3301 --http-port-base 18400 --cluster-port-base 19800 --origin-port 19400`; headless Chrome (own throwaway profile) through `cdp-lib.mjs`, 1440x1000 and 390x844. Release gateway `phantom_rev7.exe` (07:27), loopback, one Windows host. Every run below was started by clicking the card's "Demonstrate" button; nothing was typed.

Limits of this test (read first):
- `phantom_rev7.exe` has no static origin cap and no policy-engine routes. `D:\ClaudeTemp\audit\target\release\phantom-cache-demo.exe` is dated 07:21 (older than 10:00), so by the rule I was given I did not use it. Scenarios 23, 26, 27 therefore show "needs the newer binary" here (see table); they are NOT evidence of a defect in the scenarios themselves.
- The Control Center (port 3000) was not running and the L1b copy has no `ui/node_modules`, so scenario 14 (two windows) and the "open each screen in one click" half of 28 could not be tested end to end.
- The "Start the demo cluster" button (cluster down state) was not exercised: the cluster was already up from `demo_flow.mjs`, which is the real batch path. `hub_check.mjs` covers that button.

## 1. Table (first pass = each card alone, in order; second pass = inside "Demonstrate everything")

Verdict words are the hub's own. Seconds: first pass / second pass. "Feed real" = the live feed showed real timestamped steps; "hdr" = real X-Cache-Status / source per response was visible in the feed (the full request and response headers sit in a collapsed "request and response" row under each step).

| id | one-click? | seconds | verdict (both passes identical) | feed real? | issues | severity |
|---|---|---|---|---|---|---|
| tour | Opens with one click; 8 steps, Next/Back/Skip work, keyboard arrows are wired | 4+5+16+1+3+8 s of runs | steps 2,3,4,6,7 Passed; **step 5 "Is it safe?" prints "unknown scenario"** | yes (feed behind the window) | Step 5 runs id `security`, which is not in the catalogue (P1) | **blocker** |
| gallery | yes | 4.8 / 3.3 | Passed (3 checks) | yes, per-tile MISS/origin 322 ms ... sha256 OK, hdr yes | none beyond P8 | polish |
| stampede | yes | 6.4 / 4.5 | Passed (4) | yes: 300 requests summarised (origin asked 1 time, coalesced 280, slowest 935 ms); no per-request lines | raw JSON `{"coalesced":280,...}` in a sentence (P8) | polish |
| peerfill | yes | 6.4 / 5.0 | Shows (5) | yes; summary lines only | "origin requests 32 for 72 reads" in fixed order is honest but needs a reader's care | polish |
| offload | yes | 4.9 / 3.8 | Passed (3) | yes: sampled responses with sha256 OK; 3,000 sent, origin counted 502 | none | - |
| workloads | yes | 4.8 / 3.8 | Shows (3) | yes: hit ratio 98.0 / 77.2 / 5.0 / 0.0 % per shape | none | - |
| scale | yes | 17.2 / 14.0 | Passed (5) | yes: sampled stream, 16,000 requests, 0 errors, 0 integrity failures | none | - |
| auth | yes | 1.5 / 0.3 | Passed (6) | yes, real 401/403 with reasons | step for "garbage token" says "sent ... as no token" (P8); "known gaps listed: 8" with no explanation | polish |
| roles | yes | 1.6 / 0.3 | Passed (7) | yes | same "known gaps" number | polish |
| isolation | yes | 1.5 / 0.5 | Passed (7) | yes: A MISS then HIT, B MISS and origin +1 | none | - |
| targetsafety | yes | 1.6 / 0.3 | Passed (5) | yes: four 400s, origin +0 | none | - |
| mtls | yes | 3.3 / 2.4 | Passed (6) | yes | raw JSON `{"handshake":..., "error":"ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED"}` shown to a stranger | polish |
| failclosed | yes | 1.5 / 0.3 | Passed (2) | yes: exit 2 + [CONFIG ERROR] line | none | - |
| multiuser | yes (opens two windows) | 1.6 | Needs attention here ONLY because the Control Center was not running ("not running at http://localhost:3000, start it with the start batch file") | yes | Windows are opened before the reachability check, so with the Control Center down the student gets two error tabs (verified in code: `index.html:290` opens, `server.mjs:166` checks afterwards). Not tested with a live Control Center | should fix (low) |
| invalidation | yes | 3.1 / 2.4 | Passed (6) | yes: node 2 stopped serving after 40 ms, node 3 after 43 ms (second run 56 / 58 ms) | result panel says "requests sent: 0, bytes transferred: 0" although it sent 8 GETs (P4) | should fix |
| deadnode | yes | 12.4 / 6.8 | Passed (5) | yes: 6,000 requests, 0 errors, 25 retried on another node, node 3 back to Active | final line "local -1904, peer -132, origin -192, coalesced -65" negative counters (P3); raw JSON member states | should fix |
| deletestorm | yes | 31.0 / 29.4 | Shows (4) | yes: 400 DELETEs all 204 in 446 ms; node 2 cleared all 260 keys: first 36 ms, median 4171 ms, last 8616 ms | the long silent wait (about 9 s) has a note but no progress; honest | polish |
| burstdelete | yes | 23.1 / 21.2 | Passed (4) | yes: 200 DELETEs, last key cleared after 6802 ms | result panel "requests sent: 0, bytes: 0" (P4) | should fix |
| peerexpiry | yes | 13.9 / 12.9 | Passed (3) | yes | 10 s of waiting with no countdown | polish |
| freshness | yes | 13.9 / 12.4 | Passed (63 assertions) | yes: each line names its rule; 103 lines | long; fine | - |
| eviction | yes | 17.3 / 18.1 | Passed (27) | yes: resident max 8,140,015 of 8,388,608, 239 evictions | prints "GET2 502 undefined/undefined" (P8) | polish |
| largeobjects | yes | 3.0 / 1.7 | Passed (4) | yes: 0.5 MiB kept, 1.3 MiB served not kept, 2.6 MiB 502 | the 502 is labelled "Blocked as expected ... This is the security working" (wrong explanation) and shows as "502 ," with no reason text; a local Windows path appears in the feed | polish |
| originprotect | yes | 6.1 | Needs attention on this OLD binary (origin peak 60 vs cap 6) | yes | see P6: the page says "That step was slow or did not finish" and retries a deterministic failure. Needs the newer binary; not retested | should fix (message) |
| operability | yes | 13.8 | Passed (31) | yes | none | - |
| restart | yes | 6.2 / 5.4 | Passed (4) | yes: outage 1,660 ms, 2,384 clean ECONNREFUSED, 381 requests after recovery, 0 wrong bytes | the word "crash" and 2,384 "failed attempts" appear on a happy path; they are explained in the feed | polish |
| intelligence | yes | 1.5 | Needs attention on this OLD binary (registry 404) | yes | P6; needs the newer binary; not retested | should fix (message) |
| intelcontrol | yes | 1.5 | Needs attention on this OLD binary | yes | P6 | should fix (message) |
| dashboards | yes | 7.7 / 6.4 | Shows (2) | yes: 6,000 requests, gateways counted 6,000 | feed says "about 20 seconds ... OPEN THE CONTROL CENTER NOW" but the run ends after 6 s (P5) | should fix |
| benchmark | yes | 29.2 / 24.1 | Shows | yes: per-repetition lines for direct / single / cluster at two origin profiles | card says "2-4 min", real 24-29 s; labels "O0 / O2" are not explained | polish |
| everything | yes | 213 s for 27 scenarios (card says 6-8 min / 10 min) | 19 Passed, 5 Shows, 3 Needs attention (originprotect, intelligence, intelcontrol: old binary) | yes | session summary is cumulative (P2) | should fix |

Scenarios that did not pass one-click: tour (step 5 broken), multiuser (Control Center absent here), originprotect, intelligence, intelcontrol (old binary). All others passed with one click, no manual step, the same verdict twice, and after a manual stop of node 3.

Console: no errors or warnings except `GET /favicon.ico 404` on every page load (P9). No token or secret visible in any screenshot or feed line I read. No horizontal overflow at 1440 or 390 px (measured: scrollWidth 390 vs innerWidth 390).

Hostile checks (all reproduced):
| check | result |
|---|---|
| Run twice in a row | same verdict and same checks every time (27 scenarios, two passes) |
| Double click on Run, human pace (150 ms apart) | 1 job. Two clicks 10 ms apart (script speed) created 2 jobs (`server.mjs:138-141`: `await baseline()` runs before `current` is set); polish |
| Stop during "scale" | verdict "Stopped", feed says Finished, cluster still 3 of 3 running |
| Reload during a run | feed replays, title and run buttons correct, run finishes normally |
| Copy the log | toast "Log copied." |
| Run again / Next scenario buttons | present on every finished card; disabled while another job runs |
| Stop node 3 by hand, then run "invalidation" | banner "Node 3 is stopped ... Restart node 3 or Reset demo"; running the scenario restarted node 3 itself (feed line "node 3 was stopped, restarting it"), passed in 4 s |
| Reset demo | native confirm dialog, cluster stopped then whole again; BUT the session summary is not cleared (66 runs, 7 need attention) and the origin counter goes negative (P2, P3) |
| Phone width 390 px | no overflow; the live feed is above the catalogue, so after pressing Demonstrate far down the page nothing visible happens (P7) |

## 2. Top 10 problems (priority order)

**P1 BLOCKER: the guided tour step 5 "Is it safe?" fails with "unknown scenario".**
Repro: click "Take the 5-minute guided tour", Next four times, "Run this step". Screenshot `41_tour_step5_unknown_scenario.png`. Cause (verified in code): `index.html:370` sets `run: 'security'`; the catalogue (`server.mjs:38-70`) has no such id, `startJob` returns `{error:'unknown scenario'}` (`server.mjs:136`). The very first thing the student is told to click ends in a broken step. Fix: in `index.html:370` use `run: 'auth'` (and say so in the text), or add a `security` meta-scenario to the CATALOGUE that queues auth, roles, isolation, targetsafety. Add to `hub_check.mjs`: walk every tour step that has `run` and require a verdict. (hub_check today only runs step 2.)

**P2 SHOULD FIX: the session summary is cumulative and survives Reset, so it shows alarming history.** Repro: run everything once on any binary, press Reset demo; strip still says "66 run(s) of 28 different scenario(s) of 30: 58 passed or showed, 7 need attention, 1 not run or stopped". A re-run that passes still leaves the old "need attention" counted. Fix: `index.html:177` count the latest job per scenario (a Map keyed by scenario), show "N of 30 scenarios: X passed, Y need attention, Z not run yet", and clear `S.jobs` display on Reset (`server.mjs` /api/cluster reset). 

**P3 SHOULD FIX: negative counters after a gateway restarts.** Repro: run "Kill a node while traffic runs": last feed line "gateway responses during this run (after minus before, all nodes): local -1904, peer -132, origin -192, coalesced -65". Also after Reset the header strip says "origin requests since start -13998" (screenshot `52_phone_run.png`). Cause: restarted processes reset their counters; the subtraction is blind (`server.mjs:125` clean-up diff, `index.html:269-275` origin diff). Fix: if any difference is below zero, print "unavailable: a gateway or the origin was restarted during this run" (labelled unavailable) instead of the numbers; reset `originAtStart` when the origin count goes down.

**P4 SHOULD FIX: result panel numbers are 0 for scenarios that did real work.** Repro: run "Delete a key everywhere" or "Burst of deletes": "requests sent: 0, bytes transferred: 0" next to a feed that shows GETs and 200 DELETEs. Fix: in the summary the scripts emit (`real_requests_scenario.mjs`, phase `invalidation` and `burstdelete`) count GETs and DELETEs sent, or hide the "requests sent / bytes" rows when the value is 0 and the scenario sent control requests only (the numbers array is built in `server.mjs:222`).

**P5 SHOULD FIX: "Live dashboards" is over before the student can open a screen.** Repro: Demonstrate on card 28; feed says "Sending steady real traffic (about 20 seconds) ... OPEN THE CONTROL CENTER NOW" and finishes in 6.4 s (6,000 requests, 4,122 in the last 3 s). Fix: `scenarios/extra.mjs:145` (dashboards) pace the traffic by time until 20 s have passed (a fixed rate such as 300 requests/s), and in the card add an "Open the Control Center" button (`openCC`) that is enabled while it runs; update the catalogue estimate (`'25 s'`).

**P6 SHOULD FIX: a missing capability is reported as a generic failure and retried.** Repro (old binary): "Policy engine" prints "NEEDS ATTENTION ... this build does not include the policy engine API yet", then "That step was slow or did not finish as expected ... retrying once", runs the identical failure again and ends "Needs attention" (and red in the strip). Same for "Origin protection" (cap not present). Fix: in `server.mjs:218-231` do not retry when the script's failing check text contains "this build does not include" (or exit with a dedicated code); finish with a neutral verdict "Needs the newer gateway build" (not red) and exclude it from the session "need attention" count; make the start batch file's build step guarantee the right binary.

**P7 SHOULD FIX: on a phone-width screen the live feed is out of sight.** Repro: 390 px wide, scroll to card 20, press Demonstrate: the feed is about 4,500 px above (`getBoundingClientRect().top = -4576`); `index.html:56` makes `.feedcol` static below 1050 px. After a reload the previous run's feed is the first thing under the header, before "Start here" (screenshot `50_phone_top.png`). Fix: below 1050 px make the feed a bottom sheet (position:fixed, collapsed to a one-line status with the progress bar, tap to expand), or call `feed.scrollIntoView()` on job start; do not replay the last job's feed above "Start here" on load.

**P8 POLISH: wrong or ugly text in the feed (each is one line to fix).** (a) "Large objects": the 2.6 MiB 502 is tagged "Blocked as expected" with "This is the security working" (explanation belongs to the auth steps); the 502 line has no reason text ("-> 502 , 12 ms"). (b) "Who may come in": the garbage-token request reads "sent GET ... as no token" (should say "as a garbage token"). (c) "Memory budget": "GET2 502 undefined/undefined" (`eviction_scenario.mjs`, missing header printed raw). (d) raw JSON objects inside sentences (stampede sources, mtls handshake, cluster member states). (e) empty events render as a bare dot with a timestamp. (f) a local path ("results in D:\...\hub\largeobjects") is printed. (g) "known gaps listed: 8" on four safety cards has no meaning for a stranger; label it "known limitations documented" or hide it. (h) the benchmark labels "O0 / O2" are not explained ("fast origin / 100 ms origin").

**P9 POLISH: small page-level things.** (a) `favicon.ico` 404 in the console on every load: add `<link rel="icon" href="data:,">` in `index.html` head. (b) The "Guided tour" and "Demonstrate everything" cards print "starts its own gateway on separate ports" (they do not; `index.html:189` uses the same note for every card without `needsCluster`; use a per-card note for `client:true`). (c) Step 1 of "how this demo works" says "Start the cluster" even when it is already running; show "Done: the cluster is running". (d) The time estimates are wrong in the safe direction: whole storyline 213 s measured against "about 6-8 minutes" and "10 min"; benchmark 24-29 s against "2-4 min". (e) Reset uses a native `confirm()`; fine for a person, but it blocks automated browsers and looks old; a small inline "Yes, reset" button would be nicer.

**P10 POLISH: counters that do not belong to the scenario.** For the scenarios that start their own gateway and origin (183xx / 193xx) the header "Origin requests" and the feed line "origin requests since start 0 (counted by the origin)" refer to the demo origin and read 0 or unrelated numbers (e.g. "origin requests counted by the origin process during this scenario: 0" after Large objects). Hide these two lines for `needsCluster:false` cards or label them "demo origin (not used by this scenario)". Related risk: those scenarios use fixed ports (18300, 18310, 18320, 18330, 18340, 19300, 19700; `server.mjs:57-66`); I saw 18340 and 19300 already in use by another process at one point, so a second hub or another tool on those ports would make them fail with a confusing message. A "port busy, try again" message and a port-base option for the hub (`--own-port-base`) would remove the risk.

## 3. Screenshots (all under `D:\ClaudeTemp\lanes\evidence\L1b\`; I read each one named in section 2)

- Landing: `01_landing.png`, `01_landing_full.png`
- Mid-run: `stampede_midrun.png`, `gallery_midrun.png`, `scale_midrun.png`, `deadnode_midrun.png`, `31_everything_midrun.png`, `45_prepare_fix_node3_midrun.png`
- Results: `auth_result.png`, `deadnode_result.png`, `largeobjects_result.png`, `originprotect_result.png`, plus one `<id>_result.png` for every scenario id
- Session summary: `32_session_summary.png`, `33_end_top.png`
- Phone (390 px): `50_phone_top.png`, `51_phone_full.png`, `52_phone_run.png`, `53_phone_after_run.png`
- Something wrong: `41_tour_step5_unknown_scenario.png` (P1), `52_phone_run.png` (P3 negative origin count, P7), `44_node3_stopped.png`, `46_reset_in_progress.png`, `46_after_reset.png` (P2), `multiuser_result.png`, `intelligence_result.png`
- Tour and help: `40_tour_step1.png`, `41_tour_step2.png`, `47_help.png`
- Raw per-scenario data (feed text, timings, console): `data\*.json`, `data\*.log`, `logs\`

## 4. What I could not test, and why

- Scenarios 23, 26, 27 with a gateway that has the static origin cap and the policy engine: the only newer-looking binary was dated 07:21 (before the 10:00 cut-off).
- Scenario 14 (two windows with the postMessage hand-off) and the six Control Center screens in 28: the Control Center was not running, `ui/node_modules` is absent from the snapshot, and headless Chrome blocks pop-ups.
- The cluster-down landing state and the "Start the demo cluster" button, and the full batch file (it uses ports 3000/3201/18185 and builds with npm, which I did not touch).
- A real mobile device, screen readers, keyboard-only navigation beyond the tour arrows, colour-contrast measurement.
- More than two repetitions per scenario, and any timing under load from another program (timings are single runs on a busy workstation, loopback; not benchmarks).
- Dropped connections or a slow disk during Reset; I did not kill the hub while a run was in progress.
