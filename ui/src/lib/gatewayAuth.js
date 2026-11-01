// Gateway sign-in logic for the Control Center live screens. Pure functions: no network, no storage, no logging.
//
// The page NEVER verifies a token (the gateway does). It reads the claims of the token it was given, for display only, so a person can see
// who they are acting as. Nothing returned from this module contains the token string, so no screen or error message built from these
// results can echo it.
//
// Claim names follow the gateway: src/security/jwt.rs JwtClaims { sub, iss, aud, exp, iat, tenant_id, roles }.
// Answer words follow src/api/server.rs authenticate_request: 401 with X-Auth-Error MissingToken / ExpiredToken / InvalidIssuer /
// InvalidAudience / InvalidToken (+ WWW-Authenticate: Bearer), 403 with X-Auth-Error InsufficientPermissions.

export const MAX_TOKEN_CHARS = 8192;
const TOKEN_SHAPE = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]*$/;

function decodeBase64Url(s) {
  const b64 = String(s).replace(/-/g, '+').replace(/_/g, '/');
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4);
  const bin = typeof atob === 'function' ? atob(padded) : Buffer.from(padded, 'base64').toString('binary');
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

const clip = (v, n) => String(v).replace(/[\u0000-\u001f\u007f]/g, ' ').trim().slice(0, n);

/** Normalise what a person pasted: trim, drop a leading "Bearer ", reject anything that is not one JWT-shaped string. */
export function normalizeTokenInput(text) {
  if (typeof text !== 'string') return { ok: false, reason: 'empty' };
  let t = text.trim();
  if (/^bearer\s+/i.test(t)) t = t.replace(/^bearer\s+/i, '').trim();
  if (!t) return { ok: false, reason: 'empty' };
  if (t.length > MAX_TOKEN_CHARS) return { ok: false, reason: 'too_long' };
  if (/\s/.test(t)) return { ok: false, reason: 'whitespace' };
  if (!TOKEN_SHAPE.test(t)) return { ok: false, reason: 'not_jwt_shape' };
  return { ok: true, token: t };
}

export const INPUT_PROBLEM_TEXT = Object.freeze({
  empty: 'Paste a token first.',
  too_long: 'That is too long to be a token.',
  whitespace: 'A token is one piece of text with no spaces or line breaks. Paste only the token.',
  not_jwt_shape: 'That does not look like a token (three parts separated by dots). Nothing was stored.',
});

/**
 * Display-only decoding. Returns { ok: true, claims } or { ok: false, reason }. `claims` holds only short, control-free strings and
 * numbers; roles are capped. Never throws.
 */
export function decodeClaims(token) {
  try {
    const n = normalizeTokenInput(token);
    if (!n.ok) return { ok: false, reason: n.reason };
    const [h, p] = n.token.split('.');
    const header = JSON.parse(decodeBase64Url(h));
    const c = JSON.parse(decodeBase64Url(p));
    if (!c || typeof c !== 'object' || Array.isArray(c)) return { ok: false, reason: 'claims_not_object' };
    const num = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null);
    const str = (v, k) => (typeof v === 'string' && v.trim() ? clip(v, k) : null);
    const aud = Array.isArray(c.aud) ? c.aud.filter((x) => typeof x === 'string').map((x) => clip(x, 80)).join(', ') || null : str(c.aud, 80);
    const roles = Array.isArray(c.roles) ? c.roles.filter((r) => typeof r === 'string').map((r) => clip(r, 40)).slice(0, 12) : [];
    return {
      ok: true,
      claims: {
        subject: str(c.sub, 80),
        tenant: str(c.tenant_id, 80),
        roles,
        issuer: str(c.iss, 80),
        audience: aud,
        expiresAtSec: num(c.exp),
        issuedAtSec: num(c.iat),
        algorithm: header && typeof header.alg === 'string' ? clip(header.alg, 16) : null,
      },
    };
  } catch {
    return { ok: false, reason: 'unreadable' };
  }
}

/** { state: 'unknown'|'valid'|'expired', secondsLeft } by the page clock. The gateway's clock decides; this is a hint. */
export function expiryState(claims, nowMs) {
  if (!claims || claims.expiresAtSec === null || !Number.isFinite(nowMs)) return { state: 'unknown', secondsLeft: null };
  const left = Math.floor(claims.expiresAtSec - nowMs / 1000);
  return left > 0 ? { state: 'valid', secondsLeft: left } : { state: 'expired', secondsLeft: left };
}

/** "in 4 min 12 s", "in 2 h 5 min", "3 min ago". */
export function formatSpan(seconds) {
  const s = Math.abs(Math.round(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  const body = h > 0 ? `${h} h ${m} min` : m > 0 ? `${m} min ${r} s` : `${r} s`;
  return seconds >= 0 ? `in ${body}` : `${body} ago`;
}

export function expiryText(exp) {
  if (exp.state === 'unknown') return 'No expiry in the token (the gateway requires one).';
  if (exp.state === 'valid') return `Expires ${formatSpan(exp.secondsLeft)} (by this page's clock).`;
  return `Expired ${formatSpan(exp.secondsLeft)} (by this page's clock). The gateway will refuse it.`;
}

export const DECODE_LABEL = 'Decoded by this page, not verified; the gateway verifies the token.';

const READ_ROLE = ['telemetry:read', 'observer', 'admin'];
const has = (roles, list) => roles.some((r) => list.includes(String(r).toLowerCase()));
/** Which of the live screens' needs the roles in the token appear to cover (display only; the gateway decides). */
export function roleHints(claims) {
  if (!claims) return null;
  return {
    observe: has(claims.roles, READ_ROLE),
    cacheRead: has(claims.roles, ['cache:read', 'requester', 'read', 'admin']),
    invalidate: has(claims.roles, ['cache:invalidate', 'requester', 'invalidate', 'admin']),
    operate: has(claims.roles, ['scenario:operate', 'operator', 'admin']),
  };
}

/**
 * Plain-words meaning of one gateway answer.
 * @param {{status: number, authError?: string|null, hadToken: boolean}} a  status 0 = no answer
 * @returns {{key: string, tone: 'ok'|'warn'|'critical'|'info', title: string, text: string, needsAction: boolean}}
 */
export function describeAnswer({ status, authError = null, hadToken = false } = {}) {
  const e = authError || null;
  if (status === 401) {
    if (e === 'ExpiredToken') return { key: 'expired', tone: 'critical', needsAction: true, title: 'The token has expired', text: 'The gateway refused the token because its time has run out. Paste a fresh token for this gateway.' };
    if (e === 'InvalidIssuer') return { key: 'wrong_issuer', tone: 'critical', needsAction: true, title: 'The token was issued by someone this gateway does not trust', text: 'The signature was valid but the issuer (iss) is not the one this gateway accepts. Ask for a token issued for this gateway.' };
    if (e === 'InvalidAudience') return { key: 'wrong_audience', tone: 'critical', needsAction: true, title: 'The token was issued for a different audience', text: 'The signature was valid but the audience (aud) is not the one this gateway accepts. Ask for a token issued for this gateway.' };
    if (e === 'MissingToken' || (!e && !hadToken)) return { key: 'requires_token', tone: 'info', needsAction: true, title: 'Sign in to see this gateway', text: 'This gateway is secure, so it needs your token before it shows anything. Paste the token below and press Sign in. Until then no gateway values are shown.' };
    return { key: 'rejected', tone: 'critical', needsAction: true, title: 'The gateway did not accept the token', text: 'The gateway answered 401. The token could not be verified (wrong signing key, damaged, or not a token for this gateway). The gateway does not say which; it gives no detail on purpose.' };
  }
  if (status === 403) {
    if (e === 'ReservedTenant' || e === 'MissingTenant') return { key: 'no_tenant', tone: 'warn', needsAction: true, title: 'The token has no usable tenant', text: 'The token is valid but carries no tenant_id (or a reserved one). Cache reads and purges need a tenant; telemetry does not.' };
    return { key: 'forbidden', tone: 'warn', needsAction: true, title: 'The token is valid but lacks the role', text: 'The gateway answered 403 (InsufficientPermissions). Live telemetry needs the telemetry:read role (Observer); admin also satisfies it. Another token may be needed for this screen.' };
  }
  if (status >= 200 && status < 300) return { key: 'ok', tone: 'ok', needsAction: false, title: hadToken ? 'The gateway accepted the token' : 'The gateway answered without a token', text: hadToken ? 'The gateway answered the last request with this token.' : 'This gateway did not ask for a token (development mode, no authentication). Use it on this computer only.' };
  if (status === 404) return { key: 'missing', tone: 'info', needsAction: false, title: 'The gateway has no such endpoint', text: 'The gateway answered 404. It may be an older build.' };
  if (status === 421 || status === 400) return { key: 'refused', tone: 'warn', needsAction: false, title: 'The gateway refused the request', text: `The gateway answered ${status}. Check the gateway address.` };
  if (status >= 500) return { key: 'server', tone: 'warn', needsAction: false, title: 'The gateway reported an error', text: `The gateway answered ${status}.` };
  return { key: 'no_answer', tone: 'warn', needsAction: false, title: 'No answer from the gateway', text: 'The request did not reach the gateway or was blocked. This page cannot tell which. If the gateway is on another origin, its PHANTOM_CORS_ORIGINS must include this page origin.' };
}

/** What the header chip says. identity: decodeClaims result or null; answer: describeAnswer result or null. */
export function chipView({ hasToken, decoded, exp, answer, requiresToken }) {
  if (!hasToken) {
    if (requiresToken) return { tone: 'info', icon: 'info', text: 'Sign in' };
    if (answer && answer.key === 'ok') return { tone: 'neutral', icon: 'info', text: 'No token (not required)' };
    return { tone: 'neutral', icon: 'info', text: 'No token' };
  }
  if (answer && answer.key === 'expired') return { tone: 'critical', icon: 'error', text: 'Token expired' };
  if (exp && exp.state === 'expired') return { tone: 'critical', icon: 'error', text: 'Token expired' };
  if (answer && ['rejected', 'wrong_issuer', 'wrong_audience'].includes(answer.key)) return { tone: 'critical', icon: 'error', text: 'Token not accepted' };
  if (answer && ['forbidden', 'no_tenant'].includes(answer.key)) return { tone: 'warn', icon: 'warn', text: 'Token lacks a role' };
  const who = decoded && decoded.ok ? decoded.claims : null;
  const name = who ? who.tenant || who.subject || null : null;
  const label = name ? `Signed in as ${name}` : 'Signed in';
  return { tone: answer && answer.key === 'ok' ? 'ok' : 'info', icon: answer && answer.key === 'ok' ? 'ok' : 'info', text: label };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// One-time hand-off from another window (no URL). Decision (documented in docs/lanes/LANE-L2.md):
//   - The page asks its opener: postMessage({ type: 'phantomcache:token-request', v: 1 }, '*'). The request carries no secret.
//   - The opener answers the page ONLY: postMessage({ type: 'phantomcache:token', v: 1, gateway: '<gateway origin>', token }, <page origin>).
//   - The page accepts it only if all of these hold (checked here): the sender window is the page's opener; the message has exactly the
//     fields above; the gateway field equals the gateway this page uses; the token has the JWT shape. If the sender's origin is this
//     page's own origin or a loopback origin it is applied at once; any other origin is held until the person confirms it.

export const HANDOFF_TYPE = 'phantomcache:token';
export const HANDOFF_REQUEST_TYPE = 'phantomcache:token-request';

export function isLoopbackOrigin(origin) {
  try {
    const u = new URL(origin);
    return ['localhost', '127.0.0.1', '[::1]', '::1'].includes(u.hostname);
  } catch {
    return false;
  }
}

/**
 * @param {{origin: string, sourceIsOpener: boolean, data: unknown, gatewayUrl: string, pageOrigin: string|null}} m
 * @returns {{action: 'apply'|'confirm'|'ignore', token?: string, from?: string, reason?: string}}
 */
export function evaluateHandoff({ origin, sourceIsOpener, data, gatewayUrl, pageOrigin }) {
  if (!sourceIsOpener) return { action: 'ignore', reason: 'not_from_opener' };
  if (!data || typeof data !== 'object' || Array.isArray(data) || data.type !== HANDOFF_TYPE) return { action: 'ignore', reason: 'not_a_handoff' };
  const keys = Object.keys(data).sort().join(',');
  if (data.v !== 1 || keys !== 'gateway,token,type,v') return { action: 'ignore', reason: 'bad_shape' };
  if (typeof data.gateway !== 'string' || data.gateway !== gatewayUrl) return { action: 'ignore', reason: 'other_gateway' };
  const n = normalizeTokenInput(data.token);
  if (!n.ok) return { action: 'ignore', reason: 'bad_token' };
  const from = typeof origin === 'string' ? origin : '';
  const trusted = (pageOrigin && from === pageOrigin) || isLoopbackOrigin(from);
  return trusted ? { action: 'apply', token: n.token, from } : { action: 'confirm', token: n.token, from };
}
