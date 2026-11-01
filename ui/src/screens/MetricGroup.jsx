import React from 'react';
import { Metric } from '../components/Metric.jsx';
import { metricViews } from '../lib/viewmodel.js';

/*
 * Page-level metric helpers. They reuse the shared Metric tile and the viewmodel, and correct two texts that the lib still carries
 * and this increment may not edit (recorded as dependencies for the lead):
 *   - catalog.js says the gateway "reports 100% when it has seen no hits or misses yet". The gateway (server.rs:540-556) now reports the
 *     ratio as UNAVAILABLE with a reason (no_requests, no_cache_lookups, insufficient_window), so that note is withheld here;
 *   - provenance.js has no sentence for no_requests / no_cache_lookups.
 */
const REASONS = Object.freeze({
  no_requests: 'No requests have been received yet',
  no_cache_lookups: 'Requests were counted, but none has been counted as a hit or a miss yet',
});

export function fixView(v) {
  let out = v;
  if (out.id === 'hit_ratio_cumulative' && out.note) out = { ...out, note: undefined };
  if (out.reason && REASONS[out.reason]) out = { ...out, reasonText: REASONS[out.reason] };
  return out;
}

export function metricViewsFixed(payload, ids, connKind) {
  return metricViews(payload, ids, connKind).map(fixView);
}

export default function MetricGroup({ payload, ids, connKind, label }) {
  return (
    <dl className="metric-grid" aria-label={label}>
      {metricViewsFixed(payload, ids, connKind).map((v) => <Metric key={v.id} view={v} />)}
    </dl>
  );
}
