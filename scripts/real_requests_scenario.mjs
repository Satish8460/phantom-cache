#!/usr/bin/env node
// scripts/real_requests_scenario.mjs
//
// Node traffic scenario against a LIVE launcher cluster (secure or plain) with the real-content origin
// (`cluster_launcher.mjs up --origin-kind content` or `--secure`). Every response is verified against the origin's
// sha256 manifest (status, Content-Type, Content-Length, ETag, bytes, sha256). The origin counts its own requests.
//
//   node scripts/real_requests_scenario.mjs --dir <cluster dir> [--user tenantA] [--reps 3] [--sizes 1000,5000,10000]
//        [--out <result.json>] [--only stampede,warm,mixed,invalidation,peerfill,deadnode]
//
// All numbers are MEASURED by this client: release gateway, loopback, ONE host (gateways, origin and client share the CPU),
// Windows. Latency is client-side and closed-loop. They are not a benchmark of the product; repetition spread is shown,
// no confidence interval is claimed for n=3. Each run uses a fresh cache-key nonce (`?r=`) so every run starts cold.
// Exit code 0 only if there were zero errors and zero integrity failures and every stated expectation held.

import { spawn, spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ev, pickHeaders } from './lib/events.mjs';
import { clientStats, loadCluster, runLoad, httpRequest, cachePath, originStats, originManifest, gatewayOutcomes, rssKb, zipfSampler, sleep, getJson, verify } from './lib/traffic.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf(`--${n}`); return i !== -1 && argv[i + 1] !== undefined ? argv[i + 1] : d; };
const DIR = opt('dir');
if (!DIR) { console.error('usage: node scripts/real_requests_scenario.mjs --dir <cluster dir> [--user tenantA] [--reps 3] [--sizes 1000,5000,10000] [--out file] [--only a,b]'); process.exit(1); }
const st = loadCluster(DIR);
const USER = opt('user', 'tenantA');
const TOKEN = st.secure ? st.tokens[USER].token : null;
const REPS = Number(opt('reps', 3));
const SIZES = opt('sizes', '1000,5000,10000').split(',').map(Number);
const ONLY = opt('only', 'gallery,stampede,warm,mixed,invalidation,peerfill,deadnode,deletes,offload,workloads,burstdelete,peerexpiry').split(',');
const OUT = opt('out', join(DIR, 'real_requests_result.json'));
const RUN = Date.now().toString(36);
const launcher = (args) => spawnSync(process.execPath, [join(HERE, 'cluster_launcher.mjs'), ...args, '--dir', DIR], { encoding: 'utf8' });

const results = { scenario: 'real_requests', startedAt: new Date().toISOString(), host: { os: process.platform, node: process.version, note: 'loopback, one host: gateways, origin and this client share the machine' }, cluster: { nodes: st.nodes, secure: st.secure, user: st.secure ? `${USER} (tenant ${st.tokens[USER].claims.tenant_id})` : 'none (plain cluster)', gatewayExe: st.exe, originKind: st.originKind }, phases: {}, checks: [] };
const fmtB = (n) => (n < 1024 ? n + ' B' : n < 1048576 ? (n / 1024).toFixed(0) + ' KB' : (n / 1048576).toFixed(2) + ' MiB');
const who = () => (st.secure ? st.tokens[USER].label : 'no token');
const node = (port) => `node ${st.ports.indexOf(port) + 1}`;
const reqEv = (port, path, r, extra = {}) => ({ request: { method: 'GET', path: '/api/v1/cache' + path, node: st.ports.indexOf(port) + 1, as: who() }, response: { status: r.status || 0, ms: +(r.ms || 0).toFixed(1), bytes: r.bytes, headers: pickHeaders(r.headers) , ...(r.error ? { error: r.error } : {}) }, ...extra });
const line = (r) => (r.error ? `error ${r.error}` : `${r.status} ${r.headers['x-cache-status'] || ''} source ${r.headers['x-cache-source'] || '-'}, ${r.ms.toFixed(0)} ms, ${fmtB(r.bytes)}`);
const check = (id, ok, detail, means) => { ev.check(ok, id, { detail, means }); results.checks.push({ id, status: ok ? 'PASS' : 'FAIL', detail }); console.log(`[${ok ? 'PASS' : 'FAIL'}] ${id}${detail ? ' :: ' + detail : ''}`); return ok; };
const pids = () => Array.from({ length: st.nodes }, (_, i) => st.pids[`node${i + 1}`]);
const rssAll = () => pids().map((p) => rssKb(p));

async function originDelta(fn) {
  const a = await originStats(st.originPort);
  const r = await fn();
  const b = await originStats(st.originPort);
  return { result: r, originRequests: b.requests - a.requests };
}
const rr = (ports) => (i, dead, tried) => { const live = ports.filter((p) => !dead.has(p) && !tried.includes(p)); return live[i % live.length]; };
const summarise = (r) => ({ total: r.total, ok: r.ok, errors: r.errors, integrityFailures: r.integrityFailures, sources: r.sources, xCacheStatus: r.xCacheStatus, bytes: r.bytes, seconds: r.seconds, rps: r.rps, mbPerSec: r.mbPerSec, latencyMs: r.latencyMs, errorSamples: r.errorSamples, integritySamples: r.integritySamples, perPort: r.perPort, retried: r.retried });
const hitRatio = (src, total) => ((((src.local || 0) + (src.coalesced || 0) + (src.peer || 0)) / total) * 100).toFixed(1) + '% (local+peer+coalesced)';

const sampleEv = ({ port, path, r, problems }) => ev.observe(`sampled response: GET ${path} via ${node(port)} -> ${line(r)}${problems.length ? ' PROBLEM ' + problems.join(', ') : ', sha256 OK'}`, reqEv(port, path, r, { means: problems.length ? 'This response did not match the origin bytes.' : 'One of every few dozen responses is shown; every response is checked, not only these.' }));
const tickEv = (size) => (done, res) => ev.progress(done, size, { sent: done, errors: res.errors, integrityFailures: res.integrityFailures, ...res.sources });

const distinct0 = (plan) => new Set(plan.map((p) => p.path)).size;
async function main() {
  const manifest = await originManifest(st.originPort);
  if (!manifest || manifest.error) { console.error('cannot read the origin manifest (is this a content-origin cluster?)', manifest); process.exit(1); }
  const keys = Object.keys(manifest);
  results.origin = { objects: keys.length, totalBytes: keys.reduce((a, k) => a + manifest[k].bytes, 0) };
  const ports = st.ports;
  results.rssKbBefore = rssAll();
  console.log(`[real_requests] cluster ${ports.join(',')} user=${results.cluster.user} objects=${keys.length} run=${RUN}`);
  let totalReq = 0, totalErr = 0, totalInteg = 0, totalBytes = 0;
  const tally = (r) => { totalReq += r.total; totalErr += r.errors; totalInteg += r.integrityFailures; totalBytes += r.bytes; };

  // ---- 0. gallery: cold vs warm, real PNGs
  if (ONLY.includes('gallery')) {
    const imgs = keys.filter((k) => k.startsWith('/img/') && !k.includes('huge')).slice(0, 12);
    const nonce = `${RUN}-gal`;
    const pass = {};
    for (const label of ['first load (cold)', 'second load (warm)']) {
      ev.action(`Loading ${imgs.length} real PNG images through node 1 as ${who()}: ${label}`, { means: label.includes('cold') ? 'Nobody has asked for these pictures yet, so the gateway must fetch each from the origin.' : 'The same pictures again: the gateway should now answer from its own memory.' });
      const o = await originDelta(async () => {
        const out = [];
        for (const k of imgs) {
          const r = await httpRequest({ port: ports[0], path: cachePath(k, nonce), token: TOKEN });
          const bad = verify(manifest[k], r);
          ev.observe(`GET ${k} -> ${line(r)}, sha256 ${bad.length ? 'MISMATCH' : 'OK'}`, reqEv(ports[0], k, r, { means: r.headers && r.headers['x-cache-source'] === 'origin' ? 'Fetched from the slow origin and stored.' : 'Served from the gateway without asking the origin.' }));
          out.push({ k, r, bad });
          totalReq += 1; totalBytes += r.bytes || 0; if (r.error || r.status !== 200) totalErr += 1; if (bad.length) totalInteg += 1;
        }
        return out;
      });
      pass[label] = { originRequests: o.originRequests, sources: o.result.reduce((a, x) => { const s2 = x.r.headers['x-cache-source'] || 'none'; a[s2] = (a[s2] || 0) + 1; return a; }, {}), integrityOk: o.result.every((x) => !x.bad.length), meanMs: +(o.result.reduce((a, x) => a + x.r.ms, 0) / o.result.length).toFixed(1) };
      ev.observe(`${label}: origin asked ${o.originRequests} times, sources ${JSON.stringify(pass[label].sources)}, mean ${pass[label].meanMs} ms per image`);
    }
    const c = pass['first load (cold)'], w = pass['second load (warm)'];
    results.phases.gallery = pass;
    check('gallery: cold load asks the origin once per image, warm load asks it 0 times, every image byte-identical', c.originRequests === imgs.length && w.originRequests === 0 && c.integrityOk && w.integrityOk, `cold origin ${c.originRequests} (mean ${c.meanMs} ms), warm origin ${w.originRequests} (mean ${w.meanMs} ms)`, 'The first visit is slow because the origin is involved; every later visit is fast and costs the origin nothing.');
  }

  // ---- 1. cold stampede
  if (ONLY.includes('stampede')) {
    const A = '/img/tile-18.png', B = '/img/tile-22.png';
    ev.action(`Sending 300 simultaneous GETs for ${A} (${fmtB(manifest[A].bytes)}, never requested before) to node 1 as ${who()}`, { means: 'A crowd asks for the same new picture at the same moment.' });
    const a = await originDelta(() => runLoad(Array.from({ length: 300 }, () => ({ path: A, port: ports[0] })), { concurrency: 300, manifest, nonce: `${RUN}-sa`, token: TOKEN }));
    tally(a.result);
    ev.observe(`the origin was asked ${a.originRequests} time(s); the 300 answers came from: ${JSON.stringify(a.result.sources)}; slowest ${a.result.latencyMs.max} ms`, { means: 'Only one request went to the slow server; the others waited for that single answer (coalesced).' });
    ev.action(`Sending 300 simultaneous GETs for ${B} spread over all 3 nodes`);
    const b = await originDelta(() => runLoad(Array.from({ length: 300 }, (_, i) => ({ path: B, port: ports[i % ports.length] })), { concurrency: 300, manifest, nonce: `${RUN}-sb`, token: TOKEN }));
    tally(b.result);
    ev.observe(`the origin was asked ${b.originRequests} time(s); sources: ${JSON.stringify(b.result.sources)}`, { means: 'Even with three gateways, the crowd still cost the origin one request.' });
    results.phases.stampede = {
      oneNode: { image: A, bytesEach: manifest[A].bytes, originRequests: a.originRequests, ...summarise(a.result) },
      threeNodes: { image: B, bytesEach: manifest[B].bytes, originRequests: b.originRequests, ...summarise(b.result) },
    };
    check('stampede: 300 concurrent cold GETs on ONE node => 1 origin request', a.originRequests === 1 && a.result.errors === 0 && a.result.integrityFailures === 0, `origin requests ${a.originRequests}, sources ${JSON.stringify(a.result.sources)}`);
    // Coalescing is per gateway: each of the 3 gateways may fetch once when the key is cold everywhere, so the bound is one origin request
    // per node (3), not 1 (REV-008: observed 1 in one run and 3 in another with the same binary).
    check('stampede: 300 concurrent cold GETs spread over 3 nodes => origin requests (expect between 1 and 3, one per node at most)', b.originRequests >= 1 && b.originRequests <= 3 && b.result.errors === 0 && b.result.integrityFailures === 0, `origin requests ${b.originRequests}, sources ${JSON.stringify(b.result.sources)}`);
  }

  // ---- 2. warm hits
  if (ONLY.includes('warm')) {
    const P = '/img/tile-12.png';
    await httpRequest({ port: ports[0], path: cachePath(P, `${RUN}-w`), token: TOKEN });
    ev.action(`Warm hits: 1,000 GETs of ${P} on node 1 (already cached)`);
    const w = await originDelta(() => runLoad(Array.from({ length: 1000 }, () => ({ path: P, port: ports[0] })), { concurrency: 100, manifest, nonce: `${RUN}-w`, token: TOKEN }));
    tally(w.result);
    ev.observe(`origin asked ${w.originRequests} times; sources ${JSON.stringify(w.result.sources)}; p50 ${w.result.latencyMs.p50} ms (client-side)`, { means: 'Once cached, the origin is not involved at all.' });
    results.phases.warm = { image: P, originRequests: w.originRequests, ...summarise(w.result) };
    check('warm: 1,000 GETs of a cached 153 KB image => 0 origin requests, all local hits', w.originRequests === 0 && w.result.errors === 0 && w.result.integrityFailures === 0 && w.result.sources.local === 1000, `origin ${w.originRequests}, sources ${JSON.stringify(w.result.sources)}, p50/p99 ${w.result.latencyMs.p50}/${w.result.latencyMs.p99} ms`);
  }

  // ---- 3. mixed Zipf workload
  if (ONLY.includes('mixed')) {
    results.phases.mixed = [];
    const conc = { 1000: 100, 5000: 250, 10000: 500 };
    for (const size of SIZES) {
      const runs = [];
      for (let rep = 1; rep <= REPS; rep++) {
        const pick = zipfSampler(keys, { s: 1.0, seed: 1000 + rep });
        const plan = Array.from({ length: size }, () => ({ path: pick() }));
        const nonce = `${RUN}-m${size}-${rep}`;
        const rssBefore = rssAll();
        ev.action(`Sending ${size} requests at concurrency ${conc[size] || 250} (repetition ${rep} of ${REPS}) over a Zipf-like mix of ${distinct0(plan)} real objects as ${who()}, spread over the 3 nodes`, { means: 'A few popular objects get most requests, as on a real site. Every response is checked against the origin sha256.' });
        const m = await originDelta(() => runLoad(plan, { concurrency: conc[size] || 250, manifest, nonce, token: TOKEN, pickPort: rr(ports), ports, sampleEvery: Math.max(50, Math.round(size / 20)), onSample: sampleEv, onTick: tickEv(size) }));
        const rssAfter = rssAll();
        tally(m.result);
        const distinct = new Set(plan.map((p) => p.path)).size;
        ev.observe(`${size} requests done in ${m.result.seconds} s: ${m.result.rps} req/s, errors ${m.result.errors}, integrity failures ${m.result.integrityFailures}, origin asked ${m.originRequests} times, sources ${JSON.stringify(m.result.sources)}, p50/p95/p99 ${m.result.latencyMs.p50}/${m.result.latencyMs.p95}/${m.result.latencyMs.p99} ms`, { means: 'The origin was asked far less often than the number of requests: that is the saving.' });
        runs.push({ rep, distinctKeys: distinct, originRequests: m.originRequests, hitRatio: hitRatio(m.result.sources, size), rssKbBefore: rssBefore, rssKbAfter: rssAfter, ...summarise(m.result) });
        console.log(`  mixed ${size} @${conc[size] || 250} rep ${rep}: ${m.result.rps} req/s, errors ${m.result.errors}, integrity ${m.result.integrityFailures}, origin ${m.originRequests} (distinct keys ${distinct}), p50/p95/p99 ${m.result.latencyMs.p50}/${m.result.latencyMs.p95}/${m.result.latencyMs.p99} ms, sources ${JSON.stringify(m.result.sources)}`);
      }
      const rps = runs.map((x) => x.rps), p99 = runs.map((x) => x.latencyMs.p99);
      const agg = { size, concurrency: conc[size] || 250, reps: REPS, rps: { min: Math.min(...rps), mean: +(rps.reduce((a, b) => a + b, 0) / rps.length).toFixed(1), max: Math.max(...rps) }, p99Ms: { min: Math.min(...p99), max: Math.max(...p99) }, runs };
      results.phases.mixed.push(agg);
      check(`mixed ${size} @${agg.concurrency} x${REPS}: zero errors, zero integrity failures`, runs.every((x) => x.errors === 0 && x.integrityFailures === 0), `req/s min/mean/max ${agg.rps.min}/${agg.rps.mean}/${agg.rps.max}`);
    }
  }

  // ---- 4. invalidation
  if (ONLY.includes('invalidation')) {
    const P = '/img/tile-07.png', nonce = `${RUN}-inv`;
    const get = (port) => httpRequest({ port, path: cachePath(P, nonce), token: TOKEN });
    ev.action(`Warming ${P} on every node (first read on each)`);
    const g = []; for (const p of ports) { const r = await get(p); g.push(r); ev.observe(`GET ${P} via ${node(p)} -> ${line(r)}`, reqEv(p, P, r)); }
    const warmed = ports.map((p, i) => `${p}:${g[i].headers['x-cache-status']}/${g[i].headers['x-cache-source']}`);
    const g2 = []; for (const p of ports) g2.push(await get(p));
    const allHit = g2.every((r) => r.headers['x-cache-status'] === 'HIT');
    const o0 = (await originStats(st.originPort)).requests;
    ev.observe(`second read on every node: all HIT = ${allHit}`, { means: 'All three gateways now hold a copy.' });
    ev.action(`DELETE ${P} on node 1 as ${who()}`, { request: { method: 'DELETE', path: '/api/v1/cache' + P, node: 1, as: who() } });
    const del = await httpRequest({ port: ports[0], method: 'DELETE', path: cachePath(P, nonce), token: TOKEN });
    const t0 = performance.now();
    ev.observe(`DELETE answered ${del.status}`, { response: { status: del.status, ms: +del.ms.toFixed(1) }, means: 'The gateway accepted the purge and told the other gateways.' });
    const afterOnDeleter = await get(ports[0]);
    ev.observe(`node 1 GET after the DELETE -> ${line(afterOnDeleter)}`, { means: 'MISS: the deleted copy is gone on the gateway that took the DELETE.' });
    const lag = {};
    for (const p of ports.slice(1)) {
      let ms = null, last = null;
      while (performance.now() - t0 < 5000) {
        last = await get(p);
        if (last.headers['x-cache-status'] !== 'HIT') { ms = Math.round(performance.now() - t0); break; }
        await sleep(25);
      }
      ev.observe(ms === null ? `${node(p)} was still serving the deleted entry after 5 s` : `${node(p)} stopped serving the entry after ${ms} ms`, { means: ms === null ? 'FINDING: there is no guaranteed bound on how long another gateway can keep a purged entry.' : 'Measured from the DELETE answer; one run on loopback, not a guarantee.' });
      lag[p] = { msUntilNoStaleHit: ms, last: last && `${last.headers['x-cache-status']}/${last.headers['x-cache-source']}` };
    }
    const o1 = (await originStats(st.originPort)).requests;
    results.phases.invalidation = { path: P, warmedFirstGet: warmed, allThreeHitBeforeDelete: allHit, deleteStatus: del.status, afterDeleteOnDeletingNode: `${afterOnDeleter.headers['x-cache-status']}/${afterOnDeleter.headers['x-cache-source']}`, otherNodes: lag, originRequestsAfterDelete: o1 - o0 };
    check('invalidation: DELETE answered 2xx', del.status >= 200 && del.status < 300, `status ${del.status}`);
    check('invalidation: first GET after DELETE on the deleting gateway is a MISS (read-your-delete, one gateway)', afterOnDeleter.headers['x-cache-status'] === 'MISS', `${afterOnDeleter.headers['x-cache-status']}/${afterOnDeleter.headers['x-cache-source']}`);
    for (const p of ports.slice(1)) check(`invalidation: peer ${p} stops serving the deleted entry (propagation, measured from the DELETE answer)`, lag[p].msUntilNoStaleHit !== null, lag[p].msUntilNoStaleHit !== null ? `${lag[p].msUntilNoStaleHit} ms` : `still HIT after 5 s: FINDING, no cross-gateway staleness bound (docs/user/limitations.md)`);
  }

  // ---- 5. peer fill
  if (ONLY.includes('peerfill')) {
    const set = keys.filter((k) => !k.includes('huge')).filter((_, i) => i % 13 === 0).slice(0, 24);
    const tenant = st.secure ? st.tokens[USER].claims.tenant_id : '__default__';
    // Rendezvous (HRW) primary of a key: FNV-1a(key bytes) then node id byte (src/dist/router.rs:952-963); key string
    // "tenant:path:?query" (src/core/cache.rs:98). Verified below by the observed sources, not assumed.
    const M = (1n << 64n) - 1n;
    const hrw = (key, id) => { let h = 0xcbf29ce484222325n; for (const c of Buffer.from(key)) { h ^= BigInt(c); h = (h * 0x100000001b3n) & M; } h ^= BigInt(id); return (h * 0x100000001b3n) & M; };
    const primaryOf = (k, nonce) => { let best = null; for (let id = 1; id <= st.nodes; id++) { const w = hrw(`${tenant}:${k}:?r=${nonce}`, id); if (best === null || w > best.w) best = { id, w }; } return best.id; };
    const run = async (variant, nonce, primaryFirst) => {
      const seq = { first: {}, second: {}, third: {} };
      let integ = 0;
      const o = await originDelta(async () => {
        for (const k of set) {
          const pr = primaryOf(k, nonce);
          const order = primaryFirst ? [pr, ...[1, 2, 3].filter((x) => x !== pr)] : [1, 2, 3];
          for (let j = 0; j < 3; j++) {
            const r = await httpRequest({ port: ports[order[j] - 1], path: cachePath(k, nonce), token: TOKEN });
            const slot = ['first', 'second', 'third'][j]; const sname = (r.headers && r.headers['x-cache-source']) || 'none';
            seq[slot][sname] = (seq[slot][sname] || 0) + 1;
            if (verify(manifest[k], r).length) integ += 1;
          }
        }
      });
      totalReq += set.length * 3; totalInteg += integ;
      return { variant, keys: set.length, requestsPerKey: 3, sourceByRequestOrder: seq, originRequests: o.originRequests, integrityFailures: integ };
    };
    const A = await run('primary read first, then the two other nodes', `${RUN}-pfA`, true);
    const B = await run('node 1, 2, 3 in order regardless of primary', `${RUN}-pfB`, false);
    results.phases.peerFill = { primaryFirst: A, fixedOrder: B, note: 'a non-primary that misses asks the primary of the key; if the primary holds nothing the non-primary fetches from the origin itself (metrics.outcomes.peer_reads_without_hit.miss) and the primary is NOT populated' };
    check(`peer fill (primary read first): ${set.length} keys read on 3 nodes => origin requests == ${set.length}, the other ${set.length * 2} reads filled from the peer`, A.originRequests === set.length && A.integrityFailures === 0 && (A.sourceByRequestOrder.second.peer || 0) === set.length && (A.sourceByRequestOrder.third.peer || 0) === set.length, `origin requests ${A.originRequests}; sources by order ${JSON.stringify(A.sourceByRequestOrder)}`);
    check(`peer fill (fixed order node 1,2,3): observed, reported as measured behaviour (origin requests would be ${set.length} if every read after the first were a peer fill)`, B.integrityFailures === 0, `origin requests ${B.originRequests} for ${set.length * 3} reads; sources by order ${JSON.stringify(B.sourceByRequestOrder)}`);
    // the over-1-MiB object
    const H = '/img/huge-1.2mb.png';
    const hs = await originDelta(async () => { const rs = []; for (const p of ports) rs.push(await httpRequest({ port: p, path: cachePath(H, `${RUN}-h`), token: TOKEN })); return rs; });
    totalReq += 3;
    const hv = hs.result.map((r, i) => ({ port: ports[i], status: r.status, source: r.headers && r.headers['x-cache-source'], integrity: verify(manifest[H], r) }));
    results.phases.hugeObject = { path: H, bytes: manifest[H].bytes, originRequests: hs.originRequests, reads: hv, note: 'the peer-fill frame limit is 1 MiB (docs/user/limitations.md); an object above it cannot be filled from a peer' };
    totalBytes += hs.result.reduce((a, r) => a + (r.bytes || 0), 0);
    check('huge object (1.17 MiB): served byte-identical on every node', hv.every((x) => x.status === 200 && x.integrity.length === 0), `origin requests for 3 reads: ${hs.originRequests}; sources ${hv.map((x) => x.source).join(',')}`);
    if (hs.originRequests > 1) console.log(`  [observation] above 1 MiB the origin was asked ${hs.originRequests} times for 3 node reads (peer fill not used).`);
  }

  // ---- 6. dead node mid-run, then recovery
  if (ONLY.includes('deadnode')) {
    const killAt = 1500, size = 5000;
    const pick = zipfSampler(keys, { s: 1.0, seed: 777 });
    const plan = Array.from({ length: size }, () => ({ path: pick() }));
    let killed = false, killedAt = null;
    const t0 = Date.now();
    ev.action(`Sending ${size} requests at concurrency 100 over 3 nodes; node ${st.nodes} will be KILLED after ${killAt} requests`, { means: 'We stop a gateway while traffic is flowing, like a crashed machine.' });
    const m = await runLoad(plan, { concurrency: 100, sampleEvery: 250, onSample: sampleEv, onTick: tickEv(size), manifest, nonce: `${RUN}-dead`, token: TOKEN, pickPort: rr(ports), ports, failover: true, onProgress: (done) => {
      if (!killed && done >= killAt) { killed = true; killedAt = Date.now() - t0; ev.action(`KILLED node ${st.nodes} (process terminated) after ${killAt} requests`); spawn(process.execPath, [join(HERE, 'cluster_launcher.mjs'), 'stop-node', '--dir', DIR, '--node', String(st.nodes)], { stdio: 'ignore' }); }
    } });
    tally(m);
    const deadPort = ports[st.nodes - 1];
    const healthyErrors = Object.entries(m.perPort).filter(([p]) => Number(p) !== deadPort).reduce((a, [, v]) => a + v.errors, 0);
    results.phases.deadNode = { killedNode: st.nodes, killedAfterRequests: killAt, killedAtMsIntoRun: killedAt, clientFailoverRetries: m.retried, requestsServedByDeadNodeBeforeKill: (m.perPort[deadPort] || {}).n || 0, errorsOnHealthyNodes: healthyErrors, ...summarise(m) };
    check(`dead node: node ${st.nodes} killed after ${killAt} of ${size} requests; healthy nodes returned 0 errors and byte-identical bodies`, m.errors === 0 && m.integrityFailures === 0 && healthyErrors === 0, `final errors ${m.errors}, integrity ${m.integrityFailures}, client retries after connection loss ${m.retried}`);
    await sleep(1500);
    ev.observe(`traffic finished: ${m.errors} errors, ${m.integrityFailures} integrity failures, ${m.retried} requests retried on another node by the client after the connection was lost`, { means: 'The healthy gateways kept answering correctly; the client moved its requests to them.' });
    const v = await gatewayView(ports[0]);
    ev.observe(`node 1 now sees: ${JSON.stringify(v)}`, { means: 'Recovering/degraded or unreachable means the cluster has noticed node 3 is gone.' });
    ev.action(`Restarting node ${st.nodes}`);
    results.phases.deadNode.viewFromNode1WhileDead = v;
    // recovery
    launcher(['start-node', '--node', String(st.nodes)]);
    let active = false; let view = null;
    for (let i = 0; i < 100 && !active; i++) { await sleep(300); view = await gatewayView(ports[0]); active = view && Object.keys(view).length === st.nodes && Object.values(view).every((x) => x.state === 'Active'); }
    const pick2 = zipfSampler(keys, { s: 1.0, seed: 778 });
    const plan2 = Array.from({ length: 1000 }, () => ({ path: pick2() }));
    const rec = await runLoad(plan2, { concurrency: 100, manifest, nonce: `${RUN}-rec`, token: TOKEN, pickPort: rr(ports), ports });
    tally(rec);
    results.phases.recovery = { allActiveInNode1View: active, view, ...summarise(rec) };
    ev.observe(`node ${st.nodes} restarted: members seen by node 1 = ${JSON.stringify(view)}`);
    check('recovery: node restarted, every member Active in node 1 view', active, JSON.stringify(view));
    check('recovery: 1,000 requests over all 3 nodes, 0 errors, 0 integrity failures', rec.errors === 0 && rec.integrityFailures === 0, `node ${st.nodes} sources ${JSON.stringify((rec.perPort[deadPort] || {}).sources)}`);
  }

  if (ONLY.includes('offload')) {
    const N = 3000, pick = zipfSampler(keys.filter((k) => !k.includes('huge')), { s: 1.0, seed: 4242 });
    const plan = Array.from({ length: N }, () => ({ path: pick() }));
    ev.action(`Origin offload: ${N} client requests at concurrency 100 over a popularity mix; the ORIGIN counts what actually reaches it`, { means: 'Compare what the clients asked for with what the slow server had to do.' });
    const o = await originDelta(() => runLoad(plan, { concurrency: 100, manifest, nonce: `${RUN}-off`, token: TOKEN, pickPort: rr(ports), ports, sampleEvery: 300, onSample: sampleEv, onTick: tickEv(N) }));
    tally(o.result);
    const saved = (100 * (1 - o.originRequests / N)).toFixed(1);
    ev.observe(`clients sent ${N} requests; the origin counted ${o.originRequests}; so ${saved}% of the work never reached the origin`, { means: 'The saving is the gateway doing the remembering.' });
    results.phases.offload = { clientRequests: N, originRequests: o.originRequests, savedPercent: Number(saved), ...summarise(o.result) };
    check(`offload: ${N} requests, origin counted ${o.originRequests} (${saved}% saved), 0 errors, 0 integrity failures`, o.result.errors === 0 && o.result.integrityFailures === 0 && o.originRequests < N * 0.5, `origin counted ${o.originRequests}`, 'The origin did a fraction of the work.');
  }

  if (ONLY.includes('workloads')) {
    const small = keys.filter((k) => k.startsWith('/msg/') || k.startsWith('/text/'));
    const shapes = [
      ['hot keys (10 keys get all requests)', () => Array.from({ length: 1500 }, (_, i) => ({ path: small[i % 10] })), 'very high hit ratio'],
      ['popularity mix (Zipf over 260 objects)', (pick = zipfSampler(small, { s: 1.0, seed: 9 })) => Array.from({ length: 1500 }, () => ({ path: pick() })), 'high hit ratio'],
      ['all different keys (a scan)', () => Array.from({ length: 300 }, (_, i) => ({ path: small[i % 260], uniq: i })), 'no sharing is possible, so most go to the origin'],
      ['burst (100 different keys at once)', () => Array.from({ length: 100 }, (_, i) => ({ path: small[i] })), 'cold burst: all origin, then fast'],
    ];
    results.phases.workloads = [];
    for (const [name, mk, expect] of shapes) {
      const plan = mk(); const nonce = `${RUN}-wl${results.phases.workloads.length}`;
      ev.action(`Workload shape: ${name} (${plan.length} requests, concurrency 50)`, { means: 'Expected: ' + expect + '.' });
      const p2 = plan.map((x) => ({ path: x.path }));
      const o = await originDelta(() => runLoad(p2, { concurrency: 50, manifest, nonce, token: TOKEN, pickPort: rr(ports), ports }));
      tally(o.result);
      const hr = (((o.result.sources.local || 0) + (o.result.sources.peer || 0) + (o.result.sources.coalesced || 0)) / plan.length * 100).toFixed(1);
      ev.observe(`${name}: hit ratio ${hr}% (derived), origin asked ${o.originRequests} times for ${plan.length} requests, p50 ${o.result.latencyMs.p50} ms, errors ${o.result.errors}`);
      results.phases.workloads.push({ name, hitRatio: Number(hr), originRequests: o.originRequests, errors: o.result.errors, integrityFailures: o.result.integrityFailures });
    }
    const w = results.phases.workloads;
    check('workloads: four shapes with 0 errors and 0 integrity failures; hot keys hit far more than a scan', w.every((x) => x.errors === 0 && x.integrityFailures === 0) && w[0].hitRatio > w[2].hitRatio + 40, w.map((x) => x.name.split(' (')[0] + ' ' + x.hitRatio + '%').join(', '), 'Hit ratio depends on the shape of the traffic, and the gateway helps most where keys repeat.');
    ev.observe('A slow-origin shape is covered by the Quick benchmark card (origin profile with 100 ms service time).');
  }

  if (ONLY.includes('burstdelete')) {
    const N = 200, nonce = `${RUN}-bd`; const ks = keys.filter((k) => k.startsWith('/msg/')).slice(0, N);
    ev.action(`Warming ${N} keys on node 1 and node 2, then sending ${N} concurrent DELETEs through node 1`, { means: 'A purge storm with every gateway alive: the cluster must stay healthy (an old bug left a node stuck in Recovering).' });
    for (const k of ks) { await httpRequest({ port: ports[0], path: cachePath(k, nonce), token: TOKEN }); await httpRequest({ port: ports[1], path: cachePath(k, nonce), token: TOKEN }); }
    const dels = await Promise.all(ks.map((k) => httpRequest({ port: ports[0], method: 'DELETE', path: cachePath(k, nonce), token: TOKEN })));
    const codes = {}; for (const d of dels) { const c = d.error || d.status; codes[c] = (codes[c] || 0) + 1; }
    ev.observe(`${N} DELETEs answered: ${JSON.stringify(codes)}`);
    let act = false, vw = null; for (let i = 0; i < 40 && !act; i++) { await sleep(300); vw = await gatewayView(ports[0]); act = vw && Object.keys(vw).length === st.nodes && Object.values(vw).every((x) => x.state === 'Active'); }
    ev.observe(`members as seen by node 1 after the burst: ${JSON.stringify(vw)}`, { means: 'All Active means the storm did not unsettle the cluster.' });
    const fk = new Map(); const tb = performance.now();
    for (let round = 0; round < 60 && fk.size < ks.length; round++) { for (const k of ks) { if (fk.has(k)) continue; const r = await httpRequest({ port: ports[1], path: cachePath(k, nonce), token: TOKEN }); if (r.headers && r.headers['x-cache-status'] !== 'HIT') fk.set(k, Math.round(performance.now() - tb)); } if (fk.size < ks.length) await sleep(400); }
    const fv = [...fk.values()].sort((x, y) => x - y); const bad = fk.size === ks.length ? 'MISS' : 'HIT';
    ev.observe(fk.size === ks.length ? `node 2 stopped serving all ${ks.length} deleted keys: first after ${fv[0]} ms, median ${fv[fv.length >> 1]} ms, last after ${fv[fv.length - 1]} ms` : `node 2 still served ${ks.length - fk.size} deleted keys after 24 s`, { means: 'Purges are delivered to the other gateways at a measured pace.' });
    check(`burst of ${N} concurrent DELETEs: all acknowledged and every member Active afterwards`, dels.every((d) => d.status >= 200 && d.status < 300) && act, `codes ${JSON.stringify(codes)}`, 'Purge storms do not leave a gateway stuck.');
    check('after the burst a purged key is not served stale by another gateway', bad !== 'HIT', 'node 2: ' + bad);
  }

  if (ONLY.includes('peerexpiry')) {
    const P = '/short/001.json', nonce = `${RUN}-pe`;
    ev.action(`Reading ${P} (lifetime 8 seconds) on node 1, then node 2, then node 3`, { means: 'One gateway fetches from the origin; the others may get a copy from a neighbour.' });
    for (const p of ports) { const r = await httpRequest({ port: p, path: cachePath(P, nonce), token: TOKEN }); ev.observe(`GET ${P} via ${node(p)} -> ${line(r)}`, reqEv(p, P, r)); }
    ev.observe('Waiting 10 seconds for the object to expire everywhere...', { means: 'A copy given by a neighbour must not live longer than its source.' });
    await sleep(10000);
    const after = []; for (const p of ports) { const r = await httpRequest({ port: p, path: cachePath(P, nonce), token: TOKEN }); after.push(r); ev.observe(`after expiry: GET ${P} via ${node(p)} -> ${line(r)}`, reqEv(p, P, r)); }
    check('peer expiry: after 10 s no gateway serves the 8-second object as a plain HIT (copies expired with their source)', after.every((r) => r.headers['x-cache-status'] !== 'HIT'), after.map((r) => r.headers['x-cache-status'] + '/' + r.headers['x-cache-source']).join(', '), 'A neighbour\'s copy does not outlive the original.');
    totalReq += 6;
  }

  if (ONLY.includes('deletes')) {
    const N = Number(opt('deletes', 400)), nonce = `${RUN}-del`;
    const ks = Array.from({ length: N }, (_, i) => keys.filter((k) => k.startsWith('/msg/') || k.startsWith('/text/'))[i % 260]).map((k, i) => k + (i >= 260 ? '' : ''));
    ev.action(`Warming ${N} small messages on node 1 and node 2`);
    for (const k of ks) { await httpRequest({ port: ports[0], path: cachePath(k, nonce), token: TOKEN }); await httpRequest({ port: ports[1], path: cachePath(k, nonce), token: TOKEN }); }
    ev.action(`KILLED node ${st.nodes}, then sending ${N} concurrent DELETEs through node 1`, { means: 'A purge storm while one gateway is down.' });
    spawnSync(process.execPath, [join(HERE, 'cluster_launcher.mjs'), 'stop-node', '--dir', DIR, '--node', String(st.nodes)], { encoding: 'utf8' });
    await sleep(300);
    const t0 = performance.now();
    const dels = await Promise.all(ks.map((k) => httpRequest({ port: ports[0], method: 'DELETE', path: cachePath(k, nonce), token: TOKEN })));
    const codes = {}; for (const d of dels) { const c = d.error || d.status; codes[c] = (codes[c] || 0) + 1; }
    ev.observe(`${N} DELETEs answered in ${Math.round(performance.now() - t0)} ms: ${JSON.stringify(codes)}`, { means: '204 means accepted. 503 PrimaryInvalidationBusy would mean the gateway asked you to retry.' });
    const uk = [...new Set(ks)]; const first = new Map(); const t1 = performance.now(); let ms2 = null, stale = uk.length;
    ev.action(`Watching node 2 until it stops serving each of the ${uk.length} deleted keys (up to 60 s)`, { means: 'Each gateway applies purges as they arrive; we time the first, the middle and the last.' });
    for (let round = 0; round < 120 && first.size < uk.length; round++) {
      for (const k of uk) { if (first.has(k)) continue; const r = await httpRequest({ port: ports[1], path: cachePath(k, nonce), token: TOKEN }); if (r.headers && r.headers['x-cache-status'] !== 'HIT') first.set(k, Math.round(performance.now() - t1)); }
      if (first.size < uk.length) await sleep(500);
    }
    stale = uk.length - first.size; const tv = [...first.values()].sort((x, y) => x - y);
    if (!stale) ms2 = tv[tv.length - 1];
    ev.observe(stale ? `node 2 still served ${stale} of ${uk.length} deleted keys after 60 s` : `node 2 stopped serving all ${uk.length} deleted keys: first after ${tv[0]} ms, median ${tv[tv.length >> 1]} ms, last after ${ms2} ms`, { means: stale ? 'FINDING: some purges had not arrived after a minute.' : 'The live gateway honoured every purge. Delivery is paced, so a very large purge storm takes seconds to finish everywhere (measured here; the dead peer made no visible difference).' });
    ev.action(`Restarting node ${st.nodes}`);
    spawnSync(process.execPath, [join(HERE, 'cluster_launcher.mjs'), 'start-node', '--dir', DIR, '--node', String(st.nodes)], { encoding: 'utf8' });
    let act = false, vw = null; for (let i = 0; i < 100 && !act; i++) { await sleep(300); vw = await gatewayView(ports[0]); act = vw && Object.keys(vw).length === st.nodes && Object.values(vw).every((x) => x.state === 'Active'); }
    results.phases.deletes = { deletes: N, codes, node2ClearedMs: ms2, staleAfter: stale, allActiveAfterRestart: act };
    check(`delete storm with a dead node: ${N} concurrent DELETEs all accepted (2xx) and the live peer stops serving every deleted key (all cleared in ${ms2} ms)`, dels.every((d) => d.status >= 200 && d.status < 300) && ms2 !== null, `codes ${JSON.stringify(codes)}, node 2 cleared in ${ms2} ms`);
    check('delete storm: node restarted and every member Active again', act, JSON.stringify(vw));
  }

  st.pids = loadCluster(DIR).pids;
  results.rssKbAfter = rssAll();
  results.rssNote = 'working set in KB via tasklist (Windows), per gateway process in node order; node restarted in the deadnode phase has a new process, its "after" value is not comparable with "before"';
  results.totals = { requests: totalReq, errors: totalErr + 0, integrityFailures: totalInteg, bytesTransferred: totalBytes };
  results.originTotal = await originStats(st.originPort).then((s) => (s ? { requestsSinceOriginStart: s.requests, bytes: s.bytes } : { unavailable: true }));
  results.clientStaleSocketRetries = clientStats.staleSocketRetries;
  const per = {};
  for (let i = 0; i < ports.length; i++) { const o = await gatewayOutcomes(ports[i], TOKEN); per[`node${i + 1}`] = o.error ? { error: o.error } : { responses: o.responses, peer_reads_without_hit: o.peer_reads_without_hit }; }
  results.gatewayOutcomesSinceStart = { note: 'metrics.outcomes per gateway process since its start (measured by the gateway)', ...per };
  check('TOTAL: zero transport/HTTP errors across the whole scenario', results.totals.errors === 0, `${totalReq} requests (excluding phase-internal probes), ${totalBytes} bytes`);
  check('TOTAL: zero integrity failures (sha256, bytes, headers vs origin manifest) across the whole scenario', results.totals.integrityFailures === 0, '');
  results.finishedAt = new Date().toISOString();
  results.summary = { pass: results.checks.filter((c) => c.status === 'PASS').length, fail: results.checks.filter((c) => c.status === 'FAIL').length };
  writeFileSync(OUT, JSON.stringify(results, null, 2));
  ev.summary(results.summary.fail ? 'attention' : 'passed', [{ label: 'requests sent', value: totalReq, tag: 'measured' }, { label: 'errors', value: results.totals.errors, tag: 'measured' }, { label: 'integrity failures (sha256 vs origin)', value: results.totals.integrityFailures, tag: 'measured' }, { label: 'bytes transferred', value: totalBytes, tag: 'measured' }, { label: 'checks passed / failed', value: `${results.summary.pass} / ${results.summary.fail}`, tag: 'measured' }]);
  console.log(`\n[real_requests] ${results.summary.pass} PASS, ${results.summary.fail} FAIL; totals ${JSON.stringify(results.totals)}; result file ${OUT}`);
  process.exit(results.summary.fail ? 1 : 0);
}

async function gatewayView(port) {
  const j = await getJson(port, '/api/v1/metrics', TOKEN);
  const c = j && j.metrics && j.metrics.cluster;
  if (!c || !c.local) return null;
  const nodes = { [c.local.node_id]: { state: c.local.state && c.local.state.v, reachability: 'self' } };
  for (const p of c.peers) nodes[p.node_id] = { state: p.state && p.state.v, reachability: p.reachability && p.reachability.v };
  return nodes;
}

main().catch((e) => { console.error(e); process.exit(1); });
