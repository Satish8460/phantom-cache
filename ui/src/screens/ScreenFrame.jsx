import React, { useId } from 'react';
import ProvenanceBadge from '../components/ProvenanceBadge.jsx';
import { StateBlock } from '../components/States.jsx';
import { LinkActions, LinkNotice, linkOf, retryText } from './linkUi.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { screenGate } from '../lib/viewmodel.js';
import { parseErrorText } from '../lib/schema.js';
import { formatAge } from '../lib/format.js';
import { screenById } from '../lib/routes.js';
import { HelpLine } from '../components/HubParts.jsx';
import { DEFAULT_HUB } from '../lib/hub.js';

/**
 * Page header shared by every non-Lab page (spec section 2): where you are (group, skill S16 wayfinding), the one display-size title,
 * and a one-line purpose. `screenId` supplies the group caption from the route registry.
 */
export function PageHead({ screenId, title, intro, group: groupOverride }) {
  const group = groupOverride || (screenId ? screenById(screenId)?.group : null);
  return (
    <div className="page-head">
      {group ? <p className="page-head__group">{group}</p> : null}
      <h1 tabIndex={-1} data-screen-heading>{title}</h1>
      {intro ? <p className="page-head__intro">{intro}</p> : null}
    </div>
  );
}

/**
 * A page section: a headline-tier heading plus whitespace, no box (spec section 4: a panel needs a reason; nothing is nested in a card).
 * `aside` sits on the heading row (for example a provenance badge); `note` is the one-line explanation under it.
 */
export function Section({ title, note, aside, children, level = 2 }) {
  const id = useId();
  const H = `h${level >= 2 && level <= 4 ? level : 2}`;
  return (
    <section className="section" aria-labelledby={id}>
      <div className="section__head">
        <H id={id}>{title}</H>
        {aside}
      </div>
      {note ? <p className="section__note">{note}</p> : null}
      {children}
    </section>
  );
}

/**
 * Things the gateway does not report, grouped into one quiet list (state grammar: Unavailable = the word, the reason, never zero).
 * items: [{ label, reason }]. Each row carries the UNAVAILABLE badge; no value is drawn.
 */
export function NotReportedList({ items, label = 'Not reported' }) {
  if (!items.length) return null;
  return (
    <ul className="not-reported" aria-label={label}>
      {items.map((i) => (
        <li key={i.id || i.label}>
          <span className="not-reported__label">{i.label}</span>
          <ProvenanceBadge src="unavailable" />
          <span className="not-reported__reason">{i.reason}</span>
        </li>
      ))}
    </ul>
  );
}

/**
 * Wraps a data screen: heading, intro and the loading/disconnected/error gate. Every connection word and sentence comes from linkState
 * (the same vocabulary as the header indicator); the actions are the one verb "Retry now" and "Change address".
 * With data present, stale/disconnected/degraded states render ONE notice and keep (flagged) content visible.
 */
export default function ScreenFrame({ screenId, title, intro, group, ungated = false, children }) {
  const ctx = useTelemetryContext();
  const { conn, state, reconnect, openConnection } = ctx;
  const link = linkOf(ctx);
  const gate = screenGate(conn.kind, conn.hasData);
  let body;
  const actions = (
    <div className="link-actions-row">
      {retryText(link) ? <p data-retry-countdown>{retryText(link)}</p> : null}
      <LinkActions link={link} onRetry={reconnect} onChangeAddress={openConnection} />
    </div>
  );
  if (ungated) {
    body = children; // the Start here page explains offline and sign-in states itself
  } else if (gate === 'loading') {
    body = <StateBlock kind="loading" title={link.word === 'Connecting' ? 'Connecting to the gateway' : link.word}>{link.cause} No values are shown until data arrives.</StateBlock>;
  } else if (gate === 'disconnected' && !conn.hasData) {
    body = (
      <StateBlock kind="disconnected" title={link.word} action={actions}>
        {link.cause}{link.detail ? ` ${link.detail}` : ''} No telemetry has been received yet, so there is nothing to show.
      </StateBlock>
    );
  } else if (gate === 'error') {
    body = (
      <StateBlock kind="error" title={link.word} action={actions}>
        {link.cause} {state.parseError ? parseErrorText(state.parseError.code, state.parseError.detail) : link.detail}
      </StateBlock>
    );
  } else {
    const ageText = conn.ageMs === null ? null : formatAge(conn.ageMs);
    body = (
      <>
        <LinkNotice link={link} ageText={ageText} onRetry={reconnect} onChangeAddress={openConnection} />
        {children}
      </>
    );
  }
  return (
    <>
      <PageHead screenId={screenId} title={title} intro={intro} group={group} />
      <HelpLine screenId={screenId} hub={(ctx.hub && ctx.hub.url) || DEFAULT_HUB} />
      {body}
    </>
  );
}
