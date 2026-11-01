# PhantomCache UX critique rubric

**Status:** Active (first version; to be reviewed by a human before it is called validated). **Audience:** implementers, reviewers, the lead. **Last reviewed:** 2026-10-30 16:12 (+05:30, machine clock). **Authority:** owner approval of 2026-10-30 (project-owned UX critique rubric, section 9 of the brief). **Use:** during implementation (self-check before handing a screen to review) and again at final review. An AI applying this rubric produces an "AI review", not human review.

## 1. How to apply it
1. Review a **screen in a named state** (for example `#/lab/runs/<id>` in state `cancelled`), at the viewports 320, 390, 820, 1280 in light and dark, plus 200% root text at 390. Fixtures come from real backend output or are labelled SIMULATED.
2. Open the actual screenshots (a file that was not opened was not reviewed). Record the evidence path.
3. Score each criterion PASS / FINDING. Every FINDING gets an ID, severity, evidence path and an owner. No unexplained numeric scores, and no "looks modern" or "feels premium" without the observable condition below.
4. **Must-pass items** (section 3) are checked first; a failure there is a defect before any polish.
5. Footer on every report: "AI review. Not human review. States not covered: ...".

Severity: **Blocking** (false or misleading claim, data loss, unusable), **High** (task fails or likely misread), **Medium** (task slowed, accessibility gap with a workaround), **Low** (polish, inconsistency without task impact).

## 2. The twelve criteria
| # | Criterion | Observable checks (each is yes/no) |
|---|---|---|
| 1 | Visual hierarchy | One clear h1 per view; the primary answer or action is the first thing in the reading order and visually dominant; section titles, labels, metadata and supporting text are four distinguishable levels; no two competing primary buttons; large numerals only for real, labelled values |
| 2 | Information architecture | The page answers its question before showing the technical record; navigation labels match page titles; the user can tell where they are (active item, heading, breadcrumb or title); the same object has the same name on every screen; related items are grouped and unrelated items are not |
| 3 | Comprehension and cognitive load | Each technical term is explained at first use or one click away (M6, BCa, mechanism check, coalescing); no paragraph longer than about 5 lines before a heading or a visual; important results are not below a long notice; at most one tier-1 status (connection or run state) visible per context |
| 4 | Visual consistency | Every colour, space, radius and shadow comes from tokens (no raw hex or px in components); the same component looks and behaves the same everywhere (buttons, segmented controls, status marks, tables); light and dark are both designed (no inverted leftovers); provenance marks look identical on every screen |
| 5 | Interaction clarity | Every control has a visible label or accessible name and a visible focus ring; the primary action is disabled only with a stated reason; destructive actions need a confirmation whose safe option has default focus; feedback appears on press (no dead clicks); target size at least 24 px desktop and 44 px for primary touch controls |
| 6 | Motion quality | Each animation names the real event or user action behind it; only `transform` and `opacity` are animated; durations come from motion tokens (nothing above 300 ms); no looping decoration; no motion on stale or reconnecting states; measured numbers and charts are never tweened; `prefers-reduced-motion` leaves a complete experience (`document.getAnimations()` empty or opacity-only); no more than 2 concurrent animations on phone |
| 7 | Responsive behaviour | No horizontal page scroll from 320 to 1920; no clipped or overlapping controls (sticky bars included) at 390 with 200% root text; tables become readable (scroll container or stacked) without losing columns silently; touch targets hold at narrow widths; long identifiers wrap or truncate with a full-value alternative |
| 8 | Accessibility | Skip link first; one h1 and no skipped heading levels; landmarks present; focus moves to the page heading on navigation and returns correctly after dialogs; live regions are polite unless `role=alert` already exists and never announce 1 Hz noise; colour is never the only carrier of meaning; text contrast 4.5:1 (3:1 for large text and UI components) at the worst-case background; charts have a table twin with identical values; forced-colours mode keeps selection and focus distinguishable; axe: 0 violations, every `incomplete` item resolved by a manual or worst-case check |
| 9 | Performance | Baseline measured before motion or glass is added; software-path scroll test shows at most 2 long frames in 3 s (DD 16.8); no animated blur, shadow, width or height; glass surface count within budget (4 desktop, 2 phone); no layout shift from late-loading content; bundle size recorded before and after |
| 10 | Technical truthfulness and provenance | Every value carries one of REAL, SIMULATED, DERIVED, REPLAYED, PREDICTED, UNAVAILABLE (retained Scenario B summaries = REAL with qualifier plus a separate RETAINED indicator; REPLAYED only for data re-rendered from stored artifacts; PREDICTED only with real predictive functionality: DEC-007); REAL carries its qualifier and is never presented as verified production behaviour; UNAVAILABLE shows a reason, never zero; no number is computed in the chart beyond layout; intervals appear only when supplied; no overall score or invented verdict; negative results carry the same weight as positive ones; the binding Scenario B interpretation text is present and not collapsed; illustrations carry "Illustration. Not live data."; nothing implies live predictive intelligence, canary or rollback |
| 11 | Product identity | The screen reads as an instrument and evidence product (paper or blue-black ground, neutrals for hierarchy, hue only for meaning); the ghost-outline motif is used only where it carries meaning; no generic dashboard tropes (card-per-datum grids, gradients as decoration, neon, glass on data); a stranger can say in one sentence what this product is for after one screen |
| 12 | Error, empty, loading and disconnected states | Each of loading, empty, error, disconnected, stale, unauthorised-if-detectable, cancelled, failed and interrupted is designed and reachable via a fixture; each says what happened, what is known, and one next action; the same condition uses the same words and component on every screen; connection status is stated as a client observation and never implies server health; the token is never shown, stored or put in a URL |

## 3. Must-pass items (check first)
1. Provenance mark on every value; UNAVAILABLE gives a reason, not zero.
2. **No verdict, mechanism result or `VALID`-style approval on a cancelled, failed or interrupted run** (U-01). Fixtures for these pages come from real backend output (T-01), not hand-set objects.
3. One connection component and one vocabulary on every screen; "Unauthorised" and "Recovering" appear only where the code can distinguish them.
4. Limitations and the binding interpretation notice are present and not collapsed on evidence pages.
5. Verdict parity: negative and positive results have equal visual weight.
6. Charts: table twin with identical values; no tweening or smoothing of measured data.
7. Page survives 200% root text at 390 px with no clipped primary action and no horizontal page scroll.
8. Reduced motion and reduced transparency leave a complete, legible experience.
9. Token memory-only; gateway address is the only value persisted (localStorage, try/catch).

## 4. Finding format
`ID | screen + state | viewport/theme | criterion # | what is wrong (observable) | evidence path (opened) | severity | owner`

## 5. Final-review use
Re-run the rubric on the representative set: overview, one live data page, Lab home, run form, run in progress, completed Scenario B result, cancelled run, failed run, runs list, evidence index, Scenario B evidence, Simulation, Cluster, Intelligence, connection-lost state. A human review (visual, assistive technology, usability) is still required before anything is called independently verified.

## 6. Validation baseline (owner decision DEC-008, 2026-10-30)
Minimum scope for the final review: Chrome current stable, Firefox current stable, at least one real mobile phone, at least one reviewer using a screen reader (keyboard-only navigation also evaluated), and human visual and usability review with predefined tasks and success criteria. Reviewer identities and the review date are recorded before the review; none are invented. No WebKit or Safari automation and no cloud visual-testing services. Until the human review is complete, agent-driven results are reported TESTED, not INDEPENDENTLY VERIFIED.
