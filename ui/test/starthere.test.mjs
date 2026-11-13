// Start here page, Demo Hub links, per-screen help lines and the first-visit welcome (lane L2, work order D-053).
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';
import { DEFAULT_HUB, HELP, SCENARIOS, hubLink, resolveHub } from '../src/lib/hub.js';
import { SCREENS, footerFor, parseRoute, titleFor } from '../src/lib/routes.js';
import { WELCOME_KEY, markWelcomeSeen, readWelcomeSeen } from '../src/lib/tokenSession.js';

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), '../src');
const read = (f) => fs.readFileSync(path.join(SRC, f), 'utf8');
let H;
before(async () => { H = await getHarness(); });
const mem = () => { const m = new Map(); return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), _m: m }; };

test('hub address: default 127.0.0.1:3201; ?hub= accepted only for http(s) loopback without credentials; anything else falls back with a note', () => {
  assert.deepEqual(resolveHub(''), { url: DEFAULT_HUB, source: 'default', note: null });
  assert.equal(resolveHub('?hub=http://localhost:4000/x?y=1#z').url, 'http://localhost:4000');
  assert.equal(resolveHub('?hub=https://127.0.0.1:9').url, 'https://127.0.0.1:9');
  assert.equal(resolveHub('?hub=http://[::1]:3201').url, 'http://[::1]:3201');
  for (const bad of ['http://evil.example', 'http://127.0.0.1.evil.example', 'ftp://127.0.0.1', 'javascript:alert(1)', 'http://u:p@127.0.0.1:3201', 'not a url', 'http://localhost.evil.com']) {
    const r = resolveHub(`?hub=${encodeURIComponent(bad)}`);
    assert.equal(r.url, DEFAULT_HUB, bad);
    assert.match(r.note, /Ignored \?hub=/, bad);
  }
});

// The owner's list (D:\ClaudeTemp\lanes\REQUIRED_SCENARIOS.md): 30 ids, 8+ groups, storyline order. A scenario missing from src/lib/hub.js fails here.
const REQUIRED = Object.freeze(['tour', 'gallery', 'stampede', 'peerfill', 'offload', 'workloads', 'scale', 'auth', 'roles', 'isolation', 'targetsafety', 'mtls', 'failclosed', 'multiuser', 'invalidation', 'deadnode', 'deletestorm', 'burstdelete', 'peerexpiry', 'freshness', 'eviction', 'largeobjects', 'originprotect', 'operability', 'restart', 'intelligence', 'intelcontrol', 'dashboards', 'benchmark', 'everything']);
const REQUIRED_GROUPS = Object.freeze(['Start', 'Speed and savings', 'Safety', 'Resilience', 'Freshness and memory', 'Protection', 'Operability', 'Intelligence', 'Observability', 'Performance', 'All']);

test('catalogue: the single list in lib/hub.js has exactly the 30 required ids in the required order, grouped as the owner file groups them', () => {
  assert.equal(REQUIRED.length, 30);
  assert.deepEqual(SCENARIOS.map((x) => x.id), [...REQUIRED]);
  assert.deepEqual([...new Set(SCENARIOS.map((x) => x.group))], [...REQUIRED_GROUPS]);
  assert.equal(new Set(SCENARIOS.map((x) => x.id)).size, 30, 'no duplicate id');
  for (const x of SCENARIOS) assert.ok(x.title && x.what.length >= 30 && x.what.length <= 120, `${x.id}: a one-line "what you will see"`);
  // when the owner file is present on this machine, the list must equal its table (ids, order, group)
  const file = 'D:/ClaudeTemp/lanes/REQUIRED_SCENARIOS.md';
  if (fs.existsSync(file)) {
    const rows = [...fs.readFileSync(file, 'utf8').matchAll(/^\|\s*\d+\s*\|\s*([a-z]+)\s*\|\s*([^|]+?)\s*\|/gm)].map((m) => [m[1], m[2]]);
    assert.deepEqual(rows, SCENARIOS.map((x) => [x.id, x.group]), 'lib/hub.js differs from REQUIRED_SCENARIOS.md');
  }
});

test('hub links: one deep link per scenario id, unknown ids go to the catalogue, no token anywhere in a link', () => {
  for (const id of REQUIRED) assert.equal(hubLink('http://127.0.0.1:3201/', id), `http://127.0.0.1:3201/#/scenario/${id}`);
  assert.equal(hubLink('http://127.0.0.1:3201', 'nope'), 'http://127.0.0.1:3201/');
  assert.equal(hubLink('http://127.0.0.1:3201', '../x'), 'http://127.0.0.1:3201/');
  assert.equal(hubLink(undefined, 'tour'), `${DEFAULT_HUB}/#/scenario/tour`);
});

test('help lines: the screens the order names link to the matching scenarios', () => {
  assert.deepEqual(HELP.cluster.scenarios, ['deadnode', 'peerfill']);
  assert.deepEqual(HELP.traffic.scenarios, ['scale']);
  assert.deepEqual(HELP.freshness.scenarios, ['freshness']);
  assert.deepEqual(HELP.cache.scenarios, ['eviction']);
  assert.deepEqual(HELP.origin.scenarios, ['stampede']);
  for (const [id, h] of Object.entries(HELP)) for (const sc of h.scenarios) assert.ok(SCENARIOS.some((s) => s.id === sc), `${id}: unknown scenario ${sc}`);
});

test('routes: Start here is the first navigation item, in its own group, with a title and a footer that is true for it', () => {
  assert.equal(SCREENS[0].id, 'start');
  assert.equal(SCREENS[0].label, 'Start here');
  assert.equal(SCREENS[0].group, 'Start');
  assert.deepEqual(parseRoute({ hash: '#/start' }), { id: 'start', notFound: false });
  assert.equal(titleFor({ id: 'start' }), 'Start here | PhantomCache');
  assert.doesNotMatch(footerFor({ id: 'start' }), /Telemetry schema 2/);
  const nav = H.renderShell({ connKind: 'live', currentId: 'start' });
  assert.ok(nav.indexOf('data-screen="start"') < nav.indexOf('data-screen="overview"'));
});

const page = (ctx) => H.renderStart(ctx);

test('page: three steps, the tour button, every scenario with its deep link, the glossary; lint clean', () => {
  const html = page({ ...makeCtx({ frames: [livePayload()] }), hub: { url: DEFAULT_HUB, note: null } });
  assert.match(html, /<h1[^>]*>Start here<\/h1>/);
  assert.equal((html.match(/data-start-steps[^>]*>[\s\S]*?<\/ol>/)[0].match(/<li>/g) || []).length, 3);
  assert.match(html, /Take the 5-minute guided tour/);
  for (const s of SCENARIOS) assert.match(html, new RegExp(`data-scenario="${s.id}"[\\s\\S]*?href="http://127\\.0\\.0\\.1:3201/#/scenario/${s.id}"[^>]*>Run in the Demo Hub`), s.id);
  assert.match(html, /data-demonstrate-everything[^>]*>Demonstrate everything/);
  assert.match(html, /href="http:\/\/127\.0\.0\.1:3201\/#\/scenario\/everything"/);
  assert.equal((html.match(/data-start-group=/g) || []).length, 11);
  assert.equal((html.match(/data-scenario=/g) || []).length, 30);
  assert.match(html, /target="_blank" rel="noopener noreferrer"/);
  for (const w of ['HIT', 'MISS', 'Peer', 'Coalesced', 'Stale', 'Tenant', 'Token']) assert.match(html, new RegExp(`<dt>${w}</dt>`));
  assert.doesNotMatch(html, /eyJ|[?&]token=/);
  assert.deepEqual(lintHtml(html), []);
});

test('page: honours a ?hub= address and shows the note when it was refused', () => {
  const html = page({ ...makeCtx({ frames: [livePayload()] }), hub: resolveHub('?hub=http://evil.example') });
  assert.match(html, /Ignored \?hub=/);
  assert.match(html, /href="http:\/\/127\.0\.0\.1:3201\/#\/scenario\/gallery"/);
  const other = page({ ...makeCtx({ frames: [livePayload()] }), hub: resolveHub('?hub=http://localhost:4100') });
  assert.match(other, /href="http:\/\/localhost:4100\/#\/scenario\/gallery"/);
});

test('page, offline: with no answer from the gateway it says so in plain words and points to the Hub; the page is still shown (not gated)', () => {
  const ctx = makeCtx({ frames: [], phase: 'closed' });
  const html = page({ ...ctx, link: { word: 'Disconnected', tone: 'critical', icon: 'error', cause: 'This page is getting no answer from the gateway.' }, hub: { url: DEFAULT_HUB } });
  assert.match(html, /data-start-offline/);
  assert.match(html, /No gateway answers at this address/);
  assert.match(html, /Start the demo cluster/);
  assert.match(html, /data-scenario="gallery"/);
  assert.deepEqual(lintHtml(html), []);
});

test('page, secure gateway without a token: says it needs a token and where to paste it', () => {
  const ctx = makeCtx({ frames: [] });
  const html = page({
    ...ctx,
    link: { word: 'Unauthorised', tone: 'warn', icon: 'warn', cause: 'The gateway answered and refused this request.' },
    auth: { chip: { tone: 'warn', icon: 'warn', text: 'Token required' }, answer: { key: 'requires_token', needsAction: true, title: 'This gateway requires a token', text: 'The gateway answered 401.' } },
    hub: { url: DEFAULT_HUB },
  });
  assert.match(html, /data-start-needs-token/);
  assert.match(html, /This gateway requires a token/);
  assert.match(html, /token chip at the top right/);
  assert.match(html, /Signed in as[\s\S]*?Token required/);
  assert.deepEqual(lintHtml(html), []);
});

test('help line: shown under the title of Overview, Traffic, Cache, Freshness, Origin, Cluster with the matching Hub link', () => {
  const ctx = { ...makeCtx({ frames: [livePayload()] }), hub: { url: DEFAULT_HUB } };
  for (const [id, scenario] of [['traffic', 'scale'], ['cache', 'eviction'], ['freshness', 'freshness'], ['origin', 'stampede'], ['overview', 'gallery']]) {
    const html = H.renderScreen(id, ctx);
    assert.match(html, new RegExp(`data-help-line="${id}"`), id);
    assert.match(html, /What am I looking at\?/);
    assert.match(html, new RegExp(`href="http://127\\.0\\.0\\.1:3201/#/scenario/${scenario}"`), id);
  }
  assert.match(H.renderScreen('cluster', { ...ctx, payload: ctx.payload }), /data-help-line="cluster"/);
  assert.doesNotMatch(H.renderScreen('traffic', makeCtx({ frames: [livePayload()] })), /Hub: undefined/);
});

test('welcome: remembered for this tab only, holds the word "seen", survives broken storage', () => {
  const st = mem();
  assert.equal(readWelcomeSeen(() => st), false);
  markWelcomeSeen(() => st);
  assert.equal(readWelcomeSeen(() => st), true);
  assert.deepEqual([...st._m.entries()], [[WELCOME_KEY, 'seen']]);
  const broken = () => { throw new Error('denied'); };
  assert.equal(readWelcomeSeen(broken), false);
  assert.doesNotThrow(() => markWelcomeSeen(broken));
  assert.match(read('lib/tokenSession.js'), /sessionStorage/);
});

test('welcome: a banner with Start here, the Hub and Dismiss; it is dismissible and the App shows it only until dismissed and never on Start here', () => {
  const html = H.renderWelcome({ hub: DEFAULT_HUB, onDismiss: () => {} });
  assert.match(html, /data-welcome/);
  assert.match(html, /href="#\/start"/);
  assert.match(html, /data-welcome-dismiss/);
  assert.deepEqual(lintHtml(html, { requireH1: false }), []);
  const app = read('App.jsx');
  assert.match(app, /!welcomeSeen && route\.id !== 'start' \? <WelcomeBanner/);
  assert.match(app, /readWelcomeSeen\(\)/);
  assert.doesNotMatch(app, /sessionStorage|localStorage/);
});

test('guard: Start here and the hub parts make no network call, log nothing, store nothing and put no token in a link', () => {
  for (const f of ['screens/StartHere.jsx', 'components/HubParts.jsx', 'lib/hub.js']) {
    const c = read(f).split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');
    assert.doesNotMatch(c, /\bfetch\s*\(|EventSource|console\.|localStorage|sessionStorage|Math\.random|innerHTML/, f);
    assert.doesNotMatch(c, /token=|access_token|Authorization/i, f);
  }
});
