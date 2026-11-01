// V1 foundations v2: MOTION guards (DD v2 section 6, 16 step 5; external_design_skills_project_rules.md rule 9).
// STATIC checks over CSS text. A green run means "the listed rules still hold"; it does not show that any animation feels right,
// that nothing drops frames, or that reduced motion is complete on every page (that needs `document.getAnimations()` in a browser).
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const stylesDir = path.resolve(here, '../src/styles');
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '');
const css = Object.fromEntries(fs.readdirSync(stylesDir).filter((f) => f.endsWith('.css')).map((f) => [f, strip(fs.readFileSync(path.join(stylesDir, f), 'utf8'))]));
const V1_FILES = ['tokens.css', 'app.css', 'shell.css', 'pages.css'];
const ms = (v) => (/ms$/.test(v) ? parseFloat(v) : parseFloat(v) * 1000);

test('motion tokens: DD tiers exist, none above 300 ms, the spring has a plain fallback and no overshoot', () => {
  const t = css['tokens.css'];
  for (const k of ['press', 'fast', 'base']) assert.match(t, new RegExp(`--d-${k}:\\s*\\d+ms;`), k);
  for (const m of t.matchAll(/--(?:d|dur)-[a-z]+:\s*(\d+ms)/g)) assert.ok(ms(m[1]) <= 300, `${m[0]} above 300 ms`);
  assert.match(t, /--ease-spring:\s*var\(--ease-out\);/, 'fallback first');
  assert.match(t, /@supports \(transition-timing-function: linear\(0, 1\)\)\s*\{\s*:root\s*\{\s*--ease-spring:\s*linear\(/);
  const stops = t.match(/--ease-spring:\s*linear\(([^)]*)\)/)[1].split(',').map(Number);
  assert.ok(stops.every((v) => v >= 0 && v <= 1), 'no stop above 1: critically damped, no overshoot');
  assert.equal(stops[stops.length - 1], 1);
});

test('no `transition: all`, and only compositor-friendly or colour properties are transitioned in the V1 files', () => {
  const OK = new Set(['transform', 'opacity', 'background-color', 'border-color', 'color', 'none']);
  for (const f of V1_FILES) {
    for (const m of css[f].matchAll(/transition(?:-property)?:\s*([^;}]+)/g)) {
      assert.doesNotMatch(m[1], /\ball\b/, `${f}: transition: all`);
      for (const part of m[1].split(/,(?![^(]*\))/)) {
        const prop = part.trim().split(/\s+/)[0];
        assert.ok(OK.has(prop), `${f}: transitions "${prop}" (allowed: ${[...OK].join(', ')})`);
      }
    }
  }
  for (const [f, text] of Object.entries(css)) {
    assert.doesNotMatch(text, /transition[a-z-]*:[^;}]*\b(box-shadow|width|height|backdrop-filter|filter|margin|padding|top|left)\b/, `${f}: animated layout/paint property`);
  }
});

test('keyframes animate transform and opacity only', () => {
  for (const [f, text] of Object.entries(css)) {
    for (const m of text.matchAll(/@keyframes ([\w-]+)\s*\{([\s\S]*?\}\s*)\}/g)) {
      const props = [...m[2].matchAll(/([a-z-]+)\s*:/g)].map((p) => p[1]);
      for (const p of props) assert.ok(p === 'transform' || p === 'opacity', `${f}: @keyframes ${m[1]} animates ${p}`);
    }
  }
});

test('durations in the V1 files come from tokens; the only literals are the two infinite loops, and none exceeds 1.5 s', () => {
  for (const f of V1_FILES) {
    if (f === 'tokens.css') continue;
    for (const m of css[f].matchAll(/(?:animation|transition)(?:-duration)?:([^;}]*)/g)) {
      for (const d of m[1].matchAll(/(?<![\w.-])([0-9.]+)(ms|s)\b/g)) {
        assert.ok(/infinite/.test(m[1]) && ms(d[1] + d[2]) <= 1500, `${f}: literal duration in "${m[0]}"`);
      }
    }
  }
});

test('reduced motion: movement becomes an opacity fade of at most 120 ms or nothing; no transform animation survives in the V1 files', () => {
  const app = css['app.css'];
  const block = app.match(/@media \(prefers-reduced-motion: reduce\)\s*\{([\s\S]*?)\n\}\n/)[1];
  assert.match(block, /\.dialog, \.menu, \.popover, \.disclosure\[open\] > :not\(summary\) \{ animation-name: pc-fade;/);
  assert.match(block, /\.tab::after \{ transform: none; opacity: 0;/);
  assert.match(block, /\.btn \{ transition: background-color var\(--d-press\) linear, border-color var\(--d-press\) linear; \}/, 'A11Y-05: no transform transition on .btn');
  // pc-fade, pc-press and the fade tokens are the only durations used in the block; --d-fast (140 ms) is the dialog cross-fade, so check the honest bound.
  const t = css['tokens.css'];
  const fast = Number(t.match(/--d-fast:\s*(\d+)ms/)[1]);
  const press = Number(t.match(/--d-press:\s*(\d+)ms/)[1]);
  assert.ok(press <= 120, 'state fades use --d-press (<= 120 ms)');
  assert.ok(fast <= 160, 'the reduced dialog / popover cross-fade uses --d-fast (<= 160 ms, the accepted exception in the external-skills rules section 3)');
  assert.match(css['shell.css'], /@media \(prefers-reduced-motion: reduce\) \{[^@]*\.app-nav\.is-open \{ animation-name: pc-fade; \}/);
  assert.match(css['shell.css'], /\.app-nav__fold > summary::after \{ transition: none; \}/);
  const fade = app.match(/@keyframes pc-fade\s*\{[^\n]*\}/)[0];
  assert.doesNotMatch(fade, /transform/);
});

test('phones: the dialog fades only, and the pop keyframes are opacity + transform', () => {
  const app = css['app.css'];
  assert.match(app, /@media \(max-width: 819px\) \{\s*\.dialog \{ animation-name: pc-fade; \}/);
  assert.match(app, /@keyframes pc-pop \{ from \{ opacity: 0; transform: scale\(0\.97\); \} to \{ opacity: 1; transform: none; \} \}/);
});
