import React from 'react';
import ScreenFrame, { Section } from './ScreenFrame.jsx';
import Placeholder from './Placeholder.jsx';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import { StateBlock } from '../components/States.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { clusterView } from '../lib/clusterView.js';

/** One value with the provenance the gateway stated, or the word Unavailable and the reason. Never a bare number. */
function Cell({ cell, id }) {
  if (cell.text === null) {
    return (
      <>
        <ProvenanceBadge src="unavailable" />
        <span className="not-reported__reason"> {cell.reason}</span>
      </>
    );
  }
  return (
    <>
      <span className="mono" data-cluster-value={id}>{cell.text}</span> <ProvenanceBadge src={cell.src} />
    </>
  );
}

function CounterTable({ caption, rows, idPrefix }) {
  return (
    <div className="table-wrap">
      <table>
        <caption>{caption}</caption>
        <thead>
          <tr>
            <th scope="col">Counter</th>
            <th scope="col">Value</th>
            <th scope="col">Meaning</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.id} data-cluster-row={`${idPrefix}-${r.id}`}>
              <th scope="row">{r.label}</th>
              <td><Cell cell={r.cell} id={`${idPrefix}-${r.id}`} /></td>
              <td>{r.meaning}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/**
 * Cluster page (WP11, batch B2). Everything comes from `metrics.cluster` and `metrics.outcomes` of the telemetry frame of THIS
 * gateway; nothing is computed or invented here. When the gateway reports no cluster block the page is the honest capability
 * statement (Placeholder), never a made-up cluster.
 */
export default function Cluster() {
  const { payload } = useTelemetryContext();
  const view = clusterView(payload);
  if (!view.present) return <Placeholder screenId="cluster" />;

  return (
    <ScreenFrame
      screenId="cluster"
      title="Cluster"
      intro="How this gateway sees the other members, how its cache requests were answered, and whether its purge messages are getting through. One gateway's view, not a global one."
    >
      <Section
        title="Members"
        note="Routing state is the gateway's membership state machine, not a liveness verdict: a member that is down can still be listed as recovering. Reachability is inferred from the outcome of the gateway's recent connection and message attempts (three or more failures in a row means unreachable)."
      >
        <div className="table-wrap">
          <table data-cluster-members>
            <caption>Members as seen from this gateway</caption>
            <thead>
              <tr>
                <th scope="col">Node</th>
                <th scope="col">Routing state</th>
                <th scope="col">Reachability</th>
                <th scope="col">Failed attempts in a row</th>
                <th scope="col">Idle connections</th>
                <th scope="col">Open connections (idle plus in use)</th>
              </tr>
            </thead>
            <tbody>
              <tr data-cluster-row="local">
                <th scope="row">Node {view.local.available ? view.local.id : 'unknown'} (this gateway)</th>
                <td>{view.local.available ? <Cell cell={view.local.state} id="local-state" /> : <><ProvenanceBadge src="unavailable" /><span className="not-reported__reason"> {view.local.reason}</span></>}</td>
                <td colSpan={4}>Not applicable to the gateway itself.</td>
              </tr>
              {view.peers.map((p) => (
                <tr key={p.key} data-cluster-row={`peer-${p.id}`}>
                  <th scope="row">Node {p.id}</th>
                  <td><Cell cell={p.state} id={`peer-${p.id}-state`} /></td>
                  <td><Cell cell={p.reachability} id={`peer-${p.id}-reachability`} /></td>
                  <td><Cell cell={p.failedAttempts} id={`peer-${p.id}-failed`} /></td>
                  <td><Cell cell={p.idle} id={`peer-${p.id}-idle`} /></td>
                  <td><Cell cell={p.active} id={`peer-${p.id}-active`} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {view.peers.length === 0 ? <p className="section__note">The gateway reports no other members.</p> : null}
        {view.truncated ? <p className="section__note">More members are reported than are listed here.</p> : null}
      </Section>

      <Section
        title="How cache requests were answered"
        note="Counted by this gateway since it started, one per response, in the same words as the X-Cache-Source response header. A peer fill is still reported as a miss in X-Cache-Status."
      >
        <CounterTable caption="Responses by source" rows={view.outcomes} idPrefix="outcome" />
        <CounterTable caption="Peer reads that did not produce a response from a peer" rows={view.peerReads} idPrefix="peerread" />
      </Section>

      <Section
        title="Purge messages to the other members"
        note="What happened to the invalidations this gateway sent after a DELETE. A non-zero Gave up means a member may still hold an entry that was deleted."
      >
        <CounterTable caption="Purge fan-out counters" rows={view.broadcaster} idPrefix="fanout" />
      </Section>

      {view.mode !== 'live' ? (
        <StateBlock kind="warning" title="Not a live gateway">
          The gateway reports a mode other than live; the values above are labelled as the gateway labels them.
        </StateBlock>
      ) : null}
    </ScreenFrame>
  );
}
