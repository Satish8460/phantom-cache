// Tier and status vocabulary for the Phantom Lab screens. Pure data; no React.
// Tiers come from the Lab API ("tier" fields).
//
// Display vocabulary (owner decision 2026-10-30, lead decision (a); design spec section 11):
// REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED, UNAVAILABLE.
//  * MEASURED_* tiers display as REAL, ALWAYS with a basis qualifier (loopback, one host). The build profile is added by
//    the caller when it is known (tierBadgeQualifier). A three-process loopback run is never drawn as three hosts.
//  * RETAINED_* is NOT part of the provenance word: it is a separate neutral tag (`retained: true`) next to the badge.
//  * REPLAYED and PREDICTED are reserved: the backend supplies no such data, so no tier here maps to them.
//
// DEC-007 (owner, approved 2026-10-30), encoded here and in test/func.provenance.test.mjs:
//  * Retained Scenario B summary evidence = REAL (with its basis qualifier) + the separate RETAINED tag. It is NOT REPLAYED.
//  * REPLAYED is only for data RECONSTRUCTED or RE-RENDERED from stored experiment artifacts (for example a time series drawn from
//    downloaded files). Displaying a stored summary value as stored is not a replay.
//  * PREDICTED only when real predictive functionality and evidence exist. The backend supplies none today.
//  * REPLAYED and PREDICTED stay available as labels (ProvenanceBadge); no Lab tier maps to them.
// `label` is the plain-text form (tooltips, tests, screen readers); `word` and `qualifier` are what the badge draws.

export const REAL_QUALIFIER = 'loopback, one host';

function tier(src, word, qualifier, retained, meaning) {
  return { src, word, qualifier, retained, label: qualifier ? `${word} (${qualifier})` : word, meaning };
}

export const TIERS = Object.freeze({
  MEASURED_LOOPBACK: tier('measured', 'Real', REAL_QUALIFIER, false,
    'Real: measured on real gateway processes over loopback on one host with a scripted origin. Not a network, WAN, or production measurement.'),
  SIMULATED: tier('simulated', 'Simulated', null, false,
    'Simulated: deterministic virtual-time simulation driving the real cache and executor code. Not a measurement of any network, host or process.'),
  RETAINED_SIMULATED: tier('simulated', 'Simulated', null, true,
    'Simulated, read from a retained experiment folder. The experiment was a simulation (tier SIMULATED). The page only displays it.'),
  RETAINED_MEASURED_LOOPBACK: tier('measured', 'Real', REAL_QUALIFIER, true,
    'Real, read from a retained experiment folder. The experiment was measured on real processes over loopback on one host.'),
  DERIVED: tier('derived', 'Derived', null, false,
    'Derived: computed by the experiment code from measured or simulated samples (paired differences, intervals, verdict rules). Not a raw reading.'),
  UNAVAILABLE: tier('unavailable', 'Unavailable', null, false,
    'Unavailable: no value exists in the payload. The reason is stated next to it. It is not zero.'),
});

export function tierInfo(t) {
  if (typeof t === 'string' && Object.prototype.hasOwnProperty.call(TIERS, t)) return { key: t, ...TIERS[t] };
  return { key: 'UNAVAILABLE', ...TIERS.UNAVAILABLE, qualifier: 'unknown tier', label: 'Unavailable (unknown tier)', unknownTier: typeof t === 'string' ? t.slice(0, 40) : null };
}

/** Qualifier text for a badge: the tier's basis, plus ', debug build' when the caller knows the build was a debug build. */
export function tierBadgeQualifier(info, build) {
  const base = info.qualifier || null;
  if (info.src === 'measured' && build === 'debug') return base ? `${base}, debug build` : 'debug build';
  return base;
}

/** Run states from the API state machine: queued -> running -> cancelling -> completed|failed|cancelled|interrupted. */
export const RUN_STATES = Object.freeze({
  queued: { label: 'Queued', tone: 'info', icon: 'clock', terminal: false, meaning: 'Accepted, waiting for the runner.' },
  running: { label: 'Running', tone: 'info', icon: 'loading', terminal: false, meaning: 'The runner is executing.' },
  cancelling: { label: 'Cancelling', tone: 'warn', icon: 'warn', terminal: false, meaning: 'A cancel request was accepted; the runner stops cooperatively (a simulation stops at the next cell boundary).' },
  completed: { label: 'Completed', tone: 'ok', icon: 'ok', terminal: true, meaning: 'The runner finished and wrote its artifacts.' },
  failed: { label: 'Failed', tone: 'critical', icon: 'error', terminal: true, meaning: 'The runner failed or crashed. The server never reports a crash as completed.' },
  cancelled: { label: 'Cancelled', tone: 'neutral', icon: 'empty', terminal: true, meaning: 'Stopped on request. Artifacts, if any, are partial.' },
  interrupted: { label: 'Interrupted', tone: 'warn', icon: 'disconnected', terminal: true, meaning: 'The gateway stopped or restarted while the run was active. The run was not resumed and has no result.' },
});

export function runStateInfo(state) {
  if (typeof state === 'string' && Object.prototype.hasOwnProperty.call(RUN_STATES, state)) return { key: state, ...RUN_STATES[state] };
  return { key: 'unknown', label: typeof state === 'string' ? `Unknown state: ${state.slice(0, 24)}` : 'State not reported', tone: 'neutral', icon: 'info', terminal: false, meaning: 'The gateway reported a state this page does not know.' };
}

/**
 * Verdict outcomes. Every outcome is drawn by the same component at the same size; only the word, glyph and tone
 * differ, so a negative or inconclusive result is exactly as prominent as a positive one.
 */
export const VERDICTS = Object.freeze({
  SUPPORTED: { label: 'Supported', tone: 'ok', icon: 'ok', kind: 'positive' },
  BENEFIT_SUPPORTED: { label: 'Benefit supported', tone: 'ok', icon: 'ok', kind: 'positive' },
  NOT_SUPPORTED: { label: 'Not supported', tone: 'critical', icon: 'error', kind: 'negative' },
  BENEFIT_NOT_SUPPORTED: { label: 'Benefit not supported', tone: 'critical', icon: 'error', kind: 'negative' },
  TRADE_OFF_FAILURE: { label: 'Trade-off failure', tone: 'critical', icon: 'error', kind: 'negative' },
  MECHANISM_FAILED: { label: 'Mechanism failed', tone: 'critical', icon: 'error', kind: 'negative' },
  INCONCLUSIVE: { label: 'Inconclusive', tone: 'warn', icon: 'info', kind: 'inconclusive' },
  NOT_TESTABLE_AT_REGISTERED_HORIZON: { label: 'Not testable at the registered horizon', tone: 'warn', icon: 'info', kind: 'inconclusive' },
  UNVERIFIED: { label: 'Unverified', tone: 'warn', icon: 'info', kind: 'inconclusive' },
});

export function verdictInfo(outcome) {
  if (typeof outcome === 'string' && Object.prototype.hasOwnProperty.call(VERDICTS, outcome)) return { key: outcome, ...VERDICTS[outcome] };
  if (typeof outcome === 'string' && outcome) return { key: outcome, label: outcome.replace(/_/g, ' ').toLowerCase(), tone: 'neutral', icon: 'info', kind: 'other' };
  return { key: 'NONE', label: 'No verdict recorded', tone: 'neutral', icon: 'unavailable', kind: 'none' };
}

/** Experiment labels (Scenario B). VALID has a very narrow meaning, which the payload states and the page shows. */
export function labelInfo(label) {
  if (label === 'VALID') return { key: 'VALID', label: 'Run class: VALID (narrow meaning)', tone: 'info', icon: 'info', hint: 'VALID describes how the run was set up (builds, repetitions, workload). It is not a success verdict; read the verdicts next to it.' };
  if (label === 'EXPLORATORY') return { key: 'EXPLORATORY', label: 'Run class: EXPLORATORY', tone: 'warn', icon: 'warn', hint: null };
  return { key: 'NONE', label: typeof label === 'string' && label ? `Run class: ${label}` : 'Run class not reported', tone: 'neutral', icon: 'unavailable', hint: null };
}

export const KIND_TEXT = Object.freeze({
  t1_matrix: { label: 'Simulation matrix (T1)', short: 'T1 matrix', tier: 'SIMULATED' },
  scenario_b: { label: 'Scenario B: origin slowdown (T2)', short: 'Scenario B', tier: 'MEASURED_LOOPBACK' },
  m6_sealed_evaluation: { label: 'M6 sealed evaluation', short: 'M6 sealed', tier: 'RETAINED_SIMULATED' },
  m6_dry_run: { label: 'M6 dry run (development seeds)', short: 'M6 dry run', tier: 'RETAINED_SIMULATED' },
  tuning: { label: 'Tuning run', short: 'Tuning', tier: 'RETAINED_SIMULATED' },
  smoke: { label: 'Smoke run', short: 'Smoke', tier: 'RETAINED_SIMULATED' },
});

export function kindInfo(kind) {
  if (typeof kind === 'string' && Object.prototype.hasOwnProperty.call(KIND_TEXT, kind)) return { key: kind, ...KIND_TEXT[kind] };
  return { key: String(kind || 'unknown'), label: typeof kind === 'string' && kind ? `Experiment kind: ${kind.slice(0, 40)}` : 'Unknown kind', short: 'Other', tier: null };
}

/** Evidence kinds that are exploratory by nature (not a registered, gated experiment). */
export function isExploratoryKind(kind) {
  return kind === 'm6_dry_run' || kind === 'tuning' || kind === 'smoke';
}
