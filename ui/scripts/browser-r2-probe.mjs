// D-7 round 2 probe: locate the elements that cause page-level horizontal scroll, and check which text keeps a page "loading".
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, sleep } from './cdp-lib.mjs';
const evidence = process.argv[2]; const ui = 'http://localhost:3000';
const b = await new Browser({ evidenceDir: evidence, label: 'r2probe' }).launch();
const out = {};
try {
  const cases = [[390, '#/lab/evidence/scenario_b.20261030T003932Z', 'ABS'], [320, '#/lab/evidence'], [320, '#/lab/run'], [320, '#/lab/runs'], [320, '#/lab/evidence/m6_sealed_2026-10-29T234845Z'], [390, '#/lab/evidence/m6_sealed_2026-10-29T234845Z']];
  for (const [w, route, mode] of cases) {
    await b.viewport(w, 800, false);
    await b.goto('about:blank', 150); await b.goto(`${ui}/${route}`, 4000);
    if (mode === 'ABS') out['ABS ' + w + route] = await b.eval(`[...document.querySelectorAll('body *')].filter(e => getComputedStyle(e).position === 'absolute' && e.getBoundingClientRect().right > document.documentElement.clientWidth + 1).slice(0, 10).map(e => ({ tag: e.tagName.toLowerCase(), cls: String(e.className).slice(0, 30), right: Math.round(e.getBoundingClientRect().right), left: Math.round(e.getBoundingClientRect().left), txt: e.textContent.trim().slice(0, 40), parent: e.parentElement.tagName.toLowerCase() + '.' + String(e.parentElement.className).slice(0, 20), inTableWrap: !!e.closest('.table-wrap') }))`);
    out[`${w} ${route}`] = await b.eval(`(() => {
      const de = document.documentElement; const iw = de.clientWidth;
      const over = [...document.querySelectorAll('body *')].map(e => { const r = e.getBoundingClientRect(); return { tag: e.tagName.toLowerCase(), cls: String(e.className).slice(0, 40), id: e.id || '', right: Math.round(r.right), w: Math.round(r.width), sw: e.scrollWidth, cw: e.clientWidth, ox: getComputedStyle(e).overflowX, pos: getComputedStyle(e).position, inScroll: (() => { for (let q = e.parentElement; q && q !== document.body; q = q.parentElement) { const o = getComputedStyle(q).overflowX; if (o === 'auto' || o === 'scroll') return true; } return false; })(), txt: (e.children.length === 0 ? e.textContent.trim().slice(0, 40) : '') }; })
        .filter(x => x.right > iw + 1 || x.sw > x.cw + 1).filter(x => !x.inScroll).sort((a, c) => c.right - a.right).slice(0, 12);
      return { scrollWidth: de.scrollWidth, clientWidth: iw, bodyScrollWidth: document.body.scrollWidth, over, loadingText: (document.body.innerText.match(/Checking|Loading [a-z ]+|Waiting for the gateway/g) || []).slice(0, 4) };
    })()`);
  }
} finally {
  writeFileSync(join(evidence, 'r2_probe.json'), JSON.stringify(out, null, 1));
  const gone = await b.close(); console.log('chromeGone', gone);
}
