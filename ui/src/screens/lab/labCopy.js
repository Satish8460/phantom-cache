// Lab copy and small pure helpers shared by the Lab screens (increment V2A). No hooks, no I/O.
//
// QUESTION_BY_KIND: the one-sentence question of each experiment (DD v2 8.1, 10.1). The wording is taken from the sentences
// the Lab already shows on the About tab (StoryView: "The open question is whether a controller that adjusts that cap ...",
// "H1: an origin-aware adaptive concurrency controller beats the best static configuration") and from the DD v2 example for
// Scenario B. The lead has to confirm the Scenario B sentence (DD v2 8.1 says "lead to confirm the source"); it asserts no result.

export const QUESTION_BY_KIND = Object.freeze({
  scenario_b: 'Does a cap on in-flight origin requests protect the origin when it slows down, and what does it cost clients?',
  m6_sealed_evaluation: 'Does an origin-aware adaptive concurrency controller beat the best static configuration (hypothesis H1)?',
  m6_dry_run: 'Does an origin-aware adaptive concurrency controller beat the best static configuration (hypothesis H1)? This is a dry run, not the registered evaluation.',
});

export const MEASURED_BY_KIND = Object.freeze({
  scenario_b: 'Three arms (no cap, a static cap, an adaptive cap) against a scripted origin that slows down, with real gateway processes on one host over loopback. Each capped arm is paired with BASE by repetition.',
  m6_sealed_evaluation: 'A deterministic virtual-time simulation (tier 1) over pre-registered scenarios and seeds. Nothing was run on a real network, host or process.',
  m6_dry_run: 'A deterministic virtual-time simulation (tier 1) on development seeds. Nothing was run on a real network, host or process.',
});

/** Statements that hold for every Scenario B result page (the scenario never runs these cases). Shown always, in the "Not established" strip. */
export const NOT_ESTABLISHED_ALWAYS = Object.freeze([
  'Anything about production, wide-area networks or multi-host deployments: this is one host, a scripted origin and loopback.',
  'That clients who were served got faster: the latency percentiles include fast-failing 503 responses.',
  'That stale serving works under overload: the Scenario B workload does not exercise it.',
]);

/** Additional statements that belong to the retained Scenario B evidence (already shown on the About tab, "What this does not show"). */
export const NOT_ESTABLISHED_RETAINED = Object.freeze([
  'That adaptive control is useless in general: one controller design, one objective and two stand-in origins were tested.',
  'That the cap values were chosen in advance: they were picked after a pilot run.',
]);

/**
 * scenarioBView() keeps no raw summary. When a screen has only the view, rebuild just the keys resultPresentation() reads, so the
 * rule sees the same status, completeness and verdict signals. Arms are reduced to presence only.
 */
export function viewAsSummary(view) {
  if (!view || typeof view !== 'object') return null;
  const out = {
    status: view.status === null || view.status === undefined ? undefined : view.status,
    arms: Object.fromEntries((view.arms || []).map((a) => [a.name, {}])),
  };
  if (view.label) out.label = view.label;
  if (view.labelPlanned) out.label_planned = view.labelPlanned;
  if (view.labelReasons && view.labelReasons.length) out.label_reasons = view.labelReasons;
  if (view.verdictsStatus) out.verdicts_status = view.verdictsStatus;
  if (view.verdictsReason) out.verdicts_reason = view.verdictsReason;
  if (view.completeness) {
    out.run_completeness = {
      complete: view.completeness.complete === null ? undefined : view.completeness.complete,
      completed_runs: view.completeness.completedRuns === null ? undefined : view.completeness.completedRuns,
      planned_runs: view.completeness.plannedRuns === null ? undefined : view.completeness.plannedRuns,
      incomplete_reason: view.completeness.reason || undefined,
    };
  }
  return out;
}
