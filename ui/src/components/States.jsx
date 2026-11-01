import React from 'react';
import Icon from './Icon.jsx';
import ProvenanceBadge from './ProvenanceBadge.jsx';
import { CONNECTION_TEXT } from '../lib/connection.js';

/*
 * State grammar (spec section 10). Every async region uses exactly these kinds; each has a glyph AND text, never colour alone.
 *   loading      glyph + "Loading <what>" + aria-busy; no spinner without text
 *   empty        what is empty, why, one next action
 *   error        what failed, what it means, retry (action), detail disclosure; role=alert
 *   success      inline confirmation naming the object
 *   warning      before-problem notice with the specific condition
 *   disabled     control stays visible; reason shown (use <DisabledReason> next to the control)
 *   stale        last-update age shown with the value
 *   unavailable  "Unavailable" plus the reason; never zero
 *   simulated    SIMULATED badge on the region header
 * Also kept from the earlier component: disconnected, notEnabled.
 */
const KIND = {
  loading: { icon: 'loading', role: 'status' },
  empty: { icon: 'empty', role: 'status' },
  error: { icon: 'error', role: 'alert' },
  success: { icon: 'ok', role: 'status' },
  warning: { icon: 'warn', role: 'note' },
  disabled: { icon: 'unavailable', role: 'note' },
  stale: { icon: 'clock', role: 'note' },
  unavailable: { icon: 'unavailable', role: 'status' },
  simulated: { icon: 'simulated', role: 'note' },
  disconnected: { icon: 'disconnected', role: 'status' },
  notEnabled: { icon: 'unavailable', role: 'status' },
};

/** Full-region state. `detail` renders behind a disclosure (error detail); `action` is the retry / next-step control. */
export function StateBlock({ kind, title, children, action, detail }) {
  const spec = KIND[kind] || { icon: 'info', role: 'status' };
  return (
    <div className="state-block" role={spec.role} data-state-kind={kind} aria-busy={kind === 'loading' ? 'true' : undefined}>
      <div className="state-block__head">
        <Icon name={spec.icon} />
        <span>{title}</span>
        {kind === 'simulated' ? <ProvenanceBadge src="simulated" /> : null}
      </div>
      {children ? <p>{children}</p> : null}
      {detail ? (
        <details className="disclosure">
          <summary>Details</summary>
          <p className="mono">{detail}</p>
        </details>
      ) : null}
      {action}
    </div>
  );
}

/** Inline busy indicator: a glyph that turns (static under reduced motion) and ALWAYS the words. */
export function Spinner({ children = 'Loading' }) {
  return (
    <span className="spinner" role="status">
      <Icon name="loading" />
      <span>{children}</span>
    </span>
  );
}

/** Reason a control is disabled. Pair with aria-disabled (keeps focus and lets screen readers reach this text) and aria-describedby. */
export function DisabledReason({ id, children }) {
  return (
    <span className="disabled-reason" id={id}>
      <Icon name="info" />
      <span>{children}</span>
    </span>
  );
}

/** Age tag shown next to a value that keeps its provenance but is no longer current. */
export function StaleTag({ ageText }) {
  return (
    <span className="tag tag--stale">
      <Icon name="clock" />
      <span>{ageText ? `stale, ${ageText}` : 'stale'}</span>
    </span>
  );
}

/** Inline field error: glyph + text. Shown only when the field is touched (CSS: `.field:not([data-touched]) .field__error`). */
export function InlineError({ id, children }) {
  return (
    <p className="field__error" id={id}>
      <Icon name="error" />
      <span>{children}</span>
    </p>
  );
}

/** Inline completion message with the object name, e.g. "Run 2026... started". */
export function InlineSuccess({ children }) {
  return (
    <p className="inline-success" role="status">
      <Icon name="ok" />
      <span>{children}</span>
    </p>
  );
}

const TONE_ICON = { stale: ['warn', 'clock'], disconnected: ['critical', 'disconnected'], degraded: ['warn', 'warn'], error: ['critical', 'error'] };

/** Thin banner shown above content that is still visible but not trustworthy as "current". */
export function ConnectionBanner({ kind, ageText, onReconnect, extra }) {
  const spec = TONE_ICON[kind];
  if (!spec) return null;
  const text = CONNECTION_TEXT[kind];
  return (
    <div className={`banner tone-${spec[0]}`} role="status" data-connection={kind}>
      <Icon name={spec[1]} />
      <div className="banner__body">
        <span className="banner__title">{text.title}{ageText ? ` (last frame ${ageText})` : ''}</span>
        <span>{extra || text.detail}</span>
        {onReconnect && kind === 'disconnected' ? (
          <span>
            <button type="button" className="btn" onClick={onReconnect}>Reconnect now</button>
          </span>
        ) : null}
      </div>
    </div>
  );
}

export function SimulationBanner({ children }) {
  return (
    <div className="banner banner--simulated" role="note" data-banner="simulated">
      <Icon name="simulated" />
      <div className="banner__body">
        <span className="banner__title">Simulated</span>
        <span>{children}</span>
      </div>
    </div>
  );
}

export function StatusPill({ tone = 'neutral', icon = 'info', children }) {
  return (
    <span className={`pill tone-${tone}`}>
      <Icon name={icon} />
      <span>{children}</span>
    </span>
  );
}
