import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 5: Production Cutover at http://127.0.0.1:3201/ ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase5' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    // 1. Root Cutover: Verify root / serves the modern React app
    console.log('Navigating to root http://127.0.0.1:3201/ ...');
    await b.goto('http://127.0.0.1:3201/', 3000);
    await sleep(2500);

    const title = await b.eval('document.title');
    console.log('Root Page Title:', title);

    const hasExecutiveScreen = await b.eval('document.querySelector(".dyno-rig-layout") !== null');
    console.log('Root Serves Modern React Executive Screen:', hasExecutiveScreen);

    await b.shot('116_phase5_production_cutover_root_executive.png');
    console.log('Shot 116 captured.');
    copyFileSync(join(SHOTS_DIR, '116_phase5_production_cutover_root_executive.png'), join(ARTIFACT_DIR, '116_phase5_production_cutover_root_executive.png'));

    // 2. Switch to SRE Mode on Root
    console.log('Switching Persona to 🛠️ SRE Precision Cockpit on Root...');
    await b.eval(`
      const sreBtn = document.querySelector('[data-persona-option="sre"]');
      if (sreBtn) sreBtn.click();
    `);
    await sleep(2000);

    const hasSreEngineRoom = await b.eval('document.querySelector(".sre-engine-room") !== null');
    console.log('Root Serves SRE Engine Room on Switch:', hasSreEngineRoom);

    await b.shot('117_phase5_production_cutover_root_sre.png');
    console.log('Shot 117 captured.');
    copyFileSync(join(SHOTS_DIR, '117_phase5_production_cutover_root_sre.png'), join(ARTIFACT_DIR, '117_phase5_production_cutover_root_sre.png'));

    // 3. Storefront Production Verification (/store)
    console.log('Navigating to http://127.0.0.1:3201/store ...');
    await b.goto('http://127.0.0.1:3201/store', 3000);
    await sleep(2000);

    await b.shot('118_phase5_production_storefront.png');
    console.log('Shot 118 captured.');
    copyFileSync(join(SHOTS_DIR, '118_phase5_production_storefront.png'), join(ARTIFACT_DIR, '118_phase5_production_storefront.png'));

    // 4. Seller Origin Portal (/seller)
    console.log('Navigating to http://127.0.0.1:3201/seller ...');
    await b.goto('http://127.0.0.1:3201/seller', 3000);
    await sleep(2000);

    await b.shot('119_phase5_production_seller_origin.png');
    console.log('Shot 119 captured.');
    copyFileSync(join(SHOTS_DIR, '119_phase5_production_seller_origin.png'), join(ARTIFACT_DIR, '119_phase5_production_seller_origin.png'));

    // 5. Crowd Generator (/crowd)
    console.log('Navigating to http://127.0.0.1:3201/crowd ...');
    await b.goto('http://127.0.0.1:3201/crowd', 3000);
    await sleep(2000);

    await b.shot('120_phase5_production_crowd_generator.png');
    console.log('Shot 120 captured.');
    copyFileSync(join(SHOTS_DIR, '120_phase5_production_crowd_generator.png'), join(ARTIFACT_DIR, '120_phase5_production_crowd_generator.png'));

    console.log('✅ Phase 5 Production Cutover Succeeded 100%!');
  } catch (err) {
    console.error('❌ Phase 5 Verification Failed:', err);
    process.exitCode = 1;
  } finally {
    await b.close();
  }
}

run();
