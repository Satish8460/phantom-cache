import React from 'react';
import ScreenFrame, { Section } from './ScreenFrame.jsx';
import MetricGroup from './MetricGroup.jsx';
import OriginGate from './OriginGate.jsx';
import { StateBlock } from '../components/States.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import FieldTable from '../components/FieldTable.jsx';
import { additionalFields, originAvailability } from '../lib/viewmodel.js';
import { labHref } from '../lib/routes.js';

export default function Origin() {
  const { payload, conn } = useTelemetryContext();
  const avail = originAvailability(payload);
  // Origin-related fields the gateway reports that have no dedicated reading yet. origin_cap has its own section below.
  const extras = additionalFields(payload).filter((e) => e.path.startsWith('origin') && !e.path.startsWith('origin_cap'));
  return (
    <ScreenFrame screenId="origin" title="Origin" intro="Fetches the gateway makes to the origin on cache misses, and the origin concurrency controller where the gateway reports one.">
      <Section
        title="Origin concurrency controller"
        note="A limit on how many fetches may be in flight to the origin, and the controller that adjusts it. Shown only as the gateway reports it: no drawing of a gate, no claim of benefit."
      >
        <OriginGate payload={payload} />
        <p className="section__note">Retained experiment results about this mechanism are in <a href={labHref('evidence')}>Phantom Lab, Evidence</a>. They are kept apart from this live reading and do not describe it.</p>
      </Section>
      {avail.state === 'unavailable' || avail.state === 'rejected' ? (
        <StateBlock kind="unavailable" title={avail.state === 'rejected' ? 'Origin data withheld' : 'Origin metrics are unavailable'}>
          {avail.reasonText}. No origin values are shown, and none are zero.
        </StateBlock>
      ) : (
        <>
          <Section title="Fetches">
            <MetricGroup payload={payload} connKind={conn.kind} label="Origin fetches" ids={['origin_started', 'origin_ok', 'origin_in_flight']} />
          </Section>
          <Section title="Errors">
            <MetricGroup
              payload={payload}
              connKind={conn.kind}
              label="Origin errors"
              ids={['origin_err_timeout', 'origin_err_network', 'origin_err_5xx', 'origin_err_other']}
            />
          </Section>
          <Section title="Origin latency" note="Appears once the gateway has origin samples.">
            <MetricGroup payload={payload} connKind={conn.kind} label="Origin latency" ids={['origin_latency_p50', 'origin_latency_p95', 'origin_latency_p99']} />
          </Section>
        </>
      )}
      {extras.length > 0 ? (
        <Section title="Other origin fields" note="Reported by the gateway but not yet given a dedicated tile. Shown as received, with their stated label.">
          <FieldTable rows={extras} caption="Additional origin fields" />
        </Section>
      ) : null}
    </ScreenFrame>
  );
}
