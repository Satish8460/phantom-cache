// Catalog of every gateway-supplied value the live screens display.
// A new displayed value needs an entry here (path, label, format, window) - see docs/design/ui_audit.md.
import { subField } from './provenance.js';

const m = (...p) => ['metrics', ...p];
const CUM = 'Cumulative since gateway start';

export const METRICS = Object.freeze({
  requests_total: { path: m('requests_total'), label: 'GET requests', format: 'count', window: CUM },
  cache_hits: { path: m('cache_hits'), label: 'Cache hits', format: 'count', window: CUM },
  cache_misses: { path: m('cache_misses'), label: 'Cache misses', format: 'count', window: CUM },
  coalesced_waiters: { path: m('coalesced_waiters'), label: 'Coalesced waiters', format: 'count', window: CUM },
  evictions_total: { path: m('evictions_total'), label: 'Evictions', format: 'count', window: CUM },
  capacity_rejections: { path: m('capacity_rejections'), label: 'Capacity rejections', format: 'count', window: CUM },
  hit_ratio_cumulative: {
    path: m('hit_ratio_cumulative'), label: 'Hit ratio, cumulative', format: 'ratio', window: CUM,
    note: 'The gateway reports 100% when it has seen no hits or misses yet.',
  },
  hit_ratio_10s: { path: m('hit_ratio_10s'), label: 'Hit ratio, 10 s', format: 'ratio', window: 'Sliding 10 s window' },
  hit_ratio_1s: { path: m('hit_ratio_1s'), label: 'Hit ratio, 1 s', format: 'ratio', window: 'Sliding 1 s window' },
  throughput_req_s_10s: { path: m('throughput_req_s_10s'), label: 'Throughput, 10 s', format: 'rate', window: 'Sliding 10 s window' },
  throughput_req_s_1s: { path: m('throughput_req_s_1s'), label: 'Throughput, 1 s', format: 'rate', window: 'Sliding 1 s window' },

  latency_p50: { pick: (p) => subField(p?.metrics?.latency_ms, 'p50'), label: 'GET latency p50', format: 'ms', window: 'Recent GET observations (gateway ring buffer)' },
  latency_p90: { pick: (p) => subField(p?.metrics?.latency_ms, 'p90'), label: 'GET latency p90', format: 'ms', window: 'Recent GET observations (gateway ring buffer)' },
  latency_p95: { pick: (p) => subField(p?.metrics?.latency_ms, 'p95'), label: 'GET latency p95', format: 'ms', window: 'Recent GET observations (gateway ring buffer)' },
  latency_p99: { pick: (p) => subField(p?.metrics?.latency_ms, 'p99'), label: 'GET latency p99', format: 'ms', window: 'Recent GET observations (gateway ring buffer)' },
  latency_samples: { pick: (p) => subField(p?.metrics?.latency_ms, 'samples'), label: 'Latency samples', format: 'count', window: 'Current ring contents' },

  bytes_resident: { path: m('bytes_resident'), label: 'Resident bytes (logical)', format: 'bytes', window: 'Instant', note: 'Logical accounting, not process memory.' },
  byte_budget: { path: m('byte_budget'), label: 'Byte budget', format: 'bytes', window: 'Configured' },
  byte_budget_rejections: { path: m('byte_budget_rejections'), label: 'Budget rejections', format: 'count', window: CUM },
  oversize_rejections: { path: m('oversize_rejections'), label: 'Oversize rejections', format: 'count', window: CUM },
  rss_bytes: { path: m('process', 'rss_bytes'), label: 'Process working set', format: 'bytes', window: 'Instant', note: 'Working set on Windows, RSS on Linux.' },

  stale_served_total: { path: m('stale_served_total'), label: 'Stale responses served', format: 'count', window: CUM },
  stale_cap_violations: {
    path: m('stale_cap_violations'), label: 'Stale cap violations', format: 'count', window: CUM,
    criticalWhenNonZero: true, note: 'Must be 0. Any other value breaks the hard staleness cap.',
  },
  reval_started: { path: m('revalidations', 'started'), label: 'Revalidations started', format: 'count', window: CUM },
  reval_ok: { path: m('revalidations', 'ok'), label: 'Revalidated, new body', format: 'count', window: CUM },
  reval_not_modified: { path: m('revalidations', 'not_modified'), label: 'Revalidated, not modified', format: 'count', window: CUM },
  reval_error: { path: m('revalidations', 'error'), label: 'Revalidation errors', format: 'count', window: CUM },
  reval_in_flight: { path: m('revalidations', 'in_flight'), label: 'Background refreshes in flight', format: 'count', window: 'Instant' },

  origin_started: { path: m('origin', 'requests_started'), label: 'Origin requests started', format: 'count', window: CUM },
  origin_ok: { path: m('origin', 'requests_completed_ok'), label: 'Origin requests completed OK', format: 'count', window: CUM },
  origin_in_flight: { path: m('origin', 'in_flight'), label: 'Origin requests in flight', format: 'count', window: 'Instant' },
  origin_err_timeout: { path: m('origin', 'errors', 'timeout'), label: 'Origin timeouts', format: 'count', window: CUM },
  origin_err_network: { path: m('origin', 'errors', 'network'), label: 'Origin network errors', format: 'count', window: CUM },
  origin_err_5xx: { path: m('origin', 'errors', 'status_5xx'), label: 'Origin 5xx responses', format: 'count', window: CUM },
  origin_err_other: { path: m('origin', 'errors', 'other'), label: 'Origin other errors', format: 'count', window: CUM },
  origin_latency_p50: { pick: (p) => subField(p?.metrics?.origin?.latency_ms, 'p50'), label: 'Origin latency p50', format: 'ms', window: 'Origin sample window' },
  origin_latency_p95: { pick: (p) => subField(p?.metrics?.origin?.latency_ms, 'p95'), label: 'Origin latency p95', format: 'ms', window: 'Origin sample window' },
  origin_latency_p99: { pick: (p) => subField(p?.metrics?.origin?.latency_ms, 'p99'), label: 'Origin latency p99', format: 'ms', window: 'Origin sample window' },
});

/** Paths (under `metrics.`) already rendered by a dedicated tile, for the "additional fields" listing. */
export function knownMetricPaths() {
  const known = new Set();
  for (const def of Object.values(METRICS)) {
    if (def.path) known.add(def.path.slice(1).join('.'));
  }
  // Objects that carry one src for several members are handled whole.
  known.add('latency_ms');
  known.add('origin.latency_ms');
  return known;
}
