// View model of the Cluster screen (batch B2, WP11). Pure functions: no DOM, no React, no clock.
//
// Source: `metrics.cluster` and `metrics.outcomes` of the schema-2 telemetry frame (src/api/server.rs). Every value that is
// shown carries the provenance the GATEWAY stated (`measured`, `inferred`, `unavailable`); this module never upgrades or invents
// one, and never turns a missing or rejected value into a zero. Values of a class this screen does not draw (`simulated` in live
// mode, unknown classes, wrong types) are withheld and shown as unavailable with the reason.
import { isKnownSrc, readField, reasonText } from './provenance.js';

const isObject = (x) => x !== null && typeof x === 'object' && !Array.isArray(x);

const MAX_PEERS = 64;
const MAX_LABEL = 40;

/** A numeric `{src, v, unit}` field -> `{ text, src, reason }` (text null when withheld). */
export function numberCell(field, mode) {
  const r = readField(field, { mode });
  if (r.status === 'value' && typeof r.v === 'number') {
    return { text: r.v.toLocaleString('en-US'), src: r.src, reason: null };
  }
  return { text: null, src: 'unavailable', reason: reasonText(r.reason) };
}

/** A string `{src, v}` field (a node state or a reachability label) -> `{ text, src, reason }`. */
export function labelCell(field, mode) {
  if (!isObject(field)) return { text: null, src: 'unavailable', reason: reasonText('not_in_payload') };
  if (!isKnownSrc(field.src)) return { text: null, src: 'unavailable', reason: reasonText('unknown_provenance') };
  if (field.src === 'unavailable') {
    return Object.prototype.hasOwnProperty.call(field, 'v')
      ? { text: null, src: 'unavailable', reason: reasonText('unavailable_with_value') }
      : { text: null, src: 'unavailable', reason: reasonText(field.reason) };
  }
  if (field.src === 'simulated' && mode === 'live') return { text: null, src: 'unavailable', reason: reasonText('simulated_in_live') };
  if (typeof field.v !== 'string' || field.v.length === 0 || field.v.length > MAX_LABEL) {
    return { text: null, src: 'unavailable', reason: 'Rejected: value is not a short text label' };
  }
  return { text: field.v, src: field.src, reason: null };
}

/** Routing state words the gateway can report; anything else is shown as received (text) but never styled as known. */
export const KNOWN_STATES = Object.freeze(['Active', 'Joining', 'Degraded', 'Suspect', 'Recovering', 'Failed', 'Offline']);
export const KNOWN_REACHABILITY = Object.freeze(['reachable', 'degraded', 'unreachable']);

const BROADCASTER_ROWS = Object.freeze([
  { id: 'enqueued', label: 'Enqueued', meaning: 'Invalidation tasks accepted into the fan-out queue (one per DELETE).' },
  { id: 'sent', label: 'Sent', meaning: 'Invalidation frames attempted, including retries.' },
  { id: 'retried', label: 'Retried', meaning: 'Attempts that were not the first for their delivery.' },
  { id: 'acked', label: 'Acknowledged', meaning: 'Deliveries a peer acknowledged.' },
  { id: 'dropped', label: 'Dropped (queue overflow)', meaning: 'Deliveries refused because a queue was full. Should stay 0.' },
  { id: 'exhausted', label: 'Gave up', meaning: 'Deliveries that gave up after all attempts or because the destination is unhealthy; the peer may still hold the entry.' },
]);

/** The five outcomes of a cache GET, in the words of the `X-Cache-Source` response header. */
const OUTCOME_ROWS = Object.freeze([
  { id: 'local', label: 'local', meaning: 'Served from this gateway’s own cache.' },
  { id: 'peer', label: 'peer', meaning: 'Not cached here; read from the key’s primary peer and stored. No origin request.' },
  { id: 'origin', label: 'origin', meaning: 'Fetched from the origin (including bypass requests).' },
  { id: 'coalesced', label: 'coalesced', meaning: 'Waited for another request’s fetch of the same key.' },
  { id: 'stale', label: 'stale', meaning: 'Served stale while it refreshes, or because the origin failed.' },
]);

const PEER_READ_ROWS = Object.freeze([
  { id: 'miss', label: 'Peer had nothing fresh', meaning: 'The primary peer answered without an entry; the request went to the origin.' },
  { id: 'failed', label: 'Peer unreachable or timed out', meaning: 'The peer read failed; the request went to the origin.' },
  { id: 'refused_format', label: 'Peer spoke another format', meaning: 'A mixed-version cluster: the answer was refused, the request went to the origin.' },
  { id: 'rejected_lifetime', label: 'Peer lifetime rejected', meaning: 'A peer sent an absurd age or lifetime; the entry was ignored.' },
]);

function rowsFrom(group, defs, mode) {
  const g = isObject(group) ? group : null;
  return defs.map((d) => ({ ...d, cell: numberCell(g ? g[d.id] : undefined, mode) }));
}

/**
 * @returns {{present:false} | {present:true, mode:string, local:object, peers:object[], broadcaster:object[], outcomes:object[], peerReads:object[], truncated:boolean}}
 * `present` is false when the gateway does not report a cluster block (single node, or an older gateway): the screen then
 * falls back to the capability statement instead of drawing anything.
 */
export function clusterView(payload) {
  const c = payload && payload.metrics ? payload.metrics.cluster : undefined;
  if (!isObject(c) || !isObject(c.broadcaster)) return { present: false };
  const mode = payload.mode === 'simulation' ? 'simulation' : 'live';

  let local;
  if (isObject(c.local) && c.local.node_id !== undefined && isObject(c.local.state)) {
    local = { available: true, id: String(c.local.node_id), state: labelCell(c.local.state, mode) };
  } else {
    local = { available: false, reason: reasonText(isObject(c.local) ? c.local.reason : undefined) };
  }

  const rawPeers = Array.isArray(c.peers) ? c.peers : [];
  const peers = rawPeers.slice(0, MAX_PEERS).map((p, i) => ({
    key: `${p && p.node_id !== undefined ? p.node_id : 'n'}-${i}`,
    id: p && p.node_id !== undefined ? String(p.node_id) : 'unknown',
    state: labelCell(p && p.state, mode),
    reachability: labelCell(p && p.reachability, mode),
    failedAttempts: numberCell(p && p.consecutive_failed_attempts, mode),
    idle: numberCell(p && p.idle_connections, mode),
    active: numberCell(p && p.active_connections, mode),
  }));

  const outcomes = payload.metrics.outcomes;
  return {
    present: true,
    mode,
    local,
    peers,
    truncated: rawPeers.length > MAX_PEERS,
    broadcaster: rowsFrom(c.broadcaster, BROADCASTER_ROWS, mode),
    outcomes: rowsFrom(isObject(outcomes) ? outcomes.responses : null, OUTCOME_ROWS, mode),
    peerReads: rowsFrom(isObject(outcomes) ? outcomes.peer_reads_without_hit : null, PEER_READ_ROWS, mode),
  };
}
