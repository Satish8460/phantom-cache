// Loader for the F0 fixtures (ui/test/fixtures/func). Provenance: ui/test/fixtures/func/PROVENANCE.json.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

export const FUNC_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../fixtures/func');
export const readFuncText = (rel) => fs.readFileSync(path.join(FUNC_DIR, rel), 'utf8');
export const readFuncJson = (rel) => JSON.parse(readFuncText(rel));
export const sha256Of = (rel) => crypto.createHash('sha256').update(fs.readFileSync(path.join(FUNC_DIR, rel))).digest('hex');

export const FUNC = Object.freeze({
  legacyCancelled: () => ({ run: readFuncJson('legacy/cancelled.run.json'), summary: readFuncJson('legacy/cancelled.summary.json'), events: readFuncText('legacy/cancelled.events.jsonl') }),
  legacyCompleted: () => ({ run: readFuncJson('legacy/completed.run.json'), summary: readFuncJson('legacy/completed.summary.json'), events: readFuncText('legacy/completed.events.jsonl') }),
  sandboxCancelledZero: () => readFuncJson('sandbox_u01/cancelled_zero_runs.summary.json'),
  sandboxCancelledPartial: () => readFuncJson('sandbox_u01/cancelled_partial.summary.json'),
  sandboxCompleted: () => readFuncJson('sandbox_u01/completed.summary.json'),
});
