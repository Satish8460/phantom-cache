import test from 'node:test';
import assert from 'node:assert/strict';
import { createSseParser, parseEventData, parseSseText } from '../src/lib/sse.js';
import {
  BACKOFF_MAX_MS, applyRunEvent, backoffMs, deriveStreamStatus, initialRunModel, initialStream, runStream, streamReducer,
} from '../src/lib/labStream.js';
import { FX, chunkText, fakeResponse, fakeStream } from './helpers/labFixtures.mjs';

// ---------------------------------------------------------------- SSE parser

test('sse: the recorded T1 stream parses into 7 numbered events with JSON data', () => {
  const evs = parseSseText(FX.sseT1());
  assert.deepEqual(evs.map((e) => e.id), ['1', '2', '3', '4', '5', '6', '7']);
  assert.deepEqual(evs.map((e) => e.event), ['state', 'state', 'progress', 'progress', 'result', 'state', 'done']);
  assert.equal(parseEventData(evs[0]).value.state, 'queued');
  assert.equal(parseEventData(evs[6]).value.state, 'completed');
});

test('sse: every split of the recorded stream yields exactly the same events (partial chunks)', () => {
  const text = FX.sseSb();
  const whole = parseSseText(text);
  assert.ok(whole.length >= 19);
  for (const sizes of [[1], [2], [3, 1], [7], [13, 5, 2], [64], [1000]]) {
    const p = createSseParser();
    const got = [];
    for (const c of chunkText(text, sizes)) got.push(...p.push(c).filter((x) => x.type === 'event'));
    assert.deepEqual(got, whole, `chunk sizes ${sizes}`);
  }
});

test('sse: CRLF, lone CR and LF line endings are equivalent, including CR and LF split across chunks', () => {
  const lf = 'id: 5\nevent: progress\ndata: {"a":1}\n\n';
  const crlf = lf.replace(/\n/g, '\r\n');
  const cr = lf.replace(/\n/g, '\r');
  const ref = parseSseText(lf);
  assert.equal(ref.length, 1);
  assert.deepEqual(parseSseText(crlf), ref);
  assert.deepEqual(parseSseText(cr + 'x'), ref, 'a lone CR ends a line (the trailing x keeps the stream open)');
  const p = createSseParser();
  const out = [];
  for (const c of ['id: 5\r', '\nevent: progress\r', '\ndata: {"a":1}\r', '\n\r', '\n']) out.push(...p.push(c));
  assert.deepEqual(out.filter((x) => x.type === 'event'), ref);
});

test('sse: comments and heartbeats are reported separately and never become events', () => {
  const p = createSseParser();
  const items = p.push(':heartbeat\n\n: another comment\nid: 2\nevent: state\ndata: {"state":"running"}\n\n:heartbeat\n\n');
  assert.deepEqual(items.map((i) => i.type), ['comment', 'comment', 'event', 'comment']);
  assert.equal(items[0].text, 'heartbeat');
  assert.equal(items[2].event, 'state');
});

test('sse: multi-line data joins with newlines; missing event name defaults to "message"; retry and bad ids follow the standard', () => {
  const evs = parseSseText('id: 9\ndata: line one\ndata: line two\nretry: 1500\n\nid: bad\u0000id\ndata: x\n\nretry: abc\ndata: y\n\n');
  assert.equal(evs[0].data, 'line one\nline two');
  assert.equal(evs[0].event, 'message');
  assert.equal(evs[0].retry, 1500);
  assert.equal(evs[1].id, '9', 'an id containing NUL is ignored, the previous id stays');
  assert.equal(evs[2].retry, null, 'a non-numeric retry is ignored');
});

test('sse: a leading BOM is ignored; an event with no data is not dispatched; a field without colon has an empty value', () => {
  const evs = parseSseText('﻿id: 1\ndata: {}\n\nevent: nodata\n\nid\ndata: z\n\n');
  assert.equal(evs.length, 2);
  assert.equal(evs[0].id, '1');
  assert.equal(evs[1].id, '', 'an `id` line with no value resets the id');
});

test('sse: malformed JSON data is reported, not thrown', () => {
  const [ev] = parseSseText('id: 1\nevent: progress\ndata: {not json\n\n');
  assert.deepEqual(parseEventData(ev), { ok: false, value: null });
});

// ---------------------------------------------------------------- reducer

const T = 1_000;

test('reconnect: backoff doubles from 1 s and is capped at 30 s, deterministically', () => {
  assert.deepEqual([1, 2, 3, 4, 5, 6].map(backoffMs), [1000, 2000, 4000, 8000, 16000, 30000]);
  assert.equal(backoffMs(50), BACKOFF_MAX_MS);
  assert.equal(backoffMs(0), 1000);
  assert.equal(backoffMs(3), backoffMs(3));
});

test('reducer: connect -> open -> frames track lastEventId, drop replayed duplicates and count gaps', () => {
  let s = initialStream;
  s = streamReducer(s, { type: 'connect' });
  assert.equal(deriveStreamStatus(s, T), 'connecting');
  s = streamReducer(s, { type: 'opened', now: T });
  assert.equal(deriveStreamStatus(s, T), 'live');
  for (const id of [1, 2, 3]) s = streamReducer(s, { type: 'frame', id, now: T });
  assert.equal(s.lastEventId, 3);
  assert.equal(s.gaps, 0);
  const dup = streamReducer(s, { type: 'frame', id: 2, now: T + 5 });
  assert.equal(dup.lastEventId, 3);
  assert.equal(dup.eventCount, s.eventCount, 'duplicate not counted');
  const gap = streamReducer(s, { type: 'frame', id: 6, now: T });
  assert.equal(gap.gaps, 1);
  assert.equal(gap.lastEventId, 6);
  const firstGap = streamReducer(initialStream, { type: 'frame', id: 4, now: T });
  assert.equal(firstGap.gaps, 1, 'the log should start at 1');
});

test('reducer: silence beyond the heartbeat window is "stale"; any bytes (heartbeat) clear it', () => {
  let s = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  assert.equal(deriveStreamStatus(s, 10_000), 'live');
  assert.equal(deriveStreamStatus(s, 30_000), 'stale');
  s = streamReducer(s, { type: 'activity', now: 30_000 });
  assert.equal(deriveStreamStatus(s, 31_000), 'live');
  assert.equal(deriveStreamStatus(s, 31_000, { staleAfterMs: 500 }), 'stale', 'threshold is configurable');
});

test('reducer: a dropped stream backs off (reconnecting, then disconnected), and recovery resets the attempts', () => {
  let s = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  s = streamReducer(s, { type: 'frame', id: 1, now: 0 });
  s = streamReducer(s, { type: 'closed' });
  assert.equal(s.phase, 'waiting');
  assert.equal(s.attempt, 1);
  assert.equal(s.nextDelayMs, 1000);
  assert.equal(deriveStreamStatus(s, 0), 'reconnecting');
  s = streamReducer(s, { type: 'connect' });
  assert.equal(deriveStreamStatus(s, 0), 'reconnecting');
  s = streamReducer(s, { type: 'error', error: { kind: 'network' } });
  s = streamReducer(s, { type: 'error', error: { kind: 'server' } });
  assert.equal(s.attempt, 3);
  assert.equal(s.nextDelayMs, 4000);
  assert.equal(deriveStreamStatus(s, 0), 'disconnected');
  assert.equal(s.lastEventId, 1, 'resume point survives the outage');
  s = streamReducer(s, { type: 'connect' });
  s = streamReducer(s, { type: 'opened', now: 5 });
  assert.equal(s.attempt, 3, 'opening alone does not reset the backoff');
  s = streamReducer(s, { type: 'frame', id: 2, now: 6 });
  assert.equal(s.attempt, 0);
});

test('reducer: refusals that retrying cannot fix end in "failed" with the reason; done ends the stream', () => {
  const base = streamReducer(initialStream, { type: 'connect' });
  for (const kind of ['unauthorized_missing', 'unauthorized_expired', 'forbidden_role', 'not_found', 'disabled']) {
    const s = streamReducer(base, { type: 'error', error: { kind, title: 't' } });
    assert.equal(deriveStreamStatus(s, 0), 'failed', kind);
    assert.equal(s.error.kind, kind);
  }
  const done = streamReducer(streamReducer(base, { type: 'opened', now: 0 }), { type: 'done', state: 'cancelled', now: 1 });
  assert.equal(deriveStreamStatus(done, 99_999), 'ended', 'an ended stream is never stale');
  assert.equal(done.endedWith, 'cancelled');
  assert.equal(deriveStreamStatus(streamReducer(done, { type: 'stop' }), 0), 'ended');
});

// ---------------------------------------------------------------- run model

test('run model: events from the recorded Scenario B run build state, progress, result and done', () => {
  let m = initialRunModel;
  for (const ev of parseSseText(FX.sseSb())) {
    const p = parseEventData(ev);
    m = applyRunEvent(m, { id: Number(ev.id), event: ev.event, data: p.value, malformed: !p.ok });
  }
  assert.equal(m.state, 'completed');
  assert.deepEqual(m.done, { state: 'completed', exit_code: 0, source: null });
  assert.equal(m.progress.phase, 'run-complete');
  assert.equal(m.progress.done, 2);
  assert.equal(m.progress.total, 2);
  assert.equal(m.result.label, 'EXPLORATORY');
  assert.equal(m.log.length, 19);
  assert.equal(m.log[0].summary, 'state: queued');
  assert.equal(m.malformed, 0);
});

test('run model: cancel path shows cancelling then cancelled with exit code from the gateway', () => {
  let m = initialRunModel;
  const states = [];
  for (const ev of parseSseText(FX.sseCancelled())) {
    const p = parseEventData(ev);
    m = applyRunEvent(m, { id: Number(ev.id), event: ev.event, data: p.value, malformed: !p.ok });
    if (ev.event === 'state') states.push(m.state);
  }
  assert.deepEqual(states, ['queued', 'running', 'cancelling', 'cancelled']);
  assert.equal(m.done.exit_code, 130);
  assert.equal(m.result, null, 'a cancelled run reports no result event');
});

test('run model: unknown events are logged only; malformed and non-object data are counted, never applied; recovery marks are flagged', () => {
  let m = applyRunEvent(initialRunModel, { id: 1, event: 'novel', data: { x: 1 }, malformed: false });
  assert.equal(m.state, null);
  assert.equal(m.log.length, 1);
  m = applyRunEvent(m, { id: 2, event: 'state', data: null, malformed: true });
  m = applyRunEvent(m, { id: 3, event: 'progress', data: [1], malformed: false });
  assert.equal(m.malformed, 2);
  m = applyRunEvent(m, { id: 4, event: 'state', data: { state: 'interrupted', source: 'server_recovery_on_startup' }, malformed: false });
  assert.equal(m.recovery, true);
  assert.equal(m.state, 'interrupted');
  m = applyRunEvent(m, { id: 5, event: 'error', data: { code: 'runner_panic', message: 'boom' }, malformed: false });
  assert.deepEqual(m.error, { code: 'runner_panic', message: 'boom' });
  let capped = initialRunModel;
  for (let i = 1; i <= 80; i += 1) capped = applyRunEvent(capped, { id: i, event: 'state', data: { state: 'running' }, malformed: false });
  assert.equal(capped.log.length, 50);
  assert.equal(capped.log[0].id, 31);
});

// ---------------------------------------------------------------- runner

function harness(responses, { token = 'tok' } = {}) {
  const sleeps = [];
  const opened = [];
  const actions = [];
  const events = [];
  let state = initialStream;
  const dispatch = (a) => {
    actions.push(a);
    state = streamReducer(state, a);
  };
  const client = {
    async openEvents(runId, { lastEventId }) {
      opened.push({ runId, lastEventId, token });
      const r = responses.shift();
      if (!r) throw new Error('unexpected extra connection');
      return r;
    },
  };
  const ctl = new AbortController();
  const run = () => runStream({
    client, runId: 'run1', signal: ctl.signal, dispatch,
    onEvent: (e) => events.push(e),
    getLastEventId: () => state.lastEventId,
    sleep: async (ms) => { sleeps.push(ms); },
    now: () => 5,
  });
  return { run, sleeps, opened, actions, events, ctl, get state() { return state; } };
}

const body = (text, sizes = [9]) => ({ ok: true, body: fakeStream(chunkText(text, sizes)) });

test('runner: reads a whole recorded stream through awkward chunking and ends at `done` without reconnecting', async () => {
  const h = harness([body(FX.sseSb(), [5, 11, 3])]);
  const outcome = await h.run();
  assert.equal(outcome, 'ended');
  assert.equal(h.events.length, 19);
  assert.equal(h.state.phase, 'ended');
  assert.equal(h.state.endedWith, 'completed');
  assert.equal(h.opened.length, 1);
  assert.equal(h.opened[0].lastEventId, null);
  assert.deepEqual(h.sleeps, []);
});

test('runner: a drop mid-run reconnects after the backoff with Last-Event-ID and does not re-apply events', async () => {
  const text = FX.sseSb();
  const evs = parseSseText(text);
  const firstHalf = evs.slice(0, 8).map((e) => `id: ${e.id}\nevent: ${e.event}\ndata: ${e.data}\n\n`).join('');
  const rest = evs.slice(8).map((e) => `id: ${e.id}\nevent: ${e.event}\ndata: ${e.data}\n\n`).join('');
  // the server replays event 8 again on resume (a duplicate), then continues
  const replay = `id: 8\nevent: ${evs[7].event}\ndata: ${evs[7].data}\n\n${rest}`;
  const h = harness([body(firstHalf), body(replay)]);
  const outcome = await h.run();
  assert.equal(outcome, 'ended');
  assert.equal(h.opened.length, 2);
  assert.equal(h.opened[1].lastEventId, 8, 'resumes from the last event received');
  assert.deepEqual(h.sleeps, [1000]);
  assert.equal(h.state.eventCount, 19, 'every event counted once');
  assert.equal(h.state.gaps, 0);
  const ids = h.events.map((e) => e.id);
  assert.equal(ids.length, 20, 'onEvent sees the duplicate; the reducer drops it');
});

test('runner: repeated failures lengthen the delay; success resets it', async () => {
  const err = { ok: false, error: { kind: 'network', title: 'x' } };
  const first = 'id: 1\nevent: state\ndata: {"state":"queued"}\n\n';
  const final = 'id: 2\nevent: done\ndata: {"state":"completed","exit_code":0}\n\n';
  const h = harness([err, err, err, body(first), err, body(final)]);
  await h.run();
  // After data flowed the counter restarts: the close that ends the short stream is failure 1 (1 s), the next refusal failure 2 (2 s).
  assert.deepEqual(h.sleeps, [1000, 2000, 4000, 1000, 2000]);
  assert.equal(h.state.phase, 'ended');
});

test('runner: a stream that errors in the middle is treated as a drop, not as the end of the run', async () => {
  const first = 'id: 1\nevent: state\ndata: {"state":"running"}\n\n';
  const final = 'id: 2\nevent: done\ndata: {"state":"failed","exit_code":1}\n\n';
  const h = harness([{ ok: true, body: fakeStream([first], { errorAfter: 'connection reset' }) }, body(final)]);
  const outcome = await h.run();
  assert.equal(outcome, 'ended');
  assert.equal(h.state.endedWith, 'failed');
  assert.deepEqual(h.sleeps, [1000]);
  assert.equal(h.opened[1].lastEventId, 1);
});

test('runner: a stream closed without `done` is not an ended run (it reconnects)', async () => {
  const first = 'id: 1\nevent: state\ndata: {"state":"running"}\n\n';
  const final = 'id: 2\nevent: done\ndata: {"state":"completed","exit_code":0}\n\n';
  const h = harness([body(first), body(final)]);
  await h.run();
  assert.equal(h.opened.length, 2);
  assert.ok(h.actions.some((a) => a.type === 'closed'));
});

test('runner: terminal refusals stop immediately with the mapped error and no further attempts', async () => {
  for (const kind of ['unauthorized_missing', 'forbidden_role', 'not_found']) {
    const h = harness([{ ok: false, error: { kind, title: 'refused' } }]);
    assert.equal(await h.run(), 'failed');
    assert.equal(h.state.phase, 'failed');
    assert.equal(h.state.error.kind, kind);
    assert.deepEqual(h.sleeps, []);
    assert.equal(h.opened.length, 1);
  }
});

test('runner: abort stops without another connection; heartbeats alone keep the stream live', async () => {
  const h = harness([]);
  h.ctl.abort();
  assert.equal(await h.run(), 'stopped');
  assert.equal(h.opened.length, 0);
  const hb = harness([body(':heartbeat\n\n:heartbeat\n\nid: 1\nevent: done\ndata: {"state":"completed"}\n\n', [4])]);
  await hb.run();
  assert.ok(hb.actions.filter((a) => a.type === 'activity').length >= 3);
});

test('runner never puts the token anywhere but the injected client (the loop itself has no credentials)', async () => {
  const fs = await import('node:fs');
  const text = fs.readFileSync(new URL('../src/lib/labStream.js', import.meta.url), 'utf8');
  const code = text.split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');
  assert.doesNotMatch(code, /Authorization|Bearer|token/i);
  const res = fakeResponse({ stream: fakeStream([]) });
  assert.ok(res);
});
