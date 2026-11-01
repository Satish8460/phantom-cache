# D-7 round 3 browser re-verification (2026-10-30, evidence dir 20261030T055008Z)

Written by the lead from the verifying agent's returned report (the agent could not write this file). Raw evidence is in this folder: `r3_*.results.json`, `logs/`, `shots/`. Observation only; nothing was fixed. PASS means "no failure observed at the sizes, themes and routes tested", not a conformance claim.

## Setup
- Gateway: debug build `D:\Personal\_m7_target\debug\phantom-cache-demo.exe`, sha256 `B31499965E96FDB0B98325EECBE26B8A1B202ECC3122121DE1BCFB0BCEA7E0E2` (as expected; `logs/gateway_sha256.txt`). All runs are EXPLORATORY (debug build).
- Anonymous gateway 127.0.0.1:8085, JWT gateway 127.0.0.1:18086 (invented secret, CORS http://localhost:3000, sealed salt unset), Vite localhost:3000 `--strictPort`, Chrome 156.0.8078.79 headless over CDP.
- Lab dir `D:\Temp\d7c_lab` held only copied M6 and Scenario B verdict/manifest/analysis/summary files. Run created: `20261030T055055Z-2ba246aa` (Completed).
- Repo before and after: HEAD 456bb25930004f1896a84c53e776e7b3ef29ee16, branch feat/publish-phantomcache-core, 72 `git status --short` lines both times.
- New scripts: `ui/scripts/browser-r3-{mkrun,layout,jwt,stall,cancel,clips}.mjs`.
- Caveats: Chrome relaunched every 3 cold loads in the layout script (N-2 tested separately in one profile); zoom emulated by viewport size (640x400, 320x256), not real browser zoom; only mobile:false tested; headless Chrome has a 15 px classic scrollbar. The script's pass flag used the lax round-2 rule (scrollWidth <= viewport width); the results below are recomputed strictly (scrollWidth > clientWidth).

## Results
| Check | Result |
|---|---|
| N-1a: Scenario B evidence and run detail at 390 | PASS. 375 = 375 (round 2: 557/552). |
| N-1b: debug banner at 320 and 400% zoom | PASS. All 7 Lab routes 305/305 with the banner visible (round 2: 355 vs 305). |
| F-01 reflow, 15 routes x 320/390/640/820/1280 | PASS on 74 of 75 (strict). The one overflow is `#/` at 320 (N-6). |
| Zoom 200% (640x400) | PASS on all 15 routes. |
| Zoom 400% (320x256) | PASS on 14 of 15; `#/` is 319 > 305 (N-6). |
| N-3 at 390 | No element below 24 px on any route. Summary rows 44 px (was 24). Below 44: skip link 39 px (visible on focus only); on `#/lab/run` native radios/checkboxes 24x24 (checkbox labels 44, radio labels 93-108) and 2 text inputs at 36; on `#/simulation` (non-Lab demo) 13 buttons at 36-38. OBSERVED. |
| N-3 at 1280 | Lab routes minimum 32 except the Start form (controls 24x24, labels 32 or 50-54). `#/simulation` FAILS the 24 px floor: 5 buttons at 21 px (RUN, PAUSE, 0.25x, 1x, 2x), 3 more at 26. |
| N-4: M6 caption at 1280 light and dark | PASS. Figcaption 983 px in a 983 px parent (was ~100 px). |
| N-5: operator-only token, pre-submit | PASS at 1280 and 390. |
| N-5: operator, post-submit | PASS. Run accepted; run page shows the required-role alert (telemetry:read, "admin satisfies both", gateway message, Retry) plus an info block with the run id saying the run was probably accepted. No horizontal scroll; focus on H1. |
| N-5: observer, admin | No regression. |
| N-2: repeated cold loads, one profile | NOT REPRODUCED. 21 cold loads of Lab routes all ready in 312-376 ms; 6 more of a running run page and a final one OK. ESTABLISHED sockets to :8085: 0 before, 2-4 after each load, max 5, never 6. |
| N-2: Lab -> away -> Lab | No growth over 36 hash hops (counts 2-5, oscillating). |
| Regressions: skip link, Menu button, cancel Escape, favicon, View as table | All PASS. |

## Round 2 vs round 3
| Item | Round 2 | Round 3 |
|---|---|---|
| N-1a | FAIL 557/552 at 390 | PASS 375/375 |
| N-1b | FAIL 355 vs 305 at 320 | PASS 305/305 |
| N-3 summary rows | 24 px | 44 at 390, 32 at 1280 |
| N-3 radios/checkboxes | 18x18 at 1280 | 24x24, labels 32/44 high |
| N-3 other | inputs 36, skip link 39 | unchanged |
| N-4 | ~100 px caption column | 983 px |
| N-5 | silent hint, bare 403 | hint before submit; 403 block explains and shows run id |
| N-2 | stalls after ~3 loads, 12 sockets | not reproduced in 28 cold loads, max 5 sockets |

## New findings
- **N-6 (Low, pre-existing, missed by round 2):** Overview `#/` at 320 px and 400% zoom scrolls horizontally by 14 px (319 vs 305). Offending `dd` elements: "Live Inferred", "under 1s ago Inferred", "http://localhost:8085". Round 2 recorded 319/305 but marked PASS under the lax rule, so round 2's "non-Lab routes pass at 320" was too generous. Not seen at 390 or wider; mobile:true not tested.
- **N-7 (Low, non-Lab Simulation demo):** buttons 21 px high at 1280 and 36-38 px at 390. Not measured in round 2, so not a regression.
- **Observation:** the header chip says "Disconnected" on the JWT gateway while Lab works with a token. Not investigated.

## Screenshots the agent opened
`shots/r3clips/` sb_390_table, sb_320_table, m6_caption_1280_light/_dark, start_form_390_checks, start_form_1280, banner_320; `shots/r3jwt/r3_jwt_1280_operator_post_submit.png`. The full-page reflow shots (305x12326, 375x11106) were unreadable when downscaled and support nothing. About 70 other PNGs were measured only.

## Not verified
mobile:true layout; real browser zoom; Safari/Firefox; screen readers; non-text contrast; release gateway or production bundle; Enter on the Menu button (recorded "true", hand-built CDP event, weakly supported); whether the round-2 N-2 stall depended on timing these runs did not hit, or whether the UI change fixed it (cause remains unknown); multi-tab use of one gateway origin.

## Artifacts and cleanup
Folder about 11 MB. Outside the repo: `D:\Temp\d7c`, `D:\Temp\d7c_lab` (includes the operator-only JWT run), `D:\Temp\d7c_ts.txt`. Gateways 4120 and 7516 and Vite 32508 killed by recorded pid; about 56 Chrome PIDs killed by recorded pid; throwaway profiles removed; ports 3000, 8085, 18086 free at the end. The user's Edge and the codex `node.exe` were not touched. No cargo, no git mutations, no existing repo file edited.
