// scripts/lib/traffic.mjs
//
// Node traffic client used by the demo scripts (browsers cap at about 6 connections per origin; bursts come from here).
// Every response is verified against the content origin's manifest: status, Content-Type, Content-Length, ETag, byte count
// and the sha256 of the body bytes. All numbers it produces are MEASURED by this client on this host (loopback); latency is
// client-side and closed-loop (a request is issued when a slot frees), so at high concurrency it includes Node client queueing.

import http from 'node:http';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export const agent = new http.Agent({ keepAlive: true, maxSockets: 600, maxFreeSockets: 64 });

export function loadCluster(dir) {
  const st = JSON.parse(readFileSync(join(dir, 'cluster.json'), 'utf8'));
  const tokensFile = join(dir, 'tokens.json');
  st.tokens = st.secure && existsSync(tokensFile) ? JSON.parse(readFileSync(tokensFile, 'utf8')).users : null;
  st.ports = Array.from({ length: st.nodes }, (_, i) => st.httpPortBase + i);
  return st;
}

export const clientStats = { staleSocketRetries: 0 };

/** One HTTP request; retries ONCE when a reused keep-alive socket was closed by the server before any response byte
 *  (the usual idle-timeout race, which browsers also retry); counted in clientStats.staleSocketRetries. */
export async function httpRequest(o) {
  const r = await httpRequestOnce(o);
  if (r.error && /ECONNRESET|EPIPE/.test(r.error) && r.reusedBeforeResponse) { clientStats.staleSocketRetries += 1; return httpRequestOnce(o); }
  return r;
}

/** One HTTP request. Resolves always: { status, headers, bytes, sha256, ms, error, body? }. */
function httpRequestOnce({ port, method = 'GET', path, token = null, keepBody = false, timeoutMs = 30000, headers = {}, host = '127.0.0.1' }) {
  return new Promise((resolve) => {
    const t0 = performance.now();
    const h = { ...headers };
    if (token) h.authorization = `Bearer ${token}`;
    let done = false, gotResponse = false, reused = false;
    const finish = (o) => { if (!done) { done = true; resolve({ ms: performance.now() - t0, ...o }); } };
    const req = http.request({ host, port, method, path, headers: h, agent, timeout: timeoutMs }, (res) => {
      gotResponse = true;
      const hash = createHash('sha256');
      let bytes = 0;
      const chunks = keepBody ? [] : null;
      res.on('data', (c) => { hash.update(c); bytes += c.length; if (chunks) chunks.push(c); });
      res.on('end', () => finish({ status: res.statusCode, headers: res.headers, bytes, sha256: hash.digest('hex'), body: chunks ? Buffer.concat(chunks) : undefined }));
      res.on('error', (e) => finish({ error: `response: ${e.code || e.message}` }));
      res.on('aborted', () => finish({ error: 'response aborted' }));
    });
    req.on('timeout', () => { req.destroy(new Error('timeout')); });
    req.on('error', (e) => finish({ error: e.code || e.message, reusedBeforeResponse: !gotResponse && Boolean(reused) }));
    req.on('socket', (sock) => { reused = sock.reusedSocket; });
    req.end();
  });
}

export const cachePath = (p, nonce) => `/api/v1/cache${p}${nonce ? `?r=${nonce}` : ''}`;

/** Compare a response with the manifest entry; returns an array of problems (empty = byte-for-byte equal). */
export function verify(entry, r) {
  const bad = [];
  if (r.error) return [`transport: ${r.error}`];
  if (r.status !== 200) bad.push(`status ${r.status}`);
  if (!entry) return [...bad, 'not in manifest'];
  if (r.sha256 !== entry.sha256) bad.push('sha256 mismatch');
  if (r.bytes !== entry.bytes) bad.push(`bytes ${r.bytes} != ${entry.bytes}`);
  if (String(r.headers['content-length']) !== String(entry.bytes)) bad.push(`content-length ${r.headers['content-length']}`);
  if (r.headers['content-type'] !== entry.contentType) bad.push(`content-type ${r.headers['content-type']}`);
  if (r.headers.etag !== entry.etag) bad.push(`etag ${r.headers.etag}`);
  return bad;
}

export function percentile(sorted, p) {
  if (!sorted.length) return null;
  const i = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)); // nearest-rank
  return sorted[i];
}

export function mulberry32(a) {
  return () => { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

/** Zipf sampler over keys (rank 1 is the most popular). Keys are shuffled deterministically by seed first. */
export function zipfSampler(keys, { s = 1.0, seed = 1 } = {}) {
  const rnd = mulberry32(seed);
  const order = keys.slice();
  for (let i = order.length - 1; i > 0; i--) { const j = Math.floor(rnd() * (i + 1)); [order[i], order[j]] = [order[j], order[i]]; }
  const cum = []; let sum = 0;
  for (let r = 1; r <= order.length; r++) { sum += 1 / Math.pow(r, s); cum.push(sum); }
  return () => {
    const u = rnd() * sum; let lo = 0, hi = cum.length - 1;
    while (lo < hi) { const m = (lo + hi) >> 1; if (cum[m] < u) lo = m + 1; else hi = m; }
    return order[lo];
  };
}

/**
 * Run a closed-loop load. plan: array of { path, port? } (port optional when pickPort is given).
 * opts: { concurrency, manifest, nonce, token, pickPort(i, deadPorts), failover (bool), onProgress(done), method }
 * With failover, a transport error on a port marks it dead and the request is retried (once per other live port); a
 * request is an ERROR only if every attempt failed or the final answer was not 200 / not byte-identical.
 */
export async function runLoad(plan, { concurrency = 100, manifest, nonce = null, token = null, pickPort, ports, failover = false, onProgress = null, onTick = null, onSample = null, sampleEvery = 50 } = {}) {
  const dead = new Set();
  const res = {
    total: plan.length, concurrency, ok: 0, errors: 0, errorSamples: [], integrityFailures: 0, integritySamples: [],
    sources: {}, statuses: {}, bytes: 0, retried: 0, transportErrorsOnDeadPorts: 0, firstTargetHealthyErrors: 0, perPort: {},
  };
  const lat = [];
  let next = 0, done = 0;
  const t0 = performance.now();
  const worker = async () => {
    while (true) {
      const i = next++;
      if (i >= plan.length) return;
      const item = plan[i];
      const entry = manifest ? manifest[item.path] : null;
      let tried = [];
      let r = null, port = null, firstPort = null;
      for (let attempt = 0; attempt < (failover ? ports.length : 1); attempt++) {
        port = item.port ?? pickPort(i, dead, tried);
        if (firstPort === null) firstPort = port;
        tried.push(port);
        r = await httpRequest({ port, path: cachePath(item.path, nonce), token });
        if (r.error && failover) { if (/ECONNREFUSED|ECONNRESET|aborted|socket/i.test(r.error)) { dead.add(port); res.transportErrorsOnDeadPorts += 1; res.retried += 1; continue; } }
        break;
      }
      done += 1;
      const pp = (res.perPort[port] ||= { n: 0, errors: 0, sources: {} });
      pp.n += 1;
      lat.push(r.ms);
      const src = r.headers && r.headers['x-cache-source'];
      const problems = verify(entry, r);
      if (r.error) { res.errors += 1; pp.errors += 1; if (res.errorSamples.length < 8) res.errorSamples.push({ path: item.path, port, error: r.error }); }
      else if (r.status !== 200) { res.errors += 1; pp.errors += 1; res.statuses[r.status] = (res.statuses[r.status] || 0) + 1; if (res.errorSamples.length < 8) res.errorSamples.push({ path: item.path, port, status: r.status }); }
      else {
        res.statuses[200] = (res.statuses[200] || 0) + 1;
        if (problems.length) { res.integrityFailures += 1; if (res.integritySamples.length < 8) res.integritySamples.push({ path: item.path, port, problems }); }
        else res.ok += 1;
        res.bytes += r.bytes;
        res.sources[src || 'none'] = (res.sources[src || 'none'] || 0) + 1;
        pp.sources[src || 'none'] = (pp.sources[src || 'none'] || 0) + 1;
        const st = r.headers['x-cache-status'];
        res.xCacheStatus = res.xCacheStatus || {}; res.xCacheStatus[st || 'none'] = (res.xCacheStatus[st || 'none'] || 0) + 1;
      }
      if (onSample && (i % sampleEvery === 0 || r.error || r.status !== 200 || problems.length)) onSample({ i, port, path: item.path, r, problems, src });
      if (onProgress) onProgress(done);
    }
  };
  const ticker = onTick ? setInterval(() => onTick(done, res), 200) : null;
  await Promise.all(Array.from({ length: Math.min(concurrency, plan.length) }, worker));
  if (ticker) clearInterval(ticker);
  const secs = (performance.now() - t0) / 1000;
  lat.sort((a, b) => a - b);
  res.seconds = +secs.toFixed(3);
  res.rps = +(plan.length / secs).toFixed(1);
  res.mbPerSec = +(res.bytes / 1e6 / secs).toFixed(1);
  res.latencyMs = { p50: +percentile(lat, 50).toFixed(2), p95: +percentile(lat, 95).toFixed(2), p99: +percentile(lat, 99).toFixed(2), max: +lat[lat.length - 1].toFixed(2) };
  res.deadPorts = [...dead];
  return res;
}

export async function getJson(port, path, token) {
  const r = await httpRequest({ port, path, token, keepBody: true });
  if (r.error || r.status !== 200) return { error: r.error || `status ${r.status}` };
  try { return JSON.parse(r.body.toString('utf8')); } catch (e) { return { error: `bad json: ${e.message}` }; }
}

export async function originStats(originPort) {
  const j = await getJson(originPort, '/__stats');
  return j && !j.error ? j : null;
}
export async function originManifest(originPort) { return getJson(originPort, '/__manifest'); }

export async function gatewayOutcomes(port, token) {
  const j = await getJson(port, '/api/v1/metrics', token);
  if (j.error) return { error: j.error };
  const o = j.metrics && j.metrics.outcomes;
  return { responses: o && o.responses, peer_reads_without_hit: o && o.peer_reads_without_hit, raw: j.metrics };
}

/** Working-set size (KB) of a process through tasklist (Windows). null if unavailable. */
export function rssKb(pid) {
  try {
    const out = execFileSync('tasklist', ['/FI', `PID eq ${pid}`, '/FO', 'CSV', '/NH'], { encoding: 'utf8' }).trim();
    const m = out.match(/"([\d.,]+) K"\s*$/);
    return m ? Number(m[1].replace(/[.,]/g, '')) : null;
  } catch { return null; }
}
