import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'truthful_roi' });
  await b.launch();
  await b.viewport(1920, 1080);

  try {
    console.log('Navigating to http://127.0.0.1:3201/seller ...');
    await b.goto('http://127.0.0.1:3201/seller', 2000);
    await sleep(1500);

    await b.eval('document.getElementById("economic-ledger-section")?.scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);
    await b.shot('101_truthful_seller_ledger.png');
    console.log('Shot 101 captured: Truthful Seller Ledger');

    console.log('Navigating to http://127.0.0.1:3201/lab ...');
    await b.goto('http://127.0.0.1:3201/lab', 2000);
    await sleep(1500);

    await b.eval('document.querySelector(".kpi-strip")?.scrollIntoView({ behavior: "instant", block: "center" })');
    await sleep(800);
    await b.shot('102_truthful_lab_kpi_strip.png');
    console.log('Shot 102 captured: Truthful Lab KPI Strip');

    copyFileSync(join(SHOTS_DIR, '101_truthful_seller_ledger.png'), join(ARTIFACT_DIR, '101_truthful_seller_ledger.png'));
    copyFileSync(join(SHOTS_DIR, '102_truthful_lab_kpi_strip.png'), join(ARTIFACT_DIR, '102_truthful_lab_kpi_strip.png'));
    console.log('Shots 101 and 102 copied to artifacts.');
  } finally {
    await b.close();
  }
}

run().catch(err => {
  console.error(err);
  process.exit(1);
});
