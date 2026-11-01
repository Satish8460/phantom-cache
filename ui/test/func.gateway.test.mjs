// F4: gateway address model (gatewayConfig.js), the labApi probe/tester and the token rules.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  DEFAULT_GATEWAY, STORAGE_KEY, createGatewayStore, describeProbe, isLoopbackHost, originChanged, parseSaved,
  readSavedRaw, resolveGatewayConfig, validateGatewayInput, writeSavedGateway,
} from '../src/lib/gatewayConfig.js';
import { createLabClient, networkFix, testGatewayConnection } from '../src/lib/labApi.js';
import { fakeResponse } from './helpers/labFixtures.mjs';

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src');

function memStorage(initial = {}) {
  const m = new Map(Object.entries(initial));
  return { writes: [], getItem: (k) => (m.has(k) ? m.get(k) : null), setItem(k, v) { this.writes.push([k, v]); m.set(k, v); }, removeItem: (k) => { m.delete(k); }, dump: () => Object.fromEntries(m) };
}
const throwing = () => { throw new Error('denied'); };
const throwingStorage = { getItem: throwing, setItem: throwing, removeItem: throwing };

// ---------------------------------------------------------------- validation table

const V = (t, o) => validateGatewayInput(t, o);
const codes = (r) => r.errors.map((e) => e.code);
const ncodes = (r) => r.notices.map((n) => n.code);

test('validate: accepted forms are normalised to an origin', () => {
  assert.equal(V('http://localhost:8085').url, 'http://localhost:8085');
  assert.equal(V('  http://127.0.0.1:18085/  ').url, 'http://127.0.0.1:18085');
  assert.equal(V('HTTP://LOCALHOST:8085/').url, 'http://localhost:8085');
  assert.equal(V('https://gw.example.org').url, 'https://gw.example.org');
  assert.equal(V('http://[::1]:8085').url, 'http://[::1]:8085');
  assert.equal(V('http://localhost:80/').url, 'http://localhost', 'default port is dropped by URL.origin');
});

test('validate: path, query and hash are dropped with a notice', () => {
  const r = V('http://localhost:8085/x?y#z');
  assert.equal(r.ok, true);
  assert.equal(r.url, 'http://localhost:8085');
  assert.ok(ncodes(r).includes('origin_only'));
  assert.ok(!ncodes(V('http://localhost:8085/')).includes('origin_only'), 'a bare trailing slash is not worth a notice');
});

test('validate: error table', () => {
  assert.deepEqual(codes(V('')), ['empty']);
  assert.deepEqual(codes(V('   ')), ['empty']);
  assert.deepEqual(codes(V(null)), ['empty']);
  assert.deepEqual(codes(V(undefined)), ['empty']);
  assert.deepEqual(codes(V('ftp://x')), ['scheme']);
  assert.deepEqual(codes(V('javascript://example.com/%0Aalert(1)')), ['scheme']);
  assert.deepEqual(codes(V('file:///c:/x')), ['scheme']);
  assert.deepEqual(codes(V('http://u:p@host')), ['credentials']);
  assert.deepEqual(codes(V('http://user@host')), ['credentials']);
  assert.deepEqual(codes(V('http://')), ['invalid']);
  assert.deepEqual(codes(V('http://exa mple.com')), ['invalid']);
  assert.deepEqual(codes(V('http://host:99999')), ['invalid']);
  assert.deepEqual(codes(V('x'.repeat(3000))), ['too_long']);
  const ms = V('localhost:8085');
  assert.deepEqual(codes(ms), ['missing_scheme']);
  assert.equal(ms.suggestion, 'http://localhost:8085');
  assert.match(ms.errors[0].text, /http:\/\/localhost:8085/);
  assert.equal(ms.ok, false);
  assert.equal(ms.url, null, 'never auto-guessed');
  assert.match(V('ftp://x').errors[0].text, /Only http and https are allowed/);
  assert.match(V('http://u:p@host').errors[0].text, /Do not put a user name or password/);
  // the password never appears in any message
  assert.doesNotMatch(JSON.stringify(V('http://alice:s3cretpass@host')), /s3cretpass|alice/);
});

test('validate: notices (loopback, non-loopback + anonymous, mixed content, cross-origin)', () => {
  assert.ok(ncodes(V('http://localhost:8085')).includes('loopback'));
  assert.ok(ncodes(V('http://127.0.0.1:1')).includes('loopback'));
  assert.ok(ncodes(V('http://[::1]:8085')).includes('loopback'));
  const remote = V('http://gw.example.org:8085', { securityMode: 'anonymous_dev_loopback_only' });
  assert.ok(ncodes(remote).includes('non_loopback_anonymous'));
  assert.match(remote.notices.find((n) => n.code === 'non_loopback_anonymous').text, /421/);
  assert.ok(!ncodes(V('http://gw.example.org:8085', { securityMode: 'jwt' })).includes('non_loopback_anonymous'));
  assert.ok(!ncodes(V('http://gw.example.org:8085')).includes('non_loopback_anonymous'), 'security mode unknown: no claim');
  assert.ok(ncodes(V('http://gw.example.org:8085', { pageProtocol: 'https:' })).includes('mixed_content'));
  assert.ok(!ncodes(V('http://localhost:8085', { pageProtocol: 'https:' })).includes('mixed_content'), 'loopback is exempt');
  assert.ok(!ncodes(V('https://gw.example.org', { pageProtocol: 'https:' })).includes('mixed_content'));
  const cross = V('http://localhost:8085', { pageOrigin: 'http://localhost:3300' });
  assert.ok(ncodes(cross).includes('cross_origin'));
  assert.match(cross.notices.find((n) => n.code === 'cross_origin').text, /http:\/\/localhost:3300/);
  assert.ok(!ncodes(V('http://localhost:3300', { pageOrigin: 'http://localhost:3300' })).includes('cross_origin'));
});

test('isLoopbackHost', () => {
  for (const h of ['localhost', 'LOCALHOST', '127.0.0.1', '127.1.2.3', '[::1]']) assert.equal(isLoopbackHost(h), true, h);
  for (const h of ['example.org', '10.0.0.1', '128.0.0.1', 'localhost.evil.com', '']) assert.equal(isLoopbackHost(h), false, h);
});

// ---------------------------------------------------------------- resolution order

const SAVED = JSON.stringify({ url: 'http://127.0.0.1:9001' });

test('resolution: ?gateway= beats saved beats env beats default', () => {
  const all = { search: '?gateway=http://localhost:7001', savedRaw: SAVED, envValue: 'http://localhost:7003' };
  assert.deepEqual([resolveGatewayConfig(all).url, resolveGatewayConfig(all).source], ['http://localhost:7001', 'query']);
  const noQuery = { ...all, search: '' };
  assert.deepEqual([resolveGatewayConfig(noQuery).url, resolveGatewayConfig(noQuery).source], ['http://127.0.0.1:9001', 'saved']);
  const envOnly = { search: '', savedRaw: null, envValue: 'http://localhost:7003' };
  assert.deepEqual([resolveGatewayConfig(envOnly).url, resolveGatewayConfig(envOnly).source], ['http://localhost:7003', 'env']);
  const none = resolveGatewayConfig({});
  assert.deepEqual([none.url, none.source, none.status, none.notes.length], [DEFAULT_GATEWAY, 'default', 'ok', 0]);
});

test('resolution: a bad value falls back VISIBLY (status fallback + a note naming it), to the next valid source, not silently to the default', () => {
  const r = resolveGatewayConfig({ search: '?gateway=ftp://x', savedRaw: SAVED, envValue: 'http://localhost:7003' });
  assert.equal(r.url, 'http://127.0.0.1:9001');
  assert.equal(r.source, 'saved');
  assert.equal(r.status, 'fallback');
  assert.equal(r.rejected.length, 1);
  assert.equal(r.rejected[0].source, 'query');
  assert.equal(r.notes[0].code, 'ignored_query');
  assert.equal(r.notes[0].level, 'warn');
  assert.match(r.notes[0].text, /Ignored the address from the page link: only http and https are allowed/);
  // everything bad -> default, still visible
  const worst = resolveGatewayConfig({ search: '?gateway=nonsense', savedRaw: '{not json', envValue: 'http://u:p@h' });
  assert.equal(worst.url, DEFAULT_GATEWAY);
  assert.equal(worst.status, 'fallback');
  assert.deepEqual(worst.notes.map((n) => n.code), ['ignored_query', 'ignored_saved', 'ignored_env']);
  assert.match(worst.notes[1].text, /saved address/);
  assert.doesNotMatch(JSON.stringify(worst), /:p@/, 'credentials in a rejected candidate are not echoed');
});

test('resolution: corrupt saved shapes are ignored with a note', () => {
  for (const raw of ['{"url":5}', '[]', '"http://x"', 'null', '{}', '']) {
    const r = resolveGatewayConfig({ savedRaw: raw });
    assert.equal(r.source, 'default', raw);
    assert.equal(r.status, 'fallback', raw);
    assert.equal(r.notes[0].code, 'ignored_saved', raw);
  }
  assert.deepEqual(parseSaved(null), { url: null, problem: null });
  assert.equal(parseSaved('{"url":"http://a"}').url, 'http://a');
});

test('resolution: an address from the page link to a non-loopback host needs confirmation before a token is entered (D-5); loopback does not', () => {
  const remote = resolveGatewayConfig({ search: '?gateway=https://gw.example.org' });
  assert.equal(remote.fromLink, true);
  assert.equal(remote.needsLinkConfirmation, true);
  assert.ok(remote.notes.some((n) => n.code === 'from_link' && /Tokens you enter are sent there/.test(n.text)));
  const local = resolveGatewayConfig({ search: '?gateway=http://localhost:7001' });
  assert.equal(local.needsLinkConfirmation, false);
  assert.equal(resolveGatewayConfig({ savedRaw: JSON.stringify({ url: 'https://gw.example.org' }) }).needsLinkConfirmation, false);
});

// ---------------------------------------------------------------- storage

test('storage: works when storage throws (read, write, access itself)', () => {
  // getItem throws: the read itself fails, reported as unavailable (not a crash)
  assert.deepEqual(readSavedRaw(() => throwingStorage), { available: false, raw: null });
  assert.equal(readSavedRaw(throwing).available, false);
  assert.equal(readSavedRaw(() => null).available, false);
  assert.deepEqual(writeSavedGateway('http://localhost:1', throwing), { ok: false, reason: 'unavailable' });
  assert.deepEqual(writeSavedGateway('http://localhost:1', () => throwingStorage), { ok: false, reason: 'unavailable' });
  const s = createGatewayStore({ getStorage: throwing });
  assert.equal(s.getSnapshot().url, DEFAULT_GATEWAY);
  assert.equal(s.getSnapshot().storageAvailable, false);
  const r = s.apply('http://localhost:7001');
  assert.equal(r.ok, true);
  assert.equal(r.persisted, 'not_saved');
  assert.ok(r.notices.some((n) => n.code === 'not_remembered'));
  assert.equal(s.getSnapshot().url, 'http://localhost:7001', 'the address is in use even though it could not be saved');
  assert.equal(s.getSnapshot().remembered, false);
  assert.equal(s.getSnapshot().notes[0].code, 'not_remembered');
  const s2 = createGatewayStore({ getStorage: () => throwingStorage });
  assert.equal(s2.apply('http://localhost:7002').persisted, 'not_saved');
  assert.doesNotThrow(() => s2.reset());
});

test('storage: after Apply the storage holds exactly one key whose value is {"url": origin}; query, path and credentials never reach it', () => {
  const st = memStorage();
  const s = createGatewayStore({ getStorage: () => st });
  const r = s.apply('http://localhost:7001/p?token=abc.def.ghi#frag');
  assert.equal(r.ok, true);
  assert.deepEqual(Object.keys(st.dump()), [STORAGE_KEY]);
  assert.equal(st.dump()[STORAGE_KEY], '{"url":"http://localhost:7001"}');
  assert.equal(st.writes.length, 1);
  assert.doesNotMatch(JSON.stringify(st.dump()), /token|abc\.def/);
  // credentials are rejected before any write
  const before = st.writes.length;
  assert.equal(s.apply('http://u:p@localhost:7002').ok, false);
  assert.equal(st.writes.length, before);
  assert.equal(s.getSnapshot().url, 'http://localhost:7001');
});

test('storage: remember=false writes nothing; a previously saved address is untouched', () => {
  const st = memStorage({ [STORAGE_KEY]: SAVED });
  const s = createGatewayStore({ getStorage: () => st });
  assert.equal(s.getSnapshot().source, 'saved');
  assert.equal(s.getSnapshot().remembered, true);
  const r = s.apply('http://localhost:7001', { remember: false });
  assert.equal(r.persisted, 'skipped');
  assert.equal(st.writes.length, 0);
  assert.equal(s.getSnapshot().remembered, false);
  assert.equal(s.getSnapshot().source, 'applied');
});

test('only gatewayConfig.js (plus the pre-existing theme.js and Shell.jsx nav-fold convenience) touch browser storage outside the lab file set (plus lib/tokenSession.js, the single credential holder, L2 / D-045: sessionStorage only, opt-in); the gateway modules never mention a token', () => {
  const walk = (d) => fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
  const users = walk(SRC).filter((f) => /\.(js|jsx)$/.test(f)).filter((f) => /localStorage|sessionStorage/.test(fs.readFileSync(f, 'utf8'))).map((f) => path.relative(SRC, f).replace(/\\/g, '/'));
  for (const u of users) assert.ok(['lib/gatewayConfig.js', 'lib/theme.js', 'components/Shell.jsx', 'lib/tokenSession.js'].includes(u), `unexpected storage use in ${u}`);
  const code = fs.readFileSync(path.join(SRC, 'lib/gatewayConfig.js'), 'utf8').split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');
  assert.doesNotMatch(code, /console\.|document\.cookie|sessionStorage/);
  assert.doesNotMatch(code, /getToken|bearer|authorization/i);
  const hook = fs.readFileSync(path.join(SRC, 'hooks/useGateway.js'), 'utf8');
  assert.doesNotMatch(hook, /localStorage|token/i);
});

// ---------------------------------------------------------------- store behaviour

test('store: subscribe/getSnapshot are stable between changes and notify on apply and reset', () => {
  const st = memStorage();
  const s = createGatewayStore({ getStorage: () => st, envValue: 'http://localhost:7003' });
  const a = s.getSnapshot();
  assert.equal(s.getSnapshot(), a, 'referentially stable');
  assert.equal(a.url, 'http://localhost:7003');
  assert.equal(a.source, 'env');
  let calls = 0;
  const off = s.subscribe(() => { calls += 1; });
  s.apply('http://localhost:7001');
  assert.equal(calls, 1);
  const b = s.getSnapshot();
  assert.notEqual(a, b);
  assert.equal(b.url, 'http://localhost:7001');
  assert.equal(b.epoch, 1);
  assert.equal(b.hostPort, 'localhost:7001');
  assert.equal(b.remembered, true);
  assert.equal(s.apply('http://localhost:7001').changed, false);
  assert.equal(s.getSnapshot().epoch, 1, 'same origin: epoch does not advance');
  const rr = s.reset();
  assert.deepEqual([rr.ok, rr.changed, rr.originChanged], [true, true, true]);
  assert.equal(s.getSnapshot().url, 'http://localhost:7003');
  assert.equal(s.getSnapshot().epoch, 2);
  assert.deepEqual(st.dump(), {}, 'reset forgets the saved address');
  off();
  const before = calls;
  s.apply('http://localhost:7005');
  assert.equal(calls, before, 'unsubscribed');
});

test('store: an invalid apply changes nothing and says why; the visible fallback note is cleared by a valid apply', () => {
  const s = createGatewayStore({ search: '?gateway=ftp://x', getStorage: () => memStorage() });
  assert.equal(s.getSnapshot().status, 'fallback');
  assert.equal(s.getSnapshot().notes[0].code, 'ignored_query');
  const rev = s.getSnapshot().revision;
  const bad = s.apply('nope');
  assert.deepEqual([bad.ok, bad.persisted, bad.changed], [false, 'skipped', false]);
  assert.equal(bad.errors[0].code, 'missing_scheme');
  assert.equal(s.getSnapshot().revision, rev);
  s.apply('http://localhost:7001');
  assert.equal(s.getSnapshot().status, 'ok');
  assert.deepEqual([...s.getSnapshot().notes], []);
});

test('store: the address cannot change while a start/cancel request is pending (lock), and the snapshot says why', () => {
  const s = createGatewayStore({ getStorage: () => memStorage() });
  s.setLock('a run is being started');
  assert.equal(s.getSnapshot().locked, 'a run is being started');
  const r = s.apply('http://localhost:7001');
  assert.equal(r.ok, false);
  assert.equal(r.errors[0].code, 'busy');
  assert.equal(s.reset().ok, false);
  assert.equal(s.getSnapshot().url, DEFAULT_GATEWAY);
  s.setLock(null);
  assert.equal(s.apply('http://localhost:7001').ok, true);
});

// ---------------------------------------------------------------- token rule

test('originChanged: a token must be forgotten when the origin changes, not on a no-op', () => {
  assert.equal(originChanged('http://localhost:8085', 'http://localhost:8085'), false);
  assert.equal(originChanged('http://localhost:8085', 'http://localhost:8086'), true);
  assert.equal(originChanged('http://localhost:8085', 'https://localhost:8085'), true);
  assert.equal(originChanged('http://localhost:8085', 'http://127.0.0.1:8085'), true);
});

test('token rule at the client: a client built for gateway B with a getToken that reads the provider token ref sends NO Authorization once the ref is cleared', async () => {
  // Mirrors LabProvider: tokenRef survives a baseUrl change today (defect A14). The store tells the provider to clear it.
  const tokenRef = { current: 'header.payload.sig' };
  const seen = [];
  const fetchImpl = async (url, init) => { seen.push({ url, auth: init.headers.Authorization ?? null }); return fakeResponse({ status: 200, body: { schema: 'lab-1' } }); };
  const s = createGatewayStore({ getStorage: () => memStorage() });
  const clientFor = (url) => createLabClient({ baseUrl: url, getToken: () => tokenRef.current, fetchImpl });
  await clientFor(s.getSnapshot().url).capabilities();
  assert.equal(seen[0].auth, 'Bearer header.payload.sig');
  const prev = s.getSnapshot().url;
  const r = s.apply('http://localhost:7001');
  if (originChanged(prev, s.getSnapshot().url)) tokenRef.current = null; // the provider's job on epoch change
  assert.equal(r.originChanged, true);
  await clientFor(s.getSnapshot().url).capabilities();
  assert.equal(seen[1].url.startsWith('http://localhost:7001/'), true);
  assert.equal(seen[1].auth, null, 'no credential is sent to the new origin');
});

test('the token never appears in a URL: client URLs carry no credentials for any gateway the store can produce', async () => {
  const seen = [];
  const fetchImpl = async (url) => { seen.push(url); return fakeResponse({ status: 200, body: { schema: 'lab-1' } }); };
  const c = createLabClient({ baseUrl: 'http://localhost:7001', getToken: () => 'SECRETTOKEN', fetchImpl });
  await c.capabilities();
  await c.probe();
  assert.ok(seen.every((u) => !/SECRETTOKEN|token=|@/.test(u)));
});

// ---------------------------------------------------------------- probe and Test connection

test('labApi.probe: ok, 401 (answered, refused), 403, 404 is not "Lab off", network, abort', async () => {
  const mk = (resp) => createLabClient({ baseUrl: 'http://localhost:7001', fetchImpl: async () => { if (resp instanceof Error) throw resp; return resp; } });
  assert.deepEqual(await mk(fakeResponse({ status: 200, body: '{}' })).probe(), { ok: true, status: 200 });
  const r401 = await mk(fakeResponse({ status: 401, headers: { 'x-auth-error': 'MissingToken' }, body: 'no' })).probe();
  assert.equal(r401.ok, false);
  assert.equal(r401.error.kind, 'unauthorized_missing');
  assert.equal((await mk(fakeResponse({ status: 403, body: 'no' })).probe()).error.kind, 'forbidden_role');
  assert.equal((await mk(fakeResponse({ status: 404, body: 'x' })).probe()).error.kind, 'not_found');
  assert.equal((await mk(new TypeError('Failed to fetch')).probe()).error.kind, 'network');
  const abort = Object.assign(new Error('x'), { name: 'AbortError' });
  assert.equal((await mk(abort).probe()).error.kind, 'aborted');
  const urls = [];
  await createLabClient({ baseUrl: 'http://localhost:7001/', fetchImpl: async (u) => { urls.push(u); return fakeResponse({ status: 200, body: '{}' }); } }).probe();
  assert.deepEqual(urls, ['http://localhost:7001/api/v1/metrics?schema=2']);
});

test('Test connection: classifies each probe separately; no token is sent unless the caller passes getToken', async () => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, auth: init.headers.Authorization ?? null });
    if (url.includes('/lab/capabilities')) return fakeResponse({ status: 200, body: { schema: 'lab-1' } });
    return fakeResponse({ status: 401, headers: { 'x-auth-error': 'MissingToken' }, body: 'x' });
  };
  const res = await testGatewayConnection({ baseUrl: 'http://localhost:7001', fetchImpl });
  assert.deepEqual(res.capabilities, { ok: true, kind: null, status: 200 });
  assert.deepEqual(res.metrics, { ok: false, kind: 'unauthorized_missing', status: 401 });
  assert.ok(calls.every((c) => c.auth === null), 'default sends no token to a candidate address');
  await testGatewayConnection({ baseUrl: 'http://localhost:7001', fetchImpl, getToken: () => 'T' });
  assert.ok(calls.slice(2).every((c) => c.auth === 'Bearer T'));
  const down = await testGatewayConnection({ baseUrl: 'http://localhost:7001', fetchImpl: async () => { throw new TypeError('x'); } });
  assert.deepEqual([down.capabilities.kind, down.metrics.kind], ['network', 'network']);
});

test('describeProbe: says who knows (gateway answered vs no answer) and never invents a cause for a network failure', () => {
  const ok = describeProbe({ ok: true, kind: null, status: 200 });
  assert.deepEqual([ok.word, ok.who, ok.answered], ['Live', 'gateway', true]);
  const refused = describeProbe({ ok: false, kind: 'unauthorized_missing', status: 401 });
  assert.deepEqual([refused.word, refused.who], ['Unauthorised', 'gateway']);
  assert.match(refused.text, /The gateway answered/);
  const net = describeProbe({ ok: false, kind: 'network', status: 0 });
  assert.deepEqual([net.word, net.who, net.answered], ['Disconnected', 'page', false]);
  assert.match(net.text, /cannot tell a stopped gateway, a wrong address, a network fault or a blocked cross-origin request apart/);
  const off = describeProbe({ ok: false, kind: 'disabled', status: 404 });
  assert.equal(off.word, 'Unavailable');
});

test('networkFix names the actual gateway and page origin and no longer points at a header that does not show the address', () => {
  const t = networkFix('http://localhost:8085', 'http://localhost:3300');
  assert.match(t, /running at http:\/\/localhost:8085/);
  assert.match(t, /must include http:\/\/localhost:3300/);
  assert.doesNotMatch(t, /header/);
  assert.match(networkFix(null, null), /this page origin/);
  const stat = createLabClient({ baseUrl: 'x' });
  assert.ok(stat);
});
