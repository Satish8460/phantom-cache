// Honest progress for a Lab run (F6 model, docs/design/remediation/06 section 6.1; D-9, D-15).
// Pure. Only shows what the gateway's events support:
//   * counts come from progress.done / progress.total (gateway-reported). One Scenario B "run" = one arm x one repetition = one
//     fresh gateway process; a matrix run counts "cells".
//   * the phase (warmup, healthy, degraded, recovery) is a step out of four; other phases are named, not counted.
//   * "at least X left by plan" is a LOWER BOUND from the planned phase lengths minus elapsed. There is never an ETA, never a
//     weighted bar, and nothing is derived when an input is missing (the reason is returned instead).
//   * a bar is drawn only when it carries information: never for 0 of N (that looks stalled while a run is in progress).

import { formatElapsed } from './labView.js';

const isNum = (x) => typeof x === 'number' && Number.isFinite(x);

export const PHASE_STEPS = Object.freeze(['warmup', 'healthy', 'degraded', 'recovery']);
const PHASE_LABEL = Object.freeze({
  starting: 'Starting the gateway',
  warmup: 'Warm-up',
  healthy: 'Healthy',
  degraded: 'Degraded',
  recovery: 'Recovery',
  drain: 'Collecting results',
  collect: 'Collecting results',
  'run-complete': 'Run finished',
});
export const UNIT_DEFINITION = 'One run = one arm in one repetition = one fresh gateway process.';

const WAITING = ['queued', 'running', 'cancelling'];

/** Planned minimum seconds = total runs x (sum of the four phase lengths). Lower bound: start-up, drain and shut-down add time. */
export function plannedMinimum(spec, total) {
  if (!spec || typeof spec !== 'object') return { state: 'unavailable', reason: 'the run settings are not known' };
  if (!isNum(total) || total <= 0) return { state: 'unavailable', reason: 'the number of runs is not known yet' };
  const lens = ['warmup_s', 'healthy_s', 'degraded_s', 'recovery_s'].map((k) => spec[k]);
  if (!lens.every((v) => isNum(v) && v >= 0)) return { state: 'unavailable', reason: 'the gateway has not reported all four phase lengths' };
  const perRun = lens.reduce((a, b) => a + b, 0);
  return { state: 'value', seconds: perRun * total, perRunSeconds: perRun, text: `${formatElapsed(perRun * total * 1000)}` };
}

/**
 * @param {{kind?: string, state?: string, progress?: {done?: number|null, total?: number|null, arm?: string|null, rep?: number|null, phase?: string|null}|null,
 *          spec?: object|null, startedMs?: number|null, endedMs?: number|null, nowMs?: number|null, lastEventAtMs?: number|null}} input
 *   `progress` is the live model's progress when present, else the run record's {done, total}.
 */
export function progressView(input = {}) {
  const { kind = 'scenario_b', state = null, spec = null, startedMs = null, endedMs = null, nowMs = null, lastEventAtMs = null } = input;
  const p = input.progress && typeof input.progress === 'object' ? input.progress : {};
  const isMatrix = kind === 't1_matrix';
  const unit = isMatrix ? 'cell' : 'run';
  const unitPlural = isMatrix ? 'cells' : 'runs';
  const done = isNum(p.done) ? p.done : null;
  const total = isNum(p.total) && p.total > 0 ? p.total : null;
  const terminal = state !== null && !WAITING.includes(state);
  const counts = done !== null && total !== null ? { done, total } : null;

  const phaseName = typeof p.phase === 'string' ? p.phase : null;
  const stepIndex = phaseName ? PHASE_STEPS.indexOf(phaseName) : -1;
  const phase = phaseName
    ? { name: phaseName, label: PHASE_LABEL[phaseName] || phaseName, step: stepIndex >= 0 ? stepIndex + 1 : null, steps: stepIndex >= 0 ? PHASE_STEPS.length : null }
    : null;

  const repTotal = spec && isNum(spec.repetitions) ? spec.repetitions : null;
  const current = !isMatrix && counts && !terminal && done < total
    ? { index: done + 1, total, arm: typeof p.arm === 'string' ? p.arm : null, rep: isNum(p.rep) ? p.rep : null, repTotal }
    : null;

  // Bar: only informative counts. Never an empty bar for 0 of N, never for a terminal run (frozen text instead).
  const bar = { render: Boolean(counts) && !terminal && done > 0, value: counts ? done : null, max: counts ? total : null };

  // Text lines
  let text;
  if (terminal) {
    text = counts ? `${done} of ${total} ${unitPlural} finished before it stopped` : 'The number of finished runs is not known';
    if (state === 'completed' && counts) text = `${done} of ${total} ${unitPlural} finished`;
  } else if (current) {
    const bits = [`${unit[0].toUpperCase()}${unit.slice(1)} ${current.index} of ${current.total} in progress`];
    const detail = [current.arm, current.rep !== null ? (current.repTotal !== null ? `repetition ${current.rep} of ${current.repTotal}` : `repetition ${current.rep}`) : null].filter(Boolean);
    text = detail.length ? `${bits[0]}: ${detail.join(', ')}` : bits[0];
  } else if (counts) {
    text = `${done} of ${total} ${unitPlural} finished`;
  } else {
    text = state === 'queued' ? 'Waiting for the runner to start' : 'Progress not reported yet';
  }

  // Planned minimum and "at least X left" (Scenario B only; a matrix cell costs vary, so no estimate exists)
  let planned;
  let atLeastLeft;
  if (isMatrix) {
    planned = { state: 'unavailable', reason: 'a matrix cell varies in cost, so the gateway gives no basis for an estimate' };
    atLeastLeft = planned;
  } else {
    planned = plannedMinimum(spec, total);
    if (terminal || state === 'queued') atLeastLeft = { state: 'unavailable', reason: terminal ? 'the run has ended' : 'the run has not started' };
    else if (planned.state !== 'value') atLeastLeft = { state: 'unavailable', reason: planned.reason };
    else if (!isNum(startedMs)) atLeastLeft = { state: 'unavailable', reason: 'the gateway has not reported a start time' };
    else if (!isNum(nowMs) || nowMs < startedMs) atLeastLeft = { state: 'unavailable', reason: 'the clocks disagree or the browser clock is not available' };
    else {
      const elapsed = (nowMs - startedMs) / 1000;
      if (elapsed > planned.seconds) atLeastLeft = { state: 'overdue', text: 'The run is taking longer than the planned minimum', basis: 'planned phase lengths only' };
      else atLeastLeft = { state: 'value', seconds: planned.seconds - elapsed, text: `At least ${formatElapsed((planned.seconds - elapsed) * 1000)} left by plan`, basis: 'Lower bound from the planned phase lengths; start-up, drain and shut-down add time this page cannot measure' };
    }
  }

  let sinceLastEvent = { state: 'unavailable', reason: 'no event has been received by this page' };
  if (!terminal && isNum(lastEventAtMs) && isNum(nowMs)) {
    sinceLastEvent = nowMs >= lastEventAtMs ? { state: 'value', ms: nowMs - lastEventAtMs, text: formatElapsed(nowMs - lastEventAtMs), basis: 'this browser clock (inferred)' } : { state: 'unavailable', reason: 'the browser clock went backwards' };
  } else if (terminal) sinceLastEvent = { state: 'unavailable', reason: 'the run has ended' };

  return {
    unit, unitPlural, unitDefinition: isMatrix ? 'One cell = one cell of the simulation matrix, as counted by the gateway.' : UNIT_DEFINITION,
    counts, current, phase, bar, text, terminal, planned, atLeastLeft, sinceLastEvent,
    eta: { state: 'unavailable', reason: 'the gateway reports no remaining-time estimate and this page does not invent one' },
  };
}
