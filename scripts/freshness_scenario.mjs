#!/usr/bin/env node
// scripts/freshness_scenario.mjs  (lane L5, scenario 1: FRESHNESS)
//
// Scripted, assertion-based check of the freshness behaviour of ONE gateway against a scriptable content origin that serves
// REAL bytes (generated PNG images, JSON, text). Every response body is compared by sha256 with the bytes the origin served.
//
//   F1a/F1b stale-while-revalidate (origin changed / unchanged -> 200 / 304 revalidation), single-flight
//   F2a..e  stale-if-error (origin 500, origin drops the socket, origin process gone, no directive, window exceeded)
//   F3      not cached: no-store, private, Vary, Set-Cookie; no-cache + ETag (always revalidated); public control
//   F4      client conditional (If-None-Match) and gateway->origin conditional revalidation (ETag, origin 304)
//   F5      PHANTOM_MAX_STALE_MS enforced: with a 3 s cap an entry older than lifetime+cap is NOT served stale
//
// Usage: node scripts/freshness_scenario.mjs [--exe <gateway>] [--out-dir <dir>] [--origin-port 19230] [--port-base 18230]
// Exit code 0 only if every assertion held.  Results: <out-dir>/freshness.result.json and freshness.log.txt

import { join } from 'node:path';
import { argv, DEFAULT_EXE, defaultOut, httpReq, makePngOfSize, makeJson, makeText, sha256, sleep, spawnGateway, startOrigin, waitReady, metrics, mv, recorder, isValidPng } from './lib/scenario_common.mjs';

const { opt } = argv();
const EXE = opt('exe', DEFAULT_EXE);
const OUT = opt('out-dir', defaultOut('freshness'));
const OPORT = Number(opt('origin-port', 19230));
const PA = Number(opt('port-base', 18230));
const PB = PA + 1;
const CAP_MS = 3000;

const R = recorder('freshness', OUT, { prefix: 'FRESHNESS' });
const { check, log } = R;
const origin = startOrigin({ port: OPORT });
let gwA, gwB;

const png = (kb, seed) => makePngOfSize(kb * 1000, seed);
const put = (path, body, type, o = {}) => { origin.set(path, { body, type, ...o }); return sha256(body); };
const get = async (port, path, headers = {}) => {
  const r = await httpReq({ port, path: `/api/v1/cache${path}`, headers });
  if (!r.ok) return { ok: false, error: r.error, ms: r.ms };
  return { ok: true, status: r.status, cs: r.headers['x-cache-status'], src: r.headers['x-cache-source'], age: r.headers['age'], etag: r.headers['etag'], type: r.headers['content-type'], sha: r.body.length ? sha256(r.body) : null, len: r.body.length, ms: r.ms, err: r.headers['x-cache-error'] };
};
const brief = (r) => (r.ok ? `${r.status} ${r.cs ?? '-'}/${r.src ?? '-'} age=${r.age ?? '-'} etag=${r.etag ?? '-'} len=${r.len} ${r.ms.toFixed(0)}ms` : `ERR ${r.error}`);
const tag = (r, sha) => (r.ok && r.sha === sha ? 'bytes=OK' : r.ok ? `bytes=${r.sha ? 'DIFFERENT' : 'none'}` : 'bytes=n/a');

async function caseSwr(changed) {
  const n = changed ? 'swr-changed' : 'swr-304';
  const p = `/f/${n}.png`;
  const cc = 'public, max-age=1, stale-while-revalidate=30';
  const v1 = png(60, changed ? 11 : 12);
  const s1 = put(p, v1, 'image/png', { etag: '"v1"', cacheControl: cc, notModified: !changed });
  check(isValidPng(v1), `${n}: the origin object is a valid PNG (${v1.length} bytes)`);
  const a = await get(PA, p); log(`${n}: GET1 ${brief(a)} ${tag(a, s1)}`);
  check(a.status === 200 && a.cs === 'MISS' && a.src === 'origin' && a.sha === s1, `${n}: first GET is MISS/origin with the origin bytes`, brief(a));
  const b = await get(PA, p); log(`${n}: GET2 ${brief(b)}`);
  check(b.cs === 'HIT' && b.src === 'local' && b.sha === s1, `${n}: second GET inside the lifetime is HIT/local, same bytes`, brief(b));
  await sleep(1400);
  let s2 = s1;
  if (changed) s2 = put(p, png(60, 21), 'image/png', { etag: '"v2"', cacheControl: cc, delayMs: 700 });
  else origin.set(p, { body: v1, type: 'image/png', etag: '"v1"', cacheControl: cc, notModified: true, delayMs: 700 });
  const before = origin.count(p);
  const st = await get(PA, p); log(`${n}: expired GET ${brief(st)} ${tag(st, s1)}`);
  check(st.status === 200 && st.cs === 'STALE' && st.src === 'stale' && st.sha === s1, `${n}: expired entry is served STALE with the OLD bytes`, brief(st));
  check(st.ms < 400, `${n}: the stale answer does not wait for the (700 ms) origin`, `${st.ms.toFixed(0)} ms`);
  check(Number(st.age) >= 1, `${n}: Age header >= 1 s on the stale answer`, `age=${st.age}`);
  const burst = await Promise.all([0, 1, 2, 3].map(() => get(PA, p)));
  check(burst.every((r) => r.ok && r.status === 200 && r.sha === s1 && (r.cs === 'STALE' || r.cs === 'HIT' || r.cs === 'COALESCED')), `${n}: 4 more concurrent GETs during the revalidation still get 200 with the old bytes`, burst.map((r) => `${r.cs}/${r.src}`).join(' '));
  await sleep(1500);
  check(origin.count(p) - before === 1, `${n}: exactly ONE background revalidation reached the origin (single-flight)`, `origin requests since the stale GET = ${origin.count(p) - before}`);
  const last = origin.log.filter((e) => e.path === p).at(-1);
  check(last?.inm === '"v1"', `${n}: the revalidation was conditional (If-None-Match: "v1")`, `inm=${last?.inm}`);
  const after = await get(PA, p); log(`${n}: after revalidation ${brief(after)} ${tag(after, s2)}`);
  check(after.status === 200 && after.cs === 'HIT' && after.sha === s2, `${n}: after the revalidation the entry is a fresh HIT with the ${changed ? 'NEW' : 'same'} bytes`, brief(after));
  if (changed) check(after.etag === '"v2"', `${n}: the stored ETag moved to "v2"`, `etag=${after.etag}`);
  else check(Number(after.age) <= 1, `${n}: after a 304 the entry age restarted (Age <= 1)`, `age=${after.age}`);
  return { n };
}

async function caseSie() {
  const cc = 'public, max-age=1, stale-if-error=30';
  // a: origin answers 500
  const pa = '/f/sie-500.json', va = makeJson(31, 4000);
  const sa = put(pa, va, 'application/json', { etag: '"a1"', cacheControl: cc });
  // b: origin drops the TCP connection
  const pb = '/f/sie-drop.json', vb = makeJson(32, 4000);
  const sb = put(pb, vb, 'application/json', { etag: '"b1"', cacheControl: cc });
  // c: origin process gone (connection refused) - done later in phase B using this path
  const pc = '/f/sie-gone.png', vc = png(40, 33);
  const sc = put(pc, vc, 'image/png', { etag: '"c1"', cacheControl: cc });
  // d: no stale-if-error directive
  const pd = '/f/sie-nodirective.json', vd = makeJson(34, 3000);
  const sd = put(pd, vd, 'application/json', { etag: '"d1"', cacheControl: 'public, max-age=1' });
  // e: window exceeded (max-age 1 + stale-if-error 2 = 3 s)
  const pe = '/f/sie-window.json', ve = makeJson(35, 3000);
  const se = put(pe, ve, 'application/json', { etag: '"e1"', cacheControl: 'public, max-age=1, stale-if-error=2' });
  const warm = await Promise.all([pa, pb, pc, pd, pe].map((p) => get(PA, p)));
  check(warm.every((r) => r.status === 200 && r.cs === 'MISS'), 'sie: five paths warmed (MISS)', warm.map(brief).join(' | '));
  await sleep(1400);
  origin.set(pa, { status: 500, body: Buffer.from('boom'), type: 'text/plain', cacheControl: 'no-store' });
  origin.set(pb, { destroy: true });
  origin.set(pd, { status: 500, body: Buffer.from('boom'), type: 'text/plain' });
  const a = await get(PA, pa); log(`sie-500: ${brief(a)} ${tag(a, sa)}`);
  check(a.status === 200 && a.cs === 'STALE' && a.sha === sa, 'sie-500: origin 500 + stale-if-error -> expired entry served STALE (old bytes, 200)', brief(a));
  const b = await get(PA, pb); log(`sie-drop: ${brief(b)} ${tag(b, sb)}`);
  check(b.status === 200 && b.cs === 'STALE' && b.sha === sb, 'sie-drop: origin drops the connection -> expired entry served STALE', brief(b));
  const d = await get(PA, pd); log(`sie-nodirective: ${brief(d)}`);
  check(d.ok && d.status >= 500 && d.sha !== sd, 'sie-nodirective (negative control): without stale-if-error the origin 500 is reported honestly, the expired bytes are NOT served', brief(d));
  // e: wait until age > 1 + 2 s, then make the origin fail
  await sleep(2200);
  origin.set(pe, { status: 500, body: Buffer.from('boom'), type: 'text/plain' });
  const e = await get(PA, pe); log(`sie-window: ${brief(e)}`);
  check(e.ok && e.status >= 500 && e.sha !== se, 'sie-window: older than lifetime + stale-if-error window -> NOT served stale, honest 5xx', brief(e));
  return { pc, sc };
}

async function caseNotCached() {
  const cases = [
    ['no-store', { cacheControl: 'no-store' }],
    ['private', { cacheControl: 'private, max-age=60' }],
    ['vary-lang', { cacheControl: 'public, max-age=60', headers: { vary: 'Accept-Language' } }],
    ['set-cookie', { cacheControl: 'max-age=60', headers: { 'set-cookie': 'sid=abc; Path=/' } }],
    ['control-public', { cacheControl: 'public, max-age=60' }],
  ];
  for (const [n, spec] of cases) {
    const p = `/f/nc-${n}.png`;
    const body = png(35, 40 + n.length), s = put(p, body, 'image/png', { etag: `"nc-${n}"`, ...spec });
    const rs = [];
    for (let i = 0; i < 3; i++) rs.push(await get(PA, p));
    log(`${n}: ${rs.map((r) => `${r.cs}/${r.src}`).join(' ')} origin_requests=${origin.count(p)}`);
    check(rs.every((r) => r.status === 200 && r.sha === s), `${n}: all 3 responses are 200 with the origin bytes`, rs.map(brief).join(' | '));
    if (n === 'control-public') check(origin.count(p) === 1 && rs[1].cs === 'HIT' && rs[2].cs === 'HIT', 'control-public: cached (1 origin request, 2 HITs)', `origin_requests=${origin.count(p)}`);
    else check(origin.count(p) === 3 && rs.every((r) => r.cs !== 'HIT'), `${n}: NOT cached (3 origin requests, no HIT)`, `origin_requests=${origin.count(p)} statuses=${rs.map((r) => r.cs).join(',')}`);
  }
  // no-cache + ETag: stored, but revalidated every time
  const p = '/f/nc-no-cache.png', body = png(35, 77), s = put(p, body, 'image/png', { etag: '"nocache1"', cacheControl: 'no-cache', notModified: true });
  const rs = []; for (let i = 0; i < 3; i++) rs.push(await get(PA, p));
  log(`no-cache+etag: ${rs.map((r) => `${r.cs}/${r.src}`).join(' ')} origin_requests=${origin.count(p)} inm=${origin.log.filter((e) => e.path === p).map((e) => e.inm).join(',')}`);
  check(rs.every((r) => r.status === 200 && r.sha === s), 'no-cache+ETag: all 3 responses 200 with the right bytes', rs.map(brief).join(' | '));
  check(origin.count(p) === 3, 'no-cache+ETag: every request is revalidated at the origin (3 origin requests)', `origin_requests=${origin.count(p)}`);
  check(origin.log.filter((e) => e.path === p).slice(1).every((e) => e.inm === '"nocache1"'), 'no-cache+ETag: revalidations carry If-None-Match', 'ok');
}

async function caseConditional() {
  const p = '/f/cond.png', body = png(80, 51), s = put(p, body, 'image/png', { etag: '"c1"', cacheControl: 'public, max-age=60' });
  const a = await get(PA, p);
  check(a.status === 200 && a.sha === s, 'cond: warm (200, origin bytes)', brief(a));
  const m = await get(PA, p, { 'If-None-Match': '"c1"' }); log(`cond INM match: ${brief(m)}`);
  check(m.status === 304 && m.len === 0 && m.cs === 'HIT', 'cond: client If-None-Match matching the stored ETag -> 304 with no body (HIT)', brief(m));
  const w = await get(PA, p, { 'If-None-Match': 'W/"c1"' }); log(`cond INM weak: ${brief(w)}`);
  check(w.status === 304, 'cond: weak comparison W/"c1" -> 304', brief(w));
  const o = await get(PA, p, { 'If-None-Match': '"other"' });
  check(o.status === 200 && o.sha === s, 'cond: non-matching If-None-Match -> 200 with full bytes', brief(o));
  check(origin.count(p) === 1, 'cond: the conditional client requests caused no extra origin request', `origin_requests=${origin.count(p)}`);
  // gateway -> origin revalidation after expiry (no SWR directive)
  const q = '/f/reval.png', qb = png(50, 52), qs = put(q, qb, 'image/png', { etag: '"r1"', cacheControl: 'public, max-age=1', notModified: true });
  await get(PA, q); await sleep(1400);
  const r = await get(PA, q); log(`reval after expiry: ${brief(r)} ${tag(r, qs)}`);
  check(r.status === 200 && r.sha === qs, 'reval: expired entry (no stale directive) -> client gets 200 with the right bytes', brief(r));
  const qlog = origin.log.filter((e) => e.path === q);
  check(origin.count(q) === 2, 'reval: the expired entry caused exactly one more origin request (full or conditional)', `origin_requests=${origin.count(q)}`);
  if (qlog.at(-1).inm === '"r1"') log('reval: the refetch of the expired entry was conditional (If-None-Match sent)');
  else R.finding('L5-F-01', 'An entry that expired WITHOUT a stale-while-revalidate/stale-if-error window is refetched in full, without If-None-Match, once the lazy sweeper (default 100 ms tick) has removed it (src/core/cache.rs:386-440, window_ms = 0 when no stale directive). Stored ETag validators are therefore used only inside a stale window or in the <=100 ms before the sweep. Cost: the whole body (here a 50 KB PNG; up to 2 MiB) is re-transferred although the origin could have answered 304.', 'node scripts/freshness_scenario.mjs ; see case reval (max-age=1, ETag, origin honours If-None-Match; wait 1.4 s; second origin request has no If-None-Match)');
  log(`reval: X-Cache-Status on the revalidated answer = ${r.cs}/${r.src} (recorded, not asserted)`);
  const r2 = await get(PA, q);
  check(r2.cs === 'HIT' && r2.sha === qs && Number(r2.age) <= 1, 'reval: after the 304 the entry is a fresh HIT again (Age <= 1)', brief(r2));
  return { revalStatus: `${r.cs}/${r.src}` };
}

async function caseCap(label, directive) {
  const p = `/f/cap-${label}.json`, body = makeJson(60 + label.length, 5000);
  const cc = `public, max-age=1, ${directive}=600`;
  const s = put(p, body, 'application/json', { etag: '"cap1"', cacheControl: cc });
  const w = await get(PB, p);
  check(w.status === 200 && w.cs === 'MISS' && w.sha === s, `cap-${label}: warmed on the gateway with PHANTOM_MAX_STALE_MS=${CAP_MS}`, brief(w));
  const t0 = Date.now();
  origin.set(p, { status: 500, body: Buffer.from('origin down'), type: 'text/plain', cacheControl: 'no-store' }); // revalidation / refetch cannot succeed
  const rows = [];
  while (Date.now() - t0 < 7000) {
    const r = await get(PB, p);
    rows.push({ t: Date.now() - t0, status: r.status, cs: r.cs, src: r.src, age: r.age, same: r.sha === s });
    await sleep(350);
  }
  log(`cap-${label} timeline (ms since warm): ${rows.map((x) => `${x.t}:${x.status}${x.same ? '=v1' : ''}/${x.cs ?? x.src ?? '-'}/age${x.age ?? '-'}`).join('  ')}`);
  const served = rows.filter((x) => x.status === 200 && x.same && x.cs === 'STALE');
  const lifetime = 1000, bound = lifetime + CAP_MS;
  check(served.some((x) => x.t > lifetime), `cap-${label}: the expired entry IS served stale inside lifetime + cap`, `${served.length} stale answers`);
  const tol = 250;
  const late = rows.filter((x) => x.t > bound + tol && x.status === 200 && x.same);
  check(late.length === 0, `cap-${label}: NO answer with the old bytes after lifetime + cap (${bound} ms, tolerance ${tol} ms)`, late.length ? JSON.stringify(late) : 'none');
  const tail = rows.filter((x) => x.t > bound + tol);
  check(tail.length > 0 && tail.every((x) => x.status >= 500), `cap-${label}: after the bound the client gets the honest origin error (5xx)`, tail.map((x) => x.status).join(','));
  return rows;
}

async function main() {
  await origin.listen();
  gwA = spawnGateway({ exe: EXE, port: PA, originUrl: origin.url, outDir: OUT, name: 'gwA' });
  gwB = spawnGateway({ exe: EXE, port: PB, originUrl: origin.url, outDir: OUT, name: 'gwB', env: { PHANTOM_MAX_STALE_MS: String(CAP_MS) } });
  const [ra, rb] = await Promise.all([waitReady(PA), waitReady(PB)]);
  check(ra !== null && rb !== null, 'both gateways ready', `A ${ra} ms, B ${rb} ms`);
  if (ra === null || rb === null) return;

  // Phase A: independent cases, run concurrently on gateway A (default cap) and the cap cases on gateway B.
  const [, sie, , cond, rowsS, rowsE] = await Promise.all([
    caseSwr(true).then(() => caseSwr(false)),
    caseSie(),
    caseNotCached(),
    caseConditional(),
    caseCap('swr', 'stale-while-revalidate'),
    (async () => { await sleep(100); return caseCap('sie', 'stale-if-error'); })(),
  ]);
  R.data.cap_timeline_swr = rowsS; R.data.cap_timeline_sie = rowsE; R.data.reval_status = cond.revalStatus;

  // Phase B: the origin process itself disappears (connection refused). The sie-gone entry is already expired.
  await origin.close();
  const g = await get(PA, sie.pc); log(`origin gone: ${brief(g)} ${tag(g, sie.sc)}`);
  check(g.status === 200 && g.cs === 'STALE' && g.sha === sie.sc, 'sie-gone: origin process gone (connection refused) + stale-if-error -> expired entry served STALE', brief(g));
  const nk = await get(PA, '/f/never-seen.png'); log(`origin gone, uncached key: ${brief(nk)}`);
  check(nk.ok && nk.status >= 500 && nk.status < 600, 'origin gone, key never cached: honest 5xx (no stale to serve), no crash', brief(nk));
  const hl = await httpReq({ port: PA, path: '/health/ready' });
  check(hl.status === 200, 'gateway A still ready while the origin is gone', `status ${hl.status}`);

  // metrics (as the gateway reports them)
  const mA = await metrics(PA), mB = await metrics(PB);
  const pick = (m) => ({ stale_served_total: mv(m, 'stale_served_total'), stale_cap_violations: mv(m, 'stale_cap_violations'), revalidations: Object.fromEntries(['started', 'ok', 'not_modified', 'error', 'in_flight'].map((k) => [k, mv(m, 'revalidations', k)])), outcomes: Object.fromEntries(['local', 'origin', 'stale', 'coalesced', 'peer'].map((k) => [k, mv(m, 'outcomes', 'responses', k)])), hits: mv(m, 'cache_hits'), misses: mv(m, 'cache_misses') });
  R.data.metrics_A = pick(mA); R.data.metrics_B = pick(mB);
  log(`metrics A: ${JSON.stringify(R.data.metrics_A)}`);
  log(`metrics B: ${JSON.stringify(R.data.metrics_B)}`);
  check(R.data.metrics_A.stale_cap_violations === 0 && R.data.metrics_B.stale_cap_violations === 0, 'metrics: stale_cap_violations is 0 on both gateways', `A=${R.data.metrics_A.stale_cap_violations} B=${R.data.metrics_B.stale_cap_violations}`);
  check(R.data.metrics_A.stale_served_total >= 6 && R.data.metrics_B.stale_served_total >= 2, 'metrics: stale_served_total counts the stale answers (A >= 6, B >= 2)', `A=${R.data.metrics_A.stale_served_total} B=${R.data.metrics_B.stale_served_total}`);
  check(R.data.metrics_A.revalidations.started >= 2 && R.data.metrics_A.revalidations.not_modified >= 1, 'metrics: revalidations.started >= 2 and not_modified >= 1 on A', JSON.stringify(R.data.metrics_A.revalidations));
}

let ok = false;
try { await main(); } catch (e) { check(false, 'unexpected error', String(e && e.stack ? e.stack : e)); }
finally {
  await gwA?.kill(); await gwB?.kill(); try { await origin.close(); } catch { /* closed */ }
  ok = R.finish();
}
process.exit(ok ? 0 : 1);
