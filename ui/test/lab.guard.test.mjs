// Source-level guards for the Phantom Lab code: honesty, credentials, network, storage. Complements forbidden.test.mjs.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { listSourceFiles } from '../scripts/forbidden-patterns.mjs';

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src');
const rel = (f) => path.relative(SRC, f).replace(/\\/g, '/');
const all = listSourceFiles(SRC).filter((f) => path.basename(f) !== 'SimDemo.jsx');
const lab = all.filter((f) => /^(screens\/lab\/|components\/Lab|lib\/lab|lib\/sse|hooks\/(Lab|useLab|useRunStream))/.test(rel(f)));
const read = (f) => fs.readFileSync(f, 'utf8');

test('lab file set is found (the guards below are not vacuous)', () => {
  assert.ok(lab.length >= 17, `found ${lab.length}: ${lab.map(rel)}`);
  assert.ok(lab.some((f) => rel(f) === 'lib/labApi.js') && lab.some((f) => rel(f) === 'lib/sse.js'));
});

test('network access exists in exactly one module (lib/labApi.js) and is never EventSource, XHR, WebSocket or a beacon', () => {
  for (const f of all) {
    const text = read(f);
    assert.doesNotMatch(text, /XMLHttpRequest|sendBeacon|WebSocket/, rel(f));
    if (rel(f) !== 'lib/labApi.js') assert.doesNotMatch(text, /globalThis\.fetch|window\.fetch|\bfetch\s*\(/, `${rel(f)} must go through the Lab client`);
  }
  for (const f of lab) {
    const code = read(f).split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');
    assert.doesNotMatch(code, /EventSource/, `${rel(f)}: the Lab uses fetch streaming, never EventSource`);
  }
  assert.match(read(path.join(SRC, 'lib/labApi.js')), /globalThis\.fetch/);
});

test('the token never touches a URL, storage, cookies, console or the document', () => {
  for (const f of lab) {
    const code = read(f).split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');
    assert.doesNotMatch(code, /localStorage|sessionStorage|indexedDB|document\.cookie/, `${rel(f)} must not persist anything`);
    assert.doesNotMatch(code, /[?&]token=|access_token|\?auth=|searchParams\.set\(['"](token|auth)/i, `${rel(f)} must not put a credential in a URL`);
    assert.doesNotMatch(code, /console\.(log|info|debug|warn|error)/, `${rel(f)} must not log`);
    assert.doesNotMatch(code, /dangerouslySetInnerHTML|innerHTML|eval\(|new Function/, `${rel(f)} must not inject markup`);
  }
  const provider = read(path.join(SRC, 'screens/lab/LabScreen.jsx'));
  assert.match(provider, /tokenRef = useRef\(null\)/, 'the token lives in a ref');
  const ctx = read(path.join(SRC, 'hooks/LabContext.js'));
  assert.doesNotMatch(ctx, /token\s*:/i, 'the context value never exposes the token itself');
});

test('no randomness and no result-like numeric literals in Lab code: every displayed number comes from a payload', () => {
  const allowedPhrases = [/95% (BCa|interval)/, /95% BCa/];
  for (const f of lab) {
    const lines = read(f).split('\n');
    lines.forEach((line, i) => {
      if (/^\s*(\/\/|\/\*|\*)/.test(line)) return;
      assert.doesNotMatch(line, /Math\.random|crypto\.getRandomValues/, `${rel(f)}:${i + 1}`);
      // a number with a result unit inside a string or JSX text
      const m = /['"`>][^'"`<>]*\b\d+(\.\d+)?\s?(ms|%|pp|req\/s|rps)\b/.exec(line);
      if (m && !allowedPhrases.some((re) => re.test(line))) assert.fail(`${rel(f)}:${i + 1} has a numeric literal with a unit: ${line.trim()}`);
    });
  }
});

test('the Lab ships no new dependency and the screen registry marks Lab as a real screen', () => {
  const pkg = JSON.parse(read(path.join(SRC, '../package.json')));
  assert.deepEqual(Object.keys(pkg.dependencies).sort(), ['lucide-react', 'react', 'react-dom']);
  // axe-core 4.14.0 is the single owner-approved dev-only addition (2026-10-30); see forbidden.test.mjs.
  assert.deepEqual(Object.keys(pkg.devDependencies).sort(), ['@vitejs/plugin-react', 'axe-core', 'vite']);
  const routes = read(path.join(SRC, 'lib/routes.js'));
  assert.match(routes, /id: 'lab', label: 'Phantom Lab', kind: 'lab'/);
});

test('Lab CSS uses tokens only: no hex colours, no named colours other than CanvasText/Highlight', () => {
  const css = read(path.join(SRC, 'styles/lab.css'));
  assert.doesNotMatch(css, /#[0-9a-fA-F]{3,8}\b/);
  assert.doesNotMatch(css, /\b(rgb|rgba|hsl|hsla)\(/);
  assert.doesNotMatch(css, /(?<![-\w])(red|green|blue|orange|yellow|white|black|gray|grey)(?![-\w])/);
});

test('Lab text never claims the intelligence engine drives live traffic', () => {
  const text = lab.map(read).join('\n');
  assert.match(text, /not part of the live serving path/);
  assert.doesNotMatch(text, /controls? (your )?live traffic(?!\.)|live traffic is (being )?controlled|auto-?tun(es|ing) (your|the) gateway|currently (adapts|adjusts) the (live )?cap/i);
});
