// Rules-engine accessibility check (dev tool): injects axe-core into a real Chrome page over CDP and records results.
//   node scripts/check-axe.mjs [--ui http://localhost:3000] [--gateway http://localhost:8085]
//        [--routes overview,lab_runs] [--viewports 320,390,1280] [--themes light,dark] [--evidence <dir>] [--gate]
// Ownership: uses cdp-lib.mjs (own throwaway Chrome profile under D:/Temp, kills only its own process tree). Node built-ins
// plus the dev-only axe-core package; axe-core is never imported by anything under src/ and is not in the production bundle.
// Reading the output: `violations` are rules axe is sure about. `incomplete` means axe could not decide (for example text over
// a translucent surface, where the effective background is not computable): it is NEVER a pass; it needs a manual or
// worst-case check (see check-contrast.mjs). A clean run means "no violation found by these rules in these states", not
// conformance, and it does not replace a human assistive-technology pass. Without --gate the exit code is 0 (baseline mode).
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Browser, sleep } from './cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const args = Object.fromEntries(
  process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), x.startsWith('--gate') ? true : arr[i + 1]]] : a), []),
);
const UI = args.ui || 'http://localhost:3000';
const GW = args.gateway || 'http://localhost:8085';
const EV = args.evidence || join(HERE, '../../docs/design/remediation/axe_evidence');
const THEMES = (args.themes || 'light,dark').split(',');
const VIEWPORTS = (args.viewports || '320,390,640,820,1280').split(',').map(Number);
const ROUTES = [
  ['overview', '#/overview'], ['traffic', '#/traffic'], ['cache', '#/cache'], ['freshness', '#/freshness'], ['origin', '#/origin'],
  ['cluster', '#/cluster'], ['intelligence', '#/intelligence'], ['simulation', '#/simulation'],
  ['lab_home', '#/lab'], ['lab_evidence', '#/lab/evidence'], ['lab_run_form', '#/lab/run'], ['lab_runs', '#/lab/runs'],
];
const wanted = args.routes ? new Set(args.routes.split(',')) : null;
const routes = ROUTES.filter(([id]) => !wanted || wanted.has(id));
const TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

const axeSource = readFileSync(join(HERE, '../node_modules/axe-core/axe.min.js'), 'utf8');
const axeVersion = JSON.parse(readFileSync(join(HERE, '../node_modules/axe-core/package.json'), 'utf8')).version;
const slim = (r) => ({
  id: r.id, impact: r.impact, help: r.help, helpUrl: r.helpUrl, tags: r.tags.filter((t) => t.startsWith('wcag')),
  nodeCount: r.nodes.length,
  nodes: r.nodes.slice(0, 8).map((n) => ({ target: n.target, html: String(n.html).slice(0, 160), summary: String(n.failureSummary || '').slice(0, 240) })),
});
const RUN = `axe.run(document, { runOnly: { type: 'tag', values: ${JSON.stringify(TAGS)} }, resultTypes: ['violations', 'incomplete'] })
  .then((r) => ({ violations: r.violations, incomplete: r.incomplete, passes: r.passes.length, inapplicable: r.inapplicable.length }))`;

mkdirSync(EV, { recursive: true });
const rows = [];
let b = await new Browser({ evidenceDir: EV, label: 'axe' }).launch();
const meta = { tool: 'check-axe.mjs', axe: axeVersion, chrome: b.version, ui: UI, tags: TAGS, startedAt: new Date().toISOString() };
try {
  for (const w of VIEWPORTS) {
    await b.viewport(w, w >= 820 ? 900 : 844, false, 1);
    await b.goto(`${UI}/?gateway=${encodeURIComponent(GW)}`, 1800);
    await b.eval(axeSource);
    for (const theme of THEMES) {
      await b.eval(`document.documentElement.setAttribute('data-theme', ${JSON.stringify(theme)})`);
      for (const [id, hash] of routes) {
        await b.eval(`location.hash = ${JSON.stringify(hash)}`);
        await sleep(900);
        // Readiness: the app has rendered a page heading and has no loading spinner. A cell that never gets there is recorded
        // as `notRendered` and must not be read as a clean result (a blank page has almost nothing for axe to check).
        const ready = await b.waitFor(`document.querySelector('main h1, main h2, #root h1') && !document.querySelector('.spinner, [data-state=loading]')`, 15000, 200);
        const textLength = await b.eval(`(document.getElementById('root') || document.body).innerText.length`);
        const res = await b.eval(RUN);
        rows.push({
          route: id, theme, viewport: w, notRendered: !ready || textLength < 80, textLength, passes: res.passes, inapplicable: res.inapplicable,
          violations: res.violations.map(slim), incomplete: res.incomplete.map(slim),
        });
      }
    }
  }
} finally {
  meta.finishedAt = new Date().toISOString();
  meta.chromeGone = await b.close();
}
const count = (k) => rows.reduce((n, r) => n + r[k].length, 0);
const summary = {
  ...meta, cells: rows.length, notRenderedCells: rows.filter((r) => r.notRendered).length,
  violationRuleHits: count('violations'), incompleteRuleHits: count('incomplete'),
};
const byRule = {};
for (const r of rows) for (const v of r.violations) (byRule[v.id] ||= { impact: v.impact, cells: 0, nodes: 0 }).cells++, (byRule[v.id].nodes += v.nodeCount);
writeFileSync(join(EV, 'axe_results.json'), JSON.stringify({ summary, byRule, rows }, null, 1));
console.log(JSON.stringify({ ...summary, byRule }, null, 1));
if (args.gate && (summary.violationRuleHits > 0 || summary.notRenderedCells > 0)) process.exit(1);
