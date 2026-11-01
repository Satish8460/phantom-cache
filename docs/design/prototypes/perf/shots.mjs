// Screenshots of direction_preview.html (own throwaway Chrome via ui/scripts/cdp-lib.mjs).
import { Browser, sleep } from '../../../../ui/scripts/cdp-lib.mjs';
import { pathToFileURL } from 'node:url';
const P = pathToFileURL('D:/Personal/phantom-cache/docs/design/prototypes/direction_preview.html').href;
const b = new Browser({ evidenceDir: 'D:/Personal/phantom-cache/docs/design/prototypes', label: 'dd' });
await b.launch();
try {
  const sets = [[1280, 800, 'light'], [1280, 800, 'dark'], [820, 1000, 'light'], [390, 844, 'dark'], [390, 844, 'light']];
  for (const [w, h, th] of sets) {
    await b.viewport(w, h, w < 500, 1);
    await b.media([{ name: 'prefers-color-scheme', value: th }]);
    await b.goto(`${P}?theme=${th}`, 700);
    await b.shot(`direction_${w}_${th}.png`, 'shots');
    await b.shot(`direction_${w}_${th}_full.png`, 'shots', true);
  }
  await b.viewport(1280, 800, false, 1);
  await b.goto(`${P}?theme=light&conn=off`, 600); await b.shot('direction_1280_light_disconnected.png', 'shots');
  await b.media([{ name: 'prefers-reduced-transparency', value: 'reduce' }, { name: 'prefers-color-scheme', value: 'dark' }]);
  await b.goto(`${P}?theme=dark`, 600); await b.shot('direction_1280_dark_reduced_transparency.png', 'shots');
  console.log('overflowX@390', await b.eval('document.documentElement.scrollWidth'));
} finally { console.log('killed', await b.close()); }
