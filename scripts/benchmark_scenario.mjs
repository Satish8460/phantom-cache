#!/usr/bin/env node
// scripts/benchmark_scenario.mjs  (lane L5, scenario 4: BENCHMARK)   protocol: docs/research/benchmark_protocol.md
//
// Closed-loop load from ONE Node process (the client) against three targets that all serve the SAME REAL bytes from a catalog
// origin running as a separate process:
//   direct   clients go straight to the origin ("no cache" baseline)
//   single   one gateway (release build) in front of the origin
//   cluster  three mTLS gateways, requests spread round-robin over the three HTTP ports
// Workload: Zipf(s) popularity over --objects distinct objects (JSON/text/PNG, see catalogObject), N requests at concurrency C.
// Per (cell = N x C, repetition, target): fresh start (cold cache) -> warm-up of --warmup requests (other seed, not measured) ->
// measured run. Every 200 response is verified: status, Content-Type and the sha256 the ORIGIN reported in its manifest.
// Any failure is recorded; the process exits 1 if any request failed or any integrity check failed.
//
// Results are appended to <out-dir>/benchmark.runs.jsonl as they finish (an interrupted run keeps its data); the summary
// (mean and 95 % CI per cell) is written to benchmark.summary.json / .md.  --summarize <out-dir> re-computes it.
//
// Usage: node scripts/benchmark_scenario.mjs [--exe <gateway>] [--out-dir <dir>] [--reps 10] [--sizes 1000,5000,10000]
//          [--concs 50,200,500] [--targets direct,single,cluster] [--objects 1000] [--zipf 1.0] [--warmup 1000] [--seed 20261031]
//          [--certs <dir>] [--origin-port 19230] [--port-base 18230] [--cluster-port-base 19631]
// Not a pass/fail correctness check except for: 100 % success and 0 integrity failures.

import http from 'node:http';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync, openSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { argv, DEFAULT_EXE, defaultOut, httpReq, rng, sleep, spawnGateway, hostFacts, waitReady, metrics, mv } from './lib/scenario_common.mjs';
import { ci95, fmt, percentile, sortedCopy, zipfSequence, mean } from './lib/bench_stats.mjs';

const { opt, flag } = argv();
const OUT = resolve(opt('out-dir', defaultOut('benchmark')));
const JSONL = join(OUT, 'benchmark.runs.jsonl');

function summarize() {
  const runs = readFileSync(JSONL, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
  const cells = new Map();
  for (const r of runs) { const k = `${r.profile || 'O0'}|${r.target}|${r.N}|${r.C}`; if (!cells.has(k)) cells.set(k, []); cells.get(k).push(r); }
  const rows = [];
  for (const [k, rs] of [...cells.entries()].sort()) {
    const [profile, target, N, C] = k.split('|');
    const m = (f) => ci95(rs.map(f));
    const srcTot = (s) => rs.map((r) => (r.sources[s] || 0) / r.ok);
    rows.push({
      profile, target, N: Number(N), C: Number(C), reps: rs.length,
      success_rate_min: Math.min(...rs.map((r) => r.ok / r.N)), failed_total: rs.reduce((a, r) => a + r.failed, 0), integrity_failures_total: rs.reduce((a, r) => a + r.integrity_failures, 0),
      throughput_rps: m((r) => r.throughput_rps), p50_ms: m((r) => r.p50_ms), p95_ms: m((r) => r.p95_ms), p99_ms: m((r) => r.p99_ms),
      origin_requests: m((r) => r.origin_requests),
      origin_requests_per_100: m((r) => (100 * r.origin_requests) / r.N),
      src_local: ci95(srcTot('local')), src_peer: ci95(srcTot('peer')), src_origin: ci95(srcTot('origin')), src_coalesced: ci95(srcTot('coalesced')), src_stale: ci95(srcTot('stale')),
      rss_before_mb: m((r) => (r.rss_before_total ?? NaN) / 1048576), rss_after_mb: m((r) => (r.rss_after_total ?? NaN) / 1048576),
    });
  }
  // paired throughput ratio gateway / direct (same cell, same rep, same request sequence)
  const ratios = [];
  for (const t of ['single', 'cluster']) for (const [k, rs] of cells.entries()) {
    const [profile, target, N, C] = k.split('|'); if (target !== t) continue;
    const d = cells.get(`${profile}|direct|${N}|${C}`) || [];
    const pairs = rs.map((r) => ({ r, d: d.find((x) => x.rep === r.rep) })).filter((p) => p.d);
    ratios.push({ profile, target: t, N: Number(N), C: Number(C), pairs: pairs.length, throughput_ratio_vs_direct: ci95(pairs.map((p) => p.r.throughput_rps / p.d.throughput_rps)), p50_ratio_vs_direct: ci95(pairs.map((p) => p.r.p50_ms / p.d.p50_ms)), p99_ratio_vs_direct: ci95(pairs.map((p) => p.r.p99_ms / p.d.p99_ms)) });
  }
  const md = [];
  md.push('| profile | target | N | C | reps | success (min) | req/s (mean ± 95% CI) | p50 ms | p95 ms | p99 ms | origin req / 100 req | local % | peer % | origin % | coalesced % | RSS before→after MB (sum of gateways) |');
  md.push('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|');
  for (const r of rows) md.push(`| ${r.profile} | ${r.target} | ${r.N} | ${r.C} | ${r.reps} | ${(100 * r.success_rate_min).toFixed(2)}% | ${fmt(r.throughput_rps, 0)} | ${fmt(r.p50_ms, 1)} | ${fmt(r.p95_ms, 1)} | ${fmt(r.p99_ms, 1)} | ${fmt(r.origin_requests_per_100, 1)} | ${(100 * r.src_local.mean).toFixed(1)} | ${(100 * r.src_peer.mean).toFixed(1)} | ${(100 * r.src_origin.mean).toFixed(1)} | ${(100 * r.src_coalesced.mean).toFixed(1)} | ${r.target === 'direct' ? 'n/a' : `${fmt(r.rss_before_mb, 0)} → ${fmt(r.rss_after_mb, 0)}`} |`);
  md.push('', 'Paired ratios gateway / direct (same repetition, same request sequence; mean ± 95% CI of the per-repetition ratio; < 1 means the gateway is slower for throughput, > 1 means higher latency for p50/p99):', '', '| profile | target | N | C | pairs | throughput ratio | p50 ratio | p99 ratio |', '|---|---|---|---|---|---|---|---|');
  for (const q of ratios) md.push(`| ${q.profile} | ${q.target} | ${q.N} | ${q.C} | ${q.pairs} | ${fmt(q.throughput_ratio_vs_direct, 2)} | ${fmt(q.p50_ratio_vs_direct, 2)} | ${fmt(q.p99_ratio_vs_direct, 2)} |`);
  writeFileSync(join(OUT, 'benchmark.summary.json'), JSON.stringify({ rows, ratios }, null, 2));
  writeFileSync(join(OUT, 'benchmark.summary.md'), md.join('\n') + '\n');
  return { rows, ratios, md: md.join('\n'), runs };
}

if (flag('summarize') || argv().opt('summarize-only')) { /* handled below */ }
if (flag('summarize')) { const s = summarize(); console.log(s.md); process.exit(s.runs.some((r) => r.failed || r.integrity_failures) ? 1 : 0); }

// ------------------------------------------------------------------ configuration
const EXE = opt('exe', DEFAULT_EXE);
const REPS = Number(opt('reps', 10));
const SIZES = opt('sizes', '1000,5000,10000').split(',').map(Number);
const CONCS = opt('concs', '50,200,500').split(',').map(Number);
const TARGETS = opt('targets', 'direct,single,cluster').split(',');
const PROFILES = { O0: { delay: 0, max: 0 }, O1: { delay: 25, max: 0 }, O2: { delay: 100, max: 0 }, O3: { delay: 100, max: 20 } };
const PROFILE_LIST = opt('profiles', 'O0').split(',');
let PROFILE = 'O0';
const OBJECTS = Number(opt('objects', 1000));
const ZIPF = Number(opt('zipf', 1.0));
const WARMUP = Number(opt('warmup', 1000));
const SEED = Number(opt('seed', 20261031));
const OPORT = Number(opt('origin-port', 19230));
const PB = Number(opt('port-base', 18230));
const CB = Number(opt('cluster-port-base', 19631));
const CERTS = resolve(opt('certs', join(OUT, 'cluster-work', 'certs')));
const HERE = dirname(fileURLToPath(import.meta.url));
mkdirSync(OUT, { recursive: true });
const log = (m) => console.log(`[BENCH ${new Date().toISOString().slice(11, 19)}] ${m}`);

// ------------------------------------------------------------------ processes
let originProc = null, manifest = null;
async function startOrigin() {
  originProc = spawn(process.execPath, [join(HERE, 'lib', 'scenario_common.mjs'), 'origin', '--port', String(OPORT), '--delay-ms', String(PROFILES[PROFILE].delay), '--max-concurrent', String(PROFILES[PROFILE].max)], { stdio: ['ignore', openSync(join(OUT, 'origin-'+PROFILE+'.out.log'), 'w'), openSync(join(OUT, 'origin.err.log'), 'w')], windowsHide: true });
  const t0 = Date.now();
  while (Date.now() - t0 < 30000) { const r = await httpReq({ port: OPORT, path: '/__stats', timeoutMs: 500 }); if (r.ok) break; await sleep(100); }
  const r = await httpReq({ port: OPORT, path: `/__manifest?n=${OBJECTS}`, timeoutMs: 120000 });
  if (!r.ok) throw new Error('manifest fetch failed ' + r.error);
  manifest = JSON.parse(r.body.toString());
  const ids = Object.keys(manifest);
  const tot = ids.reduce((a, k) => a + manifest[k].bytes, 0);
  const byType = {}; for (const k of ids) byType[manifest[k].type] = (byType[manifest[k].type] || 0) + 1;
  log(`origin up on ${OPORT}; ${ids.length} objects, ${tot} bytes total (mean ${(tot / ids.length).toFixed(0)} B), by type ${JSON.stringify(byType)}`);
  return { objects: ids.length, total_bytes: tot, by_type: byType, min_bytes: Math.min(...ids.map((k) => manifest[k].bytes)), max_bytes: Math.max(...ids.map((k) => manifest[k].bytes)) };
}
const originStats = async () => JSON.parse((await httpReq({ port: OPORT, path: '/__stats' })).body.toString()).requests;
const originReset = () => httpReq({ port: OPORT, path: '/__reset' });

function ensureCerts() {
  if (existsSync(join(CERTS, 'members.json'))) return;
  const work = resolve(CERTS, '..');
  mkdirSync(work, { recursive: true });
  log(`creating test certificates with the existing launcher (up/down) in ${work}`);
  const a = ['up', '--exe', EXE, '--dir', work, '--http-port-base', String(PB), '--cluster-port-base', String(CB), '--origin-port', String(OPORT + 7)];
  const r = spawnSync(process.execPath, [join(HERE, 'cluster_launcher.mjs'), ...a], { encoding: 'utf8' });
  spawnSync(process.execPath, [join(HERE, 'cluster_launcher.mjs'), 'down', '--dir', work], { encoding: 'utf8' });
  if (!existsSync(join(CERTS, 'members.json'))) throw new Error('certificate creation failed: ' + (r.stderr || r.stdout));
}

async function startTarget(target, tag) {
  if (target === 'direct') return { ports: [OPORT], gws: [], prefix: '', stop: async () => {} };
  if (target === 'single') {
    const g = spawnGateway({ exe: EXE, port: PB, originUrl: `http://127.0.0.1:${OPORT}`, outDir: OUT, name: `gw-${tag}` });
    if ((await waitReady(PB, 20000)) === null) throw new Error('single gateway did not become ready');
    return { ports: [PB], gws: [g], prefix: '/api/v1/cache', stop: async () => { await g.kill(); } };
  }
  const members = JSON.parse(readFileSync(join(CERTS, 'members.json'), 'utf8'));
  const peers = [1, 2, 3].map((i) => `${i}=127.0.0.1:${CB + i - 1}`).join(',');
  const gws = [1, 2, 3].map((n) => spawnGateway({
    exe: EXE, port: PB + n - 1, originUrl: `http://127.0.0.1:${OPORT}`, outDir: OUT, name: `gw-${tag}-n${n}`,
    env: { PHANTOM_CLUSTER_ID: members.cluster_id, PHANTOM_CLUSTER_MEMBERS_FILE: join(CERTS, 'members.json'), PHANTOM_CLUSTER_CA: join(CERTS, 'ca.crt'), PHANTOM_CLUSTER_PEER_ADDRS: peers, PHANTOM_NODE_ID: String(n), PHANTOM_NODE_CERT: join(CERTS, `n${n}.crt`), PHANTOM_NODE_KEY: join(CERTS, `n${n}.p8.key`), PHANTOM_CLUSTER_LISTEN_ADDR: `127.0.0.1:${CB + n - 1}` },
  }));
  const t0 = Date.now();
  for (;;) {
    if (Date.now() - t0 > 30000) throw new Error('cluster did not become Active');
    const m = await metrics(PB);
    const c = m?.cluster;
    if (c && c.local && c.peers && [c.local.state?.v, ...c.peers.map((p) => p.state?.v)].every((s) => s === 'Active') && c.peers.length === 2) break;
    await sleep(150);
  }
  return { ports: [PB, PB + 1, PB + 2], gws, prefix: '/api/v1/cache', stop: async () => { for (const g of gws) await g.kill(); } };
}

// ------------------------------------------------------------------ load generator
const keys = Object.keys(manifest ?? {});
async function runLoad({ ports, prefix }, seq, C) {
  const agents = ports.map(() => new http.Agent({ keepAlive: true, maxSockets: Math.ceil(C / ports.length) + 2 }));
  const N = seq.length, lat = new Float64Array(N);
  const sources = {}, statuses = {}; let ok = 0, failed = 0, integrity = 0; const samples = [];
  let next = 0;
  const note = (s) => { if (samples.length < 25) samples.push(s); };
  const t0 = process.hrtime.bigint();
  const worker = async () => {
    for (;;) {
      const i = next++; if (i >= N) return;
      const key = keys[seq[i]], exp = manifest[key];
      const pi = i % ports.length;
      await new Promise((done) => {
        const s = process.hrtime.bigint();
        const req = http.request({ host: '127.0.0.1', port: ports[pi], path: prefix + key, method: 'GET', agent: agents[pi], timeout: 30000 }, (res) => {
          const h = createHash('sha256'); let len = 0;
          res.on('data', (c) => { h.update(c); len += c.length; });
          res.on('end', () => {
            lat[i] = Number(process.hrtime.bigint() - s) / 1e6;
            statuses[res.statusCode] = (statuses[res.statusCode] || 0) + 1;
            if (res.statusCode !== 200) { failed++; note(`${key}: status ${res.statusCode}`); return done(); }
            const good = h.digest('hex') === exp.sha256 && len === exp.bytes && res.headers['content-type'] === exp.type;
            if (!good) { integrity++; failed++; note(`${key}: integrity (len ${len} vs ${exp.bytes}, type ${res.headers['content-type']})`); return done(); }
            ok++; const src = res.headers['x-cache-source'] || 'none'; sources[src] = (sources[src] || 0) + 1;
            done();
          });
          res.on('error', (e) => { lat[i] = Number(process.hrtime.bigint() - s) / 1e6; failed++; note(`${key}: response error ${e.code || e.message}`); done(); });
        });
        req.on('timeout', () => req.destroy(new Error('timeout')));
        req.on('error', (e) => { lat[i] = Number(process.hrtime.bigint() - s) / 1e6; failed++; statuses[e.code || 'error'] = (statuses[e.code || 'error'] || 0) + 1; note(`${key}: ${e.code || e.message}`); done(); });
        req.end();
      });
    }
  };
  await Promise.all(Array.from({ length: C }, worker));
  const elapsed = Number(process.hrtime.bigint() - t0) / 1e9;
  for (const a of agents) a.destroy();
  return { elapsed, lat, ok, failed, integrity, sources, statuses, samples };
}

async function rssOf(t) {
  if (!t.gws.length) return null;
  const out = [];
  for (const p of t.ports) { const m = await metrics(p); out.push(mv(m, 'process', 'rss_bytes') ?? null); }
  return out;
}

// ------------------------------------------------------------------ main
const failedRuns = [];
async function main() {
  PROFILE = PROFILE_LIST[0];
  const catalogInfo = await startOrigin();
  if (TARGETS.includes('cluster')) ensureCerts();
  const exeHash = createHash('sha256').update(readFileSync(EXE)).digest('hex');
  const ver = spawnSync(EXE, ['--version'], { encoding: 'utf8' });
  const meta = { started_at: new Date().toISOString(), host: hostFacts(), gateway_exe: EXE, gateway_exe_sha256: exeHash, gateway_version: (ver.stdout || ver.stderr || '').trim().split('\n')[0], profile: 'release (per the programme brief; the script cannot verify the profile from the binary)', args: { PROFILE_LIST, PROFILES, REPS, SIZES, CONCS, TARGETS, OBJECTS, ZIPF, WARMUP, SEED }, catalog: catalogInfo, client: 'single Node process, node:http keep-alive, sha256 of every body' };
  writeFileSync(join(OUT, 'benchmark.meta.json'), JSON.stringify(meta, null, 2));
  log(`host: ${meta.host.os}, ${meta.host.cpu_model} x${meta.host.logical_cpus}, ${(meta.host.total_mem_bytes / 2 ** 30).toFixed(1)} GiB; gateway ${meta.gateway_version}; exe sha256 ${exeHash.slice(0, 16)}...`);
  keys.length = 0; keys.push(...Object.keys(manifest));
  const cells = []; for (const N of SIZES) for (const C of CONCS) cells.push({ N, C });
  for (const prof of PROFILE_LIST) {
  if (prof !== PROFILE) { try { originProc.kill(); } catch { /* */ } await sleep(500); PROFILE = prof; await startOrigin(); keys.length = 0; keys.push(...Object.keys(manifest)); }
  log(`origin profile ${PROFILE}: ${JSON.stringify(PROFILES[PROFILE])} (delay ms / max concurrent, 0 = none)`);
  for (let rep = 1; rep <= REPS; rep++) {
    for (let ci = 0; ci < cells.length; ci++) {
      const { N, C } = cells[ci];
      const mseed = SEED + rep * 1000 + ci, wseed = SEED + rep * 1000 + 500 + ci;
      const mseq = zipfSequence({ n: keys.length, s: ZIPF, count: N, seed: mseed, rng }), wseq = zipfSequence({ n: keys.length, s: ZIPF, count: WARMUP, seed: wseed, rng });
      const distinct = new Set(mseq).size;
      for (const target of TARGETS) {
        const tag = `r${rep}c${ci}${target[0]}`;
        let t = null;
        try {
          t = await startTarget(target, tag);
          await runLoad(t, wseq, C); // warm-up, not measured
          await originReset();
          const rb = await rssOf(t);
          const res = await runLoad(t, mseq, C);
          const ra = await rssOf(t);
          const oreq = await originStats();
          const sorted = sortedCopy(res.lat);
          const row = {
            profile: PROFILE, origin_profile: PROFILES[PROFILE], target, N, C, rep, seed: mseed, warmup_seed: wseed, distinct_objects_in_run: distinct, ok: res.ok, failed: res.failed, integrity_failures: res.integrity,
            duration_s: res.elapsed, throughput_rps: res.ok / res.elapsed, mean_ms: mean([...res.lat]), p50_ms: percentile(sorted, 0.5), p95_ms: percentile(sorted, 0.95), p99_ms: percentile(sorted, 0.99), max_ms: sorted[sorted.length - 1],
            origin_requests: oreq, sources: res.sources, statuses: res.statuses, failure_samples: res.samples,
            rss_before: rb, rss_after: ra, rss_before_total: rb ? rb.reduce((a, b) => a + (b ?? 0), 0) : null, rss_after_total: ra ? ra.reduce((a, b) => a + (b ?? 0), 0) : null,
          };
          appendFileSync(JSONL, JSON.stringify(row) + '\n');
          if (res.failed) failedRuns.push(`${PROFILE} ${target} N=${N} C=${C} rep=${rep}: ${res.failed} failed ${JSON.stringify(res.statuses)} ${res.samples.slice(0, 3).join('; ')}`);
          log(`${PROFILE} rep ${rep}/${REPS} ${target.padEnd(7)} N=${String(N).padEnd(5)} C=${String(C).padEnd(3)}: ${row.throughput_rps.toFixed(0)} req/s, p50 ${row.p50_ms.toFixed(1)} p95 ${row.p95_ms.toFixed(1)} p99 ${row.p99_ms.toFixed(1)} ms, ok ${res.ok}/${N}, integrity ${res.integrity}, origin ${oreq}, src ${JSON.stringify(res.sources)}${rb ? `, rss ${(row.rss_before_total / 1048576).toFixed(0)}->${(row.rss_after_total / 1048576).toFixed(0)} MB` : ''}`);
        } catch (e) {
          failedRuns.push(`${target} N=${N} C=${C} rep=${rep}: harness error ${e && e.message}`);
          log(`HARNESS ERROR ${target} N=${N} C=${C} rep=${rep}: ${e && e.stack}`);
        } finally { if (t) await t.stop(); await sleep(150); }
      }
    }
  }
  }
}

let exit = 0;
try { await main(); } catch (e) { log('fatal: ' + (e && e.stack)); exit = 1; }
finally { try { originProc?.kill(); } catch { /* */ } }
if (existsSync(JSONL)) { const s = summarize(); console.log('\n' + s.md); }
if (failedRuns.length) { log(`FAILED RUNS (${failedRuns.length}):\n  ${failedRuns.join('\n  ')}`); exit = 1; }
else if (exit === 0) log('every run: 100 % success and 0 integrity failures');
process.exit(exit);
