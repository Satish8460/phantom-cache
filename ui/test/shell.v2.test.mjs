// V1 foundations v2: SHELL guards (DD v2 section 7, 16 step 3; A11Y-01, A11Y-02, A11Y-11 from remediation/03_validation_accessibility.md).
// STATIC checks over server-rendered markup and CSS text. They do not prove keyboard behaviour or appearance in a browser; the browser
// evidence (screenshots, axe, CDP) is listed in docs/design/remediation/increment_V1_foundations_v2.md.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';

const UI = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => fs.readFileSync(path.join(UI, p), 'utf8');
const strip = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '');
const shell = strip(read('src/styles/shell.css'));
const app = strip(read('src/styles/app.css'));
const pages = strip(read('src/styles/pages.css'));
let H;
before(async () => { H = await getHarness(); });

test('header: ghost mark (decorative), product name, connection indicator with glyph and words, theme menu behind a native <details>', () => {
  const html = H.renderShell({ connKind: 'disconnected', currentId: 'overview' });
  const header = /<header[\s\S]*?<\/header>/.exec(html)[0];
  assert.match(header, /<svg class="ghost-mark"[^>]*aria-hidden="true"/);
  assert.match(header, /PhantomCache<span class="app-header__brand-more"> Control Center<\/span>/);
  assert.match(header, /<div class="app-header__status" data-conn-kind="disconnected"><span class="pill tone-critical"><svg[\s\S]*?<span>Disconnected<\/span>/);
  assert.match(header, /<details class="theme-menu"><summary aria-label="Colour theme"><svg[^>]*aria-hidden="true"[\s\S]*?<span class="visually-hidden">Colour theme<\/span><\/summary><div class="popover theme-menu__panel"><div class="seg theme-control"/);
  assert.equal((header.match(/aria-pressed="true"/g) || []).length, 1);
  assert.deepEqual(lintHtml(html, { requireH1: false }), []);
});

test('header: the status keeps the connection words of every state (no state lost in the restyle)', () => {
  for (const kind of ['live', 'loading', 'stale', 'degraded', 'disconnected', 'error']) {
    const html = H.renderShell({ connKind: kind, currentId: 'overview' });
    assert.match(/<header[\s\S]*?<\/header>/.exec(html)[0], new RegExp(`data-conn-kind="${kind}"`), kind);
    assert.match(/<div class="app-header__status"[\s\S]*?<\/div>/.exec(html)[0], /<span class="pill tone-[a-z]+"><svg[\s\S]*?<span>[A-Za-z][^<]+<\/span>/, kind);
  }
});

test('rail: "Not available yet" is one disclosure with its count, open by default; the other groups are plain lists; every link is still in the DOM', () => {
  const html = H.renderShell({ connKind: 'live', currentId: 'intelligence' });
  const fold = /<details class="app-nav__fold" open="">[\s\S]*?<\/details>/.exec(html)[0];
  assert.match(fold, /<summary><span class="app-nav__group-label" id="app-nav-group-4">Not available yet<\/span><span class="app-nav__group-count">1<\/span><\/summary>/);
  assert.equal((fold.match(/<a /g) || []).length, 1); // batch B2: Cluster moved to "Live gateway"; only Intelligence is still unavailable
  assert.match(fold, /aria-current="page"/);
  assert.equal((html.match(/<details class="app-nav__fold"/g) || []).length, 1, 'only that group folds');
  assert.equal((html.match(/<a href="#\/[^"]*"/g) || []).length, 10, 'all ten links are present (Start here is first)');
});

test('rail: the fold choice is stored in try/catch and nothing else is persisted by the shell', () => {
  const src = read('src/components/Shell.jsx');
  assert.match(src, /function readFold\(\) \{\s*try \{[\s\S]*?\} catch \{\s*return true;/);
  assert.match(src, /function writeFold\(open\) \{\s*try \{[\s\S]*?\} catch \{/);
  assert.equal((src.match(/setItem\(/g) || []).length, 1);
  assert.doesNotMatch(src, /token|sessionStorage|document\.cookie/i);
});

test('header and rail CSS: sticky 56 px glass header, rail offset below it, phone sheet fixed under the header and solid', () => {
  assert.match(shell, /\.app-header \{[^}]*position: sticky; top: 0; z-index: 6;/);
  assert.match(shell, /min-height: 3\.5rem/);
  assert.match(read('src/styles/tokens.css'), /--header-h:\s*3\.5rem;/);
  assert.match(shell, /\.app-nav__panel \{ position: sticky; top: var\(--header-h\); height: calc\(100vh - var\(--header-h\)\);/);
  assert.match(shell, /\.app-nav\.is-open \{ display: block; position: fixed; top: var\(--header-h\);[^}]*max-height: calc\(100vh - var\(--header-h\)\); overflow-y: auto;/);
  assert.match(shell, /\.app-nav \{ grid-area: nav; display: none; background: var\(--surface-raised\)/);
  assert.match(shell, /@media \(max-height: 30rem\) \{\s*\.app-header \{ position: static; \}/, 'a short viewport does not keep a sticky header');
  assert.match(app, /html \{[^}]*scroll-padding-top: calc\(var\(--header-h\) \+ var\(--space-3\)\)/, 'WCAG 2.4.11: sticky header does not hide a focused element');
  const src = read('src/components/Shell.jsx');
  assert.match(src, /setProperty\('--header-h'/);
  assert.match(src, /ResizeObserver/);
});

test('A11Y-02: the run bar wraps instead of clipping at 200% text (rem spacing, min() flex-basis, bounded height)', () => {
  assert.match(app, /\.form-bar__next \{ flex: 1 1 min\(100%, 14rem\);/);
  assert.match(app, /\.form-bar \{[^}]*flex-wrap: wrap;[^}]*max-height: 40vh; overflow-y: auto;/);
  assert.match(pages, /\.status-line p \{[^}]*flex: 1 1 min\(100%, 14rem\)/);
  assert.match(pages, /\.not-reported__reason \{[^}]*flex: 1 1 min\(100%, 12rem\)/);
  assert.match(app, /\.metric-grid \{[^}]*minmax\(min\(100%, 11rem\), 1fr\)/);
  for (const f of ['app.css', 'shell.css', 'pages.css']) {
    const text = strip(read(`src/styles/${f}`));
    for (const m of text.matchAll(/(?<![-\w(])(width|min-width|height|min-height|padding[a-z-]*|margin[a-z-]*|gap):\s*(\d+(?:\.\d+)?)px/g)) {
      assert.ok((['width', 'height'].includes(m[1]) || m[1].startsWith('padding') || m[1].startsWith('margin') || m[1] === 'gap') && Number(m[2]) <= 3,`${f}: fixed px ${m[0]} (use rem so 200% text reflows)`);
    }
  }
});

test('A11Y-01: run-id links keep their word width and are never narrower than the 24 px target floor', () => {
  assert.match(app, /\.app-main a\.mono, \.app-main th a \{ overflow-wrap: break-word; min-width: max\(var\(--target-min\), 8ch\); \}/);
});

test('A11Y-11: forced colours tell selection (filled Highlight) from focus (a CanvasText ring offset from the control)', () => {
  const fc = app.match(/@media \(forced-colors: active\)\s*\{([\s\S]*?)\n\}\n/)[1];
  assert.match(fc, /\.seg__btn\[aria-pressed="true"\] \{ forced-color-adjust: none; background: Highlight; color: HighlightText; outline: none; \}/);
  const sfc = shell.match(/@media \(forced-colors: active\)\s*\{([\s\S]*?)\n\}/)[1];
  assert.match(sfc, /\.app-nav a\[aria-current="page"\] \{ forced-color-adjust: none; background: Highlight; color: HighlightText; box-shadow: none; outline: none; \}/, 'shell.css loads after app.css, so the nav rule lives there');
  assert.match(fc, /:focus-visible \{ outline: 3px solid CanvasText; outline-offset: 3px; \}/);
  assert.match(fc, /\.tab\[aria-selected="true"\]::after, \.tab\[aria-current="page"\]::after \{ background: Highlight;/);
});

test('components: pills and tags are 5 px marks; provenance mark is 22 px high; RETAINED stamp is solid ink; tabs use a 2 px ink underline that slides by scaleX', () => {
  assert.match(app, /\.pill \{[^}]*border-radius: var\(--radius-sm\)/);
  assert.match(app, /\.prov \{[^}]*min-height: 1\.375rem/);
  assert.match(app, /\.stamp \{[^}]*border: 1px solid var\(--text\)/);
  assert.match(app, /\.tab::after \{[^}]*height: 2px; background: var\(--text\); transform: scaleX\(0\);[^}]*transition: transform var\(--d-base\)/);
  assert.match(app, /\.tab\[aria-selected="true"\]::after, \.tab\[aria-current="page"\]::after \{ transform: scaleX\(1\); \}/);
});

test('shell components keep their exported names and props (render-harness, other agents)', async () => {
  const src = read('src/components/Shell.jsx');
  for (const n of ['export function ThemeControl', 'export function Header', 'export function Nav', 'export const NAV_LIST_ID', 'export const MENU_BUTTON_ID']) assert.ok(src.includes(n), n);
  assert.match(src, /Header\(\{ connKind, theme, onTheme, menuOpen = false, onMenu = \(\) => \{\} \}\)/);
  assert.match(src, /Nav\(\{ currentId, open = false, onNavigate = \(\) => \{\}, theme = 'system', onTheme = \(\) => \{\}, mode, gatewayUrl \}\)/);
  assert.match(read('src/components/Icon.jsx'), /export function GhostMark/);
});
