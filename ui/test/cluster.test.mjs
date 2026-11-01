// Cluster screen (batch B2, WP11): view model and rendered page.
// The fixtures cluster_healthy.json / cluster_node3_dead.json are REAL /api/v1/metrics responses captured from gateway 1 of a
// three-gateway mTLS cluster started with scripts/cluster_launcher.mjs (release build, loopback, 2026-10-31); in the second one
// node 3 had been stopped and six DELETEs issued. They are test data only and never ship in the bundle.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { getHarness } from '../scripts/render-harness.mjs';
import { lintHtml } from '../scripts/a11y-lint.mjs';
import { livePayload, makeCtx } from './helpers/fixtures.mjs';
import { clusterView, labelCell, numberCell } from '../src/lib/clusterView.js';
import { hrefFor, screenById, footerFor } from '../src/lib/routes.js';

const fixture = (name) => JSON.parse(fs.readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8'));
let H;
before(async () => {
  H = await getHarness();
});
const render = (payload) => H.renderScreen('cluster', makeCtx({ frames: [payload] }));

test('route: Cluster is a data screen in the live group and the footer says so', () => {
  const s = screenById('cluster');
  assert.equal(s.kind, 'data');
  assert.equal(s.group, 'Live gateway');
  assert.equal(hrefFor('cluster'), '#/cluster');
  assert.equal(footerFor({ id: 'cluster' }), footerFor({ id: 'traffic' }));
});

test('view model: a real healthy frame yields local, remote peers only, outcomes and broadcaster counters, each with its stated provenance', () => {
  const v = clusterView(fixture('cluster_healthy.json'));
  assert.equal(v.present, true);
  assert.equal(v.local.available, true);
  assert.equal(v.local.state.text, 'Active');
  assert.equal(v.local.state.src, 'measured');
  assert.deepEqual(v.peers.map((p) => p.id), ['2', '3'], 'the local node is not listed as a peer');
  for (const p of v.peers) {
    assert.equal(p.state.text, 'Active');
    assert.equal(p.reachability.text, 'reachable');
    assert.equal(p.reachability.src, 'inferred', 'reachability is inferred, never measured');
    assert.equal(p.failedAttempts.src, 'measured');
  }
  assert.deepEqual(v.outcomes.map((o) => o.id), ['local', 'peer', 'origin', 'coalesced', 'stale']);
  assert.ok(v.outcomes.every((o) => o.cell.src === 'measured' && o.cell.text !== null));
  assert.deepEqual(v.broadcaster.map((o) => o.id), ['enqueued', 'sent', 'retried', 'acked', 'dropped', 'exhausted']);
});

test('view model: a dead peer is shown as recovering AND unreachable, with its failed-attempt count', () => {
  const v = clusterView(fixture('cluster_node3_dead.json'));
  const n3 = v.peers.find((p) => p.id === '3');
  const n2 = v.peers.find((p) => p.id === '2');
  assert.equal(n3.state.text, 'Recovering');
  assert.equal(n3.reachability.text, 'unreachable');
  assert.ok(Number(n3.failedAttempts.text.replace(/,/g, '')) >= 3);
  assert.equal(n2.reachability.text, 'reachable');
  assert.equal(v.broadcaster.find((b) => b.id === 'exhausted').cell.text !== '0', true, 'lost deliveries are visible');
});

test('view model: no cluster block (single node, older gateway) is "not present", so the screen falls back to the capability statement', () => {
  assert.deepEqual(clusterView(livePayload()), { present: false });
  assert.deepEqual(clusterView(null), { present: false });
  const p = livePayload();
  p.metrics.cluster = { src: 'unavailable', reason: 'cluster_not_configured' };
  assert.deepEqual(clusterView(p), { present: false });
});

test('view model: invalid values are withheld, never turned into zero or shown with a borrowed provenance', () => {
  assert.deepEqual(numberCell({ src: 'measured', v: 'seven', unit: 'count' }, 'live').text, null);
  assert.deepEqual(numberCell({ src: 'measured', v: Number.NaN, unit: 'count' }, 'live').text, null);
  assert.equal(numberCell({ src: 'simulated', v: 3, unit: 'count' }, 'live').text, null, 'simulated is rejected in live mode');
  assert.equal(numberCell(undefined, 'live').src, 'unavailable');
  assert.equal(numberCell({ src: 'unavailable', reason: 'not_measured' }, 'live').text, null);
  assert.equal(labelCell({ src: 'measured', v: 42 }, 'live').text, null, 'a number is not a state label');
  assert.equal(labelCell({ src: 'measured', v: '' }, 'live').text, null);
  assert.equal(labelCell({ src: 'measured', v: 'x'.repeat(200) }, 'live').text, null);
  assert.equal(labelCell({ src: 'invented', v: 'Active' }, 'live').src, 'unavailable');
  assert.equal(labelCell({ src: 'unavailable', v: 'Active' }, 'live').text, null, 'unavailable that carries a value is rejected');
  assert.equal(labelCell({ src: 'inferred', v: 'unreachable' }, 'live').src, 'inferred');
  // a missing outcomes block makes the outcome rows unavailable, not zero
  const p = fixture('cluster_healthy.json');
  delete p.metrics.outcomes;
  const v = clusterView(p);
  assert.ok(v.outcomes.every((o) => o.cell.text === null && o.cell.src === 'unavailable'));
});

test('page: every value is next to a provenance badge; reachability is drawn as Derived (inferred); no panels; accessible markup', () => {
  const html = render(fixture('cluster_node3_dead.json'));
  assert.match(html, /<h1[^>]*>Cluster<\/h1>/);
  assert.match(html, /<p class="page-head__group">Live gateway<\/p>/);
  assert.match(html, /data-cluster-members/);
  assert.match(html, /Node 3<\/th>/);
  assert.match(html, /data-cluster-value="peer-3-state"[^>]*>Recovering<\/span> <span class="prov prov--measured"/);
  assert.match(html, /data-cluster-value="peer-3-reachability"[^>]*>unreachable<\/span> <span class="prov prov--inferred"/);
  assert.match(html, /data-provenance="inferred"/);
  // each drawn value (data-cluster-value) is immediately followed by a badge
  const values = html.match(/data-cluster-value="[^"]+"/g) || [];
  const badged = html.match(/data-cluster-value="[^"]+"[^>]*>[^<]*<\/span> <span class="prov /g) || [];
  assert.ok(values.length >= 26, `expected many values, got ${values.length}`);
  assert.equal(badged.length, values.length, 'every value has a provenance badge');
  assert.doesNotMatch(html, /class="panel"/);
  assert.match(html, /How cache requests were answered/);
  assert.match(html, /Purge messages to the other members/);
  assert.match(html, /X-Cache-Source/);
  assert.deepEqual(lintHtml(html, { requireH1: true }), []);
});

test('page: the outcome rows use the X-Cache-Source words and show the counters of the frame', () => {
  const p = fixture('cluster_healthy.json');
  const html = render(p);
  for (const id of ['local', 'peer', 'origin', 'coalesced', 'stale']) {
    const expected = p.metrics.outcomes.responses[id].v.toLocaleString('en-US');
    assert.match(html, new RegExp(`data-cluster-value="outcome-${id}"[^>]*>${expected}</span>`), id);
  }
  assert.match(html, /data-cluster-value="fanout-exhausted"[^>]*>0<\/span>/);
});

test('page: unavailable values say Unavailable with the reason and never draw a number', () => {
  const p = fixture('cluster_healthy.json');
  p.metrics.cluster.peers[0].idle_connections = { src: 'unavailable', reason: 'not_measured', unit: 'count' };
  p.metrics.outcomes.responses.peer = { src: 'measured', v: 'many', unit: 'count' };
  const html = render(p);
  assert.doesNotMatch(html, /data-cluster-value="peer-2-idle"/);
  assert.doesNotMatch(html, /data-cluster-value="outcome-peer"/);
  assert.match(html, /Not measured by the gateway/);
  assert.match(html, /Rejected: value is not a finite number or boolean/);
  assert.match(html, /data-provenance="unavailable"/);
});

test('page: a gateway without a cluster block keeps the capability statement (no invented cluster)', () => {
  const html = render(livePayload());
  assert.match(html, /Cluster mode is not enabled/);
  assert.doesNotMatch(html, /data-cluster-members/);
  assert.equal((html.match(/data-metric=/g) || []).length, 0);
});

test('page: before any frame arrives the page is loading, not empty', () => {
  const html = H.renderScreen('cluster', makeCtx({}));
  assert.match(html, /data-state-kind="loading"/);
});
