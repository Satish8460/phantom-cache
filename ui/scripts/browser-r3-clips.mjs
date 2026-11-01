// D-7 round 3: viewport-sized screenshots of specific regions (full-page shots are too tall to read). Observe-only.
import { appendFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, sleep } from './cdp-lib.mjs';
const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { evidence, ui = 'http://localhost:3000' } = args;
const b = await new Browser({ evidenceDir: evidence, label: 'r3clip' }).launch();
appendFileSync(join(evidence, 'logs', 'chrome_pids.txt'), `r3_clips ${b.pid}\n`);
const SB = '#/lab/evidence/scenario_b.20261030T003932Z', M6 = '#/lab/evidence/m6_sealed_2026-10-29T234845Z';
const clip = async (name, vw, vh, route, scrollJs, theme = 'light') => {
  await b.media([{ name: 'prefers-color-scheme', value: theme }]); await b.viewport(vw, vh, false);
  await b.goto('about:blank', 150); await b.goto(`${ui}/${route}`, 2500);
  await b.waitFor('!/Checking|Loading this/.test(document.body.innerText)', 15000); await sleep(600);
  if (scrollJs) { await b.eval(scrollJs); await sleep(400); }
  console.log(name, JSON.stringify(await b.eval('({sw: document.documentElement.scrollWidth, y: Math.round(scrollY)})')));
  await b.shot(name, 'shots/r3clips', false);
};
try {
  const toSel = (sel, n = 0) => `(() => { const e = document.querySelectorAll(${JSON.stringify(sel)})[${n}]; if (e) window.scrollTo(0, e.getBoundingClientRect().top + scrollY - 70); })()`;
  await clip('sb_390_table.png', 390, 844, SB, toSel('.table-wrap', 0));
  await clip('sb_390_table2.png', 390, 844, SB, toSel('.table-wrap', 1));
  await clip('sb_320_table.png', 320, 700, SB, toSel('.table-wrap', 0));
  await clip('sb_320_table2.png', 320, 700, SB, toSel('.table-wrap', 1));
  await clip('m6_caption_1280_light.png', 1280, 900, M6, toSel('figcaption', 0));
  await clip('m6_caption_1280_dark.png', 1280, 900, M6, toSel('figcaption', 0), 'dark');
  await clip('start_form_390_top.png', 390, 844, '#/lab/run', toSel('form[aria-label="Run specification"]', 0));
  await clip('start_form_390_checks.png', 390, 844, '#/lab/run', toSel('fieldset', 1));
  await clip('start_form_1280.png', 1280, 900, '#/lab/run', toSel('form[aria-label="Run specification"]', 0));
  await clip('banner_320.png', 320, 700, '#/lab', null);
} finally { await b.close(); }
