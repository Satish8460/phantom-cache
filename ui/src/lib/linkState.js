// One vocabulary for every connection the page has (live telemetry, Lab API, Lab run stream). Pure: callers pass `now`.
// Source of truth for the words and their texts: docs/design/remediation/06_functional_ux_work_breakdown.md section 4.
//
// Honesty rules enforced here:
//  * Every state is a CLIENT OBSERVATION ("seen by this page"), except where the gateway ANSWERED (a status code was
//    received): those are marked who = 'gateway'. `who` says which.
//  * "Unauthorised" is produced only from an answer the gateway gave: a Lab API error kind (401/403/421) or a classified
//    probe result passed in by the caller. The telemetry EventSource exposes no status code, so telemetry alone can NEVER
//    yield Unauthorised. The JWT/EventSource inference (BD-05) is an opt-in, labelled note, off by default (owner decision D-3).
//  * "Recovering" is not a state: nothing in the code distinguishes it from Reconnecting.
//  * Words are identical on every surface because every surface renders `view.word`; cause text comes from LINK_TEXT[condition].

import { STALE_AFTER_MS, deriveConnection } from './connection.js';
import { ERROR_TEXT } from './labApi.js';

export const LINK_WORDS = Object.freeze({
  live: 'Live',
  connecting: 'Connecting',
  reconnecting: 'Reconnecting',
  disconnected: 'Disconnected',
  unauthorised: 'Unauthorised',
  unavailable: 'Unavailable',
  stale: 'Stale',
  degraded: 'Degraded',
  checking: 'Checking',
});

/** The single verb for re-establishing a connection. "Refresh" is reserved for re-reading data on a healthy link. */
export const RETRY_LABEL = 'Retry now';
/** Attempts 1..(n-1) are Reconnecting; attempt n and later are Disconnected (same rule for telemetry and the run stream). */
export const DISCONNECTED_FROM_ATTEMPT = 3;

/** Worst first. Used to pick the one word a header shows when several channels disagree. */
export const SEVERITY_ORDER = Object.freeze(['disconnected', 'unauthorised', 'reconnecting', 'unavailable', 'stale', 'degraded', 'connecting', 'checking', 'live']);

/** One entry per CONDITION. `cause` is the sentence every surface shows for that condition. */
export const LINK_TEXT = Object.freeze({
  live: { word: LINK_WORDS.live, tone: 'ok', icon: 'ok', cause: 'Receiving data from the gateway.' },
  checking: { word: LINK_WORDS.checking, tone: 'info', icon: 'loading', cause: 'Asking the gateway whether the feature is available.' },
  connecting: { word: LINK_WORDS.connecting, tone: 'info', icon: 'loading', cause: 'Opening the connection. Nothing has been received yet.' },
  reconnecting: { word: LINK_WORDS.reconnecting, tone: 'warn', icon: 'warn', cause: 'This page lost the connection and is trying again. Values shown, if any, are the last received.' },
  no_answer: { word: LINK_WORDS.disconnected, tone: 'critical', icon: 'error', cause: 'This page is getting no answer from the gateway. It cannot tell a stopped gateway, a wrong address, a network fault or a blocked cross-origin request apart.' },
  refused: { word: LINK_WORDS.unauthorised, tone: 'warn', icon: 'warn', cause: 'The gateway answered and refused this request.' },
  feature_off: { word: LINK_WORDS.unavailable, tone: 'warn', icon: 'unavailable', cause: 'The gateway answered that this feature is off or not provided.' },
  gateway_error: { word: LINK_WORDS.unavailable, tone: 'warn', icon: 'unavailable', cause: 'The gateway, or a proxy in front of it, answered with an error.' },
  incompatible: { word: LINK_WORDS.unavailable, tone: 'warn', icon: 'unavailable', cause: 'The gateway answered, but not in a format this page understands (page and gateway may be different versions).' },
  stale: { word: LINK_WORDS.stale, tone: 'warn', icon: 'warn', cause: 'The connection is believed open but nothing has arrived recently. Values shown may be out of date.' },
  degraded: { word: LINK_WORDS.degraded, tone: 'warn', icon: 'warn', cause: 'Data is flowing but some fields broke the contract and were withheld.' },
  ended: { word: 'Run ended', tone: 'neutral', icon: 'info', cause: 'The gateway sent the final event for this run.' },
  idle: { word: 'Not watching', tone: 'neutral', icon: 'info', cause: 'No live connection is open.' },
});

/** Lab API / run-stream error kinds (labApi mapLabError) -> link condition. Kinds not listed are request errors, not link states. */
export const LAB_KIND_CONDITION = Object.freeze({
  network: 'no_answer',
  unauthorized_missing: 'refused',
  unauthorized_invalid: 'refused',
  unauthorized_expired: 'refused',
  forbidden_role: 'refused',
  forbidden_origin: 'refused',
  host_guard: 'refused',
  disabled: 'feature_off',
  not_found: 'feature_off',
  server: 'gateway_error',
  unexpected: 'incompatible',
  bad_request: 'incompatible',
  too_large: 'incompatible',
  busy: 'gateway_error',
});

const isNum = (x) => typeof x === 'number' && Number.isFinite(x);
const FACT_KINDS = Object.freeze({ gateway: 'The gateway answered', page: 'Seen by this page' });

function make(channel, condition, extra = {}) {
  const t = LINK_TEXT[condition];
  const who = extra.who || 'page';
  const view = {
    channel,
    condition,
    word: t.word,
    key: Object.keys(LINK_WORDS).find((k) => LINK_WORDS[k] === t.word) || (condition === 'ended' ? 'ended' : 'idle'),
    tone: t.tone,
    icon: t.icon,
    cause: t.cause,
    detail: extra.detail || null,
    who,
    whoText: FACT_KINDS[who] || FACT_KINDS.page,
    fix: extra.fix || null,
    retry: Boolean(extra.retry),
    retryLabel: extra.retry ? RETRY_LABEL : null,
    changeAddress: Boolean(extra.changeAddress),
    attempts: isNum(extra.attempts) ? extra.attempts : null,
    nextRetryAt: isNum(extra.nextRetryAt) ? extra.nextRetryAt : null,
    nextRetryInMs: isNum(extra.nextRetryInMs) ? extra.nextRetryInMs : null,
    retryOwner: extra.retryOwner || null, // 'page' (the page owns the timer: a countdown is honest) | 'browser' (it does not) | null
    lastError: extra.lastError || null,
    hasStaleData: Boolean(extra.hasStaleData),
    basis: extra.basis || null,
    notes: extra.notes || [],
    isLink: condition !== 'ended' && condition !== 'idle',
  };
  view.text = `${view.word}. ${view.cause}${view.detail ? ` ${view.detail}` : ''}`;
  return Object.freeze(view);
}

function escalate(attempts) {
  return attempts >= DISCONNECTED_FROM_ATTEMPT ? 'no_answer' : 'reconnecting';
}

function countdown(retryAt, now) {
  if (!isNum(retryAt)) return { nextRetryAt: null, nextRetryInMs: null };
  return { nextRetryAt: retryAt, nextRetryInMs: isNum(now) ? Math.max(0, retryAt - now) : null };
}

// ---------------------------------------------------------------------------------------------------------------
// Telemetry (EventSource)

/**
 * @param {object} state  connectionReducer state
 * @param {number} now    client clock ms
 * @param {{staleAfterMs?: number, probe?: {kind: string}|null, security?: string|null, includeJwtInference?: boolean}} [opts]
 *   probe: classification of ONE request made by the caller while the stream was down (labApi probe). Only a probe can make
 *   telemetry Unauthorised. security: capabilities.security_mode, used only for the opt-in inference note.
 */
export function describeTelemetryLink(state, now, opts = {}) {
  const { staleAfterMs = STALE_AFTER_MS, probe = null, security = null, includeJwtInference = false } = opts;
  const conn = deriveConnection(state, now, staleAfterMs);
  const hasStaleData = conn.hasData;
  const attempts = state.attempt;
  const owner = state.phase === 'closed' ? 'page' : state.phase === 'error' ? 'browser' : null;
  const common = { who: 'page', hasStaleData, attempts, ...countdown(state.retryAt, now), retryOwner: owner, lastError: state.lastError ? { ...state.lastError } : null };
  if (conn.kind === 'disconnected') {
    // A classified probe is an answer from the gateway: it is the only way telemetry can be Unauthorised.
    if (probe && typeof probe.kind === 'string') {
      const cond = LAB_KIND_CONDITION[probe.kind];
      if (cond === 'refused') {
        return make('telemetry', 'refused', { ...common, nextRetryAt: null, nextRetryInMs: null, retryOwner: null, who: 'gateway', detail: `${ERROR_TEXT[probe.kind].title}. The live stream is refused for the same reason${probe.withToken ? '' : ', and this page has no token to send'}.`, fix: ERROR_TEXT[probe.kind].fix, retry: true, changeAddress: false });
      }
      if (probe.kind === 'ok') {
        const base = make('telemetry', escalate(attempts), { ...common, retry: true, changeAddress: attempts >= DISCONNECTED_FROM_ATTEMPT });
        return Object.freeze({ ...base, notes: [{ kind: 'probe', text: 'The gateway answered a separate request, but the live stream is not connecting. The cause of the stream failure is unknown.' }] });
      }
    }
    const cond = escalate(attempts);
    const notes = [];
    if (includeJwtInference && security === 'jwt' && !conn.hasData) {
      notes.push({ kind: 'inference', ownerDecision: 'D-3', text: 'The Lab API on this gateway uses JWT. A browser EventSource cannot send an Authorization header, so live telemetry may be refused for that reason. This page cannot confirm it.' });
    }
    return make('telemetry', cond, {
      ...common,
      detail: `${attempts} failed ${attempts === 1 ? 'attempt' : 'attempts'}${hasStaleData ? '; the last values received are shown as stale.' : '.'}`,
      basis: owner === 'page' ? 'The browser gave up on this stream; the page schedules its own retries.' : 'The browser is retrying this stream by itself; the page cannot show a countdown.',
      retry: true,
      changeAddress: cond === 'no_answer',
      notes,
    });
  }
  if (conn.kind === 'loading') return make('telemetry', 'connecting', { ...common });
  if (conn.kind === 'error') {
    return make('telemetry', 'incompatible', { ...common, who: 'gateway', detail: 'Its telemetry frames are not valid schema 2.', retry: true });
  }
  if (conn.kind === 'stale') {
    return make('telemetry', 'stale', { ...common, detail: `No frame for ${Math.floor(conn.ageMs / 1000)} s.`, basis: `Telemetry frames are normally sent every 0.1 s; this page calls the stream stale after ${staleAfterMs / 1000} s without one.`, retry: true });
  }
  if (conn.kind === 'degraded') return make('telemetry', 'degraded', { ...common, detail: state.violations.length ? `${state.violations.length} field${state.violations.length === 1 ? '' : 's'} withheld.` : 'A frame could not be read.' });
  return make('telemetry', 'live', { ...common });
}

// ---------------------------------------------------------------------------------------------------------------
// Lab API (one-shot requests)

/**
 * @param {{status: string, data?: any, error?: {kind: string, status?: number}|null}} res  useLabResource() result
 */
export function describeLabLink(res, opts = {}) {
  if (!res || res.status === 'idle') return make('lab', 'idle');
  if (res.status === 'loading') return make('lab', 'checking', { who: 'page' });
  if (res.status === 'ready') return make('lab', 'live', { who: 'gateway' });
  const err = res.error || {};
  return describeKind('lab', err, { hasStaleData: res.data !== null && res.data !== undefined, ...opts });
}

function describeKind(channel, err, extra = {}) {
  const cond = LAB_KIND_CONDITION[err.kind];
  if (!cond) {
    if (err.kind === 'aborted') return make(channel, 'idle');
    // a request-level error that is not about the link (409 run_active, 422 invalid_spec, ...): treat the link as answering
    return make(channel, 'live', { who: 'gateway', ...extra });
  }
  const t = ERROR_TEXT[err.kind];
  const status = isNum(err.status) && err.status > 0 ? ` (HTTP ${err.status})` : '';
  if (cond === 'no_answer') return make(channel, cond, { who: 'page', retry: true, changeAddress: true, lastError: { kind: err.kind }, detail: null, ...extra });
  if (cond === 'refused') return make(channel, cond, { who: 'gateway', detail: `${t.title}${status}.`, fix: t.fix, retry: false, changeAddress: err.kind === 'forbidden_origin' || err.kind === 'host_guard', lastError: { kind: err.kind, status: err.status ?? null }, ...extra });
  if (cond === 'feature_off') return make(channel, cond, { who: 'gateway', detail: `${t.title}${status}.`, fix: t.fix, retry: false, changeAddress: err.kind === 'disabled', lastError: { kind: err.kind, status: err.status ?? null }, ...extra });
  return make(channel, cond, { who: 'gateway', detail: `${t.title}${status}.`, fix: t.fix, retry: true, lastError: { kind: err.kind, status: err.status ?? null }, ...extra });
}

// ---------------------------------------------------------------------------------------------------------------
// Lab run stream (fetch streaming)

const STREAM_STATUS_CONDITION = Object.freeze({
  idle: 'idle', connecting: 'connecting', live: 'live', stale: 'stale', reconnecting: 'reconnecting', disconnected: 'no_answer', ended: 'ended',
});

/**
 * @param {object} stream  labStream streamReducer state
 * @param {string} status  deriveStreamStatus(stream, now)
 */
export function describeRunStreamLink(stream, status, now, opts = {}) {
  const attempts = stream.attempt;
  const cd = countdown(stream.retryAt, now);
  const nextInMs = cd.nextRetryInMs !== null ? cd.nextRetryInMs : null;
  const common = { who: 'page', attempts, ...cd, retryOwner: 'page', lastError: stream.error ? { kind: stream.error.kind ?? null } : null };
  if (status === 'failed') {
    const err = stream.error || { kind: 'unexpected' };
    return describeKind('run_stream', err, { attempts, retryOwner: null, ...opts });
  }
  const cond = STREAM_STATUS_CONDITION[status] || 'idle';
  if (cond === 'reconnecting' || cond === 'no_answer') {
    const delay = isNum(stream.nextDelayMs) ? ` Next try in ${Math.round((nextInMs !== null ? nextInMs : stream.nextDelayMs) / 1000)} s.` : '';
    return make('run_stream', cond, { ...common, detail: `${attempts} failed ${attempts === 1 ? 'attempt' : 'attempts'}.${delay}`, retry: true, changeAddress: cond === 'no_answer' });
  }
  if (cond === 'stale') return make('run_stream', 'stale', { ...common, detail: 'Silent for longer than the gateway heartbeat interval.', basis: 'The gateway sends a heartbeat comment every 15 s; this page calls the run stream stale after 25 s without data.', retry: true });
  return make('run_stream', cond, { ...common });
}

// ---------------------------------------------------------------------------------------------------------------
// Aggregation

/** The most severe link view (SEVERITY_ORDER). Non-link views (ended, idle) never win over a link view. */
export function worstLink(views) {
  const links = views.filter((v) => v && v.isLink);
  if (!links.length) return views.find(Boolean) || null;
  let best = null;
  let bestRank = Infinity;
  for (const v of links) {
    const rank = SEVERITY_ORDER.indexOf(v.key);
    if (rank !== -1 && rank < bestRank) {
      best = v;
      bestRank = rank;
    }
  }
  return best;
}

/**
 * @param {{telemetry?: {state: object, now: number, opts?: object}, caps?: object, runStream?: {stream: object, status: string, now: number}, security?: string|null}} input
 * @returns {{telemetry: object|null, lab: object|null, runStream: object|null, worst: object|null, words: string[]}}
 */
export function describeLink({ telemetry, caps, runStream, security = null } = {}) {
  const t = telemetry ? describeTelemetryLink(telemetry.state, telemetry.now, { security, ...(telemetry.opts || {}) }) : null;
  const l = caps ? describeLabLink(caps) : null;
  const r = runStream ? describeRunStreamLink(runStream.stream, runStream.status, runStream.now) : null;
  const all = [t, l, r].filter(Boolean);
  return { telemetry: t, lab: l, runStream: r, worst: worstLink(all), words: all.map((v) => v.word) };
}
