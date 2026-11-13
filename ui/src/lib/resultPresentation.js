// Result presentation rule (U-01): may a verdict, a mechanism verdict ("H-B1 falsified") or a run-class label be shown?
// Pure and framework free, so server-side render tests and the live page cannot diverge.
//
// Rule (docs/design/remediation/06_functional_ux_work_breakdown.md section 3, docs/research/u01_backend/contract.md):
//   1. Run state decides first: only a `completed` run may show verdicts. cancelled / failed / interrupted never do.
//      queued / running / cancelling are "waiting". An unknown or missing state shows no verdict.
//   2. Then summary.status: it must be exactly "COMPLETE".
//   3. Then the new-contract guards: verdicts_status === "not_computed" or run_completeness.complete === false.
//   The PRESENCE or SHAPE of `verdicts` is never the deciding signal: a legacy cancelled summary carries verdicts
//   (including MECHANISM_FAILED for "zero runs recorded") and a VALID label, and both are distrusted.
// Raw diagnostics stay accessible: `rawDiagnostics.available` says whether partial data exists to be shown (demoted).

const isObj = (x) => x !== null && typeof x === 'object' && !Array.isArray(x);
const isNum = (x) => typeof x === 'number' && Number.isFinite(x);

export const WAITING_STATES = Object.freeze(['queued', 'running', 'cancelling']);
export const INCOMPLETE_TERMINAL_STATES = Object.freeze(['cancelled', 'failed', 'interrupted']);
export const KNOWN_RUN_STATES = Object.freeze([...WAITING_STATES, 'completed', ...INCOMPLETE_TERMINAL_STATES]);

export const RAW_DIAGNOSTICS_LABEL = 'Partial diagnostic data (not a result)';

const ENDED_WORD = Object.freeze({ cancelled: 'cancelled', failed: 'failed', interrupted: 'was interrupted' });

/** "No verdict: the run cancelled before finishing" / "...failed before finishing" / "...was interrupted before finishing". */
function incompleteHeadline(state) {
  if (state === 'cancelled') return 'No verdict: the run cancelled before finishing';
  if (state === 'failed') return 'No verdict: the run failed before finishing';
  if (state === 'interrupted') return 'No verdict: the run was interrupted before finishing';
  return 'No verdict: the run did not finish';
}

function countsDetail(done, total) {
  if (isNum(done) && isNum(total) && total > 0) {
    return `${done} of ${total} planned runs had finished when it stopped. A verdict needs every planned repetition.`;
  }
  return 'A verdict needs every planned repetition.';
}

/** Where the "N of M" numbers come from: the summary's own run_completeness (new contract), else the run's progress. */
function finishedCounts(summary, progress) {
  const rc = isObj(summary) && isObj(summary.run_completeness) ? summary.run_completeness : null;
  if (rc && isNum(rc.completed_runs) && isNum(rc.planned_runs)) return { done: rc.completed_runs, total: rc.planned_runs, source: 'summary' };
  if (isObj(progress) && isNum(progress.done) && isNum(progress.total)) return { done: progress.done, total: progress.total, source: 'run_progress' };
  return { done: null, total: null, source: null };
}

/** Which shape the summary has. 'new' = has the u01 contract keys; 'legacy' = neither; null = not an object. */
export function summaryShape(summary) {
  if (!isObj(summary)) return null;
  return 'verdicts_status' in summary || 'run_completeness' in summary ? 'new' : 'legacy';
}

function labelFacts(summary, incomplete) {
  if (!isObj(summary)) return { label: null, plannedLabel: null, labelTrusted: false, labelReasons: [] };
  const raw = typeof summary.label === 'string' ? summary.label : null;
  const planned = typeof summary.label_planned === 'string' ? summary.label_planned : raw;
  const reasons = Array.isArray(summary.label_reasons) ? summary.label_reasons.filter((r) => typeof r === 'string') : [];
  if (!incomplete) return { label: raw, plannedLabel: planned, labelTrusted: raw !== null, labelReasons: reasons };
  // Incomplete run: only EXPLORATORY can be shown as the effective class. A VALID on an incomplete run is a planned
  // label that does not apply; it is exposed as plannedLabel for the disclosure, never as the label of the result.
  const effective = raw === 'EXPLORATORY' ? 'EXPLORATORY' : null;
  return { label: effective, plannedLabel: planned, labelTrusted: false, labelReasons: reasons };
}

function build(base) {
  return Object.freeze({
    kind: 'unavailable',
    mode: 'unavailable',
    reason: 'unknown',
    headline: '',
    detail: '',
    tone: 'neutral',
    showVerdicts: false,
    showMechanismVerdict: false,
    showLabel: false,
    label: null,
    plannedLabel: null,
    labelTrusted: false,
    labelReasons: [],
    legacy: false,
    shape: null,
    fetchSummary: false,
    rawDiagnostics: Object.freeze({ available: false, label: RAW_DIAGNOSTICS_LABEL }),
    counts: Object.freeze({ done: null, total: null, source: null }),
    ...base,
  });
}

/**
 * @param {string|null|undefined} runState  queued|running|cancelling|completed|cancelled|failed|interrupted (anything else = unknown)
 * @param {object|null|undefined} summary   parsed summary.json (Scenario B), or null/undefined when not loaded
 * @param {{runKind?: string, progress?: {done?: number, total?: number}}} [opts]
 */
export function resultPresentation(runState, summary, opts = {}) {
  const state = typeof runState === 'string' ? runState : null;
  const runKind = opts.runKind || 'scenario_b';
  const shape = summaryShape(summary);
  const hasPartialData = isObj(summary) && isObj(summary.arms) && Object.keys(summary.arms).length > 0;

  // T1 matrix: the API computes no verdict for it, in any state.
  if (runKind === 't1_matrix') {
    const finished = state === 'completed';
    return build({
      kind: 'unavailable',
      mode: finished ? 'matrix' : WAITING_STATES.includes(state) ? 'waiting' : 'no_verdict',
      reason: finished ? 't1_has_no_verdict' : WAITING_STATES.includes(state) ? 'run_in_progress' : INCOMPLETE_TERMINAL_STATES.includes(state) ? `run_${state}` : 'state_unknown',
      headline: finished ? 'No verdict is computed for a matrix run' : INCOMPLETE_TERMINAL_STATES.includes(state) ? `No result: the run ${ENDED_WORD[state]} before finishing` : '',
      shape,
    });
  }

  if (state === null || !KNOWN_RUN_STATES.includes(state)) {
    return build({
      kind: 'unavailable',
      mode: 'no_verdict',
      reason: 'state_unknown',
      headline: 'No verdict: the run state is not known to this page',
      detail: 'A verdict is shown only for a run reported as completed.',
      shape,
      legacy: shape === 'legacy',
      rawDiagnostics: Object.freeze({ available: hasPartialData, label: RAW_DIAGNOSTICS_LABEL }),
      ...labelFacts(summary, true),
      showLabel: false,
    });
  }

  if (WAITING_STATES.includes(state)) {
    return build({
      kind: 'unavailable',
      mode: 'waiting',
      reason: 'run_in_progress',
      headline: 'The result appears here when the run completes',
      shape,
      fetchSummary: false,
    });
  }

  if (INCOMPLETE_TERMINAL_STATES.includes(state)) {
    const counts = finishedCounts(summary, opts.progress);
    const lf = labelFacts(summary, true);
    return build({
      kind: 'incomplete',
      mode: 'no_verdict',
      reason: `run_${state}`,
      headline: incompleteHeadline(state),
      detail: countsDetail(counts.done, counts.total),
      tone: state === 'failed' ? 'critical' : state === 'interrupted' ? 'warn' : 'neutral',
      showVerdicts: false,
      showMechanismVerdict: false,
      ...lf,
      showLabel: lf.label === 'EXPLORATORY',
      legacy: shape === 'legacy',
      shape,
      // D-13: a failed run writes no summary and an interrupted run may carry a stale COMPLETE one; never auto-fetch them.
      fetchSummary: state === 'cancelled',
      rawDiagnostics: Object.freeze({ available: hasPartialData && state === 'cancelled', label: RAW_DIAGNOSTICS_LABEL, listedOnlyInArtifacts: hasPartialData && state !== 'cancelled' }),
      counts,
    });
  }

  // state === 'completed'
  if (summary === null || summary === undefined) {
    return build({ kind: 'unavailable', mode: 'unavailable', reason: 'summary_missing', headline: 'No result summary for this run', detail: 'The run completed but its summary is not available. No verdict is shown.', fetchSummary: true });
  }
  if (!isObj(summary)) {
    return build({ kind: 'unavailable', mode: 'unavailable', reason: 'summary_malformed', headline: 'The result summary was not in the expected format', detail: 'No verdict is shown.', fetchSummary: true });
  }

  const counts = finishedCounts(summary, opts.progress);
  const rc = isObj(summary.run_completeness) ? summary.run_completeness : null;
  const incompleteBy = (reason, detail) => {
    const lf = labelFacts(summary, true);
    return build({
      kind: 'incomplete', mode: 'no_verdict', reason, headline: 'No verdict: the result is marked incomplete', detail,
      tone: 'warn', ...lf, showLabel: lf.label === 'EXPLORATORY', legacy: shape === 'legacy', shape, fetchSummary: true, counts,
      rawDiagnostics: Object.freeze({ available: hasPartialData, label: RAW_DIAGNOSTICS_LABEL }),
    });
  };

  if (summary.verdicts_status === 'not_computed') {
    return incompleteBy('verdicts_not_computed', typeof summary.verdicts_reason === 'string' && summary.verdicts_reason ? summary.verdicts_reason : countsDetail(counts.done, counts.total));
  }
  if (rc && rc.complete === false) {
    return incompleteBy('run_completeness_incomplete', countsDetail(counts.done, counts.total));
  }
  if (typeof summary.status !== 'string') {
    return build({ kind: 'unavailable', mode: 'unavailable', reason: 'summary_status_unknown', headline: 'No verdict: the summary does not say whether the run was complete', detail: 'A verdict needs summary status COMPLETE.', legacy: shape === 'legacy', shape, fetchSummary: true, rawDiagnostics: Object.freeze({ available: hasPartialData, label: RAW_DIAGNOSTICS_LABEL }) });
  }
  if (summary.status !== 'COMPLETE') {
    return incompleteBy(`summary_status_${summary.status.toLowerCase().replace(/[^a-z0-9]+/g, '_').slice(0, 30) || 'other'}`, `The run is recorded as completed but its summary says ${summary.status.slice(0, 30)}.`);
  }
  // Guard: a COMPLETE summary whose `verdicts` is present but not an object (null) while verdicts_status is not 'computed'
  // would be a broken producer; show no verdict rather than a "needs a BASE arm" message.
  if ('verdicts' in summary && summary.verdicts !== undefined && !isObj(summary.verdicts)) {
    return build({ kind: 'unavailable', mode: 'unavailable', reason: 'verdicts_malformed', headline: 'No verdict: the summary has no readable verdicts', detail: 'The summary says COMPLETE but its verdicts field is not an object.', shape, legacy: shape === 'legacy', fetchSummary: true, rawDiagnostics: Object.freeze({ available: hasPartialData, label: RAW_DIAGNOSTICS_LABEL }), ...labelFacts(summary, true), showLabel: false });
  }
  const lf = labelFacts(summary, false);
  return build({
    kind: 'verdicts', mode: 'verdict', reason: 'run_completed', headline: '', detail: '', tone: 'neutral',
    showVerdicts: true, showMechanismVerdict: true, ...lf, showLabel: lf.label !== null,
    legacy: shape === 'legacy', shape, fetchSummary: true, counts,
    rawDiagnostics: Object.freeze({ available: hasPartialData, label: RAW_DIAGNOSTICS_LABEL }),
  });
}

/**
 * Defence in depth for screens: given the output of labView.scenarioBView() and a presentation, return a view that cannot render
 * a verdict or a mechanism pass/fail when the presentation forbids it. Raw per-arm data, limitations and settings are kept.
 */
export function applyPresentation(view, presentation) {
  if (!isObj(view)) return view;
  if (presentation && presentation.showVerdicts) return view;
  return {
    ...view,
    verdicts: [],
    mechanism: Array.isArray(view.mechanism) ? view.mechanism.map((m) => ({ ...m, pass: null })) : [],
    mechanismAllPass: null,
    label: presentation && presentation.showLabel ? view.label : null,
    labelReasons: presentation && presentation.showLabel ? view.labelReasons : [],
    withheld: true,
  };
}
