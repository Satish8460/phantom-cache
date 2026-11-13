#!/usr/bin/env node
// scripts/cluster_outcomes_check.mjs
//
// Multi-process, assertion-based proof (batch B1) that a three-gateway mTLS cluster tells the five cache
// outcomes apart with the additive `X-Cache-Source` response header and the matching per-node counters in
// `GET /api/v1/metrics` (`metrics.outcomes`), and that a peer read preserves the primary's response:
//
//   origin     a cold GET on the key's primary                     (X-Cache-Status MISS)
//   local      a repeat GET on the same gateway                    (HIT)
//   peer       a cold GET on another gateway, filled from the primary, NO origin request
//              (MISS; same Content-Type, ETag and status as the primary; a truthful Age)
//   coalesced  concurrent GETs on a cold key behind one origin fetch  (COALESCED)
//   stale      a stale-while-revalidate entry served while it refreshes (STALE)
//   plus       a cold GET on a non-primary whose primary holds nothing: `peer_reads_without_hit.miss` +1, source origin
//
// `X-Cache-Status` semantics are asserted unchanged. Every counter delta is compared with what the
// responses showed, per node, and with the counting origin's own request count.
//
// The script does not generate certificates. Pass a directory with: ca.crt, members.json,
// n1.crt n2.crt n3.crt and n1.p8.key n2.p8.key n3.p8.key (PKCS#8).
//
// Usage:
//   node scripts/cluster_outcomes_check.mjs --exe <gateway.exe> --certs <dir> [--out-dir <dir>]
//        [--origin-port 19099] [--http-port-base 18185] [--cluster-port-base 19501]
// Exit code 0 only if every assertion held; 1 otherwise.

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, openSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';

const argv = process.argv.slice(2);
const opt = (name, def) => {
  const i = argv.indexOf(`--${name}`);
  return i !== -1 && argv[i + 1] !== undefined ? argv[i + 1] : def;
};
const EXE = opt('exe');
const CERTS = opt('certs');
const OUT = resolve(opt('out-dir', join(tmpdir(), 'phantom_cluster_outcomes_check')));
const ORIGIN_PORT = Number(opt('origin-port', 19099));
const HTTP_BASE = Number(opt('http-port-base', 18185));
const CLUSTER_BASE = Number(opt('cluster-port-base', 19501));
const usage = () => {
  console.error('usage: node scripts/cluster_outcomes_check.mjs --exe <gateway> --certs <dir> [--out-dir <dir>] [--origin-port N] [--http-port-base N] [--cluster-port-base N]');
  process.exit(1);
};
if (!EXE || !CERTS || !existsSync(EXE) || !existsSync(join(CERTS, 'members.json'))) usage();
if (![ORIGIN_PORT, HTTP_BASE, CLUSTER_BASE].every((n) => Number.isFinite(n) && n > 0)) usage();

mkdirSync(OUT, { recursive: true });
const CLUSTER_ID = JSON.parse(readFileSync(join(CERTS, 'members.json'), 'utf8')).cluster_id;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[OUTCOMES] ${m}`);
const procs = {};
let originServer = null;
let originCount = 0;
let failed = false;
const failures = [];
const check = (cond, msg) => {
  if (cond) return true;
  failed = true;
  failures.push(msg);
  console.error(`[OUTCOMES-FAIL] ${msg}`);
  return false;
};
const httpPort = (n) => HTTP_BASE + (n - 1);
const clusterPort = (n) => CLUSTER_BASE + (n - 1);
const P = (n) => `http://127.0.0.1:${httpPort(n)}`;

function startOrigin() {
  return new Promise((ok) => {
    originServer = createServer((req, res) => {
      originCount += 1;
      const n = originCount;
      const delay = req.url.includes('/slow/') ? 400 : 5;
      setTimeout(() => {
        res.setHeader('content-type', 'text/plain; charset=utf-8');
        res.setHeader('etag', `"o${n}"`);
        res.setHeader('cache-control', req.url.includes('/swr/') ? 'public, max-age=1, stale-while-revalidate=60' : 'public, max-age=300');
        res.end(`origin:${req.url}:${n}`);
      }, delay);
    }).listen(ORIGIN_PORT, '127.0.0.1', ok);
  });
}
function startNode(n) {
  const peers = [1, 2, 3].map((i) => `${i}=127.0.0.1:${clusterPort(i)}`).join(',');
  const env = {
    ...process.env,
    ORIGIN_URL: `http://127.0.0.1:${ORIGIN_PORT}`,
    PHANTOM_CLUSTER_ID: CLUSTER_ID,
    PHANTOM_CLUSTER_MEMBERS_FILE: join(CERTS, 'members.json'),
    PHANTOM_CLUSTER_CA: join(CERTS, 'ca.crt'),
    PHANTOM_CLUSTER_PEER_ADDRS: peers,
    PHANTOM_BIND_ADDR: `127.0.0.1:${httpPort(n)}`,
    PHANTOM_NODE_ID: String(n),
    PHANTOM_NODE_CERT: join(CERTS, `n${n}.crt`),
    PHANTOM_NODE_KEY: join(CERTS, `n${n}.p8.key`),
    PHANTOM_CLUSTER_LISTEN_ADDR: `127.0.0.1:${clusterPort(n)}`,
  };
  procs[n] = spawn(EXE, [], {
    env,
    stdio: ['ignore', openSync(join(OUT, `node${n}.out.log`), 'a'), openSync(join(OUT, `node${n}.err.log`), 'a')],
    windowsHide: true,
  });
}
const MASK = (1n << 64n) - 1n;
const hash = (key, id) => {
  let h = 0xcbf29ce484222325n;
  for (const c of Buffer.from(key)) { h ^= BigInt(c); h = (h * 0x100000001b3n) & MASK; }
  h ^= BigInt(id);
  return (h * 0x100000001b3n) & MASK;
};
const primaryOf = (path) => {
  let best = null;
  for (const id of [1, 2, 3]) {
    const w = hash(`__default__:/${path}`, id);
    if (best === null || w > best.w) best = { id, w };
  }
  return best.id;
};
const keyOwnedBy1 = (prefix) => { for (let i = 0; i < 2000; i++) if (primaryOf(`${prefix}${i}`) === 1) return `${prefix}${i}`; throw new Error('no key'); };

const get = async (n, k) => {
  const r = await fetch(`${P(n)}/api/v1/cache/${k}`);
  const body = await r.text();
  return { status: r.status, cache: r.headers.get('x-cache-status'), source: r.headers.get('x-cache-source'), age: r.headers.get('age'), ct: r.headers.get('content-type'), etag: r.headers.get('etag'), body };
};
async function outcomes(n) {
  const j = await (await fetch(`${P(n)}/api/v1/metrics`)).json();
  const o = j?.metrics?.outcomes;
  if (!o) throw new Error('metrics.outcomes missing');
  const flat = {};
  for (const [k, v] of Object.entries(o.responses)) { if (v.src !== 'measured') throw new Error(`outcomes.${k} not measured`); flat[k] = v.v; }
  for (const [k, v] of Object.entries(o.peer_reads_without_hit)) { if (v.src !== 'measured') throw new Error(`peer_reads_without_hit.${k} not measured`); flat[`peer_read_${k}`] = v.v; }
  return flat;
}
const diff = (a, b) => Object.fromEntries(Object.keys(b).map((k) => [k, b[k] - a[k]]));
async function waitReady() {
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    try {
      const j = await (await fetch(`${P(1)}/api/v1/metrics`)).json();
      const c = j?.metrics?.cluster;
      const states = [c?.local?.state?.v, ...(c?.peers ?? []).map((p) => p.state?.v)];
      if (states.length === 3 && states.every((s) => s === 'Active')) return true;
    } catch { /* not up yet */ }
    await sleep(200);
  }
  return false;
}
const expectDelta = (label, d, expected) => {
  for (const [k, v] of Object.entries(expected)) check(d[k] === v, `${label}: expected ${k} delta ${v}, observed ${d[k]} (all: ${JSON.stringify(d)})`);
};

try {
  await startOrigin();
  [1, 2, 3].forEach(startNode);
  check(await waitReady(), 'cluster did not become Active within 20 s');
  if (!failed) {
    // ---- origin, local, peer on a key whose primary is node 1
    const k = keyOwnedBy1('outcome/basic');
    let c1 = await outcomes(1), c2 = await outcomes(2), c3 = await outcomes(3);
    const r1 = await get(1, k);
    log(`node 1 cold GET: ${r1.cache} source=${r1.source} ct=${r1.ct} etag=${r1.etag}`);
    check(r1.cache === 'MISS' && r1.source === 'origin', `cold GET on the primary should be MISS/origin, got ${r1.cache}/${r1.source}`);
    const r1b = await get(1, k);
    check(r1b.cache === 'HIT' && r1b.source === 'local', `repeat GET should be HIT/local, got ${r1b.cache}/${r1b.source}`);
    const originBeforePeer = originCount;
    await sleep(2200); // let the entry age so Age is distinguishable from 0
    const r2 = await get(2, k);
    log(`node 2 cold GET (peer fill): ${r2.cache} source=${r2.source} age=${r2.age} ct=${r2.ct} etag=${r2.etag} origin_requests_during=${originCount - originBeforePeer}`);
    check(r2.cache === 'MISS' && r2.source === 'peer', `cold GET on another gateway should be MISS/peer, got ${r2.cache}/${r2.source}`);
    check(originCount === originBeforePeer, 'the peer fill must not reach the origin');
    check(r2.ct === r1.ct && r2.etag === r1.etag && r2.status === r1.status && r2.body === r1.body, `peer response differs from the primary's (ct ${r2.ct} vs ${r1.ct}, etag ${r2.etag} vs ${r1.etag})`);
    check(Number(r2.age) >= 2, `peer-filled Age should reflect the source entry's age (>= 2 s), got ${r2.age}`);
    const r2b = await get(2, k);
    check(r2b.cache === 'HIT' && r2b.source === 'local' && r2b.ct === r1.ct && r2b.etag === r1.etag, `local HIT of the peer-filled copy should keep content type and ETag, got ${JSON.stringify(r2b)}`);
    let n1 = await outcomes(1), n2 = await outcomes(2), n3 = await outcomes(3);
    expectDelta('node 1 (origin + local)', diff(c1, n1), { origin: 1, local: 1, peer: 0, coalesced: 0, stale: 0 });
    expectDelta('node 2 (peer + local)', diff(c2, n2), { peer: 1, local: 1, origin: 0, coalesced: 0, stale: 0 });
    expectDelta('node 3 (untouched)', diff(c3, n3), { peer: 0, local: 0, origin: 0, coalesced: 0, stale: 0 });

    // ---- peer read that finds nothing -> origin, counted as a peer-read miss
    const k2 = keyOwnedBy1('outcome/peermiss');
    c2 = await outcomes(2);
    const o0 = originCount;
    const rm = await get(2, k2);
    check(rm.cache === 'MISS' && rm.source === 'origin' && originCount === o0 + 1, `cold key on a non-primary with an empty primary should go to the origin, got ${rm.cache}/${rm.source}, origin delta ${originCount - o0}`);
    n2 = await outcomes(2);
    expectDelta('node 2 (peer read miss)', diff(c2, n2), { peer_read_miss: 1, origin: 1, peer: 0 });

    // ---- coalesced: concurrent GETs behind one slow origin fetch
    const k3 = keyOwnedBy1('outcome/slow/co');
    c1 = await outcomes(1);
    const o1 = originCount;
    const burst = await Promise.all(Array.from({ length: 6 }, () => get(1, k3)));
    const tally = {};
    for (const r of burst) tally[`${r.cache}/${r.source}`] = (tally[`${r.cache}/${r.source}`] || 0) + 1;
    log(`node 1 concurrent x6 on a cold slow key: ${JSON.stringify(tally)} origin_requests=${originCount - o1}`);
    n1 = await outcomes(1);
    const d = diff(c1, n1);
    check(originCount - o1 === 1, `6 concurrent GETs should cause exactly 1 origin request, got ${originCount - o1}`);
    check((tally['COALESCED/coalesced'] || 0) >= 1, `at least one response should be COALESCED/coalesced, got ${JSON.stringify(tally)}`);
    check((tally['MISS/origin'] || 0) === 1, `exactly one leader MISS/origin expected, got ${JSON.stringify(tally)}`);
    check(burst.every((r) => r.source !== null), 'every successful response must carry X-Cache-Source');
    check(d.coalesced === (tally['COALESCED/coalesced'] || 0) && d.origin === 1 && d.local === (tally['HIT/local'] || 0), `node 1 counters should match the responses: ${JSON.stringify(d)} vs ${JSON.stringify(tally)}`);

    // ---- stale: stale-while-revalidate entry served while it refreshes
    const k4 = keyOwnedBy1('outcome/swr/k');
    await get(1, k4);
    await sleep(2300); // max-age=1 has passed; stale-while-revalidate=60 allows serving it
    c1 = await outcomes(1);
    const rs = await get(1, k4);
    log(`node 1 GET of an expired swr entry: ${rs.cache} source=${rs.source}`);
    check(rs.cache === 'STALE' && rs.source === 'stale', `an expired stale-while-revalidate entry should be STALE/stale, got ${rs.cache}/${rs.source}`);
    n1 = await outcomes(1);
    expectDelta('node 1 (stale)', diff(c1, n1), { stale: 1 });

    log(`final counters: node 1 ${JSON.stringify(await outcomes(1))}, node 2 ${JSON.stringify(await outcomes(2))}, node 3 ${JSON.stringify(await outcomes(3))}`);
  }
} catch (e) {
  failed = true;
  failures.push(String(e && e.stack ? e.stack : e));
  console.error('[OUTCOMES-FAIL] unexpected error:', e);
} finally {
  for (const n of [1, 2, 3]) { try { procs[n]?.kill(); } catch { /* already gone */ } }
  if (originServer) originServer.close();
}
if (failed) {
  console.error(`[OUTCOMES] FAILED (${failures.length} assertion(s)); logs in ${OUT}`);
  process.exit(1);
}
log('PASSED: five outcomes distinguished by X-Cache-Source and matching counters; peer read preserved content type, ETag and age');
process.exit(0);
