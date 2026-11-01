// Builds the short guide: docs/learning/short/PHANTOMCACHE_SHORT_GUIDE.md -> HTML (pandoc) -> PDF (headless Chrome, own throwaway profile).
//   node docs/learning/short/build_short.mjs [--no-pdf]
// Node built-ins only. Reads ONLY the short guide md; uses ../build/handbook.css by reference plus short.css. Does not touch the handbook.
import { readFileSync, existsSync, mkdirSync, rmSync, statSync, renameSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const IN_MD = join(HERE, 'PHANTOMCACHE_SHORT_GUIDE.md');
const OUT_HTML = join(HERE, 'PHANTOMCACHE_SHORT_GUIDE.html');
const OUT_PDF = join(HERE, 'PHANTOMCACHE_SHORT_GUIDE.pdf');
const CSS_BASE = join(HERE, '..', 'build', 'handbook.css');
const CSS_EXTRA = join(HERE, 'short.css');
const PANDOC = process.env.PANDOC || 'C:/Users/Satish/AppData/Local/Pandoc/pandoc.exe';
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';

const words = readFileSync(IN_MD, 'utf8').split(/\s+/).filter(Boolean).length;
console.log('source words (whitespace tokens, incl. title block):', words);

const p = spawnSync(PANDOC, [IN_MD, '-f', 'markdown+fenced_divs+pipe_tables+yaml_metadata_block+implicit_figures+link_attributes-smart', '-t', 'html5',
  '--standalone', '--embed-resources', '--css', CSS_BASE, '--css', CSS_EXTRA, '--resource-path', HERE, '-o', OUT_HTML], { encoding: 'utf8' });
if (p.status !== 0) { console.error(p.stderr || p.stdout); process.exit(1); }
if (p.stderr) console.log('pandoc warnings:\n' + p.stderr.split('\n').slice(0, 15).join('\n'));
console.log('html ->', OUT_HTML);
if (process.argv.includes('--no-pdf')) process.exit(0);

// Print to a temp file; replace the real PDF only after the new file is checked (exists, non-empty, newer than start).
const OUT_TMP = OUT_PDF.replace(/\.pdf$/, '.new.pdf');
const started = Date.now();
let printed = false;
for (let attempt = 1; attempt <= 3 && !printed; attempt++) {
  try { rmSync(OUT_TMP, { force: true }); } catch { /* ignore */ }
  const profile = 'D:/Temp/short_chrome_' + Math.random().toString(36).slice(2, 8);
  mkdirSync(profile, { recursive: true });
  const c = spawnSync(CHROME, ['--headless=new', '--disable-gpu', '--no-first-run', '--no-pdf-header-footer', `--user-data-dir=${profile}`,
    `--print-to-pdf=${OUT_TMP}`, '--virtual-time-budget=30000', pathToFileURL(OUT_HTML).href], { encoding: 'utf8', timeout: 120000 });
  try { rmSync(profile, { recursive: true, force: true }); } catch { /* may be locked briefly */ }
  printed = existsSync(OUT_TMP) && statSync(OUT_TMP).size > 20000 && statSync(OUT_TMP).mtimeMs >= started;
  console.log(`print attempt ${attempt}: ${printed ? 'ok' : 'FAILED'} (exit ${c.status})${printed ? '' : ' ' + String(c.stderr || '').slice(0, 200)}`);
}
if (!printed) { console.error('PDF was not produced; any previous PDF was left untouched'); process.exit(1); }
let target = OUT_PDF;
try { renameSync(OUT_TMP, OUT_PDF); }
catch (e) {
  const alt = OUT_PDF.replace(/\.pdf$/, '_rebuilt.pdf');
  try { rmSync(alt, { force: true }); renameSync(OUT_TMP, alt); target = alt; } catch { target = OUT_TMP; }
  console.log(`could not replace ${OUT_PDF} (${e.code}); new build at ${target}`);
}
const buf = readFileSync(target).toString('latin1');
const pages = (buf.match(/\/Type\s*\/Page[^s]/g) || []).length;
console.log('pdf ->', target, statSync(target).size, 'bytes; pages:', pages);
