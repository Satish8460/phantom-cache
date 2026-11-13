// Builds the handbook: assembles docs/learning/src/*.md in order into PHANTOMCACHE_ENGINEERING_HANDBOOK.md (source of truth),
// renders it with pandoc to a self-contained HTML, then prints a PDF with headless Chrome (own throwaway profile).
//   node docs/learning/build/build_handbook.mjs [--no-pdf]
// Node built-ins only; pandoc and Chrome are tools already on this machine, not product dependencies.
import { readdirSync, readFileSync, writeFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
const SRC = join(ROOT, 'src');
const OUT_MD = join(ROOT, 'PHANTOMCACHE_ENGINEERING_HANDBOOK.md');
const OUT_HTML = join(ROOT, 'build', 'handbook.html');
const OUT_PDF = join(ROOT, 'PHANTOMCACHE_ENGINEERING_HANDBOOK.pdf');
const PANDOC = process.env.PANDOC || 'C:/Users/Satish/AppData/Local/Pandoc/pandoc.exe';
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';

const files = readdirSync(SRC).filter((f) => /^(\d{2}|ch\d{2}|bm\d{2})_.+\.md$/.test(f) && !f.startsWith('00_')).sort();
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

const profile = 'D:/Temp/handbook_chrome_' + Math.random().toString(36).slice(2, 8);
mkdirSync(profile, { recursive: true });
const c = spawnSync(CHROME, ['--headless=new', '--disable-gpu', '--no-first-run', '--no-pdf-header-footer', `--user-data-dir=${profile}`,
  `--print-to-pdf=${OUT_PDF}`, '--virtual-time-budget=20000', pathToFileURL(OUT_HTML).href], { encoding: 'utf8', timeout: 180000 });
try { rmSync(profile, { recursive: true, force: true }); } catch { /* may be locked briefly */ }
if (!existsSync(OUT_PDF)) { console.error('pdf not produced', c.stderr); process.exit(1); }
console.log('pdf ->', OUT_PDF);
