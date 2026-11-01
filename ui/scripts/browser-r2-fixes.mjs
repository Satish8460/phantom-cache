// D-7 round 2 fix checklist (observe-only). Real Chrome over CDP, Node built-ins only.
//   node ui/scripts/browser-r2-fixes.mjs --evidence <dir> --ui http://localhost:3000 --runid <run id> [--t1id <run id>]
// F-01 reflow at 320/390/640/820 (+ mobile:true captures), F-02 menu, F-03 target sizes, F-04 icon size, F-05 sticky nav,
// F-06 focus order, F-10 favicon, F-11 chart caption overlap / table clipping, chart tables, VALID wording.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, ui = 'http://localhost:3000', runid, t1id } = args;
const R = new Results(join(evidence, 'r2_fixes.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r2fix' }).launch();
let cur = { w: 1280, h: 800, m: false, d: 1 }; let loads = 0;
const setVp = (w, h, m = false, d = 1) => { cur = { w, h, m, d }; return b.viewport(w, h, m, d); };
// Chrome allows 6 connections per host; abandoned live streams from earlier cold loads can exhaust them (see r2_stall3.json), so the browser is relaunched every 3 cold loads.
const relaunch = async () => { await b.close(); await b.launch(); await b.media([{ name: 'prefers-color-scheme', value: 'light' }]); await b.viewport(cur.w, cur.h, cur.m, cur.d); };
const M6 = '#/lab/evidence/m6_sealed_2026-10-29T234845Z', SB = '#/lab/evidence/scenario_b.20261030T003932Z';
const ALL = ['#/', '#/traffic', '#/cache', '#/freshness', '#/origin', '#/cluster', '#/intelligence', '#/simulation', '#/lab', '#/lab/evidence', M6, SB, '#/lab/run', '#/lab/runs', `#/lab/runs/${runid}`];
if (t1id) ALL.push(`#/lab/runs/${t1id}`);
const LAB = ALL.filter((r) => r.startsWith('#/lab'));
const rname = (r) => r.replace(/[^a-z0-9]+/gi, '_').replace(/^_|_$/g, '') || 'root';
const load = async (route, ms = 2200) => { if (++loads % 3 === 0) await relaunch(); await b.goto('about:blank', 150); await b.goto(`${ui}/${route}`, ms); if (route.startsWith('#/lab')) { const ok = await b.waitFor('!/Checking|Loading this|Loading the|Waiting for the gateway/.test(document.body.innerText)', 20000, 300); if (!ok) R.add('load.timeout.' + route, 'FAIL', 'page still loading after 20 s'); await sleep(500); } };

const OVERFLOW_JS = `(() => {
  const de = document.documentElement;
  const inScroller = (e) => { for (let p = e.parentElement; p && p !== document.body; p = p.parentElement) { const o = getComputedStyle(p).overflowX; if ((o === 'auto' || o === 'scroll' || o === 'hidden') && p.clientWidth < p.scrollWidth + 1) return true; } return false; };
  const pageW = de.clientWidth;
  const wide = [...document.querySelectorAll('body *')].filter(e => { const r = e.getBoundingClientRect(); return r.right > pageW + 1 && r.width > 0 && getComputedStyle(e).position !== 'fixed' && !inScroller(e); })
    .slice(0, 6).map(e => e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30) + ' right=' + Math.round(e.getBoundingClientRect().right));
  const ownScroll = [...document.querySelectorAll('table,pre,code,.lab-verbatim,[tabindex="0"]')].filter(e => e.scrollWidth > e.clientWidth + 1).length;
  return { scrollWidth: de.scrollWidth, clientWidth: de.clientWidth, innerWidth, overflowX: de.scrollWidth > innerWidth, wide, ownScrollBoxes: ownScroll };
})()`;

try {
  await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  // ---------- F-01 reflow
  const bad = [];
  for (const w of [320, 390, 640, 820]) {
    await setVp(w, w < 700 ? 800 : 1180, false);
    for (const route of ALL) {
      await load(route);
      const o = await b.eval(OVERFLOW_JS);
      const ok = o.scrollWidth <= w && !o.overflowX;
      if (!ok) bad.push(`${w} ${route} sw=${o.scrollWidth} wide=${o.wide.join(';')}`);
      R.add(`F01.reflow.${w}.${rname(route)}`, ok ? 'PASS' : 'FAIL', o);
      if (LAB.includes(route) || route === '#/') await b.shot(`reflow_${w}_${rname(route)}.png`, 'shots/r2layout', true);
    }
  }
  R.add('F01.summary', bad.length ? 'FAIL' : 'PASS', bad);
  // mobile:true captures at 390 and 820 (layout viewport may differ; record both)
  for (const w of [390, 820]) {
    await setVp(w, w === 390 ? 844 : 1180, true, 2);
    for (const route of ['#/lab', M6, SB, '#/lab/runs', `#/lab/runs/${runid}`, '#/lab/run']) {
      await load(route);
      const o = await b.eval(OVERFLOW_JS);
      R.add(`F01.mobile_true.${w}.${rname(route)}`, o.scrollWidth <= o.innerWidth + 1 ? 'PASS' : 'FAIL', o, await b.shot(`mobiletrue_${w}_${rname(route)}.png`, 'shots/r2layout', false));
    }
  }
  // long ids wrap
  await setVp(390, 844, false, 1);
  await load(SB);
  R.add('F01.long_ids_wrap', 'OBSERVED', await b.eval(`[...document.querySelectorAll('body *')].filter(e => e.children.length === 0 && /[0-9a-f]{40,}|scenario_b\\.\\d/.test(e.textContent)).slice(0, 8).map(e => ({ t: e.textContent.trim().slice(0, 40), w: Math.round(e.getBoundingClientRect().width), right: Math.round(e.getBoundingClientRect().right), cls: e.className, wrap: getComputedStyle(e).overflowWrap }))`));

  // ---------- F-02 menu
  await setVp(390, 844, false);
  await load('#/lab');
  const menu = {};
  menu.exists = await b.eval(`(() => { const t = document.querySelector('.app-nav__toggle'); if (!t) return null; const r = t.getBoundingClientRect(); return { text: t.innerText, expanded: t.getAttribute('aria-expanded'), controls: t.getAttribute('aria-controls'), w: Math.round(r.width), h: Math.round(r.height) }; })()`);
  menu.listVisibleClosed = await b.eval(`(() => { const l = document.querySelector('.app-nav ul'); const r = l.getBoundingClientRect(); return r.height > 0 && getComputedStyle(l).display !== 'none'; })()`);
  await b.shot('menu_closed_390.png', 'shots/r2layout');
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(300);
  menu.expandedAfterClick = await b.eval(`document.querySelector('.app-nav__toggle').getAttribute('aria-expanded')`);
  menu.listVisibleOpen = await b.eval(`(() => { const l = document.querySelector('.app-nav ul'); const r = l.getBoundingClientRect(); return r.height > 0 && getComputedStyle(l).display !== 'none'; })()`);
  await b.shot('menu_open_390.png', 'shots/r2layout');
  // Escape (focus moved into the list first)
  await b.eval(`document.querySelector('.app-nav ul a').focus()`);
  await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(300);
  menu.afterEscape = await b.eval(`({ expanded: document.querySelector('.app-nav__toggle').getAttribute('aria-expanded'), focusIsToggle: document.activeElement === document.querySelector('.app-nav__toggle') })`);
  // link choice closes
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(300);
  await b.click(`document.querySelector('.app-nav a[data-screen="traffic"]')`); await sleep(800);
  menu.afterLink = await b.eval(`({ expanded: document.querySelector('.app-nav__toggle').getAttribute('aria-expanded'), hash: location.hash })`);
  // keyboard toggle with Enter / Space
  await b.eval(`document.querySelector('.app-nav__toggle').focus()`);
  await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(250);
  menu.enterToggles = await b.eval(`document.querySelector('.app-nav__toggle').getAttribute('aria-expanded')`);
  const mpass = menu.exists && menu.exists.expanded === 'false' && menu.expandedAfterClick === 'true' && menu.afterEscape.expanded === 'false' && menu.afterEscape.focusIsToggle && menu.afterLink.expanded === 'false' && menu.listVisibleOpen && !menu.listVisibleClosed;
  R.add('F02.menu_below_960', mpass ? 'PASS' : 'FAIL', menu);
  // breakpoint behaviour
  const bp = {};
  for (const w of [959, 960, 1280]) {
    await setVp(w, 800, false); await load('#/lab', 1500);
    bp[w] = await b.eval(`(() => { const t = document.querySelector('.app-nav__toggle'); const r = t ? t.getBoundingClientRect() : null; return { toggleVisible: !!r && r.width > 0 && getComputedStyle(t).display !== 'none', navLinksVisible: [...document.querySelectorAll('.app-nav a')].filter(a => a.getBoundingClientRect().height > 0).length, scrollWidth: document.documentElement.scrollWidth }; })()`);
  }
  R.add('F02.breakpoint', bp[1280].toggleVisible === false ? 'PASS' : 'FAIL', bp);

  // ---------- F-03 target sizes
  const tsum = {};
  for (const [w, minT] of [[390, 44], [1280, 32]]) {
    await setVp(w, 844, false);
    for (const route of LAB) {
      await load(route, 2000);
      const t = await b.eval(`(() => {
        const sel = 'a[href],button,input:not([type=hidden]),select,textarea,summary,[role=button],[role=tab],[tabindex]:not([tabindex="-1"])';
        const nameOf = (e) => (e.innerText || e.getAttribute('aria-label') || e.id || e.type || '').trim().replace(/\\s+/g,' ').slice(0, 32);
        const all = [...document.querySelectorAll(sel)].map(e => { const r = e.getBoundingClientRect(); const cs = getComputedStyle(e); const vis = r.width > 0 && r.height > 0 && cs.visibility !== 'hidden'; const inline = !!e.closest('p,li,td,dd,figcaption') && cs.display.startsWith('inline'); return { vis, n: nameOf(e), tag: e.tagName.toLowerCase(), w: Math.round(r.width), h: Math.round(r.height), inline, radio: e.type === 'radio' || e.type === 'checkbox', lab: !!(e.labels && e.labels.length) }; }).filter(x => x.vis);
        const hmin = (x) => x.h;
        return { count: all.length, lt24: all.filter(x => x.w < 24 || x.h < 24), ltMin: all.filter(x => x.h < ${minT}).map(x => x), hmin: Math.min(...all.map(hmin)) };
      })()`);
      const radios = t.ltMin.filter((x) => x.radio).length;
      tsum[`${w}.${rname(route)}`] = { count: t.count, lt24: t.lt24.length, below: t.ltMin.length, belowExclRadioCheck: t.ltMin.length - radios, minH: t.hmin };
      R.add(`F03.targets.${w}.${rname(route)}`, t.lt24.length === 0 && t.ltMin.length === 0 ? 'PASS' : (t.lt24.length ? 'FAIL' : 'OBSERVED'), { count: t.count, lt24: t.lt24.slice(0, 8), belowMin: t.ltMin.slice(0, 10), belowMinCount: t.ltMin.length, minHeight: t.hmin });
    }
  }
  R.add('F03.summary', Object.values(tsum).some((x) => x.lt24) ? 'FAIL' : 'OBSERVED', tsum);

  // ---------- F-04 icon size, F-05 sticky nav
  await setVp(1920, 1080, false); await load('#/');
  R.add('F04.nav_icon_size', 'OBSERVED', await b.eval(`[...document.querySelectorAll('.app-nav svg')].map(s => { const r = s.getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)]; })`), await b.shot('nav_icon_1920.png', 'shots/r2layout'));
  await setVp(1280, 800, false); await load(M6, 2500);
  const nav0 = await b.eval(`(() => { const r = document.querySelector('.app-nav').getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom), pos: getComputedStyle(document.querySelector('.app-nav')).position }; })()`);
  await b.eval('window.scrollTo(0, 1500)'); await sleep(400);
  const nav1 = await b.eval(`(() => { const r = document.querySelector('.app-nav').getBoundingClientRect(); return { scrollY: Math.round(scrollY), top: Math.round(r.top), bottom: Math.round(r.bottom), inView: r.bottom > 0 && r.top < innerHeight }; })()`);
  R.add('F05.sticky_nav_1280', nav1.inView && nav1.top >= 0 && nav1.top < 200 ? 'PASS' : 'FAIL', { nav0, nav1 }, await b.shot('sticky_nav_scrolled_1280.png', 'shots/r2layout'));

  // ---------- F-06 focus order
  await setVp(1280, 800, false);
  for (const hash of ['#/', '#/lab', '#/lab/run']) {
    await load(hash, 3500);
    const atLoad = await b.eval(`document.activeElement.tagName + (document.activeElement.id ? '#' + document.activeElement.id : '')`);
    await b.tab();
    const first = await b.eval(`(document.activeElement.innerText || document.activeElement.getAttribute('aria-label') || document.activeElement.tagName).trim().slice(0, 40)`);
    R.add(`F06.first_tab.${rname(hash)}`, /skip/i.test(first) ? 'PASS' : 'FAIL', { activeAtLoad: atLoad, firstTab: first }, await b.shot(`first_tab_${rname(hash)}.png`, 'shots/r2layout'));
  }
  // in-app navigation moves focus to heading and scrolls top
  await load(M6, 2500); await b.eval('window.scrollTo(0, 1200)'); await sleep(200);
  await b.click(`document.querySelector('.app-nav a[data-screen="traffic"]')`); await sleep(900);
  R.add('F06.in_app_nav_focus', 'OBSERVED', await b.eval(`({ tag: document.activeElement.tagName, text: document.activeElement.innerText.slice(0, 40), scrollY: Math.round(scrollY), hash: location.hash })`));
  await b.eval('window.scrollTo(0, 800)'); await sleep(200);
  await b.eval(`location.hash = '#/lab/runs'`); await sleep(900);
  R.add('F06.in_app_nav_focus_lab', 'OBSERVED', await b.eval(`({ tag: document.activeElement.tagName, text: document.activeElement.innerText.slice(0, 40), scrollY: Math.round(scrollY) })`));
  const skipTest = await b.eval(`(() => { const a = document.querySelector('.skip-link'); return a ? a.getAttribute('href') : null; })()`);
  await b.key('Tab', { code: 'Tab', vk: 9 }); await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(300);
  R.add('F06.skip_link_activation', 'OBSERVED', { href: skipTest, active: await b.eval(`document.activeElement.tagName + '#' + document.activeElement.id`) });

  // ---------- F-10 favicon
  b.reqById.clear();
  for (const r of ['#/', '#/lab', M6]) await load(r, 1800);
  const fav = [...b.reqById.values()].filter((r) => /favicon/i.test(r.url || ''));
  R.add('F10.favicon', fav.every((r) => r.status !== 404) ? 'PASS' : 'FAIL', { favRequests: fav, linkIcon: await b.eval(`[...document.querySelectorAll('link[rel~=icon]')].map(l => l.getAttribute('href').slice(0, 60))`), fourOhFours: [...b.reqById.values()].filter((r) => r.status === 404).map((r) => r.url) });

  // ---------- F-11 chart caption overlap and table clipping (laptop and phone)
  for (const w of [1280, 390]) {
    await setVp(w, 900, false); await load(M6, 2800);
    const ov = await b.eval(`(() => {
      const leaves = [...document.querySelectorAll('.lab-chart *, [class*=chart] *')].filter(e => e.children.length === 0 && e.textContent.trim() && e.getBoundingClientRect().width > 0);
      const rects = leaves.map(e => ({ t: e.textContent.trim().slice(0, 40), r: e.getBoundingClientRect() }));
      const hits = [];
      for (let i = 0; i < rects.length; i++) for (let j = i + 1; j < rects.length; j++) { const a = rects[i].r, c = rects[j].r;
        const ix = Math.min(a.right, c.right) - Math.max(a.left, c.left), iy = Math.min(a.bottom, c.bottom) - Math.max(a.top, c.top);
        if (ix > 2 && iy > 2) hits.push([rects[i].t, rects[j].t, Math.round(ix), Math.round(iy)]); }
      const clipped = [...document.querySelectorAll('table')].map(t => { const p = t.parentElement; const cs = getComputedStyle(p); return { overflowY: cs.overflowY, clientH: p.clientHeight, scrollH: p.scrollHeight, maxH: cs.maxHeight, cut: cs.overflowY !== 'visible' && p.scrollHeight > p.clientHeight + 1 && cs.overflowY === 'hidden' }; });
      return { leaves: leaves.length, overlaps: hits.slice(0, 10), nOverlaps: hits.length, tables: clipped.length, hiddenCut: clipped.filter(c => c.cut).length, tableBoxes: clipped.slice(0, 6) };
    })()`);
    R.add(`F11.chart_captions_tables.${w}`, ov.nOverlaps === 0 && ov.hiddenCut === 0 ? 'PASS' : 'FAIL', ov);
    await b.shot(`m6_top_${w}.png`, 'shots/r2layout', true);
  }

  // ---------- Chart "View as table": closed by default, opens
  const vat = {};
  await setVp(1280, 900, false);
  for (const route of ['#/', '#/traffic', '#/cache', '#/freshness', '#/origin', '#/simulation', M6, SB, `#/lab/runs/${runid}`, ...(t1id ? [`#/lab/runs/${t1id}`] : [])]) {
    await load(route, 2500);
    const o = await b.eval(`(() => { const ds = [...document.querySelectorAll('details')].filter(d => d.querySelector('summary') && /table/i.test(d.querySelector('summary').innerText));
      const charts = document.querySelectorAll('svg[role=img], .lab-chart, [data-chart]').length;
      return { details: ds.length, openByDefault: ds.filter(d => d.open).length, summaries: ds.slice(0, 3).map(d => d.querySelector('summary').innerText.trim()), charts }; })()`);
    await b.eval(`[...document.querySelectorAll('details')].filter(d => d.querySelector('summary') && /table/i.test(d.querySelector('summary').innerText)).forEach(d => d.querySelector('summary').click())`); await sleep(400);
    o.openAfterClick = await b.eval(`[...document.querySelectorAll('details[open]')].filter(d => /table/i.test(d.querySelector('summary').innerText) && d.querySelector('table')).length`);
    vat[route] = o;
    R.add(`charts.view_as_table.${rname(route)}`, o.details === 0 ? 'OBSERVED' : (o.openByDefault === 0 && o.openAfterClick === o.details ? 'PASS' : 'FAIL'), o, o.details ? await b.shot(`view_as_table_${rname(route)}.png`, 'shots/r2layout', true) : undefined);
  }

  // ---------- VALID wording
  for (const route of [SB, `#/lab/runs/${runid}`]) {
    await load(route, 2500);
    const t = await b.eval('document.body.innerText');
    R.add(`valid_wording.${rname(route)}`, /Run class: VALID \(narrow meaning\)|Run class: EXPLORATORY/.test(t) ? 'OBSERVED' : 'FAIL', { runClassLines: t.match(/Run class[^\n]*/g), narrow: /narrow meaning/.test(t), notSuccess: t.match(/[^\n]*not a success[^\n]*/gi) });
  }
} catch (e) {
  R.add('fixes.exception', 'FAIL', String((e && e.stack) || e));
} finally {
  R.save(); b.saveLogs('r2_fixes');
  const gone = await b.close();
  writeFileSync(join(evidence, 'r2_fixes.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
  console.log(JSON.stringify({ chromePid: b.pid, chromeGone: gone }));
}
