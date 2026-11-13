import React from 'react';
import { DEFAULT_HUB, helpFor, hubLink, scenarioById } from '../lib/hub.js';

/** A link that opens a Hub page in a new window. No token is ever put in the address. */
export function HubLink({ hub, id, children }) {
  return <a href={hubLink(hub, id)} target="_blank" rel="noopener noreferrer" data-hub-link={id || 'catalogue'}>{children}</a>;
}

/** One line under a screen title: what am I looking at, plus the Hub scenarios that relate to it. */
export function HelpLine({ screenId, hub = DEFAULT_HUB }) {
  const h = helpFor(screenId);
  if (!h) return null;
  return (
    <p className="help-line" data-help-line={screenId}>
      <strong>What am I looking at?</strong> {h.text}
      {h.scenarios.map((id) => {
        const s = scenarioById(id);
        return s ? <React.Fragment key={id}> <HubLink hub={hub} id={id}>Try it in the Demo Hub: {s.title}</HubLink></React.Fragment> : null;
      })}
    </p>
  );
}
