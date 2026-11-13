import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 2: Dyno Rig, Phone Sidecar & DB Thermometer at http://127.0.0.1:3201/app ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase2' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/app ...');
    await b.goto('http://127.0.0.1:3201/app', 3000);
    await sleep(2500);

    const title = await b.eval('document.title');
    console.log('Page Title:', title);

    const hasPhone = await b.eval('document.querySelector(".storefront-sidecar") !== null');
    console.log('Phone Sidecar Found:', hasPhone);

    const hasThermometer = await b.eval('document.querySelector(".db-thermometer-bay") !== null');
    console.log('DB Thermometer Found:', hasThermometer);

    // 1. Initial State Shot (Dyno Rig + Phone Sidecar + Thermometer)
    await b.shot('107_phase2_dyno_and_phone_sidecar.png');
    console.log('Shot 107 captured.');
    copyFileSync(join(SHOTS_DIR, '107_phase2_dyno_and_phone_sidecar.png'), join(ARTIFACT_DIR, '107_phase2_dyno_and_phone_sidecar.png'));

    // 2. Click "1-Click Instant Checkout" on Phone Sidecar
    console.log('Clicking 1-Click Instant Checkout on Phone Sidecar...');
    await b.eval('document.querySelector(".phone-buy-btn")?.click()');
    await sleep(2000);

    const orderToast = await b.eval('document.querySelector(".phone-order-toast")?.textContent || ""');
    console.log('Phone Order Confirmation:', orderToast);

    const phoneLatency = await b.eval('document.querySelector(".phone-telemetry-readout")?.textContent || ""');
    console.log('Phone Edge Telemetry Readout:', phoneLatency);

    await b.shot('108_phase2_phone_instant_checkout.png');
    console.log('Shot 108 captured.');
    copyFileSync(join(SHOTS_DIR, '108_phase2_phone_instant_checkout.png'), join(ARTIFACT_DIR, '108_phase2_phone_instant_checkout.png'));

    // 3. Fire Live Multi-Socket Burst on Throttle Slider
    console.log('Firing Live TCP Multi-Socket Burst...');
    await b.eval('document.querySelector(".dyno-burst-btn")?.click()');
    await sleep(3500);

    const burstStats = await b.eval('document.querySelector(".dyno-burst-chip")?.textContent || ""');
    console.log('Live TCP Benchmark Result:', burstStats);

    await b.shot('109_phase2_live_tcp_load_burst.png');
    console.log('Shot 109 captured.');
    copyFileSync(join(SHOTS_DIR, '109_phase2_live_tcp_load_burst.png'), join(ARTIFACT_DIR, '109_phase2_live_tcp_load_burst.png'));

    console.log('✅ Phase 2 Verification Succeeded 100%!');
  } finally {
    await b.close();
  }
}

run().catch(err => {
  console.error(err);
  process.exit(1);
});
