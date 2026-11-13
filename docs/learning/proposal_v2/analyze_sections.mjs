// Classifies every section of the handbook sources, writes proposal_v2/layer_map.json and docs/learning/evidence/two_layer_analysis.md.
//   node docs/learning/proposal_v2/analyze_sections.mjs
// READ-ONLY on docs/learning/src. Runs build_two_layer.mjs --no-pdf to obtain the Part I / Part II outputs and the manifest.
import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { SRC, ROOT, HERE, wc, listFiles } from './parse_lib.mjs';
import { SCAFFOLD, loadUnits, classify, GROUPS, SPLIT_TABLES, SPLIT_MIN_WORDS, KEY_JUDGE, DEDUPE, REF, pipeTableBlocks } from './layers_lib.mjs';

const OUTMD = join(ROOT, 'evidence', 'two_layer_analysis.md');
const units = loadUnits();
const info = {}; for (const u of units) info[u.key] = classify(u);
const esc = (s) => String(s).replace(/\|/g, '/');
const fmt = (n) => Math.round(n).toLocaleString('en-GB');

// ---------- layer_map.json ----------
const sections = {}, groups = {};
for (const u of units) { sections[u.key] = info[u.key].layer; if (info[u.key].group) groups[u.key] = info[u.key].group; }
const today = new Date().toISOString().slice(0, 10);
writeFileSync(join(HERE, 'layer_map.json'), JSON.stringify({
  _meta: { generated: today, by: 'analyze_sections.mjs', note: 'PROPOSAL. Section id -> layer. Ids: chNN (chapter title and intro), chNN:X.Y (h2 section), chNN:slug (unnumbered h2), appX (appendix h1 incl. its h2s unless appX:Y is listed). learning = Part I, reference = Part II. split_tables: sections that stay in Part I but whose pipe tables >= ' + SPLIT_MIN_WORDS + ' words move to Part II R1. key_judge_boxes: judge boxes kept in Part I (id = section id + #n, n counted in source order); every other judge box moves whole to Part II R4 under the same section heading. Nothing is deleted.' },
  sections, groups, group_titles: GROUPS, split_tables: Object.keys(SPLIT_TABLES), key_judge_boxes: KEY_JUDGE, dedupe_candidates: DEDUPE,
}, null, 1) + '\n');

// ---------- run the builder (no PDF) ----------
const b = spawnSync(process.execPath, [join(HERE, 'build_two_layer.mjs'), '--no-pdf'], { encoding: 'utf8' });
if (b.status !== 0) { console.error(b.stdout, b.stderr); process.exit(1); }
const man = JSON.parse(readFileSync(join(HERE, 'out', 'manifest.json'), 'utf8'));
const R = man.result;
const stripGen = (s) => s.replace(/^---\n[\s\S]*?\n---\n/, '').split('\n').filter((l) => !l.includes('<!--gen-->')).join('\n');
const contentWs = { I: wc(stripGen(readFileSync(join(HERE, 'out', 'learning.md'), 'utf8'))), II: wc(stripGen(readFileSync(join(HERE, 'out', 'reference.md'), 'utf8'))) };

// ---------- word allocation by category and layer ----------
const front = readFileSync(join(SRC, '00_FRONT.md'), 'utf8').replace(/\r\n/g, '\n').replace(/^---\n[\s\S]*?\n---\n/, '').trim();
const frontWords = wc(front);
const cats = { A: 0, B: 0, C: 0, D: 0, E: 0 }; const layerW = { learning: 0, reference: 0 };
const rows = [];
const addW = (c, l, w) => { cats[c] += w; layerW[l] += w; };
addW('A', 'learning', frontWords);
for (const u of units) {
  const inf = info[u.key]; let own = wc(u.heading); let stay = own; let out = 0; const a = { A: 0, B: 0, C: 0, D: 0, E: 0 };
  const L = inf.layer;
  { const hc = inf.cat === 'D' ? 'A' : inf.cat; a[hc] += own; addW(hc, L, own); }
  let jMoved = 0, tMoved = 0;
  for (const it of u.items) {
    if (it.type === 'judge') {
      const p = it.parts; const base = p.total - p.further - p.evidence; const jl = it.key ? 'learning' : 'reference';
      addW(it.key ? 'A' : 'B', jl, base); addW('D', jl, p.further); addW('B', jl, p.evidence); a[it.key ? 'A' : 'B'] += base; a.D += p.further; a.B += p.evidence;
      if (jl === 'learning') stay += p.total; else { jMoved += p.total; }
      continue;
    }
    const lines = it.text.split('\n');
    let tw = 0;
    if (inf.split) { for (const bl of pipeTableBlocks(lines)) if (bl.words >= SPLIT_MIN_WORDS) tw += wc(lines.slice(bl.start, bl.end).join('\n')); }
    const w = wc(it.text);
    if (tw) { const tc = SPLIT_TABLES[u.key][0]; addW(tc, 'reference', tw); a[tc] += tw; tMoved += tw; addW('A', 'learning', w - tw); a.A += w - tw; stay += w - tw; }
    else { const c = inf.cat === 'D' ? 'A' : inf.cat; addW(c, L, w); a[c] += w; if (L === 'learning') stay += w; }
  }
  if (L === 'learning') stay = stay; else stay = 0;
  // 'own' words of the h2 including judge boxes etc.
  const total = wc(u.heading) + wc(u.lines.join('\n'));
  rows.push({ u, inf, total, stay: L === 'learning' ? stay : 0, jMoved, tMoved, a });
}
// the hand-computed allocation above double counts headings for reference-layer judge boxes? verify below
const sumCats = Object.values(cats).reduce((x, y) => x + y, 0), sumLayers = layerW.learning + layerW.reference;
const TOTAL = frontWords + rows.reduce((x, r) => x + r.total, 0);
const PAGES = 183, LEAD_WORDS = 131172;
const pg = (w) => (w * PAGES / TOTAL);
const chapName = (tag) => tag.startsWith('ch') ? 'Ch ' + +tag.slice(2) : 'App ' + tag.slice(3);

// ---------- element measures ----------
const J = {};
for (const u of units) for (const it of u.items) if (it.type === 'judge') {
  const t = u.tag.startsWith('ch') ? u.tag : 'app'; const p = it.parts; J[t] ??= { n: 0, q: 0, ans: 0, further: 0, ev: 0, lab: 0, tot: 0 };
  const x = J[t]; x.n++; x.q += p.q; x.ans += p.answer; x.further += p.further; x.ev += p.evidence; x.tot += p.total; x.lab += p.total - p.q - p.answer - p.further - p.evidence;
}
const jt = Object.values(J).reduce((s, x) => { for (const k in x) s[k] = (s[k] || 0) + x[k]; return s; }, {});
const jrows = Object.entries(J).map(([t, x]) => `| ${chapName(t)} | ${x.n} | ${x.q} | ${x.ans} | ${x.further} | ${x.ev} | ${x.lab} | ${x.tot} | ${(100 * x.further / x.tot).toFixed(0)}% |`);
jrows.push(`| **Total** | ${jt.n} | ${jt.q} | ${jt.ans} | ${jt.further} | ${jt.ev} | ${jt.lab} | ${jt.tot} | ${(100 * jt.further / jt.tot).toFixed(0)}% |`);
const keyTot = rows.flatMap((r) => r.u.items.filter((i) => i.type === 'judge' && i.key)).reduce((s, i) => s + i.parts.total, 0);
const movedTot = rows.flatMap((r) => r.u.items.filter((i) => i.type === 'judge' && !i.key)).reduce((s, i) => s + i.parts.total, 0);

// element types (all chapters + appendices) by whole-book words
const el = { table: 0, figure: 0, code: 0, example: 0, caution: 0, concept: 0, answers: 0 };
for (const u of units) { let fence = false, div = null; for (const it of u.items) if (it.type === 'text') for (const ln of it.text.split('\n')) {
  if (/^```/.test(ln)) { fence = !fence; el.code += wc(ln); continue; } if (fence) { el.code += wc(ln); continue; }
  const d = ln.match(/^::: *(\w+)/); if (d) { div = d[1]; continue; } if (/^:::\s*$/.test(ln)) { div = null; continue; }
  const w = wc(ln); if (/^\|/.test(ln)) el.table += w; else if (/^!\[/.test(ln)) el.figure += w; else if (div && el[div] !== undefined) el[div] += w; } }
const figCount = units.reduce((s, u) => s + (u.lines.join('\n').match(/^!\[/gm) || []).length, 0);

// ---------- App E structure ----------
const appE = units.filter((u) => u.tag === 'appE');
const eEntries = []; { const txt = appE.map((u) => u.lines.join('\n')).join('\n'); const parts = txt.split(/\n(?=### [A-Z]{3}-\d+\.)/).slice(1);
  for (const p of parts) { const h = p.split('\n')[0]; const ans = (p.match(/\*\*Answer\.\*\*([\s\S]*?)\n\n/) || [])[1] || ''; const ev = (p.match(/\*\*Where taught:\*\*([\s\S]*)$/) || [])[1] || ''; eEntries.push({ id: h.match(/### ([A-Z]{3}-\d+)/)[1], q: h.replace(/^### [A-Z]{3}-\d+\.\s*/, ''), words: wc(p), ans: wc(ans), ev: wc(ev) }); } }
const eAns = eEntries.reduce((s, e) => s + e.ans, 0), eEv = eEntries.reduce((s, e) => s + e.ev, 0), eAll = eEntries.reduce((s, e) => s + e.words, 0);
const stop = new Set('a an the is are do does did what how why which who can could would should you your we our of in on to for and or not it its this that with by at from be as if so than then there have has had will may whether any one'.split(' '));
const tok = (s) => new Set(s.toLowerCase().replace(/[^a-z0-9 ]/g, ' ').split(/\s+/).filter((w) => w && !stop.has(w)));
const nearQ = []; for (let i = 0; i < eEntries.length; i++) for (let j = i + 1; j < eEntries.length; j++) { const a = tok(eEntries[i].q), c = tok(eEntries[j].q); const inter = [...a].filter((x) => c.has(x)).length; const jac = inter / (a.size + c.size - inter); if (jac >= 0.34) nearQ.push([eEntries[i].id, eEntries[j].id, jac.toFixed(2), eEntries[i].q.slice(0, 70), eEntries[j].q.slice(0, 70)]); }

// ---------- duplicates ----------
const normL = (s) => s.toLowerCase().replace(/[`*_>#]/g, '').replace(/\s+/g, ' ').trim();
const lineMap = new Map(); for (const u of units) for (const ln of u.lines) { const n = normL(ln); if (wc(n) < 10 || /^\|[\s:|-]+\|$/.test(n)) continue; (lineMap.get(n) || lineMap.set(n, []).get(n)).push(u.key); }
const dupLines = [...lineMap].filter(([, ids]) => ids.length > 1);
const cellsOf = (u) => u.lines.filter((l) => /^\|.*\|\s*$/.test(l) && !/^\|[\s:|-]+\|\s*$/.test(l)).map((l) => l.slice(1, -1).split('|').map(normL));
const byKey = Object.fromEntries(units.map((u) => [u.key, u]));
const G = ['appG:G.2', 'appG:G.3', 'appG:G.4'].flatMap((k) => cellsOf(byKey[k])).flat(); const Gset = new Set(G);
const r812 = cellsOf(byKey['ch08:8.12']).slice(1); // skip header row
const hdr812 = r812.length;
let contained = 0, cw = 0, aw = 0; const notC = [];
for (const r of r812) { const w = wc(r.join(' ')); aw += w; if (r.every((c) => !c || Gset.has(c))) { contained++; cw += w; } else notC.push(r[0]); }
// cell-level check of 8.11 vs "Never say"
const ns = cellsOf(byKey['appE:never-say']).slice(1), e811 = cellsOf(byKey['ch08:8.11']).slice(1); const nsSet = new Set(ns.flat());
let ns811 = 0; for (const r of e811) if (r.every((c) => !c || nsSet.has(c))) ns811++;
// App E sentence overlap with chapter judge boxes
const sents = (t) => t.replace(/\n+/g, ' ').split(/(?<=[.?!])\s+/).map(normL).filter((s) => wc(s) >= 8);
const chapJ = new Set(); for (const u of units) if (u.tag.startsWith('ch')) for (const it of u.items) if (it.type === 'judge') sents(it.text).forEach((s) => chapJ.add(s));
let eSent = 0, eDup = 0, eDupW = 0, eW = 0; for (const u of appE) for (const s of sents(u.lines.join('\n'))) { eSent++; eW += wc(s); if (chapJ.has(s)) { eDup++; eDupW += wc(s); } }
// 8-word shingle coverage of a unit by other files
const K = 8; const toks = (s) => normL(s).replace(/[|`*_>#]/g, ' ').replace(/[^a-z0-9\s]/g, ' ').split(/\s+/).filter(Boolean);
const idx = new Map(); const ut = units.map((u) => toks(u.lines.join('\n')));
units.forEach((u, i) => { const t = ut[i]; for (let j = 0; j + K <= t.length; j++) { const s = t.slice(j, j + K).join(' '); (idx.get(s) || idx.set(s, new Set()).get(s)).add(u.tag); } });
const cover = (key) => { const i = units.findIndex((u) => u.key === key); const t = ut[i]; const cov = new Array(t.length).fill(false); for (let j = 0; j + K <= t.length; j++) { const s = t.slice(j, j + K).join(' '); const m = idx.get(s); if ([...m].some((x) => x !== units[i].tag)) for (let q = j; q < j + K; q++) cov[q] = true; } return [t.length, cov.filter(Boolean).length]; };
const shingleRows = ['appE:1-fundamentals', 'appE:2-architecture', 'appE:3-technology', 'appE:4-algorithms', 'appE:5-distributed-systems', 'appE:6-security', 'appE:7-innovation', 'appE:8-evaluation', 'appE:9-limitations-and-honesty', 'appE:never-say', 'ch08:8.12', 'appG:G.2', 'appG:G.3', 'appG:G.4', 'ch08:8.11', 'ch07:7.9', 'ch08:8.13', 'appB:B.6', 'appB:B.8', 'ch04:4.11', 'ch05:5.15', 'appA']
  .map((k) => { const [n, c] = cover(k); return `| ${k} | ${n} | ${c} | ${(100 * c / n).toFixed(0)}% |`; });
const lineNo = (file, re) => { const L = readFileSync(join(SRC, file), 'utf8').split(/\r?\n/); const i = L.findIndex((l) => re.test(l)); return i + 1; };
const f8 = readdirSync(SRC).find((f) => f.startsWith('ch08')), fG = readdirSync(SRC).find((f) => f.startsWith('bm07'));
const l812 = lineNo(f8, /^## 8\.12 /), lG2 = lineNo(fG, /^## G\.2 /), l811 = lineNo(f8, /^## 8\.11 /);
const sample812 = byKey['ch08:8.12'].lines.filter((l) => /^\|.*\|\s*$/.test(l)).slice(2, 5);
const sampleG = byKey['appG:G.2'].lines.filter((l) => /^\|.*\|\s*$/.test(l)).slice(2, 5);

// ---------- cross references ----------
const ptr = R.partI_pointers_approx; const topT = Object.entries(ptr.targetsRef).sort((a, b) => b[1] - a[1]).slice(0, 12).map(([k, v]) => `${k} (${v})`).join(', ');
let whereTaught = 0; for (const u of appE) whereTaught += (u.lines.join('\n').match(/\*\*Where taught:\*\*/g) || []).length;

// ---------- tables ----------
const secRows = [];
const fh = front.split('\n'); { let cur = null; const fr = []; let fence = false; for (const ln of fh) { const m = !fence && ln.match(/^(#{1,2}) (.+)$/); if (/^```/.test(ln)) fence = !fence; if (m) { cur = { level: m[1].length, heading: m[2], w: wc(m[2]) }; fr.push(cur); } else if (cur) cur.w += wc(ln); }
  fr.forEach((f, i) => secRows.push(`| front:F${i} | Front | ${esc(f.heading)} | ${f.w} | A | learning (Part I) | ${f.heading.startsWith('How the chapters') ? 'Orientation; its chapter list must be rewritten for two layers (generated note added in the prototype)' : 'Front matter: reading routes, status labels, scope'} |`)); }
const WHERE = (r) => r.inf.layer === 'reference' ? `reference (Part II ${r.inf.group})` : 'learning (Part I)';
for (const r of rows) {
  const extra = []; if (r.jMoved) extra.push(`${fmt(r.jMoved)} w of judge boxes move to R4`); if (r.tMoved) extra.push(`${fmt(r.tMoved)} w of tables move to R1`);
  const keep = r.u.items.filter((i) => i.type === 'judge' && i.key).length; if (keep) extra.push(`${keep} key judge box${keep > 1 ? 'es' : ''} stay`);
  secRows.push(`| ${r.u.key} | ${chapName(r.u.tag)} | ${r.u.level === 1 ? '**' + esc(r.u.heading) + '**' : esc(r.u.heading)} | ${r.total} | ${r.inf.cat} | ${WHERE(r)} | ${esc(r.inf.why)}${extra.length ? ' (' + extra.join('; ') + ')' : ''} |`);
}
// h3 table
const h3Rows = []; let h3n = 0;
for (const u of units) { let fence = false, div = 0; let cur = null; const arr = [];
  for (const ln of u.lines) { if (/^```/.test(ln)) fence = !fence; if (!fence && /^::: *\w/.test(ln)) div++; else if (!fence && /^:::\s*$/.test(ln)) div = Math.max(0, div - 1);
    const m = !fence && div === 0 && ln.match(/^### (.+)$/); if (m) { cur = { h: m[1], w: wc(m[1]) }; arr.push(cur); } else if (cur) cur.w += wc(ln); }
  for (const h of arr) { h3n++; h3Rows.push(`| ${u.key} | ${esc(h.h.slice(0, 80))} | ${h.w} | ${info[u.key].cat === 'D' ? 'D' : info[u.key].cat} | ${info[u.key].layer === 'reference' ? 'Part II' : 'Part I'} |`); } }

// ---------- group stats for the reference layer ----------
const grp = {}; for (const r of rows) if (r.inf.layer === 'reference') { const g = r.inf.group; grp[g] = grp[g] || { n: 0, w: 0 }; grp[g].n++; grp[g].w += r.total; }
const moveList = rows.filter((r) => r.inf.layer === 'reference').map((r) => `| ${r.u.key} | ${esc(r.u.heading)} | ${r.total} | ${r.inf.cat} | ${r.inf.group} |`);
const splitList = rows.filter((r) => r.tMoved).map((r) => `| ${r.u.key} | ${esc(r.u.heading)} | ${r.tMoved} | ${SPLIT_TABLES[r.u.key][0]} |`);
const movedBoxes = man.moved_judge.map((m) => `| ${m.id} | ${esc(m.question.slice(0, 90))} |`);
const keptBoxes = rows.flatMap((r) => r.u.items.filter((i) => i.type === 'judge' && i.key).map((i) => `| ${i.jid} | ${esc(i.question.slice(0, 90))} | ${i.parts.total} |`));

const wI = layerW.learning, wII = layerW.reference;
const compScaff = rows.filter((r) => r.inf.layer === 'learning' && r.u.level === 2 && SCAFFOLD.test(r.u.key.split(':')[1])).reduce((x, r) => x + r.total, 0);
const compIntro = rows.filter((r) => r.inf.layer === 'learning' && r.u.level === 1 && r.u.tag.startsWith('ch')).reduce((x, r) => x + r.total, 0);
const compAH = rows.filter((r) => r.inf.layer === 'learning' && r.u.tag.startsWith('app')).reduce((x, r) => x + r.total, 0);
const compKey = keyTot;
const compTeach = wI - frontWords - compScaff - compIntro - compAH - compKey;
const rm = (w, wpm) => (w / wpm / 60).toFixed(1);
const nRef = rows.filter((r) => r.inf.layer === 'reference').length;
// Executive companion (assembled from existing text only)
const sumKey = (keys) => keys.reduce((s, k) => s + (byKey[k] ? wc(byKey[k].heading) + wc(byKey[k].lines.join('\n')) : 0), 0);
const hard10 = ['ch05:5.5#1', 'ch05:5.6#2', 'ch05:5.7#2', 'ch05:5.14#1', 'ch05:5.14#2', 'ch06:6.9#2', 'ch08:8.13#1', 'ch08:8.13#2', 'ch08:8.13#7', 'ch08:8.13#8'];
const hard10w = hard10.reduce((s, id) => s + (rows.flatMap((r) => r.u.items).find((i) => i.jid === id)?.parts.total || 0), 0);
const scaff = (slug) => units.filter((u) => u.key.endsWith(':' + slug)).reduce((s, u) => s + wc(u.heading) + wc(u.lines.join('\n')), 0);
const comp = { 'Appendix A (one page)': sumKey(['appA']), 'Appendix H (ten concepts + self-test)': sumKey(['appH', ...units.filter((u) => u.tag === 'appH' && u.level === 2).map((u) => u.key)]), 'Key ideas to remember (8 chapters)': scaff('key-ideas-to-remember'), 'What you should now understand (8 chapters)': scaff('what-you-should-now-understand'), '8.11 Claims you must NOT make': sumKey(['ch08:8.11']), 'Appendix B (architecture explained once)': sumKey(['appB', ...units.filter((u) => u.tag === 'appB' && u.level === 2).map((u) => u.key)]), '8.3 Architectural limitations + 8.5 Incomplete functionality': sumKey(['ch08:8.3', 'ch08:8.5']), 'Ten hard questions (App E list)': sumKey(['appE:ten-hard-questions-to-rehearse-first']), 'The ten hardest judge boxes (full boxes from Part I)': hard10w, 'Chapter intro sections and 1.5 "in one minute"': sumKey(['ch01:1.5']) };
const compTotal = Object.values(comp).reduce((a, b) => a + b, 0);

const md = `# Two-layer structure analysis for the PhantomCache handbook

**Status:** PROPOSAL and analysis only. Written by agent LAYERS on ${today} (machine clock). Nothing in the delivered handbook (\`PHANTOMCACHE_ENGINEERING_HANDBOOK.md/.pdf\`, \`src/\`, \`build/\`, \`archive/\`) was edited, moved or deleted. The prototype lives in \`docs/learning/proposal_v2/\`; this file is generated by \`proposal_v2/analyze_sections.mjs\` and can be regenerated.

## 0. Summary and recommendation

- **What was measured (reproduced here by script on the current sources):** ${fmt(TOTAL)} words in the sources (whitespace tokens; front matter ${fmt(frontWords)}, ${units.filter((u) => u.level === 1).length} chapter/appendix h1 units, ${units.filter((u) => u.level === 2).length} h2 sections in the sources plus 5 in the front matter = ${units.filter((u) => u.level === 2).length + 5} h2 headings, which matches \`grep -c '^## '\` on the assembled Markdown (210); I could not reproduce the figure of 234 h2 sections; ${h3n} h3 sections); the lead's figure is ${fmt(LEAD_WORDS)} words (I measure ${fmt(TOTAL)}; I did not reconcile the difference of ${fmt(TOTAL - LEAD_WORDS)}) and 183 pages (about 717 words per page), and I use 183 pages scaled to my total for page estimates. ${figCount} figures. ${jt.n} judge boxes in the chapters plus a ${fmt(eAll)}-word Appendix E that restates them (${eEntries.length} entries).
- **Judge material is the largest single block:** chapter judge boxes ${fmt(jt.tot)} words + Appendix E ${fmt(sumE())} words = ${fmt(jt.tot + sumE())} words, ${(100 * (jt.tot + sumE()) / TOTAL).toFixed(0)}% of the book. Inside the boxes, "If the judge asks further" is ${fmt(jt.further)} words (${(100 * jt.further / jt.tot).toFixed(0)}% of a box), "Suggested answer" ${fmt(jt.ans)} (${(100 * jt.ans / jt.tot).toFixed(0)}%), "Evidence to know" ${fmt(jt.ev)} (${(100 * jt.ev / jt.tot).toFixed(0)}%).
- **Proposed split (two layers, nothing deleted):** Part I Learning Handbook **${fmt(wI)} words (about ${Math.round(pg(wI))} A4 pages, ${(100 * wI / TOTAL).toFixed(0)}%)**; Part II Technical Reference **${fmt(wII)} words (about ${Math.round(pg(wII))} pages)**. ${Object.keys(REF).length} chapter sections or appendices (${R.sections_moved_whole} h1/h2 units) move whole, ${R.judge_boxes_moved} of ${jt.n} judge boxes move (${R.judge_boxes_kept} stay), ${fmt(R.table_words_extracted)} words of tables are extracted from ${R.tables_extracted} tables in ${splitList.length} sections.
- **True duplicates found: very few.** The only cell-for-cell duplicate is the Chapter 8, 8.12 matrix (${contained} of ${hdr812} rows, ${fmt(cw)} words) which Appendix G carries with an extra pointer column. Everything else that looks redundant (Appendix E vs the judge boxes, Appendix B vs Chapter 2, 4.11 and 5.15 vs 8.12 / G) is **paraphrased, not copied**, so removing it would be rewriting, not trimming. I propose no removal beyond the 8.12 table, and only as an owner decision.
- **Recommendation (three sentences).** Adopt the two-layer structure as a build-time view of the existing sources (no rewrite): Part I at about ${fmt(wI)} words keeps every chapter, worked example, figure, knowledge check and the ${R.judge_boxes_kept} key judge boxes, and Part II keeps everything else verbatim. This leaves the reader a first book of roughly ${Math.round(pg(wI))} pages (${(100 - 100 * wI / TOTAL).toFixed(0)}% smaller than today) without deleting a word, but that is a reading order, not yet a concise book: a markedly shorter Part I needs a separate editing pass over the chapters and the paraphrased duplicates (about ${fmt(rm0())} words of upper-bound savings, section 7.3), which I have not done. Do it only after the lead has inspected the two prototype PDFs and the ${ptr.toReference} cross-references that cross the layer boundary have been given a "Part II" marker.

## 1. Method, and what this analysis is not

1. The sources in \`docs/learning/src/\` (front matter, 8 chapters, 8 appendices) were parsed read-only. A section is an h1/h2 unit; judge boxes (\`::: judge\`) are extracted as separate items; headings inside code fences or fenced divs are ignored. Section ids are \`chNN:X.Y\` (numbered h2), \`chNN:slug\` (unnumbered h2: What you will learn, Key ideas, Knowledge check, ...), \`chNN\` / \`appX\` (the h1 with its introduction). Appendix D (glossary) has no h2 and is one unit; Appendix E has nine thematic h2 sections plus two front h2s.
2. **Word counts are whitespace tokens** (including headings, table pipes counted as tokens only where separated by spaces, code and callouts). The builder's check uses alphanumeric tokens only (so '#', '|', ':::' do not count); both are reported.
3. **Classification is by judgement**, assisted by element statistics: I read the headings of every section, read the opening of roughly 40 sections in full or in part, and measured table, judge-box and callout words for all of them. I did **not** read all 131,000 words. A category is therefore a reasoned proposal, not a verified fact; "A" in particular means "teaches something a newcomer needs", which is a judgement the owner may weigh differently.
4. Categories: **A** essential explanation; **B** detailed technical reference (long tables, route/config, evidence-to-know lists); **C** repeated explanation or redundant table; **D** excessive judge follow-up ("If the judge asks further"); **E** discrepancy / evidence inventory. A section has one category; word-level totals in 3.2 additionally split judge boxes into their parts (D = the follow-up part).
5. Page estimates scale words by 183 / ${fmt(TOTAL)}. Tables and figures probably take more space per word than prose, so the Reference layer (table-heavy) will likely come out at **more** pages than its word share suggests and Part I at fewer; I did not measure per-section pages (needs the PDF's page index, not examined).

## 2. Measured facts about the current book

### 2.1 Element types (whole book, words)
| Element | Words | Share of ${fmt(TOTAL)} |
|---|---|---|
| Pipe tables | ${fmt(el.table)} | ${(100 * el.table / TOTAL).toFixed(0)}% |
| Judge boxes in chapters (${jt.n}) | ${fmt(jt.tot)} | ${(100 * jt.tot / TOTAL).toFixed(0)}% |
| Appendix E consolidated bank (${eEntries.length} entries, incl. its two intro lists and the Never-say table) | ${fmt(appE.reduce((s, u) => s + wc(u.heading) + wc(u.lines.join('\\n')), 0))} | ${(100 * appE.reduce((s, u) => s + wc(u.heading) + wc(u.lines.join('\\n')), 0) / TOTAL).toFixed(0)}% |
| Appendix D glossary | ${fmt(sumKey(['appD']))} | ${(100 * sumKey(['appD']) / TOTAL).toFixed(0)}% |
| Worked examples (\`::: example\`) | ${fmt(el.example)} | ${(100 * el.example / TOTAL).toFixed(1)}% |
| Knowledge-check answers (\`::: answers\`) | ${fmt(el.answers)} | ${(100 * el.answers / TOTAL).toFixed(1)}% |
| Caution callouts | ${fmt(el.caution)} | ${(100 * el.caution / TOTAL).toFixed(1)}% |
| Concept summaries | ${fmt(el.concept)} | ${(100 * el.concept / TOTAL).toFixed(1)}% |
| Code blocks | ${fmt(el.code)} | ${(100 * el.code / TOTAL).toFixed(1)}% |
| Figure captions (${figCount} figures; SVG not counted) | ${fmt(el.figure)} | ${(100 * el.figure / TOTAL).toFixed(1)}% |

(Tables are counted only on lines starting with a pipe; wide tables are therefore fully counted. The rows overlap: judge boxes contain no tables, App E contains the Never-say table.)

### 2.2 Judge boxes: where the words are (category D measure)
Words per box part, per chapter. "Labels" are the fixed words ("JUDGE MAY ASK", "Suggested answer (about 30 to 60 seconds spoken, plain English):" and so on).

| Where | Boxes | Question | Suggested answer | If the judge asks further | Evidence to know | Labels | Box total | Follow-up share |
|---|---|---|---|---|---|---|---|---|
${jrows.join('\n')}

- Average box: ${Math.round(jt.tot / jt.n)} words; suggested answer ${Math.round(jt.ans / jt.n)}; follow-up ${Math.round(jt.further / jt.n)}; evidence ${Math.round(jt.ev / jt.n)}. The suggested answer is sized for the 30 to 60 seconds the style guide asks for (about ${Math.round(jt.ans / jt.n)} words is roughly ${(jt.ans / jt.n / 150 * 60).toFixed(0)} s at 150 wpm), so it is **not** excessive. The follow-up share is ${(100 * jt.further / jt.tot).toFixed(0)}% and is the part I would reduce in Part I.
- Appendix E: ${eEntries.length} entries, ${fmt(eAll)} words; "Answer" paragraphs ${fmt(eAns)} words (${(100 * eAns / eAll).toFixed(0)}%); "Where taught / Evidence to know" lines ${fmt(eEv)} (${(100 * eEv / eAll).toFixed(0)}%); each entry carries a "Where taught" pointer (${whereTaught} pointers) that points to a chapter section.
- Questions asked more than once inside Appendix E (Jaccard overlap of the content words of the question text >= 0.34; automatic, so some pairs may be false positives such as DIS-2/SEC-3; the pairs INN-3/4/5 are "what is the actual intelligence" asked by Chapters 1/5/7): ${nearQ.length} pairs. ${nearQ.map((q) => `${q[0]}/${q[1]}`).join(', ')}. These are the clearest candidates for a later merge; they are paraphrased, not identical.

### 2.3 Repetition measured, not guessed
| Test | Result |
|---|---|
| Lines (>= 10 words) identical in two or more units | ${dupLines.length} distinct lines: ${dupLines.map(([n, ids]) => `"${n.slice(0, 40)}..." in ${ids.length} places`).join('; ')}. All are boilerplate (the judge-box label line, the table header of App F / App G), not content. |
| 8.12 matrix rows whose every cell appears verbatim in Appendix G (G.2 to G.4) | **${contained} of ${hdr812} rows, ${fmt(cw)} of ${fmt(aw)} words**. Not contained: ${notC.join('; ') || 'none'}. |
| 8.11 "Claims you must NOT make" vs App E "Never say" (cell-for-cell) | ${ns811} of ${e811.length} rows identical; the App E list has ${ns.length} rows and shortens the wording. A near-duplicate, not a copy. |
| App E sentences (>= 8 words) identical to a sentence in a chapter judge box | ${eDup} of ${eSent} sentences (${fmt(eDupW)} of ${fmt(eW)} words, ${(100 * eDupW / eW).toFixed(0)}%). App E is a re-writing, not a paste. |
| 8-word phrase overlap (share of a unit's words that sit in an 8-word phrase also found in another file) | see table below |

| Unit | Words | Words in shared 8-word phrases | Share |
|---|---|---|---|
${shingleRows.join('\n')}

Reading the shingle table: 50 to 65% for the App E themes means the bank reuses long phrases of the chapter boxes but is not a verbatim copy; 86% for 8.12 and 55% to 72% for G.2 to G.4 reflect the cell-level duplicate above.

## 3. Totals

### 3.1 By proposed layer
| Layer | Words | Share | Estimated A4 pages | Read at 200 wpm | Skim at 400 wpm |
|---|---|---|---|---|---|
| Part I, Learning Handbook | ${fmt(wI)} | ${(100 * wI / TOTAL).toFixed(0)}% | ${Math.round(pg(wI))} | ${rm(wI, 200)} h | ${rm(wI, 400)} h |
| Part II, Technical Reference | ${fmt(wII)} | ${(100 * wII / TOTAL).toFixed(0)}% | ${Math.round(pg(wII))} | (look-up use; ${rm(wII, 200)} h end to end) | ${rm(wII, 400)} h |
| Whole book (today) | ${fmt(TOTAL)} | 100% | 183 (measured) | ${rm(TOTAL, 200)} h | ${rm(TOTAL, 400)} h |

Cross-check from the generated files (alphanumeric-token basis): Part I ${fmt(R.words_alnum.part_I)} + Part II ${fmt(R.words_alnum.part_II)} = ${fmt(R.words_alnum.sum)} versus original ${fmt(R.words_alnum.original)}: ${R.words_alnum.check_equal ? 'equal (no content lost)' : 'MISMATCH'}. Whitespace words of the generated files, generated linking text excluded: Part I ${fmt(contentWs.I)}, Part II ${fmt(contentWs.II)}; my allocation above gives ${fmt(wI)} and ${fmt(wII)} (the small difference is how heading tokens and table-pipe tokens are counted; the check that matters is the alphanumeric one).

### 3.2 By category (word level; judge boxes split into their parts)
| Category | Words | Share | Where it sits |
|---|---|---|---|
| A Essential explanation (incl. learning scaffolds, key judge answers) | ${fmt(cats.A)} | ${(100 * cats.A / TOTAL).toFixed(0)}% | Part I |
| B Detailed technical reference (tables, glossary, source map, evidence-to-know, extended judge answers) | ${fmt(cats.B)} | ${(100 * cats.B / TOTAL).toFixed(0)}% | Part II (except evidence-to-know lines of the ${R.judge_boxes_kept} key boxes) |
| C Repeated or redundant (App E, App B, App G, 8.12, 4.11, 5.15, 4.0, the 1.6 table) | ${fmt(cats.C)} | ${(100 * cats.C / TOTAL).toFixed(0)}% | Part II |
| D Judge follow-ups ("If the judge asks further") | ${fmt(cats.D)} | ${(100 * cats.D / TOTAL).toFixed(1)}% | ${fmt(jt.further - rows.flatMap((r) => r.u.items.filter((i) => i.type === 'judge' && !i.key)).reduce((s, i) => s + i.parts.further, 0))} w in Part I (key boxes), rest in Part II |
| E Discrepancy and evidence inventories | ${fmt(cats.E)} | ${(100 * cats.E / TOTAL).toFixed(0)}% | Part II |
| Sum | ${fmt(sumCats)} | ${(100 * sumCats / TOTAL).toFixed(0)}% | (rounding: ${sumCats === TOTAL ? 'exact' : 'differs by ' + (sumCats - TOTAL) + ' tokens'}) |

### 3.3 Reference layer by group
| Group | Whole sections | Words (whole sections only) |
|---|---|---|
${Object.keys(GROUPS).map((g) => `| ${GROUPS[g]} | ${grp[g]?.n || 0} | ${fmt(grp[g]?.w || 0)} |`).join('\n')}
| R1 also receives extracted tables | ${splitList.length} sections | ${fmt(R.table_words_extracted)} |
| R4 also receives chapter judge boxes | ${R.judge_boxes_moved} boxes | ${fmt(movedTot)} |

### 3.4 What Part I is made of
| Component of Part I | Words | Share of Part I |
|---|---|---|
| Teaching sections (explanation, examples, figures, cautions, concept boxes, remaining tables) | ${fmt(compTeach)} | ${(100 * compTeach / wI).toFixed(0)}% |
| Learning scaffolds (objectives, key ideas, understanding, knowledge check and answers, exploration tasks, challenge questions) | ${fmt(compScaff)} | ${(100 * compScaff / wI).toFixed(0)}% |
| ${R.judge_boxes_kept} key judge boxes | ${fmt(compKey)} | ${(100 * compKey / wI).toFixed(0)}% |
| Appendices A and H | ${fmt(compAH)} | ${(100 * compAH / wI).toFixed(0)}% |
| Front matter | ${fmt(frontWords)} | ${(100 * frontWords / wI).toFixed(0)}% |
| Chapter titles and introductions | ${fmt(compIntro)} | ${(100 * compIntro / wI).toFixed(0)}% |

Part I is still mostly the teaching text of the chapters. The two-layer move alone does not make that text shorter: a markedly shorter Part I (for example 40 to 50 thousand words) would need the chapters themselves to be condensed, which is editing, not routing, and is outside this prototype.

## 4. What would MOVE (nothing is deleted by default)

### 4.1 Whole sections moved to Part II (${nRef})
| id | heading | words | cat | group |
|---|---|---|---|---|
${moveList.join('\n')}

### 4.2 Tables extracted from sections that stay in Part I (tables >= ${SPLIT_MIN_WORDS} words only; prose, figures and examples stay)
| id | section | table words moved | cat |
|---|---|---|---|
${splitList.join('\n')}

### 4.3 Judge boxes kept in Part I (${R.judge_boxes_kept} boxes, ${fmt(keyTot)} words)
Selection rule: the ten questions the bank itself lists as "hard" (an overclaim does the most damage) plus the questions that define each chapter's core idea. Choice is a judgement; the owner should replace it with their own list.
| id | question | words |
|---|---|---|
${keptBoxes.join('\n')}

### 4.4 Judge boxes moved whole to Part II under the same heading (${R.judge_boxes_moved} boxes, ${fmt(movedTot)} words)
| id | question |
|---|---|
${movedBoxes.join('\n')}

## 5. What could be REMOVED because it is a true duplicate

Only one true (cell-for-cell) duplicate was found.

**Pair:** \`src/${f8}\` line ${l812}, section **8.12 Implementation versus roadmap matrix** (tables, ${fmt(cw)} words in ${contained} rows) versus \`src/${fG}\` line ${lG2}, sections **G.2 to G.4** (same rows plus a fourth column "Chapter pointer"). Appendix G says so itself in its column header: "Why (as stated in Ch 8, 8.12)".

Examples (first rows of each):
\`\`\`
8.12: ${sample812.join('\n      ')}
G.2 : ${sampleG.join('\n      ')}
\`\`\`
Rows of 8.12 not found cell-for-cell in Appendix G: ${notC.length ? notC.join('; ') : 'none (all data rows are contained)'}. The introductory paragraph of 8.12 was not compared and would stay.

**Proposal:** if the owner accepts, keep Appendix G (it is the superset) and drop the 8.12 tables, leaving the 8.12 reading-rule paragraph and a pointer. That would save ${fmt(cw)} words (about ${(cw * PAGES / TOTAL).toFixed(1)} pages). The prototype implements this behind \`--apply-dedup\` (default off, so the word-count check is exact by default); the manifest then lists the removed lines. Chapter 8 then points to Appendix G, which is in the Reference layer: a reader of Part I loses the matrix unless the pointer is followed. Since 8.12 is already assigned to Part II, the effect inside the two-layer build is that **one** matrix remains in Part II instead of two.

**Not proposed for removal (near-duplicates; removing them means rewriting):**
- Appendix E (${fmt(sumE())} words) versus the ${jt.n} chapter judge boxes: the bank is a condensed rewrite; only ${eDup} sentences are identical. Merging it would mean keeping one canonical text per question and generating the bank as a list of pointers; that is an editing job with a risk of losing the small differences between versions. Section 2.2 lists the ${nearQ.length} question pairs asked more than once.
- 8.11 versus App E "Never say" (${l811 ? 'src/' + f8 + ' line ' + l811 : ''}): shortened paraphrase, 538 words in App E.
- Appendix B (${fmt(sumKey(['appB', ...units.filter((u) => u.tag === 'appB' && u.level === 2).map((u) => u.key)]))} words) versus Chapters 2, 4, 5: re-told, with only 15 to 21% phrase overlap.
- 4.11 and 5.15 versus 8.12 / Appendix G: same capabilities relabelled in different words and columns; the labels were reconciled by hand (G.5 says so), so they are the one place where a duplicate costs consistency; merging is attractive but not a verbatim removal.

## 6. Section-by-section classification (every h1 and h2 unit)

\`id\` is the key in \`proposal_v2/layer_map.json\`. "Words" is the whole unit including its h3 subsections, judge boxes and callouts. The front-matter rows (F0 to F${secRows.filter((s) => s.startsWith('| front')).length - 1}) all stay in Part I. Category D appears as the unit's category only for the two sections made entirely of judge boxes (7.9 and 8.13); in all other sections judge boxes are an element inside an A section and are accounted for in 3.2.

| id | chapter | heading | words | category | proposed layer | one-line rationale |
|---|---|---|---|---|---|---|
${secRows.join('\n')}

### 6.1 h3 sections (${h3n}), inherited classification
All h3 sections inherit the layer of their h2 parent (they move with it). The only h3 groups that are routed differently from their parent are the judge boxes inside them, as listed in 4.3 and 4.4 (8.13.1 to 8.13.5 are headings over groups of boxes). The repeated h3 pattern "Simple explanation / Terms / How it works / Complexity / Trade-offs" is the teaching pattern of the style guide and is category A everywhere it occurs.

| parent id | h3 heading | words | category | layer |
|---|---|---|---|---|
${h3Rows.join('\n')}

## 7. Honest assessment of the two-layer structure

### 7.1 Benefits
- **A reader can finish the first layer.** At about ${fmt(wI)} words (${Math.round(pg(wI))} pages; ${rm(wI, 200)} h at 200 wpm) Part I is a ${(100 - 100 * wI / TOTAL).toFixed(0)}% smaller object than the book, without removing one word from the corpus.
- **The reference material is where a reference reader looks.** Route, configuration, threat-matrix, glossary and source-map tables are consulted by search, not read in order. They interrupt the teaching flow today (for example the ${fmt(R.table_words_extracted)} extracted table words, and 6.11 alone is about 1,430 table words).
- **The status information is stated once per layer, not three times.** Capability labels live today in 4.11, 5.15, 8.12 and Appendix G; in the proposal they all sit together in R1 (and the 8.12 / G duplicate can be reduced to one).
- **Judge defence stays intact.** The ten hard questions' boxes and one core box per chapter stay in the learning layer; every other box is one step away in R4 under the same heading.
- **It is a build-time view.** The sources stay as they are; the builder routes by \`layer_map.json\`. Reverting the proposal is deleting a folder.

### 7.2 Risks and costs
1. **Cross-references.** I counted ${ptr.total} section pointers in the Part I text with a regex (approximate); ${ptr.toReference} of them (${R.partI_pointers_to_moved_percent}%) point to sections that would now be in Part II (most-cited: ${topT}). Appendix E has ${whereTaught} "Where taught: Ch N, N.M" pointers; I did not check each one; the ones that target a section now in Part II are among the counted cross-layer pointers only if they are written in the Part I text, so the Appendix E pointers (all inside Part II) were not counted. They are not rewritten by the prototype. A real adoption needs "(Part II)" markers on those ${ptr.toReference} pointers, and the front-matter "How the chapters are organised" and "Three ways to read it" lists need rewriting.
2. **The judge boxes carry the defence value.** Moving ${R.judge_boxes_moved} of ${jt.n} boxes out of the chapters means a student who reads only Part I meets ${R.judge_boxes_kept}. The mitigation is the stub at the end of each section ("N further judge boxes for this section are in Part II, R4") and the choice of which ${R.judge_boxes_kept} stay. If the owner thinks a judge question belongs next to the explanation (the style guide's step 7, "Prepare to defend it"), the teaching pattern is weakened by the move. Alternative: keep all boxes in Part I but drop "If the judge asks further" (that edits content and is **not** done here).
3. **Chapters 4 to 6 teach through worked examples and must stay.** They do: the \`::: example\` blocks (${fmt(el.example)} words, mostly in 4 and 5) are in teaching sections that are not moved. The only Chapter 4 to 6 sections moved are 4.0, 4.11, 4.12, 5.15, 6.11 and 6.13 (labels, status tables, discrepancy list, matrix, measurement inventory); each is an inventory, not a lesson. 6.11 is the largest risk: it is the one table a judge-prep reader may want next to 6.2 to 6.10; it is flagged for owner decision.
4. **Knowledge checks and exploration tasks** are all in Part I (they are not touched); Chapter 4's check answers are in a separate \`Answers\` section that stays with the chapter.
5. **Two PDFs instead of one** mean two tables of contents, two page numberings and a "which book has it" question. A single-file ordering "Part I, then Part II" avoids this if the owner prefers one PDF; the prototype can concatenate the two Markdown files (not built).
6. **Classification is judgement.** Different owners will move 8.4, 8.2 and 3.1 differently; the whole mechanism is one JSON file so that it is cheap to change.
7. **Not asked, not done:** shortening any text. The proposal reduces what the reader meets first, but total corpus size is unchanged (131k words); an owner who meant "the whole book is too big" has been given a reading order, not a smaller book.

### 7.3 What a later merge pass could save (not done; an editing task for the owner to approve)
- Appendix E re-expressed as pointers plus the Never-say table: up to about ${fmt(sumE() - 538 - 190 - 152)} words if only the pointer lines, question text and the "Never say" list were kept (this would discard the condensed answers; one canonical answer per question would then live only in the chapter box).
- The ${nearQ.length} duplicated question pairs in App E and the same-topic boxes in different chapters (for example "tests pass", "actual intelligence", "DELETE 204", "linearizable", "partitioned mutex"): choose one box, point to it from the others.
- Appendix B (about 3,400 words) merged into Chapter 2 or dropped in favour of Appendix A plus Chapter 2.
These figures are upper bounds computed from section sizes, not tested savings.
- Chapter condensing (not estimated): Chapters 4 to 7 are the largest chapters (13k, 17k, 15k and 13k words); condensing them is the only route to a Part I well below the figure above.

## 8. Recommended structure

**Part I, Learning Handbook (${fmt(wI)} words, ~${Math.round(pg(wI))} pages, ~${rm(wI, 200)} h reading):** front matter and the reading routes; Chapters 1 to 8 as they are, minus the sections listed in 4.1 and 4.2, with ${R.judge_boxes_kept} judge boxes; each chapter's Key ideas, What you should now understand, Knowledge check (and answers), exploration tasks and challenge questions; Appendix A (one page) and Appendix H (ten concepts and self-test).

**Part II, Technical Reference (${fmt(wII)} words, ~${Math.round(pg(wII))} pages, consulted):** R1 matrices, route and configuration tables (status tables 4.11, 5.15, 8.12 or Appendix G, 8.10, 6.11, the dependency and data-structure tables, Appendix C, plus extracted tables); R2 source-file map (App F); R3 glossary (App D); R4 judge bank (App E, then the ${R.judge_boxes_moved} chapter judge boxes by chapter and section); R5 discrepancy and evidence inventories (1.10, 2.11, 3.13, 4.12, 6.13, 7.10, 8.4, 8.7); R6 restated explanations (App B, 4.0).

Reading-time estimates (my assumption, not measured): Part I about ${rm(wI, 200)} h at 200 wpm for reading, ${rm(wI, 400)} h to skim; adding the knowledge checks and exploration tasks (they are in Part I) roughly doubles that for a reader who does them. The "one hour before a presentation" route in the front matter becomes: App A, App H, 8.11, then R4 "Ten hard questions" (about ${fmt(comp['Appendix A (one page)'] + comp['Appendix H (ten concepts + self-test)'] + comp['8.11 Claims you must NOT make'])} words, ${rm(comp['Appendix A (one page)'] + comp['Appendix H (ten concepts + self-test)'] + comp['8.11 Claims you must NOT make'], 200)} h).

## 9. Alternatives

**Alternative 1: one book plus "core path" markers (no restructure).** Add a marker (a badge in the heading, or a Core Path list in the front matter) to the ${rows.filter((r) => r.inf.layer === 'learning').length} Part I sections and leave everything in place. Cost: almost nothing, no broken pointers, one PDF. Benefit: the reader has a route of about ${fmt(wI)} words. Weakness: the book is still 183 pages and the reference material still interrupts the flow; "concise" is not achieved. It is a useful first step and can be derived from the same \`layer_map.json\`.

**Alternative 2: keep the book, add an "Executive Companion" of about ${fmt(compTotal)} words (assembled from existing text, nothing new written).** Made of: ${Object.entries(comp).map(([k, v]) => `${k} (${fmt(v)})`).join('; ')}. About ${Math.round(compTotal * PAGES / TOTAL)} pages, ${rm(compTotal, 200)} h. Benefit: the preparation-evening document, with no change to the book. Weakness: it duplicates ~${fmt(compTotal)} words by design (it is a view of existing text) and the existing book stays as large as now.

## 10. Recommendation
Choose the two-layer build as the primary format, but ship it in two steps: (1) generate Part I and Part II from the existing sources with \`layer_map.json\` (this prototype), add "(Part II)" markers to the ${ptr.toReference} pointers that cross the boundary and rewrite the front-matter reading routes; (2) only afterwards, and only with the owner's approval, run a separate merge pass over the paraphrased duplicates (Appendix E questions asked twice, Appendix B, the 8.12 table). Keep Alternative 2 as a cheap supplement once Part I exists, since most of its parts are already in Part I. Do not remove anything on the strength of this analysis alone.

## 11. What I could not assess
- Whether the owner experiences a ${Math.round(pg(wI))}-page Part I as concise; whether readers find the stub-and-pointer pattern acceptable; the actual page counts of the two PDFs (not rendered by me; the lead runs the PDF step) and how the typeset tables change the page estimate.
- The correctness of any technical claim in the handbook. This analysis classifies and counts; it does not re-verify claims against the code, and it labels nothing "verified" beyond the counts and the cell-level comparisons the scripts print.
- Content quality of sections I sampled rather than read in full (about 40 sections opened; the rest classified from heading, size and element statistics). Category assignments for 6.11, 8.2, 8.4, 8.7, 3.1, 3.6 and the 1.10 history are the most debatable.
- The effect of moving the ${R.judge_boxes_moved} judge boxes (plus 2 that sit inside sections that move whole) on exam-style rehearsal; the judge-box selection (4.3) is my judgement and the lead/owner should replace it.
- Cross-reference counts are from a regex and are approximate (they miss pointers phrased in other ways and may mis-attribute a bare "4.3" in a table).
- Anything about the PDF layout (the prototype's PDF step was not run by me).

## 12. Files created
\`docs/learning/evidence/two_layer_analysis.md\` (this file); \`docs/learning/proposal_v2/\`: \`parse_lib.mjs\`, \`layers_lib.mjs\`, \`analyze_sections.mjs\`, \`build_two_layer.mjs\`, \`layer_map.json\`, \`out/learning.md\`, \`out/reference.md\`, \`out/learning.html\`, \`out/reference.html\`, \`out/manifest.json\`, \`out/manifest.md\`, \`probes/\` (the exploratory scripts used for section 2.3). Rebuild: \`node docs/learning/proposal_v2/analyze_sections.mjs\` (writes the map and this file, runs the builder with \`--no-pdf\`); PDFs: \`node docs/learning/proposal_v2/build_two_layer.mjs\`.
`;
function sumE() { return appE.reduce((s, u) => s + wc(u.heading) + wc(u.lines.join('\n')), 0); }
function rm0() { return sumE() - 538 - 190 - 152 + 3443; }
writeFileSync(OUTMD, md);
console.log('analysis written', md.split(/\s+/).length, 'words; layers', wI, wII, 'cats', JSON.stringify(cats), 'sum', sumCats, 'TOTAL', TOTAL);
