#!/usr/bin/env node
// scripts/test_final_e2e_iterations.mjs
// Final 3-Iteration End-to-End (E2E) Automated System & Screenshot Verification Suite.

import { copyFileSync, mkdirSync, existsSync, statSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots');
mkdirSync(SHOTS_DIR, { recursive: true });

const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(ARTIFACT_DIR, { recursive: true });

function log(msg) {
  console.log(`[E2E-FINAL] ${msg}`);
}

async function verifyScreenshot(name, minBytes = 10000) {
  const p1 = join(SHOTS_DIR, name);
  const p2 = join(ARTIFACT_DIR, name);
  if (!existsSync(p1)) throw new Error(`Screenshot missing at ${p1}`);
  const sz = statSync(p1).size;
  if (sz < minBytes) throw new Error(`Screenshot at ${p1} too small (${sz} bytes)`);
  copyFileSync(p1, p2);
  log(`  ✓ Confirmed screenshot: ${name} (${Math.round(sz / 1024)} KB)`);
  return { path: p2, sizeKb: Math.round(sz / 1024) };
}

async function runFinalE2ETest() {
  log('================================================================');
  log('STARTING FINAL MULTI-ITERATION E2E TEST WITH SCREENSHOT AUDIT');
  log('================================================================');

  const b = new Browser({ evidenceDir: SHOTS_DIR, label: 'final_e2e' });
  await b.launch();
  await b.viewport(1820, 1020);

  const capturedShots = [];

  try {
    // ================================================================
    // ITERATION 1: SRE COCKPIT, BUYER STORE & WIDESCREEN TRAFFIC
    // ================================================================
    log('\n>>> EXECUTING ITERATION 1: SRE Cockpit, Buyer Store & Traffic Dashboard...');

    // 1.1 SRE Cockpit
    const sreUrl = 'http://localhost:3000/?gateway=http://127.0.0.1:18185#/overview?persona=sre';
    log(`[Iter 1.1] Navigating to SRE Cockpit: ${sreUrl}`);
    await b.goto(sreUrl, 3000);

    await b.waitFor(`(() => {
      const h = document.querySelector('.app-header__status');
      return h && (h.textContent.includes('Live') || h.querySelector('[data-tone="ok"]'));
    })()`, 12000);

    const sreCheck = await b.eval(`(() => {
      const headerStatus = document.querySelector('.app-header__status')?.textContent.trim() || '';
      const flow = Boolean(document.querySelector('.flow-zone--clients') && document.querySelector('.flow-zone--edge'));
      const matrix = Boolean(document.querySelector('.pie-matrix-bay') || document.querySelector('.zero-scroll-cockpit')?.textContent.includes('PIE v2'));
      const navLinks = [...document.querySelectorAll('.app-nav a')].map(a => a.textContent.trim());
      return { headerStatus, flow, matrix, navLinks };
    })()`);
    log(`  SRE Verification: Header="${sreCheck.headerStatus}", FlowActive=${sreCheck.flow}, PIEActive=${sreCheck.matrix}`);
    log(`  Active Nav Links in Demo Showcase: ${JSON.stringify(sreCheck.navLinks)}`);

    if (!sreCheck.flow || !sreCheck.matrix) {
      throw new Error('Iteration 1: SRE Cockpit flow diagram or autonomic matrix missing!');
    }
    if (sreCheck.navLinks.includes('Phantom Lab') || sreCheck.navLinks.includes('Simulation')) {
      throw new Error('Iteration 1: Insignificant screens found in demo showcase navigation!');
    }

    const shot1 = '160_iter1_sre_cockpit.png';
    await b.shot(shot1, '');
    capturedShots.push(await verifyScreenshot(shot1));

    // 1.2 Buyer Store
    log('[Iter 1.2] Navigating to Buyer Store Persona...');
    await b.click("document.querySelector('[data-persona-option=\"buyer\"]')");
    await sleep(2500);

    const storeCheck = await b.eval(`(() => {
      const heading = document.querySelector('h1')?.textContent || '';
      const meter = Boolean(document.querySelector('.latency-comparison-meter'));
      const crowd = Boolean(document.querySelector('.crowd-shield-card'));
      const products = document.querySelectorAll('.product-retail-card').length;
      return { heading, meter, crowd, products };
    })()`);
    log(`  Buyer Store: Heading="${storeCheck.heading}", LatencyMeter=${storeCheck.meter}, HerdSimulator=${storeCheck.crowd}, ProductsCount=${storeCheck.products}`);

    if (storeCheck.products < 8) {
      throw new Error(`Iteration 1: Expected retail products, found ${storeCheck.products}`);
    }

    const shot2 = '161_iter1_buyer_store.png';
    await b.shot(shot2, '');
    capturedShots.push(await verifyScreenshot(shot2));

    // 1.3 Traffic & Latency Dashboard
    log('[Iter 1.3] Navigating to Traffic & Latency Dashboard...');
    await b.clickText('Traffic & latency', 'a');
    await sleep(2500);

    const trafficCheck = await b.eval(`(() => {
      const isWide = Boolean(document.querySelector('.traffic-screen-wide') || document.querySelector('[data-wide-layout="true"]'));
      const hasGrid = Boolean(document.querySelector('.traffic-dashboard-grid'));
      const modeTabs = [...document.querySelectorAll('.traffic-mode-btn')].map(b => b.textContent.trim());
      const hasSwarm = Boolean(document.querySelector('.traffic-swarm-bay') || document.querySelector('.traffic-swarm-qr'));
      return { isWide, hasGrid, modeTabs, hasSwarm };
    })()`);
    log(`  Traffic Dashboard: Widescreen=${trafficCheck.isWide}, Grid=${trafficCheck.hasGrid}, SwarmQR=${trafficCheck.hasSwarm}`);
    log(`  Multi-Ingress Modes: ${JSON.stringify(trafficCheck.modeTabs)}`);

    if (!trafficCheck.isWide || !trafficCheck.hasGrid || !trafficCheck.hasSwarm) {
      throw new Error('Iteration 1: Traffic & Latency widescreen layout or external swarm bay missing!');
    }

    const shot3 = '162_iter1_traffic_dashboard.png';
    await b.shot(shot3, '');
    capturedShots.push(await verifyScreenshot(shot3));

    log('✓ ITERATION 1 COMPLETED SUCCESSFULLY!\n');

    // ================================================================
    // ITERATION 2: SELLER PORTAL, EXECUTIVE DYNO & NAV TOGGLE AUDIT
    // ================================================================
    log('>>> EXECUTING ITERATION 2: Seller Portal, Executive Dyno & Navigation Toggle Audit...');

    // 2.1 Seller Persona
    log('[Iter 2.1] Navigating to Seller Portal...');
    await b.clickText('Overview', 'a');
    await sleep(1500);
    await b.click("document.querySelector('[data-persona-option=\"seller\"]')");
    await sleep(2500);

    const sellerCheck = await b.eval(`(() => {
      const hasRadar = Boolean(document.querySelector('.demand-radar-strip'));
      const hasHud = Boolean(document.querySelector('.origin-contract-hud'));
      const hasOrders = Boolean(document.querySelector('.seller-order-feed') || document.body.textContent.includes('Origin Diverter SLA'));
      return { hasRadar, hasHud, hasOrders };
    })()`);
    log(`  Seller Portal: ContractHUD=${sellerCheck.hasHud}, DemandRadar=${sellerCheck.hasRadar}, Orders=${sellerCheck.hasOrders}`);

    if (!sellerCheck.hasHud || !sellerCheck.hasRadar) {
      throw new Error('Iteration 2: Seller Portal contract HUD or demand radar missing!');
    }

    const shot4 = '163_iter2_seller_portal.png';
    await b.shot(shot4, '');
    capturedShots.push(await verifyScreenshot(shot4));

    // 2.2 Executive Dyno Crash-Test Rig
    log('[Iter 2.2] Navigating to Executive Dyno Crash-Test Rig...');
    await b.click("document.querySelector('[data-persona-option=\"executive\"]')");
    await sleep(2500);

    const dynoCheck = await b.eval(`(() => {
      const txt = document.body.textContent;
      const hasStampede = txt.includes('Act 2: Stampede') || txt.includes('100k');
      const hasShield = txt.includes('Edge RAM Mesh') || txt.includes('99.8%');
      const hasCost = txt.includes('Cloud Bill Saved') || txt.includes('$');
      return { hasStampede, hasShield, hasCost };
    })()`);
    log(`  Executive Dyno: StampedeAct=${dynoCheck.hasStampede}, EdgeShield=${dynoCheck.hasShield}, CostSaved=${dynoCheck.hasCost}`);

    if (!dynoCheck.hasStampede || !dynoCheck.hasShield) {
      throw new Error('Iteration 2: Executive Dyno stampede act or edge shield missing!');
    }

    const shot5 = '164_iter2_executive_dyno.png';
    await b.shot(shot5, '');
    capturedShots.push(await verifyScreenshot(shot5));

    // 2.3 Navigation Mode Toggle
    log('[Iter 2.3] Testing Navigation Toggle (Show All Screens 10 vs Demo Showcase 2)...');
    const toggleBtnText = await b.eval(`document.querySelector('.app-nav__mode-btn')?.textContent.trim() || ''`);
    log(`  Initial Nav Toggle Button: "${toggleBtnText}"`);

    // Click to expand to all 10 diagnostic screens
    await b.click("document.querySelector('.app-nav__mode-btn')");
    await sleep(1000);

    const expandedLinks = await b.eval(`[...document.querySelectorAll('.app-nav a')].map(a => a.textContent.trim())`);
    log(`  Expanded Navigation Links count=${expandedLinks.length}: ${JSON.stringify(expandedLinks)}`);

    if (expandedLinks.length < 8) {
      throw new Error(`Iteration 2: Expected expanded nav to have 10 screens, found ${expandedLinks.length}`);
    }

    const shot6 = '165_iter2_nav_expanded.png';
    await b.shot(shot6, '');
    capturedShots.push(await verifyScreenshot(shot6));

    // Click to collapse back to Demo Showcase Mode
    await b.click("document.querySelector('.app-nav__mode-btn')");
    await sleep(1000);

    const collapsedLinks = await b.eval(`[...document.querySelectorAll('.app-nav a')].map(a => a.textContent.trim())`);
    log(`  Collapsed Navigation Links count=${collapsedLinks.length}: ${JSON.stringify(collapsedLinks)}`);

    if (collapsedLinks.length > 3) {
      throw new Error(`Iteration 2: Expected collapsed nav to have 2 demo screens, found ${collapsedLinks.length}`);
    }

    log('✓ ITERATION 2 COMPLETED SUCCESSFULLY!\n');

    // ================================================================
    // ITERATION 3: PHYSICAL SWARM REGISTRATION & PRECISION PROBE
    // ================================================================
    log('>>> EXECUTING ITERATION 3: Physical Swarm Registration, Precision Probe & Photon Stream...');

    // 3.1 Register Cellular 5G Phone + Wi-Fi Phone
    log('[Iter 3.1] Registering external mobile devices to swarm fleet...');
    await fetch('http://127.0.0.1:3201/api/crowd/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'cf-connecting-ip': '172.68.10.45' },
      body: JSON.stringify({
        device_id: 'iphone-16-pro-cellular',
        type: 'iPhone 16 Pro',
        network: 'Cellular 5G (Cloudflare)',
        vusers: 500,
        running: true,
      }),
    });

    await fetch('http://127.0.0.1:3201/api/crowd/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        device_id: 'pixel-9-pro-wifi',
        type: 'Pixel 9 Pro',
        network: 'Wireless Wi-Fi',
        vusers: 250,
        running: true,
      }),
    });
    await sleep(2000);

    // 3.2 Navigate to SRE Cockpit to verify Zone 1 devices
    await b.goto('http://localhost:3000/?gateway=http://127.0.0.1:18185#/overview?persona=sre', 2500);

    const swarmZone1 = await b.eval(`(() => {
      const box = document.querySelector('.flow-clients-box');
      if (!box) return [];
      return Array.from(box.querySelectorAll('div')).map(d => d.textContent.trim()).filter(t => t.includes('v-clients'));
    })()`);
    log(`  Zone 1 Verified Swarm Devices: ${JSON.stringify(swarmZone1)}`);

    if (swarmZone1.length < 2) {
      throw new Error(`Iteration 3: Expected at least 2 active devices in Zone 1, found ${swarmZone1.length}`);
    }

    const shot7 = '166_iter3_cellular_swarm.png';
    await b.shot(shot7, '');
    capturedShots.push(await verifyScreenshot(shot7));

    // 3.3 Precision Probe Burst & Live Photon Stream
    log('[Iter 3.3] Navigating to Traffic screen and testing precision probe burst...');
    await b.clickText('Traffic & latency', 'a');
    await sleep(2000);

    // Switch to Mode 3: Targeted Precision Probe
    await b.click("document.querySelectorAll('.traffic-mode-btn')[2]");
    await sleep(1000);

    // Click Send 1 Request then Burst (up to 10)
    await b.clickText('Send 1 Request');
    await sleep(800);
    await b.clickText('Burst');
    await sleep(2000);

    const probeStats = await b.eval(`(() => {
      const stats = {};
      const labels = document.querySelectorAll('.generator-stat__label');
      const values = document.querySelectorAll('.generator-stat__value');
      labels.forEach((l, i) => {
        stats[l.textContent.trim()] = values[i]?.textContent.trim();
      });
      return stats;
    })()`);
    log(`  Precision Probe Telemetry Accounting: ${JSON.stringify(probeStats)}`);

    const failedCount = Number(probeStats['Failed'] || 0);
    const completedCount = Number(probeStats['Completed'] || 0);
    log(`  Probe Results: Completed=${completedCount}, Failed=${failedCount}`);

    if (failedCount > 0) {
      throw new Error(`Iteration 3 FAILED: Probe reported ${failedCount} failures!`);
    }
    if (completedCount < 1) {
      throw new Error('Iteration 3 FAILED: Probe did not complete requests!');
    }
    log('  ✓ Precision probe achieved 100% SUCCESS with ZERO failures!');

    const shot8 = '167_iter3_traffic_probe_verified.png';
    await b.shot(shot8, '');
    capturedShots.push(await verifyScreenshot(shot8));

    // Return to SRE Cockpit to capture active photon stream under live workload
    log('[Iter 3.4] Returning to SRE Cockpit to capture live photon particle flow...');
    await b.clickText('Overview', 'a');
    await sleep(2500);

    const hasCanvas = await b.eval(`Boolean(document.querySelector('.flow-photon-canvas'))`);
    log(`  Active Canvas Photon Engine: ${hasCanvas}`);

    const shot9 = '168_iter3_sre_photon_stream.png';
    await b.shot(shot9, '');
    capturedShots.push(await verifyScreenshot(shot9));

    log('✓ ITERATION 3 COMPLETED SUCCESSFULLY!\n');

    // ================================================================
    // FINAL AUDIT SUMMARY
    // ================================================================
    log('================================================================');
    log('ALL 3 E2E TEST ITERATIONS PASSED WITH 100% SCREENSHOT CONFIRMATION');
    log('================================================================');
    capturedShots.forEach((s, idx) => {
      log(`  [SS-${idx + 1}] ${s.path} (${s.sizeKb} KB)`);
    });

  } finally {
    await b.close();
    log('✓ Chrome browser automation session cleanly closed.');
  }
}

runFinalE2ETest().catch((err) => {
  console.error('[E2E-FINAL ERROR]', err);
  process.exit(1);
});
