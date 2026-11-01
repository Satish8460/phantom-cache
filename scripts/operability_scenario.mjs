#!/usr/bin/env node
// scripts/operability_scenario.mjs  (lane L5, scenario 3: OPERABILITY)
//
//   O1 configuration : typos and bad values fail fast (exit 2, one [CONFIG ERROR] line per problem), unknown PHANTOM_* warns,
//                      --check-config exit codes
//   O2 health        : /health, /health/live, /health/ready
//   O3 kill/restart  : the gateway is killed and restarted while a Node client keeps requesting; errors during the outage,
//                      recovery after the restart, no wrong bytes ever
//   O4 origin down   : cached content keeps serving, uncached keys get an honest 5xx, no crash, recovery without a restart
//   O5 abuse         : oversized paths/queries/headers, control characters, traversal, bad methods, large bodies -> 4xx or a closed
//                      connection, never 5xx, never a crash; the gateway still serves correct bytes afterwards
//
// Usage: node scripts/operability_scenario.mjs [--exe <gateway>] [--out-dir <dir>] [--origin-port 19230] [--port 18233]
// Exit 0 only if every assertion held.

import { argv, DEFAULT_EXE, defaultOut, httpReq, rawRequest, makeJson, makePngOfSize, sha256, sleep, spawnGateway, startOrigin, waitReady, metrics, recorder } from './lib/scenario_common.mjs';

const { opt } = argv();
const EXE = opt('exe', DEFAULT_EXE);
const OUT = opt('out-dir', defaultOut('operability'));
const OPORT = Number(opt('origin-port', 19230));
const GPORT = Number(opt('port', 18233));
const R = recorder('operability', OUT, { prefix: 'OPERABILITY' });
const { check, log } = R;
const origin = startOrigin({ port: OPORT });
const gateways = [];
let seq = 0;
const start = (env = {}, args = []) => { const g = spawnGateway({ exe: EXE, port: GPORT, originUrl: origin.url, outDir: OUT, name: `gw${++seq}`, env, args }); gateways.push(g); return g; };
const stripAnsi = (s) => s.replace(/\x1b\[[0-9;]*m/g, '');

async function runToExit(env, args = [], ms = 8000) {
  const g = start(env, args);
  const r = await Promise.race([g.exited, sleep(ms).then(() => null)]);
  const text = stripAnsi(g.logs());
  if (r === null) await g.kill();
  return { exit: r?.code ?? null, stayedUp: r === null, text, lines: text.split(/\r?\n/).filter((l) => l.includes('[CONFIG ERROR]')) };
}

const get = async (path, headers = {}) => {
  const r = await httpReq({ port: GPORT, path: `/api/v1/cache${path}`, headers, timeoutMs: 15000 });
  return r.ok ? { ok: true, status: r.status, cs: r.headers['x-cache-status'], src: r.headers['x-cache-source'], err: r.headers['x-cache-error'], sha: sha256(r.body), body: r.body, ms: r.ms } : { ok: false, error: r.error, ms: r.ms };
};

// ------------------------------------------------------------------ O1
async function o1() {
  let r = await runToExit({ PHANTOM_REQUIRE_AUTH: 'yes' });
  log(`O1 REQUIRE_AUTH=yes -> exit ${r.exit}; ${r.lines.length} [CONFIG ERROR] line(s): ${r.lines.join(' | ')}`);
  check(r.exit === 2 && r.lines.length === 1 && /PHANTOM_REQUIRE_AUTH/.test(r.lines[0]), 'O1: PHANTOM_REQUIRE_AUTH=yes -> exit 2, exactly one [CONFIG ERROR] line naming the key', `exit=${r.exit} lines=${r.lines.length}`);

  r = await runToExit({ PHANTOM_REQUIRE_AUTH: 'yes', PHANTOM_MAX_CACHE_BYTES: 'abc', PHANTOM_BIND_ADDR: 'nonsense', PHANTOM_MODE: 'prod' });
  log(`O1 four problems -> exit ${r.exit}; ${r.lines.length} line(s):\n  ${r.lines.join('\n  ')}`);
  const keys = ['PHANTOM_REQUIRE_AUTH', 'PHANTOM_MAX_CACHE_BYTES', 'PHANTOM_BIND_ADDR', 'PHANTOM_MODE'];
  check(r.exit === 2 && keys.every((k) => r.lines.some((l) => l.includes(k))), 'O1: four independent problems -> exit 2 and a [CONFIG ERROR] line for EACH key', `exit=${r.exit} lines=${r.lines.length}`);

  r = await runToExit({ PHANTOM_MAX_STALE_MS: '0' });
  check(r.exit === 2 && r.lines.some((l) => l.includes('PHANTOM_MAX_STALE_MS')), 'O1: PHANTOM_MAX_STALE_MS=0 (out of range) -> exit 2', `exit=${r.exit} ${r.lines.join(' | ')}`);

  r = await runToExit({ PHANTOM_ALLOW_INSECURE_HTTP: 'maybe' });
  check(r.exit === 2 && r.lines.length >= 1, 'O1: PHANTOM_ALLOW_INSECURE_HTTP=maybe (not a boolean) -> exit 2', `exit=${r.exit} ${r.lines.join(' | ')}`);

  r = await runToExit({ PHANTOM_REQUIRE_AUTH: 'true' });
  check(r.exit === 2 && r.lines.some((l) => l.includes('PHANTOM_JWT_SECRET') || l.includes('PHANTOM_REQUIRE_AUTH')), 'O1: PHANTOM_REQUIRE_AUTH=true without a JWT secret -> exit 2 (fails closed)', `exit=${r.exit} ${r.lines.join(' | ')}`);

  r = await runToExit({ PHANTOM_REQUIRE_AUTH: 'yes' }, ['--check-config']);
  check(r.exit === 2, 'O1: --check-config with a bad value -> exit 2, server not started', `exit=${r.exit}`);
  r = await runToExit({}, ['--check-config']);
  check(r.exit === 0 && !r.stayedUp, 'O1: --check-config with a valid configuration -> exit 0, process ends', `exit=${r.exit}`);

  // unknown variable: warning, but starts
  const g = start({ PHANTOM_REQUIRE_AUTHH: 'true', PHANTOM_TOTALLY_UNKNOWN: '1' });
  const up = await waitReady(GPORT, 10000);
  const warn = stripAnsi(g.logs());
  const warnLines = warn.split(/\r?\n/).filter((l) => /PHANTOM_REQUIRE_AUTHH|PHANTOM_TOTALLY_UNKNOWN/.test(l));
  log(`O1 unknown variables: started=${up !== null}; lines mentioning them: ${warnLines.join(' | ')}`);
  check(up !== null && warnLines.length >= 1, 'O1: unknown PHANTOM_* variables produce a warning naming them and the gateway still starts', `lines=${warnLines.length}`);
  const typoNote = /PHANTOM_REQUIRE_AUTHH/.test(warn);
  check(typoNote, 'O1: the typo PHANTOM_REQUIRE_AUTHH is named in the output', String(typoNote));
  await g.kill();
}

// ------------------------------------------------------------------ O2
async function o2() {
  const g = start();
  check((await waitReady(GPORT)) !== null, 'O2: gateway becomes ready');
  const h = await httpReq({ port: GPORT, path: '/health' });
  check(h.status === 200 && h.body.toString().trim() === 'OK', 'O2: /health -> 200 OK', `${h.status} ${h.body.toString().slice(0, 40)}`);
  const l = await httpReq({ port: GPORT, path: '/health/live' });
  check(l.status === 200 && JSON.parse(l.body).status === 'alive', 'O2: /health/live -> 200 {"status":"alive"}', l.body.toString().slice(0, 80));
  const rd = await httpReq({ port: GPORT, path: '/health/ready' });
  const rj = JSON.parse(rd.body);
  check(rd.status === 200 && rj.status === 'ready', 'O2: /health/ready -> 200 {"status":"ready",...}', rd.body.toString().slice(0, 200));
  log(`O2 ready body: ${rd.body.toString()}`);
  await g.kill();
  const after = await httpReq({ port: GPORT, path: '/health/ready', timeoutMs: 1500 });
  check(!after.ok, 'O2: after the process is killed the health endpoint is unreachable (probe would fail)', after.error);
}

// ------------------------------------------------------------------ O3
async function o3() {
  const N = 20, objs = [];
  for (let i = 0; i < N; i++) { const body = i % 4 === 3 ? makePngOfSize(40000 + i * 3000, 300 + i) : makeJson(300 + i, 3000); const type = i % 4 === 3 ? 'image/png' : 'application/json'; origin.set(`/o/k${i}`, { body, type, etag: `"k${i}"`, cacheControl: 'public, max-age=300' }); objs.push(sha256(body)); }
  let g = start();
  await waitReady(GPORT);
  const t0 = Date.now(); const rows = []; let stop = false;
  const client = (async () => {
    let i = 0;
    while (!stop) {
      const k = i++ % N; const t = Date.now() - t0;
      const r = await get(`/o/k${k}`);
      rows.push({ k, t, ok: r.ok && r.status === 200, status: r.ok ? r.status : 0, error: r.ok ? null : r.error, good: r.ok && r.status === 200 ? r.sha === objs[k] : null, cs: r.cs });
      await sleep(25);
    }
  })();
  await sleep(2000);
  const killedAt = Date.now() - t0; await g.kill();
  await sleep(2000);
  const restartAt = Date.now() - t0; g = start();
  const readyMs = await waitReady(GPORT);
  const readyAt = Date.now() - t0;
  await sleep(2500); stop = true; await client;
  const pre = rows.filter((r) => r.t < killedAt), down = rows.filter((r) => r.t >= killedAt + 100 && r.t < restartAt), post = rows.filter((r) => r.t >= readyAt);
  const firstOkAfter = rows.find((r) => r.t >= restartAt && r.ok);
  log(`O3 timeline: killed at ${killedAt} ms, restart spawned ${restartAt} ms, ready ${readyAt} ms (${readyMs} ms after spawn); requests: before ${pre.length}, during outage ${down.length}, after ready ${post.length}`);
  const errKinds = {}; for (const r of down) errKinds[r.error ?? r.status] = (errKinds[r.error ?? r.status] || 0) + 1;
  log(`O3 outage outcomes: ${JSON.stringify(errKinds)}; first success after restart at ${firstOkAfter?.t} ms (${firstOkAfter ? firstOkAfter.t - restartAt : 'n/a'} ms after spawn)`);
  check(pre.length > 20 && pre.every((r) => r.ok && r.good), 'O3: before the kill every request succeeded with correct bytes', `${pre.length} requests`);
  check(down.length > 10 && down.every((r) => !r.ok), 'O3: during the outage the client sees errors (connection refused), not stale or wrong data', JSON.stringify(errKinds));
  check(post.length > 20 && post.every((r) => r.ok && r.good), 'O3: after the restart every request succeeds with correct bytes (zero errors after ready)', `${post.length} requests, errors ${post.filter((r) => !r.ok).length}`);
  check(rows.filter((r) => r.good === false).length === 0, 'O3: no response with wrong bytes at any time', '0 wrong');
  const firstSeen = new Map(); for (const r of rows.filter((x) => x.t >= restartAt && x.ok)) if (!firstSeen.has(r.k)) firstSeen.set(r.k, r.cs);
  const coldMiss = [...firstSeen.values()].filter((c) => c === 'MISS').length;
  check(firstSeen.size === N && coldMiss === N, 'O3: the restarted gateway starts COLD (the first response for each of the 20 keys after the restart is MISS)', `MISS ${coldMiss} of ${firstSeen.size}`);
  R.data.restart = { killed_at_ms: killedAt, restart_spawned_ms: restartAt, ready_ms_after_spawn: readyMs, outage_requests: down.length, outage_outcomes: errKinds, requests_total: rows.length };
  await g.kill();
}

// ------------------------------------------------------------------ O4
async function o4() {
  if (!origin.server.listening) await origin.listen();
  const ba = makePngOfSize(50000, 401), bb = makePngOfSize(50000, 402);
  const sa = sha256(ba), sb = sha256(bb);
  origin.set('/d/a.png', { body: ba, type: 'image/png', cacheControl: 'public, max-age=300', etag: '"da"' });
  origin.set('/d/b.png', { body: bb, type: 'image/png', cacheControl: 'public, max-age=300', etag: '"db"' });
  const g = start(); await waitReady(GPORT);
  const a1 = await get('/d/a.png');
  check(a1.status === 200 && a1.sha === sa, 'O4: key A warmed', `${a1.status}`);
  await origin.close();
  const a2 = await get('/d/a.png');
  check(a2.status === 200 && a2.cs === 'HIT' && a2.sha === sa, 'O4: origin down: cached key A is still served (HIT, correct bytes)', `${a2.status} ${a2.cs}`);
  const b1 = await get('/d/b.png');
  log(`O4 origin down, uncached key: status ${b1.status} X-Cache-Error=${b1.err} body="${b1.ok ? b1.body.toString().slice(0, 80) : ''}" ${b1.ms.toFixed(0)} ms`);
  check(b1.ok && b1.status >= 500 && b1.status < 600 && b1.ms < 10000, 'O4: origin down: uncached key -> honest 5xx within 10 s', `${b1.status} in ${b1.ms.toFixed(0)} ms`);
  if (b1.ms > 1000) R.finding('L5-O-01', `With the origin down (connection refused on loopback) an uncached key takes about ${b1.ms.toFixed(0)} ms to be answered 502 (X-Cache-Error=NetworkError). A refused connection is normally immediate, so this looks like a fixed delay/retry in the origin fetch path; clients wait ~2 s for an error. Observation, not verified in code.`, 'node scripts/operability_scenario.mjs (case O4) or: stop the origin, GET /api/v1/cache/<uncached key>, time it');
  const burst = await Promise.all(Array.from({ length: 50 }, (_, i) => get(`/d/missing-${i}.png`)));
  check(burst.every((r) => r.ok && r.status >= 500 && r.status < 600), 'O4: 50 concurrent requests for uncached keys while the origin is down -> all honest 5xx, none hang or reset', `statuses ${[...new Set(burst.map((r) => r.status))]}`);
  const ready = await httpReq({ port: GPORT, path: '/health/ready' });
  check(ready.status === 200 && g.alive, 'O4: the gateway process is alive and still ready', `${ready.status} alive=${g.alive}`);
  const m = await metrics(GPORT);
  log(`O4 metrics origin errors: ${JSON.stringify(m?.origin?.errors)}`);
  await origin.listen();
  const b2 = await get('/d/b.png');
  check(b2.status === 200 && b2.sha === sb, 'O4: origin back: the same gateway serves key B correctly, no restart', `${b2.status}`);
  await g.kill();
}

// ------------------------------------------------------------------ O5
async function o5() {
  const body = makeJson(501, 3000), sh = sha256(body);
  origin.set('/ok/doc.json', { body, type: 'application/json', cacheControl: 'public, max-age=300' });
  const g = start(); await waitReady(GPORT);
  const P = '/api/v1/cache/';
  const rq = (line, extra = '') => Buffer.from(`${line}\r\nHost: 127.0.0.1\r\nConnection: close\r\n${extra}\r\n`, 'latin1');
  const cases = [
    ['path 10 KB', rq(`GET ${P}${'a'.repeat(10 * 1024)} HTTP/1.1`)],
    ['path 70 KB', rq(`GET ${P}${'a'.repeat(70 * 1024)} HTTP/1.1`)],
    ['path 1 MB', rq(`GET ${P}${'a'.repeat(1024 * 1024)} HTTP/1.1`)],
    ['query 100 KB', rq(`GET ${P}x?${'q=1&'.repeat(25 * 1024)} HTTP/1.1`)],
    ['header 100 KB', rq(`GET ${P}x HTTP/1.1`, `X-Big: ${'h'.repeat(100 * 1024)}\r\n`)],
    ['control char 0x01 in path', rq(`GET ${P}a\x01b HTTP/1.1`)],
    ['control char 0x7f in path', rq(`GET ${P}a\x7fb HTTP/1.1`)],
    ['%00 in path', rq(`GET ${P}a%00b HTTP/1.1`)],
    ['%0d%0a header injection in path', rq(`GET ${P}a%0d%0aX-Injected:%201 HTTP/1.1`)],
    ['space in path (broken request line)', rq(`GET ${P}a b HTTP/1.1`)],
    ['dot-dot traversal', rq(`GET ${P}../../etc/passwd HTTP/1.1`)],
    ['encoded dot-dot traversal', rq(`GET ${P}%2e%2e/%2e%2e/etc/passwd HTTP/1.1`)],
    ['dot segment', rq(`GET ${P}a/./b HTTP/1.1`)],
    ['invalid method', rq(`FOO ${P}x HTTP/1.1`)],
    ['binary garbage', Buffer.from([0, 1, 2, 3, 0xff, 0xfe, 13, 10, 13, 10])],
    ['unsupported HTTP version', rq(`GET ${P}x HTTP/9.9`)],
  ];
  const results = [];
  for (const [name, bytes] of cases) {
    const r = await rawRequest(GPORT, bytes);
    results.push({ name, status: r.status, note: r.note });
    log(`O5 ${name}: ${r.status ?? 'no HTTP response'} (${r.note})`);
  }
  R.data.abuse = results;
  check(results.every((r) => r.status === null || (r.status >= 400 && r.status < 500)), 'O5: every malformed/abusive request gets a 4xx or a closed connection, never 2xx/5xx', results.map((r) => `${r.name}=${r.status}`).join('; '));
  check(!results.some((r) => r.status !== null && r.status >= 500), 'O5: no 5xx for any abusive request', 'ok');
  const noResp = results.filter((r) => r.status === null).map((r) => r.name);
  if (noResp.length) log(`O5 closed without an HTTP status: ${noResp.join(', ')}`);
  let ready = await httpReq({ port: GPORT, path: '/health/ready' });
  const good = await get('/ok/doc.json');
  check(ready.status === 200 && g.alive && good.status === 200 && good.sha === sh, 'O5: after the path/header abuse the gateway is alive and serves correct bytes', `alive=${g.alive} ready=${ready.status} get=${good.status}`);

  // large bodies
  const big = Buffer.alloc(5 * 1024 * 1024, 0x41);
  const bodies = [];
  for (const m of ['GET', 'POST', 'DELETE']) {
    const r = await httpReq({ port: GPORT, method: m, path: `${P}ok/body-${m}`, body: big, headers: { 'content-length': big.length, 'content-type': 'application/octet-stream' }, timeoutMs: 15000 });
    bodies.push({ method: m, status: r.ok ? r.status : null, error: r.error });
    log(`O5 ${m} with a 5 MiB body: ${r.ok ? r.status : 'error ' + r.error}`);
  }
  R.data.big_bodies = bodies;
  check(bodies.every((b) => b.status === null || b.status < 500), 'O5: 5 MiB request bodies on GET/POST/DELETE -> no 5xx (accepted, ignored, or refused with 4xx / closed)', JSON.stringify(bodies));
  const probeUnchanged = await get('/ok/doc.json');
  ready = await httpReq({ port: GPORT, path: '/health/ready' });
  check(g.alive && ready.status === 200 && probeUnchanged.sha === sh, 'O5: gateway alive after the large bodies, bytes still correct', `alive=${g.alive}`);
  // many abusive connections at once
  const burst = await Promise.all(Array.from({ length: 100 }, () => rawRequest(GPORT, rq(`GET ${P}a\x01b HTTP/1.1`))));
  check(burst.every((r) => r.status === null || (r.status >= 400 && r.status < 500)) && g.alive, 'O5: 100 concurrent control-character requests -> all 4xx/closed, gateway alive', `statuses ${[...new Set(burst.map((r) => r.status))]}`);
  await g.kill();
}

let ok = false;
try {
  await origin.listen();
  await o1(); await o2(); await o3(); await o4(); await o5();
} catch (e) { check(false, 'unexpected error', String(e && e.stack ? e.stack : e)); }
finally { for (const g of gateways) await g.kill(); try { await origin.close(); } catch { /* */ } ok = R.finish(); }
process.exit(ok ? 0 : 1);
