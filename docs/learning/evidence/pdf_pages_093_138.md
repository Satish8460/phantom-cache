# PDF page inspection, pages 93 to 138

Status: complete for the 46 pages in range; every page PNG was opened and viewed.
Audience: handbook build owner and editors.
Last reviewed: Fri Oct 30 17:28 IST 2026 (machine clock, `date`).

## Method

- File inspected: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf` (the real PDF, not the HTML), rendered in the Chrome PDF viewer.
- Script: `D:\Temp\pdf_insp3.mjs`, using `ui/scripts/cdp-lib.mjs` (own throwaway Chrome profile). Viewport 1100x1560, scale 1. For each page: navigate to `about:blank`, then to `...rebuilt.pdf#page=N&zoom=page-fit&toolbar=0&navpanes=0`, wait 3 s, screenshot.
- Output: `D:\Temp\pdf_insp3\p093.png` to `p138.png` (46 files). All 46 were opened with the Read tool. No page was skipped.
- Each screenshot shows the target page plus a strip of the next page; verdicts use the target page only (the printed page number matches the requested page for all 46).
- Scale: the A4 page renders about 794 px wide, so 1 px is about 1 CSS px at 96 dpi. Font sizes below are estimates from line pitch in the screenshots, not from font metadata.
- White-space figures are visual estimates of the unused part of the text area.

## Per-page results

| Page | Heading(s) | Verdict | Defect detail |
|---|---|---|---|
| 93 | 6.2 tail (trade-offs), judge box, 6.3 Management and API access | OK | Table fits. |
| 94 | 6.3 judge box, 6.4 Peer-to-peer trust | OK | Code block fits. |
| 95 | 6.4 tail, 6.5 Input validation (heading + 1 paragraph) | DEFECT: near-blank, stranded heading | About 56% blank. Heading 6.5 and one paragraph sit at the top; Figure 6.2 (break-inside: avoid) is pushed to p96. |
| 96 | Figure 6.2, 6.5 body | DEFECT: figure small, minor overflow | Figure text about 5 pt equivalent; the "Rejections" box text ("origin_cap") runs past the dashed box's right edge. Readable only with zoom. |
| 97 | 6.5 tail, 6.6 Tenant isolation | OK | Poisoning table splits across p97/98 with repeated header; fits width. |
| 98 | 6.6 tail, 6.7 Data exposure, 6.8 Resource exhaustion (heading + intro) | OK | Heading 6.8 has an intro paragraph before the page break; acceptable. |
| 99 | 6.8 Bounds tables, SSRF | OK | Table fits. |
| 100 | 6.8 judge boxes, 6.9 Failure recovery, 6.9.1 | DEFECT: near-blank | About 42% blank. Figure 6.3 (and its 6.9.2 heading) moved to p101. |
| 101 | 6.9.2 + Figure 6.3, 6.9.3 | OK | Figure legible but small (about 5 to 6 pt text). |
| 102 | 6.9.3 worked example, 6.9.4, 6.10 | OK | |
| 103 | 6.10 judge, 6.11 Table 6.11a | OK | Table fits. |
| 104 | Table 6.11a (S10-S32), caption "Table 6.11b" | DEFECT: stranded caption | The bold caption "Table 6.11b. The same rows..." is the last line of p104; its table starts at the top of p105. About 10% blank. |
| 105 | Table 6.11b | OK | Whole page is one table; fits width; text tiny (about 5 pt). |
| 106 | 6.11 note, 6.12 Evaluating performance, 6.12.1 to 6.12.3 | OK | |
| 107 | 6.12 judge boxes, 6.13, M1 to M3 | OK | |
| 108 | M4 to M8, judge box, 6.14 | OK | |
| 109 | 6.14 tail, Key ideas, What you should understand, Knowledge check | OK | |
| 110 | Knowledge check answers, Codebase exploration, Challenge questions | OK | About 27% blank before the chapter break (expected). |
| 111 | Chapter 7 title, What you will learn, 7.1 | DEFECT: near-blank | About 40% blank (borderline). Next page opens with the "Phantom Lab: who talks to whom" figure, which did not fit. |
| 112 | Figure 7.1, 7.1.1 The parts | DEFECT: figure line crosses text | In Figure 7.1 an arrow line runs through the "T1 runner: src/lab (SIMULATED)" box text, and the Lab API arrows cross box labels. Figure otherwise legible. |
| 113 | 7.2, 7.2.1 to 7.2.4 | OK | Discrepancy callout and worked examples intact. |
| 114 | 7.2.4 tail, 7.2.5, 7.2.6 | OK | |
| 115 | Figure 7.2, 7.2.7, 7.3, 7.3.1, 7.3.2 | DEFECT: figure overlap; split | Figure 7.2: the vertical feedback arrow passes through the "SIMULATED workload" box text. The sentence ending "computes a relative reduction:" is the last line; its code block `r_i = ...` is at the top of p116. |
| 116 | 7.3.2 code, 7.3.3, 7.3.4 | OK (note) | About 27% blank (before Figure 7.3 on next page; acceptable). The orphaned code block belongs to the p115 defect. |
| 117 | 7.4 + Figure 7.3, 7.4.1 to 7.4.4 | OK | Figure legible. |
| 118 | 7.4.4 tail, 7.4.5, 7.4.6, 7.5, 7.5.1 | OK | |
| 119 | 7.5.1 tail, 7.5.2 table, 7.5.3 | OK | |
| 120 | 7.5.3 tail, 7.6 table, 7.7, 7.7.1 | OK | |
| 121 | 7.7.2, 7.8, 7.8.1, 7.8.2 (intro) | DEFECT: near-blank | About 45% blank. Figure 7.4 (maturity table-as-figure) moved to p122. |
| 122 | Figure 7.4, 7.8.2 comparisons | OK | Figure text and source lines small (about 5 pt). |
| 123 | 7.8.2 tail, 7.8.3, 7.8.4, 7.8.5 | OK | |
| 124 | 7.8.6, 7.8.7, 7.9 Judge boxes | OK | About 12% blank (judge box moved whole). |
| 125 | 7.9 judge boxes | OK | About 15% blank; boxes not split. |
| 126 | 7.9 judge boxes | OK | About 22% blank; boxes not split. |
| 127 | 7.9 judge boxes, 7.10 Discrepancies, Key ideas, What you should understand | OK | |
| 128 | What you should understand tail, Knowledge check, Codebase exploration, Challenge | OK | |
| 129 | Chapter 8 title, 8.1, 8.2, 8.2.1 | OK | Table fits; last row of 8.2.1 table spills (see p130). |
| 130 | 8.2.1 tail, 8.2.2, 8.2.3 | DEFECT (minor): orphan table row | One row ("Rust code committed beyond the first snapshot") plus repeated header is alone at the top of p130. |
| 131 | Figure 8.1 (status map), 8.3 to 8.3.3 | OK | Figure legible but small. |
| 132 | 8.3.4 to 8.3.7, 8.4, 8.4.1, 8.4.2 | OK | |
| 133 | 8.4.2 table tail, 8.4.3, 8.4.4 | OK | |
| 134 | 8.4.4 tail, 8.4.5, 8.4.6, 8.5, 8.6 | OK | |
| 135 | 8.6 tail, 8.7, 8.7.1, 8.7.2 | OK | |
| 136 | 8.7.2 tail, 8.8 + Figure 8.2 | OK | Figure legible. |
| 137 | 8.8 tail, 8.9, 8.10 | OK | |
| 138 | 8.10 table tail, 8.11, 8.12 heading + intro | OK | |

## Counts

- Pages inspected: 46 of 46. Pages OK: 37 (a few with notes). Pages with defects: 9 (95, 96, 100, 104, 111, 112, 115, 121, 130).
- By type (a page can carry more than one):
  - Near-blank page (about 40% blank or more): 4 (95, 100, 111, 121).
  - Figure defects (small text, line crossing text, text past box edge): 3 (96, 112, 115).
  - Stranded heading or caption: 2 (95, 104).
  - Table row or code block separated from its context: 2 (115/116 code block, 130 orphan row).
  - Table cut off at the right edge, code line overflow: 0.
  - Raw Markdown or pandoc artefacts (`:::`, backticks, pipes): 0.
  - Broken or garbled characters: 0.
  - Wrong or missing running header or page number: 0. Header "PhantomCache Engineering Handbook" and the centred page number are present and in sequence on all 46 pages.
  - Judge boxes or callouts split across pages: 0 (all kept whole; this is the cause of the white space on 124 to 126).
  - Font inconsistency: 0 observed.

## Near-blank pages

| Page | Blank (est.) | What follows on the next page |
|---|---|---|
| 95 | about 56% | Figure 6.2 (GET request path diagram) |
| 100 | about 42% | 6.9.2 heading + Figure 6.3 |
| 111 | about 40% | Figure 7.1 (Phantom Lab architecture) |
| 121 | about 45% | Figure 7.4 (overlap/maturity figure) |

Below the threshold, for information: 116 (about 27%), 110 (about 27%), 126 (about 22%), 125 (about 15%), 128 (about 15%).

## Readability

- Observed, not good: body text measures about 13.5 px line pitch, which with the CSS line-height of 1.5 implies roughly 9 px (about 6.8 pt) body text. Table and code text is smaller, roughly 6 px (about 4.5 to 5 pt). `docs/learning/build/handbook.css` asks for `html { font-size: 10.5pt }` and tables at 9pt, so the PDF is rendered at about 0.65 of the intended size.
- On a laptop at page-fit zoom, body text is legible only with effort; table, code and figure text is not comfortably legible without zooming. On A4 print, body at about 7 pt is small but readable; table, code and figure text at about 5 pt would be hard to read, and the big tables on p103 to p105 and p133 would be the worst.
- Hypothesis (not verified): Chrome shrink-to-fit during `--print-to-pdf` (some element wider than the page content box forces a global scale-down), or a print scale from the headless flags. This is the most valuable thing to check, because fixing it would affect every page and change the pagination, so the page numbers in this report would shift.

## Recommended fixes

1. Restore true scale (affects all pages). In `docs/learning/build/build_handbook.mjs` (the `--print-to-pdf` call, about line 48), switch to a CDP `Page.printToPDF` with `scale: 1`, `preferCSSPageSize: true`, `printBackground: true`, or find and constrain the over-wide element (candidates: `pre`, wide tables, inline SVG figures) so Chrome does not shrink the whole document. Re-measure after the change; body should land near 10.5 pt.
2. Near-blank pages 95, 100, 111, 121. In `docs/learning/build/handbook.css`, line 27 (`figure { ... break-inside: avoid }`): cap tall figures with `figure img, figure svg { max-height: 70vh; width: auto; max-width: 100% }` so they fit in the remaining space, or allow the figure to sit before the long text in the Markdown source (`src/ch06_security_reliability_performance.md`: Figure 6.2 near 6.5 and Figure 6.3 near 6.9.2; the chapter 7 source: Figure 7.1 near 7.1.1 and Figure 7.4 near 7.8.2).
3. Stranded caption on p104. In `src/ch06_security_reliability_performance.md` near line 498 (`**Table 6.11b. ...**`), keep the caption with its table: give the caption a pandoc class (`::: {.tablecap}`) with `.tablecap { break-after: avoid }` in `handbook.css`, or emit it as a real table caption.
4. Figure overlaps (p112, p115) and small figure text (p96). These are in the figure sources under `docs/learning/assets/` (Figure 7.1 "Phantom Lab: who talks to whom", Figure 7.2 "One turn of the T1 loop", Figure 6.2). Re-route the arrows around the boxes, widen the "Rejections" box in Figure 6.2, and increase SVG font size (or reduce canvas width) so text is at least about 7 pt at print size.
5. Orphan code block and table row (p115/116, p130). Add `break-after: avoid` for paragraphs ending with a colon (a pandoc filter or a `.lead-in` class), and for tables keep `thead { display: table-header-group }` and add `tr:last-child { break-before: avoid }`, or accept these as minor.
6. Rebuild and re-inspect, since fix 1 will repaginate the whole document and make the page numbers in this report stale.
