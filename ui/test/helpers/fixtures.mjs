// Test fixtures. These numbers are TEST DATA ONLY: they live under ui/test and never ship in the bundle.
import { connectionReducer, deriveConnection, initialConnection } from '../../src/lib/connection.js';

const m = (v, unit = 'count') => ({ src: 'measured', v, unit });
const d = (v, unit) => ({ src: 'derived', v, unit });
const na = (reason, unit) => (unit ? { src: 'unavailable', reason, unit } : { src: 'unavailable', reason });

export function livePayload(overrides = {}) {
  return {
    schema: 2,
    mode: 'live',
    as_of_ms: 1_730_200_000_000,
    uptime_ms: 3_725_000,
    capabilities: { cluster: 'disabled', controller: 'none', origin_metrics: 'available' },
    metrics: {
      requests_total: m(500),
      cache_hits: m(450),
      cache_misses: m(50),
      coalesced_waiters: m(7),
      evictions_total: m(3),
      capacity_rejections: m(0),
      hit_ratio_cumulative: d(0.9, 'ratio'),
      hit_ratio_10s: d(0.875, 'ratio'),
      hit_ratio_1s: na('insufficient_window', 'ratio'),
      throughput_req_s_10s: d(1234.5, 'req/s'),
      throughput_req_s_1s: d(1301, 'req/s'),
      latency_ms: { src: 'measured', unit: 'ms', resolution: 'microsecond', p50: 0.412, p90: 1.9, p95: 3.2, p99: 11.75, samples: 500 },
      bytes_resident: m(1_048_576, 'bytes'),
      byte_budget: m(8_388_608, 'bytes'),
      byte_budget_rejections: m(0),
      oversize_rejections: m(0),
      stale_served_total: m(12),
      stale_cap_violations: m(0),
      revalidations: { started: m(10), ok: m(4), not_modified: m(5), error: m(1), in_flight: m(0) },
      origin: {
        requests_started: m(50),
        requests_completed_ok: m(49),
        in_flight: m(1),
        errors: { timeout: m(0), network: m(0), status_5xx: m(1), other: m(0) },
        latency_ms: { src: 'derived', unit: 'ms', p50: 12, p95: 48, p99: 95 },
      },
      process: { rss_bytes: na('unsupported_platform', 'bytes') },
    },
    hot_keys: na('not_implemented'),
    nodes: [{ node_id: 1, state: 'Active', state_src: 'simulated', cpu: na('not_measured'), memory: na('not_measured') }],
    ...overrides,
  };
}

/** Build the context value screens consume, by driving the real reducer. */
export function makeCtx({ frames = [], phase = null, now = 1_000_000, receivedAt = now, gatewayUrl = 'http://localhost:8085' } = {}) {
  let state = initialConnection;
  for (const f of frames) {
    state = connectionReducer(state, { type: 'message', text: typeof f === 'string' ? f : JSON.stringify(f), now: receivedAt });
  }
  if (phase === 'error') state = connectionReducer(state, { type: 'error', closed: false });
  if (phase === 'closed') state = connectionReducer(state, { type: 'error', closed: true });
  const conn = deriveConnection(state, now);
  const empty = [];
  return {
    payload: state.payload,
    state,
    conn,
    series: { throughput: empty, hitRatio: empty, p99: empty },
    gateway: { url: gatewayUrl, source: 'default' },
    reconnect: () => {},
    offsetMs: null,
  };
}
