// D-7 round 2 follow-up: F-09 submit-anyway with an observer-only token (the journey's submit selector matched the wrong button).
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, tokens } = args; const ui = 'http://localhost:3000'; const gw = 'http://localhost:18086';
const R = new Results(join(evidence, 'r2_f09.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r2f09' }).launch();
const T = JSON.parse(readFileSync(tokens, 'utf8'));
try {
  await b.viewport(1280, 800, false);
  for (const who of ['observer', 'operator']) {
    await b.goto(`${ui}/?gateway=${encodeURIComponent(gw)}#/lab`, 2500);
    await b.eval(`(() => { const i=document.querySelector('.lab-token input'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(i, ${JSON.stringify(T[who])}); i.dispatchEvent(new Event('input',{bubbles:true})); })()`);
    await sleep(200); await b.clickText('Use token'); await sleep(2500);
    await b.eval(`location.hash='#/lab/run'`); await sleep(1800);
    const formThere = await b.eval(`!!document.querySelector('form[aria-label="Run specification"]')`);
    const out = { who, formThere };
    if (formThere) {
      await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
      for (const [l, v] of [['Repetitions per arm', 1], ['Warm-up', 1], ['Healthy', 1], ['Degraded', 1], ['Recovery', 1], ['Arrival rate', 50], ['Key catalogue', 100]]) await b.setField(l, v);
      await sleep(400);
      out.warning = await b.eval(`(() => { const e=[...document.querySelectorAll('[role=status]')].find(x => /cannot start runs/i.test(x.innerText)); return e ? e.innerText.replace(/\s+/g,' ') : null; })()`);
      out.submitDisabled = await b.eval(`document.querySelector('form[aria-label="Run specification"] button[type=submit]').disabled`);
      await b.shot(`f09_${who}_before_submit.png`, 'shots/journey', true);
      if (!out.submitDisabled) { await b.clickText('Start run'); await sleep(2500); }
      out.alert = await b.eval(`(() => { const e=document.querySelector('[role=alert]:not([data-field-error])'); return e ? e.innerText.replace(/\s+/g,' ') : null; })()`);
      out.hash = await b.eval('location.hash');
      await b.shot(`f09_${who}_after_submit.png`, 'shots/journey', false);
    }
    R.add(`F09.submit_anyway.${who}`, 'OBSERVED', out);
  }
} catch (e) { R.add('exception', 'FAIL', String(e.stack || e)); } finally {
  R.save(); b.saveLogs('r2_f09'); const gone = await b.close();
  writeFileSync(join(evidence, 'r2_f09.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone }));
}
