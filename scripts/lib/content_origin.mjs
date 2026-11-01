#!/usr/bin/env node
// scripts/lib/content_origin.mjs
//
// A REAL-CONTENT counting origin for demos and traffic checks (development tool, not part of the gateway).
// It serves deterministic generated objects, so a client can prove integrity by comparing sha256 values:
//   /img/tile-01.png .. tile-24.png   PNG images, about 30 KB to 900 KB (pattern + big number + colour), generated with
//                                     Node `zlib` only (stored deflate blocks, so the byte size is predictable)
//   /img/huge-1.2mb.png               ONE image above 1 MiB (shows the peer-fill size limit honestly)
//   /msg/NNN.json                     JSON "chat messages" (200)
//   /text/NNN.txt                     plain text (60)
//   /page/NNN.html                    HTML pages (30)
//   /__manifest                       JSON { path: { sha256, bytes, contentType, etag } } for every object
//   /__stats                          JSON { requests, bytes, byPath } counting only object requests (not /__* paths)
// Every object carries Content-Type, Content-Length, a strong ETag (sha256 prefix), Cache-Control: public, max-age=300
// (`public` lets the gateway store authenticated reads, docs/user/security.md), and answers If-None-Match with 304.
// An artificial service time (default 25 ms, --delay) makes concurrent misses overlap, as a real origin would; it is
// a parameter of this origin, not a gateway property.
//
//   node scripts/lib/content_origin.mjs --port 19200 [--delay 25]

import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { deflateSync, crc32 } from 'node:zlib';
import { fileURLToPath } from 'node:url';

// ---------------------------------------------------------------- PNG generation
const FONT = { // 5x7 digits
  0: ['01110', '10001', '10011', '10101', '11001', '10001', '01110'],
  1: ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
  2: ['01110', '10001', '00001', '00010', '00100', '01000', '11111'],
  3: ['11110', '00001', '00001', '01110', '00001', '00001', '11110'],
  4: ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
  5: ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
  6: ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
  7: ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
  8: ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
  9: ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
};
const hsl = (h, s, l) => { // -> [r,g,b]
  const a = s * Math.min(l, 1 - l);
  const f = (n) => { const k = (n + h / 30) % 12; return Math.round(255 * (l - a * Math.max(-1, Math.min(k - 3, Math.min(9 - k, 1))))); };
  return [f(0), f(8), f(4)];
};
const COLOUR_NAMES = ['red', 'orange', 'amber', 'lime', 'green', 'teal', 'cyan', 'blue', 'indigo', 'violet', 'magenta', 'rose'];

export function makePng(w, h, number, hue) {
  const raw = Buffer.alloc((w * 3 + 1) * h);
  const bg1 = hsl(hue, 0.7, 0.35), bg2 = hsl((hue + 40) % 360, 0.75, 0.62);
  const fg = hsl(hue, 0.2, 0.97);
  const digits = String(number).split('');
  const cell = Math.max(2, Math.floor(Math.min(w / (digits.length * 6 + 1), h / 10)));
  const tw = digits.length * 6 * cell - cell, th = 7 * cell;
  const x0 = Math.floor((w - tw) / 2), y0 = Math.floor((h - th) / 2);
  let o = 0;
  for (let y = 0; y < h; y++) {
    raw[o++] = 0;
    for (let x = 0; x < w; x++) {
      const t = (x + y) / (w + h);
      let r = bg1[0] + (bg2[0] - bg1[0]) * t, g = bg1[1] + (bg2[1] - bg1[1]) * t, b = bg1[2] + (bg2[2] - bg1[2]) * t;
      if (((x >> 4) + (y >> 4)) & 1) { r *= 0.93; g *= 0.93; b *= 0.93; } // checker pattern
      if (x >= x0 - cell && x < x0 + tw + cell && y >= y0 - cell && y < y0 + th + cell) { r *= 0.35; g *= 0.35; b *= 0.35; } // plate behind digits
      if (x >= x0 && x < x0 + tw && y >= y0 && y < y0 + th) {
        const cx = Math.floor((x - x0) / cell), cy = Math.floor((y - y0) / cell);
        const di = Math.floor(cx / 6), col = cx % 6;
        if (col < 5 && FONT[digits[di]][cy][col] === '1') { r = fg[0]; g = fg[1]; b = fg[2]; }
      }
      raw[o++] = r; raw[o++] = g; raw[o++] = b;
    }
  }
  const chunk = (type, data) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const c = Buffer.alloc(4); c.writeUInt32BE(crc32(td) >>> 0);
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw, { level: 0 })), chunk('IEND', Buffer.alloc(0))]);
}

// ---------------------------------------------------------------- catalog
const WORDS = ['cache', 'edge', 'origin', 'tenant', 'peer', 'stale', 'fresh', 'purge', 'shard', 'epoch', 'quorum', 'gateway'];
const NAMES = ['alice', 'bob', 'carol', 'dave', 'erin', 'frank'];

export function buildCatalog() {
  const objs = new Map(); // path -> { body, contentType, sha256, etag }
  const add = (path, body, contentType, extra = {}) => {
    const buf = Buffer.isBuffer(body) ? body : Buffer.from(body, 'utf8');
    const sha256 = createHash('sha256').update(buf).digest('hex');
    objs.set(path, { body: buf, contentType, sha256, etag: `"sha256-${sha256.slice(0, 24)}"`, ...extra });
  };
  for (let i = 1; i <= 24; i++) {
    const target = Math.round(30000 * Math.pow(30, (i - 1) / 23)); // 30 KB .. 900 KB
    const side = Math.max(40, Math.round(Math.sqrt(target / 3)));
    const hue = ((i - 1) * 30) % 360;
    add(`/img/tile-${String(i).padStart(2, '0')}.png`, makePng(side, side, i, hue), 'image/png', { kind: 'image', label: `#${i} ${COLOUR_NAMES[(i - 1) % 12]}`, side });
  }
  add('/img/huge-1.2mb.png', makePng(640, 640, 99, 200), 'image/png', { kind: 'image', label: '#99 above 1 MiB', side: 640 });
  for (let i = 1; i <= 200; i++) {
    const n = String(i).padStart(3, '0');
    const msg = { id: i, from: NAMES[i % NAMES.length], to: NAMES[(i * 3 + 1) % NAMES.length], ts: 1700000000 + i * 37, text: `message ${i}: ${WORDS[i % 12]} ${WORDS[(i * 5 + 2) % 12]} ${WORDS[(i * 7 + 3) % 12]} `.repeat(1 + (i % 7)).trim() };
    add(`/msg/${n}.json`, JSON.stringify(msg), 'application/json', { kind: 'json' });
  }
  for (let i = 1; i <= 60; i++) {
    const n = String(i).padStart(3, '0');
    add(`/text/${n}.txt`, `Note ${i}\n` + Array.from({ length: 5 + (i % 20) }, (_, k) => `line ${k + 1}: ${WORDS[(i + k) % 12]} ${WORDS[(i * 3 + k) % 12]}`).join('\n') + '\n', 'text/plain; charset=utf-8', { kind: 'text' });
  }
  for (let i = 1; i <= 30; i++) {
    const n = String(i).padStart(3, '0');
    add(`/page/${n}.html`, `<!doctype html><html><head><meta charset="utf-8"><title>Page ${i}</title></head><body><h1>Page ${i}</h1>${Array.from({ length: 3 + (i % 9) }, (_, k) => `<p>${WORDS[(i + k) % 12]} ${WORDS[(i * 2 + k) % 12]} paragraph ${k + 1}.</p>`).join('')}</body></html>`, 'text/html; charset=utf-8', { kind: 'html' });
  }
  for (let i = 1; i <= 10; i++) add(`/short/${String(i).padStart(3, '0')}.json`, JSON.stringify({ id: i, note: 'short-lived object, max-age 8 s' }), 'application/json', { kind: 'json', maxAge: 8 });
  return objs;
}

export function manifestOf(objs) {
  const m = {};
  for (const [p, o] of objs) m[p] = { sha256: o.sha256, bytes: o.body.length, contentType: o.contentType, etag: o.etag, kind: o.kind, label: o.label || null };
  return m;
}

// ---------------------------------------------------------------- server
export function startContentOrigin({ port, host = '127.0.0.1', delayMs = 25 }) {
  const objs = buildCatalog();
  const manifest = manifestOf(objs);
  const stats = { requests: 0, bytes: 0, notModified: 0, notFound: 0, byPath: {} };
  const server = createServer((req, res) => {
    const path = (req.url || '/').split('?')[0];
    if (path === '/__stats') { res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify(stats)); }
    if (path === '/__manifest') { res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify(manifest)); }
    const o = objs.get(path);
    stats.requests += 1;
    stats.byPath[path] = (stats.byPath[path] || 0) + 1;
    setTimeout(() => {
      if (!o) { stats.notFound += 1; res.statusCode = 404; res.setHeader('content-type', 'text/plain'); return res.end('not found'); }
      res.setHeader('etag', o.etag);
      res.setHeader('cache-control', `public, max-age=${o.maxAge || 300}`);
      if (req.headers['if-none-match'] === o.etag) { stats.notModified += 1; res.statusCode = 304; return res.end(); }
      res.setHeader('content-type', o.contentType);
      res.setHeader('content-length', String(o.body.length));
      stats.bytes += o.body.length;
      res.end(o.body);
    }, delayMs);
  });
  return new Promise((ok, bad) => { server.once('error', bad); server.listen(port, host, () => ok({ server, stats, manifest })); });
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const a = process.argv.slice(2);
  const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
  const port = Number(opt('port', 19200));
  const delayMs = Number(opt('delay', 25));
  startContentOrigin({ port, delayMs }).then(({ manifest }) => console.log(`content origin listening on ${port}, ${Object.keys(manifest).length} objects, service time ${delayMs} ms`));
}
