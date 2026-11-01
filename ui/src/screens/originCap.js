// View model for the origin concurrency controller (payload.metrics.origin_cap). Pure. Shows only what the gateway reports (R4 E3, S1):
//   - the container is absent or unavailable  -> state 'unavailable' with the gateway's own reason code
//   - enforcing === true                      -> 'enforcing'
//   - enforcing === false                     -> 'not_enforcing' (counted but not applied: never drawn as protection)
//   - anything else                           -> 'unknown'
// It never invents a value, never computes a ratio, and never describes the controller as adaptive, learning or intelligent.
import { readField } from '../lib/provenance.js';
import { formatCount } from '../lib/format.js';

const UNAVAILABLE_TEXT = Object.freeze({
  intelligence_not_installed: 'The gateway reports that its intelligence executor is not installed (intelligence_not_installed), so there is no origin concurrency gate to report.',
  not_in_payload: 'The gateway did not include an origin concurrency gate in this frame.',
});

const isObject = (x) => x !== null && typeof x === 'object' && !Array.isArray(x);

const ROWS = Object.freeze([
  ['in_flight', 'Fetches in flight now'],
  ['cap', 'Cap'],
  ['dispatched', 'Fetches sent to the origin'],
  ['origin_cap_rejections', 'Fetches refused by the cap'],
  ['controller_steps', 'Controller steps'],
]);

export function originCapView(payload) {
  if (!payload) return { state: 'loading', rows: [] };
  const mode = payload.mode;
  const oc = payload.metrics ? payload.metrics.origin_cap : undefined;
  if (!isObject(oc)) return { state: 'unavailable', reason: 'not_in_payload', reasonText: UNAVAILABLE_TEXT.not_in_payload, rows: [] };
  if (oc.src !== undefined) {
    const r = readField(oc, { mode });
    const code = r.reason || 'not_in_payload';
    return { state: 'unavailable', reason: code, reasonText: UNAVAILABLE_TEXT[code] || `The gateway gave no origin concurrency gate. Reason code: ${code}.`, rows: [] };
  }
  const enforcing = readField(oc.enforcing, { mode });
  const state = enforcing.status === 'value' && typeof enforcing.v === 'boolean' ? (enforcing.v ? 'enforcing' : 'not_enforcing') : 'unknown';
  const rows = ROWS.map(([key, label]) => {
    const r = readField(oc[key], { mode });
    const isValue = r.status === 'value' && typeof r.v === 'number';
    const reason = isValue ? null : r.reason || 'not_in_payload';
    return {
      key,
      label,
      status: isValue ? 'value' : 'unavailable',
      src: isValue ? r.src : 'unavailable',
      text: isValue ? formatCount(r.v) : null,
      raw: isValue ? r.v : null,
      reason,
      reasonText: reason === 'uncapped' ? 'No cap is configured on this gateway (uncapped).' : reason ? `Not reported (${reason}).` : null,
    };
  });
  const headline = state === 'enforcing'
    ? 'Enforcing. The gateway is limiting fetches to the origin to the cap shown.'
    : state === 'not_enforcing'
      ? 'Not enforcing. The cap is counted but not applied, so requests are not limited.'
      : 'The gateway did not say whether the cap is enforced.';
  return { state, enforcingSrc: enforcing.status === 'value' ? enforcing.src : 'unavailable', headline, rows };
}
