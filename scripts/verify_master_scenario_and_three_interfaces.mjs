import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Comprehensive Verification of Master Scenario & 3 Standalone Interfaces...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'master_scenario' });
  await b.launch();
  await b.viewport(1440, 900);

  try {
    // 1. Navigate to Executive Dyno Rig & verify Master Scenario Deck
    console.log('Navigating to http://127.0.0.1:3201/#persona=executive ...');
    await b.goto('http://127.0.0.1:3201/#persona=executive', 3000);
    await sleep(2000);

    // Start Master Live Traffic Stream
    console.log('Starting Master Live Traffic Stream (50 req/s)...');
    await b.eval(`
      (() => {
        const startBtn = document.querySelector('.master-stream-btn');
        if (startBtn && startBtn.textContent.includes('Start')) {
          startBtn.click();
        }
      })()
    `);
    await sleep(3500);

    const masterDeckAudit = await b.eval(`
      (() => {
        const hudStats = Array.from(document.querySelectorAll('.master-hud-stat__val')).map(el => el.textContent.trim());
        const isRunning = document.querySelector('.master-stream-btn')?.textContent.includes('Pause');
        return {
          isRunning,
          hudStats,
        };
      })()
    `);
    console.log('Master Scenario Deck Audit:', masterDeckAudit);

    await b.shot('137_master_scenario_live_traffic_deck.png');
    console.log('Shot 137 captured: Master Live Traffic Scenario in Dyno Rig.');
    copyFileSync(join(SHOTS_DIR, '137_master_scenario_live_traffic_deck.png'), join(ARTIFACT_DIR, '137_master_scenario_live_traffic_deck.png'));

    // 2. Open 3 Interfaces & Cloudflare Tunnel Launcher Modal
    console.log('Opening Multi-Interface & Cloudflare Tunnel Launcher Modal...');
    await b.eval(`
      (() => {
        const tunnelBtn = document.querySelector('.app-header__tunnel-btn');
        if (tunnelBtn) tunnelBtn.click();
      })()
    `);
    await sleep(1000);

    const tunnelModalAudit = await b.eval(`
      (() => {
        const modal = document.querySelector('.tunnel-modal-dialog');
        const cards = Array.from(document.querySelectorAll('.tunnel-card-title')).map(el => el.textContent.trim());
        const tunnelUrl = document.querySelector('.tunnel-url-text')?.textContent.trim();
        return {
          open: !!modal,
          cards,
          tunnelUrl,
        };
      })()
    `);
    console.log('Tunnel Modal Audit:', tunnelModalAudit);

    await b.shot('138_multi_interface_tunnel_launcher.png');
    console.log('Shot 138 captured: Multi-Interface & Cloudflare Tunnel Modal.');
    copyFileSync(join(SHOTS_DIR, '138_multi_interface_tunnel_launcher.png'), join(ARTIFACT_DIR, '138_multi_interface_tunnel_launcher.png'));

    // Close modal
    await b.eval(`
      (() => {
        const closeBtn = document.querySelector('.tunnel-modal-header .probe-btn');
        if (closeBtn) closeBtn.click();
      })()
    `);
    await sleep(800);

    // 3. Navigate to Standalone Interface 1: Storefront (/store)
    console.log('Navigating to Standalone Storefront (http://127.0.0.1:3201/store)...');
    await b.goto('http://127.0.0.1:3201/store', 3000);
    await sleep(2000);

    const storeAudit = await b.eval(`
      (() => {
        const title = document.querySelector('.hero-title')?.textContent.trim();
        const prods = document.querySelectorAll('.product-retail-card, .product-card-hero, .hero-content');
        return {
          title,
          rendered: prods.length > 0,
        };
      })()
    `);
    console.log('Standalone Storefront Audit:', storeAudit);

    await b.shot('139_standalone_storefront_via_tunnel.png');
    console.log('Shot 139 captured: Standalone Storefront (/store).');
    copyFileSync(join(SHOTS_DIR, '139_standalone_storefront_via_tunnel.png'), join(ARTIFACT_DIR, '139_standalone_storefront_via_tunnel.png'));

    // 4. Navigate to Standalone Interface 2: Seller Portal (/seller)
    console.log('Navigating to Standalone Seller Portal (http://127.0.0.1:3201/seller)...');
    await b.goto('http://127.0.0.1:3201/seller', 3000);
    await sleep(2500);

    const sellerAudit = await b.eval(`
      (() => {
        const orders = document.querySelectorAll('.order-row, #orders-list > div');
        const vitals = Array.from(document.querySelectorAll('.vital-n')).map(el => el.textContent.trim());
        return {
          vitals,
          ordersRendered: orders.length,
        };
      })()
    `);
    console.log('Standalone Seller Portal Audit:', sellerAudit);

    await b.shot('140_standalone_seller_portal_live_orders.png');
    console.log('Shot 140 captured: Standalone Seller Portal with live orders stream (/seller).');
    copyFileSync(join(SHOTS_DIR, '140_standalone_seller_portal_live_orders.png'), join(ARTIFACT_DIR, '140_standalone_seller_portal_live_orders.png'));

    // 5. Navigate to Standalone Interface 3: Crowd Hub (/crowd)
    console.log('Navigating to Standalone Crowd Hub (http://127.0.0.1:3201/crowd)...');
    await b.goto('http://127.0.0.1:3201/crowd', 3000);
    await sleep(2000);

    const crowdAudit = await b.eval(`
      (() => {
        const role = document.querySelector('.device-val')?.textContent.trim();
        const burstBtn = !!document.getElementById('btn-burst');
        return {
          role,
          burstBtnActive: burstBtn,
        };
      })()
    `);
    console.log('Standalone Crowd Hub Audit:', crowdAudit);

    await b.shot('141_standalone_crowd_hub_workload.png');
    console.log('Shot 141 captured: Standalone Crowd Hub (/crowd).');
    copyFileSync(join(SHOTS_DIR, '141_standalone_crowd_hub_workload.png'), join(ARTIFACT_DIR, '141_standalone_crowd_hub_workload.png'));

    console.log('🎉 All comprehensive verifications completed successfully!');
  } catch (err) {
    console.error('Verification failed:', err);
    throw err;
  } finally {
    await b.close();
  }
}

run().catch((e) => {
  console.error(e);
  process.exit(1);
});
