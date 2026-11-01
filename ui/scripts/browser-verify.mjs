// Browser verification driver (no dependencies): launches a locally installed Chrome/Edge headless,
// drives it over the DevTools protocol using Node built-ins (global WebSocket, fetch), and records
// evidence for a running UI: screenshots per viewport/theme, console errors, failed requests,
// horizontal overflow, keyboard focus order, and a small set of DOM expectations.
//
// Usage:
//   node ui/scripts/browser-verify.mjs --url http://127.0.0.1:3000/ --out D:/Temp/browser-evidence \
//        [--browser "C:/Program Files/Google/Chrome/Application/chrome.exe"] [--routes "#/,#/traffic,#/origin"]
//
// This script only OBSERVES. A pass here is not a conformance claim: it records what a real browser did
// at the sizes and themes tested. It never starts the gateway or the UI server.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, arr) => (a.startsWith('--') ? [...acc, [a.slice(2), arr[i + 1]]] : acc), []),
);
const url = args.url;
const out = args.out;
if (!url || !out) {
  console.error('required: --url <ui url> --out <evidence dir>');
  process.exit(2);
}
const candidates = [
  args.browser,
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
].filter(Boolean);
const browser = candidates.find((p) => existsSync(p));
if (!browser) {
  console.error('no Chrome/Edge found; pass --browser');
  process.exit(2);
}
const routes = (args.routes || '#/').split(',');
const viewports = [
  { name: 'mobile', width: 390, height: 844, mobile: true },
  { name: 'tablet', width: 820, height: 1180, mobile: true },
  { name: 'laptop', width: 1280, height: 800, mobile: false },
  { name: 'desktop', width: 1920, height: 1080, mobile: false },
];
const themes = ['light', 'dark'];
mkdirSync(out, { recursive: true });
const port = 9222 + Math.floor(Math.random() * 500);
const profile = join(out, '_profile');
const proc = spawn(
  browser,
  [`--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--headless=new', '--no-first-run', '--disable-gpu', 'about:blank'],
  { stdio: 'ignore' },
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function waitForTarget() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page;
    } catch {
      /* not up yet */
    }
    await sleep(100);
  }
  throw new Error('browser did not expose a page target');
}

class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.events = [];
    ws.onmessage = (m) => {
      const msg = JSON.parse(m.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { res, rej } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
      } else if (msg.method) this.events.push(msg);
    };
  }
  send(method, params = {}) {
    const id = ++this.id;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((res, rej) => this.pending.set(id, { res, rej }));
  }
  drain() {
    const e = this.events;
    this.events = [];
    return e;
  }
}

const report = { url, browser, started: new Date().toISOString(), results: [] };
try {
  const target = await waitForTarget();
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((r, j) => ((ws.onopen = r), (ws.onerror = j)));
  const cdp = new Cdp(ws);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await cdp.send('Log.enable');
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  report.browserVersion = version.Browser;

  for (const vp of viewports) {
    for (const theme of themes) {
      for (const route of routes) {
        await cdp.send('Emulation.setDeviceMetricsOverride', {
          width: vp.width, height: vp.height, deviceScaleFactor: 1, mobile: vp.mobile,
        });
        await cdp.send('Emulation.setEmulatedMedia', {
          features: [{ name: 'prefers-color-scheme', value: theme }, { name: 'prefers-reduced-motion', value: 'no-preference' }],
        });
        cdp.drain();
        const target = url.replace(/#.*$/, '') + route;
        await cdp.send('Page.navigate', { url: target });
        await sleep(2500); // allow first SSE frame / render; states (loading, live, disconnected) are whatever really occurs
        const evalJs = async (expression) =>
          (await cdp.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.value;
        const dom = await evalJs(`(() => {
          const de = document.documentElement;
          const focusables = [...document.querySelectorAll('a[href],button,input,select,textarea,[tabindex]')]
            .filter(e => !e.disabled && e.tabIndex >= 0);
          const small = focusables.filter(e => { const r = e.getBoundingClientRect(); return r.width > 0 && (r.width < 24 || r.height < 24); }).length;
          return {
            title: document.title,
            h1: [...document.querySelectorAll('h1')].map(h => h.textContent.trim()),
            landmarks: ['header','nav','main'].map(t => [t, document.querySelectorAll(t).length]),
            scrollWidth: de.scrollWidth, innerWidth: window.innerWidth,
            horizontalOverflow: de.scrollWidth > window.innerWidth + 1,
            focusableCount: focusables.length, smallTargets: small,
            imagesWithoutAlt: [...document.images].filter(i => !i.hasAttribute('alt')).length,
            bodyTextSample: document.body.innerText.slice(0, 400),
            dataTheme: de.getAttribute('data-theme'),
            colorScheme: getComputedStyle(de).colorScheme,
          };
        })()`);
        // Keyboard: press Tab 5 times and record the focused element each time (visible-focus check is by screenshot).
        const focusTrail = [];
        for (let i = 0; i < 5; i++) {
          await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
          await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
          focusTrail.push(await evalJs(`(() => { const a = document.activeElement; if (!a) return null;
            const cs = getComputedStyle(a); return { tag: a.tagName, text: (a.innerText||a.getAttribute('aria-label')||'').trim().slice(0,40),
            outline: cs.outlineStyle + ' ' + cs.outlineWidth, boxShadow: cs.boxShadow.slice(0,60) }; })()`));
        }
        const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
        const name = `${vp.name}_${theme}_${route.replace(/[^a-z0-9]+/gi, '_') || 'root'}.png`;
        writeFileSync(join(out, name), Buffer.from(shot.data, 'base64'));
        const events = cdp.drain();
        const consoleErrors = events
          .filter((e) => (e.method === 'Runtime.exceptionThrown') ||
            (e.method === 'Runtime.consoleAPICalled' && ['error', 'assert'].includes(e.params.type)) ||
            (e.method === 'Log.entryAdded' && e.params.entry.level === 'error'))
          .map((e) => JSON.stringify(e.params).slice(0, 300));
        const failedRequests = events
          .filter((e) => e.method === 'Network.loadingFailed')
          .map((e) => `${e.params.errorText}${e.params.blockedReason ? ' blocked:' + e.params.blockedReason : ''}`);
        report.results.push({ viewport: vp.name, theme, route, screenshot: name, dom, focusTrail, consoleErrors, failedRequests });
      }
    }
  }
  report.finished = new Date().toISOString();
  ws.close();
} catch (e) {
  report.error = String(e);
  process.exitCode = 1;
} finally {
  proc.kill();
  writeFileSync(join(out, 'report.json'), JSON.stringify(report, null, 2));
  console.log(`wrote ${join(out, 'report.json')} (${report.results.length} captures)${report.error ? ' ERROR: ' + report.error : ''}`);
}
