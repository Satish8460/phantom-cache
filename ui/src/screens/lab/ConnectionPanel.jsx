import React, { useEffect, useId, useState } from 'react';
import { StatusPill } from '../../components/States.jsx';
import { labHref } from '../../lib/routes.js';
import { describeLabLink } from '../../lib/linkState.js';

// NOTE: the file keeps its old name because the render harness (ui/scripts/harness-entry.jsx) imports it. The component is the
// Lab STATUS STRIP (increment I2, lead decision (c); docs/design/phantom_lab_ui.md change log 2026-10-30): one compact row of
// text items that always shows the honesty statements (debug build, intelligence not in the serving path), plus a Details
// region that opens by itself only when attention is needed (error, API off, unreachable, token required and missing).

function AvailabilityRow({ label, k }) {
  if (!k) return null;
  const tone = k.available ? 'ok' : 'warn';
  return (
    <li data-kind-availability={k.key}>
      <StatusPill tone={tone} icon={k.available ? 'ok' : 'warn'}>{k.available ? 'Available' : 'Not available'}</StatusPill>
      <span>{label}{!k.available && k.reason ? `: ${k.reason}` : ''}</span>
    </li>
  );
}

function TokenForm({ hasToken, roles, onToken, onClear, required }) {
  const id = useId();
  const [value, setValue] = useState('');
  return (
    <form
      className="lab-token"
      onSubmit={(e) => {
        e.preventDefault();
        if (value.trim()) {
          onToken(value.trim());
          setValue('');
        }
      }}
    >
      <div className="field">
        <label className="field__label" htmlFor={id}>Bearer token {required ? '(required by this gateway)' : '(optional)'}</label>
        <div className="field__row lab-token__row">
          <input id={id} type="password" autoComplete="off" spellCheck={false} value={value} onChange={(e) => setValue(e.target.value)} aria-describedby={`${id}-h`} />
          <button type="submit" className="btn btn--primary" disabled={!value.trim()}>Use token</button>
          <button type="button" className="btn" onClick={onClear} disabled={!hasToken}>Forget token</button>
        </div>
        <p id={`${id}-h`} className="field__hint">
          {hasToken ? 'A token is set for this tab.' : 'No token is set.'}{hasToken && roles ? ` Roles read from it (not verified): ${roles.length ? roles.join(', ') : 'none'}.` : ''} It is kept in this page memory only: it is never put in a URL, never written to browser storage, and is lost on reload.
          Observer reads; Operator starts and cancels runs.
        </p>
      </div>
    </form>
  );
}

function authChip(view, hasToken) {
  if (!view) return null;
  if (view.security.key === 'anonymous_dev_loopback_only') return 'Anonymous, loopback';
  if (view.security.key === 'jwt') return hasToken ? 'Token set' : 'Token required';
  return view.security.label;
}

/**
 * Lab status strip. `res` is the useLabResource() result for GET /capabilities, `view` the capabilitiesView().
 * Chips (always visible): API state, authentication mode, active run (link), debug-build EXPLORATORY, intelligence not in the
 * serving path. Details (one disclosure away): gateway, schema, availability with reasons, explanations, token form.
 */
export default function ConnectionPanel({ res, view, hasToken, roles = null, onToken, onClear, gatewayUrl }) {
  const err = res.status === 'error' ? res.error : null;
  const disabled = err && err.kind === 'disabled';
  // One vocabulary (lib/linkState.js): the word is the same on every surface; the cause sentence says who observed it.
  const link = describeLabLink(res);
  const apiPill = <StatusPill tone={link.tone} icon={link.icon}>{`Lab API: ${link.word}`}</StatusPill>;
  const needsToken = view ? view.security.needsToken : err && /^(unauthorized|forbidden_role)/.test(err.kind) ? true : null;
  const showToken = !disabled && needsToken !== false;
  // Attention = something the person must act on or understand now. A debug build is NOT attention: it is a chip.
  const attention = Boolean(err && res.data === null) || (needsToken === true && !hasToken);
  const [open, setOpen] = useState(attention);
  useEffect(() => { if (attention) setOpen(true); }, [attention]);
  const regionId = useId();
  const auth = authChip(view, hasToken);
  return (
    <section className="lab-status" aria-label="Lab status" data-lab-status data-attention={attention ? 'true' : undefined}>
      <ul className="lab-status__chips" aria-label="Lab status summary">
        <li>{apiPill}</li>
        {auth ? <li><StatusPill tone="neutral" icon="info">{auth}</StatusPill></li> : null}
        {view && view.activeRun ? (
          <li><a className="lab-status__active" href={labHref('runs', view.activeRun.runId)}>A run is active: open it</a></li>
        ) : null}
        {view && view.debugBuild ? <li><StatusPill tone="warn" icon="warn">Debug build: results EXPLORATORY</StatusPill></li> : null}
        <li><StatusPill tone="info" icon="info">Intelligence: not in serving path</StatusPill></li>
        <li className="lab-status__toggle-item">
          <button type="button" className="btn btn--ghost lab-status__toggle" aria-expanded={open} aria-controls={regionId} onClick={() => setOpen((o) => !o)}>
            {open ? 'Hide details' : 'Details'}
          </button>
        </li>
      </ul>
      <div id={regionId} className="lab-status__details" hidden={!open}>
        <p className="lab-status__gateway">Gateway <span className="mono">{gatewayUrl}</span></p>
        {view ? (
          <>
            <dl className="kv" aria-label="Lab API capabilities">
              <dt>Authentication</dt>
              <dd data-security-mode={view.security.key}>{view.security.label}. {view.security.detail}</dd>
              <dt>Schema</dt>
              <dd>{view.schema || 'not reported'}</dd>
              <dt>Concurrent runs</dt>
              <dd>{view.maxConcurrent === null ? 'not reported' : `${view.maxConcurrent} at a time`}</dd>
            </dl>
            <ul className="lab-avail" aria-label="Run kinds">
              <AvailabilityRow label="Simulation matrix (T1, simulated)" k={view.t1} />
              <AvailabilityRow label="Scenario B (T2, real processes on loopback)" k={view.sb} />
            </ul>
            {view.activeRun ? (
              <p>
                <a href={labHref('runs', view.activeRun.runId)}>Open the active run</a> ({view.activeRun.kind}, {view.activeRun.state}). Only one run can be active.
              </p>
            ) : null}
            {view.debugBuild ? (
              <p data-debug-explainer>
                <strong>Debug build: results are EXPLORATORY.</strong> The gateway and runner were built in the debug profile, so any Scenario B run is labelled EXPLORATORY regardless of its settings. Debug-build runs check mechanics only.
                {view.sb.labelReasons.length ? <> Reasons reported: {view.sb.labelReasons.join('; ')}.</> : null}
              </p>
            ) : null}
          </>
        ) : null}
        <p data-lab-link-cause>{link.cause}</p>
        {err && res.data === null ? <p data-error-pointer>{disabled ? 'The Lab API is switched off on this gateway.' : link.who === 'gateway' ? 'The gateway answered the capabilities request with a refusal or an error.' : 'The gateway did not answer the capabilities request.'} The section below explains what failed and how to fix it.</p> : null}
        {showToken ? <TokenForm hasToken={hasToken} roles={roles} onToken={onToken} onClear={onClear} required={needsToken === true} /> : null}
        <p data-intelligence-notice>
          <strong>The intelligence engine is not part of the live serving path.</strong> Phantom Intelligence (the policy engine) is research code and is not connected to live request serving in this build. Scenario B switches on only the origin-concurrency gate (a static or AIMD cap) in isolated test gateway processes. Nothing on this page controls live traffic.
        </p>
      </div>
    </section>
  );
}
