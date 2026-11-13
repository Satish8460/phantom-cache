import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import './styles/tokens.css';
import './styles/app.css';
import './styles/lab.css';
import './styles/shell.css';
import './styles/dyno.css';
import './styles/cockpit.css';
import './styles/observability.css';
import './styles/personas.css';
import { Header, Nav, MENU_BUTTON_ID, PersonaContext, PersonaPortalControl } from './components/Shell.jsx';
import { SimulationBanner } from './components/States.jsx';
import { TelemetryContext } from './hooks/TelemetryContext.js';
import { useTelemetry } from './hooks/useTelemetry.js';
import { useHistory } from './hooks/useHistory.js';
import { useGateway, useGatewayStore } from './hooks/useGateway.js';
import { useGatewayAuth } from './hooks/useGatewayAuth.js';
import { useIntelligence } from './hooks/useIntelligence.js';
import { intelligenceNavLive } from './lib/intelligenceView.js';
import { describeTelemetryLink, worstLink } from './lib/linkState.js';
import { footerFor, parseRoute, titleFor } from './lib/routes.js';
import { applyTheme, getStorage, loadTheme, normalizeTheme, saveTheme } from './lib/theme.js';
import Overview from './screens/Overview.jsx';
import Traffic from './screens/Traffic.jsx';
import CacheMemory from './screens/CacheMemory.jsx';
import Freshness from './screens/Freshness.jsx';
import Origin from './screens/Origin.jsx';
import Intelligence from './screens/Intelligence.jsx';
import StartHere, { WelcomeBanner } from './screens/StartHere.jsx';
import ExecutiveScreen from './screens/ExecutiveScreen.jsx';
import SreCockpitScreen from './screens/SreCockpitScreen.jsx';
import BuyerPersonaScreen from './screens/BuyerPersonaScreen.jsx';
import SellerPersonaScreen from './screens/SellerPersonaScreen.jsx';
import { resolveHub } from './lib/hub.js';
import { markWelcomeSeen, readWelcomeSeen } from './lib/tokenSession.js';
import Cluster from './screens/Cluster.jsx';
import Simulation from './screens/Simulation.jsx';
import LabScreen from './screens/lab/LabScreen.jsx';
import { ConnectionControl, GatewayNotes, LinkConfirmGate, needsLinkConfirm } from './screens/linkUi.jsx';
import { AuthBanner, IdentityControl } from './screens/authUi.jsx';
import { probeClusterHealth } from './lib/labApi.js';

function readLocation() {
  if (typeof window === 'undefined') return { hash: '', search: '' };
  return { hash: window.location.hash, search: window.location.search };
}

function useRoute() {
  const [loc, setLoc] = useState(readLocation);
  useEffect(() => {
    const on = () => setLoc(readLocation());
    window.addEventListener('hashchange', on);
    return () => window.removeEventListener('hashchange', on);
  }, []);
  return parseRoute(loc);
}

/**
 * `epoch` changes only when the gateway ORIGIN changes. It is the React key of the Lab screen, so a new origin remounts the Lab provider:
 * its token ref (memory only) is dropped and the roles, client and resources start from nothing. A credential is bound to the origin it was
 * entered for (FLIB wiring spec 3; AD-11 intent) and must never be sent to a different one.
 */
function renderScreen(id, sub, epoch) {
  switch (id) {
    case 'start': return <StartHere />;
    case 'traffic': return <Traffic />;
    case 'cache': return <CacheMemory />;
    case 'freshness': return <Freshness />;
    case 'origin': return <Origin />;
    case 'lab': return <LabScreen key={`gateway-${epoch}`} sub={sub} />;
    case 'cluster': return <Cluster />;
    case 'intelligence': return <Intelligence />;
    case 'simulation': return <Simulation />;
    default: return <Overview />;
  }
}

export default function App() {
  const route = useRoute();
  const store = useGatewayStore({
    location: typeof window !== 'undefined' ? window.location : null,
    envValue: typeof import.meta !== 'undefined' && import.meta.env ? import.meta.env.VITE_PHANTOM_GATEWAY : undefined,
  });
  const gw = useGateway(store);
  const hub = useMemo(() => resolveHub(typeof window !== 'undefined' ? window.location.search : ''), []);
  const [welcomeSeen, setWelcomeSeen] = useState(() => readWelcomeSeen());
  const dismissWelcome = useCallback(() => { markWelcomeSeen(); setWelcomeSeen(true); }, []);
  const auth = useGatewayAuth(gw.url);
  const { state, conn, now, reconnect, offsetMs } = useTelemetry(gw.url, auth.feed);
  const intel = useIntelligence(auth, gw.url, { active: route.id === 'intelligence' });
  const series = useHistory(state.payload, state.receivedAt, gw.url);
  // Light is the default look; a visitor's own choice (stored for this browser) still wins.
  const [theme, setTheme] = useState(() => loadTheme(getStorage(typeof window !== 'undefined' ? window : null), 'light'));
  const [persona, setPersona] = useState(() => {
    if (typeof window !== 'undefined' && window.location) {
      const hash = window.location.hash || '';
      if (hash.includes('persona=seller') || hash === '#/seller') return 'seller';
      if (hash.includes('persona=executive') || hash === '#/executive') return 'executive';
      if (hash.includes('persona=sre') || hash === '#/sre') return 'sre';
      if (hash.includes('persona=buyer') || hash === '#/buyer') return 'buyer';
      if (hash && hash !== '#/' && hash !== '#/overview') {
        return 'sre';
      }
      return 'buyer';
    }
    return 'buyer';
  });
  const choosePersona = useCallback((p) => {
    setPersona(p);
    if (typeof window !== 'undefined' && window.location) {
      window.location.hash = `#/overview?persona=${p}`;
    }
  }, []);
  useEffect(() => {
    if (typeof window === 'undefined') return undefined;
    const handleHash = () => {
      const hash = window.location.hash || '';
      if (hash.includes('persona=seller') || hash === '#/seller') setPersona('seller');
      else if (hash.includes('persona=executive') || hash === '#/executive') setPersona('executive');
      else if (hash.includes('persona=sre') || hash === '#/sre') setPersona('sre');
      else if (hash.includes('persona=buyer') || hash === '#/buyer') setPersona('buyer');
    };
    window.addEventListener('hashchange', handleHash);
    return () => window.removeEventListener('hashchange', handleHash);
  }, []);
  const [sreView, setSreView] = useState('engine');
  const mainRef = useRef(null);
  const [menuOpen, setMenuOpen] = useState(false);

  // Auto-detect cluster gateway: if saved address has 8085 but cluster port 18185 is alive, auto-switch
  useEffect(() => {
    if (typeof window === 'undefined') return;
    if (gw.url.includes(':8085')) {
      probeClusterHealth().then((alive) => {
        if (alive) store.apply('http://127.0.0.1:18185');
      });
    }
  }, [gw.url, store]);
  // The key of the route last shown. Compared (not a first-render flag) so React StrictMode's double effect run cannot steal focus on load.
  const shownKey = useRef(null);
  const routeKey = route.sub ? `${route.id}/${route.sub.view}/${route.sub.id || ''}` : route.id;

  // ---- one connection vocabulary (lib/linkState.js) ----
  const telemetryLink = useMemo(() => describeTelemetryLink(state, now, { probe: auth.linkProbe }), [state, now, auth.linkProbe]);
  // The Lab API link lives inside the Lab provider; the Lab reports it here (reportLabLink) and it counts only while the Lab is open.
  const [labLinkRaw, setLabLinkRaw] = useState(null);
  const reportLabLink = useCallback((view) => {
    setLabLinkRaw((prev) => (prev && view && prev.key === view.key && prev.text === view.text ? prev : view || null));
  }, []);
  const onLab = route.id === 'lab';
  const links = useMemo(() => {
    const lab = onLab ? labLinkRaw : null;
    return { telemetry: telemetryLink, lab, worst: worstLink([telemetryLink, lab].filter(Boolean)) };
  }, [telemetryLink, labLinkRaw, onLab]);

  // ---- connection indicator slot (Shell.jsx is not edited: the control is portalled into .app-header__status) ----
  const [slot, setSlot] = useState(null);
  const [personaSlot, setPersonaSlot] = useState(null);
  const [meshSlot, setMeshSlot] = useState(null);
  const [tunnelSlot, setTunnelSlot] = useState(null);
  useEffect(() => {
    const el = document.querySelector('.app-header__status');
    if (el) {
      el.setAttribute('data-linked', '1');
      setSlot(el);
    }
    setPersonaSlot(document.querySelector('.app-header__persona-slot'));
    setMeshSlot(document.querySelector('.app-header__mesh-slot'));
    setTunnelSlot(document.querySelector('.app-header__tunnel-slot'));
    return () => {
      if (el) el.removeAttribute('data-linked');
    };
  }, []);
  const [openRequest, setOpenRequest] = useState(0);
  const openConnection = useCallback(() => setOpenRequest((n) => n + 1), []);

  // ---- gateway address: origin change forgets everything bound to the old origin ----
  const epochRef = useRef(gw.epoch);
  useEffect(() => {
    if (epochRef.current === gw.epoch) return;
    epochRef.current = gw.epoch;
    setLabLinkRaw(null);
    // Run and evidence ids belong to the previous gateway.
    if (/^#\/lab\/(runs|evidence)\/./.test(window.location.hash)) window.location.hash = '#/lab';
  }, [gw.epoch]);

  // AD-11: a non-loopback address that came from the page link must be confirmed before the Lab (where a token can be entered) opens.
  // Held in memory only, per address; a reload asks again.
  const [confirmedUrl, setConfirmedUrl] = useState(null);
  const needsConfirm = needsLinkConfirm(gw, confirmedUrl, route.id);

  useEffect(() => {
    applyTheme(document.documentElement, theme);
  }, [theme]);

  const title = titleFor(route);
  useEffect(() => {
    document.title = title;
  }, [title]);

  const chooseTheme = (next) => {
    const t = normalizeTheme(next);
    setTheme(t);
    saveTheme(getStorage(window), t);
  };

  // Move focus to the screen heading on navigation (not on first load) so keyboard and screen-reader users land on the new content.
  useEffect(() => {
    const previous = shownKey.current;
    shownKey.current = routeKey;
    if (previous === null || previous === routeKey) return;
    window.scrollTo(0, 0);
    mainRef.current?.querySelector('[data-screen-heading]')?.focus();
  }, [routeKey]);

  const intelLive = intelligenceNavLive(intel.raw.status);
  const ctx = useMemo(
    () => ({ payload: state.payload, state, conn, series, gateway: gw, reconnect, offsetMs, now, link: telemetryLink, store, openConnection, reportLabLink, auth, intel, intelLive, hub }),
    [state, conn, series, gw, reconnect, offsetMs, now, telemetryLink, store, openConnection, reportLabLink, auth, intel, intelLive, hub],
  );

  const simulatedGateway = state.payload?.mode === 'simulation' && route.id !== 'simulation';

  const personaCtx = useMemo(() => ({ persona, setPersona: choosePersona }), [persona, choosePersona]);

  return (
    <TelemetryContext.Provider value={ctx}>
      <PersonaContext.Provider value={personaCtx}>
        <div
          className="app"
          onKeyDown={(e) => {
            // Escape closes the phone menu and returns focus to the button that opened it (skill S7: back along the path it came).
            if (e.key === 'Escape' && menuOpen) {
              setMenuOpen(false);
              document.getElementById(MENU_BUTTON_ID)?.focus();
            }
          }}
        >
          <a
            className="skip-link"
            href="#main"
            onClick={(e) => {
              // The router owns location.hash: following "#main" would be read as an unknown route. Move focus without changing the hash.
              e.preventDefault();
              mainRef.current?.focus();
            }}
          >
            Skip to main content
          </a>
          <Header connKind={conn.kind} theme={theme} onTheme={chooseTheme} menuOpen={menuOpen} onMenu={() => setMenuOpen((o) => !o)} />
          <PersonaPortalControl personaSlot={personaSlot} meshSlot={meshSlot} tunnelSlot={tunnelSlot} />
          <ConnectionControl slot={slot} links={links} gw={gw} store={store} onRetry={reconnect} openRequest={openRequest} />
        <IdentityControl slot={slot} auth={auth} gatewayUrl={gw.url} />
        <Nav
          currentId={route.id}
          open={menuOpen}
          onNavigate={() => setMenuOpen(false)}
          theme={theme}
          onTheme={chooseTheme}
          mode={state.payload?.mode}
          gatewayUrl={gw.url}
        />
        <main id="main" className="app-main" ref={mainRef} tabIndex={-1}>
          {/* Announces the connection WORD only when it changes; the countdown and ages are never in a live region (no 1 Hz noise). */}
          <div role="status" aria-live="polite" className="visually-hidden" data-live-region>
            {links.worst ? `Connection: ${links.worst.word}` : ''}
          </div>
          {route.notFound ? (
            <p className="panel__note" role="status">That page does not exist; showing the overview.</p>
          ) : null}
          {!welcomeSeen && route.id !== 'start' ? <WelcomeBanner hub={hub.url} onDismiss={dismissWelcome} /> : null}
          <AuthBanner auth={auth} gatewayUrl={gw.url} />
          <GatewayNotes gw={gw} onChangeAddress={openConnection} />
          {simulatedGateway ? (
            <SimulationBanner>
              The gateway reports simulation mode. Fields labelled Simulated are model output, not measurements.
            </SimulationBanner>
          ) : null}
          {needsConfirm ? (
            <LinkConfirmGate
              gw={gw}
              onConfirm={() => setConfirmedUrl(gw.url)}
              onUseDefault={() => store.reset()}
            />
          ) : route.id !== 'overview' && route.id !== '' ? (
            renderScreen(route.id, route.sub, gw.epoch)
          ) : persona === 'buyer' ? (
            <BuyerPersonaScreen />
          ) : persona === 'seller' ? (
            <SellerPersonaScreen />
          ) : persona === 'executive' ? (
            <ExecutiveScreen />
          ) : sreView === 'engine' && typeof window !== 'undefined' ? (
            <SreCockpitScreen onSwitchToMetrics={() => setSreView('metrics')} />
          ) : (
            renderScreen(route.id, route.sub, gw.epoch)
          )}
        </main>
        <footer className="app-footer">
          {footerFor(route)}
        </footer>
      </div>
    </PersonaContext.Provider>
  </TelemetryContext.Provider>
);
}
