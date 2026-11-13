// Gateway address model (F4, docs/design/remediation/06_functional_ux_work_breakdown.md section 5).
//
// Resolution order, first VALID wins and every rejected candidate leaves a visible note:
//   1. ?gateway=            (this page load only; persisted only if the person presses "Use and remember")
//   2. saved address        (localStorage key STORAGE_KEY, value {"url": "<origin>"} and nothing else)
//   3. VITE_PHANTOM_GATEWAY (build value, passed in by the caller)
//   4. DEFAULT_GATEWAY
//
// Secrets: this module knows nothing about tokens. It never accepts a URL with credentials, it persists the validated ORIGIN
// only (no path, no query, no hash), and it never logs. Tokens stay memory-only in the Lab provider; the caller forgets the
// token when `originChanged(prev, next)` is true (a credential is bound to the origin it was entered for).
// Storage access is wrapped everywhere: private windows, blocked site data and quota errors must not break the app.

import { DEFAULT_GATEWAY } from './config.js';
import { LINK_TEXT, describeLabLink } from './linkState.js';

export { DEFAULT_GATEWAY };
export const STORAGE_KEY = 'phantomcache.gateway.v1';
export const MAX_INPUT_LENGTH = 2048;

const SOURCE_LABEL = Object.freeze({
  query: 'page link',
  saved: 'saved',
  env: 'build',
  default: 'built-in',
  applied: 'entered',
});

/** Where each source is explained to the person ("set by the page link", ...). */
export const SOURCE_TEXT = Object.freeze({
  query: 'set by the page link (?gateway=)',
  saved: 'saved in this browser',
  env: 'build default (VITE_PHANTOM_GATEWAY)',
  default: 'built-in default',
  applied: 'entered on this page',
});

export function isLoopbackHost(hostname) {
  const h = String(hostname || '').toLowerCase();
  return h === 'localhost' || h === '[::1]' || h === '::1' || /^127(\.\d{1,3}){3}$/.test(h);
}

const note = (level, code, text, extra = {}) => ({ level, code, text, ...extra });
const err = (code, text, extra = {}) => ({ code, text, ...extra });

/**
 * Validate one typed or linked address. Pure.
 * @returns {{ok: boolean, url: string|null, errors: {code: string, text: string}[], notices: {level: 'info'|'warn', code: string, text: string}[], suggestion?: string}}
 */
export function validateGatewayInput(text, { pageOrigin = null, pageProtocol = null, securityMode = null } = {}) {
  const fail = (e, extra = {}) => ({ ok: false, url: null, errors: [e], notices: [], ...extra });
  const raw = typeof text === 'string' ? text.trim() : '';
  if (!raw) return fail(err('empty', 'Enter the gateway address, for example http://localhost:8085'));
  if (raw.length > MAX_INPUT_LENGTH) return fail(err('too_long', 'That address is too long to be a gateway address'));
  if (/[\u0000-\u001f\u007f\s]/.test(raw)) return fail(err('invalid', 'That is not a valid URL: it contains spaces or control characters'));
  const hasScheme = /^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(raw);
  if (!hasScheme) {
    let suggestion = null;
    try {
      const u = new URL(`http://${raw}`);
      if (u.hostname) suggestion = `http://${raw}`;
    } catch {
      suggestion = null;
    }
    return fail(err('missing_scheme', suggestion ? `Add http:// or https:// at the start, for example ${suggestion}` : 'That is not a valid URL. Start with http:// or https://'), suggestion ? { suggestion } : {});
  }
  let u;
  try {
    u = new URL(raw);
  } catch {
    return fail(err('invalid', 'That is not a valid URL'));
  }
  if (u.protocol !== 'http:' && u.protocol !== 'https:') return fail(err('scheme', 'Only http and https are allowed'));
  if (u.username || u.password) return fail(err('credentials', 'Do not put a user name or password in the address; paste a token in the connection details instead'));
  if (!u.hostname) return fail(err('invalid', 'That is not a valid URL: no host name'));
  const url = u.origin;
  const notices = [];
  if ((u.pathname && u.pathname !== '/') || u.search || u.hash) {
    notices.push(note('info', 'origin_only', `Only the origin ${url} is used; the path, query and fragment were removed`));
  }
  const loopback = isLoopbackHost(u.hostname);
  if (loopback) {
    notices.push(note('info', 'loopback', 'This computer. Without JWT the Lab API only accepts loopback host names.'));
  } else {
    if (securityMode === 'anonymous_dev_loopback_only') {
      notices.push(note('warn', 'non_loopback_anonymous', 'Without JWT the gateway refuses non-loopback Host headers (HTTP 421); use a loopback name or enable JWT'));
    }
    if (pageProtocol === 'https:' && u.protocol === 'http:') {
      notices.push(note('warn', 'mixed_content', 'This page is served over https and this address is http: browsers block this mixed content'));
    }
  }
  if (typeof pageOrigin === 'string' && pageOrigin && pageOrigin !== url) {
    notices.push(note('info', 'cross_origin', `The gateway's PHANTOM_CORS_ORIGINS must include this page's origin (${pageOrigin})`));
  }
  return { ok: true, url, errors: [], notices };
}

// ---------------------------------------------------------------------------------------------------------------
// Storage (every access wrapped)

const defaultGetStorage = () => globalThis.localStorage;

function withStorage(getStorage, fn, onFail) {
  try {
    const s = getStorage();
    if (!s) return onFail('unavailable');
    return fn(s);
  } catch {
    return onFail('unavailable');
  }
}

/** @returns {{available: boolean, raw: string|null}} raw is the stored string (or null). Never throws. */
export function readSavedRaw(getStorage = defaultGetStorage) {
  return withStorage(getStorage, (s) => ({ available: true, raw: s.getItem(STORAGE_KEY) }), () => ({ available: false, raw: null }));
}

/** Parse the saved value. Only {"url": "<string>"} is accepted. */
export function parseSaved(raw) {
  if (raw === null || raw === undefined) return { url: null, problem: null };
  let v;
  try {
    v = JSON.parse(raw);
  } catch {
    return { url: null, problem: 'corrupt' };
  }
  if (!v || typeof v !== 'object' || Array.isArray(v) || typeof v.url !== 'string') return { url: null, problem: 'corrupt' };
  return { url: v.url, problem: null };
}

/** @returns {{ok: boolean, reason: string|null}} */
export function writeSavedGateway(url, getStorage = defaultGetStorage) {
  return withStorage(getStorage, (s) => {
    s.setItem(STORAGE_KEY, JSON.stringify({ url }));
    return { ok: true, reason: null };
  }, (reason) => ({ ok: false, reason }));
}

export function clearSavedGateway(getStorage = defaultGetStorage) {
  return withStorage(getStorage, (s) => {
    s.removeItem(STORAGE_KEY);
    return { ok: true, reason: null };
  }, (reason) => ({ ok: false, reason }));
}

// ---------------------------------------------------------------------------------------------------------------
// Resolution

function rejectionNote(source, errors) {
  const first = errors[0] || err('invalid', 'not a valid URL');
  const label = source === 'query' ? 'the address from the page link' : source === 'saved' ? 'the saved address' : source === 'env' ? 'the build-time address' : 'the address';
  return note('warn', `ignored_${source}`, `Ignored ${label}: ${first.text.charAt(0).toLowerCase()}${first.text.slice(1)}`, { source, reason: first.code });
}

/**
 * Pure resolution. `savedRaw` is the stored string (or null), `storageAvailable` whether storage could be read.
 * @returns {{url: string, source: 'query'|'saved'|'env'|'default', status: 'ok'|'fallback', notes: object[], rejected: object[], fromLink: boolean, needsLinkConfirmation: boolean}}
 */
export function resolveGatewayConfig({ search = '', savedRaw = null, envValue, pageOrigin = null, pageProtocol = null } = {}) {
  const query = new URLSearchParams(search).get('gateway');
  const parsedSaved = parseSaved(savedRaw);
  const candidates = [];
  if (query) candidates.push({ source: 'query', raw: query });
  if (savedRaw !== null && savedRaw !== undefined) {
    if (parsedSaved.problem) candidates.push({ source: 'saved', raw: null, problem: parsedSaved.problem });
    else candidates.push({ source: 'saved', raw: parsedSaved.url });
  }
  if (envValue) candidates.push({ source: 'env', raw: envValue });
  const notes = [];
  const rejected = [];
  for (const c of candidates) {
    if (c.problem) {
      const e = [err('corrupt', 'it could not be read')];
      rejected.push({ source: c.source, errors: e });
      notes.push(rejectionNote(c.source, e));
      continue;
    }
    const v = validateGatewayInput(c.raw, { pageOrigin, pageProtocol });
    if (!v.ok) {
      rejected.push({ source: c.source, errors: v.errors });
      notes.push(rejectionNote(c.source, v.errors));
      continue;
    }
    const out = { url: v.url, source: c.source, status: rejected.length ? 'fallback' : 'ok', notes: [...notes, ...v.notices.filter((n) => n.level === 'warn')], rejected, fromLink: c.source === 'query' };
    out.needsLinkConfirmation = out.fromLink && !isLoopbackHost(new URL(v.url).hostname);
    if (out.needsLinkConfirmation) out.notes.push(note('warn', 'from_link', 'This address came from the page link. Tokens you enter are sent there.'));
    return out;
  }
  return { url: DEFAULT_GATEWAY, source: 'default', status: rejected.length ? 'fallback' : 'ok', notes, rejected, fromLink: false, needsLinkConfirmation: false };
}

/** Two addresses differ in origin: a token entered for one must not be sent to the other. */
export function originChanged(prevUrl, nextUrl) {
  try {
    return new URL(prevUrl).origin !== new URL(nextUrl).origin;
  } catch {
    return prevUrl !== nextUrl;
  }
}

/** Short, non-secret display form of a gateway address: host:port. */
export function gatewayHostPort(url) {
  try {
    return new URL(url).host;
  } catch {
    return '';
  }
}

// ---------------------------------------------------------------------------------------------------------------
// Store (runtime setter + subscribe, shaped for React useSyncExternalStore)

/**
 * @param {{search?: string, envValue?: string, pageOrigin?: string|null, pageProtocol?: string|null, getStorage?: () => Storage}} [cfg]
 */
export function createGatewayStore(cfg = {}) {
  const { search = '', envValue, pageOrigin = null, pageProtocol = null, getStorage = defaultGetStorage } = cfg;
  const listeners = new Set();
  let lock = null; // reason text while a non-abortable request is pending
  let revision = 0;
  let epoch = 0;

  const read = () => readSavedRaw(getStorage);

  function snapshotFrom(resolved, saved, extra = {}) {
    const savedParsed = parseSaved(saved.raw);
    return Object.freeze({
      url: resolved.url,
      source: extra.source || resolved.source,
      sourceText: SOURCE_TEXT[extra.source || resolved.source],
      status: extra.status || resolved.status,
      notes: Object.freeze(extra.notes || resolved.notes),
      fromLink: extra.fromLink ?? resolved.fromLink,
      needsLinkConfirmation: extra.needsLinkConfirmation ?? resolved.needsLinkConfirmation,
      remembered: !savedParsed.problem && savedParsed.url === resolved.url,
      storageAvailable: saved.available,
      locked: lock,
      hostPort: gatewayHostPort(resolved.url),
      revision,
      epoch,
    });
  }

  const initialSaved = read();
  let snap = snapshotFrom(resolveGatewayConfig({ search, savedRaw: initialSaved.raw, envValue, pageOrigin, pageProtocol }), initialSaved);

  function publish(next) {
    revision += 1;
    snap = Object.freeze({ ...next, revision, epoch, locked: lock });
    listeners.forEach((l) => l());
  }

  return {
    getSnapshot: () => snap,
    getServerSnapshot: () => snap,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    /** Validate without applying (the form uses it while typing). */
    validate: (text, o = {}) => validateGatewayInput(text, { pageOrigin, pageProtocol, ...o }),
    /** While `reason` is a string the address cannot change (a start or cancel request is in flight). null releases it. */
    setLock(reason) {
      const next = typeof reason === 'string' && reason ? reason : null;
      if (next === lock) return;
      lock = next;
      publish({ ...snap });
    },
    /**
     * Switch to a new address at runtime. `remember` persists the origin (default true). Returns what happened so the UI can say it.
     * @returns {{ok: boolean, errors: object[], notices: object[], url: string|null, changed: boolean, originChanged: boolean, persisted: 'saved'|'not_saved'|'skipped'}}
     */
    apply(text, { remember = true } = {}) {
      if (lock) return { ok: false, errors: [err('busy', `The address cannot change now: ${lock}`)], notices: [], url: null, changed: false, originChanged: false, persisted: 'skipped' };
      const v = validateGatewayInput(text, { pageOrigin, pageProtocol });
      if (!v.ok) return { ok: false, errors: v.errors, notices: v.notices, url: null, changed: false, originChanged: false, persisted: 'skipped', suggestion: v.suggestion };
      const prev = snap.url;
      const oc = originChanged(prev, v.url);
      let persisted = 'skipped';
      if (remember) persisted = writeSavedGateway(v.url, getStorage).ok ? 'saved' : 'not_saved';
      if (oc) epoch += 1;
      const saved = read();
      const resolved = { url: v.url, source: 'applied', status: 'ok', notes: [], fromLink: false, needsLinkConfirmation: false };
      const notices = [...v.notices];
      if (persisted === 'not_saved') notices.push(note('warn', 'not_remembered', 'The address is in use but could not be saved in this browser, so it will not be remembered'));
      publish(snapshotFrom(resolved, saved, { source: 'applied', status: 'ok', notes: persisted === 'not_saved' ? [notices[notices.length - 1]] : [] }));
      return { ok: true, errors: [], notices, url: v.url, changed: prev !== v.url, originChanged: oc, persisted };
    },
    /** Forget the saved address and the page-link override; fall back to the build value, then the built-in default. */
    reset() {
      if (lock) return { ok: false, errors: [err('busy', `The address cannot change now: ${lock}`)], changed: false, originChanged: false };
      const prev = snap.url;
      clearSavedGateway(getStorage);
      const saved = read();
      const resolved = resolveGatewayConfig({ search: '', savedRaw: null, envValue, pageOrigin, pageProtocol });
      const oc = originChanged(prev, resolved.url);
      if (oc) epoch += 1;
      publish(snapshotFrom(resolved, saved));
      return { ok: true, errors: [], changed: prev !== resolved.url, originChanged: oc };
    },
  };
}

// ---------------------------------------------------------------------------------------------------------------
// Test connection result -> words (uses the shared vocabulary; the page does not invent a cause)

/**
 * @param {{ok: boolean, kind: string|null, status: number|null}} probe  one entry of labApi.testGatewayConnection()
 * @returns {{word: string, who: 'gateway'|'page', answered: boolean, text: string}}
 */
export function describeProbe(probe) {
  const view = describeLabLink(probe.ok ? { status: 'ready', data: {} } : { status: 'error', data: null, error: { kind: probe.kind, status: probe.status ?? 0 } });
  const answered = probe.ok || (probe.kind !== 'network' && probe.kind !== 'aborted');
  return {
    word: view.word,
    who: answered ? 'gateway' : 'page',
    answered,
    text: answered ? `${view.word}. The gateway answered${view.detail ? `: ${view.detail}` : '.'}` : `${view.word}. ${LINK_TEXT.no_answer.cause}`,
  };
}

export { SOURCE_LABEL };

// ---------------------------------------------------------------------------------------------------------------
// Telemetry history persistence (WP5 / F9)
// Keyed strictly by gatewayUrl; wrapped storage access; max age 5 minutes bound.
export const HISTORY_STORAGE_KEY = 'phantomcache.history.v1';
const HISTORY_MAX_AGE_MS = 5 * 60 * 1000;

export function readTelemetrySeries(gatewayUrl, getStorage = defaultGetStorage) {
  if (!gatewayUrl) return null;
  return withStorage(getStorage, (s) => {
    const raw = s.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return null;
    try {
      const data = JSON.parse(raw);
      if (!data || data.gatewayUrl !== gatewayUrl) return null;
      if (typeof data.savedAt === 'number' && Date.now() - data.savedAt > HISTORY_MAX_AGE_MS) return null;
      if (data.series && data.series.throughput && data.series.hitRatio && data.series.p99) {
        return data.series;
      }
    } catch {
      // Ignore parse failure
    }
    return null;
  }, () => null);
}

export function saveTelemetrySeries(gatewayUrl, series, getStorage = defaultGetStorage) {
  if (!gatewayUrl || !series) return false;
  return withStorage(getStorage, (s) => {
    try {
      s.setItem(HISTORY_STORAGE_KEY, JSON.stringify({
        gatewayUrl,
        savedAt: Date.now(),
        series,
      }));
      return true;
    } catch {
      return false;
    }
  }, () => false);
}

