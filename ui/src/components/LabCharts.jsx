import React, { useId } from 'react';
import ChartFrame from './ChartFrame.jsx';
import { TierBadge, CellText } from './LabParts.jsx';
import Icon from './Icon.jsx';
import { scaleLinear, FMT } from '../lib/labView.js';
import { tierBadgeQualifier, tierInfo } from '../lib/labProvenance.js';

// Hand-rolled CSS charts (no chart library), drawn inside the shared ChartFrame: title, provenance badge, a Chart / Table
// toggle (the table is always in the DOM), full-width axis captions. Every chart is text first: the numbers are in the DOM as
// text, the drawn marks are aria-hidden geometry. Charts plot received values only and compute nothing but a linear scale.

function pct(v) {
  return `${Math.max(0, Math.min(100, v)).toFixed(2)}%`;
}

function frameProps(tier, build) {
  if (!tier) return { badge: false };
  const t = typeof tier === 'object' ? tier : tierInfo(tier);
  return { src: t.src, qualifier: tierBadgeQualifier(t, build) || undefined, badge: true };
}

/** A missing chart is said in words (glyph, text and the UNAVAILABLE badge) inside a solid box. */
function UnavailablePanel({ title, reason }) {
  return (
    <div className="lab-unavail" role="note" data-unavailable-chart={title}>
      <div className="lab-unavail__head">
        <Icon name="unavailable" />
        <strong>{title}</strong>
        <TierBadge tier="UNAVAILABLE" />
      </div>
      <p>Unavailable: {reason}.</p>
    </div>
  );
}

// Bars use at most this share of the plot so the value can sit right after the bar (label and value stay adjacent).
const BAR_SPAN = 78;

/** Horizontal bars from a zero baseline. `rows`: [{ label, cell }], one unit. The bar is geometry; the text carries the value. */
function ZeroBars({ rows, unit, ariaLabel }) {
  const vals = rows.filter((r) => r.cell.state === 'value').map((r) => r.cell.num);
  const max = vals.length ? Math.max(...vals) : 0;
  const sx = scaleLinear({ min: 0, max: max > 0 ? max : 1 }, [0, BAR_SPAN]);
  return (
    <ul className="lab-bars" aria-label={ariaLabel}>
      {rows.map((r) => (
        <li key={r.label} data-bar-row={r.label}>
          <span className="lab-bars__label">{r.label}</span>
          <span className="lab-bars__plot">
            {r.cell.state === 'value' ? <span className="lab-bars__bar" aria-hidden="true" style={{ width: pct(sx(r.cell.num)) }} /> : null}
            <span className="lab-bars__value"><CellText c={r.cell} />{r.cell.state === 'value' && unit ? ` ${unit}` : ''}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}

function TableBox({ caption, head, children }) {
  return (
    <div className="table-wrap">
      <table>
        <caption>{caption}</caption>
        <thead><tr>{head.map((h) => <th key={h} scope="col">{h}</th>)}</tr></thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}

/**
 * Scenario B: the trade-off, not a victory chart. Two panels side by side with the SAME arm order:
 * peak origin concurrency (lower = more protection for the origin) and client success rate (higher = fewer failed
 * requests). Both start at zero. Reading one without the other is the mistake this chart exists to prevent.
 */
export function TradeoffChart({ data, tier, build }) {
  const peakRows = data.map((d) => ({ label: d.arm, cell: d.peak }));
  const okRows = data.map((d) => ({ label: d.arm, cell: d.success }));
  return (
    <div data-chart="tradeoff">
      <ChartFrame
        title="Origin protection versus client success, per arm"
        {...frameProps(tier, build)}
        xAxis="Each panel has its own scale, and every bar starts at 0."
        yAxis="One row per arm, in the same order in both panels."
        caption="Read the two panels together: a lower peak at the origin was paid for with requests that were shed."
        chart={(
          <div className="lab-twin">
            <div>
              <h4 className="lab-subhead">Peak origin concurrency <span className="lab-hint">lower means less load on the origin</span></h4>
              <ZeroBars rows={peakRows} ariaLabel="Peak origin concurrency per arm" />
            </div>
            <div>
              <h4 className="lab-subhead">Client success rate <span className="lab-hint">higher means fewer failed requests; shed 503 responses count as failures</span></h4>
              <ZeroBars rows={okRows} ariaLabel="Client success rate per arm" />
            </div>
          </div>
        )}
        table={(
          <TableBox caption="Peak origin concurrency and client success rate per arm (per-run means as reported)" head={['Arm', 'Peak origin concurrency', 'Client success rate']}>
            {data.map((d) => (
              <tr key={d.arm}><th scope="row">{d.arm}</th><td className="num"><CellText c={d.peak} /></td><td className="num"><CellText c={d.success} /></td></tr>
            ))}
          </TableBox>
        )}
      />
    </div>
  );
}

/** Shed fraction per arm over the whole run; the per-phase breakdown is stated as unavailable. */
export function ShedChart({ shed, perPhase, tier, build }) {
  return (
    <div data-chart="shed">
      <ChartFrame
        title="Shed requests (HTTP 503 from the origin cap)"
        {...frameProps(tier, build)}
        xAxis="Whole run; the bar starts at 0."
        yAxis="One row per arm."
        chart={(
          <>
            <ZeroBars rows={shed.map((d) => ({ label: d.arm, cell: d.whole }))} ariaLabel="Whole-run shed fraction per arm" />
            {perPhase.available ? null : <UnavailablePanel title="Shed fraction per phase" reason={perPhase.reason} />}
          </>
        )}
        table={(
          <TableBox caption="Whole-run shed fraction per arm" head={['Arm', 'Shed fraction (whole run)']}>
            {shed.map((d) => <tr key={d.arm}><th scope="row">{d.arm}</th><td className="num"><CellText c={d.whole} /></td></tr>)}
          </TableBox>
        )}
      />
    </div>
  );
}

/** All-request latency percentiles per arm, clearly separated from the (unavailable) success-only latency. */
export function LatencyChart({ latency, successOnly, tier, build }) {
  const all = latency.flatMap((d) => [d.all.p50, d.all.p95, d.all.p99]).filter((c) => c.state === 'value').map((c) => c.num);
  const max = all.length ? Math.max(...all) : 0;
  const sx = scaleLinear({ min: 0, max: max > 0 ? max : 1 }, [0, BAR_SPAN]);
  return (
    <div data-chart="latency">
      <ChartFrame
        title="Client latency, all requests"
        {...frameProps(tier, build)}
        xAxis="One scale for every arm; bars start at 0."
        unitLabel="milliseconds (pooled scheduled-time samples)"
        caption="These percentiles include the fast-failing 503 responses. They are not evidence that served clients got faster."
        chart={(
          <>
            <div className="lab-lat">
              {latency.map((d) => (
                <div key={d.arm} className="lab-lat__arm" data-latency-arm={d.arm}>
                  <h4 className="lab-subhead">{d.arm}</h4>
                  <ul className="lab-bars" aria-label={`All-request latency percentiles for ${d.arm}`}>
                    {[['P50', d.all.p50], ['P95', d.all.p95], ['P99', d.all.p99]].map(([name, c]) => (
                      <li key={name}>
                        <span className="lab-bars__label">{name}</span>
                        <span className="lab-bars__plot">
                          {c.state === 'value' ? <span className="lab-bars__bar" aria-hidden="true" style={{ width: pct(sx(c.num)) }} /> : null}
                          <span className="lab-bars__value"><CellText c={c} /></span>
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
            <UnavailablePanel title="Latency of successful (HTTP 200) responses only" reason={successOnly.reason} />
          </>
        )}
        table={(
          <TableBox caption="All-request client latency per arm (milliseconds, pooled scheduled-time samples)" head={['Arm', 'P50', 'P95', 'P99', 'Samples']}>
            {latency.map((d) => (
              <tr key={d.arm}><th scope="row">{d.arm}</th><td className="num"><CellText c={d.all.p50} /></td><td className="num"><CellText c={d.all.p95} /></td><td className="num"><CellText c={d.all.p99} /></td><td className="num"><CellText c={d.all.n} /></td></tr>
            ))}
          </TableBox>
        )}
      />
    </div>
  );
}

const EFFECT_WORD = (r) => {
  if (!r.plotted) return 'not plotted: interval not reported';
  if (r.supported) return 'supported';
  if (r.adverse) return 'credibly worse';
  if (r.nullish) return 'practically null';
  return 'not supported';
};

/**
 * M6: per-scenario relative change with its confidence interval and a zero line. Positive = the adaptive arm did
 * better than the comparator on this endpoint; negative = worse. The wording of each row comes from payload flags.
 * `directions` adds the "worse / better" labels at the two ends of the axis (only where the page states that positive means better).
 */
export function EffectChart({ title, caption, chart, tier, build, directions = false }) {
  const id = useId();
  const { rows, domain } = chart;
  const sx = domain ? scaleLinear(domain, [0, 100]) : null;
  const axis = domain
    ? `The axis runs from ${FMT.signedRatio(domain.min)} to ${FMT.signedRatio(domain.max)}. The solid vertical line is 0 (no change); the dot is the mean and the bar is the 95% interval.`
    : undefined;
  return (
    <div data-chart="effect">
      <ChartFrame
        title={title}
        {...frameProps(tier, build)}
        xAxis={axis}
        yAxis="One row per scenario."
        unitLabel="relative change, in percent"
        caption={caption}
        chart={domain ? (
          <>
            <ul className="lab-effect" aria-label={`${title}: mean with confidence interval per scenario`} aria-describedby={`${id}-d`}>
              {rows.map((r) => (
                <li key={r.scenario} data-effect-row={r.scenario}>
                  <span className="lab-bars__label">{r.scenario}</span>
                  <span className="lab-effect__track" aria-hidden="true">
                    <span className="lab-effect__zero" style={{ left: pct(sx(0)) }} />
                    {r.plotted ? (
                      <>
                        <span className="lab-effect__ci" style={{ left: pct(sx(r.lo)), width: pct(sx(r.hi) - sx(r.lo)) }} />
                        <span className={`lab-effect__dot${r.adverse ? ' lab-effect__dot--adverse' : ''}`} style={{ left: pct(sx(r.mean)) }} />
                      </>
                    ) : null}
                  </span>
                  <span className="lab-effect__value">
                    {r.plotted ? <span className="num">{FMT.signedRatio(r.mean)}</span> : null} <span className="lab-hint">{EFFECT_WORD(r)}</span>
                  </span>
                </li>
              ))}
            </ul>
            {directions ? (
              <p className="lab-effect__dir" id={`${id}-d`}>
                <span><span aria-hidden="true">← </span>left of the line: adaptive arm worse</span>
                <span>right of the line: adaptive arm better<span aria-hidden="true"> →</span></span>
              </p>
            ) : <span id={`${id}-d`} className="visually-hidden">{axis}</span>}
          </>
        ) : <UnavailablePanel title={title} reason="no scenario reported a mean with an interval" />}
        table={(
          <TableBox caption={`${title}: relative change with 95% interval, per scenario`} head={['Scenario', 'Mean', 'Interval low', 'Interval high', 'Reading']}>
            {rows.map((r) => (
              <tr key={r.scenario}>
                <th scope="row">{r.scenario}</th>
                <td className="num">{r.plotted ? FMT.signedRatio(r.mean) : 'Unavailable'}</td>
                <td className="num">{r.plotted ? FMT.signedRatio(r.lo) : 'Unavailable'}</td>
                <td className="num">{r.plotted ? FMT.signedRatio(r.hi) : 'Unavailable'}</td>
                <td>{EFFECT_WORD(r)}</td>
              </tr>
            ))}
          </TableBox>
        )}
      />
    </div>
  );
}
