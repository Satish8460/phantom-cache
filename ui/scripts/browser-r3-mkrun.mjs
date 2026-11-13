// D-7 round 3: create one small Scenario B run through the UI on the anonymous gateway (observe-only).
//   node ui/scripts/browser-r3-mkrun.mjs --evidence <dir>
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, ui = 'http://localhost:3000' } = args;
const R = new Results(join(evidence, 'r3_mkrun.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r3mk' }).launch();
try {
  await b.viewport(1280, 900, false);
  await b.goto(`${ui}/#/lab/run`, 1500);
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
  await sleep(300);
  for (const [l, v] of [['Repetitions per arm', 1], ['Warm-up', 1], ['Healthy', 1], ['Degraded', 2], ['Recovery', 1], ['Arrival rate', 100], ['Key catalogue', 100]]) await b.setField(l, v);
  await b.eval(`(() => { const c=[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.startsWith('AIMD-CAP')); const i=c&&c.querySelector('input'); if (i && i.checked) i.click(); })()`);
  await sleep(400);
  await b.clickText('Start run');
  const ok = await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  const id = ok ? await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`) : null;
  let term = false;
  for (let i = 0; i < 40 && !term; i++) { await sleep(2000); const t = await b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText||''`); term = /completed/i.test(t) && !/last known/i.test(t); }
  R.add('mkrun', term ? 'PASS' : 'FAIL', { id, completed: term });
  writeFileSync(join(evidence, 'r3_runids.json'), JSON.stringify({ sb: id }));
} catch (e) { R.add('exception', 'FAIL', String(e.stack || e)); } finally {
  R.save(); b.saveLogs('r3_mkrun'); const gone = await b.close();
  writeFileSync(join(evidence, 'r3_mkrun.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
}
