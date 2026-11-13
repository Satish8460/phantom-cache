#!/usr/bin/env node
// scripts/eviction_scenario.mjs  (lane L5, scenario 2: EVICTION and MEMORY)
//
// One gateway with a SMALL byte budget (PHANTOM_MAX_CACHE_BYTES, default 8 MiB) in front of a scriptable origin serving REAL
// generated PNG images (30-120 KB), plus a > 1 MiB and a > 2 MiB image. Phases:
//   0 accounting  : fill below the budget; compare metrics.bytes_resident with the sum of the body bytes sent
//   1 pressure    : 10 HOT keys re-read every 10 inserts while ~300 distinct ONE-HIT images stream through (~22 MB through 8 MiB);
//                   bytes_resident must stay <= the budget at every sample; evictions must happen
//   2 survival    : hot keys must be HIT more often than one-hit keys of the same age (default policy)
//   3 oversize    : a 1.2 MiB and a 2.5 MiB image (the gateway's origin body limit is 2 MiB, src/api/server.rs:2616)
//   4 leak check  : DELETE every key; bytes_resident must return to ~0
// Every body is verified by sha256 against the bytes the origin served. The metrics fields are reported as the gateway exposes them.
//
// Usage: node scripts/eviction_scenario.mjs [--exe <gateway>] [--out-dir <dir>] [--origin-port 19230] [--port 18232] [--budget 8388608] [--stream 300]
// Exit 0 only if every assertion held.

import { argv, DEFAULT_EXE, defaultOut, httpReq, makePngOfSize, sha256, sleep, spawnGateway, startOrigin, waitReady, metrics, mv, recorder } from './lib/scenario_common.mjs';

const { opt } = argv();
const EXE = opt('exe', DEFAULT_EXE);
const OUT = opt('out-dir', defaultOut('eviction'));
const OPORT = Number(opt('origin-port', 19230));
const GPORT = Number(opt('port', 18232));
const BUDGET = Number(opt('budget', 8 * 1024 * 1024));
const STREAM = Number(opt('stream', 300));
const R = recorder('eviction', OUT, { prefix: 'EVICTION' });
const { check, log } = R;
const origin = startOrigin({ port: OPORT });
let gw;

const sizeOf = (i) => 30000 + ((i * 7727) % 90000);
const objs = new Map(); // path -> {sha, len}
const reg = (path, seed, bytes) => {
  const body = makePngOfSize(bytes, seed);
  const sha = sha256(body);
  origin.set(path, { body, type: 'image/png', etag: `"${sha.slice(0, 16)}"`, cacheControl: 'public, max-age=3600' });
  objs.set(path, { sha, len: body.length });
  return objs.get(path);
};
let integrityFailures = 0, requests = 0;
const bytesSent = { v: 0 };
async function get(path) {
  const r = await httpReq({ port: GPORT, path: `/api/v1/cache${path}` });
  requests++;
  const o = objs.get(path);
  if (!r.ok) { integrityFailures++; return { cs: 'ERR', src: r.error, status: 0 }; }
  const good = r.status === 200 && o && sha256(r.body) === o.sha && r.headers['content-type'] === 'image/png';
  if (r.status === 200 && !good) integrityFailures++;
  if (r.status === 200) bytesSent.v += r.body.length;
  return { status: r.status, err: r.headers['x-cache-error'], cs: r.headers['x-cache-status'], src: r.headers['x-cache-source'], good, len: r.body.length };
}
const snap = async () => {
  const m = await metrics(GPORT);
  return { resident: mv(m, 'bytes_resident'), budget: mv(m, 'byte_budget'), evictions: mv(m, 'evictions_total'), budgetRej: mv(m, 'byte_budget_rejections'), oversize: mv(m, 'oversize_rejections'), capRej: mv(m, 'capacity_rejections'), rss: mv(m, 'process', 'rss_bytes'), hits: mv(m, 'cache_hits'), misses: mv(m, 'cache_misses'), basis: m?.bytes_resident?.basis };
};


const MIB = 1048576;
async function bigObjects(budget) {
  const shard = Math.floor(budget / 8);
  log(`large objects with PHANTOM_MAX_CACHE_BYTES=${budget}: per-shard budget ${shard} B (budget / 8 shards)`);
  const tagb = budget === BUDGET && budget < 64 * MIB ? 'small' : 'default';
  for (const [name, bytes] of [['0.9MB', 900000], ['1.2MiB', 1258291], ['2.5MiB', 2621440]]) {
    const p = `/e/big/${tagb}-${name}.png`; const o = reg(p, 777 + bytes % 13, bytes);
    const before = await snap();
    const a = await get(p), b = await get(p);
    const after = await snap();
    const fitsShard = o.len + 600 <= shard; // 600 B ~ key + fixed per-entry overhead (measured overhead 468 B per entry, section accounting)
    log(`budget ${budget}, ${name} (${o.len} B): GET1 ${a.status} ${a.cs}/${a.src} err=${a.err} good=${a.good}; GET2 ${b.status} ${b.cs}/${b.src}; oversize_rejections ${before.oversize} -> ${after.oversize}; byte_budget_rejections ${before.budgetRej} -> ${after.budgetRej}`);
    if (name === '2.5MiB') {
      check(a.status === 502 && a.err === 'ResponseTooLarge' && b.status === 502 && !a.good, `budget ${budget}: 2.5MiB (> 2 MiB origin-body limit, independent of the budget) -> 502 X-Cache-Error ResponseTooLarge, never a truncated 200`, `${a.status} ${a.err} then ${b.status}`);
    } else {
      check(a.status === 200 && a.good, `budget ${budget}: ${name} served with the correct bytes`, `${a.status} good=${a.good}`);
      if (fitsShard) check(b.cs === 'HIT' && b.src === 'local' && after.oversize === before.oversize, `budget ${budget}: ${name} fits one shard budget (${shard} B) -> cached (second GET HIT/local)`, `${a.cs} then ${b.cs}/${b.src}`);
      else check(b.cs === 'MISS' && after.oversize > before.oversize, `budget ${budget}: ${name} is larger than one shard budget (${shard} B) -> served but NOT cached (second GET MISS, oversize_rejections up)`, `${a.cs} then ${b.cs}; oversize_rejections ${before.oversize} -> ${after.oversize}`);
    }
    R.data[`big_${tagb}_${name}`] = { budget, per_shard_budget: shard, bytes: o.len, first: a.cs, second: b.cs, status: a.status, error: a.err, oversize_rejections_delta: after.oversize - before.oversize };
  }
}

async function main() {
  await origin.listen();
  gw = spawnGateway({ exe: EXE, port: GPORT, originUrl: origin.url, outDir: OUT, name: 'gw', env: { PHANTOM_MAX_CACHE_BYTES: String(BUDGET) } });
  check((await waitReady(GPORT)) !== null, 'gateway ready with PHANTOM_MAX_CACHE_BYTES=' + BUDGET);
  const s0 = await snap();
  check(s0.budget === BUDGET, 'metrics.byte_budget equals the configured budget', `reported ${s0.budget}`);
  log(`start: ${JSON.stringify(s0)}`);

  // ---- Phase 0: accounting below the budget
  const P0 = 40; let sum0 = 0;
  for (let i = 0; i < P0; i++) { const p = `/e/acct/${i}.png`; const o = reg(p, 1000 + i, sizeOf(i)); sum0 += o.len; const r = await get(p); if (!(r.good && r.cs === 'MISS')) check(false, `acct ${i}`, JSON.stringify(r)); }
  const s1 = await snap();
  const over = (s1.resident - sum0) / P0;
  log(`accounting: ${P0} images, body bytes sent ${sum0}, bytes_resident ${s1.resident} (basis ${s1.basis}), difference ${s1.resident - sum0} = ${over.toFixed(0)} B per entry, evictions ${s1.evictions}`);
  check(s1.evictions === 0 && sum0 < BUDGET, 'accounting: below the budget no eviction happened', `evictions=${s1.evictions} sum=${sum0}`);
  check(s1.resident >= sum0, 'accounting: bytes_resident >= sum of the cached body bytes', `${s1.resident} >= ${sum0}`);
  check(s1.resident <= sum0 * 1.02, 'accounting: bytes_resident is within 2 % of the body bytes (per-entry overhead small against 30-120 KB bodies)', `ratio ${(s1.resident / sum0).toFixed(4)}`);
  R.data.accounting = { images: P0, body_bytes: sum0, bytes_resident: s1.resident, per_entry_overhead_bytes: over, basis: s1.basis };

  // ---- Phase 1: pressure with hot keys and one-hit wonders
  const HOT = 10, hotKeys = [];
  for (let i = 0; i < HOT; i++) { const p = `/e/hot/${i}.png`; reg(p, 5000 + i, 60000); hotKeys.push(p); await get(p); }
  const oneHit = [];
  let maxResident = 0, samples = 0, overBudget = 0;
  const t0 = Date.now();
  for (let i = 0; i < STREAM; i++) {
    const p = `/e/once/${i}.png`; reg(p, 9000 + i, sizeOf(i + 7)); oneHit.push(p);
    const r = await get(p);
    if (!r.good) check(false, `stream ${i} integrity`, JSON.stringify(r));
    if (i % 10 === 9) for (const h of hotKeys) await get(h);
    if (i % 15 === 14) { const s = await snap(); samples++; maxResident = Math.max(maxResident, s.resident); if (s.resident > BUDGET) overBudget++; }
  }
  const s2 = await snap();
  const streamBytes = oneHit.reduce((a, p) => a + objs.get(p).len, 0);
  log(`pressure: ${STREAM} one-hit images (${streamBytes} B) + hot refreshes in ${Date.now() - t0} ms; ${JSON.stringify(s2)}; max sampled resident ${maxResident} (${samples} samples, ${overBudget} over budget)`);
  check(overBudget === 0 && s2.resident <= BUDGET, 'pressure: bytes_resident stayed <= PHANTOM_MAX_CACHE_BYTES at every sample and at the end', `max ${maxResident} of ${BUDGET}; end ${s2.resident}`);
  check(s2.evictions > 0, 'pressure: evictions happened (evictions_total > 0)', `evictions_total=${s2.evictions}`);
  check(s2.resident >= BUDGET * 0.5, 'pressure: the budget is actually used (resident >= 50 % of it), not under-filled', `resident ${s2.resident} = ${(100 * s2.resident / BUDGET).toFixed(1)} %`);
  check(integrityFailures === 0, 'integrity: every 200 had the origin sha256 and Content-Type image/png', `failures=${integrityFailures} of ${requests} requests`);
  R.data.pressure = { budget: BUDGET, stream_bytes: streamBytes, max_sampled_resident: maxResident, end: s2, rss_start: s0.rss, rss_end: s2.rss };
  log(`RSS: start ${s0.rss} end ${s2.rss} (budget ${BUDGET}); RSS is the whole process, the budget counts logical body bytes (not a RSS limit)`);

  // ---- Phase 2: survival
  const hotRes = []; for (const h of hotKeys) hotRes.push((await get(h)).cs === 'HIT');
  const early = oneHit.slice(0, 30), late = oneHit.slice(-30);
  const lateRes = []; for (const p of late) lateRes.push((await get(p)).cs === 'HIT');
  const earlyRes = []; for (const p of early) earlyRes.push((await get(p)).cs === 'HIT');
  const frac = (a) => a.filter(Boolean).length / a.length;
  log(`survival: hot ${hotRes.filter(Boolean).length}/${HOT} HIT; one-hit most recent 30: ${lateRes.filter(Boolean).length}/30 HIT; one-hit oldest 30 (inserted ${STREAM - 30}+ inserts ago): ${earlyRes.filter(Boolean).length}/30 HIT`);
  check(frac(hotRes) >= 0.9, 'survival: >= 90 % of the hot keys are still cached after ~22 MB of one-hit traffic', `${(100 * frac(hotRes)).toFixed(0)} %`);
  check(frac(hotRes) > frac(earlyRes), 'survival: hot keys survive better than one-hit keys inserted long ago', `hot ${(100 * frac(hotRes)).toFixed(0)} % vs old one-hit ${(100 * frac(earlyRes)).toFixed(0)} %`);
  check(frac(earlyRes) <= 0.1, 'survival: old one-hit keys were evicted (<= 10 % still HIT)', `${(100 * frac(earlyRes)).toFixed(0)} %`);
  R.data.survival = { hot: frac(hotRes), recent_one_hit: frac(lateRes), old_one_hit: frac(earlyRes), policy: 'default (PHANTOM has no eviction-policy variable in configuration.md)' };

  // ---- Phase 3: large objects under THIS budget. Rule (verified in code): PartitionedCache::new(8) (src/main.rs:204) splits the budget
  // into 8 shards (set_byte_budget, src/core/cache.rs:1843-1850, budget/8 each); an entry is not stored when key + overhead + body > ONE
  // shard's budget (src/core/cache.rs:1496-1499, counters byte_budget_rejections and oversize_rejections). Independently, an origin body
  // above MAX_BODY_BYTES = 2 MiB is refused with 502 X-Cache-Error: ResponseTooLarge (src/api/server.rs:2616, 3122, 3156, 2499-2503).
  await bigObjects(BUDGET);
  const alive = await httpReq({ port: GPORT, path: '/health/ready' });
  check(alive.status === 200, 'gateway still ready after the oversize requests', `${alive.status}`);

  // ---- Phase 4: leak check
  let del = 0;
  for (const p of objs.keys()) { const r = await httpReq({ port: GPORT, method: 'DELETE', path: `/api/v1/cache${p}` }); if (r.status === 204) del++; }
  await sleep(300);
  const s4 = await snap();
  log(`after DELETE of ${del}/${objs.size} keys: bytes_resident ${s4.resident}, evictions ${s4.evictions}, RSS ${s4.rss}`);
  check(del === objs.size, 'DELETE answered 204 for every key', `${del}/${objs.size}`);
  check(s4.resident <= 4096, 'leak check: bytes_resident returns to ~0 after every key was deleted (<= 4 KiB)', `${s4.resident}`);
  R.data.after_delete = s4;
  R.data.totals = { requests, bytes_sent_to_clients: bytesSent.v, integrity_failures: integrityFailures };
  // ---- Phase 5: same large objects with the DEFAULT budget (256 MiB): per-shard 32 MiB
  await gw.kill();
  gw = spawnGateway({ exe: EXE, port: GPORT, originUrl: origin.url, outDir: OUT, name: 'gw-default' });
  check((await waitReady(GPORT)) !== null, 'gateway ready with the default PHANTOM_MAX_CACHE_BYTES');
  await bigObjects(256 * MIB);
  check(integrityFailures === 0, 'integrity: zero failures over the whole scenario', `${integrityFailures}`);
}

let ok = false;
try { await main(); } catch (e) { check(false, 'unexpected error', String(e && e.stack ? e.stack : e)); }
finally { await gw?.kill(); try { await origin.close(); } catch { /* */ } ok = R.finish(); }
process.exit(ok ? 0 : 1);
