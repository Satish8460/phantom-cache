// Worst-case contrast of text on a translucent glass surface, with NO reliance on blur (blur only averages the backdrop,
// so the extreme backdrops pure black and pure white bound every real case). Dependency-free.
// Result: the minimum opaque-equivalent alpha for each theme that keeps the existing tokens at WCAG 1.4.3 (4.5:1).
const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const lin = (c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; };
const L = (rgb) => 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
const cr = (a, b) => { const x = L(a), y = L(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
const over = (fg, a, bg) => fg.map((c, i) => Math.round(c * a + bg[i] * (1 - a)));
const themes = {
  light: { surface: hex('#ffffff'), worst: hex('#000000'), best: hex('#ffffff'), text: { text: '#14181f', 'text-2': '#3f4756', 'text-3': '#566070', accent: '#0b5fd1' } },
  dark: { surface: hex('#161a21'), worst: hex('#ffffff'), best: hex('#000000'), text: { text: '#e8ebf0', 'text-2': '#b9c1ce', 'text-3': '#a5afc0', accent: '#6aa8ff' } },
};
const out = [];
for (const [name, t] of Object.entries(themes)) {
  for (const a of [0.6, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 1]) {
    const bg = over(t.surface, a, t.worst);
    const row = { theme: name, alpha: a, bgOverWorstBackdrop: '#' + bg.map((x) => x.toString(16).padStart(2, '0')).join('') };
    for (const [k, c] of Object.entries(t.text)) row[k] = +cr(hex(c), bg).toFixed(2);
    out.push(row);
  }
}
console.table(out);
import { writeFileSync } from 'node:fs';
writeFileSync(new URL('./contrast_worst_case.json', import.meta.url), JSON.stringify(out, null, 1));
