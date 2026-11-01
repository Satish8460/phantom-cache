import React from 'react';
import ScreenFrame, { NotReportedList, Section } from './ScreenFrame.jsx';
import { Metric } from '../components/Metric.jsx';
import { fixView } from './MetricGroup.jsx';
import OriginGate from './OriginGate.jsx';
import { originCapView } from './originCap.js';
import { linkOf } from './linkUi.jsx';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import FieldTable from '../components/FieldTable.jsx';
import { StatusPill } from '../components/States.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { additionalFields, capabilityView, metricViews } from '../lib/viewmodel.js';
import { formatAge, formatClock, formatDuration, clip } from '../lib/format.js';
import { hrefFor } from '../lib/routes.js';

const CAP_PILL = {
  enabled: { tone: 'ok', icon: 'ok', text: (v) => `Enabled (${v})` },
  not_enabled: { tone: 'neutral', icon: 'unavailable', text: (v) => `Not enabled (${v})` },
  unknown: { tone: 'warn', icon: 'warn', text: (v) => `Unrecognised value: ${clip(v, 40)}` },
  unreported: { tone: 'neutral', icon: 'unavailable', text: () => 'Not reported' },
};

// Key signals grouped by what they mean (skill S16 grouping). Strictly 3 groups matching shell.test.mjs.
const GROUPS = [
  { title: 'Traffic', ids: ['throughput_req_s_10s', 'hit_ratio_10s', 'latency_p99'], more: 'traffic' },
  { title: 'Freshness guard', ids: ['stale_cap_violations'], more: 'freshness' },
  { title: 'Memory and origin', ids: ['bytes_resident', 'origin_in_flight'], more: 'cache' },
];

const reported = (v) => v.state === 'value' || v.state === 'stale';

/**
 * One plain sentence about the stream, in the shared connection words, as text (not a second status pill: the header indicator is the
 * one tier-1 status). It states what this page sees; it is not a health verdict.
 */
function StatusLine() {
  const ctx = useTelemetryContext();
  const { conn } = ctx;
  const link = linkOf(ctx);
  const age = conn.ageMs === null ? '' : ` Last frame ${formatAge(conn.ageMs)}.`;
  return (
    <p className="status-line" data-status-line={conn.kind} data-link={link.key}>
      <span>{link.cause}{age}</span>
      <ProvenanceBadge src="inferred" title="Worked out by this page from the arrival of frames, not reported by the gateway." />
    </p>
  );
}

function KeySignals() {
  const { payload, conn } = useTelemetryContext();
  const groups = GROUPS.map((g) => ({ ...g, views: metricViews(payload, g.ids, conn.kind).map(fixView) }));
  const missing = groups.flatMap((g) => g.views.filter((v) => !reported(v)));
  return (
    <>
      {groups.map((g) => {
        const shown = g.views.filter(reported);
        if (shown.length === 0) return null;
        return (
          <Section
            key={g.title}
            title={g.title}
            aside={<a className="section__more" href={hrefFor(g.more)}>Details<span className="visually-hidden">: {g.title}</span></a>}
          >
            <dl className="metric-grid" aria-label={g.title}>
              {shown.map((v) => <Metric key={v.id} view={v} />)}
            </dl>
          </Section>
        );
      })}
      {missing.length > 0 ? (
        <Section title={`Not reported (${missing.length})`}>
          <NotReportedList items={missing.map((v) => ({
            id: v.id,
            label: v.label,
            reason: v.reasonText === 'Not enough samples in the window yet'
              ? 'Idle: awaiting requests in the 10 s window'
              : v.reasonText || 'The gateway did not report a value for this metric in the current frame.',
          }))} />
        </Section>
      ) : null}
    </>
  );
}

/** Origin protection summary: shown ONLY when the gateway reports a gate (R4 E3). Its absence is explained on the Origin page, not drawn here. */
function OriginSummary() {
  const { payload } = useTelemetryContext();
  const v = originCapView(payload);
  if (v.state === 'loading' || v.state === 'unavailable') return null;
  return (
    <Section
      title="Origin concurrency controller"
      note="A limit on how many fetches may be in flight to the origin, and the controller that adjusts it."
      aside={<a className="section__more" href={hrefFor('origin')}>Details<span className="visually-hidden">: origin concurrency controller</span></a>}
    >
      <OriginGate payload={payload} />
    </Section>
  );
}

/** Connection and capabilities in one definition list: what the stream is, then what the gateway says it can do. */
function GatewayFacts() {
  const ctx = useTelemetryContext();
  const { payload, state, conn, gateway, offsetMs } = ctx;
  const link = linkOf(ctx);
  return (
    <Section title="Gateway" note="Items labelled Derived, inferred are worked out by this page from the stream, not reported by the gateway.">
      <dl className="kv kv--wide" aria-label="Gateway connection and capabilities">
        <dt>Telemetry stream</dt>
        <dd>{link.word} <ProvenanceBadge src="inferred" /></dd>
        <dt>Last frame</dt>
        <dd>{conn.ageMs === null ? 'none received' : formatAge(conn.ageMs)} <ProvenanceBadge src="inferred" /></dd>
        <dt>Gateway address</dt>
        <dd className="mono">{gateway.url}</dd>
        <dt>Address set by</dt>
        <dd>{gateway.sourceText || 'unknown'}</dd>
        <dt>Schema</dt>
        <dd>{payload ? payload.schema : 'unknown'}</dd>
        <dt>Mode</dt>
        <dd>{payload ? payload.mode : 'unknown'} {payload ? <ProvenanceBadge src="measured" /> : null}</dd>
        <dt>Server time of last frame</dt>
        <dd>{payload ? formatClock(payload.as_of_ms) : 'unknown'} {payload ? <ProvenanceBadge src="measured" /> : null}</dd>
        <dt>Gateway uptime</dt>
        <dd>{payload && Number.isFinite(payload.uptime_ms) ? formatDuration(payload.uptime_ms) : 'unknown'} {payload ? <ProvenanceBadge src="measured" /> : null}</dd>
        <dt>Client minus server clock</dt>
        <dd>{offsetMs === null ? 'unknown' : `${offsetMs} ms (includes network delay)`} <ProvenanceBadge src="inferred" /></dd>
        <dt>Frames received / unreadable</dt>
        <dd>{state.messages} / {state.badFrames}</dd>
        {['cluster', 'controller', 'origin_metrics'].map((name) => {
          const cap = capabilityView(payload, name);
          const p = CAP_PILL[cap.state];
          return (
            <React.Fragment key={name}>
              <dt>{cap.label}</dt>
              <dd data-capability={name} data-state={cap.state}>
                <StatusPill tone={p.tone} icon={p.icon}>{p.text(cap.value)}</StatusPill>{' '}
                <ProvenanceBadge src={cap.state === 'unreported' ? 'unavailable' : 'derived'} />
              </dd>
            </React.Fragment>
          );
        })}
      </dl>
    </Section>
  );
}

function AdditionalFields({ payload }) {
  const extras = additionalFields(payload).filter((e) => !e.path.startsWith('origin_cap'));
  return (
    <section className="section" aria-label="Other reported fields">
      <details className="disclosure">
        <summary>Other reported fields ({extras.length})</summary>
        <p className="section__note">Fields the gateway sends that this UI has no dedicated tile for yet. Shown as received, with their stated label.</p>
        {extras.length === 0 ? (
          <p className="section__note">None. Every labelled metric in the current frame has a tile.</p>
        ) : (
          <FieldTable rows={extras} caption="Additional fields in metrics" />
        )}
      </details>
    </section>
  );
}

export default function Overview() {
  const { payload } = useTelemetryContext();
  return (
    <ScreenFrame screenId="overview" title="Overview" intro="What the gateway reports right now, grouped by meaning. Every value carries its label; this page estimates nothing.">
      <StatusLine />
      <KeySignals />
      <OriginSummary />
      <GatewayFacts />
      <AdditionalFields payload={payload} />
    </ScreenFrame>
  );
}
