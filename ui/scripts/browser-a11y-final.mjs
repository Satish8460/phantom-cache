// UI/UX remediation, TRACK 2 (accessibility) final verification. Real Chrome over CDP, Node built-ins only. Observe-only.
//   node ui/scripts/browser-a11y-final.mjs --phase prep|matrix|ax|kbd|media|scale|prov|disconnect [--vp 390] [--evidence <dir>]
// Each phase launches its own throwaway Chrome (D:/Temp profile), relaunching every 3 cold loads; route changes inside a
// session use the hash router (no reload). Results go to <evidence>/<phase>*.json. A pass here is "no failure observed
// by this script", not conformance, and does not replace an assistive-technology pass.
import { writeFileSync, readFileSync, existsSync, mkdirSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { join } from 'node:path';
import { Browser, Results, sleep, pidAlive } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const EV = args.evidence || 'D:/Personal/phantom-cache/docs/design/remediation/a11y_evidence';
const PHASE = args.phase;
const UI = 'http://localhost:3200';
const GW = 'http://localhost:8285';
const BASE_URL = `${UI}/?gateway=${encodeURIComponent(GW)}`;
mkdirSync(join(EV, 'logs'), { recursive: true });
const R = new Results(join(EV, `${PHASE}${args.vp ? '_' + args.vp : ''}.results.json`));
const M6 = 'm6_sealed_2026-10-29T234845Z';
const SB = 'scenario_b.20261030T003932Z';
const ids = existsSync(join(EV, 'runids.json')) ? JSON.parse(readFileSync(join(EV, 'runids.json'), 'utf8')) : {};
const ROUTES = [
  ['overview', '#/overview'], ['traffic', '#/traffic'], ['cache', '#/cache'], ['freshness', '#/freshness'], ['origin', '#/origin'],
  ['cluster', '#/cluster'], ['intelligence', '#/intelligence'], ['simulation', '#/simulation'],
  ['lab_home', '#/lab'], ['lab_evidence', '#/lab/evidence'], ['lab_evidence_m6', `#/lab/evidence/${M6}`], ['lab_evidence_sb', `#/lab/evidence/${SB}`],
  ['lab_run_form', '#/lab/run'], ['lab_runs', '#/lab/runs'],
  ...(ids.sb ? [['lab_run_sb', `#/lab/runs/${ids.sb}`]] : []), ...(ids.t1 ? [['lab_run_t1', `#/lab/runs/${ids.t1}`]] : []),
];
const VPS = [320, 390, 640, 820, 1280];
const vh = (w) => (w >= 820 ? 900 : 844);
const NOW = () => new Date().toISOString();

// ---------------------------------------------------------------- session management (relaunch every 3 cold loads)
class Session {
  constructor(label) { this.label = label; this.cold = 0; this.b = null; this.n = 0; this.state = {}; }
  async ensure() {
    if (this.b && this.cold < 3) return this.b;
    if (this.b) { this.b.saveLogs(`${PHASE}_${this.label}_${this.n}`); await this.b.close(); }
    this.n++; this.cold = 0;
    this.b = await new Browser({ evidenceDir: EV, label: `${PHASE}${this.n}` }).launch();
    writeFileSync(join(EV, 'logs', 'chrome_pids.txt'), `${PHASE} ${this.label} #${this.n} pid ${this.b.pid} ${NOW()}\n`, { flag: 'a' });
    await this.restore();
    return this.b;
  }
  async restore() {
    const s = this.state; const b = this.b;
    if (s.vp) await b.viewport(s.vp.w, s.vp.h, false, s.vp.d || 1);
    if (s.media) await b.media(s.media);
  }
  async setVp(w, h, d = 1) { this.state.vp = { w, h, d }; const b = await this.ensure(); await b.viewport(w, h, false, d); }
  async setMedia(f) { this.state.media = f; const b = await this.ensure(); await b.media(f); }
  async load(route = '', settle = 1800) {
    const b = await this.ensure(); this.cold++;
    await b.goto(`${BASE_URL}${route}`, settle); await this.settled(b); return b;
  }
  async go(hash, settle = 1100) {
    const b = await this.ensure();
    await b.eval(`location.hash = ${JSON.stringify(hash)}`); await sleep(settle); await this.settled(b); return b;
  }
  async settled(b) { await b.waitFor(`!document.querySelector('.spinner, [data-state=loading]')`, 6000, 200); await sleep(250); }
  async end() { if (this.b) { this.b.saveLogs(`${PHASE}_${this.label}_${this.n}`); const gone = await this.b.close(); R.add(`browser.${this.label}.${this.n}`, gone ? 'PASS' : 'FAIL', { pid: this.b.pid, gone, version: this.b.version }); this.b = null; } }
}
const jshot = async (b, name, dir, full = false, fmt = 'jpeg') => {
  const d = join(EV, dir); mkdirSync(d, { recursive: true });
  const s = await b.send('Page.captureScreenshot', fmt === 'png' ? { format: 'png', captureBeyondViewport: full } : { format: 'jpeg', quality: 62, captureBeyondViewport: full });
  writeFileSync(join(d, name + (fmt === 'png' ? '.png' : '.jpg')), Buffer.from(s.data, 'base64')); return `${dir}/${name}.${fmt === 'png' ? 'png' : 'jpg'}`;
};
const key = (b, k, vk, extra = {}) => b.key(k, { code: extra.code || k, vk });
const ENTER = async (b) => { await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r', unmodifiedText: '\r' }); await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); };
const SPACE = (b) => b.key(' ', { code: 'Space', vk: 32 });
const ESC = (b) => b.key('Escape', { code: 'Escape', vk: 27 });
const ARROW = (b, d) => b.key('Arrow' + d, { code: 'Arrow' + d, vk: { Left: 37, Up: 38, Right: 39, Down: 40 }[d] });
const inj = (fn, ...a) => `(function(){ const colorTools = ${colorTools.toString()}; return (${fn.toString()})(${a.map((x) => JSON.stringify(x)).join(',')}); })()`;
const save = (name, obj) => writeFileSync(join(EV, name), JSON.stringify(obj, null, 1));

// ---------------------------------------------------------------- page-side functions (stringified into the page)
function colorTools() {
  const cvs = document.createElement('canvas'); cvs.width = cvs.height = 1; const ctx = cvs.getContext('2d', { willReadFrequently: true });
  const cache = new Map();
  const parse = (s) => { if (cache.has(s)) return cache.get(s); ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = '#000'; ctx.fillStyle = s; ctx.fillRect(0, 0, 1, 1); const d = ctx.getImageData(0, 0, 1, 1).data; const c = { r: d[0], g: d[1], b: d[2], a: d[3] / 255 }; cache.set(s, c); return c; };
  const lin = (c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); };
  const L = (c) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  const over = (f, bg) => ({ r: f.r * f.a + bg.r * (1 - f.a), g: f.g * f.a + bg.g * (1 - f.a), b: f.b * f.a + bg.b * (1 - f.a), a: 1 });
  const ratio = (a, b) => { const l1 = L(a), l2 = L(b); return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05); };
  const bgOf = (el) => {
    const layers = []; let ind = false;
    for (let e = el; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.backgroundImage && cs.backgroundImage !== 'none') { const cols = cs.backgroundImage.startsWith('linear-gradient(') ? (cs.backgroundImage.match(/rgba?\([^)]*\)/g) || []) : []; if (cols.length && cols.every((x) => x === cols[0]) && cs.backgroundImage.replace(/rgba?\([^)]*\)/g, '').replace(/linear-gradient|[(),\s]|to |deg|top|bottom|left|right/g, '') === '') { const gc = parse(cols[0]); if (gc.a > 0) layers.push(gc); if (gc.a >= 1) break; } else { ind = 'background-image on ' + e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30) + ' ' + cs.backgroundImage.slice(0, 40); break; } }
      const c = parse(cs.backgroundColor); if (c.a > 0) layers.push(c); if (c.a >= 1) break;
    }
    let base = { r: 255, g: 255, b: 255, a: 1 };
    if (!layers.length || layers[layers.length - 1].a < 1) { const rb = parse(getComputedStyle(document.documentElement).backgroundColor); const bb = parse(getComputedStyle(document.body).backgroundColor); base = bb.a > 0 ? over(bb, { r: 255, g: 255, b: 255, a: 1 }) : rb.a > 0 ? over(rb, base) : base; }
    let acc = base; for (let i = layers.length - 1; i >= 0; i--) acc = over(layers[i], acc);
    return { color: acc, ind };
  };
  const rgb = (c) => `rgb(${[c.r, c.g, c.b].map(Math.round).join(',')})`;
  const visible = (el) => { const cs = getComputedStyle(el); if (cs.visibility === 'hidden' || cs.display === 'none') return false; const r = el.getBoundingClientRect(); return r.width >= 2 && r.height >= 2; };
  return { parse, L, over, ratio, bgOf, rgb, visible };
}

function pageProbe(opts) {
  const T = colorTools();
  const { parse, over, ratio, bgOf, rgb, visible } = T;
  const nameOf = (el) => {
    const t = (s) => (s || '').replace(/\s+/g, ' ').trim();
    const lb = el.getAttribute('aria-labelledby');
    if (lb) { const s = lb.split(/\s+/).map((i) => document.getElementById(i)?.textContent || '').join(' '); if (t(s)) return t(s); }
    if (el.getAttribute('aria-label')) return t(el.getAttribute('aria-label'));
    if (el.labels && el.labels.length) return t([...el.labels].map((l) => l.textContent).join(' '));
    return t(el.innerText || el.textContent) || t(el.getAttribute('title')) || t(el.getAttribute('placeholder'));
  };
  const out = { w: innerWidth, h: innerHeight };
  // ---- reflow
  const de = document.documentElement;
  out.overflow = { scrollW: de.scrollWidth, bodyScrollW: document.body.scrollWidth, innerW: innerWidth, pageHScroll: de.scrollWidth > innerWidth + 1 };
  if (out.overflow.pageHScroll) out.overflow.widest = [...document.querySelectorAll('body *')].map((e) => [Math.round(e.getBoundingClientRect().right), e.tagName.toLowerCase() + '.' + String(e.className).slice(0, 30)]).sort((a, c) => c[0] - a[0]).slice(0, 4);
  // ---- text contrast
  const c = { checked: 0, indeterminate: 0, indReasons: [], fails: [], min: 99, disabledLow: [] };
  const seen = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n; (n = walker.nextNode());) {
    const txt = n.nodeValue.trim(); if (!txt) continue;
    const el = n.parentElement; if (!el || ['SCRIPT', 'STYLE', 'NOSCRIPT', 'OPTION'].includes(el.tagName) || !visible(el)) continue;
    const cs = getComputedStyle(el);
    let op = 1; for (let e = el; e; e = e.parentElement) op *= parseFloat(getComputedStyle(e).opacity);
    const bg = bgOf(el); if (bg.ind) { c.indeterminate++; if (!c.indReasons.includes(bg.ind)) c.indReasons.push(bg.ind); continue; }
    const fg0 = parse(cs.color); const fg = over({ ...fg0, a: fg0.a * op }, bg.color);
    const r = ratio(fg, bg.color);
    const size = parseFloat(cs.fontSize); const bold = parseInt(cs.fontWeight, 10) >= 700;
    const need = size >= 24 || (size >= 18.66 && bold) ? 3 : 4.5;
    const disabled = !!el.closest('[disabled],[aria-disabled="true"]');
    c.checked++; if (!disabled) c.min = Math.min(c.min, r);
    if (r < need) {
      const k = el.tagName + '|' + String(el.className).slice(0, 40) + '|' + txt.slice(0, 24);
      if (seen.has(k)) continue; seen.add(k);
      const rec = { ratio: +r.toFixed(2), need, text: txt.slice(0, 50), tag: el.tagName.toLowerCase(), cls: String(el.className).slice(0, 50), size, fg: rgb(fg), bg: rgb(bg.color) };
      (disabled ? c.disabledLow : c.fails).push(rec);
    }
  }
  c.min = +c.min.toFixed(2); out.contrast = c;
  // ---- non-text boundaries of controls (1.4.11): best of border-vs-parent and fill-vs-parent
  const nt = { checked: 0, weak: [], native: 0, indeterminate: 0 };
  const seenNt = new Set();
  for (const el of document.querySelectorAll('button, a.btn, a.tab, .seg__btn, input:not([type=hidden]), select, textarea, summary, progress')) {
    if (!visible(el)) continue;
    const cs = getComputedStyle(el);
    if ((el.type === 'checkbox' || el.type === 'radio') && cs.appearance !== 'none') { nt.native++; continue; }
    const par = el.parentElement ? bgOf(el.parentElement) : null; if (!par || par.ind) { nt.indeterminate++; continue; }
    const own = bgOf(el); let best = 0; let how = 'none';
    for (const side of ['Top', 'Right', 'Bottom', 'Left']) {
      if (parseFloat(cs['border' + side + 'Width']) > 0 && cs['border' + side + 'Style'] !== 'none') { const bc = over(parse(cs['border' + side + 'Color']), par.color); const rr = ratio(bc, par.color); if (rr > best) { best = rr; how = 'border'; } }
    }
    if (!own.ind) { const rr = ratio(own.color, par.color); if (rr > best) { best = rr; how = 'fill'; } }
    nt.checked++;
    if (best < 3) { const k = el.tagName + '|' + String(el.className).slice(0, 40); let boundary = !own.ind && own.color && parse(cs.backgroundColor).a > 0; for (const side of ['Top', 'Right', 'Bottom', 'Left']) if (parseFloat(cs['border' + side + 'Width']) > 0 && cs['border' + side + 'Style'] !== 'none' && parse(cs['border' + side + 'Color']).a > 0) boundary = true; if (!boundary) { nt.textOnly = (nt.textOnly || 0) + 1; continue; } if (seenNt.has(k)) continue; seenNt.add(k); nt.weak.push({ tag: el.tagName.toLowerCase(), cls: String(el.className).slice(0, 50), name: nameOf(el).slice(0, 30), best: +best.toFixed(2), how, ariaDisabled: el.getAttribute('aria-disabled') || el.disabled || undefined }); }
  }
  out.nontext = nt;
  // ---- interactive elements: names, sizes
  const sel = 'a[href],button,input:not([type=hidden]),select,textarea,summary,[role=button],[role=link],[role=tab],[tabindex]:not([tabindex="-1"])';
  const els = [...document.querySelectorAll(sel)].filter(visible);
  const rows = els.map((e) => {
    const r = e.getBoundingClientRect(); let w = r.width, h = r.height;
    const lab = e.labels && e.labels[0] ? e.labels[0].getBoundingClientRect() : null;
    const inline = e.tagName === 'A' && getComputedStyle(e).display === 'inline' && !!e.closest('p,li,dd,td,span.lab-hint,.panel__note');
    return { tag: e.tagName.toLowerCase(), type: e.type || undefined, cls: String(e.className).slice(0, 30), name: nameOf(e).slice(0, 36), w: Math.round(w), h: Math.round(h), hitW: lab ? Math.round(Math.max(w, lab.width)) : undefined, hitH: lab ? Math.round(Math.max(h, lab.height)) : undefined, inline, primary: /btn|tab|seg__btn/.test(e.className) || e.tagName === 'BUTTON' || e.tagName === 'SUMMARY' || ['INPUT', 'SELECT', 'TEXTAREA'].includes(e.tagName) || !!e.closest('nav') };
  });
  out.interactive = rows.length;
  out.unnamed = els.filter((e) => !nameOf(e)).map((e) => e.outerHTML.slice(0, 120));
  out.below24 = rows.filter((s) => !s.inline && Math.max(s.w, s.hitW || 0) < 24 || !s.inline && Math.max(s.h, s.hitH || 0) < 24);
  out.below24Inline = rows.filter((s) => s.inline && (s.w < 24 || s.h < 24)).length;
  const minPrimary = opts.minPrimary;
  out.belowPrimary = rows.filter((s) => s.primary && !s.inline && (Math.max(s.h, s.hitH || 0) < minPrimary || Math.max(s.w, s.hitW || 0) < minPrimary)).map((s) => ({ ...s }));
  // ---- structure
  const heads = [...document.querySelectorAll('h1,h2,h3,h4,h5,h6,[role=heading]')].filter((h) => h.getBoundingClientRect().width > 0 || h.closest('.visually-hidden')).map((h) => ({ l: h.getAttribute('aria-level') ? +h.getAttribute('aria-level') : +h.tagName[1] || 0, t: h.textContent.trim().slice(0, 50) }));
  out.headings = heads; out.h1 = heads.filter((h) => h.l === 1).length; out.headingJumps = heads.filter((h, i) => i > 0 && h.l - heads[i - 1].l > 1).map((h) => h.t);
  const lm = {}; for (const [k, s] of [['banner', 'header,[role=banner]'], ['navigation', 'nav,[role=navigation]'], ['main', 'main,[role=main]'], ['contentinfo', 'footer,[role=contentinfo]'], ['complementary', 'aside,[role=complementary]']]) lm[k] = document.querySelectorAll(s).length;
  out.landmarks = lm; out.navs = [...document.querySelectorAll('nav')].map((n) => n.getAttribute('aria-label'));
  out.title = document.title; out.lang = document.documentElement.lang;
  out.themeAttr = document.documentElement.getAttribute('data-theme'); out.bodyBg = getComputedStyle(document.body).backgroundColor;
  out.textLen = document.body.innerText.length;
  return out;
}

function provProbe() {
  const vis = (e) => { const r = e.getBoundingClientRect(); const cs = getComputedStyle(e); return (r.width > 0 && r.height > 0 && cs.visibility !== 'hidden') || !!e.closest('.visually-hidden'); };
  const main = document.querySelector('main') || document.body;
  const badges = [...document.querySelectorAll('.prov')].filter(vis).map((p) => ({ word: (p.querySelector('.prov__word') || p).textContent.trim(), q: (p.querySelector('.prov__q') || {}).textContent || null, src: p.getAttribute('data-provenance'), tier: p.getAttribute('data-tier'), title: (p.getAttribute('title') || '').slice(0, 120) }));
  const words = {}; for (const b of badges) words[b.word] = (words[b.word] || 0) + 1;
  const banned = /\b(measured|inferred|estimated|synthetic|observed|actual|real-time|realtime|live data|mock|fake|demo data)\b/i;
  const hits = [];
  const w = document.createTreeWalker(main, NodeFilter.SHOW_TEXT);
  for (let n; (n = w.nextNode());) { const t = n.nodeValue; const m = banned.exec(t); if (m && n.parentElement && vis(n.parentElement)) hits.push({ word: m[0], ctx: t.trim().slice(Math.max(0, m.index - 30), m.index + 50), cls: String(n.parentElement.className).slice(0, 30), visibleText: !n.parentElement.closest('.visually-hidden') }); }
  const attrHits = [];
  for (const e of main.querySelectorAll('[title],[aria-label]')) { for (const a of ['title', 'aria-label']) { const v = e.getAttribute(a); if (v && banned.test(v)) attrHits.push({ attr: a, value: v.slice(0, 140), cls: String(e.className).slice(0, 30) }); } }
  // metric-like containers without a badge
  const noBadge = [];
  for (const m of main.querySelectorAll('.metric')) if (vis(m) && !m.querySelector('.prov')) noBadge.push('metric:' + (m.getAttribute('data-metric') || m.textContent.slice(0, 30)));
  const panelsNumbersNoBadge = [];
  for (const p of main.querySelectorAll('.panel, section.lab-section, article')) {
    if (!vis(p) || p.querySelector('.prov, [data-provenance]')) continue;
    const nums = [...p.querySelectorAll('.num, td, dd, .metric__value, .mono')].filter((e) => /\d/.test(e.textContent) && vis(e));
    if (nums.length) panelsNumbersNoBadge.push({ heading: (p.querySelector('h1,h2,h3,h4') || {}).textContent?.trim().slice(0, 50) || p.getAttribute('aria-label') || String(p.className).slice(0, 30), numericCells: nums.length, sample: nums.slice(0, 3).map((e) => e.textContent.trim().slice(0, 30)) });
  }
  const unavailable = [...main.querySelectorAll('[data-provenance=unavailable]')].filter(vis).map((u) => {
    const box = u.closest('.metric, tr, li, dd, .panel, section') || u.parentElement;
    const text = box.innerText.replace(/\s+/g, ' ').trim().slice(0, 200);
    const noWord = text.replace(/unavailable/ig, '').trim();
    return { text, hasReasonText: noWord.length > 12, bareZero: /(^|\s)0(\.0+)?(\s|$)/.test(noWord) && !/zero|0 of|\b0 (rows|runs|entries)/i.test(noWord) };
  });
  return { badges: badges.length, words, badgeSamples: badges.slice(0, 6), bannedHits: hits.slice(0, 20), attrHits: attrHits.slice(0, 20), metricsWithoutBadge: noBadge, panelsNumbersNoBadge: panelsNumbersNoBadge.slice(0, 20), unavailable: unavailable.slice(0, 20), qualifiers: [...new Set(badges.map((b) => b.q).filter(Boolean))] };
}

function focusInfo() {
  const T = colorTools();
  const a = document.activeElement;
  if (!a || a === document.body || a === document.documentElement) return { body: true };
  a.__fid = a.__fid || (window.__fn = (window.__fn || 0) + 1);
  const cs = getComputedStyle(a); const r = a.getBoundingClientRect();
  const t = (s) => (s || '').replace(/\s+/g, ' ').trim();
  const name = t(a.getAttribute('aria-label')) || t((a.getAttribute('aria-labelledby') || '').split(/\s+/).map((i) => document.getElementById(i)?.textContent || '').join(' ')) || t(a.labels && a.labels[0] && a.labels[0].textContent) || t(a.innerText || a.textContent || a.value || '');
  const ow = parseFloat(cs.outlineWidth) || 0; const hasOutline = cs.outlineStyle !== 'none' && ow > 0;
  let ring = hasOutline ? ow : 0, ringColor = hasOutline ? cs.outlineColor : null, how = hasOutline ? 'outline' : 'none';
  if (!hasOutline && cs.boxShadow && cs.boxShadow !== 'none') {
    const m = /(rgba?\([^)]*\)|color\([^)]*\)|#[0-9a-f]+)\s+(-?[\d.]+)px\s+(-?[\d.]+)px\s+([\d.]+)px(?:\s+([\d.]+)px)?/i.exec(cs.boxShadow);
    if (m) { ringColor = m[1]; ring = Math.max(parseFloat(m[5] || 0), parseFloat(m[4]) > 0 ? 0 : 0); how = 'box-shadow spread'; }
  }
  let rParent = null, rSelf = null;
  if (ringColor) {
    const par = T.bgOf(a.parentElement || a); const self = T.bgOf(a);
    const rc = T.over(T.parse(ringColor), par.color);
    rParent = par.ind ? null : +T.ratio(rc, par.color).toFixed(2); rSelf = self.ind ? null : +T.ratio(T.over(T.parse(ringColor), self.color), self.color).toFixed(2);
  }
  const cx = Math.min(Math.max(r.x + r.width / 2, 1), innerWidth - 1), cy = Math.min(Math.max(r.y + r.height / 2, 1), innerHeight - 1);
  const top = document.elementFromPoint(cx, cy);
  const obscured = !(top && (top === a || a.contains(top) || top.contains(a)));
  return { id: a.__fid, tag: a.tagName.toLowerCase(), role: a.getAttribute('role'), type: a.type || null, name: name.slice(0, 44), href: a.getAttribute('href'), expanded: a.getAttribute('aria-expanded'), pressed: a.getAttribute('aria-pressed'), current: a.getAttribute('aria-current'), how, ringPx: ring, ringColor, outlineOffset: cs.outlineOffset, ringVsParent: rParent, ringVsSelf: rSelf,
    inViewport: r.width > 0 && r.height > 0 && r.top >= -1 && r.bottom <= innerHeight + 1 && r.left >= -1 && r.right <= innerWidth + 1, obscuredBy: obscured && top ? (top.tagName.toLowerCase() + '.' + String(top.className).slice(0, 24)) : null, rect: [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)], tabindex: a.getAttribute('tabindex') };
}

function tabbables() {
  return [...document.querySelectorAll('a[href],button,input:not([type=hidden]),select,textarea,summary,[tabindex]:not([tabindex="-1"])')].filter((e) => {
    if (e.disabled || e.closest('[inert]')) return false; const cs = getComputedStyle(e); if (cs.visibility === 'hidden' || cs.display === 'none') return false;
    if (e.tagName === 'SUMMARY' && e.parentElement.tagName !== 'DETAILS') return false;
    if (e.closest('details:not([open])') && e.tagName !== 'SUMMARY') return false;
    return true;
  }).map((e) => ({ tag: e.tagName.toLowerCase(), name: e.tagName === 'INPUT' ? 'input' : (e.getAttribute('aria-label') || e.innerText || e.value || '').replace(/\s+/g, ' ').trim().slice(0, 25), href: e.getAttribute('href'), skip: !!e.closest('.visually-hidden') }));
}

function resetStart() { const s = document.createElement('span'); s.tabIndex = -1; document.body.prepend(s); s.focus(); s.remove(); if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur(); window.scrollTo(0, 0); return document.activeElement === document.body; }

// ---------------------------------------------------------------- phases
async function phasePrep() {
  const s = new Session('prep'); const b = await s.load('#/lab/run', 2500); await s.setVp(1280, 900);
  const out = {};
  const waitDone = async () => { for (let i = 0; i < 60; i++) { await sleep(2000); const t = await b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText||''`); if (/completed|failed|cancelled/i.test(t) && !/last known/i.test(t)) return t.replace(/\s+/g, ' ').slice(0, 120); } return 'timeout'; };
  try {
    await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
    await b.click(`document.querySelector('[data-preset=quick]')`); await sleep(400);
    await b.clickText('Start run'); await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
    out.sb = await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`); out.sbEnd = await waitDone();
    await s.go('#/lab/run', 1500);
    await b.click(`document.querySelector('[data-kind-option=t1_matrix] input')`); await sleep(500);
    await b.click(`document.querySelector('[data-preset=smallest]')`); await sleep(400);
    await b.clickText('Start run'); await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
    out.t1 = await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`); out.t1End = await waitDone();
    R.add('prep.runs', out.sbEnd.match(/completed/i) && out.t1End.match(/completed/i) ? 'PASS' : 'FAIL', out);
    writeFileSync(join(EV, 'runids.json'), JSON.stringify({ sb: out.sb, t1: out.t1 }));
  } catch (e) { R.add('prep.exception', 'FAIL', String(e.stack || e)); }
  await s.end();
}

async function phaseMatrix() {
  const w = +args.vp; const s = new Session('vp' + w);
  const minPrimary = w >= 820 ? 32 : 44;
  const detail = {};
  const agg = {};
  for (const theme of ['light', 'dark']) {
    await s.setVp(w, vh(w)); await s.setMedia([{ name: 'prefers-color-scheme', value: theme }, { name: 'prefers-reduced-motion', value: 'no-preference' }]);
    await s.load(ROUTES[0][1], 1800);
    for (const [name, hash] of ROUTES) {
      const b = await s.go(hash, 1200);
      await b.eval('window.scrollTo(0,0)');
      const p = await b.eval(inj(pageProbe, { minPrimary }));
      detail[`${theme}/${name}`] = p;
      if (w === 390 || w === 1280) await jshot(b, `${name}__${w}__${theme}`, 'shots/matrix', true);
      const a = (agg[theme] ||= { routes: 0, overflowRoutes: [], contrastFailRoutes: {}, contrastMin: 99, indeterminate: 0, nonTextWeak: {}, unnamed: [], below24: {}, belowPrimary: {}, h1Bad: [], headingJumps: {}, landmarkBad: [], themeAttr: {}, textLen: {} });
      a.routes++;
      if (p.overflow.pageHScroll) a.overflowRoutes.push(name);
      if (p.contrast.fails.length) a.contrastFailRoutes[name] = p.contrast.fails.length;
      a.contrastMin = Math.min(a.contrastMin, p.contrast.min); a.indeterminate += p.contrast.indeterminate;
      if (p.nontext.weak.length) a.nonTextWeak[name] = p.nontext.weak.map((x) => `${x.tag}.${x.cls}:${x.best}`);
      if (p.unnamed.length) a.unnamed.push({ name, items: p.unnamed });
      if (p.below24.length) a.below24[name] = p.below24.map((x) => `${x.tag}.${x.cls}[${x.name}] ${x.w}x${x.h}`);
      if (p.belowPrimary.length) a.belowPrimary[name] = p.belowPrimary.map((x) => `${x.tag}.${x.cls}[${x.name}] ${x.w}x${x.h}`);
      if (p.h1 !== 1) a.h1Bad.push(`${name}:${p.h1}`);
      if (p.headingJumps.length) a.headingJumps[name] = p.headingJumps;
      if (p.landmarks.main !== 1 || p.landmarks.banner < 1 || p.landmarks.navigation < 1) a.landmarkBad.push(`${name}:${JSON.stringify(p.landmarks)}`);
      a.themeAttr[name] = p.themeAttr + '|' + p.bodyBg; a.textLen[name] = p.textLen;
    }
  }
  save(`matrix_${w}.detail.json`, detail);
  for (const theme of ['light', 'dark']) {
    const a = agg[theme];
    R.add(`reflow.${w}.${theme}`, a.overflowRoutes.length ? 'FAIL' : 'PASS', { routes: a.routes, overflowRoutes: a.overflowRoutes });
    R.add(`contrast_text.${w}.${theme}`, Object.keys(a.contrastFailRoutes).length ? 'FAIL' : 'PASS', { failsPerRoute: a.contrastFailRoutes, minRatioSeen: a.contrastMin, indeterminateTextNodes: a.indeterminate });
    R.add(`contrast_nontext.${w}.${theme}`, Object.keys(a.nonTextWeak).length ? 'OBSERVED' : 'PASS', a.nonTextWeak);
    R.add(`names.${w}.${theme}`, a.unnamed.length ? 'FAIL' : 'PASS', a.unnamed);
    R.add(`targets24.${w}.${theme}`, Object.keys(a.below24).length ? 'FAIL' : 'PASS', a.below24);
    R.add(`targets_primary${minPrimary}.${w}.${theme}`, Object.keys(a.belowPrimary).length ? 'OBSERVED' : 'PASS', a.belowPrimary);
    R.add(`structure.${w}.${theme}`, a.h1Bad.length || a.landmarkBad.length ? 'FAIL' : 'PASS', { h1Bad: a.h1Bad, headingJumps: a.headingJumps, landmarkBad: a.landmarkBad, theme: a.themeAttr });
  }
  const parity = Object.keys(agg.light.textLen).filter((k) => Math.abs(agg.light.textLen[k] - agg.dark.textLen[k]) > 5);
  R.add(`parity_light_dark.${w}`, parity.length ? 'OBSERVED' : 'PASS', parity);
  await s.end();
}

async function phaseAx() {
  const s = new Session('ax'); await s.setVp(+(args.vp || 1280), 900); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }]);
  const b = await s.load(ROUTES[0][1], 1800);
  await b.send('Accessibility.enable');
  const summary = {};
  for (const [name, hash] of ROUTES) {
    await s.go(hash, 1300);
    const { nodes } = await b.send('Accessibility.getFullAXTree');
    const byId = new Map(nodes.map((n) => [n.nodeId, n]));
    const props = (n) => Object.fromEntries((n.properties || []).map((p) => [p.name, p.value && p.value.value !== undefined ? p.value.value : (p.value && p.value.relatedNodes ? p.value.relatedNodes.map((r) => r.idref || r.backendDOMNodeId) : p.value)]));
    const textOf = (n, d = 0) => { if (d > 6) return ''; const own = (n.role && n.role.value === 'StaticText') ? (n.name && n.name.value) || '' : ''; return own + (n.childIds || []).map((c) => (byId.get(c) ? textOf(byId.get(c), d + 1) : '')).join(' '); };
    const live = [], heads = [], lms = [], unnamed = [], inter = [], states = [];
    const IR = new Set(['button', 'link', 'textbox', 'checkbox', 'radio', 'combobox', 'tab', 'switch', 'menuitem', 'slider', 'spinbutton', 'searchbox', 'listbox', 'DisclosureTriangle', 'PopUpButton', 'ComboBoxSelect', 'MenuListPopup']);
    let total = 0;
    for (const n of nodes) {
      if (n.ignored) continue; total++;
      const role = n.role && n.role.value; const nm = (n.name && n.name.value) || ''; const pr = props(n);
      if (role === 'heading') heads.push({ level: pr.level, name: nm.slice(0, 50) });
      if (['banner', 'navigation', 'main', 'contentinfo', 'complementary', 'region', 'form', 'search', 'dialog', 'alertdialog'].includes(role)) lms.push({ role, name: nm.slice(0, 40) });
      if (['status', 'alert', 'log', 'timer', 'progressbar', 'marquee'].includes(role) || (pr.live && pr.live !== 'off')) live.push({ role, live: pr.live, atomic: pr.atomic, text: (textOf(n) || nm).replace(/\s+/g, ' ').trim().slice(0, 100) });
      if (IR.has(role)) {
        inter.push(role);
        if (!nm.trim()) unnamed.push({ role, backend: n.backendDOMNodeId });
        const st = {}; for (const k of ['expanded', 'selected', 'pressed', 'checked', 'invalid', 'disabled', 'haspopup', 'required', 'current']) if (pr[k] !== undefined && pr[k] !== false) st[k] = pr[k];
        if (Object.keys(st).length || (n.description && n.description.value)) states.push({ role, name: nm.slice(0, 36), ...st, description: ((n.description && n.description.value) || '').slice(0, 80) || undefined });
      }
    }
    const hjump = heads.filter((h, i) => i > 0 && h.level - heads[i - 1].level > 1).map((h) => h.name);
    // current-page markers in the nav
    const cur = nodes.filter((n) => !n.ignored && n.role && n.role.value === 'link' && props(n).current).map((n) => (n.name && n.name.value) + '=' + props(n).current);
    summary[name] = { axNodes: total, h1: heads.filter((h) => h.level === 1).length, heads, headingJumps: hjump, landmarks: lms, interactiveCount: inter.length, unnamed, live, states: states.slice(0, 40), currentLinks: cur };
    const bad = unnamed.length || heads.filter((h) => h.level === 1).length !== 1 || !lms.some((l) => l.role === 'main') || !lms.some((l) => l.role === 'banner') || !lms.some((l) => l.role === 'navigation');
    R.add(`ax.${name}`, bad ? 'FAIL' : 'PASS', { h1: summary[name].h1, headingJumps: hjump, unnamed, landmarks: lms.map((l) => l.role + (l.name ? ':' + l.name : '')), live, currentLinks: cur });
  }
  save('ax_summary.1280.json', summary);
  await s.end();
}

async function phaseKbd() {
  const s = new Session('kbd'); const results = {};
  const traverse = async (b, max = 140) => {
    await b.eval(inj(resetStart)); await sleep(120);
    const stops = []; let bodyHits = 0;
    for (let i = 0; i < max; i++) {
      await b.tab(); await sleep(150);
      const f = await b.eval(inj(focusInfo));
      if (f.body) { bodyHits++; stops.push(f); if (bodyHits >= 2) break; continue; }
      if (stops.length && stops.slice(0, -1).some((x) => x.id === f.id) && f.id === stops.find((x) => !x.body).id) break;
      stops.push(f);
    }
    return stops;
  };
  const summarize = (stops) => {
    const real = stops.filter((x) => !x.body);
    return { stops: real.length, bodyHits: stops.filter((x) => x.body).length, noIndicator: real.filter((x) => x.ringPx <= 0).map((x) => x.name), under2px: real.filter((x) => x.ringPx > 0 && x.ringPx < 2).map((x) => `${x.name}:${x.ringPx}`), lowContrast: real.filter((x) => (x.ringVsParent !== null && x.ringVsParent < 3) && (x.ringVsSelf === null || x.ringVsSelf < 3)).map((x) => `${x.name}:${x.ringVsParent}/${x.ringVsSelf}`), notInViewport: real.filter((x) => !x.inViewport).map((x) => x.name), obscured: real.filter((x) => x.obscuredBy).map((x) => `${x.name} by ${x.obscuredBy}`), minRing: Math.min(...real.map((x) => x.ringPx)), firstStop: real[0] && real[0].name, order: real.map((x) => `${x.tag}${x.role ? '[' + x.role + ']' : ''}:${x.name}`) };
  };
  // ---------- per-route Tab traversal, light + dark at 1280, light at 390
  for (const cfg of args.only === 'interact' ? [] : [{ w: 1280, theme: 'light' }, { w: 1280, theme: 'dark' }, { w: 390, theme: 'light' }]) {
    if (cfg.w === 390) s.cold = 3; // fresh Chrome for the phone-width pass
    await s.setVp(cfg.w, cfg.w >= 820 ? 900 : 844); await s.setMedia([{ name: 'prefers-color-scheme', value: cfg.theme }, { name: 'prefers-reduced-motion', value: 'no-preference' }]);
    await s.load(ROUTES[0][1], 1800);
    for (const [name, hash] of ROUTES) {
      const b = await s.go(hash, 1200);
      const stops = await traverse(b);
      if (!stops.some((x) => !x.body)) results[`DEBUG/${cfg.w}/${name}`] = await b.eval(`({active:document.activeElement&&document.activeElement.tagName, focus:document.hasFocus(), w:innerWidth})`);
      const sum = summarize(stops); const expected = await b.eval(inj(tabbables));
      const reached = new Set(stops.filter((x) => !x.body).map((x) => x.href || (x.tag === 'input' ? 'input' : x.name.slice(0, 25))));
      const unreached = expected.filter((e) => !e.skip && !reached.has(e.href || e.name)).map((e) => e.href || e.name);
      results[`${cfg.w}/${cfg.theme}/${name}`] = { ...sum, unreached: [...new Set(unreached)].slice(0, 15), expectedTabbables: expected.length };
      const bad = sum.noIndicator.length || sum.under2px.length || sum.lowContrast.length || sum.obscured.length || sum.bodyHits > 1 || sum.stops === 0;
      R.add(`kbd.traversal.${cfg.w}.${cfg.theme}.${name}`, bad ? 'FAIL' : 'PASS', { stops: sum.stops, first: sum.firstStop, noIndicator: sum.noIndicator, under2px: sum.under2px, lowContrastRing: sum.lowContrast, obscured: sum.obscured, notInViewport: sum.notInViewport.length, bodyHits: sum.bodyHits, unreached: results[`${cfg.w}/${cfg.theme}/${name}`].unreached });
      if (name === 'lab_run_form' || name === 'overview') { await b.eval(inj(resetStart)); for (let i = 0; i < 3; i++) await b.tab(); await sleep(250); await jshot(b, `focus_${name}_${cfg.w}_${cfg.theme}`, 'shots/kbd'); }
      if (cfg.w === 1280 && cfg.theme === 'light') { await b.eval(inj(resetStart)); await b.tab(true); await sleep(150); results[`${cfg.w}/${cfg.theme}/${name}`].shiftTabFromTop = (await b.eval(inj(focusInfo))).name || 'BODY'; }
    }
  }
  if (args.only !== 'interact') save('kbd_traversal.json', results);
  if (args.only === 'trav') { await s.end(); return; }

  // ---------- interaction checks at 1280
  await s.setVp(1280, 900); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }]);
  let b = await s.load('#/traffic', 1800);
  await b.eval(`window.__ann=[]; new MutationObserver(ms=>{for(const m of ms){const t=m.target.nodeType===3?m.target.parentElement:m.target; const el=t&&t.closest&&t.closest('[role=status],[role=alert],[aria-live]'); if(el) window.__ann.push({t:performance.now()|0, role:el.getAttribute('role'), live:el.getAttribute('aria-live'), text:el.innerText.replace(/\\s+/g,' ').slice(0,140)});}}).observe(document.body,{subtree:true,childList:true,characterData:true});`);
  // skip link
  await b.eval(inj(resetStart)); await b.tab(); await sleep(200);
  const skip1 = await b.eval(inj(focusInfo)); const hash0 = await b.eval('location.hash');
  const skipShot = await jshot(b, 'skip_link_focus', 'shots/kbd', false, 'png');
  const skipRect = await b.eval(`(()=>{const r=document.activeElement.getBoundingClientRect();return [Math.round(r.x),Math.round(r.y),Math.round(r.width),Math.round(r.height)]})()`);
  await ENTER(b); await sleep(300);
  const skip2 = await b.eval(`({tag:document.activeElement.tagName, id:document.activeElement.id, hash:location.hash, scrollY: scrollY})`);
  R.add('kbd.skip_link', /skip/i.test(skip1.name) && skip2.id === 'main' && skip2.hash === hash0 ? 'PASS' : 'FAIL', { first: skip1.name, rect: skipRect, afterEnter: skip2, hashBefore: hash0 }, skipShot);
  await b.tab(); await sleep(150);
  R.add('kbd.after_skip_next_tab', 'OBSERVED', { next: (await b.eval(inj(focusInfo))).name });
  // nav link activation by keyboard: focus retention and live region
  await b.eval(`document.querySelector('a[data-screen=cache]').focus()`);
  await ARROW(b, 'Down'); await sleep(150); const arrowDown = (await b.eval(inj(focusInfo))).name;
  await ARROW(b, 'Up'); await sleep(150); const arrowUp = (await b.eval(inj(focusInfo))).name;
  await b.eval(`window.__ann.length=0`);
  await b.eval(`document.querySelector('a[data-screen=origin]').focus()`); await ENTER(b); await sleep(900);
  const afterNav = await b.eval(`({hash:location.hash, active:document.activeElement.tagName+':'+(document.activeElement.innerText||'').trim().slice(0,30), scrollY, title:document.title, h1:(document.querySelector('h1')||{}).innerText, ann:window.__ann.slice(0,6)})`);
  R.add('kbd.nav_activation', afterNav.active !== 'BODY:' ? 'OBSERVED' : 'FAIL', { arrowDown, arrowUp, afterEnterOnOriginLink: afterNav });
  // theme control: arrow / Enter / Space
  await s.go('#/overview', 800);
  const theme0 = await b.eval(`[...document.querySelectorAll('.app-header .seg__btn')].map(x=>x.innerText+'='+x.getAttribute('aria-pressed')).join(',')`);
  await b.eval(`document.querySelector('.app-header .seg__btn').focus()`);
  await ARROW(b, 'Right'); await sleep(200); const arrowTheme = (await b.eval(inj(focusInfo))).name;
  await b.tab(); await sleep(150); const nextTheme = (await b.eval(inj(focusInfo))).name;
  await b.tab(); await sleep(100); await ENTER(b); await sleep(300);
  const theme1 = await b.eval(`[...document.querySelectorAll('.app-header .seg__btn')].map(x=>x.innerText+'='+x.getAttribute('aria-pressed')).join(',')+' | data-theme='+document.documentElement.getAttribute('data-theme')`);
  await b.eval(`document.querySelector('.app-header .seg__btn').focus()`); await SPACE(b); await sleep(300);
  const theme2 = await b.eval(`[...document.querySelectorAll('.app-header .seg__btn')].map(x=>x.innerText+'='+x.getAttribute('aria-pressed')).join(',')+' | data-theme='+document.documentElement.getAttribute('data-theme')`);
  await b.eval(`document.querySelector('.app-header .seg__btn[data-theme-option=system]').click()`);
  R.add('kbd.theme_control', 'OBSERVED', { initial: theme0, afterArrowRightFocus: arrowTheme, tabNext: nextTheme, afterEnterOnSecond: theme1, afterSpaceOnFirst: theme2, note: 'segmented control is a group of aria-pressed buttons (not a radiogroup): arrow keys are not expected to move; Tab moves between the three buttons' });
  // tabs: Lab subnav (links with aria-current, not role=tab)
  await s.go('#/lab', 1300);
  const tabs = await b.eval(`({roleTabs:document.querySelectorAll('[role=tab],[role=tablist]').length, subnav:[...document.querySelectorAll('nav.lab-subnav a')].map(a=>a.innerText.trim()+(a.getAttribute('aria-current')?'[current='+a.getAttribute('aria-current')+']':''))})`);
  await b.eval(`document.querySelector('nav.lab-subnav a').focus()`); await ARROW(b, 'Right'); await sleep(150);
  const tabArrow = (await b.eval(inj(focusInfo))).name;
  await b.tab(); await sleep(150); const tabTab = (await b.eval(inj(focusInfo))).name;
  await b.eval(`document.querySelector('nav.lab-subnav a[data-lab-view=runs]').focus()`); await ENTER(b); await sleep(900);
  const tabEnter = await b.eval(`({hash:location.hash, active:document.activeElement.tagName+':'+(document.activeElement.innerText||'').trim().slice(0,24), current:[...document.querySelectorAll('nav.lab-subnav a[aria-current]')].map(a=>a.innerText)})`);
  R.add('kbd.lab_subnav_tabs', 'OBSERVED', { ...tabs, arrowRightFocus: tabArrow, tabAfterFirst: tabTab, enterOnRuns: tabEnter, note: 'implemented as a link list with aria-current, not an ARIA tablist' });
  // disclosure / details
  await s.go('#/lab/run', 1500);
  const details = await b.eval(`[...document.querySelectorAll('details')].map((d,i)=>({i, summary:d.querySelector('summary').innerText.replace(/\\s+/g,' ').trim().slice(0,50), open:d.open}))`);
  const dres = [];
  for (let i = 0; i < details.length; i++) {
    await b.eval(`(()=>{const s=document.querySelectorAll('details')[${i}].querySelector('summary'); s.scrollIntoView({block:'center'}); s.focus();})()`);
    await ENTER(b); await sleep(250); const o1 = await b.eval(`document.querySelectorAll('details')[${i}].open`);
    const f1 = (await b.eval(inj(focusInfo))).name;
    await SPACE(b); await sleep(250); const o2 = await b.eval(`document.querySelectorAll('details')[${i}].open`);
    dres.push({ summary: details[i].summary, initial: details[i].open, afterEnter: o1, afterSpace: o2, focusStaysOnSummary: f1 === details[i].summary || /./.test(f1) });
  }
  R.add('kbd.details_disclosure', dres.every((d) => d.afterEnter !== d.initial && d.afterSpace === d.initial) ? 'PASS' : 'FAIL', dres);
  // Lab Details (connection) button
  await s.go('#/lab', 1200);
  const conn = await b.eval(`(()=>{const x=[...document.querySelectorAll('button')].find(b=>/^details$/i.test(b.innerText.trim())); if(!x) return null; x.focus(); return {exp:x.getAttribute('aria-expanded'), controls:x.getAttribute('aria-controls')};})()`);
  if (conn) { await ENTER(b); await sleep(300); const c2 = await b.eval(`(()=>{const x=[...document.querySelectorAll('button')].find(b=>/^(details|hide)/i.test(b.innerText.trim())&&b.hasAttribute('aria-expanded')); return {exp:x&&x.getAttribute('aria-expanded'), active:document.activeElement.tagName, inputs:[...document.querySelectorAll('input[type=password],input[type=text]')].map(i=>i.type+' label='+((i.labels&&i.labels[0]&&i.labels[0].innerText)||i.getAttribute('aria-label'))+' describedby='+i.getAttribute('aria-describedby')+' -> '+(((i.getAttribute('aria-describedby')||'').split(' ').map(x=>(document.getElementById(x)||{}).innerText||'?')).join(' / ').slice(0,100)))}})()`); R.add('kbd.lab_connection_details', 'OBSERVED', { before: conn, after: c2 }); } else R.add('kbd.lab_connection_details', 'OBSERVED', 'no Details button found');
  // failed submit with keyboard
  await s.go('#/lab/run', 1500);
  await b.click(`document.querySelector('[data-kind-option=scenario_b] input')`);
  await b.eval(`(()=>{const l=[...document.querySelectorAll('label')].find(l=>l.textContent.trim().toLowerCase().startsWith('repetitions per arm')); const el=document.getElementById(l.htmlFor); const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; set.call(el,'99'); el.dispatchEvent(new Event('input',{bubbles:true}));})()`);
  await b.eval(`window.__ann.length=0; window.__ann=[]`).catch(() => {});
  await b.eval(`window.__ann=[]; new MutationObserver(ms=>{for(const m of ms){const t=m.target.nodeType===3?m.target.parentElement:m.target; const el=t&&t.closest&&t.closest('[role=status],[role=alert],[aria-live]'); if(el) window.__ann.push({t:performance.now()|0, role:el.getAttribute('role'), live:el.getAttribute('aria-live'), text:el.innerText.replace(/\\s+/g,' ').slice(0,160)});}}).observe(document.body,{subtree:true,childList:true,characterData:true});`);
  await b.eval(`(()=>{const x=[...document.querySelectorAll('button[type=submit]')].find(b=>/start run/i.test(b.innerText)); x.scrollIntoView({block:'center'}); x.focus();})()`);
  await ENTER(b); await sleep(900);
  const fs = await b.eval(`(()=>{
    const sum=document.querySelector('[data-error-summary]'); const a=document.activeElement;
    const inv=[...document.querySelectorAll('[aria-invalid=true]')].map(e=>{const d=(e.getAttribute('aria-describedby')||'').split(/\\s+/).filter(Boolean); return {id:e.id, tag:e.tagName, label:(e.labels&&e.labels[0]&&e.labels[0].innerText.slice(0,30))||e.getAttribute('aria-label')||null, describedby:d, resolved:d.map(i=>{const x=document.getElementById(i); return x?x.innerText.replace(/\\s+/g,' ').slice(0,90):'MISSING '+i;})};});
    const links=sum?[...sum.querySelectorAll('a')].map(l=>({text:l.innerText, href:l.getAttribute('href'), target:!!document.getElementById(l.getAttribute('href').slice(1))})):[];
    return {summaryRole:sum&&sum.getAttribute('role'), summaryText:sum&&sum.innerText.replace(/\\s+/g,' ').slice(0,200), focusOnSummary:a===sum, active:a.tagName+'.'+String(a.className).slice(0,20), invalid:inv, summaryLinks:links, ann:window.__ann.slice(0,6)};})()`);
  R.add('form.failed_submit', fs.summaryRole === 'alert' && fs.focusOnSummary && fs.invalid.length && fs.invalid.every((i) => i.describedby.length && i.resolved.every((r) => !r.startsWith('MISSING'))) ? 'PASS' : 'FAIL', fs, await jshot(b, 'failed_submit', 'shots/kbd', false, 'png'));
  // AX check of the invalid fields
  await b.send('Accessibility.enable');
  const { nodes } = await b.send('Accessibility.getFullAXTree');
  const inval = nodes.filter((n) => !n.ignored && (n.properties || []).some((p) => p.name === 'invalid' && p.value.value && p.value.value !== 'false')).map((n) => ({ role: n.role.value, name: n.name && n.name.value, description: (n.description && n.description.value || '').slice(0, 120) }));
  R.add('form.failed_submit_ax', inval.length ? 'PASS' : 'FAIL', inval);
  // click first summary link: focus goes to the field?
  await b.eval(`document.querySelector('[data-error-summary] a').click()`); await sleep(400);
  R.add('form.summary_link_focus', 'OBSERVED', await b.eval(`({active:document.activeElement.tagName+':'+(document.activeElement.id||''), invalid:document.activeElement.getAttribute('aria-invalid')})`));

  // ---------- cancel dialog: create a longer run, open by keyboard
  await b.eval(`document.querySelector('[data-preset=quick]').click()`); await sleep(300);
  await b.eval(`(()=>{const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; for(const [l,v] of [['repetitions per arm',1],['warm-up',2],['healthy',8],['degraded',8],['recovery',5]]){const lab=[...document.querySelectorAll('label')].find(x=>x.textContent.trim().toLowerCase().startsWith(l)); const el=document.getElementById(lab.htmlFor); set.call(el,String(v)); el.dispatchEvent(new Event('input',{bubbles:true}));}})()`);
  await sleep(300); await b.clickText('Start run'); await b.waitFor(`document.querySelector('[data-cancel-open]')`, 20000); await sleep(2000);
  await b.eval(`window.__ann=[]; new MutationObserver(ms=>{for(const m of ms){const t=m.target.nodeType===3?m.target.parentElement:m.target; const el=t&&t.closest&&t.closest('[role=status],[role=alert],[aria-live]'); if(el) window.__ann.push({t:performance.now()|0, role:el.getAttribute('role'), live:el.getAttribute('aria-live'), text:el.innerText.replace(/\\s+/g,' ').slice(0,160)});}}).observe(document.body,{subtree:true,childList:true,characterData:true});`);
  const live0 = await b.eval(`[...document.querySelectorAll('[role=status],[role=alert],[aria-live]')].map(e=>({role:e.getAttribute('role'),live:e.getAttribute('aria-live'),vh:!!e.closest('.visually-hidden')||e.classList.contains('visually-hidden'),text:e.innerText.replace(/\\s+/g,' ').slice(0,100)}))`);
  save('run_page_live_regions.json', live0);
  await b.eval(`document.querySelector('[data-cancel-open]').focus()`); await ENTER(b); await sleep(600);
  const d1 = await b.eval(`(()=>{const d=document.querySelector('dialog[data-cancel-dialog]'); const a=document.activeElement; return {open:!!(d&&d.open), modal:d&&d.matches(':modal'), role:d&&d.getAttribute('role'), labelledby:d&&d.getAttribute('aria-labelledby'), title:d&&document.getElementById(d.getAttribute('aria-labelledby')).innerText, describedby:d&&d.getAttribute('aria-describedby'), focus:a.tagName+':'+a.innerText.trim(), focusInDialog:!!(d&&d.contains(a)), mainInert:(()=>{const m=document.querySelector('main'); const r=m.getBoundingClientRect(); const top=document.elementFromPoint(r.x+20,r.y+60); return !(top&&m.contains(top))})()};})()`);
  await jshot(b, 'cancel_dialog_open', 'shots/kbd', false, 'png');
  const trail = []; for (let i = 0; i < 7; i++) { await b.tab(); await sleep(120); const f = await b.eval(`(()=>{const d=document.querySelector('dialog[data-cancel-dialog]'); const a=document.activeElement; return (d&&d.contains(a)?'IN:':'OUT:')+a.tagName+':'+(a.innerText||'').trim().slice(0,24)})()`); trail.push(f); }
  await b.tab(true); await sleep(120); trail.push('shift:' + await b.eval(`document.activeElement.innerText.trim()`));
  await b.send('Accessibility.enable'); const axd = (await b.send('Accessibility.getFullAXTree')).nodes.filter((n) => !n.ignored && n.role && ['navigation', 'main', 'banner', 'dialog', 'alertdialog'].includes(n.role.value)).map((n) => n.role.value + ':' + ((n.name && n.name.value) || ''));
  const escShot = await ESC(b); await sleep(500);
  const d2 = await b.eval(`({dialogOpen:!!(document.querySelector('dialog[data-cancel-dialog]')||{}).open, focus:document.activeElement.tagName+':'+document.activeElement.innerText.trim(), state:(document.querySelector('.lab-detail-head')||{}).innerText.replace(/\\s+/g,' ').slice(0,60)})`);
  R.add('dialog.cancel_open_keyboard_and_escape', d1.open && d1.modal && /keep running/i.test(d1.focus) && trail.every((t) => t.startsWith('IN:') || t.startsWith('OUT:BODY') || t.startsWith('shift:')) && /cancel run/i.test(d2.focus) && !d2.dialogOpen ? 'PASS' : 'FAIL', { opened: d1, tabTrail: trail, axLandmarksWhileOpen: axd, afterEscape: d2 });
  // confirm by keyboard
  await b.eval(`document.querySelector('[data-cancel-open]').focus()`); await ENTER(b); await sleep(500);
  await b.tab(); await sleep(150); await ENTER(b);
  const stat = []; for (let i = 0; i < 100; i++) { const t = await b.eval(`(document.querySelector('[data-run-status-live]')||{}).textContent||''`); if (!stat.length || stat[stat.length - 1].text !== t) stat.push({ ms: i * 300, text: t }); if (/cancelled/i.test(t) && /ended|closed|not/i.test(t)) break; await sleep(300); }
  save('run_status_live_transitions.json', stat);
  await b.waitFor(`/cancelled/i.test((document.querySelector('.lab-detail-head')||{}).innerText||'')`, 40000, 500);
  const d3 = await b.eval(`({statusTransitions:null, focus:document.activeElement.tagName+'.'+String(document.activeElement.className).slice(0,20)+':'+(document.activeElement.innerText||'').trim().slice(0,40), state:(document.querySelector('.lab-detail-head')||{}).innerText.replace(/\\s+/g,' ').slice(0,80), dialogOpen:!!(document.querySelector('dialog[data-cancel-dialog]')||{}).open, ann:window.__ann.filter(a=>a.role==='alert'||/Run /.test(a.text)).slice(-8)})`);
  d3.statusTransitions = stat;
  R.add('dialog.cancel_confirm_focus_and_announce', d3.focus.startsWith('BODY') ? 'FAIL' : 'OBSERVED', d3, await jshot(b, 'after_cancel', 'shots/kbd', false, 'png'));
  save('cancel_run_announcements.json', d3.ann);

  // ---------- Menu button at 390
  await s.setVp(390, 844); b = await s.load('#/overview', 1800);
  const m0 = await b.eval(`(()=>{const m=document.getElementById('app-menu-button'); const nav=document.querySelector('nav'); const r=nav.querySelector('a').getBoundingClientRect(); const cs=getComputedStyle(nav.querySelector('a')); const panel=getComputedStyle(nav.querySelector('.app-nav__panel')); return {exp:m.getAttribute('aria-expanded'), controls:m.getAttribute('aria-controls'), navLinkRect:[Math.round(r.x),Math.round(r.y),Math.round(r.width),Math.round(r.height)], panelVis:panel.visibility, panelDisplay:panel.display}})()`);
  await b.eval(inj(resetStart)); const mt = []; for (let i = 0; i < 4; i++) { await b.tab(); await sleep(120); mt.push((await b.eval(inj(focusInfo))).name); }
  R.add('kbd.menu_closed_tab_order', 'OBSERVED', { menuClosed: m0, firstTabs: mt });
  await b.eval(`document.getElementById('app-menu-button').focus()`); await ENTER(b); await sleep(400);
  const m1 = await b.eval(`({exp:document.getElementById('app-menu-button').getAttribute('aria-expanded'), active:document.activeElement.id||document.activeElement.tagName})`);
  await jshot(b, 'menu_open_390', 'shots/kbd');
  await b.tab(); await sleep(150); const mNext = (await b.eval(inj(focusInfo)));
  await ESC(b); await sleep(300);
  const m2 = await b.eval(`({exp:document.getElementById('app-menu-button').getAttribute('aria-expanded'), active:document.activeElement.id||document.activeElement.tagName})`);
  await SPACE(b); await sleep(300); const m3 = await b.eval(`document.getElementById('app-menu-button').getAttribute('aria-expanded')`);
  await SPACE(b); await sleep(300); const m4 = await b.eval(`document.getElementById('app-menu-button').getAttribute('aria-expanded')`);
  await ENTER(b); await sleep(300);
  await b.eval(`document.querySelector('a[data-screen=cache]').focus()`); await ENTER(b); await sleep(900);
  const m5 = await b.eval(`({hash:location.hash, exp:document.getElementById('app-menu-button').getAttribute('aria-expanded'), active:document.activeElement.tagName+':'+(document.activeElement.innerText||document.activeElement.id||'').trim().slice(0,20)})`);
  R.add('kbd.menu_button', m1.exp === 'true' && m2.exp === 'false' && /app-menu-button/.test(m2.active) && m3 === 'true' && m4 === 'false' ? 'PASS' : 'FAIL', { enterOpens: m1, nextTabAfterOpen: mNext.name, escapeCloses: m2, spaceToggle: [m3, m4], navigateViaMenu: m5 });
  await s.end();
}

async function phaseMedia() {
  const s = new Session('media'); await s.setVp(1280, 900); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }, { name: 'prefers-reduced-motion', value: 'no-preference' }]);
  let b = await s.load('#/lab', 1800);
  const mediaRules = await b.eval(`(()=>{const o={}; for(const sh of document.styleSheets){let rules; try{rules=sh.cssRules}catch{continue} for(const r of rules){ if(r.media){const t=r.media.mediaText; if(/prefers-reduced-motion|prefers-contrast|forced-colors|prefers-color-scheme/.test(t)) o[t]=(o[t]||0)+r.cssRules.length;}}} return o;})()`);
  R.add('media.rules_present', 'OBSERVED', mediaRules);
  const motion = async (tag) => b.eval(`(()=>{
    const out={interactiveTransitions:[], animated:0, infinite:[], running:[]};
    for(const e of document.querySelectorAll('a,button,input,select,summary,textarea,.btn,.tab,.seg__btn,progress,dialog')){const cs=getComputedStyle(e); const d=cs.transitionDuration.split(',').map(parseFloat); if(d.some(x=>x>0.001)) out.interactiveTransitions.push(e.tagName.toLowerCase()+'.'+String(e.className).slice(0,20)+' '+cs.transitionProperty.slice(0,50)+' '+cs.transitionDuration); if(cs.animationName!=='none'&&parseFloat(cs.animationDuration)>0.001){out.animated++; if(cs.animationIterationCount==='infinite') out.infinite.push(e.tagName.toLowerCase()+'.'+String(e.className).slice(0,20)+' '+cs.animationName);}}
    out.interactiveTransitions=[...new Set(out.interactiveTransitions)].slice(0,12);
    out.running=document.getAnimations().map(a=>{const ef=a.effect; let props=[]; try{props=[...new Set(ef.getKeyframes().flatMap(k=>Object.keys(k)).filter(k=>!['offset','easing','composite','computedOffset'].includes(k)))]}catch{} return {name:a.animationName||a.transitionProperty||a.constructor.name, props, dur:ef&&ef.getTiming&&ef.getTiming().duration}}).slice(0,10);
    out.scrollBehavior=getComputedStyle(document.documentElement).scrollBehavior; return out;})()`);
  const pressed = async () => {
    const pt = await b.eval(`(()=>{const x=[...document.querySelectorAll('.btn:not([disabled]):not([aria-disabled=true]),button')].find(e=>{const r=e.getBoundingClientRect();return r.width>20&&r.height>20&&r.top>=0}); const r=x.getBoundingClientRect(); window.__pb=x; return {x:r.x+r.width/2,y:r.y+r.height/2, cls:x.className, text:x.innerText.trim().slice(0,20)}})()`);
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: pt.x, y: pt.y });
    await sleep(350); const hover = await b.eval(`getComputedStyle(window.__pb).transform+' | '+getComputedStyle(window.__pb).boxShadow.slice(0,30)`);
    await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: pt.x, y: pt.y, button: 'left', clickCount: 1 });
    await sleep(350); const down = await b.eval(`getComputedStyle(window.__pb).transform`);
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 5, y: 5 }); await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 5, y: 5, button: 'left', clickCount: 1 });
    return { btn: pt, hover, transformWhilePressed: down };
  };
  const res = {};
  for (const pref of ['no-preference', 'reduce']) {
    await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }, { name: 'prefers-reduced-motion', value: pref }]);
    b = await s.load('#/lab', 1500);
    res[pref] = { lab: await motion(pref), press: await pressed() };
    // route-change animations: sample immediately after hash change
    await b.eval(`location.hash='#/overview'`); await sleep(60); const a1 = await b.eval(`document.getAnimations().map(a=>a.animationName||a.transitionProperty).slice(0,10)`);
    await b.eval(`location.hash='#/lab/run'`); await sleep(60); const a2 = await b.eval(`document.getAnimations().map(a=>a.animationName||a.transitionProperty).slice(0,10)`);
    res[pref].routeChangeAnimations = [a1, a2];
    await b.eval(`location.hash='#/lab/runs/${ids.sb}'`); await sleep(1500); res[pref].runPage = await motion(pref);
    const scaleDown = res[pref].press.transformWhilePressed !== 'none' && /^matrix\(/.test(res[pref].press.transformWhilePressed) && !/^matrix\(1, 0, 0, 1, 0, 0\)$/.test(res[pref].press.transformWhilePressed);
    res[pref].scaleOnPress = scaleDown;
  }
  if (!args.mode) save('media_motion.json', res);
  const rr = res.reduce;
  R.add('motion.reduced', rr.lab.interactiveTransitions.length === 0 && rr.lab.animated === 0 && !rr.scaleOnPress && rr.lab.infinite.length === 0 ? 'PASS' : 'FAIL', { interactiveTransitions: rr.lab.interactiveTransitions, animated: rr.lab.animated, infinite: rr.lab.infinite, running: rr.lab.running, scaleOnPress: rr.scaleOnPress, press: rr.press, routeChange: rr.routeChangeAnimations, runPage: { animated: rr.runPage.animated, running: rr.runPage.running, transitions: rr.runPage.interactiveTransitions } });
  R.add('motion.no_preference_baseline', 'OBSERVED', { interactiveTransitions: res['no-preference'].lab.interactiveTransitions, animated: res['no-preference'].lab.animated, scaleOnPress: res['no-preference'].scaleOnPress, press: res['no-preference'].press });
  // prefers-contrast: more and forced-colors
  const contrastRes = {};
  for (const mode of args.mode ? args.mode.split(',') : ['base', 'more', 'forced']) {
    const f = [{ name: 'prefers-color-scheme', value: 'light' }, { name: 'prefers-reduced-motion', value: 'no-preference' }];
    if (mode === 'more') f.push({ name: 'prefers-contrast', value: 'more' });
    if (mode === 'forced') f.push({ name: 'forced-colors', value: 'active' });
    await s.setMedia(f);
    b = await s.load('#/lab', 1800);
    for (const [name, hash] of [['lab_home', '#/lab'], ['lab_run_form', '#/lab/run'], ['lab_run_sb', `#/lab/runs/${ids.sb}`], ['overview', '#/overview'], ['lab_evidence_sb', `#/lab/evidence/${SB}`]]) {
      console.error('step', mode, name, 'go'); await s.go(hash, 1400);
      console.error('step probe'); const p = await b.eval(inj(pageProbe, { minPrimary: 32 }));
      console.error('step extra'); const extra = await b.eval(`({matches: matchMedia('(forced-colors: active)').matches, more: matchMedia('(prefers-contrast: more)').matches, btn:(()=>{const x=document.querySelector('button.btn'); if(!x) return null; const cs=getComputedStyle(x); return {border:cs.borderTopWidth+' '+cs.borderTopStyle+' '+cs.borderTopColor, color:cs.color, bg:cs.backgroundColor}})(), badge:(()=>{const x=document.querySelector('.prov'); if(!x) return null; const cs=getComputedStyle(x); return {border:cs.borderTopWidth+' '+cs.borderTopColor, color:cs.color, bg:cs.backgroundColor, forcedAdjust:cs.forcedColorAdjust}})(), svgFill:(()=>{const x=document.querySelector('svg [fill]'); return x?x.getAttribute('fill'):null})()})`);
      // focus ring under this mode
      console.error('step focus'); await b.eval(inj(resetStart)); await b.tab(); await b.tab(); await b.tab(); await sleep(250);
      const fi = await b.eval(inj(focusInfo));
      (contrastRes[mode] ||= {})[name] = { contrastMin: p.contrast.min, contrastFails: p.contrast.fails.length, failSamples: p.contrast.fails.slice(0, 4), indeterminate: p.contrast.indeterminate, nonTextWeak: p.nontext.weak.length, overflow: p.overflow.pageHScroll, ...extra, focus: { name: fi.name, how: fi.how, ringPx: fi.ringPx, color: fi.ringColor } };
      if (mode !== 'base' || name === 'lab_home') await jshot(b, `${mode}__${name}`, 'shots/media', mode !== 'forced', 'png');
    }
  }
  save(`media_contrast_${args.mode || 'all'}.json`, contrastRes);
  if (contrastRes.more) R.add('prefers_contrast_more', 'OBSERVED', Object.fromEntries(Object.entries(contrastRes.more).map(([k, v]) => [k, { min: v.contrastMin, fails: v.contrastFails, mediaMatches: v.more, btn: v.btn }])));
  if (contrastRes.forced) R.add('forced_colors', Object.values(contrastRes.forced).every((v) => v.matches && v.focus.ringPx >= 2) ? 'OBSERVED' : 'FAIL', Object.fromEntries(Object.entries(contrastRes.forced).map(([k, v]) => [k, { matches: v.matches, btn: v.btn, badge: v.badge, focus: v.focus, overflow: v.overflow }])));
  await s.end();
}

async function phaseScale() {
  const s = new Session('scale');
  const SCALE_JS = (factor) => `(()=>{
    const de=document.documentElement; const p=document.querySelector('main p')||document.querySelector('main'); const before=parseFloat(getComputedStyle(p).fontSize);
    de.style.fontSize='${factor}%'; return {before, after:parseFloat(getComputedStyle(p).fontSize)}; })()`;
  const CHECK = `(()=>{
    const de=document.documentElement; const out={overflow:de.scrollWidth>innerWidth+1, scrollW:de.scrollWidth, innerW:innerWidth, clipped:[], overlaps:[], truncatedEllipsis:0};
    const els=[...document.querySelectorAll('body *')].filter(e=>{const r=e.getBoundingClientRect(); const cs=getComputedStyle(e); return r.width>1&&r.height>1&&cs.visibility!=='hidden'&&cs.display!=='none';});
    for(const e of els){const cs=getComputedStyle(e); if(/hidden|clip/.test(cs.overflowX+cs.overflowY)&&!e.closest('.visually-hidden')&&e.tagName!=='HTML'&&e.tagName!=='BODY'){ if(e.scrollWidth>e.clientWidth+1||e.scrollHeight>e.clientHeight+1) out.clipped.push(e.tagName.toLowerCase()+'.'+String(e.className).slice(0,26)+' '+e.scrollWidth+'x'+e.scrollHeight+' in '+e.clientWidth+'x'+e.clientHeight+' ['+(e.innerText||'').slice(0,24).replace(/\\s+/g,' ')+']');} if(cs.textOverflow==='ellipsis'&&e.scrollWidth>e.clientWidth+1) out.truncatedEllipsis++;}
    const leaves=els.filter(e=>[...e.childNodes].some(n=>n.nodeType===3&&n.nodeValue.trim())&&!e.closest('.visually-hidden')&&getComputedStyle(e).position!=='fixed').slice(0,500);
    const sy=scrollY, sx=scrollX; const rects=leaves.map(e=>{const r=e.getBoundingClientRect(); return {e, x:r.left+sx, y:r.top+sy, r:r.right+sx, b:r.bottom+sy};});
    for(let i=0;i<rects.length;i++)for(let j=i+1;j<rects.length;j++){const a=rects[i],c=rects[j]; if(a.e.contains(c.e)||c.e.contains(a.e)) continue; const w=Math.min(a.r,c.r)-Math.max(a.x,c.x), h=Math.min(a.b,c.b)-Math.max(a.y,c.y); if(w>3&&h>3) out.overlaps.push((a.e.className+'|'+a.e.innerText.slice(0,18)+'  X  '+c.e.className+'|'+c.e.innerText.slice(0,18)).replace(/\\s+/g,' ')+' #'+Math.round(w)+'x'+Math.round(h));}
    out.overlaps=out.overlaps.slice(0,60); out.clipped=out.clipped.slice(0,10); return out;})()`;
  const res = {};
  for (const [w, h] of [[1280, 900], [390, 844]]) {
    await s.setVp(w, h); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }]);
    await s.load('#/overview', 1500);
    for (const [name, hash] of [['lab_home', '#/lab'], ['lab_run_form', '#/lab/run'], ['overview', '#/overview'], ['lab_run_sb', `#/lab/runs/${ids.sb}`]]) {
      const b = await s.go(hash, 1300);
      await b.eval("document.documentElement.style.fontSize=''"); await sleep(400);
      const base = await b.eval(CHECK);
      const sc = await b.eval(SCALE_JS(200)); await sleep(700);
      const c = await b.eval(CHECK); const text = await b.eval('document.body.innerText.length');
      const key = (o) => o.split(' #')[0]; const baseSet = new Set(base.overlaps.map(key));
      const newOverlaps = c.overlaps.filter((o) => !baseSet.has(key(o)));
      res[`${w}/${name}`] = { scale: sc, base: { overflow: base.overflow, clipped: base.clipped.length, overlaps: base.overlaps.length }, at200: { ...c, newOverlaps } };
      R.add(`text_scale200.${w}.${name}`, sc.after >= sc.before * 1.9 ? ((c.overflow && !base.overflow) || c.clipped.length > base.clipped.length || newOverlaps.length ? 'FAIL' : 'PASS') : 'OBSERVED', { fontPx: sc, pageHScrollAt200: c.overflow, pageHScrollBaseline: base.overflow, clipped: c.clipped, newOverlapsVsBaseline: newOverlaps.slice(0, 8), baselineOverlapCount: base.overlaps.length, ellipsis: c.truncatedEllipsis, textLen: text }, await jshot(b, `text200_${name}_${w}`, 'shots/scale', true, 'png'));
      await b.eval("document.documentElement.style.fontSize=''");
    }
  }
  // 200% / 400% zoom equivalents via device metrics (width/height divided, DPR multiplied)
  const zres = {};
  for (const z of [{ n: 'zoom200', w: 640, h: 400, d: 2 }, { n: 'zoom400', w: 320, h: 256, d: 4 }]) {
    await s.setVp(z.w, z.h, z.d);
    await s.load(ROUTES[0][1], 1500);
    for (const [name, hash] of ROUTES) {
      const b = await s.go(hash, 1100);
      const o = await b.eval(`({scrollW:document.documentElement.scrollWidth, innerW:innerWidth, dpr:devicePixelRatio, pageHScroll:document.documentElement.scrollWidth>innerWidth+1, wide:[...document.querySelectorAll('body *')].filter(e=>e.getBoundingClientRect().right>innerWidth+2&&getComputedStyle(e).position!=='fixed'&&!e.closest('.table-wrap,pre,[tabindex="0"],.lab-verbatim,[class*=scroll]')).slice(0,4).map(e=>e.tagName.toLowerCase()+'.'+String(e.className).slice(0,28)), inScroller:[...document.querySelectorAll('body *')].filter(e=>e.scrollWidth>e.clientWidth+2&&/auto|scroll/.test(getComputedStyle(e).overflowX)).slice(0,4).map(e=>e.tagName.toLowerCase()+'.'+String(e.className).slice(0,28)), sticky:(()=>{const h=document.querySelector('.app-header'); return h?[getComputedStyle(h).position, Math.round(h.getBoundingClientRect().height)]:null})()})`);
      zres[`${z.n}/${name}`] = o;
      R.add(`reflow.${z.n}.${name}`, o.pageHScroll ? 'FAIL' : 'PASS', { ...o, method: `Emulation.setDeviceMetricsOverride ${z.w}x${z.h} @dpr ${z.d}` });
      if (['lab_home', 'lab_run_form', 'overview'].includes(name)) await jshot(b, `${z.n}_${name}`, 'shots/scale', false, 'jpeg');
    }
  }
  // real page-scale factor probe (pinch zoom: visual viewport only, does not reflow); recorded for transparency
  const b0 = await s.ensure(); await s.setVp(1280, 900); await s.load('#/lab', 1200);
  let psf = null; try { await b0.send('Emulation.setPageScaleFactor', { pageScaleFactor: 2 }); await sleep(200); psf = await b0.eval(`({vvScale:visualViewport.scale, innerW:innerWidth, vvW:Math.round(visualViewport.width)})`); await b0.send('Emulation.setPageScaleFactor', { pageScaleFactor: 1 }); } catch (e) { psf = 'setPageScaleFactor not usable: ' + e.message.slice(0, 120); }
  R.add('reflow.page_scale_factor_probe', 'OBSERVED', { psf, note: 'Emulation.setPageScaleFactor is pinch zoom (visual viewport) and does not re-layout; the 200%/400% reflow verdicts therefore use device-metrics equivalents' });
  save('scale_results.json', { res, zres });
  await s.end();
}

async function phaseProv() {
  const s = new Session('prov'); await s.setVp(1280, 900); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }]);
  await s.load(ROUTES[0][1], 1800); const all = {}; const words = {}; const banned = [];
  for (const [name, hash] of ROUTES) {
    const b = await s.go(hash, 1400);
    const p = await b.eval(inj(provProbe)); all[name] = p;
    for (const [w, n] of Object.entries(p.words)) words[w] = (words[w] || 0) + n;
    for (const h of p.bannedHits) banned.push({ route: name, ...h });
    for (const h of p.attrHits) banned.push({ route: name, attr: true, ...h });
    const issues = { metricsWithoutBadge: p.metricsWithoutBadge, panelsNumbersNoBadge: p.panelsNumbersNoBadge.map((x) => x.heading + ' (' + x.sample.join(',') + ')'), unavailableWithoutReason: p.unavailable.filter((u) => !u.hasReasonText).map((u) => u.text), unavailableBareZero: p.unavailable.filter((u) => u.bareZero).map((u) => u.text), bannedVisible: p.bannedHits.filter((h) => h.visibleText).map((h) => h.word + ': ' + h.ctx) };
    R.add(`prov.${name}`, issues.metricsWithoutBadge.length || issues.unavailableWithoutReason.length || issues.unavailableBareZero.length || issues.bannedVisible.length ? 'FAIL' : (issues.panelsNumbersNoBadge.length ? 'OBSERVED' : 'PASS'), { badges: p.badges, words: p.words, qualifiers: p.qualifiers, ...issues, bannedInTooltipsOrLabels: p.attrHits.map((h) => h.attr + ': ' + h.value) });
  }
  const allowed = new Set(['Real', 'Simulated', 'Replayed', 'Predicted', 'Derived', 'Unavailable']);
  const odd = Object.keys(words).filter((w) => !allowed.has(w));
  R.add('prov.badge_vocabulary', odd.length ? 'OBSERVED' : 'PASS', { words, outsideSixWordSet: odd });
  save('prov_detail.json', { all, banned });
  await s.end();
}

async function phaseDisconnect() {
  const gwPid = args.gwpid; const s = new Session('disc'); await s.setVp(1280, 900); await s.setMedia([{ name: 'prefers-color-scheme', value: 'light' }]);
  const b = await s.load('#/overview', 3500);
  await b.eval(`window.__ann=[]; new MutationObserver(ms=>{for(const m of ms){const t=m.target.nodeType===3?m.target.parentElement:m.target; const el=t&&t.closest&&t.closest('[role=status],[role=alert],[aria-live]'); if(el) window.__ann.push({t:(performance.now()|0), role:el.getAttribute('role'), live:el.getAttribute('aria-live'), text:el.innerText.replace(/\\s+/g,' ').slice(0,160)});}}).observe(document.body,{subtree:true,childList:true,characterData:true});`);
  const snap = () => b.eval(`({chip:(document.querySelector('.app-header__status')||{}).innerText, banners:[...document.querySelectorAll('[data-connection]')].map(e=>e.getAttribute('data-connection')+': '+e.innerText.replace(/\\s+/g,' ').slice(0,160)), live:(document.querySelector('[data-live-region]')||{}).innerText, metricStates:Object.entries([...document.querySelectorAll('.metric')].reduce((a,m)=>{const k=m.getAttribute('data-state'); a[k]=(a[k]||0)+1; return a;},{})), sampleMetrics:[...document.querySelectorAll('.metric')].slice(0,4).map(m=>m.innerText.replace(/\\s+/g,' ').slice(0,100)), regionStates:[...document.querySelectorAll('[data-state]:not(.metric)')].slice(0,6).map(e=>e.getAttribute('data-state')+':'+e.innerText.replace(/\\s+/g,' ').slice(0,80))})`);
  const before = await snap(); await jshot(b, 'connected_overview', 'shots/disconnect', false, 'png');
  // run page before
  let tKill = Date.now();
  if (gwPid) execSync(`taskkill /PID ${gwPid} /T /F`, { stdio: 'ignore' });
  await sleep(500);
  const gone = !pidAlive(+gwPid);
  const timeline = [];
  for (let i = 0; i < 24; i++) { await sleep(2500); const sn = await snap(); timeline.push({ sec: Math.round((Date.now() - tKill) / 1000), chip: sn.chip, live: sn.live, banners: sn.banners.map((x) => x.slice(0, 80)), states: sn.metricStates }); if (/disconnect|error|lost|offline/i.test(sn.chip || '') && i > 2) break; }
  const after = await snap(); await jshot(b, 'disconnected_overview_1280', 'shots/disconnect', true, 'png');
  const ann = await b.eval('window.__ann');
  const p = await b.eval(inj(pageProbe, { minPrimary: 32 }));
  await b.send('Accessibility.enable'); const ax = (await b.send('Accessibility.getFullAXTree')).nodes.filter((n) => !n.ignored && n.role && ['status', 'alert'].includes(n.role.value)).map((n) => n.role.value + ':' + ((n.name && n.name.value) || '') + '|' + (n.properties || []).filter((x) => ['live', 'atomic'].includes(x.name)).map((x) => x.name + '=' + x.value.value).join(','));
  R.add('disconnect.overview', /disconnect|lost|offline|error/i.test(after.chip || '') && ann.length ? 'PASS' : 'FAIL', { gatewayKilled: gwPid, processGone: gone, before, after, timeline, announcements: ann.slice(0, 12), axLiveRegions: ax, contrastFailsInState: p.contrast.fails.length, contrastFailSamples: p.contrast.fails.slice(0, 4) });
  // other regions while disconnected
  for (const [name, hash] of [['traffic', '#/traffic'], ['lab_home', '#/lab'], ['lab_runs', '#/lab/runs'], ['lab_run_sb', `#/lab/runs/${ids.sb}`], ['lab_run_form', '#/lab/run'], ['lab_evidence', '#/lab/evidence']]) {
    await s.go(hash, 2200);
    const o = await b.eval(`({chip:(document.querySelector('.app-header__status')||{}).innerText, alerts:[...document.querySelectorAll('[role=alert]')].map(e=>e.innerText.replace(/\\s+/g,' ').slice(0,160)), statuses:[...document.querySelectorAll('[role=status]')].map(e=>e.innerText.replace(/\\s+/g,' ').slice(0,100)).filter(Boolean).slice(0,6), stateBlocks:[...document.querySelectorAll('[data-state],.state')].slice(0,5).map(e=>e.innerText.replace(/\\s+/g,' ').slice(0,100)), mainText:(document.querySelector('main')||{}).innerText.replace(/\\s+/g,' ').slice(0,260), metricStates:Object.entries([...document.querySelectorAll('.metric')].reduce((a,m)=>{const k=m.getAttribute('data-state'); a[k]=(a[k]||0)+1; return a;},{}))})`);
    const pp = await b.eval(inj(pageProbe, { minPrimary: 32 }));
    o.contrastFails = pp.contrast.fails.length; o.names = pp.unnamed.length; o.h1 = pp.h1;
    R.add(`disconnect.${name}`, o.chip && /disconnect|lost|offline|error/i.test(o.chip) && (o.alerts.length || o.stateBlocks.length || o.statuses.length) ? 'OBSERVED' : 'FAIL', o, await jshot(b, `disconnected_${name}`, 'shots/disconnect', false, 'png'));
  }
  await s.setVp(390, 844); await s.go('#/overview', 1500); await jshot(b, 'disconnected_overview_390', 'shots/disconnect', true, 'png');
  save('disconnect_announcements.json', await b.eval('window.__ann'));
  await s.end();
}

try {
  const fn = { prep: phasePrep, matrix: phaseMatrix, ax: phaseAx, kbd: phaseKbd, media: phaseMedia, scale: phaseScale, prov: phaseProv, disconnect: phaseDisconnect }[PHASE];
  if (!fn) throw new Error('unknown phase ' + PHASE);
  await fn();
} catch (e) { R.add(`${PHASE}.exception`, 'FAIL', String((e && e.stack) || e)); }
finally { R.save(); }
