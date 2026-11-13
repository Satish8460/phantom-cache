import { useMemo, useSyncExternalStore } from 'react';
import { createGatewayStore } from '../lib/gatewayConfig.js';

/**
 * Build the page's gateway store once (App level). `location` and `envValue` are passed in so this stays testable;
 * a missing window (server render) simply resolves to the build value or the built-in default. Storage is read lazily and
 * every access is wrapped in gatewayConfig.js.
 */
export function useGatewayStore({ location = null, envValue, getStorage } = {}) {
  return useMemo(
    () => createGatewayStore({
      search: location ? location.search : '',
      envValue,
      pageOrigin: location ? location.origin : null,
      pageProtocol: location ? location.protocol : null,
      ...(getStorage ? { getStorage } : {}),
    }),
    // The store is created once per page load; address changes go through store.apply().
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
}

/** Subscribe to the gateway store. Returns the immutable snapshot {url, source, notes, status, remembered, epoch, ...}. */
export function useGateway(store) {
  return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getServerSnapshot);
}
