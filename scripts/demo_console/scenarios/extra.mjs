#!/usr/bin/env node
// scripts/demo_console/scenarios/extra.mjs -- Demo Hub scenarios that need their own gateway (or a live look at the demo cluster).
//   node scripts/demo_console/scenarios/extra.mjs --id largeobjects|originprotect|restart|intelligence|intelcontrol|dashboards
//        --exe <gateway> [--dir <cluster dir>] [--out-dir <dir>] [--port 18340] [--origin-port 19300] [--events]
// Every number comes from the gateway or the origin process itself. Own-gateway scenarios use the 183xx/193xx block and clean up.
import http from 'node:http';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { argv, httpReq, sleep, sha256, spawnGateway, waitReady, recorder, makePngOfSize, makeJson, startOrigin } from '../../lib/scenario_common.mjs';
import { ev, pickHeaders } from '../../lib/events.mjs';

const { opt } = argv();
const ID = opt('id'); const EXE = opt('exe'); const OUT = opt('out-dir', join(opt('dir', '.'), 'hub', ID || 'extra'));
const PORT = Number(opt('port', 18340)); const OPORT = Number(opt('origin-port', 19300));
const R = recorder(ID, OUT, { prefix: String(ID).toUpperCase() });
const cache = (p) => `/api/v1/cache${p}`;
const line = (r) => (r.ok ? `${r.status} ${r.headers['x-cache-status'] || ''}${r.headers['x-cache-source'] ? ' source ' + r.headers['x-cache-source'] : ''}, ${r.ms.toFixed(0)} ms, ${r.body.length} bytes` : `no answer (${r.error})`);
const say = (text, means, r) => ev.observe(text, { ...(means ? { means } : {}), ...(r && r.ok ? { response: { status: r.status, ms: +r.ms.toFixed(1), bytes: r.body.length, headers: pickHeaders(r.headers) } } : {}) });
const cleanups = [];
const finish = async () => { for (const c of cleanups.reverse()) { try { await c(); } catch { /* ignore */ } } const ok = R.finish(); process.exit(ok ? 0 : 1); };
async function ownGateway(env, originSetup) {
  const origin = startOrigin({ port: OPORT }); originSetup && originSetup(origin); await origin.listen(); cleanups.push(() => origin.close());
  const gw = spawnGateway({ exe: EXE, port: PORT, originUrl: origin.url, env, outDir: OUT, name: 'gateway' }); cleanups.push(() => gw.kill());
  const ready = await waitReady(PORT, 20000);
  if (ready === null) { R.check(false, 'the gateway started and is ready', gw.logs().slice(0, 200)); await finish(); }
  ev.action(`Started its own gateway on port ${PORT} with an origin on port ${OPORT}`, { means: 'A separate pair from the demo cluster, removed again at the end.' });
  return { origin, gw };
}

// ------------------------------------------------------------------ largeobjects
async function largeobjects() {
  const BUDGET = 8 * 1024 * 1024;
  const files = { '/big/half.png': makePngOfSize(500 * 1024, 11), '/big/medium.png': makePngOfSize(1300 * 1024, 12), '/big/huge.png': makePngOfSize(2600 * 1024, 13) };
  const { origin } = await ownGateway({ PHANTOM_MAX_CACHE_BYTES: String(BUDGET) }, (o) => { for (const [p, b] of Object.entries(files)) o.set(p, { body: b, type: 'image/png', cacheControl: 'public, max-age=300', etag: `"${sha256(b).slice(0, 16)}"` }); });
  ev.action(`Memory budget set to ${BUDGET / 1048576} MiB for this gateway. Rule under test: one entry may use at most budget/8 = 1 MiB, and the gateway refuses origin bodies above 2 MiB`, { means: 'A single huge object must never push everything else out of memory.' });
  const res = {};
  for (const [p, b] of Object.entries(files)) {
    ev.action(`GET ${p} (${(b.length / 1048576).toFixed(2)} MiB) twice`);
    const a1 = await httpReq({ port: PORT, path: cache(p) }); say(`first read -> ${line(a1)}`, null, a1);
    const a2 = await httpReq({ port: PORT, path: cache(p) }); say(`second read -> ${line(a2)}`, null, a2);
    res[p] = { a1, a2 };
  }
  const h = res['/big/half.png'], m = res['/big/medium.png'], g = res['/big/huge.png'];
  R.check(h.a1.status === 200 && sha256(h.a1.body) === sha256(files['/big/half.png']) && h.a2.headers['x-cache-status'] === 'HIT', 'a 0.5 MiB picture is served byte-identical and remembered (second read is a HIT)', `second: ${h.a2.headers['x-cache-status']}`);
  ev.observe('The 0.5 MiB picture is under the per-entry limit, so it is kept.', { means: 'Normal objects are cached normally.' });
  R.check(m.a1.status === 200 && sha256(m.a1.body) === sha256(files['/big/medium.png']) && m.a2.headers['x-cache-status'] !== 'HIT', 'a 1.3 MiB picture is served correctly but not kept (above budget/8), so the second read goes to the origin again', `second: ${m.a2.headers['x-cache-status']}/${m.a2.headers['x-cache-source']}`);
  R.check(g.a1.status === 502 && g.a2.status === 502, 'a 2.6 MiB picture (above the 2 MiB origin body limit) is refused with a clean 502, no crash', `statuses ${g.a1.status}, ${g.a2.status}`);
  const after = await httpReq({ port: PORT, path: cache('/big/half.png') });
  R.check(after.status === 200 && after.headers['x-cache-status'] === 'HIT', 'the gateway still serves the kept picture afterwards', line(after));
  R.finding('LARGE', 'Objects above the per-entry limit are served but not cached; above 2 MiB they are refused (502).');
}

// ------------------------------------------------------------------ originprotect
async function originprotect() {
  const CAP = Number(opt('cap', 6)); const N = 60; const DELAY = 400;
  let inflight = 0, peak = 0, served = 0;
  const origin = http.createServer((req, res) => { inflight += 1; peak = Math.max(peak, inflight); setTimeout(() => { const b = Buffer.from(JSON.stringify({ path: req.url })); res.setHeader('content-type', 'application/json'); res.setHeader('cache-control', 'public, max-age=300'); res.setHeader('content-length', b.length); res.end(b); inflight -= 1; served += 1; }, DELAY); });
  await new Promise((ok) => origin.listen(OPORT, '127.0.0.1', ok)); cleanups.push(() => new Promise((ok) => { origin.closeAllConnections?.(); origin.close(ok); }));
  const gw = spawnGateway({ exe: EXE, port: PORT, originUrl: `http://127.0.0.1:${OPORT}`, env: { PHANTOM_ORIGIN_CAP_MAX: String(CAP) }, outDir: OUT, name: 'gateway' }); cleanups.push(() => gw.kill());
  if ((await waitReady(PORT, 20000)) === null) { R.check(false, 'the gateway started with the origin cap set', gw.logs().slice(0, 300)); return; }
  ev.action(`Own gateway started with PHANTOM_ORIGIN_CAP_MAX=${CAP}; the origin takes ${DELAY} ms per request and records its own peak of simultaneous requests`, { means: 'The cap is a fixed limit on how many requests the gateway lets reach the origin at the same time.' });
  ev.action(`Sending ${N} simultaneous requests for ${N} different, never-seen keys`, { means: 'Without protection the origin would see all 60 at once.' });
  const rs = await Promise.all(Array.from({ length: N }, (_, i) => httpReq({ port: PORT, path: cache(`/cold/${Date.now().toString(36)}-${i}.json`) })));
  const by = {}; for (const r of rs) { const k = r.ok ? r.status : 'none'; by[k] = (by[k] || 0) + 1; }
  ev.observe(`gateway answers: ${JSON.stringify(by)}; the origin's own peak of simultaneous requests: ${peak} (cap ${CAP}); origin served ${served}`, { means: '503 answers are the cap doing its job: the gateway says "try again" instead of overloading the origin.' });
  R.check(peak <= CAP, `the origin never saw more than ${CAP} simultaneous requests (its own peak: ${peak})`, `peak ${peak}`);
  R.check(rs.every((r) => r.ok && (r.status === 200 || r.status === 503)), 'every answer was either 200 or a clean 503', JSON.stringify(by));
  await sleep(1200);
  // Normal traffic is a few requests at a time, never more than the cap allows at once: send 20 new keys in small groups.
  const again = [];
  for (let g = 0; g < 5; g++) {
    const grp = await Promise.all(Array.from({ length: 4 }, (_, i) => httpReq({ port: PORT, path: cache(`/cold/after-${Date.now().toString(36)}-${g}-${i}.json`) })));
    again.push(...grp);
  }
  R.check(again.filter((r) => r.status === 200).length >= 19, 'after the burst the gateway serves new keys normally again (a few at a time)', `${again.filter((r) => r.status === 200).length} of 20 ok`);
}

// ------------------------------------------------------------------ restart
async function restart() {
  const body = makeJson(5, 600); const sum = sha256(Buffer.from(body));
  const o = startOrigin({ port: OPORT }); o.set('/r/a.json', { body: Buffer.from(body), type: 'application/json', cacheControl: 'public, max-age=300' }); await o.listen(); cleanups.push(() => o.close());
  let gw = spawnGateway({ exe: EXE, port: PORT, originUrl: o.url, outDir: OUT, name: 'gateway1' }); cleanups.push(() => gw.kill());
  if ((await waitReady(PORT, 20000)) === null) { R.check(false, 'the gateway started', ''); return; }
  ev.action('A client now sends requests continuously (20 at a time) while we kill the gateway and start it again', { means: 'This is a crash and a restart under load.' });
  const t0 = Date.now(); const log = []; let stop = false;
  const worker = async () => { while (!stop) { const r = await httpReq({ port: PORT, path: cache('/r/a.json'), timeoutMs: 3000 }); log.push({ t: Date.now() - t0, ok: r.ok && r.status === 200, wrong: r.ok && r.status === 200 && sha256(r.body) !== sum, err: r.ok ? r.status : r.error }); await sleep(5); } };
  const ws = Array.from({ length: 20 }, worker);
  await sleep(1500); ev.action('Killing the gateway process now'); const tk = Date.now() - t0; await gw.kill();
  await sleep(1500); ev.action('Starting the gateway again'); const ts = Date.now() - t0;
  gw = spawnGateway({ exe: EXE, port: PORT, originUrl: o.url, outDir: OUT, name: 'gateway2' }); cleanups.push(() => gw.kill());
  const ready = await waitReady(PORT, 20000); const tr = Date.now() - t0;
  await sleep(1500); stop = true; await Promise.all(ws);
  const before = log.filter((x) => x.t < tk), outage = log.filter((x) => x.t >= tk && x.t < tr), after = log.filter((x) => x.t >= tr + 100);
  ev.observe(`before the kill: ${before.length} requests, ${before.filter((x) => !x.ok).length} problems. During the outage (${tr - tk} ms): ${outage.length} attempts, ${outage.filter((x) => x.ok).length} answered. After it was ready again: ${after.length} requests, ${after.filter((x) => !x.ok).length} problems`, { means: 'While the process is gone nothing can answer; the point is a clean outage and a clean recovery.' });
  R.check(ready !== null, 'the restarted gateway became ready', `ready ${ready} ms after start`);
  R.check(before.every((x) => x.ok) && after.length > 20 && after.every((x) => x.ok), 'no problems before the kill and none after the restart', `${after.length} requests after recovery`);
  R.check(!log.some((x) => x.wrong), 'no wrong bytes were ever served (sha256 checked on every answer)', `${log.length} requests`);
  R.check(outage.some((x) => !x.ok), 'the outage was visible to the client as clean connection failures, not wrong answers', `${outage.filter((x) => !x.ok).length} failed attempts, e.g. ${(outage.find((x) => !x.ok) || {}).err}`);
}

// ------------------------------------------------------------------ intelligence + control
async function jget(path) { const r = await httpReq({ port: PORT, path }); let j = null; try { j = JSON.parse(r.body.toString()); } catch { /* not json */ } return { r, j }; }
async function intelBase() {
  const objs = []; for (let i = 0; i < 300; i++) objs.push(i);
  const { origin } = await ownGateway({ PHANTOM_INTELLIGENCE: 'advisory' }, (o) => { for (const i of objs) { const b = Buffer.from(makeJson(i, 400)); o.set(`/k/${i}.json`, { body: b, type: 'application/json', cacheControl: 'public, max-age=300' }); } });
  const reg = await jget('/api/v1/intelligence/registry');
  if (reg.r.status !== 200 || !reg.j) { R.check(false, 'this gateway build serves the policy engine routes (registry)', `answered ${reg.r.status}: this build does not include the policy engine API yet`); return null; }
  const policies = reg.j.policies || reg.j.registry || [];
  ev.observe(`registry: version ${reg.j.registry_version}, max mode ${reg.j.max_mode}, ${Array.isArray(policies) ? policies.length : '?'} policies declared`, { means: 'The engine can only observe and advise. It is shown here as Advisory; nothing is ever applied.' });
  R.check(reg.j.max_mode === 'advisory' || reg.j.max_mode === 'observe', 'the registry reports an Observe/Advisory-only engine (no applying mode)', String(reg.j.max_mode));
  return { origin };
}
const drive = async (ms, pick) => { const t0 = Date.now(); let n = 0; while (Date.now() - t0 < ms) { await Promise.all(Array.from({ length: 10 }, () => httpReq({ port: PORT, path: cache(pick(n++)) }))); await sleep(20); } return n; };
async function showDecisions(label) {
  const d = await jget('/api/v1/intelligence/decisions?limit=50'); const recs = (d.j && (d.j.records || d.j.decisions)) || [];
  const by = {}; for (const x of recs) by[x.verdict] = (by[x.verdict] || 0) + 1;
  ev.observe(`${label}: ${recs.length} decision records, verdicts ${JSON.stringify(by)}`, { means: 'Each record names the policy, the inputs it used and why. "applied: false" means nothing was changed.' });
  for (const x of recs.slice(-4)) ev.observe(`${x.policy_id}: ${x.verdict} (${x.reason_code})${x.recommendation ? ' - ' + x.recommendation.text : ''}`, { means: x.recommendation ? 'A shadow suggestion, labelled advisory, not applied.' : 'The engine looked and decided nothing needs to change.' });
  return recs;
}
async function intelligence() {
  if (!(await intelBase())) return;
  ev.action('Sending real traffic through the gateway for about 14 seconds: a popular-key mix over 300 objects', { means: 'The engine needs real traffic to have anything to observe.' });
  const zipf = (n) => Math.floor(300 * Math.pow(Math.random(), 3));
  const n = await drive(14000, (i) => `/k/${zipf(i)}.json`);
  ev.observe(`${n} requests sent`);
  const sig = await jget('/api/v1/intelligence/signals');
  R.check(sig.r.status === 200 && sig.j && sig.j.window !== undefined, 'live signals are served (request rate, hit ratio, concentration...)', `status ${sig.r.status}`);
  if (sig.j && sig.j.window) ev.observe(`latest window: ${sig.j.window.events} events; features ${JSON.stringify(Object.fromEntries(Object.entries(sig.j.window.features || {}).slice(0, 4).map(([k, v]) => [k, v && v.v !== undefined ? v.v : v])))}`, { means: 'These are measured from the real requests above.' });
  const recs = await showDecisions('decisions so far');
  R.check(recs.every((x) => x.recommendation == null || x.recommendation.applied === false), 'every recommendation carries applied: false (nothing was changed)', `${recs.length} records`);
  const st = await jget('/api/v1/intelligence/status');
  R.check(st.r.status === 200 && st.j && /advisory|observe/i.test(String(st.j.intelligence_mode || st.j.mode || '')), 'the mode shown is Observe or Advisory only', JSON.stringify(st.j).slice(0, 160));
}
async function intelcontrol() {
  if (!(await intelBase())) return;
  { const c0 = await jget('/api/v1/intelligence/signals'); const d0 = await jget('/api/v1/intelligence/decisions?limit=20'); const cl = c0.j && c0.j.classification; const recs0 = (d0.j && (d0.j.records || d0.j.decisions)) || [];
    ev.observe(`Cold start, before any traffic: classification is ${cl && cl.value ? cl.value : 'unavailable (' + ((cl && cl.reason) || 'not enough evidence') + ')'}, ${recs0.length} decision records`, { ribbon: 'OBSERVED', means: 'With no evidence the engine says so instead of guessing: NoAction, not enough evidence. APPLIED: none: the engine never changes how requests are served.' });
    R.check(!(cl && cl.value) || recs0.length === 0, 'with no traffic the engine makes no classification or recommendation (not enough evidence)', `classification ${cl && cl.value}, ${recs0.length} records`); }
  ev.action('Phase 1: a CONSTANT workload (the same 5 keys) for about 22 seconds', { means: 'A steady workload gives the engine no reason to suggest anything: the negative control.' });
  await drive(22000, (i) => `/k/${i % 5}.json`);
  const a = await showDecisions('after the constant workload');
  const rec1 = a.filter((x) => x.verdict === 'Recommend').length;
  R.check(rec1 === 0, 'the constant workload produced no recommendation (NoAction/Observation only)', `${rec1} Recommend of ${a.length}`);
  ev.action('Phase 2: the workload CHANGES to all-different keys (a sudden scan) for about 22 seconds', { means: 'Now the hit ratio collapses and concentration disappears: a real change.' });
  await drive(22000, (i) => `/k/${(i * 7) % 300}.json`);
  const b = await showDecisions('after the workload change');
  R.check(b.length > a.length || b.some((x) => x.verdict === 'Recommend'), 'the change produced new decision records (a recommendation or a recorded observation)', `${a.length} records before, ${b.length} after; Recommend: ${b.filter((x) => x.verdict === 'Recommend').length}`);
  R.check(b.every((x) => x.recommendation == null || x.recommendation.applied === false), 'still nothing applied', '');
}

// ------------------------------------------------------------------ dashboards (demo cluster, live)
async function dashboards() {
  const { loadCluster, runLoad, originManifest, gatewayOutcomes } = await import(pathToFileURL(join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'lib', 'traffic.mjs')).href);
  const st = loadCluster(opt('dir')); const token = st.secure ? st.tokens.tenantA.token : null; const manifest = await originManifest(st.originPort); const keys = Object.keys(manifest);
  const tot = async () => { let t = 0; for (const p of st.ports) { const o = await gatewayOutcomes(p, token); const r = o.responses || {}; for (const k of Object.keys(r)) { const x = r[k]; t += x && typeof x.v === 'number' ? x.v : 0; } } return t; };
  const t0 = await tot();
  ev.action('Sending steady real traffic (about 20 seconds) through all three gateways. OPEN THE CONTROL CENTER NOW: Overview, Traffic, Cache and memory, Freshness, Origin and Cluster all move while this runs', { means: 'The numbers on those screens are the same counters printed below.' });
  const nonce = Date.now().toString(36); let stop = false;
  const plan = Array.from({ length: 6000 }, (_, i) => ({ path: keys[(i * 37 + (i % 7) * 11) % keys.length] }));
  const load = runLoad(plan, { concurrency: 8, manifest, nonce, token, pickPort: (i) => st.ports[i % st.ports.length], ports: st.ports, sampleEvery: 1000, onSample: ({ path, r, port }) => ev.observe(`sampled: GET ${path} via node ${st.ports.indexOf(port) + 1} -> ${r.status} ${r.headers && r.headers['x-cache-status']} ${r.headers && r.headers['x-cache-source']}`) });
  const tick = (async () => { let last = t0; for (let i = 0; i < 6 && !stop; i++) { await sleep(3000); const t = await tot(); ev.observe(`gateways have answered ${t - t0} requests so far (+${t - last} in the last 3 s)`, { means: 'Measured by the gateways; the Control Center reads the same counters.' }); last = t; } })();
  const res = await load; stop = true; await tick; const t1 = await tot();
  R.check(res.errors === 0 && res.integrityFailures === 0, 'all requests succeeded and every body matched the origin checksum', `${res.total} requests, ${res.errors} errors, ${res.integrityFailures} integrity failures`);
  R.check(t1 - t0 >= res.total * 0.95, 'the gateways counted the traffic (what the dashboards show matches what was sent)', `sent ${res.total}, gateways counted ${t1 - t0}`);
}

const fns = { largeobjects, originprotect, restart, intelligence, intelcontrol, dashboards };
if (!fns[ID]) { console.error('usage: --id ' + Object.keys(fns).join('|')); process.exit(1); }
if (ID !== 'dashboards' && (!EXE)) { console.error('--exe required'); process.exit(1); }
try { await fns[ID](); } catch (e) { R.check(false, 'the scenario hit an unexpected problem', String(e && e.message || e).slice(0, 200)); }
await finish();
