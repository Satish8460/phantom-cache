import { writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const IMG_DIR = join(HERE, 'images');
mkdirSync(IMG_DIR, { recursive: true });

const IMAGES = {
  'phantom-keyboard': 'https://images.unsplash.com/photo-1587829741301-dc798b83add3?auto=format&fit=crop&w=600&q=80',
  'phantom-mouse': 'https://images.unsplash.com/photo-1615663245857-ac93bb7c39e7?auto=format&fit=crop&w=600&q=80',
  'phantom-display-4k': 'https://images.unsplash.com/photo-1527443224154-c4a3942d3acf?auto=format&fit=crop&w=600&q=80',
  'phantom-headphones': 'https://images.unsplash.com/photo-1505740420928-5e560c06d30e?auto=format&fit=crop&w=600&q=80',
  'phantom-ssd-2tb': 'https://images.unsplash.com/photo-1597872200969-2b65d56bd16b?auto=format&fit=crop&w=600&q=80',
  'phantom-hub-pro': 'https://images.unsplash.com/photo-1622445262464-84b1456045b6?auto=format&fit=crop&w=600&q=80',
  'phantom-camera-4k': 'https://images.unsplash.com/photo-1516035069371-29a1b244cc32?auto=format&fit=crop&w=600&q=80',
  'phantom-microphone': 'https://images.unsplash.com/photo-1590602847861-f357a9332bbc?auto=format&fit=crop&w=600&q=80',
  'phantom-desk-mat': 'https://images.unsplash.com/photo-1616401784845-180882ba9ba8?auto=format&fit=crop&w=600&q=80',
  'phantom-power-100w': 'https://images.unsplash.com/photo-1583863788434-e58a36330cf0?auto=format&fit=crop&w=600&q=80',
  'phantom-cable-thunderbolt': 'https://images.unsplash.com/photo-1608156639585-b3a032ef9689?auto=format&fit=crop&w=600&q=80',
  'phantom-arm-mount': 'https://images.unsplash.com/photo-1593062096033-9a26b09da705?auto=format&fit=crop&w=600&q=80',
};

async function downloadAll() {
  console.log('Downloading 12 royalty-free precision hardware stock images...');
  for (const [sku, url] of Object.entries(IMAGES)) {
    try {
      const res = await fetch(url);
      if (res.ok) {
        const buf = Buffer.from(await res.arrayBuffer());
        const targetPath = join(IMG_DIR, `${sku}.jpg`);
        writeFileSync(targetPath, buf);
        console.log(`[OK] Saved ${sku}.jpg (${(buf.length / 1024).toFixed(1)} KB)`);
      } else {
        console.warn(`[WARN] Failed ${sku}: HTTP ${res.status}`);
      }
    } catch (err) {
      console.warn(`[WARN] Error downloading ${sku}: ${err.message}`);
    }
  }
  console.log('Stock images download complete!');
}

downloadAll();

