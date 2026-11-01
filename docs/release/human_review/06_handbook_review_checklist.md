# Package 06: handbook review checklist (the owner's own review)

**Status:** Draft for owner review. **Audience:** the owner, as reader and examiner of the handbook. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`.

**What is under review:** the handbook in `docs/learning/` (full text `PHANTOMCACHE_ENGINEERING_HANDBOOK.md`; chapter sources in `docs/learning/src/`: `ch01` to `ch08` and the appendices `bm01` to `bm08`), and the short guide `docs/learning/short/PHANTOMCACHE_SHORT_GUIDE.md` (an AI-prepared summary whose own verification record says "human review pending"). Handbook version or commit reviewed `[FILL]`. Reviewer: the owner, code `[FILL]`. Dates `[FILL]`.

## 1. Purpose and what it establishes
The aim is not to check the handbook for typos. It is to answer two questions: **Did it teach me?** (can I now explain the system without help and answer a hostile question honestly) and **Is it right?** (where I notice a claim that contradicts the code, the tests or another document, it is recorded, not fixed on the spot).

This establishes the owner's understanding and a list of discrepancies. It does not verify the handbook's technical correctness: the code and tests are the authority, and the handbook's own claims carry the labels IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, UNVERIFIED, and "INDEPENDENTLY VERIFIED" (which, per Appendix G, applies to nothing in the project yet). Your reading does not change any of those labels.

## 2. The rule for reading
- Read in the order below. After each item, close the book and answer the questions out loud or on paper **before** looking at the answers.
- Mark each question: **Y** (answered correctly without looking), **P** (partly, needed the text), **N** (could not). Also record time spent.
- Whenever you notice a statement that looks wrong or unclear, log it in section 7 immediately; do not stop to fix it.
- If a statement carries a status label you think is too strong (for example TESTED where you doubt what the test proves), log it as a label dispute.
- Nothing you read here is a security or usability result; it is a teaching document.

## 3. Reading plan and questions
Suggested order: Appendix A (orientation), the short guide (compression), Chapters 1 to 8 in order with Appendix H after Chapters 4, 5 and 6, and Appendix E last as the exam. Appendices B, C, D, F and G are for lookup while reading.

### 3.1 Appendix A: PhantomCache on one page
After reading, without looking:
1. In two sentences, what is PhantomCache and what is it not? (It is a research prototype in-memory HTTP caching gateway with coalescing; it is not a distributed cache, not linearizable, not AI, not independently verified.)
2. Name the three pillars and the honest status of each.
3. What happens to a GET on a miss, and what does a DELETE `204` mean?
4. Which numbers are you allowed to quote about performance? (Answer should be: none as a claim; local, dated evidence only.)
5. Which one claim in the page would you most expect an attacker or a judge to challenge?

### 3.2 The short guide
1. Can you rebuild its eight sections from memory as a list of headings?
2. Name one thing the short guide leaves out that a judge would ask about.
3. Find one sentence where the short guide is stronger than the matching chapter. (If you find one, log it.)

### 3.3 Chapter 1: The problem and the vision
Questions: What four things go wrong without a cache (stampede, stale data, invalidation, origin overload) and which part of the system answers each? What are the three pillars and how do they constrain each other? What does the vision claim that the implementation does not yet do?

### 3.4 Chapter 2: System architecture
Questions: Draw the components and the life of a hit, a miss and a DELETE. Where is the process boundary? What does `X-Cache-Status` tell a caller? Where does Phantom Lab fit relative to the serving path?

### 3.5 Chapter 3: Technology stack and engineering decisions
Questions: Why Rust, why this HTTP stack, why in-memory? What were the alternatives and what did each cost? Which decisions are owner decisions still open?

### 3.6 Chapter 4: Core cache engine and algorithms
Questions: Explain shards, single-flight coalescing, TTL and the stale cap, generation and epoch fencing in plain words. What invariants must never break (never evict a key with an active operation; stale cap violations stay at zero; byte accounting equals the recorded sizes)? What does the "SIEVE/LRU/S3-FIFO" naming really mean in the code (the project notes say some named evictors were scans; check the chapter's current statement)? Which finding is open against the fencing (G4A-01)?

### 3.7 Chapter 5: Distributed systems and workload intelligence
Questions: Why is GET local only? How is a DELETE fanned out and what does 204 promise? What is lost when the issuer crashes (the lost-invalidation case)? What is the "intelligence" actually, and what does the evidence say about the adaptive controller versus a static cap? What is only a prototype with no live caller?

### 3.8 Chapter 6: Security, reliability and performance
Questions: Name the five zones and the five adversaries. Walk Figure 6.1 and the matrix S01 to S32: pick three rows and say what protects, what tests it, what is unresolved. What are the High findings and owner-named findings? Why is `/api/control` the weakest management route? What does the performance section refuse to claim, and why? (This chapter is also the base for package 05.)

### 3.9 Chapter 7: Phantom Lab, experimentation and innovation
Questions: What is Scenario B, what is the M6 evaluation, and what do their results say, including the negative one? What do REAL, SIMULATED, DERIVED, REPLAYED, PREDICTED, UNAVAILABLE mean? What does "VALID" mean in a run class and what does it not mean? What is the innovation argument and what does it not claim?

### 3.10 Chapter 8: Limitations, roadmap and technical defence
Questions: Which items in the implementation-versus-roadmap matrix (Appendix G, 44 rows) are IMPLEMENTED, TESTED, PARTIAL, PROTOTYPE ONLY, NOT IMPLEMENTED? What would you build next and in what order? How do you answer "Who has independently verified this?" (Nobody has; AI reviews are "AI second looks".)

### 3.11 Appendix H: The ten concepts
For each of H.1 to H.10, say the three-sentence explanation aloud with the book closed, then name the three key facts and the one thing a judge will probe; then compare:
| Concept | Y / P / N | Notes |
|---|---|---|
| H.1 Cache hit, miss and origin load | | |
| H.2 Stampede and single-flight coalescing | | |
| H.3 Sharded locking | | |
| H.4 TTL, staleness and the stale cap | | |
| H.5 Generation and epoch fencing | | |
| H.6 Invalidation and what a 204 means | | |
| H.7 Rendezvous hashing and why GET is local-only | | |
| H.8 The failure model, and what is NOT guaranteed | | |
| H.9 The origin concurrency gate, and what the evidence shows | | |
| H.10 Simulation versus real execution, and the M6 negative result | | |
Then take H.11 (the self-test of ten questions) closed-book and score it: `[FILL: score]` out of 10.

### 3.12 Appendix E: The judge-question bank
The bank holds 93 entries in nine themes (Fundamentals, Architecture, Technology, Algorithms, Distributed systems, Security, Innovation, Evaluation, Limitations). Start with the "Ten hard questions to rehearse first". Then the "Never say" list. For each theme, have someone else (or a recording) pick five questions at random; answer aloud in under 90 seconds; compare with the book.
| Theme | Questions asked (IDs) | Answered honestly without overclaim (count) | Any answer that tempted me to overclaim |
|---|---|---|---|
| Ten hard questions | | | |
| Fundamentals | | | |
| Architecture | | | |
| Technology | | | |
| Algorithms | | | |
| Distributed systems | | | |
| Security | | | |
| Innovation | | | |
| Evaluation | | | |
| Limitations | | | |

### 3.13 Other appendices (lookup use)
B (architecture explained once), C (technology and algorithm reference), D (glossary: note any term you met in the book that is missing here), F (source-file map: pick three files and confirm they exist and do what F says), G (implementation versus roadmap: pick five rows and check the status label against the chapter that teaches it).

## 4. "Did it teach me?" rubric
Score each dimension after finishing, honestly, on this scale: **0** = no, **1** = partly, **2** = yes. Do not average unless you want to; the dimensions are not interchangeable.

| Dimension | Question | 0 / 1 / 2 | Evidence (what I could or could not do) |
|---|---|---|---|
| Explain | Can I explain each of the ten concepts in plain words with the book closed? | | |
| Locate | Given a claim, can I say which file or function it lives in and which test covers it? | | |
| Limit | For each capability, can I say its honest status label and what is not established? | | |
| Defend | Can I answer the ten hard questions without overclaiming and without hedging in the wrong places? | | |
| Connect | Can I tell how the three pillars constrain each other and why? | | |
| Order | Is the order of chapters one in which I rarely hit an undefined term? (Record every undefined term.) | | |
| Load | Were there sections so dense that I read them without understanding? (Record where.) | | |
| Trust | Do I know which claims in the handbook I would verify before repeating? | | |

**Verdict the owner may record:** `[FILL: teaches / teaches with fixes / does not teach]`, with the dimensions that scored 0 or 1 listed. This is the owner's judgement, not a measurement.

## 5. Checks against the current tree (optional, for the owner or a delegate)
The handbook cites code by `file::symbol`. For a sample, check that the symbol exists and does what is said. Suggested sample (the owner may change it):
| # | Claim to check | Where it is claimed | Result |
|---|---|---|---|
| 1 | GET is served from the local cache only | Appendix A; Chapter 2 | |
| 2 | `finalize_fetch` stores only if the generation and epochs still match | Chapter 4; H.5 | |
| 3 | `PHANTOM_DEFAULT_TTL_MS` has no runtime effect (the TTL fallback literal is 60,000 ms) | Chapter 3 or 8; H.4 | |
| 4 | `/api/control` with an unknown event sets every node Active | Chapter 6 (6.3) | |
| 5 | Cancelled run shows no verdict (UI side) | Chapter 7 | |
| 6 | The M6 negative result matches the stored evidence | Chapter 7; H.10 | |
Record the file and line you opened, not just the outcome.

## 6. Status labels the reader should challenge
At least one disagreement is expected; log them. Particularly: any TESTED label where the test is weak (the project notes say passing tests do not establish correctness and several assert stub behaviour); any IMPLEMENTED label for something with no live caller; any wording that reads as a guarantee (linearizable, bounded staleness, exactly-once); any performance figure; any claim of novelty (novelty claims need a verified primary source).

## 7. Where discrepancies are recorded
Do not edit the handbook, the code or other documents from this review. Record every discrepancy in the log below, in a dated results file `[FILL: path under the results folder in 00_README.md]`. The owner then decides, per entry, whether to correct the handbook, correct a source document, open a finding, or accept it.

Cross-references: project-wide document discrepancies already known are in `docs/research/vision/discrepancy_register.md`; handbook build checks are in `docs/learning/short/VERIFICATION.md` (short guide) and `docs/learning/evidence/`. Do not duplicate those; add new ones only.

| ID | Where I read it (chapter, section, line or page) | What it says | What I think is right, and the evidence (file, line, test) | Kind (factual / label too strong / unclear / undefined term / missing / contradicts another doc) | Severity (misleading / confusing / cosmetic) | Owner's decision |
|---|---|---|---|---|---|---|
| HB-001 | `[FILL]` | | | | | `[FILL]` |
| HB-002 | `[FILL]` | | | | | `[FILL]` |

## 8. Sign-off
Owner code `[FILL]`; dates `[FILL]`; time spent `[FILL]`; rubric verdict `[FILL]`; discrepancies logged `[FILL: count]`; actions assigned `[FILL]`. The handbook's status remains "AI-prepared, human review pending" until this is recorded.
