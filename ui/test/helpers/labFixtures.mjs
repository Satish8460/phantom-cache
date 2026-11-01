// Real Lab API responses captured from the gateway build named in fixtures/lab/errors.json (see docs/design/phantom_lab_ui.md).
// Test data only: lives under ui/test and never ships in the bundle.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../fixtures/lab');

export function readText(name) {
  return fs.readFileSync(path.join(DIR, name), 'utf8');
}

export function readJson(name) {
  return JSON.parse(readText(name));
}

export const FX = Object.freeze({
  caps: () => readJson('capabilities.json'),
  capsJwt: () => readJson('capabilities_jwt.json'),
  evidenceIndex: () => readJson('evidence_index.json'),
  m6Sealed: () => readJson('evidence_m6_sealed.json'),
  m6Dry: () => readJson('evidence_m6_dryrun.json'),
  sbEvidence: () => readJson('evidence_scenario_b.json'),
  runsList: () => readJson('runs_list.json'),
  runsEmpty: () => readJson('runs_empty.json'),
  runT1: () => readJson('run_t1_completed.json'),
  runSbRunning: () => readJson('run_sb_running.json'),
  runSbFinal: () => readJson('run_sb_final.json'),
  runCancelled: () => readJson('run_cancelled.json'),
  artifactsT1: () => readJson('artifacts_t1.json'),
  artifactsSb: () => readJson('artifacts_sb.json'),
  sbSummary: () => readJson('sb_summary_artifact.json'),
  errors: () => readJson('errors.json'),
  sseT1: () => readText('events_t1.sse'),
  sseSb: () => readText('events_sb.sse'),
  sseCancelled: () => readText('events_cancelled.sse'),
});

/** A fake Response good enough for the client: status, headers, text(), blob(). */
export function fakeResponse({ status = 200, headers = {}, body = '', stream = null } = {}) {
  const h = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: { get: (k) => h.get(String(k).toLowerCase()) ?? null },
    text: async () => (typeof body === 'string' ? body : JSON.stringify(body)),
    blob: async () => ({ text: async () => (typeof body === 'string' ? body : JSON.stringify(body)), size: String(body).length }),
    body: stream,
  };
}

/** A ReadableStream-like body that yields the given chunks (strings or Uint8Arrays), then ends or errors. */
export function fakeStream(chunks, { errorAfter = null } = {}) {
  let i = 0;
  let cancelled = false;
  return {
    getReader() {
      return {
        async read() {
          if (cancelled) return { done: true, value: undefined };
          if (i < chunks.length) {
            const value = chunks[i];
            i += 1;
            return { done: false, value };
          }
          if (errorAfter) throw new Error(errorAfter);
          return { done: true, value: undefined };
        },
        async cancel() {
          cancelled = true;
        },
      };
    },
  };
}

/** Split text into pseudo-random-looking but deterministic chunk sizes, to exercise chunk boundaries. */
export function chunkText(text, sizes) {
  const out = [];
  let pos = 0;
  let k = 0;
  while (pos < text.length) {
    const n = sizes[k % sizes.length];
    out.push(text.slice(pos, pos + n));
    pos += n;
    k += 1;
  }
  return out;
}
