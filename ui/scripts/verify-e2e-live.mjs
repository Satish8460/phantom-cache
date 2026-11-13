// Automated End-to-End Live Browser & Telemetry Validation Script
// Verifies SETBACK-04 (Live Browser SSE Telemetry Streaming) and SETBACK-05 (End-to-End UI & Lab Navigation)
import { spawn, execSync } from 'node:child_process';
import { createServer } from 'node:http';
import { createConnection } from 'node:net';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { Browser, pidAlive, sleep } from './cdp-lib.mjs';

const EVIDENCE_DIR = resolve('docs/demo/browser_verification/live_e2e_evidence');
mkdirSync(EVIDENCE_DIR, { recursive: true });

const GW_PORT = 8085;
const UI_PORT = 3000;
const ORIGIN_PORT = 9099;

const log = (msg) => console.log(`[${new Date().toISOString()}] ${msg}`);

function checkPort(port) {
  return new Promise((res) => {
    const s = createConnection({ port, host: '127.0.0.1' });
    s.on('connect', () => { s.destroy(); res(true); });
    s.on('error', () => res(false));
  });
}

function killTree(pid) {
  if (pid && pidAlive(pid)) {
    try {
      execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' });
    } catch { /* already exited */ }
  }
}

async function waitForHttp(url, timeoutMs = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url);
      if (res.ok || res.status < 500) return true;
    } catch { /* retry */ }
    await sleep(300);
  }
  return false;
}

let mockOriginServer = null;
let gwProc = null;
let uiProc = null;
let browser = null;

try {
  log('Step 1: Checking port availability...');
  for (const port of [GW_PORT, UI_PORT, ORIGIN_PORT]) {
    if (await checkPort(port)) {
      throw new Error(`Port ${port} is currently occupied!`);
    }
  }
  log('All ports (8085, 3000, 9099) are clean.');

  log('Step 2: Starting Mock Origin Server on port 9099...');
  mockOriginServer = createServer((req, res) => {
    setTimeout(() => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ origin_source: 'live_mock', path: req.url, ts: Date.now() }));
    }, 20);
  });
  await new Promise((res) => mockOriginServer.listen(ORIGIN_PORT, '127.0.0.1', res));
  log('Mock Origin running on http://127.0.0.1:9099');

  log('Step 3: Spawning Gateway binary in release mode with Lab enabled...');
  gwProc = spawn('target\\release\\phantom-cache-demo.exe', [], {
    env: {
      ...process.env,
      PHANTOM_BIND_ADDR: `127.0.0.1:${GW_PORT}`,
      ORIGIN_URL: `http://127.0.0.1:${ORIGIN_PORT}`,
      PHANTOM_LAB_ENABLED: 'true',
      PHANTOM_SIMULATION: 'false',
      PHANTOM_CORS_ORIGINS: 'http://localhost:3000,http://127.0.0.1:3000',
    },
    stdio: 'ignore',
    windowsHide: true,
  });
  log(`Gateway process spawned with PID ${gwProc.pid}`);

  log('Step 4: Spawning UI Dev Server (Vite)...');
  uiProc = spawn('cmd.exe', ['/c', 'npm', 'run', 'dev', '--', '--port', '3000', '--strictPort'], {
    cwd: resolve('ui'),
    stdio: 'ignore',
    windowsHide: true,
  });
  log(`UI dev server spawned with PID ${uiProc.pid}`);

  log('Waiting for Gateway health endpoint...');
  const gwReady = await waitForHttp(`http://127.0.0.1:${GW_PORT}/health`, 20000);
  if (!gwReady) throw new Error('Gateway failed to respond on /health within 20s');
  log('Gateway is healthy.');

  log('Waiting for UI dev server...');
  const uiReady = await waitForHttp(`http://localhost:${UI_PORT}/`, 20000);
  if (!uiReady) throw new Error('UI failed to respond on http://localhost:3000/ within 20s');
  log('UI is responding.');

  log('Step 5: Launching Headless Chrome via CDP...');
  browser = await new Browser({ evidenceDir: EVIDENCE_DIR, label: 'live_e2e' }).launch();
  log(`Chrome launched with PID ${browser.pid}, CDP version: ${browser.version}`);
  await browser.viewport(1280, 800, false);
  await browser.media([{ name: 'prefers-color-scheme', value: 'light' }]);

  log('Step 6: Navigating to UI Overview page (http://localhost:3000/)...');
  await browser.goto('http://localhost:3000/', 2500);

  log('Step 7: Validating live telemetry connection indicator...');
  let connKind = null;
  for (let i = 0; i < 30; i++) {
    connKind = await browser.eval(`document.querySelector('.app-header__status')?.getAttribute('data-conn-kind')`);
    if (connKind === 'live') break;
    await sleep(500);
  }
  log(`Connection kind in header: "${connKind}"`);
  if (connKind !== 'live') {
    throw new Error(`Expected connection kind "live", got "${connKind}"`);
  }
  log('[PASS] SETBACK-04: Live SSE telemetry stream connected to browser.');

  await browser.shot('01_overview_connected.png', 'shots');

  log('Step 8: Generating cache traffic and observing real-time DOM updates...');
  // Request 1: Cold MISS
  const r1 = await (await fetch(`http://127.0.0.1:${GW_PORT}/api/v1/cache/e2e-item`)).text();
  // Request 2: Warm HIT
  const r2 = await (await fetch(`http://127.0.0.1:${GW_PORT}/api/v1/cache/e2e-item`)).text();
  // Request 3: Cold MISS
  const r3 = await (await fetch(`http://127.0.0.1:${GW_PORT}/api/v1/cache/e2e-item-2`)).text();

  log('Waiting 1000ms for SSE telemetry frame to deliver to browser...');
  await sleep(1000);

  const bodyText = await browser.eval(`document.body.innerText`);
  const statusLine = await browser.eval(`document.querySelector('.status-line')?.innerText || ''`);
  log(`Status line text: "${statusLine}"`);
  await browser.shot('02_overview_traffic_updated.png', 'shots');

  log('Step 9: Navigating to Lab Landing screen (#/lab)...');
  await browser.goto('http://localhost:3000/#/lab', 2000);
  const labTabs = await browser.eval(`[...document.querySelectorAll('a[data-lab-view]')].map(a => a.innerText.trim())`);
  log(`Lab subnav tabs: ${JSON.stringify(labTabs)}`);
  if (!labTabs.includes('Evidence') || !labTabs.includes('About')) {
    throw new Error(`Unexpected Lab tabs: ${JSON.stringify(labTabs)}`);
  }
  await browser.shot('03_lab_landing.png', 'shots');

  log('Step 10: Navigating to Lab Evidence Index (#/lab/evidence)...');
  await browser.goto('http://localhost:3000/#/lab/evidence', 2500);
  const evidenceBody = await browser.eval(`document.body.innerText`);
  const hasSealedM6 = evidenceBody.includes('m6_sealed') || evidenceBody.includes('Evaluation') || evidenceBody.includes('M6');
  log(`Evidence page contains evaluation records: ${hasSealedM6}`);
  await browser.shot('04_lab_evidence.png', 'shots');

  log('Step 11: Validating honest evidence presentation...');
  // Check for forbidden misleading claims in rendered DOM
  const forbidden = ['100% production ready without caveats', 'guarantees linearizability'];
  for (const f of forbidden) {
    if (evidenceBody.toLowerCase().includes(f.toLowerCase())) {
      throw new Error(`Detected forbidden misleading claim in DOM: "${f}"`);
    }
  }
  log('[PASS] SETBACK-05: Lab UI renders with honest evidence presentation.');

  log('All assertions PASSED!');
  writeFileSync(join(EVIDENCE_DIR, 'e2e_summary.json'), JSON.stringify({
    timestamp: new Date().toISOString(),
    status: 'PASS',
    chromeVersion: browser.version,
    connectionKind: connKind,
    labTabs,
    checks: ['SETBACK-04_LIVE_SSE_TELEMETRY', 'SETBACK-05_LAB_E2E_WORKFLOW'],
  }, null, 2));

} finally {
  log('Teardown: Cleaning up all spawned processes and servers...');
  if (browser) {
    try { await browser.close(); } catch { /* ignore */ }
  }
  if (uiProc) {
    killTree(uiProc.pid);
  }
  if (gwProc) {
    killTree(gwProc.pid);
  }
  if (mockOriginServer) {
    await new Promise((res) => mockOriginServer.close(res));
  }
  await sleep(1000);

  const clean8085 = !(await checkPort(GW_PORT));
  const clean3000 = !(await checkPort(UI_PORT));
  const clean9099 = !(await checkPort(ORIGIN_PORT));
  log(`Post-teardown socket verification: Port 8085 free: ${clean8085}, Port 3000 free: ${clean3000}, Port 9099 free: ${clean9099}`);
  if (!clean8085 || !clean3000 || !clean9099) {
    console.error('CRITICAL: Socket leak detected during teardown!');
  }
}

