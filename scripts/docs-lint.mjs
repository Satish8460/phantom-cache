#!/usr/bin/env node
// docs-lint: REPORT-ONLY documentation checker for PhantomCache (Node built-ins only).
// It never writes, rewrites or fixes any repo file. The only file it may write is the
// one named by --baseline FILE (an explicit user request).
//
// Usage: node scripts/docs-lint.mjs [--json] [--strict] [--config FILE] [--baseline FILE]
//                                   [--new-only BASELINE_FILE | --new-only --baseline-in FILE]
//                                   [--limit N] [--today YYYY-MM-DD] [--root DIR]
// Exit: 0 always, except 2 on tool/config failure, or 1 with --strict when any error-level
// finding remains (after --new-only filtering).
// Secret-adjacent findings never print the matched text: only file, line and rule.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// ---------- args ----------
const argv = process.argv.slice(2);
const flag = (n) => argv.includes(n);
const opt = (n) => { const i = argv.indexOf(n); return i >= 0 && i + 1 < argv.length && !argv[i + 1].startsWith('--') ? argv[i + 1] : null; };
const ROOT = path.resolve(opt('--root') || path.join(path.dirname(fileURLToPath(import.meta.url)), '..'));
const CONFIG = path.resolve(opt('--config') || path.join(ROOT, 'docs', '.doclint.json'));
const TODAY = opt('--today') || new Date().toISOString().slice(0, 10);
const LIMIT = Number(opt('--limit') || 25);
const fail = (m) => { console.error('docs-lint: ' + m); process.exit(2); };

let cfg;
try { cfg = JSON.parse(fs.readFileSync(CONFIG, 'utf8')); } catch (e) { fail(`cannot read config ${CONFIG}: ${e.message}`); }

// ---------- helpers ----------
const posix = (p) => p.split(path.sep).join('/');
function expandBraces(g) {
  const m = g.match(/\{([^{}]*)\}/);
  return m ? m[1].split(',').flatMap((a) => expandBraces(g.slice(0, m.index) + a + g.slice(m.index + m[0].length))) : [g];
}
function globToRe(g) {
  const alts = expandBraces(g).map((x) => {
    let r = '';
    for (let i = 0; i < x.length; i++) {
      const c = x[i];
      if (c === '*') {
        if (x[i + 1] === '*') { i++; if (x[i + 1] === '/') { i++; r += '(?:.*/)?'; } else r += '.*'; } else r += '[^/]*';
      } else if (c === '?') r += '[^/]';
      else r += c.replace(/[.+^$()|[\]\\]/g, '\\$&');
    }
    return r;
  });
  return new RegExp('^(?:' + alts.join('|') + ')$');
}
const compile = (list) => list.map((e) => ({ ...(typeof e === 'string' ? { glob: e } : e), re: globToRe(typeof e === 'string' ? e : e.glob) }));
const excludeRes = compile(cfg.scanExclude || []);
const classifyRes = compile(cfg.classify || []);
const indexRes = compile(cfg.indexFiles || []);
const dupIgnore = new Set(cfg.dupBasenameIgnore || []);

// ---------- scan ----------
const files = [];
(function walk(dir) {
  let ents;
  try { ents = fs.readdirSync(dir, { withFileTypes: true }); } catch (e) { fail(`cannot read dir ${dir}: ${e.message}`); }
  ents.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  for (const e of ents) {
    if (e.isSymbolicLink()) continue;
    const abs = path.join(dir, e.name);
    const rel = posix(path.relative(ROOT, abs));
    if (e.isDirectory()) {
      if (excludeRes.some((x) => x.re.test(rel + '/x') || x.re.test(rel))) continue;
      walk(abs);
    } else if (e.isFile() && /\.md$/i.test(e.name) && !excludeRes.some((x) => x.re.test(rel))) files.push(rel);
  }
})(ROOT);
const fileSet = new Set(files);

const findings = [];
const add = (rule, level, file, line, detail) => findings.push({ rule, level, file, line: line || 0, detail: detail || '' });

// classification
const modeOf = {}, expiredSeen = new Set();
for (const f of files) {
  const hit = classifyRes.find((x) => x.re.test(f));
  modeOf[f] = hit ? hit.mode : cfg.defaultMode || 'current';
  if (hit && hit.expires && hit.expires < TODAY && !expiredSeen.has(hit.glob)) {
    expiredSeen.add(hit.glob);
    add('allowlist-expired', 'warn', 'docs/.doclint.json', 0, `classify glob ${hit.glob} expired ${hit.expires}`);
  }
}
for (const x of cfg.ignoreAbsPatterns || []) {
  if (x.expires && x.expires < TODAY) add('allowlist-expired', 'warn', 'docs/.doclint.json', 0, `ignore pattern ${x.pattern} expired ${x.expires}`);
}

// ---------- per-file parsing ----------
function parse(text) {
  const raw = text.replace(/^﻿/, '').split(/\r?\n/);
  const plain = []; // fenced lines blanked, line numbers preserved
  let fence = null;
  for (const l of raw) {
    const m = l.match(/^\s*(`{3,}|~{3,})/);
    if (fence) { if (m && m[1][0] === fence[0] && m[1].length >= fence.length) fence = null; plain.push(''); }
    else if (m) { fence = m[1]; plain.push(''); }
    else plain.push(l);
  }
  return { raw, plain };
}
const slugCache = new Map();
function slugsOf(rel) {
  if (slugCache.has(rel)) return slugCache.get(rel);
  const set = new Set(), seen = new Map();
  const doc = docs.get(rel);
  if (doc) for (const l of doc.plain) {
    const m = l.match(/^#{1,6}\s+(.*?)\s*#*\s*$/);
    if (!m) continue;
    let t = m[1].replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/<[^>]+>/g, '').replace(/[*`~]/g, '');
    let s = t.toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').trim().replace(/\s/g, '-');
    const n = seen.get(s) || 0; seen.set(s, n + 1);
    set.add(n ? `${s}-${n}` : s);
  }
  slugCache.set(rel, set);
  return set;
}

const docs = new Map();
for (const f of files) {
  try { docs.set(f, { ...parse(fs.readFileSync(path.join(ROOT, f), 'utf8')), size: fs.statSync(path.join(ROOT, f)).size }); }
  catch (e) { fail(`cannot read ${f}: ${e.message}`); }
}

const DRIVE_RE = /(?<![A-Za-z0-9])[A-Za-z]:(?:\\|\/(?![\/]))[^\s`'"<>)\]|]*/g;
const FILEURL_RE = /file:\/\/\/[^\s`'"<>)\]|]*/gi;
const normAbs = (s) => s.toLowerCase().replace(/\//g, '\\');
const ignoreAbs = (cfg.ignoreAbsPatterns || []).map((x) => normAbs(x.pattern));
const secretRe = new RegExp('\\b(' + (cfg.secretWords || ['salt', 'secret', 'key', 'password']).join('|') + ')', 'i');
const hexRe = new RegExp('(?<![0-9A-Fa-f])[0-9A-Fa-f]{' + (cfg.secretHexMinLength || 64) + ',}(?![0-9A-Fa-f])');
const vocab = cfg.statusVocabulary || [];
const maxBytes = (cfg.maxSizeKB || 60) * 1024;
const outLinks = new Map(); // file -> Set of md files it links to / mentions by path

function resolveRef(from, target, strict) {
  const cands = [path.posix.normalize(path.posix.join(path.posix.dirname(from), target))];
  if (!strict) cands.push(path.posix.normalize(target));
  for (const c of cands) if (!c.startsWith('..') && (fileSet.has(c) || fs.existsSync(path.join(ROOT, c)))) return c;
  return null;
}

for (const f of files) {
  const { raw, plain, size } = docs.get(f);
  const mode = modeOf[f];
  const isCur = mode === 'current', isHist = mode === 'historical', isArch = mode === 'archived', isFrozen = mode === 'frozen';
  const links = new Set(); outLinks.set(f, links);

  // secret-adjacent: all modes, raw text (fenced code included). NEVER prints the match.
  raw.forEach((l, i) => {
    if (!secretRe.test(l)) return;
    DRIVE_RE.lastIndex = 0;
    if (DRIVE_RE.test(l) || hexRe.test(l)) add('secret-adjacent', 'error', f, i + 1, 'drive path or long hex near secret-adjacent word (value withheld)');
  });
  if (isFrozen) continue;

  // size
  if (size > maxBytes) add('size', 'warn', f, 0, `${Math.round(size / 1024)} KB > ${cfg.maxSizeKB} KB`);

  // links + anchors (current, historical, archived)
  plain.forEach((line, i) => {
    const noCode = line.replace(/`[^`]*`/g, (m) => ' '.repeat(m.length));
    for (const m of noCode.matchAll(/!?\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)/g)) {
      let t = m[1];
      if (/^(https?:|mailto:|file:|data:|tel:)/i.test(t)) continue;
      let [p, anchor] = t.split('#');
      p = p.split('?')[0];
      try { p = decodeURIComponent(p); } catch { /* keep raw */ }
      if (!/^[\w.\-/ ~@%+()]*$/.test(p) || /[{}]|\.\.\.|\\/.test(p)) continue; // not a path (regex text, braces)
      let target;
      if (p === '') target = f;
      else { target = resolveRef(f, p, true); if (target === null) { if (!p.startsWith('/')) add('link-broken', 'error', f, i + 1, t); continue; } }
      if (target === null) continue;
      if (/\.md$/i.test(target)) links.add(target);
      if (anchor && /\.md$/i.test(target) && !/^L\d+/.test(anchor)) {
        let a = anchor; try { a = decodeURIComponent(a); } catch { /* raw */ }
        if (!slugsOf(target).has(a.toLowerCase())) add('anchor-broken', 'error', f, i + 1, `${p || '(self)'}#${anchor}`);
      }
    }
    // backtick path mentions: feed the orphan graph; missing ones reported below for current docs
    for (const m of line.matchAll(/`([^`\s]+)`/g)) {
      let t = m[1].replace(/[:#]\d+(?:-\d+)?$/, '').replace(/#L\d+.*$/, '');
      if (!(cfg.pathRefPrefixes || []).some((p) => t.startsWith(p)) || /[*{}<>$|]|\.\.\./.test(t)) continue;
      const r = resolveRef(f, t);
      if (r) { if (/\.md$/i.test(r)) links.add(r); }
      else if (isCur && /\.[A-Za-z0-9]+$/.test(t)) add('path-ref-missing', 'warn', f, i + 1, t);
    }
  });

  // absolute machine paths (current only)
  if (isCur) {
    plain.forEach((line, i) => {
      const hits = [...(line.match(FILEURL_RE) || []), ...(line.match(DRIVE_RE) || [])];
      const bad = hits.filter((h) => !ignoreAbs.some((p) => normAbs(h).includes(p)));
      if (bad.length) add('abs-path', 'error', f, i + 1, /^file:/i.test(bad[0]) ? 'file:/// URL' : 'drive path');
    });
  }

  // header rules
  const head = raw.slice(0, 40);
  const statusLine = head.findIndex((l) => /^[\s>*_-]*status[\s*_]*:/i.test(l));
  if (isCur) {
    if (!plain.some((l) => /^#\s+\S/.test(l))) add('h1-missing', 'error', f, 1, 'no H1');
    if (statusLine < 0) add('status-missing', 'error', f, 1, 'no Status header in first 40 lines');
    else {
      const v = head[statusLine].replace(/^[\s>*_-]*status[\s*_]*:[\s*_`]*/i, '');
      const w = (v.match(/^[A-Za-z]+/) || [''])[0];
      if (vocab.includes(w)) { /* ok */ }
      else if (vocab.some((x) => x.toLowerCase() === w.toLowerCase())) add('status-case', 'warn', f, statusLine + 1, `"${w}" is a case variant`);
      else add('status-vocab', 'error', f, statusLine + 1, `"${v.slice(0, 40).trim()}" not in controlled vocabulary`);
    }
    const rv = head.findIndex((l) => /last[\s_-]*reviewed[\s*_]*:/i.test(l));
    if (rv < 0) add('reviewed-missing', 'error', f, 1, 'no Last reviewed header');
    else {
      const d = head[rv].replace(/^.*last[\s_-]*reviewed[\s*_]*:[\s*_`]*/i, '').match(/^\d{4}-\d{2}-\d{2}(?!\d)/);
      if (!d || isNaN(Date.parse(d[0]))) add('reviewed-format', 'error', f, rv + 1, 'Last reviewed is not an ISO date');
    }
  } else if (isHist) {
    const markers = cfg.historicalNoticeMarkers || ['Notice (', 'Historical'];
    if (!head.some((l) => markers.some((m) => l.includes(m)))) add('notice-missing', 'warn', f, 1, 'historical doc without a notice line in first 40 lines');
  } else if (isArch) {
    const v = statusLine >= 0 ? (head[statusLine].replace(/^[\s>*_-]*status[\s*_]*:[\s*_`]*/i, '').match(/^[A-Za-z]+/) || [''])[0] : '';
    if (v !== 'Archived' && v !== 'Superseded') add('status-vocab', 'error', f, statusLine + 1 || 1, 'archived doc must have Status Archived or Superseded');
  }

  // fact drift + informational spreads (current only)
  if (isCur) {
    const tb = cfg.facts && cfg.facts.test_baseline;
    if (tb && f !== tb.source) {
      plain.forEach((line, i) => {
        if (line.includes('<!-- fact:test_baseline')) return;
        for (const p of tb.patterns) for (const m of line.matchAll(new RegExp(p.re, 'g'))) {
          if (Number(m[1]) !== tb[p.field]) add('fact-drift', tb.level || 'warn', f, i + 1, `${p.field}=${m[1]}, canonical ${tb[p.field]}`);
        }
      });
    }
  }
}

// ---------- corpus-level rules ----------
const cur = files.filter((f) => modeOf[f] === 'current');

// filename convention (current only)
const CONV = { lower_snake: /^[a-z0-9]+(?:_[a-z0-9]+)*\.md$/, 'lower-kebab': /^[a-z0-9]+(?:-[a-z0-9]+)*\.md$/, UPPER_SNAKE: /^[A-Z0-9]+(?:_[A-Z0-9]+)*\.md$/ };
const fnRules = (cfg.filenameConventions || []).slice().sort((a, b) => b.dir.length - a.dir.length);
for (const f of cur) {
  const base = path.posix.basename(f);
  if (base === 'README.md') continue;
  const r = fnRules.find((x) => f.startsWith(x.dir) && !f.slice(x.dir.length).includes('/')) || fnRules.find((x) => x.dir === path.posix.dirname(f) + '/');
  if (!r) continue;
  const re = r.regex ? new RegExp(r.regex) : CONV[r.convention];
  if (re && !re.test(base)) add('filename-convention', 'warn', f, 0, `expected ${r.label || r.convention} in ${r.dir}`);
}

// duplicate basenames (current only)
const byBase = new Map();
for (const f of cur) { const b = path.posix.basename(f).toLowerCase(); if (!dupIgnore.has(path.posix.basename(f))) (byBase.get(b) || byBase.set(b, []).get(b)).push(f); }
for (const [b, list] of byBase) if (list.length > 1) for (const f of list) add('dup-basename', 'warn', f, 0, `${b} also in ${list.length - 1} other location(s)`);

// orphans: BFS from index files over link graph, limited hops (current only)
const hops = new Map();
let frontier = files.filter((f) => indexRes.some((x) => x.re.test(f)));
frontier.forEach((f) => hops.set(f, 0));
for (let h = 1; h <= (cfg.orphanHops || 1); h++) {
  const next = [];
  for (const f of frontier) for (const t of outLinks.get(f) || []) if (!hops.has(t)) { hops.set(t, h); next.push(t); }
  frontier = next;
}
for (const f of cur) if (!hops.has(f)) add('orphan', 'warn', f, 0, `not linked from an index file within ${cfg.orphanHops || 1} hop(s)`);

// informational: SEC / claim status spread across current files
function spread(rule, idRe, lineFilter, statusRe) {
  const m = new Map(); // id -> {statuses:Set, files:Set}
  for (const f of cur) for (const l of docs.get(f).plain) {
    if (lineFilter && !lineFilter.test(l)) continue;
    const ids = l.match(idRe); if (!ids) continue;
    const st = [...new Set((l.match(statusRe) || []).map((s) => s.toUpperCase()))];
    if (!st.length) continue;
    for (const id of new Set(ids)) { const e = m.get(id) || m.set(id, { s: new Set(), f: new Set() }).get(id); st.forEach((s) => e.s.add(s)); e.f.add(f); }
  }
  for (const [id, e] of [...m].sort()) if (e.s.size > 1 && e.f.size > 1) add(rule, 'info', '(corpus)', 0, `${id}: ${[...e.s].sort().join('/')} across ${e.f.size} files`);
}
spread('sec-status-spread', /SEC-[A-Z0-9]+-\d+/g, null, /\b(OPEN|FIXED|PARTIAL|ACCEPTED|CLOSED|DEFERRED|RESOLVED)\b/gi);
spread('claim-status-spread', /^\|\s*\**C\d{1,2}\b/g, /^\|\s*\**C\d{1,2}\b/, /\b(SUPPORTED|UNVERIFIED|NOT SUPPORTED|NOT TESTABLE|REFUTED|NARROWED|HYPOTHESIS|IMPLEMENTED|EVALUATED)\b/gi);

// fact source check
const tbSrc = cfg.facts && cfg.facts.test_baseline && cfg.facts.test_baseline.source;
if (tbSrc) {
  if (!fileSet.has(tbSrc)) add('fact-source-missing', 'info', tbSrc, 0, 'canonical source does not exist yet; registry values are unverified');
  else if (!docs.get(tbSrc).raw.join('\n').includes(String(cfg.facts.test_baseline.passed))) add('fact-source-mismatch', 'warn', tbSrc, 0, `source does not mention canonical passed=${cfg.facts.test_baseline.passed}`);
}

// ---------- sort, baseline ----------
findings.sort((a, b) => (a.file < b.file ? -1 : a.file > b.file ? 1 : a.line - b.line || (a.rule < b.rule ? -1 : a.rule > b.rule ? 1 : a.detail < b.detail ? -1 : a.detail > b.detail ? 1 : 0)));
const keyOf = (x) => `${x.rule}|${x.file}|${x.detail}`;
const countKeys = (list) => list.reduce((m, x) => m.set(keyOf(x), (m.get(keyOf(x)) || 0) + 1), new Map());

const baselineWrite = opt('--baseline');
if (baselineWrite) {
  const obj = Object.fromEntries([...countKeys(findings)].sort());
  try { fs.writeFileSync(baselineWrite, JSON.stringify({ version: 1, counts: obj }, null, 1)); } catch (e) { fail(`cannot write baseline: ${e.message}`); }
}
let shown = findings;
if (flag('--new-only')) {
  const bf = opt('--new-only') || opt('--baseline-in');
  if (!bf) fail('--new-only needs a baseline file: --new-only FILE');
  let base; try { base = JSON.parse(fs.readFileSync(bf, 'utf8')).counts; } catch (e) { fail(`cannot read baseline ${bf}: ${e.message}`); }
  const used = new Map();
  shown = findings.filter((x) => { const k = keyOf(x), n = (used.get(k) || 0) + 1; used.set(k, n); return n > (base[k] || 0); });
}

// ---------- output ----------
const byRule = {};
for (const x of shown) { const r = byRule[x.rule] || (byRule[x.rule] = { level: x.level, count: 0, files: new Map() }); r.count++; r.files.set(x.file, (r.files.get(x.file) || 0) + 1); }
const modeCounts = {}; files.forEach((f) => (modeCounts[modeOf[f]] = (modeCounts[modeOf[f]] || 0) + 1));
const errors = shown.filter((x) => x.level === 'error').length;

if (flag('--json')) {
  const rules = Object.fromEntries(Object.entries(byRule).sort().map(([k, v]) => [k, { level: v.level, count: v.count, topFiles: [...v.files].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)).slice(0, 10) }]));
  console.log(JSON.stringify({ tool: 'docs-lint', filesScanned: files.length, modes: modeCounts, totals: { error: errors, warn: shown.filter((x) => x.level === 'warn').length, info: shown.filter((x) => x.level === 'info').length }, rules, findings: shown }, null, 1));
} else {
  console.log(`docs-lint (report-only) root=${ROOT}`);
  console.log(`started ${new Date().toISOString()}  files scanned: ${files.length}  modes: ${JSON.stringify(modeCounts)}`);
  for (const [rule, v] of Object.entries(byRule).sort()) {
    console.log(`\n[${v.level}] ${rule}: ${v.count} finding(s) in ${v.files.size} file(s)`);
    for (const x of shown.filter((y) => y.rule === rule).slice(0, LIMIT)) console.log(`  ${x.file}${x.line ? ':' + x.line : ''}${x.rule === 'secret-adjacent' ? '' : x.detail ? '  ' + x.detail : ''}`);
    if (v.count > LIMIT) console.log(`  ... ${v.count - LIMIT} more (use --json or --limit N)`);
  }
  console.log(`\nTOTAL: ${errors} error, ${shown.filter((x) => x.level === 'warn').length} warn, ${shown.filter((x) => x.level === 'info').length} info`);
  console.log(`finished ${new Date().toISOString()}`);
}
process.exit(flag('--strict') && errors > 0 ? 1 : 0);
