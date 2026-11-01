// Tests for the dependency-free Markdown renderer used for the binding Scenario B interpretation notice (ui/src/lib/miniMarkdown.js)
// and for the RichNotice component (rendered view, Raw view). These are REGRESSION GUARDS on parsing, escaping and text
// preservation. They are not proof of safety in every context and they say nothing about visual quality (judge that in a browser).
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { MAX_CHARS, collapse, parseInline, parseMarkdown, plainText, stripMarkers } from '../src/lib/miniMarkdown.js';
import { FX } from './helpers/labFixtures.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
let R; // { React, renderToStaticMarkup, RichNotice }

before(async () => {
  // Bundle just RichNotice (the shared render harness does not export it). Output goes to the OS temp dir, outside the repository.
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phantom-md-'));
  const outfile = path.join(outDir, 'rich.cjs');
  await build({
    stdin: {
      contents: "import React from 'react'; import { renderToStaticMarkup } from 'react-dom/server'; export { RichNotice } from './src/components/LabParts.jsx'; export { React, renderToStaticMarkup };",
      resolveDir: ROOT,
      loader: 'jsx',
    },
    outfile, bundle: true, platform: 'node', format: 'cjs', jsx: 'automatic', loader: { '.css': 'empty', '.js': 'jsx' },
    define: { 'import.meta.env': '{}', 'import.meta': '{}', 'process.env.NODE_ENV': '"production"' }, logLevel: 'error',
  });
  R = createRequire(import.meta.url)(outfile);
  process.on('exit', () => { try { fs.rmSync(outDir, { recursive: true, force: true }); } catch { /* best effort */ } });
});

const notice = () => FX.sbEvidence().interpretation.notice_text;
const render = (text, defaultView) => R.renderToStaticMarkup(R.React.createElement(R.RichNotice, { text, label: 'notice', defaultView }));
const decode = (s) => s.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#x27;/g, "'").replace(/&amp;/g, '&');
// Text a reader sees: the end of a block becomes a space so words from neighbouring blocks do not run together.
const textOf = (html) => decode(html.replace(/<\/(p|h[1-6]|li|ul|ol|pre|div)>/g, ' ').replace(/<[^>]*>/g, ''));
const PANE = {
  raw: /<pre[^>]*data-notice-pane="raw"[^>]*>([\s\S]*?)<\/pre>/,
  rendered: /<div[^>]*data-notice-pane="rendered"[^>]*>([\s\S]*)<\/div><\/div>$/,
};
const pane = (html, which) => PANE[which].exec(html.replace(/\s+$/, ''));
const ALLOWED_TAGS = new Set(['p', 'strong', 'em', 'code', 'h3', 'h4', 'h5', 'h6', 'ol', 'ul', 'li', 'br', 'pre', 'hr']);

test('fixture: the real Scenario B notice has text to render (guards the tests below against an empty payload)', () => {
  assert.ok(notice().length > 2000);
  assert.ok(notice().length < MAX_CHARS);
});

test('parser: the real notice parses into a heading, paragraphs with bold, a numbered list of 8 and inline code', () => {
  const r = parseMarkdown(notice());
  assert.equal(r.ok, true);
  const types = r.blocks.map((b) => b.type);
  assert.equal(types[0], 'h');
  assert.equal(r.blocks[0].level, 1);
  const ol = r.blocks.find((b) => b.type === 'ol');
  assert.ok(ol);
  assert.equal(ol.items.length, 8);
  assert.ok(r.blocks.some((b) => b.type === 'p' && b.inline.some((n) => n.t === 'b')));
  assert.ok(JSON.stringify(r.blocks).includes('"t":"code"'));
});

test('rendered notice: no raw markers are visible (no **, no backticks, no leading "# ")', () => {
  const html = render(notice());
  const body = pane(html, 'rendered')[1];
  const visible = textOf(body);
  assert.doesNotMatch(visible, /\*\*/);
  assert.doesNotMatch(visible, /`/);
  assert.doesNotMatch(visible, /(^|\n)\s*#{1,6}\s/);
  assert.match(body, /<h3 class="md-heading">Scenario B/);
  assert.match(body, /<strong>Verified:<\/strong>/);
  assert.match(body, /<ol>(<li>[\s\S]*?<\/li>){8}<\/ol>/);
  assert.match(body, /<code>lab-results\/scenario_b\/20261030T003932Z\/<\/code>/);
});

test('binding wording is preserved: the rendered text equals the raw text with only the markers removed', () => {
  const raw = notice();
  const rendered = textOf(pane(render(raw), 'rendered')[1]);
  // List numbers are drawn by the browser (<ol>), so they are not in the text; headings and bold lose their markers.
  assert.equal(rendered.replace(/\s+/g, ''), stripMarkers(raw).replace(/\s+/g, ''));
  const words = (s) => collapse(s).split(' ').filter(Boolean);
  const rawWords = new Set(words(raw.replace(/^#{1,6}\s+/gm, '').replace(/[*`]/g, '')));
  for (const w of words(rendered)) assert.ok(rawWords.has(w) || /^\d+[.)]$/.test(w), `word not in the source: ${w}`);
  for (const phrase of ['Qualifications (must accompany any citation of these results):', 'Permitted conclusion:', 'Not established:', 'Future work (hypothesis only):']) {
    assert.ok(rendered.includes(phrase), phrase);
  }
  assert.equal(collapse(plainText(parseMarkdown(raw).blocks)), stripMarkers(raw));
});

test('Raw view is byte-equal to the payload string (and is the only text in the DOM in that view)', () => {
  const raw = notice();
  const html = render(raw, 'raw');
  assert.match(html, /data-notice-view="raw"/);
  const inner = pane(html, 'raw')[1];
  assert.equal(decode(inner), raw);
  assert.doesNotMatch(html, /data-notice-pane="rendered"/);
  assert.match(html, /aria-pressed="true"[^>]*>Raw<\/button>/);
  assert.match(html, />Copy text<\/button>/);
});

test('hostile input is inert: no script, img, handler or link is produced; the text is still shown', () => {
  const hostile = [
    '<script>alert(1)</script>',
    '<img src=x onerror=alert(1)>',
    '[click me](javascript:alert(1))',
    '**bold <b onclick="x()">x</b>**',
    '<a href="https://evil.example">link</a>',
    '`<iframe src=//evil>`',
    '[ok](https://example.org/path)',
  ].join('\n\n');
  const html = render(hostile);
  const body = pane(html, 'rendered')[1];
  const tags = [...body.matchAll(/<\/?([a-z0-9]+)([^>]*)>/g)];
  for (const [, name, attrs] of tags) {
    assert.ok(ALLOWED_TAGS.has(name), `unexpected element <${name}>`);
    assert.doesNotMatch(attrs, /href|src|on[a-z]+=|style/i);
  }
  assert.ok(body.includes('&lt;script&gt;alert(1)&lt;/script&gt;'));
  const seen = textOf(body);
  assert.match(seen, /click me \(javascript:alert\(1\)\)/, 'a disabled link shows its text and where it pointed, as plain text');
  assert.match(seen, /ok \(https:\/\/example\.org\/path\)/);
  assert.match(seen, /link/);
  assert.doesNotMatch(body, /<a\b/);
});

test('inline edge cases: nested bold and code, unbalanced markers and stray characters are kept as text', () => {
  const inl = (s) => parseInline(s);
  assert.deepEqual(inl('a **b `c` d** e'), [
    { t: 'text', v: 'a ' }, { t: 'b', c: [{ t: 'text', v: 'b ' }, { t: 'code', v: 'c' }, { t: 'text', v: ' d' }] }, { t: 'text', v: ' e' },
  ]);
  assert.equal(JSON.stringify(inl('**unclosed')), JSON.stringify([{ t: 'text', v: '**unclosed' }]));
  assert.equal(JSON.stringify(inl('`unclosed')), JSON.stringify([{ t: 'text', v: '`unclosed' }]));
  assert.equal(JSON.stringify(inl('stale_served_total = 0')), JSON.stringify([{ t: 'text', v: 'stale_served_total = 0' }]), 'underscores in identifiers are not emphasis');
  assert.equal(JSON.stringify(inl('2 * 3 * 4')), JSON.stringify([{ t: 'text', v: '2 * 3 * 4' }]), 'spaced asterisks are not emphasis');
  assert.equal(plainText(parseMarkdown('a\\\nb').blocks), 'a\nb', 'a trailing backslash is a hard line break');
});

test('block edge cases: heading levels, bullets, continuation lines, fenced code, hr, empty input', () => {
  const r = parseMarkdown('# One\n## Two\n#### Four\n\n- a\n  more\n- b\n\n3) x\n4) y\n\n```\n**not bold**\n```\n\n---\n\ntext');
  assert.deepEqual(r.blocks.map((b) => b.type), ['h', 'h', 'h', 'ul', 'ol', 'code', 'hr', 'p']);
  assert.deepEqual(r.blocks.filter((b) => b.type === 'h').map((b) => b.level), [1, 2, 4]);
  assert.equal(plainText([r.blocks[3]]), 'a more\nb');
  assert.equal(r.blocks[4].start, 3);
  assert.equal(r.blocks[5].text, '**not bold**');
  assert.deepEqual(parseMarkdown('').blocks, []);
  assert.deepEqual(parseMarkdown('   \n\n').blocks, []);
  assert.equal(parseMarkdown(null).ok, false);
});

test('size cap: text longer than 20,000 characters is not parsed; the component shows it raw and complete, with a note', () => {
  const long = 'word '.repeat(4001); // 20,005 characters
  assert.ok(long.length > MAX_CHARS);
  assert.equal(parseMarkdown(long).ok, false);
  assert.equal(parseMarkdown(long).reason, 'too_long');
  const html = render(long);
  assert.match(html, /data-notice-view="raw"/);
  assert.equal(decode(pane(html, 'raw')[1]), long, 'nothing is cut');
  assert.match(html, /longer than the renderer accepts/);
  assert.match(html, /disabled=""[^>]*>Rendered<\/button>/);
  const exact = 'a'.repeat(MAX_CHARS);
  assert.equal(parseMarkdown(exact).ok, true, 'exactly the cap is still parsed');
});

test('performance guard: pathological marker runs at the cap parse in well under a second', () => {
  for (const unit of ['*a ', '**a ', '`a ', '[a](b ', '**`', '* ', '- ', '1. ', '# ']) {
    const t0 = Date.now();
    parseMarkdown(unit.repeat(Math.floor(MAX_CHARS / unit.length)));
    assert.ok(Date.now() - t0 < 1000, `unit ${JSON.stringify(unit)} took ${Date.now() - t0} ms`);
  }
});

test('the rendered output carries no inline style or event handler attribute', () => {
  const html = render(notice());
  assert.doesNotMatch(html, /\sstyle=|\son[a-z]+=/i);
});
