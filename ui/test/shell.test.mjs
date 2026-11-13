// Increment I4: shell, navigation and the non-Lab pages. REGRESSION GUARDS on structure, text and stylesheet rules.
// They do not show that the pages look good; that is judged by the screenshots and human review (docs/design/remediation/increment_I4_shell_pages.md).
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { SCREENS, LAB_VIEWS, footerFor, labHref, parseRoute, titleFor } from '../src/lib/routes.js';
import { THEME_OPTIONS, THEMES } from '../src/lib/theme.js';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';

const UI = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => fs.readFileSync(path.join(UI, p), 'utf8');
const shellCss = read('src/styles/shell.css');
const pagesCss = read('src/styles/pages.css');
const appCss = read('src/styles/app.css');

let H;
before(async () => {
  H = await getHarness();
});

// ------------------------------------------------------------------ routes, titles, footers

test('nav entries: Simulation (not "Simulation demo"), groups named for contents, tags in text', () => {
  const sim = SCREENS.find((s) => s.id === 'simulation');
  assert.equal(sim.label, 'Simulation');
  assert.equal(sim.group, 'Explainer');
  assert.equal(sim.tag, 'Simulated');
  assert.deepEqual([...new Set(SCREENS.map((s) => s.group))], ['Start', 'Live gateway', 'Experiments', 'Explainer', 'Not available yet']);
  for (const s of SCREENS.filter((x) => x.kind === 'placeholder')) assert.equal(s.tag, 'Not available');
  assert.doesNotMatch(read('src/lib/routes.js') + read('src/components/Shell.jsx'), /Simulation demo/);
});

test('LAB_VIEWS order and labels match the Lab tab strip; deep links still parse', () => {
  assert.deepEqual(LAB_VIEWS.map((v) => [v.id, v.label]), [['story', 'About'], ['run', 'Start a run'], ['runs', 'Runs'], ['evidence', 'Evidence']]);
  const tabs = read('src/screens/lab/LabScreen.jsx');
  for (const v of LAB_VIEWS) assert.ok(tabs.includes(`{ id: '${v.id}', label: '${v.label}' }`), `LabScreen tab ${v.id}/${v.label}`);
  assert.deepEqual(parseRoute({ hash: '#/lab/run' }).sub, { view: 'run', id: null });
  assert.deepEqual(parseRoute({ hash: '#/lab/runs' }).sub, { view: 'runs', id: null });
  assert.deepEqual(parseRoute({ hash: '#/lab/evidence' }).sub, { view: 'evidence', id: null });
  assert.deepEqual(parseRoute({ hash: '#/lab/runs/20261030T034041Z-c78a73cf' }).sub, { view: 'runs', id: '20261030T034041Z-c78a73cf' });
  assert.deepEqual(parseRoute({ hash: '#/lab/evidence/m6_sealed_x' }).sub, { view: 'evidence', id: 'm6_sealed_x' });
  assert.equal(labHref('run'), '#/lab/run');
});

test('document.title: distinct for every route and Lab sub-route, most specific first', () => {
  const titles = SCREENS.map((s) => titleFor({ id: s.id }));
  assert.equal(new Set(titles).size, titles.length);
  assert.equal(titleFor({ id: 'lab' }), 'Phantom Lab | PhantomCache');
  assert.equal(titleFor({ id: 'overview' }), 'Overview | PhantomCache');
  assert.equal(titleFor({ id: 'traffic' }), 'Traffic & latency | PhantomCache');
  assert.equal(titleFor({ id: 'lab', sub: { view: 'run', id: null } }), 'Start a run | Phantom Lab | PhantomCache');
  assert.equal(titleFor({ id: 'lab', sub: { view: 'runs', id: 'r1' } }), 'Run r1 | Runs | Phantom Lab | PhantomCache');
  assert.equal(titleFor({ id: 'lab', sub: { view: 'evidence', id: 'm6' } }), 'Evidence m6 | Evidence | Phantom Lab | PhantomCache');
  assert.equal(titleFor({ id: 'nope' }), 'Overview | PhantomCache', 'unknown routes fall back to the overview title');
  assert.match(read('src/App.jsx'), /document\.title = title/);
});

test('footer text is true for the kind of page: the telemetry sentence is only on telemetry pages', () => {
  const live = footerFor({ id: 'traffic' });
  assert.match(live, /Telemetry schema 2/);
  for (const id of ['overview', 'traffic', 'cache', 'freshness', 'origin']) assert.equal(footerFor({ id }), live);
  const lab = footerFor({ id: 'lab' });
  const sim = footerFor({ id: 'simulation' });
  const ph = footerFor({ id: 'intelligence' });
  assert.equal(footerFor({ id: 'cluster' }), live, 'Cluster is a telemetry page since batch B2');
  assert.equal(new Set([live, lab, sim, ph]).size, 4);
  for (const text of [lab, sim, ph]) assert.doesNotMatch(text, /schema 2|computes no metrics/);
  assert.match(lab, /Lab API/);
  assert.match(sim, /model output/);
  assert.match(read('src/App.jsx'), /\{footerFor\(route\)\}/);
});

// ------------------------------------------------------------------ theme control

test('theme control: Light / System / Dark, one pressed, persisted values unchanged', () => {
  assert.deepEqual(THEME_OPTIONS.map((o) => o.id), ['light', 'system', 'dark']);
  assert.deepEqual([...THEMES].sort(), THEME_OPTIONS.map((o) => o.id).sort(), 'same value set as the stored preference');
  for (const theme of ['light', 'system', 'dark']) {
    const html = H.renderShell({ connKind: 'live', currentId: 'overview', theme });
    // two controls exist (header, shown from 820 px; menu, shown below 820 px); each has exactly one pressed button
    const groups = html.match(/<div class="seg theme-control"[\s\S]*?<\/div>/g);
    assert.equal(groups.length, 2);
    for (const g of groups) {
      assert.equal((g.match(/aria-pressed="true"/g) || []).length, 1);
      assert.match(g, new RegExp(`data-theme-option="${theme}"[^>]*>|aria-pressed="true"[^>]*data-theme-option="${theme}"`));
      assert.deepEqual([...g.matchAll(/>(Light|System|Dark)<\/button>/g)].map((m) => m[1]), ['Light', 'System', 'Dark']);
    }
  }
  const html = H.renderShell({ connKind: 'live', currentId: 'overview', theme: 'dark' });
  assert.doesNotMatch(html, /Theme: dark|Theme: system/, 'the single cycling button is gone');
});

// ------------------------------------------------------------------ shell structure

test('shell: Menu button lives in the header and controls the nav list; groups are named nested lists', () => {
  const html = H.renderShell({ connKind: 'live', currentId: 'traffic' });
  assert.match(html, /<button type="button" class="btn app-nav__toggle" id="app-menu-button" aria-expanded="false" aria-controls="app-nav-list">Menu<\/button>/);
  assert.ok(html.indexOf('app-nav__toggle') < html.indexOf('<nav'), 'the Menu button is in the header, before the nav');
  assert.match(html, /<ul id="app-nav-list" class="app-nav__groups">/);
  for (const g of ['Start', 'Live gateway', 'Experiments', 'Explainer', 'Not available yet']) assert.match(html, new RegExp(`class="app-nav__group-label" id="app-nav-group-\\d">${g}<`));
  assert.equal((html.match(/<ul aria-labelledby="app-nav-group-\d">/g) || []).length, 5);
  const open = H.renderShell({ connKind: 'live', currentId: 'traffic', menuOpen: true });
  assert.match(open, /aria-expanded="true"/);
  assert.match(open, /class="app-nav is-open"/);
  assert.deepEqual(lintHtml(open, { requireH1: false }), []);
});

test('shell: the header is one compact row (brand, status chip, theme, Menu) and no longer carries the gateway URL', () => {
  const html = H.renderShell({ connKind: 'disconnected', currentId: 'overview', mode: 'live' });
  const header = /<header[\s\S]*?<\/header>/.exec(html)[0];
  assert.match(header, /Disconnected/);
  assert.doesNotMatch(header, /localhost:8085|Mode:/);
  const foot = /<div class="app-nav__foot">[\s\S]*<\/nav>/.exec(html)[0];
  assert.match(foot, /localhost:8085/);
  assert.match(foot, /Reports mode: live/);
});

test('app: skip link is the first focusable element and targets a focusable main', () => {
  const app = read('src/App.jsx');
  assert.ok(app.indexOf('className="skip-link"') < app.indexOf('<Header'), 'skip link before the header');
  assert.match(app, /<main id="main"[^>]*tabIndex=\{-1\}/);
  assert.match(app, /e\.preventDefault\(\);\s*mainRef\.current\?\.focus\(\)/, 'the skip link moves focus without writing #main into the hash router');
  assert.match(appCss, /\.disclosure > summary, \.app-main a/, 'disclosure summaries reach the touch size on phones');
  assert.match(app, /e\.key === 'Escape' && menuOpen/);
  assert.match(app, /document\.getElementById\(MENU_BUTTON_ID\)\?\.focus\(\)/, 'Escape returns focus to the Menu button');
});

test('shell.css: navigation from 820 px, Menu hidden by a rule more specific than .btn, sticky stretched side column, reduced motion', () => {
  assert.doesNotMatch(shellCss, /960px/);
  assert.match(shellCss, /@media \(min-width: 820px\) \{[\s\S]*\.app-header \.app-nav__toggle \{ display: none; \}/);
  assert.match(shellCss, /grid-template-columns: var\(--nav-width\) minmax\(0, 1fr\)/);
  assert.match(shellCss, /\.app-nav, \.app-nav\.is-open \{[^}]*align-self: stretch/);
  // DD v2: sticky glass header (top: 0), so the sticky rail is offset below it and fills the remaining viewport height; own scroll kept.
  assert.match(shellCss, /\.app-nav__panel \{ position: sticky; top: var\(--header-h\); height: calc\(100vh - var\(--header-h\)\); overflow-y: auto;/);
  assert.match(shellCss, /\.app \{[^}]*grid-template-columns: minmax\(0, 1fr\)/);
  assert.match(shellCss, /\.app > \* \{ min-width: 0; \}/);
  assert.match(shellCss, /@media \(prefers-reduced-motion: reduce\) \{[^@]*\.app-nav\.is-open \{ animation-name: pc-fade; \}/);
  assert.match(shellCss, /min-height: 3\.5rem/, 'phone header is at most 56 px tall');
  assert.match(shellCss, /\.app-nav a:active \{ background-image: linear-gradient\(var\(--tint-press\)/, 'press feedback is a tint (S1)');
  assert.match(shellCss, /\.app-nav a\[aria-current="page"\] \{[^}]*font-weight: var\(--weight-semibold\)[^}]*box-shadow: inset 3px 0 0/, 'current page is not colour alone');
  assert.match(shellCss, /@import '\.\/pages\.css';/);
  assert.match(read('src/App.jsx'), /import '\.\/styles\/shell\.css';/);
});

test('app.css: touch breakpoint follows the nav breakpoint; .choice control size lives here; .sim-frame is gone', () => {
  assert.match(appCss, /@media \(max-width: 819px\), \(pointer: coarse\) \{/);
  assert.doesNotMatch(appCss, /959px/);
  assert.match(appCss, /\.choice input\[type="checkbox"\], \.choice input\[type="radio"\] \{ width: 1\.5rem; height: 1\.5rem;/);
  assert.doesNotMatch(appCss, /sim-frame/);
  for (const f of fs.readdirSync(path.join(UI, 'src'), { recursive: true }).filter((x) => /\.(jsx?|css)$/.test(x))) {
    if (f.endsWith('app.css')) continue;
    assert.doesNotMatch(read(path.join('src', f)), /sim-frame/, `${f} must not reference .sim-frame`);
  }
  assert.match(pagesCss, /\.capacity__track/);
});

// ------------------------------------------------------------------ vocabulary

test('provenance vocabulary: the words "Inferred" and "Measured" are not used as display labels in the pages and shell', () => {
  for (const f of ['Overview', 'Traffic', 'CacheMemory', 'Freshness', 'Origin', 'Placeholder', 'ScreenFrame']) {
    const src = read(`src/screens/${f}.jsx`);
    for (const m of src.matchAll(/>[^<>{}]*\b(Inferred|Measured|INFERRED|MEASURED)\b[^<>{}]*</g)) assert.fail(`${f}: ${m[0]}`);
    for (const m of src.matchAll(/(note|title|intro)="[^"]*\b(Inferred|Measured)\b/g)) assert.fail(`${f}: ${m[0]}`);
  }
  const overview = H.renderScreen('overview', makeCtx({ frames: [livePayload()] }));
  assert.match(overview, /Items labelled Derived, inferred/);
  assert.doesNotMatch(overview, /Items marked Inferred/);
});

// ------------------------------------------------------------------ pages

test('overview: status line, key signals grouped by meaning, one definition list for connection and capabilities, other fields behind a disclosure', () => {
  const html = H.renderScreen('overview', makeCtx({ frames: [livePayload()] }));
  assert.match(html, /data-status-line="live"/);
  assert.match(html, /Receiving data from the gateway\./); // V2B: the sentence now comes from linkState (retired text: "Receiving telemetry.")
  for (const g of ['Traffic', 'Freshness guard', 'Memory and origin']) assert.match(html, new RegExp(`<h2[^>]*>${g}</h2>`));
  assert.equal((html.match(/<h2/g) || []).length, 4, 'Traffic, Freshness guard, Memory and origin, Gateway');
  assert.doesNotMatch(html, /class="panel/, 'no boxed panels on the overview (nothing nested in a card)');
  assert.match(html, /<dl class="kv kv--wide"/);
  for (const cap of ['cluster', 'controller', 'origin_metrics']) assert.match(html, new RegExp(`data-capability="${cap}"`));
  assert.equal((html.match(/<dl class="kv/g) || []).length, 1, 'capabilities and connection are one list');
  assert.match(html, /<details class="disclosure"><summary>Other reported fields \(\d+\)<\/summary>/);
  assert.equal((html.match(/data-metric="/g) || []).length, 6);
});

test('overview: values the gateway does not report are listed once, with a reason and an UNAVAILABLE label, never as zero tiles', () => {
  const p = livePayload();
  p.metrics.hit_ratio_10s = { src: 'unavailable', reason: 'insufficient_window', unit: 'ratio' };
  p.metrics.origin.in_flight = { src: 'unavailable', reason: 'not_measured' };
  const html = H.renderScreen('overview', makeCtx({ frames: [p] }));
  assert.match(html, /<h2[^>]*>Not reported \(2\)<\/h2>/);
  const list = /<ul class="not-reported"[\s\S]*?<\/ul>/.exec(html)[0];
  assert.equal((list.match(/<li>/g) || []).length, 2);
  assert.equal((list.match(/data-provenance="unavailable"/g) || []).length, 2);
  assert.match(list, /Hit ratio, 10 s/);
  assert.equal((html.match(/data-metric="/g) || []).length, 4, 'only reported values are tiles');
  assert.deepEqual(lintHtml(html, { requireH1: true }), []);
});

test('detail pages: sections instead of boxed panels, a group caption above the title, states unchanged', () => {
  for (const [id, group] of [['traffic', 'Live gateway'], ['cache', 'Live gateway'], ['freshness', 'Live gateway'], ['origin', 'Live gateway'], ['cluster', 'Live gateway'], ['intelligence', 'Not available yet']]) {
    const html = H.renderScreen(id, makeCtx({ frames: [livePayload()] }));
    assert.match(html, new RegExp(`<p class="page-head__group">${group}</p><h1`), id);
    assert.doesNotMatch(html, /class="panel"/, `${id}: no nested cards`);
    assert.match(html, /class="section"/, id);
  }
});

test('traffic: latency in a ChartFrame with a table; all-unavailable percentiles collapse to one row; flat series say so', () => {
  const ok = H.renderScreen('traffic', makeCtx({ frames: [livePayload()] }));
  assert.equal((ok.match(/class="chart-frame"/g) || []).length, 4, 'percentiles plus three time series');
  assert.match(ok, /GET latency percentiles/);
  const p = livePayload();
  p.metrics.latency_ms = { src: 'unavailable', reason: 'no_samples', unit: 'ms' };
  const none = H.renderScreen('traffic', makeCtx({ frames: [p] }));
  assert.equal((none.match(/data-bars-empty/g) || []).length, 1);
  assert.doesNotMatch(none, /class="bars"/, 'no empty tracks');
  assert.match(none, /Unavailable: p50, p90, p95, p99 are not reported yet/);
});

test('cache: capacity chart draws the scale ends and a table alternative; unreported items are a list with the UNAVAILABLE label', () => {
  const html = H.renderScreen('cache', makeCtx({ frames: [livePayload()] }));
  assert.match(html, /class="capacity__track"/);
  assert.match(html, /<div class="capacity__ends" aria-hidden="true"><span>0<\/span><span>8\.0 MiB<\/span><\/div>/);
  assert.match(html, /Resident bytes against budget/);
  assert.match(html, /<th scope="row">Byte budget<\/th>/);
  const empty = H.renderChart('capacity', { title: 'Cap', used: NaN, total: NaN, usedText: null, totalText: null, src: 'unavailable' });
  assert.match(empty, /Bar not drawn/);
  assert.match(empty, /budget unknown/);
  const list = /<ul class="not-reported"[\s\S]*?<\/ul>/.exec(html)[0];
  assert.match(list, /Entry count/);
  assert.match(list, /data-provenance="unavailable"/);
});

test('placeholders: no developer sentence, say what would show and what exists instead, link to the real Lab, no numbers', () => {
  for (const id of ['cluster', 'intelligence']) {
    const html = H.renderScreen(id, makeCtx({ frames: [livePayload()] }));
    assert.doesNotMatch(html, /section for this screen/);
    assert.match(html, /What this page would show/);
    assert.match(html, /What exists today instead/);
    assert.match(html, /href="#\/lab\/evidence"/);
    assert.match(html, /href="#\/overview"/);
    assert.match(html, /Backend dependency: a telemetry contract/);
    assert.equal((html.match(/data-metric=/g) || []).length, 0);
    assert.deepEqual(lintHtml(html), [], id);
  }
});

// ------------------------------------------------------------------ lint

test('lint knows the roles the dialogs and progress bars use', () => {
  assert.deepEqual(lintHtml('<h1>t</h1><div role="dialog" aria-label="x"></div><div role="alertdialog" aria-label="y"></div><div role="progressbar" aria-label="z"></div>'), []);
  assert.ok(lintHtml('<h1>t</h1><div role="slider"></div>').some((i) => i.rule === 'role'));
});

test('harness exports RichNotice for tests that need it', () => {
  assert.equal(typeof H.Lab.RichNotice, 'function');
});
