// D-7 recheck of items the first a11y pass could not judge because it navigated by hash inside one document
// (which keeps the sequential-focus starting point and the network log): fresh-load Tab order / skip link,
// Enter on a button with a proper text payload, and load observations from a cold navigation.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const ev = args.evidence, ui = args.ui || 'http://localhost:3000';
const R = new Results(join(ev, 'recheck.results.json'));
const b = await new Browser({ evidenceDir: ev, label: 'recheck' }).launch();
const fresh = async (hash) => { await b.goto('about:blank', 300); await b.goto(`${ui}/${hash}`, 4500); };
try {
  await b.viewport(1280, 800, false);
  for (const hash of ['#/', '#/lab', '#/lab/run']) {
    await fresh(hash);
    const trail = []; const activeAtLoad = await b.eval("document.activeElement ? document.activeElement.tagName + (document.activeElement.id ? '#' + document.activeElement.id : '') : null");
    for (let i = 0; i < 4; i++) { await b.tab(); trail.push(await b.eval(`(document.activeElement.innerText||document.activeElement.getAttribute('aria-label')||document.activeElement.tagName).trim().slice(0,40)`)); if (i === 0) await b.shot(`recheck_firsttab_${hash.replace(/\W+/g, '_')}.png`, 'shots'); }
    R.add(`fresh_tab_order.${hash}`, /skip/i.test(trail[0]) ? 'PASS' : 'FAIL', { activeElementAtLoad: activeAtLoad, first4: trail, note: 'FAIL here = first Tab does not reach the skip link; compare activeElementAtLoad (focus is moved to the page heading on load)' });
    if (/skip/i.test(trail[0])) {
      await b.key('Enter', { code: 'Enter', vk: 13 }); await sleep(300);
      R.add(`skip_link_activation.${hash}`, 'OBSERVED', await b.eval(`({ active: document.activeElement.tagName + '#' + document.activeElement.id, scrollY, hash: location.hash })`));
    }
  }
  // Enter / Space on a real button with correct key payloads
  await fresh('#/lab/run');
  await b.eval(`window.__c = 0; document.addEventListener('click', () => window.__c++, true); [...document.querySelectorAll('button')].find(x => /^Theme/i.test(x.innerText)).focus()`);
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); await sleep(300);
  const c1 = await b.eval('window.__c');
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: ' ', code: 'Space', windowsVirtualKeyCode: 32, text: ' ' }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 }); await sleep(300);
  R.add('keyboard.enter_space_on_button_recheck', c1 > 0 && (await b.eval('window.__c')) > c1 ? 'PASS' : 'FAIL', { afterEnter: c1, afterSpace: await b.eval('window.__c') });
  // Cold-load observations, 3 repetitions
  const loads = [];
  for (let i = 0; i < 3; i++) {
    b.reqById.clear(); await b.goto('about:blank', 300); await b.goto(`${ui}/#/lab`, 3500);
    const nav = await b.eval(`(() => { const n = performance.getEntriesByType('navigation')[0]; return { dcl: Math.round(n.domContentLoadedEventEnd), load: Math.round(n.loadEventEnd), fcp: Math.round((performance.getEntriesByName('first-contentful-paint')[0]||{}).startTime||-1) }; })()`);
    const rq = [...b.reqById.values()]; const sum = (f) => rq.filter(f).reduce((a, r) => a + (r.encodedBytes || 0), 0);
    loads.push({ ...nav, requests: rq.length, jsBytes: sum((r) => /javascript/.test(r.mime || '')), totalBytes: sum(() => true), failed: rq.filter((r) => r.failed && !r.canceled).length });
  }
  R.add('perf.cold_load_x3', 'OBSERVED', { loads, note: 'Vite dev server (unbundled, unminified), loopback, cache disabled, headless Chrome, 3 cold loads; observation only, not a production-bundle or network measurement' });
  // View-as-table toggle works on the Scenario B evidence page
  await fresh('#/lab/evidence/scenario_b.20261030T003932Z');
  const before = await b.eval(`document.querySelectorAll('details[open] table').length`);
  await b.eval(`[...document.querySelectorAll('details > summary')].filter(s => /table/i.test(s.innerText)).forEach(s => s.click())`); await sleep(400);
  R.add('charts.view_as_table', (await b.eval(`document.querySelectorAll('details[open] table').length`)) > before ? 'PASS' : 'FAIL', { tablesOpenBefore: before, after: await b.eval(`document.querySelectorAll('details[open] table').length`) }, await b.shot('view_as_table_open.png', 'shots', true));
  // Mobile: nav at 390 (unemulated-touch) top-of-page
  await b.viewport(390, 844, false); await fresh('#/lab');
  R.add('mobile.top_of_page', 'OBSERVED', await b.eval(`({ sw: document.documentElement.scrollWidth, headerRight: Math.round(document.querySelector('header').getBoundingClientRect().right), navRight: Math.round(document.querySelector('nav').getBoundingClientRect().right), navOverflowX: getComputedStyle(document.querySelector('nav')).overflowX })`), await b.shot('mobile_top_390.png', 'shots'));
} catch (e) { R.add('recheck.exception', 'FAIL', String(e.stack || e)); } finally {
  R.save(); b.saveLogs('recheck'); const gone = await b.close();
  writeFileSync(join(ev, 'recheck.browser.json'), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
  console.log(JSON.stringify({ chromePid: b.pid, chromeGone: gone }));
}
