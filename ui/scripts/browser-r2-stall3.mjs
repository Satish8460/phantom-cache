// D-7 round 2: when Chrome's requests to the gateway stall, does the gateway still answer a direct request? Observation only.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';
import { Browser, sleep } from './cdp-lib.mjs';
const evidence = process.argv[2]; const ui = 'http://localhost:3000';
const b = await new Browser({ evidenceDir: evidence, label: 'r2stall3' }).launch();
const out = [];
const states = () => { const t = execSync('netstat -ano -p tcp').toString().split(String.fromCharCode(10)).filter((l) => /:8085\s/.test(l)); const c = {}; for (const l of t) { const st = l.trim().split(/\s+/)[3]; c[st] = (c[st] || 0) + 1; } return c; };
try {
  await b.viewport(1280, 800, false);
  for (let k = 0; k < 8; k++) {
    await b.goto('about:blank', 150); await b.goto(`${ui}/#/simulation`, 1500); await b.goto('about:blank', 150);
    b.reqById.clear(); await b.goto(`${ui}/#/lab`, 100);
    const t0 = Date.now(); let ok = false;
    while (Date.now() - t0 < 6000) { if ([...b.reqById.values()].some((r) => /lab\/capabilities/.test(r.url || '') && r.status)) { ok = true; break; } await sleep(100); }
    const row = { k, chromeGotCaps: ok, netstat: states() };
    if (!ok) {
      const t1 = Date.now();
      try { const r = await fetch('http://localhost:8085/api/v1/lab/capabilities', { headers: { Origin: 'http://localhost:3000' }, signal: AbortSignal.timeout(5000) }); row.directFetch = { status: r.status, ms: Date.now() - t1 }; } catch (e) { row.directFetch = String(e); }
      row.pending = [...b.reqById.values()].filter((r) => !r.finished && !r.failed).map((r) => r.url.replace(/.*:\d+/, '').slice(0, 40));
    }
    out.push(row); console.log(JSON.stringify(row));
  }
} finally { writeFileSync(join(evidence, 'r2_stall3.json'), JSON.stringify(out, null, 1)); console.log('chromeGone', await b.close()); }
