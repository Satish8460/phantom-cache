import React from 'react';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import { StateBlock } from '../components/States.jsx';
import { originCapView } from './originCap.js';

/**
 * The origin concurrency controller reading (R4 E3/S1), only as the gateway reports it. No gate picture is drawn: a drawing of a
 * gate protecting the origin would be false whenever the cap is not enforcing, and the gateway reports no per-step reasons.
 * The table IS the reading (there is no chart to be a twin of).
 */
export default function OriginGate({ payload }) {
  const v = originCapView(payload);
  if (v.state === 'loading') return null;
  if (v.state === 'unavailable') {
    return (
      <StateBlock kind="unavailable" title="Unavailable: no origin concurrency gate reported">
        {v.reasonText} No cap or in-flight value is shown, and none is zero.
      </StateBlock>
    );
  }
  return (
    <div className="gate" data-origin-gate={v.state}>
      <p className="gate__state" role="status">
        {v.headline} <ProvenanceBadge src={v.enforcingSrc} />
      </p>
      <div className="table-wrap">
        <table>
          <caption className="visually-hidden">Origin concurrency controller, as reported by the gateway</caption>
          <thead><tr><th scope="col">Reading</th><th scope="col">Value</th><th scope="col">Label</th></tr></thead>
          <tbody>
            {v.rows.map((r) => (
              <tr key={r.key} data-gate-row={r.key}>
                <th scope="row">{r.label}</th>
                <td className="num">{r.status === 'value' ? r.text : <span className="gate__na">Unavailable. {r.reasonText}</span>}</td>
                <td><ProvenanceBadge src={r.src} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <ul className="not-reported" aria-label="Not reported by the origin concurrency controller">
        <li>
          <span className="not-reported__label">Why each controller step was taken</span>
          <ProvenanceBadge src="unavailable" />
          <span className="not-reported__reason">The gateway reports the current cap and the number of steps, not the reason for each step.</span>
        </li>
      </ul>
    </div>
  );
}
