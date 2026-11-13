import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('🚀 Launching Chrome CDP to verify Phase 0: Dual-Serving Bridge at http://127.0.0.1:3201/app ...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phase0' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/app ...');
    await b.goto('http://127.0.0.1:3201/app', 3000);
    await sleep(2500);

    const title = await b.eval('document.title');
    console.log('Page Title:', title);

    const rootChildren = await b.eval('document.getElementById("root")?.children.length || 0');
    console.log('React Root Children Count:', rootChildren);

    await b.shot('103_phase0_dual_serving_react_app.png');
    console.log('Shot 103: React App on port 3201 captured.');

    copyFileSync(join(SHOTS_DIR, '103_phase0_dual_serving_react_app.png'), join(ARTIFACT_DIR, '103_phase0_dual_serving_react_app.png'));
    console.log('Copied shot 103 to artifacts.');
  } finally {
    await b.close();
  }
}

run().catch(err => {
  console.error(err);
  process.exit(1);
});
