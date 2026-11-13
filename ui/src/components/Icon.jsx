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
  dyno: <path d="M8.5 1.5 3.5 8.5h4L6.5 14.5l6-7h-4z" fill="currentColor" />,
  cockpit: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
      <path d="M2.5 4h11M2.5 8h11M2.5 12h11M5 2.5v3M11 6.5v3M7 10.5v3" />
    </g>
  ),
  buyer: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 5.5h10l1 8.5H2l1-8.5z" />
      <path d="M5.5 5.5V3.8a2.5 2.5 0 0 1 5 0v1.7" />
    </g>
  ),
  seller: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M2 6h12l-1.2-3.5H3.2L2 6z" />
      <path d="M3 6v7.5h10V6" />
      <path d="M6.5 13.5v-4h3v4" />
    </g>
  ),
  sun: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
      <circle cx="8" cy="8" r="3" fill="currentColor" stroke="none" />
      <path d="M8 1.5v1.8M8 12.7v1.8M1.5 8h1.8M12.7 8h1.8M3.4 3.4l1.3 1.3M11.3 11.3l1.3 1.3M3.4 12.6l1.3-1.3M11.3 4.7l1.3-1.3" />
    </g>
  ),
  moon: <path d="M12.5 9.2A5.2 5.2 0 0 1 6.8 3.5a5.2 5.2 0 1 0 5.7 5.7z" fill="currentColor" />,
  metrics: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
      <path d="M2 14h12" />
      <path d="M4.5 11.5v-4" />
      <path d="M8 11.5v-7.5" />
      <path d="M11.5 11.5v-5.5" />
    </g>
  ),
  tunnel: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="8" r="6" />
      <path d="M2 8h12M8 2a9 9 0 0 1 0 12M8 2a9 9 0 0 0 0 12" />
    </g>
  ),
  crowd: (
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="6" cy="5" r="2.5" />
      <path d="M2 13.5v-1a3 3 0 0 1 6 0v1" />
      <circle cx="11.5" cy="6.5" r="2" />
      <path d="M9.5 13.5v-.5a2.5 2.5 0 0 1 5 0v.5" />
    </g>
  ),
  play: <path d="M5 3.5l8 4.5-8 4.5z" fill="currentColor" />,
  pause: <path d="M4.5 3.5h2.5v9H4.5zm4.5 0h2.5v9H9z" fill="currentColor" />,
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
