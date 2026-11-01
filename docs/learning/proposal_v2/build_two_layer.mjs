// PROPOSAL (non-destructive): builds a two-layer version of the handbook from the CURRENT sources, read-only.
//   node docs/learning/proposal_v2/build_two_layer.mjs [--no-pdf] [--apply-dedup]
// Reads docs/learning/src/*.md and proposal_v2/layer_map.json (written by analyze_sections.mjs); writes ONLY into proposal_v2/out/:
//   learning.md/.html/.pdf   Part I  Learning Handbook
//   reference.md/.html/.pdf  Part II Technical Reference
//   manifest.json / manifest.md  which section went where + word-count check (Part I + Part II == original, duplicates excepted)
// Generated linking text carries the marker <!--gen--> and is excluded from the word-count check. Nothing is rewritten: only moved.
// Node built-ins only; pandoc and Chrome are tools already on this machine. Same verified-PDF pattern as build/build_handbook.mjs.
import { readFileSync, writeFileSync, existsSync, mkdirSync, rmSync, statSync, renameSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
import { listFiles, SRC, ROOT, wc } from './parse_lib.mjs';
import { loadUnits, pipeTableBlocks, SPLIT_MIN_WORDS, DEDUPE, GROUPS } from './layers_lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, 'out');
const CSS = join(ROOT, 'build', 'handbook.css'); // reused by reference, never modified
const PANDOC = process.env.PANDOC || 'C:/Users/Satish/AppData/Local/Pandoc/pandoc.exe';
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const NO_PDF = process.argv.includes('--no-pdf');
const DEDUP = process.argv.includes('--apply-dedup');
const GEN = '<!--gen-->';
const g = (s) => `${s} ${GEN}`;
const awc = (s) => s.split(/\s+/).filter((t) => /[A-Za-z0-9]/.test(t)).length; // alphanumeric tokens: ignores '#', '|', '---', ':::'

const mapPath = join(HERE, 'layer_map.json');
if (!existsSync(mapPath)) throw new Error('layer_map.json missing: run analyze_sections.mjs first');
const MAP = JSON.parse(readFileSync(mapPath, 'utf8'));
mkdirSync(OUT, { recursive: true });

const units = loadUnits();
const missing = units.filter((u) => !MAP.sections[u.key]);
if (missing.length) throw new Error('units without a layer: ' + missing.map((u) => u.key).join(', '));
const splitSet = new Set(MAP.split_tables);
const keyJudge = new Set(MAP.key_judge_boxes);
const frontRaw = readFileSync(join(SRC, '00_FRONT.md'), 'utf8').replace(/\r\n/g, '\n');
const frontBody = frontRaw.replace(/^---\n[\s\S]*?\n---\n/, '').trim();

const demote = (lines) => { let fence = false, div = 0; return lines.map((ln) => {
  if (/^```/.test(ln)) fence = !fence;
  if (!fence && /^::: *\w/.test(ln)) div++; else if (!fence && /^:::\s*$/.test(ln)) div = Math.max(0, div - 1);
  return (!fence && div === 0 && /^#{3,5} /.test(ln)) ? '#' + ln : ln; }); };

const learning = [];   // blocks for Part I
const refSections = {}; // group -> [{unit, text}]
const movedJudge = []; // {unit, jid, question, text}
const extracted = [];  // {unit, text, words}
const removedByDedup = [];
const manifest = [];
const stubsPartI = [];

for (const u of units) {
  const layer = MAP.sections[u.key];
  const head = '#'.repeat(u.level) + ' ' + u.heading;
  const rec = { id: u.key, heading: u.heading, layer, group: MAP.groups[u.key] || '', words: awc(head + '\n' + u.lines.join('\n')), judge_moved: [], judge_kept: [], table_words_extracted: 0, removed_dup_words: 0 };
  if (layer === 'reference') {
    let lines = u.lines;
    if (DEDUP && DEDUPE[u.key]) {
      const blocks = pipeTableBlocks(lines); const drop = new Set(); let removed = [];
      for (const b of blocks) { for (let i = b.start; i < b.end; i++) { drop.add(i); removed.push(lines[i]); } }
      if (removed.length) {
        const rw = awc(removed.join('\n')); rec.removed_dup_words = rw; removedByDedup.push({ id: u.key, duplicate_of: DEDUPE[u.key].of, words: rw, lines: removed.length });
        lines = lines.flatMap((ln, i) => (drop.has(i) ? (i > 0 && drop.has(i - 1) ? [] : [g(`*The table that stood here is identical, cell for cell, to the tables in ${DEDUPE[u.key].of === 'appG' ? 'the implementation-versus-roadmap matrix (G.2 to G.4)' : DEDUPE[u.key].of}, which add a chapter pointer column.*`)]) : [ln]));
      }
    }
    const body = [ '#'.repeat(Math.min(u.level + 1, 6)) + ' ' + u.heading, ...demote(lines) ].join('\n');
    (refSections[rec.group] ||= []).push({ unit: u, text: body });
    if (!(u.level === 2 && u.tag.startsWith('app'))) { // one stub per appendix (its h2 sections travel with it)
      learning.push({ stub: true, text: g(`**[Part II, ${rec.group}: ${u.heading}]**`), unit: u });
      stubsPartI.push(`${u.key}: ${u.heading} -> Part II ${rec.group}`);
    }
    manifest.push(rec); continue;
  }
  // layer === 'learning'
  let out = [];
  let movedHere = 0;
  for (const it of u.items) {
    if (it.type === 'judge') {
      if (keyJudge.has(it.jid)) { rec.judge_kept.push(it.jid); out.push(it.text); }
      else { rec.judge_moved.push(it.jid); movedHere++; movedJudge.push({ unit: u, jid: it.jid, question: it.question, text: it.text }); if (movedHere === 1) out.push('@@JUDGESTUB@@'); }
    } else if (splitSet.has(u.key)) {
      const lines = it.text.split('\n'); const blocks = pipeTableBlocks(lines).filter((b) => b.words >= SPLIT_MIN_WORDS); const drop = new Map();
      blocks.forEach((b) => drop.set(b.start, b));
      const keep = []; let i = 0;
      while (i < lines.length) {
        const b = drop.get(i);
        if (b) { const t = lines.slice(b.start, b.end).join('\n'); extracted.push({ unit: u, text: t, words: awc(t) }); rec.table_words_extracted += awc(t); keep.push(g(`*[The full table is in Part II, R1, "Tables extracted from Part I sections", under ${u.heading}.]*`)); i = b.end; }
        else { keep.push(lines[i]); i++; }
      }
      out.push(keep.join('\n'));
    } else out.push(it.text);
  }
  let body = out.join('\n');
  if (movedHere) body = body.replace('@@JUDGESTUB@@', g(`*${movedHere} further judge box${movedHere > 1 ? 'es' : ''} for this section ${movedHere > 1 ? 'are' : 'is'} in Part II, R4, under the same heading.*`));
  learning.push({ text: [head, body].join('\n'), unit: u });
  manifest.push(rec);
}

// ---------- Part I ----------
const partIYaml = `---\ntitle: "PhantomCache Engineering and Technical Defence Handbook, Part I: Learning Handbook"\nsubtitle: "PROPOSAL (two-layer structure): chapters taught in full, key judge boxes only"\n---`;
const noteI = [
  g('> **Two-layer proposal.** This is Part I, the Learning Handbook: the chapters in order, with their worked examples, figures, knowledge checks,'),
  g('> exploration tasks and challenge questions, and the key judge boxes. Moved items are marked "Part II". Part II, the Technical Reference, holds the'),
  g('> full route, configuration and status tables, the full judge bank, the glossary, the source-file map and the discrepancy and evidence inventories. Nothing was deleted: `reference.md` / `reference.pdf` in the same folder.'),
  g('> The cross-references in the text below ("see Chapter 4, section 4.11") still use the original numbering; targets that moved are listed in `manifest.md`.'),
].join('\n');
const partI = [partIYaml, noteI, frontBody, ...learning.map((b) => b.text)];

// ---------- Part II ----------
const partIIYaml = `---\ntitle: "PhantomCache Engineering and Technical Defence Handbook, Part II: Technical Reference"\nsubtitle: "PROPOSAL (two-layer structure): tables, judge bank, glossary, source map, inventories"\n---`;
const noteII = [
  g('> **Two-layer proposal.** This is Part II, the Technical Reference. It is consulted, not read in order. Part I (`learning.md` / `learning.pdf`) teaches the system.'),
  g('> Every item here was moved unchanged from the original handbook; section numbers (4.11, 8.12, Appendix G) are the original ones. Where a section was split, its heading is repeated here and marked as a continuation.'),
  g('> The status labels are defined in the front matter of Part I.'),
].join('\n');
const partII = [partIIYaml, noteII];
const chapTitle = (tag) => units.find((x) => x.tag === tag && x.level === 1).heading;
for (const grp of Object.keys(GROUPS)) {
  partII.push('# ' + g(GROUPS[grp]));
  const secs = refSections[grp] || [];
  if (grp === 'R4') {
    // consolidated bank first (appE), then the chapter judge boxes not in Part I
    secs.forEach((s) => partII.push(s.text));
    partII.push('## ' + g('The full chapter judge boxes that are not in Part I'));
    partII.push(g('*Each box is unchanged. Boxes sit under the heading of the section where they were taught.*'));
    let lastTag = '', lastSec = '';
    for (const m of movedJudge) {
      if (m.unit.tag !== lastTag) { partII.push('### ' + g(chapTitle(m.unit.tag))); lastTag = m.unit.tag; lastSec = ''; }
      if (m.unit.key !== lastSec) { partII.push('#### ' + g(m.unit.heading)); lastSec = m.unit.key; }
      partII.push(m.text);
    }
  } else {
    secs.forEach((s) => partII.push(s.text));
    if (grp === 'R1' && extracted.length) {
      partII.push('## ' + g('Tables extracted from Part I sections'));
      let last = '';
      for (const e of extracted) {
        if (e.unit.key !== last) { partII.push('### ' + g(e.unit.heading + ' (tables)')); last = e.unit.key; }
        partII.push(e.text);
      }
    }
  }
}
const mdI = partI.join('\n\n') + '\n';
const mdII = partII.join('\n\n') + '\n';
writeFileSync(join(OUT, 'learning.md'), mdI);
writeFileSync(join(OUT, 'reference.md'), mdII);

// ---------- word-count check ----------
const stripGen = (s) => s.replace(/^---\n[\s\S]*?\n---\n/, '').split('\n').filter((l) => !l.includes(GEN)).join('\n');
const origFiles = listFiles();
const origWordsFront = awc(frontBody);
const origWordsBody = origFiles.reduce((a, f) => a + awc(readFileSync(join(SRC, f), 'utf8')), 0);
const orig = origWordsFront + origWordsBody;
const wI = awc(stripGen(mdI)), wII = awc(stripGen(mdII));
const removed = removedByDedup.reduce((a, r) => a + r.words, 0);
const ok = wI + wII === orig - removed;
const rawWs = (s) => s.trim().split(/\s+/).length;
const figs = (s) => (s.match(/^!\[/gm) || []).length;
const origFigs = origFiles.reduce((a, f) => a + figs(readFileSync(join(SRC, f), 'utf8')), 0);
const judgeBoxes = (s) => (s.match(/^::: judge/gm) || []).length;
const origJudges = origFiles.reduce((a, f) => a + judgeBoxes(readFileSync(join(SRC, f), 'utf8')), 0);

// ---------- cross-layer pointers (approximate regex; reported, not rewritten) ----------
const layerOfSection = (n) => { const [c, s] = n.split('.'); const id = `ch${String(c).padStart(2, '0')}:${c}.${s}`; return MAP.sections[id]; };
const refRe = /(?:sections?|Ch(?:apter)?s? \d+,|Ch \d+,)\s+((?:\d{1,2}\.\d{1,2}(?:\.\d{1,2})?)(?:(?:,| and| to|;)\s*\d{1,2}\.\d{1,2}(?:\.\d{1,2})?)*)/g;
function pointers(md) {
  const hit = { total: 0, toLearning: 0, toReference: 0, targetsRef: {} };
  for (const m of stripGen(md).matchAll(refRe)) for (const n of m[1].match(/\d{1,2}\.\d{1,2}/g)) {
    const L = layerOfSection(n); if (!L) continue; hit.total++;
    if (L === 'learning') hit.toLearning++; else { hit.toReference++; hit.targetsRef[n] = (hit.targetsRef[n] || 0) + 1; }
  }
  for (const m of stripGen(md).matchAll(/Appendix ([A-H])\b/g)) { const L = MAP.sections['app' + m[1]]; if (!L) continue; hit.total++; if (L === 'learning') hit.toLearning++; else { hit.toReference++; hit.targetsRef['App ' + m[1]] = (hit.targetsRef['App ' + m[1]] || 0) + 1; } }
  return hit;
}
const pI = pointers(mdI);
const mdOf = mdII;
const result = {
  generated: new Date().toISOString(), apply_dedup: DEDUP,
  words_alnum: { original: orig, original_front: origWordsFront, original_sources: origWordsBody, part_I: wI, part_II: wII, sum: wI + wII, removed_duplicates: removed, check_equal: ok },
  words_whitespace_raw_files: { part_I: rawWs(mdI), part_II: rawWs(mdII) },
  figures: { original: origFigs, part_I: figs(mdI), part_II: figs(mdII), preserved: origFigs === figs(mdI) + figs(mdII) },
  judge_boxes: { original: origJudges, part_I: judgeBoxes(mdI), part_II: judgeBoxes(mdII), preserved: origJudges === judgeBoxes(mdI) + judgeBoxes(mdII) },
  sections_moved_whole: manifest.filter((m) => m.layer === 'reference').length,
  sections_split: manifest.filter((m) => m.layer === 'learning' && (m.judge_moved.length || m.table_words_extracted)).length,
  judge_boxes_moved: movedJudge.length, judge_boxes_kept: keyJudge.size,
  table_words_extracted: extracted.reduce((a, e) => a + e.words, 0), tables_extracted: extracted.length,
  removed_duplicates: removedByDedup,
  partI_pointers_approx: pI, partI_pointers_to_moved_percent: pI.total ? +(100 * pI.toReference / pI.total).toFixed(1) : 0,
};
writeFileSync(join(OUT, 'manifest.json'), JSON.stringify({ result, sections: manifest, moved_judge: movedJudge.map((m) => ({ id: m.jid, section: m.unit.key, question: m.question })) }, null, 1));
const mm = [`# Manifest of the two-layer build (generated ${result.generated})`, '',
  `Word-count check (alphanumeric tokens): original ${orig} = Part I ${wI} + Part II ${wII} + removed duplicates ${removed}: **${ok ? 'EQUAL' : 'MISMATCH'}**.`,
  `Figures: original ${origFigs}, Part I ${result.figures.part_I}, Part II ${result.figures.part_II}. Judge boxes: original ${origJudges}, Part I ${result.judge_boxes.part_I}, Part II ${result.judge_boxes.part_II}.`,
  `Cross-references in Part I (approximate regex): ${pI.total}, of which ${pI.toReference} point to sections now in Part II.`, '',
  '| id | heading | layer | group | words | judge kept | judge moved | table words extracted |', '|---|---|---|---|---|---|---|---|',
  ...manifest.map((m) => `| ${m.id} | ${m.heading.replace(/\|/g, '/')} | ${m.layer} | ${m.group} | ${m.words} | ${m.judge_kept.length} | ${m.judge_moved.length} | ${m.table_words_extracted} |`), '',
  '## Judge boxes moved to Part II (R4)', ...movedJudge.map((m) => `- ${m.jid}: ${m.question.slice(0, 100)}`), ''];
writeFileSync(join(OUT, 'manifest.md'), mm.join('\n'));
console.log(JSON.stringify(result, null, 1).split('\n').filter((l) => !/targetsRef|^\s+"\d|App /.test(l)).join('\n'));
if (!ok) { console.error('WORD COUNT MISMATCH'); process.exitCode = 2; }

// ---------- pandoc HTML ----------
const layers = [['learning', mdI], ['reference', mdII]];
for (const [name] of layers) {
  const md = join(OUT, name + '.md'), html = join(OUT, name + '.html');
  const p = spawnSync(PANDOC, [md, '-f', 'markdown+fenced_divs+pipe_tables+yaml_metadata_block+implicit_figures-smart', '-t', 'html5', '--standalone',
    '--embed-resources', '--toc', '--toc-depth=2', '--css', CSS, '--resource-path', ROOT, '-o', html], { encoding: 'utf8' });
  if (p.status !== 0) { console.error(p.stderr || p.stdout); process.exit(1); }
  if (p.stderr) console.log(name + ' pandoc warnings:\n' + p.stderr.split('\n').slice(0, 8).join('\n'));
  console.log('html ->', html);
}
if (NO_PDF) process.exit(process.exitCode || 0);

// ---------- PDF (verified-output pattern from build_handbook.mjs) ----------
for (const [name] of layers) {
  const html = join(OUT, name + '.html'), pdf = join(OUT, name + '.pdf'), tmp = join(OUT, name + '.new.pdf');
  const started = Date.now(); let printed = false;
  for (let attempt = 1; attempt <= 3 && !printed; attempt++) {
    try { rmSync(tmp, { force: true }); } catch { /* ignore */ }
    const profile = 'D:/Temp/handbook_chrome_' + Math.random().toString(36).slice(2, 8);
    mkdirSync(profile, { recursive: true });
    const c = spawnSync(CHROME, ['--headless=new', '--disable-gpu', '--no-first-run', '--no-pdf-header-footer', `--user-data-dir=${profile}`,
      `--print-to-pdf=${tmp}`, '--virtual-time-budget=60000', pathToFileURL(html).href], { encoding: 'utf8', timeout: 300000 });
    try { rmSync(profile, { recursive: true, force: true }); } catch { /* may be locked briefly */ }
    printed = existsSync(tmp) && statSync(tmp).size > 50000 && statSync(tmp).mtimeMs >= started;
    console.log(`${name} print attempt ${attempt}: ${printed ? 'ok' : 'FAILED'} (exit ${c.status})`);
  }
  if (!printed) { console.error(name + ': PDF was not produced; previous PDF (if any) left untouched'); process.exitCode = 1; continue; }
  try { renameSync(tmp, pdf); console.log('pdf ->', pdf, statSync(pdf).size, 'bytes'); }
  catch (e) {
    const alt = pdf.replace(/\.pdf$/, '_rebuilt.pdf');
    try { rmSync(alt, { force: true }); renameSync(tmp, alt); } catch { /* alt also locked */ }
    console.log(`could not replace ${pdf} (${e.code}); the new build is at ${existsSync(alt) ? alt : tmp}`);
  }
}
