# Package 02: usability study protocol

**Status:** Draft for owner review. **Audience:** the owner (decides and freezes thresholds), the facilitator and note-taker who run sessions, and anyone reading the result. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`.

**Everything marked PROPOSED (thresholds, time limits, task wording, answer keys) must be read, changed if needed, and frozen by the owner before the first participant is recruited. Write the freeze date here: `[FILL]`. Thresholds changed after seeing data are not pre-registered and must be reported as changed.**

**Build under test:** commit or tree `[FILL]`; UI `[FILL: dev server or production build]`; gateway `[FILL: release or debug]`. A debug gateway labels every run EXPLORATORY by design, which participants will see; decide whether to use a release gateway so the study is not about that label. `[FILL]`

## 1. Purpose and what a result can say
Purpose: find out whether people who match the PhantomCache audience can do the main jobs in the UI and read the results correctly, with particular attention to **misreadings that would mislead them about the evidence** (a cancelled run read as a result; simulated numbers read as measured; "VALID" read as "it worked").

A result can say: how many of the people in this sample succeeded on each task, what they did, how long it took, and what they misread. It cannot say how the whole user population would do (the sample is 5 to 8), and it cannot show that a fix works (that needs a re-test).

## 2. The AI walkthrough is a source of hypotheses, not results
`docs/design/remediation/04_validation_usability.md` is a walkthrough by an AI reviewer simulating a first-time user, on an earlier build. Its own first line says it is not human data. It produced 14 issues, U-01..U-14. In this study they are **hypotheses to look for**:

| ID | Hypothesis (from the AI walkthrough, earlier build) | Task that tests it |
|---|---|---|
| U-01 | A cancelled run still shows a verdict or "mechanism failed" that could be misread | P3 |
| U-02 | A person on a different gateway address cannot recover from the UI | P6 |
| U-03 | Progress shows "0 of 2 units" and an empty bar and looks stalled | P3 |
| U-04 | Run form opens with long defaults, no selected preset state, no duration estimate | P2 |
| U-05 | The Scenario B answer is buried under a long notice; no head-to-head line | P5 |
| U-06 | The M6 page needs decoding of codes and statistics terms | P4 |
| U-07 | "VALID" invites reading as success | P5, P4 |
| U-08 | "Reports mode: live" persists when disconnected; two unrelated "mode" notions | P6 |
| U-09 | "Not available" wording differs between sidebar and page | P7 |
| U-10 | Phone: sticky Start bar overlaps the form; status hidden in the Menu | P2 on the phone |
| U-11..U-14 | Low items: several status words, duplicate "Start a run", no orientation on Overview, merged "not reachable or not authorised" | observed in any task |

Rules: the facilitator does not mention any of these to participants; a hypothesis is recorded as **observed**, **not observed** or **contradicted** by what participants did, never "fixed"; none is counted as a human finding until a participant is seen to have the problem.

## 3. Participants
**Number:** 5 to 8 completed sessions plus one pilot (the pilot does not count). Target `[FILL: 6 unless the owner decides otherwise]`. Plan for no-shows by recruiting one or two extra.

**Who (PROPOSED, from the AI recommendation and DEC-008):**
| Group | Count | Who |
|---|---|---|
| Core audience | 3 to 4 | Backend, site-reliability or performance engineers who use or run caches, proxies or gateways |
| Adjacent | 2 to 3 | Frontend or data engineers with no exposure to the project |
| Assistive technology | at least 1 (may overlap with the groups above) | A screen-reader or keyboard-only user. This does **not** replace package 03: the named screen-reader reviewer there is a separate required role |
| Phone | at least 1 (may overlap) | Does the phone-variant of P2 on a real phone (see 04 for connectivity) |

**Inclusion:** works with software professionally or studies it at an advanced level; can read English at professional level (`[FILL: language(s) the study runs in]`); has not seen the PhantomCache UI.
**Exclusion:** anyone who built, designed, tested or reviewed PhantomCache; anyone who has read the handbook or the project documents; anyone who has taken part in an earlier session of this study; close family or direct reports of the owner (to avoid pressure to please). `[FILL: owner adds or changes criteria]`

**Recruitment channel:** `[FILL: owner decides; do not invent a panel]`. **Compensation:** `[FILL]`. **Screener:** a short form asking role, years of experience, which of cache/proxy/gateway tools they have used, whether they use assistive technology or a phone for work, and whether they have seen the product. Do not collect more personal data than needed.

**Participant log (fill as people are scheduled; use codes, not names, in all results):**
| Code | Group | Assistive tech / phone? | Session date | Facilitator | Note-taker | Consent on file (Y/N) | Recording (Y/N) | Counted (Y/N, reason if N) |
|---|---|---|---|---|---|---|---|---|
| P01 | `[FILL]` | | `[FILL]` | | | | | |
| P02 | `[FILL]` | | `[FILL]` | | | | | |
| P03 | `[FILL]` | | `[FILL]` | | | | | |
| P04 | `[FILL]` | | `[FILL]` | | | | | |
| P05 | `[FILL]` | | `[FILL]` | | | | | |
| P06 | `[FILL]` | | `[FILL]` | | | | | |
| P07 | `[FILL]` | | `[FILL]` | | | | | |
| P08 | `[FILL]` | | `[FILL]` | | | | | |

## 4. Setup and materials
- **Station:** one laptop with Chrome current stable (record the version) and a second browser only if a participant needs it. A fresh browser profile with no bookmarks, saved tabs or accounts, so nothing personal shows on a shared screen. `[FILL: machine]`
- **Gateway:** reset to a known state before every session: anonymous development mode, Lab enabled, the same prepared Lab folder as package 01, no old runs unless the task needs them. Stop it cleanly between sessions. For task P6 the facilitator needs a second gateway instance on a **non-default port** (the participant's start URL uses the default address, so the page starts disconnected). `[FILL: ports]`
- **Do not tell the participant** the gateway address in advance; the P6 card contains it.
- **Phone variant:** only if a phone participant is recruited; connection method is an open item in `04_browser_device_matrix.md`.
- **Materials:** consent form, task cards (one per task, large print), answer key (facilitator only), SUS and single-ease sheets, observation sheet (section 9), stopwatch, recording tool `[FILL]`.
- Screen and audio recording only if the participant agrees (section 8). The participant's face is not recorded.

## 5. Tasks
**Wording rule:** task cards must not use product vocabulary a newcomer would not have ("Scenario B", "quick preset", "adaptive cap", "EXPLORATORY", "loopback", "M6", "mechanism check", "falsified"). The AI walkthrough's own bias list says its task wording leaked these terms. If the participant uses a term first, the facilitator may repeat it back, never introduce it.

Order: P1 first. P2 before P3. P4 and P5 alternate by participant (odd participants P4 then P5; even participants P5 then P4). P7 and P8 alternate the same way. P6 last, because the facilitator stops the gateway. Time is per task; "hard limit" is when the facilitator stops the task and records a fail with the reason.

| Task | Card text (PROPOSED; the participant reads this aloud) | Target time | Hard limit | Success criteria (all must hold) | Comprehension probe, asked after (facilitator asks, writes answer verbatim) |
|---|---|---|---|---|---|
| **P1 Orientation** (AI task T1) | "You have been told this tool helps people test whether a cache in front of a server can be trusted. Starting from this page, find where the tool explains what its experiments are, and tell me what you think the tool is for." | 3 min | 5 min | Finds the experiments area unaided within the limit; states in their own words that it runs experiments about protecting a server behind a cache | "In your own words: what do the labels like Real, Simulated and Unavailable mean?" Key: Real = measured by real processes in the stated setup; Simulated = produced by a model; Unavailable = no value, not zero. Correct if all three are right in substance |
| **P2 Start a short test** (T2) | "Start a short test against the gateway on this computer, and tell me when it has finished and what it says." | 3 min | 6 min | Reaches the run form; starts a short test (a small preset or small settings); notices when it ends; reads the outcome aloud correctly (for example "inconclusive: not enough repetitions") | "Is this a trustworthy result? Why or why not?" Key: not on its own; the page says so |
| **P3 Progress and stop** (T3) | "Start a longer test. While it is running, tell me how far along you think it is and how long it will take. Then stop it, and tell me what you can conclude from the stopped test." | 4 min | 8 min | Starts a longer test; states how far along it is **and** how they know; stops it with the confirmation; correct on the probe | "What does the page tell you about the result of the test you stopped?" **Key: nothing can be concluded; there is no result.** Critical error C1 if they say the stopped test showed a hypothesis failed or the control mechanism broke, or that it has a valid result |
| **P4 Stored result of the simulated evaluation** (T4) | "Find the stored result of the earlier simulated evaluation and tell me, in one sentence, what it found." | 3 min | 5 min | Finds it within the limit; the sentence says it is simulated (not a measurement) and states the finding without reversing its direction | "Was anything on that page measured on real servers?" Key: no, it is a simulation. Critical error C2 if they say yes |
| **P5 Stored result of the real-process experiment** (T5) | "Find the stored result of the earlier experiment run on real processes on this computer. Did the adaptive approach beat the fixed one?" | 3 min | 5 min | Finds it; answers that the evidence does not show the adaptive approach winning, with at least one reason the page gives; does not say adaptive "won" or that adaptive "is proven not to work" | "What is the main caution the page gives before the result?" Key: the notice that the result is limited (same machine, one setting, adaptive behaved like a fixed cap). Critical error C3 if they read the "VALID" label as "the adaptive approach worked" |
| **P7 Not-yet-available areas** (T7) | "Does this tool currently show anything about several servers working together, or about automatic tuning? How do you know?" | 2 min | 3 min | Finds both areas; says that neither currently shows values and says why they think so | "Is that because it is not built, or because this gateway has it switched off?" Record the answer; the key is whatever the page actually says in the build under test. `[FILL: answer key after the owner checks the page wording]` |
| **P8 Theme and the simulation page** (T8) | "Switch the page to dark colours. Then find the page that explains the simulation and tell me whether any of the numbers on it were measured." | 2 min | 4 min | Switches theme; finds the page; says none are measured / it is an illustration. Critical error C2 if they say numbers are measured | none |
| **P6 Connection** (T6 + U-02) | "The facilitator has told you a gateway is running at `http://localhost:<port>` `[FILL: port]`. Tell me whether this page is connected to it. If it is not, get it connected. Later the gateway will stop; tell me when you notice, and what you would do." | 4 min | 8 min | Correctly states the connection state at start; connects without the facilitator typing; notices the stop within 30 seconds of it happening; states a sensible next step | "Does the page tell you whether the gateway itself is healthy?" Key: it tells you what this page can observe, not the gateway's health. Critical error C4 if they say the page proves the gateway is down or healthy |

Optional **P2-phone**: P2 repeated on the phone participant's own phone. Same card; the facilitator notes sticky-bar overlap and menu use.

**Help policy:** the facilitator answers no question about the product. Allowed neutral prompts: "What are you thinking?", "What would you do next?", "Take as long as you need; tell me when you are done." After the hard limit the facilitator offers one scripted hint (written on the card back, `[FILL]`) once, records "hinted", and counts the task as not completed unaided. A participant may give up at any time; record the time and reason.

## 6. Measures and pre-registered thresholds (PROPOSED; owner freezes)
**Per task and participant record:** completed unaided (yes/no), time from first reading the card to the participant saying "done" or the hard limit, errors, hesitations, help requests, hints used, answers to probes (verbatim), and a Single Ease Question score.

Definitions:
- **Completed unaided:** meets all the success criteria with no hint and before the hard limit.
- **Error:** a wrong click or navigation to a place that cannot do the task, or a statement that contradicts the page. Count each distinct one; repeating the same wrong step counts once per attempt, up to 3.
- **Hesitation:** a pause of 10 seconds or more with no action and no speech, or an explicit "I'm not sure".
- **Critical error (C1..C4):** a misreading listed in the task table. Recorded separately; any one is a failure of the protocol's safety bar regardless of averages.
- **Single Ease Question (SEQ):** after each task, "Overall, how easy or difficult was this task?" 1 (very difficult) to 7 (very easy).

| Measure | Pre-registered threshold (PROPOSED) | Rationale and caveat |
|---|---|---|
| Task completion, unaided | At least 80 percent of counted participants on each of P1 to P8 (with n=5 that means 4; n=6, 5; n=7, 6; n=8, 7) | A small-sample screen. With n=6 and 5 successes the 95 percent Wilson interval is roughly 44 to 97 percent (hand-calculated, recompute); the observed rate is an indicator, not an estimate of the population rate |
| Critical errors | **Zero** participants with any of C1..C4 | These are the misreadings the redesign exists to prevent. One occurrence is a finding at least High, and Blocking if it is C1 |
| Comprehension probes (P1, P2, P3, P4, P5, P6) | At least 80 percent of probe answers correct per probe | Marked blind against the key by two scorers where possible |
| Time on task | Median at or below the target time per task; no participant reaches the hard limit on more than one task | Think-aloud slows people, so times are indicative only and must not be compared with silent timing |
| Errors and help | Median help requests 0 per task; median errors 3 or fewer per task | Arbitrary starting values chosen for a first study; owner may tighten |
| Single Ease Question | Mean at least 5 out of 7 per task | Self-report, one item; low n |
| SUS | Median at least 68 | See limits below |
| Trust calibration (two custom items after the session, 1 to 5 agree scale): "I could tell which numbers on these pages were real measurements." and "I could tell when a result should not be relied on." | Median at least 4 on each; plus no participant scoring 1 or 2 | Custom, unvalidated items. They check whether people can tell evidence status apart; they do not check whether the evidence is good |

**SUS (System Usability Scale).** Ten statements, each rated 1 (strongly disagree) to 5 (strongly agree), asked once after all tasks. Scoring: for odd items subtract 1 from the rating, for even items subtract the rating from 5, add everything, multiply by 2.5, giving 0 to 100. Use "tool" in place of "system" only if the owner approves the wording change. The ten standard statements must be copied from the primary source (Brooke, 1996) onto the sheet; **the wording has not been checked against the primary source here**. `[FILL: owner supplies or approves the exact sheet]`
Limits: SUS measures overall perceived usability after use; it does not say what is wrong; it is not a percentage; with 5 to 8 people the interval around the median is very wide; the often-quoted average of about 68 comes from large pooled studies of other products (literature, not verified from the primary source here), so it is a rough reference, not a PhantomCache target; scores are affected by recent task failures; and SUS says nothing about whether people read evidence correctly, which is why the tasks and critical errors carry the weight.

## 7. What a pass means
**The study passes** (for the build tested, with this sample) only if **all** of these hold:
1. Zero critical errors (C1..C4) across all counted participants.
2. Every task meets the completion threshold.
3. Every probe meets its correctness threshold.
4. The time, error, SEQ, SUS and trust-calibration thresholds are met, or each miss is listed with an owner decision to accept it or fix and re-test.
5. Consent, recording and anonymisation steps were followed and logged.

**Pass with conditions** is not defined; either all hold or the owner records which misses are accepted, by name and date. **Fail** means findings are fixed and the failed tasks re-tested with new participants where possible (people who saw the old design are biased). A pass does not mean the product is usable for everyone, and does not by itself make anything INDEPENDENTLY VERIFIED beyond usability for the sample; see `00_README.md` section 4.

## 8. Consent, recording and anonymisation
- **Consent form (to be finalised by the owner; do not invent legal wording):** says what the study is, that the product is a research prototype and the participant is evaluating the design, not themselves; that they can stop at any time with no penalty; what is recorded (screen, audio, notes) and who sees it; how long it is kept; how to withdraw; contact for questions `[FILL]`. Signed or recorded verbal consent is logged per participant. `[FILL: form text]`
- **Ethics and data-protection:** whether any institutional review, privacy notice or local law applies is not known to this document. `[FILL: owner decides and records the answer]`
- **Recording:** only with consent; screen and voice only; no face; no account or personal tab on the shared screen (fresh profile). Storage location `[FILL]`; access limited to `[FILL]`; retention period `[FILL]`; deletion date `[FILL]`.
- **Anonymisation:** notes and reports use participant codes only (P01...). The code-to-person key, if any, is kept separately from the notes and is not committed to the repository. Quotes are used verbatim but with identifying details removed. Anything the participant says that identifies a third party is removed.
- **Secrets:** the study uses the anonymous development gateway, so no token is entered. If a variant uses a token, it must be a throwaway the facilitator minted (see package 05, section 7), never shown in notes or recordings.
- Participants are told the product is a research prototype and that nothing they enter is sent anywhere outside the machine.

## 9. Facilitator script (read as written; do not improvise product explanations)
**Before the participant arrives:** gateway reset; browser on the start page (`http://localhost:3000/?gateway=<default address>`); P6's second gateway ready but not started; recording ready; task cards in order; answer key out of sight.

1. **Welcome (2 min).** "Thank you for coming. We are testing a design, not you. Nothing you do can be wrong. If something is confusing, that is useful to us. You can stop at any time. I can't answer questions about how the tool works during the tasks because I want to see what it's like without help, but I will answer everything at the end."
2. **Consent (3 min).** Go through the form; get consent; ask about recording separately.
3. **Background (2 min).** "What do you do day to day? Which of these tools have you used: caches, proxies, gateways, load tests?" Write brief codes only.
4. **Think-aloud practice (2 min).** "While you work I'd like you to say out loud what you're looking at, what you're trying to do and what you expect to happen. Practice: tell me what you'd do to find the weather forecast for tomorrow, out loud, using this browser." (Do not use the product for practice.)
5. **Tasks (about 30 min).** Hand over one card at a time. Start the timer when the participant begins reading. Do not nod, answer, point or react to correct or incorrect steps. If silent for 15 seconds say only: "What are you thinking?" Ask the probe after the task ends. Hand over the SEQ question after each probe.
6. **At P6.** Say "I'm going to stop the gateway now" only if the card says the gateway will stop; otherwise stop it silently when the participant has finished the first part, note the exact clock time, and note when the participant first mentions it.
7. **SUS and trust items (3 min).**
8. **Debrief interview (5 min).** "What was the most confusing moment? What would you change first? Was there anything you didn't trust? If a colleague asked you what this tool is for, what would you say?" Then explain the purpose, answer questions, thank them, give compensation `[FILL]`.
9. **Immediately after:** the facilitator and note-taker agree the observation sheet within 15 minutes. Reset the gateway.

**Observation sheet (per participant per task):**
| Participant | Task | Start / end time | Completed unaided | Hinted | Errors (list) | Hesitations | Help requests | Probe answer (verbatim) | Critical error (C1..C4) | SEQ | Notable quotes | U-xx observed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `[FILL]` | | | | | | | | | | | | |

## 10. Analysis plan
1. **Before looking at results,** fix the coding rules (this document) and name two people who will code: `[FILL: coder 1]`, `[FILL: coder 2]`. Probe answers and critical errors are scored by both independently against the key; disagreements are settled by discussion and the disagreement count is reported.
2. **Per task:** report counts (for example "5 of 6 completed unaided"), times as median and range, errors and help requests, SEQ values, probe results, and all verbatim misreadings. Report counts, not percentages alone. Do not average across tasks.
3. **Intervals:** where a rate is quoted, add its 95 percent Wilson interval (method named, calculation shown). No significance tests; the sample cannot support them.
4. **Subgroups** (core, adjacent, assistive-technology, phone): describe separately and anecdotally; do not pool or compare.
5. **Issue log:** every observed problem gets a finding ID `USE-001...`, the task, participants affected (codes), a verbatim or described observation, severity (the rubric's Blocking, High, Medium, Low), and the matching U-xx hypothesis if any. Severity is set by both coders.
6. **Hypothesis table:** for each of U-01..U-14: observed (which participants), not observed, or contradicted. A hypothesis that was not observed is not thereby false (task coverage may have missed it); say so.
7. **Threshold table:** each pre-registered threshold, the observed value, met or not met.
8. **Pass or fail** per section 7, then a short list of the changes the evidence supports, each linked to findings. Recommendations are not findings.
9. **Report** `[FILL: results path]` includes: build, dates, participant table (codes), deviations from this protocol, and the limits in section 11.

## 11. Known biases and limits
- **Sample:** 5 to 8 people, recruited by convenience from a channel the owner chooses; not representative of anyone; likely to over-represent people close to the owner.
- **Facilitator effects:** the facilitator knows the product and may lead by tone; mitigation is the script, a second observer, and recording; it is not eliminated.
- **Think-aloud effects:** talking changes how people work and slows them; times are not comparable to silent use.
- **Observer effect:** people try harder when watched.
- **Order effects:** later tasks benefit from learning in earlier tasks; only P4/P5 and P7/P8 are rotated.
- **Fixture effects:** the Lab folder holds retained evidence, which is real project data but not a customer's; a debug gateway adds an EXPLORATORY label that a release build would not.
- **Short session:** nothing about learning over weeks or trust over time.
- **Anchoring risk from the AI walkthrough:** the hypotheses U-01..U-14 can make observers see what they expect; the second coder works from recordings without reading section 2.
- **Task wording:** the cards avoid product terms, but the answer keys use them; scoring depends on the page text of the build tested, so keys must be re-checked if the build changes.
- **Not covered:** accessibility needs beyond the one or two assistive-technology participants (see 03); non-English users; participants under time pressure or on a poor connection.

## 12. Sign-off
Protocol frozen by owner on `[FILL]`; pilot done on `[FILL]` (changes made after the pilot: `[FILL]`); sessions held on `[FILL]`; coders `[FILL]`; result `[FILL: PASS / FAIL, with the threshold table attached]`; owner acceptance of misses by finding ID `[FILL]`. Until recorded, usability status stays **TESTED (agent walkthrough only)**.
