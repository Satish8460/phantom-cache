// Bundled by app2Harness.mjs for server-side rendering in the V2B tests. Not shipped to browsers.
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { TelemetryContext } from '../../src/hooks/TelemetryContext.js';
import App from '../../src/App.jsx';
import Overview from '../../src/screens/Overview.jsx';
import Traffic from '../../src/screens/Traffic.jsx';
import CacheMemory from '../../src/screens/CacheMemory.jsx';
import Freshness from '../../src/screens/Freshness.jsx';
import Origin from '../../src/screens/Origin.jsx';
import Placeholder from '../../src/screens/Placeholder.jsx';
import Cluster from '../../src/screens/Cluster.jsx';
import Simulation from '../../src/screens/Simulation.jsx';
import { AddressForm, GatewayNotes, LinkConfirmGate, LinkNotice } from '../../src/screens/linkUi.jsx';
import { createGatewayStore } from '../../src/lib/gatewayConfig.js';

const SCREENS = {
  overview: () => <Overview />,
  traffic: () => <Traffic />,
  cache: () => <CacheMemory />,
  freshness: () => <Freshness />,
  origin: () => <Origin />,
  cluster: () => <Cluster />,
  intelligence: () => <Placeholder screenId="intelligence" />,
  simulation: () => <Simulation />,
};

export const SCREEN_IDS = Object.keys(SCREENS);

export function renderScreen(id, ctx) {
  return renderToStaticMarkup(<TelemetryContext.Provider value={ctx}>{SCREENS[id]()}</TelemetryContext.Provider>);
}

export function renderApp() {
  return renderToStaticMarkup(<App />);
}

export function renderAddressForm({ search = '', envValue, getStorage } = {}) {
  const store = createGatewayStore({ search, envValue, pageOrigin: 'http://localhost:3500', pageProtocol: 'http:', ...(getStorage ? { getStorage } : {}) });
  const gw = store.getSnapshot();
  return { html: renderToStaticMarkup(<AddressForm gw={gw} store={store} />), store, gw };
}

export function renderNotes(gw) {
  return renderToStaticMarkup(<GatewayNotes gw={gw} onChangeAddress={() => {}} />);
}

export function renderConfirm(gw) {
  return renderToStaticMarkup(<LinkConfirmGate gw={gw} onConfirm={() => {}} onUseDefault={() => {}} />);
}

export function renderLinkNotice(link, ageText) {
  return renderToStaticMarkup(<LinkNotice link={link} ageText={ageText} onRetry={() => {}} onChangeAddress={() => {}} />);
}
