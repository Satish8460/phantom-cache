// Live telemetry through fetch streaming, used when a bearer token is set (a browser EventSource cannot send an Authorization header;
// docs/design/sse_auth_options.md option D). Plain async function with an injected client, so it is unit tested without a browser.
//
// It reports the gateway's status code and X-Auth-Error word (never the token) so the page can tell "refused" from "no answer".

import { createSseParser } from './sse.js';

export const TELEMETRY_PATH = '/api/events?schema=2';

/** 401 and 403 cannot succeed again without a person changing the token: the caller must not retry on a timer. */
export const isAuthRefusal = (status) => status === 401 || status === 403;

/**
 * @param {{client: {openStream: Function}, signal: AbortSignal, onOpen: (status: number) => void, onText: (text: string) => void,
 *          onEnd: (info: {status: number, authError: string|null, ended: boolean}) => void}} o
 * onEnd is called exactly once unless the signal was aborted (then never): ended=true when an open stream finished or broke.
 */
export async function runTelemetryStream({ client, signal, onOpen, onText, onEnd, path = TELEMETRY_PATH }) {
  const res = await client.openStream(path, { signal });
  if (signal && signal.aborted) return;
  if (!res.ok) {
    onEnd({ status: res.status, authError: res.authError || null, ended: false });
    return;
  }
  onOpen(res.status);
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const parser = createSseParser();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (signal && signal.aborted) return;
      if (done) break;
      const chunk = typeof value === 'string' ? value : decoder.decode(value, { stream: true });
      for (const item of parser.push(chunk)) if (item.type === 'event') onText(item.data);
    }
  } catch (e) {
    if (signal && signal.aborted) return;
    if (!(e && e.name === 'AbortError')) {
      onEnd({ status: 0, authError: null, ended: true });
      return;
    }
    return;
  } finally {
    try {
      reader.releaseLock();
    } catch {
      /* already released */
    }
  }
  onEnd({ status: res.status, authError: null, ended: true });
}
