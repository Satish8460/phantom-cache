// V1 foundations v2: MATERIALS guards (docs/design/phantomcache_design_direction_v2.md sections 4.3, 12.4, 16 step 2).
// STATIC checks over CSS text. They hold the written rules (glass on chrome only, a fallback ladder, a contrast floor, radius cap) in place.
// They are NOT evidence of how the glass looks, and not a performance measurement; both need screenshots and a separate measured run.
// This file supersedes the single assertion "no backdrop-filter anywhere" in tokens.test.mjs (glass is now allowed on chrome); the lead
// should change that older assertion to point here.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const stylesDir = path.resolve(here, '../src/styles');
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '');
const css = Object.fromEntries(fs.readdirSync(stylesDir).filter((f) => f.endsWith('.css')).map((f) => [f, strip(fs.readFileSync(path.join(stylesDir, f), 'utf8'))]));

// The only selectors allowed to carry a backdrop-filter. Four glass surfaces on desktop: header, rail, run bar, dialog (DD 12.4: at most 4).
const CHROME = ['.glass-1', '.glass-2', '.app-header', '.app-nav__panel', '.form-bar', '.dialog'];
const DESKTOP_SURFACES = ['.app-header', '.app-nav__panel', '.form-bar', '.dialog'];
const DATA_SELECTORS = ['.panel', 'table', '.table-wrap', '.lab-card', '.metric', '.state-block', '.banner', '.chart-frame', '.spark', '.prov', '.tag', '.capacity', 'thead', 'td', 'th'];

function backdropRules(text) {
  return [...text.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .filter((m) => /backdrop-filter\s*:(?!\s*none)/.test(m[2]))
    .map((m) => ({ selectors: m[1].split(',').map((s) => s.trim()), body: m[2] }));
}

test('backdrop-filter exists only in app.css, only on the chrome selectors, never on a data selector', () => {
  for (const [f, text] of Object.entries(css)) {
    if (f === 'app.css') continue;
    assert.doesNotMatch(text, /backdrop-filter/i, `${f} must not use backdrop-filter (lab.css, pages.css, shell.css, sim.css, tokens.css)`);
  }
  const rules = backdropRules(css['app.css']);
  assert.ok(rules.length >= 4, 'the materials block exists');
  for (const r of rules) {
    for (const s of r.selectors) {
      assert.ok(CHROME.includes(s), `backdrop-filter on "${s}" is not a chrome selector`);
      for (const d of DATA_SELECTORS) assert.notEqual(s, d, `glass on data selector ${d}`);
    }
  }
});

test('glass is behind @supports, the radius is capped, and the blur is never animated', () => {
  const app = css['app.css'];
  assert.match(app, /@supports \(\(backdrop-filter: blur\(1px\)\) or \(-webkit-backdrop-filter: blur\(1px\)\)\)/);
  const tokens = css['tokens.css'];
  for (const m of tokens.matchAll(/--blur-\d:\s*(\d+)px/g)) assert.ok(Number(m[1]) <= 14, `blur radius ${m[1]}px > 14px`);
  for (const m of app.matchAll(/(?:^|[;{\s])(?:-webkit-)?backdrop-filter:\s*([^;}]+)/g)) {
    if (m[1].trim() === 'none') continue;
    assert.match(m[1], /^blur\(var\(--blur-[12]\)\) saturate\(var\(--sat\)\)$/, `unexpected backdrop-filter value: ${m[1]}`);
  }
  for (const [f, text] of Object.entries(css)) {
    for (const m of text.matchAll(/(?:transition|animation)[a-z-]*:\s*([^;}]+)/g)) assert.doesNotMatch(m[1], /backdrop|filter/, `${f}: backdrop-filter / filter in a transition or animation: ${m[0]}`);
  }
});

test('fallback ladder: reduced transparency, prefers-contrast: more and forced colours all return the chrome to solid surfaces', () => {
  const app = css['app.css'];
  const ladder = app.match(/@media \(prefers-reduced-transparency: reduce\), \(prefers-contrast: more\), \(forced-colors: active\) \{([\s\S]*?)\n  \}/);
  assert.ok(ladder, 'one ladder rung for the three preference queries');
  for (const sel of ['.glass-1', '.glass-2', '.app-header', '.form-bar', '.dialog']) assert.ok(ladder[1].includes(sel), `${sel} in the ladder`);
  assert.match(ladder[1], /backdrop-filter: none/);
  assert.match(app, /@media \(min-width: 820px\) and \(prefers-reduced-transparency: reduce\), \(min-width: 820px\) and \(prefers-contrast: more\), \(min-width: 820px\) and \(forced-colors: active\) \{\s*\.app-nav__panel \{[^}]*backdrop-filter: none/);
  // The solid default is declared BEFORE the @supports block, so a browser without backdrop-filter keeps it.
  assert.ok(app.indexOf('.app-header, .glass-1, .glass-2 { background-color: var(--surface); }') < app.indexOf('@supports ((backdrop-filter'));
  assert.match(css['tokens.css'], /@media \(prefers-reduced-transparency: reduce\)\s*\{\s*:root\s*\{\s*--scrim: rgb\(0 0 0 \/ 0\.7\)/);
  assert.match(css['tokens.css'], /@media \(prefers-contrast: more\)[\s\S]*--shadow-2: none/);
});

test('phones: only the header is glass; rail, run bar and dialog glass exist only from 820 px', () => {
  const app = css['app.css'];
  const desktop = app.match(/@media \(min-width: 820px\) \{\s*\.app-nav__panel \{[^}]*\}\s*\.form-bar \{[^}]*\}\s*\.dialog \{[^}]*\}\s*\}/);
  assert.ok(desktop, 'the three desktop-only glass rules sit together in one min-width: 820px block');
  const outside = app.replace(desktop[0], '');
  for (const sel of ['.app-nav__panel', '.form-bar', '.dialog']) {
    for (const r of backdropRules(outside)) assert.ok(!r.selectors.includes(sel), `${sel} has glass outside the desktop block`);
  }
  assert.equal(DESKTOP_SURFACES.length, 4, 'design budget: at most 4 glass surfaces on desktop');
  assert.match(css['shell.css'], /\.app-nav \{[^}]*background: var\(--surface-raised\)/, 'the phone menu sheet is a solid raised surface');
});

test('glass alpha floors guarantee the contrast margin: >= 0.90 light and >= 0.88 dark (DD 4.3)', () => {
  const t = css['tokens.css'];
  for (const g of ['glass-1', 'glass-2']) {
    const m = t.match(new RegExp(`--${g}:\\s*light-dark\\(\\s*rgb\\([^)]*/\\s*([0-9.]+)\\)\\s*,\\s*rgb\\([^)]*/\\s*([0-9.]+)\\)`));
    assert.ok(m, g);
    assert.ok(Number(m[1]) >= 0.9, `${g} light alpha ${m[1]}`);
    assert.ok(Number(m[2]) >= 0.88, `${g} dark alpha ${m[2]}`);
  }
});

test('check-contrast measures every glass token over BOTH extremes (pure black and pure white)', () => {
  const r = spawnSync(process.execPath, [path.resolve(here, '../scripts/check-contrast.mjs'), '--markdown'], { encoding: 'utf8' });
  assert.equal(r.status, 0, r.stderr);
  for (const theme of ['light', 'dark']) {
    for (const g of ['glass-1', 'glass-2']) {
      for (const x of ['black', 'white']) {
        assert.match(r.stdout, new RegExp(`\\| ${theme} \\| text \\| ${g}@${x} \\|[^|]*\\| 4\\.5 \\| PASS`), `${theme} text on ${g}@${x}`);
      }
    }
    for (const x of ['black', 'white']) {
      assert.match(r.stdout, new RegExp(`\\| ${theme} \\| text-3 \\| glass-1@${x} \\|[^|]*\\| 4\\.5 \\| PASS`));
      assert.match(r.stdout, new RegExp(`\\| ${theme} \\| accent \\| glass-1@${x} \\|[^|]*\\| 4\\.5 \\| PASS`));
    }
  }
  assert.doesNotMatch(r.stdout, /FAIL/);
});

test('radius: no pill shape remains (no 999px, no --radius-pill), every radius token is at most 14 px', () => {
  const all = Object.values(css).join('\n');
  assert.doesNotMatch(all, /border-radius:\s*999px/);
  assert.doesNotMatch(all, /--radius-pill/);
  for (const m of css['tokens.css'].matchAll(/--radius-[a-z]+:\s*(\d+)px/g)) assert.ok(Number(m[1]) <= 14, `radius ${m[0]}`);
  assert.doesNotMatch(css['app.css'], /\.pill\s*\{[^}]*999/);
});

test('colour palette: warm paper light, blue-black dark, ink-filled primary, no brand accent on the primary button', () => {
  const t = css['tokens.css'];
  assert.match(t, /--bg:\s*light-dark\(#f3f2ee,\s*#0b0e13\)/);
  assert.match(t, /--surface:\s*light-dark\(#fbfbf9,\s*#12161d\)/);
  assert.match(t, /--primary:\s*light-dark\(#14181f,\s*#eef1f6\)/);
  assert.match(css['app.css'], /\.btn--primary\s*\{[^}]*background-color:\s*var\(--primary\)[^}]*color:\s*var\(--on-primary\)/);
  assert.doesNotMatch(css['app.css'], /\.btn--primary\s*\{[^}]*var\(--accent\)/);
});

test('system fonts only: no @font-face, no font url(), no network in the styles', () => {
  const all = Object.values(css).join('\n');
  assert.doesNotMatch(all, /@font-face|url\(|@import\s+url|https?:\/\//);
  assert.doesNotMatch(all, /perspective|preserve-3d|rotate3d|translate3d|translateZ|will-change/, 'no 3D, no layer promotion');
});
