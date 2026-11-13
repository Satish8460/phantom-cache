// scripts/live_interactive_demo.mjs
// Authoritative automated demonstrator test script verifying Control Center functionality against live stack.
// Adheres to WP7 / F11: strict assertions, non-zero exit on failure, configurable evidence path.

import { mkdirSync, copyFileSync, existsSync, rmSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const args = process.argv.slice(2);
const outArgIdx = args.indexOf('--output-dir');
const SHOTS_DIR = outArgIdx !== -1 && args[outArgIdx + 1]
  ? resolve(args[outArgIdx + 1])
  : resolve('docs/demo/live_demo_shots/repaired');

const uiUrlArgIdx = args.indexOf('--ui-url');
const UI_URL = uiUrlArgIdx !== -1 && args[uiUrlArgIdx + 1]
  ? args[uiUrlArgIdx + 1]
  : 'http://localhost:3000';

const gatewayArgIdx = args.indexOf('--gateway');
const GATEWAY_URL = gatewayArgIdx !== -1 && args[gatewayArgIdx + 1]
  ? args[gatewayArgIdx + 1]
  : 'http://localhost:8085';

mkdirSync(SHOTS_DIR, { recursive: true });

const log = (msg) => console.log(`[DEMO-TEST] ${msg}`);

// A failed assertion throws (instead of process.exit) so that the `finally` cleanup in run() always
// executes: it closes Chrome and removes the temporary profile directory on the failure path too (PC-AUTO-040).
class DemoFailure extends Error {}
const fail = (msg) => {
  throw new DemoFailure(msg);
};

// Independent backend reading. MANDATORY: if the gateway cannot be read the run fails (exit 1); a wrong
// or dead --gateway must never produce a pass (PC-AUTO-040). Returns the counters used for cross-checks.
async function readBackend(label) {
  let body;
  try {
    const res = await fetch(`${GATEWAY_URL}/api/v1/metrics`, { signal: AbortSignal.timeout(5000) });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    body = await res.json();
  } catch (e) {
    return fail(`Backend metrics unreadable at ${GATEWAY_URL} (${label}): ${e.message}`);
  }
  const m = body?.metrics ?? {};
  const num = (x) => (x && typeof x.v === 'number' ? x.v : null);
  const out = {
    requests: num(m.requests_total),
    hits: num(m.cache_hits),
    misses: num(m.cache_misses),
    coalesced: num(m.coalesced_waiters),
    origin: num(m.origin?.requests_started),
  };
  for (const [k, v] of Object.entries(out)) {
    if (v === null) fail(`Backend metric "${k}" is not a measured number at ${GATEWAY_URL} (${label}): ${JSON.stringify(body?.metrics?.[k] ?? body?.metrics?.origin)}`);
  }
  return out;
}
const delta = (after, before) => ({
  requests: after.requests - before.requests,
  hits: after.hits - before.hits,
  misses: after.misses - before.misses,
  coalesced: after.coalesced - before.coalesced,
  origin: after.origin - before.origin,
});
function expectDelta(label, d, expected) {
  for (const [k, v] of Object.entries(expected)) {
    if (d[k] !== v) fail(`Backend cross-check "${label}": expected ${k} delta ${v}, observed ${d[k]} (all deltas: ${JSON.stringify(d)})`);
  }
  log(`  Backend cross-check "${label}" OK: ${JSON.stringify(d)}`);
}

async function run() {
  // Preflight (before Chrome is started): the gateway must answer with readable measured counters.
  await readBackend('preflight');
  log(`Launching headless Chrome for demonstrator verification... (UI: ${UI_URL}, Gateway: ${GATEWAY_URL}, Evidence: ${SHOTS_DIR})`);
  const b = new Browser({ evidenceDir: SHOTS_DIR, label: 'demonstrator' });
  const evidence = [];
  let failure = null;

  const capture = async (name, fileName) => {
    await b.shot(fileName, 'shots');
    const fullSrc = join(SHOTS_DIR, 'shots', fileName);
    evidence.push({ name, file: fileName, path: fullSrc });
    log(`  Screenshot captured: ${fileName} (${name})`);
  };

  try {
    await b.launch();
    await b.viewport(1440, 900);

    // -------------------------------------------------------------------------
    // 1. Navigation to Overview
    // -------------------------------------------------------------------------
    log(`Step 1: Navigating to ${UI_URL}...`);
    await b.goto(UI_URL, 2500);

    const headerText = await b.eval("document.querySelector('.app-header__status')?.innerText || ''");
    log(`  Header status: "${headerText.trim().replace(/\\n/g, ' ')}"`);
    if (!headerText.includes('8085') && !headerText.includes('Live')) {
      fail(`Gateway status indicator unexpected: "${headerText}"`);
    }

    const overviewH2s = await b.eval("Array.from(document.querySelectorAll('h2')).map(h => h.innerText.trim())");
    log(`  Overview sections: ${JSON.stringify(overviewH2s)}`);
    if (!overviewH2s.includes('Traffic') || !overviewH2s.includes('Freshness guard') || !overviewH2s.includes('Memory and origin')) {
      fail(`Overview missing core signal sections: ${JSON.stringify(overviewH2s)}`);
    }
    await capture('Overview Live Signals', '01_overview_clean.png');

    // -------------------------------------------------------------------------
    // 2. Cache & Memory Operations (Cold GET, Warm GET, Stampede, DELETE, Refetch)
    // -------------------------------------------------------------------------
    log('Step 2: Navigating to Cache & Memory (hash: #/cache)...');
    await b.eval("window.location.hash = '#/cache'");
    await sleep(1500);

    const hasConsole = await b.eval("Boolean(document.querySelector('[data-interactive-console]'))");
    if (!hasConsole) fail('Interactive Console not rendered on #/cache screen');
    log('  Interactive Console rendered on Cache screen.');

    // 2.1 Cold GET
    const testKey = `/api/v1/cache/demo-${Date.now()}`;
    log(`  Executing Cold GET for unique key: ${testKey}`);
    const beforeCold = await readBackend('before cold GET');
    await b.eval(`(() => {
      const inp = document.querySelector('[data-interactive-console] input');
      if (inp) {
        const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
        setter.call(inp, '${testKey}');
        inp.dispatchEvent(new Event('input', { bubbles: true }));
      }
      const btn = Array.from(document.querySelectorAll('[data-interactive-console] button')).find(b => b.innerText.includes('Send GET'));
      if (btn) btn.click();
    })()`);
    await sleep(1500);

    const coldText = await b.eval("document.querySelector('.console-result')?.innerText || ''");
    log(`  Cold GET response: "${coldText.replace(/\\n/g, ' | ')}"`);
    if (!coldText.includes('MISS')) fail(`Cold GET did not report MISS: "${coldText}"`);
    if (!coldText.includes('200')) fail(`Cold GET did not return HTTP 200: "${coldText}"`);
    expectDelta('cold GET', delta(await readBackend('after cold GET'), beforeCold), { requests: 1, misses: 1, hits: 0, origin: 1 });

    // 2.2 Warm GET
    log('  Executing Warm GET on the same key...');
    const beforeWarm = await readBackend('before warm GET');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-interactive-console] button')).find(b => b.innerText.includes('Send GET'));
      if (btn) btn.click();
    })()`);
    await sleep(1000);

    const warmText = await b.eval("document.querySelector('.console-result')?.innerText || ''");
    log(`  Warm GET response: "${warmText.replace(/\\n/g, ' | ')}"`);
    if (!warmText.includes('HIT')) fail(`Warm GET did not report HIT: "${warmText}"`);
    expectDelta('warm GET', delta(await readBackend('after warm GET'), beforeWarm), { requests: 1, hits: 1, misses: 0, origin: 0 });

    // 2.3 Concurrency Stampede (6x)
    log('  Executing Concurrency Stampede (6x)...');
    const beforeStampede = await readBackend('before stampede');

    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-interactive-console] button')).find(b => b.innerText.includes('Stampede (6x)'));
      if (btn) btn.click();
    })()`);
    await sleep(2500);

    const stampedeDelta = delta(await readBackend('after stampede'), beforeStampede);

    const stampedeText = await b.eval("document.querySelector('.console-result')?.innerText || ''");
    log(`  Stampede response: "${stampedeText.replace(/\\n/g, ' | ')}"`);
    if (!stampedeText.includes('STAMPEDE') || !stampedeText.includes('origin fetches started: 1')) {
      fail(`Stampede coalescing verification failed: "${stampedeText}"`);
    }

    // Independent of the UI text: 6 requests on a unique cold key = 1 miss, 1 origin fetch, and the other 5
    // requests are served as cache hits or coalesced waiters.
    expectDelta('stampede (6x)', stampedeDelta, { requests: 6, misses: 1, origin: 1 });
    if (stampedeDelta.hits + stampedeDelta.coalesced !== 5) {
      fail(`Backend cross-check "stampede (6x)": hits+coalesced expected 5, observed ${stampedeDelta.hits + stampedeDelta.coalesced} (${JSON.stringify(stampedeDelta)})`);
    }

    // 2.4 Purge (DELETE)
    log('  Executing Purge (DELETE)...');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-interactive-console] button')).find(b => b.innerText.includes('Purge (DELETE)'));
      if (btn) btn.click();
    })()`);
    await sleep(1500);

    const deleteText = await b.eval("document.querySelector('.console-result')?.innerText || ''");
    log(`  DELETE response: "${deleteText.replace(/\\n/g, ' | ')}"`);
    if (!deleteText.includes('Delete acknowledged (HTTP 204)')) {
      fail(`DELETE acknowledgement mismatch: "${deleteText}"`);
    }

    // 2.5 Refetch after DELETE (Must be MISS)
    log('  Executing Refetch after DELETE...');
    const beforeRefetch = await readBackend('before refetch');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-interactive-console] button')).find(b => b.innerText.includes('Send GET'));
      if (btn) btn.click();
    })()`);
    await sleep(1500);

    const refetchText = await b.eval("document.querySelector('.console-result')?.innerText || ''");
    log(`  Refetch response: "${refetchText.replace(/\\n/g, ' | ')}"`);
    if (!refetchText.includes('MISS')) {
      fail(`Refetch after DELETE was not a MISS: "${refetchText}"`);
    }
    expectDelta('refetch after DELETE', delta(await readBackend('after refetch'), beforeRefetch), { requests: 1, misses: 1, origin: 1 });
    await capture('Cache Operations (GET, HIT, Stampede, DELETE)', '02_cache_operations.png');

    // -------------------------------------------------------------------------
    // 3. Traffic Generator Verification
    // -------------------------------------------------------------------------
    log('Step 3: Navigating to Traffic & Latency (hash: #/traffic)...');
    await b.eval("window.location.hash = '#/traffic'");
    await sleep(1500);

    const hasGenerator = await b.eval("Boolean(document.querySelector('[data-traffic-generator]'))");
    if (!hasGenerator) fail('Traffic Generator not rendered on #/traffic screen');
    log('  Traffic Generator rendered.');

    // 3.1 10-Request Burst (bounded concurrency)
    log('  Firing 10-Request Burst...');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-traffic-generator] button')).find(b => b.innerText.includes('Burst'));
      if (btn) btn.click();
      else throw new Error('Burst button not found');
    })()`);
    await sleep(2000);

    const burstStats = await b.eval(`(() => {
      const items = Array.from(document.querySelectorAll('.generator-stat')).map(s => ({
        label: s.querySelector('.generator-stat__label')?.innerText.trim(),
        value: Number(s.querySelector('.generator-stat__value')?.innerText.trim())
      }));
      return Object.fromEntries(items.map(i => [i.label, i.value]));
    })()`);
    log(`  Burst stats: ${JSON.stringify(burstStats)}`);
    // Under bounded concurrency (MAX_IN_FLIGHT=6), 6 are dispatched and 4 are dropped by backpressure
    const totalAttempted = burstStats.Dispatched + (burstStats['Backpressure Dropped'] || 0);
    if (totalAttempted !== 10) {
      fail(`10-Req burst accounting mismatch (total attempted != 10): ${JSON.stringify(burstStats)}`);
    }
    if (burstStats.Completed !== burstStats.Dispatched) {
      fail(`Completed != Dispatched: ${JSON.stringify(burstStats)}`);
    }

    // 3.2 Sustained Load
    log('  Starting Sustained Load (15 req/s) for 3 seconds...');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-traffic-generator] button')).find(b => b.innerText.includes('Start Sustained Load'));
      if (btn) btn.click();
    })()`);
    await sleep(3000);

    log('  Stopping Sustained Load...');
    await b.eval(`(() => {
      const btn = Array.from(document.querySelectorAll('[data-traffic-generator] button')).find(b => b.innerText.includes('Stop Sustained Load'));
      if (btn) btn.click();
    })()`);
    await sleep(1000);

    const sustainedStats = await b.eval(`(() => {
      const items = Array.from(document.querySelectorAll('.generator-stat')).map(s => ({
        label: s.querySelector('.generator-stat__label')?.innerText.trim(),
        value: Number(s.querySelector('.generator-stat__value')?.innerText.trim())
      }));
      return Object.fromEntries(items.map(i => [i.label, i.value]));
    })()`);
    log(`  Sustained stats: ${JSON.stringify(sustainedStats)}`);
    if (sustainedStats.Dispatched < 20 || sustainedStats.Completed < 20) {
      fail(`Sustained load dispatched insufficient requests: ${JSON.stringify(sustainedStats)}`);
    }
    if (sustainedStats['In Flight'] !== 0) {
      fail(`In Flight did not settle to 0 after stop: ${sustainedStats['In Flight']}`);
    }
    await capture('Traffic Generator Active Stats', '03_traffic_workload.png');

    // -------------------------------------------------------------------------
    // 4. Connection Modal Presets
    // -------------------------------------------------------------------------
    log('Step 4: Opening Connection Details modal...');
    await b.eval(`(() => {
      const btn = document.querySelector('.app-header__status button') || document.querySelector('.app-header__status');
      if (btn) btn.click();
    })()`);
    await sleep(1000);

    const hasPresets = await b.eval("Boolean(document.querySelector('.conn-presets'))");
    log(`  Connection presets visible: ${hasPresets}`);
    if (!hasPresets) fail('Connection modal missing .conn-presets buttons');
    await capture('Connection Presets Modal', '04_connection_presets.png');

    log('================================================================');
    log('DEMONSTRATOR TEST COMPLETED WITH 100% PASSING STRICT ASSERTIONS');
    log('================================================================');
    console.log(JSON.stringify({ success: true, evidence }, null, 2));

  } catch (err) {
    failure = err instanceof DemoFailure ? err.message : `Demonstrator execution error: ${err.message}\n${err.stack}`;
  } finally {
    // Runs on success AND failure: close Chrome, then remove the temporary profile directory.
    const profileDir = b?.profile;
    try { await b?.close(); } catch {}
    if (profileDir && existsSync(profileDir)) {
      let removed = false;
      for (let i = 0; i < 10 && !removed; i++) {
        await sleep(300);
        try {
          rmSync(profileDir, { recursive: true, force: true });
          removed = true;
        } catch {}
      }
      if (!removed) console.error(`[DEMO-TEST] WARNING: could not remove temporary Chrome profile ${profileDir}`);
    }
  }
  if (failure) {
    console.error(`[DEMO-TEST-FAIL] ${failure}`);
    process.exit(1);
  }
}

run().catch((err) => {
  // run() itself failed before the browser started (for example the mandatory preflight).
  console.error(`[DEMO-TEST-FAIL] ${err instanceof DemoFailure ? err.message : err.stack || err}`);
  process.exit(1);
});
