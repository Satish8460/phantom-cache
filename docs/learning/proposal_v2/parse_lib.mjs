// Shared READ-ONLY parser for the handbook sources. Used by analyze_sections.mjs and build_two_layer.mjs.
import { readdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
export const HERE = dirname(fileURLToPath(import.meta.url));
export const ROOT = join(HERE, '..');
export const SRC = join(ROOT, 'src');
export const wc = (s) => (s.trim() ? s.trim().split(/\s+/).length : 0);
const rank = (f) => (f.startsWith('ch') ? 0 : f.startsWith('bm') ? 1 : 2);
export function listFiles() {
  return readdirSync(SRC).filter((f) => /^(ch\d{2}|bm\d{2})_.+\.md$/.test(f)).sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}
export function fileTag(f) {
  const m = f.match(/^ch(\d{2})_/); if (m) return 'ch' + m[1];
  const b = f.match(/^bm(\d{2})_([a-h])_/); return 'app' + b[2].toUpperCase();
}
// Split a file into h1/h2 units. Each unit: {id, level, heading, lines[], file}. Headings inside code fences are ignored.
// Judge boxes (::: judge ... :::) are extracted from each unit as separate items so they can be routed independently.
export function parseFile(f) {
  const tag = fileTag(f);
  const text = readFileSync(join(SRC, f), 'utf8').replace(/\r\n/g, '\n').replace(/\]\(\.\.\/assets\//g, '](assets/').trim();
  const lines = text.split('\n');
  const units = []; let cur = null; let fence = false; let div = 0;
  const open = (level, heading) => { cur = { file: f, tag, level, heading, lines: [] }; units.push(cur); };
  for (const ln of lines) {
    if (/^```/.test(ln)) fence = !fence;
    if (!fence && /^:::/.test(ln)) { if (/^::: *\w/.test(ln)) div++; else if (/^:::\s*$/.test(ln)) div = Math.max(0, div - 1); }
    const h = !fence && div === 0 && ln.match(/^(#{1,2}) (.+)$/);
    if (h) open(h[1].length, h[2].trim()); else { if (!cur) open(1, '(preamble)'); cur.lines.push(ln); }
  }
  // ids
  for (const u of units) {
    const n = u.heading.match(/^(\d+\.\d+|\d+\.\d+\.\d+|[A-H]\.\d+)\b/);
    const slug = u.heading.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 40);
    u.id = u.level === 1 ? u.tag : n ? `${u.tag}:${n[1]}` : `${u.tag}:${slug}`;
  }
  return units;
}
// Split unit lines into ordered items: {type:'text'|'judge', text}
export function splitItems(lines) {
  const items = []; let buf = []; let inJ = false; let fence = false;
  const flush = (t) => { if (buf.length) { items.push({ type: t, text: buf.join('\n') }); buf = []; } };
  for (const ln of lines) {
    if (/^```/.test(ln)) fence = !fence;
    if (!fence && !inJ && /^::: *judge\b/.test(ln)) { flush('text'); inJ = true; buf.push(ln); continue; }
    if (!fence && inJ) { buf.push(ln); if (/^:::\s*$/.test(ln)) { flush('judge'); inJ = false; } continue; }
    buf.push(ln);
  }
  flush(inJ ? 'judge' : 'text');
  return items;
}
export function judgeParts(text) {
  const q = text.match(/Question:([\s\S]*?)\*\*Suggested answer\*\*/);
  const a = text.match(/\*\*Suggested answer\*\*[^\n]*\n([\s\S]*?)\*\*If the judge asks further/);
  const f = text.match(/\*\*If the judge asks further:?\*\*:?([\s\S]*?)\*\*Evidence to know/);
  const e = text.match(/\*\*Evidence to know:?\*\*:?([\s\S]*?)\n:::\s*$/);
  const W = (m) => (m ? wc(m[1]) : 0);
  return { q: W(q), answer: W(a), further: W(f), evidence: W(e), total: wc(text) };
}
