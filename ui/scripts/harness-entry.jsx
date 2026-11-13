// Bundled by render-harness.mjs for server-side rendering in tests and the a11y lint. Not shipped to browsers.
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { TelemetryContext } from '../src/hooks/TelemetryContext.js';
import Overview from '../src/screens/Overview.jsx';
import Traffic from '../src/screens/Traffic.jsx';
import CacheMemory from '../src/screens/CacheMemory.jsx';
import Freshness from '../src/screens/Freshness.jsx';
import Origin from '../src/screens/Origin.jsx';
import Placeholder from '../src/screens/Placeholder.jsx';
import Cluster from '../src/screens/Cluster.jsx';
import Simulation from '../src/screens/Simulation.jsx';
import LabScreen from '../src/screens/lab/LabScreen.jsx';
import { EvidenceDetailBody, EvidenceListBody } from '../src/screens/lab/EvidenceViews.jsx';
import { RunDetailBody, RunsListBody } from '../src/screens/lab/RunsViews.jsx';
import { RunFormBody } from '../src/screens/lab/RunForm.jsx';
import StoryView from '../src/screens/lab/StoryView.jsx';
import ConnectionPanel from '../src/screens/lab/ConnectionPanel.jsx';
import { ErrorBlock, ReadForbiddenBlock, ResourceGate } from '../src/screens/lab/Gate.jsx';
import { EffectChart, LatencyChart, ShedChart, TradeoffChart } from '../src/components/LabCharts.jsx';
import { RichNotice, StateChip, TierBadge, VerdictChip } from '../src/components/LabParts.jsx';
import { Header, Nav } from '../src/components/Shell.jsx';
import ProvenanceBadge from '../src/components/ProvenanceBadge.jsx';
import { Sparkline, PercentileChart, CapacityChart } from '../src/components/Charts.jsx';
import { AuthBanner, IdentityDetails, TokenForm } from '../src/screens/authUi.jsx';
import StartHere, { WelcomeBanner } from '../src/screens/StartHere.jsx';
import Intelligence, { IntelligenceBody } from '../src/screens/Intelligence.jsx';

const SCREEN_COMPONENTS = {
  overview: () => <Overview />,
  traffic: () => <Traffic />,
  cache: () => <CacheMemory />,
  freshness: () => <Freshness />,
  origin: () => <Origin />,
  cluster: () => <Cluster />,
  lab: (sub) => <LabScreen sub={sub} />,
  intelligence: () => <Placeholder screenId="intelligence" />,
  simulation: () => <Simulation />,
};

export function renderScreen(id, ctx, sub) {
  return renderToStaticMarkup(
    <TelemetryContext.Provider value={ctx}>
      {SCREEN_COMPONENTS[id](sub)}
    </TelemetryContext.Provider>,
  );
}

export function renderShell({ connKind, currentId, mode = 'live', theme = 'system', menuOpen = false, intelligenceLive = false }) {
  const inner = (
    <>
      <Header connKind={connKind} theme={theme} onTheme={() => {}} menuOpen={menuOpen} onMenu={() => {}} />
      <Nav currentId={currentId} open={menuOpen} theme={theme} onTheme={() => {}} mode={mode} gatewayUrl="http://localhost:8085" />
    </>
  );
  // Without a provider the Nav shows the registry default (Intelligence under "Not available yet"); with one, the gateway's statement.
  return renderToStaticMarkup(intelligenceLive ? <TelemetryContext.Provider value={{ intelLive: true }}>{inner}</TelemetryContext.Provider> : inner);
}

export function renderBadge(src) {
  return renderToStaticMarkup(<ProvenanceBadge src={src} />);
}

export function renderSparkline(props) {
  return renderToStaticMarkup(<Sparkline {...props} />);
}

export function renderChart(kind, props) {
  const C = { percentile: PercentileChart, capacity: CapacityChart, sparkline: Sparkline }[kind];
  return renderToStaticMarkup(<C {...props} />);
}

export const SCREEN_IDS = Object.keys(SCREEN_COMPONENTS);

/** Render any element inside a telemetry context (for Lab view tests). */
export function renderLab(element) {
  return renderToStaticMarkup(element);
}

export const Lab = {
  EvidenceDetailBody, EvidenceListBody, RunDetailBody, RunsListBody, RunFormBody, StoryView, ConnectionPanel, ErrorBlock, ReadForbiddenBlock, ResourceGate,
  RichNotice, EffectChart, LatencyChart, ShedChart, TradeoffChart, StateChip, TierBadge, VerdictChip,
};
export function renderStart(ctx) {
  return renderToStaticMarkup(<TelemetryContext.Provider value={ctx}><StartHere /></TelemetryContext.Provider>);
}

export function renderWelcome(props) {
  return renderToStaticMarkup(<WelcomeBanner {...props} />);
}

export const Intel = { Intelligence, IntelligenceBody };

export function renderIntelligence(ctx) {
  return renderToStaticMarkup(<TelemetryContext.Provider value={ctx}><Intelligence /></TelemetryContext.Provider>);
}
export const Auth = { AuthBanner, IdentityDetails, TokenForm };
export { React };
