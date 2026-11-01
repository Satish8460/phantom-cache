#!/usr/bin/env node
// scripts/lib/traffic_cli.mjs
//
// Runs ONE Zipf-distributed mixed workload against a launcher cluster and writes a JSON result (and live progress) file.
// Used by the demo console ("send 1,000 / 5,000 requests") and usable by hand:
//   node scripts/lib/traffic_cli.mjs --dir <cluster dir> --requests 1000 --concurrency 100 [--user tenantA|none] [--cold 1] [--out file]
// Every response is verified against the origin manifest (sha256, bytes, headers). Numbers are MEASURED by this process
// (loopback, one host); with --cold 1 every key starts cold (fresh `?r=` nonce), otherwise the keys may already be cached.

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { loadCluster, runLoad, originManifest, originStats, zipfSampler, rssKb } from './traffic.mjs';

const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const DIR = opt('dir');
const N = Number(opt('requests', 1000));
const C = Number(opt('concurrency', 100));
const USER = opt('user', 'tenantA');
const OUT = opt('out', join(DIR, 'last_burst.json'));
const st = loadCluster(DIR);
const token = st.secure && USER !== 'none' ? st.tokens[USER].token : null;

const manifest = await originManifest(st.originPort);
const keys = Object.keys(manifest);
const pick = zipfSampler(keys, { s: 1.0, seed: Number(opt('seed', Date.now() % 100000)) });
const plan = Array.from({ length: N }, () => ({ path: pick() }));
const nonce = opt('cold', '1') === '1' ? `burst-${Date.now().toString(36)}` : null;
const o0 = (await originStats(st.originPort)).requests;
const progress = (done) => writeFileSync(OUT, JSON.stringify({ state: 'running', done, total: N, concurrency: C, user: USER }));
progress(0);
const live = st.ports.filter((_, i) => { try { process.kill(st.pids[`node${i + 1}`], 0); return true; } catch { return false; } });
const rr = (i, dead, tried) => { const l = live.filter((p) => !dead.has(p) && !tried.includes(p)); return l[i % l.length]; };
const r = await runLoad(plan, { concurrency: C, manifest, nonce, token, pickPort: rr, ports: live, failover: true, onTick: progress });
const o1 = (await originStats(st.originPort)).requests;
const total = r.total;
writeFileSync(OUT, JSON.stringify({
  state: 'done', label: 'measured by the traffic client on this host (loopback); release gateway; closed-loop client-side latency', user: USER, coldKeys: Boolean(nonce),
  total, concurrency: C, ok: r.ok, errors: r.errors, integrityFailures: r.integrityFailures, errorSamples: r.errorSamples, integritySamples: r.integritySamples,
  sources: r.sources, bytes: r.bytes, seconds: r.seconds, rps: r.rps, latencyMs: r.latencyMs, originRequestsCountedByOrigin: o1 - o0, clientFailoverRetries: r.retried,
  hitRatioDerived: +((((r.sources.local || 0) + (r.sources.peer || 0) + (r.sources.coalesced || 0)) / total) * 100).toFixed(1),
  rssKbAfter: Array.from({ length: st.nodes }, (_, i) => rssKb(st.pids[`node${i + 1}`])),
}, null, 2));
