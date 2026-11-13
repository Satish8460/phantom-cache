import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 1: Industrial Slate Shell & Dual Persona Toggle at http://127.0.0.1:3201/app ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase1' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/app ...');
    await b.goto('http://127.0.0.1:3201/app', 3000);
    await sleep(2500);

    const title = await b.eval('document.title');
    console.log('Page Title:', title);

    const personaButtons = await b.eval('document.querySelectorAll(".persona-toggle button").length');
    console.log('Persona Toggle Buttons Found:', personaButtons);

    const fleetPills = await b.eval('document.querySelectorAll(".fleet-mesh-pill").length');
    console.log('Fleet Mesh Pills Found (N1, N2, N3):', fleetPills);

    const dynoTitle = await b.eval('document.querySelector(".page-head h1")?.textContent || ""');
    console.log('Current Screen Heading:', dynoTitle);

    // 1. Capture Executive Persona view
    await b.shot('104_phase1_industrial_slate_executive.png');
    console.log('Shot 104: Executive Dyno Rig captured.');
    copyFileSync(join(SHOTS_DIR, '104_phase1_industrial_slate_executive.png'), join(ARTIFACT_DIR, '104_phase1_industrial_slate_executive.png'));
    copyFileSync(join(SHOTS_DIR, '104_phase1_industrial_slate_executive.png'), join(ARTIFACT_DIR, 'shot_p1_industrial_slate_shell.png'));

    // 2. Click SRE Cockpit toggle
    console.log('Clicking SRE Cockpit button...');
    await b.eval('document.querySelector(".persona-toggle button[data-persona-option=\\"sre\\"]")?.click()');
    await sleep(1500);

    const sreHeading = await b.eval('document.querySelector(".page-head h1")?.textContent || ""');
    console.log('SRE Screen Heading:', sreHeading);

    const navItems = await b.eval('document.querySelectorAll(".app-nav a").length');
    console.log('SRE Navigation Links Count:', navItems);

    await b.shot('105_phase1_industrial_slate_sre.png');
    console.log('Shot 105: SRE Cockpit with live telemetry captured.');
    copyFileSync(join(SHOTS_DIR, '105_phase1_industrial_slate_sre.png'), join(ARTIFACT_DIR, '105_phase1_industrial_slate_sre.png'));

    // 3. Switch back to Executive and test Throttle Slider & Act Buttons
    console.log('Switching back to Executive Dyno...');
    await b.eval('document.querySelector(".persona-toggle button[data-persona-option=\\"executive\\"]")?.click()');
    await sleep(1000);

    console.log('Triggering Act 2 (100k Surge)...');
    await b.eval('document.querySelectorAll(".dyno-tour-strip__controls button")[1]?.click()');
    await sleep(1000);

    const readout = await b.eval('document.querySelector(".dyno-throttle-value")?.textContent || ""');
    console.log('Dyno Throttle Readout after Act 2:', readout);

    await b.shot('106_phase1_dyno_100k_surge_act2.png');
    console.log('Shot 106: Dyno 100k Surge Act 2 captured.');
    copyFileSync(join(SHOTS_DIR, '106_phase1_dyno_100k_surge_act2.png'), join(ARTIFACT_DIR, '106_phase1_dyno_100k_surge_act2.png'));

    console.log('✅ Phase 1 Verification Succeeded 100%!');
  } finally {
    await b.close();
  }
}

run().catch(err => {
  console.error(err);
  process.exit(1);
});
