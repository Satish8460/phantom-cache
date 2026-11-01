// Honest progress (progress.js): counts from the gateway only, no fake ETA, no empty 0-of-N bar.
import test from 'node:test';
import assert from 'node:assert/strict';
import { PHASE_STEPS, plannedMinimum, progressView } from '../src/lib/progress.js';
import { applyRunEvent, initialRunModel } from '../src/lib/labStream.js';
import { FUNC } from './helpers/funcFixtures.mjs';

const SPEC = { repetitions: 1, arms: ['BASE', 'STATIC-CAP'], warmup_s: 2, healthy_s: 3, degraded_s: 4, recovery_s: 1 };
const base = { kind: 'scenario_b', state: 'running', spec: SPEC, startedMs: 1_000_000, nowMs: 1_004_000 };

test('first progress event (0 of 2): no bar, "Run 1 of 2 in progress", phase step, no empty bar', () => {
  const v = progressView({ ...base, progress: { done: 0, total: 2, arm: 'BASE', rep: 1, phase: 'warmup' } });
  assert.equal(v.bar.render, false);
  assert.equal(v.text, 'Run 1 of 2 in progress: BASE, repetition 1 of 1');
  assert.deepEqual(v.phase, { name: 'warmup', label: 'Warm-up', step: 1, steps: 4 });
  assert.doesNotMatch(v.text, /0 of 2/);
  assert.doesNotMatch(v.text, /units/);
  assert.match(v.unitDefinition, /one fresh gateway process/);
});

test('mid-run (1 of 2 done): bar renders on gateway counts only', () => {
  const v = progressView({ ...base, progress: { done: 1, total: 2, arm: 'STATIC-CAP', rep: 1, phase: 'degraded' } });
  assert.deepEqual(v.bar, { render: true, value: 1, max: 2 });
  assert.equal(v.phase.step, 3);
  assert.equal(v.text, 'Run 2 of 2 in progress: STATIC-CAP, repetition 1 of 1');
});

test('phases outside the four steps are named, not counted', () => {
  for (const [ph, label] of [['starting', 'Starting the gateway'], ['drain', 'Collecting results'], ['collect', 'Collecting results'], ['run-complete', 'Run finished']]) {
    const v = progressView({ ...base, progress: { done: 0, total: 2, phase: ph } });
    assert.equal(v.phase.label, label);
    assert.equal(v.phase.step, null);
  }
  assert.deepEqual([...PHASE_STEPS], ['warmup', 'healthy', 'degraded', 'recovery']);
  assert.equal(progressView({ ...base, progress: { done: 0, total: 2, phase: 'odd' } }).phase.label, 'odd');
});

test('planned minimum and "at least X left by plan" are lower bounds, only with all inputs', () => {
  const v = progressView({ ...base, progress: { done: 0, total: 2, phase: 'healthy' } });
  assert.equal(v.planned.seconds, 20, '2 runs x (2+3+4+1) s');
  assert.equal(v.atLeastLeft.state, 'value');
  assert.equal(v.atLeastLeft.seconds, 16);
  assert.match(v.atLeastLeft.text, /^At least 16 s left by plan$/);
  assert.match(v.atLeastLeft.basis, /add time this page cannot measure/);
  assert.equal(v.eta.state, 'unavailable');
  const late = progressView({ ...base, nowMs: 1_000_000 + 45_000, progress: { done: 1, total: 2, phase: 'collect' } });
  assert.equal(late.atLeastLeft.state, 'overdue');
  assert.match(late.atLeastLeft.text, /longer than the planned minimum/);
});

test('missing inputs give unavailable WITH a reason, never a number', () => {
  const p = { done: 0, total: 2, phase: 'warmup' };
  const cases = [
    [{ spec: null }, /settings are not known/],
    [{ spec: { ...SPEC, healthy_s: undefined } }, /all four phase lengths/],
    [{ startedMs: null }, /start time/],
    [{ nowMs: 999_000 }, /clocks disagree/],
    [{ nowMs: null }, /clocks disagree|not available/],
  ];
  for (const [over, re] of cases) {
    const v = progressView({ ...base, ...over, progress: p });
    assert.equal(v.atLeastLeft.state, 'unavailable', JSON.stringify(over));
    assert.match(v.atLeastLeft.reason, re);
    assert.equal('seconds' in v.atLeastLeft, false);
  }
  assert.equal(progressView({ ...base, progress: { phase: 'warmup' } }).planned.state, 'unavailable');
  assert.equal(progressView({ ...base, progress: null }).counts, null);
  assert.equal(progressView({ ...base, progress: null }).text, 'Progress not reported yet');
  assert.equal(progressView({ ...base, state: 'queued', progress: null }).text, 'Waiting for the runner to start');
  assert.equal(progressView({}).eta.state, 'unavailable');
});

test('negative or non-finite numbers are ignored', () => {
  const v = progressView({ ...base, progress: { done: Number.NaN, total: -3, phase: 'warmup' } });
  assert.equal(v.counts, null);
  assert.equal(v.bar.render, false);
  assert.equal(plannedMinimum(SPEC, 0).state, 'unavailable');
});

test('terminal runs show frozen text, no bar, no estimate', () => {
  for (const state of ['cancelled', 'failed', 'interrupted']) {
    const v = progressView({ ...base, state, progress: { done: 1, total: 2 } });
    assert.equal(v.text, '1 of 2 runs finished before it stopped');
    assert.equal(v.bar.render, false);
    assert.equal(v.atLeastLeft.state, 'unavailable');
    assert.equal(v.current, null);
  }
  assert.equal(progressView({ ...base, state: 'completed', progress: { done: 2, total: 2 } }).text, '2 of 2 runs finished');
  assert.equal(progressView({ ...base, state: 'cancelled', progress: null }).text, 'The number of finished runs is not known');
});

test('T1 matrix counts cells and never estimates', () => {
  const v = progressView({ kind: 't1_matrix', state: 'running', progress: { done: 3, total: 12 }, spec: null });
  assert.equal(v.unit, 'cell');
  assert.equal(v.text, '3 of 12 cells finished');
  assert.deepEqual(v.bar, { render: true, value: 3, max: 12 });
  assert.equal(v.planned.state, 'unavailable');
  assert.equal(v.atLeastLeft.state, 'unavailable');
  assert.equal(v.current, null);
});

test('time since last event is a labelled inference from the browser clock', () => {
  const v = progressView({ ...base, lastEventAtMs: 1_001_000, progress: { done: 0, total: 2 } });
  assert.equal(v.sinceLastEvent.ms, 3000);
  assert.match(v.sinceLastEvent.basis, /inferred/);
  assert.equal(progressView({ ...base, lastEventAtMs: 1_009_000, progress: null }).sinceLastEvent.state, 'unavailable');
});

test('REAL event log replayed through applyRunEvent: done stays 0 for the whole cancelled run (real behaviour); phases advance; never an empty bar', () => {
  const { events, run } = FUNC.legacyCancelled();
  let m = initialRunModel;
  const seen = [];
  for (const line of events.trim().split('\n')) {
    const e = JSON.parse(line);
    m = applyRunEvent(m, { id: e.id, event: e.event, data: e.data, malformed: false });
    const v = progressView({ kind: 'scenario_b', state: m.state, progress: m.progress, spec: run.spec, startedMs: Date.parse(run.started_utc), nowMs: Date.parse(e.ts) });
    seen.push(v);
    assert.equal(v.bar.render, false, `event ${e.id}`);
  }
  const phases = seen.filter((v) => v.phase).map((v) => v.phase.name);
  assert.deepEqual([...new Set(phases)], ['starting', 'warmup', 'healthy', 'degraded', 'collect']);
  const last = seen[seen.length - 1];
  assert.equal(last.terminal, true);
  assert.equal(last.text, '0 of 2 runs finished before it stopped');
  const mid = seen.find((v) => v.phase && v.phase.name === 'healthy');
  assert.equal(mid.text, 'Run 1 of 2 in progress: BASE, repetition 1 of 1');
  assert.equal(mid.atLeastLeft.state, 'value');
});
