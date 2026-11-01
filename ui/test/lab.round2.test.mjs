// Regression tests for browser round 2 findings N-1a, N-1b, N-2, N-3, N-4, N-5. Static CSS/source checks plus server
// rendering; the lead re-checks layout in a real browser.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { capabilitiesView } from '../src/lib/labView.js';
import { FX } from './helpers/labFixtures.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => fs.readFileSync(path.join(ROOT, p), 'utf8');
const appCss = read('src/styles/app.css');
const labCss = read('src/styles/lab.css');
const rule = (css, selector) => {
  const i = css.indexOf(selector);
  assert.ok(i >= 0, `missing rule ${selector}`);
  return css.slice(i, css.indexOf('}', i));
};

let H;
let h;
before(async () => {
  H = await getHarness();
  h = H.React.createElement;
});
const noop = () => {};

test('N-1a: .table-wrap is a containing block so absolute .visually-hidden cell text cannot widen the page', () => {
  assert.match(rule(appCss, '.visually-hidden {'), /position: absolute/);
  assert.match(rule(appCss, '.table-wrap { max-width: 100%'), /position: relative/);
  assert.match(rule(appCss, '.table-wrap { overflow-x: auto'), /overflow-x: auto/);
});

test('N-1b: banner text (for example a long debug-build path) wraps anywhere', () => {
  assert.match(rule(appCss, '.banner__body, .banner__body *'), /overflow-wrap: anywhere/);
  const html = H.renderLab(h(H.Lab.ConnectionPanel, { res: { status: 'ready', data: FX.capsJwt(), error: null, reload: noop }, view: { ...capabilitiesView(FX.capsJwt()), debugBuild: true, sb: { ...capabilitiesView(FX.capsJwt()).sb, labelReasons: ['D:\Personal\_m7_target\debug\phantom-cache-demo.exe'] } }, hasToken: true, roles: ['admin'], onToken: noop, onClear: noop, gatewayUrl: 'x' }));
  assert.match(html, /phantom-cache-demo\.exe/);
  // I2: the debug-build reasons (with the binary path) now live inside the status strip Details as a paragraph; every <p> wraps anywhere.
  assert.match(html, /data-debug-explainer/);
  assert.match(rule(appCss, '.mono, p, dd, dt'), /overflow-wrap: anywhere/);
});

test('N-3: checkbox and radio controls are 24 px or more and the whole label is the target; summaries match other controls', () => {
  // I2: the controls sit in the shared full-row .choice label (app.css owns its min-height and cursor); lab.css sizes the control itself.
  assert.match(rule(appCss, '.choice input[type="checkbox"], .choice input[type="radio"] {'), /width: 1\.5rem; height: 1\.5rem/);
  const check = rule(appCss, '.choice {');
  assert.match(check, /min-height: var\(--target-desktop\)/);
  assert.match(check, /cursor: pointer/);
  // `details > summary { min-height: 24px }` is more specific than a bare `summary` rule, so the target rules must name it.
  // I4 (lead repoint): the desktop target rule in app.css names summary and details > summary explicitly (more specific than a bare summary rule).
  assert.ok(appCss.includes('.btn, summary, details > summary, .app-main a, .app-footer a { min-height: var(--target-desktop); }'));
  assert.ok(appCss.includes('@media (max-width: 819px), (pointer: coarse)') && appCss.includes('.btn, summary, details > summary, .disclosure > summary'));
  assert.match(rule(appCss, 'details > summary { display: flex'), /align-items: center/);
});

test('N-4: the effect chart axis text spans the full row instead of the 6.5rem label column', () => {
  // I2: the caption is the ChartFrame axes line (width 100%, a block sibling of the plot); no list item carries it any more.
  assert.match(rule(appCss, '.chart-frame__axes {'), /width: 100%/);
  assert.doesNotMatch(labCss, /lab-effect__axis-text/);
});

test('N-5: pre-submit hint for an Operator-only token; read-forbidden run page explains it', () => {
  const caps = capabilitiesView(FX.capsJwt());
  const form = (roles) => H.renderLab(h(H.Lab.RunFormBody, { caps, roles, onSubmit: noop, initialKind: 'scenario_b' }));
  const op = form(['scenario:operate']);
  assert.match(op, /can start runs but cannot read them/);
  assert.match(op, /telemetry:read/);
  assert.doesNotMatch(op, /cannot start runs/);
  for (const roles of [['admin'], ['telemetry:read', 'scenario:operate'], null]) assert.doesNotMatch(form(roles), /can start runs but cannot read them/);

  const error = { kind: 'forbidden_role', title: 'The token does not have the required role', fix: 'Reading needs Observer.', serverMessage: '' };
  const page = (roles) => H.renderLab(h(H.Lab.ReadForbiddenBlock, { error, runId: 'RUN-1', roles, onRetry: noop }));
  const a = page(['scenario:operate']);
  assert.match(a, /can start runs but cannot read them/);
  assert.match(a, /RUN-1/);
  assert.match(a, /not confirmed here/);
  const b = page(['telemetry:read']);
  assert.doesNotMatch(b, /can start runs but cannot read them/);
  assert.match(b, /Reading runs needs the Observer role/);
  assert.match(page(null), /not known to this page/);
});

test('N-2: every live stream and request is aborted on unmount, pagehide and reload', () => {
  const tele = read('src/hooks/useTelemetry.js');
  assert.match(tele, /addEventListener\('pagehide'/);
  assert.match(tele, /removeEventListener\('pagehide'/);
  assert.match(tele, /es\.close\(\)/);
  const run = read('src/hooks/useRunStream.js');
  assert.match(run, /addEventListener\('pagehide', onPageHide\)/);
  assert.match(run, /removeEventListener\('pagehide', onPageHide\)/);
  assert.match(run, /ctl\.abort\(\)/);
  const res = read('src/hooks/useLabResource.js');
  assert.match(res, /ctlRef\.current\.abort\(\)/);
  // Exactly one EventSource constructor in the app shell hook and none in the Lab (Lab uses fetch streams with an abort signal).
  const lab = fs.readdirSync(path.join(ROOT, 'src/screens/lab')).map((f) => read(`src/screens/lab/${f}`)).join('\n');
  assert.doesNotMatch(lab, /EventSource/);
});
