// Server-side render harness: bundles the UI with esbuild (already present as a Vite dependency, nothing added)
// into the OS temp directory, outside the repository, then loads it.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const here = path.dirname(fileURLToPath(import.meta.url));
let cached = null;

export async function getHarness() {
  if (cached) return cached;
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-ui-harness-'));
  const outfile = path.join(outDir, 'harness.cjs');
  await build({
    entryPoints: [path.join(here, 'harness-entry.jsx')],
    outfile,
    bundle: true,
    platform: 'node',
    format: 'cjs',
    jsx: 'automatic',
    loader: { '.css': 'empty', '.js': 'jsx' },
    define: { 'import.meta.env': '{}', 'import.meta': '{}', 'process.env.NODE_ENV': '"production"' },
    logLevel: 'error',
  });
  const require = createRequire(import.meta.url);
  cached = require(outfile);
  process.on('exit', () => {
    try {
      fs.rmSync(outDir, { recursive: true, force: true });
    } catch {
      /* best effort */
    }
  });
  return cached;
}
