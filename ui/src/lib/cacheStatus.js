// ui/src/lib/cacheStatus.js
// Authoritative cache response parser adhering to PhantomCache serving contract (ADR-002 / server.rs).

export const KNOWN_CACHE_STATUSES = Object.freeze([
  'HIT',
  'MISS',
  'COALESCED',
  'STALE',
  'BYPASS',
  'BYPASS-AUTH',
]);

const KNOWN_SET = new Set(KNOWN_CACHE_STATUSES);

/**
 * Parses HTTP status and headers according to the authoritative gateway serving contract.
 * Never infers cache status from `Age`. Distinguishes cache verdicts from non-cache responses and errors.
 *
 * @param {number} status - HTTP response status code
 * @param {Headers|Map|Object} headers - Response headers
 * @returns {{
 *   kind: 'cache' | 'gateway-error' | 'non-cache' | 'error' | 'unexpected',
 *   verdict: string,
 *   error: string|null,
 *   note?: string
 * }}
 */
export function parseCacheResponse(status, headers) {
  const getHeader = (name) => {
    if (!headers) return null;
    if (typeof headers.get === 'function') {
      return headers.get(name) || headers.get(name.toLowerCase());
    }
    const lower = name.toLowerCase();
    for (const [k, v] of Object.entries(headers)) {
      if (k.toLowerCase() === lower) return Array.isArray(v) ? v[0] : v;
    }
    return null;
  };

  const raw = getHeader('x-cache-status');
  const err = getHeader('x-cache-error');

  if (raw) {
    const norm = String(raw).trim().toUpperCase();
    if (KNOWN_SET.has(norm)) {
      return { kind: 'cache', verdict: norm, error: null };
    }
    return { kind: 'unexpected', verdict: norm, error: null, note: `unrecognised X-Cache-Status "${raw}"` };
  }

  if (err) {
    return { kind: 'gateway-error', verdict: 'N/A', error: String(err) };
  }

  if (status >= 200 && status < 300) {
    return { kind: 'non-cache', verdict: 'N/A', error: null };
  }

  return { kind: 'error', verdict: 'N/A', error: `HTTP ${status}` };
}
