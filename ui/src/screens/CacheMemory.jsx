import React from 'react';
import ScreenFrame, { NotReportedList, Section } from './ScreenFrame.jsx';
import MetricGroup, { metricViewsFixed } from './MetricGroup.jsx';
import { CapacityChart } from '../components/Charts.jsx';
import InteractiveConsole from '../components/InteractiveConsole.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';

export default function CacheMemory() {
  const { payload, conn } = useTelemetryContext();
  const [res, bud] = metricViewsFixed(payload, ['bytes_resident', 'byte_budget'], conn.kind);
  const bothValues = (res.state === 'value' || res.state === 'stale') && (bud.state === 'value' || bud.state === 'stale');
  return (
    <ScreenFrame screenId="cache" title="Cache & memory" intro="Accounted cache bytes against the configured budget, plus eviction and rejection counters.">
      <Section title="Interactive Invalidation & Cache Testing" note="Send requests or trigger instant key invalidation (DELETE) to verify byte budget changes.">
        <InteractiveConsole defaultKey="/api/v1/cache/demo-key" />
      </Section>
      <Section title="Capacity" note="Resident bytes are the cache's own logical accounting, not operating-system memory.">
        <MetricGroup payload={payload} connKind={conn.kind} label="Capacity" ids={['bytes_resident', 'byte_budget']} />
        <CapacityChart
          title="Resident bytes against budget"
          src={res.src}
          used={bothValues ? res.raw : NaN}
          total={bothValues ? bud.raw : NaN}
          usedText={res.text}
          totalText={bud.text}
        />
      </Section>
      <Section title="Evictions and rejections" note="Counts since the gateway started. The gateway reports how many, not which entries or why.">
        <MetricGroup
          payload={payload}
          connKind={conn.kind}
          label="Evictions and rejections"
          ids={['evictions_total', 'capacity_rejections', 'byte_budget_rejections', 'oversize_rejections']}
        />
      </Section>
      <Section title="Process" note="Process memory is reported only where the gateway platform supports it (Linux today).">
        <MetricGroup payload={payload} connKind={conn.kind} label="Process memory" ids={['rss_bytes']} />
      </Section>
      <Section title="Not part of the telemetry yet" note="Schema 2 has no field for these (see docs/design/ui_api_requirements.md), so nothing is drawn for them.">
        <NotReportedList
          label="Not reported on this page"
          items={[
            { id: 'entry_count', label: 'Entry count', reason: 'The gateway does not report one, so none is shown.' },
            { id: 'entry_state', label: 'Entry age and state', reason: 'The gateway does not report entry state, so no entry list or age distribution is drawn.' },
            { id: 'evicted_which', label: 'Which entries were evicted, and why', reason: 'The gateway reports how many evictions happened, not which entries or why.' },
            { id: 'per_policy', label: 'Per-policy detail', reason: 'Not part of schema 2.' },
          ]}
        />
      </Section>
    </ScreenFrame>
  );
}
