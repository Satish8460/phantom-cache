// I2b browser verification (observe-only), ports 3100 (Vite) and 8185 (debug gateway, anonymous, minimal lab fixture). Starts ONLY its own
// processes (gateway, Vite, headless Chrome with a throwaway profile under D:/Temp), starts ONE Scenario B run through the UI (phases 5/30/30/30 s)
// and kills everything at the end. Writes i2b_verification.json and PNGs under i2b/. Run: node docs/design/remediation/i2_preview/i2b_verify.mjs
import { spawn, execSync } from 'node:child_process';
import { cpSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, pidAlive, sleep } from '../../../../ui/scripts/cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, '../../../../ui');
const REPO = resolve(UI_DIR, '..');
const UP = 3100, GP = 8185;
const UI = `http://localhost:${UP}`;
const GWQ = `?gateway=http://localhost:${GP}`;
const EXE = 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const LAB_DIR = `D:/Temp/i2b_lab_${new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z')}`;
const M6 = 'm6_sealed_2026-10-29T234845Z', SBDIR = '20261030T003932Z', SB_EVID = `scenario_b.${SBDIR}`;
const OUTDIR = join(HERE, 'i2b'); mkdirSync(OUTDIR, { recursive: true });
const out = { generated: new Date().toISOString(), pids: {}, checks: {}, notes: [] };
const portBusy = (port) => new Promise((res) => { const s = createConnection({ port, host: '127.0.0.1' }); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
const spawned = {};
function startProc(name, file, a, env, cwd) { const c = spawn(file, a, { env: { ...process.env, ...env }, cwd, stdio: 'ignore', windowsHide: true }); spawned[name] = c.pid; out.pids[name] = c.pid; }
function killTree(pid) { if (pid && pidAlive(pid)) { try { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); } catch { /* gone */ } } }
async function waitHttp(url, ms = 60000) { const t0 = Date.now(); while (Date.now() - t0 < ms) { try { await fetch(url); return true; } catch { await sleep(300); } } return false; }
let b = null;
const READY = '!/Checking|Loading this|Loading the|Waiting for the gateway|Connecting/.test(document.body.innerText)';
let cur = { w: 1280, h: 800 };
async function media(features) { await b.media(features); }
async function load(route, w, h, features = [{ name: 'prefers-color-scheme', value: 'light' }]) {
  cur = { w, h }; await media(features); await b.viewport(w, h, false, 1);
  await b.goto('about:blank', 150); await b.goto(`${UI}/${GWQ}${route}`, 1500);
  await b.waitFor(READY, 20000, 300); await sleep(900);
}
const shot = (name) => b.shot(name, 'i2b', false);
const enter = async () => { await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); };
const esc = () => b.key('Escape', { code: 'Escape', vk: 27 });
const active = () => b.eval(`(() => { const e = document.activeElement; return { tag: e.tagName, text: (e.innerText || e.getAttribute('aria-label') || '').trim().replace(/\\s+/g, ' ').slice(0, 40), cancelOpen: e.hasAttribute('data-cancel-open'), inDialog: !!e.closest('dialog'), heading: e.classList.contains('lab-run-head__kind') }; })()`);

let exit = 0;
try {
  for (const p of [UP, GP]) if (await portBusy(p)) throw new Error(`port ${p} busy; not ours`);
  mkdirSync(join(LAB_DIR, M6), { recursive: true }); mkdirSync(join(LAB_DIR, 'scenario_b', SBDIR), { recursive: true });
  const src = join(REPO, 'lab-results');
  for (const n of readdirSync(join(src, M6))) if (n === 'verdict.json' || n === 'manifest.json' || /^analysis_.*\.csv$/.test(n)) cpSync(join(src, M6, n), join(LAB_DIR, M6, n));
  for (const n of ['manifest.json', 'summary.json']) cpSync(join(src, 'scenario_b', SBDIR, n), join(LAB_DIR, 'scenario_b', SBDIR, n));
  const env = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: LAB_DIR, PHANTOM_BIND_ADDR: `127.0.0.1:${GP}`, PHANTOM_CORS_ORIGINS: UI, TEMP: 'D:/Temp', TMP: 'D:/Temp' };
  delete process.env.PHANTOM_SEALED_SALT_HEX; delete process.env.PHANTOM_JWT_SECRET;
  startProc('gateway', EXE, [], env, 'D:/Temp');
  startProc('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', String(UP), '--strictPort', '--host', 'localhost'], { TEMP: 'D:/Temp', TMP: 'D:/Temp' }, UI_DIR);
  if (!(await waitHttp(`http://127.0.0.1:${GP}/`)) || !(await waitHttp(UI + '/'))) throw new Error('gateway or vite did not come up');
  await sleep(1500);
  b = await new Browser({ evidenceDir: HERE, label: 'i2b' }).launch(); out.pids.chrome = b.pid; out.browser = b.version;

  // ---- sticky action bar: 390 and 820 (matrix form with an advanced field focused)
  out.checks.action_bar = {};
  for (const [w, h] of [[390, 844], [820, 1180]]) {
    await load('#/lab/run', w, h);
    await b.click(`document.querySelector('[data-advanced] summary')`); await sleep(300);
    const r = await b.eval(`(() => {
      const bar = document.querySelector('[data-form-bar]'); const bh = Math.round(bar.getBoundingClientRect().height);
      const l = [...document.querySelectorAll('label')].find((x) => x.textContent.trim().startsWith('AIMD maximum')); const el = document.getElementById(l.htmlFor);
      el.focus(); const fr = el.getBoundingClientRect(); const br = bar.getBoundingClientRect();
      return { viewport: innerHeight, barHeight: bh, barTop: Math.round(br.top), focusedFieldBottom: Math.round(fr.bottom), clearOfBar: fr.bottom <= br.top + 1, scrollPaddingBottom: getComputedStyle(document.documentElement).scrollPaddingBottom, statusText: bar.innerText.replace(/\\s+/g, ' ').trim() };
    })()`);
    out.checks.action_bar[`${w}x${h}`] = r;
    await shot(`run_form_focus_last_field__${w}.png`);
  }

  // ---- Scenario B evidence page height + glance order
  out.checks.evidence_scenario_b = {};
  for (const [w, h] of [[1280, 800], [390, 844]]) {
    await load(`#/lab/evidence/${SB_EVID}`, w, h);
    out.checks.evidence_scenario_b[`${w}x${h}`] = await b.eval(`(() => {
      const g = document.getElementById('sb-glance'); const v = g.querySelector('[data-verdict-arm]'); const d = g.querySelector('[data-runclass-disclosure]');
      return { pageHeight: document.documentElement.scrollHeight, closedFolds: [...document.querySelectorAll('details.lab-fold')].filter((x) => !x.open).map((x) => x.id), contentsLinks: document.querySelectorAll('.lab-toc a').length,
        verdictAbove: v.getBoundingClientRect().top < d.getBoundingClientRect().top, runClassClosed: !d.open, glanceTop: Math.round(g.getBoundingClientRect().top + scrollY), verdictTop: Math.round(v.getBoundingClientRect().top + scrollY), scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth };
    })()`);
    if (w === 1280) { await b.click(`[...document.querySelectorAll('.lab-toc a')].find((a) => a.getAttribute('href') === '#sb-limits')`); await sleep(400); out.checks.evidence_scenario_b.contents_opens_limits = await b.eval(`({ open: document.getElementById('sb-limits').open, hash: location.hash })`); }
  }

  // ---- start one Scenario B run (5/30/30/30 s) for the dialog, reduced-motion and forced-colors checks
  await load('#/lab/run', 1280, 800);
  await b.click(`document.querySelector('[data-preset="quick"]')`); await sleep(300);
  await b.click(`document.querySelector('[data-advanced] summary')`); await sleep(300);
  for (const [l, v] of [['Warm-up', 5], ['Healthy', 30], ['Degraded', 30], ['Recovery', 30]]) await b.setField(l, v);
  await sleep(300); await b.clickText('Start run');
  const ok = await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  const runId = ok ? await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`) : null;
  out.runId = runId;
  if (!runId) throw new Error('run did not start');
  const RUN = `#/lab/runs/${runId}`;

  // ---- dialog: Enter opens, focus on "Keep running", Tab stays inside, Escape closes and focus returns to Cancel
  await load(RUN, 1280, 800);
  const D = (out.checks.dialog_keyboard = {});
  await b.eval(`document.querySelector('[data-cancel-open]').focus()`);
  D.focusOnCancelBefore = (await active()).cancelOpen;
  await enter(); await sleep(500);
  D.afterEnter = { dialogOpen: await b.eval(`!!document.querySelector('dialog[open]')`), focus: await active() };
  await shot('cancel_dialog_keyboard__1280.png');
  await b.tab(); await sleep(150); D.afterTab1 = await active();
  await b.tab(); await sleep(150); D.afterTab2 = await active();
  await b.tab(); await sleep(150); D.afterTab3 = await active();
  await esc(); await sleep(500);
  D.afterEscape = { dialogOpen: await b.eval(`!!document.querySelector('dialog[open]')`), focus: await active() };
  D.PASS = D.afterEnter.dialogOpen && D.afterEnter.focus.text.startsWith('Keep running') && D.afterEnter.focus.inDialog && D.afterTab1.inDialog && D.afterTab2.inDialog && D.afterTab3.inDialog && !D.afterEscape.dialogOpen && D.afterEscape.focus.cancelOpen;
  // confirm path: Cancel -> Yes -> focus lands on the run heading (the Cancel button is gone)
  await enter(); await sleep(400);
  await b.eval(`[...document.querySelectorAll('dialog button')].find((x) => /^yes/i.test(x.innerText.trim())).focus()`); await enter(); await sleep(1500);
  D.afterConfirm = { dialogOpen: await b.eval(`!!document.querySelector('dialog[open]')`), focus: await active(), notice: await b.eval(`/Cancel requested|cancel/i.test(document.body.innerText)` ) };
  D.confirmFocusPASS = !D.afterConfirm.dialogOpen && D.afterConfirm.focus.heading;

  // ---- dialog animation under reduced motion (run is cancelling/cancelled now: start the dialog state on a fresh load is not possible, so compare the CSS rule via a probe element)
  const RM = (out.checks.reduced_motion = {});
  for (const [name, features] of [['normal', [{ name: 'prefers-color-scheme', value: 'light' }]], ['reduce', [{ name: 'prefers-color-scheme', value: 'light' }, { name: 'prefers-reduced-motion', value: 'reduce' }]]]) {
    await load('#/lab', 1280, 800, features);
    RM[name] = await b.eval(`(() => { const d = document.createElement('div'); d.className = 'dialog'; document.body.appendChild(d); const dialogAnim = getComputedStyle(d).animationName; d.remove();
      const p = document.createElement('div'); p.className = 'progress-indeterminate'; document.body.appendChild(p); const bar = getComputedStyle(p, '::before').animationName; p.remove();
      const det = document.createElement('details'); det.className = 'disclosure'; det.open = true; det.innerHTML = '<summary>s</summary><p>x</p>'; document.body.appendChild(det); const disc = getComputedStyle(det.querySelector('p')).animationName; det.remove();
      return { matches: matchMedia('(prefers-reduced-motion: reduce)').matches, dialogAnimation: dialogAnim, indeterminateBarAnimation: bar, disclosureAnimation: disc }; })()`);
    if (name === 'reduce') { await shot('lab_home__reduced_motion__1280.png'); await load(RUN, 1280, 800, features); await shot('run_detail__reduced_motion__1280.png'); }
  }
  RM.PASS = RM.reduce.matches && RM.reduce.indeterminateBarAnimation === 'none' && RM.reduce.dialogAnimation === 'pc-fade' && RM.reduce.disclosureAnimation === 'pc-fade' && RM.normal.dialogAnimation === 'pc-appear';

  // ---- forced colors
  const FC = (out.checks.forced_colors = {});
  const fcf = [{ name: 'prefers-color-scheme', value: 'light' }, { name: 'forced-colors', value: 'active' }];
  await load('#/lab', 1280, 800, fcf); FC.matches = await b.eval(`matchMedia('(forced-colors: active)').matches`); await shot('lab_home__forced_colors__1280.png');
  FC.lab_home = await b.eval(`(() => { const c = (s) => getComputedStyle(document.querySelector(s)); return { stripBorder: c('.lab-status').borderTopColor, stripBorderStyle: c('.lab-status').borderTopStyle, primaryBorder: c('[data-primary-action]').borderTopColor, tabCurrentOutline: c('.lab-subnav .tab[aria-current]').outlineStyle }; })()`);
  await load(RUN, 1280, 800, fcf); await shot('run_detail__forced_colors__1280.png');
  await load('#/lab/run', 1280, 800, fcf); await shot('run_form__forced_colors__1280.png');
  FC.run_form = await b.eval(`(() => ({ checkedKindOutline: getComputedStyle(document.querySelector('.lab-kind:has(input:checked)')).outlineStyle, barBorder: getComputedStyle(document.querySelector('.form-bar')).borderTopStyle }))()`);

  // ---- 200% zoom equivalent: 640x400 CSS px
  const Z = (out.checks.zoom_200 = {});
  for (const [name, route] of [['lab_home', '#/lab'], ['run_form', '#/lab/run']]) {
    await load(route, 640, 400);
    Z[name] = await b.eval(`({ scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth, primaryVisible: (() => { const e = document.querySelector('[data-primary-action]'); return e ? e.getBoundingClientRect().bottom <= innerHeight : null; })(), barHeight: (document.querySelector('[data-form-bar]') || { getBoundingClientRect: () => ({ height: null }) }).getBoundingClientRect().height })`);
    await shot(`${name}__zoom200__640x400.png`);
  }
  out.consoleErrors = b.consoleLog.filter((e) => /exception|log\.error|console\.error/.test(e.kind)).slice(0, 10);
} catch (e) {
  console.error('FATAL', e.stack || e); out.fatal = String(e.stack || e); exit = 1;
} finally {
  try { if (b) await b.close(); } catch { /* ignore */ }
  killTree(spawned.vite); killTree(spawned.gateway);
  await sleep(1000);
  out.portsFreeAfter = { [UP]: !(await portBusy(UP)), [GP]: !(await portBusy(GP)) };
  out.pidsAliveAfter = Object.entries(spawned).filter(([, p]) => pidAlive(p)).map(([n]) => n);
  writeFileSync(join(HERE, 'i2b_verification.json'), JSON.stringify(out, null, 2));
  console.log('wrote i2b_verification.json', out.portsFreeAfter);
  process.exit(exit);
}
