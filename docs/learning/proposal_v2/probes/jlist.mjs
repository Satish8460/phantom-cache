import { listFiles, parseFile, splitItems, judgeParts, wc } from '../parse_lib.mjs';
let n=0;
for (const f of listFiles()) for (const u of parseFile(f)) { let k=0; for (const it of splitItems(u.lines)) if(it.type==='judge'){ k++; n++; const q=(it.text.match(/Question:\s*([^\n]*)/)||[])[1]; console.log(`${u.id}#${k} | ${wc(it.text)} | ${q.slice(0,95)}`);} }
