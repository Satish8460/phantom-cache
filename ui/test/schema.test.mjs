import test from 'node:test';
import assert from 'node:assert/strict';
import { parseSchema2, findViolations, MAX_EVENT_BYTES } from '../src/lib/schema.js';
import { livePayload } from './helpers/fixtures.mjs';

test('a well-formed live frame parses with no violations', () => {
  const r = parseSchema2(JSON.stringify(livePayload()));
  assert.equal(r.ok, true);
  assert.deepEqual(r.violations, []);
  assert.equal(r.payload.schema, 2);
});

test('schema 1 / missing schema is rejected, not coerced', () => {
  const v1 = { metrics: { throughput_req_sec: 5 }, nodes: [] };
  assert.deepEqual(parseSchema2(JSON.stringify(v1)), { ok: false, code: 'unsupported_schema', detail: 'missing' });
  assert.equal(parseSchema2(JSON.stringify({ ...livePayload(), schema: 3 })).detail, '3');
});

test('garbage frames fail closed', () => {
  assert.equal(parseSchema2('').code, 'empty_frame');
  assert.equal(parseSchema2(undefined).code, 'empty_frame');
  assert.equal(parseSchema2('{not json').code, 'invalid_json');
  assert.equal(parseSchema2('[1,2]').code, 'not_an_object');
  assert.equal(parseSchema2('null').code, 'not_an_object');
  assert.equal(parseSchema2(JSON.stringify({ ...livePayload(), mode: 'demo' })).code, 'unknown_mode');
  assert.equal(parseSchema2(JSON.stringify({ schema: 2, mode: 'live' })).code, 'missing_metrics');
  assert.equal(parseSchema2('x'.repeat(MAX_EVENT_BYTES + 1)).code, 'frame_too_large');
});

test('live frames with simulated fields are flagged with their path', () => {
  const p = livePayload();
  p.metrics.backend_load = { src: 'simulated', v: 0.3 };
  const r = parseSchema2(JSON.stringify(p));
  assert.equal(r.ok, true);
  assert.deepEqual(r.violations, [{ path: 'root.metrics.backend_load', code: 'simulated_in_live' }]);
});

test('unavailable-with-v and forbidden economics fields are violations in live mode only', () => {
  const p = livePayload();
  p.metrics.bytes_resident = { src: 'unavailable', v: 0, reason: 'x' };
  p.cost = { src: 'measured', v: 1 };
  const codes = findViolations(p).map((v) => v.code).sort();
  assert.deepEqual(codes, ['forbidden_in_live', 'unavailable_with_value']);
  assert.deepEqual(findViolations({ ...p, mode: 'simulation' }), []);
});

test('unknown extra fields survive parsing untouched', () => {
  const p = livePayload();
  p.metrics.origin.some_future_field = { src: 'measured', v: 42, unit: 'count' };
  p.extra_top_level = { anything: true };
  const r = parseSchema2(JSON.stringify(p));
  assert.deepEqual(r.payload.metrics.origin.some_future_field, { src: 'measured', v: 42, unit: 'count' });
  assert.deepEqual(r.payload.extra_top_level, { anything: true });
});

test('deeply nested hostile payloads do not hang or throw', () => {
  let node = { src: 'simulated', v: 1 };
  for (let i = 0; i < 200; i += 1) node = { n: node };
  const p = { schema: 2, mode: 'live', metrics: node };
  assert.doesNotThrow(() => parseSchema2(JSON.stringify(p)));
});
