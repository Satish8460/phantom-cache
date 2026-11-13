import test from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_GATEWAY, eventsUrl, resolveGateway } from '../src/lib/config.js';
import { SCREENS, LAB_VIEWS, footerFor, parseRoute, nextNavIndex, hrefFor, titleFor } from '../src/lib/routes.js';
import { applyTheme, loadTheme, nextTheme, saveTheme, getStorage, THEME_OPTIONS } from '../src/lib/theme.js';
import { HISTORY_CAP, MIN_SAMPLE_GAP_MS, emptySeries, pushPoint, toSegments } from '../src/lib/history.js';

test('gateway URL: default, query, env, and rejection of non-http schemes', () => {
  assert.deepEqual(resolveGateway({}), { url: DEFAULT_GATEWAY, source: 'default' });
  assert.deepEqual(resolveGateway({ search: '?gateway=http://10.0.0.5:9000/some/path' }), { url: 'http://10.0.0.5:9000', source: 'query' });
  assert.equal(resolveGateway({ envValue: 'https://gw.example:8443' }).url, 'https://gw.example:8443');
  assert.equal(resolveGateway({ search: '?gateway=http://a', envValue: 'http://b' }).url, 'http://a', 'query beats env');
  const js = resolveGateway({ search: '?gateway=javascript:alert(1)' });
  assert.equal(js.url, DEFAULT_GATEWAY);
  assert.match(js.error, /http and https/);
  assert.match(resolveGateway({ search: '?gateway=not a url' }).error, /not a valid URL/);
});

test('events URL always requests schema 2', () => {
  assert.equal(eventsUrl('http://x:1'), 'http://x:1/api/events?schema=2');
});

test('routes: every screen is addressable, unknown routes are flagged, legacy sim link works', () => {
  for (const s of SCREENS) assert.deepEqual(parseRoute({ hash: hrefFor(s.id) }), { id: s.id, notFound: false });
  assert.deepEqual(parseRoute({}), { id: 'overview', notFound: false });
  assert.deepEqual(parseRoute({ hash: '#/nope' }), { id: 'overview', notFound: true });
  assert.deepEqual(parseRoute({ hash: '#/<script>' }), { id: 'overview', notFound: true });
  assert.deepEqual(parseRoute({ search: '?view=sim-demo' }), { id: 'simulation', notFound: false });
  assert.equal(SCREENS.filter((s) => s.kind === 'sim').length, 1);
  assert.equal(new Set(SCREENS.map((s) => s.id)).size, SCREENS.length);
});

test('nav keyboard: arrows wrap, Home/End jump, other keys are ignored', () => {
  assert.equal(nextNavIndex('ArrowDown', 0, 5), 1);
  assert.equal(nextNavIndex('ArrowDown', 4, 5), 0);
  assert.equal(nextNavIndex('ArrowUp', 0, 5), 4);
  assert.equal(nextNavIndex('Home', 3, 5), 0);
  assert.equal(nextNavIndex('End', 1, 5), 4);
  assert.equal(nextNavIndex('Tab', 1, 5), null);
  assert.equal(nextNavIndex('ArrowDown', 0, 0), null);
});

test('theme: cycle, normalisation, storage failures and DOM application', () => {
  assert.equal(nextTheme('system'), 'light');
  assert.equal(nextTheme('light'), 'dark');
  assert.equal(nextTheme('dark'), 'system');
  const store = new Map();
  const ok = { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) };
  saveTheme(ok, 'dark');
  assert.equal(loadTheme(ok), 'dark');
  ok.setItem('phantom.ui.theme', 'purple');
  assert.equal(loadTheme(ok), 'system');
  const throwing = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('blocked'); } };
  assert.equal(loadTheme(throwing), 'system');
  assert.doesNotThrow(() => saveTheme(throwing, 'dark'));
  assert.equal(loadTheme(null), 'system');
  assert.equal(getStorage({ get localStorage() { throw new Error('denied'); } }), null);
  const attrs = {};
  const root = { setAttribute: (k, v) => { attrs[k] = v; }, removeAttribute: (k) => { delete attrs[k]; } };
  applyTheme(root, 'dark');
  assert.equal(attrs['data-theme'], 'dark');
  applyTheme(root, 'system');
  assert.equal('data-theme' in attrs, false);
});

test('history: stores only server values, gaps for unavailable, throttled and capped', () => {
  let s = emptySeries();
  s = pushPoint(s, 1000, { src: 'derived', v: 5 });
  const same = pushPoint(s, 1000 + MIN_SAMPLE_GAP_MS - 1, { src: 'derived', v: 6 });
  assert.equal(same, s, 'throttled push returns the same array');
  s = pushPoint(s, 2000, { src: 'unavailable', reason: 'insufficient_window' });
  s = pushPoint(s, 3000, { src: 'derived', v: 7 });
  assert.deepEqual(s.map((p) => p.v), [5, null, 7]);
  s = pushPoint(s, 4000, { src: 'simulated', v: 8 }, { mode: 'live' });
  assert.equal(s[s.length - 1].v, null, 'simulated in live mode becomes a gap');
  let big = emptySeries();
  for (let i = 0; i < HISTORY_CAP + 50; i += 1) big = pushPoint(big, i * 1000, { src: 'measured', v: i });
  assert.equal(big.length, HISTORY_CAP);
  assert.equal(big[big.length - 1].v, HISTORY_CAP + 49);
});

test('toSegments splits at gaps and handles empty / single / flat series', () => {
  assert.deepEqual(toSegments([], 100, 50).segments, []);
  assert.equal(toSegments([{ t: 1, v: null }], 100, 50).min, null);
  const gap = toSegments([{ t: 1, v: 1 }, { t: 2, v: null }, { t: 3, v: 3 }], 100, 50);
  assert.equal(gap.segments.length, 2);
  const flat = toSegments([{ t: 1, v: 4 }, { t: 2, v: 4 }], 100, 50);
  assert.equal(flat.segments[0].every(([, y]) => Number.isFinite(y)), true);
  assert.equal(toSegments([{ t: 1, v: 4 }], 100, 50).segments[0].length, 1);
});

test('routes (I4): every route has a distinct document title and a footer that is true for its kind; Lab tabs keep their order', () => {
  const titles = SCREENS.map((s) => titleFor({ id: s.id }));
  assert.equal(new Set(titles).size, SCREENS.length);
  assert.equal(titleFor({ id: 'lab' }), 'Phantom Lab | PhantomCache');
  assert.match(footerFor({ id: 'traffic' }), /Telemetry schema 2/);
  assert.doesNotMatch(footerFor({ id: 'lab' }), /Telemetry schema 2/);
  assert.doesNotMatch(footerFor({ id: 'simulation' }), /Telemetry schema 2/);
  assert.deepEqual(LAB_VIEWS.map((v) => v.label), ['About', 'Start a run', 'Runs', 'Evidence']);
  assert.deepEqual(THEME_OPTIONS.map((o) => o.label), ['Light', 'System', 'Dark']);
});
