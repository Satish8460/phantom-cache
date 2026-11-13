import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { backoffMs, clockOffsetMs, connectionReducer, deriveConnection, initialConnection } from '../lib/connection.js';
import { eventsUrl } from '../lib/config.js';
import { isAuthRefusal, runTelemetryStream } from '../lib/telemetryStream.js';

/**
 * Subscribes to GET /api/events?schema=2 and exposes connection state.
 * The page, not the browser, owns every reconnect (AD-10): each error closes the EventSource and a capped exponential backoff
 * (1 s doubling to 30 s) schedules the next attempt; `retryScheduled` records the time so the UI can show a real countdown.
 *
 * `feed` (from useGatewayAuth) carries the request client and whether a bearer token is set. With a token the stream is read through fetch
 * (an EventSource cannot send an Authorization header); without one it is an EventSource and a refusal is found by one rate-limited probe.
 * A 401 or 403 stops the timer: it cannot succeed until the token changes (the effect restarts then).
 */
export function useTelemetry(gatewayUrl, feed = null) {
  const [state, dispatch] = useReducer(connectionReducer, initialConnection);
  const [now, setNow] = useState(() => Date.now());
  const [nonce, setNonce] = useState(0);
  const attemptRef = useRef(0);
  const lastUrlRef = useRef(null);
  const lastFeedVersionRef = useRef(null);
  const hasToken = Boolean(feed && feed.hasToken);
  const feedVersion = feed ? feed.version : 0;
  const feedRef = useRef(feed);
  feedRef.current = feed;

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!hasToken && typeof EventSource === 'undefined') {
      dispatch({ type: 'error', closed: true });
      return undefined;
    }
    let es = null;
    let timer = null;
    let cancelled = false;
    // A different gateway address starts from nothing: the old gateway's values must never appear under the new one.
    // Retry now (same address) keeps the last values, shown as stale, until the new connection delivers a frame.
    if (lastUrlRef.current !== null && lastUrlRef.current !== gatewayUrl) {
      attemptRef.current = 0;
      dispatch({ type: 'reset' });
    }
    lastUrlRef.current = gatewayUrl;
    // A different credential (set, replaced or forgotten) also starts from nothing: values read under the old one must not stay on screen.
    if (lastFeedVersionRef.current !== null && lastFeedVersionRef.current !== feedVersion) {
      attemptRef.current = 0;
      dispatch({ type: 'reset' });
    }
    lastFeedVersionRef.current = feedVersion;

    if (hasToken) {
      // Authenticated mode: fetch streaming with the Authorization header. Same reducer, same backoff, same page-owned timer.
      const ctl = new AbortController();
      const report = (r) => { if (!cancelled && feedRef.current) feedRef.current.report({ ...r, source: 'stream' }); };
      const openFetch = () => {
        if (cancelled) return;
        dispatch({ type: 'connecting' });
        runTelemetryStream({
          client: feedRef.current.client,
          signal: ctl.signal,
          onOpen: (status) => { dispatch({ type: 'open' }); report({ status, authError: null }); },
          onText: (text) => { attemptRef.current = 0; dispatch({ type: 'message', text, now: Date.now() }); },
          onEnd: ({ status, authError }) => {
            if (cancelled) return;
            dispatch({ type: 'error', closed: true, now: Date.now() });
            if (status !== 200) report({ status, authError });
            if (isAuthRefusal(status)) return; // needs a person: the token effect restarts this
            const delay = backoffMs(attemptRef.current);
            attemptRef.current += 1;
            dispatch({ type: 'retryScheduled', at: Date.now() + delay });
            timer = setTimeout(openFetch, delay);
          },
        });
      };
      openFetch();
      const onHide = () => { if (timer) clearTimeout(timer); timer = null; ctl.abort(); };
      const onShow = (ev) => { if (ev.persisted && !cancelled && ctl.signal.aborted) setNonce((n) => n + 1); };
      window.addEventListener('pagehide', onHide);
      window.addEventListener('pageshow', onShow);
      return () => {
        cancelled = true;
        window.removeEventListener('pagehide', onHide);
        window.removeEventListener('pageshow', onShow);
        if (timer) clearTimeout(timer);
        ctl.abort();
      };
    }

    let lastProbeAt = 0;
    const probe = () => {
      const f = feedRef.current;
      const t = Date.now();
      if (!f || t - lastProbeAt < 15000) return;
      lastProbeAt = t;
      // An EventSource error carries no status. One plain request (no token set) tells "refused" from "no answer".
      f.client.getJson('/api/v1/metrics?schema=2').then((r) => { if (!cancelled && !r.aborted) f.report({ status: r.status, authError: r.authError, source: 'probe' }); });
    };

    const open = () => {
      dispatch({ type: 'connecting' });
      es = new EventSource(eventsUrl(gatewayUrl));
      es.onopen = () => {
        dispatch({ type: 'open' });
        if (feedRef.current) feedRef.current.report({ status: 200, authError: null, source: 'stream' });
      };
      es.onmessage = (ev) => {
        attemptRef.current = 0;
        dispatch({ type: 'message', text: ev.data, now: Date.now() });
      };
      es.onerror = () => {
        // AD-10 (owner-approved; FLIB D-7): the page owns EVERY retry. The browser's own EventSource reconnect hides its schedule, so the
        // stream is closed on every error and reopened by this timer, which makes the countdown shown to the person a true statement.
        // The reducer records `closed: true`, so linkState reports retryOwner 'page'.
        dispatch({ type: 'error', closed: true, now: Date.now() });
        if (cancelled) return;
        es.close();
        probe();
        const delay = backoffMs(attemptRef.current);
        attemptRef.current += 1;
        dispatch({ type: 'retryScheduled', at: Date.now() + delay });
        // A 401 or 403 found by the probe cannot succeed on a timer: wait for a new token or the person's Retry now.
        timer = setTimeout(() => { if (!(feedRef.current && feedRef.current.refused)) open(); }, delay);
      };
    };
    open();

    // Release the connection as soon as the document is hidden for unload or the back/forward cache (a held
    // event-stream socket counts against the browser's 6-per-host limit); reopen if the page is restored.
    const onPageHide = () => {
      if (timer) clearTimeout(timer);
      timer = null;
      if (es) es.close();
      es = null;
    };
    const onPageShow = (ev) => {
      if (ev.persisted && !cancelled && es === null) open();
    };
    window.addEventListener('pagehide', onPageHide);
    window.addEventListener('pageshow', onPageShow);

    return () => {
      cancelled = true;
      window.removeEventListener('pagehide', onPageHide);
      window.removeEventListener('pageshow', onPageShow);
      if (timer) clearTimeout(timer);
      if (es) es.close();
    };
  }, [gatewayUrl, nonce, hasToken, feedVersion]);

  const reconnect = useCallback(() => {
    attemptRef.current = 0;
    setNonce((n) => n + 1);
  }, []);

  const conn = useMemo(() => deriveConnection(state, now), [state, now]);
  const offsetMs = clockOffsetMs(state.payload, state.receivedAt);

  return { state, conn, now, reconnect, offsetMs };
}
