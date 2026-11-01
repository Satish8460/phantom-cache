// Server-side render harness for the V2B tests (same approach as scripts/render-harness.mjs: esbuild is already a Vite dependency).
// Bundles test/helpers/app2-entry.jsx into the OS temp directory (outside the repository) and loads it.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const here = path.dirname(fileURLToPath(import.meta.url));
let cached = null;

export async function getApp2Harness() {
  if (cached) return cached;
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-ui-app2-'));
  const outfile = path.join(outDir, 'app2.cjs');
  await build({
    entryPoints: [path.join(here, 'app2-entry.jsx')],
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

/** A telemetry context value built by driving the real reducer; `now` is the clock the link is described at. */
export async function makeLinkCtx({ frames = [], actions = [], now = 1_000_000, receivedAt = now - 100 } = {}) {
  const { connectionReducer, deriveConnection, initialConnection } = await import('../../src/lib/connection.js');
  const { describeTelemetryLink } = await import('../../src/lib/linkState.js');
  let state = initialConnection;
  for (const f of frames) state = connectionReducer(state, { type: 'message', text: typeof f === 'string' ? f : JSON.stringify(f), now: receivedAt });
  for (const a of actions) state = connectionReducer(state, a);
  const conn = deriveConnection(state, now);
  const empty = [];
  return {
    payload: state.payload,
    state,
    conn,
    now,
    link: describeTelemetryLink(state, now),
    series: { throughput: empty, hitRatio: empty, p99: empty },
    gateway: { url: 'http://localhost:8085', source: 'default', sourceText: 'built-in default', hostPort: 'localhost:8085', notes: [] },
    reconnect: () => {},
    openConnection: () => {},
    offsetMs: null,
  };
}
