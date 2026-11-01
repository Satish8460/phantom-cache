import { listFiles, parseFile, splitItems, wc } from '../parse_lib.mjs';
import { readFileSync } from 'node:fs';
const norm=(s)=>s.toLowerCase().replace(/[`*_]/g,'').replace(/\s+/g,' ').trim();
const sents=(t)=>t.replace(/\n+/g,' ').split(/(?<=[.?!])\s+/).map(norm).filter(s=>wc(s)>=8);
const chap=new Set(), appE=[];
for(const f of listFiles()) for(const u of parseFile(f)){
  if(u.tag.startsWith('ch')) for(const it of splitItems(u.lines)) if(it.type==='judge') sents(it.text).forEach(s=>chap.add(s));
  if(u.tag==='appE') sents(u.lines.join('\n')).forEach(s=>appE.push(s));
}
let tot=0,dup=0,ds=0; for(const s of appE){tot+=wc(s); if(chap.has(s)){dup+=wc(s);ds++;}}
console.log('appE sentences>=8w',appE.length,'words',tot,'identical to a chapter judge-box sentence:',ds,'sentences',dup,'words');
// within chapters: judge sentences repeated across different boxes
const m=new Map(); for(const f of listFiles()) for(const u of parseFile(f)) if(u.tag.startsWith('ch')) for(const it of splitItems(u.lines)) if(it.type==='judge') for(const s of sents(it.text)){ if(!m.has(s)) m.set(s,new Set()); m.get(s).add(u.id);} 
let c=0,w=0; for(const [s,ids] of m) if(ids.size>1){c++;w+=wc(s)*(ids.size-1);} console.log('chapter judge sentences repeated in >1 section:',c,'extra words',w);
// all prose sentences repeated across units
const all=new Map(); for(const f of listFiles()) for(const u of parseFile(f)) for(const s of sents(u.lines.filter(l=>!/^\|/.test(l)).join('\n'))){ if(!all.has(s)) all.set(s,new Set()); all.get(s).add(u.id);} 
c=0;w=0; for(const [s,ids] of all) if(ids.size>1){c++;w+=wc(s)*(ids.size-1);} console.log('any prose sentence(>=8w) appearing in >1 unit:',c,'extra words',w);
const front=readFileSync('../src/00_FRONT.md','utf8'); 
