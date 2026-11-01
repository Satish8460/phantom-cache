import test from 'node:test';
import assert from 'node:assert/strict';
import { METRICS } from '../src/lib/catalog.js';
import {
  additionalFields, capabilityView, metricView, metricViews, nodeViews, originAvailability, placeholderView, screenGate,
} from '../src/lib/viewmodel.js';
import { formatBytes, formatCount, formatDuration, formatMs, formatRatioAsPercent } from '../src/lib/format.js';
import { livePayload } from './helpers/fixtures.mjs';

test('no payload: every metric is loading, with no text', () => {
  for (const def of Object.values(METRICS)) {
    const v = metricView(null, def, 'loading');
    assert.equal(v.state, 'loading');
    assert.equal(v.text, null);
  }
});

test('live values format with units and keep their provenance class', () => {
  const p = livePayload();
  const tp = metricView(p, METRICS.throughput_req_s_1s, 'live');
  assert.equal(tp.state, 'value');
  assert.equal(tp.src, 'derived');
  assert.equal(tp.text, '1,301 req/s');
  assert.equal(metricView(p, METRICS.hit_ratio_10s, 'live').text, '87.5%');
  assert.equal(metricView(p, METRICS.latency_p99, 'live').text, '11.8 ms');
  assert.equal(metricView(p, METRICS.latency_p50, 'live').text, '0.412 ms');
  assert.equal(metricView(p, METRICS.bytes_resident, 'live').text, '1.0 MiB');
  assert.equal(metricView(p, METRICS.origin_latency_p95, 'live').src, 'derived');
});

test('a measured zero is displayed as zero; unavailable is never displayed as zero', () => {
  const p = livePayload();
  assert.equal(metricView(p, METRICS.capacity_rejections, 'live').text, '0');
  const rss = metricView(p, METRICS.rss_bytes, 'live');
  assert.equal(rss.state, 'unavailable');
  assert.equal(rss.text, null);
  assert.equal(rss.src, 'unavailable');
  assert.match(rss.reasonText, /platform/i);
  const hr1 = metricView(p, METRICS.hit_ratio_1s, 'live');
  assert.equal(hr1.state, 'unavailable');
  assert.equal(hr1.text, null);
});

test('missing fields are "missing", never zero', () => {
  const p = livePayload();
  delete p.metrics.stale_served_total;
  const v = metricView(p, METRICS.stale_served_total, 'live');
  assert.equal(v.state, 'missing');
  assert.equal(v.text, null);
});

test('stale and disconnected connections mark values stale instead of current', () => {
  const p = livePayload();
  assert.equal(metricView(p, METRICS.requests_total, 'stale').state, 'stale');
  assert.equal(metricView(p, METRICS.requests_total, 'disconnected').state, 'stale');
  assert.equal(metricView(p, METRICS.requests_total, 'degraded').state, 'value');
  // unavailable stays unavailable even when stale
  assert.equal(metricView(p, METRICS.rss_bytes, 'stale').state, 'unavailable');
});

test('contract-violating fields are withheld', () => {
  const p = livePayload();
  p.metrics.cache_hits = { src: 'simulated', v: 99999 };
  p.metrics.cache_misses = { src: 'unavailable', v: 0 };
  const hits = metricView(p, METRICS.cache_hits, 'degraded');
  assert.equal(hits.state, 'rejected');
  assert.equal(hits.text, null);
  assert.equal(metricView(p, METRICS.cache_misses, 'degraded').state, 'rejected');
});

test('a boolean in a numeric tile is withheld, not formatted', () => {
  const p = livePayload();
  p.metrics.cache_hits = { src: 'measured', v: true };
  const v = metricView(p, METRICS.cache_hits, 'live');
  assert.equal(v.state, 'rejected');
  assert.equal(v.text, null);
});

test('flag fields in the generic listing render as true/false with their provenance', () => {
  const p = livePayload();
  p.metrics.origin_cap = {
    enforcing: { src: 'measured', v: false },
    cap: { src: 'unavailable', reason: 'uncapped', unit: 'count' },
    in_flight: { src: 'measured', v: 2, unit: 'count' },
  };
  const extras = additionalFields(p);
  assert.equal(extras.find((f) => f.path === 'origin_cap.enforcing').text, 'false');
  assert.equal(extras.find((f) => f.path === 'origin_cap.cap').text, null);
  assert.equal(extras.find((f) => f.path === 'origin_cap.in_flight').text, '2 count');
});

test('simulation mode shows simulated fields with the SIMULATED class', () => {
  const p = livePayload({ mode: 'simulation' });
  p.metrics.requests_total = { src: 'simulated', v: 10 };
  const v = metricView(p, METRICS.requests_total, 'live');
  assert.equal(v.state, 'value');
  assert.equal(v.src, 'simulated');
});

test('stale cap violations are critical only when non-zero', () => {
  const p = livePayload();
  assert.equal(metricView(p, METRICS.stale_cap_violations, 'live').tone, 'neutral');
  p.metrics.stale_cap_violations.v = 2;
  assert.equal(metricView(p, METRICS.stale_cap_violations, 'live').tone, 'critical');
});

test('latency block unavailable propagates to every percentile', () => {
  const p = livePayload();
  p.metrics.latency_ms = { src: 'unavailable', reason: 'no_samples', unit: 'ms' };
  const views = metricViews(p, ['latency_p50', 'latency_p90', 'latency_p95', 'latency_p99', 'latency_samples'], 'live');
  for (const v of views) {
    assert.equal(v.state, 'unavailable', v.id);
    assert.equal(v.text, null);
  }
});

test('capability discovery maps known values and surfaces unknown ones', () => {
  const p = livePayload();
  assert.equal(capabilityView(p, 'cluster').state, 'not_enabled');
  assert.equal(capabilityView(p, 'controller').state, 'not_enabled');
  assert.equal(capabilityView(p, 'origin_metrics').state, 'enabled');
  p.capabilities.cluster = 'experimental';
  p.capabilities.controller = 'shadow';
  assert.equal(capabilityView(p, 'cluster').state, 'enabled');
  const c = capabilityView(p, 'controller');
  assert.equal(c.state, 'unknown');
  assert.equal(c.value, 'shadow');
  delete p.capabilities.origin_metrics;
  assert.equal(capabilityView(p, 'origin_metrics').state, 'unreported');
  assert.equal(capabilityView(null, 'cluster').state, 'unreported');
});

test('placeholder screens are driven by capabilities and never produce numbers', () => {
  const off = livePayload();
  assert.equal(placeholderView('cluster', off).state, 'not_enabled');
  assert.equal(placeholderView('intelligence', off).state, 'not_enabled');
  assert.equal(placeholderView('lab', off).state, 'no_contract');
  assert.equal(placeholderView('cluster', null).state, 'loading');
  const on = livePayload();
  on.capabilities.cluster = 'experimental';
  on.capabilities.controller = 'active';
  assert.equal(placeholderView('cluster', on).state, 'no_contract');
  assert.equal(placeholderView('intelligence', on).state, 'no_contract');
  for (const id of ['cluster', 'lab', 'intelligence']) {
    const v = placeholderView(id, on);
    assert.doesNotMatch(`${v.headline} ${v.detail}`.replace(/schema 2/gi, ''), /\d/,`${id} placeholder text must not contain numerals`);
  }
});

test('origin availability: whole-object unavailable, rejected, present', () => {
  const p = livePayload();
  assert.equal(originAvailability(p).state, 'present');
  p.metrics.origin = { src: 'unavailable', reason: 'origin_not_configured' };
  const a = originAvailability(p);
  assert.equal(a.state, 'unavailable');
  assert.match(a.reasonText, /origin/i);
  p.metrics.origin = { src: 'unavailable', reason: 'x', v: 0 };
  assert.equal(originAvailability(p).state, 'rejected');
  delete p.metrics.origin;
  assert.equal(originAvailability(p).state, 'unavailable');
  assert.equal(originAvailability(null).state, 'loading');
});

test('additional fields surface unknown backend fields without assuming names', () => {
  const p = livePayload();
  assert.deepEqual(additionalFields(p), [], 'a fully catalogued frame has no extras');
  p.metrics.origin.cap_budget_remaining = { src: 'measured', v: 12, unit: 'count' };
  p.metrics.brand_new_gauge = { src: 'simulated', v: 5 };
  p.metrics.another = { src: 'unavailable', reason: 'not_implemented' };
  const extras = additionalFields(p);
  assert.deepEqual(extras.map((f) => f.path).sort(), ['another', 'brand_new_gauge', 'origin.cap_budget_remaining']);
  const sim = extras.find((f) => f.path === 'brand_new_gauge');
  assert.equal(sim.state, 'rejected', 'simulated in live mode is withheld even in the generic listing');
  assert.equal(sim.text, null);
  assert.equal(extras.find((f) => f.path === 'origin.cap_budget_remaining').text, '12 count');
  assert.equal(extras.find((f) => f.path === 'another').text, null);
});

test('node list keeps the stated provenance of node state and bounds its size', () => {
  const p = livePayload();
  const n = nodeViews(p);
  assert.equal(n.length, 1);
  assert.equal(n[0].stateSrc, 'simulated');
  assert.equal(n[0].cpu.status, 'unavailable');
  p.nodes = Array.from({ length: 500 }, (_, i) => ({ node_id: i, state: 'Active', state_src: 'weird' }));
  const big = nodeViews(p);
  assert.equal(big.length, 200);
  assert.equal(big[0].stateSrc, 'unavailable', 'unknown provenance is not trusted');
  assert.deepEqual(nodeViews({ nodes: 'nope' }), []);
});

test('screenGate covers every state', () => {
  assert.equal(screenGate('loading', false), 'loading');
  assert.equal(screenGate('disconnected', false), 'disconnected');
  assert.equal(screenGate('error', false), 'error');
  assert.equal(screenGate('live', true), 'ready');
  assert.equal(screenGate('stale', true), 'stale');
  assert.equal(screenGate('disconnected', true), 'disconnected');
  assert.equal(screenGate('degraded', true), 'degraded');
});

test('formatters handle extremes and long values', () => {
  assert.equal(formatCount(1_000_000_000), '1,000,000,000');
  assert.equal(formatCount(0), '0');
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(2 ** 40 * 3), '3.0 TiB');
  assert.equal(formatBytes(2 ** 50), '1024 TiB');
  assert.equal(formatMs(0.0004), '0.000');
  assert.equal(formatRatioAsPercent(1), '100.0%');
  assert.equal(formatDuration(90_061_000), '25h 1m');
  assert.equal(formatDuration(3 * 86_400_000 + 7_200_000), '3d 2h');
  assert.equal(formatDuration(-1), 'unknown');
  assert.equal(formatDuration(Number.NaN), 'unknown');
});
