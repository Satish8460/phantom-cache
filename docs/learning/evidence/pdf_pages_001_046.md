# PDF page inspection, pages 1 to 46

Status: complete for pages 1 to 46 (all 46 page images opened and read). Findings are visual observations of the rendered PDF, plus one measurement made on the HTML source (section 4).
Audience: the handbook owner and the build maintainer.
Last reviewed: 2026-10-30 (machine clock `date`: Fri Oct 30 17:24 IST 2026, when the images were taken).
Subject: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf` (183 pages, A4 595 x 842 pt per `/MediaBox`), built from `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK.md` by pandoc and headless Chrome (`docs/learning/build/build_handbook.mjs`, `docs/learning/build/handbook.css`).

## 1. Method

- Real PDF rendering, not HTML. A Node script in `D:\Temp\pdf_p1.mjs` (derived from `D:\Temp\pdf_pages.mjs`) used `ui/scripts/cdp-lib.mjs` (its own throwaway Chrome profile, no other browser touched), viewport 1100 x 1560, device scale 1.
- For each page N in 1..46: navigate to `about:blank`, then to `file:///D:/Personal/phantom-cache/docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf#page=N&zoom=page-fit&toolbar=0&navpanes=0`, wait 3 s, save `b.shot('pNNN.png')` into `D:\Temp\pdf_shots1\`.
- All 46 PNGs (p001 to p046) were opened with the Read tool. Each image shows page N in full plus the top of page N+1; the verdicts below are for page N, and the visible top of N+1 was used only to see what follows. No page was skipped.
- Page-fit zoom here displays the page at about 96 dpi (page 794 px wide), that is, roughly the size a laptop shows a whole page.
- White-space percentages are estimates from the image (blank vertical extent inside the text area divided by the text-area height).
- Evidence tags: "observed" means seen in the page image; "measured" means computed in Chrome on `build/handbook.html` (print media, 794 px viewport).

## 2. Per-page results

Verdict key: OK = no page-specific defect found. MINOR = cosmetic or small. DEFECT = clearly wrong or hard to use. "Systemic" items (section 4) apply to every page and are not repeated per row.

| Page | Heading(s) on page | Verdict | Defect detail |
|---|---|---|---|
| 1 | Title, subtitle, table of contents (start) | MINOR | Title and the contents list share one page (no title page); no page numbers or visible links in the contents; contents text is very small. No running header or number here is intended (`@page :first`). |
| 2 | Contents (Ch 4 to Ch 7) | OK | Running header and page number "2" present. Text small but legible. |
| 3 | Contents (Ch 7 to Appendix F) | OK | Same. |
| 4 | Contents tail (Appendix F to H) | DEFECT | Near blank, about 70 percent white (content ends about a third down). Next page: "About this handbook". Cause is `#TOC { break-after: page }`. |
| 5 | About this handbook; How the chapters are organised; Three ways to read it; The status labels (table); What this book does and does not establish; Judge boxes | OK | Status-label table fits the width. |
| 6 | Chapter 1 start; What you will learn; 1.1 What a cache is | MINOR | About 28 percent blank at the bottom because Figure 1.1 did not fit and moved to page 7. |
| 7 | Figure 1.1; 1.2 Why applications need caching | MINOR | Figure legible but its text is about 6 pt; the first box label is truncated in the figure itself ("GET /api/v1/cache/.."). Table and code block fit. |
| 8 | 1.2 (judge boxes); 1.3 Where caching goes wrong; 1.3.1 Cache stampede | OK | About 15 percent blank at the bottom (Figure 1.2 moved on); acceptable. |
| 9 | Figure 1.2; 1.3.1 to 1.3.3 | OK | Figure legible. Dense text. |
| 10 | Caution box (three limits); 1.3.4 Invalidation; judge box; 1.4 Why one cache is not enough | MINOR | The sentence that introduces the "two designs" table is on page 10 but the table is at the top of page 11 (about 8 percent blank here). |
| 11 | Table (designs); concrete-cost box; Figure 1.3 (three pillars) | OK | Figure legible; dashed/dotted/solid borders readable. |
| 12 | Trade-offs; judge boxes; 1.5; 1.6 Pillar 1 (table starts) | OK | Table breaks across the page with the header row repeated on page 13. |
| 13 | 1.6 table continued; 1.7 Pillar 2 | OK | Header repeated. |
| 14 | 1.7 end; judge box; 1.8 Pillar 3; 1.9 Intended users (table starts) | OK | Table split with repeated header. |
| 15 | 1.9 table continued; 1.10 (table); 1.11 | OK | About 12 percent blank at the bottom; the next judge box did not fit. |
| 16 | Judge box; Key ideas; What you should now understand; Knowledge check; Answers box | MINOR | The "Answers" box text is about 6 pt, the smallest body text in this range. |
| 17 | Codebase exploration tasks; Challenge questions (end of Ch 1) | DEFECT | Near blank, about 58 percent white. Next page: Chapter 2 start (forced `h1 { break-before: page }`). |
| 18 | Chapter 2 start; 2.1 Two viewpoints; 2.1.3 table | OK | About 5 percent blank. |
| 19 | 2.2 Component map (Figure 2.1); 2.2.1 Responsibilities (table starts) | OK | Figure legible but small (about 6 pt). |
| 20 | Table continued; concept box; judge box; 2.3 HTTP surface (routes table starts) | OK | Tables split with repeated headers. |
| 21 | Routes table continued; 2.3.2; 2.3.3 (table); 2.4 Request flow 1 | OK | Heading 2.4 and its one-line explanation are at the bottom; the next subsection is on page 22 (not stranded). |
| 22 | 2.4.2 Terms; Figure 2.2 (sequence diagram); worked example; 2.4.3 | DEFECT | Figure 2.2 has overlapping text: the "authenticate: role cache:read, tenant from the token" label overruns its box; the dashed return arrows pass through the labels "Age, ETag" and "hit = true". Legible but untidy. About 20 percent blank at the bottom. |
| 23 | Judge box; 2.5 Request flow 2; Figure 2.3 (sequence diagram); 2.5.3 | DEFECT | Figure 2.3 has overlapping text: lines run through "(cold key)", "permit, pinned IP, 5 s timeouts" and "A: 200 MISS B: 200 COALESCED origin saw one request". |
| 24 | 2.5.3 end; worked example; 2.5.4; judge box | OK | None. |
| 25 | 2.6 Freshness; 2.6.1 to 2.6.3; Figure 2.4 (flowchart) | OK | Figure legible. |
| 26 | Judge boxes; 2.7 DELETE; 2.7.1; 2.7.2 | MINOR | About 30 percent blank because the 2.7.3 heading and Figure 2.5 moved to page 27 together. Not over the 40 percent threshold. |
| 27 | 2.7.3 How DELETE works; Figure 2.5 (sequence diagram) | DEFECT | Figure 2.5 label text collides with the dashed lifelines ("generation + 1", "150 ms deadline", "membership epoch checked", "fan-out, asynchronous, per-peer queue"). |
| 28 | 2.7.4 How nodes communicate (table); consistency box; judge box | DEFECT | The "How nodes communicate" table is cut off at the right edge: the Status column text is truncated mid-word ("..._204_primary_success_vs_504_t", "a separ", "and 504 on tim", "discards the conn"). The Transport column is squeezed into a very narrow strip (words broken over about 15 lines). This is the widest table in the book (section 4). |
| 29 | Judge box; 2.8 Phantom Lab and the UI (Figure 2.6) | OK | Figure legible. |
| 30 | Judge box; 2.9 Configuration; key table (starts) | OK | Table fits. |
| 31 | Key table continued; 2.9.3; judge box; 2.10 | OK | None. |
| 32 | 2.10 end; 2.11; Key ideas; Understand; Knowledge check | OK | None. |
| 33 | Answers box; Codebase exploration tasks; Challenge questions (end of Ch 2) | MINOR | Answers box text about 6 pt. About 28 percent blank at the bottom before Chapter 3. |
| 34 | Chapter 3 start; 3.0 How to read this chapter (table) | OK | Table fits (it is one of the tables wider than the layout in the HTML, but is not visibly clipped here). |
| 35 | Figure 3.1 (technology stack); 3.1 dependency table | OK | Figure legible. |
| 36 | 3.1 end; caution box; 3.2 Rust; judge box | OK | About 12 percent blank. |
| 37 | Judge box; 3.3 Async programming and Tokio (table) | OK | None. |
| 38 | Judge box; 3.4 Axum, hyper and tower | OK | None. |
| 39 | Discrepancy box; judge box; 3.5 Concurrency; Figure 3.2 | OK | Figure legible. |
| 40 | 3.5.2 Shards; judge box; 3.5.3 Atomics | OK | None. |
| 41 | 3.5.4 RwLock epoch snapshot; Figure 3.3 (concurrency model) | OK | Figure legible. |
| 42 | 3.5.5 Arc and channels; judge box; 3.6 Data structures (table); 3.7 start | OK | Table fits. |
| 43 | 3.7 TLS and mTLS; judge box; 3.8 JWT; 3.9 serde | OK | None. |
| 44 | 3.9 end; 3.10 Build and packaging (table); 3.11 Frontend | OK | None. |
| 45 | 3.11 end; judge box; 3.12 Testing tools (table starts) | OK | Table split, header repeated on page 46. |
| 46 | 3.12 table end; judge box; 3.13 Supply-chain observations | OK | About 8 percent blank. Next page begins "Key ideas to remember" (not stranded). |

Checked on every page and found fine: running header "PhantomCache Engineering Handbook" present on pages 2 to 46; page numbers 2 to 46 present and correct (page 1 has none, by design); order of content follows the Markdown; no raw `:::`, backticks or table pipes visible; no garbled characters; no code line cut off (code blocks wrap); no heading stranded at the bottom of a page; no judge box split across pages (they move whole to the next page, which causes the blank areas above).

## 3. Counts (pages 1 to 46)

- OK (no page-specific defect): 33 pages (2, 3, 5, 8, 9, 11, 12, 13, 14, 15, 18, 19, 20, 21, 24, 25, 29, 30, 31, 32, 34 to 46).
- MINOR: 7 pages (1, 6, 7, 10, 16, 26, 33).
- DEFECT: 6 pages (4, 17, 22, 23, 27, 28).
- Total 33 + 7 + 6 = 46. All pages also carry the systemic small-text issue (section 4).

Defects by type (page-specific):

| Defect type | Pages |
|---|---|
| Table cut off at the right edge | 28 |
| Page over 40 percent blank | 4 (about 70 percent), 17 (about 58 percent) |
| Figure with overlapping text | 22, 23, 27 |
| Figure text small or one truncated label | 7 (minor), 19 (small) |
| Table separated from its introducing sentence | 10 (minor) |
| Small answers-box text (about 6 pt) | 16, 33 (minor) |
| Page 20 to 40 percent blank | 6 (28 percent), 26 (30 percent), 33 (28 percent), 22 (20 percent) |
| Code line cut off | none |
| Raw Markdown or pandoc artefacts | none |
| Missing or wrong running header or page number | none (page 1 intentionally bare) |
| Stranded heading | none |
| Judge box split | none |
| Font inconsistency | none seen (one serif-free sans family, one monospace) |
| Garbled characters | none |

Figures seen (Figure 1.1 to 3.3, 12 in this range): all are legible at page-fit but their text is about 6 pt; none is clipped or cut by a page break (each figure moves to a new page whole).

## 4. Readability, and the root cause found

Observed: at page-fit on a laptop the body text is small. Body line pitch is about 14 px at 96 dpi, so the effective body size is roughly 7 to 8 pt; table, code and answers-box text is roughly 6 pt; the title (declared 30 pt) and chapter heading (declared 22 pt) are also smaller than declared. Printed on A4 this would be hard to read for sustained study (comfortable print body is 9 to 11 pt). Screen reading needs zooming in.

Measured on `docs/learning/build/handbook.html` (print media, 794 px viewport): the document `scrollWidth` is 1045 px against a 779 px layout width, a ratio of 0.745, and the computed body size is 14 px (10.5 pt, as `handbook.css` declares). Chrome's print shrink-to-fit therefore scaled the whole document by about 0.75 (10.5 pt becomes about 7.8 pt), matching the observed size. The cause is wide tables. Measured right edges of tables that overflow the 779 px layout: 1045 (section 2.7.4 "How nodes communicate", the widest, which sets the scale and is the one visibly clipped on page 28), 927, 919 (section 2.2.1 Component table, page 19), 896 (section 3.0 Choice table, page 34), 878, 813 and 790 (the 2.3.1 routes table, pages 20 to 21). Cause of the width: long unbreakable `<code>` identifiers (test names, paths) in table cells. This is a diagnosis from measurement; the scaling effect is inferred from the match between the 0.745 ratio and the observed text size, and was not isolated by an A/B rebuild (not done: the task forbids editing other files).

The same 7 tables were measured across the whole book, so the shrink applies to all 183 pages, not only to pages 1 to 46.

## 5. Near-blank pages (over 40 percent white)

| Page | White space (estimate) | What follows |
|---|---|---|
| 4 | about 70 percent | Page 5: "About this handbook" (forced page break after the contents). |
| 17 | about 58 percent | Page 18: "Chapter 2: System Architecture" (forced `h1` page break after the last section of Chapter 1). |

Below the threshold but notable: pages 6, 26 and 33 (about 28 to 30 percent each; a figure or heading plus figure was pushed to the next page); page 22 (about 20 percent).

The same chapter-end pattern (short last page then a forced break) will recur at every chapter end; the pages after 46 were not inspected here.

## 6. Recommended fixes

All are proposals; nothing was changed.

1. Fix the cause of the shrink and the page 28 clipping (highest value). In `docs/learning/build/handbook.css`, in the `table` rule (line `table { border-collapse: collapse; width: 100%; ... }`), add `table-layout: fixed;` and add a new rule `td code, th code { overflow-wrap: anywhere; word-break: break-word; }`. Pandoc also emits `<colgroup>` widths for pipe tables with long lines; if they still force width, add `table col { width: auto; }` or set `max-width: 100%` on `table`. Rebuild, then check that `document.documentElement.scrollWidth` equals the 779 px layout width; text will grow from about 7.5 pt to the declared 10.5 pt and the page count will rise (expect roughly 40 to 50 percent more pages).
2. After fix 1, re-decide the sizes: keep `html { font-size: 10.5pt }`, and consider `table { font-size: 9pt }` (already so) and `.answers { font-size: 9pt }` (already so) instead of the current effective 6 pt. Check that print is acceptable at the new size.
3. Table 2.7.4 specifically (`PHANTOMCACHE_ENGINEERING_HANDBOOK.md`, section "### 2.7.4 How nodes communicate", line about 914): move the long test identifiers (for example `security_phase4b_router_tests::test_p4b_http_gateway_204_primary_success_vs_504_t...`) out of the Status column into the text below the table, and widen the Transport column by shortening its cell (it currently holds a full sentence). Same treatment for the Component table in 2.2.1, the routes table in 2.3.1 and the Choice table in 3.0.
4. Near-blank pages 4 and 17 (and chapter ends in general): either accept the chapter-start page break (it is intended) and shorten the last page of the contents by lowering `#TOC { font-size }` spacing, or remove `break-after: page` on `#TOC` (`handbook.css`, the `#TOC, nav#TOC` rule) so "About this handbook" can start on page 4. For chapter ends, keep `h1 { break-before: page }`; blank space there is normal book layout.
5. Sequence diagrams (Figures 2.2, 2.3, 2.5, source files under `docs/learning/assets/` or `docs/learning/src/`; exact file not checked): widen the label boxes and give labels a white background (`paint-order: stroke` or a `<rect fill="white">` behind each label) so lifelines and arrows do not run through text; shorten "authenticate: role cache:read, tenant from the token" or wrap it on two lines.
6. Figure 1.1 first box: change the truncated "GET /api/v1/cache/.." to the full text or "GET /api/v1/cache/{path}".
7. Contents on page 1: put the title on its own page and add page numbers or links to the contents (pandoc `--toc` with `--toc-depth` limited to 2 would also cut the contents from 4 pages to about 2 and remove the near-blank page 4).
8. Table 1.4 intro (page 10): add `break-after: avoid` to the paragraph that introduces a table, or accept as is.

## 6b. Not inspected

Nothing in pages 1 to 46 was left uninspected. Pages 47 to 183 are outside this brief.
