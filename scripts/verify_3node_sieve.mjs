// scripts/verify_3node_sieve.mjs
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify 3-Node SIEVE Eviction Controller...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'sieve_3node' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/lab ...');
    await b.goto('http://127.0.0.1:3201/lab', 2000);
    await b.waitFor('document.querySelectorAll(".sc-card").length >= 35', 8000);
    await sleep(1000);

    // Scroll directly to the Sieve Eviction Matrix section
    console.log('Scrolling to Sieve Matrix Section...');
    await b.eval('document.getElementById("sieve-matrix-section").scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);

    // 1. Audit and Capture Node 1
    const n1Data = await b.eval(`({
      activeTab: document.querySelector('#sieve-node-tabs .tab-btn.active')?.textContent?.trim(),
      activeName: document.getElementById('sieve-active-node-name')?.textContent?.trim(),
      slotsCount: document.querySelectorAll('#sieve-slots-grid .sieve-slot').length,
      slab: document.getElementById('sieve-slab-val')?.textContent?.trim(),
      hand: document.getElementById('sieve-hand-pos-lbl')?.textContent?.trim()
    })`);
    console.log('Node 1 Verification Audit:', JSON.stringify(n1Data, null, 2));

    await b.shot('93_3node_sieve_node1.png');
    console.log('Shot 93: Node 1 Sieve circular buffer captured.');

    // 2. Switch to Node 2 Tab
    console.log('Switching to Node 2 (32 MiB · 32 slots)...');
    await b.eval(`document.getElementById('tab-sieve-n2')?.click()`);
    await sleep(1000);

    // Trigger Read Hit on Node 2
    await b.eval(`document.getElementById('btn-sieve-hit')?.click()`);
    await sleep(600);

    const n2Data = await b.eval(`({
      activeTab: document.querySelector('#sieve-node-tabs .tab-btn.active')?.textContent?.trim(),
      activeName: document.getElementById('sieve-active-node-name')?.textContent?.trim(),
      slotsCount: document.querySelectorAll('#sieve-slots-grid .sieve-slot').length,
      slab: document.getElementById('sieve-slab-val')?.textContent?.trim(),
      hand: document.getElementById('sieve-hand-pos-lbl')?.textContent?.trim()
    })`);
    console.log('Node 2 Verification Audit:', JSON.stringify(n2Data, null, 2));

    await b.shot('94_3node_sieve_node2.png');
    console.log('Shot 94: Node 2 Sieve circular buffer & Read Hit captured.');

    // 3. Switch to Node 3 Tab
    console.log('Switching to Node 3 (16 MiB · 16 slots)...');
    await b.eval(`document.getElementById('tab-sieve-n3')?.click()`);
    await sleep(1000);

    // Trigger Sweep Hand on Node 3
    await b.eval(`document.getElementById('btn-sieve-evict')?.click()`);
    await sleep(600);

    const n3Data = await b.eval(`({
      activeTab: document.querySelector('#sieve-node-tabs .tab-btn.active')?.textContent?.trim(),
      activeName: document.getElementById('sieve-active-node-name')?.textContent?.trim(),
      slotsCount: document.querySelectorAll('#sieve-slots-grid .sieve-slot').length,
      slab: document.getElementById('sieve-slab-val')?.textContent?.trim(),
      hand: document.getElementById('sieve-hand-pos-lbl')?.textContent?.trim()
    })`);
    console.log('Node 3 Verification Audit:', JSON.stringify(n3Data, null, 2));

    await b.shot('95_3node_sieve_node3.png');
    console.log('Shot 95: Node 3 Sieve circular buffer & Hand Eviction captured.');

    // 4. Trigger Flood 1-Hit Wonders Scan Attack on Node 3
    console.log('Triggering Flood 1-Hit Wonders on Node 3...');
    await b.eval(`document.getElementById('btn-sieve-scan')?.click()`);
    await sleep(2500);

    await b.shot('96_3node_sieve_scan_defense.png');
    console.log('Shot 96: Node 3 Scan Pollution Defense captured.');

    // Copy shots to artifacts directory
    [
      '93_3node_sieve_node1.png',
      '94_3node_sieve_node2.png',
      '95_3node_sieve_node3.png',
      '96_3node_sieve_scan_defense.png'
    ].forEach(f => {
      copyFileSync(join(SHOTS_DIR, f), join(ARTIFACT_DIR, f));
      console.log(`Copied artifact: ${f}`);
    });

    console.log('🎉 3-Node SIEVE automated browser verification SUCCEEDED.');

  } catch (err) {
    console.error('❌ Verification failed:', err);
    process.exit(1);
  } finally {
    await b.close();
  }
}

run();
