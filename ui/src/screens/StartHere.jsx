import React from 'react';
import ScreenFrame, { Section } from './ScreenFrame.jsx';
import { StatusPill } from '../components/States.jsx';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { DEFAULT_HUB, SCENARIOS, hubLink } from '../lib/hub.js';
import { HubLink } from '../components/HubParts.jsx';
import { hrefFor } from '../lib/routes.js';

// "Start here": the first page for someone who has never seen PhantomCache. Plain words, the same vocabulary as the Demo Hub.
// It shows what THIS PAGE sees (connection word, signed-in identity). It measures nothing and links to the Hub for the scenarios.

const hubOf = (ctx) => (ctx.hub && ctx.hub.url) || DEFAULT_HUB;

/** First-visit welcome: dismissible, remembered for this tab only (the App owns the flag). */
export function WelcomeBanner({ hub, onDismiss }) {
  return (
    <section className="welcome" aria-label="Welcome" data-welcome>
      <p><strong>Welcome to PhantomCache!</strong> It is a caching gateway that sits in front of a slow server and answers repeat requests itself. This app shows what the gateway is doing right now.</p>
      <p>
        <a className="btn btn--primary" href={hrefFor('start')} onClick={onDismiss} data-welcome-start>Start here</a>{' '}
        <HubLink hub={hub}>Open the Demo Hub</HubLink>{' '}
        <button type="button" className="btn" onClick={onDismiss} data-welcome-dismiss>Dismiss</button>
      </p>
    </section>
  );
}

const STEPS = Object.freeze([
  { n: 1, title: 'Run a scenario', text: 'Open the Demo Hub and press Demonstrate on any card. It runs a real test against the real gateways on this computer and narrates each step.' },
  { n: 2, title: 'Watch the gateway', text: 'Come back to this app: Overview, Traffic, Cache and Cluster show the same activity as counters, each labelled measured, derived or unavailable.' },
  { n: 3, title: 'Switch user', text: 'On a secure gateway the token chip at the top right shows who you are. Open a second window as another user to see tenants kept apart.' },
]);

function Connection({ ctx }) {
  const link = ctx.link || null;
  const auth = ctx.auth || null;
  const word = link ? link.word : 'Connecting';
  const tone = link ? link.tone : 'info';
  const icon = link ? link.icon : 'loading';
  const offline = link && ['Disconnected', 'Reconnecting'].includes(link.word);
  const needsToken = auth && auth.answer && auth.answer.needsAction;
  return (
    <>
      <dl className="kv" aria-label="Connection and identity">
        <dt>Gateway</dt>
        <dd className="mono" data-start-gateway>{(ctx.gateway && ctx.gateway.url) || 'not set'}</dd>
        <dt>Connection</dt>
        <dd data-start-connection={word}><StatusPill tone={tone} icon={icon}>{word}</StatusPill> {link ? link.cause : 'Waiting for the first answer.'}</dd>
        <dt>Signed in as</dt>
        <dd data-start-identity>{auth ? auth.chip.text : 'not known'}{auth && auth.chip.text === 'No token (not required)' ? ': this gateway does not ask for a token.' : ''}</dd>
      </dl>
      {needsToken ? (
        <p data-start-needs-token><strong>{auth.answer.title}.</strong> {auth.answer.text} Use the token chip at the top right of the page.</p>
      ) : null}
      {offline ? (
        <p data-start-offline>
          <strong>No gateway answers at this address.</strong> The demo cluster may not be running. Open the Demo Hub and press <em>Start the demo cluster</em>, then come back and press <em>Retry now</em> in the connection panel at the top right.
        </p>
      ) : null}
    </>
  );
}

export default function StartHere() {
  const ctx = useTelemetryContext();
  const hub = hubOf(ctx);
  const groups = [];
  for (const s of SCENARIOS) {
    let g = groups[groups.length - 1];
    if (!g || g.name !== s.group) { g = { name: s.group, items: [] }; groups.push(g); }
    g.items.push(s);
  }
  return (
    <ScreenFrame screenId="start" ungated title="Start here" intro="PhantomCache is a caching gateway: it answers repeat requests itself so the slow server behind it is asked far less often. This app shows what a running gateway reports.">
      <Section title="How this demo works">
        <ol className="plain-list" data-start-steps>
          {STEPS.map((s) => <li key={s.n}><strong>{s.title}.</strong> {s.text}</li>)}
        </ol>
        <p><a className="btn btn--primary" href={hubLink(hub, 'tour')} target="_blank" rel="noopener noreferrer" data-hub-link="tour">Take the 5-minute guided tour</a> <HubLink hub={hub}>Open the Demo Hub</HubLink></p>
        {ctx.hub && ctx.hub.note ? <p className="section__note">{ctx.hub.note}</p> : null}
      </Section>
      <Section title="This page's connection" note="What this page itself sees right now. It is not a health check of the gateway.">
        <Connection ctx={ctx} />
      </Section>
      <Section title="Scenarios in the Demo Hub" note={`Every scenario runs for real on this computer, with one click, in the Demo Hub (${hub}). Each link opens it at that scenario in a new window.`}>
        <p><a className="btn btn--primary" href={hubLink(hub, 'everything')} target="_blank" rel="noopener noreferrer" data-hub-link="everything" data-demonstrate-everything>Demonstrate everything</a> The whole storyline below, in order, with a summary at the end.</p>
        {groups.map((g, i) => (
          <details className="disclosure" key={g.name} open={i < 2} data-start-group={g.name}>
            <summary>{g.name} ({g.items.length})</summary>
            <ul className="plain-list">
              {g.items.map((s) => (
                <li key={s.id} data-scenario={s.id}><strong>{s.title}.</strong> {s.what} <HubLink hub={hub} id={s.id}>Run in the Demo Hub</HubLink></li>
              ))}
            </ul>
          </details>
        ))}
      </Section>
      <Section title="Words you will meet">
        <dl className="kv" aria-label="Glossary">
          <dt>HIT</dt><dd>The gateway answered from its cache; the slow server was not asked.</dd>
          <dt>MISS</dt><dd>The gateway did not have it and asked the slow server (the origin).</dd>
          <dt>Peer</dt><dd>Another gateway in the cluster that answered instead of the origin.</dd>
          <dt>Coalesced</dt><dd>Many identical requests waited for one shared fetch.</dd>
          <dt>Stale</dt><dd>An older answer served on purpose, within a limit, while a fresh one is fetched.</dd>
          <dt>Tenant</dt><dd>A separate cache namespace; one tenant never sees another's entries.</dd>
          <dt>Token</dt><dd>A signed pass that says who you are and what you may do.</dd>
        </dl>
      </Section>
    </ScreenFrame>
  );
}
