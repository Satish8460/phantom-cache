// Small dependency-free CDP helper shared by browser-journey.mjs and browser-a11y.mjs (Node 24 built-ins only).
// Ownership rules: it launches ONLY its own Chrome with a throwaway profile and kills ONLY that process tree.
import { spawn, execSync } from 'node:child_process';
import { mkdirSync, writeFileSync, existsSync, rmSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
export const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';

export function pidAlive(pid) {
  try { return execSync(`tasklist /FI "PID eq ${pid}" /NH`).toString().includes(String(pid)); } catch { return false; }
}

export class Browser {
  constructor({ evidenceDir, label }) {
    this.evidenceDir = evidenceDir;
    this.label = label;
    this.port = 9600 + Math.floor(Math.random() * 300);
    this.profile = `D:/Temp/browser_profile_${label}_${Math.random().toString(36).slice(2, 8)}`;
    this.consoleLog = [];
    this.netLog = [];
    this.reqById = new Map();
  }
  async launch() {
    mkdirSync(this.profile, { recursive: true });
    this.proc = spawn(CHROME, [
      `--remote-debugging-port=${this.port}`, `--user-data-dir=${this.profile}`, '--headless=new', '--no-first-run',
      '--no-default-browser-check', '--disable-gpu', '--disable-extensions', 'about:blank',
    ], { stdio: 'ignore', windowsHide: true });
    this.pid = this.proc.pid;
    let target;
    for (let i = 0; i < 150 && !target; i++) {
      try {
        const list = await (await fetch(`http://127.0.0.1:${this.port}/json`)).json();
        target = list.find((t) => t.type === 'page');
      } catch { /* not up */ }
      if (!target) await sleep(100);
    }
    if (!target) throw new Error('chrome did not expose a page target');
    this.version = (await (await fetch(`http://127.0.0.1:${this.port}/json/version`)).json()).Browser;
    this.ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((r, j) => { this.ws.onopen = r; this.ws.onerror = j; });
    this.id = 0; this.pending = new Map(); this.waiters = [];
    this.ws.onmessage = (m) => {
      const msg = JSON.parse(m.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { res, rej } = this.pending.get(msg.id); this.pending.delete(msg.id);
        msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
      } else if (msg.method) this.onEvent(msg);
    };
    for (const d of ['Page', 'Runtime', 'Network', 'Log']) await this.send(`${d}.enable`);
    await this.send('Network.setCacheDisabled', { cacheDisabled: true });
    return this;
  }
  send(method, params = {}) {
    const id = ++this.id;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((res, rej) => this.pending.set(id, { res, rej }));
  }
  onEvent(e) {
    const p = e.params; const t = new Date().toISOString();
    if (e.method === 'Runtime.consoleAPICalled') this.consoleLog.push({ t, kind: 'console.' + p.type, text: p.args.map((a) => a.value ?? a.description ?? a.type).join(' ').slice(0, 500) });
    else if (e.method === 'Runtime.exceptionThrown') this.consoleLog.push({ t, kind: 'exception', text: (p.exceptionDetails.exception?.description || p.exceptionDetails.text || '').slice(0, 600) });
    else if (e.method === 'Log.entryAdded') this.consoleLog.push({ t, kind: 'log.' + p.entry.level, text: `${p.entry.source}: ${p.entry.text} ${p.entry.url || ''}`.slice(0, 500) });
    else if (e.method === 'Network.requestWillBeSent') { this.reqById.set(p.requestId, { t, method: p.request.method, url: p.request.url, hasAuth: Boolean(p.request.headers?.Authorization || p.request.headers?.authorization) }); }
    else if (e.method === 'Network.responseReceived') { const r = this.reqById.get(p.requestId); if (r) { r.status = p.response.status; r.mime = p.response.mimeType; } }
    else if (e.method === 'Network.loadingFinished') { const r = this.reqById.get(p.requestId); if (r) { r.encodedBytes = p.encodedDataLength; r.finished = true; } }
    else if (e.method === 'Network.loadingFailed') { const r = this.reqById.get(p.requestId) || {}; r.failed = `${p.errorText}${p.blockedReason ? ' blocked:' + p.blockedReason : ''}${p.corsErrorStatus ? ' cors:' + p.corsErrorStatus.corsError : ''}`; r.canceled = p.canceled; this.reqById.set(p.requestId, r); }
    for (const w of this.waiters.slice()) if (w.method === e.method) w.fn(e);
  }
  async eval(expression) {
    const r = await this.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error('eval failed: ' + (r.exceptionDetails.exception?.description || r.exceptionDetails.text));
    return r.result.value;
  }
  async goto(url, settle = 1200) { await this.send('Page.navigate', { url }); await sleep(settle); }
  async viewport(width, height, mobile = false, dpr = 1) {
    await this.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: dpr, mobile });
  }
  async media(features) { await this.send('Emulation.setEmulatedMedia', { features }); }
  async shot(name, dir = 'shots', fullPage = false) {
    const d = join(this.evidenceDir, dir); mkdirSync(d, { recursive: true });
    const s = await this.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: fullPage });
    writeFileSync(join(d, name), Buffer.from(s.data, 'base64'));
    return `${dir}/${name}`;
  }
  async key(key, { code = key, vk = 0, shift = false } = {}) {
    const mods = shift ? 8 : 0;
    await this.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode: vk, modifiers: mods, text: key.length === 1 ? key : undefined });
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: vk, modifiers: mods });
  }
  tab(shift = false) { return this.key('Tab', { code: 'Tab', vk: 9, shift }); }
  async waitFor(expr, timeoutMs = 15000, every = 250) {
    const t0 = Date.now();
    while (Date.now() - t0 < timeoutMs) {
      try { const v = await this.eval("!!(" + expr + ")"); if (v) return v; } catch { /* page navigating */ }
      await sleep(every);
    }
    return false;
  }
  /** Click an element found by a JS predicate expression returning the element. */
  async click(findExpr) {
    return this.eval(`(() => { const el = (${findExpr}); if (!el) return false; el.scrollIntoView({block:'center'}); el.click(); return true; })()`);
  }
  async clickText(text, sel = 'button,a,[role=button]') {
    return this.click(`[...document.querySelectorAll(${JSON.stringify(sel)})].find(e => (e.innerText||e.getAttribute('aria-label')||'').trim().toLowerCase().startsWith(${JSON.stringify(text.toLowerCase())}) && !e.disabled)`);
  }
  async setField(labelStart, value) {
    return this.eval(`(() => {
      const lab = [...document.querySelectorAll('label')].find(l => l.textContent.trim().toLowerCase().startsWith(${JSON.stringify(labelStart.toLowerCase())}));
      if (!lab) return 'no label';
      const el = document.getElementById(lab.htmlFor); if (!el) return 'no input';
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(el, ${JSON.stringify(String(value))});
      el.dispatchEvent(new Event('input', { bubbles: true })); return 'ok'; })()`);
  }
  async storageAndCookies() {
    const inPage = await this.eval(`(() => { const dump = (s) => { try { return Object.entries(s).map(([k,v]) => [k, String(v).slice(0,80)]); } catch (e) { return 'unavailable ' + e; } };
      return { local: dump(localStorage), session: dump(sessionStorage), href: location.href, cookie: document.cookie }; })()`);
    let cookies = [];
    try { cookies = (await this.send('Storage.getCookies')).cookies; } catch (e) { cookies = 'Storage.getCookies failed: ' + e; }
    let idb = null;
    try { idb = await this.eval(`indexedDB.databases ? indexedDB.databases().then(d => d.map(x => x.name)) : 'unsupported'`); } catch { /* ignore */ }
    return { ...inPage, cookies, indexedDB: idb };
  }
  saveLogs(prefix) {
    const d = join(this.evidenceDir, 'logs'); mkdirSync(d, { recursive: true });
    writeFileSync(join(d, `${prefix}.console.json`), JSON.stringify(this.consoleLog, null, 1));
    writeFileSync(join(d, `${prefix}.network.json`), JSON.stringify([...this.reqById.values()], null, 1));
  }
  async close() {
    try { this.ws?.close(); } catch { /* ignore */ }
    if (this.pid && pidAlive(this.pid)) { try { execSync(`taskkill /PID ${this.pid} /T /F`, { stdio: 'ignore' }); } catch { /* ignore */ } }
    await sleep(500);
    this.pidGone = !pidAlive(this.pid);
    // Remove the throwaway profile (our own directory under D:/Temp/browser_profile_*).
    try { if (/browser_profile_/.test(this.profile)) rmSync(this.profile, { recursive: true, force: true }); } catch { /* may be locked briefly */ }
    return this.pidGone;
  }
}

export class Results {
  constructor(file) { this.file = file; this.items = []; }
  add(id, status, detail, shot) {
    const it = { id, status, detail, shot: shot || null, t: new Date().toISOString() };
    this.items.push(it);
    console.log(`[${status}] ${id} ${typeof detail === 'string' ? detail.slice(0, 160) : ''}`);
    appendFileSync(this.file + '.jsonl', JSON.stringify(it) + '\n');
    return it;
  }
  save() { writeFileSync(this.file, JSON.stringify(this.items, null, 2)); }
}
