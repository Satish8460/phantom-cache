# Human review packages: index and rules

**Status:** Draft for owner review. **Audience:** the product owner (who names reviewers and sets the date), the people who will run the reviews, and anyone reading a review result. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Prepared by:** agent HR (documents only). No review has been run. No reviewer, participant, date or result appears in this folder because none has been recorded yet.

## 1. Why these packages exist
Owner decision DEC-008 (`docs/design/OWNER_DECISION_REGISTER.md`, approved 2026-10-30) sets the minimum validation scope for the redesigned UI:

- Chrome current stable, and Firefox current stable.
- At least one real mobile phone.
- At least one reviewer who uses a screen reader, with keyboard-only navigation also evaluated.
- Human visual and usability review, with tasks and success criteria written down before the review.
- No WebKit or Safari automation and no cloud visual-testing service.
- Reviewer identities and the review date are recorded **before** the final review.
- Until then, results are **TESTED**, not **INDEPENDENTLY VERIFIED**. Firefox and phone checks are manual; no new automation tooling is approved.

These packages turn that decision into things a person can pick up and use. They are scripts and forms, not results.

## 2. The packages
| File | What it is for | Who runs it |
|---|---|---|
| `01_visual_design_review_package.md` | Judge the screens and states against the 12 criteria and 9 must-pass items of `docs/design/critique_rubric.md` | A human visual reviewer |
| `02_usability_study_protocol.md` | A moderated think-aloud study with 5 to 8 people, tasks, pre-registered thresholds, analysis plan | A facilitator and participants |
| `03_accessibility_review_package.md` | Screen-reader pass, keyboard-only script, text size, forced colours, reduced motion, touch targets | A screen-reader user chosen by the owner, plus a second person for keyboard-only |
| `04_browser_device_matrix.md` | What to record in Chrome, Firefox and one real phone | Any competent tester |
| `05_external_security_review_package.md` | Scope, assets, boundaries, how to build and run, the known findings, deliverable format | An outside security reviewer |
| `06_handbook_review_checklist.md` | The owner's own review of the learning handbook (Appendix A, Appendix H, Appendix E, chapters) | The owner |
| `07_multi_host_review_note.md` | One page pointing to the multi-host plan (written elsewhere) | The owner |

## 3. What each package establishes, and what it does not
| Package | If completed and recorded by named humans, it establishes | It does NOT establish |
|---|---|---|
| 01 Visual design | A human judged the listed screens in the listed states against the rubric, on the listed browsers and viewports | That unlisted states or viewports are fine; that the design is "good" beyond the rubric; any accessibility or usability claim |
| 02 Usability | How the people in the sample performed on the written tasks, with their timings, errors and comments | That the product is usable for everyone; statistical significance (the sample is 5 to 8); that a fix works (a fix needs a re-test) |
| 03 Accessibility | What the named screen reader, browser and OS combination announced and allowed, plus the keyboard, zoom, colour and motion checks | WCAG conformance; behaviour with other screen readers; that axe or any automated tool is sufficient |
| 04 Browser matrix | What happened in Chrome stable, Firefox stable and one phone, on the recorded versions | Safari or WebKit behaviour (out of scope by DEC-008); any other phone or browser version |
| 05 Security | What an outside reviewer found in the agreed scope and time | That the system is secure; coverage outside the scope; that the 99 known findings are fixed (they are not) |
| 06 Handbook | That the owner understood the material and recorded where it is wrong or unclear | That the handbook is correct (the code and tests are the authority) |
| 07 Multi-host | A pointer only | Anything; it contains no plan |

## 4. The rule about AI results (read this first)
**Agent-driven and AI-reviewed results stay TESTED until a human review of the same thing is recorded in the log in section 6.** This covers the AI usability walkthrough (U-01..U-14), the automated accessibility runs, the axe runs, the rubric self-checks, the AI "second look" code reviews, and the screenshot galleries. Wording to use:

- Before a human review is recorded: "TESTED (agent-driven). Not independently verified."
- After a human review is recorded: say which review (package number, reviewer role, date from the log) and what it covered. Only the covered scope may be called independently verified. A human review of Chrome does not verify Firefox.
- A human reading an AI result and agreeing with it is not a review unless they performed the package steps themselves.
- Hypotheses from AI work (for example U-01..U-14, A11Y-01..A11Y-11) are inputs to a human review. They are never counted as human findings.

The status vocabulary (IMPLEMENTED, TESTED, INDEPENDENTLY VERIFIED, and so on) is the one in `docs/design/remediation/07_frontend_redesign_implementation_record.md` and Appendix G of the handbook.

## 5. How to run the reviews
1. **Before anything else, fill the reviewer-and-date log (section 6).** DEC-008 requires names and the date to be recorded before the final review. A reviewer who is not in the log before the session does not count.
2. **Freeze the build under review.** Record the git commit (or "working tree, uncommitted"), the UI build kind (Vite dev server or production build), the gateway build (debug or release; a debug build labels results EXPLORATORY by design), and who built it. Write it in each package's header block. If the code changes after a review, the review applies to the old build only.
3. **Prepare the environment** from section 7 below. Use one machine setup for all UI packages so results are comparable.
4. **Run the packages in this suggested order:** 04 (cheap, finds blockers early), 03, 01, 02. The usability study should happen on a build that has no known blocking visual or accessibility defect, or the study measures the defects instead of the design. Security (05) and the handbook review (06) are independent of the UI and can run any time.
5. **Record as you go**, in the tables inside each package, on paper or a copy of the file. Do not edit this folder's originals with results; copy them into a dated results folder chosen by the owner: `[FILL: results folder path]`.
6. **Disposition findings.** Each finding gets an ID, severity, evidence and an owner (the format in the rubric, section 4). Nobody closes a finding without a re-test by a human.
7. **Report.** Each package ends with a sign-off block. The owner decides what is called independently verified.

## 6. Reviewer-and-date log (TEMPLATE: all fields blank on purpose)
Fill before the final review. Do not enter anyone who has not agreed. Personal data (names, contact details) may need to be held outside the repository: `[FILL: owner decides where this log lives and whether real names are stored here or only reviewer codes]`.

| Package | Reviewer code | Reviewer name or alias | Role / relevant experience | Independent of the builders? (yes/no, why) | Agreed on (date) | Session date(s) | Browser / OS / device / assistive technology used (exact versions) | Build reviewed (commit or tree, UI build, gateway build) | Sign-off (date) |
|---|---|---|---|---|---|---|---|---|---|
| 01 Visual | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 02 Usability: facilitator | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 02 Usability: note-taker | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 02 Usability: participants | see the participant log in file 02 | | | | | | | | |
| 03 Accessibility: screen-reader reviewer | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL: screen reader NAME and version, chosen and named by the reviewer]` | `[FILL]` | `[FILL]` |
| 03 Accessibility: keyboard-only evaluator | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 04 Matrix: Chrome / Firefox tester | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 04 Matrix: phone tester | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL: phone make, model, OS and browser versions]` | `[FILL]` | `[FILL]` |
| 05 Security: external reviewer or firm | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` | `[FILL]` |
| 06 Handbook: owner | `[FILL]` | `[FILL]` | owner | n/a | `[FILL]` | `[FILL]` | n/a | `[FILL: handbook build or commit]` | `[FILL]` |

**Final review date (DEC-008):** `[FILL]`. **Recorded in this log on:** `[FILL]`. **Recorded by:** `[FILL]`. The log entry must predate the session.

## 7. Shared environment (how to open the dev build and the gateway)
Details are repeated in the packages that need them. Everything below comes from `docs/user/*.md`, `docs/design/remediation/03_validation_accessibility.md` and `ui/package.json`; the commands were not re-run when this folder was written.

1. **Gateway, anonymous development mode (loopback only).** Build: `cargo build --release --locked -j 1` (on this Windows machine `-j 1` is required). Run with the Lab enabled and a Lab folder that holds only the copied retained evidence you want reviewers to see: `PHANTOM_LAB_ENABLED=true`, `PHANTOM_LAB_DIR=<folder>`, `PHANTOM_MODE=development`, default bind `127.0.0.1:8085`. Set `PHANTOM_CORS_ORIGINS` to the page origin if the UI is not on `http://localhost:3000`. In this mode the gateway only answers loopback host names, so reviewers must use `localhost`, `127.0.0.1` or `[::1]`. Use a release gateway if you want runs not to be labelled EXPLORATORY; a debug gateway makes every run EXPLORATORY by design (`docs/user/lab_api.md`).
2. **UI, development server.** `cd ui`, `npm ci`, `npm run dev`; opens on `http://localhost:3000` (`ui/vite.config.js`). Point it at another gateway with `?gateway=http://localhost:<port>` on the URL (this page load only) or the `VITE_PHANTOM_GATEWAY` build value. The UI default gateway is `http://localhost:8085`.
3. **UI, production build.** `cd ui`, `npm run build`; serve `ui/dist` with any static server you trust. The earlier automated runs used the unminified dev server only (`03_validation_accessibility.md`, "Could not test"); a production build has not been checked. Say which one you reviewed.
4. **Run states.** A running, completed or cancelled run comes from using the Lab (Start a run, Cancel). Failed and interrupted runs are harder; see `01_visual_design_review_package.md` section 4 for the recipes and their limits.
5. **Cleaning up.** Stop the gateway and the dev server when the session ends. The gateway cleans up its own run children on Windows (job object). Do not leave a loopback-only gateway reachable through a tunnel or proxy; that defeats its loopback guard (`docs/user/lab_api.md`).

## 8. Known limits of this folder
- Written from documents, not from running the product. Commands and fixtures are copied from project documentation and may have drifted; the first runner should note any that fail and report them, not work around them silently.
- The UI is under active change in the working tree. Some screens named here (for example the connection popover) exist in code but have not been human-reviewed at all.
- It cannot choose reviewers, a phone, a screen reader, a date, a participant pool, compensation, or whether ethics review is needed. Those are the owner's. Each is a visible placeholder.
- The multi-host plan is being written elsewhere; see file 07.
