export const DEFAULT_GATEWAY = 'http://localhost:8085';

/** Resolve the gateway base URL. Only http(s) origins are accepted; anything else falls back with an error note. */
export function resolveGateway({ search = '', envValue } = {}) {
  const fromQuery = new URLSearchParams(search).get('gateway');
  const candidates = [
    fromQuery ? { raw: fromQuery, source: 'query' } : null,
    envValue ? { raw: envValue, source: 'env' } : null,
  ].filter(Boolean);
  for (const c of candidates) {
    try {
      const u = new URL(c.raw);
      if (u.protocol === 'http:' || u.protocol === 'https:') {
        return { url: u.origin, source: c.source };
      }
      return { url: DEFAULT_GATEWAY, source: 'default', error: `Ignored ${c.source} gateway: only http and https are allowed` };
    } catch {
      return { url: DEFAULT_GATEWAY, source: 'default', error: `Ignored ${c.source} gateway: not a valid URL` };
    }
  }
  return { url: DEFAULT_GATEWAY, source: 'default' };
}

export function eventsUrl(base) {
  return `${base}/api/events?schema=2`;
}
