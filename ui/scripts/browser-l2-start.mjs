// Lane L2: Start here page, welcome banner, help lines and Hub links in headless Chrome.
// Usage: node scripts/browser-l2-start.mjs <ui-origin> <secure-gateway> <offline-gateway> <evidence-dir>
import { writeFileSync, mkdirSync } from 'node:fs';
import { Browser, sleep } from './cdp-lib.mjs';

const [ui, secure, offline, evidence] = process.argv.slice(2);
const out = [];
const log = (id, ok, d) => { out.push({ id, ok, d }); console.log(ok ? 'PASS' : 'FAIL', id, d || ''); };
const b = await new Browser({ evidenceDir: evidence, label: 'l2start' }).launch();
const url = (gw, hash, extra = '') => `${ui}/?gateway=${encodeURIComponent(gw)}${extra}#/${hash}`;
const text = () => b.eval('document.body.innerText');
try {
  await b.viewport(1280, 1000);
  // fresh tab, secure gateway, no token: welcome banner on the overview, then Start here
  await b.goto(url(secure, ''), 800);
  await b.send('Page.reload');
  await sleep(800);
  await b.waitFor(`document.querySelector('[data-welcome]')`, 15000);
  log('welcome banner shown on first visit', true);
  await b.shot('30_welcome_secure_no_token.png');
  await b.eval(`document.querySelector('[data-welcome-start]').click()`);
  await b.waitFor(`document.querySelector('[data-start-needs-token]')`, 15000);
  const t = await text();
  log('start here: needs-token state in plain words', /Sign in to see this gateway/.test(t) && /Take the 5-minute guided tour/.test(t), '');
  log('start here: welcome gone after clicking Start here', !(await b.eval(`!!document.querySelector('[data-welcome]')`)), '');
  await b.shot('31_start_here_secure_no_token.png');
  await b.shot('32_start_here_secure_no_token_full.png', 'shots', true);
  const links = await b.eval(`JSON.stringify([...document.querySelectorAll('[data-scenario] a[href]')].map(a => a.href))`);
  log('start here: 30 scenario deep links to the Hub', JSON.parse(links).length === 30 && JSON.parse(links).every((h) => /^http:\/\/127\.0\.0\.1:3201\/#\/scenario\/[a-z]+$/.test(h)), '');
  await b.eval(`location.reload()`);
  await sleep(1500);
  log('welcome stays dismissed after reload in the same tab', !(await b.eval(`!!document.querySelector('[data-welcome]')`)), '');
  // offline gateway
  await b.goto(url(offline, 'start'), 800);
  await b.send('Page.reload');
  await sleep(2500);
  await b.waitFor(`document.querySelector('[data-start-offline]')`, 20000);
  log('offline: friendly state with the Hub pointer', /No gateway answers at this address/.test(await text()), '');
  await b.shot('33_start_here_offline.png');
  // custom hub address and refused one
  await b.goto(url(offline, 'start', `&hub=${encodeURIComponent('http://evil.example')}`), 800);
  await b.send('Page.reload');
  await b.waitFor(`/Ignored \?hub=/.test(document.body.innerText)`, 15000);
  log('refused ?hub= shows the note and keeps the default Hub', /Ignored \?hub=/.test(await text()), '');
  // phone width
  await b.viewport(390, 800, true);
  await b.goto(url(secure, 'start'), 800);
  await b.send('Page.reload');
  await sleep(1500);
  const overflow = await b.eval(`document.documentElement.scrollWidth > window.innerWidth + 1`);
  log('phone width: no horizontal overflow', !overflow, '');
  await b.shot('34_start_here_phone.png');
  log('console: no exceptions', b.consoleLog.filter((c) => /exception/.test(c.kind)).length === 0, '');
} finally {
  mkdirSync(evidence, { recursive: true });
  writeFileSync(`${evidence}/l2start.results.json`, JSON.stringify(out, null, 1));
  console.log('chrome closed:', await b.close());
}
