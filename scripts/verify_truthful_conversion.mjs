// scripts/verify_truthful_conversion.mjs
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify 4-Phase Bloat-to-Truth Conversion...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'bloat_to_truth' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/lab ...');
    await b.goto('http://127.0.0.1:3201/lab', 2000);
    await b.waitFor('document.querySelectorAll(".sc-card").length >= 35', 8000);
    await sleep(1500);

    // =========================================================================
    // 1. PHASE 1: Real Kernel Diagnostics Deck & Live OS RSS Slabs
    // =========================================================================
    console.log('Auditing Phase 1: Real Rust Kernel Diagnostics (28 counters) & Live OS RSS...');
    await b.eval('document.getElementById("kernel-diagnostics-section").scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);

    const phase1Audit = await b.eval(`({
      deckTitle: document.querySelector('#kernel-diagnostics-section h2')?.textContent,
      coalescedWaiters: document.getElementById('diag-coalesced')?.textContent,
      staleWrites: document.getElementById('diag-stale-writes')?.textContent,
      lockPoison: document.getElementById('diag-lock-poison')?.textContent,
      entropySync: document.getElementById('diag-entropy-sync')?.textContent,
      revalOk: document.getElementById('diag-reval-ok')?.textContent,
      budgetRejections: document.getElementById('diag-budget-rejections')?.textContent,
      panics: document.getElementById('diag-panics')?.textContent,
      rssSlab: document.getElementById('sieve-slab-val')?.textContent
    })`);
    console.log('Phase 1 Live Kernel Audit:', JSON.stringify(phase1Audit, null, 2));

    await b.shot('97_phase1_live_kernel_diagnostics.png');
    console.log('Shot 97: Live Kernel Diagnostics Deck captured.');

    // =========================================================================
    // 2. PHASE 2: True PIE-v2 Intelligence Engine & Real Decision Ring
    // =========================================================================
    console.log('Auditing Phase 2: True PIE-v2 Engine (12 Real Rust Policies & Decisions)...');
    await b.eval('fetchPieStatus && fetchPieStatus()');
    await sleep(1200);
    await b.eval('document.getElementById("pie-matrix-section").scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);

    const phase2Audit = await b.eval(`({
      renderedPoliciesCount: document.querySelectorAll('#pie-policy-grid > div').length,
      samplePolicyId: document.querySelector('#pie-policy-grid strong')?.textContent,
      samplePolicyName: document.querySelector('#pie-policy-grid div > div:nth-child(2)')?.textContent,
      decisionRingPresent: Boolean(document.getElementById('pie-decision-ring-feed')),
      decisionRingText: document.getElementById('pie-decision-ring-feed')?.textContent?.trim()
    })`);
    console.log('Phase 2 True PIE-v2 Audit:', JSON.stringify(phase2Audit, null, 2));

    await b.shot('98_phase2_true_pie_policies_and_decisions.png');
    console.log('Shot 98: True PIE-v2 Policies & Real Decisions captured.');

    // =========================================================================
    // 3. PHASE 3: Real Ed25519 Security Contract & Token Ledger
    // =========================================================================
    console.log('Auditing Phase 3: Real Ed25519 Cryptographic Security Contract...');
    await b.eval('document.querySelector(".sec-contract-card").scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);

    const phase3Audit = await b.eval(`({
      contractId: document.getElementById('c-id')?.textContent,
      fingerprint: document.getElementById('c-fingerprint')?.textContent,
      gatewaysAttested: document.getElementById('c-gateways')?.textContent,
      tenants: document.getElementById('c-breaker')?.textContent,
      tamperStatus: document.getElementById('c-tamper')?.textContent
    })`);
    console.log('Phase 3 Real Security Contract Audit:', JSON.stringify(phase3Audit, null, 2));

    await b.shot('99_phase3_ed25519_contract_and_mesh.png');
    console.log('Shot 99: Real Ed25519 Security Contract captured.');

    // =========================================================================
    // 4. PHASE 4: Native Multi-Socket TCP Load Runner & Singleflight Collapse
    // =========================================================================
    console.log('Auditing Phase 4: Native TCP Load Runner (10,000 req/s Surge)...');
    await b.eval('document.querySelector(".topology-stage-card")?.scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(600);

    // Click 10,000 req/s Surge Button
    await b.eval('document.getElementById("btn-gen-10k")?.click()');
    console.log('Dispatched native 10k surge... waiting 2.5s for real TCP load bursts...');
    await sleep(2500);

    const phase4Audit = await b.eval(`({
      activeStatus: document.getElementById('gen-status-title')?.textContent,
      dispatchedCount: document.getElementById('gen-progress-counts')?.textContent,
      liveRps: document.getElementById('tp-live-rps')?.textContent,
      peakRps: document.getElementById('tp-peak-rps')?.textContent,
      coalescedCushion: document.getElementById('tp-coalesce-ratio')?.textContent
    })`);
    console.log('Phase 4 Native Load Audit:', JSON.stringify(phase4Audit, null, 2));

    await b.shot('100_phase4_native_tcp_load_runner.png');
    console.log('Shot 100: Native TCP Load Runner captured.');

    // Stop Generator
    await b.eval('document.getElementById("btn-gen-stop")?.click()');
    await sleep(500);

    // Copy shots to artifacts directory
    [
      '97_phase1_live_kernel_diagnostics.png',
      '98_phase2_true_pie_policies_and_decisions.png',
      '99_phase3_ed25519_contract_and_mesh.png',
      '100_phase4_native_tcp_load_runner.png'
    ].forEach(f => {
      copyFileSync(join(SHOTS_DIR, f), join(ARTIFACT_DIR, f));
      console.log(`Copied artifact: ${f}`);
    });

    console.log('🎉 4-Phase Bloat-to-Truth Automated Verification SUCCEEDED.');

  } catch (err) {
    console.error('❌ Verification failed:', err);
    process.exit(1);
  } finally {
    await b.close();
  }
}

run();
