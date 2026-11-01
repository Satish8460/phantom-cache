// WCAG 2.x contrast check for the design tokens in src/styles/tokens.css, both themes.
// Colour tokens are written once as `light-dark(<light>, <dark>)` (hex or rgb()), so this script reads both values
// from the same declaration. Translucent tokens (tints) are blended over the solid surface they sit on before measuring.
// Usage: node scripts/check-contrast.mjs [--markdown]
// Exit code 1 if any required pair is below its threshold. This is a regression guard for token pairs, not proof that
// a rendered page is legible: it does not see text over images, composite UI, or anything not listed in PAIRS.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const css = fs.readFileSync(path.resolve(here, '../src/styles/tokens.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

const COLOR = String.raw`(#[0-9a-fA-F]{6}|rgb\(\s*\d+\s+\d+\s+\d+(?:\s*\/\s*[0-9.]+)?\s*\))`;
const DECL = new RegExp(String.raw`--([a-z0-9-]+)\s*:\s*light-dark\(\s*${COLOR}\s*,\s*${COLOR}\s*\)\s*;`, 'g');

export function parseColor(text) {
  if (text.startsWith('#')) {
    const n = parseInt(text.slice(1), 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a: 1 };
  }
  const m = /rgb\(\s*(\d+)\s+(\d+)\s+(\d+)(?:\s*\/\s*([0-9.]+))?\s*\)/.exec(text);
  return { r: +m[1], g: +m[2], b: +m[3], a: m[4] === undefined ? 1 : +m[4] };
}

export function parseThemes(source = css) {
  const light = {};
  const dark = {};
  for (const m of source.matchAll(new RegExp(DECL))) {
    light[m[1]] = parseColor(m[2]);
    dark[m[1]] = parseColor(m[3]);
  }
  return { light, dark };
}

const channel = (c) => {
  const s = c / 255;
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
};
const lum = ({ r, g, b }) => 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
export function ratio(a, b) {
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
const over = (fg, base) => ({
  r: Math.round(fg.r * fg.a + base.r * (1 - fg.a)),
  g: Math.round(fg.g * fg.a + base.g * (1 - fg.a)),
  b: Math.round(fg.b * fg.a + base.b * (1 - fg.a)),
  a: 1,
});
// "surface" -> the token; "surface+tint-press" -> that token with the translucent tint drawn over it;
// "glass-1@black" / "glass-1@white" -> the translucent glass token blended over the WORST-CASE backdrop (DD v2 4.3, guarantee 5):
// chrome floats over arbitrary content, so text on it must pass over pure black AND pure white, not only over the solid surface.
const EXTREME = { black: { r: 0, g: 0, b: 0, a: 1 }, white: { r: 255, g: 255, b: 255, a: 1 } };
function resolve(vars, spec) {
  const [token, backdrop] = spec.split('@');
  const [base, tint] = token.split('+');
  if (!vars[base]) return null;
  let out = vars[base];
  if (tint) out = vars[tint] ? over(vars[tint], vars[base]) : null;
  if (out && backdrop) out = EXTREME[backdrop] ? over(out, EXTREME[backdrop]) : null;
  return out;
}

const TEXT = 4.5;
const UI = 3;
const SURFACES = ['bg', 'surface', 'surface-2', 'surface-3', 'surface-raised', 'surface-overlay'];
const PAIRS = [];
const add = (fg, bg, min) => PAIRS.push([fg, bg, min]);

// Text on every surface level, and on the hover and pointer-down tints drawn over the surfaces that carry interactive rows.
for (const s of SURFACES) for (const t of ['text', 'text-2', 'text-3']) add(t, s, TEXT);
for (const s of ['surface', 'surface-2', 'surface-raised']) for (const t of ['tint-hover', 'tint-press']) for (const f of ['text', 'text-2', 'text-3']) add(f, `${s}+${t}`, TEXT);
add('accent', 'bg', TEXT); add('accent', 'surface', TEXT); add('accent', 'surface-2', TEXT); add('accent', 'surface-raised', TEXT);
// Ink-filled primary button (DD 4.1): label on the fill, and on the hover / pointer-down tint drawn over it.
add('on-primary', 'primary', TEXT); add('on-primary', 'primary+tint-hover', TEXT); add('on-primary', 'primary+tint-press', TEXT);
add('primary', 'bg', UI); add('primary', 'surface', UI);
// Text drawn directly on the page ground sits on the static technical grid line (DD 4.1 item 4): measure on the grid colour too.
for (const t of ['text', 'text-2', 'text-3', 'accent']) add(t, 'bg+grid-line', TEXT);
add('accent-contrast', 'accent', TEXT); add('accent-contrast', 'accent+tint-hover', TEXT); add('accent-contrast', 'accent+tint-press', TEXT);

// State colours: as text on their own background, on surfaces, and body text on their background (banners).
for (const k of ['ok', 'warn', 'critical', 'info']) {
  add(k, `${k}-bg`, TEXT); add(k, 'surface', TEXT); add(k, 'surface-2', TEXT); add('text', `${k}-bg`, TEXT);
}
// Provenance (spec section 11): label on its own fill, on surfaces, and the 1 px border (same colour) as non-text UI against surfaces.
for (const k of ['measured', 'derived', 'simulated', 'replayed', 'predicted', 'unavailable']) {
  add(`prov-${k}`, `prov-${k}-bg`, TEXT); add(`prov-${k}`, 'surface', TEXT); add(`prov-${k}`, 'surface-2', TEXT);
  add(`prov-${k}`, 'bg', UI); add(`prov-${k}`, 'surface-raised', UI);
}
// Interaction and structure: input and control borders (1.4.11), focus ring against every surface, chart marks.
for (const s of ['surface', 'bg', 'surface-2', 'surface-raised', 'surface-overlay']) { add('border-strong', s, UI); add('focus', s, UI); }
add('chart-line', 'surface', UI); add('chart-line', 'surface-2', UI);

// Glass (DD v2 4.3). Chrome only: header, rail, dialog (glass-1) and run bar (glass-2). Each pair is measured over BOTH extremes.
for (const g of ['glass-1', 'glass-2']) {
  for (const x of ['black', 'white']) {
    const s = `${g}@${x}`;
    for (const t of ['text', 'text-2']) add(t, s, TEXT);
    add('border-strong', s, UI); add('focus', s, UI);
    for (const k of ['critical', 'ok', 'warn']) add(k, s, TEXT); // inline errors and reasons inside the run bar / dialog
  }
}
for (const x of ['black', 'white']) {
  const s = `glass-1@${x}`;
  for (const t of ['text-3', 'accent']) add(t, s, TEXT); // header and rail carry captions (text-3) and links (accent)
}

const { light, dark } = parseThemes();
const themes = { light, dark };
let failures = 0;
const rows = [];
for (const [name, vars] of Object.entries(themes)) {
  for (const [fg, bg, min] of PAIRS) {
    const a = resolve(vars, fg);
    const b = resolve(vars, bg);
    if (!a || !b) {
      failures += 1;
      rows.push({ name, fg, bg, min, r: NaN, ok: false, note: 'token missing' });
      continue;
    }
    const r = ratio(a, b);
    const ok = r >= min;
    if (!ok) failures += 1;
    rows.push({ name, fg, bg, min, r, ok });
  }
}

if (!import.meta.url.endsWith(path.basename(process.argv[1] || ''))) {
  // imported as a module (tests): no output, no exit.
} else {
  if (process.argv.includes('--markdown')) {
    console.log('| Theme | Foreground | Background | Ratio | Required | Result |\n|---|---|---|---|---|---|');
    for (const x of rows) console.log(`| ${x.name} | ${x.fg} | ${x.bg} | ${Number.isNaN(x.r) ? 'n/a' : x.r.toFixed(2)} | ${x.min} | ${x.ok ? 'PASS' : 'FAIL'} |`);
  } else {
    for (const x of rows.filter((r) => !r.ok)) console.error(`FAIL [${x.name}] ${x.fg} on ${x.bg}: ${Number.isNaN(x.r) ? x.note : x.r.toFixed(2)} < ${x.min}`);
    console.log(`[check-contrast] ${rows.length - failures}/${rows.length} checks passed`);
  }
  process.exit(failures ? 1 : 0);
}
