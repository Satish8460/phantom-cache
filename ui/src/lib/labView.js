// View-models for the Phantom Lab screens. Pure functions from Lab API payloads (lab-1) to render-ready shapes.
//
// Rules this module enforces (docs/design/principles.md, docs/design/lab_api_contract.md section 6):
//  * Every number shown comes from a payload field. Formatting scales for display only (ratio -> percent).
//  * A missing value, a `null`, or `{unavailable:true, reason}` becomes an `unavailable` cell with a reason in words.
//    It is never turned into 0.
//  * Negative and inconclusive results go through exactly the same shapes as positive ones.
//  * Server text is clipped; no field is trusted to be a string.

import { formatBytes, formatCount, formatMs, formatRatioAsPercent, clip } from './format.js';
import { kindInfo, isExploratoryKind, tierInfo, verdictInfo } from './labProvenance.js';

const isObj = (x) => x !== null && typeof x === 'object' && !Array.isArray(x);
const isNum = (x) => typeof x === 'number' && Number.isFinite(x);
const str = (x, max = 200) => (typeof x === 'string' ? clip(x, max) : null);

export function unavailableReason(raw, fallback) {
  if (isObj(raw) && raw.unavailable === true) return str(raw.reason, 160) || fallback;
  return fallback;
}

/** One value cell: { state: 'value'|'unavailable', text, reason, num } */
export function cell(raw, fmt, fallbackReason = 'not reported in the payload') {
  if (isNum(raw)) return { state: 'value', text: fmt(raw), num: raw, reason: null };
  return { state: 'unavailable', text: 'Unavailable', num: null, reason: unavailableReason(raw, fallbackReason) };
}

export const FMT = Object.freeze({
  ms: (n) => `${formatMs(n)} ms`,
  count: (n) => formatCount(n),
  smart: (n) => (Number.isInteger(n) ? formatCount(n) : n.toFixed(Math.abs(n) >= 100 ? 1 : 2)),
  ratio: (n) => formatRatioAsPercent(n),
  signedRatio: (n) => `${n > 0 ? '+' : ''}${formatRatioAsPercent(n)}`,
  pp: (n) => `${n > 0 ? '+' : ''}${n.toFixed(2)} pp`,
  bytes: (n) => formatBytes(n),
  p: (n) => (n < 0.0001 ? 'p < 0.0001' : `p = ${n.toFixed(4)}`),
  plain: (n) => String(n),
});

export function utcText(raw, fallback = 'not recorded') {
  if (typeof raw === 'string' && raw) return clip(raw, 40);
  const r = unavailableReason(raw, null);
  return r ? `${fallback} (${r})` : fallback;
}

// ---------------------------------------------------------------------------------------------------------------
// Capabilities

const SECURITY_MODES = Object.freeze({
  anonymous_dev_loopback_only: {
    label: 'Anonymous, development, loopback only',
    detail: 'No token is needed. The gateway accepts only loopback Host names and the origins in PHANTOM_CORS_ORIGINS. Any local process can reach it.',
    needsToken: false,
  },
  jwt: {
    label: 'JWT authentication',
    detail: 'Reading needs an Observer token. Starting and cancelling runs needs an Operator token.',
    needsToken: true,
  },
});

export function securityModeInfo(mode) {
  if (typeof mode === 'string' && SECURITY_MODES[mode]) return { key: mode, ...SECURITY_MODES[mode] };
  return { key: 'unknown', label: typeof mode === 'string' ? `Unknown mode: ${clip(mode, 40)}` : 'Mode not reported', detail: 'The gateway reported a security mode this page does not know.', needsToken: null };
}

export function capabilitiesView(c) {
  if (!isObj(c)) return null;
  const kinds = isObj(c.kinds) ? c.kinds : {};
  const mk = (key) => {
    const k = isObj(kinds[key]) ? kinds[key] : null;
    const info = kindInfo(key);
    if (!k) return { key, label: info.label, present: false, available: false, reason: 'not listed by the gateway', tier: info.tier, buildProfile: null, limits: null };
    return {
      key,
      label: info.label,
      present: true,
      available: k.available === true,
      reason: str(k.reason, 200),
      tier: str(k.tier, 40) || info.tier,
      buildProfile: str(k.driver_build_profile, 20),
      gatewayProfile: str(k.gateway_profile_inferred_from_path, 20),
      predictedLabel: str(k.label_for_default_experiment, 20),
      labelReasons: Array.isArray(k.label_reasons) ? k.label_reasons.map((x) => str(x, 400)).filter(Boolean) : [],
      labelMeaning: str(k.label_meaning, 1200),
      limits: isObj(k.limits) ? k.limits : null,
      arms: Array.isArray(k.arms) ? k.arms.filter((x) => typeof x === 'string') : [],
      scenarios: Array.isArray(k.scenarios) ? k.scenarios.filter((x) => typeof x === 'string') : [],
      families: Array.isArray(k.families) ? k.families.filter((x) => typeof x === 'string') : [],
      profiles: Array.isArray(k.profiles) ? k.profiles.filter((x) => typeof x === 'string') : [],
      seedSets: Array.isArray(k.seed_sets) ? k.seed_sets.filter((s) => isObj(s) && typeof s.name === 'string').map((s) => ({ name: s.name, size: isNum(s.size) ? s.size : null })) : [],
      armParams: isObj(k.arm_params) ? { frozen: k.arm_params.frozen === true, untuned: k.arm_params.untuned === true } : null,
      seedSetsNote: str(k.seed_sets_note, 300),
    };
  };
  const debug = ['t1_matrix', 'scenario_b'].some((k) => isObj(kinds[k]) && kinds[k].driver_build_profile === 'debug');
  return {
    schema: str(c.schema, 20),
    security: securityModeInfo(c.security_mode),
    t1: mk('t1_matrix'),
    sb: mk('scenario_b'),
    debugBuild: debug,
    activeRun: isObj(c.active_run) ? { runId: str(c.active_run.run_id, 64), kind: str(c.active_run.kind, 30), state: str(c.active_run.state, 20) } : null,
    activeRunRaw: c.active_run ?? null,
    maxConcurrent: isNum(c.max_concurrent_runs) ? c.max_concurrent_runs : null,
    limits: isObj(c.limits) ? {
      bodyBytes: isNum(c.limits.body_bytes) ? c.limits.body_bytes : null,
      artifactBytes: isNum(c.limits.artifact_bytes) ? c.limits.artifact_bytes : null,
      heartbeatSeconds: isNum(c.limits.heartbeat_seconds) ? c.limits.heartbeat_seconds : null,
      indexMax: isNum(c.limits.index_max) ? c.limits.index_max : null,
    } : null,
  };
}

// ---------------------------------------------------------------------------------------------------------------
// Evidence

function verdictChipsM6(snapshot) {
  const labels = { H1: 'H1', H2: 'H2', H3: 'H3', H4: 'H4', H5_H6: 'H5 and H6' };
  return Object.keys(labels).filter((k) => typeof snapshot[k] === 'string').map((k) => ({ name: labels[k], ...verdictInfo(snapshot[k]) }));
}

function verdictChipsSb(snapshot) {
  const v = isObj(snapshot.verdicts) ? snapshot.verdicts : {};
  return Object.keys(v).map((arm) => ({ name: arm, ...verdictInfo(isObj(v[arm]) ? v[arm].outcome : null), reasons: Array.isArray(v[arm]?.reasons) ? v[arm].reasons.map((r) => str(r, 400)).filter(Boolean) : [] }));
}

export function evidenceItemView(it) {
  if (!isObj(it) || typeof it.id !== 'string') return null;
  const k = kindInfo(it.kind);
  const snap = isObj(it.verdict_snapshot) ? it.verdict_snapshot : null;
  const noVerdict = !snap || snap.unavailable === true;
  let chips = [];
  if (!noVerdict) chips = it.kind === 'scenario_b' ? verdictChipsSb(snap) : verdictChipsM6(snap);
  return {
    id: it.id,
    href: `#/lab/evidence/${encodeURIComponent(it.id)}`,
    kind: k,
    exploratory: isExploratoryKind(it.kind),
    tier: tierInfo(it.tier),
    originalTier: typeof it.original_tier === 'string' ? tierInfo(it.original_tier) : null,
    started: utcText(it.started_utc),
    ended: utcText(it.ended_utc),
    readOnly: it.read_only === true,
    verdictChips: chips,
    verdictMissing: noVerdict ? unavailableReason(snap, 'this retained artifact set records no verdict') : null,
    label: it.kind === 'scenario_b' && snap ? str(snap.label, 20) : null,
    labelReasons: it.kind === 'scenario_b' && snap && Array.isArray(snap.label_reasons) ? snap.label_reasons.map((r) => str(r, 400)).filter(Boolean) : [],
    banner: snap ? str(snap.banner, 400) : null,
    artifacts: Array.isArray(it.artifacts) ? it.artifacts.filter((a) => isObj(a) && typeof a.name === 'string').map((a) => ({ name: clip(a.name, 80), bytes: isNum(a.bytes) ? FMT.bytes(a.bytes) : 'size not reported' })) : [],
  };
}

export function evidenceListView(payload) {
  if (!isObj(payload) || !Array.isArray(payload.items)) return null;
  const items = payload.items.map(evidenceItemView).filter(Boolean);
  return {
    items,
    registered: items.filter((i) => !i.exploratory),
    exploratory: items.filter((i) => i.exploratory),
    total: isNum(payload.indexed) ? payload.indexed : items.length,
    boundedTo: isNum(payload.bounded_to_newest) ? payload.bounded_to_newest : null,
  };
}

// --- M6 (T1 simulation) -----------------------------------------------------------------------------------------

function endpointView(e, unitFmt) {
  if (!isObj(e)) return null;
  return {
    mean: cell(e.mean_rel_reduction, FMT.signedRatio),
    lo: cell(e.ci_lo, FMT.signedRatio),
    hi: cell(e.ci_hi, FMT.signedRatio),
    meanCtrl: cell(e.mean_ctrl, unitFmt),
    meanTreat: cell(e.mean_treat, unitFmt),
    n: isNum(e.n_seeds) ? e.n_seeds : null,
    pHolm: cell(e.p_holm, FMT.p),
    supported: e.supported === true,
    adverse: e.adverse === true,
    practicallyNull: e.practically_null === true,
    endpoint: str(e.endpoint, 60),
  };
}

const H1_FLAGS = Object.freeze([
  ['supported_rule_met', 'Support rule met'],
  ['not_supported_triggered', 'Not-supported rule triggered'],
  ['any_adverse_ci_peak', 'Any scenario credibly worse on peak origin concurrency'],
  ['any_adverse_ci_p99', 'Any scenario credibly worse on client P99'],
  ['fewer_than_3_scenarios_supported_on_an_endpoint', 'Fewer than 3 scenarios supported on an endpoint'],
  ['fresh_hit_constraint_holds_in_all_scenarios', 'Fresh-hit constraint holds in all scenarios'],
  ['error_rate_constraint_holds_in_all_scenarios', 'Error-rate constraint holds in all scenarios'],
  ['n_seeds_paired_in_every_hypothesis', 'Same seeds paired in every hypothesis'],
]);

export function m6DetailView(result) {
  if (!isObj(result)) return null;
  const per = Array.isArray(result.per_scenario) ? result.per_scenario.filter(isObj) : [];
  const h1 = isObj(result.H1) ? result.H1 : {};
  const hyp = [];
  hyp.push({
    id: 'H1', title: 'H1: the adaptive controller beats the best static configuration', verdict: verdictInfo(h1.verdict),
    lines: [
      { label: 'Scenarios supported on peak origin concurrency', value: cell(h1.supported_scenarios_peak_concurrency, FMT.count), of: per.length },
      { label: 'Scenarios supported on client P99', value: cell(h1.supported_scenarios_client_p99, FMT.count), of: per.length },
    ],
    flags: H1_FLAGS.filter(([k]) => typeof h1[k] === 'boolean').map(([k, label]) => ({ key: k, label, value: h1[k] })),
  });
  for (const [id, title] of [['H2', 'H2: dynamic stale window'], ['H3', 'H3: overhead'], ['H4', 'H4: stale window under load'], ['H5_H6', 'H5 and H6']]) {
    const h = isObj(result[id]) ? result[id] : null;
    if (!h) continue;
    hyp.push({ id, title, verdict: verdictInfo(h.status), reason: str(h.reason, 400) || str(h.note, 400), lines: [], flags: [], compared: id === 'H2' ? { compared: cell(h.compared_cells, FMT.count), identical: cell(h.identical_cells, FMT.count) } : null });
  }
  const falsifiers = isObj(result.falsifiers) ? Object.keys(result.falsifiers).map((id) => {
    const f = result.falsifiers[id];
    const fires = isObj(f) ? f.fires : null;
    return { id, fires: fires === true ? 'fires' : fires === false ? 'does not fire' : 'not mechanical', note: isObj(f) ? str(f.note, 300) || str(f.rule, 300) : null };
  }) : [];
  const scenarios = per.map((s) => ({
    scenario: str(s.scenario, 20) || 'unnamed',
    peak: endpointView(s.peak, FMT.smart),
    p99: endpointView(s.p99, FMT.smart),
    errorDelta: cell(s.d_error_rate, FMT.signedRatio),
    freshDelta: cell(s.d_fresh_hit_ratio, FMT.signedRatio),
    errorOk: typeof s.error_rate_ok === 'boolean' ? s.error_rate_ok : null,
    freshOk: typeof s.fresh_hit_ok === 'boolean' ? s.fresh_hit_ok : null,
  }));
  return {
    banner: str(result.banner, 400),
    protocol: str(result.protocol_version, 20),
    ruleSource: str(result.rule_source, 300),
    cells: cell(result.cells, FMT.count),
    expectedCells: cell(result.expected_cells, FMT.count),
    completenessProblems: Array.isArray(result.completeness_problems) ? result.completeness_problems.length : null,
    hypotheses: hyp,
    falsifiers,
    scenarios,
    peakChart: effectChartData(scenarios, 'peak'),
    p99Chart: effectChartData(scenarios, 'p99'),
  };
}

/** Rows and a common axis for a "mean with CI" chart. Only plots rows where mean, lo and hi are all numbers. */
export function effectChartData(scenarios, endpoint) {
  const rows = scenarios.map((s) => {
    const e = s[endpoint];
    const ok = e && e.mean.state === 'value' && e.lo.state === 'value' && e.hi.state === 'value';
    return { scenario: s.scenario, plotted: Boolean(ok), mean: ok ? e.mean.num : null, lo: ok ? e.lo.num : null, hi: ok ? e.hi.num : null, adverse: Boolean(e && e.adverse), supported: Boolean(e && e.supported), nullish: Boolean(e && e.practicallyNull) };
  });
  const vals = rows.filter((r) => r.plotted).flatMap((r) => [r.mean, r.lo, r.hi]);
  const domain = vals.length ? padDomain(Math.min(0, ...vals), Math.max(0, ...vals)) : null;
  return { rows, domain };
}

export function padDomain(min, max) {
  if (min === max) return { min: min - 1, max: max + 1 };
  const pad = (max - min) * 0.06;
  return { min: min - pad, max: max + pad };
}

/** Linear map from a data domain to a pixel range. Layout only; no metric is derived. */
export function scaleLinear(domain, range) {
  const [r0, r1] = range;
  const span = domain.max - domain.min;
  return (v) => (span === 0 ? r0 : r0 + ((v - domain.min) / span) * (r1 - r0));
}

// --- Scenario B (T2 loopback) -----------------------------------------------------------------------------------

const PAIRED_METRICS = Object.freeze([
  ['origin_peak_concurrency_window', 'Peak origin concurrency (whole window)', FMT.smart, 'count', 'lower is more origin protection'],
  ['origin_peak_concurrency_degraded', 'Peak origin concurrency (degraded phase)', FMT.smart, 'count', 'lower is more origin protection'],
  ['client_success_rate_pp', 'Client success rate (percentage points)', FMT.smart, 'pp', 'shed 503 responses count as failures'],
  ['shed_fraction_pp', 'Shed fraction (percentage points)', FMT.smart, 'pp', 'whole run'],
  ['client_p50_ms', 'Client P50 latency, ALL requests', FMT.smart, 'ms', 'includes fast-failing 503 sheds'],
  ['client_p95_ms', 'Client P95 latency, ALL requests', FMT.smart, 'ms', 'includes fast-failing 503 sheds'],
  ['client_p99_ms', 'Client P99 latency, ALL requests', FMT.smart, 'ms', 'includes fast-failing 503 sheds; not served-client latency'],
  ['origin_p99_service_ms', 'Origin P99 service time', FMT.smart, 'ms', 'scripted origin: differences are timer noise'],
  ['origin_requests_window', 'Origin requests in the window', FMT.smart, 'count', ''],
]);

function pairedRow(key, label, fmt, unit, note, m) {
  if (!isObj(m)) return { key, label, unit, note, present: false };
  const ci = Array.isArray(m.diff_ci95_bca) ? m.diff_ci95_bca : [];
  const rci = Array.isArray(m.relative_reduction_ci95_bca) ? m.relative_reduction_ci95_bca : [];
  return {
    key, label, unit, note, present: true,
    base: cell(m.mean_base, fmt), treat: cell(m.mean_treat, fmt),
    diff: cell(m.mean_diff_treat_minus_base, fmt), ciLo: cell(ci[0], fmt), ciHi: cell(ci[1], fmt),
    rel: cell(m.relative_reduction_mean, FMT.signedRatio), relLo: cell(rci[0], FMT.signedRatio), relHi: cell(rci[1], FMT.signedRatio),
    p: cell(m.permutation_p_sign_flip, FMT.p), n: isNum(m.n_pairs) ? m.n_pairs : null,
  };
}

export function scenarioBView(s) {
  if (!isObj(s)) return null;
  const armsRaw = isObj(s.arms) ? s.arms : {};
  const armOrder = ['BASE', 'STATIC-CAP', 'AIMD-CAP'].filter((a) => armsRaw[a]).concat(Object.keys(armsRaw).filter((a) => !['BASE', 'STATIC-CAP', 'AIMD-CAP'].includes(a)));
  const arms = armOrder.map((name) => {
    const a = isObj(armsRaw[name]) ? armsRaw[name] : {};
    const m = isObj(a.per_run_mean) ? a.per_run_mean : {};
    const lat = isObj(a.client_latency_ms_pooled_scheduled_time) ? a.client_latency_ms_pooled_scheduled_time : {};
    const missing = 'not recorded: the run ended before this value was produced';
    return {
      name,
      runs: isNum(a.runs) ? a.runs : null,
      validRuns: isNum(a.valid_runs) ? a.valid_runs : null,
      peakWindow: cell(m.origin_peak_concurrency_window, FMT.smart, missing),
      peakDegraded: cell(m.origin_peak_concurrency_degraded, FMT.smart, missing),
      success: cell(m.client_success_rate, FMT.ratio, missing),
      shed: cell(m.shed_fraction, FMT.ratio, missing),
      inFlightMax: cell(m.gateway_in_flight_max, FMT.smart, 'no gateway cap snapshot'),
      capMinSeen: cell(m.gateway_cap_min_seen, FMT.smart, name === 'BASE' ? 'BASE runs without a cap' : missing),
      rejections: cell(m.gateway_rejections_final, FMT.smart, 'no gateway counter recorded'),
      staleServed: cell(m.stale_served_total_final, FMT.smart, missing),
      latencyAll: { p50: cell(lat.p50, FMT.ms, missing), p95: cell(lat.p95, FMT.ms, missing), p99: cell(lat.p99, FMT.ms, missing), n: cell(lat.n, FMT.count, missing) },
      statusCounts: isObj(a.status_counts_total) ? Object.keys(a.status_counts_total).sort().map((k) => ({ status: clip(k, 8), count: isNum(a.status_counts_total[k]) ? formatCount(a.status_counts_total[k]) : 'not reported' })) : [],
    };
  });
  const pairedRaw = isObj(s.paired_vs_base) ? s.paired_vs_base : {};
  const paired = Object.keys(pairedRaw).map((arm) => ({
    arm,
    rows: PAIRED_METRICS.map(([key, label, fmt, unit, note]) => pairedRow(key, label, fmt, unit, note, pairedRaw[arm]?.[key])),
  }));
  const vRaw = isObj(s.verdicts) ? s.verdicts : {};
  const verdicts = Object.keys(vRaw).map((arm) => {
    const v = isObj(vRaw[arm]) ? vRaw[arm] : {};
    const ev = isObj(v.evidence) ? v.evidence : {};
    return {
      arm,
      outcome: verdictInfo(v.outcome),
      reasons: Array.isArray(v.reasons) ? v.reasons.map((r) => str(r, 500)).filter(Boolean) : [],
      validPairs: isNum(ev.n_valid_pairs) ? ev.n_valid_pairs : null,
      mechanismPass: typeof ev.mechanism_pass === 'boolean' ? ev.mechanism_pass : null,
      contentionFlagged: typeof ev.contention_flagged === 'boolean' ? ev.contention_flagged : null,
    };
  });
  const mech = isObj(s.mechanism_h_b1) ? Object.keys(s.mechanism_h_b1).map((arm) => {
    const m = s.mechanism_h_b1[arm];
    return { arm, pass: isObj(m) && typeof m.pass === 'boolean' ? m.pass : null, cap: cell(m?.cap_limit_in_force, FMT.count), observed: cell(m?.max_origin_concurrent_observed_whole_run, FMT.count), runs: isNum(m?.runs) ? m.runs : null, note: str(m?.note, 300) };
  }) : [];
  const cfg = isObj(s.config) ? s.config : {};
  const ph = isObj(cfg.phases) ? cfg.phases : {};
  const orig = isObj(cfg.origin) ? cfg.origin : {};
  const wl = isObj(cfg.workload) ? cfg.workload : {};
  return {
    label: str(s.label, 20),
    labelMeaning: str(s.label_meaning, 1500),
    labelReasons: Array.isArray(s.label_reasons) ? s.label_reasons.map((r) => str(r, 600)).filter(Boolean) : [],
    status: str(s.status, 30),
    // u01 contract (docs/research/u01_backend/contract.md): present only on summaries written by the patched backend.
    // null here means "the summary does not say" (legacy shape), not "computed".
    verdictsStatus: str(s.verdicts_status, 20),
    verdictsReason: str(s.verdicts_reason, 400),
    labelPlanned: str(s.label_planned, 20),
    completeness: isObj(s.run_completeness) ? {
      complete: typeof s.run_completeness.complete === 'boolean' ? s.run_completeness.complete : null,
      reason: str(s.run_completeness.incomplete_reason, 30),
      plannedRuns: isNum(s.run_completeness.planned_runs) ? s.run_completeness.planned_runs : null,
      completedRuns: isNum(s.run_completeness.completed_runs) ? s.run_completeness.completed_runs : null,
    } : null,
    tier: tierInfo(s.tier),
    minValidRuns: isNum(s.min_valid_runs_required) ? s.min_valid_runs_required : null,
    arms,
    paired,
    verdicts,
    mechanism: mech,
    mechanismAllPass: typeof s.mechanism_h_b1_all_pass === 'boolean' ? s.mechanism_h_b1_all_pass : null,
    limitations: Array.isArray(s.limitations) ? s.limitations.map((x) => str(x, 1600)).filter(Boolean) : [],
    interpretationNotes: Array.isArray(s.interpretation_notes) ? s.interpretation_notes.map((x) => str(x, 1600)).filter(Boolean) : [],
    config: {
      phases: [['Warm-up', ph.warmup_ms], ['Healthy', ph.healthy_ms], ['Degraded', ph.degraded_ms], ['Recovery', ph.recovery_ms]].map(([name, ms]) => ({ name, value: cell(ms, (n) => `${formatCount(n)} ms`) })),
      rate: cell(wl.rate_rps, FMT.smart),
      keys: cell(wl.n_keys, FMT.count),
      staticCap: cell(cfg.static_cap, FMT.count),
      aimdMin: cell(cfg.aimd_min, FMT.count),
      aimdMax: cell(cfg.aimd_max, FMT.count),
      reps: cell(cfg.reps, FMT.count),
      degradedMultiplier: cell(orig.degraded_mult, FMT.smart),
      maxAgeS: cell(orig.max_age_s, FMT.count),
    },
    provenanceNote: isObj(s.provenance) ? { whatRan: str(s.provenance.what_ran, 600), notCovered: str(s.provenance.not_covered, 600) } : null,
    charts: scenarioBCharts(arms),
  };
}

/** Chart inputs for Scenario B. Per-phase shed fractions are NOT in the payload and are reported as unavailable. */
export function scenarioBCharts(arms) {
  return {
    tradeoff: arms.map((a) => ({ arm: a.name, peak: a.peakWindow, success: a.success })),
    shed: arms.map((a) => ({ arm: a.name, whole: a.shed })),
    shedPerPhase: { available: false, reason: 'per-phase shed fractions are not part of the evidence or summary payload (per-repetition files are not exposed by the API); only the whole-run shed fraction is shown' },
    latency: arms.map((a) => ({ arm: a.name, all: a.latencyAll })),
    latencySuccessOnly: { available: false, reason: 'success-only (HTTP 200) latency is recorded per repetition by the runner but is not exposed by the API; do not read the all-request percentiles as served-client latency' },
  };
}

/** Older retained summaries predate the label wording; the evidence payload carries it in `interpretation`. */
function withLabelFallback(view, interp) {
  if (view && !view.labelMeaning && interp && typeof interp.label_meaning === 'string') return { ...view, labelMeaning: str(interp.label_meaning, 1500) };
  return view;
}

export function evidenceDetailView(d) {
  if (!isObj(d) || typeof d.id !== 'string') return null;
  const base = evidenceItemView(d);
  const prov = isObj(d.provenance) ? d.provenance : null;
  const interp = isObj(d.interpretation) ? d.interpretation : null;
  const result = isObj(d.recorded_result) ? d.recorded_result : null;
  const isSb = d.kind === 'scenario_b';
  const isM6 = d.kind === 'm6_sealed_evaluation' || d.kind === 'm6_dry_run';
  return {
    ...base,
    provenance: prov ? {
      manifestTier: str(prov.manifest_tier, 40),
      gitHead: str(prov.git_head, 12),
      profile: isObj(prov.profile) ? { driver: str(prov.profile.driver_profile, 20), gateway: str(prov.profile.gateway_profile, 20) } : null,
      reps: isNum(prov.reps) ? prov.reps : null,
      seedSet: str(prov.seed_set, 10),
      seedListSha256: str(prov.seed_list_sha256, 64),
      scenario: str(prov.scenario, 80),
      arms: Array.isArray(prov.arms) ? prov.arms.filter((x) => typeof x === 'string') : [],
      status: str(prov.status, 30),
    } : null,
    sealedNote: isObj(d.sealed) ? str(d.sealed.note, 300) : null,
    interpretation: interp ? {
      binding: interp.binding === true,
      mustAccompany: interp.must_accompany_any_citation === true,
      appliesTo: str(interp.notice_applies_to, 400),
      noticeText: typeof interp.notice_text === 'string' ? interp.notice_text.slice(0, 20000) : null,
      limitations: Array.isArray(interp.limitations) ? interp.limitations.map((x) => str(x, 1600)).filter(Boolean) : [],
      labelMeaning: str(interp.label_meaning, 1500),
    } : null,
    m6: isM6 && result ? m6DetailView(result) : null,
    sb: isSb && result ? withLabelFallback(scenarioBView(result), interp) : null,
    resultMissing: !result,
  };
}

// ---------------------------------------------------------------------------------------------------------------
// Runs

const SPEC_LABELS = Object.freeze({
  name: 'Name', scenarios: 'Scenarios', families: 'Families', arms: 'Arms', seed_set: 'Seed set', n_seeds: 'Seeds', profile: 'Profile',
  arm_params: 'Arm parameters', record_raw: 'Record raw CSVs', cells: 'Cells (estimated by the gateway)', repetitions: 'Repetitions',
  warmup_s: 'Warm-up (s)', healthy_s: 'Healthy (s)', degraded_s: 'Degraded (s)', recovery_s: 'Recovery (s)', rate_rps: 'Arrival rate (req/s)',
  n_keys: 'Key catalogue', static_cap: 'Static cap', aimd_min: 'AIMD minimum', aimd_max: 'AIMD maximum',
});

export function specRows(spec) {
  if (!isObj(spec)) return [];
  return Object.keys(spec).sort().map((k) => {
    const v = spec[k];
    let text;
    if (Array.isArray(v)) text = v.map((x) => clip(String(x), 40)).join(', ');
    else if (typeof v === 'boolean') text = v ? 'yes' : 'no';
    else if (isNum(v)) text = Number.isInteger(v) ? formatCount(v) : String(v);
    else text = clip(String(v), 80);
    return { key: k, label: SPEC_LABELS[k] || clip(k, 30), text };
  });
}

/** Milliseconds since the epoch for an RFC 3339 / ISO timestamp string from the gateway; null when absent or unparsable. */
export function parseUtcMs(raw) {
  if (typeof raw !== 'string' || !raw) return null;
  const t = Date.parse(raw);
  return Number.isFinite(t) ? t : null;
}

/**
 * Elapsed time of a run, computed ONLY from the gateway's own timestamps (and, while the run is active, the browser clock).
 * The caller labels the value DERIVED ("from timestamps"). Returns { state: 'value', text, basis } or { state: 'unavailable', reason }.
 * Never invents a start: no started_utc means unavailable. A negative span (clock disagreement) is unavailable, not zero.
 */
export function elapsedView({ startedMs, endedMs, nowMs, terminal }) {
  if (startedMs === null || startedMs === undefined) return { state: 'unavailable', reason: 'the gateway has not reported a start time' };
  let end;
  let basis;
  if (terminal) {
    if (endedMs === null || endedMs === undefined) return { state: 'unavailable', reason: 'the gateway reported no end time' };
    end = endedMs;
    basis = 'gateway start and end timestamps';
  } else {
    if (!Number.isFinite(nowMs)) return { state: 'unavailable', reason: 'the browser clock is not available' };
    end = nowMs;
    basis = 'gateway start timestamp and this browser clock (the two clocks may differ)';
  }
  const ms = end - startedMs;
  if (!Number.isFinite(ms) || ms < 0) return { state: 'unavailable', reason: 'the timestamps disagree (the clocks differ)' };
  return { state: 'value', ms, text: formatElapsed(ms), basis };
}

export function formatElapsed(ms) {
  const total = Math.floor(ms / 1000);
  if (total < 60) return `${total} s`;
  const m = Math.floor(total / 60);
  const sec = total % 60;
  if (m < 60) return `${m} min ${sec} s`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

export function runView(r) {
  if (!isObj(r) || typeof r.run_id !== 'string') return null;
  const prog = isObj(r.progress) ? r.progress : {};
  const interp = isObj(r.interpretation) ? r.interpretation : null;
  const idx = isObj(r.artifact_index) ? r.artifact_index : {};
  return {
    id: r.run_id,
    href: `#/lab/runs/${encodeURIComponent(r.run_id)}`,
    state: str(r.state, 20),
    kind: kindInfo(r.kind),
    tier: tierInfo(r.tier),
    created: utcText(r.created_utc),
    started: utcText(r.started_utc, 'not started'),
    ended: utcText(r.ended_utc, 'not ended'),
    startedMs: parseUtcMs(r.started_utc),
    endedMs: parseUtcMs(r.ended_utc),
    exitCode: isNum(r.exit_code) ? r.exit_code : null,
    buildProfile: str(r.build_profile, 20),
    progress: {
      done: isNum(prog.done) ? prog.done : null,
      total: isNum(prog.total) ? prog.total : null,
    },
    spec: specRows(r.spec),
    specRaw: isObj(r.spec) ? r.spec : null,
    error: isObj(r.error) ? { code: str(r.error.code, 40) || 'unspecified', message: str(r.error.message, 400) || '' } : null,
    persistenceErrors: isNum(r.persistence_errors) ? r.persistence_errors : null,
    hasSummaryArtifact: Boolean(idx['summary.json']),
    hasCells: Boolean(idx['cells.csv']),
    interpretation: interp ? { labelMeaning: str(interp.label_meaning, 1500), limitations: Array.isArray(interp.limitations) ? interp.limitations.map((x) => str(x, 1600)).filter(Boolean) : [], notice: str(interp.notice, 300) } : null,
    recovery: isObj(r.recovery) ? { gatewayReap: str(r.recovery.gateway_reap, 100) } : null,
  };
}

export function runsListView(p) {
  if (!isObj(p) || !Array.isArray(p.items)) return null;
  const items = p.items.filter((i) => isObj(i) && typeof i.run_id === 'string').map(runView);
  return { items, total: isNum(p.indexed) ? p.indexed : items.length, returned: items.length, limit: isNum(p.limit) ? p.limit : null, offset: isNum(p.offset) ? p.offset : null, boundedTo: isNum(p.bounded_to_newest) ? p.bounded_to_newest : null };
}

export function artifactsView(p) {
  if (!isObj(p) || !Array.isArray(p.artifacts)) return null;
  return p.artifacts.filter((a) => isObj(a) && typeof a.name === 'string').map((a) => {
    const hashRaw = a.sha256;
    return {
      name: clip(a.name, 64),
      bytes: isNum(a.bytes) ? FMT.bytes(a.bytes) : 'size not reported',
      rawBytes: isNum(a.bytes) ? a.bytes : null,
      contentType: str(a.content_type, 60),
      downloadable: a.downloadable === true,
      sha256: typeof hashRaw === 'string' ? { state: 'value', text: hashRaw } : { state: 'unavailable', text: 'Unavailable', reason: unavailableReason(hashRaw, 'no hash reported') },
      sha256Of: str(a.sha256_of, 80),
      tier: a.tier ? tierInfo(a.tier) : null,
    };
  });
}

/** T1 matrix result as reported by the `result` event: a cell count and the hash of cells.csv, nothing else. */
export function t1ResultView(result) {
  if (!isObj(result)) return null;
  return {
    cells: cell(result.cells, FMT.count),
    cellsSha256: typeof result.cells_csv_sha256 === 'string' ? result.cells_csv_sha256 : null,
    note: str(result.note, 300),
    tier: tierInfo(result.tier),
  };
}

/** Live Scenario B result event: label and reasons; the numeric summary comes from the summary.json artifact. */
export function resultEventLabel(result) {
  if (!isObj(result)) return null;
  return {
    label: str(result.label, 20),
    labelMeaning: str(result.label_meaning, 1500),
    labelReasons: Array.isArray(result.label_reasons) ? result.label_reasons.map((r) => str(r, 600)).filter(Boolean) : [],
    limitations: Array.isArray(result.limitations) ? result.limitations.map((x) => str(x, 1600)).filter(Boolean) : [],
  };
}

export function progressText(p) {
  if (!p) return null;
  const bits = [];
  if (p.arm) bits.push(`arm ${p.arm}`);
  if (Number.isFinite(p.rep)) bits.push(`repetition ${p.rep}`);
  if (p.phase) bits.push(`phase ${p.phase}`);
  if (Number.isFinite(p.done) && Number.isFinite(p.total)) bits.push(`${formatCount(p.done)} of ${formatCount(p.total)} units done`);
  return bits.join(', ');
}
