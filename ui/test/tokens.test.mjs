// Foundation regression guards for the design system (spec: docs/design/phantomcache_design_system_spec.md, increment I1).
//
// IMPORTANT: these are STATIC checks over CSS text and server-rendered markup. They catch regressions of specific rules
// (a tier removed, a duplicated palette, a dashed border outside a badge, a missing reduced-motion rule). They are NOT
// evidence of visual quality, hierarchy, spacing, legibility or accessibility in a browser; those need human review of
// screenshots and the browser rounds. A green run here means "the listed rules still hold", nothing more.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { build } from 'esbuild';

const here = path.dirname(fileURLToPath(import.meta.url));
const stylesDir = path.resolve(here, '../src/styles');
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '');
const files = fs.readdirSync(stylesDir).filter((f) => f.endsWith('.css'));
const css = Object.fromEntries(files.map((f) => [f, strip(fs.readFileSync(path.join(stylesDir, f), 'utf8'))]));
const tokens = css['tokens.css'];
const app = css['app.css'];
const allCss = Object.values(css).join('\n');

// Files owned by LATER increments may still hold old patterns; this list is an upper bound, so the test fails when a NEW
// violation appears and passes (without edits) when the owner removes an old one.
const PENDING_I2_DASHED = ['.btn:disabled', '.lab-tag', '.lab-effect__zero', '.lab-unavail-panel'];

test('type tiers exist with size, leading, tracking and weight; sizes in rem; no size below 12 px', () => {
  for (const t of ['display', 'headline', 'title', 'body', 'dense', 'caption', 'metric', 'mono']) {
    for (const part of ['', '-lh', '-ls', '-wt']) assert.match(tokens, new RegExp(`--type-${t}${part}:`), `--type-${t}${part}`);
    assert.match(tokens, new RegExp(`--type-${t}:\\s*[0-9.]+rem;`), `${t} size is rem`);
  }
  const sizes = Object.fromEntries([...tokens.matchAll(/--type-([a-z]+):\s*([0-9.]+)rem;/g)].map((m) => [m[1], Number(m[2])]));
  for (const [k, v] of Object.entries(sizes)) assert.ok(v >= 0.75, `${k} ${v}rem >= 12 px`);
  const ls = {};
  for (const m of tokens.matchAll(/--type-([a-z]+)-ls:\s*(-?[0-9.]+)(em)?;/g)) ls[m[1]] = Number(m[2]);
  for (const k of ['display', 'headline', 'title', 'metric']) assert.ok(ls[k] < 0, `${k} tracking is negative`);
  assert.equal(ls.body, 0, 'body tracking is 0');
  assert.ok(ls.caption > 0, 'caption tracking is positive');
  assert.ok(sizes.display > sizes.headline && sizes.headline > sizes.title && sizes.title > sizes.body && sizes.body > sizes.dense && sizes.dense > sizes.caption);
  assert.match(tokens, /@media \(max-width: 30rem\)\s*\{\s*:root\s*\{\s*--type-display:\s*1\.5rem/, 'display steps down on phones');
  assert.match(app, /\.metric__value\s*\{[^}]*tabular-nums/);
  assert.match(app, /body\s*\{[^}]*tabular-nums/);
});

test('deprecated --text-* aliases point at the tiers; --text-lg is gone', () => {
  for (const [old, tier] of [['xs', 'caption'], ['sm', 'dense'], ['md', 'body'], ['xl', 'headline'], ['metric', 'metric']]) {
    assert.match(tokens, new RegExp(`--text-${old}:\\s*var\\(--type-${tier}\\);`), old);
  }
  assert.doesNotMatch(tokens, /--text-lg\b/);
  assert.doesNotMatch(allCss, /var\(--text-lg\)/);
});

test('motion tokens are the specified values; one-shot motion uses tokens (literal durations only for the two loops)', () => {
  for (const [k, v] of [['press', 100], ['fast', 150], ['base', 200], ['slow', 300]]) assert.match(tokens, new RegExp(`--dur-${k}:\\s*${v}ms;`), k);
  for (const [f, text] of Object.entries(css)) {
    if (f === 'tokens.css') continue;
    for (const m of text.matchAll(/(?:animation|transition)(?:-duration)?:([^;}]*)/g)) {
      for (const d of m[1].matchAll(/(?<![\w.-])([0-9.]+)(ms|s)\b/g)) {
        const ms = d[2] === 's' ? Number(d[1]) * 1000 : Number(d[1]);
        assert.ok(/infinite/.test(m[1]) && ms <= 1500, `${f}: literal duration in "${m[0]}" (only spinner / indeterminate loops may use one)`);
      }
    }
  }
});

test('single-source theme: each colour token is declared once with light-dark(); no duplicated dark palette; data-theme only sets color-scheme', () => {
  for (const t of ['bg', 'surface', 'surface-2', 'border', 'border-strong', 'text', 'text-2', 'text-3', 'accent', 'focus', 'ok', 'warn', 'critical', 'info', 'prov-measured', 'prov-derived', 'prov-simulated', 'prov-replayed', 'prov-predicted', 'prov-unavailable', 'chart-line']) {
    const n = [...tokens.matchAll(new RegExp(`--${t}:`, 'g'))].length;
    if (t === 'border') assert.ok(n <= 2, '--border: one palette declaration plus the prefers-contrast override');
    else assert.equal(n, 1, `--${t} declared ${n} times`);
    assert.match(tokens, new RegExp(`--${t}:\\s*light-dark\\(`), `--${t} uses light-dark()`);
  }
  assert.match(tokens, /:root\s*\{\s*color-scheme:\s*light dark;/);
  assert.match(tokens, /:root\[data-theme="light"\]\s*\{\s*color-scheme:\s*light;\s*\}/);
  assert.match(tokens, /:root\[data-theme="dark"\]\s*\{\s*color-scheme:\s*dark;\s*\}/);
  assert.doesNotMatch(tokens, /@media \(prefers-color-scheme/, 'the OS preference comes from color-scheme, not a second block');
});

test('elevation: --shadow-1..3 exist and are used by panels, menus, dialog and sticky surfaces; no other shadow literals', () => {
  for (const n of [1, 2, 3]) assert.match(tokens, new RegExp(`--shadow-${n}:`));
  assert.match(app, /\.panel\s*\{[^}]*box-shadow:\s*var\(--shadow-1\)/);
  assert.match(app, /\.menu[^{]*\{[^}]*box-shadow:\s*var\(--shadow-2\)/);
  assert.match(app, /\.sticky-bar\s*\{[^}]*var\(--shadow-2\)/);
  assert.match(app, /\.form-bar\s*\{[^}]*var\(--shadow-2\)/);
  assert.match(app, /\.dialog\s*\{[^}]*box-shadow:\s*var\(--shadow-3\)/);
  for (const t of ['surface-raised', 'surface-overlay', 'scrim']) assert.match(tokens, new RegExp(`--${t}:`));
  for (const [f, text] of Object.entries(css)) {
    if (f === 'tokens.css') continue;
    for (const m of text.matchAll(/box-shadow:\s*([^;}]+)/g)) {
      assert.ok(/^(var\(--shadow-[123]\)|none|inset [0-9]+px 0 0 var\(--[a-z-]+\))$/.test(m[1].trim()), `${f}: box-shadow ${m[1]} must be a --shadow token (or an inset rule marker)`);
    }
  }
  assert.match(tokens, /prefers-contrast: more[\s\S]*--shadow-1:\s*none/);
});

test('press feedback on pointer-down: buttons scale 0.97 in 100 ms; rows, tabs, nav and list items use a tint; reduced motion removes the scale', () => {
  assert.match(app, /\.btn:active\s*\{[^}]*transform:\s*scale\(0\.97\)/);
  assert.match(app, /\.btn\s*\{[^}]*transform var\(--dur-press\)/);
  // I4: .app-nav a:active moved to shell.css with the rest of the shell; the tint-not-scale rule is unchanged.
  for (const sel of ['tbody tr:active', '.tab:active', '.app-nav a:active', '.seg__btn:active', '.choice:active']) {
    const m = `${app}
${css["shell.css"]}`.match(new RegExp(`${sel.replace(/[.[\]]/g, '\\$&')}\\s*\\{([^}]*)\\}`));
    assert.ok(m, `${sel} present`);
    assert.ok(/--tint-press/.test(m[1]) && !/scale\(/.test(m[1]), `${sel} uses the tint, not scale`);
  }
  assert.match(app, /:focus-visible\s*\{[^}]*outline:\s*var\(--focus-width\) solid var\(--focus\)/);
  assert.match(app, /\.btn:disabled,\s*\.btn\[aria-disabled="true"\]/);
  assert.match(app, /\.btn\[aria-busy="true"\]/);
  assert.match(app, /@media \(prefers-reduced-motion: reduce\)\s*\{[^@]*\.btn:active[^}]*transform:\s*none/);
});

test('reduced motion: movement becomes an opacity cross-fade; loops become static', () => {
  const block = app.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}\n/);
  assert.ok(block, 'reduced-motion block exists');
  assert.match(block[1], /animation-name:\s*pc-fade/);
  assert.match(block[1], /\.spinner svg[^{]*\{[^}]*animation:\s*none/);
  assert.match(block[1], /\.progress-indeterminate::before[^{]*\{[^}]*animation:\s*none/);
  const fade = app.match(/@keyframes pc-fade\s*\{[^\n]*\}/)[0];
  assert.match(fade, /opacity/);
  assert.doesNotMatch(fade, /transform/);
});

test('dashed and dotted borders appear only in provenance badge selectors (pending-I2 leftovers in lab.css are bounded)', () => {
  for (const [f, text] of Object.entries(css)) {
    for (const m of text.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      if (!/(?:border|outline)[a-z-]*:[^;]*\b(?:dashed|dotted|double)\b|border-style:\s*(?:dashed|dotted|double)/.test(m[2])) continue;
      const sel = m[1].trim();
      if (f === 'app.css' || f === 'tokens.css') {
        assert.ok(/^\.prov--(simulated|replayed|predicted|unavailable)$/.test(sel), `${f}: dashed/dotted/double outside a badge selector: ${sel}`);
      } else {
        assert.ok(PENDING_I2_DASHED.some((p) => sel.includes(p)), `${f}: NEW dashed/dotted/double border outside the badge: ${sel}`);
      }
    }
  }
  assert.doesNotMatch(allCss, /(?:border|outline)[a-z-]*:\s*[^;]*\bdouble\b/, 'no double borders anywhere');
  assert.doesNotMatch(app, /\.prov--inferred\s*\{[^}]*dashed/, 'inferred is shown as DERIVED: solid');
});

// DD v2 (owner-approved 2026-10-30): glass is allowed on application CHROME only. The old blanket ban is replaced by the same intent, narrowed:
// backdrop-filter exists only in app.css (the Materials block), and never in lab.css, pages.css, sim.css, shell.css or tokens.css (data surfaces).
// The full guard (selector allow-list, @supports, fallback ladder, phone budget, contrast floor) is in tokens.materials.test.mjs.
test('glass only on chrome: backdrop-filter is confined to the guarded app.css chrome block, never in data-surface stylesheets', () => {
  for (const f of ['lab.css', 'pages.css', 'sim.css', 'shell.css', 'tokens.css']) assert.doesNotMatch(css[f], /backdrop-filter/i, f);
  const supports = app.indexOf('@supports ((backdrop-filter');
  assert.ok(supports > 0, 'the glass block is behind @supports');
  assert.ok(app.search(/backdrop-filter\s*:\s*blur/) > supports, 'no glass before the @supports guard');
  assert.doesNotMatch(app, /\.(panel|lab-card|metric|state-block|banner|chart-frame|table-wrap)\b[^{]*\{[^}]*backdrop-filter/, 'no glass on a data surface');
});

test('no font-size literal below 12 px in the styles', () => {
  for (const [f, text] of Object.entries(css)) {
    for (const m of text.matchAll(/font-size:\s*([0-9.]+)(px|rem)/g)) {
      const px = m[2] === 'px' ? Number(m[1]) : Number(m[1]) * 16;
      assert.ok(px >= 12, `${f}: ${m[0]}`);
    }
  }
});

test('every defined token is referenced somewhere (CSS or components) or is on the reserved allowlist', () => {
  const srcDir = path.resolve(here, '../src');
  const body = [allCss.replace(tokens, ''), ...fs.readdirSync(srcDir, { recursive: true }).filter((f) => /\.(jsx?|html)$/.test(f)).map((f) => fs.readFileSync(path.join(srcDir, f), 'utf8'))].join('\n');
  const RESERVED = new Set(['type-mono-lh', 'type-mono-ls', 'type-mono-wt', 'type-body-wt', 'type-dense-wt', 'dur-slow', 'space-6', 'prov-inferred', 'prov-inferred-bg', 'text-md', 'text-xs', 'text-sm', 'text-xl', 'text-metric', 'leading-tight', 'leading-body', 'weight-regular', 'radius-lg', 'surface-overlay']);
  const tokenSelf = tokens;
  const defined = [...new Set([...tokens.matchAll(/(--[a-z0-9-]+):/g)].map((m) => m[1]))];
  const unused = defined.filter((t) => !RESERVED.has(t.slice(2)) && !body.includes(`var(${t}`) && !new RegExp(`var\\(${t}[,)]`).test(tokenSelf));
  assert.deepEqual(unused, [], `unused tokens: ${unused.join(', ')}`);
});

test('forced colours keep the badge border style (colour only changes)', () => {
  const fc = app.match(/@media \(forced-colors: active\)\s*\{([\s\S]*?)\n\}\n/)[1];
  assert.match(fc, /\.prov\s*\{\s*border-color:\s*CanvasText;/);
  assert.doesNotMatch(fc, /\.prov[^{]*\{[^}]*border:\s*1px solid/);
});

// ---- Rendered foundation components (server-side markup; still only structure, not appearance) ----
let C;
async function load() {
  if (C) return C;
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-found-'));
  const outfile = path.join(outDir, 'found.cjs');
  await build({
    stdin: {
      contents: `import React from 'react'; import { renderToStaticMarkup as r } from 'react-dom/server';
        import ChartFrame from './ChartFrame.jsx'; import Panel from './Panel.jsx';
        import { StateBlock, Spinner, DisabledReason, StaleTag, InlineError } from './States.jsx';
        export const h = (el) => r(el); export { React, ChartFrame, Panel, StateBlock, Spinner, DisabledReason, StaleTag, InlineError };`,
      resolveDir: path.resolve(here, '../src/components'), loader: 'jsx',
    },
    outfile, bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', logLevel: 'error', loader: { '.js': 'jsx' }, define: { 'process.env.NODE_ENV': '"production"' },
  });
  C = createRequire(import.meta.url)(outfile);
  process.on('exit', () => { try { fs.rmSync(outDir, { recursive: true, force: true }); } catch { /* best effort */ } });
  return C;
}

test('ChartFrame: title, provenance badge, full-width axis captions with units, and a table alternative that is always in the DOM', async () => {
  const { h, React, ChartFrame } = await load();
  const html = h(React.createElement(ChartFrame, { title: 'Latency', src: 'measured', qualifier: 'loopback', unitLabel: 'ms', xAxis: 'run time (s)', yAxis: 'p99', chart: React.createElement('svg'), table: React.createElement('table', null, React.createElement('caption', null, 'Latency values')) }));
  assert.match(html, /<figure class="chart-frame"/);
  assert.match(html, /data-provenance="measured"/);
  assert.match(html, /class="chart-frame__axes"/);
  assert.match(html, /run time \(s\)/);
  assert.match(html, /aria-pressed="true"[^>]*>Chart</);
  assert.match(html, /aria-pressed="false"[^>]*>Table</);
  assert.match(html, /<div class="chart-frame__pane" hidden="">[^]*<table>/, 'table pane exists, hidden until chosen');
  const noTable = h(React.createElement(ChartFrame, { title: 'x', src: 'derived', chart: React.createElement('svg') }));
  assert.doesNotMatch(noTable, /class="seg"/, 'no toggle without an alternative (callers must supply one)');
});

test('state grammar: every kind has a glyph and words; errors are alerts; loading is aria-busy; reason and stale tags carry text', async () => {
  const { h, React, StateBlock, Spinner, DisabledReason, StaleTag, InlineError } = await load();
  for (const kind of ['loading', 'empty', 'error', 'success', 'warning', 'disabled', 'stale', 'unavailable', 'simulated', 'disconnected', 'notEnabled']) {
    const html = h(React.createElement(StateBlock, { kind, title: `T-${kind}` }, 'why'));
    assert.match(html, /<svg/, kind);
    assert.match(html, new RegExp(`T-${kind}`));
    assert.match(html, new RegExp(`data-state-kind="${kind}"`));
  }
  assert.match(h(React.createElement(StateBlock, { kind: 'error', title: 'x' })), /role="alert"/);
  assert.match(h(React.createElement(StateBlock, { kind: 'loading', title: 'x' })), /aria-busy="true"/);
  assert.match(h(React.createElement(StateBlock, { kind: 'simulated', title: 'x' })), /data-provenance="simulated"/);
  assert.match(h(React.createElement(StateBlock, { kind: 'error', title: 'x', detail: 'E123' })), /<details class="disclosure">[^]*E123/);
  assert.match(h(React.createElement(Spinner)), />Loading</);
  assert.match(h(React.createElement(DisabledReason, { id: 'r' }, 'Pick a scenario first')), /id="r"[^]*Pick a scenario first/);
  assert.match(h(React.createElement(StaleTag, { ageText: '12 s ago' })), /stale, 12 s ago/);
  assert.match(h(React.createElement(InlineError, { id: 'e' }, 'Seed is required')), /<svg[^]*Seed is required/);
});

test('Panel: one h2 title by default, optional heading level, plain variant', async () => {
  const { h, React, Panel } = await load();
  assert.match(h(React.createElement(Panel, { title: 'A' }, 'x')), /<section class="panel" aria-labelledby="[^"]+"><div class="panel__head"><h2 id="[^"]+" class="panel__title">A</);
  assert.match(h(React.createElement(Panel, { title: 'A', level: 3 }, 'x')), /<h3 /);
  assert.match(h(React.createElement(Panel, { title: 'A', plain: true }, 'x')), /class="panel panel--plain"/);
});
