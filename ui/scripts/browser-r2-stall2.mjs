// D-7 round 2: does the stall of cold reloads depend on the gateway (anonymous guard vs JWT) ? Observation only.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, sleep } from './cdp-lib.mjs';
const evidence = process.argv[2]; const ui = 'http://localhost:3000';
const b = await new Browser({ evidenceDir: evidence, label: 'r2stall2' }).launch();
const out = [];
try {
  await b.viewport(1280, 800, false);
  for (const [name, q] of [['anon_8085', ''], ['jwt_18086', '?gateway=http%3A%2F%2Flocalhost%3A18086'], ['anon_8085_again', '']]) {
    for (let k = 0; k < 5; k++) {
      await b.goto('about:blank', 150); b.reqById.clear();
      await b.goto(`${ui}/${q}#/lab`, 100);
      const t0 = Date.now(); let st = null;
      while (Date.now() - t0 < 8000) { st = [...b.reqById.values()].find((r) => /lab\/capabilities/.test(r.url || '') && r.status); if (st) break; await sleep(100); }
      const row = { name, k, capStatus: st ? st.status : null, ms: st ? Date.now() - t0 : null, requests: [...b.reqById.values()].filter((r) => /:(8085|18086)/.test(r.url || '')).map((r) => `${r.url.replace(/.*:\d+/, '').slice(0, 36)} ${r.status || (r.failed ? 'FAILED ' + r.failed : 'PENDING')}`) };
      out.push(row); console.log(JSON.stringify(row).slice(0, 260));
    }
  }
} finally { writeFileSync(join(evidence, 'r2_stall2.json'), JSON.stringify(out, null, 1)); console.log('chromeGone', await b.close()); }
