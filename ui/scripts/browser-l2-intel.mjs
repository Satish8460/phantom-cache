// Lane L2 live check of the Intelligence screen in headless Chrome, through the sign-in (secure gateway).
// Usage: node scripts/browser-l2-intel.mjs <ui-origin> <gateway-with-fixtures> <gateway-legacy> <gateway-not-installed> <tokens.json> <evidence-dir>
// <gateway-with-fixtures> is lane-local _work/mock_pie_proxy.mjs: the REAL gateway for authentication and everything else, contract-shaped
// FIXTURES for registry/signals/decisions. Screenshots taken against it are named *_FIXTURE and show fixture data, not a running PIE.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { Browser, sleep } from './cdp-lib.mjs';

const [ui, gwFix, gwLegacy, gwNotInstalled, tokensFile, evidence] = process.argv.slice(2);
const T = JSON.parse(readFileSync(tokensFile, 'utf8'));
const out = [];
const log = (id, ok, detail) => { out.push({ id, ok, detail }); console.log(ok ? 'PASS' : 'FAIL', id, detail || ''); };
const b = await new Browser({ evidenceDir: evidence, label: 'l2intel' }).launch();
const text = () => b.eval('document.body.innerText');
const useToken = async (tok) => {
  await b.eval(`window.__t = ${JSON.stringify(tok)}`);
  await b.eval(`(() => { const i = document.querySelector('[data-auth-banner] input[type=password], [data-identity-popover] input[type=password]'); if (!i) return false; const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, window.__t); i.dispatchEvent(new Event('input', { bubbles: true })); window.__t = null; return true; })()`);
  await sleep(100);
  await b.eval(`document.querySelector('[data-token-form] button[type=submit]').click()`);
};
const open = async (gw, hash) => {
  await b.goto(`${ui}/?gateway=${encodeURIComponent(gw)}#/${hash}`, 800);
  await b.send('Page.reload');
  await sleep(800);
  await b.waitFor(`document.querySelector('[data-auth-key="requires_token"]')`, 15000);
  await useToken(T.tenantA);
  await b.waitFor(`document.querySelector('[data-conn-kind="live"]') && !document.querySelector('[data-auth-banner]')`, 20000);
};
try {
  await b.viewport(1280, 1000);

  // A. the real gateway build in the audit folder: older intelligence status only
  await open(gwLegacy, 'intelligence');
  await b.waitFor(`document.querySelector('[data-intel-mode]')`, 20000);
  const a = await text();
  log('real gateway: older status shown as such, mode Off, nothing invented', /only the older intelligence status/.test(a) && /data-intel-mode/.test(await b.eval('document.body.innerHTML')) && /Current mode:\s*Off/.test(a), '');
  log('real gateway: nav keeps Intelligence under "Not available yet"', await b.eval(`!!document.querySelector('.app-nav__fold [data-screen="intelligence"]')`), '');
  log('real gateway: no registry, no decisions on the page', !/Policy registry|Recent decisions/.test(a), '');
  await b.shot('20_intel_legacy_REAL_gateway.png');

  // B. fixtures behind the real gateway's authentication
  await open(gwFix, 'intelligence');
  await b.waitFor(`document.querySelector('[data-intel-registry]') && document.querySelector('[data-intel-decision]')`, 25000);
  await sleep(600);
  const nav = await b.eval(`JSON.stringify([...document.querySelectorAll('.app-nav [data-screen]')].map(a => a.dataset.screen))`);
  log('fixture gateway: nav moved Intelligence after Cluster, no unavailable group', nav === JSON.stringify(['overview', 'traffic', 'cache', 'freshness', 'origin', 'cluster', 'intelligence', 'lab', 'simulation']) && !(await b.eval(`!!document.querySelector('.app-nav__fold')`)), nav);
  const t = await text();
  for (const [id, re] of [['mode statement', /Only three modes exist/], ['no benefit claim', /NOT_SUPPORTED/], ['registry', /POL-EVICT-1/], ['unavailable reason', /Not enough samples yet/], ['decision explanation', /shadow S3-FIFO cache/], ['not applied', /Not applied: nothing was changed/], ['drops counter', /Events dropped \(queue full\)/], ['observed unavailable', /Observed effect after the fact/]]) log(`fixture gateway: ${id}`, re.test(t), '');
  const unbadged = await b.eval(`[...document.querySelectorAll('td')].filter(td => td.querySelector('[data-intel-value]') && !td.querySelector('.prov')).length`);
  log('fixture gateway: every value cell has a provenance badge', unbadged === 0, `${unbadged} without`);
  await b.shot('21_intel_ready_FIXTURE_top.png');
  await b.shot('22_intel_ready_FIXTURE_full.png', 'shots', true);
  await b.eval(`document.querySelector('[data-intel-decision="19"] summary').scrollIntoView({block:'start'})`);
  await sleep(300);
  await b.shot('23_intel_decision_FIXTURE.png');
  const chip = await b.eval(`document.querySelector('[data-identity-chip]')?.innerText`);
  log('fixture gateway: header shows the token identity on this screen', /Signed in as tenant-a/.test(chip || ''), chip);

  // C. process without PIE v2
  await open(gwNotInstalled, 'intelligence');
  await b.waitFor(`/not installed in this gateway process/.test(document.body.innerText)`, 20000);
  log('not installed: stated, nothing shown', !/Policy registry/.test(await text()), '');
  await b.shot('24_intel_not_installed_FIXTURE.png');

  // D. step 3: overview and cluster with the identity chip and the "open as another user" hint
  await open(gwFix, '');
  await b.eval(`document.querySelector('[data-identity-chip]').click()`);
  await sleep(400);
  await b.eval(`document.querySelector('[data-open-as-another] summary').click()`);
  await sleep(300);
  await b.shot('25_overview_identity_hint.png');
  await b.eval(`location.hash = '#/cluster'`);
  await sleep(800);
  await b.shot('26_cluster_identity.png');

  // E. network: requests that carried the token went only to the gateways; nothing in a URL
  const reqs = [...b.reqById.values()];
  log('Authorization only to the gateways used', reqs.filter((r) => r.hasAuth).every((r) => [gwFix, gwLegacy, gwNotInstalled].some((g) => r.url.startsWith(g))), '');
  log('no token in any URL', reqs.every((r) => !/eyJ/.test(r.url)), '');
  const polls = reqs.filter((r) => /intelligence\/decisions/.test(r.url)).length;
  log('decision polling is bounded (2 s while the screen is open)', polls < 40, `${polls} decision requests in the run`);
  log('console: no exceptions, no token', b.consoleLog.filter((c) => /exception/.test(c.kind)).length === 0 && !JSON.stringify(b.consoleLog).includes('eyJ'), '');
} finally {
  b.saveLogs('l2intel');
  mkdirSync(evidence, { recursive: true });
  writeFileSync(`${evidence}/l2intel.results.json`, JSON.stringify(out, null, 1));
  console.log('chrome closed:', await b.close());
}
