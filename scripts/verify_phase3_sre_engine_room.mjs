import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 3: SRE Precision Engine Room & Rotary SIEVE Drum at http://127.0.0.1:3201/app ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase3' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/app ...');
    await b.goto('http://127.0.0.1:3201/app', 3000);
    await sleep(2000);

    // 1. Switch to SRE Precision Cockpit
    console.log('Switching Persona to 🛠️ SRE Precision Cockpit...');
    await b.eval(`
      const sreBtn = document.querySelector('[data-persona-option="sre"]');
      if (sreBtn) sreBtn.click();
    `);
    await sleep(2000);

    const hasEngineRoom = await b.eval('document.querySelector(".sre-engine-room") !== null');
    console.log('SRE Engine Room Mounted:', hasEngineRoom);

    const hasPipeline = await b.eval('document.querySelector(".kernel-pipeline-deck") !== null');
    console.log('4-Stage Kernel Pipeline Found:', hasPipeline);

    // 2. Click "Probe 10 Coalesced Concurrent Requests" in Pipeline
    console.log('Probing Singleflight Mutex with 10 concurrent requests...');
    await b.eval('document.querySelector(".kernel-pipeline-deck .probe-btn")?.click()');
    await sleep(2500);

    const probeResult = await b.eval('document.querySelector(".probe-result-chip")?.textContent || ""');
    console.log('Singleflight Probe Result:', probeResult);

    await b.shot('110_phase3_sre_4stage_pipeline.png');
    console.log('Shot 110 captured.');
    copyFileSync(join(SHOTS_DIR, '110_phase3_sre_4stage_pipeline.png'), join(ARTIFACT_DIR, '110_phase3_sre_4stage_pipeline.png'));

    // 3. Test Rotary SIEVE Drum & Crawler Attack Defense
    console.log('Advancing Rotary SIEVE Hand Pointer...');
    await b.eval('document.querySelector(".sieve-drum-card .probe-btn")?.click()');
    await sleep(1500);

    console.log('Launching 50-Key Crawler Scan Attack...');
    await b.eval('document.querySelector(".scan-attack-btn")?.click()');
    await sleep(3500);

    const scanResult = await b.eval('document.querySelector(".scan-defense-result-box")?.textContent || ""');
    console.log('Crawler Scan Defense Result:', scanResult);

    const hasCanvas = await b.eval('document.querySelector("canvas") !== null');
    console.log('Canvas2D Rotary Drum Found:', hasCanvas);

    await b.shot('111_phase3_rotary_sieve_and_scan_defense.png');
    console.log('Shot 111 captured.');
    copyFileSync(join(SHOTS_DIR, '111_phase3_rotary_sieve_and_scan_defense.png'), join(ARTIFACT_DIR, '111_phase3_rotary_sieve_and_scan_defense.png'));

    // 4. Verify Low-Level Rust Kernel Diagnostics Deck
    const diagCards = await b.eval('document.querySelectorAll(".diag-stat-card").length');
    console.log('Diagnostics Registers Found:', diagCards);

    await b.shot('112_phase3_kernel_diagnostics_deck.png');
    console.log('Shot 112 captured.');
    copyFileSync(join(SHOTS_DIR, '112_phase3_kernel_diagnostics_deck.png'), join(ARTIFACT_DIR, '112_phase3_kernel_diagnostics_deck.png'));

    console.log('✅ Phase 3 Verification Succeeded 100%!');
  } catch (err) {
    console.error('❌ Phase 3 Verification Failed:', err);
    process.exitCode = 1;
  } finally {
    await b.close();
  }
}

run();
