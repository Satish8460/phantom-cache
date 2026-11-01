#!/usr/bin/env node
// scripts/cluster_deadpeer_storm_check.mjs
//
// Multi-process, assertion-based proof for PC-AUTO-043 / 044 (batch B1): with one cluster member dead, a
// storm of concurrent DELETEs must still purge the HEALTHY replica promptly, the memory of the gateway must
// stay bounded, the dead member must be reported as unreachable (not as a recovering one), and when it
// comes back the cluster returns to Active and invalidations reach it again.
//
//   1. start 3 gateways with mutual TLS and a counting origin; node 3 is then killed;
//   2. warm `--keys` keys on node 2, fire all `--keys` DELETEs at node 1 at once (statuses 204 or the
//      retryable 503 are both fine: the invalidation must be delivered either way);
//   3. assert node 2 no longer serves any of the first half of the keys 1 s after the storm ended, and
//      none of the keys 3 s after;
//   4. assert `dropped` stayed 0, `exhausted` is at most the number of keys sent to the dead member, node
//      1's working set grew by less than `--max-rss-growth-mb`, and node 1 reports node 3 as
//      `reachability: unreachable` (src inferred);
//   5. restart node 3: within `--recover-s` seconds node 1 sees it `reachable` and `Active`, and the next
//      DELETE reaches it.
//
// The script does not generate certificates. Pass a directory with: ca.crt, members.json,
// n1.crt n2.crt n3.crt and n1.p8.key n2.p8.key n3.p8.key (PKCS#8).
//
// Usage:
//   node scripts/cluster_deadpeer_storm_check.mjs --exe <gateway.exe> --certs <dir> [--keys 400] [--out-dir <dir>]
//        [--max-rss-growth-mb 150] [--recover-s 20] [--origin-port 19099] [--http-port-base 18185] [--cluster-port-base 19501]
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
const KEYS = Number(opt('keys', 400));
const MAX_RSS_MB = Number(opt('max-rss-growth-mb', 150));
const RECOVER_S = Number(opt('recover-s', 20));
const OUT = resolve(opt('out-dir', join(tmpdir(), 'phantom_deadpeer_storm_check')));
const ORIGIN_PORT = Number(opt('origin-port', 19099));
const HTTP_BASE = Number(opt('http-port-base', 18185));
const CLUSTER_BASE = Number(opt('cluster-port-base', 19501));
const usage = () => {
  console.error('usage: node scripts/cluster_deadpeer_storm_check.mjs --exe <gateway> --certs <dir> [--keys 400] [--out-dir <dir>] [--max-rss-growth-mb N] [--recover-s N] [--origin-port N] [--http-port-base N] [--cluster-port-base N]');
  process.exit(1);
};
if (!EXE || !CERTS || !existsSync(EXE) || !existsSync(join(CERTS, 'members.json'))) usage();
if (![KEYS, MAX_RSS_MB, RECOVER_S, ORIGIN_PORT, HTTP_BASE, CLUSTER_BASE].every((n) => Number.isFinite(n) && n > 0)) usage();

mkdirSync(OUT, { recursive: true });
const CLUSTER_ID = JSON.parse(readFileSync(join(CERTS, 'members.json'), 'utf8')).cluster_id;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (m) => console.log(`[STORM-CHECK] ${m}`);
const procs = {};
let originServer = null;
let failed = false;
const failures = [];
const check = (cond, msg) => {
  if (cond) return true;
  failed = true;
  failures.push(msg);
  console.error(`[STORM-CHECK-FAIL] ${msg}`);
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
const get = async (n, k) => { const r = await fetch(`${P(n)}/api/v1/cache/${k}`); await r.text(); return r.headers.get('x-cache-status'); };
const del = async (n, k) => { try { return (await fetch(`${P(n)}/api/v1/cache/${k}`, { method: 'DELETE' })).status; } catch { return 0; } };
async function view1() {
  const j = await (await fetch(`${P(1)}/api/v1/metrics`)).json();
  const c = j?.metrics?.cluster;
  if (!c?.broadcaster || !c.peers || !c.local) throw new Error('metrics.cluster missing or malformed');
  const peers = Object.fromEntries(c.peers.map((p) => [p.node_id, p]));
  const b = Object.fromEntries(Object.entries(c.broadcaster).map(([k, v]) => [k, v.v]));
  const rss = j?.metrics?.process?.rss_bytes?.v ?? null;
  return { local: c.local, peers, b, rss };
}
async function waitFor(pred, timeoutS) {
  const deadline = Date.now() + timeoutS * 1000;
  let last;
  while (Date.now() < deadline) {
    try { last = await view1(); if (pred(last)) return { ok: true, v: last }; } catch (e) { last = { error: String(e) }; }
    await sleep(200);
  }
  return { ok: false, v: last };
}

try {
  await startOrigin();
  [1, 2, 3].forEach(startNode);
  const up = await waitFor((v) => v.local.state?.v === 'Active' && [2, 3].every((i) => v.peers[i]?.state?.v === 'Active'), 20);
  check(up.ok, `cluster did not become Active within 20 s: ${JSON.stringify(up.v)}`);
  if (!failed) {
    const base = await view1();
    check(base.peers[3].reachability?.src === 'inferred', 'peer reachability must be labelled inferred');
    procs[3].kill();
    await sleep(800);

    const keys = [];
    for (let i = 0; keys.length < KEYS; i++) { const k = `deadstorm/k${i}`; if (primaryOf(k) !== 3 || i % 2 === 0) keys.push(k); }
    await Promise.all(keys.map((k) => get(2, k)));
    const rss0 = (await view1()).rss;
    const t0 = Date.now();
    const statuses = {};
    await Promise.all(keys.map((k) => del(1, k).then((s) => { statuses[s] = (statuses[s] || 0) + 1; })));
    log(`${KEYS} concurrent DELETEs on node 1 with node 3 dead took ${Date.now() - t0} ms; statuses ${JSON.stringify(statuses)}`);
    check(Object.keys(statuses).every((s) => s === '204' || s === '503'), `DELETE statuses must be 204 or retryable 503, got ${JSON.stringify(statuses)}`);

    await sleep(1000);
    const half = Math.floor(KEYS / 2);
    const first = await Promise.all(keys.slice(0, half).map((k) => get(2, k)));
    const stale1 = first.filter((s) => s === 'HIT').length;
    log(`+1 s after the storm: healthy node 2 still serves ${stale1}/${half} deleted keys`);
    check(stale1 === 0, `${stale1} of ${half} deleted keys still served as HIT by the healthy replica 1 s after the storm`);
    await sleep(2000);
    const rest = await Promise.all(keys.slice(half).map((k) => get(2, k)));
    const stale3 = rest.filter((s) => s === 'HIT').length;
    log(`+3 s after the storm: healthy node 2 still serves ${stale3}/${KEYS - half} deleted keys`);
    check(stale3 === 0, `${stale3} of ${KEYS - half} deleted keys still served as HIT by the healthy replica 3 s after the storm`);

    await sleep(2000);
    const after = await view1();
    log(`broadcaster after the storm: ${JSON.stringify(after.b)}; node 3 as seen by node 1: state=${after.peers[3].state.v} reachability=${after.peers[3].reachability?.v} failed_attempts=${after.peers[3].consecutive_failed_attempts?.v}; rss ${rss0} -> ${after.rss}`);
    check(after.b.dropped === 0, `dropped must stay 0 (queue overflow), got ${after.b.dropped}`);
    check(after.b.exhausted <= KEYS + 8, `exhausted ${after.b.exhausted} is more than the ${KEYS} deliveries owed to the dead member`);
    check(after.peers[3].reachability?.v === 'unreachable', `dead node 3 must be reported unreachable, got ${JSON.stringify(after.peers[3].reachability)} (state ${after.peers[3].state.v})`);
    check(after.peers[2].reachability?.v === 'reachable', `healthy node 2 must be reported reachable, got ${JSON.stringify(after.peers[2].reachability)}`);
    if (rss0 !== null && after.rss !== null) {
      const growthMb = (after.rss - rss0) / (1024 * 1024);
      check(growthMb < MAX_RSS_MB, `node 1 working set grew by ${growthMb.toFixed(1)} MiB (limit ${MAX_RSS_MB})`);
    } else {
      log('working set not available on this platform; memory growth not asserted');
    }

    // Recovery: node 3 returns.
    startNode(3);
    const rec = await waitFor((v) => v.peers[3]?.state?.v === 'Active' && v.peers[3]?.reachability?.v === 'reachable' && v.local.state?.v === 'Active', RECOVER_S);
    check(rec.ok, `after the restart node 1 does not see node 3 Active and reachable within ${RECOVER_S} s: ${JSON.stringify(rec.v?.peers?.[3])}`);
    if (rec.ok) {
      const probe = (() => { for (let i = 0; ; i++) { const k = `deadstorm/probe${i}`; if (primaryOf(k) !== 3) return k; } })();
      await get(3, probe);
      check((await get(3, probe)) === 'HIT', 'probe key was not cached on the restarted node 3');
      check((await del(1, probe)) === 204, 'probe DELETE should return 204');
      await sleep(500);
      check((await get(3, probe)) === 'MISS', 'the DELETE after the restart did not reach node 3');
      log('node 3 restarted: Active, reachable, and the next DELETE reached it');
    }
  }
} catch (e) {
  failed = true;
  failures.push(String(e && e.stack ? e.stack : e));
  console.error('[STORM-CHECK-FAIL] unexpected error:', e);
} finally {
  for (const n of [1, 2, 3]) { try { procs[n]?.kill(); } catch { /* already gone */ } }
  if (originServer) originServer.close();
}
if (failed) {
  console.error(`[STORM-CHECK] FAILED (${failures.length} assertion(s)); logs in ${OUT}`);
  process.exit(1);
}
log('PASSED');
process.exit(0);
