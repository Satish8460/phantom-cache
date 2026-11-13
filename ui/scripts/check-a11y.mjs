// Runs the static accessibility lint over every screen in every data state (server-side render, no browser).
// Usage: node scripts/check-a11y.mjs        Exit code 1 on any issue.
import { getHarness } from './render-harness.mjs';
import { lintHtml } from './a11y-lint.mjs';
import { livePayload, makeCtx } from '../test/helpers/fixtures.mjs';

const H = await getHarness();
const degraded = livePayload();
degraded.metrics.cache_hits = { src: 'simulated', v: 1 };
const noOrigin = livePayload();
noOrigin.metrics.origin = { src: 'unavailable', reason: 'origin_not_configured' };
const cluster = livePayload();
cluster.capabilities.cluster = 'experimental';
const states = {
  loading: makeCtx({}),
  disconnected: makeCtx({ phase: 'closed' }),
  error: makeCtx({ frames: ['{"schema":1}'] }),
  live: makeCtx({ frames: [livePayload()] }),
  stale: makeCtx({ frames: [livePayload()], receivedAt: 0, now: 60_000 }),
  degraded: makeCtx({ frames: [degraded] }),
  'origin unavailable': makeCtx({ frames: [noOrigin] }),
  'cluster experimental': makeCtx({ frames: [cluster] }),
};

let pages = 0;
let issues = 0;
for (const id of H.SCREEN_IDS.filter((s) => s !== 'simulation')) {
  for (const [name, ctx] of Object.entries(states)) {
    pages += 1;
    for (const i of lintHtml(H.renderScreen(id, ctx))) {
      issues += 1;
      console.error(`[a11y] ${id} / ${name}: ${i.rule}: ${i.detail}`);
    }
  }
}

// Phantom Lab bodies from REAL captured API responses (test/fixtures/lab), in the states the screens can reach.
import { FX } from '../test/helpers/labFixtures.mjs';
import { capabilitiesView, artifactsView, evidenceDetailView, evidenceListView, runView, runsListView, scenarioBView } from '../src/lib/labView.js';
import { initialRunModel, initialStream } from '../src/lib/labStream.js';
const h = H.React.createElement;
const noop = () => {};
const capsV = capabilitiesView(FX.caps());
const live = { status: 'live', stream: initialStream, model: initialRunModel };
const labPages = {
  'lab evidence list': h(H.Lab.EvidenceListBody, { view: evidenceListView(FX.evidenceIndex()) }),
  'lab evidence empty': h(H.Lab.EvidenceListBody, { view: evidenceListView({ items: [], indexed: 0 }) }),
  'lab m6 sealed': h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.m6Sealed()) }),
  'lab scenario b evidence': h(H.Lab.EvidenceDetailBody, { d: evidenceDetailView(FX.sbEvidence()) }),
  'lab story': h(H.Lab.StoryView, { evidence: evidenceListView(FX.evidenceIndex()), m6Detail: evidenceDetailView(FX.m6Sealed()).m6, sbDetail: evidenceDetailView(FX.sbEvidence()).sb }),
  'lab run form t1': h(H.Lab.RunFormBody, { caps: capsV, onSubmit: noop }),
  'lab run form sb': h(H.Lab.RunFormBody, { caps: capsV, onSubmit: noop, initialKind: 'scenario_b' }),
  'lab runs list': h(H.Lab.RunsListBody, { view: runsListView(FX.runsList()) }),
  'lab run detail': h(H.Lab.RunDetailBody, { run: runView(FX.runSbFinal()), live, artifacts: { state: 'ready', rows: artifactsView(FX.artifactsSb()) }, summary: { state: 'ready', view: scenarioBView(FX.sbSummary()) }, cancel: { phase: 'idle', error: null }, onCancelStart: noop, onCancelConfirm: noop, onCancelAbort: noop, onReconnect: noop, onRefresh: noop, onDownload: noop, downloading: null, downloadError: null }),
  'lab connection': h(H.Lab.ConnectionPanel, { res: { status: 'ready', data: FX.caps(), error: null, reload: noop }, view: capsV, hasToken: false, onToken: noop, onClear: noop, gatewayUrl: 'http://127.0.0.1:18085' }),
};
for (const [name, el] of Object.entries(labPages)) {
  pages += 1;
  for (const i of lintHtml(H.renderLab(el), { requireH1: false })) {
    issues += 1;
    console.error(`[a11y] ${name}: ${i.rule}: ${i.detail}`);
  }
}
console.log(`[check-a11y] ${pages} screen/state combinations linted, ${issues} issue(s). (Static checks only; see docs/design/ui_audit.md section 5.)`);
process.exit(issues ? 1 : 0);
