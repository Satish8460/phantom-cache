import React from 'react';
import ChartFrame from './ChartFrame.jsx';
import { toSegments } from '../lib/history.js';
import { formatClock } from '../lib/format.js';

const W = 320;
const H = 96;

/**
 * Time-series line of server-supplied values collected since page load (not server-side history), inside a ChartFrame:
 * title, provenance badge, Chart/Table toggle and full-width axis captions. The table alternative is always in the DOM.
 * `format` formats an axis/table value. Axis text only formats the received minimum and maximum; nothing is computed here.
 */
export function Sparkline({ title, series, src, format, unitLabel }) {
  const { segments, min, max } = toSegments(series, W, H);
  const points = series.filter((p) => p.v !== null);
  const last = points[points.length - 1];
  const summary = last
    ? `${title}: ${points.length} samples since page load, latest ${format(last.v)}${unitLabel ? ` ${unitLabel}` : ''}.`
    : `${title}: no samples yet.`;
  const rows = series.slice(-30).reverse();
  const unit = unitLabel ? ` ${unitLabel}` : '';
  let yAxis = 'No samples yet';
  if (min !== null) yAxis = min === max ? `Constant at ${format(min)}${unit}` : `${format(min)} to ${format(max)}${unit}`;
  const chart = points.length === 0 ? (
    <p className="panel__note">No samples yet. Gaps appear where the gateway reports a value as unavailable.</p>
  ) : (
    <svg className="spark" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={summary} preserveAspectRatio="none">
      <line className="spark__grid" x1="0" x2={W} y1={H / 2} y2={H / 2} />
      {segments.map((seg, i) => (
        <polyline key={i} className="spark__line" points={seg.map((p) => p.join(',')).join(' ')} />
      ))}
    </svg>
  );
  const table = (
    <div className="table-wrap">
      <table>
        <caption>{title}, newest first (up to 30 rows)</caption>
        <thead>
          <tr><th scope="col">Received (client clock)</th><th scope="col">Value</th></tr>
        </thead>
        <tbody>
          {rows.map((p) => (
            <tr key={p.t}>
              <td>{formatClock(p.t)}</td>
              <td className="num">{p.v === null ? 'unavailable' : `${format(p.v)}${unit}`}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
  return (
    <ChartFrame
      title={title}
      src={src}
      xAxis={`Time since page load, oldest to newest. History kept by this page only, last ${series.length} samples.`}
      yAxis={yAxis}
      unitLabel={unitLabel}
      chart={chart}
      table={table}
    />
  );
}

/** Horizontal bars for a handful of server-supplied values. Text values are the accessible content. */
export function PercentileBars({ items }) {
  const present = items.filter((i) => i.raw !== undefined && i.raw !== null);
  const max = present.length ? Math.max(...present.map((i) => i.raw)) : 0;
  // Nothing reported at all: one unavailable row instead of four empty tracks.
  if (present.length === 0) {
    const waiting = items.length > 0 && items.every((i) => i.state === 'loading');
    return <p className="panel__note" data-bars-empty>{waiting ? 'Waiting for data.' : `Unavailable: ${items.map((i) => i.short).join(', ')} are not reported yet.`}</p>;
  }
  return (
    <ul className="bars">
      {items.map((i) => {
        const has = i.raw !== undefined && i.raw !== null;
        const pct = has && max > 0 ? Math.max(1, Math.round((i.raw / max) * 100)) : 0;
        return (
          <li key={i.id}>
            <span>{i.short}</span>
            <span className="bars__track" aria-hidden="true"><span className="bars__fill" style={{ display: 'block', width: `${pct}%` }} /></span>
            <span className="bars__value">{has ? i.text : i.state === 'loading' ? 'waiting' : 'unavailable'}</span>
          </li>
        );
      })}
    </ul>
  );
}

/** Percentile bars in a ChartFrame, with the same values as a table (Chart/Table toggle). items as for PercentileBars. */
export function PercentileChart({ title, items, src, unitLabel = 'ms' }) {
  const table = (
    <div className="table-wrap">
      <table>
        <caption>{title}</caption>
        <thead><tr><th scope="col">Percentile</th><th scope="col">Value</th></tr></thead>
        <tbody>
          {items.map((i) => (
            <tr key={i.id}>
              <th scope="row">{i.short}</th>
              <td className="num">{i.raw !== undefined && i.raw !== null ? i.text : i.state === 'loading' ? 'waiting' : 'unavailable'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
  return (
    <ChartFrame
      title={title}
      src={src}
      xAxis="Bar length is relative to the largest reported percentile"
      unitLabel={unitLabel}
      chart={<PercentileBars items={items} />}
      table={table}
    />
  );
}

/** Used vs capacity bar. No percentage is computed or shown; the bar is geometry only. */
export function CapacityBar({ used, total, usedText, totalText }) {
  const ok = Number.isFinite(used) && Number.isFinite(total) && total > 0;
  const rawPct = ok ? (used / total) * 100 : 0;
  const width = ok ? Math.min(100, Math.max(0, Math.round(rawPct))) : 0;
  return (
    <div className="capacity">
      {/* The track always shows where the scale starts and ends, so an empty bar still reads as "0 of budget". */}
      <div className="capacity__track" aria-hidden="true"><div className="bars__fill" style={{ width: `${width}%` }} /></div>
      <div className="capacity__ends" aria-hidden="true"><span>0</span><span>{ok ? totalText : 'budget unknown'}</span></div>
      <p className="panel__note">{ok ? `${usedText} of ${totalText} budget` : 'Bar not drawn: a resident value and a positive budget are both required.'}</p>
    </div>
  );
}

/** CapacityBar in a ChartFrame with a table alternative. `src` is the provenance of the resident value. */
export function CapacityChart({ title, used, total, usedText, totalText, src }) {
  const table = (
    <div className="table-wrap">
      <table>
        <caption>{title}</caption>
        <thead><tr><th scope="col">Item</th><th scope="col">Value</th></tr></thead>
        <tbody>
          <tr><th scope="row">Resident bytes (logical)</th><td className="num">{usedText || 'unavailable'}</td></tr>
          <tr><th scope="row">Byte budget</th><td className="num">{totalText || 'unavailable'}</td></tr>
        </tbody>
      </table>
    </div>
  );
  return (
    <ChartFrame
      title={title}
      src={src}
      xAxis="Scale runs from 0 to the configured byte budget"
      chart={<CapacityBar used={used} total={total} usedText={usedText} totalText={totalText} />}
      table={table}
    />
  );
}
