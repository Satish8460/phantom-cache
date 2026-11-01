// Server-side render tests: verify what the DOM would contain in each data state, without a browser.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { getHarness } from '../scripts/render-harness.mjs';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';

let H;
before(async () => {
  H = await getHarness();
});

const DATA_SCREENS = ['overview', 'traffic', 'cache', 'freshness', 'origin'];
const count = (html, needle) => html.split(needle).length - 1;
const metricBlocks = (html) => html.split('data-metric="').length - 1;

test('loading: no payload shows a loading state and zero metric values on every screen', () => {
  const ctx = makeCtx({});
  for (const id of DATA_SCREENS) {
    const html = H.renderScreen(id, ctx);
    assert.match(html, /data-state-kind="loading"/, id);
    assert.match(html, /aria-busy="true"/, id);
    assert.equal(metricBlocks(html), 0, `${id} must not render metrics before data`);
    assert.doesNotMatch(html, /data-provenance/, `${id} shows no provenance badge without data`);
  }
});

test('disconnected with no data: explicit block with a Retry now button, no values', () => {
  const ctx = makeCtx({ phase: 'closed' });
  for (const id of DATA_SCREENS) {
    const html = H.renderScreen(id, ctx);
    assert.match(html, /data-state-kind="disconnected"/, id);
    assert.match(html, /Retry now/, id); // V2B: the one verb of the shared connection vocabulary (the retired string was "Reconnect now")
    assert.doesNotMatch(html, /Reconnect now/, id);
    assert.equal(metricBlocks(html), 0, id);
  }
});

test('unreadable first frame: error state announced as an alert', () => {
  const ctx = makeCtx({ frames: ['{"schema":1}'] });
  assert.equal(ctx.conn.kind, 'error');
  const html = H.renderScreen('traffic', ctx);
  assert.match(html, /role="alert"/);
  assert.match(html, /schema "1"|schema 2/);
  assert.equal(metricBlocks(html), 0);
});

test('live: every metric has a provenance badge, and units/values render', () => {
  const ctx = makeCtx({ frames: [livePayload()] });
  assert.equal(ctx.conn.kind, 'live');
  for (const id of DATA_SCREENS) {
    const html = H.renderScreen(id, ctx);
    assert.doesNotMatch(html, /data-connection=/, `${id}: no stale/degraded banner when live`);
    const metrics = metricBlocks(html);
    if (id === 'overview') assert.ok(metrics >= 6);
    // Each [data-metric] block must contain a badge: compare counts block by block.
    for (const block of html.split('data-metric="').slice(1)) {
      const upToNext = block.split('data-metric="')[0];
      assert.match(upToNext, /data-provenance="(measured|derived|inferred|simulated|unavailable)"/);
    }
  }
  const traffic = H.renderScreen('traffic', ctx);
  assert.match(traffic, /1,301 req\/s/);
  assert.match(traffic, /11\.8 ms/);
  assert.match(traffic, /data-provenance="derived"/);
  assert.match(traffic, /data-provenance="measured"/);
});

test('unavailable values render as words with a reason, never as 0 or a dash', () => {
  const ctx = makeCtx({ frames: [livePayload()] });
  const cache = H.renderScreen('cache', ctx);
  assert.match(cache, /data-metric="rss_bytes" data-state="unavailable"/);
  assert.match(cache, /Unavailable/);
  assert.match(cache, /Not supported on the gateway platform/);
  const block = cache.split('data-metric="rss_bytes"')[1].split('</div>')[0];
  assert.doesNotMatch(block, />0 B<|>0<|>—</);
});

test('origin not configured: one unavailable block with the reason and no origin numbers', () => {
  const p = livePayload();
  p.metrics.origin = { src: 'unavailable', reason: 'origin_not_configured' };
  p.capabilities.origin_metrics = 'unavailable';
  const html = H.renderScreen('origin', makeCtx({ frames: [p] }));
  assert.match(html, /data-state-kind="unavailable"/);
  assert.match(html, /No origin URL configured/);
  assert.equal(metricBlocks(html), 0);
});

test('origin metrics with fields from the backend that the UI does not know are listed generically', () => {
  const p = livePayload();
  p.metrics.origin.future_cap_field = { src: 'measured', v: 3, unit: 'count' };
  const html = H.renderScreen('overview', makeCtx({ frames: [p] }));
  assert.match(html, /origin\.future_cap_field/);
  assert.match(html, /3 count/);
});

test('origin screen: origin_cap is shown as the origin concurrency controller reading (V2B), not as generic fields', () => {
  const p = livePayload();
  p.metrics.origin_cap = {
    enforcing: { src: 'measured', v: true },
    cap: { src: 'unavailable', reason: 'uncapped', unit: 'count' },
    dispatched: { src: 'measured', v: 17, unit: 'count' },
  };
  const html = H.renderScreen('origin', makeCtx({ frames: [p] }));
  assert.match(html, /Origin concurrency controller/);
  assert.match(html, /data-origin-gate="enforcing"/);
  assert.match(html, /data-gate-row="dispatched"[\s\S]*?<td class="num">17</);
  assert.match(html, /No cap is configured on this gateway \(uncapped\)/);
  assert.doesNotMatch(html, /Other origin fields/, 'origin_cap no longer falls into the generic list');
  const none = H.renderScreen('origin', makeCtx({ frames: [livePayload()] }));
  assert.doesNotMatch(none, /Other origin fields/);
  assert.match(none, /Unavailable: no origin concurrency gate reported/);
});

test('stale: values stay visible but are labelled stale, with a banner', () => {
  const ctx = makeCtx({ frames: [livePayload()], receivedAt: 1_000_000, now: 1_000_000 + 10_000 });
  assert.equal(ctx.conn.kind, 'stale');
  const html = H.renderScreen('traffic', ctx);
  assert.match(html, /data-connection="stale"/);
  assert.match(html, /data-state="stale"/);
  assert.match(html, /stale, last received value/);
  assert.match(html, /last frame 10s ago/);
});

test('disconnected with data: notice with Retry now, values flagged stale', () => {
  const ctx = makeCtx({ frames: [livePayload()], phase: 'error' });
  assert.equal(ctx.conn.kind, 'disconnected');
  const html = H.renderScreen('freshness', ctx);
  assert.match(html, /data-connection="disconnected"/);
  assert.match(html, /Retry now/);
  assert.doesNotMatch(html, /Reconnect now|Unauthorised|Recovering/);
  assert.match(html, /data-state="stale"/);
});

test('degraded: contract violation is withheld and explained', () => {
  const p = livePayload();
  p.metrics.cache_hits = { src: 'simulated', v: 123456 };
  const ctx = makeCtx({ frames: [p] });
  assert.equal(ctx.conn.kind, 'degraded');
  const html = H.renderScreen('traffic', ctx);
  assert.match(html, /data-connection="degraded"/);
  assert.match(html, /data-metric="cache_hits" data-state="rejected"/);
  assert.doesNotMatch(html, /123,456/, 'the simulated number must not be displayed');
  assert.match(html, /Rejected: simulated value in live mode/);
});

test('stale cap violations render critical with text, not colour alone', () => {
  const p = livePayload();
  p.metrics.stale_cap_violations.v = 3;
  const html = H.renderScreen('freshness', makeCtx({ frames: [p] }));
  assert.match(html, /metric--critical/);
  assert.match(html, /critical: expected 0/);
});

test('hit ratio: no "100% when nothing was seen" note (the gateway now reports Unavailable with a reason); the reason is shown, never a number', () => {
  const p = livePayload();
  p.metrics.hit_ratio_cumulative = { src: 'unavailable', reason: 'no_requests', unit: 'ratio' };
  const html = H.renderScreen('traffic', makeCtx({ frames: [p] }));
  assert.doesNotMatch(html, /reports 100%/);
  assert.match(html, /data-metric="hit_ratio_cumulative" data-state="unavailable"/);
  assert.match(html, /No requests have been received yet/);
  const live = H.renderScreen('traffic', makeCtx({ frames: [livePayload()] }));
  assert.doesNotMatch(live, /reports 100%/);
});

test('placeholders: capability-driven text, no metric blocks, no fabricated numbers', () => {
  const off = makeCtx({ frames: [livePayload()] });
  // 'lab' is no longer a placeholder (G3): it is the Phantom Lab screen, covered in lab.render.test.mjs.
  for (const id of ['cluster', 'intelligence']) {
    const html = H.renderScreen(id, off);
    assert.equal(metricBlocks(html), 0, id);
    assert.match(html, /ui_api_requirements\.md/);
  }
  assert.match(H.renderScreen('cluster', off), /Cluster mode is not enabled/);
  assert.match(H.renderScreen('intelligence', off), /Phantom Intelligence is not available yet/); // V2B: the old "is not enabled ... no decisions exist" overstated absence (TRACE O-3)
  assert.doesNotMatch(H.renderScreen('cluster', off), /<table/, 'no node table when cluster is not enabled');

  const p = livePayload();
  p.capabilities.cluster = 'experimental';
  const on = H.renderScreen('cluster', makeCtx({ frames: [p] }));
  assert.match(on, /no cluster telemetry contract/);
  assert.match(on, /<table/);
  assert.match(on, /data-provenance="simulated"/, 'node state provenance is shown as the gateway states it');
  assert.doesNotMatch(on, /section for this screen/, 'no developer sentence on a product page');
});

test('placeholders before data: loading, not an empty claim', () => {
  const html = H.renderScreen('cluster', makeCtx({}));
  assert.match(html, /data-state-kind="loading"/);
});

test('simulation screen is bannered before any simulated content', () => {
  const html = H.renderScreen('simulation', makeCtx({}));
  const banner = html.indexOf('data-banner="simulated"');
  const frame = html.indexOf('data-simulated="true"');
  assert.ok(banner >= 0 && frame > banner);
  assert.match(html, /generated by a model, not measured/);
});

test('shell: nav marks the current page, flags placeholder/simulated entries in text, header shows status', () => {
  const html = H.renderShell({ connKind: 'stale', currentId: 'traffic' });
  assert.equal(count(html, 'aria-current="page"'), 1);
  assert.match(html, /data-screen="traffic"/);
  assert.match(html, /\(not yet available\)/);
  assert.match(html, /\(simulated\)/);
  assert.match(html, /Data is stale/);
  assert.match(html, />Simulation<\/span>/,'the explainer is called Simulation, not Simulation demo');
  assert.doesNotMatch(html, /Simulation demo/);
});

test('badge: unknown classes degrade to Unavailable, every class has shape and text', () => {
  for (const c of ['measured', 'derived', 'inferred', 'simulated', 'unavailable']) {
    const b = H.renderBadge(c);
    assert.match(b, /<svg/);
    assert.match(b, new RegExp(`data-provenance="${c}"`));
    assert.match(b, new RegExp(c, 'i'));
  }
  assert.match(H.renderBadge('estimated'), /data-provenance="unavailable"/);
});

test('sparkline sits in a ChartFrame: badge, Chart/Table toggle, table in the DOM, full-width axis captions and an accessible summary', () => {
  const series = [{ t: 1000, v: 5 }, { t: 2000, v: null }, { t: 3000, v: 9 }];
  const html = H.renderSparkline({ title: 'Throughput', series, src: 'derived', format: (v) => String(v), unitLabel: 'req/s' });
  assert.match(html, /role="img"/);
  assert.match(html, /aria-label="Throughput: 2 samples since page load, latest 9 req\/s\."/);
  assert.match(html, /class="chart-frame"/);
  assert.match(html, /aria-pressed="true">Chart<\/button><button[^>]*aria-pressed="false">Table<\/button>/);
  assert.match(html, /<div class="chart-frame__pane" hidden=""><div class="table-wrap"><table>/, 'the table alternative is always in the DOM');
  assert.match(html, /data-provenance="derived"/);
  assert.match(html, /<p class="chart-frame__axes">/);
  assert.match(html, /5 to 9 req\/s/);
  assert.match(html, /unavailable/);
  const empty = H.renderSparkline({ title: 'Throughput', series: [], src: 'derived', format: String });
  assert.doesNotMatch(empty, /<svg class="spark"/);
  assert.match(empty, /No samples yet/);
  const flat = H.renderSparkline({ title: 'Flat', series: [{ t: 1000, v: 4 }, { t: 2000, v: 4 }], src: 'derived', format: (v) => String(v) });
  assert.match(flat, /Constant at 4/, 'a flat series says so instead of an empty-looking axis range');
  assert.doesNotMatch(flat, /4 to 4/);
});
