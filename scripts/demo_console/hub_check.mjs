#!/usr/bin/env node
// scripts/demo_console/hub_check.mjs -- test the Demo Hub as a naive first-time user (headless Chrome through ui/scripts/cdp-lib.mjs).
//   node scripts/demo_console/hub_check.mjs --exe <gateway> --evidence <dir> [--work <empty dir>] [--port 3201]
//        [--reps 1] [--storylines 1] [--seed 7] [--skip a,b] [--http-port-base 18200 --cluster-port-base 19601 --origin-port 19200]
// Starts the hub on an EMPTY work directory and then: checks the 30 required scenario ids (card + Run button), starts the cluster
// from the page, opens the tour, runs the whole storyline N times (same verdicts each time), runs every scenario individually
// in a seeded random order (--reps times each), checks the cluster is healthy at the end, the session summary strip, a failure
// state, the phone layout and that the page prints no token and no console errors.
import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, rmSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, sleep } from '../../ui/scripts/cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));

// Ensure Git usr/bin (openssl.exe) is in PATH on Windows if available
if (process.platform === 'win32') {
  const gitUsrBin = 'C:\\Program Files\\Git\\usr\\bin';
  if (existsSync(join(gitUsrBin, 'openssl.exe')) && !process.env.PATH.includes(gitUsrBin)) {
    process.env.PATH = gitUsrBin + ';' + process.env.PATH;
  }
}
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const EXE = opt('exe'), EVID = opt('evidence'); const PORT = Number(opt('port', 3201));
if (!EXE || !EVID) { console.error('usage: --exe <gateway> --evidence <dir>'); process.exit(1); }
const WORK = resolve(opt('work', join(EVID, 'hub_work')));
const REPS = Number(opt('reps', 1)), STORIES = Number(opt('storylines', 1)); let SEED = Number(opt('seed', 7));
const SKIP = opt('skip', '').split(',').filter(Boolean);
const REQUIRED = ['tour', 'gallery', 'stampede', 'peerfill', 'offload', 'workloads', 'scale', 'auth', 'roles', 'isolation', 'targetsafety', 'mtls', 'failclosed', 'multiuser', 'invalidation', 'deadnode', 'deletestorm', 'burstdelete', 'peerexpiry', 'freshness', 'eviction', 'largeobjects', 'originprotect', 'operability', 'restart', 'intelligence', 'intelcontrol', 'pie-hotkey', 'pie-flashcrowd', 'pie-shift', 'pie-eviction', 'pie-admission', 'dashboards', 'benchmark', 'everything'];
const PB = opt('http-port-base', '18200'), CB = opt('cluster-port-base', '19601'), OP = opt('origin-port', '19200');
rmSync(WORK, { recursive: true, force: true }); mkdirSync(WORK, { recursive: true }); mkdirSync(EVID, { recursive: true });

let pass = 0, fail = 0; const results = [];
const ok = (cond, name, detail = '') => { (cond ? pass++ : fail++); results.push({ name, pass: !!cond, detail }); console.log(`[${cond ? 'PASS' : 'FAIL'}] ${name}${detail ? ' :: ' + String(detail).slice(0, 240) : ''}`); return !!cond; };
const srv = spawn(process.execPath, [join(HERE, 'server.mjs'), '--dir', WORK, '--port', String(PORT), '--exe', EXE, '--http-port-base', PB, '--cluster-port-base', CB, '--origin-port', OP, '--scenario-offset', opt('scenario-offset', '0')], { stdio: 'ignore', windowsHide: true, env: process.env });
const b = new Browser({ evidenceDir: EVID, label: 'hub' });
const click = (sel) => b.click(`document.querySelector(${JSON.stringify(sel)})`);
const txt = (sel) => b.eval(`(document.querySelector(${JSON.stringify(sel)})||{}).innerText||''`);
const shot = (n, full = false) => b.shot(n, '.', full);
const launcher = (args) => spawnSync(process.execPath, [join(HERE, '..', 'cluster_launcher.mjs'), ...args, '--dir', WORK], { encoding: 'utf8' });
const state = () => fetch(`http://127.0.0.1:${PORT}/api/state`).then((r) => r.json());
const rnd = () => { SEED = (SEED * 1664525 + 1013904223) >>> 0; return SEED / 4294967296; };
const verdictOf = async (id) => { const s = await state(); const j = (s.jobs || []).filter((x) => x.scenario === id).pop(); return j ? { v: j.verdict, checks: j.checks, ms: j.elapsedMs, nums: (j.numbers || []).length } : null; };
async function runCard(id) {
  await b.waitFor(`!document.querySelector('[data-run="${id}"]').disabled`, 600000);
  const before = (await state()).jobs.filter((x) => x.scenario === id).length;
  await b.eval(`document.querySelector('#card-${id}').scrollIntoView({block:'center'})`);
  await click(`#card-${id} [data-run="${id}"]`);
  const t0 = Date.now(); let v = null;
  while (Date.now() - t0 < 600000) { await sleep(1500); const s = await state(); const js = s.jobs.filter((x) => x.scenario === id); if (js.length > before && !s.job) { v = js[js.length - 1]; break; } }
  return v;
}

try {
  await sleep(1500);
  await b.launch(); await b.viewport(1440, 1000);
  await b.goto(`http://127.0.0.1:${PORT}/`, 2000);
  ok(await b.waitFor(`document.querySelector('#start h2')`, 8000), 'landing: "Start here" section is shown');
  ok((await txt('#start')).includes('caching gateway'), 'landing: explains what PhantomCache is in plain words');
  ok(await b.waitFor(`document.querySelector('#banner').innerText.includes('Start the demo cluster')`, 8000), 'landing: cluster not running -> a clear "Start the demo cluster" button');
  const cards = await b.eval(`[...document.querySelectorAll('.card')].map(c => ({ id: c.id.replace('card-',''), run: !!c.querySelector('[data-run]') }))`);
  for (const id of REQUIRED) { const c = cards.find((x) => x.id === id); ok(!!c && c.run, `required scenario "${id}": card with a Run button`); }
  ok(cards.length >= REQUIRED.length, 'catalogue has every required id', `${cards.length} cards`);
  ok(JSON.stringify(cards.map((c) => c.id).filter((i) => REQUIRED.includes(i))) === JSON.stringify(REQUIRED), 'catalogue order is the required storyline order');
  ok(await b.eval(`document.querySelectorAll('.tip').length >= 3`), 'glossary hints present');
  await shot('01_landing_cluster_down.png', true);

  await click('#b-up');
  ok(await b.waitFor(`document.querySelectorAll('#health .pill.ok').length === 3`, 120000), 'start from the page: all 3 nodes Active');
  await sleep(1500); await shot('03_cluster_started.png');

  await click('#b-tour'); ok(await b.waitFor(`document.getElementById('tour').open`, 5000), 'tour: opens'); await shot('04_tour_step1.png');
  await click('#t-next'); await click('#t-run');
  ok(await b.waitFor(`document.querySelector('#tour-result .verdict')`, 90000), 'tour step 2: runs the real scenario and explains the result');
  await shot('05_tour_step2_result.png');
  await click('#t-skip'); ok(await b.eval(`!document.getElementById('tour').open`), 'tour: Skip closes it');

  // whole storyline N times: identical verdicts
  const storyVerdicts = [];
  for (let k = 1; k <= STORIES; k++) {
    await b.eval(`window.scrollTo(0,0)`); await b.waitFor(`!document.getElementById('b-all').disabled`, 600000);
    const jobsBefore = (await state()).jobs.length;
    await click('[data-run="everything"]');
    const t0 = Date.now(); let s;
    while (Date.now() - t0 < 1500000) { await sleep(4000); s = await state(); if (!s.queue.running && !s.job && s.jobs.length > jobsBefore) break; }
    const run = s.jobs.slice(jobsBefore); const map = Object.fromEntries(run.map((j) => [j.scenario, j.verdict]));
    storyVerdicts.push(map);
    ok(run.length >= 25 && run.every((j) => j.verdict === 'passed' || j.verdict === 'shows'), `storyline run ${k}: every scenario ends Passed/Shows`, `${run.length} scenarios; not ok: ${run.filter((j) => !(j.verdict === 'passed' || j.verdict === 'shows')).map((j) => j.scenario + ':' + j.verdict).join(', ') || 'none'}`);
    await b.eval(`document.getElementById('session').scrollIntoView()`); await shot(`06_session_summary_run${k}.png`);
  }
  if (storyVerdicts.length > 1) ok(storyVerdicts.every((m) => JSON.stringify(m) === JSON.stringify(storyVerdicts[0])), 'storyline: identical verdicts on every run');

  // every scenario individually, random seeded order, REPS times
  const ids = REQUIRED.filter((i) => !['tour', 'everything', 'multiuser'].includes(i) && !SKIP.includes(i));
  const order = []; for (let r = 0; r < REPS; r++) { const o = ids.slice(); for (let i = o.length - 1; i > 0; i--) { const j = Math.floor(rnd() * (i + 1)); [o[i], o[j]] = [o[j], o[i]]; } order.push(...o); }
  console.log('[hub_check] random order:', order.join(','));
  const tally = {};
  for (const id of order) {
    const t0 = Date.now(); const v = await runCard(id); await sleep(400);
    const good = v && (v.verdict === 'passed' || v.verdict === 'shows');
    (tally[id] ||= { ok: 0, n: 0 }); tally[id].n += 1; if (good) tally[id].ok += 1;
    ok(good, `${id}: verdict`, v ? `${v.verdict} ${v.checks.pass}/${v.checks.fail} checks, ${Math.round((Date.now() - t0) / 1000)} s` : 'no verdict');
    ok(v && (v.numbers || []).length > 0, `${id}: result panel has numbers`);
    await shot(`11_${id}_result.png`);
  }
  console.log('[hub_check] per-scenario pass counts:', JSON.stringify(tally));

  // multiuser: windows cannot open headless; the card must still give a verdict or a plain message
  const mu = await runCard('multiuser');
  ok(!!mu, 'multiuser: finishes with a verdict (plain message if the Control Center or pop-ups are unavailable)', mu && mu.verdict);

  // health at the end
  await b.eval(`window.scrollTo(0,0)`); await sleep(2500);
  const fin = await state();
  ok(fin.cluster && fin.cluster.nodes.every((n) => n.running) && fin.cluster.nodes[0].view && Object.values(fin.cluster.nodes[0].view).every((v) => v === 'self' || /^Active/.test(v)), 'end: cluster healthy (3 gateways running and Active)', JSON.stringify(fin.cluster && fin.cluster.nodes[0].view));
  await b.eval(`document.getElementById('session').scrollIntoView()`); await shot('07_session_summary_end.png');
  ok((await txt('#sess-text')).includes('run(s)'), 'session summary strip shows runs and passed counts', await txt('#sess-text'));

  // failure state
  launcher(['stop-node', '--node', '3']);
  ok(await b.waitFor(`document.querySelector('#banner').innerText.includes('Node 3 is stopped')`, 10000), 'failure: node stopped -> banner names it and offers Restart');
  await shot('20_failure_node_stopped.png');
  const heal = await runCard('gallery');
  ok(heal && heal.verdict === 'passed', 'back-to-back safety: running a scenario with node 3 stopped restarts it automatically and passes', heal && heal.verdict);

  await b.viewport(390, 844, true, 2); await b.goto(`http://127.0.0.1:${PORT}/`, 2500);
  const overflow = await b.eval(`document.documentElement.scrollWidth - document.documentElement.clientWidth`);
  ok(overflow <= 1, 'phone width: no horizontal scroll', `overflow ${overflow}px`); await shot('30_mobile_landing.png');
  ok(!(await b.eval(`document.body.innerText`)).match(/eyJ[A-Za-z0-9_-]{20,}/), 'no token string anywhere on the page');
  b.saveLogs('hub');
  const bad = b.consoleLog.filter((c) => c.kind === 'exception' || /^(console\.error|log\.error|log\.warning|console\.warning)/.test(c.kind));
  ok(bad.length === 0, 'no browser console errors or warnings', JSON.stringify(bad.slice(0, 2)));
} catch (e) { ok(false, 'hub_check crashed', e && e.stack || e); }
finally {
  await b.close(); try { srv.kill(); } catch { /* ignore */ }
  launcher(['down']);
  console.log(`\n[hub_check] ${pass} PASS, ${fail} FAIL`);
  process.exit(fail ? 1 : 0);
}
