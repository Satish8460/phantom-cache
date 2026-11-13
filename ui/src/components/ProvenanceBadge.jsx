import React from 'react';
import Icon from './Icon.jsx';
import { PROVENANCE, isKnownSrc } from '../lib/provenance.js';

/**
 * Display vocabulary (owner decision 2026-10-30; spec section 11): REAL, SIMULATED, REPLAYED, PREDICTED, DERIVED, UNAVAILABLE.
 * The DOM contract is unchanged: class `prov prov--<key>` and `data-provenance="<key>"` still use the five telemetry
 * classes (measured, derived, inferred, simulated, unavailable) plus the two reserved ones. Only the visible word moves:
 *   measured -> "Real"      (always with a basis qualifier when the caller knows one, e.g. "loopback")
 *   inferred -> "Derived"   with the fixed qualifier "inferred"
 *   replayed, predicted     reserved; drawn only when a caller passes them. lib/provenance.js readField() still rejects
 *                           them as unknown_provenance, so telemetry cannot produce them today (no such data exists).
 */
export const DISPLAY = Object.freeze({
  measured: { word: 'Real', qualifier: null },
  derived: { word: 'Derived', qualifier: null },
  inferred: { word: 'Derived', qualifier: 'inferred' },
  simulated: { word: 'Simulated', qualifier: null },
  replayed: { word: 'Replayed', qualifier: null },
  predicted: { word: 'Predicted', qualifier: null },
  unavailable: { word: 'Unavailable', qualifier: null },
});

export const RESERVED_SRC = Object.freeze(['replayed', 'predicted']);

const RESERVED_MEANING = Object.freeze({
  replayed: 'Re-played from a recorded trace. Not live and not a fresh measurement.',
  predicted: 'Forecast by a model. Not observed.',
});

/**
 * Provenance badge: glyph + word (+ short qualifier), caption tier, 1 px border; never colour alone. Unknown classes render as Unavailable.
 * `label` replaces the whole text (tier-specific wording such as "Retained: simulated"); `qualifier` adds the short basis
 * ("loopback"); `title` overrides the tooltip; `tier` is recorded in `data-tier`.
 */
export default function ProvenanceBadge({ src, label, title, tier, qualifier }) {
  const key = isKnownSrc(src) || RESERVED_SRC.includes(src) ? src : 'unavailable';
  const display = DISPLAY[key];
  const q = qualifier || (label ? null : display.qualifier);
  const meaning = PROVENANCE[key] ? PROVENANCE[key].meaning : RESERVED_MEANING[key];
  return (
    <span className={`prov prov--${key}`} data-provenance={key} data-tier={tier || undefined} title={title || meaning}>
      <Icon name={key} />
      <span className="prov__word">{label || display.word}</span>
      {q ? <span className="prov__q">{q}</span> : null}
    </span>
  );
}
