import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';

let H;
before(async () => {
  H = await getHarness();
});

function states() {
  const stalePayload = livePayload();
  const degraded = livePayload();
  degraded.metrics.cache_hits = { src: 'simulated', v: 1 };
  const noOrigin = livePayload();
  noOrigin.metrics.origin = { src: 'unavailable', reason: 'origin_not_configured' };
  const cluster = livePayload();
  cluster.capabilities.cluster = 'experimental';
  cluster.capabilities.controller = 'active';
  return {
    loading: makeCtx({}),
    disconnected: makeCtx({ phase: 'closed' }),
    error: makeCtx({ frames: ['{"schema":1}'] }),
    live: makeCtx({ frames: [livePayload()] }),
    stale: makeCtx({ frames: [stalePayload], receivedAt: 0, now: 60_000 }),
    degraded: makeCtx({ frames: [degraded] }),
    noOrigin: makeCtx({ frames: [noOrigin] }),
    cluster: makeCtx({ frames: [cluster] }),
  };
}

test('every screen, including the simulation explainer, is lint-clean in every data state', () => {
  const all = states();
  const failures = [];
  for (const id of H.SCREEN_IDS) {
    for (const [name, ctx] of Object.entries(all)) {
      for (const issue of lintHtml(H.renderScreen(id, ctx))) failures.push(`${id}/${name}: ${issue.rule}: ${issue.detail}`);
    }
  }
  assert.deepEqual(failures, []);
});

test('shell (header + nav) is lint-clean for every route', () => {
  for (const id of ['overview', 'traffic', 'cache', 'freshness', 'origin', 'cluster', 'lab', 'intelligence', 'simulation']) {
    for (const menuOpen of [false, true]) {
      assert.deepEqual(lintHtml(H.renderShell({ connKind: 'live', currentId: id, menuOpen }), { requireH1: false }), [], `${id} menuOpen=${menuOpen}`);
    }
  }
});

test('the lint itself catches the defects it claims to catch', () => {
  const bad = [
    ['<h1>a</h1><h1>b</h1>', 'single-h1'],
    ['<h1>a</h1><h3>x</h3>', 'heading-order'],
    ['<h1>a</h1><button></button>', 'accessible-name'],
    ['<h1>a</h1><svg viewBox="0 0 1 1"></svg>', 'svg-name'],
    ['<h1>a</h1><div id="x"></div><div id="x"></div>', 'unique-id'],
    ['<h1>a</h1><div aria-labelledby="nope"></div>', 'aria-ref'],
    ['<h1>a</h1><table><tr><th>h</th></tr></table>', 'table-name'],
    ['<h1>a</h1><table><caption>c</caption><tr><th>h</th></tr></table>', 'th-scope'],
    ['<h1>a</h1><div tabindex="3"></div>', 'tabindex'],
    ['<h1>a</h1><img src="x">', 'img-alt'],
    ['<h1>a</h1><div style="color:red">x</div>', 'inline-colour'],
    ['<h1>a</h1><div data-metric="m">1</div>', 'provenance'],
    ['<h1>a</h1><section>x</section>', 'region-name'],
  ];
  for (const [html, rule] of bad) {
    assert.ok(lintHtml(html).some((i) => i.rule === rule), `${rule} not detected in ${html}`);
  }
  assert.deepEqual(lintHtml('<h1>ok</h1><button>Go</button><svg aria-hidden="true"></svg>'), []);
});

test('keyboard: every interactive element is a native button or link, with no positive tabindex', () => {
  const html = H.renderScreen('overview', makeCtx({ frames: [livePayload()] })) + H.renderShell({ connKind: 'live', currentId: 'overview' });
  assert.doesNotMatch(html, /tabindex="[1-9]/);
  assert.doesNotMatch(html, /onclick=|role="button"/);
  assert.match(html, /<h1 tabindex="-1"/, 'heading is a programmatic focus target for route changes');
});
