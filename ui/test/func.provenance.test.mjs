// DEC-007: retained Scenario B summary evidence is REAL + RETAINED, never REPLAYED; PREDICTED is never produced.
import test from 'node:test';
import assert from 'node:assert/strict';
import { TIERS, tierInfo, kindInfo } from '../src/lib/labProvenance.js';
import { evidenceItemView } from '../src/lib/labView.js';
import { FX } from './helpers/labFixtures.mjs';

test('DEC-007: retained Scenario B evidence maps to word Real + retained flag, not Replayed', () => {
  const t = tierInfo('RETAINED_MEASURED_LOOPBACK');
  assert.equal(t.word, 'Real');
  assert.equal(t.retained, true);
  assert.ok(t.qualifier && /loopback/.test(t.qualifier), 'REAL always carries its basis qualifier');
  assert.doesNotMatch(t.label, /Replayed|Predicted|Retained/);
  const item = evidenceItemView(FX.sbEvidence());
  assert.equal(item.tier.word, 'Real');
  assert.equal(item.tier.retained, true);
});

test('DEC-007: no Lab tier or kind produces REPLAYED or PREDICTED; they remain labels only', () => {
  for (const [k, v] of Object.entries(TIERS)) assert.ok(!/replayed|predicted/i.test(v.word), k);
  for (const kind of ['scenario_b', 't1_matrix', 'm6_sealed_evaluation', 'm6_dry_run', 'tuning', 'smoke']) {
    const tier = kindInfo(kind).tier;
    assert.ok(tier === null || !/REPLAYED|PREDICTED/.test(tier), kind);
  }
  assert.equal(tierInfo('REPLAYED').key, 'UNAVAILABLE', 'a tier string REPLAYED from the API is unknown, not honoured');
  assert.equal(tierInfo('PREDICTED').key, 'UNAVAILABLE');
});
