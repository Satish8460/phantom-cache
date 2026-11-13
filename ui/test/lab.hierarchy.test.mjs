// Tests for the Phantom Lab hierarchy and primary workflow (increment I2): status strip, page head and primary action, form
// behaviour (touched-state validation, presets, defaults), provenance wording, elapsed time, evidence structure, CSS hygiene.
// These are REGRESSION GUARDS on markup, logic and stylesheet rules. They are not evidence of visual quality: layout, spacing,
// reading speed and "does the landing page answer what this is in ten seconds" are judged by looking at the page in a browser.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { capabilitiesView, elapsedView, evidenceDetailView, evidenceListView, formatElapsed, parseUtcMs, runView, runsListView, scenarioBView } from '../src/lib/labView.js';
import { initialRunModel, initialStream, streamReducer } from '../src/lib/labStream.js';
import { TIERS, tierBadgeQualifier, tierInfo } from '../src/lib/labProvenance.js';
import { mapLabError } from '../src/lib/labApi.js';
import {
  ADVANCED_FIELDS, defaultKind, emptySbForm, emptyT1Form, errorSummary, presetsFor, validateSb, validateT1, visibleErrors,
} from '../src/lib/labForms.js';
import { FX } from './helpers/labFixtures.mjs';
import { makeCtx, livePayload } from './helpers/fixtures.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (p) => fs.readFileSync(path.join(ROOT, p), 'utf8');
const labCss = read('src/styles/lab.css');
let H;
let h;
before(async () => {
  H = await getHarness();
  h = H.React.createElement;
});
const noop = () => {};
const count = (html, needle) => html.split(needle).length - 1;
const lint = (html, label) => assert.deepEqual(lintHtml(html, { requireH1: false }), [], label);
const ready = (data) => ({ status: 'ready', data, error: null, reload: noop });
const strip = (res, view, extra = {}) => H.renderLab(h(H.Lab.ConnectionPanel, { res, view, hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'http://127.0.0.1:18085', ...extra }));

// ---------------------------------------------------------------- page head, primary action, status strip

test('first screen: page title, then the primary action "Start a run", then the status strip, then the tabs; no banner above the tabs', () => {
  const html = H.renderScreen('lab', makeCtx({ frames: [livePayload()] }), undefined);
  const at = (needle) => html.indexOf(needle);
  assert.ok(at('<h1 ') >= 0 && at('<h1 ') < at('data-primary-action'));
  assert.ok(at('data-primary-action') < at('data-lab-status'));
  assert.ok(at('data-lab-status') < at('aria-label="Lab sections"'));
  const aboveTabs = html.slice(0, at('aria-label="Lab sections"'));
  assert.equal(count(aboveTabs, 'class="banner'), 0, 'at most two banner rows above the primary task; the strip is not a banner');
  assert.equal(count(aboveTabs, 'data-lab-status'), 1);
  assert.doesNotMatch(aboveTabs, /<h2/, 'no section heading competes with the page head above the tabs');
  assert.doesNotMatch(html, /data-run-detail/);
  lint(html, 'lab first screen');
});

test('status strip: an error, API off, or a missing required token opens Details by itself; a debug build alone does not', () => {
  const err = (status, headers, body) => ({ status: 'error', data: null, error: mapLabError({ status, headers, bodyJson: null, bodyText: body }), reload: noop });
  const closed = strip(ready(FX.caps()), capabilitiesView(FX.caps()));
  assert.match(closed, /aria-expanded="false"/);
  assert.match(closed, /class="lab-status__details" hidden=""/);
  assert.doesNotMatch(closed, /data-attention/);
  const net = strip(err(502, {}, ''), null);
  const off = strip(err(404, {}, ''), null);
  const missing = strip(err(401, { 'x-auth-error': 'MissingToken' }, 'Missing Bearer authorization token'), null);
  for (const [name, html] of [['unreachable', net], ['api off', off], ['token required', missing]]) {
    assert.match(html, /aria-expanded="true"/, name);
    assert.match(html, /data-attention="true"/, name);
    assert.doesNotMatch(html, /class="lab-status__details" hidden=""/, name);
    assert.match(html, /Intelligence: not in serving path/, `${name}: the honesty chip stays`);
  }
  assert.match(missing, /type="password"/);
  const jwtNoToken = strip(ready(FX.capsJwt()), capabilitiesView(FX.capsJwt()));
  assert.match(jwtNoToken, /Token required/);
  assert.match(jwtNoToken, /aria-expanded="true"/, 'a JWT gateway without a token needs the token field');
  const jwtToken = strip(ready(FX.capsJwt()), capabilitiesView(FX.capsJwt()), { hasToken: true, roles: ['admin'] });
  assert.match(jwtToken, /Token set/);
  assert.match(jwtToken, /aria-expanded="false"/);
  for (const html of [closed, net, off, missing, jwtNoToken, jwtToken]) lint(html, 'strip');
});

test('status strip: honesty chips are text with glyphs; an active run is a link chip; Details repeats the full intelligence statement', () => {
  const c = FX.caps();
  c.active_run = { run_id: '20261030T034041Z-c78a73cf', kind: 'scenario_b', state: 'running' };
  const html = strip(ready(c), capabilitiesView(c));
  const chips = /<ul class="lab-status__chips"[\s\S]*?<\/ul>/.exec(html)[0];
  assert.match(chips, /<a class="lab-status__active" href="#\/lab\/runs\/20261030T034041Z-c78a73cf">A run is active: open it<\/a>/);
  assert.match(chips, /<svg[\s\S]*?Debug build: results EXPLORATORY/);
  assert.match(html, /<strong>The intelligence engine is not part of the live serving path\.<\/strong>/);
  assert.match(html, /Nothing on this page controls live traffic/);
  assert.doesNotMatch(html, /Mode: live|Gateway <span class="mono">http:\/\/localhost/, 'the shell header owns the gateway mode');
});

// ---------------------------------------------------------------- form logic

test('touched-state validation: no error on a pristine form; one field after its blur; all after a failed submit; summary in form order', () => {
  const caps = capabilitiesView(FX.caps());
  const form = emptyT1Form();
  const r = validateT1(form, caps);
  assert.equal(r.valid, false);
  assert.deepEqual(visibleErrors(r.errors, {}, false), {}, 'pristine: nothing visible');
  assert.deepEqual(Object.keys(visibleErrors(r.errors, { nSeeds: true }, false)), ['nSeeds'], 'after blur of one field only that field shows');
  assert.deepEqual(Object.keys(visibleErrors(r.errors, {}, true)).sort(), Object.keys(r.errors).sort(), 'after a submit attempt everything shows');
  assert.deepEqual(errorSummary('t1_matrix', r.errors).map((e) => e.field), ['scenarios', 'families', 'arms', 'nSeeds']);
  assert.deepEqual(errorSummary('t1_matrix', r.errors).map((e) => e.label), ['Scenarios', 'Workload families', 'Arms', 'Number of seeds']);
  assert.ok(errorSummary('t1_matrix', r.errors).every((e) => e.message.length > 5));
  const sb = validateSb({ ...emptySbForm(caps), rate: '0', aimdMin: '9', aimdMax: '3' }, caps);
  assert.deepEqual(errorSummary('scenario_b', sb.errors).map((e) => e.field), ['rate', 'aimdMin']);
  assert.deepEqual(errorSummary('scenario_b', { _form: 'x' }).map((e) => e.label), ['This form']);
});

test('advanced fields: every advanced field is a real form field; the groups are one level deeper, and a failed submit can open them', () => {
  const caps = capabilitiesView(FX.caps());
  assert.deepEqual(ADVANCED_FIELDS.scenario_b.sort(), ['aimdMax', 'aimdMin', 'degraded', 'healthy', 'keys', 'rate', 'recovery', 'staticCap', 'warmup']);
  for (const f of ADVANCED_FIELDS.scenario_b) assert.ok(f in emptySbForm(caps), f);
  for (const f of ADVANCED_FIELDS.t1_matrix) assert.ok(f in emptyT1Form(), f);
  const bad = validateSb({ ...emptySbForm(caps), warmup: 'x' }, caps);
  assert.ok(Object.keys(bad.errors).some((k) => ADVANCED_FIELDS.scenario_b.includes(k)));
});

test('defaults and presets: Scenario B is the default kind when available; presets fill only form fields and yield VALID requests', () => {
  const caps = capabilitiesView(FX.caps());
  assert.equal(defaultKind(caps), 'scenario_b');
  const off = FX.caps();
  off.kinds.scenario_b.available = false;
  assert.equal(defaultKind(capabilitiesView(off)), 't1_matrix');
  assert.equal(defaultKind(null), 't1_matrix');
  const sbPresets = presetsFor('scenario_b', caps);
  assert.deepEqual(sbPresets.map((p) => p.id), ['quick', 'full']);
  for (const p of sbPresets) {
    const r = validateSb({ ...emptySbForm(caps), ...p.patch }, caps);
    assert.equal(r.valid, true, `${p.id}: ${JSON.stringify(r.errors)}`);
    assert.ok(p.summary.length > 30);
    assert.ok(Object.keys(p.patch).every((k) => k in emptySbForm(caps)), `${p.id} sets only form fields`);
  }
  assert.deepEqual(sbPresets[0].patch.arms, ['BASE', 'STATIC-CAP']);
  assert.equal(sbPresets[1].patch.repetitions, '10', 'the full preset uses the number the gateway says a VALID label needs');
  assert.match(sbPresets[0].summary, /EXPLORATORY/);
  const t1 = presetsFor('t1_matrix', caps);
  assert.equal(t1.length, 1);
  const r = validateT1({ ...emptyT1Form(), ...t1[0].patch }, caps);
  assert.equal(r.valid, true, JSON.stringify(r.errors));
  assert.equal(r.spec.seed_set, 'D');
  assert.equal(r.spec.n_seeds, 1);
  assert.equal(r.spec.profile, 'smoke');
  assert.match(t1[0].summary, /says nothing about the comparison/);
});

test('long catalogues: 12 scenarios, 7 families and 19 arms are three collapsed groups with counts, not 38 visible rows', () => {
  const caps = capabilitiesView(FX.caps());
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps, onSubmit: noop, initialKind: 't1_matrix' }));
  assert.equal(count(html, 'type="checkbox"') - count(html, 'Record raw per-request files'), 12 + 7 + 19 + 0);
  assert.equal(count(html, '<details class="disclosure">'), 3, 'the three catalogue groups are closed');
  assert.match(html, /<details class="disclosure" data-request-disclosure="true">/, 'the exact request is a closed disclosure too');
  assert.equal(count(html, '<details class="disclosure" open'), 0);
  assert.equal(count(html, 'class="choice-list__tools"'), 3);
  assert.match(html, /class="lab-choices"/);
  assert.match(read('src/styles/lab.css'), /\.lab-choices \{[^}]*grid-template-columns: repeat\(auto-fill, minmax\(min\(100%, 13rem\), 1fr\)\)/);
  lint(html, 'matrix form');
});

// ---------------------------------------------------------------- provenance wording (lead decision (a))

test('provenance wording: MEASURED displays as Real with its basis qualifier everywhere in the Lab; Retained is a separate tag; no "Measured"/"Inferred" word', () => {
  assert.deepEqual(Object.keys(TIERS), ['MEASURED_LOOPBACK', 'SIMULATED', 'RETAINED_SIMULATED', 'RETAINED_MEASURED_LOOPBACK', 'DERIVED', 'UNAVAILABLE']);
  assert.equal(tierBadgeQualifier(tierInfo('MEASURED_LOOPBACK'), 'debug'), 'loopback, one host, debug build');
  assert.equal(tierBadgeQualifier(tierInfo('MEASURED_LOOPBACK'), 'release'), 'loopback, one host');
  assert.equal(tierBadgeQualifier(tierInfo('SIMULATED'), 'debug'), null, 'the build qualifier applies to REAL only');
  const badge = (tier, build) => H.renderLab(h(H.Lab.TierBadge, { tier, build }));
  assert.match(badge('MEASURED_LOOPBACK', 'debug'), /<span class="prov__word">Real<\/span><span class="prov__q">loopback, one host, debug build<\/span>/);
  assert.match(badge('MEASURED_LOOPBACK'), /<span class="prov__word">Real<\/span><span class="prov__q">loopback, one host<\/span>/);
  assert.doesNotMatch(badge('MEASURED_LOOPBACK'), /data-retained/);
  const retained = badge('RETAINED_MEASURED_LOOPBACK');
  assert.match(retained, /prov__word">Real<\/span>/);
  assert.match(retained, /<span class="stamp" data-retained="true"[^>]*>(?:<svg[\s\S]*?<\/svg>)?Retained<\/span>/);
  assert.match(badge('RETAINED_SIMULATED'), /prov__word">Simulated<\/span>[\s\S]*data-retained="true"/);
  assert.match(badge('DERIVED'), /prov__word">Derived<\/span>/);
  assert.match(badge('UNAVAILABLE'), /prov__word">Unavailable<\/span>/);
  assert.match(badge('SOMETHING_NEW'), /prov__word">Unavailable<\/span><span class="prov__q">unknown tier<\/span>/);
  // No reserved class is invented: REPLAYED and PREDICTED never come out of a Lab tier.
  for (const k of Object.keys(TIERS)) assert.doesNotMatch(badge(k), /prov--(replayed|predicted)/);
});

test('provenance wording across pages: every rendered Lab page uses Real/Simulated/Derived/Unavailable and never the old words', () => {
  const sb = evidenceDetailView(FX.sbEvidence());
  const m6 = evidenceDetailView(FX.m6Sealed());
  const stream = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  const detail = (run, summary) => H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView(run), live: { status: 'live', stream, model: initialRunModel }, artifacts: { state: 'ready', rows: [] }, summary: summary || { state: 'none' },
    cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null, nowMs: Date.parse(run.started_utc) + 5000,
  }));
  const pages = {
    sbEvidence: H.renderLab(h(H.Lab.EvidenceDetailBody, { d: sb })),
    m6Evidence: H.renderLab(h(H.Lab.EvidenceDetailBody, { d: m6 })),
    list: H.renderLab(h(H.Lab.EvidenceListBody, { view: evidenceListView(FX.evidenceIndex()) })),
    runs: H.renderLab(h(H.Lab.RunsListBody, { view: runsListView(FX.runsList()) })),
    running: detail(FX.runSbRunning()),
    done: detail(FX.runSbFinal(), { state: 'ready', view: scenarioBView(FX.sbSummary()) }),
    story: H.renderLab(h(H.Lab.StoryView, { evidence: evidenceListView(FX.evidenceIndex()), m6Detail: m6.m6, sbDetail: sb.sb })),
  };
  for (const [name, html] of Object.entries(pages)) {
    const visible = html.replace(/<[^>]*>/g, ' ');
    assert.doesNotMatch(visible, /\bMeasured\b|\bMEASURED\b|\bInferred\b|Retained: /, `${name}: old wording`);
    for (const m of html.matchAll(/<span class="prov prov--measured"[^>]*>[\s\S]*?<\/span><span class="prov__q">([^<]*)<\/span>/g)) assert.match(m[1], /loopback/, `${name}: REAL always has its qualifier`);
    assert.equal(count(html, 'prov--measured') , count(html, 'prov--measured"') , name);
    lint(html, name);
  }
  // REAL without a qualifier would be a bug: count badges against qualifiers.
  for (const [name, html] of Object.entries(pages)) {
    const real = [...html.matchAll(/<span class="prov prov--measured"[^>]*>[\s\S]*?<\/span>(<span class="prov__q">[^<]*<\/span>)?/g)];
    for (const m of real) assert.ok(m[1], `${name}: a REAL badge without a qualifier`);
  }
  assert.match(pages.running, /prov__q">loopback, one host, debug build</, 'the run header says REAL, loopback, debug build');
});

// ---------------------------------------------------------------- elapsed time

test('elapsedView: value only from real timestamps; unavailable (with a reason) otherwise; never zero or negative', () => {
  const t0 = Date.parse('2026-10-30T07:06:10Z');
  assert.equal(parseUtcMs('2026-10-30T07:06:10Z'), t0);
  assert.equal(parseUtcMs(null), null);
  assert.equal(parseUtcMs('not a date'), null);
  assert.equal(parseUtcMs({ unavailable: true, reason: 'x' }), null);
  assert.deepEqual(elapsedView({ startedMs: t0, endedMs: null, nowMs: t0 + 5500, terminal: false }).text, '5 s');
  assert.equal(elapsedView({ startedMs: t0, endedMs: t0 + 125000, nowMs: 0, terminal: true }).text, '2 min 5 s');
  assert.equal(elapsedView({ startedMs: t0, endedMs: null, nowMs: 0, terminal: true }).state, 'unavailable');
  assert.match(elapsedView({ startedMs: t0, endedMs: null, nowMs: 0, terminal: true }).reason, /no end time/);
  assert.match(elapsedView({ startedMs: null, nowMs: t0, terminal: false }).reason, /start time/);
  assert.match(elapsedView({ startedMs: t0, nowMs: t0 - 1, terminal: false }).reason, /disagree/);
  assert.equal(elapsedView({ startedMs: t0, nowMs: NaN, terminal: false }).state, 'unavailable');
  assert.equal(formatElapsed(59999), '59 s');
  assert.equal(formatElapsed(3600000 + 120000), '1 h 2 min');
});

// ---------------------------------------------------------------- evidence structure

test('Scenario B evidence: an "On this page" list with one link per section; operational detail is a closed fold with a count; the notice is not in a fold', () => {
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  const nav = /<nav class="lab-toc"[\s\S]*?<\/nav>/.exec(html)[0];
  const ids = [...nav.matchAll(/href="#([a-z0-9-]+)"/g)].map((m) => m[1]);
  assert.deepEqual(ids, ['sb-glance', 'sb-not-established', 'sb-notice', 'sb-charts', 'sb-arms', 'sb-paired', 'sb-mechanism', 'sb-config', 'sb-limits']);
  for (const id of ids) assert.equal(count(html, ` id="${id}"`), 1, `anchor target ${id}`);
  const folds = [...html.matchAll(/<details class="disclosure lab-fold" id="([^"]+)"( open="")?/g)];
  // V2A: the limitations are an open section, never a closed fold (rubric must-pass 4); the other operational detail stays folded.
  assert.deepEqual(folds.map((m) => m[1]), ['sb-arms', 'sb-paired', 'sb-mechanism', 'sb-config', 'evidence-files', 'evidence-manifest']);
  assert.ok(folds.every((m) => !m[2]), 'every fold starts closed');
  assert.match(html, /<span class="lab-fold__count">2 arm\(s\) compared with BASE<\/span>/);
  const noticeAt = html.indexOf('id="sb-notice"');
  assert.ok(noticeAt > 0 && !/<details[^>]*>(?:(?!<\/details>)[\s\S])*id="sb-notice"/.test(html), 'the binding notice is not inside a closed fold');
  assert.ok(html.indexOf('id="sb-limits"') > 0);
  assert.equal(count(html, 'data-rich-notice'), 1);
  assert.match(html, /data-notice-pane="rendered"/);
  assert.doesNotMatch(html, /Scenario B — dated interpretation and correction notice \(2026-10-30 07:14 \+05:30\)\s*\n/);
  // the anchors never touch the router: the click handler prevents default (hash routing owns location.hash)
  assert.match(read('src/components/LabParts.jsx'), /e\.preventDefault\(\);\s*\n\s*const el = document\.getElementById\(id\)/);
  lint(html, 'sb evidence');
});

test('negative results keep the same visual weight: one VerdictChip component, same classes, for every outcome (TRADE_OFF_FAILURE, NOT_SUPPORTED, SUPPORTED)', () => {
  const chip = (o) => H.renderLab(h(H.Lab.VerdictChip, { outcome: o, name: 'X' })).replace(/data-verdict="[A-Z_]+" data-verdict-kind="[a-z]+"/, 'V').replace(/pill tone-[a-z]+/, 'pill tone-T').replace(/<svg[\s\S]*?<\/svg>/, '<svg/>').replace(/<span>[^<]*<\/span><\/span><\/span>$/, '<span>W</span></span></span>');
  const shapes = new Set(['TRADE_OFF_FAILURE', 'NOT_SUPPORTED', 'MECHANISM_FAILED', 'SUPPORTED', 'BENEFIT_SUPPORTED', 'INCONCLUSIVE', 'UNVERIFIED'].map(chip));
  assert.equal(shapes.size, 1);
  const sb = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  assert.match(sb, /<li data-verdict-arm="STATIC-CAP">[\s\S]*?<div class="lab-verdict-head"><span class="verdict" data-verdict="TRADE_OFF_FAILURE"/);
  assert.doesNotMatch(labCss, /data-verdict|verdict--|verdict-kind/, 'no stylesheet rule treats a particular outcome differently');
});

// ---------------------------------------------------------------- run detail structure

test('run detail: the header holds name, id, state, REAL tier with qualifier, elapsed, progress, live status and Cancel; Cancel is the only danger action', () => {
  const run = FX.runSbRunning();
  const html = H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView({ ...run, progress: undefined }), live: { status: 'live', stream: streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 }), model: initialRunModel },
    artifacts: { state: 'loading', rows: [] }, summary: { state: 'none' }, cancel: { phase: 'idle', error: null },
    onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null, nowMs: Date.parse(run.started_utc) + 3000,
  }));
  const head = /<div class="lab-detail-head lab-run-head"[\s\S]*?data-run-status-live[^>]*>[^<]*<\/p><\/div>/.exec(html)[0];
  assert.match(head, /<h2 class="lab-run-head__kind"/);
  assert.match(head, /data-run-state="running"/);
  assert.match(head, /prov--measured/);
  assert.match(head, /data-elapsed="true"/);
  assert.match(head, /data-live-chip="live"/);
  assert.match(head, /btn btn--danger[^>]*data-cancel-open/);
  assert.equal(count(html, 'btn--danger'), 1);
  assert.match(html, /class="progress-indeterminate"/, 'no progress figures: an indeterminate bar with words, not a made-up percentage');
  assert.match(html, /Progress not reported yet/);
  assert.doesNotMatch(html, /<progress /, 'no determinate bar without figures');
  assert.match(labCss, /\.lab-run-head \{ position: sticky; top: calc\(var\(--header-h\) \+ var\(--lab-sticky-top, 0px\)\); z-index: 3; \}/, 'sticky below the sticky app header');
  assert.match(labCss, /@media \(min-width: 820px\) \{[\s\S]*?\.lab-run-head \{ position: sticky/);
  lint(html, 'run header');
});

test('run detail (completed): "Result at a glance" comes first with a contents list; artifacts show sizes and hashes; settings and event log are closed folds', () => {
  const stream = streamReducer(initialStream, { type: 'done', state: 'completed', now: 1 });
  const html = H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView(FX.runSbFinal()), live: { status: 'ended', stream, model: initialRunModel },
    artifacts: { state: 'ready', rows: [{ name: 'summary.json', bytes: '12.3 KiB', rawBytes: 12600, downloadable: true, sha256: { state: 'value', text: 'ab12'.repeat(16) }, sha256Of: 'file bytes', tier: null }] },
    summary: { state: 'ready', view: scenarioBView(FX.sbSummary()) }, cancel: { phase: 'idle', error: null },
    onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null,
  }));
  assert.ok(html.indexOf('id="sb-glance"') > 0 && html.indexOf('id="sb-glance"') < html.indexOf('id="sb-charts"'));
  assert.ok(html.indexOf('id="sb-glance"') < html.indexOf('id="run-artifacts"'));
  assert.ok(html.indexOf('id="run-artifacts"') < html.indexOf('id="run-settings"'));
  assert.match(html, /<span class="num">12\.3 KiB<\/span>|<td class="num">12\.3 KiB<\/td>/);
  assert.match(html, /ab12ab12ab12ab12/);
  const folds = [...html.matchAll(/<details class="disclosure lab-fold" id="([^"]+)"( open="")?/g)].map((m) => [m[1], Boolean(m[2])]);
  assert.deepEqual(folds.filter(([id]) => ['run-settings', 'run-events'].includes(id)), [['run-settings', false], ['run-events', false]]);
  assert.doesNotMatch(html, /data-event-tail/, 'the tail is for active runs; a finished run has the full log fold');
  assert.doesNotMatch(html, /data-live-chip="live"/);
  assert.equal(count(html, 'aria-label="Download '), 1);
  lint(html, 'completed');
});

test('run detail: a gap in event numbers is still reported while the stream is live (the banner exists for problems, not for normal live)', () => {
  const stream = { ...streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 }), gaps: 2 };
  const html = H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView(FX.runSbRunning()), live: { status: 'live', stream, model: initialRunModel }, artifacts: { state: 'loading', rows: [] }, summary: { state: 'none' },
    cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null,
  }));
  assert.match(html, /2 gap\(s\) in event numbers/);
  assert.doesNotMatch(html, /Reconnect now/, 'live with a gap: no reconnect button, only the statement');
});

// ---------------------------------------------------------------- stylesheet hygiene (I2 acceptance 5)

test('lab.css: tokens only; no dashed or dotted borders; no duplicated input/danger/disabled rules; no nested-card leftovers; shadows only from tokens', () => {
  const css = labCss.replace(/\/\*[\s\S]*?\*\//g, '');
  assert.doesNotMatch(css, /#[0-9a-fA-F]{3,8}\b/, 'no hex colours');
  assert.doesNotMatch(css, /\brgba?\(|\bhsla?\(/, 'no raw colour functions (the scrim is a token)');
  assert.doesNotMatch(css, /\bdashed\b|\bdotted\b|\bdouble\b/, 'dashed and dotted are reserved for provenance badges');
  assert.doesNotMatch(css, /input\[type="(text|password)"\]/, 'input rules live in app.css');
  assert.doesNotMatch(css, /\.btn--danger\s*[{:]|\.btn:disabled/, 'button rules live in app.css');
  assert.doesNotMatch(css, /\.lab-tag|\.lab-effect__zero \{[^}]*dashed|\.lab-unavail-panel|\.lab-fieldset|\.lab-check\b|\.lab-run-layout/);
  for (const m of css.matchAll(/box-shadow:\s*([^;}]+)/g)) assert.match(m[1].trim(), /^(none|var\(--shadow-[123]\))$/, `box-shadow ${m[1]}`);
  for (const m of css.matchAll(/(?<![-\w])z-index:\s*(\d+)/g)) assert.ok(Number(m[1]) <= 3, 'sticky layers stay below the dialog/scrim (z-index 20)');
  const src = ['src/screens/lab/LabScreen.jsx', 'src/screens/lab/RunForm.jsx', 'src/screens/lab/RunsViews.jsx', 'src/screens/lab/EvidenceViews.jsx', 'src/screens/lab/StoryView.jsx', 'src/screens/lab/ScenarioBResult.jsx', 'src/components/LabParts.jsx', 'src/components/LabCharts.jsx'].map(read).join('\n');
  assert.doesNotMatch(src, /lab-tag|lab-unavail-panel|lab-fieldset|lab-check\b|lab-run-layout|<Panel\b/, 'no old class and no boxed Panel wrappers left in the Lab');
  assert.doesNotMatch(src, /dangerouslySetInnerHTML|innerHTML/, 'no raw HTML anywhere in the Lab');
});

test('a Lab page contains no nested panel: cards (L1) appear only as sections or list items, never inside another card', () => {
  const pages = [
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) })),
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) })),
    H.renderLab(h(H.Lab.EvidenceListBody, { view: evidenceListView(FX.evidenceIndex()) })),
  ];
  for (const html of pages) {
    assert.doesNotMatch(html, /class="panel[ "]/);
    // a card section never contains another card section or a .lab-card
    for (const m of html.matchAll(/<section class="lab-section lab-section--card"[\s\S]*?<\/section>/g)) {
      assert.doesNotMatch(m[0].slice(m[0].indexOf('>') + 1), /lab-section--card|class="lab-card"|class="panel[ "]/);
    }
  }
});

test('accessibility: one h1 per Lab screen; headings never skip a level inside the Lab body; every section has a heading and an id', () => {
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  const levels = [...html.matchAll(/<h([1-6])[\s>]/g)].map((m) => Number(m[1]));
  assert.ok(levels.every((l) => l >= 2));
  let prev = 1;
  for (const l of levels) {
    assert.ok(l <= prev + 1, `heading level jumps from ${prev} to ${l}`);
    prev = l;
  }
  const sections = [...html.matchAll(/<section class="lab-section[^>]*>/g)].map((m) => m[0]);
  assert.ok(sections.length >= 3);
  for (const tag of sections) assert.match(tag, /id="[^"]+"[^>]*aria-labelledby="[^"]+"/, 'every section has an id and is labelled by its heading');
  assert.match(html, /<h2 id="[^"]+" tabindex="-1" class="lab-section__title">/, 'section headings can take focus from the contents list');
});

// ---------------------------------------------------------------- I2b

test('I2b: Result at a glance leads with the verdicts; the run-class statement is a closed disclosure below them, verbatim; negative results keep equal weight', () => {
  const d = evidenceDetailView(FX.sbEvidence());
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d }));
  const glance = html.slice(html.indexOf('id="sb-glance"'), html.indexOf('id="sb-charts"'));
  const verdictAt = glance.indexOf('data-verdict-arm=');
  const discAt = glance.indexOf('data-runclass="true"');
  assert.ok(verdictAt > 0 && discAt > verdictAt, 'verdicts come first, the run-class statement after');
  assert.ok(!/<details/.test(glance), 'V2A: the run-class statement is binding wording and is not collapsed');
  assert.equal(count(glance, 'data-verdict="TRADE_OFF_FAILURE"'), 2);
  assert.ok(glance.includes(`data-label-meaning="true">${d.sb.labelMeaning.replace(/&/g, '&amp;').replace(/'/g, '&#x27;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;')}</p>`), 'the statement is the payload text, unchanged');
});

test('I2b: the strip does not repeat an error that the body states; the catalogue ids are mono with a note that the gateway gives no names', () => {
  const caps = capabilitiesView(FX.caps());
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps, onSubmit: noop, initialKind: 't1_matrix' }));
  assert.equal(count(html, 'data-ids-note'), 3);
  assert.match(html, /it provides no names or descriptions for them/);
  assert.ok(count(html, '<span class="mono">') >= 38);
  assert.match(html, /<span class="lab-choice-group__count">none selected, 19 available<\/span>/);
  assert.match(labCss, /:root:has\(\.lab-form\) \{ scroll-padding-bottom: 6rem; \}/, 'focus and scrolling keep fields clear of the sticky bar');
});

test('I2b: on a Scenario B result the Arms, Mechanism and Limitations sections are closed folds with visible headings and counts', () => {
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  for (const id of ['sb-arms', 'sb-mechanism']) assert.match(html, new RegExp(`<details class="disclosure lab-fold" id="${id}" data-lab-section="${id}" data-fold="true"><summary class="lab-fold__summary"><h2 class="lab-fold__title">`), id);
  // V2A: the limitations are an open section with the same counts in its note
  assert.match(html, /<section class="lab-section" id="sb-limits"[\s\S]*?\d+ limitation\(s\), \d+ interpretation note\(s\) carried in the result itself/);
  // folding is not removal: the limitation text is in the page
  assert.match(html, /data-limitations/);
});
