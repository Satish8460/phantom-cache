# Package 03: accessibility review

**Status:** Draft for owner review. **Audience:** the screen-reader reviewer and the keyboard-only evaluator who run it, and the owner who reads it. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`. **Not a conformance audit:** this package gathers human evidence. It does not certify WCAG conformance.

**Build under review (fill first):** commit or tree `[FILL]`; UI `[FILL: dev server or production build]`; gateway `[FILL]`. Browser(s) and version `[FILL]`; OS and version `[FILL]`.
**Screen reader (the reviewer chooses and names it; do not assume):** name `[FILL]`, version `[FILL]`, paired browser `[FILL]`, speech or braille settings worth noting (rate, verbosity, browse vs focus mode behaviour) `[FILL]`. DEC-008 asks for at least one real screen reader; NVDA, JAWS and VoiceOver are the common choices but the reviewer decides, and results apply only to the one named.

## 1. What this review is for
The earlier accessibility evidence is agent-driven (headless Chrome 156 driven over CDP, `docs/design/remediation/03_validation_accessibility.md`) and its first line says automated and scripted checks do not replace an assistive-technology pass. No screen reader, real device or non-Chrome browser has ever been used on this UI (`r5_coverage_matrix.md`, rows K05, K08, K09: NOT COVERED). This package is that pass.

It establishes what the named reviewer experienced with the named combination. It does not establish what other screen readers do, nor full WCAG conformance.

## 2. Who does what
| Role | What they do | Notes |
|---|---|---|
| Screen-reader reviewer | Sections 4 and 5, using their own screen reader and settings | Must be a regular user of the tool they name; a sighted person following a tutorial is not equivalent. `[FILL: who]` |
| Keyboard-only evaluator | Section 6 with no mouse, trackpad or touch. May be the same person as the screen-reader reviewer only if they declare it; DEC-008 says keyboard-only navigation is evaluated as well | `[FILL: who]` |
| Visual and device checker | Sections 7 to 11 | Any careful tester. `[FILL: who]` |
| Recorder | Fills the template in section 13 | Can be the reviewer |

Record all names in the log in `00_README.md` section 6 before starting.

## 3. Hypotheses carried in from earlier AI work (to check, not to count as human findings)
These are from the automated run on an earlier build; the redesign may have changed them. Look for them; record observed, not observed or different.

| ID | Earlier agent observation |
|---|---|
| A11Y-01 | Run-id links in the runs list were below 24 px at 320 to 820 wide and wrapped one or two characters per line |
| A11Y-02 | At 390 px with 200 percent root text, page-level horizontal scroll and a clipped "Ready to start." in the sticky bar |
| A11Y-06 | Provenance wording outside the six-word vocabulary ("inferred", "measured", "observed", "Simulated illustration") |
| A11Y-03/04 | Theme buttons and Chart/Table toggles 26 px tall; "Details" 74 by 32 at 390 |
| A11Y-05 | Under reduced motion `.btn` still faded colours |
| A11Y-08 | Every Lab sub-view had the same h1 "Phantom Lab"; run form had a heading-level jump |
| A11Y-09 | A static "Before you start" notice was a polite status region and might be announced on load |
| A11Y-10/11 | Arrow keys did not move within the theme group or Lab sub-nav; in forced colours the selected and focused buttons shared one ring style |

Never tested by anything: announcement order and wording, whether live regions are noisy, a failed-run alert, the token field (not shown on the anonymous gateway), a production build, Windows High Contrast with real system colours.

## 4. Setup for the screen-reader pass
1. Start the gateway and UI as in `00_README.md` section 7. For the run journeys, a debug gateway is acceptable but say so.
2. Start the screen reader first, then the browser, then open `http://localhost:3000/?gateway=http://localhost:8085`.
3. Use a clean browser profile with extensions off, except the screen reader's own.
4. Test with the reviewer's normal settings. Record any non-default setting.
5. Allow extra time: an unfamiliar interface read by a screen reader takes much longer than a visual pass.
6. If something cannot be done, say "could not do X" and what stopped you. Do not look at the source code to work around it.

## 5. Screen-reader journeys
For each journey, record what was announced (verbatim where possible), what you could and could not do, whether the order of information was sensible, and anything that was missing, wrong, repeated or noisy. A pass for a step means you could complete it and understand what happened without sight.

| # | Journey | Steps | Look and listen for |
|---|---|---|---|
| J1 | Arrive and orient | Open the page. Read the page title, the first things announced, find the main content, skip to it | A skip link; one page heading; landmarks (banner, primary navigation, main, footer); the page says where you are |
| J2 | Navigate between pages | Use the navigation to go to Traffic and latency, Phantom Lab, Simulation, Cluster | On navigation, focus moves to the new page heading and the heading is announced; the current page is announced as current; the same page is not announced twice |
| J3 | Read a live data page | On Traffic and latency, read the values and the chart. Switch to the Table view | Every value is announced with its label **and** provenance word (Real, Simulated, Derived, Unavailable); unavailable values give a reason; the chart has a table with the same values; no stream of constant announcements |
| J4 | Connection status | With the gateway running, find the connection status. Stop the gateway and listen | Status is announced politely and only on change; the word is the same everywhere; reconnecting does not announce every second; the popover (if present) opens, announces its title "Gateway connection", keeps focus inside, and closes with Escape returning focus |
| J5 | Start a test | Go to Phantom Lab, Start a run. Fill the form with a short test; cause a validation error on purpose; fix it; start | Every field has a name; hints and errors are tied to their fields; an error summary is announced and focus goes to it; presets announce which is selected; the disabled start button says why |
| J6 | Follow a run and stop it | Watch progress. Cancel. Read the dialog. Confirm | Progress is announced at sensible points (not every tick); the dialog announces as a dialog, focus lands on the safe option ("Keep running"), Tab stays inside, Escape closes and returns focus; after the run is cancelled the page announces the state |
| J7 | Read a cancelled run | After cancelling, read the page from top | **No verdict, mechanism result or approval label is announced** (must-pass 2); the page says what happened and what is known |
| J8 | Read the completed Scenario B result | Run a quick check to completion (or open the retained Scenario B evidence) and read it from top | The answer or summary comes before the long notice; the binding notice and limitations are readable without opening anything; the verdict and its qualifiers are understandable aurally; tables have headers and are navigable by cell |
| J9 | Evidence pages | Open Evidence, the M6 result, the Scenario B retained result | Negative and positive results are announced with equal weight; codes are explained or reachable; links make sense out of context |
| J10 | Simulation page | Open Simulation | Announced as a simulation; illustrations have text alternatives that say "Illustration. Not live data." or equivalent; nothing sounds like live data |
| J11 | Runs list and a failed or interrupted run (if reachable) | Open the runs list; open each row type | Run ids are announced usefully; a failed run is announced (alert or status) with what happened and one next action. If not reachable, say so |
| J12 | Disconnected page | Stop the gateway while on a Lab page | The disconnected message is announced; a retry action exists and is reachable; the page does not claim the server is down for certain |
| J13 | Theme and menu | Change the theme; on a narrow window open the menu | Controls announce name and state (pressed or expanded); the menu opens, closes with Escape and returns focus |

**Questions to answer at the end of the pass (screen-reader reviewer):**
1. Could you complete J1 to J10 without sight? Which could you not?
2. Was anything announced that was wrong, misleading or untrue? (A value without its provenance, a verdict on a cancelled run, a status that sounded like server health.)
3. Was anything important never announced?
4. What was noisy or repeated?
5. What would you have to ask a sighted colleague for?

## 6. Keyboard-only script
No mouse, trackpad or touch. Use a visible browser window (not a screen reader).

| # | Check | Pass if |
|---|---|---|
| K1 | Tab from the top of each page type (Overview, Traffic, Lab home, Start a run, Runs, a run page, Evidence, Simulation) | The first stop is a "skip to content" link that is visible when focused; Enter moves focus to the main content; the order matches the visual order |
| K2 | Reach and activate every control: navigation, theme choice, menu button on narrow windows, tabs/sub-navigation, buttons, presets, form fields, disclosure details, chart/table toggle, connection control and popover | Each control is reachable and operable with Enter or Space; no control needs a mouse |
| K3 | Focus visibility | Focus is always visible, never hidden under the sticky header or the sticky Start bar, never lost off-screen |
| K4 | No keyboard trap | You can always leave a control, panel or dialog using Tab/Shift+Tab or Escape |
| K5 | Cancel dialog | Opens from the keyboard; initial focus is the safe option; Tab stays inside; Escape closes and focus returns to the Cancel button; confirming leaves focus on a sensible element, not the page top or nothing |
| K6 | Connection popover | Opens with Enter or Space; focus enters; Escape closes and returns focus to the control; Tab does not escape to the page behind unless the popover is designed non-modal (then record what you observed) |
| K7 | Navigation focus management | After using a link to change page, focus lands on the new page heading |
| K8 | Form errors | Submitting an invalid run form moves focus to an error summary; its links move focus to the field |
| K9 | Shortcut conflicts | No single-key shortcut fires while typing in a field |
| K10 | Group controls | Record how Arrow keys behave in the theme group and the Lab sub-navigation (the earlier agent run found they do nothing, only Tab works; this is a note, not necessarily a defect) |
| K11 | Menu (narrow window) | Opens, Escape closes and returns focus, items reachable, closes after navigation |
| K12 | Run stream and live updates | While a run is going, keyboard focus is not stolen by updates |

## 7. Text size and zoom (use the real browser features, not emulation)
The earlier run used a script that set the root font to 200 percent; people use the browser's own settings, which behave differently. Do both of these and record which.

| # | Check | Pass if |
|---|---|---|
| Z1 | Browser page zoom to 200 percent, then 400 percent (width 1280 down to 320 effective) | No horizontal scroll of the whole page; no overlapping or clipped controls; primary actions reachable |
| Z2 | Browser default **text size** set to the largest (browser setting, not zoom) or OS text scaling to 200 percent, at a narrow window or phone width | Same as Z1; the sticky Start bar does not cover the primary action or the fields beneath it; "Ready to start." and similar status text is not clipped |
| Z3 | Tables and long identifiers (runs list, run id) | Readable; scroll inside the table container if needed; the full value reachable |
| Z4 | Text-only spacing (if you have a bookmarklet or extension: line height 1.5, paragraph spacing 2 times, letter spacing 0.12, word spacing 0.16) | Nothing disappears or overlaps |

## 8. Forced colours and contrast preferences
| # | Check | Pass if |
|---|---|---|
| C1 | Windows High Contrast (contrast themes) or the OS equivalent; and Firefox's "Override colours" if used. Open S01, S04, S06, S07 | Everything is legible; buttons and fields still have edges; focus is visible; selected and focused states can be told apart (earlier note A11Y-11); icons and chart marks that carry meaning remain distinguishable |
| C2 | "Increase contrast" if the OS offers it | Legible, nothing disappears |
| C3 | Colour is never the only carrier of meaning | Turn colour off (greyscale display or filter): provenance, verdict, and status are still distinguishable by text or shape |

## 9. Reduced motion and reduced transparency
Turn on the OS settings: reduce motion (Windows: Animation effects off; macOS/others: the equivalents) and reduce transparency (Windows: Transparency effects off).
| # | Check | Pass if |
|---|---|---|
| M1 | Navigate, open the menu, start and cancel a run, open the popover | No sliding, scaling or fading beyond a simple opacity change; nothing keeps moving; nothing important disappears |
| M2 | Transparency off | No text sits on a blurred or translucent background that becomes hard to read; surfaces are solid |

## 10. Touch targets and gestures (real phone)
Done on the same phone as package 04. Record phone, OS, browser and version in the log.
| # | Check | Pass if |
|---|---|---|
| T1 | Primary controls: Start run, Cancel, Retry, Menu, theme, presets | Each can be hit reliably with a thumb; at least 44 by 44 px for primary controls (24 by 24 px is the WCAG 2.2 AA minimum for others) |
| T2 | Sticky bars | Do not cover content you need to see or tap; they do not overlap form fields |
| T3 | No action needs hover, long-press, multi-finger or path-based gestures | True |
| T4 | Phone zoom and landscape | Pinch-zoom is not disabled; landscape works |
| T5 | If the phone has a screen reader (VoiceOver or TalkBack), you may repeat J1, J5, J6, J8 with it. This is **optional extra evidence**; name it separately in the template | Same as section 5 |

Measure by eye or with the browser's developer overlay; where you measure, record the number.

## 11. What the automated axe runs cover and do not
**Tool:** `ui/scripts/check-axe.mjs` (run `npm run check:axe` from `ui/`; options `--ui`, `--gateway`, `--routes`, `--viewports`, `--themes`, `--evidence`, `--gate`). It injects the dev-only axe-core 4.14.0 into the tool's own throwaway Chrome, per route, theme and viewport, and records violations and `incomplete` results separately. Its own header says what it means.

| Covers | Does not cover |
|---|---|
| Rules axe can decide mechanically on the page as rendered: missing names, roles, landmarks, heading order, duplicate ids, many ARIA mistakes, some contrast | Whether announcements make sense; reading order as heard; whether alt text is meaningful; keyboard flow and focus management as a person uses them; dialogs and popovers behaviour end to end; live-region noise; real zoom and text scaling; forced colours; reduced motion; touch; non-Chrome browsers; chart meaning; cognitive load; most of WCAG 2.2 |
| A run on a page that has rendered | A run on a page that has not rendered looks "clean" (the tool has a readiness guard because an earlier run produced false clean cells) |

An `incomplete` result is **never** a pass: it means axe could not decide (the earlier self-test had 12 incompletes, all colour contrast). A clean axe result means "no violation found by these rules in these states". The recorded self-test covered only overview, run form and simulation, two viewports, both themes, with the gateway not running, and was not a baseline. A full baseline with a connected gateway and run pages has not been recorded as a human-checked result. `[FILL: whoever runs axe records the output path and date; running it is a tool step and does not change the TESTED status]`

## 12. Acceptance criteria
**Complete** when: a screen reader is named; J1 to J10 are each marked done, partly done or could not do, with notes; every keyboard check K1 to K12 is marked; sections 7 to 10 are marked pass, fail or not done with reasons; the template in section 13 is filled; names and dates match the log.

**The accessibility review passes** only if, as recorded by the named humans on this build:
1. J1 to J8 and J10 can be completed with the named screen reader without sighted help.
2. No screen-reader finding of Blocking or High severity is open (Blocking: a false or misleading announcement such as a verdict on a cancelled run; High: a task cannot be completed or a status is misread).
3. K1 to K8 all pass.
4. Z1, Z2, C1, M1, M2 and T1 to T3 pass, or each failure is a recorded Medium or Low finding the owner accepts by ID.
5. Every `incomplete` axe item for the reviewed set is resolved by a manual check or listed as a finding.

Passing means "the named reviewer found no blocking barrier with the named tools on this build". It is not WCAG conformance and says nothing about other screen readers.

## 13. Recording template
Severity uses the rubric (`critique_rubric.md` section 1). IDs `AH-001...`. Do not reuse the earlier `A11Y-nn` IDs.

| ID | Journey or check | Screen reader, browser, OS (exact) | What was announced or observed (verbatim) | What was expected | Barrier? (none / slows / blocks) | Severity | Matching earlier hypothesis | Evidence (note, audio, video path) | Owner |
|---|---|---|---|---|---|---|---|---|---|
| AH-001 | `[FILL]` | `[FILL]` | | | | | | | |
| AH-002 | `[FILL]` | `[FILL]` | | | | | | | |

**Per-step completion record:**
| Journey / check | Done / partly / could not | Notes |
|---|---|---|
| J1 | | |
| J2 | | |
| ... | | |

**Sign-off:** reviewer code(s) `[FILL]`; screen reader and version `[FILL]`; date `[FILL]`; result `[FILL: PASS / FAIL]`; owner acceptance of Medium/Low by ID `[FILL]`; date `[FILL]`. Until recorded the status is **TESTED (agent-driven, headless Chrome only)**.
