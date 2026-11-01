#!/usr/bin/env node
// scripts/demo_console/server.mjs  --  Demo Hub driver (loopback only)
//   node scripts/demo_console/server.mjs --dir <cluster dir> [--port 3201] [--exe <gateway>] [--ui-base http://localhost:3000]
//         [--http-port-base N] [--cluster-port-base N] [--origin-port N]
// Serves the hub page, proxies gallery requests (adds Authorization for the selected demo user; the page never receives a
// token except through the explicit /api/token endpoint used for the Control Center postMessage hand-off), runs the REAL
// scenario scripts as child processes with `--events`, and streams their structured events to the page with Server-Sent Events.
// Nothing here is simulated or canned: every event comes from a script that talked to the running gateways.

import { createServer } from 'node:http';
import { spawn, spawnSync } from 'node:child_process';
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { httpRequest, cachePath, getJson, originStats, originManifest, loadCluster } from '../lib/traffic.mjs';
import { decodeClaimsForDisplay } from '../lib/jwt_mint.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPTS = join(HERE, '..');
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const DIR = resolve(opt('dir', ''));
if (!opt('dir')) { console.error('usage: node scripts/demo_console/server.mjs --dir <cluster dir> [--port 3201] [--exe gw] ...'); process.exit(1); }
const PORT = Number(opt('port', 3201));
const UI_BASE = opt('ui-base', 'http://localhost:3000');
const OFFSET = Number(opt('scenario-offset', 0)); // shifts the 183xx/193xx/197xx block when two hubs run on one machine
const P = (n) => String(n + OFFSET);
const HUBDIR = join(DIR, 'hub'); mkdirSync(HUBDIR, { recursive: true });
const launcher = (args) => spawnSync(process.execPath, [join(SCRIPTS, 'cluster_launcher.mjs'), ...args, '--dir', DIR], { encoding: 'utf8' });
const cluster = () => (existsSync(join(DIR, 'cluster.json')) ? loadCluster(DIR) : null);
const alive = (pid) => { if (!pid) return false; try { process.kill(pid, 0); return true; } catch { return false; } };
const exeOf = () => opt('exe') || (cluster() && cluster().exe) || '';

// ------------------------------------------------------------------ the scenario catalogue (real scripts only)
// Scenarios that spawn their own gateways use the 183xx/197xx/193xx block, never the demo cluster's ports.
const RR = (phase, extra = []) => () => ['real_requests_scenario.mjs', '--dir', DIR, '--only', phase, '--out', join(HUBDIR, `${phase}.result.json`), ...extra];
const SEC = (part) => () => ['security_scenario.mjs', '--dir', DIR, '--exe', exeOf(), '--part', part, '--no-browser', '--out', join(HUBDIR, `security-${part}.result.json`)];
const EX = (id, extra = []) => () => ['demo_console/scenarios/extra.mjs', '--id', id, '--exe', exeOf(), '--dir', DIR, '--out-dir', join(HUBDIR, id), '--port', P(18340), '--origin-port', P(19300), ...extra];
const own = 'starts its own gateway on separate ports and removes it afterwards';
export const CATALOGUE = [
  { id: 'tour', group: 'Start', title: 'Guided tour (5 minutes)', client: true, needsCluster: false, style: 'passed', est: '5 min', what: 'A step-by-step walk through the key scenarios, with two sentences of explanation after each.', look: 'Use Next and Back; each step runs the real scenario.', proves: 'You can see the whole story without reading anything else.' },
  { id: 'gallery', group: 'Speed and savings', title: 'Cold vs warm gallery', needsCluster: true, style: 'passed', est: '10 s', what: 'Twelve real pictures are loaded twice: the first time the origin is asked for each, the second time it is not asked at all.', look: 'The source badge changes from "origin" to "local" and the milliseconds drop.', proves: 'Repeat visits are fast and free for the origin, and the bytes are identical.', args: RR('gallery') },
  { id: 'stampede', group: 'Speed and savings', title: 'Stampede: 300 at once', needsCluster: true, style: 'passed', est: '10 s', what: '300 simultaneous requests for one picture nobody has asked for yet.', look: 'The origin counter rises by one per gateway at most, not by 300.', proves: 'A crowd cannot overload the origin: the others wait for the first answer.', args: RR('stampede') },
  { id: 'peerfill', group: 'Speed and savings', title: 'Peer fill: a neighbour answers', needsCluster: true, style: 'shows', shows: 'a neighbour gateway can answer without the origin', est: '15 s', what: 'A second gateway gets a picture from its neighbour instead of the origin.', look: 'The "peer" source badges and the origin count.', proves: 'Gateways share what they know, so the origin is asked once per picture.', args: RR('peerfill') },
  { id: 'offload', group: 'Speed and savings', title: 'Origin offload', needsCluster: true, style: 'passed', est: '15 s', what: '3,000 client requests; the origin counts how many actually reached it.', look: 'Requests sent versus requests the origin counted.', proves: 'Most of the work never reaches the slow server.', args: RR('offload') },
  { id: 'workloads', group: 'Speed and savings', title: 'Workload shapes', needsCluster: true, style: 'shows', shows: 'how hit ratio follows the shape of the traffic', est: '25 s', what: 'Hot keys, a popularity mix, a scan of all-different keys and a cold burst.', look: 'Hit ratio per shape.', proves: 'The gateway helps most where keys repeat, and is honest where they do not.', args: RR('workloads') },
  { id: 'scale', group: 'Speed and savings', title: 'Real requests at scale', needsCluster: true, style: 'passed', est: '30 s', what: '1,000, then 5,000, then 10,000 requests from a Node client over 315 real objects.', look: 'Errors and integrity failures are 0.', proves: 'Thousands of concurrent requests are served correctly.', args: RR('mixed', ['--sizes', '1000,5000,10000', '--reps', '1']) },
  { id: 'auth', group: 'Safety', title: 'Who may come in', needsCluster: true, style: 'passed', est: '10 s', what: 'No token, an expired token, the wrong audience, a forged token and a token without a customer.', look: 'Each is blocked as expected: this is the security working.', proves: 'Only a valid, current, correctly addressed token gets in.', args: SEC('auth') },
  { id: 'roles', group: 'Safety', title: 'Roles: read-only vs invalidate', needsCluster: true, style: 'passed', est: '10 s', what: 'A read-only token can read but its delete is refused; metrics need a token.', look: 'The refused delete leaves the cached entry in place.', proves: 'What you may do depends on your role.', args: SEC('roles') },
  { id: 'isolation', group: 'Safety', title: 'Tenant isolation', needsCluster: true, style: 'passed', est: '10 s', what: 'Tenant B asks for the same address tenant A just warmed.', look: 'B gets its own MISS; A\'s delete leaves B\'s copy alone.', proves: 'Customers never see each other\'s cached data.', args: SEC('isolation') },
  { id: 'targetsafety', group: 'Safety', title: 'Unsafe addresses', needsCluster: true, style: 'passed', est: '10 s', what: 'Addresses with line breaks, NUL, spaces and dot-segments.', look: 'All blocked as expected; the origin counter does not move.', proves: 'Malformed requests are stopped at the gateway.', args: SEC('targetsafety') },
  { id: 'mtls', group: 'Safety', title: 'Cluster certificates (mTLS)', needsCluster: true, style: 'passed', est: '10 s', what: 'A real member certificate is answered; four kinds of intruder certificate are not.', look: 'One answered, four blocked as expected.', proves: 'Only known gateways can talk to each other.', args: SEC('mtls') },
  { id: 'failclosed', group: 'Safety', title: 'Fail closed at start-up', needsCluster: false, style: 'passed', est: '10 s', what: 'A gateway that must require auth, or listens on the network, refuses to start without a secret.', look: 'Exit code 2 and a plain message.', proves: 'A misconfiguration cannot silently run open.', args: SEC('failclosed') },
  { id: 'multiuser', group: 'Safety', title: 'Two people, two windows', native: 'multiuser', needsCluster: true, style: 'shows', shows: 'two Control Center windows, two identities', est: '30 s', what: 'Opens the Control Center as tenant A and as tenant B; each receives its own token by a one-time message, never in the address.', look: 'Allow the pop-ups; each window shows who it is.', proves: 'Several people can use the same gateways with separate identities.' },
  { id: 'invalidation', group: 'Resilience', title: 'Delete a key everywhere', needsCluster: true, style: 'passed', est: '10 s', what: 'A picture is cached on all three gateways, then deleted through one.', look: 'Milliseconds until the others stop serving it.', proves: 'A delete reaches every gateway quickly (measured here, not promised).', args: RR('invalidation') },
  { id: 'deadnode', group: 'Resilience', title: 'Kill a node while traffic runs', needsCluster: true, style: 'passed', est: '25 s', what: 'Traffic keeps flowing while node 3 is killed, then restarted.', look: 'No errors on healthy nodes; node 3 returns to Active.', proves: 'A crashed gateway does not stop service.', args: RR('deadnode') },
  { id: 'deletestorm', group: 'Resilience', title: 'Delete storm with a dead peer', needsCluster: true, style: 'shows', shows: 'purges while a peer is down', est: '20 s', what: '400 concurrent deletes while node 3 is down.', look: 'All acknowledged; the live peer serves nothing stale.', proves: 'Purges are honoured even when a gateway is down.', args: RR('deletes') },
  { id: 'burstdelete', group: 'Resilience', title: 'Burst of deletes', needsCluster: true, style: 'passed', est: '15 s', what: '200 concurrent deletes with every gateway alive.', look: 'Every member is Active afterwards.', proves: 'A purge storm does not unsettle the cluster.', args: RR('burstdelete') },
  { id: 'peerexpiry', group: 'Resilience', title: 'Neighbour copies expire in time', needsCluster: true, style: 'passed', est: '20 s', what: 'An object with an 8-second lifetime is read on three gateways, then read again after it expired.', look: 'No gateway serves it as a plain hit after expiry.', proves: 'A copy from a neighbour does not outlive its source.', args: RR('peerexpiry') },
  { id: 'freshness', group: 'Freshness and memory', title: 'Freshness suite', needsCluster: false, style: 'passed', est: '60 s', what: 'Stale-while-revalidate, stale-if-error, no-store and private, ETag revalidation and the staleness cap.', look: 'Each line names the rule it tests.', proves: 'Content is kept as long as, and no longer than, the origin allows.', args: () => ['freshness_scenario.mjs', '--exe', exeOf(), '--out-dir', join(HUBDIR, 'freshness'), '--origin-port', P(19300), '--port-base', P(18300), '--events'] },
  { id: 'eviction', group: 'Freshness and memory', title: 'Memory budget and eviction', needsCluster: false, style: 'passed', est: '60 s', what: 'A gateway with a small memory budget is flooded with pictures.', look: 'Memory stays under the budget; popular pictures survive.', proves: 'Memory use is bounded.', args: () => ['eviction_scenario.mjs', '--exe', exeOf(), '--out-dir', join(HUBDIR, 'eviction'), '--origin-port', P(19300), '--port', P(18310), '--events'] },
  { id: 'largeobjects', group: 'Freshness and memory', title: 'Large objects (the 1 MiB rule)', needsCluster: false, style: 'passed', est: '15 s', what: 'A 0.5, a 1.3 and a 2.6 MiB picture through a gateway with an 8 MiB budget.', look: 'Kept, served but not kept, and refused cleanly.', proves: 'One huge object cannot push everything else out.', args: EX('largeobjects') },
  { id: 'originprotect', group: 'Protection', title: 'Origin protection (static cap)', needsCluster: false, style: 'passed', est: '15 s', what: 'A slow origin and 60 simultaneous cold keys with a cap of 6.', look: 'The origin\'s own peak never exceeds the cap.', proves: 'The gateway protects a fragile origin from overload.', args: EX('originprotect') },
  { id: 'operability', group: 'Operability', title: 'Fail fast, health, abuse', needsCluster: false, style: 'passed', est: '60 s', what: 'Bad configuration is refused at start-up, health answers, abusive requests are rejected.', look: 'Exit code 2 for bad config; 4xx never 5xx for abuse.', proves: 'It is safe and easy to operate.', args: () => ['operability_scenario.mjs', '--exe', exeOf(), '--out-dir', join(HUBDIR, 'operability'), '--origin-port', P(19300), '--port', P(18320), '--events'] },
  { id: 'restart', group: 'Operability', title: 'Kill and restart under load', needsCluster: false, style: 'passed', est: '15 s', what: 'A gateway is killed and restarted while a client keeps asking.', look: 'A clean outage and a clean recovery; no wrong bytes.', proves: 'Restarting is safe.', args: EX('restart') },
  { id: 'intelligence', group: 'Intelligence', title: 'Policy engine (observe and advise)', needsCluster: false, style: 'shows', shows: 'the engine observing real traffic and advising (applied: none)', est: '30 s', what: 'Real traffic is sent to a gateway with the engine in Advisory mode.', look: 'Registry, live signals and decision records; applied is always none.', proves: 'The engine only observes and advises; it never changes the cache.', args: EX('intelligence') },
  { id: 'intelcontrol', group: 'Intelligence', title: 'Policy engine: negative control', needsCluster: false, style: 'shows', shows: 'a steady workload is quiet, a changed one is noticed', est: '60 s', what: 'A constant workload, then a sudden change of workload.', look: 'No recommendation first, records after the change.', proves: 'The engine reacts to real change, not to noise.', args: EX('intelcontrol') },
  { id: 'pie-hotkey', group: 'Intelligence', title: 'Hot key detection', pie: true, needsCluster: true, style: 'shows', shows: 'a hot key found from real traffic, with an honest error bound', est: '16 s', what: 'One key gets about a third of the real requests, 200 other keys share the rest.', look: 'The hot key (hash only), its share and confidence, the classification, and the distinct-keys estimate next to the true number.', proves: 'The engine can see a dominating key and count keys from a tiny sketch, and it only proposes.', args: () => ['demo_console/scenarios/pie.mjs', '--id', 'pie-hotkey', '--dir', DIR, '--out-dir', join(HUBDIR, 'pie-hotkey')] },
  { id: 'pie-flashcrowd', group: 'Intelligence', title: 'Flash crowd detection', pie: true, needsCluster: true, style: 'shows', shows: 'what the flash-crowd detector saw during a real burst', est: '25 s', what: 'Calm traffic first, then a sudden burst on one key.', look: 'Whether and when the detector fired, its evidence, and its heuristic confidence. If it does not fire, the numbers it saw.', proves: 'A sudden concentrated burst is noticed; the engine proposes pre-warming and applies nothing.', args: () => ['demo_console/scenarios/pie.mjs', '--id', 'pie-flashcrowd', '--dir', DIR, '--out-dir', join(HUBDIR, 'pie-flashcrowd')] },
  { id: 'pie-shift', group: 'Intelligence', title: 'Workload shift', pie: true, needsCluster: true, style: 'shows', shows: 'the engine noticing a change in the shape of the traffic', est: '50 s', what: 'Spread-out traffic, then traffic dominated by one key.', look: 'The top-1 share, the classification label and any new decision records.', proves: 'The engine notices when the shape of the traffic changes.', args: () => ['demo_console/scenarios/pie.mjs', '--id', 'pie-shift', '--dir', DIR, '--out-dir', join(HUBDIR, 'pie-shift')] },
  { id: 'pie-eviction', group: 'Intelligence', title: 'Eviction what-if (shadow)', pie: true, needsCluster: true, style: 'shows', shows: 'shadow what-if comparisons of replacement policies on a sample', est: '30 s', what: 'A scan mixed with a hot set of 20 keys.', look: 'Shadow-evaluated hit-ratio changes of other replacement policies against the incumbent (1,024-entry shadow, sampled).', proves: 'Alternative cache policies are compared beside serving; real serving is not changed.', args: () => ['demo_console/scenarios/pie.mjs', '--id', 'pie-eviction', '--dir', DIR, '--out-dir', join(HUBDIR, 'pie-eviction')] },
  { id: 'pie-admission', group: 'Intelligence', title: 'Admission: one-hit wonders', pie: true, needsCluster: true, style: 'shows', shows: 'frequency features and admission advice from one-hit-wonder traffic', est: '20 s', what: 'Mostly never-repeated keys plus a small hot set, sent fast.', look: 'Frequency features F18 to F22 with their sample counts, and the admission advice record.', proves: 'The engine can tell that most keys are seen only once, and advises; it needs thousands of keys in one window to say so.', args: () => ['demo_console/scenarios/pie.mjs', '--id', 'pie-admission', '--dir', DIR, '--out-dir', join(HUBDIR, 'pie-admission')] },
  { id: 'dashboards', group: 'Observability', title: 'Live dashboards', needsCluster: true, style: 'shows', shows: 'the Control Center numbers moving with real traffic', est: '25 s', what: 'Steady traffic for 20 seconds while you open the Control Center screens.', look: 'Overview, Traffic, Cache and memory, Freshness, Origin and Cluster move together.', proves: 'The screens show live measured values.', args: EX('dashboards') },
  { id: 'benchmark', group: 'Performance', title: 'Quick benchmark', needsCluster: false, style: 'shows', shows: 'gateway versus straight to the origin (quick, 3 repetitions)', est: '2-4 min', what: 'The same real requests straight to the origin, through one gateway and through three; fast and slow origin.', look: 'Ratios against "direct".', proves: 'The honest cost and benefit, labelled. The full protocol is in the research docs.', args: () => ['benchmark_scenario.mjs', '--exe', exeOf(), '--out-dir', join(HUBDIR, 'benchmark'), '--reps', '3', '--sizes', '1000', '--concs', '100', '--profiles', 'O0,O2', '--objects', '300', '--warmup', '200', '--origin-port', P(19300), '--port-base', P(18330), '--cluster-port-base', P(19701)] },
  { id: 'everything', group: 'All', title: 'Demonstrate everything', client: true, needsCluster: false, style: 'passed', est: '10 min', what: 'The whole storyline above, in order, then the session summary.', look: 'Watch the numbers on the session strip.', proves: 'Every claim, one after another.' },
];
// storyline order = catalogue order
const ORDER_ALL = CATALOGUE.filter((c) => !c.client && !c.native).map((c) => c.id); // the queue skips the tour and the two-window scenario (they need your clicks)

// ------------------------------------------------------------------ jobs and the event feed
const feed = []; // every event, in order: { seq, job, scenario, ... }
const clients = new Set();
let seq = 0;
const jobs = new Map();
let current = null;
let queue = { ids: [], paused: false, running: false };
function push(ev) {
  const e = { seq: ++seq, ...ev };
  feed.push(e); if (feed.length > 20000) feed.splice(0, 5000);
  for (const c of clients) c.write(`id: ${e.seq}\ndata: ${JSON.stringify(e)}\n\n`);
  return e;
}
const note = (job, kind, text, extra = {}) => push({ job: job.id, scenario: job.scenario, t: new Date().toISOString(), kind, text, ...extra });
const FORBIDDEN = /eyJ[A-Za-z0-9_-]{20,}|PHANTOM_JWT_SECRET=|BEGIN [A-Z ]*PRIVATE KEY/;
const scrub = (s) => String(s).replace(/eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]*/g, '[token hidden]');
const jobPublic = (j) => { const { child, summary, ...rest } = j; return rest; };

async function baseline() {
  const st = cluster(); if (!st) return null;
  const os = await originStats(st.originPort);
  return os ? os.requests : null;
}

async function counters() {
  const st = cluster(); if (!st) return null; const tA = tokenFor(st, 'tenantA'); const tot = { local: 0, peer: 0, origin: 0, coalesced: 0, stale: 0 };
  for (let i = 0; i < st.nodes; i++) { if (!alive(st.pids[`node${i + 1}`])) continue; const j = await getJson(st.ports[i], '/api/v1/metrics', tA); const r = !j.error && j.metrics && j.metrics.outcomes && j.metrics.outcomes.responses; if (r) for (const k of Object.keys(tot)) { const x = r[k]; if (x && x.src === 'measured' && typeof x.v === 'number') tot[k] += x.v; } }
  const os = await originStats(st.originPort); return { responses: tot, origin: os ? os.requests : null };
}
async function activeView(st) { const tA = tokenFor(st, 'tenantA'); for (let i = 0; i < st.nodes; i++) { if (!alive(st.pids[`node${i + 1}`])) continue; const j = await getJson(st.ports[i], '/api/v1/metrics', tA); const c = !j.error && j.metrics && j.metrics.cluster; if (c && c.local) { const states = [c.local.state && c.local.state.v, ...c.peers.map((p) => p.state && p.state.v)]; return states.length === st.nodes && states.every((x) => x === 'Active'); } } return false; }
// Make the cluster whole: restart any stopped gateway and wait until every member is Active. Returns the number restarted, or -1 if it did not settle.
async function ensureCluster(job, why) {
  let st = cluster(); if (!st) return -1;
  let restarted = 0;
  for (let n = 1; n <= st.nodes; n++) { if (!alive(st.pids[`node${n}`])) { note(job, 'action', `${why}: node ${n} was stopped, restarting it`, { means: 'A previous scenario (or you) stopped this gateway; every run starts from a whole cluster.' }); launcher(['start-node', '--node', String(n)]); restarted += 1; } }
  st = cluster(); const t0 = Date.now();
  while (Date.now() - t0 < 45000) { if (Array.from({ length: st.nodes }, (_, i) => alive(st.pids[`node${i + 1}`])).every(Boolean) && await activeView(st)) return restarted; await new Promise((r) => setTimeout(r, 400)); }
  return -1;
}
async function prepare(job, sc) {
  if (!sc.needsCluster) { note(job, 'action', 'Preparing: this scenario starts its own gateway and origin on a separate port block (183xx / 193xx), so it cannot disturb the demo cluster', { means: 'It can be run again immediately: it cleans up its own processes.' }); return true; }
  note(job, 'action', 'Preparing: checking the cluster: 3 gateways running and Active?');
  const n = await ensureCluster(job, 'prepare');
  if (n < 0) { note(job, 'observation', 'The cluster did not become fully Active within 45 s.', { means: 'Press Reset demo, then run this again.' }); return false; }
  const c = await counters(); job.before = c;
  note(job, 'observation', `all gateways Active (${n ? n + ' restarted' : 'none needed restarting'}). Run id ${job.id}: every key this run uses is new, so cold-start behaviour is real. Counters are snapshotted now; only differences are reported.`, { means: 'Numbers below are changes during this run, never totals that depend on earlier runs.' });
  return true;
}
async function cleanup(job, sc) {
  if (sc.needsCluster) {
    const n = await ensureCluster(job, 'clean-up');
    const c = await counters();
    if (job.before && c) { const d = Object.fromEntries(Object.keys(c.responses).map((k) => [k, c.responses[k] - job.before.responses[k]])); note(job, 'observation', `gateway responses during this run (after minus before, all nodes): local ${d.local}, peer ${d.peer}, origin ${d.origin}, coalesced ${d.coalesced}, stale ${d.stale}`, { means: 'Measured by the gateways themselves.' }); }
    if (n > 0) note(job, 'observation', `clean-up: restored ${n} stopped gateway(s); cluster whole again`);
    else if (n < 0) note(job, 'observation', 'clean-up: the cluster did not settle; press Reset demo', {});
  } else {
    for (const p of [18300, 18310, 18320, 18330, 18331, 18332, 19300]) { /* the scenario scripts own and free these; nothing to do unless a process survived */ }
  }
}

async function startJob(scenarioId) {
  if (current && current.state === 'running') return { error: 'Another scenario is running. Wait for it or press Stop.' };
  const sc = CATALOGUE.find((s) => s.id === scenarioId);
  if (!sc) return { error: 'unknown scenario' };
  if (sc.client) return { error: 'This one is started from the page.' };
  const st = cluster();
  const job = { id: `${scenarioId}-${Date.now().toString(36)}`, scenario: scenarioId, title: sc.title, state: 'running', startedAt: Date.now(), verdict: null, numbers: null, checks: { pass: 0, fail: 0 }, exit: null, originAtStart: await baseline(), progress: null };
  jobs.set(job.id, job); current = job;
  push({ job: job.id, scenario: scenarioId, t: new Date().toISOString(), kind: 'job-start', text: sc.title });
  if (sc.needsCluster && !st) return finishJob(job, 'blocked', 'The demo cluster has not been started yet. Click "Start the demo cluster" at the top, then run this again.');
  if (sc.native === 'multiuser') { runMultiuser(job, sc); return { started: job.id }; }
  if (!exeOf() || !existsSync(exeOf())) return finishJob(job, 'blocked', 'The gateway program was not found. Start the hub with --exe pointing at the gateway executable.');
  if (!(await prepare(job, sc))) return finishJob(job, 'blocked', 'Not run: the cluster could not be made ready.');
  return spawnScenario(job, sc, st);
}

function spawnScenario(job, sc, st) {
  const [script, ...rest] = sc.args(st);
  const args = [join(SCRIPTS, script), ...rest, ...(rest.includes('--events') ? [] : ['--events'])];
  let child;
  try { child = spawn(process.execPath, args, { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, PHANTOM_JWT_SECRET: '' } }); } catch { return finishJob(job, 'failed', 'That step could not start. Click Run again.'); }
  job.child = child; job.pid = child.pid;
  let buf = '';
  const onData = (d) => { buf += d; let i; while ((i = buf.indexOf('\n')) >= 0) { const line = buf.slice(0, i).replace(/\r$/, ''); buf = buf.slice(i + 1); handleLine(job, sc, line); } };
  child.stdout.on('data', onData); child.stderr.on('data', onData);
  child.on('exit', (code) => { if (buf.trim()) handleLine(job, sc, buf.trim()); onExit(job, sc, code); });
  return { started: job.id };
}

const acks = new Set();
async function runMultiuser(job, sc) {
  acks.clear();
  if (!(await prepare(job, sc))) return finishJob(job, 'blocked', 'Not run: the cluster could not be made ready.');
  note(job, 'action', `Checking that the Control Center is reachable at ${UI_BASE}`);
  let up = false; try { const r = await fetch(UI_BASE, { signal: AbortSignal.timeout(3000) }); up = r.ok; } catch { /* not running */ }
  if (!up) { note(job, 'observation', `The Control Center is not running at ${UI_BASE}.`, { means: 'Start it with the start batch file (or npm run dev in ui), then run this again.' }); return finishJob(job, 'attention', null, 1); }
  note(job, 'observation', 'The Control Center is running.');
  note(job, 'action', 'Window 1 (tenant A on node 1) was opened by your click; click "Open window 2 as Tenant B" (top right of the page) for the second one, because a browser opens only one new window per click. Waiting up to 60 s for each to ask for its token by one-time message (never in the address)', { means: 'If nothing happens, allow pop-ups for this page and press Run again.' });
  const t0 = Date.now(); const seen = new Set();
  while (Date.now() - t0 < 60000 && job.state === 'running') {
    for (const u of acks) if (!seen.has(u)) { seen.add(u); note(job, 'check', `the window for ${u === 'tenantA' ? 'tenant A' : 'tenant B'} received its token by message (the token is not in its address)`, { pass: true, means: 'That window is now signed in as that customer.' }); job.checks.pass += 1; }
    if (seen.has('tenantA') && seen.has('tenantB')) break;
    await new Promise((r) => setTimeout(r, 400));
  }
  const ok = seen.has('tenantA') && seen.has('tenantB');
  if (!ok) { note(job, 'check', 'both windows received their token', { pass: false, means: 'A pop-up may have been blocked, or the Control Center build has no token hand-off.' }); job.checks.fail += 1; }
  else note(job, 'observation', 'Look at the two windows: each shows its own identity and its own live data. Deleting a key in one is visible in the other through the gateway counters.', { means: 'Two people, two identities, the same gateways.' });
  job.numbers = [{ label: 'windows signed in by message', value: seen.size, tag: 'measured' }];
  finishJob(job, ok ? 'shows' : 'attention', null, ok ? 0 : 1);
}

function handleLine(job, sc, line) {
  if (!line) return;
  if (line.startsWith('@EV ')) {
    let e; try { e = JSON.parse(line.slice(4)); } catch { return; }
    if (FORBIDDEN.test(JSON.stringify(e))) return;
    if (e.kind === 'check') { if (e.pass) job.checks.pass += 1; else job.checks.fail += 1; }
    if (e.kind === 'progress') job.progress = { done: e.done, total: e.total, counters: e.counters };
    if (e.kind === 'summary') job.summary = e;
    const text = scrub(e.text || '');
    note(job, e.kind, text, { ...e, text, t: e.t || new Date().toISOString() });
    return;
  }
  const clean = scrub(line);
  push({ job: job.id, scenario: job.scenario, t: new Date().toISOString(), kind: 'raw', text: clean });
  if (sc.id === 'benchmark' && clean.startsWith('[BENCH')) note(job, 'observation', clean.replace(/^\[BENCH [\d:]+\] /, ''));
}

function benchNumbers() {
  try {
    const j = JSON.parse(readFileSync(join(HUBDIR, 'benchmark', 'benchmark.summary.json'), 'utf8'));
    return (j.ratios || []).map((q) => ({ label: `${q.target} vs direct, origin profile ${q.profile}: throughput ratio / p50 ratio / p99 ratio`, value: `${q.throughput_ratio_vs_direct.mean.toFixed(2)} / ${q.p50_ratio_vs_direct.mean.toFixed(2)} / ${q.p99_ratio_vs_direct.mean.toFixed(2)}`, tag: 'derived' }));
  } catch { return null; }
}

function onExit(job, sc, code) {
  if (job.state !== 'running') return;
  job.exit = code;
  let verdict = job.summary && job.summary.verdict ? job.summary.verdict : (code === 0 ? 'passed' : 'attention');
  if (code !== 0 || job.checks.fail > 0) verdict = 'attention';
  else if (sc.style === 'shows' && verdict === 'passed') verdict = 'shows';
  let numbers = (job.summary && job.summary.numbers) || [];
  if (sc.id === 'benchmark') { const bn = benchNumbers(); if (bn) numbers = [...numbers, ...bn]; if (code === 0) verdict = 'shows'; }
  if (verdict === 'attention' && !job.retried && job.state === 'running') {
    job.retried = true; job.checks = { pass: 0, fail: 0 }; job.summary = null;
    note(job, 'observation', 'That step was slow or did not finish as expected. The hub is restoring the cluster and retrying once.', { means: 'One honest retry is allowed; if it fails again the verdict says so and the session summary counts it.' });
    (async () => { try { await cleanup(job, sc); await prepare(job, sc); spawnScenario(job, sc, cluster()); } catch { finishJob(job, 'attention', null, 1); } })();
    return;
  }
  const done = async () => { job.numbers = numbers; if (job.retried) numbers.push({ label: 'first attempt needed a retry', value: 'yes', tag: 'measured' }); try { await cleanup(job, sc); } catch { /* ignore */ } finishJob(job, verdict, null, code); };
  if (job.originAtStart !== null) baseline().then((now) => { if (now !== null) note(job, 'observation', `origin requests counted by the origin process during this scenario: ${now - job.originAtStart}`, { means: 'Measured by the origin itself, not by the gateways.' }); done(); }); else done();
}

function finishJob(job, verdict, message, code) {
  if (job.state !== 'running') return { done: true };
  job.state = verdict === 'blocked' ? 'blocked' : (verdict === 'stopped' ? 'stopped' : 'done');
  job.verdict = verdict; job.endedAt = Date.now(); job.elapsedMs = job.endedAt - job.startedAt; job.exit = code ?? job.exit;
  if (message) note(job, 'observation', message, { means: 'Nothing was run.' });
  if (verdict === 'attention') note(job, 'observation', 'That step did not finish as expected, even after one retry. The hub has restored the cluster. Click Run again; the raw log has the exact lines.');
  push({ job: job.id, scenario: job.scenario, t: new Date().toISOString(), kind: 'job-end', verdict, elapsedMs: job.elapsedMs, checks: job.checks, numbers: job.numbers, originAtStart: job.originAtStart, exit: job.exit, title: job.title });
  if (current === job) current = null;
  if (queue.running) setTimeout(nextInQueue, 300);
  return { finished: verdict };
}

function stopCurrent() {
  queue.ids = []; queue.running = false;
  const j = current; if (!j || j.state !== 'running') return { idle: true };
  note(j, 'action', 'STOP pressed: ending the scenario now', { means: 'The scenario process and any gateways it started are terminated.' });
  try { if (j.pid) spawnSync('taskkill', ['/PID', String(j.pid), '/T', '/F'], { stdio: 'ignore' }); } catch { /* ignore */ }
  const scx = CATALOGUE.find((x) => x.id === j.scenario); if (scx) cleanup(j, scx).catch(() => {});
  finishJob(j, 'stopped', 'Stopped by you. If a node was being killed or restarted, check the cluster bar: use "Restart node" or "Reset demo" if a node is stopped.');
  return { stopped: true };
}

async function nextInQueue() {
  if (!queue.running) return;
  if (queue.paused) { setTimeout(nextInQueue, 500); return; }
  if (current && current.state === 'running') return;
  const id = queue.ids.shift();
  if (!id) { queue.running = false; push({ job: 'queue', t: new Date().toISOString(), kind: 'queue-end', text: 'Demonstrate everything finished' }); return; }
  await startJob(id);
}

// ------------------------------------------------------------------ state for the header and cards
const tokenFor = (st, user) => (st && st.secure && st.tokens && st.tokens[user] ? st.tokens[user].token : null);
function identity(st, user) {
  if (user === 'none') return { user, label: 'No token', note: 'no Authorization header is sent', claims: null };
  const u = st && st.tokens && st.tokens[user];
  if (!u) return { user, label: user, claims: null, note: 'unknown user' };
  const c = decodeClaimsForDisplay(u.token); const now = Math.floor(Date.now() / 1000);
  return { user, label: u.label, description: u.description, claims: c ? { sub: c.sub, tenant_id: c.tenant_id ?? null, roles: c.roles, iss: c.iss, aud: c.aud, exp: c.exp, iat: c.iat } : null, expired: c ? c.exp < now : null, provenance: 'decoded for display (signature not verified here); the expiry verdict is derived from the claims' };
}
let _ossl = null;
function opensslOk() { if (_ossl === null) { const r = spawnSync('openssl', ['version'], { encoding: 'utf8' }); _ossl = !r.error && r.status === 0; } return _ossl; }
let clusterJob = { state: 'idle' }; let launcherLog = [];
async function state() {
  const st = cluster();
  const base = { catalogue: CATALOGUE.map(({ args, ...c }, i) => ({ ...c, n: i + 1 })), opensslOk: opensslOk(), order: ORDER_ALL, job: current ? jobPublic(current) : null, jobs: [...jobs.values()].map((j) => ({ id: j.id, scenario: j.scenario, state: j.state, verdict: j.verdict, numbers: j.numbers, checks: j.checks, elapsedMs: j.elapsedMs, startedAt: j.startedAt, originAtStart: j.originAtStart })), queue: { n: queue.ids.length, running: queue.running, paused: queue.paused }, clusterJob, launcherLog, uiBase: UI_BASE, exeKnown: Boolean(exeOf() && existsSync(exeOf())) };
  if (!st) return { ...base, cluster: null };
  const nodes = []; const tA = tokenFor(st, 'tenantA');
  for (let i = 0; i < st.nodes; i++) {
    const pid = st.pids[`node${i + 1}`]; const n = { n: i + 1, port: st.ports[i], running: alive(pid), responses: null, view: null };
    if (n.running) {
      const j = await getJson(st.ports[i], '/api/v1/metrics', tA);
      if (!j.error) {
        const o = j.metrics && j.metrics.outcomes;
        n.responses = o && o.responses ? Object.fromEntries(Object.entries(o.responses).map(([k, x]) => [k, x && x.src === 'measured' && typeof x.v === 'number' ? x.v : null])) : null;
        const c = j.metrics && j.metrics.cluster;
        if (c && c.local) n.view = Object.fromEntries([[c.local.node_id, 'self'], ...c.peers.map((p) => [p.node_id, `${p.state && p.state.v}/${p.reachability && p.reachability.v}`])]);
      } else n.error = j.error;
    }
    nodes.push(n);
  }
  const os = await originStats(st.originPort);
  return { ...base, cluster: { secure: st.secure, nodes, originPort: st.originPort, originKind: st.originKind }, origin: os ? { requests: os.requests, bytes: os.bytes } : null, users: ['tenantA', 'tenantB', 'readonly', 'expired', 'wrongAudience', 'none'].map((u) => identity(st, u)) };
}
let manifestCache = null;
async function manifest() { const st = cluster(); if (!manifestCache && st) { const m = await originManifest(st.originPort); if (!m.error) manifestCache = m; } return manifestCache; }

function clusterUp(reset) {
  const exe = exeOf(); const st = cluster();
  if (!exe) return { error: 'No gateway executable known. Start the hub with --exe <gateway>.' };
  if (!reset && st && Object.values(st.pids).some((p) => alive(p))) return { out: 'A cluster is already running.' };
  clusterJob = { state: 'running', startedAt: Date.now() }; launcherLog = [];
  if (reset) launcher(['down']);
  manifestCache = null;
  const args = [join(SCRIPTS, 'cluster_launcher.mjs'), 'up', '--secure', '--exe', exe, '--dir', DIR, '--ui-base', UI_BASE,
    '--http-port-base', String(opt('http-port-base', st ? st.httpPortBase : 18185)), '--cluster-port-base', String(opt('cluster-port-base', st ? st.clusterPortBase : 19501)), '--origin-port', String(opt('origin-port', st ? st.originPort : 19099))];
  const p = spawn(process.execPath, args, { windowsHide: true });
  const take = (d) => { launcherLog.push(...String(d).split('\n').filter(Boolean).map((l) => l.replace(/^\[CLUSTER-LAUNCHER\] /, '')).filter((l) => !/token:/.test(l) && !FORBIDDEN.test(l))); launcherLog = launcherLog.slice(-10); };
  p.stdout.on('data', take); p.stderr.on('data', take);
  p.on('exit', (code) => { clusterJob = { state: code === 0 ? 'idle' : 'error', exit: code }; manifestCache = null; });
  return { started: true };
}

// ------------------------------------------------------------------ http
function send(res, code, body, type = 'application/json', headers = {}) {
  res.writeHead(code, { 'content-type': type, 'cache-control': 'no-store', 'x-content-type-options': 'nosniff', ...headers });
  res.end(typeof body === 'string' || Buffer.isBuffer(body) ? body : JSON.stringify(body));
}
const readBody = (req) => new Promise((ok) => { let s = ''; req.on('data', (d) => { s += d; }); req.on('end', () => { try { ok(JSON.parse(s || '{}')); } catch { ok({}); } }); });
const hostOk = (req) => { const h = String(req.headers.host || ''); return h === `127.0.0.1:${PORT}` || h === `localhost:${PORT}`; };

const server = createServer(async (req, res) => {
  try {
    if (!hostOk(req)) return send(res, 403, { error: 'loopback only' });
    const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
    const st = cluster();
    if (req.method === 'GET' && (url.pathname === '/' || url.pathname === '/index.html')) return send(res, 200, readFileSync(join(HERE, 'index.html')), 'text/html; charset=utf-8');
    if (url.pathname === '/api/state') return send(res, 200, await state());
    if (url.pathname === '/api/stream') { // Server-Sent Events: replay from Last-Event-ID, then live
      res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store', connection: 'keep-alive' });
      const from = Number(req.headers['last-event-id'] || url.searchParams.get('from') || 0);
      res.write(': hub\n\n');
      for (const e of feed) if (e.seq > from) res.write(`id: ${e.seq}\ndata: ${JSON.stringify(e)}\n\n`);
      clients.add(res); req.on('close', () => clients.delete(res)); return;
    }
    if (url.pathname === '/api/manifest') { const m = await manifest(); const out = {}; if (m) for (const [p, v] of Object.entries(m)) out[p] = { bytes: v.bytes, kind: v.kind, label: v.label, sha256: v.sha256, contentType: v.contentType }; return send(res, 200, out); }
    if (url.pathname.startsWith('/proxy/')) {
      const [, , nodeS, ...rest] = url.pathname.split('/'); const node = Number(nodeS); const path = '/' + rest.join('/'); const user = url.searchParams.get('as') || 'none';
      if (!st || !(node >= 1 && node <= st.nodes)) return send(res, 400, { error: 'bad node' });
      const r = await httpRequest({ port: st.ports[node - 1], path: cachePath(path, url.searchParams.get('nonce')), token: tokenFor(st, user), keepBody: true });
      const m = await manifest(); const exp = m && m[path];
      const h = { 'x-demo-ms': r.ms ? r.ms.toFixed(1) : '', 'x-demo-expected-sha256': exp ? exp.sha256 : '', 'x-demo-status': String(r.status || 0), 'x-demo-error': r.error || '' };
      for (const k of ['x-cache-status', 'x-cache-source', 'age', 'etag', 'x-auth-error']) if (r.headers && r.headers[k]) h[k] = r.headers[k];
      h['access-control-expose-headers'] = Object.keys(h).join(',') + ',content-type';
      return send(res, r.error ? 502 : r.status, r.body || '', (r.headers && r.headers['content-type']) || 'text/plain', h);
    }
    if (req.method === 'POST' && url.pathname === '/api/delete') {
      const b = await readBody(req); if (!st) return send(res, 400, { error: 'no cluster' });
      const r = await httpRequest({ port: st.ports[(b.node || 1) - 1], method: 'DELETE', path: cachePath(b.path, b.nonce || null), token: tokenFor(st, b.as || 'none') });
      return send(res, 200, { status: r.status, error: r.error || null, ms: +(r.ms || 0).toFixed(1), xAuthError: (r.headers && r.headers['x-auth-error']) || null });
    }
    if (req.method === 'GET' && url.pathname === '/api/token') { // Control Center hand-off only; same-origin fetches only
      const sfs = req.headers['sec-fetch-site']; const user = url.searchParams.get('user');
      if (req.headers['x-hub'] !== '1' || (sfs && sfs !== 'same-origin')) return send(res, 403, { error: 'forbidden' });
      const t = tokenFor(st, user); if (!t) return send(res, 404, { error: 'no token for that user' });
      return send(res, 200, { token: t });
    }
    if (req.method === 'POST' && url.pathname === '/api/ack') { const b = await readBody(req); if (['tenantA', 'tenantB', 'readonly'].includes(b.user)) acks.add(b.user); return send(res, 200, { ok: true }); }
    if (req.method === 'POST' && url.pathname === '/api/run') { const b = await readBody(req); return send(res, 200, await startJob(String(b.id))); }
    if (req.method === 'POST' && url.pathname === '/api/run-all') { const bb = await readBody(req); queue = { ids: Array.isArray(bb.ids) && bb.ids.length ? bb.ids.filter((x) => ORDER_ALL.includes(x)) : ORDER_ALL.slice(), paused: false, running: true }; setTimeout(nextInQueue, 50); return send(res, 200, { started: true }); }
    if (req.method === 'POST' && url.pathname === '/api/pause') { const b = await readBody(req); queue.paused = Boolean(b.paused); return send(res, 200, { paused: queue.paused }); }
    if (req.method === 'POST' && url.pathname === '/api/stop') return send(res, 200, stopCurrent());
    if (req.method === 'POST' && url.pathname === '/api/node') { const b = await readBody(req); const r = launcher([b.action === 'start' ? 'start-node' : 'stop-node', '--node', String(b.node)]); return send(res, 200, { out: scrub((r.stdout || r.stderr || '').trim()) }); }
    if (req.method === 'POST' && url.pathname === '/api/cluster') {
      const b = await readBody(req);
      if (b.action === 'down') { stopCurrent(); const r = launcher(['down']); manifestCache = null; return send(res, 200, { out: r.stdout.trim() }); }
      if (b.action === 'reset') stopCurrent();
      return send(res, 200, clusterUp(b.action === 'reset'));
    }
    send(res, 404, { error: 'not found' });
  } catch { send(res, 500, { error: 'internal error' }); }
});
server.listen(PORT, '127.0.0.1', () => console.log(`Demo Hub on http://127.0.0.1:${PORT}/ (cluster dir ${DIR}; loopback only)`));
