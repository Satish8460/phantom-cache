import { listFiles, parseFile, wc } from '../parse_lib.mjs';
const norm = (s) => s.toLowerCase().replace(/[`*_>#|]/g, ' ').replace(/[^a-z0-9\s]/g,' ').split(/\s+/).filter(Boolean);
const K=8;
const units=[]; for (const f of listFiles()) for (const u of parseFile(f)) units.push(u);
// index: shingle -> set of unit ids (with file tag)
const idx=new Map();
const toks=units.map(u=>norm(u.lines.join('\n')));
units.forEach((u,i)=>{ const t=toks[i]; for(let j=0;j+K<=t.length;j++){const s=t.slice(j,j+K).join(' '); let m=idx.get(s); if(!m){m=new Set();idx.set(s,m);} m.add(i);} });
const rows=[];
units.forEach((u,i)=>{ const t=toks[i]; if(t.length<K) return; let cov=0; const from={};
  const covered=new Array(t.length).fill(false);
  for(let j=0;j+K<=t.length;j++){const s=t.slice(j,j+K).join(' '); const m=idx.get(s); const others=[...m].filter(k=>units[k].tag!==u.tag); if(others.length){ for(let q=j;q<j+K;q++) covered[q]=true; others.forEach(k=>from[units[k].tag]=(from[units[k].tag]||0)+1);} }
  cov=covered.filter(Boolean).length;
  rows.push([u.id,t.length,cov,(100*cov/t.length).toFixed(0)+'%',Object.entries(from).sort((a,b)=>b[1]-a[1]).slice(0,3).map(x=>x.join(':')).join(',')]);
});
rows.filter(r=>r[2]>=15).sort((a,b)=>b[2]-a[2]).forEach(r=>console.log(r.join(' | ')));
