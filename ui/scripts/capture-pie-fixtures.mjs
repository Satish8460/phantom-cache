// Captures the REAL responses of the Phantom Intelligence API into fixture files, so the contract-shaped fixtures can be replaced.
// Usage: node scripts/capture-pie-fixtures.mjs <gateway-origin> <tokens.json> <out-dir> [tokenName=tenantA]
// The token is read from the file and sent only as the Authorization header; it is never printed or written.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const [gw, tokensFile, outDir, name = 'tenantA'] = process.argv.slice(2);
if (!gw || !tokensFile || !outDir) {
  console.error('usage: node scripts/capture-pie-fixtures.mjs <gateway-origin> <tokens.json> <out-dir> [tokenName]');
  process.exit(2);
}
const token = JSON.parse(readFileSync(tokensFile, 'utf8'))[name];
mkdirSync(outDir, { recursive: true });
const targets = {
  'status.real.json': '/api/v1/intelligence/status',
  'registry.real.json': '/api/v1/intelligence/registry',
  'signals.real.json': '/api/v1/intelligence/signals',
  'decisions.real.json': '/api/v1/intelligence/decisions?limit=20&offset=0',
};
for (const [file, path] of Object.entries(targets)) {
  const res = await fetch(`${gw}${path}`, { headers: { Authorization: `Bearer ${token}`, Accept: 'application/json' } });
  const text = await res.text();
  console.log(`${res.status} ${path} -> ${file}`);
  if (res.ok) writeFileSync(`${outDir}/${file}`, text.endsWith('\n') ? text : `${text}\n`);
}
