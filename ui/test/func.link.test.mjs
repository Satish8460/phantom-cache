// F3: one connection vocabulary (linkState.js) and the reducer additions it relies on.
import test from 'node:test';
import assert from 'node:assert/strict';
import { connectionReducer, initialConnection } from '../src/lib/connection.js';
import { initialStream, streamReducer, deriveStreamStatus } from '../src/lib/labStream.js';
import { ERROR_TEXT } from '../src/lib/labApi.js';
import {
  DISCONNECTED_FROM_ATTEMPT, LAB_KIND_CONDITION, LINK_TEXT, LINK_WORDS, RETRY_LABEL, SEVERITY_ORDER,
  describeLabLink, describeLink, describeRunStreamLink, describeTelemetryLink, worstLink,
} from '../src/lib/linkState.js';
import { frame, msg } from './helpers/linkHelpers.mjs';

const T0 = 1_000_000;

const errors = (state, n, closed = false) => {
  let s = state;
  for (let i = 0; i < n; i += 1) s = connectionReducer(s, { type: 'error', closed, now: T0 + i });
  return s;
};

// ---------------------------------------------------------------- telemetry table

test('telemetry: nothing received yet = Connecting; no values, no attempts', () => {
  const v = describeTelemetryLink(initialConnection, T0);
  assert.equal(v.word, 'Connecting');
  assert.equal(v.condition, 'connecting');
  assert.equal(v.attempts, 0);
  assert.equal(v.retry, false);
  assert.equal(v.who, 'page');
  assert.equal(v.isLink, true);
});

test('telemetry: errors 1 and 2 are Reconnecting, from the 3rd Disconnected (same rule as the run stream); with and without prior data', () => {
  for (const withData of [false, true]) {
    const base = withData ? msg(initialConnection, frame(), T0) : initialConnection;
    const words = [1, 2, 3, 4, 10].map((n) => describeTelemetryLink(errors(base, n), T0 + 100).word);
    assert.deepEqual(words, ['Reconnecting', 'Reconnecting', 'Disconnected', 'Disconnected', 'Disconnected'], `withData=${withData}`);
    const v = describeTelemetryLink(errors(base, 1), T0 + 100);
    assert.equal(v.hasStaleData, withData);
    assert.equal(v.retry, true);
    assert.equal(v.retryLabel, RETRY_LABEL);
  }
  assert.equal(DISCONNECTED_FROM_ATTEMPT, 3);
});

test('telemetry: first good frame returns to Live and clears attempts, lastError and retryAt', () => {
  let s = errors(msg(initialConnection, frame(), T0), 3);
  s = connectionReducer(s, { type: 'retryScheduled', at: T0 + 5000 });
  assert.equal(s.retryAt, T0 + 5000);
  s = msg(s, frame(), T0 + 6000);
  const v = describeTelemetryLink(s, T0 + 6050);
  assert.equal(v.word, 'Live');
  assert.equal(v.attempts, 0);
  assert.equal(v.lastError, null);
  assert.equal(v.nextRetryAt, null);
});

test('telemetry: stale (frame age > threshold), degraded (violations), incompatible frames (never a good frame) each map to one word', () => {
  const live = msg(initialConnection, frame(), T0);
  assert.equal(describeTelemetryLink(live, T0 + 500).word, 'Live');
  const stale = describeTelemetryLink(live, T0 + 4000);
  assert.equal(stale.word, 'Stale');
  assert.match(stale.detail, /No frame for 4 s/);
  assert.match(stale.basis, /0\.1 s/);
  const bad = msg(initialConnection, 'not json', T0);
  const inc = describeTelemetryLink(bad, T0 + 10);
  assert.equal(inc.word, 'Unavailable');
  assert.equal(inc.condition, 'incompatible');
  assert.equal(inc.who, 'gateway');
  const goodThenBad = msg(live, 'not json', T0 + 10);
  assert.equal(describeTelemetryLink(goodThenBad, T0 + 20).word, 'Degraded');
});

test('telemetry: who owns the retry. closed = the page (countdown is honest); error = the browser (no countdown)', () => {
  const closed = connectionReducer(errors(initialConnection, 1, true), { type: 'retryScheduled', at: T0 + 3000 });
  const vc = describeTelemetryLink(closed, T0 + 1000);
  assert.equal(vc.retryOwner, 'page');
  assert.equal(vc.nextRetryAt, T0 + 3000);
  assert.equal(vc.nextRetryInMs, 2000);
  const browser = describeTelemetryLink(errors(initialConnection, 1, false), T0 + 1000);
  assert.equal(browser.retryOwner, 'browser');
  assert.equal(browser.nextRetryAt, null);
  assert.match(browser.basis, /cannot show a countdown/);
  // clamped, never negative
  assert.equal(describeTelemetryLink(closed, T0 + 9000).nextRetryInMs, 0);
  // the page's own timer firing clears the schedule
  assert.equal(connectionReducer(closed, { type: 'connecting' }).retryAt, null);
});

test('telemetry reducer: reset returns the initial state (nothing from the old gateway survives); lastError records the error', () => {
  const s = errors(msg(initialConnection, frame(), T0), 2);
  assert.equal(s.lastError.closed, false);
  assert.equal(s.lastError.at, T0 + 1);
  const r = connectionReducer(s, { type: 'reset' });
  assert.equal(r, initialConnection);
  assert.equal(r.payload, null);
  assert.equal(r.attempt, 0);
  assert.equal(r.lastError, null);
});

// ---------------------------------------------------------------- Unauthorised is only ever an answer

test('telemetry alone is NEVER Unauthorised: not with security jwt, not with the inference opted in', () => {
  for (const n of [1, 2, 3, 20]) {
    for (const opts of [{}, { security: 'jwt' }, { security: 'jwt', includeJwtInference: true }]) {
      const v = describeTelemetryLink(errors(initialConnection, n, true), T0 + 50, opts);
      assert.notEqual(v.word, 'Unauthorised', `${n} ${JSON.stringify(opts)}`);
      assert.equal(v.who, 'page', 'a stream failure is a page observation');
    }
  }
});

test('the JWT/EventSource inference (BD-05) is a labelled note, only when opted in, only for an outage with no data, and never changes the word', () => {
  const down = errors(initialConnection, 3, true);
  const off = describeTelemetryLink(down, T0 + 50, { security: 'jwt' });
  assert.deepEqual(off.notes, []);
  const on = describeTelemetryLink(down, T0 + 50, { security: 'jwt', includeJwtInference: true });
  assert.equal(on.word, 'Disconnected');
  assert.equal(on.notes.length, 1);
  assert.equal(on.notes[0].kind, 'inference');
  assert.equal(on.notes[0].ownerDecision, 'D-3');
  assert.match(on.notes[0].text, /cannot confirm/);
  assert.deepEqual(describeTelemetryLink(down, T0 + 50, { security: 'anonymous_dev_loopback_only', includeJwtInference: true }).notes, []);
  assert.deepEqual(describeTelemetryLink(errors(msg(initialConnection, frame(), T0), 3), T0 + 50, { security: 'jwt', includeJwtInference: true }).notes, [], 'had data: not the never-got-a-frame case');
});

test('a classified probe (an actual gateway answer) can make telemetry Unauthorised; a network probe keeps it Disconnected; an ok probe adds an honest note', () => {
  const down = errors(initialConnection, 3, true);
  for (const kind of ['unauthorized_missing', 'unauthorized_invalid', 'unauthorized_expired', 'forbidden_role']) {
    const v = describeTelemetryLink(down, T0 + 50, { probe: { kind } });
    assert.equal(v.word, 'Unauthorised', kind);
    assert.equal(v.who, 'gateway');
    assert.equal(v.fix, ERROR_TEXT[kind].fix);
  }
  assert.equal(describeTelemetryLink(down, T0 + 50, { probe: { kind: 'network' } }).word, 'Disconnected');
  const ok = describeTelemetryLink(down, T0 + 50, { probe: { kind: 'ok' } });
  assert.equal(ok.word, 'Disconnected');
  assert.match(ok.notes[0].text, /cause of the stream failure is unknown/);
  // a probe is ignored while the stream is healthy
  assert.equal(describeTelemetryLink(msg(initialConnection, frame(), T0), T0 + 50, { probe: { kind: 'unauthorized_missing' } }).word, 'Live');
});

test('"Recovering" is not a word: nothing in the code distinguishes it from Reconnecting', () => {
  assert.ok(!Object.values(LINK_WORDS).includes('Recovering'));
  assert.ok(!Object.values(LINK_TEXT).some((t) => /Recovering/.test(t.word)));
});

// ---------------------------------------------------------------- Lab API

test('Lab API: every error kind in ERROR_TEXT is classified or explicitly a request error', () => {
  const requestErrors = ['run_active', 'invalid_spec', 'aborted'];
  for (const kind of Object.keys(ERROR_TEXT)) {
    assert.ok(kind in LAB_KIND_CONDITION || requestErrors.includes(kind), `unclassified error kind ${kind}`);
  }
  for (const [kind, cond] of Object.entries(LAB_KIND_CONDITION)) {
    assert.ok(kind in ERROR_TEXT, `unknown kind ${kind}`);
    assert.ok(cond in LINK_TEXT, cond);
  }
});

const EXPECT_LAB = {
  network: ['Disconnected', 'page'],
  unauthorized_missing: ['Unauthorised', 'gateway'],
  unauthorized_invalid: ['Unauthorised', 'gateway'],
  unauthorized_expired: ['Unauthorised', 'gateway'],
  forbidden_role: ['Unauthorised', 'gateway'],
  forbidden_origin: ['Unauthorised', 'gateway'],
  host_guard: ['Unauthorised', 'gateway'],
  disabled: ['Unavailable', 'gateway'],
  not_found: ['Unavailable', 'gateway'],
  server: ['Unavailable', 'gateway'],
  unexpected: ['Unavailable', 'gateway'],
  bad_request: ['Unavailable', 'gateway'],
  too_large: ['Unavailable', 'gateway'],
  busy: ['Unavailable', 'gateway'],
};
for (const [kind, [word, who]] of Object.entries(EXPECT_LAB)) {
  test(`Lab API error kind ${kind} -> ${word} (${who})`, () => {
    const v = describeLabLink({ status: 'error', data: null, error: { kind, status: kind === 'server' ? 502 : 0 } });
    assert.equal(v.word, word);
    assert.equal(v.who, who);
    if (v.retry) assert.equal(v.retryLabel, RETRY_LABEL);
    if (kind !== 'network') assert.equal(v.fix, ERROR_TEXT[kind].fix);
    assert.ok(!/Not reachable or not authorised/.test(v.text), 'the merged wording is gone');
  });
}

test('Lab API: loading = Checking, ready = Live, idle/aborted = not a link, request errors (409, 422) do not make the link bad', () => {
  assert.equal(describeLabLink({ status: 'loading' }).word, 'Checking');
  assert.equal(describeLabLink({ status: 'ready', data: {} }).word, 'Live');
  assert.equal(describeLabLink({ status: 'idle' }).isLink, false);
  assert.equal(describeLabLink({ status: 'error', error: { kind: 'aborted' } }).isLink, false);
  assert.equal(describeLabLink({ status: 'error', data: null, error: { kind: 'run_active', status: 409 } }).word, 'Live');
  assert.equal(describeLabLink({ status: 'error', data: { x: 1 }, error: { kind: 'network' } }).hasStaleData, true);
});

test('Lab 5xx: carries the HTTP status in words and says "gateway or a proxy" (the page cannot tell which)', () => {
  const v = describeLabLink({ status: 'error', data: null, error: { kind: 'server', status: 502 } });
  assert.match(v.detail, /HTTP 502/);
  assert.match(v.cause, /gateway, or a proxy/);
});

// ---------------------------------------------------------------- run stream

function streamAfter(failures, { kind = 'network', now0 = T0 } = {}) {
  let s = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: now0 });
  s = streamReducer(s, { type: 'frame', id: 1, now: now0 });
  for (let i = 0; i < failures; i += 1) s = streamReducer(s, { type: 'error', error: { kind }, now: now0 + 100 * i });
  return s;
}

test('run stream: waiting after 1-2 failures = Reconnecting with a countdown; from 3 = Disconnected; retryAt is recorded by the reducer', () => {
  const s1 = streamAfter(1);
  assert.equal(s1.retryAt, T0 + 1000, 'first backoff is 1 s after the error time');
  const v1 = describeRunStreamLink(s1, deriveStreamStatus(s1, T0 + 10), T0 + 500);
  assert.equal(v1.word, 'Reconnecting');
  assert.equal(v1.nextRetryInMs, 500);
  assert.match(v1.detail, /1 failed attempt\./);
  assert.match(v1.detail, /Next try in 1 s/);
  const s3 = streamAfter(3);
  const v3 = describeRunStreamLink(s3, deriveStreamStatus(s3, T0 + 10), T0 + 10);
  assert.equal(v3.word, 'Disconnected');
  assert.equal(v3.changeAddress, true);
  assert.equal(v3.retryOwner, 'page');
});

test('run stream: stale, live, connecting, ended, idle, and terminal failure by cause', () => {
  let s = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  assert.equal(describeRunStreamLink(s, deriveStreamStatus(s, 1000), 1000).word, 'Live');
  const stale = describeRunStreamLink(s, deriveStreamStatus(s, 30_000), 30_000);
  assert.equal(stale.word, 'Stale');
  assert.match(stale.basis, /15 s/);
  assert.equal(describeRunStreamLink(initialStream, 'idle', 0).isLink, false);
  assert.equal(describeRunStreamLink(streamReducer(initialStream, { type: 'connect' }), 'connecting', 0).word, 'Connecting');
  const ended = streamReducer(s, { type: 'done', state: 'completed', now: 5 });
  const ev = describeRunStreamLink(ended, 'ended', 6);
  assert.equal(ev.isLink, false);
  assert.equal(ev.word, 'Run ended');
  const authFail = streamReducer(s, { type: 'error', error: { kind: 'unauthorized_expired', status: 401 }, now: 7 });
  assert.equal(authFail.phase, 'failed');
  const fv = describeRunStreamLink(authFail, 'failed', 8);
  assert.equal(fv.word, 'Unauthorised');
  assert.equal(fv.retry, false, 'retrying will not help until the cause is fixed');
  const offFail = streamReducer(s, { type: 'error', error: { kind: 'not_found', status: 404 }, now: 7 });
  assert.equal(describeRunStreamLink(offFail, 'failed', 8).word, 'Unavailable');
});

// ---------------------------------------------------------------- one vocabulary

test('same condition, same word and same cause on every surface (telemetry, Lab API, run stream)', () => {
  const tel = describeTelemetryLink(errors(initialConnection, 3), T0 + 5);
  const lab = describeLabLink({ status: 'error', data: null, error: { kind: 'network' } });
  const run = (() => { const s = streamAfter(3); return describeRunStreamLink(s, deriveStreamStatus(s, T0), T0); })();
  for (const v of [tel, lab, run]) {
    assert.equal(v.word, 'Disconnected');
    assert.equal(v.condition, 'no_answer');
    assert.equal(v.cause, LINK_TEXT.no_answer.cause);
    assert.equal(v.tone, 'critical');
    assert.equal(v.retryLabel, RETRY_LABEL);
  }
  const reTel = describeTelemetryLink(errors(initialConnection, 1), T0 + 5);
  const s1 = streamAfter(1);
  const reRun = describeRunStreamLink(s1, deriveStreamStatus(s1, T0), T0);
  assert.equal(reTel.cause, reRun.cause);
  assert.equal(reTel.word, reRun.word);
  const staleTel = describeTelemetryLink(msg(initialConnection, frame(), T0), T0 + 9000);
  const staleRun = describeRunStreamLink(streamReducer(streamReducer(initialStream, { type: 'opened', now: 0 }), { type: 'activity', now: 0 }), 'stale', 99_000);
  assert.equal(staleTel.cause, staleRun.cause);
});

test('every link view uses a word from the vocabulary and the single retry verb; no view uses the retired phrases', () => {
  const retired = [/Reconnect now/, /Not reachable or not authorised/, /No connection to the gateway/, /Could not reach the gateway/];
  const views = [];
  for (const n of [0, 1, 2, 3]) views.push(describeTelemetryLink(errors(initialConnection, n), T0 + 5));
  for (const kind of Object.keys(EXPECT_LAB)) views.push(describeLabLink({ status: 'error', data: null, error: { kind, status: 0 } }));
  for (const f of [0, 1, 3]) { const s = streamAfter(f); views.push(describeRunStreamLink(s, deriveStreamStatus(s, T0), T0)); }
  const allowed = new Set([...Object.values(LINK_WORDS), 'Run ended', 'Not watching']);
  for (const v of views) {
    assert.ok(allowed.has(v.word), v.word);
    if (v.retryLabel !== null) assert.equal(v.retryLabel, RETRY_LABEL);
    for (const re of retired) assert.doesNotMatch(v.text + (v.fix || ''), re);
    assert.equal(v.text.startsWith(v.word), true);
  }
});

test('worstLink picks by severity (Disconnected > Unauthorised > Reconnecting > Unavailable > Stale > Degraded > Live); non-link views never win', () => {
  assert.deepEqual([...SEVERITY_ORDER], ['disconnected', 'unauthorised', 'reconnecting', 'unavailable', 'stale', 'degraded', 'connecting', 'checking', 'live']);
  const live = describeTelemetryLink(msg(initialConnection, frame(), T0), T0 + 5);
  const auth = describeLabLink({ status: 'error', data: null, error: { kind: 'unauthorized_missing', status: 401 } });
  const off = describeLabLink({ status: 'error', data: null, error: { kind: 'disabled', status: 404 } });
  const down = describeTelemetryLink(errors(initialConnection, 3), T0 + 5);
  assert.equal(worstLink([live, auth]).word, 'Unauthorised');
  assert.equal(worstLink([auth, off]).word, 'Unauthorised');
  assert.equal(worstLink([live, auth, down]).word, 'Disconnected');
  assert.equal(worstLink([live, off]).word, 'Unavailable');
  assert.equal(worstLink([describeRunStreamLink(initialStream, 'idle', 0), live]).word, 'Live');
  assert.equal(worstLink([]), null);
});

test('describeLink aggregates the three channels and reports the word list', () => {
  const out = describeLink({
    telemetry: { state: msg(initialConnection, frame(), T0), now: T0 + 5 },
    caps: { status: 'error', data: null, error: { kind: 'unauthorized_missing', status: 401 } },
    runStream: { stream: streamAfter(0), status: 'live', now: T0 + 5 },
    security: 'jwt',
  });
  assert.deepEqual(out.words, ['Live', 'Unauthorised', 'Live']);
  assert.equal(out.worst.word, 'Unauthorised');
  const none = describeLink({});
  assert.equal(none.worst, null);
  assert.deepEqual(none.words, []);
});

test('views are frozen client observations: they cannot be mutated by a screen', () => {
  const v = describeTelemetryLink(initialConnection, 0);
  assert.throws(() => { 'use strict'; v.word = 'Live'; }, TypeError);
});
