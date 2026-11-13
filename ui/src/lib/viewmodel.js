// Turns (payload, connection) into render-ready descriptions. All state-rendering decisions live here
// so they can be tested without a DOM.
import { METRICS, knownMetricPaths } from './catalog.js';
import { FORMATTERS, unitSuffix } from './format.js';
import { flattenProvenanceFields, readField, reasonText, subField } from './provenance.js';

function getPath(obj, path) {
  let cur = obj;
  for (const k of path) {
    if (cur === null || typeof cur !== 'object') return undefined;
    cur = cur[k];
  }
  return cur;
}

export function fieldFor(payload, def) {
  if (!payload) return undefined;
  return def.pick ? def.pick(payload) : getPath(payload, def.path);
}

/**
 * state:
 *   loading      no payload has ever arrived
 *   value        fresh trustworthy value
 *   stale        value from the last frame while the stream is stale/down
 *   unavailable  gateway said unavailable (reason shown; never rendered as 0)
 *   missing      gateway did not send the field at all
 *   rejected     contract violation; value withheld
 */
export function metricView(payload, def, connKind) {
  const base = { label: def.label, windowText: def.window, note: def.note, format: def.format };
  if (!payload) return { ...base, state: 'loading', src: 'unavailable', text: null, reason: 'no_data_yet', reasonText: 'Waiting for the first frame' };
  const r = readField(fieldFor(payload, def), { mode: payload.mode });
  if (r.status !== 'value') {
    return { ...base, state: r.status, src: r.src, text: null, reason: r.reason, reasonText: reasonText(r.reason) };
  }
  if (typeof r.v !== 'number') {
    return { ...base, state: 'rejected', src: 'unavailable', text: null, reason: 'non_numeric_value', reasonText: reasonText('non_numeric_value') };
  }
  const stale = connKind === 'stale' || connKind === 'disconnected';
  const formatted = FORMATTERS[def.format](r.v);
  const suffix = unitSuffix(def.format);
  return {
    ...base,
    state: stale ? 'stale' : 'value',
    src: r.src,
    raw: r.v,
    text: suffix ? `${formatted} ${suffix}` : formatted,
    tone: def.criticalWhenNonZero && r.v !== 0 ? 'critical' : 'neutral',
  };
}

export function metricViews(payload, ids, connKind) {
  return ids.map((id) => ({ id, ...metricView(payload, METRICS[id], connKind) }));
}

const CAPABILITY_RULES = Object.freeze({
  cluster: { enabled: ['experimental'], off: ['disabled'], label: 'Cluster' },
  controller: { enabled: ['active'], off: ['none'], label: 'Adaptive controller' },
  origin_metrics: { enabled: ['available'], off: ['unavailable'], label: 'Origin metrics' },
});

/** state: enabled | not_enabled | unknown (unrecognised value shown verbatim) | unreported */
export function capabilityView(payload, name) {
  const rule = CAPABILITY_RULES[name];
  const raw = payload?.capabilities?.[name];
  const label = rule ? rule.label : name;
  if (!payload) return { name, label, state: 'unreported', value: null };
  if (typeof raw !== 'string') return { name, label, state: 'unreported', value: null };
  if (rule && rule.enabled.includes(raw)) return { name, label, state: 'enabled', value: raw };
  if (rule && rule.off.includes(raw)) return { name, label, state: 'not_enabled', value: raw };
  return { name, label, state: 'unknown', value: raw };
}

/** Origin block: whole-object availability first, then per-field. */
export function originAvailability(payload) {
  if (!payload) return { state: 'loading' };
  const origin = payload.metrics?.origin;
  // A container without its own `src` is normal: its members each carry provenance.
  if (origin !== null && typeof origin === 'object' && !Array.isArray(origin) && origin.src === undefined) return { state: 'present' };
  const r = readField(origin, { mode: payload.mode });
  if (r.status === 'unavailable') return { state: 'unavailable', reason: r.reason, reasonText: reasonText(r.reason) };
  if (r.status === 'missing') return { state: 'unavailable', reason: 'not_in_payload', reasonText: reasonText('not_in_payload') };
  if (r.status === 'rejected') return { state: 'rejected', reason: r.reason, reasonText: reasonText(r.reason) };
  return { state: 'present' };
}

/** Fields the gateway sent that the UI catalog does not know yet. Shown generically, never hidden. */
export function additionalFields(payload) {
  if (!payload?.metrics) return [];
  const known = knownMetricPaths();
  return flattenProvenanceFields(payload.metrics)
    .filter(({ path }) => !known.has(path) && !path.startsWith('latency_ms') && !path.startsWith('origin.latency_ms'))
    .map(({ path, field }) => {
      const r = readField(field, { mode: payload.mode });
      return {
        path,
        state: r.status,
        src: r.src,
        unit: r.unit,
        text: r.status === 'value' ? `${typeof r.v === 'number' ? r.v.toLocaleString('en-US') : String(r.v)}${r.unit ? ` ${r.unit}` : ''}` : null,
        reasonText: r.status === 'value' ? null : reasonText(r.reason),
      };
    });
}

/** Node list for the cluster placeholder. state_src is shown as the provenance of the node state. */
export function nodeViews(payload) {
  if (!payload || !Array.isArray(payload.nodes)) return [];
  return payload.nodes.slice(0, 200).map((n, i) => ({
    key: `${n && n.node_id !== undefined ? n.node_id : 'n'}-${i}`,
    id: n && n.node_id !== undefined ? String(n.node_id) : 'unknown',
    state: typeof n?.state === 'string' ? n.state : 'unknown',
    stateSrc: ['measured', 'derived', 'simulated', 'inferred'].includes(n?.state_src) ? n.state_src : 'unavailable',
    cpu: readField(n?.cpu, { mode: payload.mode }),
    memory: readField(n?.memory, { mode: payload.mode }),
  }));
}

/**
 * Placeholder screens are driven by capability discovery, never by invented numbers.
 * Returns {state: 'not_enabled' | 'no_contract' | 'unreported' | 'loading', headline, detail}.
 */
export function placeholderView(screenId, payload) {
  if (!payload) return { state: 'loading', headline: 'Waiting for capability discovery', detail: 'Connect to a gateway to learn which capabilities it reports.' };
  if (screenId === 'cluster') {
    const cap = capabilityView(payload, 'cluster');
    if (cap.state === 'not_enabled') return { state: 'not_enabled', headline: 'Cluster mode is not enabled', detail: 'The gateway reports cluster: disabled, so it runs standalone and there is no cluster to show.', cap };
    if (cap.state === 'enabled') return { state: 'no_contract', headline: 'Cluster is experimental; no cluster telemetry contract yet', detail: 'The gateway reports cluster: experimental. Schema 2 carries only a basic node list, shown below with its stated provenance. Membership, invalidation status and recovery need the backend contracts listed in ui_api_requirements.md.', cap };
    return { state: 'unreported', headline: 'Cluster capability not reported', detail: 'The gateway did not report a recognisable cluster capability.', cap };
  }
  if (screenId === 'intelligence') {
    const cap = capabilityView(payload, 'controller');
    if (cap.state === 'not_enabled') return { state: 'not_enabled', headline: 'Phantom Intelligence is not enabled', detail: 'The gateway reports controller: none. No decisions, signals or candidates exist to display.', cap };
    if (cap.state === 'enabled') return { state: 'no_contract', headline: 'Controller reported active; no UI contract yet', detail: 'The gateway reports controller: active, but schema 2 carries no decision, signal or guardrail data. The UI shows nothing rather than guessing.', cap };
    return { state: 'unreported', headline: 'Controller capability not reported', detail: 'The gateway did not report a recognisable controller capability.', cap };
  }
  if (screenId === 'lab') {
    return { state: 'no_contract', headline: 'No lab capability is advertised', detail: 'Schema 2 has no lab capability key. Lab runs are offline files today and are not served by the gateway.' };
  }
  return { state: 'unreported', headline: 'Unknown screen', detail: '' };
}

/** Overall banner decision for a data screen. */
export function screenGate(connKind, hasData) {
  if (!hasData) {
    if (connKind === 'loading') return 'loading';
    if (connKind === 'disconnected') return 'disconnected';
    if (connKind === 'error') return 'error';
    return 'loading';
  }
  return connKind === 'live' ? 'ready' : connKind; // stale | disconnected | degraded keep content visible
}

export { subField };
