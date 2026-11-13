// D-7 round 3: N-2 re-test. Repeated cold loads of Lab pages in ONE Chrome profile (no relaunch), with netstat counts
// of ESTABLISHED sockets whose foreign address is port 8085, plus Lab -> away -> Lab navigation connection counts.
//   node ui/scripts/browser-r3-stall.mjs --evidence <dir> --runid <completed sb run id>
import { execSync } from 'node:child_process';
import { writeFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, runid } = args; const ui = 'http://localhost:3000';
const R = new Results(join(evidence, 'r3_stall.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r3stall' }).launch();
appendFileSync(join(evidence, 'logs', 'chrome_pids.txt'), `r3_stall ${b.pid}\n`);
const conns = () => {
  const lines = execSync('netstat -ano -p tcp').toString().split(/\r?\n/).filter((l) => /ESTABLISHED/.test(l));
  const toGw = lines.filter((l) => /\s\S+:8085\s+ESTABLISHED/.test(l));       // client side: foreign address is :8085
  const fromGw = lines.filter((l) => /^\s*TCP\s+\S+:8085\s/.test(l));        // gateway side: local address is :8085
  return { clientSide: toGw.length, gatewaySide: fromGw.length };
};
const ready = (ms) => b.waitFor(`/Lab API enabled|Lab API off|Not reachable/.test(document.body.innerText)`, ms, 200);
const out = [];
try {
  await b.viewport(1280, 800, false);
  R.add('conns.before', 'OBSERVED', conns());
  const targets = ['#/lab', '#/lab/evidence', '#/lab/evidence/m6_sealed_2026-10-29T234845Z', '#/lab/evidence/scenario_b.20261030T003932Z', '#/lab/run', '#/lab/runs', `#/lab/runs/${runid}`];
  // A: 3 passes over every Lab route, about:blank hop before each cold load (>= 21 cold loads in one profile)
  let stalls = 0; let n = 0;
  for (let pass = 0; pass < 3; pass++) for (const r of targets) {
    await b.goto('about:blank', 150);
    b.reqById.clear();
    const t0 = Date.now(); await b.goto(`${ui}/${r}`, 100);
    const ok = await ready(12000); const dt = Date.now() - t0; n++;
    if (!ok) stalls++;
    const pending = [...b.reqById.values()].filter((x) => !x.finished && !x.failed).map((x) => x.url.replace(/.*:80\d\d/, '').slice(0, 50));
    await sleep(1200);
    const row = { n, route: r, ok: !!ok, ms: dt, pending, conns: conns() };
    out.push(row); console.log(JSON.stringify(row).slice(0, 240));
    R.add(`A.cold.${n}`, ok ? 'PASS' : 'FAIL', row);
  }
  R.add('A.summary', stalls ? 'FAIL' : 'PASS', { coldLoads: n, stalls });
  // B: hash navigation Lab -> away -> Lab, connection counts after each hop
  const hops = [];
  await b.goto(`${ui}/#/lab`, 1500); await ready(8000);
  for (let i = 0; i < 6; i++) {
    for (const h of ['#/', '#/lab', '#/lab/evidence', '#/traffic', `#/lab/runs/${runid}`, '#/lab/runs']) {
      await b.eval(`location.hash='${h}'`); await sleep(1200);
      hops.push({ i, h, conns: conns() });
    }
  }
  R.add('B.hash_nav', 'OBSERVED', { first: hops[0], last: hops.at(-1), max: Math.max(...hops.map((x) => x.conns.clientSide)), min: Math.min(...hops.map((x) => x.conns.clientSide)), all: hops.map((x) => x.conns.clientSide) });
  // C: a running run: cold-load its page while it runs (live stream), then after it ends
  await b.goto(`${ui}/#/lab/run`, 1500);
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
  for (const [l, v] of [['Repetitions per arm', 1], ['Warm-up', 2], ['Healthy', 4], ['Degraded', 4], ['Recovery', 2], ['Arrival rate', 100], ['Key catalogue', 100]]) await b.setField(l, v);
  await b.eval(`(() => { const c=[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.startsWith('AIMD-CAP')); const i=c&&c.querySelector('input'); if (i && i.checked) i.click(); })()`);
  await sleep(300); await b.clickText('Start run');
  await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  const rid = await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`);
  const live = [];
  for (let i = 0; i < 6; i++) {
    await b.goto('about:blank', 150); await b.goto(`${ui}/#/lab/runs/${rid}`, 100);
    const ok = await ready(12000); await sleep(800);
    live.push({ i, ok: !!ok, status: await b.eval(`(document.querySelector('[data-stream-status]')||{}).getAttribute?.('data-stream-status')||null`), conns: conns() });
  }
  R.add('C.running_run_cold_loads', live.every((x) => x.ok) ? 'PASS' : 'FAIL', { rid, live });
  // wait for the run to end, leave the page, count
  await b.waitFor(`/completed|failed|cancelled|interrupted/i.test((document.querySelector('.lab-detail-head')||{}).innerText||'')`, 60000, 1000);
  await sleep(1000);
  R.add('C.after_run_ended', 'OBSERVED', conns());
  await b.goto('about:blank', 1500);
  R.add('C.after_about_blank', 'OBSERVED', conns());
  // D: one more cold load after everything (does the 7th+ load still work?)
  const t0 = Date.now(); await b.goto(`${ui}/#/lab`, 100); const ok = await ready(12000);
  R.add('D.final_cold_load', ok ? 'PASS' : 'FAIL', { ms: Date.now() - t0, conns: conns() });
} catch (e) { R.add('exception', 'FAIL', String((e && e.stack) || e)); } finally {
  R.save(); writeFileSync(join(evidence, 'r3_stall.rows.json'), JSON.stringify(out, null, 1));
  const before = conns(); const gone = await b.close();
  const after = conns();
  writeFileSync(join(evidence, 'r3_stall.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, connsBeforeClose: before, connsAfterClose: after }));
  console.log(JSON.stringify({ before, after, gone }));
}
