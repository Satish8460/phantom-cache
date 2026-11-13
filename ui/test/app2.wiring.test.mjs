// V2B app-level wiring tests: the connection vocabulary, page-owned retry, the Change address interaction, the memory-only token rule,
// the non-loopback link confirmation (AD-11), and the retired strings. SSR and reducer level: no browser effect is exercised here
// (the browser pass is in docs/design/remediation/increment_V2B_pages_app_wiring.md). Passing these tests is not evidence of visual quality.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getApp2Harness, makeLinkCtx } from './helpers/app2Harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { livePayload } from './helpers/fixtures.mjs';
import { describeTelemetryLink, LINK_WORDS } from '../src/lib/linkState.js';
import { needsLinkConfirm, retryText } from '../src/screens/linkUi.jsx';
import { backoffMs } from '../src/lib/connection.js';
import { resolveGatewayConfig, STORAGE_KEY } from '../src/lib/gatewayConfig.js';

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src');
const read = (rel) => fs.readFileSync(path.join(SRC, rel), 'utf8');
const stripComments = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '');
let H;
before(async () => {
  H = await getApp2Harness();
});

const DATA = ['overview', 'traffic', 'cache', 'freshness', 'origin'];
const memStorage = () => {
  const m = new Map();
  return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: (k) => m.delete(k), _m: m };
};

// ---------------------------------------------------------------- AD-10: the page owns the retry

test('AD-10: useTelemetry closes the stream on EVERY error and schedules the retry itself (no browser-owned retry left)', () => {
  const src = read('hooks/useTelemetry.js');
  assert.match(src, /es\.onerror = \(\) => \{[\s\S]*?dispatch\(\{ type: 'error', closed: true/);
  assert.doesNotMatch(src, /readyState/, 'the browser-owned retry branch (readyState 0) must be gone');
  assert.match(src, /es\.close\(\);[\s\S]*?retryScheduled/);
  assert.match(src, /backoffMs\(attemptRef\.current\)/);
});

test('AD-10: the countdown is derived from the scheduled time and only when the page owns the timer', async () => {
  const t0 = 5_000_000;
  const mk = async (attempts, now) => {
    const actions = [];
    for (let i = 0; i < attempts; i += 1) actions.push({ type: 'error', closed: true, now: t0 });
    actions.push({ type: 'retryScheduled', at: t0 + backoffMs(attempts - 1) });
    return makeLinkCtx({ actions, now });
  };
  const first = await mk(1, t0 + 400);
  assert.equal(first.link.word, LINK_WORDS.reconnecting);
  assert.equal(first.link.retryOwner, 'page');
  assert.equal(retryText(first.link), 'Next try in 1 s.');
  const third = await mk(3, t0 + 1000);
  assert.equal(third.link.word, LINK_WORDS.disconnected, 'attempt 3 and later is Disconnected');
  assert.equal(retryText(third.link), 'Next try in 3 s.', 'backoff for the third failure is 4 s; 1 s has passed');
  assert.equal(third.link.changeAddress, true, 'Change address is offered once the page is Disconnected');
  const due = await mk(3, t0 + 9000);
  assert.equal(retryText(due.link), 'Trying again now.');
  // A browser-owned retry (a non-closed error) shows NO countdown, honestly.
  const browser = await makeLinkCtx({ actions: [{ type: 'error', closed: false, now: t0 }], now: t0 + 500 });
  assert.equal(browser.link.retryOwner, 'browser');
  assert.match(retryText(browser.link), /cannot show when/);
});

// ---------------------------------------------------------------- one vocabulary; AD-9 and no "Recovering"

test('connection words: every data screen in every state shows the shared words only; never Unauthorised, Recovering or a retired string', async () => {
  const live = livePayload();
  const states = {
    loading: await makeLinkCtx({}),
    reconnecting: await makeLinkCtx({ actions: [{ type: 'error', closed: true, now: 1 }, { type: 'retryScheduled', at: 1_002_000 }] }),
    disconnected: await makeLinkCtx({ actions: [1, 2, 3].map(() => ({ type: 'error', closed: true, now: 1 })).concat([{ type: 'retryScheduled', at: 1_004_000 }]) }),
    stale: await makeLinkCtx({ frames: [live], receivedAt: 900_000, now: 910_000 }),
    degraded: await makeLinkCtx({ frames: [{ ...live, metrics: { ...live.metrics, cache_hits: { src: 'simulated', v: 5 } } }] }),
    badFrame: await makeLinkCtx({ frames: ['{"schema":1}'] }),
    withData: await makeLinkCtx({ frames: [live], actions: [{ type: 'error', closed: true, now: 1 }, { type: 'error', closed: true, now: 1 }, { type: 'error', closed: true, now: 1 }, { type: 'retryScheduled', at: 1_005_000 }] }),
  };
  for (const [name, ctx] of Object.entries(states)) {
    for (const id of DATA) {
      const html = H.renderScreen(id, ctx);
      assert.doesNotMatch(html, /Unauthori[sz]ed/, `${id}/${name}: telemetry can never be Unauthorised (AD-9)`);
      assert.doesNotMatch(html, /Recovering/, `${id}/${name}`);
      assert.doesNotMatch(html, /Reconnect now|Not reachable or not authorised|No connection to the gateway|Could not reach the gateway/, `${id}/${name}: retired strings`);
      assert.deepEqual(lintHtml(html, { requireH1: true }), [], `${id}/${name}`);
    }
  }
  // The word on the page is the word linkState gives, so the header (same view) and the page cannot disagree.
  const dis = H.renderScreen('overview', states.disconnected);
  assert.match(dis, /<span>Disconnected<\/span>/);
  assert.match(dis, /data-retry-countdown>Next try in 4 s\.</);
  assert.match(dis, /Retry now/);
  assert.match(dis, /Change address/);
  const rec = H.renderScreen('traffic', states.reconnecting);
  assert.match(rec, /<span>Reconnecting<\/span>/);
  assert.match(rec, /data-retry-countdown>Next try in 2 s\.</);
  assert.doesNotMatch(rec, /Change address/, 'before the third failure the address is not blamed');
  const withData = H.renderScreen('freshness', states.withData);
  assert.match(withData, /data-link="no_answer"|data-link="disconnected"/);
  assert.match(withData, /last values received are shown as stale/);
  assert.match(withData, /data-state="stale"/);
});

test('the notice is a client observation: it says who knows and never claims gateway health', async () => {
  const ctx = await makeLinkCtx({ frames: [livePayload()], receivedAt: 900_000, now: 910_000 });
  const html = H.renderScreen('traffic', ctx);
  assert.match(html, /Seen by this page/);
  assert.match(html, /What this page can and cannot tell/);
  assert.doesNotMatch(html, /gateway is (down|healthy|running|stopped)/i);
  const dis = await makeLinkCtx({ actions: [1, 2, 3].map(() => ({ type: 'error', closed: true, now: 1 })) });
  assert.match(H.renderScreen('overview', dis), /cannot tell a stopped gateway, a wrong address, a network fault or a blocked cross-origin request apart/);
});

// ---------------------------------------------------------------- Change address and the memory-only / safe-storage rules

test('Change address form: label, validation, Test, two Use actions, Reset, and the memory-only token sentence; no token field', () => {
  const { html, gw } = H.renderAddressForm({});
  assert.match(html, /<label[^>]*>Gateway address<\/label>/);
  assert.match(html, /data-address-input/);
  assert.match(html, /Test connection/);
  assert.match(html, /Use and remember/);
  assert.match(html, /Use for this visit/);
  assert.match(html, /Reset address/);
  assert.match(html, new RegExp(`In use: <span class="mono">${gw.url.replace(/[/.:]/g, '\\$&')}</span>, built-in default`));
  assert.doesNotMatch(html, /type="password"|Bearer|token field/i);
  assert.deepEqual(lintHtml(`<main><h1>t</h1>${html}</main>`, { requireH1: true }), []);
});

test('Change address: apply validates, remembers only the origin, tolerates blocked storage, and signals an origin change (token must be forgotten)', () => {
  const { store } = H.renderAddressForm({ getStorage: () => memStorage() });
  assert.equal(store.validate('localhost:8085').ok, false);
  assert.match(store.validate('localhost:8085').errors[0].text, /Add http:\/\/ or https:\/\//);
  assert.equal(store.validate('http://user:pw@host:1').ok, false, 'credentials in an address are refused');
  const mem = memStorage();
  const s2 = H.renderAddressForm({ getStorage: () => mem }).store;
  const e0 = s2.getSnapshot().epoch;
  const r = s2.apply('http://127.0.0.1:9000/ignored?x=1#y', { remember: true });
  assert.equal(r.ok, true);
  assert.equal(r.persisted, 'saved');
  assert.equal(mem._m.get(STORAGE_KEY), '{"url":"http://127.0.0.1:9000"}', 'only the origin is saved');
  assert.equal(s2.getSnapshot().epoch, e0 + 1, 'an origin change bumps the epoch, which remounts the Lab provider (token ref dropped)');
  const same = s2.apply('http://127.0.0.1:9000', { remember: false });
  assert.equal(same.originChanged, false);
  assert.equal(s2.getSnapshot().epoch, e0 + 1);
  // storage that throws: the address still applies and the person is told it is not remembered
  const broken = H.renderAddressForm({ getStorage: () => { throw new Error('blocked'); } }).store;
  const rb = broken.apply('http://127.0.0.1:9001', { remember: true });
  assert.equal(rb.ok, true);
  assert.equal(rb.persisted, 'not_saved');
  assert.ok(rb.notices.some((n) => n.code === 'not_remembered'));
  // while a run request is pending the address is locked
  s2.setLock('a run request is in progress');
  assert.equal(s2.apply('http://127.0.0.1:9002').ok, false);
});

test('App and the link UI never handle a token and touch storage only through lib/gatewayConfig and lib/theme', () => {
  for (const f of ['App.jsx', 'screens/linkUi.jsx', 'main.jsx']) {
    const src = read(f);
    assert.doesNotMatch(stripComments(src), /Authorization|getToken|setToken|tokenRef/, f);
    assert.doesNotMatch(stripComments(src), /localStorage|sessionStorage|indexedDB|document\.cookie/, `${f}: storage is reached only through the lib helpers`);
  }
  const link = read('screens/linkUi.jsx').replace(/\/\*[\s\S]*?\*\//g, '');
  assert.doesNotMatch(link, /Authorization|tokenRef|getToken|setToken/);
  assert.match(link, /Bearer tokens are kept in this page's memory only and are never saved/);
});

test('the Lab remounts on an origin change so its memory-only token is dropped; run and evidence ids are left behind', () => {
  const app = read('App.jsx');
  assert.match(app, /<LabScreen key=\{`gateway-\$\{epoch\}`\} sub=\{sub\} \/>/);
  assert.match(app, /renderScreen\(route\.id, route\.sub, gw\.epoch\)/);
  assert.match(app, /\^#\\\/lab\\\/\(runs\|evidence\)\\\/\./);
  assert.match(app, /window\.location\.hash = '#\/lab'/);
});

// ---------------------------------------------------------------- AD-11

test('AD-11: a non-loopback address from the page link blocks the Lab until confirmed for that address; loopback and typed addresses never ask', () => {
  const remote = resolveGatewayConfig({ search: '?gateway=http%3A%2F%2Fgateway.example%3A8085' });
  assert.equal(remote.needsLinkConfirmation, true);
  assert.equal(needsLinkConfirm(remote, null, 'lab'), true);
  assert.equal(needsLinkConfirm(remote, null, 'overview'), false, 'only the Lab has a token field');
  assert.equal(needsLinkConfirm({ ...remote }, remote.url, 'lab'), false, 'confirmed for this address');
  assert.equal(needsLinkConfirm({ ...remote }, 'http://other.example:1', 'lab'), true, 'a confirmation does not carry to another address');
  const loop = resolveGatewayConfig({ search: '?gateway=http%3A%2F%2F127.0.0.1%3A8097' });
  assert.equal(needsLinkConfirm(loop, null, 'lab'), false);
  const fromEnv = resolveGatewayConfig({ search: '', envValue: 'http://build.example:8085' });
  assert.equal(needsLinkConfirm(fromEnv, null, 'lab'), false, 'only the link source is distrusted');
  const app = read('App.jsx');
  assert.match(app, /needsLinkConfirm\(gw, confirmedUrl, route\.id\)/);
  assert.match(app, /\{needsConfirm \? \(\s*<LinkConfirmGate/);
});

test('AD-11: the confirmation names the host, says tokens would be sent there, saves nothing, and puts the safe option first', () => {
  const remote = resolveGatewayConfig({ search: '?gateway=http%3A%2F%2Fgateway.example%3A8085' });
  const html = H.renderConfirm({ ...remote, hostPort: 'gateway.example:8085' });
  assert.match(html, /role="alertdialog"/);
  assert.match(html, /gateway\.example:8085/);
  assert.match(html, /the token would be sent to that address/);
  assert.match(html, /Nothing is saved by this choice/);
  assert.ok(html.indexOf('Use the default address instead') < html.indexOf('I trust'), 'the safe option comes first');
  assert.deepEqual(lintHtml(`<main><h1>t</h1>${html}</main>`, { requireH1: true }), []);
});

test('gateway notes: a rejected or link-origin address is stated on every route, with Change address', () => {
  const bad = resolveGatewayConfig({ search: '?gateway=ftp%3A%2F%2Fx' });
  const html = H.renderNotes({ ...bad });
  assert.match(html, /Ignored the address from the page link/);
  assert.match(html, /Change address/);
  assert.equal(H.renderNotes({ notes: [] }), '');
});

// ---------------------------------------------------------------- the whole App, server-rendered

test('App renders: skip link first, one h1, shell indicator fallback, a live region that names the word only, no retired text', () => {
  const html = H.renderApp();
  assert.match(html, /^<div class="app"><a class="skip-link" href="#main"/);
  assert.equal(html.split('<h1').length - 1, 1);
  assert.match(html, /data-live-region="true">Connection: Connecting</);
  assert.match(html, /Connecting to the gateway/);
  assert.doesNotMatch(html, /Reconnect now|Unauthori[sz]ed|Recovering/);
  assert.match(html, /<footer class="app-footer">/);
  assert.deepEqual(lintHtml(html, { requireH1: true }), []);
});

test('the shell pill stays as the fallback and is hidden only once the control has taken the slot (data-linked)', () => {
  const css = fs.readFileSync(path.join(SRC, 'styles/pages.css'), 'utf8');
  assert.match(css, /\.app-header__status\[data-linked\] > \.pill \{ display: none; \}/);
  const app = read('App.jsx');
  assert.match(app, /setAttribute\('data-linked', '1'\)/);
  assert.match(app, /removeAttribute\('data-linked'\)/);
  assert.match(app, /<ConnectionControl slot=\{slot\}/);
});

test('the connection popover is a labelled non-modal dialog with Escape and outside-press close, and the indicator names its state', () => {
  const src = read('screens/linkUi.jsx');
  assert.match(src, /role="dialog" aria-label="Gateway connection"/);
  assert.match(src, /aria-haspopup="dialog"/);
  assert.match(src, /aria-expanded=\{open\}/);
  assert.match(src, /e\.key === 'Escape'/);
  assert.match(src, /btnRef\.current\?\.focus\(\)/);
  assert.match(src, /pointerdown/);
  assert.match(src, /Gateway connection: \$\{worst\.word\}/);
  assert.match(src, /Words describe what this page sees\. They say nothing about the health of the gateway itself\./);
});

test('Test connection sends no token and says who knows (the gateway answered vs no answer)', () => {
  const src = read('screens/linkUi.jsx');
  assert.match(src, /testGatewayConnection\(\{ baseUrl: v\.url, signal: ac\.signal \}\)/, 'no getToken is passed, so none is sent');
  assert.match(src, /No token is sent/);
});

test('telemetry cannot be Unauthorised even when the stream was refused: only a classified probe can say so, and the app passes none (AD-9)', async () => {
  const ctx = await makeLinkCtx({ actions: [1, 2, 3, 4].map(() => ({ type: 'error', closed: true, now: 1 })) });
  assert.notEqual(ctx.link.word, LINK_WORDS.unauthorised);
  assert.equal(describeTelemetryLink(ctx.state, ctx.now, { includeJwtInference: false }).notes.length, 0, 'the JWT inference note is off');
  const app = read('App.jsx');
  assert.doesNotMatch(app, /probe|includeJwtInference/);
});
