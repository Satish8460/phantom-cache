// scripts/lib/scenario_common.mjs
//
// Shared helpers for the lane-L5 scenario scripts (freshness, eviction, operability, benchmark).
// Development tooling only: nothing here is part of the gateway.
//
//  * deterministic REAL content: valid PNG images (30 KB .. 900 KB, plus a > 1 MiB file), JSON, text, HTML;
//  * a scriptable content origin that counts requests, records conditional headers and can be changed at runtime;
//    it also runs standalone:  node scripts/lib/scenario_common.mjs origin --port N   (catalog mode, used by the benchmark);
//  * gateway process spawn/kill helpers, a keep-alive HTTP client that returns the full body, sha256, a recorder.
//
// Every value a scenario reports is measured on this host; see docs/research/benchmark_protocol.md.

import { createHash } from 'node:crypto';
import http from 'node:http';
import net from 'node:net';
import zlib from 'node:zlib';
import { spawn } from 'node:child_process';
import { mkdirSync, openSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const emit = (kind, text, extra = {}) => { if (process.argv.includes('--events')) process.stdout.write('@EV ' + JSON.stringify({ t: new Date().toISOString(), kind, text, ...extra }) + String.fromCharCode(10)); };
import os from 'node:os';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export const sha256 = (buf) => createHash('sha256').update(buf).digest('hex');

// ------------------------------------------------------------------ deterministic content
export function rng(seed) {
  // xorshift32 (never 0)
  let s = (seed >>> 0) || 0x9e3779b9;
  const next = () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return s; };
  return { next, float: () => next() / 4294967296 };
}

const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; }
  return t;
})();
const crc32 = (buf) => { let c = 0xffffffff; for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
const chunk = (type, data) => {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
};

/** A valid 8-bit RGB PNG, w x h, a colour gradient (top 3 bits) with seeded grain (low 5 bits). */
export function makePng(w, h, seed) {
  const r = rng(seed);
  const stride = w * 3 + 1;
  const raw = Buffer.alloc(stride * h);
  const hueShift = r.next() & 0xff;
  for (let y = 0; y < h; y++) {
    const o = y * stride;
    raw[o] = 0; // filter: none
    for (let x = 0; x < w * 3; x += 4) {
      const v = r.next();
      for (let k = 0; k < 4 && x + k < w * 3; k++) {
        const px = ((x + k) / 3) | 0, ch = (x + k) % 3;
        const grad = ch === 0 ? (px * 255) / w : ch === 1 ? (y * 255) / h : (hueShift + px + y) & 0xff;
        raw[o + 1 + x + k] = (grad & 0xe0) | ((v >>> (8 * k)) & 0x1f);
      }
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 2;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 1 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/** PNG whose encoded size is within ~12 % of targetBytes (one calibration pass). */
export function makePngOfSize(targetBytes, seed) {
  let side = Math.max(16, Math.round(Math.sqrt(targetBytes / 3 / 0.62)));
  let png = makePng(side, side, seed);
  const ratio = png.length / targetBytes;
  if (ratio < 0.9 || ratio > 1.1) { side = Math.max(16, Math.round(side / Math.sqrt(ratio))); png = makePng(side, side, seed); }
  return png;
}

export function isValidPng(buf) {
  if (buf.length < 45 || buf.readUInt32BE(0) !== 0x89504e47 || buf.readUInt32BE(4) !== 0x0d0a1a0a) return false;
  // IHDR CRC and IEND at the end
  const ihdrCrc = buf.readUInt32BE(29);
  return crc32(buf.subarray(12, 29)) === ihdrCrc && buf.subarray(buf.length - 8, buf.length - 4).toString('latin1') === 'IEND';
}

export function makeJson(seed, approxBytes) {
  const r = rng(seed);
  const words = ['alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot', 'golf', 'hotel', 'india', 'juliet', 'kilo', 'lima'];
  const msgs = [];
  let size = 0;
  while (size < approxBytes) {
    const m = { id: r.next(), from: `user${r.next() % 997}`, text: Array.from({ length: 8 + (r.next() % 12) }, () => words[r.next() % words.length]).join(' '), ts: 1700000000 + (r.next() % 100000000) };
    msgs.push(m); size += JSON.stringify(m).length + 1;
  }
  return Buffer.from(JSON.stringify({ seed, messages: msgs }));
}
export function makeText(seed, approxBytes) {
  const r = rng(seed);
  const words = ['cache', 'origin', 'stale', 'fresh', 'edge', 'node', 'shard', 'epoch', 'purge', 'token', 'tenant', 'budget'];
  const out = [];
  let size = 0;
  while (size < approxBytes) { const l = Array.from({ length: 10 }, () => words[r.next() % words.length]).join(' ') + '\n'; out.push(l); size += l.length; }
  return Buffer.from(out.join(''));
}
export function makeHtml(seed, approxBytes) {
  return Buffer.from(`<!doctype html><html><head><meta charset="utf-8"><title>doc ${seed}</title></head><body><pre>${makeText(seed, approxBytes).toString()}</pre></body></html>`);
}

/** Benchmark catalog: object i (0-based) is deterministic. Mix chosen so total size stays small (see protocol). */
export function catalogObject(i) {
  const m = i % 50;
  if (m < 36) return { ext: 'json', type: 'application/json', body: makeJson(i + 1, 1000 + ((i * 7919) % 5000)) }; // 72 %: 1-6 KB
  if (m < 44) return { ext: 'txt', type: 'text/plain; charset=utf-8', body: makeText(i + 1, 2000 + ((i * 104729) % 6000)) }; // 16 %: 2-8 KB
  if (m < 49) return { ext: 'png', type: 'image/png', body: makePngOfSize(30000 + ((i * 7727) % 30000), i + 1) }; // 10 %: 30-60 KB
  return { ext: 'png', type: 'image/png', body: makePngOfSize(300000 + ((i * 7727) % 600000), i + 1) }; // 2 %: 300-900 KB
}

// ------------------------------------------------------------------ scriptable content origin
/**
 * startOrigin({port}) -> { server, port, set(path, spec|fn), count(path), total(), log, reset(), close(), url }
 * spec: { body:Buffer, type, status=200, headers:{}, etag, cacheControl, delayMs, notModified(bool, true => if the request carries
 *         a matching If-None-Match answer 304), destroy(bool: drop the socket), hang(bool) }
 * Catalog mode: GET /obj/<i>.<ext> serves catalogObject(i) with `public, max-age=3600` and a strong ETag (sha-derived);
 *   GET /__manifest returns {path: sha256} for the first N catalog objects it has generated or is asked for (?n=).
 * GET /__stats -> {requests, byPath}, POST /__reset.
 */
export function startOrigin({ port, host = '127.0.0.1', catalog = false, delayMs = 0, maxConcurrent = 0 } = {}) {
  let inService = 0; const waitQ = [];
  const acquire = () => new Promise((ok) => { if (!maxConcurrent || inService < maxConcurrent) { inService++; ok(); } else waitQ.push(ok); });
  const release = () => { const n = waitQ.shift(); if (n) n(); else inService--; };
  const specs = new Map();
  const counts = new Map();
  const reqLog = []; // {t, path, inm}
  let total = 0;
  const cat = new Map();
  const getCat = (i) => { let o = cat.get(i); if (!o) { o = catalogObject(i); o.sha = sha256(o.body); cat.set(i, o); } return o; };
  const sockets = new Set();
  const server = http.createServer(async (req, res) => {
    const path = req.url.split('?')[0];
    if (path === '/__stats') { res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify({ requests: total, byPath: Object.fromEntries(counts) })); }
    if (path === '/__reset') { total = 0; counts.clear(); reqLog.length = 0; return res.end('ok'); }
    if (path === '/__manifest') {
      const n = Number(new URL(req.url, 'http://x').searchParams.get('n') || 1000);
      const m = {}; for (let i = 0; i < n; i++) { const o = getCat(i); m[`/obj/${i}.${o.ext}`] = { sha256: o.sha, bytes: o.body.length, type: o.type }; }
      res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify(m));
    }
    total += 1; counts.set(path, (counts.get(path) || 0) + 1);
    const inm = req.headers['if-none-match'];
    reqLog.push({ t: Date.now(), path, inm: inm || null });
    if (catalog) {
      const mt = /^\/obj\/(\d+)\.(\w+)$/.exec(path);
      if (mt) {
        const o = getCat(Number(mt[1]));
        if (delayMs || maxConcurrent) { await acquire(); try { await sleep(delayMs); } finally { release(); } }
        const etag = `"${o.sha.slice(0, 32)}"`;
        res.setHeader('content-type', o.type); res.setHeader('etag', etag); res.setHeader('cache-control', 'public, max-age=3600');
        if (inm === etag) { res.statusCode = 304; return res.end(); }
        res.setHeader('content-length', o.body.length); return res.end(o.body);
      }
    }
    let spec = specs.get(path);
    if (typeof spec === 'function') spec = spec(req, { count: counts.get(path), inm });
    if (!spec) { res.statusCode = 404; res.setHeader('cache-control', 'no-store'); return res.end('not found'); }
    if (spec.destroy) return req.socket.destroy();
    if (spec.hang) return; // never answer
    if (spec.delayMs) await sleep(spec.delayMs);
    const status = spec.status ?? 200;
    if (spec.etag) res.setHeader('etag', spec.etag);
    if (spec.cacheControl) res.setHeader('cache-control', spec.cacheControl);
    for (const [k, v] of Object.entries(spec.headers || {})) res.setHeader(k, v);
    if (spec.notModified && inm && spec.etag && inm === spec.etag) { res.statusCode = 304; return res.end(); }
    res.statusCode = status;
    res.setHeader('content-type', spec.type || 'application/octet-stream');
    const body = spec.body ?? Buffer.alloc(0);
    res.setHeader('content-length', body.length);
    res.end(body);
  });
  server.on('connection', (s) => { sockets.add(s); s.on('close', () => sockets.delete(s)); });
  const api = {
    server, port, url: `http://${host}:${port}`,
    set: (p, spec) => specs.set(p, spec),
    count: (p) => counts.get(p) || 0,
    total: () => total,
    log: reqLog,
    reset: () => { total = 0; counts.clear(); reqLog.length = 0; },
    close: () => new Promise((ok) => { for (const s of sockets) s.destroy(); server.close(() => ok()); }),
    listen: () => new Promise((ok, bad) => { server.once('error', bad); server.listen(port, host, ok); }),
  };
  return api;
}

// ------------------------------------------------------------------ http client
export function httpReq({ port, host = '127.0.0.1', method = 'GET', path, headers = {}, body, agent, timeoutMs = 20000 }) {
  return new Promise((ok) => {
    const t0 = process.hrtime.bigint();
    const done = (r) => ok({ ms: Number(process.hrtime.bigint() - t0) / 1e6, ...r });
    const req = http.request({ host, port, method, path, headers, agent: agent ?? false, timeout: timeoutMs }, (res) => {
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => { const b = Buffer.concat(chunks); done({ ok: true, status: res.statusCode, headers: res.headers, body: b }); });
      res.on('error', (e) => done({ ok: false, error: String(e.code || e.message) }));
      res.on('aborted', () => done({ ok: false, error: 'aborted' }));
    });
    req.on('timeout', () => { req.destroy(new Error('timeout')); });
    req.on('error', (e) => done({ ok: false, error: String(e.code || e.message) }));
    if (body) req.write(body);
    req.end();
  });
}

export async function rawRequest(port, bytes, { timeoutMs = 5000, host = '127.0.0.1' } = {}) {
  return new Promise((ok) => {
    const s = net.connect(port, host);
    const chunks = [];
    let settled = false;
    const fin = (note) => { if (settled) return; settled = true; s.destroy(); const text = Buffer.concat(chunks).toString('latin1'); const m = /^HTTP\/1\.[01] (\d{3})/.exec(text); ok({ status: m ? Number(m[1]) : null, note, head: text.split('\r\n\r\n')[0].slice(0, 300) }); };
    s.setTimeout(timeoutMs, () => fin('timeout'));
    s.on('connect', () => { s.write(bytes); });
    s.on('data', (c) => { chunks.push(c); if (Buffer.concat(chunks).includes('\r\n\r\n')) setTimeout(() => fin('response'), 100); });
    s.on('close', () => fin('closed'));
    s.on('error', (e) => fin(`error:${e.code}`));
  });
}

// ------------------------------------------------------------------ gateway process
export function spawnGateway({ exe, port, originUrl, env = {}, outDir, name, args = [] }) {
  mkdirSync(outDir, { recursive: true });
  const outPath = join(outDir, `${name}.out.log`), errPath = join(outDir, `${name}.err.log`);
  const penv = { ...process.env };
  for (const k of Object.keys(penv)) if (k.startsWith('PHANTOM_') || k === 'ORIGIN_URL') delete penv[k];
  const e = { ...penv, PHANTOM_BIND_ADDR: `127.0.0.1:${port}`, ...(originUrl ? { ORIGIN_URL: originUrl } : {}), ...env };
  const out = openSync(outPath, 'w'), err = openSync(errPath, 'w');
  const proc = spawn(exe, args, { env: e, stdio: ['ignore', out, err], windowsHide: true });
  const exited = new Promise((ok) => proc.on('exit', (code, sig) => ok({ code, sig })));
  let exitInfo = null; exited.then((x) => { exitInfo = x; });
  return {
    proc, port, outPath, errPath, exited,
    get alive() { return exitInfo === null; },
    get exitInfo() { return exitInfo; },
    kill: async () => { if (exitInfo === null) { try { proc.kill(); } catch { /* gone */ } } await Promise.race([exited, sleep(3000)]); },
    logs: () => `${existsSync(outPath) ? readFileSync(outPath, 'utf8') : ''}\n${existsSync(errPath) ? readFileSync(errPath, 'utf8') : ''}`,
    base: `http://127.0.0.1:${port}`,
  };
}

export async function waitReady(port, timeoutMs = 20000) {
  const t0 = Date.now();
  while (Date.now() - t0 < timeoutMs) {
    const r = await httpReq({ port, path: '/health/ready', timeoutMs: 1000 });
    if (r.ok && r.status === 200) return Date.now() - t0;
    await sleep(100);
  }
  return null;
}

export async function metrics(port) {
  const r = await httpReq({ port, path: '/api/v1/metrics', timeoutMs: 5000 });
  if (!r.ok || r.status !== 200) return null;
  const j = JSON.parse(r.body.toString());
  return j.metrics;
}
/** flatten {src,v} leaves for readable logs */
export const mv = (m, ...path) => { let x = m; for (const p of path) x = x?.[p]; return x && typeof x === 'object' && 'v' in x ? x.v : x; };

// ------------------------------------------------------------------ host facts and recorder
export function hostFacts() {
  const cpus = os.cpus();
  return {
    os: `${os.type()} ${os.release()} ${os.version?.() ?? ''}`.trim(), arch: os.arch(),
    cpu_model: cpus[0]?.model, logical_cpus: cpus.length, total_mem_bytes: os.totalmem(), free_mem_bytes: os.freemem(),
    node: process.version, hostname_hash: sha256(Buffer.from(os.hostname())).slice(0, 8),
  };
}

export function recorder(title, outDir, { prefix } = {}) {
  mkdirSync(outDir, { recursive: true });
  const lines = [];
  const asserts = [];
  const findings = [];
  const data = {};
  const stamp = () => new Date().toISOString();
  const log = (m) => { const l = `${stamp()} ${m}`; lines.push(l); console.log(`[${prefix || title}] ${m}`); if (!/^(PASS|FAIL) /.test(m)) emit('observation', String(m).slice(0, 400)); };
  const check = (cond, name, detail) => {
    asserts.push({ name, pass: !!cond, detail: detail === undefined ? undefined : detail });
    emit('check', name, { pass: !!cond, detail: detail === undefined ? undefined : (typeof detail === 'string' ? detail : JSON.stringify(detail)).slice(0, 400) });
    log(`${cond ? 'PASS' : 'FAIL'} ${name}${detail !== undefined ? '  -> ' + (typeof detail === 'string' ? detail : JSON.stringify(detail)) : ''}`);
    return !!cond;
  };
  const finding = (id, text, repro) => { findings.push({ id, text, repro }); log(`FINDING ${id}: ${text}`); };
  const finish = (extra = {}) => {
    const passed = asserts.filter((a) => a.pass).length, failed = asserts.length - passed;
    const result = { scenario: title, finished_at: stamp(), host: hostFacts(), assertions_passed: passed, assertions_failed: failed, assertions: asserts, findings, data, ...extra };
    writeFileSync(join(outDir, `${title}.result.json`), JSON.stringify(result, null, 2));
    writeFileSync(join(outDir, `${title}.log.txt`), lines.join('\n') + '\n');
    emit('summary', '', { verdict: failed === 0 ? 'passed' : 'attention', numbers: [{ label: 'assertions passed', value: passed, tag: 'measured' }, { label: 'assertions failed', value: failed, tag: 'measured' }, { label: 'findings recorded', value: findings.length, tag: 'measured' }] });
    log(`${failed === 0 ? 'ALL ASSERTIONS PASSED' : 'FAILED'}: ${passed} passed, ${failed} failed; findings: ${findings.length}; results in ${outDir}`);
    return failed === 0;
  };
  return { log, check, finding, finish, data, asserts, findings };
}

export function argv() {
  const a = process.argv.slice(2);
  const opt = (name, def) => { const i = a.indexOf(`--${name}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : def; };
  const flag = (name) => a.includes(`--${name}`);
  return { opt, flag, raw: a };
}
export const DEFAULT_EXE = 'D:\\ClaudeTemp\\audit\\phantom_rev6.exe';
export const defaultOut = (name) => resolve('D:\\ClaudeTemp\\lanes\\evidence\\L5', name);

// ------------------------------------------------------------------ standalone origin (benchmark)
if (process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url)) && process.argv[2] === 'origin') {
  const i = process.argv.indexOf('--port');
  const port = Number(process.argv[i + 1]);
  const ai = (n, d) => { const k = process.argv.indexOf(n); return k !== -1 ? Number(process.argv[k + 1]) : d; };
  const o = startOrigin({ port, catalog: true, delayMs: ai('--delay-ms', 0), maxConcurrent: ai('--max-concurrent', 0) });
  await o.listen();
  console.log(`origin(catalog) listening on ${port}`);
}
