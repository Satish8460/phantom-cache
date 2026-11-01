// Small static accessibility lint over server-rendered HTML. It is NOT a replacement for axe-core, a screen
// reader pass, or visual review (docs/design/ui_audit.md section 5 lists what it cannot see).

const VOID = new Set(['br', 'hr', 'img', 'input', 'meta', 'link', 'source', 'wbr']);
const VALID_ROLES = new Set(['img', 'status', 'alert', 'note', 'group', 'list', 'listitem', 'region', 'navigation', 'main', 'banner', 'contentinfo', 'tablist', 'tab', 'tabpanel', 'dialog', 'alertdialog', 'progressbar']);
const INTERACTIVE_NEEDS_NAME = new Set(['button', 'a']);

export function parseHtml(html) {
  const nodes = [];
  const stack = [];
  const re = /<(\/?)([a-zA-Z][a-zA-Z0-9]*)((?:\s+[^<>]*?)?)(\/?)>|([^<]+)/g;
  let m;
  while ((m = re.exec(html))) {
    if (m[5] !== undefined) {
      const text = m[5].replace(/&amp;|&lt;|&gt;|&quot;|&#x27;/g, ' ').trim();
      for (const n of stack) n.text += ` ${text}`;
      continue;
    }
    const [, closing, tagRaw, attrStr, selfClose] = m;
    const tag = tagRaw.toLowerCase();
    if (closing) {
      for (let i = stack.length - 1; i >= 0; i -= 1) {
        if (stack[i].tag === tag) {
          stack.length = i;
          break;
        }
      }
      continue;
    }
    const attrs = {};
    for (const a of attrStr.matchAll(/([:@\w-]+)(?:="([^"]*)")?/g)) attrs[a[1]] = a[2] === undefined ? '' : a[2];
    const node = { tag, attrs, text: '', parent: stack[stack.length - 1] || null, children: [] };
    if (node.parent) node.parent.children.push(node);
    nodes.push(node);
    if (!selfClose && !VOID.has(tag)) stack.push(node);
  }
  return nodes;
}

const hasAncestor = (node, pred) => {
  for (let p = node.parent; p; p = p.parent) if (pred(p)) return true;
  return false;
};

const descendants = (node) => node.children.flatMap((c) => [c, ...descendants(c)]);

/** Returns a list of {rule, detail}. Empty list = clean. */
export function lintHtml(html, { requireH1 = true } = {}) {
  const nodes = parseHtml(html);
  const issues = [];
  const add = (rule, detail) => issues.push({ rule, detail });

  const ids = new Map();
  for (const n of nodes) if (n.attrs.id) ids.set(n.attrs.id, (ids.get(n.attrs.id) || 0) + 1);
  for (const [id, c] of ids) if (c > 1) add('unique-id', `id "${id}" used ${c} times`);

  const refAttr = (name) => nodes.filter((n) => n.attrs[name] !== undefined).forEach((n) => {
    for (const ref of n.attrs[name].split(/\s+/).filter(Boolean)) {
      if (!ids.has(ref)) add('aria-ref', `${n.tag}[${name}] points at missing id "${ref}"`);
    }
  });
  refAttr('aria-labelledby');
  refAttr('aria-describedby');

  const headings = nodes.filter((n) => /^h[1-6]$/.test(n.tag));
  const h1s = headings.filter((h) => h.tag === 'h1');
  if (requireH1 && h1s.length !== 1) add('single-h1', `expected exactly one h1, found ${h1s.length}`);
  let prev = 0;
  for (const h of headings) {
    const level = Number(h.tag[1]);
    if (prev && level > prev + 1) add('heading-order', `${h.tag} follows h${prev}`);
    if (!h.text.trim()) add('heading-empty', `${h.tag} has no text`);
    prev = level;
  }

  for (const n of nodes) {
    if (n.attrs.tabindex !== undefined && Number(n.attrs.tabindex) > 0) add('tabindex', 'positive tabindex');
    if (n.attrs.role && !VALID_ROLES.has(n.attrs.role)) add('role', `unexpected role "${n.attrs.role}"`);
    if (n.attrs.style && /(^|;)\s*(color|background(-color)?)\s*:/.test(n.attrs.style)) add('inline-colour', `${n.tag} has inline colour style`);
    if (INTERACTIVE_NEEDS_NAME.has(n.tag)) {
      const name = `${n.text} ${n.attrs['aria-label'] || ''}`.trim();
      if (!name) add('accessible-name', `${n.tag} has no accessible name`);
      if (n.tag === 'a' && n.attrs.href === undefined) add('link-href', 'a without href');
    }
    if (n.tag === 'svg') {
      const hidden = n.attrs['aria-hidden'] === 'true';
      const labelled = n.attrs.role === 'img' && (n.attrs['aria-label'] || n.attrs['aria-labelledby']);
      if (!hidden && !labelled) add('svg-name', 'svg is neither aria-hidden nor a labelled img');
      if (n.attrs.focusable === undefined && hidden) { /* React emits focusable=false on our icons */ }
    }
    if (n.tag === 'img' && n.attrs.alt === undefined) add('img-alt', 'img without alt');
    if (n.tag === 'th' && !n.attrs.scope) add('th-scope', 'th without scope');
    if (n.tag === 'table') {
      const hasCaption = descendants(n).some((c) => c.tag === 'caption');
      if (!hasCaption && !n.attrs['aria-label'] && !n.attrs['aria-labelledby']) add('table-name', 'table has no caption or label');
    }
    if (n.tag === 'details' && !n.children.some((c) => c.tag === 'summary')) add('details-summary', 'details without summary');
    if (n.tag === 'section' && !n.attrs['aria-labelledby'] && !n.attrs['aria-label']) add('region-name', 'section without an accessible name');
    if (n.attrs['aria-hidden'] === 'true' && hasAncestor(n, (p) => p.attrs['aria-hidden'] === 'true')) continue;
  }

  // Domain rule: every displayed metric carries a provenance badge.
  for (const n of nodes.filter((x) => x.attrs['data-metric'])) {
    if (!descendants(n).some((c) => c.attrs['data-provenance'])) add('provenance', `metric "${n.attrs['data-metric']}" has no provenance badge`);
  }
  return issues;
}
