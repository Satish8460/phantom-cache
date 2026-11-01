import { listFiles, parseFile, wc } from '../parse_lib.mjs';
import { readFileSync } from 'node:fs';
const seen = new Map();
const norm = (s) => s.toLowerCase().replace(/[`*_>#]/g, '').replace(/\s+/g, ' ').trim();
for (const f of listFiles()) for (const u of parseFile(f)) {
  let fence=false;
  for (const ln of u.lines) {
    if (/^```/.test(ln)) fence=!fence;
    const n = norm(ln); if (wc(n) < 10 || /^\|[\s:|-]+\|$/.test(n)) continue;
    if (!seen.has(n)) seen.set(n, []);
    seen.get(n).push(u.id);
  }
}
let out=[], words=0;
for (const [n, ids] of seen) if (ids.length>1) { out.push([wc(n), ids.join(' & '), n.slice(0,130)]); words+=wc(n)*(ids.length-1); }
out.sort((a,b)=>b[0]-a[0]);
console.log(out.length, 'dup lines, extra words', words);
for (const o of out) console.log(o.join(' | '));
