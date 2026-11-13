// ui/test/cacheStatus.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { parseCacheResponse, KNOWN_CACHE_STATUSES } from '../src/lib/cacheStatus.js';

test('WP1: parseCacheResponse recognises all six authoritative cache statuses', () => {
  for (const status of KNOWN_CACHE_STATUSES) {
    const headers = new Headers({ 'x-cache-status': status });
    const res = parseCacheResponse(200, headers);
    assert.equal(res.kind, 'cache');
    assert.equal(res.verdict, status);
    assert.equal(res.error, null);
  }
});

test('WP1: parseCacheResponse handles lowercase and mixed-case status headers', () => {
  const cases = [
    ['hit', 'HIT'],
    ['Miss', 'MISS'],
    ['coalesced', 'COALESCED'],
    ['Stale', 'STALE'],
    ['bypass', 'BYPASS'],
    ['bypass-auth', 'BYPASS-AUTH'],
  ];
  for (const [input, expected] of cases) {
    const headers = new Headers({ 'x-cache-status': input });
    const res = parseCacheResponse(200, headers);
    assert.equal(res.kind, 'cache');
    assert.equal(res.verdict, expected);
  }
});

test('WP1: parseCacheResponse never infers cache status from Age header', () => {
  // Age: 0 on MISS must remain MISS
  const missHeaders = new Headers({ 'x-cache-status': 'MISS', 'age': '0' });
  const missRes = parseCacheResponse(200, missHeaders);
  assert.equal(missRes.kind, 'cache');
  assert.equal(missRes.verdict, 'MISS');

  // Age: 0 on HIT must remain HIT
  const hitHeaders = new Headers({ 'x-cache-status': 'HIT', 'age': '0' });
  const hitRes = parseCacheResponse(200, hitHeaders);
  assert.equal(hitRes.kind, 'cache');
  assert.equal(hitRes.verdict, 'HIT');

  // Age: 0 with NO X-Cache-Status must NOT be inferred as HIT
  const noStatusHeaders = new Headers({ 'age': '0' });
  const noStatusRes = parseCacheResponse(200, noStatusHeaders);
  assert.equal(noStatusRes.kind, 'non-cache');
  assert.equal(noStatusRes.verdict, 'N/A');
});

test('WP1: parseCacheResponse identifies X-Cache-Error without misleading cache verdict', () => {
  const headers = new Headers({ 'x-cache-error': 'UpstreamTimeout' });
  const res = parseCacheResponse(504, headers);
  assert.equal(res.kind, 'gateway-error');
  assert.equal(res.verdict, 'N/A');
  assert.equal(res.error, 'UpstreamTimeout');
});

test('WP1: parseCacheResponse handles 2xx non-cache responses (e.g. /health)', () => {
  const headers = new Headers({ 'content-type': 'text/plain' });
  const res = parseCacheResponse(200, headers);
  assert.equal(res.kind, 'non-cache');
  assert.equal(res.verdict, 'N/A');
  assert.equal(res.error, null);
});

test('WP1: parseCacheResponse handles HTTP errors (400, 404, 414, 500) without cache pills', () => {
  for (const code of [400, 404, 414, 500, 503]) {
    const headers = new Headers({});
    const res = parseCacheResponse(code, headers);
    assert.equal(res.kind, 'error');
    assert.equal(res.verdict, 'N/A');
    assert.equal(res.error, `HTTP ${code}`);
  }
});

test('WP1: parseCacheResponse reports unknown/unrecognised X-Cache-Status values cleanly', () => {
  const headers = new Headers({ 'x-cache-status': 'STRANGE_CUSTOM_VAL' });
  const res = parseCacheResponse(200, headers);
  assert.equal(res.kind, 'unexpected');
  assert.equal(res.verdict, 'STRANGE_CUSTOM_VAL');
  assert.ok(res.note.includes('unrecognised X-Cache-Status'));
});
