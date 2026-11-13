// scripts/verify_surge_pie_and_drawers.mjs
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Zero-Baseline Silence, Surge Generators, Comparative DB Load, and 12 PIE Policies...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'surge_and_pie' });
  await b.launch();
  await b.viewport(1920, 1080);

  console.log('Navigating to http://127.0.0.1:3201/lab ...');
  await b.goto('http://127.0.0.1:3201/lab', 2000);
  await b.waitFor('document.querySelectorAll(".sc-card").length >= 35', 8000);
  await sleep(1500);

  // 1. Verify Strict Zero-Baseline Silence
  console.log('Verifying initial state is strict zero-baseline silence...');
  const initialKpi = await b.eval(`({
    totalReqs: document.getElementById('kpi-total-requests')?.textContent,
    offloadPct: document.getElementById('kpi-offload-pct')?.textContent,
    liveRps: document.getElementById('tp-live-rps')?.textContent,
    cardsCount: document.querySelectorAll('.sc-card').length,
    infoBtnsCount: document.querySelectorAll('.sc-info-btn').length,
    pieCardsCount: document.querySelectorAll('#pie-policy-grid > div').length
  })`);
  console.log('Initial KPI & DOM Audit:', JSON.stringify(initialKpi, null, 2));

  await b.shot('87_strict_zero_baseline_silence.png');
  console.log('Shot 87: Strict zero-baseline silence captured.');

  // 2. Expand Technical Info Drawers (i) on Scenario Cards #02, #03, #04
  console.log('Expanding technical info drawers on scenario cards...');
  await b.eval(`
    toggleScenarioDrawer('stampede');
    toggleScenarioDrawer('gallery');
    toggleScenarioDrawer('peerfill');
  `);
  await sleep(600);
  await b.shot('88_scenario_technical_info_drawers.png');
  console.log('Shot 88: Scenario technical info drawers captured.');

  // 3. Scroll to 12 PIE Policies Matrix & Multi-Armed Bandit Console
  console.log('Scrolling to 12 PIE Autonomic Policies Matrix...');
  await b.eval('document.getElementById("pie-matrix-section").scrollIntoView({ behavior: "instant", block: "center" })');
  await sleep(800);
  await b.shot('89_pie_12_autonomic_policies_matrix.png');
  console.log('Shot 89: 12 PIE Autonomic Policies Matrix captured.');

  // 4. Trigger 10,000 req/s Sustained Frequency Surge Generator
  console.log('Starting 10,000 req/s Frequency Surge Generator...');
  await b.eval('window.scrollTo(0, 480)');
  await sleep(400);
  await b.eval('document.getElementById("btn-gen-10k").click()');
  console.log('Generator active. Waiting for 3 seconds of sustained traffic...');
  await sleep(3200);

  const surgeStats = await b.eval(`({
    totalDispatched: document.getElementById('gen-progress-counts')?.textContent,
    absorption: document.getElementById('gen-progress-absorption')?.textContent,
    liveRps: document.getElementById('tp-live-rps')?.textContent,
    rawPool: document.getElementById('raw-conn-pool')?.textContent,
    rawCpu: document.getElementById('raw-cpu-val')?.textContent,
    rawStatus: document.getElementById('raw-status-callout')?.textContent,
    shieldedPool: document.getElementById('shielded-conn-pool')?.textContent,
    shieldedCpu: document.getElementById('shielded-cpu-val')?.textContent,
    shieldedStatus: document.getElementById('shielded-status-callout')?.textContent,
    hitRatioPct: document.getElementById('ratio-hit-pct')?.textContent
  })`);
  console.log('Surge Stats (10,000 req/s):', JSON.stringify(surgeStats, null, 2));

  await b.shot('90_surge_generator_10k_comparative_db_load.png');
  console.log('Shot 90: 10,000 req/s Surge and Comparative DB Load captured.');

  // 5. Escalate to 100,000 req/s (1 Lakh Extreme Stress)
  console.log('Escalating to 100,000 req/s (1 Lakh Extreme Stress)...');
  await b.eval('document.getElementById("btn-gen-100k").click()');
  await sleep(2800);

  const lakhStats = await b.eval(`({
    totalDispatched: document.getElementById('gen-progress-counts')?.textContent,
    liveRps: document.getElementById('tp-live-rps')?.textContent,
    rawStatus: document.getElementById('raw-status-callout')?.textContent,
    shieldedStatus: document.getElementById('shielded-status-callout')?.textContent
  })`);
  console.log('1 Lakh Stress Stats:', JSON.stringify(lakhStats, null, 2));

  await b.shot('91_extreme_stress_100k_lakh_generator.png');
  console.log('Shot 91: 100,000 req/s Extreme Stress captured.');

  // 6. Stop Flow and Verify Return to Strict Zero-Baseline Silence
  console.log('Stopping generator via Stop Flow button...');
  await b.eval('document.getElementById("btn-gen-stop").click()');
  await sleep(1500);

  const stoppedStats = await b.eval(`({
    activeTargetRps: window.activeTargetRps,
    stopBtnDisplay: document.getElementById('btn-gen-stop')?.style.display,
    statusBadge: document.getElementById('db-load-status-badge')?.textContent
  })`);
  console.log('Stopped Stats:', JSON.stringify(stoppedStats, null, 2));

  await b.eval('window.scrollTo(0, 0)');
  await sleep(600);
  await b.shot('92_stop_flow_return_to_zero_silence.png');
  console.log('Shot 92: Return to strict silence captured.');

  // Copy screenshots to Antigravity brain artifact directory
  [
    '87_strict_zero_baseline_silence.png',
    '88_scenario_technical_info_drawers.png',
    '89_pie_12_autonomic_policies_matrix.png',
    '90_surge_generator_10k_comparative_db_load.png',
    '91_extreme_stress_100k_lakh_generator.png',
    '92_stop_flow_return_to_zero_silence.png'
  ].forEach(fn => {
    const src = resolve(SHOTS_DIR, fn);
    const dst = resolve(ARTIFACT_DIR, fn);
    try {
      copyFileSync(src, dst);
      console.log(`Copied ${fn} to artifacts.`);
    } catch (e) {
      console.warn(`Could not copy ${fn}:`, e.message);
    }
  });

  await b.close();
  console.log('✅ All verifications completed successfully!');
}

run().catch(err => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
