// Lane L2 live check of the Control Center sign-in against a REAL secure gateway (PHANTOM_JWT_SECRET set).
// Usage: node scripts/browser-l2-auth.mjs <ui-origin> <gateway-origin> <tokens.json> <evidence-dir>
// The tokens file is read here; no token is printed, typed into a visible field, or written to the evidence (password field, shots only).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { Browser, sleep } from './cdp-lib.mjs';

const [ui, gw, tokensFile, evidence] = process.argv.slice(2);
const T = JSON.parse(readFileSync(tokensFile, 'utf8'));
const out = [];
const log = (id, ok, detail) => { out.push({ id, ok, detail }); console.log(ok ? 'PASS' : 'FAIL', id, detail || ''); };
const b = await new Browser({ evidenceDir: evidence, label: 'l2auth' }).launch();
try {
  await b.viewport(1280, 900);
  const url = `${ui}/?gateway=${encodeURIComponent(gw)}#/`;
  const text = () => b.eval('document.body.innerText');
  const useToken = async (tok, keep = false) => {
    await b.eval(`window.__t = ${JSON.stringify(tok)}`);
    await b.eval(`(() => { const i = document.querySelector('[data-auth-banner] input[type=password], [data-identity-popover] input[type=password]'); if (!i) return false; const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, window.__t); i.dispatchEvent(new Event('input', { bubbles: true })); window.__t = null; return true; })()`);
    if (keep) await b.eval(`document.querySelector('[data-keep-for-tab] input').click()`);
    await sleep(100);
    await b.eval(`document.querySelector('[data-token-form] button[type=submit]').click()`);
  };
  const forget = async () => {
    if (!(await b.eval(`!!document.querySelector('[data-identity-popover]')`))) { await b.eval(`document.querySelector('[data-identity-chip]').click()`); await sleep(300); }
    await b.eval(`[...document.querySelectorAll('[data-identity-popover] button, [data-auth-banner] button')].find(x => /Forget token/.test(x.innerText)).click()`);
  };

  // 1. no token on a secure gateway
  await b.goto(url, 800);
  const gotBanner = await b.waitFor(`document.querySelector('[data-auth-key="requires_token"]')`, 15000);
  log('no-token: banner "requires a token"', Boolean(gotBanner) && (await text()).includes('Sign in to see this gateway'), '');
  await b.shot('01_requires_token.png');
  const chip1 = await b.eval(`document.querySelector('[data-identity-chip]')?.innerText`);
  log('no-token: header chip', /Sign in/.test(chip1 || ''), chip1);
  const px = await b.eval(`JSON.stringify([...(performance.getEntriesByType('resource'))].map(e => e.name).filter(n => /eyJ|bearer|[?&]token=/i.test(n)))`);
  log('no token in any request URL', px === '[]', px);

  // 2. garbage input is refused locally, nothing sent or stored
  await useToken('this is not a token');
  await sleep(300);
  const prob = await b.eval(`document.querySelector('[data-token-problem]')?.innerText`);
  log('garbage input: local refusal, no echo', Boolean(prob) && !(await text()).includes('this is not a token'), prob);
  await b.shot('02_bad_input.png');

  // 3. expired token
  await useToken(T.expired);
  await b.waitFor(`document.querySelector('[data-auth-key="expired"]')`, 15000);
  const t3 = await text();
  log('expired token: gateway 401 ExpiredToken in words', /The token has expired/.test(t3) && /HTTP 401/.test(t3), '');
  await b.shot('03_expired.png');

  // 4. wrong audience
  await useToken(T.wrongAudience);
  await b.waitFor(`document.querySelector('[data-auth-key="wrong_audience"]')`, 15000);
  log('wrong audience: 401 InvalidAudience in words', /different audience/.test(await text()), '');
  await b.shot('04_wrong_audience.png');

  // 5. wrong signing key
  await useToken(T.wrongKey);
  await b.waitFor(`document.querySelector('[data-auth-key="rejected"]')`, 15000);
  log('wrong key: 401 InvalidToken, no detail claimed', /did not accept the token/.test(await text()), '');
  await b.shot('05_rejected.png');

  // 6. token without telemetry:read
  await useToken(T.noObserver);
  await b.waitFor(`document.querySelector('[data-auth-key="forbidden"]')`, 15000);
  log('no telemetry role: 403 in words', /lacks the role/.test(await text()) && /HTTP 403/.test(await text()), '');
  await b.shot('06_forbidden.png');

  // 7. valid tenant-a token: live data flows, chip shows identity
  const mark = b.consoleLog.length;
  await useToken(T.tenantA);
  const live = await b.waitFor(`document.querySelector('[data-conn-kind="live"]') && !document.querySelector('[data-auth-banner]')`, 20000);
  log('valid token: stream live, banner gone', Boolean(live), '');
  await sleep(1500);
  const chip7 = await b.eval(`document.querySelector('[data-identity-chip]')?.innerText`);
  log('valid token: chip names the tenant', /Signed in as tenant-a/.test(chip7 || ''), chip7);
  const noisy = b.consoleLog.slice(mark).filter((x) => /console.(error|warning)|log.(error|warning)|exception/.test(x.kind));
  log('happy path: no console errors or warnings after signing in', noisy.length === 0, JSON.stringify(noisy.slice(0, 2)));
  await b.shot('07_live_tenant_a.png');
  await b.eval(`document.querySelector('[data-identity-chip]').click()`);
  await sleep(500);
  const pop = await b.eval(`document.querySelector('[data-identity-popover]')?.innerText`);
  log('popover: identity + label + hint', /tenant-a/.test(pop) && /alice/.test(pop) && /not verified/.test(pop) && /Open as another user/.test(pop), '');
  await b.eval(`document.querySelector('[data-open-as-another] summary').click()`);
  await sleep(300);
  await b.shot('08_identity_popover.png');
  const stor = await b.storageAndCookies();
  log('no storage without keep', stor.session.length === 0 && stor.local.every(([k]) => !/token/i.test(k)) && !stor.cookie, JSON.stringify({ session: stor.session.length, local: stor.local.map(([k]) => k) }));
  log('URL has no token', !/[?&]token=|bearer|eyJ/i.test(stor.href), stor.href);

  // 8. network evidence
  const reqs = [...b.reqById.values()];
  const withAuth = reqs.filter((r) => r.hasAuth);
  log('Authorization only to the gateway', withAuth.length > 0 && withAuth.every((r) => r.url.startsWith(gw)), `${withAuth.length} requests with a header`);
  log('statuses seen on /api/events', true, JSON.stringify([...new Set(reqs.filter((r) => /\/api\/events/.test(r.url)).map((r) => r.status))]));
  log('no request URL contains a token', reqs.every((r) => !/eyJ/.test(r.url)), '');

  // 9. cluster with identity, token kept while navigating
  await b.eval(`document.querySelector('[data-identity-chip]').click()`);
  await b.eval(`location.hash = '#/cluster'`);
  await sleep(1000);
  await b.shot('09_cluster_with_identity.png');
  await b.eval(`location.hash = '#/'`);
  await sleep(500);

  // 10. forget token -> gateway requires a token again
  await forget();
  const back = await b.waitFor(`document.querySelector('[data-auth-key="requires_token"]')`, 20000);
  log('forget token: gateway requires a token again', Boolean(back), '');
  await b.shot('10_after_forget.png');

  // 11. keep for this tab survives a reload; forget clears it
  await useToken(T.tenantA, true);
  await b.waitFor(`document.querySelector('[data-conn-kind="live"]')`, 20000);
  const st2 = await b.storageAndCookies();
  log('keep: exactly one tab-storage entry, none in localStorage', st2.session.length === 1 && st2.local.every(([k]) => !/token/i.test(k)), JSON.stringify(st2.session.map(([k]) => k)));
  await b.send('Page.reload');
  await sleep(800);
  const stillLive = await b.waitFor(`document.querySelector('[data-conn-kind="live"]')`, 20000);
  log('keep: after reload the page is live without asking', Boolean(stillLive), '');
  await forget();
  await sleep(500);
  const st3 = await b.storageAndCookies();
  log('forget clears tab storage', st3.session.length === 0, '');

  // 12. memory only: reload loses it
  await b.waitFor(`document.querySelector('[data-auth-banner]')`, 20000);
  await useToken(T.tenantA, false);
  await b.waitFor(`document.querySelector('[data-conn-kind="live"]')`, 20000);
  await b.send('Page.reload');
  await sleep(800);
  const asks = await b.waitFor(`document.querySelector('[data-auth-key="requires_token"]')`, 20000);
  log('memory only: reload asks for the token again', Boolean(asks), '');

  // 13. expiry countdown (a token that lives 25 s, minted now with the lane-local helper)
  const { mint } = await import(process.env.L2_MINT);
  const nowS = Math.floor(Date.now() / 1000);
  await useToken(mint({ iss: 'phantom-cache-auth', aud: 'phantom-cache-api', iat: nowS, sub: 'erin', tenant_id: 'tenant-a', roles: ['telemetry:read'], exp: nowS + 25 }));
  await b.waitFor(`document.querySelector('[data-conn-kind="live"]')`, 20000);
  await b.eval(`document.querySelector('[data-identity-chip]').click()`);
  await sleep(500);
  const exp1 = await b.eval(`document.querySelector('[data-claim="expiry"]')?.innerText`);
  await b.shot('11_expiry_countdown.png');
  await sleep(27000);
  const exp2 = await b.eval(`document.querySelector('[data-claim="expiry"]')?.innerText`);
  const chip13 = await b.eval(`document.querySelector('[data-identity-chip]')?.innerText`);
  log('expiry countdown then expired', /Expires in/.test(exp1 || '') && /Expired/.test(exp2 || '') && /Token expired/.test(chip13 || ''), `${exp1} -> ${exp2} | chip: ${chip13}`);
  await b.shot('12_expired_after_countdown.png');
  await forget();

  // 14. hand-off by postMessage from the opener
  await b.goto(url, 800);
  await b.eval(`window.__tok = ${JSON.stringify(T.tenantB)}; window.__gw = ${JSON.stringify(gw)};
    window.addEventListener('message', (e) => { if (e.data && e.data.type === 'phantomcache:token-request') e.source.postMessage({ type: 'phantomcache:token', v: 1, gateway: window.__gw, token: window.__tok }, e.origin); });`);
  await b.send('Runtime.evaluate', { expression: `window.open(${JSON.stringify(`${ui}/?gateway=${encodeURIComponent(gw)}#/`)}, 'second'); true`, userGesture: true });
  await sleep(2500);
  const targets = await (await fetch(`http://127.0.0.1:${b.port}/json`)).json();
  const pages = targets.filter((t) => t.type === 'page');
  const other = pages.find((t) => t.webSocketDebuggerUrl !== b.ws.url);
  if (!other) {
    log('hand-off: second window found', false, `${pages.length} pages`);
  } else {
    const ws = new WebSocket(other.webSocketDebuggerUrl);
    await new Promise((r, j) => { ws.onopen = r; ws.onerror = j; });
    let mid = 0;
    const pend = new Map();
    ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && pend.has(d.id)) { pend.get(d.id)(d); pend.delete(d.id); } };
    const send = (method, params = {}) => new Promise((res) => { const id = ++mid; pend.set(id, res); ws.send(JSON.stringify({ id, method, params })); });
    await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false });
    const ev = async (e) => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
    let ok = false;
    for (let i = 0; i < 40 && !ok; i++) { ok = await ev(`!!document.querySelector('[data-conn-kind="live"]') && /Signed in as tenant-b/.test(document.querySelector('[data-identity-chip]')?.innerText || '')`); if (!ok) await sleep(500); }
    log('hand-off: second window received the tenant-b token by postMessage and is live', Boolean(ok), '');
    const shot = await send('Page.captureScreenshot', { format: 'png' });
    mkdirSync(`${evidence}/shots`, { recursive: true });
    writeFileSync(`${evidence}/shots/13_handoff_second_window.png`, Buffer.from(shot.result.data, 'base64'));
    log('hand-off: nothing in the second window URL', !/eyJ|token/i.test(await ev('location.href')), '');
    log('hand-off: not written to tab storage', (await ev('sessionStorage.length')) === 0, '');
    ws.close();
  }
  const bad = b.consoleLog.filter((c) => /exception|log\.error/.test(c.kind) && !/Failed to load resource|401|403|ERR_/.test(c.text));
  log('console: no page exceptions', bad.length === 0, JSON.stringify(bad.slice(0, 3)));
  log('console: no token printed', !JSON.stringify(b.consoleLog).includes('eyJ'), '');
} finally {
  b.saveLogs('l2auth');
  mkdirSync(evidence, { recursive: true });
  writeFileSync(`${evidence}/l2auth.results.json`, JSON.stringify(out, null, 1));
  console.log('chrome closed:', await b.close());
}
