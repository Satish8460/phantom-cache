import React from 'react';
import { StateBlock } from '../../components/States.jsx';
import { Notice } from '../../components/LabParts.jsx';
import { roleCapabilities } from '../../lib/labToken.js';

/** Error block for a mapped Lab API error (see lib/labApi.js mapLabError). Distinguishes the causes in words. */
export function ErrorBlock({ error, onRetry, what = 'this' }) {
  const e = error || { kind: 'unexpected', title: 'Something went wrong', fix: '', serverMessage: '' };
  const kind = e.kind === 'disabled' ? 'notEnabled' : 'error';
  const auth = ['unauthorized_missing', 'unauthorized_invalid', 'unauthorized_expired', 'forbidden_role'].includes(e.kind);
  return (
    <StateBlock
      kind={kind}
      title={e.title}
      action={(
        <>
          {e.serverMessage ? <p className="panel__note">Gateway message: <span className="mono">{e.serverMessage}</span></p> : null}
          {auth ? <p className="panel__note">Open Details in the Lab status strip above to set a token for {what}.</p> : null}
          {onRetry ? <button type="button" className="btn" onClick={onRetry}>Retry</button> : null}
        </>
      )}
    >
      {e.fix}
    </StateBlock>
  );
}

/** Renders loading / error / ready for a useLabResource() result. `children(data)` renders the ready state. */
export function ResourceGate({ res, what, children }) {
  if (res.status === 'idle') return null;
  if (res.status === 'loading') {
    return <StateBlock kind="loading" title={`Loading ${what}`}>Waiting for the gateway to answer.</StateBlock>;
  }
  if (res.status === 'error' && res.data === null) {
    return <ErrorBlock error={res.error} onRetry={res.reload} what={what} />;
  }
  return (
    <>
      {res.status === 'error' ? <ErrorBlock error={res.error} onRetry={res.reload} what={what} /> : null}
      {children(res.data)}
    </>
  );
}

/**
 * Shown on a run page when reading it was refused with 403 (missing telemetry:read). Explains the common case of an
 * Operator-only token: it may start and cancel runs but cannot read them. The role list is read from the pasted token for
 * display only; the gateway decides.
 */
export function ReadForbiddenBlock({ error, runId, roles = null, onRetry }) {
  const caps = roleCapabilities(roles);
  const operatorOnly = caps !== null && caps.canOperate && !caps.canRead;
  return (
    <>
      <ErrorBlock error={error} onRetry={onRetry} what="this run" />
      <Notice tone="info" title={operatorOnly ? 'This token can start runs but cannot read them' : 'Reading a run needs a different role'} role="note">
        {operatorOnly
          ? <>Your token ({roles.join(', ')}) has the Operator role, so the gateway can accept a run you start, but it has no Observer role (telemetry:read), so it cannot show that run, its progress or its results. If you just started this run it was probably accepted and is not affected by this refusal; that is not confirmed here. Its id is <span className="mono">{runId}</span>. Paste an Observer or admin token under Details in the Lab status strip to watch it.</>
          : <>Reading runs needs the Observer role (telemetry:read) or admin. Starting runs needs Operator (scenario:operate). {roles && roles.length ? <>Your token lists: {roles.join(', ')}.</> : <>The roles in your token are not known to this page.</>}</>}
      </Notice>
    </>
  );
}
