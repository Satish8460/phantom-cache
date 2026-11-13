# LANE-L1 handover (Demo Hub: 35 one-click scenarios, live feed, five policy-engine cards)

Lane copy: `D:\ClaudeTemp\lanes\L1\`. Gateway for the final test: `D:\ClaudeTemp\audit\phantom_rev10.exe` (release, policy engine with hot-key, cardinality, frequency, prediction and flash-crowd detectors, origin cap). No cargo run, no manifest or dependency change. Evidence: `D:\ClaudeTemp\lanes\evidence\L1\hubC\` (screenshots and hub_work results), `hubC.log` (full hub_check output). Tags: reproduced here unless stated; one host, loopback, release binary.

## 1. Result of this order (policy-engine cards)
| id | what it does (real traffic through node 1 of the demo cluster, tenant A token held by the driver) | evidence in the run |
|---|---|---|
| `pie-hotkey` | 16 s, one key about 1/3 of the requests plus 200 others | engine found the hot key (hash only, share 0.335-0.347, confidence 1.0), classification `hot_key_dominated`, distinct-keys estimate 102 +/- 3.3 vs true 101 (run 1) and 151 vs 150 (hub run) |
| `pie-flashcrowd` | 9 s calm baseline (3 workers), then 7 s burst on one key from 40 workers | detector fired about 0.9 s after the burst began: rate ratio 2.22, top-10 share 0.91 vs baseline 0.41, heuristic confidence 0.6 (declared rule score), decision record POL-FLASH-1 `Recommend (rate_change)`, applied false. If it does not fire the card prints the largest values seen instead (code path written, not exercised: it fired in every run) |
| `pie-shift` | 22 s uniform over 290 keys, then 22 s with one key at half | top-1 share 0.014 -> 0.532, 7 new decision records (POL-ORIGIN-1 NoAction, POL-FLASH-1 Recommend, POL-EVICT-1 NoAction). The classification LABEL did not change in my run (it stayed `hot_key_dominated`, carried over from an earlier card on the same gateway, re-evaluated only every 10 windows); the card says so plainly and shows the top-1 share change. This card therefore cannot reliably show a label change when run straight after `pie-hotkey`. |
| `pie-eviction` | 26 s scan (never-repeated keys) mixed with a 20-key hot set | classification `scan_cold`, hit ratio 0.31, working set 1024; shadow candidates LRU and S3-FIFO rejected (below_margin, estimated change 0, 95% interval about [-0.025, 0.025]); the card states SIEVE and TinyLFU are not listed as candidates in this engine version's record, shadow capacity 1,024 entries on a 1/16 sample, real serving (SIEVE) unchanged |
| `pie-admission` | 14 s, 90% never-repeated keys, 80 concurrent clients | window with 2,977 events: F18 1.0, F19 0, F20 0, F21 0.012, F22 0.101 with 168 sampled keys, distinct keys est 2,857, one-hit-wonder ratio 1.0; POL-ADMIT-1 NoAction (no_candidate_exceeds_margin). Below the minimum sample count the card prints "unavailable, insufficient samples" (code path written; in the first attempt it showed exactly that because the window was read after traffic stopped: the card now reads the richest window DURING traffic) |
| `intelcontrol` (existing) | added a cold-start step before any traffic: classification/recommendation must be absent ("not enough evidence") | passed in the storyline and individually |

Every PIE result carries the ribbon words OBSERVED / SHADOW-EVALUATED / PROPOSED on the individual steps, a legend on the card, and the line "APPLIED: none: the engine never changes how requests are served" in the feed and the card. Nothing says canary, committed or rolled back; no adaptive TTL; no claim that the engine improved anything. Verdict is `shows`; `attention` only if a check fails (traffic not answered 200, hot key not found, F23 estimate outside 3 standard errors or 30%, shift without a top-1 rise, flash-crowd block missing).

Cards that could not be made to show the full evidence in every run: `pie-shift` (label change not guaranteed, see above); `pie-eviction` (no candidate beat the incumbent; the card shows the what-if numbers, which are all about 0, and does not claim a recommendation); the flash-crowd "did not fire" branch and the admission "unavailable" branch exist but were not triggered in the final runs.

## 2. hub_check against my own cluster (rev10, seed 7, 1 storyline + 32 individual runs)
`node scripts/demo_console/hub_check.mjs --port 3203 --scenario-offset 500 --exe phantom_rev10.exe --evidence <dir>`: **118 PASS, 0 FAIL.**
- 35 required ids (30 original + 5 pie) each have a card with a Run button, in order (35 PASS plus order check).
- Storyline ("Demonstrate everything"): 32 scenarios, all Passed/Shows (includes originprotect, intelligence and intelcontrol, which now pass on rev10).
- 32 individual runs in random order (the multiuser run is separate): all verdicts Passed/Shows with numbers; cluster healthy at the end; node 3 auto-restart check; phone width no overflow; no token on the page; zero console errors/warnings (favicon fixed).
- Not run: 5x repetition and 3x storyline (time). Earlier first attempt at this run failed 8 own-gateway scenarios with `EADDRINUSE 19300` because the owner's hub (port 3201, same scenario port block) was running concurrently; this is why `--scenario-offset N` now shifts the 183xx/193xx/197xx block (default 0).

## 3. Files (relative to lane root; changed or new in this order)
- New: `scripts/demo_console/scenarios/pie.mjs`
- Changed: `scripts/demo_console/server.mjs` (5 cards after `intelcontrol`, `--scenario-offset`), `scripts/demo_console/index.html` (event ribbon tag, PIE legend), `scripts/demo_console/scenarios/extra.mjs` (cold-start step in `intelcontrol`), `scripts/demo_console/hub_check.mjs` (5 new required ids, `--scenario-offset`), `docs/lanes/LANE-L1.md` (this file)
- Unchanged from the previous handover: everything else (see git history of the lane); the owner's light theme, Acting-as guide, two-click second window, tour fix, originprotect check fix and removal of the known-gaps list were preserved (I only added to those files).

## 4. Limitations and notes
- The PIE cards need a gateway with the policy-engine routes in Observe/Advisory mode (the demo cluster's default); otherwise the first check says so plainly. The window is the engine's fixed 1 s; decisions and the classification are re-evaluated every 10 windows, so cards run 16-50 s.
- The engine state is per gateway process and shared between cards run back to back; a card can therefore start with labels or top keys left by the previous card (`pie-shift`).
- F23 comparison uses the client's last N requests (N = events in the window): approximate alignment, stated in the feed.
- Signals/decisions are read with the tenant A token through the driver; the token is never printed or shown.
- Processes: my hub_check, hub, cluster and browser were stopped; the owner's hub on 3201 and the L1b copy were not touched.
