#!/usr/bin/env node
// scripts/cluster_burst_check.mjs
//
// Multi-process, assertion-based regression check for PC-AUTO-037 (a burst of concurrent cluster
// invalidations used to leave every node `Recovering` and later DELETEs stopped reaching peers) and
// for PC-AUTO-034 (`exhausted` counter). It starts THREE real gateway processes with mutual TLS plus
// a counting origin, then repeats `--cycles` times:
//
//   1. warm `--burst` keys on node 3 (keys whose rendezvous primary is not node 3),
//   2. fire `--burst` truly concurrent DELETEs at node 1,
//   3. assert within `--settle-s` seconds that node 1's own view (metrics.cluster) reports ALL three
//      nodes Active,
//   4. assert every burst key was purged on node 3 (GET there is a MISS, not a HIT),
//   5. assert the NEXT, single DELETE of a freshly warmed key also reaches node 3,
//   6. assert the broadcaster counters `dropped` and `exhausted` did not move.
//
// Finally (unless --no-exhaustion) it kills node 3, issues DELETEs and asserts `exhausted` rises
// while `dropped` stays 0.
//
// The script does not generate certificates. Pass a directory with: ca.crt, members.json,
// n1.crt n2.crt n3.crt and n1.p8.key n2.p8.key n3.p8.key (PKCS#8). members.json carries the cluster
// id and the member fingerprints.
//
// Usage:
//   node scripts/cluster_burst_check.mjs --exe <gateway.exe> --certs <dir> [--burst 40] [--cycles 20]
//        [--settle-s 5] [--out-dir <dir>] [--origin-port 19099] [--http-port-base 18185]
//        [--cluster-port-base 19501] [--no-exhaustion]
// Exit code 0 only if every assertion held; 1 otherwise (and on bad arguments).

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
const flag = (name) => argv.includes(`--${name}`);

const EXE = opt('exe');
const CERTS = opt('certs');
const BURST = Number(opt('burst', 40));
const CYCLES = Number(opt('cycles', 20));
const SETTLE_S = Number(opt('settle-s', 5));
const OUT = resolve(opt('out-dir', join(tmpdir(), 'phantom_cluster_burst_check')));
const ORIGIN_PORT = Number(opt('origin-port', 19099));
const HTTP_BASE = Number(opt('http-port-base', 18185));
const CLUSTER_BASE = Number(opt('cluster-port-base', 19501));
const DO_EXHAUSTION = !flag('no-exhaustion');

const usage = () => {
  console.error('usage: node scripts/cluster_burst_check.mjs --exe <gateway> --certs <dir> [--burst 40] [--cycles 20] [--settle-s 5] [--out-dir <dir>] [--origin-port N] [--http-port-base N] [--cluster-port-base N] [--no-exhaustion]');
  process.exit(1);
};
if (!EXE || !CERTS || !existsSync(EXE) || !existsSync(join(CERTS, 'members.json'))) usage();
if (![BURST, CYCLES, SETTLE_S, ORIGIN_PORT, HTTP_BASE, CLUSTER_BASE].every((n) => Number.isFinite(n) && n > 0)) usage();

mkdirSync(OUT, { recursive: true });
const members = JSON.parse(readFileSync(join(CERTS, 'members.json'), 'utf8'));
const CLUSTER_ID = members.cluster_id;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[BURST-CHECK] ${m}`);
const procs = {};
let originServer = null;
let failed = false;
const failures = [];
const check = (cond, msg) => {
  if (cond) return true;
  failed = true;
  failures.push(msg);
  console.error(`[BURST-CHECK-FAIL] ${msg}`);
  return false;
};

const httpPort = (n) => HTTP_BASE + (n - 1);
const clusterPort = (n) => CLUSTER_BASE + (n - 1);
const P = (n) => `http://127.0.0.1:${httpPort(n)}`;

function startOrigin() {
  return new Promise((ok) => {
    originServer = createServer((req, res) => {
      res.setHeader('cache-control', 'public, max-age=300');
      res.setHeader('content-type', 'text/plain');
      res.end(`origin:${req.url}`);
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

// 64-bit FNV-1a rendezvous hash, identical to src/dist/router.rs `calculate_hash` (see ui/src/lib/hrw.js).
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
const keysNotOwnedBy3 = (prefix, count) => {
  const out = [];
  for (let i = 0; out.length < count; i++) {
    const k = `${prefix}${i}`;
    if (primaryOf(k) !== 3) out.push(k);
  }
  return out;
};

const get = async (n, k) => {
  const r = await fetch(`${P(n)}/api/v1/cache/${k}`);
  await r.text();
  return r.headers.get('x-cache-status');
};
const del = async (n, k) => (await fetch(`${P(n)}/api/v1/cache/${k}`, { method: 'DELETE' })).status;

// Node 1's view: local node plus peers, with the broadcaster counters.
async function view1() {
  const j = await (await fetch(`${P(1)}/api/v1/metrics`)).json();
  const c = j?.metrics?.cluster;
  if (!c || !c.broadcaster || !c.peers || !c.local) throw new Error('metrics.cluster missing or malformed');
  const states = { [c.local.node_id]: c.local.state?.v };
  for (const p of c.peers) states[p.node_id] = p.state?.v;
  const b = Object.fromEntries(Object.entries(c.broadcaster).map(([k, v]) => [k, v.v]));
  return { states, b };
}
const allActive = (states) => [1, 2, 3].every((i) => states[i] === 'Active');

async function waitAllActive(timeoutS) {
  const deadline = Date.now() + timeoutS * 1000;
  let last;
  while (Date.now() < deadline) {
    try { last = await view1(); if (allActive(last.states)) return { ok: true, ...last, ms: timeoutS * 1000 - (deadline - Date.now()) }; } catch (e) { last = { states: { error: String(e) }, b: {} }; }
    await sleep(100);
  }
  return { ok: false, ...last };
}

async function main() {
  await startOrigin();
  [1, 2, 3].forEach(startNode);
  const up = await waitAllActive(20);
  check(up.ok, `cluster did not become Active within 20 s: ${JSON.stringify(up.states)}`);
  if (!up.ok) return;
  log(`cluster up: ${JSON.stringify(up.states)} (burst=${BURST}, cycles=${CYCLES})`);
  const start = await view1();
  check(start.b.exhausted === 0 && start.b.dropped === 0, `clean start must show exhausted=0 dropped=0, got ${JSON.stringify(start.b)}`);

  const statusTally = {};
  for (let cycle = 1; cycle <= CYCLES && !failed; cycle++) {
    const before = (await view1()).b;
    const burstKeys = keysNotOwnedBy3(`burst/c${cycle}/k`, BURST);
    const warm = await Promise.all(burstKeys.map((k) => get(3, k)));
    check(warm.every((s) => s === 'MISS' || s === 'HIT' || s === 'COALESCED'), `cycle ${cycle}: warm GETs on node 3 failed: ${[...new Set(warm)]}`);
    const t0 = Date.now();
    const codes = await Promise.all(burstKeys.map((k) => del(1, k)));
    const burstMs = Date.now() - t0;
    for (const c of codes) statusTally[c] = (statusTally[c] || 0) + 1;
    check(codes.every((c) => c === 204 || c === 503), `cycle ${cycle}: DELETE statuses must be 204 (or retryable 503 Busy), got ${JSON.stringify(codes.reduce((a, c) => ((a[c] = (a[c] || 0) + 1), a), {}))}`);

    const settled = await waitAllActive(SETTLE_S);
    check(settled.ok, `cycle ${cycle}: node 1 does not see all nodes Active within ${SETTLE_S} s after the ${BURST}-way burst: ${JSON.stringify(settled.states)}`);

    // Let background fan-out finish, then every burst key must be gone from node 3.
    await sleep(500);
    const after = await Promise.all(burstKeys.map((k) => get(3, k)));
    const stale = after.filter((s) => s === 'HIT').length;
    check(stale === 0, `cycle ${cycle}: ${stale} of ${BURST} burst keys are still served as HIT by node 3 after DELETE`);

    // The next single DELETE must reach node 3.
    const probe = keysNotOwnedBy3(`burst/c${cycle}/probe`, 1)[0];
    await get(3, probe);
    const warmed = await get(3, probe);
    check(warmed === 'HIT', `cycle ${cycle}: probe key was not cached on node 3 (got ${warmed})`);
    const pc = await del(1, probe);
    check(pc === 204, `cycle ${cycle}: probe DELETE returned ${pc}`);
    await sleep(300);
    const probeAfter = await get(3, probe);
    check(probeAfter === 'MISS', `cycle ${cycle}: DELETE after the burst did not reach node 3 (GET there is ${probeAfter})`);

    const end = (await view1()).b;
    check(end.dropped === before.dropped, `cycle ${cycle}: broadcaster dropped moved ${before.dropped} -> ${end.dropped}`);
    check(end.exhausted === before.exhausted, `cycle ${cycle}: broadcaster exhausted moved ${before.exhausted} -> ${end.exhausted} on a healthy cluster`);
    log(`cycle ${cycle}/${CYCLES}: burst ${burstMs} ms, ${settled.ok ? `all Active after ${settled.ms} ms` : 'NOT all Active'}, stale-after-delete=${stale}, probe=${probeAfter}, broadcaster ${JSON.stringify(end)}`);
  }
  log(`DELETE status tally over all cycles: ${JSON.stringify(statusTally)}`);

  if (DO_EXHAUSTION && !failed) {
    const before = (await view1()).b;
    procs[3].kill();
    await sleep(500);
    const lost = keysNotOwnedBy3('burst/exhaust/k', 3);
    const codes = [];
    for (const k of lost) codes.push(await del(1, k));
    check(codes.every((c) => c === 204), `exhaustion phase: DELETEs with node 3 down should still be acknowledged by the primary, got ${codes}`);
    await sleep(3000);
    const after = (await view1()).b;
    log(`exhaustion phase (node 3 killed, ${lost.length} DELETEs): broadcaster before ${JSON.stringify(before)} after ${JSON.stringify(after)}`);
    check(after.exhausted - before.exhausted >= lost.length, `exhaustion phase: exhausted rose by ${after.exhausted - before.exhausted}, expected >= ${lost.length}`);
    check(after.dropped === before.dropped, 'exhaustion phase: dropped must keep meaning queue overflow (unchanged)');
  }
}

try {
  await main();
} catch (e) {
  failed = true;
  failures.push(String(e && e.stack ? e.stack : e));
  console.error('[BURST-CHECK-FAIL] unexpected error:', e);
} finally {
  for (const n of [1, 2, 3]) { try { procs[n]?.kill(); } catch { /* already gone */ } }
  if (originServer) originServer.close();
}

if (failed) {
  console.error(`[BURST-CHECK] FAILED (${failures.length} assertion(s)); logs in ${OUT}`);
  process.exit(1);
}
log(`PASSED: ${CYCLES} consecutive cycles of a ${BURST}-way burst; logs in ${OUT}`);
process.exit(0);
