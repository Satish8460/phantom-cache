import React, { useId, useState } from 'react';
import ProvenanceBadge from './ProvenanceBadge.jsx';

/**
 * Shared chart container (spec section 7). Title, provenance badge, units, full-width axis captions and a Chart / Table
 * toggle. The table alternative is ALWAYS in the DOM (the inactive pane is `hidden`), so it is available without scripts,
 * to assistive technology and to tests.
 *
 * Props: title, src (provenance class), qualifier, unitLabel, xAxis, yAxis, caption, chart (node), table (node),
 * defaultView ('chart' | 'table'), headingLevel (default 3), badge (set false only if the chart draws its own badge).
 * The component draws no data and computes no numbers; callers pass the already-labelled chart and table.
 */
export default function ChartFrame({ title, src, qualifier, unitLabel, xAxis, yAxis, caption, chart, table, defaultView = 'chart', headingLevel = 3, badge = true }) {
  const id = useId();
  const [view, setView] = useState(table ? defaultView : 'chart');
  const H = `h${headingLevel >= 2 && headingLevel <= 4 ? headingLevel : 3}`;
  const hasAxis = Boolean(xAxis || yAxis || unitLabel);
  return (
    <figure className="chart-frame" aria-labelledby={`${id}-t`} data-view={view}>
      <div className="chart-frame__head">
        <H id={`${id}-t`} className="chart-frame__title">{title}</H>
        {badge && src ? <ProvenanceBadge src={src} qualifier={qualifier} /> : null}
        {table ? (
          <div className="seg" role="group" aria-label={`${title}: view`}>
            <button type="button" className="seg__btn" aria-pressed={view === 'chart'} onClick={() => setView('chart')}>Chart</button>
            <button type="button" className="seg__btn" aria-pressed={view === 'table'} onClick={() => setView('table')}>Table</button>
          </div>
        ) : null}
      </div>
      <div className="chart-frame__pane" hidden={view !== 'chart'}>{chart}</div>
      {table ? <div className="chart-frame__pane" hidden={view !== 'table'}>{table}</div> : null}
      {hasAxis ? (
        <p className="chart-frame__axes">
          {xAxis ? <span><span className="chart-frame__axis-key">X</span> {xAxis}</span> : null}
          {yAxis ? <span><span className="chart-frame__axis-key">Y</span> {yAxis}</span> : null}
          {unitLabel ? <span><span className="chart-frame__axis-key">Unit</span> {unitLabel}</span> : null}
        </p>
      ) : null}
      {caption ? <figcaption className="chart-frame__caption">{caption}</figcaption> : null}
    </figure>
  );
}
