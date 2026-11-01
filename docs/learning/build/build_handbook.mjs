// Builds the handbook: assembles docs/learning/src/*.md in order into PHANTOMCACHE_ENGINEERING_HANDBOOK.md (source of truth),
// renders it with pandoc to a self-contained HTML, then prints a PDF with headless Chrome (own throwaway profile).
//   node docs/learning/build/build_handbook.mjs [--no-pdf]
// Node built-ins only; pandoc and Chrome are tools already on this machine, not product dependencies.
import { readdirSync, readFileSync, writeFileSync, existsSync, mkdirSync, rmSync, statSync, renameSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';
// The repo's dependency-free CDP helper drives headless Chrome (own throwaway profile); used for Page.printToPDF so the PDF
// gets bookmarks (document outline) and a tagged structure tree, which the plain --print-to-pdf flag does not produce.
import { Browser, sleep } from '../../../ui/scripts/cdp-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
const SRC = join(ROOT, 'src');
const OUT_MD = join(ROOT, 'PHANTOMCACHE_ENGINEERING_HANDBOOK.md');
const OUT_HTML = join(ROOT, 'build', 'handbook.html');
const OUT_PDF = join(ROOT, 'PHANTOMCACHE_ENGINEERING_HANDBOOK.pdf');
const PANDOC = process.env.PANDOC || 'C:/Users/Satish/AppData/Local/Pandoc/pandoc.exe';
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';

// Order: front matter (read separately), chapters ch01..ch99, then appendices bm01..bm99. Plain name sorting would put
// 'bm*' before 'ch*' (b < c), which put the appendices ahead of Chapter 1 in the first build.
const rank = (f) => (f.startsWith('ch') ? 0 : f.startsWith('bm') ? 1 : 2);
const files = readdirSync(SRC)
  .filter((f) => /^(\d{2}|ch\d{2}|bm\d{2})_.+\.md$/.test(f) && !f.startsWith('00_'))
  .sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
if (!files.length) throw new Error('no chapter files in ' + SRC);
const front = readFileSync(join(SRC, '00_FRONT.md'), 'utf8');
const body = files.map((f) => readFileSync(join(SRC, f), 'utf8').replace(/\]\(\.\.\/assets\//g, '](assets/').trim()).join('\n\n');
writeFileSync(OUT_MD, front.trim() + '\n\n' + body + '\n');
console.log('assembled', files.length, 'parts ->', OUT_MD, '(' + body.split(/\s+/).length + ' words)');

const p = spawnSync(PANDOC, [OUT_MD, '-f', 'markdown+fenced_divs+pipe_tables+yaml_metadata_block+implicit_figures-smart', '-t', 'html5', '--standalone',
  '--embed-resources', '--toc', '--toc-depth=2', '--css', join(HERE, 'handbook.css'), '--resource-path', ROOT, '-o', OUT_HTML], { encoding: 'utf8' });
if (p.status !== 0) { console.error(p.stderr || p.stdout); process.exit(1); }
if (p.stderr) console.log('pandoc warnings:\n' + p.stderr.split('\n').slice(0, 15).join('\n'));
console.log('html ->', OUT_HTML);
if (process.argv.includes('--no-pdf')) process.exit(0);

// Print to a temporary file and replace the real PDF only after checking the new file exists, is non-empty and is newer than
// the HTML. (An earlier version only checked that OUT_PDF existed, so a failed print silently left a stale PDF in place.)
const OUT_TMP = OUT_PDF.replace(/\.pdf$/, '.new.pdf');
const started = Date.now();
let printed = false;
for (let attempt = 1; attempt <= 3 && !printed; attempt++) {
  try { rmSync(OUT_TMP, { force: true }); } catch { /* ignore */ }
  let err = '';
  try {
    const b = await new Browser({ evidenceDir: 'D:/Temp/handbook_print', label: 'hbprint' }).launch();
    try {
      await b.send('Page.navigate', { url: pathToFileURL(OUT_HTML).href });
      await sleep(3000);
      await b.waitFor(`document.readyState === 'complete' && document.querySelectorAll('img').length > 0`, 90000, 500);
      await sleep(3000);
      const r = await b.send('Page.printToPDF', { preferCSSPageSize: true, printBackground: true, displayHeaderFooter: false, generateDocumentOutline: true, generateTaggedPDF: true });
      writeFileSync(OUT_TMP, Buffer.from(r.data, 'base64'));
    } finally { await b.close(); }
  } catch (e) { err = String(e).slice(0, 200); }
  printed = existsSync(OUT_TMP) && statSync(OUT_TMP).size > 100000 && statSync(OUT_TMP).mtimeMs >= started;
  console.log(`print attempt ${attempt}: ${printed ? 'ok' : 'FAILED'} ${err}`);;
}
if (!printed) { console.error('PDF was not produced; the previous PDF (if any) was left untouched'); process.exit(1); }
// If the PDF is open in a viewer (Windows locks it) do NOT fight for it: keep the new build beside it under a clear name.
try {
  renameSync(OUT_TMP, OUT_PDF);
  console.log('pdf ->', OUT_PDF, statSync(OUT_PDF).size, 'bytes');
} catch (e) {
  const alt = OUT_PDF.replace(/\.pdf$/, '_rebuilt.pdf');
  try { rmSync(alt, { force: true }); renameSync(OUT_TMP, alt); } catch { /* alt also locked */ }
  console.log(`could not replace ${OUT_PDF} (${e.code}: probably open in a viewer); the new build is at ${existsSync(alt) ? alt : OUT_TMP}`);
}
