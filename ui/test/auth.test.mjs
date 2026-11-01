// Control Center sign-in (lane L2, board D-045 item 2): token decoding, answers in plain words, the token holder, the authenticated
// request client, the telemetry stream reader, the window hand-off rules and the rendered surfaces.
// Tokens here are made-up test strings signed with nothing: the page never verifies, so the tests only need the shape.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import {
  HANDOFF_TYPE, decodeClaims, describeAnswer, evaluateHandoff, expiryState, expiryText, formatSpan, isLoopbackOrigin, normalizeTokenInput, chipView, roleHints,
} from '../src/lib/gatewayAuth.js';
import { TAB_KEY, createTokenSession } from '../src/lib/tokenSession.js';
import { createLabClient, isGatewayPath } from '../src/lib/labApi.js';
import { isAuthRefusal, runTelemetryStream } from '../src/lib/telemetryStream.js';

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), '../src');
const read = (f) => fs.readFileSync(path.join(SRC, f), 'utf8');
const code = (f) => read(f).split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');

const b64u = (o) => Buffer.from(JSON.stringify(o)).toString('base64url');
const SECRET_MARK = 'SECRETMARKER9f3a';
const jwt = (claims, header = { alg: 'HS256', typ: 'JWT' }) => `${b64u(header)}.${b64u(claims)}.${SECRET_MARK}`;
const NOW = 1_800_000_000_000;
const claims = (over = {}) => ({ sub: 'alice', iss: 'phantom-cache-auth', aud: 'phantom-cache-api', exp: NOW / 1000 + 600, iat: NOW / 1000 - 5, tenant_id: 'tenant-a', roles: ['cache:read', 'telemetry:read'], ...over });

const memStorage = () => {
  const m = new Map();
  return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: (k) => m.delete(k), _m: m };
};

// ------------------------------------------------------------------------------------------------ decoding

test('normalizeTokenInput: one JWT-shaped string, "Bearer " dropped, everything else refused with a reason and no echo', () => {
  const t = jwt(claims());
  assert.deepEqual(normalizeTokenInput(`  Bearer ${t}\n`), { ok: true, token: t });
  assert.equal(normalizeTokenInput('').reason, 'empty');
  assert.equal(normalizeTokenInput('abc').reason, 'not_jwt_shape');
  assert.equal(normalizeTokenInput(`${t} ${t}`).reason, 'whitespace');
  assert.equal(normalizeTokenInput('a.b.c'.padEnd(9000, 'c')).reason, 'too_long');
  assert.equal(normalizeTokenInput(null).reason, 'empty');
});

test('decodeClaims reads the gateway claim names (sub, iss, aud, exp, iat, tenant_id, roles) and never returns the token', () => {
  const t = jwt(claims());
  const d = decodeClaims(t);
  assert.equal(d.ok, true);
  assert.deepEqual(d.claims, {
    subject: 'alice', tenant: 'tenant-a', roles: ['cache:read', 'telemetry:read'], issuer: 'phantom-cache-auth', audience: 'phantom-cache-api',
    expiresAtSec: NOW / 1000 + 600, issuedAtSec: NOW / 1000 - 5, algorithm: 'HS256',
  });
  assert.doesNotMatch(JSON.stringify(d), new RegExp(SECRET_MARK), 'the signature part is not in the result');
  assert.doesNotMatch(JSON.stringify(d), new RegExp(t.split('.')[1]), 'the payload part is not in the result');
});

test('decodeClaims: unreadable input gives a reason, hostile values are clipped and stripped of control characters', () => {
  for (const bad of ['', 'x', 'a.b.c', `${b64u({ alg: 'HS256' })}.${Buffer.from('not json').toString('base64url')}.sig`, 7, undefined]) assert.equal(decodeClaims(bad).ok, false);
  const long = 'x'.repeat(500);
  const d = decodeClaims(jwt(claims({ sub: `a\u0000b${long}`, roles: ['r'.repeat(100), 5, ...Array(30).fill('z')], aud: ['one', 2, 'two'] })));
  assert.equal(d.ok, true);
  assert.ok(d.claims.subject.length <= 80 && !/[\u0000-\u001f]/.test(d.claims.subject));
  assert.equal(d.claims.roles[0].length, 40);
  assert.equal(d.claims.roles.length, 12);
  assert.equal(d.claims.audience, 'one, two');
  assert.equal(decodeClaims(jwt(claims({ exp: 'soon', tenant_id: 5 }))).claims.expiresAtSec, null);
  assert.equal(decodeClaims(jwt(claims({ tenant_id: 5 }))).claims.tenant, null);
});

test('expiry: valid with a countdown, expired with "ago", unknown when the claim is absent; text says the page clock only hints', () => {
  const c = decodeClaims(jwt(claims())).claims;
  assert.deepEqual(expiryState(c, NOW), { state: 'valid', secondsLeft: 600 });
  assert.equal(expiryText(expiryState(c, NOW)), 'Expires in 10 min 0 s (by this page\'s clock).');
  const past = expiryState(c, NOW + 700_000);
  assert.equal(past.state, 'expired');
  assert.match(expiryText(past), /Expired 1 min 40 s ago .*The gateway will refuse it\./);
  assert.equal(expiryState({ expiresAtSec: null }, NOW).state, 'unknown');
  assert.equal(expiryState(null, NOW).state, 'unknown');
  assert.equal(formatSpan(7500), 'in 2 h 5 min');
  assert.equal(formatSpan(-5), '5 s ago');
});

test('roleHints reflects the gateway role aliases (jwt.rs has_role); admin covers all', () => {
  assert.deepEqual(roleHints({ roles: ['telemetry:read'] }), { observe: true, cacheRead: false, invalidate: false, operate: false });
  assert.deepEqual(roleHints({ roles: ['requester'] }), { observe: false, cacheRead: true, invalidate: true, operate: false });
  assert.deepEqual(roleHints({ roles: ['ADMIN'] }), { observe: true, cacheRead: true, invalidate: true, operate: true });
  assert.equal(roleHints(null), null);
});

// ------------------------------------------------------------------------------------------------ plain-words answers

test('describeAnswer: each gateway answer has its own words, and none of them contains a token', () => {
  const cases = [
    [{ status: 401, authError: 'MissingToken', hadToken: false }, 'requires_token', /sign in/i],
    [{ status: 401, hadToken: false }, 'requires_token', /sign in/i],
    [{ status: 401, authError: 'ExpiredToken', hadToken: true }, 'expired', /expired/i],
    [{ status: 401, authError: 'InvalidIssuer', hadToken: true }, 'wrong_issuer', /issuer/i],
    [{ status: 401, authError: 'InvalidAudience', hadToken: true }, 'wrong_audience', /audience/i],
    [{ status: 401, authError: 'InvalidToken', hadToken: true }, 'rejected', /did not accept/i],
    [{ status: 403, authError: 'InsufficientPermissions', hadToken: true }, 'forbidden', /lacks the role/i],
    [{ status: 403, authError: 'MissingTenant', hadToken: true }, 'no_tenant', /tenant/i],
    [{ status: 200, hadToken: true }, 'ok', /accepted/i],
    [{ status: 200, hadToken: false }, 'ok', /without a token/i],
    [{ status: 0, hadToken: true }, 'no_answer', /no answer/i],
    [{ status: 404 }, 'missing', /no such endpoint/i],
    [{ status: 503 }, 'server', /error/i],
  ];
  for (const [input, key, re] of cases) {
    const a = describeAnswer(input);
    assert.equal(a.key, key, JSON.stringify(input));
    assert.match(`${a.title} ${a.text}`, re, JSON.stringify(input));
    assert.equal(a.needsAction, ['requires_token', 'expired', 'wrong_issuer', 'wrong_audience', 'rejected', 'forbidden', 'no_tenant'].includes(key));
  }
  // 401 with no token is "requires a token"; 401 WITH a token and no detail is "not accepted", not "requires"
  assert.equal(describeAnswer({ status: 401, hadToken: true }).key, 'rejected');
});

test('chipView: the header chip says what is true, in this order of precedence', () => {
  const decoded = decodeClaims(jwt(claims()));
  const valid = expiryState(decoded.claims, NOW);
  const expired = expiryState(decoded.claims, NOW + 9e6);
  assert.equal(chipView({ hasToken: false, requiresToken: true }).text, 'Sign in');
  assert.equal(chipView({ hasToken: false, answer: { key: 'ok' } }).text, 'No token (not required)');
  assert.equal(chipView({ hasToken: false }).text, 'No token');
  assert.equal(chipView({ hasToken: true, decoded, exp: valid, answer: { key: 'ok' } }).text, 'Signed in as tenant-a');
  assert.equal(chipView({ hasToken: true, decoded, exp: expired, answer: null }).text, 'Token expired');
  assert.equal(chipView({ hasToken: true, decoded, exp: valid, answer: { key: 'expired' } }).text, 'Token expired');
  assert.equal(chipView({ hasToken: true, decoded, exp: valid, answer: { key: 'rejected' } }).text, 'Token not accepted');
  assert.equal(chipView({ hasToken: true, decoded, exp: valid, answer: { key: 'forbidden' } }).text, 'Token lacks a role');
  assert.equal(chipView({ hasToken: true, decoded: { ok: false }, exp: valid, answer: null }).text, 'Signed in');
});

// ------------------------------------------------------------------------------------------------ the holder

test('token holder: bound to the gateway origin, memory by default, nothing written to storage', () => {
  const st = memStorage();
  const s = createTokenSession({ getStorage: () => st });
  const t = jwt(claims());
  assert.equal(s.set('http://127.0.0.1:18210', t), true);
  assert.equal(s.tokenFor('http://127.0.0.1:18210'), t);
  assert.equal(s.tokenFor('http://127.0.0.1:18211'), null, 'a token is never offered for another gateway');
  assert.equal(st._m.size, 0, 'storage untouched without keep');
  assert.equal(s.isKept('http://127.0.0.1:18210'), false);
  assert.equal(s.forget('http://127.0.0.1:18210'), true);
  assert.equal(s.tokenFor('http://127.0.0.1:18210'), null);
});

test('token holder: keep writes ONE tab-storage entry, a new holder (reload) restores it, forget removes it', () => {
  const st = memStorage();
  const t = jwt(claims());
  const a = createTokenSession({ getStorage: () => st });
  a.set('http://localhost:1', t, { keep: true });
  assert.deepEqual([...st._m.keys()], [TAB_KEY]);
  const b = createTokenSession({ getStorage: () => st });
  assert.equal(b.tokenFor('http://localhost:1'), t);
  assert.equal(b.isKept('http://localhost:1'), true);
  b.forget('http://localhost:1');
  assert.equal(st._m.size, 0);
  // kept and not-kept entries: only the kept one is stored
  a.set('http://localhost:2', jwt(claims({ sub: 'bob' })), { keep: false });
  assert.deepEqual(Object.keys(JSON.parse(st._m.get(TAB_KEY) || '{}')), ['http://localhost:1'].filter(() => st._m.has(TAB_KEY)));
});

test('token holder: the snapshot carries a counter and a flag, never a token; subscribers are notified; broken storage does not break it', () => {
  const s = createTokenSession({ getStorage: () => { throw new Error('denied'); } });
  let n = 0;
  const off = s.subscribe(() => { n += 1; });
  const t = jwt(claims());
  s.set('http://localhost:1', t, { keep: true });
  assert.equal(s.tokenFor('http://localhost:1'), t, 'memory works when storage throws');
  assert.equal(s.getSnapshot().storageProblem, true);
  assert.equal(n, 1);
  assert.doesNotMatch(JSON.stringify(s.getSnapshot()), new RegExp(SECRET_MARK));
  assert.equal(s.getSnapshot(), s.getSnapshot(), 'stable between changes (useSyncExternalStore)');
  off();
  s.forget('http://localhost:1');
  assert.equal(n, 1);
  assert.equal(s.set('', t), false);
  assert.equal(s.set('http://x', ''), false);
});

test('token holder: every change publishes a NEW snapshot with a higher version (the sign-in hook re-renders on it)', () => {
  const s = createTokenSession({ getStorage: () => memStorage() });
  const v0 = s.getSnapshot();
  s.set('http://localhost:1', jwt(claims()));
  const v1 = s.getSnapshot();
  assert.notEqual(v1, v0, 'a new object, or useSyncExternalStore never sees the change');
  assert.ok(v1.version > v0.version);
  s.forget('http://localhost:1');
  const v2 = s.getSnapshot();
  assert.notEqual(v2, v1);
  assert.ok(v2.version > v1.version);
  assert.equal(s.forget('http://localhost:1'), false, 'forgetting nothing changes nothing');
  assert.equal(s.getSnapshot(), v2);
});

test('token holder: a damaged stored value is ignored; the number of origins is bounded', () => {
  const st = memStorage();
  st.setItem(TAB_KEY, '{not json');
  const s = createTokenSession({ getStorage: () => st });
  assert.equal(s.tokenFor('http://localhost:1'), null);
  for (let i = 0; i < 12; i += 1) s.set(`http://localhost:${i}`, jwt(claims({ sub: `u${i}` })), { keep: true });
  assert.ok(Object.keys(JSON.parse(st._m.get(TAB_KEY))).length <= 8);
});

// ------------------------------------------------------------------------------------------------ request client

function fakeFetch(handler) {
  const calls = [];
  const f = async (url, init) => {
    calls.push({ url, init });
    return handler(url, init);
  };
  f.calls = calls;
  return f;
}
const res = ({ status = 200, headers = {}, body = '{}' } = {}) => ({
  ok: status >= 200 && status < 300,
  status,
  headers: { get: (k) => headers[k.toLowerCase()] ?? null },
  text: async () => body,
  body: null,
});

test('client: Authorization is sent only when a token is set, only as a header, never in the URL, and the result never holds it', async () => {
  const t = jwt(claims());
  const f = fakeFetch(() => res({ body: '{"status":"ok"}' }));
  const withTok = createLabClient({ baseUrl: 'http://127.0.0.1:18210', getToken: () => t, fetchImpl: f });
  const r = await withTok.getJson('/api/v1/intelligence/status');
  assert.equal(r.ok, true);
  assert.deepEqual(r.data, { status: 'ok' });
  assert.equal(f.calls[0].init.headers.Authorization, `Bearer ${t}`);
  assert.doesNotMatch(f.calls[0].url, new RegExp(SECRET_MARK));
  assert.equal(f.calls[0].init.credentials, 'omit');
  assert.doesNotMatch(JSON.stringify(r), new RegExp(SECRET_MARK));
  const f2 = fakeFetch(() => res());
  const none = createLabClient({ baseUrl: 'http://127.0.0.1:18210', getToken: () => null, fetchImpl: f2 });
  await none.getJson('/api/v1/metrics?schema=2');
  assert.equal('Authorization' in f2.calls[0].init.headers, false, 'no token, no header');
});

test('client: 401 and 403 are reported with the status and the X-Auth-Error word; a network failure is status 0; abort is flagged', async () => {
  const mk = (r) => createLabClient({ baseUrl: 'http://h:1', getToken: () => null, fetchImpl: fakeFetch(r) });
  const a = await mk(() => res({ status: 401, headers: { 'x-auth-error': 'ExpiredToken' }, body: 'Authentication failed' })).getJson('/api/v1/metrics');
  assert.deepEqual([a.ok, a.status, a.authError, a.data], [false, 401, 'ExpiredToken', null]);
  const b = await mk(() => res({ status: 403, headers: { 'x-auth-error': 'InsufficientPermissions' } })).getJson('/api/v1/metrics');
  assert.deepEqual([b.status, b.authError], [403, 'InsufficientPermissions']);
  const c = await mk(() => { throw new TypeError('Failed to fetch'); }).getJson('/api/v1/metrics');
  assert.deepEqual([c.ok, c.status, c.aborted], [false, 0, false]);
  const d = await mk(() => { const e = new Error('x'); e.name = 'AbortError'; throw e; }).getJson('/api/v1/metrics');
  assert.equal(d.aborted, true);
});

test('client: only /api/ paths without scheme, dot segments, double slashes or control characters can be requested', async () => {
  for (const ok of ['/api/events?schema=2', '/api/v1/intelligence/status', '/api/v1/intelligence/audit?limit=50&offset=0', '/api/v1/metrics?schema=2']) assert.equal(isGatewayPath(ok), true, ok);
  for (const bad of ['http://evil/api/x', '//evil/api/x', '/other', '/api/../etc', '/api/./x', '/api//x', '/api/x y', '/api/x\n', '/api/x#frag', '', null, 7, '/api/x?token=abc def']) assert.equal(isGatewayPath(bad), false, String(bad));
  const f = fakeFetch(() => res());
  const cl = createLabClient({ baseUrl: 'http://h:1', getToken: () => 'x.y.z', fetchImpl: f });
  const r = await cl.getJson('http://evil/api/x');
  assert.equal(r.bad, true);
  assert.equal(f.calls.length, 0, 'nothing is sent for a refused path');
});

// ------------------------------------------------------------------------------------------------ telemetry stream reader

function streamBody(chunks, { failAt = null } = {}) {
  let i = 0;
  return {
    getReader: () => ({
      read: async () => {
        if (failAt !== null && i === failAt) throw new TypeError('network error');
        return i < chunks.length ? { done: false, value: new TextEncoder().encode(chunks[i++]) } : { done: true, value: undefined };
      },
      releaseLock: () => {},
    }),
  };
}
const streamClient = (r) => ({ openStream: async () => r });

test('telemetry stream: events are delivered whole even when a chunk splits a frame; heartbeats are ignored; the end is reported once', async () => {
  const seen = [];
  const ends = [];
  let opened = 0;
  await runTelemetryStream({
    client: streamClient({ ok: true, status: 200, body: streamBody([':heartbeat\n\n', 'data: {"schema":2,"a"', ':1}\n\ndata: {"b":2}\n\n']) }),
    signal: new AbortController().signal,
    onOpen: () => { opened += 1; },
    onText: (t) => seen.push(t),
    onEnd: (e) => ends.push(e),
  });
  assert.equal(opened, 1);
  assert.deepEqual(seen, ['{"schema":2,"a":1}', '{"b":2}']);
  assert.deepEqual(ends, [{ status: 200, authError: null, ended: true }]);
});

test('telemetry stream: a refusal is reported with its status and word and never opens; 401 and 403 are not retried on a timer', async () => {
  const ends = [];
  let opened = 0;
  await runTelemetryStream({ client: streamClient({ ok: false, status: 401, authError: 'MissingToken' }), signal: new AbortController().signal, onOpen: () => { opened += 1; }, onText: () => {}, onEnd: (e) => ends.push(e) });
  assert.equal(opened, 0);
  assert.deepEqual(ends, [{ status: 401, authError: 'MissingToken', ended: false }]);
  assert.equal(isAuthRefusal(401) && isAuthRefusal(403), true);
  assert.equal(isAuthRefusal(500) || isAuthRefusal(0) || isAuthRefusal(200), false);
});

test('telemetry stream: a broken connection is status 0 / ended; an abort reports nothing at all', async () => {
  const ends = [];
  await runTelemetryStream({ client: streamClient({ ok: true, status: 200, body: streamBody(['data: {}\n\n'], { failAt: 1 }) }), signal: new AbortController().signal, onOpen: () => {}, onText: () => {}, onEnd: (e) => ends.push(e) });
  assert.deepEqual(ends, [{ status: 0, authError: null, ended: true }]);
  const ctl = new AbortController();
  const ends2 = [];
  ctl.abort();
  await runTelemetryStream({ client: streamClient({ ok: true, status: 200, body: streamBody(['data: {}\n\n']) }), signal: ctl.signal, onOpen: () => {}, onText: () => {}, onEnd: (e) => ends2.push(e) });
  assert.deepEqual(ends2, []);
});

// ------------------------------------------------------------------------------------------------ window hand-off

test('hand-off rules: only the opener, exact shape, this gateway, a JWT-shaped token; loopback or same origin applies, anything else asks first', () => {
  const t = jwt(claims());
  const gw = 'http://127.0.0.1:18210';
  const good = { type: HANDOFF_TYPE, v: 1, gateway: gw, token: t };
  const base = { origin: 'http://127.0.0.1:3000', sourceIsOpener: true, data: good, gatewayUrl: gw, pageOrigin: 'http://localhost:3202' };
  assert.deepEqual(evaluateHandoff(base), { action: 'apply', token: t, from: 'http://127.0.0.1:3000' });
  assert.equal(evaluateHandoff({ ...base, origin: 'http://localhost:3202' }).action, 'apply');
  assert.equal(evaluateHandoff({ ...base, origin: 'https://demo.example' }).action, 'confirm');
  assert.equal(evaluateHandoff({ ...base, sourceIsOpener: false }).reason, 'not_from_opener');
  assert.equal(evaluateHandoff({ ...base, data: { ...good, gateway: 'http://127.0.0.1:18211' } }).reason, 'other_gateway');
  assert.equal(evaluateHandoff({ ...base, data: { ...good, extra: 1 } }).reason, 'bad_shape');
  assert.equal(evaluateHandoff({ ...base, data: { ...good, v: 2 } }).reason, 'bad_shape');
  assert.equal(evaluateHandoff({ ...base, data: { ...good, token: 'nope' } }).reason, 'bad_token');
  assert.equal(evaluateHandoff({ ...base, data: { type: 'other' } }).reason, 'not_a_handoff');
  assert.equal(evaluateHandoff({ ...base, data: 'string' }).reason, 'not_a_handoff');
  assert.equal(evaluateHandoff({ ...base, data: null }).action, 'ignore');
  assert.equal(isLoopbackOrigin('http://[::1]:3000'), true);
  assert.equal(isLoopbackOrigin('https://localhost.evil.example'), false);
  assert.equal(isLoopbackOrigin('not a url'), false);
  assert.doesNotMatch(JSON.stringify(evaluateHandoff({ ...base, data: { ...good, gateway: 'x' } })), new RegExp(SECRET_MARK), 'a refusal does not echo the token');
});

// ------------------------------------------------------------------------------------------------ source guards

const NEW_FILES = ['lib/gatewayAuth.js', 'lib/tokenSession.js', 'lib/telemetryStream.js', 'hooks/useGatewayAuth.js', 'screens/authUi.jsx'];

test('guard: the sign-in modules never log, never use localStorage, cookies, IndexedDB, location or history, never inject markup, never build a URL with a token', () => {
  for (const f of NEW_FILES) {
    const c = code(f);
    assert.doesNotMatch(c, /console\./, f);
    assert.doesNotMatch(c, /localStorage|indexedDB|document\.cookie|history\.(push|replace)State/, f);
    assert.doesNotMatch(c, /dangerouslySetInnerHTML|innerHTML|eval\(|new Function/, f);
    assert.doesNotMatch(c, /[?&]token=|access_token|\?auth=|searchParams\.set\(['"](token|auth)/i, f);
    assert.doesNotMatch(c, /\bfetch\s*\(|EventSource|XMLHttpRequest|WebSocket/, `${f}: network access stays in lib/labApi.js (and the one EventSource in useTelemetry)`);
  }
  // sessionStorage only in the holder
  for (const f of NEW_FILES.filter((x) => x !== 'lib/tokenSession.js')) assert.doesNotMatch(code(f), /sessionStorage/, f);
  assert.match(code('lib/tokenSession.js'), /sessionStorage/);
});

test('guard: the token string is never rendered, put in an attribute or kept in React state/context by the sign-in UI', () => {
  const ui = code('screens/authUi.jsx');
  assert.doesNotMatch(ui, /tokenFor|\.token\b(?!s)/, 'the UI has no way to read the token');
  const hook = code('hooks/useGatewayAuth.js');
  // tokenFor is called to build the client / decide hasToken / decode, never stored in state or returned
  assert.doesNotMatch(hook, /useState\([^)]*tokenFor|set[A-Z]\w*\(\s*session\.tokenFor/);
  assert.doesNotMatch(hook.replace(/pendingToken\.current/g, ''), /return\s*\{[^}]*\btoken\b\s*[,:}]/);
  assert.doesNotMatch(read('lib/gatewayAuth.js').replace(/\/\/.*$/gm, ''), /return\s*\{[^}]*\btoken: n\.token[^}]*claims/);
});

test('guard: useTelemetry reads the stream through the authenticated client when a token is set, and a refusal stops the timer', () => {
  const src = read('hooks/useTelemetry.js');
  assert.match(src, /if \(hasToken\) \{/);
  assert.match(src, /runTelemetryStream\(/);
  assert.match(src, /if \(isAuthRefusal\(status\)\) return;/);
  assert.match(src, /\}, \[gatewayUrl, nonce, hasToken, feedVersion\]\);/, 'a new or forgotten token restarts the stream');
  // the EventSource path remains the only EventSource constructor
  assert.equal((src.match(/new EventSource\(/g) || []).length, 1);
});

test('guard: App passes the sign-in object to the telemetry hook and the screens, renders the banner and the header chip', () => {
  const app = read('App.jsx');
  assert.match(app, /useTelemetry\(gw\.url, auth\.feed\)/);
  assert.match(app, /<AuthBanner auth=\{auth\}/);
  assert.match(app, /<IdentityControl slot=\{slot\} auth=\{auth\}/);
  assert.match(app, /reportLabLink, auth, intel, intelLive, hub \}/);
  assert.match(app, /describeTelemetryLink\(state, now, \{ probe: auth\.linkProbe \}\)/);
});

// ------------------------------------------------------------------------------------------------ rendered surfaces

let H;
before(async () => {
  H = await getHarness();
});

function mkAuth(over = {}) {
  const t = jwt(claims());
  const decoded = decodeClaims(t);
  return {
    hasToken: true,
    decoded,
    exp: expiryState(decoded.claims, NOW),
    answer: null,
    requiresToken: false,
    chip: { tone: 'info', icon: 'info', text: 'Tenant tenant-a' },
    problem: null,
    keptForTab: false,
    storageProblem: false,
    handoffPending: null,
    accept: () => true,
    forget: () => {},
    confirmHandoff: () => {},
    dismissHandoff: () => {},
    clearProblem: () => {},
    ...over,
  };
}
const GW = 'http://127.0.0.1:18210';
const html = (el) => H.renderLab(el);

test('render: a secure gateway with no token shows "this gateway requires a token" with the form, not a broken screen', () => {
  const a = describeAnswer({ status: 401, authError: 'MissingToken', hadToken: false });
  const out = html(H.React.createElement(H.Auth.AuthBanner, { auth: mkAuth({ hasToken: false, decoded: null, requiresToken: true, answer: { ...a, status: 401 } }), gatewayUrl: GW }));
  assert.match(out, /data-auth-key="requires_token"/);
  assert.match(out, /Sign in to see this gateway/);
  assert.match(out, /HTTP 401/);
  assert.match(out, /type="password"/);
  assert.match(out, />Sign in</);
  assert.match(out, /Open as another user or on another gateway/);
  assert.deepEqual(lintHtml(out, { requireH1: false }), []);
});

test('render: 401 expired, 401 rejected, 403 each get their own title and name the decoded identity as NOT verified', () => {
  for (const [status, authError, title] of [[401, 'ExpiredToken', /The token has expired/], [401, 'InvalidToken', /did not accept the token/], [403, 'InsufficientPermissions', /lacks the role/]]) {
    const a = describeAnswer({ status, authError, hadToken: true });
    const out = html(H.React.createElement(H.Auth.AuthBanner, { auth: mkAuth({ answer: { ...a, status } }), gatewayUrl: GW }));
    assert.match(out, title);
    assert.match(out, new RegExp(`HTTP ${status}`));
    assert.match(out, /Decoded by this page, not verified; the gateway verifies the token\./);
    assert.match(out, /data-claim="tenant">tenant-a</);
    assert.deepEqual(lintHtml(out, { requireH1: false }), [], `${status} ${authError}`);
  }
});

test('render: nothing is shown when there is nothing to say (healthy and anonymous-dev gateways)', () => {
  const ok = describeAnswer({ status: 200, hadToken: true });
  assert.equal(html(H.React.createElement(H.Auth.AuthBanner, { auth: mkAuth({ answer: { ...ok, status: 200 } }), gatewayUrl: GW })), '');
  assert.equal(html(H.React.createElement(H.Auth.AuthBanner, { auth: mkAuth({ hasToken: false, decoded: null }), gatewayUrl: GW })), '');
});

test('render: identity shows tenant, subject, roles, issuer, audience and the expiry countdown; an expired token says so', () => {
  const out = html(H.React.createElement(H.Auth.IdentityDetails, { auth: mkAuth() }));
  for (const re of [/data-claim="tenant">tenant-a</, /data-claim="subject">alice</, /cache:read, telemetry:read/, /phantom-cache-auth/, /phantom-cache-api/, /Expires in 10 min 0 s/, /telemetry yes, cache reads yes, purges no, operating no/]) assert.match(out, re);
  const dead = mkAuth();
  dead.exp = expiryState(dead.decoded.claims, NOW + 1e7);
  const out2 = html(H.React.createElement(H.Auth.IdentityDetails, { auth: dead }));
  assert.match(out2, /data-expiry-state="expired"/);
  assert.match(out2, /Expired .* ago/);
  assert.doesNotMatch(out2, /data-expired-but-streaming/, 'only when the gateway has answered OK on an open stream');
  const streaming = mkAuth({ answer: { ...describeAnswer({ status: 200, hadToken: true }), status: 200 } });
  streaming.exp = dead.exp;
  assert.match(html(H.React.createElement(H.Auth.IdentityDetails, { auth: streaming })), /data-expired-but-streaming/);
  assert.equal(html(H.React.createElement(H.Auth.IdentityDetails, { auth: mkAuth({ hasToken: false, decoded: null }) })), '');
  assert.match(html(H.React.createElement(H.Auth.IdentityDetails, { auth: mkAuth({ decoded: { ok: false, reason: 'unreadable' } }) })), /cannot read its claims/);
});

test('render: the token string, its payload and its signature appear nowhere in any sign-in markup', () => {
  const t = jwt(claims());
  const a = describeAnswer({ status: 401, authError: 'InvalidToken', hadToken: true });
  const auth = mkAuth({ answer: { ...a, status: 401 }, problem: 'That does not look like a token' });
  const out = [
    html(H.React.createElement(H.Auth.AuthBanner, { auth, gatewayUrl: GW })),
    html(H.React.createElement(H.Auth.IdentityDetails, { auth })),
    html(H.React.createElement(H.Auth.TokenForm, { auth, gatewayUrl: GW })),
  ].join('\n');
  for (const part of t.split('.')) assert.equal(out.includes(part), false, 'a token part leaked into the markup');
  assert.match(out, /data-token-problem/);
});

test('render: a hand-off from a non-loopback window is shown with its origin and waits for a click; the token is not in the markup', () => {
  const out = html(H.React.createElement(H.Auth.AuthBanner, { auth: mkAuth({ hasToken: false, decoded: null, handoffPending: { from: 'https://demo.example' } }), gatewayUrl: GW }));
  assert.match(out, /data-handoff-pending/);
  assert.match(out, /https:\/\/demo\.example/);
  assert.match(out, /Use the offered token/);
  assert.match(out, /Ignore it/);
  assert.deepEqual(lintHtml(out, { requireH1: false }), []);
});

test('render: the keep-for-this-tab choice is a labelled checkbox, off by default, disabled with an explanation when storage is blocked', () => {
  const out = html(H.React.createElement(H.Auth.TokenForm, { auth: mkAuth(), gatewayUrl: GW }));
  assert.match(out, /<input type="checkbox"(?![^>]*checked)/);
  assert.match(out, /Keep for this tab/);
  const blocked = html(H.React.createElement(H.Auth.TokenForm, { auth: mkAuth({ storageProblem: true }), gatewayUrl: GW }));
  assert.match(blocked, /type="checkbox"[^>]*disabled/);
  assert.match(blocked, /blocks tab storage/);
});

test('the connection panel footnote stays true: memory by default, tab storage only when ticked', () => {
  const link = read('screens/linkUi.jsx');
  assert.match(link, /Bearer tokens are kept in this page's memory only and are never saved, unless you tick Keep for this tab/);
});

// ------------------------------------------------------------------------------------------------ link words (lib/linkState.js seam)

test('link: a classified 401/403 answer makes the telemetry link Unauthorised from the GATEWAY, with no retry countdown; with a token the text does not say "no token"', async () => {
  const { describeTelemetryLink } = await import('../src/lib/linkState.js');
  const { connectionReducer, initialConnection } = await import('../src/lib/connection.js');
  let st = initialConnection;
  st = connectionReducer(st, { type: 'error', closed: true, now: 1000 });
  st = connectionReducer(st, { type: 'retryScheduled', at: 3000 });
  const noToken = describeTelemetryLink(st, 1500, { probe: { kind: 'unauthorized_missing', withToken: false } });
  assert.equal(noToken.word, 'Unauthorised');
  assert.equal(noToken.who, 'gateway');
  assert.equal(noToken.nextRetryInMs, null);
  assert.equal(noToken.retryOwner, null, 'the page does not claim a timer it will not run');
  assert.match(noToken.text, /no token to send/);
  const withTok = describeTelemetryLink(st, 1500, { probe: { kind: 'unauthorized_expired', withToken: true } });
  assert.equal(withTok.word, 'Unauthorised');
  assert.doesNotMatch(withTok.text, /no token to send/);
  assert.equal(describeTelemetryLink(st, 1500).word, 'Reconnecting', 'without an answer from the gateway the old rule holds: telemetry alone never says Unauthorised');
});
