// Schema v2 parsing and contract checking (see docs/research/telemetry_schema_v2.md).

export const MAX_EVENT_BYTES = 1024 * 1024; // refuse absurdly large frames instead of parsing them
export const LIVE_FORBIDDEN_ROOT_FIELDS = Object.freeze(['avoided_compute_dollars', 'backend_load_ratio', 'cost']);

function isObject(x) {
  return x !== null && typeof x === 'object' && !Array.isArray(x);
}

/** Walk the payload and list contract violations. Never throws, never mutates. */
export function findViolations(payload) {
  const violations = [];
  if (!isObject(payload) || payload.mode !== 'live') return violations;
  const walk = (node, path, depth) => {
    if (depth > 12 || node === null || typeof node !== 'object') return;
    if (Array.isArray(node)) {
      node.forEach((item, i) => walk(item, `${path}[${i}]`, depth + 1));
      return;
    }
    if (node.src === 'simulated') violations.push({ path, code: 'simulated_in_live' });
    if (node.src === 'unavailable' && Object.prototype.hasOwnProperty.call(node, 'v')) {
      violations.push({ path, code: 'unavailable_with_value' });
    }
    for (const [k, v] of Object.entries(node)) walk(v, `${path}.${k}`, depth + 1);
  };
  walk(payload, 'root', 0);
  for (const f of LIVE_FORBIDDEN_ROOT_FIELDS) {
    if (Object.prototype.hasOwnProperty.call(payload, f)) violations.push({ path: `root.${f}`, code: 'forbidden_in_live' });
  }
  return violations;
}

/**
 * Parse one SSE frame body.
 * Returns {ok:true, payload, violations} or {ok:false, code, detail}.
 * Unknown extra fields are kept as-is (never coerced).
 */
export function parseSchema2(text) {
  if (typeof text !== 'string' || text.length === 0) return { ok: false, code: 'empty_frame' };
  if (text.length > MAX_EVENT_BYTES) return { ok: false, code: 'frame_too_large' };
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, code: 'invalid_json' };
  }
  if (!isObject(data)) return { ok: false, code: 'not_an_object' };
  if (data.schema !== 2) {
    return { ok: false, code: 'unsupported_schema', detail: data.schema === undefined ? 'missing' : String(data.schema) };
  }
  if (data.mode !== 'live' && data.mode !== 'simulation') {
    return { ok: false, code: 'unknown_mode', detail: String(data.mode) };
  }
  if (!isObject(data.metrics)) return { ok: false, code: 'missing_metrics' };
  return { ok: true, payload: data, violations: findViolations(data) };
}

export function parseErrorText(code, detail) {
  switch (code) {
    case 'unsupported_schema':
      return `The gateway sent schema "${detail}"; this UI only understands schema 2.`;
    case 'invalid_json':
      return 'The gateway sent a frame that is not valid JSON.';
    case 'frame_too_large':
      return 'The gateway sent a frame larger than the safety limit.';
    case 'unknown_mode':
      return `The gateway reported an unknown mode "${detail}".`;
    case 'missing_metrics':
      return 'The gateway frame has no metrics object.';
    case 'not_an_object':
      return 'The gateway frame is not a JSON object.';
    default:
      return 'The gateway frame could not be read.';
  }
}
