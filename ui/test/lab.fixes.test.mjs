// Regression tests for the findings of the first real-browser verification (F-01 .. F-11).
// Static and server-render checks only: the lead re-checks each item in a browser.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { capabilitiesView, evidenceDetailView, runView } from '../src/lib/labView.js';
import { initialRunModel, initialStream, streamReducer } from '../src/lib/labStream.js';
import { mapLabError } from '../src/lib/labApi.js';
import { roleCapabilities, tokenRoles } from '../src/lib/labToken.js';
import { FX } from './helpers/labFixtures.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => fs.readFileSync(path.join(ROOT, p), 'utf8');
const appCss = read('src/styles/app.css');
const labCss = read('src/styles/lab.css');
// I4: the shell moved from app.css to shell.css (header, nav, main grid) and pages.css; the guards below follow the rules to their new home.
const shellCss = read('src/styles/shell.css');
const pagesCss = read('src/styles/pages.css');
const allCss = `${appCss}\n${shellCss}\n${pagesCss}\n${labCss}`;

let H;
let h;
before(async () => {
  H = await getHarness();
  h = H.React.createElement;
});
const noop = () => {};

function jwt(claims) {
  const b = (o) => Buffer.from(JSON.stringify(o)).toString('base64url');
  return `${b({ alg: 'HS256', typ: 'JWT' })}.${b(claims)}.sig`;
}

// ---------------------------------------------------------------- F-01 reflow

test('F-01: grid and flex roots can shrink; long text wraps; wide content scrolls inside its own box', () => {
  // I4: the grid rules now live in shell.css (same intent: the single column must not size to content; the desktop column is minmax(0, 1fr)).
  assert.match(shellCss, /\.app \{[^}]*grid-template-columns: minmax\(0, 1fr\)/, 'single-column app grid must not size to content');
  assert.match(shellCss, /grid-template-columns: var\(--nav-width\) minmax\(0, 1fr\)/, 'desktop grid column must be minmax(0, 1fr)');
  assert.match(shellCss, /\.app > \* \{ min-width: 0; \}/);
  // .app-header__meta no longer exists (the header lost its meta row). .app-main is in shell.css, the rest stay in app.css.
  for (const sel of ['.panel', '.banner__body', '.state-block']) {
    assert.ok(new RegExp(`${sel.replace('.', '\\.')}[,\\s][^{]*\\{[^}]*min-width: 0`).test(appCss), `${sel} needs min-width: 0`);
  }
  assert.match(shellCss, /\.app-main \{[^}]*min-width: 0/, '.app-main needs min-width: 0');
  assert.match(shellCss, /\.app-header \{[^}]*flex-wrap: wrap/, 'the header wraps instead of widening the page');
  assert.match(appCss, /\.mono[^{]*\{[^}]*overflow-wrap: anywhere/, 'ids, hashes and URLs wrap');
  assert.match(appCss, /pre \{[^}]*overflow: auto/);
  assert.match(appCss, /\.table-wrap \{[^}]*overflow-x: auto/);
  assert.match(labCss, /\.lab-verbatim \{[^}]*overflow: auto/);
  // I2: the run form is a single column (.lab-form); these Lab containers must be able to shrink.
  assert.match(labCss, /\.lab-card, \.lab-field, \.lab-form[^{]*\{[^}]*min-width: 0/);
});

test('F-01: no fixed pixel widths that can exceed a 320 px viewport, and no hidden page-level scroll workaround', () => {
  for (const m of allCss.matchAll(/(?<![-\w(])(min-width|width):\s*(\d+(?:\.\d+)?)px/g)) {
    assert.ok(Number(m[2]) <= 44, `fixed ${m[1]}: ${m[2]}px could overflow a narrow viewport`);
  }
  for (const m of allCss.matchAll(/(?<![-\w(])(min-width|width):\s*(\d+(?:\.\d+)?)rem/g)) {
    if (m[1] === 'min-width') assert.ok(Number(m[2]) <= 12, `min-width: ${m[2]}rem is too wide for 320 px`);
  }
  assert.doesNotMatch(allCss, /(html|body)[^{]*\{[^}]*overflow-x:\s*(hidden|clip)/, 'overflow must be fixed at the cause, not hidden on the page');
  assert.doesNotMatch(`${appCss}\n${shellCss}`, /\.app-nav[^{]*\{[^}]*overflow-x/, 'the nav is no longer a hidden horizontal scroller');
  // minmax(Nrem, ...) columns must be able to collapse: every fixed minimum is inside min(100%, ...) or <= 11rem
  for (const m of labCss.matchAll(/repeat\(auto-(?:fit|fill), minmax\((?!min\(100%)(\d+(?:\.\d+)?)rem/g)) assert.ok(Number(m[1]) <= 11, `auto-fit minimum ${m[1]}rem`);
  assert.match(labCss, /\.lab-table-note|\.lab-body \.table-wrap[^{]*\{[^}]*max-height: none/, 'lab tables are not cut off mid-row');
});

// ---------------------------------------------------------------- F-02 mobile nav

// I4: the breakpoint moved from 960 to 820 px, the Menu button moved into the header and the open state into App (so Escape can return focus to it).
test('F-02: below the tablet breakpoint the nav list sits behind a Menu disclosure button; at 820 px and up it is always shown', () => {
  assert.match(shellCss, /\.app-nav \{[^}]*display: none/, 'closed by default on phones');
  assert.match(shellCss, /\.app-nav\.is-open \{ display: block/, 'opened by the Menu button');
  assert.match(shellCss, /@media \(min-width: 820px\) \{[\s\S]*\.app-header \.app-nav__toggle \{ display: none; \}/, 'hide rule is more specific than .btn');
  const html = H.renderShell({ connKind: 'live', currentId: 'lab' });
  assert.match(html, /<button type="button" class="btn app-nav__toggle" id="app-menu-button" aria-expanded="false" aria-controls="([^"]+)">Menu<\/button>/);
  const id = /aria-controls="([^"]+)"/.exec(html)[1];
  assert.ok(html.includes(`<ul id="${id}"`), 'aria-controls points at the list');
  assert.deepEqual(lintHtml(html, { requireH1: false }), []);
  const app = read('src/App.jsx');
  assert.match(app, /e\.key === 'Escape' && menuOpen/, 'Escape closes the menu');
  assert.match(app, /document\.getElementById\(MENU_BUTTON_ID\)\?\.focus\(\)/, 'and returns focus to the button');
  assert.match(app, /onNavigate=\{\(\) => setMenuOpen\(false\)\}/, 'choosing a link closes it');
});

// ---------------------------------------------------------------- F-03 targets

test('F-03: every interactive target is >= 32 px on desktop and >= 44 px on narrow or coarse screens (24 px floor everywhere)', () => {
  const tk = read('src/styles/tokens.css');
  assert.match(tk, /--target-touch: 44px/);
  assert.match(tk, /--target-desktop: 32px/);
  // I4: navigation links are sized in shell.css (44 px in the phone menu, 32 px in the side column); the shared rule keeps the other controls.
  assert.match(appCss, /\.btn, summary, details > summary, \.app-main a, \.app-footer a \{ min-height: var\(--target-desktop\); \}/);
  assert.match(shellCss, /\.app-nav a \{[^}]*min-height: var\(--target-touch\)/);
  assert.match(shellCss, /@media \(min-width: 820px\) \{[\s\S]*\.app-nav a \{ min-height: var\(--target-desktop\); \}/);
  assert.match(appCss, /\.app-main a, \.app-footer a \{ display: inline-flex; align-items: center; \}/, 'inline links get a real box');
  assert.match(appCss, /@media \(max-width: 819px\), \(pointer: coarse\) \{[^}]*min-height: var\(--target-touch\)/);
  // I2: Lab tabs and choice rows use the shared .tab and .choice classes (app.css owns their targets); Lab-only links are sized in lab.css.
  assert.match(appCss, /\.tab \{[^}]*min-height: var\(--target-desktop\)/);
  assert.match(appCss, /\.choice \{[^}]*min-height: var\(--target-desktop\)/);
  assert.match(appCss, /\.tab, \.seg__btn, \.choice, input\[type="text"\], input\[type="password"\] \{ min-height: var\(--target-touch\); \}/);
  assert.match(labCss, /\.lab-toc a, \.lab-status__active \{ min-height: var\(--target-desktop\); \}/);
  assert.match(labCss, /\.lab-toc a, \.lab-status__active \{ min-height: var\(--target-touch\); \}/);
  // no rule may set a smaller interactive minimum than the 24 px floor
  assert.doesNotMatch(allCss, /(a|button|summary|\.btn)[^{]*\{[^}]*min-height:\s*(1[0-9]|2[0-3])px/);
});

// ---------------------------------------------------------------- F-04 / F-05 nav icon and sticky

test('F-04/F-05: nav icons are sized, and the side navigation is sticky with its own scroll', () => {
  // I4: the nav cell stretches the full row height (its surface reaches the bottom); the panel inside it is the sticky, self-scrolling part.
  assert.match(shellCss, /\.app-nav a svg \{ width: 1rem; height: 1rem; flex: none; \}/);
  // DD v2: the header is sticky too, so the rail sticks BELOW it (top: var(--header-h)) and fills the rest of the viewport; same intent (sticky, own scroll).
  assert.match(shellCss, /\.app-nav__panel \{ position: sticky; top: var\(--header-h\); height: calc\(100vh - var\(--header-h\)\); overflow-y: auto;/);
  const sticky = shellCss.indexOf('position: sticky; top: var(--header-h)');
  assert.ok(sticky > shellCss.indexOf('@media (min-width: 820px)'), 'sticky only applies to the side navigation (>= 820 px)');
});

// ---------------------------------------------------------------- F-06 focus

test('F-06: focus is moved to the heading only on in-app route changes, never on load (also under StrictMode double effects)', () => {
  const app = read('src/App.jsx');
  assert.doesNotMatch(app, /firstRoute/);
  assert.match(app, /shownKey/);
  assert.match(app, /previous === null \|\| previous === routeKey\) return;/);
  assert.match(app, /window\.scrollTo\(0, 0\);\s*\n\s*mainRef\.current\?\.querySelector\('\[data-screen-heading\]'\)\?\.focus\(\)/);
  // simulate the effect body: StrictMode runs it twice on load with the same key
  const shown = { current: null };
  const focused = [];
  const effect = (key) => { const p = shown.current; shown.current = key; if (p === null || p === key) return; focused.push(key); };
  effect('lab'); effect('lab');
  assert.deepEqual(focused, [], 'no focus move on load');
  effect('lab/runs/x');
  assert.deepEqual(focused, ['lab/runs/x']);
});

// ---------------------------------------------------------------- F-07 escape

test('F-07: the cancel confirmation closes on Escape and focus returns to the Cancel button', () => {
  const src = read('src/screens/lab/RunsViews.jsx');
  // I2: the confirmation is now a modal dialog (L3). Escape closes it unless the request is being sent; the caller returns focus
  // to the Cancel button, or to the run heading when that button is gone (cancel requested).
  assert.match(src, /el\.showModal\(\)/, 'a native modal dialog: the browser traps focus and makes the page inert');
  assert.match(src, /onCancel=\{\(e\) => \{ e\.preventDefault\(\); if \(!sending\) onAbort\(\); \}\}/, 'Escape (the dialog cancel event) closes it unless the request is being sent');
  assert.match(src, /cancelRef\.current && cancelRef\.current\.isConnected \? cancelRef\.current : headingRef\.current/);
  assert.match(src, /keepRef\.current\?\.focus\(\)/, 'focus enters the confirmation on the safe choice');
  assert.doesNotMatch(src, /role="alertdialog"/, 'the native dialog element carries the role');
});

// ---------------------------------------------------------------- F-08 honest chip

function detail(run, sse, status, stream) {
  return h(H.Lab.RunDetailBody, {
    run: runView(run), live: { status, stream, model: initialRunModel }, artifacts: { state: 'loading', rows: [] }, summary: { state: 'none' },
    cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop,
    downloading: null, downloadError: null,
  });
}
const withEvent = (n) => streamReducer(streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 }), { type: 'frame', id: n, now: 0 });

test('F-08: while the stream is down the state chip is qualified ("last known ... last event N"); live and terminal runs are not', () => {
  for (const status of ['reconnecting', 'disconnected', 'failed', 'stale']) {
    const html = H.renderLab(detail(FX.runSbRunning(), null, status, withEvent(5)));
    assert.match(html, new RegExp(`data-state-qualified="true"[^>]*>.*Running \\(last known; stream ${status}, last event 5\\)`), status);
    assert.match(html, new RegExp(`data-stream-status="${status}"`), 'the banner stays');
  }
  const live = H.renderLab(detail(FX.runSbRunning(), null, 'live', withEvent(5)));
  assert.doesNotMatch(live, /data-state-qualified/);
  const done = H.renderLab(detail(FX.runSbFinal(), null, 'disconnected', withEvent(5)));
  assert.doesNotMatch(done, /data-state-qualified/, 'a persisted terminal state is not "last known"');
  const none = H.renderLab(detail(FX.runSbRunning(), null, 'disconnected', streamReducer(initialStream, { type: 'connect' })));
  assert.match(none, /stream disconnected, no event received/);
});

// ---------------------------------------------------------------- F-10 favicon

test('F-10: an inline favicon exists and nothing in index.html loads an external resource', () => {
  const html = read('index.html');
  assert.match(html, /<link rel="icon" href="data:image\/svg\+xml,/);
  assert.doesNotMatch(html, /(href|src)="https?:/);
});

// ---------------------------------------------------------------- F-11 chart axis and tables

test('F-11: the effect chart has no absolutely positioned axis captions (they overlapped); the axis caption is one full-width sentence in the chart frame', () => {
  // I2: the caption moved into ChartFrame (.chart-frame__axes, width 100%), a block sibling of the plot, so it can no longer be squeezed into a narrow first column.
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) }));
  assert.doesNotMatch(html, /lab-effect__axis-zero|lab-effect__axis"|lab-effect__axis-text|data-effect-axis/);
  assert.equal(html.split('class="chart-frame__axes"').length - 1, 2, 'one axis caption per effect chart');
  assert.match(html, /The axis runs from -[0-9.]+% to \+?[0-9.]+%/);
  assert.match(appCss, /\.chart-frame__axes \{[^}]*width: 100%/);
  assert.doesNotMatch(labCss, /lab-effect__axis-zero/);
  assert.doesNotMatch(labCss, /\.lab-effect__axis \{/);
});

test('every chart has a Chart / Table toggle; the table alternative is always in the DOM (the inactive pane is hidden) and has a caption', () => {
  // I2: replaces the closed "View as table" details. Same tables, now one click away on a visible toggle.
  const pages = [
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) })),
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) })),
  ];
  for (const html of pages) {
    const figs = html.split('<figure').slice(1);
    assert.ok(figs.length >= 2);
    for (const f of figs) {
      const block = f.split('</figure>')[0];
      assert.match(block, /class="seg" role="group" aria-label="[^"]+: view"/, 'a visible toggle');
      assert.match(block, />Chart<\/button>/);
      assert.match(block, />Table<\/button>/);
      assert.match(block, /<div class="chart-frame__pane" hidden=""><div class="table-wrap"><table><caption>/, 'the table pane is in the DOM, hidden by default, with a caption');
    }
  }
});

// ---------------------------------------------------------------- F-09 roles

test('F-09: roles are read from the token for display only; unreadable tokens show nothing', () => {
  assert.deepEqual(tokenRoles(jwt({ roles: ['telemetry:read'] })), ['telemetry:read']);
  assert.deepEqual(tokenRoles(jwt({ roles: ['observer', 5, 'admin'] })), ['observer', 'admin']);
  for (const bad of ['', 'abc', 'a.b.c', jwt({}), jwt({ roles: 'admin' }), null, undefined]) assert.equal(tokenRoles(bad), null);
  assert.deepEqual(roleCapabilities(['telemetry:read']), { canRead: true, canOperate: false });
  assert.deepEqual(roleCapabilities(['Operator']), { canRead: false, canOperate: true });
  assert.deepEqual(roleCapabilities(['admin']), { canRead: true, canOperate: true });
  assert.equal(roleCapabilities(null), null);
  const src = read('src/lib/labToken.js');
  assert.doesNotMatch(src, /localStorage|sessionStorage|console\./);
});

test('F-09: the Start form warns BEFORE submitting when the token has no Operator role, and stays silent when roles are unknown or sufficient', () => {
  const caps = capabilitiesView(FX.capsJwt());
  const form = (roles) => H.renderLab(h(H.Lab.RunFormBody, { caps, roles, onSubmit: noop, initialKind: 'scenario_b' }));
  const obs = form(['telemetry:read']);
  assert.match(obs, /This token cannot start runs/);
  assert.match(obs, /telemetry:read/);
  assert.match(obs, /403/);
  assert.doesNotMatch(form(['scenario:operate']), /cannot start runs/);
  assert.doesNotMatch(form(['admin']), /cannot start runs/);
  assert.doesNotMatch(form(null), /cannot start runs/);
  assert.match(form([]), /\(none\)/);
  assert.deepEqual(lintHtml(obs, { requireH1: false }), []);
});

test('F-09: the 403-on-POST path shows the role message (recorded gateway response)', () => {
  const e = FX.errors().forbidden_observer_post;
  const error = mapLabError({ status: e.status, headers: e.headers, bodyJson: null, bodyText: e.body, hadToken: true });
  assert.equal(error.kind, 'forbidden_role');
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps: capabilitiesView(FX.capsJwt()), onSubmit: noop, submitError: error }));
  assert.match(html, /The token does not have the required role/);
  assert.match(html, /Starting and cancelling runs needs Operator/);
  assert.match(html, /role="alert"/);
});

test('connection panel shows the roles read from the token and says they are not verified', () => {
  const view = capabilitiesView(FX.capsJwt());
  const html = H.renderLab(h(H.Lab.ConnectionPanel, { res: { status: 'ready', data: FX.capsJwt(), error: null, reload: noop }, view, hasToken: true, roles: ['telemetry:read'], onToken: noop, onClear: noop, gatewayUrl: 'x' }));
  assert.match(html, /Roles read from it \(not verified\): telemetry:read/);
});

// ---------------------------------------------------------------- run class wording

test('"VALID" is never shown as a bare success label: it reads "Run class: VALID (narrow meaning)" with an inline explanation', () => {
  const detailHtml = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  assert.match(detailHtml, /Run class: VALID \(narrow meaning\)/);
  assert.doesNotMatch(detailHtml, /Label: VALID/);
  assert.match(detailHtml, /not a success verdict/);
});
