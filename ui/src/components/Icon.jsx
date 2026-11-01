import React from 'react';

// Hand-drawn 16x16 glyphs (original shapes; no third-party or platform icon sets). Decorative: always aria-hidden.
const SHAPES = {
  measured: <circle cx="8" cy="8" r="5.5" fill="currentColor" />,
  derived: <path d="M8 1.5 14.5 8 8 14.5 1.5 8z" fill="currentColor" />,
  inferred: <path d="M8 2.5 13.5 8 8 13.5 2.5 8z" fill="none" stroke="currentColor" strokeWidth="1.8" />,
  simulated: <path d="M8 1.8 15 14H1z" fill="currentColor" />,
  replayed: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M8.5 3 3.5 8l5 5M13.5 3l-5 5 5 5" />
    </g>
  ),
  predicted: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="8" cy="8" r="5.5" strokeDasharray="2 2.4" />
      <circle cx="8" cy="8" r="1.4" fill="currentColor" stroke="none" />
    </g>
  ),
  chevron: <path d="M5.5 3.5 10 8l-4.5 4.5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />,
  unavailable: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8">
      <circle cx="8" cy="8" r="5.5" />
      <path d="M4 12 12 4" />
    </g>
  ),
  ok: <path d="M3 8.5 6.5 12 13 4.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />,
  warn: (
    <g>
      <path d="M8 1.8 15 14H1z" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" />
      <path d="M8 6.5v3.5M8 11.6v.4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </g>
  ),
  error: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="8" cy="8" r="6" />
      <path d="M5.5 5.5 10.5 10.5M10.5 5.5 5.5 10.5" />
    </g>
  ),
  info: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="8" cy="8" r="6" />
      <path d="M8 7.2v4M8 4.9v.2" />
    </g>
  ),
  disconnected: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <path d="M2 8h3M11 8h3M5 5.5v5M11 5.5v5" />
    </g>
  ),
  clock: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="8" cy="8" r="6" />
      <path d="M8 4.5V8l2.5 1.5" />
    </g>
  ),
  loading: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="8" cy="8" r="6" opacity="0.35" />
      <path d="M8 2a6 6 0 0 1 6 6" />
    </g>
  ),
  empty: <rect x="2.5" y="3.5" width="11" height="9" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeDasharray="2.5 2" />,
  // V1 additions (DD v2 4.5): the product mark, a plug for the gateway, an archive box for retained evidence, a half-filled disc for the theme.
  ghost: (
    <g>
      <rect x="6.5" y="6.5" width="8" height="8" rx="2" fill="none" stroke="currentColor" strokeWidth="1.4" strokeDasharray="2 1.8" />
      <rect x="1.5" y="1.5" width="8.5" height="8.5" rx="2" fill="currentColor" />
    </g>
  ),
  gateway: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5.5 2v3M10.5 2v3M4 5h8v2.5a4 4 0 0 1-8 0zM8 11.5V14" />
    </g>
  ),
  retained: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M2 3.5h12v3H2zM3 6.5v6.5h10V6.5M6.5 9.5h3" />
    </g>
  ),
  theme: (
    <g stroke="currentColor" strokeWidth="1.6">
      <circle cx="8" cy="8" r="5.8" fill="none" />
      <path d="M8 2.2a5.8 5.8 0 0 1 0 11.6z" fill="currentColor" />
    </g>
  ),
};

/**
 * The product mark (DD v2 4.1 item 3): a solid rounded square with a dashed, offset copy behind it, "the cache copy that is not the origin".
 * Decorative (aria-hidden); the product name is the text beside it. Drawn in currentColor so it follows the theme and forced colours.
 */
export function GhostMark({ className = 'ghost-mark' }) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <rect x="8" y="8" width="14" height="14" rx="3.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeDasharray="3 2.6" />
      <rect x="2" y="2" width="14" height="14" rx="3.5" fill="currentColor" />
    </svg>
  );
}

/** Names: provenance glyphs (measured, derived, inferred, simulated, replayed, predicted, unavailable) and state glyphs
 *  (ok, warn, error, info, disconnected, clock, loading, empty, chevron), plus ghost, gateway, retained, theme. One stroke family, 16 px viewBox. */
export default function Icon({ name }) {
  const shape = SHAPES[name] || SHAPES.info;
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      {shape}
    </svg>
  );
}
