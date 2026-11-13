// Regression guards for the Simulation explainer (increment I3). These guard against regressions in wording and structure;
// they are NOT evidence of visual quality, accessibility in a browser, or layout at any width (see the I3 record for the browser pass).
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { makeCtx } from './helpers/fixtures.mjs';
import { parseRoute } from '../src/lib/routes.js';

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src');
let H;
let html;
before(async () => {
  H = await getHarness();
  html = H.renderScreen('simulation', makeCtx({}));
});

test('routes #/simulation and ?view=sim-demo still resolve to the simulation page', () => {
  assert.deepEqual(parseRoute({ hash: '#/simulation' }).id, 'simulation');
  assert.deepEqual(parseRoute({ search: '?view=sim-demo' }), { id: 'simulation', notFound: false });
});

test('the page renders with one h1, the SIMULATED badge and the simulated banner first', () => {
  assert.equal(html.split('<h1').length - 1, 1);
  assert.match(html, /data-provenance="simulated"/);
  assert.match(html, /prov__word">Simulated</);
  assert.ok(html.indexOf('data-banner="simulated"') >= 0 && html.indexOf('data-banner="simulated"') < html.indexOf('data-simulated="true"'));
  assert.match(html, /not measured from a gateway/);
});

test('no fabricated tokens: no currency, no CONNECTED literal, no fake status, counterfactual, canary or PIE panel, no digits in visible text', () => {
  const text = html.replace(/<svg[\s\S]*?<\/svg>/g, ' ').replace(/<[^>]+>/g, ' ');
  assert.doesNotMatch(text, /[$€£]|CONNECTED|COMMITTED|SEED:|SYSTEM_BOOT|\bcanary\b|counterfactual/);
  assert.doesNotMatch(text.replace(/\(T1\)/g, ''), /\d/, 'the explainer shows no numbers');
});

test('the page states what it does not do and points to Phantom Lab', () => {
  assert.match(html, /does not compare Phantom on and off/);
  for (const href of ['#/lab/run', '#/lab/evidence', '#/lab/runs']) assert.ok(html.includes(`href="${href}"`), href);
  assert.match(html, /Real<[^>]*>[\s\S]*loopback/);
});

test('no inline style attributes and no panel nested in a panel', () => {
  assert.doesNotMatch(html, /\sstyle="/);
  let depth = 0;
  for (const m of html.matchAll(/<section class="panel[^"]*"|<\/section>|<section(?![^>]*class="panel)/g)) {
    if (m[0].startsWith('<section class="panel')) { depth += 1; assert.ok(depth <= 1, 'nested panel'); } else if (m[0] === '</section>') depth = Math.max(0, depth - 1);
  }
  const src = fs.readFileSync(path.join(SRC, 'screens/Simulation.jsx'), 'utf8');
  assert.doesNotMatch(src, /style=\{/);
});

test('sim.css uses tokens only: no hex or rgb colour, no px/rem font size, no double border, no dashed or dotted', () => {
  const css = fs.readFileSync(path.join(SRC, 'styles/sim.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
  assert.doesNotMatch(css, /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/);
  assert.doesNotMatch(css, /font-size:\s*[0-9.]+(px|rem|em)/);
  assert.doesNotMatch(css, /\bdouble\b|\bdashed\b|\bdotted\b|backdrop-filter/);
});

test('the page has no dialog, no network call and no data hook', () => {
  const src = fs.readFileSync(path.join(SRC, 'screens/Simulation.jsx'), 'utf8');
  assert.doesNotMatch(src, /fetch\s*\(|EventSource|useTelemetry|role="dialog"|useState|useEffect/);
});

test('the explainer is lint-clean (heading order, names, ids, tables)', () => {
  assert.deepEqual(lintHtml(html), []);
});
