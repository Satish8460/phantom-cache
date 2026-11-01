import { createContext, useContext } from 'react';

// value: { payload, conn: {kind, ageMs, hasData}, state, series: {throughput, hitRatio, p99}, gateway, reconnect, offsetMs }
export const TelemetryContext = createContext(null);

export function useTelemetryContext() {
  const v = useContext(TelemetryContext);
  if (!v) throw new Error('TelemetryContext missing');
  return v;
}
