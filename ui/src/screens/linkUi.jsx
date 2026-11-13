import React, { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import Icon from '../components/Icon.jsx';
import { StatusPill } from '../components/States.jsx';
import { describeTelemetryLink, RETRY_LABEL } from '../lib/linkState.js';
import { describeProbe } from '../lib/gatewayConfig.js';
import { testGatewayConnection } from '../lib/labApi.js';

/*
 * Connection UI (increment V2B). One vocabulary: every word and cause sentence comes from lib/linkState.js (describe*Link), and every
 * surface renders view.word. Honesty rules kept here:
 *   - the telemetry channel can never be "Unauthorised" (EventSource exposes no status code; AD-9) and "Recovering" is not a state;
 *   - a Lab 5xx is "Unavailable" (AD-8), decided in linkState;
 *   - every statement is labelled with who knows it ("The gateway answered" vs "Seen by this page");
 *   - the countdown is shown only when the PAGE owns the retry timer (retryOwner 'page'; AD-10);
 *   - tokens are never handled here; the address is the only value that may be saved (gatewayConfig store, try/catch inside).
 */

/** The telemetry link for a context value. Falls back to deriving it from the connection state when a caller (a test) gave no `link`. */
export function linkOf(ctx) {
  if (ctx.link) return ctx.link;
  const { state, conn } = ctx;
  const now = ctx.now ?? (state.receivedAt !== null && conn.ageMs !== null ? state.receivedAt + conn.ageMs : Date.now());
  return describeTelemetryLink(state, now);
}

/** "Next try in 8 s." only when the page owns the timer; otherwise say that no schedule is known. Pure. */
export function retryText(link) {
  if (!link || !link.retry) return null;
  if (link.retryOwner === 'page') {
    if (link.nextRetryInMs === null) return 'Trying again now.';
    const s = Math.ceil(link.nextRetryInMs / 1000);
    return s <= 0 ? 'Trying again now.' : `Next try in ${s} s.`;
  }
  if (link.retryOwner === 'browser') return 'The browser retries by itself, so this page cannot show when.';
  return null;
}

const CONDITION_TO_OLD_KIND = { stale: 'stale', no_answer: 'disconnected', reconnecting: 'disconnected', degraded: 'degraded', incompatible: 'error' };

/**
 * AD-11 gate, pure: the Lab (where a token can be typed) must not open while the address came from the page link, is not loopback
 * (the store sets needsLinkConfirmation), and has not been confirmed for THIS address in this page's memory.
 */
export function needsLinkConfirm(gw, confirmedUrl, routeId) {
  return routeId === 'lab' && Boolean(gw && gw.needsLinkConfirmation) && confirmedUrl !== gw.url;
}

/** Retry and Change address actions for a link view (the one verb is "Retry now"). */
export function LinkActions({ link, onRetry, onChangeAddress }) {
  if (!link || (!link.retry && !link.changeAddress)) return null;
  return (
    <span className="link-actions">
      {link.retry && onRetry ? <button type="button" className="btn" onClick={onRetry}>{link.retryLabel || RETRY_LABEL}</button> : null}
      {link.changeAddress && onChangeAddress ? <button type="button" className="btn" onClick={onChangeAddress}>Change address</button> : null}
    </span>
  );
}

/** What this page can and cannot tell: who knows it, the basis, caveat notes, the fix. One disclosure, closed. */
export function LinkWhy({ link }) {
  const rows = [];
  rows.push(['Who knows this', `${link.whoText}.`]);
  if (link.basis) rows.push(['Basis', link.basis]);
  if (link.fix) rows.push(['What to try', link.fix]);
  for (const n of link.notes || []) rows.push(['Note', n.text]);
  return (
    <details className="disclosure link-why">
      <summary>What this page can and cannot tell</summary>
      <dl className="kv">
        {rows.map(([k, v], i) => (
          <React.Fragment key={`${k}-${i}`}><dt>{k}</dt><dd>{v}</dd></React.Fragment>
        ))}
      </dl>
    </details>
  );
}

/**
 * The one concise notice (DD v2 9.2): what is wrong, the facts this page has, and the two actions. It is a client observation and
 * never implies server health. Rendered above content that is still visible but not current.
 */
export function LinkNotice({ link, ageText, onRetry, onChangeAddress }) {
  if (!link || link.key === 'live' || link.key === 'connecting') return null;
  const tone = link.tone === 'critical' ? 'critical' : 'warn';
  const countdown = retryText(link);
  return (
    <div className={`banner tone-${tone}`} role="status" data-connection={CONDITION_TO_OLD_KIND[link.condition] || link.key} data-link={link.key} data-link-condition={link.condition}>
      <Icon name={link.icon} />
      <div className="banner__body">
        <span className="banner__title">{link.word}{ageText ? ` (last frame ${ageText})` : ''}</span>
        <span>{link.cause}{link.detail ? ` ${link.detail}` : ''}</span>
        {countdown ? <span data-retry-countdown>{countdown}</span> : null}
        <LinkActions link={link} onRetry={onRetry} onChangeAddress={onChangeAddress} />
        <LinkWhy link={link} />
      </div>
    </div>
  );
}

/** Gateway-address notes (rejected ?gateway= value, saved value ignored, link-origin warning). Shown on every route. */
export function GatewayNotes({ gw, onChangeAddress }) {
  const notes = (gw && gw.notes) || [];
  if (notes.length === 0) return null;
  return (
    <div className="banner tone-warn" role="note" data-gateway-notes>
      <Icon name="warn" />
      <div className="banner__body">
        <span className="banner__title">Gateway address</span>
        {notes.map((n) => <span key={n.code}>{n.text}</span>)}
        {onChangeAddress ? <span><button type="button" className="btn" onClick={onChangeAddress}>Change address</button></span> : null}
      </div>
    </div>
  );
}

/**
 * AD-11: a non-loopback address that came from the page link must be confirmed before ANY token can be entered. The Lab (where the
 * token field lives) is not rendered until the person confirms; the confirmation is held in memory only, so a reload asks again.
 */
export function LinkConfirmGate({ gw, onConfirm, onUseDefault }) {
  return (
    <div className="confirm-gate" role="alertdialog" aria-labelledby="confirm-gate-title" aria-describedby="confirm-gate-body" data-link-confirm>
      <h2 id="confirm-gate-title" className="confirm-gate__title">Check this gateway address before continuing</h2>
      <p id="confirm-gate-body">
        This page was opened from a link that points it at <span className="mono">{gw.hostPort}</span>, which is not this computer. Phantom Lab lets you paste a bearer token, and the token would be sent to that address. Continue only if you recognise it and trust the person who gave you the link.
      </p>
      <p className="section__note">Nothing is saved by this choice. Telemetry from the address is already being requested; no token is sent without your entering one.</p>
      <div className="confirm-gate__actions">
        <button type="button" className="btn" onClick={onUseDefault} autoFocus>Use the default address instead</button>
        <button type="button" className="btn" onClick={onConfirm}>I trust {gw.hostPort}; continue</button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Header indicator and connection popover

function ChannelRow({ label, view }) {
  return (
    <li data-channel={view.channel}>
      <span className="conn-channel__label">{label}</span>
      <StatusPill tone={view.tone} icon={view.icon}>{view.word}</StatusPill>
      <span className="conn-channel__cause">{view.cause}{view.detail ? ` ${view.detail}` : ''}</span>
    </li>
  );
}

export function AddressForm({ gw, store, onApplied = () => {} }) {
  const id = useId();
  const [text, setText] = useState(gw.url);
  const [touched, setTouched] = useState(false);
  const [result, setResult] = useState(null);
  const [test, setTest] = useState({ status: 'idle' });
  const abortRef = useRef(null);
  const v = store.validate(text);
  const locked = gw.locked;
  useEffect(() => () => { if (abortRef.current) abortRef.current.abort(); }, []);

  const run = (remember) => {
    setTouched(true);
    const r = store.apply(text, { remember });
    setResult(r);
    if (r.ok) {
      setText(r.url);
      onApplied(r);
    }
  };
  const doTest = async () => {
    setTouched(true);
    if (!v.ok) return;
    if (abortRef.current) abortRef.current.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setTest({ status: 'testing', url: v.url });
    try {
      // Two requests to the CANDIDATE address; no token is attached (the default getToken sends none).
      const r = await testGatewayConnection({ baseUrl: v.url, signal: ac.signal });
      if (!ac.signal.aborted) setTest({ status: 'done', url: v.url, metrics: describeProbe(r.metrics), caps: describeProbe(r.capabilities) });
    } catch {
      if (!ac.signal.aborted) setTest({ status: 'done', url: v.url, metrics: null, caps: null, failed: true });
    }
  };
  const reset = () => {
    const r = store.reset();
    if (r.ok === false) setResult({ ok: false, errors: r.errors, notices: [] });
    else {
      setResult({ ok: true, errors: [], notices: [], url: null, reset: true, originChanged: r.originChanged });
      onApplied(r);
    }
  };
  // After a reset or apply the store snapshot changes; keep the field in step when it is not being edited.
  const lastUrl = useRef(gw.url);
  useEffect(() => {
    if (lastUrl.current !== gw.url) {
      lastUrl.current = gw.url;
      setText(gw.url);
    }
  }, [gw.url]);

  const showErrors = touched && !v.ok;
  return (
    <form className="conn-form" onSubmit={(e) => { e.preventDefault(); run(true); }} noValidate>
      <label className="field__label" htmlFor={id}>Gateway address</label>
      <input
        id={id}
        className="input mono"
        type="text"
        inputMode="url"
        autoComplete="off"
        autoCapitalize="off"
        spellCheck={false}
        value={text}
        onChange={(e) => { setText(e.target.value); setResult(null); setTest({ status: 'idle' }); }}
        aria-invalid={showErrors ? 'true' : undefined}
        aria-describedby={`${id}-hint ${id}-msg`}
        data-address-input
        disabled={Boolean(locked)}
      />
      <div className="conn-presets console-row">
        <span className="t-caption">Fill address:</span>
        {(() => {
          const h = typeof window !== 'undefined' && window.location && window.location.hostname ? window.location.hostname : 'localhost';
          return (
            <>
              <button type="button" className="btn btn--ghost" onClick={() => { setText(`http://${h}:18185`); setResult(null); }}>Node 1 (18185)</button>
              <button type="button" className="btn btn--ghost" onClick={() => { setText(`http://${h}:18186`); setResult(null); }}>Node 2 (18186)</button>
              <button type="button" className="btn btn--ghost" onClick={() => { setText(`http://${h}:18187`); setResult(null); }}>Node 3 (18187)</button>
              <button type="button" className="btn btn--ghost" onClick={() => { setText(`http://${h}:8085`); setResult(null); }}>Standalone (8085)</button>
            </>
          );
        })()}
      </div>
      <p id={`${id}-hint`} className="section__note">
        In use: <span className="mono">{gw.url}</span>, {gw.sourceText}.{' '}
        {gw.remembered ? 'Saved in this browser.' : gw.storageAvailable ? 'Not saved.' : 'This browser does not allow saving it.'}
      </p>
      <div id={`${id}-msg`} className="conn-msgs" aria-live="polite">
        {showErrors ? v.errors.map((e) => <p key={e.code} className="field__error"><Icon name="error" /><span>{e.text}</span></p>) : null}
        {v.ok ? v.notices.map((n) => <p key={n.code} className={n.level === 'warn' ? 'conn-note conn-note--warn' : 'conn-note'}>{n.text}</p>) : null}
        {locked ? <p className="conn-note conn-note--warn">The address cannot change now: {locked}.</p> : null}
        {result && result.ok && result.reset ? <p className="conn-note" role="status">Saved address forgotten. Using {gw.url}.</p> : null}
        {result && result.ok && !result.reset ? (
          <p className="conn-note" role="status">
            Using {result.url}.{' '}
            {result.persisted === 'saved' ? 'Saved in this browser.' : result.persisted === 'not_saved' ? 'It could not be saved in this browser, so it will not be remembered.' : 'Not saved.'}
            {result.originChanged ? ' Any token entered for the previous address was forgotten.' : ''}
          </p>
        ) : null}
        {result && !result.ok ? result.errors.map((e) => <p key={e.code} className="field__error"><Icon name="error" /><span>{e.text}</span></p>) : null}
      </div>
      <div className="conn-form__actions">
        <button type="button" className="btn" onClick={doTest} disabled={test.status === 'testing'}>{test.status === 'testing' ? 'Testing' : 'Test connection'}</button>
        <button type="submit" className="btn btn--primary" disabled={Boolean(locked)}>Use and remember</button>
        <button type="button" className="btn" onClick={() => run(false)} disabled={Boolean(locked)}>Use for this visit</button>
        <button type="button" className="btn" onClick={reset} disabled={Boolean(locked)}>Reset address</button>
      </div>
      <div className="conn-test" role="status" aria-live="polite">
        {test.status === 'testing' ? <p>Asking {test.url} for its capabilities and one telemetry snapshot. No token is sent.</p> : null}
        {test.status === 'done' && !test.failed ? (
          <ul className="conn-test__list" data-test-result>
            <li><strong>Live telemetry</strong> {test.metrics.text}</li>
            <li><strong>Lab API</strong> {test.caps.text}</li>
          </ul>
        ) : null}
        {test.status === 'done' && test.failed ? <p>The test could not run.</p> : null}
      </div>
    </form>
  );
}

/**
 * The header connection control: one button (glyph + word + host) and a non-modal popover with the per-channel words, the retry
 * facts and the address form. Rendered into the Header's `.app-header__status` slot by App (portal), so Shell.jsx is untouched.
 */
export function ConnectionControl({ slot, links, gw, store, onRetry, openRequest }) {
  const [open, setOpen] = useState(false);
  const btnRef = useRef(null);
  const popRef = useRef(null);
  const popId = useId();
  const { worst } = links;
  const lastReq = useRef(openRequest);

  useEffect(() => {
    if (openRequest !== lastReq.current) {
      lastReq.current = openRequest;
      setOpen(true);
    }
  }, [openRequest]);

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

  if (!slot || !worst) return null;
  const countdown = retryText(links.telemetry);
  const body = (
    <>
      <button
        type="button"
        ref={btnRef}
        className="conn__btn"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={popId}
        data-conn-indicator
        data-link={worst.key}
        onClick={() => setOpen((o) => !o)}
        aria-label={`Gateway connection: ${worst.word}. ${gw.hostPort || gw.url}. ${open ? 'Close' : 'Open'} connection details`}
      >
        <StatusPill tone={worst.tone} icon={worst.icon}>{worst.word}</StatusPill>
        <span className="conn__host mono" aria-hidden="true">{gw.hostPort || gw.url}</span>
      </button>
      {open ? (
        <div id={popId} ref={popRef} className="popover conn-popover" role="dialog" aria-label="Gateway connection" tabIndex={-1} data-conn-popover>
          <h2 className="conn-popover__title">Gateway connection</h2>
          <p className="section__note">Words describe what this page sees. They say nothing about the health of the gateway itself.</p>
          <ul className="conn-channels">
            {links.telemetry ? <ChannelRow label="Live telemetry" view={links.telemetry} /> : null}
            {links.lab ? <ChannelRow label="Lab API" view={links.lab} /> : null}
          </ul>
          {countdown ? <p data-retry-countdown>{countdown}</p> : null}
          {links.telemetry && links.telemetry.retry ? <p><button type="button" className="btn" onClick={onRetry}>{RETRY_LABEL}</button></p> : null}
          <AddressForm gw={gw} store={store} onApplied={() => {}} />
          <p className="footnote">Bearer tokens are kept in this page's memory only and are never saved, unless you tick Keep for this tab in the token panel (tab storage, cleared when the tab closes). Only the gateway address can be saved in this browser for good.</p>
        </div>
      ) : null}
    </>
  );
  return createPortal(body, slot);
}
