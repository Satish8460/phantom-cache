# PDF inspection, pages 139 to 183 (inspector 4)

Status: complete. All 45 page images were opened and viewed.
Audience: handbook maintainers (build owner, editors).
Last reviewed: Fri Oct 30 2026 (machine clock, `date` gave 17:24 IST at start of the run).

## Method

- Target: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf` (183 pages, A4), pages 139 to 183.
- Tool: Node script `D:\Temp\pdf_insp4.mjs` using the repo CDP helper `ui/scripts/cdp-lib.mjs` (its own throwaway Chrome profile). Viewport 1100x1560. For each page: navigate to `about:blank`, then to `file:///.../PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf#page=N&zoom=page-fit&toolbar=0&navpanes=0`, wait 3 s, save PNG. Output folder `D:\Temp\pdf_insp4\p139.png` to `p183.png` (45 files). This is the real PDF viewer, not the HTML.
- Each PNG shows page N at the top plus the first part of page N+1 below it. Only page N was judged. The part of N+1 was used to see what follows.
- White space was estimated by eye from the screenshot (text area is about 970 px tall at this zoom). Estimates are rough (plus or minus 5 points).
- Evidence labels: all findings are "reproduced here" from the screenshots. Causes in the Fixes section are hypotheses unless stated.
- Zoom note: at page-fit the page is 793 px wide, so 1 px is about 0.75 pt.

## Readability (global)

- Measured at page-fit (line pitch about 14 px = 10.5 pt, so glyph size is about 7 pt): body text about 7 pt, table text about 6 to 6.5 pt, inline code inside tables about 5 to 5.5 pt, figure labels on page 146 about 5 pt, figure caption (Figure B.1) about 4.5 pt.
- `build/handbook.css` says `html{font-size:10.5pt}` and tables 9 pt, so the rendered text is about two thirds of the intended size. Hypothesis: Chrome shrink-to-fit scaling (some wide element makes the layout wider than the page, and the print is scaled down; the @page margins are not scaled, which matches: the text area is the full 257 mm tall). Not confirmed. Another inspector covering chapters 1 to 8 should confirm whether the same ratio holds there.
- On a laptop at page-fit the body text is legible but small; tables and code spans in tables are hard to read without zooming. On A4 print, 7 pt body is readable with effort; 5 pt code in table cells and the figure labels are marginal to illegible.
- Line length: body text runs full width, about 150 to 175 characters per line (pages 153 to 157 glossary, 158 to 171 question bank). This is long for comfortable reading; consistent with the small scaled size.
- Fonts are consistent (sans body, monospace code) on all 45 pages. No broken or garbled characters.
- Running header ("PhantomCache Engineering Handbook", top right) and centred page number are present and correct on all 45 pages (139 to 183). The header is the same on every page (it does not name the current appendix); noted, not a defect.

## Per-page results

Legend: OK = no defect found. Codes: BLANK = mostly blank page; ORPHAN = table fragment of 1 to 3 rows alone on a page; FIG = figure legibility; RAW = raw Markdown token visible; DENSE = global small text (applies to every page, only repeated here where it is worse than elsewhere).

| Page | Heading(s) on page | Verdict | Defect detail |
|---|---|---|---|
| 139 | 8.12 Implementation versus roadmap matrix (table continues) | OK | Dense 6 pt tables; table splits to next page with header repeated. |
| 140 | 8.13 Preparing to defend; 8.13.1 Honesty and verification | OK | Two judge boxes whole. About 10% blank at bottom. |
| 141 | 8.13.2 Consistency; 8.13.3 Intelligence and performance | OK | Judge boxes whole. About 12% blank. |
| 142 | 8.13.4 Security; 8.13.5 Process, debt and the future | OK | Judge boxes whole; about 8% blank. |
| 143 | End of 8.13.5; Key ideas; What you should now understand; Knowledge check | OK | About 13% blank because the Answers box moves whole to next page. |
| 144 | Answers box; Codebase exploration tasks; Challenge questions | OK | About 28% blank before Appendix A (forced page break, expected). |
| 145 | Appendix A: PhantomCache on one page | OK (watch) | About 39% blank before Appendix B (forced page break). Just under the 40% line. Three-column pillar table fits. |
| 146 | Appendix B: complete architecture; B.1 to B.3 | FIG (minor) | Figure B.1 fits the page and is not clipped, but box text is about 5 pt and the caption about 4.5 pt; the "DELETE" and "miss" arrow labels touch box borders. Readable only when zoomed. |
| 147 | B.3 (end) to B.6.3 (B.4 The cache core, B.5 A hit, B.6 A miss) | OK | Dense text. |
| 148 | B.7 Memory; B.8 DELETE and the cluster; B.9 Observation | OK | Dense paragraphs, long lines. |
| 149 | B.10 Phantom Lab and UI; B.11 Security; B.12 Where architecture stops | BLANK | About 43% blank (content ends at 58% of text area). Next page: Appendix C (forced page break). |
| 150 | Appendix C: Technology and algorithm reference; C.1 Technologies | OK | 5-column table fits; code spans about 5 pt. |
| 151 | C.1 (end); C.2 Algorithms and mechanisms | OK | Table split across pages with header repeated. |
| 152 | C.2 (end); C.3 How to use the tables in a defence | OK | About 22% blank before Appendix D (forced break). |
| 153 | Appendix D: Glossary; A, B, C entries | OK | Letter headings are small bold letters with a large gap; fine. |
| 154 | Glossary C (end), D, E, F, G | OK | |
| 155 | Glossary G, H, I, J, K, L, M | OK | |
| 156 | Glossary M, N, O, P, Q, R, S | OK | |
| 157 | Glossary S (end), T, U, V, W, Z | OK | About 11% blank. |
| 158 | Appendix E: Consolidated judge-question bank; Ten hard questions; Never say | RAW (informational) | Intro text contains the literal `::: judge` inside an inline code span (the fenced-div opener quoted on purpose). Not a pandoc leak, but it looks like one; consider rewording. "Never say" table fits; last row is cut off to the next page (see 159). |
| 159 | Never say (one row); 1. Fundamentals; 2. Architecture (ARC-1, ARC-2) | ORPHAN | Page starts with the table header plus a single row ("Tests passing means it is correct") alone, then the section continues. Minor. |
| 160 | ARC-2 end to TEC-1; 2. Architecture, 3. Technology | OK | Q&A entry ARC-2 split across pages (acceptable, not a box). |
| 161 | TEC-2 to TEC-9 | OK | |
| 162 | TEC-10, TEC-11; 4. Algorithms (ALG-1 to ALG-5) | OK | |
| 163 | ALG-6 to ALG-13 | OK | |
| 164 | End of ALG-13; 5. Distributed systems (DIS-1 to DIS-7) | OK | About 7% blank at bottom. |
| 165 | DIS-8 to DIS-14 | OK | |
| 166 | DIS-15; 6. Security (SEC-1 to SEC-6) | OK | |
| 167 | SEC-7 to SEC-11; 7. Innovation (INN-1, INN-2) | OK | About 7% blank. |
| 168 | INN-3 to INN-7; 8. Evaluation (EVA-1, EVA-2) | OK | EVA-2 "Where taught" line moves to next page (acceptable). |
| 169 | EVA-3 to EVA-9 | OK | |
| 170 | EVA-10 to EVA-12; 9. Limitations and honesty (LIM-1 to LIM-4) | OK | EVA-10 title wraps to two lines; fine. |
| 171 | LIM-4 end to LIM-11 | OK | About 7% blank. |
| 172 | Appendix F: Important source files; F.1 to F.4 tables | OK | 4-column tables fit; path code spans about 5 pt. |
| 173 | F.4 (end), F.5, F.6, F.7 | OK | Section 4 table continues from 172 with header repeated. |
| 174 | F.7 (end), F.8, F.9 src/lab and src/lab_live | OK | About 6% blank. |
| 175 | F.9 (end, one row), F.10 Tests and benchmarks, F.11 ui, scripts and packaging | ORPHAN (minor) | Page starts with table header plus a single row (`src/lab_live/analysis.rs ...`) continuing F.9 from the previous page. |
| 176 | F.11 (end), F.12 Key documents and registers | OK | |
| 177 | F.12 (end, three rows) | BLANK + ORPHAN | About 89% blank. Only the header and 3 rows (CLAUDE.md, bench-results, lab-results) of the F.12 table, then nothing. Next page: Appendix G (forced page break). Worst page in this range. |
| 178 | Appendix G: Implementation versus roadmap matrix; G.1 Status vocabulary; G.2.1, G.2.2 | OK | About 8% blank. |
| 179 | G.2.3; G.3 Pillar 2; G.4.1; G.4.2 | OK | |
| 180 | G.4.2 (one row); G.5 Reconciled labels (red caution box) | BLANK + ORPHAN | About 70% blank. Page starts with a header plus a single row ("Dependency advisory scan and SBOM"), then G.5 and its box, then empty. Next page: Appendix H (forced break). |
| 181 | Appendix H: ten concepts; H.1 to H.4 | OK | |
| 182 | H.4 (end), H.5, H.6, H.7, H.8 | OK | |
| 183 | H.9, H.10, H.11 Self-test; end of document | OK | Last page, about 30% blank at the end (expected). |

## Counts

- Pages inspected: 45 of 45 (139 to 183). Pages not inspected: none.
- Pages OK (no defect): 38 (this includes 145, which is a borderline "watch" at about 39% blank, and 139 to 144, which have only global density).
- Pages with a defect or note: 7 (146, 149, 158, 159, 175, 177, 180).
- By type:
  - BLANK (over 40% blank): 3 pages (149 at about 43%, 177 at about 89%, 180 at about 70%).
  - ORPHAN (a 1 to 3 row table fragment alone at the top of a page): 4 pages (159, 175, 177, 180).
  - FIG (figure text too small): 1 page (146, minor).
  - RAW (literal `:::` token visible): 1 page (158, informational; it is quoted code, not a leak).
  - Table cut off at right edge: 0. Code line overflowing: 0. Heading stranded at page bottom: 0. Judge box split: 0. Wrong or missing header or page number: 0. Font inconsistency: 0. Broken characters: 0. Wrong content order: 0.
- Global: DENSE small text on all pages (see Readability).

## Near-blank pages (over 40%, plus borderline)

| Page | Estimated blank | What follows |
|---|---|---|
| 177 | about 89% | Appendix G starts on page 178 (forced `h1` page break). |
| 180 | about 70% | Appendix H starts on page 181. |
| 149 | about 43% | Appendix C starts on page 150. |
| 145 (borderline) | about 39% | Appendix B starts on page 146. |
| 144 (for info) | about 28% | Appendix A starts on page 145. |
| 152 (for info) | about 22% | Appendix D starts on page 153. |

Every near-blank page is caused by `h1 { break-before: page }` in `docs/learning/build/handbook.css` line 11 (each appendix starts a new page), combined with a short last page of the previous appendix.

## Recommended fixes

1. Orphan table rows and tail pages (pages 159, 175, 177, 180). File: `docs/learning/build/handbook.css`, near line 22 to 25 (table and `tr` rules). Options: (a) keep the last rows of a table with the previous ones, for example `tbody tr:nth-last-child(-n+3) { break-before: avoid; }` (Chrome honours this only partly, so verify in the PDF); (b) reduce `table { margin }` or `font-size` slightly so F.12 and G.4.2 tables finish on the previous page. Page 177 (three rows) is the clearest case: F.12 needs about 3 rows of space on page 176.
2. Near-blank pages (149, 177, 180; optionally 145). File: `docs/learning/build/handbook.css` line 11. Options: allow the shorter appendices to share a page, for example `h1.appendix { break-before: auto; }` or a `break-before: avoid` for Appendix H and G; or leave the forced breaks and accept blank space (standard print practice for appendices). If the owner wants fewer pages, removing the forced break before Appendix H alone saves page 180's blank space. Source Markdown: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK.md`, the `# Appendix G` and `# Appendix H` headings; a pandoc attribute such as `{.no-break}` plus a CSS class would be needed.
3. Figure B.1 legibility (page 146). Source: `docs/learning/assets/` SVG for `ch02_components.svg` (the figure names `ch02_components.svg`, `ch05_cluster_topology.svg`, `ch02_lab_ui.svg`) and the figure markup in the Markdown Appendix B. Increase the SVG font size (minimum 7 pt at print) or enlarge the figure to the full text width; add a small gap so the "DELETE" and "miss" arrow labels do not touch box borders. Raise `figcaption` size in `handbook.css` line 29 (currently 9 pt before scaling).
4. Overall small text (all pages). Find out why the rendered scale is about 0.67 of the CSS size: check `docs/learning/build/build_handbook.mjs` line 48 (headless Chrome `--print-to-pdf`) and look for a wide element (wide `pre`, unbreakable path in a table cell, or a wide SVG) that makes Chrome shrink the whole page. Candidate: long unbreakable code spans in the F and C appendix tables. Adding `overflow-wrap: anywhere` to `td code` and `table { table-layout: fixed }` is a low-risk test. Confirm by comparing glyph size on a chapter page against CSS 10.5 pt.
5. Literal `::: judge` on page 158. File: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK.md`, Appendix E intro ("Counted with `grep` on the fenced-div opener `::: judge`"). Reword to "counted by searching the source for the judge-box opener" so a reader does not think pandoc markers leaked.
6. Line length (pages 153 to 171). Optional: set a `max-width` of about 100 characters on the Appendix D and E body text, or enlarge the font once item 4 is resolved, to keep lines at 90 to 110 characters.
