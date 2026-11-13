import React from 'react';
import ScreenFrame, { NotReportedList, Section } from './ScreenFrame.jsx';
import MetricGroup from './MetricGroup.jsx';
import Illustration from './Illustration.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';

/** The three words used for an entry's freshness, drawn as equal segments with NO scale and NO time (the configured lifetime is not the effective one). */
function LifecycleIllustration() {
  return (
    <Illustration
      title="How an entry's freshness is described"
      desc="A bar of three equal segments, left to right: Fresh, Stale and servable, Not servable. A marker between the first two reads Lifetime ends. A marker between the last two reads Hard staleness cap. Segment widths are not to scale and no time is drawn."
      viewBox="0 0 360 100"
      notShown="Segment widths are not to scale and no time or count is drawn. It does not show any entry, the configured lifetime, or how long this gateway actually keeps entries. The gateway reports counters only, above."
    >
      <text className="illus__sub" x="120" y="22" textAnchor="middle">Lifetime ends</text>
      <text className="illus__sub" x="240" y="22" textAnchor="middle">Hard staleness cap</text>
      <path className="illus__line" d="M120 28V40M240 28V40" />
      <rect className="illus__box illus__box--ghost" x="2" y="40" width="118" height="36" />
      <rect className="illus__box illus__box--ghost" x="120" y="40" width="120" height="36" />
      <rect className="illus__box illus__box--ghost" x="240" y="40" width="118" height="36" />
      <text className="illus__text" x="61" y="62" textAnchor="middle">Fresh</text>
      <text className="illus__text" x="180" y="62" textAnchor="middle">Stale, servable</text>
      <text className="illus__text" x="299" y="62" textAnchor="middle">Not servable</text>
    </Illustration>
  );
}

export default function Freshness() {
  const { payload, conn } = useTelemetryContext();
  return (
    <ScreenFrame screenId="freshness" title="Freshness" intro="Stale serving and revalidation counters. Only fields the gateway reports today appear here.">
      <Section title="Stale serving" note="The hard staleness cap is enforced by the gateway. The violation counter must stay at 0. A zero here is a real zero: the gateway counted none. The retained Scenario B results (Phantom Lab, Evidence) do not establish anything about the effect of stale serving.">
        <MetricGroup payload={payload} connKind={conn.kind} label="Stale serving" ids={['stale_served_total', 'stale_cap_violations']} />
      </Section>
      <Section title="Revalidation" note="Background checks that ask the origin whether a stale entry is still current. Not modified means the origin said the entry is unchanged.">
        <MetricGroup
          payload={payload}
          connKind={conn.kind}
          label="Revalidation"
          ids={['reval_started', 'reval_ok', 'reval_not_modified', 'reval_error', 'reval_in_flight']}
        />
      </Section>
      <Section title="How freshness is described" note="The drawing explains the three words used on this page. It is not read from the gateway.">
        <LifecycleIllustration />
      </Section>
      <Section title="Not part of the telemetry yet" note="Schema 2 has no field for these (see docs/design/ui_api_requirements.md), so nothing is drawn for them.">
        <NotReportedList
          label="Not reported on this page"
          items={[
            { id: 'stale_age', label: 'Age of stale responses', reason: 'The gateway does not report it, so none is shown.' },
            { id: 'stale_fraction', label: 'Stale fraction', reason: 'The gateway does not report it.' },
            { id: 'per_key', label: 'Per-key freshness', reason: 'Not part of schema 2.' },
          ]}
        />
      </Section>
    </ScreenFrame>
  );
}
