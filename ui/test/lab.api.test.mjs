import test from 'node:test';
import assert from 'node:assert/strict';
import { ARTIFACT_PATTERN, ID_PATTERN, createLabClient, isTerminalError, labUrl, mapLabError } from '../src/lib/labApi.js';
import { FX, fakeResponse, fakeStream } from './helpers/labFixtures.mjs';

const E = FX.errors();
const BASE = 'http://127.0.0.1:18085';

function recorder(respond) {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, init });
    return respond(url, init);
  };
  return { calls, fetchImpl };
}

function map(key, hadToken = false) {
  const e = E[key];
  let json = null;
  try {
    json = e.body ? JSON.parse(e.body) : null;
  } catch {
    json = null;
  }
  return mapLabError({ status: e.status, headers: e.headers, bodyJson: json, bodyText: e.body, hadToken });
}

test('error mapping: every recorded gateway failure maps to its own kind with words for the person', () => {
  assert.equal(map('lab_off_404').kind, 'disabled');
  assert.match(map('lab_off_404').fix, /PHANTOM_LAB_ENABLED=true/);
  assert.equal(map('unauthorized_missing').kind, 'unauthorized_missing');
  assert.equal(map('unauthorized_expired', true).kind, 'unauthorized_expired');
  assert.equal(map('forbidden_operator_only_read', true).kind, 'forbidden_role');
  assert.match(map('forbidden_operator_only_read', true).fix, /Observer role/);
  assert.equal(map('forbidden_observer_post', true).kind, 'forbidden_role');
  assert.equal(map('run_active_409').kind, 'run_active');
  assert.equal(map('bad_origin_403').kind, 'forbidden_origin');
  assert.equal(map('bad_host_421').kind, 'host_guard');
  assert.equal(map('sealed_422').kind, 'invalid_spec');
  assert.match(map('sealed_422').serverMessage, /sealed seed set S is refused/);
  assert.equal(map('reps_422').kind, 'invalid_spec');
  assert.equal(map('not_found_404').kind, 'not_found');
  assert.equal(map('limit_400').kind, 'bad_request');
  assert.equal(map('unsupported_media_415').kind, 'bad_request');
});

test('error mapping: missing token and insufficient role are different messages; a lab-1 404 is not "disabled"', () => {
  const missing = map('unauthorized_missing');
  const role = map('forbidden_operator_only_read', true);
  assert.notEqual(missing.title, role.title);
  assert.notEqual(missing.kind, role.kind);
  assert.notEqual(map('not_found_404').kind, 'disabled');
  // 401 without the header: decided by whether a token was sent
  assert.equal(mapLabError({ status: 401, headers: {}, bodyJson: null, bodyText: '', hadToken: false }).kind, 'unauthorized_missing');
  assert.equal(mapLabError({ status: 401, headers: {}, bodyJson: null, bodyText: '', hadToken: true }).kind, 'unauthorized_invalid');
  assert.equal(mapLabError({ status: 500, headers: {}, bodyJson: null, bodyText: 'x'.repeat(5000) }).serverMessage.length <= 160, true, 'server text is clipped');
  assert.equal(mapLabError({ status: 429, headers: {}, bodyJson: null, bodyText: '' }).kind, 'busy');
  assert.equal(mapLabError({ status: 413, headers: {}, bodyJson: null, bodyText: '' }).kind, 'too_large');
});

test('terminal error kinds are exactly those retrying cannot fix', () => {
  for (const k of ['disabled', 'unauthorized_missing', 'unauthorized_invalid', 'unauthorized_expired', 'forbidden_role', 'forbidden_origin', 'host_guard', 'not_found']) assert.equal(isTerminalError(k), true, k);
  for (const k of ['network', 'server', 'busy', 'run_active', 'invalid_spec']) assert.equal(isTerminalError(k), false, k);
});

test('requests carry the token only in the Authorization header, never in the URL, body or query', async () => {
  const secret = 'SECRET-TOKEN-VALUE-1234567890';
  const rec = recorder(async () => fakeResponse({ status: 200, body: JSON.stringify(FX.caps()) }));
  const client = createLabClient({ baseUrl: BASE, getToken: () => secret, fetchImpl: rec.fetchImpl });
  await client.capabilities();
  await client.evidenceIndex();
  await client.evidence('m6_sealed_2026-10-29T234845Z');
  await client.runs({ limit: 5, offset: 10 });
  await client.run('20261030T034025Z-a9e34238');
  await client.artifacts('20261030T034025Z-a9e34238');
  await client.startRun('t1_matrix', { n_seeds: 1 });
  await client.cancelRun('20261030T034025Z-a9e34238');
  assert.equal(rec.calls.length, 8);
  for (const c of rec.calls) {
    assert.ok(!c.url.includes(secret), `token leaked into URL ${c.url}`);
    assert.ok(!String(c.init.body ?? '').includes(secret), 'token leaked into body');
    assert.equal(c.init.headers.Authorization, `Bearer ${secret}`);
    assert.equal(c.init.credentials, 'omit');
    assert.equal(c.init.cache, 'no-store');
    assert.ok(c.url.startsWith(`${BASE}/api/v1/lab/`));
  }
  const posts = rec.calls.filter((c) => c.init.method === 'POST');
  assert.equal(posts.length, 2);
  for (const p of posts) assert.equal(p.init.headers['Content-Type'], 'application/json');
  assert.deepEqual(JSON.parse(posts[0].init.body), { kind: 't1_matrix', spec: { n_seeds: 1 } });
  assert.deepEqual(JSON.parse(posts[1].init.body), {}, 'cancel sends an empty JSON object (the API requires the JSON content type)');
  assert.ok(rec.calls[3].url.endsWith('/runs?limit=5&offset=10'));
});

test('without a token no Authorization header is sent (anonymous development mode)', async () => {
  const rec = recorder(async () => fakeResponse({ body: JSON.stringify(FX.caps()) }));
  const client = createLabClient({ baseUrl: BASE, getToken: () => null, fetchImpl: rec.fetchImpl });
  await client.capabilities();
  assert.equal('Authorization' in rec.calls[0].init.headers, false);
});

test('identifiers are validated before they reach a URL (no traversal, no extra segments, no query)', async () => {
  const rec = recorder(async () => fakeResponse({ body: '{}' }));
  const client = createLabClient({ baseUrl: BASE, fetchImpl: rec.fetchImpl });
  for (const bad of ['..', '.', '../x', 'a/b', 'a?b=1', 'a#b', '', ' ', 'a b', '%2e%2e', 'x'.repeat(200), '\u0000']) {
    assert.throws(() => client.run(bad), /invalid identifier/, JSON.stringify(bad));
    assert.throws(() => client.evidence(bad), /invalid identifier/);
  }
  assert.equal(rec.calls.length, 0, 'nothing was sent');
  assert.equal(ID_PATTERN.test('20261030T034025Z-a9e34238'), true);
  assert.equal(ID_PATTERN.test('scenario_b.20261030T003932Z'), true);
  assert.equal(ARTIFACT_PATTERN.test('summary.json'), true);
  assert.equal(ARTIFACT_PATTERN.test('.hidden'), false);
  assert.throws(() => labUrl(BASE, ['runs'], { limit: -1 }), /unsupported query/);
  assert.throws(() => labUrl(BASE, ['runs'], { 'a=b&c': 1 }), /unsupported query/);
  assert.throws(() => labUrl(BASE, ['runs'], { limit: 'x' }), /unsupported query/);
});

test('client: success returns the parsed lab-1 document; HTTP and network failures never throw', async () => {
  const ok = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: JSON.stringify(FX.runsEmpty()) }) });
  const r = await ok.runs();
  assert.equal(r.ok, true);
  assert.equal(r.data.schema, 'lab-1');

  const off = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ status: 404, body: '' }) });
  const o = await off.capabilities();
  assert.equal(o.ok, false);
  assert.equal(o.error.kind, 'disabled');

  const net = createLabClient({ baseUrl: BASE, fetchImpl: async () => { throw new TypeError('Failed to fetch'); } });
  const n = await net.capabilities();
  assert.equal(n.error.kind, 'network');
  assert.match(n.error.fix, /PHANTOM_CORS_ORIGINS/);

  const aborted = createLabClient({ baseUrl: BASE, fetchImpl: async () => { const e = new Error('x'); e.name = 'AbortError'; throw e; } });
  assert.equal((await aborted.capabilities()).error.kind, 'aborted');

  const html = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: '<html>proxy page</html>' }) });
  assert.equal((await html.capabilities()).error.kind, 'unexpected', 'a 200 that is not lab-1 JSON is not accepted');

  const wrongSchema = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: '{"schema":"lab-2"}' }) });
  assert.equal((await wrongSchema.capabilities()).error.kind, 'unexpected');
});

test('client: start/cancel responses recorded from the gateway are accepted', async () => {
  const cancelled = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ status: 202, body: E.cancel_first_202.body }) });
  const r = await cancelled.cancelRun('20261030T034142Z-a4de24b6');
  assert.equal(r.ok, true);
  assert.equal(r.data.state, 'cancelling');
  const conflict = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ status: 409, body: E.run_active_409.body }) });
  const c = await conflict.startRun('t1_matrix', {});
  assert.equal(c.error.kind, 'run_active');
  assert.equal(c.error.status, 409);
});

test('artifact download uses fetch with the Authorization header and returns bytes; failures map like any other call', async () => {
  const rec = recorder(async () => fakeResponse({ headers: { 'content-type': 'text/csv' }, body: 'a,b\n1,2\n' }));
  const client = createLabClient({ baseUrl: BASE, getToken: () => 'tok', fetchImpl: rec.fetchImpl });
  const r = await client.downloadArtifact('20261030T034025Z-a9e34238', 'cells.csv');
  assert.equal(r.ok, true);
  assert.equal(r.contentType, 'text/csv');
  assert.equal(rec.calls[0].init.headers.Authorization, 'Bearer tok');
  assert.ok(!rec.calls[0].url.includes('tok'));
  assert.ok(rec.calls[0].url.endsWith('/runs/20261030T034025Z-a9e34238/artifacts/cells.csv'));
  assert.equal((await client.downloadArtifact('x', '../manifest.json')).error.kind, 'bad_request', 'a hostile artifact name is refused before any request');
  assert.equal(rec.calls.length, 1);
  const denied = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ status: 403, headers: { 'x-auth-error': 'InsufficientPermissions' }, body: 'Forbidden' }) });
  assert.equal((await denied.downloadArtifact('abc', 'cells.csv')).error.kind, 'forbidden_role');

  const json = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: JSON.stringify(FX.sbSummary()) }) });
  const j = await json.artifactJson('20261030T034041Z-c78a73cf', 'summary.json');
  assert.equal(j.ok, true);
  assert.equal(j.data.tier, 'MEASURED_LOOPBACK');
  const notJson = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: 'not json' }) });
  assert.equal((await notJson.artifactJson('abc', 'summary.json')).error.kind, 'unexpected');
});

test('openEvents sends Last-Event-ID and Accept headers, returns the body stream, and maps refusals', async () => {
  const rec = recorder(async () => fakeResponse({ headers: { 'content-type': 'text/event-stream' }, stream: fakeStream(['id: 1\n\n']) }));
  const client = createLabClient({ baseUrl: BASE, getToken: () => 'tok', fetchImpl: rec.fetchImpl });
  const r = await client.openEvents('20261030T034025Z-a9e34238', { lastEventId: 7 });
  assert.equal(r.ok, true);
  assert.equal(rec.calls[0].init.headers['Last-Event-ID'], '7');
  assert.equal(rec.calls[0].init.headers.Accept, 'text/event-stream');
  assert.equal(rec.calls[0].init.headers.Authorization, 'Bearer tok');
  assert.ok(!rec.calls[0].url.includes('tok') && !rec.calls[0].url.includes('?') && !/last-event-id/i.test(rec.calls[0].url), 'no credential, query or resume id in the URL');
  await client.openEvents('20261030T034025Z-a9e34238');
  assert.equal('Last-Event-ID' in rec.calls[1].init.headers, false);
  const busy = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ status: 429, body: '{"schema":"lab-1","error":{"code":"too_many_streams","message":"x"}}' }) });
  assert.equal((await busy.openEvents('abc')).error.kind, 'busy');
  const noBody = createLabClient({ baseUrl: BASE, fetchImpl: async () => fakeResponse({ body: '' }) });
  assert.equal((await noBody.openEvents('abc')).error.kind, 'unexpected');
});

test('the client module never touches storage, cookies or location', async () => {
  const fs = await import('node:fs');
  const text = fs.readFileSync(new URL('../src/lib/labApi.js', import.meta.url), 'utf8');
  assert.doesNotMatch(text, /localStorage|sessionStorage|indexedDB|document\.cookie|location\.|history\./);
});
