// Client-side history of server-supplied values, for charts only. No new metrics are derived:
// each point is exactly a value the gateway sent (or a gap where it sent none).
import { readField } from './provenance.js';

export const HISTORY_CAP = 120;
export const MIN_SAMPLE_GAP_MS = 500;
export const MAX_SAMPLE_GAP_MS = 3000;

export function emptySeries() {
  return [];
}

/** Returns a new series. Skips the push when called faster than MIN_SAMPLE_GAP_MS. Inserts a null gap point if gap > MAX_SAMPLE_GAP_MS. */
export function pushPoint(series, t, field, { mode = 'live', cap = HISTORY_CAP } = {}) {
  const last = series[series.length - 1];
  if (last && t - last.t < MIN_SAMPLE_GAP_MS) return series;
  const r = readField(field, { mode });
  const point = r.status === 'value' ? { t, v: r.v } : { t, v: null };
  const next = series.slice();

  // If time elapsed since last sample exceeds threshold (e.g. reload or pause), insert gap marker (F9 / PC-AUTO-024)
  if (last && t - last.t > MAX_SAMPLE_GAP_MS) {
    next.push({ t: last.t + 1, v: null });
  }

  next.push(point);
  if (next.length > cap) {
    return next.slice(next.length - cap);
  }
  return next;
}

/** Plot geometry for an SVG polyline set. Gaps (null) split the line into segments. */
export function toSegments(series, width, height, pad = 2) {
  const values = series.filter((p) => p.v !== null).map((p) => p.v);
  if (values.length === 0) return { segments: [], min: null, max: null };
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const n = series.length;
  const x = (i) => (n <= 1 ? width / 2 : pad + (i * (width - 2 * pad)) / (n - 1));
  const y = (v) => height - pad - ((v - min) / span) * (height - 2 * pad);
  const segments = [];
  let cur = [];
  series.forEach((p, i) => {
    if (p.v === null) {
      if (cur.length) segments.push(cur);
      cur = [];
    } else {
      cur.push([Number(x(i).toFixed(1)), Number(y(p.v).toFixed(1))]);
    }
  });
  if (cur.length) segments.push(cur);
  return { segments, min, max };
}
