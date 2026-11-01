// View model of the Intelligence screen (lane L2, step 2). Pure functions: no DOM, no React, no clock, no network.
//
// Source: the PIE v2 HTTP API, docs/lanes/CONTRACT-pie-api-v1.md (schema "pie-api-v1"): GET /api/v1/intelligence/{status,registry,signals,decisions}.
// Honesty rules enforced here (CLAUDE.md sections 4 and 6b, board D-045, brief D-048):
//  * Every value shown carries the provenance the GATEWAY stated (measured / derived / unavailable). Nothing is upgraded, extrapolated or
//    turned into zero. `simulated` is withheld in live mode (the contract says the live gateway never emits it as a result).
//  * Only Off, Observe and Advisory exist. A mode outside those three is shown as unexpected, never as a normal state.
//  * Nothing is ever applied. A recommendation whose `applied` is not exactly false is withheld.
//  * Declared documentation (registry text, config) is shown as declared, not as a measurement.
//  * No client-side metric arithmetic: values are formatted, not computed. Requests keys and tenants never appear (the contract sends hashes only).

export const MODES = Object.freeze(['off', 'observe', 'advisory']);
export const MODE_STATEMENT = 'Only three modes exist: Off, Observe and Advisory. There is no canary and no active mode, so nothing this page shows has changed how the gateway serves requests. Advisory results are shadow estimates, not applied changes.';
export const NO_BENEFIT_STATEMENT = 'This page makes no benefit claim. The research hypothesis that intelligence improves the hit ratio (M6 H1) is NOT_SUPPORTED, and advisory output is an estimate from a sampled shadow stream, not verified on the serving path.';

const isObject = (x) => x !== null && typeof x === 'object' && !Array.isArray(x);
const clipText = (v, n = 200) => String(v ?? '').replace(/[\u0000-\u001f\u007f]/g, ' ').trim().slice(0, n);
const intOrNull = (v) => (Number.isInteger(v) && v >= 0 ? v : null);

export const REASON_TEXT = Object.freeze({
  not_in_response: 'Not in the gateway response',
  unknown_provenance: 'Withheld: unknown provenance class',
  simulated_in_live: 'Withheld: a simulated value cannot be shown as live',
  unavailable_with_value: 'Withheld: marked unavailable but carries a value',
  unsupported_value_type: 'Withheld: value is not a number, flag or short label',
  insufficient_samples: 'Not enough samples yet',
  window_degraded: 'The last window was degraded (events were dropped)',
  no_window_closed_yet: 'No window has closed yet',
  not_implemented: 'Not implemented in this version',
  needs_multi_node_view: 'Needs a multi-node view this gateway does not have',
  intelligence_v2_not_installed: 'Phantom Intelligence v2 is not installed in this gateway process',
});

export function reasonText(code) {
  if (typeof code !== 'string' || !code) return 'No reason given';
  return REASON_TEXT[code] || `Unavailable: ${clipText(code, 60).replace(/_/g, ' ')}`;
}

/** Format a number for display. No scaling: only rounding to four significant digits. */
export function formatNumber(n) {
  if (!Number.isFinite(n)) return null;
  if (Number.isInteger(n)) return n.toLocaleString('en-US');
  return Number(n.toPrecision(4)).toLocaleString('en-US', { maximumSignificantDigits: 4 });
}

/**
 * One contract value object V = { value, src, samples, [reason], [unit], [note] } -> { text, src, samples, unit, note, reason }.
 * `text` is null when the value is withheld or unavailable; `src` is then 'unavailable' and `reason` says why. Never throws.
 */
export function cell(field) {
  const out = (text, src, extra = {}) => ({ text, src, samples: null, unit: null, note: null, reason: null, ...extra });
  if (!isObject(field)) return out(null, 'unavailable', { reason: reasonText('not_in_response') });
  const src = field.src;
  const samples = intOrNull(field.samples);
  const unit = typeof field.unit === 'string' ? clipText(field.unit, 24) : null;
  const note = typeof field.note === 'string' ? clipText(field.note, 160) : null;
  if (!['measured', 'derived', 'simulated', 'unavailable'].includes(src)) return out(null, 'unavailable', { reason: reasonText('unknown_provenance') });
  if (src === 'unavailable') {
    if (field.value !== null && field.value !== undefined) return out(null, 'unavailable', { reason: reasonText('unavailable_with_value'), samples });
    return out(null, 'unavailable', { reason: reasonText(field.reason), samples, unit });
  }
  if (src === 'simulated') return out(null, 'unavailable', { reason: reasonText('simulated_in_live') });
  const v = field.value;
  let text = null;
  if (typeof v === 'number') text = formatNumber(v);
  else if (typeof v === 'boolean') text = v ? 'yes' : 'no';
  else if (typeof v === 'string' && v.length > 0 && v.length <= 80) text = clipText(v, 80);
  if (text === null) return out(null, 'unavailable', { reason: reasonText('unsupported_value_type'), samples });
  return out(text, src, { samples, unit, note });
}

/** A plain number the page shows with an explicit provenance (for fixed counters of the contract that are numbers, not V objects). */
function plain(n, src) {
  return Number.isFinite(n) ? { text: formatNumber(n), src, samples: null, unit: null, note: null, reason: null } : { text: null, src: 'unavailable', samples: null, unit: null, note: null, reason: reasonText('not_in_response') };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Mode

/** The mode of the process as a cell plus a verdict about it. `raw` is a V object or a legacy string like "Advisory". */
export function modeView(raw) {
  const c = typeof raw === 'string' ? { text: clipText(raw, 20), src: 'measured', samples: null, unit: null, note: null, reason: null } : cell(raw);
  if (c.text === null) return { ...c, key: 'unknown', known: false, label: null };
  const key = c.text.toLowerCase();
  const known = MODES.includes(key);
  return { ...c, key: known ? key : 'unexpected', known, label: known ? key.charAt(0).toUpperCase() + key.slice(1) : c.text };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Result classification of one HTTP answer

/**
 * @param {{ok: boolean, status: number, data: any, authError?: string|null}|null} res
 * @returns {{kind: 'loading'|'ok'|'unauthorised'|'no_answer'|'missing'|'error', status: number|null}}
 */
export function classify(res) {
  if (!res) return { kind: 'loading', status: null };
  if (res.status === 401 || res.status === 403) return { kind: 'unauthorised', status: res.status };
  if (res.status === 0) return { kind: 'no_answer', status: 0 };
  if (res.status === 404) return { kind: 'missing', status: 404 };
  if (res.ok && isObject(res.data)) return { kind: 'ok', status: res.status };
  return { kind: 'error', status: res.status };
}

export function sectionProblem(res) {
  const k = classify(res);
  if (k.kind === 'ok') return null;
  const words = { loading: 'Not loaded yet', unauthorised: `The gateway refused this request (HTTP ${k.status})`, no_answer: 'No answer from the gateway', missing: 'This gateway does not serve this endpoint (HTTP 404)', error: `The gateway answered ${k.status} with a body this page cannot use` };
  return { kind: k.kind, reason: words[k.kind] };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Availability of the whole screen, from the status answer

/**
 * 'loading' | 'unauthorised' | 'no_answer' | 'endpoints_missing' | 'error' | 'legacy' | 'not_installed' | 'ready'
 *  legacy      = the gateway answers /status in the old shape (no `v2` block): mode only; registry, signals and decisions are not served.
 *  not_installed = the gateway serves the API but Phantom Intelligence v2 is not installed in its process.
 */
export function availability(statusRes) {
  const k = classify(statusRes);
  if (k.kind === 'loading' || k.kind === 'unauthorised' || k.kind === 'no_answer') return k.kind;
  if (k.kind === 'missing') return 'endpoints_missing';
  if (k.kind === 'error') return 'error';
  const d = statusRes.data;
  const v2 = d.v2;
  if (v2 === undefined) return 'legacy';
  if (!isObject(v2)) return 'error';
  if (v2.installed === false) return 'not_installed';
  if (v2.installed === true && v2.schema === 'pie-api-v1') return 'ready';
  return 'error';
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Status (v2 block)

export function statusView(statusRes) {
  const av = availability(statusRes);
  const base = { availability: av };
  if (av === 'legacy') {
    const d = statusRes.data;
    return { ...base, mode: modeView(typeof d.intelligence_mode === 'string' ? d.intelligence_mode : null), legacy: { truthful: typeof d.truthful === 'boolean' ? d.truthful : null, activeCandidate: d.active_candidate === null || typeof d.active_candidate === 'string' ? d.active_candidate : null, revertCount: Number.isInteger(d.revert_count) ? d.revert_count : null } };
  }
  if (av === 'not_installed') return { ...base, mode: modeView(statusRes.data.intelligence_mode), reason: reasonText(statusRes.data.v2.reason) };
  if (av !== 'ready') return base;
  const v2 = statusRes.data.v2;
  const q = isObject(v2.queue) ? v2.queue : {};
  const w = isObject(v2.window) ? v2.window : {};
  const m = isObject(v2.memory) ? v2.memory : {};
  const dec = isObject(v2.decisions) ? v2.decisions : {};
  const cfg = isObject(v2.config) ? v2.config : {};
  const claims = isObject(v2.claims) ? v2.claims : {};
  const bd = isObject(m.breakdown) ? m.breakdown : {};
  const fault = isObject(v2.fault) ? { decisionId: intOrNull(v2.fault.decision_id), reason: clipText(v2.fault.reason_code, 40), detail: clipText(v2.fault.detail, 200) } : null;
  return {
    ...base,
    mode: modeView(v2.mode),
    maxMode: typeof v2.max_mode === 'string' ? clipText(v2.max_mode, 20) : null,
    fault,
    registryVersion: clipText(v2.registry_version, 20) || null,
    queue: {
      capacity: plain(q.capacity, 'derived'), // a declared capacity, shown as declared (see note in the screen)
      enqueued: cell(q.enqueued_total),
      dropped: cell(q.dropped_total),
      depth: cell(q.depth),
    },
    window: {
      windowMs: Number.isFinite(w.window_ms) ? formatNumber(w.window_ms) : null,
      decisionEveryWindows: Number.isFinite(w.decision_every_windows) ? formatNumber(w.decision_every_windows) : null,
      closed: cell(w.windows_closed),
      lastDegraded: cell(w.last_window_degraded),
      lastTickMs: cell(w.last_tick_ms),
    },
    memory: {
      bytes: cell(m.bytes),
      budget: Number.isFinite(m.budget_bytes) ? formatNumber(m.budget_bytes) : null,
      breakdown: ['queue', 'window_state', 'shadow_caches', 'decision_ring'].map((k) => ({ id: k, text: Number.isFinite(bd[k]) ? formatNumber(bd[k]) : null })),
    },
    decisions: {
      recorded: cell(dec.recorded_total),
      retained: cell(dec.retained),
      ringCapacity: Number.isFinite(dec.ring_capacity) ? formatNumber(dec.ring_capacity) : null,
      ringBytes: cell(dec.ring_bytes),
      suppressed: cell(dec.suppressed_by_cooldown),
      collapsed: cell(dec.collapsed_repeat_noaction),
      retainedCount: intOrNull(isObject(dec.retained) ? dec.retained.value : null),
    },
    config: {
      labMode: typeof cfg.lab_mode === 'boolean' ? cfg.lab_mode : null,
      cacheCapacity: Number.isFinite(cfg.cache_capacity_entries) ? formatNumber(cfg.cache_capacity_entries) : null,
      shadowSampleModulus: Number.isFinite(cfg.shadow_sample_modulus) ? formatNumber(cfg.shadow_sample_modulus) : null,
      incumbent: typeof cfg.incumbent_policy === 'string' ? clipText(cfg.incumbent_policy, 40) : null,
    },
    claims: { benefit: typeof claims.benefit_claim === 'string' ? clipText(claims.benefit_claim, 40) : null, note: typeof claims.note === 'string' ? clipText(claims.note, 300) : null },
  };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Registry

const LIFECYCLES = Object.freeze(['Proposed', 'Implemented', 'UnitTested', 'Integrated', 'ShadowEvaluated', 'Retained', 'Narrowed', 'Rejected']);
const MAX_POLICIES = 32;

export function registryView(res) {
  const problem = sectionProblem(res);
  if (problem) return { present: false, ...problem };
  const d = res.data;
  if (d.installed === false) return { present: false, kind: 'not_installed', reason: reasonText(d.reason) };
  if (!Array.isArray(d.policies)) return { present: false, kind: 'error', reason: 'The registry answer has no policy list' };
  const policies = d.policies.slice(0, MAX_POLICIES).filter(isObject).map((p) => {
    const eff = cell(p.effective_runtime_state);
    const maxState = typeof p.max_runtime_state === 'string' ? p.max_runtime_state.toLowerCase() : null;
    return {
      id: clipText(p.policy_id, 40),
      name: clipText(p.name, 120),
      kind: clipText(p.kind, 60),
      implemented: p.implemented === true,
      lifecycle: LIFECYCLES.includes(p.lifecycle) ? p.lifecycle : `${clipText(p.lifecycle, 30)} (not a known lifecycle state)`,
      lifecycleKnown: LIFECYCLES.includes(p.lifecycle),
      version: clipText(p.version, 20),
      maxRuntime: MODES.includes(maxState) ? maxState : `${clipText(p.max_runtime_state, 20)} (unexpected)`,
      maxRuntimeKnown: MODES.includes(maxState),
      effective: eff,
      docs: ['objective', 'trigger', 'expected_effect', 'evaluation_method', 'limitations'].map((f) => ({ field: f, text: clipText(p[f], 400) })).filter((x) => x.text),
      safety: Array.isArray(p.safety_constraints) ? p.safety_constraints.slice(0, 8).map((s) => clipText(s, 200)) : [],
      inputs: Array.isArray(p.required_inputs) ? p.required_inputs.slice(0, 12).filter(isObject).map((r) => ({ feature: clipText(r.feature, 8), minSamples: intOrNull(r.min_samples), allowed: Array.isArray(r.allowed_src) ? r.allowed_src.map((s) => clipText(s, 12)).join(', ') : '' })) : [],
      parameters: Array.isArray(p.parameters) ? p.parameters.slice(0, 16).filter(isObject).map((r) => ({ name: clipText(r.name, 40), current: formatNumber(r.current), def: formatNumber(r.default), min: formatNumber(r.min), max: formatNumber(r.max), unit: clipText(r.unit, 24) })) : [],
    };
  });
  return { present: true, registryVersion: clipText(d.registry_version, 20), maxMode: clipText(d.max_mode, 20), processMode: modeView(d.process_mode), policies, truncated: d.policies.length > MAX_POLICIES };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Signals

const MAX_FEATURES = 32;

export function signalsView(res) {
  const problem = sectionProblem(res);
  if (problem) return { present: false, ...problem };
  const d = res.data;
  if (d.installed === false) return { present: false, kind: 'not_installed', reason: reasonText(d.reason) };
  const w = isObject(d.window) ? d.window : null;
  const features = isObject(d.features)
    ? Object.entries(d.features).slice(0, MAX_FEATURES).filter(([, f]) => isObject(f) && !(f.src === 'unavailable' && PERMANENT_REASONS.includes(f.reason))).map(([id, f]) => ({ id: clipText(id, 8), name: clipText(f.name, 60), unit: typeof f.unit === 'string' ? clipText(f.unit, 24) : null, cell: cell(f) }))
    : [];
  const top = isObject(d.top_keys) && Array.isArray(d.top_keys.items) ? d.top_keys.items.slice(0, 10).filter(isObject).map((i) => ({ hash: /^[0-9a-f]{16}$/.test(String(i.key_hash)) ? i.key_hash : null, count: formatNumber(i.count) })).filter((i) => i.hash) : [];
  return {
    present: true,
    mode: modeView(d.mode),
    window: w
      ? { index: intOrNull(w.index), durationMs: formatNumber(w.duration_ms), events: formatNumber(w.events), dropped: formatNumber(w.dropped_events), degraded: w.degraded === true, degradedReason: typeof w.degraded_reason === 'string' ? clipText(w.degraded_reason, 40) : null }
      : null,
    features,
    topKeys: { src: isObject(d.top_keys) && d.top_keys.src === 'derived' ? 'derived' : 'unavailable', items: top },
    classification: cell(d.classification),
    history: Array.isArray(d.history) ? d.history.slice(-8).filter(isObject).map((h) => ({ index: intOrNull(h.index), events: formatNumber(h.events), degraded: h.degraded === true })) : [],
  };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Decisions

const VERDICTS = Object.freeze(['Recommend', 'NoAction', 'Observation', 'Fault']);
const MAX_RECORDS = 50;

function effectView(e) {
  if (e === null || e === undefined) return null;
  if (!isObject(e)) return { text: null, reason: reasonText('unsupported_value_type') };
  const c = cell({ value: e.value, src: e.src, samples: e.samples ?? null, unit: e.unit });
  const ci = Array.isArray(e.ci95) && e.ci95.length === 2 && e.ci95.every((x) => Number.isFinite(x)) ? `${formatNumber(e.ci95[0])} to ${formatNumber(e.ci95[1])}` : null;
  return { metric: clipText(e.metric, 40), cell: c, ci95: ci, basis: clipText(e.basis, 60) };
}

export function decisionView(r) {
  const verdict = VERDICTS.includes(r.verdict) ? r.verdict : null;
  const rec = isObject(r.recommendation) ? r.recommendation : null;
  // Nothing is ever applied. A recommendation that does not say applied === false is withheld, not shown as advice.
  const recOk = rec && rec.applied === false;
  const modeKey = typeof r.mode === 'string' ? r.mode.toLowerCase() : '';
  return {
    id: intOrNull(r.decision_id),
    timestampMs: formatNumber(r.timestamp_ms),
    policyId: clipText(r.policy_id, 40),
    policyVersion: clipText(r.policy_version, 20),
    mode: MODES.includes(modeKey) ? modeKey : `${clipText(r.mode, 20)} (unexpected)`,
    modeKnown: MODES.includes(modeKey),
    verdict: verdict || `${clipText(r.verdict, 20)} (unknown verdict)`,
    verdictKnown: Boolean(verdict),
    reasonCode: clipText(r.reason_code, 60),
    inputs: Array.isArray(r.inputs) ? r.inputs.slice(0, 16).filter(isObject).map((i) => ({ feature: clipText(i.feature, 8), cell: cell({ value: i.value, src: i.src, samples: i.samples }), windowMs: formatNumber(i.window_ms) })) : [],
    candidates: Array.isArray(r.candidates)
      ? r.candidates.slice(0, 12).filter(isObject).map((c) => ({ id: clipText(c.candidate_id, 40), status: c.status === 'selected' ? 'selected' : c.status === 'rejected' ? 'rejected' : clipText(c.status, 20), rejection: c.rejection_reason ? clipText(c.rejection_reason, 40) : null, effect: effectView(c.estimated_effect) }))
      : [],
    constraints: Array.isArray(r.constraints) ? r.constraints.slice(0, 12).filter(isObject).map((c) => ({ name: clipText(c.name, 60), value: clipText(c.value, 60), holds: c.holds === true })) : [],
    recommendation: recOk ? { text: clipText(rec.text, 400), label: clipText(rec.label, 80) || 'advisory, not applied' } : null,
    recommendationWithheld: Boolean(rec) && !recOk,
    guardrails: Array.isArray(r.guardrail_events) ? r.guardrail_events.slice(0, 12).map((g) => clipText(g, 80)) : [],
    conflict: isObject(r.conflict) && r.conflict.lever && r.conflict.lever !== 'none' ? { lever: clipText(r.conflict.lever, 40), role: clipText(r.conflict.role, 20), with: Array.isArray(r.conflict.with) ? r.conflict.with.filter(Number.isInteger).slice(0, 8) : [] } : null,
    observed: cell(r.observed_effect),
    // `simulated: true` in a live gateway answer is not shown as a live result.
    simulatedClaim: r.simulated === true,
    explanation: clipText(r.explanation, 500),
  };
}

export function decisionsView(res) {
  const problem = sectionProblem(res);
  if (problem) return { present: false, ...problem };
  const d = res.data;
  if (d.installed === false) return { present: false, kind: 'not_installed', reason: reasonText(d.reason) };
  if (!Array.isArray(d.records)) return { present: false, kind: 'error', reason: 'The decisions answer has no record list' };
  const records = d.records.slice(0, MAX_RECORDS).filter(isObject).map(decisionView).reverse(); // newest first
  return { present: true, totalRecorded: formatNumber(d.total_recorded), retained: formatNumber(d.retained), returned: intOrNull(d.returned), ordering: d.ordering === 'oldest_first' ? 'oldest_first' : 'unexpected', records };
}

/** The request path for the newest `limit` records, given how many the gateway says it retains. */
export function decisionsPath(retained, limit = 20) {
  const r = Number.isInteger(retained) && retained > 0 ? retained : 0;
  const offset = Math.max(0, r - limit);
  return `/api/v1/intelligence/decisions?limit=${limit}&offset=${offset}`;
}

export const PATHS = Object.freeze({
  status: '/api/v1/intelligence/status',
  registry: '/api/v1/intelligence/registry',
  signals: '/api/v1/intelligence/signals',
});

/** Whether the nav may list Intelligence as a working screen: only when the gateway says v2 is installed. */
export function intelligenceLive(statusRes) {
  return availability(statusRes) === 'ready';
}

/** Nav grouping: Intelligence is a normal live item unless the gateway has ANSWERED that it lacks the API (no flash of 'Not available' while asking). */
export function intelligenceNavLive(statusRes) {
  return !['endpoints_missing', 'legacy', 'not_installed'].includes(availability(statusRes));
}

/** Reasons that mean the value can never exist on this gateway: the row is left out instead of printing Unavailable. */
export const PERMANENT_REASONS = Object.freeze(['not_implemented', 'needs_multi_node_view']);
