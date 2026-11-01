# Package 01: visual design review

**Status:** Draft for owner review. **Audience:** a human visual reviewer (no need to know the code), and the owner who reads the result. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`. **Authority for the criteria:** `docs/design/critique_rubric.md` (itself "to be reviewed by a human before it is called validated"; this review is also that human review of the rubric, see section 9).

**Build under review (fill before starting):** commit or tree `[FILL]`; UI build `[FILL: dev server or production build]`; gateway build `[FILL: release or debug]`; backend includes the U-01 patch? `[FILL: yes/no; the patch is approved but integration was in progress on 2026-10-30]`. Reviewer code `[FILL]`; browser and version `[FILL]`; OS `[FILL]`; screen size and pixel ratio `[FILL]`; date `[FILL]` (must match the log in 00_README section 6).

## 1. What this review is for
To have a person look at the real screens, in the states that matter, and decide for each of the rubric's 12 criteria whether the screen passes, and for each of the 9 must-pass items whether the screen is allowed to ship. It is judged by eye on a screen, not from a script. It does not replace the accessibility (03), browser (04) or usability (02) packages.

What it cannot establish: that unlisted states are fine; that people understand the screens (that is package 02); that assistive technology works (03). Taste is not a finding. A finding is something you can point at and a second reviewer can see.

## 2. Ground rules
1. Open and look at the actual screen. A screenshot you did not open was not reviewed. Record the screenshot path or write "live" for each row.
2. Check the **must-pass items first** (section 5). A failure there is a defect before any polish.
3. Score each criterion PASS or FINDING. No numeric scores. No "looks modern" or "feels premium"; say what you see.
4. Every FINDING gets an ID (`VIS-001`, `VIS-002`, ...), a severity, the screen and state, the viewport and theme, the criterion number, what is wrong in observable terms, the evidence path, and an owner to fix it (leave owner blank for the owner to assign).
5. Severity (from the rubric): **Blocking** = false or misleading claim, data loss, unusable. **High** = task fails or likely misread. **Medium** = task slowed, or an accessibility gap with a workaround. **Low** = polish or inconsistency with no task impact.
6. If a state cannot be reached, write **NOT REACHABLE** and why. Do not guess what it looks like. Do not substitute a mock-up.
7. If you disagree with the rubric itself, record it as a rubric note (section 8), not as a finding against a screen.
8. If you run into a value that looks like a number from nowhere (an invented score, a verdict on a run that did not finish), stop and record it as Blocking against criterion 10; do not wait until the end.

## 3. Screens and states to review
This is the rubric's representative set (`critique_rubric.md` section 5). Routes come from `ui/src/lib/routes.js`. "Needs gateway" means a gateway must be running.

| # | Screen | Route | State(s) to reach | Needs gateway |
|---|---|---|---|---|
| S01 | Overview | `#/overview` | connected; disconnected | yes |
| S02 | One live data page (pick Traffic and latency) | `#/traffic` | connected; disconnected; an unavailable value (for example hit ratio with no requests) | yes |
| S03 | Phantom Lab home (About) | `#/lab` | connected; disconnected | yes |
| S04 | Start a run (run form) | `#/lab/run` | default; a validation error; a preset chosen | yes |
| S05 | Run in progress | `#/lab/runs/<id>` | running; cancelling | yes |
| S06 | Completed Scenario B result | `#/lab/runs/<id>` | completed (and the retained one under S10) | yes |
| S07 | Cancelled run | `#/lab/runs/<id>` | cancelled (the cancel dialog too) | yes |
| S08 | Failed run | `#/lab/runs/<id>` | failed (and interrupted) | yes |
| S09 | Runs list | `#/lab/runs` | empty; with runs | yes |
| S10 | Evidence index and Scenario B evidence | `#/lab/evidence`, `#/lab/evidence/<id>` | index; the retained Scenario B result; the retained M6 result | yes |
| S11 | Simulation | `#/simulation` | default; page loaded with no gateway | no |
| S12 | Cluster | `#/cluster` | not available | yes or no |
| S13 | Intelligence | `#/intelligence` | not available | yes or no |
| S14 | Connection popover | header connection control, opens a "Gateway connection" panel | connected; disconnected; after "Test connection" | yes |
| S15 | Connection lost (whole page) | any page | gateway stopped after loading | yes |

### Viewports and themes
The rubric asks for 320, 390, 820 and 1280 CSS px wide, in light and dark, plus 200 percent root text at 390. That is 8 combinations per screen state and about 120 screen-states, which may be more than one reviewer can do well. **Proposed tiers (owner to confirm or change before the review):**

| Tier | What | Coverage |
|---|---|---|
| A (must do) | The 9 must-pass items, on every screen in the table | 1280 light and 390 dark; plus 200 percent text at 390 for S04, S05, S06, S07 |
| B (should do) | All 12 criteria | 1280 light, 1280 dark, 390 light, 390 dark for S01, S03, S04, S06, S07, S08, S10, S11, S14 |
| C (if time) | The rest | 320 and 820, light and dark, remaining screens |

Record the tier you reached. `[FILL: owner confirms tiers]`

You can resize a desktop browser window or use the browser's device toolbar for 320, 390 and 820. The real phone view is covered by package 04; this package may use the phone as an extra viewport but the browser-matrix cell for the phone belongs to 04.

## 4. How to open each state (fixtures)
All commands come from project documents; they were not re-run when this package was written. The first person to run them should correct this section where it is wrong.

**Fixture rule (rubric must-pass 2):** pages for cancelled, failed and interrupted runs must come from real gateway output, not from hand-edited objects. A page built from a hand-set object is not evidence for these states.

### Common setup
1. Start the gateway in anonymous development mode with the Lab enabled (see `00_README.md` section 7). Put only the retained evidence you want reviewers to see in the Lab folder: for the M6 result copy `verdict.json`, `manifest.json` and `analysis_*.csv`; for retained Scenario B copy `manifest.json` and `summary.json` (this is the arrangement the AI usability run used; `04_validation_usability.md`, Setup). `[FILL: path of the prepared Lab folder]`
2. Start the UI dev server and open `http://localhost:3000/?gateway=http://localhost:8085` (change the port to match your gateway).
3. Check the header says connected (the word "Live") before testing any state.

### Per state
| State | How to reach it | Notes and limits |
|---|---|---|
| **Connected** | Gateway running; open any page. Header control shows Live. | On a debug gateway the Lab says results are EXPLORATORY. That is correct behaviour, not a finding. |
| **Disconnected** | With a page open and connected, stop the gateway process. Within about 3 seconds the header control changes and values show as stale (measured by the agent run in `03_validation_accessibility.md`, check 10). Alternative with no gateway at all: open `http://localhost:3000/?gateway=http://localhost:<an unused port>`. | Wording to expect is defined in `06_functional_ux_work_breakdown.md` section 4.2 (Disconnected, Reconnecting, Unauthorised, Unavailable, Stale). The page cannot tell stopped, wrong address and blocked-by-CORS apart; the page should say so rather than guess. |
| **Run in progress** | Lab, Start a run, choose the Quick check preset, Start run. On a debug gateway a quick check took about 11 seconds in the AI run, so for a longer window use the advanced fields to lengthen the phases. | Progress wording and the empty bar were hypotheses U-03/U-04; judge what you see. |
| **Cancelled run** | Start a run with longer phases, and while it is running press Cancel and confirm in the dialog. | The dialog's safe option ("Keep running") should have focus. **Must-pass 2 applies:** a cancelled run must show no verdict, no mechanism result and nothing that looks like a validity approval. If the gateway build does not contain the U-01 backend patch, the backend may still send a verdict-like label; record the build and note it as a possible backend cause, not only a UI defect. |
| **Failed run** | **No verified recipe exists in the documents.** The AI accessibility run notes that a failed run "was not triggered (no run failed)" and the coverage matrix lists failed and interrupted runs as fixture-only. Ask the builders for a real failed run produced by the gateway and copied into the Lab folder, or a documented way to make one fail. A candidate (UNVERIFIED hypothesis): start the gateway with `PHANTOM_LAB_GATEWAY_EXE` pointing at a readable file that is not a valid program, then start a run; the configuration check only requires a readable regular file. | If you cannot reach it, mark S08 failed state **NOT REACHABLE** and put it in the open list (section 8). Do not review a mock-up. `[FILL: builders supply recipe or fixture]` |
| **Interrupted run** | Start a run, kill the gateway process while it runs, then start the gateway again with the same Lab folder. The gateway marks unfinished runs on startup (`docs/user/lab_api.md`, startup recovery). Open the run. | Same must-pass 2 rule as cancelled. Not observed by any prior agent run in a browser. |
| **Completed Scenario B result (live)** | Quick check, wait for the page to switch to the result. | With one repetition the AI run got "Inconclusive: only 1 valid paired runs; 10 required", which is the correct reading for a quick check. Check that the answer comes before the long interpretation notice and that the binding notice is present and not collapsed (must-pass 4). |
| **Completed Scenario B result (retained)** | Evidence, then Scenario B. | A retained result is REAL with a qualifier plus a separate RETAINED indicator (DEC-007). |
| **M6 retained result** | Evidence, then M6 sealed evaluation. | Negative and positive results must have equal visual weight (must-pass 5). |
| **Simulation** | Open `#/simulation`. Also open it with no gateway running. | Illustrations must carry "Illustration. Not live data."; nothing may look like a live demo. |
| **Cluster, Intelligence** | Open `#/cluster`, `#/intelligence`. | These are honest placeholders ("Nothing below is a value"). Judge that they say what is missing and what to do, in the same words as the rest of the product. |
| **Connection popover** | Click the connection control in the header (a button that opens a panel titled "Gateway connection"). Open it connected and disconnected. Try "Test connection" and "Change address" if present. | The popover exists in the working tree (`ui/src/screens/linkUi.jsx`) but no human has reviewed it. If it is absent in the build under review, record NOT PRESENT; do not score it as a pass. Never type a real secret into a field you do not trust; the token must not appear in the address bar or any visible text (criterion 12). |
| **Unavailable value** | Open Traffic and latency on a gateway that has served no requests. | Must show "Unavailable" plus a reason, never zero (must-pass 1). |
| **Empty runs list** | Use a Lab folder with no runs. | Fixture `runs_empty` exists for tests only; use a clean Lab folder in the browser. |

## 5. Must-pass items (check first; one row per screen state reviewed)
Source: `critique_rubric.md` section 3. Mark each row Y (satisfied), N (failed; raise a finding) or NA (not applicable; say why).

| ID | Must-pass item | How to tell by eye (observable) |
|---|---|---|
| M1 | Provenance mark on every value; UNAVAILABLE gives a reason, not zero | Every number, chart and verdict has a visible label from the six words (REAL, SIMULATED, DERIVED, REPLAYED, PREDICTED, UNAVAILABLE). Click or hover a value you doubt. Any unavailable value shows a reason in words. |
| M2 | No verdict, mechanism result or approval-style label on a cancelled, failed or interrupted run | On S07 and S08 there is no result chip, no "VALID"-style label, no "mechanism failed/passed", no "hypothesis falsified". The page says what happened and what is known. |
| M3 | One connection component and vocabulary on every screen; "Unauthorised" and "Recovering" only where the code can tell | The same words in the header, popover, Lab strip, page body, and run stream for the same condition. |
| M4 | Limitations and the binding interpretation notice present and not collapsed on evidence pages | On S10 and S06 you can read the limitations and the notice without clicking anything open. |
| M5 | Negative and positive results have equal visual weight | Compare the M6 and Scenario B cards: same size, colour weight and position for a negative result as for a positive one. |
| M6 | Charts: table twin with identical values; no tweening or smoothing of measured data | Each chart has a Table view; the numbers match; no animated count-up on measured values. |
| M7 | Survives 200 percent root text at 390 px: no clipped primary action, no horizontal page scroll | Set root text to 200 percent (browser setting or devtools), width 390. The primary action is fully visible and reachable. No sideways scroll of the whole page. |
| M8 | Reduced motion and reduced transparency leave a complete, legible experience | Turn on both in the OS. Nothing is missing, nothing is unreadable, nothing keeps moving. |
| M9 | Token is memory-only; the gateway address is the only value persisted | Look at the address bar and visible text for any token. (Storage inspection is done in package 04, not by eye.) |

Result sheet for must-pass items:

| Screen / state | Viewport / theme | M1 | M2 | M3 | M4 | M5 | M6 | M7 | M8 | M9 | Evidence path or "live" | Finding IDs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `[FILL]` | `[FILL]` | | | | | | | | | | | |
| `[FILL]` | `[FILL]` | | | | | | | | | | | |

## 6. The twelve criteria: what to look at
Use these as yes/no questions. They are condensed from `critique_rubric.md` section 2; if there is a difference, the rubric governs.

| # | Criterion | Yes/no checks to make on the screen |
|---|---|---|
| 1 | Visual hierarchy | Is there one clear page heading? Is the primary answer or action the first thing you read and the most prominent? Can you tell apart section titles, labels, metadata and supporting text? Are there two competing primary buttons? Are large numerals used only for real, labelled values? |
| 2 | Information architecture | Does the page answer its question before the technical record? Do nav labels match page titles? Can you tell where you are? Is the same thing called the same name on every screen? |
| 3 | Comprehension and cognitive load | Is every technical term explained where it first appears or one click away? Any paragraph longer than about 5 lines with no heading or visual? Is an important result buried under a long notice? More than one top-level status visible at once? |
| 4 | Visual consistency | Do buttons, segmented controls, status marks and tables look and behave the same everywhere? Do light and dark both look designed (no leftover inverted bits)? Do provenance marks look identical on every screen? (Tokens in code are checked by tools, not by eye.) |
| 5 | Interaction clarity | Does every control have a visible label and a visible focus ring? If the main action is disabled, does the page say why? Does a destructive action ask for confirmation with the safe option focused? Does every press give feedback? Are touch targets large enough (at least 44 px for primary touch controls on phone)? |
| 6 | Motion quality | For each animation, can you name the real event behind it? Does anything loop for decoration? Does anything move on a stale or reconnecting state? Are measured numbers ever tweened? With reduced motion on, is the experience complete? |
| 7 | Responsive behaviour | No sideways page scroll from 320 up? Nothing clipped or overlapped (sticky bars too)? Do tables stay readable without silently losing columns? Do long identifiers wrap or truncate with the full value reachable? |
| 8 | Accessibility (visual part) | Colour never the only carrier of meaning? Text legible against its background? Charts have a table twin? Forced-colours mode keeps selection and focus distinguishable? (Screen reader and keyboard are package 03; axe is a tool, not an eye check.) |
| 9 | Performance (visible part) | Does anything jump or shift as the page loads? Any laggy scrolling on your machine? (Measurements are tool work; record only what you felt and on which machine. No performance claim comes from this package.) |
| 10 | Technical truthfulness and provenance | Does any value look invented, an overall score or verdict that nothing computed? Does a negative result carry the same weight as a positive one? Is the binding Scenario B notice present and not collapsed? Are illustrations labelled "Illustration. Not live data."? Does anything imply live predictive intelligence, a canary or a rollback? |
| 11 | Product identity | Does it read as an instrument and evidence product, not a generic dashboard (no card-per-datum grids, decorative gradients, neon, glass over data)? After one screen, can a stranger say in one sentence what this is for? Ask a person nearby and write down what they said. |
| 12 | Error, empty, loading, disconnected states | For each state in section 3: does it say what happened, what is known, and one next action? Same words and component on every screen for the same condition? Does connection status read as an observation by this page, never a statement about server health? |

### Scoring and finding sheet
| Screen / state | Viewport / theme | Criterion # | PASS / FINDING | Finding ID | What is wrong (observable) | Evidence path | Severity | Owner |
|---|---|---|---|---|---|---|---|---|
| `[FILL]` | `[FILL]` | | | | | | | |
| `[FILL]` | `[FILL]` | | | | | | | |

Use one row per criterion per screen state reviewed, or one row for "all PASS" per screen with the criteria listed.

## 7. Observable acceptance criteria for this package
**The package is complete** when: every Tier A row is filled for every screen state in section 3 that is reachable; every unreachable state is listed as NOT REACHABLE with the reason; every must-pass row has Y, N or NA with a reason; the reviewer has opened and read the actual screens (evidence path or "live" on every row); and the sign-off block is filled.

**The design passes human visual review** only if all of these hold, as recorded:
1. No must-pass item is N on any reviewed screen state (no Blocking finding open).
2. No Blocking or High finding is open against any criterion on the reviewed set. Medium and Low findings may be open if the owner accepts each by ID.
3. Every state in section 3 is either reviewed or an owner-accepted NOT REACHABLE.
4. The reviewer, build, browser, OS and date in the header match the log in `00_README.md`.

Passing means "a named human, on this build and these viewports, found no Blocking or High visual defect against the rubric". It does not mean the design is good, accessible or usable.

## 8. Open items and rubric notes
- Rubric notes (things you think the rubric gets wrong or misses): `[FILL]`
- States not reachable: `[FILL]`
- Anything you could not tell without reading the code: `[FILL]`
- Previously known visual-adjacent hypotheses to look for (from AI work; confirm or not observed, never count as human findings): empty progress bar with "0 of 2 units" (U-03); preset has no selected state (U-04); answer below long notice (U-05); jargon on the M6 page (U-06); "VALID" read as success (U-07); stale "mode: live" while disconnected (U-08); "not available" wording mismatch (U-09); sticky bar overlapping the form on phone (U-10). These came from an AI walkthrough of an earlier build and may have changed.

## 9. Sign-off
Reviewer code `[FILL]`; date `[FILL]`; scope covered (tier, screens, viewports) `[FILL]`; result `[FILL: PASS / FAIL / PASS WITH ACCEPTED FINDINGS]`; owner acceptance of Medium/Low findings by ID `[FILL]`; owner date `[FILL]`. Until the owner records this, the visual status stays **TESTED (agent-driven)**.
