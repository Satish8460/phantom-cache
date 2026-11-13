// D-7 round 3: regression check of the cancel confirmation (Escape returns focus to "Cancel run"). Observe-only.
import { writeFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, ui = 'http://localhost:3000' } = args;
const R = new Results(join(evidence, 'r3_cancel.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r3cx' }).launch();
appendFileSync(join(evidence, 'logs', 'chrome_pids.txt'), `r3_cancel ${b.pid}\n`);
const act = () => b.eval(`(() => { const a = document.activeElement; return a.tagName + ':' + (a.innerText || a.getAttribute('aria-label') || '').trim().slice(0, 30); })()`);
try {
  await b.viewport(1280, 900, false);
  await b.goto(`${ui}/#/lab/run`, 1500);
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
  for (const [l, v] of [['Repetitions per arm', 1], ['Warm-up', 2], ['Healthy', 5], ['Degraded', 5], ['Recovery', 3], ['Arrival rate', 100], ['Key catalogue', 100]]) await b.setField(l, v);
  await b.eval(`(() => { const c=[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.startsWith('AIMD-CAP')); const i=c&&c.querySelector('input'); if (i && i.checked) i.click(); })()`);
  await sleep(300); await b.clickText('Start run');
  await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000); await sleep(1500);
  const o = {};
  await b.clickText('Cancel run'); await sleep(500);
  o.afterOpen = await act();
  o.dialogText = await b.eval(`[...document.querySelectorAll('[role=alertdialog],[role=dialog],[role=alert]')].map(e => e.innerText.replace(/\\s+/g,' ').slice(0, 200))`);
  await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(400);
  o.afterEscape = await act();
  o.stillRunning = await b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText`);
  await b.clickText('Cancel run'); await sleep(400); await b.clickText('Keep running'); await sleep(400);
  o.afterKeep = await act();
  await b.clickText('Cancel run'); await sleep(400);
  await b.eval(`[...document.querySelectorAll('button')].find(x => /^(yes|cancel the run|cancel run now|confirm)/i.test(x.innerText.trim()) )?.click()`);
  await b.waitFor(`/cancelled/i.test((document.querySelector('.lab-detail-head')||{}).innerText||'')`, 30000, 500);
  o.final = await b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText`);
  R.add('cancel.escape', /Keep running/i.test(o.afterOpen) && /Cancel run/i.test(o.afterEscape) && /Running/i.test(o.stillRunning) ? 'PASS' : 'FAIL', o);
} catch (e) { R.add('exception', 'FAIL', String(e.stack || e)); } finally {
  R.save(); b.saveLogs('r3_cancel'); const gone = await b.close();
  writeFileSync(join(evidence, 'r3_cancel.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone }));
}
