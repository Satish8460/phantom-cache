import { useCallback, useEffect, useRef, useState } from 'react';
import { PATHS, classify, decisionsPath, statusView } from '../lib/intelligenceView.js';

const EMPTY = Object.freeze({ status: null, registry: null, signals: null, decisions: null, at: null });
const ACTIVE_MS = 2000;
const IDLE_MS = 15000;

const slim = (r) => (r && !r.aborted ? { ok: r.ok, status: r.status, authError: r.authError || null, data: r.data } : null);

/**
 * Reads the Phantom Intelligence API (docs/lanes/CONTRACT-pie-api-v1.md) through the signed-in client.
 *  - Always asks /status (once per gateway or token change, then every 15 s) so the navigation can tell whether the screen works.
 *  - While the Intelligence screen is open (`active`) it also reads signals and the newest decisions every 2 s and the registry once.
 * A refusal (401/403) is reported to the sign-in object so the banner and the header chip explain it; other answers stay in `raw`.
 * Nothing is cached across a gateway or token change: the old answers are dropped first.
 */
export function useIntelligence(auth, gatewayUrl, { active = false } = {}) {
  const [raw, setRaw] = useState(EMPTY);
  const [nonce, setNonce] = useState(0);
  const clientRef = useRef(auth.client);
  clientRef.current = auth.client;
  const reportRef = useRef(auth.report);
  reportRef.current = auth.report;
  const activeRef = useRef(active);
  activeRef.current = active;
  const registryDone = useRef(false);

  useEffect(() => {
    setRaw(EMPTY);
    registryDone.current = false;
  }, [gatewayUrl, auth.version]);

  useEffect(() => {
    const ctl = new AbortController();
    let timer = null;
    let cancelled = false;
    const patch = (p) => { if (!cancelled) setRaw((r) => ({ ...r, ...p, at: Date.now() })); };

    const tick = async () => {
      const client = clientRef.current;
      const st = slim(await client.getJson(PATHS.status, { signal: ctl.signal }));
      if (cancelled || !st) return;
      if (st.status === 401 || st.status === 403) reportRef.current({ status: st.status, authError: st.authError, source: 'intelligence' });
      patch({ status: st });
      let next = IDLE_MS;
      if (activeRef.current && classify(st).kind === 'ok') {
        next = ACTIVE_MS;
        const view = statusView(st);
        const retained = view.availability === 'ready' ? view.decisions.retainedCount : null;
        const jobs = [
          client.getJson(PATHS.signals, { signal: ctl.signal }).then((r) => patch({ signals: slim(r) })),
          client.getJson(decisionsPath(retained), { signal: ctl.signal }).then((r) => patch({ decisions: slim(r) })),
        ];
        if (!registryDone.current) {
          jobs.push(client.getJson(PATHS.registry, { signal: ctl.signal }).then((r) => { const s = slim(r); if (s) { registryDone.current = true; patch({ registry: s }); } }));
        }
        await Promise.all(jobs);
      }
      if (!cancelled) timer = setTimeout(tick, next);
    };
    tick();
    const onHide = () => { cancelled = true; if (timer) clearTimeout(timer); ctl.abort(); };
    window.addEventListener('pagehide', onHide);
    return () => {
      cancelled = true;
      window.removeEventListener('pagehide', onHide);
      if (timer) clearTimeout(timer);
      ctl.abort();
    };
  }, [gatewayUrl, auth.version, auth.client, nonce, active]);

  const refresh = useCallback(() => {
    registryDone.current = false;
    setNonce((n) => n + 1);
  }, []);

  return { raw, refresh };
}
