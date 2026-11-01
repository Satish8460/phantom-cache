import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { FORBIDDEN_PATTERNS, listSourceFiles, scanSource } from '../scripts/forbidden-patterns.mjs';

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src');

test('live UI source contains no fixed example numbers, random data, or client-side metric arithmetic', () => {
  assert.deepEqual(scanSource(SRC), []);
});

test('the scanner has no exempt files and the legacy SimDemo is gone', () => {
  assert.equal(fs.existsSync(path.join(SRC, 'SimDemo.jsx')), false);
  const scanner = fs.readFileSync(new URL('../scripts/forbidden-patterns.mjs', import.meta.url), 'utf8');
  assert.doesNotMatch(scanner, /EXEMPT_FILES|simDemoImporters/);
  for (const f of listSourceFiles(SRC)) assert.doesNotMatch(fs.readFileSync(f, 'utf8'), /SimDemo/, `${path.relative(SRC, f)} still refers to SimDemo`);
});

test('the forbidden scanner actually detects planted violations (it is not vacuous)', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-forbidden-'));
  try {
    const planted = [
      'const hit = 0.94;',
      'const qps = 12050;',
      'const x = Math.random();',
      "const url = base + '/api/events';",
      'const y = metric.v * 100;',
      "const z = 'avoided_compute_dollars';",
      "const hot = '9200 req/s';",
    ];
    fs.writeFileSync(path.join(dir, 'bad.js'), planted.join('\n'));
    const found = scanSource(dir);
    const lines = new Set(found.map((v) => v.line));
    for (let i = 1; i <= planted.length; i += 1) assert.ok(lines.has(i), `line ${i} not flagged: ${planted[i - 1]}`);
    // the sanctioned file may scale values; a schema file may name forbidden fields
    fs.writeFileSync(path.join(dir, 'bad.js'), '');
    fs.writeFileSync(path.join(dir, 'format.js'), 'export const pct = (f) => f.v * 100;');
    fs.writeFileSync(path.join(dir, 'schema.js'), "export const F = ['avoided_compute_dollars'];");
    assert.deepEqual(scanSource(dir), []);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('every original fixed-example pattern is still enforced', () => {
  const descs = FORBIDDEN_PATTERNS.map((p) => p.desc).join('|');
  for (const needle of ['94%', '88%', '70%', '85%', '40%', '$0.15', '$0.05', '8000', '9200', 'PIE trace', 'PIE decision', 'counterfactual', 'schema-1']) {
    assert.ok(descs.includes(needle), `missing pattern for ${needle}`);
  }
});

test('colours live in tokens.css only: no hex colours or colour styles in UI source', () => {
  for (const f of listSourceFiles(SRC)) {
    const text = fs.readFileSync(f, 'utf8');
    assert.doesNotMatch(text, /#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{3}\b(?![0-9a-zA-Z-])/, `${path.relative(SRC, f)} hard-codes a colour`);
  }
  const appCss = fs.readFileSync(path.join(SRC, 'styles/app.css'), 'utf8');
  assert.doesNotMatch(appCss, /#[0-9a-fA-F]{3,8}\b/, 'app.css must use tokens only');
});

test('axe-core stays out of the production source tree', () => {
  for (const f of listSourceFiles(SRC)) {
    assert.doesNotMatch(fs.readFileSync(f, 'utf8'), /axe-core|axe\.min/, `${path.relative(SRC, f)} must not reference axe-core`);
  }
});

test('the UI ships no new dependencies and no network calls other than the gateway', () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(SRC, '../package.json'), 'utf8'));
  assert.deepEqual(Object.keys(pkg.dependencies).sort(), ['lucide-react', 'react', 'react-dom']);
  // axe-core 4.14.0 (MPL-2.0) is the one approved dev-only addition (owner approval 2026-10-30); it is used only by
  // scripts/check-axe.mjs and must never be imported under src/ (checked by the next test).
  assert.deepEqual(Object.keys(pkg.devDependencies).sort(), ['@vitejs/plugin-react', 'axe-core', 'vite']);
  assert.equal(pkg.devDependencies['axe-core'], '4.14.0', 'axe-core must stay exactly pinned');
  for (const f of listSourceFiles(SRC)) {
    const text = fs.readFileSync(f, 'utf8');
    assert.doesNotMatch(text, /\bfetch\s*\(|XMLHttpRequest|sendBeacon|WebSocket/, `${path.relative(SRC, f)} makes an unexpected network call`);
  }
});
