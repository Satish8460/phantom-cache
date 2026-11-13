// Classification + routing logic shared by analyze_sections.mjs (writes layer_map.json + the analysis) and build_two_layer.mjs.
// READ-ONLY on docs/learning/src. Categories: A essential, B detailed reference, C repeated/redundant, D excessive judge follow-up,
// E discrepancy/evidence inventory. Classification is by judgement from headings, element statistics and sampled reading (see the analysis).
import { listFiles, parseFile, splitItems, judgeParts, wc } from './parse_lib.mjs';

export const SCAFFOLD = /^(what-you-will-learn|key-ideas-to-remember|what-you-should-now-understand|knowledge-check|answers-to-the-knowledge-check|codebase-exploration-tasks|challenge-questions)$/;

// Whole-section moves to the Reference layer: id -> [category, group, one-line rationale]
export const REF = {
  'ch01:1.10': ['E', 'R5', 'Discrepancy inventory (document vs code) and positioning history; table of corrections'],
  'ch02:2.11': ['E', 'R5', 'Per-chapter discrepancy list'],
  'ch03:3.1': ['B', 'R1', 'Dependency table (crate / version / purpose); the stack figure in 3.0 stays in Part I'],
  'ch03:3.6': ['B', 'R1', 'Table of data structures with file locations; the key ones are taught in Ch 4'],
  'ch03:3.13': ['E', 'R5', 'Supply-chain observation list (evidence inventory)'],
  'ch04:4.0': ['C', 'R6', 'Restates the status labels already defined in the front matter and in G.1'],
  'ch04:4.11': ['C', 'R1', 'Capability status table; the same capabilities are labelled again in 8.12 and App G'],
  'ch04:4.12': ['E', 'R5', 'Per-chapter discrepancy list'],
  'ch05:5.15': ['C', 'R1', 'Chapter status summary; the same capabilities are labelled again in 8.12 and App G'],
  'ch06:6.11': ['B', 'R1', 'Full threat / failure-scenario matrix (about 1,430 table words); 6.2 to 6.10 teach the protections in prose'],
  'ch06:6.13': ['E', 'R5', 'Inventory of every measurement with profile, host and limits; 6.12 teaches how to evaluate'],
  'ch07:7.10': ['E', 'R5', 'Per-chapter discrepancy list'],
  'ch08:8.4': ['E', 'R5', 'Register of defects by theme (99 findings); 8.3 and 8.5 carry the essential limitations'],
  'ch08:8.7': ['E', 'R5', 'Debt and out-of-date statements in project documents (inventory)'],
  'ch08:8.10': ['B', 'R1', 'Production-maturity checklist (reference table)'],
  'ch08:8.12': ['C', 'R1', 'Matrix whose rows are carried cell-for-cell in Appendix G (true duplicate; see analysis)'],
  appB: ['C', 'R6', 'Architecture re-explained a second time; Ch 2, 4 and 5 own the explanation'],
  appC: ['B', 'R1', 'Technology and algorithm reference tables'],
  appD: ['B', 'R3', 'Glossary (about 5,100 words)'],
  appE: ['C', 'R4', 'Consolidated judge bank: restates the 93 chapter judge boxes in condensed form'],
  appF: ['B', 'R2', 'Source-file map'],
  appG: ['C', 'R1', 'Implementation-versus-roadmap matrix (superset of 8.12)'],
};
// Sections that stay in Part I but whose pipe tables of >= 100 words move to Reference R1 (prose, figures and examples stay).
export const SPLIT_TABLES = {
  'ch01:1.6': ['C', 'status-by-level table repeats the labels of 8.12 / App G'],
  'ch02:2.3': ['B', 'full route and header tables'],
  'ch02:2.9': ['B', 'full configuration-key table'],
  'ch03:3.10': ['B', 'build / packaging inventory table'],
  'ch05:5.10': ['B', 'inventory table of intelligence modules and where they run'],
  'ch08:8.2': ['E', 'repository and test facts table (time-sensitive evidence inventory)'],
};
export const SPLIT_MIN_WORDS = 100;
// Judge boxes kept in Part I (full box). All others move whole to Reference R4 under the same section heading.
export const KEY_JUDGE = [
  'ch01:1.4#1', 'ch01:1.5#1',
  'ch02:2.2#1', 'ch02:2.6#2',
  'ch03:3.2#1', 'ch03:3.5#1',
  'ch04:4.5#1', 'ch04:4.7#2', 'ch04:4.9#1',
  'ch05:5.5#1', 'ch05:5.6#2', 'ch05:5.7#2', 'ch05:5.14#1', 'ch05:5.14#2',
  'ch06:6.4#1', 'ch06:6.9#2',
  'ch07:7.9#1', 'ch07:7.9#5', 'ch07:7.9#12',
  'ch08:8.13#1', 'ch08:8.13#2', 'ch08:8.13#7', 'ch08:8.13#8',
];
// True duplicates proposed for removal (applied only with --apply-dedup): pipe tables of the key are cell-for-cell contained in `of`.
export const DEDUPE = { 'ch08:8.12': { of: 'appG', kind: 'tables' } };

export const GROUPS = {
  R1: 'R1. Reference matrices, route and configuration tables',
  R2: 'R2. Source-file map',
  R3: 'R3. Glossary and abbreviations',
  R4: 'R4. Judge-question bank',
  R5: 'R5. Discrepancy and evidence inventories',
  R6: 'R6. Restated explanations (kept for reference)',
};

// Contiguous pipe-table runs outside code fences: [{start,end,words}] (end exclusive)
export function pipeTableBlocks(lines) {
  const out = []; let fence = false, s = -1;
  lines.forEach((ln, i) => {
    if (/^```/.test(ln)) fence = !fence;
    const t = !fence && /^\|.*\|\s*$/.test(ln);
    if (t && s < 0) s = i;
    if (!t && s >= 0) { out.push({ start: s, end: i }); s = -1; }
  });
  if (s >= 0) out.push({ start: s, end: lines.length });
  return out.map((b) => ({ ...b, words: wc(lines.slice(b.start, b.end).join('\n')) }));
}

export function loadUnits() {
  const units = [];
  for (const f of listFiles()) for (const u of parseFile(f)) {
    const items = splitItems(u.lines);
    let k = 0;
    for (const it of items) if (it.type === 'judge') {
      k++; it.jid = `${u.id}#${k}`; it.parts = judgeParts(it.text);
      it.question = ((it.text.match(/Question:\s*([^\n]*)/) || [])[1] || '').trim(); it.key = KEY_JUDGE.includes(it.jid);
    }
    units.push({ ...u, key: u.id, items });
  }
  return units;
}

// Classify one unit (h1 or h2 section). Returns {cat, layer, group, why, split}
export function classify(u) {
  const tag = u.tag; const id = u.key;
  const rk = REF[id] ? id : (tag.startsWith('app') && REF[tag] ? tag : null); // appendix h2 sections inherit the appendix
  if (rk) return { cat: REF[rk][0], layer: 'reference', group: REF[rk][1], why: u.level === 2 && rk !== id ? 'Part of ' + rk + ': ' + REF[rk][2] : REF[rk][2] };
  if (u.level === 1) {
    if (tag === 'appA') return { cat: 'A', layer: 'learning', group: '', why: 'One-page overview: the first thing to read' };
    if (tag === 'appH') return { cat: 'A', layer: 'learning', group: '', why: 'Ten essential concepts (restated as a learning check); H.1 to H.11 below' };
    return { cat: 'A', layer: 'learning', group: '', why: 'Chapter title and introduction' };
  }
  const slug = id.split(':')[1];
  if (SCAFFOLD.test(slug)) return { cat: 'A', layer: 'learning', group: '', why: slug === 'what-you-will-learn' ? 'Chapter objectives (learning scaffold)' : 'Learning scaffold: belongs with the teaching, not the reference' };
  if (tag === 'appH') return { cat: 'A', layer: 'learning', group: '', why: 'One of the ten concepts, explained without help' };
  if (id === 'ch07:7.9' || id === 'ch08:8.13') return { cat: 'D', layer: 'learning', group: '', why: 'Judge-box section: key boxes stay in Part I, the rest move whole to R4 under the same heading' };
  const st = SPLIT_TABLES[id];
  if (st) return { cat: 'A', layer: 'learning', group: '', why: `Teaching section; ${st[1]} moves to R1 (${st[0]})`, split: true };
  return { cat: 'A', layer: 'learning', group: '', why: 'Teaching section (explanation, how it works, where in code, trade-offs, what can go wrong)' };
}
