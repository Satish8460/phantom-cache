// D-7 accessibility / robustness probe (observe-only). Real Chrome over CDP, Node built-ins only.
//   node ui/scripts/browser-a11y.mjs --evidence <dir> --ui http://localhost:3000
// Covers: keyboard traversal + measured focus indicators, Enter/Space on buttons, landmarks/headings/live regions,
// accessible names, target sizes, reduced motion, forced colors, 200%/400% reflow, light/dark parity, in-page
// text contrast (solid backgrounds only; gradients/images are reported as indeterminate), console/network health,
// and load-time / transferred-bytes observations. A pass is an observation at the sizes tested, not a conformance claim.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const evidence = args.evidence; const ui = args.ui || 'http://localhost:3000';
const R = new Results(join(evidence, 'a11y.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'a11y' }).launch();
const ROUTES = ['#/', '#/lab', '#/lab/evidence', '#/lab/evidence/m6_sealed_2026-10-29T234845Z', '#/lab/evidence/scenario_b.20261030T003932Z', '#/lab/run', '#/lab/runs'];
const rname = (r) => r.replace(/[^a-z0-9]+/gi, '_').replace(/^_|_$/g, '') || 'root';
const load = async (route, ms = 2500) => { await b.goto(`${ui}/${route}`, ms); };

const NAME_FN = `(el) => {
  const t = (s) => (s || '').replace(/\\s+/g, ' ').trim();
  const lb = el.getAttribute('aria-labelledby');
  if (lb) { const s = lb.split(/\\s+/).map(i => document.getElementById(i)?.textContent || '').join(' '); if (t(s)) return t(s); }
  if (el.getAttribute('aria-label')) return t(el.getAttribute('aria-label'));
  if (el.labels && el.labels.length) return t([...el.labels].map(l => l.textContent).join(' '));
  if (el.tagName === 'IMG') return t(el.getAttribute('alt'));
  if (el.tagName === 'SVG' || el.tagName === 'svg') return t(el.querySelector('title')?.textContent);
  return t(el.innerText || el.textContent) || t(el.getAttribute('title')) || t(el.getAttribute('placeholder'));
}`;

const CONTRAST_JS = `(() => {
  const parse = (s) => {
    let m = /^rgba?\\(([^)]+)\\)$/.exec(s);
    if (m) { const p = m[1].split(/[ ,\\/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p[3] === undefined ? 1 : p[3] }; }
    m = /^color\\(srgb ([^)]+)\\)$/.exec(s);
    if (m) { const p = m[1].split(/[ \\/]+/).filter(Boolean).map(Number); return { r: p[0]*255, g: p[1]*255, b: p[2]*255, a: p[3] === undefined ? 1 : p[3] }; }
    return null;
  };
  const lin = (c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); };
  const L = (c) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  const over = (f, bg) => ({ r: f.r * f.a + bg.r * (1 - f.a), g: f.g * f.a + bg.g * (1 - f.a), b: f.b * f.a + bg.b * (1 - f.a), a: 1 });
  const bgOf = (el) => {
    const layers = []; let indeterminate = false;
    for (let e = el; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.backgroundImage && cs.backgroundImage !== 'none') { indeterminate = 'background-image on ' + e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30); break; }
      const c = parse(cs.backgroundColor); if (!c) { indeterminate = 'unparsed bg ' + cs.backgroundColor; break; }
      if (c.a > 0) layers.push(c);
      if (c.a >= 1) break;
    }
    let base = { r: 255, g: 255, b: 255, a: 1 };
    if (!layers.length || layers[layers.length - 1].a < 1) { const rootBg = parse(getComputedStyle(document.documentElement).backgroundColor); const bodyBg = parse(getComputedStyle(document.body).backgroundColor); base = (bodyBg && bodyBg.a > 0 ? bodyBg : rootBg && rootBg.a > 0 ? rootBg : base); }
    let acc = base; for (let i = layers.length - 1; i >= 0; i--) acc = over(layers[i], acc);
    return { color: acc, indeterminate };
  };
  const out = { checked: 0, indeterminate: 0, fails: [], minRatio: 99 };
  const seen = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n; (n = walker.nextNode());) {
    const txt = n.nodeValue.trim(); if (!txt) continue;
    const el = n.parentElement; if (!el || ['SCRIPT', 'STYLE', 'NOSCRIPT'].includes(el.tagName)) continue;
    const cs = getComputedStyle(el); if (cs.visibility === 'hidden' || cs.display === 'none') continue;
    const r = el.getBoundingClientRect(); if (r.width === 0 || r.height === 0) continue;
    let op = 1; for (let e = el; e; e = e.parentElement) op *= parseFloat(getComputedStyle(e).opacity);
    const fg0 = parse(cs.color); if (!fg0) { out.indeterminate++; continue; }
    const bg = bgOf(el); if (bg.indeterminate) { out.indeterminate++; const k = 'ind:' + bg.indeterminate; if (!seen.has(k)) { seen.add(k); (out.indeterminateReasons ||= []).push(bg.indeterminate); } continue; }
    const fg = over({ ...fg0, a: fg0.a * op }, bg.color);
    const l1 = L(fg), l2 = L(bg.color); const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
    const size = parseFloat(cs.fontSize); const bold = parseInt(cs.fontWeight, 10) >= 700;
    const large = size >= 24 || (size >= 18.66 && bold); const need = large ? 3 : 4.5;
    out.checked++; out.minRatio = Math.min(out.minRatio, ratio);
    if (ratio < need) {
      const key = el.tagName + '|' + String(el.className).slice(0, 40) + '|' + txt.slice(0, 30);
      if (!seen.has(key)) { seen.add(key); out.fails.push({ ratio: +ratio.toFixed(2), need, text: txt.slice(0, 50), tag: el.tagName.toLowerCase(), cls: String(el.className).slice(0, 50), size, fg: cs.color, bg: 'rgb(' + [bg.color.r, bg.color.g, bg.color.b].map(Math.round).join(',') + ')' }); }
    }
  }
  out.minRatio = +out.minRatio.toFixed(2);
  return out;
})()`;

try {
  // ---------------- Supplemental top-of-page full-page screenshots (browser-verify captures after 5 Tab presses, which scroll the page)
  const SHOT_ROUTES = ['#/', '#/traffic', '#/cache', '#/freshness', '#/origin', '#/lab', '#/lab/evidence', '#/lab/evidence/m6_sealed_2026-10-29T234845Z', '#/lab/evidence/scenario_b.20261030T003932Z', '#/lab/run', '#/lab/runs'];
  const VPS = [{ n: 'mobile', w: 390, h: 844, m: false }, { n: 'tablet', w: 820, h: 1180, m: false }, { n: 'laptop', w: 1280, h: 800, m: false }, { n: 'desktop', w: 1920, h: 1080, m: false }];
  const overflowByVp = {};
  for (const vp of VPS) {
    await b.viewport(vp.w, vp.h, vp.m);
    for (const theme of ['light', 'dark']) {
      await b.media([{ name: 'prefers-color-scheme', value: theme }]);
      for (const route of SHOT_ROUTES) {
        await load(route, 2300);
        await b.eval('window.scrollTo(0,0)');
        const o = await b.eval(`({ sw: document.documentElement.scrollWidth, iw: innerWidth, widest: [...document.querySelectorAll('body *')].map(e => [Math.round(e.getBoundingClientRect().right), e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 28)]).sort((a, c) => c[0] - a[0]).slice(0, 2) })`);
        (overflowByVp[`${vp.n}_${theme}`] ||= {})[route] = o;
        await b.shot(`top_${vp.n}_${theme}_${rname(route)}.png`, 'shots/top', true);
      }
    }
  }
  R.add('overflow.layout_viewport_unemulated_mobile', Object.values(overflowByVp).some((v) => Object.values(v).some((o) => o.sw > o.iw + 1)) ? 'FAIL' : 'PASS', overflowByVp);

  // ---------------- Contrast + landmarks + names + targets + parity, laptop & mobile, both themes
  const parity = {};
  for (const theme of ['light', 'dark']) {
    for (const vp of [{ n: 'laptop', w: 1280, h: 800, m: false }, { n: 'mobile', w: 390, h: 844, m: false }]) {
      await b.viewport(vp.w, vp.h, vp.m);
      await b.media([{ name: 'prefers-color-scheme', value: theme }, { name: 'prefers-reduced-motion', value: 'no-preference' }]);
      for (const route of ROUTES) {
        await load(route);
        const id = `${vp.n}_${theme}_${rname(route)}`;
        const contrast = await b.eval(CONTRAST_JS);
        R.add(`contrast.${id}`, contrast.fails.length ? 'FAIL' : 'PASS', { checked: contrast.checked, indeterminate: contrast.indeterminate, indeterminateReasons: contrast.indeterminateReasons, minRatio: contrast.minRatio, fails: contrast.fails.slice(0, 12), failCount: contrast.fails.length });
        const dom = await b.eval(`(() => {
          const nameOf = ${NAME_FN};
          const sel = 'a[href],button,input,select,textarea,summary,[role=button],[role=link],[role=tab],[tabindex]:not([tabindex="-1"])';
          const els = [...document.querySelectorAll(sel)].filter(e => { const r = e.getBoundingClientRect(); const cs = getComputedStyle(e); return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden'; });
          const unnamed = els.filter(e => !nameOf(e)).map(e => e.outerHTML.slice(0, 120));
          const sizes = els.map(e => { const r = e.getBoundingClientRect(); return { w: Math.round(r.width), h: Math.round(r.height), n: nameOf(e).slice(0, 30), tag: e.tagName.toLowerCase() }; });
          const lt24 = sizes.filter(s => s.w < 24 || s.h < 24), lt44 = sizes.filter(s => s.w < 44 || s.h < 44);
          const heads = [...document.querySelectorAll('h1,h2,h3,h4,h5,h6')].filter(h => h.getBoundingClientRect().width > 0).map(h => ({ l: +h.tagName[1], t: h.textContent.trim().slice(0, 50) }));
          const jumps = heads.filter((h, i) => i > 0 && h.l - heads[i - 1].l > 1).map(h => h.t);
          const lm = {}; for (const [k, s] of [['banner', 'header,[role=banner]'], ['nav', 'nav,[role=navigation]'], ['main', 'main,[role=main]'], ['contentinfo', 'footer,[role=contentinfo]'], ['complementary', 'aside,[role=complementary]'], ['region', '[role=region],section[aria-label],section[aria-labelledby]']]) lm[k] = document.querySelectorAll(s).length;
          const live = [...document.querySelectorAll('[aria-live],[role=status],[role=alert],[role=log],[role=progressbar],progress')].map(e => ({ tag: e.tagName.toLowerCase(), role: e.getAttribute('role'), live: e.getAttribute('aria-live'), text: (e.innerText||'').trim().slice(0, 50) }));
          const skip = [...document.querySelectorAll('a[href^="#"]')].find(a => /skip/i.test(a.textContent));
          const text = document.body.innerText;
          return { focusables: els.length, unnamed, lt24: lt24.length, lt44: lt44.length, lt24List: lt24.slice(0, 8), lt44Sample: lt44.slice(0, 8), h1: heads.filter(h => h.l === 1).length, heads, headingJumps: jumps, landmarks: lm, live, skipLink: skip ? skip.textContent.trim() : null, overflowX: document.documentElement.scrollWidth > innerWidth + 1, scrollW: document.documentElement.scrollWidth, innerW: innerWidth, textLen: text.length, lang: document.documentElement.lang, title: document.title };
        })()`);
        R.add(`structure.${id}`, 'OBSERVED', { unnamed: dom.unnamed, h1: dom.h1, headingJumps: dom.headingJumps, landmarks: dom.landmarks, skipLink: dom.skipLink, live: dom.live.length, lang: dom.lang, title: dom.title });
        R.add(`targets.${id}`, dom.lt24 ? 'FAIL' : 'PASS', { focusables: dom.focusables, below24: dom.lt24, below44: dom.lt44, below24List: dom.lt24List, below44Sample: dom.lt44Sample });
        R.add(`names.${id}`, dom.unnamed.length ? 'FAIL' : 'PASS', dom.unnamed);
        R.add(`overflow.${id}`, dom.overflowX ? 'FAIL' : 'PASS', { scrollW: dom.scrollW, innerW: dom.innerW });
        (parity[route] ||= {})[`${vp.n}_${theme}`] = { textLen: dom.textLen, overflow: dom.overflowX, focusables: dom.focusables, minRatio: contrast.minRatio, fails: contrast.fails.length };
        if (vp.n === 'laptop' && theme === 'dark' && route === '#/lab/run') R.items.at(-1).heads = dom.heads;
      }
    }
  }
  const parityBad = Object.entries(parity).filter(([, v]) => v.laptop_light.textLen !== v.laptop_dark.textLen || v.laptop_light.focusables !== v.laptop_dark.focusables);
  R.add('parity.light_dark', parityBad.length ? 'FAIL' : 'PASS', parity);

  // ---------------- Keyboard traversal (laptop, light), per Lab route
  await b.viewport(1280, 800, false);
  await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  for (const route of ['#/', '#/lab', '#/lab/evidence', '#/lab/run', '#/lab/runs']) {
    await load(route, 2200);
    await b.eval(`document.activeElement && document.activeElement.blur(); window.scrollTo(0,0)`);
    const trail = []; const maxTabs = 45;
    await b.eval(`(${NAME_FN}) && (window.__nameOf = ${NAME_FN})`);
    for (let i = 0; i < maxTabs; i++) {
      await b.tab();
      const f = await b.eval(`(() => { const a = document.activeElement; if (!a || a === document.body) return { body: true };
        const cs = getComputedStyle(a); const r = a.getBoundingClientRect();
        const ow = parseFloat(cs.outlineWidth) || 0; const hasOutline = cs.outlineStyle !== 'none' && ow > 0; const hasShadow = cs.boxShadow && cs.boxShadow !== 'none';
        const vis = r.width > 0 && r.height > 0 && r.top >= 0 && r.bottom <= innerHeight + 1;
        return { tag: a.tagName.toLowerCase(), role: a.getAttribute('role'), name: window.__nameOf(a).slice(0, 40), type: a.type || null,
          outline: cs.outlineStyle + ' ' + cs.outlineWidth + ' ' + cs.outlineColor, outlineOffset: cs.outlineOffset, boxShadow: (cs.boxShadow||'').slice(0, 70), indicator: hasOutline || hasShadow, inViewport: vis,
          rect: [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)], tabindex: a.getAttribute('tabindex') }; })()`);
      trail.push(f);
      if (i === 0 || i === 4) await b.shot(`kbd_${rname(route)}_tab${i + 1}.png`, 'shots');
      if (i > 3 && trail.slice(0, -1).some((t) => t.name && t.name === f.name && t.rect && f.rect && t.rect.join() === f.rect.join())) break; // cycled
    }
    const noIndicator = trail.filter((t) => !t.body && !t.indicator);
    const notInView = trail.filter((t) => !t.body && !t.inViewport).length;
    R.add(`keyboard.tab_order.${rname(route)}`, noIndicator.length ? 'FAIL' : 'PASS', { tabs: trail.length, first: trail[0], withoutIndicator: noIndicator.map((t) => t.name), focusedButNotInViewport: notInView, trail: trail.map((t) => (t.body ? 'BODY' : `${t.tag}${t.role ? '[' + t.role + ']' : ''}:${t.name}`)) });
    // reverse tab check
    await b.tab(true); await sleep(100);
    R.add(`keyboard.shift_tab.${rname(route)}`, 'OBSERVED', await b.eval(`document.activeElement ? (document.activeElement.innerText||document.activeElement.getAttribute('aria-label')||document.activeElement.tagName).slice(0,40) : null`));
  }
  // skip link: first Tab on the overview, Enter, see where focus lands
  await load('#/lab', 2200); await b.eval(`document.activeElement && document.activeElement.blur()`);
  await b.tab(); const first = await b.eval(`document.activeElement ? { text: document.activeElement.innerText.trim(), href: document.activeElement.getAttribute('href') } : null`);
  const isSkip = first && /skip/i.test(first.text);
  if (isSkip) { await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(300); }
  R.add('keyboard.skip_link', isSkip ? 'PASS' : 'FAIL', { first, afterEnterFocus: isSkip ? await b.eval(`({ tag: document.activeElement.tagName, id: document.activeElement.id, hash: location.hash })`) : 'no skip link as first tab stop' }, await b.shot('skip_link_focus.png', 'shots'));
  // Enter / Space on a link and a button; arrow keys in the nav
  await load('#/lab', 2000);
  await b.eval(`window.__clicks = 0; document.addEventListener('click', () => window.__clicks++, true); document.activeElement && document.activeElement.blur();`);
  const ev = await b.eval(`(() => { const a = [...document.querySelectorAll('a[data-lab-view]')].find(x => x.dataset.labView === 'runs'); a.focus(); return a.textContent; })()`);
  await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(900);
  R.add('keyboard.enter_on_link', (await b.eval('location.hash')).includes('runs') ? 'PASS' : 'FAIL', { link: ev, hash: await b.eval('location.hash') });
  await load('#/lab/run', 2000);
  await b.eval(`window.__clicks = 0; document.addEventListener('click', () => window.__clicks++, true);`);
  await b.eval(`(() => { const t = [...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input'); t.focus(); })()`);
  await b.key(' ', { code: 'Space', vk: 32 }); await sleep(400);
  const radioSb = await b.eval(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input').checked`);
  R.add('keyboard.space_on_radio', radioSb ? 'PASS' : 'FAIL', { checked: radioSb });
  const btnTest = await b.eval(`(() => { const btn = [...document.querySelectorAll('button')].find(x => /Copy JSON/i.test(x.innerText)); if (!btn) return 'no Copy JSON button'; btn.focus(); window.__clicks = 0; return 'focused'; })()`);
  await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(300); const c1 = await b.eval('window.__clicks');
  await b.eval(`window.__clicks = 0`); await b.key(' ', { code: 'Space', vk: 32 }); await sleep(300); const c2 = await b.eval('window.__clicks');
  R.add('keyboard.enter_space_on_button', c1 > 0 && c2 > 0 ? 'PASS' : 'FAIL', { btnTest, clicksAfterEnter: c1, clicksAfterSpace: c2 });
  // Lab sub-navigation arrow keys (nav roving focus exists in routes.js nextNavIndex)
  await load('#/', 2000);
  await b.eval(`(() => { const n = document.querySelector('nav a, nav button'); if (n) n.focus(); })()`);
  const f0 = await b.eval(`document.activeElement.innerText.trim()`);
  await b.key('ArrowDown', { code: 'ArrowDown', vk: 40 }); await sleep(200);
  const f1 = await b.eval(`document.activeElement.innerText.trim()`);
  R.add('keyboard.nav_arrow_keys', f0 !== f1 ? 'PASS' : 'OBSERVED', { before: f0, afterArrowDown: f1 });

  // ---------------- Reduced motion
  for (const pref of ['no-preference', 'reduce']) {
    await b.media([{ name: 'prefers-reduced-motion', value: pref }]);
    await load('#/lab', 1800);
    const motion = await b.eval(`(() => { let anim = 0, trans = 0, infinite = 0; const names = new Set();
      for (const e of document.querySelectorAll('*')) { const cs = getComputedStyle(e);
        if (cs.animationName !== 'none' && parseFloat(cs.animationDuration) > 0.01) { anim++; names.add(cs.animationName); if (cs.animationIterationCount === 'infinite') infinite++; }
        const d = cs.transitionDuration.split(',').map(parseFloat); if (d.some(x => x > 0.01)) trans++; }
      return { animated: anim, infinite, transitioning: trans, names: [...names], scrollBehaviorSmooth: getComputedStyle(document.documentElement).scrollBehavior }; })()`);
    R.add(`motion.${pref}`, pref === 'reduce' ? (motion.animated === 0 && motion.transitioning === 0 ? 'PASS' : 'FAIL') : 'OBSERVED', motion, pref === 'reduce' ? await b.shot('reduced_motion_lab.png', 'shots') : undefined);
  }
  await b.media([{ name: 'prefers-reduced-motion', value: 'no-preference' }]);

  // ---------------- Forced colors
  await b.media([{ name: 'forced-colors', value: 'active' }]);
  for (const route of ['#/lab', '#/lab/run', '#/lab/evidence/scenario_b.20261030T003932Z']) {
    await load(route, 2000);
    const fc = await b.eval(`({ matches: matchMedia('(forced-colors: active)').matches, textOnly: document.body.innerText.length, buttonBorders: [...document.querySelectorAll('button')].slice(0,3).map(x => getComputedStyle(x).borderStyle + ' ' + getComputedStyle(x).borderTopWidth) })`);
    R.add(`forced_colors.${rname(route)}`, fc.matches ? 'OBSERVED' : 'FAIL', fc, await b.shot(`forced_colors_${rname(route)}.png`, 'shots'));
  }
  await b.media([{ name: 'forced-colors', value: 'none' }]);

  // ---------------- Reflow: 200% zoom (640x400 CSS px at DPR 2) and 400% (320x256)
  for (const z of [{ n: 'zoom200', w: 640, h: 400, d: 2 }, { n: 'zoom400', w: 320, h: 256, d: 4 }]) {
    await b.viewport(z.w, z.h, false, z.d);
    for (const route of ['#/lab', '#/lab/run', '#/lab/evidence/scenario_b.20261030T003932Z', '#/lab/runs']) {
      await load(route, 2000);
      const o = await b.eval(`({ scrollW: document.documentElement.scrollWidth, innerW: innerWidth, overflowX: document.documentElement.scrollWidth > innerWidth + 1, wide: [...document.querySelectorAll('body *')].filter(e => e.getBoundingClientRect().right > innerWidth + 2 && getComputedStyle(e).position !== 'fixed' && !e.closest('.table-wrap,[tabindex="0"],pre,.lab-verbatim,[class*=scroll]')).slice(0, 5).map(e => e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30)) })`);
      R.add(`reflow.${z.n}.${rname(route)}`, o.overflowX ? (o.wide.length ? 'FAIL' : 'OBSERVED') : 'PASS', o, await b.shot(`${z.n}_${rname(route)}.png`, 'shots'));
    }
  }
  await b.viewport(1280, 800, false, 1);

  // ---------------- Mobile navigation usability
  await b.viewport(390, 844, false);
  await load('#/lab', 2000);
  const mnav = await b.eval(`(() => { const n = document.querySelector('nav'); const r = n ? n.getBoundingClientRect() : null; const toggles = [...document.querySelectorAll('button[aria-expanded],button[aria-controls]')].map(x => ({ name: (x.innerText||x.getAttribute('aria-label')||'').trim(), expanded: x.getAttribute('aria-expanded') }));
    return { navRect: r && [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)], navVisible: !!r && r.width > 0, toggles, navLinks: n ? [...n.querySelectorAll('a')].length : 0, bodyScrollH: document.body.scrollHeight }; })()`);
  R.add('mobile.nav_structure', 'OBSERVED', mnav, await b.shot('mobile_nav_closed.png', 'shots'));
  if (mnav.toggles.length) {
    await b.click(`document.querySelector('button[aria-expanded],button[aria-controls]')`); await sleep(500);
    R.add('mobile.nav_open', 'OBSERVED', await b.eval(`document.querySelector('button[aria-expanded]')?.getAttribute('aria-expanded')`), await b.shot('mobile_nav_open.png', 'shots'));
  }
  await b.viewport(1280, 800, false, 1);

  // ---------------- Performance observation (dev server: unbundled modules; NOT a production bundle measurement)
  await b.send('Network.clearBrowserCache'); b.reqById.clear();
  await b.goto(`${ui}/#/lab`, 3500);
  const nav = await b.eval(`(() => { const n = performance.getEntriesByType('navigation')[0]; return { domContentLoadedMs: Math.round(n.domContentLoadedEventEnd), loadMs: Math.round(n.loadEventEnd), transferSizeDoc: n.transferSize }; })()`);
  const reqs = [...b.reqById.values()];
  const sum = (f) => reqs.filter(f).reduce((a, r) => a + (r.encodedBytes || 0), 0);
  R.add('perf.load_observation', 'OBSERVED', { ...nav, requests: reqs.length, jsBytesTransferred: sum((r) => /javascript/.test(r.mime || '') ), cssBytes: sum((r) => /css/.test(r.mime || '')), totalBytes: sum(() => true), note: 'Vite dev server, unminified ES modules, single run, localhost loopback; not a production-bundle or network measurement' });

  // ---------------- Console / network health across the whole a11y run
  const errs = b.consoleLog.filter((c) => /error|exception|warn/.test(c.kind));
  const failed = [...b.reqById.values()].filter((r) => r.failed && !r.canceled);
  R.add('health.console_errors', errs.length ? 'FAIL' : 'PASS', errs.slice(0, 20));
  R.add('health.failed_requests', failed.length ? 'FAIL' : 'PASS', failed.slice(0, 20));
} catch (e) {
  R.add('a11y.exception', 'FAIL', String((e && e.stack) || e));
} finally {
  R.save(); b.saveLogs('a11y');
  const gone = await b.close();
  writeFileSync(join(evidence, 'a11y.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
  console.log(JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
}
