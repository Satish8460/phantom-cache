// Connection state machine for the SSE stream. Pure: the caller supplies `now`.
import { parseSchema2 } from './schema.js';

export const STALE_AFTER_MS = 3000; // 30 missed 100 ms ticks
export const BACKOFF_BASE_MS = 1000;
export const BACKOFF_MAX_MS = 30000;

export const initialConnection = Object.freeze({
  phase: 'connecting', // connecting | open | error | closed
  payload: null,
  receivedAt: null, // client clock, ms
  violations: [],
  parseError: null, // {code, detail} of the latest bad frame, cleared by next good frame
  badFrames: 0,
  messages: 0,
  attempt: 0,
  lastError: null, // {closed, at} of the latest stream error; cleared by the next good frame (client observation)
  retryAt: null, // client clock ms at which THIS PAGE will reopen the stream; null when the browser owns the retry
});

export function connectionReducer(state, action) {
  switch (action.type) {
    case 'connecting':
      return { ...state, phase: 'connecting', retryAt: null };
    case 'reset':
      // The gateway address changed: nothing received from the old gateway may stay on screen (F4/F3).
      return initialConnection;
    case 'retryScheduled':
      return { ...state, retryAt: Number.isFinite(action.at) ? action.at : null };
    case 'open':
      return { ...state, phase: 'open' };
    case 'error':
      // readyState 2 (CLOSED) means the browser will not retry by itself.
      return { ...state, phase: action.closed ? 'closed' : 'error', attempt: state.attempt + 1, lastError: { closed: Boolean(action.closed), at: Number.isFinite(action.now) ? action.now : null } };
    case 'message': {
      const parsed = parseSchema2(action.text);
      if (!parsed.ok) {
        return { ...state, phase: 'open', parseError: { code: parsed.code, detail: parsed.detail }, badFrames: state.badFrames + 1 };
      }
      return {
        ...state,
        phase: 'open',
        payload: parsed.payload,
        receivedAt: action.now,
        violations: parsed.violations,
        parseError: null,
        messages: state.messages + 1,
        attempt: 0,
        lastError: null,
        retryAt: null,
      };
    }
    default:
      return state;
  }
}

/**
 * kind:
 *   loading      - nothing received yet
 *   live         - fresh, contract-clean data
 *   stale        - stream believed open but no frame for STALE_AFTER_MS
 *   disconnected - stream is down (last data, if any, is shown as stale)
 *   error        - frames arrive but cannot be understood and no good frame was ever seen
 *   degraded     - data is flowing but contains contract violations or a bad frame
 */
export function deriveConnection(state, now, staleAfterMs = STALE_AFTER_MS) {
  const ageMs = state.receivedAt === null ? null : Math.max(0, now - state.receivedAt);
  const down = state.phase === 'error' || state.phase === 'closed';
  let kind;
  if (!state.payload) {
    if (down) kind = 'disconnected';
    else if (state.parseError) kind = 'error';
    else kind = 'loading';
  } else if (down) kind = 'disconnected';
  else if (ageMs > staleAfterMs) kind = 'stale';
  else if (state.violations.length > 0 || state.parseError) kind = 'degraded';
  else kind = 'live';
  return { kind, ageMs, hasData: Boolean(state.payload) };
}

export function backoffMs(attempt) {
  const a = Math.max(0, Math.min(attempt, 10));
  return Math.min(BACKOFF_MAX_MS, BACKOFF_BASE_MS * 2 ** a);
}

/** Client minus server wall clock at receipt. Includes network delay; informational only (inferred). */
export function clockOffsetMs(payload, receivedAt) {
  if (!payload || !Number.isFinite(payload.as_of_ms) || receivedAt === null) return null;
  return receivedAt - payload.as_of_ms;
}

export const CONNECTION_TEXT = Object.freeze({
  loading: { title: 'Connecting to the gateway', detail: 'Waiting for the first telemetry frame. No values are shown until data arrives.' },
  live: { title: 'Live', detail: 'Receiving telemetry.' },
  stale: { title: 'Data is stale', detail: 'No telemetry frame has arrived recently. Values shown are the last received and may be out of date.' },
  disconnected: { title: 'Disconnected', detail: 'The telemetry stream is down. The browser will retry; any values shown are the last received.' },
  error: { title: 'Telemetry cannot be read', detail: 'The gateway is answering, but its frames are not valid schema 2.' },
  degraded: { title: 'Degraded data', detail: 'Telemetry is flowing but some fields broke the schema contract and were withheld.' },
});
