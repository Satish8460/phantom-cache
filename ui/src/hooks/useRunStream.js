import { useCallback, useEffect, useReducer, useRef, useState } from 'react';
import { applyRunEvent, deriveStreamStatus, initialRunModel, initialStream, runStream, streamReducer } from '../lib/labStream.js';

function sleep(ms, signal) {
  return new Promise((resolve) => {
    const t = setTimeout(resolve, ms);
    if (signal) signal.addEventListener('abort', () => { clearTimeout(t); resolve(); }, { once: true });
  });
}

/**
 * Watch one run through fetch streaming with an Authorization header (docs/design/sse_auth_options.md option D).

 * The caller remounts this hook per run id (key), so state never leaks between runs.
 * Reconnects with Last-Event-ID and backoff; `reconnectNow()` restarts immediately (also after a terminal failure).
 */
export function useRunStream(client, runId, { enabled = true, staleAfterMs } = {}) {
  const [stream, dispatchStream] = useReducer(streamReducer, initialStream);
  const [model, setModel] = useState(initialRunModel);
  const [epoch, setEpoch] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const lastId = useRef(null);
  lastId.current = stream.lastEventId;
  // Once the gateway has sent `done` the log is complete: never reconnect (a new client object must not replay it forever).
  const endedRef = useRef(false);
  endedRef.current = stream.phase === 'ended';

  useEffect(() => {
    if (!enabled || !runId || endedRef.current) return undefined;
    const ctl = new AbortController();
    // Nothing from an aborted run may reach the reducer: in development StrictMode starts, aborts and restarts the
    // effect, and a late "stop" from the first run must not overwrite the state of the second.
    const guarded = (fn) => (arg) => { if (!ctl.signal.aborted) fn(arg); };
    runStream({
      client,
      runId,
      signal: ctl.signal,
      dispatch: guarded(dispatchStream),
      onEvent: guarded((ev) => setModel((m) => applyRunEvent(m, ev))),
      getLastEventId: () => lastId.current,
      sleep,
      now: Date.now,
      shouldContinue: () => !endedRef.current,
    });
    // Abort the fetch stream when the document is hidden for unload or the back/forward cache; a restored page
    // (pageshow persisted) restarts it through the epoch.
    const onPageHide = () => ctl.abort();
    const onPageShow = (ev) => { if (ev.persisted && ctl.signal.aborted && !endedRef.current) setEpoch((e) => e + 1); };
    window.addEventListener('pagehide', onPageHide);
    window.addEventListener('pageshow', onPageShow);
    return () => {
      window.removeEventListener('pagehide', onPageHide);
      window.removeEventListener('pageshow', onPageShow);
      ctl.abort();
      dispatchStream({ type: 'stop' });
    };
    // `client` changes when the token changes: the stream restarts with the new credential.
  }, [client, runId, enabled, epoch]);

  // Re-derive "stale" without events: a one second tick while the stream is open.
  useEffect(() => {
    if (stream.phase !== 'open') return undefined;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [stream.phase]);

  const reconnectNow = useCallback(() => setEpoch((e) => e + 1), []);
  return { stream, model, status: deriveStreamStatus(stream, now, { staleAfterMs }), reconnectNow };
}
