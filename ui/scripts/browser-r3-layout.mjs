// D-7 round 3: reflow (F-01/N-1a/N-1b), zoom, target sizes (N-3), M6 caption (N-4), regression pass. Observe-only.
//   node ui/scripts/browser-r3-layout.mjs --evidence <dir> --runid <sb run id>
import { writeFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, ui = 'http://localhost:3000', runid } = args;
const R = new Results(join(evidence, 'r3_layout.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r3lay' }).launch();
const pidLog = (join(evidence, 'logs', 'chrome_pids.txt'));
appendFileSync(pidLog, `r3_layout ${b.pid}\n`);
let cur = { w: 1280, h: 800, m: false, d: 1, theme: 'light' }; let loads = 0; const timeouts = [];
const setVp = (w, h, m = false, d = 1) => { cur = { ...cur, w, h, m, d }; return b.viewport(w, h, m, d); };
const relaunch = async () => {
  await b.close(); await b.launch(); appendFileSync(pidLog, `r3_layout ${b.pid}\n`);
  await b.media([{ name: 'prefers-color-scheme', value: cur.theme }]); await b.viewport(cur.w, cur.h, cur.m, cur.d);
};
const M6 = '#/lab/evidence/m6_sealed_2026-10-29T234845Z', SB = '#/lab/evidence/scenario_b.20261030T003932Z';
const NONLAB = ['#/', '#/traffic', '#/cache', '#/freshness', '#/origin', '#/cluster', '#/intelligence', '#/simulation'];
const LAB = ['#/lab', '#/lab/evidence', M6, SB, '#/lab/run', '#/lab/runs', `#/lab/runs/${runid}`];
const ALL = [...NONLAB, ...LAB];
const rname = (r) => r.replace(/[^a-z0-9]+/gi, '_').replace(/^_|_$/g, '') || 'root';
const load = async (route, ms = 1800) => {
  if (++loads % 3 === 0) await relaunch();
  await b.goto('about:blank', 150); await b.goto(`${ui}/${route}`, ms);
  if (route.startsWith('#/lab')) {
    const ok = await b.waitFor('!/Checking|Loading this|Loading the|Waiting for the gateway/.test(document.body.innerText)', 20000, 300);
    if (!ok) { timeouts.push(route); R.add('load.timeout.' + route, 'FAIL', 'still loading after 20 s'); }
    await sleep(500);
  }
};
const OVERFLOW_JS = `(() => {
  const de = document.documentElement;
  const inScroller = (e) => { for (let p = e.parentElement; p && p !== document.body; p = p.parentElement) { const o = getComputedStyle(p).overflowX; if ((o === 'auto' || o === 'scroll' || o === 'hidden') && p.clientWidth < p.scrollWidth + 1) return true; } return false; };
  const pageW = de.clientWidth;
  const wide = [...document.querySelectorAll('body *')].filter(e => { const r = e.getBoundingClientRect(); return r.right > pageW + 1 && r.width > 0 && getComputedStyle(e).position !== 'fixed' && !inScroller(e); })
    .slice(0, 6).map(e => e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30) + ' right=' + Math.round(e.getBoundingClientRect().right) + ' "' + (e.textContent||'').trim().slice(0,30) + '"');
  const banner = [...document.querySelectorAll('body *')].filter(e => e.children.length === 0 && /debug/i.test(e.textContent) && /phantom-cache-demo|_m7_target|[.]exe/i.test(e.textContent)).slice(0,1).map(e => ({ t: e.textContent.trim().slice(0,90), right: Math.round(e.getBoundingClientRect().right), w: Math.round(e.getBoundingClientRect().width) }));
  return { scrollWidth: de.scrollWidth, clientWidth: de.clientWidth, innerWidth, overflowX: de.scrollWidth > innerWidth, wide, bannerVisible: banner.length > 0, banner };
})()`;
const TARGETS_JS = (minT) => `(() => {
  const sel = 'a[href],button,input:not([type=hidden]),select,textarea,summary,[role=button],[role=tab],[tabindex]:not([tabindex="-1"])';
  const nameOf = (e) => (e.innerText || e.getAttribute('aria-label') || e.id || e.type || '').trim().replace(/\\s+/g,' ').slice(0, 40);
  const all = [...document.querySelectorAll(sel)].map(e => { const r = e.getBoundingClientRect(); const cs = getComputedStyle(e);
    const vis = r.width > 0 && r.height > 0 && cs.visibility !== 'hidden';
    const isCheck = e.tagName === 'INPUT' && (e.type === 'radio' || e.type === 'checkbox');
    const lab = isCheck ? (e.closest('label') || (e.labels && e.labels[0])) : null; const lr = lab ? lab.getBoundingClientRect() : null;
    return { vis, n: nameOf(e) || e.tagName, tag: e.tagName.toLowerCase() + (e.type ? ':' + e.type : ''), w: Math.round(r.width), h: Math.round(r.height), labelW: lr ? Math.round(lr.width) : null, labelH: lr ? Math.round(lr.height) : null, summary: e.tagName === 'SUMMARY', inlineLink: e.tagName === 'A' && !!e.closest('p,li,td,dd,figcaption') && cs.display.startsWith('inline') }; }).filter(x => x.vis);
  const eff = (x) => (x.labelH ? Math.max(x.h, x.labelH) : x.h);
  return { count: all.length, lt24: all.filter(x => x.h < 24 || x.w < 24).slice(0, 40),
    ltMin: all.filter(x => x.h < ${minT}).slice(0, 60), summaries: all.filter(x => x.summary).map(x => x.h),
    checks: all.filter(x => x.labelH).map(x => ({ n: x.n, ctl: x.w + 'x' + x.h, label: x.labelW + 'x' + x.labelH })).slice(0, 4),
    checkCount: all.filter(x => x.labelH).length, minH: Math.min(...all.map(x => x.h)), minLabelEff: Math.min(...all.filter(x => x.labelH).map(eff), 9999) };
})()`;

try {
  await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  // ---- reflow
  const bad = [];
  for (const w of [320, 390, 640, 820, 1280]) {
    await setVp(w, w < 700 ? 800 : 900, false);
    for (const route of ALL) {
      await load(route);
      const o = await b.eval(OVERFLOW_JS);
      const ok = o.scrollWidth <= w && !o.overflowX;
      if (!ok) bad.push(`${w} ${route} sw=${o.scrollWidth}`);
      R.add(`reflow.${w}.${rname(route)}`, ok ? 'PASS' : 'FAIL', o);
      if (w === 320 || w === 390) await b.shot(`reflow_${w}_${rname(route)}.png`, 'shots/r3layout', true);
    }
  }
  R.add('reflow.summary', bad.length ? 'FAIL' : 'PASS', bad);
  // ---- zoom emulation
  for (const [name, w, h] of [['zoom200', 640, 400], ['zoom400', 320, 256]]) {
    await setVp(w, h, false);
    for (const route of ALL) {
      await load(route);
      const o = await b.eval(OVERFLOW_JS);
      R.add(`${name}.${rname(route)}`, o.scrollWidth <= w ? 'PASS' : 'FAIL', o);
      if (route === '#/lab') await b.shot(`${name}_lab.png`, 'shots/r3layout', false);
    }
  }
  // ---- N-3 targets
  for (const [w, minT] of [[390, 44], [1280, 32]]) {
    await setVp(w, 844, false);
    for (const route of ALL) {
      await load(route, 1800);
      const t = await b.eval(TARGETS_JS(minT));
      R.add(`targets.${w}.${rname(route)}`, t.lt24.length ? 'FAIL' : (t.ltMin.length ? 'OBSERVED' : 'PASS'), { count: t.count, minH: t.minH, lt24: t.lt24.slice(0, 12), belowMinCount: t.ltMin.length, belowMin: t.ltMin.slice(0, 25), summaryHeights: [...new Set(t.summaries)], checkCount: t.checkCount, checkSample: t.checks, minCheckLabelEff: t.minLabelEff });
    }
  }
  // ---- N-4 M6 caption
  for (const theme of ['light', 'dark']) {
    cur.theme = theme; await b.media([{ name: 'prefers-color-scheme', value: theme }]);
    await setVp(1280, 900, false); await load(M6, 2500);
    const cap = await b.eval(`[...document.querySelectorAll('figcaption, [class*=caption]')].map(e => { const r = e.getBoundingClientRect(); const p = e.parentElement.getBoundingClientRect(); return { tag: e.tagName, cls: String(e.className).slice(0,40), w: Math.round(r.width), parentW: Math.round(p.width), h: Math.round(r.height), text: e.innerText.slice(0, 60) }; })`);
    R.add(`N4.caption.${theme}`, 'OBSERVED', cap, await b.shot(`m6_${theme}_1280.png`, 'shots/r3layout', true));
    await b.shot(`m6_${theme}_1280_viewport.png`, 'shots/r3layout', false);
  }
  cur.theme = 'light'; await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  // ---- regression: skip link first Tab
  await setVp(1280, 800, false);
  for (const hash of ['#/', '#/lab', '#/lab/run']) {
    await load(hash, 2500);
    await b.tab();
    const first = await b.eval(`(document.activeElement.innerText || document.activeElement.tagName).trim().slice(0, 40)`);
    R.add(`reg.first_tab.${rname(hash)}`, /skip/i.test(first) ? 'PASS' : 'FAIL', first);
  }
  // menu open / Escape / Enter
  await setVp(390, 844, false); await load('#/lab');
  const m = {};
  m.closed = await b.eval(`document.querySelector('.app-nav__toggle').getAttribute('aria-expanded')`);
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(300);
  m.open = await b.eval(`document.querySelector('.app-nav__toggle').getAttribute('aria-expanded')`);
  await b.eval(`document.querySelector('.app-nav ul a').focus()`); await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(300);
  m.afterEsc = await b.eval(`({ e: document.querySelector('.app-nav__toggle').getAttribute('aria-expanded'), focusToggle: document.activeElement === document.querySelector('.app-nav__toggle') })`);
  await b.eval(`document.querySelector('.app-nav__toggle').focus()`);
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); await sleep(300);
  m.afterEnter = await b.eval(`document.querySelector('.app-nav__toggle').getAttribute('aria-expanded')`);
  R.add('reg.menu', m.closed === 'false' && m.open === 'true' && m.afterEsc.e === 'false' && m.afterEsc.focusToggle ? 'PASS' : 'FAIL', m);
  // favicon
  await setVp(1280, 800, false); b.reqById.clear();
  for (const r of ['#/', '#/lab']) await load(r, 1500);
  const fav = [...b.reqById.values()].filter((r) => /favicon/i.test(r.url || ''));
  R.add('reg.favicon', [...b.reqById.values()].every((r) => r.status !== 404) ? 'PASS' : 'FAIL', { favRequests: fav.length, notFound: [...b.reqById.values()].filter((r) => r.status === 404).map((r) => r.url) });
  // view as table
  for (const route of ['#/traffic', M6, SB, `#/lab/runs/${runid}`]) {
    await load(route, 2500);
    const o = await b.eval(`(() => { const ds = [...document.querySelectorAll('details')].filter(d => d.querySelector('summary') && /table/i.test(d.querySelector('summary').innerText)); return { details: ds.length, openByDefault: ds.filter(d => d.open).length }; })()`);
    await b.eval(`[...document.querySelectorAll('details')].filter(d => d.querySelector('summary') && /table/i.test(d.querySelector('summary').innerText)).forEach(d => d.querySelector('summary').click())`); await sleep(400);
    o.openAfterClick = await b.eval(`[...document.querySelectorAll('details[open]')].filter(d => /table/i.test(d.querySelector('summary').innerText) && d.querySelector('table')).length`);
    R.add(`reg.view_as_table.${rname(route)}`, o.details === 0 ? 'OBSERVED' : (o.openByDefault === 0 && o.openAfterClick === o.details ? 'PASS' : 'FAIL'), o);
  }
  R.add('load.timeouts', timeouts.length ? 'FAIL' : 'PASS', timeouts);
} catch (e) { R.add('exception', 'FAIL', String((e && e.stack) || e)); } finally {
  R.save(); b.saveLogs('r3_layout'); const gone = await b.close();
  writeFileSync(join(evidence, 'r3_layout.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version, relaunches: Math.floor(loads / 3) }));
}
