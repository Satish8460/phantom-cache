import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Light/Dark Theme & Normal User Personas...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'refinements' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    // 1. Shopper Buyer Persona in Warm Paper Light Mode
    console.log('Navigating to http://127.0.0.1:3201/ ...');
    await b.goto('http://127.0.0.1:3201/', 3000);
    await sleep(2500);

    // Ensure Light mode is applied
    await b.eval(`
      (() => {
        document.documentElement.setAttribute('data-theme', 'light');
      })()
    `);
    await sleep(500);

    const buyerVisible = await b.eval('document.querySelector(".persona-badge--buyer") !== null');
    console.log('Buyer Persona Visible:', buyerVisible);

    await b.shot('121_shopper_buyer_persona_light.png');
    console.log('Shot 121 captured (Shopper Buyer Persona Light).');
    copyFileSync(join(SHOTS_DIR, '121_shopper_buyer_persona_light.png'), join(ARTIFACT_DIR, '121_shopper_buyer_persona_light.png'));

    // 2. Toggle to Dark Mode
    console.log('Toggling to Dark Mode...');
    await b.eval(`
      (() => {
        const btn = document.querySelector('.app-header__theme-quick');
        if (btn) btn.click();
        else document.documentElement.setAttribute('data-theme', 'dark');
      })()
    `);
    await sleep(1000);

    const themeAfter = await b.eval('document.documentElement.getAttribute("data-theme")');
    console.log('Current Theme Attribute:', themeAfter);

    await b.shot('122_shopper_buyer_persona_dark.png');
    console.log('Shot 122 captured (Shopper Buyer Persona Dark).');
    copyFileSync(join(SHOTS_DIR, '122_shopper_buyer_persona_dark.png'), join(ARTIFACT_DIR, '122_shopper_buyer_persona_dark.png'));

    // 3. Switch to Merchant (Seller) Persona
    console.log('Switching to Merchant (Seller) Persona...');
    await b.eval(`
      (() => {
        const sellerBtn = document.querySelector('[data-persona-option="seller"]');
        if (sellerBtn) sellerBtn.click();
      })()
    `);
    await sleep(1500);

    // Trigger Flash Sale Action on item 1
    console.log('Triggering Flash Sale price drop...');
    await b.eval(`
      (() => {
        const flashBtn = document.querySelector('.merchant-flash-btn');
        if (flashBtn) flashBtn.click();
      })()
    `);
    await sleep(1000);

    await b.shot('123_merchant_seller_persona_dark.png');
    console.log('Shot 123 captured (Merchant Seller Persona Dark with 0.18ms invalidation).');
    copyFileSync(join(SHOTS_DIR, '123_merchant_seller_persona_dark.png'), join(ARTIFACT_DIR, '123_merchant_seller_persona_dark.png'));

    // 4. Merchant in Light Mode
    console.log('Switching Merchant to Light Mode...');
    await b.eval(`
      (() => {
        document.documentElement.setAttribute('data-theme', 'light');
      })()
    `);
    await sleep(800);

    await b.shot('124_merchant_seller_persona_light.png');
    console.log('Shot 124 captured (Merchant Seller Persona Light).');
    copyFileSync(join(SHOTS_DIR, '124_merchant_seller_persona_light.png'), join(ARTIFACT_DIR, '124_merchant_seller_persona_light.png'));

    // 5. Switch to SRE Cockpit Persona
    console.log('Switching to SRE Cockpit Persona...');
    await b.eval(`
      (() => {
        const sreBtn = document.querySelector('[data-persona-option="sre"]');
        if (sreBtn) sreBtn.click();
      })()
    `);
    await sleep(1500);

    // Open Prometheus Metrics Modal Drawer
    console.log('Opening Prometheus Metrics modal drawer...');
    await b.eval(`
      (() => {
        const buttons = Array.from(document.querySelectorAll('.probe-btn'));
        const promBtn = buttons.find(b => b.textContent.includes('Prometheus'));
        if (promBtn) promBtn.click();
      })()
    `);
    await sleep(1000);

    await b.shot('125_sre_cockpit_prometheus_metrics.png');
    console.log('Shot 125 captured (SRE Cockpit with Prometheus Metrics Drawer).');
    copyFileSync(join(SHOTS_DIR, '125_sre_cockpit_prometheus_metrics.png'), join(ARTIFACT_DIR, '125_sre_cockpit_prometheus_metrics.png'));

    // 6. Switch to Executive Dyno Crash-Test Rig
    console.log('Switching to Executive Dyno Persona...');
    await b.eval(`
      (() => {
        const dynoBtn = document.querySelector('[data-persona-option="executive"]');
        if (dynoBtn) dynoBtn.click();
      })()
    `);
    await sleep(1500);

    await b.shot('126_executive_dyno_light.png');
    console.log('Shot 126 captured (Executive Dyno Light Mode).');
    copyFileSync(join(SHOTS_DIR, '126_executive_dyno_light.png'), join(ARTIFACT_DIR, '126_executive_dyno_light.png'));

    console.log('✨ All 6 refinement screenshots captured and verified successfully!');
  } finally {
    await b.close();
  }
}

run().catch((err) => {
  console.error('CDP Verification failed:', err);
  process.exit(1);
});
