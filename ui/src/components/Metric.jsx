import React from 'react';
import ProvenanceBadge from './ProvenanceBadge.jsx';
import Icon from './Icon.jsx';
import { metricViews } from '../lib/viewmodel.js';

const STATE_WORD = {
  loading: 'Waiting for data',
  unavailable: 'Unavailable',
  missing: 'Not reported',
  rejected: 'Withheld',
};

/** One value with its label, window and provenance badge. `view` comes from metricView(). */
export function Metric({ view }) {
  const hasValue = view.state === 'value' || view.state === 'stale';
  const cls = ['metric', `metric--${view.state}`, view.tone === 'critical' ? 'metric--critical' : ''].filter(Boolean).join(' ');
  return (
    <div className={cls} data-metric={view.id} data-state={view.state}>
      <dt>{view.label}</dt>
      <dd className="metric__value">
        {hasValue ? view.text : STATE_WORD[view.state]}
        {view.state === 'stale' ? <span className="visually-hidden"> (stale, last received value)</span> : null}
        {view.tone === 'critical' ? <span className="visually-hidden"> (critical: expected 0)</span> : null}
      </dd>
      <dd className="metric__meta">
        <ProvenanceBadge src={view.src} />
        {hasValue ? <span>{view.windowText}</span> : null}
        {view.state === 'stale' ? <span className="tag tag--stale"><Icon name="clock" /><span>stale</span></span> : null}
      </dd>
      {!hasValue && view.state !== 'loading' ? <dd className="metric__reason">{view.reasonText}</dd> : null}
      {hasValue && view.note ? <dd className="metric__reason">{view.note}</dd> : null}
    </div>
  );
}

export function MetricGrid({ payload, ids, connKind, label }) {
  const views = metricViews(payload, ids, connKind);
  return (
    <dl className="metric-grid" aria-label={label}>
      {views.map((v) => (
        <Metric key={v.id} view={v} />
      ))}
    </dl>
  );
}
