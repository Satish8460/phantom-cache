// scripts/verify_compact_and_analytics.mjs
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Chrome CDP verification...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'analytics_deck' });
  await b.launch();
  await b.viewport(1920, 1080);

  console.log('Navigating to http://127.0.0.1:3201/lab ...');
  await b.goto('http://127.0.0.1:3201/lab', 2000);
  await b.waitFor('document.querySelectorAll(".sc-card").length > 10', 5000);

  // 1. Initial State Shot
  await b.shot('79_compact_nodes_and_analytics_deck');
  console.log('Shot 1 taken.');

  // 2. Trigger Sieve Flood Scan
  console.log('Triggering Sieve Flood 1-Hit Wonders...');
  await b.eval('document.getElementById("btn-sieve-scan").click()');
  await sleep(1400);
  await b.shot('80_sieve_scan_pollution_defense');
  console.log('Shot 2 taken.');

  // 3. Switch Tabs to Adaptive TTL & RAM Slabs
  console.log('Switching tabs to Freshness & RAM Slabs...');
  await b.eval(`
    document.getElementById('tab-btn-freshness').click();
    document.getElementById('tab-btn-ram').click();
  `);
  await sleep(600);
  await b.shot('81_freshness_adaptive_ttl_and_ram_slabs');
  console.log('Shot 3 taken.');

  // 4. Test HRW Key Router with custom key
  console.log('Evaluating custom key in HRW Key Router...');
  await b.eval(`
    document.getElementById('tab-btn-hrw').click();
    document.getElementById('hrw-test-key-input').value = '/store/custom-gaming-monitor-4k';
    document.getElementById('btn-eval-hrw').click();
  `);
  await sleep(600);
  await b.shot('82_hrw_custom_key_evaluated');
  console.log('Shot 4 taken.');

  // 5. Scroll down to show Analytical Deck clearly
  console.log('Scrolling down to capture analytical deck in detail...');
  await b.eval('window.scrollTo(0, 520)');
  await sleep(600);
  await b.shot('83_analytical_deck_scrolled_view');
  console.log('Shot 5 taken.');

  // 6. Focus specifically on Throughput & Latency Graphs
  console.log('Capturing dedicated shot of Throughput & Latency graphs...');
  await b.eval('window.scrollTo(0, 620)');
  await sleep(600);
  await b.shot('85_throughput_and_latency_graphs');
  console.log('Shot 6 taken.');

  await b.close();

  // Copy all shots to artifacts
  ['79_compact_nodes_and_analytics_deck', '80_sieve_scan_pollution_defense', '81_freshness_adaptive_ttl_and_ram_slabs', '82_hrw_custom_key_evaluated', '83_analytical_deck_scrolled_view', '85_throughput_and_latency_graphs'].forEach(name => {
    try {
      copyFileSync(`${SHOTS_DIR}/${name}`, `${ARTIFACT_DIR}/${name}.png`);
      console.log(`Copied ${name} to artifacts.`);
    } catch (e) {
      console.error(`Failed to copy ${name}:`, e.message);
    }
  });

  console.log('All verification tasks complete!');
}

run().catch(console.error);
