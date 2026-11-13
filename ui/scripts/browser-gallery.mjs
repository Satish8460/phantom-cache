// Screenshot gallery (BEFORE/AFTER pairs for the UI/UX remediation). Dependency-free: Node built-ins + ./cdp-lib.mjs.
//
//   node ui/scripts/browser-gallery.mjs --out <dir>
//
// Self-contained and observe-only with respect to the repo: it starts and stops ONLY the processes it spawns
// (anonymous debug gateway on 127.0.0.1:8085, Vite on localhost:3000 --strictPort, headless Chrome with throwaway
// profiles under D:/Temp), copies a minimal lab fixture (read-only from the repo's lab-results) into a throwaway
// lab dir under D:/Temp, and never touches the repo's lab-results, the salt directory, the user's Edge or any
// other process. It refuses to start if ports 3000/8085 are not free and verifies they are free afterwards.
// Both captures (BEFORE and AFTER) use this script's state set so file names pair up. The BEFORE set was produced by the
// earlier revision (old Lab selectors); this revision drives the CURRENT Lab (tabs, choice cards, presets, native cancel dialog).
// FULL-PAGE FIX: the BEFORE full-page PNGs were captured with captureBeyondViewport and are cropped by the classic scrollbar gutter
// (layout 1265 CSS px wide in a 1280 emulated viewport -> 1265 px PNG; the right 15 px of the 1280 frame are lost). This revision
// captures *__full.png by emulating a viewport of the full emulated width and the full document height (no scrollbar, no crop) and
// records pngWidth/pngHeight per entry. BEFORE and AFTER full-page widths are therefore NOT strictly comparable; the viewport-only
// *.png files (same emulation as before) remain comparable.
//
// Options: --out <dir> (required)  --gateway-exe <path>  --ui <url, default http://localhost:3000>
// Themes are produced with prefers-color-scheme emulation (the app theme control stays on its default "system").
// Viewports use mobile:false emulation, DPR 1.
import { spawn, execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, pidAlive, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
if (!args.out) { console.error('usage: node ui/scripts/browser-gallery.mjs --out <dir>'); process.exit(2); }
const OUT = resolve(args.out);
const UI = args.ui || 'http://localhost:3000';
const EXE = args['gateway-exe'] || 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, '..');
const REPO = resolve(UI_DIR, '..');
const STAMP = new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z');
const LAB_DIR = `D:/Temp/gallery_lab_${STAMP}`;
const M6 = 'm6_sealed_2026-10-29T234845Z', SBDIR = '20261030T003932Z', SB_EVID = `scenario_b.${SBDIR}`;
const VIEWPORTS = [[390, 844], [820, 1180], [1280, 800]];
const THEMES = ['light', 'dark'];

mkdirSync(OUT, { recursive: true });
const manifest = { generated: new Date().toISOString(), script: 'ui/scripts/browser-gallery.mjs', themeMethod: 'Emulation.setEmulatedMedia prefers-color-scheme (app theme control left on default "system")',
  viewportMethod: 'Emulation.setDeviceMetricsOverride mobile:false, deviceScaleFactor 1',
  fullPageMethod: 'AFTER revision: viewport emulated to (width x document scrollHeight, capped 16000), Page.captureScreenshot captureBeyondViewport:false, viewport restored. Fixes the 15 px right-hand crop of the BEFORE full-page PNGs (BEFORE full-page PNG width 1265 vs 1280 viewport). Full-page widths are NOT strictly comparable with BEFORE; viewport-only shots are.', entries: [], notReached: [], pids: {}, notes: [] };
const entries = manifest.entries;
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

// ---------- hashes ----------
async function sha256File(p) { return new Promise((res, rej) => { const h = createHash('sha256'); createReadStream(p).on('data', (d) => h.update(d)).on('end', () => res(h.digest('hex').toUpperCase())).on('error', rej); }); }
function uiSrcHash() {
  const files = [];
  const walk = (d) => { for (const n of readdirSync(d, { withFileTypes: true })) { const p = join(d, n.name); n.isDirectory() ? walk(p) : files.push(p); } };
  walk(join(UI_DIR, 'src'));
  files.sort((a, b) => relative(UI_DIR, a).replace(/\\/g, '/').localeCompare(relative(UI_DIR, b).replace(/\\/g, '/')));
  const h = createHash('sha256');
  for (const f of files) h.update(readFileSync(f));
  return { sha256: h.digest('hex'), files: files.length };
}

// ---------- process control (only what we spawn) ----------
const spawned = {};
const portBusy = (port) => new Promise((res) => { const s = createConnection({ port, host: '127.0.0.1' }); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
function startProc(name, file, a, env, cwd) {
  const c = spawn(file, a, { env: { ...process.env, ...env }, cwd, stdio: 'ignore', windowsHide: true });
  spawned[name] = c.pid; manifest.pids[name] = c.pid; log('started', name, c.pid);
  return c;
}
function killTree(pid) { if (pid && pidAlive(pid)) { try { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); } catch { /* gone */ } } }
async function waitHttp(url, ms = 60000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) { try { await fetch(url); return true; } catch { await sleep(300); } }
  return false;
}

// ---------- browser helpers ----------
let b = null; let loads = 0; let cur = { w: 1280, h: 800, theme: 'light' };
async function newBrowser() {
  b = await new Browser({ evidenceDir: OUT, label: 'gal' }).launch();
  manifest.pids.chrome = [...(manifest.pids.chrome || []), b.pid]; manifest.chromeVersion = b.version;
  await b.media([{ name: 'prefers-color-scheme', value: cur.theme }]); await b.viewport(cur.w, cur.h, false, 1);
}
async function relaunch() { await b.close(); await newBrowser(); }
async function setCtx(w, h, theme) {
  cur = { w, h, theme };
  await b.media([{ name: 'prefers-color-scheme', value: theme }]); await b.viewport(w, h, false, 1);
}
const READY = `!/Checking|Loading this|Loading the|Waiting for the gateway|Connecting/.test(document.body.innerText)`;
async function load(route, { lab = route.startsWith('#/lab'), settle = 1800 } = {}) {
  if (++loads % 3 === 0) await relaunch(); // harness caveat from round 3: relaunch Chrome every 3 cold loads
  await b.goto('about:blank', 150); await b.goto(`${UI}/${route}`, settle);
  const ok = await b.waitFor(READY, 20000, 300);
  await sleep(lab ? 700 : 1200);
  return ok;
}
const pageInfo = () => b.eval(`(() => { const t = (s) => ((document.querySelector(s) || {}).innerText || '').replace(/\\s+/g, ' ').trim().slice(0, 100);
  return { h1: t('h1'), chip: t('[class*=conn],[class*=status-chip],header [role=status]'), scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, title: document.title }; })()`);
const base = { gatewaySha256: null, uiSrcSha256: null };
const pngSize = (f) => { const p = readFileSync(join(OUT, f)); return [p.readUInt32BE(16), p.readUInt32BE(20)]; };
const docH = () => b.eval('Math.max(document.documentElement.scrollHeight, document.body.scrollHeight)');
async function shotFull(name) {
  let h = await docH();
  const w0 = cur.w, h0 = cur.h;
  await b.viewport(w0, Math.min(Math.max(h, h0), 16000), false, 1); await sleep(500);
  const h2 = await docH();
  if (h2 !== h) { h = h2; await b.viewport(w0, Math.min(Math.max(h, h0), 16000), false, 1); await sleep(400); }
  const s = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  writeFileSync(join(OUT, name), Buffer.from(s.data, 'base64'));
  await b.viewport(w0, h0, false, 1); await sleep(300);
  return { file: name, docHeight: h };
}
async function capture(state, route, { full = false, note = '', extraNotes = [] } = {}) {
  const info = await pageInfo().catch(() => ({}));
  const stem = `${state}__${cur.w}__${cur.theme}`;
  const files = [await b.shot(`${stem}.png`, '.', false)];
  let docHeight;
  if (full) { const r = await shotFull(`${stem}__full.png`); docHeight = r.docHeight; files.push('./' + r.file); }
  for (const f of files) { const fp = f.endsWith('__full.png'); const [pw, ph] = pngSize(f.replace(/^\.\//, '')); entries.push({ file: f.replace(/^\.\//, ''), state, route, viewport: `${cur.w}x${cur.h}`, theme: cur.theme, fullPage: fp, pngWidth: pw, pngHeight: ph, docHeight: fp ? docHeight : undefined, timestamp: new Date().toISOString(), gatewaySha256: base.gatewaySha256, uiSrcSha256: base.uiSrcSha256, afterUiSrcSha256: base.uiSrcSha256, page: info, notes: [note, ...extraNotes].filter(Boolean).join(' | ') }); }
  return info;
}
const notReached = (state, reason) => { manifest.notReached.push({ state, reason }); log('NOT REACHED', state, reason); };

// ---------- UI actions (same selectors as the round-3 scripts) ----------
const runStatusText = () => b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText||''`);
async function fillSb({ reps, arms, phases, rate, keys }) {
  // CURRENT Lab: choice card + "Quick check" preset (1 rep, BASE + STATIC-CAP, 1 s phases); advanced fields only when phases differ
  await b.click(`document.querySelector('label[data-kind-option="scenario_b"] input')`);
  await sleep(300);
  await b.click(`document.querySelector('[data-preset="quick"]')`);
  await sleep(300);
  if (phases.some((p) => p !== 1) || rate || keys) { // rate/keys 100/100 as in the BEFORE recipe
    await b.click(`document.querySelector('[data-advanced] summary')`); await sleep(300);
    for (const [l, v] of [['Warm-up', phases[0]], ['Healthy', phases[1]], ['Degraded', phases[2]], ['Recovery', phases[3]], ['Arrival rate', rate], ['Key catalogue', keys]]) if (v !== undefined) await b.setField(l, v);
  }
  if (reps !== 1) await b.setField('Repetitions per arm', reps);
  await sleep(400);
}
async function startRunViaUi(prep) {
  await load('#/lab/run');
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  await prep();
  await b.clickText('Start run');
  const ok = await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  return ok ? b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`) : null;
}
async function waitTerminal(maxS = 240) {
  for (let i = 0; i < maxS / 2; i++) { await sleep(2000); const t = await runStatusText(); if (/(completed|failed|cancelled|invalid)/i.test(t) && !/last known/i.test(t)) return t; }
  return null;
}

// =========================== main ===========================
let exit = 0;
try {
  for (const p of [3000, 8085]) if (await portBusy(p)) throw new Error(`port ${p} is not free; refusing to start (not ours to kill)`);
  if (!existsSync(EXE)) throw new Error('gateway exe missing: ' + EXE);
  base.gatewaySha256 = await sha256File(EXE); const h = uiSrcHash(); base.uiSrcSha256 = h.sha256;
  Object.assign(manifest, { gatewayExe: EXE, gatewaySha256: base.gatewaySha256, uiSrcSha256: h.sha256, uiSrcFileCount: h.files, uiSrcHashMethod: 'sha256 over the raw bytes of every file under ui/src concatenated in sorted relative-path order' });
  log('gateway sha256', base.gatewaySha256, 'ui/src sha256', h.sha256, `(${h.files} files)`);

  // lab fixture: ONLY verdict.json, manifest.json, analysis_*.csv (M6) and manifest.json, summary.json (Scenario B)
  const src = join(REPO, 'lab-results');
  mkdirSync(join(LAB_DIR, M6), { recursive: true }); mkdirSync(join(LAB_DIR, 'scenario_b', SBDIR), { recursive: true });
  for (const n of readdirSync(join(src, M6))) if (n === 'verdict.json' || n === 'manifest.json' || /^analysis_.*\.csv$/.test(n)) cpSync(join(src, M6, n), join(LAB_DIR, M6, n));
  for (const n of ['manifest.json', 'summary.json']) cpSync(join(src, 'scenario_b', SBDIR, n), join(LAB_DIR, 'scenario_b', SBDIR, n));
  manifest.labDir = LAB_DIR;

  const env = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: LAB_DIR, PHANTOM_BIND_ADDR: '127.0.0.1:8085', TEMP: 'D:/Temp', TMP: 'D:/Temp' };
  delete process.env.PHANTOM_SEALED_SALT_HEX; delete process.env.PHANTOM_JWT_SECRET;
  startProc('gateway', EXE, [], env, 'D:/Temp');
  startProc('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', '3000', '--strictPort', '--host', 'localhost'], { TEMP: 'D:/Temp', TMP: 'D:/Temp' }, UI_DIR);
  if (!(await waitHttp('http://127.0.0.1:8085/')) || !(await waitHttp(UI + '/'))) throw new Error('gateway or vite did not come up');
  await sleep(1500);

  await newBrowser();
  await setCtx(1280, 800, 'light');

  // ---- Phase B: create the run states through the real UI ----
  const ids = {};
  ids.sbDone = await startRunViaUi(() => fillSb({ reps: 1, arms: ['BASE', 'STATIC-CAP'], phases: [1, 1, 1, 1], rate: 100, keys: 100 }));
  log('sbDone', ids.sbDone);
  const t1 = ids.sbDone ? await waitTerminal(300) : null;
  if (!ids.sbDone || !/completed/i.test(t1 || '')) notReached('run_detail_sb_completed', `small Scenario B run did not complete (${ids.sbDone}; "${(t1 || '').slice(0, 60)}")`);

  ids.t1Done = await startRunViaUi(async () => {
    await b.click(`document.querySelector('label[data-kind-option="t1_matrix"] input')`); await sleep(400);
    await b.click(`document.querySelector('[data-preset="smallest"]')`); await sleep(500); // "Smallest run" = T1 smoke, 1 seed
  });
  log('t1Done', ids.t1Done);
  const t2 = ids.t1Done ? await waitTerminal(300) : null;
  if (!ids.t1Done || !/completed/i.test(t2 || '')) notReached('run_detail_t1_completed', `T1 matrix run did not complete (${ids.t1Done}; "${(t2 || '').slice(0, 60)}")`);

  // long-ish Scenario B run kept in progress while the 6 in-progress / cancel-dialog captures are taken
  ids.sbLive = await startRunViaUi(() => fillSb({ reps: 1, arms: ['BASE', 'STATIC-CAP'], phases: [5, 30, 30, 30], rate: 100, keys: 100 }));
  log('sbLive', ids.sbLive);
  manifest.runIds = ids;
  if (!ids.sbLive) {
    notReached('run_detail_sb_running', 'could not start the long Scenario B run'); notReached('cancel_dialog', 'no running run to cancel');
  } else {
    for (const theme of THEMES) for (const [w, h] of VIEWPORTS) {
      await setCtx(w, h, theme);
      const route = `#/lab/runs/${ids.sbLive}`;
      await load(route);
      const st = await runStatusText();
      if (!/running|starting|queued/i.test(st)) { notReached(`run_detail_sb_running__${w}__${theme}`, `run no longer in progress: "${st.slice(0, 60)}"`); continue; }
      await capture('run_detail_sb_running', route, { full: true, note: 'Phases 5/30/30/30 s (not the 1 s phases) so the run stays in progress for the capture sweep; arms BASE+STATIC-CAP, 1 rep.', extraNotes: [`status: ${st.slice(0, 60)}`] });
      const opened = await b.clickText('Cancel run'); await sleep(600);
      if (!opened) { notReached(`cancel_dialog__${w}__${theme}`, 'Cancel run button not found'); continue; }
      const dlg = await b.eval(`[...document.querySelectorAll('dialog[open],[role=alertdialog],[role=dialog],[role=alert]')].map(e => e.innerText.replace(/\\s+/g,' ').slice(0, 120))`);
      await capture('cancel_dialog', route, { full: false, note: 'Cancel confirmation open on the running run', extraNotes: [`dialog: ${JSON.stringify(dlg)}`] });
    }
    await setCtx(1280, 800, 'light'); await load(`#/lab/runs/${ids.sbLive}`);
    await b.clickText('Cancel run'); await sleep(500);
    await b.eval(`[...document.querySelectorAll('button')].find(x => /^(yes|cancel the run|cancel run now|confirm)/i.test(x.innerText.trim()) )?.click()`);
    await b.waitFor(`/cancelled/i.test((document.querySelector('.lab-detail-head')||{}).innerText||'')`, 40000, 500);
  }

  // ---- Phase C: static sweep, every viewport x theme ----
  const STATES = [
    ['overview', '#/', false], ['traffic', '#/traffic', false], ['cache', '#/cache', false], ['freshness', '#/freshness', false],
    ['origin', '#/origin', false], ['lab_home', '#/lab', true], ['cluster', '#/cluster', false], ['intelligence', '#/intelligence', false],
    ['simulation', '#/simulation', false], ['lab_evidence_list', '#/lab/evidence', true], ['evidence_m6', `#/lab/evidence/${M6}`, true],
    ['evidence_scenario_b', `#/lab/evidence/${SB_EVID}`, true], ['run_form_t1_empty', '#/lab/run', true, 't1'], ['run_form_sb_empty', '#/lab/run', true, 'sb'],
    ['run_form_t1_invalid', '#/lab/run', true, 'invalid'], ['runs_history', '#/lab/runs', true],
    ...(ids.sbDone ? [['run_detail_sb_completed', `#/lab/runs/${ids.sbDone}`, true]] : []),
    ...(ids.t1Done ? [['run_detail_t1_completed', `#/lab/runs/${ids.t1Done}`, true]] : []),
  ];
  for (const theme of THEMES) for (const [w, h] of VIEWPORTS) {
    await setCtx(w, h, theme);
    for (const [state, route, full, variant] of STATES) {
      try {
        const ok = await load(route);
        let note = ok ? '' : 'page still showed a loading/connecting state after 20 s';
        if (variant === 'sb') { await b.click(`document.querySelector('label[data-kind-option="scenario_b"] input')`); await sleep(600); }
        if (variant === 't1') { await b.click(`document.querySelector('label[data-kind-option="t1_matrix"] input')`); await sleep(600); }
        if (variant === 'invalid') {
          await b.click(`document.querySelector('label[data-kind-option="t1_matrix"] input')`); await sleep(500);
          await b.setField('Number of seeds', 'abc');
          await b.eval(`(() => { const l=[...document.querySelectorAll('label')].find(x=>x.textContent.trim().startsWith('Number of seeds')); const el=document.getElementById(l.htmlFor); el.focus(); el.blur(); })()`);
          await b.click(`document.querySelector('[data-advanced] summary')`); await sleep(300);
          await b.setField('Run name', 'bad/name!');
          await b.clickText('Start run'); await sleep(700);
          const errs = await b.eval(`document.querySelectorAll('[data-field-error]').length`);
          note = `T1 form: Number of seeds="abc", Run name="bad/name!", nothing selected; Start run pressed once (failed submit). Field errors visible: ${errs}. (BEFORE typed the same values without pressing Start run.)`;
          if (!errs) { notReached(`${state}__${w}__${theme}.validation_message`, 'no field error appeared after a failed submit'); }
        }
        await capture(state, route, { full, note });
      } catch (e) { notReached(`${state}__${w}__${theme}`, String(e.message || e).slice(0, 160)); }
    }
  }

  // ---- Phase D: gateway stopped (disconnected/error state) ----
  killTree(spawned.gateway); await sleep(1500);
  manifest.notes.push('gateway process killed by recorded pid before the gateway_stopped captures');
  for (const theme of THEMES) for (const [w, h] of VIEWPORTS) {
    await setCtx(w, h, theme);
    for (const [state, route, full] of [['gateway_stopped_overview', '#/', false], ['gateway_stopped_lab_home', '#/lab', true], ['gateway_stopped_lab_runs', '#/lab/runs', false]]) {
      try { await load(route, { settle: 2500 }); await sleep(2500); await capture(state, route, { full, note: 'gateway process stopped' }); } catch (e) { notReached(`${state}__${w}__${theme}`, String(e.message || e).slice(0, 160)); }
    }
  }
} catch (e) {
  console.error('FATAL', e.stack || e); manifest.fatal = String(e.stack || e); exit = 1;
} finally {
  try { if (b) await b.close(); } catch { /* ignore */ }
  for (const pid of manifest.pids.chrome || []) killTree(pid);
  killTree(spawned.vite); killTree(spawned.gateway);
  await sleep(1000);
  manifest.portsFreeAfter = { 3000: !(await portBusy(3000)), 8085: !(await portBusy(8085)) };
  manifest.pidsAliveAfter = Object.entries(spawned).filter(([, p]) => pidAlive(p)).map(([n]) => n);
  try { if (LAB_DIR.startsWith('D:/Temp/gallery_lab_')) manifest.labDirLeft = LAB_DIR; } catch { /* ignore */ }
  manifest.afterUiSrcSha256 = manifest.uiSrcSha256;
  manifest.counts = { files: entries.length, fullPage: entries.filter((e) => e.fullPage).length, notReached: manifest.notReached.length };
  writeFileSync(join(OUT, 'manifest.json'), JSON.stringify(manifest, null, 2));
  log('wrote manifest', manifest.counts, manifest.portsFreeAfter);
  process.exit(exit);
}
