#!/usr/bin/env node
// scripts/security_scenario.mjs
//
// End-to-end security story against a LIVE SECURE launcher cluster (`cluster_launcher.mjs up --secure ...`).
// Each check is PASS or FAIL as observed; a documented behaviour that does not hold is reported as a FINDING, not hidden.
//
//   node scripts/security_scenario.mjs --dir <cluster dir> --exe <gateway exe> [--out <result.json>] [--evidence <dir>] [--no-browser]
//
// Checks (all HTTP checks use the real PNG origin, so isolation is shown with real image bytes):
//   ingress   no token 401, expired 401, wrong audience 401, bad signature 401, garbage 401, missing tenant 403,
//             read-only DELETE 403, health is public, metrics need a token
//   tenants   A warms a key, B gets MISS for the same path (origin asked again), A DELETE purges only A, x-tenant-id ignored
//   target    CRLF-injected path 400, dot-segment path 400, origin untouched
//   cluster   mTLS on the cluster listener: a member certificate is answered (positive control); no certificate, a
//             certificate signed by the cluster CA but not in the membership, a certificate with a member's name but another key,
//             and a certificate from a foreign CA get NO cluster response
//   startup   the gateway refuses to start with PHANTOM_REQUIRE_AUTH=true and no secret (exit code 2)
// Exit code 0 only when every check passed.

import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import tls from 'node:tls';
import { loadCluster, httpRequest, cachePath, originStats, originManifest, sleep } from './lib/traffic.mjs';
import { ev, pickHeaders } from './lib/events.mjs';
import { mintHS256 } from './lib/jwt_mint.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const argv = process.argv.slice(2);
const opt = (n, d) => { const i = argv.indexOf(`--${n}`); return i !== -1 && argv[i + 1] !== undefined ? argv[i + 1] : d; };
const DIR = opt('dir'); const EXE = opt('exe');
if (!DIR || !EXE) { console.error('usage: node scripts/security_scenario.mjs --dir <cluster dir> --exe <gateway exe> [--out file] [--evidence dir] [--no-browser]'); process.exit(1); }
const st = loadCluster(DIR);
if (!st.secure) { console.error('this cluster was not started with --secure'); process.exit(1); }
const OUT = opt('out', join(DIR, 'security_result.json'));
const EVID = opt('evidence', null);
const RUN = Date.now().toString(36);
const U = st.tokens;
const P = st.ports;
const secret = readFileSync(join(DIR, 'jwt_secret.txt'), 'utf8').trim(); // used only to mint negative tokens; never printed

const items = [];
const MEANS = [
  [/no token/, 'Without a token the gateway must refuse: it cannot know who is asking.'],
  [/expired/, 'A token past its expiry date is useless even if the signature is genuine.'],
  [/audience/, 'A token issued for another service must not open this one.'],
  [/another secret|forged/, 'A token not signed with the gateway secret is a forgery.'],
  [/read-only token DELETE/, 'Reading is allowed, deleting is not: roles are enforced.'],
  [/ISOLATION/, 'Two customers asking for the same address never share cached data.'],
  [/DELETE did NOT purge B/, 'One customer purging a key cannot empty another customer\'s cache.'],
  [/CRLF|NUL|space|dot-segment|reached the origin/, 'Malformed addresses are rejected before they can reach the origin.'],
  [/cluster mTLS/, 'Only gateways with a certificate the cluster knows can talk to each other.'],
  [/startup/, 'A gateway that is told to require auth but has no secret refuses to start (fail closed).'],
];
const whoIs = (tok) => { const e = Object.values(U).find((v) => v.token === tok); return e ? e.label : 'no token'; };
const add = (group, id, ok, detail, extra = {}) => { ev.check(ok, `${group}: ${id}`, { detail, means: (MEANS.find(([re]) => re.test(id)) || [])[1] }); items.push({ group, id, status: ok ? 'PASS' : 'FAIL', detail, ...extra }); console.log(`[${ok ? 'PASS' : 'FAIL'}] ${group}: ${id}${detail ? ' :: ' + detail : ''}`); return ok; };
const get = async (port, path, token, headers) => {
  const who = whoIs(token); ev.action(`sent GET ${path.replace(/\?r=.*/, '')} as ${who} to node ${P.indexOf(port) + 1}`, { request: { method: 'GET', path, node: P.indexOf(port) + 1, as: who } });
  const r = await httpRequest({ port, path, token, headers });
  ev.observe(`-> ${r.error ? 'error ' + r.error : r.status + (r.headers['x-cache-status'] ? ' ' + r.headers['x-cache-status'] + ' source ' + r.headers['x-cache-source'] : '') + (r.headers['x-auth-error'] ? ' (' + r.headers['x-auth-error'] + ')' : '')}, ${r.ms.toFixed(0)} ms`, { response: { status: r.status || 0, ms: +r.ms.toFixed(1), bytes: r.bytes, headers: pickHeaders(r.headers) } });
  return r;
};
const del = async (port, path, token) => {
  const who = whoIs(token); ev.action(`sent DELETE ${path.replace(/\?r=.*/, '')} as ${who} to node ${P.indexOf(port) + 1}`, { request: { method: 'DELETE', path, node: P.indexOf(port) + 1, as: who } });
  const r = await httpRequest({ port, method: 'DELETE', path, token });
  ev.observe(`-> DELETE answered ${r.error || r.status}`, { response: { status: r.status || 0, ms: +(r.ms || 0).toFixed(1), headers: pickHeaders(r.headers) } });
  return r;
};
const h = (r, n) => r.headers && r.headers[n];

const PART = opt('part', 'all'); // auth | roles | isolation | targetsafety | mtls | failclosed | all
async function ingress() {
  const path = (n) => cachePath(`/msg/${String(n).padStart(3, '0')}.json`, `sec-${RUN}`);
  let r;
  if (PART !== 'roles') {
  r = await get(P[0], path(1), null);
  add('ingress', 'no token -> 401 with WWW-Authenticate: Bearer', r.status === 401 && /bearer/i.test(h(r, 'www-authenticate') || ''), `status ${r.status}, www-authenticate "${h(r, 'www-authenticate')}", x-auth-error "${h(r, 'x-auth-error')}"`);
  r = await get(P[0], path(2), U.expired.token);
  add('ingress', 'expired token -> 401 (X-Auth-Error expected: Expired)', r.status === 401, `status ${r.status}, x-auth-error "${h(r, 'x-auth-error')}"`);
  r = await get(P[0], path(3), U.wrongAudience.token);
  add('ingress', 'wrong-audience token -> 401', r.status === 401, `status ${r.status}, x-auth-error "${h(r, 'x-auth-error')}"`);
  const forged = mintHS256({ ...U.tenantA.claims }, 'this-is-not-the-gateway-secret-0123456789');
  r = await get(P[0], path(4), forged);
  add('ingress', 'token signed with another secret -> 401 (generic InvalidToken)', r.status === 401, `status ${r.status}, x-auth-error "${h(r, 'x-auth-error')}"`);
  r = await get(P[0], path(5), 'not.a.jwt');
  add('ingress', 'garbage token -> 401', r.status === 401, `status ${r.status}`);
  const noTenant = mintHS256({ sub: 'demo-notenant', iss: U.tenantA.claims.iss, aud: U.tenantA.claims.aud, exp: Math.floor(Date.now() / 1000) + 600, roles: ['cache:read'] }, secret);
  r = await get(P[0], path(6), noTenant);
  add('ingress', 'valid token without tenant_id -> 403 on cache routes (MissingTenant)', r.status === 403, `status ${r.status}, x-auth-error "${h(r, 'x-auth-error')}"`);
  }
  if (PART !== 'auth') {
  r = await get(P[0], path(7), U.readonly.token);
  add('ingress', 'read-only token can GET (positive control)', r.status === 200, `status ${r.status}, ${h(r, 'x-cache-status')}/${h(r, 'x-cache-source')}`);
  r = await del(P[0], path(7), U.readonly.token);
  add('ingress', 'read-only token DELETE -> 403', r.status === 403, `status ${r.status}`);
  r = await get(P[0], path(7), U.readonly.token);
  add('ingress', 'the refused DELETE purged nothing (entry still a HIT)', h(r, 'x-cache-status') === 'HIT', `${h(r, 'x-cache-status')}/${h(r, 'x-cache-source')}`);
  r = await get(P[0], '/health', null);
  add('ingress', '/health is public by design (documented)', r.status === 200, `status ${r.status}`);
  r = await get(P[0], '/api/v1/metrics', null);
  add('ingress', 'GET /api/v1/metrics without token -> 401', r.status === 401, `status ${r.status}`);
  r = await get(P[0], '/api/v1/intelligence/status', null);
  add('ingress', 'GET /api/v1/intelligence/status without token -> 401', r.status === 401, `status ${r.status}`);
  r = await get(P[0], '/api/v1/metrics', U.tenantA.token);
  add('ingress', 'GET /api/v1/metrics with telemetry:read -> 200 (positive control)', r.status === 200, `status ${r.status}`);
  }
}

async function tenants() {
  const manifest = await originManifest(st.originPort);
  const IMG = '/img/tile-15.png';
  const nonce = `sec-iso-${RUN}`;
  const sha = manifest[IMG].sha256;
  const o0 = (await originStats(st.originPort)).requests;
  const a1 = await get(P[0], cachePath(IMG, nonce), U.tenantA.token);
  const a2 = await get(P[0], cachePath(IMG, nonce), U.tenantA.token);
  const o1 = (await originStats(st.originPort)).requests;
  const b1 = await get(P[0], cachePath(IMG, nonce), U.tenantB.token);
  const o2 = (await originStats(st.originPort)).requests;
  const b2 = await get(P[0], cachePath(IMG, nonce), U.tenantB.token);
  add('tenants', 'tenant A: first GET MISS, second GET HIT (real PNG, sha256 equals the origin manifest)', h(a1, 'x-cache-status') === 'MISS' && h(a2, 'x-cache-status') === 'HIT' && a1.sha256 === sha && a2.sha256 === sha, `A: ${h(a1, 'x-cache-status')}/${h(a1, 'x-cache-source')} then ${h(a2, 'x-cache-status')}/${h(a2, 'x-cache-source')}, ${a1.bytes} bytes, origin +${o1 - o0}`);
  add('tenants', 'ISOLATION: tenant B GET of the SAME path is a MISS and goes to the origin again', h(b1, 'x-cache-status') === 'MISS' && o2 - o1 === 1 && b1.sha256 === sha, `B: ${h(b1, 'x-cache-status')}/${h(b1, 'x-cache-source')}, origin +${o2 - o1} (A's cached copy was not served to B)`);
  add('tenants', 'tenant B second GET is a HIT of B\'s own entry', h(b2, 'x-cache-status') === 'HIT', `${h(b2, 'x-cache-status')}/${h(b2, 'x-cache-source')}`);
  const delR = await del(P[0], cachePath(IMG, nonce), U.tenantA.token);
  const a3 = await get(P[0], cachePath(IMG, nonce), U.tenantA.token);
  const b3 = await get(P[0], cachePath(IMG, nonce), U.tenantB.token);
  add('tenants', 'tenant A DELETE (204) purges A: next A GET is a MISS', delR.status === 204 && h(a3, 'x-cache-status') === 'MISS', `DELETE ${delR.status}; A: ${h(a3, 'x-cache-status')}/${h(a3, 'x-cache-source')}`);
  add('tenants', 'tenant A DELETE did NOT purge B: next B GET is still a HIT', h(b3, 'x-cache-status') === 'HIT', `B: ${h(b3, 'x-cache-status')}/${h(b3, 'x-cache-source')}`);
  // x-tenant-id header must be ignored in secure mode
  const path2 = cachePath('/msg/150.json', `sec-hdr-${RUN}`);
  await get(P[0], path2, U.tenantA.token); // A warms
  const spoof = await get(P[0], path2, U.tenantB.token, { 'x-tenant-id': 'tenant-a' });
  add('tenants', 'B sending "x-tenant-id: tenant-a" does not read A\'s entry (header ignored in secure mode)', h(spoof, 'x-cache-status') === 'MISS', `${h(spoof, 'x-cache-status')}/${h(spoof, 'x-cache-source')}`);
  // peers: the entry fetched on node 1 for tenant A is not visible to tenant B on node 2
  const path3 = cachePath('/msg/151.json', `sec-peer-${RUN}`);
  await get(P[0], path3, U.tenantA.token);
  const bOther = await get(P[1], path3, U.tenantB.token);
  const bo = h(bOther, 'x-cache-source');
  add('tenants', 'tenant B on ANOTHER gateway does not receive A\'s entry (peer fill is per tenant): B is served from the origin or from B\'s own entry only', h(bOther, 'x-cache-status') === 'MISS' && bOther.status === 200, `B on node 2: ${h(bOther, 'x-cache-status')}/${bo}`);
}

async function target() {
  const o0 = (await originStats(st.originPort)).requests;
  const crlf = await get(P[0], '/api/v1/cache/img/x%0d%0aX-Injected:%20yes', U.tenantA.token);
  add('target', 'CRLF-injected path (%0d%0a) -> 400', crlf.status === 400, `status ${crlf.status}`);
  const nul = await get(P[0], '/api/v1/cache/img/x%00y', U.tenantA.token);
  add('target', 'NUL byte (%00) in path -> 400', nul.status === 400, `status ${nul.status}`);
  const space = await get(P[0], '/api/v1/cache/img/a%20b', U.tenantA.token);
  add('target', 'space (%20) in path -> 400', space.status === 400, `status ${space.status}`);
  const dots = await get(P[0], '/api/v1/cache/img/../msg/001.json', U.tenantA.token);
  const dots2 = await get(P[0], '/api/v1/cache/img/%2e%2e/msg/001.json', U.tenantA.token);
  add('target', 'dot-segment paths (../ and %2e%2e) -> 400 (one resource, one key)', dots.status === 400 && dots2.status === 400, `status ${dots.status} / ${dots2.status}`);
  const o1 = (await originStats(st.originPort)).requests;
  add('target', 'none of the rejected request targets reached the origin', o1 === o0, `origin requests +${o1 - o0}`);
}

// ---------------------------------------------------------------- cluster mTLS
function openssl(args, input) {
  const r = spawnSync('openssl', args, { encoding: 'utf8', input });
  if (r.status !== 0) throw new Error(`openssl ${args.slice(0, 2).join(' ')} failed: ${r.stderr}`);
  return r.stdout;
}
function issue(dir, name, cn, san, caCrt, caKey) {
  const cnf = join(dir, `${name}.cnf`);
  writeFileSync(cnf, `[req]\ndistinguished_name=dn\nprompt=no\nreq_extensions=v3\n[dn]\nCN=${cn}\n[v3]\nsubjectAltName=DNS:${san}\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth,clientAuth\nbasicConstraints=CA:FALSE\n`);
  openssl(['req', '-newkey', 'rsa:2048', '-nodes', '-keyout', join(dir, `${name}.key`), '-out', join(dir, `${name}.csr`), '-config', cnf]);
  openssl(['x509', '-req', '-in', join(dir, `${name}.csr`), '-CA', caCrt, '-CAkey', caKey, '-CAcreateserial', '-out', join(dir, `${name}.crt`), '-days', '2', '-extfile', cnf, '-extensions', 'v3']);
  return { cert: readFileSync(join(dir, `${name}.crt`)), key: readFileSync(join(dir, `${name}.key`)) };
}

function heartbeatFrame() {
  const b = Buffer.alloc(28);
  Buffer.from('PHAN').copy(b, 0); b[4] = 1; b[5] = 0x05; b.writeUInt16BE(0, 6); b.writeBigUInt64BE(4242n, 8); b.writeBigUInt64BE(1n, 16); b.writeUInt32BE(0, 24);
  return b;
}

/** Connect to a cluster listener and send a Heartbeat frame. Returns what happened. */
function clusterProbe({ port, ca, cert, key, servername }) {
  return new Promise((resolve) => {
    const t0 = Date.now();
    const out = { handshake: 'not completed', response: null, closed: null, error: null };
    let done = false;
    const fin = () => { if (!done) { done = true; try { sock.destroy(); } catch { /* ignore */ } resolve({ ...out, ms: Date.now() - t0 }); } };
    const sock = tls.connect({ host: '127.0.0.1', port, ca, cert, key, servername, minVersion: 'TLSv1.3', rejectUnauthorized: true }, () => {
      out.handshake = 'completed (client side)';
      sock.write(heartbeatFrame());
    });
    sock.on('data', (d) => { if (d.length >= 28 && d.subarray(0, 4).toString() === 'PHAN') { out.response = `frame type 0x${d[5].toString(16).padStart(2, '0')} (${d[5] === 0x06 ? 'HeartbeatResponse' : 'other'})`; fin(); } });
    sock.on('error', (e) => { out.error = e.code || e.message; fin(); });
    sock.on('close', () => { out.closed = true; fin(); });
    setTimeout(fin, 4000);
  });
}

async function cluster() {
  const certs = join(DIR, 'certs');
  const dir = join(DIR, 'security_certs'); mkdirSync(dir, { recursive: true });
  const ca = readFileSync(join(certs, 'ca.crt'));
  const caCrt = join(certs, 'ca.crt'), caKey = join(certs, 'ca.key');
  const cid = st.clusterId;
  const port = st.clusterPortBase; // node 1's cluster listener
  const servername = `node-1.${cid}.phantom.internal`;
  const member = { cert: readFileSync(join(certs, 'n2.crt')), key: readFileSync(join(certs, 'n2.p8.key')) };
  const outsider = issue(dir, 'outsider', 'node-9', `node-9.${cid}.phantom.internal`, caCrt, caKey);
  const impostor = issue(dir, 'impostor', 'node-2', `node-2.${cid}.phantom.internal`, caCrt, caKey); // member NAME, different key
  // foreign CA
  openssl(['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', join(dir, 'foreign_ca.key'), '-out', join(dir, 'foreign_ca.crt'), '-days', '2', '-subj', '/CN=foreign CA']);
  const foreign = issue(dir, 'foreign', 'node-2', `node-2.${cid}.phantom.internal`, join(dir, 'foreign_ca.crt'), join(dir, 'foreign_ca.key'));

  const pos = await clusterProbe({ port, ca, ...member, servername });
  const posOk = pos.response && /HeartbeatResponse/.test(pos.response);
  add('cluster mTLS', `positive control: a real member certificate (node 2) sends a Heartbeat to node 1's cluster listener and gets a HeartbeatResponse`, !!posOk, JSON.stringify(pos));
  const cases = [
    ['no client certificate', { port, ca, servername }],
    ['certificate signed by the cluster CA but for a node (9) that is NOT in the membership', { port, ca, ...outsider, servername }],
    ['certificate signed by the cluster CA with a member\'s name (node 2) but another key (fingerprint not pinned)', { port, ca, ...impostor, servername }],
    ['certificate from a foreign CA (claims node 2)', { port, ca, ...foreign, servername }],
  ];
  for (const [name, args] of cases) {
    const r = await clusterProbe(args);
    const refused = !r.response;
    add('cluster mTLS', `${name}: NO cluster response`, posOk ? refused : false, posOk ? `observed: ${JSON.stringify(r)}` : `INCONCLUSIVE, the positive control failed; observed ${JSON.stringify(r)}`);
  }
  add('cluster mTLS', 'scope note', true, 'proved here: a Heartbeat frame from each client gets or does not get a response; not proved: that no other frame type would be processed (the refusal observed is at the TLS connection level, which suggests it applies to every frame type, but that is an inference; see tests/integration/security_phase3_cluster_tests.rs for the in-process proofs)');
}

// ---------------------------------------------------------------- startup refusal
async function startup() {
  const env = { ...process.env, PHANTOM_REQUIRE_AUTH: 'true', PHANTOM_BIND_ADDR: `127.0.0.1:${st.httpPortBase + 3}`, ORIGIN_URL: `http://127.0.0.1:${st.originPort}` };
  delete env.PHANTOM_JWT_SECRET;
  const run = (envx, label, expectCode) => new Promise((resolve) => {
    const p = spawn(EXE, [], { env: envx, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
    let err = '';
    p.stderr.on('data', (d) => { err += d; }); p.stdout.on('data', (d) => { err += d; });
    const timer = setTimeout(() => { try { p.kill(); } catch { /* ignore */ } resolve({ code: 'STILL RUNNING after 8 s (killed)', err }); }, 8000);
    p.on('exit', (c) => { clearTimeout(timer); resolve({ code: c, err }); });
  });
  const r = await run(env);
  add('startup', 'PHANTOM_REQUIRE_AUTH=true without PHANTOM_JWT_SECRET: the gateway refuses to start with exit code 2', r.code === 2, `exit ${r.code}; message: ${(r.err.split('\n').find((l) => /secret|auth/i.test(l)) || r.err.trim().split('\n')[0] || '').slice(0, 200)}`);
  const env2 = { ...env, PHANTOM_REQUIRE_AUTH: 'false', PHANTOM_BIND_ADDR: `0.0.0.0:${st.httpPortBase + 3}` };
  const r2 = await run(env2);
  add('startup', 'network bind (0.0.0.0) without a secret: the gateway refuses to start (fail-closed, documented)', r2.code === 1 || r2.code === 2, `exit ${r2.code}; ${(r2.err.split('\n').find((l) => /fail|secret|bind/i.test(l)) || '').slice(0, 200)}`);
}

async function report() {
  ev.summary(items.some((i) => i.status === 'FAIL') ? 'attention' : 'passed', [{ label: 'checks passed', value: items.filter((i) => i.status === 'PASS').length, tag: 'measured' }, { label: 'checks failed', value: items.filter((i) => i.status === 'FAIL').length, tag: 'measured' }]);
  const pass = items.filter((i) => i.status === 'PASS').length, fail = items.length - pass;
  const result = { scenario: 'security', startedAt: new Date().toISOString(), cluster: { nodes: st.nodes, ports: P, gatewayExe: st.exe }, users: Object.fromEntries(Object.entries(U).map(([k, v]) => [k, { label: v.label, claims: { ...v.claims } }])), summary: { pass, fail }, items };
  writeFileSync(OUT, JSON.stringify(result, null, 2));
  console.log(`\n[security] ${pass} PASS, ${fail} FAIL; result file ${OUT}`);
  if (EVID && !argv.includes('--no-browser')) await screenshot(result);
  process.exit(fail ? 1 : 0);
}

async function screenshot(result) {
  const { Browser } = await import('../ui/scripts/cdp-lib.mjs');
  mkdirSync(EVID, { recursive: true });
  const esc = (s) => String(s).replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
  const rows = result.items.map((i) => `<tr class="${i.status}"><td>${i.status}</td><td>${esc(i.group)}</td><td>${esc(i.id)}</td><td class="d">${esc(i.detail || '')}</td></tr>`).join('');
  const html = `<!doctype html><meta charset="utf-8"><title>Security scenario</title><style>body{font:13px system-ui;margin:16px;background:#0f1720;color:#e6edf3}h1{font-size:18px}table{border-collapse:collapse;width:100%}td{border-bottom:1px solid #263241;padding:4px 6px;vertical-align:top}.PASS td:first-child{color:#3fb950;font-weight:700}.FAIL td:first-child{color:#ff6b6b;font-weight:700}.d{color:#9fb0c3;font-family:ui-monospace,monospace;font-size:11px}</style><h1>Security scenario: ${result.summary.pass} PASS, ${result.summary.fail} FAIL <small style="color:#9fb0c3">(measured by scripts/security_scenario.mjs against a live secure cluster on loopback; no token or secret is shown)</small></h1><table>${rows}</table>`;
  const f = join(EVID, 'security_report.html'); writeFileSync(f, html);
  const b = new Browser({ evidenceDir: EVID, label: 'sec' });
  try {
    await b.launch(); await b.viewport(1500, 1900);
    await b.goto('file:///' + f.replace(/\\/g, '/'), 800);
    console.log('[security] screenshot', await b.shot('security_report.png', '.', true));
  } finally { await b.close(); }
}

(async () => {
  const STEPS = { auth: ['ingress'], roles: ['ingress'], isolation: ['tenants'], targetsafety: ['target'], mtls: ['cluster'], failclosed: ['startup'], all: ['ingress', 'tenants', 'target', 'cluster', 'startup'] }[PART] || ['ingress', 'tenants', 'target', 'cluster', 'startup'];
  for (const [name, fn] of [['ingress', ingress], ['tenants', tenants], ['target', target], ['cluster', cluster], ['startup', startup]].filter(([n]) => STEPS.includes(n))) {
    try { await fn(); } catch (e) { add(name, 'scenario step crashed', false, String(e && e.stack || e).split('\n').slice(0, 3).join(' | ')); }
  }
  await report();
})();
