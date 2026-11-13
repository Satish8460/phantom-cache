// Minimal, dependency-free Markdown renderer for the Phantom Lab (the binding interpretation notice and evidence text).
//
// Safety model (tested in ui/test/lab.markdown.test.mjs, a regression guard and not proof of safety in every context):
//  * Input is plain text. Output is a small AST that the caller turns into React ELEMENTS (renderMarkdown). Nothing is ever
//    written as HTML, so `<script>`, `<img onerror=...>` and the like are shown as inert text (React escapes text nodes).
//  * Links are DISABLED: `[text](url)` shows `text (url)` as plain text. No element in the output has an href or an event handler.
//  * Unknown or unbalanced syntax is left as text. Nothing the author wrote is dropped (see plainText / stripMarkers).
//  * Input longer than MAX_CHARS is not parsed (ok: false, reason 'too_long'); the caller shows the Raw text instead, so a
//    long binding text is never truncated.
// Supported: headings (# to ####), paragraphs, bulleted and numbered lists (single level, continuation lines), fenced and
// inline code, **bold**, *italic*, horizontal rule, hard line break (two trailing spaces or a backslash).

export const MAX_CHARS = 20000;
const MAX_LINES = 1000;

const HEADING = /^(#{1,6})[ \t]+(.*?)[ \t]*#*[ \t]*$/;
const FENCE = /^(```|~~~)/;
const HR = /^(?:-{3,}|\*{3,}|_{3,})[ \t]*$/;
const UL = /^[ \t]{0,3}[-*+][ \t]+(.*)$/;
const OL = /^[ \t]{0,3}(\d{1,9})[.)][ \t]+(.*)$/;

/** Inline parsing: code spans first, then **bold**, then *italic*, then `[text](url)` shown as text. */
export function parseInline(src) {
  const out = [];
  let i = 0;
  let buf = '';
  const flush = () => {
    if (buf) {
      out.push({ t: 'text', v: buf });
      buf = '';
    }
  };
  while (i < src.length) {
    const ch = src[i];
    if (ch === '`') {
      const end = src.indexOf('`', i + 1);
      if (end > i + 1) {
        flush();
        out.push({ t: 'code', v: src.slice(i + 1, end) });
        i = end + 1;
        continue;
      }
    }
    if (ch === '*' && src[i + 1] === '*') {
      const end = findClose(src, i + 2, '**');
      if (end > i + 2) {
        flush();
        out.push({ t: 'b', c: parseInline(src.slice(i + 2, end)) });
        i = end + 2;
        continue;
      }
    }
    if (ch === '*' && src[i + 1] !== '*' && src[i + 1] !== undefined && !/\s/.test(src[i + 1])) {
      const end = findClose(src, i + 1, '*');
      if (end > i + 1 && !/\s/.test(src[end - 1]) && src[end + 1] !== '*') {
        flush();
        out.push({ t: 'i', c: parseInline(src.slice(i + 1, end)) });
        i = end + 1;
        continue;
      }
    }
    if (ch === '[') {
      const m = /^\[([^\]\n]*)\]\(([^)\s]*)\)/.exec(src.slice(i));
      if (m) {
        // Links are disabled: show the text and, in plain text, where it pointed.
        buf += m[2] ? `${m[1]} (${m[2]})` : m[1];
        i += m[0].length;
        continue;
      }
    }
    if (ch === '\\' && src[i + 1] === '\n') {
      flush();
      out.push({ t: 'br' });
      i += 2;
      continue;
    }
    if (ch === '\n') {
      if (buf.endsWith('  ')) {
        buf = buf.replace(/ +$/, '');
        flush();
        out.push({ t: 'br' });
      } else {
        buf += ' ';
      }
      i += 1;
      continue;
    }
    buf += ch;
    i += 1;
  }
  flush();
  return out;
}

// Closing delimiter search that skips code spans, so `**a `**` b**` does not close inside the code.
function findClose(src, from, delim) {
  let i = from;
  while (i < src.length) {
    if (src[i] === '`') {
      const end = src.indexOf('`', i + 1);
      if (end === -1) return -1;
      i = end + 1;
      continue;
    }
    if (src.startsWith(delim, i)) return i;
    i += 1;
  }
  return -1;
}

/** Parse text into blocks. Returns { ok: true, blocks } or { ok: false, reason, blocks: [] }. */
export function parseMarkdown(text) {
  if (typeof text !== 'string') return { ok: false, reason: 'not_text', blocks: [] };
  if (text.length > MAX_CHARS) return { ok: false, reason: 'too_long', blocks: [] };
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  if (lines.length > MAX_LINES) return { ok: false, reason: 'too_long', blocks: [] };
  const blocks = [];
  let para = [];
  let list = null;
  const flushPara = () => {
    if (para.length) {
      blocks.push({ type: 'p', inline: parseInline(para.join('\n')) });
      para = [];
    }
  };
  const flushList = () => {
    if (list) {
      blocks.push(list);
      list = null;
    }
  };
  for (let n = 0; n < lines.length; n += 1) {
    const line = lines[n];
    if (FENCE.test(line.trim())) {
      flushPara();
      flushList();
      const fence = line.trim().slice(0, 3);
      const code = [];
      n += 1;
      while (n < lines.length && !lines[n].trim().startsWith(fence)) {
        code.push(lines[n]);
        n += 1;
      }
      blocks.push({ type: 'code', text: code.join('\n') });
      continue;
    }
    if (line.trim() === '') {
      flushPara();
      flushList();
      continue;
    }
    const h = HEADING.exec(line);
    if (h) {
      flushPara();
      flushList();
      blocks.push({ type: 'h', level: h[1].length, inline: parseInline(h[2]) });
      continue;
    }
    if (HR.test(line.trim())) {
      flushPara();
      flushList();
      blocks.push({ type: 'hr' });
      continue;
    }
    const ol = OL.exec(line);
    const ul = ol ? null : UL.exec(line);
    if (ol || ul) {
      flushPara();
      const kind = ol ? 'ol' : 'ul';
      if (!list || list.type !== kind) {
        flushList();
        list = { type: kind, start: ol ? Number(ol[1]) : 1, items: [] };
      }
      list.items.push({ lines: [ol ? ol[2] : ul[1]] });
      continue;
    }
    if (list && /^[ \t]+\S/.test(line)) {
      // continuation of the previous list item
      list.items[list.items.length - 1].lines.push(line.trim());
      continue;
    }
    flushList();
    para.push(line);
  }
  flushPara();
  flushList();
  for (const b of blocks) {
    if (b.type === 'ul' || b.type === 'ol') b.items = b.items.map((it) => parseInline(it.lines.join('\n')));
  }
  return { ok: true, blocks };
}

function inlineText(nodes) {
  return nodes.map((n) => (n.t === 'text' || n.t === 'code' ? n.v : n.t === 'br' ? '\n' : inlineText(n.c))).join('');
}

/** The text a reader sees in the rendered output (list numbers and bullets excluded), for the equality test against the raw text. */
export function plainText(blocks) {
  return blocks.map((b) => {
    if (b.type === 'h' || b.type === 'p') return inlineText(b.inline);
    if (b.type === 'ul' || b.type === 'ol') return b.items.map(inlineText).join('\n');
    if (b.type === 'code') return b.text;
    return '';
  }).filter(Boolean).join('\n');
}

/** Raw text with Markdown markers removed and whitespace collapsed. stripMarkers(raw) must equal collapse(plainText(parse(raw))). */
export function stripMarkers(raw) {
  const r = parseMarkdown(raw);
  return collapse(r.ok ? plainText(r.blocks) : String(raw));
}

export function collapse(s) {
  return String(s).replace(/\s+/g, ' ').trim();
}

/**
 * Turn blocks into React elements. `h` is React.createElement (passed in so this module has no React import).
 * `headingBase`: the heading level a single `#` maps to (the notice sits under an h2, so the default is 3).
 */
export function renderMarkdown(blocks, h, { headingBase = 3 } = {}) {
  const inline = (nodes) => nodes.map((n, i) => {
    switch (n.t) {
      case 'text': return n.v;
      case 'code': return h('code', { key: i }, n.v);
      case 'b': return h('strong', { key: i }, inline(n.c));
      case 'i': return h('em', { key: i }, inline(n.c));
      case 'br': return h('br', { key: i });
      default: return null;
    }
  });
  return blocks.map((b, i) => {
    switch (b.type) {
      case 'h': return h(`h${Math.min(6, headingBase + b.level - 1)}`, { key: i, className: 'md-heading' }, inline(b.inline));
      case 'p': return h('p', { key: i }, inline(b.inline));
      case 'ul': return h('ul', { key: i }, b.items.map((it, j) => h('li', { key: j }, inline(it))));
      case 'ol': return h('ol', { key: i, start: b.start !== 1 ? b.start : undefined }, b.items.map((it, j) => h('li', { key: j }, inline(it))));
      case 'code': return h('pre', { key: i, className: 'lab-verbatim' }, h('code', null, b.text));
      case 'hr': return h('hr', { key: i });
      default: return null;
    }
  });
}
