#!/usr/bin/env node
// scripts/demo_console/console_shots.mjs
// Drives the demo console in headless Chrome (ui/scripts/cdp-lib.mjs) through the key states and saves screenshots.
//   node scripts/demo_console/console_shots.mjs --dir <cluster dir> --evidence <dir> [--port 3201]
import { spawn } from 'node:child_process';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, sleep } from '../../ui/scripts/cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const DIR = opt('dir'), EVID = opt('evidence'), PORT = Number(opt('port', 3201));
if (!DIR || !EVID) { console.error('usage: --dir <cluster dir> --evidence <dir>'); process.exit(1); }

const srv = spawn(process.execPath, [join(HERE, 'server.mjs'), '--dir', DIR, '--port', String(PORT)], { stdio: 'ignore', windowsHide: true });
const b = new Browser({ evidenceDir: EVID, label: 'console' });
const log = (m) => console.log('[console-shots] ' + m);
let failed = 0;
const must = async (name, expr, t = 20000) => { const ok = await b.waitFor(expr, t); log(`${ok ? 'OK ' : 'FAIL'} ${name}`); if (!ok) failed += 1; return ok; };
const clickBtn = (sel) => b.click(`document.querySelector(${JSON.stringify(sel)})`);
const text = (sel) => b.eval(`document.querySelector(${JSON.stringify(sel)}).innerText`);
try {
  await sleep(1200);
  await b.launch(); await b.viewport(1400, 1000);
  await b.goto(`http://127.0.0.1:${PORT}/`, 2500);
  await must('cluster cards render with 3 running nodes', `document.querySelectorAll('#nodes .node').length === 3 && document.querySelectorAll('#nodes .node.dead').length === 0`);
  await b.shot('01_initial.png', '.', true);

  await clickBtn('[data-user="tenantA"]'); await clickBtn('[data-gw="1"]');
  await clickBtn('#b-load');
  await must('gallery: 13 tiles loaded with ticks', `document.querySelectorAll('.tile .tick').length >= 13`);
  log('tile badges (first load): ' + await b.eval(`[...document.querySelectorAll('.tile .badge')].map(e=>e.textContent).join(',')`));
  await b.shot('02_gallery_first_load_tenantA.png', '.', true);

  await clickBtn('#b-reload');
  await must('gallery reload: tiles show local/peer', `[...document.querySelectorAll('.tile .badge')].filter(e=>e.textContent==='local').length >= 10`);
  log('tile badges (reload): ' + await b.eval(`[...document.querySelectorAll('.tile .badge')].map(e=>e.textContent).join(',')`));
  await b.shot('03_gallery_reload_hits.png', '.', true);

  await clickBtn('[data-user="tenantB"]'); await clickBtn('#b-reload');
  await must('tenant B same images: first load is origin again (isolation)', `[...document.querySelectorAll('.tile .badge')].filter(e=>e.textContent==='origin').length >= 8`);
  log('tenant B badges: ' + await b.eval(`[...document.querySelectorAll('.tile .badge')].map(e=>e.textContent).join(',')`));
  await b.shot('04_gallery_tenantB_isolation.png', '.', true);

  await clickBtn('[data-user="none"]'); await clickBtn('#b-reload');
  await must('no token: tiles say 401 requires a token', `document.body.innerText.includes('401: this gateway requires a valid token')`);
  await b.shot('05_no_token_401.png', '.', true);

  await clickBtn('[data-user="readonly"]'); await clickBtn('#b-reload');
  await must('read-only user loads', `document.querySelectorAll('.tile .tick').length >= 13`);
  await clickBtn('[data-del]');
  await must('read-only DELETE answered 403', `document.getElementById('toast').innerText.includes('HTTP 403')`, 8000);
  await b.shot('06_readonly_delete_403.png', '.', false);

  await clickBtn('[data-user="tenantA"]'); await clickBtn('#b-msgs');
  await must('messages panel', `document.querySelectorAll('#msgs .msg .tick').length >= 6`);
  await clickBtn('[data-burst="1000,100"]');
  await must('burst 1000 done', `document.getElementById('burst-out').innerText.includes('0 errors') && document.getElementById('burst-out').innerText.includes('1000')`, 60000);
  const firstBurst = await text('#burst-out');
  log('burst: ' + (await text('#burst-out')).replace(/\n/g, ' | '));
  await b.shot('07_burst_1000.png', '.', true);

  await clickBtn('[data-node="3"][data-act="stop"]');
  await must('node 3 shows STOPPED', `document.querySelectorAll('#nodes .node.dead').length === 1`, 10000);
  await clickBtn('[data-burst="1000,100"]');
  await must('burst with node 3 dead done, 0 errors (fresh result)', `document.getElementById('burst-out').innerText.includes('0 errors') && document.getElementById('burst-out').innerText !== ${JSON.stringify(firstBurst)}`, 60000);
  log('burst (node 3 dead): ' + (await text('#burst-out')).replace(/\n/g, ' | '));
  await b.shot('08_node3_dead_burst.png', '.', true);

  await clickBtn('[data-node="3"][data-act="start"]');
  await must('node 3 running again', `document.querySelectorAll('#nodes .node.dead').length === 0`, 20000);
  await sleep(3000);
  await b.shot('09_node3_restarted.png', '.', true);
  b.saveLogs('console');
  log('console errors: ' + JSON.stringify(b.consoleLog.filter((c) => /exception|error/.test(c.kind)).slice(0, 5)));
} catch (e) { failed += 1; console.error(e); }
finally { await b.close(); try { srv.kill(); } catch { /* ignore */ } }
process.exit(failed ? 1 : 0);
