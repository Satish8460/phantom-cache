export function getDefaultGateway() {
  if (typeof window !== 'undefined' && window.location && window.location.hostname) {
    const host = window.location.hostname;
    // When accessed from an external phone or LAN device, use the current host IP instead of localhost
    if (host !== 'localhost' && host !== '127.0.0.1' && host !== '::1') {
      return `http://${host}:18185`;
    }
  }
  return 'http://127.0.0.1:18185';
}

export const DEFAULT_GATEWAY = getDefaultGateway();

/** Resolve the gateway base URL. Only http(s) origins are accepted; anything else falls back with an error note. */
export function resolveGateway({ search = '', envValue } = {}) {
  const fromQuery = typeof window !== 'undefined' && window.location
    ? new URLSearchParams(search || window.location.search).get('gateway')
    : new URLSearchParams(search).get('gateway');
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
      return { url: getDefaultGateway(), source: 'default', error: `Ignored ${c.source} gateway: only http and https are allowed` };
    } catch {
      return { url: getDefaultGateway(), source: 'default', error: `Ignored ${c.source} gateway: not a valid URL` };
    }
  }
  return { url: getDefaultGateway(), source: 'default' };
}

export function eventsUrl(base) {
  return `${base}/api/events?schema=2`;
}
