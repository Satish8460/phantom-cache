import test from 'node:test';
import assert from 'node:assert/strict';
import {
  FMT, artifactsView, capabilitiesView, cell, effectChartData, evidenceDetailView, evidenceListView, m6DetailView, padDomain, runView,
  runsListView, scaleLinear, scenarioBView, specRows, t1ResultView, utcText,
} from '../src/lib/labView.js';
import { RUN_STATES, TIERS, isExploratoryKind, kindInfo, labelInfo, runStateInfo, tierInfo, verdictInfo } from '../src/lib/labProvenance.js';
import { parseSseText, parseEventData } from '../src/lib/sse.js';
import { FX } from './helpers/labFixtures.mjs';

const caps = () => capabilitiesView(FX.caps());

// ---------------------------------------------------------------- provenance vocabulary

test('tiers: every API tier maps to a provenance class and wording; unknown tiers degrade to Unavailable', () => {
  assert.equal(tierInfo('MEASURED_LOOPBACK').src, 'measured');
  assert.match(tierInfo('MEASURED_LOOPBACK').label, /loopback/i);
  assert.equal(tierInfo('SIMULATED').src, 'simulated');
  assert.equal(tierInfo('RETAINED_SIMULATED').src, 'simulated');
  // Lead decision (a), 2026-10-30: 'Retained' is a separate neutral tag, not part of the provenance word.
  assert.equal(tierInfo('RETAINED_SIMULATED').retained, true);
  assert.doesNotMatch(tierInfo('RETAINED_SIMULATED').label, /Retained/);
  assert.equal(tierInfo('MEASURED_LOOPBACK').retained, false);
  // MEASURED displays as REAL, always with its basis qualifier; INFERRED/MEASURED words never appear for tiers.
  for (const k of ['MEASURED_LOOPBACK', 'RETAINED_MEASURED_LOOPBACK']) {
    assert.equal(tierInfo(k).word, 'Real');
    assert.match(tierInfo(k).qualifier, /loopback/);
    assert.match(tierInfo(k).qualifier, /one host/);
    assert.doesNotMatch(tierInfo(k).label, /Measured/);
  }
  assert.equal(tierInfo('RETAINED_MEASURED_LOOPBACK').src, 'measured');
  assert.equal(tierInfo('DERIVED').src, 'derived');
  assert.equal(tierInfo('UNAVAILABLE').src, 'unavailable');
  const odd = tierInfo('SOMETHING_NEW');
  assert.equal(odd.src, 'unavailable');
  assert.equal(odd.unknownTier, 'SOMETHING_NEW');
  assert.equal(tierInfo(undefined).src, 'unavailable');
  for (const t of Object.values(TIERS)) assert.ok(t.meaning.length > 20);
});

test('run states: all seven states have a glyph, a word and a stated meaning; terminal ones are marked', () => {
  assert.deepEqual(Object.keys(RUN_STATES), ['queued', 'running', 'cancelling', 'completed', 'failed', 'cancelled', 'interrupted']);
  for (const [k, v] of Object.entries(RUN_STATES)) {
    assert.ok(v.label && v.icon && v.tone && v.meaning, k);
  }
  assert.deepEqual(Object.keys(RUN_STATES).filter((k) => RUN_STATES[k].terminal), ['completed', 'failed', 'cancelled', 'interrupted']);
  assert.equal(runStateInfo('weird').key, 'unknown');
  assert.equal(runStateInfo(undefined).label, 'State not reported');
});

test('verdicts: negative and inconclusive outcomes have a glyph and word just like positive ones', () => {
  for (const o of ['SUPPORTED', 'NOT_SUPPORTED', 'TRADE_OFF_FAILURE', 'INCONCLUSIVE', 'NOT_TESTABLE_AT_REGISTERED_HORIZON', 'UNVERIFIED', 'MECHANISM_FAILED', 'BENEFIT_NOT_SUPPORTED', 'BENEFIT_SUPPORTED']) {
    const v = verdictInfo(o);
    assert.ok(v.label && v.icon && v.tone && v.kind !== 'other', o);
  }
  assert.equal(verdictInfo('NOT_SUPPORTED').kind, 'negative');
  assert.equal(verdictInfo('TRADE_OFF_FAILURE').kind, 'negative');
  assert.equal(verdictInfo('UNVERIFIED').kind, 'inconclusive');
  assert.equal(verdictInfo('SOMETHING_ELSE').label, 'something else', 'unknown outcomes are shown as recorded, not hidden');
  assert.equal(verdictInfo(null).label, 'No verdict recorded');
  assert.equal(labelInfo('VALID').tone, 'info');
  assert.equal(labelInfo('EXPLORATORY').tone, 'warn');
  assert.equal(kindInfo('m6_sealed_evaluation').short, 'M6 sealed');
  assert.equal(isExploratoryKind('m6_dry_run'), true);
  assert.equal(isExploratoryKind('m6_sealed_evaluation'), false);
  assert.equal(isExploratoryKind('scenario_b'), false);
});

// ---------------------------------------------------------------- cells

test('cell: numbers format, null / missing / {unavailable} become words with a reason, never 0', () => {
  assert.equal(cell(0, FMT.count).text, '0', 'a real zero stays zero');
  assert.equal(cell(0, FMT.count).state, 'value');
  for (const raw of [null, undefined, NaN, 'abc', {}, { unavailable: true, reason: 'no samples' }]) {
    const c = cell(raw, FMT.count, 'not recorded');
    assert.equal(c.state, 'unavailable');
    assert.equal(c.text, 'Unavailable');
    assert.equal(c.num, null);
  }
  assert.equal(cell({ unavailable: true, reason: 'live file; not hashed' }, FMT.count).reason, 'live file; not hashed');
  assert.equal(cell(null, FMT.count, 'because').reason, 'because');
  assert.equal(FMT.ratio(0.95053), '95.1%');
  assert.equal(FMT.signedRatio(0.25), '+25.0%');
  assert.equal(FMT.signedRatio(-3), '-300.0%');
  assert.equal(FMT.p(0.00005), 'p < 0.0001');
  assert.equal(utcText({ unavailable: true, reason: 'not recorded in the manifest' }), 'not recorded (not recorded in the manifest)');
  assert.equal(utcText('2026-10-30T00:41:16Z'), '2026-10-30T00:41:16Z');
});

// ---------------------------------------------------------------- capabilities

test('capabilities: the recorded anonymous-mode response is read into availability, limits, catalogues and the debug label', () => {
  const c = caps();
  assert.equal(c.schema, 'lab-1');
  assert.equal(c.security.key, 'anonymous_dev_loopback_only');
  assert.equal(c.security.needsToken, false);
  assert.equal(c.t1.available, true);
  assert.equal(c.sb.available, true);
  assert.equal(c.debugBuild, true, 'the gateway under test is a debug build');
  assert.equal(c.sb.predictedLabel, 'EXPLORATORY');
  assert.ok(c.sb.labelReasons.some((r) => /DEBUG/i.test(r)));
  assert.ok(c.sb.labelReasons.every((r) => !/Personal|D:\\|[A-Z]:\\/.test(r)), 'server paths are redacted in the recorded fixture');
  assert.equal(c.maxConcurrent, 1);
  assert.equal(c.activeRun, null);
  assert.equal(c.t1.scenarios.length, 12);
  assert.ok(c.t1.arms.includes('BL-0') && c.t1.families.includes('W-Z'));
  assert.deepEqual(c.t1.seedSets.map((s) => s.name), ['T', 'D']);
  assert.ok(!c.t1.seedSets.some((s) => /^s(ealed)?$/i.test(s.name)), 'the sealed set is not offered by the API');
  assert.equal(c.sb.limits.repetitions.valid_label_needs_at_least, 10);
  assert.equal(c.limits.heartbeatSeconds, 15);
});

test('capabilities: JWT mode is recognised; a missing kind or an unknown mode degrades to words, not to a crash', () => {
  const j = capabilitiesView(FX.capsJwt());
  assert.equal(j.security.key, 'jwt');
  assert.equal(j.security.needsToken, true);
  const odd = capabilitiesView({ schema: 'lab-1', security_mode: 'mtls', kinds: {} });
  assert.equal(odd.security.key, 'unknown');
  assert.equal(odd.security.needsToken, null);
  assert.equal(odd.sb.present, false);
  assert.equal(odd.sb.available, false);
  assert.equal(capabilitiesView(null), null);
  const unavailable = FX.caps();
  unavailable.kinds.scenario_b.available = false;
  unavailable.kinds.scenario_b.reason = 'gateway executable not found';
  const u = capabilitiesView(unavailable);
  assert.equal(u.sb.available, false);
  assert.equal(u.sb.reason, 'gateway executable not found');
  const running = FX.caps();
  running.active_run = { run_id: '20261030T034041Z-c78a73cf', kind: 'scenario_b', state: 'running' };
  assert.equal(capabilitiesView(running).activeRun.runId, '20261030T034041Z-c78a73cf');
});

// ---------------------------------------------------------------- evidence list

test('evidence list: the recorded index splits into registered and exploratory, with negative verdicts as first-class chips', () => {
  const v = evidenceListView(FX.evidenceIndex());
  assert.equal(v.items.length, 3);
  assert.deepEqual(v.registered.map((i) => i.kind.key).sort(), ['m6_sealed_evaluation', 'scenario_b']);
  assert.deepEqual(v.exploratory.map((i) => i.kind.key), ['m6_dry_run']);
  const sealed = v.items.find((i) => i.kind.key === 'm6_sealed_evaluation');
  assert.equal(sealed.tier.key, 'RETAINED_SIMULATED');
  assert.equal(sealed.originalTier.key, 'SIMULATED');
  const h1 = sealed.verdictChips.find((c) => c.name === 'H1');
  assert.equal(h1.key, 'NOT_SUPPORTED');
  assert.equal(h1.kind, 'negative');
  assert.deepEqual(sealed.verdictChips.map((c) => c.name), ['H1', 'H2', 'H3', 'H4', 'H5 and H6']);
  assert.match(sealed.banner, /SIMULATION RESULT/);
  const sb = v.items.find((i) => i.kind.key === 'scenario_b');
  assert.equal(sb.tier.key, 'RETAINED_MEASURED_LOOPBACK');
  assert.deepEqual(sb.verdictChips.map((c) => [c.name, c.key]), [['AIMD-CAP', 'TRADE_OFF_FAILURE'], ['STATIC-CAP', 'TRADE_OFF_FAILURE']]);
  assert.equal(sb.label, 'VALID');
  assert.ok(sb.verdictChips[1].reasons[0].includes('4.947 pp below BASE'), 'reasons are carried verbatim');
  assert.equal(sb.href, '#/lab/evidence/scenario_b.20261030T003932Z');
  assert.ok(sealed.artifacts.some((a) => a.name === 'verdict.json'));
  assert.equal(v.total, 3);
});

test('evidence list: an empty index is an empty view; an item without a verdict says so with the reason', () => {
  assert.deepEqual(evidenceListView({ items: [], indexed: 0 }).items, []);
  assert.equal(evidenceListView({}), null);
  assert.equal(evidenceListView(null), null);
  const idx = FX.evidenceIndex();
  idx.items[1].verdict_snapshot = { unavailable: true, reason: 'this retained artifact set records no verdict' };
  const it = evidenceListView(idx).items[1];
  assert.equal(it.verdictChips.length, 0);
  assert.equal(it.verdictMissing, 'this retained artifact set records no verdict');
  idx.items[0].tier = 'FUTURE_TIER';
  assert.equal(evidenceListView(idx).items[0].tier.src, 'unavailable');
});

// ---------------------------------------------------------------- M6 detail

test('M6 detail: the sealed result keeps H1 NOT_SUPPORTED and the counts that justify it', () => {
  const d = evidenceDetailView(FX.m6Sealed());
  assert.equal(d.kind.key, 'm6_sealed_evaluation');
  assert.equal(d.sb, null);
  const m = d.m6;
  assert.equal(m.protocol, '2.2');
  assert.equal(m.scenarios.length, 8);
  const h1 = m.hypotheses.find((h) => h.id === 'H1');
  assert.equal(h1.verdict.key, 'NOT_SUPPORTED');
  assert.deepEqual(h1.lines.map((l) => [l.value.num, l.of]), [[0, 8], [2, 8]]);
  assert.equal(m.hypotheses.find((h) => h.id === 'H2').verdict.key, 'NOT_TESTABLE_AT_REGISTERED_HORIZON');
  assert.match(m.hypotheses.find((h) => h.id === 'H2').reason, /PIE-no-stale executes cell-for-cell identically/);
  assert.equal(m.hypotheses.find((h) => h.id === 'H3').verdict.key, 'UNVERIFIED');
  assert.deepEqual(m.falsifiers.map((f) => [f.id, f.fires]), [['F-a', 'fires'], ['F-b', 'not mechanical'], ['F-c', 'does not fire'], ['F-d', 'does not fire'], ['F-e', 'does not fire'], ['F-f', 'not mechanical']]);
  assert.equal(m.cells.num, 12000);
  assert.equal(m.expectedCells.num, 12000);
  assert.equal(d.sealedNote, 'seed values of the sealed set are never read, derived or exposed by this API');
  assert.equal(d.provenance.seedSet, 'S');
  assert.equal(d.provenance.seedListSha256.length, 64);
  const flag = h1.flags.find((f) => f.key === 'not_supported_triggered');
  assert.equal(flag.value, true);
  assert.ok(!JSON.stringify(d).includes('seed_values":['), 'no seed values anywhere in the view');
});

test('M6 detail: effect-size chart data keeps negative means, includes the zero line, and marks adverse rows', () => {
  const m = evidenceDetailView(FX.m6Sealed()).m6;
  const peak = m.peakChart;
  assert.equal(peak.rows.length, 8);
  assert.ok(peak.domain.min < -3 && peak.domain.max > 0, 'axis spans the data and includes zero');
  const deg = peak.rows.find((r) => r.scenario === 'O-DEG');
  assert.equal(deg.mean, -3);
  assert.equal(deg.adverse, true);
  assert.equal(deg.supported, false);
  const het = m.p99Chart.rows.find((r) => r.scenario === 'O-HET');
  assert.equal(het.supported, true);
  assert.ok(het.lo > 0.27 && het.hi < 0.31, 'interval as recorded');
  const sx = scaleLinear(peak.domain, [0, 100]);
  assert.ok(sx(0) > 0 && sx(0) < 100, 'zero sits inside the plot');
  assert.ok(sx(deg.mean) < sx(0));
  // rows with a missing interval are listed but not plotted
  const bad = effectChartData([{ scenario: 'X', peak: { mean: cell(null, FMT.smart), lo: cell(null, FMT.smart), hi: cell(null, FMT.smart), adverse: false, supported: false, practicallyNull: false } }], 'peak');
  assert.equal(bad.rows[0].plotted, false);
  assert.equal(bad.domain, null);
  assert.deepEqual(padDomain(1, 1), { min: 0, max: 2 });
});

test('M6 detail: the dry run is the same shape but is flagged exploratory', () => {
  const d = evidenceDetailView(FX.m6Dry());
  assert.equal(d.exploratory, true);
  assert.equal(d.kind.key, 'm6_dry_run');
  assert.ok(d.m6.scenarios.length > 0);
  assert.equal(d.sealedNote, null, 'a dry run carries no sealed-set statement');
});

// ---------------------------------------------------------------- Scenario B detail

test('Scenario B detail: the retained experiment carries the binding notice and limitations from the payload', () => {
  const d = evidenceDetailView(FX.sbEvidence());
  assert.equal(d.interpretation.binding, true);
  assert.equal(d.interpretation.mustAccompany, true);
  assert.match(d.interpretation.noticeText, /dated interpretation and correction notice/);
  assert.match(d.interpretation.noticeText, /Permitted conclusion/);
  assert.match(d.interpretation.noticeText, /Not established/);
  assert.ok(d.interpretation.limitations.length >= 8);
  assert.match(d.interpretation.labelMeaning, /Label VALID means only/);
  assert.equal(d.provenance.profile.gateway, 'release');
  assert.equal(d.provenance.reps, 10);
  assert.equal(d.m6, null);
  assert.equal(d.resultMissing, false);
});

test('Scenario B detail: arms, paired differences and verdicts come straight from the payload', () => {
  const sb = evidenceDetailView(FX.sbEvidence()).sb;
  assert.deepEqual(sb.arms.map((a) => a.name), ['BASE', 'STATIC-CAP', 'AIMD-CAP']);
  const by = Object.fromEntries(sb.arms.map((a) => [a.name, a]));
  assert.equal(by.BASE.peakWindow.num, 39);
  assert.equal(by['STATIC-CAP'].peakWindow.num, 24);
  assert.equal(by['STATIC-CAP'].success.text, '95.1%');
  assert.equal(by.BASE.success.text, '100.0%');
  assert.equal(by['AIMD-CAP'].success.text, '82.9%');
  assert.equal(by.BASE.capMinSeen.state, 'unavailable', 'BASE has no cap: the cell is unavailable, not zero');
  assert.match(by.BASE.capMinSeen.reason, /BASE runs without a cap/);
  assert.equal(sb.label, 'VALID');
  assert.deepEqual(sb.verdicts.map((v) => [v.arm, v.outcome.key]).sort(), [['AIMD-CAP', 'TRADE_OFF_FAILURE'], ['STATIC-CAP', 'TRADE_OFF_FAILURE']]);
  const st = sb.paired.find((p) => p.arm === 'STATIC-CAP');
  const p99 = st.rows.find((r) => r.key === 'client_p99_ms');
  assert.equal(p99.present, true);
  assert.ok(p99.ciLo.num < 0 && p99.ciHi.num > 0, 'the static-cap P99 interval spans zero: no evidence of an effect');
  const shed = st.rows.find((r) => r.key === 'shed_fraction_pp');
  assert.equal(shed.rel.state, 'unavailable', 'relative change of a zero baseline is null in the payload and stays unavailable');
  assert.equal(shed.diff.state, 'value');
  assert.ok(sb.mechanismAllPass);
  assert.equal(sb.mechanism.find((m) => m.arm === 'STATIC-CAP').observed.num, 24);
  assert.ok(sb.limitations.length >= 8);
});

test('Scenario B charts: trade-off pairs peak with success; per-phase shed and success-only latency are unavailable with a reason', () => {
  const sb = evidenceDetailView(FX.sbEvidence()).sb;
  const c = sb.charts;
  assert.deepEqual(c.tradeoff.map((t) => t.arm), ['BASE', 'STATIC-CAP', 'AIMD-CAP']);
  assert.equal(c.shedPerPhase.available, false);
  assert.match(c.shedPerPhase.reason, /not part of the evidence or summary payload/);
  assert.equal(c.latencySuccessOnly.available, false);
  assert.match(c.latencySuccessOnly.reason, /not exposed by the API/);
  assert.equal(c.latency.length, 3);
  assert.equal(c.latency[0].all.p99.state, 'value');
});

test('Scenario B partial summary (cancelled run): nulls are unavailable cells, never zeros; verdicts are absent', () => {
  const s = FX.sbSummary();
  for (const arm of Object.values(s.arms)) {
    for (const k of Object.keys(arm.per_run_mean)) arm.per_run_mean[k] = null;
    for (const k of Object.keys(arm.client_latency_ms_pooled_scheduled_time)) arm.client_latency_ms_pooled_scheduled_time[k] = null;
  }
  s.verdicts = {};
  s.paired_vs_base = {};
  s.status = 'CANCELLED';
  const v = scenarioBView(s);
  assert.equal(v.status, 'CANCELLED');
  assert.equal(v.verdicts.length, 0);
  assert.equal(v.paired.length, 0);
  for (const a of v.arms) {
    assert.equal(a.success.state, 'unavailable', a.name);
    assert.equal(a.peakWindow.text, 'Unavailable');
    assert.match(a.peakWindow.reason, /ended before this value was produced/);
    assert.equal(a.latencyAll.p99.state, 'unavailable');
  }
  assert.equal(scenarioBView(null), null);
  assert.equal(scenarioBView({}).arms.length, 0, 'an empty object yields an empty, safe view');
});

test('Scenario B run summary (debug, 1 repetition): EXPLORATORY with every reason, INCONCLUSIVE rather than a victory', () => {
  const v = scenarioBView(FX.sbSummary());
  assert.equal(v.label, 'EXPLORATORY');
  assert.ok(v.labelReasons.length >= 3);
  assert.ok(v.labelReasons.some((r) => /DEBUG/.test(r)));
  assert.ok(v.labelReasons.some((r) => /deviates_from_retained_experiment_parameters/.test(r)));
  assert.equal(v.verdicts[0].outcome.key, 'INCONCLUSIVE');
  assert.equal(v.verdicts[0].outcome.kind, 'inconclusive');
  assert.match(v.verdicts[0].reasons[0], /only 1 valid paired runs; 10 required/);
  assert.ok(v.interpretationNotes.length > 0);
  assert.ok(v.labelReasons.every((r) => !/[A-Z]:\\/.test(r)), 'paths redacted');
});

// ---------------------------------------------------------------- runs

test('runs: the recorded running, completed, cancelled and listed runs are read into views with their states', () => {
  const running = runView(FX.runSbRunning());
  assert.equal(running.state, 'running');
  assert.equal(running.kind.key, 'scenario_b');
  assert.equal(running.tier.key, 'MEASURED_LOOPBACK');
  assert.equal(running.progress.total, 2);
  assert.equal(running.ended, 'not ended');
  assert.equal(running.exitCode, null);
  assert.ok(running.interpretation.limitations.length >= 8);
  const done = runView(FX.runSbFinal());
  assert.equal(done.state, 'completed');
  assert.equal(done.exitCode, 0);
  assert.equal(done.hasSummaryArtifact, true);
  assert.equal(done.hasCells, false);
  const cancelled = runView(FX.runCancelled());
  assert.equal(cancelled.state, 'cancelled');
  assert.equal(cancelled.exitCode, 130);
  const t1 = runView(FX.runT1());
  assert.equal(t1.hasCells, true);
  assert.equal(t1.tier.key, 'SIMULATED');
  assert.ok(t1.spec.some((r) => r.key === 'cells' && r.text === '4'));
  const list = runsListView(FX.runsList());
  assert.equal(list.items.length, 4);
  assert.deepEqual(list.items.map((i) => i.state), ['cancelled', 'completed', 'completed', 'completed']);
  assert.equal(list.items[0].href, `#/lab/runs/${list.items[0].id}`);
  assert.deepEqual(runsListView(FX.runsEmpty()).items, []);
  assert.equal(runView({}), null);
  assert.equal(runView({ run_id: 'x', state: 'interrupted', error: { code: 'interrupted', message: 'the gateway stopped' } }).error.code, 'interrupted');
});

test('run spec rows list every echoed field with a label, arrays joined, booleans as words', () => {
  const rows = specRows(FX.runSbFinal().spec);
  const get = (k) => rows.find((r) => r.key === k).text;
  assert.equal(get('arms'), 'BASE, STATIC-CAP');
  assert.equal(get('repetitions'), '1');
  assert.equal(get('static_cap'), '24', 'defaults the gateway filled in are visible');
  assert.equal(specRows(FX.runT1().spec).find((r) => r.key === 'record_raw').text, 'no');
});

test('artifacts: sizes and hashes as recorded; live files show an unavailable hash with the reason, not an empty cell', () => {
  const rows = artifactsView(FX.artifactsSb());
  assert.deepEqual(rows.map((r) => r.name), ['run.json', 'events.jsonl', 'summary.json', 'manifest.json']);
  const sum = rows.find((r) => r.name === 'summary.json');
  assert.equal(sum.sha256.state, 'value');
  assert.equal(sum.sha256.text.length, 64);
  assert.equal(sum.sha256Of, 'served bytes (server paths redacted)');
  assert.equal(sum.tier.key, 'MEASURED_LOOPBACK');
  const live = rows.find((r) => r.name === 'run.json');
  assert.equal(live.sha256.state, 'unavailable');
  assert.equal(live.sha256.reason, 'live file; not hashed');
  assert.equal(live.tier, null);
  assert.equal(sum.downloadable, true);
  assert.equal(artifactsView({}), null);
});

test('T1 result: the result event carries only a cell count and the hash of cells.csv', () => {
  const ev = parseSseText(FX.sseT1()).find((e) => e.event === 'result');
  const r = t1ResultView(parseEventData(ev).value);
  assert.equal(r.cells.num, 4);
  assert.equal(r.cellsSha256, '0ea1926af10bb76e897ac2f666610829fbc03dbbccb2e7cb3f6edf13ca343811');
  assert.equal(r.tier.key, 'SIMULATED');
  assert.match(r.note, /not a measurement/);
  assert.equal(t1ResultView(null), null);
});

// ---------------------------------------------------------------- redaction / hostile payloads

test('hostile payload text is clipped and stays text: no field is trusted to be short or a string', () => {
  const evil = '<img src=x onerror=alert(1)>'.repeat(200);
  const idx = FX.evidenceIndex();
  idx.items[0].id = 'scenario_b.x';
  idx.items[0].verdict_snapshot.label = evil;
  idx.items[0].verdict_snapshot.label_reasons = [evil, 5, null];
  const v = evidenceListView(idx).items[0];
  assert.ok(v.label.length <= 20);
  assert.ok(v.labelReasons.every((r) => r.length <= 400));
  const run = FX.runCancelled();
  run.error = { code: evil, message: evil };
  const rv = runView(run);
  assert.ok(rv.error.code.length <= 40 && rv.error.message.length <= 400);
  const caps2 = FX.caps();
  caps2.kinds.scenario_b.reason = evil;
  assert.ok(capabilitiesView(caps2).sb.reason.length <= 200);
});

test('the views never contain server filesystem paths or seed values for the recorded fixtures', () => {
  const all = [evidenceDetailView(FX.sbEvidence()), evidenceDetailView(FX.m6Sealed()), capabilitiesView(FX.caps()), runView(FX.runSbFinal()), scenarioBView(FX.sbSummary())];
  for (const v of all) {
    const text = JSON.stringify(v);
    assert.doesNotMatch(text, /[A-Z]:\\\\|\/home\/|Personal|g3_lab/, 'no server path');
    assert.doesNotMatch(text, /"seed_values":\[|"seeds":\[/);
  }
});

test('RUN_STATES wording does not promise more than the contract says', () => {
  assert.match(RUN_STATES.cancelling.meaning, /cooperatively/);
  assert.match(RUN_STATES.interrupted.meaning, /not resumed/);
  assert.match(RUN_STATES.failed.meaning, /never reports a crash as completed/);
});
