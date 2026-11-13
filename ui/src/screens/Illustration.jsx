import React, { createContext, useContext, useId } from 'react';

const MarkerContext = createContext('illus-arrow');

/**
 * A figure that is NOT drawn from received data (rule 12; owner decisions DEC-011, DEC-012).
 * It always carries the plain caption "Illustration. Not live data." (a caption, never a provenance badge), a <title> and <desc> inside the
 * SVG, and a text alternative in the page (`notShown`: what the drawing does not show). The SVG is never aria-hidden.
 * Drawings use ink strokes only, dashed outlines for anything that is not the origin, no numbers, no state colours and no motion.
 */
export const ILLUSTRATION_CAPTION = 'Illustration. Not live data.';

export default function Illustration({ title, desc, viewBox, notShown, children }) {
  const id = useId();
  const marker = `${id}-arrow`;
  return (
    <MarkerContext.Provider value={marker}>
    <figure className="illus" aria-labelledby={`${id}-c`}>
      <svg className="illus__svg" viewBox={viewBox} role="img" aria-labelledby={`${id}-t ${id}-d`} focusable="false">
        <title id={`${id}-t`}>{title}</title>
        <desc id={`${id}-d`}>{desc}</desc>
        {children}
      </svg>
      <figcaption id={`${id}-c`} className="illus__caption">
        <span className="illus__tag">{ILLUSTRATION_CAPTION}</span>
        {notShown ? <span className="illus__alt">{notShown}</span> : null}
      </figcaption>
    </figure>
    </MarkerContext.Provider>
  );
}

/** Drawing helpers (user units; the figure scales as one piece, so label sizes stay at or above 12 px at phone width). */
export function Box({ x, y, w, h, label, ghost = true }) {
  return (
    <g>
      <rect className={ghost ? 'illus__box illus__box--ghost' : 'illus__box'} x={x} y={y} width={w} height={h} rx="6" />
      <text className="illus__text" x={x + w / 2} y={y + h / 2 + 4} textAnchor="middle">{label}</text>
    </g>
  );
}

export function Arrow({ d }) {
  const marker = useContext(MarkerContext);
  return <path className="illus__line" d={d} markerEnd={`url(#${marker})`} />;
}

/** One shared arrowhead definition, placed inside each SVG that uses Arrow. */
export function ArrowDefs() {
  const marker = useContext(MarkerContext);
  return (
    <defs>
      <marker id={marker} viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
        <path className="illus__head" d="M1 1 9 5 1 9z" />
      </marker>
    </defs>
  );
}
