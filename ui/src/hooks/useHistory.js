import { useEffect, useRef, useState } from 'react';
import { emptySeries, pushPoint } from '../lib/history.js';
import { METRICS } from '../lib/catalog.js';
import { fieldFor } from '../lib/viewmodel.js';
import { readTelemetrySeries, saveTelemetrySeries } from '../lib/gatewayConfig.js';

const SERIES_DEFS = {
  throughput: METRICS.throughput_req_s_1s,
  hitRatio: METRICS.hit_ratio_1s,
  p99: METRICS.latency_p99,
};

function freshSeries() {
  return { throughput: emptySeries(), hitRatio: emptySeries(), p99: emptySeries() };
}

/**
 * Keeps a short per-page history of server-supplied values. Appends once per received frame.
 * Persists series across reloads via gatewayConfig (F9 / WP5). Does NOT wipe on initial null payload.
 */
export function useHistory(payload, receivedAt, gatewayUrl = '') {
  const lastGatewayRef = useRef(gatewayUrl);
  const [series, setSeries] = useState(() => {
    const stored = readTelemetrySeries(gatewayUrl);
    return stored || freshSeries();
  });

  // Gateway URL change: drop old gateway's series
  useEffect(() => {
    if (gatewayUrl && lastGatewayRef.current && lastGatewayRef.current !== gatewayUrl) {
      lastGatewayRef.current = gatewayUrl;
      const fresh = freshSeries();
      setSeries(fresh);
      saveTelemetrySeries(gatewayUrl, fresh);
    } else if (gatewayUrl) {
      lastGatewayRef.current = gatewayUrl;
    }
  }, [gatewayUrl]);

  useEffect(() => {
    if (!payload || receivedAt === null) {
      // Do not clear stored series on transient initial null payload or disconnected state (F9 fix)
      return;
    }
    setSeries((prev) => {
      const next = {};
      let changed = false;
      for (const [k, def] of Object.entries(SERIES_DEFS)) {
        next[k] = pushPoint(prev[k], receivedAt, fieldFor(payload, def), { mode: payload.mode });
        if (next[k] !== prev[k]) changed = true;
      }
      if (changed) {
        if (gatewayUrl) {
          saveTelemetrySeries(gatewayUrl, next);
        }
        return next;
      }
      return prev;
    });
  }, [payload, receivedAt, gatewayUrl]);

  return series;
}
