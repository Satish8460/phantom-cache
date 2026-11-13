import React from 'react';
import ScreenFrame, { NotReportedList, Section } from './ScreenFrame.jsx';
import MetricGroup, { metricViewsFixed } from './MetricGroup.jsx';
import { PercentileChart, Sparkline } from '../components/Charts.jsx';
import TrafficGenerator from '../components/TrafficGenerator.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { formatMs, formatRate, formatRatioAsPercent } from '../lib/format.js';

export default function Traffic() {
  const { payload, conn, series } = useTelemetryContext();
  const lat = metricViewsFixed(payload, ['latency_p50', 'latency_p90', 'latency_p95', 'latency_p99'], conn.kind);
  const items = lat.map((v) => ({ id: v.id, short: v.label.replace('GET latency ', ''), raw: v.raw, text: v.text, state: v.state }));
  const latSrc = lat.find((v) => v.state === 'value' || v.state === 'stale')?.src || 'unavailable';

  return (
    <div className="traffic-screen-wide" data-wide-layout="true">
      <ScreenFrame screenId="traffic" title="Traffic & latency" intro="GET request volume, hit ratios and latency percentiles as reported by the gateway.">
        <Section title="Live Traffic & Workload Generator" note="Multi-source workload engine: physical smartphone swarm fleet, continuous Poisson cluster stream, or targeted precision probe.">
          <TrafficGenerator />
        </Section>

        {/* 2-Column Responsive Dashboard Bay for Rates, Latency & Outcomes */}
        <div className="traffic-dashboard-grid">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-section)' }}>
            <Section title="Rates and ratios" note="Windows are computed by the gateway. This page computes no rates. A ratio the gateway cannot yet work out is shown as Unavailable with its reason, never as a number.">
              <MetricGroup
                payload={payload}
                connKind={conn.kind}
                label="Rates and ratios"
                ids={['throughput_req_s_1s', 'throughput_req_s_10s', 'hit_ratio_1s', 'hit_ratio_10s', 'hit_ratio_cumulative', 'requests_total']}
              />
            </Section>

            <Section
              title="Request outcomes"
              note="Each counter is shown as the gateway reports it, cumulative since the gateway started. They do not add up to the request total: for example a miss is also counted when the gateway refreshes an entry in the background, and a request that waits on another request's fetch is counted only as a waiter. So this page shows no share and no remainder."
            >
              <MetricGroup payload={payload} connKind={conn.kind} label="Request counters" ids={['cache_hits', 'cache_misses', 'coalesced_waiters']} />
            </Section>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-section)' }}>
            <Section title="GET latency" note="Over the gateway's most recent GET observations; DELETE is excluded.">
              <PercentileChart title="GET latency percentiles" items={items} src={latSrc} />
              <MetricGroup payload={payload} connKind={conn.kind} label="Latency sample count" ids={['latency_samples']} />
            </Section>
          </div>
        </div>

        <Section title="Over time" note="Server values collected by this page since it loaded. This is not gateway history.">
          <div className="grid">
            <Sparkline title="Throughput, 1 s window" series={series.throughput} src="derived" format={formatRate} unitLabel="req/s" />
            <Sparkline title="Hit ratio, 1 s window" series={series.hitRatio} src="derived" format={formatRatioAsPercent} />
            <Sparkline title="GET latency p99" series={series.p99} src="measured" format={formatMs} unitLabel="ms" />
          </div>
        </Section>

        <Section title="Not part of the telemetry" note="Schema 2 has no field for these, so nothing is drawn for them.">
          <NotReportedList
            label="Not reported on this page"
            items={[
              { id: 'latency_distribution', label: 'Latency distribution', reason: 'The gateway reports percentiles of recent GETs, not the distribution.' },
              { id: 'per_request', label: 'Individual requests', reason: 'The gateway reports counters, not one record per request, so no request path is drawn.' },
            ]}
          />
        </Section>
      </ScreenFrame>
    </div>
  );
}
