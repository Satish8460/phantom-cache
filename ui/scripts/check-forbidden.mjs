import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { scanSource } from './forbidden-patterns.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC_DIR = path.resolve(__dirname, '../src');

console.log('[check-forbidden] Scanning ui/src for forbidden synthetic constants...');
const violations = scanSource(SRC_DIR);

if (violations.length > 0) {
  console.error(`[check-forbidden] FAILED: Found ${violations.length} violation(s):`);
  for (const v of violations) console.error(`  - ${v.file}:${v.line} [${v.desc}]: "${v.matched}"`);
  process.exit(1);
}
console.log('[check-forbidden] PASSED: Zero forbidden synthetic constants found in live UI code.');
process.exit(0);
