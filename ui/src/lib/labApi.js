// Phantom Lab HTTP client (docs/design/lab_api_contract.md, docs/user/lab_api.md).
// - The bearer token is read from `getToken()` per request and goes only into the Authorization header.
//   It is never put in a URL, a query string, storage, or a log line.
// - Every method resolves to { ok: true, status, data } or { ok: false, error } (never throws for HTTP or network
//   failures), where error is the object produced by mapLabError().
// - The path segments the caller supplies (run ids, evidence ids, artifact names) are validated against a strict
//   charset before they are placed in a URL, so a hostile id cannot add path segments or a query.

export const LAB_PREFIX = '/api/v1/lab';
export const ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/; // must start alphanumeric: "." and ".." never reach a URL
export const ARTIFACT_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;

/** User-facing text per error kind. `fix` says what the person can do. */
export const ERROR_TEXT = Object.freeze({
  disabled: {
    title: 'The Lab API is not enabled on this gateway',
    fix: 'Start the gateway with PHANTOM_LAB_ENABLED=true (and PHANTOM_LAB_DIR if retained results live elsewhere). Without it no /api/v1/lab route exists. See docs/user/lab_api.md.',
  },
  unauthorized_missing: {
    title: 'A token is required',
    fix: 'This gateway has JWT authentication on. Paste an Observer token (to read) or an Operator token (to start and cancel runs) in the connection panel.',
  },
  unauthorized_invalid: {
    title: 'The token was not accepted',
    fix: 'The gateway rejected the token (invalid signature, issuer, or audience). Check that it was issued for this gateway.',
  },
  unauthorized_expired: {
    title: 'The token has expired',
    fix: 'Paste a fresh token. Open streams are closed by the gateway when their token expires.',
  },
  forbidden_role: {
    title: 'The token does not have the required role',
    fix: 'Reading needs the Observer role (telemetry:read). Starting and cancelling runs needs Operator (scenario:operate). An Operator-only token cannot read; admin satisfies both.',
  },
  forbidden_origin: {
    title: 'The gateway refused this page origin',
    fix: 'In anonymous development mode the gateway only accepts requests from its own origin or from PHANTOM_CORS_ORIGINS. Serve this page from an allowed origin (the development profile allows http://localhost:3000 and http://127.0.0.1:3000).',
  },
  host_guard: {
    title: 'The gateway refused the Host header',
    fix: 'In anonymous development mode the Host must be 127.0.0.1, localhost or [::1] with the bind port. Reach the gateway by one of those names.',
  },
  run_active: {
    title: 'A run is already active',
    fix: 'The gateway allows one active run. Wait for it to finish or cancel it, then start the next run.',
  },
  invalid_spec: {
    title: 'The gateway rejected the run specification',
    fix: 'The form mirrors the documented limits, but the gateway is authoritative. Read the message and adjust the field.',
  },
  busy: {
    title: 'Too many open streams',
    fix: 'The gateway allows a limited number of concurrent event streams. Close other tabs watching runs and retry.',
  },
  not_found: {
    title: 'Not found',
    fix: 'The gateway does not know this item. It may have been removed, or the identifier is wrong.',
  },
  too_large: {
    title: 'The request or artifact exceeded a size limit',
    fix: 'The gateway caps request bodies at 16 KiB and artifact downloads at 64 MiB.',
  },
  bad_request: {
    title: 'The gateway could not process the request',
    fix: 'Retry. If it persists, the page and the gateway may be different versions.',
  },
  server: {
    title: 'The gateway reported an internal error',
    fix: 'Retry. The error body is deliberately generic; check the gateway log.',
  },
  network: {
    title: 'Could not reach the gateway',
    fix: 'Check that the gateway is running at the address this page is using. If this page and the gateway have different origins, the gateway PHANTOM_CORS_ORIGINS must include this page origin.',
  },
  unexpected: {
    title: 'The response was not in the expected format',
    fix: 'The page and gateway may be different versions (the Lab API reports schema "lab-1").',
  },
  aborted: { title: 'Cancelled', fix: '' },
});

/** Kinds after which retrying the same request cannot succeed without a person changing something. */
export const TERMINAL_ERROR_KINDS = Object.freeze([
  'disabled', 'unauthorized_missing', 'unauthorized_invalid', 'unauthorized_expired', 'forbidden_role',
  'forbidden_origin', 'host_guard', 'not_found',
]);

export function isTerminalError(kind) {
  return TERMINAL_ERROR_KINDS.includes(kind);
}

function clipText(s, n = 160) {
  const t = String(s ?? '').replace(/[\u0000-\u001f\u007f]/g, ' ').trim();
  return t.length > n ? `${t.slice(0, n - 1)}…` : t;
}

function header(headers, name) {
  if (!headers) return null;
  if (typeof headers.get === 'function') return headers.get(name);
  const k = Object.keys(headers).find((x) => x.toLowerCase() === name.toLowerCase());
  return k ? headers[k] : null;
}

/**
 * Map an HTTP failure to a stable error kind. `bodyJson` is the parsed body or null; `bodyText` the raw text.
 * A 404 whose body is not a lab-1 document means the route itself does not exist (Lab API off).
 */
export function mapLabError({ status, headers, bodyJson, bodyText, hadToken = false }) {
  const isLab = bodyJson && typeof bodyJson === 'object' && bodyJson.schema === 'lab-1';
  const code = isLab && bodyJson.error && typeof bodyJson.error.code === 'string' ? bodyJson.error.code : null;
  const serverMessage = clipText(isLab && bodyJson.error ? bodyJson.error.message : bodyText);
  const authErr = header(headers, 'x-auth-error');
  let kind;
  if (status === 404) kind = isLab ? 'not_found' : 'disabled';
  else if (status === 401) {
    if (authErr === 'MissingToken' || (!authErr && !hadToken)) kind = 'unauthorized_missing';
    else if (authErr === 'ExpiredToken') kind = 'unauthorized_expired';
    else kind = 'unauthorized_invalid';
  } else if (status === 403) kind = code === 'bad_origin' ? 'forbidden_origin' : 'forbidden_role';
  else if (status === 421) kind = 'host_guard';
  else if (status === 409) kind = 'run_active';
  else if (status === 422) kind = 'invalid_spec';
  else if (status === 429) kind = 'busy';
  else if (status === 413) kind = 'too_large';
  else if (status >= 500) kind = 'server';
  else kind = 'bad_request';
  return { kind, status, code, serverMessage, ...ERROR_TEXT[kind] };
}

/** Network-failure fix text with the two addresses filled in. Neither value is a secret (origins only). */
export function networkFix(gatewayUrl, pageOrigin) {
  const at = typeof gatewayUrl === 'string' && gatewayUrl ? gatewayUrl : 'the address this page is using';
  const origin = typeof pageOrigin === 'string' && pageOrigin ? pageOrigin : 'this page origin';
  return `Check that the gateway is running at ${at}. If this page and the gateway have different origins, the gateway PHANTOM_CORS_ORIGINS must include ${origin}.`;
}

export function networkError(err) {
  if (err && err.name === 'AbortError') return { kind: 'aborted', status: 0, code: null, serverMessage: '', ...ERROR_TEXT.aborted };
  return { kind: 'network', status: 0, code: null, serverMessage: '', ...ERROR_TEXT.network };
}

export function labUrl(baseUrl, segments, query) {
  const base = String(baseUrl).replace(/\/+$/, '');
  const path = [LAB_PREFIX, ...segments].join('/');
  let qs = '';
  if (query) {
    const parts = [];
    for (const [k, v] of Object.entries(query)) {
      if (v === undefined || v === null) continue;
      if (!/^[a-z_]+$/.test(k) || !Number.isInteger(v) || v < 0) throw new Error(`unsupported query parameter ${k}`);
      parts.push(`${k}=${v}`);
    }
    qs = parts.length ? `?${parts.join('&')}` : '';
  }
  return `${base}${path}${qs}`;
}

function checkId(id, pattern = ID_PATTERN) {
  if (typeof id !== 'string' || !pattern.test(id)) throw new Error('invalid identifier');
  return id;
}

/** A gateway path the live screens may request: /api/..., no scheme, no space or control character, no dot segment, no double slash. */
export function isGatewayPath(path) {
  return typeof path === 'string' && /^\/api\/[A-Za-z0-9_\-./?=&,%:]*$/.test(path) && !/\/\/|\/\.\.?(\/|$|\?)/.test(path);
}

async function readBody(res) {
  let text = '';
  try {
    text = await res.text();
  } catch {
    text = '';
  }
  let json = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = null;
  }
  return { text, json };
}

export function createLabClient({ baseUrl, getToken = () => null, fetchImpl } = {}) {
  const doFetch = (...a) => (fetchImpl || globalThis.fetch)(...a);

  function init(method, { body, signal, extraHeaders } = {}) {
    const headers = { Accept: 'application/json', ...extraHeaders };
    const token = getToken();
    if (token) headers.Authorization = `Bearer ${token}`;
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    return {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal,
      credentials: 'omit',
      cache: 'no-store',
      redirect: 'error',
    };
  }

  async function request(method, segments, { query, body, signal } = {}) {
    let url;
    try {
      url = labUrl(baseUrl, segments, query);
    } catch {
      return { ok: false, error: { kind: 'bad_request', status: 0, code: null, serverMessage: '', ...ERROR_TEXT.bad_request } };
    }
    let res;
    try {
      res = await doFetch(url, init(method, { body, signal }));
    } catch (e) {
      return { ok: false, error: networkError(e) };
    }
    const { text, json } = await readBody(res);
    if (!res.ok) {
      return { ok: false, error: mapLabError({ status: res.status, headers: res.headers, bodyJson: json, bodyText: text, hadToken: Boolean(getToken()) }) };
    }
    if (!json || typeof json !== 'object' || json.schema !== 'lab-1') {
      return { ok: false, error: { kind: 'unexpected', status: res.status, code: null, serverMessage: '', ...ERROR_TEXT.unexpected } };
    }
    return { ok: true, status: res.status, data: json };
  }

  return {
    baseUrl,
    capabilities: (o) => request('GET', ['capabilities'], o),
    evidenceIndex: (o) => request('GET', ['evidence'], o),
    evidence: (id, o) => request('GET', ['evidence', encodeURIComponent(checkId(id))], o),
    runs: ({ limit, offset, signal } = {}) => request('GET', ['runs'], { query: { limit, offset }, signal }),
    run: (id, o) => request('GET', ['runs', encodeURIComponent(checkId(id))], o),
    artifacts: (id, o) => request('GET', ['runs', encodeURIComponent(checkId(id)), 'artifacts'], o),
    startRun: (kind, spec, o = {}) => request('POST', ['runs'], { ...o, body: { kind, spec } }),
    cancelRun: (id, o = {}) => request('POST', ['runs', encodeURIComponent(checkId(id)), 'cancel'], { ...o, body: {} }),

    /** Download an artifact as bytes with the Authorization header (no token in the URL). */
    async downloadArtifact(runId, name, { signal } = {}) {
      let url;
      try {
        url = labUrl(baseUrl, ['runs', encodeURIComponent(checkId(runId)), 'artifacts', encodeURIComponent(checkId(name, ARTIFACT_PATTERN))]);
      } catch {
        return { ok: false, error: { kind: 'bad_request', status: 0, code: null, serverMessage: '', ...ERROR_TEXT.bad_request } };
      }
      let res;
      try {
        res = await doFetch(url, { ...init('GET', { signal }), headers: { ...init('GET').headers, Accept: '*/*' } });
      } catch (e) {
        return { ok: false, error: networkError(e) };
      }
      if (!res.ok) {
        const { text, json } = await readBody(res);
        return { ok: false, error: mapLabError({ status: res.status, headers: res.headers, bodyJson: json, bodyText: text, hadToken: Boolean(getToken()) }) };
      }
      try {
        const blob = await res.blob();
        return { ok: true, status: res.status, blob, contentType: header(res.headers, 'content-type') };
      } catch (e) {
        return { ok: false, error: networkError(e) };
      }
    },

    /** Fetch one JSON artifact and parse it (used for summary.json). */
    async artifactJson(runId, name, { signal } = {}) {
      const r = await this.downloadArtifact(runId, name, { signal });
      if (!r.ok) return r;
      try {
        const text = typeof r.blob.text === 'function' ? await r.blob.text() : String(r.blob);
        return { ok: true, status: r.status, data: JSON.parse(text) };
      } catch {
        return { ok: false, error: { kind: 'unexpected', status: r.status, code: null, serverMessage: '', ...ERROR_TEXT.unexpected } };
      }
    },

    /**
     * One classifying request to the telemetry metrics endpoint (not a lab-1 document). Used to tell "the gateway answered and
     * refused" from "no answer" when the telemetry EventSource is down. Resolves { ok: true, status } or { ok: false, error }.
     * UNVERIFIED: the exact status codes of this endpoint under JWT (docs/design/remediation/06 section 4.4).
     */
    async probe({ signal, path = '/api/v1/metrics?schema=2' } = {}) {
      const url = `${String(baseUrl).replace(/\/+$/, '')}${path}`;
      let res;
      try {
        res = await doFetch(url, init('GET', { signal }));
      } catch (e) {
        return { ok: false, error: networkError(e) };
      }
      if (res.ok) return { ok: true, status: res.status };
      const { text, json } = await readBody(res);
      const mapped = mapLabError({ status: res.status, headers: res.headers, bodyJson: json, bodyText: text, hadToken: Boolean(getToken()) });
      // a 404 here means this route is missing, never "the Lab API is off"
      if (res.status === 404) return { ok: false, error: { ...mapped, kind: 'not_found', ...ERROR_TEXT.not_found } };
      return { ok: false, error: mapped };
    },

    /**
     * Control Center live screens (not the Lab): one authenticated JSON GET of a gateway path such as /api/v1/intelligence/status.
     * Resolves { ok, status, authError, data } and never throws. The Authorization header is the only place the token goes; the result
     * never contains it. status 0 = no answer (network, CORS, abort). `data` is null when the body is not JSON.
     */
    async getJson(path, { signal } = {}) {
      if (!isGatewayPath(path)) return { ok: false, status: 0, authError: null, data: null, aborted: false, bad: true };
      const url = `${String(baseUrl).replace(/\/+$/, '')}${path}`;
      let res;
      try {
        res = await doFetch(url, init('GET', { signal }));
      } catch (e) {
        return { ok: false, status: 0, authError: null, data: null, aborted: Boolean(e && e.name === 'AbortError') };
      }
      const authError = header(res.headers, 'x-auth-error');
      const { json } = await readBody(res);
      return { ok: res.ok, status: res.status, authError: authError ? clipText(authError, 40) : null, data: json, aborted: false };
    },

    /**
     * The live telemetry stream with an Authorization header (EventSource cannot send one). Resolves { ok: true, status, body } on 2xx
     * (the caller reads body), else { ok: false, status, authError } with status 0 for no answer.
     */
    async openStream(path, { signal } = {}) {
      if (!isGatewayPath(path)) return { ok: false, status: 0, authError: null, aborted: false, bad: true };
      const url = `${String(baseUrl).replace(/\/+$/, '')}${path}`;
      let res;
      try {
        res = await doFetch(url, init('GET', { signal, extraHeaders: { Accept: 'text/event-stream' } }));
      } catch (e) {
        return { ok: false, status: 0, authError: null, aborted: Boolean(e && e.name === 'AbortError') };
      }
      if (!res.ok) {
        const authError = header(res.headers, 'x-auth-error');
        return { ok: false, status: res.status, authError: authError ? clipText(authError, 40) : null, aborted: false };
      }
      if (!res.body || typeof res.body.getReader !== 'function') return { ok: false, status: res.status, authError: null, aborted: false, bad: true };
      return { ok: true, status: res.status, authError: null, body: res.body };
    },

    /** Open the SSE response. Resolves with the raw Response on 200; the caller reads res.body. */
    async openEvents(runId, { lastEventId, signal } = {}) {
      let url;
      try {
        url = labUrl(baseUrl, ['runs', encodeURIComponent(checkId(runId)), 'events']);
      } catch {
        return { ok: false, error: { kind: 'bad_request', status: 0, code: null, serverMessage: '', ...ERROR_TEXT.bad_request } };
      }
      const extra = { Accept: 'text/event-stream' };
      if (lastEventId !== undefined && lastEventId !== null && lastEventId !== '') extra['Last-Event-ID'] = String(lastEventId);
      let res;
      try {
        res = await doFetch(url, init('GET', { signal, extraHeaders: extra }));
      } catch (e) {
        return { ok: false, error: networkError(e) };
      }
      if (!res.ok) {
        const { text, json } = await readBody(res);
        return { ok: false, error: mapLabError({ status: res.status, headers: res.headers, bodyJson: json, bodyText: text, hadToken: Boolean(getToken()) }) };
      }
      if (!res.body || typeof res.body.getReader !== 'function') {
        return { ok: false, error: { kind: 'unexpected', status: res.status, code: null, serverMessage: '', ...ERROR_TEXT.unexpected } };
      }
      return { ok: true, status: res.status, body: res.body };
    },

    /**
     * Cache request against the gateway (GET, DELETE, etc.).
     * Does not infer verdicts; returns raw response status, headers, and text for authoritative parsing.
     */
    async cacheFetch(path, { method = 'GET', signal, body } = {}) {
      const url = `${String(baseUrl).replace(/\/+$/, '')}${path.startsWith('/') ? path : `/${path}`}`;
      const t0 = (typeof performance !== 'undefined' ? performance : Date).now();
      let res;
      try {
        res = await doFetch(url, {
          method,
          headers: { Accept: '*/*' },
          body: body ? JSON.stringify(body) : undefined,
          signal,
          credentials: 'omit',
          cache: 'no-store',
          redirect: 'error',
        });
      } catch (e) {
        const elapsed = ((typeof performance !== 'undefined' ? performance : Date).now() - t0);
        return { ok: false, status: 0, headers: new Headers(), bodyText: e.message, elapsedMs: elapsed, error: networkError(e) };
      }
      const elapsed = ((typeof performance !== 'undefined' ? performance : Date).now() - t0);
      const text = await res.text().catch(() => '');
      return {
        ok: res.ok,
        status: res.status,
        headers: res.headers,
        bodyText: text,
        elapsedMs: elapsed,
        error: null,
      };
    },
  };
}

/** Standalone gateway cache fetcher used by InteractiveConsole and TrafficGenerator. */
export function executeGatewayRequest(baseUrl, path, options = {}) {
  const client = createLabClient({ baseUrl });
  return client.cacheFetch(path, options);
}

/**
 * "Test connection" for a CANDIDATE gateway address (nothing is switched). Two requests: capabilities and the metrics probe.
 * No token is sent unless the caller passes getToken (the default sends none). Resolves to
 * { capabilities: { ok, kind, status }, metrics: { ok, kind, status } } where kind is an error kind or null.
 */
export async function testGatewayConnection({ baseUrl, getToken = () => null, fetchImpl, signal } = {}) {
  const client = createLabClient({ baseUrl, getToken, fetchImpl });
  const shape = (r) => (r.ok ? { ok: true, kind: null, status: r.status ?? null } : { ok: false, kind: r.error.kind, status: r.error.status ?? null });
  const caps = shape(await client.capabilities({ signal }));
  const metrics = shape(await client.probe({ signal }));
  return { capabilities: caps, metrics };
}
