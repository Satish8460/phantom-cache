// scripts/live_browser_walkthrough.mjs
// Automated live browser walkthrough of PhantomCache Control Center via CDP
import { mkdirSync, copyFileSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, pidAlive, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots');
mkdirSync(SHOTS_DIR, { recursive: true });

const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/live_demo_shots');
try { mkdirSync(ARTIFACT_DIR, { recursive: true }); } catch {}

const log = (msg) => console.log(`[WALKTHROUGH] ${msg}`);

async function run() {
  log('Starting Chrome via CDP...');
  const b = new Browser({ evidenceDir: SHOTS_DIR, label: 'walkthrough' });
  await b.launch();
  await b.viewport(1440, 900);

  const defects = [];
  const screens = [];

  const captureScreen = async (screenName, fileName, navScreenId) => {
    log(`Navigating to ${screenName} (${navScreenId ? 'data-screen=' + navScreenId : 'root'})...`);
    if (navScreenId) {
      await b.eval(`(() => {
        const link = document.querySelector('a[data-screen="${navScreenId}"]');
        if (link) {
          link.click();
        } else {
          window.location.hash = '#${navScreenId}';
        }
      })()`);
      await sleep(1200);
    }

    // Capture title & main content summary
    const summary = await b.eval(`(() => {
      const h1 = document.querySelector('h1, h2')?.innerText || '';
      const text = document.querySelector('main')?.innerText || '';
      return {
        heading: h1,
        length: text.length,
        hasError: text.includes('Error') || text.includes('Crash') || text.includes('NaN'),
      };
    })()`);
    log(`[${screenName}] Heading: "${summary.heading}" | Content length: ${summary.length}`);
    if (summary.hasError) {
      defects.push(`Potential error text rendered on screen ${screenName}`);
    }

    await b.shot(fileName, 'shots');
    const fullSrc = join(SHOTS_DIR, 'shots', fileName);
    const fullDst = join(ARTIFACT_DIR, fileName);
    try { copyFileSync(fullSrc, fullDst); } catch {}
    screens.push({ name: screenName, file: fileName, path: fullDst, heading: summary.heading });
  };

  try {
    // 1. Initial Overview load
    await b.goto('http://localhost:3000', 2500);

    const liveText = await b.eval("document.querySelector('.app-header__status')?.innerText || ''");
    log(`Header live status: "${liveText.trim().replace(/\\n/g, ' ')}"`);
    if (!liveText.toLowerCase().includes('live')) {
      defects.push(`Expected Live indicator in header, got: "${liveText}"`);
    }

    await captureScreen('Overview', '01_overview.png', null);
    await captureScreen('Traffic & Latency', '02_traffic_latency.png', 'traffic');
    await captureScreen('Cache & Memory', '03_cache_memory.png', 'cache');
    await captureScreen('Freshness Guard', '04_freshness.png', 'freshness');
    await captureScreen('Origin Concurrency', '05_origin.png', 'origin');
    await captureScreen('Phantom Lab', '06_phantom_lab.png', 'lab');
    await captureScreen('Cluster Mode', '07_cluster.png', 'cluster');
    await captureScreen('Workload Intelligence', '08_intelligence.png', 'intelligence');

    // 9. Click Connection Status Chip in Header to open Gateway Modal
    log('Clicking Connection Status Chip in Header...');
    await b.eval(`(() => {
      const btn = document.querySelector('.app-header__status button') || document.querySelector('.app-header__status');
      if (btn) btn.click();
    })()`);
    await sleep(1000);

    const modalOpen = await b.eval("Boolean(document.querySelector('.dialog, [role=\"dialog\"], .modal, .connection-control'))");
    log(`Connection Modal open state: ${modalOpen}`);
    if (!modalOpen) defects.push('Connection status chip click did not render connection modal');

    await b.shot('09_connection_modal.png', 'shots');
    try { copyFileSync(join(SHOTS_DIR, 'shots', '09_connection_modal.png'), join(ARTIFACT_DIR, '09_connection_modal.png')); } catch {}
    screens.push({ name: 'Connection Modal', file: '09_connection_modal.png', path: join(ARTIFACT_DIR, '09_connection_modal.png') });

    // Close modal via Escape
    await b.key('Escape');
    await sleep(500);

    // 10. Audit Console Logs & Errors
    const errors = b.consoleLog.filter((c) => c.kind === 'exception' || c.kind === 'console.error');
    log(`Console errors count: ${errors.length}`);
    for (const e of errors) {
      log(`[CONSOLE ERROR] ${e.text}`);
      defects.push(`Console error: ${e.text}`);
    }

    // 11. Audit Failed Network Requests
    const failedNet = Array.from(b.reqById.values()).filter((r) => r.failed || (r.status && r.status >= 500));
    log(`Failed network requests count: ${failedNet.length}`);
    for (const n of failedNet) {
      log(`[NET FAILURE] ${n.method} ${n.url} -> ${n.status || n.failed}`);
      defects.push(`Network failure: ${n.method} ${n.url} (${n.status || n.failed})`);
    }

    log(`Walkthrough successfully finished. Total screens captured: ${screens.length}. Total defects: ${defects.length}`);
    console.log(JSON.stringify({ screens, defects }, null, 2));

  } finally {
    if (b.proc && pidAlive(b.pid)) {
      try { process.kill(b.pid); } catch {}
    }
  }
}

run().catch((e) => {
  console.error('Fatal walkthrough error:', e);
  process.exit(1);
});
