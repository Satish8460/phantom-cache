#!/usr/bin/env node
// scripts/cluster_launcher.mjs
//
// Reproducible launcher for a local PhantomCache cluster of N gateways with mutual TLS and a counting origin, for demos,
// the Control Center Cluster screen and the live check scripts. It is a development tool: it is not part of the gateway,
// it does not change the product, and the certificates it creates are TEST certificates (RSA 2048, valid 30 days, private
// keys written next to them). Do not use them for anything else.
//
// Requirements: Node 18+ (24 for the demo scripts), the `openssl` command on the PATH, a built gateway executable.
//
// Commands (all state lives in --dir; nothing is written anywhere else):
//   up       --exe <gateway> --dir <dir> [--nodes 3] [--cluster-id demo] [--origin-port 19099]
//            [--http-port-base 18185] [--cluster-port-base 19501] [--cors-origin http://localhost:3000[,more]]
//            [--secure] [--ui-base http://localhost:3000] [--origin-kind text|content] [--origin-delay-ms 25]
//            Creates the certificates once (kept on later runs), starts the origin and the gateways in the background,
//            waits until every member reports Active in node 1's metrics, writes <dir>/cluster.json, prints the URLs.
//            --secure: generates a random JWT secret (48 bytes, kept in <dir>/jwt_secret.txt, never printed), starts every
//            gateway with PHANTOM_JWT_SECRET / PHANTOM_REQUIRE_AUTH=true, mints the demo tokens (HS256, Node crypto) into
//            <dir>/tokens.json, and prints per demo user and gateway the Control Center URL plus where the token is.
//            The token itself is never printed and never put in a URL.
//            --origin-kind: `text` (the original tiny counting origin) or `content` (scripts/lib/content_origin.mjs: real PNG,
//            JSON, text and HTML objects with a sha256 manifest). Default: content with --secure, text otherwise.
//   tokens   --dir <dir>          Mints fresh demo tokens with the stored secret (use after they expired).
//   status   --dir <dir>          Prints each node's process state and its routing state / reachability as seen by node 1.
//   stop-node --dir <dir> --node N    Kills one gateway (to show a dead peer).
//   start-node --dir <dir> --node N   Starts it again with the same settings.
//   down     --dir <dir>          Stops every process it started.
//   origin   --port N             (internal) runs the counting text origin in the foreground.
//
// The text origin answers any path with `text/plain` and an ETag; a path containing `/slow/` waits 400 ms; a path containing
// `/swr/` has max-age=1 with stale-while-revalidate=60; everything else is cacheable for 300 s. `/__stats` returns the
// number of requests it has served.
//
// Example:
//   node scripts/cluster_launcher.mjs up --exe target/release/phantom-cache-demo.exe --dir <work-dir>
//   (open the Control Center with  http://localhost:3000/?gateway=http://127.0.0.1:18185#/cluster )
//   node scripts/cluster_launcher.mjs up --secure --exe <gateway> --dir <work-dir>      (secure, real content, demo tokens)
//   node scripts/cluster_launcher.mjs stop-node --dir <work-dir> --node 3
//   node scripts/cluster_launcher.mjs down --dir <work-dir>

import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, openSync, readFileSync, writeFileSync, existsSync, closeSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { newSecret, mintDemoTokens } from './lib/jwt_mint.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const argv = process.argv.slice(2);
const cmd = argv[0];
const opt = (name, def) => {
  const i = argv.indexOf(`--${name}`);
  return i !== -1 && argv[i + 1] !== undefined ? argv[i + 1] : def;
};
const flag = (name) => argv.includes(`--${name}`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const die = (m) => { console.error(`[CLUSTER-LAUNCHER] ${m}`); process.exit(1); };
const log = (m) => console.log(`[CLUSTER-LAUNCHER] ${m}`);

const ISSUER = 'phantom-cache-auth'; // the gateway defaults (src/config.rs:110-113); set explicitly below as well
const AUDIENCE = 'phantom-cache-api';

function usage() {
  console.error('usage: node scripts/cluster_launcher.mjs <up|tokens|status|stop-node|start-node|down|origin> [options]  (see the header of this file)');
  process.exit(1);
}

// ---------------------------------------------------------------- origin (foreground, internal)
if (cmd === 'origin') {
  const port = Number(opt('port', 19099));
  if (!Number.isFinite(port) || port <= 0) usage();
  let count = 0;
  createServer((req, res) => {
    if (req.url === '/__stats') { res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify({ requests: count })); }
    count += 1;
    const n = count;
    setTimeout(() => {
      res.setHeader('content-type', 'text/plain; charset=utf-8');
      res.setHeader('etag', `"o${n}"`);
      res.setHeader('cache-control', req.url.includes('/swr/') ? 'public, max-age=1, stale-while-revalidate=60' : 'public, max-age=300');
      res.end(`origin:${req.url}:${n}`);
    }, req.url.includes('/slow/') ? 400 : 5);
  }).listen(port, '127.0.0.1', () => console.log(`origin listening on ${port}`));
} else {
  main().catch((e) => die(e && e.stack ? e.stack : String(e)));
}

// ---------------------------------------------------------------- helpers
function sh(file, args, opts = {}) {
  const r = spawnSync(file, args, { encoding: 'utf8', ...opts });
  if (r.error) die(`cannot run ${file}: ${r.error.message}`);
  if (r.status !== 0) die(`${file} ${args.join(' ')} failed:\n${r.stderr || r.stdout}`);
  return r.stdout;
}

function pidAlive(pid) {
  if (!pid) return false;
  try { process.kill(pid, 0); return true; } catch { return false; }
}

function loadState(dir) {
  const f = join(dir, 'cluster.json');
  if (!existsSync(f)) die(`no cluster.json in ${dir}; run "up" first`);
  return JSON.parse(readFileSync(f, 'utf8'));
}
function saveState(dir, st) { writeFileSync(join(dir, 'cluster.json'), JSON.stringify(st, null, 2)); }

function makeCerts(dir, nodes, clusterId) {
  const certs = join(dir, 'certs');
  if (existsSync(join(certs, 'members.json'))) return certs; // keep an existing set so fingerprints stay valid
  mkdirSync(certs, { recursive: true });
  // members.json (with the certificate fingerprints) is written by writeMembers() once the certificates exist.
  sh('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', join(certs, 'ca.key'), '-out', join(certs, 'ca.crt'), '-days', '30', '-subj', `/CN=PhantomCache test CA ${clusterId}`]);
  for (let n = 1; n <= nodes; n++) {
    const san = `node-${n}.${clusterId}.phantom.internal`;
    const cnf = join(certs, `n${n}.cnf`);
    writeFileSync(cnf, `[req]\ndistinguished_name=dn\nprompt=no\nreq_extensions=v3\n[dn]\nCN=${san}\n[v3]\nsubjectAltName=DNS:${san}\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth,clientAuth\nbasicConstraints=CA:FALSE\n`);
    sh('openssl', ['req', '-newkey', 'rsa:2048', '-nodes', '-keyout', join(certs, `n${n}.key`), '-out', join(certs, `n${n}.csr`), '-config', cnf]);
    sh('openssl', ['x509', '-req', '-in', join(certs, `n${n}.csr`), '-CA', join(certs, 'ca.crt'), '-CAkey', join(certs, 'ca.key'), '-CAcreateserial', '-out', join(certs, `n${n}.crt`), '-days', '30', '-extfile', cnf, '-extensions', 'v3']);
    sh('openssl', ['pkcs8', '-topk8', '-nocrypt', '-in', join(certs, `n${n}.key`), '-out', join(certs, `n${n}.p8.key`)]);
  }
  return certs;
}

function writeMembers(certs, st) {
  const members = {};
  const addresses = {};
  for (let n = 1; n <= st.nodes; n++) {
    const der = spawnSync('openssl', ['x509', '-in', join(certs, `n${n}.crt`), '-outform', 'DER']);
    const fp = spawnSync('openssl', ['dgst', '-sha256', '-binary'], { input: der.stdout });
    members[String(n)] = { node_id: n, fingerprints: [fp.stdout.toString('hex')], active: true };
    addresses[String(n)] = `127.0.0.1:${st.clusterPortBase + n - 1}`;
  }
  writeFileSync(join(certs, 'members.json'), JSON.stringify({ cluster_id: st.clusterId, version: 1, members, addresses }, null, 1));
}

function spawnDetached(file, args, env, logBase) {
  const out = openSync(`${logBase}.out.log`, 'a');
  const err = openSync(`${logBase}.err.log`, 'a');
  const child = spawn(file, args, { env, stdio: ['ignore', out, err], detached: true, windowsHide: true });
  child.unref();
  closeSync(out);
  closeSync(err);
  return child.pid;
}

function readSecret(st) {
  return readFileSync(join(st.dir, 'jwt_secret.txt'), 'utf8').trim();
}

function gatewayEnv(st, n) {
  const certs = join(st.dir, 'certs');
  const peers = Array.from({ length: st.nodes }, (_, i) => `${i + 1}=127.0.0.1:${st.clusterPortBase + i}`).join(',');
  const env = {
    ...process.env,
    ORIGIN_URL: `http://127.0.0.1:${st.originPort}`,
    PHANTOM_CLUSTER_ID: st.clusterId,
    PHANTOM_CLUSTER_MEMBERS_FILE: join(certs, 'members.json'),
    PHANTOM_CLUSTER_CA: join(certs, 'ca.crt'),
    PHANTOM_CLUSTER_PEER_ADDRS: peers,
    PHANTOM_BIND_ADDR: `127.0.0.1:${st.httpPortBase + n - 1}`,
    PHANTOM_NODE_ID: String(n),
    PHANTOM_NODE_CERT: join(certs, `n${n}.crt`),
    PHANTOM_NODE_KEY: join(certs, `n${n}.p8.key`),
    PHANTOM_CLUSTER_LISTEN_ADDR: `127.0.0.1:${st.clusterPortBase + n - 1}`,
  };
  if (st.corsOrigin) env.PHANTOM_CORS_ORIGINS = st.corsOrigin;
  if (st.secure) {
    env.PHANTOM_JWT_SECRET = readSecret(st); // passed through the environment only; never printed or logged
    env.PHANTOM_JWT_ISSUER = ISSUER;
    env.PHANTOM_JWT_AUDIENCE = AUDIENCE;
    env.PHANTOM_REQUIRE_AUTH = 'true';
  }
  if (st.intelligence) env.PHANTOM_INTELLIGENCE = st.intelligence; // 'advisory' = observe and advise, NOTHING is ever applied
  return env;
}

function startGateway(st, n) {
  mkdirSync(join(st.dir, 'logs'), { recursive: true });
  const pid = spawnDetached(st.exe, [], gatewayEnv(st, n), join(st.dir, 'logs', `node${n}`));
  st.pids[`node${n}`] = pid;
  return pid;
}

function mintTokens(st) {
  const users = mintDemoTokens({ secret: readSecret(st), issuer: ISSUER, audience: AUDIENCE });
  writeFileSync(join(st.dir, 'tokens.json'), JSON.stringify({ note: 'DEMO tokens, valid for 24 h (except the two negative ones). Secret material: keep this file inside the work directory.', issuer: ISSUER, audience: AUDIENCE, users }, null, 2));
  return users;
}

function adminHeaders(st) {
  if (!st.secure) return {};
  const t = JSON.parse(readFileSync(join(st.dir, 'tokens.json'), 'utf8')).users.tenantA.token; // telemetry:read
  return { authorization: `Bearer ${t}` };
}

async function view(st) {
  const url = `http://127.0.0.1:${st.httpPortBase}/api/v1/metrics`;
  const j = await (await fetch(url, { signal: AbortSignal.timeout(3000), headers: adminHeaders(st) })).json();
  const c = j && j.metrics && j.metrics.cluster;
  if (!c || !c.local || !c.peers) throw new Error('metrics.cluster missing');
  const nodes = { [c.local.node_id]: { state: c.local.state && c.local.state.v, reachability: 'self' } };
  for (const p of c.peers) nodes[p.node_id] = { state: p.state && p.state.v, reachability: p.reachability && p.reachability.v };
  return nodes;
}

function printUrls(st) {
  const ui = st.uiBase || 'http://localhost:3000';
  const urls = {};
  log(`Control Center (UI base ${ui}); the page asks for a token itself, tokens are never put in a URL:`);
  const users = st.secure ? [['tenantA', 'Tenant A (alice)'], ['tenantB', 'Tenant B (bob)'], ['readonly', 'Read-only (carol)']] : [['none', 'no token needed (plain cluster)']];
  for (const [key, label] of users) {
    for (let n = 1; n <= st.nodes; n++) {
      const u = `${ui}/?gateway=http://127.0.0.1:${st.httpPortBase + n - 1}#/cluster`;
      urls[`${key}@node${n}`] = u;
      log(`  ${label.padEnd(20)} node ${n}: ${u}${st.secure ? `   token: tokens.json -> users.${key}` : ''}`);
    }
  }
  if (st.secure) log(`  negative-test tokens: users.expired and users.wrongAudience in ${join(st.dir, 'tokens.json')}`);
  writeFileSync(join(st.dir, 'demo_urls.json'), JSON.stringify(urls, null, 2));
}

async function main() {
  if (!['up', 'tokens', 'status', 'stop-node', 'start-node', 'down'].includes(cmd)) usage();
  const dir = resolve(opt('dir', ''));
  if (!opt('dir')) usage();

  if (cmd === 'up') {
    const exe = opt('exe');
    if (!exe || !existsSync(exe)) die('--exe must point to a built gateway executable');
    const nodes = Number(opt('nodes', 3));
    if (!Number.isInteger(nodes) || nodes < 2 || nodes > 9) die('--nodes must be an integer from 2 to 9');
    const secure = flag('secure');
    const originKind = opt('origin-kind', secure ? 'content' : 'text');
    if (!['text', 'content'].includes(originKind)) die('--origin-kind must be text or content');
    const uiBase = opt('ui-base', 'http://localhost:3000');
    const st = {
      dir, exe: resolve(exe), nodes, clusterId: opt('cluster-id', 'demo'),
      originPort: Number(opt('origin-port', 19099)), httpPortBase: Number(opt('http-port-base', 18185)),
      clusterPortBase: Number(opt('cluster-port-base', 19501)), corsOrigin: opt('cors-origin', secure ? uiBase : null),
      secure, originKind, uiBase, pids: {}, intelligence: opt('intelligence', secure ? 'advisory' : null),
    };
    if (existsSync(join(dir, 'cluster.json'))) {
      const old = loadState(dir);
      if (Object.values(old.pids).some(pidAlive)) die(`a cluster from ${dir} is still running; run "down" first`);
    }
    mkdirSync(dir, { recursive: true });
    if (secure) {
      if (!existsSync(join(dir, 'jwt_secret.txt'))) writeFileSync(join(dir, 'jwt_secret.txt'), newSecret(48));
      mintTokens(st);
    }
    const certs = makeCerts(dir, nodes, st.clusterId);
    writeMembers(certs, st);
    if (originKind === 'content') {
      st.pids.origin = spawnDetached(process.execPath, [join(HERE, 'lib', 'content_origin.mjs'), '--port', String(st.originPort), '--delay', opt('origin-delay-ms', '25')], process.env, join(dir, 'origin'));
    } else {
      st.pids.origin = spawnDetached(process.execPath, [fileURLToPath(import.meta.url), 'origin', '--port', String(st.originPort)], process.env, join(dir, 'origin'));
    }
    mkdirSync(join(dir, 'logs'), { recursive: true });
    for (let n = 1; n <= nodes; n++) startGateway(st, n);
    saveState(dir, st);
    const deadline = Date.now() + 30000;
    let last;
    while (Date.now() < deadline) {
      try { last = await view(st); if (Object.keys(last).length === nodes && Object.values(last).every((v) => v.state === 'Active')) break; } catch { /* not up yet */ }
      await sleep(300);
    }
    if (!last || !Object.values(last).every((v) => v.state === 'Active') || Object.keys(last).length !== nodes) {
      die(`the cluster did not become Active within 30 s (last view: ${JSON.stringify(last)}); logs are in ${join(dir, 'logs')}`);
    }
    log(`cluster up: ${nodes} gateways${secure ? ' (SECURE: JWT required on data and telemetry routes)' : ''}, ${originKind} origin on ${st.originPort}; every member Active in node 1's view`);
    for (let n = 1; n <= nodes; n++) log(`  node ${n}: http://127.0.0.1:${st.httpPortBase + n - 1}  (cluster listener ${st.clusterPortBase + n - 1}, pid ${st.pids[`node${n}`]})`);
    printUrls(st);
    log(`start the gateways with --cors-origin <the UI origin> when the UI is served from another origin (done automatically with --secure: ${st.corsOrigin || 'none'})`);
    log(`state: ${join(dir, 'cluster.json')}   logs: ${join(dir, 'logs')}${secure ? `   tokens: ${join(dir, 'tokens.json')}` : ''}   stop with: down --dir ${dir}`);
    return;
  }

  const st = loadState(dir);
  if (cmd === 'tokens') {
    if (!st.secure) die('this cluster was not started with --secure');
    const u = mintTokens(st);
    log(`minted ${Object.keys(u).length} demo tokens into ${join(dir, 'tokens.json')} (values not printed)`);
    return;
  }
  if (cmd === 'status') {
    for (const [k, pid] of Object.entries(st.pids)) log(`${k}: pid ${pid} ${pidAlive(pid) ? 'running' : 'NOT running'}`);
    try { log(`as seen by node 1: ${JSON.stringify(await view(st))}`); } catch (e) { log(`node 1 view unavailable: ${e.message}`); }
    return;
  }
  if (cmd === 'stop-node' || cmd === 'start-node') {
    const n = Number(opt('node'));
    if (!Number.isInteger(n) || n < 1 || n > st.nodes) die('--node must be a member number');
    if (cmd === 'stop-node') {
      try { process.kill(st.pids[`node${n}`]); } catch { /* already gone */ }
      log(`node ${n} stopped`);
    } else {
      if (pidAlive(st.pids[`node${n}`])) die(`node ${n} is already running`);
      log(`node ${n} started, pid ${startGateway(st, n)}`);
      saveState(dir, st);
    }
    return;
  }
  if (cmd === 'down') {
    for (const [k, pid] of Object.entries(st.pids)) {
      try { process.kill(pid); log(`stopped ${k} (pid ${pid})`); } catch { log(`${k} (pid ${pid}) was not running`); }
    }
    st.pids = {};
    saveState(dir, st);
  }
}
