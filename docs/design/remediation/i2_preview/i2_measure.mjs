// I2 browser measurements (observe-only). Starts ONLY its own processes (anonymous debug gateway on 127.0.0.1:8085 with the same minimal
// lab fixture as the gallery, Vite on localhost:3000 --strictPort, headless Chrome with a throwaway profile under D:/Temp), measures the
// Lab pages in a real browser and writes measurements.json next to this script. It refuses to start if a port is busy and checks that
// the ports are free afterwards. No run is started. Run: node docs/design/remediation/i2_preview/i2_measure.mjs
import { spawn, execSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, pidAlive, sleep } from '../../../../ui/scripts/cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const UI_DIR = resolve(HERE, '../../../../ui');
const REPO = resolve(UI_DIR, '..');
const UI = 'http://localhost:3000';
const EXE = 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const STAMP = new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d+Z$/, 'Z');
const LAB_DIR = `D:/Temp/i2_measure_lab_${STAMP}`;
const M6 = 'm6_sealed_2026-10-29T234845Z', SBDIR = '20261030T003932Z', SB_EVID = `scenario_b.${SBDIR}`;
const out = { generated: new Date().toISOString(), browser: null, pids: {}, results: {}, notes: [] };
const portBusy = (port) => new Promise((res) => { const s = createConnection({ port, host: '127.0.0.1' }); s.on('connect', () => { s.destroy(); res(true); }); s.on('error', () => res(false)); });
const spawned = {};
function startProc(name, file, a, env, cwd) { const c = spawn(file, a, { env: { ...process.env, ...env }, cwd, stdio: 'ignore', windowsHide: true }); spawned[name] = c.pid; out.pids[name] = c.pid; return c; }
function killTree(pid) { if (pid && pidAlive(pid)) { try { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); } catch { /* gone */ } } }
async function waitHttp(url, ms = 60000) { const t0 = Date.now(); while (Date.now() - t0 < ms) { try { await fetch(url); return true; } catch { await sleep(300); } } return false; }
let b = null;
const READY = '!/Checking|Loading this|Loading the|Waiting for the gateway|Connecting/.test(document.body.innerText)';
async function load(route, w, h, theme = 'light') {
  await b.media([{ name: 'prefers-color-scheme', value: theme }]); await b.viewport(w, h, false, 1);
  await b.goto('about:blank', 150); await b.goto(`${UI}/${route}`, 1500);
  await b.waitFor(READY, 20000, 300); await sleep(900);
}
const rect = (sel) => `(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return { top: Math.round(r.top + scrollY), height: Math.round(r.height), width: Math.round(r.width), left: Math.round(r.left), bottom: Math.round(r.bottom + scrollY) }; })()`;

let exit = 0;
try {
  for (const p of [3000, 8085]) if (await portBusy(p)) throw new Error(`port ${p} busy; not ours`);
  mkdirSync(join(LAB_DIR, M6), { recursive: true }); mkdirSync(join(LAB_DIR, 'scenario_b', SBDIR), { recursive: true });
  const src = join(REPO, 'lab-results');
  for (const n of readdirSync(join(src, M6))) if (n === 'verdict.json' || n === 'manifest.json' || /^analysis_.*\.csv$/.test(n)) cpSync(join(src, M6, n), join(LAB_DIR, M6, n));
  for (const n of ['manifest.json', 'summary.json']) cpSync(join(src, 'scenario_b', SBDIR, n), join(LAB_DIR, 'scenario_b', SBDIR, n));
  const env = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: LAB_DIR, PHANTOM_BIND_ADDR: '127.0.0.1:8085', TEMP: 'D:/Temp', TMP: 'D:/Temp' };
  delete process.env.PHANTOM_SEALED_SALT_HEX; delete process.env.PHANTOM_JWT_SECRET;
  startProc('gateway', EXE, [], env, 'D:/Temp');
  startProc('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', '3000', '--strictPort', '--host', 'localhost'], { TEMP: 'D:/Temp', TMP: 'D:/Temp' }, UI_DIR);
  if (!(await waitHttp('http://127.0.0.1:8085/')) || !(await waitHttp(UI + '/'))) throw new Error('gateway or vite did not come up');
  await sleep(1500);
  b = await new Browser({ evidenceDir: HERE, label: 'i2m' }).launch(); out.browser = b.version; out.pids.chrome = b.pid;

  for (const [w, h] of [[1280, 800], [390, 844]]) {
    const R = (out.results[`${w}x${h}`] = {});
    // 1. Lab home: status strip, primary action, tabs, first heading
    await load('#/lab', w, h);
    R.lab_home = await b.eval(`(() => {
      const q = (s) => document.querySelector(s); const rc = (s) => { const e = q(s); if (!e) return null; const r = e.getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom), height: Math.round(r.height), width: Math.round(r.width) }; };
      const chips = [...document.querySelectorAll('.lab-status__chips li')].map((l) => Math.round(l.getBoundingClientRect().top));
      const rows = new Set(chips).size;
      const primary = rc('[data-primary-action]');
      const sections = [...document.querySelectorAll('h2')].slice(0, 1).map((e) => ({ text: e.innerText.slice(0, 40), top: Math.round(e.getBoundingClientRect().top) }));
      return { viewportHeight: innerHeight, h1: rc('h1'), primaryAction: primary, primaryVisible: !!primary && primary.bottom <= innerHeight, strip: rc('[data-lab-status]'), stripChipRows: rows, stripDetailsHidden: q('.lab-status__details').hidden,
        tabs: rc('.lab-subnav'), firstH2: sections[0] || null, bannersAboveTabs: document.querySelectorAll('.banner').length, pageHeight: document.documentElement.scrollHeight, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth };
    })()`);
    // 2. Start a run (default kind, pristine)
    await load('#/lab/run', w, h);
    R.run_form_pristine = await b.eval(`(() => {
      const rc = (s) => { const e = document.querySelector(s); if (!e) return null; const r = e.getBoundingClientRect(); return { top: Math.round(r.top), bottom: Math.round(r.bottom), height: Math.round(r.height) }; };
      const legend = [...document.querySelectorAll('legend')][0];
      const first = document.querySelector('.lab-form input, .lab-form button');
      return { viewportHeight: innerHeight, firstLegend: legend ? { text: legend.innerText, top: Math.round(legend.getBoundingClientRect().top) } : null, firstInput: rc('.lab-form input[type=text]'), startButton: rc('.form-bar .btn--primary'), formBar: rc('[data-form-bar]'),
        visibleFieldErrors: [...document.querySelectorAll('[data-field-error]')].filter((e) => e.offsetParent !== null).length, errorSummary: !!document.querySelector('[data-error-summary]'),
        kindChecked: (document.querySelector('label[data-kind-option] input:checked') || {}).closest ? document.querySelector('label[data-kind-option] input:checked').closest('label').getAttribute('data-kind-option') : null,
        pageHeight: document.documentElement.scrollHeight, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth };
    })()`);
    // matrix form: height with all three long groups closed
    await b.click(`document.querySelector('label[data-kind-option="t1_matrix"] input')`); await sleep(500);
    R.run_form_t1_pristine = await b.eval(`(() => ({ pageHeight: document.documentElement.scrollHeight, groups: [...document.querySelectorAll('.lab-choice-group > details')].map((d) => ({ open: d.open, summary: d.querySelector('summary').innerText.replace(/\\s+/g, ' ') })), visibleFieldErrors: [...document.querySelectorAll('[data-field-error]')].filter((e) => e.offsetParent !== null).length, checkboxesInDom: document.querySelectorAll('input[type=checkbox]').length, checkboxesVisible: [...document.querySelectorAll('input[type=checkbox]')].filter((e) => e.offsetParent !== null).length }))()`);
    // open everything to show the worst case height
    await b.eval(`document.querySelectorAll('.lab-choice-group > details').forEach((d) => { d.open = true; })`); await sleep(400);
    R.run_form_t1_all_groups_open = await b.eval(`({ pageHeight: document.documentElement.scrollHeight })`);
    // select all arms, then clear: counts update
    await b.eval(`(() => { const g = document.querySelector('[data-choice-group="arms"]'); const btn = [...g.querySelectorAll('button')].find((x) => /select all/i.test(x.innerText)); btn.click(); })()`); await sleep(300);
    R.select_all_arms = await b.eval(`(document.querySelector('[data-choice-group="arms"] .lab-choice-count') || {}).innerText`);
    // failed submit: focus, summary, inline errors, advanced opened
    await b.eval(`document.querySelectorAll('.lab-choice-group > details').forEach((d) => { d.open = false; })`);
    await b.clickText('Start run'); await sleep(600);
    R.failed_submit = await b.eval(`(() => ({ activeIsSummary: document.activeElement && document.activeElement.hasAttribute('data-error-summary'), summaryItems: [...document.querySelectorAll('[data-error-summary] li')].map((l) => l.innerText), visibleFieldErrors: [...document.querySelectorAll('[data-field-error]')].filter((e) => e.offsetParent !== null).length, startDisabled: document.querySelector('.form-bar .btn--primary').getAttribute('aria-disabled') }))()`);
    // 3. evidence pages: heights, contents, caption widths, rendered notice
    await load(`#/lab/evidence/${SB_EVID}`, w, h);
    R.evidence_scenario_b = await b.eval(`(() => {
      const notice = document.querySelector('[data-notice-pane="rendered"]');
      const axes = [...document.querySelectorAll('.chart-frame__axes')].map((a) => { const f = a.closest('.chart-frame'); const plot = f.querySelector('.chart-frame__pane:not([hidden])'); return { captionWidth: Math.round(a.getBoundingClientRect().width), plotWidth: Math.round(plot.getBoundingClientRect().width), captionLines: Math.round(a.getBoundingClientRect().height / parseFloat(getComputedStyle(a).lineHeight)) }; });
      return { pageHeight: document.documentElement.scrollHeight, contentsLinks: document.querySelectorAll('.lab-toc a').length, closedFolds: [...document.querySelectorAll('details.lab-fold')].filter((d) => !d.open).length,
        noticeRendered: !!notice, noticeHasRawMarkers: notice ? /\\*\\*|\`|^#\\s/m.test(notice.innerText) : null, noticeHeadings: notice ? [...notice.querySelectorAll('h3,h4,h5')].length : null, noticeScrollBox: notice ? notice.scrollHeight > notice.clientHeight + 1 : null,
        charts: document.querySelectorAll('.chart-frame').length, tableToggles: document.querySelectorAll('.chart-frame .seg').length, axes, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth };
    })()`);
    await load(`#/lab/evidence/${M6}`, w, h);
    R.evidence_m6 = await b.eval(`(() => {
      const axes = [...document.querySelectorAll('.chart-frame__axes')].map((a) => { const f = a.closest('.chart-frame'); const plot = f.querySelector('.chart-frame__pane:not([hidden])'); return { captionWidth: Math.round(a.getBoundingClientRect().width), plotWidth: Math.round(plot.getBoundingClientRect().width), captionLines: Math.round(a.getBoundingClientRect().height / parseFloat(getComputedStyle(a).lineHeight)) }; });
      return { pageHeight: document.documentElement.scrollHeight, contentsLinks: document.querySelectorAll('.lab-toc a').length, closedFolds: [...document.querySelectorAll('details.lab-fold')].filter((d) => !d.open).length, axes, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth };
    })()`);
    // 4. keyboard: Tab order on the lab home reaches the primary action before the strip details and the tabs
    await load('#/lab', w, h);
    const order = [];
    for (let i = 0; i < 12; i++) { await b.tab(); order.push(await b.eval(`(() => { const e = document.activeElement; return (e.getAttribute('data-primary-action') ? 'primary:' : '') + (e.innerText || e.getAttribute('aria-label') || e.tagName).trim().replace(/\\s+/g, ' ').slice(0, 30); })()`)); }
    R.tab_order_lab_home = order;
    // 5. contents link: scroll + focus + hash untouched + closed fold opens
    await load(`#/lab/evidence/${SB_EVID}`, w, h);
    R.contents_link = await b.eval(`(() => { const hashBefore = location.hash; const a = [...document.querySelectorAll('.lab-toc a')].find((x) => x.getAttribute('href') === '#sb-paired'); a.click(); const d = document.getElementById('sb-paired'); return { hashUnchanged: location.hash === hashBefore, foldOpened: d.open, focusIsSummary: document.activeElement === d.querySelector('summary'), top: Math.round(d.getBoundingClientRect().top) }; })()`);
    // 6. details strip toggle
    await load('#/lab', w, h);
    await b.clickText('Details'); await sleep(300);
    R.strip_open = await b.eval(`(() => { const r = document.querySelector('[data-lab-status]').getBoundingClientRect(); return { expanded: document.querySelector('.lab-status__toggle').getAttribute('aria-expanded'), height: Math.round(r.height), detailsVisible: !document.querySelector('.lab-status__details').hidden }; })()`);
    // 7. runs history
    await load('#/lab/runs', w, h);
    R.runs_history = await b.eval(`({ pageHeight: document.documentElement.scrollHeight, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth })`);
  }
  // reduced motion + forced colors smoke at 1280: dialog and progress animation absent
  out.consoleErrors = b.consoleLog.filter((e) => /exception|log\.error|console\.error/.test(e.kind)).slice(0, 10);
} catch (e) {
  console.error('FATAL', e.stack || e); out.fatal = String(e.stack || e); exit = 1;
} finally {
  try { if (b) await b.close(); } catch { /* ignore */ }
  killTree(spawned.vite); killTree(spawned.gateway);
  await sleep(1000);
  out.portsFreeAfter = { 3000: !(await portBusy(3000)), 8085: !(await portBusy(8085)) };
  out.pidsAliveAfter = Object.entries(spawned).filter(([, p]) => pidAlive(p)).map(([n]) => n);
  writeFileSync(join(HERE, 'measurements.json'), JSON.stringify(out, null, 2));
  console.log('wrote measurements.json', out.portsFreeAfter);
  process.exit(exit);
}
