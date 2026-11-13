// Server-side render tests for the Phantom Lab screens, driven by REAL captured API responses.
// No browser: these check what the DOM would contain in each data state. Visual quality is not tested here.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import {
  artifactsView, capabilitiesView, evidenceDetailView, evidenceListView, runView, runsListView, scenarioBView,
} from '../src/lib/labView.js';
import { applyRunEvent, initialRunModel, initialStream, streamReducer } from '../src/lib/labStream.js';
import { parseEventData, parseSseText } from '../src/lib/sse.js';
import { mapLabError } from '../src/lib/labApi.js';
import { FX } from './helpers/labFixtures.mjs';
import { makeCtx, livePayload } from './helpers/fixtures.mjs';

let H;
let h;
before(async () => {
  H = await getHarness();
  h = H.React.createElement;
});

const noop = () => {};
const caps = () => capabilitiesView(FX.caps());
const ready = (data) => ({ status: 'ready', data, error: null, reload: noop });

function modelFrom(sse) {
  let m = initialRunModel;
  for (const ev of parseSseText(sse)) {
    const p = parseEventData(ev);
    m = applyRunEvent(m, { id: Number(ev.id), event: ev.event, data: p.value, malformed: !p.ok });
  }
  return m;
}

function streamState(kind) {
  let s = streamReducer(streamReducer(initialStream, { type: 'connect' }), { type: 'opened', now: 0 });
  s = streamReducer(s, { type: 'frame', id: 1, now: 0 });
  if (kind === 'closed') s = streamReducer(s, { type: 'closed' });
  if (kind === 'ended') s = streamReducer(s, { type: 'done', state: 'completed', now: 1 });
  if (kind === 'failed') s = streamReducer(s, { type: 'error', error: { kind: 'forbidden_role', title: 'The token does not have the required role' } });
  return s;
}

function detail({ run, sse, status = 'live', stream = streamState('open'), artifacts, summary = { state: 'none' }, cancel = { phase: 'idle', error: null }, downloading = null, downloadError = null }) {
  const rv = runView(run);
  return h(H.Lab.RunDetailBody, {
    run: rv,
    live: { status, stream, model: sse ? modelFrom(sse) : initialRunModel },
    artifacts: artifacts || { state: 'loading', rows: [] },
    summary,
    cancel,
    onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop,
    downloading, downloadError,
  });
}

const lint = (html, label) => assert.deepEqual(lintHtml(html, { requireH1: false }), [], label);
const count = (html, needle) => html.split(needle).length - 1;

// ---------------------------------------------------------------- shell / routes

test('Lab is a real screen: reachable at #/lab and its sub-routes, no longer a placeholder, with one h1 and a skip-friendly structure', () => {
  const ctx = makeCtx({ frames: [livePayload()] });
  for (const sub of [undefined, { view: 'evidence', id: null }, { view: 'evidence', id: 'scenario_b.20261030T003932Z' }, { view: 'run', id: null }, { view: 'runs', id: null }, { view: 'runs', id: '20261030T034142Z-a4de24b6' }]) {
    const html = H.renderScreen('lab', ctx, sub);
    assert.equal(count(html, '<h1 '), 1, JSON.stringify(sub));
    assert.match(html, /data-screen-heading/);
    assert.match(html, /aria-label="Lab sections"/);
    assert.match(html, /data-state-kind="loading"/, 'first paint is a loading state, never numbers');
    assert.doesNotMatch(html, /ui_api_requirements/, 'not the placeholder');
    lint(html, JSON.stringify(sub));
  }
  const nav = H.renderShell({ connKind: 'live', currentId: 'lab' });
  assert.match(nav, /Phantom Lab/);
  assert.doesNotMatch(nav, /Phantom Lab<\/span><span class="visually-hidden"> \(not yet available\)/);
});

test('Lab sub-navigation marks the current view and every link has an href and a name', () => {
  const ctx = makeCtx({});
  const html = H.renderScreen('lab', ctx, { view: 'runs', id: null });
  // I2: workflow order (About, Start a run, Runs, Evidence), shared .tab class, and a primary "Start a run" action in the page head.
  assert.match(html, /<a class="tab" href="#\/lab\/runs" aria-current="page"/);
  assert.equal(count(html, 'aria-current="page"'), 1);
  for (const href of ['#/lab"', '#/lab/evidence"', '#/lab/run"', '#/lab/runs"']) assert.ok(html.includes(`href="${href}`), href);
  const labels = [...html.matchAll(/data-lab-view="[a-z]+">([^<]+)</g)].map((m) => m[1]);
  assert.deepEqual(labels, ['About', 'Start a run', 'Runs', 'Evidence']);
  assert.match(html, /<a class="btn btn--primary lab-head__action" href="#\/lab\/run" data-primary-action="true">Start a run<\/a>/);
  const runPage = H.renderScreen('lab', ctx, { view: 'run', id: null });
  assert.doesNotMatch(runPage, /data-primary-action/, 'the primary action is not repeated on the page it leads to');
});

// ---------------------------------------------------------------- connection panel

test('status strip: anonymous mode shows the honesty chips, a closed Details region with security mode and availability; no token field', () => {
  const view = caps();
  const html = H.renderLab(h(H.Lab.ConnectionPanel, { res: ready(FX.caps()), view, hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'http://127.0.0.1:18085' }));
  assert.match(html, /Lab API: Live/);
  assert.match(html, /data-security-mode="anonymous_dev_loopback_only"/);
  assert.match(html, /Anonymous, development, loopback only/);
  assert.match(html, /Simulation matrix \(T1, simulated\)/);
  assert.match(html, /Scenario B \(T2, real processes on loopback\)/);
  // I2 (lead decision (c)): the always-open Connection panel is a status strip. The honesty statements stay visible as chips.
  const chips = /<ul class="lab-status__chips"[\s\S]*?<\/ul>/.exec(html)[0];
  assert.match(chips, /Lab API: Live/);
  assert.match(chips, /Anonymous, loopback/);
  assert.match(chips, /Debug build: results EXPLORATORY/);
  assert.match(chips, /Intelligence: not in serving path/);
  assert.match(html, /data-debug-explainer/);
  assert.match(html, /The intelligence engine is not part of the live serving path/);
  assert.match(html, /aria-expanded="false"/);
  assert.match(html, /class="lab-status__details" hidden=""/, 'closed when nothing needs attention (a debug build is a chip, not an alarm)');
  assert.doesNotMatch(html, /data-attention/);
  assert.doesNotMatch(html, /class="banner/, 'the strip is not a stack of banners');
  assert.doesNotMatch(html, /type="password"/, 'anonymous gateway: no token field');
  lint(html, 'anonymous');
});

test('connection panel: JWT mode shows a password-type token field that never echoes a value and explains memory-only storage', () => {
  const view = capabilitiesView(FX.capsJwt());
  const html = H.renderLab(h(H.Lab.ConnectionPanel, { res: ready(FX.capsJwt()), view, hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'http://127.0.0.1:18086' }));
  assert.match(html, /JWT authentication/);
  assert.match(html, /<input id="[^"]+" type="password" autoComplete="off"/);
  assert.match(html, /required by this gateway/);
  assert.match(html, /never put in a URL, never written to browser storage/);
  assert.match(html, /value=""/);
  const withToken = H.renderLab(h(H.Lab.ConnectionPanel, { res: ready(FX.capsJwt()), view, hasToken: true, onToken: noop, onClear: noop, gatewayUrl: 'x' }));
  assert.match(withToken, /A token is set for this tab/);
  assert.doesNotMatch(withToken, /value="[^"]+"/, 'the token is never rendered back');
  lint(html, 'jwt');
});

test('status strip: loading, disabled (Lab API off), unauthorized and unreachable are each distinct states; the error text is NOT repeated in the strip (the page body states it once)', () => {
  // I2b: the strip carries a chip and a one-line pointer; the title and remedy come from the body's ErrorBlock / StateBlock (see the error-block test).
  const panel = (res, view = null) => H.renderLab(h(H.Lab.ConnectionPanel, { res, view, hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'http://x' }));
  const failed = (status, headers, body) => ({ status: 'error', data: null, error: mapLabError({ status, headers, bodyJson: null, bodyText: body }), reload: noop });
  const loading = panel({ status: 'loading', data: null, error: null });
  assert.match(loading, /Checking/);
  const off = panel(failed(404, {}, ''));
  assert.match(off, /Lab API: Unavailable/);
  assert.match(off, /switched off on this gateway/);
  assert.doesNotMatch(off, /type="password"/, 'turning the Lab on is the fix, not a token');
  const missing = panel(failed(401, { 'x-auth-error': 'MissingToken' }, 'Missing Bearer authorization token'));
  assert.match(missing, /Lab API: Unauthorised/); // V2A: the merged phrase is retired (func.link); one vocabulary
  assert.match(missing, /type="password"/);
  const role = panel(failed(403, { 'x-auth-error': 'InsufficientPermissions' }, 'Forbidden'));
  const net = panel(failed(502, {}, ''));
  for (const [name, html, res] of [['off', off, failed(404, {}, '')], ['missing', missing, failed(401, { 'x-auth-error': 'MissingToken' }, '')], ['role', role, failed(403, {}, '')], ['net', net, failed(502, {}, '')]]) {
    assert.match(html, /data-error-pointer/, name);
    assert.ok(!html.includes(res.error.title.replace(/'/g, '&#x27;')), `${name}: the error title appears once, in the body, not in the strip`);
    assert.doesNotMatch(html, /role="alert"/, name);
  }
  assert.notEqual(missing, net);
  for (const html of [loading, off, missing, role, net]) lint(html, 'panel state');
});


test('connection panel: an active run is linked, and a gateway that reports Scenario B unavailable says why', () => {
  const c = FX.caps();
  c.active_run = { run_id: '20261030T034041Z-c78a73cf', kind: 'scenario_b', state: 'running' };
  c.kinds.scenario_b.available = false;
  c.kinds.scenario_b.reason = 'gateway executable not found';
  const html = H.renderLab(h(H.Lab.ConnectionPanel, { res: ready(c), view: capabilitiesView(c), hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'x' }));
  assert.match(html, /href="#\/lab\/runs\/20261030T034041Z-c78a73cf"/);
  assert.match(html, /Not available/);
  assert.match(html, /gateway executable not found/);
});

// ---------------------------------------------------------------- error / gate states

test('error blocks: each error kind renders its own title and remedy, as an alert (or a status for "disabled")', () => {
  const E = FX.errors();
  const seen = new Set();
  for (const [key, e] of Object.entries(E)) {
    if (key.startsWith('_') || key.startsWith('cancel_')) continue;
    let json = null;
    try { json = e.body ? JSON.parse(e.body) : null; } catch { json = null; }
    const error = mapLabError({ status: e.status, headers: e.headers, bodyJson: json, bodyText: e.body, hadToken: true });
    const html = H.renderLab(h(H.Lab.ErrorBlock, { error, onRetry: noop, what: 'the evidence index' }));
    assert.ok(html.includes(error.title.replace(/'/g, '&#x27;')), key);
    assert.match(html, error.kind === 'disabled' ? /role="status"/ : /role="alert"/);
    assert.match(html, /Retry/);
    seen.add(error.kind);
    lint(html, key);
  }
  assert.ok(seen.size >= 8, `distinct kinds rendered: ${[...seen]}`);
  const auth = H.renderLab(h(H.Lab.ErrorBlock, { error: mapLabError({ status: 401, headers: { 'x-auth-error': 'MissingToken' }, bodyJson: null, bodyText: '' }), what: 'runs' }));
  assert.match(auth, /Lab status strip above/);
});

test('resource gate: loading and error never render the children; ready does; refresh errors keep the data visible', () => {
  const body = () => h('p', { id: 'kid' }, 'child');
  const g = (res) => H.renderLab(h(H.Lab.ResourceGate, { res, what: 'x' }, body));
  assert.doesNotMatch(g({ status: 'loading', data: null, error: null }), /id="kid"/);
  assert.match(g({ status: 'loading', data: null, error: null }), /aria-busy="true"/);
  assert.doesNotMatch(g({ status: 'error', data: null, error: { kind: 'network', title: 't', fix: 'f' } }), /id="kid"/);
  assert.match(g({ status: 'ready', data: {}, error: null }), /id="kid"/);
  const both = g({ status: 'error', data: {}, error: { kind: 'network', title: 'could not refresh', fix: '' } });
  assert.match(both, /id="kid"/);
  assert.match(both, /could not refresh/);
  assert.equal(g({ status: 'idle', data: null, error: null }), '');
});

// ---------------------------------------------------------------- story

test('story: problem, hypothesis and evidence; both negative results are shown with equal weight and the intelligence notice', () => {
  const idx = evidenceListView(FX.evidenceIndex());
  const m6 = evidenceDetailView(FX.m6Sealed());
  const sb = evidenceDetailView(FX.sbEvidence());
  const html = H.renderLab(h(H.Lab.StoryView, { evidence: idx, m6Detail: m6.m6, sbDetail: sb.sb }));
  assert.match(html, /The problem/);
  assert.match(html, /The hypothesis/);
  assert.match(html, /What the evidence shows/);
  assert.match(html, /What this does not show/);
  assert.match(html, /data-verdict="NOT_SUPPORTED"/);
  assert.equal(count(html, 'data-verdict="TRADE_OFF_FAILURE"'), 2);
  assert.match(html, /data-story-m6-counts/);
  assert.match(html, /supported in <span class="num">0<\/span> of 8 scenarios/);
  assert.match(html, /supported in <span class="num">2<\/span> of 8 scenarios/);
  assert.match(html, /4\.947 pp below BASE/, 'the reasons are verbatim from the payload');
  assert.match(html, /17\.118 pp below BASE/);
  assert.match(html, /data-exp-label="VALID"/);
  assert.match(html, /narrow meaning/);
  assert.match(html, /negative result for H1/);
  assert.match(html, /trade-off failure/);
  assert.match(html, /not part of the live serving path/);
  // Lead decision (a): MEASURED displays as REAL with its qualifier; "Retained" is a separate neutral tag, not part of the word.
  assert.equal(count(html, 'data-retained="true"'), 2, 'one Retained tag on each result card');
  assert.match(html, /data-retained-legend/);
  assert.match(html, /<span class="prov__word">Real<\/span><span class="prov__q">loopback, one host<\/span>/);
  assert.doesNotMatch(html, /Measured|Retained: /);
  for (const k of ['Real', 'Simulated', 'Derived', 'Unavailable']) assert.ok(html.includes(`>${k}<`), k);
  lint(html, 'story');
});

test('story: without retained evidence it says so and shows no result cards; without detail it does not invent counts', () => {
  const empty = H.renderLab(h(H.Lab.StoryView, { evidence: evidenceListView({ items: [], indexed: 0 }), m6Detail: null, sbDetail: null }));
  assert.match(empty, /No retained results to summarise/);
  assert.doesNotMatch(empty, /data-verdict=/);
  assert.doesNotMatch(empty, /How to read the results together/);
  const noDetail = H.renderLab(h(H.Lab.StoryView, { evidence: evidenceListView(FX.evidenceIndex()), m6Detail: null, sbDetail: null }));
  assert.match(noDetail, /detailed counts did not load/);
  assert.doesNotMatch(noDetail, /data-story-m6-counts/);
  lint(empty, 'empty story');
  lint(noDetail, 'story without detail');
});

// ---------------------------------------------------------------- evidence

test('evidence list: registered and exploratory are separate groups; the sealed negative result and Scenario B sit side by side', () => {
  const html = H.renderLab(h(H.Lab.EvidenceListBody, { view: evidenceListView(FX.evidenceIndex()) }));
  assert.match(html, /Registered experiments/);
  assert.match(html, /Exploratory runs/);
  assert.equal(count(html, 'data-evidence-kind="m6_sealed_evaluation"'), 1);
  assert.equal(count(html, 'data-evidence-kind="scenario_b"'), 1);
  assert.equal(count(html, 'data-evidence-kind="m6_dry_run"'), 1);
  assert.equal(count(html, 'data-exploratory'), 1, 'only the dry run is tagged exploratory');
  assert.ok(html.indexOf('Exploratory runs') < html.indexOf('data-evidence-kind="m6_dry_run"'));
  assert.ok(html.indexOf('data-evidence-kind="m6_sealed_evaluation"') < html.indexOf('Exploratory runs'));
  assert.match(html, /Not supported/);
  assert.match(html, /Trade-off failure/);
  assert.match(html, /Not testable at the registered horizon/);
  assert.match(html, /Unverified/);
  assert.match(html, /read-only|Read-only/i);
  lint(html, 'evidence list');
});

test('evidence list: empty directory is an empty state, not an error and not a result', () => {
  const html = H.renderLab(h(H.Lab.EvidenceListBody, { view: evidenceListView({ items: [], indexed: 0 }) }));
  assert.match(html, /data-state-kind="empty"/);
  assert.match(html, /No retained experiments found/);
  assert.match(html, /This is not a result/);
  lint(html, 'empty evidence');
});

test('verdict chips: negative, inconclusive and positive use the same markup and size (only the word, glyph and tone differ)', () => {
  const chip = (o) => H.renderLab(h(H.Lab.VerdictChip, { outcome: o, name: 'X' }));
  const shape = (s) => s.replace(/data-verdict="[^"]*"/, '').replace(/data-verdict-kind="[^"]*"/, '').replace(/tone-\w+/, 'tone-T').replace(/<svg.*?<\/svg>/, '<svg/>').replace(/<span>[^<]*<\/span><\/span><\/span>$/, '</span></span>');
  const pos = chip('SUPPORTED');
  const neg = chip('NOT_SUPPORTED');
  const inc = chip('INCONCLUSIVE');
  assert.equal(shape(pos), shape(neg));
  assert.equal(shape(pos), shape(inc));
  assert.match(neg, /Not supported/);
  assert.match(inc, /Inconclusive/);
  assert.match(pos, /<svg[^>]*aria-hidden="true"/);
  assert.match(neg, /<svg[^>]*aria-hidden="true"/);
});

test('M6 sealed detail: simulation banner first, NOT_SUPPORTED as recorded, effect charts with a zero line, table alternatives, falsifiers, static limits labelled', () => {
  const d = evidenceDetailView(FX.m6Sealed());
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d }));
  assert.ok(html.indexOf('data-banner="simulated"') >= 0 && html.indexOf('data-banner="simulated"') < html.indexOf('id="m6-verdicts"'));
  assert.match(html, /NOT A MEASUREMENT of any network, host or process/);
  assert.match(html, /data-hypothesis="H1"[\s\S]*?data-verdict="NOT_SUPPORTED"/);
  assert.match(html, /data-hypothesis="H2"[\s\S]*?Not testable at the registered horizon/);
  assert.match(html, /data-hypothesis="H3"/);
  assert.equal(count(html, 'data-chart="effect"'), 2);
  assert.equal(count(html, 'lab-effect__zero'), 16, 'a zero line in each row of both charts');
  assert.match(html, /solid vertical line is 0 \(no change\)/);
  assert.match(html, /left of the line: adaptive arm worse/);
  assert.match(html, /right of the line: adaptive arm better/);
  assert.equal(count(html, 'data-effect-row='), 16);
  assert.match(html, /credibly worse/);
  assert.match(html, /practically null/);
  assert.match(html, />Table<\/button>/, 'Chart / Table toggle');
  assert.match(html, /data-scenario-row="O-HET"/);
  assert.match(html, /Falsifiers/);
  assert.match(html, /not mechanical/);
  assert.match(html, /From the research note \(static text, not part of the payload\)/);
  assert.match(html, /seed values are never shown/);
  assert.doesNotMatch(html, /Exploratory run/, 'the sealed result is not exploratory');
  assert.match(html, /data-tier="RETAINED_SIMULATED"/);
  assert.match(html, /data-tier="DERIVED"/);
  lint(html, 'm6 sealed');
});

test('M6 dry run: tagged exploratory with a warning that it is not the registered evaluation', () => {
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Dry()) }));
  assert.match(html, /class="lab-hint">Exploratory/);
  assert.match(html, /Exploratory run/);
  assert.match(html, /not the registered evaluation/);
  lint(html, 'm6 dry');
});

test('Scenario B evidence: the binding notice is shown in full, before the charts; the trade-off is two panels; unavailable charts are words', () => {
  const d = evidenceDetailView(FX.sbEvidence());
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d }));
  const noticeAt = html.indexOf('Interpretation notice (binding)');
  assert.ok(noticeAt >= 0);
  assert.ok(noticeAt < html.indexOf('data-chart="tradeoff"'), 'notice precedes the numbers');
  // V2A (DD v2 10.1): the answer first, "Not established" visible, then the binding notice, then the charts.
  assert.ok(html.indexOf('id="sb-glance"') < html.indexOf('id="sb-not-established"') && html.indexOf('id="sb-not-established"') < html.indexOf('id="sb-notice"'), 'glance, then Not established, then the binding notice');
  assert.match(html, /dated interpretation and correction notice/);
  assert.match(html, /Permitted conclusion/);
  assert.match(html, /Not established/);
  assert.match(html, /must accompany any citation/);
  assert.equal(count(html, 'data-verdict="TRADE_OFF_FAILURE"'), 2);
  assert.match(html, /Peak origin concurrency <span class="lab-hint">lower means less load on the origin/);
  assert.match(html, /Client success rate <span class="lab-hint">higher means fewer failed requests/);
  assert.equal(count(html, 'data-bar-row='), 2 * 3 + 3, 'three arms in each of the two trade-off panels plus the shed panel');
  assert.match(html, /data-unavailable-chart="Shed fraction per phase"/);
  assert.match(html, /data-unavailable-chart="Latency of successful \(HTTP 200\) responses only"/);
  assert.match(html, /per-phase shed fractions are not part of the evidence or summary payload/);
  assert.match(html, /include the fast-failing 503 responses/);
  assert.match(html, /Client latency, all requests/);
  assert.match(html, /data-paired-arm="STATIC-CAP"/);
  assert.match(html, /data-paired-arm="AIMD-CAP"/);
  assert.match(html, /data-paired-metric="client_success_rate_pp"/);
  assert.match(html, /Difference 95% interval/);
  assert.match(html, /data-limitations/);
  assert.match(html, /data-label-meaning/);
  assert.match(html, /Run class: VALID \(narrow meaning\)/);
  assert.match(html, /data-run-class-hint[^>]*>VALID describes how the run was set up[^<]*not a success verdict/);
  assert.match(html, /data-retained/);
  assert.match(html, /data-tier="RETAINED_MEASURED_LOOPBACK"/);
  assert.match(html, /<span class="prov__word">Real<\/span><span class="prov__q">loopback, one host/);
  assert.doesNotMatch(html, /Measured: |Retained: /);
  assert.match(html, /This is not a production, WAN or multi-host measurement/);
  lint(html, 'scenario b evidence');
});

test('Scenario B evidence: BASE "lowest cap seen" renders as Unavailable with a reason, never 0; null relative change likewise', () => {
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }));
  const baseRow = html.split('data-arm-row="BASE"')[1].split('</tr>')[0];
  assert.match(baseRow, /Unavailable/);
  assert.match(baseRow, /BASE runs without a cap/);
  assert.doesNotMatch(baseRow.split('Unavailable')[0].split('<td>').slice(-1)[0], /^<span class="num">0/);
  const shedRow = html.split('data-paired-metric="shed_fraction_pp"')[1].split('</tr>')[0];
  assert.match(shedRow, /Unavailable/);
});

test('Scenario B evidence: if the payload lacks the notice text the page says so loudly instead of staying silent', () => {
  const raw = FX.sbEvidence();
  delete raw.interpretation;
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(raw) }));
  assert.match(html, /The interpretation notice is missing from this payload/);
  assert.match(html, /role="alert"/);
  assert.doesNotMatch(html, /Interpretation notice \(binding\)/);
  lint(html, 'missing notice');
});

test('evidence detail: a folder with no result document is an unavailable state, not a blank page', () => {
  const raw = FX.m6Sealed();
  delete raw.recorded_result;
  raw.verdict_snapshot = { unavailable: true, reason: 'this retained artifact set records no verdict' };
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(raw) }));
  assert.match(html, /No result document in this experiment/);
  assert.match(html, /data-state-kind="unavailable"/);
  assert.doesNotMatch(html, /data-chart="effect"/);
  lint(html, 'no result');
});

// ---------------------------------------------------------------- run form

test('run form (simulation matrix): a pristine form shows no error and a neutral "still needed" line; long catalogues are collapsible groups with counts; the sealed seed set is never offered', () => {
  // I2: validation is by touched state (spec section 9). Replaces the old "disabled submit with 4 errors" expectation.
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps: caps(), onSubmit: noop, initialKind: 't1_matrix' }));
  assert.equal(count(html, 'data-field-error'), 0, 'no error is visible before the form is touched');
  assert.doesNotMatch(html, /data-error-summary|role="alert"|aria-invalid/);
  assert.match(html, /<button type="submit" class="btn btn--primary" aria-describedby="[^"]+">Start run<\/button>/, 'Start stays enabled; pressing it lists what is missing');
  assert.match(html, /Still needed: Scenarios, Workload families, Arms, Number of seeds\./);
  assert.match(html, /data-runspec-empty/);
  assert.match(html, /data-command-empty/);
  for (const n of [12, 7, 19]) assert.match(html, new RegExp(`none selected, ${n} available`), `a collapsible group with a count for ${n} choices`);
  assert.equal(count(html, '>Select all</button>'), 3);
  assert.equal(count(html, '>Clear</button>'), 3);
  assert.match(html, /data-preset="smallest"/);
  assert.doesNotMatch(html, /<details class="disclosure" open/, 'the long groups start closed so the form stays short');
  assert.doesNotMatch(html, /value="S"/);
  assert.doesNotMatch(html, /sealed.*\(\d+ seeds\)/i);
  assert.match(html, /Seed set/);
  assert.match(html, /type="radio"[^>]*value="T"/);
  assert.match(html, /type="radio"[^>]*value="D"/);
  assert.match(html, /Deterministic virtual-time simulation/);
  assert.match(html, /Real gateway processes on this machine/);
  assert.match(html, /data-tier="SIMULATED"/);
  assert.match(html, /data-tier="MEASURED_LOOPBACK"/);
  for (const o of ['O-DEG', 'W-Z', 'BL-0']) assert.ok(html.includes(`<span class="mono">${o}`), o);
  lint(html, 'run form');
});

test('run form: opens on Scenario B when it is available (else the matrix); fieldset groups What to run / Settings / Review; advanced options are one level deeper; a sticky action bar', () => {
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps: caps(), onSubmit: noop }));
  assert.match(html, /data-kind-option="scenario_b"[\s\S]*?checked=""/, 'Scenario B is the default kind');
  assert.ok(html.indexOf('data-kind-option="scenario_b"') < html.indexOf('data-kind-option="t1_matrix"'));
  for (const legend of ['What to run', 'Settings', 'Review']) assert.match(html, new RegExp(`<legend>${legend}</legend>`), legend);
  assert.match(html, /data-advanced/);
  assert.match(html, /<summary><span class="lab-choice-group__title">Advanced settings<\/span>/);
  assert.match(html, /data-preset="quick"/);
  assert.match(html, /data-preset="full"/);
  assert.match(html, /data-form-bar/);
  assert.match(html, /Starts real gateway processes on this machine; a run takes minutes/);
  const off = FX.caps();
  off.kinds.scenario_b.available = false;
  off.kinds.scenario_b.reason = 'x';
  const t1 = H.renderLab(h(H.Lab.RunFormBody, { caps: capabilitiesView(off), onSubmit: noop }));
  assert.match(t1, /data-kind-option="t1_matrix"[\s\S]*?checked=""/);
  assert.match(t1, /<legend>Experiment design<\/legend>/);
  assert.equal(count(html, 'data-field-error'), 0);
  lint(html, 'run form default');
  lint(t1, 'run form fallback kind');
});

test('run form: Scenario B variant has the documented limits in its hints, arm explanations, and a valid default that shows the exact request', () => {
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps: caps(), onSubmit: noop, initialKind: 'scenario_b' }));
  assert.match(html, /1 to 30/);
  assert.match(html, /1 to 60/);
  assert.match(html, /1 to 2000/);
  assert.match(html, /10 to 100000/);
  assert.match(html, /STATIC-CAP/);
  assert.match(html, /may settle near its minimum and then behave like a static cap/);
  assert.doesNotMatch(html, /aria-disabled/, 'defaults are valid and nothing blocks starting');
  assert.match(html, /data-runspec/);
  assert.match(html, /&quot;kind&quot;: &quot;scenario_b&quot;/);
  assert.match(html, /phantom-lab scenario-b --reps 10/);
  assert.match(html, /debug build/);
  assert.match(html, /--gateway PATH/);
  assert.match(html, /<summary><span class="lab-choice-group__title">Exact request and terminal command<\/span>/);
  lint(html, 'run form sb');
});

test('run form: when the gateway has an active run, or the kind is unavailable, the form says why the primary action is disabled; a server error is shown as an alert', () => {
  const c = FX.caps();
  c.active_run = { run_id: '20261030T034041Z-c78a73cf', kind: 'scenario_b', state: 'running' };
  const html = H.renderLab(h(H.Lab.RunFormBody, { caps: capabilitiesView(c), onSubmit: noop, initialKind: 'scenario_b', submitError: mapLabError({ status: 409, headers: {}, bodyJson: JSON.parse(FX.errors().run_active_409.body), bodyText: '' }) }));
  assert.match(html, /A run is already active \(20261030T034041Z-c78a73cf\)/);
  assert.match(html, /already active/);
  assert.match(html, /role="alert"/);
  assert.match(html, /<button type="submit" class="btn btn--primary" aria-disabled="true"/);
  assert.match(html, /data-submit-disabled="true"[^>]*><span class="disabled-reason">[\s\S]*?A run is already active/, 'a disabled primary action states why, next to the button');
  const off = FX.caps();
  off.kinds.scenario_b.available = false;
  off.kinds.scenario_b.reason = 'gateway executable not found';
  const html2 = H.renderLab(h(H.Lab.RunFormBody, { caps: capabilitiesView(off), onSubmit: noop, initialKind: 'scenario_b' }));
  assert.match(html2, /This kind of run is not available here/);
  assert.match(html2, /gateway executable not found/);
  assert.match(html2, /aria-disabled="true"/);
  const submitting = H.renderLab(h(H.Lab.RunFormBody, { caps: caps(), onSubmit: noop, initialKind: 'scenario_b', submitting: true }));
  assert.match(submitting, />Starting<\/button>/);
  assert.match(submitting, /aria-disabled="true" aria-busy="true"/);
  assert.match(submitting, /Waiting for the gateway to accept the run/);
  lint(html, 'active');
  lint(html2, 'unavailable');
  lint(submitting, 'submitting');
});

// ---------------------------------------------------------------- runs list and detail

test('runs list: every state has a chip with a glyph and a word; empty history is an empty state', () => {
  const list = runsListView(FX.runsList());
  const html = H.renderLab(h(H.Lab.RunsListBody, { view: list }));
  assert.match(html, /data-runs-table/);
  assert.equal(count(html, 'data-run-id='), 4);
  assert.match(html, /data-run-state="cancelled"/);
  assert.match(html, /data-run-state="completed"/);
  assert.match(html, /href="#\/lab\/runs\/20261030T034142Z-a4de24b6"/);
  assert.match(html, /Newest first/);
  lint(html, 'runs list');
  const none = H.renderLab(h(H.Lab.RunsListBody, { view: runsListView(FX.runsEmpty()) }));
  assert.match(none, /No runs yet/);
  assert.match(none, /data-state-kind="empty"/);
  assert.match(none, /not a failure/);
  lint(none, 'empty runs');
  const states = ['queued', 'running', 'cancelling', 'completed', 'failed', 'cancelled', 'interrupted'].map((s) => ({ ...FX.runT1(), run_id: `r-${s}`, state: s }));
  const all = H.renderLab(h(H.Lab.RunsListBody, { view: runsListView({ items: states, indexed: 7 }) }));
  for (const s of ['queued', 'running', 'cancelling', 'completed', 'failed', 'cancelled', 'interrupted']) assert.match(all, new RegExp(`data-run-state="${s}"`), s);
  const words = ['Queued', 'Running', 'Cancelling', 'Completed', 'Failed', 'Cancelled', 'Interrupted'];
  for (const w of words) assert.ok(all.includes(`<span>${w}</span>`), w);
  lint(all, 'all states');
});

test('run detail (running): sticky header, progress from events only, live event tail, small live status, and a result placeholder that does not invent a result', () => {
  const sse = FX.sseSb().split('id: 12\n')[0];
  const html = H.renderLab(detail({ run: FX.runSbRunning(), sse, status: 'live', stream: streamState('open') }));
  assert.match(html, /data-run-state="running"/);
  // I2: "Live updates: live" is a small status chip, not a banner; the banner exists only for the problem states.
  assert.match(html, /data-live-chip="live"/);
  assert.doesNotMatch(html, /data-stream-status=/);
  assert.doesNotMatch(html, /class="banner/);
  assert.match(html, /<progress value="1" max="2"/);
  assert.match(html, /arm STATIC-CAP/);
  assert.match(html, /phase starting/);
  assert.match(html, /data-progress-text="true">Run 2 of 2 in progress: STATIC-CAP/);
  assert.match(html, /data-progress-phase="true">Phase 1 of 4|data-progress-phase="true">Phase: Starting the gateway/);
  assert.match(html, />Cancel run<\/button>/);
  assert.match(html, /data-result-waiting/);
  assert.doesNotMatch(html, /data-chart=/, 'no charts before a result exists');
  assert.match(html, /How to read the result when it arrives/);
  // last 5 events are always visible; the full list is one disclosure away
  assert.match(html, /data-event-tail/);
  assert.match(html, /aria-label="Latest events, oldest first" aria-live="off"/);
  assert.match(html, /Show all 11 events received/);
  assert.equal(count(html, 'data-run-status-live'), 1, 'one polite live region for the run status');
  lint(html, 'running');
});

test('run detail: elapsed time is DERIVED from the gateway timestamps (labelled) or UNAVAILABLE with the reason; never invented', () => {
  const run = FX.runSbRunning();
  const started = Date.parse(run.started_utc);
  assert.ok(Number.isFinite(started), 'fixture has a start time');
  const at = (ms) => H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView(run), live: { status: 'live', stream: streamState('open'), model: initialRunModel }, artifacts: { state: 'loading', rows: [] }, summary: { state: 'none' },
    cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null, nowMs: ms,
  }));
  const html = at(started + 72000);
  assert.match(html, /data-elapsed="true"/);
  assert.match(html, /<span class="num">1 min 12 s<\/span><span class="prov prov--derived"[^>]*>[\s\S]*?<span class="prov__q">from timestamps<\/span>/);
  assert.match(html, /browser clock/);
  const before = at(started - 5000);
  assert.match(before, /data-elapsed="unavailable"/, 'a negative span is unavailable, not zero');
  assert.match(before, /timestamps disagree/);
  const noStart = H.renderLab(h(H.Lab.RunDetailBody, {
    run: runView({ ...run, started_utc: null }), live: { status: 'live', stream: streamState('open'), model: initialRunModel }, artifacts: { state: 'loading', rows: [] }, summary: { state: 'none' },
    cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null, nowMs: started,
  }));
  assert.match(noStart, /data-elapsed="unavailable"/);
  assert.match(noStart, /data-provenance="unavailable"/);
  assert.match(noStart, /has not reported a start time/);
  const done = H.renderLab(detail({ run: FX.runSbFinal(), sse: FX.sseSb(), status: 'ended', stream: streamState('ended') }));
  assert.match(done, /data-elapsed="true"/, 'a finished run uses its own start and end timestamps');
  assert.match(done, /from timestamps/);
});

test('run detail: cancel opens a modal dialog (L3) with the safe choice first; sending state; then a "requested" notice; no cancel control on finished runs', () => {
  const base = { run: FX.runSbRunning(), sse: FX.sseSb().split('id: 12\n')[0] };
  const idle = H.renderLab(detail(base));
  assert.doesNotMatch(idle, /<dialog/, 'no dialog until the person asks to cancel');
  const confirm = H.renderLab(detail({ ...base, cancel: { phase: 'confirm', error: null } }));
  assert.match(confirm, /<dialog class="dialog lab-dialog" aria-labelledby="[^"]+" aria-describedby="[^"]+" data-cancel-dialog="true">/, 'a native <dialog>, opened modally by the component');
  assert.match(confirm, /Cancel this run\?/);
  assert.match(confirm, /Yes, cancel the run/);
  assert.match(confirm, /Keep running/);
  assert.ok(confirm.indexOf('Keep running') < confirm.indexOf('Yes, cancel the run'), 'the safe choice comes first');
  assert.equal(count(confirm, 'aria-haspopup="dialog"'), 1);
  const sending = H.renderLab(detail({ ...base, cancel: { phase: 'sending', error: null } }));
  assert.match(sending, /Cancelling<\/button>/);
  assert.match(sending, /disabled=""/);
  const sent = H.renderLab(detail({ ...base, cancel: { phase: 'sent', error: null } }));
  assert.match(sent, /Cancel requested/);
  assert.doesNotMatch(sent, />Cancel run<\/button>/);
  assert.doesNotMatch(sent, /<dialog/);
  const failed = H.renderLab(detail({ ...base, cancel: { phase: 'error', error: mapLabError({ status: 403, headers: { 'x-auth-error': 'InsufficientPermissions' }, bodyJson: null, bodyText: '' }) } }));
  assert.match(failed, /does not have the required role/);
  const finished = H.renderLab(detail({ run: FX.runSbFinal(), sse: FX.sseSb(), status: 'ended', stream: streamState('ended') }));
  assert.doesNotMatch(finished, /Cancel run/);
  const cancelling = H.renderLab(detail({ run: { ...FX.runSbRunning(), state: 'cancelling' } }));
  assert.match(cancelling, /Cancel requested/);
  for (const html of [idle, confirm, sending, sent, failed, finished, cancelling]) lint(html, 'cancel states');
});

test('run detail: stream states (stale, reconnecting, disconnected, failed) are stated in words with a reconnect button and the resume point', () => {
  const base = { run: FX.runSbRunning(), sse: FX.sseSb().split('id: 12\n')[0] };
  const cases = [
    ['stale', streamState('open'), /Live updates: Stale/, /Silent for longer than the gateway heartbeat/],
    ['reconnecting', streamState('closed'), /Live updates: Reconnecting/, /lost the connection and is trying again/],
    ['disconnected', { ...streamState('closed'), attempt: 4 }, /Live updates: Disconnected/, /no answer from the gateway/],
    ['failed', streamState('failed'), /Live updates: Unauthorised/, /gateway answered and refused/],
  ];
  for (const [status, stream, title, detailRe] of cases) {
    const html = H.renderLab(detail({ ...base, status, stream }));
    assert.match(html, new RegExp(`data-stream-status="${status}"`), status);
    assert.match(html, title, status);
    assert.match(html, detailRe, status);
    // V2A: one retry verb (linkState RETRY_LABEL). A refusal (failed) is not fixed by retrying, so it offers none.
    if (status === 'failed') assert.doesNotMatch(html, /Retry now/, status);
    else assert.match(html, /Retry now/, status);
    assert.doesNotMatch(html, /Reconnect now/, status);
    assert.match(html, /Last event received: number 1/, status);
    lint(html, status);
  }
  const ended = H.renderLab(detail({ ...base, status: 'ended', stream: streamState('ended') }));
  assert.doesNotMatch(ended, /Reconnect now|Retry now/);
  const gap = H.renderLab(detail({ ...base, stream: { ...streamState('open'), gaps: 2 } }));
  assert.match(gap, /2 gap\(s\) in event numbers/);
});

test('run detail (cancelled): flagged partial, no verdict, unavailable cells in words, exit code from the gateway', () => {
  const partial = FX.sbSummary();
  partial.status = 'CANCELLED';
  for (const arm of Object.values(partial.arms)) for (const k of Object.keys(arm.per_run_mean)) arm.per_run_mean[k] = null;
  partial.verdicts = {};
  partial.paired_vs_base = {};
  const html = H.renderLab(detail({
    run: FX.runCancelled(), sse: FX.sseCancelled(), status: 'ended', stream: streamState('ended'),
    summary: { state: 'ready', view: scenarioBView(partial) },
    artifacts: { state: 'ready', rows: artifactsView(FX.artifactsSb()) },
  }));
  assert.match(html, /data-run-state="cancelled"/);
  assert.match(html, /This run was cancelled/);
  // V2A (U-01): a cancelled run shows no verdict; its raw numbers sit in a closed "not a result" disclosure.
  assert.match(html, /data-no-verdict/);
  assert.match(html, /No verdict: the run cancelled before finishing/);
  assert.match(html, /<details class="disclosure lab-partial" id="sb-partial" data-partial-diagnostics="true"><summary>[\s\S]*?Partial diagnostic data \(not a result\)/);
  assert.match(html, /Exit code<\/dt><dd>130/);
  assert.match(html, /Unavailable/);
  assert.doesNotMatch(html, /data-verdict=/);
  lint(html, 'cancelled');
});

test('run detail (interrupted): says the server wrote the marks after a restart, nothing resumed, no result shown', () => {
  const run = { ...FX.runCancelled(), state: 'interrupted', exit_code: null, error: { code: 'interrupted', message: 'the gateway process stopped or restarted while this run was active; the run was not resumed and has no result' } };
  delete run.artifact_index;
  const sse = 'id: 1\nevent: state\ndata: {"state":"queued"}\n\nid: 2\nevent: state\ndata: {"source":"server_recovery_on_startup","state":"interrupted"}\n\nid: 3\nevent: done\ndata: {"source":"server_recovery_on_startup","state":"interrupted"}\n\n';
  const html = H.renderLab(detail({ run, sse, status: 'ended', stream: streamState('ended') }));
  assert.match(html, /data-run-state="interrupted"/);
  assert.match(html, /Interrupted by a gateway restart/);
  assert.match(html, /not resumed/);
  assert.match(html, /No verdict: the run was interrupted before finishing/);
  assert.match(html, /No summary was read for this run/);
  assert.doesNotMatch(html, /data-chart=|data-verdict=/);
  lint(html, 'interrupted');
});

test('run detail (failed): the gateway error is shown as an alert; a failure is never presented as completed', () => {
  const run = { ...FX.runSbFinal(), state: 'failed', exit_code: 1, error: { code: 'runner_failed', message: 'gateway exited early' } };
  delete run.artifact_index;
  const html = H.renderLab(detail({ run, status: 'ended', stream: streamState('ended') }));
  assert.match(html, /data-run-state="failed"/);
  assert.match(html, /The run failed \(runner_failed\)/);
  assert.match(html, /gateway exited early/);
  assert.match(html, /role="alert"/);
  assert.doesNotMatch(html, /data-run-state="completed"/);
  lint(html, 'failed');
});

test('run detail (completed Scenario B): the summary is fetched with credentials and rendered with arms side by side, paired differences and the mechanical verdict', () => {
  const html = H.renderLab(detail({
    run: FX.runSbFinal(), sse: FX.sseSb(), status: 'ended', stream: streamState('ended'),
    summary: { state: 'ready', view: scenarioBView(FX.sbSummary()) },
    artifacts: { state: 'ready', rows: artifactsView(FX.artifactsSb()) },
  }));
  assert.match(html, /data-run-state="completed"/);
  assert.match(html, /Run class: EXPLORATORY/);
  assert.match(html, /Why the label is not VALID/);
  assert.match(html, /DEBUG profile/);
  assert.match(html, /data-verdict="INCONCLUSIVE"/);
  assert.match(html, /only 1 valid paired runs; 10 required/);
  assert.match(html, /data-arm-row="BASE"/);
  assert.match(html, /data-arm-row="STATIC-CAP"/);
  assert.match(html, /data-paired-arm="STATIC-CAP"/);
  assert.match(html, /data-tier="MEASURED_LOOPBACK"/);
  assert.match(html, /data-artifacts-table/);
  assert.match(html, /data-artifact="summary.json"/);
  assert.match(html, /live file; not hashed/);
  assert.match(html, /aria-label="Download summary.json"/);
  assert.equal(count(html, 'aria-label="Download '), 4);
  assert.match(html, /Downloads use your credentials in a request header, never in the address/);
  assert.match(html, /Settings used/);
  assert.match(html, /Static cap<\/dt><dd>24/);
  assert.match(html, /Event log/);
  lint(html, 'completed sb');
});

test('run detail (completed T1): a cell count and the hash of cells.csv, an explicit statement that no statistics are computed, and the download', () => {
  const html = H.renderLab(detail({
    run: FX.runT1(), sse: FX.sseT1(), status: 'ended', stream: streamState('ended'),
    artifacts: { state: 'ready', rows: artifactsView(FX.artifactsT1()) },
  }));
  assert.match(html, /data-tier="SIMULATED"/);
  assert.match(html, /Cells run<\/dt><dd><span class="num">4<\/span>/);
  assert.match(html, /0ea1926af10bb76e897ac2f666610829fbc03dbbccb2e7cb3f6edf13ca343811/);
  assert.match(html, /No summary statistics for matrix runs/);
  assert.match(html, /aria-label="Download cells.csv"/);
  assert.doesNotMatch(html, /data-verdict=|data-chart=/, 'no verdict is invented for a matrix run');
  lint(html, 'completed t1');
});

test('run detail: artifact loading, error and empty states; a download error is shown; the unhashed live files are unavailable with the reason', () => {
  const loading = H.renderLab(detail({ run: FX.runT1(), artifacts: { state: 'loading', rows: [] } }));
  assert.match(loading, /Loading the artifact list/);
  const err = H.renderLab(detail({ run: FX.runT1(), artifacts: { state: 'error', error: mapLabError({ status: 403, headers: { 'x-auth-error': 'InsufficientPermissions' }, bodyJson: null, bodyText: '' }) } }));
  assert.match(err, /does not have the required role/);
  const none = H.renderLab(detail({ run: FX.runT1(), artifacts: { state: 'ready', rows: [] } }));
  assert.match(none, /data-no-artifacts/);
  const dl = H.renderLab(detail({ run: FX.runT1(), artifacts: { state: 'ready', rows: artifactsView(FX.artifactsT1()) }, downloading: 'cells.csv', downloadError: mapLabError({ status: 413, headers: {}, bodyJson: null, bodyText: '' }) }));
  assert.match(dl, /Downloading<\/button>/);
  assert.match(dl, /exceeded a size limit/);
  const live = loading.includes('events.jsonl') ? loading : H.renderLab(detail({ run: FX.runT1(), artifacts: { state: 'ready', rows: artifactsView(FX.artifactsT1()) } }));
  assert.match(live, /data-artifact="run.json"[\s\S]*?Unavailable/);
  for (const html of [loading, err, none, dl, live]) lint(html, 'artifact states');
});

test('charts: every chart is a ChartFrame with a Chart / Table toggle and a captioned table, and bars are aria-hidden geometry', () => {
  // I2: charts moved into the shared ChartFrame (toggle instead of a closed "View as table" details).
  const html = H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }))
    + H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) }));
  const figures = html.split('<figure').length - 1;
  assert.equal(figures, 5);
  assert.equal(count(html, 'class="chart-frame"'), figures);
  assert.equal(count(html, '>Table</button>'), figures);
  assert.equal(count(html, 'class="chart-frame__axes"'), figures, 'every chart has a full-width axis caption');
  assert.ok(count(html, 'class="lab-bars__bar" aria-hidden="true"') + count(html, 'class="lab-effect__track" aria-hidden="true"') > 0);
  assert.doesNotMatch(html, /<svg[^>]*role="img"/, 'no unlabeled graphics; charts are text first');
});

test('no inline colours, no event handlers in markup, no external resources in any Lab markup', () => {
  const pages = [
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) })),
    H.renderLab(h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) })),
    H.renderLab(h(H.Lab.RunFormBody, { caps: caps(), onSubmit: noop })),
    H.renderLab(detail({ run: FX.runSbFinal(), sse: FX.sseSb(), summary: { state: 'ready', view: scenarioBView(FX.sbSummary()) } })),
  ];
  for (const html of pages) {
    assert.doesNotMatch(html, /style="[^"]*(color|background)/i);
    assert.doesNotMatch(html, /\son[a-z]+="|javascript:|<script|<iframe|<img|https?:\/\/(?!localhost|127\.0\.0\.1)/);
  }
});
