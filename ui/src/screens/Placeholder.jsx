import React from 'react';
import ScreenFrame, { Section } from './ScreenFrame.jsx';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import { StateBlock, StatusPill } from '../components/States.jsx';
import Illustration, { Arrow, ArrowDefs, Box } from './Illustration.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { capabilityView, nodeViews } from '../lib/viewmodel.js';
import { hrefFor, labHref } from '../lib/routes.js';
import { clip } from '../lib/format.js';

// Per page: the one-line purpose, what it would show once the gateway reports it, and what exists today instead. No numbers.
const COPY = {
  cluster: {
    title: 'Cluster',
    intro: 'Node membership, purge fan-out and recovery. The gateway does not report these yet.',
    wouldShow: ['Node membership', 'Purge fan-out status', 'Recovery after a node returns'],
    instead: [
      { href: hrefFor('overview'), text: 'Overview, Gateway: whether this gateway reports cluster mode at all.' },
      { href: labHref('evidence'), text: 'Phantom Lab, Evidence: retained experiment results, kept apart from live telemetry.' },
    ],
  },
  intelligence: {
    title: 'Phantom Intelligence',
    intro: 'Autonomous controller decisions, policy changes and safety gates. The gateway does not report these yet.',
    wouldShow: ['Current controller policy and tier', 'Model decisions (propose, canary, commit, rollback)', 'Audit log of autonomous actions taken'],
    instead: [
      { href: hrefFor('overview'), text: 'Overview, Gateway: whether this gateway reports controller mode at all.' },
      { href: labHref('evidence'), text: 'Phantom Lab, Evidence: retained experiment results, kept apart from live telemetry.' },
    ],
  },
};

/**
 * Derives the honest statement for a placeholder screen from gateway capabilities.
 *
 * Ladder:
 *   - no frame yet: Loading
 *   - capability says not_enabled: "Cluster mode is not enabled"
 *   - capability says enabled but no contract: "Cluster mode is enabled, but no cluster telemetry contract exists in schema 2"
 *   - capability unknown: "Unrecognised cluster capability: <value>"
 *   - capability not reported: "The gateway does not report cluster capability in schema 2"
 */
export function placeholderStatement(screenId, payload) {
  if (!payload) {
    return {
      state: 'loading',
      headline: 'Checking gateway capabilities\u2026',
      detail: 'The gateway has not sent a telemetry frame yet.',
    };
  }
  const cap = capabilityView(payload, screenId === 'cluster' ? 'cluster' : 'controller');
  if (cap.state === 'not_enabled') {
    return {
      state: 'not_enabled',
      headline: screenId === 'cluster' ? 'Cluster mode is not enabled' : 'Phantom Intelligence is not available yet',
      detail: screenId === 'cluster'
        ? 'This gateway is running in standalone mode. Cluster membership and cross-node purges are inactive.'
        : 'The gateway does not have autonomous intelligence active in the serving path.',
    };
  }
  if (cap.state === 'enabled') {
    return {
      state: 'no_contract',
      headline: screenId === 'cluster'
        ? 'Cluster mode is enabled, but no cluster telemetry contract exists in schema 2'
        : 'The controller capability is reported, but no intelligence contract exists in schema 2',
      detail: 'The gateway configuration indicates this feature is running, but schema 2 defines no telemetry fields for it. This page remains a placeholder until a contract is specified (docs/design/ui_api_requirements.md).',
    };
  }
  if (cap.state === 'unknown') {
    return {
      state: 'unknown',
      headline: `Unrecognised ${screenId} capability: ${clip(cap.value, 40)}`,
      detail: 'The gateway sent a capability value this interface does not recognise.',
    };
  }
  return {
    state: 'unreported',
    headline: `The gateway does not report ${screenId} capability`,
    detail: 'Schema 2 defines capability fields, but the current telemetry frame does not include one for this screen.',
  };
}

function NodeList({ payload }) {
  const nodes = nodeViews(payload);
  if (!nodes || nodes.length === 0) {
    return <p className="section__note">No individual node entries reported.</p>;
  }
  return (
    <div className="table-wrap">
      <table>
        <caption>Cluster members reported by the gateway</caption>
        <thead>
          <tr>
            <th scope="col">Node</th>
            <th scope="col">State</th>
            <th scope="col">Provenance</th>
          </tr>
        </thead>
        <tbody>
          {nodes.map((n) => (
            <tr key={n.key}>
              <th scope="row">Node {n.id}</th>
              <td>{n.state}</td>
              <td><ProvenanceBadge src={n.stateSrc} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Three boxes: membership, purge fan-out, recovery. No health colours, no counts, no flights between nodes. */
function ClusterIllustration() {
  return (
    <Illustration
      title="Cluster ideas: membership, purge fan-out, recovery"
      desc="Three dashed boxes in a row, left to right: Members, Purge fan-out, Recovery, joined by arrows. Nothing is counted and no node has a status."
      viewBox="0 0 360 90"
      notShown="Cluster state is not reported. It shows the order of three ideas, not any node, message, delay or count. In a cluster, a purge (DELETE) is sent to peers; a GET is answered from the gateway's own cache only. Delivery of a purge to other nodes is not reported and no bound is claimed."
    >
      <ArrowDefs />
      <Box x="2" y="25" w="104" h="40" label="Members" />
      <Box x="128" y="25" w="104" h="40" label="Purge fan-out" />
      <Box x="254" y="25" w="104" h="40" label="Recovery" />
      <Arrow d="M106 45H126" />
      <Arrow d="M232 45H252" />
    </Illustration>
  );
}

/** The designed loop, drawn as a loop that is not wired in the serving path. */
function IntelligenceIllustration() {
  return (
    <Illustration
      title="The designed intelligence loop"
      desc="Six dashed boxes in two rows joined by arrows. Top row, left to right: Observe, Detect, Predict. Bottom row, left to right: Evaluate, Adapt, Explain. An arrow leads from Predict down to Evaluate."
      viewBox="0 0 360 150"
      notShown="This is the loop the research design describes. It is not wired in the serving path: the gateway makes no predictions, runs no canary and performs no rollback, and no decision or signal is shown anywhere in this interface."
    >
      <ArrowDefs />
      <Box x="2" y="10" w="104" h="40" label="Observe" />
      <Box x="128" y="10" w="104" h="40" label="Detect" />
      <Box x="254" y="10" w="104" h="40" label="Predict" />
      <Box x="2" y="100" w="104" h="40" label="Evaluate" />
      <Box x="128" y="100" w="104" h="40" label="Adapt" />
      <Box x="254" y="100" w="104" h="40" label="Explain" />
      <Arrow d="M106 30H126" />
      <Arrow d="M232 30H252" />
      <Arrow d="M306 50V75H54V98" />
      <Arrow d="M106 120H126" />
      <Arrow d="M232 120H252" />
    </Illustration>
  );
}

/** Honest placeholder driven by capability discovery. Never shows numbers it does not have. */
export default function Placeholder({ screenId }) {
  const { payload } = useTelemetryContext();
  const copy = COPY[screenId];
  const view = placeholderStatement(screenId, payload);
  const kind = view.state === 'loading' ? 'loading' : view.state === 'not_enabled' ? 'notEnabled' : 'unavailable';
  return (
    <ScreenFrame screenId={screenId} title={copy.title} intro={copy.intro}>
      <StateBlock kind={kind} title={view.headline}>{view.detail}</StateBlock>
      {screenId === 'cluster' && view.state === 'no_contract' ? (
        <Section title="Reported nodes"><NodeList payload={payload} /></Section>
      ) : null}
      <Section title={screenId === 'cluster' ? 'The ideas this page is for' : 'The loop this page is for'} note="A drawing of the design, not a reading from the gateway.">
        {screenId === 'cluster' ? <ClusterIllustration /> : <IntelligenceIllustration />}
      </Section>
      <Section title="What this page would show" note="Only once the gateway reports it. Nothing below is a value.">
        <ul className="plain-list">
          {copy.wouldShow.map((t) => <li key={t}>{t}</li>)}
        </ul>
      </Section>
      <Section title="What exists today instead">
        <ul className="plain-list">
          {copy.instead.map((i) => <li key={i.href}><a href={i.href}>{i.text}</a></li>)}
        </ul>
      </Section>
      <p className="footnote">Backend dependency: a telemetry contract for this page (docs/design/ui_api_requirements.md).</p>
    </ScreenFrame>
  );
}
