// Dependency-free CDP measurement for docs/design/prototypes/materials_perf.html (Node 24 built-ins + ui/scripts/cdp-lib.mjs).
// Usage: node run_perf.mjs <gpu|sw> [reps] [variants,comma] [out.json]
//   sw  = Chrome headless with --disable-gpu (software compositing, same flags as cdp-lib.mjs)
//   gpu = Chrome headless WITHOUT --disable-gpu (whatever the host GPU/ANGLE path gives; recorded via SystemInfo.getInfo)
// Frame timing: rAF deltas collected in-page while a scripted scroll runs (main-thread driven). Trace rep: Tracing for paint/raster/composite time.
// Owns only its own Chrome process (throwaway profile under D:/Temp) and kills only that process tree.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { Browser, CHROME, sleep } from '../../../../ui/scripts/cdp-lib.mjs';

const mode = process.argv[2] || 'sw';
const REPS = Number(process.argv[3] || 5);
const VARIANTS = (process.argv[4] || 'a,b,c,d,d32,e,f,g,h').split(',');
const OUT = process.argv[5] || `D:/Personal/phantom-cache/docs/design/prototypes/perf/perf_${mode}.json`;
const PAGE = pathToFileURL('D:/Personal/phantom-cache/docs/design/prototypes/materials_perf.html').href;
const VIEWPORTS = [
  { name: 'desktop-1280x800', w: 1280, h: 800, mobile: false, dpr: 1, cpu: 1 },
  { name: 'mobile-390x844-cpu4x', w: 390, h: 844, mobile: true, dpr: 2, cpu: 4 },
];
const SCROLL_MS = 3000, PXPS = 1800;

class PerfBrowser extends Browser {
  async launch() {
    mkdirSync(this.profile, { recursive: true });
    const args = [`--remote-debugging-port=${this.port}`, `--user-data-dir=${this.profile}`, '--headless=new', '--no-first-run',
      '--no-default-browser-check', '--disable-extensions', 'about:blank'];
    if (mode.startsWith('sw')) args.splice(4, 0, '--disable-gpu');
    if (mode.endsWith('uncapped')) args.push('--disable-frame-rate-limit', '--disable-gpu-vsync');
    this.proc = spawn(CHROME, args, { stdio: 'ignore', windowsHide: true });
    this.pid = this.proc.pid;
    let target;
    for (let i = 0; i < 150 && !target; i++) {
      try { const list = await (await fetch(`http://127.0.0.1:${this.port}/json`)).json(); target = list.find((t) => t.type === 'page'); } catch { /* not up */ }
      if (!target) await sleep(100);
    }
    this.version = (await (await fetch(`http://127.0.0.1:${this.port}/json/version`)).json()).Browser;
    this.ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((r, j) => { this.ws.onopen = r; this.ws.onerror = j; });
    this.id = 0; this.pending = new Map(); this.waiters = []; this.trace = []; this.layers = null;
    this.ws.onmessage = (m) => {
      const msg = JSON.parse(m.data);
      if (msg.id && this.pending.has(msg.id)) { const { res, rej } = this.pending.get(msg.id); this.pending.delete(msg.id); msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result); }
      else if (msg.method === 'Tracing.dataCollected') this.trace.push(...msg.params.value);
      else if (msg.method === 'LayerTree.layerTreeDidChange') { if (msg.params.layers) { this.layers = msg.params.layers.length; this.maxLayers = Math.max(this.maxLayers||0, this.layers); } }
      else if (msg.method) this.onEvent(msg);
    };
    await this.send('Page.enable'); await this.send('Runtime.enable');
    await this.send('Network.setCacheDisabled', { cacheDisabled: true }).catch(() => {});
  }
}

const med = (a) => { const s = [...a].sort((x, y) => x - y); const n = s.length; return n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2; };
const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(p * s.length))]; };
const stat = (a) => ({ median: +med(a).toFixed(2), min: +Math.min(...a).toFixed(2), max: +Math.max(...a).toFixed(2) });

function traceBreakdown(events) {
  const names = ['Paint', 'PrePaint', 'Layerize', 'Commit', 'UpdateLayoutTree', 'Layout', 'RasterTask', 'CompositeLayers', 'DrawFrame', 'FunctionCall', 'UpdateLayer'];
  const sum = Object.fromEntries(names.map((n) => [n, 0])); let gpuTask = 0;
  for (const e of events) {
    if (e.ph !== 'X' || !e.dur) continue;
    if (names.includes(e.name)) sum[e.name] += e.dur / 1000;
    if (e.name === 'GPUTask') gpuTask += e.dur / 1000;
  }
  for (const k of Object.keys(sum)) sum[k] = +sum[k].toFixed(1);
  sum.GPUTask = +gpuTask.toFixed(1);
  return sum; // ms, summed across all threads/processes during the traced scroll
}

const b = new PerfBrowser({ evidenceDir: 'D:/Temp', label: 'perf' });
const result = { mode, started: new Date().toISOString(), scrollMs: SCROLL_MS, pxPerSec: PXPS, reps: REPS, rows: [] };
try {
  await b.launch();
  result.browser = b.version;
  try { const si = await b.send('SystemInfo.getInfo'); result.gpu = (si.gpu?.devices || []).map((d) => d.deviceString || d.vendorString); result.glRenderer = si.gpu?.auxAttributes?.glRenderer || null; result.compositing = si.gpu?.featureStatus || null; } catch (e) { result.gpu = 'SystemInfo unavailable ' + e.message; }
  result.cores = 24;
  try { await b.goto('about:blank',200); result.glRendererInPage = await b.eval("(()=>{const c=document.createElement('canvas');const g=c.getContext('webgl');if(!g)return 'no webgl';const e=g.getExtension('WEBGL_debug_renderer_info');return e?g.getParameter(e.UNMASKED_RENDERER_WEBGL):'no debug info'})()"); } catch (e) { result.glRendererInPage = 'err ' + e.message; }
  for (const vp of VIEWPORTS) {
    await b.viewport(vp.w, vp.h, vp.mobile, vp.dpr);
    await b.send('Emulation.setCPUThrottlingRate', { rate: vp.cpu });
    for (const v of VARIANTS) {
      const frames = [], long17 = [], long34 = [], p95s = [], maxs = [], means = [];
      for (let r = 0; r < REPS; r++) {
        await b.goto(`${PAGE}?v=${v}`, 1500);
        const out = await b.eval(`window.__scroll(${SCROLL_MS},${PXPS}).then(o=>({d:o.d.slice(3),h:o.h}))`);
        const d = out.d;
        frames.push(d.length); long17.push(d.filter((x) => x > 17.7).length); long34.push(d.filter((x) => x > 34).length);
        p95s.push(pct(d, 0.95)); maxs.push(Math.max(...d)); means.push(d.reduce((s, x) => s + x, 0) / d.length);
      }
      // one traced + layer-tree rep (tracing adds overhead; frame stats above are from untraced reps)
      b.trace = []; b.layers = null; b.maxLayers = 0;
      try { await b.send('LayerTree.enable'); } catch { /* ignore */ }
      await b.goto(`${PAGE}?v=${v}`, 1500);
      await b.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-devtools.timeline,cc,gpu', transferMode: 'ReportEvents' });
      await b.eval(`window.__scroll(${SCROLL_MS},${PXPS}).then(()=>1)`);
      const done = new Promise((res) => { const h = b.ws.onmessage; b.ws.addEventListener('message', (m) => { try { if (JSON.parse(m.data).method === 'Tracing.tracingComplete') res(); } catch { /* ignore */ } }); });
      await b.send('Tracing.end'); await Promise.race([done, sleep(8000)]);
      const layers = b.maxLayers || null; try { await b.send('LayerTree.disable'); } catch { /* ignore */ }
      const row = {
        viewport: vp.name, variant: v, framesPerRun: stat(frames), long_gt17_7ms: stat(long17), long_gt34ms: stat(long34),
        meanFrameMs: stat(means), p95FrameMs: stat(p95s), maxFrameMs: stat(maxs), layerCount: layers, traceMsSummedAllThreads: traceBreakdown(b.trace),
        raw: { long17, long34, means: means.map((x) => +x.toFixed(2)), p95s: p95s.map((x) => +x.toFixed(2)), maxs: maxs.map((x) => +x.toFixed(2)) },
      };
      result.rows.push(row);
      console.log(vp.name, v, 'frames', row.framesPerRun.median, 'long>17.7', row.long_gt17_7ms.median, '>34', row.long_gt34ms.median, 'p95', row.p95FrameMs.median, 'max', row.maxFrameMs.median, 'layers', layers);
      writeFileSync(OUT, JSON.stringify(result, null, 1));
    }
  }
} finally {
  result.pidKilled = await b.close();
  result.finished = new Date().toISOString();
  writeFileSync(OUT, JSON.stringify(result, null, 1));
}
