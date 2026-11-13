import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Load one API resource. `loader(signal)` must resolve to { ok, data } or { ok:false, error } (the Lab client shape).
 * Status: loading | ready | error. A reload keeps the previous data visible (status stays 'ready' with `refreshing`).
 * `deps` restart the load (for example the selected id or a changed token).
 */
export function useLabResource(loader, deps, { enabled = true } = {}) {
  const [state, setState] = useState({ status: enabled ? 'loading' : 'idle', data: null, error: null, refreshing: false });
  const seq = useRef(0);
  const ctlRef = useRef(null);
  const loaderRef = useRef(loader);
  loaderRef.current = loader;

  const load = useCallback(() => {
    const my = ++seq.current;
    if (ctlRef.current) ctlRef.current.abort(); // a reload supersedes (and cancels) the request still in flight
    const ctl = new AbortController();
    ctlRef.current = ctl;
    setState((s) => (s.status === 'ready' ? { ...s, refreshing: true } : { status: 'loading', data: null, error: null, refreshing: false }));
    Promise.resolve(loaderRef.current(ctl.signal)).then((r) => {
      if (my !== seq.current) return;
      if (r.ok) setState({ status: 'ready', data: r.data, error: null, refreshing: false });
      else if (r.error && r.error.kind === 'aborted') return;
      else setState((s) => ({ status: 'error', data: s.status === 'ready' ? s.data : null, error: r.error, refreshing: false }));
    });
    return () => {
      ctl.abort();
    };
  }, []);

  useEffect(() => {
    if (!enabled) {
      setState({ status: 'idle', data: null, error: null, refreshing: false });
      return undefined;
    }
    const cancel = load();
    return () => {
      seq.current += 1;
      cancel();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps]);

  return { ...state, reload: load };
}
