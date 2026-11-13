import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { build } from 'esbuild';
import { readField, subField, reasonText, flattenProvenanceFields, isKnownSrc, PROVENANCE_ORDER } from '../src/lib/provenance.js';

const here = path.dirname(fileURLToPath(import.meta.url));

test('provenance classes are exactly the five in the schema', () => {
  assert.deepEqual([...PROVENANCE_ORDER].sort(), ['derived', 'inferred', 'measured', 'simulated', 'unavailable']);
  assert.equal(isKnownSrc('estimated'), false);
  assert.equal(isKnownSrc(undefined), false);
});

test('measured and derived numbers are values', () => {
  assert.deepEqual(readField({ src: 'measured', v: 5, unit: 'count' }), { status: 'value', src: 'measured', v: 5, unit: 'count' });
  assert.equal(readField({ src: 'derived', v: 0, unit: 'ratio' }).status, 'value'); // a real zero stays a value
  assert.equal(readField({ src: 'derived', v: 0, unit: 'ratio' }).v, 0);
});

test('unavailable never yields a value and keeps its reason', () => {
  const r = readField({ src: 'unavailable', reason: 'unsupported_platform', unit: 'bytes' });
  assert.equal(r.status, 'unavailable');
  assert.equal(r.reason, 'unsupported_platform');
  assert.equal('v' in r, false);
  assert.equal(readField({ src: 'unavailable' }).reason, 'unspecified');
});

test('unavailable carrying a value is rejected, not shown as 0', () => {
  const r = readField({ src: 'unavailable', v: 0, reason: 'x' });
  assert.equal(r.status, 'rejected');
  assert.equal(r.reason, 'unavailable_with_value');
  assert.equal('v' in r, false);
});

test('simulated is rejected in live mode and accepted in simulation mode', () => {
  assert.equal(readField({ src: 'simulated', v: 1 }, { mode: 'live' }).status, 'rejected');
  assert.equal(readField({ src: 'simulated', v: 1 }, { mode: 'live' }).reason, 'simulated_in_live');
  const sim = readField({ src: 'simulated', v: 1 }, { mode: 'simulation' });
  assert.equal(sim.status, 'value');
  assert.equal(sim.src, 'simulated');
});

test('missing, malformed and unknown fields never become numbers', () => {
  for (const bad of [undefined, null, 5, 'x', [], {}]) {
    const r = readField(bad);
    assert.notEqual(r.status, 'value', JSON.stringify(bad));
    assert.equal('v' in r, false);
  }
  assert.equal(readField(undefined).status, 'missing');
  assert.equal(readField({ src: 'guessed', v: 3 }).reason, 'unknown_provenance');
  assert.equal(readField({ src: 'measured' }).reason, 'missing_value');
  assert.equal(readField({ src: 'measured', v: '5' }).reason, 'non_numeric_value');
  assert.equal(readField({ src: 'measured', v: Number.NaN }).reason, 'non_numeric_value');
  assert.equal(readField({ src: 'measured', v: null }).reason, 'non_numeric_value');
});

test('booleans are accepted as flag values; strings and objects are not', () => {
  const flag = readField({ src: 'measured', v: false });
  assert.equal(flag.status, 'value');
  assert.equal(flag.v, false, 'false is a real value, not an absence');
  assert.equal(readField({ src: 'measured', v: 'yes' }).status, 'rejected');
  assert.equal(readField({ src: 'measured', v: { a: 1 } }).status, 'rejected');
});

test('subField inherits provenance and respects unavailable parents', () => {
  const lat = { src: 'measured', unit: 'ms', p99: 7, samples: 3 };
  assert.deepEqual(subField(lat, 'p99'), { src: 'measured', v: 7, unit: 'ms' });
  assert.equal(readField(subField(lat, 'p50')).status, 'unavailable'); // key absent
  const gone = { src: 'unavailable', reason: 'no_samples', unit: 'ms', v: 9 };
  assert.equal(readField(subField(gone, 'p99')).status, 'rejected'); // a stray v is not laundered into a value
  assert.equal(subField(undefined, 'p99'), undefined);
});

test('reason codes have text; unknown codes are shown verbatim, not hidden', () => {
  assert.match(reasonText('unsupported_platform'), /platform/i);
  assert.equal(reasonText('some_new_code'), 'Reason code: some_new_code');
  assert.equal(reasonText(undefined), reasonText('unspecified'));
});

test('flattenProvenanceFields finds leaves at any depth and stops at a src', () => {
  const out = flattenProvenanceFields({ a: { src: 'measured', v: 1 }, b: { c: { src: 'derived', v: 2 } }, d: 5 });
  assert.deepEqual(out.map((o) => o.path).sort(), ['a', 'b.c']);
});

// ---- Badge display vocabulary (I1, spec section 11). Renders the real component through esbuild (already a Vite dependency). ----
// Regression guards for the display words and DOM hooks; they say nothing about how the badge looks.
let badgeMod;
async function loadBadge() {
  if (badgeMod) return badgeMod;
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-badge-'));
  const outfile = path.join(outDir, 'badge.cjs');
  await build({
    stdin: {
      contents: `import React from 'react'; import { renderToStaticMarkup } from 'react-dom/server';
        import B, { DISPLAY, RESERVED_SRC } from './ProvenanceBadge.jsx';
        export const render = (props) => renderToStaticMarkup(React.createElement(B, props)); export { DISPLAY, RESERVED_SRC };`,
      resolveDir: path.resolve(here, '../src/components'),
      loader: 'jsx',
    },
    outfile, bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', logLevel: 'error',
    loader: { '.js': 'jsx' }, define: { 'process.env.NODE_ENV': '"production"' },
  });
  badgeMod = createRequire(import.meta.url)(outfile);
  process.on('exit', () => { try { fs.rmSync(outDir, { recursive: true, force: true }); } catch { /* best effort */ } });
  return badgeMod;
}

test('badge: MEASURED is displayed as REAL and INFERRED as DERIVED with the qualifier "inferred"; DOM hooks keep the telemetry class', async () => {
  const { render } = await loadBadge();
  const real = render({ src: 'measured' });
  assert.match(real, /class="prov prov--measured"/);
  assert.match(real, /data-provenance="measured"/);
  assert.match(real, />Real</);
  assert.doesNotMatch(real, />Measured</);
  const inferred = render({ src: 'inferred' });
  assert.match(inferred, /data-provenance="inferred"/);
  assert.match(inferred, />Derived</);
  assert.match(inferred, /class="prov__q">inferred</);
  assert.match(render({ src: 'measured', qualifier: 'loopback · single host' }), /class="prov__q">loopback · single host</);
  assert.match(render({ src: 'simulated' }), />Simulated</);
  assert.match(render({ src: 'unavailable' }), />Unavailable</);
});

test('badge: a caller label replaces the word (tier wording such as "Measured: loopback") and tier is kept in data-tier', async () => {
  const { render } = await loadBadge();
  const html = render({ src: 'measured', label: 'Measured: loopback', tier: 'MEASURED_LOOPBACK' });
  assert.match(html, /Measured: loopback/);
  assert.match(html, /data-tier="MEASURED_LOOPBACK"/);
  assert.doesNotMatch(html, /prov__q/);
});

test('badge: REPLAYED and PREDICTED are reserved: exported, drawn with their own class, glyph and wording only when passed; unknown still degrades', async () => {
  const { render, DISPLAY, RESERVED_SRC } = await loadBadge();
  assert.deepEqual([...RESERVED_SRC].sort(), ['predicted', 'replayed']);
  for (const k of RESERVED_SRC) {
    const html = render({ src: k });
    assert.match(html, new RegExp(`class="prov prov--${k}"`));
    assert.match(html, new RegExp(`data-provenance="${k}"`));
    assert.match(html, /<svg/);
    assert.match(html, new RegExp(`>${DISPLAY[k].word}<`));
    assert.equal(isKnownSrc(k), false, 'the telemetry model must not accept reserved classes');
    assert.equal(readField({ src: k, v: 1 }).status, 'rejected', 'no payload can produce a reserved badge today');
  }
  assert.match(render({ src: 'estimated' }), /data-provenance="unavailable"/);
  assert.deepEqual(Object.keys(DISPLAY).sort(), ['derived', 'inferred', 'measured', 'predicted', 'replayed', 'simulated', 'unavailable']);
});
