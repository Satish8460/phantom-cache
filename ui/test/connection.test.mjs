import test from 'node:test';
import assert from 'node:assert/strict';
import {
  STALE_AFTER_MS, backoffMs, clockOffsetMs, connectionReducer, deriveConnection, initialConnection,
} from '../src/lib/connection.js';
import { livePayload } from './helpers/fixtures.mjs';

const frame = (p = livePayload()) => JSON.stringify(p);
const msg = (state, text, now) => connectionReducer(state, { type: 'message', text, now });

test('before any frame the state is loading, not live', () => {
  const c = deriveConnection(initialConnection, 5000);
  assert.equal(c.kind, 'loading');
  assert.equal(c.hasData, false);
});

test('a good frame makes it live; age past the threshold makes it stale', () => {
  const s = msg(initialConnection, frame(), 10_000);
  assert.equal(deriveConnection(s, 10_000 + 100).kind, 'live');
  assert.equal(deriveConnection(s, 10_000 + STALE_AFTER_MS).kind, 'live'); // boundary: not yet stale
  assert.equal(deriveConnection(s, 10_000 + STALE_AFTER_MS + 1).kind, 'stale');
});

test('connection loss with data is disconnected (data retained); without data it is disconnected with no data', () => {
  const withData = connectionReducer(msg(initialConnection, frame(), 1000), { type: 'error', closed: false });
  const c1 = deriveConnection(withData, 1200);
  assert.equal(c1.kind, 'disconnected');
  assert.equal(c1.hasData, true);
  const noData = connectionReducer(initialConnection, { type: 'error', closed: true });
  const c2 = deriveConnection(noData, 1200);
  assert.equal(c2.kind, 'disconnected');
  assert.equal(c2.hasData, false);
});

test('recovery: a new frame after an error returns to live and resets the attempt counter', () => {
  let s = connectionReducer(msg(initialConnection, frame(), 1000), { type: 'error', closed: false });
  s = connectionReducer(s, { type: 'error', closed: false });
  assert.equal(s.attempt, 2);
  s = msg(s, frame(), 5000);
  assert.equal(deriveConnection(s, 5050).kind, 'live');
  assert.equal(s.attempt, 0);
});

test('unreadable frames before any good frame give error; after a good frame they give degraded', () => {
  const bad = msg(initialConnection, 'not json', 100);
  assert.equal(deriveConnection(bad, 150).kind, 'error');
  assert.equal(bad.parseError.code, 'invalid_json');
  const good = msg(initialConnection, frame(), 100);
  const bothe = msg(good, '{"schema":1}', 200);
  assert.equal(deriveConnection(bothe, 250).kind, 'degraded');
  assert.equal(bothe.payload.schema, 2, 'the last good payload is kept');
  const healed = msg(bothe, frame(), 300);
  assert.equal(deriveConnection(healed, 310).kind, 'live');
  assert.equal(healed.parseError, null);
});

test('contract violations make the connection degraded', () => {
  const p = livePayload();
  p.metrics.x = { src: 'simulated', v: 1 };
  const s = msg(initialConnection, frame(p), 100);
  assert.equal(deriveConnection(s, 150).kind, 'degraded');
});

test('stale outranks degraded, disconnected outranks stale', () => {
  const p = livePayload();
  p.metrics.x = { src: 'simulated', v: 1 };
  let s = msg(initialConnection, frame(p), 0);
  assert.equal(deriveConnection(s, STALE_AFTER_MS + 10).kind, 'stale');
  s = connectionReducer(s, { type: 'error', closed: true });
  assert.equal(deriveConnection(s, STALE_AFTER_MS + 10).kind, 'disconnected');
});

test('backoff doubles, is capped, and tolerates garbage', () => {
  assert.equal(backoffMs(0), 1000);
  assert.equal(backoffMs(1), 2000);
  assert.equal(backoffMs(3), 8000);
  assert.equal(backoffMs(50), 30_000);
  assert.equal(backoffMs(-5), 1000);
});

test('clock offset is null without data and signed otherwise', () => {
  assert.equal(clockOffsetMs(null, null), null);
  assert.equal(clockOffsetMs({ as_of_ms: 1000 }, 1250), 250);
  assert.equal(clockOffsetMs({ as_of_ms: 'x' }, 1250), null);
});
