// Live run stream: reconnect state machine, runner loop and the run model built from events.
// No React, no timers of its own (sleep and now are injected), so every transition is unit tested.
//
// Design (docs/design/sse_auth_options.md option D): fetch streaming with an Authorization header, resume with a
// Last-Event-ID header, deterministic exponential backoff (no jitter: the page has a single client per run view and
// the code base forbids random numbers in UI source), and honest states when the stream is silent or down.

import { createSseParser, parseEventData } from './sse.js';
import { isTerminalError } from './labApi.js';

export const BACKOFF_BASE_MS = 1000;
export const BACKOFF_MAX_MS = 30000;
/** The gateway sends a heartbeat comment every 15 s; the stream is "stale" after this many silent milliseconds. */
export const DEFAULT_STALE_AFTER_MS = 25000;
export const TERMINAL_RUN_STATES = Object.freeze(['completed', 'failed', 'cancelled', 'interrupted']);

export function isTerminalRunState(state) {
  return TERMINAL_RUN_STATES.includes(state);
}

/** delay before reconnect attempt number `attempt` (1-based): 1 s, 2 s, 4 s ... capped at 30 s. */
export function backoffMs(attempt) {
  const n = Math.max(1, Math.floor(attempt));
  const exp = Math.min(n - 1, 10);
  return Math.min(BACKOFF_MAX_MS, BACKOFF_BASE_MS * 2 ** exp);
}

export const initialStream = Object.freeze({
  phase: 'idle', // idle | connecting | open | waiting | ended | failed
  lastEventId: null, // number of the last event applied (events are numbered 1.. without gaps)
  attempt: 0,
  nextDelayMs: null,
  lastActivityAt: null,
  openedAt: null,
  error: null,
  retryAt: null, // client clock ms of the next reconnect attempt, when the caller supplied `now` with the error
  endedWith: null, // terminal run state from the `done` event
  gaps: 0,
  eventCount: 0,
});

export function streamReducer(state, action) {
  switch (action.type) {
    case 'connect':
      return { ...state, phase: 'connecting', error: null, nextDelayMs: null };
    case 'opened':
      // The attempt counter resets only once data flows again, so a server that accepts and immediately drops
      // does not defeat the backoff.
      return { ...state, phase: 'open', openedAt: action.now, lastActivityAt: action.now, error: null };
    case 'activity':
      return { ...state, lastActivityAt: action.now };
    case 'frame': {
      const id = action.id;
      let gaps = state.gaps;
      let lastEventId = state.lastEventId;
      if (Number.isInteger(id)) {
        if (lastEventId !== null && id <= lastEventId) {
          return { ...state, lastActivityAt: action.now }; // replayed duplicate: ignore
        }
        if (lastEventId !== null && id > lastEventId + 1) gaps += 1;
        if (lastEventId === null && id > 1) gaps += 1;
        lastEventId = id;
      }
      return { ...state, lastEventId, gaps, lastActivityAt: action.now, attempt: 0, eventCount: state.eventCount + 1 };
    }
    case 'done':
      return { ...state, phase: 'ended', endedWith: action.state ?? null, lastActivityAt: action.now, nextDelayMs: null };
    case 'closed': // the connection ended without a `done` event
    case 'error': {
      const err = action.type === 'error' ? action.error : { kind: 'network', message: 'stream closed before the run ended' };
      if (action.type === 'error' && err && isTerminalError(err.kind)) {
        return { ...state, phase: 'failed', error: err, nextDelayMs: null };
      }
      const attempt = state.attempt + 1;
      return { ...state, phase: 'waiting', attempt, nextDelayMs: backoffMs(attempt), retryAt: Number.isFinite(action.now) ? action.now + backoffMs(attempt) : null, error: err };
    }
    case 'stop':
      return { ...state, phase: state.phase === 'ended' ? 'ended' : 'idle', nextDelayMs: null };
    default:
      return state;
  }
}

/**
 * What the person sees: connecting | live | stale | reconnecting | disconnected | ended | failed | idle.
 * 'reconnecting' = backing off after a drop (first attempts); 'disconnected' = repeated failures.
 */
export function deriveStreamStatus(state, now, { staleAfterMs = DEFAULT_STALE_AFTER_MS } = {}) {
  switch (state.phase) {
    case 'idle': return 'idle';
    case 'connecting': return state.attempt > 0 ? 'reconnecting' : 'connecting';
    case 'open':
      return state.lastActivityAt !== null && now - state.lastActivityAt > staleAfterMs ? 'stale' : 'live';
    case 'waiting': return state.attempt >= 3 ? 'disconnected' : 'reconnecting';
    case 'ended': return 'ended';
    case 'failed': return 'failed';
    default: return 'idle';
  }
}

export const STREAM_TEXT = Object.freeze({
  idle: { title: 'Not watching', detail: 'No live stream is open.' },
  connecting: { title: 'Connecting', detail: 'Opening the event stream.' },
  live: { title: 'Live', detail: 'Events are arriving from the gateway.' },
  stale: { title: 'Stale', detail: 'The stream is open but silent for longer than the heartbeat interval. Values below may be out of date.' },
  reconnecting: { title: 'Reconnecting', detail: 'The stream dropped. Resuming from the last event received.' },
  disconnected: { title: 'Disconnected', detail: 'The stream is down and reconnection keeps failing. Values below are the last received.' },
  ended: { title: 'Run ended', detail: 'The gateway sent the final event for this run.' },
  failed: { title: 'Stream cannot continue', detail: 'The gateway refused the stream; retrying will not help until the cause is fixed.' },
});

async function* readText(body, signal) {
  const reader = body.getReader();
  const decoder = new TextDecoder('utf-8');
  try {
    for (;;) {
      if (signal && signal.aborted) return;
      const { value, done } = await reader.read();
      if (done) break;
      const text = typeof value === 'string' ? value : decoder.decode(value, { stream: true });
      yield text;
    }
    const tail = decoder.decode();
    if (tail) yield tail;
  } finally {
    try {
      await reader.cancel();
    } catch {
      /* already closed */
    }
  }
}

/**
 * Runs until the run ends, a terminal error occurs, or `signal` aborts.
 * dispatch(action) receives reducer actions; onEvent({id, event, data}) receives each NEW parsed event once.
 * getLastEventId() returns the id to resume from (kept by the caller's reducer state).
 */
export async function runStream({ client, runId, signal, dispatch, onEvent, getLastEventId, sleep, now, shouldContinue = () => true }) {
  let attempt = 0; // consecutive failures since the last event; mirrors the reducer
  for (;;) {
    if (signal.aborted || !shouldContinue()) {
      dispatch({ type: 'stop' });
      return 'stopped';
    }
    dispatch({ type: 'connect' });
    const res = await client.openEvents(runId, { lastEventId: getLastEventId(), signal });
    if (signal.aborted) {
      dispatch({ type: 'stop' });
      return 'stopped';
    }
    if (!res.ok) {
      dispatch({ type: 'error', error: res.error, now: now() });
      if (isTerminalError(res.error.kind)) return 'failed';
      attempt += 1;
    } else {
      dispatch({ type: 'opened', now: now() });
      const parser = createSseParser();
      let ended = false;
      try {
        for await (const text of readText(res.body, signal)) {
          dispatch({ type: 'activity', now: now() });
          for (const item of parser.push(text)) {
            if (item.type !== 'event') continue;
            const idNum = /^[0-9]+$/.test(item.id) ? Number(item.id) : null;
            const parsed = parseEventData(item);
            attempt = 0;
            dispatch({ type: 'frame', id: idNum, now: now() });
            onEvent({ id: idNum, event: item.event, data: parsed.ok ? parsed.value : null, malformed: !parsed.ok });
            if (item.event === 'done') {
              ended = true;
              dispatch({ type: 'done', state: parsed.ok && parsed.value ? parsed.value.state ?? null : null, now: now() });
            }
          }
          if (ended) break;
        }
      } catch (e) {
        if (signal.aborted) {
          dispatch({ type: 'stop' });
          return 'stopped';
        }
        dispatch({ type: 'error', error: { kind: 'network', message: String(e && e.message ? e.message : e) }, now: now() });
      }
      if (ended) return 'ended';
      if (signal.aborted) {
        dispatch({ type: 'stop' });
        return 'stopped';
      }
      dispatch({ type: 'closed', now: now() });
      attempt += 1;
    }
    await sleep(backoffMs(attempt), signal);
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Run model: what the run detail page shows, built only from events the gateway sent.

export const initialRunModel = Object.freeze({
  state: null,
  stateTier: null,
  progress: null, // { done, total, arm, rep, phase, tier }
  result: null,
  error: null,
  done: null, // { state, exit_code, source }
  recovery: false, // the gateway wrote interrupted marks after a restart
  malformed: 0,
  log: [], // last N event summaries for the accessible event list
});

const LOG_CAP = 50;

export function applyRunEvent(model, ev) {
  if (ev.malformed || !ev.data || typeof ev.data !== 'object' || Array.isArray(ev.data)) {
    return { ...model, malformed: model.malformed + 1 };
  }
  const d = ev.data;
  let next = { ...model };
  switch (ev.event) {
    case 'state':
      next.state = typeof d.state === 'string' ? d.state : model.state;
      if (typeof d.tier === 'string') next.stateTier = d.tier;
      if (d.source === 'server_recovery_on_startup') next.recovery = true;
      break;
    case 'progress':
      next.progress = {
        done: Number.isFinite(d.done) ? d.done : null,
        total: Number.isFinite(d.total) ? d.total : null,
        arm: typeof d.arm === 'string' ? d.arm : null,
        rep: Number.isFinite(d.rep) ? d.rep : null,
        phase: typeof d.phase === 'string' ? d.phase : null,
        tier: typeof d.tier === 'string' ? d.tier : null,
      };
      break;
    case 'result':
      next.result = d;
      break;
    case 'error':
      next.error = { code: typeof d.code === 'string' ? d.code : 'unspecified', message: typeof d.message === 'string' ? d.message : '' };
      break;
    case 'done':
      next.done = { state: d.state ?? null, exit_code: Number.isFinite(d.exit_code) ? d.exit_code : null, source: d.source ?? null };
      if (typeof d.state === 'string') next.state = d.state;
      if (d.source === 'server_recovery_on_startup') next.recovery = true;
      break;
    default:
      break; // unknown event kinds are kept in the log only
  }
  const entry = { id: ev.id, event: ev.event, summary: summarizeEvent(ev.event, d) };
  next.log = [...model.log, entry].slice(-LOG_CAP);
  return next;
}

export function summarizeEvent(kind, d) {
  switch (kind) {
    case 'state': return `state: ${d.state}`;
    case 'progress': {
      const bits = [];
      if (d.arm) bits.push(`arm ${d.arm}`);
      if (Number.isFinite(d.rep)) bits.push(`repetition ${d.rep}`);
      if (d.phase) bits.push(`phase ${d.phase}`);
      if (Number.isFinite(d.done) && Number.isFinite(d.total)) bits.push(`${d.done} of ${d.total} done`);
      return bits.length ? bits.join(', ') : 'progress';
    }
    case 'result': return 'result recorded';
    case 'error': return `error: ${d.code || 'unspecified'}`;
    case 'done': return `finished: ${d.state}`;
    default: return String(kind);
  }
}
