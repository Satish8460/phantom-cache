import { listFiles, parseFile, splitItems, judgeParts, wc } from '../parse_lib.mjs';
let tot=0;
for (const f of listFiles()) for (const u of parseFile(f)) {
  const items = splitItems(u.lines); let j=0,jn=0,t=0,tab=0,figs=0;
  for (const it of items){ if(it.type==='judge'){jn++;j+=wc(it.text)} else t+=wc(it.text)}
  const body=u.lines.join('\n'); tab=(body.match(/^\|.*\|\s*$/gm)||[]).join(' ').split(/\s+/).length; figs=(body.match(/^!\[/gm)||[]).length;
  tot+=wc(body)+wc(u.heading);
  console.log([u.id,u.level,wc(body),'j'+jn+':'+j,'tab'+tab,'fig'+figs,u.heading.slice(0,60)].join(' | '));
}
console.log(tot);
