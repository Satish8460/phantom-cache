// V2A evidence capture (Phantom Lab and evidence screens). Dependency-free: Node built-ins + ./cdp-lib.mjs.
//
//   node ui/scripts/browser-v2a.mjs --out <dir> [--only <state,state>]
//
// Starts ONLY what it spawns: an anonymous debug gateway (127.0.0.1:8113, Lab API enabled, cluster off, no token), Vite on
// localhost:3400 (--strictPort) and headless Chrome with throwaway profiles under D:/Temp. It never starts a run. Run records are
// served from a throwaway lab dir that holds READ-ONLY COPIES of:
//   * the retained M6 sealed and Scenario B evidence from the repo's lab-results (verdict/manifest/summary files only);
//   * two genuine LEGACY run directories (cancelled, completed) from D:/Temp/usab_lab/runs (unpatched backend output);
//   * one NEW-CONTRACT cancelled run: the legacy cancelled run record with summary.json REPLACED by the sandbox-patched
//     summarize() output (ui/test/fixtures/func/sandbox_u01/cancelled_partial.summary.json). This is a SYNTHETIC MIX (legacy run
//     record + sandbox summary), not a run the backend wrote; it exists to show how the UI treats the new contract.
// It refuses to start when ports 3400 or 8097 are busy and checks that they are free afterwards.
import { spawn, execSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, pidAlive, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
if (!args.out) { console.error('usage: node ui/scripts/browser-v2a.mjs --out <dir>'); process.exit(2); }
const ONLY = args.only ? new Set(args.only.split(',')) : null;
const OUT = resolve(args.out);
const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, '..');
const REPO = resolve(UI_DIR, '..');
const UI_PORT = 3400; const GW_PORT = 8113; const DEAD_PORT = 8114;
const UI = `http://localhost:${UI_PORT}`;
const GW = `http://127.0.0.1:${GW_PORT}`;
const EXE = args['gateway-exe'] || 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const LAB_DIR = `D:/Temp/v2a_lab_${Date.now()}`;
const M6 = 'm6_sealed_2026-10-29T234845Z'; const SBDIR = '20261030T003932Z'; const SB_EVID = `scenario_b.${SBDIR}`;
const LEG_CANCELLED = '20261030T085920Z-9c26980f'; const LEG_COMPLETED = '20261030T085851Z-5ce0cf9f'; const NEW_CANCELLED = '20261030T085930Z-0badc0de';
const VIEWPORTS = [[390, 844], [820, 1180], [1280, 800]];
const THEMES = ['light', 'dark'];

mkdirSync(OUT, { recursive: true });
const report = { generated: new Date().toISOString(), script: 'ui/scripts/browser-v2a.mjs', gateway: EXE, labDir: LAB_DIR, shots: [], notes: [], pids: {} };
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const spawned = {};
const portBusy = (port) => new Promise((res) => { const s = createConnection({ port, host: '127.0.0.1' }); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
function startProc(name, file, a, env, cwd) {
  const c = spawn(file, a, { env: { ...process.env, ...env }, cwd, stdio: 'ignore', windowsHide: true });
  spawned[name] = c.pid; report.pids[name] = c.pid; log('started', name, c.pid);
}
function killTree(pid) { if (pid && pidAlive(pid)) { try { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); } catch { /* gone */ } } }
async function waitHttp(url, ms = 60000) { const t0 = Date.now(); while (Date.now() - t0 < ms) { try { await fetch(url); return true; } catch { await sleep(300); } } return false; }

let b = null; let loads = 0; let cur = { w: 1280, h: 800, theme: 'light' };
async function newBrowser() { b = await new Browser({ evidenceDir: OUT, label: 'v2a' }).launch(); report.chrome = b.version; await b.media([{ name: 'prefers-color-scheme', value: cur.theme }]); await b.viewport(cur.w, cur.h, false, 1); }
async function relaunch() { await b.close(); await newBrowser(); }
async function setCtx(w, h, theme) { cur = { w, h, theme }; await b.media([{ name: 'prefers-color-scheme', value: theme }]); await b.viewport(w, h, false, 1); }
const READY = `!/Checking|Loading this|Loading the|Loading run|Waiting for the gateway|Connecting/.test(document.body.innerText)`;
async function load(route, { gateway = GW, settle = 1500 } = {}) {
  if (++loads % 3 === 0) await relaunch();
  await b.goto('about:blank', 150);
  await b.goto(`${UI}/?gateway=${encodeURIComponent(gateway)}${route}`, settle);
  const ok = await b.waitFor(READY, 20000, 300);
  await sleep(900);
  return ok;
}
const docH = () => b.eval('Math.max(document.documentElement.scrollHeight, document.body.scrollHeight)');
async function shotFull(name) {
  let h = await docH(); const w0 = cur.w; const h0 = cur.h;
  await b.viewport(w0, Math.min(Math.max(h, h0), 16000), false, 1); await sleep(500);
  const h2 = await docH(); if (h2 !== h) { await b.viewport(w0, Math.min(Math.max(h2, h0), 16000), false, 1); await sleep(400); }
  const s = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  writeFileSync(join(OUT, name), Buffer.from(s.data, 'base64'));
  await b.viewport(w0, h0, false, 1); await sleep(300);
}
const probe = () => b.eval(`(() => { const h = document.querySelector('h1'); const marks = (sel) => document.querySelectorAll(sel).length;
  return { h1: h ? h.innerText : '', scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth,
  verdictChips: marks('[data-verdict-arm]'), noVerdict: marks('[data-no-verdict]'), partial: marks('[data-partial-diagnostics]'), notEstablished: marks('[data-not-established]'),
  backdropInMain: [...document.querySelectorAll('main *')].filter((e) => { const s = getComputedStyle(e); return s.backdropFilter && s.backdropFilter !== 'none'; }).length,
  bodyHasVALID: /\\bVALID\\b/.test((document.querySelector('[data-lab-body]') || document.body).innerText) }; })()`);
async function capture(state, { full = false, vp = VIEWPORTS, themes = THEMES, route, prep, gateway } = {}) {
  if (ONLY && !ONLY.has(state)) return;
  for (const theme of themes) for (const [w, h] of vp) {
    await setCtx(w, h, theme);
    const ok = await load(route, { gateway });
    if (prep) await prep();
    const info = await probe().catch((e) => ({ error: String(e) }));
    const stem = `${state}__${w}__${theme}`;
    await b.shot(`${stem}.png`, '.', false);
    if (full) await shotFull(`${stem}__full.png`);
    report.shots.push({ file: `${stem}.png`, full: full ? `${stem}__full.png` : null, route, ready: ok, ...info });
    log('shot', stem, ok ? '' : 'NOT READY', JSON.stringify({ sw: info.scrollWidth, cw: info.clientWidth }));
  }
}

let exit = 0;
try {
  for (const p of [UI_PORT, GW_PORT, DEAD_PORT]) if (await portBusy(p)) throw new Error(`port ${p} is not free; refusing to start`);
  if (!existsSync(EXE)) throw new Error('gateway exe missing: ' + EXE);
  const src = join(REPO, 'lab-results');
  mkdirSync(join(LAB_DIR, M6), { recursive: true }); mkdirSync(join(LAB_DIR, 'scenario_b', SBDIR), { recursive: true }); mkdirSync(join(LAB_DIR, 'runs'), { recursive: true });
  for (const n of readdirSync(join(src, M6))) if (n === 'verdict.json' || n === 'manifest.json' || /^analysis_.*\.csv$/.test(n)) cpSync(join(src, M6, n), join(LAB_DIR, M6, n));
  for (const n of ['manifest.json', 'summary.json']) cpSync(join(src, 'scenario_b', SBDIR, n), join(LAB_DIR, 'scenario_b', SBDIR, n));
  const legacy = 'D:/Temp/usab_lab/runs';
  for (const id of [LEG_CANCELLED, LEG_COMPLETED]) cpSync(join(legacy, id), join(LAB_DIR, 'runs', id), { recursive: true });
  // new-contract cancelled run (synthetic mix, see header)
  cpSync(join(legacy, LEG_CANCELLED), join(LAB_DIR, 'runs', NEW_CANCELLED), { recursive: true });
  const rj = JSON.parse(readFileSync(join(LAB_DIR, 'runs', NEW_CANCELLED, 'run.json'), 'utf8')); rj.run_id = NEW_CANCELLED;
  const sum = readFileSync(join(UI_DIR, 'test/fixtures/func/sandbox_u01/cancelled_partial.summary.json'));
  writeFileSync(join(LAB_DIR, 'runs', NEW_CANCELLED, 'out', 'summary.json'), sum);
  const { createHash } = await import('node:crypto');
  rj.artifact_index['summary.json'] = { bytes: sum.length, sha256: createHash('sha256').update(sum).digest('hex') };
  writeFileSync(join(LAB_DIR, 'runs', NEW_CANCELLED, 'run.json'), JSON.stringify(rj, null, 2));
  report.notes.push(`runs served: legacy cancelled ${LEG_CANCELLED}, legacy completed ${LEG_COMPLETED}, new-contract cancelled ${NEW_CANCELLED} (legacy run record + SANDBOX summary, synthetic mix)`);

  const env = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: LAB_DIR, PHANTOM_BIND_ADDR: `127.0.0.1:${GW_PORT}`, PHANTOM_CORS_ORIGINS: UI, TEMP: 'D:/Temp', TMP: 'D:/Temp' };
  delete process.env.PHANTOM_SEALED_SALT_HEX; delete process.env.PHANTOM_JWT_SECRET;
  startProc('gateway', EXE, [], env, 'D:/Temp');
  startProc('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', String(UI_PORT), '--strictPort', '--host', 'localhost'], { TEMP: 'D:/Temp', TMP: 'D:/Temp' }, UI_DIR);
  if (!(await waitHttp(`${GW}/`)) || !(await waitHttp(UI + '/'))) throw new Error('gateway or vite did not come up');
  await sleep(1500);
  await newBrowser();

  await capture('lab_about', { route: '#/lab', full: true });
  await capture('evidence_list', { route: '#/lab/evidence', full: true });
  await capture('evidence_sb', { route: `#/lab/evidence/${SB_EVID}`, full: true });
  await capture('evidence_m6', { route: `#/lab/evidence/${M6}`, full: true, vp: [[1280, 800], [390, 844]], themes: ['light'] });
  await capture('run_cancelled_legacy', { route: `#/lab/runs/${LEG_CANCELLED}`, full: true });
  await capture('run_cancelled_newcontract', { route: `#/lab/runs/${NEW_CANCELLED}`, full: true });
  await capture('run_completed_legacy', { route: `#/lab/runs/${LEG_COMPLETED}`, full: true });
  await capture('runs_list', { route: '#/lab/runs', vp: [[1280, 800], [390, 844]], themes: ['light'] });
  await capture('run_form', { route: '#/lab/run', full: true });
  await capture('run_form_sb_preset', { route: '#/lab/run', vp: [[1280, 800], [390, 844]], themes: ['light'], prep: async () => {
    await b.click(`document.querySelector('label[data-kind-option="scenario_b"] input')`); await sleep(300);
    await b.click(`document.querySelector('[data-preset]')`); await sleep(400);
    await b.click(`document.querySelector('[data-advanced] summary')`); await sleep(300);
    for (const [l, v] of [['Warm-up', 2], ['Healthy', 3], ['Degraded', 4], ['Recovery', 3]]) await b.setField(l, v);
    await sleep(400);
  } });
  await capture('run_form_text200', { route: '#/lab/run', vp: [[390, 844]], themes: ['light', 'dark'], prep: async () => {
    await b.eval(`document.documentElement.style.fontSize = '200%'`); await sleep(500);
  } });
  await capture('gateway_stopped_lab', { route: '#/lab', gateway: `http://127.0.0.1:${DEAD_PORT}`, full: true });

  writeFileSync(join(OUT, '_capture_report.json'), JSON.stringify(report, null, 2));
} catch (e) {
  console.error('FAILED', e); exit = 1; report.error = String(e); try { writeFileSync(join(OUT, '_capture_report.json'), JSON.stringify(report, null, 2)); } catch { /* ignore */ }
} finally {
  try { await b?.close(); } catch { /* ignore */ }
  for (const [n, pid] of Object.entries(spawned)) { killTree(pid); log('stopped', n, pid); }
  await sleep(800);
  for (const p of [UI_PORT, GW_PORT]) log('port', p, (await portBusy(p)) ? 'STILL BUSY' : 'free');
}
process.exit(exit);
