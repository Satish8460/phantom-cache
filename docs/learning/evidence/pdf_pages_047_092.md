Status: inspection evidence (read-only; no handbook source or CSS was changed)
Audience: handbook maintainer fixing the PDF build
Last reviewed: Fri Oct 30 17:24 IST 2026 (machine clock, via `date`; inspection run ended a few minutes later)

# PDF inspection, pages 47 to 92

## Method

- File: `docs/learning/PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf` (183 pages, A4). The real PDF was opened in headless Chrome's PDF viewer, not the HTML.
- Script: `D:\Temp\pdf_insp2.mjs`, using the repo helper `ui/scripts/cdp-lib.mjs` (its own throwaway Chrome profile). For each page N it navigated to `about:blank`, then to `file:///.../PHANTOMCACHE_ENGINEERING_HANDBOOK_rebuilt.pdf#page=N&zoom=page-fit&toolbar=0&navpanes=0`, waited 3 s, and saved a PNG. Viewport 1100x1560. Output: `D:\Temp\pdf_insp2\p047.png` to `p092.png`.
- All 46 PNGs were opened with the Read tool and looked at. No page was skipped. The lower part of each PNG shows the top of the next page; only the top page of each PNG was judged.
- White-space figures are visual estimates of the vertical share of the page body left empty below the last content (tagged: estimate). Font sizes are estimated from pixels (A4 = 790 px wide in the PNG, so about 3.76 px/mm); tag: estimate, not read from PDF font metadata.
- Running header ("PhantomCache Engineering Handbook") and the centred page number were present and correct (printed number = PDF page number) on every page 47 to 92.
- No ':::' markers, unrendered backticks, pipe-table text or garbled glyphs were seen, with the exceptions listed under "Raw Markdown artefacts" below.

## Readability judgement

- Body text is small. At page-fit on a 1100x1560 viewport the body is about 8 pt (estimate), set on lines of 150 to 175 characters. It is legible on a laptop at page-fit only with effort; it would print legibly on A4 but is dense, and the line length is well above the comfortable 75 to 90 characters.
- Inline code is about 6.5 pt, table cells about 6.5 to 7 pt, judge-box text about 7.5 pt, figure labels and figure captions about 5.5 to 6.5 pt (estimate). Figure text on pages 50, 55, 70, 76, 80, 83 and 91 is readable at page-fit but marginal; it is likely to be hard on a printed A4 page.
- Fonts were consistent (one sans body face, one mono code face, one serif-free heading style); no font inconsistency seen.

## Per-page results

Verdict key: OK = no defect beyond the global small-type note; DEFECT(type) = see detail.

| Page | Heading(s) on the page | Verdict | Defect detail |
|---|---|---|---|
| 47 | Ch 3 end: Key ideas, What you should now understand, Knowledge check, Codebase exploration, Challenge questions | OK | Challenge question 1 continues onto p48 (normal flow). Answer box text about 6.5 pt. |
| 48 | Ch 3 challenge answers (end of chapter) | DEFECT(near-blank) | About 85% blank: only 8 lines at top, then white to the footer. Next page (49) starts Chapter 4 (forced chapter break). |
| 49 | Chapter 4; What you will learn; 4.0; 4.1 Storage and lookup | OK | KeyState table fits within margins. |
| 50 | 4.1 Complexity; 4.2 Sharded mutexes; Fig 4.1; worked example | DEFECT(figure, minor) | Fig 4.1 caption line "...never wait for each other" is crossed by the arrow into the shard box (text partly struck through). Figure text small. |
| 51 | 4.2 text; Contention and its limits; Poisoned mutexes callout; Lock order; JUDGE MAY ASK | OK | About 8% blank at bottom. |
| 52 | 4.3 Concurrency; 4.4 Request coalescing (Terms) | DEFECT(near-blank) | About 42% blank (estimate). The "How it works" heading and Fig 4.2 (tall) moved whole to p53. Heading is not stranded. |
| 53 | How it works (Fig 4.2); code block; worked example | DEFECT(figure, minor) | In Fig 4.2 the dotted lifelines run through labels "2 Initiate(registration)...", "8 origin answers (one request in total)" and "12 C gets result (COALESCED)". Code block fits. |
| 54 | Trade-offs; Origin protection; two JUDGE MAY ASK boxes | OK | About 12% blank; boxes intact. |
| 55 | 4.5 Entry lifecycle; Fig 4.3 | DEFECT(near-blank, minor) | About 25% blank at the bottom because the following code block moved to p56. Fig 4.3 text small (about 5.5 pt). |
| 56 | finalize_fetch code; worked example; Cancellation; Pinning; JUDGE MAY ASK | OK | Code block is not cut off. |
| 57 | 4.6 Memory management; worked example; JUDGE MAY ASK | OK | About 17% blank. |
| 58 | 4.7 Invalidation and epochs; 3-column table; overflow fallback; sweeper | OK | Table fits. About 14% blank. |
| 59 | The known race G4A-01; Fig 4.4 | OK | Figure whole and legible (small). About 17% blank; next page begins with judge boxes. |
| 60 | JUDGE MAY ASK x2; 4.8 Freshness; default TTL not wired | OK | None. |
| 61 | Serving stale and the hard cap (code); worked example; Revalidation; Known conformance gaps | OK | About 15% blank. |
| 62 | JUDGE MAY ASK; 4.9 Eviction (Terms, three algorithms) | DEFECT(near-blank) | About 38% blank (estimate; just under the 40% threshold). Fig 4.5 plus "What is implemented" moved whole to p63. |
| 63 | What is implemented (Fig 4.5); complexity table; worked example; Where the policy is told about hits | OK | 4-column table fits. |
| 64 | Switching policy; S3-FIFO finding; Tests; two JUDGE MAY ASK | OK | None. |
| 65 | 4.10 Origin fails (table); 4.11 Capability status table (starts) | OK | Table 4.11 splits across pages with the header repeated on p66 (acceptable). |
| 66 | 4.11 (cont.); 4.12; Key ideas; What you should now understand; Knowledge check | OK | None. |
| 67 | Codebase exploration; Challenge questions; Answers to the knowledge check | DEFECT(markdown artefact; blank) | Answer 9 renders as "9.   1. Leader passes the epoch fence check..." (a numbered sub-list nested inside list item 9, number printed twice). About 30% blank after, followed by the Chapter 5 break (expected). |
| 68 | Chapter 5; What you will learn; 5.1; table "Finding in the 2026-10-28 investigation" (start) | DEFECT(table split, minor) | The table breaks after its first row (A1) at the page bottom; remaining 8 rows plus repeated header are on p69. An orphaned single row. |
| 69 | Table (cont.); Trade-offs; 5.2 Cluster membership; Node states; Heartbeats | OK | Tables fit. |
| 70 | The wire protocol; Admission: the mTLS handshake; Fig 5.1 | DEFECT(figure, minor; blank) | About 31% blank at the bottom (Fig 5.2 moved to p71). In Fig 5.1 the line under "full mesh of mTLS" runs through the sentence "frames carried: InvalidateRequest/Ack..." ("fram|es", "sen|ds"). Figure text about 5.5 pt. |
| 71 | Fig 5.2 (handshake sequence); Key points; non-mTLS configurations | DEFECT(figure, minor) | The dotted lifeline crosses "(max 8 per peer)" in step 1 and "Admission is checked at handshake..." at the bottom. About 20% blank. |
| 72 | JUDGE MAY ASK x2; 5.3 Rendezvous hashing; The maths | OK | About 12% blank. |
| 73 | Worked example (real numbers) table and replica box; What the code actually does | OK | Table and boxed figure fit; legible. |
| 74 | Which operations are routed (table); Alternatives; JUDGE MAY ASK x2 | OK | Table fits. Trailing section rule near the bottom. |
| 75 | 5.4 Epochs and versioning (table); worked example; What epochs do across nodes; JUDGE MAY ASK | OK | 5-column table fits. |
| 76 | 5.5 Invalidation propagation; Fig 5.4; numbered steps 1 to 7 | DEFECT(figure, minor) | The figure's last line ("Source: router.rs (invalidate_key_across_cluster...") sits on the dashed bottom border of the box. Figure text about 5.5 pt. |
| 77 | Step 8; Anti-entropy; Status table; JUDGE MAY ASK | OK | About 15% blank. |
| 78 | JUDGE MAY ASK; 5.6 Consistency; failure-model table; G1, G2 | OK | Table fits. |
| 79 | G3; The honest statement; JUDGE MAY ASK x2 | DEFECT(near-blank) | About 33% blank (estimate) after a trailing section rule; Fig 5.5 (5.7) moved whole to p80. |
| 80 | 5.7 Node failure (Fig 5.5 panels A, B, C); A receiver node goes down; B partition | OK | Figure whole; small text. About 9% blank. |
| 81 | C. The issuer crashes; Cross-node concurrency; JUDGE MAY ASK x2; 5.8 starts | OK | None. |
| 82 | 5.8 (cont.); DR-027; JUDGE MAY ASK; 5.9 durability table | OK | Table fits; the callout starts on p83 top (a separate red box, not split). |
| 83 | 5.10 Workload intelligence; Fig 5.6; component table (start) | OK | Fig 5.6 legible but small (about 5.5 pt); vertical rotated text "BOUNDARY: no wire between the two sides" readable. Table splits to p84 with header repeated. |
| 84 | Table (cont.); 5.11 Telemetry; Error bound; worked example; A discrepancy you must know | OK | None. |
| 85 | JUDGE MAY ASK; 5.12 Probabilistic structures; Count-Min; Space-Saving; Where hot keys are handled | OK | About 8% blank. |
| 86 | JUDGE MAY ASK; 5.13 The adaptive algorithms (A AIMD, B Shadow evaluation) | DEFECT(markdown artefact) | "Findings that apply." paragraph shows raw list dashes run together: `- "Canary" is not a canary ... - The "p99" is the window maximum ... - It can behave as a static cap.` (a Markdown list not separated by a blank line, so it rendered as one paragraph with literal " - "). |
| 87 | C. Decision engine; D. Executor; E. Legacy PIE stub; 5.14 The M6 result | OK | About 16% blank. |
| 88 | JUDGE MAY ASK x2; 5.15 Chapter status summary (table) | OK | 3-column table fits. |
| 89 | Key ideas to remember; What you should now understand; Knowledge check; Codebase exploration; Challenge questions | DEFECT(stray rule, minor) | A lone horizontal rule sits at the very top of the page above the heading (section separator carried over). Challenge 2 continues onto p90. |
| 90 | Challenge 2 end; Answers to the knowledge check | DEFECT(near-blank; markdown artefact) | About 60% blank; Chapter 6 starts on p91 (forced chapter break). Answer 3 renders as "3.   a. Local tenant epoch..." (nested sub-list marker). |
| 91 | Chapter 6; What you will learn; 6.1 Threat model; Fig 6.1 | DEFECT(figure overflow, minor) | In Fig 6.1 some lines run past their box borders: "GET/DELETE /api/v1/cache/*key (JWT cache:read / cache:invalidate)" crosses the right border of the ingress box; "Cluster listener + router (only with mTLS cluster config)" and "pinned address, 5 s timeouts," touch/cross their boxes. Figure text about 5.5 pt. |
| 92 | Where it lives; Adversaries; 6.2 Authentication; worked example; What is not done (table) | OK | Table fits. |

## Counts

- Pages inspected: 46 of 46 (47 to 92). Pages that could not be inspected: none.
- Pages OK: 30 (47, 49, 51, 54, 56 to 61, 63 to 66, 69, 72 to 75, 77, 78, 80 to 85, 87, 88, 92).
- Pages with at least one defect: 16 (48, 50, 52, 53, 55, 62, 67, 68, 70, 71, 76, 79, 86, 89, 90, 91).
- By defect type (a page may have more than one):
  - Near-blank over 40%: 3 (48, 52, 90). Between 25% and 40%: 5 (55, 62, 67, 70, 79).
  - Figure text crossed by a line or running past its box: 7 (50, 53, 70, 71, 76, 91; 70 and 71 counted once each; total distinct pages 6).
  - Raw Markdown or pandoc artefact: 4 pages (67 nested numbering, 86 inline dashes, 90 nested numbering, 89 stray rule).
  - Table split leaving an orphaned row: 1 (68).
  - Table cut off at the right edge or overflowing: 0.
  - Code line cut off or overflowing: 0.
  - Heading stranded at the bottom of a page: 0.
  - Judge box or callout split across pages: 0 (long boxes were moved whole to the next page).
  - Missing or wrong running header or page number: 0.
  - Garbled characters: 0.
  - Wrong content order: 0.

## Near-blank pages (white-space estimate, vertical)

| Page | Blank | What follows |
|---|---|---|
| 48 | about 85% | p49: Chapter 4 title page (chapter break) |
| 90 | about 60% | p91: Chapter 6 (chapter break) |
| 52 | about 42% | p53: "How it works" + Fig 4.2 (figure did not fit) |
| 62 | about 38% | p63: "What is implemented" + Fig 4.5 |
| 79 | about 33% | p80: 5.7 Fig 5.5 |
| 70 | about 31% | p71: Fig 5.2 |
| 67 | about 30% | p68: Chapter 5 (chapter break) |
| 55 | about 25% | p56: finalize_fetch code block |

Blank space before a chapter heading (48, 67, 90) is expected from page-break-before on chapters; the others (52, 55, 62, 70, 79) come from tall unbreakable figures or code blocks and are the avoidable ones.

## Recommended fixes

1. Tall figures causing 25 to 42% gaps (52, 55, 62, 70, 79): in the print CSS used for the Chrome build (the stylesheet passed to pandoc/Chrome in the handbook build; the repo's handbook build script was not read for this pass), allow the heading to stay with the figure but shrink the figures. Add `figure svg, figure img { max-height: 150mm; width: auto; }` and `figure { break-inside: avoid; margin: 0.6em 0; }` so more figures fit under the preceding text. Alternatively reorder the Markdown in `PHANTOMCACHE_ENGINEERING_HANDBOOK.md` so a figure follows a short paragraph instead of a long Terms list (sections 4.4, 4.5, 4.9, 5.2, 5.7).
2. Chapter-end near-blank pages (48, 67, 90): acceptable; if a shorter book is wanted, use `h1 { break-before: page }` only (not `break-after`) and avoid a trailing `<hr>`; consider trimming the model answers so the chapter ends nearer a page boundary.
3. Nested numbering in "Answers to the knowledge check" (p67 answer 9, p90 answer 3; also check the same answers in chapters 4 and 5 source): in the Markdown, those answers contain an inline "(1) ... (2) ..." or "a. ... b. ..." that pandoc turned into an ordered sub-list. In the answer list, escape the marker (for example `9\. ` is not needed; write "(1) Leader passes..." and "(a) Local tenant epoch ..." with parentheses, or put them in a sentence). Locate by searching the source for the answer text "Leader passes the epoch fence check" (ch 4) and "Local tenant epoch and global epoch in the cache" (ch 5).
4. Inline dashes on p86: in the source, the paragraph beginning "Findings that apply." needs a blank line before the first `- ` item (and between it and the lead-in sentence) so pandoc emits a real bullet list. Search for "Findings that apply" in `PHANTOMCACHE_ENGINEERING_HANDBOOK.md` (chapter 5, section 5.13 A).
5. Figures with lines crossing text (Fig 4.1, 4.2, 5.1, 5.2, 5.4, 6.1): these are SVGs inside the Markdown or its figure files. Fix in the SVG source: move the offending labels off the lifelines or add a white `paint-order: stroke; stroke: #fff; stroke-width: 3px` halo to label text so lines do not strike through it; widen the Fig 6.1 ingress and cluster boxes (or wrap those two labels onto two lines) so text stays inside the borders; move Fig 5.4's "Source:" line below the dashed box.
6. Orphaned single table row (p68): add `tr { break-inside: avoid; }` and `table { break-before: auto; }`, and `thead { display: table-header-group; }`; to keep this table whole add `table { break-inside: avoid; }` for tables of 10 rows or fewer (this would also keep the 5.4 and 4.10 tables intact).
7. Stray horizontal rules (p79 trailing, p89 leading): remove the `---` separators before "Key ideas to remember" in the source, or add `hr { break-before: avoid; break-after: avoid; }` and hide an `hr` that immediately precedes `h2`/`h3` (`hr + h2 { margin-top: 0 }` and hide the rule in print).
8. Readability: raise the base size from about 8 pt to 9.5 to 10 pt and cap the text block at about 150 mm (larger left and right margins) so line length drops to about 90 to 100 characters; set code and table text to at least 7.5 pt; raise figure font sizes in the SVGs to at least 7 pt at A4. This will add pages; recheck blank-space after the change.

## Limits

- Judgements were made from page-fit PNGs; sub-pixel clipping inside figures could be missed. Font sizes are estimates from pixel measurements.
- Source and CSS locations in the recommendations are inferred from visible symptoms; the build script and CSS file were not opened in this pass.
