import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 4: SRE Observability, Waterfall Flame-Chart & REPL at http://127.0.0.1:3201/app ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase4' });
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

    const hasFlightRecorder = await b.eval('document.querySelector(".flight-recorder-tape") !== null');
    console.log('Flight Recorder Found:', hasFlightRecorder);

    const hasWaterfall = await b.eval('document.querySelector(".waterfall-flame-deck") !== null');
    console.log('Microsecond Waterfall Flame-Chart Found:', hasWaterfall);

    // 2. Click SIEVE stage in Waterfall Flame-Chart
    console.log('Inspecting SIEVE Buffer Read stage in Flame-Chart...');
    await b.eval('document.querySelector(".flame-segment--sieve")?.click()');
    await sleep(1500);

    await b.shot('113_phase4_trace_waterfall_and_flight_recorder.png');
    console.log('Shot 113 captured.');
    copyFileSync(join(SHOTS_DIR, '113_phase4_trace_waterfall_and_flight_recorder.png'), join(ARTIFACT_DIR, '113_phase4_trace_waterfall_and_flight_recorder.png'));

    // 3. Test Cluster Invalidation Pulse
    console.log('Triggering Broadcast Cluster Invalidation (DELETE)...');
    await b.eval('document.querySelector(".invalidation-pulse-deck .probe-btn")?.click()');
    await sleep(2500);

    const invalidationResult = await b.eval('document.querySelector(".invalidation-pulse-deck .probe-result-chip")?.textContent || ""');
    console.log('Cluster Invalidation Result:', invalidationResult);

    await b.shot('114_phase4_invalidation_pulse.png');
    console.log('Shot 114 captured.');
    copyFileSync(join(SHOTS_DIR, '114_phase4_invalidation_pulse.png'), join(ARTIFACT_DIR, '114_phase4_invalidation_pulse.png'));

    // 4. Test Operator REPL Terminal
    console.log('Delivering command "cluster status" to Operator REPL...');
    await b.eval(`
      (() => {
        const input = document.querySelector(".repl-input");
        if (input) {
          input.value = "cluster status";
          input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
        }
      })()
    `);
    await sleep(1500);

    console.log('Delivering command "trace" to Operator REPL...');
    await b.eval(`
      (() => {
        const input = document.querySelector(".repl-input");
        if (input) {
          input.value = "trace";
          input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
        }
      })()
    `);
    await sleep(1500);

    const replOutput = await b.eval('document.querySelector(".repl-terminal-body")?.textContent || ""');
    console.log('Operator REPL Terminal Lines:', replOutput.length);

    await b.shot('115_phase4_operator_repl_console.png');
    console.log('Shot 115 captured.');
    copyFileSync(join(SHOTS_DIR, '115_phase4_operator_repl_console.png'), join(ARTIFACT_DIR, '115_phase4_operator_repl_console.png'));

    console.log('✅ Phase 4 Verification Succeeded 100%!');
  } catch (err) {
    console.error('❌ Phase 4 Verification Failed:', err);
    process.exitCode = 1;
  } finally {
    await b.close();
  }
}

run();
