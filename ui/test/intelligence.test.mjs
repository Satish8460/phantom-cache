// Intelligence screen (lane L2, step 2): view model and rendered page.
// Fixtures: test/fixtures/intelligence/README.md. status_legacy_real.json is a REAL capture; the others are CONTRACT-SHAPED (hand-written from
// docs/lanes/CONTRACT-pie-api-v1.md), not captured, because no gateway served the v2 API when this lane ran.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';
import {
  availability, cell, classify, decisionsPath, decisionsView, formatNumber, intelligenceLive, intelligenceNavLive, modeView, registryView, signalsView, statusView,
} from '../src/lib/intelligenceView.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.join(HERE, '../src');
const fx = (n) => JSON.parse(fs.readFileSync(path.join(HERE, 'fixtures/intelligence', n), 'utf8'));
const ok = (data) => ({ ok: true, status: 200, authError: null, data });
const http = (status) => ({ ok: false, status, authError: null, data: null });
const code = (f) => fs.readFileSync(path.join(SRC, f), 'utf8').split('\n').filter((l) => !/^\s*(\/\/|\/\*|\*)/.test(l)).join('\n');

let H;
before(async () => {
  H = await getHarness();
});

// ------------------------------------------------------------------------------------------------ value cells

test('cell: measured and derived values keep the provenance the gateway stated; samples and units are carried; numbers are rounded, not scaled', () => {
  assert.deepEqual(cell({ value: 0.4237, src: 'derived', samples: 118, unit: 'ratio' }), { text: '0.4237', src: 'derived', samples: 118, unit: 'ratio', note: null, reason: null });
  assert.equal(cell({ value: 48210, src: 'measured', samples: 5 }).text, '48,210');
  assert.equal(cell({ value: 0.123456789, src: 'derived', samples: 1 }).text, '0.1235');
  assert.equal(cell({ value: true, src: 'derived', samples: 1 }).text, 'yes');
  assert.equal(cell({ value: 'skewed_read_heavy', src: 'derived', samples: 1 }).text, 'skewed_read_heavy');
  assert.equal(formatNumber(Number.NaN), null);
});

test('cell: unavailable shows the reason in words and no value; invalid values are withheld, never turned into zero', () => {
  const u = cell({ value: null, src: 'unavailable', samples: 40, reason: 'insufficient_samples' });
  assert.deepEqual([u.text, u.src, u.reason, u.samples], [null, 'unavailable', 'Not enough samples yet', 40]);
  assert.match(cell({ value: null, src: 'unavailable', samples: 0, reason: 'something_new_here' }).reason, /something new here/);
  assert.equal(cell({ value: 3, src: 'unavailable', samples: 0, reason: 'x' }).text, null, 'unavailable that carries a value is rejected');
  assert.match(cell({ value: 3, src: 'unavailable', samples: 0 }).reason, /marked unavailable but carries a value/);
  assert.match(cell({ value: 3, src: 'simulated', samples: 1 }).reason, /simulated value cannot be shown as live/);
  assert.equal(cell({ value: 3, src: 'invented', samples: 1 }).src, 'unavailable');
  assert.equal(cell({ value: Number.NaN, src: 'measured', samples: 1 }).text, null);
  assert.equal(cell({ value: 'x'.repeat(200), src: 'measured', samples: 1 }).text, null);
  assert.equal(cell({ value: { a: 1 }, src: 'measured', samples: 1 }).text, null);
  assert.equal(cell(undefined).src, 'unavailable');
  assert.equal(cell(7).src, 'unavailable');
  assert.equal(cell({ value: 1, src: 'measured', samples: -3 }).samples, null);
});

test('mode: only off, observe and advisory are normal; canary and active are shown as unexpected, never as a state', () => {
  for (const m of ['off', 'observe', 'advisory']) assert.equal(modeView({ value: m, src: 'measured', samples: 1 }).known, true, m);
  assert.equal(modeView({ value: 'advisory', src: 'measured', samples: 1 }).label, 'Advisory');
  for (const m of ['canary', 'active', 'Active', 'on']) {
    const v = modeView({ value: m, src: 'measured', samples: 1 });
    assert.equal(v.known, false, m);
    assert.equal(v.key, 'unexpected');
  }
  assert.equal(modeView('Advisory').key, 'advisory', 'the legacy status gives a name');
  assert.equal(modeView(null).key, 'unknown');
  assert.equal(modeView({ value: null, src: 'unavailable', samples: 0, reason: 'x' }).text, null);
});

// ------------------------------------------------------------------------------------------------ availability

test('availability: every answer of the status endpoint maps to one honest state', () => {
  assert.equal(availability(null), 'loading');
  assert.equal(availability(http(401)), 'unauthorised');
  assert.equal(availability(http(403)), 'unauthorised');
  assert.equal(availability(http(0)), 'no_answer');
  assert.equal(availability(http(404)), 'endpoints_missing');
  assert.equal(availability(http(500)), 'error');
  assert.equal(availability({ ok: true, status: 200, data: 'not an object' }), 'error');
  assert.equal(availability(ok(fx('status_legacy_real.json'))), 'legacy');
  assert.equal(availability(ok(fx('status_v2_not_installed.json'))), 'not_installed');
  assert.equal(availability(ok(fx('status_v2_advisory.json'))), 'ready');
  assert.equal(availability(ok({ ...fx('status_legacy_real.json'), v2: 'x' })), 'error');
  assert.equal(availability(ok({ v2: { installed: true, schema: 'pie-api-v2' } })), 'error', 'a different schema string is not read as v1');
  assert.equal(classify(null).kind, 'loading');
  assert.equal(intelligenceLive(ok(fx('status_v2_advisory.json'))), true);
  assert.equal(intelligenceLive(ok(fx('status_legacy_real.json'))), false);
  assert.equal(intelligenceLive(http(404)), false);
});

test('status (REAL capture from the release binary): the older shape gives the mode only, nothing else is invented', () => {
  const real = fx('status_legacy_real.json');
  assert.equal(real.intelligence_mode, 'Off');
  const v = statusView(ok(real));
  assert.equal(v.availability, 'legacy');
  assert.equal(v.mode.label, 'Off');
  assert.equal(v.mode.src, 'measured');
  assert.equal(v.legacy.truthful, true);
  assert.equal(v.queue, undefined, 'no queue, window or memory block exists for a gateway that does not report one');
});

test('status v2: counters carry the gateway provenance; drops and memory are shown with their basis; the claims block says no benefit', () => {
  const v = statusView(ok(fx('status_v2_advisory.json')));
  assert.equal(v.mode.label, 'Advisory');
  assert.deepEqual([v.queue.enqueued.text, v.queue.enqueued.src, v.queue.dropped.text, v.queue.dropped.src], ['48,210', 'measured', '17', 'measured']);
  assert.equal(v.memory.bytes.src, 'derived', 'a sum of declared capacities is derived, not measured');
  assert.equal(v.memory.bytes.note, 'sum of declared capacities');
  assert.equal(v.memory.budget, '8,388,608');
  assert.equal(v.decisions.retainedCount, 3);
  assert.equal(v.claims.benefit, 'none');
  assert.equal(v.config.labMode, false);
  assert.equal(v.fault, null);
  const faulty = fx('status_v2_advisory.json');
  faulty.v2.fault = { decision_id: 31, timestamp_ms: 410500, reason_code: 'clock_jump', detail: 'backwards' };
  assert.equal(statusView(ok(faulty)).fault.reason, 'clock_jump');
});

// ------------------------------------------------------------------------------------------------ registry, signals, decisions

test('registry: policies with lifecycle, highest mode and the effective state; documentation is declared text; a policy that is not implemented says so', () => {
  const r = registryView(ok(fx('registry.json')));
  assert.equal(r.present, true);
  assert.deepEqual(r.policies.map((p) => [p.id, p.implemented, p.lifecycle, p.maxRuntime]), [['POL-EVICT-1', true, 'ShadowEvaluated', 'advisory'], ['POL-TTL-1', false, 'Proposed', 'off']]);
  assert.equal(r.policies[0].effective.src, 'measured');
  assert.equal(r.policies[0].parameters[0].current, '0.02');
  assert.equal(r.policies[0].inputs[0].minSamples, 100);
  assert.ok(r.policies[0].docs.some((d) => d.field === 'limitations'));
  const odd = fx('registry.json');
  odd.policies[0].lifecycle = 'Deployed';
  odd.policies[0].max_runtime_state = 'active';
  const o = registryView(ok(odd)).policies[0];
  assert.equal(o.lifecycleKnown, false);
  assert.equal(o.maxRuntimeKnown, false, 'a policy claiming it may run "active" is flagged, not shown as normal');
  assert.equal(registryView(http(404)).present, false);
  assert.match(registryView(http(404)).reason, /does not serve this endpoint/);
  assert.equal(registryView(ok({ installed: false, reason: 'intelligence_v2_not_installed' })).kind, 'not_installed');
});

test('signals: unavailable features keep their reason; only 16-hex key hashes are shown (a raw key or tenant can never appear); a cold start says no window', () => {
  const s = signalsView(ok(fx('signals_advisory.json')));
  assert.equal(s.present, true);
  const f = Object.fromEntries(s.features.map((x) => [x.id, x.cell]));
  assert.equal(f.F2.text, '0.4237');
  assert.equal(f.F4.reason, 'Not enough samples yet');
  assert.equal(f.F8, undefined, 'a feature that can never exist here is left out, not printed as Unavailable');
  assert.equal(f.F15, undefined);
  assert.equal(f.F7.text, 'no', 'a derived flag is shown as the word, not as an absence');
  assert.equal(s.classification.text, 'skewed_read_heavy');
  assert.deepEqual(s.topKeys.items.map((i) => i.hash), ['00ab12cd34ef5678', 'ffee00112233aabb']);
  const hostile = fx('signals_advisory.json');
  hostile.top_keys.items.push({ key_hash: '/api/v1/cache/secret/path?token=abc', count: 9 }, { key_hash: 'tenant-a', count: 1 });
  assert.equal(signalsView(ok(hostile)).topKeys.items.length, 2, 'anything that is not a 16-digit hash is dropped');
  const cold = signalsView(ok(fx('signals_observe_cold.json')));
  assert.equal(cold.window, null);
  assert.ok(cold.features.every((x) => x.cell.text === null && /No window has closed yet/.test(x.cell.reason)));
  assert.equal(cold.classification.text, null);
});

test('decisions: newest first; inputs, candidates with intervals, rejection reasons, constraints and the explanation are kept', () => {
  const d = decisionsView(ok(fx('decisions.json')));
  assert.equal(d.present, true);
  assert.deepEqual(d.records.map((r) => r.id), [31, 19, 7], 'the gateway sends oldest first; the page shows newest first');
  const rec = d.records.find((r) => r.id === 19);
  assert.equal(rec.verdict, 'Recommend');
  assert.deepEqual(rec.inputs.map((i) => [i.feature, i.cell.src, i.cell.samples]), [['F2', 'derived', 512], ['F10', 'measured', 512]]);
  assert.deepEqual(rec.candidates.map((c) => [c.id, c.status, c.rejection]), [['S3-FIFO', 'selected', null], ['LRU', 'rejected', 'below_margin']]);
  assert.equal(rec.candidates[0].effect.ci95, '0.02 to 0.08');
  assert.equal(rec.candidates[0].effect.cell.src, 'derived');
  assert.equal(rec.recommendation.label, 'advisory, not applied');
  assert.equal(rec.observed.text, null, 'the observed effect is unavailable and says why');
  assert.equal(rec.observed.reason, 'Not implemented in this version');
  assert.deepEqual(rec.guardrails, ['provenance_refused:F10']);
  assert.equal(rec.conflict.lever, 'cache_replacement');
  assert.match(rec.explanation, /shadow S3-FIFO cache/);
  assert.equal(d.records.find((r) => r.id === 7).conflict, null);
  assert.equal(decisionsView(ok(fx('decisions_empty.json'))).records.length, 0);
});

test('decisions: nothing is ever presented as applied; a record claiming it was applied, simulated or in an unknown mode is flagged, not trusted', () => {
  const bad = fx('decisions.json');
  const r19 = bad.records.find((r) => r.decision_id === 19);
  r19.recommendation.applied = true;
  r19.simulated = true;
  r19.mode = 'active';
  r19.verdict = 'Executed';
  const v = decisionsView(ok(bad)).records.find((r) => r.id === 19);
  assert.equal(v.recommendation, null, 'a recommendation that is not marked applied:false is withheld');
  assert.equal(v.recommendationWithheld, true);
  assert.equal(v.simulatedClaim, true);
  assert.equal(v.modeKnown, false);
  assert.equal(v.verdictKnown, false);
  const missing = fx('decisions.json');
  delete missing.records.find((r) => r.decision_id === 19).recommendation.applied;
  assert.equal(decisionsView(ok(missing)).records.find((r) => r.id === 19).recommendation, null, 'applied missing is not false');
});

test('decisions: sections that the gateway does not serve are unavailable with the reason, never empty tables', () => {
  assert.match(decisionsView(http(404)).reason, /does not serve this endpoint/);
  assert.match(decisionsView(http(0)).reason, /No answer/);
  assert.match(signalsView(http(403)).reason, /refused/);
  assert.equal(decisionsView(ok({ installed: true, records: 'nope' })).present, false);
});

test('decisionsPath asks for the NEWEST records using the retained count the gateway reported', () => {
  assert.equal(decisionsPath(100, 20), '/api/v1/intelligence/decisions?limit=20&offset=80');
  assert.equal(decisionsPath(3, 20), '/api/v1/intelligence/decisions?limit=20&offset=0');
  assert.equal(decisionsPath(null, 20), '/api/v1/intelligence/decisions?limit=20&offset=0');
  assert.equal(decisionsPath(-5), '/api/v1/intelligence/decisions?limit=20&offset=0');
});

// ------------------------------------------------------------------------------------------------ rendered page

const body = (raw) => H.renderLab(H.React.createElement(H.Intel.IntelligenceBody, { raw }));
const full = (status, registry, signals, decisions) => ({ status: ok(status), registry: ok(registry), signals: ok(signals), decisions: ok(decisions), at: 1 });
const READY = () => full(fx('status_v2_advisory.json'), fx('registry.json'), fx('signals_advisory.json'), fx('decisions.json'));

test('render ready: every value in a table cell is followed by a provenance badge; the mode statement and the no-benefit statement are present', () => {
  const html = body(READY());
  assert.match(html, /Only three modes exist: Off, Observe and Advisory\. There is no canary and no active mode/);
  assert.match(html, /M6 H1\) is NOT_SUPPORTED/);
  assert.match(html, /data-intel-mode="advisory"/);
  assert.match(html, /Applied changes: none/);
  const cells = html.match(/<td>(?:(?!<\/td>)[\s\S])*<\/td>/g) || [];
  const valued = cells.filter((c) => c.includes('data-intel-value='));
  assert.ok(valued.length >= 15, `found ${valued.length} value cells`);
  for (const c of valued) assert.match(c, /class="prov prov--(measured|derived)"/, 'every shown value has its badge');
  // every unavailable cell has the UNAVAILABLE badge and a reason
  assert.match(html, /prov--unavailable[\s\S]*?Not enough samples yet/);
  assert.doesNotMatch(html, /Not implemented in this version/, 'permanent gaps are omitted, not listed');
  assert.match(html, /data-intel-decision="19"/);
  assert.match(html, /advisory, not applied/);
  assert.match(html, /Not applied: nothing was changed\./);
  assert.match(html, /95% interval 0\.02 to 0\.08/);
  assert.doesNotMatch(html, /Observed effect after the fact/, 'a row that has no value on this gateway is left out');
  assert.deepEqual(lintHtml(html, { requireH1: false }), []);
});

test('render ready: no simulated badge and no invented rows; drops and memory counters are on the page', () => {
  const html = body(READY());
  assert.doesNotMatch(html, /prov--simulated/);
  assert.match(html, /Events dropped \(queue full\), total/);
  assert.match(html, /data-intel-value="queue-dropped">17</);
  assert.match(html, /data-intel-value="memory-bytes">3,145,728</);
  assert.match(html, /A sum of declared capacities, not an allocator measurement/);
  assert.equal((html.match(/data-intel-decision="/g) || []).length, 3);
  assert.equal((html.match(/data-intel-policy="/g) || []).length, 2);
  assert.match(html, /POL-TTL-1[\s\S]*?\(not implemented\)/);
});

test('render ready: empty decisions say so, a cold signal set says no window; nothing is filled in', () => {
  const html = body(full(fx('status_v2_advisory.json'), fx('registry.json'), fx('signals_observe_cold.json'), fx('decisions_empty.json')));
  assert.match(html, /data-intel-decisions="none"/);
  assert.match(html, /No decisions have been recorded/);
  assert.match(html, /data-intel-window="none"/);
  assert.equal((html.match(/data-intel-decision="/g) || []).length, 0);
  assert.equal((html.match(/data-intel-feature=/g) || []).length, 2, 'only the two features the gateway listed');
  assert.deepEqual(lintHtml(html, { requireH1: false }), []);
});

test('render ready: a section the gateway did not serve says so, with the reason, while the others still render', () => {
  const raw = { ...READY(), registry: http(404), decisions: http(0) };
  const html = body(raw);
  assert.match(html, /data-intel-unavailable="registry"[\s\S]*?does not serve this endpoint/);
  assert.match(html, /data-intel-unavailable="decisions"[\s\S]*?No answer from the gateway/);
  assert.match(html, /data-intel-features/);
});

test('render ready: an unexpected mode and a fault are shown as what they are', () => {
  const st = fx('status_v2_advisory.json');
  st.v2.mode.value = 'active';
  st.v2.fault = { decision_id: 31, timestamp_ms: 1, reason_code: 'panic', detail: 'worker panicked' };
  const html = body({ ...READY(), status: ok(st) });
  assert.match(html, /data-intel-mode="unexpected"/);
  assert.match(html, /does not recognise/);
  assert.match(html, /stopped itself after a fault/);
  assert.match(html, /Reason panic: worker panicked/);
});

test('render states: loading, refused, no answer, endpoints missing (with the old capability statement), legacy, not installed', () => {
  assert.match(body({ status: null }), /Asking the gateway about Phantom Intelligence/);
  assert.match(body({ status: http(401) }), /The gateway refused the request/);
  assert.match(body({ status: http(403) }), /telemetry:read role/);
  assert.match(body({ status: http(0) }), /No answer from the gateway/);
  const missing = H.renderIntelligence({ ...makeCtx({ frames: [livePayload()] }), intel: { raw: { status: http(404) } } });
  assert.match(missing, /does not serve the Phantom Intelligence API/);
  assert.match(missing, /Phantom Intelligence is not available yet|does not report controller capability/, 'the capability statement stays as the honest fallback');
  assert.doesNotMatch(missing, /data-intel-mode=/);
  const legacy = body({ status: ok(fx('status_legacy_real.json')) });
  assert.match(legacy, /only the older intelligence status/);
  assert.match(legacy, /data-intel-mode="off"/);
  assert.match(legacy, /Only three modes exist/);
  assert.doesNotMatch(legacy, /data-intel-registry|data-intel-decision/);
  const ni = body({ status: ok(fx('status_v2_not_installed.json')) });
  assert.match(ni, /not installed in this gateway process/);
  assert.match(ni, /Nothing is shown here because nothing is running/);
  for (const html of [missing, legacy, ni]) assert.deepEqual(lintHtml(html, { requireH1: false }), []);
});

test('render screen: Intelligence inside the page frame, with data it shows the page, with no telemetry yet it shows the connection state (no values)', () => {
  const ctxOk = { ...makeCtx({ frames: [livePayload()] }), intel: { raw: READY(), refresh: () => {} } };
  const html = H.renderIntelligence(ctxOk);
  assert.match(html, /<p class="page-head__group">Live gateway<\/p><h1[^>]*>Phantom Intelligence<\/h1>/, 'with a working API the caption matches the nav group');
  assert.match(html, /data-intel-registry/);
  assert.deepEqual(lintHtml(html), []);
  const none = H.renderIntelligence({ ...makeCtx({ frames: [] }), intel: { raw: READY(), refresh: () => {} } });
  assert.doesNotMatch(none, /data-intel-registry|data-intel-value/);
});

// ------------------------------------------------------------------------------------------------ navigation

test('nav: Intelligence stays under "Not available yet" until the gateway says the API is installed, then moves to Live gateway after Cluster', () => {
  const before = H.renderShell({ connKind: 'live', currentId: 'overview' });
  assert.match(before, /Not available yet/);
  assert.match(before, /data-screen="intelligence"[^>]*>[\s\S]*?Not available/);
  const live = H.renderShell({ connKind: 'live', currentId: 'intelligence', intelligenceLive: true });
  assert.doesNotMatch(live, /Not available yet/);
  assert.doesNotMatch(live, /app-nav__fold/);
  assert.equal((live.match(/<a href="#\/[^"]*"/g) || []).length, 10, 'still ten links');
  const order = [...live.matchAll(/data-screen="([a-z]+)"/g)].map((m) => m[1]);
  assert.deepEqual(order, ['start', 'overview', 'traffic', 'cache', 'freshness', 'origin', 'cluster', 'intelligence', 'lab', 'simulation']);
  assert.match(live, /data-screen="intelligence"[^>]*>(?:(?!<\/a>)[\s\S])*<\/a>/);
  assert.doesNotMatch(/data-screen="intelligence"[^>]*>(?:(?!<\/a>)[\s\S])*<\/a>/.exec(live)[0], /not yet available/);
  assert.equal((live.match(/aria-current="page"/g) || []).length, 1);
  assert.deepEqual(lintHtml(live, { requireH1: false }), []);
});

// ------------------------------------------------------------------------------------------------ source guards

test('guard: the Intelligence modules make no network call of their own, log nothing, store nothing, and compute no metrics', () => {
  for (const f of ['lib/intelligenceView.js', 'screens/Intelligence.jsx', 'hooks/useIntelligence.js']) {
    const c = code(f);
    assert.doesNotMatch(c, /\bfetch\s*\(|EventSource|XMLHttpRequest|WebSocket/, f);
    assert.doesNotMatch(c, /console\.|localStorage|sessionStorage|document\.cookie|Math\.random|dangerouslySetInnerHTML|innerHTML/, f);
    assert.doesNotMatch(c, /\.value\s*[*/+-]\s*[\w(]/, `${f}: no arithmetic on gateway values`);
  }
  const hook = code('hooks/useIntelligence.js');
  assert.match(hook, /client\.getJson\(PATHS\.status/);
  assert.match(hook, /if \(activeRef\.current && classify\(st\)\.kind === 'ok'\)/, 'signals, decisions and registry are read only while the screen is open and the gateway answered');
  assert.match(hook, /ctl\.abort\(\)/);
  assert.match(hook, /addEventListener\('pagehide'/);
  assert.match(hook, /\[gatewayUrl, auth\.version\]/, 'answers are dropped when the gateway or the token changes');
  // a refusal is passed to the sign-in object
  assert.match(hook, /st\.status === 401 \|\| st\.status === 403\) reportRef\.current\(/);
});

test('guard: App routes Intelligence to the screen, reads the API through the signed-in client, and tells the nav whether it is live', () => {
  const app = fs.readFileSync(path.join(SRC, 'App.jsx'), 'utf8');
  assert.match(app, /case 'intelligence': return <Intelligence \/>;/);
  assert.match(app, /useIntelligence\(auth, gw\.url, \{ active: route\.id === 'intelligence' \}\)/);
  assert.match(app, /const intelLive = intelligenceNavLive\(intel\.raw\.status\);/);
  const shell = fs.readFileSync(path.join(SRC, 'components/Shell.jsx'), 'utf8');
  assert.match(shell, /useContext\(TelemetryContext\)/);
  assert.doesNotMatch(shell, /token|sessionStorage/i);
});

test('nav (friendly): Intelligence is a normal live item while the gateway is being asked and when the API is there; only a gateway that ANSWERED without the API gets the old copy', () => {
  assert.equal(intelligenceNavLive(null), true, 'no flash of "Not available yet" while asking');
  assert.equal(intelligenceNavLive(ok(fx('status_v2_advisory.json'))), true);
  assert.equal(intelligenceNavLive(http(401)), true, 'a sign-in is needed, not a missing feature');
  assert.equal(intelligenceNavLive(http(404)), false);
  assert.equal(intelligenceNavLive(ok(fx('status_legacy_real.json'))), false);
  assert.equal(intelligenceNavLive(ok(fx('status_v2_not_installed.json'))), false);
});

test('screen (friendly): with the live API there is no "Not available", no "Unavailable" row for a gap that is permanent; transient gaps keep their honest label', () => {
  const html = body(READY());
  assert.doesNotMatch(html, /Not available|not yet available|placeholder/i);
  assert.match(html, /Not enough samples yet/, 'a value that will appear once samples exist is still labelled');
  assert.doesNotMatch(html, /needs a multi-node view|Not implemented in this version/i);
});
