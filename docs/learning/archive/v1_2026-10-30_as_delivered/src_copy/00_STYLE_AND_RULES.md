# Handbook style and rules (for every writer)

**Status:** Active. **Owner brief:** "PhantomCache Engineering and Technical Defence Handbook" (2026-10-30). One coherent handbook, eight chapters plus back matter. Reader: knows programming basics, is still building deep distributed-systems expertise, must be able to explain AND defend the system to technically strong judges. Plain English first; no technical detail lost.

## 1. Evidence rules (non-negotiable)
- Read the CURRENT code, tests and docs. Do not trust the README, older handoffs, or this brief's examples. Name real files, functions, structs, routes, config keys and tests. Verify every symbol exists (Grep/Read) before you cite it. No line numbers unless you opened the line in the current tree; prefer `path::symbol`.
- Status labels, used exactly and consistently, on every capability claim: **IMPLEMENTED** (code exists), **TESTED** (a named test exercises it; say what the test does and does NOT prove), **INDEPENDENTLY VERIFIED** (a human or external reviewer; almost nothing qualifies: AI reviews are "AI second look", never this), **PARTIAL**, **PROTOTYPE ONLY**, **DESIGNED BUT NOT IMPLEMENTED**, **DEFERRED**, **BLOCKED**, **UNVERIFIED**. Code existing is not tested behaviour; passing tests are not correctness.
- Performance: never invent a number or claim superiority. Quote a measurement only with its source file, build profile, host, workload, seed, repetitions and limits (CLAUDE.md section 4). Keep loopback, simulated (T1), WSL2 and physical-host results separate and labelled. Simulation is not real execution.
- Check older review findings against the current tree and their latest status: `docs/release/g4/cycle1_*_review.md`, `docs/release/g4/cycle1_findings_disposition_register.md`, `docs/release/known_limitations_frozen_code.md`, `docs/research/g4a01_race/`, `docs/research/u01_backend/`, `docs/security/`, `docs/research/m3_durability/`, `docs/research/claims_ledger.md`, `docs/research/code_findings.md`, `CLAUDE.md` (section 6 notes that parts are outdated). Do not assume a finding is open or fixed: check, and say what you checked. Sandbox patches (U-01, G4A-01 proposal, M3) are NOT integrated; say so.
- Known facts to VERIFY (do not copy): GET is served from the local cache only; routing is plain rendezvous with constant weight and used for DELETE; PIE/AIMD intelligence and the `pie` controller are not what older docs call them; the default TTL setting is not wired; invalidations owed by the issuer are in memory only; peer GetRequest stub. If you find they changed, say so.
- If you cannot read or verify something, write "UNVERIFIED: <why>" in the text. Never invent implementation detail. If you find a discrepancy between documents and code, document it accurately in the chapter and in your evidence file; do not fix the product.
- Frozen/sealed: never edit frozen evidence, preregistration/freeze records, protected source. Never read `D:\Personal\.phantom-sealed-m6`. Never print any 64-hex commitment string or secret.

## 2. Teaching pattern (repeat it for every major idea)
1. Simple explanation (an analogy is welcome, but only as the doorway). 2. The correct technical terms, defined. 3. How it works: diagram, pseudocode, small worked example with numbers. 4. Where it lives in PhantomCache (file::symbol). 5. Trade-offs and alternatives (only claim why an alternative was rejected when evidence in the repo supports it; otherwise say "our reasoning, not a recorded decision"). 6. What can go wrong: failure modes, limits, security and performance effects. 7. Prepare to defend it: judge boxes.
Language: short sentences, active voice, concrete nouns. Define each term at first use. Do not pad, do not repeat what another chapter owns: link instead (`see Chapter 4, section 4.3`). No marketing words ("robust", "seamless", "cutting-edge", "state of the art") and no unsupported superlatives.

## 3. Mandatory callouts (Pandoc fenced divs; the build styles them)
Judge box (at least 4 per chapter, more in Ch 3 to 7; each question must be one a technically strong judge would really ask; answers must NOT overstate capability; where a feature is incomplete, give the honest, technically strong answer):
```
::: judge
**JUDGE MAY ASK**

Question: ...

**Suggested answer** (about 30 to 60 seconds spoken, plain English):

...

**If the judge asks further:**

...

**Evidence to know:** `path::symbol`, test name, document, status label.
:::
```
Key concept summary: `::: concept` ... `:::` (3 to 6 lines). Worked example: `::: example` ... `:::`. Warning about a limitation or a trap: `::: caution` ... `:::`. Status block for a capability table: ordinary Markdown tables with a Status column.
Diagrams: original SVG files you write by hand into `docs/learning/assets/chNN_<name>.svg` (viewBox width 900, readable text at least 13 px equivalent, no external fonts/images/scripts, `<title>` and `<desc>`, colours distinguishable in greyscale print), embedded as `![Figure N.M: caption](../assets/chNN_name.svg)` followed by one line "Source: files/functions this was derived from". Every diagram must be checked against the code (list what you checked in the evidence file). ASCII diagrams in code fences are allowed for small sequences. Sequence diagrams: draw participants and numbered messages in SVG or a fenced text block.
Code: short excerpts only (at most about 15 lines), always with the path; pseudocode for algorithms; never paste long source.

## 4. Chapter skeleton (every chapter)
Title, "What you will learn" (5 bullets), the sections (with the teaching pattern), then in this order: **Key ideas to remember**; **What you should now understand** (bullets); **Knowledge check** (5 to 10 questions, answers at the end of the chapter in a `::: answers` block); **Codebase exploration tasks** (2 to 3, each with start file, what to find, what you should learn); **Challenge questions** (1 to 2 hard engineering questions with a short model answer). Heading levels: `#` chapter, `##` section, `###` subsection, numbered like `4.3`. Put the status label next to every capability claim.

## 5. Files and ownership
You write ONLY: `docs/learning/src/chNN_<slug>.md` (your chapter), `docs/learning/evidence/chNN_evidence.md` (table: claim | source file::symbol | status label | how you verified (what you opened/ran) | limits), and `docs/learning/assets/chNN_*.svg`. Nothing else. Do not edit product code, tests, configs, other docs, other chapters. Read-only everywhere else. No git add/commit/stash/restore/reset/clean/checkout. No cargo builds or test runs unless the chapter needs a fact only a run can show (then: via `pwsh -NoProfile -File D:\Temp\locks\with_cargo_lock.ps1 -Label LRN -Cmd "<cmd -j 1>"`, `CARGO_TARGET_DIR=D:\Personal\_learn_target`, copy sources to `D:\Temp\sbx_learn` first; avoid if possible). TEMP=TMP=D:/Temp. Use `date` for any timestamp.
Length: concise relative to depth. Target 5,000 to 9,000 words per chapter (Chapters 4 to 6 may be longer), dense, no repetition. Keep a running list of "discrepancies found" at the end of your evidence file.
Final report to the lead: chapter file path and word count, number of figures, judge boxes, discrepancies found (document vs code), UNVERIFIED items, anything another chapter must reconcile with.
