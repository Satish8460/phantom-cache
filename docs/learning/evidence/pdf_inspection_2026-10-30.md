# PDF inspection record, 2026-10-30

**Status:** PARTIAL (9 of 183 pages opened and read). **Subject:** the handbook PDF built by `node docs/learning/build/build_handbook.mjs`. **Method:** the PDF was opened in headless Chrome's built-in PDF viewer over CDP (page-fit, toolbar hidden) and each captured page image was opened and read. HTML-only inspection is not accepted as PDF quality evidence (owner, 2026-10-30).

## Defects found and fixed
| # | Defect | Evidence | Fix |
|---|---|---|---|
| 1 | The appendices were assembled BEFORE Chapter 1 (table of contents listed Appendix A to H first). The build script sorted files by name, and `bm*` sorts before `ch*`. My earlier checks counted headings but not their order. | page 1 and page 6 of the first PDF | build script now orders chapters, then appendices (explicit sort key); verified by the heading sequence in the assembled Markdown and by page 1 of the corrected PDF |
| 2 | A failed PDF print left an OLD PDF in place and the script reported success (it only checked that the file existed). A viewer that has the PDF open locks it on Windows (Microsoft Edge had the first build open). | file timestamps: HTML 17:17, PDF 17:13 | script prints to a temporary file, checks it is new and non-empty, retries, and when the target is locked writes `*_rebuilt.pdf` beside it |
| 3 | The glossary's A to Z letters each appeared as a table-of-contents entry (about 25 clutter lines) | HTML table of contents | letters are bold labels, not headings |

The first build is preserved unchanged in `docs/learning/archive/v1_2026-10-30_as_delivered/` (with a manifest and a README that states defect 1).

## Observations (cosmetic, not fixed)
- Pages 6, 62 and 95 end with 30 to 55 percent white space because the next block (a figure, for example Figure 6.2) is kept whole and does not fit. Acceptable in print; fewer gaps would need smaller figures or a different layout.
- Body text is 10.5 pt, tables 9 pt, code 8.6 pt: legible at page-fit zoom on a laptop, small but normal in print.
- The title/contents page has no header or page number (by design); all other pages show the running header and a page number.

## Pages opened and read (corrected PDF)
1 (title and contents), 6 (Chapter 1 opening: heading, body, code block), 45 (list and a 4-column table), 62 (judge box, headings, bullets), 95 (callouts, judge box, white space), 96 (full-page Figure 6.2: legible, complete), 140 (44-row matrix table, two judge boxes), 165 (Appendix E entries), 172 (Appendix F four-column tables). Findings on these pages: headings, page breaks at chapter starts, table borders and column widths, code formatting, running header and page numbers, judge-box styling and fonts were consistent and readable.

## Not inspected
174 of 183 pages (including the other 32 figures and most tables); printed paper output; other PDF viewers; PDF bookmarks or accessibility tags; font embedding (checked only visually); appendix navigation beyond the table of contents (there are no clickable PDF bookmarks verified). A fuller pass is still required before acceptance.
