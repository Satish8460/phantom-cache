import React, { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { StatusPill } from '../components/States.jsx';
import { DECODE_LABEL, expiryText, roleHints } from '../lib/gatewayAuth.js';

// Sign-in surfaces of the Control Center (board D-045 item 2). Everything shown here comes from useGatewayAuth: display data only.
// The token string is never rendered, never put in an attribute, a URL or an error message; the input is a password field that is
// emptied as soon as the token is handed to the holder.

function hostOf(url) {
  try {
    return new URL(url).host;
  } catch {
    return String(url || '');
  }
}

const yes = (b) => (b ? 'yes' : 'no');

/** Who the token is, display-only, with the label that says the page did not verify it. */
export function IdentityDetails({ auth }) {
  if (!auth.hasToken) return null;
  const d = auth.decoded;
  if (!d || !d.ok) {
    return (
      <p data-token-unreadable>
        A token is set, but this page cannot read its claims. The gateway still decides whether to accept it.
      </p>
    );
  }
  const c = d.claims;
  const h = roleHints(c);
  return (
    <div data-token-identity>
      <dl className="kv" aria-label="Who this token is">
        <dt>Tenant</dt>
        <dd data-claim="tenant">{c.tenant || 'none (cache requests will be refused with 403)'}</dd>
        <dt>Subject</dt>
        <dd data-claim="subject">{c.subject || 'not stated'}</dd>
        <dt>Roles</dt>
        <dd data-claim="roles">{c.roles.length ? c.roles.join(', ') : 'none'}</dd>
        <dt>Issuer</dt>
        <dd data-claim="issuer">{c.issuer || 'not stated'}</dd>
        <dt>Audience</dt>
        <dd data-claim="audience">{c.audience || 'not stated'}</dd>
        <dt>Expiry</dt>
        <dd data-claim="expiry" data-expiry-state={auth.exp.state}>{expiryText(auth.exp)}</dd>
      </dl>
      {auth.exp.state === 'expired' && auth.answer && auth.answer.key === 'ok' ? (
        <p className="section__note" data-expired-but-streaming>
          The gateway checks a token when a request or stream starts (src/api/server.rs, sse_handler). This stream was authorised before the token expired and keeps running; the next request or reconnect will be refused with 401.
        </p>
      ) : null}
      {h ? (
        <p className="section__note" data-role-hints>
          By its roles the token appears to allow: telemetry {yes(h.observe)}, cache reads {yes(h.cacheRead)}, purges {yes(h.invalidate)}, operating {yes(h.operate)}.
        </p>
      ) : null}
      <p className="section__note" data-decode-label>{DECODE_LABEL}</p>
    </div>
  );
}

function AnswerLine({ answer }) {
  if (!answer) return <p className="section__note" data-gateway-answer="none">The gateway has not answered yet.</p>;
  return (
    <div data-gateway-answer={answer.key}>
      <p><StatusPill tone={answer.tone} icon={answer.tone === 'ok' ? 'ok' : answer.tone === 'info' ? 'info' : 'warn'}>{answer.title}</StatusPill></p>
      <p>{answer.text}</p>
    </div>
  );
}

export function TokenForm({ auth, gatewayUrl, required = false }) {
  const id = useId();
  const [value, setValue] = useState('');
  const [keep, setKeep] = useState(false);
  return (
    <form
      className="auth-form"
      data-token-form
      onSubmit={(e) => {
        e.preventDefault();
        if (auth.accept(value, { keep })) setValue('');
      }}
    >
      <div className="field">
        <label className="field__label" htmlFor={id}>Bearer token for {hostOf(gatewayUrl)} {required ? '(required by this gateway)' : '(optional)'}</label>
        <div className="field__row">
          <input
            id={id}
            type="password"
            autoComplete="off"
            spellCheck={false}
            value={value}
            aria-describedby={`${id}-h`}
            aria-invalid={auth.problem ? 'true' : undefined}
            onChange={(e) => {
              setValue(e.target.value);
              if (auth.problem) auth.clearProblem();
            }}
          />
          <button type="submit" className="btn btn--primary" disabled={!value.trim()}>Sign in</button>
          <button type="button" className="btn" onClick={auth.forget} disabled={!auth.hasToken}>Forget token</button>
        </div>
        <p id={`${id}-h`} className="field__hint">
          {auth.hasToken ? 'A token is set for this gateway in this tab.' : 'No token is set.'} It is sent only to {hostOf(gatewayUrl)}, only in the Authorization header, never in a link.
        </p>
        {auth.problem ? <p className="field__error" role="alert" data-token-problem>{auth.problem}</p> : null}
      </div>
      <label className="choice" data-keep-for-tab>
        <input type="checkbox" checked={keep || auth.keptForTab} onChange={(e) => setKeep(e.target.checked)} disabled={auth.storageProblem} />
        <span>Keep for this tab (stored in tab storage, cleared when the tab closes). Off: the token is lost on reload.</span>
      </label>
      {auth.storageProblem ? <p className="section__note">This browser blocks tab storage, so the token can only be kept in memory.</p> : null}
    </form>
  );
}

export function HandoffNotice({ auth }) {
  if (!auth.handoffPending) return null;
  return (
    <div className="state-block" role="note" data-handoff-pending>
      <div className="state-block__head"><span>A window offers a token for this gateway</span></div>
      <p>The window at <span className="mono">{auth.handoffPending.from}</span> opened this page and sent a token for this gateway. It has not been used. Use it only if you started that window.</p>
      <p>
        <button type="button" className="btn btn--primary" onClick={auth.confirmHandoff}>Use the offered token</button>{' '}
        <button type="button" className="btn" onClick={auth.dismissHandoff}>Ignore it</button>
      </p>
    </div>
  );
}

export function OpenAsAnother({ gatewayUrl }) {
  return (
    <details className="disclosure" data-open-as-another>
      <summary>Open as another user or on another gateway</summary>
      <p>
        Each window keeps its own token. Open this page in a new window with <span className="mono">?gateway=</span> and the address of that
        gateway (now: <span className="mono">{gatewayUrl}</span>), then paste that user&rsquo;s token there. A second user needs a second window
        (or a private window): one tab holds one token per gateway.
      </p>
      <p>Tokens are never put in links. A window opened by the demo console receives its token from the console by message, not by link.</p>
    </details>
  );
}

/**
 * Banner at the top of the main area. Appears only when the page must say something: the gateway needs a token, refused one, or a
 * window offered one. When there is nothing to say it renders nothing, so a healthy page is not cluttered.
 */
export function AuthBanner({ auth, gatewayUrl }) {
  const a = auth.answer;
  const show = Boolean(auth.handoffPending) || (a && a.needsAction);
  if (!show) return null;
  const kind = a ? (a.tone === 'critical' ? 'error' : a.tone === 'info' ? 'info' : 'warning') : 'warning';
  return (
    <section className="auth-banner" aria-label="Gateway sign-in" data-auth-banner data-auth-key={a ? a.key : 'handoff'}>
      <HandoffNotice auth={auth} />
      {a && a.needsAction ? (
        <div className="state-block" role={kind === 'error' ? 'alert' : 'note'} data-state-kind={kind}>
          <div className="state-block__head"><span>{a.title}</span></div>
          <p>{a.text}</p>
          <p className="section__note">Gateway: <span className="mono">{hostOf(gatewayUrl)}</span>{a.status ? <> &middot; HTTP {a.status}</> : null}</p>
          {auth.hasToken ? <IdentityDetails auth={auth} /> : null}
          <TokenForm auth={auth} gatewayUrl={gatewayUrl} required={a.key === 'requires_token'} />
          <OpenAsAnother gatewayUrl={gatewayUrl} />
        </div>
      ) : null}
    </section>
  );
}

/** Header chip + popover (portalled into the header status slot by App, like the connection control). */
export function IdentityControl({ slot, auth, gatewayUrl }) {
  const [open, setOpen] = useState(false);
  const btnRef = useRef(null);
  const popRef = useRef(null);
  const popId = useId();
  useEffect(() => {
    if (!open) return undefined;
    popRef.current?.focus();
    const onDown = (e) => {
      if (popRef.current && !popRef.current.contains(e.target) && !btnRef.current?.contains(e.target)) setOpen(false);
    };
    const onKey = (e) => {
      if (e.key === 'Escape') {
        setOpen(false);
        btnRef.current?.focus();
      }
    };
    document.addEventListener('pointerdown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('pointerdown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);
  if (!slot) return null;
  const chip = auth.chip;
  const body = (
    <>
      <button
        type="button"
        ref={btnRef}
        className="conn__btn"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={popId}
        data-identity-chip
        data-auth-tone={chip.tone}
        onClick={() => setOpen((o) => !o)}
        aria-label={`Token: ${chip.text}. ${open ? 'Close' : 'Open'} token details`}
      >
        <StatusPill tone={chip.tone} icon={chip.icon}>{chip.text}</StatusPill>
      </button>
      {open ? (
        <div id={popId} ref={popRef} className="popover conn-popover" role="dialog" aria-label="Token and identity" tabIndex={-1} data-identity-popover>
          <h2 className="conn-popover__title">Token and identity</h2>
          <AnswerLine answer={auth.answer} />
          <IdentityDetails auth={auth} />
          <HandoffNotice auth={auth} />
          <TokenForm auth={auth} gatewayUrl={gatewayUrl} required={auth.requiresToken} />
          <OpenAsAnother gatewayUrl={gatewayUrl} />
        </div>
      ) : null}
    </>
  );
  return createPortal(body, slot);
}
