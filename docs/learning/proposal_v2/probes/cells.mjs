import { listFiles, parseFile, wc } from '../parse_lib.mjs';
const norm=(s)=>s.toLowerCase().replace(/[`*_]/g,'').replace(/\s+/g,' ').trim();
const us={}; for(const f of listFiles()) for(const u of parseFile(f)) us[u.id]=u;
const cells=(u)=>{const out=[];for(const ln of u.lines){ if(/^\|.*\|\s*$/.test(ln)&&!/^\|[\s:|-]+\|\s*$/.test(ln)) out.push(ln.slice(1,-1).split('|').map(norm));}return out;};
function cmp(a,bIds){ const A=cells(us[a]); const B=bIds.flatMap(i=>cells(us[i])).flat(); const Bset=new Set(B);
  let rows=0,full=0,words=0,fw=0; const missing=[];
  for(const r of A){rows++; const w=wc(r.join(' ')); words+=w; const ok=r.every(c=>!c||Bset.has(c)||[...Bset].some(x=>x.includes(c)&&c.length>3)); if(ok){full++;fw+=w;} else missing.push(r.map(c=>c.slice(0,40)).join(' | '));}
  console.log(a,'rows',rows,'fully-contained rows',full,'words',words,'in contained rows',fw); if(process.argv[2]) console.log(missing.join('\n'));
}
cmp('ch08:8.12',['appG:G.2','appG:G.3','appG:G.4']);
cmp('ch08:8.11',['appE:never-say']);
cmp('appE:never-say',['ch08:8.11']);
cmp('appG:G.2',['ch08:8.12']);cmp('appG:G.3',['ch08:8.12']);cmp('appG:G.4',['ch08:8.12']);
console.log('---- more');
cmp('ch04:4.11',['ch08:8.12','appG:G.2']);
cmp('ch05:5.15',['ch08:8.12','appG:G.2','appG:G.3','appG:G.4']);
cmp('appG:G.1',['ch08:8.12']);
cmp('ch06:6.11',['ch08:8.4']);
