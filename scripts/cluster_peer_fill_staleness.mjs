#!/usr/bin/env node
// scripts/cluster_peer_fill_staleness.mjs
//
// Wall-clock MEASUREMENT of peer-fill freshness (PC-AUTO-018 / PC-AUTO-035, WP9) on three real
// gateway processes with mutual TLS and a counting origin. It measures, it does not assume:
//
//   * origin sends `Cache-Control: max-age=<L>`, so the source entry on the key's primary (node 1)
//     expires after L seconds;
//   * at t = `--fill-at-s` node 2 receives its first GET for that key. It has no local copy, asks
//     the primary over the cluster transport (peer read) and stores what it gets (peer fill);
//   * from then on both nodes are polled once per second. The script records when node 1 stops
//     serving the original body (source expiry) and when node 2 does (peer-copy expiry).
//
// Asserted (exit 1 if violated):
//   - the first GET at node 2 did not reach the origin (origin request count stays 1) and carries
//     `X-Cache-Source: peer`;
//   - fidelity (PC-AUTO-041): the peer-filled response has the primary's Content-Type (text/plain) and ETag;
//   - truthful age (PC-AUTO-042): its `Age` is about `--fill-at-s`, not 0;
//   - the peer copy expires WITH its source: it stops being served within `--tolerance-s` of the moment
//     the source entry expired (polling granularity is 1 s). Before batch B1 it outlived the source by
//     about 58 s (fixed 60 s lifetime from fill time, `Age: 0`, JSON content type, no ETag).
// Run against a pre-B1 binary this script FAILS on purpose (that is its job).
//
// Limits of this measurement: loopback, one key, one hop (primary -> one peer), default
// PHANTOM_MAX_STALE_MS, no stale serving observed. It says nothing about chains of peer fills,
// other TTLs than max-age=L, or multi-host clocks.
//
// The script does not generate certificates. Pass a directory with: ca.crt, members.json,
// n1.crt n2.crt n3.crt and n1.p8.key n2.p8.key n3.p8.key (PKCS#8).
//
// Usage:
//   node scripts/cluster_peer_fill_staleness.mjs --exe <gateway.exe> --certs <dir>
//        [--max-age-s 8] [--fill-at-s 6] [--tolerance-s 3] [--out-dir <dir>]
//        [--origin-port 19099] [--http-port-base 18185] [--cluster-port-base 19501]
// Takes about fill-at-s + 60 + a few seconds.

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, openSync, readFileSync, existsSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';

const argv = process.argv.slice(2);
const opt = (name, def) => {
  const i = argv.indexOf(`--${name}`);
  return i !== -1 && argv[i + 1] !== undefined ? argv[i + 1] : def;
};
const EXE = opt('exe');
const CERTS = opt('certs');
const MAX_AGE = Number(opt('max-age-s', 8));
const FILL_AT = Number(opt('fill-at-s', 6));
const TOL = Number(opt('tolerance-s', 3));
const OUT = resolve(opt('out-dir', join(tmpdir(), 'phantom_peer_fill_staleness')));
const ORIGIN_PORT = Number(opt('origin-port', 19099));
const HTTP_BASE = Number(opt('http-port-base', 18185));
const CLUSTER_BASE = Number(opt('cluster-port-base', 19501));
const PEER_TTL_S = 60; // fixed TTL used by peer fill (src/api/server.rs, `ttl_ms: 60_000`)

const usage = () => {
  console.error('usage: node scripts/cluster_peer_fill_staleness.mjs --exe <gateway> --certs <dir> [--max-age-s 8] [--fill-at-s 6] [--tolerance-s 3] [--out-dir <dir>] [--origin-port N] [--http-port-base N] [--cluster-port-base N]');
  process.exit(1);
};
if (!EXE || !CERTS || !existsSync(EXE) || !existsSync(join(CERTS, 'members.json'))) usage();
if (![MAX_AGE, FILL_AT, TOL, ORIGIN_PORT, HTTP_BASE, CLUSTER_BASE].every((n) => Number.isFinite(n) && n > 0)) usage();
if (FILL_AT >= MAX_AGE) { console.error('--fill-at-s must be smaller than --max-age-s (the source entry must still be fresh at fill time)'); process.exit(1); }

mkdirSync(OUT, { recursive: true });
const CLUSTER_ID = JSON.parse(readFileSync(join(CERTS, 'members.json'), 'utf8')).cluster_id;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[PEER-FILL] ${m}`);
const procs = {};
let originServer = null;
let originCount = 0;
let failed = false;
const failures = [];
const check = (cond, msg) => {
  if (cond) return true;
  failed = true;
  failures.push(msg);
  console.error(`[PEER-FILL-FAIL] ${msg}`);
  return false;
};
const httpPort = (n) => HTTP_BASE + (n - 1);
const clusterPort = (n) => CLUSTER_BASE + (n - 1);
const P = (n) => `http://127.0.0.1:${httpPort(n)}`;

function startOrigin() {
  return new Promise((ok) => {
    originServer = createServer((req, res) => {
      originCount += 1;
      res.setHeader('cache-control', `public, max-age=${MAX_AGE}`);
      res.setHeader('content-type', 'text/plain; charset=utf-8');
      res.setHeader('etag', `"e${originCount}"`);
      res.end(`v${originCount}`);
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
const getFull = async (n, k) => {
  const r = await fetch(`${P(n)}/api/v1/cache/${k}`);
  const body = await r.text();
  return { status: r.status, cache: r.headers.get('x-cache-status'), source: r.headers.get('x-cache-source'), age: r.headers.get('age'), ct: r.headers.get('content-type'), etag: r.headers.get('etag'), body };
};
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

const record = { max_age_s: MAX_AGE, fill_at_s: FILL_AT, peer_ttl_s: PEER_TTL_S, samples: [] };
try {
  await startOrigin();
  [1, 2, 3].forEach(startNode);
  check(await waitReady(), 'cluster did not become Active within 20 s');
  if (!failed) {
    let key = null;
    for (let i = 0; i < 1000 && !key; i++) if (primaryOf(`peerfill/k${i}`) === 1) key = `peerfill/k${i}`;
    const t0 = Date.now();
    const sec = () => (Date.now() - t0) / 1000;

    const first = await getFull(1, key);
    log(`t=${sec().toFixed(1)}s node 1 (primary) GET: ${first.cache} age=${first.age} body=${first.body} origin_requests=${originCount}`);
    check(first.body === 'v1' && originCount === 1, `primary fetch expected body v1 with 1 origin request, got ${first.body} / ${originCount}`);

    await sleep(Math.max(0, FILL_AT * 1000 - (Date.now() - t0)));
    const fill = await getFull(2, key);
    const tFill = sec();
    log(`t=${tFill.toFixed(1)}s node 2 first GET (peer fill): ${fill.cache} source=${fill.source} age=${fill.age} ct=${fill.ct} etag=${fill.etag} body=${fill.body} origin_requests=${originCount}`);
    check(originCount === 1, `node 2's first GET reached the origin (origin_requests=${originCount}); it was expected to be a peer fill`);
    check(fill.body === 'v1', `peer fill should return the primary's body v1, got ${fill.body}`);
    check(fill.source === 'peer', `X-Cache-Source of the peer fill should be peer, got ${fill.source}`);
    check((fill.ct || '').startsWith('text/plain'), `peer-filled Content-Type should equal the primary's text/plain, got ${fill.ct}`);
    check(fill.etag === '"e1"', `peer-filled ETag should equal the primary's "e1", got ${fill.etag}`);
    check(Number(fill.age) >= Math.floor(tFill) - 1 && Number(fill.age) <= Math.ceil(tFill) + 1, `peer-filled Age should be about ${tFill.toFixed(1)} s (the source entry's age), got ${fill.age}`);
    record.fill = { t_s: tFill, age_header: fill.age, x_cache_status: fill.cache };

    let srcExpiredAt = null;
    let peerExpiredAt = null;
    let lastV1ServedByPeerAt = null;
    const end = FILL_AT + PEER_TTL_S + TOL + 5;
    while (sec() < end && peerExpiredAt === null) {
      await sleep(1000);
      const t = sec();
      const a = await getFull(1, key);
      const b = await getFull(2, key);
      record.samples.push({ t_s: Number(t.toFixed(2)), n1: { cache: a.cache, age: a.age, body: a.body }, n2: { cache: b.cache, age: b.age, body: b.body } });
      if (srcExpiredAt === null && a.body !== 'v1') { srcExpiredAt = t; log(`t=${t.toFixed(1)}s node 1 (source) now serves ${a.body} (source entry expired and was refetched)`); }
      if (b.body === 'v1') lastV1ServedByPeerAt = t;
      if (b.body !== 'v1') { peerExpiredAt = t; log(`t=${t.toFixed(1)}s node 2 (peer copy) now serves ${b.body} (peer copy expired)`); }
    }
    check(srcExpiredAt !== null, 'source entry never expired within the observation window');
    check(peerExpiredAt !== null, 'peer copy never expired within the observation window');
    if (srcExpiredAt !== null && peerExpiredAt !== null) {
      const overshoot = peerExpiredAt - srcExpiredAt;
      const bodyAgeAtLastPeerV1 = lastV1ServedByPeerAt; // body v1 was created at t~0
      record.result = { source_expired_at_s: srcExpiredAt, peer_expired_at_s: peerExpiredAt, overshoot_s: overshoot, last_v1_served_by_peer_at_s: lastV1ServedByPeerAt };
      log(`MEASURED: source expired at ~${srcExpiredAt.toFixed(1)} s, peer copy expired at ~${peerExpiredAt.toFixed(1)} s; peer copy outlived its source by ~${overshoot.toFixed(1)} s; last v1 (body age) served by the peer at ~${bodyAgeAtLastPeerV1?.toFixed(1)} s`);
      log(`MODEL: the peer copy keeps the source age and remaining lifetime, so it expires with its source (max-age ${MAX_AGE} s); the old fixed 60 s from fill time would be ${(tFill + PEER_TTL_S).toFixed(1)} s`);
      check(overshoot <= TOL, `peer copy outlived its source by ${overshoot.toFixed(1)} s (limit ${TOL} s)`);
      check(overshoot >= -TOL, `peer copy expired ${(-overshoot).toFixed(1)} s BEFORE its source (limit ${TOL} s)`);
    }
  }
} catch (e) {
  failed = true;
  failures.push(String(e && e.stack ? e.stack : e));
  console.error('[PEER-FILL-FAIL] unexpected error:', e);
} finally {
  for (const n of [1, 2, 3]) { try { procs[n]?.kill(); } catch { /* already gone */ } }
  if (originServer) originServer.close();
  try { writeFileSync(join(OUT, 'peer_fill_staleness.json'), JSON.stringify(record, null, 2)); } catch { /* best effort */ }
}
if (failed) {
  console.error(`[PEER-FILL] FAILED (${failures.length} assertion(s)); raw samples in ${OUT}`);
  process.exit(1);
}
log(`PASSED; raw samples in ${join(OUT, 'peer_fill_staleness.json')}`);
process.exit(0);
