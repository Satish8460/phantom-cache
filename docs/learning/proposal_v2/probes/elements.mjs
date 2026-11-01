import { listFiles, parseFile, splitItems, judgeParts, wc } from '../parse_lib.mjs';
const T={}; const add=(k,tag,w)=>{T[k]??={};T[k][tag]=(T[k][tag]||0)+w; T[k].ALL=(T[k].ALL||0)+w;};
for (const f of listFiles()) for (const u of parseFile(f)) {
  const tag=u.tag.startsWith('ch')?u.tag:'app';
  const items=splitItems(u.lines);
  for(const it of items){
    if(it.type==='judge'){const p=judgeParts(it.text); add('judge:question',tag,p.q);add('judge:suggested',tag,p.answer);add('judge:further',tag,p.further);add('judge:evidence',tag,p.evidence); add('judge:count',tag,1);add('judge:other(header/labels)',tag,p.total-p.q-p.answer-p.further-p.evidence);}
    else{
      let fence=false,div=null,buf=[];
      for(const ln of it.text.split('\n')){
        if(/^```/.test(ln)){fence=!fence;add('code',tag,wc(ln));continue;}
        if(fence){add('code',tag,wc(ln));continue;}
        const d=ln.match(/^::: *(\w+)/); if(d){div=d[1];continue;} if(/^:::\s*$/.test(ln)){div=null;continue;}
        const w=wc(ln); if(!w) continue;
        if(/^\|/.test(ln)) add('tables',tag,w); else if(/^!\[/.test(ln)) add('figure captions',tag,w);
        else if(div) add('div:'+div,tag,w); else add('prose/lists',tag,w);
      }
    }
  }
}
const tags=['ch01','ch02','ch03','ch04','ch05','ch06','ch07','ch08','app','ALL'];
console.log(['element',...tags].join(' | '));
for(const k of Object.keys(T).sort()) console.log([k,...tags.map(t=>T[k][t]||0)].join(' | '));
