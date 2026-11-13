// I4 PREVIEW: screenshot gallery plus measurements for the shell, navigation and non-Lab pages. Dependency-free (Node built-ins + ui/scripts/cdp-lib.mjs).
//   node docs/design/remediation/i4_preview/i4_gallery.mjs --out docs/design/remediation/i4_preview [--measure-only] [--no-gallery]
// Starts and stops ONLY the processes it spawns (anonymous debug gateway on 127.0.0.1:8085, Vite on localhost:3000 --strictPort,
// headless Chrome with throwaway profiles under D:/Temp). Refuses to start if ports 3000/8085 are busy. Lab fixture copies only
// verdict.json/manifest.json/analysis_*.csv (M6) and manifest.json/summary.json (Scenario B) into a throwaway directory.
// File names match the BEFORE gallery: <state>__<width>__<theme>.png and ..__full.png. Themes use prefers-color-scheme emulation.
import { spawn, execSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, pidAlive, sleep } from '../../../../ui/scripts/cdp-lib.mjs';

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf('--' + k); return i >= 0 ? args[i + 1] : d; };
const OUT = resolve(opt('out', '.'));
const UP = 3000, GP = 8085;
const UI = `http://localhost:${UP}`;
const EXE = 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, '../../../../ui');
const REPO = resolve(UI_DIR, '..');
const STAMP = new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z');
const LAB_DIR = `D:/Temp/i4_lab_${STAMP}`;
const M6 = 'm6_sealed_2026-10-29T234845Z', SBDIR = '20261030T003932Z';
const VIEWPORTS = [[390, 844], [820, 1180], [1280, 800]];
const THEMES = ['light', 'dark'];
const ROUTES = [['overview', '#/'], ['traffic', '#/traffic'], ['cache', '#/cache'], ['freshness', '#/freshness'], ['origin', '#/origin'],
  ['cluster', '#/cluster'], ['intelligence', '#/intelligence'], ['simulation', '#/simulation'], ['lab_home', '#/lab']];

mkdirSync(OUT, { recursive: true });
const manifest = { generated: new Date().toISOString(), entries: [], pids: {}, notes: [], measurements: {} };
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

const spawned = {};
const portBusy = (port) => new Promise((res) => { const s = createConnection({ port, host: '127.0.0.1' }); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
function startProc(name, file, a, env, cwd) {
  const c = spawn(file, a, { env: { ...process.env, ...env }, cwd, stdio: 'ignore', windowsHide: true });
  spawned[name] = c.pid; manifest.pids[name] = c.pid; log('started', name, c.pid);
}
function killTree(pid) { if (pid && pidAlive(pid)) { try { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); } catch { /* gone */ } } }
async function waitHttp(url, ms = 60000) { const t0 = Date.now(); while (Date.now() - t0 < ms) { try { await fetch(url); return true; } catch { await sleep(300); } } return false; }

let b = null; let loads = 0; let cur = { w: 1280, h: 800, theme: 'light' };
async function newBrowser() {
  b = await new Browser({ evidenceDir: OUT, label: 'i4' }).launch();
  manifest.pids.chrome = [...(manifest.pids.chrome || []), b.pid]; manifest.chromeVersion = b.version;
  await b.media([{ name: 'prefers-color-scheme', value: cur.theme }]); await b.viewport(cur.w, cur.h, false, 1);
}
async function relaunch() { await b.close(); await newBrowser(); }
async function setCtx(w, h, theme, extra = []) { cur = { w, h, theme }; await b.media([{ name: 'prefers-color-scheme', value: theme }, ...extra]); await b.viewport(w, h, false, 1); }
const READY = `!/Checking|Loading this|Loading the|Waiting for the gateway|Connecting/.test(document.body.innerText)`;
async function load(route, { settle = 1200, wait = true } = {}) {
  if (++loads % 4 === 0) await relaunch();
  await b.goto('about:blank', 150); await b.goto(`${UI}/${route}`, settle);
  const ok = wait ? await b.waitFor(READY, 20000, 300) : true;
  await sleep(900);
  return ok;
}
async function capture(state, route, note = '') {
  const stem = `${state}__${cur.w}__${cur.theme}`;
  const info = await b.eval(`({ h1: (document.querySelector('h1')||{}).innerText, title: document.title, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, docHeight: document.documentElement.scrollHeight })`).catch(() => ({}));
  const f1 = await b.shot(`${stem}.png`, '.', false);
  const f2 = await b.shot(`${stem}__full.png`, '.', true);
  manifest.entries.push({ state, route, viewport: `${cur.w}x${cur.h}`, theme: cur.theme, files: [f1, f2], page: info, note });
}

const evalJson = (expr) => b.eval(`JSON.stringify((${expr})())`).then(JSON.parse);
const M = manifest.measurements;

async function measure() {
  // ---- (5) gutter: measure in viewport mode, then emulate a full-page capture by resizing the viewport to the document height ----
  await setCtx(1280, 800, 'light'); await load('#/');
  const rects = `() => { const r = (s) => { const e = document.querySelector(s); if (!e) return null; const x = e.getBoundingClientRect(); return { left: Math.round(x.left * 10) / 10, right: Math.round(x.right * 10) / 10, width: Math.round(x.width * 10) / 10 }; };
    const lastSection = [...document.querySelectorAll('.app-main > *')].filter((e) => e.getBoundingClientRect().height > 0).pop();
    return { innerWidth: innerWidth, clientWidth: document.documentElement.clientWidth, scrollbar: innerWidth - document.documentElement.clientWidth, header: r('.app-header'), nav: r('.app-nav'), main: r('.app-main'), footer: r('.app-footer'),
      firstChildRight: (() => { const e = document.querySelector('.app-main > *:not([role=status])'); return e ? Math.round(e.getBoundingClientRect().right * 10) / 10 : null; })(), mainPaddingRight: getComputedStyle(document.querySelector('.app-main')).paddingRight, lastSectionRight: lastSection ? lastSection.getBoundingClientRect().right : null }; }`;
  M.gutter_viewport_1280 = await evalJson(rects);
  const docH = await b.eval('document.documentElement.scrollHeight');
  await b.viewport(1280, docH, false, 1); await sleep(500);
  M.gutter_tall_viewport_1280 = { emulatedHeight: docH, ...(await evalJson(rects)) };
  const shotFull = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  const png = Buffer.from(shotFull.data, 'base64'); M.gutter_tall_viewport_1280.pngWidth = png.readUInt32BE(16); M.gutter_tall_viewport_1280.pngHeight = png.readUInt32BE(20);
  await b.viewport(1280, 800, false, 1); await sleep(300);
  const vp = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  const fp = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
  const dim = (d) => { const p = Buffer.from(d.data, 'base64'); return [p.readUInt32BE(16), p.readUInt32BE(20)]; };
  M.png_dimensions_1280 = { viewportShot: dim(vp), fullPageShot: dim(fp) };
  // ---- title per route ----
  M.titles = {};
  for (const [name, route] of [...ROUTES, ['lab_run', '#/lab/run'], ['lab_runs', '#/lab/runs'], ['lab_evidence', '#/lab/evidence'], ['lab_evidence_item', `#/lab/evidence/${M6}`], ['not_found', '#/nope']]) {
    await load(route, { wait: false, settle: 700 }); M.titles[name] = await b.eval('document.title');
  }
  // ---- overflow at 320/390/640/820/1280 on every route, light ----
  M.overflow = {};
  for (const w of [320, 390, 640, 820, 1280]) {
    await setCtx(w, 900, 'light'); M.overflow[w] = {};
    for (const [name, route] of ROUTES) { await load(route); M.overflow[w][name] = await evalJson(`() => ({ scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, over: document.documentElement.scrollWidth - document.documentElement.clientWidth })`); }
  }
  // ---- header height, nav visibility, menu behaviour ----
  M.header = {};
  for (const w of [320, 390, 640, 819, 820, 1280]) {
    await setCtx(w, 900, 'light'); await load('#/traffic');
    M.header[w] = await evalJson(`() => { const v = (s) => { const e = document.querySelector(s); if (!e) return null; const cs = getComputedStyle(e); const r = e.getBoundingClientRect(); return { display: cs.display, visible: r.width > 0 && r.height > 0, w: Math.round(r.width), h: Math.round(r.height) }; };
      return { headerHeight: Math.round(document.querySelector('.app-header').getBoundingClientRect().height), menuButton: v('.app-header .app-nav__toggle'), headerTheme: v('.app-header__theme'), navPanel: v('.app-nav'), navLinksVisible: [...document.querySelectorAll('.app-nav a')].filter((a) => a.getBoundingClientRect().height > 0).length, footTheme: v('.app-nav__foot-theme') }; }`);
  }
  await setCtx(390, 844, 'light'); await load('#/traffic');
  M.menu = {};
  const state = () => evalJson(`() => ({ expanded: document.querySelector('.app-nav__toggle').getAttribute('aria-expanded'), navVisible: document.querySelector('.app-nav').getBoundingClientRect().height > 0, active: (document.activeElement.id || document.activeElement.getAttribute('data-screen') || document.activeElement.tagName) })`);
  M.menu.initial = await state();
  await b.eval(`document.querySelector('.app-nav__toggle').focus()`);
  const enter = async () => { await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: String.fromCharCode(13) }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); };
  await enter(); await sleep(400); M.menu.afterEnter = await state();
  await b.eval(`document.querySelector('.app-nav a[data-screen="cache"]').focus()`); await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(300); M.menu.afterEscape = await state();
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(300);
  M.menu.openHeights = await evalJson(`() => ({ header: Math.round(document.querySelector('.app-header').getBoundingClientRect().height), nav: Math.round(document.querySelector('.app-nav').getBoundingClientRect().height) })`);
  await b.click(`document.querySelector('.app-nav a[data-screen="freshness"]')`); await sleep(700);
  M.menu.afterLink = { ...(await state()), hash: await b.eval('location.hash'), title: await b.eval('document.title'), h1Focused: await b.eval(`document.activeElement === document.querySelector('h1')`) };
  // ---- keyboard order at 1280 ----
  await setCtx(1280, 800, 'light'); await load('#/');
  const order = [];
  await b.eval('document.body.focus(); window.scrollTo(0,0)');
  for (let i = 0; i < 24; i++) { await b.tab(); order.push(await b.eval(`(() => { const e = document.activeElement; const r = e.getBoundingClientRect(); return (e.className && typeof e.className === 'string' ? e.className.split(' ')[0] : e.tagName) + ':' + (e.getAttribute('data-screen') || e.getAttribute('data-theme-option') || (e.innerText || '').trim().slice(0, 24)) + (r.width === 0 ? ' [zero-size]' : ''); })()`)); }
  M.keyboardOrder1280 = order;
  // skip link
  await load('#/traffic'); await b.eval('document.body.focus()'); await b.tab();
  M.skipLink = await evalJson(`() => { const e = document.activeElement; const r = e.getBoundingClientRect(); return { cls: e.className, text: e.innerText, top: Math.round(r.top), visible: r.top >= 0 && r.height > 0 }; }`);
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: String.fromCharCode(13) }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); await sleep(400);
  M.skipLinkActivated = await evalJson(`() => ({ active: document.activeElement.id || document.activeElement.tagName, hash: location.hash, notFoundMessage: /does not exist/.test(document.body.innerText), h1: document.querySelector('h1').innerText })`);
  // ---- sticky side nav at 1280 ----
  await load('#/traffic'); await b.eval('window.scrollTo(0, 700)'); await sleep(300);
  M.stickyNav = await evalJson(`() => { const p = document.querySelector('.app-nav__panel').getBoundingClientRect(); const n = document.querySelector('.app-nav').getBoundingClientRect(); return { scrollY: Math.round(scrollY), panelTop: Math.round(p.top), panelBottom: Math.round(p.bottom), navCellHeight: Math.round(n.height), viewport: innerHeight, docHeight: document.documentElement.scrollHeight }; }`);
  // ---- theme control ----
  await setCtx(1280, 800, 'light'); await load('#/');
  const th = () => evalJson(`() => ({ dataTheme: document.documentElement.getAttribute('data-theme'), stored: localStorage.getItem('phantom.ui.theme'), bg: getComputedStyle(document.body).backgroundColor, pressed: [...document.querySelectorAll('.app-header__theme .seg__btn')].map((x) => x.textContent + ':' + x.getAttribute('aria-pressed')) })`);
  M.theme = { initial: await th() };
  await b.click(`document.querySelector('.app-header__theme [data-theme-option="dark"]')`); await sleep(300); M.theme.afterDark = await th();
  await b.send('Page.reload'); await sleep(2500); M.theme.afterReloadStillDark = await th();
  await b.click(`document.querySelector('.app-header__theme [data-theme-option="light"]')`); await sleep(300); M.theme.afterLight = await th();
  await b.click(`document.querySelector('.app-header__theme [data-theme-option="system"]')`); await sleep(300); M.theme.afterSystem = await th();
  await b.eval(`document.querySelector('.app-header__theme [data-theme-option="dark"]').focus()`); await b.key('ArrowRight', { code: 'ArrowRight', vk: 39 });
  M.theme.keyboardNote = 'segmented control buttons are in the Tab order (no roving arrows); Space/Enter activate';
  await b.eval(`localStorage.removeItem('phantom.ui.theme')`);
  // ---- target sizes ----
  M.targets = {};
  for (const [w, route] of [[390, '#/'], [390, '#/traffic'], [1280, '#/'], [1280, '#/traffic'], [1280, '#/cache']]) {
    await setCtx(w, 900, 'light'); await load(route);
    if (w === 390) { await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(300); }
    M.targets[`${w}${route}`] = await evalJson(`() => { const out = []; for (const e of document.querySelectorAll('a[href], button, summary, input, select')) { const r = e.getBoundingClientRect(); if (r.width === 0 || r.height === 0) continue; if (r.height < ${w === 390 ? 44 : 32} || r.width < 24) out.push({ el: e.tagName + '.' + (e.className || '') + ' ' + (e.innerText || '').trim().slice(0, 24), w: Math.round(r.width), h: Math.round(r.height) }); } return { smallerThanTarget: out.slice(0, 30), count: out.length, total: document.querySelectorAll('a[href], button, summary, input, select').length }; }`);
  }
  // ---- reduced motion ----
  await setCtx(390, 844, 'light', [{ name: 'prefers-reduced-motion', value: 'reduce' }]); await load('#/traffic');
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(100);
  M.reducedMotion = await evalJson(`() => ({ matches: matchMedia('(prefers-reduced-motion: reduce)').matches, navAnimation: getComputedStyle(document.querySelector('.app-nav')).animationName, spinnerAnim: [...document.querySelectorAll('.spinner svg')].length })`);
  await setCtx(390, 844, 'light', [{ name: 'prefers-reduced-motion', value: 'no-preference' }]); await load('#/traffic');
  await b.click(`document.querySelector('.app-nav__toggle')`); await sleep(100);
  M.noReducedMotion = await evalJson(`() => ({ matches: matchMedia('(prefers-reduced-motion: reduce)').matches, navAnimation: getComputedStyle(document.querySelector('.app-nav')).animationName })`);
  // ---- type tiers on a page (computed) ----
  await setCtx(1280, 800, 'light'); await load('#/');
  M.typeTiers = await evalJson(`() => { const f = (s) => { const e = document.querySelector(s); if (!e) return null; const c = getComputedStyle(e); return { size: c.fontSize, weight: c.fontWeight, ls: c.letterSpacing, lh: c.lineHeight }; }; return { h1: f('h1'), h2: f('h2'), metricValue: f('.metric__value'), dense: f('.kv'), caption: f('.page-head__group') }; }`);
  M.accessibleHeadings = await evalJson(`() => [...document.querySelectorAll('h1,h2,h3')].map((h) => h.tagName + ' ' + h.innerText.trim().slice(0, 40))`);
  M.consoleErrors = b.consoleLog.filter((l) => /exception|error/.test(l.kind)).slice(0, 20);
}

async function gallery() {
  for (const theme of THEMES) for (const [w, h] of VIEWPORTS) {
    await setCtx(w, h, theme);
    for (const [state, route] of ROUTES) { try { const ok = await load(route); await capture(state, route, ok ? '' : 'still loading after 20 s'); } catch (e) { manifest.notes.push(`${state}__${w}__${theme}: ${String(e.message).slice(0, 120)}`); } }
  }
}
async function stopped() {
  killTree(spawned.gateway); await sleep(1500); manifest.notes.push('gateway process killed by recorded pid before the gateway_stopped captures');
  for (const theme of THEMES) for (const [w, h] of VIEWPORTS) {
    await setCtx(w, h, theme);
    try { await load('#/', { wait: false, settle: 2500 }); await sleep(2500); await capture('gateway_stopped_overview', '#/', 'gateway stopped'); } catch (e) { manifest.notes.push(`stopped ${w} ${theme}: ${e.message}`); }
  }
  if (!opt('no-measure-stopped')) {
    await setCtx(1280, 800, 'light'); await load('#/traffic', { wait: false, settle: 2500 }); await sleep(2500);
    M.stoppedTraffic = await evalJson(`() => ({ statusChip: document.querySelector('.app-header__status').innerText, metrics: document.querySelectorAll('[data-metric]').length, body: document.querySelector('.state-block') ? document.querySelector('.state-block').innerText.slice(0, 200) : null, footer: document.querySelector('.app-footer').innerText })`);
  }
}

let exit = 0;
try {
  for (const p of [UP, GP]) if (await portBusy(p)) throw new Error(`port ${p} is not free; refusing to start`);
  const src = join(REPO, 'lab-results');
  mkdirSync(join(LAB_DIR, M6), { recursive: true }); mkdirSync(join(LAB_DIR, 'scenario_b', SBDIR), { recursive: true });
  for (const n of readdirSync(join(src, M6))) if (n === 'verdict.json' || n === 'manifest.json' || /^analysis_.*\.csv$/.test(n)) cpSync(join(src, M6, n), join(LAB_DIR, M6, n));
  for (const n of ['manifest.json', 'summary.json']) cpSync(join(src, 'scenario_b', SBDIR, n), join(LAB_DIR, 'scenario_b', SBDIR, n));
  manifest.labDir = LAB_DIR;
  const env = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: LAB_DIR, PHANTOM_BIND_ADDR: `127.0.0.1:${GP}`, PHANTOM_CORS_ORIGINS: UI, TEMP: 'D:/Temp', TMP: 'D:/Temp' };
  delete process.env.PHANTOM_SEALED_SALT_HEX; delete process.env.PHANTOM_JWT_SECRET;
  startProc('gateway', EXE, [], env, 'D:/Temp');
  startProc('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', String(UP), '--strictPort', '--host', 'localhost'], { TEMP: 'D:/Temp', TMP: 'D:/Temp' }, UI_DIR);
  if (!(await waitHttp(`http://127.0.0.1:${GP}/`)) || !(await waitHttp(UI + '/'))) throw new Error('gateway or vite did not come up');
  await sleep(1500);
  await newBrowser(); await setCtx(1280, 800, 'light');
  if (!args.includes('--no-measure')) { await measure(); writeFileSync(join(OUT, 'measurements.json'), JSON.stringify(M, null, 2)); }
  if (!args.includes('--no-gallery')) { await gallery(); await stopped(); }
} catch (e) {
  console.error('FATAL', e.stack || e); manifest.fatal = String(e.stack || e); exit = 1;
} finally {
  try { if (b) await b.close(); } catch { /* ignore */ }
  for (const pid of manifest.pids.chrome || []) killTree(pid);
  killTree(spawned.vite); killTree(spawned.gateway);
  await sleep(1000);
  manifest.portsFreeAfter = { [UP]: !(await portBusy(UP)), [GP]: !(await portBusy(GP)) };
  manifest.pidsAliveAfter = Object.entries(spawned).filter(([, p]) => pidAlive(p)).map(([n]) => n);
  writeFileSync(join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 2));
  if (Object.keys(M).length) writeFileSync(join(OUT, 'measurements.json'), JSON.stringify(M, null, 2));
  log('done', { files: manifest.entries.length, ports: manifest.portsFreeAfter, alive: manifest.pidsAliveAfter });
  process.exit(exit);
}
