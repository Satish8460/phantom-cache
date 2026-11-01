import { createContext, useContext } from 'react';

// value: { client, baseUrl, hasToken, setToken(token), clearToken(), caps: resource, capsView, reloadCaps() }
// The token itself is held in a React ref inside LabProvider and never exposed through this context.
export const LabContext = createContext(null);

export function useLab() {
  const v = useContext(LabContext);
  if (!v) throw new Error('LabContext missing');
  return v;
}
