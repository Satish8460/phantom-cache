// F0 + F1: real-backend fixtures and the result presentation rule (U-01).
// Fixtures: ui/test/fixtures/func (PROVENANCE.json). LEGACY = genuine unpatched-backend output; SANDBOX = real summarize()
// output of the not-yet-integrated U-01 patch. No fixture is hand-edited; synthetic variants below are built in code and
// their test names say "synthetic".
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { getHarness } from '../scripts/render-harness.mjs';
import { FUNC, readFuncJson, sha256Of } from './helpers/funcFixtures.mjs';
import { initialRunModel, initialStream, streamReducer } from '../src/lib/labStream.js';
import { runView, scenarioBView } from '../src/lib/labView.js';
import {
  KNOWN_RUN_STATES, RAW_DIAGNOSTICS_LABEL, applyPresentation, resultPresentation, summaryShape,
} from '../src/lib/resultPresentation.js';

let H;
let h;
before(async () => {
  H = await getHarness();
  h = H.React.createElement;
});

// ---------------------------------------------------------------- F0: provenance

test('F0 fixtures: every file recomputes to the sha256 recorded in PROVENANCE.json (no silent edits)', () => {
  const prov = readFuncJson('PROVENANCE.json');
  const files = { ...prov.legacy.files, ...prov.sandbox_u01.files };
  assert.ok(Object.keys(files).length >= 9);
  for (const [rel, meta] of Object.entries(files)) assert.equal(sha256Of(rel), meta.sha256, rel);
  assert.match(prov.sandbox_u01.label, /SANDBOX-PATCHED BACKEND OUTPUT, not integrated/);
});

test('F0 fixtures are what the provenance says: legacy cancelled carries the U-01 defect data, legacy completed does not', () => {
  const c = FUNC.legacyCancelled();
  assert.equal(c.run.state, 'cancelled');
  assert.equal(c.summary.status, 'CANCELLED');
  assert.equal(c.summary.verdicts['STATIC-CAP'].outcome, 'MECHANISM_FAILED');
  assert.equal(c.summary.mechanism_h_b1['STATIC-CAP'].runs, 0);
  assert.equal(c.summary.mechanism_h_b1_all_pass, false);
  assert.equal('verdicts_status' in c.summary, false, 'legacy shape');
  const k = FUNC.legacyCompleted();
  assert.equal(k.run.state, 'completed');
  assert.equal(k.summary.status, 'COMPLETE');
  for (const s of [FUNC.sandboxCancelledZero(), FUNC.sandboxCancelledPartial()]) {
    assert.equal(s.verdicts, null);
    assert.equal(s.verdicts_status, 'not_computed');
    assert.equal(s.run_completeness.complete, false);
    assert.equal(s.label, 'EXPLORATORY');
    assert.equal(s.mechanism_h_b1_all_pass, null);
  }
  assert.equal(FUNC.sandboxCompleted().verdicts_status, 'computed');
  assert.equal(summaryShape(c.summary), 'legacy');
  assert.equal(summaryShape(FUNC.sandboxCancelledZero()), 'new');
  assert.equal(summaryShape(null), null);
});

// ---------------------------------------------------------------- F0(a): U-01 reproduced in the CURRENT UI code path

function renderDetail({ run, summary, state = 'ready' }) {
  const stream = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  return H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView(run),
    live: { status: 'live', stream, model: initialRunModel },
    artifacts: { state: 'loading', rows: [] },
    summary: { state, view: summary ? scenarioBView(summary) : null },
    cancel: { phase: 'idle', error: null },
    onCancelStart() {}, onCancelConfirm() {}, onCancelAbort() {}, onReconnect() {}, onRefresh() {}, onDownload() {},
    downloading: null, downloadError: null,
  }));
}

// REPRODUCTION OF THE DEFECT (agent FLIB, 2026-10-30). While the screens are not yet wired to resultPresentation this test
// passes and documents U-01 as seen in the rendered DOM. When the screen agent wires the rule, flip SCREENS_WIRED to true:
// the same assertions then require the verdict to be absent.
const SCREENS_WIRED = true;

test('U-01 REPRODUCED in the current UI: the legacy CANCELLED fixture renders a verdict and "Mechanism failed" while the rule says none', () => {
  const { run, summary } = FUNC.legacyCancelled();
  const html = renderDetail({ run, summary });
  const rule = resultPresentation(run.state, summary);
  assert.equal(rule.showVerdicts, false, 'the rule forbids a verdict for a cancelled run');
  const shownVerdict = /data-verdict-arm="STATIC-CAP"/.test(html);
  const shownMechanism = /Mechanism failed/.test(html) && /falsified/.test(html);
  if (SCREENS_WIRED) {
    assert.equal(shownVerdict, false);
    assert.equal(shownMechanism, false);
  } else {
    assert.equal(shownVerdict, true, 'defect present: verdict chip rendered for a cancelled run');
    assert.equal(shownMechanism, true, 'defect present: "Mechanism failed ... falsified" rendered with zero runs recorded');
    assert.match(html, /no verdict can be drawn/, 'the page contradicts itself: the notice says no verdict, the chip shows one');
  }
});

test('the legacy COMPLETED fixture renders its verdict (the rule must not suppress completed runs)', () => {
  const { run, summary } = FUNC.legacyCompleted();
  const html = renderDetail({ run, summary });
  assert.match(html, /data-verdict-arm="STATIC-CAP"/);
  assert.equal(resultPresentation(run.state, summary).showVerdicts, true);
});

// ---------------------------------------------------------------- F1: the rule

const STATES = ['queued', 'running', 'cancelling', 'completed', 'cancelled', 'failed', 'interrupted'];

test('F1: KNOWN_RUN_STATES is exactly the seven states', () => {
  assert.deepEqual([...KNOWN_RUN_STATES].sort(), [...STATES].sort());
});

const SHAPES = {
  none: () => null,
  undefinedSummary: () => undefined,
  notObject: () => 'oops',
  array: () => [],
  empty: () => ({}),
  legacyCancelled: () => FUNC.legacyCancelled().summary,
  legacyCompleted: () => FUNC.legacyCompleted().summary,
  newCancelledZero: () => FUNC.sandboxCancelledZero(),
  newCancelledPartial: () => FUNC.sandboxCancelledPartial(),
  newCompleted: () => FUNC.sandboxCompleted(),
};

// Expected outcome: [kind, showVerdicts] per state x shape. Only a completed run with a COMPLETE, computed summary may show verdicts.
function expected(state, shapeName) {
  if (['queued', 'running', 'cancelling'].includes(state)) return ['unavailable', false];
  if (['cancelled', 'failed', 'interrupted'].includes(state)) return ['incomplete', false];
  // completed
  switch (shapeName) {
    case 'none': case 'undefinedSummary': case 'notObject': case 'array': return ['unavailable', false];
    case 'empty': return ['unavailable', false]; // status missing: cannot be shown as a verdict
    case 'legacyCancelled': case 'newCancelledZero': case 'newCancelledPartial': return ['incomplete', false];
    case 'legacyCompleted': case 'newCompleted': return ['verdicts', true];
    default: throw new Error(shapeName);
  }
}

for (const state of STATES) {
  for (const shapeName of Object.keys(SHAPES)) {
    test(`F1 table: state ${state} x summary ${shapeName}`, () => {
      const p = resultPresentation(state, SHAPES[shapeName]());
      const [kind, show] = expected(state, shapeName);
      assert.equal(p.kind, kind, JSON.stringify({ reason: p.reason }));
      assert.equal(p.showVerdicts, show);
      assert.equal(p.showMechanismVerdict, show);
      assert.ok(['verdicts', 'incomplete', 'unavailable'].includes(p.kind));
      assert.equal(typeof p.reason, 'string');
      assert.ok(p.reason.length > 0);
      assert.equal(typeof p.showLabel, 'boolean');
      // anything that is not a verdict must say so in words when it is a no-verdict mode
      if (p.mode === 'no_verdict') assert.match(p.headline, /^No verdict/);
      if (!show) assert.equal(p.rawDiagnostics.label, RAW_DIAGNOSTICS_LABEL);
    });
  }
}

test('F1: unknown / missing run state never shows a verdict, whatever the summary says', () => {
  for (const s of [null, undefined, '', 'weird', 'COMPLETED', 7, {}]) {
    const p = resultPresentation(s, FUNC.legacyCompleted().summary);
    assert.equal(p.showVerdicts, false, String(s));
    assert.equal(p.reason, 'state_unknown');
    assert.match(p.headline, /state is not known/);
  }
});

test('F1: legacy cancelled (REAL fixture): no verdict, VALID/MECHANISM_FAILED distrusted, headline and counts, raw diagnostics available', () => {
  const { run, summary } = FUNC.legacyCancelled();
  const p = resultPresentation(run.state, summary, { progress: run.progress });
  assert.equal(p.kind, 'incomplete');
  assert.equal(p.legacy, true);
  assert.equal(p.headline, 'No verdict: the run cancelled before finishing');
  assert.equal(p.detail, '0 of 2 planned runs had finished when it stopped. A verdict needs every planned repetition.');
  assert.deepEqual({ ...p.counts }, { done: 0, total: 2, source: 'run_progress' });
  assert.equal(p.fetchSummary, true);
  assert.equal(p.rawDiagnostics.available, true);
  assert.equal(p.labelTrusted, false);
  // the fixture's label is EXPLORATORY (debug build); a conservative class may be shown, never a trusted one
  assert.equal(p.label, 'EXPLORATORY');
  assert.equal(p.tone, 'neutral');
});

test('F1: a legacy summary with label VALID on a cancelled run is not shown as VALID (synthetic variant of the real fixture)', () => {
  const s = { ...FUNC.legacyCancelled().summary, label: 'VALID' };
  const p = resultPresentation('cancelled', s);
  assert.equal(p.label, null);
  assert.equal(p.showLabel, false);
  assert.equal(p.plannedLabel, 'VALID');
  assert.equal(p.labelTrusted, false);
});

test('F1: new-contract cancelled (SANDBOX output): counts come from run_completeness, label EXPLORATORY with planned VALID kept apart', () => {
  const s = FUNC.sandboxCancelledPartial();
  const p = resultPresentation('cancelled', s);
  assert.equal(p.kind, 'incomplete');
  assert.equal(p.legacy, false);
  assert.equal(p.detail, '2 of 20 planned runs had finished when it stopped. A verdict needs every planned repetition.');
  assert.equal(p.counts.source, 'summary');
  assert.equal(p.label, 'EXPLORATORY');
  assert.equal(p.showLabel, true);
  assert.equal(p.plannedLabel, 'VALID');
  assert.ok(p.labelReasons.some((r) => /incomplete/.test(r)));
  const z = resultPresentation('cancelled', FUNC.sandboxCancelledZero());
  assert.equal(z.detail, '0 of 30 planned runs had finished when it stopped. A verdict needs every planned repetition.');
});

test('F1: verdicts_status not_computed / run_completeness incomplete block verdicts even if the run state says completed (synthetic state)', () => {
  const p = resultPresentation('completed', FUNC.sandboxCancelledPartial());
  assert.equal(p.kind, 'incomplete');
  assert.equal(p.reason, 'verdicts_not_computed');
  assert.equal(p.showVerdicts, false);
  const onlyCompleteness = { ...FUNC.sandboxCompleted(), run_completeness: { complete: false, incomplete_reason: 'runs_missing', planned_runs: 4, completed_runs: 2 } };
  delete onlyCompleteness.verdicts_status;
  const q = resultPresentation('completed', onlyCompleteness);
  assert.equal(q.reason, 'run_completeness_incomplete');
  assert.equal(q.showVerdicts, false);
});

test('F1: a stale COMPLETE summary on a failed or interrupted run is not shown (D-13) and is not auto-fetched', () => {
  const stale = FUNC.legacyCompleted().summary;
  for (const state of ['failed', 'interrupted']) {
    const p = resultPresentation(state, stale);
    assert.equal(p.showVerdicts, false);
    assert.equal(p.fetchSummary, false, state);
    assert.equal(p.rawDiagnostics.available, false, 'not offered inline; the artifacts table still lists it');
    assert.equal(p.rawDiagnostics.listedOnlyInArtifacts, true);
    assert.match(p.headline, state === 'failed' ? /failed before finishing/ : /was interrupted before finishing/);
  }
  assert.equal(resultPresentation('failed', null).tone, 'critical');
  assert.equal(resultPresentation('interrupted', null).tone, 'warn');
  assert.equal(resultPresentation('cancelled', null).fetchSummary, true);
});

test('F1: waiting states fetch nothing and show the waiting line', () => {
  for (const s of ['queued', 'running', 'cancelling']) {
    const p = resultPresentation(s, null);
    assert.equal(p.mode, 'waiting');
    assert.equal(p.fetchSummary, false);
    assert.equal(p.showVerdicts, false);
  }
});

test('F1: completed + summary status other than COMPLETE is a contradiction: no verdict (synthetic)', () => {
  const p = resultPresentation('completed', { ...FUNC.legacyCompleted().summary, status: 'CANCELLED' });
  assert.equal(p.kind, 'incomplete');
  assert.equal(p.reason, 'summary_status_cancelled');
  const q = resultPresentation('completed', { ...FUNC.legacyCompleted().summary, status: 'WEIRD STATE!' });
  assert.equal(q.showVerdicts, false);
  assert.match(q.reason, /^summary_status_/);
});

test('F1: a COMPLETE summary with verdicts: null but no verdicts_status is not a verdict and is not the "needs a BASE arm" case (synthetic)', () => {
  const p = resultPresentation('completed', { ...FUNC.sandboxCompleted(), verdicts: null });
  assert.equal(p.showVerdicts, false);
  assert.equal(p.reason, 'verdicts_malformed');
});

test('F1: a BASE-only completed run (verdicts {}) is a verdict-mode result; the screen shows its own empty-verdict note (synthetic)', () => {
  const p = resultPresentation('completed', { ...FUNC.legacyCompleted().summary, verdicts: {} });
  assert.equal(p.kind, 'verdicts');
  assert.equal(p.showVerdicts, true);
});

test('F1: T1 matrix never has a verdict in any state', () => {
  for (const s of [...STATES, null, 'x']) {
    const p = resultPresentation(s, null, { runKind: 't1_matrix' });
    assert.equal(p.showVerdicts, false, String(s));
    assert.equal(p.kind, 'unavailable');
  }
  assert.match(resultPresentation('cancelled', null, { runKind: 't1_matrix' }).headline, /^No result: the run cancelled before finishing/);
});

test('F1: the rule does not read the SHAPE of verdicts: identical output whether a cancelled summary carries verdicts, {} or null', () => {
  const base = FUNC.sandboxCancelledPartial();
  const variants = [null, {}, FUNC.legacyCancelled().summary.verdicts, undefined].map((v) => {
    const s = { ...base };
    if (v === undefined) delete s.verdicts; else s.verdicts = v;
    return resultPresentation('cancelled', s);
  });
  for (const p of variants) {
    assert.equal(p.showVerdicts, false);
    assert.equal(p.headline, variants[0].headline);
  }
});

// ---------------------------------------------------------------- labView tolerance + applyPresentation (defence in depth)

test('scenarioBView tolerates the new shape: verdicts null -> no chips, mechanism pass null, completeness and planned label exposed', () => {
  const v = scenarioBView(FUNC.sandboxCancelledPartial());
  assert.deepEqual(v.verdicts, []);
  assert.equal(v.mechanismAllPass, null);
  assert.ok(v.mechanism.length > 0);
  assert.ok(v.mechanism.every((m) => m.pass === null));
  assert.equal(v.status, 'CANCELLED');
  assert.equal(v.verdictsStatus, 'not_computed');
  assert.equal(v.labelPlanned, 'VALID');
  assert.deepEqual(v.completeness, { complete: false, reason: 'cancelled', plannedRuns: 20, completedRuns: 2 });
  assert.ok(v.limitations.length > 0, 'limitations are kept');
  assert.ok(v.arms.length > 0, 'raw per-arm data is kept');
  const legacy = scenarioBView(FUNC.legacyCancelled().summary);
  assert.equal(legacy.verdictsStatus, null);
  assert.equal(legacy.completeness, null);
  const done = scenarioBView(FUNC.sandboxCompleted());
  assert.equal(done.verdictsStatus, 'computed');
  assert.equal(done.completeness.complete, true);
});

test('applyPresentation strips verdicts and mechanism pass/fail from a legacy cancelled view but keeps raw data and limitations', () => {
  const { run, summary } = FUNC.legacyCancelled();
  const view = scenarioBView(summary);
  assert.equal(view.verdicts.length, 1, 'precondition: the legacy view carries the misleading verdict');
  assert.equal(view.mechanismAllPass, false);
  const out = applyPresentation(view, resultPresentation(run.state, summary));
  assert.deepEqual(out.verdicts, []);
  assert.equal(out.mechanismAllPass, null);
  assert.ok(out.mechanism.every((m) => m.pass === null));
  assert.equal(out.withheld, true);
  assert.deepEqual(out.arms, view.arms);
  assert.deepEqual(out.limitations, view.limitations);
  // completed: identity
  const ok = FUNC.legacyCompleted();
  const v2 = scenarioBView(ok.summary);
  assert.equal(applyPresentation(v2, resultPresentation(ok.run.state, ok.summary)), v2);
  assert.equal(applyPresentation(null, null), null);
  // a VALID label on an incomplete run is dropped
  const validCancelled = scenarioBView({ ...summary, label: 'VALID' });
  assert.equal(applyPresentation(validCancelled, resultPresentation('cancelled', { ...summary, label: 'VALID' })).label, null);
});

test('F1 events: the real cancelled event log ends cancelled with exit 130 and never reports done as 2 (supports the N-of-M text)', () => {
  const { events } = FUNC.legacyCancelled();
  const lines = events.trim().split('\n').map((l) => JSON.parse(l));
  const last = lines[lines.length - 1];
  assert.equal(last.event, 'done');
  assert.equal(last.data.state, 'cancelled');
  assert.ok(lines.filter((l) => l.event === 'progress').every((l) => l.data.done === 0 && l.data.total === 2));
});
