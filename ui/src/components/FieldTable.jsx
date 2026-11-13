import React from 'react';
import ProvenanceBadge from './ProvenanceBadge.jsx';
import { clip } from '../lib/format.js';

/** Generic listing of provenance fields the UI catalog does not know yet (rows come from additionalFields()). */
export default function FieldTable({ rows, caption }) {
  return (
    <div className="table-wrap">
      <table>
        <caption>{caption}</caption>
        <thead><tr><th scope="col">Field</th><th scope="col">Value</th><th scope="col">Provenance</th></tr></thead>
        <tbody>
          {rows.map((e) => (
            <tr key={e.path} data-field={e.path}>
              <td className="mono">{clip(e.path, 60)}</td>
              <td>{e.text ?? e.reasonText}</td>
              <td><ProvenanceBadge src={e.src} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
