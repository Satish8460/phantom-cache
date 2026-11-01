import React, { useId } from 'react';

/**
 * L1 surface (spec section 4): solid surface, 1 px border, resting shadow-1 (light theme only).
 * A panel needs a reason: prefer headings plus whitespace (`plain`) for grouping, and never nest a panel in a panel
 * (a nested panel renders as a ruled section, see `.panel .panel` in app.css).
 * `level` sets the heading element (default h2); the visual tier is always the title tier.
 */
export default function Panel({ title, note, aside, children, level = 2, plain = false }) {
  const id = useId();
  const H = `h${level >= 2 && level <= 4 ? level : 2}`;
  return (
    <section className={plain ? 'panel panel--plain' : 'panel'} aria-labelledby={id}>
      <div className="panel__head">
        <H id={id} className="panel__title">{title}</H>
        {aside}
      </div>
      {note ? <p className="panel__note">{note}</p> : null}
      {children}
    </section>
  );
}
