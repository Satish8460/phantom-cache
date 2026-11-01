// D-7 round 3: N-5 (operator-only token hint + run-page 403 explanation), observer/admin non-regression. Observe-only.
//   node ui/scripts/browser-r3-jwt.mjs --evidence <dir> --tokens <tokens.json>
import { readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, tokens } = args; const ui = 'http://localhost:3000'; const gw = 'http://127.0.0.1:18086';
const gwUi = 'http://localhost:18086';
const R = new Results(join(evidence, 'r3_jwt.results.json'));
const b = await new Browser({ evidenceDir: evidence, label: 'r3jwt' }).launch();
appendFileSync(join(evidence, 'logs', 'chrome_pids.txt'), `r3_jwt ${b.pid}\n`);
const T = JSON.parse(readFileSync(tokens, 'utf8'));
const idle = async () => { for (let i = 0; i < 60; i++) { try { const r = await fetch(`${gw}/api/v1/lab/capabilities`, { headers: { Authorization: `Bearer ${T.admin}` } }); const j = await r.json(); if (!j.active_run) return true; } catch { /* retry */ } await sleep(2000); } return false; };
const grab = (re) => b.eval(`(() => { const e=[...document.querySelectorAll('[role=status],[role=alert]')].filter(x => ${re}.test(x.innerText)); return e.map(x => x.getAttribute('role') + ': ' + x.innerText.replace(/\\s+/g,' ').slice(0, 400)); })()`);
try {
  for (const vp of [[1280, 800], [390, 844]]) {
    await b.viewport(vp[0], vp[1], false);
    for (const who of ['observer', 'operator', 'admin']) {
      if (!(await idle())) R.add(`idle.${who}`, 'FAIL', 'gateway never idle');
      await b.goto(`${ui}/?gateway=${encodeURIComponent(gw)}#/lab`, 2500);
      await b.waitFor(`document.querySelector('.lab-token input')`, 10000);
      await b.eval(`(() => { const i=document.querySelector('.lab-token input'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(i, ${JSON.stringify(T[who])}); i.dispatchEvent(new Event('input',{bubbles:true})); })()`);
      await sleep(200); await b.clickText('Use token'); await sleep(2500);
      await b.eval(`location.hash='#/lab/run'`); await sleep(1800);
      const out = { who, vp: vp.join('x'), formThere: await b.eval(`!!document.querySelector('form[aria-label="Run specification"]')`) };
      if (out.formThere) {
        await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
        for (const [l, v] of [['Repetitions per arm', 1], ['Warm-up', 1], ['Healthy', 1], ['Degraded', 1], ['Recovery', 1], ['Arrival rate', 50], ['Key catalogue', 100]]) await b.setField(l, v);
        await b.eval(`(() => { const c=[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.startsWith('AIMD-CAP')); const i=c&&c.querySelector('input'); if (i && i.checked) i.click(); })()`);
        await sleep(500);
        out.hint = await grab('/./');
        out.submitDisabled = await b.eval(`document.querySelector('form[aria-label="Run specification"] button[type=submit]').disabled`);
        await b.shot(`r3_jwt_${vp[0]}_${who}_pre_submit.png`, 'shots/r3jwt', true);
        if (!out.submitDisabled) { await b.clickText('Start run'); await sleep(3500); }
        out.hash = await b.eval('location.hash');
        out.statusAndAlerts = await grab('/./');
        out.h1 = await b.eval(`(document.querySelector('h1')||{}).innerText`);
        out.bodyExcerpt = (await b.eval('document.body.innerText')).replace(/\s+/g, ' ').slice(0, 1200);
        out.buttons = await b.eval(`[...document.querySelectorAll('main button, main a')].map(e => e.innerText.trim()).filter(Boolean).slice(0, 20)`);
        out.activeAfter = await b.eval(`document.activeElement.tagName + ':' + (document.activeElement.innerText||'').slice(0, 30)`);
        await b.shot(`r3_jwt_${vp[0]}_${who}_post_submit.png`, 'shots/r3jwt', true);
        // scroll width check on the 403 page
        out.scrollWidth = await b.eval('document.documentElement.scrollWidth');
        // try the explanation's own link/retry if present (observe only)
        out.retryExists = await b.eval(`!![...document.querySelectorAll('button')].find(x => /retry/i.test(x.innerText))`);
      }
      R.add(`N5.${vp[0]}.${who}`, 'OBSERVED', out);
    }
  }
  await idle();
} catch (e) { R.add('exception', 'FAIL', String(e.stack || e)); } finally {
  R.save(); b.saveLogs('r3_jwt'); const gone = await b.close();
  writeFileSync(join(evidence, 'r3_jwt.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone }));
}
