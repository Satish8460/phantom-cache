import test from 'node:test';
import assert from 'node:assert/strict';
import { ALPHA_MAX, OFFERED_SEED_SETS, emptySbForm, emptyT1Form, labCommand, runSpecJson, validateSb, validateT1 } from '../src/lib/labForms.js';
import { capabilitiesView } from '../src/lib/labView.js';
import { FX } from './helpers/labFixtures.mjs';

const caps = () => capabilitiesView(FX.caps());

function goodT1(over = {}) {
  return { ...emptyT1Form(), scenarios: ['O-DEG'], families: ['W-Z'], arms: ['BL-0', 'BL-1'], nSeeds: '2', ...over };
}

test('T1 form: empty form is invalid with a message per required field; submit stays disabled', () => {
  const r = validateT1(emptyT1Form(), caps());
  assert.equal(r.valid, false);
  assert.deepEqual(Object.keys(r.errors).sort(), ['arms', 'families', 'nSeeds', 'scenarios']);
  assert.equal(r.spec, null);
  assert.equal(r.cells, null);
});

test('T1 form: a valid selection yields the exact spec the API recorded and the cell count the gateway echoed', () => {
  const r = validateT1(goodT1(), caps());
  assert.equal(r.valid, true);
  assert.deepEqual(r.spec, { scenarios: ['O-DEG'], families: ['W-Z'], arms: ['BL-0', 'BL-1'], seed_set: 'D', n_seeds: 2, profile: 'smoke', arm_params: 'untuned', record_raw: false });
  // the recorded run with this selection was echoed by the gateway as 4 cells
  assert.equal(r.cells, FX.runT1().spec.cells);
});

test('T1 form: limits come from /capabilities (seeds <= 30 and <= the set size, <= 20,000 cells)', () => {
  assert.equal(validateT1(goodT1({ nSeeds: '30' }), caps()).valid, true);
  assert.match(validateT1(goodT1({ nSeeds: '31' }), caps()).errors.nSeeds, /between 1 and 30/);
  assert.match(validateT1(goodT1({ nSeeds: '0' }), caps()).errors.nSeeds, /between 1 and 30/);
  assert.match(validateT1(goodT1({ nSeeds: '2.5' }), caps()).errors.nSeeds, /whole number/);
  assert.match(validateT1(goodT1({ nSeeds: '-1' }), caps()).errors.nSeeds, /whole number/);
  const c = caps();
  c.t1.seedSets = [{ name: 'T', size: 5 }, { name: 'D', size: 5 }];
  assert.match(validateT1(goodT1({ nSeeds: '6' }), c).errors.nSeeds, /between 1 and 5/, 'the set size also bounds the count');
  const all = caps();
  const big = validateT1(goodT1({ scenarios: all.t1.scenarios, families: all.t1.families, arms: all.t1.arms, nSeeds: '30' }), all);
  assert.equal(big.valid, false);
  assert.match(big.errors.nSeeds, /would run \d+ cells; the gateway allows at most \d+/);
  const lowered = caps();
  lowered.t1.limits = { max_cells: 3, max_seeds: 30 };
  assert.match(validateT1(goodT1(), lowered).errors.nSeeds, /at most 3/, 'a different gateway limit is honoured');
});

test('T1 form: the sealed seed set is never offered and is rejected in any spelling', () => {
  assert.deepEqual([...OFFERED_SEED_SETS], ['T', 'D']);
  for (const s of ['S', 's', 'sealed', ' Sealed ', 'X', '']) {
    const r = validateT1(goodT1({ seedSet: s }), caps());
    assert.equal(r.valid, false, JSON.stringify(s));
    assert.ok(r.errors.seedSet);
    assert.equal(r.spec, null);
  }
  assert.match(validateT1(goodT1({ seedSet: 'S' }), caps()).errors.seedSet, /sealed seed set is not available/);
  const ok = validateT1(goodT1({ seedSet: 'T' }), caps());
  assert.equal(ok.valid, true);
  assert.equal(ok.spec.seed_set, 'T');
});

test('T1 form: ids must come from the gateway catalogue; names follow the documented charset', () => {
  assert.match(validateT1(goodT1({ scenarios: ['NOPE'] }), caps()).errors.scenarios, /Unknown scenario/);
  assert.match(validateT1(goodT1({ arms: ['BL-0', 'x; rm -rf'] }), caps()).errors.arms, /Unknown arm/);
  assert.match(validateT1(goodT1({ families: ['W-SF'] }), caps()).errors.families, /Unknown family/, 'the sealed family is not in the catalogue');
  assert.equal(validateT1(goodT1({ name: 'my run_1.2-x' }), caps()).valid, true);
  for (const name of ['a/b', 'a;b', 'x'.repeat(65), '../../x', 'tab\tname', 'ünï']) {
    assert.ok(validateT1(goodT1({ name }), caps()).errors.name, JSON.stringify(name));
  }
  assert.equal(validateT1(goodT1({ name: 'my run' }), caps()).spec.name, 'my run');
  assert.equal('name' in validateT1(goodT1({ name: '   ' }), caps()).spec, false, 'blank name is omitted');
});

test('T1 form: W-Z alpha applies to W-Z only, must be in (0, 10], and rewrites the family id', () => {
  assert.equal(ALPHA_MAX, 10);
  assert.deepEqual(validateT1(goodT1({ wzAlpha: '0.9' }), caps()).spec.families, ['W-Z(0.9)']);
  assert.deepEqual(validateT1(goodT1({ wzAlpha: '10' }), caps()).spec.families, ['W-Z(10)']);
  for (const a of ['0', '10.1', '-1', 'abc', '1e2', '0.0']) assert.ok(validateT1(goodT1({ wzAlpha: a }), caps()).errors.wzAlpha, a);
  assert.match(validateT1(goodT1({ families: ['W-FC'], wzAlpha: '1' }), caps()).errors.wzAlpha, /W-Z only/);
});

test('T1 form: profile and arm-parameter choices must be ones the gateway offers', () => {
  assert.ok(validateT1(goodT1({ profile: 'huge' }), caps()).errors.profile);
  assert.ok(validateT1(goodT1({ armParams: 'custom' }), caps()).errors.armParams);
  const c = caps();
  c.t1.armParams = { frozen: false, untuned: true };
  assert.match(validateT1(goodT1({ armParams: 'frozen' }), c).errors.armParams, /unavailable/);
});

test('T1 form: when the kind is unavailable the form cannot be valid and says why', () => {
  const c = caps();
  c.t1.available = false;
  const r = validateT1(goodT1(), c);
  assert.equal(r.valid, false);
  assert.ok(r.errors._form);
  assert.equal(validateT1(goodT1(), null).valid, false);
});

function goodSb(over = {}) {
  return { ...emptySbForm(caps()), ...over };
}

test('Scenario B form: defaults come from the capabilities limits; the default form is valid and sends only what was filled in', () => {
  const f = emptySbForm(caps());
  assert.equal(f.repetitions, '10');
  assert.equal(f.rate, '300');
  assert.equal(f.keys, '5000');
  assert.deepEqual(f.arms, ['BASE', 'STATIC-CAP', 'AIMD-CAP']);
  const r = validateSb(f, caps());
  assert.equal(r.valid, true);
  assert.deepEqual(r.spec, { repetitions: 10, arms: ['BASE', 'STATIC-CAP', 'AIMD-CAP'], rate_rps: 300, n_keys: 5000 });
  assert.equal('warmup_s' in r.spec, false, 'untouched optional fields are omitted so the API default applies');
});

test('Scenario B form: ranges mirror the API (repetitions, phases, rate, keys, caps)', () => {
  assert.match(validateSb(goodSb({ repetitions: '31' }), caps()).errors.repetitions, /between 1 and 30/);
  assert.match(validateSb(goodSb({ repetitions: '0' }), caps()).errors.repetitions, /between 1 and 30/);
  assert.match(validateSb(goodSb({ repetitions: '' }), caps()).errors.repetitions, /required/);
  assert.match(validateSb(goodSb({ warmup: '61' }), caps()).errors.warmup, /between 1 and 60/);
  assert.match(validateSb(goodSb({ degraded: '0' }), caps()).errors.degraded, /between 1 and 60/);
  assert.match(validateSb(goodSb({ rate: '2001' }), caps()).errors.rate, /between 1 and 2000/);
  assert.equal(validateSb(goodSb({ rate: '12.5' }), caps()).spec.rate_rps, 12.5, 'the rate may be fractional');
  assert.match(validateSb(goodSb({ keys: '9' }), caps()).errors.keys, /between 10 and 100000/);
  assert.match(validateSb(goodSb({ keys: '100001' }), caps()).errors.keys, /between 10 and 100000/);
  assert.match(validateSb(goodSb({ staticCap: '1001' }), caps()).errors.staticCap, /between 1 and 1000/);
  assert.match(validateSb(goodSb({ aimdMin: '20', aimdMax: '10' }), caps()).errors.aimdMin, /must not exceed/);
  const ok = validateSb(goodSb({ warmup: '1', healthy: '1', degraded: '2', recovery: '1', staticCap: '5', aimdMin: '2', aimdMax: '9' }), caps());
  assert.equal(ok.valid, true);
  assert.deepEqual(ok.spec, { repetitions: 10, arms: ['BASE', 'STATIC-CAP', 'AIMD-CAP'], warmup_s: 1, healthy_s: 1, degraded_s: 2, recovery_s: 1, rate_rps: 300, n_keys: 5000, static_cap: 5, aimd_min: 2, aimd_max: 9 });
});

test('Scenario B form: arms must be a non-empty set of the three known arms, without duplicates', () => {
  assert.ok(validateSb(goodSb({ arms: [] }), caps()).errors.arms);
  assert.ok(validateSb(goodSb({ arms: ['BASE', 'BASE'] }), caps()).errors.arms);
  assert.ok(validateSb(goodSb({ arms: ['BASE', 'FAST'] }), caps()).errors.arms);
  assert.equal(validateSb(goodSb({ arms: ['BASE'] }), caps()).valid, true);
});

test('Scenario B form: warnings state what will not be VALID, and why, without blocking submission', () => {
  const few = validateSb(goodSb({ repetitions: '3' }), caps());
  assert.equal(few.valid, true);
  assert.ok(few.warnings.some((w) => /Fewer than 10 repetitions.*EXPLORATORY/.test(w)));
  assert.ok(few.warnings.some((w) => /debug build/.test(w)), 'the recorded gateway is a debug build');
  const nobase = validateSb(goodSb({ arms: ['STATIC-CAP'] }), caps());
  assert.ok(nobase.warnings.some((w) => /BASE arm/.test(w)));
  const unavailable = caps();
  unavailable.sb.available = false;
  unavailable.sb.reason = 'gateway executable not found';
  const u = validateSb(goodSb(), unavailable);
  assert.equal(u.valid, false);
  assert.match(u.errors._form, /gateway executable not found/);
});

test('exact RunSpec JSON: the POST body is {kind, spec} and parses back to the same values', () => {
  const r = validateT1(goodT1({ name: 'repro' }), caps());
  const json = runSpecJson('t1_matrix', r.spec);
  assert.deepEqual(JSON.parse(json), { kind: 't1_matrix', spec: r.spec });
  assert.ok(json.includes('\n'), 'pretty-printed');
  assert.ok(!/seed_values|"S"/.test(json));
});

test('equivalent command: T1 and Scenario B map every field to the phantom-lab flags; unsafe characters are quoted', () => {
  const t1 = validateT1(goodT1({ name: 'my run', wzAlpha: '0.9', recordRaw: true, profile: 'standard', armParams: 'frozen', seedSet: 'T' }), caps());
  const cmd = labCommand('t1_matrix', t1.spec);
  assert.equal(cmd, 'phantom-lab run --scenarios O-DEG --families "W-Z(0.9)" --arms BL-0,BL-1 --seed-set T --n-seeds 2 --profile standard --arm-params frozen --name "my run" --raw --out lab-results/my-run');
  const sb = validateSb(goodSb({ repetitions: '3', arms: ['BASE', 'STATIC-CAP'], warmup: '1', healthy: '1', degraded: '2', recovery: '1', staticCap: '5', aimdMin: '2', aimdMax: '9' }), caps());
  assert.equal(
    labCommand('scenario_b', sb.spec),
    'phantom-lab scenario-b --reps 3 --arms BASE,STATIC-CAP --warmup-ms 1000 --healthy-ms 1000 --degraded-ms 2000 --recovery-ms 1000 --rate 300 --keys 5000 --static-cap 5 --aimd-min 2 --aimd-max 9 --out lab-results/scenario_b/my-run',
  );
  assert.equal(labCommand('t1_matrix', null), '');
  assert.ok(!/seed-set S/i.test(cmd));
});
