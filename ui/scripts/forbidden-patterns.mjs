// Shared by scripts/check-forbidden.mjs and test/forbidden.test.mjs so the CLI and the tests cannot drift.
import fs from 'node:fs';
import path from 'node:path';

export const FORBIDDEN_PATTERNS = [
  { pattern: /\b0\.94\b/, desc: 'hardcoded 94% flash-crowd hit rate' },
  { pattern: /\b0\.88\b/, desc: 'hardcoded 88% normal hit rate' },
  { pattern: /\b0\.70\b/, desc: 'hardcoded 70% baseline hit rate' },
  { pattern: /\b0\.85\b/, desc: 'hardcoded 85% flash-crowd coalescing ratio' },
  { pattern: /\b0\.40\b/, desc: 'hardcoded 40% normal coalescing ratio' },
  { pattern: /\b0\.15\b/, desc: 'hardcoded backend cost per million ($0.15)' },
  { pattern: /\b0\.05\b/, desc: 'hardcoded node cost per hour ($0.05)' },
  { pattern: /\b8000\b/, desc: 'hardcoded break-even QPS (8000)' },
  { pattern: /\b9200\b/, desc: 'hardcoded fake hot-key QPS (9200)' },
  { pattern: /\b12,?050\b/, desc: 'hardcoded fake initial throughput (12050)' },
  { pattern: /\b62,?000\b/, desc: 'hardcoded fake flash-crowd throughput (62000)' },
  { pattern: /\b536870912\b/, desc: 'hardcoded fake node memory (512 MiB)' },
  { pattern: /\b28\.4\b/, desc: 'hardcoded fake node CPU (28.4)' },
  { pattern: /Math\.random\s*\(/, desc: 'random number generation in live UI code' },
  { pattern: /HOT KEY DENSITY DETECTED/, desc: 'fabricated PIE trace text' },
  { pattern: /✓ COMMITTED/, desc: 'fabricated PIE decision text' },
  { pattern: /p50_latency_ms\s*\*\s*2/, desc: 'fabricated phantom-off counterfactual multiplier' },
  { pattern: /p95_latency_ms\s*\*\s*2\.5/, desc: 'fabricated phantom-off counterfactual multiplier' },
  { pattern: /p99_latency_ms\s*\*\s*3/, desc: 'fabricated phantom-off counterfactual multiplier' },
  { pattern: /avoided_compute|backend_load_ratio/, desc: 'simulation-only economics field referenced by live UI code', allowIn: ['schema.js'] },
  { pattern: /http:\/\/localhost:8085\/api\/events(?!\?schema=2)/, desc: 'unversioned schema-1 events endpoint' },
  { pattern: /\/api\/events(?!\?schema=2)/, desc: 'events endpoint without ?schema=2' },
];

// Heuristic "no client-side metric arithmetic": arithmetic applied directly to a server value (`x.v * y`).
// lib/format.js is the single sanctioned place where a server value may be scaled for display.
export const ARITHMETIC_PATTERN = /\.v\s*[*/+-]\s*[\w(]/;
export const ARITHMETIC_ALLOWED_FILES = ['format.js'];

export function listSourceFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listSourceFiles(full));
    else if (entry.isFile() && /\.(js|jsx|mjs)$/.test(entry.name)) out.push(full);
  }
  return out;
}

/** Scans every source file under srcDir. There are no exempt files (the legacy SimDemo exemption was removed in I3). */
export function scanSource(srcDir) {
  const violations = [];
  for (const file of listSourceFiles(srcDir)) {
    const base = path.basename(file);
    const lines = fs.readFileSync(file, 'utf8').split('\n');
    lines.forEach((line, idx) => {
      for (const { pattern, desc, allowIn } of FORBIDDEN_PATTERNS) {
        if (allowIn && allowIn.includes(base)) continue;
        if (pattern.test(line)) violations.push({ file: path.relative(srcDir, file), line: idx + 1, desc, matched: line.trim() });
      }
      if (!ARITHMETIC_ALLOWED_FILES.includes(base) && ARITHMETIC_PATTERN.test(line)) {
        violations.push({ file: path.relative(srcDir, file), line: idx + 1, desc: 'arithmetic on a server value outside lib/format.js', matched: line.trim() });
      }
    });
  }
  return violations;
}
