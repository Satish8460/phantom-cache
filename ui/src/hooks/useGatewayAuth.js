import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { createLabClient } from '../lib/labApi.js';
import { createTokenSession } from '../lib/tokenSession.js';
import {
  HANDOFF_REQUEST_TYPE, chipView, decodeClaims, describeAnswer, evaluateHandoff, expiryState, normalizeTokenInput, INPUT_PROBLEM_TEXT,
} from '../lib/gatewayAuth.js';

/**
 * One sign-in object for the whole Control Center (App level). It owns the token holder, the request client used by every live
 * request (telemetry stream, probes, the Intelligence screen) and the plain-words answer of the gateway.
 *
 * What it exposes is display data only: whether a token is set, the decoded claims (no token string), the expiry, the latest answer.
 * The token itself stays inside the holder and inside the client's Authorization header.
 */
export function useGatewayAuth(gatewayUrl, { windowRef = typeof window !== 'undefined' ? window : null, now = Date.now } = {}) {
  const session = useMemo(() => createTokenSession(), []);
  const snap = useSyncExternalStore(session.subscribe, session.getSnapshot, session.getSnapshot);
  const [answer, setAnswerState] = useState(null);
  const [problem, setProblem] = useState(null);
  const [pending, setPending] = useState(null); // a hand-off from a non-loopback origin awaiting a click: { from }
  const pendingToken = useRef(null);
  const [tick, setTick] = useState(() => now());

  const origin = gatewayUrl;
  const client = useMemo(() => createLabClient({ baseUrl: gatewayUrl, getToken: () => session.tokenFor(gatewayUrl) }), [gatewayUrl, session]);
  const hasToken = session.tokenFor(origin) !== null;
  void snap.version; // re-render when the holder changes

  // A different gateway or a different token starts with no answer: the old one must not be shown for the new situation.
  useEffect(() => {
    setAnswerState(null);
    setProblem(null);
  }, [gatewayUrl, snap.version]);

  const decoded = useMemo(() => {
    const t = session.tokenFor(origin);
    return t ? decodeClaims(t) : null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, origin, snap.version]);

  useEffect(() => {
    if (!hasToken) return undefined;
    const id = setInterval(() => setTick(now()), 1000);
    return () => clearInterval(id);
  }, [hasToken, now]);

  const exp = useMemo(() => expiryState(decoded && decoded.ok ? decoded.claims : null, tick), [decoded, tick]);

  const report = useCallback((r) => {
    if (!r) return;
    setAnswerState((prev) => {
      const view = describeAnswer({ status: r.status, authError: r.authError || null, hadToken: session.tokenFor(gatewayUrl) !== null });
      const next = { ...view, status: r.status, source: r.source || 'request' };
      return prev && prev.key === next.key && prev.status === next.status && prev.source === next.source ? prev : next;
    });
  }, [session, gatewayUrl]);

  const accept = useCallback((text, { keep = false } = {}) => {
    const n = normalizeTokenInput(text);
    if (!n.ok) {
      setProblem(INPUT_PROBLEM_TEXT[n.reason] || INPUT_PROBLEM_TEXT.not_jwt_shape);
      return false;
    }
    session.set(gatewayUrl, n.token, { keep });
    return true;
  }, [session, gatewayUrl]);

  const forget = useCallback(() => {
    session.forget(gatewayUrl);
    setPending(null);
    pendingToken.current = null;
  }, [session, gatewayUrl]);

  // ---- one-time hand-off from the opener window (never a URL) ----
  useEffect(() => {
    const w = windowRef;
    if (!w || typeof w.addEventListener !== 'function') return undefined;
    const onMessage = (ev) => {
      const r = evaluateHandoff({
        origin: ev.origin,
        sourceIsOpener: Boolean(w.opener) && ev.source === w.opener,
        data: ev.data,
        gatewayUrl,
        pageOrigin: w.location ? w.location.origin : null,
      });
      if (r.action === 'apply') {
        session.set(gatewayUrl, r.token, { keep: false });
      } else if (r.action === 'confirm') {
        pendingToken.current = r.token;
        setPending({ from: r.from });
      }
    };
    w.addEventListener('message', onMessage);
    // Ask the opener (if any) once per gateway. The request contains no secret.
    try {
      if (w.opener && !w.opener.closed && session.tokenFor(gatewayUrl) === null) w.opener.postMessage({ type: HANDOFF_REQUEST_TYPE, v: 1 }, '*');
    } catch {
      /* a closed or cross-origin-isolated opener: nothing to ask */
    }
    return () => w.removeEventListener('message', onMessage);
  }, [windowRef, gatewayUrl, session]);

  const confirmHandoff = useCallback(() => {
    if (pendingToken.current) session.set(gatewayUrl, pendingToken.current, { keep: false });
    pendingToken.current = null;
    setPending(null);
  }, [session, gatewayUrl]);
  const dismissHandoff = useCallback(() => {
    pendingToken.current = null;
    setPending(null);
  }, []);

  // The one classified answer the link vocabulary accepts (lib/linkState.js: only an answer from the gateway can make telemetry Unauthorised).
  const KIND_OF_ANSWER = { requires_token: 'unauthorized_missing', expired: 'unauthorized_expired', rejected: 'unauthorized_invalid', wrong_issuer: 'unauthorized_invalid', wrong_audience: 'unauthorized_invalid', forbidden: 'forbidden_role', no_tenant: 'forbidden_role' };
  const refused = Boolean(answer) && Boolean(KIND_OF_ANSWER[answer.key]) && (answer.status === 401 || answer.status === 403);
  const probeKind = refused ? KIND_OF_ANSWER[answer.key] : null;
  const linkProbe = useMemo(() => (probeKind ? { kind: probeKind, withToken: hasToken } : null), [probeKind, hasToken]);
  const requiresToken = !hasToken && Boolean(answer) && answer.key === 'requires_token';
  const chip = chipView({ hasToken, decoded, exp, answer, requiresToken });

  return {
    hasToken,
    decoded, // { ok, claims } | { ok:false, reason } | null  (no token string)
    exp,
    answer,
    requiresToken,
    chip,
    problem,
    keptForTab: session.isKept(gatewayUrl),
    storageProblem: snap.storageProblem,
    handoffPending: pending,
    client,
    version: snap.version,
    report,
    accept,
    forget,
    confirmHandoff,
    dismissHandoff,
    clearProblem: () => setProblem(null),
    // what useTelemetry needs; no token inside
    linkProbe,
    feed: { client, hasToken, version: snap.version, report, refused },
  };
}
