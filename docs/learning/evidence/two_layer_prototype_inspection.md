# Two-layer handbook prototype: PDF inspection (Part I and Part II)

**Status:** AI inspection (agent "PDF inspector L"); not independently reviewed by a human. PARTIAL: 70 of 327 pages opened and read (see section 2). Nothing in the repository other than this file was edited.
**Audience:** the lead and the owner, deciding whether to adopt the two-layer structure and what the builder must fix first.
**Last reviewed:** Fri Oct 30 18:22 IST 2026 (machine clock, `date`).

## 1. Method (reproduced here)

- Subjects: `docs/learning/proposal_v2/out/learning.pdf` (Part I, 194 pages) and `reference.pdf` (Part II, 133 pages), built by `build_two_layer.mjs` with the single-volume stylesheet at true text size. Read first: `evidence/two_layer_analysis.md` (what moved where) and `out/manifest.md`.
- Real PDF in headless Chrome's built-in viewer over the repo CDP helper (`ui/scripts/cdp-lib.mjs`, throwaway profile, no other browser touched). Script `D:\Temp\pdfL_pages.mjs`: viewport 1100x1560, for each page `about:blank`, then `file:///.../out/<name>.pdf#page=N&zoom=page-fit&toolbar=0&navpanes=0`, wait 3 s, screenshot to `D:\Temp\pdfL\<name>_NNN.png`. Every PNG was opened and read. The viewer shows page N plus the top of page N+1; verdicts below are for page N only.
- Pages opened: Part I 1-8, then every 8th page (16 ... 192), plus 193 and 194 (33 pages). Part II 1-6, then every 8th page (8, 16 ... 128), plus 131, 132, 133 (25 pages). Total 58 pages opened (33 + 25); the brief's "about 70" was an estimate and the stated page rule gives 58.
- Also read (not rendered): word counts of `out/learning.md`, `out/reference.md`, `out/manifest.json`; grep counts of stubs, markers and cross-references in the two Markdown files.
- Limits: sampling only (58 of 327 pages = 18%); no print test, other PDF viewers, PDF outline or accessibility check; text correctness of the handbook was not re-verified.

## 2. Per-page table

Verdict key: OK, or the defect type. "Near-blank" = more than about 35 to 40 percent white space.

### 2.1 Part I, `learning.pdf` (194 pages)

| Page | Heading(s) on page | Verdict |
|---|---|---|
| 1 | Title (Part I: Learning Handbook), subtitle "PROPOSAL (two-layer structure)", start of contents | OK (cover has no header or number by design). Contents has no page numbers. |
| 2 | Contents (Ch 3 to 5) | OK, running header and number "2". |
| 3 | Contents (Ch 5 to 7) | OK. |
| 4 | Contents (Ch 8, App A, App H) | OK; about 40% blank at the end of the contents (acceptable). |
| 5 | Two-layer callout box only | DEFECT: near-blank (about 80% empty); the callout is followed by a page break before "About this handbook". |
| 6 | About this handbook; How the chapters are organised; Three ways to read it | OK visually. Content stale: see 3(c). |
| 7 | The status labels used everywhere (9-row table); What this book does and does not establish; Judge boxes | OK; about 22% blank at the bottom. |
| 8 | Chapter 1; What you will learn; 1.1 What a cache is | OK. The line "How it works. Every cache runs the same loop:" is at the page foot and its code block starts on page 9 (minor). |
| 16 | (end of 1.4) judge box "Why a distributed cache" | DEFECT: about 45% blank below the box (next heading plus box kept together on page 17). |
| 24 | Chapter 2 (reading rules); What you will learn; 2.1 Two viewpoints | OK. |
| 32 | Figure 2.3 (miss/coalescing sequence diagram); 2.5.3 How it works | OK; figure legible and complete. |
| 40 | Caution callout; stub "2 further judge boxes for this section are in Part II, R4, under the same heading."; 2.8 and Figure 2.6 | OK; figure legible; stub present and clear. |
| 48 | Figure 3.1 (technology stack); stub "[Part II, R1: 3.1 The dependency picture in one table]"; 3.2 Rust | OK. The numbering jumps from 3.0 to 3.2, but the stub explains it. |
| 56 | 3.5 concurrency (Relaxed/SeqCst, RwLock epoch snapshot, caution callout) | OK. |
| 64 | Chapter 3 end: answers box, Codebase exploration tasks, Challenge questions | OK; about 25% blank at the chapter end. |
| 72 | 4.4 pseudocode block; worked example "stampede arithmetic"; Trade-offs | OK. |
| 80 | "The known race: G4A-01"; Figure 4.4; Status today; stub; start of a judge box | OK; figure legible. |
| 88 | 4.9 judge box; stub; 4.10 table (splits to p89, header row repeated); stubs "[Part II, R1: 4.11]" and "[Part II, R5: 4.12]" | OK. |
| 96 | Figure 5.1 (cluster topology) then white space | DEFECT (borderline): about 38% blank; the next figure (mTLS sequence) is kept whole on page 97. |
| 104 | Figure 5.4 (DELETE fan-out) and its numbered steps | OK; figure legible. |
| 112 | End of a judge box; 5.8 GetRequest stub, DR-029 and DR-027 | OK. |
| 120 | 5.13 D and E; 5.14 M6 result; judge box starts p121 | OK. |
| 128 | 6.2 worked example; "What is not done" table; 6.3 route table (splits to p129, header repeated) | OK. |
| 136 | End of DR-029 text; stub; 6.7; 6.8 (heading plus two lines at the foot) | OK; the table of bounds starts on p137. |
| 144 | 6.14 Reliability, health and bottlenecks; stubs; Key ideas to remember (starts p145) | OK; about 25% blank before the next heading (callout kept whole). |
| 152 | 7.2 worked example (alias sampling), 7.2.4 workload-family table | OK. |
| 160 | 7.5 Scenario B: 7.5.1, 7.5.2; results table | MINOR: the results table breaks after its first data row (header repeated on p161); about 25% blank at the page foot. |
| 168 | 7.8 claims table, "Evidence that would strengthen", 7.8.6, 7.8.7; 7.9 begins | OK. |
| 176 | 8.3.1 to 8.3.5; 8.3.6, 8.3.7 and stub "[Part II, R5: 8.4 ...]" (p177) | OK. |
| 184 | 8.13.1 judge boxes; 8.13.2 heading with only "8 further judge boxes ... are in Part II, R4"; 8.13.3 | OK technically, but 8.13.2 is a heading plus one stub line (reads as a gap; see 3(a)). |
| 192 | Appendix H: H.5 to H.7 | OK. |
| 193 | H.7 to H.10 | OK; page number 193. |
| 194 | H.9 to H.11 Self-test: ten questions (last page) | OK; the book ends cleanly. No closing pointer to Part II. |

### 2.2 Part II, `reference.pdf` (133 pages)

| Page | Heading(s) on page | Verdict |
|---|---|---|
| 1 | Title (Part II: Technical Reference), subtitle, contents R1 to R6 | OK; about 50% blank (short contents). Contents lists groups and appendices only, no sections, no page numbers. |
| 2 | Two-layer callout box only | DEFECT: near-blank (about 85% empty); same page-break-after-callout problem as Part I p5. |
| 3 | R1 heading; 3.1 dependency table; caution callout | OK. |
| 4 | Caution callout; 3.6 data-structures table | OK; about 15% blank. |
| 5 | 4.11 capability status table; 5.15 starts | OK; table splits with repeated header. |
| 6 | 5.15 table (cont.); 6.11 intro and caption "Table 6.11a. Threats and the protections that exist." at the page foot | DEFECT: stranded table caption; the table itself starts on p7. |
| 8 | 6.11 matrix rows S13 to S25 | OK; legible, columns intact. |
| 16 | Appendix C: C.1 tail, C.2 algorithm table | OK. |
| 24 | "Tables extracted from Part I sections": 1.6 table; 2.3 table begins | OK; heading carries the original section number, readable. |
| 32 | R2; Appendix F entries 1 to 2 | OK. |
| 40 | App F 12 Key documents and registers | OK; about 15% blank. |
| 48 | Appendix D glossary (P to S) | OK; every entry ends "(Ch N, N.M)" with no "(Part I)" marker. |
| 56 | R4 Appendix E: ARC-5 to ARC-8; 3. Technology; TEC-1 | OK. |
| 64 | App E DIS-8 to DIS-12 | OK. |
| 72 | App E EVA-6 to EVA-12 | OK. |
| 80 | "full chapter judge boxes": 2.5 and 2.6 boxes (with small grey section headings), 2.7 begins | OK; heading kept above each box. |
| 88 | 3.12 box; "Chapter 4: Core Cache Engine and Algorithms" heading; 4.2 boxes | OK. |
| 96 | 5.6 and 5.7 boxes; 5.8 begins | OK; about 12% blank. |
| 104 | 6.12 box; next box on p105 | DEFECT: about 45% blank (box kept whole, next box does not fit). |
| 112 | Two boxes (6.6-area, 7.9 M6 box); next box on p113 | OK; about 20% blank. The 7.9 boxes carry no section heading at the top (first one of that section is on an earlier page). |
| 120 | R5: 7.10 discrepancies table; 8.4 Known defects, 8.4.1; 8.4.2 table | OK. |
| 128 | R6 Appendix B: Figure B.1; B.1; B.2; B.3 | OK; figure legible. |
| 131 | B.7 and B.8 | OK. |
| 132 | B.8 end; B.9; B.10 begins | OK. |
| 133 | B.10 to B.12 (last page) | OK; ends cleanly. |

### 2.3 Counts from the table

- Opened: 58 pages (Part I 33, Part II 25). OK: 49. Defects: 5 near-blank or stranded-caption defects (Part I p5, p16, p96 borderline; Part II p2, p104), 1 stranded caption (Part II p6), 1 table split after one row (Part I p160, minor), 1 minor orphan (Part I p8).
- No raw Markdown artefacts, no code or table clipped at the right margin, no illegible or clipped figure, no missing running header or page number (other than the covers, by design), and no content out of order in the sampled pages. Table columns in Part II (6.11, App C, App F) stay inside the margins and are readable at this size.
- Pattern: all near-blank pages come from keep-together rules (figure, judge box, callout) plus the "page break after the front-matter callout". These are layout rules, so they are fixable in the builder or stylesheet.

## 3. Reader evaluation

### (a) Does Part I read as a coherent progressive text?

Mostly yes, with three weaknesses.
1. The chapter spine is intact (what you will learn, sections, worked examples, figures, key ideas, knowledge check, exploration, challenge questions). On the sampled pages (8, 24, 32, 72, 80, 104, 120, 152) I met no passage that depended on something that was removed without a pointer.
2. Pointer stubs exist and are clear. Two kinds: "[Part II, R1: 3.1 The dependency picture in one table]" (22 in the Markdown, bold, at the point where a section left) and italic "N further judge boxes for this section are in Part II, R4, under the same heading." (44 in the Markdown). Table extractions use "[The full table is in Part II, R1, 'Tables extracted from Part I sections', under 2.3 ...]" (7). Reproduced by grep on `out/learning.md`.
3. Gaps are visible: section numbers jump (3.0 to 3.2; 4.10 then Key ideas; 1.9 to 1.11 in the contents) and the contents lists only what stayed, so a reader sees numbering holes with no explanation until they reach the stub in the body. Section 8.13.2 is a heading plus one stub line (all eight of its judge boxes moved), which reads as an empty section.
4. The "(Part II)" marker convention for in-text cross-references does NOT exist. The text still says "see Chapter 4, section 4.11" with the original numbering; `grep "(Part II"` returns nothing. The front matter says "targets that moved are listed in `manifest.md`", which a reader of the PDF does not have. Cross-references found to moved targets in `learning.md` (regex, approximate): 4.11 (line 9, front matter), 6.13 (lines 144, 4234, 4302, 4457), 4.12 (1745), 8.7 (4046), Appendix E (line 32, "One hour before a presentation"). This matches the analysis figure (about 10). I did not find a marker on any of them.
5. Front matter is stale: "How the chapters are organised" still says "The appendices at the end give ... a technology and algorithm reference, a glossary, a consolidated judge-question bank, a source-file map, an implementation-versus-roadmap matrix", all now in Part II; the reading route "One hour before a presentation: Appendix A, Appendix H, then the judge bank (Appendix E)" sends the reader to a book they are not holding. The Appendix A tail in Part I does carry six "[Part II, R?: Appendix X]" stubs, which is the one clear piece of wayfinding.
6. The mixed density is unchanged: Part I still contains long code listings, route and bounds tables (6.8, 6.3), worked-example boxes, 23 full judge boxes and the 8.x limitation text, so it is a "teach and defend" book, not a short learning text.

### (b) Does Part II stand as a usable reference?

Partly.
1. Every moved item is present and keeps its original heading and numbers (3.1, 3.6, 4.11, 5.15, 6.11, 7.10, 8.4, Appendices B to G; judge boxes under their section heading, with the chapter heading shown once). Pages 5, 6, 8, 16, 32, 48, 56, 80, 88, 120, 128 confirm this.
2. Findability is weak. The contents is six lines (R1 to R6 with appendix names), no section list, no page numbers. R1 holds items from five chapters (3.1, 3.6, 4.11, 5.15, 6.11, App C, App G, extracted tables) in chapter order without a chapter label on the heading, so "where is the capability table for Chapter 5" requires scanning. The 68 moved judge boxes are in one block with section headings but are not listed in the contents. There is no index. The glossary (R3) exists and is usable.
3. The PDF has no outline (no `/Outlines` in either file). Part I carries about 324 link annotations (internal contents links); Part II about 28; the 453 "Ch N, N.M" pointers in Part II (glossary, App F, App E "Where taught") are plain text, with no "(Part I)" marker and no link, and a reader holding only Part II has no way to follow them.
4. Page numbering restarts at 1 in each PDF, so "see page N" is ambiguous; there is no running label saying "Part I" or "Part II" (the header says "PhantomCache Engineering Handbook" on both).
5. Judge boxes that moved keep their wording, labels and the question text; the 68 R4 boxes are about 22 pages of near-uniform shaded boxes plus Appendix E (about 25 pages), so R4 is the largest block of Part II (page numbers for R4 were not measured; see section 4).

### (c) Front matter of each part

1. Relationship statement: both parts have a boxed "Two-layer proposal" note. Part I says it is the Learning Handbook, moved items are marked "Part II", Part II holds the tables, judge bank, glossary, source map and inventories, "Nothing was deleted", and gives file names `reference.md` / `reference.pdf`. Part II says it is consulted, not read in order, items moved unchanged, section numbers are the original ones, and where a section was split its heading is repeated and marked as a continuation. This is clear for a developer, but it uses repository file names and the word "proposal" and sits alone on a mostly blank page (p5 / Part II p2).
2. Status label: neither part carries the AI-PREPARED / AI-REVIEWED / HUMAN REVIEW PENDING label asked about; a grep of both Markdown files and `src/` finds none of those strings. The only status wording is "PROPOSAL (two-layer structure)" on the title page and, in Part I p7, "What this book does and does not establish" (written and checked by AI agents; "No part of the project, including this book, has had an independent human review"). That is honest but is not the label requested.
3. Status-label legend (IMPLEMENTED ... UNVERIFIED, 9 rows): present only in Part I p7. Part II says "The status labels are defined in the front matter of Part I", but its pages are dense with those labels (4.11, 5.15, App G), so Part II is not self-contained; a one-page legend repeated at the head of R1 is needed.

### (d) Size versus the aim: is 194 pages "concise"?

- Reproduced: Part I 194 pages, Part II 133 pages, total 327 pages. Words: `learning.md` 77,569 and `reference.md` 56,320 whitespace tokens (133,889 together, including repeated continuation headings and generated stubs); the manifest's alphanumeric count is 73,256 + 52,142 = 125,398, equal to the original, with removed duplicates 0.
- The analysis estimated Part I at "about 106 pages" by scaling the old 183-page figure. That scaling does not hold: the 183 pages were typeset with shrunk text. At true size Part I is 194 pages (about 380 words per page, 33 figures and many shaded boxes), so the Part I page figure is 83% above the analysis estimate. The split removed about 41% of the words from the first book, but the page gain is smaller than that because the type is now true size. The single volume at true size would be about 327 pages (derived; I did not render it).
- Honest view: 194 pages (about 6 hours of reading at the analysis's 200 wpm, more with the study aids) is not "concise". It is a complete learning-and-defence text with the reference material removed. It would be called concise only next to 327 pages.
- To reach about 100 to 120 pages without deleting a technical fact, the content must move or reflow, not be cut. Estimated levers, from the manifest word counts and the pages seen (estimates, not tested; pages assume about 380 words per page):
  1. Study aids to a separate workbook or Part III: knowledge checks, exploration tasks, challenge questions and answers are about 7,700 words, so about 18 to 20 pages. Keep "Key ideas to remember" (one page each) in Part I.
  2. Judge boxes: 23 kept boxes are about 5,300 words, so about 14 pages; keep about 8 in Part I and move the rest (as the analysis already did for 68): about 8 pages.
  3. Appendix H (about 2,170 words, 6 pages) restates the chapters (it is the "ten concepts" card plus self-test); move it to Part II or merge into Key ideas: about 5 to 6 pages.
  4. Layout: the near-blank pages (Part I p5, p16, p96, and the 25 to 45 percent gaps seen at p4, 64, 144, 160) come from keep-together rules; allow judge boxes and figures to break or shrink, drop the page break after the front-matter callout, collapse the contents (the 4-page contents repeats five stock headings per chapter, about 40 of its 150 lines, and has no page numbers): about 8 to 12 pages.
  5. Typography: tighter margins or line height on the stylesheet (a layout choice, no content change): about 5 to 10 pages. The text is true size now, so go carefully.
  6. Repeated explanations that are paraphrases, not copies (the analysis lists about 16,000 words as an upper bound; DELETE and 204 are explained in 2.7, 5.5, 6.9, 8.3.3 and H.6; G4A-01 in 4.5, 4.7, 5.x, 8.x, H.5 and several boxes): replace later copies with a pointer. That is editing, which was not done and needs approval: about 10 to 15 pages.
  7. Combined: 194 minus about 18 to 20, 8, 6, 10, 7, and 12 is roughly 130 to 135 pages. Reaching 100 to 120 would also need moving whole deep-dive sections to Part II (for example 7.2 deterministic simulation steps, 5.13 adaptive algorithms one by one, 3.11 to 3.12) or a smaller body font. I could not verify this estimate by rebuild.

## 4. What I could not assess

- Page numbers of the start of each Part II group (R4 and the Appendix E block lengths were estimated from the table of pages seen, not located).
- Whether the PDF text is selectable or the contents links work (324 link annotations in Part I were counted, not clicked).
- The 12 Part I pages and 8 Part II pages between samples are unseen; defects there are possible.
- The correctness of any technical statement.

## 5. Recommendations for the builder (`build_two_layer.mjs`, `layer_map.json`)

Priority order. Items 1 to 4 are needed before the structure can be called usable.

1. Add the "(Part II, R?)" marker to every in-text cross-reference whose target moved. The builder already knows the moved ids (the 22 stubs show it). Regenerate "see section 6.13" as "see section 6.13 (Part II, R5)". Do the same in Part II for every "Ch N, N.M" in the glossary, App E "Where taught", App F and App B, as "(Part I)" (453 instances in `reference.md`).
2. Rewrite the generated front matter. Part I "How the chapters are organised" and "Three ways to read it" must name Part II for Appendices B to G and the judge bank; the "one hour" route should read "Appendix A, Appendix H, Part II R4 'Ten hard questions', 8.11". Remove the manifest.md pointer from the PDF text.
3. Put the status-label legend (9 rows) at the head of Part II R1, and add whichever document status label the owner decides on (AI-PREPARED / AI-REVIEWED / HUMAN REVIEW PENDING) to both title pages; the current subtitle is only "PROPOSAL".
4. Fix the near-blank pages: drop the page break after the front-matter callout on both parts (Part I p5, Part II p2); add `break-inside: auto` for judge boxes and allow the heading that follows to stay with its first paragraph rather than keeping the whole box whole; set a maximum figure height so two figures can share a page (Part I p96). Add `break-after: avoid` on the "Table 6.11a" caption (Part II p6) and `break-after: avoid` on the first table row (Part I p160).
5. Contents: add page numbers (the PDF is generated by Chrome print, so use target-counter or generate the index in a second pass); in Part I contents list chapters and numbered sections only (drop the five stock headings per chapter, about 40 lines); in Part II contents list every R-group item by section number and give R4 a per-chapter list (about 70 lines). Add an index or at least a "section number to location" table to Part II (so "4.11" is findable).
6. Section 8.13.2 (all boxes moved) and any other section left as a heading plus stub: either keep one box or write the heading as a stub line ("8.13.2 Consistency and the distributed layer: judge boxes are in Part II, R4") so it does not look like an empty chapter. Check the manifest for other sections with only stubs.
7. Header: change the running header per part ("PhantomCache Handbook, Part I: Learning" and "Part II: Technical Reference") and start Part II numbering with a prefix ("R-1" or continuing from 195). Without it the two PDFs are indistinguishable by header.
8. Contents links: Part II cross-references to Part I are plain text; consider one combined PDF (Part I then Part II) so that "(Part II)" and "(Part I)" markers can be real links and a single page numbering exists. The analysis lists this as an alternative; the inspection supports it.
9. In `layer_map.json` consider moving: study aids (to a workbook), Appendix H, and about 15 of the 23 kept judge boxes; keep the "Ten hard questions" and one box per chapter. A `learning_core` / `learning_full` pair could be generated from the same map for the 100 to 120 page goal.
10. Report the page counts in `manifest.md` (it states word counts only) so that the "about 106 pages" figure from the analysis, now disproved (194 pages measured), is replaced by the measured number.
11. Re-run after fixes and re-inspect pages 5, 16, 96 (Part I) and 2, 6, 104 (Part II) plus the contents pages.

## 6. Verdict

The two-layer structure is workable and nothing is lost: Part I teaches in order, stubs mark every gap, Part II keeps the moved items under their original headings, the PDFs have no clipped tables or figures on the sampled pages, and the word count matches the original exactly. It is not yet finished or concise: cross-reference markers, the front matter and the contents are not updated for the split (the main reader-facing defects), the status label requested is absent, Part II is hard to navigate, and at true size Part I is 194 pages, so the "focused learning handbook of about 100 to 120 pages" aim needs the study-aid, judge-box, layout and de-duplication changes above (about 130 to 135 pages without editing prose; below that requires moving more sections or changing type size). Recommend adopting the structure and fixing items 1 to 5 before the owner judges it.
