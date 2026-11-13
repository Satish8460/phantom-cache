// D-7 round 2: reproduce the intermittent "Checking" (capabilities never answers) state seen while sweeping routes.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, sleep } from './cdp-lib.mjs';
const evidence = process.argv[2]; const ui = 'http://localhost:3000';
const b = await new Browser({ evidenceDir: evidence, label: 'r2stall' }).launch();
const out = [];
const ready = (ms) => b.waitFor(`/Lab API enabled|Lab API off|Not reachable/.test(document.body.innerText)`, ms, 200);
try {
  await b.viewport(1280, 800, false);
  const seqs = [['#/simulation', '#/lab'], ['#/lab/runs', '#/lab/runs/20261030T045804Z-07140d71'], ['#/', '#/lab'], ['#/lab', '#/lab'], ['#/origin', '#/lab']];
  for (const useBlank of [true, false]) for (const [a, c] of seqs) for (let k = 0; k < 3; k++) {
    if (useBlank) await b.goto('about:blank', 150);
    await b.goto(`${ui}/${a}`, 1500);
    if (useBlank) await b.goto('about:blank', 150);
    b.reqById.clear();
    const t0 = Date.now(); await b.goto(`${ui}/${c}`, 100);
    const ok = await ready(12000); const dt = Date.now() - t0;
    const pending = [...b.reqById.values()].filter((r) => !r.finished && !r.failed).map((r) => r.url.replace(/.*:80\d\d/, '').slice(0, 50));
    out.push({ useBlank, a, c, k, ok: !!ok, ms: dt, pending, loadingText: ok ? null : await b.eval(`document.body.innerText.slice(0, 200).split(String.fromCharCode(10)).join(' | ')`) });
    console.log(JSON.stringify(out.at(-1)).slice(0, 220));
  }
} finally {
  writeFileSync(join(evidence, 'r2_stall.json'), JSON.stringify(out, null, 1));
  console.log('chromeGone', await b.close());
}
